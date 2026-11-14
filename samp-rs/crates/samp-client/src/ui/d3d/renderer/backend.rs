//! [`UiBackend`] on Direct3D 9: each method is the D3D / D3DX / DXUT call samp's code makes at that point, so the
//! shared pass (`crate::ui::render`) produces samp's call stream. Plus the chat render-target update and the
//! number-plate painter, which only this backend has.

use std::ffi::c_void;

use sa_samp_api::ui::{ColourQuad, TextdrawFont, UiBackend, UiFont, UiMetrics, UiRect, UiSprite};
use samp_ui::chat::ChatWindow;
use samp_ui::colour;

use super::{Renderer, client_size, cursor_pos, gdi_width, to_wide, video_height, video_width};
use crate::patch::{Memory, ProcessMemory};
use crate::ui::d3d::{self, Font, Rect, Sprite, devcall};
use crate::ui::dxut;

/// Unit constants read at draw time (1/640, 1/448).
const UNIT_X: u32 = 0x859520;
const UNIT_Y: u32 = 0x859524;

fn rect(r: UiRect) -> Rect {
    Rect { left: r.left, top: r.top, right: r.right, bottom: r.bottom }
}

fn ui_rect(r: Rect) -> UiRect {
    UiRect::new(r.left, r.top, r.right, r.bottom)
}

impl Renderer {
    fn sprite(&self, s: UiSprite) -> &Sprite {
        let sp = &self.sprites;
        match s {
            UiSprite::Text => &sp.text,
            UiSprite::Chat => &sp.chat,
            UiSprite::Blit => &sp.blit,
            UiSprite::Gui => &sp.gui,
            UiSprite::Death => &sp.death,
            UiSprite::Tags => &sp.tags,
            UiSprite::Text3d => &sp.text3d,
        }
    }

    fn font(&self, f: UiFont) -> Option<&Font> {
        let fo = &self.fonts;
        Some(match f {
            UiFont::Main => &fo.main,
            UiFont::MainOutline => &fo.main_outline,
            UiFont::Small => &fo.small,
            UiFont::SmallOutline => &fo.small_outline,
            UiFont::Gui0 => &fo.gui0,
            UiFont::Gui1 => &fo.gui1,
            UiFont::DeathNames => &fo.death_names,
            UiFont::DeathIcon => &fo.death_icon,
            UiFont::DeathBackdrop => &fo.death_backdrop,
            UiFont::Aux20 => &self.aux.as_ref()?.0,
            UiFont::Aux22 => &self.aux.as_ref()?.1,
        })
    }

    /// World point to screen `(x, y, z)` with samp's copies of the view and projection matrices.
    pub(super) fn project_point(&self, p: [f32; 3]) -> [f32; 3] {
        let vp = devcall::viewport();
        let proj = *d3d::PROJECTION.lock().unwrap_or_else(|e| e.into_inner());
        let view = *d3d::VIEW.lock().unwrap_or_else(|e| e.into_inner());
        self.d3dx.project(p, &vp, &proj, &view)
    }

    /// Render-target update, once per game frame from the `CGame::Process` hook: invalid while the GTA menu is
    /// on; redrawn when the chat is dirty or the scroll position changed. Mirrors samp.dll 0x10067ED0.
    pub fn update_chat_rt(&mut self, chat: &mut ChatWindow, menu: bool) {
        if menu {
            self.rt_valid = false;
            return;
        }
        if !chat.mode().chat_visible() {
            return;
        }
        if self.rt.is_none() {
            return;
        }
        if self.rt_valid && !chat.is_dirty() && self.rt_scroll == chat.scroll_pos() {
            return;
        }
        let layout = chat.layout(&crate::ui::render::Measure(self), video_width());
        if self.rt.as_ref().map_or(-1, |r| r.begin()) < 0 {
            return;
        }
        devcall::clear(3, 0, 0.0, 0);
        self.sprites.chat.begin(d3d::sprite::ALPHABLEND);
        devcall::set_render_state(20, 14); // D3DRS_DESTBLEND = D3DBLEND_BLENDFACTOR, as samp sets it
        crate::ui::render::chat_lines(self, &layout);
        self.sprites.chat.end();
        if let Some(rt) = &self.rt {
            rt.end();
        }
        self.rt_valid = true;
        self.rt_scroll = chat.scroll_pos();
        chat.clear_dirty();
    }
}

impl UiBackend for Renderer {
    fn video_size(&self) -> (i32, i32) {
        (video_width(), video_height())
    }

    fn client_size(&self) -> (i32, i32) {
        client_size()
    }

    fn cursor_pos(&self) -> (i32, i32) {
        cursor_pos()
    }

