use std::process::Command;
#[test]
fn query_mode_uses_the_same_features_and_returns_structured_results() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("config.json"), br#"{"search_roots":[]}"#).unwrap();
    for (query, expected) in [
        ("sqrt(144)+2^5", "44"),
        ("32 F to C", "0 °C"),
        ("2026-11-11 + 45 days", "Dec 26, 2026"),
        ("help", "Calculator"),
        ("help calendar", "Calendar"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_athanor"))
            .args(["--query", query, "--json"])
            .env("ATHANOR_DATA_DIR", dir.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["items"][0]["title"], expected);
    }
}
