mod relay;

use std::time::Duration;

use antilog_types::{
    AuditEntry, AuditOutcome, ExperimentId, MemberId, OrganizationId, Session, SessionId,
    SessionStatus,
};
use jiff::SignedDuration;

use crate::deps::Deps;
use crate::platform::{Platform, PlatformError, SessionHandle, SessionSpec};
use crate::spend::{BudgetError, ensure_budget};
use crate::store::{SessionChange, SessionStore, StoreError};
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
    deps.timer().wake_at(job(session), at).await
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
        book(deps, session.id, STEP).await?;
    }
    Ok(session)
}

/// The session an agent on a person's own machine works under: no platform, no relay.
pub async fn open_external<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    experiment: ExperimentId,
) -> Result<Session, SessionError> {
    let session = Session::new(organization, experiment, "mcp", "external", now_ms(deps));
    let entry = audit(deps, organization, None, "session.start", session.id);
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
    text: &str,
) -> Result<Session, SessionError> {
    let (_, handle) = active(deps, organization, id).await?;
    deps.platform().send_input(&handle, text).await?;
    let change = SessionChange {
        status: Some(SessionStatus::Working),
        ..SessionChange::default()
    };
    let session = deps
        .store()
        .update_session(organization, id, &change, now_ms(deps), None)
        .await?;
    book(deps, id, Duration::ZERO).await?;
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
