# A runner job

A job is work the server queues and the runner executes in the customer's infrastructure: start a workspace, start an evaluator, remove a run's sandboxes, fetch a diff or a log. The server never does the work itself and never connects to data. (target: nothing here is built yet.)

## The contract (target)

| Part | Rule |
| --- | --- |
| Queue | A Postgres row, queued by an action in the same transaction as the change that caused it. |
| Delivery | The runner long-polls or streams over HTTPS. Self-hosted, the launcher in the server takes the same jobs in process. |
| Payload down | Ids, versions, the environment id, the budget cap, the scopes to mint. Never a secret, never a credential, never data. |
| Result up | Ids, numbers, states and spend. A diff or a log is not a result: it is fetched through the runner when asked for, and not stored. |
| Idempotent | A job carries an id and a lease. Running it twice leaves one outcome, so a runner that restarts can take it again. |
| Run tag | Every resource a job creates is tagged with its run, before anything else happens to it. |
| Caps | The runner enforces the budget cap and the clean-up check on what it already runs, even if the server is away. |
| Clean-up | A job that creates sandboxes ends with a verified-removed result, or reports what it could not remove. |
| Credentials | The runner mints them from the customer's secret manager. An agent job gets the scoped, read-only, holdout-excluding one. Only an evaluator job gets the one with the holdout. A job type never mixes them. |

## Adding a job (target)

1. Name the job and write its payload and result types in `crates/ratchet-server/src/jobs/`, shared with `ratchet-runner`. Both sides use the same types.
2. Add the table or state it needs by [ledger-table.md](ledger-table.md), and the action that queues it by [write-action.md](write-action.md).
3. Implement the handler in `crates/ratchet-runner/src/`, behind the backend trait (see [infra/add-backend.md](../infra/add-backend.md)). Provider words stay in the backend.
4. Report progress as writes to the server, so the action layer publishes events. See [live-event.md](live-event.md).
5. Tests: with a fake runner against the real server (the job is queued, taken, finished, and a lost lease is retaken); with the real runner against fake backends (tags, caps, clean-up); a test that the payload and result carry no secret; a test that the server opens no connection to a connector.
6. Version the contract. A runner may be older than the server, so a new job type is additive and an old runner refuses a job it does not know with a visible state.

## Decided

- The first transport is long-poll over HTTPS, because it works through every proxy. A stream can come later behind the same contract.
- A runner sends a heartbeat with each poll. After 5 minutes without one, the server marks the runner away and its runs as needing attention. The server starts nothing to replace it; the runner keeps enforcing caps and clean-up on what it already runs.
