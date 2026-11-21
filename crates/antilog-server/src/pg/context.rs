use antilog_core::store::{ContextStore, StoreError};
use antilog_types::{
    ContextMark, ContextPart, ContextUse, OrganizationId, SessionContext, SessionId,
};

use super::{PgStore, corrupt, map};

fn tokens(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

impl ContextStore for PgStore {
    async fn put_context_use(
        &self,
        organization: OrganizationId,
        session: SessionId,
        context: ContextUse,
        at: i64,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_scalar!(
            "select id from sessions where id = $1 and organization_id = $2",
            session.as_uuid(),
            organization.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .ok_or(StoreError::NotFound)?;
        sqlx::query!(
            "insert into session_context (session_id, organization_id, used_tokens, window_tokens, reported_at) values ($1, $2, $3, $4, to_timestamp($5::bigint / 1000.0)) on conflict (session_id) do update set used_tokens = excluded.used_tokens, window_tokens = coalesce(excluded.window_tokens, session_context.window_tokens), reported_at = excluded.reported_at",
            session.as_uuid(),
            organization.as_uuid(),
            tokens(context.used),
            context.window.filter(|window| *window > 0).map(tokens),
            at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        tx.commit().await.map_err(map)
    }

    async fn put_context_parts(
        &self,
        organization: OrganizationId,
        session: SessionId,
        parts: &[ContextPart],
        at: i64,
    ) -> Result<(), StoreError> {
        let text = serde_json::to_string(parts)
            .map_err(|error| StoreError::Unavailable(Box::new(error)))?;
        let mut tx = self.scoped(organization).await?;
        sqlx::query_scalar!(
            "select id from sessions where id = $1 and organization_id = $2",
            session.as_uuid(),
            organization.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .ok_or(StoreError::NotFound)?;
        sqlx::query!(
            "insert into session_context (session_id, organization_id, parts, reported_at) values ($1, $2, $3, to_timestamp($4::bigint / 1000.0)) on conflict (session_id) do update set parts = excluded.parts, reported_at = excluded.reported_at",
            session.as_uuid(),
            organization.as_uuid(),
            text,
            at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        tx.commit().await.map_err(map)
    }

    async fn context(
        &self,
        organization: OrganizationId,
        session: SessionId,
    ) -> Result<SessionContext, StoreError> {
        let mut tx = self.scoped(organization).await?;
        let row = sqlx::query!(
            "select used_tokens, window_tokens, parts, (extract(epoch from reported_at) * 1000)::bigint as \"reported_at!\" from session_context where session_id = $1 and organization_id = $2",
            session.as_uuid(),
            organization.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?;
        let Some(row) = row else {
            return Ok(SessionContext::default());
        };
        Ok(SessionContext {
            used: row.used_tokens.and_then(|n| u64::try_from(n).ok()),
            window: row.window_tokens.and_then(|n| u64::try_from(n).ok()),
            parts: serde_json::from_str(&row.parts).map_err(|_| corrupt("parts", "not json"))?,
            reported_at: Some(row.reported_at),
        })
    }

    async fn context_marks(
        &self,
        organization: OrganizationId,
        since: i64,
        limit: u32,
    ) -> Result<Vec<ContextMark>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        let rows = sqlx::query!(
            "select session_id, (extract(epoch from reported_at) * 1000)::bigint as \"reported_at!\" from session_context where organization_id = $1 and reported_at >= to_timestamp($2::bigint / 1000.0) order by reported_at, session_id limit $3",
            organization.as_uuid(),
            since,
            i64::from(limit),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?;
        Ok(rows
            .into_iter()
            .map(|row| ContextMark {
                session: SessionId::from_uuid(row.session_id),
                reported_at: row.reported_at,
            })
            .collect())
    }
}
