//! WRITER-MVP-G: `CMirrors::RenderMirrorBuffer`. Ported from the 1.0 US bytes; gta-reversed
//! `Mirrors.cpp` as reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{THE_CAMERA, g, jp, rd, x, xi};

orig! {
    /// `DefinedState()` (0x734650)
    fn defined_state() = cdecl 0x734650 => unported;
    /// `RwRenderStateSet(state, value)` (`RwGlobals` +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwIm2DRenderIndexedPrimitive(prim, verts, numVerts, indices, numIndices)` through the thunk
    /// 0x734EA0 (`RwEngineInstance` +0x34, protection-decoded)
    fn rw_im2d_render_indexed_primitive(prim: u32, verts: u32, nv: u32, indices: u32, ni: u32) -> u32 = cdecl 0x734EA0 => unported;
    /// `RwIm3DTransform(verts, count, ltm, flags)` (0x7EF450)
    fn rw_im3d_transform(verts: u32, count: u32, ltm: u32, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(primType, indices, numIndices)` (0x7EF550)
    fn rw_im3d_render_indexed_primitive(prim: u32, indices: u32, n: u32) -> u32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    fn rw_im3d_end() -> u32 = cdecl 0x7EF520 => unported;
}

/// `CMirrors::pBuffer`, its raster dword, `CMirrors::TypeOfMirror`-style flags byte and the
/// `bRenderingReflection` byte.
const MIRROR_BUFFER: usize = 0xC7_C724;
const MIRROR_RASTER: usize = 0xC7_C71C;
const MIRROR_FLAGS: usize = 0xC7_C618;
const MIRROR_FLAG2: usize = 0xC7_C72A;
/// The four screen vertices (RwIm2DVertex, 0x1C bytes).
const SCREEN_VERTS: u32 = 0xC7_C5A8;
/// The six quad indices (u16) and the two mirror quads' corner positions (4 x CVector each, `.data`).
const QUAD_INDICES: u32 = 0x8D_5E38;
const MIRROR_QUADS: u32 = 0x8D_5DD8;
/// `TempBufferVerticesStored`, `TempBufferIndicesStored`, `aTempBufferIndices`, `TempBufferRenderVertices`.
const TEMP_VERTICES_STORED: usize = 0xC4_B950;
const TEMP_INDICES_STORED: usize = 0xC4_B954;
const TEMP_BUFFER_INDICES: u32 = 0xC4_B958;
const TEMP_RENDER_VERTICES: u32 = 0xC4_D958;
/// `Scene.camera`.
const SCENE_CAMERA: usize = 0xC1_703C;
const CAM_MATRIX: usize = 0xB6_F03C;

fn gw(addr: u32, v: u32) {
    g::<u32>(addr as usize).write(v)
}

/// Copies the six quad indices into `aTempBufferIndices`, sets the four vertex colours to white and
/// the UVs (0,0) (1,0) (1,1) (0,1), the counts 6 / 4, and draws the temp buffer as an indexed
/// triangle list (`RwIm3DTransform(.., 4, NULL, 1)`).
fn draw_temp_quad() {
    for i in 0..6u32 {
        g::<u16>((TEMP_BUFFER_INDICES + 2 * i) as usize).write(g::<u16>((QUAD_INDICES + 2 * i) as usize).read());
    }
    for (i, (u, v)) in [(0u32, 0u32), (0x3F80_0000, 0), (0x3F80_0000, 0x3F80_0000), (0, 0x3F80_0000)].iter().enumerate() {
        let b = TEMP_RENDER_VERTICES + 0x24 * i as u32;
        gw(b + 0x18, 0xFFFF_FFFF);
        gw(b + 0x1C, *u);
        gw(b + 0x20, *v);
    }
    g::<u32>(TEMP_INDICES_STORED).write(6);
    g::<u32>(TEMP_VERTICES_STORED).write(4);
    if !rw_im3d_transform(TEMP_RENDER_VERTICES, 4, 0, 1).is_null() {
        rw_im3d_render_indexed_primitive(3, TEMP_BUFFER_INDICES, g::<u32>(TEMP_INDICES_STORED).read());
        rw_im3d_end();
    }
}

/// `CMirrors::RenderMirrorBuffer()` (0x726090). Nothing without `CMirrors::pBuffer`. Otherwise:
/// `DefinedState`, states (9,2) (14,0) (6,0) (8,0), the mirror raster (1), (12,0) (10,2) (11,1).
/// Without flag bit 1 (0xC7C618) and 0xC7C72A: the mirror buffer as a full-screen 2D quad (the
/// camera frame buffer's width/height, z = `RwGlobals` near screen z (+0x18), rhw =
/// `1 / Scene.camera->farPlane`, white, U mirrored) through `RwIm2DRenderIndexedPrimitive`. With
/// either: states (8,1) (6,1) (12,0) (14,1) and the two world quads of 0x8D5DD8 (3D, temp buffer).
/// Then, within 50 of (216, -42, 1003) (camera position; `sqrt((dz^2 + dy^2) + dx^2)`), states (8,1)
/// (12,0) (10,1) (11,2) and the fixed quad x = 216, y -45..-39, z 1000..1006. Finally states (14,0)
/// (6,1) (8,1) (1,0) (12,0) (10,5) (11,6).
#[hook(addr = 0x726090, conv = cdecl, name = "CMirrors::RenderMirrorBuffer", expect = "A1 24 C7 C7 00",
       unverified = "mvpg-render-1", route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "grove-walk",
       note = "fsqrt for the camera distance; writes the 2D vertices 0xC7C5A8.., TempBufferRenderVertices and aTempBufferIndices")]
pub fn render_mirror_buffer() {
    if g::<u32>(MIRROR_BUFFER).read() == 0 {
        return;
    }
    let fb = rd::<GPtr<u8>>(g::<GPtr<u8>>(SCENE_CAMERA).read(), 0x60);
    let width = xi(rd::<i32>(fb, 0xC)).to_f32(); // [esp+8]
    let height = xi(rd::<i32>(fb, 0x10)).to_f32(); // [esp+4]
    defined_state();
    rw_render_state_set(9, 2);
    rw_render_state_set(0xE, 0);
    rw_render_state_set(6, 0);
    rw_render_state_set(8, 0);
    rw_render_state_set(1, g::<u32>(MIRROR_RASTER).read());
    rw_render_state_set(0xC, 0);
    rw_render_state_set(0xA, 2);
    rw_render_state_set(0xB, 1);
    if g::<u8>(MIRROR_FLAGS).read() & 2 == 0 && g::<u8>(MIRROR_FLAG2).read() == 0 {
        let rw = GPtr::<u8>::from_addr(g::<u32>(0xC9_7B24).read() as usize);
        let rhw = || x(1.0).fdiv_m(rd::<f32>(g::<GPtr<u8>>(SCENE_CAMERA).read(), 0x80)).to_f32().to_bits();
        let (w, h) = (width.to_bits(), height.to_bits());
        let one = 0x3F80_0000;
        // (x, y, u, v) per vertex
        for (i, (vx, vy, u, v)) in [(0, 0, one, 0), (0, h, one, one), (w, h, 0, one), (w, 0, 0, 0)].iter().enumerate() {
            let b = SCREEN_VERTS + 0x1C * i as u32;
            gw(b, *vx);
            gw(b + 4, *vy);
            gw(b + 8, rd::<u32>(rw, 0x18));
            gw(b + 0xC, rhw());
            gw(b + 0x10, 0xFFFF_FFFF);
            gw(b + 0x14, *u);
            gw(b + 0x18, *v);
        }
        rw_im2d_render_indexed_primitive(3, SCREEN_VERTS, 4, QUAD_INDICES, 6);
    } else {
        rw_render_state_set(8, 1);
        rw_render_state_set(6, 1);
        rw_render_state_set(0xC, 0);
        rw_render_state_set(0xE, 1);
        for q in 0..2u32 {
            let src = MIRROR_QUADS + 0x30 * q;
            for c in 0..4u32 {
                for j in 0..3u32 {
                    gw(TEMP_RENDER_VERTICES + 0x24 * c + 4 * j, g::<u32>((src + 0xC * c + 4 * j) as usize).read());
                }
            }
            draw_temp_quad();
        }
    }
    let cm = g::<GPtr<u8>>(CAM_MATRIX).read();
    let cp: GPtr<u8> = if cm.is_null() { GPtr::from_addr(THE_CAMERA + 4) } else { cm.byte_add(0x30) };
    let dx = x(rd::<f32>(cp, 0)).fsub_m(216.0);
    let dy = x(rd::<f32>(cp, 4)).fsub_m(-42.0);
    let dz = x(rd::<f32>(cp, 8)).fsub_m(1003.0);
    let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
    if !jp(d, x(50.0), 5) {
        rw_render_state_set(8, 1);
        rw_render_state_set(0xC, 0);
        rw_render_state_set(0xA, 1);
        rw_render_state_set(0xB, 2);
        let pos = [(216.0, -45.0, 1000.0), (216.0, -45.0, 1006.0), (216.0, -39.0, 1006.0), (216.0, -39.0, 1000.0)];
        for (c, (px, py, pz)) in pos.iter().enumerate() {
            let b = TEMP_RENDER_VERTICES + 0x24 * c as u32;
            gw(b, f32::to_bits(*px));
            gw(b + 4, f32::to_bits(*py));
            gw(b + 8, f32::to_bits(*pz));
        }
        draw_temp_quad();
    }
    rw_render_state_set(0xE, 0);
    rw_render_state_set(6, 1);
    rw_render_state_set(8, 1);
    rw_render_state_set(1, 0);
    rw_render_state_set(0xC, 0);
    rw_render_state_set(0xA, 5);
    rw_render_state_set(0xB, 6);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_RENDER_MIRROR_BUFFER];
