//! `CPostEffects::HeatHazeFX(float intensity, bool alwaysOn)` (0x701780, batch `std12-render-2`): the heat-haze post
//! effect (copy of the frame into the haze raster, then 180 drifting jittered quads drawn from it, or one full-screen
//! quad). Unreversed in gta-reversed; ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::{ftol32, g, rd, x, xi};

orig! {
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwCameraClear(RwCamera*, RwRGBA*, int32)` (0x7EE340)
    fn rw_camera_clear(cam: GPtr<u8>, col: GPtr<u8>, mode: i32) -> GPtr<u8> = cdecl 0x7EE340 => unported;
    /// `CPostEffects::ImmediateModeRenderStatesStore()` (0x700CC0), `...Set()` (0x700D70), `...ReStore()` (0x700E00)
    fn rs_store() = cdecl 0x700CC0 => unported;
    fn rs_set() = cdecl 0x700D70 => unported;
    fn rs_restore() = cdecl 0x700E00 => unported;
    /// `CPostEffects::DrawQuad(x, y, w, h, r, g, b, a, raster)` (0x700EC0), floats as bits
    fn draw_quad(x0: u32, y0: u32, w: u32, h: u32, r: u32, gr: u32, b: u32, a: u32, raster: u32) = cdecl 0x700EC0 => unported;
    /// `Fx_c::Render(RwCamera*, bool)` (0x49E650; ECX = `g_fx` is ignored by the callee)
    fn fx_render(cam: u32, haze: u32) = stdcall 0x49E650 => unported;
    /// `CPostEffects::HeatHazeFXInit()` (0x701450)
    fn heat_haze_fx_init() = cdecl 0x701450 => unported;
    /// `RwCameraEndUpdate(RwCamera*)` (0x7EE180)
    fn rw_camera_end_update(cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EE180 => unported;
    /// `RwRasterPushContext(RwRaster*)` (0x7FB060), `RwRasterRenderFast(RwRaster*, int32, int32)` (0x7FAF50),
    /// `RwRasterPopContext()` (0x7FB110)
    fn rw_raster_push_context(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7FB060 => unported;
    fn rw_raster_render_fast(r: GPtr<u8>, x0: i32, y0: i32) -> GPtr<u8> = cdecl 0x7FAF50 => unported;
    fn rw_raster_pop_context() -> GPtr<u8> = cdecl 0x7FB110 => unported;
    /// `RsCameraBeginUpdate(RwCamera*)` (0x619450)
    fn rs_camera_begin_update(cam: GPtr<u8>) -> u32 = cdecl 0x619450 => unported;
    /// `RwIm2DRenderIndexedPrimitive(type, verts, numVerts, indices, numIndices)` (0x734EA0)
    fn im2d_render_indexed(ty: u32, v: u32, nv: u32, idx: u32, ni: u32) -> i32 = cdecl 0x734EA0 => unported;
}

/// `Scene.m_pRwCamera`, `RwEngineInstance`, the haze raster, the full-screen switch, the jitter.
const CAMERA: usize = 0xC1703C;
const ENGINE: usize = 0xC97B24;
const HAZE_RASTER: usize = 0xC402D8;
const FULL_SCREEN: usize = 0xC402BB;
const JITTER: usize = 0xC402C0;
/// The quad rectangle (x0, y0, x1, y1) and the particle arrays (x, y, speed; 180 each).
const RECT: usize = 0xC40304;
const PX: usize = 0xC3FE08;
const PY: usize = 0xC3FB38;
const SPEED: usize = 0xC3F868;
/// The Im2D vertex buffer (0x1C-byte vertices), the vertex count, the index count and the index buffer.
const VERTS: usize = 0xC5F958;
const NVERTS: usize = 0xC4B950;
const NIDX: usize = 0xC4B954;
const IDX: usize = 0xC4B958;

fn rs(s: u32, v: u32) {
    rw_render_state_set(s, v);
}

fn k(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn gi(a: usize) -> i32 {
    g::<i32>(a).read()
}

/// `(a - b) / 2` as `cdq; sub eax, edx; sar eax, 1` (truncating).
fn half(a: i32, b: i32) -> i32 {
    a.wrapping_sub(b) / 2
}

/// `_ftol((rand() & 0xFFFF) / 32768 * n)` with `rand` returning to `site`.
fn rand_mul(site: u32, n: i32) -> i32 {
    let r = at_site(site, sa_core::crt::rand);
    ftol32(xi(r & 0xFFFF).fmul_m(k(0x858B14)).fmul(xi(n)))
}

/// `1 / camera near plane` as stored into each vertex's rhw.
fn rhw() -> f32 {
    x(k(0x858624)).fdiv_m(rd::<f32>(g::<GPtr<u8>>(CAMERA).read(), 0x80)).to_f32()
}

fn near_z() -> u32 {
    rd::<u32>(g::<GPtr<u8>>(ENGINE).read(), 0x18)
}

/// Vertex `n`'s field at `off` (x 0, y 4, z 8, rhw 0xC, colour 0x10, u 0x14, v 0x18).
fn vtx(n: i32, off: usize) -> GPtr<u8> {
    GPtr::from_addr(VERTS + (n as u32).wrapping_mul(0x1C) as usize + off)
}

/// Six indices of two triangles `(b, b+2, b+1), (b+1, b+2, b+3)` at index `i`.
fn quad_indices(i: i32, b: i32) {
    for (j, v) in [b, b + 2, b + 1, b + 1, b + 2, b + 3].into_iter().enumerate() {
        g::<u16>(IDX + (i as usize + j) * 2).write(v as u16);
    }
}

/// `CPostEffects::HeatHazeFX(float intensity, bool alwaysOn)` (0x701780, cdecl).
///
/// `alwaysOn`: the camera is cleared to transparent black (the RwRGBA is built in the original's own `alwaysOn`
/// argument slot), the immediate-mode states stored and set, blend 5 / 2, on non-16-bit frame buffers stencil
/// states (0x15..0x1A), a full-screen `DrawQuad` (alpha 255), `Fx_c::Render(camera, true)` and the states restored;
/// otherwise the full-screen switch 0xC402BB is cleared. Intensity clamped to [0, 1] (in its argument slot).
/// `HeatHazeFXInit`, the frame copied into the haze raster (`RwRasterRenderFast` inside the raster's context between
/// `RwCameraEndUpdate` / `RsCameraBeginUpdate`), alpha `_ftol([0x8D50E8] * intensity)`. Full-screen switch: one quad
/// (vertex 2 and 3 keep the old rhw: the original stores vertex 0's and 1's twice; u / v not written), colour
/// `alpha << 24 | 0xFF0000`. Else for each of the 180 particles: the quad at the particle minus half the rectangle,
/// jittered by `±[0xC402C0]` (two `rand`), clamped to the raster (moving the texture position by half the rectangle),
/// u / v from the particle position over the raster size; the particle's y drops by `_ftol(timeStep * [0x858B8C] *
/// speed)` and below 0 it respawns at a random x, the bottom, and a random speed in `[0x8D50EC, 0x8D50F0)`. Then the
/// indexed triangle list with the haze raster and the states reset.
#[hook(addr = 0x701780, conv = cdecl, name = "CPostEffects::HeatHazeFX", expect = "83 EC 34 A1 3C 70 C1 00",
       unverified = "std12-render-2", route = "L2", hazards = "stack-residue,heap-writes", recs = "grove-walk",
       note = "writes its own argument slots (the clear colour, the clamped intensity)")]
