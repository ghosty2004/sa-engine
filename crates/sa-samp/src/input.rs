//! Pads, keyboard, mouse and the game window on the native engine: the native backend's `sa_samp_api::input` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::input::*;
use sa_samp_api::*;

use crate::addr;
use crate::{NativeGame, View};

/// The `int16` of `CKeyboardState` (`CPad::NewKeyState`) that holds the key with Windows virtual-key code `vk`:
/// `FKeys[12]`, `standardKeys[256]` (indexed by the upper-case character), then the named keys from `esc`.
fn key_state_index(vk: u16) -> Option<u32> {
    const NAMED: u32 = 12 + 256;
    let vk32 = u32::from(vk);
    Some(match vk {
        0x70..=0x7B => vk32 - 0x70,
        0x20 | 0x30..=0x39 | 0x41..=0x5A => 12 + vk32,
        0x1B => NAMED,
        0x2D => NAMED + 1,
        0x2E => NAMED + 2,
        0x24 => NAMED + 3,
        0x23 => NAMED + 4,
        0x21 => NAMED + 5,
        0x22 => NAMED + 6,
        0x26 => NAMED + 7,
        0x28 => NAMED + 8,
        0x25 => NAMED + 9,
        0x27 => NAMED + 10,
        0x0D => NAMED + 18,
        0x61..=0x69 => NAMED + 20 + (vk32 - 0x61),
        0x60 => NAMED + 29,
        0x08 => NAMED + 30,
        0x09 => NAMED + 31,
        0x10 => NAMED + 36,
        0xA0 => NAMED + 34,
        0xA1 => NAMED + 35,
        0xA2 => NAMED + 37,
        0xA3 => NAMED + 38,
        0xA4 => NAMED + 39,
        0xA5 => NAMED + 40,
        _ => return None,
    })
}

impl<V: View> Input for NativeGame<V> {
    fn pad(&self) -> (PadState, PadState) {
        (
            read_pad(&self.view, addr::PAD0),
            read_pad(&self.view, addr::PAD0 + 0x30),
        )
    }

    fn set_pad(&mut self, _new: PadState) -> GameResult<()> {
        Err(GameError::Unsupported("Input::set_pad"))
    }

    fn game_key_down(&self, key_offset: u16) -> bool {
        self.view.read_u16(addr::NEW_KEY_STATE + u32::from(key_offset)) != 0
    }

    fn key_held(&self, vk: u16) -> bool {
        // the keyboard state the game itself reads this frame, not the OS's
        key_state_index(vk).is_some_and(|i| self.view.read_i16(addr::NEW_KEY_STATE + 2 * i) != 0)
    }

    fn mouse(&self) -> MouseState {
        let mut buttons = [false; 7];
        for (i, b) in buttons.iter_mut().enumerate() {
            *b = self.view.read_u8(addr::NEW_MOUSE + i as u32) != 0;
        }
        MouseState {
            dx: self.view.read_f32(addr::NEW_MOUSE + 0xC),
            dy: self.view.read_f32(addr::NEW_MOUSE + 0x10),
            buttons,
        }
    }

    fn clear_mouse_history(&mut self) -> GameResult<()> {
        Err(GameError::Unsupported("Input::clear_mouse_history"))
    }

    fn update_pads(&mut self) -> GameResult<()> {
        Err(GameError::Unsupported("Input::update_pads"))
    }

    fn set_pad_counter(&mut self, _v: u8) -> GameResult<()> {
        Err(GameError::Unsupported("Input::set_pad_counter"))
    }

    fn set_cursor_visible(&mut self, _visible: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Input::set_cursor_visible"))
    }

    fn screen_size(&self) -> (u32, u32) {
        (
            self.view.read_u32(addr::RS_MAX_WIDTH),
            self.view.read_u32(addr::RS_MAX_HEIGHT),
        )
    }

    fn native_window(&self) -> usize {
        0
    }
}

/// One `CControllerState` (24 i16 fields, 0x30 bytes): sticks, shoulders, d-pad, start/select, face buttons,
/// shocks, then `m_bChatIndicated` (+0x28), `m_bPedWalk` (+0x2A), `m_bVehicleMouseLook`, `m_bRadioTrackSkip`.
fn read_pad<V: View>(v: &V, a: u32) -> PadState {
    let s = |i: u32| v.read_i16(a + 2 * i);
    PadState {
        left_x: s(0),
        left_y: s(1),
        right_x: s(2),
        right_y: s(3),
        shoulders: [s(4), s(5), s(6), s(7)],
        dpad: [s(8), s(9), s(10), s(11)],
        start_select: [s(12), s(13)],
        face: [s(14), s(15), s(16), s(17)],
        shock: [s(18), s(19)],
        chat_indicated: s(20),
        ped_walk: s(21),
    }
}
