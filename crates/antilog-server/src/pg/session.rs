use antilog_core::store::{Invariant, NewEvent, SessionChange, SessionStore, StoreError};
use antilog_types::{
    Agent, AuditEntry, CostTier, EventKind, MemberId, OrganizationId, ProjectId, Session,
    SessionEvent, SessionId, SessionStatus, ToolRecord, ToolStatus,
};
use uuid::Uuid;

use super::{PgStore, audit, corrupt, map};

struct SessionRow {
    id: Uuid,
    organization_id: Uuid,
    project_id: Uuid,
    title: String,
    created_by: Uuid,
    agent: String,
    model: String,
    effort: Option<String>,
    repos: Vec<String>,
    environment: Option<String>,
    environment_version: Option<i32>,
    budget_cap_cents: Option<i64>,
    follow_policy: bool,
    status: String,
    note: Option<String>,
    created_at: i64,
    updated_at: i64,
}

impl TryFrom<SessionRow> for Session {
    type Error = StoreError;

    fn try_from(row: SessionRow) -> Result<Self, StoreError> {
        Ok(Session {
            id: SessionId::from_uuid(row.id),
            organization_id: OrganizationId::from_uuid(row.organization_id),
            project_id: ProjectId::from_uuid(row.project_id),
            title: row.title,
            created_by: MemberId::from_uuid(row.created_by),
            agent: row.agent,
            model: row.model,
            effort: row.effort,
            repos: row.repos,
            environment: row.environment,
            environment_version: row
                .environment_version
                .map(|version| u32::try_from(version).unwrap_or(0)),
            budget_cap_cents: row.budget_cap_cents,
            follow_policy: row.follow_policy,
            status: SessionStatus::parse(&row.status)
                .ok_or_else(|| corrupt("session status", &row.status))?,
            note: row.note,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

impl SessionStore for PgStore {
    async fn create_session(
        &self,
        session: &Session,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(session.organization_id).await?;
        sqlx::query!(
            "insert into sessions (id, organization_id, project_id, title, created_by, agent, model, effort, repos, environment, environment_version, budget_cap_cents, follow_policy, status, note, created_at, updated_at) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, to_timestamp($16::bigint / 1000.0), to_timestamp($17::bigint / 1000.0))",
            session.id.as_uuid(),
            session.organization_id.as_uuid(),
            session.project_id.as_uuid(),
            session.title,
            session.created_by.as_uuid(),
            session.agent,
            session.model,
            session.effort,
            &session.repos,
            session.environment,
            session.environment_version.map(|version| i32::try_from(version).unwrap_or(i32::MAX)),
            session.budget_cap_cents,
            session.follow_policy,
            session.status.as_str(),
            session.note,
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
            "select id, organization_id, project_id, title, created_by, agent, model, effort, repos, environment, environment_version, budget_cap_cents, follow_policy, status, note, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", (extract(epoch from updated_at) * 1000)::bigint as \"updated_at!\" from sessions where organization_id = $1 and id = $2",
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
        project: Option<ProjectId>,
        creator: Option<MemberId>,
    ) -> Result<Vec<Session>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_as!(
            SessionRow,
            "select id, organization_id, project_id, title, created_by, agent, model, effort, repos, environment, environment_version, budget_cap_cents, follow_policy, status, note, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", (extract(epoch from updated_at) * 1000)::bigint as \"updated_at!\" from sessions where organization_id = $1 and ($2::uuid is null or project_id = $2) and ($3::uuid is null or created_by = $3) order by created_at desc, id desc",
            organization.as_uuid(),
            project.map(|project| project.as_uuid()),
            creator.map(|member| member.as_uuid()),
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
            "update sessions set status = coalesce($3, status), note = coalesce($4, note), updated_at = to_timestamp($5::bigint / 1000.0) where organization_id = $1 and id = $2 returning id, organization_id, project_id, title, created_by, agent, model, effort, repos, environment, environment_version, budget_cap_cents, follow_policy, status, note, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", (extract(epoch from updated_at) * 1000)::bigint as \"updated_at!\"",
            organization.as_uuid(),
            id.as_uuid(),
            change.status.map(SessionStatus::as_str),
            change.note,
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

    async fn set_session_environment(
        &self,
        organization: OrganizationId,
        session: SessionId,
        environment: Option<&str>,
        version: Option<u32>,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let version = version.map(|version| i32::try_from(version).unwrap_or(i32::MAX));
        let changed = sqlx::query!(
            "update sessions set environment = $3, environment_version = $4 where organization_id = $1 and id = $2",
            organization.as_uuid(),
            session.as_uuid(),
            environment,
            version,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?
        .rows_affected();
        if changed == 0 {
            return Err(StoreError::NotFound);
        }
        // The machine's seed names the environment a claim and the proxy read.
        let machine = sqlx::query!(
            "select seed from machines where session_id = $1 and organization_id = $2 for update",
            session.as_uuid(),
            organization.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?;
        if let Some(machine) = machine {
            let mut seed: antilog_types::Seed = serde_json::from_str(&machine.seed)
                .map_err(|error| StoreError::Unavailable(Box::new(error)))?;
            seed.environment = environment.map(str::to_owned);
            seed.environment_version = version.map(|version| u32::try_from(version).unwrap_or(0));
            sqlx::query!(
                "update machines set seed = $2 where session_id = $1",
                session.as_uuid(),
                serde_json::to_string(&seed)
                    .map_err(|error| StoreError::Unavailable(Box::new(error)))?,
            )
            .execute(&mut *tx)
            .await
            .map_err(map)?;
        }
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn set_session_model(
        &self,
        organization: OrganizationId,
        session: SessionId,
        model: &str,
        effort: Option<&str>,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let changed = sqlx::query!(
            "update sessions set model = $3, effort = $4 where organization_id = $1 and id = $2",
            organization.as_uuid(),
            session.as_uuid(),
            model,
            effort,
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

    async fn append_event(&self, event: &NewEvent) -> Result<(), StoreError> {
        let mut tx = self.scoped(event.organization).await?;
        sqlx::query!(
            "insert into session_events (organization_id, session_id, key, kind, member_id, text, at, tool_label, tool_status, tool_input, tool_output) values ($1, $2, $3, $4, $5, $6, to_timestamp($7::bigint / 1000.0), $8, $9, $10, $11) on conflict (session_id, key) do nothing",
            event.organization.as_uuid(),
            event.session.as_uuid(),
            event.key,
            event.kind.as_str(),
            event.member.map(|member| member.as_uuid()),
            event.text,
            event.at,
            event.tool.as_ref().map(|tool| tool.label.as_str()),
            event.tool.as_ref().map(|tool| tool.status.as_str()),
            event.tool.as_ref().and_then(|tool| tool.input.as_deref()),
            event.tool.as_ref().and_then(|tool| tool.output.as_deref()),
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
            "select seq, kind, member_id, text, tool_label, tool_status, tool_input, tool_output, (extract(epoch from at) * 1000)::bigint as \"at!\" from session_events where organization_id = $1 and session_id = $2 and seq > $3 order by seq limit $4",
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
                tool: match (row.tool_label, row.tool_status) {
                    (Some(label), Some(status)) => Some(ToolRecord {
                        label,
                        status: ToolStatus::parse(&status)
                            .ok_or_else(|| corrupt("tool status", &status))?,
                        input: row.tool_input,
                        output: row.tool_output,
                    }),
                    _ => None,
                },
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
            "select id, organization_id, project_id, title, created_by, agent, model, effort, repos, environment, environment_version, budget_cap_cents, follow_policy, status, note, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", (extract(epoch from updated_at) * 1000)::bigint as \"updated_at!\" from sessions where organization_id = $1 and updated_at >= to_timestamp($2::bigint / 1000.0) order by updated_at, id limit $3",
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
            "insert into agents (organization_id, id, name, enabled, is_default, cost_tier, created_at) values ($1, $2, $3, $4, $5, $6, to_timestamp($7::bigint / 1000.0)) on conflict (organization_id, id) do update set name = excluded.name, enabled = excluded.enabled, is_default = excluded.is_default, cost_tier = excluded.cost_tier",
            organization.as_uuid(),
            agent.id,
            agent.name,
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
            "select id, name, enabled, is_default, cost_tier from agents where organization_id = $1 order by id",
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
                enabled: row.enabled,
                is_default: row.is_default,
                cost_tier: CostTier::parse(&row.cost_tier)
                    .ok_or_else(|| corrupt("cost tier", &row.cost_tier))?,
                built_in: false,
            })
        })
        .collect()
    }
}
