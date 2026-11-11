//! The Win32 personality the ported code expects, implemented on platform-neutral state.
//!
//! The ported functions keep calling the Win32 API exactly where the original does (through
//! `orig!` declarations of the import, ADR-001 §4). In phase A those calls reach Wine/Windows. In
//! phase B there is no Windows: `sa-game` binds the declarations to the functions of this module
//! (docs/arch/platform.md §Binding), which reproduce the documented Win32 semantics the game relies
//! on:
//!
//! * keyboard: physical key → virtual-key code, scan code, extended flag ([`key_info`]), the
//!   `lParam` bit layout of `WM_KEYDOWN`/`WM_KEYUP`/`WM_SYSKEY*` ([`key_lparam`]), `MapVirtualKeyA`
//!   for the US layout ([`map_virtual_key`]), `GetKeyState`/`GetAsyncKeyState` ([`KeyStates`]);
//! * the message queue (`PeekMessageA`/`TranslateMessage`/`DispatchMessageA`): [`MessageQueue`]
//!   turns [`InputEvent`]s into `MSG` records in arrival order;
//! * time: `QueryPerformanceFrequency`/`QueryPerformanceCounter`/`timeGetTime`/`GetTickCount` on a
//!   monotonic [`Clock`] ([`Win32Time`]).
//!
//! Everything here is target-independent (no OS calls), so the web build (phase C) uses it too.

use std::collections::VecDeque;

use crate::clock::Clock;
use crate::input::{InputEvent, Key, MouseButton};

// ------------------------------------------------------------------------------- messages ---

pub const WM_SIZE: u32 = 0x0005;
pub const WM_ACTIVATE: u32 = 0x0006;
pub const WM_SETFOCUS: u32 = 0x0007;
pub const WM_KILLFOCUS: u32 = 0x0008;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_QUIT: u32 = 0x0012;
pub const WM_ACTIVATEAPP: u32 = 0x001C;
pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_KEYUP: u32 = 0x0101;
pub const WM_CHAR: u32 = 0x0102;
pub const WM_SYSKEYDOWN: u32 = 0x0104;
pub const WM_SYSKEYUP: u32 = 0x0105;
pub const WM_MOUSEMOVE: u32 = 0x0200;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_RBUTTONDOWN: u32 = 0x0204;
pub const WM_RBUTTONUP: u32 = 0x0205;
pub const WM_MBUTTONDOWN: u32 = 0x0207;
pub const WM_MBUTTONUP: u32 = 0x0208;
pub const WM_MOUSEWHEEL: u32 = 0x020A;
pub const WM_XBUTTONDOWN: u32 = 0x020B;
pub const WM_XBUTTONUP: u32 = 0x020C;

/// `PeekMessageA` `wRemoveMsg` flag.
pub const PM_REMOVE: u32 = 0x0001;

/// `MSG` (28 bytes on 32-bit Windows: hwnd, message, wParam, lParam, time, pt.x, pt.y), as the
/// ported `MessageLoop` hands it to `PeekMessageA` (`[u32; 7]`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Msg {
    pub hwnd: u32,
    pub message: u32,
    pub wparam: u32,
    pub lparam: u32,
    pub time: u32,
    pub pt: (i32, i32),
}

impl Msg {
    /// The seven dwords of the 32-bit `MSG`.
    pub fn to_words(&self) -> [u32; 7] {
        [
            self.hwnd,
            self.message,
            self.wparam,
            self.lparam,
            self.time,
            self.pt.0 as u32,
            self.pt.1 as u32,
        ]
    }
}

// -------------------------------------------------------------------------- virtual keys ---

pub const VK_SHIFT: u8 = 0x10;
pub const VK_CONTROL: u8 = 0x11;
pub const VK_MENU: u8 = 0x12;
pub const VK_CAPITAL: u8 = 0x14;
pub const VK_NUMLOCK: u8 = 0x90;
pub const VK_SCROLL: u8 = 0x91;
pub const VK_LSHIFT: u8 = 0xA0;
pub const VK_RSHIFT: u8 = 0xA1;
pub const VK_LCONTROL: u8 = 0xA2;
pub const VK_RCONTROL: u8 = 0xA3;
pub const VK_LMENU: u8 = 0xA4;
pub const VK_RMENU: u8 = 0xA5;
pub const VK_F10: u8 = 0x79;

