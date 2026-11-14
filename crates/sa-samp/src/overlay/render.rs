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
use sa_samp_api::ui::TextdrawFont;

use super::draw::{Cmd, DrawList};
use super::font::FontAtlas;
use super::skin;

use sa_hud::bindings::font_draw_fonts;
use sa_hud::bindings::font_print_string;
use sa_hud::bindings::font_set_background;
use sa_hud::bindings::font_set_color;
use sa_hud::bindings::font_set_edge;
use sa_hud::bindings::font_set_font_style;
use sa_hud::bindings::font_set_orientation;
use sa_hud::bindings::font_set_proportional;
use sa_stream::bindings::pop_current_txd as txd_pop_current;
use sa_stream::bindings::push_current_txd as txd_push_current;
use sa_core::bindings::rw_im2d_render_primitive;
use sa_rwcore::bindings::rw_raster_destroy;
use sa_rwcore::bindings::rw_raster_lock;
use sa_stream::bindings::set_current_txd as txd_set_current;
use sa_core::bindings::sprite2d_draw;
use sa_core::bindings::sprite2d_set_texture;
use sa_render::bindings::sprite_draw_rect as sprite2d_draw_rect;
use sa_stream::bindings::txd_add_ref;
use sa_core::bindings::add_txd_slot as txd_add_slot;
use sa_core::bindings::calc_screen_coors;
use sa_core::bindings::file_close;
use sa_core::bindings::find_txd_slot as txd_find_slot;
use sa_core::bindings::font_set_background_color;
use sa_core::bindings::font_set_centre_size;
use sa_core::bindings::font_set_drop_color;
use sa_core::bindings::font_set_drop_shadow_position;
use sa_core::bindings::font_set_justify;
use sa_hud::bindings::font_set_scale;
use sa_core::bindings::font_set_wrapx;
use sa_core::bindings::load_txd as txd_load;
use sa_core::bindings::open_file as file_open;
use sa_rwcore::bindings::rw_raster_create;
use sa_rwcore::bindings::rw_raster_unlock;
use sa_core::bindings::world_get_is_line_of_sight_clear as line_of_sight_clear;
orig! {
    /// `RwRenderStateSet(state, value)` through `RwEngineInstance` +0x20
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
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
/// `rwRASTERFORMATMIPMAP`: a raster with its full mip chain (each level written by the caller).
const RASTER_MIPMAP: u32 = 0x8000;
/// `rwFILTERLINEARMIPLINEAR`.
const FILTER_TRILINEAR: u32 = 6;
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

/// samp's nametag / label test (opcode 06BD: buildings and objects, no see-through or camera-ignore check).
pub fn line_clear(a: Vec3, b: Vec3) -> bool {
    let mut pa = [a.x, a.y, a.z];
    let mut pb = [b.x, b.y, b.z];
    line_of_sight_clear(
        GPtr::<f32>::cast::<u8>(GPtr::from_raw(pa.as_mut_ptr())),
        GPtr::<f32>::cast::<u8>(GPtr::from_raw(pb.as_mut_ptr())),
        1,
        0,
        0,
        1,
        0,
        0,
        0,
    ) != 0
}

/// Frees an atlas raster (the font sizes changed).
pub fn destroy_raster(r: u32) {
    rw_raster_destroy(GPtr::from_addr(r as usize));
}

/// Creates the atlas raster and fills it (white, alpha = coverage). `None` when RW refuses the raster.
pub fn upload_atlas(atlas: &FontAtlas) -> Option<u32> {
    upload_rgba(atlas.width, atlas.height, &atlas.rgba(), false)
}

/// Creates a `w × h` texture raster from RGBA8 pixels; with `mipmaps` also its full mip chain, filled with
/// [`skin::mip_chain`] (D3DX's box filter). `None` when RW refuses the raster.
pub fn upload_rgba(w: u32, h: u32, rgba: &[u8], mipmaps: bool) -> Option<u32> {
    if rgba.len() < (w * h * 4) as usize {
        return None;
    }
    let flags = RASTER_TEXTURE_8888 | if mipmaps { RASTER_MIPMAP } else { 0 };
    let r = rw_raster_create(((w as i32) as u32) as i32, ((h as i32) as u32) as i32, 32, flags);
    if r.is_null() {
        return None;
    }
    let levels = if mipmaps {
        skin::mip_chain(w, h, rgba)
    } else {
        vec![(w, h, rgba[..(w * h * 4) as usize].to_vec())]
    };
    for (level, (lw, lh, pixels)) in levels.iter().enumerate() {
        let px = GPtr::<u8>::addr(GPtr::from_addr(rw_raster_lock(r, level as u32, LOCK_WRITE) as usize)) as u32;
        if px == 0 {
            return if level == 0 { None } else { Some(r.addr() as u32) };
        }
        let stride = GPtr::<i32>::from_addr(r.addr() + 0x18)
            .read()
            .max(*lw as i32 * 4) as usize;
        for y in 0..*lh as usize {
            let row = GPtr::<u8>::from_addr(px as usize + y * stride);
            for x in 0..*lw as usize {
                let i = 4 * (y * *lw as usize + x);
                let (r8, g8, b8, a8) = (
                    u32::from(pixels[i]),
                    u32::from(pixels[i + 1]),
                    u32::from(pixels[i + 2]),
                    u32::from(pixels[i + 3]),
                );
                // D3DFMT_A8R8G8B8 in memory: B, G, R, A.
                row.byte_add(4 * x as isize)
                    .cast::<u32>()
                    .write(a8 << 24 | r8 << 16 | g8 << 8 | b8);
            }
        }
        rw_raster_unlock(r);
    }
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
    /// `rwFILTERNEAREST` (1) or `rwFILTERLINEAR` (2).
    filter: u32,
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
        rw_render_state_set(rs::TEXTUREFILTER, self.filter);
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

/// A NUL-terminated copy.
fn cstr(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.push(0);
    v
}

/// The `CFont` calls of a text textdraw in samp's setter order, then `PrintString` and `SetEdge(0)`.
pub fn game_font(f: &TextdrawFont, text: &[u8]) {
    font_set_scale(f.scale.0.to_bits(), f.scale.1.to_bits());
    font_set_color(f.colour);
    font_set_justify(0);
    font_set_orientation(u32::from(f.orientation));
    font_set_wrapx(f.wrap_x);
    font_set_centre_size(f.centre_size);
    font_set_background(u32::from(f.background), 0);
    font_set_background_color(f.background_colour);
    font_set_proportional(u32::from(f.proportional));
    font_set_drop_color(f.drop_colour);
    if f.outline != 0 {
        font_set_edge(u32::from(f.outline));
    } else {
        font_set_drop_shadow_position(u32::from(f.shadow));
    }
    font_set_font_style(u32::from(f.font));
    let mut buf = cstr(text);
    font_print_string(f.x, f.y, GPtr::from_raw(buf.as_mut_ptr()));
    font_set_edge(0);
}

/// Draws `list`. `raster` is the uploaded atlas (0 = none: UI text then goes through `CFont` font 1);
/// `skin` the skin texture raster ([`skin`]; 0 = none: the backend drew flat controls).
///
/// `flush_fonts`: draw `CFont`'s buffer after each game-font text (the `Present` pass, after `Game::Idle`'s
/// own `DrawFonts`); without it (the `After2d` pass) the textdraws' text stays in the buffer with the HUD's,
/// as on samp.
pub fn draw(
    list: &DrawList,
    atlas: &FontAtlas,
    raster: u32,
    skin: u32,
    flush_fonts: bool,
) {
    let z = GPtr::<f32>::from_addr(NEAR_SCREEN_Z).read();
    let rhw = GPtr::<f32>::from_addr(RECIP_NEAR_CLIP).read();
    let mut glyphs = Batch {
        raster,
        filter: 1,
        verts: Vec::new(),
        z,
        rhw,
    };
    let mut solid = Batch {
        raster: 0,
        filter: 1,
        verts: Vec::new(),
        z,
        rhw,
    };
    // samp's GUI skin is a D3DX texture with its full mip chain (`D3DXCreateTextureFromFileEx`, D3DX_DEFAULT
    // levels, box filter) and `ID3DXSprite::Begin` sets MIN / MAG / MIP filter LINEAR: scaled controls
    // (buttons 136 × 54 → 90 × 30) are sampled trilinearly. The skin raster carries the same box-filtered chain.
    let mut skins = Batch {
        raster: skin,
        filter: FILTER_TRILINEAR,
        verts: Vec::new(),
        z,
        rhw,
    };
    let (sw, sh) = (skin::TEXTURE_W as f32, skin::TEXTURE_H as f32);
    let (aw, ah) = (atlas.width as f32, atlas.height as f32);
    for cmd in &list.cmds {
        match cmd {
            Cmd::Rect { rect, colour } => {
                glyphs.flush();
                solid.flush();
                skins.flush();
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
                skins.flush();
                solid.push_tri(*pts, *colour);
            }
            Cmd::Text {
                x,
                y,
                slot,
                colour,
                text,
                outline,
            } => {
                solid.flush();
                skins.flush();
                if raster == 0 {
                    glyphs.flush();
                    let h = atlas.heights()[*slot] as f32;
                    let f = TextdrawFont {
                        scale: (h / 36.0, h / 18.0),
                        colour: crgba(*colour),
                        orientation: 1,
                        wrap_x: 4000.0,
                        centre_size: 4000.0,
                        outline: *outline,
                        font: 1,
                        proportional: true,
                        x: *x,
                        y: *y,
                        ..TextdrawFont::default()
                    };
                    game_font(&f, text);
                    font_draw_fonts();
                    continue;
                }
                let mut pen = x.round();
                let top = y.round();
                for &b in text {
                    let g = atlas.glyph(*slot, *outline, b);
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
            Cmd::Skin { src, dst, colour } => {
                glyphs.flush();
                solid.flush();
                if skin == 0 {
                    continue;
                }
                skins.push_quad(
                    [dst[0] - 0.5, dst[1] - 0.5, dst[2] - 0.5, dst[3] - 0.5],
                    [src[0] / sw, src[1] / sh, src[2] / sw, src[3] / sh],
                    *colour,
                );
            }
            Cmd::GameFont { font, text } => {
                glyphs.flush();
                solid.flush();
                skins.flush();
                game_font(font, text);
                if flush_fonts {
                    font_draw_fonts();
                }
            }
            Cmd::Sprite { tex, rect, colour } => {
                glyphs.flush();
                solid.flush();
                skins.flush();
                let tex = *tex;
                if tex != 0 {
                    // samp: bilinear filter, then `CSprite2d::Draw(CRect{l, b, r, t}, colour)`.
                    rw_render_state_set(rs::TEXTUREFILTER, 2);
                    let mut sprite = tex;
                    let mut r = [rect[0], rect[3], rect[2], rect[1]];
                    let mut c = *colour;
                    sprite2d_draw(
                        GPtr::from_raw(&mut sprite).cast(),
                        GPtr::<u8>::cast::<f32>(GPtr::from_raw(r.as_mut_ptr().cast::<u8>())),
                        GPtr::from_raw(&mut c).cast(),
                    );
                }
            }
        }
    }
    glyphs.flush();
    solid.flush();
    skins.flush();
}

/// Whether the game's file layer can open `path` (`CFileMgr::OpenFile` / `CloseFile`): `LoadTxd` retries
/// forever on a missing file, as the original does.
fn file_exists(path: &[u8]) -> bool {
    let mut p = cstr(path);
    let mut mode = cstr(b"rb");
    let f = file_open(GPtr::from_raw(p.as_mut_ptr()), GPtr::from_raw(mode.as_mut_ptr()));
    if f.is_null() {
        return false;
    }
    file_close(f);
    true
}

/// The texture of a sprite textdraw's `txd:name` (0 when unavailable), loaded as samp does
/// (`samp_ui::textdraw::sprite_source`): the slot by name, else a new slot (added also when the file is missing)
/// with the TXD loaded and referenced, then `CSprite2d::SetTexture` with the slot current. Mirrors samp.dll
/// 0x100B2D10 / 0x100B2BD0.
pub fn load_sprite(text: &[u8]) -> u32 {
    let Some(src) = samp_ui::textdraw::sprite_source(text) else { return 0 };
    let mut slot_name = cstr(&src.slot);
    let mut slot = txd_find_slot(GPtr::from_raw(slot_name.as_mut_ptr()));
    if slot == -1 {
        let Some(path) = src.path() else { return 0 };
        slot = txd_add_slot(GPtr::from_raw(slot_name.as_mut_ptr()));
        if !file_exists(&path) {
            return 0;
        }
        let mut f = cstr(&path);
        txd_load(slot, GPtr::from_raw(f.as_mut_ptr()));
        txd_add_ref(slot);
    }
    let mut sprite: u32 = 0;
    let mut name = cstr(&src.texture);
    txd_push_current();
    txd_set_current(slot);
    sprite2d_set_texture(GPtr::from_raw(&mut sprite).cast(), GPtr::from_raw(name.as_mut_ptr()));
    txd_pop_current();
    sprite
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_and_text_conversions() {
        assert_eq!(crgba(0x8011_2233), 0x8033_2211);
        assert_eq!(cstr(b"ab"), b"ab\0");
        assert_eq!(std::mem::size_of::<Vertex>(), 0x1C);
    }
}
