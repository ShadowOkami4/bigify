//! Controller input.
//!
//! Controllers are read with gilrs (evdev on Linux) on a background thread
//! and translated into navigation [`Action`]s. Actions are delivered to Qt as
//! ordinary key events (see `cpp/input.cpp`), so QML has a single navigation
//! path for controllers, remotes and keyboards.
//!
//! Directional input repeats while held (like a TV remote): a first repeat
//! after [`REPEAT_DELAY`], then every [`REPEAT_INTERVAL`].

use std::time::{Duration, Instant};

pub const REPEAT_DELAY: Duration = Duration::from_millis(380);
pub const REPEAT_INTERVAL: Duration = Duration::from_millis(90);
/// Stick deflection that counts as a press, and the lower release threshold
/// (hysteresis prevents chatter around the threshold).
pub const STICK_PRESS: f32 = 0.55;
pub const STICK_RELEASE: f32 = 0.35;

/// Keep values in sync with `keyForAction` in `cpp/input.cpp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Action {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
    Activate = 4,
    Back = 5,
    PlayPause = 6,
    More = 7,
    Previous = 8,
    Next = 9,
    VolumeDown = 10,
    VolumeUp = 11,
    NowPlaying = 12,
    Queue = 13,
}

impl Action {
    /// Whether holding the input should repeat the action.
    pub fn repeats(self) -> bool {
        matches!(
            self,
            Action::Up
                | Action::Down
                | Action::Left
                | Action::Right
                | Action::VolumeDown
                | Action::VolumeUp
        )
    }
}

/// Output of the input state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub action: Action,
    pub pressed: bool,
    pub autorepeat: bool,
}

/// Tracks held directions and stick state; pure logic, unit tested.
#[derive(Debug, Default)]
pub struct InputState {
    held: Vec<(Action, Instant, Instant)>,
    stick_x: Option<Action>,
    stick_y: Option<Action>,
}

impl InputState {
    pub fn press(&mut self, action: Action, now: Instant, out: &mut Vec<KeyEvent>) {
        if self.held.iter().any(|(a, ..)| *a == action) {
            return;
        }
        if action.repeats() {
            // A new direction cancels the old one, like a D-pad.
            let released: Vec<Action> = self
                .held
                .iter()
                .map(|(a, ..)| *a)
                .filter(|a| is_direction(*a) && is_direction(action))
                .collect();
            for a in released {
                self.release(a, out);
            }
            self.held.push((action, now, now + REPEAT_DELAY));
        }
        out.push(KeyEvent {
            action,
            pressed: true,
            autorepeat: false,
        });
        if !action.repeats() {
            // Non-repeating actions are delivered as a full click.
            out.push(KeyEvent {
                action,
                pressed: false,
                autorepeat: false,
            });
        }
    }

    pub fn release(&mut self, action: Action, out: &mut Vec<KeyEvent>) {
        let before = self.held.len();
        self.held.retain(|(a, ..)| *a != action);
        if self.held.len() != before {
            out.push(KeyEvent {
                action,
                pressed: false,
                autorepeat: false,
            });
        }
    }

    /// Emit repeats that are due. Returns the time until the next repeat.
    pub fn tick(&mut self, now: Instant, out: &mut Vec<KeyEvent>) -> Option<Duration> {
        let mut next: Option<Duration> = None;
        for (action, _, due) in &mut self.held {
            if now >= *due {
                out.push(KeyEvent {
                    action: *action,
                    pressed: true,
                    autorepeat: true,
                });
                *due = now + REPEAT_INTERVAL;
            }
            let wait = due.saturating_duration_since(now);
            next = Some(next.map_or(wait, |n| n.min(wait)));
        }
        next
    }

    /// Feed an analog stick axis value (-1..1). `positive`/`negative` are
    /// the actions for each direction.
    pub fn axis(
        &mut self,
        vertical: bool,
        value: f32,
        positive: Action,
        negative: Action,
        now: Instant,
        out: &mut Vec<KeyEvent>,
    ) {
        let current = if vertical { self.stick_y } else { self.stick_x };
        let target = match current {
            Some(a)
                if (a == positive && value > STICK_RELEASE)
                    || (a == negative && value < -STICK_RELEASE) =>
            {
                Some(a)
            }
            _ if value >= STICK_PRESS => Some(positive),
            _ if value <= -STICK_PRESS => Some(negative),
            _ => None,
        };
        if target == current {
            return;
        }
        if let Some(a) = current {
            self.release(a, out);
        }
        if let Some(a) = target {
            self.press(a, now, out);
        }
        if vertical {
            self.stick_y = target;
        } else {
            self.stick_x = target;
        }
    }
}

fn is_direction(a: Action) -> bool {
    matches!(a, Action::Up | Action::Down | Action::Left | Action::Right)
}

#[cfg(feature = "gamepad")]
pub mod gamepad {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use gilrs::{Axis, Button, EventType, Gilrs};

    use super::{Action, InputState, KeyEvent};

