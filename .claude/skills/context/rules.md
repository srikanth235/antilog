# Context: rules that never bend

- Let a holdout-marked dataset be granted to the evaluator only, and enforce that in one place, because two places drift and a proposer that reads the holdout fits it.
- Never give an agent a source credential. Give it short-lived, per-run read access to granted datasets, because a credential outlives the run that needed it.
- Never change a dataset version after it is created, because a run that names a version must be reproducible.
- Treat the code host as a source here, and a pull request as a write-back action requested by optimization, because writing out must be explicit and audited.
- Give each outside system exactly one translator and stop its vocabulary there, because a second translator forks the meaning of a column.
- Build no transforms or pipelines. The customer prepares data in their own warehouse, because a transform layer makes Ratchet responsible for data quality it cannot see.
- Never route dataset rows through the Ratchet server, because data stays in the customer's cloud.
- Keep sources read-only, because a read path that can write is not a read path.
- Never let the evaluator read a skill pack, and never put holdout data in one, because a pack agents read must not carry what only the evaluator may see.
- Pin the skill pack versions an experiment read, and record them on the receipt, because a run that names a version must be reproducible.
- Let a plugin add skill packs and connectors but never mark, unmark or grant a dataset, because grants and markings are enforced in one place.
- Store every result, failures included, in history, because a history of only successes hides what was tried.
