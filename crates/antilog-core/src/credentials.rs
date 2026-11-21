//! Short-lived credentials a session's machine holds instead of real ones (`docs/V0-contracts.md`).
//! The proxy swaps a `Model` token for the organization's model key, the MCP endpoint accepts an
//! `Mcp` token, and the runner reports with a `Runner` token. Only a hash of each token is kept.

use std::future::Future;

use antilog_types::SessionId;
use jiff::SignedDuration;

use crate::token::RandomUnavailable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Purpose {
    Model,
    Mcp,
    Runner,
}

#[derive(Debug, thiserror::Error)]
pub enum CredentialsError {
    #[error(transparent)]
    Random(#[from] RandomUnavailable),
    #[error("the credential store is unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub trait SessionCredentials: Send + Sync {
    /// A fresh opaque token for this session and purpose, live for `ttl`. Shown once.
    fn mint(
        &self,
        session: SessionId,
        purpose: Purpose,
        ttl: SignedDuration,
    ) -> impl Future<Output = Result<String, CredentialsError>> + Send;

    /// The session the token belongs to, if it is live, not revoked and for this purpose.
    fn verify(
        &self,
        token: &str,
        purpose: Purpose,
    ) -> impl Future<Output = Result<Option<SessionId>, CredentialsError>> + Send;

    /// Every token of the session stops working.
    fn revoke_all(
        &self,
        session: SessionId,
    ) -> impl Future<Output = Result<(), CredentialsError>> + Send;
}
