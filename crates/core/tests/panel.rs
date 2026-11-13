use athanor_core::panel::*;
use std::time::Duration;
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn pinned() -> PanelState {
    let mut state = PanelState::default();
    state.reduce(PanelEvent::Invoke, ms(0));
    state.reduce(PanelEvent::Pin(true), ms(0));
    state
}
#[test]
fn pinned_empty_collapses_at_three_seconds_and_typing_expands_immediately() {
    let mut state = pinned();
    state.reduce(PanelEvent::IdleExpired, ms(2999));
    assert!(!state.compact);
    state.reduce(PanelEvent::IdleExpired, ms(3000));
    assert!(state.compact);
    let effects = state.reduce(PanelEvent::QueryChanged("2+2".into()), ms(3001));
    assert!(!state.compact);
    assert_eq!(state.query, "2+2");
    assert!(
        effects
            .iter()
            .any(|x| matches!(x, PanelEffect::Query { .. }))
    );
    state.reduce(PanelEvent::IdleExpired, ms(50000));
    assert!(!state.compact);
}
#[test]
fn pinned_focus_loss_preserves_queries_and_ignores_typing_elsewhere() {
    let mut state = pinned();
    state.reduce(PanelEvent::QueryChanged("project".into()), ms(1));
    state.reduce(PanelEvent::FocusChanged(false), ms(2));
    state.reduce(PanelEvent::QueryChanged("other app".into()), ms(3));
    assert!(state.visible);
    assert_eq!(state.query, "project");
    state.reduce(PanelEvent::Invoke, ms(4));
    assert!(state.focused);
}
#[test]
fn escape_invalidates_results_clears_and_dismisses_even_when_pinned() {
    let mut state = pinned();
    state.reduce(PanelEvent::QueryChanged("project".into()), ms(1));
    let old = state.generation;
    let effects = state.reduce(PanelEvent::Escape, ms(2));
    assert!(!state.visible && state.query.is_empty());
    assert!(state.generation > old);
    assert!(state.deadline.is_none());
    assert!(effects.contains(&PanelEffect::CancelQuery));
    state.reduce(PanelEvent::IdleExpired, ms(99999));
    assert!(!state.visible);
}
#[test]
fn action_success_and_failure_respect_pinning() {
    let mut state = pinned();
    state.reduce(PanelEvent::QueryChanged("2+2".into()), ms(1));
    state.reduce(
        PanelEvent::ActionFailed("Clipboard unavailable".into()),
        ms(2),
    );
    assert!(state.visible);
    assert_eq!(state.query, "2+2");
    state.reduce(PanelEvent::ActionSucceeded { yield_focus: false }, ms(3));
    assert!(state.visible);
    state.reduce(PanelEvent::Pin(false), ms(4));
    state.reduce(PanelEvent::ActionSucceeded { yield_focus: false }, ms(5));
    assert!(state.visible);
    assert_eq!(state.query, "2+2");
}
#[test]
fn composition_and_activity_defer_idle_without_creating_periodic_work() {
    let mut state = pinned();
    state.reduce(PanelEvent::Activity, ms(2000));
    state.reduce(PanelEvent::IdleExpired, ms(3000));
    assert!(!state.compact);
    state.reduce(PanelEvent::BeginComposition, ms(4000));
    state.reduce(PanelEvent::IdleExpired, ms(9000));
    assert!(!state.compact);
    state.reduce(PanelEvent::EndComposition("雪".into()), ms(9001));
    assert_eq!(state.query, "雪");
    assert!(state.deadline.is_none());
}
#[test]
fn unpinned_focus_loss_preserves_visibility_and_resume_never_steals_focus() {
    let mut state = PanelState::default();
    state.reduce(PanelEvent::Invoke, ms(0));
    state.reduce(PanelEvent::QueryChanged("sqrt(144)+2^5".into()), ms(0));
    let generation = state.generation;
    state.reduce(PanelEvent::FocusChanged(false), ms(1));
    assert!(state.visible);
    assert_eq!(state.query, "sqrt(144)+2^5");
    assert_eq!(state.generation, generation);
    let effects = state.reduce(PanelEvent::Resume, ms(2));
    assert!(!effects.contains(&PanelEffect::FocusInput));
    assert!(state.visible);
}

#[test]
fn changing_pin_only_changes_window_level_and_preserves_input() {
    let mut state = PanelState::default();
    state.reduce(PanelEvent::Invoke, ms(0));
    state.reduce(PanelEvent::QueryChanged("12 km in mi".into()), ms(1));
    let generation = state.generation;
    for pinned in [true, false] {
        let effects = state.reduce(PanelEvent::Pin(pinned), ms(2));
        assert!(effects.contains(&PanelEffect::SetAlwaysOnTop(pinned)));
        assert!(state.visible);
        assert_eq!(state.query, "12 km in mi");
        assert_eq!(state.generation, generation);
        assert!(!effects.iter().any(|effect| matches!(
            effect,
            PanelEffect::Hide
                | PanelEffect::Show
                | PanelEffect::CancelQuery
                | PanelEffect::FocusInput
        )));
    }
}

#[test]
fn settings_suspend_idle_until_closed() {
    let mut state = pinned();
    state.reduce(PanelEvent::SuspendIdle(true), ms(1000));
    state.reduce(PanelEvent::IdleExpired, ms(9000));
    assert!(!state.compact);
    state.reduce(PanelEvent::SuspendIdle(false), ms(10000));
    state.reduce(PanelEvent::IdleExpired, ms(13000));
    assert!(state.compact);
}

#[test]
fn staged_examples_stay_editable_until_explicit_submission() {
    let mut state = PanelState::default();
    state.reduce(PanelEvent::Invoke, ms(0));
    state.reduce(PanelEvent::QueryChanged("help calculator".into()), ms(1));
    let old = state.generation;
    let effects = state.reduce(PanelEvent::StageQuery("1+1".into()), ms(2));
    assert!(state.query_staged);
    assert!(state.generation > old);
    assert!(effects.contains(&PanelEffect::CancelQuery));
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, PanelEffect::Query { .. } | PanelEffect::Hide))
    );
    let edits = state.reduce(PanelEvent::QueryChanged("1+2".into()), ms(3));
    assert!(
        !edits
            .iter()
            .any(|effect| matches!(effect, PanelEffect::Query { .. }))
    );
    state.reduce(PanelEvent::BeginComposition, ms(4));
    let composed = state.reduce(PanelEvent::EndComposition("1+3".into()), ms(5));
    assert!(
        !composed
            .iter()
            .any(|effect| matches!(effect, PanelEffect::Query { .. }))
    );
    let submitted = state.reduce(PanelEvent::SubmitQuery, ms(6));
    assert!(!state.query_staged);
    assert!(
        submitted
            .iter()
            .any(|effect| matches!(effect,PanelEffect::Query{text,..} if text=="1+3"))
    );
    state.reduce(PanelEvent::Escape, ms(7));
    assert!(!state.query_staged);
}
