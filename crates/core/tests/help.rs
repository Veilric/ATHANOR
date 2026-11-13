use athanor_core::{
    cancel::Cancellation,
    feature::*,
    features::{
        calculator::CalculatorFeature,
        calendar::CalendarFeature,
        currency::{CurrencyFeature, RateSnapshot, Rates},
        help::{HelpFeature, TOPICS},
        measurements::MeasurementFeature,
        search::{SearchEntry, SearchFeature, SearchSnapshot},
    },
    query::{QueryContext, QueryEngine, QueryRequest},
};
use chrono::{NaiveDate, TimeZone, Utc};
use std::{
    path::Path,
    sync::{
        Arc, RwLock,
        atomic::{AtomicUsize, Ordering},
    },
};

fn context() -> QueryContext {
    QueryContext {
        today: NaiveDate::from_ymd_opt(2026, 11, 14).unwrap(),
        angle_mode: AngleMode::Radians,
    }
}
fn request(text: &str) -> QueryRequest {
    QueryRequest {
        generation: 7,
        text: Arc::from(text),
    }
}
fn help(text: &str) -> Vec<ResultItem> {
    let engine = QueryEngine::new(vec![Arc::new(HelpFeature::new())]).unwrap();
    engine
        .query(&request(text), &context(), &Cancellation::default())
        .items
}
struct NoSideEffects;
impl ActionHost for NoSideEffects {
    fn copy_text(&self, _: &str) -> Result<(), ActionError> {
        panic!("help must not touch the clipboard")
    }
    fn open(&self, _: &OpenTarget) -> Result<(), ActionError> {
        panic!("help must not open applications")
    }
    fn reveal(&self, _: &Path) -> Result<(), ActionError> {
        panic!("help must not reveal paths")
    }
}
#[test]
fn root_help_lists_every_topic_and_actions_navigate_to_the_exact_command() {
    let items = help("help");
    assert_eq!(items.len(), 5);
    for topic in TOPICS {
        let item = items.iter().find(|item| item.title == topic.title).unwrap();
        let outcome = item.actions[0].execute(&NoSideEffects).unwrap();
        assert_eq!(
            outcome.input,
            Some(QueryInput {
                text: format!("help {}", topic.command),
                submit: true
            })
        );
    }
}
#[test]
fn topic_examples_are_staged_without_host_side_effects() {
    for topic in TOPICS {
        let items = help(&format!("help {}", topic.command));
        assert!(items.iter().any(|item| item.title == "Syntax"));
        assert!(items.iter().any(|item| item.title == "Limitations"));
        for example in topic.examples {
            let item = items.iter().find(|item| item.title == *example).unwrap();
            assert_eq!(
                item.actions[0].execute(&NoSideEffects).unwrap().input,
                Some(QueryInput {
                    text: example.to_string(),
                    submit: false
                })
            );
        }
    }
}
struct OrdinaryFeature(Arc<AtomicUsize>);
impl Feature for OrdinaryFeature {
    fn id(&self) -> &'static str {
        "ordinary"
    }
    fn query(&self, _: &QueryRequest, _: &QueryContext, _: &Cancellation) -> FeatureReply {
        self.0.fetch_add(1, Ordering::Relaxed);
        FeatureReply::Results(vec![ResultItem::answer(
            "ordinary",
            "test",
            "search hit",
            "",
            "search hit",
        )])
    }
}
#[test]
fn help_reserves_its_namespace_before_search_and_handles_unknown_topics_offline() {
    let calls = Arc::new(AtomicUsize::new(0));
    let engine = QueryEngine::new(vec![
        Arc::new(HelpFeature::new()),
        Arc::new(OrdinaryFeature(calls.clone())),
    ])
    .unwrap();
    for text in [
        "help",
        "help calendar",
        " HELP CURRENCY ",
        "help missing",
        "help calendar extra",
    ] {
        let batch = engine.query(&request(text), &context(), &Cancellation::default());
        assert!(!batch.items.is_empty());
        assert!(batch.items.iter().all(|item| item.key.feature == "help"));
    }
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    for text in ["helper", "help.txt", "a help document"] {
        engine.query(&request(text), &context(), &Cancellation::default());
    }
    assert_eq!(calls.load(Ordering::Relaxed), 3);
}
struct ReferenceRates;
impl Rates for ReferenceRates {
    fn snapshot(&self, base: &str, quote: &str) -> Option<RateSnapshot> {
        Some(RateSnapshot {
            base: base.into(),
            quote: quote.into(),
            rate: 2.into(),
            observed_on: context().today,
            fetched_at: Utc.with_ymd_and_hms(2026, 11, 14, 0, 0, 0).unwrap(),
            source: "Test reference".into(),
        })
    }
    fn request_refresh(&self, _: &str, _: &str) {}
}
#[test]
fn every_advertised_example_is_accepted_by_its_existing_feature() {
    let entries = [
        ("Firefox", OpenTarget::Application("test:firefox".into())),
        (
            "Project notes.md",
            OpenTarget::Path("/Documents/Project notes.md".into()),
        ),
    ]
    .into_iter()
    .map(|(title, target)| SearchEntry {
        id: title.into(),
        title: title.into(),
        location: title.into(),
        target,
        is_folder: false,
    })
    .collect();
    let features: Vec<(&str, Arc<dyn Feature>)> = vec![
        ("calculator", Arc::new(CalculatorFeature::new())),
        ("units", Arc::new(MeasurementFeature::new())),
        ("calendar", Arc::new(CalendarFeature::new())),
        (
            "currency",
            Arc::new(CurrencyFeature::new(Arc::new(ReferenceRates))),
        ),
        (
            "search",
            Arc::new(SearchFeature::new(Arc::new(RwLock::new(Arc::new(
                SearchSnapshot::new(entries),
            ))))),
        ),
    ];
    for topic in TOPICS {
        let feature = &features
            .iter()
            .find(|(name, _)| *name == topic.command)
            .unwrap()
            .1;
        for example in topic.examples {
            let reply = feature.query(&request(example), &context(), &Cancellation::default());
            assert!(
                matches!(reply,FeatureReply::Results(items) if !items.is_empty()),
                "unsupported example {example}"
            );
        }
    }
}
#[test]
fn help_respects_cancellation() {
    let cancel = Cancellation::default();
    cancel.cancel();
    let engine = QueryEngine::new(vec![Arc::new(HelpFeature::new())]).unwrap();
    assert!(
        engine
            .query(&request("help"), &context(), &cancel)
            .items
            .is_empty()
    );
}
