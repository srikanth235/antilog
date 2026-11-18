use std::sync::Arc;

use antilog_core::deps::Deps;
use antilog_core::session::{self, NewSession, SessionError};
use antilog_core::store::SessionStore;
use antilog_core::tools::{Caller, ToolHost, Toolbox, definitions};
use antilog_types::{
    ApiError, ExperimentId, OrganizationId, SendMessage, Session, SessionId, StartSession,
    StepResult, StopSession, ToolCall, ToolDefinition, ToolResult,
};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(start, sessions, read, message, stop, step, tools, call))]
pub struct Doc;

type Failure = (StatusCode, Json<ApiError>);

fn fail(status: StatusCode, code: &str, message: impl ToString) -> Failure {
    (
        status,
        Json(ApiError {
            code: code.to_owned(),
            message: message.to_string(),
            fields: Vec::new(),
        }),
    )
}

fn refuse(error: SessionError) -> Failure {
    match error {
        SessionError::NotFound => fail(
            StatusCode::NOT_FOUND,
            "not_found",
            "No such experiment or session here: check the ids.",
        ),
        SessionError::Ended => fail(
            StatusCode::CONFLICT,
            "session_ended",
            "The session has ended: start a new one.",
        ),
        SessionError::Budget(error) => fail(StatusCode::CONFLICT, "budget_spent", error),
        SessionError::Platform(_) | SessionError::Timer(_) | SessionError::Store(_) => fail(
            StatusCode::SERVICE_UNAVAILABLE,
            "unavailable",
            "A dependency does not answer; retry shortly.",
        ),
    }
}

#[utoipa::path(
    post,
    path = "/organizations/{organization}/experiments/{experiment}/sessions",
    params(("organization" = OrganizationId, Path), ("experiment" = ExperimentId, Path)),
    request_body = StartSession,
    responses((status = 201, body = Session), (status = 404, body = ApiError), (status = 409, body = ApiError))
)]
async fn start<D: Deps + 'static>(
    State(deps): State<Arc<D>>,
    Path((organization, experiment)): Path<(OrganizationId, ExperimentId)>,
    Json(body): Json<StartSession>,
) -> Result<(StatusCode, Json<Session>), Failure> {
    let new = NewSession {
        agent: body.agent,
        prompt: body.prompt,
        repos: body.repos,
        environment: body.environment,
        ..NewSession::default()
    };
    let session = session::start(&*deps, organization, experiment, new)
        .await
        .map_err(refuse)?;
    Ok((StatusCode::CREATED, Json(session)))
}

#[utoipa::path(
    get,
    path = "/organizations/{organization}/experiments/{experiment}/sessions",
    params(("organization" = OrganizationId, Path), ("experiment" = ExperimentId, Path)),
    responses((status = 200, body = Vec<Session>))
)]
async fn sessions<D: Deps + 'static>(
    State(deps): State<Arc<D>>,
    Path((organization, experiment)): Path<(OrganizationId, ExperimentId)>,
) -> Result<Json<Vec<Session>>, Failure> {
    let rows = deps
        .store()
        .sessions(organization, experiment)
        .await
        .map_err(|e| refuse(e.into()))?;
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    path = "/organizations/{organization}/sessions/{session}",
    params(("organization" = OrganizationId, Path), ("session" = SessionId, Path)),
    responses((status = 200, body = Session), (status = 404, body = ApiError))
)]
async fn read<D: Deps + 'static>(
    State(deps): State<Arc<D>>,
    Path((organization, id)): Path<(OrganizationId, SessionId)>,
) -> Result<Json<Session>, Failure> {
    deps.store()
        .session(organization, id)
        .await
        .map_err(|e| refuse(e.into()))?
        .map(Json)
        .ok_or_else(|| refuse(SessionError::NotFound))
}

#[utoipa::path(
    post,
    path = "/organizations/{organization}/sessions/{session}/messages",
    params(("organization" = OrganizationId, Path), ("session" = SessionId, Path)),
    request_body = SendMessage,
    responses((status = 200, body = Session), (status = 409, body = ApiError))
)]
async fn message<D: Deps + 'static>(
    State(deps): State<Arc<D>>,
    Path((organization, id)): Path<(OrganizationId, SessionId)>,
    Json(body): Json<SendMessage>,
) -> Result<Json<Session>, Failure> {
    session::message(&*deps, organization, id, &body.text)
        .await
        .map(Json)
        .map_err(refuse)
}

#[utoipa::path(
    post,
    path = "/organizations/{organization}/sessions/{session}/stop",
    params(("organization" = OrganizationId, Path), ("session" = SessionId, Path)),
    request_body = StopSession,
    responses((status = 200, body = Session), (status = 409, body = ApiError))
)]
async fn stop<D: Deps + 'static>(
    State(deps): State<Arc<D>>,
    Path((organization, id)): Path<(OrganizationId, SessionId)>,
    Json(body): Json<StopSession>,
) -> Result<Json<Session>, Failure> {
    session::stop(&*deps, organization, id, None, &body.by)
        .await
        .map(Json)
        .map_err(refuse)
}

