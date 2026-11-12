//! Draws a [`DrawList`] through the ported engine: `CSprite2d::DrawRect` for rectangles, RW Im2D
//! triangle lists (`RwIm2DRenderPrimitive`) for UI-font glyphs (one RW raster holds the glyph atlas) and
//! the cursor, `CFont` for game-font text, `CSprite::CalcScreenCoors` for world → screen. Every call is an
//! `orig!` call: in native sa-game it reaches the Rust port through the address registry, exactly like a
//! port calling another port; no new GPU path.
//!
//! Called at `FramePhase::After2d` (after `Render2dStuff`, inside the camera's `BeginUpdate` /
//! `EndUpdate`), where the HUD's own 2D state is current.

use sa_core::mem::GPtr;
use sa_core::orig;
use sa_samp_api::Vec3;
use sa_samp_api::hud::{FontStyle, TextAlign};
use sa_samp_ui::font::FontAtlas;
use sa_samp_ui::textdraw::SpriteSource;
use sa_samp_ui::{Cmd, DrawList};

orig! {
    /// `CSprite2d::Draw(const CRect&, const CRGBA&)` (0x728350, thiscall)
    fn sprite2d_draw(this: GPtr<u8>, r: GPtr<u8>, c: GPtr<u8>) = thiscall 0x728350 => unported;
    /// `CSprite2d::SetTexture(const char*)` (0x727270, thiscall)
    fn sprite2d_set_texture(this: GPtr<u8>, name: GPtr<u8>) = thiscall 0x727270 => unported;
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    fn txd_find_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => unported;
    /// `CTxdStore::AddTxdSlot(const char*)` (0x731C80)
    fn txd_add_slot(name: GPtr<u8>) -> i32 = cdecl 0x731C80 => unported;
    /// `CTxdStore::LoadTxd(int, const char* filename)` (0x7320B0)
    fn txd_load(slot: i32, file: GPtr<u8>) -> u8 = cdecl 0x7320B0 => unported;
    /// `CTxdStore::AddRef(int)` (0x731A00)
    fn txd_add_ref(slot: i32) = cdecl 0x731A00 => unported;
    /// `CTxdStore::PushCurrentTxd()` (0x7316A0)
    fn txd_push_current() = cdecl 0x7316A0 => unported;
    /// `CTxdStore::SetCurrentTxd(int)` (0x7319C0)
    fn txd_set_current(slot: i32) = cdecl 0x7319C0 => unported;
    /// `CTxdStore::PopCurrentTxd()` (0x7316B0)
    fn txd_pop_current() = cdecl 0x7316B0 => unported;
    /// `CFileMgr::OpenFile(path, mode)` (0x538900)
    fn file_open(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => unported;
    /// `CFileMgr::CloseFile(file)` (0x5389D0)
    fn file_close(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => unported;

    /// `RwRenderStateSet(state, value)` through `RwEngineInstance` +0x20
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwIm2DRenderPrimitive(type, vertices, n)` (0x734E90)
    fn rw_im2d_render_primitive(ty: u32, verts: GPtr<u8>, n: i32) -> i32 = cdecl 0x734E90 => unported;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727B60)
    fn sprite2d_draw_rect(r: GPtr<u8>, c: GPtr<u8>) = cdecl 0x727B60 => unported;
    /// `RwRasterCreate(width, height, depth, flags)` (0x7FB230)
    fn rw_raster_create(w: i32, h: i32, depth: i32, flags: u32) -> GPtr<u8> = cdecl 0x7FB230 => unported;
    /// `RwRasterLock(raster, level, mode)` (0x7FB2D0)
    fn rw_raster_lock(r: GPtr<u8>, level: u32, mode: u32) -> u32 = cdecl 0x7FB2D0 => unported;
    /// `RwRasterUnlock(raster)` (0x7FAEC0)
    fn rw_raster_unlock(r: GPtr<u8>) -> u32 = cdecl 0x7FAEC0 => unported;
    /// `CSprite::CalcScreenCoors(const RwV3d&, RwV3d* out, float* w, float* h, bool, bool)` (0x70CE30)
    fn calc_screen_coors(inp: GPtr<f32>, out: GPtr<f32>, w: GPtr<f32>, h: GPtr<f32>, a: u32, b: u32) -> u8 = cdecl 0x70CE30 => unported;

    /// `CFont::SetScale(float, float)` (0x719380)
    fn font_set_scale(w: f32, h: f32) = cdecl 0x719380 => unported;
    /// `CFont::SetColor(CRGBA)` (0x719430)
    fn font_set_color(c: u32) = cdecl 0x719430 => unported;
    /// `CFont::SetJustify(bool)` (0x719600)
    fn font_set_justify(v: u32) = cdecl 0x719600 => unported;
    /// `CFont::SetOrientation(eFontAlignment)` (0x719610)
    fn font_set_orientation(v: u32) = cdecl 0x719610 => unported;
    /// `CFont::SetWrapx(float)` (0x7194D0)
    fn font_set_wrapx(v: f32) = cdecl 0x7194D0 => unported;
    /// `CFont::SetCentreSize(float)` (0x7194E0)
    fn font_set_centre_size(v: f32) = cdecl 0x7194E0 => unported;
    /// `CFont::SetBackground(bool, bool)` (0x7195C0)
    fn font_set_background(on: u32, outline: u32) = cdecl 0x7195C0 => unported;
    /// `CFont::SetBackgroundColor(CRGBA)` (0x7195E0)
    fn font_set_background_color(c: u32) = cdecl 0x7195E0 => unported;
    /// `CFont::SetProportional(bool)` (0x7195B0)
    fn font_set_proportional(v: u32) = cdecl 0x7195B0 => unported;
    /// `CFont::SetDropColor(CRGBA)` (0x719510)
    fn font_set_drop_color(c: u32) = cdecl 0x719510 => unported;
    /// `CFont::SetDropShadowPosition(int8)` (0x719570)
    fn font_set_drop_shadow_position(v: u32) = cdecl 0x719570 => unported;
    /// `CFont::SetEdge(int8)` (0x719590)
    fn font_set_edge(v: u32) = cdecl 0x719590 => unported;
    /// `CFont::SetFontStyle(eFontStyle)` (0x719490)
    fn font_set_font_style(v: u32) = cdecl 0x719490 => unported;
    /// `CFont::PrintString(float x, float y, const GxtChar*)` (0x71A700)
    fn font_print_string(x: f32, y: f32, text: GPtr<u8>) = cdecl 0x71A700 => unported;
    /// `CFont::DrawFonts()` (0x71A210)
    fn font_draw_fonts() = cdecl 0x71A210 => unported;
    /// `CMessages::AddBigMessage(const GxtChar*, uint32 time, uint16 style)` (0x69F2B0)
    fn add_big_message(text: GPtr<u8>, time: u32, style: u32) = cdecl 0x69F2B0 => unported;
}

/// RenderWare render states.
mod rs {
    pub const TEXTURERASTER: u32 = 1;
    pub const ZTESTENABLE: u32 = 6;
    pub const ZWRITEENABLE: u32 = 8;
    pub const TEXTUREFILTER: u32 = 9;
    pub const SRCBLEND: u32 = 10;
    pub const DESTBLEND: u32 = 11;
    pub const VERTEXALPHAENABLE: u32 = 12;
}
/// `rwPRIMTYPETRILIST`.
const TRILIST: u32 = 3;
/// `rwRASTERTYPETEXTURE | rwRASTERFORMAT8888`.
const RASTER_TEXTURE_8888: u32 = 0x0504;
/// `rwRASTERLOCKWRITE`.
const LOCK_WRITE: u32 = 1;
/// `CSprite2d::NearScreenZ` / `RecipNearClipPlane` (the z / rhw every 2D vertex of the game carries).
const NEAR_SCREEN_Z: usize = 0xC4B8D8;
const RECIP_NEAR_CLIP: usize = 0xC4B8D0;
/// `RsGlobal.maximumWidth` / `maximumHeight`.
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// Vertices per Im2D call (the own vertex buffer holds 0x2492; whole triangles).
const MAX_VERTS: usize = 9000;

/// The screen size the game renders at.
pub fn screen_size() -> (f32, f32) {
    let w = GPtr::<i32>::from_addr(RS_MAX_WIDTH).read();
    let h = GPtr::<i32>::from_addr(RS_MAX_HEIGHT).read();
    (w.max(1) as f32, h.max(1) as f32)
}

/// `CSprite::CalcScreenCoors` with both visibility checks: the screen position or `None`.
pub fn project(p: Vec3) -> Option<(f32, f32)> {
    let mut inp = [p.x, p.y, p.z];
    let mut out = [0.0f32; 3];
    let (mut w, mut h) = (0.0f32, 0.0f32);
    let ok = calc_screen_coors(
        GPtr::from_raw(inp.as_mut_ptr()),
        GPtr::from_raw(out.as_mut_ptr()),
        GPtr::from_raw(&mut w),
        GPtr::from_raw(&mut h),
        1,
        1,
    );
    (ok != 0).then_some((out[0], out[1]))
}

/// Creates the atlas raster and fills it (white, alpha = coverage). `None` when RW refuses the raster.
pub fn upload_atlas(atlas: &FontAtlas) -> Option<u32> {
    let r = rw_raster_create(atlas.width as i32, atlas.height as i32, 32, RASTER_TEXTURE_8888);
    if r.is_null() {
        return None;
    }
    let px = rw_raster_lock(r, 0, LOCK_WRITE);
    if px == 0 {
        return None;
    }
    let stride = GPtr::<i32>::from_addr(r.addr() + 0x18)
        .read()
        .max(atlas.width as i32 * 4) as usize;
    for y in 0..atlas.height as usize {
        let row = GPtr::<u8>::from_addr(px as usize + y * stride);
        for x in 0..atlas.width as usize {
            let i = y * atlas.width as usize + x;
            let (a, l) = (u32::from(atlas.alpha[i]), u32::from(atlas.lum[i]));
            // D3DFMT_A8R8G8B8 in memory: B, G, R, A.
            row.byte_add(4 * x as isize)
                .cast::<u32>()
                .write(a << 24 | l << 16 | l << 8 | l);
        }
    }
    rw_raster_unlock(r);
    Some(r.addr() as u32)
}

/// One `RwIm2DVertex` (D3D9: x, y, z, rhw, ARGB, u, v).
#[derive(Clone, Copy)]
#[repr(C)]
struct Vertex {
    x: f32,
    y: f32,
    z: f32,
    rhw: f32,
    colour: u32,
    u: f32,
    v: f32,
}

/// Batches triangles of one texture (or none) until a draw of another kind needs the screen.
struct Batch {
    raster: u32,
    verts: Vec<Vertex>,
    z: f32,
    rhw: f32,
}

impl Batch {
    fn push_quad(&mut self, q: [f32; 4], uv: [f32; 4], colour: u32) {
        let v = |x: f32, y: f32, u: f32, vv: f32| Vertex {
            x,
            y,
            z: self.z,
            rhw: self.rhw,
            colour,
            u,
            v: vv,
        };
        let (a, b, c, d) = (
            v(q[0], q[1], uv[0], uv[1]),
            v(q[2], q[1], uv[2], uv[1]),
            v(q[2], q[3], uv[2], uv[3]),
            v(q[0], q[3], uv[0], uv[3]),
        );
        self.verts.extend_from_slice(&[a, b, c, a, c, d]);
    }

    fn push_tri(&mut self, pts: [(f32, f32); 3], colour: u32) {
        for (x, y) in pts {
            self.verts.push(Vertex {
                x,
                y,
                z: self.z,
                rhw: self.rhw,
                colour,
                u: 0.0,
                v: 0.0,
            });
        }
    }

    fn flush(&mut self) {
        if self.verts.is_empty() {
            return;
        }
        rw_render_state_set(rs::TEXTURERASTER, self.raster);
        rw_render_state_set(rs::TEXTUREFILTER, 1);
        rw_render_state_set(rs::ZTESTENABLE, 0);
        rw_render_state_set(rs::ZWRITEENABLE, 0);
        rw_render_state_set(rs::VERTEXALPHAENABLE, 1);
        rw_render_state_set(rs::SRCBLEND, 5);
        rw_render_state_set(rs::DESTBLEND, 6);
        for chunk in self.verts.chunks_mut(MAX_VERTS) {
            rw_im2d_render_primitive(
                TRILIST,
                GPtr::from_raw(chunk.as_mut_ptr().cast::<u8>()),
                chunk.len() as i32,
            );
        }
        rw_render_state_set(rs::TEXTURERASTER, 0);
        self.verts.clear();
    }
}

/// `CRGBA` as the setters take it by value: R, G, B, A in memory order.
fn crgba(argb: u32) -> u32 {
    let (a, r, g, b) = (argb >> 24, (argb >> 16) & 0xFF, (argb >> 8) & 0xFF, argb & 0xFF);
    r | g << 8 | b << 16 | a << 24
}

fn rgba_of(c: sa_samp_api::Rgba) -> u32 {
    u32::from(c.r) | u32::from(c.g) << 8 | u32::from(c.b) << 16 | u32::from(c.a) << 24
}

/// Game text for `CFont`: Latin-1 bytes, NUL-terminated; characters outside become `?`.
pub fn gxt_bytes(text: &str) -> Vec<u8> {
    let mut v: Vec<u8> = text
        .chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
        .collect();
    v.push(0);
    v
}

/// The `CFont` state of a textdraw, in samp's setter order, then `PrintString` and `DrawFonts`.
fn game_font(style: &FontStyle, x: f32, y: f32, text: &str) {
    font_set_scale(style.scale.x, style.scale.y);
    font_set_color(rgba_of(style.colour));
    font_set_justify(0);
    font_set_orientation(match style.align {
        TextAlign::Centre => 0,
        TextAlign::Left => 1,
        TextAlign::Right => 2,
    });
    font_set_wrapx(style.wrap_x);
    font_set_centre_size(style.centre_size);
    font_set_background(u32::from(style.background), 0);
    font_set_background_color(rgba_of(style.background_colour));
    font_set_proportional(u32::from(style.proportional));
    font_set_drop_color(rgba_of(style.drop_colour));
    if style.edge != 0 {
        font_set_edge(style.edge as u8 as u32);
    } else {
        font_set_drop_shadow_position(style.shadow as u8 as u32);
    }
    font_set_font_style(u32::from(style.font));
    let mut buf = gxt_bytes(text);
    font_print_string(x, y, GPtr::from_raw(buf.as_mut_ptr()));
    font_set_edge(0);
    font_draw_fonts();
}

/// Draws `list`. `raster` is the uploaded atlas (0 = none: UI text then goes through `CFont` font 1).
pub fn draw(list: &DrawList, atlas: &FontAtlas, raster: u32, sprites: &mut Sprites) {
    let z = GPtr::<f32>::from_addr(NEAR_SCREEN_Z).read();
    let rhw = GPtr::<f32>::from_addr(RECIP_NEAR_CLIP).read();
    let mut glyphs = Batch {
        raster,
        verts: Vec::new(),
        z,
        rhw,
    };
    let mut solid = Batch {
        raster: 0,
        verts: Vec::new(),
        z,
        rhw,
    };
    let (aw, ah) = (atlas.width as f32, atlas.height as f32);
    for cmd in &list.cmds {
        match cmd {
            Cmd::Rect { rect, colour } => {
                glyphs.flush();
                solid.flush();
                // `CRect` {left, bottom, right, top} as the HUD passes it (y down: bottom = the larger y).
                let mut r = [rect[0], rect[3], rect[2], rect[1]];
                let mut c = crgba(*colour);
                sprite2d_draw_rect(
                    GPtr::from_raw(r.as_mut_ptr().cast::<u8>()),
                    GPtr::from_raw(&mut c).cast(),
                );
            }
            Cmd::Triangle { pts, colour } => {
                glyphs.flush();
                solid.push_tri(*pts, *colour);
            }
            Cmd::Text {
                x,
                y,
                font,
                colour,
                text,
                outline,
            } => {
                solid.flush();
                if raster == 0 {
                    glyphs.flush();
                    let h = atlas.heights()[font.index()] as f32;
                    let style = FontStyle {
                        scale: sa_samp_api::Point2 {
                            x: h / 36.0,
                            y: h / 18.0,
                        },
                        colour: sa_samp_api::Rgba::from_argb_u32(*colour),
                        font: 1,
                        wrap_x: 4000.0,
                        centre_size: 4000.0,
                        drop_colour: sa_samp_api::Rgba::default(),
                        shadow: 0,
                        edge: i8::from(*outline),
                        proportional: true,
                        background: false,
                        background_colour: sa_samp_api::Rgba::default(),
                        align: TextAlign::Left,
                    };
                    game_font(&style, *x, *y, text);
                    continue;
                }
                let mut pen = x.round();
                let top = y.round();
                for ch in text.chars() {
                    let g = atlas.glyph_variant(*font, *outline, ch);
                    if g.rect[2] > 0 {
                        let gx = pen + g.offset.0.round() - 0.5;
                        let gy = top + g.offset.1.round() - 0.5;
                        let [rx, ry, rw, rh] = g.rect.map(f32::from);
                        glyphs.push_quad(
                            [gx, gy, gx + rw, gy + rh],
                            [rx / aw, ry / ah, (rx + rw) / aw, (ry + rh) / ah],
                            *colour,
                        );
                    }
                    pen += g.advance;
                }
            }
            Cmd::GameFont { style, x, y, text } => {
                glyphs.flush();
                solid.flush();
                game_font(style, *x, *y, text);
            }
            Cmd::Sprite { source, rect, colour } => {
                glyphs.flush();
                solid.flush();
                let tex = sprites.get(source);
                if tex != 0 {
                    // samp: bilinear filter, then `CSprite2d::Draw(CRect{l, b, r, t}, colour)`.
                    rw_render_state_set(rs::TEXTUREFILTER, 2);
                    let mut sprite = tex;
                    let mut r = [rect[0], rect[3], rect[2], rect[1]];
                    let mut c = crgba(*colour);
                    sprite2d_draw(
                        GPtr::from_raw(&mut sprite).cast(),
                        GPtr::from_raw(r.as_mut_ptr().cast::<u8>()),
                        GPtr::from_raw(&mut c).cast(),
                    );
                }
            }
        }
    }
    glyphs.flush();
    solid.flush();
}

/// Sprite textdraw textures (`RwTexture*` by `txd:name`; 0 = not available), loaded on first use.
#[derive(Default)]
pub struct Sprites {
    loaded: std::collections::HashMap<(String, String), u32>,
}

impl Sprites {
    /// The texture of `source`, loading its TXD like samp (find the slot by name, else add one and load the
    /// file when it exists), then `CSprite2d::SetTexture` with the slot current.
    pub fn get(&mut self, source: &SpriteSource) -> u32 {
        let key = (source.slot.clone(), source.texture.clone());
        if let Some(&t) = self.loaded.get(&key) {
            return t;
        }
        let t = load_sprite(source);
        self.loaded.insert(key, t);
        t
    }

    /// Textures loaded (non-zero).
    pub fn count(&self) -> usize {
        self.loaded.values().filter(|&&t| t != 0).count()
    }
}

/// A NUL-terminated copy.
fn cstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

/// Whether the game's file layer can open `path` (`CFileMgr::OpenFile` / `CloseFile`): `LoadTxd` retries
/// forever on a missing file, as the original does.
fn file_exists(path: &str) -> bool {
    let mut p = cstr(path);
    let mut mode = cstr("rb");
    let f = file_open(GPtr::from_raw(p.as_mut_ptr()), GPtr::from_raw(mode.as_mut_ptr()));
    if f.is_null() {
        return false;
    }
    file_close(f);
    true
}

fn load_sprite(source: &SpriteSource) -> u32 {
    let mut slot_name = cstr(&source.slot);
    let mut slot = txd_find_slot(GPtr::from_raw(slot_name.as_mut_ptr()));
    if slot == -1 {
        let Some(file) = &source.file else { return 0 };
        if !file_exists(file) {
            return 0;
        }
        slot = txd_add_slot(GPtr::from_raw(slot_name.as_mut_ptr()));
        let mut f = cstr(file);
        txd_load(slot, GPtr::from_raw(f.as_mut_ptr()));
        txd_add_ref(slot);
    }
    let mut sprite: u32 = 0;
    let mut name = cstr(&source.texture);
    txd_push_current();
    txd_set_current(slot);
    sprite2d_set_texture(
        GPtr::from_raw(&mut sprite).cast(),
        GPtr::from_raw(name.as_mut_ptr()),
    );
    txd_pop_current();
    sprite
}

/// Big-message buffers: `AddBigMessage` keeps the pointer (samp copies into a static buffer per call).
pub struct BigMessages {
    /// Seven buffers, one per style; the vector is never resized, so the pointers stay valid.
    bufs: Vec<[u8; 512]>,
}

impl Default for BigMessages {
    fn default() -> Self {
        Self {
            bufs: vec![[0u8; 512]; 7],
        }
    }
}

impl BigMessages {
    /// `CMessages::AddBigMessage(text, time, style)` with a buffer that lives as long as the overlay.
    pub fn show(&mut self, text: &str, time_ms: u32, style: u8) {
        let Some(buf) = self.bufs.get_mut(usize::from(style)) else {
            return;
        };
        let b = gxt_bytes(text);
        let n = b.len().min(511);
        buf[..n].copy_from_slice(&b[..n]);
        buf[n] = 0;
        add_big_message(GPtr::from_raw(buf.as_mut_ptr()), time_ms, u32::from(style));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_and_text_conversions() {
        assert_eq!(crgba(0x8011_2233), 0x8033_2211);
        assert_eq!(gxt_bytes("Hé€"), vec![b'H', 0xE9, b'?', 0]);
        assert_eq!(std::mem::size_of::<Vertex>(), 0x1C);
    }
}
