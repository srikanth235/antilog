# What infra owes

| Promise | The rule |
| --- | --- |
| Spend | Nothing is spent without a stated limit. A cap stops the run. Every dollar is tied to a run and an agent, and each agent has its share (target). |
| Clean up | Every session and sandbox Ratchet starts is ended, deleted on its platform, and shown as removed (target). |
| Least access | Ratchet holds only agent platform credentials: the customer's when self-hosted, its own on Ratchet cloud. No cross-account role, no cloud keys, no connector admin secrets (target). |
| Any platform | The same promises hold on every agent platform (target). |

## How each is checked

All four are owed: nothing on this branch checks them. What the earlier code had is in [today.md](today.md).

**Spend.** Check: a run cannot start without a budget; Ratchet stops a session at its share; every spend row carries the run and the agent and is read from the platform's usage. The earlier code stopped a run at its budget (`crates/ratchet-engine/src/tick.rs`) and kept spend append-only (`crates/ratchet-core/src/cost.rs`).

**Clean up.** Check: after a run, listing the platform's sessions and sandboxes tagged with the run finds nothing, and the page shows removed. The earlier code's reaper did this for AWS instances (`crates/ratchet-sandbox/src/reaper.rs`).

**Least access.** Check: no path stores a cloud key, a cross-account role or a connector admin secret, and no response, event or log carries a platform credential.

**Any platform.** Check: one contract suite that every platform adapter must pass, run against a fake platform in CI and against each real platform with the owner's go-ahead (see [../land/real-cloud.md](../land/real-cloud.md)).
