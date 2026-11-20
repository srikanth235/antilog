//! The model gateway. A session's machine calls `/anthropic/v1/*`, `/openai/v1/*` or `/opencode/v1/*` with its
//! short-lived token where a key would go. The gateway checks the token, checks the experiment's
//! budget, swaps in the organization's key, forwards the call as it came, and books spend from the
//! usage the provider reports. A key is never logged and never reaches a session.

use std::io;

use antilog_core::budget;
use antilog_core::credentials::Purpose;
use antilog_core::organization::Refusal;
use antilog_core::spend::record_usage;
use antilog_core::store::GatewayScope;
use antilog_types::{ModelProvider, SessionId};
use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::{self, HeaderName};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use bytes::Bytes;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::price;
use crate::usage::{EventWatcher, Usage};
use crate::{Backend, Proxy, Shared, Time, Tokens as TokenStore};

/// The largest request a session may send through: a long conversation with files in it.
const MAX_REQUEST: usize = 64 << 20;

/// Headers that belong to one hop, not to the call.
const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

/// What the session sent that the gateway replaces or drops. The token goes; the provider's own
/// key header is written afresh; the body length and encoding are settled by this hop.
const NOT_FORWARDED: &[&str] = &[
    "host",
    "content-length",
    "accept-encoding",
    "authorization",
    "x-api-key",
];

fn hop_by_hop(name: &HeaderName) -> bool {
    HOP_BY_HOP.contains(&name.as_str())
}

/// An error in the provider's own shape, so the agent's SDK reads it as it would the provider's.
fn provider_error(
    provider: ModelProvider,
    status: StatusCode,
    kind: &str,
    message: &str,
) -> Response {
    let body = match provider {
        ModelProvider::Anthropic => {
            json!({"type": "error", "error": {"type": kind, "message": message}})
        }
        ModelProvider::Openai | ModelProvider::Opencode => {
            json!({"error": {"message": message, "type": kind, "param": null, "code": kind}})
        }
    };
    (status, axum::Json(body)).into_response()
}

fn token_of(provider: ModelProvider, headers: &HeaderMap) -> Option<String> {
    match provider {
        ModelProvider::Anthropic => headers.get("x-api-key")?.to_str().ok().map(str::to_owned),
        ModelProvider::Openai | ModelProvider::Opencode => headers
            .get(header::AUTHORIZATION)?
            .to_str()
            .ok()?
            .strip_prefix("Bearer ")
            .map(|token| token.trim().to_owned()),
    }
}

/// Calls whose reply carries a bill. A model list or a token count is free and passes unmetered.
fn is_metered(method: &Method, path: &str) -> bool {
    *method == Method::POST
        && [
            "/messages",
            "/chat/completions",
            "/responses",
            "/completions",
        ]
        .iter()
        .any(|tail| path.ends_with(tail))
}

/// OpenAI sends a chat stream's usage only when asked. Without it the stream could not be metered
/// and would cost nothing, so the one field is turned on. Nothing else in the body changes, and
/// every other request goes through byte for byte.
fn ask_for_stream_usage(path: &str, body: &Bytes) -> Option<Bytes> {
    if !path.ends_with("/chat/completions") {
        return None;
    }
    let mut value: Value = serde_json::from_slice(body).ok()?;
    if value.get("stream")?.as_bool() != Some(true) {
        return None;
    }
    let options = value
        .as_object_mut()?
        .entry("stream_options")
        .or_insert_with(|| json!({}));
    let options = options.as_object_mut()?;
    if options.get("include_usage").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    options.insert("include_usage".to_owned(), Value::Bool(true));
    serde_json::to_vec(&value).ok().map(Bytes::from)
}

struct Call {
    provider: ModelProvider,
    session: SessionId,
    scope: GatewayScope,
}

