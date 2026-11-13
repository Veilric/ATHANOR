use athanor_core::feature::{ActionError, ActionHost, ActionOutcome, FeatureAction};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
pub struct Completion {
    pub generation: u64,
    pub result: Result<ActionOutcome, ActionError>,
}
struct Job {
    generation: u64,
    action: Arc<dyn FeatureAction>,
}
pub struct ActionWorker {
    sender: mpsc::SyncSender<Option<Job>>,
    completed: Arc<Mutex<Option<Completion>>>,
    busy: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl ActionWorker {
    pub fn new(host: Arc<dyn ActionHost>, changed: Arc<dyn Fn() + Send + Sync>) -> Self {
        let (sender, receiver) = mpsc::sync_channel::<Option<Job>>(1);
        let completed = Arc::new(Mutex::new(None));
        let output = completed.clone();
        let busy = Arc::new(AtomicBool::new(false));
        let thread = std::thread::Builder::new()
            .name("athanor-actions".into())
            .spawn(move || {
                while let Ok(Some(job)) = receiver.recv() {
                    let result = job.action.execute(host.as_ref());
                    *output.lock().unwrap_or_else(|p| p.into_inner()) = Some(Completion {
                        generation: job.generation,
                        result,
                    });
                    changed();
                }
            })
            .expect("Could not start the bounded action worker");
        Self {
            sender,
            completed,
            busy,
            thread: Some(thread),
        }
    }
    pub fn submit(
        &self,
        generation: u64,
        action: Arc<dyn FeatureAction>,
    ) -> Result<(), ActionError> {
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(ActionError("An action is already in progress".into()));
        }
        if self
            .sender
            .try_send(Some(Job { generation, action }))
            .is_err()
        {
            self.busy.store(false, Ordering::Release);
            return Err(ActionError("The action worker is unavailable".into()));
        }
        Ok(())
    }
    pub fn take_latest(&self) -> Option<Completion> {
        let result = self
            .completed
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        if result.is_some() {
            self.busy.store(false, Ordering::Release);
        }
        result
    }
}
impl Drop for ActionWorker {
    fn drop(&mut self) {
        let _ = self.sender.send(None);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
