//! The environments an organization manages: the machine a session runs on. Every change makes
//! a new version; a session records the version it started on. The built-in default is always
//! offered, so a first session starts without a visit to settings.

use antilog_types::{
    AuditOutcome, Environment, EnvironmentSpec, NetworkSecret, NewEnvironment, NewNetworkSecret,
    OrganizationId,
};

use crate::organization::{Action, Principal, Refusal, entry, member_of, permit};
use crate::secrets::Secret;
use crate::store::{SecretValue, Store};

/// The id of the built-in environment. No stored environment may take it.
pub const DEFAULT: &str = "default";

const MAX_HOSTS: usize = 100;
const MAX_VARIABLES: usize = 100;
const MAX_VALUE_BYTES: usize = 8 * 1024;
const MAX_REPOS: usize = 50;
const MAX_SECRETS: usize = 50;
/// The longest setup script, in bytes.
pub const MAX_SETUP_BYTES: usize = 64 * 1024;

/// A stable id from the name: lower-case letters and digits joined by single hyphens.
fn slug(name: &str) -> String {
    let mut id = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            id.push(c.to_ascii_lowercase());
        } else if !id.is_empty() && !id.ends_with('-') {
            id.push('-');
        }
    }
    id.truncate(64);
    id.trim_end_matches('-').to_owned()
}

