//! What the Lambda server runs on: Postgres and the receipt key from the environment. The
//! session machine is the runner queue; the code host and session credentials are still fakes.

use antilog_core::clock::SystemClock;
use antilog_core::deps::Deps;
use std::sync::Arc;
use std::time::Duration;

use antilog_core::fakes::{FakeCodeHost, FakeCredentials};
use antilog_core::judge::ed25519::Ed25519Signer;
use antilog_core::secrets::EnvSecrets;
use antilog_server::pg::{PgJudge, PgStore};
use antilog_server::queue::QueueMachine;
use antilog_types::OrganizationId;
use anyhow::Context;

pub struct AwsDeps {
    pub store: PgStore,
    machine: QueueMachine<PgStore>,
    signer: Ed25519Signer,
    code: FakeCodeHost,
    credentials: FakeCredentials,
}

/// A request lives at most 15 minutes, so the judge waits for a machine less than that.
const MACHINE_TIMEOUT: Duration = Duration::from_secs(10 * 60);

impl AwsDeps {
    /// Everything is read from the environment the deploy set; nothing connects yet.
    pub async fn from_env(store: PgStore) -> anyhow::Result<Self> {
        let signer = Ed25519Signer::from_secrets(&EnvSecrets::from_env(), "receipt-key")
            .await
            .context("the receipt key")?;
        Ok(Self {
            machine: QueueMachine::new(store.clone(), MACHINE_TIMEOUT),
            credentials: FakeCredentials::new(Arc::new(SystemClock)),
            store,
            signer,
            code: FakeCodeHost::default(),
        })
    }
}

impl Deps for AwsDeps {
    type Store = PgStore;
    type Judge = PgJudge;
    type Machine = QueueMachine<PgStore>;
    type Signer = Ed25519Signer;
    type Clock = SystemClock;
    type Code = FakeCodeHost;
    type Credentials = FakeCredentials;

    fn store(&self) -> &PgStore {
        &self.store
    }
    fn judge(&self, organization: OrganizationId) -> PgJudge {
        self.store.judge(organization)
    }
    fn machine(&self) -> &QueueMachine<PgStore> {
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
    fn credentials(&self) -> &FakeCredentials {
        &self.credentials
    }
}
