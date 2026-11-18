use std::collections::HashMap;
use std::future::Future;
use std::sync::{Mutex, MutexGuard, PoisonError};

use antilog_types::judge::{
    ExperimentId, Finished, Receipt, SealedExperiment, Trial, TrialId, TrialRecord, ValueModel,
    ValueModelId,
};

#[derive(Debug, thiserror::Error)]
pub enum JudgeStoreError {
    #[error("already exists")]
    Conflict,
    #[error("not found")]
    NotFound,
    #[error("store unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Everything the judge writes is write-once: a sealed version, a trial's outcome, a receipt.
pub trait JudgeStore: Send + Sync {
    fn put_sealed(
        &self,
        sealed: &SealedExperiment,
    ) -> impl Future<Output = Result<(), JudgeStoreError>> + Send;

    fn sealed(
        &self,
        experiment: ExperimentId,
        version: u32,
    ) -> impl Future<Output = Result<Option<SealedExperiment>, JudgeStoreError>> + Send;

    /// The trial starts with the outcome Measuring.
    fn begin_trial(
        &self,
        trial: &Trial,
    ) -> impl Future<Output = Result<(), JudgeStoreError>> + Send;

    /// Writes the outcome once. `Conflict` when it was already written.
    fn finish_trial(
        &self,
        trial: TrialId,
        finished: &Finished,
    ) -> impl Future<Output = Result<(), JudgeStoreError>> + Send;

    fn trial(
        &self,
        id: TrialId,
    ) -> impl Future<Output = Result<Option<TrialRecord>, JudgeStoreError>> + Send;

    /// In the order the trials began.
    fn trials(
        &self,
        experiment: ExperimentId,
        version: u32,
    ) -> impl Future<Output = Result<Vec<TrialRecord>, JudgeStoreError>> + Send;

    fn put_receipt(
        &self,
        receipt: &Receipt,
    ) -> impl Future<Output = Result<(), JudgeStoreError>> + Send;

    fn receipt(
        &self,
        trial: TrialId,
    ) -> impl Future<Output = Result<Option<Receipt>, JudgeStoreError>> + Send;

    fn put_value_model(
        &self,
        model: &ValueModel,
    ) -> impl Future<Output = Result<(), JudgeStoreError>> + Send;

    fn value_model(
        &self,
        id: ValueModelId,
        version: u32,
    ) -> impl Future<Output = Result<Option<ValueModel>, JudgeStoreError>> + Send;
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Default)]
struct State {
    sealed: HashMap<(ExperimentId, u32), SealedExperiment>,
    trials: Vec<TrialRecord>,
    receipts: HashMap<TrialId, Receipt>,
    models: HashMap<(ValueModelId, u32), ValueModel>,
}

#[derive(Default)]
pub struct FakeJudgeStore(Mutex<State>);

impl JudgeStore for FakeJudgeStore {
    async fn put_sealed(&self, sealed: &SealedExperiment) -> Result<(), JudgeStoreError> {
        let mut state = locked(&self.0);
        let key = (sealed.experiment(), sealed.version());
        if state.sealed.contains_key(&key) {
            return Err(JudgeStoreError::Conflict);
        }
        state.sealed.insert(key, sealed.clone());
        Ok(())
    }

    async fn sealed(
        &self,
        experiment: ExperimentId,
        version: u32,
    ) -> Result<Option<SealedExperiment>, JudgeStoreError> {
        Ok(locked(&self.0).sealed.get(&(experiment, version)).cloned())
    }

    async fn begin_trial(&self, trial: &Trial) -> Result<(), JudgeStoreError> {
        let mut state = locked(&self.0);
        if state.trials.iter().any(|r| r.trial.id == trial.id) {
            return Err(JudgeStoreError::Conflict);
        }
        state.trials.push(TrialRecord {
            trial: trial.clone(),
            finished: None,
        });
        Ok(())
    }

    async fn finish_trial(
        &self,
        trial: TrialId,
        finished: &Finished,
    ) -> Result<(), JudgeStoreError> {
        let mut state = locked(&self.0);
        let record = state
            .trials
            .iter_mut()
            .find(|r| r.trial.id == trial)
            .ok_or(JudgeStoreError::NotFound)?;
        if record.finished.is_some() {
            return Err(JudgeStoreError::Conflict);
        }
        record.finished = Some(finished.clone());
        Ok(())
    }

