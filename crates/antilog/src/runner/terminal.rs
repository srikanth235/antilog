//! A terminal in the machine: OpenCode's PTY, carried over an outbound WebSocket to the server,
//! which relays it to the person's browser. Nothing connects inward. The bytes pass through
//! unread and unrecorded.
//!
//! On the server's socket a binary frame is a person's keys (towards the shell) or the shell's
//! output (towards the browser); a text frame is a JSON control message: `resize` from the
//! browser, `exit` from here when the shell ends.

use std::collections::BTreeMap;
use std::time::Duration;

use antilog_types::SessionId;
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use uuid::Uuid;

use super::api::Server;
use super::opencode::{Api, websocket_url};

/// What one terminal needs.
pub struct Open {
    pub session: SessionId,
    pub terminal: Uuid,
    pub token: String,
    pub cwd: String,
    pub env: BTreeMap<String, String>,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Control {
    Resize { cols: u16, rows: u16 },
}

/// Opens the shell and carries it until either side closes. The shell is removed at the end.
pub async fn run(api: Api, server: Server, open: Open) -> Result<()> {
    install_crypto();
    let pty = api.pty_create(&open.cwd, &open.env).await?;
    let result = carry(&api, &server, &open, &pty).await;
    api.pty_remove(&pty).await.ok();
    result
}

/// TLS needs a process-wide provider before the first `wss` connection; one already installed
/// stays.
fn install_crypto() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .ok();
}

async fn carry(api: &Api, server: &Server, open: &Open, pty: &str) -> Result<()> {
    api.pty_resize(pty, open.cols, open.rows).await?;
    let (mut shell, _) = tokio_tungstenite::connect_async(api.pty_connect_request(pty)?)
        .await
        .context("connecting to the shell")?;
    let mut request = websocket_url(&server.url(&format!(
        "/runner/sessions/{}/terminals/{}",
        open.session, open.terminal
    )))
    .into_client_request()?;
    request.headers_mut().insert(
        "authorization",
        format!("Bearer {}", open.token)
            .parse()
            .context("the runner token is not a header")?,
    );
    let (mut relay, _) = tokio::time::timeout(
        Duration::from_secs(30),
        tokio_tungstenite::connect_async(request),
    )
    .await
    .context("the server did not answer the terminal")?
    .context("connecting to the server")?;
    loop {
        tokio::select! {
            from_server = relay.next() => match from_server {
                Some(Ok(Message::Binary(keys))) => {
                    let text = String::from_utf8_lossy(&keys).into_owned();
                    shell.send(Message::Text(text.into())).await?;
                }
                Some(Ok(Message::Text(text))) => {
                    if let Ok(Control::Resize { cols, rows }) = serde_json::from_str(text.as_str())
                        && cols > 0
                        && rows > 0
                    {
                        api.pty_resize(pty, cols, rows).await.ok();
                    }
                }
                Some(Ok(Message::Close(_))) | None => return Ok(()),
                Some(Ok(_)) => {}
                Some(Err(error)) => return Err(error.into()),
            },
            from_shell = shell.next() => match from_shell {
                Some(Ok(Message::Text(text))) => {
                    relay.send(Message::Binary(text.as_bytes().to_vec().into())).await?;
                }
                // The shell's first frame says where its output is; it is not output.
                Some(Ok(Message::Binary(frame))) if frame.first() == Some(&0) => {}
                Some(Ok(Message::Binary(bytes))) => relay.send(Message::Binary(bytes)).await?,
                // However the shell went, the person is told it is over.
                Some(Ok(Message::Close(_)) | Err(_)) | None => {
                    relay.send(Message::Text(r#"{"type":"exit"}"#.into())).await.ok();
                    relay.close(None).await.ok();
                    return Ok(());
                }
                Some(Ok(_)) => {}
            },
        }
    }
}
