//! Skills (skill packs): organization `SKILL.md` folders with fixed versions. Text only, read by
//! agents and never by the judge.

use antilog_types::{
    AuditOutcome, MemberId, NewSkill, Skill, SkillFile, SkillVersion,
};

use crate::organization::{Action, Principal, Refusal, entry, member_of, permit};
use crate::store::{Store, StoreError};

pub const MAX_FILES: usize = 100;
pub const MAX_FILE_BYTES: usize = 256 * 1024;
pub const MAX_TOTAL_BYTES: usize = 1024 * 1024;

fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 200
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

/// `name` and `description` from the front matter of a `SKILL.md`.
fn front_matter(text: &str) -> Option<(String, String)> {
    let body = text.strip_prefix("---")?.trim_start_matches(['\r', '\n']);
    let (head, _) = body.split_once("\n---")?;
    let field = |key: &str| {
        head.lines().find_map(|line| {
            let value = line.strip_prefix(key)?.strip_prefix(':')?;
            Some(value.trim().trim_matches(['"', '\'']).to_owned())
        })
    };
    Some((field("name")?, field("description")?))
}

/// Checks a pack and returns its name, description and size.
pub fn check(files: &[SkillFile]) -> Result<(String, String, u64), Refusal> {
    if files.is_empty() || files.len() > MAX_FILES {
        return Err(Refusal::Invalid("a skill is 1 to 100 text files"));
    }
    let mut total = 0usize;
    for (i, file) in files.iter().enumerate() {
        if !valid_path(&file.path) {
            return Err(Refusal::Invalid(
                "a file path is relative, inside the skill's folder, with no .. parts",
            ));
        }
        if files.iter().take(i).any(|other| other.path == file.path) {
            return Err(Refusal::Invalid("two files have the same path"));
        }
        if file.content.len() > MAX_FILE_BYTES || file.content.contains('\0') {
            return Err(Refusal::Invalid(
                "a skill holds text files of at most 256 KB each, nothing binary",
            ));
        }
        if file.content.contains("PRIVATE KEY-----") {
            return Err(Refusal::Invalid(
                "a skill holds no credential: remove the private key",
            ));
        }
        total += file.content.len();
    }
    if total > MAX_TOTAL_BYTES {
        return Err(Refusal::Invalid("a skill is at most 1 MB in all"));
    }
    let main = files
        .iter()
        .find(|file| file.path == "SKILL.md")
        .ok_or(Refusal::Invalid("a skill needs a SKILL.md at the top"))?;
    let (name, description) = front_matter(&main.content).ok_or(Refusal::Invalid(
        "SKILL.md starts with front matter holding a name and a description",
    ))?;
    if !valid_name(&name) {
        return Err(Refusal::Invalid(
            "the skill's name is lower case letters, digits and hyphens, up to 64",
        ));
    }
    if description.is_empty() || description.chars().count() > 1024 {
        return Err(Refusal::Invalid("the description is 1 to 1024 characters"));
    }
    Ok((name, description, u64::try_from(total).unwrap_or(u64::MAX)))
}

/// Every skill of the organization, with its versions.
pub async fn list<S: Store>(store: &S, who: &Principal) -> Result<Vec<Skill>, Refusal> {
    member_of(store, who).await?;
    Ok(store.skills(who.organization).await?)
}

pub async fn version<S: Store>(
    store: &S,
    who: &Principal,
    name: &str,
    version: u32,
) -> Result<SkillVersion, Refusal> {
    member_of(store, who).await?;
    store
        .skill_version(who.organization, name, version)
        .await?
        .ok_or(Refusal::NotFound)
}

pub(crate) async fn next_version<S: Store>(
    store: &S,
    organization: antilog_types::OrganizationId,
    name: &str,
) -> Result<u32, Refusal> {
    Ok(store
        .skills(organization)
        .await?
        .iter()
        .find(|skill| skill.name == name)
        .map_or(1, |skill| skill.latest + 1))
}

pub(crate) fn build(
    files: Vec<SkillFile>,
    version: u32,
    by: MemberId,
    now: i64,
) -> Result<SkillVersion, Refusal> {
    let (name, description, size_bytes) = check(&files)?;
    Ok(SkillVersion {
        name,
        version,
        description,
        files,
        size_bytes,
        created_by: by,
        created_at: now,
    })
}

/// Editors and owners upload. A name that exists gets its next version; no version changes.
pub async fn add<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    new: NewSkill,
) -> Result<SkillVersion, Refusal> {
    let (name, _, _) = check(&new.files)?;
    permit(store, who, Action::SkillAdd, &name, now).await?;
    for _ in 0..3 {
        let version = next_version(store, who.organization, &name).await?;
        let skill = build(new.files.clone(), version, who.member, now)?;
        let audit = entry(
            who,
            "skill.add",
            format!("{name}@{version}"),
            AuditOutcome::Done,
            now,
        );
        match store
            .add_skill_version(who.organization, &skill, None, &audit)
            .await
        {
            Ok(()) => return Ok(skill),
            Err(StoreError::Conflict) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Err(Refusal::Conflict)
}

/// One `SKILL.md` as a single-file pack.
pub fn single(text: &str) -> NewSkill {
    NewSkill {
        files: vec![SkillFile {
            path: "SKILL.md".to_owned(),
            content: text.to_owned(),
        }],
    }
}

/// Removes a skill with all its versions. Owners only.
pub async fn remove<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    name: &str,
) -> Result<(), Refusal> {
    permit(store, who, Action::SkillRemove, name, now).await?;
    store
        .remove_skill(
            who.organization,
            name,
            &entry(who, "skill.remove", name, AuditOutcome::Done, now),
        )
        .await?;
    Ok(())
}
