//! What a model costs. The table is LiteLLM's price list, trimmed and vendored in
//! `data/prices.json` (MIT; see `data/prices.header.md` and the README for how it is refreshed),
//! with the model as the key. A model that is not in it is booked at the highest price in it, so an
//! unknown model never costs less than a known one. OpenCode Zen is the exception: its prices come
//! from its own model list (`ZenPrices`), loaded at start and refreshed every hour.
//!
//! Prices are thousandths of a dollar per million tokens (3000 is $3.00 per million).

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;

use antilog_types::ModelProvider;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Price {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

#[derive(Deserialize)]
struct Row {
    provider: String,
    input: u64,
    output: u64,
    cache_read: u64,
    cache_write: u64,
}

const DATA: &str = include_str!("../data/prices.json");

/// Used only if the vendored file cannot be read, which a test rules out: dear on purpose.
const FALLBACK: Price = Price {
    input: 100_000,
    output: 500_000,
    cache_read: 100_000,
    cache_write: 100_000,
};

fn table() -> &'static BTreeMap<String, (ModelProvider, Price)> {
    static TABLE: OnceLock<BTreeMap<String, (ModelProvider, Price)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let rows: BTreeMap<String, Row> = serde_json::from_str(DATA).unwrap_or_else(|error| {
            tracing::error!(%error, "the vendored price table is unreadable: every model is booked at the fallback price");
            BTreeMap::new()
        });
        rows.into_iter()
            .filter_map(|(model, row)| {
                let provider = ModelProvider::parse(&row.provider)?;
                Some((
                    model,
                    (
                        provider,
                        Price {
                            input: row.input,
                            output: row.output,
                            cache_read: row.cache_read,
                            cache_write: row.cache_write,
                        },
                    ),
                ))
            })
            .collect()
    })
}

/// The models the table knows for a provider.
pub fn models(provider: ModelProvider) -> Vec<&'static str> {
    table()
        .iter()
        .filter(|(_, (p, _))| *p == provider)
        .map(|(model, _)| model.as_str())
        .collect()
}

/// A dated snapshot ("-20250929" or "-2025-08-07") is priced as the model without its date.
fn undated(model: &str) -> Option<&str> {
    let (stem, date) = model.rsplit_once('-')?;
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    if digits(date) && date.len() == 8 {
        return Some(stem);
    }
    // 2025-08-07 style: three numeric parts.
    let (stem, day) = (stem, date);
    let (stem, month) = stem.rsplit_once('-')?;
    let (stem, year) = stem.rsplit_once('-')?;
    (digits(year)
        && year.len() == 4
        && digits(month)
        && month.len() == 2
        && digits(day)
        && day.len() == 2)
        .then_some(stem)
}

/// The highest of every price in the table, part by part.
pub fn highest() -> Price {
    table()
        .values()
        .fold(None::<Price>, |top, (_, p)| {
            Some(match top {
                None => *p,
                Some(top) => Price {
                    input: top.input.max(p.input),
                    output: top.output.max(p.output),
                    cache_read: top.cache_read.max(p.cache_read),
                    cache_write: top.cache_write.max(p.cache_write),
                },
            })
        })
        .unwrap_or(FALLBACK)
}

/// OpenCode Zen's prices, read from `GET <upstream>/v1/models`. Cloning shares the table.
///
/// The list is the same for any key, so it is fetched without one (the simpler choice: no
/// organization, no sealed key). If the endpoint ever asks for a key the fetch fails, the table
/// stays empty and every Zen model is booked at the highest price, which is safe for budgets.
///
/// Zen's list also carries `multipliers` (time-of-day surcharges). v0 ignores them: a call is
/// priced at the base rate, so a surcharged hour is booked a little low.
#[derive(Debug, Clone, Default)]
pub struct ZenPrices {
    table: Arc<RwLock<Option<BTreeMap<String, Price>>>>,
    failing: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Deserialize)]
struct ZenList {
    data: Vec<ZenModel>,
}

#[derive(Deserialize)]
struct ZenModel {
    id: String,
    cost: Option<ZenCost>,
}

#[derive(Deserialize)]
struct ZenCost {
    text: Option<ZenText>,
}

#[derive(Deserialize)]
struct ZenText {
    input: ZenSide,
    output: ZenSide,
}

