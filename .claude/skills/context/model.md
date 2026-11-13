# Context: objects and words

`docs/GLOSSARY.md` defines "Note" and "Connectors". This file keeps the glossary's Note. The glossary's "connector" means a signed-in account today (GitHub, AWS, an agent login). The target word for a connection to a data system is "source" (on screen: "connector"). Say which one you mean.

| Object | Means | Not |
| --- | --- | --- |
| source | A read-only connection to an outside system: Snowflake, BigQuery, Postgres, object storage, GitHub, GitLab, monitoring, any MCP server | a dataset, a credential |
| dataset | A named, governed table or file set that experiments read | a source, a query |
| dataset version | One immutable state of a dataset, with an id | latest, a snapshot someone can edit |
| virtual dataset | A dataset that points at data left in place, pinned by an as-of time | a copy |
| grant | A rule that lets an audience read a dataset | a credential |
| audience | Who a grant is for: agents, or the evaluator | a person, a role (see organization) |
| marking | A label on a dataset that limits its grants, such as holdout | a tag for search |
| holdout | A marking: data only the evaluator may read | a validation split a proposer can see |
| skill pack | A versioned pack of written know-how that agents read before proposing changes, for example "Postgres query tuning". Screen word: Skill. Audience: agents only | a dataset, an agent instruction file in `.claude/skills/`, a plugin |
| note | Text left on a run to steer the next candidate (glossary) | comment, prompt |
| history | Every past attempt and its result, failures included, stored as a dataset | a log |
| write-back action | A separate, explicit, audited action that writes to an outside system, such as opening a pull request; requested by an optimization | a source feature |

## Skill pack

A skill pack is not the agent guidance in this repo's `.claude/skills/`; that is for people and agents working on Ratchet's code. A skill pack is a product object. It has a version, it is chosen per experiment and it can be turned On or Off per project. The experiment records which skill versions were read. A plugin may bring skill packs, but a pack is governed here, whatever added it.

## Source kinds

Snowflake, BigQuery, Postgres, object storage, GitHub, GitLab, monitoring, any MCP server. Each has one translator. None is built as a source today.

## How this area refers to the others

- Optimization: by optimization id and run id only. A write-back action carries the optimization's id and the win's id. Context never reads the accept rule or a verdict.
- Infra: by environment id only. A read happens on a named environment. Regions, accounts and instance ids stay in infra.
- Organization: by project id and actor id. Grants belong to a project.
