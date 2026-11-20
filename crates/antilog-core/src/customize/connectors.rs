//! Connectors: remote MCP servers an experiment may switch on. Antilog calls them for the
//! session, so the credential never enters a machine.

use std::future::Future;
use std::net::IpAddr;

use antilog_types::{
    AuditOutcome, Connector, ConnectorHealth, ConnectorId, ConnectorTool, MemberId, NewConnector,
    SetToolMode, ToolMode,
};
use serde_json::Value;

use crate::evaluations::slug;
use crate::organization::{Action, Principal, Refusal, entry, member_of, permit};
use crate::sealed::Sealer;
use crate::store::{ConnectorRow, Store};

#[derive(Debug, Clone, PartialEq)]
pub struct RemoteTool {
    pub name: String,
    pub description: String,
    pub schema: Value,
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("{0}")]
    Unreachable(String),
    #[error("the connector refused the credential")]
    Refused,
}

/// The remote end of a connector. A real one speaks MCP over HTTP; a test scripts one.
pub trait ConnectorClient: Send + Sync {
    /// Whether a connector may sit on a private or loopback address (local development only).
    fn allow_private(&self) -> bool;

    fn list_tools(
        &self,
        url: &str,
        bearer: Option<&str>,
    ) -> impl Future<Output = Result<Vec<RemoteTool>, ClientError>> + Send;

    /// The server's `tools/call` result, as it sent it.
    fn call_tool(
        &self,
        url: &str,
        bearer: Option<&str>,
        tool: &str,
        arguments: Value,
    ) -> impl Future<Output = Result<Value, ClientError>> + Send;
}

/// A connector's address must be https and name a public host. A literal private address,
/// `localhost` and internal names are refused, so the server cannot be pointed at a network the
/// customer does not own. (A name that later resolves to a private address is not caught here.)
pub fn check_url(url: &str, allow_private: bool) -> Result<(), &'static str> {
    let bad = "a connector's address is an https URL of a public server";
    let (scheme, rest) = url.split_once("://").ok_or(bad)?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') || url.len() > 2000 {
        return Err(bad);
    }
    let host = if let Some(inner) = authority.strip_prefix('[') {
        inner.split(']').next().unwrap_or_default()
    } else {
        authority
            .rsplit_once(':')
            .map_or(authority, |(host, _)| host)
    }
    .to_ascii_lowercase();
    let private = host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
        || host.ends_with(".localdomain")
        || (!host.contains('.') && host.parse::<IpAddr>().is_err())
        || host.parse::<IpAddr>().is_ok_and(|ip| !is_public(ip));
    match (scheme, private, allow_private) {
        ("https", false, _) => Ok(()),
        ("http" | "https", _, true) => Ok(()),
        _ => Err(bad),
    }
}

/// Whether an address is one a connector may sit on: not loopback, private, link-local (which
/// holds the cloud metadata address), unique-local, shared (CGNAT), multicast or unspecified.
/// An IPv4 address written inside IPv6 is judged as the IPv4 address.
pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let o = ip.octets();
            !(ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
                || ip.is_multicast()
                || (o[0] == 100 && (64..128).contains(&o[1]))
                || o[0] == 0)
        }
        IpAddr::V6(ip) => match ip.to_ipv4_mapped() {
            Some(mapped) => is_public(IpAddr::V4(mapped)),
            None => {
                let first = ip.segments()[0];
                !(ip.is_loopback()
                    || ip.is_unspecified()
                    || ip.is_multicast()
                    || (first & 0xfe00) == 0xfc00
                    || (first & 0xffc0) == 0xfe80)
            }
        },
    }
}

/// `slug__tool`: the name a session sees.
pub fn namespaced(connector: &Connector, tool: &str) -> String {
    format!("{}__{tool}", connector.slug)
}

pub async fn list<S: Store>(store: &S, who: &Principal) -> Result<Vec<Connector>, Refusal> {
    member_of(store, who).await?;
    Ok(store.connectors(who.organization).await?)
}

