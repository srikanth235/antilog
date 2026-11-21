use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use jiff::{SignedDuration, Timestamp};

mod approval;
mod budget;
mod card;
mod changes;
mod connectors;
mod context;
mod credentials;
mod customize;
mod environment;
pub(crate) mod ledger;
mod model_key;
mod organization;
mod runner;
mod session;
mod world;

pub use connectors::{Call, FakeConnectors};
pub use credentials::FakeCredentials;
pub use world::FakeDeps;

use crate::clock::Clock;
use crate::secrets::{Secret, Secrets, SecretsError};
use crate::store::{Store, StoreError};

mod card_state {
    use std::collections::HashSet;

    use antilog_types::{ApprovalId, OrganizationId};

    #[derive(Default)]
    pub struct Cards {
        pub told: HashSet<(OrganizationId, ApprovalId)>,
    }
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Clones share the same time.
#[derive(Clone)]
pub struct FakeClock(std::sync::Arc<Mutex<Timestamp>>);

impl FakeClock {
    pub fn new(now: Timestamp) -> Self {
        Self(std::sync::Arc::new(Mutex::new(now)))
    }

    pub fn now_plus(&self, by: std::time::Duration) -> Timestamp {
        let now = *locked(&self.0);
        SignedDuration::try_from(by)
            .ok()
            .and_then(|d| now.checked_add(d).ok())
            .unwrap_or(now)
    }

    pub fn advance(&self, by: SignedDuration) {
        let mut now = locked(&self.0);
        *now = now.checked_add(by).unwrap_or(*now);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Timestamp {
        *locked(&self.0)
    }
}

#[derive(Default)]
pub struct FakeStore {
    ledger: std::sync::Arc<Mutex<ledger::Ledger>>,
    cards: Mutex<card_state::Cards>,
    runner: Mutex<runner::State>,
    changes: Mutex<changes::State>,
    context: Mutex<context::State>,
    vault: Mutex<model_key::Vault>,
    customize: Mutex<customize::State>,
}

impl Store for FakeStore {
    async fn ping(&self) -> Result<(), StoreError> {
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeSecrets(Mutex<HashMap<String, Secret>>);

impl FakeSecrets {
    pub fn with(self, name: &str, value: &str) -> Self {
        locked(&self.0).insert(name.to_owned(), Secret::new(value));
        self
    }
}

impl Secrets for FakeSecrets {
    async fn get(&self, name: &str) -> Result<Secret, SecretsError> {
        locked(&self.0)
            .get(name)
            .cloned()
            .ok_or_else(|| SecretsError::NotFound(name.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::store_suite;

    #[tokio::test]
    async fn fake_store_passes_the_store_suite() {
        store_suite(&FakeStore::default()).await;
    }

    #[tokio::test]
    async fn fake_store_leases_sessions_to_a_runner() {
        crate::seed::runner_suite(&FakeStore::default()).await;
    }

    #[tokio::test]
    async fn fake_store_locks_a_runner_to_one_organization() {
        crate::seed::runner_lock_suite(&FakeStore::default()).await;
    }

    #[tokio::test]
    async fn fake_store_limits_the_session_machine() {
        crate::seed::runner_limits_suite(&FakeStore::default()).await;
    }

    #[tokio::test]
    async fn fake_store_keeps_model_keys_and_finds_a_session_for_the_gateway() {
        crate::gateway_suite::model_key_suite(&FakeStore::default()).await;
    }

    #[tokio::test]
    async fn fake_credentials_pass_the_credentials_suite() {
        let store = FakeStore::default();
        let creds = FakeCredentials::new(std::sync::Arc::new(crate::clock::SystemClock));
        crate::gateway_suite::credentials_suite(&store, &creds).await;
    }

    #[tokio::test]
    async fn fake_store_keeps_the_card_state() {
        crate::seed::card_suite(&FakeStore::default()).await;
    }

    #[tokio::test]
    async fn fake_store_keeps_the_context_window() {
        crate::seed::context_suite(&FakeStore::default()).await;
    }

    #[test]
    fn fake_clock_moves_only_when_advanced() {
        let start: Timestamp = "2026-11-18T00:00:00Z".parse().unwrap();
        let clock = FakeClock::new(start);
        assert_eq!(clock.now(), start);
        clock.advance(SignedDuration::from_secs(60));
        assert_eq!(
            clock.now(),
            start.checked_add(SignedDuration::from_secs(60)).unwrap()
        );
    }

    #[tokio::test]
    async fn secrets_are_found_and_never_printed() {
        let secrets = FakeSecrets::default().with("token", "hunter2");
        let secret = secrets.get("token").await.unwrap();
        assert_eq!(secret.expose(), "hunter2");
        assert!(!format!("{secret:?} {secret}").contains("hunter2"));
        assert!(matches!(
            secrets.get("none").await,
            Err(SecretsError::NotFound(_))
        ));
    }
}
