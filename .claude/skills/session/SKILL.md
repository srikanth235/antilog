---
name: session
description: >-
  How does the chat work, and where does an agent run? Use for sessions and their status (Working, Waiting for you, Done, Failed), the conversation and what the server stores of it, agents and agent platforms (Claude Managed Agents, AWS AgentCore, the OpenAI Agents API, Microsoft Foundry Agent Service, Google Agent Runtime) and their one trait, attaching repos, environments, connectors and credentials, sessions inside an experiment, spawned sessions, the setup tools (attach_repo, request_credential, set_environment, request_budget), what agents are told, spend and clean-up, and adding an agent platform, an agent, an environment or a connector.
---

# Session

A session is one agent's work inside an experiment: a chat with one agent in one sandbox on one agent platform, with any number of repos attached (target). It always belongs to an experiment and is not a top-level noun in the app: the sidebar lists experiments, and on screen a session is named by its agent ("Claude Code session"). A person talks to a coding agent the way they do in Claude Code on the web. The agent drives the work. The server relays the session, runs the judge and keeps a ledger, and runs no loop. Ratchet runs no sandboxes: each agent platform runs the agent's loop and its sandbox as one durable session, behind one trait. Self-hosted, Ratchet uses the customer's own platform accounts; on Ratchet cloud, its own. The judge, and what counts as a win, is in [../judge/SKILL.md](../judge/SKILL.md). The earlier code ran its own sandboxes; [today.md](today.md) says what it built.

| If you are doing | Read |
| --- | --- |
| Deciding what this area promises | [owes.md](owes.md) |
| Looking up a word (session, agent, agent platform, platform account, environment, attached repo, connector, credential reference, spawned session, session event) | [model.md](model.md) |
| Checking a change against the rules that never bend | [rules.md](rules.md) |
| Changing `spawn`, `attach_repo`, `request_credential`, `set_environment` or `request_budget`, or how tools reach a session | [tools.md](tools.md) |
| Changing what agents are told | [prompts.md](prompts.md) |
| Supporting a new agent platform | [add-platform.md](add-platform.md) |
| Supporting a new agent (a model on a platform) | [add-agent.md](add-agent.md) |
| Adding an environment (image, setup script, size, network) | [add-environment.md](add-environment.md) |
| Adding a connector (a data system or MCP server a session may read) | [add-connector.md](add-connector.md) |
| Finding what the earlier code built | [today.md](today.md) |
| `seal`, `measure`, `history`, `deliver`, `watch`, the experiment and the holdout | [../judge/SKILL.md](../judge/SKILL.md) |
| Cards, policy, roles, budgets, who may approve | [../organization/SKILL.md](../organization/SKILL.md) |
| How the chat and its cards look and read | [../design/behaviour.md](../design/SKILL.md) |
| Deciding whether a change belongs in the product | [../product/SKILL.md](../product/SKILL.md) |
