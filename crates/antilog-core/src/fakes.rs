use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use jiff::{SignedDuration, Timestamp};
use sha2::{Digest, Sha256};

mod approval;
mod budget;
mod ledger;
mod organization;
mod platform;
mod relay;
mod session;
mod world;

pub use platform::FakePlatform;
pub use world::{FakeCodeHost, FakeDeps};

use crate::clock::Clock;
use crate::secrets::{Secret, Secrets, SecretsError};
use crate::signer::{Signature, Signer};
use crate::store::{Store, StoreError};
use crate::timer::{JobId, Timer, TimerError};

mod relay_state {
    use std::collections::{HashMap, HashSet};

    use antilog_types::{ApprovalId, OrganizationId, SessionId};

    #[derive(Default)]
    pub struct Relay {
        pub cursors: HashMap<(OrganizationId, SessionId), String>,
        pub proposals: HashMap<(OrganizationId, ApprovalId), String>,
        pub told: HashSet<(OrganizationId, ApprovalId)>,
    }
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

pub struct FakeClock(Mutex<Timestamp>);

impl FakeClock {
    pub fn new(now: Timestamp) -> Self {
        Self(Mutex::new(now))
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
    ledger: Mutex<ledger::Ledger>,
    relay: Mutex<relay_state::Relay>,
}

impl Store for FakeStore {
    async fn ping(&self) -> Result<(), StoreError> {
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeTimer(Mutex<HashMap<JobId, Timestamp>>);

impl FakeTimer {
    pub fn booked(&self) -> Vec<(JobId, Timestamp)> {
        let mut booked: Vec<_> = locked(&self.0)
            .iter()
            .map(|(job, at)| (*job, *at))
            .collect();
        booked.sort_by_key(|(job, at)| (*at, job.0));
        booked
    }
}

impl Timer for FakeTimer {
    async fn wake_at(
        &self,
        _: antilog_types::OrganizationId,
        job: JobId,
        at: Timestamp,
    ) -> Result<(), TimerError> {
        locked(&self.0).insert(job, at);
        Ok(())
    }

    async fn cancel(&self, _: antilog_types::OrganizationId, job: JobId) -> Result<(), TimerError> {
        locked(&self.0).remove(&job);
        Ok(())
    }
}

pub struct FakeSigner {
    key_id: String,
}

impl FakeSigner {
    pub fn new(key_id: impl Into<String>) -> Self {
        Self {
            key_id: key_id.into(),
        }
    }

    fn digest(&self, message: &[u8]) -> Vec<u8> {
        Sha256::new()
            .chain_update(self.key_id.as_bytes())
            .chain_update(message)
            .finalize()
            .to_vec()
    }
}

impl Signer for FakeSigner {
    fn key_id(&self) -> &str {
        &self.key_id
    }

    fn sign(&self, message: &[u8]) -> Signature {
        Signature(self.digest(message))
    }

    fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        signature.0 == self.digest(message)
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
    async fn fake_store_keeps_the_relay_state() {
        crate::seed::relay_suite(&FakeStore::default()).await;
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
    async fn fake_timer_books_and_cancels() {
        let timer = FakeTimer::default();
        let at: Timestamp = "2026-11-18T00:10:00Z".parse().unwrap();
        timer
            .wake_at(antilog_types::OrganizationId::new(), JobId(1), at)
            .await
            .unwrap();
        assert_eq!(timer.booked(), vec![(JobId(1), at)]);
        timer
            .cancel(antilog_types::OrganizationId::new(), JobId(1))
            .await
            .unwrap();
        assert!(timer.booked().is_empty());
    }

    #[test]
    fn fake_signer_rejects_a_changed_message_and_a_foreign_key() {
        let signer = FakeSigner::new("k1");
        let signature = signer.sign(b"receipt");
        assert!(signer.verify(b"receipt", &signature));
        assert!(!signer.verify(b"receipt!", &signature));
        assert!(!FakeSigner::new("k2").verify(b"receipt", &signature));
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
