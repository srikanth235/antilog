#![expect(
    clippy::expect_used,
    reason = "test support: a failed call fails the test"
)]
//! What every `SessionCredentials` and `ModelKeyStore` must do, run against the fake and Postgres.

use antilog_types::{AuditEntry, AuditOutcome, ModelProvider, Session};
use jiff::SignedDuration;

use crate::credentials::{Purpose, SessionCredentials};
use crate::secrets::Secret;
use crate::seed::{self, T0};
use crate::store::{GatewayScope, Store, StoreError};

async fn session_of<S: Store>(store: &S, seed: &seed::Seed) -> Session {
    let session = seed::session(seed, "opencode", T0);
    store
        .create_session(
            &session,
            &AuditEntry::new(
                seed.organization.id,
                Some(seed.owner.id),
                "session.start",
                session.id.to_string(),
                AuditOutcome::Done,
                T0,
            ),
        )
        .await
        .expect("a session is created");
    session
}

/// Tokens are found by hash for their own purpose until they expire or are revoked.
pub async fn credentials_suite<S: Store, C: SessionCredentials>(store: &S, creds: &C) {
    let seed = seed::organization(store).await;
    let one = session_of(store, &seed).await;
    let two = session_of(store, &seed).await;
    let ttl = SignedDuration::from_mins(10);

    let model = creds.mint(one.id, Purpose::Model, ttl).await.expect("mint");
    let mcp = creds.mint(one.id, Purpose::Mcp, ttl).await.expect("mint");
    let other = creds.mint(two.id, Purpose::Model, ttl).await.expect("mint");
    assert_ne!(model, mcp);

    assert_eq!(
        creds.verify(&model, Purpose::Model).await.expect("verify"),
        Some(one.id)
    );
    assert_eq!(
        creds.verify(&other, Purpose::Model).await.expect("verify"),
        Some(two.id)
    );
    // A token opens only what it was minted for.
    assert_eq!(
        creds.verify(&model, Purpose::Mcp).await.expect("verify"),
        None
    );
    assert_eq!(
        creds.verify(&mcp, Purpose::Model).await.expect("verify"),
        None
    );
    assert_eq!(
        creds.verify(&model, Purpose::Runner).await.expect("verify"),
        None
    );
    assert_eq!(
        creds
            .verify("alc_nothing", Purpose::Model)
            .await
            .expect("verify"),
        None
    );
    assert_eq!(
        creds.verify("", Purpose::Model).await.expect("verify"),
        None
    );

    // One that is already past its time is refused.
    let stale = creds
        .mint(one.id, Purpose::Model, SignedDuration::from_secs(-1))
        .await
        .expect("mint");
    assert_eq!(
        creds.verify(&stale, Purpose::Model).await.expect("verify"),
        None
    );

    // Revoking a session ends all of its tokens and none of another's.
    creds.revoke_all(one.id).await.expect("revoke");
    assert_eq!(
        creds.verify(&model, Purpose::Model).await.expect("verify"),
        None
    );
    assert_eq!(
        creds.verify(&mcp, Purpose::Mcp).await.expect("verify"),
        None
    );
    assert_eq!(
        creds.verify(&other, Purpose::Model).await.expect("verify"),
        Some(two.id)
    );
    // Revoking again, or a session with none, is not an error.
    creds.revoke_all(one.id).await.expect("revoke");
}

/// Keys are kept per organization and provider, replaced in place, removed on request, and
/// found for the gateway by session.
pub async fn model_key_suite<S: Store>(store: &S) {
    let seed = seed::organization(store).await;
    let other = seed::organization(store).await;
    let org = seed.organization.id;
    let audit = |action: &str| {
        AuditEntry::new(
            org,
            Some(seed.owner.id),
            action,
            "anthropic",
            AuditOutcome::Done,
            T0,
        )
    };
    assert!(store.model_keys(org).await.expect("list").is_empty());
    assert_eq!(
        store
            .model_key(org, ModelProvider::Anthropic)
            .await
            .expect("get")
            .map(|s| s.expose().to_owned()),
        None
    );

    store
        .put_model_key(
            org,
            ModelProvider::Anthropic,
            &Secret::new("sk-ant-one"),
            T0,
            &audit("model_key.set"),
        )
        .await
        .expect("put");
    store
        .put_model_key(
            org,
            ModelProvider::Openai,
            &Secret::new("sk-oai-one"),
            T0 + 1,
            &audit("model_key.set"),
        )
        .await
        .expect("put");
    store
        .put_model_key(
            org,
            ModelProvider::OpencodeGo,
            &Secret::new("oc_sk_go"),
            T0 + 2,
            &audit("model_key.set"),
        )
        .await
        .expect("put");
    assert_eq!(
        store
            .model_key(org, ModelProvider::OpencodeGo)
            .await
            .expect("read")
            .map(|key| key.expose().to_owned())
            .as_deref(),
        Some("oc_sk_go")
    );
    let listed = store.model_keys(org).await.expect("list");
    assert_eq!(
        listed
            .iter()
            .map(|k| (k.provider, k.set_at))
            .collect::<Vec<_>>(),
        vec![
            (ModelProvider::Anthropic, T0),
            (ModelProvider::Openai, T0 + 1),
            (ModelProvider::OpencodeGo, T0 + 2)
        ]
    );

    // Replacing keeps one row and the new value.
    store
        .put_model_key(
            org,
            ModelProvider::Anthropic,
            &Secret::new("sk-ant-two"),
            T0 + 2,
            &audit("model_key.set"),
        )
        .await
        .expect("replace");
    let got = store
        .model_key(org, ModelProvider::Anthropic)
        .await
        .expect("get");
    assert_eq!(
        got.map(|s| s.expose().to_owned()).as_deref(),
        Some("sk-ant-two")
    );
    assert_eq!(store.model_keys(org).await.expect("list").len(), 3);

    // Another organization sees none of it.
    let foreign = other.organization.id;
    assert!(store.model_keys(foreign).await.expect("list").is_empty());
    assert!(
        store
            .model_key(foreign, ModelProvider::Anthropic)
            .await
            .expect("get")
            .is_none()
    );

    // Removing is audited, and removing what is not there is NotFound.
    store
        .remove_model_key(org, ModelProvider::Openai, &audit("model_key.remove"))
        .await
        .expect("remove");
    assert!(
        store
            .model_key(org, ModelProvider::Openai)
            .await
            .expect("get")
            .is_none()
    );
    assert!(matches!(
        store
            .remove_model_key(org, ModelProvider::Openai, &audit("model_key.remove"))
            .await,
        Err(StoreError::NotFound)
    ));
    let log = store.audit_log(org, None, 1000).await.expect("audit");
    assert_eq!(
        log.iter().filter(|e| e.action == "model_key.set").count(),
        4
    );
    assert_eq!(
        log.iter()
            .filter(|e| e.action == "model_key.remove")
            .count(),
        1
    );

    // The gateway finds a session's organization from its id alone.
    let session = session_of(store, &seed).await;
    assert_eq!(
        store.gateway_scope(session.id).await.expect("scope"),
        Some(GatewayScope { organization: org })
    );
    assert_eq!(
        store
            .gateway_scope(antilog_types::SessionId::new())
            .await
            .expect("scope"),
        None
    );
}
