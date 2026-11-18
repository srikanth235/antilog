---
name: antilog
description: Drive Antilog from the shell with the `antilog` command (target; only `antilog mcp` is built on this branch). Use when the user asks about their experiments (list, show, history, receipt, spend), wants to start an experiment from the terminal (`antilog "cut checkout p95"`), sign in to an Antilog server, open a pull request for a win, or wants Antilog's tools as an MCP server (`antilog mcp`) for an agent on their own machine, or names Antilog.
---

# Antilog

`antilog` is a command-line client of an Antilog server, for the same experiments as the web app: `antilog "cut checkout p95"` starts an experiment, with its lead session, in the terminal. `antilog mcp` serves the Antilog tools to an agent the person already uses on their own machine. It is (target): the commands below are the design and may change as the program is built.

The server is Antilog cloud or a self-hosted server at a URL. The client holds no data and runs no sandbox: it asks the server, which starts sessions on agent platforms. Only the judge's numbers count, whichever client proposed the change.

## Sign in

- `antilog sign-in <server-url>` opens browser sign-in (OpenID Connect or GitHub) and stores a session for that server. With no URL it uses Antilog cloud.
- For CI, scripts and `antilog mcp`, use an access token: `ANTILOG_TOKEN` with `ANTILOG_SERVER`, or `antilog sign-in --token`. A token is shown once when a member creates it under Settings, Access tokens, and acts with that member's role.
- Never print or log a token. Never write one into a file in the repo.
- Roles are the person's roles in the project the command touches (the organization role unless the project set another; see [../organization/SKILL.md](../organization/SKILL.md)). A refused command says which role it needs.

## Rules

- Always add `--json` and read the JSON. Exit 0 is ok, 1 is a failure (read `error.message` and `error.hint`), 2 is a usage error or a command that needs `--yes`.
- Ids are in `antilog experiments --json`. A wrong id exits 1 with a hint.
- A command that spends money or changes things (a new experiment, `budget`, `stop`, `deliver`) previews without `--yes` and exits 2. Show that preview to the user and ask before you run it again with `--yes`. An approval card in a terminal session is answered by the person, never by you on their behalf. A session with no budget is refused.
- Say "experiment" everywhere, flags, JSON keys (`experiment_id`) and output included; a session is one agent's work inside an experiment. Retired words are listed in `docs/GLOSSARY.md`.

## Commands you will use most

| Command | Does |
| --- | --- |
| `antilog "cut checkout p95"` | Start an experiment in the terminal. Add `--project`, `--repo owner/name`, `--agent`, `--environment` and `--budget` for the chips the web composer has. With no `--project` it uses the person's default project |
| `antilog projects --json` | Projects with id, the person's role, experiments, wins and spend this month |
| `antilog experiments --json` | Experiments (add `--project` to narrow) with id, project, state (Not sealed, Sealed, Watching, Regressed), activity (Working, Waiting for you, Done, Failed), metric, baseline to best, spend of budget |
| `antilog show <experiment> --json` | One experiment: state, activity, its sessions, trials, kept, spend of budget |
| `antilog history <experiment> --json` | Every trial for an experiment across its sessions: hypothesis, outcome, values, commit ids, what it started from |
| `antilog receipt <win> --json` | The signed receipt for a win and its lineage |
| `antilog spend <experiment> --json` | Spend against the experiment's budget, cost per trial and cost per kept trial, and where the spend went (Kept, Within noise, Worse, Guardrail failed, Rejected, Failed, Not measured) |
| `antilog budget <experiment> <amount> --yes` | Change the experiment's budget (editor or owner) |
| `antilog stop <session> --yes` | Stop a session. It is Done, "Stopped by NAME". Wins so far are kept |
| `antilog deliver <experiment> --yes` | Open the pull requests with the receipt, by the same card policy as the web app |
| `antilog sign-in <server-url>` | Sign in |
| `antilog mcp` | Serve the Antilog tools on stdio |

Anything not listed is whatever `antilog --help` shows; it is the source once the program exists.

## The Antilog tools (target)

Every session gets the Antilog tools, and `antilog mcp` serves the same ones. They are not shell commands a person runs.

| Tool | Does | Owner page |
| --- | --- | --- |
| `seal`, `measure`, `history`, `deliver`, `watch` | Propose and seal the definition, measure a trial (quick check, then full measurement), read every trial, open the pull requests, measure again later | [../judge/tools.md](../judge/tools.md) |
| `spawn`, `attach_repo`, `request_credential`, `set_environment`, `request_budget` | Start another session, and set up in the chat with a card. A credential value is typed into the form, never the chat | [../session/tools.md](../session/tools.md) |

The judge measures every trial in a judge sandbox with no agent in it, and the result never has a holdout value. Numbers an agent makes in its own session never count.

## MCP

`antilog mcp` serves the Antilog tools on stdio, for an agent on the person's own machine (Claude Code, Codex). The agent can propose a change anywhere; only the judge measures it. It works inside a session in the experiment, with no platform sandbox, and pushes trial commits to that session's branch with a short-lived git token minted for the session. It ships in the first release. It signs in with an access token and is registered with `claude mcp add antilog -- antilog mcp`. A tool that matters shows its approval card to the person, in the terminal or the web app.

## Decided

- The terminal session is the chat: an agent proposes the definition in it with `seal`. There is no separate drafting step.
- Approvals are answered by a person. A viewer cannot approve, and Add credential can never be pre-approved.
