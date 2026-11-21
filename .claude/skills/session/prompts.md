# What agents are told

What agents are told is part of the product, not an implementation detail. It decides whether an agent asks, stops and reports honestly.

## Where it lives

In v0 an agent is told three things, all in code: the person's first message (the session's prompt, unchanged), the MCP endpoint's instructions (`antilog-server/src/api/mcp.rs`: "Antilog sets this session up: attach a repo, ask for a credential, change the environment, ask for more budget. A person approves each request."), and the setup tools' descriptions (`antilog-core/src/tools/mod.rs`, `definitions()`). Skill packs sit in OpenCode's skills folder and are loaded on demand. There is no separate session brief: a change to any of these is a change to the product and is reviewed like one.

## What must be covered

- **The tool descriptions.** One short, exact description per tool: `attach_repo`, `request_credential`, `set_environment`, `request_budget`. Each says what the tool does, whether it asks a person, and what it refuses.
- **When to ask.** For setup, ask through a card, not in prose. For a product call or a choice the task does not settle, ask the person.
- **Credentials.** Never ask for a secret in chat; call `request_credential`.
- **Spend.** The session may have a cap of its own; `request_budget` raises it, and says so when the session has none.
- **Skill packs.** OpenCode lists the packs by name and description and loads one when it fits. They are advice. The packs are not repeated in a prompt.
- **Connector tools.** A connector's tools appear as `<connector>__<tool>`. A call may come back as waiting for a person (Ask first): the agent calls again with the same input once it is approved, and each approval pays for one call.

## Principles

- Short and plain. A long prompt is read less.
- Say what is true of the product, not what is hoped of the model.
- The same text for every agent, so agents compare fairly.
- No vendor words.
- A rule that must never bend lives in code, not in a prompt. The prompt explains it.

## Proving a change

The tool definitions and the endpoint's instructions are covered by the tool tests (`antilog-core`, `antilog-server/tests`). Follow [../land/SKILL.md](../land/SKILL.md).
