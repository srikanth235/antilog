use std::collections::HashMap;

use antilog_types::{ModelKeyStatus, ModelProvider, OrganizationId, SessionId};

use super::{FakeStore, ledger::not_found, locked};
use crate::sealed::Sealer;
use crate::secrets::Secret;
use crate::store::{GatewayScope, ModelKeyStore, StoreError};

/// Sealed keys by organization and provider, as the real store keeps them: bytes, not text.
pub(super) struct Vault {
    sealer: Sealer,
    rows: HashMap<(OrganizationId, ModelProvider), (Vec<u8>, i64)>,
}

impl Default for Vault {
    fn default() -> Self {
        let master = Secret::new("42".repeat(32));
        match Sealer::new(&master) {
            Ok(sealer) => Self {
                sealer,
                rows: HashMap::new(),
            },
            Err(error) => unreachable!("the test master key is valid: {error}"),
        }
    }
}

fn row(organization: OrganizationId, provider: ModelProvider) -> String {
    format!("model_key:{}:{}", organization.as_uuid(), provider.as_str())
}

fn sealing(error: crate::sealed::SealedError) -> StoreError {
    StoreError::Unavailable(Box::new(error))
}

impl ModelKeyStore for FakeStore {
    async fn put_model_key(
        &self,
        organization: OrganizationId,
        provider: ModelProvider,
        key: &Secret,
        at: i64,
        audit: &antilog_types::AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.organization(organization))?;
        let mut vault = locked(&self.vault);
        let sealed = vault
            .sealer
            .seal(key.expose(), &row(organization, provider))
            .map_err(sealing)?;
        vault.rows.insert((organization, provider), (sealed, at));
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn remove_model_key(
        &self,
        organization: OrganizationId,
        provider: ModelProvider,
        audit: &antilog_types::AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(locked(&self.vault).rows.remove(&(organization, provider)))?;
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn model_keys(
        &self,
        organization: OrganizationId,
    ) -> Result<Vec<ModelKeyStatus>, StoreError> {
        let vault = locked(&self.vault);
        Ok(ModelProvider::ALL
            .into_iter()
            .filter_map(|provider| {
                vault
                    .rows
                    .get(&(organization, provider))
                    .map(|(_, at)| ModelKeyStatus {
                        provider,
                        set_at: *at,
                    })
            })
            .collect())
    }

    async fn model_key(
        &self,
        organization: OrganizationId,
        provider: ModelProvider,
    ) -> Result<Option<Secret>, StoreError> {
        let vault = locked(&self.vault);
        vault
            .rows
            .get(&(organization, provider))
            .map(|(sealed, _)| {
                vault
                    .sealer
                    .open(sealed, &row(organization, provider))
                    .map_err(sealing)
            })
            .transpose()
    }

    async fn gateway_scope(&self, session: SessionId) -> Result<Option<GatewayScope>, StoreError> {
        Ok(locked(&self.ledger)
            .sessions
            .iter()
            .find(|row| row.id == session)
            .map(|row| GatewayScope {
                organization: row.organization_id,
                experiment: row.experiment_id,
            }))
    }
}

impl FakeStore {
    /// The sealed bytes kept for a key, for a test that checks the key is not stored as text.
    pub fn sealed_model_key(
        &self,
        organization: OrganizationId,
        provider: ModelProvider,
    ) -> Option<Vec<u8>> {
        locked(&self.vault)
            .rows
            .get(&(organization, provider))
            .map(|(sealed, _)| sealed.clone())
    }
}
