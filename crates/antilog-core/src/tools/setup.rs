use antilog_types::{Approval, ApprovalStatus, CardKind, CardPayload, RepoAccess, SessionStatus};
use serde::Deserialize;
use serde_json::json;

use super::{Caller, Done, ToolError, input};
use crate::approval::{self, CardRequest};
use crate::deps::Deps;
use crate::organization::Refusal;
use crate::session::{NewSession, now_ms};
use crate::store::{SessionChange, SessionStore};

/// A card in front of a person, or approved by policy when the experiment follows it.
/// The agent gets the answer in the tool's reply either way.
pub(super) struct Card {
    pub kind: CardKind,
    pub summary: String,
    pub payload: CardPayload,
    pub cost_cents: Option<i64>,
    pub needs_owner: bool,
}

pub(super) async fn ask<D: Deps>(
    deps: &D,
    caller: &Caller,
    tool: &str,
    card: Card,
) -> Result<Approval, ToolError> {
    let Card {
        kind,
        summary,
        payload,
        cost_cents,
        needs_owner,
    } = card;
    let now = now_ms(deps);
    let experiment = deps
        .store()
        .experiment(caller.organization, caller.experiment)
        .await?
        .ok_or_else(|| ToolError::new("this experiment is unknown"))?;
    let approval = approval::raise(
        deps.store(),
        now,
        caller.actor,
        CardRequest {
            organization: caller.organization,
            experiment: caller.experiment,
            session: Some(caller.session),
            card: kind,
            summary,
            payload: Some(payload),
            cost_cents,
            needs_owner,
            follow_policy: experiment.follow_policy,
        },
    )
    .await
    .map_err(|refusal| match refusal {
        Refusal::Invalid(message) => ToolError::new(message),
        _ => ToolError::new(format!(
            "{tool} failed: the store is unavailable, retry in a minute"
        )),
    })?;
    if approval.status == ApprovalStatus::Pending {
        let waiting = SessionChange {
            status: Some(SessionStatus::WaitingForYou),
            ..SessionChange::default()
        };
        deps.store()
            .update_session(caller.organization, caller.session, &waiting, now, None)
            .await?;
    }
    Ok(approval)
}

/// Applies a card the experiment's policy approved at once; a card in front of a person is
/// applied when they decide it.
pub(super) async fn follow<D: Deps>(deps: &D, caller: &Caller, approval: &Approval) {
    if approval.status != ApprovalStatus::Pending {
        let _ = crate::session::settle(deps, caller.organization, caller.session).await;
    }
}

pub(super) async fn waiting<D: Deps>(
    deps: &D,
    caller: &Caller,
    approval: &Approval,
) -> serde_json::Value {
    follow(deps, caller, approval).await;
    if approval.status == ApprovalStatus::Pending {
        json!({
            "approval": approval.id,
            "status": "waiting for a person",
            "note": "A card is in front of a person. Do not call this tool again for it."
        })
    } else {
        json!({
            "approval": approval.id,
            "status": "approved by policy",
            "note": "Policy approved it. Do not call this tool again for it."
        })
    }
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
        Card {
            kind: CardKind::AttachRepo,
            summary: format!("Attach repo {repo}"),
            payload: CardPayload::AttachRepo {
                repo,
                access: RepoAccess::Push,
                branch: None,
            },
            cost_cents: None,
            needs_owner: false,
        },
    )
    .await?;
    Ok((waiting(deps, caller, &approval).await, None))
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
    let payload = CardPayload::AddCredential {
        connector,
        scope,
        why: reason,
    };
    let approval = ask(
        deps,
        caller,
        "request_credential",
        Card {
            kind: CardKind::AddCredential,
            summary,
            payload,
            cost_cents: None,
            needs_owner: false,
        },
    )
    .await?;
    Ok((waiting(deps, caller, &approval).await, None))
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
    let payload = CardPayload::ChangeEnvironment {
        image: env.image.clone(),
        setup_script_lines: env
            .setup_script
            .as_ref()
            .map(|script| u32::try_from(script.lines().count()).unwrap_or(u32::MAX)),
        size: env.size.clone(),
        network: env.network.clone(),
    };
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
        Card {
            kind: CardKind::ChangeEnvironment,
            summary,
            payload,
            cost_cents: None,
            needs_owner: false,
        },
    )
    .await?;
    Ok((waiting(deps, caller, &approval).await, None))
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
    let now_cents = deps
        .store()
        .experiment(caller.organization, caller.experiment)
        .await?
        .map_or(0, |row| row.budget_cap_cents);
    let summary = format!(
        "Raise the budget by ${}.{:02}: {reason}",
        amount_cents / 100,
        amount_cents % 100
    );
    let payload = CardPayload::RaiseBudget {
        now_cents,
        requested_cents: amount_cents,
        why: reason,
    };
    let approval = ask(
        deps,
        caller,
        "request_budget",
        Card {
            kind: CardKind::RaiseBudget,
            summary,
            payload,
            cost_cents: Some(amount_cents),
            needs_owner: false,
        },
    )
    .await?;
    Ok((waiting(deps, caller, &approval).await, None))
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
