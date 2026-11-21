//! The models a person may pick: what `GET /models` lists, and the check that an effort level
//! belongs to its model.
//!
//! Anthropic's and OpenAI's models come from the vendored LiteLLM table (the proxy's
//! `prices.json`: context window, tool support, reasoning levels and prices). OpenCode Zen's and
//! Go's come from their own model lists (`<url>/v1/models`, fetched without a key and kept for an
//! hour; a failed fetch keeps the last list). A provider's models are listed only when the
//! organization has a key for it.
//!
//! An effort level is an OpenCode model variant, so the levels here are the names OpenCode
//! uses: LiteLLM says whether a model reasons and whether it has `xhigh` and `max`; a Zen or Go
//! entry names its own levels, or reasons at the three every provider takes.

use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use antilog_types::{ModelInfo, ModelProvider};
use serde::Deserialize;
use tokio::sync::Mutex;

/// The vendored LiteLLM table, shared with the proxy (which prices calls from it).
const VENDORED: &str = include_str!("../../antilog-proxy/data/prices.json");

const REFRESH: Duration = Duration::from_secs(3600);
/// After a failed fetch, try again in this long rather than on every request.
const RETRY: Duration = Duration::from_secs(300);
/// What a reasoning model offers when its list names no levels.
const COMMON: [&str; 3] = ["low", "medium", "high"];

#[derive(Deserialize)]
struct Row {
    provider: String,
    input: u64,
    output: u64,
    #[serde(default)]
    context: u64,
    #[serde(default)]
    reasoning: Vec<String>,
    #[serde(default)]
    tools: bool,
}

/// "glm-5-3-flash" is "Glm 5.3 Flash", "o4-mini" is "O4 Mini", a leading "gpt" is "GPT".
fn pretty(id: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    let mut after_number = false;
    for part in id.split('-') {
        let number = !part.is_empty() && part.chars().all(|c| c.is_ascii_digit());
        if number && after_number {
            if let Some(last) = words.last_mut() {
                last.push('.');
                last.push_str(part);
            }
            continue;
        }
        after_number = number;
        words.push(match part {
            "gpt" => "GPT".to_owned(),
            _ if number => part.to_owned(),
            _ => {
                let mut chars = part.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                })
            }
        });
    }
    words.join(" ")
}

/// A dated snapshot ("-20250929", "-2025-08-07") of a model the table has undated.
fn is_snapshot(id: &str, table: &BTreeMap<String, Row>) -> bool {
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let stem = id.rsplit_once('-').and_then(|(stem, date)| {
        if digits(date) && date.len() == 8 {
            return Some(stem);
        }
        let (rest, month) = stem.rsplit_once('-')?;
        let (rest, year) = rest.rsplit_once('-')?;
        (digits(year) && year.len() == 4 && digits(month) && month.len() == 2 && digits(date))
            .then_some(rest)
    });
    stem.is_some_and(|stem| table.contains_key(stem))
}

/// Anthropic's and OpenAI's models that can call tools, from the vendored table.
fn litellm() -> &'static Vec<ModelInfo> {
    static LIST: OnceLock<Vec<ModelInfo>> = OnceLock::new();
    LIST.get_or_init(|| {
        let table: BTreeMap<String, Row> = serde_json::from_str(VENDORED).unwrap_or_default();
        table
            .iter()
            .filter(|(id, row)| row.tools && !is_snapshot(id, &table))
            .filter_map(|(id, row)| {
                let provider = ModelProvider::parse(&row.provider)?;
                Some(ModelInfo {
                    id: format!("{}/{id}", provider.as_str()),
                    name: pretty(id),
                    provider,
                    context_window: (row.context > 0).then_some(row.context),
                    input_price: Some(thousandths(row.input)),
                    output_price: Some(thousandths(row.output)),
                    reasoning_levels: row.reasoning.clone(),
                })
            })
            .collect()
    })
}

#[expect(clippy::cast_precision_loss, reason = "a price is a few digits")]
fn thousandths(value: u64) -> f64 {
    value as f64 / 1000.0
}

