use athanor_core::{
    cancel::Cancellation, feature::*, features::measurements::MeasurementFeature, query::*,
};
use chrono::NaiveDate;
use std::sync::Arc;
fn query(s: &str) -> FeatureReply {
    MeasurementFeature::new().query(
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
fn title(s: &str) -> String {
    match query(s) {
        FeatureReply::Results(x) => x[0].title.clone(),
        _ => panic!("no answer for {s}"),
    }
}
#[test]
fn representative_dimensions_and_affine_temperatures_are_correct() {
    assert!(title("12 km in mi").starts_with("7.45645"));
    assert_eq!(title("32 F to C"), "0 °C");
    assert_eq!(title("-40 c to f"), "-40 °F");
    assert_eq!(title("1 m2 to cm2"), "10000 cm²");
    assert_eq!(title("1 l to ml"), "1000 mL");
    assert_eq!(title("36 km/h in m/s"), "10 m/s");
    assert_eq!(title("2 days to hours"), "48 h");
    assert_eq!(title("1 kg to g"), "1000 g");
}
#[test]
fn binary_and_decimal_storage_are_distinct() {
    assert_eq!(title("1 MiB to KiB"), "1024 KiB");
    assert_eq!(title("1 MB to B"), "1000000 B");
    assert_eq!(title("1 MiB to B"), "1048576 B");
    assert_eq!(title("8 bits to bytes"), "1 B");
}
#[test]
fn ambiguous_gallons_require_an_explicit_choice() {
    let FeatureReply::Results(items) = query("1 gal in l") else {
        panic!("missing clarification")
    };
    assert!(items[0].actions.is_empty());
    assert!(items.iter().any(|x| x.title.contains("3.785")));
    assert!(items.iter().any(|x| x.title.contains("4.546")));
    assert_ne!(title("1 usgal to l"), title("1 impgal to l"));
}
#[test]
fn invalid_dimensions_and_locale_ambiguity_are_not_guessed() {
    assert!(matches!(query("1 kg to km"), FeatureReply::Invalid { .. }));
    assert!(matches!(
        query("1,5 km in mi"),
        FeatureReply::Invalid { .. }
    ));
    assert!(matches!(
        query("project notes"),
        FeatureReply::NotApplicable
    ));
    assert!(matches!(
        query("100 usd to php"),
        FeatureReply::NotApplicable
    ));
}
#[test]
fn nonzero_values_never_silently_underflow_to_zero() {
    for input in ["1e-999 m in km", "1e-320 mm to km"] {
        assert!(
            matches!(query(input), FeatureReply::Invalid { .. }),
            "{input} must report its range limit"
        );
    }
    assert_eq!(title("0 mm to km"), "0 km");
}
#[test]
fn scaled_bits_and_bytes_keep_their_case_sensitive_meaning() {
    assert_eq!(title("1 kb to B"), "125 B");
    assert_eq!(title("1 Mb to MB"), "0.125 MB");
    assert_eq!(title("1 Kib to B"), "128 B");
    assert_eq!(title("1 kB to B"), "1000 B");
}
