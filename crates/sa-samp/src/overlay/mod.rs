//! The engine backend of samp-rs's UI (docs/samp/overlay.md §10): [`Overlay`] implements
//! `sa_samp_api::ui::UiBackend` on the ported engine, served to the client through `GameAdapter::ui`. The client's
//! pass (samp-client `ui::render`) calls it from its frame callback; the overlay collects the draws in a
//! [`draw::DrawList`] and [`NativeGame::overlay_frame`] draws them right after the callback through the ported
//! engine ([`render`]): `CSprite2d::DrawRect`, RW Im2D glyph and skin quads, `CFont`, then the cursor.

pub mod draw;
pub mod font;
pub mod render;
pub mod skin;

use sa_samp_api::hooks::FramePhase;
use sa_samp_api::input::{EventResult, WindowEvent};
use sa_samp_api::peds::Peds as _;
use sa_samp_api::ui::{ColourQuad, TextdrawFont, UiBackend, UiFont, UiMetrics, UiRect, UiSprite, dt};
use sa_samp_api::{PedRef, Vec3};

use self::draw::{Cmd, DrawList};
use self::font::{Faces, FontAtlas, MASK_BYTE, outline, slot};
use crate::addr::{self, off};
use crate::{NativeGame, View};

/// `gta_sa.exe`'s textdraw unit constants (1/640, 1/448).
const UNIT_X: usize = 0x859520;
const UNIT_Y: usize = 0x859524;

/// The engine backend's resources and the draws of the current pass.
#[derive(Default)]
pub struct Overlay {
    atlas: Option<FontAtlas>,
    /// The atlas raster (`RwRaster*`), 0 before the upload or when it failed.
    raster: u32,
    upload_failed: bool,
    /// samp's skin (`sampgui.png` + `mouse.png`, [`skin`]) as RGBA8, from the user's SA:MP install; `None` = flat
    /// controls.
    skin_image: Option<Vec<u8>>,
    /// The skin raster, 0 before the upload or when it failed.
    skin_raster: u32,
    skin_failed: bool,
    /// The UI font file from the user's system (Arial Bold), `None` = the committed Liberation Sans Bold.
    font_file: Option<Vec<u8>>,
    /// `gtaweap3.ttf` (death window icons).
    weapon_file: Option<Vec<u8>>,
    /// `sampaux3.ttf` (AFK icon).
    aux_file: Option<Vec<u8>>,
    /// The draws of the current pass, drawn after the client's callback.
    list: DrawList,
    /// Sprite textdraw textures: id, text, `RwTexture*` (0 = not available).
    sprites: Vec<(u16, Vec<u8>, u32)>,
    /// The mouse in client coordinates (from the window events).
    cursor: (i32, i32),
    /// samp's input lock and cursor as the client last set them: keyboard, mouse, cursor shown.
    input: (bool, bool, bool),
    /// Passes drawn (for logs).
    pub frames: u64,
}

impl std::fmt::Debug for Overlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Overlay").field("raster", &self.raster).field("frames", &self.frames).finish()
    }
}

/// Bytes of UTF-16 text in the atlas's code page (Latin-1; DXUT's password mask at [`MASK_BYTE`]).
fn narrow(text: &[u16]) -> Vec<u8> {
    text.iter()
        .map(|&c| match c {
            0x25CF => MASK_BYTE,
            0..=0xFF => c as u8,
            _ => b'?',
        })
        .collect()
}

impl Overlay {
    /// Gives the overlay samp's skin texture: `rgba` is [`skin::TEXTURE_W`] × [`skin::TEXTURE_H`] RGBA8
    /// (`sampgui.png` at the top, `mouse.png` at (0, 512)).
    pub fn set_skin(&mut self, rgba: Vec<u8>) {
        if self.skin_raster != 0 {
            render::destroy_raster(self.skin_raster);
        }
        self.skin_raster = 0;
        self.skin_failed = false;
        self.skin_image = Some(rgba);
    }