async fn handle<S, C, K>(
    proxy: Shared<S, C, K>,
    provider: ModelProvider,
    request: Request,
) -> Response
where
    S: Backend,
    C: TokenStore,
    K: Time,
{
    let (parts, body) = request.into_parts();

    let Some(token) = token_of(provider, &parts.headers) else {
        return provider_error(
            provider,
            StatusCode::UNAUTHORIZED,
            "authentication_error",
            "Send the session's Antilog token as the API key.",
        );
    };
    let session = match proxy.credentials.verify(&token, Purpose::Model).await {
        Ok(Some(session)) => session,
        Ok(None) => {
            tracing::warn!(
                provider = provider.as_str(),
                "model call refused: no valid session credential"
            );
            return provider_error(
                provider,
                StatusCode::UNAUTHORIZED,
                "authentication_error",
                "This session's Antilog token is not valid, or has expired.",
            );
        }
        Err(error) => {
            tracing::error!(%error, "the credential store is unavailable");
            return provider_error(
                provider,
                StatusCode::SERVICE_UNAVAILABLE,
                "api_error",
                "Antilog cannot check this session's token right now. Retry shortly.",
            );
        }
    };
    let scope = match proxy.store.gateway_scope(session).await {
        Ok(Some(scope)) => scope,
        Ok(None) => {
            return provider_error(
                provider,
                StatusCode::UNAUTHORIZED,
                "authentication_error",
                "This session's Antilog token is not valid, or has expired.",
            );
        }
        Err(error) => {
            tracing::error!(%session, %error, "the session could not be looked up");
            return provider_error(
                provider,
                StatusCode::SERVICE_UNAVAILABLE,
                "api_error",
                "Antilog cannot look up this session right now. Retry shortly.",
            );
        }
    };
    let call = Call {
        provider,
        session,
        scope,
    };

    let now = proxy.clock.now().as_millisecond();
    match budget::ensure_room(&proxy.store, scope.organization, scope.experiment, now).await {
        Ok(_) => {}
        Err(Refusal::Spent(spent)) => {
            tracing::info!(%session, provider = provider.as_str(), %spent, "model call refused: budget spent");
            let (kind, status) = match provider {
                ModelProvider::Anthropic => ("billing_error", StatusCode::PAYMENT_REQUIRED),
                ModelProvider::Openai | ModelProvider::Opencode => {
                    ("insufficient_quota", StatusCode::PAYMENT_REQUIRED)
                }
            };
            return provider_error(
                provider,
                status,
                kind,
                &format!(
                    "Antilog: {spent}. Stop calling the model. If more is needed, call request_budget with an amount and a reason."
                ),
            );
        }
        Err(error) => {
            tracing::error!(%session, %error, "the budget could not be checked");
            return provider_error(
                provider,
                StatusCode::SERVICE_UNAVAILABLE,
                "api_error",
                "Antilog cannot check this experiment's budget right now. Retry shortly.",
            );
        }
    }

    let key = match proxy.store.model_key(scope.organization, provider).await {
        Ok(Some(key)) => key,
        Ok(None) => {
            tracing::warn!(%session, provider = provider.as_str(), "model call refused: the organization has no key");
            return provider_error(
                provider,
                StatusCode::FORBIDDEN,
                "permission_error",
                &format!(
                    "Antilog: this organization has no {} key. An owner adds one in Settings.",
                    provider.as_str()
                ),
            );
        }
        Err(error) => {
            tracing::error!(%session, %error, "the model key could not be read");
            return provider_error(
                provider,
                StatusCode::SERVICE_UNAVAILABLE,
                "api_error",
                "Antilog cannot read this organization's model key right now.",
            );
        }
    };

    let prefix = format!("/{}", provider.as_str());
    let path = parts
        .uri
        .path()
        .strip_prefix(prefix.as_str())
        .unwrap_or(parts.uri.path())
        .to_owned();
    let base = match provider {
        ModelProvider::Anthropic => &proxy.upstreams.anthropic,
        ModelProvider::Openai => &proxy.upstreams.openai,
        ModelProvider::Opencode => &proxy.upstreams.opencode,
    };
    let url = match parts.uri.query() {
        Some(query) => format!("{}{path}?{query}", base.trim_end_matches('/')),
        None => format!("{}{path}", base.trim_end_matches('/')),
    };

    let body = match axum::body::to_bytes(body, MAX_REQUEST).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return provider_error(
                provider,
                StatusCode::PAYLOAD_TOO_LARGE,
                "request_too_large",
                "The request is too large for the Antilog gateway.",
            );
        }
    };
    let body = ask_for_stream_usage(&path, &body).unwrap_or(body);

    let mut upstream = proxy.http.request(parts.method.clone(), &url);
    for (name, value) in &parts.headers {
        if !hop_by_hop(name) && !NOT_FORWARDED.contains(&name.as_str()) {
            upstream = upstream.header(name, value);
        }
    }
    upstream = upstream.header(header::ACCEPT_ENCODING, "identity");
    upstream = match provider {
        ModelProvider::Anthropic => upstream.header("x-api-key", key.expose()),
        ModelProvider::Openai | ModelProvider::Opencode => {
            upstream.header(header::AUTHORIZATION, format!("Bearer {}", key.expose()))
        }
    };
    let upstream = match upstream.body(body).send().await {
        Ok(response) => response,
        Err(error) => {
            // The error can quote the URL, never a header.
            tracing::warn!(%session, provider = provider.as_str(), %error, "the provider did not answer");
            return provider_error(
                provider,
                StatusCode::BAD_GATEWAY,
                "api_error",
                "The model provider did not answer.",
            );
        }
    };

    let status = upstream.status();
    let mut response = Response::builder().status(status);
    for (name, value) in upstream.headers() {
        if !hop_by_hop(name) && name != header::CONTENT_LENGTH && name != header::CONTENT_ENCODING {
            response = response.header(name, value);
        }
    }
    let metered = status.is_success() && is_metered(&parts.method, &path);
    let streaming = upstream
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|kind| kind.starts_with("text/event-stream"));

    let body = if !metered {
        match upstream.bytes().await {
            Ok(bytes) => Body::from(bytes),
            Err(error) => {
                tracing::warn!(%session, %error, "the provider's reply broke off");
                return provider_error(
                    provider,
                    StatusCode::BAD_GATEWAY,
                    "api_error",
                    "The model provider's reply broke off.",
                );
            }
        }
    } else if streaming {
        let (tx, rx) = mpsc::channel::<Result<Bytes, io::Error>>(16);
        tokio::spawn(pump(proxy, call, upstream, tx));
        Body::from_stream(ReceiverStream::new(rx))
    } else {
        match upstream.bytes().await {
            Ok(bytes) => {
                let mut usage = Usage::default();
                if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
                    usage.absorb(provider, &value);
                }
                book(&proxy, &call, &usage, Outcome::Complete).await;
                Body::from(bytes)
            }
            Err(error) => {
                tracing::warn!(%session, %error, "the provider's reply broke off");
                return provider_error(
                    provider,
                    StatusCode::BAD_GATEWAY,
                    "api_error",
                    "The model provider's reply broke off.",
                );
            }
        }
    };
    response
        .body(body)
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
}

