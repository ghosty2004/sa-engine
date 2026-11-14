//! The GUI layer (dialogs, buttons, list boxes, scrollbars, edit boxes), drawn the way samp's statically
//! linked DXUT does so that the Direct3D call stream matches the original's:
//!
//! - a dialog render is `sprite.Begin(DONOTSAVESTATE)`, the fixed render/texture-stage
//!   states, the background fan (`DrawPrimitiveUP`, FVF 0x144, also when transparent),
//!   the texture states, `SetTexture(sampgui.png)`, the controls, `sprite.End`;
//! - an element sprite is `SetTransform(scale)` + `Draw(src, NULL, pos / scale, colour)`
//!   (skipped when the colour's alpha is 0);
//! - text is `SetTransform(identity)` + the colour-code wrapper's `DrawTextA` (or the
//!   font's `DrawTextW` for edit boxes);
//! - solid rectangles (caret, selection) flush the sprite and draw a fan with the
//!   texture stage set to "diffuse only", restoring the vertex declaration.
//!
//! These are the D3D9 backend's primitives; the controls themselves (caption, button, scrollbar, edit box, list
//! box) are drawn once for every backend by `crate::ui::render`, from samp-ui's skin rects and geometry
//! (`samp_ui::dxut`). Spec: docs/re/subsystems/ui-rendering.md, "4. GUI layer (DXUT-derived)".

use std::ffi::c_void;

use samp_ui::colour;

use super::d3d::{self, D3dx, Font, IDENTITY, Rect, Sprite, devcall};

const FVF_SCREEN: u32 = 0x144;
const FVF_COLOUR: u32 = 0x44;
const TRIANGLE_FAN: u32 = 6;

/// `DXUT_SCREEN_VERTEX`.
#[repr(C)]
#[derive(Clone, Copy)]
struct ScreenVertex {
    x: f32,
    y: f32,
    z: f32,
    rhw: f32,
    colour: u32,
    u: f32,
    v: f32,
}

/// Pre-transformed coloured vertex (FVF 0x44), used by the nametag bars.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ColourVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rhw: f32,
    pub colour: u32,
}

/// The render/texture-stage states DXUT sets at the start of a dialog render; the
/// nametag bar pass sets the same list.
pub fn ui_states() {
    devcall::set_render_state(27, 1); // ALPHABLENDENABLE
    devcall::set_render_state(19, 5); // SRCBLEND SRCALPHA
    devcall::set_render_state(20, 6); // DESTBLEND INVSRCALPHA
    devcall::set_render_state(15, 0); // ALPHATESTENABLE
    devcall::set_tss(0, 1, 3); // COLOROP SELECTARG2
    devcall::set_tss(0, 3, 0); // COLORARG2 DIFFUSE
    devcall::set_tss(0, 4, 2); // ALPHAOP SELECTARG1
    devcall::set_tss(0, 5, 0); // ALPHAARG1 DIFFUSE
    devcall::set_vertex_shader_null();
    devcall::set_pixel_shader_null();
    devcall::set_render_state(7, 0); // ZENABLE
}

/// The GUI resource manager's `D3DSBT_ALL` state block, created right after the GUI sprite's
/// `OnResetDevice`. Every dialog render captures it first and applies it last, so the states the
/// dialog sets (ZENABLE 0, blending, stage 0, shaders, FVF, texture, the sprite's transforms)
/// do not leak into GTA's next frame (RenderWare caches its render states).
static RM_STATE_BLOCK: std::sync::atomic::AtomicPtr<c_void> = std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

pub fn set_rm_state_block(sb: *mut c_void) {
    RM_STATE_BLOCK.store(sb, std::sync::atomic::Ordering::SeqCst);
}

pub fn rm_state_block() -> *mut c_void {
    RM_STATE_BLOCK.load(std::sync::atomic::Ordering::SeqCst)
}

/// Start of `CDXUTDialog::OnRender` for a dialog at `rect` with a uniform background.
pub fn dialog_begin(sprite: &Sprite, rect: Rect, background: u32, gui_tex: *mut c_void) {
    d3d::state_block_capture(rm_state_block());
    sprite.begin(d3d::sprite::DONOTSAVESTATE);
    ui_states();
    let (l, t, r, b) = (rect.left as f32, rect.top as f32, rect.right as f32, rect.bottom as f32);
    let v = [
        ScreenVertex { x: l, y: t, z: 0.5, rhw: 1.0, colour: background, u: 0.0, v: 0.5 },
        ScreenVertex { x: r, y: t, z: 0.5, rhw: 1.0, colour: background, u: 1.0, v: 0.5 },
        ScreenVertex { x: r, y: b, z: 0.5, rhw: 1.0, colour: background, u: 1.0, v: 1.0 },
        ScreenVertex { x: l, y: b, z: 0.5, rhw: 1.0, colour: background, u: 0.0, v: 1.0 },
    ];
    devcall::set_fvf(FVF_SCREEN);
    devcall::draw_primitive_up(TRIANGLE_FAN, 2, v.as_ptr().cast(), std::mem::size_of::<ScreenVertex>() as u32);
    devcall::set_tss(0, 1, 4); // COLOROP MODULATE
    devcall::set_tss(0, 2, 2); // COLORARG1 TEXTURE
    devcall::set_tss(0, 3, 0); // COLORARG2 DIFFUSE
    devcall::set_tss(0, 4, 4); // ALPHAOP MODULATE
    devcall::set_tss(0, 5, 2); // ALPHAARG1 TEXTURE
    devcall::set_tss(0, 6, 0); // ALPHAARG2 DIFFUSE
    devcall::set_sampler_state(0, 6, 2); // MINFILTER LINEAR
    devcall::set_texture(0, gui_tex);
}

