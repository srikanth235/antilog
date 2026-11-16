# A platform job

A platform job is work the server does on an agent platform through the one platform trait: start a session, steer it, answer its `measure` calls, run an evaluator sandbox, stop and delete. The server never does the work itself, never runs a sandbox and never connects to data. (target: nothing here is built yet.)

## The contract (target)

| Part | Rule |
| --- | --- |
| Queue | A row in the store (Postgres self-hosted, the organization's Durable Object in the cloud), queued by an action in the same transaction as the change that caused it. |
| Worker | The server takes the job and calls the platform over HTTPS, through the trait. No platform calls the server: a `measure` call arrives on the session's event stream, which the server reads. |
| Sent to the platform | Ids, the repo and branch, the agent definition, the environment id, limits, and references to credentials in the platform's credential store. Never a secret, never data. |
| Result back | Ids, numbers, states, commit ids and spend. A diff, a transcript or a log is not a result: it is fetched from the platform when someone asks, and not stored. |
| Idempotent | A job carries an id and a lease. Running it twice leaves one outcome, so a server that restarts can take it again. A session is found again by its run and agent tag, not started twice. |
| Run tag | Every session and sandbox a job creates is tagged with its run and agent, before anything else happens to it. |
| Caps | The server stops a session at its share from usage events. The platform's own limits (turns, time, tokens) are the second stop. |
| Clean-up | A job that creates a session or a sandbox ends with a verified-deleted result, or reports what it could not delete. |
| Credentials | Handed over only through the platform's credential store. An agent session gets the read-only, holdout-excluding connector credential and a short-lived git token for its own branch. Only an evaluator sandbox gets the credential with the holdout. A job type never mixes them. |

## Job kinds (target)

- **Start a drafting session.** Repo, config branch, agent, read-only connector credentials. Ends when the session pushes the experiment's files.
- **Start a run session.** One per agent, from the config branch, with its agent branch, budget share and limits.
- **Steer.** Send a note to one session or all of them. Send an answer to a client tool call.
- **Measure.** On a `measure` call, start an evaluator sandbox on the measuring platform with the trial's commit and the evaluator's credential, run the evaluation with no agent in it, read the numbers, remove the sandbox, and answer the tool call. A Kept trial is measured again in a fresh sandbox and on the holdout.
- **Stop.** End and delete a session at its share, its turn cap or the run's deadline.
- **Clean up.** List what carries the run's tag on each platform and delete it.
- **Fetch.** Read a transcript or a log from the platform for a person who asked, without storing it.

## Adding a job (target)

1. Name the job and write its payload and result types in `crates/ratchet-core/src/jobs/`.
2. Add the table or state it needs by [ledger-table.md](ledger-table.md), and the action that queues it by [write-action.md](write-action.md).
3. Implement the handler behind the platform trait (see [infra/add-platform.md](../infra/add-platform.md)). Platform words stay in the adapter.
4. Report progress as writes to the store, so the action layer publishes events. See [live-event.md](live-event.md).
5. Tests: against the fake platform (the job is queued, taken, finished, and a lost lease is retaken without a second session); the shared platform contract suite for every adapter; a test that the payload and result carry no secret; a test that the server opens no connection to a connector; a test that no session is given the evaluator's credential.

## How the server follows a session (target)

- The server reads each session's events by streaming or polling the platform over HTTPS, whichever the adapter offers. Self-hosted servers behind a firewall work because every connection goes out.
- A session whose events stop for a set time (a starting default of 5 minutes) is marked as needing attention. The server does not start a replacement on its own; it keeps enforcing caps and clean-up from what it knows.