    fn metrics(&self) -> UiMetrics {
        let f = &self.fonts;
        UiMetrics {
            main_h: f.main_h,
            small_h: f.small_h,
            gui0_h: f.gui0_height,
            gui1_h: f.gui1_height,
            nick_w: f.nick_w,
            icon_w: f.icon_w,
            icon_h: f.icon_h,
            timestamp_w: f.timestamp_w,
        }
    }

    fn measure(&self, font: UiFont, text: &[u8], count: i32, r: UiRect, flags: u32, colour: u32) -> UiRect {
        let Some(f) = self.font(font) else { return r };
        let mut rr = rect(r);
        f.draw_n(std::ptr::null_mut(), text, count, &mut rr, flags, colour);
        ui_rect(rr)
    }

    fn sprite_begin(&mut self, sprite: UiSprite, flags: u32) {
        self.sprite(sprite).begin(flags);
    }

    fn sprite_end(&mut self, sprite: UiSprite) {
        self.sprite(sprite).end();
    }

    fn draw_text(
        &mut self,
        font: UiFont,
        sprite: Option<UiSprite>,
        text: &[u8],
        count: i32,
        r: &mut UiRect,
        flags: u32,
        colour: u32,
    ) -> i32 {
        let Some(f) = self.font(font) else { return 0 };
        let sp = sprite.map_or(std::ptr::null_mut(), |s| self.sprite(s).ptr);
        let mut rr = rect(*r);
        let h = f.draw_n(sp, text, count, &mut rr, flags, colour);
        *r = ui_rect(rr);
        h
    }

    fn draw_text_coloured(
        &mut self,
        font: UiFont,
        sprite: UiSprite,
        text: &[u8],
        r: &mut UiRect,
        flags: u32,
        colour: u32,
        glyphs: &[u32],
    ) {
        let Some(f) = self.font(font) else { return };
        let mut rr = rect(*r);
        d3d::draw_glyph_coloured(f, self.sprite(sprite), text, &mut rr, flags, colour, glyphs.to_vec());
        *r = ui_rect(rr);
    }

    fn dialog_begin(&mut self, r: UiRect, background: u32) {
        dxut::dialog_begin(&self.sprites.gui, rect(r), background, self.gui);
    }

    fn dialog_end(&mut self) {
        dxut::dialog_end(&self.sprites.gui);
    }

    fn gui_sprite(&mut self, src: UiRect, dst: UiRect, colour: u32) {
        dxut::draw_sprite(&self.sprites.gui, self.gui, rect(src), rect(dst), colour);
    }

    fn gui_text(&mut self, font: UiFont, text: &[u8], r: UiRect, format: u32, colour: u32) {
        if let Some(f) = self.font(font) {
            dxut::draw_text(&self.sprites.gui, f, text, rect(r), format, colour);
        }
    }

    fn gui_text_wide(&mut self, font: UiFont, text: &[u16], count: i32, r: UiRect, format: u32, colour: u32) {
        if let Some(f) = self.font(font) {
            dxut::draw_text_w(&self.sprites.gui, f, text, count, rect(r), format, colour);
        }
    }

    fn gui_rect(&mut self, r: UiRect, colour: u32) {
        dxut::draw_rect(&self.sprites.gui, rect(r), colour);
    }

    fn text_width_wide(&self, font: UiFont, text: &[u16]) -> i32 {
        self.font(font).map_or(0, |f| gdi_width(f, text))
    }

    fn to_wide(&self, text: &[u8]) -> Vec<u16> {
        to_wide(text)
    }

    /// The painter's begin creates its `D3DSBT_ALL` state block on first use (after the sprite's
    /// `OnResetDevice`) and captures it; its end applies it after the sprite's `End` (ui-rendering.md, "Bars").
    fn bars_begin(&mut self) {
        if self.tags_state_block.is_null() {
            self.sprites.tags.on_reset();
            self.tags_state_block = devcall::create_state_block(1);
        }
        d3d::state_block_capture(self.tags_state_block);
        self.sprites.tags.begin(d3d::sprite::DONOTSAVESTATE);
        dxut::ui_states();
    }

    fn bars_group(&mut self, quads: &[ColourQuad; 3]) {
        dxut::set_colour_fvf();
        for q in quads {
            dxut::colour_quad(q.x0, q.y0, q.x1, q.y1, q.colour);
        }
    }

    fn bars_end(&mut self) {
        self.sprites.tags.end();
        d3d::state_block_apply(self.tags_state_block);
    }

    fn project(&self, p: [f32; 3]) -> [f32; 3] {
        self.project_point(p)
    }

