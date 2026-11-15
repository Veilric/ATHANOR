use athanor_core::feature::*;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
struct FailingHost;
impl ActionHost for FailingHost {
    fn copy_text(&self, _: &str) -> Result<(), ActionError> {
        Ok(())
    }
    fn open(&self, _: &OpenTarget) -> Result<(), ActionError> {
        Err(ActionError("The opener rejected the target".into()))
    }
    fn reveal(&self, _: &std::path::Path) -> Result<(), ActionError> {
        Err(ActionError("The opener rejected the target".into()))
    }
}
#[test]
fn background_action_completion_reports_failure_with_its_original_generation() {
    let worker =
        athanor::services::action_worker::ActionWorker::new(Arc::new(FailingHost), Arc::new(|| {}));
    worker
        .submit(
            42,
            Arc::new(OpenAction(OpenTarget::Path("/missing".into()))),
        )
        .unwrap();
    let until = Instant::now() + Duration::from_secs(2);
    let completed = loop {
        if let Some(done) = worker.take_latest() {
            break done;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(completed.generation, 42);
    assert!(completed.result.is_err());
}
#[test]
fn another_action_cannot_replace_an_unconsumed_completion() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let notified = Arc::new(AtomicBool::new(false));
    let ready = notified.clone();
    let worker = athanor::services::action_worker::ActionWorker::new(
        Arc::new(FailingHost),
        Arc::new(move || {
            ready.store(true, Ordering::Release);
        }),
    );
    let action: Arc<dyn FeatureAction> = Arc::new(OpenAction(OpenTarget::Path("/missing".into())));
    worker.submit(1, action.clone()).unwrap();
    let end = Instant::now() + Duration::from_secs(2);
    while !notified.load(Ordering::Acquire) {
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(worker.submit(2, action).is_err());
    assert_eq!(worker.take_latest().unwrap().generation, 1);
}
#[cfg(unix)]
#[test]
fn a_started_helper_that_exits_unsuccessfully_is_not_a_successful_open() {
    // Linux and macOS both provide /bin/sh, with a deterministic exit builtin.
    let mut helper = std::process::Command::new("/bin/sh");
    helper.args(["-c", "exit 1"]);
    let error = athanor::platform::launch_helper(&mut helper, Duration::from_secs(1)).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::Other);
    assert!(error.to_string().contains("rejected the target"));
    let mut helper = std::process::Command::new("/bin/sh");
    helper.args(["-c", "exit 0"]);
    assert!(athanor::platform::launch_helper(&mut helper, Duration::from_secs(1)).is_ok());
}
