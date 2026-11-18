use std::future::Future;

use antilog_types::{
    AccessToken, AuditEntry, Identity, Member, MemberId, Organization, OrganizationId, Project,
    ProjectId, ProjectRole, Role, TokenId,
};

use super::StoreError;

/// An access token as stored: only the hash of its secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenRecord {
    pub token: AccessToken,
    pub hash: [u8; 32],
}

/// Organizations, members, projects and their roles, access tokens.
/// Every write takes the audit entry that is stored with it or not at all.
pub trait OrganizationStore {
    fn create_organization(
        &self,
        organization: &Organization,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn organization(
        &self,
        id: OrganizationId,
    ) -> impl Future<Output = Result<Option<Organization>, StoreError>> + Send;

    fn add_member(
        &self,
        member: &Member,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn member(
        &self,
        organization: OrganizationId,
        id: MemberId,
    ) -> impl Future<Output = Result<Option<Member>, StoreError>> + Send;

    fn member_by_identity(
        &self,
        organization: OrganizationId,
        identity: &Identity,
    ) -> impl Future<Output = Result<Option<Member>, StoreError>> + Send;

    fn members(
        &self,
        organization: OrganizationId,
    ) -> impl Future<Output = Result<Vec<Member>, StoreError>> + Send;

    /// Refused with `LastOwner` when a project would be left without an owner.
    fn set_member_role(
        &self,
        organization: OrganizationId,
        member: MemberId,
        role: Role,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// The creator becomes the project's first owner.
    fn create_project(
        &self,
        project: &Project,
        owner: MemberId,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn project(
        &self,
        organization: OrganizationId,
        id: ProjectId,
    ) -> impl Future<Output = Result<Option<Project>, StoreError>> + Send;

    fn projects(
        &self,
        organization: OrganizationId,
    ) -> impl Future<Output = Result<Vec<Project>, StoreError>> + Send;

    /// Sets a member's role for one project, or with `None` returns them to the
    /// organization role. Refused with `LastOwner` as above.
    fn set_project_role(
        &self,
        organization: OrganizationId,
        project: ProjectId,
        member: MemberId,
        role: Option<Role>,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn project_roles(
        &self,
        organization: OrganizationId,
        project: ProjectId,
    ) -> impl Future<Output = Result<Vec<ProjectRole>, StoreError>> + Send;

    /// The one place a role is resolved: the project's role for the member if
    /// set, else their organization role. `None` for an unknown member.
    fn role_in_project(
        &self,
        organization: OrganizationId,
        project: ProjectId,
        member: MemberId,
    ) -> impl Future<Output = Result<Option<Role>, StoreError>> + Send;

    fn create_access_token(
        &self,
        record: &TokenRecord,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// By id alone, because the organization is learned from the token.
    fn access_token(
        &self,
        id: TokenId,
    ) -> impl Future<Output = Result<Option<TokenRecord>, StoreError>> + Send;

    fn access_tokens(
        &self,
        organization: OrganizationId,
        member: MemberId,
    ) -> impl Future<Output = Result<Vec<AccessToken>, StoreError>> + Send;

    fn revoke_access_token(
        &self,
        organization: OrganizationId,
        id: TokenId,
        at: i64,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;
}
