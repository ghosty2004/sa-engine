//! The SA:MP user interface's drawing backend (`docs/samp/overlay.md` §10).
//!
//! samp-rs has one UI implementation: the logic in `samp-ui`, the state machine and the per-frame pass in
//! `samp-client` (`ui::core`, `ui::session`, `ui::render`). It draws through [`UiBackend`], which has two
//! implementations: samp-client's Direct3D 9 / D3DX / DXUT code on the original `gta_sa.exe`, and the ported
//! engine (`sa-samp`'s overlay) natively and in the browser. The trait sits at the level samp's own code
//! draws: D3DX sprites ([`UiSprite`]) and fonts ([`UiFont`]) with `DrawText` format flags ([`dt`]), the
//! DXUT dialog with its skin sprites, the nametag bar painter, projection and the textdraw `CFont` calls.
//!
//! Text is bytes in the Windows ANSI code page (1252), like samp's. Colours are D3D `0xAARRGGBB` unless a
//! name says otherwise. Changed at API v5 (replaces the engine-owned `SampUi` of v1..v4).

use crate::Rgba;

impl Rgba {
    /// From samp's wire format `0xRRGGBBAA`.
    pub const fn from_rgba_u32(v: u32) -> Self {
        Self {
            r: (v >> 24) as u8,
            g: (v >> 16) as u8,
            b: (v >> 8) as u8,
            a: v as u8,
        }
    }

    /// From D3D's `0xAARRGGBB`.
    pub const fn from_argb_u32(v: u32) -> Self {
        Self {
            a: (v >> 24) as u8,
            r: (v >> 16) as u8,
            g: (v >> 8) as u8,
            b: v as u8,
        }
    }

    /// To D3D's `0xAARRGGBB`.
    pub const fn to_argb_u32(self) -> u32 {
        (self.a as u32) << 24 | (self.r as u32) << 16 | (self.g as u32) << 8 | self.b as u32
    }
}

/// D3DX `DrawText` format flags.
pub mod dt {
    /// `DT_CENTER`.
    pub const CENTER: u32 = 0x1;
    /// `DT_RIGHT`.
    pub const RIGHT: u32 = 0x2;
    /// `DT_VCENTER`.
    pub const VCENTER: u32 = 0x4;
    /// `DT_BOTTOM`.
    pub const BOTTOM: u32 = 0x8;
    /// `DT_SINGLELINE`.
    pub const SINGLELINE: u32 = 0x20;
    /// `DT_EXPANDTABS`.
    pub const EXPANDTABS: u32 = 0x40;
    /// `DT_NOCLIP`.
    pub const NOCLIP: u32 = 0x100;
    /// `DT_CALCRECT`: measure only, the rectangle is updated.
    pub const CALCRECT: u32 = 0x400;
}

/// A rectangle in pixels (`right` / `bottom` exclusive, D3D `RECT`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiRect {
    /// Left.
    pub left: i32,
    /// Top.
    pub top: i32,
    /// Right.
    pub right: i32,
    /// Bottom.
    pub bottom: i32,
}

impl UiRect {
    /// From the four edges.
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { left, top, right, bottom }
    }
}

/// samp's D3DX sprites: each pass draws inside one sprite's `Begin` / `End`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UiSprite {
    /// The font renderer's sprite (dialog body, scoreboard texts, F5, audio title).
    Text,
    /// The chat window's direct-draw sprite.
    Chat,
    /// The chat window's render-target blit sprite.
    Blit,
    /// The GUI resource manager's sprite (DXUT dialogs; [`UiBackend::dialog_begin`]).
    Gui,
    /// The death window's sprite.
    Death,
    /// The nametag painter's sprite.
    Tags,
    /// The 3D text painter's sprite (labels, bubbles).
    Text3d,
}

/// samp's D3DX fonts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UiFont {
    /// Font renderer main font (chat lines, F5).
    Main,
    /// The main font's outline twin (draws the black outline passes).
    MainOutline,
    /// Font renderer small font (main − 2): nametags, labels, dialog body, scoreboard texts.
    Small,
    /// The small font's outline twin.
    SmallOutline,
    /// GUI font 0: buttons, edit boxes.
    Gui0,
    /// GUI font 1 (GUI − 2): captions, list rows.
    Gui1,
    /// Death-window names (aux height).
    DeathNames,
    /// Death-window GTAWEAPON3 icon font (aux + 8).
    DeathIcon,
    /// Death-window GTAWEAPON3 backdrop font (aux + 12).
    DeathBackdrop,
    /// SAMPAUX3 20 (AFK icon).
    Aux20,
    /// SAMPAUX3 22 (AFK backdrop).
    Aux22,
}

