use antilog_core::clock::SystemClock;
use antilog_core::deps::Deps;
use antilog_core::fakes::{FakeCodeHost, FakeMachine, FakeSigner};
use antilog_types::OrganizationId;

use crate::pg::{PgJudge, PgStore};

/// What the server runs on: Postgres for the ledger and the judge's tables, and the fakes
/// for the session machine, signer and code host until their real adapters exist.
pub struct ServerDeps {
    pub store: PgStore,
    pub machine: FakeMachine,
    pub signer: FakeSigner,
    pub code: FakeCodeHost,
}

impl ServerDeps {
    pub fn on_fakes(store: PgStore) -> Self {
        Self {
            store,
            machine: FakeMachine::default(),
            signer: FakeSigner::new("fake"),
            code: FakeCodeHost::default(),
        }
    }
}

impl Deps for ServerDeps {
    type Store = PgStore;
    type Judge = PgJudge;
    type Machine = FakeMachine;
    type Signer = FakeSigner;
    type Clock = SystemClock;
    type Code = FakeCodeHost;

    fn store(&self) -> &PgStore {
        &self.store
    }
    fn judge(&self, organization: OrganizationId) -> PgJudge {
        self.store.judge(organization)
    }
    fn machine(&self) -> &FakeMachine {
        &self.machine
    }
    fn signer(&self) -> &FakeSigner {
        &self.signer
    }
    fn clock(&self) -> &SystemClock {
        &SystemClock
    }
    fn code(&self) -> &FakeCodeHost {
        &self.code
    }
}
