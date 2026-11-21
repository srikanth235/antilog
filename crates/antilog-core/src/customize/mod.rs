//! Customize: what an agent can reach beyond its machine. Connectors are remote MCP servers the
//! server calls on a session's behalf, skills are organization `SKILL.md` folders with fixed
//! versions, and a plugin is a bundle of both (`docs/V0.md`). Every session gets all of an organization's
//! connectors and the latest version of each skill.

pub mod connectors;
pub mod plugins;
pub mod reach;
pub mod skills;
pub mod suite;

pub use connectors::{ClientError, ConnectorClient, RemoteTool};

/// "Load test (p95)" becomes `load-test-p95`.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').chars().take(64).collect()
}
