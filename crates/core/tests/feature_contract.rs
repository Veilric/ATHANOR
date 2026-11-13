use athanor_core::{
    cancel::Cancellation,
    feature::*,
    query::{QueryContext, QueryEngine, QueryRequest},
};
use chrono::NaiveDate;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

struct Example(&'static str, Presentation);
impl Feature for Example {
    fn id(&self) -> &'static str {
        self.0
    }
    fn query(&self, _: &QueryRequest, _: &QueryContext, cancel: &Cancellation) -> FeatureReply {
        if cancel.is_cancelled() {
            return FeatureReply::NotApplicable;
        }
        FeatureReply::Results(vec![ResultItem {
            key: ResultKey {
                feature: self.0,
                local: "one".into(),
            },
            title: self.0.into(),
            detail: "Independent provider".into(),
            presentation: self.1,
            actions: vec![Arc::new(CustomAction)],
        }])
    }
}
struct CustomAction;
impl FeatureAction for CustomAction {
    fn label(&self) -> &str {
        "Copy special value"
    }
    fn execute(&self, host: &dyn ActionHost) -> Result<ActionOutcome, ActionError> {
        host.copy_text("a provider-owned action")?;
        Ok(ActionOutcome::completed(false))
    }
}
#[derive(Default)]
struct Host(Mutex<Vec<String>>);
impl ActionHost for Host {
    fn copy_text(&self, text: &str) -> Result<(), ActionError> {
        self.0.lock().unwrap().push(text.into());
        Ok(())
    }
    fn open(&self, _: &OpenTarget) -> Result<(), ActionError> {
        panic!("unexpected open")
    }
    fn reveal(&self, _: &Path) -> Result<(), ActionError> {
        panic!("unexpected reveal")
    }
}
fn context() -> QueryContext {
    QueryContext {
        today: NaiveDate::from_ymd_opt(2026, 11, 11).unwrap(),
        angle_mode: AngleMode::Radians,
    }
}
fn request() -> QueryRequest {
    QueryRequest {
        generation: 7,
        text: Arc::from("sample"),
    }
}

#[test]
fn a_new_feature_supplies_its_own_action_without_dispatch_changes() {
    let engine = QueryEngine::new(vec![Arc::new(Example("custom", Presentation::Answer))]).unwrap();
    let batch = engine.query(&request(), &context(), &Cancellation::default());
    let host = Host::default();
    batch.items[0].actions[0].execute(&host).unwrap();
    assert_eq!(*host.0.lock().unwrap(), ["a provider-owned action"]);
    assert_eq!(batch.generation, 7);
}

#[test]
fn explicit_answers_precede_search_and_order_is_stable() {
    let engine = QueryEngine::new(vec![
        Arc::new(Example("search", Presentation::File)),
        Arc::new(Example("calculator", Presentation::Answer)),
        Arc::new(Example("third-feature", Presentation::Answer)),
    ])
    .unwrap();
    for _ in 0..5 {
        let batch = engine.query(&request(), &context(), &Cancellation::default());
        let ids: Vec<_> = batch.items.iter().map(|x| x.key.feature).collect();
        assert_eq!(ids, ["calculator", "third-feature", "search"]);
    }
}

#[test]
fn duplicate_ids_are_rejected_and_cancelled_queries_return_no_results() {
    assert!(
        QueryEngine::new(vec![
            Arc::new(Example("same", Presentation::File)),
            Arc::new(Example("same", Presentation::Answer)),
        ])
        .is_err()
    );
    let engine = QueryEngine::new(vec![Arc::new(Example("only", Presentation::File))]).unwrap();
    let cancel = Cancellation::default();
    cancel.cancel();
    assert!(
        engine
            .query(&request(), &context(), &cancel)
            .items
            .is_empty()
    );
}

#[test]
fn batches_cannot_apply_to_another_generation_or_hidden_panel() {
    let engine = QueryEngine::new(vec![Arc::new(Example("only", Presentation::File))]).unwrap();
    let batch = engine.query(&request(), &context(), &Cancellation::default());
    assert!(batch.can_apply(7, true));
    assert!(!batch.can_apply(8, true));
    assert!(!batch.can_apply(7, false));
}

struct Ambiguous;
impl Feature for Ambiguous {
    fn id(&self) -> &'static str {
        "ambiguous"
    }
    fn query(&self, _: &QueryRequest, _: &QueryContext, _: &Cancellation) -> FeatureReply {
        FeatureReply::Results(vec![
            ResultItem::explanation("ambiguous", "Choose a definition", "Select an option"),
            ResultItem::answer("ambiguous", "us", "US result", "", "1"),
            ResultItem::answer("ambiguous", "imperial", "Imperial result", "", "2"),
        ])
    }
}
#[test]
fn clarification_stays_before_its_choices_even_when_search_is_registered_first() {
    let engine = QueryEngine::new(vec![
        Arc::new(Example("search", Presentation::File)),
        Arc::new(Ambiguous),
    ])
    .unwrap();
    let batch = engine.query(&request(), &context(), &Cancellation::default());
    assert_eq!(batch.items[0].title, "Choose a definition");
    assert!(batch.items[0].actions.is_empty());
    assert_eq!(batch.items[1].title, "US result");
}
