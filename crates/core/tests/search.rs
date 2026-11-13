use athanor_core::{cancel::Cancellation, feature::*, features::search::*, query::*};
use chrono::NaiveDate;
use std::{
    fs,
    sync::{Arc, RwLock},
};
fn query(feature: &SearchFeature, text: &str) -> Vec<ResultItem> {
    let FeatureReply::Results(items) = feature.query(
        &QueryRequest {
            generation: 1,
            text: Arc::from(text),
        },
        &QueryContext {
            today: NaiveDate::from_ymd_opt(2026, 11, 11).unwrap(),
            angle_mode: AngleMode::Radians,
        },
        &Cancellation::default(),
    ) else {
        return vec![];
    };
    items
}
#[test]
fn combined_snapshots_share_entry_allocations() {
    let entry = Arc::new(SearchEntry {
        id: "sample".into(),
        title: "Sample".into(),
        location: "/sample".into(),
        target: OpenTarget::Path("/sample".into()),
        is_folder: false,
    });
    let snapshot = SearchSnapshot::from_shared(vec![entry.clone()]);
    assert!(Arc::ptr_eq(&entry, &snapshot.entries[0]));
}
#[test]
fn utf8_paths_use_compact_readable_cache_strings() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("雪.md"), "").unwrap();
    let snapshot = build_file_snapshot(&[dir.path().to_owned()], &Cancellation::default()).unwrap();
    let cache = dir.path().join("cache.jsonl");
    write_cache(&cache, &snapshot).unwrap();
    let text = fs::read_to_string(&cache).unwrap();
    let entry: serde_json::Value = serde_json::from_str(text.lines().nth(1).unwrap()).unwrap();
    assert!(entry["location"].is_string());
    assert_eq!(
        read_cache(&cache).unwrap().entries[0].location,
        snapshot.entries[0].location
    );
}
#[test]
fn indexing_excludes_hidden_build_and_symlink_cycles_and_queries_are_cached() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("Documents")).unwrap();
    fs::create_dir_all(dir.path().join("target")).unwrap();
    fs::write(
        dir.path().join("Documents/Project notes.md"),
        "content is not searched",
    )
    .unwrap();
    fs::write(dir.path().join("target/project-output"), "").unwrap();
    fs::write(dir.path().join(".project-secret"), "").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(dir.path(), dir.path().join("Documents/cycle")).unwrap();
    let snapshot = build_file_snapshot(&[dir.path().to_owned()], &Cancellation::default()).unwrap();
    let snapshot = Arc::new(RwLock::new(Arc::new(snapshot)));
    let feature = SearchFeature::new(snapshot.clone());
    let hits = query(&feature, "project");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "Project notes.md");
    fs::remove_file(dir.path().join("Documents/Project notes.md")).unwrap();
    assert_eq!(
        query(&feature, "project").len(),
        1,
        "queries use the metadata snapshot"
    );
    assert!(query(&feature, "content is not searched").is_empty());
}
#[test]
fn exact_prefix_and_fuzzy_order_is_stable_with_duplicate_names() {
    let entries = [
        ("project", "/a"),
        ("project notes", "/b"),
        ("a project", "/c"),
        ("product object", "/d"),
        ("project", "/z"),
    ];
    let feature = SearchFeature::new(Arc::new(RwLock::new(Arc::new(SearchSnapshot::new(
        entries
            .into_iter()
            .map(|(title, path)| {
                let path = std::path::PathBuf::from(path);
                SearchEntry {
                    id: path.to_string_lossy().into_owned(),
                    title: title.into(),
                    location: path.clone(),
                    target: OpenTarget::Path(path),
                    is_folder: false,
                }
            })
            .collect(),
    )))));
    let a = query(&feature, "project");
    let b = query(&feature, "project");
    assert_eq!(a[0].title, "project");
    assert!(a[0].detail.contains("/a"));
    assert_eq!(a[1].title, "project");
    assert_eq!(a[2].title, "project notes");
    assert_eq!(
        a.iter().map(|x| &x.key).collect::<Vec<_>>(),
        b.iter().map(|x| &x.key).collect::<Vec<_>>()
    );
    assert!(!query(&feature, "prj").is_empty());
}
#[test]
fn cancelled_or_missing_roots_do_not_crash() {
    let cancel = Cancellation::default();
    cancel.cancel();
    assert!(build_file_snapshot(&[std::path::PathBuf::from("/not-a-real-root")], &cancel).is_err());
    let snapshot = build_file_snapshot(
        &[std::path::PathBuf::from("/not-a-real-root")],
        &Cancellation::default(),
    )
    .unwrap();
    assert!(snapshot.entries.is_empty());
    assert!(!snapshot.warnings.is_empty());
}
#[cfg(unix)]
#[test]
fn non_utf8_paths_survive_cache_roundtrip_and_unicode_names_are_searchable() {
    use std::os::unix::ffi::OsStringExt;
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path()
            .join(std::ffi::OsString::from_vec(b"report-\xff.md".to_vec())),
        "",
    )
    .unwrap();
    fs::write(dir.path().join("雪の計画.md"), "").unwrap();
    let snapshot = build_file_snapshot(&[dir.path().to_owned()], &Cancellation::default()).unwrap();
    let cache = dir.path().join("cache.jsonl");
    write_cache(&cache, &snapshot).unwrap();
    let restored = read_cache(&cache).unwrap();
    assert_eq!(restored.entries.len(), 2);
    let feature = SearchFeature::new(Arc::new(RwLock::new(Arc::new(restored))));
    assert_eq!(query(&feature, "report").len(), 1);
    assert_eq!(query(&feature, "計画").len(), 1);
}
#[test]
fn query_of_one_hundred_thousand_entries_is_bounded() {
    let snapshot = SearchSnapshot::new(
        (0..100_000)
            .map(|n| SearchEntry {
                id: n.to_string(),
                title: format!("project-{n:06}.md"),
                location: format!("/docs/{n}").into(),
                target: OpenTarget::Path(format!("/docs/{n}").into()),
                is_folder: false,
            })
            .collect(),
    );
    let feature = SearchFeature::new(Arc::new(RwLock::new(Arc::new(snapshot))));
    let begin = std::time::Instant::now();
    let hits = query(&feature, "project");
    eprintln!("100k snapshot query: {:?}", begin.elapsed());
    assert_eq!(hits.len(), 50);
}
