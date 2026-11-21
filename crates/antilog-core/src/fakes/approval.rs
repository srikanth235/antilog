use antilog_types::{
    Approval, ApprovalId, ApprovalStatus, AuditEntry, AuditId, CardKind, Decision, MemberId,
    OrganizationId, Policy, PolicyMode, SessionId,
};

use super::{FakeStore, ledger::not_found, locked};
use crate::store::{ApprovalStore, Invariant, StoreError};

#[derive(Debug, thiserror::Error)]
#[error("a policy's limit is set exactly when its mode is allow_up_to")]
struct BadPolicy;

impl ApprovalStore for FakeStore {
    async fn set_policy(
        &self,
        organization: OrganizationId,
        policy: &Policy,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.organization(organization))?;
        if policy.card == CardKind::AddCredential && policy.mode != PolicyMode::AlwaysAsk {
            return Err(Invariant::AlwaysAsk.into());
        }
        if (policy.mode == PolicyMode::AllowUpTo) != policy.limit_cents.is_some() {
            return Err(StoreError::Unavailable(Box::new(BadPolicy)));
        }
        ledger
            .policies
            .retain(|(id, row)| !(*id == organization && row.card == policy.card));
        ledger.policies.push((organization, *policy));
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn policies(&self, organization: OrganizationId) -> Result<Vec<Policy>, StoreError> {
        let mut policies: Vec<_> = locked(&self.ledger)
            .policies
            .iter()
            .filter(|(id, _)| *id == organization)
            .map(|(_, policy)| *policy)
            .collect();
        policies.sort_by_key(|policy| policy.card.as_str());
        Ok(policies)
    }

    async fn request_approval(
        &self,
        approval: &Approval,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.session(approval.organization_id, approval.session_id))?;
        let by_policy = approval.status == ApprovalStatus::ApprovedByPolicy;
        if by_policy && approval.card == CardKind::AddCredential {
            return Err(Invariant::AlwaysAsk.into());
        }
        if !(by_policy || approval.status == ApprovalStatus::Pending)
            || approval.decided_by.is_some()
        {
            return Err(Invariant::IllegalTransition.into());
        }
        if ledger.approvals.iter().any(|row| row.id == approval.id) {
            return Err(StoreError::Conflict);
        }
        ledger.approvals.push(approval.clone());
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn decide_approval(
        &self,
        organization: OrganizationId,
        id: ApprovalId,
        decision: Decision,
        by: MemberId,
        at: i64,
        audit: &AuditEntry,
    ) -> Result<Approval, StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.member(organization, by))?;
        let row = not_found(
            ledger
                .approvals
                .iter_mut()
                .find(|row| row.organization_id == organization && row.id == id),
        )?;
        if row.status != ApprovalStatus::Pending {
            return Err(Invariant::IllegalTransition.into());
        }
        row.status = match decision {
            Decision::Approve => ApprovalStatus::Approved,
            Decision::Decline => ApprovalStatus::Declined,
        };
        row.decided_by = Some(by);
        row.decided_at = Some(at);
        let decided = row.clone();
        ledger.audit.push(audit.clone());
        Ok(decided)
    }

    async fn approval(
        &self,
        organization: OrganizationId,
        id: ApprovalId,
    ) -> Result<Option<Approval>, StoreError> {
        Ok(locked(&self.ledger)
            .approvals
            .iter()
            .find(|row| row.organization_id == organization && row.id == id)
            .cloned())
    }

    async fn approvals(
        &self,
        organization: OrganizationId,
        session: SessionId,
    ) -> Result<Vec<Approval>, StoreError> {
        Ok(locked(&self.ledger)
            .approvals
            .iter()
            .filter(|row| row.organization_id == organization && row.session_id == session)
            .cloned()
            .collect())
    }

    async fn pending_approvals(
        &self,
        organization: OrganizationId,
    ) -> Result<Vec<Approval>, StoreError> {
        let mut pending: Vec<_> = locked(&self.ledger)
            .approvals
            .iter()
            .filter(|row| {
                row.organization_id == organization && row.status == ApprovalStatus::Pending
            })
            .cloned()
            .collect();
        pending.sort_by_key(|row| (row.created_at, row.id.as_uuid()));
        Ok(pending)
    }

    async fn append_audit(&self, entry: &AuditEntry) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.organization(entry.organization_id))?;
        if ledger.audit.iter().any(|row| row.id == entry.id) {
            return Err(StoreError::Conflict);
        }
        ledger.audit.push(entry.clone());
        Ok(())
    }

    async fn audit_log(
        &self,
        organization: OrganizationId,
        before: Option<(i64, AuditId)>,
        limit: u32,
    ) -> Result<Vec<AuditEntry>, StoreError> {
        let mut entries: Vec<_> = locked(&self.ledger)
            .audit
            .iter()
            .filter(|row| {
                row.organization_id == organization
                    && before.is_none_or(|cursor| (row.at, row.id) < cursor)
            })
            .cloned()
            .collect();
        entries.sort_by_key(|row| std::cmp::Reverse((row.at, row.id)));
        entries.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
        Ok(entries)
    }
}
