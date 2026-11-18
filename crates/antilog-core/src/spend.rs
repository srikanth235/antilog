use antilog_types::{ExperimentId, OrganizationId, SessionId, Spend, SpendId, SpendKind};
use jiff::SignedDuration;

use crate::clock::Clock;
use crate::ids::uuid_of;
use crate::store::{BudgetStore, SpendScope, SpendTotal, StoreError};

/// One metered usage report from a platform. `seed` makes a replayed report the same row.
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
    experiment: ExperimentId,
) -> Result<SpendTotal, StoreError> {
    let to = clock
        .now()
        .checked_add(SignedDuration::from_hours(24 * 366))
        .map_or(i64::MAX / 4096, |t| t.as_millisecond());
    store
        .spend(organization, SpendScope::Experiment(experiment), 0, to)
        .await
}

#[derive(Debug, thiserror::Error)]
pub enum BudgetError {
    #[error(
        "budget exhausted: ${spent:.2} of ${cap:.2} is spent. Call request_budget with an amount and a reason"
    )]
    Exhausted { spent: f64, cap: f64 },
    #[error(transparent)]
    Store(#[from] StoreError),
}

fn dollars(cents: i64) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "cents shown as dollars")]
    let cents = cents as f64;
    cents / 100.0
}

/// Refuses when the experiment's budget is spent: no new session, no new measurement.
pub async fn ensure_budget<B: BudgetStore, C: Clock>(
    store: &B,
    clock: &C,
    organization: OrganizationId,
    experiment: ExperimentId,
    cap_cents: i64,
) -> Result<(), BudgetError> {
    let total = spent(store, clock, organization, experiment).await?;
    if total.cents >= cap_cents {
        return Err(BudgetError::Exhausted {
            spent: dollars(total.cents),
            cap: dollars(cap_cents),
        });
    }
    Ok(())
}
