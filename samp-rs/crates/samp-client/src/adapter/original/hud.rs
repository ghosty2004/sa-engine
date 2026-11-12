//! HUD, 2D text and GTA menus on the original executable: the original backend's `sa_samp_api::hud` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not routed through
//! the adapter yet; the client still does that work in its own modules.

use sa_samp_api::hud::*;
use sa_samp_api::*;

#[cfg(windows)]
use std::cell::UnsafeCell;

use sa_samp_api::script::ScriptArg;

use super::{OriginalGame, write_error};
use crate::game;
use crate::patch::Memory;
use crate::script::op;

/// `CMessages::AddBigMessage(text, time, style)` (cdecl).
#[cfg(windows)]
const ADD_BIG_MESSAGE: u32 = 0x69F2B0;
/// `FrontEndMenuManager.m_bMenuActive` (byte; samp-rs also reads the dword there).
const MENU_ACTIVE: u32 = 0xBA67A4;

/// The text buffer GTA's big-message queue points into (it keeps the pointer, not a copy).
#[cfg(windows)]
struct BigMessageText(UnsafeCell<[u8; 0x200]>);
// SAFETY: only touched on the game's main thread.
#[cfg(windows)]
unsafe impl Sync for BigMessageText {}
#[cfg(windows)]
static BIG_MESSAGE: BigMessageText = BigMessageText(UnsafeCell::new([0; 0x200]));

#[cfg(windows)]
impl BigMessageText {
    /// Copies `text` (NUL-terminated, at most 0x1FF bytes) and returns the buffer address.
    fn set(&self, text: &[u8]) -> u32 {
        // SAFETY: main thread only; GTA reads the buffer between our calls.
        let b = unsafe { &mut *self.0.get() };
        b.fill(0);
        let n = text.len().min(b.len() - 1);
        b[..n].copy_from_slice(&text[..n]);
        b.as_ptr() as usize as u32
    }
}

impl<M: Memory> Hud for OriginalGame<M> {
    /// The menu "HUD on" flag, then the script radar flag. Mirrors samp.dll 0x100A1DF0.
    fn set_hud_visible(&mut self, on: bool) -> GameResult<()> {
        self.mem.write_data(game::MENU_HUD_ON, &[u8::from(on)]).map_err(write_error)?;
        self.mem.write_data(game::HUD_SCRIPT_DONT_DISPLAY_RADAR, &[u8::from(!on)]).map_err(write_error)
    }

    fn menu_active(&self) -> bool {
        self.mem.read_u8(MENU_ACTIVE).unwrap_or(0) != 0
    }

    fn menu_active_dword(&self) -> u32 {
        self.mem.read_u32(MENU_ACTIVE).unwrap_or(0)
    }

    #[cfg(windows)]
    fn big_message(&mut self, text: &str, time_ms: u32, style: u16) -> GameResult<()> {
        let p = BIG_MESSAGE.set(text.as_bytes());
        // SAFETY: GTA's cdecl message queue entry, main thread, text in a static buffer.
        unsafe {
            let f: extern "C" fn(u32, u32, u32) = std::mem::transmute(ADD_BIG_MESSAGE as usize);
            f(p, time_ms, u32::from(style));
        }
        Ok(())
    }

    #[cfg(not(windows))]
    fn big_message(&mut self, _text: &str, _time_ms: u32, _style: u16) -> GameResult<()> {
        Err(GameError::Unsupported("Hud::big_message outside the game"))
    }

    /// `CLEAR_PRINTS` (00BE).
    fn clear_prints(&mut self) -> GameResult<()> {
        self.run_op(op::CLEAR_PRINTS, &[]).map(|_| ())
    }

    /// `DISPLAY_ZONE_NAMES` (09BA).
    fn set_zone_names(&mut self, on: bool) -> GameResult<()> {
        self.run_op(op::DISPLAY_ZONE_NAMES, &[ScriptArg::Int(i32::from(on))]).map(|_| ())
    }

    /// `ADD_SCORE(0, delta)` (0109).
    fn add_score(&mut self, delta: i32) -> GameResult<()> {
        self.run_op(op::ADD_SCORE, &[ScriptArg::Int(0), ScriptArg::Int(delta)]).map(|_| ())
    }

    fn expand_game_text(&self, _text: &str, _numbers: &[i32]) -> String {
        String::new()
    }

    fn gxt(&self, _key: &str) -> Option<String> {
        None
    }
}

impl<M: Memory> Text2d for OriginalGame<M> {
    fn print(&mut self, _style: &FontStyle, _at: Point2, _text: &str) -> GameResult<()> {
        Err(GameError::Unsupported("Text2d::print"))
    }

    fn text_width(&self, _style: &FontStyle, _text: &str) -> f32 {
        0.0
    }

    fn load_txd(&mut self, _name: &str, _path: Option<&str>) -> GameResult<TxdSlot> {
        Err(GameError::Unsupported("Text2d::load_txd"))
    }

    fn sprite(&mut self, _txd: TxdSlot, _texture: &str) -> GameResult<TextureRef> {
        Err(GameError::Unsupported("Text2d::sprite"))
    }

    fn draw_sprite(&mut self, _sprite: TextureRef, _rect: Rect2, _colour: Rgba) -> GameResult<()> {
        Err(GameError::Unsupported("Text2d::draw_sprite"))
    }

    fn release_sprite(&mut self, _sprite: TextureRef) -> GameResult<()> {
        Err(GameError::Unsupported("Text2d::release_sprite"))
    }
}

impl<M: Memory> Menus for OriginalGame<M> {
    fn show_menu(&mut self, _menu: &MenuSpec) -> GameResult<MenuRef> {
        Err(GameError::Unsupported("Menus::show_menu"))
    }

    fn menu_selection(&self, _menu: MenuRef) -> GameResult<i32> {
        Err(GameError::Unsupported("Menus::menu_selection"))
    }

    fn delete_menu(&mut self, _menu: MenuRef) -> GameResult<()> {
        Err(GameError::Unsupported("Menus::delete_menu"))
    }
}
