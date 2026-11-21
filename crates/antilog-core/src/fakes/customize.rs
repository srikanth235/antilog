use std::collections::HashSet;

use antilog_types::{
    ApprovalId, AuditEntry, Connector, ConnectorHealth, ConnectorId, ConnectorTool, OrganizationId,
    Plugin, PluginPart, SessionId, Skill, SkillVersion, ToolMode,
};

use super::{FakeStore, locked};
use crate::store::{ConnectorRow, CustomizeStore, SessionScope, StoreError};

#[derive(Default)]
pub(super) struct State {
    used: HashSet<(OrganizationId, ApprovalId)>,
    connectors: Vec<(OrganizationId, ConnectorRow)>,
    skills: Vec<(OrganizationId, SkillVersion, Option<String>)>,
    plugins: Vec<(OrganizationId, Plugin)>,
}

impl FakeStore {
    fn audit_one(&self, entry: &AuditEntry) {
        locked(&self.ledger).audit.push(entry.clone());
    }
}

impl CustomizeStore for FakeStore {
    async fn session_scope(&self, session: SessionId) -> Result<Option<SessionScope>, StoreError> {
        let ledger = locked(&self.ledger);
        let Some(row) = ledger.sessions.iter().find(|row| row.id == session) else {
            return Ok(None);
        };
        Ok(Some(SessionScope {
            organization: row.organization_id,
            member: row.created_by,
        }))
    }

    async fn use_approval(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
    ) -> Result<bool, StoreError> {
        Ok(locked(&self.customize)
            .used
            .insert((organization, approval)))
    }

    async fn add_connector(
        &self,
        organization: OrganizationId,
        row: &ConnectorRow,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut state = locked(&self.customize);
        if state
            .connectors
            .iter()
            .any(|(org, have)| *org == organization && have.connector.slug == row.connector.slug)
        {
            return Err(StoreError::Conflict);
        }
        state.connectors.push((organization, row.clone()));
        self.audit_one(audit);
        Ok(())
    }

    async fn connectors(&self, organization: OrganizationId) -> Result<Vec<Connector>, StoreError> {
        Ok(locked(&self.customize)
            .connectors
            .iter()
            .filter(|(org, _)| *org == organization)
            .map(|(_, row)| row.connector.clone())
            .collect())
    }

    async fn connector(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
    ) -> Result<Option<Connector>, StoreError> {
        Ok(locked(&self.customize)
            .connectors
            .iter()
            .find(|(org, row)| *org == organization && row.connector.id == id)
            .map(|(_, row)| row.connector.clone()))
    }

    async fn connector_credential(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        Ok(locked(&self.customize)
            .connectors
            .iter()
            .find(|(org, row)| *org == organization && row.connector.id == id)
            .and_then(|(_, row)| row.sealed.clone()))
    }

    async fn set_connector_check(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
        health: ConnectorHealth,
        tools: &[ConnectorTool],
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut state = locked(&self.customize);
        let (_, row) = state
            .connectors
            .iter_mut()
            .find(|(org, row)| *org == organization && row.connector.id == id)
            .ok_or(StoreError::NotFound)?;
        row.connector.health = health;
        row.connector.tools = tools.to_vec();
        self.audit_one(audit);
        Ok(())
    }

    async fn set_tool_mode(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
        tool: &str,
        mode: ToolMode,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut state = locked(&self.customize);
        let (_, row) = state
            .connectors
            .iter_mut()
            .find(|(org, row)| *org == organization && row.connector.id == id)
            .ok_or(StoreError::NotFound)?;
        let found = row
            .connector
            .tools
            .iter_mut()
            .find(|have| have.name == tool)
            .ok_or(StoreError::NotFound)?;
        found.mode = mode;
        self.audit_one(audit);
        Ok(())
    }

    async fn set_connector_credential(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
        sealed: &[u8],
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut state = locked(&self.customize);
        let (_, row) = state
            .connectors
            .iter_mut()
            .find(|(org, row)| *org == organization && row.connector.id == id)
            .ok_or(StoreError::NotFound)?;
        row.sealed = Some(sealed.to_vec());
        row.connector.has_credential = true;
        row.connector.health = ConnectorHealth::NotChecked;
        self.audit_one(audit);
        Ok(())
    }

    async fn remove_connector(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut state = locked(&self.customize);
        let before = state.connectors.len();
        state
            .connectors
            .retain(|(org, row)| !(*org == organization && row.connector.id == id));
        if state.connectors.len() == before {
            return Err(StoreError::NotFound);
        }
        self.audit_one(audit);
        Ok(())
    }

