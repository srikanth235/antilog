use antilog_types::{
    Agent, AuditEntry, MemberId, OrganizationId, ProjectId, Session, SessionEvent, SessionId,
};

use super::{FakeStore, ledger::not_found, locked};
use crate::store::{Invariant, NewEvent, SessionChange, SessionStore, StoreError};

impl SessionStore for FakeStore {
    async fn create_session(
        &self,
        session: &Session,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.member(session.organization_id, session.created_by))?;
        ledger.session_fits(session)?;
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
        project: Option<ProjectId>,
        creator: Option<MemberId>,
    ) -> Result<Vec<Session>, StoreError> {
        let mut rows: Vec<Session> = locked(&self.ledger)
            .sessions
            .iter()
            .filter(|row| {
                row.organization_id == organization
                    && project.is_none_or(|project| project == row.project_id)
                    && creator.is_none_or(|member| member == row.created_by)
            })
            .cloned()
            .collect();
        rows.sort_by_key(|row| std::cmp::Reverse((row.created_at, row.id)));
        Ok(rows)
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

    async fn set_session_environment(
        &self,
        organization: OrganizationId,
        session: SessionId,
        environment: Option<&str>,
        version: Option<u32>,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        {
            let mut ledger = locked(&self.ledger);
            not_found(ledger.session(organization, session))?;
            if let Some(row) = ledger.sessions.iter_mut().find(|row| row.id == session) {
                row.environment = environment.map(str::to_owned);
                row.environment_version = version;
            }
            ledger.audit.push(audit.clone());
        }
        self.move_seed(session, environment, version);
        Ok(())
    }

    async fn set_session_model(
        &self,
        organization: OrganizationId,
        session: SessionId,
        model: &str,
        effort: Option<&str>,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.session(organization, session))?;
        if let Some(row) = ledger.sessions.iter_mut().find(|row| row.id == session) {
            row.model = model.to_owned();
            row.effort = effort.map(str::to_owned);
        }
        ledger.audit.push(audit.clone());
        Ok(())
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

    async fn remove_trailing_event(
        &self,
        organization: OrganizationId,
        session: SessionId,
        prefix: &str,
    ) -> Result<bool, StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.session(organization, session))?;
        let own = format!("{}:", session.as_uuid());
        let last = ledger
            .events
            .iter()
            .rposition(|(key, _)| key.starts_with(&own));
        let matches = last.is_some_and(|at| {
            ledger
                .events
                .get(at)
                .and_then(|(key, _)| key.strip_prefix(&own))
                .is_some_and(|rest| rest.starts_with(prefix))
        });
        match last {
            Some(at) if matches => {
                ledger.events.remove(at);
                Ok(true)
            }
            _ => Ok(false),
        }
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
}
