//! Layout-independent keys and per-frame input state.

use std::collections::HashSet;

/// Physical keys (positions on a US layout, like KeyboardEvent `code` on the web).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    W,
    A,
    S,
    D,
    Q,
    E,
    R,
    F,
    T,
    G,
    Space,
    ShiftLeft,
    ControlLeft,
    AltLeft,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    BracketLeft,
    BracketRight,
    Comma,
    Period,
    Minus,
    Equal,
    Digit(u8),
    Fn(u8),
    Escape,
    Tab,
    Enter,
}

/// Mouse buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Input gathered since the last frame.
#[derive(Debug, Clone, Default)]
pub struct InputState {
    /// Keys currently held.
    pub held: HashSet<Key>,
    /// Keys that went down since the last frame (in order).
    pub pressed: Vec<Key>,
    pub mouse_held: HashSet<MouseButton>,
    /// Relative mouse motion since the last frame (pixels, device units).
    pub mouse_delta: (f64, f64),
    /// Wheel lines since the last frame (positive = up).
    pub wheel: f32,
    /// Whether the pointer is captured (relative mode).
    pub pointer_captured: bool,
}

impl InputState {
    pub fn is_held(&self, k: Key) -> bool {
        self.held.contains(&k)
    }

    pub fn was_pressed(&self, k: Key) -> bool {
        self.pressed.contains(&k)
    }

    /// Clears the per-frame parts (called by the platform after each frame).
    pub fn end_frame(&mut self) {
        self.pressed.clear();
        self.mouse_delta = (0.0, 0.0);
        self.wheel = 0.0;
    }

    pub fn key_down(&mut self, k: Key) {
        if self.held.insert(k) {
            self.pressed.push(k);
        }
    }

    pub fn key_up(&mut self, k: Key) {
        self.held.remove(&k);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_is_edge_triggered() {
        let mut i = InputState::default();
        i.key_down(Key::W);
        i.key_down(Key::W);
        assert_eq!(i.pressed, vec![Key::W]);
        i.end_frame();
        assert!(i.is_held(Key::W) && !i.was_pressed(Key::W));
        i.key_up(Key::W);
        assert!(!i.is_held(Key::W));
    }
}
