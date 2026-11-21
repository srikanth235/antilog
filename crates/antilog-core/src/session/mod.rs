mod environment;
mod settle;

use crate::deps::Deps;
use crate::environments::{Unavailable, pin};
use crate::store::{
    ApprovalStore, NewEvent, OrganizationStore, RunnerStore, SessionChange, SessionStore,
    StoreError,
};
use antilog_types::{
    AuditEntry, AuditOutcome, Command, EventKind, MachineState, MemberId, OrganizationId,
    ProjectId, Seed, Session, SessionId, SessionParams, SessionStatus, ToolRecord,
};

pub use environment::move_to_environment;
pub use settle::settle;

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("no such project or session in this organization")]
    NotFound,
    #[error("the session has ended; start a new one")]
    Ended,
    #[error(transparent)]
    Environment(#[from] Unavailable),
    #[error("the session's machine is not running: resume the session")]
    NotRunning,
    #[error("{0}")]
    Invalid(&'static str),
    #[error(
        "a monthly cap is reached, the project's or the organization's. An owner raises it in Settings, Budgets"
    )]
    CapReached,
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

/// Keeps one item of the conversation: the words said. A tool call goes through `record_tool`. Keyed, so an item delivered twice is recorded once.
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
            tool: None,
            at: now_ms(deps),
        })
        .await?;
    Ok(())
}

/// Keeps one finished tool call: its name as the text and a cleaned `ToolRecord` (label, how it
/// ended, input and output cut small). Keyed by the call, so a call recorded twice is kept once.
pub(crate) async fn record_tool<D: Deps>(
    deps: &D,
    session: &Session,
    key: String,
    name: &str,
    tool: ToolRecord,
) -> Result<(), SessionError> {
    deps.store()
        .append_event(&NewEvent {
            organization: session.organization_id,
            session: session.id,
            key,
            kind: EventKind::Tool,
            member: None,
            text: name.chars().take(200).collect(),
            tool: Some(tool.cleaned()),
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
    pub effort: Option<String>,
    pub prompt: String,
    pub repos: Vec<String>,
    pub environment: Option<String>,
    /// The branch to check out in the repos picked here.
    pub branch: Option<String>,
    /// Left out, the organization's default, if it has one.
    pub budget_cap_cents: Option<i64>,
    pub follow_policy: bool,
    pub actor: MemberId,
}

fn branch_ok(branch: &str) -> bool {
    branch.len() <= 200
        && !branch.starts_with(['-', '/'])
        && !branch.contains("..")
        && branch
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'/'))
}

/// The title a session shows when its starter gave none: the prompt's first line.
fn title_of(title: Option<&str>, prompt: &str) -> String {
    let given = title.map(str::trim).filter(|t| !t.is_empty());
    let line = given.unwrap_or_else(|| {
        prompt
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .unwrap_or("")
    });
    let line: String = line.chars().take(80).collect();
    if line.is_empty() {
        "New session".to_owned()
    } else {
        line
    }
}

