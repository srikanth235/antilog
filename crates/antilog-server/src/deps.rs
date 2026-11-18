use antilog_core::clock::SystemClock;
use antilog_core::deps::Deps;
use antilog_core::fakes::{FakeCodeHost, FakePlatform, FakeSigner, FakeTimer};
use antilog_types::OrganizationId;

use crate::pg::{PgJudge, PgStore};

/// What the server runs on: Postgres for the ledger and the judge's tables, and the fakes
/// for the platform, timer, signer and code host until their real adapters exist.
pub struct ServerDeps {
    pub store: PgStore,
    pub platform: FakePlatform,
    pub signer: FakeSigner,
    pub timer: FakeTimer,
    pub code: FakeCodeHost,
}

impl ServerDeps {
    pub fn on_fakes(store: PgStore) -> Self {
        Self {
            store,
            platform: FakePlatform::default(),
            signer: FakeSigner::new("fake"),
            timer: FakeTimer::default(),
            code: FakeCodeHost::default(),
        }
    }
}

impl Deps for ServerDeps {
    type Store = PgStore;
    type Judge = PgJudge;
    type Platform = FakePlatform;
    type Signer = FakeSigner;
    type Clock = SystemClock;
    type Timer = FakeTimer;
    type Code = FakeCodeHost;

    fn store(&self) -> &PgStore {
        &self.store
    }
    fn judge(&self, organization: OrganizationId) -> PgJudge {
        self.store.judge(organization)
    }
    fn platform(&self) -> &FakePlatform {
        &self.platform
    }
    fn signer(&self) -> &FakeSigner {
        &self.signer
    }
    fn clock(&self) -> &SystemClock {
        &SystemClock
    }
    fn timer(&self) -> &FakeTimer {
        &self.timer
    }
    fn code(&self) -> &FakeCodeHost {
        &self.code
    }
}
