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
            GameOption::StartScreens => F::StartScreens,
            GameOption::VehicleNameOnHud => F::VehicleNameOnHud,
            GameOption::VitalStatsOnHud => F::VitalStatsOnHud,
            GameOption::SinglePlayerPickups => F::SinglePlayerPickups,
            GameOption::AmbientGunfire => F::AmbientGunfire,
            GameOption::RealTimeShadows => F::RealTimeShadows,
            GameOption::PauseMenuOnFocus => F::PauseMenuOnFocus,
            GameOption::DeathModelCleanup => F::DeathModelCleanup,
            GameOption::SpeedBlur => F::SpeedBlur,
            GameOption::WastedMessage => F::WastedMessage,
            GameOption::VehicleFreebies => F::VehicleFreebies,
            GameOption::PlayerIdleAnims => F::PlayerIdleAnims,
            GameOption::GunAimedAtResponse => F::GunAimedAtResponse,
            GameOption::FireFailSounds => F::FireFailSounds,
            GameOption::StatChanges => F::StatChanges,
            GameOption::HandbrakeNeedsControl => F::HandbrakeNeedsControl,
        };
        set(feature, on);
    }
}

/// Prints `text` (one byte per character, the game font's code page; up to the first NUL) in the game's font with
/// `t`'s `CFont` state, into `CFont`'s buffer, which the game's frame draws with the HUD's text. Only valid while
/// the game draws its 2D phase (after `Render2dStuff`, before the frame's `DrawFonts`).
pub fn print_text(t: &sa_types::text::FontText, text: &[u8]) {
    use sa_core::bindings::*;
    // CFont takes CRGBA by value: R, G, B, A in memory order
    let crgba = |c: u32| c.swap_bytes();
    font_set_scale(t.scale.0.to_bits(), t.scale.1.to_bits());
    font_set_color(crgba(t.colour));
    font_set_justify(0);
    font_set_orientation(u32::from(t.orientation));
    font_set_wrapx(t.wrap_x);
    font_set_centre_size(t.centre_size);
    font_set_background(u32::from(t.background), 0);
    font_set_background_color(crgba(t.background_colour));
    font_set_proportional(u32::from(t.proportional));
    font_set_drop_color(crgba(t.drop_colour));
    if t.edge != 0 {
        font_set_edge(u32::from(t.edge));
    } else {
        font_set_drop_shadow_position(u32::from(t.shadow));
    }
    font_set_font_style(u32::from(t.style));
    let mut buf: Vec<u8> = text.iter().copied().take_while(|&b| b != 0).collect();
    buf.push(0);
    font_print_string(t.x, t.y, GPtr::from_raw(buf.as_mut_ptr()));
    font_set_edge(0);
}

/// `CSprite::CalcScreenCoors` of world point `p` with both visibility checks: the screen position in pixels and the
/// depth in front of the camera, `None` off screen or behind the camera.
pub fn world_to_screen(p: sa_types::Vec3) -> Option<(f32, f32, f32)> {
    let mut inp = [p.x, p.y, p.z];
    let mut out = [0.0f32; 3];
    let (mut w, mut h) = (0.0f32, 0.0f32);
    let ok = sa_core::bindings::calc_screen_coors(
        GPtr::from_raw(inp.as_mut_ptr()),
        GPtr::from_raw(out.as_mut_ptr()),
        GPtr::from_raw(&mut w),
        GPtr::from_raw(&mut h),
        1,
        1,
    );
    (ok != 0).then_some((out[0], out[1], out[2]))
}

/// A NUL-terminated copy of `s`.
pub(crate) fn c_str(s: &[u8]) -> Vec<u8> {
    let mut v: Vec<u8> = s.iter().copied().take_while(|&b| b != 0).collect();
    v.push(0);
    v
}

/// The slot of the game's texture dictionary `name`.
pub fn txd_slot(name: &[u8]) -> Option<i32> {
    let mut n = c_str(name);
    Some(sa_core::bindings::find_txd_slot(GPtr::from_raw(n.as_mut_ptr()))).filter(|&s| s != -1)
}

/// Loads the texture dictionary file `path` (a game path) into a new slot `name` and keeps it loaded (a reference
/// the game never drops), as the game loads its own loose dictionaries (`CGame::Init1`'s "particle"). The slot, or
/// `None` when the file cannot be opened.
pub fn load_txd_slot(name: &[u8], path: &[u8]) -> Option<i32> {
    use sa_core::bindings::*;
    let mut slot_name = c_str(name);
    let mut path = c_str(path);
    // `LoadTxd` retries forever on a missing file, as the original does: look first
    let mut mode = c_str(b"rb");
    let f = open_file(GPtr::from_raw(path.as_mut_ptr()), GPtr::from_raw(mode.as_mut_ptr()));
    if f.is_null() {
        return None;
    }
    file_close(f);
    let slot = add_txd_slot(GPtr::from_raw(slot_name.as_mut_ptr()));
    load_txd(slot, GPtr::from_raw(path.as_mut_ptr()));
    txd_add_ref(slot);
    Some(slot)
}

/// The texture `name` of the game's texture dictionary `dict` (an `RwTexture*`, as `CSprite2d::SetTexture` finds it
/// with the dictionary current). Without a dictionary of that name, `file` (a path in the game folder) is loaded into
/// a new one, which stays loaded. `None` when the file or the texture is not there.
pub fn load_texture(dict: &[u8], file: Option<&[u8]>, name: &[u8]) -> Option<u32> {
    use sa_core::bindings::*;
    let slot = match txd_slot(dict) {
        Some(s) => s,
        None => load_txd_slot(dict, file?)?,
    };
    let mut sprite: u32 = 0;
    let mut tex = c_str(name);
    push_current_txd();
    set_current_txd(slot);
    sprite2d_set_texture(GPtr::from_raw(&mut sprite).cast(), GPtr::from_raw(tex.as_mut_ptr()));
    pop_current_txd();
    (sprite != 0).then_some(sprite)
}

/// `CSprite2d::Draw` of `texture` ([`load_texture`]) into `rect` (`[left, top, right, bottom]`, pixels) tinted with
/// `rgba` (`0xRRGGBBAA`), bilinear. Only valid while the game draws its 2D phase.
pub fn draw_sprite(texture: u32, rect: [f32; 4], rgba: u32) {
    use sa_core::bindings::*;
    sa_core::input::rw_render_state_set(RW_TEXTURE_FILTER, RW_FILTER_LINEAR);
    let mut sprite = texture;
    // `CRect` as the game passes it: left, bottom, right, top
    let mut r = [rect[0], rect[3], rect[2], rect[1]];
    let mut c = rgba.swap_bytes();
    sprite2d_draw(GPtr::from_raw(&mut sprite).cast(), GPtr::from_raw(r.as_mut_ptr()), GPtr::from_raw(&mut c).cast());
}

/// `rwRENDERSTATETEXTUREFILTER`, `rwFILTERLINEAR`.
const RW_TEXTURE_FILTER: i32 = 9;
const RW_FILTER_LINEAR: u32 = 2;
