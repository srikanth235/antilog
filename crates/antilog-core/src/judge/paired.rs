//! The base and the trial measured side by side: each repeat runs the sealed command on the
//! base, then on the trial, in the session's machine. The command ends its output with one line
//! of JSON: `{"metric": 12.5, "guardrails": {"name": 3.0}}`.

use antilog_types::SessionId;
use antilog_types::judge::{CommitRef, Guardrail, ProtectedFile};
use serde::Deserialize;

pub use crate::machine::Data;
use crate::machine::{MachineError, Run, SessionMachine};

/// One paired measurement: the base and the trial interleaved `repeats` times.
/// Protected files come from the seal, never from the trial.
#[derive(Debug, Clone, Copy)]
pub struct PairedRun<'a> {
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
pub enum RunError {
    #[error("the run failed: {0}")]
    Failed(String),
    #[error(transparent)]
    Machine(#[from] MachineError),
}

#[derive(Deserialize)]
struct Reading {
    metric: f64,
    #[serde(default)]
    guardrails: std::collections::HashMap<String, f64>,
}

async fn read<M: SessionMachine>(
    machine: &M,
    session: SessionId,
    run: &PairedRun<'_>,
    commits: &[CommitRef],
) -> Result<Reading, RunError> {
    let output = machine
        .run(&Run {
            session,
            command: run.command,
            commits,
            files: run.files,
            data: run.data,
        })
        .await?;
    if output.exit != 0 {
        return Err(RunError::Failed(format!(
            "the command exited with {}",
            output.exit
        )));
    }
    output
        .output
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .and_then(|line| serde_json::from_str(line).ok())
        .ok_or_else(|| RunError::Failed("the command printed no result line".into()))
}

/// Runs the command `repeats` times on each side, base first, one after the other.
pub async fn run_paired<M: SessionMachine>(
    machine: &M,
    session: SessionId,
    run: &PairedRun<'_>,
) -> Result<Pairs, RunError> {
    let mut pairs = Pairs {
        metric: Vec::new(),
        guardrails: run
            .guardrails
            .iter()
            .map(|g| (g.name.clone(), Vec::new()))
            .collect(),
    };
    for _ in 0..run.repeats {
        let base = read(machine, session, run, run.base).await?;
        let trial = read(machine, session, run, run.trial).await?;
        pairs.metric.push((base.metric, trial.metric));
        for (name, series) in &mut pairs.guardrails {
            let both = base.guardrails.get(name).zip(trial.guardrails.get(name));
            let (b, t) = both.ok_or_else(|| {
                RunError::Failed(format!("the command printed no value for guardrail {name}"))
            })?;
            series.push((*b, *t));
        }
    }
    Ok(pairs)
}

#[cfg(test)]
mod tests {
    use antilog_types::judge::{DatasetRef, Direction};
    use std::sync::Mutex;

    use super::*;
    use crate::machine::Output;

    /// Answers each run with the next scripted output.
    struct Script(Mutex<Vec<Output>>);

    impl SessionMachine for Script {
        async fn run(&self, _: &Run<'_>) -> Result<Output, MachineError> {
            self.0
                .lock()
                .map_err(|_| MachineError::NoMachine)?
                .pop()
                .ok_or(MachineError::NoMachine)
        }
    }

    fn said(exit: i32, output: &str) -> Output {
        Output {
            exit,
            output: output.to_owned(),
        }
    }

    async fn run_with(outputs: Vec<Output>, guardrails: &[Guardrail]) -> Result<Pairs, RunError> {
        let dataset = DatasetRef {
            id: "d".into(),
            version: 1,
        };
        let machine = Script(Mutex::new(outputs.into_iter().rev().collect()));
        run_paired(
            &machine,
            SessionId::new(),
            &PairedRun {
                command: "make bench",
                files: &[],
                data: Data::Evaluation(&dataset),
                base: &[],
                trial: &[],
                repeats: 2,
                guardrails,
            },
        )
        .await
    }

    #[tokio::test]
    async fn the_last_line_of_output_is_the_reading_and_runs_alternate_base_then_trial() {
        let pairs = run_with(
            vec![
                said(0, "noise\n{\"metric\": 10}\n"),
                said(0, "{\"metric\": 8}"),
                said(0, "{\"metric\": 11}\n\n"),
                said(0, "{\"metric\": 9}"),
            ],
            &[],
        )
        .await
        .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(pairs.metric, vec![(10.0, 8.0), (11.0, 9.0)]);
    }

    #[tokio::test]
    async fn guardrail_readings_pair_up_and_a_missing_one_fails_the_run() {
        let guardrail = Guardrail {
            name: "errors".into(),
            direction: Direction::Minimize,
            tolerance: 0.0,
        };
        let line = |m: f64, g: f64| {
            said(
                0,
                &format!("{{\"metric\": {m}, \"guardrails\": {{\"errors\": {g}}}}}"),
            )
        };
        let pairs = run_with(
            vec![
                line(1.0, 2.0),
                line(1.0, 3.0),
                line(1.0, 4.0),
                line(1.0, 5.0),
            ],
            std::slice::from_ref(&guardrail),
        )
        .await
        .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            pairs.guardrails,
            vec![("errors".to_owned(), vec![(2.0, 3.0), (4.0, 5.0)])]
        );
        let missing = run_with(
            vec![said(0, "{\"metric\": 1}"), said(0, "{\"metric\": 1}")],
            std::slice::from_ref(&guardrail),
        )
        .await;
        assert!(matches!(missing, Err(RunError::Failed(_))));
    }

    #[tokio::test]
    async fn a_failed_command_or_a_missing_result_line_is_a_failed_run() {
        for output in [said(1, "{\"metric\": 1}"), said(0, "built it"), said(0, "")] {
            let ran = run_with(vec![output], &[]).await;
            assert!(matches!(ran, Err(RunError::Failed(_))), "{ran:?}");
        }
    }
}
