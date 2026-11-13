# Infra: objects and words

`docs/GLOSSARY.md` defines "Agent" and "Where" (This machine or Cloud). This file keeps both. The glossary's Agent is a login or key that proposes changes; here it is a worker, and the model provider is whoever serves its model.

| Object | Means | Not |
| --- | --- | --- |
| cloud account | The customer's account or project on AWS, GCP or Azure where compute runs; dedicated to Ratchet | a login, a region |
| environment | A machine type and an image, named by id; what a benchmark or a workspace runs on | a server, a config file |
| workspace | Where agents propose: machines and storage an agent may use, with no access to the evaluator | a repo, a branch |
| evaluator | Where measuring happens: separate machines, pinned hardware, the only place the holdout is read | the benchmark (that is optimization's procedure) |
| agent | A worker that proposes changes: Claude Code, Codex or Grok (glossary) | harness (code only), connection |
| model provider | Whoever serves an agent's model, by subscription or API key | the agent |
| budget | A stated spend limit for a run, set by optimization | a bill |
| spend record | One append-only row of money or tokens, tied to a run | an invoice |
| machine | One launched instance, tagged with its run | a server someone owns |
| clean-up check | The verification that every tagged resource of a run is gone | a hope |
| where | This machine or Cloud: where agents run (glossary) | location |

## How this area refers to the others

- Optimization: by optimization id and run id. Optimization chooses which agents and how many, and sets the cap. Infra never reads a verdict or the accept rule.
- Context: by dataset id and version, only to grant a read to a machine. Infra never names a source or a table.
- Organization: by project id. Every account and environment belongs to a project.

Provider words (regions, instance ids, bucket names, role ARNs) stay in this area.
