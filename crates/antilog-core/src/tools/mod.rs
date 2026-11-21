mod setup;

use std::future::Future;

use antilog_types::{MemberId, OrganizationId, SessionId};
use serde_json::{Value, json};

use crate::deps::Deps;
use crate::store::{SessionStore, StoreError};

pub(crate) use setup::{Card, ask, waiting};

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

impl<D: Deps> ToolHost for Toolbox<'_, D> {
    async fn call(&self, caller: &Caller, name: &str, input: Value) -> ToolReply {
        // The conversation keeps the name of the tool, never its input or its answer.
        if let Ok(Some(session)) = self
            .0
            .store()
            .session(caller.organization, caller.session)
            .await
        {
            let _ = crate::session::record(
                self.0,
                &session,
                format!("tool:{}", caller.call),
                antilog_types::EventKind::Tool,
                None,
                name,
            )
            .await;
        }
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
        match result {
            Ok(answer) => ToolReply { answer },
            Err(error) => ToolReply::error(error),
        }
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
            "Propose a change to the session's environment: image or setup script, size, network.",
            json!({"type":"object","properties":{"image":{"type":"string"},"setup_script":{"type":"string"},"size":{"type":"string"},"network":{"type":"string"}}}),
        ),
        tool(
            "request_budget",
            "Ask to raise this session's budget, in US cents, with a reason.",
            json!({"type":"object","properties":{"amount_cents":{"type":"integer"},"reason":{"type":"string"}},"required":["amount_cents","reason"]}),
        ),
    ]
}
