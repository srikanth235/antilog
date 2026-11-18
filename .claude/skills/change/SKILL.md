---
name: change
description: >-
  How does this kind of change travel through the layers? Use to add or change an API endpoint, a tool, a write, a live event, a ledger table or migration, a platform job, or a screen; or to add an evaluation kind or template, a connector, a skill pack, a plugin kind, a Customize tab, an agent platform, an agent, an environment, a project, a role (organization or project), a sign-in provider, an audited action, a tool or a card kind.
---

# Change

A change crosses layers that must agree: Rust types, the generated OpenAPI document, the generated TypeScript client, the Antilog tools (served to a session and by `antilog mcp`), the CLI, the action layer, the store (Postgres), the platform job interface, the boards, the tests and the docs. Checks fail until they do. Find the row, follow it, then prove it with [land](../land/SKILL.md).

The pages describe the target: `antilog-core`, the server `antilog-server` (one Helm chart for Antilog cloud and self-hosted), and `antilog` (the CLI and MCP). Nothing of it is built on this branch, so each path and command that does not exist yet is marked (target).

| If you are doing | Read |
| --- | --- |
| An endpoint, a field, a request or response type | [api-endpoint.md](api-endpoint.md) |
| A tool (one of the Antilog tools), new or changed | [tool.md](tool.md) |
| A write (a command that changes state) | [write-action.md](write-action.md) |
| A live event the clients react to | [live-event.md](live-event.md) |
| A table, column or migration | [ledger-table.md](ledger-table.md) |
| A job the server queues and runs on an agent platform (the judge's queue, the agent trait calls) | [platform-job.md](platform-job.md) |
| A screen or a page | [screen.md](screen.md) |
| A card kind (an approval card or a result card) | [tool.md](tool.md), then [organization/approvals.md](../organization/approvals.md) |
| An evaluation kind or an evaluation template (Customize, Evaluations) | [judge/add-evaluation-kind.md](../judge/add-evaluation-kind.md) |
| The acceptance criteria | [judge/change-acceptance.md](../judge/change-acceptance.md) |
| A connector | [session/add-connector.md](../session/add-connector.md) |
| A skill pack | [session/add-skill-pack.md](../session/add-skill-pack.md) |
| A plugin, or a new kind of thing a plugin may bring | [session/add-skill-pack.md](../session/add-skill-pack.md) (a plugin has no powers of its own), then [screen.md](screen.md) for its Customize tab and detail page |
| A project, or a role in a project | [organization/add-role.md](../organization/add-role.md), then [organization/add-audited-action.md](../organization/add-audited-action.md) |
| An agent platform | [session/add-platform.md](../session/add-platform.md) |
| An agent | [session/add-agent.md](../session/add-agent.md) |
| An environment | [session/add-environment.md](../session/add-environment.md) |
| A role | [organization/add-role.md](../organization/add-role.md) |
| A sign-in provider | [organization/add-sign-in-provider.md](../organization/add-sign-in-provider.md) |
| An audited action | [organization/add-audited-action.md](../organization/add-audited-action.md) |
| How the language is written, which library | [stack](../stack/SKILL.md) |
| What the product should do first | [product](../product/SKILL.md) |
