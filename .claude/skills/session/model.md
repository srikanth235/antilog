# Session: objects and words

All of this is target. [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md) defines the words for screens; here an agent is a worker: a model on an agent platform.

| Object | Means | Not |
| --- | --- | --- |
| project | A group of experiments for one area of the code, with members and roles per project (see [../organization/model.md](../organization/model.md)). Every experiment belongs to exactly one project. A project page lists its experiments, the connectors they use, its budget and its members | a repo, a folder |
| experiment | The sealed goal and everything measured against it: the definition, its trials, scoreboard, wins and receipts. It belongs to one project, owns the budget and holds one or more sessions (see [../judge/model.md](../judge/model.md)). Its activity comes from its sessions: Working, Waiting for you, Done, Failed | a session, a thread |
| session | One agent's work inside an experiment: a chat with one agent in one sandbox on one agent platform, with any number of repos attached. It always belongs to an experiment and is not a top-level noun in the app, though the sidebar nests it under an open or pinned experiment. Its only state is a status: Working, Waiting for you, Done, Failed. A session a person stops is Done, with the line "Stopped by NAME". On screen it is named by its agent, such as "Claude Code session" | a thread, a task, a step machine |
| agent platform | A managed service that runs an agent's loop and its sandbox as one durable session: Claude Managed Agents, AWS AgentCore, the OpenAI Agents API, Microsoft Foundry Agent Service, Google Agent Runtime. On the last two Ratchet supplies one agent image per agent, with the agent's own harness inside it; the platform still runs the sandbox. Each sits behind the one agent trait, through its own adapter | a sandbox backend we run, a model provider |
| agent trait | The one interface every platform adapter implements: start a session (repos, an agent, an environment, credentials, tools), send input, read events, answer a client tool call, stop it and delete it | a second way to start work |
| platform account | The account Ratchet uses on a platform. Self-hosted: the customer's own, on whichever platforms they have, at least one. Ratchet cloud: Ratchet's, billed through to the customer at cost through the budget | a cloud account Ratchet can reach |
| agent | A model on an agent platform, such as Claude on Claude Managed Agents, Codex on the OpenAI Agents API, Gemini or Grok on AWS AgentCore, GitHub Copilot on Microsoft Foundry Agent Service, or Gemini on Google Agent Runtime | the platform, a login or key |
| environment | Image or setup script, size and network policy, on one platform. The agent may propose a change with `set_environment` | a server, a config file |
| attached repo | A repo on the code host that the session works in. A session attaches any number. It gets a short-lived git token per repo that may push only to the session's own branch | a clone Ratchet keeps |
| connector | A data system or MCP server an agent's code may read: Snowflake, Databricks, DynamoDB, BigQuery, Postgres, Amazon S3, any MCP server. It holds two credentials, one for agents and one for the judge, and the datasets under it. It has one translator for its system. The code host (GitHub, GitLab) is how repos attach and pull requests open, and is not a connector. Listed in Customize, Connectors. Status: Connected, Reconnect, Not checked | a dataset, a credential, a pipe Ratchet reads through, a cloud account |
| skill pack | A versioned pack of written know-how that agents read before they propose changes, for example "Postgres query tuning". Screen word: Skill, under Customize, Skills. An experiment selects the packs it uses and records the versions read. Audience: agents only; the judge never reads one, and a pack never holds holdout data. Not this repo's `.claude/skills/` | a dataset, an agent instruction file in `.claude/skills/`, a plugin, a note, `history` |
| plugin | A bundle that adds evaluations, skill packs, suggested connectors and recommended agents under their own kinds. It has no powers of its own and can never mark, unmark or grant access to a dataset (see [add-skill-pack.md](add-skill-pack.md)) | a connector, an agent |
| dataset | A named table or file set under one connector that an experiment reads. Ratchet records its id, its versions, its size and its marking, never its rows | a connector, a query |
| dataset version | One state of a dataset, with an id and a date. It never changes after it is created. An experiment and a receipt name the versions they read | latest, a snapshot someone can edit |
| marking | A label on a dataset that limits who may read it. Holdout is the only one. Shown in the Datasets table | a tag for search |
| holdout | A marking on a dataset: only the judge's credential reaches it. Enforced by the credentials, not by a setting: the agent credential leaves it out. No control grants it to an agent. Shown as "Restricted to the judge" | a validation split an agent can see |
| agent credential | A read-only credential for one connector, on the views the person lists, holdout excluded. Registered once, kept in the platform's credential store, and handed to a session as a short-lived credential minted for that session | a stored secret in Ratchet's database, a write credential |
| judge credential | A separate read-only credential for the same connector that also covers the holdout. Registered once, kept on the judge platform, minted per judge sandbox. No agent ever holds it | the agent credential |
| credential reference | Ratchet's record that a credential exists in a platform's credential store: its id, its connector, its holder (agent or judge) and who approved it. Never the value | the secret, a field in Ratchet's database that holds one |
| credential store | The platform's own place for secrets a session may use: the Claude Managed Agents vault, AgentCore Identity, the agent's Microsoft Entra agent identity on Foundry Agent Service (secrets reached through a Foundry connection to Azure Key Vault), the agent's Agent Identity or service account on Google Agent Runtime (secrets read from Secret Manager). The exact stores on the last two are confirmed in the spike, as Agent Identity is partly in preview. The only way a credential reaches a session | Ratchet's database, a prompt, a repo file |
| spawned session | A session another session started with `spawn`, in the same experiment and on the same budget, on any agent and platform. A spawned session cannot spawn. A person can open it, as its own chat inside the experiment, and talk to it | a sub-task, a thread |
| session event | One item in a session's stream: a message, a tool call, a card, a measurement arriving, or a status change | a log line |
| budget | One per experiment, covering all its sessions and their judge time. It sits inside its project's optional monthly cap, inside the organization's. Spend is model tokens, sandbox time and judge time. A person sets it when starting (a budget, defaulting to the project's default budget per experiment, which falls back to the organization's default from Settings, Budgets) and an editor or owner can change it at any time; the agent can ask with `request_budget`, which shows a Raise budget card. Lowering it below what is spent stops new measurements | a bill, a per-session allowance |
| spend record | One append-only row of money or tokens, tied to a session and its agent, read from the platform's events and usage. Shown per session on the experiment detail page and in Settings, Budgets | an invoice |
| clean-up check | The verification that every session and sandbox Ratchet started is ended and deleted on its platform. Each session shows its clean-up state: Removed, or Not removed while Ratchet retries | a hope |
| clean-up state | Removed (the platform confirms the session and its sandbox are gone) or Not removed (Ratchet retries, and the cost counts until it is gone). Shown in the sessions table | a status of the work |

