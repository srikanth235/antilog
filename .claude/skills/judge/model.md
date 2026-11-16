# Judge: objects and words

Where [the glossary](../../../docs/GLOSSARY.md) already defines a word, this file keeps its meaning. All of this is target; nothing is built. The session is the other object; see [../session/model.md](../session/model.md).

| Object | Means | Not |
| --- | --- | --- |
| experiment | The sealed goal and everything measured against it. The sealed part is a definition of "better": the metric and its direction, the evaluation, guardrails, the data, repeats and the acceptance criteria. Around it: its trials, scoreboard, wins, receipts, watch and budget. Its sessions (see [../session/model.md](../session/model.md)) do the work. State: Not sealed, Sealed, Watching, Regressed | evaluation (that is the command and files inside it), benchmark, contract, a session |
| experiment version | One immutable state of an experiment's definition. A change makes a new version of the same experiment. Numbers never carry across versions | a draft, an edit |
| metric | The measured quantity and its direction, minimize or maximize | wish, deadline |
| evaluation | The command and its protected files, held in one or more repos at pinned commits. Agents may read it and never change it. Also the name of a reusable template under Customize, Evaluations (see evaluation template) | evaluator, test suite |
| evaluation template | A reusable starting point in Customize, Evaluations: a kind (Speed, Quality, Size or Cost), a command, a direction, guardrails, the standard planted cheats and an environment. A sealed experiment starts from one: the agent drafts a seal from a template, and the sealed definition holds its own copy, so editing or removing a template never changes a sealed experiment. The seal checks (baseline, noise, cheats) stay the judge's | a sealed definition, a benchmark |
| skill pack | Written know-how agents read, selected per experiment (see [../session/model.md](../session/model.md)). The judge never reads one, it never holds holdout data, and the receipt lists the versions read | an input to the judge, a grant on a dataset |
| value model | What one unit of the metric is worth in dollars a year, with a source (text and an optional link) and who set it: "Each 100 ms of checkout p95 is worth $3,600 a year". A project owner sets one per metric. Part of the sealed definition, referenced by id and version, so it never changes for that version. An experiment may have none ("No value set"). Agents never set or edit one | an agent's estimate, a price |
| quick check | The first step of `measure`: one repeat on the evaluation data. Only if it beats the best by more than the noise does the full measurement follow. A trial that stops here is Within noise or Worse, marked "Quick check", at a lower cost. Part of the sealed definition (on or off, default on), run only in the judge. It never decides Kept | a measurement an agent runs, a second way to mark a win |
| patience | The number of trials in a row, across all sessions, that may fail to be Kept before the experiment pauses (default 6, sealed). It then becomes Waiting for you with a Keep going? card | a timeout, a budget |
| cost per trial | The agent's spend since that session's previous trial, plus the judge time for the measurement. Derived from spend records, never stored. "Cost per kept trial" is spent divided by Kept trials | a price list |
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
| receipt | The signed record of a win. Three groups: Experiment (version and seal, the value model with its source and who set it, values, repeats, noise, trials tried, holdout result, second opinion, spend, the cost of the win, which counts trials not kept, and the win's own Return), Data (dataset versions read, skill pack versions read, commits per repo, the credential scopes used: agents read-only with the holdout excluded, the judge its own) and Compute (judge platform and environment, judge image and digest, agents and sessions with their platforms). "Signed by the judge", with Verify again | a log, report text |
| evaluation kind | The shape of an evaluation: Speed, Quality, Size or Cost. The agent drafts an experiment from a kind and its template, and the Seal experiment card shows the kind with the planted cheats the judge runs for it. The templates are listed in Customize, Evaluations | a sealed definition |
| second opinion | The read of a winning diff by an agent from a different model provider than the proposer. It flags and never decides | a review that blocks |
| watch | The judge measuring a delivered experiment again, on a schedule or when the default branch moves | a loop |
| regression | A watch result worse than the delivered value by more than the noise | a failed trial |
| scoreboard | Trials for an experiment, by session, with cost per kept trial. The agents' figures across experiments are the agent scoreboard (see [../session/model.md](../session/model.md)) | a ranking of agents, a leaderboard |
| delivered | Yearly value of wins whose pull request is merged (glossary), from the value model: (baseline minus best) times the value per unit. A win counts as Delivered only once merged. With no value model a win shows the metric only and counts in no dollar figure. While an experiment is Regressed, the regressed amount is taken out of Delivered | locked in, realised |
| at risk | The part of Delivered that a regression has put in doubt, shown as "At risk" until a fix is Kept and merged | a loss, a failed trial |
| pending | Yearly value of wins in an open pull request; not counted in Return, and it stays Pending while the pull request is open (glossary) | unclaimed |
| Return | Delivered divided by everything spent in the scope (project, organization or time range), including experiments that found nothing, were stopped or failed, shown as `68x` (glossary). Never "ROI" | a percentage or payback figure, Delivered divided by the spend of winning experiments only |

## How the objects relate

- An experiment belongs to one project (see [../organization/model.md](../organization/model.md)), and has many sessions and many versions. It may name the evaluation template and the skill pack versions it started from. A version has one seal. The experiment owns one budget.
- A trial belongs to one experiment version, and was made by one of its sessions. It names the trial it started from: the baseline or an earlier Kept trial. That chain is the lineage.
- A trial has one measurement and one outcome. A trial that stops at the quick check has a measurement too, marked "Quick check".
- An experiment version names zero or one value model version. The estimate on the Seal experiment card (what one measurement costs, and how many trials the budget buys) comes from the seal checks, which already measured the baseline.
- A win is the best Kept trial of an experiment version. A win has one receipt and one pull request per repo changed.
- A watch belongs to a delivered experiment. Each watch result is a measurement.

## Multi-repo

A session may attach any number of repos, and an experiment may span several. A trial is a set of commits, one per repo changed. A protected path names its repo, so the seal covers each repo at a pinned commit and a trial that touches a protected path in any repo is Rejected. `deliver` opens one pull request per repo changed, linked to each other.

## What is stored, and what is derived

Stored: experiment and version ids, value model ids and versions, the quick check and patience settings, project id, seal hashes, dataset ids and versions, skill pack ids and versions read, the evaluation template id and version it started from, trial ids, commit ids per repo, the hypothesis, the outcome, measured values, repeats, noise, receipt ids and signatures, watch settings, spend rows. Never diffs, file contents, tool output or data.

Derived from measurements and spend records, never stored on their own: the best, the baseline-to-best movement, the scoreboard, cost per trial, where the spend went (Kept, Within noise, Worse, Guardrail failed, Rejected, Failed, and Not measured, agent work that never reached `measure`), an experiment's state (Not sealed, Sealed, Watching, Regressed) and its activity (from its sessions), delivered, pending, at risk and Return.

## How this area refers to the others

- Session: by session id and commit ids. The judge never names a platform's agent ids, regions or instance ids.
- Data: by dataset id and version only. An experiment never names a connector or a credential.
- Organization: by organization id, project id and member id for approvals and audit rows.
