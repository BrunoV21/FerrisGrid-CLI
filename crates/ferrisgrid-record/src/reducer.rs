use ferrisgrid_core::{
    ActionKind, ErrorKind, FerrisError, MouseButton, Result, ScreenInfo, map_desktop_point,
    screen_for_desktop_point,
};

pub const DOUBLE_CLICK_MS: u64 = 500;
pub const SCROLL_DEBOUNCE_MS: u64 = 250;
const DRAG_THRESHOLD: i32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlEvent {
    Pause,
    Resume,
    Stop,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub command: bool,
    pub control: bool,
    pub option: bool,
    pub shift: bool,
}

impl Modifiers {
    pub fn is_text_only(&self) -> bool {
        !self.command && !self.control && !self.option
    }

    pub fn hotkey_keys(&self, key: &str) -> Vec<String> {
        let mut keys = Vec::new();
        if self.control {
            keys.push("ctrl".to_string());
        }
        if self.option {
            keys.push("alt".to_string());
        }
        if self.shift {
            keys.push("shift".to_string());
        }
        if self.command {
            keys.push("cmd".to_string());
        }
        keys.push(key.to_string());
        keys
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RawInputEvent {
    MouseDown {
        at_ms: u64,
        x: i32,
        y: i32,
        button: MouseButton,
        click_count: u8,
    },
    MouseUp {
        at_ms: u64,
        x: i32,
        y: i32,
        button: MouseButton,
        click_count: u8,
    },
    MouseMove {
        at_ms: u64,
        x: i32,
        y: i32,
    },
    Scroll {
        at_ms: u64,
        x: i32,
        y: i32,
        delta_x: i32,
        delta_y: i32,
    },
    KeyDown {
        at_ms: u64,
        key: String,
        text: Option<String>,
        modifiers: Modifiers,
        repeat: bool,
    },
    Tick {
        at_ms: u64,
    },
    Control {
        at_ms: u64,
        control: ControlEvent,
    },
}

impl RawInputEvent {
    pub fn at_ms(&self) -> u64 {
        match self {
            Self::MouseDown { at_ms, .. }
            | Self::MouseUp { at_ms, .. }
            | Self::MouseMove { at_ms, .. }
            | Self::Scroll { at_ms, .. }
            | Self::KeyDown { at_ms, .. }
            | Self::Tick { at_ms }
            | Self::Control { at_ms, .. } => *at_ms,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointReason {
    Click,
    DoubleClick,
    Drag,
    Scroll,
    BoundaryKey,
    Hotkey,
    Stop,
}

impl CheckpointReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::DoubleClick => "double_click",
            Self::Drag => "drag",
            Self::Scroll => "scroll",
            Self::BoundaryKey => "boundary_key",
            Self::Hotkey => "hotkey",
            Self::Stop => "stop",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "click" => Some(Self::Click),
            "double_click" => Some(Self::DoubleClick),
            "drag" => Some(Self::Drag),
            "scroll" => Some(Self::Scroll),
            "boundary_key" => Some(Self::BoundaryKey),
            "hotkey" => Some(Self::Hotkey),
            "stop" => Some(Self::Stop),
            "none" => None,
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticStep {
    pub occurred_at_ms: u64,
    pub started_at_ms: u64,
    pub action: ActionKind,
    pub checkpoint: Option<CheckpointReason>,
    pub external_state: Option<String>,
}

#[derive(Debug, Clone)]
struct MouseDownState {
    at_ms: u64,
    x: i32,
    y: i32,
    button: MouseButton,
}

#[derive(Debug, Clone)]
struct PendingClick {
    step: SemanticStep,
}

#[derive(Debug, Clone)]
struct PendingScroll {
    started_at_ms: u64,
    last_at_ms: u64,
    x: i32,
    y: i32,
    delta_x: i32,
    delta_y: i32,
}

pub struct SemanticReducer {
    screens: Vec<ScreenInfo>,
    pending_text: String,
    text_started_at_ms: Option<u64>,
    last_text_at_ms: u64,
    mouse_down: Option<MouseDownState>,
    last_drag_point: Option<(i32, i32)>,
    pending_click: Option<PendingClick>,
    pending_scroll: Option<PendingScroll>,
    paused: bool,
}

impl SemanticReducer {
    pub fn new(screens: Vec<ScreenInfo>) -> Self {
        Self {
            screens,
            pending_text: String::new(),
            text_started_at_ms: None,
            last_text_at_ms: 0,
            mouse_down: None,
            last_drag_point: None,
            pending_click: None,
            pending_scroll: None,
            paused: false,
        }
    }

    pub fn push(&mut self, event: RawInputEvent) -> Result<Vec<SemanticStep>> {
        let mut emitted = Vec::new();
        match event {
            RawInputEvent::Control { at_ms, control } => match control {
                ControlEvent::Pause => {
                    emitted.extend(self.flush_all(at_ms)?);
                    self.paused = true;
                }
                ControlEvent::Resume => self.paused = false,
                ControlEvent::Stop => {
                    emitted.extend(self.flush_all(at_ms)?);
                }
            },
            RawInputEvent::Tick { at_ms } => {
                if !self.paused {
                    emitted.extend(self.flush_expired(at_ms)?);
                }
            }
            _ if self.paused => {}
            RawInputEvent::MouseDown {
                at_ms,
                x,
                y,
                button,
                ..
            } => {
                emitted.extend(self.flush_text()?);
                emitted.extend(self.flush_scroll()?);
                self.mouse_down = Some(MouseDownState {
                    at_ms,
                    x,
                    y,
                    button,
                });
                self.last_drag_point = Some((x, y));
            }
            RawInputEvent::MouseMove { x, y, .. } => {
                if self.mouse_down.is_some() {
                    self.last_drag_point = Some((x, y));
                }
            }
            RawInputEvent::MouseUp {
                at_ms,
                x,
                y,
                button,
                click_count,
            } => {
                let Some(down) = self.mouse_down.take() else {
                    return Ok(emitted);
                };
                self.last_drag_point = None;
                let moved = (x - down.x).abs().max((y - down.y).abs());
                if moved > DRAG_THRESHOLD {
                    emitted.extend(self.take_pending_click());
                    emitted.push(self.drag_step(down, at_ms, x, y)?);
                } else if down.button == button {
                    let click = self.click_step(down.at_ms, at_ms, x, y, button)?;
                    if click_count >= 2 {
                        self.pending_click.take();
                        emitted.push(SemanticStep {
                            checkpoint: Some(CheckpointReason::DoubleClick),
                            action: match click.action {
                                ActionKind::Click {
                                    screen_id,
                                    x,
                                    y,
                                    button,
                                } => ActionKind::DoubleClick {
                                    screen_id,
                                    x,
                                    y,
                                    button,
                                },
                                action => action,
                            },
                            ..click
                        });
                    } else {
                        if self.pending_click.is_some() {
                            emitted.extend(self.take_pending_click());
                        }
                        self.pending_click = Some(PendingClick { step: click });
                    }
                }
            }
            RawInputEvent::Scroll {
                at_ms,
                x,
                y,
                delta_x,
                delta_y,
            } => {
                emitted.extend(self.flush_text()?);
                emitted.extend(self.take_pending_click());
                match &mut self.pending_scroll {
                    Some(scroll)
                        if at_ms.saturating_sub(scroll.last_at_ms) <= SCROLL_DEBOUNCE_MS =>
                    {
                        scroll.last_at_ms = at_ms;
                        scroll.x = x;
                        scroll.y = y;
                        scroll.delta_x = scroll.delta_x.saturating_add(delta_x);
                        scroll.delta_y = scroll.delta_y.saturating_add(delta_y);
                    }
                    Some(_) => {
                        emitted.extend(self.flush_scroll()?);
                        self.pending_scroll = Some(PendingScroll {
                            started_at_ms: at_ms,
                            last_at_ms: at_ms,
                            x,
                            y,
                            delta_x,
                            delta_y,
                        });
                    }
                    None => {
                        self.pending_scroll = Some(PendingScroll {
                            started_at_ms: at_ms,
                            last_at_ms: at_ms,
                            x,
                            y,
                            delta_x,
                            delta_y,
                        });
                    }
                }
            }
            RawInputEvent::KeyDown {
                at_ms,
                key,
                text,
                modifiers,
                repeat: _,
            } => {
                emitted.extend(self.take_pending_click());
                emitted.extend(self.flush_scroll()?);
                if modifiers.is_text_only()
                    && text.as_ref().is_some_and(|value| !value.is_empty())
                    && !is_boundary_key(&key)
                {
                    if self.text_started_at_ms.is_none() {
                        self.text_started_at_ms = Some(at_ms);
                    }
                    self.pending_text
                        .push_str(text.as_deref().unwrap_or_default());
                    self.last_text_at_ms = at_ms;
                } else {
                    emitted.extend(self.flush_text()?);
                    if !modifiers.is_text_only() {
                        let keys = modifiers.hotkey_keys(&key);
                        emitted.push(SemanticStep {
                            occurred_at_ms: at_ms,
                            started_at_ms: at_ms,
                            external_state: keys
                                .iter()
                                .any(|key| key.eq_ignore_ascii_case("v"))
                                .then(|| "clipboard".to_string()),
                            action: ActionKind::Hotkey { keys },
                            checkpoint: Some(CheckpointReason::Hotkey),
                        });
                    } else {
                        emitted.push(SemanticStep {
                            occurred_at_ms: at_ms,
                            started_at_ms: at_ms,
                            action: ActionKind::PressKey { key: key.clone() },
                            checkpoint: is_boundary_key(&key)
                                .then_some(CheckpointReason::BoundaryKey),
                            external_state: None,
                        });
                    }
                }
            }
        }
        Ok(emitted)
    }

    pub fn finish(&mut self, at_ms: u64) -> Result<Vec<SemanticStep>> {
        self.flush_all(at_ms)
    }

    fn flush_all(&mut self, _at_ms: u64) -> Result<Vec<SemanticStep>> {
        let mut emitted = self.flush_text()?;
        emitted.extend(self.take_pending_click());
        emitted.extend(self.flush_scroll()?);
        Ok(emitted)
    }

    fn flush_expired(&mut self, at_ms: u64) -> Result<Vec<SemanticStep>> {
        let mut emitted = Vec::new();
        if self
            .pending_click
            .as_ref()
            .is_some_and(|click| at_ms.saturating_sub(click.step.occurred_at_ms) >= DOUBLE_CLICK_MS)
        {
            emitted.extend(self.take_pending_click());
        }
        if self
            .pending_scroll
            .as_ref()
            .is_some_and(|scroll| at_ms.saturating_sub(scroll.last_at_ms) >= SCROLL_DEBOUNCE_MS)
        {
            emitted.extend(self.flush_scroll()?);
        }
        Ok(emitted)
    }

    fn flush_text(&mut self) -> Result<Vec<SemanticStep>> {
        if self.pending_text.is_empty() {
            return Ok(Vec::new());
        }
        let text = std::mem::take(&mut self.pending_text);
        let started_at_ms = self
            .text_started_at_ms
            .take()
            .unwrap_or(self.last_text_at_ms);
        Ok(vec![SemanticStep {
            occurred_at_ms: self.last_text_at_ms,
            started_at_ms,
            action: ActionKind::Type { text },
            checkpoint: None,
            external_state: None,
        }])
    }

    fn take_pending_click(&mut self) -> Vec<SemanticStep> {
        self.pending_click
            .take()
            .map(|click| click.step)
            .into_iter()
            .collect()
    }

    fn flush_scroll(&mut self) -> Result<Vec<SemanticStep>> {
        let Some(scroll) = self.pending_scroll.take() else {
            return Ok(Vec::new());
        };
        let screen = screen_for_desktop_point(scroll.x, scroll.y, &self.screens)?;
        let (x, y) = map_desktop_point(scroll.x, scroll.y, screen)?;
        Ok(vec![SemanticStep {
            occurred_at_ms: scroll.last_at_ms,
            started_at_ms: scroll.started_at_ms,
            action: ActionKind::Scroll {
                screen_id: Some(screen.screen_id.clone()),
                x: Some(x),
                y: Some(y),
                delta_x: scroll.delta_x.clamp(-2000, 2000),
                delta_y: scroll.delta_y.clamp(-2000, 2000),
            },
            checkpoint: Some(CheckpointReason::Scroll),
            external_state: None,
        }])
    }

    fn click_step(
        &self,
        started_at_ms: u64,
        occurred_at_ms: u64,
        x: i32,
        y: i32,
        button: MouseButton,
    ) -> Result<SemanticStep> {
        let screen = screen_for_desktop_point(x, y, &self.screens)?;
        let (x, y) = map_desktop_point(x, y, screen)?;
        let action = match button {
            MouseButton::Right => ActionKind::RightClick {
                screen_id: Some(screen.screen_id.clone()),
                x,
                y,
            },
            _ => ActionKind::Click {
                screen_id: Some(screen.screen_id.clone()),
                x,
                y,
                button,
            },
        };
        Ok(SemanticStep {
            occurred_at_ms,
            started_at_ms,
            action,
            checkpoint: Some(CheckpointReason::Click),
            external_state: None,
        })
    }

    fn drag_step(
        &self,
        down: MouseDownState,
        occurred_at_ms: u64,
        x: i32,
        y: i32,
    ) -> Result<SemanticStep> {
        let from_screen = screen_for_desktop_point(down.x, down.y, &self.screens)?;
        let to_screen = screen_for_desktop_point(x, y, &self.screens)?;
        if from_screen.screen_id != to_screen.screen_id {
            return Err(FerrisError::new(
                ErrorKind::Coordinate,
                "cross-screen drag recording is not yet supported by the action protocol",
            ));
        }
        let (from_x, from_y) = map_desktop_point(down.x, down.y, from_screen)?;
        let (to_x, to_y) = map_desktop_point(x, y, to_screen)?;
        Ok(SemanticStep {
            occurred_at_ms,
            started_at_ms: down.at_ms,
            action: ActionKind::Drag {
                screen_id: Some(from_screen.screen_id.clone()),
                from_x,
                from_y,
                to_x,
                to_y,
                duration_ms: occurred_at_ms.saturating_sub(down.at_ms).min(5000),
                button: down.button,
            },
            checkpoint: Some(CheckpointReason::Drag),
            external_state: None,
        })
    }
}

fn is_boundary_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "enter"
            | "return"
            | "tab"
            | "escape"
            | "esc"
            | "up"
            | "down"
            | "left"
            | "right"
            | "home"
            | "end"
            | "pageup"
            | "pagedown"
            | "insert"
            | "delete"
            | "f1"
            | "f2"
            | "f3"
            | "f4"
            | "f5"
            | "f6"
            | "f7"
            | "f8"
            | "f9"
            | "f10"
            | "f11"
            | "f12"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> ScreenInfo {
        ScreenInfo {
            screen_id: "screen-1".to_string(),
            display_fingerprint: "test".to_string(),
            name: "Test".to_string(),
            is_primary: true,
            origin_x: 0,
            origin_y: 0,
            logical_width: 1000,
            logical_height: 1000,
            native_width: 2000,
            native_height: 2000,
            scale_factor: 2.0,
        }
    }

    #[test]
    fn groups_typing_until_enter_without_per_character_checkpoints() {
        let mut reducer = SemanticReducer::new(vec![screen()]);
        for (at_ms, value) in [(10, "h"), (20, "i")] {
            assert!(
                reducer
                    .push(RawInputEvent::KeyDown {
                        at_ms,
                        key: value.to_string(),
                        text: Some(value.to_string()),
                        modifiers: Modifiers::default(),
                        repeat: false,
                    })
                    .unwrap()
                    .is_empty()
            );
        }
        let steps = reducer
            .push(RawInputEvent::KeyDown {
                at_ms: 30,
                key: "enter".to_string(),
                text: None,
                modifiers: Modifiers::default(),
                repeat: false,
            })
            .unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].action, ActionKind::Type { text: "hi".into() });
        assert_eq!(steps[0].checkpoint, None);
        assert_eq!(steps[1].checkpoint, Some(CheckpointReason::BoundaryKey));
    }

    #[test]
    fn aggregates_scroll_burst_after_idle_tick() {
        let mut reducer = SemanticReducer::new(vec![screen()]);
        for at_ms in [10, 40, 100] {
            assert!(
                reducer
                    .push(RawInputEvent::Scroll {
                        at_ms,
                        x: 500,
                        y: 500,
                        delta_x: 0,
                        delta_y: -10,
                    })
                    .unwrap()
                    .is_empty()
            );
        }
        let steps = reducer.push(RawInputEvent::Tick { at_ms: 351 }).unwrap();
        assert_eq!(steps.len(), 1);
        assert!(matches!(
            &steps[0].action,
            ActionKind::Scroll { delta_y: -30, .. }
        ));
    }
}
