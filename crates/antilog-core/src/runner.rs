//! The queue a runner polls and what it reports back (`docs/V0-contracts.md`). A runner serves
//! every organization; it holds a session by lease and everything it says about a session comes
//! with that session's own `Runner` token.

use antilog_types::{
    ApprovalStatus, CardKind, CardPayload, ChangesReport, Command, EventKind, Item, ItemKind,
    Machine, MachineState, PollRequest, PollResponse, Release, ReleaseOutcome, Reply, RepoSpec,
    SessionId, SessionSpec, SessionStatus, SkillDownload,
};
use jiff::SignedDuration;
use serde_json::{Value, json};

use crate::approval::{CardRequest, raise};
use crate::credentials::{CredentialsError, Purpose, SessionCredentials};
use crate::deps::Deps;
use crate::environments;
use crate::organization::Refusal;
use crate::session::{SessionError, audit, now_ms, record, record_tool};
use crate::store::{ApprovalStore, ChangesStore, CustomizeStore, Store};
use crate::store::{RunnerStore, SessionChange, SessionStore, StoreError};

/// How a server reaches its runners and what it tells them. From server config.
#[derive(Debug, Clone)]
pub struct Settings {
    /// Where OpenCode sends model calls: the proxy's base, such as `https://proxy.example.com`.
    pub proxy_url: String,
    /// This server as the machine reaches it.
    pub server_url: String,
    /// Prepended to a repo name to clone it, such as `https://github.com/`.
    pub clone_base: String,
    /// `provider/model`, for a session that names none.
    pub model: String,
    /// A claim's lease; each poll renews it.
    pub lease: SignedDuration,
    /// How long a session's tokens live.
    pub token_ttl: SignedDuration,
    /// How long one claim may hold a machine; then the session is let go as a release does, and
    /// a message resumes it.
    pub max_lifetime: SignedDuration,
}

impl Settings {
    /// `model` is `provider/model`: the server config names it, nothing here picks one.
    pub fn new(proxy_url: &str, server_url: &str, model: &str) -> Self {
        Self {
            proxy_url: proxy_url.trim_end_matches('/').to_owned(),
            server_url: server_url.trim_end_matches('/').to_owned(),
            clone_base: "https://github.com/".to_owned(),
            model: model.to_owned(),
            lease: SignedDuration::from_secs(60),
            token_ttl: SignedDuration::from_hours(12),
            max_lifetime: SignedDuration::from_hours(24),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("no such session or command")]
    NotFound,
    #[error("the session is not held by this runner")]
    NotHeld,
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error(transparent)]
    Refused(#[from] Refusal),
    #[error(transparent)]
    Credentials(#[from] CredentialsError),
    #[error(transparent)]
    Store(StoreError),
}

impl From<StoreError> for RunnerError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::NotFound => Self::NotFound,
            other => Self::Store(other),
        }
    }
}

/// The machine-wide OpenCode config, exactly as the contract lists it. The managed path beats a
/// repo's own `opencode.json`, so a repo cannot point the model calls elsewhere.
pub fn opencode_config(
    settings: &Settings,
    session: SessionId,
    model: &str,
    model_token: &str,
    mcp_token: &str,
) -> Value {
    json!({
        "autoupdate": false,
        // Titles and summaries use the session's own model, so a key that pays for one model
        // (or a free one) covers the whole session.
        "small_model": model,
        "provider": {
            "anthropic": {"options": {
                "baseURL": format!("{}/anthropic/v1", settings.proxy_url),
                "apiKey": model_token,
            }},
            "openai": {"options": {
                "baseURL": format!("{}/openai/v1", settings.proxy_url),
                "apiKey": model_token,
            }},
            "opencode": {"options": {
                "baseURL": format!("{}/opencode/v1", settings.proxy_url),
                "apiKey": model_token,
            }},
            // OpenCode Go is not one of OpenCode's own providers in this version's catalogue
            // (its model list is not fetched), so it is a custom OpenAI-compatible one.
            "opencode-go": {
                "npm": "@ai-sdk/openai-compatible",
                "name": "OpenCode Go",
                "options": {
                    "baseURL": format!("{}/opencode-go/v1", settings.proxy_url),
                    "apiKey": model_token,
                },
            },
        },
        "mcp": {"antilog": {
            "type": "remote",
            "url": format!("{}/mcp/sessions/{session}", settings.server_url),
            "headers": {"Authorization": format!("Bearer {mcp_token}")},
        }},
        // A seal check or a measurement runs trials in a fresh machine and takes minutes; the
        // default tool timeout is one minute.
        "experimental": {"mcp_timeout": 1_800_000},
        "skills": {"paths": ["/opt/antilog/skills"]},
        "permission": {"bash": "allow", "edit": "allow"},
    })
}

