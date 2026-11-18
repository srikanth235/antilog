mod relay;

use std::time::Duration;

use antilog_types::{
    AuditEntry, AuditOutcome, EventKind, ExperimentId, MemberId, OrganizationId, Session,
    SessionId, SessionStatus,
};
use jiff::SignedDuration;

use crate::deps::Deps;
use crate::platform::{Platform, PlatformError, SessionHandle, SessionSpec};
use crate::spend::{BudgetError, ensure_budget};
use crate::store::{NewEvent, SessionChange, SessionStore, StoreError};
use crate::timer::{JobId, Timer, TimerError};
use crate::tools::definitions;

pub use relay::{Step, step};

/// One relay step is about this long; the next wake-up is booked at its end.
pub const STEP: Duration = Duration::from_secs(600);

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("no such experiment or session in this organization")]
    NotFound,
    #[error("the session has ended; start a new one")]
    Ended,
    #[error(transparent)]
    Budget(#[from] BudgetError),
    #[error(transparent)]
    Store(StoreError),
    #[error(transparent)]
    Platform(#[from] PlatformError),
    #[error(transparent)]
    Timer(#[from] TimerError),
}

impl From<StoreError> for SessionError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::NotFound => Self::NotFound,
            other => Self::Store(other),
        }
    }
}

pub fn job(session: SessionId) -> JobId {
    JobId(session.as_uuid().as_u128())
}

pub(crate) fn now_ms<D: Deps>(deps: &D) -> i64 {
    use crate::clock::Clock;
    deps.clock().now().as_millisecond()
}

pub(crate) async fn book<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    session: SessionId,
    after: Duration,
) -> Result<(), TimerError> {
    use crate::clock::Clock;
    let after = SignedDuration::try_from(after).unwrap_or(SignedDuration::from_secs(600));
    let at = deps
        .clock()
        .now()
        .checked_add(after)
        .unwrap_or_else(|_| deps.clock().now());
    deps.timer().wake_at(organization, job(session), at).await
}

/// Keeps one item of the conversation. Only the words said and the name of a tool called, never a
/// tool's input or output. Keyed, so a relay that delivers it twice records it once.
pub(crate) async fn record<D: Deps>(
    deps: &D,
    session: &Session,
    key: String,
    kind: EventKind,
    member: Option<MemberId>,
    text: &str,
) -> Result<(), SessionError> {
    deps.store()
        .append_event(&NewEvent {
            organization: session.organization_id,
            session: session.id,
            key,
            kind,
            member,
            text: text.chars().take(100_000).collect(),
            at: now_ms(deps),
        })
        .await?;
    Ok(())
}

pub(crate) fn audit<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    actor: Option<MemberId>,
    action: &str,
    target: impl ToString,
) -> AuditEntry {
    AuditEntry::new(
        organization,
        actor,
        action,
        target,
        AuditOutcome::Done,
        now_ms(deps),
    )
}

#[derive(Debug, Clone, Default)]
pub struct NewSession {
    pub agent: String,
    pub prompt: String,
    pub repos: Vec<String>,
    pub environment: Option<String>,
    pub parent: Option<SessionId>,
    pub actor: Option<MemberId>,
}

/// Starts a session in an experiment on the platform and books its first relay step.
/// A platform that is down leaves the session Waiting for you, never lost.
pub async fn start<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    experiment: ExperimentId,
    new: NewSession,
) -> Result<Session, SessionError> {
    let row = deps
        .store()
        .experiment(organization, experiment)
        .await?
        .ok_or(SessionError::NotFound)?;
    ensure_budget(
        deps.store(),
        deps.clock(),
        organization,
        experiment,
        row.budget_cap_cents,
    )
    .await?;
    let mut session = Session::new(
        organization,
        experiment,
        &new.agent,
        deps.platform().name(),
        now_ms(deps),
    );
    session.parent = new.parent;
    let entry = audit(deps, organization, new.actor, "session.start", session.id);
    deps.store().create_session(&session, &entry).await?;
    if !new.prompt.trim().is_empty() {
        record(
            deps,
            &session,
            "prompt".to_owned(),
            EventKind::Person,
            new.actor,
            &new.prompt,
        )
        .await?;
    }
    let spec = SessionSpec {
        agent: new.agent,
        prompt: new.prompt,
        repos: new.repos,
        environment: new.environment,
        tools: definitions(new.parent.is_some()),
    };
    let change = match deps.platform().start_session(&spec).await {
        Ok(SessionHandle(handle)) => SessionChange {
            platform_handle: Some(handle),
            ..SessionChange::default()
        },
        Err(error) => {
            let _ = &error;
            SessionChange {
                status: Some(SessionStatus::WaitingForYou),
                note: Some("The platform is unavailable. Continue on another agent?".into()),
                ..SessionChange::default()
            }
        }
    };
    let started = change.platform_handle.is_some();
    let session = deps
        .store()
        .update_session(organization, session.id, &change, now_ms(deps), None)
        .await?;
    if started {
        book(deps, organization, session.id, STEP).await?;
    }
    Ok(session)
}