    fn map_button(b: Button) -> Option<Action> {
        Some(match b {
            Button::DPadUp => Action::Up,
            Button::DPadDown => Action::Down,
            Button::DPadLeft => Action::Left,
            Button::DPadRight => Action::Right,
            Button::South => Action::Activate,
            Button::East => Action::Back,
            Button::West => Action::PlayPause,
            Button::North => Action::More,
            Button::LeftTrigger => Action::Previous,
            Button::RightTrigger => Action::Next,
            Button::LeftTrigger2 => Action::VolumeDown,
            Button::RightTrigger2 => Action::VolumeUp,
            Button::Start => Action::NowPlaying,
            Button::Select => Action::Queue,
            // Mode/Guide belongs to Steam and the system.
            _ => return None,
        })
    }

    /// Run the controller loop until `stop` is set. `deliver` is called with
    /// batches of key events and must be cheap (it queues onto the Qt thread).
    /// `enabled` lets the user turn controller input off at runtime.
    pub fn run(stop: Arc<AtomicBool>, enabled: Arc<AtomicBool>, deliver: impl Fn(Vec<KeyEvent>)) {
        let mut gilrs = match Gilrs::new() {
            Ok(g) => g,
            Err(e) => {
                log::warn!("controller input unavailable: {e}");
                return;
            }
        };
        for (_, pad) in gilrs.gamepads() {
            log::info!("controller connected: {}", pad.name());
        }
        let mut state = InputState::default();
        let mut wait = Duration::from_millis(250);
        while !stop.load(Ordering::Relaxed) {
            let mut out = Vec::new();
            // Block until an event arrives or the next repeat is due, so an
            // idle controller costs no CPU.
            if let Some(ev) = gilrs.next_event_blocking(Some(wait)) {
                let now = Instant::now();
                if enabled.load(Ordering::Relaxed) {
                    match ev.event {
                        EventType::ButtonPressed(b, _) => {
                            if let Some(a) = map_button(b) {
                                state.press(a, now, &mut out);
                            }
                        }
                        EventType::ButtonReleased(b, _) => {
                            if let Some(a) = map_button(b) {
                                state.release(a, &mut out);
                            }
                        }
                        EventType::AxisChanged(Axis::LeftStickX, v, _) => {
                            state.axis(false, v, Action::Right, Action::Left, now, &mut out)
                        }
                        // gilrs reports up as positive Y.
                        EventType::AxisChanged(Axis::LeftStickY, v, _) => {
                            state.axis(true, v, Action::Up, Action::Down, now, &mut out)
                        }
                        EventType::Connected => {
                            log::info!("controller connected: {}", gilrs.gamepad(ev.id).name())
                        }
                        EventType::Disconnected => {
                            log::info!("controller disconnected");
                            state = InputState::default();
                        }
                        _ => {}
                    }
                }
            }
            wait = state
                .tick(Instant::now(), &mut out)
                .unwrap_or(Duration::from_millis(250))
                .max(Duration::from_millis(5));
            if !out.is_empty() {
                deliver(out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_click_and_directions_repeat() {
        let mut s = InputState::default();
        let t0 = Instant::now();
        let mut out = Vec::new();
        s.press(Action::Activate, t0, &mut out);
        assert_eq!(out.len(), 2, "press+release for a click");
        out.clear();
        s.press(Action::Down, t0, &mut out);
        assert_eq!(out.len(), 1);
        out.clear();
        assert!(s.tick(t0 + Duration::from_millis(100), &mut out).is_some());
        assert!(out.is_empty(), "no repeat before the delay");
        s.tick(t0 + REPEAT_DELAY, &mut out);
        assert_eq!(out.len(), 1);
        assert!(out[0].autorepeat);
        out.clear();
        s.release(Action::Down, &mut out);
        assert_eq!(out.len(), 1);
        assert!(!out[0].pressed);
        assert!(s
            .tick(t0 + Duration::from_secs(5), &mut Vec::new())
            .is_none());
    }

    #[test]
    fn new_direction_replaces_old() {
        let mut s = InputState::default();
        let t0 = Instant::now();
        let mut out = Vec::new();
        s.press(Action::Left, t0, &mut out);
        s.press(Action::Up, t0, &mut out);
        let releases: Vec<_> = out.iter().filter(|e| !e.pressed).collect();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].action, Action::Left);
    }

    #[test]
    fn stick_has_hysteresis() {
        let mut s = InputState::default();
        let t0 = Instant::now();
        let mut out = Vec::new();
        s.axis(false, 0.5, Action::Right, Action::Left, t0, &mut out);
        assert!(out.is_empty(), "below press threshold");
        s.axis(false, 0.6, Action::Right, Action::Left, t0, &mut out);
        assert_eq!(out.len(), 1);
        out.clear();
        s.axis(false, 0.4, Action::Right, Action::Left, t0, &mut out);
        assert!(out.is_empty(), "still held above release threshold");
        s.axis(false, 0.1, Action::Right, Action::Left, t0, &mut out);
        assert_eq!(out.len(), 1);
        assert!(!out[0].pressed);
        out.clear();
        s.axis(false, -0.9, Action::Right, Action::Left, t0, &mut out);
        assert_eq!(out[0].action, Action::Left);
    }
}
