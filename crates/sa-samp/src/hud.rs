//! HUD, 2D text and GTA menus on the native engine: the native backend's `sa_samp_api::hud` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::hud::*;
use sa_samp_api::*;

use crate::addr;
use crate::{NativeGame, View};

impl<V: View> Hud for NativeGame<V> {
    fn set_hud_visible(&mut self, _on: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Hud::set_hud_visible"))
    }

    fn menu_active(&self) -> bool {
        self.view.read_u8(addr::MENU_ACTIVE) != 0
    }

    fn menu_active_dword(&self) -> u32 {
        self.view.read_u32(addr::MENU_ACTIVE)
    }

    fn big_message(&mut self, _text: &str, _time_ms: u32, _style: u16) -> GameResult<()> {
        Err(GameError::Unsupported("Hud::big_message"))
    }

    fn clear_prints(&mut self) -> GameResult<()> {
        Err(GameError::Unsupported("Hud::clear_prints"))
    }

    fn set_zone_names(&mut self, _on: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Hud::set_zone_names"))
    }

    fn add_score(&mut self, _delta: i32) -> GameResult<()> {
        Err(GameError::Unsupported("Hud::add_score"))
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