/// Metrics samp measures once when it creates its fonts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiMetrics {
    /// Height of "Y" in the main font.
    pub main_h: i32,
    /// Height of "Y" in the small font.
    pub small_h: i32,
    /// GUI font 0 cell height.
    pub gui0_h: i32,
    /// GUI font 1 cell height.
    pub gui1_h: i32,
    /// Death window: width of `LONGESTNICKNICK_NICKNICK` in the names font.
    pub nick_w: i32,
    /// Death window: GTAWEAPON3 `G` glyph width.
    pub icon_w: i32,
    /// Death window: GTAWEAPON3 `G` glyph height.
    pub icon_h: i32,
    /// Width of the chat timestamp sample `[19:58:34]` in the main font.
    pub timestamp_w: i32,
}

/// A pre-transformed colour quad of the nametag bar painter (screen pixels).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ColourQuad {
    /// Left.
    pub x0: f32,
    /// Top.
    pub y0: f32,
    /// Right.
    pub x1: f32,
    /// Bottom.
    pub y1: f32,
    /// `0xAARRGGBB`.
    pub colour: u32,
}

/// The `CFont` state of one text textdraw (samp's setter order: scale, colour, justify 0, orientation, wrap,
/// centre size, background, background colour, proportional, drop colour, edge or shadow, font style; then
/// `PrintString` at `(x, y)` and `SetEdge(0)`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextdrawFont {
    /// `SetScale`.
    pub scale: (f32, f32),
    /// `CRGBA` as stored (R, G, B, A in memory order).
    pub colour: u32,
    /// 0 = centre, 1 = left, 2 = right.
    pub orientation: u8,
    /// `SetWrapx`.
    pub wrap_x: f32,
    /// `SetCentreSize`.
    pub centre_size: f32,
    /// `SetBackground`.
    pub background: bool,
    /// `SetBackgroundColor` (`CRGBA`).
    pub background_colour: u32,
    /// `SetProportional`.
    pub proportional: bool,
    /// `SetDropColor` (`CRGBA`).
    pub drop_colour: u32,
    /// `SetEdge` when non-zero, else `SetDropShadowPosition(shadow)`.
    pub outline: u8,
    /// Shadow offset.
    pub shadow: u8,
    /// `SetFontStyle`.
    pub font: u8,
    /// `PrintString` position.
    pub x: f32,
    /// `PrintString` position.
    pub y: f32,
}

/// Where samp's UI draws. Methods are called on the game thread from the client's frame callbacks; the
/// Present pass may only draw in [`crate::hooks::FramePhase::Present`], textdraws in `After2d`.
pub trait UiBackend {
    /// The video mode (`RsGlobal.maximumWidth` / `maximumHeight`): what the font sizes and the scoreboard scale
    /// with.
    fn video_size(&self) -> (i32, i32);

    /// The game window's client area (dialogs and the class-selection panel are placed in it).
    fn client_size(&self) -> (i32, i32);

    /// The mouse position in client coordinates.
    fn cursor_pos(&self) -> (i32, i32);

    /// samp's font metrics for the current fonts.
    fn metrics(&self) -> UiMetrics;

    /// `ID3DXFont::DrawTextA` without a sprite and with `flags` holding [`dt::CALCRECT`] (samp's measures): the
    /// extent of `count` bytes (−1 = all) of `text` laid out from `rect`. `colour` is passed through as samp
    /// passes it (it draws nothing).
    fn measure(&self, font: UiFont, text: &[u8], count: i32, rect: UiRect, flags: u32, colour: u32) -> UiRect;

    /// `ID3DXSprite::Begin(flags)` of one of samp's sprites.
    fn sprite_begin(&mut self, sprite: UiSprite, flags: u32);

    /// `ID3DXSprite::End`.
    fn sprite_end(&mut self, sprite: UiSprite);

    /// `ID3DXFont::DrawTextA` of `count` bytes (−1 = up to NUL / the end) on `sprite` (`None` = the font's own
    /// sprite). Returns the height.
    #[allow(clippy::too_many_arguments)]
    fn draw_text(
        &mut self,
        font: UiFont,
        sprite: Option<UiSprite>,
        text: &[u8],
        count: i32,
        rect: &mut UiRect,
        flags: u32,
        colour: u32,
    ) -> i32;

    /// samp's colour-code font wrapper: `text` (codes already stripped) drawn with one colour per visible
    /// glyph (`glyphs`, in order; blanks have none).
    #[allow(clippy::too_many_arguments)]
    fn draw_text_coloured(
        &mut self,
        font: UiFont,
        sprite: UiSprite,
        text: &[u8],
        rect: &mut UiRect,
        flags: u32,
        colour: u32,
        glyphs: &[u32],
    );

