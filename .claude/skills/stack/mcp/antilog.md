# MCP in Antilog

`general/` is a copy of Anthropic's `mcp-builder` skill from [anthropics/skills](https://github.com/anthropics/skills) (Apache-2.0, [licence](licenses/anthropic-skills-mcp-builder-LICENSE)). Start at [general/INDEX.md](general/INDEX.md). The list is in [sync.py](sync.py).

**This page wins over `general/`.** The skill teaches how to build an MCP server in TypeScript or Python. Antilog's is neither.

## Our server

The session's MCP endpoint (`/mcp/sessions/{id}`, built: `crates/antilog-server/src/api/mcp.rs`, with `rmcp`'s streamable HTTP server feature; version in [../libraries.md](../libraries.md)) serves the setup tools `attach_repo`, `request_credential`, `set_environment` and `request_budget` and the tools of every organization connector. The tools are defined in `crates/antilog-core/src/tools/`. There is no `antilog mcp` command and no REST tool endpoint. The tools are described in [../../session/tools.md](../../session/tools.md), and a new or changed tool follows [../../change/tool.md](../../change/tool.md).

Read the skill for **design**, not code: how to name tools, write descriptions a model can choose between, shape inputs and outputs, paginate, return errors that say what to do next, and write an evaluation of realistic questions. Skip its TypeScript and Python server guides and its scripts.

## Rules

- **A tool can do nothing its caller's token cannot.** Each tool acts with the session's `Purpose::Mcp` token and the role of the member who started the session. It never has a back door and never widens a scope. A denied call returns the error.
- **A tool does a whole job.** An agent calls it from its name and description alone, so a tool takes intent and does the steps inside: no tool the agent must call in a set order, no ids it must look up first, no cleanup it must remember. See "Every language" in [../SKILL.md](../SKILL.md).
- **Every error says how to recover.** A refusal names the next action: "this session has no budget of its own, only the organization's monthly cap, which an owner changes in Settings". An error an agent cannot act on makes it guess.
- **Output is bounded.** Every list is capped and paginated with a cursor, and long text is truncated with a note on how to get more.
- **Secrets never appear in a tool result.** Not a token, a connector secret or a model key.
- **Names follow the glossary** ("session", "project"). A tool that matters shows an approval card to a person.
- **Evaluate it.** Write the skill's evaluation questions against a test server and keep them with the tool tests ([../../land/tests.md](../../land/tests.md)).

## Connecting to other MCP servers

That is a different thing: a connector is a remote MCP server the session endpoint calls for an agent, and it is built in the session area (`antilog-server/src/connectors.rs`). See [../../session/add-connector.md](../../session/add-connector.md) and [../../session/SKILL.md](../../session/SKILL.md). Antilog is the client there, and the credential rules of that page apply.

## Updating general/

```bash
git clone --depth 1 https://github.com/anthropics/skills /tmp/anthropic-skills
python3 .claude/skills/stack/mcp/sync.py /tmp/anthropic-skills
python3 .claude/skills/stack/mcp/sync.py --check
```

Never edit `general/` by hand.
