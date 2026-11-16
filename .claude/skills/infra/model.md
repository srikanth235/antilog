# Infra: objects and words

All of this is target. `docs/GLOSSARY.md` defines "Agent" for screens; here an agent is a worker: a model on an agent platform.

| Object | Means | Not |
| --- | --- | --- |
| agent platform | A managed service that runs an agent's loop and its sandbox as one durable session: Claude Managed Agents, the OpenAI Agents API, the AWS AgentCore harness. Each sits behind the one platform trait, through its own adapter | a sandbox backend we run, a model provider |
| platform trait | The one interface every platform adapter implements: start a session (repo, branch, agent definition, environment, credentials, limits), send input, read events, answer a client tool call, run a command in a sandbox with no agent, stop and delete | a second way to start work |
| platform account | The account Ratchet uses on a platform. Self-hosted: the customer's own, on whichever platforms they have. Ratchet cloud: Ratchet's, billed through to the customer at cost | a cloud account Ratchet can reach |
| credential store | The platform's own place for secrets a session may use (the Claude Managed Agents vault, AgentCore Identity, or the platform's equivalent). The only way a credential reaches a session | Ratchet's database, a prompt, a file in the repo |
| agent | A worker that proposes changes: a model on a platform, such as Claude on Claude Managed Agents, Codex on the OpenAI Agents API, or Gemini or Grok on AgentCore | the platform, the glossary's login or key |
| session | One durable instance of an agent on a platform: its loop, its sandbox, its files and its transcript. Two kinds: drafting session and run session | a chat thread stored by Ratchet |
| drafting session | An interactive session where a person and one agent work out the experiment. It ends by pushing the experiment's files to its config branch | a run |
| run session | One autonomous session per agent of a run, started from the config branch with the agent's budget share; it loops until it stops | the run (a run has one per agent) |
| agent branch | The git branch one run session may push to. A trial is a commit on it | the config branch |
| measure tool | The tool an agent calls to ask for a measurement. Ratchet provides it as a client tool and answers it over the session's event stream, so no platform has to reach the server | a number the agent computes |
| measuring platform | The one platform and environment an experiment's evaluator sandboxes run on. It must be able to run a command in a sandbox with no agent | the agents' platforms (a run may mix those) |
| evaluator sandbox | A sandbox on the measuring platform that measures one trial: it checks out the trial's commit, gets the evaluator's credential (which includes the holdout), runs the evaluation with no agent in it and is removed after | the benchmark (optimization's procedure), a run session |
| environment | An image and a size on a platform, named by id; what a session or an evaluator sandbox runs on | a server, a config file |
| budget | A stated spend limit for a run, set by optimization. Starts as even shares across the run's agents; each share is that agent's cap, and a stopped agent's unspent share goes evenly to the agents still iterating | a bill |
| spend record | One append-only row of money or tokens, tied to a run and an agent, read from the platform's events and usage. An agent's spend is its model tokens, its session's sandbox time and the evaluator time for its trials | an invoice |
| clean-up check | The verification that every session and sandbox of a run is ended and deleted on its platform | a hope |

## How this area refers to the others

- Optimization: by experiment id and run id. Optimization chooses which agents and how many, and sets the cap. Infra never reads a verdict or the accept rule.
- Context: by connector id and dataset id and version, only to hand a credential to a session through the credential store. Infra never names a table.
- Organization: by project id. Every platform account and environment belongs to an organization or project.

Platform words (agent ids, environment ids, vault ids, ARNs, regions) stay inside each platform's adapter.

## How work runs

1. **Drafting.** A person opens a draft experiment. Ratchet starts a drafting session on the chosen agent's platform with the repo, a git token for the config branch, and read-only connector credentials through the credential store. The person and the agent work out the goal, limits, subject, benchmark and accept rule. The session pushes the experiment's files to the config branch and ends. The draft is ready to approve.
2. **Approve.** Ratchet checks the run has a budget and a measuring platform, splits the budget into even shares, and starts one run session per agent, each from the config branch, with its own agent branch, its credentials and its limits (turns, wall-clock time, tokens where the platform offers them).
3. **Iterate.** The agent's code runs the experiment and reads data through connectors itself. It checks changes cheaply in its own sandbox; those numbers never count. It commits a change to its agent branch and calls the `measure` tool.
4. **Measure.** Ratchet receives the tool call from the session's events and starts an evaluator sandbox on the measuring platform with the trial's commit and the evaluator's credential. The evaluation runs there with no agent, and may bring up what it needs (for example the service and its database, from the subject at that commit) and remove it after. Ratchet reads the numbers, removes the sandbox and answers the tool call with the measured value and guardrails. A Kept trial is measured again in a fresh evaluator sandbox and on the holdout.
5. **Spend and stop.** Ratchet reads each session's usage from its events and records spend against the run and the agent. It stops a session at its share, its turn cap or the run's deadline, deletes it, and passes the unspent share evenly to the agents still iterating, recorded as a budget move. When the run ends, every session and sandbox is deleted and shown removed.

The server orchestrates and records: ids, versions, commit ids, numbers, states, spend and audit rows. It never connects to data. Transcripts and logs stay on the platform and are fetched when someone asks to see them.
