//! The player's controls, the HUD, on-screen messages, the player's model and the game's option switches.

use sa_core::mem::GPtr;
use sa_types::input::PadState;
use sa_types::script::ScriptArg::Int as I;
use sa_types::{GameError, GameOption, GameResult, ModelId, PedRef};

use crate::addr;
use crate::script::op;
use crate::{Game, View};

/// The longest big message, in bytes (the game keeps a pointer to the text, so it lives in [`Game`]).
pub const BIG_MESSAGE_MAX: usize = 0x1FF;

impl<V: View> Game<V> {
    /// The local player's pad: the new and the old `CControllerState`.
    pub fn pad(&self) -> (PadState, PadState) {
        (self.read_pad(addr::PAD0), self.read_pad(addr::PAD0 + 0x30))
    }

    /// One `CControllerState` (24 i16 fields, 0x30 bytes): sticks, shoulders, d-pad, start/select, face buttons,
    /// shocks, then `m_bChatIndicated` (+0x28), `m_bPedWalk` (+0x2A).
    fn read_pad(&self, a: u32) -> PadState {
        let s = |i: u32| self.view.read_i16(a + 2 * i);
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

    /// Whether the pause menu is up (`FrontEndMenuManager.m_bMenuActive`).
    pub fn menu_active(&self) -> bool {
        self.view.read_u8(addr::MENU_ACTIVE) != 0
    }

    /// Shows or hides the HUD and the radar: the menu's "HUD on" flag, then the script radar flag.
    pub fn set_hud_visible(&mut self, on: bool) {
        self.view.write(addr::MENU_HUD_ON, &[u8::from(on)]);
        self.view.write(addr::HUD_DONT_DISPLAY_RADAR, &[u8::from(!on)]);
    }

    /// `CMessages::AddBigMessage` of `text` (one byte per character: Latin-1, anything else `?`) for `time_ms` in
    /// style `style` (0..=6).
    pub fn big_message(&mut self, text: &str, time_ms: u32, style: u16) -> GameResult<()> {
        if style > 6 {
            return Err(GameError::Other(format!("big message style {style}")));
        }
        let bytes: Vec<u8> = text.chars().map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?')).collect();
        let n = bytes.len().min(BIG_MESSAGE_MAX);
        self.big_message.fill(0);
        self.big_message[..n].copy_from_slice(&bytes[..n]);
        sa_core::bindings::add_big_message(GPtr::from_raw(self.big_message.as_mut_ptr()), time_ms, u32::from(style));
        Ok(())
    }

    /// Clears the messages on screen (`CLEAR_PRINTS`).
    pub fn clear_prints(&mut self) -> GameResult<()> {
        self.run_op(op::CLEAR_PRINTS, &[]).map(drop)
    }

    /// Gives the player ped `ped` the ped model `model` (loaded first), through `SET_PLAYER_MODEL`
    /// (`DeleteRwObject` + `SetModelIndex`). Only player peds change models this way.
    pub fn set_ped_model(&mut self, ped: PedRef, model: ModelId) -> GameResult<()> {
        if !self.is_ped_model(model) {
            return Err(GameError::Other(format!("{} is not a ped model", model.0)));
        }
        let slot = self.player_slot_of(ped).ok_or(GameError::Unsupported("set_ped_model of a non-player ped"))?;
        if !self.model_loaded(model) {
            self.load_model_now(model)?;
        }
        self.run_op(op::SET_PLAYER_MODEL, &[I(i32::from(slot)), I(model.0)]).map(drop)
    }

    /// Turns one of the game's own features on or off.
    pub fn set_option(&mut self, option: GameOption, on: bool) {
        use sa_core::options::{Feature as F, set};
        let feature = match option {
            GameOption::TypedCheats => F::TypedCheats,
            GameOption::Clock => F::Clock,
            GameOption::AmbientPeds => F::AmbientPeds,
            GameOption::RandomTraffic => F::RandomTraffic,
            GameOption::CarGenerators => F::CarGenerators,
            GameOption::AmbientPlanes => F::AmbientPlanes,
            GameOption::Cutscenes => F::Cutscenes,
            GameOption::Replay => F::Replay,
            GameOption::Plants => F::Plants,
            GameOption::SunAndMoon => F::SunAndMoon,
            GameOption::IdleCamera => F::IdleCamera,
            GameOption::HelpMessages => F::HelpMessages,
            GameOption::ZoneNames => F::ZoneNames,
        };
        set(feature, on);
    }
}
