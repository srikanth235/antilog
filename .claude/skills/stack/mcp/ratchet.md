# MCP in Ratchet

`general/` is a copy of Anthropic's `mcp-builder` skill from [anthropics/skills](https://github.com/anthropics/skills) (Apache-2.0, [licence](licenses/anthropic-skills-mcp-builder-LICENSE)). Start at [general/INDEX.md](general/INDEX.md). The list is in [sync.py](sync.py).

**This page wins over `general/`.** The skill teaches how to build an MCP server in TypeScript or Python. Ratchet's is neither.

## Our server

(target) `ratchet mcp` is a mode of the `ratchet` CLI that serves the Ratchet tools on stdio, built with `rmcp` (version in [../libraries.md](../libraries.md)). The tools are `seal`, `measure`, `history`, `spawn`, `deliver`, `watch` and the setup tools `attach_repo`, `request_credential`, `set_environment` and `request_budget`. They are the same tools every session gets, so an agent a person already uses on their own machine (Claude Code, Codex) can propose a change that only the judge measures. It is Rust, on the API's own types. How a client signs in and how it is registered is in [../../ratchet/SKILL.md](../../ratchet/SKILL.md). The tools are described in [../../session/tools.md](../../session/tools.md) and [../../judge/tools.md](../../judge/tools.md), and a new or changed tool follows [../../change/tool.md](../../change/tool.md).

Read the skill for **design**, not code: how to name tools, write descriptions a model can choose between, shape inputs and outputs, paginate, return errors that say what to do next, and write an evaluation of realistic questions. Skip its TypeScript and Python server guides and its scripts.

## Rules

- **A tool can do nothing its caller's token cannot.** Each tool calls the API with the caller's access token and role. It never has a back door and never widens a scope. A denied call returns the API's error.
- **Holdout data is never reachable.** No tool reads, lists or summarizes the holdout, and no tool output may carry it. An agent never holds the holdout credential and never learns a holdout value; see [../../judge/rules.md](../../judge/rules.md).
- **One definition, served twice.** A tool has one definition, served to a session and by `ratchet mcp`. Name tools and describe them as the glossary does ("experiment" and "session", never "contract" or "automation"). Do not add a tool the API lacks. A tool that matters shows an approval card to a person; a tool whose command needs `--yes` takes `confirm: true` and shows its preview first, as [../../ratchet/SKILL.md](../../ratchet/SKILL.md) says. `measure` returns a ticket at once and its result arrives later as an event, so no tool timeout applies.
- **A tool does a whole job.** An agent calls it from its name and description alone, so a tool takes intent (`measure` this commit) and does the steps inside: no tool the agent must call in a set order, no ids it must look up first, no cleanup it must remember. See "Every language" in [../SKILL.md](../SKILL.md).
- **Secrets never appear in a tool result.** Not a token, a connector secret or a platform credential.
- **Evaluate it.** Write the skill's evaluation questions against a test server and keep them with the CLI's tests ([../../land/tests.md](../../land/tests.md)).

## Connecting to other MCP servers

That is a different thing: an MCP connector lets an agent's code read an outside system, and it is built in the session area. See [../../session/add-connector.md](../../session/add-connector.md) and [../../session/SKILL.md](../../session/SKILL.md). Ratchet is the client there, and the credential rules of that page apply.

## Updating general/

```bash
git clone --depth 1 https://github.com/anthropics/skills /tmp/anthropic-skills
python3 .claude/skills/stack/mcp/sync.py /tmp/anthropic-skills
python3 .claude/skills/stack/mcp/sync.py --check
```

Never edit `general/` by hand.
