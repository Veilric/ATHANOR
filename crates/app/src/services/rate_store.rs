use athanor_core::{
    features::currency::{RateSnapshot, Rates, currency_code},
    storage::atomic_write_with,
};
use chrono::{DateTime, Duration, Utc};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
};

pub trait RateTransport: Send + Sync {
    fn currencies(&self) -> Result<HashSet<String>, String>;
    fn fetch(&self, base: &str, quote: &str, now: DateTime<Utc>) -> Result<RateSnapshot, String>;
}
pub type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;
type Pair = (String, String);
#[derive(Default)]
struct Entry {
    rate: Option<RateSnapshot>,
    attempted: Option<DateTime<Utc>>,
    pending: bool,
    error: Option<String>,
}
struct Inner {
    entries: Mutex<BTreeMap<Pair, Entry>>,
    clock: Clock,
    path: PathBuf,
}
pub struct RateStore {
    inner: Arc<Inner>,
    sender: mpsc::SyncSender<Pair>,
}
impl RateStore {
    pub fn new(
        path: PathBuf,
        transport: Arc<dyn RateTransport>,
        clock: Clock,
        changed: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        let now = clock();
        let cached: Vec<RateSnapshot> = fs::metadata(&path)
            .ok()
            .filter(|m| m.len() <= 256 * 1024)
            .and_then(|_| fs::read(&path).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let entries = cached
            .into_iter()
            .filter(|r| valid_rate(r, now))
            .take(128)
            .map(|rate| {
                (
                    (rate.base.clone(), rate.quote.clone()),
                    Entry {
                        rate: Some(rate),
                        ..Default::default()
                    },
                )
            })
            .collect();
        let inner = Arc::new(Inner {
            entries: Mutex::new(entries),
            clock,
            path,
        });
        let (sender, receiver) = mpsc::sync_channel::<Pair>(16);
        let worker = inner.clone();
        let _ = std::thread::Builder::new()
            .name("athanor-rates".into())
            .spawn(move || {
                let mut catalog: Option<(DateTime<Utc>, HashSet<String>)> = None;
                while let Ok((base, quote)) = receiver.recv() {
                    let now = (worker.clock)();
                    let result = (|| {
                        if catalog.as_ref().is_none_or(|(at, _)| {
                            now.signed_duration_since(*at) >= Duration::days(30)
                        }) {
                            catalog = Some((now, transport.currencies()?));
                        }
                        let supported = &catalog.as_ref().ok_or("Currency catalog unavailable")?.1;
                        if !supported.contains(&base) || !supported.contains(&quote) {
                            return Err("Unsupported fiat currency pair".into());
                        }
                        let mut rate = transport.fetch(&base, &quote, now)?;
                        if !valid_rate(&rate, now) || rate.base != base || rate.quote != quote {
                            return Err("Rate service returned invalid reference data".into());
                        }
                        rate.fetched_at = now;
                        Ok(rate)
                    })();
                    let success = result.is_ok();
                    {
                        let mut entries = worker.entries.lock().unwrap_or_else(|p| p.into_inner());
                        let entry = entries.entry((base.clone(), quote.clone())).or_default();
                        entry.pending = false;
                        match result {
                            Ok(rate) => {
                                entry.rate = Some(rate);
                                entry.error = None;
                            }
                            Err(error) => entry.error = Some(error),
                        }
                    }
                    if success {
                        let rates: Vec<_> = worker
                            .entries
                            .lock()
                            .unwrap_or_else(|p| p.into_inner())
                            .values()
                            .filter_map(|e| e.rate.clone())
                            .collect();
                        let saved = atomic_write_with(&worker.path, |file| {
                            serde_json::to_writer(file, &rates).map_err(std::io::Error::other)
                        });
                        if saved.is_err()
                            && let Some(entry) = worker
                                .entries
                                .lock()
                                .unwrap_or_else(|p| p.into_inner())
                                .get_mut(&(base, quote))
                        {
                            entry.error =
                                Some("Rate updated, but the local cache could not be saved".into());
                        }
                    }
                    changed();
                }
            });
        Self { inner, sender }
    }
}
impl Rates for RateStore {
    fn snapshot(&self, base: &str, quote: &str) -> Option<RateSnapshot> {
        self.inner
            .entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&(base.into(), quote.into()))
            .and_then(|e| e.rate.clone())
    }
    fn request_refresh(&self, base: &str, quote: &str) {
        let (Some(base), Some(quote)) = (currency_code(base), currency_code(quote)) else {
            return;
        };
        let key = (base, quote);
        let now = (self.inner.clock)();
        {
            let mut entries = self.inner.entries.lock().unwrap_or_else(|p| p.into_inner());
            if entries.len() >= 128 && !entries.contains_key(&key) {
                if let Some(old) = entries
                    .iter()
                    .find(|(_, e)| !e.pending)
                    .map(|(k, _)| k.clone())
                {
                    entries.remove(&old);
                } else {
                    return;
                }
            }
            let entry = entries.entry(key.clone()).or_default();
            if entry.pending
                || entry
                    .rate
                    .as_ref()
                    .is_some_and(|r| now.signed_duration_since(r.fetched_at) < Duration::hours(24))
                || entry
                    .attempted
                    .is_some_and(|at| now.signed_duration_since(at) < Duration::seconds(60))
            {
                return;
            }
            entry.pending = true;
            entry.attempted = Some(now);
        }
        if self.sender.try_send(key.clone()).is_err()
            && let Some(entry) = self
                .inner
                .entries
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .get_mut(&key)
        {
            entry.pending = false;
            entry.error =
                Some("Reference-rate refresh is busy or unavailable; try again shortly".into());
        }
    }
    fn error(&self, base: &str, quote: &str) -> Option<String> {
        self.inner
            .entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&(base.into(), quote.into()))
            .and_then(|e| e.error.clone())
    }
}
pub fn valid_rate(rate: &RateSnapshot, now: DateTime<Utc>) -> bool {
    rate.rate > rust_decimal::Decimal::ZERO
        && rate.observed_on <= now.date_naive()
        && rate.fetched_at <= now + Duration::minutes(1)
        && currency_code(&rate.base).as_deref() == Some(rate.base.as_str())
        && currency_code(&rate.quote).as_deref() == Some(rate.quote.as_str())
}
