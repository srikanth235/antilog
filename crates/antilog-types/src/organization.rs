use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::id::text_enum;
use crate::{MemberId, OrganizationId, ProjectId, SignInId, TokenId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    /// Epoch milliseconds.
    pub created_at: i64,
    /// Monthly cap on all spend, in US cents. `None`: no cap.
    pub monthly_cap_cents: Option<i64>,
    /// Default budget per experiment, in US cents.
    pub default_budget_cents: Option<i64>,
}

impl Organization {
    pub fn new(name: impl Into<String>, created_at: i64) -> Self {
        Self {
            id: OrganizationId::new(),
            name: name.into(),
            created_at,
            monthly_cap_cents: None,
            default_budget_cents: None,
        }
    }
}

text_enum!(
    /// Ordered: a higher role may do what a lower one may.
    #[derive(PartialOrd, Ord)]
    Role { Viewer => "viewer", Editor => "editor", Owner => "owner" }
);

text_enum!(MemberKind { Person => "person", Service => "service" });

/// A person's identity at a sign-in provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Identity {
    pub provider: String,
    pub subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Member {
    pub id: MemberId,
    pub organization_id: OrganizationId,
    pub kind: MemberKind,
    pub name: String,
    /// A person has one; a service account has none.
    pub identity: Option<Identity>,
    /// The organization role, the default in every project.
    pub role: Role,
    pub created_at: i64,
}

impl Member {
    pub fn person(
        organization_id: OrganizationId,
        name: impl Into<String>,
        identity: Identity,
        role: Role,
        created_at: i64,
    ) -> Self {
        Self {
            id: MemberId::new(),
            organization_id,
            kind: MemberKind::Person,
            name: name.into(),
            identity: Some(identity),
            role,
            created_at,
        }
    }

    pub fn service(
        organization_id: OrganizationId,
        name: impl Into<String>,
        role: Role,
        created_at: i64,
    ) -> Self {
        Self {
            id: MemberId::new(),
            organization_id,
            kind: MemberKind::Service,
            name: name.into(),
            identity: None,
            role,
            created_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Project {
    pub id: ProjectId,
    pub organization_id: OrganizationId,
    pub name: String,
    pub description: String,
    /// Monthly cap on the spend of its experiments, in US cents.
    pub monthly_cap_cents: Option<i64>,
    /// Default budget per experiment, in US cents.
    pub default_budget_cents: Option<i64>,
    pub created_at: i64,
}

impl Project {
    pub fn new(
        organization_id: OrganizationId,
        name: impl Into<String>,
        description: impl Into<String>,
        created_at: i64,
    ) -> Self {
        Self {
            id: ProjectId::new(),
            organization_id,
            name: name.into(),
            description: description.into(),
            monthly_cap_cents: None,
            default_budget_cents: None,
            created_at,
        }
    }
}

/// A member's role in one project when it differs from the organization role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectRole {
    pub member_id: MemberId,
    pub role: Role,
}

/// What may be shown of an access token; the secret is shown once, at creation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccessToken {
    pub id: TokenId,
    pub organization_id: OrganizationId,
    pub member_id: MemberId,
    pub name: String,
    pub created_at: i64,
    pub revoked_at: Option<i64>,
}

/// A signed-in browser or app session. Its secret is shown once and stored as a hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SignInSession {
    pub id: SignInId,
    pub organization_id: OrganizationId,
    pub member_id: MemberId,
    pub created_at: i64,
    pub expires_at: i64,
    pub ended_at: Option<i64>,
}
