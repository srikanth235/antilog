use std::sync::Arc;

use super::{FakeClock, FakeCredentials, FakeStore};
use crate::deps::Deps;

pub struct FakeDeps {
    pub store: FakeStore,
    pub clock: FakeClock,
    pub credentials: FakeCredentials,
}

impl Default for FakeDeps {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeDeps {
    pub fn new() -> Self {
        let store = FakeStore::default();
        let clock = FakeClock::new("2026-11-18T09:00:00Z".parse().unwrap_or_default());
        Self {
            store,
            credentials: FakeCredentials::new(Arc::new(clock.clone())),
            clock,
        }
    }
}

impl Deps for FakeDeps {
    type Store = FakeStore;
    type Clock = FakeClock;
    type Credentials = FakeCredentials;

    fn store(&self) -> &FakeStore {
        &self.store
    }
    fn clock(&self) -> &FakeClock {
        &self.clock
    }
    fn credentials(&self) -> &FakeCredentials {
        &self.credentials
    }
}
