# Judge: the anti-cheat

> v0: the test runs inside the session's own machine and the judge takes the number from the sealed test command, not from the agent's report; there is no separate judge sandbox. Where this page says "judge sandbox", read [docs/V0.md](../../../docs/V0.md).

Six mechanisms built into the judge (target). An agent that is rewarded for a number will find the cheapest way to move it, so each mechanism says what it stops and what it does not.

## 1. Sealed experiment

The seal is a hash over the experiment's definition and its protected files at pinned commits. For every trial the judge takes the protected files from the seal, never from the trial. A trial that changes a protected path, in any repo, gets the outcome Rejected.

- Stops: changing the evaluation, the scoring or the data it reads.
- Does not stop: a change to unprotected code that fools the evaluation. A protected path list that is too short.

## 2. Separate sandbox

Each measurement runs in a fresh judge sandbox. The rules:

- No agent is in it.
- It holds the judge's credential, which alone reaches the holdout. The judge sandbox is the only holder.
- It has no network route except to the experiment's data.
- It returns only the numbers the experiment names.
- It reads no skill pack: a pack is advice to agents and never an input to the judge.
- It is removed afterwards.


- Stops: an agent reading the holdout, tampering with the measurement, or leaking data out.
- Does not stop: a leak through the numbers themselves. Returning only named numbers narrows it and does not remove it.

## 3. Seal checks

Before an experiment can be sealed the judge measures the baseline, pairs the same commit with itself to get the noise of a paired difference (the noise check), and runs planted cheats the evaluation must reject (the cheat checks): skip the work, hard-code the answer, return early, and behave differently on the holdout (the holdout is shown at the same path and shape as the evaluation data, so this cheat must fail). There is a standard set per kind of evaluation (see [add-evaluation-kind.md](add-evaluation-kind.md)) plus any the proposing agent adds. The seal card shows the results. An experiment that fails a check cannot be sealed.

- Stops: an evaluation that is trivially broken, or so noisy a win means nothing.
- Does not stop: a bad evaluation that passes the planted cheats. Seal checks catch a trivially broken evaluation and cannot prove a good one. A person still reads the card.

## 4. Confirm before Kept

Every measurement is paired: in one fresh judge sandbox the trial and the trial it started from run interleaved for the sealed number of repeats, so drift between sandboxes cancels. A trial beats the bar when the one-sided lower confidence bound of its mean paired gain is above zero at confidence 1 − 0.05/k, where k counts the trials measured in this experiment version. It is then measured again, paired, in another fresh judge sandbox and checked on the holdout. The bar rises with k, so many attempts cannot win by luck.

The quick check that precedes this (one pair against the best) only stops weak trials early to save cost. It never decides Kept, runs only in the judge, and is sealed with the experiment, so a trial cannot skip or fake it.

- Stops: a lucky measurement, and fitting to the evaluation data.
When patience is reached, the judge measures each session's branch head that differs from every measured trial as one cumulative trial against the best, so real gains each below the bar can add up. It counts in k and meets the same bar.

The holdout answers one bit per trial, so it is an oracle an agent could probe. The judge credential is read-only and holdout checks are capped per experiment version (sealed, default 20); at the cap a new version with a new holdout is needed.

- Does not stop: a holdout that is too small or too like the evaluation data, or probing below the cap.

## 5. Second opinion

Before a win is delivered, an agent from a different model provider than the proposer reads the diff and says whether it special-cases the evaluation. Its notes go on the receipt.

- Stops: a cheat a person would not spot in a large diff.
- Does not stop anything by itself. It flags and never decides, and it can be wrong in both directions.

## 6. Signed receipt

Its fields are in [model.md](model.md). It is signed by the judge, and Verify again checks the signature, the signer and the image digest.

- Stops: a win nobody can check later, and numbers edited after the fact.
- Does not stop: a cheat the other five mechanisms missed. It makes one findable.
