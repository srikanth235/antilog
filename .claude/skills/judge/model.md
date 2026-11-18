# Judge: objects and words

Where [the glossary](../../../docs/GLOSSARY.md) defines a word, this file keeps its meaning. All of this is target. The session is the other object ([../session/model.md](../session/model.md)).

| Object | Means | Not |
| --- | --- | --- |
| experiment | The sealed goal and everything measured against it. The sealed part defines "better": metric and direction, evaluation, guardrails, data, repeats, quick check, patience, value model and acceptance criteria. Around it: trials, scoreboard, wins, receipts, watch and budget. Its sessions do the work. State: Not sealed, Sealed, Watching, Regressed | evaluation (that is the command and files inside it), benchmark, a session |
| experiment version | One immutable state of an experiment's definition. A change makes a new version of the same experiment. Numbers never carry across versions | a draft, an edit |
| metric | The measured quantity and its direction, minimize or maximize | wish, deadline |
| evaluation | The command and its protected files, in one or more repos at pinned commits. Agents may read it and never change it | a test suite |
| evaluation template | A reusable starting point in Customize, Evaluations: kind, command, direction, guardrails, standard planted cheats and an environment ([add-evaluation-kind.md](add-evaluation-kind.md)). The agent drafts a seal from one and the sealed definition holds its own copy, so editing or removing a template never changes a sealed experiment | a sealed definition, a benchmark |
| skill pack | Written know-how agents read ([../session/model.md](../session/model.md)). The judge never reads one; the receipt lists the versions read | an input to the judge, a grant on a dataset |
| value model | What one unit of the metric is worth in dollars a year, with a source (text and optional link) and who set it: "Each 100 ms of checkout p95 is worth $3,600 a year". A project owner sets one per metric. Sealed by id and version, so it never changes for that version. An experiment may have none ("No value set"). Agents never set or edit one | an agent's estimate, a price |
| quick check | The first step of `measure`: one repeat on the evaluation data; the full measurement follows only if it beats the best by more than the noise. A trial that stops here is Within noise or Worse, marked "Quick check", at a lower cost. Sealed (on or off, default on), run only in the judge, never decides Kept | a measurement an agent runs, a second way to mark a win |
| patience | Trials in a row, across all sessions, that may fail to be Kept before the experiment pauses (default 6, sealed): it becomes Waiting for you with a Keep going? card | a timeout, a budget |
| cost per trial | The agent's spend since that session's previous trial, plus the judge time for the measurement. Derived from spend records, never stored. "Cost per kept trial" is spent divided by Kept trials | a price list |
| protected path | A file or glob in the evaluation. It names its repo | a file path alone |
| guardrail | A measured quantity that must not get worse while the metric improves | budget |
| data | The evaluation data an agent sees values for, and the holdout it never sees, named by dataset id and version (datasets: [../session/model.md](../session/model.md)) | a connector, a credential |
| holdout | Data only the judge sandbox may read, enforced by credentials. Shown as "Restricted to the judge" | a validation split an agent can see |
| seal | A hash over the definition and its protected files at pinned commits, made when a person approves an experiment that passed the seal checks | a signature on a win |
| seal checks | The baseline measurement, the noise check (the same commit paired with itself) and the cheat checks | review |
| trial | One `measure` call: a set of commits (one per repo changed), the trial it started from, and a one-line hypothesis | a session, a win |
| measurement | The judge's signed result for one trial under one experiment version | an agent's own number |
| outcome | One of: Measuring, Kept, Within noise, Worse, Guardrail failed, Failed holdout, Rejected (changed a protected path), Failed (did not run) | verdict, opinion |
| Kept | A trial that beat the trial it started from by more than the bar, held every guardrail and was confirmed in a fresh judge sandbox and on the holdout | accepted, a win |
| bar | What a trial's paired gain must clear: the one-sided lower confidence bound of the mean paired difference above zero, at confidence 1 − 0.05/k for the k-th trial measured in the experiment version | a fixed threshold |
| best | The experiment's best Kept trial | any trial that looked good |
| win | An experiment's best Kept trial, ready to deliver or delivered | a Kept trial that is not the best |
| receipt | The signed record of a win, in three groups. **Experiment:** version and seal, the value model with its source and who set it, values, repeats, noise, trials tried, holdout result (Passed or Failed), second opinion, spend, the cost of the win (counting trials not kept) and the win's own Return. **Data:** dataset versions read, skill pack versions read, commits per repo, credential scopes used (agents read-only with the holdout excluded, the judge its own). **Compute:** judge platform and environment, judge image and digest, agents and sessions with their platforms. "Signed by the judge", with Verify again | a log, report text |
| evaluation kind | The shape of an evaluation: Speed, Quality, Size or Cost. The Seal experiment card shows the kind with the planted cheats the judge runs for it | a sealed definition |
| second opinion | The read of a winning diff by an agent from a different model provider than the proposer. It flags and never decides | a review that blocks |
| watch | The judge pairing the delivered commit with the current head of the default branch, on a schedule or when the default branch moves | a loop |
| regression | A watch result where the current head is worse than the delivered commit: the lower confidence bound of the paired loss is above zero at 95% | a failed trial |
| cumulative trial | A trial the judge measures when patience is reached: a session's branch head that differs from every measured trial, against the best. It counts in k | a person's pick |
| holdout cap | The sealed number of holdout checks an experiment version allows (default 20). At the cap a new version with a new holdout is needed | a budget |
| scoreboard | Trials for an experiment, by session, with cost per kept trial. Agents across experiments: the agent scoreboard ([../session/model.md](../session/model.md)) | a ranking of agents |
| delivered | Yearly value, measured offline by the judge, of wins whose pull request is merged: (baseline minus best) times the value per unit. With no value model a win shows the metric only and counts in no dollar figure. While an experiment is Regressed, the regressed amount is taken out of Delivered | locked in, realised |
| at risk | The regressed part of Delivered, shown as "At risk" until a fix is Kept and merged | a loss, a failed trial |
| pending | Yearly value of wins in an open pull request. Not in Return | unclaimed |
| Return | Delivered divided by everything spent in the scope (project, organization or time range), including experiments that found nothing, were stopped or failed, shown as `68x`. | a percentage or payback figure, Delivered divided by the spend of winning experiments only |

