//! The server's `SessionMachine`: the judge's command goes to the runner that holds the session
//! and its outcome comes back through `/runner/sessions/{id}/results`.

use std::time::Duration;

use antilog_core::clock::{Clock, SystemClock};
use antilog_core::machine::{Data, MachineError, Output, Run, SessionMachine};
use antilog_core::store::{RunnerStore, StoreError};
use antilog_types::{Command, DataSpec, MachineState, RunSpec};

pub struct QueueMachine<S> {
    store: S,
    /// How long a command may wait for its outcome: a claim, the checkout and the run.
    timeout: Duration,
    /// How often the outcome is looked for.
    tick: Duration,
}

impl<S> QueueMachine<S> {
    pub fn new(store: S, timeout: Duration) -> Self {
        Self {
            store,
            timeout,
            tick: Duration::from_millis(200),
        }
    }

    pub fn polling_every(self, tick: Duration) -> Self {
        Self { tick, ..self }
    }
}

fn unavailable(error: StoreError) -> MachineError {
    MachineError::Unavailable(Box::new(error))
}

impl<S: RunnerStore + Send + Sync> SessionMachine for QueueMachine<S> {
    async fn run(&self, run: &Run<'_>) -> Result<Output, MachineError> {
        let machine = self
            .store
            .machine(run.session)
            .await
            .map_err(unavailable)?
            .ok_or(MachineError::NoMachine)?;
        if matches!(machine.state, MachineState::Done | MachineState::Failed) {
            return Err(MachineError::NoMachine);
        }
        let now = SystemClock.now().as_millisecond();
        // A machine that was let go is queued again to run the command.
        self.store
            .wake_machine(run.session, now)
            .await
            .map_err(unavailable)?;
        let (holdout, dataset) = match run.data {
            Data::Evaluation(dataset) => (false, dataset.clone()),
            Data::Holdout(dataset) => (true, dataset.clone()),
        };
        let command = Command::Run {
            run: RunSpec {
                command: run.command.to_owned(),
                commits: run.commits.to_vec(),
                files: run.files.to_vec(),
                data: DataSpec { holdout, dataset },
            },
        };
        let id = self
            .store
            .push_command(machine.organization, run.session, &command, now)
            .await
            .map_err(unavailable)?;
        let deadline = tokio::time::Instant::now() + self.timeout;
        loop {
            if let Some(result) = self.store.result(id).await.map_err(unavailable)? {
                return Ok(Output {
                    exit: result.exit,
                    output: result.output,
                });
            }
            if tokio::time::Instant::now() >= deadline {
                // Nobody waits for it any more, so a runner that comes later does not run it.
                self.store.drop_command(id).await.map_err(unavailable)?;
                return Err(MachineError::Unavailable(
                    "the machine did not answer in time".into(),
                ));
            }
            tokio::time::sleep(self.tick).await;
        }
    }
}
