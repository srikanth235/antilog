//! Experiments, sessions and tools: each call is a signed-in member or token, checked by the
//! action layer; handlers decode, call `antilog-core` and encode.

use std::sync::Arc;

use antilog_core::deps::Deps;
use antilog_core::organization::{self as floor, Action, Refusal};
use antilog_core::session::{self, NewSession, SessionError};
use antilog_core::store::judge::JudgeStore;
use antilog_core::store::{OrganizationStore, SessionStore};
use antilog_core::tools::{Caller, ToolHost, Toolbox, definitions};
use antilog_core::{agents, evaluations as evals};
use antilog_types::judge::{Receipt, TrialId};
use antilog_types::{
    Agent, ApiError, AutoChoice, Environment, EvaluationTemplate, EventPage, Experiment,
    ExperimentId, NewAgent, NewEvaluationTemplate, NewExperiment, ProjectId, SendMessage, Session,
    SessionId, StartSession, ToolCall, ToolDefinition, ToolResult,
};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use utoipa::{IntoParams, OpenApi};

use super::auth::{App, Authed, Fail};

#[derive(OpenApi)]
#[openapi(paths(
    create_experiment,
    experiments,
    experiment,
    start,
    sessions,
    read,
    message,
    stop,
    events,
    agents,
    auto_agent,
    set_agent,
    environments,
    evaluations,
    evaluation,
    add_evaluation,
    tools,
    call,
    history,
    receipt
))]
pub struct Doc;

type Shared<D> = State<Arc<App<D>>>;

pub fn refuse(error: SessionError) -> Response {
    let (status, code, message) = match error {
        SessionError::NotFound => (
            StatusCode::NOT_FOUND,
            "not_found",
            "No such experiment or session here: check the ids.".to_owned(),
        ),
        SessionError::Ended => (
            StatusCode::CONFLICT,
            "session_ended",
            "The session has ended: start a new one.".to_owned(),
        ),
        SessionError::Budget(error) => (StatusCode::CONFLICT, "budget_spent", error.to_string()),
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
    experiment: ExperimentId,
) -> Result<(), Fail> {
    Ok(floor::permit_session(
        app.store(),
        &authed.who,
        app.now(),
        action,
        experiment,
        experiment,
    )
    .await?)
}

#[utoipa::path(
    post, path = "/projects/{id}/experiments", security(("bearer" = [])),
    params(("id" = ProjectId, Path, description = "The project")),
    request_body = NewExperiment,
    responses(
        (status = 201, description = "Created; editors and owners of the project", body = Experiment),
        (status = 409, description = "The budget would pass a cap", body = ApiError),
    )
)]
async fn create_experiment<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(project): Path<ProjectId>,
    Json(new): Json<NewExperiment>,
) -> Result<(StatusCode, Json<Experiment>), Fail> {
    let made = floor::create_experiment(app.store(), &authed.who, app.now(), project, new).await?;
    Ok((StatusCode::CREATED, Json(made)))
}

#[utoipa::path(
    get, path = "/projects/{id}/experiments", security(("bearer" = [])),
    params(("id" = ProjectId, Path, description = "The project")),
    responses((status = 200, description = "Its experiments, oldest first", body = Vec<Experiment>))
)]
async fn experiments<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(project): Path<ProjectId>,
) -> Result<Json<Vec<Experiment>>, Fail> {
    Ok(Json(
        floor::experiments(app.store(), &authed.who, project).await?,
    ))
}

#[utoipa::path(
    get, path = "/experiments/{id}", security(("bearer" = [])),
    params(("id" = ExperimentId, Path, description = "The experiment")),
    responses((status = 200, body = Experiment), (status = 404, body = ApiError))
)]
async fn experiment<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<ExperimentId>,
) -> Result<Json<Experiment>, Fail> {
    Ok(Json(floor::experiment(app.store(), &authed.who, id).await?))
}

