# Context: objects and words

`docs/GLOSSARY.md` defines "Note" and "Connector". This file keeps the glossary's Note. In the target, "connector" is the connection to an outside system, on screen and in code. Today's "connector" is a signed-in account (GitHub, AWS, an agent login) in the earlier code. Do not say "source" for either.

| Object | Means | Not |
| --- | --- | --- |
| connector | A connection to an outside system that the agent's code reads through: Snowflake, Databricks, DynamoDB, BigQuery, Postgres, S3, GitHub, GitLab, any MCP server. It knows how to mint a scoped credential for its system | a dataset, a credential, a pipe that Ratchet reads through, "source" (the old word) |
| scoped credential | A short-lived, read-only credential for one connector, minted per run by the launcher. The agent's has the holdout left out; the evaluator's is separate and includes it | a stored secret, the connector admin secret |
| connector admin secret | The secret the launcher uses to mint scoped credentials. It stays in the customer's secret manager, or, for Ratchet-managed, in our encrypted secret store held by the managed runner. No agent or server ever holds it | a scoped credential |
| dataset | A named, governed table or file set that experiments read. Ratchet records its id and versions, never its rows | a connector, a query |
| dataset version | One immutable state of a dataset, with an id | latest, a snapshot someone can edit |
| virtual dataset | A dataset that points at data left in place, pinned by an as-of time | a copy |
| grant | A rule that says which audience gets a scoped credential that reaches a dataset | a credential |
| audience | Who a grant is for: agents, or the evaluator | a person, a role (see organization) |
| marking | A label on a dataset that limits its grants, such as holdout | a tag for search |
| holdout | A marking: data only the evaluator may read. Enforced by credentials: the agent's scoped credential is a read-only role, view or policy that excludes it | a validation split a proposer can see |
| skill pack | A versioned pack of written know-how that agents read before proposing changes, for example "Postgres query tuning". Screen word: Skill. Audience: agents only | a dataset, an agent instruction file in `.claude/skills/`, a plugin |
| note | Text that steers the agents of a run: to all of them, or to one with @ ("@Codex focus on database queries"). Each agent reads it before its next trial (glossary) | comment, prompt |
| history | Every trial and its result, failures included, stored as a dataset. Every agent in a run reads it live, other agents' trials and the experiment's best trial with its diff included, and later runs read it too (target) | a log |
| write-back action | A separate, explicit, audited action that writes to an outside system, such as opening a pull request; requested by an experiment | a connector feature |

## History and notes (target)

An agent reads the history before each trial: every trial by every agent in this run and in earlier runs, with measured values, outcomes and diffs. It may start from the experiment's best trial. The history carries measured values on the evaluation data and never the holdout value. A note from a person goes to all agents or to one with @, and is read before the agent's next trial.

## Skill pack

A skill pack is not the agent guidance in this repo's `.claude/skills/`; that is for people and agents working on Ratchet's code. A skill pack is a product object. It has a version, it is chosen per experiment and it can be turned On or Off per project. The experiment records which skill versions were read. A plugin may bring skill packs, but a pack is governed here, whatever added it.

## Connector kinds

Snowflake, Databricks, DynamoDB, BigQuery, Postgres, S3, GitHub, GitLab, any MCP server. Each has one translator and one way to mint a scoped, read-only, holdout-excluding credential. None is built as a connector today.

## How data is read

The coding agent writes code, and that code runs the experiment and reads data through the connectors, using the scoped credential the launcher minted for the run. The Ratchet server never connects to data and stores none. There is no context plane that pulls or brokers reads. Model keys are not connector credentials: they go through the egress proxy (see [../infra/model.md](../infra/model.md)). Git access is a short-lived scoped token.

## How this area refers to the others

- Optimization: by experiment id and run id only. A write-back action carries the experiment's id and the win's id. Context never reads the accept rule or a verdict.
- Infra: by environment id only. The launcher mints the credentials and runs the code on a named environment. Regions, accounts and instance ids stay in infra.
- Organization: by project id and actor id. Grants belong to a project.
