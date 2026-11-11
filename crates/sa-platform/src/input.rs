//! Layout-independent keys, per-frame input state, and the DirectInput 8 device-state records the
//! ported input code reads (`DIMOUSESTATE2`, `DIJOYSTATE2`).
//!
//! The ported `CPad` code gets input three ways in the original (docs/arch/platform.md §Input):
//! keyboard through Win32 window messages (`WM_KEYDOWN` → `GTATranslateKey` →
//! `RsKeyboardEventHandler` → `CPad::TempKeyState`), the mouse through DirectInput 8
//! (`GetDeviceState(DIMOUSESTATE2)` in `CPad::GetMouseState`) and pads through DirectInput 8
//! (`GetDeviceState(DIJOYSTATE2)` in `CPad::UpdatePads`). This module keeps the platform-neutral
//! state; [`crate::win32`] turns key events into the exact Win32 messages, and [`DiMouseState2`] /
//! [`DiJoyState2`] produce the byte records the DirectInput stand-ins hand back.

use std::collections::HashSet;

/// Physical keys (positions on a US layout, like KeyboardEvent `code` on the web).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    /// Top-row digit 0..=9.
    Digit(u8),
    /// Function key 1..=24.
    Fn(u8),
    /// Numpad digit 0..=9.
    Numpad(u8),
    NumpadDivide,
    NumpadMultiply,
    NumpadSubtract,
    NumpadAdd,
    NumpadDecimal,
    NumpadEnter,
    NumLock,
    Space,
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    MetaLeft,
    MetaRight,
    ContextMenu,
    CapsLock,
    Up,
    Down,
    Left,
    Right,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    PrintScreen,
    ScrollLock,
    Pause,
    BracketLeft,
    BracketRight,
    Backslash,
    Semicolon,
    Quote,
    Backquote,
    Comma,
    Period,
    Slash,
    Minus,
    Equal,
    /// The key between left Shift and Z on ISO keyboards (`VK_OEM_102`).
    IntlBackslash,
    Escape,
    Tab,
    Enter,
    Backspace,
}

/// Mouse buttons (DirectInput order: left, right, middle, X1, X2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

impl MouseButton {
    /// Index in `DIMOUSESTATE2::rgbButtons`.
    pub fn di_index(self) -> usize {
        match self {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            MouseButton::Middle => 2,
            MouseButton::X1 => 3,
            MouseButton::X2 => 4,
        }
    }
}

/// One input event, in arrival order (the Win32 message queue needs the order within a frame).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    /// A key went down (`repeat`: auto-repeat while held).
    KeyDown {
        key: Key,
        repeat: bool,
    },
    KeyUp {
        key: Key,
    },
    /// A character the layout produced (WM_CHAR; the game uses it only for text entry).
    Char(char),
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    /// Pointer position in client pixels.
    MouseMove {
        x: i32,
        y: i32,
    },
    /// Wheel lines (positive = away from the user).
    Wheel(f32),
    /// The window gained (`true`) or lost focus.
    Focus(bool),
    /// The drawable size changed.
    Resized {
        width: u32,
        height: u32,
    },
    /// The user asked to close the window.
    Close,
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
    /// Every event since the last frame, in order.
    pub events: Vec<InputEvent>,
    /// Connected game pads (index = DirectInput enumeration order).
    pub pads: Vec<GamepadState>,
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
        self.events.clear();
    }

    pub fn key_down(&mut self, k: Key) {
        if self.held.insert(k) {
            self.pressed.push(k);
            self.events.push(InputEvent::KeyDown {
                key: k,
                repeat: false,
            });
        }
    }

    /// An auto-repeat of a held key (Win32 sends `WM_KEYDOWN` with the previous-state bit set).
    pub fn key_repeat(&mut self, k: Key) {
        if self.held.contains(&k) {
            self.events.push(InputEvent::KeyDown { key: k, repeat: true });
        }
    }

    pub fn key_up(&mut self, k: Key) {
        if self.held.remove(&k) {
            self.events.push(InputEvent::KeyUp { key: k });
        }
    }

    /// Releases every held key and button (focus loss), with events.
    pub fn release_all(&mut self) {
        let mut keys: Vec<Key> = self.held.drain().collect();
        keys.sort_by_key(|k| format!("{k:?}"));
        for k in keys {
            self.events.push(InputEvent::KeyUp { key: k });
        }
        let mut b: Vec<MouseButton> = self.mouse_held.drain().collect();
        b.sort_by_key(|b| b.di_index());
        for b in b {
            self.events.push(InputEvent::MouseUp(b));
        }
    }

    pub fn mouse_down(&mut self, b: MouseButton) {
        if self.mouse_held.insert(b) {
            self.events.push(InputEvent::MouseDown(b));
        }
    }

    pub fn mouse_up(&mut self, b: MouseButton) {
        if self.mouse_held.remove(&b) {
            self.events.push(InputEvent::MouseUp(b));
        }
    }

    pub fn wheel_lines(&mut self, lines: f32) {
        self.wheel += lines;
        self.events.push(InputEvent::Wheel(lines));
    }

    /// The DirectInput mouse record for this frame (`CPad::GetMouseState`).
    pub fn di_mouse_state(&self) -> DiMouseState2 {
        DiMouseState2::from_input(self)
    }
}

