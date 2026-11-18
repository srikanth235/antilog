use antilog_core::store::{SessionChange, SessionStore, StoreError};
use antilog_types::{
    AuditEntry, Cleanup, Experiment, ExperimentId, ExperimentStatus, MemberId, OrganizationId,
    ProjectId, Session, SessionId, SessionStatus,
};
use uuid::Uuid;

use super::{PgStore, audit, corrupt, map};

struct ExperimentRow {
    id: Uuid,
    organization_id: Uuid,
    project_id: Uuid,
    goal: String,
    status: String,
    budget_cap_cents: i64,
    created_by: Uuid,
    created_at: i64,
    follow_policy: bool,
}

impl TryFrom<ExperimentRow> for Experiment {
    type Error = StoreError;

    fn try_from(row: ExperimentRow) -> Result<Self, StoreError> {
        Ok(Experiment {
            id: ExperimentId::from_uuid(row.id),
            organization_id: OrganizationId::from_uuid(row.organization_id),
            project_id: ProjectId::from_uuid(row.project_id),
            goal: row.goal,
            status: ExperimentStatus::parse(&row.status)
                .ok_or_else(|| corrupt("experiment status", &row.status))?,
            budget_cap_cents: row.budget_cap_cents,
            created_by: MemberId::from_uuid(row.created_by),
            created_at: row.created_at,
            follow_policy: row.follow_policy,
        })
    }
}

struct SessionRow {
    id: Uuid,
    organization_id: Uuid,
    experiment_id: Uuid,
    parent_session_id: Option<Uuid>,
    agent: String,
    platform: String,
    platform_handle: Option<String>,
    status: String,
    note: Option<String>,
    cleanup: Option<String>,
    created_at: i64,
    updated_at: i64,
}

impl TryFrom<SessionRow> for Session {
    type Error = StoreError;

