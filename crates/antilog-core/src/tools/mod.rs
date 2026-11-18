mod judge;
mod setup;

use std::future::Future;

use antilog_types::judge::Trial;
use antilog_types::{MemberId, OrganizationId, SessionId};
use serde_json::{Value, json};

use crate::deps::Deps;
use crate::platform::ToolDef;
use crate::store::StoreError;

pub(crate) use judge::issue_receipt;
pub use judge::seal_approved;

/// Who calls: a session of an experiment, and the person behind it when there is one.
#[derive(Debug, Clone)]
pub struct Caller {
    pub organization: OrganizationId,
    pub experiment: antilog_types::ExperimentId,
    pub session: SessionId,
    pub actor: Option<MemberId>,
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

impl From<crate::store::judge::JudgeStoreError> for ToolError {
    fn from(_: crate::store::judge::JudgeStoreError) -> Self {
        Self::new("the judge's store is unavailable: retry in a minute")
    }
}

/// A measurement that was accepted and is still to run; its outcome arrives as a message.
#[derive(Debug, Clone)]
pub struct Ticket {
    pub trial: Trial,
}

#[derive(Debug, Clone)]
pub struct ToolReply {
    pub answer: Value,
    pub ticket: Option<Ticket>,
}

impl ToolReply {
    pub fn error(error: ToolError) -> Self {
        Self {
            answer: json!({ "error": error.0 }),
            ticket: None,
        }
    }
}

/// What serves tool calls: the relay for a session, the API for `antilog mcp`.
pub trait ToolHost: Send + Sync {
    fn call(
        &self,
        caller: &Caller,
        name: &str,
        input: Value,
    ) -> impl Future<Output = ToolReply> + Send;

    /// Runs an accepted measurement and says in words how it ended. Never a holdout value.
    fn finish(&self, caller: &Caller, ticket: &Ticket) -> impl Future<Output = String> + Send;
}

/// The host of a session that is stopping: nothing new starts.
pub struct NoTools;

impl ToolHost for NoTools {
    async fn call(&self, _: &Caller, _: &str, _: Value) -> ToolReply {
        ToolReply::error(ToolError::new(
            "the session is stopping: nothing new starts",
        ))
    }

    async fn finish(&self, _: &Caller, _: &Ticket) -> String {
        "The session is stopping.".to_owned()
    }
}

pub struct Toolbox<'a, D: Deps>(pub &'a D);

impl<D: Deps> ToolHost for Toolbox<'_, D> {
    async fn call(&self, caller: &Caller, name: &str, input: Value) -> ToolReply {
        let result = match name {
            "seal" => judge::seal(self.0, caller, input).await,
            "measure" => judge::measure(self.0, caller, input).await,
            "history" => judge::history(self.0, caller, input).await,
            "deliver" => judge::deliver(self.0, caller, input).await,
            "spawn" => setup::spawn(self.0, caller, input).await,
            "attach_repo" => setup::attach_repo(self.0, caller, input).await,
            "request_credential" => setup::request_credential(self.0, caller, input).await,
            "set_environment" => setup::set_environment(self.0, caller, input).await,
            "request_budget" => setup::request_budget(self.0, caller, input).await,
            other => Err(ToolError::new(format!(
                "no tool named {other}: the tools are {}",
                definitions(false)
                    .iter()
                    .map(|t| t.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        };
        match result {
            Ok((answer, ticket)) => ToolReply { answer, ticket },
            Err(error) => ToolReply::error(error),
        }
    }

    async fn finish(&self, caller: &Caller, ticket: &Ticket) -> String {
        judge::finish(self.0, caller, ticket).await
    }
}

pub(crate) type Done = Result<(Value, Option<Ticket>), ToolError>;

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

/// One definition, served to a session and by `antilog mcp`. A spawned session has no `spawn`.
pub fn definitions(spawned: bool) -> Vec<ToolDef> {
    let commits = json!({"type":"array","items":{"type":"object","properties":{"repo":{"type":"string"},"sha":{"type":"string"}},"required":["repo","sha"]}});
    let mut tools = vec![
        tool(
            "seal",
            "Propose the experiment's definition (metric, evaluation, pinned commits, datasets, repeats). Runs the seal checks and puts a Seal experiment card in front of a person. Nothing is measured until it is sealed.",
            json!({"type":"object","properties":{
                "definition":{"type":"object","description":"The experiment definition."},
                "files":{"type":"array","description":"Protected files with their content hashes."},
                "cheats":{"type":"array","description":"Planted cheats the evaluation must reject: {name, commits, changed}."}
            },"required":["definition"]}),
        ),
        tool(
            "measure",
            "Measure a change. Returns a ticket at once; the judge's outcome arrives later as a message. Give one commit per repo changed and the Kept trial it started from (none: the baseline).",
            json!({"type":"object","properties":{
                "commits":commits,
                "parent":{"type":"string","description":"A Kept trial id from history."},
                "hypothesis":{"type":"string","description":"One line: what you think this changes."},
                "version":{"type":"integer","description":"Defaults to the latest sealed version."}
            },"required":["commits","hypothesis"]}),
        ),
        tool(
            "history",
            "Every trial of the experiment across all sessions, failures included. Read it before you try something.",
            json!({"type":"object","properties":{
                "session":{"type":"string"},"outcome":{"type":"string"},
                "limit":{"type":"integer"},"offset":{"type":"integer"}
            }}),
        ),
        tool(
            "deliver",
            "Open pull requests for a Kept trial that is the win. A person may be asked first.",
            json!({"type":"object","properties":{"trial":{"type":"string"}},"required":["trial"]}),
        ),
        tool(
            "attach_repo",
            "Ask to attach a repo (owner/name) to this session. A person approves.",
            json!({"type":"object","properties":{"repo":{"type":"string"}},"required":["repo"]}),
        ),
        tool(
            "request_credential",
            "Ask for a read-only credential for a connector. A person enters the value on the card; you never see it. The holdout is never in scope.",
            json!({"type":"object","properties":{"connector":{"type":"string"},"reason":{"type":"string"},"scope":{"type":"string"}},"required":["connector","reason","scope"]}),
        ),
        tool(
            "set_environment",
            "Propose a change to the session's environment: image or setup script, size, network.",
            json!({"type":"object","properties":{"image":{"type":"string"},"setup_script":{"type":"string"},"size":{"type":"string"},"network":{"type":"string"}}}),
        ),
        tool(
            "request_budget",
            "Ask to raise the experiment's budget, in US cents, with a reason.",
            json!({"type":"object","properties":{"amount_cents":{"type":"integer"},"reason":{"type":"string"}},"required":["amount_cents","reason"]}),
        ),
    ];
    if !spawned {
        tools.insert(
            4,
            tool(
                "spawn",
                "Start another session in this experiment on a named agent, with what has been tried so far. Costs the experiment's budget; prefer the lower-cost agent.",
                json!({"type":"object","properties":{"agent":{"type":"string"},"prompt":{"type":"string"},"environment":{"type":"string"}},"required":["agent","prompt"]}),
            ),
        );
    }
    tools
}