## Connectors and datasets

A connector is registered once, in Customize, Connectors, or from an Add credential card in a chat. It has two credentials and the datasets under it.

| | Agent credential | Judge credential |
| --- | --- | --- |
| Reads | The views the person lists, read-only, holdout tables not granted | The same views and the holdout, read-only |
| Kept | The platform's credential store | The judge platform's credential store |
| Reaches a session as | A short-lived credential minted for that session | A short-lived credential minted for that judge sandbox |
| Held by | Agents, in their sessions | The judge sandbox only |

Ratchet keeps neither value. It keeps a credential reference. The customer makes both credentials in their own system with the recipe the connector gives (see [add-connector.md](add-connector.md)), and Ratchet never holds a connector admin secret.

- **Datasets live under a connector.** A dataset has a name, a size, a marking and versions. A dataset version never changes after it is created; a new state is a new version with its own id and date (for example v4 today, v3 three weeks ago). A virtual dataset points at data left in place and is pinned by an as-of time that is part of the version id.
- **A holdout is a marking on a dataset.** Only the judge's credential reaches it. In the Datasets table its row reads "Restricted to the judge" and has no grant control: the control is absent, not disabled. The dataset's page lists its versions and the experiments that use it.
- **Status.** Connected (the last check read the datasets), Reconnect (a credential was refused or expired), Not checked (nothing has checked it yet, or the check could not run). Datasets of a connector that is not Connected show "Not checked", never empty, and the page names the experiments that cannot measure until it is fixed.
- **The server never reads data.** It stores ids, versions, sizes, markings, status and who added what, never rows. Checks run in a sandbox on a platform, not on the Ratchet server.

## Agents in Customize

Customize, Agents holds the agents table. Settings, Agent platforms keeps the platform accounts only and links here. The table shows the agent (with its logo), its platform, its model, its parallel limit (how many sessions of it may work at once), the number of experiments it is used in and when it was last used. "Add agent" adds one. An agent's page shows its platform and account, model, parallel limit, "What the agent holds" (a short-lived git token for its own branch per repo, and read-only connector credentials that leave out the holdout; no model key, which belongs to the platform account) and its recent trials. There is no agent order and no per-agent budget share: the experiment's budget covers all its sessions, and the agent a person starts or the agent spawns is chosen in the chat.

