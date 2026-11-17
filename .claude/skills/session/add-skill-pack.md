# Add a skill pack

A skill pack is a versioned pack of written know-how that agents read before they propose changes, for example "Postgres query tuning". On screen it is a Skill, under Customize, Skills. It is not a file in this repo's `.claude/skills/`, which guides people and agents working on Antilog's code, so docs and code say "skill pack" where the two could be confused. All steps are target.

A skill pack is added from Customize, Skills (boards `Skills`, `SkillsEmpty`, `SkillsDiscover`, `SkillDetail`, `AddSkill`), or arrives with a plugin. What a card in a chat adds appears there too.

1. **Written text only.** A pack is documents. It holds no code that runs on the server and no credential.
2. **A version on every change.** Editing a pack makes a new immutable version. An experiment names the versions it read.
3. **Agents are the only audience.** The judge never reads a pack. A pack is never a grant on a dataset.
4. **No holdout data.** Refuse a pack, at upload and at each new version, that contains or links to a holdout-marked dataset.
5. **Chosen per experiment.** A person or the agent selects the packs an experiment uses, in the composer's session brief or from the experiment page. A project can turn a pack On or Off for its experiments: Off hides the pack from New experiment in that project, and an experiment that already read a version keeps it on its record.
6. **Told to the agent.** The packs an experiment selected are part of what its agents are told (see [prompts.md](prompts.md)). Each is read as chat context, never as an instruction that changes what counts.
7. **Record the read.** The receipt's Data column lists the skill pack versions the agents read, next to the dataset versions (see [../judge/owes.md](../judge/owes.md)).
8. **A plugin brings it, governance stays here.** A plugin may add a pack. The pack is governed like any other. Removing the plugin asks whether to keep the pack, and a pack in use by an experiment is never removed. A plugin can never mark, unmark or grant a dataset.
9. **Audit.** Adding, changing, turning On or Off and removing a pack are named actions with an actor (see [../organization/add-audited-action.md](../organization/add-audited-action.md)).
10. **Words.** Add the word to [model.md](model.md) and check its row in [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md).
11. **Tests.** The judge identity cannot read a pack. A pack with holdout content is refused. A receipt lists a version for every pack read. A pack in use cannot be removed. A pack never changes an outcome.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).
