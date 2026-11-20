use std::future::Future;

use antilog_types::{ApprovalId, OrganizationId};

use super::StoreError;

/// What a decided card needs kept: the seal a person was asked to approve, and whether its
/// outcome was applied.
pub trait CardStore {
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

    /// True the first time only: the card's outcome was applied.
    fn mark_applied(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
    ) -> impl Future<Output = Result<bool, StoreError>> + Send;
}
