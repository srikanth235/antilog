//! Request and response bodies of the organization floor's API.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    AccessToken, AuditEntry, AuditId, Decision, Identity, Member, MemberId, Organization,
    OrganizationId, ProjectId, Role,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SignInProviders {
    pub providers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SignInStart {
    pub authorize_url: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SignInCallback {
    /// Left out, the person's organizations are found from their identity: the oldest
    /// membership signs in, and `GET /me` lists the rest.
    #[serde(default)]
    pub organization: Option<OrganizationId>,
    pub code: String,
    pub state: String,
}

/// Who is signed in: the member, their organization and their role in it, and every
/// organization the same identity belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Me {
    pub member: Member,
    pub organization: Organization,
    /// Their organization role, which is their default in every project.
    pub role: Role,
    pub organizations: Vec<OrganizationRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizationRef {
    pub id: OrganizationId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewOrganization {
    pub name: String,
    pub code: String,
    pub state: String,
}

/// A sign-in session; the secret is shown once and is also set as a cookie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionIssued {
    pub member: Member,
    pub secret: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizationCreated {
    pub organization: Organization,
    pub session: SessionIssued,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewMember {
    pub name: String,
    pub role: Role,
    /// A person's identity at a sign-in provider; none makes a service account.
    pub identity: Option<Identity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetRole {
    pub role: Role,
}

/// `None` returns the member to their organization role in this project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetProjectRole {
    pub role: Option<Role>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewProject {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Limits {
    pub monthly_cap_cents: Option<i64>,
    pub default_budget_cents: Option<i64>,
    /// Most sessions running on machines at once; omitted: unchanged.
    #[serde(default)]
    pub max_running_sessions: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExperimentBudget {
    pub budget_cap_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewToken {
    /// A service account to issue it for (owners only); omitted: the caller.
    pub member_id: Option<MemberId>,
    pub name: String,
}

/// An access token; the secret is shown once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TokenIssued {
    pub token: AccessToken,
    pub secret: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Decide {
    pub decision: Decision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LimitSpend {
    pub cap_cents: Option<i64>,
    pub spent_cents: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectSpend {
    pub project_id: ProjectId,
    pub cap_cents: Option<i64>,
    pub spent_cents: i64,
}

/// This month's spend against the caps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetSummary {
    pub organization: LimitSpend,
    pub projects: Vec<ProjectSpend>,
}

/// What one kind of spend came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KindSpend {
    pub kind: crate::SpendKind,
    pub cents: i64,
    pub tokens: i64,
}

/// What one session spent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionSpend {
    pub session_id: crate::SessionId,
    pub cents: i64,
    pub tokens: i64,
}

/// All the spend of one experiment since it began, against its budget. Spend rows name a session
/// and a kind, not a trial, so it is split by kind and by session, never by a trial's outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExperimentSpend {
    pub experiment_id: crate::ExperimentId,
    pub project_id: ProjectId,
    pub budget_cap_cents: i64,
    pub spent_cents: i64,
    pub tokens: i64,
    pub by_kind: Vec<KindSpend>,
    pub by_session: Vec<SessionSpend>,
}

/// Newest first; `next` is the cursor for the page after this one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditPage {
    pub entries: Vec<AuditEntry>,
    pub next: Option<AuditCursor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditCursor {
    pub at: i64,
    pub id: AuditId,
}
