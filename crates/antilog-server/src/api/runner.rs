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
fn keyed<D: Deps>(app: &App<D>, headers: &HeaderMap) -> Result<(), Response> {
    let settings = app.settings.runner.as_ref().ok_or_else(unauthenticated)?;
    let given = bearer(headers).ok_or_else(unauthenticated)?;
    if bool::from(given.as_bytes().ct_eq(settings.key.as_bytes())) {
        Ok(())
    } else {
        Err(unauthenticated())
    }
}

/// The holder of the session, once its token is proved to be this session's.
async fn holder<D: Deps>(
    app: &App<D>,
    headers: &HeaderMap,
    session: SessionId,
) -> Result<String, Response> {
    let token = bearer(headers).ok_or_else(unauthenticated)?;
    let proved = app
        .deps
        .credentials()
        .verify(token, Purpose::Runner)
        .await
        .map_err(|error| failed(error.into()))?;
    if proved != Some(session) {
        return Err(unauthenticated());
    }
    let machine = app
        .store()
        .machine(session)
        .await
        .map_err(|error| failed(error.into()))?
        .ok_or_else(|| failed(RunnerError::NotFound))?;
    machine.holder.ok_or_else(|| failed(RunnerError::NotHeld))
}

async fn poll<D: Deps + 'static>(
    State(app): Shared<D>,
    headers: HeaderMap,
    Json(request): Json<PollRequest>,
) -> Response {
    if let Err(response) = keyed(&app, &headers) {
        return response;
    }
    let Some(settings) = app.settings.runner.as_ref() else {
        return unauthenticated();
    };
    if request.runner.is_empty() || request.runner.len() > 200 {
        return refuse(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid",
            "A runner has a name of up to 200 characters.",
        );
    }
    match runner::poll(&*app.deps, &settings.core, &request).await {
        Ok(response) => Json::<PollResponse>(response).into_response(),
        Err(error) => failed(error),
    }
}

async fn events<D: Deps + 'static>(
    State(app): Shared<D>,
    headers: HeaderMap,
    Path(session): Path<SessionId>,
    Json(items): Json<Items>,
) -> Response {
    let holder = match holder(&app, &headers, session).await {
        Ok(holder) => holder,
        Err(response) => return response,
    };
    match runner::ingest(&*app.deps, session, &holder, &items.items).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => failed(error),
    }
}

async fn results<D: Deps + 'static>(
    State(app): Shared<D>,
    headers: HeaderMap,
    Path(session): Path<SessionId>,
    Json(result): Json<RunResult>,
) -> Response {
    let holder = match holder(&app, &headers, session).await {
        Ok(holder) => holder,
        Err(response) => return response,
    };
    match runner::result(&*app.deps, session, &holder, &result).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => failed(error),
    }
}

async fn release<D: Deps + 'static>(
    State(app): Shared<D>,
    headers: HeaderMap,
    Path(session): Path<SessionId>,
    Json(release): Json<Release>,
) -> Response {
    let holder = match holder(&app, &headers, session).await {
        Ok(holder) => holder,
        Err(response) => return response,
    };
    match runner::release(&*app.deps, session, &holder, &release).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => failed(error),
    }
}

/// The export of an earlier run, for `opencode import`.
async fn export<D: Deps + 'static>(
    State(app): Shared<D>,
    headers: HeaderMap,
    Path(session): Path<SessionId>,
) -> Response {
    if let Err(response) = holder(&app, &headers, session).await {
        return response;
    }
    match app.store().export(session).await {
        Ok(Some(body)) => (StatusCode::OK, body).into_response(),
        Ok(None) => failed(RunnerError::NotFound),
        Err(error) => failed(error.into()),
    }
}

/// A skill pack at one version. The content is the skill packs' own (Customize); until it is
/// wired here, no pack is available.
async fn skill<D: Deps + 'static>(
    State(app): Shared<D>,
    headers: HeaderMap,
    Path((session, _name, _version)): Path<(SessionId, String, String)>,
) -> Response {
    if let Err(response) = holder(&app, &headers, session).await {
        return response;
    }
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
        if let Some(skills) = super::auth::setting("ANTILOG_SKILLS") {
            core.skills = skills
                .split(',')
                .filter_map(|pair| pair.trim().split_once('@'))
                .map(|(name, version)| (name.to_owned(), version.to_owned()))
                .collect();
        }
        Some(Self { key, core })
    }
}
