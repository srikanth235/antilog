use std::future::Future;

use antilog_types::{ApprovalId, OrganizationId};

use super::StoreError;

/// What a decided card needs kept: whether its outcome was applied.
pub trait CardStore {
    /// True the first time only: the card's outcome was applied.
    fn mark_applied(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
    ) -> impl Future<Output = Result<bool, StoreError>> + Send;
}
