use std::future::Future;

use antilog_types::{
    AuditEntry, ModelKeyStatus, ModelProvider, OrganizationId, SessionId,
};

use super::StoreError;
use crate::secrets::Secret;

/// What the model gateway needs to know about a session before it forwards a call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GatewayScope {
    pub organization: OrganizationId,
}

/// An organization's own model keys. The store seals each key before it is written and opens it
/// on the way out, so the key is never a column anyone can read. A store with no master key
/// refuses with `Unavailable`.
pub trait ModelKeyStore {
    /// Sets or replaces the organization's key for a provider, with its audit entry.
    fn put_model_key(
        &self,
        organization: OrganizationId,
        provider: ModelProvider,
        key: &Secret,
        at: i64,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Removes it, with its audit entry; `NotFound` when there is none.
    fn remove_model_key(
        &self,
        organization: OrganizationId,
        provider: ModelProvider,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// The providers that have a key, with when it was set. Never a key.
    fn model_keys(
        &self,
        organization: OrganizationId,
    ) -> impl Future<Output = Result<Vec<ModelKeyStatus>, StoreError>> + Send;

    /// The key itself, for the gateway alone.
    fn model_key(
        &self,
        organization: OrganizationId,
        provider: ModelProvider,
    ) -> impl Future<Output = Result<Option<Secret>, StoreError>> + Send;

    /// The organization of a session, found from its id alone because the
    /// gateway holds only a session token. `None` for a session that is not there.
    fn gateway_scope(
        &self,
        session: SessionId,
    ) -> impl Future<Output = Result<Option<GatewayScope>, StoreError>> + Send;
}
