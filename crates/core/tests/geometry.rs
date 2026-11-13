use athanor_core::geometry::*;
#[test]
fn placement_accounts_for_negative_origins_and_dpi_once() {
    let areas = vec![WorkArea {
        id: "left".into(),
        origin: (-1920, 0),
        logical_size: (960.0, 540.0),
        scale: 2.0,
    }];
    let placed = place_panel(None, &areas, None, (330.0, 53.0)).unwrap();
    assert_eq!(placed.position.0, -708);
    assert_eq!(placed.size, (330.0, 53.0));
}
#[test]
fn missing_monitor_and_offscreen_saved_coordinates_are_recovered() {
    let areas = vec![WorkArea {
        id: "main".into(),
        origin: (0, 0),
        logical_size: (800.0, 600.0),
        scale: 1.0,
    }];
    let saved = SavedPlacement {
        monitor_id: "gone".into(),
        offset: (2000.0, -400.0),
    };
    let placed = place_panel(Some(&saved), &areas, None, (330.0, 350.0)).unwrap();
    assert_eq!(placed.monitor_id, "main");
    assert!(placed.position.0 >= 0 && placed.position.0 <= 470);
    assert!(placed.position.1 >= 0 && placed.position.1 <= 250);
    assert!(place_panel(None, &[], None, (330.0, 53.0)).is_none());
}
#[test]
fn target_monitor_uses_active_display_unless_a_preference_is_available() {
    let areas = vec![
        WorkArea {
            id: "left".into(),
            origin: (0, 0),
            logical_size: (1920., 1080.),
            scale: 1.,
        },
        WorkArea {
            id: "right".into(),
            origin: (1920, 0),
            logical_size: (1920., 1080.),
            scale: 1.,
        },
    ];
    assert_eq!(
        target_monitor(&areas, None, Some((2200, 100))).unwrap(),
        "right"
    );
    assert_eq!(
        target_monitor(&areas, Some("left"), Some((2200, 100))).unwrap(),
        "left"
    );
    assert_eq!(
        target_monitor(&areas, Some("removed"), Some((2200, 100))).unwrap(),
        "right"
    );
}