pub fn dialog_end(sprite: &Sprite) {
    sprite.end();
    d3d::state_block_apply(rm_state_block());
}

fn scale_matrix(sx: f32, sy: f32) -> [f32; 16] {
    let mut m = IDENTITY;
    m[0] = sx;
    m[5] = sy;
    m
}

/// `CDXUTDialog::DrawSprite`: `src` of `tex` scaled into the screen rect `dst`.
pub fn draw_sprite(sprite: &Sprite, tex: *mut c_void, src: Rect, dst: Rect, colour: u32) {
    if colour >> 24 == 0 {
        return;
    }
    let sx = (dst.right - dst.left) as f32 / (src.right - src.left) as f32;
    let sy = (dst.bottom - dst.top) as f32 / (src.bottom - src.top) as f32;
    sprite.set_transform(&scale_matrix(sx, sy));
    let pos = [dst.left as f32 / sx, dst.top as f32 / sy, 0.0];
    sprite.draw(tex, Some(&src), None, Some(&pos), colour);
}

/// `CDXUTDialog::DrawText` through the colour-code wrapper (GUI fonts are wrapper fonts:
/// `{RRGGBB}` recolours glyphs; the stripped text is drawn).
pub fn draw_text(sprite: &Sprite, font: &Font, text: &[u8], rect: Rect, format: u32, colour: u32) {
    if colour >> 24 == 0 {
        return;
    }
    sprite.set_transform(&IDENTITY);
    let stripped = colour::strip_codes(text);
    let glyphs: Vec<u32> = colour::glyph_colours(text, colour).iter().map(|g| g.colour).collect();
    let mut r = rect;
    d3d::draw_glyph_coloured(font, sprite, &stripped, &mut r, format, colour, glyphs);
}

/// `CDXUTDialog::DrawText` of wide text with the real sprite (edit boxes).
pub fn draw_text_w(sprite: &Sprite, font: &Font, text: &[u16], count: i32, rect: Rect, format: u32, colour: u32) {
    if colour >> 24 == 0 {
        return;
    }
    sprite.set_transform(&IDENTITY);
    let mut r = rect;
    font.draw_w(sprite.ptr, text, count, &mut r, format, colour);
}

/// `CDXUTDialog::DrawRect`.
pub fn draw_rect(sprite: &Sprite, rect: Rect, colour: u32) {
    let (l, t, r, b) =
        (rect.left as f32 - 0.5, rect.top as f32 - 0.5, rect.right as f32 - 0.5, rect.bottom as f32 - 0.5);
    let v = [
        ScreenVertex { x: l, y: t, z: 0.5, rhw: 1.0, colour, u: 0.0, v: 0.0 },
        ScreenVertex { x: r, y: t, z: 0.5, rhw: 1.0, colour, u: 0.0, v: 0.0 },
        ScreenVertex { x: r, y: b, z: 0.5, rhw: 1.0, colour, u: 0.0, v: 0.0 },
        ScreenVertex { x: l, y: b, z: 0.5, rhw: 1.0, colour, u: 0.0, v: 0.0 },
    ];
    sprite.flush();
    let decl = devcall::get_vertex_declaration();
    devcall::set_fvf(FVF_SCREEN);
    devcall::set_tss(0, 1, 3);
    devcall::set_tss(0, 4, 3);
    devcall::draw_primitive_up(TRIANGLE_FAN, 2, v.as_ptr().cast(), std::mem::size_of::<ScreenVertex>() as u32);
    devcall::set_tss(0, 1, 4);
    devcall::set_tss(0, 4, 4);
    devcall::set_vertex_declaration(decl);
    // SAFETY: GetVertexDeclaration AddRef'd it.
    unsafe { d3d::release(decl) };
}

/// A coloured fan (FVF 0x44 was set by the caller), quad corners in the nametag order
/// (x0,y0), (x0,y1), (x1,y1), (x1,y0).
pub fn colour_quad(x0: f32, y0: f32, x1: f32, y1: f32, colour: u32) {
    let v = [
        ColourVertex { x: x0, y: y0, z: 0.0, rhw: 1.0, colour },
        ColourVertex { x: x0, y: y1, z: 0.0, rhw: 1.0, colour },
        ColourVertex { x: x1, y: y1, z: 0.0, rhw: 1.0, colour },
        ColourVertex { x: x1, y: y0, z: 0.0, rhw: 1.0, colour },
    ];
    devcall::draw_primitive_up(TRIANGLE_FAN, 2, v.as_ptr().cast(), std::mem::size_of::<ColourVertex>() as u32);
}

pub fn set_colour_fvf() {
    devcall::set_fvf(FVF_COLOUR);
}

/// Loads the GUI skin texture and the D3DX objects the GUI needs.
pub fn load_skin(d3dx: &D3dx) -> *mut c_void {
    match d3d::load_texture(d3dx, b"sampgui.png") {
        Ok(t) => t,
        Err(e) => {
            crate::log!("ui: {e}; GUI controls are not drawn");
            std::ptr::null_mut()
        }
    }
}
