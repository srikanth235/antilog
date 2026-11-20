use std::future::Future;

use antilog_types::{
    Command, Envelope, Machine, MachineState, OrganizationId, RunResult, Seed, SessionId,
};
use uuid::Uuid;

use super::StoreError;

/// A runner's organization lock lapses when the runner has not polled for this long (ten
/// minutes), so a crashed runner does not hold its organization for ever.
pub const RUNNER_LOCK_LAPSE_MS: i64 = 10 * 60 * 1000;

/// The queue of sessions and what passes between the server and a runner. A runner serves every
/// organization, so these calls name a session rather than an organization.
pub trait RunnerStore {
    /// A new session waits in `Queued`.
    fn enqueue_machine(
        &self,
        organization: OrganizationId,
        session: SessionId,
        seed: &Seed,
        at: i64,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn machine(
        &self,
        session: SessionId,
    ) -> impl Future<Output = Result<Option<Machine>, StoreError>> + Send;

    /// Puts every session whose lease lapsed before `now` back in `Queued`, then claims up to
    /// `limit` queued sessions for `holder` with a lease until `until` (milliseconds). A
    /// session whose organization budget or state forbids running is the caller's concern.
    ///
    /// A runner locks to the organization of the first session it claims and claims only that
    /// organization's sessions afterwards, until `unlock_runner`, so customers never share a
    /// host. A call with no room (`limit` 0) claims nothing and locks nothing. Every call is the
    /// runner's poll: a lock whose runner has not polled for `RUNNER_LOCK_LAPSE_MS` lapses, and
    /// any runner of any organization may then take its place. An organization is never handed
    /// a claim beyond its `max_running_sessions` machines (claimed, running and idle); its
    /// queued sessions wait.
    fn claim(
        &self,
        holder: &str,
        limit: u32,
        now: i64,
        until: i64,
    ) -> impl Future<Output = Result<Vec<Machine>, StoreError>> + Send;

    /// The runner is leaving, so its organization lock goes, unless it still holds a session
    /// (then the lock stays: its host still holds that organization's work).
    fn unlock_runner(&self, holder: &str) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Flags every session `holder` holds whose claim began before `claimed_before`
    /// (milliseconds) and was not flagged yet, and returns them: their machine time is up.
    fn mark_overdue(
        &self,
        holder: &str,
        claimed_before: i64,
    ) -> impl Future<Output = Result<Vec<SessionId>, StoreError>> + Send;

    /// Extends the lease of every session `holder` still holds, and returns them.
    fn renew(
        &self,
        holder: &str,
        now: i64,
        until: i64,
    ) -> impl Future<Output = Result<Vec<SessionId>, StoreError>> + Send;

    /// Moves a session the holder holds to `state`; false when it holds no such session.
    /// `Released`, `Done` and `Failed` end the lease. `Done` and `Failed` are final.
    fn set_machine_state(
        &self,
        session: SessionId,
        holder: &str,
        state: MachineState,
        now: i64,
    ) -> impl Future<Output = Result<bool, StoreError>> + Send;

    /// A person wrote to a session whose machine was released: it waits for a runner again.
    /// True when it was requeued.
    fn wake_machine(
        &self,
        session: SessionId,
        now: i64,
    ) -> impl Future<Output = Result<bool, StoreError>> + Send;

    /// Adds a repo to the session's seed, once, so a later claim or resume clones it. False when
    /// the seed had it already.
    fn add_seed_repo(
        &self,
        session: SessionId,
        repo: &str,
    ) -> impl Future<Output = Result<bool, StoreError>> + Send;

    /// A queue entry for the session's holder. Commands are delivered once, oldest first.
    fn push_command(
        &self,
        organization: OrganizationId,
        session: SessionId,
        command: &Command,
        at: i64,
    ) -> impl Future<Output = Result<Uuid, StoreError>> + Send;

    /// Takes the undelivered commands of the sessions `holder` holds and marks them delivered.
    fn take_commands(
        &self,
        holder: &str,
    ) -> impl Future<Output = Result<Vec<Envelope>, StoreError>> + Send;

    /// A runner reports these commands as handled; one not reported is sent again after a
    /// requeue.
    fn finish_commands(
        &self,
        holder: &str,
        ids: &[Uuid],
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Forgets a command nobody waits for any more.
    fn drop_command(&self, id: Uuid) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Keeps the outcome of a `Run`. False when the session has no such command.
    fn put_result(
        &self,
        session: SessionId,
        result: &RunResult,
    ) -> impl Future<Output = Result<bool, StoreError>> + Send;

    fn result(
        &self,
        command: Uuid,
    ) -> impl Future<Output = Result<Option<RunResult>, StoreError>> + Send;

    /// The session's `opencode export`, replaced on each release.
    fn put_export(
        &self,
        organization: OrganizationId,
        session: SessionId,
        body: &str,
        at: i64,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn export(
        &self,
        session: SessionId,
    ) -> impl Future<Output = Result<Option<String>, StoreError>> + Send;
}
