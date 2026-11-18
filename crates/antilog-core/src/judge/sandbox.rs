use std::collections::HashMap;
use std::future::Future;
use std::sync::{Mutex, PoisonError};

use antilog_types::judge::{CommitRef, DatasetRef, Guardrail, ProtectedFile, SealHash};

#[derive(Debug, Clone, Copy)]
pub enum Data<'a> {
    Evaluation(&'a DatasetRef),
    Holdout(&'a DatasetRef),
}

/// One fresh judge sandbox: the base and the trial run interleaved `repeats` times.
/// Protected files come from the seal, never from the trial. `seal` is `None` only
/// for the seal checks, before there is a seal.
#[derive(Debug, Clone, Copy)]
pub struct PairedRun<'a> {
    pub seal: Option<&'a SealHash>,
    pub command: &'a str,
    pub files: &'a [ProtectedFile],
    pub data: Data<'a>,
    pub base: &'a [CommitRef],
    pub trial: &'a [CommitRef],
    pub repeats: u32,
    pub guardrails: &'a [Guardrail],
}

/// `(base, trial)` per repeat.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pairs {
    pub metric: Vec<(f64, f64)>,
    pub guardrails: Vec<(String, Vec<(f64, f64)>)>,
}

#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("the run failed: {0}")]
    Failed(String),
    #[error("sandbox unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// The judge sandbox on a platform. It returns only the numbers the experiment names.
pub trait Sandbox: Send + Sync {
    fn run(&self, run: &PairedRun<'_>) -> impl Future<Output = Result<Pairs, SandboxError>> + Send;
}

#[derive(Debug, Clone, Default)]
pub struct Profile {
    pub value: f64,
    pub holdout: f64,
    pub guardrails: Vec<(String, f64)>,
    pub wobble: f64,
}

impl Profile {
    pub fn new(value: f64) -> Self {
        Self {
            value,
            holdout: value,
            guardrails: vec![],
            wobble: 0.5,
        }
    }

    pub fn holdout(self, holdout: f64) -> Self {
        Self { holdout, ..self }
    }

    pub fn guardrail(mut self, name: &str, value: f64) -> Self {
        self.guardrails.push((name.to_owned(), value));
        self
    }
}

/// Scripted by the last commit sha of a side. Wobble is a fixed pattern, so runs repeat.
#[derive(Default)]
pub struct FakeSandbox {
    profiles: Mutex<HashMap<String, Profile>>,
    broken: Mutex<Vec<String>>,
    calls: Mutex<Vec<(bool, u32)>>,
}

impl FakeSandbox {
    pub fn with(self, sha: &str, profile: Profile) -> Self {
        self.profiles
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(sha.to_owned(), profile);
        self
    }

    pub fn failing(self, sha: &str) -> Self {
        self.broken
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(sha.to_owned());
        self
    }

    /// `(on the holdout, repeats)` for each run so far.
    pub fn calls(&self) -> Vec<(bool, u32)> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

const WOBBLE: [f64; 5] = [1.0, -1.0, 0.5, -0.5, 0.0];

fn wobble(i: usize, shift: usize) -> f64 {
    WOBBLE
        .get((i + shift) % WOBBLE.len())
        .copied()
        .unwrap_or(0.0)
}

impl Sandbox for FakeSandbox {
    async fn run(&self, run: &PairedRun<'_>) -> Result<Pairs, SandboxError> {
        let holdout = matches!(run.data, Data::Holdout(_));
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((holdout, run.repeats));
        let side = |commits: &[CommitRef]| {
            let sha = commits.last().map_or("", |c| c.sha.as_str()).to_owned();
            let profile = self
                .profiles
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get(&sha)
                .cloned()
                .unwrap_or_else(|| Profile::new(100.0));
            (sha, profile)
        };
        let ((base_sha, base), (trial_sha, trial)) = (side(run.base), side(run.trial));
        let broken = self.broken.lock().unwrap_or_else(PoisonError::into_inner);
        if broken.contains(&base_sha) || broken.contains(&trial_sha) {
            return Err(SandboxError::Failed("the command exited with 1".into()));
        }
        let n = usize::try_from(run.repeats).unwrap_or(0);
        let pick = |p: &Profile| if holdout { p.holdout } else { p.value };
        let metric = (0..n)
            .map(|i| {
                (
                    pick(&base) + base.wobble * wobble(i, 0),
                    pick(&trial) + trial.wobble * wobble(i, 1),
                )
            })
            .collect();
        let guardrails = run
            .guardrails
            .iter()
            .map(|g| {
                let of = |p: &Profile| {
                    p.guardrails
                        .iter()
                        .find(|(name, _)| *name == g.name)
                        .map_or(0.0, |(_, v)| *v)
                };
                let pairs = (0..n)
                    .map(|i| {
                        (
                            of(&base) + 0.1 * wobble(i, 0),
                            of(&trial) + 0.1 * wobble(i, 1),
                        )
                    })
                    .collect();
                (g.name.clone(), pairs)
            })
            .collect();
        Ok(Pairs { metric, guardrails })
    }
}
