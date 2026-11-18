# A write

Every write goes through one action layer, and nowhere else. A write that skips a step is a bug. A new write is an entry in the action layer, not a handler with its own checks. Built for the organization floor: the functions of `crates/antilog-core/src/organization.rs`, `approval.rs` and `budget.rs`; step 3 (events) is target.

## The three steps, in order (target)

1. **Check.** The caller's role is checked first, in one place: their role in the project the write touches, which is the organization role unless the project set another. A denied call changes nothing and records the denial.
2. **Write and audit.** The change and its audit entry happen in one transaction of the store (Postgres), or neither does. Every row carries its organization.
3. **Publish.** After the commit, the layer publishes one live event naming what changed (see [live-event.md](live-event.md)).

An action that needs work on an agent platform queues a job in the same transaction. The server takes it (see [platform-job.md](platform-job.md)). The action layer never calls a platform or a connector directly.

## Adding a write (target)

1. Name it, verb-object, such as `session.stop`, and register it with [organization/add-audited-action.md](../organization/add-audited-action.md). That page owns the matrix, the audit fields and the denial rules.
2. Add the types and handler by [api-endpoint.md](api-endpoint.md). The handler calls the layer.
3. Add the CLI command, and the Antilog tool if an agent calls it (see [tool.md](tool.md)). A command that spends or changes things needs `--yes`. A tool call that matters is an approval card by policy (see [organization/approvals.md](../organization/approvals.md)).
4. Tests: allowed role writes and audits once; denied role writes nothing and audits the denial; the event is published after the commit and not on a denial; no secret in the entry or the event; the audit row and the change roll back together.

## Refusals

- A refused write is audited, with the actor, the action and the reason.
- A refused read is not audited. It is logged with the actor for operators, and repeated refusals can raise an alert.

