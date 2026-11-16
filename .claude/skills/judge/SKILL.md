---
name: judge
description: >-
  What does "better" mean, and how is a win decided? Use for experiments (the metric and its direction, the evaluation and its protected paths, guardrails, the data and the holdout, repeats, the acceptance criteria), sealing and seal checks, trials and their outcomes (Kept), measurements, the judge sandbox and the judge platform, the seal, measure, history, deliver and watch tools, receipts, wins, the second opinion, regressions, delivered and pending value, and Return.
---

# Judge

Ratchet is a chat and a judge. The judge is the part of the server that measures: an agent asks it to measure a change, and only its numbers count. Code decides which trials are Kept and which is the win (target). The judge depends on the session area by id only, and the session area calls the judge through five tools.

| If you are doing | Read |
| --- | --- |
| Deciding what the judge promises | [owes.md](owes.md) |
| Looking up a word (experiment, seal, trial, measurement, outcome, win, receipt, watch) or what is stored | [model.md](model.md) |
| Checking a change against the rules that never bend | [rules.md](rules.md) |
| Adding or changing a judge tool (seal, measure, history, deliver, watch) | [tools.md](tools.md) |
| Working on the sealed experiment, the judge sandbox, seal checks, confirmation, the second opinion or the receipt | [anti-cheat.md](anti-cheat.md) |
| Changing the acceptance criteria | [change-acceptance.md](change-acceptance.md) |
| Adding a kind of evaluation | [add-evaluation-kind.md](add-evaluation-kind.md) |
| Finding what the earlier code did, and what it taught | [today.md](today.md) |
| `spawn`, the setup tools, platforms, agents, environments, connectors, credentials | [../session/SKILL.md](../session/SKILL.md), [../session/tools.md](../session/tools.md) |
| Cards, policy, who may approve, the audit log | [../organization/approvals.md](../organization/approvals.md), [../organization/SKILL.md](../organization/SKILL.md) |
| How the experiment, scoreboard, receipt and holdout marking look | [../design/behaviour.md](../design/behaviour.md) |
| Deciding whether a change belongs in the product | [../product/SKILL.md](../product/SKILL.md) |
| Making the change, then proving it | [../change/SKILL.md](../change/SKILL.md), [../land/SKILL.md](../land/SKILL.md) |
