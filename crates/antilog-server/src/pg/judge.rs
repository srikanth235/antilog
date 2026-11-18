use antilog_core::store::StoreError;
use antilog_core::store::judge::{JudgeStore, JudgeStoreError};
use antilog_types::OrganizationId;
use antilog_types::judge::{
    ExperimentId, Finished, Receipt, SealedExperiment, Trial, TrialId, TrialRecord, ValueModel,
    ValueModelId,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::{PgStore, map};

/// The judge's tables for one organization: every query runs in a scoped transaction.
#[derive(Clone)]
pub struct PgJudge {
    pub(super) store: PgStore,
    pub(super) organization: OrganizationId,
}

fn fail(error: sqlx::Error) -> JudgeStoreError {
    match map(error) {
        StoreError::Conflict => JudgeStoreError::Conflict,
        StoreError::NotFound => JudgeStoreError::NotFound,
        StoreError::Invariant(rule) => JudgeStoreError::Unavailable(Box::new(rule)),
        StoreError::Unavailable(source) => JudgeStoreError::Unavailable(source),
    }
}

fn text<T: Serialize>(value: &T) -> Result<String, JudgeStoreError> {
    serde_json::to_string(value).map_err(|error| JudgeStoreError::Unavailable(Box::new(error)))
}

fn parse<T: DeserializeOwned>(doc: &str) -> Result<T, JudgeStoreError> {
    serde_json::from_str(doc).map_err(|error| JudgeStoreError::Unavailable(Box::new(error)))
}

fn version(version: u32) -> Result<i32, JudgeStoreError> {
    i32::try_from(version).map_err(|error| JudgeStoreError::Unavailable(Box::new(error)))
}

impl PgJudge {
    async fn tx(&self) -> Result<sqlx::Transaction<'static, sqlx::Postgres>, JudgeStoreError> {
        self.store
            .scoped(self.organization)
            .await
            .map_err(|error| match error {
                StoreError::Unavailable(source) => JudgeStoreError::Unavailable(source),
                other => JudgeStoreError::Unavailable(Box::new(other)),
            })
    }

    fn org(&self) -> uuid::Uuid {
        self.organization.as_uuid()
    }
}

impl PgStore {
    pub fn judge(&self, organization: OrganizationId) -> PgJudge {
        PgJudge {
            store: self.clone(),
            organization,
        }
    }
}

fn record(trial: &str, finished: Option<&str>) -> Result<TrialRecord, JudgeStoreError> {
    Ok(TrialRecord {
        trial: parse(trial)?,
        finished: finished.map(parse).transpose()?,
    })
}

impl JudgeStore for PgJudge {
    async fn put_sealed(&self, sealed: &SealedExperiment) -> Result<(), JudgeStoreError> {
        let mut tx = self.tx().await?;
        sqlx::query!(
            "insert into sealed_experiments (organization_id, experiment_id, version, doc) values ($1, $2, $3, $4)",
            self.org(),
            sealed.experiment().as_uuid(),
            version(sealed.version())?,
            text(sealed)?,
        )
        .execute(&mut *tx)
        .await
        .map_err(fail)?;
        tx.commit().await.map_err(fail)
    }

