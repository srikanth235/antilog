use antilog_core::store::{Store, StoreError};
use antilog_types::{Organization, OrganizationId};
use sqlx::PgPool;
use sqlx::migrate::Migrator;

pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Clone)]
pub struct PgStore {
    pool: PgPool,
}

impl PgStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
        MIGRATOR.run(&self.pool).await
    }
}

fn unavailable(error: sqlx::Error) -> StoreError {
    StoreError::Unavailable(Box::new(error))
}

impl Store for PgStore {
    async fn ping(&self) -> Result<(), StoreError> {
        sqlx::query_scalar!("select 1 as \"one!\"")
            .fetch_one(&self.pool)
            .await
            .map(|_| ())
            .map_err(unavailable)
    }

    async fn create_organization(&self, organization: &Organization) -> Result<(), StoreError> {
        sqlx::query!(
            "insert into organizations (id, name, created_at) values ($1, $2, to_timestamp($3::bigint / 1000.0))",
            organization.id.as_uuid(),
            organization.name,
            organization.created_at,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|error| match error {
            sqlx::Error::Database(db) if db.is_unique_violation() => StoreError::Conflict,
            other => unavailable(other),
        })
    }

    async fn organization(&self, id: OrganizationId) -> Result<Option<Organization>, StoreError> {
        let row = sqlx::query!(
            "select id, name, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\" from organizations where id = $1",
            id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(unavailable)?;
        Ok(row.map(|row| Organization {
            id: OrganizationId::from_uuid(row.id),
            name: row.name,
            created_at: row.created_at,
        }))
    }
}
