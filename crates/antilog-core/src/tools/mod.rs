mod setup;

use std::future::Future;

use antilog_types::{MemberId, OrganizationId, SessionId};
use serde_json::{Value, json};

use crate::deps::Deps;
use crate::store::{SessionStore, StoreError};

pub(crate) use setup::{Card, ask, own_name, waiting};

/// A tool an agent may call: its name, what it does and the shape of its input.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub schema: Value,
}

/// Who calls: a session, and the member who started it.
#[derive(Debug, Clone)]
pub struct Caller {
    pub organization: OrganizationId,
    pub session: SessionId,
    pub actor: MemberId,
    /// Unique per call in a session; a repeated call changes nothing twice.
    pub call: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolError(pub String);

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl ToolError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl From<StoreError> for ToolError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::NotFound => Self::new("not found in this organization: check the ids"),
            StoreError::Invariant(rule) => Self::new(format!("refused: {rule}")),
            _ => Self::new("the store is unavailable: retry in a minute"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolReply {
    pub answer: Value,
}

impl ToolReply {
    pub fn error(error: ToolError) -> Self {
        Self {
            answer: json!({ "error": error.0 }),
        }
    }
}

/// What serves tool calls: the MCP endpoint of a session.
pub trait ToolHost: Send + Sync {
    fn call(
        &self,
        caller: &Caller,
        name: &str,
        input: Value,
    ) -> impl Future<Output = ToolReply> + Send;
}

pub struct Toolbox<'a, D: Deps>(pub &'a D);

/// Records one ended call of a session's tool in its conversation. A failed write never fails the call.
pub(crate) async fn record_call<D: Deps>(
    deps: &D,
    caller: &Caller,
    name: &str,
    input: &Value,
    output: &str,
    failed: bool,
) {
    if let Ok(Some(session)) = deps
        .store()
        .session(caller.organization, caller.session)
        .await
    {
        let _ = crate::session::record_tool(
            deps,
            &session,
            format!("tool:{}", caller.call),
            name,
            antilog_types::ToolRecord {
                label: name.to_owned(),
                status: if failed {
                    antilog_types::ToolStatus::Failed
                } else {
                    antilog_types::ToolStatus::Done
                },
                input: Some(input.to_string()),
                output: Some(output.to_owned()),
            },
        )
        .await;
    }
}

impl<D: Deps> ToolHost for Toolbox<'_, D> {
    async fn call(&self, caller: &Caller, name: &str, input: Value) -> ToolReply {
        let shown = input.clone();
        let result = match name {
            "attach_repo" => setup::attach_repo(self.0, caller, input).await,
            "request_credential" => setup::request_credential(self.0, caller, input).await,
            "set_environment" => setup::set_environment(self.0, caller, input).await,
            "request_budget" => setup::request_budget(self.0, caller, input).await,
            other => Err(ToolError::new(format!(
                "no tool named {other}: the tools are {}",
                definitions()
                    .iter()
                    .map(|t| t.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        };
        let reply = match result {
            Ok(answer) => ToolReply { answer },
            Err(error) => ToolReply::error(error),
        };
        // The conversation keeps a short, cleaned record of the call once it has ended.
        record_call(
            self.0,
            caller,
            name,
            &shown,
            &reply.answer.to_string(),
            reply.answer.get("error").is_some(),
        )
        .await;
        reply
    }
}

pub(crate) type Done = Result<Value, ToolError>;

pub(crate) fn input<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, ToolError> {
    serde_json::from_value(value).map_err(|error| {
        ToolError::new(format!(
            "the input is wrong: {error}. Check the tool's input fields"
        ))
    })
}

fn tool(name: &str, description: &str, schema: Value) -> ToolDef {
    ToolDef {
        name: name.to_owned(),
        description: description.to_owned(),
        schema,
    }
}

/// The tools a session's agent is given.
pub fn definitions() -> Vec<ToolDef> {
    vec![
        tool(
            "attach_repo",
            "Ask to attach a repo (owner/name) to this session. A person approves.",
            json!({"type":"object","properties":{"repo":{"type":"string"}},"required":["repo"]}),
        ),
        tool(
            "request_credential",
            "Ask for a read-only credential for a connector. A person enters the value on the card; you never see it.",
            json!({"type":"object","properties":{"connector":{"type":"string"},"reason":{"type":"string"},"scope":{"type":"string"}},"required":["connector","reason","scope"]}),
        ),
        tool(
            "set_environment",
            "Propose a change to the environment this session runs on; a person approves it. Name only what changes: network (none, limited or full), add_hosts and remove_hosts (exact host names, which apply at the limited level), set_variables (name to value, plain text the agent can read, never a secret) and unset_variables, setup_script (the whole new script), add_repos and remove_repos (owner/name, default repos). The network applies within seconds of approval; the variables reach new commands and the setup script runs once now. The environment is shared: other sessions on it get the change when they start. Names ANTILOG_* and proxy variables are refused.",
            json!({"type":"object","properties":{
                "network":{"type":"string","enum":["none","limited","full"]},
                "add_hosts":{"type":"array","items":{"type":"string"}},
                "remove_hosts":{"type":"array","items":{"type":"string"}},
                "set_variables":{"type":"object","additionalProperties":{"type":"string"}},
                "unset_variables":{"type":"array","items":{"type":"string"}},
                "setup_script":{"type":"string"},
                "add_repos":{"type":"array","items":{"type":"string"}},
                "remove_repos":{"type":"array","items":{"type":"string"}}}}),
        ),
        tool(
            "request_budget",
            "Ask to raise this session's budget, in US cents, with a reason.",
            json!({"type":"object","properties":{"amount_cents":{"type":"integer"},"reason":{"type":"string"}},"required":["amount_cents","reason"]}),
        ),
    ]
}
