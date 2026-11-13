use athanor::platform::capabilities::{CapabilityReport, CheckStatus, PanelBackend};

#[test]
fn requested_native_options_are_not_verified_capabilities() {
    let report = CapabilityReport::candidate(PanelBackend::X11Window);
    assert!(!report.meets_overlay_requirements());
}

#[test]
fn native_wayland_without_layer_shell_is_not_advertised_as_topmost() {
    let mut report = CapabilityReport::candidate(PanelBackend::Unsupported);
    report.typing = CheckStatus::Passed;
    report.alpha = CheckStatus::Passed;
    report.topmost = CheckStatus::Passed;
    report.focus = CheckStatus::Passed;
    report.shortcut = CheckStatus::Passed;
    assert!(!report.meets_overlay_requirements());
}

#[test]
fn every_required_observation_must_pass() {
    let mut report = CapabilityReport::candidate(PanelBackend::X11Window);
    report.typing = CheckStatus::Passed;
    report.alpha = CheckStatus::Passed;
    report.topmost = CheckStatus::Passed;
    report.focus = CheckStatus::Passed;
    assert!(!report.meets_overlay_requirements());
    report.shortcut = CheckStatus::Passed;
    assert!(report.meets_overlay_requirements());
    report.focus = CheckStatus::Failed;
    assert!(!report.meets_overlay_requirements());
}
