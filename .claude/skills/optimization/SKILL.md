---
name: optimization
description: >-
  What does "better" mean, and how is a win decided? Use for the goal and its direction, limits that must not get worse, what may change (the subject), the benchmark, the accept rule, the stop rule, runs, agents' loops, trials and their outcomes (Kept), verdicts, wins, receipts, delivered and pending value, and Return.
---

# Optimization

An experiment is a goal with a trusted benchmark. Each agent runs its own loop on a subject (change, measure, keep or discard, repeat) (target); code decides which trials count and which win. This area depends on context and infra by id and version only; they do not depend on it.

| If you are doing | Read |
| --- | --- |
| Deciding what this area promises | [owes.md](owes.md) |
| Looking up a word (goal, limit, run, trial, kept, verdict, win), or how a run works | [model.md](model.md) |
| Checking a change against the rules that never bend | [rules.md](rules.md) |
| Finding where this lives in the repo today, and what is only target | [today.md](today.md) |
| Where an evaluation template and the draft, then Approve flow belong | [design behaviour](../design/behaviour.md): a template is added under Customize and picked in New experiment; the draft is created at once and nothing is created in the cloud until Approve (target) |
| Adding a new kind of benchmark | [add-benchmark-kind.md](add-benchmark-kind.md) |
| Adding a new kind of thing that may change | [add-subject-kind.md](add-subject-kind.md) |
| Changing the accept rule or the stop rule | [change-accept-rule.md](change-accept-rule.md) |
| What a proposer may know, datasets, holdout | [../context/SKILL.md](../context/SKILL.md) |
| Where work runs, machines, spend caps | [../infra/SKILL.md](../infra/SKILL.md) |
| Who may do what, audit | [../organization/SKILL.md](../organization/SKILL.md) |
| Deciding whether a change belongs in the product | [../product/SKILL.md](../product/SKILL.md) |
| Making the change, then proving it | [../change/SKILL.md](../change/SKILL.md), [../land/SKILL.md](../land/SKILL.md) |