/// What Windows reports for a physical key (US layout, Num Lock on).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyInfo {
    /// `wParam` of `WM_KEYDOWN` (generic `VK_SHIFT`/`VK_CONTROL`/`VK_MENU` for the modifiers).
    pub vk: u8,
    /// Left/right-specific virtual key (`VK_LSHIFT` …), else `vk`.
    pub vk_side: u8,
    /// Set 1 scan code (`lParam` bits 16..23).
    pub scan: u8,
    /// `lParam` bit 24 (`KF_EXTENDED`).
    pub extended: bool,
}

const fn ki(vk: u8, scan: u8, extended: bool) -> KeyInfo {
    KeyInfo {
        vk,
        vk_side: vk,
        scan,
        extended,
    }
}

const fn kis(vk: u8, vk_side: u8, scan: u8, extended: bool) -> KeyInfo {
    KeyInfo {
        vk,
        vk_side,
        scan,
        extended,
    }
}

const LETTER_SCAN: [u8; 26] = [
    0x1E, 0x30, 0x2E, 0x20, 0x12, 0x21, 0x22, 0x23, 0x17, 0x24, 0x25, 0x26, 0x32, 0x31, 0x18, 0x19, 0x10,
    0x13, 0x1F, 0x14, 0x16, 0x2F, 0x11, 0x2D, 0x15, 0x2C,
];

const LETTERS: [Key; 26] = [
    Key::A,
    Key::B,
    Key::C,
    Key::D,
    Key::E,
    Key::F,
    Key::G,
    Key::H,
    Key::I,
    Key::J,
    Key::K,
    Key::L,
    Key::M,
    Key::N,
    Key::O,
    Key::P,
    Key::Q,
    Key::R,
    Key::S,
    Key::T,
    Key::U,
    Key::V,
    Key::W,
    Key::X,
    Key::Y,
    Key::Z,
];

/// Numpad digit scan codes 0..=9.
const NUMPAD_SCAN: [u8; 10] = [0x52, 0x4F, 0x50, 0x51, 0x4B, 0x4C, 0x4D, 0x47, 0x48, 0x49];
/// F1..=F24 scan codes.
const FN_SCAN: [u8; 24] = [
    0x3B, 0x3C, 0x3D, 0x3E, 0x3F, 0x40, 0x41, 0x42, 0x43, 0x44, 0x57, 0x58, 0x64, 0x65, 0x66, 0x67, 0x68,
    0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E, 0x76,
];

