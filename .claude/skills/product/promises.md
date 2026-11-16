# The promises

What every area owes the customer, and the one rule each follows. A promise kept in some places and broken in others is a broken promise. Each area's `owes.md` states its promises in full; this page is the index.

| Area | Promise | The rule |
| --- | --- | --- |
| Optimization | **Decide** | Only trials that beat the agent's best, are confirmed, and hold on the holdout move forward. Only the evaluator's numbers count. |
| Optimization | **Receipt** | Every win carries its full measurements, the dataset versions and the machines it ran on. |
| Optimization | **Honest numbers** | Pending is never counted as delivered. |
| Optimization | **Keep** | A win is re-checked and never slips back. |
| Context | **Tamper-proof** | A proposer never reads the holdout; its credentials cannot. |
| Context | **Pinned** | Every run names its dataset versions. |
| Context | **Stays put** | The server never connects to data. Data is read only by agent and evaluator code inside an agent platform's sandbox: the customer's own platforms when self-hosted, Ratchet's on Ratchet cloud. Sandboxes are removed when their session ends, and Ratchet keeps nothing. |
| Context | **Read-only** | Writing out needs an explicit, audited action. |
| Infrastructure | **Spend** | No spend without a limit. |
| Infrastructure | **Clean up** | Everything created is removed, and shown. |
| Infrastructure | **Least access** | Ratchet holds only agent platform credentials: the customer's when self-hosted, its own on Ratchet cloud. No cross-account role, no cloud keys, no connector admin secrets (target). |
| Infrastructure | **Any platform** | The same promises hold on every agent platform. |
| Organization | **Permission** | Every read and write is role-checked. |
| Organization | **Audit** | Every write is a logged action. |
| Organization | **Secrets** | No response, event or log carries one. |
| Organization | **No surprise calls** | Only configured endpoints are contacted. |
| Every client | **Same everywhere** | The web app (target), the CLI and MCP agree. |
| Every client | **Preview first** | A command that spends or changes things shows what it would do first. |

Owners: [optimization](../optimization/owes.md), [context](../context/owes.md), [infrastructure](../infra/owes.md), [organization](../organization/owes.md). The two client promises belong to every area that adds a command or a screen; see [design](../design/SKILL.md) and [change](../change/SKILL.md).

## Kept or owed

A machine-checked register of which promise is kept, with the file that proves it, is to be added. Until it exists, only these checks are confirmed in the repository today. Everything else is owed.

| Promise | What exists today |
| --- | --- |
| **Secrets** (in part) | `crates/ratchetd/tests/api/real_service.rs`, test `secrets_never_leave`: no response, event or error carries a stored key or token. `crates/ratchetd/tests/api/sim.rs` checks that settings never return a stored key. `api/CONTRACT.md` lists "no response can carry a secret" among the contract checks. It covers the daemon's own secret names, not a general scan. |
| **Preview first** (in part) | The CLI takes a global `--yes` (`crates/ratchet-cli/src/cli.rs`); without it, `run`, `stop`, `env end` and `pr` print what they would do and exit 2. `ratchet mcp` (`crates/ratchet-cli/src/mcp.rs`) mirrors it: those tools preview until the call carries `confirm: true`. `crates/ratchet-cli/tests/contract.sh` checks that a changing command needs `--yes`. |

Owed, with nothing checking it today: Decide, Receipt, Honest numbers, Keep, Tamper-proof, Pinned, Stays put, Read-only, Spend, Clean up, Least access, Any platform, Permission, Audit, No surprise calls, Same everywhere. Some are partly true in the current daemon (the evaluator sandbox and teardown are built for AWS), but no check ties the code to the promise. Do not call one kept until a check does.