pub fn heat_haze_fx(intensity: f32, always_on: u32) {
    let cam = g::<GPtr<u8>>(CAMERA).read();
    let not16 = rd::<i32>(rd::<GPtr<u8>>(cam, 0x60), 0x14) != 0x10;
    if always_on as u8 != 0 {
        let mut col = [0u8; 4];
        rw_camera_clear(cam, GPtr::from_raw(col.as_mut_ptr()), 2);
        rs_store();
        rs_set();
        rs(0xA, 5);
        rs(0xB, 2);
        if not16 {
            for (s, v) in [(0x15, 1), (0x16, 1), (0x17, 1), (0x18, 3), (0x1A, 0), (0x19, 8)] {
                rs(s, v);
            }
        }
        let w = xi(gi(0xC17044)).to_f32();
        let h = xi(gi(0xC17048)).to_f32();
        draw_quad(0, 0, w.to_bits(), h.to_bits(), 0, 0, 0, 0xFF, 0);
        if not16 {
            rs(0x1A, 1);
        }
        fx_render(g::<u32>(0xB6F97C).read(), 1);
        rs_restore();
    } else {
        g::<u8>(FULL_SCREEN).write(0);
    }
    let mut intensity = intensity;
    if !x(intensity).fcom(x(k(0x858B50))).test_parity(0x05) {
        intensity = 0.0;
    } else if !x(intensity).fcom(x(k(0x858624))).test_any(0x41) {
        intensity = 1.0;
    }
    heat_haze_fx_init();
    rs(2, 3);
    let cam = g::<GPtr<u8>>(CAMERA).read();
    rw_camera_end_update(cam);
    rw_raster_push_context(g::<GPtr<u8>>(HAZE_RASTER).read());
    rw_raster_render_fast(rd::<GPtr<u8>>(g::<GPtr<u8>>(CAMERA).read(), 0x60), 0, 0);
    rw_raster_pop_context();
    rs_camera_begin_update(g::<GPtr<u8>>(CAMERA).read());
    g::<i32>(NVERTS).write(0);
    g::<i32>(NIDX).write(0);
    rs(0xE, 0);
    rs(0xC, 1);
    let alpha = ftol32(xi(gi(0x8D50E8)).fmul_m(intensity));
    g::<i32>(NVERTS).write(0);
    if not16 {
        rs(0x18, 1);
        rs(0x1A, 1);
        rs(0x19, 3);
    }
    rs(6, 0);
    rs(8, 0);
    rs(1, g::<u32>(HAZE_RASTER).read());
    rs(9, 2);
    rs(0xA, 5);
    rs(0xB, 6);

    if g::<u8>(FULL_SCREEN).read() != 0 {
        full_screen_quad(alpha);
    } else {
        particles(alpha);
    }
    rs(1, g::<u32>(HAZE_RASTER).read());
    let n = g::<i32>(NVERTS).read();
    if n != 0 {
        im2d_render_indexed(3, VERTS as u32, n as u32, IDX as u32, g::<u32>(NIDX).read());
    }
    rs(0xE, 0);
    g::<i32>(NVERTS).write(0);
    for (s, v) in [(6, 1), (8, 1), (1, 0), (0xC, 0), (0xA, 5), (0xB, 6)] {
        rs(s, v);
    }
    if not16 {
        rs(0x15, 0);
    }
}

