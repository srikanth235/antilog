//! Customize: connectors, skills and plugins (`docs/V0.md`, "What the agent can reach").

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::id::text_enum;
use crate::{ConnectorId, MemberId};

text_enum!(
    /// What a session may do with one tool of a connector.
    ToolMode { On => "on", Off => "off", Ask => "ask" }
);

text_enum!(
    /// Never shown as healthy before it was checked.
    ConnectorHealth { Connected => "connected", Reconnect => "reconnect", NotChecked => "not_checked" }
);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectorTool {
    pub name: String,
    pub description: String,
    pub mode: ToolMode,
}

/// A remote MCP server. The credential is kept sealed on the server and never shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Connector {
    pub id: ConnectorId,
    /// Also the prefix of its tools' names in a session: `<slug>__<tool>`.
    pub name: String,
    pub slug: String,
    pub url: String,
    pub has_credential: bool,
    pub health: ConnectorHealth,
    pub tools: Vec<ConnectorTool>,
    pub created_by: MemberId,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewConnector {
    pub name: String,
    pub url: String,
    /// Sent to the server as a bearer token. Stored sealed; never returned.
    #[serde(default)]
    pub credential: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetToolMode {
    pub mode: ToolMode,
}

/// One text file of a skill pack, by path inside its folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewSkill {
    /// Text files; `SKILL.md` is required at the top. The name comes from its front matter.
    pub files: Vec<SkillFile>,
}

/// One immutable version of an organization skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillVersion {
    pub name: String,
    pub version: u32,
    pub description: String,
    pub files: Vec<SkillFile>,
    pub size_bytes: u64,
    pub created_by: MemberId,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub latest: u32,
    pub versions: Vec<u32>,
    /// A project turns it Off to hide it from New experiment there.
    pub on: bool,
    pub from_plugin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetOn {
    pub on: bool,
}

/// The skill versions an experiment selected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillRef {
    pub name: String,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectSkills {
    /// Names; each is pinned to its latest version now.
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectConnectors {
    pub connectors: Vec<ConnectorId>,
}

text_enum!(PartKind { Skill => "skill", Connector => "connector", Evaluation => "evaluation" });

/// What a plugin brought, by kind and name (a skill's name, a connector's id, an evaluation's id).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginPart {
    pub kind: PartKind,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub parts: Vec<PluginPart>,
    pub created_by: MemberId,
    pub created_at: i64,
}

/// A connector a plugin names. Its credential is added afterwards, on the connector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginConnector {
    pub name: String,
    pub url: String,
}

/// The bundle an owner or editor adds: a manifest and its parts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewPlugin {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub skills: Vec<NewSkill>,
    #[serde(default)]
    pub connectors: Vec<PluginConnector>,
    #[serde(default)]
    pub evaluations: Vec<crate::NewEvaluationTemplate>,
}

/// What to do with each part when a plugin is removed; a part not named is removed, and one in use refuses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemovePlugin {
    #[serde(default)]
    pub keep: Vec<PluginPart>,
}
