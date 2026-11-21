use antilog_types::{AuditEntry, Environment, NetworkSecret, OrganizationId};

use super::{FakeStore, ledger::not_found, locked};
use crate::secrets::Secret;
use crate::store::{EnvironmentStore, Invariant, SecretValue, StoreError};

/// Files a version's secret values; one with no value takes the previous version's.
fn keep_secrets(
    ledger: &mut super::ledger::Ledger,
    organization: OrganizationId,
    environment: &Environment,
    secrets: &[SecretValue],
) -> Result<(), StoreError> {
    let mut filed = Vec::new();
    for secret in secrets {
        let value = match &secret.value {
            Some(value) => value.clone(),
            None => ledger
                .environment_secrets
                .iter()
                .rev()
                .find(|(org, id, version, host, header, _)| {
                    *org == organization
                        && *id == environment.id
                        && *version < environment.version
                        && *host == secret.host
                        && header.eq_ignore_ascii_case(&secret.header)
                })
                .map(|row| row.5.clone())
                .ok_or(StoreError::NotFound)?,
        };
        filed.push((
            organization,
            environment.id.clone(),
            environment.version,
            secret.host.clone(),
            secret.header.clone(),
            value,
        ));
    }
    ledger.environment_secrets.extend(filed);
    Ok(())
}

impl EnvironmentStore for FakeStore {
    async fn add_environment(
        &self,
        organization: OrganizationId,
        environment: &Environment,
        secrets: &[SecretValue],
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
        keep_secrets(&mut ledger, organization, environment, secrets)?;
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
        secrets: &[SecretValue],
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
        keep_secrets(&mut ledger, organization, environment, secrets)?;
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

    async fn network_secret_values(
        &self,
        organization: OrganizationId,
        id: &str,
        version: u32,
    ) -> Result<Vec<(NetworkSecret, Secret)>, StoreError> {
        let ledger = locked(&self.ledger);
        let Some((_, row)) = ledger
            .environments
            .iter()
            .find(|(org, row)| *org == organization && row.id == id && row.version == version)
        else {
            return Ok(Vec::new());
        };
        Ok(row
            .network_secrets
            .iter()
            .filter_map(|meta| {
                ledger
                    .environment_secrets
                    .iter()
                    .find(|(org, eid, v, host, header, _)| {
                        *org == organization
                            && eid == id
                            && *v == version
                            && *host == meta.host
                            && *header == meta.header
                    })
                    .map(|found| (meta.clone(), found.5.clone()))
            })
            .collect())
    }
}
