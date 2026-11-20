//! The real remote end of a connector: MCP over streamable HTTP, one short exchange per call
//! (initialize, initialized, then the request). Redirects are not followed, so a connector cannot
//! send the server somewhere its address check did not allow.

use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use antilog_core::customize::connectors::is_public;
use antilog_core::customize::{ClientError, ConnectorClient, RemoteTool};
use serde_json::{Value, json};

const MAX_BODY: usize = 4 * 1024 * 1024;
const PROTOCOL: &str = "2025-03-26";

/// Turns a host name into addresses. A test scripts one; the real one asks the system.
pub trait Resolver: Send + Sync {
    fn resolve<'a>(
        &'a self,
        host: &'a str,
        port: u16,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<Vec<IpAddr>>> + Send + 'a>>;
}

pub struct SystemResolver;

impl Resolver for SystemResolver {
    fn resolve<'a>(
        &'a self,
        host: &'a str,
        port: u16,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<Vec<IpAddr>>> + Send + 'a>> {
        Box::pin(async move {
            Ok(tokio::net::lookup_host((host, port))
                .await?
                .map(|addr| addr.ip())
                .collect())
        })
    }
}

/// Every request resolves the host itself, refuses the request if any address is not public,
/// and connects to exactly the addresses it checked, so a name cannot answer differently to the
/// check and to the connection (DNS rebinding), and the cloud metadata address is never reached.
pub struct HttpConnectors {
    allow_private: bool,
    resolver: Arc<dyn Resolver>,
}

impl HttpConnectors {
    pub fn new(allow_private: bool) -> Result<Self, reqwest::Error> {
        Ok(Self::with_resolver(allow_private, Arc::new(SystemResolver)))
    }

    pub fn with_resolver(allow_private: bool, resolver: Arc<dyn Resolver>) -> Self {
        Self {
            allow_private,
            resolver,
        }
    }

