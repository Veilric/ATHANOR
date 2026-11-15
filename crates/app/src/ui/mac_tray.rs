//! Rune's tray does not yet host macOS status items; keep this adapter platform-local.
use super::{AppState, flush, native};
use tray_icon::{
    Icon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};

pub fn create(state: &mut AppState) {
    if state.mac_tray.is_some() {
        return;
    }
    let Some(handle) = state.handle.clone() else {
        return;
    };
    let show = MenuItem::new("Show Athanor", true, None);
    let settings = MenuItem::new("Settings", true, None);
    let quit = MenuItem::new("Quit Athanor", true, None);
    let (show_id, settings_id, quit_id) =
        (show.id().clone(), settings.id().clone(), quit.id().clone());
    let menu = Menu::new();
    let result = (|| {
        menu.append_items(&[&show, &settings, &quit])
            .map_err(|e| e.to_string())?;
        let icon = Icon::from_rgba(native::tray_pixels(), 32, 32).map_err(|e| e.to_string())?;
        TrayIconBuilder::new()
            .with_tooltip("Athanor")
            .with_icon_templated(icon)
            .with_menu(Box::new(menu))
            .build()
            .map_err(|e| e.to_string())
    })();
    match result {
        Ok(tray) => {
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                let id = event.id;
                let show_id = show_id.clone();
                let settings_id = settings_id.clone();
                let quit_id = quit_id.clone();
                handle.update(move |state, tx| {
                    if id == show_id {
                        native::invoke(state, tx);
                    } else if id == settings_id {
                        native::invoke(state, tx);
                        state.show_settings();
                        flush(state, tx);
                    } else if id == quit_id
                        && let Some(handle) = &state.handle
                    {
                        handle.quit();
                    }
                });
            }));
            state.mac_tray = Some(tray);
        }
        Err(_) => {
            state.status = Some("Menu-bar icon unavailable. Use the invocation shortcut.".into())
        }
    }
}
