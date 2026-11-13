use athanor_core::{
    cancel::Cancellation, feature::*, features::calculator::CalculatorFeature,
    number::format_number, query::*,
};
use chrono::NaiveDate;
use std::sync::Arc;
fn query(s: &str, mode: AngleMode) -> FeatureReply {
    CalculatorFeature::new().query(
        &QueryRequest {
            generation: 1,
            text: Arc::from(s),
        },
        &QueryContext {
            today: NaiveDate::from_ymd_opt(2026, 11, 11).unwrap(),
            angle_mode: mode,
        },
        &Cancellation::default(),
    )
}
fn answer(s: &str, mode: AngleMode) -> String {
    match query(s, mode) {
        FeatureReply::Results(x) => x[0].title.clone(),
        _ => panic!("no answer for {s}"),
    }
}
#[test]
fn scientific_expressions_have_documented_precedence() {
    assert_eq!(answer("sqrt(144) + 2^5", AngleMode::Radians), "44");
    assert_eq!(answer("2^3^2", AngleMode::Radians), "512");
    assert_eq!(answer("-2^2", AngleMode::Radians), "-4");
    assert_eq!(answer("sin(30)", AngleMode::Degrees), "0.5");
    assert_eq!(answer("log(100)", AngleMode::Radians), "2");
    assert_eq!(answer("2 × 3", AngleMode::Radians), "6");
}
#[test]
fn formatting_avoids_artifacts_and_preserves_tiny_values() {
    assert_eq!(answer("0.1+0.2", AngleMode::Radians), "0.3");
    let tiny = format_number(1e-12).unwrap();
    assert_ne!(tiny, "0");
    assert!(tiny.contains("e-12"));
    assert!(format_number(f64::INFINITY).is_err());
}
#[test]
fn incomplete_invalid_and_search_queries_are_distinct() {
    assert!(matches!(
        query("2 +", AngleMode::Radians),
        FeatureReply::Incomplete
    ));
    assert!(matches!(
        query("sqrt(", AngleMode::Radians),
        FeatureReply::Incomplete
    ));
    assert!(matches!(
        query("1/0", AngleMode::Radians),
        FeatureReply::Invalid { .. }
    ));
    assert!(matches!(
        query("sqrt(-1)", AngleMode::Radians),
        FeatureReply::Invalid { .. }
    ));
    for s in [
        "project",
        "finance+reports.md",
        "2026",
        "2026-11-11 + 45 days",
        "12 km in mi",
    ] {
        assert!(
            matches!(query(s, AngleMode::Radians), FeatureReply::NotApplicable),
            "{s}"
        );
    }
    assert!(matches!(
        query(
            &format!("{}1{}", "(".repeat(65), ")".repeat(65)),
            AngleMode::Radians
        ),
        FeatureReply::Invalid { .. }
    ));
}
