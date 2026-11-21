use antilog_core::store::{ConnectorRow, CustomizeStore, SessionScope, StoreError};
use antilog_types::{
    ApprovalId, AuditEntry, Connector, ConnectorHealth, ConnectorId, ConnectorTool,
    MemberId, OrganizationId, PartKind, Plugin, PluginPart, SessionId, Skill, SkillFile,
    SkillVersion, ToolMode,
};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{PgStore, audit, corrupt, map};

fn json<T: serde::Serialize>(value: &T) -> Result<String, StoreError> {
    serde_json::to_string(value).map_err(|error| StoreError::Unavailable(Box::new(error)))
}

fn unjson<T: serde::de::DeserializeOwned>(what: &str, text: &str) -> Result<T, StoreError> {
    serde_json::from_str(text).map_err(|_| corrupt(what, "not valid JSON"))
}

#[allow(clippy::too_many_arguments)]
fn connector(
    id: Uuid,
    name: String,
    slug: String,
    url: String,
    has_credential: bool,
    health: &str,
    tools: &str,
    created_by: Uuid,
    created_at: i64,
) -> Result<Connector, StoreError> {
    Ok(Connector {
        id: ConnectorId::from_uuid(id),
        name,
        slug,
        url,
        has_credential,
        health: ConnectorHealth::parse(health)
            .ok_or_else(|| corrupt("connector health", health))?,
        tools: unjson("connector tools", tools)?,
        created_by: MemberId::from_uuid(created_by),
        created_at,
    })
}

fn version(value: i32) -> Result<u32, StoreError> {
    u32::try_from(value).map_err(|_| corrupt("version", "negative"))
}

fn skill_version_of(
    name: String,
    version_number: i32,
    description: String,
    files: &str,
    size_bytes: i64,
    created_by: Uuid,
    created_at: i64,
) -> Result<SkillVersion, StoreError> {
    Ok(SkillVersion {
        name,
        version: version(version_number)?,
        description,
        files: unjson::<Vec<SkillFile>>("skill files", files)?,
        size_bytes: u64::try_from(size_bytes).map_err(|_| corrupt("skill size", "negative"))?,
        created_by: MemberId::from_uuid(created_by),
        created_at,
    })
}

async fn insert_connector(
    tx: &mut PgConnection,
    organization: OrganizationId,
    row: &ConnectorRow,
) -> Result<(), StoreError> {
    let c = &row.connector;
    sqlx::query!(
        "insert into connectors (organization_id, id, name, slug, url, sealed, health, tools, plugin_id, created_by, created_at) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, to_timestamp($11::bigint / 1000.0))",
        organization.as_uuid(),
        c.id.as_uuid(),
        c.name,
        c.slug,
        c.url,
        row.sealed.as_deref(),
        c.health.as_str(),
        json(&c.tools)?,
        row.plugin,
        c.created_by.as_uuid(),
        c.created_at,
    )
    .execute(tx)
    .await
    .map(|_| ())
    .map_err(map)
}

async fn insert_skill(
    tx: &mut PgConnection,
    organization: OrganizationId,
    skill: &SkillVersion,
    plugin: Option<&str>,
) -> Result<(), StoreError> {
    sqlx::query!(
        "insert into skill_versions (organization_id, name, version, description, files, size_bytes, plugin_id, created_by, created_at) values ($1, $2, $3, $4, $5, $6, $7, $8, to_timestamp($9::bigint / 1000.0))",
        organization.as_uuid(),
        skill.name,
        i32::try_from(skill.version).map_err(|_| StoreError::Conflict)?,
        skill.description,
        json(&skill.files)?,
        i64::try_from(skill.size_bytes).map_err(|_| StoreError::Conflict)?,
        plugin,
        skill.created_by.as_uuid(),
        skill.created_at,
    )
    .execute(tx)
    .await
    .map(|_| ())
    .map_err(map)
}

