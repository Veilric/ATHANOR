//! Requested native options and observed desktop behavior are deliberately distinct.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelBackend {
    MacOSPanel,
    WindowsWindow,
    X11Window,
    LayerShell,
    GnomeXWayland,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckStatus {
    Unverified,
    Passed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityReport {
    pub backend: PanelBackend,
    pub typing: CheckStatus,
    pub alpha: CheckStatus,
    pub topmost: CheckStatus,
    pub focus: CheckStatus,
    pub shortcut: CheckStatus,
}

impl CapabilityReport {
    pub fn candidate(backend: PanelBackend) -> Self {
        Self {
            backend,
            typing: CheckStatus::Unverified,
            alpha: CheckStatus::Unverified,
            topmost: CheckStatus::Unverified,
            focus: CheckStatus::Unverified,
            shortcut: CheckStatus::Unverified,
        }
    }

    pub fn meets_overlay_requirements(&self) -> bool {
        self.backend != PanelBackend::Unsupported
            && [
                self.typing,
                self.alpha,
                self.topmost,
                self.focus,
                self.shortcut,
            ]
            .into_iter()
            .all(|status| status == CheckStatus::Passed)
    }
}