#[derive(Debug, Clone, Copy)]
enum Outcome {
    Complete,
    /// The session stopped reading before the stream ended.
    Aborted,
    /// The provider's stream broke off.
    Broken,
}

/// Relays a stream chunk by chunk, watching it for usage, and books when it ends, however it ends.
async fn pump<S, C, K>(
    proxy: Shared<S, C, K>,
    call: Call,
    mut upstream: reqwest::Response,
    tx: mpsc::Sender<Result<Bytes, io::Error>>,
) where
    S: Backend,
    C: TokenStore,
    K: Time,
{
    let mut usage = Usage::default();
    let mut watcher = EventWatcher::default();
    let outcome = loop {
        tokio::select! {
            () = tx.closed() => break Outcome::Aborted,
            next = upstream.chunk() => match next {
                Ok(Some(chunk)) => {
                    watcher.feed(call.provider, &chunk, &mut usage);
                    if tx.send(Ok(chunk)).await.is_err() {
                        break Outcome::Aborted;
                    }
                }
                Ok(None) => break Outcome::Complete,
                Err(error) => {
                    tracing::warn!(session = %call.session, %error, "the provider's stream broke off");
                    let _ = tx.send(Err(io::Error::other("the provider's stream broke off"))).await;
                    break Outcome::Broken;
                }
            },
        }
    };
    book(&proxy, &call, &usage, outcome).await;
}

