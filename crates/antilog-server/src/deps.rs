use antilog_core::clock::SystemClock;
use antilog_core::deps::Deps;
use std::time::Duration;

use antilog_core::fakes::{FakeCodeHost, FakeSigner};
use antilog_types::OrganizationId;

use crate::pg::{PgJudge, PgStore};
use crate::queue::QueueMachine;

/// What the server runs on: Postgres for the ledger, the judge's tables, the runner queue and
/// session credentials (so a token minted at claim verifies at the proxy and the MCP endpoint),
/// and the fakes for the signer and the code host until their real adapters exist.
pub struct ServerDeps {
    pub store: PgStore,
    pub machine: QueueMachine<PgStore>,
    pub signer: FakeSigner,
    pub code: FakeCodeHost,
}

/// How long the judge waits for a command it sent to a session's machine.
const MACHINE_TIMEOUT: Duration = Duration::from_secs(30 * 60);

impl ServerDeps {
    pub fn on_fakes(store: PgStore) -> Self {
        Self {
            machine: QueueMachine::new(store.clone(), MACHINE_TIMEOUT),
            signer: FakeSigner::new("fake"),
            code: FakeCodeHost::default(),
            store,
        }
    }
}

impl Deps for ServerDeps {
    type Store = PgStore;
    type Judge = PgJudge;
    type Machine = QueueMachine<PgStore>;
    type Signer = FakeSigner;
    type Clock = SystemClock;
    type Code = FakeCodeHost;
    type Credentials = PgStore;

    fn store(&self) -> &PgStore {
        &self.store
    }
    fn judge(&self, organization: OrganizationId) -> PgJudge {
        self.store.judge(organization)
    }
    fn machine(&self) -> &QueueMachine<PgStore> {
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
    fn credentials(&self) -> &PgStore {
        &self.store
    }
}
