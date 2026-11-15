---
name: context
description: >-
  What may a proposer know? Use for connectors (connections to outside systems), datasets and their versions, grants and audiences, markings such as holdout, skill packs (versioned know-how agents read, shown as Skills), notes from people (to all agents or one), the history of past trials (read by every agent live), write-back actions such as opening a pull request, and adding a Snowflake, Databricks, DynamoDB, BigQuery, Postgres, S3, GitHub, GitLab or MCP connector, and scoped credentials.
---

# Context

Context decides what a proposer may read. The coding agent writes code, and that code runs the experiment and reads data through connectors itself. Ratchet does not pull or broker data. Context names the datasets and their versions, and decides which credentials the launcher mints: short-lived, read-only, scoped, and without the holdout for agents. Most of this is target; [today.md](today.md) says what exists.

| If you are doing | Read |
| --- | --- |
| Deciding what this area promises | [owes.md](owes.md) |
| Looking up a word (connector, scoped credential, skill pack, dataset, grant, marking, holdout) | [model.md](model.md) |
| Checking a change against the rules that never bend | [rules.md](rules.md) |
| Finding what exists in the repo today, and what is only target | [today.md](today.md) |
| Connecting a new outside system, and how it mints a scoped credential | [add-connector.md](add-connector.md) |
| Adding a new kind of dataset | [add-dataset-kind.md](add-dataset-kind.md) |
| Adding a skill pack, or a plugin's skills | [add-skill-pack.md](add-skill-pack.md) |
| Adding a new marking, or changing who a marking hides data from | [add-marking.md](add-marking.md) |
| What a benchmark does with a dataset | [../optimization/SKILL.md](../optimization/SKILL.md) |
| Where the code that reads runs, and who mints credentials | [../infra/SKILL.md](../infra/SKILL.md) |
| Who may grant, and how grants are audited | [../organization/SKILL.md](../organization/SKILL.md) |
| Deciding whether a data feature belongs in the product | [../product/SKILL.md](../product/SKILL.md) |
