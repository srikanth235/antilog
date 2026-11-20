use antilog_core::store::{CardStore, StoreError};
use antilog_types::{ApprovalId, OrganizationId};

use super::{PgStore, map};

impl CardStore for PgStore {
    async fn put_proposal(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
        body: &str,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query!(
            "insert into seal_proposals (approval_id, organization_id, body) values ($1, $2, $3)",
            approval.as_uuid(),
            organization.as_uuid(),
            body,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        tx.commit().await.map_err(map)
    }

    async fn proposal(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
    ) -> Result<Option<String>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_scalar!(
            "select body from seal_proposals where organization_id = $1 and approval_id = $2",
            organization.as_uuid(),
            approval.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)
    }

    async fn mark_applied(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
    ) -> Result<bool, StoreError> {
        let mut tx = self.scoped(organization).await?;
        let told = sqlx::query!(
            "insert into approvals_told (approval_id, organization_id) values ($1, $2) on conflict do nothing",
            approval.as_uuid(),
            organization.as_uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?
        .rows_affected();
        tx.commit().await.map_err(map)?;
        Ok(told == 1)
    }
}
