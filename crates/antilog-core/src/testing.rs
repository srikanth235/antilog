use antilog_types::{Organization, OrganizationId};

use crate::store::{Store, StoreError};

#[expect(
    clippy::expect_used,
    reason = "test support: a failed call fails the suite"
)]
pub async fn store_suite<S: Store>(store: &S) {
    store.ping().await.expect("the store answers a ping");

    let organization = Organization {
        id: OrganizationId::new(),
        name: format!("acme-{}", OrganizationId::new().as_uuid()),
        created_at: 1_700_000_000_000,
    };
    store
        .create_organization(&organization)
        .await
        .expect("a new organization is created");
    assert_eq!(
        store.organization(organization.id).await.expect("read"),
        Some(organization.clone())
    );

    let duplicate = Organization {
        id: OrganizationId::new(),
        ..organization.clone()
    };
    assert!(matches!(
        store.create_organization(&duplicate).await,
        Err(StoreError::Conflict)
    ));

    assert_eq!(
        store
            .organization(OrganizationId::new())
            .await
            .expect("read"),
        None
    );
}
