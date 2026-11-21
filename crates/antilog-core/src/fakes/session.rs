use antilog_types::{
    Agent, AuditEntry, EvaluationTemplate, Experiment, ExperimentId, ExperimentStatus,
    OrganizationId, ProjectId, Session, SessionEvent, SessionId,
};

use super::{FakeStore, ledger::not_found, locked};
use crate::store::{Invariant, NewEvent, SessionChange, SessionStore, StoreError};

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
        row.updated_at = at;
        let updated = row.clone();
        ledger.audit.extend(audit.cloned());
        Ok(updated)
    }

    async fn append_event(&self, event: &NewEvent) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.session(event.organization, event.session))?;
        let key = format!("{}:{}", event.session.as_uuid(), event.key);
        if ledger.events.iter().any(|(known, _)| *known == key) {
            return Ok(());
        }
        let seq = i64::try_from(ledger.events.len()).unwrap_or(i64::MAX) + 1;
        ledger.events.push((
            key,
            SessionEvent {
                seq,
                session_id: event.session,
                kind: event.kind,
                member_id: event.member,
                text: event.text.clone(),
                tool: event.tool.clone(),
                at: event.at,
            },
        ));
        Ok(())
    }

    async fn session_events(
        &self,
        organization: OrganizationId,
        session: SessionId,
        after: i64,
        limit: u32,
    ) -> Result<Vec<SessionEvent>, StoreError> {
        let ledger = locked(&self.ledger);
        if ledger.session(organization, session).is_none() {
            return Ok(Vec::new());
        }
        Ok(ledger
            .events
            .iter()
            .map(|(_, event)| event)
            .filter(|event| event.session_id == session && event.seq > after)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn event_marks(
        &self,
        organization: OrganizationId,
        after: i64,
        limit: u32,
    ) -> Result<Vec<(i64, SessionId)>, StoreError> {
        let ledger = locked(&self.ledger);
        Ok(ledger
            .events
            .iter()
            .map(|(_, event)| event)
            .filter(|event| {
                event.seq > after && ledger.session(organization, event.session_id).is_some()
            })
            .take(limit as usize)
            .map(|event| (event.seq, event.session_id))
            .collect())
    }

    async fn latest_event_seq(&self, organization: OrganizationId) -> Result<i64, StoreError> {
        let ledger = locked(&self.ledger);
        Ok(ledger
            .events
            .iter()
            .map(|(_, event)| event)
            .filter(|event| ledger.session(organization, event.session_id).is_some())
            .map(|event| event.seq)
            .max()
            .unwrap_or(0))
    }

    async fn sessions_changed(
        &self,
        organization: OrganizationId,
        since: i64,
        limit: u32,
    ) -> Result<Vec<Session>, StoreError> {
        let mut rows: Vec<Session> = locked(&self.ledger)
            .sessions
            .iter()
            .filter(|row| row.organization_id == organization && row.updated_at >= since)
            .cloned()
            .collect();
        rows.sort_by_key(|row| row.updated_at);
        rows.truncate(limit as usize);
        Ok(rows)
    }

    async fn put_agent(
        &self,
        organization: OrganizationId,
        agent: &Agent,
        _at: i64,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.organization(organization))?;
        if agent.is_default && !agent.enabled {
            return Err(Invariant::IllegalTransition.into());
        }
        if agent.is_default {
            for (id, row) in &mut ledger.agents {
                if *id == organization {
                    row.is_default = false;
                }
            }
        }
        match ledger
            .agents
            .iter_mut()
            .find(|(id, row)| *id == organization && row.id == agent.id)
        {
            Some((_, row)) => *row = agent.clone(),
            None => ledger.agents.push((organization, agent.clone())),
        }
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn agents(&self, organization: OrganizationId) -> Result<Vec<Agent>, StoreError> {
        let mut agents: Vec<Agent> = locked(&self.ledger)
            .agents
            .iter()
            .filter(|(id, _)| *id == organization)
            .map(|(_, agent)| agent.clone())
            .collect();
        agents.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(agents)
    }

    async fn add_evaluation(
        &self,
        template: &EvaluationTemplate,
        organization: OrganizationId,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.member(organization, template.created_by))?;
        if ledger
            .evaluations
            .iter()
            .any(|(id, row)| *id == organization && row.id == template.id)
        {
            return Err(StoreError::Conflict);
        }
        ledger.evaluations.push((organization, template.clone()));
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn evaluations(
        &self,
        organization: OrganizationId,
    ) -> Result<Vec<EvaluationTemplate>, StoreError> {
        Ok(locked(&self.ledger)
            .evaluations
            .iter()
            .filter(|(id, _)| *id == organization)
            .map(|(_, row)| row.clone())
            .collect())
    }
}
