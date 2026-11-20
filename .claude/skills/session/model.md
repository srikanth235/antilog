# Session: objects and words

> v0 replaces the agent platforms, the stepping relay and the judge sandbox: see [docs/V0.md](../../../docs/V0.md). Lines that still name them are being removed.

Experiment rows, sessions and spend records are in the store ([../change/ledger-table.md](../change/ledger-table.md)); the tools, the budget stop, the model proxy (`crates/antilog-proxy`: model gateway, spend booking and egress allowlist) and the organization's model keys are built; the rest is target. [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md) defines the words for screens; here an agent is a worker: a model run by a harness. What each screen shows is in [../design/behaviour.md](../design/behaviour.md), not here.

## Where a session sits

| Object | Means | Not |
| --- | --- | --- |
| project | A group of experiments for one area of the code, with members and roles per project (see [../organization/model.md](../organization/model.md)) | a repo, a folder |
| experiment | The sealed goal and everything measured against it; it owns the budget and holds one or more sessions (see [../judge/model.md](../judge/model.md)). Its activity comes from its sessions: Working, Waiting for you, Done, Failed | a session |
| session | One agent's work inside an experiment: a chat with one agent in one machine, with any number of repos attached. Status: Working, Waiting for you, Done, Failed. A session a person stops is Done, "Stopped by NAME". Named on screen by its agent ("Claude Code session"); never a top-level noun | a thread, a task, a step machine |
| session event | One item in a session's stream: a message, tool call, card, measurement arriving or status change | a log line |

## Agents

| Object | Means | Not |
| --- | --- | --- |
| agent | A model run by a harness (OpenCode in v0). It holds a short-lived git token for its own branch per repo and read-only connector credentials that leave out the holdout, never a model key | the platform, a login or key |
| cost tier | "$" to "$$$" on each agent, with the measured figure ("about $2.40 per trial here"), priced from the proxy's token counts ([add-agent.md](add-agent.md)) | a quality ranking |
| agent scoreboard | Per agent, filtered by evaluation kind and project: experiments, trials, kept, wins delivered, cost per win, median time to a win, Delivered value. Only from the judge's measurements here: "Measured by the judge on your code. Not a vendor benchmark." | a vendor benchmark |
| Auto | The agent choice that picks the best cost per win for this evaluation kind in this project among the agents the organization turned on, and says why in one line. A person can always pick a named agent. With no data it uses the default agent an owner sets in Customize, Agents. No agent is preselected by brand | a favoured agent |
| hand off | Moving a session to another agent: the new session starts in the same experiment on the same branch with a summary and the history; the old one ends Done, "Handed off to Codex" | a spawn, a new experiment |
| failover | An agent that is unavailable or rate-limited leaves the session Waiting for you with a Continue on another agent? card naming the agent Auto would choose. Policy may allow it without asking; a timeline line and an audit row record it | a retry on the same platform |
| environment | What a session runs on: network level (none, limited with exact hosts, full), plain variables, a setup script and a size. An organization manages them in Settings, Connections; every change is a new version and a session keeps the version it started on. An agent proposes a change with `set_environment` (target) | a server, a config file |
| attached repo | A repo on the code host the session works in, any number. Short-lived git token per repo, push only to the session's own branch | a clone Antilog keeps |

## Spend

| Object | Means | Not |
| --- | --- | --- |
| budget | One per experiment, covering all its sessions and their judge time. It sits inside the project's optional monthly cap, inside the organization's (see [../organization/approvals.md](../organization/approvals.md)). Set when starting (default: the project's, else the organization's); an editor or owner can change it at any time; lowering it below the spend stops new measurements. The agent asks with `request_budget` | a bill, a per-session allowance |
| spend record | One append-only row of money or tokens, tied to a session and its agent. Model spend is booked by the proxy from the usage in each reply: whole cents, rounded up, priced from one table in code with the model as the key (`antilog-proxy/src/price.rs`) | an invoice |
| cost per trial | Defined in [../judge/model.md](../judge/model.md); derived from spend records | |
| where the spend went | The experiment's spend split by Kept, Within noise, Worse, Guardrail failed, Rejected, Failed and Not measured (agent work that never reached `measure`), as dollars and a share. Derived | a bill |
| compaction | Summarising a session's context and continuing. Antilog does it on its own at 80 percent and says so in a timeline line; a person can also compact, or start a fresh session with a summary | forgetting |

Spending well is the sum of these, each owned once: the estimate on the Seal card and patience are the judge's ([../judge/tools.md](../judge/tools.md)); `history` first and the lower-cost default for `spawn` are in [tools.md](tools.md) and [prompts.md](prompts.md).

## Connectors, skill packs and plugins

What an agent can reach beyond its machine ([docs/V0.md](../../../docs/V0.md)).

