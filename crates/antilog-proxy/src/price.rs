//! What a model costs. The table is LiteLLM's price list, trimmed and vendored in
//! `data/prices.json` (MIT; see `data/prices.header.md` and the README for how it is refreshed),
//! with the model as the key. A model that is not in it is booked at the highest price in it, so an
//! unknown model never costs less than a known one.
//!
//! Prices are thousandths of a dollar per million tokens (3000 is $3.00 per million).

use std::collections::BTreeMap;
use std::sync::OnceLock;

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

/// The price of a model, and whether the table knew it.
pub fn lookup(provider: ModelProvider, model: Option<&str>) -> (Price, bool) {
    let found = |name: &str| {
        table()
            .get(name)
            .filter(|(p, _)| *p == provider)
            .map(|(_, price)| *price)
    };
    let known = model.and_then(|model| found(model).or_else(|| undated(model).and_then(found)));
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
        for provider in ModelProvider::ALL {
            let model = models(provider)[0];
            let (price, known) = lookup(provider, Some(model));
            assert!(known, "{model}");
            assert_eq!(
                lookup(provider, Some(&format!("{model}-20991231"))),
                (price, true)
            );
            assert_eq!(
                lookup(provider, Some(&format!("{model}-2099-12-31"))),
                (price, true)
            );
            // Under the other provider the same name is not known.
            let other = if provider == ModelProvider::Anthropic {
                ModelProvider::Openai
            } else {
                ModelProvider::Anthropic
            };
            assert!(!lookup(other, Some(model)).1);
        }
    }

    #[test]
    fn an_unknown_model_costs_the_most_the_table_has() {
        let (price, known) = lookup(ModelProvider::Anthropic, Some("no-such-model-9"));
        assert!(!known);
        assert_eq!(price, highest());
        assert_eq!(lookup(ModelProvider::Openai, None).0, highest());
        assert!(
            table()
                .values()
                .all(|(_, p)| p.output <= price.output && p.input <= price.input)
        );
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
