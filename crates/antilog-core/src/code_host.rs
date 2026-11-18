use std::future::Future;

use antilog_types::judge::CommitRef;

#[derive(Debug, thiserror::Error)]
pub enum CodeHostError {
    #[error("the code host cannot read {0}")]
    Unreadable(String),
    #[error("code host unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// The code host, read only. Which files a trial changes comes from here, never from the agent.
pub trait CodeHost: Send + Sync {
    /// `(repo, path)` for every file these commits change against their parents.
    fn changed_files(
        &self,
        commits: &[CommitRef],
    ) -> impl Future<Output = Result<Vec<(String, String)>, CodeHostError>> + Send;
}
