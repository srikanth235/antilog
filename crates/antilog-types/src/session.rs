use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::id::text_enum;
use crate::{ExperimentId, MemberId, OrganizationId, ProjectId, SessionId};

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

text_enum!(Cleanup { Removed => "removed", NotRemoved => "not_removed" });

/// One agent's work inside an experiment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Session {
    pub id: SessionId,
    pub organization_id: OrganizationId,
    pub experiment_id: ExperimentId,
    /// Set on a spawned session; a spawned session cannot spawn.
    pub parent: Option<SessionId>,
    pub agent: String,
    pub platform: String,
    pub platform_handle: Option<String>,
    pub status: SessionStatus,
    /// Why it ended, such as "Stopped by Ada" or "Handed off to Codex".
    pub note: Option<String>,
    pub cleanup: Option<Cleanup>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Session {
    pub fn new(
        organization_id: OrganizationId,
        experiment_id: ExperimentId,
        agent: impl Into<String>,
        platform: impl Into<String>,
        created_at: i64,
    ) -> Self {
        Self {
            id: SessionId::new(),
            organization_id,
            experiment_id,
            parent: None,
            agent: agent.into(),
            platform: platform.into(),
            platform_handle: None,
            status: SessionStatus::Working,
            note: None,
            cleanup: None,
            created_at,
            updated_at: created_at,
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StopSession {
    /// Shown as "Stopped by NAME".
    pub by: String,
}

/// The result of one relay step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StepResult {
    pub status: SessionStatus,
    pub events: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[schema(value_type = Object)]
    pub input_schema: serde_json::Value,
}

/// A call of one Antilog tool from an agent that is not in a platform session, such as `antilog mcp`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolCall {
    /// The session the calls belong to; the first call without one opens it.
    pub session: Option<SessionId>,
    /// Unique per call; sending a call again with the same id changes nothing twice.
    pub call: Option<String>,
    #[schema(value_type = Object)]
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolResult {
    pub session: SessionId,
    #[schema(value_type = Object)]
    pub answer: serde_json::Value,
    /// A measurement that was accepted: how it ended, in words. Never a holdout value.
    pub followup: Option<String>,
}
