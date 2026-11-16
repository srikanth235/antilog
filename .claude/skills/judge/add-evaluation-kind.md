# Add an evaluation kind

A kind is a new way to measure, such as a replay of recorded traffic, a scored set of model answers or a timed command (target). An evaluation is a command and its protected files, so a kind is a shape of command with a known output.

1. **A name and a one-line meaning.** The kind has a word in [the glossary](../../../docs/GLOSSARY.md) and in [model.md](model.md).
2. **A pure output contract.** The command prints one JSON object with the metric as a dotted path, plus any guardrail metrics. An agent asking for a measurement gets the measured value, guardrail values and noise band back, never the holdout value. The kind may bring up what it needs inside the judge sandbox and removes it after.
3. **Data by reference only.** The evaluation names datasets by id and version. It never holds a credential or a connection string.
4. **Protected paths.** The kind lists what an agent may never change: the evaluation command, the scoring and the holdout. Each path names its repo. A trial that touches one is Rejected before it is measured.
5. **Runnable with no agent.** The kind must run as a command in a judge sandbox on the judge platform (see [anti-cheat.md](anti-cheat.md)). It needs no network route except to the contract's data.
6. **A holdout.** The kind says what its holdout is and how it is marked "Restricted to the judge". A kind with no way to hold data back cannot confirm a win and is not accepted.
7. **Noise.** The kind defines repeats and how the noise is derived from the baseline and the same-commit measurement. The acceptance criteria are not changed by a new kind.
8. **Standard planted cheats.** At least: skip the work, hard-code the answer, return early. Add the ones that fit the kind, for example a replay that serves cached responses, or a scorer fed the expected answers. Each must be rejected by the evaluation, or the seal check fails.
9. **Where a win lands.** A win reaches the world as one pull request per repo changed, through `deliver`. Anything else that writes outside Ratchet is a card a person approves (see [../organization/approvals.md](../organization/approvals.md)).
10. **A starter.** A starter contract for the kind that a proposing agent may copy.
11. **Tests.** A golden test of the starter, a parse test of the output, each planted cheat rejected, and one end-to-end measurement with a scripted agent.
12. **Receipt.** Say which receipt fields the kind produces (see [owes.md](owes.md)).

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).
