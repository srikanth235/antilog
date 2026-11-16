---
name: change
description: >-
  How does this kind of change travel through the layers? Use to add or change an API endpoint, a write, a live event, a ledger table or migration, a platform job, or a screen; or to add a benchmark kind, subject kind, accept rule, connector, dataset kind, marking, agent platform, agent, environment, role, sign-in provider or audited action.
---

# Change

A change crosses layers that must agree: Rust types, the generated OpenAPI document, the generated TypeScript client, the CLI and MCP tool, the action layer, the store (Postgres and SQLite), the platform job contract, the boards, the tests and the docs. Checks fail until they do. Find the row, follow it, then prove it with [land](../land/SKILL.md).

The pages describe the target: `ratchet-core` with its two shells `ratchet-server` (self-hosted) and `ratchet-cloud` (Ratchet cloud), and `ratchet` (the CLI and MCP). Nothing of it is built on this branch, so each path and command that does not exist yet is marked (target). What the earlier daemon did, and what it taught, is in [today.md](today.md).

| If you are doing | Read |
| --- | --- |
| An endpoint, a field, a request or response type | [api-endpoint.md](api-endpoint.md) |
| A write (a command that changes state) | [write-action.md](write-action.md) |
| A live event the clients react to | [live-event.md](live-event.md) |
| A table, column or migration | [ledger-table.md](ledger-table.md) |
| A job the server queues and runs on an agent platform | [platform-job.md](platform-job.md) |
| A screen or a page | [screen.md](screen.md) |
| A benchmark kind | [optimization/add-benchmark-kind.md](../optimization/add-benchmark-kind.md) |
| A subject kind (what may change) | [optimization/add-subject-kind.md](../optimization/add-subject-kind.md) |
| The accept rule | [optimization/change-accept-rule.md](../optimization/change-accept-rule.md) |
| A connector | [context/add-connector.md](../context/add-connector.md) |
| A dataset kind | [context/add-dataset-kind.md](../context/add-dataset-kind.md) |
| A marking (such as the holdout) | [context/add-marking.md](../context/add-marking.md) |
| A skill pack | [context/add-skill-pack.md](../context/add-skill-pack.md) |
| An agent platform | [infra/add-platform.md](../infra/add-platform.md) |
| An agent | [infra/add-agent.md](../infra/add-agent.md) |
| An environment | [infra/add-environment.md](../infra/add-environment.md) |
| A role | [organization/add-role.md](../organization/add-role.md) |
| A sign-in provider | [organization/add-sign-in-provider.md](../organization/add-sign-in-provider.md) |
| An audited action | [organization/add-audited-action.md](../organization/add-audited-action.md) |
| How the language is written, which library | [stack](../stack/SKILL.md) |
| What the product should do first | [product](../product/SKILL.md) |
