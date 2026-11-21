use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{SessionId, SessionStatus};

/// One message on `GET /events/stream`: a hint that something changed, so a client refetches
/// what it shows. It carries ids, a count and a status, never words, a card's text or any
/// secret. The SSE `event:` name is the kebab-case `type` and the `id:` is a conversation
/// `seq` (on `hello`, `resync` and `message`), which a reconnecting browser sends back as
/// `Last-Event-ID`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum LiveEvent {
    /// First message of a connection. Refetch what you show: it may have changed while you
    /// were away. `cursor` is the `seq` the stream continues from.
    Hello { cursor: i64 },
    /// The server cannot replay what was missed. Refetch everything you show.
    Resync { cursor: i64 },
    /// A conversation item was added to a session. Fetch `GET /sessions/{id}/events?after=`.
    #[serde(rename_all = "camelCase")]
    Message { session_id: SessionId, seq: i64 },
    /// A session's status changed.
    #[serde(rename_all = "camelCase")]
    Session {
        session_id: SessionId,
        status: SessionStatus,
    },
    /// A runner reported a session's changes: refetch `GET /sessions/{id}/changes`.
    #[serde(rename_all = "camelCase")]
    Changes { session_id: SessionId },
    /// The cards waiting for this member changed; `pending` is how many wait now.
    Cards { pending: u32 },
}

impl LiveEvent {
    /// The SSE event name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Hello { .. } => "hello",
            Self::Resync { .. } => "resync",
            Self::Message { .. } => "message",
            Self::Session { .. } => "session",
            Self::Changes { .. } => "changes",
            Self::Cards { .. } => "cards",
        }
    }

    /// The SSE `id:`: the conversation `seq` this event leaves a reconnecting client at.
    pub fn id(&self) -> Option<i64> {
        match self {
            Self::Hello { cursor } | Self::Resync { cursor } => Some(*cursor),
            Self::Message { seq, .. } => Some(*seq),
            Self::Session { .. } | Self::Changes { .. } | Self::Cards { .. } => None,
        }
    }
}
