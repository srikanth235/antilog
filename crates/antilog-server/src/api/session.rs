//! Sessions, agents and environments: each call is a signed-in member or token, checked by the
//! action layer; handlers decode, call `antilog-core` and encode.

use std::sync::Arc;

use antilog_core::deps::Deps;
use antilog_core::organization::{self as floor, Action, Refusal};
use antilog_core::session::{self, NewSession, SessionError};
use antilog_core::store::{ChangesStore, OrganizationStore, SessionStore};
use antilog_core::{agents, budget, environments as env};
use antilog_types::{
    Agent, ApiError, AutoChoice, CommitChanges, Environment, EventPage, NewAgent, NewEnvironment,
    ProjectId, SendMessage, Session, SessionBudget, SessionChanges, SessionId, SessionSpend,
    StartSession,
};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use utoipa::OpenApi;

use super::auth::{App, Authed, Fail};

#[derive(OpenApi)]
#[openapi(paths(
    start,
    sessions,
    project_sessions,
    read,
    message,
    stop,
    events,
    changes,
    commit_changes,
    spend,
    set_budget,
    agents,
    auto_agent,
    set_agent,
    environments,
    create_environment,
    edit_environment,
    archive_environment
))]
pub struct Doc;

type Shared<D> = State<Arc<App<D>>>;

pub fn refuse(error: SessionError) -> Response {
    let (status, code, message) = match &error {
        SessionError::NotFound => (
            StatusCode::NOT_FOUND,
            "not_found",
            "No such project or session here: check the ids.".to_owned(),
        ),
        SessionError::Ended => (
            StatusCode::CONFLICT,
            "session_ended",
            "The session has ended: start a new one.".to_owned(),
        ),
        SessionError::CapReached => (StatusCode::CONFLICT, "budget_spent", error.to_string()),
        SessionError::Invalid(message) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid",
            (*message).to_owned(),
        ),
        SessionError::Environment(error) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_environment",
            error.to_string(),
        ),
        SessionError::Store(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "unavailable",
            "A dependency does not answer; retry shortly.".to_owned(),
        ),
    };
    (
        status,
        Json(ApiError {
            code: code.to_owned(),
            message,
            fields: Vec::new(),
        }),
    )
        .into_response()
}

async fn owned<D: Deps + 'static>(
    app: &App<D>,
    authed: &Authed,
    id: SessionId,
) -> Result<Session, Fail> {
    floor::member_of(app.store(), &authed.who).await?;
    Ok(app
        .store()
        .session(authed.who.organization, id)
        .await?
        .ok_or(Refusal::NotFound)?)
}

async fn permit<D: Deps + 'static>(
    app: &App<D>,
    authed: &Authed,
    action: Action,
    project: ProjectId,
) -> Result<(), Fail> {
    floor::permit_in(
        app.store(),
        &authed.who,
        action,
        project,
        project,
        app.now(),
    )
    .await?;
    Ok(())
}

