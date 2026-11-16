# Judge: the tools

Five tools every session gets (target). `ratchet mcp` serves the same tools to an agent a person already uses on their own machine, so a change can be proposed anywhere and is measured only by the judge. `spawn` and the setup tools (`attach_repo`, `request_credential`, `set_environment`, `request_budget`) are in [../session/tools.md](../session/tools.md).

| Tool | Asks a person? |
| --- | --- |
| `seal` | Yes, once per experiment version |
| `measure` | No, inside the budget |
| `history` | No |
| `deliver` | By policy |
| `watch` | No |

## seal

- **In:** a proposed definition for the experiment, which may start from an evaluation template (id and version): the metric and direction, the evaluation (command, protected paths with their repos, pinned commits), guardrails, dataset ids and versions including the holdout, repeats, and any planted cheats the proposer adds.
- **Back:** the seal check results (baseline, noise, each cheat), shown on a Seal experiment card, which also carries a Budget row. After approval, the experiment version and its seal. Until then the experiment is Not sealed.
- **Asks:** yes. A person seals it from the card.
- **Refuses:** a definition that fails a seal check. An unpinned commit or dataset version. A holdout the agent's own credential can reach.

## measure

- **In:** the experiment version, the commits (one per repo changed), the trial it started from, and a one-line hypothesis.
- **Back:** a ticket at once. The measurement arrives later as an event in the session: the outcome, the measured value and guardrail values on the evaluation data, the noise band. Never a holdout value.
- **Asks:** no, inside the budget.
- **Refuses:** when the experiment's budget is spent (its sessions become Waiting for you with a Raise budget card). A trial that starts from a trial that is not on this experiment version. An unsealed experiment. No platform tool timeout applies, because the call returns a ticket.

## history

- **In:** an experiment, and optionally a filter by session, outcome or lineage.
- **Back:** every trial for the experiment across all its sessions: hypothesis, outcome, values, commit ids, what it started from. Failures included. Never a holdout value, never a diff (the agent reads diffs from git by commit id).
- **Asks:** no. **Refuses:** nothing beyond role and organization checks. This is the notebook: there is no separate notes feature.

## deliver

- **In:** a trial, normally the win.
- **Back:** one pull request per repo changed, linked to each other, each carrying the receipt. The session stays attached: review comments and CI failures arrive as messages, and a rebase is measured again before the receipt is updated.
- **Asks:** by policy, with an Open pull request card.
- **Refuses:** any trial that is not Kept. A win still waiting for its second opinion.

## watch

- **In:** a delivered experiment, and a schedule or "when the default branch moves".
- **Back:** a watch id. Each result is a measurement.
- **Asks:** no. A regression (worse than the delivered value by more than the noise) wakes a session, or starts a new one in the experiment, and sends a notification.
- **Refuses:** an experiment that was never delivered.

## Rules for every judge tool

- Each call that changes something is an audit row (see [../organization/SKILL.md](../organization/SKILL.md)).
- A tool reads and writes ids, hashes, numbers and states only.
- A refusal says why and what to do. It is never shown as an empty result.
