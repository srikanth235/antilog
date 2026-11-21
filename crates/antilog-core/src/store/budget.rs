use std::future::Future;

use antilog_types::{AuditEntry, OrganizationId, ProjectId, SessionId, Spend, SpendKind};

use super::StoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpendScope {
    Session(SessionId),
    Project(ProjectId),
    Organization,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpendTotal {
    pub cents: i64,
    pub tokens: i64,
}

/// What one session spent of one kind, summed over all time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpendLine {
    pub kind: SpendKind,
    pub cents: i64,
    pub tokens: i64,
}

/// Caps nest: a session's budget sits inside its project's cap, which sits inside the
/// organization's. Every setter refuses with `OverCap` what would pass one, and lowering a cap
/// below what sits inside it.
pub trait BudgetStore {
    fn set_organization_limits(
        &self,
        organization: OrganizationId,
        monthly_cap_cents: Option<i64>,
        default_budget_cents: Option<i64>,
        max_running_sessions: Option<u32>,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn set_project_limits(
        &self,
        organization: OrganizationId,
        project: ProjectId,
        monthly_cap_cents: Option<i64>,
        default_budget_cents: Option<i64>,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn set_session_budget(
        &self,
        organization: OrganizationId,
        session: SessionId,
        budget_cap_cents: i64,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn record_spend(&self, spend: &Spend) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// One session's spend summed per kind, over all time.
    fn spend_lines(
        &self,
        organization: OrganizationId,
        session: SessionId,
    ) -> impl Future<Output = Result<Vec<SpendLine>, StoreError>> + Send;

    /// Spend that occurred in `from..to` (epoch milliseconds).
    fn spend(
        &self,
        organization: OrganizationId,
        scope: SpendScope,
        from: i64,
        to: i64,
    ) -> impl Future<Output = Result<SpendTotal, StoreError>> + Send;
}
