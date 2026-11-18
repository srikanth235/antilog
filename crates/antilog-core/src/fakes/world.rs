use std::collections::HashMap;
use std::sync::Mutex;

use antilog_types::judge::CommitRef;

use super::{FakeClock, FakePlatform, FakeSigner, FakeStore, FakeTimer, locked};
use crate::code_host::{CodeHost, CodeHostError};
use crate::deps::Deps;
use crate::store::judge::FakeJudgeStore;

/// Changed files by the sha of a commit; a sha not scripted changes nothing.
#[derive(Default)]
pub struct FakeCodeHost(Mutex<HashMap<String, Vec<(String, String)>>>);

impl FakeCodeHost {
    pub fn with(self, sha: &str, repo: &str, path: &str) -> Self {
        locked(&self.0)
            .entry(sha.to_owned())
            .or_default()
            .push((repo.to_owned(), path.to_owned()));
        self
    }
}

impl CodeHost for FakeCodeHost {
    async fn changed_files(
        &self,
        commits: &[CommitRef],
    ) -> Result<Vec<(String, String)>, CodeHostError> {
        let scripted = locked(&self.0);
        Ok(commits
            .iter()
            .flat_map(|c| scripted.get(&c.sha).cloned().unwrap_or_default())
            .collect())
    }
}

pub struct FakeDeps {
    pub store: FakeStore,
    pub judge: FakeJudgeStore,
    pub platform: FakePlatform,
    pub signer: FakeSigner,
    pub clock: FakeClock,
    pub timer: FakeTimer,
    pub code: FakeCodeHost,
}

impl FakeDeps {
    pub fn new(platform: FakePlatform, code: FakeCodeHost) -> Self {
        Self {
            store: FakeStore::default(),
            judge: FakeJudgeStore::default(),
            platform,
            signer: FakeSigner::new("fake"),
            clock: FakeClock::new("2026-11-18T09:00:00Z".parse().unwrap_or_default()),
            timer: FakeTimer::default(),
            code,
        }
    }
}

impl Deps for FakeDeps {
    type Store = FakeStore;
    type Judge = FakeJudgeStore;
    type Platform = FakePlatform;
    type Signer = FakeSigner;
    type Clock = FakeClock;
    type Timer = FakeTimer;
    type Code = FakeCodeHost;

    fn store(&self) -> &FakeStore {
        &self.store
    }
    fn judge(&self) -> &FakeJudgeStore {
        &self.judge
    }
    fn platform(&self) -> &FakePlatform {
        &self.platform
    }
    fn signer(&self) -> &FakeSigner {
        &self.signer
    }
    fn clock(&self) -> &FakeClock {
        &self.clock
    }
    fn timer(&self) -> &FakeTimer {
        &self.timer
    }
    fn code(&self) -> &FakeCodeHost {
        &self.code
    }
}