/// The Win32 view of a physical key, `None` for keys Windows has no code for (out-of-range
/// `Digit`/`Fn`/`Numpad` values).
pub fn key_info(k: Key) -> Option<KeyInfo> {
    if let Some(i) = LETTERS.iter().position(|&l| l == k) {
        return Some(ki(b'A' + i as u8, LETTER_SCAN[i], false));
    }
    Some(match k {
        Key::Digit(d @ 0..=9) => ki(b'0' + d, if d == 0 { 0x0B } else { 0x01 + d }, false),
        Key::Fn(n @ 1..=24) => ki(0x6F + n, FN_SCAN[n as usize - 1], false),
        Key::Numpad(d @ 0..=9) => ki(0x60 + d, NUMPAD_SCAN[d as usize], false),
        Key::NumpadMultiply => ki(0x6A, 0x37, false),
        Key::NumpadAdd => ki(0x6B, 0x4E, false),
        Key::NumpadSubtract => ki(0x6D, 0x4A, false),
        Key::NumpadDecimal => ki(0x6E, 0x53, false),
        Key::NumpadDivide => ki(0x6F, 0x35, true),
        Key::NumpadEnter => ki(0x0D, 0x1C, true),
        Key::NumLock => ki(VK_NUMLOCK, 0x45, true),
        Key::Escape => ki(0x1B, 0x01, false),
        Key::Minus => ki(0xBD, 0x0C, false),
        Key::Equal => ki(0xBB, 0x0D, false),
        Key::Backspace => ki(0x08, 0x0E, false),
        Key::Tab => ki(0x09, 0x0F, false),
        Key::BracketLeft => ki(0xDB, 0x1A, false),
        Key::BracketRight => ki(0xDD, 0x1B, false),
        Key::Enter => ki(0x0D, 0x1C, false),
        Key::Semicolon => ki(0xBA, 0x27, false),
        Key::Quote => ki(0xDE, 0x28, false),
        Key::Backquote => ki(0xC0, 0x29, false),
        Key::Backslash => ki(0xDC, 0x2B, false),
        Key::Comma => ki(0xBC, 0x33, false),
        Key::Period => ki(0xBE, 0x34, false),
        Key::Slash => ki(0xBF, 0x35, false),
        Key::IntlBackslash => ki(0xE2, 0x56, false),
        Key::Space => ki(0x20, 0x39, false),
        Key::CapsLock => ki(VK_CAPITAL, 0x3A, false),
        Key::ScrollLock => ki(VK_SCROLL, 0x46, false),
        Key::Pause => ki(0x13, 0x45, false),
        Key::PrintScreen => ki(0x2C, 0x37, true),
        Key::ShiftLeft => kis(VK_SHIFT, VK_LSHIFT, 0x2A, false),
        Key::ShiftRight => kis(VK_SHIFT, VK_RSHIFT, 0x36, false),
        Key::ControlLeft => kis(VK_CONTROL, VK_LCONTROL, 0x1D, false),
        Key::ControlRight => kis(VK_CONTROL, VK_RCONTROL, 0x1D, true),
        Key::AltLeft => kis(VK_MENU, VK_LMENU, 0x38, false),
        Key::AltRight => kis(VK_MENU, VK_RMENU, 0x38, true),
        Key::MetaLeft => ki(0x5B, 0x5B, true),
        Key::MetaRight => ki(0x5C, 0x5C, true),
        Key::ContextMenu => ki(0x5D, 0x5D, true),
        Key::Home => ki(0x24, 0x47, true),
        Key::Up => ki(0x26, 0x48, true),
        Key::PageUp => ki(0x21, 0x49, true),
        Key::Left => ki(0x25, 0x4B, true),
        Key::Right => ki(0x27, 0x4D, true),
        Key::End => ki(0x23, 0x4F, true),
        Key::Down => ki(0x28, 0x50, true),
        Key::PageDown => ki(0x22, 0x51, true),
        Key::Insert => ki(0x2D, 0x52, true),
        Key::Delete => ki(0x2E, 0x53, true),
        _ => return None,
    })
}

/// Every key [`key_info`] knows (for reverse lookups and tests).
pub fn all_keys() -> Vec<Key> {
    let mut v: Vec<Key> = LETTERS.to_vec();
    v.extend((0..=9).map(Key::Digit));
    v.extend((1..=24).map(Key::Fn));
    v.extend((0..=9).map(Key::Numpad));
    v.extend([
        Key::NumpadMultiply,
        Key::NumpadAdd,
        Key::NumpadSubtract,
        Key::NumpadDecimal,
        Key::NumpadDivide,
        Key::NumpadEnter,
        Key::NumLock,
        Key::Escape,
        Key::Minus,
        Key::Equal,
        Key::Backspace,
        Key::Tab,
        Key::BracketLeft,
        Key::BracketRight,
        Key::Enter,
        Key::Semicolon,
        Key::Quote,
        Key::Backquote,
        Key::Backslash,
        Key::Comma,
        Key::Period,
        Key::Slash,
        Key::IntlBackslash,
        Key::Space,
        Key::CapsLock,
        Key::ScrollLock,
        Key::Pause,
        Key::PrintScreen,
        Key::ShiftLeft,
        Key::ShiftRight,
        Key::ControlLeft,
        Key::ControlRight,
        Key::AltLeft,
        Key::AltRight,
        Key::MetaLeft,
        Key::MetaRight,
        Key::ContextMenu,
        Key::Home,
        Key::Up,
        Key::PageUp,
        Key::Left,
        Key::Right,
        Key::End,
        Key::Down,
        Key::PageDown,
        Key::Insert,
        Key::Delete,
    ]);
    v
}

/// `lParam` of a key message: repeat count 1, scan code, `KF_EXTENDED` (bit 24), context code
/// (bit 29: Alt held), previous key state (bit 30), transition state (bit 31, set on key up).
pub fn key_lparam(info: KeyInfo, down: bool, was_down: bool, alt: bool) -> u32 {
    let mut l = 1 | (info.scan as u32) << 16;
    if info.extended {
        l |= 1 << 24;
    }
    if alt {
        l |= 1 << 29;
    }
    if was_down || !down {
        l |= 1 << 30;
    }
    if !down {
        l |= 1 << 31;
    }
    l
}

