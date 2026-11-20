use std::collections::HashMap;

use antilog_types::{
    Command, Envelope, Machine, MachineState, OrganizationId, RunResult, Seed, SessionId,
};
use uuid::Uuid;

use super::{FakeStore, locked};
use crate::store::{RunnerStore, StoreError};

struct Row {
    machine: Machine,
    lease_until: i64,
}

struct Pending {
    id: Uuid,
    session: SessionId,
    command: Command,
    delivered: bool,
    finished: bool,
    result: Option<RunResult>,
}

#[derive(Default)]
pub(super) struct State {
    machines: Vec<Row>,
    commands: Vec<Pending>,
    exports: HashMap<SessionId, String>,
    /// The organization each runner is locked to.
    locks: HashMap<String, OrganizationId>,
}

impl State {
    fn requeue_lapsed(&mut self, now: i64) {
        for row in &mut self.machines {
            if row.machine.state.is_held() && row.lease_until < now {
                row.machine.state = MachineState::Queued;
                row.machine.holder = None;
                for pending in &mut self.commands {
                    if pending.session == row.machine.session && !pending.finished {
                        pending.delivered = false;
                    }
                }
            }
        }
    }

    fn row(&mut self, session: SessionId) -> Option<&mut Row> {
        self.machines
            .iter_mut()
            .find(|row| row.machine.session == session)
    }
}

impl RunnerStore for FakeStore {
    async fn enqueue_machine(
        &self,
        organization: OrganizationId,
        session: SessionId,
        seed: &Seed,
        _at: i64,
    ) -> Result<(), StoreError> {
        let row = locked(&self.ledger)
            .session(organization, session)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let mut state = locked(&self.runner);
        if state.row(session).is_some() {
            return Err(StoreError::Conflict);
        }
        state.machines.push(Row {
            machine: Machine {
                session,
                organization,
                experiment: row.experiment_id,
                agent: row.agent,
                state: MachineState::Queued,
                holder: None,
                seed: seed.clone(),
                resume: false,
            },
            lease_until: 0,
        });
        Ok(())
    }

    async fn machine(&self, session: SessionId) -> Result<Option<Machine>, StoreError> {
        Ok(locked(&self.runner)
            .row(session)
            .map(|row| row.machine.clone()))
    }

    async fn claim(
        &self,
        holder: &str,
        limit: u32,
        now: i64,
        until: i64,
    ) -> Result<Vec<Machine>, StoreError> {
        let active: Vec<SessionId> = locked(&self.ledger)
            .sessions
            .iter()
            .filter(|row| row.status.is_active())
            .map(|row| row.id)
            .collect();
        let mut state = locked(&self.runner);
        state.requeue_lapsed(now);
        if limit == 0 {
            return Ok(Vec::new());
        }
        let eligible = |row: &Row| {
            row.machine.state == MachineState::Queued && active.contains(&row.machine.session)
        };
        let lock = match state.locks.get(holder) {
            Some(lock) => *lock,
            None => {
                let Some(first) = state.machines.iter().find(|row| eligible(row)) else {
                    return Ok(Vec::new());
                };
                let organization = first.machine.organization;
                state.locks.insert(holder.to_owned(), organization);
                organization
            }
        };
        let mut claimed = Vec::new();
        for row in &mut state.machines {
            if claimed.len() >= limit as usize {
                break;
            }
            if eligible(row) && row.machine.organization == lock {
                row.machine.state = MachineState::Claimed;
                row.machine.holder = Some(holder.to_owned());
                row.lease_until = until;
                claimed.push(row.machine.clone());
            }
        }
        Ok(claimed)
    }

    async fn unlock_runner(&self, holder: &str) -> Result<(), StoreError> {
        let mut state = locked(&self.runner);
        let holds = state.machines.iter().any(|row| {
            row.machine.state.is_held() && row.machine.holder.as_deref() == Some(holder)
        });
        if !holds {
            state.locks.remove(holder);
        }
        Ok(())
    }