fn repo_spec(settings: &Settings, name: &str) -> RepoSpec {
    RepoSpec {
        name: name.to_owned(),
        url: format!("{}{name}", settings.clone_base),
        token: None,
    }
}

async fn spec<D: Deps>(
    deps: &D,
    settings: &Settings,
    machine: &Machine,
) -> Result<SessionSpec, RunnerError> {
    let creds = deps.credentials();
    let session = machine.session;
    let ttl = settings.token_ttl;
    // A runner that held it before no longer speaks for it.
    creds.revoke_all(session).await?;
    let model_token = creds.mint(session, Purpose::Model, ttl).await?;
    let mcp_token = creds.mint(session, Purpose::Mcp, ttl).await?;
    let runner_token = creds.mint(session, Purpose::Runner, ttl).await?;
    let row = deps
        .store()
        .session(machine.organization, session)
        .await?
        .ok_or(RunnerError::NotFound)?;
    // The latest version of each of the organization's skills; the runner downloads each.
    let skills = deps
        .store()
        .skills(machine.organization)
        .await?
        .into_iter()
        .map(|skill| SkillDownload {
            path: format!(
                "/runner/sessions/{session}/skills/{}/{}",
                skill.name, skill.latest
            ),
            name: skill.name,
            version: skill.latest.to_string(),
        })
        .collect();
    // The version the session started on; one that has gone missing fails the claim, so a
    // session never runs with a network its environment did not give it.
    let environment = environments::of_seed(deps.store(), machine.organization, &machine.seed)
        .await?
        .ok_or(RunnerError::NotFound)?;
    let model = if row.model.is_empty() {
        settings.model.clone()
    } else {
        row.model.clone()
    };
    Ok(SessionSpec {
        session,
        organization: machine.organization,
        agent: machine.agent.clone(),
        model: model.clone(),
        repos: machine
            .seed
            .repos
            .iter()
            .map(|name| repo_spec(settings, name))
            .collect(),
        environment,
        skills,
        opencode_config: opencode_config(settings, session, &model, &model_token, &mcp_token),
        model_token,
        mcp_token,
        runner_token,
        resume: machine.resume,
    })
}

/// The heartbeat: renews the runner's leases, takes new work up to its capacity, and returns the
/// commands for the sessions it holds.
pub async fn poll<D: Deps>(
    deps: &D,
    settings: &Settings,
    request: &PollRequest,
) -> Result<PollResponse, RunnerError> {
    let now = now_ms(deps);
    let until = now.saturating_add(settings.lease.as_millis().try_into().unwrap_or(60_000));
    let store = deps.store();
    if request.exiting {
        // The last poll: report what is finished, let the organization lock go, claim nothing.
        store
            .finish_commands(&request.runner, &request.done)
            .await?;
        store.unlock_runner(&request.runner).await?;
        return Ok(PollResponse::default());
    }
    let renewed = store.renew(&request.runner, now, until).await?;
    let lifetime = settings
        .max_lifetime
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX);
    for session in store
        .mark_overdue(&request.runner, now.saturating_sub(lifetime))
        .await?
    {
        time_up(deps, settings, session, now).await?;
    }
    let lost = request
        .held
        .iter()
        .filter(|session| !renewed.contains(session))
        .copied()
        .collect();
    store
        .finish_commands(&request.runner, &request.done)
        .await?;
    let mut claims = Vec::new();
    for machine in store
        .claim(&request.runner, request.capacity, now, until)
        .await?
    {
        claims.push(spec(deps, settings, &machine).await?);
    }
    let mut commands = store.take_commands(&request.runner).await?;
    // The queue holds only the name of an attached repo; where to clone it from is the server's.
    for envelope in &mut commands {
        if let Command::AttachRepo { repo } = &mut envelope.command {
            *repo = repo_spec(settings, &repo.name);
        }
    }
    Ok(PollResponse {
        claims,
        commands,
        lost,
    })
}