    fn aux_fonts(&mut self, create: bool) -> bool {
        if create && self.aux.is_none() {
            match (super::aux_font(&self.d3dx, 20), super::aux_font(&self.d3dx, 22)) {
                (Ok(a), Ok(b)) => self.aux = Some((a, b)),
                (Err(e), _) | (_, Err(e)) => crate::log!("ui: SAMPAUX3 fonts: {e}"),
            }
        }
        self.aux.is_some()
    }

    fn chat_blit(&mut self) -> bool {
        let (Some(rt), true) = (&self.rt, self.rt_valid) else { return false };
        let s = &self.sprites.blit;
        s.begin(d3d::sprite::ALPHABLEND);
        s.draw(rt.tex, None, None, None, 0xFFFF_FFFF);
        s.end();
        true
    }

    fn textdraw_units(&self) -> (f32, f32) {
        let mem = ProcessMemory;
        (mem.read_f32(UNIT_X).unwrap_or(1.0 / 640.0), mem.read_f32(UNIT_Y).unwrap_or(1.0 / 448.0))
    }

    fn textdraw_text(&mut self, font: &TextdrawFont, text: &[u8], has_keys: bool) {
        crate::ui::textdraws::print(font, text, has_keys);
    }

    fn textdraw_sprite(&mut self, id: u16, style: u8, text: &[u8], serial: u64, r: [f32; 4], colour: u32) -> bool {
        let tex = if style == samp_ui::textdraw::FONT_SPRITE {
            crate::ui::textdraws::sprite_texture(id, text)
        } else {
            crate::ui::preview::texture(id, serial).unwrap_or(0)
        };
        if tex != 0 {
            crate::ui::textdraws::draw_texture(tex, r, colour);
        }
        tex != 0
    }

    fn textdraw_sync(&mut self, live: &[(u16, &[u8])]) {
        crate::ui::textdraws::sync_sprites(live);
    }
}

/// The number-plate painter: the 128×32 render target is cleared to 0xFFBEB6A8 and the plate text drawn by the
/// font renderer (Arial 38/10 bold, four 0xB4F0EEE4 outline passes at (0,−2)/(+2,0)/(0,+2)/(−2,0) around
/// (0, 3), then 0xEE444470; rect right / bottom 128 / 32, format 0x105, colour codes removed), then copied with
/// a linear filter into a new 64×32 texture (default pool). Returns the texture, 0 on failure.
/// Mirrors samp.dll B 0x1006D840.
impl Renderer {
    pub fn paint_plate(&self, text: &[u8]) -> u32 {
        const FMT: u32 = 0x105;
        const OUTLINE: u32 = 0xB4F0_EEE4;
        const TEXT: u32 = 0xEE44_4470;
        let Some(rt) = &self.unused.plate_rt else { return 0 };
        let mut tex = std::ptr::null_mut();
        // 22 = D3DFMT_X8R8G8B8, the format samp's plate painter uses.
        let hr = (self.d3dx.create_texture)(d3d::real_device(), 64, 32, 1, 0, 22, 0, &mut tex);
        if hr < 0 || tex.is_null() {
            return 0;
        }
        rt.begin();
        devcall::clear(3, 0xFFBE_B6A8, 1.0, 0);
        let stripped = colour::strip_codes(text);
        let s = &self.sprites.text;
        s.begin(d3d::sprite::ALPHABLEND);
        let font = &self.unused.plate_font;
        // Every pass goes through the font renderer's colour proxy, as in samp: the glyphs land on the renderer's
        // sprite.
        for (dx, dy, argb) in [(0, -2, OUTLINE), (2, 0, OUTLINE), (0, 2, OUTLINE), (-2, 0, OUTLINE), (0, 0, TEXT)] {
            let mut r = Rect { left: dx, top: 3 + dy, right: 128, bottom: 32 };
            let glyphs: Vec<u32> = colour::glyph_colours(&stripped, argb).iter().map(|g| g.colour).collect();
            d3d::draw_glyph_coloured(font, s, &stripped, &mut r, FMT, argb, glyphs);
        }
        s.end();
        rt.end();
        // SAFETY: `tex` is the live texture just created (slot 14 SetAutoGenFilterType, slot 18 GetSurfaceLevel);
        // `surf` is checked for null and released after the copy.
        unsafe {
            let f: extern "system" fn(*mut c_void, u32) -> d3d::HResult = d3d::vfn(tex, 14);
            f(tex, 2);
            let mut surf = std::ptr::null_mut();
            let f: extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> d3d::HResult = d3d::vfn(tex, 18);
            f(tex, 0, &mut surf);
            if !surf.is_null() {
                (self.d3dx.load_surface_from_surface)(
                    surf,
                    std::ptr::null(),
                    std::ptr::null(),
                    rt.surface,
                    std::ptr::null(),
                    std::ptr::null(),
                    3,
                    0,
                );
                d3d::release(surf);
            }
        }
        tex as u32
    }
}
