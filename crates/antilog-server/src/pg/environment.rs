use antilog_core::secrets::Secret;
use antilog_core::store::{EnvironmentStore, Invariant, SecretValue, StoreError};
use antilog_types::{
    AuditEntry, Environment, MemberId, NetworkLevel, NetworkSecret, OrganizationId,
};

use super::{PgStore, audit, corrupt, map};

fn json<T: serde::Serialize>(value: &T) -> Result<String, StoreError> {
    serde_json::to_string(value).map_err(|error| StoreError::Unavailable(Box::new(error)))
}

struct Row {
    id: String,
    archived: bool,
    version: i32,
    name: String,
    network: String,
    allowed_hosts: String,
    allow_package_managers: bool,
    variables: String,
    setup_script: String,
    repos: String,
    network_secrets: String,
    cpus: i32,
    memory_mb: i32,
    created_by: Option<uuid::Uuid>,
    created_at: i64,
}

impl TryFrom<Row> for Environment {
    type Error = StoreError;

    fn try_from(row: Row) -> Result<Self, StoreError> {
        let number = |what: &str, value: i32| {
            u32::try_from(value).map_err(|_| corrupt(what, &value.to_string()))
        };
        Ok(Environment {
            id: row.id,
            name: row.name,
            version: number("environment version", row.version)?,
            network: NetworkLevel::parse(&row.network)
                .ok_or_else(|| corrupt("network level", &row.network))?,
            allowed_hosts: serde_json::from_str(&row.allowed_hosts)
                .map_err(|_| corrupt("allowed hosts", "not a list"))?,
            allow_package_managers: row.allow_package_managers,
            variables: serde_json::from_str(&row.variables)
                .map_err(|_| corrupt("environment variables", "not an object"))?,
            setup_script: row.setup_script,
            repos: serde_json::from_str(&row.repos)
                .map_err(|_| corrupt("environment repos", "not a list"))?,
            network_secrets: serde_json::from_str(&row.network_secrets)
                .map_err(|_| corrupt("network secrets", "not a list"))?,
            cpus: number("cpus", row.cpus)?,
            memory_mb: number("memory", row.memory_mb)?,
            built_in: false,
            archived: row.archived,
            created_by: row.created_by.map(MemberId::from_uuid),
            created_at: row.created_at,
        })
    }
}

/// Where a secret's value lives. It is not the version's, so a carried-over value opens in the
/// next version.
fn secret_row(organization: OrganizationId, id: &str, host: &str, header: &str) -> String {
    format!(
        "env_secret:{}:{id}:{host}:{}",
        organization.as_uuid(),
        header.to_ascii_lowercase()
    )
}

fn sealing(error: antilog_core::sealed::SealedError) -> StoreError {
    StoreError::Unavailable(Box::new(error))
}

impl PgStore {
    /// Seals each new value, and copies the sealed value of the previous version for one left
    /// out. Called in the version's own transaction, after the version row is in.
    async fn file_secrets(
        &self,
        tx: &mut sqlx::PgConnection,
        organization: OrganizationId,
        environment: &Environment,
        secrets: &[SecretValue],
    ) -> Result<(), StoreError> {
        for secret in secrets {
            let sealed: Vec<u8> = match &secret.value {
                Some(value) => self
                    .sealer
                    .as_deref()
                    .ok_or_else(|| {
                        StoreError::Unavailable(
                            "no master key is set, so network secrets cannot be kept".into(),
                        )
                    })?
                    .seal(
                        value.expose(),
                        &secret_row(organization, &environment.id, &secret.host, &secret.header),
                    )
                    .map_err(sealing)?,
                None => sqlx::query_scalar!(
                    "select sealed from environment_secrets where organization_id = $1 and id = $2 and host = $3 and lower(header) = lower($4) and version < $5 order by version desc limit 1",
                    organization.as_uuid(),
                    environment.id,
                    secret.host,
                    secret.header,
                    i32::try_from(environment.version).unwrap_or(i32::MAX),
                )
                .fetch_optional(&mut *tx)
                .await
                .map_err(map)?
                .ok_or(StoreError::NotFound)?,
            };
            sqlx::query!(
                "insert into environment_secrets (organization_id, id, version, host, header, sealed) values ($1, $2, $3, $4, $5, $6)",
                organization.as_uuid(),
                environment.id,
                i32::try_from(environment.version).unwrap_or(i32::MAX),
                secret.host,
                secret.header,
                sealed,
            )
            .execute(&mut *tx)
            .await
            .map_err(map)?;
        }
        Ok(())
    }
}

