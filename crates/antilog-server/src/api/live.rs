//! Live updates: one server-sent event stream per signed-in member.
//!
//! The stream carries hints, never data (`antilog_types::LiveEvent`): a conversation item was
//! added, a session changed status, the member's cards changed. The client refetches what it
//! shows. What a member may hear is exactly what they may read through the REST routes: every
//! read goes through the organization's row-level security (the store is scoped to it) and
//! the same permission check as `GET /sessions/{id}/events`.
//!
//! Events are the rows of `session_events`, whose `seq` is the SSE id. A reconnect sends
//! `Last-Event-ID` and the stream replays from that `seq`; a client too far behind (or ahead
//! of a rebuilt ledger) gets `resync`. An open stream polls Postgres rather than
//! holding a `LISTEN/NOTIFY` connection. It ends after [`Options::max_age`], well inside the
//! idle timeout of any proxy in front, and the browser reconnects.

use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use antilog_core::approval;
use antilog_core::auth;
use antilog_core::deps::Deps;
use antilog_core::organization::{self as floor, Principal};
use antilog_core::store::SessionStore;
use antilog_types::{ApiError, ExperimentId, LiveEvent, SessionId, SessionStatus};
use axum::Router;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};
use utoipa::OpenApi;

use super::auth::{App, Authed, Fail};

#[derive(OpenApi)]
#[openapi(paths(stream), components(schemas(LiveEvent)))]
pub struct Doc;

/// How an open stream paces itself.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Between looks for new conversation items.
    pub poll: Duration,
    /// Every this many polls, look at sessions and cards too.
    pub slow_every: u32,
    /// The caller's credential and membership are checked again this often; the stream ends
    /// when they no longer hold.
    pub recheck: Duration,
    /// The stream ends after this long and the browser reconnects.
    pub max_age: Duration,
    /// A client further behind than this many items gets `resync`.
    pub max_replay: i64,
    /// A comment line this often keeps proxies from cutting a quiet stream.
    pub keep_alive: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            poll: Duration::from_secs(1),
            slow_every: 2,
            recheck: Duration::from_secs(30),
            max_age: Duration::from_secs(240),
            max_replay: 2000,
            keep_alive: Duration::from_secs(10),
        }
    }
}

const BATCH: u32 = 200;

type Frame = Result<Event, Infallible>;

fn frame(event: &LiveEvent) -> Frame {
    let mut out = Event::default().event(event.name());
    if let Some(id) = event.id() {
        out = out.id(id.to_string());
    }
    Ok(out
        .json_data(event)
        .unwrap_or_else(|_| Event::default().event("resync")))
}

/// What one stream remembers between polls.
struct Watch {
    who: Principal,
    cursor: i64,
    /// Whether this member may read each session, by session: its experiment, or `None`.
    readable: HashMap<SessionId, Option<ExperimentId>>,
    statuses: HashMap<SessionId, SessionStatus>,
    changed_since: i64,
    cards: HashSet<antilog_types::ApprovalId>,
}

impl Watch {
    async fn experiment_of<D: Deps + 'static>(
        &mut self,
        app: &App<D>,
        session: SessionId,
    ) -> Option<ExperimentId> {
        if let Some(known) = self.readable.get(&session) {
            return *known;
        }
        let found = match app.store().session(self.who.organization, session).await {
            Ok(Some(row)) => floor::experiment(app.store(), &self.who, row.experiment_id)
                .await
                .ok()
                .map(|_| row.experiment_id),
            _ => None,
        };
        self.readable.insert(session, found);
        found
    }

    /// New conversation items, in order, for sessions this member may read.
    async fn messages<D: Deps + 'static>(&mut self, app: &App<D>) -> Vec<LiveEvent> {
        let Ok(marks) = app
            .store()
            .event_marks(self.who.organization, self.cursor, BATCH)
            .await
        else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (seq, session) in marks {
            self.cursor = seq;
            if let Some(experiment_id) = self.experiment_of(app, session).await {
                out.push(LiveEvent::Message {
                    session_id: session,
                    experiment_id,
                    seq,
                });
            }
        }
        out
    }

    async fn status_changes<D: Deps + 'static>(&mut self, app: &App<D>) -> Vec<LiveEvent> {
        let Ok(rows) = app
            .store()
            .sessions_changed(self.who.organization, self.changed_since, BATCH)
            .await
        else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for row in rows {
            self.changed_since = self.changed_since.max(row.updated_at);
            if self.statuses.get(&row.id) == Some(&row.status) {
                continue;
            }
            self.statuses.insert(row.id, row.status);
            if self.experiment_of(app, row.id).await.is_some() {
                out.push(LiveEvent::Session {
                    session_id: row.id,
                    experiment_id: row.experiment_id,
                    status: row.status,
                });
            }
        }
        out
    }

    async fn card_changes<D: Deps + 'static>(&mut self, app: &App<D>) -> Option<LiveEvent> {
        let waiting: HashSet<_> = approval::pending(app.store(), &self.who)
            .await
            .ok()?
            .into_iter()
            .map(|card| card.id)
            .collect();
        if waiting == self.cards {
            return None;
        }
        self.cards = waiting;
        Some(LiveEvent::Cards {
            pending: u32::try_from(self.cards.len()).unwrap_or(u32::MAX),
        })
    }
}