/// The full-screen quad of [`heat_haze_fx`] (0x701E84).
fn full_screen_quad(alpha: i32) {
    let n = g::<i32>(NVERTS).read();
    let r = g::<GPtr<u8>>(HAZE_RASTER).read();
    let (rw, rh) = (xi(rd::<i32>(r, 0xC)).to_f32(), xi(rd::<i32>(r, 0x10)).to_f32());
    vtx(n, 0).cast::<u32>().write(0);
    vtx(n, 4).cast::<u32>().write(0);
    vtx(n, 8).cast::<u32>().write(near_z());
    vtx(n, 0xC).cast::<f32>().write(rhw());
    vtx(n + 1, 0).cast::<f32>().write(rw);
    vtx(n + 1, 4).cast::<u32>().write(0);
    vtx(n + 1, 8).cast::<u32>().write(near_z());
    vtx(n + 1, 0xC).cast::<f32>().write(rhw());
    vtx(n + 2, 0).cast::<u32>().write(0);
    vtx(n + 2, 4).cast::<f32>().write(rh);
    vtx(n + 2, 8).cast::<u32>().write(near_z());
    vtx(n, 0xC).cast::<f32>().write(rhw());
    vtx(n + 3, 0).cast::<f32>().write(rw);
    vtx(n + 3, 4).cast::<f32>().write(rh);
    vtx(n + 3, 8).cast::<u32>().write(near_z());
    vtx(n + 1, 0xC).cast::<f32>().write(rhw());
    let col = ((alpha as u32) << 24) | 0xFF_0000;
    for i in 0..4 {
        vtx(n + i, 0x10).cast::<u32>().write(col);
    }
    let ni = g::<i32>(NIDX).read();
    quad_indices(ni, n);
    g::<i32>(NVERTS).write(n + 4);
    g::<i32>(NIDX).write(ni + 6);
}

