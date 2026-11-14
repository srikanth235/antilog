---
name: change
description: >-
  How does this kind of change travel through the layers? Use to add or change an API endpoint, a write, a live event, a ledger table or migration, or a screen; or to add a benchmark kind, subject kind, accept rule, connector, dataset kind, marking, cloud, agent, environment, role, sign-in provider or audited action.
---

# Change

A change crosses layers that must agree: Rust types, the generated OpenAPI file, the CLI and MCP tool, tests, the design and the docs. Tests fail until they do. Find the row, follow it, then prove it with [land](../land/SKILL.md).

| If you are doing | Read |
| --- | --- |
| An endpoint, a field, a request or response type | [api-endpoint.md](api-endpoint.md) |
| A write (a command that changes state) | [write-action.md](write-action.md) |
| A live event the clients react to | [live-event.md](live-event.md) |
| A ledger table, column or migration | [ledger-table.md](ledger-table.md) |
| A screen or a page | [screen.md](screen.md) |
| A benchmark kind | [optimization/add-benchmark-kind.md](../optimization/add-benchmark-kind.md) |
| A subject kind (what may change) | [optimization/add-subject-kind.md](../optimization/add-subject-kind.md) |
| The accept rule | [optimization/change-accept-rule.md](../optimization/change-accept-rule.md) |
| A connector | [context/add-connector.md](../context/add-connector.md) |
| A dataset kind | [context/add-dataset-kind.md](../context/add-dataset-kind.md) |
| A marking (such as the holdout) | [context/add-marking.md](../context/add-marking.md) |
| A cloud | [infra/add-cloud.md](../infra/add-cloud.md) |
| An agent | [infra/add-agent.md](../infra/add-agent.md) |
| An environment | [infra/add-environment.md](../infra/add-environment.md) |
| A role | [organization/add-role.md](../organization/add-role.md) |
| A sign-in provider | [organization/add-sign-in-provider.md](../organization/add-sign-in-provider.md) |
| An audited action | [organization/add-audited-action.md](../organization/add-audited-action.md) |
| What the product should do first | [product](../product/SKILL.md) |