/// One exact host name: dotted labels of letters, digits and hyphens. An address, a wildcard,
/// a port or a scheme is not a host name here.
fn host(raw: &str) -> Option<String> {
    let host = raw.trim().trim_end_matches('.').to_ascii_lowercase();
    let labels: Vec<&str> = host.split('.').collect();
    let well_formed = host.len() <= 253
        && labels.len() >= 2
        && labels.iter().all(|label| {
            (1..=63).contains(&label.len())
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
        && labels
            .last()
            .is_some_and(|tld| tld.bytes().any(|b| b.is_ascii_lowercase()));
    well_formed.then_some(host)
}

/// `owner/repo`: two names of letters, digits, dots, hyphens and underscores.
fn repo(raw: &str) -> Option<String> {
    let clean = raw.trim();
    let part = |part: &str| {
        !part.is_empty()
            && part.len() <= 100
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
    };
    clean
        .split_once('/')
        .is_some_and(|(owner, name)| part(owner) && part(name))
        .then(|| clean.to_owned())
}

/// A header name a person may have the proxy set. The ones that frame the request or name its
/// destination are the proxy's own.
fn header_ok(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        && !matches!(
            name.to_ascii_lowercase().as_str(),
            "host"
                | "content-length"
                | "transfer-encoding"
                | "connection"
                | "upgrade"
                | "te"
                | "trailer"
                | "proxy-authorization"
                | "proxy-authenticate"
        )
}

/// A name the machine's own wiring relies on: the way out is the proxy, and the proxy is told
/// which session this is by Antilog, never by a variable an environment sets.
fn reserved(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    upper.starts_with("ANTILOG_")
        || matches!(
            upper.as_str(),
            "HTTP_PROXY" | "HTTPS_PROXY" | "ALL_PROXY" | "NO_PROXY"
        )
}

fn variable_name_ok(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.len() <= 128
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Checks what a person gave and returns it cleaned: the name trimmed, host names lower-cased
/// and without repeats.
fn checked(mut new: NewEnvironment) -> Result<NewEnvironment, Refusal> {
    let name = new.name.trim().to_owned();
    if !(1..=200).contains(&name.chars().count()) {
        return Err(Refusal::Invalid(
            "an environment's name is 1 to 200 characters",
        ));
    }
    if new.allowed_hosts.len() > MAX_HOSTS {
        return Err(Refusal::Invalid("an environment allows at most 100 hosts"));
    }
    let mut hosts = Vec::new();
    for raw in &new.allowed_hosts {
        let Some(clean) = host(raw) else {
            return Err(Refusal::Invalid(
                "an allowed host is an exact host name such as api.example.com: no address, wildcard, port or scheme",
            ));
        };
        if !hosts.contains(&clean) {
            hosts.push(clean);
        }
    }
    if new.variables.len() > MAX_VARIABLES {
        return Err(Refusal::Invalid("an environment has at most 100 variables"));
    }
    for (key, value) in &new.variables {
        if !variable_name_ok(key) {
            return Err(Refusal::Invalid(
                "a variable's name is letters, digits and underscores, and does not start with a digit",
            ));
        }
        if reserved(key) {
            return Err(Refusal::Invalid(
                "a variable cannot be named ANTILOG_* or set a proxy: the machine's way out is Antilog's",
            ));
        }
        if value.len() > MAX_VALUE_BYTES || value.contains('\0') {
            return Err(Refusal::Invalid(
                "a variable's value is at most 8 KB of text",
            ));
        }
    }
    if new.setup_script.len() > MAX_SETUP_BYTES || new.setup_script.contains('\0') {
        return Err(Refusal::Invalid(
            "the setup script is at most 64 KB of text",
        ));
    }
    if new.repos.len() > MAX_REPOS {
        return Err(Refusal::Invalid("an environment has at most 50 repos"));
    }
    let mut repos = Vec::new();
    for raw in &new.repos {
        let Some(clean) = repo(raw) else {
            return Err(Refusal::Invalid("a repo looks like owner/name"));
        };
        if !repos.contains(&clean) {
            repos.push(clean);
        }
    }
    if new.network_secrets.len() > MAX_SECRETS {
        return Err(Refusal::Invalid(
            "an environment has at most 50 network secrets",
        ));
    }
    let mut secrets: Vec<NewNetworkSecret> = Vec::new();
    for mut secret in std::mem::take(&mut new.network_secrets) {
        let Some(clean) = host(&secret.host) else {
            return Err(Refusal::Invalid(
                "a network secret's host is an exact host name such as api.example.com",
            ));
        };
        secret.host = clean;
        if !header_ok(&secret.header) {
            return Err(Refusal::Invalid(
                "a network secret's header is a plain header name such as Authorization",
            ));
        }
        secret.prefix = secret
            .prefix
            .take()
            .map(|prefix| prefix.trim().to_owned())
            .filter(|prefix| !prefix.is_empty());
        if secret
            .prefix
            .as_ref()
            .is_some_and(|prefix| prefix.len() > 32 || prefix.chars().any(char::is_control))
        {
            return Err(Refusal::Invalid(
                "a network secret's prefix is at most 32 characters, such as Bearer",
            ));
        }
        if let Some(value) = &secret.value
            && (value.is_empty()
                || value.len() > MAX_VALUE_BYTES
                || value.chars().any(char::is_control))
        {
            return Err(Refusal::Invalid(
                "a network secret's value is 1 byte to 8 KB on one line",
            ));
        }
        if secrets.iter().any(|earlier| {
            earlier.host == secret.host && earlier.header.eq_ignore_ascii_case(&secret.header)
        }) {
            return Err(Refusal::Invalid(
                "a host takes each header once: a network secret repeats one",
            ));
        }
        secrets.push(secret);
    }
    if !(1..=16).contains(&new.cpus) {
        return Err(Refusal::Invalid("an environment has 1 to 16 CPUs"));
    }
    if !(1024..=65536).contains(&new.memory_mb) {
        return Err(Refusal::Invalid("an environment has 1 to 64 GB of memory"));
    }
    Ok(NewEnvironment {
        name,
        allowed_hosts: hosts,
        repos,
        network_secrets: secrets,
        ..new
    })
}

/// The version to store and the secret values that go with it. `None` keeps a stored value.
fn build(
    who: &Principal,
    now: i64,
    id: String,
    version: u32,
    new: NewEnvironment,
    archived: bool,
) -> (Environment, Vec<SecretValue>) {
    let values = new
        .network_secrets
        .iter()
        .map(|secret| SecretValue {
            host: secret.host.clone(),
            header: secret.header.clone(),
            value: secret.value.as_deref().map(Secret::new),
        })
        .collect();
    let made = Environment {
        id,
        name: new.name,
        version,
        network: new.network,
        allowed_hosts: new.allowed_hosts,
        allow_package_managers: new.allow_package_managers,
        variables: new.variables,
        setup_script: new.setup_script,
        repos: new.repos,
        network_secrets: new
            .network_secrets
            .into_iter()
            .map(|secret| NetworkSecret {
                host: secret.host,
                header: secret.header,
                prefix: secret.prefix,
            })
            .collect(),
        cpus: new.cpus,
        memory_mb: new.memory_mb,
        built_in: false,
        archived,
        created_by: Some(who.member),
        created_at: now,
    };
    (made, values)
}

/// Every member reads the environments: the built-in default first, then the organization's,
/// archived ones included and marked.
pub async fn list<S: Store>(store: &S, who: &Principal) -> Result<Vec<Environment>, Refusal> {
    member_of(store, who).await?;
    let mut all = vec![Environment::built_in()];
    all.extend(store.environments(who.organization).await?);
    Ok(all)
}

/// Owners and editors make an environment, at version 1. Its id comes from its name and is
/// kept for good.
pub async fn create<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    new: NewEnvironment,
) -> Result<Environment, Refusal> {
    permit(store, who, Action::EnvironmentCreate, &new.name, now).await?;
    let new = checked(new)?;
    let id = slug(&new.name);
    if id.is_empty() {
        return Err(Refusal::Invalid(
            "an environment's name needs a letter or a digit",
        ));
    }
    if id == DEFAULT {
        return Err(Refusal::Conflict);
    }
    if new
        .network_secrets
        .iter()
        .any(|secret| secret.value.is_none())
    {
        return Err(Refusal::Invalid("a new network secret needs a value"));
    }
    let (made, values) = build(who, now, id.clone(), 1, new, false);
    store
        .add_environment(
            who.organization,
            &made,
            &values,
            &entry(who, "environment.create", &id, AuditOutcome::Done, now),
        )
        .await?;
    Ok(made)
}

/// Owners and editors change an environment: the change is its next version. Sessions that
/// started on an earlier version keep it.
pub async fn edit<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    id: &str,
    new: NewEnvironment,
) -> Result<Environment, Refusal> {
    permit(store, who, Action::EnvironmentEdit, id, now).await?;
    let new = checked(new)?;
    if id == DEFAULT {
        return Err(Refusal::Invalid(
            "the default environment cannot be changed",
        ));
    }
    let current = store
        .environments(who.organization)
        .await?
        .into_iter()
        .find(|row| row.id == id)
        .ok_or(Refusal::NotFound)?;
    if current.archived {
        return Err(Refusal::Invalid(
            "an archived environment cannot be changed",
        ));
    }
    // A value left out keeps the one the same host and header had: only a stored secret can.
    if new.network_secrets.iter().any(|secret| {
        secret.value.is_none()
            && !current.network_secrets.iter().any(|had| {
                had.host == secret.host && had.header.eq_ignore_ascii_case(&secret.header)
            })
    }) {
        return Err(Refusal::Invalid("a new network secret needs a value"));
    }
    let (made, values) = build(who, now, id.to_owned(), current.version + 1, new, false);
    store
        .add_environment_version(
            who.organization,
            &made,
            &values,
            &entry(who, "environment.edit", id, AuditOutcome::Done, now),
        )
        .await?;
    Ok(made)
}

/// Owners and editors archive an environment: new sessions are not offered it, and sessions
/// already on it keep running.
pub async fn archive<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    id: &str,
) -> Result<(), Refusal> {
    permit(store, who, Action::EnvironmentArchive, id, now).await?;
    if id == DEFAULT {
        return Err(Refusal::Invalid(
            "the default environment cannot be archived",
        ));
    }
    store
        .archive_environment(
            who.organization,
            id,
            &entry(who, "environment.archive", id, AuditOutcome::Done, now),
        )
        .await?;
    Ok(())
}

