# Judge: rules that never bend

- Count only the judge's numbers, because a number made in an agent's session is not a trusted measurement. It is never shown as a result.
- Never give an agent the holdout credential, because an agent that can see the held-back data will fit it. Only the judge sandbox holds it. An agent never learns a holdout value, only Kept or Failed holdout.
- Never change a sealed experiment, because a definition that can move is not a benchmark. A new version starts a new record, and numbers never carry across versions.
- Never let a person or an agent mark a win, because a rule a person can override is not a ratchet. Only the acceptance criteria can, and they are one pure function in code.
- Let chat change what an agent tries and never what counts, because the words in a conversation are not evidence.
- Take protected files from the seal for every trial, never from the trial, because a trial that carries its own evaluation grades itself.
- Let an agent read the evaluation's code and never change it, because reading helps it iterate and changing it changes the score.
- Never change an outcome once written, because an outcome that can move is not evidence. A new finding is a new measurement.
- Pin the experiment version, the seal and the dataset versions on every measurement, because a result nobody can reproduce is not a result.
- Refuse `measure` when the budget is spent, and let the session layer meter it, because the experiment's owner knows what a win is worth and the budget knows what was spent.
- Count only Kept wins as value, and keep pending apart from delivered, because a number that mixes them overstates Return.
- Show a failed trial as failed, never as zero or empty, and never show an unchecked experiment as healthy, because zero reads as a real measurement.
- Never let the second opinion decide, because it flags and a person reads it.
- Say "experiment", "trial" and "Return" in every word a person reads. Never "contract", "run" as a noun or "ROI".