/// The session an agent on a person's own machine works under: no platform, no relay.
pub async fn open_external<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    experiment: ExperimentId,
    actor: Option<MemberId>,
) -> Result<Session, SessionError> {
    let session = Session::new(organization, experiment, "mcp", "external", now_ms(deps));
    let entry = audit(deps, organization, actor, "session.start", session.id);
    deps.store().create_session(&session, &entry).await?;
    Ok(session)
}

async fn active<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    id: SessionId,
) -> Result<(Session, SessionHandle), SessionError> {
    let session = deps
        .store()
        .session(organization, id)
        .await?
        .ok_or(SessionError::NotFound)?;
    match (&session.platform_handle, session.status.is_active()) {
        (Some(handle), true) => {
            let handle = SessionHandle(handle.clone());
            Ok((session, handle))
        }
        _ => Err(SessionError::Ended),
    }
}

/// A person talks to the session: the agent gets the text and the session is Working again.
pub async fn message<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    id: SessionId,
    actor: Option<MemberId>,
    text: &str,
) -> Result<Session, SessionError> {
    let (row, handle) = active(deps, organization, id).await?;
    deps.platform().send_input(&handle, text).await?;
    record(
        deps,
        &row,
        format!("message:{}", uuid::Uuid::now_v7()),
        EventKind::Person,
        actor,
        text,
    )
    .await?;
    let change = SessionChange {
        status: Some(SessionStatus::Working),
        ..SessionChange::default()
    };
    let session = deps
        .store()
        .update_session(
            organization,
            id,
            &change,
            now_ms(deps),
            Some(&audit(deps, organization, actor, "session.message", id)),
        )
        .await?;
    book(deps, organization, id, Duration::ZERO).await?;
    Ok(session)
}

/// A person stops the session: Done, "Stopped by NAME". What it spent until now is kept.
pub async fn stop<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    id: SessionId,
    actor: Option<MemberId>,
    name: &str,
) -> Result<Session, SessionError> {
    let (_, handle) = active(deps, organization, id).await?;
    deps.platform().stop(&handle).await?;
    relay::drain(deps, organization, id, &handle).await?;
    let session = relay::finish(
        deps,
        organization,
        id,
        &handle,
        SessionStatus::Done,
        Some(format!("Stopped by {name}")),
        audit(deps, organization, actor, "session.stop", id),
    )
    .await?;
    Ok(session)
}

/// Spend is metered by the relay; this is its total for an experiment.
pub async fn budget_left<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    experiment: ExperimentId,
) -> Result<i64, SessionError> {
    let row = deps
        .store()
        .experiment(organization, experiment)
        .await?
        .ok_or(SessionError::NotFound)?;
    let spent = crate::spend::spent(deps.store(), deps.clock(), organization, experiment).await?;
    Ok(row.budget_cap_cents.saturating_sub(spent.cents))
}

/// The default and largest page of a conversation.
pub const PAGE: u32 = 100;
pub const PAGE_MAX: u32 = 500;

/// A page of the conversation, oldest first, after the `seq` the reader last saw.
pub async fn transcript<S: SessionStore>(
    store: &S,
    organization: OrganizationId,
    session: SessionId,
    after: Option<i64>,
    limit: Option<u32>,
) -> Result<antilog_types::EventPage, SessionError> {
    let limit = limit.unwrap_or(PAGE).clamp(1, PAGE_MAX);
    store
        .session(organization, session)
        .await?
        .ok_or(SessionError::NotFound)?;
    let mut events = store
        .session_events(organization, session, after.unwrap_or(0).max(0), limit + 1)
        .await?;
    let more = events.len() > limit as usize;
    events.truncate(limit as usize);
    let next = events.last().filter(|_| more).map(|event| event.seq);
    Ok(antilog_types::EventPage { events, next })
}
