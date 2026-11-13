use super::{AppState, event, flush, scenes, tide};
use crate::services::instance::{InstanceServer, Start};
use crate::{
    bootstrap::Services,
    config::{AppPaths, Config},
};
use athanor_core::{
    geometry::{SavedPlacement, WorkArea, place_panel, target_monitor},
    panel::PanelEvent,
};
use rune::{App, AppSpec, FileDialogOptions, MacWindowOptions, Tx, WindowOptions, WindowSpec};
#[cfg(not(target_os = "macos"))]
use rune::{TrayEvent, TrayIcon, TrayItem, TraySpec};
use std::sync::Arc;

pub fn run() -> Result<(), String> {
    let paths = AppPaths::discover().map_err(|e| e.to_string())?;
    let (config, warning) = Config::load(&paths.config);
    let args: Vec<_> = std::env::args().collect();
    let preview = args
        .iter()
        .any(|a| ["--snap", "--tree", "--script", "--list-scenes"].contains(&a.as_str()));
    let show = args.iter().any(|a| a == "--show");
    let x11_opacity = !preview && config.x11_compositor_opacity;
    let measure = std::env::var_os("ATHANOR_MEASURE").is_some();
    let reveal_clock = measure.then(|| Arc::new(std::sync::Mutex::new(None::<std::time::Instant>)));
    #[cfg(target_os = "linux")]
    if !preview && std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return Err("Native Wayland overlay support is not verified in this build. Use an X11 session; GNOME XWayland compatibility remains opt-in and requires native checks.".into());
    }
    if !preview && !rune::single_instance("dev.veilric.athanor") {
        return Ok(());
    }
    let (instance, commands) = if preview {
        (None, None)
    } else {
        match InstanceServer::start(&paths.data).map_err(|e| e.to_string())? {
            Start::Primary(server, receiver) => (Some(server), Some(receiver)),
            Start::Forwarded => return Ok(()),
        }
    };
    let wake_slot = instance.as_ref().map(|server| server.wake.clone());
    rune::set_app_id("dev.veilric.athanor");
    rune::run_windows(move |displays| {
        let areas: Vec<_> = displays
            .iter()
            .enumerate()
            .map(|(i, d)| WorkArea {
                id: d
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("Display {}", i + 1)),
                origin: d.position,
                logical_size: (f64::from(d.size.0) / d.scale, f64::from(d.size.1) / d.scale),
                scale: d.scale,
            })
            .collect();
        let spec = AppSpec {
            min_size: Some((0.0, 0.0)),
            scenes: scenes::SCENES,
            poll: Some(super::poll),
            on_key: Some(tide::on_key),
            families: ("system-ui, Inter", "ui-monospace"),
            ..AppSpec::new(
                "Athanor",
                (330.0, 238.0),
                AppState::preview,
                tide::tide_view,
            )
        };
        let app = App::new(spec).windowless().keep_running().accessory();
        let frame_clock = reveal_clock.clone();
        let app = if let Some(clock) = frame_clock {
            app.on_frame_encoded(move |frame| {
                if frame.outcome() == rune::FrameOutcome::Drawn
                    && let Some(start) = clock.lock().unwrap_or_else(|p| p.into_inner()).take()
                {
                    eprintln!(
                        "athanor-measure reveal_submit_ms={:.3}",
                        start.elapsed().as_secs_f64() * 1000.
                    );
                }
            })
        } else {
            app
        };
        let handle = app.handle();
        let state_handle = handle.clone();
        let app = app
            .init(move || {
                let wake_handle = state_handle.clone();
                let services = if preview {
                    Services::preview()
                } else {
                    Services::real(&config, &paths, Arc::new(move || wake_handle.wake()))
                };
                let mut state = AppState::new(config, services, preview);
                state.handle = Some(state_handle);
                state.areas = areas;
                state.paths = Some(paths);
                state.status = warning;
                state.commands = commands;
                state.reveal_clock = reveal_clock;
                if !preview{
                    let wake=state.handle.as_ref().expect("Native handle exists").clone();
                    state.enable_actions(state.host.clone(),Arc::new(move||wake.wake()));
                    #[cfg(target_os="linux")]{
                        let dirty=state.system_dirty.clone();let wake=state.handle.as_ref().expect("Native handle exists").clone();
                        match super::system_events::SystemEvents::start(Arc::new(move||{dirty.store(true,std::sync::atomic::Ordering::Release);wake.wake();})){
                            Ok(events)=>state.system_events=Some(events),
                            Err(_)=>state.status=Some("Display notifications unavailable. Invoke Athanor to recover its placement after a monitor change.".into()),
                        }
                    }
                }
                if let Some(slot) = wake_slot {
                    let wake = state.handle.as_ref().expect("Native handle exists").clone();
                    *slot.lock().unwrap_or_else(|p| p.into_inner()) =
                        Some(Arc::new(move || wake.wake()));
                }
                state
            })
            .on_focus(|state, focused, tx| {
                // X11 window managers may replace pre-map _NET_WM_STATE when mapping.
                if focused && let Some(window) = &state.window {
                    window.set_always_on_top(state.panel.pinned);
                }
                if !focused {
                    remember_position(state);
                }
                event(state, PanelEvent::FocusChanged(focused), tx);
            })
            .on_close(|state, _| {
                remember_position(state);
                state.transition(PanelEvent::Escape);
            })
            .on_quit(|state, _| remember_position(state))
            .on_reopen(|state, _, tx| {
                invoke(state, tx);
                true
            });
        let ready_handle = handle.clone();
        let app = app.on_window(move |window| {
            let window = window.clone();
            ready_handle.update(move |state, tx| {
                window.set_always_on_top(state.panel.pinned);
                #[cfg(target_os = "linux")]
                apply_x11_opacity(&window, &state.config);
                state.window = Some(window);
                position(state);
                flush(state, tx);
            });
        });
        handle.update(move |state, tx| {
            if !state.preview {
                apply_shortcut(state, tx);
                create_tray(state);
            }
            if show {
                invoke(state, tx);
            }
        });
        let options = if cfg!(target_os = "macos") {
            WindowOptions::launcher_panel()
                .macos(|native| native.level(0).corner_radius(16.0).shadow(true))
        } else {
            WindowOptions {
                visible: false,
                transparent: !(cfg!(target_os = "linux") && x11_opacity),
                borderless: true,
                always_on_top: false,
                min_size: Some((0.0, 0.0)),
                resizable: false,
                ..Default::default()
            }
            .macos(MacWindowOptions::default())
        };
        vec![WindowSpec::from_app("athanor", app).configured_options(options)]
    });
    drop(instance);
    Ok(())
}
pub fn invoke(state: &mut AppState, tx: &mut Tx) {
    if let Some(clock) = &state.reveal_clock {
        *clock.lock().unwrap_or_else(|p| p.into_inner()) = Some(std::time::Instant::now());
    }
    if !state.panel.visible {
        position(state);
    }
    event(state, PanelEvent::Invoke, tx);
}
pub fn recover(state: &mut AppState) {
    let (areas, _) = super::displays::current(&state.areas);
    let position = state
        .window
        .as_ref()
        .and_then(|window| window.placement())
        .map(|p| p.position);
    if let Some(placed) = state.recover_after_system_change(areas, position)
        && let Some(handle) = &state.handle
    {
        handle.set_position(placed.position);
    }
}
pub fn position(state: &mut AppState) {
    if !state.preview {
        let (areas, foreground) = super::displays::current(&state.areas);
        state.areas = areas;
        // Pinning preserves the current display and offset; an invocation still clamps a removed display.
        if state.panel.pinned {
            remember_position(state);
        }
        let preferred = if state.panel.pinned {
            state
                .window
                .as_ref()
                .and_then(|window| window.placement())
                .and_then(|p| target_monitor(&state.areas, None, Some(p.position)))
                .map(str::to_string)
        } else {
            target_monitor(
                &state.areas,
                state.config.preferred_display.as_deref(),
                foreground,
            )
            .map(str::to_string)
        };
        place(state, preferred.as_deref());
    } else {
        place(state, state.config.preferred_display.clone().as_deref());
    }
}
fn place(state: &mut AppState, preferred: Option<&str>) {
    let saved = preferred.and_then(|id| state.config.positions.get(id));
    if let Some(placed) = place_panel(
        saved,
        &state.areas,
        preferred,
        (330.0, f64::from(state.desired_height())),
    ) && let Some(handle) = &state.handle
    {
        handle.set_position(placed.position);
    }
}
pub fn remember_position(state: &mut AppState) {
    if state.preview {
        return;
    }
    let Some(placement) = state.window.as_ref().and_then(|w| w.placement()) else {
        return;
    };
    let (x, y) = placement.position;
    let area = state.areas.iter().find(|a| {
        f64::from(x) >= f64::from(a.origin.0)
            && f64::from(x) < f64::from(a.origin.0) + a.logical_size.0 * a.scale
            && f64::from(y) >= f64::from(a.origin.1)
            && f64::from(y) < f64::from(a.origin.1) + a.logical_size.1 * a.scale
    });
    if let Some(area) = area {
        let saved = SavedPlacement {
            monitor_id: area.id.clone(),
            offset: (
                (f64::from(x) - f64::from(area.origin.0)) / area.scale,
                (f64::from(y) - f64::from(area.origin.1)) / area.scale,
            ),
        };
        let changed = state
            .config
            .positions
            .get(&area.id)
            .is_none_or(|old| old.offset != saved.offset);
        if changed {
            state.config.positions.insert(area.id.clone(), saved);
            state.save_config();
        }
    }
}
pub fn apply_shortcut(state: &mut AppState, _: &mut Tx) {
    if state.binding_pending {
        state.status = Some("Waiting for shortcut registration.".into());
        return;
    }
    let text = state.shortcut_edit.text.trim().to_string();
    let candidate = match text.parse::<rune::Shortcut>() {
        Ok(candidate) => candidate,
        Err(_) => {
            state.status = Some("Use a modifier and key, for example ctrl+alt+space.".into());
            return;
        }
    };
    if candidate.require_modifier().is_err() {
        state.status = Some("Add Ctrl, Alt, or Command to the shortcut.".into());
        return;
    }
    if state.preview {
        state.config.shortcut = text;
        state.status = Some("Shortcut preview updated.".into());
        return;
    }
    let Some(handle) = state.handle.clone() else {
        return;
    };
    state.binding_generation = state.binding_generation.wrapping_add(1);
    state.binding_pending = true;
    let generation = state.binding_generation;
    std::thread::spawn(move || {
        let result = handle.register_shortcut(candidate, invoke);
        let completed_handle = handle.clone();
        handle.update(move |state, tx| {
            if generation != state.binding_generation {
                if let Ok(id) = result {
                    let _ = completed_handle.unregister_shortcut(id);
                }
                return;
            }
            state.binding_pending = false;
            match result {
                Ok(id) => {
                    if let Some(old) = state.shortcut_id.replace(id) {
                        let _ = completed_handle.unregister_shortcut(old);
                    }
                    state.config.shortcut = text;
                    state.save_config();
                    state.status = Some("Shortcut saved.".into());
                }
                Err(error) => {
                    state.status = Some(format!(
                        "Could not use that shortcut: {error}. Your previous binding is unchanged."
                    ));
                    if state.shortcut_id.is_none() {
                        invoke(state, tx);
                        state.show_settings();
                        flush(state, tx);
                    }
                }
            }
        });
    });
}
pub fn add_folder(state: &mut AppState, tx: &mut Tx) {
    if state.preview {
        state.status = Some("The native folder picker opens in the desktop app.".into());
        return;
    }
    tx.open_folder(
        FileDialogOptions {
            title: "Add a search folder".into(),
            ..Default::default()
        },
        |state: &mut AppState, result, _| match result {
            Ok(Some(path)) => {
                if !state.folders_edit.text.is_empty() {
                    state.folders_edit.text.push('\n');
                }
                state.folders_edit.text.push_str(&path.to_string_lossy());
                state.update_folders();
            }
            Ok(None) => {}
            Err(_) => {
                state.status =
                    Some("Folder picker unavailable. Enter a path in Search folders.".into())
            }
        },
    );
}
fn create_tray(state: &mut AppState) {
    #[cfg(target_os = "macos")]
    {
        super::mac_tray::create(state);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let Some(handle) = state.handle.clone() else {
            return;
        };
        let icon = TrayIcon::rgba(32, 32, tray_pixels());
        let spec = TraySpec::new("dev.veilric.athanor", "Athanor", icon)
            .tooltip("Athanor")
            .menu(vec![
                TrayItem::action("show", "Show Athanor"),
                TrayItem::action("settings", "Settings"),
                TrayItem::Separator,
                TrayItem::action("quit", "Quit"),
            ]);
        match rune::Tray::new(spec, move |event| {
            let next = handle.clone();
            handle.update(move |state, tx| match event {
                TrayEvent::Activate { token } => {
                    invoke(state, tx);
                    if let Some(token) = token {
                        next.activate(token);
                    }
                }
                TrayEvent::Menu(id) if id == "show" => invoke(state, tx),
                TrayEvent::Menu(id) if id == "settings" => {
                    invoke(state, tx);
                    state.show_settings();
                    flush(state, tx);
                }
                TrayEvent::Menu(id) if id == "quit" => next.quit(),
                _ => {}
            });
        }) {
            Ok(tray) => state.tray = Some(tray),
            Err(_) => {
                state.status = Some(
                    "Tray unavailable. Use the shortcut; settings are available from the panel."
                        .into(),
                )
            }
        }
    }
}
pub fn tray_pixels() -> Vec<u8> {
    let mut pixels = vec![0u8; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let dx = x as f32 - 16.0;
            let dy = y as f32 - 15.0;
            let distance = (dx * dx + dy * dy).sqrt();
            let arch =
                y <= 15 && ((10.0..=12.0).contains(&distance) || (4.0..=6.0).contains(&distance));
            let uprights = (15..=26).contains(&y) && ([5, 6, 10, 11, 21, 22, 26, 27].contains(&x));
            let base = (26..=27).contains(&y) && (3..=29).contains(&x);
            if arch || uprights || base {
                let i = (y * 32 + x) * 4;
                pixels[i..i + 4].copy_from_slice(&[220, 236, 245, 255]);
            }
        }
    }
    pixels
}
#[cfg(target_os = "linux")]
pub fn apply_x11_opacity(window: &rune::WindowHandle, config: &Config) {
    use rune::raw_window_handle::RawWindowHandle;
    use x11rb::{
        connection::Connection,
        protocol::xproto::{AtomEnum, ConnectionExt as _, PropMode},
        wrapper::ConnectionExt as _,
    };
    let _ = window.with_raw_handle(|handle| {
        let id = match handle.as_raw() {
            RawWindowHandle::Xlib(h) => h.window as u32,
            RawWindowHandle::Xcb(h) => h.window.get(),
            _ => return,
        };
        let Ok((connection, _)) = x11rb::connect(None) else {
            return;
        };
        let Ok(cookie) = connection.intern_atom(false, b"_NET_WM_WINDOW_OPACITY") else {
            return;
        };
        let Ok(atom) = cookie.reply() else { return };
        if config.x11_compositor_opacity {
            let value = (f64::from(config.opacity) * f64::from(u32::MAX)).round() as u32;
            let _ = connection.change_property32(
                PropMode::REPLACE,
                id,
                atom.atom,
                AtomEnum::CARDINAL,
                &[value],
            );
        } else {
            let _ = connection.delete_property(id, atom.atom);
        }
        let _ = connection.flush();
    });
}
