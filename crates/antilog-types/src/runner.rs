//! What the server and an `antilog runner` say to each other (`docs/V0-contracts.md`). These
//! routes are not in the public OpenAPI, so the types carry no schema.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::id::text_enum;
use crate::{ContextUse, OrganizationId, SessionId, ToolRecord};

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
pub struct SkillDownload {
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
    pub agent: String,
    /// `provider/model`, as OpenCode names it.
    pub model: String,
    pub repos: Vec<RepoSpec>,
    /// The environment version the session runs on; its variables are the machine's.
    pub environment: EnvironmentSpec,
    pub skills: Vec<SkillDownload>,
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
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Command {
    /// A person's message for the agent.
    Message { text: String },
    /// A repo a person approved: clone it into the live machine if it is not there yet.
    AttachRepo { repo: RepoSpec },
    /// The session moved to another version of its environment (or another environment): the
    /// machine applies the variables for new processes and runs the setup script if it changed.
    /// The network needs nothing here, the proxy reads it from the session.
    ApplyEnvironment { environment: Box<EnvironmentSpec> },
    /// A person's answer to a permission the agent asked.
    Approval { permission: String, reply: Reply },
    /// Interrupt the agent's current turn.
    Abort,
    /// Summarise the conversation so far to free the window (OpenCode's compact).
    Compact,
    /// Export, stop OpenCode and let the machine go.
    Release { outcome: ReleaseOutcome },
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
    /// This is the runner's last poll: it is leaving, so the server lets its organization lock go.
    #[serde(default)]
    pub exiting: bool,
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
        /// For a tool call: what it did. The server cleans it again before storing.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool: Option<ToolRecord>,
    },
    /// A note from Antilog for the conversation, such as a failed setup script. It is shown as
    /// a note in the transcript; `key` is unique within the session.
    Note { key: String, text: String },
    /// The agent asked for a permission: it becomes an approval card.
    Permission {
        id: String,
        tool: String,
        detail: String,
    },
    /// The model's provider refused or failed a call; shown as an error in the transcript.
    Error { key: String, text: String },
    /// The agent is busy again or has stopped.
    Status { busy: bool },
    /// A model call finished: how much of the window it filled. Only the latest is kept.
    Context { context: ContextUse },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Items {
    pub items: Vec<Item>,
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
    /// Why the machine failed, cut short and with credentials taken out; the session's note and
    /// the transcript say it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// What a session asked for when it started, kept until a runner claims it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Seed {
    pub repos: Vec<String>,
    /// The stored environment the session started on; `None` is the built-in default.
    pub environment: Option<String>,
    /// The version of that environment, pinned when the session started.
    #[serde(default)]
    pub environment_version: Option<u32>,
}

/// A session's machine row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    pub session: SessionId,
    pub organization: OrganizationId,
    pub agent: String,
    pub state: MachineState,
    /// The runner that holds the lease.
    pub holder: Option<String>,
    pub seed: Seed,
    /// An export of an earlier run is stored.
    pub resume: bool,
}

text_enum!(
    /// How much of the network a session's machine may reach (docs/V0.md, Environments). `None`
    /// is no outbound traffic except the platform hosts (model gateway, MCP, code host); `Limited` is `allowed_hosts` plus package
    /// registries when allowed; `Full` is any host except private, loopback and metadata addresses.
    NetworkLevel {
        None => "none",
        Limited => "limited",
        Full => "full",
    }
);

/// One version of an environment, as a runner receives it at claim. Every change to an
/// environment makes a new version. It carries no network secret: a value never reaches a runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentSpec {
    pub id: String,
    pub version: u32,
    pub network: NetworkLevel,
    pub allowed_hosts: Vec<String>,
    pub allow_package_managers: bool,
    /// Plain variables, readable by the agent; the screen says so.
    pub variables: BTreeMap<String, String>,
    /// Runs as root in a fresh container after the session's repos are cloned and before the
    /// agent starts, from the folder that holds the checkouts; empty for none.
    pub setup_script: String,
    /// The environment's default repos (`owner/repo`). The claim's `repos` already include them.
    #[serde(default)]
    pub repos: Vec<String>,
    pub cpus: u32,
    pub memory_mb: u32,
}

impl Default for EnvironmentSpec {
    /// The built-in environment: limited network with package registries, 2 vCPU, 8 GB.
    fn default() -> Self {
        Self {
            id: "default".into(),
            version: 1,
            network: NetworkLevel::Limited,
            allowed_hosts: Vec::new(),
            allow_package_managers: true,
            variables: BTreeMap::new(),
            setup_script: String::new(),
            repos: Vec::new(),
            cpus: 2,
            memory_mb: 8192,
        }
    }
}
