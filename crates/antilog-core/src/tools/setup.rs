use std::collections::BTreeMap;

use antilog_types::{
    Approval, ApprovalStatus, CardKind, CardPayload, EnvironmentChange, EnvironmentProposal,
    EnvironmentView, NetworkLevel, RepoAccess, SessionStatus,
};
use serde::Deserialize;
use serde_json::json;

use super::{Caller, Done, ToolError, input};
use crate::approval::{self, CardRequest};
use crate::deps::Deps;
use crate::environments;
use crate::organization::Refusal;
use crate::session::now_ms;
use crate::store::{SessionChange, SessionStore};

/// A card in front of a person, or approved by policy when the session follows it.
/// The agent gets the answer in the tool's reply either way.
pub(crate) struct Card {
    pub kind: CardKind,
    pub summary: String,
    pub payload: CardPayload,
    pub cost_cents: Option<i64>,
}

pub(crate) async fn ask<D: Deps>(
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
    } = card;
    let now = now_ms(deps);
    let session = deps
        .store()
        .session(caller.organization, caller.session)
        .await?
        .ok_or_else(|| ToolError::new("this session is unknown"))?;
    let approval = approval::raise(
        deps.store(),
        now,
        Some(caller.actor),
        CardRequest {
            organization: caller.organization,
            session: caller.session,
            card: kind,
            summary,
            payload: Some(payload),
            cost_cents,
            follow_policy: session.follow_policy,
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

/// Applies a card the session's policy approved at once; a card in front of a person is
/// applied when they decide it.
pub(super) async fn follow<D: Deps>(deps: &D, caller: &Caller, approval: &Approval) {
    if approval.status != ApprovalStatus::Pending {
        let _ = crate::session::settle(deps, caller.organization, caller.session).await;
    }
}

pub(crate) async fn waiting<D: Deps>(
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
        },
    )
    .await?;
    Ok(waiting(deps, caller, &approval).await)
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
    if ["write", "admin"].iter().any(|word| lower.contains(word)) {
        return Err(ToolError::new(
            "a credential is read-only: ask for a read scope",
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
        },
    )
    .await?;
    Ok(waiting(deps, caller, &approval).await)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Environment {
    network: Option<String>,
    #[serde(default)]
    add_hosts: Vec<String>,
    #[serde(default)]
    remove_hosts: Vec<String>,
    #[serde(default)]
    set_variables: BTreeMap<String, String>,
    #[serde(default)]
    unset_variables: Vec<String>,
    setup_script: Option<String>,
    #[serde(default)]
    add_repos: Vec<String>,
    #[serde(default)]
    remove_repos: Vec<String>,
}

/// The built-in default cannot change, so a session on it gets an environment of its own, named
/// from its id.
pub(crate) fn own_name(session: antilog_types::SessionId) -> String {
    let id = session.to_string();
    format!("Session {}", id.get(..8).unwrap_or(&id))
}

pub(super) async fn set_environment<D: Deps>(
    deps: &D,
    caller: &Caller,
    value: serde_json::Value,
) -> Done {
    let env: Environment = input(value)?;
    let network = match env.network.as_deref() {
        None => None,
        Some(level) => Some(
            NetworkLevel::parse(level)
                .ok_or_else(|| ToolError::new("network is none, limited or full"))?,
        ),
    };
    let change = EnvironmentChange {
        network,
        add_hosts: env.add_hosts,
        remove_hosts: env.remove_hosts,
        set_variables: env.set_variables,
        unset_variables: env.unset_variables,
        setup_script: env.setup_script,
        add_repos: env.add_repos,
        remove_repos: env.remove_repos,
    };
    if change.is_empty() {
        return Err(ToolError::new(
            "name what to change: network, add_hosts, remove_hosts, set_variables, unset_variables, setup_script, add_repos or remove_repos",
        ));
    }
    let session = deps
        .store()
        .session(caller.organization, caller.session)
        .await?
        .ok_or_else(|| ToolError::new("this session is unknown"))?;
    let refused = |error: Refusal| match error {
        Refusal::Invalid(message) => ToolError::new(message),
        _ => ToolError::new("the session's environment cannot be read now: retry in a minute"),
    };
    let current = environments::of_session(deps.store(), caller.organization, &session)
        .await
        .map_err(refused)?;
    let newest = environments::newest_of_session(deps.store(), caller.organization, &session)
        .await
        .map_err(refused)?;
    let creates_environment = newest.built_in;
    let name = if creates_environment {
        own_name(session.id)
    } else {
        newest.name.clone()
    };
    let after = environments::changed(&newest, &change, &name).map_err(refused)?;
    let after = EnvironmentView {
        network: after.network,
        allowed_hosts: after.allowed_hosts,
        allow_package_managers: after.allow_package_managers,
        variables: after.variables,
        setup_script: after.setup_script,
        repos: after.repos,
    };
    let before = environments::view(&current);
    if after == before {
        return Err(ToolError::new(
            "the session's environment already has all of that: nothing would change",
        ));
    }
    let mut parts = Vec::new();
    if let Some(network) = change.network {
        parts.push(format!("network {}", network.as_str()));
    }
    for (what, count) in [
        ("host added", change.add_hosts.len()),
        ("host removed", change.remove_hosts.len()),
        ("variable set", change.set_variables.len()),
        ("variable removed", change.unset_variables.len()),
        ("repo added", change.add_repos.len()),
        ("repo removed", change.remove_repos.len()),
    ] {
        if count > 0 {
            parts.push(format!("{count} {what}"));
        }
    }
    if let Some(script) = &change.setup_script {
        parts.push(format!(
            "a setup script of {} lines",
            script.lines().count()
        ));
    }
    let summary = format!("Change environment {name}: {}", parts.join(", "));
    let payload = CardPayload::ChangeEnvironment(Box::new(EnvironmentProposal {
        environment: newest.id.clone(),
        environment_name: name,
        session_version: current.version,
        base_version: newest.version,
        creates_environment,
        change,
        before,
        after,
    }));
    let approval = ask(
        deps,
        caller,
        "set_environment",
        Card {
            kind: CardKind::ChangeEnvironment,
            summary,
            payload,
            cost_cents: None,
        },
    )
    .await?;
    Ok(waiting(deps, caller, &approval).await)
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
    let Some(now_cents) = deps
        .store()
        .session(caller.organization, caller.session)
        .await?
        .and_then(|row| row.budget_cap_cents)
    else {
        return Err(ToolError::new(
            "this session has no budget of its own, only the organization's monthly cap, which an owner changes in Settings",
        ));
    };
    let summary = format!(
        "Raise this session's budget by ${}.{:02}: {reason}",
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
        },
    )
    .await?;
    Ok(waiting(deps, caller, &approval).await)
}
