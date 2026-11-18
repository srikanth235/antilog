use antilog_types::{AuditEntry, ExperimentId, OrganizationId, ProjectId, Spend};

use super::{FakeStore, ledger::not_found, locked};
use crate::store::{BudgetStore, SpendScope, SpendTotal, StoreError};

impl BudgetStore for FakeStore {
    async fn set_organization_limits(
        &self,
        organization: OrganizationId,
        monthly_cap_cents: Option<i64>,
        default_budget_cents: Option<i64>,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        let mut next = not_found(ledger.organization(organization))?.clone();
        next.monthly_cap_cents = monthly_cap_cents;
        next.default_budget_cents = default_budget_cents;
        ledger.organization_limits_fit(&next)?;
        if let Some(row) = ledger
            .organizations
            .iter_mut()
            .find(|row| row.id == organization)
        {
            *row = next;
        }
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn set_project_limits(
        &self,
        organization: OrganizationId,
        project: ProjectId,
        monthly_cap_cents: Option<i64>,
        default_budget_cents: Option<i64>,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        let mut next = not_found(ledger.project(organization, project))?.clone();
        next.monthly_cap_cents = monthly_cap_cents;
        next.default_budget_cents = default_budget_cents;
        ledger.project_limits_fit(&next)?;
        if let Some(row) = ledger.projects.iter_mut().find(|row| row.id == project) {
            *row = next;
        }
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn set_experiment_budget(
        &self,
        organization: OrganizationId,
        experiment: ExperimentId,
        budget_cap_cents: i64,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        let mut next = not_found(ledger.experiment(organization, experiment))?.clone();
        next.budget_cap_cents = budget_cap_cents;
        ledger.experiment_fits(&next)?;
        if let Some(row) = ledger
            .experiments
            .iter_mut()
            .find(|row| row.id == experiment)
        {
            *row = next;
        }
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn record_spend(&self, spend: &Spend) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.session(spend.organization_id, spend.session_id))?;
        if ledger.spend.iter().any(|row| row.id == spend.id) {
            return Err(StoreError::Conflict);
        }
        ledger.spend.push(spend.clone());
        Ok(())
    }

    async fn spend(
        &self,
        organization: OrganizationId,
        scope: SpendScope,
        from: i64,
        to: i64,
    ) -> Result<SpendTotal, StoreError> {
        let ledger = locked(&self.ledger);
        let mut total = SpendTotal::default();
        for row in ledger.spend.iter().filter(|row| {
            row.organization_id == organization && (from..to).contains(&row.occurred_at)
        }) {
            let experiment = ledger
                .session(organization, row.session_id)
                .map(|session| session.experiment_id);
            let inside = match scope {
                SpendScope::Organization => true,
                SpendScope::Experiment(id) => experiment == Some(id),
                SpendScope::Project(id) => experiment
                    .and_then(|experiment| ledger.experiment(organization, experiment))
                    .is_some_and(|experiment| experiment.project_id == id),
            };
            if inside {
                total.cents += row.amount_cents;
                total.tokens += row.tokens;
            }
        }
        Ok(total)
    }
}
