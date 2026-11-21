use antilog_core::sealed::Sealer;
use antilog_core::secrets::Secret;
use antilog_core::store::{GatewayScope, ModelKeyStore, StoreError};
use antilog_types::{
    AuditEntry, ModelKeyStatus, ModelProvider, OrganizationId, SessionId,
};

use super::{PgStore, audit, corrupt, map};

fn row(organization: OrganizationId, provider: ModelProvider) -> String {
    format!("model_key:{}:{}", organization.as_uuid(), provider.as_str())
}

fn sealing(error: antilog_core::sealed::SealedError) -> StoreError {
    StoreError::Unavailable(Box::new(error))
}

impl PgStore {
    fn sealer(&self) -> Result<&Sealer, StoreError> {
        self.sealer.as_deref().ok_or_else(|| {
            StoreError::Unavailable("no master key is set, so model keys cannot be kept".into())
        })
    }
}

impl ModelKeyStore for PgStore {
    async fn put_model_key(
        &self,
        organization: OrganizationId,
        provider: ModelProvider,
        key: &Secret,
        at: i64,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let sealed = self
            .sealer()?
            .seal(key.expose(), &row(organization, provider))
            .map_err(sealing)?;
        let mut tx = self.scoped(organization).await?;
        sqlx::query!(
            "insert into model_keys (organization_id, provider, sealed, set_at) values ($1, $2, $3, to_timestamp($4::bigint / 1000.0)) on conflict (organization_id, provider) do update set sealed = excluded.sealed, set_at = excluded.set_at",
            organization.as_uuid(),
            provider.as_str(),
            sealed,
            at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn remove_model_key(
        &self,
        organization: OrganizationId,
        provider: ModelProvider,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let removed = sqlx::query!(
            "delete from model_keys where organization_id = $1 and provider = $2",
            organization.as_uuid(),
            provider.as_str(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        if removed.rows_affected() == 0 {
            return Err(StoreError::NotFound);
        }
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn model_keys(
        &self,
        organization: OrganizationId,
    ) -> Result<Vec<ModelKeyStatus>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query!(
            r#"select provider, (extract(epoch from set_at) * 1000)::bigint as "set_at!" from model_keys where organization_id = $1 order by provider"#,
            organization.as_uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?
        .into_iter()
        .map(|found| {
            Ok(ModelKeyStatus {
                provider: ModelProvider::parse(&found.provider)
                    .ok_or_else(|| corrupt("model provider", &found.provider))?,
                set_at: found.set_at,
            })
        })
        .collect()
    }

    async fn model_key(
        &self,
        organization: OrganizationId,
        provider: ModelProvider,
    ) -> Result<Option<Secret>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        let sealed = sqlx::query_scalar!(
            "select sealed from model_keys where organization_id = $1 and provider = $2",
            organization.as_uuid(),
            provider.as_str(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?;
        sealed
            .map(|sealed| {
                self.sealer()?
                    .open(&sealed, &row(organization, provider))
                    .map_err(sealing)
            })
            .transpose()
    }

    async fn gateway_scope(&self, session: SessionId) -> Result<Option<GatewayScope>, StoreError> {
        let found = sqlx::query!(
            r#"select organization_id as "organization_id!" from gateway_scope($1)"#,
            session.as_uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map)?;
        Ok(found.map(|found| GatewayScope {
            organization: OrganizationId::from_uuid(found.organization_id),
        }))
    }
}
