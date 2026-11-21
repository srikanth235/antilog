use antilog_types::{AuditEntry, OrganizationId, SessionId, Spend};

use super::{FakeStore, ledger::not_found, locked};
use crate::store::{BudgetStore, SpendLine, SpendScope, SpendTotal, StoreError};

impl BudgetStore for FakeStore {
    async fn set_organization_limits(
        &self,
        organization: OrganizationId,
        monthly_cap_cents: Option<i64>,
        default_budget_cents: Option<i64>,
        max_running_sessions: Option<u32>,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        let mut next = not_found(ledger.organization(organization))?.clone();
        next.monthly_cap_cents = monthly_cap_cents;
        next.default_budget_cents = default_budget_cents;
        if let Some(max) = max_running_sessions {
            next.max_running_sessions = max;
        }
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

    async fn set_session_budget(
        &self,
        organization: OrganizationId,
        session: SessionId,
        budget_cap_cents: i64,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        let mut next = not_found(ledger.session(organization, session))?.clone();
        next.budget_cap_cents = Some(budget_cap_cents);
        ledger.session_fits(&next)?;
        if let Some(row) = ledger.sessions.iter_mut().find(|row| row.id == session) {
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
        session: SessionId,
    ) -> Result<Vec<SpendLine>, StoreError> {
        let ledger = locked(&self.ledger);
        let mut lines: Vec<SpendLine> = Vec::new();
        for row in ledger
            .spend
            .iter()
            .filter(|row| row.organization_id == organization && row.session_id == session)
        {
            match lines.iter_mut().find(|line| line.kind == row.kind) {
                Some(line) => {
                    line.cents += row.amount_cents;
                    line.tokens += row.tokens;
                }
                None => lines.push(SpendLine {
                    kind: row.kind,
                    cents: row.amount_cents,
                    tokens: row.tokens,
                }),
            }
        }
        Ok(lines)
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
            let inside = match scope {
                SpendScope::Organization => true,
                SpendScope::Session(id) => row.session_id == id,
            };
            if inside {
                total.cents += row.amount_cents;
                total.tokens += row.tokens;
            }
        }
        Ok(total)
    }
}
