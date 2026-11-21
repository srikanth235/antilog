use std::future::Future;

use antilog_types::{
    ChangeMark, ChangesReport, CommitChanges, OrganizationId, RepoChanges, SessionId,
};

use super::StoreError;

/// What a session's machine changed in each repo, as the runner last reported it.
pub trait ChangesStore {
    /// Replaces the session's report: every repo in `report` is kept as given and a repo no
    /// longer in it is dropped. `NotFound` for a session that is not in the organization.
    fn put_changes(
        &self,
        organization: OrganizationId,
        session: SessionId,
        report: &ChangesReport,
        at: i64,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// One entry per repo, in the order the repos were reported; empty before the first report.
    fn changes(
        &self,
        organization: OrganizationId,
        session: SessionId,
    ) -> impl Future<Output = Result<Vec<RepoChanges>, StoreError>> + Send;

    /// The diff of one commit of `repo`, or of the first repo that has the commit when `repo` is
    /// `None`.
    fn commit_changes(
        &self,
        organization: OrganizationId,
        session: SessionId,
        repo: Option<&str>,
        sha: &str,
    ) -> impl Future<Output = Result<Option<CommitChanges>, StoreError>> + Send;

    /// Sessions reported at or after `since` (milliseconds), oldest first, for the live stream.
    fn changes_marks(
        &self,
        organization: OrganizationId,
        since: i64,
        limit: u32,
    ) -> impl Future<Output = Result<Vec<ChangeMark>, StoreError>> + Send;
}
