//! `CPostEffects::Radiosity`: the front-buffer raster repeatedly copied into its own top-left
//! quarter (each copy half the size of the last), then the blurred result added over the screen.
//!
//! Every pass appends two Im2D vertices of a triangle strip to the temporary buffer: a corner at the
//! origin and the opposite corner, the texture reaching the previous copy's size.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, x, xi};
use sa_core::bindings::rw_im2d_render_primitive;

orig! {
    /// `RwRenderStateSet(state, value)` (`RwGlobals` +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
}

const CAMERA: usize = 0xC1_703C;
const FRONT_BUFFER: usize = 0xC4_02D8;
const DEBUG: usize = 0xC4_02CD;
const LINEAR_FILTER: usize = 0x8D_510A;
const STRIP_COPY_MODE: usize = 0x8D_510B;
const FILTER_U_CORRECTION: usize = 0x8D_511C;
const FILTER_V_CORRECTION: usize = 0x8D_5120;
const PIXELS_X: usize = 0xC4_0314;
const PIXELS_Y: usize = 0xC4_0318;
const SCREEN_WIDTH: usize = 0xC1_7044;
const SCREEN_HEIGHT: usize = 0xC1_7048;
const ONE: usize = 0x85_8624;
/// The Im2D vertex buffer (0x1C-byte vertices: x, y, z, rhw, colour, u, v) and its vertex count.
const VERTS: usize = 0xC5_F958;
const NVERTS: usize = 0xC4_B950;
const RW_PRIM_TRI_STRIP: u32 = 4;

fn rs(state: u32, value: u32) {
    rw_render_state_set(state, value);
}

fn gi(a: usize) -> i32 {
    g::<i32>(a).read()
}

/// Field `off` of vertex `n` (x 0, y 4, z 8, rhw 0xC, colour 0x10, u 0x14, v 0x18).
fn vtx(n: i32, off: usize) -> GPtr<u32> {
    GPtr::from_addr(VERTS + (n as u32).wrapping_mul(0x1C) as usize + off)
}

fn put(n: i32, off: usize, v: f32) {
    vtx(n, off).write(v.to_bits());
}

fn rhw() -> f32 {
    x(g::<f32>(ONE).read()).fdiv_m(rd::<f32>(g::<GPtr<u8>>(CAMERA).read(), 0x80)).to_f32()
}

/// `a / b` of two integers loaded with `fild`.
fn ratio(a: i32, b: i32) -> f32 {
    xi(a).fdiv(xi(b)).to_f32()
}

/// One strip pass at the vertex count: the first vertex at the origin with texture (u0, v0), the
/// second at (x1, y1) with texture (u1, v1), both coloured `colour`.
fn add_pass(u0: f32, v0: f32, u1: f32, v1: f32, x1: f32, y1: f32, colour: u32) {
    let n = gi(NVERTS);
    put(n, 0x14, u0);
    put(n, 0x18, v0);
    for off in [0, 4, 8] {
        vtx(n, off).write(0);
    }
    put(n, 0xC, rhw());
    put(n, 0x30, u1);
    put(n, 0x34, v1);
    put(n, 0x1C, x1);
    put(n, 0x20, y1);
    vtx(n, 0x24).write(0);
    put(n, 0x28, rhw());
    vtx(n, 0x10).write(colour);
    vtx(n, 0x2C).write(colour);
    g::<i32>(NVERTS).write(n + 2);
}

fn flush() {
    let n = gi(NVERTS);
    if n > 2 {
        rw_im2d_render_primitive(RW_PRIM_TRI_STRIP, GPtr::from_addr(VERTS), n);
    }
    g::<i32>(NVERTS).write(0);
}

