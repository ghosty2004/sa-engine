//! The front end shared by every platform backend (ADR-003 §3.1, §3.6): the native window ([`crate::game_window`],
//! winit) and the browser worker ([`crate::web`], wasm32) are two backends of this one front end. A backend turns its
//! platform's events into calls of [`FrontEnd`]'s input methods, drives the main loop by calling
//! [`FrontEnd::iterate`] (or [`FrontEnd::take`] / [`FrontEnd::after`]) once per main-loop iteration, and asks
//! [`in_menu`] whether the pointer is the game's (captured, relative motion) or the menu's (free, absolute positions).
//!
//! The input pipeline: platform events → [`InputState`] (here) → the Win32 message queue and the DirectInput state
//! ([`Session::iterate`]) → the game. Its rules, the same on every backend:
//!
//! - relative mouse motion and wheel lines accumulate until the game's `GetDeviceState` reads them (kept by
//!   `Session::iterate`; this pipeline hands each motion over once);
//! - the game reads its input only in the iterations that run a frame (the ones that present); between two frames
//!   the main loop may run many iterations that only peek messages (always in the browser, where the frame limiter
//!   does not sleep). So a key pressed since the last frame stays pending until an iteration presents, and a key
//!   released before any frame saw it held is released after the next frame instead (a quick tap, or one shorter
//!   than a slow frame, still reaches `CPad`). Window-message events go to the queue once.
//!
//! No platform `cfg` here: those belong to the backends.

use sa_platform::input::{GamepadState, InputEvent, InputState, Key, MouseButton};

use crate::app::{Session, merge_input, synthetic_input};

/// The game's client size in pixels (its default video mode, 800 x 600): pointer positions are given in these.
pub const CLIENT_WIDTH: u32 = 800;
pub const CLIENT_HEIGHT: u32 = 600;

/// A position in a drawable of `size` pixels as the game's client pixels (the back buffer is stretched over it).
pub fn to_client(x: i32, y: i32, size: (u32, u32)) -> (i32, i32) {
    (
        (x as i64 * CLIENT_WIDTH as i64 / size.0.max(1) as i64) as i32,
        (y as i64 * CLIENT_HEIGHT as i64 / size.1.max(1) as i64) as i32,
    )
}

/// The menus (front end, or the pause menu over the game: `FrontEndMenuManager.m_bMenuActive` with gGameState
/// IDLE), or the SA:MP UI showing its cursor (dialogs, scoreboard, class-selection panel): the game / the UI draws its
/// own cursor at the `WM_MOUSEMOVE` position, so the backend does not capture the pointer and sends absolute
/// positions and clicks. Elsewhere the game owns the pointer (DirectInput exclusive mode: captured, relative motion).
pub fn in_menu() -> bool {
    crate::winmain::game_state() != crate::winmain::STATE_IDLE
        || sa_core::game::MENU_ACTIVE.get().get()
        || crate::samp::ui_cursor_visible()
}

/// Frames presented so far (every `Present`): an iteration that presents ran a game frame.
fn presents() -> u32 {
    crate::d3d9::PRESENTS.load(std::sync::atomic::Ordering::Relaxed)
}

/// The input gathered since the last main-loop iteration and the key rules above.
#[derive(Default)]
pub struct FrontEnd {
    /// Platform input not yet handed to an iteration (held keys and buttons persist).
    pending: InputState,
    /// Keys released before a frame saw them held, with the present count at their key-down.
    deferred_up: Vec<(Key, u32)>,
    /// The present count when each held key went down.
    down_at: Vec<(Key, u32)>,
    /// The offscreen mode's synthetic input is added to every iteration (`--auto-input`, `SA_WEB_AUTO_INPUT`).
    auto_input: bool,
}

impl FrontEnd {
    pub fn new(auto_input: bool) -> FrontEnd {
        FrontEnd {
            auto_input,
            ..FrontEnd::default()
        }
    }

    /// A key going down (`repeat`: auto-repeat while held) or up.
    pub fn key(&mut self, k: Key, down: bool, repeat: bool) {
        let now = presents();
        match (down, repeat) {
            (true, false) => {
                self.deferred_up.retain(|&(x, _)| x != k);
                self.down_at.retain(|&(x, _)| x != k);
                self.down_at.push((k, now));
                self.pending.key_down(k)
            }
            (true, true) => self.pending.key_repeat(k),
            (false, _) => {
                let at = self.down_at.iter().find(|&&(x, _)| x == k).map(|&(_, a)| a);
                self.down_at.retain(|&(x, _)| x != k);
                // no frame since the key went down: release it after the next one
                match at {
                    Some(at) if at >= now && self.pending.held.contains(&k) => self.deferred_up.push((k, at)),
                    _ => self.pending.key_up(k),
                }
            }
        }
    }

