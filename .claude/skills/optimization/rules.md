# Optimization: rules that never bend

- Apply the accept rule in code, never by a person or an agent, because a rule a person can override is not a ratchet.
- Never let a proposer see or alter the scoring, because a proposer that can read the score will fit it instead of improving the subject.
- Keep a benchmark inside its experiment, pointing only at data (context) and machines (infra), because a benchmark that reaches further breaks the boundary that keeps results comparable.
- Never change a verdict once written, because a verdict that can move is not evidence. A new finding is a new verdict on a new check.
- Record the dataset versions and the environment on every run, because a result nobody can reproduce is not a result.
- Set spend caps here and let infra enforce them, because the goal owner knows what a win is worth and infra knows what was spent.
- Count only verified moves as value, and keep pending apart from delivered, because a number that mixes them overstates Return.
- Show a failed run as failed, never as zero, because zero reads as a real measurement.
- Say "experiment" and "Return" in every word a person reads.
