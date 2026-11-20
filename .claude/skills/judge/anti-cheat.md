# Judge: the anti-cheat

Six mechanisms built into the judge (target). An agent that is rewarded for a number will find the cheapest way to move it, so each mechanism says what it stops and what it does not.

## 1. Sealed experiment

The seal is a hash over the experiment's definition and its protected files at pinned commits. For every trial the judge takes the protected files from the seal, never from the trial. A trial that changes a protected path, in any repo, gets the outcome Rejected.

- Stops: changing the evaluation, the scoring or the data it reads.
- Does not stop: a change to unprotected code that fools the evaluation. A protected path list that is too short.

## 2. The number comes from the sealed command

The test runs inside the session's own machine ([docs/V0.md](../../../docs/V0.md)). The runner checks each commit out in a `git worktree` beside the clone, restores the protected files by hash from the repo's history, runs the sealed test command and returns its exit status and output. The judge takes the number from that command, never from the agent's report. A skill pack is advice to agents and never an input to the judge.

- Stops: tampering with the measurement through the agent's own report.
- Does not stop: an agent that is still running in the same machine. Returning only named numbers narrows a leak through the numbers and does not remove it.

## 3. Seal checks

Before an experiment can be sealed the judge measures the baseline, pairs the same commit with itself to get the noise of a paired difference (the noise check), and runs planted cheats the evaluation must reject (the cheat checks): skip the work, hard-code the answer, return early, and behave differently on the holdout (the holdout is shown at the same path and shape as the evaluation data, so this cheat must fail). There is a standard set per kind of evaluation (see [add-evaluation-kind.md](add-evaluation-kind.md)) plus any the proposing agent adds. The seal card shows the results. An experiment that fails a check cannot be sealed.

- Stops: an evaluation that is trivially broken, or so noisy a win means nothing.
- Does not stop: a bad evaluation that passes the planted cheats. Seal checks catch a trivially broken evaluation and cannot prove a good one. A person still reads the card.

## 4. Confirm before Kept

Every measurement is paired: the trial and the trial it started from run interleaved for the sealed number of repeats, so drift cancels. A trial beats the bar when the one-sided lower confidence bound of its mean paired gain is above zero at confidence 1 − 0.05/k, where k counts the trials measured in this experiment version. It is then measured again, paired, on a fresh checkout and checked on the holdout, in a fresh container of the same environment version that only that container's run receives the holdout data in. The agent's live machine never holds it. The bar rises with k, so many attempts cannot win by luck.

The quick check that precedes this (one pair against the best) only stops weak trials early to save cost. It never decides Kept, runs only in the judge, and is sealed with the experiment, so a trial cannot skip or fake it.

- Stops: a lucky measurement, and fitting to the evaluation data.
When patience is reached, the judge measures each session's branch head that differs from every measured trial as one cumulative trial against the best, so real gains each below the bar can add up. It counts in k and meets the same bar.

The holdout answers one bit per trial, so it is an oracle an agent could probe. The holdout credential is read-only, goes only to that fresh container, and holdout checks are capped per experiment version (sealed, default 20); at the cap a new version with a new holdout is needed.

- Does not stop: a holdout that is too small or too like the evaluation data, or probing below the cap.

## 5. Second opinion

Before a win is delivered, an agent from a different model provider than the proposer reads the diff and says whether it special-cases the evaluation. Its notes go on the receipt.

- Stops: a cheat a person would not spot in a large diff.
- Does not stop anything by itself. It flags and never decides, and it can be wrong in both directions.

## 6. Signed receipt

Its fields are in [model.md](model.md). It is signed by the judge, and Verify again checks the signature, the signer and the image digest.

- Stops: a win nobody can check later, and numbers edited after the fact.
- Does not stop: a cheat the other five mechanisms missed. It makes one findable.