/// `CPostEffects::Radiosity(intensityLimit, filterPasses, renderPasses, intensity)` (0x702080).
///
/// The first filter pass maps the whole front buffer onto a copy half the radiosity size (signed
/// divisions), every further pass the previous copy onto one half its size, the texture shifted by
/// the filter U / V corrections, white. A last pair, whose texture coordinates stay as they were, is
/// coloured (limit, limit, limit, 0x80). Each render pass then draws the last copy over the screen
/// with alpha `intensity` (white in debug mode unless the strip copy mode is on).
#[hook(addr = 0x702080, conv = cdecl, name = "CPostEffects::Radiosity", expect = "83 EC 14 A1 18 03 C4 00",
       unverified = "frontier2-c36", route = "L2", hazards = "heap-writes")]
pub fn post_radiosity(limit: u32, filter_passes: u32, render_passes: u32, intensity: u32) {
    let (filter_passes, render_passes) = (filter_passes as i32, render_passes as i32);
    let mut w = gi(PIXELS_X);
    let mut h = gi(PIXELS_Y);
    rs(9, 1);
    rs(0xE, 0);
    rs(6, 1);
    rs(8, 0);
    rs(1, g::<u32>(FRONT_BUFFER).read());
    rs(0xC, 0);
    rs(0xA, 5);
    rs(0xB, 6);
    g::<i32>(NVERTS).write(0);
    for off in [8, 4, 0, 0x18, 0x14] {
        vtx(0, off).write(0);
    }
    put(0, 0xC, rhw());
    vtx(0, 0x24).write(0);
    put(0, 0x34, 1.0);
    put(0, 0x30, 1.0);
    let (sx, sy) = if g::<u8>(STRIP_COPY_MODE).read() == 0 {
        (w, h)
    } else {
        (gi(SCREEN_WIDTH), gi(SCREEN_HEIGHT))
    };
    put(0, 0x1C, sx as f32);
    put(0, 0x20, sy as f32);
    g::<i32>(NVERTS).write(0);
    put(0, 0x28, rhw());
    rs(6, 1);
    if filter_passes > 0 {
        rs(9, 2);
        let raster = g::<GPtr<u8>>(FRONT_BUFFER).read();
        for _ in 0..filter_passes {
            let (pw, ph) = (w, h);
            w /= 2;
            h /= 2;
            add_pass(
                ratio(gi(FILTER_U_CORRECTION), rd(raster, 0xC)),
                ratio(gi(FILTER_V_CORRECTION), rd(raster, 0x10)),
                ratio(pw, rd(raster, 0xC)),
                ratio(ph, rd(raster, 0x10)),
                (w + 1) as f32,
                (h + 1) as f32,
                u32::MAX,
            );
        }
    }
    rs(0xA, 5);
    rs(0xB, 6);
    let n = gi(NVERTS);
    for off in [0, 4, 8] {
        vtx(n, off).write(0);
    }
    put(n, 0xC, rhw());
    put(n, 0x1C, (w + 1) as f32);
    put(n, 0x20, (h + 1) as f32);
    vtx(n, 0x24).write(0);
    g::<i32>(NVERTS).write(n + 2);
    put(n, 0x28, rhw());
    let subtract = ((limit | 0xFFFF_8000) << 8 | limit) << 8 | limit;
    vtx(n, 0x10).write(subtract);
    vtx(n, 0x2C).write(subtract);
    if render_passes > 0 {
        flush();
        rs(9, if g::<u8>(LINEAR_FILTER).read() != 0 { 2 } else { 3 });
        let (screen_w, screen_h) = (gi(SCREEN_WIDTH), gi(SCREEN_HEIGHT));
        let colour = if g::<u8>(STRIP_COPY_MODE).read() == 0 && g::<u8>(DEBUG).read() != 0 {
            u32::MAX
        } else {
            intensity << 24
        };
        let raster = g::<GPtr<u8>>(FRONT_BUFFER).read();
        for _ in 0..render_passes {
            add_pass(0.0, 0.0, ratio(w, rd(raster, 0xC)), ratio(h, rd(raster, 0x10)), screen_w as f32, screen_h as f32, colour);
        }
    }
    flush();
    rs(0xE, 0);
    rs(6, 1);
    rs(8, 1);
    rs(1, 0);
    rs(0xC, 0);
    rs(0xA, 5);
    rs(0xB, 6);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_POST_RADIOSITY];
