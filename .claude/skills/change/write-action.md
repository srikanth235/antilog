# A write

Every write goes through one action layer, and nowhere else. A write that skips a step is a bug. A new write is an entry in the action layer, not a handler with its own checks. (target: the layer is not built yet.)

## The three steps, in order (target)

1. **Check.** The caller's role on the project is checked first, in one place. A denied call changes nothing and records the denial.
2. **Write and audit.** The change and its audit entry happen in one Postgres transaction, or neither does. Every row carries its organization.
3. **Publish.** After the commit, the layer publishes one live event naming what changed (see [live-event.md](live-event.md)).

An action that needs work in the customer's infrastructure queues a job in the same transaction. The runner executes it (see [runner-job.md](runner-job.md)). The action layer never calls a runner or a connector directly.

## Adding a write (target)

1. Name it, verb-object, such as `experiment.stop`, and register it with [organization/add-audited-action.md](../organization/add-audited-action.md). That page owns the matrix, the audit fields and the denial rules.
2. Add the types and handler by [api-endpoint.md](api-endpoint.md). The handler calls the layer.
3. Add the CLI command and MCP tool. A command that spends or changes things needs `--yes`.
4. Tests: allowed role writes and audits once; denied role writes nothing and audits the denial; the event is published after the commit and not on a denial; no secret in the entry or the event; the audit row and the change roll back together.

## Open

- Whether an audit entry for a denied read is kept, or only for denied writes. The organization rules say every write; reads are not settled.

Today's path, one `AppService` method per write with no actor, is in [today.md](today.md).