/// `DIMOUSESTATE2` as the game reads it (`GetDeviceState(0x14, …)`): relative axes since the last
/// read (`DIPROPAXISMODE_REL`, the DirectInput default), `lZ` in wheel units of 120 per notch
/// (`WHEEL_DELTA`), `rgbButtons[i] & 0x80` = down. The game reads only the first 0x14 bytes of the
/// 0x14-byte structure (`lX, lY, lZ` + 8 buttons).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiMouseState2 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub buttons: [u8; 8],
}

impl DiMouseState2 {
    /// One notch of the wheel in DirectInput `lZ` units.
    pub const WHEEL_DELTA: i32 = 120;

    /// Relative motion, wheel and buttons since the last frame. Pixel deltas are rounded to whole
    /// mickeys (DirectInput reports integer counts).
    pub fn from_input(i: &InputState) -> DiMouseState2 {
        let mut s = DiMouseState2 {
            x: i.mouse_delta.0.round() as i32,
            y: i.mouse_delta.1.round() as i32,
            z: (i.wheel * Self::WHEEL_DELTA as f32).round() as i32,
            buttons: [0; 8],
        };
        for b in &i.mouse_held {
            s.buttons[b.di_index()] = 0x80;
        }
        s
    }

    /// The 0x14-byte record (little endian, as the game's `DIMOUSESTATE2` buffer).
    pub fn to_bytes(&self) -> [u8; 0x14] {
        let mut b = [0u8; 0x14];
        b[0..4].copy_from_slice(&self.x.to_le_bytes());
        b[4..8].copy_from_slice(&self.y.to_le_bytes());
        b[8..12].copy_from_slice(&self.z.to_le_bytes());
        b[12..20].copy_from_slice(&self.buttons);
        b
    }
}

/// A standard ("Xbox layout") game pad, as gilrs or the web Gamepad API report it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GamepadState {
    pub name: String,
    /// Left stick x/y, right stick x/y in -1..=1 (y positive = down, DirectInput convention).
    pub left: (f32, f32),
    pub right: (f32, f32),
    /// Analogue triggers 0..=1.
    pub left_trigger: f32,
    pub right_trigger: f32,
    /// Buttons in [`PadButton`] order.
    pub buttons: [bool; PadButton::COUNT],
}

/// Standard pad buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadButton {
    South,
    East,
    West,
    North,
    LeftShoulder,
    RightShoulder,
    Select,
    Start,
    LeftStick,
    RightStick,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    Guide,
}

impl PadButton {
    pub const COUNT: usize = 15;
}

/// `DIJOYSTATE2` (0x110 bytes) as the game reads it in `CPad::UpdatePads`
/// (`GetDeviceState(0x110, …)`).
///
/// Mapping of a standard pad onto the DirectInput view Windows gives an XInput pad (the layout the
/// game's `InitDefaultControlConfigJoyPad` was written against): `lX/lY` left stick, `lZ` the
/// triggers on one axis (left trigger towards the maximum, right towards the minimum, centre when
/// both or neither are pressed), `lRx/lRy` right stick; buttons 0..9 = A, B, X, Y, LB, RB, Back,
/// Start, LS, RS; `rgdwPOV[0]` = the D-pad in hundredths of a degree (0 = up, clockwise), 0xFFFFFFFF
/// centred, the other three POVs centred. Axis ranges are the ones the game sets with
/// `diPadSetRanges` (`DIPROP_RANGE` -2000..2000 for every axis), passed in as `range`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiJoyState2 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub rx: i32,
    pub ry: i32,
    pub rz: i32,
    pub sliders: [i32; 2],
    pub pov: [u32; 4],
    pub buttons: [u8; 128],
}

impl Default for DiJoyState2 {
    fn default() -> Self {
        DiJoyState2 {
            x: 0,
            y: 0,
            z: 0,
            rx: 0,
            ry: 0,
            rz: 0,
            sliders: [0; 2],
            pov: [u32::MAX; 4],
            buttons: [0; 128],
        }
    }
}