/// The machine has had its time: the runner is told to export and let the session go, the way a
/// release does, and the conversation says why. A person's next message resumes it.
async fn time_up<D: Deps>(
    deps: &D,
    settings: &Settings,
    session: SessionId,
    now: i64,
) -> Result<(), RunnerError> {
    let store = deps.store();
    let Some(machine) = store.machine(session).await? else {
        return Ok(());
    };
    store
        .push_command(
            machine.organization,
            session,
            &Command::Release {
                outcome: ReleaseOutcome::Idle,
            },
            now,
        )
        .await?;
    let Some(row) = store.session(machine.organization, session).await? else {
        return Ok(());
    };
    let minutes = settings.max_lifetime.as_mins();
    let span = if minutes % 60 == 0 {
        format!("{} hours", minutes / 60)
    } else {
        format!("{minutes} minutes")
    };
    let text = format!(
        "This session used its {span} on a machine, so Antilog stopped the machine and kept the conversation. Send a message to continue."
    );
    record(
        deps,
        &row,
        format!("lifetime:{session}:{now}"),
        EventKind::Agent,
        None,
        &text,
    )
    .await?;
    set_status(store, now, &machine, SessionStatus::WaitingForYou).await
}

async fn held_by<D: Deps>(
    deps: &D,
    session: SessionId,
    holder: &str,
) -> Result<Machine, RunnerError> {
    let machine = deps
        .store()
        .machine(session)
        .await?
        .ok_or(RunnerError::NotFound)?;
    if machine.state.is_held() && machine.holder.as_deref() == Some(holder) {
        Ok(machine)
    } else {
        Err(RunnerError::NotHeld)
    }
}

async fn ask_card<S: Store>(
    store: &S,
    now: i64,
    machine: &Machine,
    permission: &str,
    tool: &str,
    detail: &str,
) -> Result<(), RunnerError> {
    let existing = store
        .approvals(machine.organization, machine.session)
        .await?;
    if existing.iter().any(|approval| {
        matches!(&approval.payload, Some(CardPayload::AskFirst { permission: id, .. }) if id == permission)
    }) {
        return Ok(());
    }
    let session = store
        .session(machine.organization, machine.session)
        .await?
        .ok_or(RunnerError::NotFound)?;
    let summary: String = format!("{} asks to use {tool}: {detail}", machine.agent)
        .chars()
        .take(2000)
        .collect();
    let approval = raise(
        store,
        now,
        None,
        CardRequest {
            organization: machine.organization,
            session: machine.session,
            card: CardKind::AskFirst,
            summary,
            payload: Some(CardPayload::AskFirst {
                tool: tool.to_owned(),
                detail: detail.chars().take(2000).collect(),
                permission: permission.to_owned(),
            }),
            cost_cents: None,
            follow_policy: session.follow_policy,
        },
    )
    .await?;
    if approval.status == ApprovalStatus::ApprovedByPolicy {
        store
            .push_command(
                machine.organization,
                machine.session,
                &Command::Approval {
                    permission: permission.to_owned(),
                    reply: Reply::Once,
                },
                now,
            )
            .await?;
    } else {
        set_status(store, now, machine, SessionStatus::WaitingForYou).await?;
    }
    Ok(())
}

async fn set_status<S: Store>(
    store: &S,
    now: i64,
    machine: &Machine,
    status: SessionStatus,
) -> Result<(), RunnerError> {
    let Some(row) = store.session(machine.organization, machine.session).await? else {
        return Err(RunnerError::NotFound);
    };
    if row.status.is_active() && row.status != status {
        let change = SessionChange {
            status: Some(status),
            ..SessionChange::default()
        };
        store
            .update_session(machine.organization, machine.session, &change, now, None)
            .await?;
    }
    Ok(())
}

