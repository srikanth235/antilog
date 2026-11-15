# Infra: objects and words

`docs/GLOSSARY.md` defines "Agent" and "Where" (This machine or Cloud). This file keeps both. The glossary's Agent is a login or key that proposes changes; here it is a worker, and the model provider is whoever serves its model.

| Object | Means | Not |
| --- | --- | --- |
| launcher | The one component that starts and removes sandboxes, and mints the scoped credentials a run gets. Built into the server when self-hosted | a scheduler of experiments, a data reader |
| where work runs | A per-organization setting, owner-only: Your infrastructure or Ratchet-managed. Self-hosted offers only Your infrastructure. An enterprise on Ratchet cloud may switch to Ratchet-managed after a warning (target) | a backend, a region |
| Ratchet-managed | The value of where work runs in which sandboxes run on Cloudflare Containers in Ratchet's account, on Ratchet cloud only. It is one more sandbox backend behind the same trait and job queue. Largest sandbox 4 vCPU, 12 GiB, 20 GB disk, no GPUs; timing benchmarks are noisier on shared machines (target) | a second launcher, the default for enterprises |
| runner | The launcher installed as a small program in the customer's infrastructure, for Ratchet cloud. It connects out to the hosted server over HTTPS: it pulls jobs and the server streams updates. No inbound port | a second implementation, a worker that holds cloud access for Ratchet |
| managed runner | The runner we operate for Ratchet-managed organizations. It holds their connector admin secrets in our encrypted secret store; the server never does (target) | a runner in the customer's infrastructure |
| sandbox | One isolated place where agent or evaluator code runs, started and removed by the launcher | a machine someone owns, a container image |
| sandbox backend | An adapter behind one trait that makes sandboxes: Kubernetes first, then VMs on AWS, GCP and Azure | a cloud account |
| scoped credential | A short-lived, read-only connector credential minted per run (see [../context/model.md](../context/model.md)) | a stored secret |
| egress proxy | The proxy that adds model keys to outbound HTTP, so a sandbox never holds one, whatever the key source | a firewall |
| cloud account | The customer's account, project or cluster where sandboxes run under Your infrastructure; dedicated to Ratchet. On-prem Kubernetes counts. Ratchet-managed has none | a login, a region |
| environment | A machine type and an image, named by id; what a benchmark or a workspace runs on | a server, a config file |
| workspace | Where one agent works: its own sandbox (a separate node pool on Kubernetes), kept for its whole loop, with no access to the evaluator. One per agent (target) | a repo, a branch |
| evaluator | Where measuring happens: separate machines (a separate node pool on Kubernetes), pinned hardware, the only place with the holdout credential. A measurement job starts an evaluator sandbox for one trial and removes it after (target) | the benchmark (that is optimization's procedure) |
| agent | A worker that proposes changes: Claude Code, Codex or Grok (glossary) | harness (code only), connection |
| model provider | Whoever serves an agent's model, by subscription or API key | the agent |
| model key source | Ratchet-supplied (Ratchet-managed only, its default, billed at cost through the run's budget and spend records) or Your own key (always available; the only choice for Your infrastructure; kept in the customer's secret manager, or our encrypted secret store for Ratchet-managed) (target) | a key an agent holds |
| budget | A stated spend limit for a run, set by optimization. Starts as even shares across the run's agents; each share is that agent's cap, and a stopped agent's unspent share goes evenly to the agents still iterating (target) | a bill |
| spend record | One append-only row of money or tokens, tied to a run and an agent. An agent's spend is its model tokens, its sandbox and the evaluator time for its trials (target) | an invoice |
| machine | One launched instance, tagged with its run | a server someone owns |
| clean-up check | The verification that every tagged resource of a run is gone | a hope |
| where | This machine or Cloud: where agents run (glossary). This machine is a Mac running the macOS app (target), a backend like any other; measuring never runs there | location |

## How this area refers to the others

- Optimization: by experiment id and run id. Optimization chooses which agents and how many, and sets the cap. Infra never reads a verdict or the accept rule.
- Context: by dataset id and version, only to grant a read to a machine. Infra never names a connector or a table.
- Organization: by project id. Every account and environment belongs to a project.

Provider words (regions, instance ids, bucket names, role ARNs, node pools, NetworkPolicy) stay in this area, inside each backend's adapter.

## How the runner and the server talk (target)

- The runner connects out to the server over HTTPS and long-polls or streams for jobs. It opens no inbound port.
- Jobs go down: start a workspace, start an evaluator, remove a run's sandboxes, fetch a diff or log. Up come ids, numbers, states and spend. Connector secrets, credentials, data, diffs and logs do not travel up on their own.
- Self-hosted, the launcher runs in process in the server and the same job contract applies, with no network between them.
- A diff or a log is fetched through the runner when someone asks to see it, and is never stored by the server.
- If the server is away, the runner keeps enforcing the spend cap and the clean-up check on what it already runs, and reports when it can. It starts nothing new without a job.

## How work runs (target)

Each agent runs its own loop (see [../optimization/model.md](../optimization/model.md)), so the steps below repeat for measuring.

1. The launcher reads the connector admin secrets from the customer's secret manager (Ratchet-managed: our encrypted secret store, held by the managed runner) and mints a scoped credential per run: read-only and holdout-excluding for the agent's sandbox, a separate one with the holdout for the evaluator's.
2. It starts one workspace sandbox per agent, each with no route to the evaluator (on Kubernetes, a NetworkPolicy) and gives it the agent's credentials, a short-lived scoped git token, and an egress proxy that adds the model key on the way out.
3. The agent's code runs the experiment and reads data through connectors itself. It checks its changes cheaply there; those numbers never count.
4. When an agent asks for a measurement, a measurement job starts an evaluator sandbox on different machines for that one trial. The evaluation may bring up what it needs inside it (for example the service and its database, from the subject at that commit) and removes it after. A Kept trial is measured again in a fresh evaluator sandbox and on the holdout.
5. The run's budget starts as even shares, and spend is attributed to the agent that caused it. When an agent stops, its sandboxes are removed at once and its unspent share is split evenly among the agents still iterating, recorded as a budget move. When the run ends, every sandbox is removed and shown removed.

The server orchestrates and records: ids, versions, numbers, states and audit rows. It never connects to data, and Ratchet holds no access to the customer's cloud. There is no cross-account role. Compute in Ratchet's account exists only for Ratchet-managed sandboxes, which an owner chooses, and they are removed when the run ends. Measuring on someone's laptop is not supported; a Mac may be the "This machine" backend where agents propose, with the same scoped credentials, budget and clean-up as any backend.
