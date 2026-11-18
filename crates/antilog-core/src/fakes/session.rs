use antilog_types::{
    AuditEntry, Experiment, ExperimentId, ExperimentStatus, OrganizationId, ProjectId, Session,
    SessionId,
};

use super::{FakeStore, ledger::not_found, locked};
use crate::store::{Invariant, SessionChange, SessionStore, StoreError};

impl SessionStore for FakeStore {
    async fn create_experiment(
        &self,
        experiment: &Experiment,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.member(experiment.organization_id, experiment.created_by))?;
        ledger.experiment_fits(experiment)?;
        if ledger.experiments.iter().any(|row| row.id == experiment.id) {
            return Err(StoreError::Conflict);
        }
        ledger.experiments.push(experiment.clone());
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn experiment(
        &self,
        organization: OrganizationId,
        id: ExperimentId,
    ) -> Result<Option<Experiment>, StoreError> {
        Ok(locked(&self.ledger).experiment(organization, id).cloned())
    }

    async fn experiments(
        &self,
        organization: OrganizationId,
        project: ProjectId,
    ) -> Result<Vec<Experiment>, StoreError> {
        Ok(locked(&self.ledger)
            .experiments
            .iter()
            .filter(|row| row.organization_id == organization && row.project_id == project)
            .cloned()
            .collect())
    }

    async fn set_experiment_status(
        &self,
        organization: OrganizationId,
        id: ExperimentId,
        status: ExperimentStatus,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        let row = not_found(
            ledger
                .experiments
                .iter_mut()
                .find(|row| row.organization_id == organization && row.id == id),
        )?;
        if row.status == ExperimentStatus::Sealed && status != ExperimentStatus::Sealed {
            return Err(Invariant::IllegalTransition.into());
        }
        row.status = status;
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn create_session(
        &self,
        session: &Session,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.experiment(session.organization_id, session.experiment_id))?;
        if let Some(parent) = session.parent {
            let parent = not_found(ledger.session(session.organization_id, parent))?;
            if parent.experiment_id != session.experiment_id {
                return Err(StoreError::NotFound);
            }
            if parent.parent.is_some() {
                return Err(Invariant::SpawnedCannotSpawn.into());
            }
        }
        if ledger.sessions.iter().any(|row| row.id == session.id) {
            return Err(StoreError::Conflict);
        }
        ledger.sessions.push(session.clone());
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn session(
        &self,
        organization: OrganizationId,
        id: SessionId,
    ) -> Result<Option<Session>, StoreError> {
        Ok(locked(&self.ledger).session(organization, id).cloned())
    }

    async fn find_session(&self, id: SessionId) -> Result<Option<Session>, StoreError> {
        Ok(locked(&self.ledger)
            .sessions
            .iter()
            .find(|row| row.id == id)
            .cloned())
    }

    async fn sessions(
        &self,
        organization: OrganizationId,
        experiment: ExperimentId,
    ) -> Result<Vec<Session>, StoreError> {
        Ok(locked(&self.ledger)
            .sessions
            .iter()
            .filter(|row| row.organization_id == organization && row.experiment_id == experiment)
            .cloned()
            .collect())
    }

    async fn update_session(
        &self,
        organization: OrganizationId,
        id: SessionId,
        change: &SessionChange,
        at: i64,
        audit: Option<&AuditEntry>,
    ) -> Result<Session, StoreError> {
        let mut ledger = locked(&self.ledger);
        let row = not_found(
            ledger
                .sessions
                .iter_mut()
                .find(|row| row.organization_id == organization && row.id == id),
        )?;
        if let Some(status) = change.status {
            if !row.status.is_active() && status != row.status {
                return Err(Invariant::IllegalTransition.into());
            }
            row.status = status;
        }
        if change.note.is_some() {
            row.note.clone_from(&change.note);
        }
        if change.platform_handle.is_some() {
            row.platform_handle.clone_from(&change.platform_handle);
        }
        row.cleanup = change.cleanup.or(row.cleanup);
        row.updated_at = at;
        let updated = row.clone();
        ledger.audit.extend(audit.cloned());
        Ok(updated)
    }
}