#[derive(Deserialize)]
struct ZenSide {
    price: f64,
    cache: Option<ZenCache>,
}

#[derive(Deserialize)]
struct ZenCache {
    read: Option<f64>,
    write: Option<f64>,
}

/// US dollars per million tokens to thousandths of a dollar, rounded up.
fn thousandths(dollars: f64) -> u64 {
    // A float in a price list is a handful of decimals: the cast cannot lose a real value.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value = (dollars.max(0.0) * 1000.0 - 1e-6).ceil().max(0.0) as u64;
    value
}

/// Reads Zen's model list. A model with no text prices is left out, so it is unknown.
fn parse_zen(body: &[u8]) -> Result<BTreeMap<String, Price>, serde_json::Error> {
    let list: ZenList = serde_json::from_slice(body)?;
    Ok(list
        .data
        .into_iter()
        .filter_map(|model| {
            let text = model.cost?.text?;
            let input = thousandths(text.input.price);
            let cache = text.input.cache;
            // A missing cache price is the plain input price: never less than it costs.
            let cache_read = cache
                .as_ref()
                .and_then(|c| c.read)
                .map_or(input, thousandths);
            let cache_write = cache
                .as_ref()
                .and_then(|c| c.write)
                .map_or(input, thousandths);
            Some((
                model.id,
                Price {
                    input,
                    output: thousandths(text.output.price),
                    cache_read,
                    cache_write,
                },
            ))
        })
        .collect())
}

impl ZenPrices {
    fn get(&self, model: &str) -> Option<Price> {
        let table = self.table.read().unwrap_or_else(|e| e.into_inner());
        table.as_ref()?.get(model).copied()
    }

    /// Fetches the list from `<upstream>/v1/models` and replaces the table. On any failure the
    /// last table stays, and the first failure of a run of them is logged.
    pub async fn refresh(&self, http: &reqwest::Client, upstream: &str) {
        let url = format!("{}/v1/models", upstream.trim_end_matches('/'));
        let fetched = async {
            let reply = http
                .get(&url)
                .timeout(Duration::from_secs(20))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if !reply.status().is_success() {
                return Err(format!("status {}", reply.status()));
            }
            let body = reply.bytes().await.map_err(|e| e.to_string())?;
            parse_zen(&body).map_err(|e| e.to_string())
        }
        .await;
        match fetched {
            Ok(table) => {
                tracing::info!(models = table.len(), "loaded the Zen price table");
                *self.table.write().unwrap_or_else(|e| e.into_inner()) = Some(table);
                self.failing
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }
            Err(error) => {
                if !self
                    .failing
                    .swap(true, std::sync::atomic::Ordering::Relaxed)
                {
                    tracing::warn!(%error, "could not fetch the Zen model list: keeping the last prices, or the highest price if there are none");
                }
            }
        }
    }

    /// Loads now, then every `period`, until the task is dropped.
    pub async fn keep_fresh(self, http: reqwest::Client, upstream: String, period: Duration) {
        loop {
            self.refresh(&http, &upstream).await;
            tokio::time::sleep(period).await;
        }
    }
}

/// The price of a model, and whether the table knew it.
pub fn lookup(provider: ModelProvider, model: Option<&str>, zen: &ZenPrices) -> (Price, bool) {
    let known = if provider == ModelProvider::Opencode {
        model.and_then(|model| zen.get(model))
    } else {
        let found = |name: &str| {
            table()
                .get(name)
                .filter(|(p, _)| *p == provider)
                .map(|(_, price)| *price)
        };
        model.and_then(|model| found(model).or_else(|| undated(model).and_then(found)))
    };
    match known {
        Some(price) => (price, true),
        None => (highest(), false),
    }
}

/// What the tokens used.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tokens {
    /// Input that was not read from the cache.
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

impl Tokens {
    pub fn total(&self) -> u64 {
        self.input
            .saturating_add(self.output)
            .saturating_add(self.cache_read)
            .saturating_add(self.cache_write)
    }
}

