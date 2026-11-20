use antilog_types::SessionId;
use antilog_types::judge::{
    CheatCheck, CommitRef, Definition, ExperimentId, Outcome, ProtectedFile, SealChecks,
    SealRefused, SealedExperiment,
};

use super::paired::{Data, PairedRun, RunError, run_paired};
use super::run::{Env, Sides, evaluate};
use super::stats::noise;
use crate::clock::Clock;
use crate::machine::SessionMachine;
use crate::store::judge::{JudgeStore, JudgeStoreError};

/// Code the proposer plants to see whether the evaluation rejects it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlantedCheat {
    pub name: String,
    pub commits: Vec<CommitRef>,
    pub changed: Vec<(String, String)>,
}

#[derive(Debug, thiserror::Error)]
pub enum SealError {
    #[error(transparent)]
    Refused(#[from] SealRefused),
    #[error(transparent)]
    Run(#[from] RunError),
    #[error("the baseline measured too few or unusable repeats")]
    Unusable,
    #[error(transparent)]
    Store(#[from] JudgeStoreError),
}

fn baseline(def: &Definition) -> Vec<CommitRef> {
    def.evaluation
        .repos
        .iter()
        .map(|p| CommitRef {
            repo: p.repo.clone(),
            sha: p.commit.clone(),
        })
        .collect()
}

/// The baseline measurement, the noise check (the commit paired with itself) and the
/// planted cheats. A definition that fails any of them gets no `SealChecks`.
pub async fn run_checks<M: SessionMachine>(
    machine: &M,
    session: SessionId,
    definition: &Definition,
    files: &[ProtectedFile],
    cheats: &[PlantedCheat],
) -> Result<SealChecks, SealError> {
    let problems = definition.problems();
    if !problems.is_empty() {
        return Err(SealRefused::Definition(problems).into());
    }
    let base = baseline(definition);
    let pairs = run_paired(
        machine,
        session,
        &PairedRun {
            command: &definition.evaluation.command,
            files,
            data: Data::Evaluation(&definition.data.evaluation),
            base: &base,
            trial: &base,
            repeats: definition.repeats,
            guardrails: &definition.guardrails,
        },
    )
    .await?;
    let direction = definition.metric.direction;
    let diffs: Vec<f64> = pairs
        .metric
        .iter()
        .map(|(b, t)| direction.gain(*b, *t))
        .collect();
    let spread = noise(&diffs).ok_or(SealError::Unusable)?;
    let n = f64::from(u32::try_from(pairs.metric.len()).map_err(|_| SealError::Unusable)?);
    let value = pairs.metric.iter().map(|p| p.0).sum::<f64>() / n;

    let mut checked = Vec::new();
    for cheat in cheats {
        let mut commits = base.clone();
        for c in &cheat.commits {
            match commits.iter_mut().find(|x| x.repo == c.repo) {
                Some(slot) => slot.sha.clone_from(&c.sha),
                None => commits.push(c.clone()),
            }
        }
        let ran = evaluate(
            machine,
            session,
            Env { definition, files },
            std::num::NonZeroU32::MIN,
            spread,
            definition.touched(&cheat.changed),
            Sides {
                best: &base,
                parent: &base,
                trial: &commits,
            },
        )
        .await;
        checked.push(CheatCheck {
            name: cheat.name.clone(),
            rejected: ran.decision.outcome != Outcome::Kept,
        });
    }
    Ok(SealChecks::passed(value, spread, checked)?)
}

/// What a person approves on the Seal experiment card.
#[derive(Debug, Clone)]
pub struct Proposal {
    pub experiment: ExperimentId,
    pub version: u32,
    pub definition: Definition,
    pub files: Vec<ProtectedFile>,
    pub cheats: Vec<PlantedCheat>,
}

/// Runs the seal checks, seals the definition as one new version and stores it.
/// Call it only after a person approved the card.
pub async fn seal<M: SessionMachine, J: JudgeStore, C: Clock>(
    machine: &M,
    session: SessionId,
    store: &J,
    clock: &C,
    proposal: Proposal,
) -> Result<SealedExperiment, SealError> {
    let Proposal {
        experiment,
        version,
        definition,
        files,
        cheats,
    } = proposal;
    let checks = run_checks(machine, session, &definition, &files, &cheats).await?;
    let sealed = SealedExperiment::seal(
        experiment,
        version,
        definition,
        files,
        checks,
        clock.now().as_millisecond(),
    )?;
    store.put_sealed(&sealed).await?;
    Ok(sealed)
}
