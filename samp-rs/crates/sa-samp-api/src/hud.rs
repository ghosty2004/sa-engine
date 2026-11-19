//! HUD, game text, 2D text and sprites drawn with the game's own `CFont`/`CSprite2d`, and GTA menus.
//! samp-rs today: `ui/textdraws.rs`, `ui/menus.rs`, `ui/glue/*.rs`, `behaviourclient.rs`, `game.rs`.
//! Mapping: `docs/samp/game-adapter.md` §4.6.

use crate::{GameError, GameResult, MenuRef, Point2, Rect2, Rgba, TextureRef, TxdSlot};

/// The complete `CFont` state for one print call. Replaces the 14 setter calls samp-rs makes in samp's
/// order (`CFont::SetScale` 0x719380 … `SetOrientation` 0x719610); the backend issues them in that order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontStyle {
    /// Letter size (x, y).
    pub scale: Point2,
    /// Text colour.
    pub colour: Rgba,
    /// Font (0..=3).
    pub font: u8,
    /// Wrap x.
    pub wrap_x: f32,
    /// Centre size (wrap width when centred).
    pub centre_size: f32,
    /// Shadow / outline colour.
    pub drop_colour: Rgba,
    /// Shadow offset (0 = none).
    pub shadow: i8,
    /// Outline size (0 = none).
    pub edge: i8,
    /// Proportional spacing.
    pub proportional: bool,
    /// Box background.
    pub background: bool,
    /// Box colour.
    pub background_colour: Rgba,
    /// Alignment.
    pub align: TextAlign,
}

/// Text alignment (`SetJustify` / `SetOrientation`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    /// Left.
    Left,
    /// Centre.
    Centre,
    /// Right.
    Right,
}

/// HUD switches and game messages.
pub trait Hud {
    /// HUD and radar on/off (`FrontEndMenuManager.m_bHudOn` 0xBA6769, `CHud::bScriptDontDisplayRadar`
    /// 0xBAA3FB).
    fn set_hud_visible(&mut self, on: bool) -> GameResult<()>;
    /// Whether the pause menu is open: the **byte** `FrontEndMenuManager.m_bMenuActive` 0xBA67A4 is non-zero;
    /// samp draws no UI then. Call sites that read the dword at 0xBA67A4 (it also covers the next three
    /// bytes) are not equivalent; use [`Hud::menu_active_dword`] for them.
    fn menu_active(&self) -> bool;
    /// The dword at 0xBA67A4 (`m_bMenuActive` and the three bytes after it), for samp-rs sites that test
    /// it as a `u32`. Native backends return `menu_active() as u32` plus the same following fields.
    fn menu_active_dword(&self) -> u32;
    /// GameText / big message (`CMessages::AddBigMessage` 0x69F2B0).
    fn big_message(&mut self, text: &str, time_ms: u32, style: u16) -> GameResult<()>;
    /// `CLEAR_PRINTS` (00BE).
    fn clear_prints(&mut self) -> GameResult<()>;
    /// `DISPLAY_ZONE_NAMES` (09BA).
    fn set_zone_names(&mut self, on: bool) -> GameResult<()>;
    /// `ADD_SCORE` (0109), used for money deltas.
    fn add_score(&mut self, delta: i32) -> GameResult<()>;
    /// Expands `~k~~KEY~` tokens and numbers like GTA (`CMessages::InsertPlayerControlKeysInString`
    /// 0x69E160, `InsertNumberInString` 0x69DE90).
    fn expand_game_text(&self, text: &str, numbers: &[i32]) -> String;
    /// GXT text lookup through the game (`CText::Get` 0x6A0050), for samp's menus.
    fn gxt(&self, key: &str) -> Option<String>;
}

