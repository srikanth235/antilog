//! The organization floor's action layer: one permission check, one audit entry per write.

use antilog_types::{
    AccessToken, AuditEntry, AuditId, AuditOutcome, ExperimentId, Identity, Member, MemberId,
    Organization, OrganizationId, Project, ProjectId, ProjectRole, Role, TokenId,
};

use crate::store::{Invariant, Store, StoreError};
use crate::token::{self, IssuedToken, RandomUnavailable};

#[derive(Debug, thiserror::Error)]
pub enum Refusal {
    #[error("not permitted")]
    Denied,
    #[error("not found")]
    NotFound,
    #[error("already exists")]
    Conflict,
    #[error("{0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Invariant(Invariant),
    #[error(transparent)]
    Spent(crate::budget::Spent),
    #[error("unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl From<StoreError> for Refusal {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::Conflict => Self::Conflict,
            StoreError::NotFound => Self::NotFound,
            StoreError::Invariant(invariant) => Self::Invariant(invariant),
            StoreError::Unavailable(source) => Self::Unavailable(source),
        }
    }
}

impl From<RandomUnavailable> for Refusal {
    fn from(error: RandomUnavailable) -> Self {
        Self::Unavailable(Box::new(error))
    }
}

/// Who is acting: a member of one organization, proven by a session or an access token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Principal {
    pub organization: OrganizationId,
    pub member: MemberId,
}

/// Every write that changes something, each with the least role that may run it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    MemberAdd,
    MemberSetRole,
    ProjectCreate,
    ProjectRoleSet,
    ProjectLimitsSet,
    OrganizationLimitsSet,
    PolicySet,
    ExperimentBudgetSet,
    ApprovalDecide,
    ApprovalDecideValueModel,
    TokenCreateOwn,
    TokenCreateForOther,
    TokenRevokeOther,
    AuditRead,
}

impl Action {
    pub const ALL: &'static [Self] = &[
        Self::MemberAdd,
        Self::MemberSetRole,
        Self::ProjectCreate,
        Self::ProjectRoleSet,
        Self::ProjectLimitsSet,
        Self::OrganizationLimitsSet,
        Self::PolicySet,
        Self::ExperimentBudgetSet,
        Self::ApprovalDecide,
        Self::ApprovalDecideValueModel,
        Self::TokenCreateOwn,
        Self::TokenCreateForOther,
        Self::TokenRevokeOther,
        Self::AuditRead,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::MemberAdd => "member.add",
            Self::MemberSetRole => "member.set_role",
            Self::ProjectCreate => "project.create",
            Self::ProjectRoleSet => "project.set_role",
            Self::ProjectLimitsSet => "project.set_limits",
            Self::OrganizationLimitsSet => "organization.set_limits",
            Self::PolicySet => "policy.set",
            Self::ExperimentBudgetSet => "experiment.set_budget",
            Self::ApprovalDecide | Self::ApprovalDecideValueModel => "approval.decide",
            Self::TokenCreateOwn | Self::TokenCreateForOther => "token.create",
            Self::TokenRevokeOther => "token.revoke",
            Self::AuditRead => "audit.read",
        }
    }

    /// The one permission matrix. Project actions are checked against the member's role
    /// in that project, the rest against their organization role.
    pub const fn minimum(self) -> Role {
        match self {
            Self::TokenCreateOwn => Role::Viewer,
            Self::ExperimentBudgetSet | Self::ApprovalDecide => Role::Editor,
            Self::MemberAdd
            | Self::MemberSetRole
            | Self::ProjectCreate
            | Self::ProjectRoleSet
            | Self::ProjectLimitsSet
            | Self::OrganizationLimitsSet
            | Self::PolicySet
            | Self::ApprovalDecideValueModel
            | Self::TokenCreateForOther
            | Self::TokenRevokeOther
            | Self::AuditRead => Role::Owner,
        }
    }

    pub const fn in_project(self) -> bool {
        matches!(
            self,
            Self::ProjectRoleSet
                | Self::ProjectLimitsSet
                | Self::ExperimentBudgetSet
                | Self::ApprovalDecide
                | Self::ApprovalDecideValueModel
        )
    }
}

pub(crate) fn entry(
    who: &Principal,
    action: &str,
    target: impl ToString,
    outcome: AuditOutcome,
    now: i64,
) -> AuditEntry {
    AuditEntry {
        id: AuditId::new(),
        organization_id: who.organization,
        actor: Some(who.member),
        action: action.to_owned(),
        target: target.to_string(),
        outcome,
        at: now,
    }
}

/// The only place permission is decided. A denial is audited and changes nothing.
pub async fn permit<S: Store>(
    store: &S,
    who: &Principal,
    action: Action,
    project: Option<ProjectId>,
    target: impl ToString,
    now: i64,
) -> Result<Role, Refusal> {
    let role = match project.filter(|_| action.in_project()) {
        Some(project) => {
            store
                .project(who.organization, project)
                .await?
                .ok_or(Refusal::NotFound)?;
            store
                .role_in_project(who.organization, project, who.member)
                .await?
        }
        None => store
            .member(who.organization, who.member)
            .await?
            .map(|member| member.role),
    };
    match role {
        Some(role) if role >= action.minimum() => Ok(role),
        _ => {
            store
                .append_audit(&entry(
                    who,
                    action.name(),
                    target,
                    AuditOutcome::Denied,
                    now,
                ))
                .await?;
            Err(Refusal::Denied)
        }
    }
}

/// A read needs only membership: every member reads every project.
pub async fn member_of<S: Store>(store: &S, who: &Principal) -> Result<Member, Refusal> {
    store
        .member(who.organization, who.member)
        .await?
        .ok_or(Refusal::Denied)
}

