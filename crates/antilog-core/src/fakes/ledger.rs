use antilog_types::{
    Agent, Approval, AuditEntry, Environment, Member, MemberId, Organization, OrganizationId,
    Policy, Project, ProjectId, ProjectRole, Role, Session, SessionEvent, SessionId, Spend,
};

use crate::secrets::Secret;
use crate::store::{Invariant, SignInRecord, StoreError, TokenRecord};

#[derive(Default)]
pub(crate) struct Ledger {
    pub organizations: Vec<Organization>,
    pub members: Vec<Member>,
    pub projects: Vec<Project>,
    pub project_roles: Vec<(ProjectId, ProjectRole)>,
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
    /// (organization, environment id, version, host, header, value)
    pub environment_secrets: Vec<(OrganizationId, String, u32, String, String, Secret)>,
}

pub(super) fn least(a: Option<i64>, b: Option<i64>) -> Option<i64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, None) => a,
        (None, b) => b,
    }
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

    pub fn project(&self, organization: OrganizationId, id: ProjectId) -> Option<&Project> {
        self.projects
            .iter()
            .find(|row| row.organization_id == organization && row.id == id)
    }

    pub fn role_in_project(
        &self,
        organization: OrganizationId,
        project: ProjectId,
        member: MemberId,
    ) -> Option<Role> {
        self.project(organization, project)?;
        let member = self.member(organization, member)?;
        let own = self
            .project_roles
            .iter()
            .find(|(id, row)| *id == project && row.member_id == member.id);
        Some(own.map_or(member.role, |(_, row)| row.role))
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
        let orphan = self
            .projects
            .iter()
            .filter(|project| project.organization_id == organization)
            .any(|project| {
                !self.members.iter().any(|member| {
                    member.organization_id == organization
                        && self.role_in_project(organization, project.id, member.id)
                            == Some(Role::Owner)
                })
            });
        if orphan {
            Err(Invariant::LastOwner.into())
        } else {
            Ok(())
        }
    }

    pub fn project_limits_fit(&self, project: &Project) -> Result<(), StoreError> {
        let organization_cap = self
            .organization(project.organization_id)
            .and_then(|organization| organization.monthly_cap_cents);
        let ceiling = least(project.monthly_cap_cents, organization_cap);
        let over = exceeds(project.monthly_cap_cents, organization_cap)
            || exceeds(project.default_budget_cents, ceiling)
            || self.sessions.iter().any(|session| {
                session.project_id == project.id && exceeds(session.budget_cap_cents, ceiling)
            });
        if over {
            Err(Invariant::OverCap.into())
        } else {
            Ok(())
        }
    }

    pub fn organization_limits_fit(&self, organization: &Organization) -> Result<(), StoreError> {
        let cap = organization.monthly_cap_cents;
        let over = exceeds(organization.default_budget_cents, cap)
            || self.projects.iter().any(|project| {
                project.organization_id == organization.id
                    && (exceeds(project.monthly_cap_cents, cap)
                        || exceeds(project.default_budget_cents, cap))
            })
            || self.sessions.iter().any(|session| {
                session.organization_id == organization.id && exceeds(session.budget_cap_cents, cap)
            });
        if over {
            Err(Invariant::OverCap.into())
        } else {
            Ok(())
        }
    }

    pub fn session_fits(&self, session: &Session) -> Result<(), StoreError> {
        let project = not_found(self.project(session.organization_id, session.project_id))?;
        let ceiling = least(
            project.monthly_cap_cents,
            self.organization(project.organization_id)
                .and_then(|organization| organization.monthly_cap_cents),
        );
        if exceeds(session.budget_cap_cents, ceiling) {
            Err(Invariant::OverCap.into())
        } else {
            Ok(())
        }
    }
}
