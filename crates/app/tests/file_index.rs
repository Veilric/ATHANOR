use athanor::services::file_index::FileIndexService;
use athanor_core::features::search::SearchSnapshot;
use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
fn wait(mut f: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(4);
    while !f() {
        assert!(Instant::now() < end, "index did not update");
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn filesystem_changes_refresh_the_shared_index_and_shutdown_does_not_poll() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("files");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("first.txt"), "").unwrap();
    let snapshot = Arc::new(RwLock::new(Arc::new(SearchSnapshot::default())));
    let service = FileIndexService::start(
        vec![root.clone()],
        dir.path().join("index.jsonl"),
        snapshot.clone(),
        Arc::new(|| {}),
    );
    wait(|| {
        snapshot
            .read()
            .unwrap()
            .entries
            .iter()
            .any(|e| e.title == "first.txt")
    });
    std::fs::write(root.join("second.txt"), "").unwrap();
    wait(|| {
        snapshot
            .read()
            .unwrap()
            .entries
            .iter()
            .any(|e| e.title == "second.txt")
    });
    let begin = Instant::now();
    drop(service);
    assert!(begin.elapsed() < Duration::from_secs(1));
}
#[test]
fn reading_indexed_directories_does_not_trigger_an_endless_rescan() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("files");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("first.txt"), "").unwrap();
    let snapshot = Arc::new(RwLock::new(Arc::new(SearchSnapshot::default())));
    let revisions = Arc::new(AtomicUsize::new(0));
    let count = revisions.clone();
    let service = FileIndexService::start(
        vec![root.clone()],
        dir.path().join("index.jsonl"),
        snapshot,
        Arc::new(move || {
            count.fetch_add(1, Ordering::Relaxed);
        }),
    );
    wait(|| revisions.load(Ordering::Relaxed) > 0);
    std::thread::sleep(Duration::from_millis(1000));
    let before = revisions.load(Ordering::Relaxed);
    for _ in 0..10 {
        let _ = std::fs::read_dir(&root).unwrap().count();
    }
    std::thread::sleep(Duration::from_millis(1000));
    assert_eq!(
        revisions.load(Ordering::Relaxed),
        before,
        "access events must not rebuild the index"
    );
    drop(service);
}
#[test]
fn relative_cache_inside_an_indexed_root_does_not_rescan_itself() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let cwd = std::env::current_dir().unwrap();
    let dir = tempfile::tempdir_in(&cwd).unwrap();
    let cache = dir
        .path()
        .strip_prefix(&cwd)
        .unwrap()
        .join("data/index.jsonl");
    let snapshot = Arc::new(RwLock::new(Arc::new(SearchSnapshot::default())));
    let revisions = Arc::new(AtomicUsize::new(0));
    let count = revisions.clone();
    let service = FileIndexService::start(
        vec![dir.path().to_owned()],
        cache,
        snapshot,
        Arc::new(move || {
            count.fetch_add(1, Ordering::Relaxed);
        }),
    );
    wait(|| revisions.load(Ordering::Relaxed) > 0);
    std::thread::sleep(Duration::from_millis(800));
    let before = revisions.load(Ordering::Relaxed);
    std::thread::sleep(Duration::from_millis(900));
    assert_eq!(
        revisions.load(Ordering::Relaxed),
        before,
        "cache writes must not cause new scans"
    );
    drop(service);
}
