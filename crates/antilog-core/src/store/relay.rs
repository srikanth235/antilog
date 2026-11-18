use std::future::Future;

use antilog_types::{ApprovalId, OrganizationId, SessionId};

use super::StoreError;

/// What the session relay keeps between steps.
pub trait RelayStore {
    fn relay_cursor(
        &self,
        organization: OrganizationId,
        session: SessionId,
    ) -> impl Future<Output = Result<Option<String>, StoreError>> + Send;

    fn save_relay_cursor(
        &self,
        organization: OrganizationId,
        session: SessionId,
        cursor: &str,
        at: i64,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// What a seal card will seal if a person approves it. Written once.
    fn put_proposal(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
        body: &str,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn proposal(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
    ) -> impl Future<Output = Result<Option<String>, StoreError>> + Send;

    /// True the first time only: the session was told how the approval ended.
    fn mark_told(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
    ) -> impl Future<Output = Result<bool, StoreError>> + Send;
}
