# Add a skill pack

A skill pack is a versioned folder of written know-how that an agent loads on demand, for example "Postgres query tuning". On screen it is a Skill, under Customize, Skills. It is not a file in this repo's `.claude/skills/`, which guides people and agents working on Antilog's code, so docs and code say "skill pack" where the two could be confused. Built: `antilog-core/src/customize/skills.rs`, the routes in `antilog-server/src/api/customize.rs`.

A skill pack is added from Customize, Skills (boards `Skills`, `SkillsEmpty`, `SkillDetail`, `AddSkill`), or arrives with a plugin.

1. **A `SKILL.md` folder, text only.** `SKILL.md` at the top starts with front matter holding `name` (lower case letters, digits, hyphens) and `description`. Every file is text: at most 100 files, 256 KB each, 1 MB in all, relative paths with no `..`, no NUL, no private key. A pack holds no code the server runs and no credential. Upload one `SKILL.md`, or a folder: the screen reads its text files and sends them as a list (no zip in v0).
2. **A version on every upload.** Uploading a name that exists makes the next immutable version.
3. **Agents are the only audience.** A pack is advice and changes nothing the server decides.
4. **Every session gets the latest.** At claim the runner takes the latest version of every organization skill; there is no per-session selection and no project On or Off switch. Removing a pack is not blocked by use.
5. **Loaded on demand, not told.** The runner downloads each version from `GET /runner/sessions/{id}/skills/{name}/{version}` (the handler is `api::customize::session_skill`, which serves any version of the session's organization's skills) and unpacks it into `/opt/antilog/skills/<name>/`; OpenCode lists them and loads one when its description fits ([../../../docs/V0-contracts.md](../../../docs/V0-contracts.md)). The prompts do not carry their text ([prompts.md](prompts.md)).
6. **A plugin brings it, governance stays here.** A pack from a plugin is governed like any other. Removing the plugin asks whether to keep each pack.
7. **Audit.** Adding a version and removing are named actions with an actor (`skill.add`, `skill.remove`).
8. **Words.** [model.md](model.md) and [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md).
9. **Tests.** A bad pack is refused (no `SKILL.md`, bad name, path out of the folder, binary, too large, a key); versions are immutable; a claim lists the latest version of every skill; the runner route serves only the session's organization's packs.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).