    /// A client for one request, pinned to the addresses checked now.
    async fn client_for(&self, url: &str) -> Result<reqwest::Client, ClientError> {
        let unreachable = |why: &str| ClientError::Unreachable(why.to_owned());
        let parsed =
            reqwest::Url::parse(url).map_err(|_| unreachable("its address is not a URL"))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| unreachable("its address has no host"))?;
        let port = parsed
            .port_or_known_default()
            .ok_or_else(|| unreachable("its address has no port"))?;
        let literal = host.trim_matches(['[', ']']).parse::<IpAddr>().ok();
        let addrs = match literal {
            Some(ip) => vec![ip],
            None => self
                .resolver
                .resolve(host, port)
                .await
                .map_err(|_| unreachable("its name did not resolve"))?,
        };
        if addrs.is_empty() {
            return Err(unreachable("its name did not resolve"));
        }
        if !self.allow_private && addrs.iter().any(|ip| !is_public(*ip)) {
            return Err(unreachable(
                "its name points at a private address, which is refused",
            ));
        }
        let mut builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60));
        if literal.is_none() {
            let sockets: Vec<SocketAddr> =
                addrs.iter().map(|ip| SocketAddr::new(*ip, port)).collect();
            builder = builder.resolve_to_addrs(host, &sockets);
        }
        builder
            .build()
            .map_err(|error| ClientError::Unreachable(error.without_url().to_string()))
    }

    /// Local development only: `ANTILOG_CONNECTORS_ALLOW_PRIVATE=1`.
    pub fn from_env() -> Result<Self, reqwest::Error> {
        Self::new(std::env::var("ANTILOG_CONNECTORS_ALLOW_PRIVATE").as_deref() == Ok("1"))
    }

    /// One JSON-RPC message; `Some(id)` waits for its answer, `None` is a notification.
    async fn post(
        &self,
        url: &str,
        bearer: Option<&str>,
        session: Option<&str>,
        id: Option<u64>,
        method: &str,
        params: Value,
    ) -> Result<(Option<Value>, Option<String>), ClientError> {
        let body = match id {
            Some(id) => json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}),
            None => json!({"jsonrpc": "2.0", "method": method, "params": params}),
        };
        let mut request = self
            .client_for(url)
            .await?
            .post(url)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", PROTOCOL)
            .json(&body);
        if let Some(bearer) = bearer {
            request = request.bearer_auth(bearer);
        }
        if let Some(session) = session {
            request = request.header("mcp-session-id", session);
        }
        let mut response = request
            .send()
            .await
            .map_err(|error| ClientError::Unreachable(error.without_url().to_string()))?;
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(ClientError::Refused);
        }
        if !status.is_success() {
            return Err(ClientError::Unreachable(format!("answered {status}")));
        }
        let session_out = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let sse = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/event-stream"));
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| ClientError::Unreachable(error.without_url().to_string()))?
        {
            if bytes.len() + chunk.len() > MAX_BODY {
                return Err(ClientError::Unreachable(
                    "its answer is too large".to_owned(),
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        let Some(id) = id else {
            return Ok((None, session_out));
        };
        let text = String::from_utf8_lossy(&bytes);
        let messages: Vec<Value> = if sse {
            text.split("\n\n")
                .filter_map(|event| {
                    let data: Vec<&str> = event
                        .lines()
                        .filter_map(|line| line.strip_prefix("data:"))
                        .map(str::trim_start)
                        .collect();
                    serde_json::from_str(&data.join("\n")).ok()
                })
                .collect()
        } else {
            serde_json::from_str(&text).into_iter().collect()
        };
        let answer = messages
            .into_iter()
            .find(|message| message.get("id") == Some(&json!(id)))
            .ok_or_else(|| ClientError::Unreachable("it sent no answer".to_owned()))?;
        if let Some(error) = answer.get("error") {
            let said = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("an error");
            return Err(ClientError::Unreachable(said.chars().take(300).collect()));
        }
        Ok((answer.get("result").cloned(), session_out))
    }

    /// Opens the exchange and returns the session id the server gave, if it did.
    async fn open(&self, url: &str, bearer: Option<&str>) -> Result<Option<String>, ClientError> {
        let (_, session) = self
            .post(
                url,
                bearer,
                None,
                Some(1),
                "initialize",
                json!({
                    "protocolVersion": PROTOCOL,
                    "capabilities": {},
                    "clientInfo": {"name": "antilog", "version": env!("CARGO_PKG_VERSION")}
                }),
            )
            .await?;
        self.post(
            url,
            bearer,
            session.as_deref(),
            None,
            "notifications/initialized",
            json!({}),
        )
        .await?;
        Ok(session)
    }
}

impl ConnectorClient for HttpConnectors {
    fn allow_private(&self) -> bool {
        self.allow_private
    }

    async fn list_tools(
        &self,
        url: &str,
        bearer: Option<&str>,
    ) -> Result<Vec<RemoteTool>, ClientError> {
        let session = self.open(url, bearer).await?;
        let mut tools = Vec::new();
        let mut cursor: Option<String> = None;
        for page in 0..10u64 {
            let params = cursor
                .as_ref()
                .map_or_else(|| json!({}), |c| json!({"cursor": c}));
            let (result, _) = self
                .post(
                    url,
                    bearer,
                    session.as_deref(),
                    Some(2 + page),
                    "tools/list",
                    params,
                )
                .await?;
            let result = result.unwrap_or_default();
            for tool in result
                .get("tools")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(name) = tool.get("name").and_then(Value::as_str) {
                    tools.push(RemoteTool {
                        name: name.to_owned(),
                        description: tool
                            .get("description")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        schema: tool.get("inputSchema").cloned().unwrap_or_default(),
                    });
                }
            }
            cursor = result
                .get("nextCursor")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if cursor.is_none() {
                break;
            }
        }
        Ok(tools)
    }

    async fn call_tool(
        &self,
        url: &str,
        bearer: Option<&str>,
        tool: &str,
        arguments: Value,
    ) -> Result<Value, ClientError> {
        let session = self.open(url, bearer).await?;
        let (result, _) = self
            .post(
                url,
                bearer,
                session.as_deref(),
                Some(2),
                "tools/call",
                json!({"name": tool, "arguments": arguments}),
            )
            .await?;
        result.ok_or_else(|| ClientError::Unreachable("it sent no result".to_owned()))
    }
}

impl crate::api::customize::Backend<HttpConnectors> {
    /// The real connector client, and the sealer when `master-key` (`ANTILOG_SECRET_MASTER_KEY`,
    /// 64 hex characters) is set. Without the key a connector is added without a credential.
    pub async fn from_env() -> anyhow::Result<Self> {
        use antilog_core::secrets::{EnvSecrets, Secrets};
        use anyhow::Context;
        let sealer = match EnvSecrets::from_env().get("master-key").await {
            Ok(key) => Some(
                antilog_core::sealed::Sealer::new(&key)
                    .context("ANTILOG_SECRET_MASTER_KEY is not 64 hex characters")?,
            ),
            Err(_) => {
                tracing::warn!("no master-key secret: connectors cannot hold a credential");
                None
            }
        };
        Ok(Self {
            client: HttpConnectors::from_env().context("building the connector client")?,
            sealer,
        })
    }
}