#[utoipa::path(
    post, path = "/projects/{id}/sessions", operation_id = "start_session", security(("bearer" = [])),
    params(("id" = ProjectId, Path, description = "The project")),
    request_body = StartSession,
    responses(
        (status = 201, description = "Started: queued for a runner; editors and owners", body = Session),
        (status = 409, description = "The organization's monthly cap is reached", body = ApiError),
        (status = 422, body = ApiError),
    )
)]
async fn start<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(project): Path<ProjectId>,
    Json(body): Json<StartSession>,
) -> Result<(StatusCode, Json<Session>), Fail> {
    permit(&app, &authed, Action::SessionStart, project).await?;
    if body.prompt.trim().is_empty() {
        return Err(Refusal::Invalid("say what you want done: the prompt is empty").into());
    }
    let agent = agents::resolve(
        app.store(),
        authed.who.organization,
        body.agent.as_deref().unwrap_or(agents::AUTO),
    )
    .await?;
    let model = body
        .model
        .map(|model| model.trim().to_owned())
        .filter(|model| !model.is_empty())
        .or_else(|| {
            app.settings
                .runner
                .as_ref()
                .map(|runner| runner.core.model.clone())
        })
        .ok_or(Refusal::Invalid(
            "name a model as provider/model: this server has no default",
        ))?;
    if model.split_once('/').is_none_or(|(provider, name)| {
        provider.is_empty() || name.is_empty() || model.contains(char::is_whitespace)
    }) {
        return Err(Refusal::Invalid("a model looks like provider/model").into());
    }
    let new = NewSession {
        title: body.title,
        agent,
        model,
        prompt: body.prompt,
        repos: body.repos,
        environment: body.environment,
        budget_cap_cents: body.budget_cap_cents,
        follow_policy: body.follow_policy,
        actor: authed.who.member,
    };
    let session = session::start(&*app.deps, authed.who.organization, project, new)
        .await
        .map_err(Fail::Session)?;
    Ok((StatusCode::CREATED, Json(session)))
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
struct SessionsQuery {
    /// Only the sessions the caller started.
    mine: Option<bool>,
}

#[utoipa::path(
    get, path = "/projects/{id}/sessions", operation_id = "project_sessions", security(("bearer" = [])),
    params(("id" = ProjectId, Path, description = "The project"), SessionsQuery),
    responses((status = 200, description = "The project's sessions, newest first; `mine` keeps the caller's", body = Vec<Session>), (status = 404, body = ApiError))
)]
async fn project_sessions<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(project): Path<ProjectId>,
    Query(query): Query<SessionsQuery>,
) -> Result<Json<Vec<Session>>, Fail> {
    floor::member_of(app.store(), &authed.who).await?;
    app.store()
        .project(authed.who.organization, project)
        .await?
        .ok_or(Refusal::NotFound)?;
    let creator = query.mine.unwrap_or(false).then_some(authed.who.member);
    Ok(Json(
        app.store()
            .sessions(authed.who.organization, Some(project), creator)
            .await?,
    ))
}

#[utoipa::path(
    get, path = "/sessions", security(("bearer" = [])), params(SessionsQuery),
    responses((status = 200, description = "The organization's sessions across its projects, newest first, each with its status and project; `mine` keeps the caller's", body = Vec<Session>))
)]
async fn sessions<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Query(query): Query<SessionsQuery>,
) -> Result<Json<Vec<Session>>, Fail> {
    floor::member_of(app.store(), &authed.who).await?;
    let creator = query.mine.unwrap_or(false).then_some(authed.who.member);
    Ok(Json(
        app.store()
            .sessions(authed.who.organization, None, creator)
            .await?,
    ))
}

#[utoipa::path(
    get, path = "/sessions/{id}", security(("bearer" = [])),
    params(("id" = SessionId, Path, description = "The session")),
    responses((status = 200, body = Session), (status = 404, body = ApiError))
)]
async fn read<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<SessionId>,
) -> Result<Json<Session>, Fail> {
    Ok(Json(owned(&app, &authed, id).await?))
}

#[utoipa::path(
    post, path = "/sessions/{id}/messages", security(("bearer" = [])),
    params(("id" = SessionId, Path, description = "The session")),
    request_body = SendMessage,
    responses((status = 200, body = Session), (status = 409, body = ApiError))
)]
async fn message<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<SessionId>,
    Json(body): Json<SendMessage>,
) -> Result<Json<Session>, Fail> {
    let row = owned(&app, &authed, id).await?;
    permit(&app, &authed, Action::SessionMessage, row.project_id).await?;
    session::message(
        &*app.deps,
        authed.who.organization,
        id,
        Some(authed.who.member),
        &body.text,
    )
    .await
    .map(Json)
    .map_err(Fail::Session)
}

#[utoipa::path(
    post, path = "/sessions/{id}/stop", security(("bearer" = [])),
    params(("id" = SessionId, Path, description = "The session")),
    responses((status = 200, description = "Done, \"Stopped by\" the caller", body = Session), (status = 409, body = ApiError))
)]
async fn stop<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<SessionId>,
) -> Result<Json<Session>, Fail> {
    let row = owned(&app, &authed, id).await?;
    permit(&app, &authed, Action::SessionStop, row.project_id).await?;
    let name = app
        .store()
        .member(authed.who.organization, authed.who.member)
        .await?
        .map(|member| member.name)
        .ok_or(Refusal::NotFound)?;
    session::stop(
        &*app.deps,
        authed.who.organization,
        id,
        Some(authed.who.member),
        &name,
    )
    .await
    .map(Json)
    .map_err(Fail::Session)
}

