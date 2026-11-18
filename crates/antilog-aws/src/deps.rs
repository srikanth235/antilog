//! What the Lambda server runs on: Postgres, AgentCore, the Scheduler timer and the
//! receipt key from the environment. The code host is still the fake: no real one is built.

use antilog_agentcore::{AgentCore, Config as AgentCoreConfig, SdkRuntime};
use antilog_core::clock::SystemClock;
use antilog_core::deps::Deps;
use antilog_core::fakes::FakeCodeHost;
use antilog_core::judge::ed25519::Ed25519Signer;
use antilog_core::secrets::EnvSecrets;
use antilog_server::pg::{PgJudge, PgStore};
use antilog_types::OrganizationId;
use anyhow::Context;

use crate::scheduler::{SchedulerConfig, SchedulerTimer, SdkScheduling};

pub struct AwsDeps {
    pub store: PgStore,
    platform: AgentCore<SdkRuntime>,
    signer: Ed25519Signer,
    timer: SchedulerTimer<SdkScheduling>,
    code: FakeCodeHost,
}

impl AwsDeps {
    /// Everything is read from the environment the deploy set; nothing connects yet except
    /// the lookup of the runtimes' names.
    pub async fn from_env(store: PgStore) -> anyhow::Result<Self> {
        let agentcore = AgentCoreConfig::from_env().context("AgentCore runtimes")?;
        let platform = AgentCore::connect(&agentcore)
            .await
            .context("connecting to AgentCore")?;
        let scheduler = SchedulerConfig::from_env().map_err(anyhow::Error::msg)?;
        let secret = scheduler.wake_secret.clone();
        let timer = SchedulerTimer::new(SdkScheduling::connect(scheduler).await, secret);
        let signer = Ed25519Signer::from_secrets(&EnvSecrets::from_env(), "receipt-key")
            .await
            .context("the receipt key")?;
        Ok(Self {
            store,
            platform,
            signer,
            timer,
            code: FakeCodeHost::default(),
        })
    }
}

impl Deps for AwsDeps {
    type Store = PgStore;
    type Judge = PgJudge;
    type Platform = AgentCore<SdkRuntime>;
    type Signer = Ed25519Signer;
    type Clock = SystemClock;
    type Timer = SchedulerTimer<SdkScheduling>;
    type Code = FakeCodeHost;

    fn store(&self) -> &PgStore {
        &self.store
    }
    fn judge(&self, organization: OrganizationId) -> PgJudge {
        self.store.judge(organization)
    }
    fn platform(&self) -> &Self::Platform {
        &self.platform
    }
    fn signer(&self) -> &Ed25519Signer {
        &self.signer
    }
    fn clock(&self) -> &SystemClock {
        &SystemClock
    }
    fn timer(&self) -> &Self::Timer {
        &self.timer
    }
    fn code(&self) -> &FakeCodeHost {
        &self.code
    }
}
