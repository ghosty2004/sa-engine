//! WRITER-MVP-G: `CBrightLights::Render`. Ported from the 1.0 US bytes; gta-reversed
//! `BrightLights.cpp` as reading aid. x87 arithmetic follows the original's instruction order on the
//! `sa_math::x87::X87` model; the direct `call rand` runs under `orig::at_site` with its original
//! return address.
//!
//! `CBrightLights::aBrightLights` (0xC7CB58, 0x38 bytes each): +0 position, +0xC the first and +0x18
//! the second axis, +0x30 the camera distance, +0x34 the type (1..3 and 13..15 are drawn).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, jp, k, rd, x, xi};

orig! {
    /// `RwRenderStateSet(state, value)` (`RwGlobals` +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CBrightLights::RenderOutGeometryBuffer()` (0x722150)
    fn render_out_geometry_buffer() = cdecl 0x722150 => unported;
    /// `Common::LittleTest()` (0x541330)
    fn little_test() = cdecl 0x541330 => sa_core::common::little_test;
    /// `RwIm3DTransform(verts, count, ltm, flags)` (0x7EF450)
    fn rw_im3d_transform(verts: u32, count: u32, ltm: u32, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(primType, indices, numIndices)` (0x7EF550)
    fn rw_im3d_render_indexed_primitive(prim: u32, indices: u32, n: u32) -> u32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    fn rw_im3d_end() -> u32 = cdecl 0x7EF520 => unported;
}

/// `CBrightLights::NumBrightLights` and the array.
const NUM_LIGHTS: usize = 0xC7_C6FC;
const LIGHTS: usize = 0xC7_CB58;
/// `TempBufferVerticesStored`, `TempBufferIndicesStored`, `aTempBufferIndices`, `TempBufferRenderVertices`.
const TEMP_VERTICES_STORED: usize = 0xC4_B950;
const TEMP_INDICES_STORED: usize = 0xC4_B954;
const TEMP_BUFFER_INDICES: u32 = 0xC4_B958;
const TEMP_RENDER_VERTICES: u32 = 0xC4_D958;
/// The six box corners' factors along the two axes (`.data` f32 tables) and the twelve box indices
/// (`.data` u16).
const BOX_FACTOR_A: usize = 0x8D_5DA4;
const BOX_FACTOR_B: usize = 0x8D_5D8C;
const BOX_INDICES: usize = 0x8D_5DBC;
/// The circle radius (`.data` f32).
const CIRCLE_RADIUS: usize = 0x8D_5E48;

fn vert(n: u32) -> GPtr<u8> {
    GPtr::from_addr(TEMP_RENDER_VERTICES.wrapping_add(n.wrapping_mul(0x24)) as usize)
}

fn wf(p: GPtr<u8>, off: isize, v: X87) {
    crate::common::wr::<f32>(p, off, v.to_f32());
}

/// `CBrightLights::Render()` (0x7241C0). Nothing without lights. States (12,1) (8,1) (10,5) (11,6)
/// (1,0) (20,1), both temp-buffer counts zeroed; for each light (the count re-read per iteration):
/// flush (`RenderOutGeometryBuffer`) when more than 0xFD8 indices or 0x7D8 vertices are stored, then
/// `v = ftol((rand() & 0xFF) * 0.2)` (return address 0x72426A) sets the colour (r, g, b) by type
/// (1 and 13: (v, 255, v); 2 and 14: (255, 128, v); 3 and 15: (255, v, v); other types are not drawn
/// and keep the previous bytes, which start as uninitialised stack bytes in the original), the alpha `255` within 45 of the camera else
/// `ftol((1 - (d - 45) / 15) * 255)` (low byte). Types 1..3 add a six-vertex box `pos + A * a[j] +
/// B * b[j]` (tables 0x8D5DA4 / 0x8D5D8C, indices 0x8D5DBC + the vertex base) with alpha
/// `ftol(alpha * 0.5)`; types 13..15 an eight-vertex fan `pos + A * cos(k * pi/4) * r + B * sin(k *
/// pi/4) * r` (r = 0x8D5E48) with the full alpha. At the end the buffer is drawn (`LittleTest`,
/// `RwIm3DTransform(.., NULL, 1)`, indexed triangle list, end) when it holds indices and the counts
/// zeroed; then state (12,0) and `NumBrightLights = 0`.
#[hook(addr = 0x7241C0, conv = cdecl, name = "CBrightLights::Render", expect = "A1 FC C6 C7 00 83 EC 0C",
       route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "grove-walk",
       note = "fsin/fcos for the fans; rand at 0x724265; writes TempBufferRenderVertices / aTempBufferIndices")]
