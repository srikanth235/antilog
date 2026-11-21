---
name: session
description: >-
  How does the chat work, and where does an agent run? Use for sessions and their status (Queued, Working, Waiting for you, Done, Stopped, Failed), the conversation and what the server stores of it, a session's project, model and agent, attaching repos, environments, connectors and credentials, skill packs, the setup tools (attach_repo, request_credential, set_environment, request_budget), what agents are told, spend and a session's budget, the runner, clean-up, and adding an agent, an environment, a connector or a skill pack.
---

# Session

A session is one agent's work in a project: a chat with one agent in one machine, with any number of repos attached. It belongs to one project, and its repos, environment version, agent and model are fixed when it starts. The agent drives the work. The server keeps a ledger of the session and runs no loop. A runner polls the server's queue and runs the agent's harness in one machine per session ([docs/V0.md](../../../docs/V0.md)); how it works is in [runner.md](runner.md). Experiments and the judge are parked ([../judge/SKILL.md](../judge/SKILL.md)).

| If you are doing | Read |
| --- | --- |
| Deciding what this area promises | [owes.md](owes.md) |
| Looking up a word or a field (session, agent, model, environment, attached repo, budget, connector, tool mode, session endpoint, skill pack, plugin, session event) or a route | [model.md](model.md) |
| Checking a change against the rules that never bend | [rules.md](rules.md) |
| What a prompt-injected agent could do, and what stops it | [threat-model.md](threat-model.md) |
| Changing the queue, the leases, `/runner/*`, `antilog runner` or how OpenCode is driven | [runner.md](runner.md) |
| Changing `attach_repo`, `request_credential`, `set_environment` or `request_budget`, or how tools reach a session | [tools.md](tools.md) |
| Changing what agents are told | [prompts.md](prompts.md) |
| Supporting a new agent (a model run by a harness) | [add-agent.md](add-agent.md) |
| Adding or changing an environment (network, variables, setup script, size) | [add-environment.md](add-environment.md) |
| Adding a connector (a remote MCP server the session endpoint calls for an agent), its sealed credential and each tool's On, Off or Ask first | [add-connector.md](add-connector.md) |
| Adding a skill pack (a versioned `SKILL.md` folder agents load on demand), or a plugin that brings them | [add-skill-pack.md](add-skill-pack.md) |
| Cards, policy, roles, budgets, who may approve | [../organization/SKILL.md](../organization/SKILL.md) |
| How the chat and its cards look and read | [../design/behaviour.md](../design/behaviour.md) |
| Deciding whether a change belongs in the product | [../product/SKILL.md](../product/SKILL.md) |
