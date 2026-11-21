//! A terminal in a session's machine, for editors and owners (`docs/V0-contracts.md`). Opening
//! one is a call with a role check and an audit row (`terminal.opened`); the WebSocket that
//! carries it is proved by the one-time token that call returned, not by a cookie, so a page
//! on another site cannot open one. The WebSocket is not in the OpenAPI document.

use std::sync::Arc;

use antilog_core::deps::Deps;
use antilog_core::organization::Action;
use antilog_core::session;
use antilog_types::{ApiError, OpenTerminal, SessionId, TerminalTicket};
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use utoipa::OpenApi;
use uuid::Uuid;

use super::auth::{App, Authed, Fail};
use super::session::{owned, permit};

#[derive(OpenApi)]
#[openapi(paths(open))]
pub struct Doc;

type Shared<D> = State<Arc<App<D>>>;

pub fn routes<D: Deps + 'static>() -> Router<Arc<App<D>>> {
    Router::new()
        .route("/sessions/{id}/terminals", post(open::<D>))
        .route("/terminals/{id}/connect", get(connect::<D>))
}

#[utoipa::path(
    post, path = "/sessions/{id}/terminals", security(("bearer" = [])),
    params(("id" = SessionId, Path, description = "The session")),
    request_body = OpenTerminal,
    responses(
        (status = 201, description = "Asked of the machine; editors and owners. Open the WebSocket at /terminals/{terminal}/connect?token= within a minute", body = TerminalTicket),
        (status = 403, description = "A viewer", body = ApiError),
        (status = 409, description = "The machine is not running (code not_running), or the session has ended", body = ApiError),
    )
)]
async fn open<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<SessionId>,
    Json(size): Json<OpenTerminal>,
) -> Result<(StatusCode, Json<TerminalTicket>), Fail> {
    let row = owned(&app, &authed, id).await?;
    permit(&app, &authed, Action::TerminalOpen, row.project_id).await?;
    let terminal = Uuid::now_v7();
    // Registered first: the runner may connect before this call returns.
    let token = app.terminals.open(terminal, id);
    if let Err(error) = session::open_terminal(
        &*app.deps,
        authed.who.organization,
        id,
        Some(authed.who.member),
        terminal,
        size.cols,
        size.rows,
    )
    .await
    {
        app.terminals.discard(terminal);
        return Err(Fail::Session(error));
    }
    app.terminals.wake();
    let expires_at = app.now() + i64::try_from(crate::terminal::TOKEN_TTL.as_millis()).unwrap_or(0);
    Ok((
        StatusCode::CREATED,
        Json(TerminalTicket {
            terminal,
            token,
            expires_at,
        }),
    ))
}

#[derive(Deserialize)]
struct Token {
    token: String,
}

/// The browser's WebSocket. A wrong or used token, or a terminal that is gone, is a 404 before
/// the upgrade, whoever asks.
async fn connect<D: Deps + 'static>(
    State(app): Shared<D>,
    Path(id): Path<Uuid>,
    Query(query): Query<Token>,
    upgrade: WebSocketUpgrade,
) -> Response {
    let Some(ticket) = app.terminals.browser(id, &query.token) else {
        return (StatusCode::NOT_FOUND, "Not found.").into_response();
    };
    let terminals = app.terminals.clone();
    upgrade.on_upgrade(move |socket| async move {
        terminals.carry(id, socket, ticket, true).await;
    })
}
