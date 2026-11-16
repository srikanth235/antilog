# Judge: objects and words

Where [the glossary](../../../docs/GLOSSARY.md) already defines a word, this file keeps its meaning. All of this is target; nothing is built. The session is the other object; see [../session/model.md](../session/model.md).

| Object | Means | Not |
| --- | --- | --- |
| contract | A sealed definition of "better": the metric and its direction, the evaluation, guardrails, the data, repeats and the acceptance criteria. Reusable by any session | evaluation (that is the command and files inside it), benchmark, experiment |
| contract version | One immutable state of a contract. A change makes a new version. Numbers never carry across versions | a draft, an edit |
| metric | The measured quantity and its direction, minimize or maximize | wish, deadline |
| evaluation | The command and its protected files, held in one or more repos at pinned commits. Agents may read it and never change it | evaluator, test suite |
| protected path | A file or glob in the evaluation. It names its repo | a file path alone |
| guardrail | A measured quantity that must not get worse while the metric improves | budget |
| data | The evaluation data an agent sees values for, and the holdout it never sees. Named by dataset id and version | a connector, a credential |
| holdout | Data only the judge sandbox may read. Enforced by credentials: the agent's credential excludes it. Shown as "Restricted to the judge" | a validation split an agent can see |
| seal | A hash over the contract and its protected files at pinned commits, made when a person approves a contract that passed the seal checks | a signature on a win |
| seal checks | The baseline measurement, the noise check and the cheat checks | review |
| trial | One `measure` call: a set of commits (one per repo changed), the trial it started from, and a one-line hypothesis | a session, a win |
| measurement | The judge's signed result for one trial under one contract version | an agent's own number |
| outcome | One of: Measuring, Kept, Within noise, Worse, Guardrail failed, Failed holdout, Rejected (changed a protected path), Failed (did not run) | verdict, opinion |
| Kept | A trial that beat the trial it started from by more than the bar, held every guardrail and was confirmed in a fresh judge sandbox and on the holdout | accepted, a win |
| bar | The gain a trial must show. Tied to the noise and raised with the number of trials tried. The exact rule is open and settled with the first code | a fixed threshold |
| best | The contract's best Kept trial | any trial that looked good |
| win | A contract's best Kept trial, ready to deliver or delivered | a Kept trial that is not the best |
| receipt | The signed record of a win | a log, report text |
| second opinion | The read of a winning diff by an agent from a different model provider than the proposer. It flags and never decides | a review that blocks |
| watch | The judge measuring a delivered contract again, on a schedule or when the default branch moves | a loop |
| regression | A watch result worse than the delivered value by more than the noise | a failed trial |
| scoreboard | Trials for a contract, by session | a ranking of agents |
| delivered | Yearly value of merged wins (glossary) | locked in, realised |
| pending | Yearly value of wins in an open pull request; not counted in Return (glossary) | unclaimed |
| Return | Delivered divided by spent, shown as `68x` (glossary). Never "ROI" | a percentage or payback figure |

## How the objects relate

- A contract has many versions. A version has one seal.
- A trial belongs to one contract version. It names the trial it started from: the baseline or an earlier Kept trial. That chain is the lineage.
- A trial has one measurement and one outcome.
- A win is the best Kept trial of a contract version. A win has one receipt and one pull request per repo changed.
- A watch belongs to a delivered contract. Each watch result is a measurement.

## Multi-repo

A session may attach any number of repos. A trial is a set of commits, one per repo changed. A protected path names its repo, so the seal covers each repo at a pinned commit and a trial that touches a protected path in any repo is Rejected. `deliver` opens one pull request per repo changed, linked to each other.

## What is stored, and what is derived

Stored: contract and version ids, seal hashes, dataset ids and versions, trial ids, commit ids per repo, the hypothesis, the outcome, measured values, repeats, noise, receipt ids and signatures, watch settings, spend rows. Never diffs, file contents, tool output or data.

Derived from measurements, never stored on their own: the best, the baseline-to-best movement, the scoreboard, a contract's state (Sealed, Watching, Regressed), delivered, pending and Return.

## How this area refers to the others

- Session: by session id and commit ids. The judge never names a platform's agent ids, regions or instance ids.
- Data: by dataset id and version only. A contract never names a connector or a credential.
- Organization: by organization id and member id for approvals and audit rows.
