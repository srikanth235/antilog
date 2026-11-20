//! What the Lambda server runs on: Postgres and the receipt key from the environment. The
//! session machine and the code host are still fakes: the runner and a real code host are not
//! built.

use antilog_core::clock::SystemClock;
use antilog_core::deps::Deps;
use antilog_core::fakes::{FakeCodeHost, FakeMachine};
use antilog_core::judge::ed25519::Ed25519Signer;
use antilog_core::secrets::EnvSecrets;
use antilog_server::pg::{PgJudge, PgStore};
use antilog_types::OrganizationId;
use anyhow::Context;

pub struct AwsDeps {
    pub store: PgStore,
    machine: FakeMachine,
    signer: Ed25519Signer,
    code: FakeCodeHost,
}

impl AwsDeps {
    /// Everything is read from the environment the deploy set; nothing connects yet.
    pub async fn from_env(store: PgStore) -> anyhow::Result<Self> {
        let signer = Ed25519Signer::from_secrets(&EnvSecrets::from_env(), "receipt-key")
            .await
            .context("the receipt key")?;
        Ok(Self {
            store,
            machine: FakeMachine::default(),
            signer,
            code: FakeCodeHost::default(),
        })
    }
}

impl Deps for AwsDeps {
    type Store = PgStore;
    type Judge = PgJudge;
    type Machine = FakeMachine;
    type Signer = Ed25519Signer;
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
    fn signer(&self) -> &Ed25519Signer {
        &self.signer
    }
    fn clock(&self) -> &SystemClock {
        &SystemClock
    }
    fn code(&self) -> &FakeCodeHost {
        &self.code
    }
}
