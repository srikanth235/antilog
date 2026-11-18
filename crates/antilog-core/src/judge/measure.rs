use std::num::NonZeroU32;

use antilog_types::judge::{
    CommitRef, Direction, Finished, Measurement, Outcome, SealedExperiment, Trial, TrialId,
    TrialRecord,
};

use super::run::{Env, Sides, evaluate};
use crate::platform::Platform;
use crate::store::judge::{JudgeStore, JudgeStoreError};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Refusal {
    #[error("the trial belongs to another experiment version")]
    WrongVersion,
    #[error("the trial changes a repo the experiment does not name")]
    UnknownRepo,
    #[error("the trial starts from a trial that is not a Kept trial of this experiment version")]
    BadParent,
    #[error("patience is reached: {0} trials in a row were not Kept")]
    PatienceReached(u32),
    #[error("the holdout cap of {0} checks is reached; a new version with a new holdout is needed")]
    HoldoutCapReached(u32),
}

#[derive(Debug, thiserror::Error)]
pub enum MeasureError {
    #[error(transparent)]
    Refused(#[from] Refusal),
    #[error(transparent)]
    Store(#[from] JudgeStoreError),
}

fn value_of(record: &TrialRecord) -> Option<f64> {
    record
        .finished
        .as_ref()?
        .measurement
        .as_ref()
        .map(|m| m.value)
}

/// The experiment's best Kept trial; the earliest wins a tie.
pub fn best(records: &[TrialRecord], direction: Direction) -> Option<&TrialRecord> {
    records
        .iter()
        .filter(|r| r.outcome() == Outcome::Kept)
        .filter_map(|r| value_of(r).map(|v| (r, v)))
        .fold(None, |top: Option<(&TrialRecord, f64)>, (r, v)| match top {
            Some((_, tv)) if direction.gain(tv, v) <= 0.0 => top,
            _ => Some((r, v)),
        })
        .map(|(r, _)| r)
}

/// The full commit set of a trial: the pinned baseline with each trial on its lineage laid over it.
pub fn commits_at(
    sealed: &SealedExperiment,
    records: &[TrialRecord],
    at: Option<TrialId>,
    overlay: &[CommitRef],
) -> Vec<CommitRef> {
    let mut chain = Vec::new();
    let mut next = at;
    while let Some(id) = next {
        let Some(record) = records.iter().find(|r| r.trial.id == id) else {
            break;
        };
        chain.push(&record.trial.commits);
        next = record.trial.parent;
    }
    let mut state: Vec<CommitRef> = sealed
        .definition()
        .evaluation
        .repos
        .iter()
        .map(|p| CommitRef {
            repo: p.repo.clone(),
            sha: p.commit.clone(),
        })
        .collect();
    for commits in chain.into_iter().rev().chain([&overlay.to_vec()]) {
        for commit in commits {
            match state.iter_mut().find(|c| c.repo == commit.repo) {
                Some(slot) => slot.sha.clone_from(&commit.sha),
                None => state.push(commit.clone()),
            }
        }
    }
    state
}

fn trailing_unkept(records: &[TrialRecord]) -> u32 {
    let n = records
        .iter()
        .rev()
        .filter(|r| r.outcome() != Outcome::Measuring)
        .take_while(|r| r.outcome() != Outcome::Kept)
        .count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Measures one trial against a sealed version and writes its outcome once.
/// `changed` is `(repo, path)` for every file the trial's commits change.
pub async fn measure<S: Platform, J: JudgeStore>(
    sandbox: &S,
    store: &J,
    sealed: &SealedExperiment,
    trial: Trial,
    changed: &[(String, String)],
) -> Result<TrialRecord, MeasureError> {
    let def = sealed.definition();
    if trial.experiment != sealed.experiment() || trial.version != sealed.version() {
        return Err(Refusal::WrongVersion.into());
    }
    if !sealed.knows_repos(&trial.commits) {
        return Err(Refusal::UnknownRepo.into());
    }
    let records = store.trials(sealed.experiment(), sealed.version()).await?;
    if let Some(parent) = trial.parent {
        let kept = records
            .iter()
            .any(|r| r.trial.id == parent && r.outcome() == Outcome::Kept);
        if !kept {
            return Err(Refusal::BadParent.into());
        }
    }
    if trailing_unkept(&records) >= def.patience {
        return Err(Refusal::PatienceReached(def.patience).into());
    }
    let holdout_checks = records
        .iter()
        .filter(|r| {
            r.finished
                .as_ref()
                .and_then(|f| f.measurement.as_ref())
                .is_some_and(|m| m.holdout.is_some())
        })
        .count();
    if u32::try_from(holdout_checks).unwrap_or(u32::MAX) >= def.holdout_cap {
        return Err(Refusal::HoldoutCapReached(def.holdout_cap).into());
    }
    let counted = records
        .iter()
        .filter(|r| !matches!(r.outcome(), Outcome::Rejected | Outcome::Failed))
        .count();
    let k = u32::try_from(counted)
        .ok()
        .and_then(|n| n.checked_add(1))
        .and_then(NonZeroU32::new)
        .unwrap_or(NonZeroU32::MAX);

    let best_commits = commits_at(
        sealed,
        &records,
        best(&records, def.metric.direction).map(|r| r.trial.id),
        &[],
    );
    let parent_commits = commits_at(sealed, &records, trial.parent, &[]);
    let trial_commits = commits_at(sealed, &records, trial.parent, &trial.commits);

    store.begin_trial(&trial).await?;
    let ran = evaluate(
        sandbox,
        Env {
            definition: def,
            files: sealed.files(),
            seal: Some(sealed.seal_hash()),
        },
        k,
        sealed.checks().noise(),
        def.touched(changed),
        Sides {
            best: &best_commits,
            parent: &parent_commits,
            trial: &trial_commits,
        },
    )
    .await;
    let measurement = ran.measured.then(|| Measurement {
        trial: trial.id,
        experiment: sealed.experiment(),
        version: sealed.version(),
        seal: sealed.seal_hash().clone(),
        stage: ran.stage,
        k: k.get(),
        noise: sealed.checks().noise(),
        parent_value: ran.parent_value,
        value: ran.value,
        gain: ran.gain,
        confirmation: ran.confirmation,
        guardrails: ran.guardrails,
        holdout: ran.holdout,
    });
    let finished = Finished {
        measurement,
        decision: ran.decision,
    };
    store.finish_trial(trial.id, &finished).await?;
    Ok(TrialRecord {
        trial,
        finished: Some(finished),
    })
}