/// Whole US cents, rounded up: a call never books less than it cost.
pub fn cents(price: Price, tokens: Tokens) -> u64 {
    let thousandths = [
        (tokens.input, price.input),
        (tokens.output, price.output),
        (tokens.cache_read, price.cache_read),
        (tokens.cache_write, price.cache_write),
    ]
    .iter()
    .map(|(count, per_million)| u128::from(*count) * u128::from(*per_million))
    .sum::<u128>();
    // thousandths of a dollar times tokens, per million tokens, is 1 / 10_000_000 of a cent.
    u64::try_from(thousandths.div_ceil(10_000_000)).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vendored_table_reads_and_has_both_providers() {
        assert!(!DATA.is_empty());
        assert!(!models(ModelProvider::Anthropic).is_empty());
        assert!(!models(ModelProvider::Openai).is_empty());
        assert!(highest() != FALLBACK);
    }

    #[test]
    fn a_known_model_has_its_price_and_a_dated_snapshot_has_the_same() {
        // OpenCode Zen has no rows of its own: its models are priced by rule.
        for provider in [ModelProvider::Anthropic, ModelProvider::Openai] {
            let model = models(provider)[0];
            let zen = ZenPrices::default();
            let (price, known) = lookup(provider, Some(model), &zen);
            assert!(known, "{model}");
            assert_eq!(
                lookup(provider, Some(&format!("{model}-20991231")), &zen),
                (price, true)
            );
            assert_eq!(
                lookup(provider, Some(&format!("{model}-2099-12-31")), &zen),
                (price, true)
            );
            // Under the other provider the same name is not known.
            let other = if provider == ModelProvider::Anthropic {
                ModelProvider::Openai
            } else {
                ModelProvider::Anthropic
            };
            assert!(!lookup(other, Some(model), &zen).1);
        }
    }

    #[test]
    fn an_unknown_model_costs_the_most_the_table_has() {
        let (price, known) = lookup(
            ModelProvider::Anthropic,
            Some("no-such-model-9"),
            &ZenPrices::default(),
        );
        assert!(!known);
        assert_eq!(price, highest());
        assert_eq!(
            lookup(ModelProvider::Openai, None, &ZenPrices::default()).0,
            highest()
        );
        assert!(
            table()
                .values()
                .all(|(_, p)| p.output <= price.output && p.input <= price.input)
        );
    }

    #[test]
    fn zen_prices_are_read_from_the_model_list() {
        let body = br#"{"data":[
            {"id":"a","cost":{"text":{"input":{"price":0.15,"cache":{"read":0.03,"write":0.2}},"output":{"price":0.5}}},"multipliers":[]},
            {"id":"b","cost":{"text":{"input":{"price":2},"output":{"price":8}}}},
            {"id":"free","cost":{"text":{"input":{"price":0},"output":{"price":0}}}},
            {"id":"none"}]}"#;
        let table = parse_zen(body).unwrap();
        assert_eq!(table.len(), 3);
        assert_eq!(
            table["a"],
            Price {
                input: 150,
                output: 500,
                cache_read: 30,
                cache_write: 200
            }
        );
        // No cache prices: the input price.
        assert_eq!(
            table["b"],
            Price {
                input: 2000,
                output: 8000,
                cache_read: 2000,
                cache_write: 2000
            }
        );
        assert_eq!(table["free"].output, 0);
    }

    #[test]
    fn dates_are_taken_off_only_when_they_are_dates() {
        assert_eq!(undated("m-20250929"), Some("m"));
        assert_eq!(undated("m-2025-08-07"), Some("m"));
        assert_eq!(undated("m-mini"), None);
        assert_eq!(undated("m-5"), None);
    }

    #[test]
    fn cost_rounds_up_to_a_cent_and_adds_the_parts() {
        let price = Price {
            input: 3_000,
            output: 15_000,
            cache_read: 300,
            cache_write: 3_750,
        };
        // 1M input tokens at $3 and 1M output at $15: $18.
        let million = Tokens {
            input: 1_000_000,
            output: 1_000_000,
            ..Tokens::default()
        };
        assert_eq!(cents(price, million), 1_800);
        // One token still costs a cent: it is never booked as free.
        assert_eq!(
            cents(
                price,
                Tokens {
                    input: 1,
                    ..Tokens::default()
                }
            ),
            1
        );
        assert_eq!(cents(price, Tokens::default()), 0);
        let cached = Tokens {
            cache_read: 1_000_000,
            cache_write: 1_000_000,
            ..Tokens::default()
        };
        assert_eq!(cents(price, cached), 405);
    }
}