/// `MapVirtualKeyA` for the US layout. `kind`: 0 `MAPVK_VK_TO_VSC`, 1 `MAPVK_VSC_TO_VK`,
/// 2 `MAPVK_VK_TO_CHAR` (unshifted character, letters upper case, 0 if none), 3
/// `MAPVK_VSC_TO_VK_EX` (left/right-specific). Unknown codes give 0, as Windows does.
pub fn map_virtual_key(code: u32, kind: u32) -> u32 {
    let keys = all_keys();
    let infos = keys.iter().filter_map(|&k| key_info(k));
    match kind {
        0 => {
            // generic modifiers map to the left key's scan code
            let vk = match code as u8 {
                VK_LSHIFT | VK_SHIFT => return 0x2A,
                VK_RSHIFT => return 0x36,
                VK_LCONTROL | VK_RCONTROL | VK_CONTROL => return 0x1D,
                VK_LMENU | VK_RMENU | VK_MENU => return 0x38,
                v => v,
            };
            if code > 0xFF {
                return 0;
            }
            let mut found: Vec<KeyInfo> = infos.filter(|i| i.vk == vk).collect();
            found.sort_by_key(|i| i.extended);
            found.first().map_or(0, |i| i.scan as u32)
        }
        1 | 3 => infos
            .filter(|i| !i.extended && i.scan as u32 == code)
            .map(|i| if kind == 3 { i.vk_side } else { i.vk } as u32)
            .next()
            .unwrap_or(0),
        2 => match code {
            0x30..=0x39 | 0x41..=0x5A => code,
            0x60..=0x69 => code - 0x60 + u32::from(b'0'),
            0x08 | 0x09 | 0x0D | 0x1B | 0x20 => code,
            0x6A => u32::from(b'*'),
            0x6B => u32::from(b'+'),
            0x6D => u32::from(b'-'),
            0x6E => u32::from(b'.'),
            0x6F => u32::from(b'/'),
            0xBA => u32::from(b';'),
            0xBB => u32::from(b'='),
            0xBC => u32::from(b','),
            0xBD => u32::from(b'-'),
            0xBE => u32::from(b'.'),
            0xBF => u32::from(b'/'),
            0xC0 => u32::from(b'`'),
            0xDB => u32::from(b'['),
            0xDC => u32::from(b'\\'),
            0xDD => u32::from(b']'),
            0xDE => u32::from(b'\''),
            0xE2 => u32::from(b'\\'),
            _ => 0,
        },
        _ => 0,
    }
}

/// `GetKeyState`/`GetAsyncKeyState` table: bit 15 = down, bit 0 = toggled (Caps/Num/Scroll Lock
/// and every key, as Windows keeps a toggle bit for each).
#[derive(Debug, Clone)]
pub struct KeyStates {
    down: [bool; 256],
    toggled: [bool; 256],
}

impl Default for KeyStates {
    fn default() -> Self {
        let mut toggled = [false; 256];
        // Num Lock on (the numpad tables above assume it)
        toggled[VK_NUMLOCK as usize] = true;
        KeyStates {
            down: [false; 256],
            toggled,
        }
    }
}

impl KeyStates {
    /// Records a key transition; returns whether the key was already down.
    pub fn set(&mut self, info: KeyInfo, down: bool) -> bool {
        let was = self.down[info.vk_side as usize];
        for vk in [info.vk, info.vk_side] {
            if down && !self.down[vk as usize] {
                self.toggled[vk as usize] ^= true;
            }
            self.down[vk as usize] = down;
        }
        // a generic modifier stays down while the other side is held
        let pairs = [
            (VK_SHIFT, VK_LSHIFT, VK_RSHIFT),
            (VK_CONTROL, VK_LCONTROL, VK_RCONTROL),
            (VK_MENU, VK_LMENU, VK_RMENU),
        ];
        for (g, l, r) in pairs {
            if info.vk == g {
                self.down[g as usize] = self.down[l as usize] || self.down[r as usize];
            }
        }
        was
    }

    pub fn is_down(&self, vk: u8) -> bool {
        self.down[vk as usize]
    }

