use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::id::text_enum;
use crate::{EnvironmentSpec, MemberId, NetworkLevel, OrganizationId, ProjectId, SessionId};

text_enum!(
    SessionStatus {
        Queued => "queued",
        Working => "working",
        WaitingForYou => "waiting_for_you",
        Done => "done",
        Stopped => "stopped",
        Failed => "failed",
    }
);

impl SessionStatus {
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Queued | Self::Working | Self::WaitingForYou)
    }
}

/// One agent's work in a project: a chat that runs in a machine of its own. The repos,
/// the environment version and the model are fixed when it starts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Session {
    pub id: SessionId,
    pub organization_id: OrganizationId,
    pub project_id: ProjectId,
    pub title: String,
    /// The member who started it; its calls and cards are theirs.
    pub created_by: MemberId,
    pub agent: String,
    /// `provider/model`, as the harness names it.
    pub model: String,
    pub repos: Vec<String>,
    /// The stored environment it runs on; `None` is the built-in default.
    pub environment: Option<String>,
    /// The version of that environment, pinned when the session started.
    pub environment_version: Option<u32>,
    /// What this session alone may spend, in US cents. `None`: only the organization's monthly cap.
    pub budget_cap_cents: Option<i64>,
    /// The composer's Mode: false asks a person for every card, true lets policy approve.
    pub follow_policy: bool,
    pub status: SessionStatus,
    /// Why it ended, such as "Stopped by Ada", or what went wrong when it failed.
    pub note: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// What a person gives to start a session.
#[derive(Debug, Clone, Default)]
pub struct SessionParams {
    pub title: String,
    pub agent: String,
    pub model: String,
    pub repos: Vec<String>,
    pub environment: Option<String>,
    pub environment_version: Option<u32>,
    pub budget_cap_cents: Option<i64>,
    pub follow_policy: bool,
}

impl Session {
    pub fn new(
        organization_id: OrganizationId,
        project_id: ProjectId,
        created_by: MemberId,
        params: SessionParams,
        created_at: i64,
    ) -> Self {
        Self {
            id: SessionId::new(),
            organization_id,
            project_id,
            title: params.title,
            created_by,
            agent: params.agent,
            model: params.model,
            repos: params.repos,
            environment: params.environment,
            environment_version: params.environment_version,
            budget_cap_cents: params.budget_cap_cents,
            follow_policy: params.follow_policy,
            status: SessionStatus::Queued,
            note: None,
            created_at,
            updated_at: created_at,
        }
    }
}

text_enum!(
    /// Who spoke: a person, an agent, the name of a tool an agent called, or an error the model's
    /// provider or the machine reported.
    EventKind { Person => "person", Agent => "agent", Tool => "tool", Error => "error" }
);

text_enum!(
    /// How a tool call ended. A call still running is not stored: it is recorded when it ends.
    ToolStatus { Done => "done", Failed => "failed" }
);

/// What one tool call did, kept small: a short label (the first line of a command, a file path,
/// the Antilog tool's name), how it ended, and its input and output cut to 4 KB each with
/// anything that looks like a credential taken out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolRecord {
    pub label: String,
    pub status: ToolStatus,
    pub input: Option<String>,
    pub output: Option<String>,
}

/// The longest input or output kept, in characters.
pub const TOOL_TEXT_CAP: usize = 4096;
const LABEL_CAP: usize = 200;
const REDACTED: &str = "[redacted]";
const TOKEN_PREFIXES: [&str; 3] = ["alt_", "als_", "alc_"];

impl ToolRecord {
    /// The call as it may be stored: labels and texts cut, credentials removed. Both the runner
    /// and the server apply it, so a runner that forgets cannot store a secret.
    #[must_use]
    pub fn cleaned(self) -> Self {
        Self {
            label: cut(
                &redact(self.label.lines().next().unwrap_or_default()),
                LABEL_CAP,
            ),
            status: self.status,
            input: self.input.map(|text| cut(&redact(&text), TOOL_TEXT_CAP)),
            output: self.output.map(|text| cut(&redact(&text), TOOL_TEXT_CAP)),
        }
    }
}

fn cut(text: &str, cap: usize) -> String {
    if text.chars().count() <= cap {
        return text.to_owned();
    }
    let mut kept: String = text.chars().take(cap).collect();
    kept.push_str("\n… cut");
    kept
}

/// Takes out an `Authorization` header's value, `Bearer` tokens and Antilog's own token forms.
fn redact(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while at < text.len() {
        let rest = &lower[at..];
        let hide = if rest.starts_with("authorization") {
            // The value runs to the end of the line or the quote that closes the header.
            let end = rest.find(['\n', '"', '\'']).unwrap_or(rest.len());
            Some(("authorization", end))
        } else if let Some(after) = rest.strip_prefix("bearer ") {
            let end = after
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                .map_or(rest.len(), |n| n + 7);
            Some(("bearer", end))
        } else if TOKEN_PREFIXES.iter().any(|p| rest.starts_with(p)) {
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
                .unwrap_or(rest.len());
            Some(("", end))
        } else {
            None
        };
        match hide {
            Some((name, end)) if end > name.len() || name.is_empty() => {
                out.push_str(&text[at..at + name.len()]);
                if !name.is_empty() {
                    out.push_str(if name == "bearer" { " " } else { ": " });
                }
                out.push_str(REDACTED);
                at += end;
            }
            _ => {
                let Some(c) = text[at..].chars().next() else {
                    break;
                };
                out.push(c);
                at += c.len_utf8();
            }
        }
    }
    out
}

