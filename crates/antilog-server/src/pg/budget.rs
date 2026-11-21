use antilog_core::store::{BudgetStore, SpendLine, SpendScope, SpendTotal, StoreError};
use antilog_types::{AuditEntry, OrganizationId, ProjectId, SessionId, Spend, SpendKind};

use super::{PgStore, audit, corrupt, map};

impl BudgetStore for PgStore {
    async fn set_organization_limits(
        &self,
        organization: OrganizationId,
        monthly_cap_cents: Option<i64>,
        default_budget_cents: Option<i64>,
        max_running_sessions: Option<u32>,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let changed = sqlx::query!(
            "update organizations set monthly_cap_cents = $2, default_budget_cents = $3, max_running_sessions = coalesce($4, max_running_sessions) where id = $1",
            organization.as_uuid(),
            monthly_cap_cents,
            default_budget_cents,
            max_running_sessions.map(|max| i32::try_from(max).unwrap_or(i32::MAX)),
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

    async fn set_project_limits(
        &self,
        organization: OrganizationId,
        project: ProjectId,
        monthly_cap_cents: Option<i64>,
        default_budget_cents: Option<i64>,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let changed = sqlx::query!(
            "update projects set monthly_cap_cents = $3, default_budget_cents = $4 where organization_id = $1 and id = $2",
            organization.as_uuid(),
            project.as_uuid(),
            monthly_cap_cents,
            default_budget_cents,
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

    async fn set_session_budget(
        &self,
        organization: OrganizationId,
        session: SessionId,
        budget_cap_cents: i64,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let changed = sqlx::query!(
            "update sessions set budget_cap_cents = $3 where organization_id = $1 and id = $2",
            organization.as_uuid(),
            session.as_uuid(),
            budget_cap_cents,
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

    async fn record_spend(&self, spend: &Spend) -> Result<(), StoreError> {
        let mut tx = self.scoped(spend.organization_id).await?;
        sqlx::query!(
            "insert into spend (id, organization_id, session_id, kind, amount_cents, tokens, occurred_at) values ($1, $2, $3, $4, $5, $6, to_timestamp($7::bigint / 1000.0))",
            spend.id.as_uuid(),
            spend.organization_id.as_uuid(),
            spend.session_id.as_uuid(),
            spend.kind.as_str(),
            spend.amount_cents,
            spend.tokens,
            spend.occurred_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        tx.commit().await.map_err(map)
    }

    async fn spend_lines(
        &self,
        organization: OrganizationId,
        session: SessionId,
    ) -> Result<Vec<SpendLine>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query!(
            "select kind, coalesce(sum(amount_cents), 0)::bigint as \"cents!\", coalesce(sum(tokens), 0)::bigint as \"tokens!\" from spend where organization_id = $1 and session_id = $2 group by kind order by kind",
            organization.as_uuid(),
            session.as_uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?
        .into_iter()
        .map(|row| {
            Ok(SpendLine {
                kind: SpendKind::parse(&row.kind).ok_or_else(|| corrupt("spend kind", &row.kind))?,
                cents: row.cents,
                tokens: row.tokens,
            })
        })
        .collect()
    }

    async fn spend(
        &self,
        organization: OrganizationId,
        scope: SpendScope,
        from: i64,
        to: i64,
    ) -> Result<SpendTotal, StoreError> {
        let mut tx = self.scoped(organization).await?;
        let (cents, tokens) = match scope {
            SpendScope::Organization => {
                let row = sqlx::query!(
                    "select coalesce(sum(amount_cents), 0)::bigint as \"cents!\", coalesce(sum(tokens), 0)::bigint as \"tokens!\" from spend where organization_id = $1 and occurred_at >= to_timestamp($2::bigint / 1000.0) and occurred_at < to_timestamp($3::bigint / 1000.0)",
                    organization.as_uuid(),
                    from,
                    to,
                )
                .fetch_one(&mut *tx)
                .await
                .map_err(map)?;
                (row.cents, row.tokens)
            }
            SpendScope::Session(session) => {
                let row = sqlx::query!(
                    "select coalesce(sum(amount_cents), 0)::bigint as \"cents!\", coalesce(sum(tokens), 0)::bigint as \"tokens!\" from spend where organization_id = $1 and session_id = $2 and occurred_at >= to_timestamp($3::bigint / 1000.0) and occurred_at < to_timestamp($4::bigint / 1000.0)",
                    organization.as_uuid(),
                    session.as_uuid(),
                    from,
                    to,
                )
                .fetch_one(&mut *tx)
                .await
                .map_err(map)?;
                (row.cents, row.tokens)
            }
            SpendScope::Project(project) => {
                let row = sqlx::query!(
                    "select coalesce(sum(s.amount_cents), 0)::bigint as \"cents!\", coalesce(sum(s.tokens), 0)::bigint as \"tokens!\" from spend s join sessions n on n.organization_id = s.organization_id and n.id = s.session_id where s.organization_id = $1 and n.project_id = $2 and s.occurred_at >= to_timestamp($3::bigint / 1000.0) and s.occurred_at < to_timestamp($4::bigint / 1000.0)",
                    organization.as_uuid(),
                    project.as_uuid(),
                    from,
                    to,
                )
                .fetch_one(&mut *tx)
                .await
                .map_err(map)?;
                (row.cents, row.tokens)
            }
        };
        Ok(SpendTotal { cents, tokens })
    }
}