/// The 180 particle quads of [`heat_haze_fx`] (0x7019FD..0x701E79).
fn particles(alpha: i32) {
    let (x0, y0, x1, y1) = (gi(RECT), gi(RECT + 4), gi(RECT + 8), gi(RECT + 0xC));
    let hw = half(x1, x0);
    let hh = half(y1, y0);
    let col = ((alpha as u32) << 24) | 0xFF_FFFF;
    for i in 0..180usize {
        let mut px = gi(PX + 4 * i);
        let mut py = gi(PY + 4 * i);
        let mut a = px.wrapping_sub(hw);
        let mut c = py.wrapping_sub(hh);
        let j = gi(JITTER);
        if j > 0 {
            a = a.wrapping_add(rand_mul(0x701AB1, j.wrapping_sub(j.wrapping_neg())).wrapping_sub(j));
            let j = gi(JITTER);
            c = c.wrapping_add(rand_mul(0x701AEF, j.wrapping_sub(j.wrapping_neg())).wrapping_sub(j));
        }
        let r = g::<GPtr<u8>>(HAZE_RASTER).read();
        if a < 0 {
            px = px.wrapping_add(hw);
            a = 0;
        }
        let rw = rd::<i32>(r, 0xC);
        let lim = rw.wrapping_sub(x1);
        if a > lim {
            a = lim;
            px = px.wrapping_sub(hw);
        }
        if c < 0 {
            py = py.wrapping_add(hh);
            c = 0;
        }
        let rh = rd::<i32>(r, 0x10);
        let lim = rh.wrapping_sub(y1);
        if c > lim {
            c = lim;
            py = py.wrapping_sub(hh);
        }
        let n = g::<i32>(NVERTS).read();
        let pyf = xi(py).to_f32();
        let v0 = xi(rh).fdivr_m(pyf);
        let pxf = xi(px).to_f32();
        vtx(n, 0x14).cast::<f32>().write(xi(rw).fdivr_m(pxf).to_f32());
        vtx(n, 0x18).cast::<f32>().write(v0.to_f32());
        vtx(n, 0).cast::<f32>().write(xi(a).to_f32());
        vtx(n, 4).cast::<f32>().write(xi(c).to_f32());
        vtx(n, 8).cast::<u32>().write(near_z());
        vtx(n, 0xC).cast::<f32>().write(rhw());
        let ux = px.wrapping_add(x0);
        let ax = a.wrapping_add(x1);
        let v1 = xi(rd::<i32>(r, 0x10)).fdivr_m(pyf);
        let uxf = xi(ux).to_f32();
        vtx(n + 1, 0x14).cast::<f32>().write(xi(rd::<i32>(r, 0xC)).fdivr_m(uxf).to_f32());
        vtx(n + 1, 0x18).cast::<f32>().write(v1.to_f32());
        let axf = xi(ax).to_f32();
        vtx(n + 1, 0).cast::<f32>().write(axf);
        vtx(n + 1, 4).cast::<f32>().write(xi(c).to_f32());
        vtx(n + 1, 8).cast::<u32>().write(near_z());
        vtx(n + 1, 0xC).cast::<f32>().write(rhw());
        let vy = py.wrapping_add(y0);
        let cy = c.wrapping_add(y1);
        let vyf = xi(vy).to_f32();
        let v2 = xi(rd::<i32>(r, 0x10)).fdivr_m(vyf);
        vtx(n + 2, 0x14).cast::<f32>().write(xi(rd::<i32>(r, 0xC)).fdivr_m(pxf).to_f32());
        vtx(n + 2, 0x18).cast::<f32>().write(v2.to_f32());
        vtx(n + 2, 0).cast::<f32>().write(xi(a).to_f32());
        vtx(n + 2, 4).cast::<f32>().write(xi(cy).to_f32());
        vtx(n + 2, 8).cast::<u32>().write(near_z());
        vtx(n + 2, 0xC).cast::<f32>().write(rhw());
        let v3 = xi(rd::<i32>(r, 0x10)).fdivr_m(vyf);
        vtx(n + 3, 0x14).cast::<f32>().write(xi(rd::<i32>(r, 0xC)).fdivr_m(uxf).to_f32());
        vtx(n + 3, 0x18).cast::<f32>().write(v3.to_f32());
        vtx(n + 3, 0).cast::<f32>().write(axf);
        vtx(n + 3, 4).cast::<f32>().write(xi(cy).to_f32());
        vtx(n + 3, 8).cast::<u32>().write(near_z());
        vtx(n + 3, 0xC).cast::<f32>().write(rhw());
        let ni = g::<i32>(NIDX).read();
        quad_indices(ni, n);
        g::<i32>(NIDX).write(ni + 6);
        for q in 0..4 {
            vtx(n + q, 0x10).cast::<u32>().write(col);
        }
        g::<i32>(NVERTS).write(n + 4);
        let drop = ftol32(x(g::<f32>(0xB7CB5C).read()).fmul_m(k(0x858B8C)).fmul(xi(gi(SPEED + 4 * i))));
        let ny = gi(PY + 4 * i).wrapping_sub(drop);
        g::<i32>(PY + 4 * i).write(ny);
        if ny < 0 {
            let r = g::<GPtr<u8>>(HAZE_RASTER).read();
            g::<i32>(PX + 4 * i).write(rand_mul(0x701DE6, rd::<i32>(r, 0xC).wrapping_sub(x0)));
            let r = g::<GPtr<u8>>(HAZE_RASTER).read();
            g::<i32>(PY + 4 * i).write(rd::<i32>(r, 0x10).wrapping_sub(y0));
            let (hi, lo) = (gi(0x8D50F0), gi(0x8D50EC));
            g::<i32>(SPEED + 4 * i).write(rand_mul(0x701E2E, hi.wrapping_sub(lo)).wrapping_add(lo));
        }
    }
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[HOOK_HEAT_HAZE_FX];
