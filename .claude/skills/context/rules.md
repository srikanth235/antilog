# Context: rules that never bend

- Let a holdout-marked dataset be granted to the evaluator only, and enforce it in the credential each connector mints plus the one place that checks grants, because a proposer that reads the holdout fits it.
- Never give an agent a long-lived, write or holdout credential, and never a connector admin secret. Give it a short-lived, read-only, scoped credential minted per run, because a credential that outlives its run, can write or can see the holdout is one a proposer can leak or fit.
- Give the evaluator its own credential that includes the holdout, because the holdout is enforced by credentials and the agent's must never be able to reach it.
- Keep connector admin secrets in the customer's infrastructure, in their secret manager, used only by the launcher to mint scoped credentials, because the server must hold no access to customer data.
- Never change a dataset version after it is created, because a run that names a version must be reproducible.
- Treat the code host as a connector here, and a pull request as a write-back action requested by optimization, because writing out must be explicit and audited.
- Give each outside system exactly one translator and stop its vocabulary there, because a second translator forks the meaning of a column.
- Build no transforms or pipelines. The customer prepares data in their own warehouse, because a transform layer makes Ratchet responsible for data quality it cannot see.
- Never connect the Ratchet server to data and never route dataset rows through it, because data stays in the customer's infrastructure. The agent's code reads through connectors.
- Keep scoped credentials read-only, because a read path that can write is not a read path.
- Never let the evaluator read a skill pack, and never put holdout data in one, because a pack agents read must not carry what only the evaluator may see.
- Pin the skill pack versions an experiment read, and record them on the receipt, because a run that names a version must be reproducible.
- Let a plugin add skill packs and connectors but never mark, unmark or grant a dataset, because grants and markings are enforced in one place.
- Store every result, failures included, in history, because a history of only successes hides what was tried.
