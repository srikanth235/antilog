//! What the adapter says to an Antilog agent runtime and judge runtime, and what they
//! answer. Both are container images we ship; AgentCore's own contract is one HTTP
//! `POST /invocations` carrying a JSON body, so this is that body.
//!
//! One runtime session is one microVM, so the agent image keeps the session's event log
//! in memory and serves `events` from it. A session that outlives the microVM (its idle
//! timeout, 8 hours at most) loses that log and reports `Ended` with a failure; the spike
//! in `product/rulings.md` measures how often that happens.

use antilog_core::platform::{PlatformEvent, ToolDef};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request<'a> {
    Start {
        agent: &'a str,
        prompt: &'a str,
        repos: &'a [String],
        environment: Option<&'a str>,
        tools: Vec<ToolOut<'a>>,
        model: &'a str,
        fallback_model: Option<&'a str>,
    },
    Input {
        text: &'a str,
    },
    Events {
        cursor: Option<&'a str>,
        wait_ms: u64,
    },
    Answer {
        call: &'a str,
        result: &'a Value,
    },
    Stop,
    Paired(&'a Value),
}

#[derive(Debug, Serialize)]
pub struct ToolOut<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub schema: &'a Value,
}

impl<'a> From<&'a ToolDef> for ToolOut<'a> {
    fn from(tool: &'a ToolDef) -> Self {
        Self {
            name: &tool.name,
            description: &tool.description,
            schema: &tool.schema,
        }
    }
}

/// Every answer is one of these, whatever the request.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Answer {
    Refused { ok: bool, error: String },
    Events(EventsAnswer),
    Pairs(PairsAnswer),
    Done { ok: bool },
}

#[derive(Debug, Deserialize)]
pub struct EventsAnswer {
    pub events: Vec<EventIn>,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventIn {
    Message {
        text: String,
    },
    ToolCall {
        id: String,
        name: String,
        input: Value,
    },
    Usage {
        cents: i64,
        tokens: i64,
    },
    Idle,
    Ended {
        #[serde(default)]
        failed: Option<String>,
    },
}

impl From<EventIn> for PlatformEvent {
    fn from(event: EventIn) -> Self {
        match event {
            EventIn::Message { text } => Self::Message { text },
            EventIn::ToolCall { id, name, input } => Self::ToolCall { id, name, input },
            EventIn::Usage { cents, tokens } => Self::Usage { cents, tokens },
            EventIn::Idle => Self::Idle,
            EventIn::Ended { failed } => Self::Ended { failed },
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PairsAnswer {
    pub metric: Vec<(f64, f64)>,
    #[serde(default)]
    pub guardrails: Vec<GuardrailPairs>,
}

#[derive(Debug, Deserialize)]
pub struct GuardrailPairs {
    pub name: String,
    pub pairs: Vec<(f64, f64)>,
}
