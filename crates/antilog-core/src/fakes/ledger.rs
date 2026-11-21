use antilog_types::{
    Agent, Approval, AuditEntry, Environment, Member, MemberId, Organization, OrganizationId,
    Policy, Role, Session, SessionEvent, SessionId, Spend,
};

use crate::store::{Invariant, SignInRecord, StoreError, TokenRecord};

#[derive(Default)]
pub(crate) struct Ledger {
    pub organizations: Vec<Organization>,
    pub members: Vec<Member>,
    pub tokens: Vec<TokenRecord>,
    pub sign_ins: Vec<SignInRecord>,
    pub sessions: Vec<Session>,
    pub spend: Vec<Spend>,
    pub policies: Vec<(OrganizationId, Policy)>,
    pub approvals: Vec<Approval>,
    pub audit: Vec<AuditEntry>,
    pub events: Vec<(String, SessionEvent)>,
    pub agents: Vec<(OrganizationId, Agent)>,
    /// Every version of every environment; `archived` is set on all of an environment's rows.
    pub environments: Vec<(OrganizationId, Environment)>,
}

pub(super) fn exceeds(value: Option<i64>, ceiling: Option<i64>) -> bool {
    matches!((value, ceiling), (Some(value), Some(ceiling)) if value > ceiling)
}

pub(super) fn not_found<T>(found: Option<T>) -> Result<T, StoreError> {
    found.ok_or(StoreError::NotFound)
}

impl Ledger {
    pub fn organization(&self, id: OrganizationId) -> Option<&Organization> {
        self.organizations.iter().find(|row| row.id == id)
    }

    pub fn member(&self, organization: OrganizationId, id: MemberId) -> Option<&Member> {
        self.members
            .iter()
            .find(|row| row.organization_id == organization && row.id == id)
    }

    pub fn session(&self, organization: OrganizationId, id: SessionId) -> Option<&Session> {
        self.sessions
            .iter()
            .find(|row| row.organization_id == organization && row.id == id)
    }

    pub fn keeps_owners(&self, organization: OrganizationId) -> Result<(), StoreError> {
        let members = || {
            self.members
                .iter()
                .filter(|member| member.organization_id == organization)
        };
        if members().next().is_some() && !members().any(|member| member.role == Role::Owner) {
            return Err(Invariant::LastOwner.into());
        }
        Ok(())
    }

    pub fn organization_limits_fit(&self, organization: &Organization) -> Result<(), StoreError> {
        let cap = organization.monthly_cap_cents;
        let over = exceeds(organization.default_budget_cents, cap)
            || self.sessions.iter().any(|session| {
                session.organization_id == organization.id
                    && exceeds(session.budget_cap_cents, cap)
            });
        if over {
            Err(Invariant::OverCap.into())
        } else {
            Ok(())
        }
    }

    pub fn session_fits(&self, session: &Session) -> Result<(), StoreError> {
        let cap = self
            .organization(session.organization_id)
            .and_then(|organization| organization.monthly_cap_cents);
        if exceeds(session.budget_cap_cents, cap) {
            Err(Invariant::OverCap.into())
        } else {
            Ok(())
        }
    }
}