impl CustomizeStore for PgStore {
    async fn session_scope(&self, session: SessionId) -> Result<Option<SessionScope>, StoreError> {
        let mut tx = self.within("antilog.session_id", session.as_uuid()).await?;
        let found = sqlx::query!(
            r#"select organization_id, created_by from sessions where id = $1"#,
            session.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?;
        Ok(found.map(|row| SessionScope {
            organization: OrganizationId::from_uuid(row.organization_id),
            member: MemberId::from_uuid(row.created_by),
        }))
    }

    async fn use_approval(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
    ) -> Result<bool, StoreError> {
        let mut tx = self.scoped(organization).await?;
        let used = sqlx::query!(
            "insert into connector_tool_uses (organization_id, approval_id) values ($1, $2) on conflict do nothing",
            organization.as_uuid(),
            approval.as_uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?
        .rows_affected();
        tx.commit().await.map_err(map)?;
        Ok(used == 1)
    }

    async fn add_connector(
        &self,
        organization: OrganizationId,
        row: &ConnectorRow,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        insert_connector(&mut tx, organization, row).await?;
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn connectors(&self, organization: OrganizationId) -> Result<Vec<Connector>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query!(
            r#"select id, name, slug, url, sealed is not null as "has_credential!", health, tools, created_by, (extract(epoch from created_at) * 1000)::bigint as "created_at!" from connectors where organization_id = $1 order by created_at, id"#,
            organization.as_uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?
        .into_iter()
        .map(|r| {
            connector(r.id, r.name, r.slug, r.url, r.has_credential, &r.health, &r.tools, r.created_by, r.created_at)
        })
        .collect()
    }

    async fn connector(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
    ) -> Result<Option<Connector>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query!(
            r#"select id, name, slug, url, sealed is not null as "has_credential!", health, tools, created_by, (extract(epoch from created_at) * 1000)::bigint as "created_at!" from connectors where organization_id = $1 and id = $2"#,
            organization.as_uuid(),
            id.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .map(|r| {
            connector(r.id, r.name, r.slug, r.url, r.has_credential, &r.health, &r.tools, r.created_by, r.created_at)
        })
        .transpose()
    }

    async fn connector_credential(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        Ok(sqlx::query_scalar!(
            "select sealed from connectors where organization_id = $1 and id = $2",
            organization.as_uuid(),
            id.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .flatten())
    }

    async fn set_connector_check(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
        health: ConnectorHealth,
        tools: &[ConnectorTool],
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let changed = sqlx::query!(
            "update connectors set health = $3, tools = $4 where organization_id = $1 and id = $2",
            organization.as_uuid(),
            id.as_uuid(),
            health.as_str(),
            json(&tools)?,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?
        .rows_affected();
        if changed != 1 {
            return Err(StoreError::NotFound);
        }
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn set_tool_mode(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
        tool: &str,
        mode: ToolMode,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let text = sqlx::query_scalar!(
            "select tools from connectors where organization_id = $1 and id = $2 for update",
            organization.as_uuid(),
            id.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .ok_or(StoreError::NotFound)?;
        let mut tools: Vec<ConnectorTool> = unjson("connector tools", &text)?;
        tools
            .iter_mut()
            .find(|have| have.name == tool)
            .ok_or(StoreError::NotFound)?
            .mode = mode;
        sqlx::query!(
            "update connectors set tools = $3 where organization_id = $1 and id = $2",
            organization.as_uuid(),
            id.as_uuid(),
            json(&tools)?,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn set_connector_credential(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
        sealed: &[u8],
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let changed = sqlx::query!(
            "update connectors set sealed = $3, health = 'not_checked' where organization_id = $1 and id = $2",
            organization.as_uuid(),
            id.as_uuid(),
            sealed,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?
        .rows_affected();
        if changed != 1 {
            return Err(StoreError::NotFound);
        }
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn remove_connector(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let gone = sqlx::query!(
            "delete from connectors where organization_id = $1 and id = $2",
            organization.as_uuid(),
            id.as_uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?
        .rows_affected();
        if gone != 1 {
            return Err(StoreError::NotFound);
        }
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }




    async fn add_skill_version(
        &self,
        organization: OrganizationId,
        skill: &SkillVersion,
        plugin: Option<&str>,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        insert_skill(&mut tx, organization, skill, plugin).await?;
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn skills(&self, organization: OrganizationId) -> Result<Vec<Skill>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        let rows = sqlx::query!(
            "select name, version, description, plugin_id from skill_versions where organization_id = $1 order by name, version",
            organization.as_uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?;
        let mut out: Vec<Skill> = Vec::new();
        for row in rows {
            let number = version(row.version)?;
            match out.last_mut().filter(|have| have.name == row.name) {
                Some(have) => {
                    have.versions.push(number);
                    have.latest = number;
                    have.description = row.description;
                }
                None => out.push(Skill {
                    name: row.name,
                    description: row.description,
                    latest: number,
                    versions: vec![number],
                    from_plugin: row.plugin_id,
                }),
            }
        }
        Ok(out)
    }

    async fn skill_version(
        &self,
        organization: OrganizationId,
        name: &str,
        number: u32,
    ) -> Result<Option<SkillVersion>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query!(
            r#"select name, version, description, files, size_bytes, created_by, (extract(epoch from created_at) * 1000)::bigint as "created_at!" from skill_versions where organization_id = $1 and name = $2 and version = $3"#,
            organization.as_uuid(),
            name,
            i32::try_from(number).map_err(|_| StoreError::NotFound)?,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .map(|r| skill_version_of(r.name, r.version, r.description, &r.files, r.size_bytes, r.created_by, r.created_at))
        .transpose()
    }

    async fn remove_skill(
        &self,
        organization: OrganizationId,
        name: &str,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let gone = sqlx::query!(
            "delete from skill_versions where organization_id = $1 and name = $2",
            organization.as_uuid(),
            name,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?
        .rows_affected();
        if gone == 0 {
            return Err(StoreError::NotFound);
        }
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }






    async fn add_plugin(
        &self,
        organization: OrganizationId,
        plugin: &Plugin,
        connectors: &[ConnectorRow],
        skills: &[SkillVersion],
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query!(
            "insert into plugins (organization_id, id, name, description, version, parts, created_by, created_at) values ($1, $2, $3, $4, $5, $6, $7, to_timestamp($8::bigint / 1000.0))",
            organization.as_uuid(),
            plugin.id,
            plugin.name,
            plugin.description,
            plugin.version,
            json(&plugin.parts)?,
            plugin.created_by.as_uuid(),
            plugin.created_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        for row in connectors {
            let mut row = row.clone();
            row.plugin = Some(plugin.id.clone());
            insert_connector(&mut tx, organization, &row).await?;
        }
        for skill in skills {
            insert_skill(&mut tx, organization, skill, Some(&plugin.id)).await?;
        }
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn plugins(&self, organization: OrganizationId) -> Result<Vec<Plugin>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query!(
            r#"select id, name, description, version, parts, created_by, (extract(epoch from created_at) * 1000)::bigint as "created_at!" from plugins where organization_id = $1 order by created_at, id"#,
            organization.as_uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?
        .into_iter()
        .map(|r| {
            Ok(Plugin {
                id: r.id,
                name: r.name,
                description: r.description,
                version: r.version,
                parts: unjson("plugin parts", &r.parts)?,
                created_by: MemberId::from_uuid(r.created_by),
                created_at: r.created_at,
            })
        })
        .collect()
    }

    async fn remove_plugin(
        &self,
        organization: OrganizationId,
        id: &str,
        drop: &[PluginPart],
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        for part in drop {
            match part.kind {
                PartKind::Skill => {
                    sqlx::query!(
                        "delete from skill_versions where organization_id = $1 and plugin_id = $2 and name = $3",
                        organization.as_uuid(),
                        id,
                        part.name,
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(map)?;
                }
                PartKind::Connector => {
                    sqlx::query!(
                        "delete from connectors where organization_id = $1 and plugin_id = $2 and slug = $3",
                        organization.as_uuid(),
                        id,
                        part.name,
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(map)?;
                }
            }
        }
        sqlx::query!(
            "update skill_versions set plugin_id = null where organization_id = $1 and plugin_id = $2",
            organization.as_uuid(),
            id,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        sqlx::query!(
            "update connectors set plugin_id = null where organization_id = $1 and plugin_id = $2",
            organization.as_uuid(),
            id,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        let gone = sqlx::query!(
            "delete from plugins where organization_id = $1 and id = $2",
            organization.as_uuid(),
            id,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?
        .rows_affected();
        if gone != 1 {
            return Err(StoreError::NotFound);
        }
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }
}
