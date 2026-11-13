#[cfg(target_os = "linux")]
#[test]
fn desktop_exec_is_arguments_not_a_shell_and_field_codes_are_explicit() {
    use athanor::platform::linux::parse_exec;
    let args = parse_exec(
        r#"app --title "A name" %u %% %c"#,
        "Sample",
        None,
        "/tmp/sample.desktop",
    )
    .unwrap();
    assert_eq!(args, ["app", "--title", "A name", "%", "Sample"]);
    let args = parse_exec(
        r#"app "literal; $(whoami)" %i"#,
        "Sample",
        Some("icon"),
        "/tmp/sample.desktop",
    )
    .unwrap();
    assert_eq!(args, ["app", "literal; $(whoami)", "--icon", "icon"]);
    assert!(parse_exec("app %Z", "Sample", None, "/tmp/sample.desktop").is_err());
    assert!(parse_exec("app \"unterminated", "Sample", None, "/tmp/sample.desktop").is_err());
}
