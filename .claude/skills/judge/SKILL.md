---
name: judge
description: >-
  What does "better" mean, and how is a win decided? Use for experiments (the metric and its direction, the evaluation and its protected paths, evaluation templates, guardrails, the data and the holdout, repeats, the acceptance criteria), sealing and seal checks, trials and their outcomes (Kept), measurements, the seal, measure, history, deliver and watch tools, receipts, wins, the second opinion, regressions, delivered, pending and at-risk value, the value model, the quick check, patience, cost per trial, and Return.
---

# Judge

Antilog is a chat and a judge. The judge is the part of the server that measures: an agent asks it to measure a change, and only its numbers count. Code decides which trials are Kept and which is the win: `antilog_core::judge::acceptance::decide`, one pure function. Types are in `crates/antilog-types/src/judge/`, logic in `crates/antilog-core/src/judge/`, and the `JudgeStore` trait with its fake in `crates/antilog-core/src/store/judge.rs`. Its Postgres tables are in `crates/antilog-server/src/pg/judge.rs`, the tools in `crates/antilog-core/src/tools/` and a Kept win gets its signed receipt at `GET /trials/{id}/receipt`. Value is built: value models, Delivered, Pending, At risk, Return and the Report are derived in `crates/antilog-core/src/value.rs` and served by `crates/antilog-server/src/api/value.rs`. Not built yet: a real code host for delivery (a fake only), watch and the second opinion. The judge depends on the session area by id only, and the session area calls the judge through five tools.

| If you are doing | Read |
| --- | --- |
| Deciding what the judge promises | [owes.md](owes.md) |
| Looking up a word (experiment, seal, value model, quick check, patience, trial, measurement, outcome, win, receipt, watch, Return) or what is stored | [model.md](model.md) |
| Checking a change against the rules that never bend | [rules.md](rules.md) |
| Adding or changing a judge tool (seal, measure, history, deliver, watch) | [tools.md](tools.md) |
| Working on the sealed experiment, seal checks, confirmation, the second opinion or the receipt | [anti-cheat.md](anti-cheat.md) |
| Changing the acceptance criteria | [change-acceptance.md](change-acceptance.md) |
| Adding a kind of evaluation or an evaluation template (listed in Customize, Evaluations) | [add-evaluation-kind.md](add-evaluation-kind.md) |
| Skill packs and plugins (the judge never reads a pack, and a plugin never changes what counts) | [../session/add-skill-pack.md](../session/add-skill-pack.md) |
| the setup tools, agents, environments, connectors, credentials | [../session/SKILL.md](../session/SKILL.md), [../session/tools.md](../session/tools.md) |
| Cards, policy, who may approve, the audit log | [../organization/approvals.md](../organization/approvals.md), [../organization/SKILL.md](../organization/SKILL.md) |
| How the experiment, scoreboard, receipt and holdout marking look | [../design/behaviour.md](../design/behaviour.md) |
| Deciding whether a change belongs in the product | [../product/SKILL.md](../product/SKILL.md) |
| Making the change, then proving it | [../change/SKILL.md](../change/SKILL.md), [../land/SKILL.md](../land/SKILL.md) |
