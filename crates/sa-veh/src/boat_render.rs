//! PORT-VEH-4 (batch `veh4-2`): `CBoat::Render`. Ported from the 1.0 US bytes; gta-reversed as a
//! reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr};

orig! {
    /// `CVehicle::Render()` (0x6D0E60)
    fn vehicle_render(v: GPtr<u8>) = thiscall 0x6D0E60 => unported;
    /// `RwRenderStateSet(RwRenderState, void*)` through `RwEngineInstance` (0xC97B24) +0x20
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70)
    fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `RwIm3DTransform(RwIm3DVertex*, uint32, RwMatrix*, uint32)` (0x7EF450)
    fn rw_im3d_transform(verts: GPtr<u8>, n: u32, ltm: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(RwPrimitiveType, RwImVertexIndex*, int32)` (0x7EF550)
    fn rw_im3d_render_indexed_primitive(prim: u32, idx: GPtr<u8>, n: i32) -> i32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    fn rw_im3d_end() -> i32 = cdecl 0x7EF520 => unported;
}

/// The boat's water-mask quad (`RwIm3DVertex` x4, 0x24 bytes each) at 0xC278F8.
const QUAD: usize = 0xC2_78F8;

/// The mask quad's corner positions by model: (x0, y0, z0) and (x2, y2, z2) as bits; vertices 1 and 3
/// mirror 0 and 2 in x.
fn quad_corners(model: i16) -> Option<[u32; 6]> {
    Some(match model {
        430 => [0xBFB9_999A, 0x3FF3_3333, 0x3F75_C28F, 0xBFB9_999A, 0xC070_0000, 0x3F75_C28F],
        446 => [0xBF9C_6A7F, 0x4000_4189, 0x3FB4_5A1D, 0xBF9E_B852, 0xBFAE_F9DB, 0x3F58_9375],
        452 => [0xBF93_3333, 0x4067_0A3D, 0x3F83_D70A, 0xBF93_3333, 0x3D75_C28F, 0x3F83_D70A],
        453 => [0xBFD4_7AE1, 0xC08F_5C29, 0x3F54_7AE1, 0xBFF3_3333, 0x4035_1EB8, 0x3F80_0000],
        454 => [0xBFF1_6873, 0xC016_353F, 0x3F49_78D5, 0xBFF1_6873, 0xC095_70A4, 0x3F57_8D50],
        472 => [0xBF29_BA5E, 0x4064_28F6, 0x3EC3_9581, 0xBF8B_22D1, 0x3F54_BC6A, 0x3EC3_126F],
        473 => [0xBF4C_0831, 0x3FD2_0C4A, 0x3F12_B021, 0xBF5D_70A4, 0xBFB8_D4FE, 0x3F02_4DD3],
        484 => [0xBF9F_7CEE, 0xBFAF_BE77, 0x3F49_78D5, 0xBF82_F1AA, 0xC0AA_4DD3, 0x3F49_78D5],
        595 => [0xBF80_0000, 0x4020_0000, 0x3E99_999A, 0xBF80_0000, 0xC0AC_CCCD, 0x3E99_999A],
        _ => return None,
    })
}

/// Writes the quad's corners: vertex k at `QUAD + k * 0x24`.
fn set_corners(c: [u32; 6]) {
    let v = |k: usize, o: usize, val: u32| g::<u32>(QUAD + k * 0x24 + o).write(val);
    for (k, base, flip) in [(0usize, 0usize, 0u32), (1, 0, 0x8000_0000), (2, 3, 0), (3, 3, 0x8000_0000)] {
        v(k, 0, c[base] ^ flip);
        v(k, 4, c[base + 1]);
        v(k, 8, c[base + 2]);
    }
}

/// The quad's uvs (0,0), (1,0), (0,1), (1,1).
fn set_uvs() {
    for (k, u, vv) in [(0usize, 0u32, 0u32), (1, 0x3F80_0000, 0), (2, 0, 0x3F80_0000), (3, 0x3F80_0000, 0x3F80_0000)] {
        g::<u32>(QUAD + k * 0x24 + 0x1C).write(u);
        g::<u32>(QUAD + k * 0x24 + 0x20).write(vv);
    }
}

/// Transforms the quad by the boat's matrix and draws it as two triangles (indices at 0xC27988).
fn draw_quad(this: GPtr<u8>) {
    let mut ltm = [0u32; 0x10];
    let lp: GPtr<u8> = GPtr::from_raw(&mut ltm as *mut [u32; 0x10]).cast();
    matrix_update_rw_matrix(rd::<GPtr<u8>>(this, 0x14), lp);
    if !rw_im3d_transform(GPtr::from_addr(QUAD), 4, lp, 1).is_null() {
        rw_im3d_render_indexed_primitive(3, GPtr::from_addr(0xC2_7988), 6);
        rw_im3d_end();
    }
}

/// `CBoat::Render()` (0x6F0210): the boat stays rendered for 3 s (+0x4D0 = now + 3000) and is drawn
/// as a vehicle unless 0x96914B is set; except for the Skimmer (460) a water-mask quad (white, indices
/// 0,2,1,1,2,3) with per-model corners is drawn into the z-buffer only (texture raster from 0xC22EA8,
/// z-write on, z-test off, src blend 1 / dst blend 2) for the boats that have one; the Coastguard (472)
/// draws a second quad; the render states are reset (z-test on, blend 5/6). The draws are skipped
/// while 0x96914B is set.
#[hook(addr = 0x6F0210, conv = thiscall, name = "CBoat::Render", this = "CBoat",
       expect = "A1 84 CB B7 00",
       unverified = "veh4-2", route = "L3-only", hazards = "none",
       note = "rendering: RW render states and Im3D; the quad statics 0xC278F8.. are written; the texture-raster value is decoded by a protection fragment (0x40120A: [0xC22EA8])")]
pub fn boat_render(this: GPtr<u8>) {
    wr::<u32>(this, 0x4D0, g::<u32>(0xB7_CB84).read().wrapping_add(0xBB8));
    let hidden = || g::<u8>(0x96_914B).read() != 0;
    if !hidden() {
        vehicle_render(this);
    }
    let model = rd::<i16>(this, 0x22);
    if model == 0x1CC {
        return;
    }
    for k in 0..4 {
        g::<u32>(QUAD + k * 0x24 + 0x18).write(0xFFFF_FFFF);
    }
    for (k, i) in [0u16, 2, 1, 1, 2, 3].into_iter().enumerate() {
        g::<u16>(0xC2_7988 + 2 * k).write(i);
    }
    let Some(c) = quad_corners(model) else { return };
    set_corners(c);
    let raster = g::<u32>(0xC2_2EA8).read();
    set_uvs();
    rw_render_state_set(1, raster);
    rw_render_state_set(0xC, 1);
    rw_render_state_set(0xE, 0);
    rw_render_state_set(0xA, 1);
    rw_render_state_set(0xB, 2);
    if !hidden() {
        draw_quad(this);
    }
    if model == 0x1D8 {
        set_corners([0xBF8B_22D1, 0x3F54_BC6A, 0x3EC3_126F, 0xBF8C_6A7F, 0xC03E_872B, 0x3EC3_126F]);
        set_uvs();
        if !hidden() {
            draw_quad(this);
        }
    }
    rw_render_state_set(0xE, 1);
    rw_render_state_set(0xA, 5);
    rw_render_state_set(0xB, 6);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_BOAT_RENDER];