    /// `GetKeyState(vk)` as the `SHORT` the API returns, sign-extended to `i32` like the
    /// original's `movsx`/`test ah, 0x80` readers expect (`0xFF80`-style values are negative).
    pub fn get_key_state(&self, vk: i32) -> i32 {
        let vk = (vk & 0xFF) as usize;
        let mut s: u16 = 0;
        if self.down[vk] {
            s |= 0x8000;
        }
        if self.toggled[vk] {
            s |= 1;
        }
        s as i16 as i32
    }

    /// `GetAsyncKeyState(vk)`: bit 15 = down now (the "pressed since last call" bit 0 is not
    /// emulated; Microsoft documents it as unreliable).
    pub fn get_async_key_state(&self, vk: i32) -> i32 {
        if self.down[(vk & 0xFF) as usize] {
            0x8000u16 as i16 as i32
        } else {
            0
        }
    }
}

/// The thread message queue of the game window.
#[derive(Debug, Default)]
pub struct MessageQueue {
    queue: VecDeque<Msg>,
    pub keys: KeyStates,
    /// Window handle written into every message (any non-zero value; the game compares it with
    /// `PSGLOBAL(window)` only in `WndProc`).
    pub hwnd: u32,
    cursor: (i32, i32),
    quit: Option<u32>,
}

impl MessageQueue {
    pub fn new(hwnd: u32) -> MessageQueue {
        MessageQueue {
            hwnd,
            ..Default::default()
        }
    }

    pub fn post(&mut self, message: u32, wparam: u32, lparam: u32, time: u32) {
        self.queue.push_back(Msg {
            hwnd: self.hwnd,
            message,
            wparam,
            lparam,
            time,
            pt: self.cursor,
        });
    }

    /// `PostQuitMessage(code)`: `WM_QUIT` is returned once the queue is otherwise empty.
    pub fn post_quit(&mut self, code: u32) {
        self.quit = Some(code);
    }

    /// `PeekMessageA(msg, NULL, 0, 0, remove)`.
    pub fn peek(&mut self, remove: bool) -> Option<Msg> {
        if let Some(m) = if remove {
            self.queue.pop_front()
        } else {
            self.queue.front().copied()
        } {
            return Some(m);
        }
        let code = self.quit?;
        if remove {
            self.quit = None;
        }
        Some(Msg {
            hwnd: 0,
            message: WM_QUIT,
            wparam: code,
            lparam: 0,
            time: 0,
            pt: self.cursor,
        })
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty() && self.quit.is_none()
    }

    /// Mouse-button flags (`MK_*`) for `wParam` of mouse messages.
    fn mk_flags(&self) -> u32 {
        let mut f = 0;
        for (vk, bit) in [
            (0x01u8, 0x01u32),
            (0x02, 0x02),
            (0x04, 0x10),
            (0x05, 0x20),
            (0x06, 0x40),
        ] {
            if self.keys.is_down(vk) {
                f |= bit;
            }
        }
        if self.keys.is_down(VK_SHIFT) {
            f |= 0x04;
        }
        if self.keys.is_down(VK_CONTROL) {
            f |= 0x08;
        }
        f
    }

    fn button(&mut self, b: MouseButton, down: bool, time: u32) {
        let (vk, msg_down, msg_up, x) = match b {
            MouseButton::Left => (0x01u8, WM_LBUTTONDOWN, WM_LBUTTONUP, 0),
            MouseButton::Right => (0x02, WM_RBUTTONDOWN, WM_RBUTTONUP, 0),
            MouseButton::Middle => (0x04, WM_MBUTTONDOWN, WM_MBUTTONUP, 0),
            MouseButton::X1 => (0x05, WM_XBUTTONDOWN, WM_XBUTTONUP, 1),
            MouseButton::X2 => (0x06, WM_XBUTTONDOWN, WM_XBUTTONUP, 2),
        };
        self.keys.set(ki(vk, 0, false), down);
        let w = self.mk_flags() | x << 16;
        let l = (self.cursor.1 as u32 & 0xFFFF) << 16 | (self.cursor.0 as u32 & 0xFFFF);
        self.post(if down { msg_down } else { msg_up }, w, l, time);
    }

