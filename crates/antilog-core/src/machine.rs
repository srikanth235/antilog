use std::future::Future;

use antilog_types::SessionId;
use antilog_types::judge::{CommitRef, DatasetRef, ProtectedFile};

/// The data the command reads.
#[derive(Debug, Clone, Copy)]
pub enum Data<'a> {
    Evaluation(&'a DatasetRef),
    Holdout(&'a DatasetRef),
}

/// One command to run in a session's machine, on a fixed checkout.
#[derive(Debug, Clone, Copy)]
pub struct Run<'a> {
    pub session: SessionId,
    pub command: &'a str,
    /// What the machine checks out first: one commit per repo.
    pub commits: &'a [CommitRef],
    /// Restored from the seal after the checkout, never taken from the commits.
    pub files: &'a [ProtectedFile],
    pub data: Data<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub exit: i32,
    pub output: String,
}

#[derive(Debug, thiserror::Error)]
pub enum MachineError {
    #[error("the session has no machine")]
    NoMachine,
    #[error("the machine is unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// The machine a session's agent works in. The judge runs the sealed command there; the number
/// it reads comes from the command's output, never from what the agent says.
pub trait SessionMachine: Send + Sync {
    fn run(&self, run: &Run<'_>) -> impl Future<Output = Result<Output, MachineError>> + Send;
}
