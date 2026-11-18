//! The code host's webhook. Not in the OpenAPI document: the code host calls it, no client does.

use std::sync::Arc;

use antilog_core::delivery::{self, Handled, PullRequestAction, PullRequestEvent};
use antilog_core::deps::Deps;
use antilog_core::signature;
use antilog_types::judge::WinEventKind;
use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use serde::Deserialize;
use serde_json::json;

use super::auth::{App, Fail};

#[derive(Deserialize)]
struct Payload {
    action: Option<String>,
    number: Option<u64>,
    pull_request: Option<Inner>,
    repository: Option<Repository>,
}

#[derive(Deserialize)]
struct Inner {
    #[serde(default)]
    merged: bool,
}

#[derive(Deserialize)]
struct Repository {
    full_name: String,
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn answer(status: &str) -> Response {
    axum::Json(json!({ "status": status })).into_response()
}

/// A pull request event from the code host. The signature covers the raw body and is checked
/// first, in constant time, before anything is read from it. A delivery id makes a redelivery a no-op.
async fn code_host<D: Deps + 'static>(
    State(app): State<Arc<App<D>>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, Fail> {
    let secret = app
        .settings
        .code_host_webhook_secret
        .as_deref()
        .unwrap_or_default();
    let signature = header(&headers, "x-hub-signature-256").unwrap_or_default();
    if !signature::verify(secret.as_bytes(), &body, signature) {
        return Err(Fail::Unauthenticated);
    }
    match header(&headers, "x-github-event") {
        Some("ping") => return Ok(answer("pong")),
        Some("pull_request") => {}
        _ => return Ok(answer("ignored")),
    }
    let Some(delivery) = header(&headers, "x-github-delivery").filter(|id| !id.is_empty()) else {
        return Ok((StatusCode::BAD_REQUEST, "a delivery id is needed").into_response());
    };
    let Ok(payload) = serde_json::from_slice::<Payload>(&body) else {
        return Ok((
            StatusCode::BAD_REQUEST,
            "the body is not a pull request event",
        )
            .into_response());
    };
    let (Some(number), Some(repository)) = (payload.number, payload.repository) else {
        return Ok((StatusCode::BAD_REQUEST, "the body names no pull request").into_response());
    };
    let merged = payload.pull_request.is_some_and(|inner| inner.merged);
    let action = match payload.action.as_deref() {
        Some("closed") => PullRequestAction::Closed { merged },
        Some("reopened") => PullRequestAction::Reopened,
        _ => PullRequestAction::Other,
    };
    let handled = delivery::handle(
        &*app.deps,
        &PullRequestEvent {
            delivery: delivery.to_owned(),
            repo: repository.full_name,
            number,
            action,
        },
    )
    .await?;
    Ok(answer(match handled {
        Handled::Unknown => "unknown",
        Handled::Duplicate => "duplicate",
        Handled::Ignored => "ignored",
        Handled::Waiting => "waiting",
        Handled::Recorded(WinEventKind::Merged) => "merged",
        Handled::Recorded(WinEventKind::Closed) => "closed",
        Handled::Recorded(_) => "opened",
    }))
}

/// Mounted only when `ANTILOG_CODE_HOST_WEBHOOK_SECRET` is set.
pub fn routes<D: Deps + 'static>() -> Router<Arc<App<D>>> {
    Router::new().route("/webhooks/code-host", post(code_host::<D>))
}
