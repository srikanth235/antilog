//! The agents an organization turned on, and Auto.

use antilog_types::{Agent, AuditOutcome, CostTier, NewAgent, OrganizationId};

use crate::organization::{Action, Principal, Refusal, entry, member_of, permit};
use crate::store::Store;

/// The id a person or an agent writes to say "choose for me".
pub const AUTO: &str = "auto";

/// The one built-in agent: the OpenCode harness (see `docs/V0.md`).
/// Offered only while the organization has set up no agent of its own, so a first experiment
/// starts without a settings visit. An owner's list replaces it, and Auto says it is the reason.
fn built_in() -> Agent {
    Agent {
        id: "opencode".to_owned(),
        name: "OpenCode".to_owned(),
        enabled: true,
        is_default: false,
        cost_tier: CostTier::Low,
        built_in: true,
    }
}

async fn known<S: Store>(store: &S, organization: OrganizationId) -> Result<Vec<Agent>, Refusal> {
    let stored = store.agents(organization).await?;
    Ok(if stored.is_empty() {
        vec![built_in()]
    } else {
        stored
    })
}

/// Every member reads the agents.
pub async fn list<S: Store>(store: &S, who: &Principal) -> Result<Vec<Agent>, Refusal> {
    member_of(store, who).await?;
    known(store, who.organization).await
}

/// The agent Auto picks and why. Cost per win needs the judge's history of this project and
/// evaluation kind, which is not recorded yet, so before it exists Auto uses the owner's default,
/// or the only agent turned on, and never one by brand.
pub async fn auto<S: Store>(
    store: &S,
    organization: OrganizationId,
) -> Result<(Agent, String), Refusal> {
    let agents = known(store, organization).await?;
    let enabled: Vec<&Agent> = agents.iter().filter(|agent| agent.enabled).collect();
    if let Some(default) = enabled.iter().find(|agent| agent.is_default) {
        return Ok((
            (*default).clone(),
            "Your organization's default agent: there is no measured cost per win here yet."
                .to_owned(),
        ));
    }
    match enabled.as_slice() {
        [] => Err(Refusal::Invalid(
            "No agent is turned on. An owner turns one on in Customize, Agents.",
        )),
        [only] if only.built_in => Ok((
            (*only).clone(),
            "No agent is set up yet, so Antilog uses the built-in OpenCode agent.".to_owned(),
        )),
        [only] => Ok(((*only).clone(), "The only agent turned on.".to_owned())),
        _ => Err(Refusal::Invalid(
            "Several agents are turned on and none is the default: pick one, or ask an owner to set the default.",
        )),
    }
}

/// `auto` becomes the agent Auto picks; any other name is the person's own choice and stands.
pub async fn resolve<S: Store>(
    store: &S,
    organization: OrganizationId,
    requested: &str,
) -> Result<String, Refusal> {
    if requested == AUTO {
        Ok(auto(store, organization).await?.0.id)
    } else {
        Ok(requested.to_owned())
    }
}

fn slug_ok(id: &str) -> bool {
    id != AUTO
        && (1..=64).contains(&id.len())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !id.starts_with('-')
}

/// Only an organization owner adds or changes an agent; making one the default turns it on.
pub async fn set<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    id: &str,
    new: NewAgent,
) -> Result<Agent, Refusal> {
    if !slug_ok(id) {
        return Err(Refusal::Invalid(
            "an agent's id is lower-case letters, digits and hyphens, and is not \"auto\"",
        ));
    }
    if !(1..=200).contains(&new.name.trim().chars().count()) {
        return Err(Refusal::Invalid("an agent's name is 1 to 200 characters"));
    }
    if new.is_default && !new.enabled {
        return Err(Refusal::Invalid("the default agent must be turned on"));
    }
    permit(store, who, Action::AgentSet, id, now).await?;
    let agent = Agent {
        id: id.to_owned(),
        name: new.name.trim().to_owned(),
        enabled: new.enabled,
        is_default: new.is_default,
        cost_tier: new.cost_tier,
        built_in: false,
    };
    store
        .put_agent(
            who.organization,
            &agent,
            now,
            &entry(who, "agent.set", id, AuditOutcome::Done, now),
        )
        .await?;
    Ok(agent)
}
