use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use antilog_types::{Organization, OrganizationId};
use jiff::{SignedDuration, Timestamp};
use sha2::{Digest, Sha256};

use crate::clock::Clock;
use crate::platform::{
    JudgeRun, JudgeSpec, Platform, PlatformError, SessionEvents, SessionHandle, SessionSpec,
};
use crate::secrets::{Secret, Secrets, SecretsError};
use crate::signer::{Signature, Signer};
use crate::store::{Store, StoreError};
use crate::timer::{JobId, Timer, TimerError};

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

pub struct FakeClock(Mutex<Timestamp>);

impl FakeClock {
    pub fn new(now: Timestamp) -> Self {
        Self(Mutex::new(now))
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
pub struct FakeStore(Mutex<HashMap<OrganizationId, Organization>>);

impl Store for FakeStore {
    async fn ping(&self) -> Result<(), StoreError> {
        Ok(())
    }

    async fn create_organization(&self, organization: &Organization) -> Result<(), StoreError> {
        let mut rows = locked(&self.0);
        let taken = rows
            .values()
            .any(|row| row.id == organization.id || row.name == organization.name);
        if taken {
            return Err(StoreError::Conflict);
        }
        rows.insert(organization.id, organization.clone());
        Ok(())
    }

    async fn organization(&self, id: OrganizationId) -> Result<Option<Organization>, StoreError> {
        Ok(locked(&self.0).get(&id).cloned())
    }
}

#[derive(Default)]
pub struct FakePlatform {
    state: Mutex<FakePlatformState>,
}

#[derive(Default)]
struct FakePlatformState {
    sessions: HashMap<String, Vec<String>>,
    next: u64,
}

impl Platform for FakePlatform {
    async fn start_session(&self, spec: &SessionSpec) -> Result<SessionHandle, PlatformError> {
        let mut state = locked(&self.state);
        state.next += 1;
        let handle = format!("fake-session-{}", state.next);
        state.sessions.insert(
            handle.clone(),
            vec![format!("started {}: {}", spec.agent, spec.prompt)],
        );
        Ok(SessionHandle(handle))
    }

    async fn session_events(
        &self,
        session: &SessionHandle,
        cursor: Option<&str>,
    ) -> Result<SessionEvents, PlatformError> {
        let state = locked(&self.state);
        let all = state
            .sessions
            .get(&session.0)
            .ok_or(PlatformError::UnknownSession)?;
        let from = cursor
            .and_then(|c| c.parse::<usize>().ok())
            .unwrap_or(0)
            .min(all.len());
        Ok(SessionEvents {
            events: all.get(from..).unwrap_or_default().to_vec(),
            next_cursor: Some(all.len().to_string()),
        })
    }

    async fn run_judge(&self, spec: &JudgeSpec) -> Result<JudgeRun, PlatformError> {
        Ok(JudgeRun {
            samples: (0..spec.repeats).map(|_| 1.0).collect(),
        })
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
    async fn wake_at(&self, job: JobId, at: Timestamp) -> Result<(), TimerError> {
        locked(&self.0).insert(job, at);
        Ok(())
    }

    async fn cancel(&self, job: JobId) -> Result<(), TimerError> {
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
    async fn fake_platform_streams_events_from_a_cursor() {
        let platform = FakePlatform::default();
        let handle = platform
            .start_session(&SessionSpec {
                agent: "a".into(),
                prompt: "go".into(),
            })
            .await
            .unwrap();
        let first = platform.session_events(&handle, None).await.unwrap();
        assert_eq!(first.events.len(), 1);
        let again = platform
            .session_events(&handle, first.next_cursor.as_deref())
            .await
            .unwrap();
        assert!(again.events.is_empty());
        let unknown = platform
            .session_events(&SessionHandle("x".into()), None)
            .await;
        assert!(matches!(unknown, Err(PlatformError::UnknownSession)));
    }

    #[tokio::test]
    async fn fake_timer_books_and_cancels() {
        let timer = FakeTimer::default();
        let at: Timestamp = "2026-11-18T00:10:00Z".parse().unwrap();
        timer.wake_at(JobId(1), at).await.unwrap();
        assert_eq!(timer.booked(), vec![(JobId(1), at)]);
        timer.cancel(JobId(1)).await.unwrap();
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