/// Starts a session in a project: its row, its first words and its audit entry. The
/// agent runs wherever the runner puts it; nothing here starts one.
pub async fn start<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    project: ProjectId,
    new: NewSession,
) -> Result<Session, SessionError> {
    let now = now_ms(deps);
    let row = deps
        .store()
        .organization(organization)
        .await?
        .ok_or(SessionError::NotFound)?;
    let home = deps
        .store()
        .project(organization, project)
        .await?
        .ok_or(SessionError::NotFound)?;
    let cap = new
        .budget_cap_cents
        .or(home.default_budget_cents)
        .or(row.default_budget_cents);
    if cap.is_some_and(|cap| cap < 0) {
        return Err(SessionError::Invalid("a budget is not negative"));
    }
    let ceiling = [home.monthly_cap_cents, row.monthly_cap_cents]
        .into_iter()
        .flatten()
        .min();
    if let (Some(cap), Some(ceiling)) = (cap, ceiling)
        && cap > ceiling
    {
        return Err(SessionError::Invalid(
            "a session's budget cannot pass its project's or the organization's monthly cap",
        ));
    }
    if !crate::budget::has_room(deps.store(), organization, project, now)
        .await
        .map_err(|_| SessionError::Store(StoreError::Unavailable("budget check".into())))?
    {
        return Err(SessionError::CapReached);
    }
    // The environment is pinned at its version now: a later edit does not move a session.
    let pinned = pin(deps.store(), organization, new.environment.as_deref()).await?;
    // The environment's default repos come first, then the ones the person picked.
    let mut repos = pinned.repos;
    let mut branches = std::collections::BTreeMap::new();
    let branch = new
        .branch
        .as_deref()
        .map(str::trim)
        .filter(|b| !b.is_empty());
    if branch.is_some_and(|b| !branch_ok(b)) {
        return Err(SessionError::Invalid("a branch name has no spaces or .."));
    }
    for picked in new.repos {
        if let Some(branch) = branch {
            branches.insert(picked.clone(), branch.to_owned());
        }
        if !repos.contains(&picked) {
            repos.push(picked);
        }
    }
    let (environment, environment_version) = (pinned.id, pinned.version);
    let session = Session::new(
        organization,
        project,
        new.actor,
        SessionParams {
            title: title_of(new.title.as_deref(), &new.prompt),
            agent: new.agent,
            model: new.model,
            effort: new.effort,
            repos: repos.clone(),
            environment: environment.clone(),
            environment_version,
            budget_cap_cents: cap,
            follow_policy: new.follow_policy,
        },
        now,
    );
    let entry = audit(
        deps,
        organization,
        Some(new.actor),
        "session.start",
        session.id,
    );
    deps.store().create_session(&session, &entry).await?;
    let seed = Seed {
        repos,
        environment,
        environment_version,
        branches,
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
pub(crate) async fn tell<D: Deps>(
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

pub(crate) async fn active<D: Deps>(
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

/// Puts a released machine back in the queue and says what the session is until a runner holds
/// it: Queued, or Working when its machine is still held.
async fn wake<D: Deps>(deps: &D, id: SessionId, now: i64) -> Result<SessionStatus, SessionError> {
    deps.store().wake_machine(id, now).await?;
    let queued = deps
        .store()
        .machine(id)
        .await?
        .is_some_and(|machine| machine.state == MachineState::Queued);
    Ok(if queued {
        SessionStatus::Queued
    } else {
        SessionStatus::Working
    })
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
    let resumed = wake(deps, id, now).await?;
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
        status: Some(resumed),
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

/// A person changes the session's model or effort. It applies from the next message: a running
/// machine is told in order with the messages (`Command::SetModel`), a released one reads the
/// session's row when it is claimed. The timeline gets `note` ("Switched to X · High").
pub async fn set_model<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    id: SessionId,
    actor: Option<MemberId>,
    model: &str,
    effort: Option<&str>,
    note: &str,
) -> Result<Session, SessionError> {
    let row = active(deps, organization, id).await?;
    if row.model == model && row.effort.as_deref() == effort {
        return Ok(row);
    }
    deps.store()
        .set_session_model(
            organization,
            id,
            model,
            effort,
            &audit(deps, organization, actor, "session.set_model", id),
        )
        .await?;
    tell(
        deps,
        organization,
        id,
        &Command::SetModel {
            model: model.to_owned(),
            effort: effort.map(str::to_owned),
        },
    )
    .await?;
    record(
        deps,
        &row,
        format!("model:{}", uuid::Uuid::now_v7()),
        EventKind::Note,
        actor,
        note,
    )
    .await?;
    deps.store()
        .session(organization, id)
        .await?
        .ok_or(SessionError::NotFound)
}

/// A person asks the agent's harness to compact the conversation now. The machine is woken if it
/// was let go; the runner reports the compaction as a note in the transcript.
pub async fn compact<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    id: SessionId,
    actor: Option<MemberId>,
) -> Result<Session, SessionError> {
    let row = active(deps, organization, id).await?;
    let now = now_ms(deps);
    let resumed = wake(deps, id, now).await?;
    tell(deps, organization, id, &Command::Compact).await?;
    let change = SessionChange {
        status: Some(resumed),
        ..SessionChange::default()
    };
    Ok(deps
        .store()
        .update_session(
            organization,
            row.id,
            &change,
            now_ms(deps),
            Some(&audit(deps, organization, actor, "session.message", id)),
        )
        .await?)
}

/// A person stops the session: Stopped, "Stopped by NAME". What it spent until now is kept.
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
        status: Some(SessionStatus::Stopped),
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

/// A person wakes a session whose machine was let go (idle, or a runner that went away) without
/// saying anything: it waits for a runner again. A session whose machine is held is unchanged.
pub async fn resume<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    id: SessionId,
    actor: Option<MemberId>,
) -> Result<Session, SessionError> {
    let row = active(deps, organization, id).await?;
    let released = deps
        .store()
        .machine(id)
        .await?
        .is_some_and(|machine| machine.state == MachineState::Released);
    if !released {
        return Ok(row);
    }
    let status = wake(deps, id, now_ms(deps)).await?;
    let change = SessionChange {
        status: Some(status),
        ..SessionChange::default()
    };
    Ok(deps
        .store()
        .update_session(
            organization,
            id,
            &change,
            now_ms(deps),
            Some(&audit(deps, organization, actor, "session.resume", id)),
        )
        .await?)
}

/// A person opens a terminal in the session's machine. Only a machine that is up (OpenCode
/// running) takes one; the command waits for the runner's next poll. Audited as
/// `terminal.opened` with the session; what is typed is never recorded.
pub async fn open_terminal<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    id: SessionId,
    actor: Option<MemberId>,
    terminal: uuid::Uuid,
    cols: u16,
    rows: u16,
) -> Result<(), SessionError> {
    active(deps, organization, id).await?;
    let up =
        deps.store().machine(id).await?.is_some_and(|machine| {
            matches!(machine.state, MachineState::Running | MachineState::Idle)
        });
    if !up {
        return Err(SessionError::NotRunning);
    }
    if cols == 0 || rows == 0 || cols > 1000 || rows > 1000 {
        return Err(SessionError::Invalid(
            "The terminal's size is out of range.",
        ));
    }
    tell(
        deps,
        organization,
        id,
        &Command::OpenTerminal {
            terminal,
            cols,
            rows,
        },
    )
    .await?;
    deps.store()
        .append_audit(&audit(deps, organization, actor, "terminal.opened", id))
        .await?;
    Ok(())
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
