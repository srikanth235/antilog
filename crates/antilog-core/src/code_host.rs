use std::future::Future;

use antilog_types::judge::CommitRef;

#[derive(Debug, thiserror::Error)]
pub enum CodeHostError {
    #[error("the code host cannot read {0}")]
    Unreadable(String),
    #[error("code host unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// A pull request to open: the trial's commit in one repo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestSpec {
    pub repo: String,
    /// The trial's commit in this repo, which the pull request proposes.
    pub sha: String,
    pub title: String,
    /// Carries the receipt's summary and names the linked pull requests in the other repos.
    pub body: String,
}

/// What the code host made of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedPullRequest {
    pub repo: String,
    pub number: u64,
    pub url: String,
}

/// The code host. Which files a trial changes comes from here, never from the agent, and a win
/// leaves through it as a pull request.
pub trait CodeHost: Send + Sync {
    /// `(repo, path)` for every file these commits change against their parents.
    fn changed_files(
        &self,
        commits: &[CommitRef],
    ) -> impl Future<Output = Result<Vec<(String, String)>, CodeHostError>> + Send;

    /// Opens one pull request. The pull request is the agent's own branch proposed against the
    /// default branch; nothing is merged here.
    fn open_pull_request(
        &self,
        spec: &PullRequestSpec,
    ) -> impl Future<Output = Result<OpenedPullRequest, CodeHostError>> + Send;
}
