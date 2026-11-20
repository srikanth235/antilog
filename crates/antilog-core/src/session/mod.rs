mod settle;

use crate::deps::Deps;
use crate::spend::{BudgetError, ensure_budget};
use crate::store::{NewEvent, RunnerStore, SessionChange, SessionStore, StoreError};
use antilog_types::{
    AuditEntry, AuditOutcome, Command, EventKind, ExperimentId, MemberId, OrganizationId, Seed,
    Session, SessionId, SessionStatus,
};

pub use settle::settle;

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
}

impl From<StoreError> for SessionError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::NotFound => Self::NotFound,
            other => Self::Store(other),
        }
    }
}

pub(crate) fn now_ms<D: Deps>(deps: &D) -> i64 {
    use crate::clock::Clock;
    deps.clock().now().as_millisecond()
}

/// Keeps one item of the conversation. Only the words said and the name of a tool called, never a
/// tool's input or output. Keyed, so an item delivered twice is recorded once.
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
    pub actor: Option<MemberId>,
}

/// Starts a session in an experiment: its row, its first words and its audit entry. The agent
/// runs wherever the runner puts it; nothing here starts one.
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
    let session = Session::new(organization, experiment, &new.agent, now_ms(deps));
    let entry = audit(deps, organization, new.actor, "session.start", session.id);
    deps.store().create_session(&session, &entry).await?;
    let seed = Seed {
        repos: new.repos,
        environment: new.environment,
    };
    deps.store()
        .enqueue_machine(organization, session.id, &seed, now_ms(deps))
        .await?;
    if !new.prompt.trim().is_empty() {
        tell(
            deps,
            organization,
            session.id,
            &Command::Message {
                text: new.prompt.clone(),
            },
        )
        .await?;
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
    Ok(session)
}

/// The session an agent on a person's own machine works under.
pub async fn open_external<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    experiment: ExperimentId,
    actor: Option<MemberId>,
) -> Result<Session, SessionError> {
    let session = Session::new(organization, experiment, "mcp", now_ms(deps));
    let entry = audit(deps, organization, actor, "session.start", session.id);
    deps.store().create_session(&session, &entry).await?;
    Ok(session)
}

/// Tells the session's machine, if it has one: a session of an agent on a person's own machine
/// has none.
async fn tell<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    id: SessionId,
    command: &Command,
) -> Result<(), SessionError> {
    match deps
        .store()
        .push_command(organization, id, command, now_ms(deps))
        .await
    {
        Ok(_) | Err(StoreError::NotFound) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

async fn active<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    id: SessionId,
) -> Result<Session, SessionError> {
    let session = deps
        .store()
        .session(organization, id)
        .await?
        .ok_or(SessionError::NotFound)?;
    if session.status.is_active() {
        Ok(session)
    } else {
        Err(SessionError::Ended)
    }
}

/// A person talks to the session: their words join the conversation and it is Working again.
pub async fn message<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    id: SessionId,
    actor: Option<MemberId>,
    text: &str,
) -> Result<Session, SessionError> {
    let row = active(deps, organization, id).await?;
    record(
        deps,
        &row,
        format!("message:{}", uuid::Uuid::now_v7()),
        EventKind::Person,
        actor,
        text,
    )
    .await?;
    let now = now_ms(deps);
    // A machine that was let go waits for a runner again; the words reach it once one holds it.
    deps.store().wake_machine(id, now).await?;
    tell(
        deps,
        organization,
        id,
        &Command::Message {
            text: text.to_owned(),
        },
    )
    .await?;
    let change = SessionChange {
        status: Some(SessionStatus::Working),
        ..SessionChange::default()
    };
    Ok(deps
        .store()
        .update_session(
            organization,
            id,
            &change,
            now_ms(deps),
            Some(&audit(deps, organization, actor, "session.message", id)),
        )
        .await?)
}

/// A person stops the session: Done, "Stopped by NAME". What it spent until now is kept.
pub async fn stop<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    id: SessionId,
    actor: Option<MemberId>,
    name: &str,
) -> Result<Session, SessionError> {
    active(deps, organization, id).await?;
    tell(
        deps,
        organization,
        id,
        &Command::Release {
            outcome: antilog_types::ReleaseOutcome::Done,
        },
    )
    .await?;
    let change = SessionChange {
        status: Some(SessionStatus::Done),
        note: Some(format!("Stopped by {name}")),
    };
    Ok(deps
        .store()
        .update_session(
            organization,
            id,
            &change,
            now_ms(deps),
            Some(&audit(deps, organization, actor, "session.stop", id)),
        )
        .await?)
}

/// What the experiment has spent; this is its total for an experiment.
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
