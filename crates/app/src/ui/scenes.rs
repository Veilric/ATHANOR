use super::AppState;
use athanor_core::{
    feature::OpenTarget,
    features::search::{SearchEntry, SearchSnapshot},
    panel::PanelEvent,
};
use std::{sync::Arc, time::Duration};
fn query(s: &mut AppState, q: &str) {
    s.transition(PanelEvent::Invoke);
    s.transition(PanelEvent::QueryChanged(q.into()));
}
type Scene = (&'static str, fn(&mut AppState));
pub const SCENES: &[Scene] = &[
    ("help", |s| query(s, "help")),
    ("help-calculator", |s| query(s, "help calculator")),
    ("help-currency", |s| query(s, "help currency")),
    ("calculator", |s| query(s, "sqrt(144) + 2^5")),
    ("units", |s| query(s, "12 km in mi")),
    ("calendar", |s| query(s, "2026-11-11 + 45 days")),
    ("currency-offline", |s| query(s, "100 usd to php")),
    ("error", |s| {
        query(s, "2+2");
        s.transition(PanelEvent::ActionFailed(
            "The clipboard is temporarily unavailable. Try again.".into(),
        ));
    }),
    ("pinned-idle", |s| {
        s.transition(PanelEvent::Invoke);
        s.transition(PanelEvent::Pin(true));
        s.tick(Duration::from_secs(3));
    }),
    ("settings", |s| {
        s.transition(PanelEvent::Invoke);
        s.show_settings();
    }),
    ("search", |s| {
        let entries = ["Project notes.md", "Project brief.pdf", "Projects"]
            .into_iter()
            .map(|name| SearchEntry {
                id: name.into(),
                title: name.into(),
                location: format!("/Documents/Studio/{name}").into(),
                target: OpenTarget::Path(format!("/Documents/Studio/{name}").into()),
                is_folder: name == "Projects",
            })
            .collect();
        *s.services
            .snapshot
            .write()
            .unwrap_or_else(|p| p.into_inner()) = Arc::new(SearchSnapshot::new(entries));
        query(s, "project");
    }),
];