    /// Appends the messages Windows would queue for `e` (`time` = `GetTickCount` now).
    pub fn push_event(&mut self, e: &InputEvent, time: u32) {
        match *e {
            InputEvent::KeyDown { key, .. } | InputEvent::KeyUp { key } => {
                let down = matches!(e, InputEvent::KeyDown { .. });
                let Some(info) = key_info(key) else { return };
                let was = self.keys.set(info, down);
                let alt = self.keys.is_down(VK_MENU);
                // Alt combinations and F10 are system keys; releasing Alt itself is WM_SYSKEYUP
                let sys = alt || info.vk == VK_F10 || (info.vk == VK_MENU && !down);
                let msg = match (down, sys) {
                    (true, false) => WM_KEYDOWN,
                    (true, true) => WM_SYSKEYDOWN,
                    (false, false) => WM_KEYUP,
                    (false, true) => WM_SYSKEYUP,
                };
                let ctx = alt && info.vk != VK_MENU || (info.vk == VK_MENU && down);
                self.post(msg, info.vk as u32, key_lparam(info, down, was, ctx), time);
            }
            InputEvent::Char(c) => {
                // TranslateMessage's WM_CHAR, in the ANSI code page (characters outside Latin-1
                // have no CP1252 byte and are dropped)
                let code = c as u32;
                if code < 0x100 {
                    self.post(WM_CHAR, code, 1, time);
                }
            }
            InputEvent::MouseDown(b) => self.button(b, true, time),
            InputEvent::MouseUp(b) => self.button(b, false, time),
            InputEvent::MouseMove { x, y } => {
                self.cursor = (x, y);
                let l = (y as u32 & 0xFFFF) << 16 | (x as u32 & 0xFFFF);
                let w = self.mk_flags();
                self.post(WM_MOUSEMOVE, w, l, time);
            }
            InputEvent::Wheel(lines) => {
                let delta = (lines * 120.0).round() as i32 as u32 & 0xFFFF;
                let w = delta << 16 | self.mk_flags();
                let l = (self.cursor.1 as u32 & 0xFFFF) << 16 | (self.cursor.0 as u32 & 0xFFFF);
                self.post(WM_MOUSEWHEEL, w, l, time);
            }
            InputEvent::Focus(on) => {
                self.post(WM_ACTIVATEAPP, u32::from(on), 0, time);
                self.post(WM_ACTIVATE, u32::from(on), 0, time);
                self.post(if on { WM_SETFOCUS } else { WM_KILLFOCUS }, 0, 0, time);
            }
            InputEvent::Resized { width, height } => {
                self.post(WM_SIZE, 0, (height & 0xFFFF) << 16 | (width & 0xFFFF), time);
            }
            InputEvent::Close => self.post(WM_CLOSE, 0, 0, time),
        }
    }
}

// ------------------------------------------------------------------------------------ time ---

/// `QueryPerformanceFrequency` value: 10 MHz, what Wine (and Windows 10+ with an invariant TSC)
/// reports, so phase B sees the same `CTimer::m_snTimerDivider` (10,000) as the phase A runs.
pub const QPC_FREQUENCY: u64 = 10_000_000;

/// The Win32 time sources on a monotonic clock.
pub struct Win32Time<C: Clock> {
    pub clock: C,
    /// `timeGetTime`/`GetTickCount` value at clock zero (Windows: milliseconds since boot).
    pub tick_base: u32,
}

impl<C: Clock> Win32Time<C> {
    pub fn new(clock: C) -> Self {
        Win32Time { clock, tick_base: 0 }
    }

    /// `QueryPerformanceFrequency(&f)`: always succeeds.
    pub fn query_performance_frequency(&self) -> u64 {
        QPC_FREQUENCY
    }

    /// `QueryPerformanceCounter(&c)`: 100 ns ticks since the clock's origin.
    pub fn query_performance_counter(&self) -> u64 {
        (self.clock.now_seconds() * QPC_FREQUENCY as f64) as u64
    }

