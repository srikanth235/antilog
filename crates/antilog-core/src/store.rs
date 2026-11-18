use std::future::Future;

use antilog_types::{Organization, OrganizationId};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("already exists")]
    Conflict,
    #[error("store unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub trait Store: Send + Sync {
    fn ping(&self) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn create_organization(
        &self,
        organization: &Organization,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn organization(
        &self,
        id: OrganizationId,
    ) -> impl Future<Output = Result<Option<Organization>, StoreError>> + Send;
}