    /// A character the layout produced (`WM_CHAR`).
    pub fn text(&mut self, c: char) {
        self.pending.events.push(InputEvent::Char(c));
    }

    /// Relative pointer motion (device units; DirectInput's mouse axes).
    pub fn mouse_motion(&mut self, dx: f64, dy: f64) {
        self.pending.mouse_delta.0 += dx;
        self.pending.mouse_delta.1 += dy;
    }

    /// Absolute pointer position in the game's client pixels (`WM_MOUSEMOVE`, which the menus read for their cursor).
    pub fn mouse_pos(&mut self, x: i32, y: i32) {
        self.pending.events.push(InputEvent::MouseMove { x, y });
    }

    /// A mouse button going down or up.
    pub fn mouse_button(&mut self, b: MouseButton, down: bool) {
        if down { self.pending.mouse_down(b) } else { self.pending.mouse_up(b) }
    }

    /// Wheel lines (positive = away from the user).
    pub fn wheel(&mut self, lines: f32) {
        self.pending.wheel_lines(lines);
    }

    /// Focus lost: every held key and button is released.
    pub fn blur(&mut self) {
        self.deferred_up.clear();
        self.down_at.clear();
        self.pending.release_all();
    }

    /// Any other platform event, passed to the message queue as it is (focus, resize, close).
    pub fn event(&mut self, e: InputEvent) {
        self.pending.events.push(e);
    }

    /// The connected game pads (polled by the backend).
    pub fn set_pads(&mut self, pads: Vec<GamepadState>) {
        self.pending.pads = pads;
    }

    /// The input of the next main-loop iteration, and the present count before it (for [`FrontEnd::after`]). Keys
    /// whose release was deferred are released now if a frame has run since they went down.
    pub fn take(&mut self, n: u32) -> (InputState, u32) {
        let now = presents();
        let pending = &mut self.pending;
        self.deferred_up.retain(|&(k, at)| {
            if now > at {
                pending.key_up(k);
                false
            } else {
                true
            }
        });
        let out = pending.clone();
        pending.events.clear();
        // the mouse motion and the wheel are handed over once: `Session::iterate` keeps them until the game's
        // `GetDeviceState` reads them (DirectInput semantics), so passing the pending sum again would count it twice
        pending.mouse_delta = (0.0, 0.0);
        pending.wheel = 0.0;
        let input = if self.auto_input { merge_input(&out, &synthetic_input(n)) } else { out };
        (input, now)
    }

    /// After the iteration [`FrontEnd::take`] fed: when it ran a frame (presented), the keys pressed for it are done.
    pub fn after(&mut self, presents_before: u32) {
        if presents() != presents_before {
            self.pending.end_frame();
        }
    }

    /// One main-loop iteration of `s` with the pending input; false once the loop has stopped.
    pub fn iterate(&mut self, s: &mut Session) -> bool {
        let (input, before) = self.take(s.n);
        let more = s.iterate(input);
        self.after(before);
        more
    }
}

// Host tests: the pipeline's rules (no session needed; PRESENTS stays 0 unless a test moves it).
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn a_tap_between_frames_reaches_the_next_frame() {
        let mut f = FrontEnd::new(false);
        let p0 = presents();
        f.key(Key::W, true, false);
        f.key(Key::W, false, false);
        // released before any frame: still held for the next iteration, its release deferred
        let (i, before) = f.take(0);
        assert!(i.held.contains(&Key::W));
        assert_eq!(before, p0);
        crate::d3d9::PRESENTS.fetch_add(1, Ordering::Relaxed);
        f.after(before);
        let (i, _) = f.take(1);
        assert!(!i.held.contains(&Key::W));
        assert!(i.events.contains(&InputEvent::KeyUp { key: Key::W }));
    }

    #[test]
    fn motion_is_handed_over_once() {
        let mut f = FrontEnd::new(false);
        f.mouse_motion(3.0, -1.0);
        f.wheel(1.0);
        let (i, _) = f.take(0);
        assert_eq!((i.mouse_delta, i.wheel), ((3.0, -1.0), 1.0));
        let (i, _) = f.take(1);
        assert_eq!((i.mouse_delta, i.wheel), ((0.0, 0.0), 0.0));
        assert!(i.events.is_empty());
    }

    #[test]
    fn client_positions_scale_to_the_video_mode() {
        assert_eq!(to_client(1600, 1200, (1600, 1200)), (800, 600));
        assert_eq!(to_client(400, 300, (800, 600)), (400, 300));
        assert_eq!(to_client(5, 5, (0, 0)), (4000, 3000));
    }
}
