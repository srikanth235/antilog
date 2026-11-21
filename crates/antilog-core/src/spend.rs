use antilog_types::{OrganizationId, SessionId, Spend, SpendId, SpendKind};
use jiff::SignedDuration;

use crate::clock::Clock;
use crate::ids::uuid_of;
use crate::store::{BudgetStore, SpendScope, SpendTotal, StoreError};

/// One metered usage report from the model gateway. `seed` makes a replayed report the same row.
pub async fn record_usage<B: BudgetStore>(
    store: &B,
    organization: OrganizationId,
    session: SessionId,
    cents: i64,
    tokens: i64,
    at: i64,
    seed: &str,
) -> Result<(), StoreError> {
    let spend = Spend {
        id: SpendId::from_uuid(uuid_of(&format!("spend:{session}:{seed}"))),
        organization_id: organization,
        session_id: session,
        kind: SpendKind::Tokens,
        amount_cents: cents.max(0),
        tokens: tokens.max(0),
        occurred_at: at,
    };
    match store.record_spend(&spend).await {
        Err(StoreError::Conflict) => Ok(()),
        other => other,
    }
}

pub async fn spent<B: BudgetStore, C: Clock>(
    store: &B,
    clock: &C,
    organization: OrganizationId,
    session: SessionId,
) -> Result<SpendTotal, StoreError> {
    let to = clock
        .now()
        .checked_add(SignedDuration::from_hours(24 * 366))
        .map_or(i64::MAX / 4096, |t| t.as_millisecond());
    store
        .spend(organization, SpendScope::Session(session), 0, to)
        .await
}
