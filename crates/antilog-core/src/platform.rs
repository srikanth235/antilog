use std::future::Future;
use std::time::Duration;

use antilog_types::judge::{CommitRef, DatasetRef, Guardrail, ProtectedFile, SealHash};
use serde_json::Value;

/// A tool the platform lets the agent call; the answer comes back through `answer_tool`.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub schema: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionSpec {
    pub agent: String,
    pub prompt: String,
    pub repos: Vec<String>,
    pub environment: Option<String>,
    pub tools: Vec<ToolDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionHandle(pub String);

#[derive(Debug, Clone, PartialEq)]
pub enum PlatformEvent {
    Message { text: String },
    ToolCall { id: String, name: String, input: Value },
    Usage { cents: i64, tokens: i64 },
    /// The agent waits for a person.
    Idle,
    Ended { failed: Option<String> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionEvents {
    pub events: Vec<PlatformEvent>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum Data<'a> {
    Evaluation(&'a DatasetRef),
    Holdout(&'a DatasetRef),
}

/// One fresh judge sandbox: the base and the trial run interleaved `repeats` times.
/// Protected files come from the seal, never from the trial. `seal` is `None` only
/// for the seal checks, before there is a seal.
#[derive(Debug, Clone, Copy)]
pub struct PairedRun<'a> {
    pub seal: Option<&'a SealHash>,
    pub command: &'a str,
    pub files: &'a [ProtectedFile],
    pub data: Data<'a>,
    pub base: &'a [CommitRef],
    pub trial: &'a [CommitRef],
    pub repeats: u32,
    pub guardrails: &'a [Guardrail],
}

/// `(base, trial)` per repeat.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pairs {
    pub metric: Vec<(f64, f64)>,
    pub guardrails: Vec<(String, Vec<(f64, f64)>)>,
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("unknown session")]
    UnknownSession,
    #[error("the run failed: {0}")]
    Failed(String),
    #[error("platform unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// The one trait every agent platform adapter implements. It runs the agent's loop
/// and the judge's sandboxes; the server relays, it runs no loop.
pub trait Platform: Send + Sync {
    fn start_session(
        &self,
        spec: &SessionSpec,
    ) -> impl Future<Output = Result<SessionHandle, PlatformError>> + Send;

    fn send_input(
        &self,
        session: &SessionHandle,
        text: &str,
    ) -> impl Future<Output = Result<(), PlatformError>> + Send;

    /// Events after `cursor`. An adapter may wait up to `wait` for one; empty means none came.
    fn session_events(
        &self,
        session: &SessionHandle,
        cursor: Option<&str>,
        wait: Duration,
    ) -> impl Future<Output = Result<SessionEvents, PlatformError>> + Send;

    /// Answers a tool call once; the same call id again changes nothing.
    fn answer_tool(
        &self,
        session: &SessionHandle,
        call: &str,
        result: &Value,
    ) -> impl Future<Output = Result<(), PlatformError>> + Send;

    fn stop(&self, session: &SessionHandle)
    -> impl Future<Output = Result<(), PlatformError>> + Send;

    /// Removes the session and its sandbox; deleting twice is fine.
    fn delete(
        &self,
        session: &SessionHandle,
    ) -> impl Future<Output = Result<(), PlatformError>> + Send;

    /// A judge sandbox. It returns only the numbers the experiment names.
    fn run_paired(
        &self,
        run: &PairedRun<'_>,
    ) -> impl Future<Output = Result<Pairs, PlatformError>> + Send;
}
