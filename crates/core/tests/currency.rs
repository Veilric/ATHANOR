use athanor_core::{cancel::Cancellation, feature::*, features::currency::*, query::*};
use chrono::{NaiveDate, TimeZone, Utc};
use rust_decimal::Decimal;
use std::{
    str::FromStr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
struct Store {
    data: Mutex<Option<RateSnapshot>>,
    refreshes: AtomicUsize,
}
impl Rates for Store {
    fn snapshot(&self, _: &str, _: &str) -> Option<RateSnapshot> {
        self.data.lock().unwrap().clone()
    }
    fn request_refresh(&self, _: &str, _: &str) {
        self.refreshes.fetch_add(1, Ordering::Relaxed);
    }
}
fn rate(value: &str, date: (i32, u32, u32)) -> RateSnapshot {
    RateSnapshot {
        base: "USD".into(),
        quote: "PHP".into(),
        rate: Decimal::from_str(value).unwrap(),
        observed_on: NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap(),
        fetched_at: Utc.with_ymd_and_hms(2026, 11, 11, 12, 0, 0).unwrap(),
        source: "Frankfurter blended".into(),
    }
}
fn query(feature: &CurrencyFeature, text: &str) -> FeatureReply {
    feature.query(
        &QueryRequest {
            generation: 1,
            text: Arc::from(text),
        },
        &QueryContext {
            today: NaiveDate::from_ymd_opt(2026, 11, 11).unwrap(),
            angle_mode: AngleMode::Radians,
        },
        &Cancellation::default(),
    )
}
#[test]
fn cached_rate_keeps_its_source_date_and_uses_decimal_arithmetic() {
    let store = Arc::new(Store {
        data: Mutex::new(Some(rate("58.50", (2026, 11, 10)))),
        refreshes: AtomicUsize::new(0),
    });
    let feature = CurrencyFeature::new(store.clone());
    let FeatureReply::Results(items) = query(&feature, "100 usd to php") else {
        panic!("no conversion")
    };
    assert_eq!(items[0].title, "5850 PHP");
    assert!(items[0].detail.contains("2026-11-10"));
    assert!(items[0].detail.contains("Frankfurter"));
    assert_eq!(store.refreshes.load(Ordering::Relaxed), 1);
}
#[test]
fn absent_and_stale_rates_are_explained_without_fake_values() {
    let store = Arc::new(Store {
        data: Mutex::new(None),
        refreshes: AtomicUsize::new(0),
    });
    let feature = CurrencyFeature::new(store.clone());
    let FeatureReply::Results(items) = query(&feature, "100 usd to php") else {
        panic!("no explanation")
    };
    assert!(items[0].actions.is_empty());
    *store.data.lock().unwrap() = Some(rate("58.50", (2026, 11, 1)));
    let FeatureReply::Results(items) = query(&feature, "100 usd to php") else {
        panic!("no cached result")
    };
    assert!(items[0].detail.to_lowercase().contains("stale"));
}
#[test]
fn invalid_rates_and_out_of_scope_assets_never_become_answers() {
    for sample in [
        rate("0", (2026, 11, 10)),
        rate("-2", (2026, 11, 10)),
        rate("58", (2099, 1, 1)),
    ] {
        let feature = CurrencyFeature::new(Arc::new(Store {
            data: Mutex::new(Some(sample)),
            refreshes: AtomicUsize::new(0),
        }));
        assert!(matches!(
            query(&feature, "100 usd to php"),
            FeatureReply::Invalid { .. }
        ));
    }
    let feature = CurrencyFeature::new(Arc::new(Store {
        data: Mutex::new(None),
        refreshes: AtomicUsize::new(0),
    }));
    assert!(matches!(
        query(&feature, "100 btc to usd"),
        FeatureReply::NotApplicable
    ));
    assert!(matches!(
        query(&feature, "12 km in mi"),
        FeatureReply::NotApplicable
    ));
}
#[test]
fn unsupported_decimal_precision_and_multiplication_underflow_are_explained() {
    let feature = CurrencyFeature::new(Arc::new(Store {
        data: Mutex::new(Some(rate("0.01", (2026, 11, 10)))),
        refreshes: AtomicUsize::new(0),
    }));
    assert!(matches!(
        query(&feature, "0.00000000000000000000000000001 USD to USD"),
        FeatureReply::Invalid { .. }
    ));
    assert!(matches!(
        query(&feature, "0.0000000000000000000000000001 USD to PHP"),
        FeatureReply::Invalid { .. }
    ));
}
