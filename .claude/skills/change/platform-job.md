# A platform job

A platform job is work the server does on an agent platform. There are two traits. The agent trait starts a session with repos, an agent, an environment, credentials and tools, sends input, reads its events, and stops and deletes it. The judge trait runs a command at commits with secrets in a judge sandbox and returns numbers. The server runs no loop of its own: the agent drives, and the only server-side state machines are the judge's queue and the budget meter. The server never does the work itself, never runs a sandbox and never connects to data. (target: nothing here is built yet.)

## The interface (target)

| Part | Rule |
| --- | --- |
| Queue | A row in the store (Postgres), queued by an action in the same transaction as the change that caused it. |
| Worker | The server takes the job and calls the platform over HTTPS, through the trait. On pay-per-use a scheduler wake-up invokes it (the timer trait); on Kubernetes a worker in the process polls. No platform calls the server: a `measure` call arrives on the session's event stream, which the server reads. |
| Sent to the platform | Ids, the repos and branches, the agent definition, the environment id, limits, and references to credentials in the platform's credential store. Never a secret, never data. |
| Result back | Ids, numbers, states, commit ids and spend. A diff, a transcript or a log is not a result: it is fetched from the platform when someone asks, and not stored. |
| Idempotent | A job carries an id and a lease. Running it twice leaves one outcome, so a server that restarts can take it again. A session is found again by its session tag, not started twice. |
| Session tag | Every session and sandbox a job creates is tagged with its Antilog session, before anything else happens to it. |
| Caps | The budget meter reads usage events; when the budget is spent the judge refuses `measure` and the session is Waiting for you with a Raise budget card. The platform's own limits (turns, time, tokens) are the second stop. |
| Clean-up | A job that creates a session or a sandbox ends with a verified-deleted result, or reports what it could not delete. |
| Credentials | Handed over only through the platform's credential store. An agent session gets the read-only, holdout-excluding connector credential and a short-lived git token per repo for its own branch. Only a judge sandbox gets the credential with the holdout. A job type never mixes them. |

## Job kinds (target)

- **Start a session.** Repos, agent, environment, credentials, tools and the session brief (see [session/prompts.md](../session/prompts.md)). A person starts it from the composer; `spawn` starts others in the same experiment, on its budget.
- **Send input.** A message from a person, or an answer to a card, goes to the session.
- **Measure.** On a `measure` call, return a ticket, queue a judge job: start a judge sandbox on the judge platform with the trial's commits and the judge's credential, run the evaluation with no agent in it, read the numbers, remove the sandbox, and send the measurement to the session as an event. A trial that beats the bar is measured again in a fresh judge sandbox and on the holdout. No platform tool timeout applies, because the call has already returned.
- **Seal checks.** Measure the baseline, measure one commit twice for the noise, and run the planted cheats (see [judge/anti-cheat.md](../judge/anti-cheat.md)).
- **Second opinion.** Before delivery, an agent from a different model provider reads the diff.
- **Watch.** Measure a delivered experiment again on a schedule or when the default branch moves.
- **Stop.** End and delete a session when a person stops it, it is idle, or the budget is spent.
- **Clean up.** List what carries the session's tag on each platform and delete it.
- **Fetch.** Read tool output or a log from the platform for a person who asked, without storing it.

## Adding a job (target)

1. Name the job and write its payload and result types in `crates/antilog-core/src/jobs/`.
2. Add the table or state it needs by [ledger-table.md](ledger-table.md), and the action that queues it by [write-action.md](write-action.md).
3. Implement the handler behind the platform trait (see [session/add-platform.md](../session/add-platform.md)). Platform words stay in the adapter.
4. Report progress as writes to the store, so the action layer publishes events. See [live-event.md](live-event.md).
5. Make the handler resumable and idempotent: it saves its cursor, finishes inside about 10 minutes, and starts no work that outlives its response ([../stack/rust/antilog.md](../stack/rust/antilog.md)).
6. Tests: against the fake platform (the job is queued, taken, finished, and a lost lease is retaken without a second session); the shared platform test suite for every adapter, and the judge trait's own suite; a test that the payload and result carry no secret; a test that the server opens no connection to a connector; a test that no agent session is given the judge's credential.

## How the server follows a session (target)

- The server reads each session's events by streaming or polling the platform over HTTPS, whichever the adapter offers. On pay-per-use this runs as resumable steps of about 10 minutes (a Lambda invocation ends at 15): each step saves the platform's event cursor, does nothing after it returns, and books the next wake-up with the timer trait. A handler that waits on a measurement or a watch is a step too, never a loop. Self-hosted servers behind a firewall work because every connection goes out.
- A session whose events stop for a set time (a starting default of 5 minutes) is marked as needing attention. The server does not start a replacement on its own; it keeps enforcing caps and clean-up from what it knows.