    /// Whether the skin texture is on the GPU.
    pub fn skin_uploaded(&self) -> bool {
        self.skin_raster != 0
    }

    /// Uses `ttf` (the user's own Arial Bold, as samp's D3DX "Arial") for the UI fonts instead of the committed
    /// Liberation Sans Bold. Ignored when it does not parse.
    pub fn set_font(&mut self, ttf: Vec<u8>) {
        if font::parses(&ttf) {
            self.font_file = Some(ttf);
            self.atlas = None;
        }
    }

    /// The symbol fonts samp draws with: `gtaweap3.ttf` (death window) and `sampaux3.ttf` (AFK icon).
    pub fn set_symbol_fonts(&mut self, weapon: Option<Vec<u8>>, aux: Option<Vec<u8>>) {
        self.weapon_file = weapon.filter(|t| font::parses(t));
        self.aux_file = aux.filter(|t| font::parses(t));
        self.atlas = None;
    }

    /// Whether the glyph atlas is on the GPU (otherwise UI text falls back to the game font).
    pub fn atlas_uploaded(&self) -> bool {
        self.raster != 0
    }

    /// Sprite textdraw textures loaded so far.
    pub fn sprites_loaded(&self) -> usize {
        self.sprites.iter().filter(|s| s.2 != 0).count()
    }

    /// samp's input lock as the client set it: (keyboard, mouse).
    pub fn input_lock(&self) -> (bool, bool) {
        (self.input.0, self.input.1)
    }

    /// Whether the UI shows its cursor.
    pub fn cursor_visible(&self) -> bool {
        self.input.2
    }

    /// Builds the atlas for video width `w` and `/fontsize` `fs` when the sizes changed.
    fn ensure_atlas(&mut self, w: i32, fs: i32) -> bool {
        let heights = font::heights(w, fs);
        if self.atlas.as_ref().is_some_and(|a| a.heights() == heights) {
            return false;
        }
        let faces =
            Faces { ui: self.font_file.as_deref(), weapon: self.weapon_file.as_deref(), aux: self.aux_file.as_deref() };
        self.atlas = Some(FontAtlas::build(faces, heights));
        if self.raster != 0 {
            render::destroy_raster(self.raster);
        }
        self.raster = 0;
        self.upload_failed = false;
        true
    }

    fn atlas(&self) -> Option<&FontAtlas> {
        self.atlas.as_ref()
    }

    /// Uploads what is missing and draws the collected list (and, at `Present`, the cursor).
    fn draw_pass(&mut self, present: bool) {
        if self.atlas.is_none() {
            let (w, _) = render::screen_size();
            self.ensure_atlas(w as i32, 0);
        }
        let atlas = self.atlas.as_ref().expect("built above");
        if self.raster == 0 && !self.upload_failed {
            match render::upload_atlas(atlas) {
                Some(r) => self.raster = r,
                None => self.upload_failed = true,
            }
        }
        if self.skin_raster == 0
            && !self.skin_failed
            && let Some(img) = &self.skin_image
        {
            match render::upload_rgba(skin::TEXTURE_W, skin::TEXTURE_H, img, true) {
                Some(r) => self.skin_raster = r,
                None => self.skin_failed = true,
            }
        }
        let mut list = std::mem::take(&mut self.list);
        if present && self.input.2 {
            skin::cursor(&mut list, self.skin_raster != 0, self.cursor.0 as f32, self.cursor.1 as f32);
        }
        render::draw(&list, atlas, self.raster, self.skin_raster, present);
    }
}

impl UiBackend for Overlay {
    fn video_size(&self) -> (i32, i32) {
        let (w, h) = render::screen_size();
        (w as i32, h as i32)
    }

    fn client_size(&self) -> (i32, i32) {
        self.video_size()
    }

    fn cursor_pos(&self) -> (i32, i32) {
        self.cursor
    }

