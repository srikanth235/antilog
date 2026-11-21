//! What a model request is made of, for the Overview's Context section. The gateway reads the
//! request body it forwards, estimates each part with the cheapest count there is (four
//! characters to a token), and scales the estimates so that together they equal the input the
//! provider reports for that call. The estimate gives the proportions, the provider the total.

use antilog_types::{ContextPart, ContextPartKind};
use serde_json::Value;

/// What OpenCode names the tools of the `antilog` MCP entry.
const ANTILOG_TOOL_PREFIX: &str = "antilog_";

/// Estimated characters per part of one request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Measured {
    pub system: u64,
    pub tools: u64,
    pub antilog_tools: u64,
    pub messages: u64,
}

/// The characters in a value's text, keys left out. An image's bytes are not text, so a
/// `data` or `image_url` string is not counted, nor is a `role` or `type` label.
fn text(value: &Value) -> u64 {
    match value {
        Value::String(s) => s.chars().count() as u64,
        Value::Array(items) => items.iter().map(text).sum(),
        Value::Object(map) => map
            .iter()
            .filter(|(key, _)| !matches!(key.as_str(), "data" | "image_url" | "role" | "type"))
            .map(|(_, v)| text(v))
            .sum(),
        _ => 0,
    }
}

/// A tool definition counts by what the model is shown: its name, description and schema.
fn definition(tool: &Value) -> u64 {
    tool.to_string().chars().count() as u64
}

fn tool_name(tool: &Value) -> &str {
    tool.get("name")
        .or_else(|| tool.pointer("/function/name"))
        .and_then(Value::as_str)
        .unwrap_or_default()
}

/// Whether a model call is one the agent's conversation makes: it offers tools. A title or a
/// summary asked of a small model has none, and must not replace the agent's own breakdown.
pub fn is_agent_call(body: &Value) -> bool {
    body.get("tools")
        .and_then(Value::as_array)
        .is_some_and(|tools| !tools.is_empty())
}

/// Reads a request of any of the three shapes: Anthropic's messages, OpenAI's chat completions
/// and OpenAI's responses. `None` when it has nothing to measure.
pub fn measure(body: &Value) -> Option<Measured> {
    let mut out = Measured::default();
    // Anthropic: `system` is text or blocks. Responses: `instructions`.
    for key in ["system", "instructions"] {
        if let Some(system) = body.get(key) {
            out.system += text(system);
        }
    }
    if let Some(tools) = body.get("tools").and_then(Value::as_array) {
        for tool in tools {
            if tool_name(tool).starts_with(ANTILOG_TOOL_PREFIX) {
                out.antilog_tools += definition(tool);
            } else {
                out.tools += definition(tool);
            }
        }
    }
    // Chat completions and Anthropic: `messages`. Responses: `input`, text or items. A
    // system or developer message of the chat shape is the system prompt.
    for key in ["messages", "input"] {
        match body.get(key) {
            Some(Value::Array(items)) => {
                for item in items {
                    let role = item.get("role").and_then(Value::as_str);
                    if matches!(role, Some("system" | "developer")) {
                        out.system += text(item);
                    } else {
                        out.messages += text(item);
                    }
                }
            }
            Some(other @ Value::String(_)) => out.messages += text(other),
            _ => {}
        }
    }
    (out.system + out.tools + out.antilog_tools + out.messages > 0).then_some(out)
}

/// The parts scaled so that they add up to `total` tokens, the provider's own count of the
/// call's input (new, cache read and cache written). A part with no text is left out. Empty
/// when the provider reported no input.
pub fn scale(measured: Measured, total: u64) -> Vec<ContextPart> {
    let weights = [
        (ContextPartKind::System, measured.system),
        (ContextPartKind::Tools, measured.tools),
        (ContextPartKind::AntilogTools, measured.antilog_tools),
        (ContextPartKind::Messages, measured.messages),
    ];
    let sum: u64 = weights.iter().map(|(_, w)| w).sum();
    if sum == 0 || total == 0 {
        return Vec::new();
    }
    let mut parts: Vec<ContextPart> = weights
        .iter()
        .filter(|(_, w)| *w > 0)
        .map(|(kind, w)| ContextPart {
            kind: *kind,
            tokens: u64::try_from(u128::from(*w) * u128::from(total) / u128::from(sum))
                .unwrap_or(u64::MAX),
        })
        .collect();
    // Rounding down leaves a few tokens over: give them to the largest part.
    let given: u64 = parts.iter().map(|p| p.tokens).sum();
    if let Some(largest) = parts.iter_mut().max_by_key(|p| p.tokens) {
        largest.tokens += total.saturating_sub(given);
    }
    parts
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn sum(parts: &[ContextPart]) -> u64 {
        parts.iter().map(|p| p.tokens).sum()
    }

    #[test]
    fn an_anthropic_request_is_split_into_system_tools_and_messages() {
        let body = json!({
            "system": [{"type": "text", "text": "x".repeat(400)}],
            "tools": [
                {"name": "bash", "description": "run", "input_schema": {"type": "object"}},
                {"name": "antilog_attach_repo", "description": "attach", "input_schema": {}},
            ],
            "messages": [{"role": "user", "content": "y".repeat(800)}],
        });
        let measured = measure(&body).unwrap();
        assert_eq!(measured.system, 400);
        assert_eq!(measured.messages, 800);
        assert!(measured.tools > 0 && measured.antilog_tools > 0);
        assert!(is_agent_call(&body));
    }

    #[test]
    fn the_estimates_are_scaled_to_the_providers_input() {
        let measured = Measured {
            system: 100,
            tools: 300,
            antilog_tools: 0,
            messages: 600,
        };
        let parts = scale(measured, 10_000);
        assert_eq!(sum(&parts), 10_000);
        assert_eq!(parts.len(), 3, "an empty part is left out");
        assert_eq!(parts[0].tokens, 1_000);
        // A total that does not divide evenly still adds up.
        assert_eq!(sum(&scale(measured, 7)), 7);
        assert!(scale(measured, 0).is_empty());
        assert!(scale(Measured::default(), 10).is_empty());
    }

    #[test]
    fn chat_and_responses_shapes_are_read() {
        let chat = json!({
            "messages": [
                {"role": "system", "content": "s".repeat(40)},
                {"role": "user", "content": "u".repeat(80)},
            ],
            "tools": [{"type": "function", "function": {"name": "antilog_set_environment", "parameters": {}}}],
        });
        let measured = measure(&chat).unwrap();
        assert_eq!(measured.system, 40);
        assert_eq!(measured.messages, 80);
        assert!(measured.antilog_tools > 0 && measured.tools == 0);
        let responses = json!({"instructions": "i".repeat(20), "input": "q".repeat(10)});
        let measured = measure(&responses).unwrap();
        assert_eq!((measured.system, measured.messages), (20, 10));
        assert!(!is_agent_call(&responses), "no tools: a title or a summary");
        assert!(measure(&json!({"model": "m"})).is_none());
    }

    #[test]
    fn an_images_bytes_are_not_text() {
        let body = json!({"messages": [{"role": "user", "content": [
            {"type": "image", "source": {"type": "base64", "data": "A".repeat(100_000)}},
            {"type": "text", "text": "hi"},
        ]}]});
        let measured = measure(&body).unwrap();
        assert!(measured.messages < 100);
    }
}
