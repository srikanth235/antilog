# Session: objects and words

Sessions, their events and spend are in the store ([../change/ledger-table.md](../change/ledger-table.md)); the tools, the budget stop, the model proxy (`crates/antilog-proxy`: model gateway, spend booking and network allowlist), the organization's model keys, the runner and environments are built; the real code host is not. [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md) defines the words for screens; here an agent is a worker: a model run by a harness. What each screen shows is in [../design/behaviour.md](../design/behaviour.md), not here.

## Where a session sits

| Object | Means | Not |
| --- | --- | --- |
| project | A group of sessions for one area of the code, with members and roles per project (see [../organization/model.md](../organization/model.md)). It has an optional monthly cap and an optional default budget per session | a repo, a folder |
| session | One agent's work in a project: a chat with one agent in one machine, with any number of repos attached. Status: Working, Waiting for you, Done, Failed. A session a person stops is Done, "Stopped by NAME" | a thread, a task, a step machine |
| session event | One item in a session's conversation: a message, a tool call name, a card or a status change | a log line |

A session's fields (`antilog-types/src/session.rs`): `id`, `organizationId`, `projectId`, `title` (the first line of the prompt, up to 80 characters, when none is given), `createdBy` (the member who started it), `agent` (OpenCode; `auto` resolves to the organization's default agent), `model` (`provider/model`, from the request or else the server's `ANTILOG_MODEL`), `repos`, `environment` and `environmentVersion` (pinned at start), `budgetCapCents` (optional, the session's own cap), `followPolicy` (the composer's Mode), `status`, `note`, `createdAt`, `updatedAt`. Events and spend belong to the session.

## Routes

| Route | Does |
| --- | --- |
| `POST /projects/{id}/sessions` | Start a session: `{title?, agent?, model?, prompt, repos[], environment?, budgetCapCents?, followPolicy?}`. Editors and owners of the project. 201; 409 `budget_spent` when the project's or the organization's monthly cap is reached; 422 for a bad model or an empty prompt |
| `GET /projects/{id}/sessions` | The project's sessions, newest first; `?mine=true` keeps the caller's |
| `GET /sessions` | The organization's sessions across projects, newest first, each with `projectId` and `status`; `?mine=true` |
| `GET /sessions/{id}` | One session |
| `POST /sessions/{id}/messages` | A person's message to the agent |
| `POST /sessions/{id}/stop` | Stop it: Done, "Stopped by NAME" |
| `GET /sessions/{id}/events` | The conversation |
| `GET /sessions/{id}/spend` | `SessionSpend`: `budgetCapCents` or null, `spentCents`, tokens, `byKind` |
| `PUT /sessions/{id}/budget` | `{budgetCapCents}`: editors and owners of the project; 409 `over_cap` above the project's or the organization's cap |
| `PUT /sessions/{id}/environment` | `{environment}` (an id, `default` for the built-in): moves the session to its newest version, network at once, variables, default repos and setup script to a running machine now; editors and owners of the project; 422 for an unknown or archived one |
| `GET /sessions/{id}/approvals`, `GET /approvals/pending`, `POST /approvals/{id}/decision` | Cards of a session; the cards the caller may decide by their role in the card's project; a decision |

## Agents

| Object | Means | Not |
| --- | --- | --- |
| agent | A model run by a harness (OpenCode in v0). It runs in the session's machine and never holds a model key (the proxy holds the customer's keys and meters spend) | a login or key |
| model | `provider/model`. It decides the provider whose key the proxy uses and the harness's `small_model` | an agent |
| cost tier | "$" to "$$$" on each agent, set when the agent is added ([add-agent.md](add-agent.md)) | a quality ranking |
| Auto | `auto`: the organization's default agent, else the only agent turned on, and the built-in OpenCode agent while none is set up; it says why in one line. A person can always pick a named agent. No agent is preselected by brand | a favoured agent |
| environment | What a session runs on: default repos, a network level (none, limited with exact hosts, full), write-only network secrets, plain variables, a setup script and a size. An organization manages them in Settings, Connections; every change is a new version and a session keeps the version it started on. An agent proposes a change with `set_environment` and a person approves it: that is a new version of the environment and the session moves to it, live ([tools.md](tools.md)); a person can also move a session to another environment from its Overview | a server, a config file |
| attached repo | A repo the session works in, any number, cloned into the machine at start or after an approved Attach repo | a clone Antilog keeps |

