use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use antilog_types::judge::{
    ExperimentId, Finished, ProjectValueModel, Receipt, SealedExperiment, Trial, TrialId,
    TrialRecord, ValueModel, ValueModelId, WinEvent,
};
use antilog_types::{AuditEntry, ProjectId};

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

    /// A new version of a project's model for a metric. `Conflict` when that version exists.
    fn put_value_model(
        &self,
        project: ProjectId,
        model: &ValueModel,
        at: i64,
    ) -> impl Future<Output = Result<(), JudgeStoreError>> + Send;

    /// The same, with the audit entry for it: both land or neither does, in one transaction.
    fn put_value_model_audited(
        &self,
        project: ProjectId,
        model: &ValueModel,
        at: i64,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), JudgeStoreError>> + Send;

    fn value_model(
        &self,
        id: ValueModelId,
        version: u32,
    ) -> impl Future<Output = Result<Option<ProjectValueModel>, JudgeStoreError>> + Send;

    /// The newest version of each metric's model in the project, by metric.
    fn value_models(
        &self,
        project: ProjectId,
    ) -> impl Future<Output = Result<Vec<ProjectValueModel>, JudgeStoreError>> + Send;

    /// Appends a fact about a win: its pull request opened, merged or closed, or the watch
    /// found it regressed or recovered. Nothing is ever changed or removed.
    fn put_win_event(
        &self,
        event: &WinEvent,
    ) -> impl Future<Output = Result<(), JudgeStoreError>> + Send;

    /// In the order they were appended.
    fn win_events(
        &self,
        experiment: ExperimentId,
    ) -> impl Future<Output = Result<Vec<WinEvent>, JudgeStoreError>> + Send;
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Default)]
struct State {
    sealed: HashMap<(ExperimentId, u32), SealedExperiment>,
    trials: Vec<TrialRecord>,
    receipts: HashMap<TrialId, Receipt>,
    models: HashMap<(ValueModelId, u32), ProjectValueModel>,
    win_events: Vec<WinEvent>,
}

/// Where a fake judge store puts the audit entries of its audited writes.
type Audit = Arc<Mutex<crate::fakes::ledger::Ledger>>;

#[derive(Default, Clone)]
pub struct FakeJudgeStore(Arc<Mutex<State>>, Option<Audit>);

impl FakeJudgeStore {
    pub(crate) fn linked(ledger: Audit) -> Self {
        Self(Arc::default(), Some(ledger))
    }

    fn insert_model(
        &self,
        project: ProjectId,
        model: &ValueModel,
        at: i64,
    ) -> Result<(), JudgeStoreError> {
        let mut state = locked(&self.0);
        let key = (model.id, model.version);
        let metric_taken = state.models.values().any(|row| {
            row.project_id == project
                && row.model.metric == model.metric
                && row.model.version == model.version
        });
        if state.models.contains_key(&key) || metric_taken {
            return Err(JudgeStoreError::Conflict);
        }
        state.models.insert(
            key,
            ProjectValueModel {
                project_id: project,
                model: model.clone(),
                set_at: at,
            },
        );
        Ok(())
    }
}

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

    async fn put_value_model(
        &self,
        project: ProjectId,
        model: &ValueModel,
        at: i64,
    ) -> Result<(), JudgeStoreError> {
        self.insert_model(project, model, at)
    }

    async fn put_value_model_audited(
        &self,
        project: ProjectId,
        model: &ValueModel,
        at: i64,
        audit: &AuditEntry,
    ) -> Result<(), JudgeStoreError> {
        // The audit log is checked before anything is written, and both locks are held to the end.
        let mut ledger = self.1.as_ref().map(|ledger| locked(ledger));
        if let Some(ledger) = &ledger
            && (ledger.organization(audit.organization_id).is_none()
                || ledger.audit.iter().any(|row| row.id == audit.id))
        {
            return Err(JudgeStoreError::Conflict);
        }
        self.insert_model(project, model, at)?;
        if let Some(ledger) = &mut ledger {
            ledger.audit.push(audit.clone());
        }
        Ok(())
    }

    async fn value_model(
        &self,
        id: ValueModelId,
        version: u32,
    ) -> Result<Option<ProjectValueModel>, JudgeStoreError> {
        Ok(locked(&self.0).models.get(&(id, version)).cloned())
    }

    async fn value_models(
        &self,
        project: ProjectId,
    ) -> Result<Vec<ProjectValueModel>, JudgeStoreError> {
        let state = locked(&self.0);
        let mut newest: Vec<ProjectValueModel> = Vec::new();
        for row in state
            .models
            .values()
            .filter(|row| row.project_id == project)
        {
            match newest
                .iter_mut()
                .find(|kept| kept.model.metric == row.model.metric)
            {
                Some(kept) if kept.model.version < row.model.version => *kept = row.clone(),
                Some(_) => {}
                None => newest.push(row.clone()),
            }
        }
        newest.sort_by(|a, b| a.model.metric.cmp(&b.model.metric));
        Ok(newest)
    }

    async fn put_win_event(&self, event: &WinEvent) -> Result<(), JudgeStoreError> {
        let mut state = locked(&self.0);
        if !state
            .trials
            .iter()
            .any(|record| record.trial.id == event.trial)
        {
            return Err(JudgeStoreError::NotFound);
        }
        state.win_events.push(event.clone());
        Ok(())
    }

    async fn win_events(&self, experiment: ExperimentId) -> Result<Vec<WinEvent>, JudgeStoreError> {
        Ok(locked(&self.0)
            .win_events
            .iter()
            .filter(|event| event.experiment == experiment)
            .cloned()
            .collect())
    }
}

