use std::future::Future;

use antilog_types::{
    ApprovalId, AuditEntry, Connector, ConnectorHealth, ConnectorId, ConnectorTool, MemberId,
    OrganizationId, Plugin, PluginPart, SessionId, Skill, SkillVersion, ToolMode,
};

use super::StoreError;

/// A connector and its sealed credential, as stored. The sealed bytes never leave the server.
#[derive(Debug, Clone)]
pub struct ConnectorRow {
    pub connector: Connector,
    /// `Sealer::seal(credential, "connector:<id>")`.
    pub sealed: Option<Vec<u8>>,
    pub plugin: Option<String>,
}

/// Who a session belongs to, found from its id alone: what the MCP endpoint needs once a token
/// has named the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionScope {
    pub organization: OrganizationId,
    /// The member the session works for.
    pub member: MemberId,
}

pub trait CustomizeStore {
    fn session_scope(
        &self,
        session: SessionId,
    ) -> impl Future<Output = Result<Option<SessionScope>, StoreError>> + Send;

    /// True the first time only: the approved card pays for one call.
    fn use_approval(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
    ) -> impl Future<Output = Result<bool, StoreError>> + Send;

    fn add_connector(
        &self,
        organization: OrganizationId,
        row: &ConnectorRow,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn connectors(
        &self,
        organization: OrganizationId,
    ) -> impl Future<Output = Result<Vec<Connector>, StoreError>> + Send;

    fn connector(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
    ) -> impl Future<Output = Result<Option<Connector>, StoreError>> + Send;

    fn connector_credential(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, StoreError>> + Send;

    /// What a check found: its health and the tools it listed (modes already merged).
    fn set_connector_check(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
        health: ConnectorHealth,
        tools: &[ConnectorTool],
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn set_tool_mode(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
        tool: &str,
        mode: ToolMode,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn set_connector_credential(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
        sealed: &[u8],
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn remove_connector(
        &self,
        organization: OrganizationId,
        id: ConnectorId,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// `Conflict` when the name and version exist.
    fn add_skill_version(
        &self,
        organization: OrganizationId,
        skill: &SkillVersion,
        plugin: Option<&str>,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Every skill with its versions.
    fn skills(
        &self,
        organization: OrganizationId,
    ) -> impl Future<Output = Result<Vec<Skill>, StoreError>> + Send;

    fn skill_version(
        &self,
        organization: OrganizationId,
        name: &str,
        version: u32,
    ) -> impl Future<Output = Result<Option<SkillVersion>, StoreError>> + Send;

    fn remove_skill(
        &self,
        organization: OrganizationId,
        name: &str,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// A plugin and the parts it brought, in one step. `Conflict` when the id exists.
    fn add_plugin(
        &self,
        organization: OrganizationId,
        plugin: &Plugin,
        connectors: &[ConnectorRow],
        skills: &[SkillVersion],
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn plugins(
        &self,
        organization: OrganizationId,
    ) -> impl Future<Output = Result<Vec<Plugin>, StoreError>> + Send;

    /// Removes the plugin and `drop`; every other part stays and loses its link to the plugin.
    fn remove_plugin(
        &self,
        organization: OrganizationId,
        id: &str,
        drop: &[PluginPart],
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;
}
