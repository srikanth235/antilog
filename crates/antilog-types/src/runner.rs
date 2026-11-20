//! What the server and an `antilog runner` say to each other (`docs/V0-contracts.md`). These
//! routes are not in the public OpenAPI, so the types carry no schema.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::id::text_enum;
use crate::judge::{CommitRef, DatasetRef, ProtectedFile};
use crate::{ExperimentId, OrganizationId, SessionId};

text_enum!(
    /// Where a session's machine is. A claim holds a lease; a lapsed lease is `Queued` again.
    /// `Idle` is held with nothing to do, `Released` is let go with its export saved.
    MachineState {
        Queued => "queued",
        Claimed => "claimed",
        Running => "running",
        Idle => "idle",
        Released => "released",
        Done => "done",
        Failed => "failed",
    }
);

impl MachineState {
    /// A runner holds the session and must keep renewing its lease.
    pub const fn is_held(self) -> bool {
        matches!(self, Self::Claimed | Self::Running | Self::Idle)
    }
}

/// One repository to clone into the machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoSpec {
    pub name: String,
    pub url: String,
    /// A short-lived clone credential, when the code host needs one.
    pub token: Option<String>,
}

/// A skill pack version to unpack into the skills folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRef {
    pub name: String,
    pub version: String,
    /// Where to download it, relative to the server.
    pub path: String,
}

/// Everything a runner needs to start a session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSpec {
    pub session: SessionId,
    pub organization: OrganizationId,
    pub experiment: ExperimentId,
    pub agent: String,
    /// `provider/model`, as OpenCode names it.
    pub model: String,
    pub repos: Vec<RepoSpec>,
    pub environment: Option<String>,
    /// Plain variables of the environment.
    pub variables: BTreeMap<String, String>,
    pub skills: Vec<SkillRef>,
    /// The machine-wide OpenCode config, generated here.
    pub opencode_config: serde_json::Value,
    pub model_token: String,
    pub mcp_token: String,
    /// Authenticates this runner's calls about this session.
    pub runner_token: String,
    /// An export of an earlier run is waiting at `GET /runner/sessions/{id}/export`.
    pub resume: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reply {
    Once,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSpec {
    pub holdout: bool,
    pub dataset: DatasetRef,
}

/// A `SessionMachine` command: check out the commits, restore the protected files, run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSpec {
    pub command: String,
    pub commits: Vec<CommitRef>,
    pub files: Vec<ProtectedFile>,
    pub data: DataSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Command {
    /// A person's message for the agent.
    Message { text: String },
    /// A person's answer to a permission the agent asked.
    Approval { permission: String, reply: Reply },
    /// Interrupt the agent's current turn.
    Abort,
    /// Export, stop OpenCode and let the machine go.
    Release { outcome: ReleaseOutcome },
    /// Run the judge's sealed command.
    Run { run: RunSpec },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    pub id: Uuid,
    pub session: SessionId,
    pub command: Command,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollRequest {
    /// A name for this runner; a lease is held by it.
    pub runner: String,
    /// How many more sessions it will take.
    pub capacity: u32,
    /// Commands it has finished since the last poll.
    #[serde(default)]
    pub done: Vec<Uuid>,
    /// The sessions it is running now; any it names that it no longer holds come back as lost.
    #[serde(default)]
    pub held: Vec<SessionId>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollResponse {
    pub claims: Vec<SessionSpec>,
    pub commands: Vec<Envelope>,
    /// Sessions the runner held whose lease had lapsed or that were released elsewhere:
    /// it must stop them.
    pub lost: Vec<SessionId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Agent,
    Tool,
}

/// One item of OpenCode's conversation. `key` is unique within the session (its message id and
/// part id), so sending an item again records nothing twice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Item {
    Words {
        key: String,
        kind: ItemKind,
        text: String,
    },
    /// The agent asked for a permission: it becomes an approval card.
    Permission {
        id: String,
        tool: String,
        detail: String,
    },
    /// The agent is busy again or has stopped.
    Status { busy: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Items {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    pub command: Uuid,
    pub exit: i32,
    pub output: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseOutcome {
    /// Hold the work for the next message: the session is idle.
    Idle,
    Done,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub outcome: ReleaseOutcome,
    /// What `opencode export` wrote, so a later claim can import it.
    pub export: Option<String>,
}

/// What a session asked for when it started, kept until a runner claims it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Seed {
    pub repos: Vec<String>,
    pub environment: Option<String>,
}

/// A session's machine row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    pub session: SessionId,
    pub organization: OrganizationId,
    pub experiment: ExperimentId,
    pub agent: String,
    pub state: MachineState,
    /// The runner that holds the lease.
    pub holder: Option<String>,
    pub seed: Seed,
    /// An export of an earlier run is stored.
    pub resume: bool,
}