    fn fonts(&mut self, _face: &[u8], _weight: u32, size: i32, force: bool) -> bool {
        // The face is the user's Arial Bold or Liberation Sans Bold (set by the host), always bold.
        if force {
            self.atlas = None;
        }
        let (w, _) = self.video_size();
        self.ensure_atlas(w, size)
    }

    fn metrics(&self) -> UiMetrics {
        let Some(a) = self.atlas() else { return UiMetrics::default() };
        let (icon_w, icon_h) = if a.has(slot(UiFont::DeathBackdrop)) {
            let si = slot(UiFont::DeathBackdrop);
            (a.width(si, b"G"), a.line_height(si))
        } else {
            // Without GTAWEAPON3: its aux + 12 cell and the glyph's measured width (31 px at 24).
            let h = a.line_height(slot(UiFont::DeathNames)) + 12;
            (((1.3 * h as f32).round()) as i32, h)
        };
        UiMetrics {
            main_h: a.line_height(0),
            small_h: a.line_height(1),
            gui0_h: a.line_height(2),
            gui1_h: a.line_height(3),
            nick_w: a.width(slot(UiFont::DeathNames), samp_ui::deathwindow::NICK_SAMPLE),
            icon_w,
            icon_h,
            timestamp_w: a.width(0, samp_ui::chat::TIMESTAMP_SAMPLE),
        }
    }

    fn measure(&self, f: UiFont, text: &[u8], count: i32, rect: UiRect, flags: u32, _colour: u32) -> UiRect {
        let Some(a) = self.atlas() else { return rect };
        let text = if count < 0 { text } else { &text[..(count as usize).min(text.len())] };
        draw::layout(a, slot(f), text, rect, flags).1
    }

    fn sprite_begin(&mut self, _sprite: UiSprite, _flags: u32) {}

    fn sprite_end(&mut self, _sprite: UiSprite) {}

    fn draw_text(
        &mut self,
        f: UiFont,
        _sprite: Option<UiSprite>,
        text: &[u8],
        count: i32,
        rect: &mut UiRect,
        flags: u32,
        colour: u32,
    ) -> i32 {
        let text = if count < 0 { text } else { &text[..(count as usize).min(text.len())] };
        let Some(a) = self.atlas.as_ref() else { return 0 };
        if flags & dt::CALCRECT != 0 {
            *rect = draw::layout(a, slot(f), text, *rect, flags).1;
            return rect.bottom - rect.top;
        }
        let e = self.list.text(a, slot(f), text, *rect, flags, colour, None, outline::NONE);
        e.bottom - e.top
    }

    fn draw_text_coloured(
        &mut self,
        f: UiFont,
        _sprite: UiSprite,
        text: &[u8],
        rect: &mut UiRect,
        flags: u32,
        colour: u32,
        glyphs: &[u32],
    ) {
        let Some(a) = self.atlas.as_ref() else { return };
        self.list.text(a, slot(f), text, *rect, flags, colour, Some(glyphs), outline::NONE);
    }

    /// The outline passes are the atlas's pre-outlined glyphs: one draw instead of five (or nine).
    fn draw_text_outlined(
        &mut self,
        _sprite: UiSprite,
        f: UiFont,
        _outline_font: UiFont,
        stripped: &[u8],
        rect: UiRect,
        flags: u32,
        colour: u32,
        glyphs: &[u32],
        offsets: &[(i32, i32)],
    ) {
        let Some(a) = self.atlas.as_ref() else { return };
        let kind = match offsets.len() {
            0 => outline::NONE,
            8 => outline::TWO,
            _ => outline::ONE,
        };
        self.list.text(a, slot(f), stripped, rect, flags | dt::NOCLIP, colour, Some(glyphs), kind);
    }

    fn dialog_begin(&mut self, rect: UiRect, background: u32) {
        if background >> 24 != 0 {
            self.list.rect([rect.left, rect.top, rect.right, rect.bottom].map(|v| v as f32), background);
        }
    }

    fn dialog_end(&mut self) {}

