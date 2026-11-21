mod settle;

use crate::deps::Deps;
use crate::environments::{Unavailable, pin};
use crate::store::{
    NewEvent, OrganizationStore, RunnerStore, SessionChange, SessionStore, StoreError,
};
use antilog_types::{
    AuditEntry, AuditOutcome, Command, EventKind, MemberId, OrganizationId, Seed, Session,
    SessionId, SessionParams, SessionStatus,
};

pub use settle::settle;

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("no such session in this organization")]
    NotFound,
    #[error("the session has ended; start a new one")]
    Ended,
    #[error(transparent)]
    Environment(#[from] Unavailable),
    #[error("{0}")]
    Invalid(&'static str),
    #[error(
        "the organization's monthly cap is reached. An owner raises it in Settings, Budgets"
    )]
    OrganizationCapReached,
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

/// What a person gives to start a session. The agent and the model are already resolved.
#[derive(Debug, Clone, Default)]
pub struct NewSession {
    pub title: Option<String>,
    pub agent: String,
    pub model: String,
    pub prompt: String,
    pub repos: Vec<String>,
    pub environment: Option<String>,
    /// Left out, the organization's default, if it has one.
    pub budget_cap_cents: Option<i64>,
    pub follow_policy: bool,
    pub actor: MemberId,
}

/// The title a session shows when its starter gave none: the prompt's first line.
fn title_of(title: Option<&str>, prompt: &str) -> String {
    let given = title.map(str::trim).filter(|t| !t.is_empty());
    let line = given.unwrap_or_else(|| prompt.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or(""));
    let line: String = line.chars().take(80).collect();
    if line.is_empty() {
        "New session".to_owned()
    } else {
        line
    }
}

/// Starts a session in the organization: its row, its first words and its audit entry. The
/// agent runs wherever the runner puts it; nothing here starts one.
pub async fn start<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    new: NewSession,
) -> Result<Session, SessionError> {
    let now = now_ms(deps);
    let row = deps
        .store()
        .organization(organization)
        .await?
        .ok_or(SessionError::NotFound)?;
    let cap = new.budget_cap_cents.or(row.default_budget_cents);
    if cap.is_some_and(|cap| cap < 0) {
        return Err(SessionError::Invalid("a budget is not negative"));
    }
    if let (Some(cap), Some(monthly)) = (cap, row.monthly_cap_cents)
        && cap > monthly
    {
        return Err(SessionError::Invalid(
            "a session's budget cannot pass the organization's monthly cap",
        ));
    }
    if !crate::budget::organization_has_room(deps.store(), organization, now)
        .await
        .map_err(|_| SessionError::Store(StoreError::Unavailable("budget check".into())))?
    {
        return Err(SessionError::OrganizationCapReached);
    }
    // The environment is pinned at its version now: a later edit does not move a session.
    let (environment, environment_version) =
        pin(deps.store(), organization, new.environment.as_deref()).await?;
    let session = Session::new(
        organization,
        new.actor,
        SessionParams {
            title: title_of(new.title.as_deref(), &new.prompt),
            agent: new.agent,
            model: new.model,
            repos: new.repos.clone(),
            environment: environment.clone(),
            environment_version,
            budget_cap_cents: cap,
            follow_policy: new.follow_policy,
        },
        now,
    );
    let entry = audit(deps, organization, Some(new.actor), "session.start", session.id);
    deps.store().create_session(&session, &entry).await?;
    let seed = Seed {
        repos: new.repos,
        environment,
        environment_version,
    };
    deps.store()
        .enqueue_machine(organization, session.id, &seed, now)
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
            Some(new.actor),
            &new.prompt,
        )
        .await?;
    }
    Ok(session)
}

/// Tells the session's machine, if it has one yet.
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

/// What a session may still spend: the least left under its own cap and the organization's
/// monthly cap, in US cents; `None` when neither limits it.
pub async fn budget_left<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    id: SessionId,
) -> Result<Option<i64>, SessionError> {
    crate::budget::headroom(deps.store(), organization, id, now_ms(deps))
        .await
        .map(|room| room.remaining())
        .map_err(|error| match error {
            crate::organization::Refusal::NotFound => SessionError::NotFound,
            _ => SessionError::Store(StoreError::Unavailable("budget check".into())),
        })
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
