# Session: objects and words

All of this is target. [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md) defines the words for screens; here an agent is a worker: a model on an agent platform. What each screen shows is in [../design/behaviour.md](../design/behaviour.md), not here.

## Where a session sits

| Object | Means | Not |
| --- | --- | --- |
| project | A group of experiments for one area of the code, with members and roles per project (see [../organization/model.md](../organization/model.md)) | a repo, a folder |
| experiment | The sealed goal and everything measured against it; it owns the budget and holds one or more sessions (see [../judge/model.md](../judge/model.md)). Its activity comes from its sessions: Working, Waiting for you, Done, Failed | a session |
| session | One agent's work inside an experiment: a chat with one agent in one sandbox on one agent platform, with any number of repos attached. Status: Working, Waiting for you, Done, Failed. A session a person stops is Done, "Stopped by NAME". Named on screen by its agent ("Claude Code session"); never a top-level noun | a thread, a task, a step machine |
| spawned session | A session another session started with `spawn`: same experiment, same budget, any agent and platform. It cannot spawn. A person can open it and talk to it | a sub-task |
| session event | One item in a session's stream: a message, tool call, card, measurement arriving or status change | a log line |

## Platforms and agents

| Object | Means | Not |
| --- | --- | --- |
| agent platform | A managed service that runs an agent's loop and its sandbox as one durable session: Claude Managed Agents, AWS AgentCore, the OpenAI Agents API, Microsoft Foundry Agent Service, Google Agent Runtime. On the last two Ratchet supplies one agent image per agent, with the agent's harness inside; the platform still runs the sandbox | a sandbox we run, a model provider |
| agent trait | The one interface every platform adapter implements: start a session (repos, agent, environment, credentials, tools), send input, read events, answer a client tool call, stop, delete. The trait is designed against all five platforms ([add-platform.md](add-platform.md)) | a second way to start work |
| judge platform | An agent platform that also hosts judge sandboxes: a separate trait, in [../judge/model.md](../judge/model.md) | an agent platform for sessions |
| platform account | The account Ratchet uses on a platform: the customer's own when self-hosted (at least one), Ratchet's on Ratchet cloud, billed at cost through the budget | a cloud account Ratchet can reach |
| agent | A model on an agent platform, such as Claude on Claude Managed Agents, Codex on the OpenAI Agents API, Gemini on AWS AgentCore or Google Agent Runtime. It holds a short-lived git token for its own branch per repo and read-only connector credentials that leave out the holdout, never a model key | the platform, a login or key |
| cost tier | "$" to "$$$" on each agent, with the measured figure ("about $2.40 per trial here"), priced from the platform's usage ([add-agent.md](add-agent.md)) | a quality ranking |
| agent scoreboard | Per agent, filtered by evaluation kind and project: experiments, trials, kept, wins delivered, cost per win, median time to a win, Delivered value. Only from the judge's measurements here: "Measured by the judge on your code. Not a vendor benchmark." | a vendor benchmark |
| Auto | The agent choice that picks the best cost per win for this evaluation kind in this project among the agents the organization turned on, and says why in one line. A person can always pick a named agent. With no data it uses the default agent an owner sets in Customize, Agents. No agent is preselected by brand | a favoured agent |
| hand off | Moving a session to another agent: the new session starts in the same experiment on the same branch with a summary and the history; the old one ends Done, "Handed off to Codex" | a spawn, a new experiment |
| failover | A platform that is unavailable or rate-limited leaves the session Waiting for you with a Continue on another agent? card naming the agent Auto would choose. Policy may allow it without asking; a timeline line and an audit row record it | a retry on the same platform |
| environment | Image or setup script, size and network policy, on one platform. An agent proposes a change with `set_environment` | a server, a config file |
| attached repo | A repo on the code host the session works in, any number. Short-lived git token per repo, push only to the session's own branch | a clone Ratchet keeps |

## Spend

| Object | Means | Not |
| --- | --- | --- |
| budget | One per experiment, covering all its sessions and their judge time. It sits inside the project's optional monthly cap, inside the organization's (see [../organization/approvals.md](../organization/approvals.md)). Set when starting (default: the project's, else the organization's); an editor or owner can change it at any time; lowering it below the spend stops new measurements. The agent asks with `request_budget` | a bill, a per-session allowance |
| spend record | One append-only row of money or tokens, tied to a session and its agent, read from the platform's events and usage | an invoice |
| cost per trial | Defined in [../judge/model.md](../judge/model.md); derived from spend records | |
| where the spend went | The experiment's spend split by Kept, Within noise, Worse, Guardrail failed, Rejected, Failed and Not measured (agent work that never reached `measure`), as dollars and a share. Derived | a bill |
| compaction | Summarising a session's context and continuing. Ratchet does it on its own at 80 percent and says so in a timeline line; a person can also compact, or start a fresh session with a summary | forgetting |
| clean-up state | Removed (the platform confirms the session and its sandbox are gone) or Not removed (Ratchet retries and the cost counts until it is gone) | a status of the work |

Spending well is the sum of these, each owned once: the estimate on the Seal card and patience are the judge's ([../judge/tools.md](../judge/tools.md)); `history` first and the lower-cost default for `spawn` are in [tools.md](tools.md) and [prompts.md](prompts.md).

## Connectors and data

