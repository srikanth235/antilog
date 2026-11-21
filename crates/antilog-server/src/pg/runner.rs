use antilog_core::store::{RUNNER_LOCK_LAPSE_MS, RunnerStore, StoreError};
use antilog_types::{
    Command, Envelope, Machine, MachineState, OrganizationId, Seed, SessionId,
};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::{PgStore, corrupt, map};

struct Row {
    session_id: Uuid,
    organization_id: Uuid,
    agent: String,
    state: String,
    holder: Option<String>,
    seed: String,
    resume: bool,
}

impl TryFrom<Row> for Machine {
    type Error = StoreError;

    fn try_from(row: Row) -> Result<Self, StoreError> {
        Ok(Machine {
            session: SessionId::from_uuid(row.session_id),
            organization: OrganizationId::from_uuid(row.organization_id),
            agent: row.agent,
            state: MachineState::parse(&row.state)
                .ok_or_else(|| corrupt("machine state", &row.state))?,
            holder: row.holder,
            seed: serde_json::from_str::<Seed>(&row.seed)
                .map_err(|_| corrupt("machine seed", &row.seed))?,
            resume: row.resume,
        })
    }
}

impl PgStore {
    /// A transaction for a runner, which serves every organization.
    async fn runner(&self) -> Result<Transaction<'static, Postgres>, StoreError> {
        let mut tx = self.pool.begin().await.map_err(map)?;
        sqlx::query("select set_config('antilog.runner', 'on', true)")
            .execute(&mut *tx)
            .await
            .map_err(map)?;
        Ok(tx)
    }
}

fn json<T: serde::Serialize>(value: &T) -> Result<String, StoreError> {
    serde_json::to_string(value).map_err(|error| StoreError::Unavailable(Box::new(error)))
}