pub async fn get<S: Store>(
    store: &S,
    who: &Principal,
    id: ConnectorId,
) -> Result<Connector, Refusal> {
    member_of(store, who).await?;
    store
        .connector(who.organization, id)
        .await?
        .ok_or(Refusal::NotFound)
}

fn row_id(id: ConnectorId) -> String {
    format!("connector:{id}")
}

/// Asks the server which tools it has, keeping the modes people already set. A tool seen for the
/// first time asks first. Never says Connected for a server that did not answer.
async fn discover<C: ConnectorClient>(
    client: &C,
    url: &str,
    bearer: Option<&str>,
    previous: &[ConnectorTool],
) -> (ConnectorHealth, Vec<ConnectorTool>) {
    match client.list_tools(url, bearer).await {
        Ok(found) => {
            let tools = found
                .into_iter()
                .map(|tool| ConnectorTool {
                    mode: previous
                        .iter()
                        .find(|have| have.name == tool.name)
                        .map_or(ToolMode::Ask, |have| have.mode),
                    description: tool.description.chars().take(1000).collect(),
                    name: tool.name,
                })
                .collect();
            (ConnectorHealth::Connected, tools)
        }
        Err(_) => (ConnectorHealth::Reconnect, previous.to_vec()),
    }
}

async fn check<S: Store, C: ConnectorClient>(
    store: &S,
    client: &C,
    sealer: Option<&Sealer>,
    organization: antilog_types::OrganizationId,
    connector: &Connector,
) -> (ConnectorHealth, Vec<ConnectorTool>) {
    let bearer = match store.connector_credential(organization, connector.id).await {
        Ok(Some(sealed)) => {
            match sealer.and_then(|s| s.open(&sealed, &row_id(connector.id)).ok()) {
                Some(secret) => Some(secret),
                None => return (ConnectorHealth::Reconnect, connector.tools.clone()),
            }
        }
        Ok(None) => None,
        Err(_) => return (ConnectorHealth::NotChecked, connector.tools.clone()),
    };
    discover(
        client,
        &connector.url,
        bearer.as_ref().map(|s| s.expose()),
        &connector.tools,
    )
    .await
}

/// Owners add one. The credential is sealed at once and never read back out.
pub async fn add<S: Store, C: ConnectorClient>(
    store: &S,
    client: &C,
    sealer: Option<&Sealer>,
    who: &Principal,
    now: i64,
    new: NewConnector,
) -> Result<Connector, Refusal> {
    let slug = slug(&new.name);
    if slug.is_empty() || new.name.trim().chars().count() > 100 {
        return Err(Refusal::Invalid(
            "a connector needs a name of up to 100 characters",
        ));
    }
    check_url(new.url.trim(), client.allow_private()).map_err(Refusal::Invalid)?;
    permit(store, who, Action::ConnectorSet, None, &slug, now).await?;
    let id = ConnectorId::new();
    let credential = new.credential.filter(|c| !c.trim().is_empty());
    let sealed = match &credential {
        Some(value) => {
            if value.len() > 4000 {
                return Err(Refusal::Invalid("a credential is at most 4000 characters"));
            }
            let sealer = sealer.ok_or(Refusal::Invalid(
                "this server has no master key, so it cannot keep a credential: add the connector without one",
            ))?;
            Some(
                sealer
                    .seal(value.trim(), &row_id(id))
                    .map_err(|e| Refusal::Unavailable(Box::new(e)))?,
            )
        }
        None => None,
    };
    let mut connector = Connector {
        id,
        name: new.name.trim().to_owned(),
        slug,
        url: new.url.trim().to_owned(),
        has_credential: sealed.is_some(),
        health: ConnectorHealth::NotChecked,
        tools: Vec::new(),
        created_by: who.member,
        created_at: now,
    };
    let (health, tools) = discover(
        client,
        &connector.url,
        credential.as_deref().map(str::trim),
        &[],
    )
    .await;
    connector.health = health;
    connector.tools = tools;
    store
        .add_connector(
            who.organization,
            &ConnectorRow {
                connector: connector.clone(),
                sealed,
                plugin: None,
            },
            &entry(who, "connector.add", id, AuditOutcome::Done, now),
        )
        .await?;
    Ok(connector)
}

