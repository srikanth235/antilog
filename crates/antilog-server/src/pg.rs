use antilog_core::store::{Invariant, Store, StoreError};
use antilog_types::{AuditEntry, OrganizationId};
use sqlx::migrate::Migrator;
use sqlx::{PgConnection, PgPool, Postgres, Transaction};

mod approval;
mod budget;
mod card;
mod code_host;
mod judge;
mod organization;
mod session;

pub use judge::PgJudge;

pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Clone)]
pub struct PgStore {
    pool: PgPool,
}

impl PgStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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
                if matches!(
                    db.constraint(),
                    Some("organizations_default_within_cap" | "projects_default_within_cap")
                ) =>
            {
                Some(Invariant::OverCap)
            }
            Some("23514")
                if matches!(
                    db.constraint(),
                    Some(
                        "policies_add_credential_asks"
                            | "approvals_credential_asks"
                            | "approvals_owner_asks"
                    )
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
