use athanor::config::Config;
#[test]
fn defaults_and_customizations_roundtrip_without_changing_opacity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let (mut config, warning) = Config::load(&path);
    assert!(warning.is_none());
    assert_eq!(config.opacity, 0.65);
    config.shortcut = "ctrl+shift+space".into();
    config.save(&path).unwrap();
    let (read, warning) = Config::load(&path);
    assert!(warning.is_none());
    assert_eq!(read.opacity, 0.65);
    assert_eq!(read.shortcut, "ctrl+shift+space");
    assert!(!read.start_at_login);
}
#[test]
fn corrupt_configuration_is_not_deleted_or_silently_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    std::fs::write(&path, b"{broken settings").unwrap();
    let (config, warning) = Config::load(&path);
    assert_eq!(config.opacity, 0.65);
    assert!(warning.is_some());
    assert_eq!(std::fs::read(&path).unwrap(), b"{broken settings");
}
#[test]
fn saving_after_corrupt_settings_preserves_a_recovery_copy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    std::fs::write(&path, b"{my unrecovered settings").unwrap();
    let (config, _) = Config::load(&path);
    config.save(&path).unwrap();
    assert!(
        std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .any(|entry| entry.path() != path
                && std::fs::read(entry.path())
                    .is_ok_and(|data| data == b"{my unrecovered settings"))
    );
    assert!(Config::load(&path).1.is_none());
}