/// Check again: asks the server for its tools. Owners and editors may run it.
pub async fn recheck<S: Store, C: ConnectorClient>(
    store: &S,
    client: &C,
    sealer: Option<&Sealer>,
    who: &Principal,
    now: i64,
    id: ConnectorId,
) -> Result<Connector, Refusal> {
    let connector = get(store, who, id).await?;
    permit(store, who, Action::ConnectorSet, None, id, now).await?;
    let (health, tools) = check(store, client, sealer, who.organization, &connector).await;
    store
        .set_connector_check(
            who.organization,
            id,
            health,
            &tools,
            &entry(who, "connector.check", id, AuditOutcome::Done, now),
        )
        .await?;
    get(store, who, id).await
}

pub async fn set_tool_mode<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    id: ConnectorId,
    tool: &str,
    change: SetToolMode,
) -> Result<Connector, Refusal> {
    permit(store, who, Action::ConnectorSet, None, id, now).await?;
    store
        .set_tool_mode(
            who.organization,
            id,
            tool,
            change.mode,
            &entry(
                who,
                "connector.set_tool",
                format!("{id}:{tool}"),
                AuditOutcome::Done,
                now,
            ),
        )
        .await?;
    get(store, who, id).await
}

/// A new credential replaces the old one; the connector is checked again.
pub async fn update_credential<S: Store, C: ConnectorClient>(
    store: &S,
    client: &C,
    sealer: Option<&Sealer>,
    who: &Principal,
    now: i64,
    id: ConnectorId,
    credential: &str,
) -> Result<Connector, Refusal> {
    permit(store, who, Action::ConnectorSet, None, id, now).await?;
    get(store, who, id).await?;
    let sealer = sealer.ok_or(Refusal::Invalid("this server has no master key"))?;
    if credential.trim().is_empty() || credential.len() > 4000 {
        return Err(Refusal::Invalid("a credential is 1 to 4000 characters"));
    }
    let sealed = sealer
        .seal(credential.trim(), &row_id(id))
        .map_err(|e| Refusal::Unavailable(Box::new(e)))?;
    store
        .set_connector_credential(
            who.organization,
            id,
            &sealed,
            &entry(
                who,
                "connector.update_credential",
                id,
                AuditOutcome::Done,
                now,
            ),
        )
        .await?;
    recheck(store, client, Some(sealer), who, now, id).await
}

/// A connector an experiment has switched on is in use and cannot be removed.
pub async fn remove<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    id: ConnectorId,
) -> Result<(), Refusal> {
    permit(store, who, Action::ConnectorSet, None, id, now).await?;
    get(store, who, id).await?;
    if !store
        .connector_users(who.organization, id)
        .await?
        .is_empty()
    {
        return Err(Refusal::Invalid(
            "an experiment has this connector switched on: switch it off there first",
        ));
    }
    store
        .remove_connector(
            who.organization,
            id,
            &entry(who, "connector.remove", id, AuditOutcome::Done, now),
        )
        .await?;
    Ok(())
}

/// The plain connector row for a plugin's connector: no credential, not checked.
pub(crate) fn from_plugin(name: &str, url: &str, by: MemberId, now: i64) -> Option<ConnectorRow> {
    let slug = slug(name);
    if slug.is_empty() {
        return None;
    }
    Some(ConnectorRow {
        connector: Connector {
            id: ConnectorId::new(),
            name: name.trim().to_owned(),
            slug,
            url: url.trim().to_owned(),
            has_credential: false,
            health: ConnectorHealth::NotChecked,
            tools: Vec::new(),
            created_by: by,
            created_at: now,
        },
        sealed: None,
        plugin: None,
    })
}
