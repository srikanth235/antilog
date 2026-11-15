# MCP in Ratchet

`general/` is a copy of Anthropic's `mcp-builder` skill from [anthropics/skills](https://github.com/anthropics/skills) (Apache-2.0, [licence](licenses/anthropic-skills-mcp-builder-LICENSE)). Start at [general/INDEX.md](general/INDEX.md). The list is in [sync.py](sync.py).

**This page wins over `general/`.** The skill teaches how to build an MCP server in TypeScript or Python. Ratchet's is neither.

## Our server

(target) `ratchet mcp` is a mode of the `ratchet` CLI that serves its commands as MCP tools on stdio, built with `rmcp` (version in [../libraries.md](../libraries.md)). It is Rust, on the API's own types. How a client signs in and how it is registered is in [../../ratchet/SKILL.md](../../ratchet/SKILL.md). The step in [../../change/api-endpoint.md](../../change/api-endpoint.md) adds a tool with each endpoint.

Read the skill for **design**, not code: how to name tools, write descriptions a model can choose between, shape inputs and outputs, paginate, return errors that say what to do next, and write an evaluation of realistic questions. Skip its TypeScript and Python server guides and its scripts.

## Rules

- **A tool can do nothing its caller's token cannot.** Each tool calls the API with the caller's access token and role. It never has a back door and never widens a scope. A denied call returns the API's error.
- **Holdout data is never reachable.** No tool reads, lists or summarizes the holdout, and no tool output may carry it. The holdout marking is in [../../context/rules.md](../../context/rules.md).
- **Tools mirror the CLI and the API.** One tool per command, named as the command and the glossary name them ("experiment", never "automation"). Do not add a tool the API lacks. A tool whose command needs `--yes` takes `confirm: true` and shows its preview first, as [../../ratchet/SKILL.md](../../ratchet/SKILL.md) says.
- **Secrets never appear in a tool result.** Not a token, a connector secret or a model key.
- **Evaluate it.** Write the skill's evaluation questions against a test server and keep them with the CLI's contract tests ([../../land/tests.md](../../land/tests.md)).

## Connecting to other MCP servers

That is a different thing: an MCP connector lets an agent's code read an outside system, and it is built in the context layer. See [../../context/add-connector.md](../../context/add-connector.md) and [../../context/SKILL.md](../../context/SKILL.md). Ratchet is the client there, and the credential rules of that page apply.

## Updating general/

```bash
git clone --depth 1 https://github.com/anthropics/skills /tmp/anthropic-skills
python3 .claude/skills/stack/mcp/sync.py /tmp/anthropic-skills
python3 .claude/skills/stack/mcp/sync.py --check
```

Never edit `general/` by hand.
