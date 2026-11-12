//! The platform input path of the SA:MP UI: every frame's input events go to the SA:MP host first
//! (`NativeGame::window_event`: chat input, dialogs, textdraw selection, then the client's
//! `GameHooks::window_event`) as Windows-style window events; consumed ones never reach the game's message
//! queue. While the UI has the keyboard (chat line or dialog open: samp's input lock) the held keys and
//! mouse buttons are hidden from the game too, so typing does not move CJ.
//!
//! The host registers itself with [`set_target`]; without one the input is untouched.

use std::cell::Cell;

use sa_platform::input::{InputEvent, InputState, MouseButton};
use sa_platform::win32::key_info;
use sa_samp_api::input::{EventResult, WindowEvent};

/// The host: delivers one event, and tells what the UI locks.
#[derive(Clone, Copy)]
pub struct Target {
    /// Delivers a window event.
    pub event: fn(WindowEvent) -> EventResult,
    /// samp's input lock: (keyboard, mouse). Keyboard while the chat line or a dialog is open; mouse also
    /// while the cursor is up for a dialog, the scoreboard, textdraw selection or a locked cursor mode.
    pub lock: fn() -> (bool, bool),
}

thread_local! {
    static TARGET: Cell<Option<Target>> = const { Cell::new(None) };
}

/// Registers the host (replaces an earlier one).
pub fn set_target(t: Target) {
    TARGET.with(|c| c.set(Some(t)));
}

/// The window event of a platform event (`None`: nothing samp reacts to).
pub fn window_event(e: &InputEvent) -> Option<WindowEvent> {
    let button = |b: &MouseButton| match b {
        MouseButton::Left => Some(0),
        MouseButton::Right => Some(1),
        MouseButton::Middle => Some(2),
        MouseButton::X1 | MouseButton::X2 => None,
    };
    Some(match e {
        InputEvent::KeyDown { key, repeat } => WindowEvent::KeyDown {
            vk: u16::from(key_info(*key)?.vk),
            repeat: *repeat,
        },
        InputEvent::KeyUp { key } => WindowEvent::KeyUp {
            vk: u16::from(key_info(*key)?.vk),
        },
        InputEvent::Char(c) => WindowEvent::Char {
            ch: u32::from(*c),
            unicode: true,
        },
        InputEvent::MouseDown(b) => WindowEvent::MouseButton {
            button: button(b)?,
            down: true,
        },
        InputEvent::MouseUp(b) => WindowEvent::MouseButton {
            button: button(b)?,
            down: false,
        },
        InputEvent::MouseMove { x, y } => WindowEvent::MouseMove { x: *x, y: *y },
        InputEvent::Wheel(lines) => {
            WindowEvent::Wheel((lines * 120.0).round().clamp(-32768.0, 32767.0) as i16)
        }
        InputEvent::Focus(f) => WindowEvent::Focus(*f),
        InputEvent::Resized { .. } | InputEvent::Close => return None,
    })
}

/// Runs this frame's input through the host and removes what it consumed.
pub fn filter(input: &mut InputState) {
    let Some(t) = TARGET.with(Cell::get) else { return };
    input.events.retain(|e| match window_event(e) {
        Some(w) => (t.event)(w) == EventResult::PassToGame,
        None => true,
    });
    let (keyboard, mouse) = (t.lock)();
    if keyboard {
        input.held.clear();
        input.pressed.clear();
    }
    if keyboard || mouse {
        input.mouse_held.clear();
        input.mouse_delta = (0.0, 0.0);
        input.wheel = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_platform::input::Key;

    #[test]
    fn platform_events_map_to_windows_events() {
        assert_eq!(
            window_event(&InputEvent::KeyDown {
                key: Key::T,
                repeat: false
            }),
            Some(WindowEvent::KeyDown {
                vk: 0x54,
                repeat: false
            })
        );
        assert_eq!(
            window_event(&InputEvent::KeyUp { key: Key::Enter }),
            Some(WindowEvent::KeyUp { vk: 0x0D })
        );
        assert_eq!(
            window_event(&InputEvent::Char('é')),
            Some(WindowEvent::Char {
                ch: 0xE9,
                unicode: true
            })
        );
        assert_eq!(
            window_event(&InputEvent::Wheel(-1.0)),
            Some(WindowEvent::Wheel(-120))
        );
        assert_eq!(
            window_event(&InputEvent::MouseDown(MouseButton::Left)),
            Some(WindowEvent::MouseButton {
                button: 0,
                down: true
            })
        );
        assert_eq!(window_event(&InputEvent::Close), None);
    }

    #[test]
    fn consumed_events_are_dropped_and_keys_hidden_while_typing() {
        fn ev(e: WindowEvent) -> EventResult {
            match e {
                WindowEvent::Char { .. } => EventResult::Consumed,
                _ => EventResult::PassToGame,
            }
        }
        set_target(Target {
            event: ev,
            lock: || (true, true),
        });
        let mut i = InputState::default();
        i.held.insert(Key::W);
        i.events = vec![InputEvent::Char('a'), InputEvent::KeyUp { key: Key::W }];
        filter(&mut i);
        assert_eq!(i.events, vec![InputEvent::KeyUp { key: Key::W }]);
        assert!(i.held.is_empty());
        // Mouse only (the scoreboard): keys stay, the camera does not turn.
        set_target(Target {
            event: ev,
            lock: || (false, true),
        });
        let mut i = InputState::default();
        i.held.insert(Key::W);
        i.mouse_delta = (3.0, 1.0);
        filter(&mut i);
        assert!(i.held.contains(&Key::W) && i.mouse_delta == (0.0, 0.0));
        TARGET.with(|c| c.set(None));
    }
}