    /// The font renderer's outlined draw: black passes of the stripped text at `outline` (the 4-way passes move
    /// only left / top, the 8-way chat passes the whole rectangle) with `outline_font`, then the coloured pass
    /// with `font`. The default does exactly that; a backend with pre-outlined glyphs may draw it at once.
    #[allow(clippy::too_many_arguments)]
    fn draw_text_outlined(
        &mut self,
        sprite: UiSprite,
        font: UiFont,
        outline_font: UiFont,
        stripped: &[u8],
        rect: UiRect,
        flags: u32,
        colour: u32,
        glyphs: &[u32],
        outline: &[(i32, i32)],
    ) {
        let whole = outline.len() > 4;
        for &(dx, dy) in outline {
            let (rx, ry) = if whole { (dx, dy) } else { (0, 0) };
            let mut r = UiRect::new(rect.left + dx, rect.top + dy, rect.right + rx, rect.bottom + ry);
            self.draw_text(outline_font, Some(sprite), stripped, stripped.len() as i32, &mut r, flags | dt::NOCLIP, 0xFF00_0000);
        }
        let mut r = rect;
        self.draw_text_coloured(font, sprite, stripped, &mut r, flags | dt::NOCLIP, colour, glyphs);
    }

    /// Start of `CDXUTDialog::OnRender` for a dialog at `rect` with a uniform background (the GUI sprite
    /// begins; the skin texture is bound).
    fn dialog_begin(&mut self, rect: UiRect, background: u32);

    /// End of the dialog render.
    fn dialog_end(&mut self);

    /// `CDXUTDialog::DrawSprite`: the skin (`sampgui.png`) rectangle `src` scaled into `dst`, tinted (alpha 0
    /// draws nothing).
    fn gui_sprite(&mut self, src: UiRect, dst: UiRect, colour: u32);

    /// `CDXUTDialog::DrawText` (through the colour-code wrapper) in the GUI sprite.
    fn gui_text(&mut self, font: UiFont, text: &[u8], rect: UiRect, format: u32, colour: u32);

    /// `CDXUTDialog::DrawText` of UTF-16 text (edit boxes), `count` characters (−1 = all).
    fn gui_text_wide(&mut self, font: UiFont, text: &[u16], count: i32, rect: UiRect, format: u32, colour: u32);

    /// `CDXUTDialog::DrawRect`: a solid rectangle (caret, selection).
    fn gui_rect(&mut self, rect: UiRect, colour: u32);

    /// Width of UTF-16 text as DXUT's edit box measures it (GDI).
    fn text_width_wide(&self, font: UiFont, text: &[u16]) -> i32;

    /// ANSI → UTF-16 (`MultiByteToWideChar(CP_ACP)`).
    fn to_wide(&self, text: &[u8]) -> Vec<u16>;

    /// The nametag bar painter begins (state block, sprite, the fixed states).
    fn bars_begin(&mut self);

    /// One bar: border, background, fill.
    fn bars_group(&mut self, quads: &[ColourQuad; 3]);

    /// The bar painter ends.
    fn bars_end(&mut self);

    /// World → screen `(x, y, z)` with the game's camera (z > 1: behind the camera).
    fn project(&self, p: [f32; 3]) -> [f32; 3];

    /// The SAMPAUX3 fonts (AFK icon): created when `create` and missing; whether they exist.
    fn aux_fonts(&mut self, create: bool) -> bool;

    /// The chat window's render-target cache: draws the cached chat and returns true, or returns false and the
    /// caller draws the lines directly.
    fn chat_blit(&mut self) -> bool;

    /// The game's textdraw unit constants (1/640, 1/448).
    fn textdraw_units(&self) -> (f32, f32);

    /// The `CFont` calls of one text textdraw (`text` NUL-free; `has_keys`: run the key-name insertion).
    fn textdraw_text(&mut self, font: &TextdrawFont, text: &[u8], has_keys: bool);

    /// A sprite (font 4, `txd:name`) or model preview (font 5) textdraw `id`: loads its texture on first use and
    /// draws `rect` (`[left, top, right, bottom]`) tinted with `colour` (`CRGBA`). Returns false when there is no
    /// texture (a preview without a render falls back to the text path).
    fn textdraw_sprite(&mut self, id: u16, style: u8, text: &[u8], serial: u64, rect: [f32; 4], colour: u32) -> bool;

    /// Drops textdraw textures whose textdraw is gone or changed (`live`: id and text of the sprite textdraws).
    fn textdraw_sync(&mut self, live: &[(u16, &[u8])]);


    /// samp's input lock and cursor for the host (native backends route their own input): `keyboard` = the chat
    /// input or a dialog has the keys, `mouse` = the UI owns the mouse, `cursor` = the UI's cursor shows.
    fn set_input_state(&mut self, keyboard: bool, mouse: bool, cursor: bool) {
        let _ = (keyboard, mouse, cursor);
    }
}
