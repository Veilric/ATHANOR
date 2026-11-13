use athanor::services::query_worker::QueryWorker;
use athanor_core::{cancel::Cancellation, feature::*, query::*};
use chrono::NaiveDate;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
struct Slow(Arc<AtomicBool>);
impl Feature for Slow {
    fn id(&self) -> &'static str {
        "test"
    }
    fn query(
        &self,
        request: &QueryRequest,
        _: &QueryContext,
        cancel: &Cancellation,
    ) -> FeatureReply {
        if request.text.as_ref() == "slow" {
            self.0.store(true, Ordering::SeqCst);
            for _ in 0..100 {
                if cancel.is_cancelled() {
                    return FeatureReply::NotApplicable;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        FeatureReply::Results(vec![ResultItem::answer(
            "test",
            "result",
            request.text.to_string(),
            "",
            request.text.to_string(),
        )])
    }
}
fn ctx() -> QueryContext {
    QueryContext {
        today: NaiveDate::from_ymd_opt(2026, 11, 11).unwrap(),
        angle_mode: AngleMode::Radians,
    }
}
fn wait(mut f: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(3);
    while !f() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(2));
    }
}
#[test]
fn a_new_query_supersedes_slow_work_and_cancellation_discards_late_results() {
    let started = Arc::new(AtomicBool::new(false));
    let engine = Arc::new(QueryEngine::new(vec![Arc::new(Slow(started.clone()))]).unwrap());
    let worker = QueryWorker::new(engine, Arc::new(|| {}));
    worker.submit(
        QueryRequest {
            generation: 1,
            text: Arc::from("slow"),
        },
        ctx(),
    );
    wait(|| started.load(Ordering::SeqCst));
    worker.submit(
        QueryRequest {
            generation: 2,
            text: Arc::from("fast"),
        },
        ctx(),
    );
    let mut answer = None;
    wait(|| {
        answer = worker.take_latest();
        answer.is_some()
    });
    let answer = answer.unwrap();
    assert_eq!(answer.generation, 2);
    assert_eq!(answer.items[0].title, "fast");
    worker.submit(
        QueryRequest {
            generation: 3,
            text: Arc::from("slow"),
        },
        ctx(),
    );
    worker.cancel();
    std::thread::sleep(Duration::from_millis(30));
    assert!(worker.take_latest().is_none());
}
