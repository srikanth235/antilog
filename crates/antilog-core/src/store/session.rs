use std::future::Future;

use antilog_types::{
    Agent, AuditEntry, EventKind, MemberId, OrganizationId, ProjectId, Session, SessionEvent,
    SessionId, SessionStatus, ToolRecord,
};

use super::StoreError;

/// What to change on a session; `None` leaves a field as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionChange {
    pub status: Option<SessionStatus>,
    pub note: Option<String>,
}

/// One item to append to a conversation. `key` is unique within the session: appending it again
/// changes nothing, so an item delivered twice is recorded once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEvent {
    pub organization: OrganizationId,
    pub session: SessionId,
    pub key: String,
    pub kind: EventKind,
    pub member: Option<MemberId>,
    pub text: String,
    pub tool: Option<ToolRecord>,
    pub at: i64,
}

pub trait SessionStore {
    fn create_session(
        &self,
        session: &Session,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn session(
        &self,
        organization: OrganizationId,
        id: SessionId,
    ) -> impl Future<Output = Result<Option<Session>, StoreError>> + Send;

    /// The organization's sessions, newest first; `project` keeps one project's, `creator` only
    /// that member's.
    fn sessions(
        &self,
        organization: OrganizationId,
        project: Option<ProjectId>,
        creator: Option<MemberId>,
    ) -> impl Future<Output = Result<Vec<Session>, StoreError>> + Send;

    /// Done and Failed are final: a change of status after them is `IllegalTransition`.
    fn update_session(
        &self,
        organization: OrganizationId,
        id: SessionId,
        change: &SessionChange,
        at: i64,
        audit: Option<&AuditEntry>,
    ) -> impl Future<Output = Result<Session, StoreError>> + Send;

    /// Moves the session to a version of an environment (`None`: the built-in default): the
    /// session's record and, when it has one, its machine's seed, with the audit entry, together.
    /// The proxy reads the seed, so the network follows at once. `NotFound` for no such session.
    fn set_session_environment(
        &self,
        organization: OrganizationId,
        session: SessionId,
        environment: Option<&str>,
        version: Option<u32>,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Idempotent on (session, key). Refused with `NotFound` for a session that is not there.
    fn append_event(&self, event: &NewEvent)
    -> impl Future<Output = Result<(), StoreError>> + Send;

    /// The conversation, oldest first: up to `limit` items after `after` (a `seq`; 0 for the start).
    fn session_events(
        &self,
        organization: OrganizationId,
        session: SessionId,
        after: i64,
        limit: u32,
    ) -> impl Future<Output = Result<Vec<SessionEvent>, StoreError>> + Send;

    /// The live stream's view of the whole organization: the `seq` and session of each
    /// conversation item after `after`, oldest first. No words, only where something changed.
    fn event_marks(
        &self,
        organization: OrganizationId,
        after: i64,
        limit: u32,
    ) -> impl Future<Output = Result<Vec<(i64, SessionId)>, StoreError>> + Send;

    /// The newest conversation `seq` in the organization, 0 when there is none.
    fn latest_event_seq(
        &self,
        organization: OrganizationId,
    ) -> impl Future<Output = Result<i64, StoreError>> + Send;

    /// Sessions changed at or after `since` (milliseconds), oldest change first.
    fn sessions_changed(
        &self,
        organization: OrganizationId,
        since: i64,
        limit: u32,
    ) -> impl Future<Output = Result<Vec<Session>, StoreError>> + Send;

    /// Adds or replaces one of the organization's agents, with its audit entry. Making it the
    /// default clears the previous default; a default must be enabled.
    fn put_agent(
        &self,
        organization: OrganizationId,
        agent: &Agent,
        at: i64,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// The agents an owner set, by id.
    fn agents(
        &self,
        organization: OrganizationId,
    ) -> impl Future<Output = Result<Vec<Agent>, StoreError>> + Send;
}