#[utoipa::path(
    get, path = "/agents", security(("bearer" = [])),
    responses((status = 200, description = "The agents the organization turned on; its own list, or the built-in one until it sets up any", body = Vec<Agent>))
)]
async fn agents<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
) -> Result<Json<Vec<Agent>>, Fail> {
    Ok(Json(agents::list(app.store(), &authed.who).await?))
}

#[utoipa::path(
    get, path = "/agents/auto", security(("bearer" = [])),
    responses(
        (status = 200, description = "The agent Auto would start, and why", body = AutoChoice),
        (status = 422, description = "No agent is turned on, or several are and none is the default", body = ApiError),
    )
)]
async fn auto_agent<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
) -> Result<Json<AutoChoice>, Fail> {
    floor::member_of(app.store(), &authed.who).await?;
    let (agent, reason) = agents::auto(app.store(), authed.who.organization).await?;
    Ok(Json(AutoChoice { agent, reason }))
}

#[utoipa::path(
    put, path = "/agents/{id}", operation_id = "set_agent", security(("bearer" = [])),
    params(("id" = String, Path, description = "A lower-case slug such as opencode")),
    request_body = NewAgent,
    responses((status = 200, description = "Set; organization owners only", body = Agent))
)]
async fn set_agent<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<String>,
    Json(new): Json<NewAgent>,
) -> Result<Json<Agent>, Fail> {
    Ok(Json(
        agents::set(app.store(), &authed.who, app.now(), &id, new).await?,
    ))
}

#[utoipa::path(
    get, path = "/environments", security(("bearer" = [])),
    responses((status = 200, description = "The built-in default, then the organization's environments by id, archived ones marked", body = Vec<Environment>))
)]
async fn environments<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
) -> Result<Json<Vec<Environment>>, Fail> {
    Ok(Json(env::list(app.store(), &authed.who).await?))
}

#[utoipa::path(
    post, path = "/environments", operation_id = "create_environment", security(("bearer" = [])),
    request_body = NewEnvironment,
    responses((status = 201, description = "Created at version 1; owners and editors", body = Environment), (status = 409, body = ApiError), (status = 422, body = ApiError))
)]
async fn create_environment<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Json(new): Json<NewEnvironment>,
) -> Result<(StatusCode, Json<Environment>), Fail> {
    let made = env::create(app.store(), &authed.who, app.now(), new).await?;
    Ok((StatusCode::CREATED, Json(made)))
}

#[utoipa::path(
    put, path = "/environments/{id}", operation_id = "edit_environment", security(("bearer" = [])),
    params(("id" = String, Path, description = "The environment's id")),
    request_body = NewEnvironment,
    responses((status = 200, description = "The next version; owners and editors", body = Environment), (status = 404, body = ApiError), (status = 409, body = ApiError), (status = 422, body = ApiError))
)]
async fn edit_environment<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<String>,
    Json(new): Json<NewEnvironment>,
) -> Result<Json<Environment>, Fail> {
    Ok(Json(
        env::edit(app.store(), &authed.who, app.now(), &id, new).await?,
    ))
}

#[utoipa::path(
    post, path = "/environments/{id}/archive", operation_id = "archive_environment", security(("bearer" = [])),
    params(("id" = String, Path, description = "The environment's id")),
    responses((status = 204, description = "Archived; owners and editors"), (status = 404, body = ApiError))
)]
async fn archive_environment<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<String>,
) -> Result<StatusCode, Fail> {
    env::archive(app.store(), &authed.who, app.now(), &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
struct EventsQuery {
    /// The `seq` of the last item already seen; leave out for the start.
    after: Option<i64>,
    /// At most this many items (default 100, at most 500).
    limit: Option<u32>,
}

#[utoipa::path(
    get, path = "/sessions/{id}/events", security(("bearer" = [])),
    params(("id" = SessionId, Path, description = "The session"), EventsQuery),
    responses(
        (status = 200, description = "The conversation, oldest first: words people and agents said and the names of tools called, never a tool's output", body = EventPage),
        (status = 404, body = ApiError),
    )
)]
async fn events<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<SessionId>,
    Query(query): Query<EventsQuery>,
) -> Result<Json<EventPage>, Fail> {
    owned(&app, &authed, id).await?;
    session::transcript(
        app.store(),
        authed.who.organization,
        id,
        query.after,
        query.limit,
    )
    .await
    .map(Json)
    .map_err(Fail::Session)
}

