use std::future::Future;

use antilog_types::OrganizationId;
use antilog_types::judge::{ExperimentId, PullRequestState, TrialId};

use super::StoreError;

/// One pull request Antilog opened for a win, and the code host's last word on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestRow {
    pub organization: OrganizationId,
    pub repo: String,
    pub number: u64,
    pub experiment: ExperimentId,
    pub version: u32,
    pub trial: TrialId,
    pub url: String,
    pub state: PullRequestState,
}

/// The pull requests Antilog opened, so the code host's events find the win they belong to.
pub trait CodeHostStore {
    /// `Conflict` when one of them is already recorded.
    fn put_pull_requests(
        &self,
        organization: OrganizationId,
        rows: &[PullRequestRow],
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// By the code host's own name and number, when exactly one organization opened it: a
    /// webhook names no organization.
    fn pull_request(
        &self,
        repo: &str,
        number: u64,
    ) -> impl Future<Output = Result<Option<PullRequestRow>, StoreError>> + Send;

    /// In the order they were opened.
    fn pull_requests_of(
        &self,
        organization: OrganizationId,
        trial: TrialId,
    ) -> impl Future<Output = Result<Vec<PullRequestRow>, StoreError>> + Send;

    fn set_pull_request_state(
        &self,
        organization: OrganizationId,
        repo: &str,
        number: u64,
        state: PullRequestState,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Whether this delivery was already handled.
    fn delivery_seen(
        &self,
        organization: OrganizationId,
        delivery: &str,
    ) -> impl Future<Output = Result<bool, StoreError>> + Send;

    /// Marks a delivery handled. Marking it twice is not an error.
    fn put_delivery(
        &self,
        organization: OrganizationId,
        delivery: &str,
        at: i64,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;
}