/// Opens a stream for `who`, resuming after `last` (a `Last-Event-ID`) when given. Events
/// arrive on the receiver until the stream ends or the receiver is dropped.
pub fn open<D: Deps + 'static>(
    app: Arc<App<D>>,
    who: Principal,
    secret: String,
    last: Option<i64>,
    options: Options,
) -> mpsc::Receiver<LiveEvent> {
    let (tx, rx) = mpsc::channel(64);
    tokio::spawn(async move {
        let latest = app
            .store()
            .latest_event_seq(who.organization)
            .await
            .unwrap_or(0);
        let (cursor, resync) = match last {
            Some(last) if last > latest || latest - last > options.max_replay => (latest, true),
            Some(last) => (last.max(0), false),
            None => (latest, false),
        };
        let mut watch = Watch {
            who,
            cursor,
            readable: HashMap::new(),
            statuses: HashMap::new(),
            changed_since: app.now(),
            cards: HashSet::new(),
        };
        // Learn the cards now waiting, so only a change is announced.
        watch.card_changes(&app).await;
        let first = if resync {
            LiveEvent::Resync { cursor }
        } else {
            LiveEvent::Hello { cursor }
        };
        if tx.send(first).await.is_err() {
            return;
        }
        let deadline = tokio::time::Instant::now() + options.max_age;
        let mut tick = 0u32;
        let mut checked = tokio::time::Instant::now();
        loop {
            tokio::select! {
                () = tx.closed() => return,
                () = tokio::time::sleep_until(deadline) => return,
                () = tokio::time::sleep(options.poll) => {}
            }
            if checked.elapsed() >= options.recheck {
                checked = tokio::time::Instant::now();
                if !still_holds(&app, &watch.who, &secret).await {
                    return;
                }
            }
            tick = tick.wrapping_add(1);
            let mut events = watch.messages(&app).await;
            if tick.is_multiple_of(options.slow_every.max(1)) {
                events.extend(watch.status_changes(&app).await);
                events.extend(watch.card_changes(&app).await);
            }
            for event in events {
                if tx.send(event).await.is_err() {
                    return;
                }
            }
        }
    });
    rx
}

/// Whether the credential that opened the stream still names the same member, and that member
/// is still in the organization. A store that cannot answer ends the stream too: the browser
/// reconnects and is checked afresh.
async fn still_holds<D: Deps + 'static>(app: &App<D>, who: &Principal, secret: &str) -> bool {
    match auth::authenticate(app.store(), secret, app.now()).await {
        Ok(Some((found, _))) if found == *who => {}
        _ => return false,
    }
    floor::member_of(app.store(), who).await.is_ok()
}

fn last_event_id(headers: &HeaderMap) -> Option<i64> {
    headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse().ok())
}

/// The stream, as the response: `text/event-stream`, never cached, not buffered.
pub fn response(
    rx: mpsc::Receiver<LiveEvent>,
    keep_alive: Duration,
) -> Sse<impl Stream<Item = Frame>> {
    Sse::new(ReceiverStream::new(rx).map(|event| frame(&event)))
        .keep_alive(KeepAlive::new().interval(keep_alive))
}

#[utoipa::path(
    get, path = "/events/stream", security(("bearer" = [])),
    params(("Last-Event-ID" = Option<i64>, Header, description = "The `id` of the last event seen; the stream replays what came after it")),
    responses(
        (status = 200, description = "Server-sent events, each the JSON of a `LiveEvent`; the first is `hello` (or `resync`). The server ends the stream after a few minutes and the browser reconnects", content_type = "text/event-stream", body = LiveEvent),
        (status = 401, body = ApiError),
    )
)]
async fn stream<D: Deps + 'static>(
    State(app): State<Arc<App<D>>>,
    authed: Authed,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Frame>>, Fail> {
    floor::member_of(app.store(), &authed.who).await?;
    let options = Options::default();
    let secret = super::auth::presented(&headers)
        .ok_or(Fail::Unauthenticated)?
        .to_owned();
    let rx = open(app, authed.who, secret, last_event_id(&headers), options);
    Ok(response(rx, options.keep_alive))
}

pub fn routes<D: Deps + 'static>() -> Router<Arc<App<D>>> {
    Router::new().route("/events/stream", get(stream::<D>))
}