impl DiJoyState2 {
    /// The axis range `diPadSetRanges` sets (`DIPROP_RANGE` lMin/lMax).
    pub const GAME_RANGE: (i32, i32) = (-2000, 2000);

    pub fn from_pad(p: &GamepadState, range: (i32, i32)) -> DiJoyState2 {
        let (lo, hi) = range;
        let axis = |v: f32| -> i32 {
            let t = (v.clamp(-1.0, 1.0) + 1.0) * 0.5;
            lo + ((hi - lo) as f32 * t).round() as i32
        };
        let mut s = DiJoyState2 {
            x: axis(p.left.0),
            y: axis(p.left.1),
            z: axis(p.left_trigger - p.right_trigger),
            rx: axis(p.right.0),
            ry: axis(p.right.1),
            rz: axis(0.0),
            ..DiJoyState2::default()
        };
        let order = [
            PadButton::South,
            PadButton::East,
            PadButton::West,
            PadButton::North,
            PadButton::LeftShoulder,
            PadButton::RightShoulder,
            PadButton::Select,
            PadButton::Start,
            PadButton::LeftStick,
            PadButton::RightStick,
        ];
        for (i, b) in order.iter().enumerate() {
            if p.buttons[*b as usize] {
                s.buttons[i] = 0x80;
            }
        }
        let d = |b: PadButton| p.buttons[b as usize];
        let (u, dn, l, r) = (
            d(PadButton::DPadUp),
            d(PadButton::DPadDown),
            d(PadButton::DPadLeft),
            d(PadButton::DPadRight),
        );
        let dy = i32::from(dn) - i32::from(u);
        let dx = i32::from(r) - i32::from(l);
        s.pov[0] = match (dx, dy) {
            (0, -1) => 0,
            (1, -1) => 4500,
            (1, 0) => 9000,
            (1, 1) => 13500,
            (0, 1) => 18000,
            (-1, 1) => 22500,
            (-1, 0) => 27000,
            (-1, -1) => 31500,
            _ => u32::MAX,
        };
        s
    }

    /// The 0x110-byte record (`lX lY lZ lRx lRy lRz rglSlider[2] rgdwPOV[4] rgbButtons[128]` +
    /// velocity/acceleration/force fields, zero).
    pub fn to_bytes(&self) -> [u8; 0x110] {
        let mut b = [0u8; 0x110];
        let words = [
            self.x,
            self.y,
            self.z,
            self.rx,
            self.ry,
            self.rz,
            self.sliders[0],
            self.sliders[1],
        ];
        for (i, w) in words.iter().enumerate() {
            b[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
        }
        for (i, p) in self.pov.iter().enumerate() {
            b[0x20 + i * 4..0x24 + i * 4].copy_from_slice(&p.to_le_bytes());
        }
        b[0x30..0xB0].copy_from_slice(&self.buttons);
        b
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
        assert_eq!(
            i.events,
            vec![InputEvent::KeyDown {
                key: Key::W,
                repeat: false
            }]
        );
        i.end_frame();
        assert!(i.is_held(Key::W) && !i.was_pressed(Key::W) && i.events.is_empty());
        i.key_up(Key::W);
        assert!(!i.is_held(Key::W));
        assert_eq!(i.events, vec![InputEvent::KeyUp { key: Key::W }]);
    }

    #[test]
    fn mouse_record() {
        let mut i = InputState {
            mouse_delta: (3.4, -2.6),
            ..Default::default()
        };
        i.wheel_lines(-1.0);
        i.mouse_down(MouseButton::Right);
        let b = i.di_mouse_state().to_bytes();
        assert_eq!(&b[0..4], &3i32.to_le_bytes());
        assert_eq!(&b[4..8], &(-3i32).to_le_bytes());
        assert_eq!(&b[8..12], &(-120i32).to_le_bytes());
        assert_eq!(&b[12..20], &[0, 0x80, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn joy_record() {
        let mut p = GamepadState {
            left: (1.0, -1.0),
            left_trigger: 1.0,
            ..Default::default()
        };
        p.buttons[PadButton::Start as usize] = true;
        p.buttons[PadButton::DPadUp as usize] = true;
        p.buttons[PadButton::DPadRight as usize] = true;
        let s = DiJoyState2::from_pad(&p, DiJoyState2::GAME_RANGE);
        assert_eq!((s.x, s.y, s.z, s.rx), (2000, -2000, 2000, 0));
        assert_eq!(s.pov, [4500, u32::MAX, u32::MAX, u32::MAX]);
        let b = s.to_bytes();
        assert_eq!(b[0x30 + 7], 0x80);
        assert_eq!(&b[0x20..0x24], &4500u32.to_le_bytes());
        assert_eq!(&b[0x24..0x28], &u32::MAX.to_le_bytes());
    }
}
