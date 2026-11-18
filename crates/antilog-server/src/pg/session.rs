use antilog_core::store::{Invariant, NewEvent, SessionChange, SessionStore, StoreError};
use antilog_types::{
    Agent, AuditEntry, Cleanup, CostTier, EvaluationKind, EvaluationTemplate, EventKind,
    Experiment, ExperimentId, ExperimentStatus, MemberId, OrganizationId, ProjectId, Session,
    SessionEvent, SessionId, SessionStatus,
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

    async fn append_event(&self, event: &NewEvent) -> Result<(), StoreError> {
        let mut tx = self.scoped(event.organization).await?;
        sqlx::query!(
            "insert into session_events (organization_id, session_id, key, kind, member_id, text, at) values ($1, $2, $3, $4, $5, $6, to_timestamp($7::bigint / 1000.0)) on conflict (session_id, key) do nothing",
            event.organization.as_uuid(),
            event.session.as_uuid(),
            event.key,
            event.kind.as_str(),
            event.member.map(|member| member.as_uuid()),
            event.text,
            event.at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        tx.commit().await.map_err(map)
    }

    async fn session_events(
        &self,
        organization: OrganizationId,
        session: SessionId,
        after: i64,
        limit: u32,
    ) -> Result<Vec<SessionEvent>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query!(
            "select seq, kind, member_id, text, (extract(epoch from at) * 1000)::bigint as \"at!\" from session_events where organization_id = $1 and session_id = $2 and seq > $3 order by seq limit $4",
            organization.as_uuid(),
            session.as_uuid(),
            after,
            i64::from(limit),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?
        .into_iter()
        .map(|row| {
            Ok(SessionEvent {
                seq: row.seq,
                session_id: session,
                kind: EventKind::parse(&row.kind).ok_or_else(|| corrupt("event kind", &row.kind))?,
                member_id: row.member_id.map(MemberId::from_uuid),
                text: row.text,
                at: row.at,
            })
        })
        .collect()
    }

    async fn event_marks(
        &self,
        organization: OrganizationId,
        after: i64,
        limit: u32,
    ) -> Result<Vec<(i64, SessionId)>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        Ok(sqlx::query!(
            "select seq, session_id from session_events where organization_id = $1 and seq > $2 order by seq limit $3",
            organization.as_uuid(),
            after,
            i64::from(limit),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?
        .into_iter()
        .map(|row| (row.seq, SessionId::from_uuid(row.session_id)))
        .collect())
    }

    async fn latest_event_seq(&self, organization: OrganizationId) -> Result<i64, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_scalar!(
            "select coalesce(max(seq), 0) as \"seq!\" from session_events where organization_id = $1",
            organization.as_uuid(),
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(map)
    }

    async fn sessions_changed(
        &self,
        organization: OrganizationId,
        since: i64,
        limit: u32,
    ) -> Result<Vec<Session>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_as!(
            SessionRow,
            "select id, organization_id, experiment_id, parent_session_id, agent, platform, platform_handle, status, note, cleanup, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", (extract(epoch from updated_at) * 1000)::bigint as \"updated_at!\" from sessions where organization_id = $1 and updated_at >= to_timestamp($2::bigint / 1000.0) order by updated_at, id limit $3",
            organization.as_uuid(),
            since,
            i64::from(limit),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?
        .into_iter()
        .map(Session::try_from)
        .collect()
    }

    async fn put_agent(
        &self,
        organization: OrganizationId,
        agent: &Agent,
        at: i64,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        if agent.is_default && !agent.enabled {
            return Err(Invariant::IllegalTransition.into());
        }
        let mut tx = self.scoped(organization).await?;
        if agent.is_default {
            sqlx::query!(
                "update agents set is_default = false where organization_id = $1 and id <> $2",
                organization.as_uuid(),
                agent.id,
            )
            .execute(&mut *tx)
            .await
            .map_err(map)?;
        }
        sqlx::query!(
            "insert into agents (organization_id, id, name, platform, enabled, is_default, cost_tier, created_at) values ($1, $2, $3, $4, $5, $6, $7, to_timestamp($8::bigint / 1000.0)) on conflict (organization_id, id) do update set name = excluded.name, platform = excluded.platform, enabled = excluded.enabled, is_default = excluded.is_default, cost_tier = excluded.cost_tier",
            organization.as_uuid(),
            agent.id,
            agent.name,
            agent.platform,
            agent.enabled,
            agent.is_default,
            agent.cost_tier.as_str(),
            at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn agents(&self, organization: OrganizationId) -> Result<Vec<Agent>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query!(
            "select id, name, platform, enabled, is_default, cost_tier from agents where organization_id = $1 order by id",
            organization.as_uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?
        .into_iter()
        .map(|row| {
            Ok(Agent {
                id: row.id,
                name: row.name,
                platform: row.platform,
                enabled: row.enabled,
                is_default: row.is_default,
                cost_tier: CostTier::parse(&row.cost_tier)
                    .ok_or_else(|| corrupt("cost tier", &row.cost_tier))?,
                built_in: false,
            })
        })
        .collect()
    }

    async fn add_evaluation(
        &self,
        template: &EvaluationTemplate,
        organization: OrganizationId,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let paths = serde_json::to_string(&template.protected_paths)
            .map_err(|error| StoreError::Unavailable(Box::new(error)))?;
        let mut tx = self.scoped(organization).await?;
        sqlx::query!(
            "insert into evaluation_templates (organization_id, id, name, summary, kind, metric, lower_is_better, command, protected_paths, created_by, created_at) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, to_timestamp($11::bigint / 1000.0))",
            organization.as_uuid(),
            template.id,
            template.name,
            template.summary,
            template.kind.as_str(),
            template.metric,
            template.lower_is_better,
            template.command,
            paths,
            template.created_by.as_uuid(),
            template.created_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn evaluations(
        &self,
        organization: OrganizationId,
    ) -> Result<Vec<EvaluationTemplate>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query!(
            "select id, name, summary, kind, metric, lower_is_better, command, protected_paths, created_by, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\" from evaluation_templates where organization_id = $1 order by created_at, id",
            organization.as_uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?
        .into_iter()
        .map(|row| {
            Ok(EvaluationTemplate {
                id: row.id,
                name: row.name,
                summary: row.summary,
                kind: EvaluationKind::parse(&row.kind)
                    .ok_or_else(|| corrupt("evaluation kind", &row.kind))?,
                metric: row.metric,
                lower_is_better: row.lower_is_better,
                command: row.command,
                protected_paths: serde_json::from_str(&row.protected_paths)
                    .map_err(|_| corrupt("protected paths", "not a list"))?,
                created_by: MemberId::from_uuid(row.created_by),
                created_at: row.created_at,
            })
        })
        .collect()
    }
}
