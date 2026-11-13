use athanor_core::{
    cancel::Cancellation, feature::*, features::calendar::CalendarFeature, query::*,
};
use chrono::NaiveDate;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
#[derive(Default)]
struct Host(Mutex<String>);
impl ActionHost for Host {
    fn copy_text(&self, s: &str) -> Result<(), ActionError> {
        *self.0.lock().unwrap() = s.into();
        Ok(())
    }
    fn open(&self, _: &OpenTarget) -> Result<(), ActionError> {
        unreachable!()
    }
    fn reveal(&self, _: &Path) -> Result<(), ActionError> {
        unreachable!()
    }
}
fn query(s: &str) -> FeatureReply {
    CalendarFeature::new().query(
        &QueryRequest {
            generation: 1,
            text: Arc::from(s),
        },
        &QueryContext {
            today: NaiveDate::from_ymd_opt(2026, 11, 11).unwrap(),
            angle_mode: AngleMode::Radians,
        },
        &Cancellation::default(),
    )
}
fn copied(s: &str) -> String {
    let FeatureReply::Results(items) = query(s) else {
        panic!("no answer for {s}")
    };
    let host = Host::default();
    items[0].actions[0].execute(&host).unwrap();
    host.0.into_inner().unwrap()
}
#[test]
fn calendar_days_and_leap_dates_are_deterministic() {
    assert_eq!(copied("today + 45 days"), "2026-12-26");
    assert_eq!(copied("2024-03-01 - 1 day"), "2024-02-29");
    assert_eq!(copied("2026-03-07 + 2 days"), "2026-03-09");
    assert_eq!(copied("today"), "2026-11-11");
}
#[test]
fn differences_are_signed_and_exclude_the_start_date() {
    assert_eq!(copied("days between 2026-11-11 and 2026-12-25"), "44");
    assert_eq!(copied("days between 2026-12-25 and 2026-11-11"), "-44");
    assert_eq!(copied("days between 2026-11-11 and 2026-11-11"), "0");
}
#[test]
fn invalid_or_overflowing_dates_are_explained_without_catching_math() {
    assert!(matches!(
        query("2026-02-30 + 1 day"),
        FeatureReply::Invalid { .. }
    ));
    assert!(matches!(
        query("today + 18446744073709551615 days"),
        FeatureReply::Invalid { .. }
    ));
    assert!(matches!(query("today +"), FeatureReply::Incomplete));
    assert!(matches!(query("12 km in mi"), FeatureReply::NotApplicable));
    assert!(matches!(query("2+3*4"), FeatureReply::NotApplicable));
}

#[test]
fn unicode_search_text_is_not_sliced_at_a_byte_boundary() {
    assert!(matches!(
        query("雪雪雪雪project"),
        FeatureReply::NotApplicable
    ));
}
