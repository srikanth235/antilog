---
name: ratchet
description: Drive Ratchet from the shell with the `ratchet` command (target; not built on this branch). Use when the user asks about their experiments (list, show, history, receipt, spend), wants to start an experiment from the terminal (`ratchet "cut checkout p95"`), sign in to a Ratchet server, open a pull request for a win, or wants Ratchet's tools as an MCP server (`ratchet mcp`) for an agent on their own machine, or names Ratchet.
---

# Ratchet

`ratchet` is a command-line client of a Ratchet server. It is a client of the same experiments as the web app: `ratchet "cut checkout p95"` starts an experiment, with its lead session, in the terminal. `ratchet mcp` serves the Ratchet tools to an agent the person already uses on their own machine. It is (target): it does not exist on this branch, so the commands below are the design and may change as the program is built. The earlier daemon-based CLI is in [today.md](today.md).

There is no daemon, no LaunchAgent and no simulation mode. The server is Ratchet cloud or a self-hosted server at a URL. The client holds no data and runs no sandbox: it asks the server, and the server starts sessions on agent platforms, which do the work. Only the judge's numbers count, whichever client proposed the change.

## Sign in

- `ratchet sign-in <server-url>` opens browser sign-in (OpenID Connect or GitHub) and stores a session for that server. With no URL it uses Ratchet cloud.
- For CI, scripts and `ratchet mcp`, use an access token: `RATCHET_TOKEN` with `RATCHET_SERVER`, or `ratchet sign-in --token`. A token is shown once when a member creates it under Settings, Access tokens, and acts with that member's role.
- Never print or log a token. Never write one into a file in the repo.
- A viewer can read experiments and approves nothing; an editor can start experiments, approve cards and change an experiment's budget; an owner can also manage members and settings. A refused command says which role it needs.

## Rules

- Always add `--json` and read the JSON. Exit 0 is ok, 1 is a failure (read `error.message` and `error.hint`), 2 is a usage error or a command that needs `--yes`.
- Ids are in `ratchet experiments --json`. A wrong id exits 1 with a hint.
- A command that spends money or changes things (a new experiment, `budget`, `stop`, `deliver`) previews without `--yes` and exits 2. Show that preview to the user and ask before you run it again with `--yes`. An approval card in a terminal session is answered by the person, never by you on their behalf. A session with no budget is refused.
- Say "experiment" everywhere, flags, JSON keys (`experiment_id`) and output included; a session is one agent's work inside an experiment. Never "contract", "run" or "evaluator". See `docs/GLOSSARY.md`. Never "automation".

## Commands you will use most

| Command | Does |
| --- | --- |
| `ratchet "cut checkout p95"` | Start an experiment in the terminal. Add `--repo owner/name`, `--agent`, `--environment` and `--budget` for the chips the web composer has |
| `ratchet experiments --json` | Experiments with id, state (Not sealed, Sealed, Watching, Regressed), activity (Working, Waiting for you, Done, Failed), metric, baseline to best, spend of budget |
| `ratchet show <experiment> --json` | One experiment: state, activity, its sessions, trials, kept, spend of budget |
| `ratchet history <experiment> --json` | Every trial for an experiment across its sessions: hypothesis, outcome, values, commit ids, what it started from |
| `ratchet receipt <win> --json` | The signed receipt for a win and its lineage |
| `ratchet spend <experiment> --json` | Spend against the experiment's budget |
| `ratchet budget <experiment> <amount> --yes` | Change the experiment's budget (editor or owner) |
| `ratchet stop <session> --yes` | Stop a session. It is Done, "Stopped by NAME". Wins so far are kept |
| `ratchet deliver <experiment> --yes` | Open the pull requests with the receipt, by the same card policy as the web app |
| `ratchet sign-in <server-url>` | Sign in |
| `ratchet mcp` | Serve the Ratchet tools on stdio |

Anything not listed is whatever `ratchet --help` shows; it is the source once the program exists.

## The Ratchet tools (target)

Every session gets the Ratchet tools, and `ratchet mcp` serves the same ones. They are not shell commands a person runs.

| Tool | Does |
| --- | --- |
| `seal` | Proposes the experiment's definition. The judge runs the seal checks. A person seals it from a card |
| `measure` | Sends a trial (commits, the trial it started from, a one-line hypothesis). Returns a ticket at once; the measurement arrives later as an event |
| `history` | Every trial for an experiment, failures included |
| `spawn` | Starts another session in the same experiment, on its budget |
| `deliver` | Opens one pull request per repo changed, each with the receipt. Refuses a trial that is not Kept |
| `watch` | Has the judge measure the experiment again on a schedule or when the default branch moves |
| `attach_repo`, `request_credential`, `set_environment`, `request_budget` | The setup tools. Each shows a card in the chat. A credential value is typed into the form, never into the chat |

A trial is measured by the judge in a judge sandbox with no agent in it. The result has the measured value and guardrail values on the evaluation data and the outcome (Kept, Within noise, Worse, Guardrail failed, Failed holdout, Rejected, Failed). It never has the holdout value. Numbers an agent makes in its own session never count. See [judge/tools.md](../judge/tools.md) and [session/tools.md](../session/tools.md).

## MCP

`ratchet mcp` serves the Ratchet tools above on stdio, for an agent on the person's own machine (Claude Code, Codex). The agent can propose a change anywhere; only the judge measures it. It signs in with an access token. Register it with `claude mcp add ratchet -- ratchet mcp`. A tool that matters shows its approval card to the person, in the terminal or in the web app.

## Decided

- There is no `ratchet chat` and no separate drafting step. The terminal session is the chat, and an agent proposes the definition in it with `seal`.
- Approvals are answered by a person. A viewer cannot approve, and Add credential can never be pre-approved.