## Sessions in an experiment

The experiment detail page lists the experiment's sessions in a table: the session (named by its agent), platform, started, ended, spend, state (Working, Done, Failed) and clean-up (Removed, Not removed). A session that is not removed raises a banner on the page: "The Gemini session was not removed. Ratchet retries, and its cost counts until it is gone." A spawned session's card shows its spend of the experiment's budget ("$14 of $100"). Settings, Budgets shows spend by experiment and by session.

## Talking to a session

The composer takes an @-mention of a session in the experiment ("@Codex focus on database queries"). The text goes to that session; without a mention it goes to the session being viewed. The first time a measurement arrives, the chat offers once, inline, "Notify me when this experiment needs me". It is a prompt, never a banner, and it asks the browser's notification permission only after a person says yes.

## The agent trait

One adapter per platform. The trait is designed against all five from the start (see [add-platform.md](add-platform.md)). A platform may also be the **judge platform**, which hosts judge sandboxes; that is a separate trait, described in [../judge/model.md](../judge/model.md).

## Session events and what the server stores

A session's stream carries messages (what people and agents said), tool calls and their results, cards (approval cards and result cards), and measurements arriving from the judge. A person can talk at any time and can interrupt.

| The server stores | The server never stores |
| --- | --- |
| The conversation: what people and agents said (decided by the owner; see [../product/rulings.md](../product/rulings.md)) | Tool output |
| Tool call names and card states | File contents |
| Commit ids, dataset ids and versions, connector status | Diffs |
| Measurements | Data |
| Spend and audit rows | Raw logs |

Tool output and raw logs stay on the platform and are fetched when someone opens them. They may expire, and the session says so. Failing to fetch them is shown as failed, never as empty.

## The terminal

A person with edit access can open a terminal in a session from the header (the Terminal toggle). It is a shell in that session's own sandbox on its agent platform, the same sandbox the agent works in, never a Ratchet host and never the judge sandbox. A viewer sees it read-only. Opening one is an audit row ("Opened a terminal in Claude Code session"). The conversation store rule holds: terminal input and output go between the browser and the platform and the server stores none of it. The same goes for the Changes, Preview and Files panels: they read from the sandbox when opened and the server keeps no copy.

## Lifecycle

1. **Start.** A person types one message in the composer with five chips (Project, Repos, Agent, Environment, Budget). The Project chip defaults to the project the person is in. Nothing else is asked first. Ratchet starts a new experiment in that project and its lead session on the agent's platform with the repos, the environment, credentials and tools.
2. **Work.** The agent drives. It reads, edits, calls tools and talks to the person. Setup happens as cards, when it is needed. Until a person seals the experiment it is Not sealed: the agent explores, measures the baseline and proposes a definition with `seal`. An experiment may stay Not sealed and end there.
3. **Idle or stopped.** The sandbox ends when the session is idle or stopped. The conversation does not. A person stopping a session makes it Done.
4. **Resume.** A new sandbox starts from the session's branches. Whatever was not pushed is gone.
5. **Clean-up.** Every session and sandbox is ended and deleted on its platform, and shown as Removed. One that is not deleted shows Not removed and is retried.

## Git is the hand-off

Each repo gets a short-lived git token that may push only to the session's own branch. A trial is a set of commits, one per repo changed. Ratchet stores commit ids, never diffs. Another session or the judge reads the code from git.

## Two offerings

- **Self-hosted:** a Docker Compose service (server and Postgres) using the customer's own platform accounts, at least one.
- **Ratchet cloud:** we host the server and run every session on Ratchet's platform accounts, billed at cost through the budget.

One core, two shells. A promise that holds in one and not the other is a defect.

## How this area refers to the others

- Judge: by experiment id and trial id. An evaluation template under Customize is where an experiment's definition starts; the sealed definition is the judge's (see [../judge/model.md](../judge/model.md)). The judge decides outcomes and the budget meter refuses `measure` when the experiment's spend reaches its budget. Sessions never read the accept rule.
- Organization: by actor id, project id and role. Every platform account, environment, attached repo and credential reference belongs to the organization, and every experiment to a project. Approvals and policy are in [../organization/approvals.md](../organization/approvals.md).

Platform words (agent ids, vault ids, ARNs, regions) stay inside each platform's adapter.
