use antilog_types::{ApprovalId, OrganizationId};

use super::{FakeStore, locked};
use crate::store::{CardStore, StoreError};

impl CardStore for FakeStore {
    async fn put_proposal(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
        body: &str,
    ) -> Result<(), StoreError> {
        let mut cards = locked(&self.cards);
        if cards.proposals.contains_key(&(organization, approval)) {
            return Err(StoreError::Conflict);
        }
        cards
            .proposals
            .insert((organization, approval), body.to_owned());
        Ok(())
    }

    async fn proposal(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
    ) -> Result<Option<String>, StoreError> {
        Ok(locked(&self.cards)
            .proposals
            .get(&(organization, approval))
            .cloned())
    }

    async fn mark_applied(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
    ) -> Result<bool, StoreError> {
        Ok(locked(&self.cards).told.insert((organization, approval)))
    }
}
