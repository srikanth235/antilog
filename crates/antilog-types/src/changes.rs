//! What a session's machine has changed in each repo, against the commit it was cloned at. The
//! runner reports it at the end of every agent turn; the server keeps the latest report per
//! repo and serves it to the Changes panel.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::SessionId;
use crate::id::text_enum;

/// The most patch text kept for one diff, in bytes. A file whose patch would pass it is kept
/// without its patch and marked too large.
pub const DIFF_BYTE_CAP: usize = 2 * 1024 * 1024;

/// The most commits listed per repo; their diffs share one `DIFF_BYTE_CAP`.
pub const COMMIT_CAP: usize = 50;

text_enum!(
    /// How a file differs from the base.
    FileStatus {
        Added => "added",
        Modified => "modified",
        Deleted => "deleted",
        Renamed => "renamed",
    }
);

/// One file of a diff. `patch` is the unified diff text from its first `@@` line on, with
/// three lines of context; it is empty for a binary file and for one that is too large.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    /// The path before a rename; `null` otherwise.
    pub old_path: Option<String>,
    pub status: FileStatus,
    pub additions: u32,
    pub deletions: u32,
    pub binary: bool,
    pub too_large: bool,
    pub patch: String,
}

/// One commit made since the base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommitInfo {
    pub sha: String,
    pub subject: String,
    /// When it was committed, epoch milliseconds.
    pub time: i64,
}

/// The diff of one commit against its parent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommitDiff {
    pub sha: String,
    pub files: Vec<FileChange>,
}

/// One repo's state, as the runner reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoReport {
    pub repo: String,
    pub base_branch: String,
    pub head_branch: String,
    pub base_sha: String,
    pub head_sha: String,
    /// Newest first.
    pub commits: Vec<CommitInfo>,
    /// Base against the working tree, uncommitted work included, ignored files left out.
    pub files: Vec<FileChange>,
    pub commit_diffs: Vec<CommitDiff>,
}

/// Everything a runner reports at the end of a turn: one entry per repo in the machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangesReport {
    pub repos: Vec<RepoReport>,
}

/// One repo's changes, as the Changes panel reads them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RepoChanges {
    pub repo: String,
    pub base_branch: String,
    pub head_branch: String,
    pub base_sha: String,
    pub head_sha: String,
    /// Newest first. Each has its own diff at `GET /sessions/{id}/changes/commits/{sha}`.
    pub commits: Vec<CommitInfo>,
    /// All changes: the base against the working tree.
    pub files: Vec<FileChange>,
    /// When the runner reported it, epoch milliseconds.
    pub reported_at: i64,
}

/// The latest report for each repo of the session; empty before the first one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionChanges {
    pub repos: Vec<RepoChanges>,
}

/// The files of one commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommitChanges {
    pub sha: String,
    pub files: Vec<FileChange>,
}

/// A session's changes were reported. Not a conversation item; see `LiveEvent::Changes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeMark {
    pub session: SessionId,
    pub reported_at: i64,
}

fn cap_files(files: &mut [FileChange], budget: &mut usize) {
    for file in files {
        if file.patch.len() > *budget {
            file.patch.clear();
            file.too_large = !file.binary;
        } else {
            *budget -= file.patch.len();
        }
    }
}

impl ChangesReport {
    /// Enforces the caps whoever sent the report: at most `COMMIT_CAP` commits per repo, and no
    /// more than `DIFF_BYTE_CAP` bytes of patch in a repo's all-changes diff and in its commit
    /// diffs together. A file past the cap keeps its counts and loses its patch.
    #[must_use]
    pub fn capped(mut self) -> Self {
        for repo in &mut self.repos {
            repo.commits.truncate(COMMIT_CAP);
            let kept: Vec<&str> = repo.commits.iter().map(|c| c.sha.as_str()).collect();
            repo.commit_diffs
                .retain(|diff| kept.contains(&diff.sha.as_str()));
            let mut budget = DIFF_BYTE_CAP;
            cap_files(&mut repo.files, &mut budget);
            let mut budget = DIFF_BYTE_CAP;
            for diff in &mut repo.commit_diffs {
                cap_files(&mut diff.files, &mut budget);
            }
        }
        self
    }
}
