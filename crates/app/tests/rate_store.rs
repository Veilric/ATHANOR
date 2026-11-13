use athanor::services::{
    rate_http::parse_pair_response,
    rate_store::{RateStore, RateTransport},
};
use athanor_core::features::currency::{RateSnapshot, Rates};
use chrono::{DateTime, Duration, TimeZone, Utc};
use std::{
    collections::HashSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration as Wait, Instant},
};
struct Fake {
    calls: AtomicUsize,
    fail: Mutex<bool>,
}
impl RateTransport for Fake {
    fn currencies(&self) -> Result<HashSet<String>, String> {
        Ok(["USD".into(), "PHP".into()].into())
    }
    fn fetch(&self, base: &str, quote: &str, now: DateTime<Utc>) -> Result<RateSnapshot, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(Wait::from_millis(30));
        if *self.fail.lock().unwrap() {
            return Err("Offline".into());
        }
        parse_pair_response(
            br#"{"date":"2026-11-10","base":"USD","quote":"PHP","rate":58.50}"#,
            base,
            quote,
            now,
        )
    }
}
fn wait_until(mut done: impl FnMut() -> bool) {
    let end = Instant::now() + Wait::from_secs(3);
    while !done() {
        assert!(Instant::now() < end, "rate worker did not finish");
        std::thread::sleep(Wait::from_millis(5));
    }
}
#[test]
fn refresh_is_coalesced_cached_and_failed_refresh_preserves_good_data() {
    let dir = tempfile::tempdir().unwrap();
    let clock = Arc::new(Mutex::new(
        Utc.with_ymd_and_hms(2026, 11, 11, 12, 0, 0).unwrap(),
    ));
    let clock_read = clock.clone();
    let fake = Arc::new(Fake {
        calls: AtomicUsize::new(0),
        fail: Mutex::new(false),
    });
    let store = RateStore::new(
        dir.path().join("rates.json"),
        fake.clone(),
        Arc::new(move || *clock_read.lock().unwrap()),
        Arc::new(|| {}),
    );
    for _ in 0..20 {
        store.request_refresh("USD", "PHP");
    }
    wait_until(|| store.snapshot("USD", "PHP").is_some());
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    let first = store.snapshot("USD", "PHP").unwrap();
    assert_eq!(first.observed_on.to_string(), "2026-11-10");
    *clock.lock().unwrap() += Duration::hours(25);
    *fake.fail.lock().unwrap() = true;
    store.request_refresh("USD", "PHP");
    wait_until(|| store.error("USD", "PHP").is_some());
    assert_eq!(store.snapshot("USD", "PHP").unwrap().rate, first.rate);
    store.request_refresh("USD", "PHP");
    assert_eq!(
        fake.calls.load(Ordering::SeqCst),
        2,
        "failure cooldown prevents hammering the provider"
    );
}
#[test]
fn malformed_or_future_source_data_cannot_be_cached_as_a_real_rate() {
    let now = Utc.with_ymd_and_hms(2026, 11, 11, 12, 0, 0).unwrap();
    for body in [
        br#"{"date":"2026-11-10","base":"USD","quote":"PHP","rate":0}"#.as_slice(),
        br#"{"date":"2099-01-01","base":"USD","quote":"PHP","rate":58.5}"#,
        br#"{"date":"2026-11-10","base":"EUR","quote":"PHP","rate":58.5}"#,
        br#"not json"#,
    ] {
        assert!(parse_pair_response(body, "USD", "PHP", now).is_err());
    }
}
#[test]
fn corrupt_cache_is_preserved_and_unknown_pairs_are_not_sent_to_rate_endpoint() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rates.json");
    std::fs::write(&path, b"broken cache").unwrap();
    let fake = Arc::new(Fake {
        calls: AtomicUsize::new(0),
        fail: Mutex::new(false),
    });
    let now = Utc.with_ymd_and_hms(2026, 11, 11, 12, 0, 0).unwrap();
    let store = RateStore::new(
        path.clone(),
        fake.clone(),
        Arc::new(move || now),
        Arc::new(|| {}),
    );
    assert!(store.snapshot("USD", "PHP").is_none());
    assert_eq!(std::fs::read(&path).unwrap(), b"broken cache");
    store.request_refresh("USD", "XYZ");
    wait_until(|| store.error("USD", "XYZ").is_some());
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
}

#[test]
#[ignore = "opt-in public API contract check; needs a network connection"]
fn live_http_provider_supports_php_and_exposes_reference_metadata() {
    let http = athanor::services::rate_http::HttpRates::default();
    let catalog = http.currencies().unwrap();
    assert!(catalog.contains("PHP") && catalog.contains("USD"));
    assert!(!catalog.contains("BTC"));
    let snapshot = http.fetch("USD", "PHP", Utc::now()).unwrap();
    assert!(snapshot.rate > rust_decimal::Decimal::ZERO);
    assert!(!snapshot.source.is_empty());
}
