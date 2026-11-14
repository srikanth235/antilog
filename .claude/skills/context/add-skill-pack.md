# Add a skill pack

A skill pack is a versioned pack of written know-how that agents read before proposing changes, for example "Postgres query tuning". Screen word: Skill. It is not a file in this repo's `.claude/skills/`, which guides people and agents working on Ratchet's code. Nothing like it is built today (see [today.md](today.md)). This is the shape for adding one, or for a plugin that brings one.

1. **Written text only.** A pack is documents. It holds no code that runs on the server and no credential. (target)
2. **A version on every change.** Editing a pack makes a new immutable version. An experiment names the versions it read. (target)
3. **Agents are the only audience.** The evaluator never reads a pack. A pack is never a grant on a dataset. (target)
4. **No holdout data.** Refuse a pack, at upload and at each new version, that contains or links to a holdout-marked dataset. (target)
5. **Chosen per experiment, On or Off per project.** Off in a project hides the pack from New experiment there. An experiment that already read a version keeps it on its record. (target)
6. **Record the read.** The result's lineage, in the Data column of the three-column lineage, lists the pack versions the agents read, next to the dataset versions. (target)
7. **A plugin brings it, governance stays here.** A plugin may add a pack. The pack is governed by context like any other. Removing the plugin asks whether to keep the pack, and a pack in use by an experiment is never removed. (target)
8. **Audit.** Adding, changing, turning On or Off and removing a pack are named actions with an actor (see [../organization/SKILL.md](../organization/SKILL.md)). (target)
9. **Words.** Add the word to [model.md](model.md) and check the screen word in the "On screen" column of [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md). Say "skill pack" in code and docs where "skill" could mean the repo's guidance files.
10. **Tests.** An evaluator identity cannot read a pack. A pack with holdout content is refused. A run record lists a version for every pack read. A pack in use cannot be removed.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).