| Object | Means | Not |
| --- | --- | --- |
| connector | A remote MCP server, added in Customize, Connectors with a URL and an optional credential. The server calls it for a session, so the credential never reaches a machine. Each of its tools is On, Off or Ask first. Status: Connected, Reconnect, Not checked. The code host is not a connector. Steps in [add-connector.md](add-connector.md) | a data system, a cloud account |
| tool mode | On, Off or Ask first, per tool of a connector. Ask first raises a card and one approval pays for one call | a role |
| sealed credential | A connector's credential, encrypted with the master key and bound to its row. Opened on the server for one call; never returned by a route | a session secret |
| session endpoint | `/mcp/sessions/{id}`: the one MCP endpoint of a session, with a bearer token for `Purpose::Mcp`. Serves the Antilog tools and the switched-on connectors' tools | a connector |
| skill pack | A versioned `SKILL.md` folder of written text, immutable per version. Screen word: Skill, in Customize, Skills. An experiment selects packs, each pinned to a version; OpenCode loads one on demand; the receipt lists the versions. Agents only: the judge never reads one. Steps in [add-skill-pack.md](add-skill-pack.md) | a file in `.claude/skills/`, a plugin |
| plugin | An Antilog bundle (a manifest with skills, connectors and evaluation templates). Adding one unpacks it into those parts, each governed as usual; it has no powers of its own. Removing one asks whether to keep each part, and a part in use is never removed | a connector, a harness plugin |

## Datasets and the holdout

The judge owns these words ([../judge/model.md](../judge/model.md)). The earlier design hung them under data connectors with an agent credential and a judge credential. That design is not built, and in v0 a connector is a remote MCP server that holds no datasets; where evaluation data lives and how the holdout is kept from agents is for the judge pages to settle.

| Object | Means | Not |
| --- | --- | --- |
| dataset | A named table or file set an experiment reads. Antilog records its id, versions, size and marking, never rows | a connector, a query |
| dataset version | One state of a dataset, with an id and a date; it never changes. A virtual dataset points at data left in place, pinned by an as-of time in the version id | latest, an editable snapshot |
| marking | A label that limits who may read a dataset. Holdout is the only one | a search tag |
| holdout | A marking: evaluation data only the judge can read. No control grants it to an agent (the control is absent, not disabled). Shown as "Restricted to the judge" | a split an agent can see |


## What the server stores

The server keeps a ledger of a session. It runs no loop. A person can talk or interrupt at any time.

| Stores | Never stores |
| --- | --- |
| The conversation: what people and agents said ([../product/rulings.md](../product/rulings.md)); built as `session_events`, read by `GET /sessions/{id}/events` | Tool output |
| Tool call names (built, in the conversation) and card states | File contents, diffs |
| Commit ids, skill pack versions, connector status | Data |
| Measurements | Raw logs |
| Spend and audit rows | |

Tool output and raw logs stay on the session machine and are fetched when opened. They may expire, and the session says so; a failed fetch shows as failed, never as empty.

The terminal, Changes, Preview and Files panels read from the agent's own sandbox when opened and the server keeps no copy. A person with edit access can type in the terminal, a viewer sees it read-only, and opening one is an audit row ("Opened a terminal in Claude Code session"). It is never an Antilog host or the judge sandbox. How the panels look is in [../design/behaviour.md](../design/behaviour.md).

## Lifecycle

1. **Start.** One message in the composer with the chips (Project, Repos, Agent, Environment, Budget), each defaulted. Antilog starts a new experiment in the project and its lead session, queued for a runner ([runner.md](runner.md)).
2. **Work.** The agent drives and sets up through cards when needed. Until a person seals it the experiment is Not sealed: the agent explores, measures the baseline and proposes a definition with `seal`. It may stay Not sealed and end there.
3. **Idle or stopped.** The machine is released with an export; the conversation does not end. A person stopping a session makes it Done.
4. **Resume.** A message queues the session again. The next machine clones the repos and imports the export, so the agent has its history. Whatever was not pushed is gone if it lands on a new machine.

Git is the hand-off: a trial is a set of commits, one per repo changed. Antilog stores commit ids, never diffs; another session or the judge reads the code from git.

## Two offerings

- **Antilog cloud (phase 1):** we run the server on our AWS account and every session on Antilog's own machines; spend is rebilled through the budget.

One server, one chart. A promise that holds in one and not the other is a defect.

## How this area refers to the others

- Judge: by experiment id and trial id. Value models, evaluation templates and the accept rule are the judge's; the budget meter makes the judge refuse `measure` when spend reaches the budget.
- Organization: by actor id, project id and role. Environments, attached repos and credential references belong to the organization, experiments to a project. Approvals and policy: [../organization/approvals.md](../organization/approvals.md).
