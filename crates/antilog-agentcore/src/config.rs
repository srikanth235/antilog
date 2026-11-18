//! What the adapter needs to find its runtimes and pick models, read from the
//! environment so a container image and a Lambda function configure it the same way.

use std::time::Duration;

/// The cheap model agents use on AgentCore (product rulings, 17 Nov 2026).
pub const DEFAULT_MODEL: &str = "zai.glm-4.7-flash";
/// What an agent falls back to when the first model is unavailable.
pub const DEFAULT_FALLBACK_MODEL: &str = "qwen.qwen3-coder-30b-a3b-v1:0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The agent runtime: an ARN, or a name the control plane resolves to one.
    pub agent_runtime: String,
    /// The judge runtime, a separate runtime with its own role and the holdout credential.
    pub judge_runtime: String,
    /// A published runtime endpoint, such as `DEFAULT` or a pinned version.
    pub qualifier: Option<String>,
    pub model: String,
    pub fallback_model: Option<String>,
    /// The longest one call to a runtime may take. A paired run is the slow one.
    pub call_timeout: Duration,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("{0} is not set")]
pub struct Missing(pub &'static str);

impl Config {
    pub fn new(agent_runtime: impl Into<String>, judge_runtime: impl Into<String>) -> Self {
        Self {
            agent_runtime: agent_runtime.into(),
            judge_runtime: judge_runtime.into(),
            qualifier: None,
            model: DEFAULT_MODEL.to_owned(),
            fallback_model: Some(DEFAULT_FALLBACK_MODEL.to_owned()),
            call_timeout: Duration::from_secs(9 * 60),
        }
    }

    pub fn from_env() -> Result<Self, Missing> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, Missing> {
        let get = |name: &str| lookup(name).filter(|v| !v.trim().is_empty());
        let mut config = Self::new(
            get("ANTILOG_AGENTCORE_AGENT_RUNTIME")
                .ok_or(Missing("ANTILOG_AGENTCORE_AGENT_RUNTIME"))?,
            get("ANTILOG_AGENTCORE_JUDGE_RUNTIME")
                .ok_or(Missing("ANTILOG_AGENTCORE_JUDGE_RUNTIME"))?,
        );
        config.qualifier = get("ANTILOG_AGENTCORE_QUALIFIER");
        if let Some(model) = get("ANTILOG_AGENT_MODEL") {
            config.model = model;
        }
        if let Some(fallback) = lookup("ANTILOG_AGENT_FALLBACK_MODEL") {
            // An empty value turns the fallback off.
            config.fallback_model = Some(fallback).filter(|v| !v.trim().is_empty());
        }
        if let Some(seconds) =
            get("ANTILOG_AGENTCORE_CALL_TIMEOUT_SECONDS").and_then(|v| v.parse().ok())
        {
            config.call_timeout = Duration::from_secs(seconds);
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn lookup(vars: &[(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<&str, &str> = vars.iter().copied().collect();
        move |name| map.get(name).map(|v| (*v).to_owned())
    }

    #[test]
    fn defaults_are_the_two_cheap_models() {
        let config = Config::from_lookup(lookup(&[
            ("ANTILOG_AGENTCORE_AGENT_RUNTIME", "agent"),
            ("ANTILOG_AGENTCORE_JUDGE_RUNTIME", "judge"),
        ]))
        .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(config.model, "zai.glm-4.7-flash");
        assert_eq!(
            config.fallback_model.as_deref(),
            Some("qwen.qwen3-coder-30b-a3b-v1:0")
        );
    }

    #[test]
    fn the_model_comes_from_configuration() {
        let config = Config::from_lookup(lookup(&[
            ("ANTILOG_AGENTCORE_AGENT_RUNTIME", "agent"),
            ("ANTILOG_AGENTCORE_JUDGE_RUNTIME", "judge"),
            ("ANTILOG_AGENT_MODEL", "other.model"),
            ("ANTILOG_AGENT_FALLBACK_MODEL", ""),
        ]))
        .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(config.model, "other.model");
        assert_eq!(config.fallback_model, None);
    }

    #[test]
    fn a_missing_runtime_is_named() {
        let err = Config::from_lookup(lookup(&[("ANTILOG_AGENTCORE_JUDGE_RUNTIME", "j")]));
        assert_eq!(err, Err(Missing("ANTILOG_AGENTCORE_AGENT_RUNTIME")));
    }
}
