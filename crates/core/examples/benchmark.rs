//! Repeatable local fixture and search benchmark; never touches user directories.
use athanor_core::{
    cancel::Cancellation,
    feature::{AngleMode, Feature},
    features::search::{SearchFeature, build_file_snapshot, write_cache},
    query::{QueryContext, QueryRequest},
};
use chrono::NaiveDate;
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
    time::Instant,
};
fn main() {
    let root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .expect("Pass an empty benchmark fixture directory");
    std::fs::create_dir_all(&root).unwrap();
    let begin = Instant::now();
    for group in 0..100 {
        let folder = root.join(format!("batch-{group:03}"));
        std::fs::create_dir_all(&folder).unwrap();
        for item in 0..1000 {
            let path = folder.join(format!("research-report-{group:03}-{item:04}.txt"));
            if !path.exists() {
                std::fs::write(path, []).unwrap();
            }
        }
    }
    let fixture_ms = begin.elapsed().as_secs_f64() * 1000.;
    let begin = Instant::now();
    let snapshot =
        build_file_snapshot(std::slice::from_ref(&root), &Cancellation::default()).unwrap();
    let indexing_ms = begin.elapsed().as_secs_f64() * 1000.;
    let count = snapshot.entries.len();
    let cache = root.parent().unwrap().join("benchmark-index.jsonl");
    let begin = Instant::now();
    write_cache(&cache, &snapshot).unwrap();
    let cache_ms = begin.elapsed().as_secs_f64() * 1000.;
    let feature = SearchFeature::new(Arc::new(RwLock::new(Arc::new(snapshot))));
    let context = QueryContext {
        today: NaiveDate::from_ymd_opt(2026, 11, 12).unwrap(),
        angle_mode: AngleMode::Radians,
    };
    let mut timings = Vec::new();
    for i in 0..60 {
        let request = QueryRequest {
            generation: i,
            text: Arc::from(if i % 2 == 0 {
                "research report 042"
            } else {
                "rpt 0123"
            }),
        };
        let begin = Instant::now();
        let result = feature.query(&request, &context, &Cancellation::default());
        std::hint::black_box(result);
        if i >= 10 {
            timings.push(begin.elapsed().as_secs_f64() * 1000.);
        }
    }
    timings.sort_by(f64::total_cmp);
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let memory = |field: &str| {
        status
            .lines()
            .find(|line| line.starts_with(field))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|value| value.parse::<u64>().ok())
    };
    println!(
        "{}",
        serde_json::json!({"entries":count,"fixture_ms":fixture_ms,"indexing_ms":indexing_ms,"cache_write_ms":cache_ms,"cache_bytes":std::fs::metadata(cache).unwrap().len(),"query_samples":timings.len(),"query_p95_ms":timings[47],"query_median_ms":timings[25],"rss_kib":memory("VmRSS:"),"peak_rss_kib":memory("VmHWM:"),"root":root})
    );
}
