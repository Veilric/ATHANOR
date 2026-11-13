use std::time::Duration;

pub const IDLE_DELAY: Duration = Duration::from_secs(3);
#[derive(Default, Clone, Debug)]
pub struct PanelState {
    pub visible: bool,
    pub pinned: bool,
    pub focused: bool,
    pub compact: bool,
    pub composing: bool,
    pub query: String,
    pub query_staged: bool,
    pub generation: u64,
    pub idle_suspended: bool,
    pub deadline: Option<Duration>,
    pub error: Option<String>,
}
#[derive(Clone, Debug)]
pub enum PanelEvent {
    Invoke,
    QueryChanged(String),
    StageQuery(String),
    SubmitQuery,
    Pin(bool),
    FocusChanged(bool),
    Activity,
    IdleExpired,
    Escape,
    ActionSucceeded { yield_focus: bool },
    ActionFailed(String),
    BeginComposition,
    EndComposition(String),
    SuspendIdle(bool),
    Resume,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PanelEffect {
    Show,
    Hide,
    FocusInput,
    SetAlwaysOnTop(bool),
    Resize { compact: bool },
    CancelQuery,
    Query { generation: u64, text: String },
    ScheduleIdle { deadline: Duration },
    CancelIdle,
    Error(String),
}
impl PanelState {
    pub fn reduce(&mut self, event: PanelEvent, now: Duration) -> Vec<PanelEffect> {
        let mut effects = Vec::new();
        match event {
            PanelEvent::Invoke => {
                self.visible = true;
                self.focused = true;
                effects.extend([PanelEffect::Show, PanelEffect::FocusInput]);
                self.schedule(now, &mut effects);
            }
            PanelEvent::QueryChanged(text) => {
                if self.visible && self.focused {
                    self.edit(text, now, &mut effects);
                }
            }
            PanelEvent::StageQuery(text) => {
                if self.visible && self.focused {
                    self.query_staged = true;
                    self.edit(text, now, &mut effects);
                    effects.push(PanelEffect::FocusInput);
                }
            }
            PanelEvent::SubmitQuery => {
                if self.visible && self.focused {
                    self.query_staged = false;
                    self.edit(self.query.clone(), now, &mut effects);
                }
            }
            PanelEvent::Pin(pinned) => {
                if self.pinned != pinned {
                    effects.push(PanelEffect::SetAlwaysOnTop(pinned));
                }
                self.pinned = pinned;
                if !pinned && self.compact {
                    self.compact = false;
                    effects.push(PanelEffect::Resize { compact: false });
                }
                self.schedule(now, &mut effects);
            }
            PanelEvent::FocusChanged(focused) => {
                self.focused = focused;
                self.schedule(now, &mut effects);
            }
            PanelEvent::Activity => {
                if self.visible && self.focused {
                    self.schedule(now, &mut effects);
                }
            }
            PanelEvent::IdleExpired => {
                if self.eligible()
                    && let Some(deadline) = self.deadline
                {
                    if now >= deadline {
                        self.compact = true;
                        self.cancel_idle(&mut effects);
                        effects.push(PanelEffect::Resize { compact: true });
                    } else {
                        effects.push(PanelEffect::ScheduleIdle { deadline });
                    }
                }
            }
            PanelEvent::Escape => self.dismiss(&mut effects),
            PanelEvent::ActionSucceeded { yield_focus } => {
                self.error = None;
                if yield_focus {
                    self.focused = false;
                }
            }
            PanelEvent::ActionFailed(message) => {
                self.error = Some(message.clone());
                effects.push(PanelEffect::Error(message));
            }
            PanelEvent::BeginComposition => {
                if self.visible && self.focused {
                    self.composing = true;
                    self.compact = false;
                    self.cancel_idle(&mut effects);
                    effects.push(PanelEffect::Resize { compact: false });
                }
            }
            PanelEvent::EndComposition(text) => {
                self.composing = false;
                if self.visible && self.focused {
                    self.edit(text, now, &mut effects);
                }
            }
            PanelEvent::Resume => {
                if self.visible {
                    effects.push(PanelEffect::Resize {
                        compact: self.compact,
                    });
                    self.schedule(now, &mut effects);
                }
            }
            PanelEvent::SuspendIdle(suspended) => {
                self.idle_suspended = suspended;
                if suspended {
                    self.compact = false;
                    effects.push(PanelEffect::Resize { compact: false });
                }
                self.schedule(now, &mut effects);
            }
        }
        effects
    }
    fn eligible(&self) -> bool {
        self.visible
            && self.pinned
            && self.query.trim().is_empty()
            && !self.composing
            && !self.idle_suspended
    }
    fn schedule(&mut self, now: Duration, effects: &mut Vec<PanelEffect>) {
        if self.eligible() && !self.compact {
            self.deadline = now.checked_add(IDLE_DELAY);
            if let Some(deadline) = self.deadline {
                effects.push(PanelEffect::ScheduleIdle { deadline });
            }
        } else {
            self.cancel_idle(effects);
        }
    }
    fn cancel_idle(&mut self, effects: &mut Vec<PanelEffect>) {
        if self.deadline.take().is_some() {
            effects.push(PanelEffect::CancelIdle);
        }
    }
    fn edit(&mut self, text: String, now: Duration, effects: &mut Vec<PanelEffect>) {
        self.query = text;
        self.generation = self.generation.wrapping_add(1);
        self.compact = false;
        self.error = None;
        effects.extend([
            PanelEffect::CancelQuery,
            PanelEffect::Resize { compact: false },
        ]);
        if !self.composing && !self.query_staged && !self.query.trim().is_empty() {
            effects.push(PanelEffect::Query {
                generation: self.generation,
                text: self.query.clone(),
            });
        }
        self.schedule(now, effects);
    }
    fn dismiss(&mut self, effects: &mut Vec<PanelEffect>) {
        self.generation = self.generation.wrapping_add(1);
        self.visible = false;
        self.focused = false;
        self.compact = false;
        self.composing = false;
        self.idle_suspended = false;
        self.query.clear();
        self.query_staged = false;
        self.error = None;
        self.cancel_idle(effects);
        effects.extend([PanelEffect::CancelQuery, PanelEffect::Hide]);
    }
}
