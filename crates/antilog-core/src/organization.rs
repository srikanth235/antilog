//! The organization floor's action layer: one permission check, one audit entry per write.

use antilog_types::{
    AccessToken, AuditEntry, AuditId, AuditOutcome, Identity, Me, Member, MemberId, Organization,
    OrganizationId, OrganizationRef, Role, TokenId,
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
    OrganizationLimitsSet,
    PolicySet,
    AgentSet,
    EnvironmentCreate,
    EnvironmentEdit,
    EnvironmentArchive,
    ModelKeySet,
    ModelKeyRemove,
    ConnectorSet,
    SkillAdd,
    SkillRemove,
    PluginAdd,
    PluginRemove,
    SessionBudgetSet,
    SessionStart,
    SessionMessage,
    SessionStop,
    ToolCall,
    ApprovalDecide,
    TokenCreateOwn,
    TokenCreateForOther,
    TokenRevokeOther,
    AuditRead,
}

impl Action {
    pub const ALL: &'static [Self] = &[
        Self::MemberAdd,
        Self::MemberSetRole,
        Self::OrganizationLimitsSet,
        Self::PolicySet,
        Self::AgentSet,
        Self::EnvironmentCreate,
        Self::EnvironmentEdit,
        Self::EnvironmentArchive,
        Self::ModelKeySet,
        Self::ModelKeyRemove,
        Self::ConnectorSet,
        Self::SkillAdd,
        Self::SkillRemove,
        Self::PluginAdd,
        Self::PluginRemove,
        Self::SessionBudgetSet,
        Self::SessionStart,
        Self::SessionMessage,
        Self::SessionStop,
        Self::ToolCall,
        Self::ApprovalDecide,
        Self::TokenCreateOwn,
        Self::TokenCreateForOther,
        Self::TokenRevokeOther,
        Self::AuditRead,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::MemberAdd => "member.add",
            Self::MemberSetRole => "member.set_role",
            Self::OrganizationLimitsSet => "organization.set_limits",
            Self::PolicySet => "policy.set",
            Self::AgentSet => "agent.set",
            Self::EnvironmentCreate => "environment.create",
            Self::EnvironmentEdit => "environment.edit",
            Self::EnvironmentArchive => "environment.archive",
            Self::ModelKeySet => "model_key.set",
            Self::ModelKeyRemove => "model_key.remove",
            Self::ConnectorSet => "connector.set",
            Self::SkillAdd => "skill.add",
            Self::SkillRemove => "skill.remove",
            Self::PluginAdd => "plugin.add",
            Self::PluginRemove => "plugin.remove",
            Self::SessionBudgetSet => "session.set_budget",
            Self::SessionStart => "session.start",
            Self::SessionMessage => "session.message",
            Self::SessionStop => "session.stop",
            Self::ToolCall => "tool.call",
            Self::ApprovalDecide => "approval.decide",
            Self::TokenCreateOwn | Self::TokenCreateForOther => "token.create",
            Self::TokenRevokeOther => "token.revoke",
            Self::AuditRead => "audit.read",
        }
    }

    /// The one permission matrix, checked against the member's organization role.
    pub const fn minimum(self) -> Role {
        match self {
            Self::TokenCreateOwn => Role::Viewer,
            Self::SessionBudgetSet
            | Self::EnvironmentCreate
            | Self::EnvironmentEdit
            | Self::EnvironmentArchive
            | Self::SkillAdd
            | Self::SessionStart
            | Self::SessionMessage
            | Self::SessionStop
            | Self::ToolCall
            | Self::ApprovalDecide => Role::Editor,
            Self::MemberAdd
            | Self::MemberSetRole
            | Self::OrganizationLimitsSet
            | Self::PolicySet
            | Self::AgentSet
            | Self::ModelKeySet
            | Self::ModelKeyRemove
            | Self::ConnectorSet
            | Self::SkillRemove
            | Self::PluginAdd
            | Self::PluginRemove
            | Self::TokenCreateForOther
            | Self::TokenRevokeOther
            | Self::AuditRead => Role::Owner,
        }
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
    target: impl ToString,
    now: i64,
) -> Result<Role, Refusal> {
    let role = store
        .member(who.organization, who.member)
        .await?
        .map(|member| member.role);
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

/// Who is signed in, and every organization their identity belongs to.
pub async fn me<S: Store>(store: &S, who: &Principal) -> Result<Me, Refusal> {
    let member = member_of(store, who).await?;
    let organization = store
        .organization(who.organization)
        .await?
        .ok_or(Refusal::NotFound)?;
    let mut organizations = match &member.identity {
        Some(identity) => store
            .memberships(identity)
            .await?
            .into_iter()
            .map(|found| OrganizationRef {
                id: found.organization.id,
                name: found.organization.name,
            })
            .collect(),
        None => Vec::new(),
    };
    if organizations.is_empty() {
        organizations.push(OrganizationRef {
            id: organization.id,
            name: organization.name.clone(),
        });
    }
    Ok(Me {
        role: member.role,
        member,
        organization,
        organizations,
    })
}

/// A read needs only membership: every member reads every session.
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
    permit(store, who, Action::MemberAdd, name, now).await?;
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
    permit(store, who, Action::MemberSetRole, member, now).await?;
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
    permit(store, who, action, member, now).await?;
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
        permit(store, who, Action::TokenRevokeOther, token, now).await?;
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
    permit(store, who, Action::AuditRead, who.organization, now).await?;
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
