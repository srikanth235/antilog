# Optimization: objects and words

Where `docs/GLOSSARY.md` already defines a word, this file keeps its meaning and says so.

| Object | Means | Not |
| --- | --- | --- |
| optimization | A goal Ratchet works on, with its schedule, limits and agents (glossary) | job, task, scheduled script |
| goal | The measured quantity and its direction, minimize or maximize | wish, target date |
| limit | A measured quantity that must not get worse while the goal improves (today a guardrail in `ratchet.yaml`) | budget (that is infra spend) |
| subject | Anything with a version and a diff that agents may change | a file path, a repo |
| benchmark | The trusted procedure that measures a candidate; points at data (context) and machines (infra) | test suite, agent script |
| accept rule | The pure function that turns baseline, noise, candidate and confirmation into a verdict | a review, a vote |
| stop rule | The conditions that end a run: budget, deadline, plateau | pause |
| run | One execution of an optimization, numbered `#14` (glossary) | attempt, session |
| candidate | One proposed change to the subject, submitted by an agent | win |
| verdict | The accept rule's answer for a candidate: verified, rejected, failed confirm, blocked, pending confirm | opinion, label |
| receipt | The evidence a win carries: baseline, candidate, confirmation, holdout, noise, dataset versions, machines | log, report text |
| win | A candidate with a verified verdict that moved the best value | any candidate that looked good |
| delivered | Yearly value of merged wins (glossary) | locked in, realised |
| pending | Yearly value of wins in an open PR; not counted in Return (glossary) | unclaimed, waiting |
| return | Delivered divided by spent, shown as `68x` (glossary, written "Return") | a percentage or payback figure |

## How this area refers to the others

- Context: by dataset id and dataset version only. A benchmark names the exact versions it reads. It never names a source, a connector or a credential.
- Infra: by environment id and machine type only. A benchmark names the environment it is measured in. It never names a provider, region or instance id.
- Organization: by project id. Every optimization belongs to one project.
