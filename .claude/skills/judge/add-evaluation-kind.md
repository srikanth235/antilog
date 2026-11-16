# Add an evaluation kind

A kind is a new way to measure, such as a replay of recorded traffic, a scored set of model answers or a timed command (target). The four kinds are Speed, Quality, Size and Cost. An evaluation is a command and its protected files, so a kind is a shape of command with a known output.

## Kinds and templates

An evaluation has a **kind**, which says what is being measured, and may start from a **template**, a starter command for a common job. The agent proposes an experiment from a kind and a template. Templates are reusable and are listed in Customize, Evaluations, under Yours and Discover (boards `Evaluations`, `EvaluationsEmpty`, `EvaluationsDiscover`, `EvaluationDetail`, `AddEvaluation`). A template holds the kind, command, direction, guardrails, the standard planted cheats and an environment. A plugin may bring templates. An experiment starts from a copy, so changing or removing a template never changes a sealed experiment. The Seal experiment card shows the kind and names the standard planted cheats for it, so a person sees what the judge will try to fool.

| Kind | What it measures | Templates |
| --- | --- | --- |
| Speed | How long something takes, lower is better | Load test, Query latency, Test suite duration |
| Quality | How good an output is, higher is better | Forecast error (a scored set of answers against known ones) |
| Size | How big something is, lower is better | Bundle size |
| Cost | What something costs to run, lower is better | Your own (a command that prints a cost) |

"Your own" is the template for any command that prints one JSON object with the metric. Each template names its own planted cheats beside the standard three (skip the work, hard-code the answer, return early): for example a load test that serves cached responses, a scorer fed the expected answers, or a size check that drops the files it measures. A new kind or template follows the steps below.

## Steps

1. **A name and a one-line meaning.** The kind has a word in [the glossary](../../../docs/GLOSSARY.md) and in [model.md](model.md).
2. **A pure output format.** The command prints one JSON object with the metric as a dotted path, plus any guardrail metrics. An agent asking for a measurement gets the measured value, guardrail values and noise band back, never the holdout value. The kind may bring up what it needs inside the judge sandbox and removes it after.
3. **Data by reference only.** The evaluation names datasets by id and version. It never holds a credential or a connection string.
4. **Protected paths.** The kind lists what an agent may never change: the evaluation command, the scoring and the holdout. Each path names its repo. A trial that touches one is Rejected before it is measured.
5. **Runnable with no agent.** The kind must run as a command in a judge sandbox on the judge platform (see [anti-cheat.md](anti-cheat.md)). It needs no network route except to the experiment's data.
6. **A holdout.** The kind says what its holdout is and how it is marked "Restricted to the judge". A kind with no way to hold data back cannot confirm a win and is not accepted.
7. **Noise and the quick check.** The kind defines repeats and how the noise is derived from the baseline and the same-commit measurement. It says what one repeat is, because the quick check runs exactly one, and what one measurement costs, because the estimate on the Seal experiment card comes from the seal checks. The acceptance criteria are not changed by a new kind.
8. **Standard planted cheats.** At least: skip the work, hard-code the answer, return early. Add the ones that fit the kind, for example a replay that serves cached responses, or a scorer fed the expected answers. Each must be rejected by the evaluation, or the seal check fails.
9. **Where a win lands.** A win reaches the world as one pull request per repo changed, through `deliver`. Anything else that writes outside Ratchet is a card a person approves (see [../organization/approvals.md](../organization/approvals.md)).
10. **A template.** A starter command for the kind that a proposing agent may copy, listed above, with the planted cheats the Seal experiment card will show.
11. **Tests.** A golden test of the template, a parse test of the output, each planted cheat rejected, and one end-to-end measurement with a scripted agent.
12. **Receipt and value.** Say which receipt fields the kind produces (see [owes.md](owes.md)) and which metric a project owner may attach a value model to, with its unit (for example ms). A kind never sets a value model.
13. **Customize.** Add the template to the Evaluations list, with its detail page and an Add evaluation path for a person's own. Adding, changing and removing one are audited actions (see [../organization/add-audited-action.md](../organization/add-audited-action.md)).

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).
