use antilog_types::{ApprovalId, OrganizationId, SessionId};

use super::{FakeStore, locked};
use crate::store::{RelayStore, StoreError};

impl RelayStore for FakeStore {
    async fn relay_cursor(
        &self,
        organization: OrganizationId,
        session: SessionId,
    ) -> Result<Option<String>, StoreError> {
        Ok(locked(&self.relay)
            .cursors
            .get(&(organization, session))
            .cloned())
    }

    async fn save_relay_cursor(
        &self,
        organization: OrganizationId,
        session: SessionId,
        cursor: &str,
        _at: i64,
    ) -> Result<(), StoreError> {
        locked(&self.relay)
            .cursors
            .insert((organization, session), cursor.to_owned());
        Ok(())
    }

    async fn put_proposal(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
        body: &str,
    ) -> Result<(), StoreError> {
        let mut relay = locked(&self.relay);
        if relay.proposals.contains_key(&(organization, approval)) {
            return Err(StoreError::Conflict);
        }
        relay
            .proposals
            .insert((organization, approval), body.to_owned());
        Ok(())
    }

    async fn proposal(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
    ) -> Result<Option<String>, StoreError> {
        Ok(locked(&self.relay)
            .proposals
            .get(&(organization, approval))
            .cloned())
    }

    async fn mark_told(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
    ) -> Result<bool, StoreError> {
        Ok(locked(&self.relay).told.insert((organization, approval)))
    }
}
