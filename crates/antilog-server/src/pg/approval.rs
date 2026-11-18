use antilog_core::store::{ApprovalStore, Invariant, StoreError};
use antilog_types::{
    Approval, ApprovalId, ApprovalStatus, AuditEntry, AuditId, AuditOutcome, CardKind, Decision,
    ExperimentId, MemberId, OrganizationId, Policy, PolicyMode, SessionId,
};
use uuid::Uuid;

use super::{PgStore, audit, corrupt, map};

struct ApprovalRow {
    id: Uuid,
    organization_id: Uuid,
    experiment_id: Uuid,
    session_id: Option<Uuid>,
    card: String,
    summary: String,
    cost_cents: Option<i64>,
    status: String,
    decided_by: Option<Uuid>,
    created_at: i64,
    decided_at: Option<i64>,
}

impl TryFrom<ApprovalRow> for Approval {
    type Error = StoreError;

    fn try_from(row: ApprovalRow) -> Result<Self, StoreError> {
        Ok(Approval {
            id: ApprovalId::from_uuid(row.id),
            organization_id: OrganizationId::from_uuid(row.organization_id),
            experiment_id: ExperimentId::from_uuid(row.experiment_id),
            session_id: row.session_id.map(SessionId::from_uuid),
            card: CardKind::parse(&row.card).ok_or_else(|| corrupt("card kind", &row.card))?,
            summary: row.summary,
            cost_cents: row.cost_cents,
            status: ApprovalStatus::parse(&row.status)
                .ok_or_else(|| corrupt("approval status", &row.status))?,
            decided_by: row.decided_by.map(MemberId::from_uuid),
            created_at: row.created_at,
            decided_at: row.decided_at,
        })
    }
}

impl ApprovalStore for PgStore {
    async fn set_policy(
        &self,
        organization: OrganizationId,
        policy: &Policy,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await.map_err(map)?;
        sqlx::query!(
            "insert into policies (organization_id, card, mode, limit_cents) values ($1, $2, $3, $4) on conflict (organization_id, card) do update set mode = excluded.mode, limit_cents = excluded.limit_cents",
            organization.as_uuid(),
            policy.card.as_str(),
            policy.mode.as_str(),
            policy.limit_cents,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn policies(&self, organization: OrganizationId) -> Result<Vec<Policy>, StoreError> {
        sqlx::query!(
            "select card, mode, limit_cents from policies where organization_id = $1 order by card",
            organization.as_uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map)?
        .into_iter()
        .map(|row| {
            Ok(Policy {
                card: CardKind::parse(&row.card).ok_or_else(|| corrupt("card kind", &row.card))?,
                mode: PolicyMode::parse(&row.mode)
                    .ok_or_else(|| corrupt("policy mode", &row.mode))?,
                limit_cents: row.limit_cents,
            })
        })
        .collect()
    }

    async fn request_approval(
        &self,
        approval: &Approval,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        if !matches!(
            approval.status,
            ApprovalStatus::Pending | ApprovalStatus::ApprovedByPolicy
        ) || approval.decided_by.is_some()
        {
            return Err(Invariant::IllegalTransition.into());
        }
        let mut tx = self.pool.begin().await.map_err(map)?;
        sqlx::query!(
            "insert into approvals (id, organization_id, experiment_id, session_id, card, summary, cost_cents, status, decided_by, created_at, decided_at) values ($1, $2, $3, $4, $5, $6, $7, $8, null, to_timestamp($9::bigint / 1000.0), to_timestamp($10::bigint / 1000.0))",
            approval.id.as_uuid(),
            approval.organization_id.as_uuid(),
            approval.experiment_id.as_uuid(),
            approval.session_id.map(|session| session.as_uuid()),
            approval.card.as_str(),
            approval.summary,
            approval.cost_cents,
            approval.status.as_str(),
            approval.created_at,
            approval.decided_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn decide_approval(
        &self,
        organization: OrganizationId,
        id: ApprovalId,
        decision: Decision,
        by: MemberId,
        at: i64,
        entry: &AuditEntry,
    ) -> Result<Approval, StoreError> {
        let status = match decision {
            Decision::Approve => ApprovalStatus::Approved,
            Decision::Decline => ApprovalStatus::Declined,
        };
        let mut tx = self.pool.begin().await.map_err(map)?;
        let row = sqlx::query_as!(
            ApprovalRow,
            "update approvals set status = $3, decided_by = $4, decided_at = to_timestamp($5::bigint / 1000.0) where organization_id = $1 and id = $2 returning id, organization_id, experiment_id, session_id, card, summary, cost_cents, status, decided_by, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", (extract(epoch from decided_at) * 1000)::bigint as \"decided_at\"",
            organization.as_uuid(),
            id.as_uuid(),
            status.as_str(),
            by.as_uuid(),
            at,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .ok_or(StoreError::NotFound)?;
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)?;
        row.try_into()
    }

    async fn approval(
        &self,
        organization: OrganizationId,
        id: ApprovalId,
    ) -> Result<Option<Approval>, StoreError> {
        sqlx::query_as!(
            ApprovalRow,
            "select id, organization_id, experiment_id, session_id, card, summary, cost_cents, status, decided_by, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", (extract(epoch from decided_at) * 1000)::bigint as \"decided_at\" from approvals where organization_id = $1 and id = $2",
            organization.as_uuid(),
            id.as_uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map)?
        .map(Approval::try_from)
        .transpose()
    }

    async fn approvals(
        &self,
        organization: OrganizationId,
        experiment: ExperimentId,
    ) -> Result<Vec<Approval>, StoreError> {
        sqlx::query_as!(
            ApprovalRow,
            "select id, organization_id, experiment_id, session_id, card, summary, cost_cents, status, decided_by, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", (extract(epoch from decided_at) * 1000)::bigint as \"decided_at\" from approvals where organization_id = $1 and experiment_id = $2 order by created_at, id",
            organization.as_uuid(),
            experiment.as_uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map)?
        .into_iter()
        .map(Approval::try_from)
        .collect()
    }

    async fn append_audit(&self, entry: &AuditEntry) -> Result<(), StoreError> {
        let mut connection = self.pool.acquire().await.map_err(map)?;
        audit(&mut connection, entry).await
    }

    async fn audit_log(
        &self,
        organization: OrganizationId,
        before: Option<(i64, AuditId)>,
        limit: u32,
    ) -> Result<Vec<AuditEntry>, StoreError> {
        sqlx::query!(
            "select id, actor_id, action, target, outcome, (extract(epoch from at) * 1000)::bigint as \"at!\" from audit_log where organization_id = $1 and ($2::bigint is null or (at, id) < (to_timestamp($2::bigint / 1000.0), $3::uuid)) order by at desc, id desc limit $4",
            organization.as_uuid(),
            before.map(|(at, _)| at),
            before.map(|(_, id)| id.as_uuid()),
            i64::from(limit),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map)?
        .into_iter()
        .map(|row| {
            Ok(AuditEntry {
                id: AuditId::from_uuid(row.id),
                organization_id: organization,
                actor: row.actor_id.map(MemberId::from_uuid),
                action: row.action,
                target: row.target,
                outcome: AuditOutcome::parse(&row.outcome)
                    .ok_or_else(|| corrupt("audit outcome", &row.outcome))?,
                at: row.at,
            })
        })
        .collect()
    }
}
