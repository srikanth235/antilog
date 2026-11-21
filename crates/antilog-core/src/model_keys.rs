//! The model keys an organization brings: one per provider, kept sealed, never shown again.
//! The proxy uses them (`docs/V0.md`); a session never holds one.

use antilog_types::{AuditOutcome, ModelKeyStatus, ModelProvider, NewModelKey};

use crate::organization::{Action, Principal, Refusal, entry, member_of, permit};
use crate::secrets::Secret;
use crate::store::Store;

/// Every member sees which providers have a key. Never a key.
pub async fn list<S: Store>(store: &S, who: &Principal) -> Result<Vec<ModelKeyStatus>, Refusal> {
    member_of(store, who).await?;
    Ok(store.model_keys(who.organization).await?)
}

/// Only an organization owner sets or replaces a key. It is audited by provider, not by value.
pub async fn set<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    provider: ModelProvider,
    new: NewModelKey,
) -> Result<ModelKeyStatus, Refusal> {
    let key = new.key.trim();
    if key.is_empty() || key.len() > 512 || key.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(Refusal::Invalid(
            "a model key is 1 to 512 characters with no spaces",
        ));
    }
    permit(
        store,
        who,
        Action::ModelKeySet,
        provider.as_str(),
        now,
    )
    .await?;
    store
        .put_model_key(
            who.organization,
            provider,
            &Secret::new(key),
            now,
            &entry(
                who,
                "model_key.set",
                provider.as_str(),
                AuditOutcome::Done,
                now,
            ),
        )
        .await?;
    Ok(ModelKeyStatus {
        provider,
        set_at: now,
    })
}

/// Only an organization owner removes a key; sessions that need it are refused until one is set.
pub async fn remove<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    provider: ModelProvider,
) -> Result<(), Refusal> {
    permit(
        store,
        who,
        Action::ModelKeyRemove,
        provider.as_str(),
        now,
    )
    .await?;
    store
        .remove_model_key(
            who.organization,
            provider,
            &entry(
                who,
                "model_key.remove",
                provider.as_str(),
                AuditOutcome::Done,
                now,
            ),
        )
        .await?;
    Ok(())
}
