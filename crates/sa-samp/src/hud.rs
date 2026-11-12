//! HUD, 2D text and GTA menus on the native engine: the native backend's `sa_samp_api::hud` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_core::mem::GPtr;
use sa_samp_api::hud::*;
use sa_samp_api::script::ScriptArg::Int as I;
use sa_samp_api::*;

use crate::addr;
use crate::script::op;

sa_core::orig! {
    /// `CMessages::AddBigMessage(text, time, style)` (0x69F2B0).
    fn add_big_message(text: GPtr<u8>, time: u32, style: u32) = cdecl 0x69F2B0 => unported;
}
use crate::{NativeGame, View};

impl<V: View> Hud for NativeGame<V> {
    fn set_hud_visible(&mut self, on: bool) -> GameResult<()> {
        // The menu "HUD on" flag, then the script radar flag (samp.dll 0x100A1DF0).
        self.view.write(addr::MENU_HUD_ON, &[u8::from(on)]);
        self.view.write(addr::HUD_DONT_DISPLAY_RADAR, &[u8::from(!on)]);
        Ok(())
    }

    fn menu_active(&self) -> bool {
        self.view.read_u8(addr::MENU_ACTIVE) != 0
    }

    fn menu_active_dword(&self) -> u32 {
        self.view.read_u32(addr::MENU_ACTIVE)
    }

    fn big_message(&mut self, text: &str, time_ms: u32, style: u16) -> GameResult<()> {
        // GTA keeps the pointer, so the text lives in the adapter until the next message.
        let n = text.len().min(self.big_message.len() - 1);
        self.big_message.fill(0);
        self.big_message[..n].copy_from_slice(&text.as_bytes()[..n]);
        add_big_message(
            GPtr::from_raw(self.big_message.as_mut_ptr()),
            time_ms,
            u32::from(style),
        );
        Ok(())
    }

    fn clear_prints(&mut self) -> GameResult<()> {
        self.run_op(op::CLEAR_PRINTS, &[]).map(drop)
    }

    fn set_zone_names(&mut self, on: bool) -> GameResult<()> {
        self.run_op(op::DISPLAY_ZONE_NAMES, &[I(i32::from(on))]).map(drop)
    }

    fn add_score(&mut self, delta: i32) -> GameResult<()> {
        self.run_op(op::ADD_SCORE, &[I(0), I(delta)]).map(drop)
    }

    fn expand_game_text(&self, _text: &str, _numbers: &[i32]) -> String {
        String::new()
    }

    fn gxt(&self, _key: &str) -> Option<String> {
        None
    }
}

impl<V: View> Text2d for NativeGame<V> {
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

impl<V: View> Menus for NativeGame<V> {
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
