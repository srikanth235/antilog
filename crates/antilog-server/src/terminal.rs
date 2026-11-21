//! The relay between a person's browser and a terminal in a session's machine. Nothing connects
//! inward to a runner: when a person opens a terminal the server queues a command, the runner
//! opens an outbound WebSocket here, and the server hands frames between that socket and the
//! browser's. The state is in this process's memory (v0 runs one server): a restart closes every
//! terminal, which is what a terminal is allowed to do.
//!
//! Each terminal has two ends, joined by a pair of channels, so either side may connect first.
//! The browser's end is taken with a one-time token minted when the terminal was opened (valid
//! for a minute); the runner's end by the session's own runner token, once. A frame is a
//! WebSocket message: binary is the shell's bytes (to the browser) or a person's keys (to the
//! shell), text is a JSON control message such as `{"type":"resize","cols":80,"rows":24}`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use antilog_types::SessionId;
use axum::extract::ws::{Message, WebSocket};
use subtle::ConstantTimeEq;
use tokio::sync::{Notify, mpsc, watch};
use tokio::time::Instant;
use uuid::Uuid;

/// How long a minted token may wait for the browser to use it.
pub const TOKEN_TTL: Duration = Duration::from_secs(60);
/// A terminal nobody types in or reads from for this long is closed.
pub const IDLE: Duration = Duration::from_secs(30 * 60);
/// How long a browser waits for the machine's runner to connect before giving up.
pub const CONNECT: Duration = Duration::from_secs(30);

const QUEUE: usize = 256;

/// One side of a terminal: what it sends to the other and what it receives from it.
pub struct End {
    tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<Message>,
}

struct Entry {
    session: SessionId,
    token: String,
    created: Instant,
    browser: Option<End>,
    runner: Option<End>,
    /// True once the runner has taken its end.
    attached: watch::Sender<bool>,
    /// Dropped (or set) to close both ends.
    closed: watch::Sender<bool>,
}

/// What a side keeps while it carries frames.
pub struct Ticket {
    end: End,
    closed: watch::Receiver<bool>,
    attached: watch::Receiver<bool>,
}

/// Every open terminal of this server.
pub struct Terminals {
    open: Mutex<HashMap<Uuid, Entry>>,
    /// Woken when a terminal is asked for, so a runner holding its poll answers at once.
    wake: Notify,
    idle: Duration,
    connect: Duration,
    ttl: Duration,
}

impl Default for Terminals {
    fn default() -> Self {
        Self::with_limits(IDLE, CONNECT, TOKEN_TTL)
    }
}

fn token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

impl Terminals {
    pub fn with_limits(idle: Duration, connect: Duration, ttl: Duration) -> Self {
        Self {
            open: Mutex::new(HashMap::new()),
            wake: Notify::new(),
            idle,
            connect,
            ttl,
        }
    }

    fn map(&self) -> std::sync::MutexGuard<'_, HashMap<Uuid, Entry>> {
        self.open.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A terminal the browser has not claimed in time is dropped.
    fn sweep(&self, map: &mut HashMap<Uuid, Entry>) {
        let ttl = self.ttl;
        map.retain(|_, entry| entry.browser.is_none() || entry.created.elapsed() < ttl);
    }

    /// Registers a terminal for `session` and returns the browser's token.
    pub fn open(&self, id: Uuid, session: SessionId) -> String {
        let (to_runner, from_browser) = mpsc::channel(QUEUE);
        let (to_browser, from_runner) = mpsc::channel(QUEUE);
        let token = token();
        let mut map = self.map();
        self.sweep(&mut map);
        map.insert(
            id,
            Entry {
                session,
                token: token.clone(),
                created: Instant::now(),
                browser: Some(End {
                    tx: to_runner,
                    rx: from_runner,
                }),
                runner: Some(End {
                    tx: to_browser,
                    rx: from_browser,
                }),
                attached: watch::channel(false).0,
                closed: watch::channel(false).0,
            },
        );
        token
    }

    /// Asks the runners that are holding a poll to answer now.
    pub fn wake(&self) {
        self.wake.notify_waiters();
    }

    /// A future that completes at the next `wake`; arm it before reading the queue.
    pub fn woken(&self) -> tokio::sync::futures::Notified<'_> {
        self.wake.notified()
    }

    /// Forgets a terminal that was never connected, such as one whose command was refused.
    pub fn discard(&self, id: Uuid) {
        self.map().remove(&id);
    }

    /// The browser's end, once, for the one-time token.
    pub fn browser(&self, id: Uuid, given: &str) -> Option<Ticket> {
        let mut map = self.map();
        self.sweep(&mut map);
        let entry = map.get_mut(&id)?;
        if !bool::from(given.as_bytes().ct_eq(entry.token.as_bytes())) {
            return None;
        }
        let end = entry.browser.take()?;
        Some(Ticket {
            end,
            closed: entry.closed.subscribe(),
            attached: entry.attached.subscribe(),
        })
    }

    /// The runner's end, once, for the session it was asked of.
    pub fn runner(&self, id: Uuid, session: SessionId) -> Option<Ticket> {
        let mut map = self.map();
        let entry = map.get_mut(&id)?;
        if entry.session != session {
            return None;
        }
        let end = entry.runner.take()?;
        entry.attached.send_replace(true);
        Some(Ticket {
            end,
            closed: entry.closed.subscribe(),
            attached: entry.attached.subscribe(),
        })
    }

    /// Closes every terminal of the session: it was stopped or let go.
    pub fn close_session(&self, session: SessionId) {
        self.map().retain(|_, entry| {
            let keep = entry.session != session;
            if !keep {
                entry.closed.send_replace(true);
            }
            keep
        });
    }

    /// Closes the terminal; the pumps forget it when they end.
    fn forget(&self, id: Uuid) {
        self.map().remove(&id);
    }

    /// Carries frames between `socket` and the other end until either closes, the terminal is
    /// closed, or nothing moves for the idle time. `browser` ends also give up when the runner
    /// does not connect in time.
    pub async fn carry(
        self: &Arc<Self>,
        id: Uuid,
        mut socket: WebSocket,
        ticket: Ticket,
        browser: bool,
    ) {
        let Ticket {
            mut end,
            mut closed,
            mut attached,
        } = ticket;
        let mut deadline = Instant::now() + self.idle;
        let connect = Instant::now() + self.connect;
        // A dropped signal is not a close: the other end's buffered frames are still carried.
        let (mut closing, mut watching) = (true, true);
        loop {
            let waiting = browser && watching && !*attached.borrow();
            tokio::select! {
                frame = socket.recv() => match frame {
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                    Some(Ok(message)) => {
                        deadline = Instant::now() + self.idle;
                        if end.tx.send(message).await.is_err() {
                            break;
                        }
                    }
                },
                frame = end.rx.recv() => match frame {
                    Some(message) => {
                        deadline = Instant::now() + self.idle;
                        if socket.send(message).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                },
                changed = closed.changed(), if closing => match changed {
                    Ok(()) if *closed.borrow() => break,
                    Ok(()) => {}
                    Err(_) => closing = false,
                },
                changed = attached.changed(), if waiting => {
                    if changed.is_err() {
                        watching = false;
                    }
                }
                _ = tokio::time::sleep_until(connect), if waiting => break,
                _ = tokio::time::sleep_until(deadline) => break,
            }
        }
        socket.send(Message::Close(None)).await.ok();
        // Dropping the entry drops the other end's channels and its close signal.
        self.forget(id);
    }
}
