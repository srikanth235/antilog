use std::collections::HashMap;

use antilog_types::{Command, Envelope, Machine, MachineState, OrganizationId, Seed, SessionId};
use uuid::Uuid;

use super::{FakeStore, locked};
use crate::store::{RUNNER_LOCK_LAPSE_MS, RunnerStore, StoreError};

struct Row {
    machine: Machine,
    lease_until: i64,
    /// When the current claim began.
    claimed_at: i64,
    /// The release for an over-long claim was sent.
    expiring: bool,
}

struct Pending {
    id: Uuid,
    session: SessionId,
    command: Command,
    delivered: bool,
    finished: bool,
}

#[derive(Default)]
pub(super) struct State {
    machines: Vec<Row>,
    commands: Vec<Pending>,
    exports: HashMap<SessionId, String>,
    /// The organization each runner is locked to, and when it last polled.
    locks: HashMap<String, (OrganizationId, i64)>,
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

impl FakeStore {
    /// The machine's seed follows the session's environment, as the real store does it in one
    /// transaction.
    pub(super) fn move_seed(
        &self,
        session: SessionId,
        environment: Option<&str>,
        version: Option<u32>,
    ) {
        if let Some(row) = locked(&self.runner).row(session) {
            row.machine.seed.environment = environment.map(str::to_owned);
            row.machine.seed.environment_version = version;
        }
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
                agent: row.agent,
                state: MachineState::Queued,
                holder: None,
                seed: seed.clone(),
                resume: false,
            },
            lease_until: 0,
            claimed_at: 0,
            expiring: false,
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
        let caps: HashMap<OrganizationId, usize> = locked(&self.ledger)
            .organizations
            .iter()
            .map(|row| (row.id, row.max_running_sessions as usize))
            .collect();
        let mut state = locked(&self.runner);
        state.requeue_lapsed(now);
        state
            .locks
            .retain(|_, (_, polled)| *polled >= now - RUNNER_LOCK_LAPSE_MS);
        if let Some((_, polled)) = state.locks.get_mut(holder) {
            *polled = now;
        }
        if limit == 0 {
            return Ok(Vec::new());
        }
        let running = |state: &State, organization: OrganizationId| {
            state
                .machines
                .iter()
                .filter(|row| {
                    row.machine.state.is_held() && row.machine.organization == organization
                })
                .count()
        };
        let room = |state: &State, organization: OrganizationId| {
            caps.get(&organization)
                .copied()
                .unwrap_or(0)
                .saturating_sub(running(state, organization))
        };
        let eligible = |row: &Row| {
            row.machine.state == MachineState::Queued && active.contains(&row.machine.session)
        };
        let lock = match state.locks.get(holder) {
            Some((lock, _)) => *lock,
            None => {
                let Some(first) = state
                    .machines
                    .iter()
                    .find(|row| eligible(row) && room(&state, row.machine.organization) > 0)
                else {
                    return Ok(Vec::new());
                };
                let organization = first.machine.organization;
                state.locks.insert(holder.to_owned(), (organization, now));
                organization
            }
        };
        let mut left = room(&state, lock).min(limit as usize);
        let mut claimed = Vec::new();
        for row in &mut state.machines {
            if left == 0 {
                break;
            }
            if eligible(row) && row.machine.organization == lock {
                row.machine.state = MachineState::Claimed;
                row.machine.holder = Some(holder.to_owned());
                row.lease_until = until;
                row.claimed_at = now;
                row.expiring = false;
                claimed.push(row.machine.clone());
                left -= 1;
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

    async fn mark_overdue(
        &self,
        holder: &str,
        claimed_before: i64,
    ) -> Result<Vec<SessionId>, StoreError> {
        let mut state = locked(&self.runner);
        let mut overdue = Vec::new();
        for row in &mut state.machines {
            if row.machine.state.is_held()
                && row.machine.holder.as_deref() == Some(holder)
                && !row.expiring
                && row.claimed_at < claimed_before
            {
                row.expiring = true;
                overdue.push(row.machine.session);
            }
        }
        Ok(overdue)
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

    async fn add_seed_repo(&self, session: SessionId, repo: &str) -> Result<bool, StoreError> {
        let mut state = locked(&self.runner);
        let row = state.row(session).ok_or(StoreError::NotFound)?;
        if row.machine.seed.repos.iter().any(|name| name == repo) {
            return Ok(false);
        }
        row.machine.seed.repos.push(repo.to_owned());
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