#[utoipa::path(
    get, path = "/sessions/{id}/changes", security(("bearer" = [])),
    params(("id" = SessionId, Path, description = "The session")),
    responses(
        (status = 200, description = "What the session changed in each repo against the commit it was cloned at, as of the end of the agent's last turn: commits, files and their unified diffs. Empty before the first report", body = SessionChanges),
        (status = 404, body = ApiError),
    )
)]
async fn changes<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<SessionId>,
) -> Result<Json<SessionChanges>, Fail> {
    owned(&app, &authed, id).await?;
    Ok(Json(SessionChanges {
        repos: app.store().changes(authed.who.organization, id).await?,
    }))
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct CommitQuery {
    /// The repo, when more than one has the commit.
    repo: Option<String>,
}

#[utoipa::path(
    get, path = "/sessions/{id}/changes/commits/{sha}", security(("bearer" = [])),
    params(
        ("id" = SessionId, Path, description = "The session"),
        ("sha" = String, Path, description = "A commit listed in the session's changes"),
        CommitQuery,
    ),
    responses(
        (status = 200, description = "The files of that commit against its parent", body = CommitChanges),
        (status = 404, body = ApiError),
    )
)]
async fn commit_changes<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path((id, sha)): Path<(SessionId, String)>,
    Query(query): Query<CommitQuery>,
) -> Result<Json<CommitChanges>, Fail> {
    owned(&app, &authed, id).await?;
    let found = app
        .store()
        .commit_changes(authed.who.organization, id, query.repo.as_deref(), &sha)
        .await?;
    Ok(Json(found.ok_or(Refusal::NotFound)?))
}

#[utoipa::path(
    get, path = "/sessions/{id}/spend", security(("bearer" = [])),
    params(("id" = SessionId, Path, description = "The session")),
    responses((status = 200, description = "What it spent since it began, against its own cap, by kind", body = SessionSpend), (status = 404, body = ApiError))
)]
async fn spend<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<SessionId>,
) -> Result<Json<SessionSpend>, Fail> {
    Ok(Json(
        budget::session_spend(app.store(), &authed.who, id).await?,
    ))
}

#[utoipa::path(
    put, path = "/sessions/{id}/budget", security(("bearer" = [])),
    params(("id" = SessionId, Path, description = "The session")),
    request_body = SessionBudget,
    responses(
        (status = 204, description = "Set; editors and owners"),
        (status = 409, description = "It would pass the organization's monthly cap", body = ApiError),
    )
)]
async fn set_budget<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<SessionId>,
    Json(body): Json<SessionBudget>,
) -> Result<StatusCode, Fail> {
    budget::set_session_budget(
        app.store(),
        &authed.who,
        app.now(),
        id,
        body.budget_cap_cents,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes<D: Deps + 'static>() -> Router<Arc<App<D>>> {
    Router::new()
        .route(
            "/projects/{id}/sessions",
            post(start::<D>).get(project_sessions::<D>),
        )
        .route("/sessions", get(sessions::<D>))
        .route("/sessions/{id}", get(read::<D>))
        .route("/sessions/{id}/messages", post(message::<D>))
        .route("/sessions/{id}/events", get(events::<D>))
        .route("/sessions/{id}/changes", get(changes::<D>))
        .route(
            "/sessions/{id}/changes/commits/{sha}",
            get(commit_changes::<D>),
        )
        .route("/sessions/{id}/stop", post(stop::<D>))
        .route("/sessions/{id}/spend", get(spend::<D>))
        .route("/sessions/{id}/budget", put(set_budget::<D>))
        .route("/agents", get(agents::<D>))
        .route("/agents/auto", get(auto_agent::<D>))
        .route("/agents/{id}", put(set_agent::<D>))
        .route(
            "/environments",
            get(environments::<D>).post(create_environment::<D>),
        )
        .route("/environments/{id}", put(edit_environment::<D>))
        .route("/environments/{id}/archive", post(archive_environment::<D>))
}
