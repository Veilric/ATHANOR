use super::{AppState, flush, tide::button, tokens as t};
use athanor_core::feature::AngleMode;
use rune::{Cx, El, TextStyle, col, row, spacer, text, text_input};

pub fn settings_view(s: &AppState) -> El {
    let header = row()
        .h(52.0)
        .px(14.0)
        .center_y()
        .child(button("‹", "settings-back", "Back to query", |s, tx| {
            s.close_settings();
            flush(s, tx)
        }))
        .child(
            text("Settings")
                .font(TextStyle::new(16.0, 600.0))
                .color(t::ink()),
        )
        .child(spacer());
    let mut body = col().w_full().padding(17.0).gap(10.0);
    body = body
        .child(text("Panel opacity").font(t::label()).color(t::ink()))
        .child(
            row()
                .gap(12.0)
                .center_y()
                .child(button("−", "opacity-less", "Decrease opacity", |s, _| {
                    s.config.opacity = (s.config.opacity - 0.05).max(0.35);
                    s.save_config();
                }))
                .child(
                    text(format!("{}%", (s.config.opacity * 100.0).round() as u32))
                        .font(t::body())
                        .color(t::ink()),
                )
                .child(button("+", "opacity-more", "Increase opacity", |s, _| {
                    s.config.opacity = (s.config.opacity + 0.05).min(1.0);
                    s.save_config();
                })),
        );
    body = body
        .child(text("Global shortcut").font(t::label()).color(t::ink()))
        .child(
            text_input(&s.shortcut_edit, t::body())
                .key("shortcut-input")
                .label("Global shortcut")
                .single_line()
                .w_full()
                .h(31.0)
                .color(t::ink())
                .on_edit(|s: &mut AppState| &mut s.shortcut_edit)
                .on_submit(|s: &mut AppState, _| {
                    s.status = Some("Use Apply shortcut to validate the new binding.".into())
                }),
        )
        .child(button(
            "Apply shortcut",
            "apply-shortcut",
            "Apply shortcut",
            super::native::apply_shortcut,
        ));
    body = body.child(
        row()
            .center_y()
            .gap(12.0)
            .child(text("Angles").font(t::label()).color(t::ink()))
            .child(button(
                if s.config.angle_mode == AngleMode::Radians {
                    "Radians ✓"
                } else {
                    "Degrees ✓"
                },
                "angle-mode",
                "Toggle degrees and radians",
                |s, _| {
                    s.config.angle_mode = if s.config.angle_mode == AngleMode::Radians {
                        AngleMode::Degrees
                    } else {
                        AngleMode::Radians
                    };
                    s.save_config();
                    s.requery();
                },
            )),
    );
    body = body
        .child(text("Search folders").font(t::label()).color(t::ink()))
        .child(
            text_input(&s.folders_edit, t::small())
                .key("folders-input")
                .label("Search folders, one per line")
                .w_full()
                .h(76.0)
                .color(t::ink())
                .on_edit(|s: &mut AppState| &mut s.folders_edit),
        )
        .child(
            row()
                .gap(6.0)
                .child(button(
                    "Add folder…",
                    "add-folder",
                    "Choose a search folder",
                    super::native::add_folder,
                ))
                .child(button(
                    "Apply folders",
                    "apply-folders",
                    "Apply search folders",
                    |s, _| s.update_folders(),
                )),
        )
        .child(button(
            "Refresh search",
            "refresh-search",
            "Refresh the file index",
            |s, _| {
                if let Some(index) = &s.services.index {
                    index.refresh();
                    s.status = Some("Refreshing search in the background.".into());
                }
            },
        ));
    body = body.child(button(
        if s.config.start_at_login {
            "Start at login: on"
        } else {
            "Start at login: off"
        },
        "start-at-login",
        "Toggle start at login",
        |s, _| {
            let enabled = !s.config.start_at_login;
            let result = if s.preview {
                Ok(())
            } else {
                std::env::current_exe()
                    .and_then(|exe| crate::platform::set_start_at_login(enabled, &exe))
            };
            match result {
                Ok(()) => {
                    s.config.start_at_login = enabled;
                    s.save_config();
                }
                Err(_) => s.status = Some("Could not change start-at-login settings.".into()),
            }
        },
    ));
    body = body.child(button(
        "Reset panel placement",
        "reset-placement",
        "Reset saved panel placement",
        |s, _| {
            s.config.positions.clear();
            s.save_config();
            super::native::position(s);
        },
    ));
    body = body
        .child(text("Display").font(t::label()).color(t::ink()))
        .child(button(
            if s.config.preferred_display.is_none() {
                "Active display ✓"
            } else {
                "Active display"
            },
            "display-active",
            "Follow the active display",
            |s, _| {
                s.config.preferred_display = None;
                s.save_config();
            },
        ));
    for (i, area) in s.areas.iter().enumerate() {
        let id = area.id.clone();
        let label = if s.config.preferred_display.as_ref() == Some(&id) {
            format!("{} ✓", id)
        } else {
            id.clone()
        };
        body = body.child(button(
            &label,
            &format!("display-{i}"),
            &format!("Prefer display {id}"),
            move |s, _| {
                s.config.preferred_display = Some(id.clone());
                s.save_config();
            },
        ));
    }
    let snapshot = s
        .services
        .snapshot
        .read()
        .unwrap_or_else(|p| p.into_inner());
    for warning in snapshot.warnings.iter().take(4) {
        body = body.child(
            text(warning.clone())
                .font(t::small())
                .color(t::muted())
                .w_full(),
        );
    }
    if let Some(status) = &s.status {
        body = body.child(
            text(status.clone())
                .font(t::small())
                .color(t::muted())
                .w_full(),
        );
    }
    body = body.child(button("Quit Athanor", "quit", "Quit Athanor", |s, _| {
        if let Some(handle) = &s.handle {
            handle.quit();
        }
    }));
    #[cfg(target_os = "linux")]
    if !s.preview {
        body=body.child(button(if s.config.x11_compositor_opacity{"X11 opacity: compositor"}else{"X11 opacity: native alpha"},"opacity-backend","Toggle X11 opacity mechanism; restart to apply",|s,_|{
        s.config.x11_compositor_opacity = !s.config.x11_compositor_opacity;s.save_config();s.status=Some("Restart Athanor to change the X11 opacity mechanism. Compositor mode also dims text.".into());
    }));
    }
    col()
        .w_full()
        .h_full()
        .radius(16.0)
        .clip_rounded(16.0)
        .fill(t::panel(s.panel_alpha()))
        .border(1.0, t::muted().alpha(0.24))
        .child(header)
        .child(
            body.scroll(rune::ScrollSpec {
                stick_bottom: false,
            })
            .grow(1.0)
            .min_h(0.0),
        )
}
pub fn settings_window(s: &AppState, _: &mut Cx<AppState>) -> El {
    settings_view(s)
}
