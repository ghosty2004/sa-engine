//! Pads, keyboard, mouse and the game window on the original executable: the original backend's `sa_samp_api::input` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not routed through
//! the adapter yet; the client still does that work in its own modules.

use sa_samp_api::input::*;
use sa_samp_api::*;

use super::OriginalGame;
use crate::game;
use crate::patch::Memory;

impl<M: Memory> Input for OriginalGame<M> {
    fn pad(&self) -> (PadState, PadState) {
        (PadState::default(), PadState::default())
    }

    fn set_pad(&mut self, _new: PadState) -> GameResult<()> {
        Err(GameError::Unsupported("Input::set_pad"))
    }

    fn game_key_down(&self, key_offset: u16) -> bool {
        game::key_down(&self.mem, u32::from(key_offset))
    }

    fn key_held(&self, _vk: u16) -> bool {
        false
    }

    fn mouse(&self) -> MouseState {
        MouseState::default()
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
        (0, 0)
    }

    fn native_window(&self) -> usize {
        0
    }
}