/// One item of a session's conversation. A tool call carries what `ToolRecord` keeps, never more:
/// the server stores the words people and agents said and a short, cleaned record of each tool call.
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
    /// What a tool call did; set only when `kind` is `tool`, and absent on calls recorded before it existed.
    pub tool: Option<ToolRecord>,
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
    /// Runs as root in a fresh container after the repos are cloned, before the agent starts;
    /// empty for none. A failing script does not fail the session.
    pub setup_script: String,
    /// Default repos (`owner/repo`): a new session on this environment starts with them.
    pub repos: Vec<String>,
    /// What the proxy adds to HTTPS requests to a host. Never a value (write-only).
    pub network_secrets: Vec<NetworkSecret>,
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
            repos: spec.repos,
            network_secrets: Vec::new(),
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
            repos: self.repos.clone(),
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
    /// `owner/repo`, at most 50.
    #[serde(default)]
    pub repos: Vec<String>,
    /// On an edit, an entry without a `value` keeps the value the same host and header had.
    #[serde(default)]
    pub network_secrets: Vec<NewNetworkSecret>,
    pub cpus: u32,
    pub memory_mb: u32,
}

/// A header the proxy adds to HTTPS requests to `host` from sessions in an environment, so the
/// value never enters the machine. The value is not part of this type: it is write-only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkSecret {
    pub host: String,
    pub header: String,
    /// Put before the value, such as `Bearer`; none when empty or left out.
    #[serde(default)]
    pub prefix: Option<String>,
}

/// A network secret a person writes: the value is sealed at once and never read back.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewNetworkSecret {
    pub host: String,
    pub header: String,
    #[serde(default)]
    pub prefix: Option<String>,
    /// Left out on an edit to keep the stored value.
    #[serde(default)]
    pub value: Option<String>,
}

impl std::fmt::Debug for NewNetworkSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewNetworkSecret")
            .field("host", &self.host)
            .field("header", &self.header)
            .field("value", &self.value.as_ref().map(|_| "[sealed]"))
            .finish()
    }
}

/// What starts a session in a project: `POST /projects/{id}/sessions`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartSession {
    /// Left out, the first line of the prompt.
    #[serde(default)]
    pub title: Option<String>,
    /// Left out or `auto`: the agent Auto chooses.
    #[serde(default)]
    pub agent: Option<String>,
    /// `provider/model`; left out, the server's default model.
    #[serde(default)]
    pub model: Option<String>,
    pub prompt: String,
    #[serde(default)]
    pub repos: Vec<String>,
    pub environment: Option<String>,
    /// This session's own cap in US cents; left out, the project's default, then the organization's.
    #[serde(default)]
    pub budget_cap_cents: Option<i64>,
    /// The composer's Mode: false asks a person for every card, true lets policy approve.
    #[serde(default)]
    pub follow_policy: bool,
}

/// The environment to move a session to: its newest version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionEnvironment {
    /// An environment's id; `default` is the built-in one.
    pub environment: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SendMessage {
    pub text: String,
}
#[cfg(test)]
mod tool_tests {
    use super::*;

    fn call(input: &str) -> ToolRecord {
        ToolRecord {
            label: input.into(),
            status: ToolStatus::Done,
            input: Some(input.into()),
            output: None,
        }
        .cleaned()
    }

    #[test]
    fn credentials_never_reach_the_store() {
        let cleaned = call(
            "curl -H \"Authorization: Bearer alc_0123abcd\" https://x\nuse als_aa11-bb and ALT_zz",
        );
        let input = cleaned.input.expect("input");
        assert!(!input.contains("alc_0123"), "{input}");
        assert!(!input.contains("als_aa11"), "{input}");
        assert!(input.contains("https://x"), "{input}");
        assert_eq!(
            cleaned.label,
            "curl -H \"Authorization: [redacted]\" https://x"
        );
    }

    #[test]
    fn a_label_is_one_short_line_and_a_text_is_cut() {
        let long = "x".repeat(10_000);
        let cleaned = ToolRecord {
            label: format!("first\nsecond {long}"),
            status: ToolStatus::Failed,
            input: None,
            output: Some(long),
        }
        .cleaned();
        assert_eq!(cleaned.label, "first");
        let output = cleaned.output.expect("output");
        assert!(output.chars().count() <= TOOL_TEXT_CAP + 8);
        assert!(output.ends_with("… cut"));
    }
}