    /// `timeGetTime()` (and `GetTickCount()`): milliseconds, wrapping at 2^32.
    pub fn time_get_time(&self) -> u32 {
        let ms = (self.clock.now_seconds() * 1000.0) as u64;
        self.tick_base.wrapping_add(ms as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_table_is_consistent() {
        let keys = all_keys();
        let mut seen = std::collections::HashSet::new();
        for k in &keys {
            let i = key_info(*k).unwrap_or_else(|| panic!("{k:?}"));
            assert!(seen.insert((i.vk_side, i.scan, i.extended)), "duplicate {k:?}");
        }
        assert_eq!(key_info(Key::W), Some(ki(0x57, 0x11, false)));
        assert_eq!(key_info(Key::Fn(12)).map(|i| (i.vk, i.scan)), Some((0x7B, 0x58)));
        assert_eq!(
            key_info(Key::Digit(0)).map(|i| (i.vk, i.scan)),
            Some((0x30, 0x0B))
        );
        assert_eq!(key_info(Key::Digit(10)), None);
    }

    #[test]
    fn map_virtual_key_us() {
        assert_eq!(map_virtual_key(0x41, 2), 'A' as u32);
        assert_eq!(map_virtual_key(0xBD, 2), '-' as u32);
        assert_eq!(map_virtual_key(0x70, 2), 0);
        assert_eq!(map_virtual_key(0x57, 0), 0x11);
        assert_eq!(map_virtual_key(0x26, 0), 0x48);
        assert_eq!(map_virtual_key(0x11, 1), 0x57);
        assert_eq!(map_virtual_key(0x36, 3), VK_RSHIFT as u32);
        assert_eq!(map_virtual_key(0x36, 1), VK_SHIFT as u32);
        assert_eq!(map_virtual_key(VK_LCONTROL as u32, 0), 0x1D);
    }

    #[test]
    fn key_messages() {
        let mut q = MessageQueue::new(1);
        q.push_event(
            &InputEvent::KeyDown {
                key: Key::Up,
                repeat: false,
            },
            5,
        );
        q.push_event(
            &InputEvent::KeyDown {
                key: Key::Up,
                repeat: true,
            },
            6,
        );
        q.push_event(&InputEvent::KeyUp { key: Key::Up }, 7);
        let m = q.peek(true).unwrap();
        assert_eq!(
            (m.message, m.wparam, m.lparam, m.time),
            (WM_KEYDOWN, 0x26, 0x0148_0001, 5)
        );
        assert_eq!(q.peek(true).unwrap().lparam, 0x4148_0001);
        assert_eq!(q.peek(true).unwrap().lparam, 0xC148_0001);
        assert!(q.peek(true).is_none());
        q.post_quit(0);
        assert_eq!(q.peek(false).unwrap().message, WM_QUIT);
        assert_eq!(q.peek(true).unwrap().message, WM_QUIT);
        assert!(q.peek(true).is_none());
    }

    #[test]
    fn alt_is_a_system_key() {
        let mut q = MessageQueue::new(1);
        q.push_event(
            &InputEvent::KeyDown {
                key: Key::AltLeft,
                repeat: false,
            },
            0,
        );
        q.push_event(
            &InputEvent::KeyDown {
                key: Key::Enter,
                repeat: false,
            },
            0,
        );
        q.push_event(&InputEvent::KeyUp { key: Key::Enter }, 0);
        q.push_event(&InputEvent::KeyUp { key: Key::AltLeft }, 0);
        let v: Vec<(u32, u32)> = std::iter::from_fn(|| q.peek(true))
            .map(|m| (m.message, m.lparam >> 29))
            .collect();
        assert_eq!(
            v,
            vec![
                (WM_SYSKEYDOWN, 1),
                (WM_SYSKEYDOWN, 1),
                (WM_SYSKEYUP, 0b111),
                (WM_SYSKEYUP, 0b110)
            ]
        );
    }

    #[test]
    fn key_state_modifiers() {
        let mut k = KeyStates::default();
        k.set(key_info(Key::ShiftLeft).unwrap(), true);
        k.set(key_info(Key::ShiftRight).unwrap(), true);
        k.set(key_info(Key::ShiftLeft).unwrap(), false);
        assert!(k.get_key_state(VK_SHIFT as i32) < 0);
        assert!(k.get_key_state(VK_RSHIFT as i32) < 0);
        assert!(k.get_key_state(VK_LSHIFT as i32) >= 0);
        assert_eq!(k.get_key_state(VK_NUMLOCK as i32), 1);
    }

    struct Fixed(f64);
    impl Clock for Fixed {
        fn now_seconds(&self) -> f64 {
            self.0
        }
    }

    #[test]
    fn time_sources() {
        let t = Win32Time {
            clock: Fixed(1.5),
            tick_base: u32::MAX - 499,
        };
        assert_eq!(t.query_performance_counter(), 15_000_000);
        assert_eq!(t.time_get_time(), 1000);
        assert_eq!(t.query_performance_frequency() as u32 / 1000, 10_000);
    }
}