/// The timer's wake-up for a session. At least once, so safe to call again.
#[utoipa::path(
    post,
    path = "/organizations/{organization}/sessions/{session}/step",
    params(("organization" = OrganizationId, Path), ("session" = SessionId, Path)),
    responses((status = 200, body = StepResult), (status = 404, body = ApiError))
)]
async fn step<D: Deps + 'static>(
    State(deps): State<Arc<D>>,
    Path((organization, id)): Path<(OrganizationId, SessionId)>,
) -> Result<Json<StepResult>, Failure> {
    let done = session::step(&*deps, &Toolbox(&*deps), organization, id)
        .await
        .map_err(refuse)?;
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
    post,
    path = "/organizations/{organization}/experiments/{experiment}/tools/{tool}",
    params(("organization" = OrganizationId, Path), ("experiment" = ExperimentId, Path), ("tool" = String, Path)),
    request_body = ToolCall,
    responses((status = 200, body = ToolResult), (status = 404, body = ApiError))
)]
async fn call<D: Deps + 'static>(
    State(deps): State<Arc<D>>,
    Path((organization, experiment, tool)): Path<(OrganizationId, ExperimentId, String)>,
    Json(body): Json<ToolCall>,
) -> Result<Json<ToolResult>, Failure> {
    let session = match body.session {
        Some(id) => deps
            .store()
            .session(organization, id)
            .await
            .map_err(|e| refuse(e.into()))?
            .filter(|s| s.experiment_id == experiment)
            .ok_or_else(|| refuse(SessionError::NotFound))?,
        None => session::open_external(&*deps, organization, experiment)
            .await
            .map_err(refuse)?,
    };
    let caller = Caller {
        organization,
        experiment,
        session: session.id,
        actor: None,
        call: body
            .call
            .unwrap_or_else(|| uuid::Uuid::now_v7().to_string()),
    };
    let host = Toolbox(&*deps);
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

pub fn router<D: Deps + 'static>(deps: Arc<D>) -> Router {
    Router::new()
        .route("/tools", get(tools))
        .route(
            "/organizations/{organization}/experiments/{experiment}/sessions",
            post(start::<D>).get(sessions::<D>),
        )
        .route(
            "/organizations/{organization}/experiments/{experiment}/tools/{tool}",
            post(call::<D>),
        )
        .route(
            "/organizations/{organization}/sessions/{session}",
            get(read::<D>),
        )
        .route(
            "/organizations/{organization}/sessions/{session}/messages",
            post(message::<D>),
        )
        .route(
            "/organizations/{organization}/sessions/{session}/stop",
            post(stop::<D>),
        )
        .route(
            "/organizations/{organization}/sessions/{session}/step",
            post(step::<D>),
        )
        .with_state(deps)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use antilog_core::fakes::{FakeCodeHost, FakeDeps, FakePlatform};
    use antilog_core::seed::experiment;
    use axum::body::Body;
    use axum::http::Request;
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::*;

    async fn send(app: &Router, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    #[tokio::test]
    async fn a_session_is_started_stepped_talked_to_and_stopped() {
        let deps = Arc::new(FakeDeps::new(
            FakePlatform::default(),
            FakeCodeHost::default(),
        ));
        let seed = experiment(&deps.store, 1_000).await;
        let (org, exp) = (seed.organization.id, seed.experiment.id);
        let app = router(deps.clone());
        let base = format!("/organizations/{org}");

        let (status, session) = send(
            &app,
            "POST",
            &format!("{base}/experiments/{exp}/sessions"),
            json!({"agent": "claude", "prompt": "cut p95"}),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{session}");
        let id = session["id"].as_str().unwrap().to_owned();
        let (status, stepped) = send(
            &app,
            "POST",
            &format!("{base}/sessions/{id}/step"),
            json!({}),
        )
        .await;
        assert_eq!(
            (status, stepped["status"].as_str()),
            (StatusCode::OK, Some("working"))
        );
        let (status, _) = send(
            &app,
            "POST",
            &format!("{base}/sessions/{id}/messages"),
            json!({"text": "go"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, stopped) = send(
            &app,
            "POST",
            &format!("{base}/sessions/{id}/stop"),
            json!({"by": "Grace"}),
        )
        .await;
        assert_eq!(
            (status, stopped["note"].as_str()),
            (StatusCode::OK, Some("Stopped by Grace"))
        );
        let (status, ended) = send(
            &app,
            "POST",
            &format!("{base}/sessions/{id}/messages"),
            json!({"text": "again"}),
        )
        .await;
        assert_eq!(
            (status, ended["code"].as_str()),
            (StatusCode::CONFLICT, Some("session_ended"))
        );
        let (status, listed) = send(
            &app,
            "GET",
            &format!("{base}/experiments/{exp}/sessions"),
            json!({}),
        )
        .await;
        assert_eq!(
            (status, listed.as_array().map(Vec::len)),
            (StatusCode::OK, Some(1))
        );
        let (status, _) = send(
            &app,
            "GET",
            &format!("{base}/sessions/{}", SessionId::new()),
            json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn a_tool_call_without_a_session_opens_one_and_says_what_to_do_next() {
        let deps = Arc::new(FakeDeps::new(
            FakePlatform::default(),
            FakeCodeHost::default(),
        ));
        let seed = experiment(&deps.store, 1_000).await;
        let (org, exp) = (seed.organization.id, seed.experiment.id);
        let app = router(deps);
        let uri = format!("/organizations/{org}/experiments/{exp}/tools/measure");
        let call = json!({"input": {"commits": [{"repo": "app", "sha": "x"}], "hypothesis": "h"}});
        let (status, first) = send(&app, "POST", &uri, call).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            first["answer"]["error"]
                .as_str()
                .unwrap()
                .contains("call seal")
        );
        let again = json!({"session": first["session"], "input": {}});
        let (_, second) = send(&app, "POST", &uri, again).await;
        assert_eq!(second["session"], first["session"]);
        let (status, tools) = send(&app, "GET", "/tools", json!({})).await;
        assert_eq!(
            (status, tools.as_array().map(Vec::len)),
            (StatusCode::OK, Some(9))
        );
    }
}
