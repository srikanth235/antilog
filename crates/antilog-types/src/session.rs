use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::id::text_enum;
use crate::{
    EnvironmentSpec, ExperimentId, MemberId, NetworkLevel, OrganizationId, ProjectId, SessionId,
};

text_enum!(
    /// Sealing is the judge's; Watching and Regressed are derived, never stored.
    ExperimentStatus { NotSealed => "not_sealed", Sealed => "sealed" }
);

/// The row of an experiment; its sealed definition belongs to the judge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Experiment {
    pub id: ExperimentId,
    pub organization_id: OrganizationId,
    pub project_id: ProjectId,
    pub goal: String,
    pub status: ExperimentStatus,
    /// What all its sessions and judge time may spend, in US cents.
    pub budget_cap_cents: i64,
    pub created_by: MemberId,
    pub created_at: i64,
    /// The composer's Mode: false asks a person for every card, true lets policy approve.
    pub follow_policy: bool,
}

impl Experiment {
    pub fn new(
        organization_id: OrganizationId,
        project_id: ProjectId,
        goal: impl Into<String>,
        budget_cap_cents: i64,
        created_by: MemberId,
        created_at: i64,
    ) -> Self {
        Self {
            id: ExperimentId::new(),
            organization_id,
            project_id,
            goal: goal.into(),
            status: ExperimentStatus::NotSealed,
            budget_cap_cents,
            created_by,
            created_at,
            follow_policy: false,
        }
    }
}

text_enum!(
    SessionStatus {
        Working => "working",
        WaitingForYou => "waiting_for_you",
        Done => "done",
        Failed => "failed",
    }
);

impl SessionStatus {
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Working | Self::WaitingForYou)
    }
}

/// One agent's work inside an experiment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Session {
    pub id: SessionId,
    pub organization_id: OrganizationId,
    pub experiment_id: ExperimentId,
    pub agent: String,
    pub status: SessionStatus,
    /// Why it ended, such as "Stopped by Ada" or "Handed off to Codex".
    pub note: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Session {
    pub fn new(
        organization_id: OrganizationId,
        experiment_id: ExperimentId,
        agent: impl Into<String>,
        created_at: i64,
    ) -> Self {
        Self {
            id: SessionId::new(),
            organization_id,
            experiment_id,
            agent: agent.into(),
            status: SessionStatus::Working,
            note: None,
            created_at,
            updated_at: created_at,
        }
    }
}

text_enum!(
    /// Who spoke: a person, an agent, or the name of a tool an agent called.
    EventKind { Person => "person", Agent => "agent", Tool => "tool" }
);

/// One item of a session's conversation. A tool call carries only its name, never its input or
/// output; the server stores the words people and agents said and nothing a tool returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionEvent {
    /// Orders the conversation; a page is asked for by the last `seq` seen.
    pub seq: i64,
    pub session_id: SessionId,
    pub kind: EventKind,
    /// Who said it, for a person's message.
    pub member_id: Option<MemberId>,
    pub text: String,
    pub at: i64,
}

/// Oldest first. `next` is the `after` for the page that follows, or `null` when this is the end so far.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EventPage {
    pub events: Vec<SessionEvent>,
    pub next: Option<i64>,
}

text_enum!(
    /// "$", "$$" or "$$$", set from the agent's price.
    CostTier { Low => "low", Medium => "medium", High => "high" }
);

/// An agent an organization may run: a model and the harness around it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Agent {
    /// A lower-case slug such as `claude-code`; sessions name it.
    pub id: String,
    pub name: String,
    /// Auto chooses only among the agents that are turned on.
    pub enabled: bool,
    /// What Auto uses before it has data; an owner sets it, and it is never set by brand.
    pub is_default: bool,
    pub cost_tier: CostTier,
    /// Offered because the organization has set none up yet; an owner's own list replaces it.
    pub built_in: bool,
}

/// What an owner sets for an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewAgent {
    pub name: String,
    pub enabled: bool,
    pub is_default: bool,
    pub cost_tier: CostTier,
}

/// The agent Auto would start, and why in one line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutoChoice {
    pub agent: Agent,
    pub reason: String,
}