    async fn add_skill_version(
        &self,
        organization: OrganizationId,
        skill: &SkillVersion,
        plugin: Option<&str>,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut state = locked(&self.customize);
        if state.skills.iter().any(|(org, have, _)| {
            *org == organization && have.name == skill.name && have.version == skill.version
        }) {
            return Err(StoreError::Conflict);
        }
        state
            .skills
            .push((organization, skill.clone(), plugin.map(str::to_owned)));
        self.audit_one(audit);
        Ok(())
    }

    async fn skills(&self, organization: OrganizationId) -> Result<Vec<Skill>, StoreError> {
        let state = locked(&self.customize);
        let mut out: Vec<Skill> = Vec::new();
        for (org, skill, plugin) in &state.skills {
            if *org != organization {
                continue;
            }
            match out.iter_mut().find(|have| have.name == skill.name) {
                Some(have) => {
                    have.versions.push(skill.version);
                    if skill.version >= have.latest {
                        have.latest = skill.version;
                        have.description = skill.description.clone();
                    }
                }
                None => out.push(Skill {
                    name: skill.name.clone(),
                    description: skill.description.clone(),
                    latest: skill.version,
                    versions: vec![skill.version],
                    from_plugin: plugin.clone(),
                }),
            }
        }
        for skill in &mut out {
            skill.versions.sort_unstable();
        }
        Ok(out)
    }

    async fn skill_version(
        &self,
        organization: OrganizationId,
        name: &str,
        version: u32,
    ) -> Result<Option<SkillVersion>, StoreError> {
        Ok(locked(&self.customize)
            .skills
            .iter()
            .find(|(org, have, _)| {
                *org == organization && have.name == name && have.version == version
            })
            .map(|(_, have, _)| have.clone()))
    }

    async fn remove_skill(
        &self,
        organization: OrganizationId,
        name: &str,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut state = locked(&self.customize);
        let before = state.skills.len();
        state
            .skills
            .retain(|(org, have, _)| !(*org == organization && have.name == name));
        if state.skills.len() == before {
            return Err(StoreError::NotFound);
        }
        self.audit_one(audit);
        Ok(())
    }

    async fn add_plugin(
        &self,
        organization: OrganizationId,
        plugin: &Plugin,
        connectors: &[ConnectorRow],
        skills: &[SkillVersion],
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut state = locked(&self.customize);
        if state
            .plugins
            .iter()
            .any(|(org, have)| *org == organization && have.id == plugin.id)
            || connectors.iter().any(|row| {
                state.connectors.iter().any(|(org, have)| {
                    *org == organization && have.connector.slug == row.connector.slug
                })
            })
            || skills.iter().any(|skill| {
                state.skills.iter().any(|(org, have, _)| {
                    *org == organization && have.name == skill.name && have.version == skill.version
                })
            })
        {
            return Err(StoreError::Conflict);
        }
        state.plugins.push((organization, plugin.clone()));
        for row in connectors {
            let mut row = row.clone();
            row.plugin = Some(plugin.id.clone());
            state.connectors.push((organization, row));
        }
        for skill in skills {
            state
                .skills
                .push((organization, skill.clone(), Some(plugin.id.clone())));
        }
        self.audit_one(audit);
        Ok(())
    }

    async fn plugins(&self, organization: OrganizationId) -> Result<Vec<Plugin>, StoreError> {
        Ok(locked(&self.customize)
            .plugins
            .iter()
            .filter(|(org, _)| *org == organization)
            .map(|(_, plugin)| plugin.clone())
            .collect())
    }

    async fn remove_plugin(
        &self,
        organization: OrganizationId,
        id: &str,
        drop: &[PluginPart],
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        use antilog_types::PartKind;
        let mut state = locked(&self.customize);
        let before = state.plugins.len();
        state
            .plugins
            .retain(|(org, have)| !(*org == organization && have.id == id));
        if state.plugins.len() == before {
            return Err(StoreError::NotFound);
        }
        let dropped = |kind: PartKind, name: &str| {
            drop.iter()
                .any(|part| part.kind == kind && part.name == name)
        };
        state.skills.retain(|(org, have, plugin)| {
            !(*org == organization
                && plugin.as_deref() == Some(id)
                && dropped(PartKind::Skill, &have.name))
        });
        state.connectors.retain(|(org, row)| {
            !(*org == organization
                && row.plugin.as_deref() == Some(id)
                && dropped(PartKind::Connector, &row.connector.slug))
        });
        for (org, _, plugin) in &mut state.skills {
            if *org == organization && plugin.as_deref() == Some(id) {
                *plugin = None;
            }
        }
        for (org, row) in &mut state.connectors {
            if *org == organization && row.plugin.as_deref() == Some(id) {
                row.plugin = None;
            }
        }
        self.audit_one(audit);
        Ok(())
    }
}
