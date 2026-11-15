---
name: ratchet
description: Drive Ratchet from the shell with the `ratchet` command (target; not built on this branch). Use when the user asks about their experiments (list, show, status, spend, wins, report), wants to sign in to a Ratchet server, start or stop one, open a pull request for a win, or wants Ratchet as an MCP server (`ratchet mcp`), or names Ratchet.
---

# Ratchet

`ratchet` is a command-line client of a Ratchet server, and `ratchet mcp` is the same client as an MCP server. It is (target): it does not exist on this branch, so the commands below are the design and may change as the program is built. The earlier daemon-based CLI is in [today.md](today.md).

There is no daemon, no LaunchAgent and no simulation mode. The server is Ratchet cloud or a self-hosted server at a URL. The client holds no data and runs no sandbox: it asks the server, and the server's runner does the work in the customer's infrastructure.

## Sign in

- `ratchet login <server-url>` opens browser sign-in (OpenID Connect) and stores a session for that server. With no URL it uses Ratchet cloud.
- For CI, scripts and MCP, use an access token: `RATCHET_TOKEN` with `RATCHET_SERVER`, or `ratchet login --token`. A token is shown once when a member creates it under Settings, Access tokens, and acts with that member's role.
- Never print or log a token. Never write one into a file in the repo.
- A viewer can read and not change; an editor can start and stop; an owner can also manage members and settings. A refused command says which role it needs.

## Rules

- Always add `--json` and read the JSON. Exit 0 is ok, 1 is a failure (read `error.message` and `error.hint`), 2 is a usage error or a command that needs `--yes`.
- Ids are in `ratchet list --json`. A wrong id exits 1 with a hint.
- `start`, `stop` and `pr` spend money or change things. Without `--yes` they only print what they would do and exit 2. Show that preview to the user and ask before you run it again with `--yes`. A start with no budget is refused.
- Say "experiment" for the object everywhere, flags, JSON keys and output included. "Optimization" names only the basic. See `docs/GLOSSARY.md`. Never "automation".

## Commands you will use most

| Command | Does |
| --- | --- |
| `ratchet status --json` | The server you are signed in to and your experiments at a glance |
| `ratchet list --json` | All experiments with ids, state, best result, spend |
| `ratchet show <id> --json` | One experiment: agents, baseline vs best, receipt |
| `ratchet spend <id> --json` | Spend against the budget |
| `ratchet wins <id> --json` | Verified wins and their receipts |
| `ratchet report <id>` | The markdown report |
| `ratchet start <id> --budget 50 --yes` | Start an approved experiment (preview without `--yes`) |
| `ratchet stop <id> --yes` | Stop one. Wins found so far are kept |
| `ratchet pr <id> --yes` | Open the pull request with the verified wins |

Anything not listed is whatever `ratchet --help` shows; it is the source once the program exists.

## MCP

`ratchet mcp` serves the commands above as MCP tools on stdio, for clients with no shell. It signs in the same way, with an access token. Register it with `claude mcp add ratchet -- ratchet mcp`. A tool that needs `--yes` takes `confirm: true`, and shows its preview first.

## Decided

- There is no `ratchet chat`. Drafting an experiment with the assistant happens in the web and Mac apps; another agent drives Ratchet through `ratchet mcp`.
- `start` only starts an experiment that exists as an approved draft, so Approve stays a deliberate step.
