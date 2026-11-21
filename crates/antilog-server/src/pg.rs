use antilog_core::sealed::Sealer;
use antilog_core::store::{Invariant, Store, StoreError};
use antilog_types::{AuditEntry, OrganizationId};
use sqlx::migrate::Migrator;
use sqlx::{PgConnection, PgPool, Postgres, Transaction};

mod approval;
mod budget;
mod card;
mod credentials;
mod customize;
mod environment;
mod model_key;
mod organization;
mod runner;
mod session;

pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Clone)]
pub struct PgStore {
    pool: PgPool,
    /// Seals the organization's model keys; without it they cannot be kept or read.
    sealer: Option<std::sync::Arc<Sealer>>,
}

impl PgStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool, sealer: None }
    }

    /// The master key for model keys, from the `Secrets` store.
    #[must_use]
    pub fn with_sealer(mut self, sealer: Sealer) -> Self {
        self.sealer = Some(std::sync::Arc::new(sealer));
        self
    }

    /// A transaction that sees only this organization's rows: the second guard
    /// behind each query's own filter.
    async fn scoped(
        &self,
        organization: OrganizationId,
    ) -> Result<Transaction<'static, Postgres>, StoreError> {
        self.within("antilog.organization_id", organization.as_uuid())
            .await
    }

    /// A transaction that sees one token or sign-in session whose organization is not yet known.
    async fn found(&self, id: uuid::Uuid) -> Result<Transaction<'static, Postgres>, StoreError> {
        self.within("antilog.token_id", id).await
    }

    async fn within(
        &self,
        setting: &str,
        value: uuid::Uuid,
    ) -> Result<Transaction<'static, Postgres>, StoreError> {
        let mut tx = self.pool.begin().await.map_err(map)?;
        sqlx::query("select set_config($1, $2, true)")
            .bind(setting)
            .bind(value.to_string())
            .execute(&mut *tx)
            .await
            .map_err(map)?;
        Ok(tx)
    }

    /// Seals model keys with the master key named `model-key-master` in `secrets`. Without one the
    /// store still runs; saving or reading a model key is refused, and that is logged here once.
    #[must_use]
    pub async fn sealed_by(self, secrets: &impl antilog_core::secrets::Secrets) -> Self {
        match secrets.get("model-key-master").await {
            Ok(master) => match Sealer::new(&master) {
                Ok(sealer) => self.with_sealer(sealer),
                Err(error) => {
                    tracing::error!(%error, "model-key-master is not a usable master key");
                    self
                }
            },
            Err(error) => {
                tracing::warn!(%error, "no master key: model keys cannot be saved or used");
                self
            }
        }
    }

    pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
        MIGRATOR.run(&self.pool).await
    }
}

fn unavailable(error: sqlx::Error) -> StoreError {
    StoreError::Unavailable(Box::new(error))
}

fn corrupt(what: &str, value: &str) -> StoreError {
    unavailable(sqlx::Error::Decode(
        format!("unknown {what} {value:?} in the store").into(),
    ))
}

fn map(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(db) = &error {
        let invariant = match db.code().as_deref() {
            Some("23505") => return StoreError::Conflict,
            Some("23503") => return StoreError::NotFound,
            Some("AL001") => Some(Invariant::LastOwner),
            Some("AL002") => Some(Invariant::OverCap),
            Some("AL003") => Some(Invariant::IllegalTransition),
            Some("AL005") => Some(Invariant::AppendOnly),
            Some("23514")
                if matches!(db.constraint(), Some("organizations_default_within_cap")) =>
            {
                Some(Invariant::OverCap)
            }
            Some("23514")
                if matches!(
                    db.constraint(),
                    Some("policies_add_credential_asks" | "approvals_credential_asks")
                ) =>
            {
                Some(Invariant::AlwaysAsk)
            }
            _ => None,
        };
        if let Some(invariant) = invariant {
            return invariant.into();
        }
    }
    unavailable(error)
}

async fn audit(connection: &mut PgConnection, entry: &AuditEntry) -> Result<(), StoreError> {
    sqlx::query!(
        "insert into audit_log (id, organization_id, actor_id, action, target, outcome, at) values ($1, $2, $3, $4, $5, $6, to_timestamp($7::bigint / 1000.0))",
        entry.id.as_uuid(),
        entry.organization_id.as_uuid(),
        entry.actor.map(|actor| actor.as_uuid()),
        entry.action,
        entry.target,
        entry.outcome.as_str(),
        entry.at,
    )
    .execute(connection)
    .await
    .map(|_| ())
    .map_err(map)
}

impl Store for PgStore {
    async fn ping(&self) -> Result<(), StoreError> {
        sqlx::query_scalar!("select 1 as \"one!\"")
            .fetch_one(&self.pool)
            .await
            .map(|_| ())
            .map_err(unavailable)
    }
}
