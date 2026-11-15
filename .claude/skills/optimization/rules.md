# Optimization: rules that never bend

- Apply the accept rule in code, never by a person or an agent, because a rule a person can override is not a ratchet.
- Never let a proposer alter the evaluation or the evaluator, or read the holdout, because a proposer that can change the score or see the held-back data will fit it instead of improving the subject. It does see each trial's measured value on the evaluation data, so it can iterate.
- Count only the evaluator's numbers. Nothing measured in an agent sandbox counts, because an agent's own number is not a trusted measurement.
- Keep a benchmark inside its experiment, pointing only at data (context) and machines (infra), because a benchmark that reaches further breaks the boundary that keeps results comparable.
- Never change a verdict once written, because a verdict that can move is not evidence. A new finding is a new verdict on a new check.
- Record the dataset versions and the environment on every run, because a result nobody can reproduce is not a result.
- Set spend caps here and let infra enforce them, because the goal owner knows what a win is worth and infra knows what was spent.
- Count only verified moves as value, and keep pending apart from delivered, because a number that mixes them overstates Return.
- Show a failed trial or run as failed, never as zero, because zero reads as a real measurement.
- Say "experiment" and "Return" in every word a person reads.