    fn try_from(row: SessionRow) -> Result<Self, StoreError> {
        Ok(Session {
            id: SessionId::from_uuid(row.id),
            organization_id: OrganizationId::from_uuid(row.organization_id),
            experiment_id: ExperimentId::from_uuid(row.experiment_id),
            parent: row.parent_session_id.map(SessionId::from_uuid),
            agent: row.agent,
            platform: row.platform,
            platform_handle: row.platform_handle,
            status: SessionStatus::parse(&row.status)
                .ok_or_else(|| corrupt("session status", &row.status))?,
            note: row.note,
            cleanup: row
                .cleanup
                .map(|text| Cleanup::parse(&text).ok_or_else(|| corrupt("clean-up state", &text)))
                .transpose()?,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

impl SessionStore for PgStore {
    async fn create_experiment(
        &self,
        experiment: &Experiment,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(experiment.organization_id).await?;
        sqlx::query!(
            "insert into experiments (id, organization_id, project_id, goal, status, budget_cap_cents, created_by, created_at, follow_policy) values ($1, $2, $3, $4, $5, $6, $7, to_timestamp($8::bigint / 1000.0), $9)",
            experiment.id.as_uuid(),
            experiment.organization_id.as_uuid(),
            experiment.project_id.as_uuid(),
            experiment.goal,
            experiment.status.as_str(),
            experiment.budget_cap_cents,
            experiment.created_by.as_uuid(),
            experiment.created_at,
            experiment.follow_policy,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn experiment(
        &self,
        organization: OrganizationId,
        id: ExperimentId,
    ) -> Result<Option<Experiment>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_as!(
            ExperimentRow,
            "select id, organization_id, project_id, goal, status, budget_cap_cents, created_by, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", follow_policy from experiments where organization_id = $1 and id = $2",
            organization.as_uuid(),
            id.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .map(Experiment::try_from)
        .transpose()
    }

    async fn experiments(
        &self,
        organization: OrganizationId,
        project: ProjectId,
    ) -> Result<Vec<Experiment>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_as!(
            ExperimentRow,
            "select id, organization_id, project_id, goal, status, budget_cap_cents, created_by, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", follow_policy from experiments where organization_id = $1 and project_id = $2 order by created_at, id",
            organization.as_uuid(),
            project.as_uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?
        .into_iter()
        .map(Experiment::try_from)
        .collect()
    }

    async fn set_experiment_status(
        &self,
        organization: OrganizationId,
        id: ExperimentId,
        status: ExperimentStatus,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let changed = sqlx::query!(
            "update experiments set status = $3 where organization_id = $1 and id = $2",
            organization.as_uuid(),
            id.as_uuid(),
            status.as_str(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?
        .rows_affected();
        if changed == 0 {
            return Err(StoreError::NotFound);
        }
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn create_session(
        &self,
        session: &Session,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(session.organization_id).await?;
        sqlx::query!(
            "insert into sessions (id, organization_id, experiment_id, parent_session_id, agent, platform, platform_handle, status, note, cleanup, created_at, updated_at) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, to_timestamp($11::bigint / 1000.0), to_timestamp($12::bigint / 1000.0))",
            session.id.as_uuid(),
            session.organization_id.as_uuid(),
            session.experiment_id.as_uuid(),
            session.parent.map(|parent| parent.as_uuid()),
            session.agent,
            session.platform,
            session.platform_handle,
            session.status.as_str(),
            session.note,
            session.cleanup.map(Cleanup::as_str),
            session.created_at,
            session.updated_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn session(
        &self,
        organization: OrganizationId,
        id: SessionId,
    ) -> Result<Option<Session>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_as!(
            SessionRow,
            "select id, organization_id, experiment_id, parent_session_id, agent, platform, platform_handle, status, note, cleanup, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", (extract(epoch from updated_at) * 1000)::bigint as \"updated_at!\" from sessions where organization_id = $1 and id = $2",
            organization.as_uuid(),
            id.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .map(Session::try_from)
        .transpose()
    }

    async fn find_session(&self, id: SessionId) -> Result<Option<Session>, StoreError> {
        let mut tx = self.found(id.as_uuid()).await?;
        sqlx::query_as!(
            SessionRow,
            "select id, organization_id, experiment_id, parent_session_id, agent, platform, platform_handle, status, note, cleanup, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", (extract(epoch from updated_at) * 1000)::bigint as \"updated_at!\" from sessions where id = $1",
            id.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .map(Session::try_from)
        .transpose()
    }

    async fn sessions(
        &self,
        organization: OrganizationId,
        experiment: ExperimentId,
    ) -> Result<Vec<Session>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_as!(
            SessionRow,
            "select id, organization_id, experiment_id, parent_session_id, agent, platform, platform_handle, status, note, cleanup, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", (extract(epoch from updated_at) * 1000)::bigint as \"updated_at!\" from sessions where organization_id = $1 and experiment_id = $2 order by created_at, id",
            organization.as_uuid(),
            experiment.as_uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?
        .into_iter()
        .map(Session::try_from)
        .collect()
    }

    async fn update_session(
        &self,
        organization: OrganizationId,
        id: SessionId,
        change: &SessionChange,
        at: i64,
        entry: Option<&AuditEntry>,
    ) -> Result<Session, StoreError> {
        let mut tx = self.scoped(organization).await?;
        let row = sqlx::query_as!(
            SessionRow,
            "update sessions set status = coalesce($3, status), note = coalesce($4, note), platform_handle = coalesce($5, platform_handle), cleanup = coalesce($6, cleanup), updated_at = to_timestamp($7::bigint / 1000.0) where organization_id = $1 and id = $2 returning id, organization_id, experiment_id, parent_session_id, agent, platform, platform_handle, status, note, cleanup, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", (extract(epoch from updated_at) * 1000)::bigint as \"updated_at!\"",
            organization.as_uuid(),
            id.as_uuid(),
            change.status.map(SessionStatus::as_str),
            change.note,
            change.platform_handle,
            change.cleanup.map(Cleanup::as_str),
            at,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .ok_or(StoreError::NotFound)?;
        if let Some(entry) = entry {
            audit(&mut tx, entry).await?;
        }
        tx.commit().await.map_err(map)?;
        row.try_into()
    }
}
