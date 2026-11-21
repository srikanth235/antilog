use std::future::Future;

use antilog_types::{AuditEntry, Environment, NetworkSecret, OrganizationId};

use super::StoreError;
use crate::secrets::Secret;

/// A network secret's value on its way in. `None` keeps the value the same host and header had in
/// the previous version.
#[derive(Debug, Clone)]
pub struct SecretValue {
    pub host: String,
    pub header: String,
    pub value: Option<Secret>,
}

/// The environments an organization manages. Every change is a new version; the versions are
/// kept, because a session records the one it ran. The built-in default is not stored.
pub trait EnvironmentStore {
    /// A new environment at version 1, with its audit entry; `Conflict` when the organization
    /// already has the id, archived or not.
    fn add_environment(
        &self,
        organization: OrganizationId,
        environment: &Environment,
        secrets: &[SecretValue],
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// The next version of an environment: `environment.version` must be one more than the
    /// newest stored (`Conflict` otherwise, so two edits made from one version cannot both land).
    /// `NotFound` when there is no such environment; `IllegalTransition` when it is archived.
    fn add_environment_version(
        &self,
        organization: OrganizationId,
        environment: &Environment,
        secrets: &[SecretValue],
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Archived environments are no longer offered to new sessions; sessions already started on
    /// one keep their version. Archiving twice changes nothing.
    fn archive_environment(
        &self,
        organization: OrganizationId,
        id: &str,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// The newest version of each environment, archived ones included, by id.
    fn environments(
        &self,
        organization: OrganizationId,
    ) -> impl Future<Output = Result<Vec<Environment>, StoreError>> + Send;

    /// One stored version, whether or not the environment has been archived since.
    fn environment_version(
        &self,
        organization: OrganizationId,
        id: &str,
        version: u32,
    ) -> impl Future<Output = Result<Option<Environment>, StoreError>> + Send;

    /// The values of a version's network secrets, opened: what the proxy adds to a request.
    /// Never served by the API.
    fn network_secret_values(
        &self,
        organization: OrganizationId,
        id: &str,
        version: u32,
    ) -> impl Future<Output = Result<Vec<(NetworkSecret, Secret)>, StoreError>> + Send;
}
