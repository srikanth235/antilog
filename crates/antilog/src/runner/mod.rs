mod api;
mod changes;
mod container;
pub mod opencode;
mod sandbox;
mod session;
mod terminal;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use antilog_types::{Command, PollRequest, SessionId};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub use container::{ContainerConfig, ContainerSandbox, Docker};
pub use opencode::Opencode;
pub use sandbox::Sandbox;

/// Sessions a runner holds at once unless told otherwise, before the host's size bounds it.
const DEFAULT_SESSIONS: u32 = 4;

/// The most sessions a host of this size can hold: one per CPU and per 2 GiB of memory, never
/// fewer than one.
fn bound_sessions(cpus: u32, memory_gib: u32) -> u32 {
    cpus.min(memory_gib / 2).max(1)
}

/// How many sessions to hold when nobody said: four, or fewer on a small host.
pub fn default_sessions() -> u32 {
    let cpus = std::thread::available_parallelism()
        .map_or(1, |count| u32::try_from(count.get()).unwrap_or(u32::MAX));
    let memory_gib = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|text| {
            let line = text.lines().find(|line| line.starts_with("MemTotal:"))?;
            let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
            u32::try_from(kib / 1024 / 1024).ok()
        })
        .unwrap_or(u32::MAX);
    DEFAULT_SESSIONS.min(bound_sessions(cpus, memory_gib))
}

pub struct Config {
    pub server: String,
    pub key: String,
    /// Names this runner to the server.
    pub name: String,
    pub poll: Duration,
    /// Exit once the last session is let go, so the host can be wiped. The server then lets the
    /// runner's organization lock go.
    pub exit_after_last_session: bool,
    /// How long a draining runner waits for its sessions to be let go before it stops them and
    /// exits. A runner drains on SIGTERM or SIGINT, and after its last session with
    /// `exit_after_last_session`. It claims nothing new while it drains.
    pub drain_grace: Duration,
    /// How long an environment's setup script may run before it is stopped. A script that fails
    /// or times out does not fail the session.
    pub setup_timeout: Duration,
    /// How many sessions to hold at once, each in a container of its own.
    pub sessions: u32,
    /// How long a session waiting for a person keeps its machine. After it the runner exports the
    /// session and lets the machine go, and a person's message starts it again. Zero keeps it.
    pub idle_timeout: Duration,
}

impl Config {
    pub fn new(server: &str, key: &str) -> Self {
        Self {
            server: server.to_owned(),
            key: key.to_owned(),
            name: format!("runner-{}", uuid::Uuid::now_v7().simple()),
            poll: Duration::from_secs(5),
            exit_after_last_session: false,
            drain_grace: Duration::from_secs(30),
            setup_timeout: Duration::from_secs(600),
            sessions: 1,
            idle_timeout: Duration::from_secs(600),
        }
    }
}

struct Held {
    commands: mpsc::UnboundedSender<session::Msg>,
    task: JoinHandle<()>,
}

/// Set when the process is told to stop.
fn stop_signal() -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    #[cfg(unix)]
    {
        let flag = flag.clone();
        tokio::spawn(async move {
            use tokio::signal::unix::{SignalKind, signal};
            let (Ok(mut term), Ok(mut int)) = (
                signal(SignalKind::terminate()),
                signal(SignalKind::interrupt()),
            ) else {
                return;
            };
            tokio::select! {
                _ = term.recv() => {}
                _ = int.recv() => {}
            }
            flag.store(true, Ordering::SeqCst);
        });
    }
    flag
}

/// Polls until the process is stopped, or, when draining, until the sessions are let go. Each
/// poll renews the leases, takes new work and carries the commands for the sessions held to their
/// tasks.
pub async fn run<S: Sandbox>(config: Config, opencode: Opencode, sandbox: S) -> anyhow::Result<()> {
    run_until(config, opencode, sandbox, stop_signal()).await
}