| Object | Means | Not |
| --- | --- | --- |
| connector | A data system or MCP server an agent's code may read: Snowflake, Databricks, DynamoDB, BigQuery, Postgres, Amazon S3, any MCP server. It holds two credentials and the datasets under it, with one translator for its system. Status: Connected, Reconnect, Not checked. The code host is not a connector. Steps in [add-connector.md](add-connector.md) | a dataset, a cloud account |
| dataset | A named table or file set under one connector. Ratchet records its id, versions, size and marking, never rows | a connector, a query |
| dataset version | One state of a dataset, with an id and a date; it never changes. A virtual dataset points at data left in place, pinned by an as-of time in the version id | latest, an editable snapshot |
| marking | A label that limits who may read a dataset. Holdout is the only one | a search tag |
| holdout | A marking: only the judge's credential reaches it. Enforced by the credentials; no control grants it to an agent (the control is absent, not disabled). Shown as "Restricted to the judge" | a split an agent can see |
| agent credential | Read-only on the views the person lists, holdout excluded | a write credential |
| judge credential | A separate read-only credential that also covers the holdout. No agent ever holds it | the agent credential |
| credential reference | Ratchet's record that a credential exists in a platform's store: id, connector, holder (agent or judge), who approved it. Never the value | the secret |
| credential store | The platform's own place for secrets: the Claude Managed Agents vault, AgentCore Identity, the agent's Microsoft Entra identity on Foundry (secrets via a Foundry connection to Azure Key Vault), the agent's Agent Identity or service account on Google Agent Runtime (secrets from Secret Manager; exact stores confirmed in the spike). The only way a credential reaches a session | Ratchet's database, a prompt |

| | Agent credential | Judge credential |
| --- | --- | --- |
| Kept in | The platform's credential store | The judge platform's credential store |
| Reaches a session as | A short-lived credential minted for that session | A short-lived credential minted for that judge sandbox |
| Held by | Agents | The judge sandbox only |

- Ratchet keeps neither value. The customer makes both with the connector's recipe; Ratchet holds no connector admin secret.
- The server never reads data. It stores ids, versions, sizes, markings, status and who added what. Checks run in a sandbox on a platform.
- A connector that is not Connected shows its datasets as "Not checked", never empty, and names the experiments that cannot measure until it is fixed.

## Skill packs and plugins

| Object | Means | Not |
| --- | --- | --- |
| skill pack | Versioned written know-how agents read before they propose changes ("Postgres query tuning"). Screen word: Skill, in Customize, Skills. An experiment selects packs and records the versions read. Agents only: the judge never reads one and a pack never holds holdout data. Steps in [add-skill-pack.md](add-skill-pack.md) | a dataset, a file in `.claude/skills/`, a plugin |
| plugin | A bundle that adds evaluations, skill packs, suggested connectors and recommended agents under their own kinds. No powers of its own: it can never mark, unmark or grant a dataset | a connector, an agent |

## What the server stores

The server relays a session and keeps a ledger. It runs no loop. A person can talk or interrupt at any time.

| Stores | Never stores |
| --- | --- |
| The conversation: what people and agents said ([../product/rulings.md](../product/rulings.md)) | Tool output |
| Tool call names and card states | File contents, diffs |
| Commit ids, dataset ids and versions, connector status | Data |
| Measurements | Raw logs |
| Spend and audit rows | |

Tool output and raw logs stay on the platform and are fetched when opened. They may expire, and the session says so; a failed fetch shows as failed, never as empty.

The terminal, Changes, Preview and Files panels read from the agent's own sandbox when opened and the server keeps no copy. A person with edit access can type in the terminal, a viewer sees it read-only, and opening one is an audit row ("Opened a terminal in Claude Code session"). It is never a Ratchet host or the judge sandbox. How the panels look is in [../design/behaviour.md](../design/behaviour.md).

## Lifecycle

1. **Start.** One message in the composer with the chips (Project, Repos, Agent, Environment, Budget), each defaulted. Ratchet starts a new experiment in the project and its lead session on the agent's platform.
2. **Work.** The agent drives and sets up through cards when needed. Until a person seals it the experiment is Not sealed: the agent explores, measures the baseline and proposes a definition with `seal`. It may stay Not sealed and end there.
3. **Idle or stopped.** The sandbox ends; the conversation does not. A person stopping a session makes it Done.
4. **Resume.** A new sandbox starts from the session's branches. Whatever was not pushed is gone.
5. **Clean-up.** Every session and sandbox is deleted on its platform and shown as Removed; one that is not shows Not removed and is retried.

Git is the hand-off: a trial is a set of commits, one per repo changed. Ratchet stores commit ids, never diffs; another session or the judge reads the code from git.

## Two offerings

- **Ratchet cloud (phase 1):** we run the server on our AWS account and every session on Ratchet's platform accounts; spend is rebilled through the budget.
- **Self-hosted (phase 2):** the same Helm chart in the client's own AWS, Azure or GCP account, on its own platform accounts.

One server, one chart. A promise that holds in one and not the other is a defect.

## How this area refers to the others

- Judge: by experiment id and trial id. Value models, evaluation templates and the accept rule are the judge's; the budget meter makes the judge refuse `measure` when spend reaches the budget.
- Organization: by actor id, project id and role. Platform accounts, environments, attached repos and credential references belong to the organization, experiments to a project. Approvals and policy: [../organization/approvals.md](../organization/approvals.md).
- Platform words (agent ids, vault ids, ARNs, regions) stay inside each platform's adapter.
