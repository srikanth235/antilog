use std::future::Future;

use antilog_types::{
    Approval, ApprovalId, AuditEntry, AuditId, Decision, MemberId, OrganizationId, Policy,
    SessionId,
};

use super::StoreError;

pub trait ApprovalStore {
    /// Add credential only ever asks: anything else is `AlwaysAsk`.
    fn set_policy(
        &self,
        organization: OrganizationId,
        policy: &Policy,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Only the card kinds that were set; the rest take their default.
    fn policies(
        &self,
        organization: OrganizationId,
    ) -> impl Future<Output = Result<Vec<Policy>, StoreError>> + Send;

    /// A new approval is `Pending` or `ApprovedByPolicy`, never decided by a person.
    fn request_approval(
        &self,
        approval: &Approval,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// A decided approval cannot be decided again (`IllegalTransition`).
    fn decide_approval(
        &self,
        organization: OrganizationId,
        id: ApprovalId,
        decision: Decision,
        by: MemberId,
        at: i64,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<Approval, StoreError>> + Send;

    fn approval(
        &self,
        organization: OrganizationId,
        id: ApprovalId,
    ) -> impl Future<Output = Result<Option<Approval>, StoreError>> + Send;

    /// The cards of one session, oldest first.
    fn approvals(
        &self,
        organization: OrganizationId,
        session: SessionId,
    ) -> impl Future<Output = Result<Vec<Approval>, StoreError>> + Send;

    /// Every card of the organization still waiting for a person, oldest first.
    fn pending_approvals(
        &self,
        organization: OrganizationId,
    ) -> impl Future<Output = Result<Vec<Approval>, StoreError>> + Send;

    /// For what has no write to ride on, such as a denial.
    fn append_audit(
        &self,
        entry: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Newest first, `limit` entries older than `before` (time and id).
    fn audit_log(
        &self,
        organization: OrganizationId,
        before: Option<(i64, AuditId)>,
        limit: u32,
    ) -> impl Future<Output = Result<Vec<AuditEntry>, StoreError>> + Send;
}