## How the objects relate

- An experiment belongs to one project (see [../organization/model.md](../organization/model.md)), and has many sessions and many versions. It may name the evaluation template and the skill pack versions it started from. A version has one seal. The experiment owns one budget.
- A trial belongs to one experiment version, and was made by one of its sessions. It names the trial it started from: the baseline or an earlier Kept trial. That chain is the lineage.
- A trial has one measurement and one outcome. A trial that stops at the quick check has a measurement too, marked "Quick check".
- An experiment version names zero or one value model version. The Seal experiment card's estimate (what one measurement costs, how many trials the budget buys) comes from the seal checks, which already measured the baseline.
- A win is the best Kept trial of an experiment version. A win has one receipt and one pull request per repo changed.
- A watch belongs to a delivered experiment. Each watch result is a measurement.

## Multi-repo

An experiment may span several repos. A trial is a set of commits, one per repo changed. A protected path names its repo, so the seal covers each repo at a pinned commit and a trial that touches a protected path in any repo is Rejected. `deliver` opens one linked pull request per repo changed.

## What is stored, and what is derived

Stored: experiment and version ids, value models (one id per project and metric, a new version on each change, never edited), win events (a pull request opened, merged or closed, a regression read by the watch or its recovery; append-only, written only by Antilog, never by an agent or a person), the quick check and patience settings, project id, seal hashes, dataset ids and versions, skill pack ids and versions read, the evaluation template id and version it started from, trial ids, commit ids per repo, the hypothesis, the outcome, measured values, repeats, noise, receipt ids and signatures, watch settings, spend rows. Never diffs, file contents, tool output or data.

Derived from measurements, win events and spend records, never stored on their own (`antilog-core/src/value.rs`): the best, the baseline-to-best movement, the scoreboard, cost per trial, where the spend went ([../session/model.md](../session/model.md)), an experiment's state (Not sealed, Sealed, Watching, Regressed) and its activity (from its sessions), delivered, pending, at risk and Return.

## How this area refers to the others

- Session: by session id and commit ids. The judge never names a platform's agent ids, regions or instance ids.
- Data: by dataset id and version only. An experiment never names a connector or a credential.
- Organization: by organization id, project id and member id for approvals and audit rows.
