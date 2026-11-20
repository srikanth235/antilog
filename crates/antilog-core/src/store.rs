pub mod judge;

use std::future::Future;

mod approval;
mod budget;
mod card;
mod code_host;
mod organization;
mod session;

pub use approval::ApprovalStore;
pub use budget::{BudgetStore, SpendLine, SpendRow, SpendScope, SpendTotal};
pub use card::CardStore;
pub use code_host::{CodeHostStore, PullRequestRow};
pub use organization::{Membership, OrganizationStore, SignInRecord, TokenRecord};
pub use session::{NewEvent, SessionChange, SessionStore};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Invariant {
    #[error("an organization or a project needs at least one owner")]
    LastOwner,
    #[error("a budget or cap would pass the cap above it")]
    OverCap,
    #[error("that state change is not allowed")]
    IllegalTransition,
    #[error("a spawned session cannot spawn")]
    SpawnedCannotSpawn,
    #[error("add credential is never pre-approved")]
    AlwaysAsk,
    #[error("the audit log and the spend ledger are append-only")]
    AppendOnly,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("already exists")]
    Conflict,
    #[error("not found in this organization")]
    NotFound,
    #[error(transparent)]
    Invariant(#[from] Invariant),
    #[error("store unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub trait Store:
    OrganizationStore
    + SessionStore
    + BudgetStore
    + ApprovalStore
    + CardStore
    + CodeHostStore
    + Send
    + Sync
{
    fn ping(&self) -> impl Future<Output = Result<(), StoreError>> + Send;
}
