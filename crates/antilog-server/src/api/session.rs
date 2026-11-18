//! Experiments, sessions and tools: each call is a signed-in member or token, checked by the
//! action layer; handlers decode, call `antilog-core` and encode.

use std::sync::Arc;

use antilog_core::deps::Deps;
use antilog_core::organization::{self as floor, Action, Refusal};
use antilog_core::session::{self, NewSession, SessionError};
use antilog_core::store::judge::JudgeStore;
use antilog_core::store::{OrganizationStore, SessionStore};
use antilog_core::tools::{Caller, ToolHost, Toolbox, definitions};
use antilog_types::judge::{Receipt, TrialId};
use antilog_types::{
    ApiError, Experiment, ExperimentId, NewExperiment, ProjectId, SendMessage, Session, SessionId,
    StartSession, StepResult, ToolCall, ToolDefinition, ToolResult,
};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use utoipa::OpenApi;

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
    step,
    tools,
    call,
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
        SessionError::Platform(_) | SessionError::Timer(_) | SessionError::Store(_) => (
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
    post, path = "/experiments/{id}/sessions", security(("bearer" = [])),
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
    let new = NewSession {
        agent: body.agent,
        prompt: body.prompt,
        repos: body.repos,
        environment: body.environment,
        actor: Some(authed.who.member),
        ..NewSession::default()
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

/// The timer's wake-up for a session. At least once, so safe to call again.
#[utoipa::path(
    post, path = "/sessions/{id}/step", security(("bearer" = [])),
    params(("id" = SessionId, Path, description = "The session")),
    responses((status = 200, body = StepResult), (status = 404, body = ApiError))
)]
async fn step<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<SessionId>,
) -> Result<Json<StepResult>, Fail> {
    let row = owned(&app, &authed, id).await?;
    permit(&app, &authed, Action::SessionStep, row.experiment_id).await?;
    let done = session::step(
        &*app.deps,
        &Toolbox(&*app.deps),
        authed.who.organization,
        id,
    )
    .await
    .map_err(Fail::Session)?;
    Ok(Json(StepResult {
        status: done.status,
        events: u32::try_from(done.events).unwrap_or(u32::MAX),
    }))
}

#[utoipa::path(get, path = "/tools", responses((status = 200, body = Vec<ToolDefinition>)))]
async fn tools() -> Json<Vec<ToolDefinition>> {
    Json(
        definitions(false)
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

/// The scheduler's wake-up for a job, which is a session id. No sign-in: the platform in front
/// blocks `/internal` for the internet, and `ANTILOG_WAKE_SECRET`, when set, is also required.
/// A job that no longer exists is answered 204 so the scheduler stops retrying it.
async fn wake<D: Deps + 'static>(
    State(app): Shared<D>,
    headers: axum::http::HeaderMap,
    Path(job): Path<uuid::Uuid>,
) -> Result<Response, Fail> {
    if let Some(secret) = &app.settings.wake_secret {
        let given = headers
            .get("x-antilog-wake")
            .map_or(&[][..], |value| value.as_bytes());
        if !bool::from(subtle::ConstantTimeEq::ct_eq(given, secret.as_bytes())) {
            return Err(Fail::Unauthenticated);
        }
    }
    let Some(found) = app.store().find_session(SessionId::from_uuid(job)).await? else {
        return Ok(StatusCode::NO_CONTENT.into_response());
    };
    let done = session::step(
        &*app.deps,
        &Toolbox(&*app.deps),
        found.organization_id,
        found.id,
    )
    .await
    .map_err(Fail::Session)?;
    Ok(Json(StepResult {
        status: done.status,
        events: u32::try_from(done.events).unwrap_or(u32::MAX),
    })
    .into_response())
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
        .route("/sessions/{id}/stop", post(stop::<D>))
        .route("/sessions/{id}/step", post(step::<D>))
        .route("/internal/wake/{job}", post(wake::<D>))
        .route("/trials/{id}/receipt", get(receipt::<D>))
}
