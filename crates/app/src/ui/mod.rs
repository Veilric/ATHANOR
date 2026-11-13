mod displays;
#[cfg(target_os = "macos")]
mod mac_tray;
pub mod native;
pub mod scenes;
pub mod settings;
#[cfg(target_os = "linux")]
mod system_events;
pub mod tide;
pub mod tokens;
use crate::{
    bootstrap::{self, Services},
    config::{AppPaths, Config},
    platform::NativeActionHost,
};
use athanor_core::{
    cancel::Cancellation,
    feature::*,
    geometry::WorkArea,
    panel::{PanelEffect, PanelEvent, PanelState},
    query::*,
};
use rune::{AppHandle, TextEdit, Tx, WindowHandle};
use std::{
    path::Path,
    sync::{Arc, Mutex, atomic::Ordering},
    time::Duration,
};

#[derive(Default)]
struct RecordingHost(Arc<Mutex<Vec<String>>>);
impl ActionHost for RecordingHost {
    fn copy_text(&self, text: &str) -> Result<(), ActionError> {
        self.0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(text.into());
        Ok(())
    }
    fn open(&self, _: &OpenTarget) -> Result<(), ActionError> {
        Ok(())
    }
    fn reveal(&self, _: &Path) -> Result<(), ActionError> {
        Ok(())
    }
}
pub struct AppState {
    pub panel: PanelState,
    pub input: TextEdit,
    pub items: Vec<ResultItem>,
    pub selected: usize,
    pub config: Config,
    pub services: Services,
    pub handle: Option<AppHandle<AppState>>,
    pub window: Option<WindowHandle>,
    pub areas: Vec<WorkArea>,
    pub paths: Option<AppPaths>,
    pub preview: bool,
    pub settings_open: bool,
    pub shortcut_edit: TextEdit,
    pub folders_edit: TextEdit,
    pub status: Option<String>,
    pub query_pending: bool,
    pub shortcut_id: Option<rune::ShortcutId>,
    pub binding_generation: u64,
    pub binding_pending: bool,
    pub reveal_clock: Option<Arc<Mutex<Option<std::time::Instant>>>>,
    pub tray: Option<rune::Tray>,
    #[cfg(target_os = "macos")]
    pub mac_tray: Option<tray_icon::TrayIcon>,
    pub commands: Option<std::sync::mpsc::Receiver<crate::services::instance::InstanceCommand>>,
    pub now: Duration,
    pub pending: Vec<PanelEffect>,
    pub last_revision: u64,
    pub last_size: (f32, f32),
    copied: Arc<Mutex<Vec<String>>>,
    host: Arc<dyn ActionHost>,
    pub actions: Option<crate::services::action_worker::ActionWorker>,
    pub system_dirty: Arc<std::sync::atomic::AtomicBool>,
    #[cfg(target_os = "linux")]
    pub(crate) system_events: Option<system_events::SystemEvents>,
}
impl AppState {
    pub fn preview() -> Self {
        let copied = Arc::new(Mutex::new(Vec::new()));
        let mut state = Self::new(Config::default(), Services::preview(), true);
        state.host = Arc::new(RecordingHost(copied.clone()));
        state.copied = copied;
        state
    }
    pub fn new(config: Config, services: Services, preview: bool) -> Self {
        let shortcut_edit = TextEdit::new(&config.shortcut);
        let folders_edit = TextEdit::new(
            config
                .search_roots
                .iter()
                .map(|p| p.to_string_lossy())
                .collect::<Vec<_>>()
                .join("\n"),
        );
        Self {
            panel: PanelState::default(),
            input: TextEdit::new(""),
            items: Vec::new(),
            selected: 0,
            config,
            services,
            handle: None,
            window: None,
            areas: Vec::new(),
            paths: None,
            preview,
            settings_open: false,
            shortcut_edit,
            folders_edit,
            status: None,
            query_pending: false,
            shortcut_id: None,
            binding_generation: 0,
            binding_pending: false,
            reveal_clock: None,
            tray: None,
            now: Duration::ZERO,
            pending: Vec::new(),
            last_revision: 0,
            last_size: (330.0, 0.0),
            copied: Arc::new(Mutex::new(Vec::new())),
            host: Arc::new(NativeActionHost::default()),
            actions: None,
            system_dirty: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            #[cfg(target_os = "linux")]
            system_events: None,
            commands: None,
            #[cfg(target_os = "macos")]
            mac_tray: None,
        }
    }
    pub fn copied_values(&self) -> Vec<String> {
        self.copied
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }
    pub fn transition(&mut self, event: PanelEvent) {
        if matches!(event, PanelEvent::Escape) {
            self.settings_open = false;
        }
        let effects = self.panel.reduce(event, self.now);
        for effect in effects {
            match &effect {
                PanelEffect::CancelQuery => {
                    if let Some(worker) = &self.services.worker {
                        worker.cancel();
                    }
                    self.items.clear();
                    self.selected = 0;
                    self.query_pending = false;
                }
                PanelEffect::Query { generation, text } => {
                    let request = QueryRequest {
                        generation: *generation,
                        text: Arc::from(text.as_str()),
                    };
                    let context = bootstrap::context(&self.config, self.preview);
                    if let Some(worker) = &self.services.worker {
                        self.query_pending = true;
                        worker.submit(request, context);
                    } else {
                        self.items = self
                            .services
                            .engine
                            .query(&request, &context, &Cancellation::default())
                            .items;
                    }
                }
                PanelEffect::FocusInput => self.input.focused = true,
                PanelEffect::Hide => {
                    self.input = TextEdit::new("");
                }
                _ => {}
            }
            self.pending.push(effect);
        }
        if self.input.text != self.panel.query {
            self.input = TextEdit::new(&self.panel.query);
            self.input.focused = self.panel.focused;
        }
        if !self.panel.focused {
            self.input.focused = false;
        }
    }
    pub fn tick(&mut self, now: Duration) {
        self.now = now;
        if self.input.is_composing() && !self.panel.composing {
            self.transition(PanelEvent::BeginComposition);
        } else if !self.input.is_composing() && self.panel.composing {
            self.transition(PanelEvent::EndComposition(self.input.text.clone()));
        }
        if self.panel.deadline.is_some_and(|deadline| now >= deadline) {
            self.transition(PanelEvent::IdleExpired);
        }
    }
    pub fn desired_height(&self) -> f32 {
        let height = if self.settings_open
            || self
                .items
                .iter()
                .any(|item| item.presentation == Presentation::Reference)
        {
            550.0
        } else if self.panel.compact {
            53.0
        } else if self.items.len() == 1 && self.items[0].presentation == Presentation::Answer {
            238.0
        } else {
            (53.0 + (self.items.len().min(8) as f32 * 54.0).max(110.0) + 34.0).min(550.0)
        };
        if let Some(p) = self.window.as_ref().and_then(|window| window.placement())
            && let Some(area) = self.areas.iter().find(|area| {
                f64::from(p.position.0) >= f64::from(area.origin.0)
                    && f64::from(p.position.0)
                        < f64::from(area.origin.0) + area.logical_size.0 * area.scale
                    && f64::from(p.position.1) >= f64::from(area.origin.1)
                    && f64::from(p.position.1)
                        < f64::from(area.origin.1) + area.logical_size.1 * area.scale
            })
        {
            let available = (area.logical_size.1
                - (f64::from(p.position.1) - f64::from(area.origin.1)) / area.scale)
                .max(53.0) as f32;
            return height.min(available);
        }
        height
    }
    pub fn panel_alpha(&self) -> f32 {
        if cfg!(target_os = "linux") && !self.preview && self.config.x11_compositor_opacity {
            1.0
        } else {
            self.config.opacity
        }
    }
    pub fn act(&mut self, index: usize) {
        let Some(action) = self
            .items
            .get(self.selected)
            .and_then(|item| item.actions.get(index))
            .cloned()
        else {
            return;
        };
        if action.background()
            && let Some(worker) = &self.actions
        {
            match worker.submit(self.panel.generation, action) {
                Ok(()) => self.status = Some("Opening…".into()),
                Err(error) => self.transition(PanelEvent::ActionFailed(error.to_string())),
            }
            return;
        }
        self.finish_action(action.execute(self.host.as_ref()));
    }
    pub fn submit(&mut self) {
        if self.panel.query_staged {
            self.transition(PanelEvent::SubmitQuery);
        } else {
            self.act(0);
        }
    }
    pub fn enable_actions(&mut self, host: Arc<dyn ActionHost>, wake: Arc<dyn Fn() + Send + Sync>) {
        self.actions = Some(crate::services::action_worker::ActionWorker::new(
            host.clone(),
            wake,
        ));
        self.host = host;
    }
    fn finish_action(&mut self, result: Result<ActionOutcome, ActionError>) {
        match result {
            Ok(outcome) => {
                if let Some(input) = outcome.input {
                    if input.submit {
                        self.transition(PanelEvent::QueryChanged(input.text));
                    } else {
                        self.transition(PanelEvent::StageQuery(input.text));
                    }
                    return;
                }
                self.status = Some(if outcome.yield_focus {
                    "Opened".into()
                } else {
                    "Copied".into()
                });
                self.transition(PanelEvent::ActionSucceeded {
                    yield_focus: outcome.yield_focus,
                });
            }
            Err(error) => self.transition(PanelEvent::ActionFailed(error.to_string())),
        }
    }
    pub fn save_config(&mut self) {
        if self.preview {
            return;
        }
        #[cfg(target_os = "linux")]
        if let Some(window) = &self.window {
            native::apply_x11_opacity(window, &self.config);
        }
        if let Some(paths) = &self.paths
            && let Err(error) = self.config.save(&paths.config)
        {
            self.status = Some(format!("Could not save settings: {error}"));
        }
    }
    pub fn show_settings(&mut self) {
        self.settings_open = true;
        self.transition(PanelEvent::SuspendIdle(true));
        self.input.focused = false;
        self.shortcut_edit = TextEdit::new(&self.config.shortcut);
        self.folders_edit = TextEdit::new(
            self.config
                .search_roots
                .iter()
                .map(|p| p.to_string_lossy())
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    pub fn close_settings(&mut self) {
        self.settings_open = false;
        self.transition(PanelEvent::SuspendIdle(false));
        self.input.focused = self.panel.focused;
    }
    pub fn update_folders(&mut self) {
        let roots: Vec<_> = self
            .folders_edit
            .text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(std::path::PathBuf::from)
            .collect();
        if roots.len() > 32 {
            self.status = Some("Use at most 32 search folders.".into());
            return;
        }
        self.config.search_roots = roots;
        self.save_config();
        if let Some(paths) = &self.paths {
            self.services.restart_index(&self.config, paths);
        }
        self.status = Some("Search folders updated.".into());
    }
    pub fn requery(&mut self) {
        if self.panel.visible && !self.panel.query_staged && !self.panel.query.trim().is_empty() {
            let request = QueryRequest {
                generation: self.panel.generation,
                text: Arc::from(self.panel.query.as_str()),
            };
            let context = bootstrap::context(&self.config, self.preview);
            if let Some(worker) = &self.services.worker {
                worker.submit(request, context)
            } else {
                self.items = self
                    .services
                    .engine
                    .query(&request, &context, &Cancellation::default())
                    .items;
            }
        }
    }
}
impl AppState {
    pub fn recover_after_system_change(
        &mut self,
        areas: Vec<WorkArea>,
        position: Option<(i32, i32)>,
    ) -> Option<athanor_core::geometry::PlacedPanel> {
        use athanor_core::geometry::{SavedPlacement, place_panel, target_monitor};
        self.areas = areas;
        if !self.panel.visible {
            return None;
        }
        let id = target_monitor(&self.areas, None, position)?;
        let area = self.areas.iter().find(|a| a.id == id)?;
        let saved = position
            .filter(|(x, y)| {
                f64::from(*x) >= f64::from(area.origin.0)
                    && f64::from(*y) >= f64::from(area.origin.1)
                    && f64::from(*x) < f64::from(area.origin.0) + area.logical_size.0 * area.scale
                    && f64::from(*y) < f64::from(area.origin.1) + area.logical_size.1 * area.scale
            })
            .map(|(x, y)| SavedPlacement {
                monitor_id: id.to_string(),
                offset: (
                    (f64::from(x) - f64::from(area.origin.0)) / area.scale,
                    (f64::from(y) - f64::from(area.origin.1)) / area.scale,
                ),
            });
        let placed = place_panel(
            saved.as_ref().or_else(|| self.config.positions.get(id)),
            &self.areas,
            Some(id),
            (330., f64::from(self.desired_height())),
        );
        self.transition(PanelEvent::Resume);
        placed
    }
}
pub fn event(state: &mut AppState, event: PanelEvent, tx: &mut Tx) {
    state.transition(event);
    flush(state, tx);
}
pub fn flush(state: &mut AppState, tx: &mut Tx) {
    for effect in std::mem::take(&mut state.pending) {
        match effect {
            PanelEffect::Show => {
                if let Some(handle) = &state.handle {
                    handle.show();
                }
            }
            PanelEffect::Hide => {
                if let Some(handle) = &state.handle {
                    handle.hide();
                }
            }
            PanelEffect::FocusInput => {
                if let Some(handle) = &state.handle {
                    if let Some(token) = tx.activation_token() {
                        handle.activate(token);
                    } else {
                        handle.open();
                    }
                }
            }
            PanelEffect::SetAlwaysOnTop(pinned) => {
                if let Some(window) = &state.window {
                    window.set_always_on_top(pinned);
                }
            }
            PanelEffect::ScheduleIdle { deadline } => {
                tx.wake_after(deadline.saturating_sub(state.now))
            }
            _ => {}
        }
    }
    let size = (330.0, state.desired_height());
    if state.last_size != size {
        tx.resize_window_instant(size);
        state.last_size = size;
    }
    if let Some(deadline) = state.panel.deadline {
        tx.wake_after(deadline.saturating_sub(state.now));
    }
}
pub fn poll(state: &mut AppState, elapsed: f64, tx: &mut Tx) -> bool {
    let before = (
        state.panel.compact,
        state.panel.composing,
        state.items.len(),
    );
    state.tick(Duration::from_secs_f64(elapsed.max(0.0)));
    let system_changed = state.system_dirty.swap(false, Ordering::AcqRel);
    if system_changed {
        native::recover(state);
    }
    let mut commands_changed = false;
    loop {
        let command = state.commands.as_ref().and_then(|rx| rx.try_recv().ok());
        let Some(command) = command else { break };
        commands_changed = true;
        match command {
            crate::services::instance::InstanceCommand::Show => native::invoke(state, tx),
            crate::services::instance::InstanceCommand::Settings => {
                native::invoke(state, tx);
                state.show_settings();
            }
            crate::services::instance::InstanceCommand::Quit => {
                if let Some(handle) = &state.handle {
                    handle.quit();
                }
            }
        }
    }
    let revision = state.services.revision.load(Ordering::Acquire);
    if revision != state.last_revision {
        state.last_revision = revision;
        state.requery();
    }
    let mut updated = false;
    if let Some(completion) = state
        .actions
        .as_ref()
        .and_then(|worker| worker.take_latest())
        && completion.generation == state.panel.generation
        && state.panel.visible
    {
        state.finish_action(completion.result);
        updated = true;
    }
    if let Some(batch) = state
        .services
        .worker
        .as_ref()
        .and_then(|worker| worker.take_latest())
        && batch.can_apply(state.panel.generation, state.panel.visible)
    {
        state.items = batch.items;
        state.query_pending = false;
        state.selected = state.selected.min(state.items.len().saturating_sub(1));
        updated = true;
    }
    flush(state, tx);
    updated
        || commands_changed
        || system_changed
        || before
            != (
                state.panel.compact,
                state.panel.composing,
                state.items.len(),
            )
}