    async fn renew(
        &self,
        holder: &str,
        _now: i64,
        until: i64,
    ) -> Result<Vec<SessionId>, StoreError> {
        let mut state = locked(&self.runner);
        let mut held = Vec::new();
        for row in &mut state.machines {
            if row.machine.state.is_held() && row.machine.holder.as_deref() == Some(holder) {
                row.lease_until = until;
                held.push(row.machine.session);
            }
        }
        Ok(held)
    }

    async fn set_machine_state(
        &self,
        session: SessionId,
        holder: &str,
        next: MachineState,
        _now: i64,
    ) -> Result<bool, StoreError> {
        let mut state = locked(&self.runner);
        let Some(row) = state.row(session) else {
            return Ok(false);
        };
        if !row.machine.state.is_held() || row.machine.holder.as_deref() != Some(holder) {
            return Ok(false);
        }
        row.machine.state = next;
        if !next.is_held() {
            row.machine.holder = None;
        }
        Ok(true)
    }

    async fn wake_machine(&self, session: SessionId, _now: i64) -> Result<bool, StoreError> {
        let mut state = locked(&self.runner);
        let resume = state.exports.contains_key(&session);
        let Some(row) = state.row(session) else {
            return Ok(false);
        };
        if row.machine.state != MachineState::Released {
            return Ok(false);
        }
        row.machine.state = MachineState::Queued;
        row.machine.resume = resume;
        Ok(true)
    }

    async fn push_command(
        &self,
        _organization: OrganizationId,
        session: SessionId,
        command: &Command,
        _at: i64,
    ) -> Result<Uuid, StoreError> {
        let mut state = locked(&self.runner);
        if state.row(session).is_none() {
            return Err(StoreError::NotFound);
        }
        let id = Uuid::now_v7();
        state.commands.push(Pending {
            id,
            session,
            command: command.clone(),
            delivered: false,
            finished: false,
            result: None,
        });
        Ok(id)
    }

    async fn take_commands(&self, holder: &str) -> Result<Vec<Envelope>, StoreError> {
        let mut state = locked(&self.runner);
        let held: Vec<SessionId> = state
            .machines
            .iter()
            .filter(|row| {
                row.machine.state.is_held() && row.machine.holder.as_deref() == Some(holder)
            })
            .map(|row| row.machine.session)
            .collect();
        let mut taken = Vec::new();
        for pending in &mut state.commands {
            if !pending.delivered && held.contains(&pending.session) {
                pending.delivered = true;
                taken.push(Envelope {
                    id: pending.id,
                    session: pending.session,
                    command: pending.command.clone(),
                });
            }
        }
        Ok(taken)
    }

    async fn finish_commands(&self, holder: &str, ids: &[Uuid]) -> Result<(), StoreError> {
        let mut state = locked(&self.runner);
        let mine: Vec<SessionId> = state
            .machines
            .iter()
            .filter(|row| row.machine.holder.as_deref() == Some(holder))
            .map(|row| row.machine.session)
            .collect();
        for pending in &mut state.commands {
            if ids.contains(&pending.id) && mine.contains(&pending.session) {
                pending.finished = true;
            }
        }
        Ok(())
    }

    async fn drop_command(&self, id: Uuid) -> Result<(), StoreError> {
        locked(&self.runner)
            .commands
            .retain(|pending| pending.id != id);
        Ok(())
    }

    async fn put_result(&self, session: SessionId, result: &RunResult) -> Result<bool, StoreError> {
        let mut state = locked(&self.runner);
        match state
            .commands
            .iter_mut()
            .find(|pending| pending.id == result.command && pending.session == session)
        {
            Some(pending) => {
                pending.result = Some(result.clone());
                pending.finished = true;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    async fn result(&self, command: Uuid) -> Result<Option<RunResult>, StoreError> {
        Ok(locked(&self.runner)
            .commands
            .iter()
            .find(|pending| pending.id == command)
            .and_then(|pending| pending.result.clone()))
    }

    async fn put_export(
        &self,
        _organization: OrganizationId,
        session: SessionId,
        body: &str,
        _at: i64,
    ) -> Result<(), StoreError> {
        locked(&self.runner)
            .exports
            .insert(session, body.to_owned());
        Ok(())
    }

    async fn export(&self, session: SessionId) -> Result<Option<String>, StoreError> {
        Ok(locked(&self.runner).exports.get(&session).cloned())
    }
}