/// One spend row from what a reply reported. Whole cents, rounded up. A model the table does not
/// know is booked at the highest price it has, and that is logged.
async fn book<S, C, K>(proxy: &Proxy<S, C, K>, call: &Call, usage: &Usage, outcome: Outcome)
where
    S: Backend,
    C: TokenStore,
    K: Time,
{
    let session = call.session;
    let provider = call.provider.as_str();
    if !usage.seen {
        tracing::warn!(%session, provider, ?outcome, "the reply reported no usage, so nothing is booked");
        return;
    }
    let (price, known) = price::lookup(call.provider, usage.model.as_deref(), &proxy.zen);
    if !known {
        tracing::warn!(%session, provider, model = ?usage.model, "unknown model: booked at the highest known price");
    }
    let cents = i64::try_from(price::cents(price, usage.tokens)).unwrap_or(i64::MAX);
    let tokens = i64::try_from(usage.tokens.total()).unwrap_or(i64::MAX);
    if matches!(outcome, Outcome::Aborted | Outcome::Broken) {
        tracing::warn!(%session, provider, ?outcome, tokens, cents, "stream ended early: booked what was seen");
    }
    let result = record_usage(
        &proxy.store,
        call.scope.organization,
        session,
        cents,
        tokens,
        proxy.clock.now().as_millisecond(),
        &uuid::Uuid::new_v4().to_string(),
    )
    .await;
    match result {
        Ok(()) => tracing::info!(%session, provider, model = ?usage.model, tokens, cents, "booked"),
        Err(error) => {
            tracing::error!(%session, provider, %error, tokens, cents, "spend could not be booked")
        }
    }
}

/// The gateway's routes: one per provider, and a health check.
pub fn router<S, C, K>(proxy: Shared<S, C, K>) -> Router
where
    S: Backend,
    C: TokenStore,
    K: Time,
{
    async fn anthropic<S: Backend, C: TokenStore, K: Time>(
        State(proxy): State<Shared<S, C, K>>,
        request: Request,
    ) -> Response {
        handle(proxy, ModelProvider::Anthropic, request).await
    }
    async fn openai<S: Backend, C: TokenStore, K: Time>(
        State(proxy): State<Shared<S, C, K>>,
        request: Request,
    ) -> Response {
        handle(proxy, ModelProvider::Openai, request).await
    }
    async fn opencode<S: Backend, C: TokenStore, K: Time>(
        State(proxy): State<Shared<S, C, K>>,
        request: Request,
    ) -> Response {
        handle(proxy, ModelProvider::Opencode, request).await
    }
    Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }))
        .route("/anthropic/v1/{*rest}", any(anthropic::<S, C, K>))
        .route("/openai/v1/{*rest}", any(openai::<S, C, K>))
        .route("/opencode/v1/{*rest}", any(opencode::<S, C, K>))
        .route("/mcp/{*rest}", any(crate::mcp::forward::<S, C, K>))
        .with_state(proxy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_chat_stream_that_did_not_ask_for_usage_is_changed() {
        let ask =
            |path: &str, body: &str| ask_for_stream_usage(path, &Bytes::from(body.to_owned()));
        let changed = ask("/v1/chat/completions", r#"{"model":"m","stream":true}"#).unwrap();
        let value: Value = serde_json::from_slice(&changed).unwrap();
        assert_eq!(value["stream_options"]["include_usage"], true);
        assert_eq!(value["model"], "m");
        assert!(ask("/v1/chat/completions", r#"{"stream":false}"#).is_none());
        assert!(
            ask(
                "/v1/chat/completions",
                r#"{"stream":true,"stream_options":{"include_usage":true}}"#
            )
            .is_none()
        );
        assert!(ask("/v1/responses", r#"{"stream":true}"#).is_none());
        assert!(ask("/v1/chat/completions", "not json").is_none());
    }

    #[test]
    fn only_calls_with_a_bill_are_metered() {
        assert!(is_metered(&Method::POST, "/v1/messages"));
        assert!(is_metered(&Method::POST, "/v1/chat/completions"));
        assert!(is_metered(&Method::POST, "/v1/responses"));
        assert!(!is_metered(&Method::POST, "/v1/messages/count_tokens"));
        assert!(!is_metered(&Method::GET, "/v1/models"));
    }
}