/// Why a session cannot start on the environment it asked for.
#[derive(Debug, thiserror::Error)]
pub enum Unavailable {
    #[error("no such environment in this organization")]
    Unknown,
    #[error("that environment is archived: pick another")]
    Archived,
    #[error(transparent)]
    Store(#[from] crate::store::StoreError),
}

/// The environment a new session starts on, as the seed records it.
#[derive(Debug, Default)]
pub struct Pinned {
    /// `None` for the built-in default.
    pub id: Option<String>,
    /// The newest version now.
    pub version: Option<u32>,
    /// The environment's default repos, which the session starts with.
    pub repos: Vec<String>,
}

/// The environment a new session starts on: `None` for the built-in default, otherwise the id,
/// the newest version now and its default repos.
pub async fn pin<S: Store>(
    store: &S,
    organization: OrganizationId,
    requested: Option<&str>,
) -> Result<Pinned, Unavailable> {
    let Some(id) = requested.filter(|id| *id != DEFAULT) else {
        return Ok(Pinned::default());
    };
    match store
        .environments(organization)
        .await?
        .into_iter()
        .find(|row| row.id == id)
    {
        None => Err(Unavailable::Unknown),
        Some(row) if row.archived => Err(Unavailable::Archived),
        Some(row) => Ok(Pinned {
            id: Some(row.id),
            version: Some(row.version),
            repos: row.repos,
        }),
    }
}

/// The version a session was started on, for a claim or an enforcement. A seed with no
/// environment is on the built-in default. `None` when the stored version has gone missing.
pub async fn of_seed<S: Store>(
    store: &S,
    organization: OrganizationId,
    seed: &antilog_types::Seed,
) -> Result<Option<EnvironmentSpec>, crate::store::StoreError> {
    let (Some(id), Some(version)) = (&seed.environment, seed.environment_version) else {
        return Ok(Some(EnvironmentSpec::default()));
    };
    Ok(store
        .environment_version(organization, id, version)
        .await?
        .map(|row| row.spec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_becomes_a_slug() {
        assert_eq!(slug("Checkout staging"), "checkout-staging");
        assert_eq!(slug("  Bench -- std! "), "bench-std");
        assert_eq!(slug("???"), "");
    }

    #[test]
    fn a_host_is_an_exact_name() {
        assert_eq!(host("API.Stripe.com.").as_deref(), Some("api.stripe.com"));
        for bad in [
            "*.stripe.com",
            "stripe",
            "10.0.0.1",
            "https://stripe.com",
            "stripe.com:443",
            "a..com",
            "-a.com",
            "a.com/x",
            "",
        ] {
            assert_eq!(host(bad), None, "{bad}");
        }
    }
}
