# Session: objects and words

All of this is target. [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md) defines the words for screens; here an agent is a worker: a model on an agent platform.

| Object | Means | Not |
| --- | --- | --- |
| session | A chat with one agent in one sandbox on one agent platform, with any number of repos attached. Its only state is a status: Working, Waiting for you, Done, Failed. A session a person stops is Done, with the line "Stopped by NAME" | a thread, a task, a step machine |
| agent platform | A managed service that runs an agent's loop and its sandbox as one durable session: Claude Managed Agents, AWS AgentCore, the OpenAI Agents API, Microsoft Foundry Agent Service, Google Agent Runtime. On the last two Ratchet supplies one agent image per agent, with the agent's own harness inside it; the platform still runs the sandbox. Each sits behind the one agent trait, through its own adapter | a sandbox backend we run, a model provider |
| agent trait | The one interface every platform adapter implements: start a session (repos, an agent, an environment, credentials, tools), send input, read events, answer a client tool call, stop it and delete it | a second way to start work |
| platform account | The account Ratchet uses on a platform. Self-hosted: the customer's own, on whichever platforms they have, at least one. Ratchet cloud: Ratchet's, billed through to the customer at cost through the budget | a cloud account Ratchet can reach |
| agent | A model on an agent platform, such as Claude on Claude Managed Agents, Codex on the OpenAI Agents API, Gemini or Grok on AWS AgentCore, GitHub Copilot on Microsoft Foundry Agent Service, or Gemini on Google Agent Runtime | the platform, a login or key |
| environment | Image or setup script, size and network policy, on one platform. The agent may propose a change with `set_environment` | a server, a config file |
| attached repo | A repo on the code host that the session works in. A session attaches any number. It gets a short-lived git token per repo that may push only to the session's own branch | a clone Ratchet keeps |
| connector | A data system or MCP server an agent's code may read: Snowflake, Databricks, DynamoDB, BigQuery, Postgres, Amazon S3, any MCP server. The code host (GitHub, GitLab) is how repos attach and pull requests open | a dataset, a credential, a pipe Ratchet reads through |
| credential reference | Ratchet's record that a credential exists in a platform's credential store: its id, its connector, its holder and who approved it. Never the value | the secret, a field in Ratchet's database that holds one |
| credential store | The platform's own place for secrets a session may use: the Claude Managed Agents vault, AgentCore Identity, the agent's Microsoft Entra agent identity on Foundry Agent Service (secrets reached through a Foundry connection to Azure Key Vault), the agent's Agent Identity or service account on Google Agent Runtime (secrets read from Secret Manager). The exact stores on the last two are confirmed in the spike, as Agent Identity is partly in preview. The only way a credential reaches a session | Ratchet's database, a prompt, a repo file |
| spawned session | A session another session started with `spawn`, on the same contract and budget, on any agent and platform. A spawned session cannot spawn. A person can open it and talk to it | a sub-task, a thread |
| session event | One item in a session's stream: a message, a tool call, a card, a measurement arriving, or a status change | a log line |
| budget | One budget per session and the sessions it spawns. Spend is model tokens, sandbox time and judge time. Policy gives a standing allowance; `request_budget` raises it by a card | a bill |
| spend record | One append-only row of money or tokens, tied to a session and its agent, read from the platform's events and usage | an invoice |
| clean-up check | The verification that every session and sandbox Ratchet started is ended and deleted on its platform | a hope |

## The agent trait

One adapter per platform. The trait is designed against all five from the start (see [add-platform.md](add-platform.md)). A platform may also be the **judge platform**, which hosts judge sandboxes; that is a separate trait, described in [../judge/model.md](../judge/model.md).

## Session events and what the server stores

A session's stream carries messages (what people and agents said), tool calls and their results, cards (approval cards and result cards), and measurements arriving from the judge. A person can talk at any time and can interrupt.

| The server stores | The server never stores |
| --- | --- |
| The conversation: what people and agents said (the owner's open call; see [../product/rulings.md](../product/rulings.md)) | Tool output |
| Tool call names and card states | File contents |
| Commit ids | Diffs |
| Measurements | Data |
| Spend and audit rows | Raw logs |

Tool output and raw logs stay on the platform and are fetched when someone opens them. They may expire, and the session says so. Failing to fetch them is shown as failed, never as empty.

## Lifecycle

1. **Start.** A person types one message in the composer with three chips (Repos, Agent, Environment). Nothing else is asked first. Ratchet starts the session on the agent's platform with the repos, the environment, credentials and tools.
2. **Work.** The agent drives. It reads, edits, calls tools and talks to the person. Setup happens as cards, when it is needed.
3. **Idle or stopped.** The sandbox ends when the session is idle or stopped. The conversation does not. A person stopping a session makes it Done.
4. **Resume.** A new sandbox starts from the session's branches. Whatever was not pushed is gone.
5. **Clean-up.** Every session and sandbox is ended and deleted on its platform, and shown as removed.

## Git is the hand-off

Each repo gets a short-lived git token that may push only to the session's own branch. A trial is a set of commits, one per repo changed. Ratchet stores commit ids, never diffs. Another session or the judge reads the code from git.

## Two offerings

- **Self-hosted:** a Docker Compose service (server and Postgres) using the customer's own platform accounts, at least one.
- **Ratchet cloud:** we host the server and run every session on Ratchet's platform accounts, billed at cost through the budget.

One core, two shells. A promise that holds in one and not the other is a defect.

## How this area refers to the others

- Judge: by contract id and trial id. The judge decides outcomes and the budget meter refuses `measure` when spend is gone. Sessions never read the accept rule.
- Organization: by actor id and role. Every platform account, environment, attached repo and credential reference belongs to the organization. Approvals and policy are in [../organization/approvals.md](../organization/approvals.md).

Platform words (agent ids, vault ids, ARNs, regions) stay inside each platform's adapter.
