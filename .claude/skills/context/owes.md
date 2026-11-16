# What context owes

| Promise | The rule |
| --- | --- |
| Tamper-proof | A proposer never reads holdout data. Its credentials cannot reach it, and nothing a proposer reads, a skill pack, the history and a measurement result included, carries it. |
| Pinned | Every run names the exact dataset versions and skill pack versions it read. |
| Stays put | The server never connects to data. Data is read by agent and evaluator code inside an agent platform's sandbox: the customer's own platforms when self-hosted, Ratchet's on Ratchet cloud (target). Sandboxes are removed when their session ends. It never passes through the Ratchet server. |
| Read-only | Agent credentials are read-only. Writing out, for example opening a pull request, is a separate, explicit, audited action. |

## How each is checked

**Tamper-proof.** Check: a test that an agent identity cannot read a holdout-marked dataset, run against the single place that enforces grants. Partly exists for the sealed bucket: the infra module denies agent roles, covered by `infra/tests/src/agent-boundary.test.ts` and `infra/aws/tests/sandbox.tftest.hcl`. A check at the credential level is to add: the credential an agent gets is refused by the connector when it reads holdout data, run against each connector. Datasets and connectors do not exist yet.

**Pinned.** Check: a test that a run record lists a version for every dataset the benchmark read and every skill pack agents read, and that a read of an unpinned name fails. To add. The ledger has no dataset versions today.

**Stays put.** Check: a test that no Ratchet server route returns dataset rows, and that the server holds no connector credential and opens no connection to an outside system. To add. Today the daemon runs on the user's machine and there is no Ratchet-operated server, so the promise holds by construction, not by test.

**Read-only.** Check: every connector's agent credential is read-only, a test per connector tries a write and expects a refusal, and every write-back is a named action in the audit log. To add. Today the one write-out, a pull request, is made by the engine through `crates/ratchet-github/src/github_host.rs` with no audit entry.
