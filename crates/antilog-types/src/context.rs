//! How full a session's context window is. The runner reports what the latest model call used
//! and the model's window; the proxy measures what that call was made of. The server keeps the
//! latest of each per session and serves them to the Overview's Context section.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::SessionId;
use crate::id::text_enum;

/// The share of the window, in percent, at which the composer and the Overview warn.
pub const CONTEXT_WARN_PERCENT: u64 = 80;

text_enum!(
    /// What a part of a model request is made of.
    ContextPartKind {
        System => "system",
        Tools => "tools",
        AntilogTools => "antilog_tools",
        Messages => "messages",
    }
);

/// One measured part of the latest model request, in tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContextPart {
    pub kind: ContextPartKind,
    pub tokens: u64,
}

/// What the runner reports after a step: the context the latest call filled, and the model's
/// window when OpenCode knows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextUse {
    pub used: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<u64>,
}

/// A session's context window as the Overview reads it. A field the server has not been told
/// yet is `null` (or an empty list); a session that has made no call has all of them empty.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionContext {
    /// Tokens the latest model call filled: input plus cache reads and writes.
    pub used: Option<u64>,
    /// The model's context window, in tokens.
    pub window: Option<u64>,
    /// What the latest request was made of, scaled to the provider's own input count. Empty
    /// until the gateway has measured one.
    pub parts: Vec<ContextPart>,
    /// When anything last changed, epoch milliseconds; `null` before the first report.
    pub reported_at: Option<i64>,
}

/// A session's context changed. Not a conversation item; see `LiveEvent::Context`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextMark {
    pub session: SessionId,
    pub reported_at: i64,
}