#[derive(Deserialize)]
struct ZenList {
    data: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct ZenModel {
    id: String,
    name: Option<String>,
    capabilities: Option<ZenCapabilities>,
    limit: Option<ZenLimit>,
    cost: Option<ZenCost>,
}

#[derive(Deserialize)]
struct ZenCapabilities {
    tools: Option<ZenSupported>,
    reasoning: Option<ZenReasoning>,
}

#[derive(Deserialize)]
struct ZenSupported {
    supported: bool,
}

#[derive(Deserialize)]
struct ZenReasoning {
    supported: bool,
    #[serde(default)]
    effort: Vec<String>,
}

#[derive(Deserialize)]
struct ZenLimit {
    context: Option<u64>,
    input: Option<u64>,
}

#[derive(Deserialize)]
struct ZenCost {
    text: Option<ZenText>,
}

#[derive(Deserialize)]
struct ZenText {
    input: ZenPrice,
    output: ZenPrice,
}

#[derive(Deserialize)]
struct ZenPrice {
    price: f64,
}

/// Reads a Zen or Go list. A model that cannot call tools is left out (an agent needs them), and so
/// is one whose entry does not parse.
pub fn parse_zen(provider: ModelProvider, body: &[u8]) -> Vec<ModelInfo> {
    let Ok(list) = serde_json::from_slice::<ZenList>(body) else {
        return Vec::new();
    };
    list.data
        .into_iter()
        .filter_map(|model| serde_json::from_value::<ZenModel>(model).ok())
        .filter_map(|model| {
            let capabilities = model.capabilities?;
            if !capabilities.tools.is_some_and(|tools| tools.supported) {
                return None;
            }
            let levels = match capabilities.reasoning {
                Some(reasoning) if reasoning.supported && reasoning.effort.is_empty() => {
                    COMMON.iter().map(|level| (*level).to_owned()).collect()
                }
                Some(reasoning) if reasoning.supported => reasoning.effort,
                _ => Vec::new(),
            };
            let text = model.cost.and_then(|cost| cost.text);
            Some(ModelInfo {
                id: format!("{}/{}", provider.as_str(), model.id),
                name: model.name.unwrap_or_else(|| pretty(&model.id)),
                provider,
                context_window: model.limit.and_then(|limit| limit.context.or(limit.input)),
                input_price: text.as_ref().map(|text| text.input.price),
                output_price: text.as_ref().map(|text| text.output.price),
                reasoning_levels: levels,
            })
        })
        .collect()
}

struct Cached {
    at: Option<Instant>,
    zen: Vec<ModelInfo>,
    go: Vec<ModelInfo>,
}

/// OpenCode Zen's and Go's model lists, fetched when asked for and at most once an hour.
pub struct ModelLists {
    zen_url: Option<String>,
    go_url: Option<String>,
    http: reqwest::Client,
    cache: Mutex<Cached>,
}

impl ModelLists {
    fn with(zen_url: Option<String>, go_url: Option<String>, cached: Cached) -> Self {
        Self {
            zen_url,
            go_url,
            http: reqwest::Client::new(),
            cache: Mutex::new(cached),
        }
    }

    /// The lists at `ANTILOG_OPENCODE_URL` and `ANTILOG_OPENCODE_GO_URL` (default: opencode.ai).
    pub fn from_env() -> Self {
        let url = |name: &str, default: &str| {
            Some(super::api::auth::setting(name).unwrap_or_else(|| default.to_owned()))
        };
        Self::with(
            url("ANTILOG_OPENCODE_URL", "https://opencode.ai/zen"),
            url("ANTILOG_OPENCODE_GO_URL", "https://opencode.ai/zen/go"),
            Cached {
                at: None,
                zen: Vec::new(),
                go: Vec::new(),
            },
        )
    }

    /// No Zen or Go list is fetched: only Anthropic's and OpenAI's models exist (tests).
    pub fn offline() -> Self {
        Self::with(
            None,
            None,
            Cached {
                at: None,
                zen: Vec::new(),
                go: Vec::new(),
            },
        )
    }

    /// Lists that are already loaded and never fetched again (tests).
    pub fn fixed(zen: &[u8], go: &[u8]) -> Self {
        Self::with(
            None,
            None,
            Cached {
                at: None,
                zen: parse_zen(ModelProvider::Opencode, zen),
                go: parse_zen(ModelProvider::OpencodeGo, go),
            },
        )
    }

    async fn fetch(&self, url: &str) -> Option<Vec<u8>> {
        let reply = self
            .http
            .get(format!("{}/v1/models", url.trim_end_matches('/')))
            .timeout(Duration::from_secs(20))
            .send()
            .await
            .ok()?;
        if !reply.status().is_success() {
            return None;
        }
        reply.bytes().await.ok().map(|body| body.to_vec())
    }

