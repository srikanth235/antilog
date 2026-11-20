# Add a skill pack

A skill pack is a versioned folder of written know-how that an agent loads on demand, for example "Postgres query tuning". On screen it is a Skill, under Customize, Skills. It is not a file in this repo's `.claude/skills/`, which guides people and agents working on Antilog's code, so docs and code say "skill pack" where the two could be confused. Built: `antilog-core/src/customize/skills.rs`, the routes in `antilog-server/src/api/customize.rs`.

A skill pack is added from Customize, Skills (boards `Skills`, `SkillsEmpty`, `SkillDetail`, `AddSkill`), or arrives with a plugin.

1. **A `SKILL.md` folder, text only.** `SKILL.md` at the top starts with front matter holding `name` (lower case letters, digits, hyphens) and `description`. Every file is text: at most 100 files, 256 KB each, 1 MB in all, relative paths with no `..`, no NUL, no private key. A pack holds no code the server runs and no credential. Upload one `SKILL.md` or a set of text files. Zip upload comes after approval of a zip crate.
2. **A version on every upload.** Uploading a name that exists makes the next immutable version. An experiment names the versions it selected; a new version never moves a selection.
3. **Agents are the only audience.** The judge never reads a pack. A pack never changes what counts.
4. **Chosen per experiment, On or Off per project.** An editor or owner of the project selects the packs an experiment uses; each is pinned to its latest version at that moment. A project owner turns a pack Off to hide it from New experiment in that project, and an experiment that already selected a version keeps it. A pack an experiment selected is never removed.
5. **Loaded on demand, not told.** The runner downloads each selected version from `GET /runner/sessions/{id}/skills/{name}/{version}` (the handler is `api::customize::session_skill`, which serves only what the session's experiment selected) and unpacks it into `/opt/antilog/skills/<name>/`; OpenCode lists them and loads one when its description fits ([../../../docs/V0-contracts.md](../../../docs/V0-contracts.md)). The brief does not carry their text ([prompts.md](prompts.md)).
6. **Record the read.** The receipt's Data section lists the skill pack versions the experiment selected (`skillPacks`), next to the commits.
7. **A plugin brings it, governance stays here.** A pack from a plugin is governed like any other. Removing the plugin asks whether to keep each pack, and a pack in use is never removed.
8. **Audit.** Adding a version, removing, turning On or Off and an experiment's selection are named actions with an actor.
9. **Words.** [model.md](model.md) and [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md).
10. **Tests.** A bad pack is refused (no `SKILL.md`, bad name, path out of the folder, binary, too large, a key); versions are immutable; Off hides a pack; a selected pack cannot be removed; the runner route serves only selected versions; a receipt lists the versions.

Not built yet: refusing a pack that links to a holdout-marked dataset (there is no dataset registry in v0).

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).
