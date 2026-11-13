#![cfg(feature = "native-ui")]
use athanor::ui::{AppState, tide::tide_view};
use athanor_core::panel::PanelEvent;
use rune::{Env, Runtime};
use std::time::Duration;
fn runtime() -> Runtime<AppState> {
    let mut state = AppState::preview();
    state.transition(PanelEvent::Invoke);
    Runtime::new(
        state,
        tide_view,
        (330.0, 420.0),
        Env {
            system_dark: true,
            scale: 1.0,
        },
    )
}
#[test]
fn pin_button_empty_idle_and_typing_control_which_elements_exist() {
    let mut rt = runtime();
    rt.frame(0.0);
    let (x, y) = rt.find("pin").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.0);
    assert!(rt.state.panel.pinned);
    rt.update(|state, _| state.tick(Duration::from_millis(3000)));
    rt.frame(0.0);
    assert!(rt.find("query").is_some());
    assert!(rt.find("pin").is_some());
    assert!(rt.find("results").is_none());
    let before = rt.presented("query").unwrap().0[1];
    rt.update(|state, _| state.transition(PanelEvent::QueryChanged("sqrt(144)+2^5".into())));
    rt.frame(0.0);
    assert!(rt.find("results").is_some());
    assert_eq!(rt.state.items[0].title, "44");
    assert_eq!(before, rt.presented("query").unwrap().0[1]);
}

#[test]
fn clicking_the_unpinned_drag_handle_and_losing_focus_keeps_the_query_visible() {
    let mut rt = runtime();
    rt.update(|state, _| state.transition(PanelEvent::QueryChanged("2+2".into())));
    rt.frame(0.0);
    let anchor = rt.presented("query").unwrap().0;
    let (x, y) = rt.find("drag-handle").unwrap();
    rt.pointer_down(x, y);
    assert_eq!(rt.take_titlebar_action(), Some(rune::TitlebarAction::Drag));
    rt.pointer_up(x, y);
    rt.frame(0.0);
    rt.update(|state, _| state.transition(PanelEvent::FocusChanged(false)));
    rt.frame(0.0);
    assert!(rt.state.panel.visible);
    assert_eq!(rt.state.panel.query, "2+2");
    assert!(rt.find("query").is_some());
    assert_eq!(rt.presented("query").unwrap().0, anchor);
}

#[test]
fn help_navigation_stages_an_example_and_waits_for_enter_after_edits_and_refresh() {
    let mut rt = runtime();
    rt.update(|state, _| state.transition(PanelEvent::QueryChanged("help".into())));
    rt.frame(0.0);
    let category = rt
        .state
        .items
        .iter()
        .position(|item| item.title == "Calculator")
        .unwrap();
    let (x, y) = rt.find(&format!("result-{category}")).unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.0);
    assert_eq!(rt.state.panel.query, "help calculator");
    let example = rt
        .state
        .items
        .iter()
        .position(|item| item.title == "1+1")
        .unwrap();
    for _ in 0..example {
        rt.key(rune::KeyEvent {
            key: rune::Key::Down,
            mods: Default::default(),
        });
        rt.frame(0.0);
    }
    assert_eq!(rt.state.selected, example);
    rt.key(rune::KeyEvent {
        key: rune::Key::Enter,
        mods: Default::default(),
    });
    rt.frame(0.0);
    assert_eq!(rt.state.panel.query, "1+1");
    assert!(rt.state.items.is_empty());
    assert!(rt.state.copied_values().is_empty());
    rt.update(|state, _| {
        state.transition(PanelEvent::QueryChanged("1+2".into()));
        state.requery();
    });
    rt.frame(0.0);
    assert!(rt.state.items.is_empty());
    assert!(rt.find("staged-example").is_some());
    rt.key(rune::KeyEvent {
        key: rune::Key::Enter,
        mods: Default::default(),
    });
    rt.frame(0.0);
    assert_eq!(rt.state.items[0].title, "3");
    assert!(rt.state.copied_values().is_empty());
    rt.key(rune::KeyEvent {
        key: rune::Key::Enter,
        mods: Default::default(),
    });
    rt.frame(0.0);
    assert_eq!(rt.state.copied_values(), vec!["3"]);
    assert!(rt.state.panel.visible);
}
#[test]
fn unpin_is_reachable_from_compact_and_escape_removes_results() {
    let mut rt = runtime();
    rt.update(|state, _| {
        state.transition(PanelEvent::Pin(true));
        state.tick(Duration::from_secs(3));
    });
    rt.frame(0.0);
    let (x, y) = rt.find("pin").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.0);
    assert!(!rt.state.panel.pinned);
    assert!(!rt.state.panel.compact);
    rt.update(|state, _| {
        state.transition(PanelEvent::QueryChanged("2+2".into()));
        state.transition(PanelEvent::Escape);
    });
    rt.frame(0.0);
    assert!(rt.state.items.is_empty());
    assert!(rt.find("query").is_none());
}
#[test]
fn submitting_a_result_uses_its_action_object_and_keeps_a_pinned_query() {
    let mut rt = runtime();
    rt.update(|state, _| {
        state.transition(PanelEvent::Pin(true));
        state.transition(PanelEvent::QueryChanged("2+2".into()));
    });
    rt.frame(0.0);
    let (x, y) = rt.find("submit").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.0);
    assert_eq!(rt.state.copied_values(), vec!["4"]);
    assert_eq!(rt.state.panel.query, "2+2");
    assert!(rt.state.panel.visible);
}