    fn gui_sprite(&mut self, src: UiRect, dst: UiRect, colour: u32) {
        if colour >> 24 == 0 {
            return;
        }
        let dst = [dst.left, dst.top, dst.right, dst.bottom].map(|v| v as f32);
        if self.skin_image.is_some() && !self.skin_failed {
            let src = [src.left, src.top, src.right, src.bottom].map(|v| v as f32);
            self.list.cmds.push(Cmd::Skin { src, dst, colour });
        } else {
            self.list.rect(dst, colour);
        }
    }

    fn gui_text(&mut self, f: UiFont, text: &[u8], rect: UiRect, format: u32, colour: u32) {
        if colour >> 24 == 0 {
            return;
        }
        let stripped = samp_ui::colour::strip_codes(text);
        let glyphs: Vec<u32> = samp_ui::colour::glyph_colours(text, colour).iter().map(|g| g.colour).collect();
        let Some(a) = self.atlas.as_ref() else { return };
        self.list.text(a, slot(f), &stripped, rect, format, colour, Some(&glyphs), outline::NONE);
    }

    fn gui_text_wide(&mut self, f: UiFont, text: &[u16], count: i32, rect: UiRect, format: u32, colour: u32) {
        if colour >> 24 == 0 {
            return;
        }
        let text = if count < 0 { text } else { &text[..(count as usize).min(text.len())] };
        let bytes = narrow(text);
        let Some(a) = self.atlas.as_ref() else { return };
        self.list.text(a, slot(f), &bytes, rect, format, colour, None, outline::NONE);
    }

    fn gui_rect(&mut self, rect: UiRect, colour: u32) {
        self.list.rect([rect.left, rect.top, rect.right, rect.bottom].map(|v| v as f32), colour);
    }

    fn text_width_wide(&self, f: UiFont, text: &[u16]) -> i32 {
        self.atlas().map_or(0, |a| a.width(slot(f), &narrow(text)))
    }

    fn to_wide(&self, text: &[u8]) -> Vec<u16> {
        text.iter().map(|&b| u16::from(b)).collect()
    }

    fn bars_begin(&mut self) {}

    fn bars_group(&mut self, quads: &[ColourQuad; 3]) {
        for q in quads {
            self.list.rect([q.x0, q.y0, q.x1, q.y1], q.colour);
        }
    }

    fn bars_end(&mut self) {}

    fn project(&self, p: [f32; 3]) -> [f32; 3] {
        match render::project(Vec3::new(p[0], p[1], p[2])) {
            Some((x, y)) => [x, y, 0.5],
            None => [0.0, 0.0, 2.0],
        }
    }

    fn aux_fonts(&mut self, _create: bool) -> bool {
        self.atlas().is_some_and(|a| a.has(slot(UiFont::Aux20)) && a.has(slot(UiFont::Aux22)))
    }

    fn chat_blit(&mut self) -> bool {
        false
    }

    fn textdraw_units(&self) -> (f32, f32) {
        use sa_core::mem::GPtr;
        (GPtr::<f32>::from_addr(UNIT_X).read(), GPtr::<f32>::from_addr(UNIT_Y).read())
    }

    fn textdraw_text(&mut self, f: &TextdrawFont, text: &[u8], _has_keys: bool) {
        self.list.cmds.push(Cmd::GameFont { font: *f, text: text.to_vec() });
    }

    fn textdraw_sprite(&mut self, id: u16, style: u8, text: &[u8], _serial: u64, rect: [f32; 4], colour: u32) -> bool {
        // Model previews (font 5) have no render on the engine yet: they take the text path.
        if style != samp_ui::textdraw::FONT_SPRITE {
            return false;
        }
        let tex = match self.sprites.iter().find(|s| s.0 == id) {
            Some(s) => s.2,
            None => {
                let t = render::load_sprite(text);
                self.sprites.push((id, text.to_vec(), t));
                t
            }
        };
        if tex != 0 {
            self.list.cmds.push(Cmd::Sprite { tex, rect, colour });
        }
        tex != 0
    }

