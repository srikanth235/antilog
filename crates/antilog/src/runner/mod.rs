//! `antilog runner`: polls the server's queue over outbound HTTPS, claims a session with a lease,
//! clones its repos, starts OpenCode with the generated config and carries the conversation
//! between OpenCode and the server. Nothing connects inward (`docs/V0.md`).

mod api;
mod machine;
pub mod opencode;
mod session;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use antilog_types::{PollRequest, SessionId};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub use opencode::{Harness, Opencode};

/// One session per machine in v0: OpenCode's managed config is one file for the machine.
const SESSIONS: u32 = 1;

pub struct Config {
    pub server: String,
    pub key: String,
    /// Names this runner to the server.
    pub name: String,
    /// Where sessions are cloned.
    pub workdir: PathBuf,
    /// OpenCode's machine-wide config, which a repo's own `opencode.json` cannot override.
    pub config_path: PathBuf,
    /// Where the skill packs are unpacked.
    pub skills_dir: PathBuf,
    pub poll: Duration,
}

impl Config {
    pub fn new(server: &str, key: &str) -> Self {
        Self {
            server: server.to_owned(),
            key: key.to_owned(),
            name: format!("runner-{}", uuid::Uuid::now_v7().simple()),
            workdir: PathBuf::from("/var/lib/antilog/sessions"),
            config_path: PathBuf::from("/etc/opencode/opencode.json"),
            skills_dir: PathBuf::from("/opt/antilog/skills"),
            poll: Duration::from_secs(5),
        }
    }
}

struct Held {
    commands: mpsc::UnboundedSender<session::Msg>,
    task: JoinHandle<()>,
}

/// Polls until the process is stopped. Each poll renews the leases, takes new work and carries
/// the commands for the sessions held to their tasks.
pub async fn run<H: Harness>(config: Config, harness: H) -> anyhow::Result<()> {
    let config = Arc::new(config);
    let harness = Arc::new(harness);
    let server = api::Server::new(&config.server);
    let done: session::Done = session::Done::default();
    let mut held: HashMap<SessionId, Held> = HashMap::new();
    loop {
        held.retain(|_, session| !session.task.is_finished());
        let finished: Vec<_> = std::mem::take(
            &mut *done
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        let capacity = SESSIONS.saturating_sub(u32::try_from(held.len()).unwrap_or(SESSIONS));
        let request = PollRequest {
            runner: config.name.clone(),
            capacity,
            done: finished.clone(),
            held: held.keys().copied().collect(),
        };
        match server.poll(&config.key, &request).await {
            Ok(answer) => {
                for lost in answer.lost {
                    if let Some(session) = held.remove(&lost) {
                        session.commands.send(session::Msg::Stop).ok();
                    }
                }
                for spec in answer.claims {
                    let (sender, receiver) = mpsc::unbounded_channel();
                    let id = spec.session;
                    tracing::info!(session = %id, "claimed");
                    let task = tokio::spawn(session::drive(
                        config.clone(),
                        harness.clone(),
                        server.clone(),
                        spec,
                        receiver,
                        done.clone(),
                    ));
                    held.insert(
                        id,
                        Held {
                            commands: sender,
                            task,
                        },
                    );
                }
                for envelope in answer.commands {
                    match held.get(&envelope.session) {
                        Some(session) => {
                            session.commands.send(session::Msg::Command(envelope)).ok();
                        }
                        None => {
                            tracing::warn!(session = %envelope.session, "a command for a session this runner does not hold")
                        }
                    }
                }
            }
            Err(error) => {
                // Commands finished but not reported go in the next poll.
                done.lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .extend(finished);
                tracing::warn!(error = %format!("{error:#}"), "poll failed");
            }
        }
        tokio::time::sleep(config.poll).await;
    }
}

#[cfg(test)]
mod tests;
