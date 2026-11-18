use std::future::Future;

use antilog_types::{
    AuditEntry, Cleanup, Experiment, ExperimentId, ExperimentStatus, OrganizationId, ProjectId,
    Session, SessionId, SessionStatus,
};

use super::StoreError;

/// What to change on a session; `None` leaves a field as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionChange {
    pub status: Option<SessionStatus>,
    pub note: Option<String>,
    pub platform_handle: Option<String>,
    pub cleanup: Option<Cleanup>,
}

pub trait SessionStore {
    /// Refused with `OverCap` when its budget passes the project's or organization's cap.
    fn create_experiment(
        &self,
        experiment: &Experiment,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn experiment(
        &self,
        organization: OrganizationId,
        id: ExperimentId,
    ) -> impl Future<Output = Result<Option<Experiment>, StoreError>> + Send;

    fn experiments(
        &self,
        organization: OrganizationId,
        project: ProjectId,
    ) -> impl Future<Output = Result<Vec<Experiment>, StoreError>> + Send;

    /// Only `NotSealed` to `Sealed`; anything else is `IllegalTransition`.
    fn set_experiment_status(
        &self,
        organization: OrganizationId,
        id: ExperimentId,
        status: ExperimentStatus,
        audit: &AuditEntry,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// A parent must be a session of the same experiment that is not itself spawned.
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

    /// A session by its id alone, for a wake-up that carries no organization.
    fn find_session(
        &self,
        id: SessionId,
    ) -> impl Future<Output = Result<Option<Session>, StoreError>> + Send;

    fn sessions(
        &self,
        organization: OrganizationId,
        experiment: ExperimentId,
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
}