    fn textdraw_sync(&mut self, live: &[(u16, &[u8])]) {
        self.sprites.retain(|(id, text, _)| live.iter().any(|(i, t)| i == id && *t == text.as_slice()));
    }

    fn set_input_state(&mut self, keyboard: bool, mouse: bool, cursor: bool) {
        self.input = (keyboard, mouse, cursor);
    }
}

impl<V: View> NativeGame<V> {
    /// The overlay (the UI backend and its engine resources).
    pub fn overlay(&mut self) -> &mut Overlay {
        &mut self.overlay
    }

    /// Draws what the client's pass collected (called by [`NativeGame::dispatch_frame`] right after the client's
    /// callback): at `After2d` the textdraws (their text joins the HUD's in `CFont`'s buffer, flushed by
    /// `Game::Idle`), at `Present` (the end of the frame, over the game text and the fade) the rest and the
    /// cursor. Nothing while the pause menu is up (samp draws no UI then).
    pub(crate) fn overlay_frame(&mut self, phase: FramePhase) {
        use sa_samp_api::hud::Hud as _;
        if !matches!(phase, FramePhase::After2d | FramePhase::Present) {
            return;
        }
        if self.menu_active() {
            self.overlay.list.cmds.clear();
            return;
        }
        let present = phase == FramePhase::Present;
        self.overlay.draw_pass(present);
        if present {
            self.overlay.frames += 1;
        }
    }

    /// A window event from the platform: the mouse position is kept for the UI's cursor, then the client's
    /// `GameHooks::window_event` (where samp-rs's UI routes it). Returns whether the game may also see it.
    pub fn window_event(&mut self, e: WindowEvent) -> EventResult {
        if let WindowEvent::MouseMove { x, y } = e {
            self.overlay.cursor = (x, y);
        }
        match self.hooks.take() {
            Some(mut hooks) => {
                let r = hooks.window_event(e, self);
                self.hooks = Some(hooks);
                r
            }
            None => EventResult::PassToGame,
        }
    }

    /// samp's input lock while the UI is up: (keyboard, mouse).
    pub fn ui_input_lock(&self) -> (bool, bool) {
        self.overlay.input_lock()
    }

    /// Whether the SA:MP UI shows its cursor: the host then gives the pointer back to the UI (absolute moves and
    /// clicks).
    pub fn ui_cursor_visible(&self) -> bool {
        self.overlay.cursor_visible()
    }

    /// Live peds within `radius` of `centre` (pool order), for nametag demos and tests.
    pub fn peds_near(&self, centre: Vec3, radius: f32) -> Vec<PedRef> {
        let v = &self.view;
        let pool = v.read_u32(addr::PED_POOL.global);
        if pool == 0 {
            return Vec::new();
        }
        let (objects, map, size) = (v.read_u32(pool), v.read_u32(pool + 4), v.read_i32(pool + 8));
        let mut out = Vec::new();
        for slot in 0..size.max(0) as u32 {
            let b = v.read_u8(map + slot);
            if b & sa_core::pool::FREE != 0 {
                continue;
            }
            let ped = PedRef((slot << 8) | u32::from(b));
            let e = objects + slot * addr::PED_POOL.element;
            if v.read_u32(e + off::VTABLE) == 0 {
                continue;
            }
            if let Ok(s) = self.ped_snapshot(ped) {
                let p = s.matrix.pos;
                let d = ((p.x - centre.x).powi(2) + (p.y - centre.y).powi(2) + (p.z - centre.z).powi(2)).sqrt();
                if d <= radius {
                    out.push(ped);
                }
            }
        }
        out
    }

    /// The camera position (`TheCamera.m_mCameraMatrix`), for nametag distances.
    pub fn camera_pos(&self) -> Vec3 {
        self.view.read_vec3(addr::camera::MATRIX + addr::matrix::POS)
    }
}
