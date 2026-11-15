# Optimization: objects and words

Where `docs/GLOSSARY.md` already defines a word, this file keeps its meaning and says so.

| Object | Means | Not |
| --- | --- | --- |
| experiment | A goal Ratchet works on, with its schedule, limits and agents (glossary) | optimization (for the object), job, task, scheduled script |
| goal | The measured quantity and its direction, minimize or maximize | wish, target date |
| limit | A measured quantity that must not get worse while the goal improves (today a guardrail in `ratchet.yaml`) | budget (that is infra spend) |
| subject | Anything with a version and a diff that agents may change | a file path, a repo |
| benchmark | The trusted procedure that measures a candidate; points at data (context) and machines (infra) | test suite, agent script |
| accept rule | The pure function that turns the agent's best so far, noise, the trial's measured value, the guardrails and the confirmation into a verdict | a review, a vote |
| stop rule | The conditions that end an agent (its budget share is spent, or no kept trial in its last 8 trials) and a run (budget, deadline, or no kept trial by any agent in the last 12 trials) (target) | pause |
| run | One execution of an experiment, numbered `#14` (glossary) | attempt, session |
| trial | One measurement of one agent's change to the subject (target; the earlier code says candidate). A run has many per agent. Numbered per agent ("Claude Code trial 11") | win, a run |
| candidate | The earlier code's word for a trial | win |
| verdict | The accept rule's answer for a trial (earlier code: verified, rejected, failed confirm, blocked, pending confirm). On screen, one outcome per trial: Kept, Within noise, Worse, Guardrail failed, Failed holdout, Failed (did not run), Measuring (target) | opinion, label |
| kept | A trial that beat its agent's best by more than the acceptance criteria require, held the guardrails and was confirmed on the holdout (target). The agent learns only "Kept" or "Failed holdout" | accepted, a win |
| agent's best | The best kept value in one agent's lineage; what its next trial must beat (target) | the run's best |
| lineage | One agent's chain of kept trials. An agent may start from the experiment's best trial, and the lineage records it (target) | git history |
| receipt | The signed evidence a win carries: baseline, candidate, confirmation, holdout, noise, signed numbers, dataset versions, the evaluator image, the machine type and the credential scopes used | log, report text |
| win | A kept trial that is the best across all agents and moved the best value | any trial that looked good |
| delivered | Yearly value of merged wins (glossary) | locked in, realised |
| pending | Yearly value of wins in an open PR; not counted in Return (glossary) | unclaimed, waiting |
| return | Delivered divided by spent, shown as `68x` (glossary, written "Return") | a percentage or payback figure |

## How a run works (target)

1. Each agent runs its own loop in its own sandbox: write a change, check it cheaply there (numbers made there never count), ask for a measurement, read the result, then build on it if Kept or go back to its best, read the history, and go again.
2. A measurement runs on the evaluator's separate machines with the evaluation's repeats and reports the noise band. The agent sees the measured value and guardrail values on the evaluation data, never the holdout.
3. The accept rule, one pure function, decides each trial. A Kept trial is confirmed in a fresh evaluator sandbox and on the holdout.
4. An agent stops by the stop rule. The run's result is the best kept trial across all agents.

The agent decides what to try. Ratchet decides what counts. The run's budget is split evenly across its agents; infra meters it.

## How this area refers to the others

- Context: by dataset id and dataset version only. A benchmark names the exact versions it reads. It never names a connector or a credential.
- Infra: by environment id and machine type only. A benchmark names the environment it is measured in. It never names a provider, region or instance id.
- Organization: by project id. Every experiment belongs to one project.