/// One version of an environment an organization manages, or the built-in default. A change
/// makes a new version; a session records the version it ran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub version: u32,
    pub network: NetworkLevel,
    /// Exact host names; used at the limited level.
    pub allowed_hosts: Vec<String>,
    pub allow_package_managers: bool,
    /// Plain variables, readable by the agent. Model keys never go here.
    pub variables: BTreeMap<String, String>,
    /// Runs as root once per version before the agent starts; empty for none.
    pub setup_script: String,
    pub cpus: u32,
    pub memory_mb: u32,
    /// The default every organization has; it is not stored and cannot be changed.
    pub built_in: bool,
    pub archived: bool,
    /// Who made this version; empty for the built-in default.
    pub created_by: Option<MemberId>,
    /// When this version was made, in milliseconds.
    pub created_at: i64,
}

impl Environment {
    /// The built-in default, which matches `EnvironmentSpec::default()`.
    pub fn built_in() -> Self {
        let spec = EnvironmentSpec::default();
        Self {
            id: spec.id,
            name: "Default".to_owned(),
            version: spec.version,
            network: spec.network,
            allowed_hosts: spec.allowed_hosts,
            allow_package_managers: spec.allow_package_managers,
            variables: spec.variables,
            setup_script: spec.setup_script,
            cpus: spec.cpus,
            memory_mb: spec.memory_mb,
            built_in: true,
            archived: false,
            created_by: None,
            created_at: 0,
        }
    }

    /// What a runner receives and the proxy enforces.
    pub fn spec(&self) -> EnvironmentSpec {
        EnvironmentSpec {
            id: self.id.clone(),
            version: self.version,
            network: self.network,
            allowed_hosts: self.allowed_hosts.clone(),
            allow_package_managers: self.allow_package_managers,
            variables: self.variables.clone(),
            setup_script: self.setup_script.clone(),
            cpus: self.cpus,
            memory_mb: self.memory_mb,
        }
    }
}

/// What makes an environment, or a new version of one. The id comes from the name on create.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewEnvironment {
    pub name: String,
    pub network: NetworkLevel,
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    pub allow_package_managers: bool,
    /// Readable by the agent: a service key goes here, a model key never does.
    #[serde(default)]
    pub variables: BTreeMap<String, String>,
    /// At most 64 KB.
    #[serde(default)]
    pub setup_script: String,
    pub cpus: u32,
    pub memory_mb: u32,
}

/// What starts an experiment in a project. The budget defaults to the project's, then the organization's.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewExperiment {
    pub goal: String,
    pub budget_cap_cents: Option<i64>,
    #[serde(default)]
    pub follow_policy: bool,
}

/// What starts a session in an experiment.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartSession {
    pub agent: String,
    pub prompt: String,
    #[serde(default)]
    pub repos: Vec<String>,
    pub environment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SendMessage {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[schema(value_type = std::collections::HashMap<String, serde_json::Value>)]
    pub input_schema: serde_json::Value,
}

/// A call of one Antilog tool from an agent that is not in an Antilog session, such as `antilog mcp`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolCall {
    /// The session the calls belong to; the first call without one opens it.
    pub session: Option<SessionId>,
    /// Unique per call; sending a call again with the same id changes nothing twice.
    pub call: Option<String>,
    #[schema(value_type = std::collections::HashMap<String, serde_json::Value>)]
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolResult {
    pub session: SessionId,
    #[schema(value_type = std::collections::HashMap<String, serde_json::Value>)]
    pub answer: serde_json::Value,
    /// A measurement that was accepted: how it ended, in words. Never a holdout value.
    pub followup: Option<String>,
}

text_enum!(
    /// What an evaluation measures.
    EvaluationKind { Speed => "speed", Quality => "quality", Size => "size", Cost => "cost" }
);

/// A starting point for an experiment's evaluation in Customize, Evaluations. An experiment starts
/// from a copy; the template holds a command and paths, never a credential or data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvaluationTemplate {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub kind: EvaluationKind,
    /// What is measured, as a dotted path in the command's JSON output.
    pub metric: String,
    pub lower_is_better: bool,
    pub command: String,
    /// Files an agent may never change, each as `repo:path`.
    pub protected_paths: Vec<String>,
    pub created_by: MemberId,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewEvaluationTemplate {
    pub name: String,
    #[serde(default)]
    pub summary: String,
    pub kind: EvaluationKind,
    pub metric: String,
    pub lower_is_better: bool,
    pub command: String,
    #[serde(default)]
    pub protected_paths: Vec<String>,
}
