# Judge: objects and words

Where [the glossary](../../../docs/GLOSSARY.md) already defines a word, this file keeps its meaning. All of this is target; nothing is built. The session is the other object; see [../session/model.md](../session/model.md).

| Object | Means | Not |
| --- | --- | --- |
| experiment | The sealed goal and everything measured against it. The sealed part is a definition of "better": the metric and its direction, the evaluation, guardrails, the data, repeats and the acceptance criteria. Around it: its trials, scoreboard, wins, receipts, watch and budget. Its sessions (see [../session/model.md](../session/model.md)) do the work. State: Not sealed, Sealed, Watching, Regressed | evaluation (that is the command and files inside it), benchmark, contract, a session |
| experiment version | One immutable state of an experiment's definition. A change makes a new version of the same experiment. Numbers never carry across versions | a draft, an edit |
| metric | The measured quantity and its direction, minimize or maximize | wish, deadline |
| evaluation | The command and its protected files, held in one or more repos at pinned commits. Agents may read it and never change it | evaluator, test suite |
| protected path | A file or glob in the evaluation. It names its repo | a file path alone |
| guardrail | A measured quantity that must not get worse while the metric improves | budget |
| data | The evaluation data an agent sees values for, and the holdout it never sees. Named by dataset id and version. A dataset lives under a connector and a dataset version never changes (see [../session/model.md](../session/model.md)) | a connector, a credential |
| holdout | Data only the judge sandbox may read. Enforced by credentials: the agent's credential excludes it. Shown as "Restricted to the judge" | a validation split an agent can see |
| seal | A hash over the definition and its protected files at pinned commits, made when a person approves an experiment that passed the seal checks | a signature on a win |
| seal checks | The baseline measurement, the noise check and the cheat checks | review |
| trial | One `measure` call: a set of commits (one per repo changed), the trial it started from, and a one-line hypothesis | a session, a win |
| measurement | The judge's signed result for one trial under one experiment version | an agent's own number |
| outcome | One of: Measuring, Kept, Within noise, Worse, Guardrail failed, Failed holdout, Rejected (changed a protected path), Failed (did not run) | verdict, opinion |
| Kept | A trial that beat the trial it started from by more than the bar, held every guardrail and was confirmed in a fresh judge sandbox and on the holdout | accepted, a win |
| bar | The gain a trial must show. Tied to the noise and raised with the number of trials tried. The exact rule is settled with the first code | a fixed threshold |
| best | The experiment's best Kept trial | any trial that looked good |
| win | An experiment's best Kept trial, ready to deliver or delivered | a Kept trial that is not the best |
| receipt | The signed record of a win. Three groups: Experiment (version and seal, values, repeats, noise, trials tried, holdout result, second opinion, spend), Data (dataset versions read, commits per repo, the credential scopes used: agents read-only with the holdout excluded, the judge its own) and Compute (judge platform and environment, judge image and digest, agents and sessions with their platforms). "Signed by the judge", with Verify again | a log, report text |
| evaluation kind | The shape of an evaluation: Speed, Quality, Size or Cost. The agent drafts an experiment from a kind and its template, and the Seal experiment card shows the kind with the planted cheats the judge runs for it | a catalogue page |
| second opinion | The read of a winning diff by an agent from a different model provider than the proposer. It flags and never decides | a review that blocks |
| watch | The judge measuring a delivered experiment again, on a schedule or when the default branch moves | a loop |
| regression | A watch result worse than the delivered value by more than the noise | a failed trial |
| scoreboard | Trials for an experiment, by session | a ranking of agents |
| delivered | Yearly value of wins whose pull request is merged (glossary). A win counts as Delivered only once merged | locked in, realised |
| pending | Yearly value of wins in an open pull request; not counted in Return, and it stays Pending while the pull request is open (glossary) | unclaimed |
| Return | Delivered divided by spent, shown as `68x` (glossary). Never "ROI" | a percentage or payback figure |

## How the objects relate

- An experiment has many sessions and many versions. A version has one seal. The experiment owns one budget.
- A trial belongs to one experiment version, and was made by one of its sessions. It names the trial it started from: the baseline or an earlier Kept trial. That chain is the lineage.
- A trial has one measurement and one outcome.
- A win is the best Kept trial of an experiment version. A win has one receipt and one pull request per repo changed.
- A watch belongs to a delivered experiment. Each watch result is a measurement.

## Multi-repo

A session may attach any number of repos, and an experiment may span several. A trial is a set of commits, one per repo changed. A protected path names its repo, so the seal covers each repo at a pinned commit and a trial that touches a protected path in any repo is Rejected. `deliver` opens one pull request per repo changed, linked to each other.

## What is stored, and what is derived

Stored: experiment and version ids, seal hashes, dataset ids and versions, trial ids, commit ids per repo, the hypothesis, the outcome, measured values, repeats, noise, receipt ids and signatures, watch settings, spend rows. Never diffs, file contents, tool output or data.

Derived from measurements, never stored on their own: the best, the baseline-to-best movement, the scoreboard, an experiment's state (Not sealed, Sealed, Watching, Regressed) and its activity (from its sessions), delivered, pending and Return.

## How this area refers to the others

- Session: by session id and commit ids. The judge never names a platform's agent ids, regions or instance ids.
- Data: by dataset id and version only. An experiment never names a connector or a credential.
- Organization: by organization id and member id for approvals and audit rows.
