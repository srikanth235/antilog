use antilog_types::{AuditEntry, ExperimentId, OrganizationId, ProjectId, Spend};

use super::{FakeStore, ledger::not_found, locked};
use crate::store::{BudgetStore, SpendLine, SpendRow, SpendScope, SpendTotal, StoreError};

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

    async fn spend_lines(
        &self,
        organization: OrganizationId,
        experiment: Option<ExperimentId>,
    ) -> Result<Vec<SpendLine>, StoreError> {
        Ok(lines(
            &locked(&self.ledger),
            organization,
            experiment,
            0..i64::MAX,
        ))
    }

    async fn spend_lines_between(
        &self,
        organization: OrganizationId,
        from: i64,
        to: i64,
    ) -> Result<Vec<SpendLine>, StoreError> {
        Ok(lines(&locked(&self.ledger), organization, None, from..to))
    }

    async fn spend_rows_between(
        &self,
        organization: OrganizationId,
        from: i64,
        to: i64,
    ) -> Result<Vec<SpendRow>, StoreError> {
        let ledger = locked(&self.ledger);
        let mut rows: Vec<SpendRow> = ledger
            .spend
            .iter()
            .filter(|row| {
                row.organization_id == organization && (from..to).contains(&row.occurred_at)
            })
            .filter_map(|row| {
                let session = ledger.session(organization, row.session_id)?;
                Some(SpendRow {
                    experiment: session.experiment_id,
                    session: row.session_id,
                    kind: row.kind,
                    cents: row.amount_cents,
                    at: row.occurred_at,
                    trial: row.trial_id,
                })
            })
            .collect();
        rows.sort_by_key(|row| row.at);
        Ok(rows)
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

fn lines(
    ledger: &super::ledger::Ledger,
    organization: OrganizationId,
    experiment: Option<ExperimentId>,
    within: std::ops::Range<i64>,
) -> Vec<SpendLine> {
    let mut lines: Vec<SpendLine> = Vec::new();
    for row in ledger
        .spend
        .iter()
        .filter(|row| row.organization_id == organization && within.contains(&row.occurred_at))
    {
        let Some(session) = ledger.session(organization, row.session_id) else {
            continue;
        };
        if experiment.is_some_and(|wanted| wanted != session.experiment_id) {
            continue;
        }
        match lines
            .iter_mut()
            .find(|line| line.session == row.session_id && line.kind == row.kind)
        {
            Some(line) => {
                line.cents += row.amount_cents;
                line.tokens += row.tokens;
            }
            None => lines.push(SpendLine {
                experiment: session.experiment_id,
                session: row.session_id,
                kind: row.kind,
                cents: row.amount_cents,
                tokens: row.tokens,
            }),
        }
    }
    lines
}
