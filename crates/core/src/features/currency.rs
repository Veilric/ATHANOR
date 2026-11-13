use crate::{cancel::Cancellation, feature::*, query::*};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateSnapshot {
    pub base: String,
    pub quote: String,
    pub rate: Decimal,
    pub observed_on: NaiveDate,
    pub fetched_at: DateTime<Utc>,
    pub source: String,
}
pub trait Rates: Send + Sync {
    fn snapshot(&self, base: &str, quote: &str) -> Option<RateSnapshot>;
    fn request_refresh(&self, base: &str, quote: &str);
    fn error(&self, _base: &str, _quote: &str) -> Option<String> {
        None
    }
}
pub struct CurrencyFeature {
    rates: Arc<dyn Rates>,
}
impl CurrencyFeature {
    pub fn new(rates: Arc<dyn Rates>) -> Self {
        Self { rates }
    }
}
impl Feature for CurrencyFeature {
    fn id(&self) -> &'static str {
        "currency"
    }
    fn query(
        &self,
        request: &QueryRequest,
        context: &QueryContext,
        cancel: &Cancellation,
    ) -> FeatureReply {
        if cancel.is_cancelled() {
            return FeatureReply::NotApplicable;
        }
        let Some((amount, base, quote)) = pair_parts(&request.text) else {
            return FeatureReply::NotApplicable;
        };
        let Ok(amount) = Decimal::from_str_exact(amount) else {
            return FeatureReply::Invalid {
                message:
                    "Use a decimal-point amount within 28 decimal places and the supported range."
                        .into(),
            };
        };
        if base == quote {
            let value = format!("{} {quote}", amount.normalize());
            return FeatureReply::Results(vec![ResultItem::answer(
                self.id(),
                "identity",
                value.clone(),
                "Same currency · no exchange rate needed",
                value,
            )]);
        }
        self.rates.request_refresh(&base, &quote);
        let Some(rate) = self.rates.snapshot(&base, &quote) else {
            return FeatureReply::Results(vec![ResultItem::explanation(
                self.id(),
                "No cached exchange rate",
                self.rates.error(&base, &quote).unwrap_or_else(|| {
                    "Fetching reference data. A connection is needed for the first conversion."
                        .into()
                }),
            )]);
        };
        if rate.rate <= Decimal::ZERO
            || rate.observed_on > context.today
            || rate.base != base
            || rate.quote != quote
        {
            return FeatureReply::Invalid {
                message: "The cached rate is invalid. Refresh before converting.".into(),
            };
        }
        let Some(value) = amount.checked_mul(rate.rate) else {
            return FeatureReply::Invalid {
                message: "The amount exceeds the supported precision.".into(),
            };
        };
        if amount != Decimal::ZERO && value == Decimal::ZERO {
            return FeatureReply::Invalid {
                message: "The conversion is smaller than the supported decimal precision.".into(),
            };
        }
        let stale = context
            .today
            .signed_duration_since(rate.observed_on)
            .num_days()
            >= 3;
        let mut detail = format!(
            "{} · {} · {}",
            rate.source,
            rate.observed_on,
            if stale {
                "Stale cached rate"
            } else {
                "Cached reference rate"
            }
        );
        if let Some(error) = self.rates.error(&base, &quote) {
            detail.push_str(&format!(" · {error}"));
        }
        let copy = format!("{} {quote}", value.normalize());
        FeatureReply::Results(vec![ResultItem::answer(
            self.id(),
            format!("{base}-{quote}"),
            copy.clone(),
            detail,
            copy,
        )])
    }
}
pub fn parse_pair(query: &str) -> Option<(Decimal, String, String)> {
    let (amount, base, quote) = pair_parts(query)?;
    Some((Decimal::from_str_exact(amount).ok()?, base, quote))
}
fn pair_parts(query: &str) -> Option<(&str, String, String)> {
    let words: Vec<_> = query.split_whitespace().collect();
    let (amount, base, separator, quote) = match words.as_slice() {
        [amount, base, separator, quote] => (*amount, *base, *separator, *quote),
        [lhs, separator, quote] if lhs.len() > 3 => (
            lhs.get(..lhs.len() - 3)?,
            lhs.get(lhs.len() - 3..)?,
            *separator,
            *quote,
        ),
        _ => return None,
    };
    if !separator.eq_ignore_ascii_case("to") && !separator.eq_ignore_ascii_case("in") {
        return None;
    }
    let base = currency_code(base)?;
    let quote = currency_code(quote)?;
    Some((amount, base, quote))
}
pub fn currency_code(input: &str) -> Option<String> {
    if input.len() != 3 || !input.bytes().all(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    let code = input.to_ascii_uppercase();
    if ["BTC", "ETH", "SOL", "XRP", "XAU", "XAG", "XPT", "XPD"].contains(&code.as_str()) {
        return None;
    }
    Some(code)
}
