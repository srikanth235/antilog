use antilog_types::{AuditEntry, Environment, OrganizationId};

use super::{FakeStore, ledger::not_found, locked};
use crate::store::{EnvironmentStore, Invariant, StoreError};

impl EnvironmentStore for FakeStore {
    async fn add_environment(
        &self,
        organization: OrganizationId,
        environment: &Environment,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        if ledger
            .environments
            .iter()
            .any(|(org, row)| *org == organization && row.id == environment.id)
        {
            return Err(StoreError::Conflict);
        }
        ledger
            .environments
            .push((organization, environment.clone()));
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn add_environment_version(
        &self,
        organization: OrganizationId,
        environment: &Environment,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        let newest = not_found(
            ledger
                .environments
                .iter()
                .filter(|(org, row)| *org == organization && row.id == environment.id)
                .map(|(_, row)| row)
                .max_by_key(|row| row.version),
        )?;
        if newest.archived {
            return Err(Invariant::IllegalTransition.into());
        }
        if environment.version != newest.version + 1 {
            return Err(StoreError::Conflict);
        }
        ledger
            .environments
            .push((organization, environment.clone()));
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn archive_environment(
        &self,
        organization: OrganizationId,
        id: &str,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        let mut found = false;
        for (_, row) in ledger
            .environments
            .iter_mut()
            .filter(|(org, row)| *org == organization && row.id == id)
        {
            row.archived = true;
            found = true;
        }
        if !found {
            return Err(StoreError::NotFound);
        }
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn environments(
        &self,
        organization: OrganizationId,
    ) -> Result<Vec<Environment>, StoreError> {
        let ledger = locked(&self.ledger);
        let mut newest: Vec<Environment> = Vec::new();
        for (_, row) in ledger
            .environments
            .iter()
            .filter(|(org, _)| *org == organization)
        {
            match newest.iter_mut().find(|kept| kept.id == row.id) {
                Some(kept) if kept.version < row.version => *kept = row.clone(),
                Some(_) => {}
                None => newest.push(row.clone()),
            }
        }
        newest.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(newest)
    }

    async fn environment_version(
        &self,
        organization: OrganizationId,
        id: &str,
        version: u32,
    ) -> Result<Option<Environment>, StoreError> {
        Ok(locked(&self.ledger)
            .environments
            .iter()
            .find(|(org, row)| *org == organization && row.id == id && row.version == version)
            .map(|(_, row)| row.clone()))
    }
}
