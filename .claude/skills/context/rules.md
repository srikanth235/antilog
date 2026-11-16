# Context: rules that never bend

- Let a holdout-marked dataset be granted to the evaluator only, and enforce it in the credential each connector makes plus the one place that checks grants, because a proposer that reads the holdout fits it.
- Never give an agent a long-lived, write or holdout credential, and never a connector admin secret. Give it a read-only, scoped credential that excludes the holdout, through the platform's credential store only, because a credential that can write or can see the holdout is one a proposer can leak or fit. Whether credentials are minted per run or registered once is open (see [rulings](../product/rulings.md)).
- Give the evaluator its own credential that includes the holdout, because the holdout is enforced by credentials and the agent's must never be able to reach it.
- Keep any connector admin secret out of the Ratchet server on Ratchet cloud and out of every response, event and log, because the server must hold no access to customer data. Whether one exists at all is open (see [rulings](../product/rulings.md)).
- Never change a dataset version after it is created, because a run that names a version must be reproducible.
- Treat the code host as a connector here, and a pull request as a write-back action requested by optimization, because writing out must be explicit and audited.
- Give each outside system exactly one translator and stop its vocabulary there, because a second translator forks the meaning of a column.
- Build no transforms or pipelines. The customer prepares data in their own warehouse, because a transform layer makes Ratchet responsible for data quality it cannot see.
- Never connect the Ratchet server to data and never route dataset rows through it, because data is read only inside platform sandboxes. The agent's code reads through connectors.
- Keep agent credentials read-only, because a read path that can write is not a read path.
- Never let the evaluator read a skill pack, and never put holdout data in one, because a pack agents read must not carry what only the evaluator may see.
- Pin the skill pack versions an experiment read, and record them on the receipt, because a run that names a version must be reproducible.
- Let a plugin add skill packs and connectors but never mark, unmark or grant a dataset, because grants and markings are enforced in one place.
- Store every result, failures included, in history, and let every agent in the run read it live, because a history of only successes hides what was tried and an agent that cannot see others' trials repeats them.
- Never put a holdout value in history, a note or a trial result an agent reads, because the history is what agents read.