pub fn render() {
    if g::<u32>(NUM_LIGHTS).read() == 0 {
        return;
    }
    rw_render_state_set(0xC, 1);
    rw_render_state_set(8, 1);
    rw_render_state_set(0xA, 5);
    rw_render_state_set(0xB, 6);
    rw_render_state_set(1, 0);
    rw_render_state_set(0x14, 1);
    g::<u32>(TEMP_INDICES_STORED).write(0);
    g::<u32>(TEMP_VERTICES_STORED).write(0);
    if (g::<u32>(NUM_LIGHTS).read() as i32) > 0 {
        let (mut red, mut green, mut blue) = (0u8, 0u8, 0u8); // bl, [esp+0x12], [esp+0x13]
        let mut i: i32 = 0;
        loop {
            let l = GPtr::<u8>::from_addr(LIGHTS + 0x38 * i as usize);
            if g::<i32>(TEMP_INDICES_STORED).read() > 0xFD8 || g::<i32>(TEMP_VERTICES_STORED).read() > 0x7D8 {
                render_out_geometry_buffer();
            }
            let r = sa_core::orig::at_site(0x72_426A, sa_core::crt::rand) as u32 & 0xFF;
            let v = xi(r as i32).fmul_m(k(0x3E4C_CCCD)).to_f32(); // [esp+0x14]
            let ty = rd::<u8>(l, 0x34);
            match ty {
                1 | 13 => {
                    red = ftol32(x(v)) as u8;
                    green = 0xFF;
                    blue = red;
                }
                2 | 14 => {
                    red = 0xFF;
                    green = 0x80;
                    blue = ftol32(x(v)) as u8;
                }
                3 | 15 => {
                    red = 0xFF;
                    let c = ftol32(x(v)) as u8;
                    green = c;
                    blue = c;
                }
                _ => {}
            }
            let d = rd::<f32>(l, 0x30);
            let alpha: u8 = if !jp(x(d), x(45.0), 5) {
                0xFF
            } else {
                ftol32(x(d).fsub_m(45.0).fmul_m(k(0x3D88_8889)).fsubr_m(1.0).fmul_m(255.0)) as u8
            };
            let pos = |j: isize| rd::<f32>(l, j * 4);
            let ax = |j: isize| rd::<f32>(l, 0xC + j * 4);
            let bx = |j: isize| rd::<f32>(l, 0x18 + j * 4);
            match ty {
                1..=3 => {
                    let a2 = ftol32(xi(i32::from(alpha)).fmul_m(0.5)) as u8;
                    let colour = u32::from_be_bytes([a2, red, green, blue]);
                    let base = g::<u32>(TEMP_VERTICES_STORED).read();
                    for j in 0..6u32 {
                        let p = vert(base + j);
                        let fa = g::<f32>(BOX_FACTOR_A + 4 * j as usize).read();
                        let fb = g::<f32>(BOX_FACTOR_B + 4 * j as usize).read();
                        crate::common::wr::<u32>(p, 0x18, colour);
                        for c in 0..3isize {
                            wf(p, 4 * c, x(fa).fmul_m(ax(c)).fadd(x(fb).fmul_m(bx(c))).fadd_m(pos(c)));
                        }
                    }
                    let ni = g::<u32>(TEMP_INDICES_STORED).read();
                    for j in 0..12u32 {
                        let v = g::<u16>(BOX_INDICES + 2 * j as usize).read().wrapping_add(base as u16);
                        g::<u16>((TEMP_BUFFER_INDICES + 2 * (ni + j)) as usize).write(v);
                    }
                    g::<u32>(TEMP_INDICES_STORED).write(ni + 12);
                    g::<u32>(TEMP_VERTICES_STORED).write(base + 6);
                }
                13..=15 => {
                    let colour = u32::from_be_bytes([alpha, red, green, blue]);
                    let base = g::<u32>(TEMP_VERTICES_STORED).read();
                    let rad = g::<f32>(CIRCLE_RADIUS).read();
                    for kk in 0..8u32 {
                        let p = vert(base + kk);
                        crate::common::wr::<u32>(p, 0x18, colour);
                        let ang = xi(kk as i32).fmul_m(k(0x3F49_0FDB));
                        let s = ang.fsin().fmul_m(rad);
                        let c = ang.fcos().fmul_m(rad);
                        for j in 0..3isize {
                            wf(p, 4 * j, c.fmul_m(ax(j)).fadd(s.fmul_m(bx(j))).fadd_m(pos(j)));
                        }
                    }
                    let ni = g::<u32>(TEMP_INDICES_STORED).read();
                    for j in 0..6u32 {
                        let at = |n: u32| g::<u16>((TEMP_BUFFER_INDICES + 2 * (ni + 3 * j + n)) as usize);
                        at(0).write(base as u16);
                        at(1).write((j + base + 1) as u16);
                        at(2).write((j + base + 2) as u16);
                    }
                    g::<u32>(TEMP_INDICES_STORED).write(ni + 18);
                    g::<u32>(TEMP_VERTICES_STORED).write(base + 8);
                }
                _ => {}
            }
            i += 1;
            if i >= g::<i32>(NUM_LIGHTS).read() {
                break;
            }
        }
        if g::<u32>(TEMP_INDICES_STORED).read() != 0 {
            little_test();
            if !rw_im3d_transform(TEMP_RENDER_VERTICES, g::<u32>(TEMP_VERTICES_STORED).read(), 0, 1).is_null() {
                rw_im3d_render_indexed_primitive(3, TEMP_BUFFER_INDICES, g::<u32>(TEMP_INDICES_STORED).read());
                rw_im3d_end();
            }
            g::<u32>(TEMP_VERTICES_STORED).write(0);
            g::<u32>(TEMP_INDICES_STORED).write(0);
        }
    }
    rw_render_state_set(0xC, 0);
    g::<u32>(NUM_LIGHTS).write(0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_RENDER];