## Spend

| Object | Means | Not |
| --- | --- | --- |
| session budget | The session's own optional cap, in US cents. It is at most the project's monthly cap, which is at most the organization's monthly cap (409 `over_cap` both ways). A new session's cap defaults to the project's default budget per session, then the organization's. An editor or owner can change it at any time; the agent asks with `request_budget`, which works only when the session has a cap | a bill |
| spend record | One append-only row of money or tokens, tied to a session and its agent. Kinds: tokens and machine. Model spend is booked by the proxy from the usage in each reply: whole cents, rounded up, priced from one table in code with the model as the key (`antilog-proxy/src/price.rs`) | an invoice |
| gateway check | Before forwarding, the proxy looks at the session's cap, then the project's monthly cap, then the organization's, and answers 402 in the provider's own error shape once one is spent (`budget::ensure_room`) | advice |

## Connectors, skill packs and plugins

What an agent can reach beyond its machine ([docs/V0.md](../../../docs/V0.md)). Every session gets all of the organization's connectors and the latest version of each skill; there is no per-session selection.

| Object | Means | Not |
| --- | --- | --- |
| connector | A remote MCP server, added in Customize, Connectors with a URL and an optional credential. The server calls it for a session, so the credential never reaches a machine. Each of its tools is On, Off or Ask first. Status: Connected, Reconnect, Not checked. The code host is not a connector. Steps in [add-connector.md](add-connector.md) | a data system, a cloud account |
| tool mode | On, Off or Ask first, per tool of a connector. Ask first raises a card and one approval pays for one call | a role |
| sealed credential | A connector's credential, encrypted with the master key and bound to its row. Opened on the server for one call; never returned by a route | a session secret |
| session endpoint | `/mcp/sessions/{id}`: the one MCP endpoint of a session, with a bearer token for `Purpose::Mcp`. Serves the setup tools and the connectors' tools | a connector |
| skill pack | A versioned `SKILL.md` folder of written text, immutable per version. Screen word: Skill, in Customize, Skills. Each session gets the latest version of every organization skill; OpenCode loads one on demand. Steps in [add-skill-pack.md](add-skill-pack.md) | a file in `.claude/skills/`, a plugin |
| plugin | An Antilog bundle (a manifest with skills and connectors). Adding one unpacks it into those parts, each governed as usual; it has no powers of its own. Removing one asks whether to keep each part | a connector, a harness plugin |

## What the server stores

The server keeps a ledger of a session. It runs no loop. A person can talk or interrupt at any time.

| Stores | Never stores |
| --- | --- |
| The conversation: what people and agents said; built as `session_events`, read by `GET /sessions/{id}/events` | File contents, diffs, whole tool output |
| Each finished tool call as a short label, its status (done or failed) and its input and output cut to 4 KB with credentials taken out (owner, 21 Nov), and card states | Data |
| Connector status | Data |
| Spend and audit rows | Raw logs |

Whole tool output and raw logs stay on the session machine and are fetched when opened. They may expire, and the session says so; a failed fetch shows as failed, never as empty.

The terminal, Changes, Preview and Files panels read from the session's machine when opened and the server keeps no copy. A person with edit access can type in the terminal, a viewer sees it read-only, and opening one is an audit row. How the panels look is in [../design/behaviour.md](../design/behaviour.md).

## Lifecycle

1. **Start.** One message in the composer with the chips (Project, Repos, Agent, Environment, Budget), each defaulted. Antilog starts the session in the project, queued for a runner ([runner.md](runner.md)).
2. **Work.** The agent drives and sets up through cards when needed.
3. **Idle or stopped.** The machine is released with an export; the conversation does not end. A person stopping a session makes it Done.
4. **Resume.** A message queues the session again. The next machine clones the repos and imports the export, so the agent has its history. Whatever was not pushed is gone if it lands on a new machine.

## How this area refers to the others

- Organization: by actor id, project id and role. Environments, attached repos and credential references belong to the organization, sessions to a project. Approvals and policy: [../organization/approvals.md](../organization/approvals.md).