#[utoipa::path(
    post, path = "/experiments/{id}/sessions", operation_id = "start_session", security(("bearer" = [])),
    params(("id" = ExperimentId, Path, description = "The experiment")),
    request_body = StartSession,
    responses((status = 201, body = Session), (status = 404, body = ApiError), (status = 409, body = ApiError))
)]
async fn start<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(experiment): Path<ExperimentId>,
    Json(body): Json<StartSession>,
) -> Result<(StatusCode, Json<Session>), Fail> {
    permit(&app, &authed, Action::SessionStart, experiment).await?;
    let agent = agents::resolve(app.store(), authed.who.organization, &body.agent).await?;
    let new = NewSession {
        agent,
        prompt: body.prompt,
        repos: body.repos,
        environment: body.environment,
        actor: Some(authed.who.member),
    };
    let session = session::start(&*app.deps, authed.who.organization, experiment, new)
        .await
        .map_err(Fail::Session)?;
    Ok((StatusCode::CREATED, Json(session)))
}

#[utoipa::path(
    get, path = "/experiments/{id}/sessions", security(("bearer" = [])),
    params(("id" = ExperimentId, Path, description = "The experiment")),
    responses((status = 200, body = Vec<Session>))
)]
async fn sessions<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(experiment): Path<ExperimentId>,
) -> Result<Json<Vec<Session>>, Fail> {
    floor::experiment(app.store(), &authed.who, experiment).await?;
    Ok(Json(
        app.store()
            .sessions(authed.who.organization, experiment)
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
    permit(&app, &authed, Action::SessionMessage, row.experiment_id).await?;
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
    permit(&app, &authed, Action::SessionStop, row.experiment_id).await?;
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
    get, path = "/evaluations", security(("bearer" = [])),
    responses((status = 200, description = "The organization's evaluation templates, oldest first", body = Vec<EvaluationTemplate>))
)]
async fn evaluations<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
) -> Result<Json<Vec<EvaluationTemplate>>, Fail> {
    Ok(Json(evals::list(app.store(), &authed.who).await?))
}

#[utoipa::path(
    get, path = "/evaluations/{id}", security(("bearer" = [])),
    params(("id" = String, Path, description = "The template's id")),
    responses((status = 200, body = EvaluationTemplate), (status = 404, body = ApiError))
)]
async fn evaluation<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<String>,
) -> Result<Json<EvaluationTemplate>, Fail> {
    Ok(Json(evals::get(app.store(), &authed.who, &id).await?))
}

#[utoipa::path(
    post, path = "/evaluations", security(("bearer" = [])),
    request_body = NewEvaluationTemplate,
    responses(
        (status = 201, description = "Added; editors and owners", body = EvaluationTemplate),
        (status = 409, description = "A template with that name exists", body = ApiError),
    )
)]
async fn add_evaluation<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Json(new): Json<NewEvaluationTemplate>,
) -> Result<(StatusCode, Json<EvaluationTemplate>), Fail> {
    let made = evals::add(app.store(), &authed.who, app.now(), new).await?;
    Ok((StatusCode::CREATED, Json(made)))
}

#[utoipa::path(
    get, path = "/environments", security(("bearer" = [])),
    responses((status = 200, description = "Where a session may run", body = Vec<Environment>))
)]
async fn environments<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
) -> Result<Json<Vec<Environment>>, Fail> {
    Ok(Json(agents::environments(app.store(), &authed.who).await?))
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

#[utoipa::path(get, path = "/tools", responses((status = 200, body = Vec<ToolDefinition>)))]
async fn tools() -> Json<Vec<ToolDefinition>> {
    Json(
        definitions()
            .into_iter()
            .map(|t| ToolDefinition {
                name: t.name,
                description: t.description,
                input_schema: t.schema,
            })
            .collect(),
    )
}

#[utoipa::path(
    post, path = "/experiments/{id}/tools/{tool}", security(("bearer" = [])),
    params(("id" = ExperimentId, Path, description = "The experiment"), ("tool" = String, Path, description = "A tool from /tools")),
    request_body = ToolCall,
    responses((status = 200, body = ToolResult), (status = 404, body = ApiError))
)]
async fn call<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path((experiment, tool)): Path<(ExperimentId, String)>,
    Json(body): Json<ToolCall>,
) -> Result<Json<ToolResult>, Fail> {
    permit(&app, &authed, Action::ToolCall, experiment).await?;
    let organization = authed.who.organization;
    let session = match body.session {
        Some(id) => app
            .store()
            .session(organization, id)
            .await?
            .filter(|s| s.experiment_id == experiment)
            .ok_or(Refusal::NotFound)?,
        None => session::open_external(
            &*app.deps,
            organization,
            experiment,
            Some(authed.who.member),
        )
        .await
        .map_err(Fail::Session)?,
    };
    let caller = Caller {
        organization,
        experiment,
        session: session.id,
        actor: Some(authed.who.member),
        call: body
            .call
            .unwrap_or_else(|| uuid::Uuid::now_v7().to_string()),
    };
    let host = Toolbox(&*app.deps);
    let reply = host.call(&caller, &tool, body.input).await;
    let followup = match &reply.ticket {
        Some(ticket) => Some(host.finish(&caller, ticket).await),
        None => None,
    };
    Ok(Json(ToolResult {
        session: session.id,
        answer: reply.answer,
        followup,
    }))
}