    async fn sealed(
        &self,
        experiment: ExperimentId,
        number: u32,
    ) -> Result<Option<SealedExperiment>, JudgeStoreError> {
        let mut tx = self.tx().await?;
        sqlx::query_scalar!(
            "select doc from sealed_experiments where organization_id = $1 and experiment_id = $2 and version = $3",
            self.org(),
            experiment.as_uuid(),
            version(number)?,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(fail)?
        .map(|doc| parse(&doc))
        .transpose()
    }

    async fn begin_trial(&self, trial: &Trial) -> Result<(), JudgeStoreError> {
        let mut tx = self.tx().await?;
        sqlx::query!(
            "insert into trials (id, organization_id, experiment_id, version, doc) values ($1, $2, $3, $4, $5)",
            trial.id.as_uuid(),
            self.org(),
            trial.experiment.as_uuid(),
            version(trial.version)?,
            text(trial)?,
        )
        .execute(&mut *tx)
        .await
        .map_err(fail)?;
        tx.commit().await.map_err(fail)
    }

    async fn finish_trial(
        &self,
        trial: TrialId,
        finished: &Finished,
    ) -> Result<(), JudgeStoreError> {
        let mut tx = self.tx().await?;
        let changed = sqlx::query!(
            "update trials set finished = $3 where organization_id = $1 and id = $2 and finished is null",
            self.org(),
            trial.as_uuid(),
            text(finished)?,
        )
        .execute(&mut *tx)
        .await
        .map_err(fail)?
        .rows_affected();
        if changed == 1 {
            return tx.commit().await.map_err(fail);
        }
        let exists = sqlx::query_scalar!(
            "select exists (select 1 from trials where organization_id = $1 and id = $2) as \"exists!\"",
            self.org(),
            trial.as_uuid(),
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(fail)?;
        Err(if exists {
            JudgeStoreError::Conflict
        } else {
            JudgeStoreError::NotFound
        })
    }

    async fn trial(&self, id: TrialId) -> Result<Option<TrialRecord>, JudgeStoreError> {
        let mut tx = self.tx().await?;
        sqlx::query!(
            "select doc, finished from trials where organization_id = $1 and id = $2",
            self.org(),
            id.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(fail)?
        .map(|row| record(&row.doc, row.finished.as_deref()))
        .transpose()
    }

    async fn trials(
        &self,
        experiment: ExperimentId,
        number: u32,
    ) -> Result<Vec<TrialRecord>, JudgeStoreError> {
        let mut tx = self.tx().await?;
        sqlx::query!(
            "select doc, finished from trials where organization_id = $1 and experiment_id = $2 and version = $3 order by seq",
            self.org(),
            experiment.as_uuid(),
            version(number)?,
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(fail)?
        .iter()
        .map(|row| record(&row.doc, row.finished.as_deref()))
        .collect()
    }

    async fn put_receipt(&self, receipt: &Receipt) -> Result<(), JudgeStoreError> {
        let mut tx = self.tx().await?;
        sqlx::query!(
            "insert into receipts (trial_id, organization_id, doc) values ($1, $2, $3)",
            receipt.body().experiment.trial.as_uuid(),
            self.org(),
            text(receipt)?,
        )
        .execute(&mut *tx)
        .await
        .map_err(fail)?;
        tx.commit().await.map_err(fail)
    }

    async fn receipt(&self, trial: TrialId) -> Result<Option<Receipt>, JudgeStoreError> {
        let mut tx = self.tx().await?;
        sqlx::query_scalar!(
            "select doc from receipts where organization_id = $1 and trial_id = $2",
            self.org(),
            trial.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(fail)?
        .map(|doc| parse(&doc))
        .transpose()
    }

    async fn put_value_model(&self, model: &ValueModel) -> Result<(), JudgeStoreError> {
        let mut tx = self.tx().await?;
        sqlx::query!(
            "insert into value_models (id, version, organization_id, doc) values ($1, $2, $3, $4)",
            model.id.as_uuid(),
            version(model.version)?,
            self.org(),
            text(model)?,
        )
        .execute(&mut *tx)
        .await
        .map_err(fail)?;
        tx.commit().await.map_err(fail)
    }

    async fn value_model(
        &self,
        id: ValueModelId,
        number: u32,
    ) -> Result<Option<ValueModel>, JudgeStoreError> {
        let mut tx = self.tx().await?;
        sqlx::query_scalar!(
            "select doc from value_models where organization_id = $1 and id = $2 and version = $3",
            self.org(),
            id.as_uuid(),
            version(number)?,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(fail)?
        .map(|doc| parse(&doc))
        .transpose()
    }
}