#[test]
fn invalid_shortcut_does_not_replace_the_previous_binding() {
    let mut state = AppState::preview();
    let previous = state.config.shortcut.clone();
    state.shortcut_edit = rune::TextEdit::new("space");
    athanor::ui::native::apply_shortcut(&mut state, &mut rune::Tx::default());
    assert_eq!(state.config.shortcut, previous);
}

#[test]
fn asynchronous_queries_show_pending_instead_of_a_false_no_match() {
    use std::sync::Arc;
    let mut services = athanor::bootstrap::Services::preview();
    services.worker = Some(athanor::services::query_worker::QueryWorker::new(
        services.engine.clone(),
        Arc::new(|| {}),
    ));
    let mut state = AppState::new(Default::default(), services, false);
    state.transition(PanelEvent::Invoke);
    state.transition(PanelEvent::QueryChanged("2+2".into()));
    let mut rt = Runtime::new(
        state,
        tide_view,
        (330.0, 238.0),
        Env {
            system_dark: true,
            scale: 1.0,
        },
    );
    rt.frame(0.0);
    assert!(rt.find("pending").is_some());
}
#[test]
fn small_text_meets_contrast_over_light_and_dark_backgrounds_in_both_opacity_modes() {
    fn luminance(c: [f64; 3]) -> f64 {
        c.into_iter()
            .zip([0.2126, 0.7152, 0.0722])
            .map(|(v, w)| {
                w * if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            })
            .sum()
    }
    fn blend(a: [f64; 3], b: [f64; 3], alpha: f64) -> [f64; 3] {
        std::array::from_fn(|i| a[i] * alpha + b[i] * (1. - alpha))
    }
    let rune::Color { r, g, b, .. } = athanor::ui::tokens::muted();
    fn srgb(v: f32) -> f64 {
        let v = f64::from(v);
        if v <= 0.0031308 {
            12.92 * v
        } else {
            1.055 * v.powf(1. / 2.4) - 0.055
        }
    }
    let ink = [srgb(r), srgb(g), srgb(b)];
    let panel = [21. / 255., 34. / 255., 43. / 255.];
    for backdrop in [[1.; 3], [0.; 3]] {
        for compositor in [false, true] {
            let background = blend(panel, backdrop, 0.65);
            let text = if compositor {
                blend(ink, backdrop, 0.65)
            } else {
                ink
            };
            let ratio = (luminance(text) + 0.05) / (luminance(background) + 0.05);
            assert!(ratio >= 4.5, "contrast {ratio}, compositor {compositor}");
        }
    }
}
#[test]
fn failed_background_open_keeps_the_query_and_shows_the_error() {
    use athanor_core::feature::*;
    use std::sync::Arc;
    struct Rejected;
    impl ActionHost for Rejected {
        fn copy_text(&self, _: &str) -> Result<(), ActionError> {
            Ok(())
        }
        fn open(&self, _: &OpenTarget) -> Result<(), ActionError> {
            Err(ActionError("Opener failed".into()))
        }
        fn reveal(&self, _: &std::path::Path) -> Result<(), ActionError> {
            Ok(())
        }
    }
    let mut s = AppState::preview();
    s.transition(PanelEvent::Invoke);
    s.transition(PanelEvent::QueryChanged("report".into()));
    s.items = vec![ResultItem {
        key: ResultKey {
            feature: "search",
            local: "report".into(),
        },
        title: "report".into(),
        detail: String::new(),
        presentation: Presentation::File,
        actions: vec![Arc::new(OpenAction(OpenTarget::Path("/report".into())))],
    }];
    s.enable_actions(Arc::new(Rejected), Arc::new(|| {}));
    s.act(0);
    let until = std::time::Instant::now() + Duration::from_secs(2);
    while s.panel.error.is_none() {
        athanor::ui::poll(&mut s, 0.0, &mut rune::Tx::default());
        assert!(std::time::Instant::now() < until);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(s.panel.query, "report");
    assert!(s.panel.visible);
}
#[test]
fn system_display_recovery_clamps_a_visible_panel_without_requesting_focus() {
    use athanor_core::{geometry::WorkArea, panel::PanelEffect};
    let mut state = AppState::preview();
    state.transition(PanelEvent::Invoke);
    state.transition(PanelEvent::Pin(true));
    state.pending.clear();
    let area = WorkArea {
        id: "surviving".into(),
        origin: (0, 0),
        logical_size: (800., 600.),
        scale: 1.,
    };
    let placed = state
        .recover_after_system_change(vec![area], Some((-1800, 200)))
        .unwrap();
    assert!((0..=470).contains(&placed.position.0));
    assert!(state.panel.visible);
    assert!(
        !state
            .pending
            .iter()
            .any(|effect| matches!(effect, PanelEffect::FocusInput | PanelEffect::Show))
    );
}