/// 2D drawing with the game's font and sprites. Only valid in [`crate::hooks::FramePhase::After2d`].
pub trait Text2d {
    /// Prints a string (`CFont::PrintString` 0x71A700) in HUD coordinates.
    fn print(&mut self, style: &FontStyle, at: Point2, text: &str) -> GameResult<()>;
    /// Width the game's font gives a string (for textdraw boxes).
    fn text_width(&self, style: &FontStyle, text: &str) -> f32;
    /// Finds or adds a TXD slot and loads it (`CTxdStore::FindTxdSlot` 0x731850, `AddTxdSlot` 0x731C80,
    /// `LoadTxd` 0x7320B0, `AddRef` 0x731A00).
    fn load_txd(&mut self, name: &str, path: Option<&str>) -> GameResult<TxdSlot>;
    /// Gets a texture from a slot (`PushCurrentTxd`/`SetCurrentTxd`/`CSprite2d::SetTexture`/`PopCurrentTxd`).
    fn sprite(&mut self, txd: TxdSlot, texture: &str) -> GameResult<TextureRef>;
    /// Draws a sprite (`CSprite2d::Draw` 0x728350).
    fn draw_sprite(&mut self, sprite: TextureRef, rect: Rect2, colour: Rgba) -> GameResult<()>;
    /// Releases a sprite (`CSprite2d::~CSprite2d` 0x7281E0).
    fn release_sprite(&mut self, sprite: TextureRef) -> GameResult<()>;
}

/// A GTA script menu to create (`CREATE_MENU` 08D4). Its title, headers and rows are text keys the game looks up
/// (`CText::Get` 0x6A0050), registered with [`Menus::set_text_entry`]; `DUMMY` is no text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuSpec<'a> {
    /// The title's key.
    pub title: &'a str,
    /// Top left, in the game's 640 x 448 menu units.
    pub pos: Point2,
    /// Column width, in menu units.
    pub width: f32,
    /// Columns (1 ..= 4).
    pub columns: u8,
    /// Rows can be selected.
    pub interactive: bool,
    /// A background box.
    pub background: bool,
    /// Text alignment (`eFontAlignment`: 0 centre, 1 left, 2 right).
    pub alignment: u8,
}

/// GTA's script menus (`CMenuSystem`, two at a time) and the text entries they show.
pub trait Menus {
    /// Registers the text of key `key` (code page 1252 bytes): the game's lookups of the key find it before its own
    /// texts. samp.dll serves its `SAMP…` keys from its `CText::Get` hook (0x6A0043 slot).
    fn set_text_entry(&mut self, _key: &str, _text: &[u8]) -> GameResult<()> {
        Err(GameError::Unsupported("Menus::set_text_entry"))
    }
    /// Removes a key of [`Menus::set_text_entry`].
    fn remove_text_entry(&mut self, _key: &str) -> GameResult<()> {
        Err(GameError::Unsupported("Menus::remove_text_entry"))
    }
    /// Creates a menu (`CREATE_MENU` 08D4); shown from the next frame.
    fn create_menu(&mut self, _menu: &MenuSpec<'_>) -> GameResult<MenuRef> {
        Err(GameError::Unsupported("Menus::create_menu"))
    }
    /// A column's header key and up to 12 row keys (`SET_MENU_COLUMN` 08DB).
    fn set_menu_column(&mut self, _menu: MenuRef, _column: u8, _header: &str, _rows: &[&str]) -> GameResult<()> {
        Err(GameError::Unsupported("Menus::set_menu_column"))
    }
    /// A column's width in menu units (`SET_MENU_COLUMN_WIDTH` 09DB).
    fn set_menu_column_width(&mut self, _menu: MenuRef, _column: u8, _width: u16) -> GameResult<()> {
        Err(GameError::Unsupported("Menus::set_menu_column_width"))
    }
    /// Whether a row can be selected (`ACTIVATE_MENU_ITEM` 08D9).
    fn set_menu_row_enabled(&mut self, _menu: MenuRef, _row: u8, _on: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Menus::set_menu_row_enabled"))
    }
    /// Selected row (`GET_MENU_ITEM_SELECTED` 08D7).
    fn menu_selection(&self, _menu: MenuRef) -> GameResult<i32> {
        Err(GameError::Unsupported("Menus::menu_selection"))
    }
    /// `DELETE_MENU` (08DA).
    fn delete_menu(&mut self, _menu: MenuRef) -> GameResult<()> {
        Err(GameError::Unsupported("Menus::delete_menu"))
    }
}
