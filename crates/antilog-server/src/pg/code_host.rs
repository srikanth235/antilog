use antilog_core::store::{CodeHostStore, PullRequestRow, StoreError};
use antilog_types::OrganizationId;
use antilog_types::judge::{ExperimentId, PullRequestState, TrialId};
use uuid::Uuid;

use super::{PgStore, corrupt, map};

struct Row {
    organization_id: Uuid,
    repo: String,
    number: i64,
    experiment_id: Uuid,
    version: i32,
    trial_id: Uuid,
    url: String,
    state: String,
}

impl TryFrom<Row> for PullRequestRow {
    type Error = StoreError;

    fn try_from(row: Row) -> Result<Self, StoreError> {
        Ok(Self {
            organization: OrganizationId::from_uuid(row.organization_id),
            repo: row.repo,
            number: u64::try_from(row.number).map_err(|_| corrupt("pull request number", ""))?,
            experiment: ExperimentId::from_uuid(row.experiment_id),
            version: u32::try_from(row.version).map_err(|_| corrupt("version", ""))?,
            trial: TrialId::from_uuid(row.trial_id),
            url: row.url,
            state: PullRequestState::parse(&row.state)
                .ok_or_else(|| corrupt("pull request state", &row.state))?,
        })
    }
}

fn number(number: u64) -> Result<i64, StoreError> {
    i64::try_from(number).map_err(|_| StoreError::NotFound)
}

impl CodeHostStore for PgStore {
    async fn put_pull_requests(
        &self,
        organization: OrganizationId,
        rows: &[PullRequestRow],
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        for row in rows {
            if row.organization != organization {
                return Err(StoreError::Conflict);
            }
            sqlx::query!(
                "insert into pull_requests (organization_id, repo, number, experiment_id, version, trial_id, url, state) values ($1, $2, $3, $4, $5, $6, $7, $8)",
                organization.as_uuid(),
                row.repo,
                number(row.number)?,
                row.experiment.as_uuid(),
                i32::try_from(row.version).map_err(|_| StoreError::Conflict)?,
                row.trial.as_uuid(),
                row.url,
                row.state.as_str(),
            )
            .execute(&mut *tx)
            .await
            .map_err(map)?;
        }
        tx.commit().await.map_err(map)
    }

    async fn pull_request(
        &self,
        repo: &str,
        wanted: u64,
    ) -> Result<Option<PullRequestRow>, StoreError> {
        let wanted = number(wanted)?;
        let owner = sqlx::query_scalar!("select pull_request_organization($1, $2)", repo, wanted,)
            .fetch_one(&self.pool)
            .await
            .map_err(map)?;
        let Some(owner) = owner.map(OrganizationId::from_uuid) else {
            return Ok(None);
        };
        let mut tx = self.scoped(owner).await?;
        sqlx::query_as!(
            Row,
            "select organization_id, repo, number, experiment_id, version, trial_id, url, state from pull_requests where organization_id = $1 and repo = $2 and number = $3",
            owner.as_uuid(),
            repo,
            wanted,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .map(PullRequestRow::try_from)
        .transpose()
    }

    async fn pull_requests_of(
        &self,
        organization: OrganizationId,
        trial: TrialId,
    ) -> Result<Vec<PullRequestRow>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_as!(
            Row,
            "select organization_id, repo, number, experiment_id, version, trial_id, url, state from pull_requests where organization_id = $1 and trial_id = $2 order by repo, number",
            organization.as_uuid(),
            trial.as_uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?
        .into_iter()
        .map(PullRequestRow::try_from)
        .collect()
    }

    async fn set_pull_request_state(
        &self,
        organization: OrganizationId,
        repo: &str,
        wanted: u64,
        state: PullRequestState,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let changed = sqlx::query!(
            "update pull_requests set state = $4 where organization_id = $1 and repo = $2 and number = $3",
            organization.as_uuid(),
            repo,
            number(wanted)?,
            state.as_str(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?
        .rows_affected();
        if changed == 0 {
            return Err(StoreError::NotFound);
        }
        tx.commit().await.map_err(map)
    }

    async fn delivery_seen(
        &self,
        organization: OrganizationId,
        delivery: &str,
    ) -> Result<bool, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_scalar!(
            "select exists (select 1 from code_host_deliveries where organization_id = $1 and delivery_id = $2) as \"seen!\"",
            organization.as_uuid(),
            delivery,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(map)
    }

    async fn put_delivery(
        &self,
        organization: OrganizationId,
        delivery: &str,
        at: i64,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query!(
            "insert into code_host_deliveries (organization_id, delivery_id, at) values ($1, $2, $3) on conflict do nothing",
            organization.as_uuid(),
            delivery,
            at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        tx.commit().await.map_err(map)
    }
}
