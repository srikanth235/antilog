use antilog_types::{ApprovalId, OrganizationId};

use super::{FakeStore, locked};
use crate::store::{CardStore, StoreError};

impl CardStore for FakeStore {
    async fn mark_applied(
        &self,
        organization: OrganizationId,
        approval: ApprovalId,
    ) -> Result<bool, StoreError> {
        Ok(locked(&self.cards).told.insert((organization, approval)))
    }
}
