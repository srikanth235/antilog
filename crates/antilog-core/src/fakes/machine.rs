use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::json;

use super::locked;
use crate::machine::{Data, MachineError, Output, Run, SessionMachine};

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

#[derive(Default)]
struct State {
    profiles: HashMap<String, Profile>,
    broken: Vec<String>,
    /// On the holdout or not, for each run so far.
    calls: Vec<bool>,
}

const WOBBLE: [f64; 5] = [1.0, -1.0, 0.5, -0.5, 0.0];

fn wobble(i: usize, shift: usize) -> f64 {
    WOBBLE
        .get((i + shift) % WOBBLE.len())
        .copied()
        .unwrap_or(0.0)
}

/// A session machine scripted by the last commit sha of a run. The judge runs the base first
/// and the trial second, so the fake takes even runs for the base and odd runs for the trial.
/// Wobble is a fixed pattern, so runs repeat.
#[derive(Default)]
pub struct FakeMachine {
    state: Mutex<State>,
}

impl FakeMachine {
    pub fn with(self, sha: &str, profile: Profile) -> Self {
        locked(&self.state).profiles.insert(sha.to_owned(), profile);
        self
    }

    pub fn failing(self, sha: &str) -> Self {
        locked(&self.state).broken.push(sha.to_owned());
        self
    }

    /// For each command run so far: whether it read the holdout.
    pub fn calls(&self) -> Vec<bool> {
        locked(&self.state).calls.clone()
    }
}

impl SessionMachine for FakeMachine {
    async fn run(&self, run: &Run<'_>) -> Result<Output, MachineError> {
        let holdout = matches!(run.data, Data::Holdout(_));
        let mut state = locked(&self.state);
        let n = state.calls.len();
        state.calls.push(holdout);
        let sha = run.commits.last().map_or("", |c| c.sha.as_str());
        if state.broken.iter().any(|b| b == sha) {
            return Ok(Output {
                exit: 1,
                output: String::new(),
            });
        }
        let profile = state
            .profiles
            .get(sha)
            .cloned()
            .unwrap_or_else(|| Profile::new(100.0));
        let (repeat, shift) = (n / 2, n % 2);
        let value = if holdout {
            profile.holdout
        } else {
            profile.value
        };
        let guardrails: HashMap<_, _> = profile
            .guardrails
            .iter()
            .map(|(name, v)| (name.clone(), v + 0.1 * wobble(repeat, shift)))
            .collect();
        let reading = json!({
            "metric": value + profile.wobble * wobble(repeat, shift),
            "guardrails": guardrails,
        });
        Ok(Output {
            exit: 0,
            output: format!("ran {}\n{reading}\n", run.command),
        })
    }
}
