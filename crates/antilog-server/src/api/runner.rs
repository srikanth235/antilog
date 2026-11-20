//! What an `antilog runner` calls (`docs/V0-contracts.md`). None of it is in the public OpenAPI.
//! `/runner/poll` takes the runner key; every route about one session takes that session's own
//! `Runner` token, which stops working when the session is let go or claimed by another runner.

use std::sync::Arc;

use antilog_core::credentials::{Purpose, SessionCredentials};
use antilog_core::deps::Deps;
use antilog_core::runner::{self, RunnerError};
use antilog_core::store::RunnerStore;
use antilog_types::{ApiError, Items, PollRequest, PollResponse, Release, RunResult, SessionId};
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use subtle::ConstantTimeEq;

use super::auth::App;

/// A release carries an `opencode export`, which can be large.
const EXPORT_LIMIT: usize = 100 * 1024 * 1024;

/// From server config. A runner presents `key`; `core` tells it where everything is.
pub struct RunnerSettings {
    pub key: String,
    pub core: runner::Settings,
}

type Shared<D> = State<Arc<App<D>>>;

pub fn routes<D: Deps + 'static>() -> Router<Arc<App<D>>> {
    Router::new()
        .route("/runner/poll", post(poll::<D>))
        .route("/runner/sessions/{id}/events", post(events::<D>))
        .route("/runner/sessions/{id}/results", post(results::<D>))
        .route("/runner/sessions/{id}/release", post(release::<D>))
        .route("/runner/sessions/{id}/export", get(export::<D>))
        .route(
            "/runner/sessions/{id}/skills/{name}/{version}",
            get(skill::<D>),
        )
        .layer(DefaultBodyLimit::max(EXPORT_LIMIT))
}

fn refuse(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(ApiError {
            code: code.to_owned(),
            message: message.to_owned(),
            fields: Vec::new(),
        }),
    )
        .into_response()
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

/// Why a runner's call was turned away.
enum Fail {
    Unauthenticated,
    Runner(RunnerError),
}

impl<E: Into<RunnerError>> From<E> for Fail {
    fn from(error: E) -> Self {
        Self::Runner(error.into())
    }
}

impl IntoResponse for Fail {
    fn into_response(self) -> Response {
        match self {
            Self::Unauthenticated => unauthenticated(),
            Self::Runner(error) => failed(error),
        }
    }
}

fn unauthenticated() -> Response {
    refuse(
        StatusCode::UNAUTHORIZED,
        "unauthenticated",
        "Present the runner key, or the session's runner token.",
    )
}

fn failed(error: RunnerError) -> Response {
    match error {
        RunnerError::NotFound => refuse(StatusCode::NOT_FOUND, "not_found", "Not found."),
        RunnerError::NotHeld => refuse(
            StatusCode::CONFLICT,
            "not_held",
            "This runner no longer holds the session: stop it.",
        ),
        other => {
            tracing::error!(error = %other, "runner call failed");
            refuse(
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "A dependency does not answer; retry shortly.",
            )
        }
    }
}

/// The runner key, compared in constant time.
fn keyed<D: Deps>(app: &App<D>, headers: &HeaderMap) -> Result<(), Fail> {
    let settings = app.settings.runner.as_ref().ok_or(Fail::Unauthenticated)?;
    let given = bearer(headers).ok_or(Fail::Unauthenticated)?;
    if bool::from(given.as_bytes().ct_eq(settings.key.as_bytes())) {
        Ok(())
    } else {
        Err(Fail::Unauthenticated)
    }
}

/// The holder of the session, once its token is proved to be this session's.
async fn holder<D: Deps>(
    app: &App<D>,
    headers: &HeaderMap,
    session: SessionId,
) -> Result<String, Fail> {
    let token = bearer(headers).ok_or(Fail::Unauthenticated)?;
    let proved = app
        .deps
        .credentials()
        .verify(token, Purpose::Runner)
        .await
        .map_err(RunnerError::from)?;
    if proved != Some(session) {
        return Err(Fail::Unauthenticated);
    }
    let machine = app
        .store()
        .machine(session)
        .await
        .map_err(RunnerError::from)?
        .ok_or(RunnerError::NotFound)?;
    Ok(machine.holder.ok_or(RunnerError::NotHeld)?)
}

