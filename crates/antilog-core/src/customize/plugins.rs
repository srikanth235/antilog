//! Plugins: an Antilog bundle of skills and connectors. Adding one unpacks
//! it into those parts, each governed as usual; a plugin has no powers of its own.

use antilog_types::{
    AuditOutcome, NewPlugin, PartKind, Plugin, PluginPart, RemovePlugin, SkillVersion,
};

use super::{connectors, skills, slug};
use crate::organization::{Action, Principal, Refusal, entry, member_of, permit};
use crate::store::{ConnectorRow, Store, StoreError};

pub async fn list<S: Store>(store: &S, who: &Principal) -> Result<Vec<Plugin>, Refusal> {
    member_of(store, who).await?;
    Ok(store.plugins(who.organization).await?)
}

/// Owners add one: the connectors and the skills it names appear in their own
/// Customize tabs. Its connectors come without a credential and unchecked.
pub async fn add<S: Store>(
    store: &S,
    allow_private: bool,
    who: &Principal,
    now: i64,
    new: NewPlugin,
) -> Result<Plugin, Refusal> {
    let id = slug(&new.name);
    if id.is_empty()
        || new.name.trim().chars().count() > 100
        || new.version.trim().is_empty()
        || new.version.len() > 40
        || new.description.chars().count() > 500
    {
        return Err(Refusal::Invalid(
            "a plugin needs a name, a version and a description of up to 500 characters",
        ));
    }
    let mut parts = Vec::new();
    let mut versions: Vec<SkillVersion> = Vec::new();
    for pack in new.skills {
        let built = skills::build(pack.files, 1, who.member, now)?;
        if versions.iter().any(|have| have.name == built.name) {
            return Err(Refusal::Invalid("two skills in the plugin have one name"));
        }
        let version = skills::next_version(store, who.organization, &built.name).await?;
        parts.push(PluginPart {
            kind: PartKind::Skill,
            name: built.name.clone(),
        });
        versions.push(SkillVersion { version, ..built });
    }
    let mut rows: Vec<ConnectorRow> = Vec::new();
    for connector in &new.connectors {
        connectors::check_url(connector.url.trim(), allow_private).map_err(Refusal::Invalid)?;
        let row = connectors::from_plugin(&connector.name, &connector.url, who.member, now)
            .ok_or(Refusal::Invalid("a connector in the plugin needs a name"))?;
        parts.push(PluginPart {
            kind: PartKind::Connector,
            name: row.connector.slug.clone(),
        });
        rows.push(row);
    }
    permit(store, who, Action::PluginAdd, &id, now).await?;
    let plugin = Plugin {
        id: id.clone(),
        name: new.name.trim().to_owned(),
        description: new.description.trim().to_owned(),
        version: new.version.trim().to_owned(),
        parts,
        created_by: who.member,
        created_at: now,
    };
    store
        .add_plugin(
            who.organization,
            &plugin,
            &rows,
            &versions,
            &entry(who, "plugin.add", &id, AuditOutcome::Done, now),
        )
        .await
        .map_err(|error| match error {
            StoreError::Conflict => Refusal::Conflict,
            other => other.into(),
        })?;
    Ok(plugin)
}

/// Removing a plugin asks, per part, whether to keep it. A part not kept is removed.
pub async fn remove<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    id: &str,
    choice: RemovePlugin,
) -> Result<(), Refusal> {
    permit(store, who, Action::PluginRemove, id, now).await?;
    let plugin = store
        .plugins(who.organization)
        .await?
        .into_iter()
        .find(|plugin| plugin.id == id)
        .ok_or(Refusal::NotFound)?;
    let mut drop = Vec::new();
    for part in &plugin.parts {
        if choice.keep.contains(part) {
            continue;
        }
        drop.push(part.clone());
    }
    store
        .remove_plugin(
            who.organization,
            id,
            &drop,
            &entry(who, "plugin.remove", id, AuditOutcome::Done, now),
        )
        .await?;
    Ok(())
}