impl RunnerStore for PgStore {
    async fn enqueue_machine(
        &self,
        organization: OrganizationId,
        session: SessionId,
        seed: &Seed,
        at: i64,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let session_row = sqlx::query!(
            "select agent from sessions where organization_id = $1 and id = $2",
            organization.as_uuid(),
            session.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .ok_or(StoreError::NotFound)?;
        sqlx::query!(
            "insert into machines (session_id, organization_id, agent, state, seed, updated_at) values ($1, $2, $3, 'queued', $4, to_timestamp($5::bigint / 1000.0))",
            session.as_uuid(),
            organization.as_uuid(),
            session_row.agent,
            json(seed)?,
            at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        tx.commit().await.map_err(map)
    }

    async fn machine(&self, session: SessionId) -> Result<Option<Machine>, StoreError> {
        let mut tx = self.runner().await?;
        sqlx::query_as!(
            Row,
            "select session_id, organization_id, agent, state, holder, seed, resume from machines where session_id = $1",
            session.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .map(Machine::try_from)
        .transpose()
    }

    async fn claim(
        &self,
        holder: &str,
        limit: u32,
        now: i64,
        until: i64,
    ) -> Result<Vec<Machine>, StoreError> {
        let mut tx = self.runner().await?;
        let claimed = sqlx::query!(
            "select session_id as \"session_id!\", organization_id as \"organization_id!\", agent as \"agent!\", state as \"state!\", holder, seed as \"seed!\", resume as \"resume!\" from claim_machines($1, $2, to_timestamp($3::bigint / 1000.0), to_timestamp($4::bigint / 1000.0), $5)",
            holder,
            i32::try_from(limit).unwrap_or(i32::MAX),
            now,
            until,
            RUNNER_LOCK_LAPSE_MS,
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?;
        let machines = claimed
            .into_iter()
            .map(|row| {
                Machine::try_from(Row {
                    session_id: row.session_id,
                    organization_id: row.organization_id,
                    agent: row.agent,
                    state: row.state,
                    holder: row.holder,
                    seed: row.seed,
                    resume: row.resume,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        tx.commit().await.map_err(map)?;
        Ok(machines)
    }

    async fn unlock_runner(&self, holder: &str) -> Result<(), StoreError> {
        let mut tx = self.runner().await?;
        sqlx::query!(
            "delete from runner_locks where holder = $1 and not exists (select 1 from machines where holder = $1 and state in ('claimed', 'running', 'idle'))",
            holder,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        tx.commit().await.map_err(map)?;
        Ok(())
    }

    async fn mark_overdue(
        &self,
        holder: &str,
        claimed_before: i64,
    ) -> Result<Vec<SessionId>, StoreError> {
        let mut tx = self.runner().await?;
        let overdue = sqlx::query_scalar!(
            "update machines set expiring = true where holder = $1 and state in ('claimed', 'running', 'idle') and not expiring and claimed_at < to_timestamp($2::bigint / 1000.0) returning session_id",
            holder,
            claimed_before,
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?;
        tx.commit().await.map_err(map)?;
        Ok(overdue.into_iter().map(SessionId::from_uuid).collect())
    }

    async fn renew(
        &self,
        holder: &str,
        now: i64,
        until: i64,
    ) -> Result<Vec<SessionId>, StoreError> {
        let mut tx = self.runner().await?;
        let held = sqlx::query_scalar!(
            "update machines set lease_until = to_timestamp($3::bigint / 1000.0), updated_at = to_timestamp($2::bigint / 1000.0) where holder = $1 and state in ('claimed', 'running', 'idle') returning session_id",
            holder,
            now,
            until,
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?;
        tx.commit().await.map_err(map)?;
        Ok(held.into_iter().map(SessionId::from_uuid).collect())
    }

    async fn set_machine_state(
        &self,
        session: SessionId,
        holder: &str,
        state: MachineState,
        now: i64,
    ) -> Result<bool, StoreError> {
        let mut tx = self.runner().await?;
        let held = state.is_held();
        let changed = sqlx::query!(
            "update machines set state = $3, holder = case when $4 then holder else null end, lease_until = case when $4 then lease_until else null end, updated_at = to_timestamp($5::bigint / 1000.0) where session_id = $1 and holder = $2 and state in ('claimed', 'running', 'idle')",
            session.as_uuid(),
            holder,
            state.as_str(),
            held,
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?
        .rows_affected();
        tx.commit().await.map_err(map)?;
        Ok(changed == 1)
    }

    async fn add_seed_repo(&self, session: SessionId, repo: &str) -> Result<bool, StoreError> {
        let mut tx = self.runner().await?;
        let row = sqlx::query!(
            "select seed from machines where session_id = $1 for update",
            session.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .ok_or(StoreError::NotFound)?;
        let mut seed: Seed = serde_json::from_str(&row.seed)
            .map_err(|error| StoreError::Unavailable(Box::new(error)))?;
        if seed.repos.iter().any(|name| name == repo) {
            return Ok(false);
        }
        seed.repos.push(repo.to_owned());
        sqlx::query!(
            "update machines set seed = $2 where session_id = $1",
            session.as_uuid(),
            json(&seed)?,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        tx.commit().await.map_err(map)?;
        Ok(true)
    }

    async fn wake_machine(&self, session: SessionId, now: i64) -> Result<bool, StoreError> {
        let mut tx = self.runner().await?;
        let changed = sqlx::query!(
            "update machines set state = 'queued', resume = exists (select 1 from machine_exports e where e.session_id = machines.session_id), updated_at = to_timestamp($2::bigint / 1000.0) where session_id = $1 and state = 'released'",
            session.as_uuid(),
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?
        .rows_affected();
        tx.commit().await.map_err(map)?;
        Ok(changed == 1)
    }

    async fn push_command(
        &self,
        organization: OrganizationId,
        session: SessionId,
        command: &Command,
        at: i64,
    ) -> Result<Uuid, StoreError> {
        let mut tx = self.scoped(organization).await?;
        let id = Uuid::now_v7();
        sqlx::query!(
            "insert into machine_commands (id, organization_id, session_id, body, created_at) select $1, $2, $3, $4, to_timestamp($5::bigint / 1000.0) where exists (select 1 from machines where session_id = $3 and organization_id = $2)",
            id,
            organization.as_uuid(),
            session.as_uuid(),
            json(command)?,
            at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)
        .and_then(|done| {
            if done.rows_affected() == 1 {
                Ok(())
            } else {
                Err(StoreError::NotFound)
            }
        })?;
        tx.commit().await.map_err(map)?;
        Ok(id)
    }

    async fn take_commands(&self, holder: &str) -> Result<Vec<Envelope>, StoreError> {
        let mut tx = self.runner().await?;
        let rows = sqlx::query!(
            "update machine_commands c set delivered = true from machines m where m.session_id = c.session_id and m.holder = $1 and m.state in ('claimed', 'running', 'idle') and not c.delivered returning c.id, c.seq, c.session_id, c.body",
            holder,
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?;
        tx.commit().await.map_err(map)?;
        let mut rows = rows;
        rows.sort_by_key(|row| row.seq);
        rows.into_iter()
            .map(|row| {
                Ok(Envelope {
                    id: row.id,
                    session: SessionId::from_uuid(row.session_id),
                    command: serde_json::from_str(&row.body)
                        .map_err(|_| corrupt("machine command", &row.body))?,
                })
            })
            .collect()
    }

    async fn finish_commands(&self, holder: &str, ids: &[Uuid]) -> Result<(), StoreError> {
        let mut tx = self.runner().await?;
        sqlx::query!(
            "update machine_commands c set finished = true from machines m where m.session_id = c.session_id and m.holder = $1 and c.id = any($2)",
            holder,
            ids,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        tx.commit().await.map_err(map)
    }




    async fn put_export(
        &self,
        organization: OrganizationId,
        session: SessionId,
        body: &str,
        at: i64,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query!(
            "insert into machine_exports (session_id, organization_id, body, at) values ($1, $2, $3, to_timestamp($4::bigint / 1000.0)) on conflict (session_id) do update set body = excluded.body, at = excluded.at",
            session.as_uuid(),
            organization.as_uuid(),
            body,
            at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        tx.commit().await.map_err(map)
    }

    async fn export(&self, session: SessionId) -> Result<Option<String>, StoreError> {
        let mut tx = self.runner().await?;
        sqlx::query_scalar!(
            "select body from machine_exports where session_id = $1",
            session.as_uuid()
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)
    }
}
