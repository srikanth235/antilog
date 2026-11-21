# The promises

What every area owes the customer, and the one rule each follows. A promise kept in some places and broken in others is a broken promise. Each area's `owes.md` states its promises in full and its `rules.md` holds the never-bend rules; this page is the index.

| Area | Promise | The rule |
| --- | --- | --- |
| Session | **Stays put** | The server never connects to data. Data is read only by code inside the session's machine. The server keeps ids, states, the conversation's words, spend and audit rows, never tool output, file contents, diffs or data. |
| Session | **Least access** | Credentials reach a session only through the server, never a prompt, a repo file or Antilog's database. A model key never enters a machine. |
| Session | **Spend** | Model spend is booked by the proxy from each reply's usage, and machine time is booked beside it. A session never spends past its own cap, its project's monthly cap or the organization's monthly cap: the proxy refuses the call. A person can change a session's cap at any time, inside the caps above it. |
| Session | **Clean up** | A session's machine is released when idle or stopped, or past its maximum lifetime, and the conversation outlives it. |
| Organization | **Permission** | Every read and write is role-checked at project scope, with the organization role as the default, and roles decide who may approve which card. |
| Organization | **Approval** | A tool call that matters asks a person unless policy pre-approved it. Add credential can never be pre-approved. |
| Organization | **Audit** | Every tool call that changed something, and every approval, is logged: who, what, when, the outcome. |
| Organization | **Secrets** | No response, event or log carries one. |
| Organization | **No surprise calls** | Only configured endpoints are contacted. |
| Every client | **Same everywhere** | The web app and the API agree: same words, same state. |
| Every client | **Preview first** | A card that spends or changes things shows what it would do first. |

Owners: [session](../session/owes.md) ([rules](../session/rules.md)) and [organization](../organization/owes.md) ([rules](../organization/rules.md)). The client promises belong to every area that adds a tool or a screen; see [design](../design/SKILL.md) and [change](../change/SKILL.md).

## Parked

The judge's promises (Decide, Sealed, Tamper-proof, Receipt, Honest numbers, Keep) and the promises of spending well by trial, of no tie to one provider by scoreboard, and of no vendor claims are parked with the judge. Read them in `daa445f` (`.claude/skills/judge/owes.md`, `.claude/skills/product/promises.md`).
