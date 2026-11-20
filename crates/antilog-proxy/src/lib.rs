#![forbid(unsafe_code)]
//! The one proxy outside the session machine (`docs/V0.md`). It is the model gateway, swapping a
//! session's short-lived token for the organization's model key and booking spend from each reply,
//! and it is the session's outbound allowlist, a CONNECT tunnel that never decrypts what passes.

pub mod egress;
pub mod gateway;
pub mod mcp;
pub mod price;
pub mod server;
pub mod usage;

use std::sync::Arc;

use antilog_core::clock::Clock;
use antilog_core::credentials::SessionCredentials;
use antilog_core::store::Store;

pub use egress::{NetworkAccess, SessionNetworks};

/// Where each provider's API is. A test points these at a fake.
#[derive(Debug, Clone)]
pub struct Upstreams {
    pub anthropic: String,
    pub openai: String,
    /// OpenCode Zen: chat completions, Responses or Messages, by the path called.
    pub opencode: String,
    /// The Antilog server, where `/mcp/*` is passed on to; none turns the pass-through off.
    pub server: Option<String>,
}

impl Default for Upstreams {
    fn default() -> Self {
        Self {
            anthropic: "https://api.anthropic.com".to_owned(),
            openai: "https://api.openai.com".to_owned(),
            opencode: "https://opencode.ai/zen".to_owned(),
            server: None,
        }
    }
}

/// Everything a request needs. `S` is the ledger, the sealed keys and the environments, `C` the
/// session tokens.
pub struct Proxy<S, C, K> {
    pub store: S,
    pub credentials: C,
    pub clock: K,
    /// What each session's environment lets it reach, looked up in `store` and kept briefly.
    pub networks: SessionNetworks,
    pub upstreams: Upstreams,
    pub http: reqwest::Client,
    /// Hosts every session reaches whatever its environment says: this proxy's and the server's
    /// public names (the model gateway and the MCP endpoint) and the code host. Without them a
    /// session at `NetworkLevel::None` could not talk to its own model.
    pub platform_hosts: Vec<String>,
    /// Whether a tunnel may reach a private or loopback address. Only a test turns this on.
    pub allow_private_addresses: bool,
}

pub type Shared<S, C, K> = Arc<Proxy<S, C, K>>;

/// The bounds every handler needs, named once.
pub trait Backend: Store + 'static {}
impl<T: Store + 'static> Backend for T {}

pub trait Tokens: SessionCredentials + 'static {}
impl<T: SessionCredentials + 'static> Tokens for T {}

pub trait Time: Clock + 'static {}
impl<T: Clock + 'static> Time for T {}
