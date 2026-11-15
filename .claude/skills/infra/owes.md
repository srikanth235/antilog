# What infra owes

| Promise | The rule |
| --- | --- |
| Spend | Nothing is spent without a stated limit. A cap stops the run. Every dollar is tied to a run and an agent, and each agent has its share (target). |
| Clean up | Everything Ratchet creates is tagged, removed, and shown as removed. |
| Customer's account | Under Your infrastructure, work runs in the customer's infrastructure and Ratchet holds no access to it. Ratchet-managed runs in Ratchet's account, by the owner's choice (target). |
| Any cloud | The same promises hold on every sandbox backend. |

## How each is checked

**Spend.** Check: a run cannot start without a budget; the engine stops at the cap; spend rows carry the run. Partly exists: `budget_usd` is on every optimization and the tick stops the run when spend reaches it (`crates/ratchet-engine/src/tick.rs`); spend events are append-only and reconciled against billing (`crates/ratchet-core/src/cost.rs`, `crates/ratchet-ledger/tests/cost_reconcile.rs`, `crates/ratchet-sandbox/src/cost_explorer.rs`). Spend rows carry the optimization and agent, not a run id. The run id on spend and a refusal to start without a cap are to add.

**Clean up.** Check: after a run, a tag-scoped listing finds nothing, and the page shows removed. Partly exists: a reaper terminates tagged instances that should not exist (`crates/ratchet-sandbox/src/reaper.rs`, tests in `crates/ratchet-sandbox/tests/suite/reaper.rs`), and the teardown checks for orphans (`crates/ratchet-engine/tests/engine/sandbox_lifecycle.rs`). The reaper keys on `ratchet:opt`, not a run tag, and no screen shows "removed". The run tag and the verified-removed state are to add.

**Customer's account.** Check: sandboxes start only through the launcher in the customer's infrastructure, and under Your infrastructure no Ratchet-operated account, cross-account role or stored cloud key appears in any path. Ratchet-managed sandboxes start only for an organization whose owner chose it (target). Exists by construction: the app drives OpenTofu with the user's AWS profile (`infra/README.md`). A test that asserts it is to add.

**Any cloud.** Check: one contract test suite that every sandbox backend must pass (`infra/contract.md` lists the invariants). Exists for AWS only: `infra/tests/src/` and `infra/aws/tests/`. Kubernetes and other providers are target, so the check has never run across backends.