    /// Every model the vendored table and the cached lists know, for the providers in `only`
    /// (all of them when `None`), by provider then name.
    pub async fn models(&self, only: Option<&[ModelProvider]>) -> Vec<ModelInfo> {
        let mut cache = self.cache.lock().await;
        let wanted = |provider| only.is_none_or(|only| only.contains(&provider));
        let stale = cache.at.is_none_or(|at| at.elapsed() > REFRESH);
        if stale && (self.zen_url.is_some() || self.go_url.is_some()) {
            // A failed fetch keeps the last list and waits RETRY before the next try.
            let mut failed = false;
            if wanted(ModelProvider::Opencode)
                && let Some(url) = &self.zen_url
            {
                match self.fetch(url).await {
                    Some(body) => cache.zen = parse_zen(ModelProvider::Opencode, &body),
                    None => failed = true,
                }
            }
            if wanted(ModelProvider::OpencodeGo)
                && let Some(url) = &self.go_url
            {
                match self.fetch(url).await {
                    Some(body) => cache.go = parse_zen(ModelProvider::OpencodeGo, &body),
                    None => failed = true,
                }
            }
            let now = Instant::now();
            cache.at = Some(if failed {
                now.checked_sub(REFRESH - RETRY).unwrap_or(now)
            } else {
                now
            });
        }
        let mut all: Vec<ModelInfo> = litellm()
            .iter()
            .chain(&cache.zen)
            .chain(&cache.go)
            .filter(|model| wanted(model.provider))
            .cloned()
            .collect();
        all.sort_by(|a, b| {
            let order = |p: ModelProvider| ModelProvider::ALL.iter().position(|x| *x == p);
            order(a.provider)
                .cmp(&order(b.provider))
                .then_with(|| a.name.cmp(&b.name))
        });
        all
    }

    /// Whether `effort` is one of `model`'s levels. No effort is always fine. An effort for a
    /// model this server does not list, or that offers none, is not.
    pub async fn check_effort(
        &self,
        model: &str,
        effort: Option<&str>,
    ) -> Result<(), &'static str> {
        let Some(effort) = effort else {
            return Ok(());
        };
        let known = self.models(None).await;
        let Some(found) = known.iter().find(|info| info.id == model) else {
            return Err("an effort needs a model GET /models lists");
        };
        if found.reasoning_levels.iter().any(|level| level == effort) {
            Ok(())
        } else if found.reasoning_levels.is_empty() {
            Err("this model has no effort levels")
        } else {
            Err("that is not one of this model's effort levels")
        }
    }
}

/// How a level reads: "xhigh" is "Extra high".
pub fn level_label(level: &str) -> String {
    match level {
        "xhigh" => "Extra high".to_owned(),
        _ => {
            let mut chars = level.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_read_like_a_person_wrote_them() {
        assert_eq!(pretty("glm-5-3-flash"), "Glm 5.3 Flash");
        assert_eq!(pretty("gpt-oss"), "GPT Oss");
        assert_eq!(pretty("o4-mini"), "O4 Mini");
    }

    #[tokio::test]
    async fn an_effort_must_be_one_of_the_models_levels() {
        let lists = ModelLists::offline();
        let check = |model, effort| lists.check_effort(model, effort);
        assert!(check("openai/o3", None).await.is_ok());
        assert!(check("openai/o3", Some("high")).await.is_ok());
        assert!(check("openai/o3", Some("turbo")).await.is_err());
        assert!(check("openai/not-a-model", Some("high")).await.is_err());
        assert_eq!(level_label("xhigh"), "Extra high");
        assert_eq!(level_label("medium"), "Medium");
    }

    #[tokio::test]
    async fn litellm_models_carry_a_window_a_price_and_levels() {
        let lists = ModelLists::offline();
        let all = lists.models(Some(&[ModelProvider::Openai])).await;
        let o3 = all
            .iter()
            .find(|m| m.id == "openai/o3")
            .expect("o3 is listed");
        assert_eq!(o3.name, "O3");
        assert_eq!(o3.context_window, Some(200_000));
        assert_eq!(o3.input_price, Some(2.0));
        assert_eq!(o3.reasoning_levels, ["low", "medium", "high"]);
        // A dated snapshot is the same model: not listed twice.
        assert!(all.iter().all(|m| !m.id.ends_with("-2025-04-16")));
    }
}
