use std::future::Future;

use antilog_types::{
    ContextMark, ContextPart, ContextUse, OrganizationId, SessionContext, SessionId,
};

use super::StoreError;

/// How full a session's context window is: the latest figures only, replaced on each report.
/// The runner writes the use, the gateway writes the parts; each leaves the other's alone.
pub trait ContextStore {
    /// Replaces the latest use. `NotFound` for a session that is not in the organization.
    fn put_context_use(
        &self,
        organization: OrganizationId,
        session: SessionId,
        context: ContextUse,
        at: i64,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Replaces the latest measured parts.
    fn put_context_parts(
        &self,
        organization: OrganizationId,
        session: SessionId,
        parts: &[ContextPart],
        at: i64,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// The latest figures; all empty before the first report.
    fn context(
        &self,
        organization: OrganizationId,
        session: SessionId,
    ) -> impl Future<Output = Result<SessionContext, StoreError>> + Send;

    /// Sessions reported at or after `since` (milliseconds), oldest first, for the live stream.
    fn context_marks(
        &self,
        organization: OrganizationId,
        since: i64,
        limit: u32,
    ) -> impl Future<Output = Result<Vec<ContextMark>, StoreError>> + Send;
}
