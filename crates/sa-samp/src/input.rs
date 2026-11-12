//! Pads, keyboard, mouse and the game window on the native engine: the native backend's `sa_samp_api::input` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::input::*;
use sa_samp_api::*;

use crate::addr;
use crate::{NativeGame, View};

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

    fn key_held(&self, _vk: u16) -> bool {
        false
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
/// The API's `home` ("home button / ped walk") is taken as `m_bPedWalk`, the field samp's on-foot sync uses.
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
        home: s(21),
    }
}