async fn insert_version(
    tx: &mut sqlx::PgConnection,
    organization: OrganizationId,
    environment: &Environment,
) -> Result<(), StoreError> {
    sqlx::query!(
        "insert into environment_versions (organization_id, id, version, name, network, allowed_hosts, allow_package_managers, variables, setup_script, repos, network_secrets, cpus, memory_mb, created_by, created_at) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, to_timestamp($15::bigint / 1000.0))",
        organization.as_uuid(),
        environment.id,
        i32::try_from(environment.version).unwrap_or(i32::MAX),
        environment.name,
        environment.network.as_str(),
        json(&environment.allowed_hosts)?,
        environment.allow_package_managers,
        json(&environment.variables)?,
        environment.setup_script,
        json(&environment.repos)?,
        json(&environment.network_secrets)?,
        i32::try_from(environment.cpus).unwrap_or(i32::MAX),
        i32::try_from(environment.memory_mb).unwrap_or(i32::MAX),
        environment.created_by.map(|member| member.as_uuid()),
        environment.created_at,
    )
    .execute(tx)
    .await
    .map(|_| ())
    .map_err(map)
}

impl EnvironmentStore for PgStore {
    async fn add_environment(
        &self,
        organization: OrganizationId,
        environment: &Environment,
        secrets: &[SecretValue],
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query!(
            "insert into environments (organization_id, id, archived, created_at) values ($1, $2, false, to_timestamp($3::bigint / 1000.0))",
            organization.as_uuid(),
            environment.id,
            environment.created_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        insert_version(&mut tx, organization, environment).await?;
        self.file_secrets(&mut tx, organization, environment, secrets)
            .await?;
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn add_environment_version(
        &self,
        organization: OrganizationId,
        environment: &Environment,
        secrets: &[SecretValue],
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        // The row lock serializes two edits of one environment: the second sees the first's
        // version and is refused as a conflict.
        let head = sqlx::query!(
            r#"select e.archived as "archived!", coalesce((select max(v.version) from environment_versions v where v.organization_id = e.organization_id and v.id = e.id), 0) as "newest!"
               from environments e where e.organization_id = $1 and e.id = $2 for update"#,
            organization.as_uuid(),
            environment.id,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .ok_or(StoreError::NotFound)?;
        if head.archived {
            return Err(Invariant::IllegalTransition.into());
        }
        if i64::from(environment.version) != i64::from(head.newest) + 1 {
            return Err(StoreError::Conflict);
        }
        insert_version(&mut tx, organization, environment).await?;
        self.file_secrets(&mut tx, organization, environment, secrets)
            .await?;
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn archive_environment(
        &self,
        organization: OrganizationId,
        id: &str,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let changed = sqlx::query!(
            "update environments set archived = true where organization_id = $1 and id = $2",
            organization.as_uuid(),
            id,
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

    async fn environments(
        &self,
        organization: OrganizationId,
    ) -> Result<Vec<Environment>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_as!(
            Row,
            r#"select distinct on (v.id) v.id, e.archived, v.version, v.name, v.network, v.allowed_hosts, v.allow_package_managers, v.variables, v.setup_script, v.repos, v.network_secrets, v.cpus, v.memory_mb, v.created_by, (extract(epoch from v.created_at) * 1000)::bigint as "created_at!"
               from environment_versions v join environments e on e.organization_id = v.organization_id and e.id = v.id
               where v.organization_id = $1 order by v.id, v.version desc"#,
            organization.as_uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?
        .into_iter()
        .map(Environment::try_from)
        .collect()
    }

    async fn environment_version(
        &self,
        organization: OrganizationId,
        id: &str,
        version: u32,
    ) -> Result<Option<Environment>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_as!(
            Row,
            r#"select v.id, e.archived, v.version, v.name, v.network, v.allowed_hosts, v.allow_package_managers, v.variables, v.setup_script, v.repos, v.network_secrets, v.cpus, v.memory_mb, v.created_by, (extract(epoch from v.created_at) * 1000)::bigint as "created_at!"
               from environment_versions v join environments e on e.organization_id = v.organization_id and e.id = v.id
               where v.organization_id = $1 and v.id = $2 and v.version = $3"#,
            organization.as_uuid(),
            id,
            i32::try_from(version).unwrap_or(i32::MAX),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .map(Environment::try_from)
        .transpose()
    }

    async fn network_secret_values(
        &self,
        organization: OrganizationId,
        id: &str,
        version: u32,
    ) -> Result<Vec<(NetworkSecret, Secret)>, StoreError> {
        let Some(found) = self.environment_version(organization, id, version).await? else {
            return Ok(Vec::new());
        };
        let sealer = self.sealer.as_deref().ok_or_else(|| {
            StoreError::Unavailable(
                "no master key is set, so network secrets cannot be read".into(),
            )
        })?;
        let mut tx = self.scoped(organization).await?;
        let rows = sqlx::query!(
            "select host, header, sealed from environment_secrets where organization_id = $1 and id = $2 and version = $3",
            organization.as_uuid(),
            id,
            i32::try_from(version).unwrap_or(i32::MAX),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?;
        let mut opened = Vec::new();
        for meta in found.network_secrets {
            if let Some(row) = rows
                .iter()
                .find(|row| row.host == meta.host && row.header == meta.header)
            {
                let value = sealer
                    .open(
                        &row.sealed,
                        &secret_row(organization, id, &row.host, &row.header),
                    )
                    .map_err(sealing)?;
                opened.push((meta, value));
            }
        }
        Ok(opened)
    }
}