    async fn trial(&self, id: TrialId) -> Result<Option<TrialRecord>, JudgeStoreError> {
        Ok(locked(&self.0)
            .trials
            .iter()
            .find(|r| r.trial.id == id)
            .cloned())
    }

    async fn trials(
        &self,
        experiment: ExperimentId,
        version: u32,
    ) -> Result<Vec<TrialRecord>, JudgeStoreError> {
        Ok(locked(&self.0)
            .trials
            .iter()
            .filter(|r| r.trial.experiment == experiment && r.trial.version == version)
            .cloned()
            .collect())
    }

    async fn put_receipt(&self, receipt: &Receipt) -> Result<(), JudgeStoreError> {
        let mut state = locked(&self.0);
        let trial = receipt.body().experiment.trial;
        if state.receipts.contains_key(&trial) {
            return Err(JudgeStoreError::Conflict);
        }
        state.receipts.insert(trial, receipt.clone());
        Ok(())
    }

    async fn receipt(&self, trial: TrialId) -> Result<Option<Receipt>, JudgeStoreError> {
        Ok(locked(&self.0).receipts.get(&trial).cloned())
    }

    async fn put_value_model(&self, model: &ValueModel) -> Result<(), JudgeStoreError> {
        let mut state = locked(&self.0);
        let key = (model.id, model.version);
        if state.models.contains_key(&key) {
            return Err(JudgeStoreError::Conflict);
        }
        state.models.insert(key, model.clone());
        Ok(())
    }

    async fn value_model(
        &self,
        id: ValueModelId,
        version: u32,
    ) -> Result<Option<ValueModel>, JudgeStoreError> {
        Ok(locked(&self.0).models.get(&(id, version)).cloned())
    }
}

/// One suite for every `JudgeStore`, so the Postgres store can run it too.
#[expect(
    clippy::expect_used,
    reason = "test support: a failed call fails the suite"
)]
pub async fn judge_store_suite<S: JudgeStore>(store: &S) {
    use crate::judge::fixtures;
    use antilog_types::judge::{Decision, Outcome};

    let sealed = fixtures::sealed();
    store.put_sealed(&sealed).await.expect("seal is stored");
    assert!(matches!(
        store.put_sealed(&sealed).await,
        Err(JudgeStoreError::Conflict)
    ));
    assert_eq!(
        store
            .sealed(sealed.experiment(), 1)
            .await
            .expect("read")
            .as_ref(),
        Some(&sealed)
    );
    assert_eq!(
        store.sealed(sealed.experiment(), 2).await.expect("read"),
        None
    );

    let first = fixtures::trial(&sealed, None, "a");
    let second = fixtures::trial(&sealed, Some(first.id), "b");
    store.begin_trial(&first).await.expect("begin");
    store.begin_trial(&second).await.expect("begin");
    assert!(matches!(
        store.begin_trial(&first).await,
        Err(JudgeStoreError::Conflict)
    ));
    let all = store.trials(sealed.experiment(), 1).await.expect("list");
    assert_eq!(
        all.iter().map(|r| r.trial.id).collect::<Vec<_>>(),
        vec![first.id, second.id]
    );
    assert_eq!(
        all.first().map(TrialRecord::outcome),
        Some(Outcome::Measuring)
    );

    let finished = Finished {
        measurement: None,
        decision: Decision {
            outcome: Outcome::Rejected,
            reasons: vec!["changed a protected path".into()],
            criteria: 1,
        },
    };
    store
        .finish_trial(first.id, &finished)
        .await
        .expect("finish");
    let again = Finished {
        decision: Decision {
            outcome: Outcome::Kept,
            ..finished.decision.clone()
        },
        ..finished.clone()
    };
    assert!(matches!(
        store.finish_trial(first.id, &again).await,
        Err(JudgeStoreError::Conflict)
    ));
    assert!(matches!(
        store.finish_trial(TrialId::new(), &finished).await,
        Err(JudgeStoreError::NotFound)
    ));
    let read = store.trial(first.id).await.expect("read").expect("exists");
    assert_eq!(read.outcome(), Outcome::Rejected);
    assert_eq!(
        store.trials(sealed.experiment(), 2).await.expect("list"),
        vec![]
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_judge_store_passes_the_suite() {
        judge_store_suite(&FakeJudgeStore::default()).await;
    }
}
