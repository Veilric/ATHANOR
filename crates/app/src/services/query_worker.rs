use athanor_core::{
    cancel::Cancellation,
    query::{QueryBatch, QueryContext, QueryEngine, QueryRequest},
};
use std::{
    collections::HashMap,
    sync::{Arc, Condvar, Mutex},
    thread::JoinHandle,
};
struct Job {
    serial: u64,
    request: QueryRequest,
    context: QueryContext,
    cancel: Cancellation,
}
#[derive(Default)]
struct Work {
    pending: Option<Job>,
    active: HashMap<u64, Cancellation>,
    serial: u64,
    stopped: bool,
}
struct Shared {
    work: Mutex<Work>,
    ready: Condvar,
    completed: Mutex<Option<QueryBatch>>,
}
pub struct QueryWorker {
    shared: Arc<Shared>,
    threads: Vec<JoinHandle<()>>,
}
impl QueryWorker {
    pub fn new(engine: Arc<QueryEngine>, changed: Arc<dyn Fn() + Send + Sync>) -> Self {
        let shared = Arc::new(Shared {
            work: Mutex::new(Work::default()),
            ready: Condvar::new(),
            completed: Mutex::new(None),
        });
        let threads = (0..2)
            .map(|index| {
                let shared = shared.clone();
                let engine = engine.clone();
                let changed = changed.clone();
                std::thread::Builder::new()
                    .name(format!("athanor-query-{index}"))
                    .spawn(move || {
                        loop {
                            let job = {
                                let mut work =
                                    shared.work.lock().unwrap_or_else(|p| p.into_inner());
                                while work.pending.is_none() && !work.stopped {
                                    work =
                                        shared.ready.wait(work).unwrap_or_else(|p| p.into_inner());
                                }
                                if work.stopped {
                                    return;
                                }
                                let Some(job) = work.pending.take() else {
                                    continue;
                                };
                                work.active.insert(job.serial, job.cancel.clone());
                                job
                            };
                            let batch = engine.query(&job.request, &job.context, &job.cancel);
                            let mut work = shared.work.lock().unwrap_or_else(|p| p.into_inner());
                            work.active.remove(&job.serial);
                            if !job.cancel.is_cancelled()
                                && work.serial == job.serial
                                && !work.stopped
                            {
                                *shared.completed.lock().unwrap_or_else(|p| p.into_inner()) =
                                    Some(batch);
                                drop(work);
                                changed();
                            }
                        }
                    })
                    .expect("Could not start a bounded query worker")
            })
            .collect();
        Self { shared, threads }
    }
    pub fn submit(&self, request: QueryRequest, context: QueryContext) {
        let mut work = self.shared.work.lock().unwrap_or_else(|p| p.into_inner());
        for cancel in work.active.values() {
            cancel.cancel();
        }
        if let Some(old) = work.pending.take() {
            old.cancel.cancel();
        }
        work.serial = work.serial.wrapping_add(1);
        let serial = work.serial;
        work.pending = Some(Job {
            serial,
            request,
            context,
            cancel: Cancellation::default(),
        });
        *self
            .shared
            .completed
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = None;
        self.shared.ready.notify_one();
    }
    pub fn cancel(&self) {
        let mut work = self.shared.work.lock().unwrap_or_else(|p| p.into_inner());
        for cancel in work.active.values() {
            cancel.cancel();
        }
        if let Some(old) = work.pending.take() {
            old.cancel.cancel();
        }
        work.serial = work.serial.wrapping_add(1);
        *self
            .shared
            .completed
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = None;
    }
    pub fn take_latest(&self) -> Option<QueryBatch> {
        self.shared
            .completed
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take()
    }
}
impl Drop for QueryWorker {
    fn drop(&mut self) {
        self.cancel();
        {
            let mut work = self.shared.work.lock().unwrap_or_else(|p| p.into_inner());
            work.stopped = true;
            self.shared.ready.notify_all();
        }
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}
