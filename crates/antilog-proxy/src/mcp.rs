//! The MCP pass-through. A session's machine reaches only the proxy, so its MCP calls come here and
//! go on to the server unchanged (`ANTILOG_PROXY_SERVER_URL`). The server checks the session's
//! MCP token; the proxy adds nothing and keeps nothing. Answers stream back as they arrive, since
//! MCP over streamable HTTP can answer with server-sent events.

use std::io;

use axum::body::{Body, Bytes};
use axum::extract::{Request, State};
use axum::http::{HeaderName, StatusCode, header};
use axum::response::{IntoResponse, Response};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::{Backend, Shared, Time, Tokens};

/// The largest MCP request body forwarded.
const MAX_REQUEST: usize = 4 << 20;

/// Connection-level headers, and the ones the proxy sets itself.
fn passed(name: &HeaderName) -> bool {
    !matches!(
        name.as_str(),
        "host"
            | "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "proxy-connection"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "content-length"
    )
}

pub async fn forward<S: Backend, C: Tokens, K: Time>(
    State(proxy): State<Shared<S, C, K>>,
    request: Request,
) -> Response {
    let Some(server) = proxy.upstreams.server.as_deref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let (parts, body) = request.into_parts();
    let path = parts.uri.path();
    let url = match parts.uri.query() {
        Some(query) => format!("{}{path}?{query}", server.trim_end_matches('/')),
        None => format!("{}{path}", server.trim_end_matches('/')),
    };
    let Ok(body) = axum::body::to_bytes(body, MAX_REQUEST).await else {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    };
    let mut call = proxy.http.request(parts.method.clone(), &url);
    for (name, value) in &parts.headers {
        if passed(name) {
            call = call.header(name, value);
        }
    }
    let mut upstream = match call.body(body).send().await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(%error, "mcp: the server did not answer");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };
    let mut response = Response::builder().status(upstream.status());
    for (name, value) in upstream.headers() {
        if passed(name) && name != header::CONTENT_ENCODING {
            response = response.header(name, value);
        }
    }
    let (tx, rx) = mpsc::channel::<Result<Bytes, io::Error>>(16);
    tokio::spawn(async move {
        loop {
            match upstream.chunk().await {
                Ok(Some(chunk)) => {
                    if tx.send(Ok(chunk)).await.is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    let _ = tx.send(Err(io::Error::other(error))).await;
                    break;
                }
            }
        }
    });
    response
        .body(Body::from_stream(ReceiverStream::new(rx)))
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
}
