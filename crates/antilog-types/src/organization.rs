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
    /// Most sessions this organization has on machines at once; queued ones wait.
    pub max_running_sessions: u32,
}

/// What an organization may run at once until an owner says otherwise.
pub const DEFAULT_MAX_RUNNING_SESSIONS: u32 = 10;

impl Organization {
    pub fn new(name: impl Into<String>, created_at: i64) -> Self {
        Self {
            id: OrganizationId::new(),
            name: name.into(),
            created_at,
            monthly_cap_cents: None,
            default_budget_cents: None,
            max_running_sessions: DEFAULT_MAX_RUNNING_SESSIONS,
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

/// A model provider whose key an organization brings. The proxy forwards to it unchanged.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum ModelProvider {
    Anthropic,
    Openai,
}

impl ModelProvider {
    pub const ALL: [Self; 2] = [Self::Anthropic, Self::Openai];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::Openai => "openai",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.as_str() == name)
    }
}

/// A key to set or replace. It is never returned.
#[derive(Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewModelKey {
    pub key: String,
}

impl std::fmt::Debug for NewModelKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NewModelKey([redacted])")
    }
}

/// Which providers an organization has a key for, and when it was last set. Never the key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelKeyStatus {
    pub provider: ModelProvider,
    pub set_at: i64,
}
