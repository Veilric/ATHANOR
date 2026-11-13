use super::rate_store::{RateTransport, valid_rate};
use athanor_core::features::currency::{RateSnapshot, currency_code};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use std::{collections::HashSet, io::Read, str::FromStr, time::Duration};

pub struct HttpRates {
    agent: ureq::Agent,
}
impl Default for HttpRates {
    fn default() -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(5)))
                .build()
                .into(),
        }
    }
}
impl HttpRates {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        let mut response = self
            .agent
            .get(url)
            .header("User-Agent", "Athanor/0.1")
            .call()
            .map_err(|_| "Offline or reference-rate service unavailable".to_string())?;
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(256 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Reference-rate request timed out or was interrupted".to_string())?;
        if bytes.len() > 256 * 1024 {
            return Err("Reference-rate response exceeds the allowed size".into());
        }
        Ok(bytes)
    }
}
impl RateTransport for HttpRates {
    fn currencies(&self) -> Result<HashSet<String>, String> {
        let bytes = self.get("https://api.frankfurter.dev/v2/currencies")?;
        let values: Vec<Value> =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid currency catalog".to_string())?;
        Ok(values
            .iter()
            .filter(|v| v["type"] == "currency")
            .filter_map(|v| v["iso_code"].as_str().and_then(currency_code))
            .collect())
    }
    fn fetch(&self, base: &str, quote: &str, now: DateTime<Utc>) -> Result<RateSnapshot, String> {
        if currency_code(base).as_deref() != Some(base)
            || currency_code(quote).as_deref() != Some(quote)
        {
            return Err("Use supported three-letter fiat currency codes".into());
        }
        let bytes = self.get(&format!(
            "https://api.frankfurter.dev/v2/rate/{base}/{quote}?expand=providers"
        ))?;
        parse_pair_response(&bytes, base, quote, now)
    }
}
pub fn parse_pair_response(
    bytes: &[u8],
    base: &str,
    quote: &str,
    now: DateTime<Utc>,
) -> Result<RateSnapshot, String> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| "Invalid rate response".to_string())?;
    let failure = || "Invalid reference-rate data".to_string();
    let raw = value["rate"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value["rate"].to_string());
    let rate = Decimal::from_str(&raw)
        .or_else(|_| Decimal::from_scientific(&raw))
        .map_err(|_| failure())?;
    let snapshot = RateSnapshot {
        base: value["base"].as_str().ok_or_else(failure)?.into(),
        quote: value["quote"].as_str().ok_or_else(failure)?.into(),
        rate,
        observed_on: value["date"]
            .as_str()
            .ok_or_else(failure)?
            .parse()
            .map_err(|_| failure())?,
        fetched_at: now,
        source: "Frankfurter blended".into(),
    };
    if snapshot.base != base || snapshot.quote != quote || !valid_rate(&snapshot, now) {
        return Err(failure());
    }
    Ok(snapshot)
}
