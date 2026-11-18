use antilog_types::{
    Approval, ApprovalId, ApprovalStatus, AuditEntry, AuditOutcome, CardKind, SessionStatus,
};
use serde::Deserialize;
use serde_json::json;

use super::{Caller, Done, ToolError, input};
use crate::deps::Deps;
use crate::session::{NewSession, now_ms};
use crate::store::{ApprovalStore, SessionChange, SessionStore};

/// A card in front of a person; the agent waits for the answer as a message.
pub(super) async fn ask<D: Deps>(
    deps: &D,
    caller: &Caller,
    tool: &str,
    card: CardKind,
    summary: String,
    cost_cents: Option<i64>,
) -> Result<Approval, ToolError> {
    let now = now_ms(deps);
    let approval = Approval {
        id: ApprovalId::new(),
        organization_id: caller.organization,
        experiment_id: caller.experiment,
        session_id: Some(caller.session),
        card,
        summary,
        cost_cents,
        status: ApprovalStatus::Pending,
        decided_by: None,
        created_at: now,
        decided_at: None,
    };
    let audit = AuditEntry::new(
        caller.organization,
        caller.actor,
        format!("tool.{tool}"),
        approval.id,
        AuditOutcome::Done,
        now,
    );
    deps.store().request_approval(&approval, &audit).await?;
    let waiting = SessionChange {
        status: Some(SessionStatus::WaitingForYou),
        ..SessionChange::default()
    };
    deps.store()
        .update_session(caller.organization, caller.session, &waiting, now, None)
        .await?;
    Ok(approval)
}

pub(super) fn waiting(approval: &Approval) -> serde_json::Value {
    json!({
        "approval": approval.id,
        "status": "waiting for a person",
        "note": "A card is in front of a person. The answer arrives as a message; do not call this tool again for it."
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Repo {
    repo: String,
}

pub(super) async fn attach_repo<D: Deps>(
    deps: &D,
    caller: &Caller,
    value: serde_json::Value,
) -> Done {
    let Repo { repo } = input(value)?;
    let shaped = repo.split_once('/').is_some_and(|(owner, name)| {
        !owner.is_empty()
            && !name.is_empty()
            && !name.contains('/')
            && !repo.contains(char::is_whitespace)
    });
    if !shaped {
        return Err(ToolError::new("the repo must look like owner/name"));
    }
    let approval = ask(
        deps,
        caller,
        "attach_repo",
        CardKind::AttachRepo,
        format!("Attach repo {repo}"),
        None,
    )
    .await?;
    Ok((waiting(&approval), None))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Credential {
    connector: String,
    reason: String,
    scope: String,
}

pub(super) async fn request_credential<D: Deps>(
    deps: &D,
    caller: &Caller,
    value: serde_json::Value,
) -> Done {
    let Credential { connector, reason, scope } = input(value).map_err(|_| {
        ToolError::new("send connector, reason and scope only: never a secret value; a person enters it on the card")
    })?;
    let lower = scope.to_lowercase();
    if ["holdout", "write", "admin"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return Err(ToolError::new(
            "a credential is read-only and never reaches the holdout: ask for a read scope on the evaluation data",
        ));
    }
    let summary = format!("Add a read-only credential for {connector} ({scope}): {reason}");
    let approval = ask(
        deps,
        caller,
        "request_credential",
        CardKind::AddCredential,
        summary,
        None,
    )
    .await?;
    Ok((waiting(&approval), None))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Environment {
    image: Option<String>,
    setup_script: Option<String>,
    size: Option<String>,
    network: Option<String>,
}

pub(super) async fn set_environment<D: Deps>(
    deps: &D,
    caller: &Caller,
    value: serde_json::Value,
) -> Done {
    let env: Environment = input(value)?;
    let mut parts = Vec::new();
    if let Some(image) = env.image {
        parts.push(format!("image {image}"));
    }
    if let Some(script) = env.setup_script {
        parts.push(format!(
            "a setup script of {} lines",
            script.lines().count()
        ));
    }
    if let Some(size) = env.size {
        parts.push(format!("size {size}"));
    }
    if let Some(network) = env.network {
        parts.push(format!("network {network}"));
    }
    if parts.is_empty() {
        return Err(ToolError::new(
            "name what to change: image, setup_script, size or network",
        ));
    }
    let summary = format!("Change environment: {}", parts.join(", "));
    let approval = ask(
        deps,
        caller,
        "set_environment",
        CardKind::ChangeEnvironment,
        summary,
        None,
    )
    .await?;
    Ok((waiting(&approval), None))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raise {
    amount_cents: i64,
    reason: String,
}

pub(super) async fn request_budget<D: Deps>(
    deps: &D,
    caller: &Caller,
    value: serde_json::Value,
) -> Done {
    let Raise {
        amount_cents,
        reason,
    } = input(value)?;
    if amount_cents <= 0 {
        return Err(ToolError::new("amount_cents must be more than zero"));
    }
    let summary = format!(
        "Raise the budget by ${}.{:02}: {reason}",
        amount_cents / 100,
        amount_cents % 100
    );
    let approval = ask(
        deps,
        caller,
        "request_budget",
        CardKind::RaiseBudget,
        summary,
        Some(amount_cents),
    )
    .await?;
    Ok((waiting(&approval), None))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Spawn {
    agent: String,
    prompt: String,
    environment: Option<String>,
}

pub(super) async fn spawn<D: Deps>(deps: &D, caller: &Caller, value: serde_json::Value) -> Done {
    let Spawn {
        agent,
        prompt,
        environment,
    } = input(value)?;
    let me = deps
        .store()
        .session(caller.organization, caller.session)
        .await?
        .ok_or_else(|| ToolError::new("this session is unknown"))?;
    if me.parent.is_some() {
        return Err(ToolError::new(
            "a spawned session cannot spawn: ask the lead session to start it",
        ));
    }
    let tried = super::judge::tried(deps, caller).await;
    let prompt = if tried.is_empty() {
        prompt
    } else {
        format!("{prompt}\n\nTried so far in this experiment:\n{tried}")
    };
    let session = crate::session::start(
        deps,
        caller.organization,
        caller.experiment,
        NewSession {
            agent,
            prompt,
            environment,
            parent: Some(caller.session),
            actor: caller.actor,
            ..NewSession::default()
        },
    )
    .await
    .map_err(|error| ToolError::new(error.to_string()))?;
    Ok((
        json!({ "session": session.id, "status": session.status, "agent": session.agent }),
        None,
    ))
}