#[derive(serde::Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct HistoryQuery {
    /// Only this session's trials.
    session: Option<SessionId>,
    /// Only this outcome, by word.
    outcome: Option<String>,
    /// At most this many (1 to 50, default 20).
    limit: Option<usize>,
    offset: Option<usize>,
}

#[utoipa::path(
    get, path = "/experiments/{id}/history", security(("bearer" = [])),
    params(("id" = ExperimentId, Path, description = "The experiment"), HistoryQuery),
    responses((status = 200, description = "What the `history` tool answers; a read that opens no session", body = serde_json::Value), (status = 404, body = ApiError))
)]
async fn history<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(experiment): Path<ExperimentId>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, Fail> {
    floor::experiment(app.store(), &authed.who, experiment).await?;
    let input = serde_json::json!({
        "session": query.session, "outcome": query.outcome,
        "limit": query.limit, "offset": query.offset,
    });
    antilog_core::tools::read_history(&*app.deps, authed.who.organization, experiment, input)
        .await
        .map(Json)
        .map_err(|error| {
            Refusal::Unavailable(Box::new(std::io::Error::other(error.to_string()))).into()
        })
}

#[utoipa::path(
    get, path = "/trials/{id}/receipt", security(("bearer" = [])),
    params(("id" = TrialId, Path, description = "A Kept trial that was the win when it ran")),
    responses((status = 200, description = "The judge's signed receipt", body = Receipt), (status = 404, body = ApiError))
)]
async fn receipt<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<TrialId>,
) -> Result<Json<Receipt>, Fail> {
    floor::member_of(app.store(), &authed.who).await?;
    app.deps
        .judge(authed.who.organization)
        .receipt(id)
        .await
        .map_err(|error| Refusal::Unavailable(Box::new(error)))?
        .map(Json)
        .ok_or(Refusal::NotFound.into())
}

pub fn routes<D: Deps + 'static>() -> Router<Arc<App<D>>> {
    Router::new()
        .route("/tools", get(tools))
        .route(
            "/projects/{id}/experiments",
            post(create_experiment::<D>).get(experiments::<D>),
        )
        .route("/experiments/{id}", get(experiment::<D>))
        .route(
            "/experiments/{id}/sessions",
            post(start::<D>).get(sessions::<D>),
        )
        .route("/experiments/{id}/tools/{tool}", post(call::<D>))
        .route("/sessions/{id}", get(read::<D>))
        .route("/sessions/{id}/messages", post(message::<D>))
        .route("/agents", get(agents::<D>))
        .route("/agents/auto", get(auto_agent::<D>))
        .route("/agents/{id}", put(set_agent::<D>))
        .route("/environments", get(environments::<D>))
        .route(
            "/evaluations",
            get(evaluations::<D>).post(add_evaluation::<D>),
        )
        .route("/evaluations/{id}", get(evaluation::<D>))
        .route("/sessions/{id}/events", get(events::<D>))
        .route("/sessions/{id}/stop", post(stop::<D>))
        .route("/experiments/{id}/history", get(history::<D>))
        .route("/trials/{id}/receipt", get(receipt::<D>))
}
