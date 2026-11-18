use antilog_core::store::{RelayStore, StoreError};
use antilog_types::{ApprovalId, OrganizationId, SessionId};

use super::{PgStore, map};

impl RelayStore for PgStore {
    async fn relay_cursor(
        &self,
        organization: OrganizationId,
        session: SessionId,
    ) -> Result<Option<String>, StoreError> {
        sqlx::query_scalar!(
            "select cursor from session_relay where organization_id = $1 and session_id = $2",
            organization.as_uuid(),
            session.as_uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map)
    }

    async fn save_relay_cursor(
        &self,
        organization: OrganizationId,
        session: SessionId,
        cursor: &str,
        at: i64,
    ) -> Result<(), StoreError> {
        sqlx::query!(
            "insert into session_relay (organization_id, session_id, cursor, updated_at) values ($1, $2, $3, to_timestamp($4::bigint / 1000.0)) on conflict (session_id) do update set cursor = excluded.cursor, updated_at = excluded.updated_at",
            organization.as_uuid(),
            session.as_uuid(),
            cursor,
            at,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(map)
    }

    async fn put_proposal(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
        body: &str,
    ) -> Result<(), StoreError> {
        sqlx::query!(
            "insert into seal_proposals (approval_id, organization_id, body) values ($1, $2, $3)",
            approval.as_uuid(),
            organization.as_uuid(),
            body,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(map)
    }

    async fn proposal(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
    ) -> Result<Option<String>, StoreError> {
        sqlx::query_scalar!(
            "select body from seal_proposals where organization_id = $1 and approval_id = $2",
            organization.as_uuid(),
            approval.as_uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map)
    }

    async fn mark_told(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
    ) -> Result<bool, StoreError> {
        sqlx::query!(
            "insert into approvals_told (approval_id, organization_id) values ($1, $2) on conflict do nothing",
            approval.as_uuid(),
            organization.as_uuid(),
        )
        .execute(&self.pool)
        .await
        .map(|done| done.rows_affected() == 1)
        .map_err(map)
    }
}
