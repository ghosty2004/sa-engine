//! `CShadows::StoreShadowForPole` (batch `mvpk-render-1`): the static sun shadow of a lamp post or pole. Ported from
//! the 1.0 US bytes (gta-reversed `Shadows.cpp` as a reading aid; the bytes win). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, x};

orig! {
    /// `CStencilShadows::GraphicsHighQuality()` (0x70F9B0)
    fn stencil_graphics_high_quality() -> u8 = cdecl 0x70F9B0 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CShadows::StoreStaticShadow(id, type, texture, pos, frontX, frontY, sideX, sideY, intensity, r, g, b, zDistance,
    /// scale, drawDistance, temporary, upDistance)` (0x70BA00)
    fn store_static_shadow(id: u32, ty: u32, tex: u32, pos: GPtr<u8>, fx: u32, fy: u32, sx: u32, sy: u32, intensity: u32, r: u32, gr: u32, b: u32, zd: u32, scale: u32, dd: u32, temp: u32, up: u32) -> u8 = cdecl 0x70BA00 => unported;
}

/// `CTimeCycle::m_CurrentColours.m_nShadowStrength` (0xB7C4EC, i16)
const SHADOW_STRENGTH: usize = 0xB7C4EC;
/// `CTimeCycle::m_CurrentStoredValue` (0xB79FD0): the sun direction index
const SUN_INDEX: usize = 0xB79FD0;
/// `CTimeCycle::m_VectorToSun` (0xB7CA50, 12 bytes each), `m_fShadowSideX` (0xB79ED0), `m_fShadowSideY` (0xB79F10)
const VECTOR_TO_SUN: usize = 0xB7CA50;
const SHADOW_SIDE_X: usize = 0xB79ED0;
const SHADOW_SIDE_Y: usize = 0xB79F10;
/// `gpPostShadowTex` (0xC4040C)
const POST_SHADOW_TEX: usize = 0xC4040C;

fn kf(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// The entity matrix, allocated from the placement first when missing (the inlined `CPlaceable::GetMatrix`).
fn matrix_of(e: GPtr<u8>) -> GPtr<u8> {
    if rd::<GPtr<u8>>(e, 0x14).is_null() {
        placeable_allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    rd::<GPtr<u8>>(e, 0x14)
}

/// `CShadows::StoreShadowForPole(CEntity*, float offX, float offY, float offZ, float poleHeight, float poleWidth, uint32
/// localId)` (0x70C750, cdecl). x87. Nothing with high-quality stencil shadows or a zero shadow strength (0xB7C4EC), or
/// when the entity's matrix "at" z is below 0.5 (a fallen pole; NaN continues). The matrix is allocated when missing
/// (inlined `GetMatrix`). `k = _ftol((at.z - 0.5) * strength * 2)`; with the sun index `i` (0xB79FD0) and `h = height *
/// 0.5`: position `(f32(f32(offY * F.x + offX * R.x + pos.x) + f32(-(h * sun[i].x))), f32(f32(-(h * sun[i].y)) + (offX *
/// R.y + offY * F.y + pos.y)), f32(pos.z + offZ))` (R right +0, F forward +0x10); `StoreStaticShadow(entity + localId +
/// 0x33, 1, gpPostShadowTex, &pos, f32(-(h * sun.x)), f32(-(h * sun.y)), width * sideY[i], width * sideX[i], (2k) / 3, 0,
/// 0, 0, 9.0, 1.0, 40.0, false, 0)`.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x70c750, conv = cdecl, name = "CShadows::StoreShadowForPole", expect = "83 EC 14 E8 58 32 00 00",
       route = "L2", hazards = "none", recs = "grove-walk",
       note = "the chunk at 0x70C760 is a no-CD fragment (the shadow strength compare)")]
pub fn shadows_store_shadow_for_pole(ent: GPtr<u8>, off_x: f32, off_y: f32, off_z: f32, height: f32, width: f32, local_id: u32) {
    if stencil_graphics_high_quality() != 0 {
        return;
    }
    if g::<i16>(SHADOW_STRENGTH).read() == 0 {
        return;
    }
    let m0 = rd::<GPtr<u8>>(ent, 0x14);
    if !m0.is_null() && !x(rd::<f32>(m0, 0x28)).fcom(x(kf(0x858B8C))).test_parity(5) {
        return;
    }
    let m = matrix_of(ent);
    let strength = i32::from(g::<i16>(SHADOW_STRENGTH).read());
    let k = x(rd::<f32>(m, 0x28)).fsub_m(kf(0x858B8C)).fmul(X87::from_i32(strength)).fdouble().ftol() as u32 as i32;
    let p = if m.is_null() { ent.byte_add(4) } else { m.byte_add(0x30) };
    let pos = [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)];
    let f = matrix_of(ent);
    let r = matrix_of(ent);
    let px1 = x(off_y).fmul_m(rd::<f32>(f, 0x10)).fadd(x(off_x).fmul_m(rd::<f32>(r, 0))).fadd_m(pos[0]).to_f32();
    let f = matrix_of(ent);
    let r = matrix_of(ent);
    let py1 = x(off_x).fmul_m(rd::<f32>(r, 4)).fadd(x(off_y).fmul_m(rd::<f32>(f, 0x14))).fadd_m(pos[1]);
    let i = g::<u32>(SUN_INDEX).read() as usize;
    let pz = x(pos[2]).fadd_m(off_z).to_f32();
    let h = x(height).fmul_m(kf(0x858B8C));
    let fx = h.fmul_m(kf(VECTOR_TO_SUN + 12 * i)).fchs().to_f32();
    let fy = h.fmul_m(kf(VECTOR_TO_SUN + 12 * i + 4)).fchs().to_f32();
    let mut out = [x(fx).fadd_m(px1).to_f32(), x(fy).fadd(py1).to_f32(), pz];
    let side_y = x(width).fmul_m(kf(SHADOW_SIDE_X + 4 * i)).to_f32();
    let side_x = x(width).fmul_m(kf(SHADOW_SIDE_Y + 4 * i)).to_f32();
    let intensity = k.wrapping_mul(2) / 3;
    store_static_shadow(
        (ent.addr() as u32).wrapping_add(local_id).wrapping_add(0x33),
        1,
        g::<u32>(POST_SHADOW_TEX).read(),
        GPtr::from_raw(out.as_mut_ptr()).cast(),
        fx.to_bits(),
        fy.to_bits(),
        side_x.to_bits(),
        side_y.to_bits(),
        intensity as u32,
        0,
        0,
        0,
        0x4110_0000,
        0x3F80_0000,
        0x4220_0000,
        0,
        0,
    );
}

pub const HOOKS: &[HookReg] = &[HOOK_SHADOWS_STORE_SHADOW_FOR_POLE];
