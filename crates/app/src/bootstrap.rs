use crate::{
    config::{AppPaths, Config},
    services::{
        file_index::FileIndexService, query_worker::QueryWorker, rate_http::HttpRates,
        rate_store::RateStore,
    },
};
use athanor_core::{
    feature::Feature,
    features::{
        calculator::CalculatorFeature,
        calendar::CalendarFeature,
        currency::{CurrencyFeature, RateSnapshot, Rates},
        help::HelpFeature,
        measurements::MeasurementFeature,
        search::{SearchFeature, SearchSnapshot},
    },
    query::{QueryContext, QueryEngine},
};
use chrono::{Local, NaiveDate, Utc};
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicU64, Ordering},
};

pub struct Services {
    pub engine: Arc<QueryEngine>,
    pub worker: Option<QueryWorker>,
    pub snapshot: Arc<RwLock<Arc<SearchSnapshot>>>,
    pub revision: Arc<AtomicU64>,
    pub index: Option<FileIndexService>,
    pub rates: Arc<dyn Rates>,
    changed: Arc<dyn Fn() + Send + Sync>,
}
fn engine(snapshot: Arc<RwLock<Arc<SearchSnapshot>>>, rates: Arc<dyn Rates>) -> Arc<QueryEngine> {
    let features: Vec<Arc<dyn Feature>> = vec![
        Arc::new(HelpFeature::new()),
        Arc::new(CalendarFeature::new()),
        Arc::new(MeasurementFeature::new()),
        Arc::new(CurrencyFeature::new(rates)),
        Arc::new(CalculatorFeature::new()),
        Arc::new(SearchFeature::new(snapshot)),
    ];
    Arc::new(QueryEngine::new(features).expect("Built-in feature identities are unique"))
}
impl Services {
    pub fn real(config: &Config, paths: &AppPaths, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        let revision = Arc::new(AtomicU64::new(0));
        let revision_changed = revision.clone();
        let wake_changed = wake.clone();
        let changed: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            revision_changed.fetch_add(1, Ordering::Release);
            wake_changed();
        });
        let snapshot = Arc::new(RwLock::new(Arc::new(SearchSnapshot::default())));
        let rates: Arc<dyn Rates> = Arc::new(RateStore::new(
            paths.rates.clone(),
            Arc::new(HttpRates::default()),
            Arc::new(Utc::now),
            changed.clone(),
        ));
        let engine = engine(snapshot.clone(), rates.clone());
        let worker = Some(QueryWorker::new(engine.clone(), wake));
        let index = Some(FileIndexService::start(
            config.search_roots.clone(),
            paths.index.clone(),
            snapshot.clone(),
            changed.clone(),
        ));
        Self {
            engine,
            worker,
            snapshot,
            revision,
            index,
            rates,
            changed,
        }
    }
    pub fn preview() -> Self {
        let snapshot = Arc::new(RwLock::new(Arc::new(SearchSnapshot::default())));
        let rates: Arc<dyn Rates> = Arc::new(OfflineRates);
        let engine = engine(snapshot.clone(), rates.clone());
        Self {
            engine,
            worker: None,
            snapshot,
            revision: Arc::new(AtomicU64::new(0)),
            index: None,
            rates,
            changed: Arc::new(|| {}),
        }
    }
    pub fn restart_index(&mut self, config: &Config, paths: &AppPaths) {
        self.index.take();
        self.index = Some(FileIndexService::start(
            config.search_roots.clone(),
            paths.index.clone(),
            self.snapshot.clone(),
            self.changed.clone(),
        ));
    }
}
struct OfflineRates;
impl Rates for OfflineRates {
    fn snapshot(&self, _: &str, _: &str) -> Option<RateSnapshot> {
        None
    }
    fn request_refresh(&self, _: &str, _: &str) {}
    fn error(&self, _: &str, _: &str) -> Option<String> {
        Some("Offline — connect to fetch the first reference rate.".into())
    }
}
pub fn context(config: &Config, preview: bool) -> QueryContext {
    QueryContext {
        today: if preview {
            NaiveDate::from_ymd_opt(2026, 11, 11).expect("Valid fixed preview date")
        } else {
            Local::now().date_naive()
        },
        angle_mode: config.angle_mode,
    }
}
