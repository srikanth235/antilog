---
name: context
description: >-
  What may a proposer know? Use for sources (connections to outside systems), datasets and their versions, grants and audiences, markings such as holdout, skill packs (versioned know-how agents read, shown as Skills), notes from people, the history of past attempts, write-back actions such as opening a pull request, and adding a Snowflake, BigQuery, Postgres, object storage, GitHub, GitLab, monitoring or MCP source.
---

# Context

Context decides what a proposer may read. It follows Foundry: an experiment never reads a connector directly. A source produces a governed, versioned dataset, and the experiment reads the dataset. Most of this is target; [today.md](today.md) says what exists.

| If you are doing | Read |
| --- | --- |
| Deciding what this area promises | [owes.md](owes.md) |
| Looking up a word (source, skill pack, dataset, grant, marking, holdout) | [model.md](model.md) |
| Checking a change against the rules that never bend | [rules.md](rules.md) |
| Finding what exists in the repo today, and what is only target | [today.md](today.md) |
| Connecting a new outside system | [add-source.md](add-source.md) |
| Adding a new kind of dataset | [add-dataset-kind.md](add-dataset-kind.md) |
| Adding a skill pack, or a plugin's skills | [add-skill-pack.md](add-skill-pack.md) |
| Adding a new marking, or changing who a marking hides data from | [add-marking.md](add-marking.md) |
| What a benchmark does with a dataset | [../optimization/SKILL.md](../optimization/SKILL.md) |
| Where reads run | [../infra/SKILL.md](../infra/SKILL.md) |
| Who may grant, and how grants are audited | [../organization/SKILL.md](../organization/SKILL.md) |
| Deciding whether a data feature belongs in the product | [../product/SKILL.md](../product/SKILL.md) |
