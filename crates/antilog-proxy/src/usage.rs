//! Reading what a call used from what the provider sent back: the usage object of a reply, or the
//! usage events of a stream. Nothing is estimated; a reply that reports none books none.

use antilog_types::ModelProvider;
use serde_json::Value;

use crate::price::Tokens;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Usage {
    pub model: Option<String>,
    pub tokens: Tokens,
    /// A usage object was seen at all.
    pub seen: bool,
}

fn count(object: &Value, key: &str) -> Option<u64> {
    object.get(key).and_then(Value::as_u64)
}

impl Usage {
    /// One JSON value of a reply or one event of a stream. Anthropic reports input in
    /// `message_start` and the running output in `message_delta`; OpenAI reports it once, at the
    /// end. The numbers are totals so far, so a later report replaces an earlier one.
    pub fn absorb(&mut self, provider: ModelProvider, value: &Value) {
        match provider {
            ModelProvider::Anthropic => self.absorb_anthropic(value),
            ModelProvider::Openai => self.absorb_openai(value),
            // Zen answers in either shape; Anthropic's replies and events are typed "message".
            ModelProvider::Opencode => {
                let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
                if kind == "message" || kind.starts_with("message_") {
                    self.absorb_anthropic(value)
                } else {
                    self.absorb_openai(value)
                }
            }
        }
    }

    fn model_of(&mut self, holder: &Value) {
        if let Some(model) = holder.get("model").and_then(Value::as_str) {
            self.model = Some(model.to_owned());
        }
    }

    fn absorb_anthropic(&mut self, value: &Value) {
        // A stream's first event wraps the message; a plain reply is the message.
        let message = value.get("message").unwrap_or(value);
        self.model_of(message);
        for holder in [message, value] {
            let Some(usage) = holder.get("usage").filter(|u| u.is_object()) else {
                continue;
            };
            self.seen = true;
            if let Some(n) = count(usage, "input_tokens") {
                self.tokens.input = n;
            }
            if let Some(n) = count(usage, "output_tokens") {
                self.tokens.output = n;
            }
            if let Some(n) = count(usage, "cache_read_input_tokens") {
                self.tokens.cache_read = n;
            }
            if let Some(n) = count(usage, "cache_creation_input_tokens") {
                self.tokens.cache_write = n;
            }
        }
    }

    fn absorb_openai(&mut self, value: &Value) {
        // Chat completions carry `usage` at the top; the Responses API nests it in `response`.
        let holder = value.get("response").unwrap_or(value);
        self.model_of(holder);
        let Some(usage) = holder.get("usage").filter(|u| u.is_object()) else {
            return;
        };
        self.seen = true;
        let prompt = count(usage, "prompt_tokens")
            .or_else(|| count(usage, "input_tokens"))
            .unwrap_or(0);
        let cached = ["prompt_tokens_details", "input_tokens_details"]
            .iter()
            .filter_map(|key| usage.get(key))
            .find_map(|details| count(details, "cached_tokens"))
            .unwrap_or(0);
        self.tokens.cache_read = cached;
        // OpenAI counts cached tokens inside the prompt; they are priced apart.
        self.tokens.input = prompt.saturating_sub(cached);
        self.tokens.output = count(usage, "completion_tokens")
            .or_else(|| count(usage, "output_tokens"))
            .unwrap_or(0);
    }
}

/// Splits a server-sent stream into events as the bytes arrive and hands each `data:` payload to
/// the usage reader. The bytes themselves are never changed; this only watches them go by.
#[derive(Default)]
pub struct EventWatcher {
    pending: Vec<u8>,
}

const MAX_EVENT: usize = 4 << 20;

impl EventWatcher {
    pub fn feed(&mut self, provider: ModelProvider, chunk: &[u8], usage: &mut Usage) {
        self.pending.extend_from_slice(chunk);
        while let Some((end, gap)) = event_end(&self.pending) {
            let event: Vec<u8> = self.pending.drain(..end + gap).collect();
            let text = String::from_utf8_lossy(event.get(..end).unwrap_or_default());
            let data: String = text
                .lines()
                .filter_map(|line| line.strip_prefix("data:"))
                .map(str::trim_start)
                .collect::<Vec<_>>()
                .join("\n");
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<Value>(&data) {
                usage.absorb(provider, &value);
            }
        }
        if self.pending.len() > MAX_EVENT {
            // Not a stream of events after all; stop holding it.
            self.pending.clear();
        }
    }
}

/// Where the first blank line ends an event, and how many bytes the blank line takes.
fn event_end(buffer: &[u8]) -> Option<(usize, usize)> {
    (0..buffer.len()).find_map(|i| {
        let rest = buffer.get(i..)?;
        if rest.starts_with(b"\r\n\r\n") {
            Some((i, 4))
        } else if rest.starts_with(b"\n\n") {
            Some((i, 2))
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn anthropic_stream_totals_come_from_the_start_and_the_last_delta() {
        let mut usage = Usage::default();
        let mut watcher = EventWatcher::default();
        let stream = concat!(
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"model\":\"m1\",\"usage\":{\"input_tokens\":120,\"cache_read_input_tokens\":30,\"output_tokens\":1}}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"hi\"}}\n\n",
            "event: message_delta\ndata: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":55}}\n\n",
        );
        // Split mid-event to show events are joined across chunks.
        let (a, b) = stream.as_bytes().split_at(97);
        watcher.feed(ModelProvider::Anthropic, a, &mut usage);
        watcher.feed(ModelProvider::Anthropic, b, &mut usage);
        assert_eq!(usage.model.as_deref(), Some("m1"));
        assert_eq!(
            usage.tokens,
            Tokens {
                input: 120,
                output: 55,
                cache_read: 30,
                cache_write: 0
            }
        );
    }

    #[test]
    fn openai_counts_cached_tokens_apart_from_input() {
        let mut usage = Usage::default();
        usage.absorb(
            ModelProvider::Openai,
            &json!({"model":"m2","usage":{"prompt_tokens":1000,"completion_tokens":40,"prompt_tokens_details":{"cached_tokens":600}}}),
        );
        assert_eq!(
            usage.tokens,
            Tokens {
                input: 400,
                output: 40,
                cache_read: 600,
                cache_write: 0
            }
        );
        let mut responses = Usage::default();
        responses.absorb(
            ModelProvider::Openai,
            &json!({"type":"response.completed","response":{"model":"m2","usage":{"input_tokens":10,"output_tokens":5}}}),
        );
        assert_eq!(responses.tokens.input, 10);
        assert_eq!(responses.tokens.output, 5);
    }

    #[test]
    fn a_reply_with_no_usage_reports_none() {
        let mut usage = Usage::default();
        usage.absorb(ModelProvider::Openai, &json!({"data":[]}));
        let mut watcher = EventWatcher::default();
        watcher.feed(ModelProvider::Openai, b"data: [DONE]\r\n\r\n", &mut usage);
        assert!(!usage.seen);
    }
}
