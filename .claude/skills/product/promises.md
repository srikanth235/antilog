# The promises

What every area owes the customer, and the one rule each follows. A promise kept in some places and broken in others is a broken promise. Each area's `owes.md` states its promises in full and its `rules.md` holds the never-bend rules; this page is the index. All of it is target.

| Area | Promise | The rule |
| --- | --- | --- |
| Judge | **Decide** | Only a trial that beats what it started from, holds every guardrail and is confirmed in a fresh checkout and on the holdout is Kept. Only the judge's numbers count. |
| Judge | **Sealed** | A sealed experiment never changes. A new version starts a new record, and numbers never carry across versions. |
| Judge | **Tamper-proof** | An agent never holds the holdout credential or learns a holdout value. A trial that changes a protected path is Rejected. A skill pack or a plugin never changes what counts. |
| Judge | **Receipt** | Every win carries a signed receipt: the experiment version and seal, commits, values, repeats, noise, trials tried, the holdout result, the second opinion and what was spent. |
| Judge | **Honest numbers** | Pending is never counted as delivered. Return counts everything spent in its scope, and a regressed win is At risk, not Delivered. A value model is a person's, never an agent's. |
| Judge | **Keep** | A delivered win is watched, and a regression wakes a session in the experiment. |
| Session | **Stays put** | The server never connects to data. Data is read only by code inside the session's machine. The server keeps ids, versions, numbers, states, the conversation's words and audit rows, never tool output, file contents, diffs or data. |
| Session | **Least access** | Credentials reach a session only through the server, never a prompt, a repo file or Antilog's database. An agent gets read-only connector credentials that leave out the holdout, and a short-lived git token that pushes only to its own branch. |
| Session | **Spend** | No spend without a budget, one per experiment across all its sessions, inside its project's cap and the organization's. A person can change it at any time. When it is spent the judge refuses `measure` and the sessions wait for a person. |
| Session | **Spend well** | Cost per trial is shown, a measurement is estimated before it is bought, a quick check precedes the full one, and the experiment pauses when trials stop paying. No agent is favoured: Auto says why it chose. |
| Session | **Clean up** | A session's machine is paused when idle and reaped when stopped or past its maximum lifetime, and the conversation outlives it (target). |
| Organization | **Permission** | Every read and write is role-checked at project scope, with the organization role as the default, and roles decide who may approve which card. |
| Organization | **Approval** | A tool call that matters asks a person unless policy pre-approved it. Add credential can never be pre-approved, and a value model is approved by a project owner. |
| Organization | **Audit** | Every tool call that changed something, and every approval, is logged: who, what, when, the outcome. |
| Organization | **Secrets** | No response, event or log carries one. |
| Organization | **No surprise calls** | Only configured endpoints are contacted. |
| Every client | **Same everywhere** | The web app, the CLI and MCP agree: same tools, same words. |
| Every client | **No vendor claims** | A figure about an agent is measured by the judge on the customer's own code. We publish no vendor benchmark. |
| Every client | **Preview first** | A command or card that spends or changes things shows what it would do first. |

Owners: [session](../session/owes.md) ([rules](../session/rules.md)), [judge](../judge/owes.md) ([rules](../judge/rules.md)), [organization](../organization/owes.md) ([rules](../organization/rules.md)). The two client promises belong to every area that adds a tool or a screen; see [design](../design/SKILL.md) and [change](../change/SKILL.md).

## Kept or owed

Nothing is built on this branch, so no promise is kept yet. A machine-checked register of which promise is kept, with the file that proves it, is to be added with the first code. Until a check ties code to a promise, it is owed. Do not call one kept before then.
