use std::future::Future;

use antilog_types::judge::TrialId;
use antilog_types::{
    AuditEntry, ExperimentId, OrganizationId, ProjectId, SessionId, Spend, SpendKind,
};

use super::StoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpendScope {
    Experiment(ExperimentId),
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
    pub experiment: ExperimentId,
    pub session: SessionId,
    pub kind: SpendKind,
    pub cents: i64,
    pub tokens: i64,
}

/// One spend row with the experiment its session belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpendRow {
    pub experiment: ExperimentId,
    pub session: SessionId,
    pub kind: SpendKind,
    pub cents: i64,
    pub at: i64,
    pub trial: Option<TrialId>,
}

/// Caps nest: an experiment's budget sits inside its project's cap, which sits
/// inside the organization's. Every setter refuses with `OverCap` what would pass one,
/// and lowering a cap below what sits inside it.
pub trait BudgetStore {
    fn set_organization_limits(
        &self,
        organization: OrganizationId,
        monthly_cap_cents: Option<i64>,
        default_budget_cents: Option<i64>,
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

    fn set_experiment_budget(
        &self,
        organization: OrganizationId,
        experiment: ExperimentId,
        budget_cap_cents: i64,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn record_spend(&self, spend: &Spend) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// The organization's spend summed per experiment, session and kind, or one experiment's.
    fn spend_lines(
        &self,
        organization: OrganizationId,
        experiment: Option<ExperimentId>,
    ) -> impl Future<Output = Result<Vec<SpendLine>, StoreError>> + Send;

    /// The organization's spend that occurred in `from..to`, summed per experiment, session and kind.
    fn spend_lines_between(
        &self,
        organization: OrganizationId,
        from: i64,
        to: i64,
    ) -> impl Future<Output = Result<Vec<SpendLine>, StoreError>> + Send;

    /// Every spend row that occurred in `from..to`, oldest first.
    fn spend_rows_between(
        &self,
        organization: OrganizationId,
        from: i64,
        to: i64,
    ) -> impl Future<Output = Result<Vec<SpendRow>, StoreError>> + Send;

    /// Spend that occurred in `from..to` (epoch milliseconds).
    fn spend(
        &self,
        organization: OrganizationId,
        scope: SpendScope,
        from: i64,
        to: i64,
    ) -> impl Future<Output = Result<SpendTotal, StoreError>> + Send;
}