fn name_ok(name: &str, what: &'static str) -> Result<(), Refusal> {
    let length = name.trim().chars().count();
    if (1..=200).contains(&length) {
        Ok(())
    } else {
        Err(Refusal::Invalid(what))
    }
}

pub async fn add_member<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    name: &str,
    identity: Option<Identity>,
    role: Role,
) -> Result<Member, Refusal> {
    name_ok(name, "a member's name is 1 to 200 characters")?;
    permit(store, who, Action::MemberAdd, None, name, now).await?;
    let name = name.trim();
    let member = match identity {
        Some(identity) => Member::person(who.organization, name, identity, role, now),
        None => Member::service(who.organization, name, role, now),
    };
    store
        .add_member(
            &member,
            &entry(who, "member.add", member.id, AuditOutcome::Done, now),
        )
        .await?;
    Ok(member)
}

pub async fn set_member_role<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    member: MemberId,
    role: Role,
) -> Result<(), Refusal> {
    permit(store, who, Action::MemberSetRole, None, member, now).await?;
    store
        .set_member_role(
            who.organization,
            member,
            role,
            &entry(who, "member.set_role", member, AuditOutcome::Done, now),
        )
        .await?;
    Ok(())
}

pub async fn create_project<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    name: &str,
    description: &str,
) -> Result<Project, Refusal> {
    name_ok(name, "a project's name is 1 to 200 characters")?;
    if description.chars().count() > 2000 {
        return Err(Refusal::Invalid("a description is at most 2000 characters"));
    }
    permit(store, who, Action::ProjectCreate, None, name, now).await?;
    let project = Project::new(who.organization, name.trim(), description, now);
    store
        .create_project(
            &project,
            who.member,
            &entry(who, "project.create", project.id, AuditOutcome::Done, now),
        )
        .await?;
    Ok(project)
}

pub async fn set_project_role<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    project: ProjectId,
    member: MemberId,
    role: Option<Role>,
) -> Result<(), Refusal> {
    permit(
        store,
        who,
        Action::ProjectRoleSet,
        Some(project),
        project,
        now,
    )
    .await?;
    store
        .set_project_role(
            who.organization,
            project,
            member,
            role,
            &entry(who, "project.set_role", project, AuditOutcome::Done, now),
        )
        .await?;
    Ok(())
}

pub async fn project_roles<S: Store>(
    store: &S,
    who: &Principal,
    project: ProjectId,
) -> Result<Vec<ProjectRole>, Refusal> {
    member_of(store, who).await?;
    store
        .project(who.organization, project)
        .await?
        .ok_or(Refusal::NotFound)?;
    Ok(store.project_roles(who.organization, project).await?)
}

/// A token for oneself, or for another member (a service account) if an organization owner.
pub async fn create_token<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    member: Option<MemberId>,
    name: &str,
) -> Result<IssuedToken, Refusal> {
    name_ok(name, "a token's name is 1 to 200 characters")?;
    let member = member.unwrap_or(who.member);
    let action = if member == who.member {
        Action::TokenCreateOwn
    } else {
        Action::TokenCreateForOther
    };
    permit(store, who, action, None, member, now).await?;
    store
        .member(who.organization, member)
        .await?
        .ok_or(Refusal::NotFound)?;
    let issued = token::issue(who.organization, member, name.trim(), now)?;
    store
        .create_access_token(
            &issued.record,
            &entry(
                who,
                "token.create",
                issued.record.token.id,
                AuditOutcome::Done,
                now,
            ),
        )
        .await?;
    Ok(issued)
}

pub async fn tokens<S: Store>(
    store: &S,
    who: &Principal,
    member: Option<MemberId>,
) -> Result<Vec<AccessToken>, Refusal> {
    let me = member_of(store, who).await?;
    let member = member.unwrap_or(me.id);
    if member != me.id && me.role < Role::Owner {
        return Err(Refusal::Denied);
    }
    Ok(store.access_tokens(who.organization, member).await?)
}

pub async fn revoke_token<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    token: TokenId,
) -> Result<(), Refusal> {
    let record = store.access_token(token).await?;
    let owned = record
        .filter(|record| record.token.organization_id == who.organization)
        .ok_or(Refusal::NotFound)?;
    if owned.token.member_id != who.member {
        permit(store, who, Action::TokenRevokeOther, None, token, now).await?;
    } else {
        member_of(store, who).await?;
    }
    store
        .revoke_access_token(
            who.organization,
            token,
            now,
            &entry(who, "token.revoke", token, AuditOutcome::Done, now),
        )
        .await?;
    Ok(())
}

pub async fn audit_log<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    before: Option<(i64, AuditId)>,
    limit: u32,
) -> Result<Vec<AuditEntry>, Refusal> {
    permit(store, who, Action::AuditRead, None, who.organization, now).await?;
    Ok(store
        .audit_log(who.organization, before, limit.clamp(1, 200))
        .await?)
}

pub async fn organization<S: Store>(store: &S, who: &Principal) -> Result<Organization, Refusal> {
    member_of(store, who).await?;
    store
        .organization(who.organization)
        .await?
        .ok_or(Refusal::NotFound)
}

/// The project an experiment belongs to, for a permission check at project scope.
pub async fn project_of<S: Store>(
    store: &S,
    who: &Principal,
    experiment: ExperimentId,
) -> Result<ProjectId, Refusal> {
    store
        .experiment(who.organization, experiment)
        .await?
        .map(|experiment| experiment.project_id)
        .ok_or(Refusal::NotFound)
}