/// One suite for every `JudgeStore`, so the Postgres store can run it too.
#[expect(
    clippy::expect_used,
    reason = "test support: a failed call fails the suite"
)]
pub async fn judge_store_suite<S: JudgeStore>(
    store: &S,
    project: ProjectId,
    experiment: antilog_types::ExperimentId,
) {
    use crate::judge::fixtures;
    use antilog_types::judge::{Decision, Outcome, WinEventKind};

    let sealed = fixtures::sealed_for(experiment);
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

    let receipt = fixtures::receipt(&sealed, first.id);
    assert_eq!(store.receipt(first.id).await.expect("read"), None);
    store.put_receipt(&receipt).await.expect("receipt");
    assert!(matches!(
        store.put_receipt(&receipt).await,
        Err(JudgeStoreError::Conflict)
    ));
    assert_eq!(store.receipt(first.id).await.expect("read"), Some(receipt));

    let model = fixtures::value_model(1);
    store
        .put_value_model(project, &model, 10)
        .await
        .expect("model");
    assert!(matches!(
        store.put_value_model(project, &model, 11).await,
        Err(JudgeStoreError::Conflict)
    ));
    let read = store
        .value_model(model.id, 1)
        .await
        .expect("read")
        .expect("exists");
    assert_eq!(
        (read.project_id, read.model.clone(), read.set_at),
        (project, model.clone(), 10)
    );
    assert_eq!(store.value_model(model.id, 2).await.expect("read"), None);
    // A metric has one model; a change is its next version under the same id.
    let mut next = model.clone();
    next.version = 2;
    next.cents_per_unit = 720.0;
    store
        .put_value_model(project, &next, 20)
        .await
        .expect("version 2");
    let mut other_id = fixtures::value_model(2);
    other_id.metric = model.metric.clone();
    assert!(matches!(
        store.put_value_model(project, &other_id, 21).await,
        Err(JudgeStoreError::Conflict)
    ));
    let mut size = fixtures::value_model(1);
    size.metric = "bundle size".into();
    store
        .put_value_model(project, &size, 30)
        .await
        .expect("another metric");
    let newest = store.value_models(project).await.expect("list");
    assert_eq!(
        newest
            .iter()
            .map(|row| (row.model.metric.as_str(), row.model.version))
            .collect::<Vec<_>>(),
        vec![("bundle size", 1), ("p95", 2)]
    );
    assert_eq!(
        store.value_models(ProjectId::new()).await.expect("list"),
        vec![]
    );

    let opened = WinEvent {
        experiment: sealed.experiment(),
        version: 1,
        trial: first.id,
        kind: WinEventKind::Opened,
        value: None,
        link: Some("https://github.com/acme/app/pull/7".into()),
        at: 100,
    };
    let regressed = WinEvent {
        kind: WinEventKind::Regressed,
        value: Some(95.0),
        link: None,
        at: 300,
        ..opened.clone()
    };
    let merged = WinEvent {
        kind: WinEventKind::Merged,
        value: None,
        at: 200,
        ..opened.clone()
    };
    for event in [&opened, &merged, &regressed] {
        store.put_win_event(event).await.expect("event");
    }
    assert_eq!(
        store.win_events(sealed.experiment()).await.expect("events"),
        vec![opened.clone(), merged, regressed]
    );
    assert_eq!(
        store
            .win_events(antilog_types::ExperimentId::new())
            .await
            .expect("events"),
        vec![]
    );
    let stray = WinEvent {
        trial: TrialId::new(),
        ..opened
    };
    assert!(matches!(
        store.put_win_event(&stray).await,
        Err(JudgeStoreError::NotFound)
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_judge_store_passes_the_suite() {
        judge_store_suite(
            &FakeJudgeStore::default(),
            ProjectId::new(),
            antilog_types::ExperimentId::new(),
        )
        .await;
    }
}
