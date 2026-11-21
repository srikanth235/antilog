---
name: antilog
description: Drive Antilog from the shell. The `antilog` command is `antilog runner` only; everything else is the server's HTTP API, called with curl and an access token. Use when the user wants to start a session, list sessions or projects, message or stop a session, check a session's spend, decide an approval card, run a runner on a host, or names Antilog.
---

# Antilog

The `antilog` program is the runner: `antilog runner` polls a server's queue and runs its sessions on this machine. It is for the machine image and for hosts we keep, not for people ([../session/runner.md](../session/runner.md)). There is no `antilog mcp`, no `antilog "goal"` and no other subcommand. A person or an agent drives Antilog over HTTP with an access token. The client holds no data and runs no machine: the server queues sessions and a runner starts them ([docs/V0.md](../../../docs/V0.md)).

## Sign in

- An access token is shown once when a member creates it (`POST /tokens` `{name}`, or Settings, Access tokens) and acts with that member's role. Keep it in `ANTILOG_TOKEN`, and the server's address in `ANTILOG_SERVER`.
- Send it as `Authorization: Bearer $ANTILOG_TOKEN`.
- Never print or log a token. Never write one into a file in the repo.
- Roles are the person's roles in the project the call touches (the organization role unless the project set another; see [../organization/SKILL.md](../organization/SKILL.md)). A refused call says which role it needs.

## Rules

- Ids come from the list calls. A wrong id answers 404.
- Calls that spend money or change things (start a session, change a budget, stop a session, decide a card) are the person's to authorise. Show what a call would do and ask before you run it. An approval card is decided by the person, never by you on their behalf.
- Say "session" and "project", as the API does. Words are in `docs/GLOSSARY.md`.

## Calls you will use most

```bash
H="Authorization: Bearer $ANTILOG_TOKEN"; S="$ANTILOG_SERVER"

# a project (organization owners)
curl -s -H "$H" -H 'content-type: application/json' -d '{"name":"Checkout","description":"The checkout service"}' $S/projects

# a session in it (editors and owners of the project)
curl -s -H "$H" -H 'content-type: application/json' \
  -d '{"prompt":"Cut the test suite time","repos":["acme/checkout"],"budgetCapCents":1000}' $S/projects/$PROJECT/sessions
```

| Call | Does |
| --- | --- |
| `GET /projects` | Projects with the caller's role |
| `POST /projects/{id}/sessions` | Start a session: `{title?, agent?, model?, prompt, repos[], environment?, budgetCapCents?, followPolicy?}`. 201; 409 `budget_spent` when the project's or the organization's monthly cap is reached; 422 for a bad model or an empty prompt |
| `GET /projects/{id}/sessions`, `GET /sessions` | A project's sessions, or the organization's across projects, newest first, each with `projectId` and `status`; add `?mine=true` for the caller's |
| `GET /sessions/{id}`, `GET /sessions/{id}/events` | One session; its conversation |
| `POST /sessions/{id}/messages` | Send the agent a message: `{text}` |
| `POST /sessions/{id}/stop` | Stop it. It is Done, "Stopped by NAME" |
| `GET /sessions/{id}/spend` | `budgetCapCents` or null, `spentCents`, tokens, `byKind` |
| `PUT /sessions/{id}/budget` | `{budgetCapCents}`: change the session's cap (409 `over_cap` above the project's or the organization's) |
| `GET /approvals/pending`, `POST /approvals/{id}/decision` | The cards the caller may decide; a decision |
| `GET /budgets` | The organization's cap and spend, and each project's |
| `PUT /projects/{id}/limits`, `PUT /organization/limits` | `{monthlyCapCents, defaultBudgetCents}` for a project (project owners); the organization's limits (owners) |
| `GET /audit` | The audit log (owners) |

The OpenAPI document is `openapi/antilog.json`; it is generated, never edited by hand ([../change/api-endpoint.md](../change/api-endpoint.md)). Errors are `{code, message, fields}`.

## `antilog runner`

`antilog runner --server URL --proxy HOST:PORT` with the runner key in `ANTILOG_RUNNER_KEY` polls the server's queue and runs each session in a Docker container. Flags include `--image`, `--docker`, `--opencode`, `--exit-after-last-session`, `--drain-grace-sec`, `--setup-timeout-sec`, `--disk-gb`, `--sessions`, `--idle-timeout-sec`, `--poll-secs`, `--pure` and `--disable-project-config`; `antilog runner --help` is the source. What it does is in [../session/runner.md](../session/runner.md).

## The tools an agent gets

A session's agent reaches Antilog only through its MCP endpoint (`/mcp/sessions/{id}`): the setup tools `attach_repo`, `request_credential`, `set_environment` and `request_budget`, and the tools of the organization's connectors. They are not shell commands ([../session/tools.md](../session/tools.md)).