/// `run`, with the flag that starts a drain given by the caller.
async fn run_until<S: Sandbox>(
    config: Config,
    opencode: Opencode,
    sandbox: S,
    stop: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let config = Arc::new(config);
    let opencode = Arc::new(opencode);
    let sandbox = Arc::new(sandbox);
    let server = api::Server::new(&config.server);
    let done: session::Done = session::Done::default();
    let mut held: HashMap<SessionId, Held> = HashMap::new();
    let mut claimed_any = false;
    let mut draining: Option<tokio::time::Instant> = None;
    let mut again = false;
    loop {
        held.retain(|_, session| !session.task.is_finished());
        if draining.is_none()
            && (stop.load(Ordering::SeqCst)
                || (config.exit_after_last_session && claimed_any && held.is_empty()))
        {
            tracing::info!(held = held.len(), "draining: claiming nothing more");
            draining = Some(tokio::time::Instant::now());
        }
        if let Some(since) = draining
            && !held.is_empty()
            && since.elapsed() >= config.drain_grace
        {
            tracing::warn!(
                held = held.len(),
                "the drain grace is over: stopping the sessions"
            );
            for session in held.values() {
                session.commands.send(session::Msg::Stop).ok();
            }
            for (_, session) in held.drain() {
                tokio::time::timeout(Duration::from_secs(30), session.task)
                    .await
                    .ok();
            }
        }
        let finished: Vec<_> = std::mem::take(
            &mut *done
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        let leaving = draining.is_some() && held.is_empty();
        let capacity = if draining.is_some() {
            0
        } else {
            config
                .sessions
                .saturating_sub(u32::try_from(held.len()).unwrap_or(u32::MAX))
        };
        let request = PollRequest {
            runner: config.name.clone(),
            capacity,
            done: finished.clone(),
            held: held.keys().copied().collect(),
            exiting: leaving,
        };
        match server.poll(&config.key, &request).await {
            Ok(answer) => {
                if leaving {
                    tracing::info!(
                        "exiting: the server has let this runner's organization lock go"
                    );
                    return Ok(());
                }
                for lost in answer.lost {
                    if let Some(session) = held.remove(&lost) {
                        session.commands.send(session::Msg::Stop).ok();
                    }
                }
                for spec in answer.claims {
                    let (sender, receiver) = mpsc::unbounded_channel();
                    let id = spec.session;
                    tracing::info!(session = %id, "claimed");
                    claimed_any = true;
                    let task = tokio::spawn(session::drive(
                        sandbox.clone(),
                        opencode.clone(),
                        server.clone(),
                        spec,
                        receiver,
                        done.clone(),
                        session::Timing {
                            setup: config.setup_timeout,
                            idle: config.idle_timeout,
                        },
                    ));
                    held.insert(
                        id,
                        Held {
                            commands: sender,
                            task,
                        },
                    );
                }
                // A person is waiting for a terminal: ask again at once, not after the sleep.
                again = answer
                    .commands
                    .iter()
                    .any(|envelope| matches!(envelope.command, Command::OpenTerminal { .. }));
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
                if leaving {
                    // The lock stays with this runner's name, which a new process does not reuse.
                    tracing::warn!(error = %format!("{error:#}"), "could not tell the server this runner is leaving");
                    return Ok(());
                }
                // Commands finished but not reported go in the next poll.
                done.lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .extend(finished);
                tracing::warn!(error = %format!("{error:#}"), "poll failed");
            }
        }
        if !std::mem::take(&mut again) {
            tokio::time::sleep(config.poll).await;
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod sizing {
    use super::bound_sessions;

    #[test]
    fn a_small_host_holds_fewer_sessions() {
        assert_eq!(bound_sessions(16, 64), 16);
        assert_eq!(bound_sessions(2, 64), 2);
        assert_eq!(bound_sessions(16, 6), 3);
        assert_eq!(bound_sessions(1, 1), 1);
        assert_eq!(bound_sessions(0, 0), 1);
    }
}
