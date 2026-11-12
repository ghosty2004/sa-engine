//! Pads, keyboard and mouse. samp-rs today: `ui/inputlock.rs`, `ui/wndproc.rs`, `cursorhook.rs`,
//! `ui/glue/input.rs`, `frametimer.rs`, `remote.rs` (key injection), `game.rs`. Mapping:
//! `docs/samp/game-adapter.md` §4.7.

use crate::GameResult;

/// One pad state (`CControllerState`, 0x30 bytes; `CPad::Pads` 0xB73458: NewState +0x0, OldState +0x30).
/// Values are the game's raw shorts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadState {
    /// Left stick x.
    pub left_x: i16,
    /// Left stick y.
    pub left_y: i16,
    /// Right stick x.
    pub right_x: i16,
    /// Right stick y.
    pub right_y: i16,
    /// Shoulders: L1, L2, R1, R2.
    pub shoulders: [i16; 4],
    /// D-pad: up, down, left, right (`+0x10…`).
    pub dpad: [i16; 4],
    /// Start, select.
    pub start_select: [i16; 2],
    /// Square, triangle (+0x1E), cross, circle.
    pub face: [i16; 4],
    /// Left / right shock (stick press, `Pads + 0x24` 0xB7347C for L).
    pub shock: [i16; 2],
    /// `m_bChatIndicated` (+0x28).
    pub chat_indicated: i16,
    /// `m_bPedWalk` (+0x2A): the walk key, sent by samp's on-foot sync.
    pub ped_walk: i16,
}

/// Mouse state (`CPad::NewMouseControllerState` 0xB73418, temp state 0xB73404).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MouseState {
    /// Delta x (+0xC, 0xB73424).
    pub dx: f32,
    /// Delta y (+0x10, 0xB73428).
    pub dy: f32,
    /// Buttons: left, right, middle, wheel up, wheel down, x1, x2.
    pub buttons: [bool; 7],
}

/// A window input event, delivered to [`crate::hooks::GameHooks::window_event`] before the game sees it.
/// Key codes are Windows virtual-key codes on every backend (samp-ui's key tables use them).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WindowEvent {
    /// Key pressed (`vk`, with auto-repeat flag).
    KeyDown {
        /// Virtual-key code.
        vk: u16,
        /// Auto-repeat.
        repeat: bool,
    },
    /// Key released.
    KeyUp {
        /// Virtual-key code.
        vk: u16,
    },
    /// Character typed (code page 1252 byte or Unicode scalar, see `unicode`).
    Char {
        /// Character value.
        ch: u32,
        /// Whether `ch` is a Unicode scalar (native) or a CP1252 byte (Windows `WM_CHAR` under samp).
        unicode: bool,
    },
    /// Mouse moved to client coordinates.
    MouseMove {
        /// X.
        x: i32,
        /// Y.
        y: i32,
    },
    /// Mouse button (0 left, 1 right, 2 middle).
    MouseButton {
        /// Button.
        button: u8,
        /// Pressed.
        down: bool,
    },
    /// Wheel delta (120 per notch).
    Wheel(i16),
    /// Window focus gained / lost.
    Focus(bool),
}

/// Whether the client consumed an input event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventResult {
    /// The game also gets it.
    PassToGame,
    /// The game does not see it.
    Consumed,
}

/// Input access.
pub trait Input {
    /// Pad 0 new and old state.
    fn pad(&self) -> (PadState, PadState);
    /// Overwrites pad 0's new state (remote key injection goes through
    /// [`crate::hooks::PlayerContext`], not through this).
    fn set_pad(&mut self, new: PadState) -> GameResult<()>;
    /// Whether a game key is down (`CPad::NewKeyState` 0xB73190, by game key offset).
    fn game_key_down(&self, key_offset: u16) -> bool;
    /// Whether a physical key is held now (`GetAsyncKeyState` & 0x8000 under Windows).
    fn key_held(&self, vk: u16) -> bool;
    /// Mouse state of this frame.
    fn mouse(&self) -> MouseState;
    /// Clears mouse history (`CPad::ClearMouseHistory` 0x541BD0).
    fn clear_mouse_history(&mut self) -> GameResult<()>;
    /// Re-reads pads now (`CPad::UpdatePads` 0x541DD0), used when the input lock is released.
    fn update_pads(&mut self) -> GameResult<()>;
    /// Pad counter samp writes for its 30 ms tick (`CPad::Pads + 0x116` 0xB7356E).
    fn set_pad_counter(&mut self, v: u8) -> GameResult<()>;
    /// Shows / hides the OS cursor over the game window (samp-rs neutralises `ShowCursor` today).
    fn set_cursor_visible(&mut self, visible: bool) -> GameResult<()>;
    /// Window size in pixels (`RsGlobal.maximumWidth/Height` 0xC17044/0xC17048).
    fn screen_size(&self) -> (u32, u32);
    /// Platform window handle where one exists (`HWND` 0xC97C1C under Windows; 0 natively).
    fn native_window(&self) -> usize;
}
