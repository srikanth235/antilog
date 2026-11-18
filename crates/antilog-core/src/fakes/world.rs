use std::collections::HashMap;
use std::sync::Mutex;

use antilog_types::judge::CommitRef;

use super::{FakeClock, FakePlatform, FakeSigner, FakeStore, FakeTimer, locked};
use crate::code_host::{CodeHost, CodeHostError, OpenedPullRequest, PullRequestSpec};
use crate::deps::Deps;
use crate::store::judge::FakeJudgeStore;

#[derive(Default)]
struct CodeHostState {
    changed: HashMap<String, Vec<(String, String)>>,
    opened: Vec<PullRequestSpec>,
    numbers: HashMap<String, u64>,
    refuse: bool,
}

/// Changed files by the sha of a commit (a sha not scripted changes nothing), and the pull
/// requests it was asked to open, numbered per repo from 1.
#[derive(Default)]
pub struct FakeCodeHost(Mutex<CodeHostState>);

impl FakeCodeHost {
    pub fn with(self, sha: &str, repo: &str, path: &str) -> Self {
        locked(&self.0)
            .changed
            .entry(sha.to_owned())
            .or_default()
            .push((repo.to_owned(), path.to_owned()));
        self
    }

    /// Every pull request it was asked to open, in order.
    pub fn opened(&self) -> Vec<PullRequestSpec> {
        locked(&self.0).opened.clone()
    }

    /// From now on it refuses to open a pull request, as a host that is down would.
    pub fn refuse_pull_requests(&self) {
        locked(&self.0).refuse = true;
    }

    /// The address the fake gives a pull request.
    pub fn url(repo: &str, number: u64) -> String {
        format!("https://code.example/{repo}/pull/{number}")
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
            .flat_map(|c| scripted.changed.get(&c.sha).cloned().unwrap_or_default())
            .collect())
    }

    async fn open_pull_request(
        &self,
        spec: &PullRequestSpec,
    ) -> Result<OpenedPullRequest, CodeHostError> {
        let mut state = locked(&self.0);
        if state.refuse {
            return Err(CodeHostError::Unavailable("the fake host refuses".into()));
        }
        let number = state.numbers.entry(spec.repo.clone()).or_insert(0);
        *number += 1;
        let number = *number;
        state.opened.push(spec.clone());
        Ok(OpenedPullRequest {
            repo: spec.repo.clone(),
            number,
            url: Self::url(&spec.repo, number),
        })
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
        let store = FakeStore::default();
        Self {
            judge: store.judge_store(),
            store,
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
    fn judge(&self, _: antilog_types::OrganizationId) -> FakeJudgeStore {
        self.judge.clone()
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