/// A batch of what OpenCode said and did. Each item is applied once however often it arrives.
pub async fn ingest<D: Deps>(
    deps: &D,
    session: SessionId,
    holder: &str,
    items: &[Item],
) -> Result<(), RunnerError> {
    let machine = held_by(deps, session, holder).await?;
    let row = deps
        .store()
        .session(machine.organization, session)
        .await?
        .ok_or(RunnerError::NotFound)?;
    for item in items {
        match item {
            Item::Words {
                key,
                kind: ItemKind::Tool,
                text,
                tool: Some(tool),
            } => {
                record_tool(deps, &row, format!("oc:{key}"), text, tool.clone()).await?;
            }
            Item::Words {
                key, kind, text, ..
            } => {
                let kind = match kind {
                    ItemKind::Agent => EventKind::Agent,
                    ItemKind::Tool => EventKind::Tool,
                };
                record(deps, &row, format!("oc:{key}"), kind, None, text).await?;
            }
            Item::Note { key, text } => {
                // As the machine-time and card notes are: Antilog's words, kept as agent text.
                record(
                    deps,
                    &row,
                    format!("note:{key}"),
                    EventKind::Agent,
                    None,
                    text,
                )
                .await?;
            }
            Item::Permission { id, tool, detail } => {
                ask_card(deps.store(), now_ms(deps), &machine, id, tool, detail).await?;
            }
            Item::Status { busy } => {
                let (state, status) = if *busy {
                    (MachineState::Running, SessionStatus::Working)
                } else {
                    (MachineState::Idle, SessionStatus::WaitingForYou)
                };
                deps.store()
                    .set_machine_state(session, holder, state, now_ms(deps))
                    .await?;
                // A permission still waiting for a person keeps the session waiting.
                let asking = deps
                    .store()
                    .pending_approvals(machine.organization)
                    .await?
                    .iter()
                    .any(|approval| approval.session_id == session);
                if !(asking && *busy) {
                    set_status(deps.store(), now_ms(deps), &machine, status).await?;
                }
            }
        }
    }
    Ok(())
}

/// The runner's report of what the session changed in each repo; it replaces the last one.
pub async fn report_changes<D: Deps>(
    deps: &D,
    session: SessionId,
    holder: &str,
    report: ChangesReport,
) -> Result<(), RunnerError> {
    let machine = held_by(deps, session, holder).await?;
    deps.store()
        .put_changes(
            machine.organization,
            session,
            &report.capped(),
            now_ms(deps),
        )
        .await?;
    Ok(())
}

/// The runner lets the session go: its export is kept, its tokens stop working, and the
/// session is waiting for a message, done or failed.
pub async fn release<D: Deps>(
    deps: &D,
    session: SessionId,
    holder: &str,
    release: &Release,
) -> Result<(), RunnerError> {
    let machine = held_by(deps, session, holder).await?;
    let now = now_ms(deps);
    if let Some(export) = &release.export {
        deps.store()
            .put_export(machine.organization, session, export, now)
            .await?;
    }
    let (state, status, note) = match release.outcome {
        ReleaseOutcome::Idle => (MachineState::Released, None, None),
        ReleaseOutcome::Done => (MachineState::Done, None, None),
        ReleaseOutcome::Failed => (
            MachineState::Failed,
            Some(SessionStatus::Failed),
            Some("The machine failed".to_owned()),
        ),
    };
    deps.store()
        .set_machine_state(session, holder, state, now)
        .await?;
    deps.credentials().revoke_all(session).await?;
    if let Some(status) = status {
        let change = SessionChange {
            status: Some(status),
            note,
        };
        let entry = audit(deps, machine.organization, None, "session.release", session);
        match deps
            .store()
            .update_session(machine.organization, session, &change, now, Some(&entry))
            .await
        {
            // A session that already ended stays as it ended.
            Ok(_) | Err(StoreError::Invariant(_)) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

/// The person's answer to a permission card reaches the machine that asked it, and the session
/// is Working again.
pub async fn answer<S: Store>(
    store: &S,
    approval: &antilog_types::Approval,
    now: i64,
) -> Result<(), RunnerError> {
    let Some(CardPayload::AskFirst { permission, .. }) = &approval.payload else {
        return Ok(());
    };
    let session = approval.session_id;
    let reply = if matches!(
        approval.status,
        ApprovalStatus::Approved | ApprovalStatus::ApprovedByPolicy
    ) {
        Reply::Once
    } else {
        Reply::Reject
    };
    let command = Command::Approval {
        permission: permission.clone(),
        reply,
    };
    match store
        .push_command(approval.organization_id, session, &command, now)
        .await
    {
        Ok(_) => {}
        Err(StoreError::NotFound) => return Ok(()),
        Err(error) => return Err(error.into()),
    }
    if let Some(machine) = store.machine(session).await? {
        set_status(store, now, &machine, SessionStatus::Working).await?;
    }
    Ok(())
}
