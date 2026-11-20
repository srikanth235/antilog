//! Customize: what an agent can reach beyond its machine. Connectors are remote MCP servers the
//! server calls on a session's behalf, skills are organization `SKILL.md` folders with fixed
//! versions, and a plugin is a bundle of both and of evaluation templates (`docs/V0.md`).

pub mod connectors;
pub mod plugins;
pub mod reach;
pub mod skills;
pub mod suite;

pub use connectors::{ClientError, ConnectorClient, RemoteTool};

use antilog_types::{AuditOutcome, ExperimentId, SelectConnectors, SelectSkills};

use crate::organization::{Action, Principal, Refusal, entry, permit, project_of};
use crate::store::Store;

/// The connectors an experiment switched on. Editors and owners of its project choose.
pub async fn select_connectors<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    experiment: ExperimentId,
    choice: SelectConnectors,
) -> Result<(), Refusal> {
    let project = project_of(store, who, experiment).await?;
    permit(
        store,
        who,
        Action::ExperimentReachSet,
        Some(project),
        experiment,
        now,
    )
    .await?;
    let mut ids = choice.connectors;
    ids.sort();
    ids.dedup();
    store
        .select_connectors(
            who.organization,
            experiment,
            &ids,
            &entry(
                who,
                "experiment.set_connectors",
                experiment,
                AuditOutcome::Done,
                now,
            ),
        )
        .await?;
    Ok(())
}

/// Every member of the organization reads which connectors an experiment has.
pub async fn chosen_connectors<S: Store>(
    store: &S,
    who: &Principal,
    experiment: ExperimentId,
) -> Result<Vec<antilog_types::ConnectorId>, Refusal> {
    crate::organization::member_of(store, who).await?;
    project_of(store, who, experiment).await?;
    Ok(store
        .experiment_connectors(who.organization, experiment)
        .await?)
}

/// The skills an experiment selected, each pinned to its latest version now. A skill the
/// project turned Off cannot be selected; one the experiment already holds keeps its version.
pub async fn select_skills<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    experiment: ExperimentId,
    choice: SelectSkills,
) -> Result<Vec<antilog_types::SkillRef>, Refusal> {
    let project = project_of(store, who, experiment).await?;
    permit(
        store,
        who,
        Action::ExperimentReachSet,
        Some(project),
        experiment,
        now,
    )
    .await?;
    let off = store.project_skills_off(who.organization, project).await?;
    let known = store.skills(who.organization).await?;
    let held = store
        .experiment_skills(who.organization, experiment)
        .await?;
    let mut refs: Vec<antilog_types::SkillRef> = Vec::new();
    for name in choice.skills {
        if refs.iter().any(|have| have.name == name) {
            continue;
        }
        if let Some(kept) = held.iter().find(|have| have.name == name) {
            refs.push(kept.clone());
            continue;
        }
        if off.contains(&name) {
            return Err(Refusal::Invalid(
                "that skill is Off in this project: a project owner turns it On",
            ));
        }
        let skill = known
            .iter()
            .find(|skill| skill.name == name)
            .ok_or(Refusal::NotFound)?;
        refs.push(antilog_types::SkillRef {
            name,
            version: skill.latest,
        });
    }
    store
        .select_skills(
            who.organization,
            experiment,
            &refs,
            &entry(
                who,
                "experiment.set_skills",
                experiment,
                AuditOutcome::Done,
                now,
            ),
        )
        .await?;
    Ok(refs)
}

pub async fn chosen_skills<S: Store>(
    store: &S,
    who: &Principal,
    experiment: ExperimentId,
) -> Result<Vec<antilog_types::SkillRef>, Refusal> {
    crate::organization::member_of(store, who).await?;
    project_of(store, who, experiment).await?;
    Ok(store
        .experiment_skills(who.organization, experiment)
        .await?)
}
