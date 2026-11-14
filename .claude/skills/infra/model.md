# Infra: objects and words

`docs/GLOSSARY.md` defines "Agent" and "Where" (This machine or Cloud). This file keeps both. The glossary's Agent is a login or key that proposes changes; here it is a worker, and the model provider is whoever serves its model.

| Object | Means | Not |
| --- | --- | --- |
| launcher | The one component that starts and removes sandboxes, and mints the scoped credentials a run gets. Built into the server when self-hosted | a scheduler of experiments, a data reader |
| runner | The launcher installed as a small program in the customer's infrastructure, for Ratchet cloud. It connects out to the hosted server over HTTPS: it pulls jobs and the server streams updates. No inbound port | a second implementation, a worker that holds cloud access for Ratchet |
| sandbox | One isolated place where agent or evaluator code runs, started and removed by the launcher | a machine someone owns, a container image |
| sandbox backend | An adapter behind one trait that makes sandboxes: Kubernetes first, then VMs on AWS, GCP and Azure | a cloud account |
| scoped credential | A short-lived, read-only connector credential minted per run (see [../context/model.md](../context/model.md)) | a stored secret |
| egress proxy | The proxy that adds model keys to outbound HTTP, so a sandbox never holds one | a firewall |
| cloud account | The customer's account, project or cluster where sandboxes run; dedicated to Ratchet. On-prem Kubernetes counts | a login, a region |
| environment | A machine type and an image, named by id; what a benchmark or a workspace runs on | a server, a config file |
| workspace | Where agents propose: sandboxes (a separate node pool on Kubernetes) an agent may use, with no access to the evaluator | a repo, a branch |
| evaluator | Where measuring happens: separate machines (a separate node pool on Kubernetes), pinned hardware, the only place with the holdout credential | the benchmark (that is optimization's procedure) |
| agent | A worker that proposes changes: Claude Code, Codex or Grok (glossary) | harness (code only), connection |
| model provider | Whoever serves an agent's model, by subscription or API key | the agent |
| budget | A stated spend limit for a run, set by optimization | a bill |
| spend record | One append-only row of money or tokens, tied to a run | an invoice |
| machine | One launched instance, tagged with its run | a server someone owns |
| clean-up check | The verification that every tagged resource of a run is gone | a hope |
| where | This machine or Cloud: where agents run (glossary). This machine is a Mac running the macOS app (target), a backend like any other; measuring never runs there | location |

## How this area refers to the others

- Optimization: by optimization id and run id. Optimization chooses which agents and how many, and sets the cap. Infra never reads a verdict or the accept rule.
- Context: by dataset id and version, only to grant a read to a machine. Infra never names a connector or a table.
- Organization: by project id. Every account and environment belongs to a project.

Provider words (regions, instance ids, bucket names, role ARNs, node pools, NetworkPolicy) stay in this area, inside each backend's adapter.

## How work runs (target)

1. The launcher reads the connector admin secrets from the customer's secret manager and mints a scoped credential per run: read-only and holdout-excluding for the agent's sandbox, a separate one with the holdout for the evaluator's.
2. It starts a workspace sandbox with no route to the evaluator (on Kubernetes, a NetworkPolicy) and gives it the agent's credentials, a short-lived scoped git token, and an egress proxy that adds the model key on the way out.
3. The agent's code runs the experiment and reads data through connectors itself.
4. It starts the evaluator on different machines, measures, then removes every sandbox and shows it removed.

The server orchestrates and records: ids, versions, numbers, states and audit rows. It never connects to data, and Ratchet holds no access to the customer's cloud. There is no cross-account role and no serverless compute in Ratchet's account for now. Measuring on someone's laptop is not supported; a Mac may be the "This machine" backend where agents propose, with the same scoped credentials, budget and clean-up as any backend.