async fn poll<D: Deps + 'static>(
    State(app): Shared<D>,
    headers: HeaderMap,
    Json(request): Json<PollRequest>,
) -> Result<Json<PollResponse>, Fail> {
    keyed(&app, &headers)?;
    let settings = app.settings.runner.as_ref().ok_or(Fail::Unauthenticated)?;
    if request.runner.is_empty() || request.runner.len() > 200 {
        return Err(Fail::Unauthenticated);
    }
    Ok(Json(
        runner::poll(&*app.deps, &settings.core, &request).await?,
    ))
}

async fn events<D: Deps + 'static>(
    State(app): Shared<D>,
    headers: HeaderMap,
    Path(session): Path<SessionId>,
    Json(items): Json<Items>,
) -> Result<StatusCode, Fail> {
    let holder = holder(&app, &headers, session).await?;
    runner::ingest(&*app.deps, session, &holder, &items.items).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn results<D: Deps + 'static>(
    State(app): Shared<D>,
    headers: HeaderMap,
    Path(session): Path<SessionId>,
    Json(result): Json<RunResult>,
) -> Result<StatusCode, Fail> {
    let holder = holder(&app, &headers, session).await?;
    runner::result(&*app.deps, session, &holder, &result).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn release<D: Deps + 'static>(
    State(app): Shared<D>,
    headers: HeaderMap,
    Path(session): Path<SessionId>,
    Json(release): Json<Release>,
) -> Result<StatusCode, Fail> {
    let holder = holder(&app, &headers, session).await?;
    runner::release(&*app.deps, session, &holder, &release).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The export of an earlier run, for `opencode import`.
async fn export<D: Deps + 'static>(
    State(app): Shared<D>,
    headers: HeaderMap,
    Path(session): Path<SessionId>,
) -> Result<String, Fail> {
    holder(&app, &headers, session).await?;
    Ok(app
        .store()
        .export(session)
        .await
        .map_err(RunnerError::from)?
        .ok_or(RunnerError::NotFound)?)
}

/// A skill pack at one version: its files as text, from Customize. Only the holding runner may
/// download, and only a version the session's experiment selected.
async fn skill<D: Deps + 'static>(
    State(app): Shared<D>,
    headers: HeaderMap,
    Path((session, name, version)): Path<(SessionId, String, String)>,
) -> Result<Response, Fail> {
    holder(&app, &headers, session).await?;
    let Ok(version) = version.parse::<u32>() else {
        return Ok(unavailable());
    };
    match super::customize::session_skill(app.deps.as_ref(), session, &name, version).await {
        Ok(pack) => Ok(Json(pack).into_response()),
        Err(_) => Ok(unavailable()),
    }
}

fn unavailable() -> Response {
    refuse(
        StatusCode::NOT_FOUND,
        "skill_unavailable",
        "This skill pack version is not available.",
    )
}

impl RunnerSettings {
    /// A runner key turns the queue on. Where the machine reaches the proxy and this server comes
    /// from `ANTILOG_PROXY_URL` and `ANTILOG_PUBLIC_URL`.
    pub fn from_env() -> Option<Self> {
        let key = super::auth::secret("ANTILOG_RUNNER_KEY")?;
        let url = |name: &str| super::auth::setting(name).unwrap_or_default();
        let Some(model) = super::auth::setting("ANTILOG_MODEL") else {
            tracing::error!("ANTILOG_MODEL (provider/model) is not set: the runner queue is off");
            return None;
        };
        let mut core = runner::Settings::new(
            &url("ANTILOG_PROXY_URL"),
            &url("ANTILOG_PUBLIC_URL"),
            &model,
        );
        if let Some(base) = super::auth::setting("ANTILOG_CLONE_BASE") {
            core.clone_base = base;
        }
        Some(Self { key, core })
    }
}
