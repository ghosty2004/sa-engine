//! Camera collision scans (batch `mvpm-coll-1`): the ped hit-sphere depth helper 0x50D590 that
//! `CCollision::CheckCameraCollisionPeds` calls. (`CCollision::CheckCameraCollisionVehicles` 0x41A990 is not hooked: the
//! no-CD protection reads its entry bytes as data and a protection call targets 0x41A997.) Ported from the 1.0 US bytes (gta-reversed
//! `Collision.cpp` as a reading aid; the bytes win). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::world::common::{g, rd, x};

orig! {
    /// `CPedModelInfo::AnimatePedColModelSkinnedWorld(RpClump*)` (0x4C7170, `ret 4`)
    fn ped_mi_animate_col_model_skinned_world(mi: GPtr<u8>, clump: u32) = thiscall 0x4C7170 => unported;
}

/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8)
const MODEL_INFOS: u32 = 0xA9B0C8;
/// `TheCamera.ActiveCam` (u8, 0xB6F081) and `TheCamera.Cams` (0xB6F19C, 0x238 each); `Scene.m_pRwCamera` (0xC1703C)
const ACTIVE_CAM: usize = 0xB6F081;
const CAMS: u32 = 0xB6F19C;
const SCENE_CAMERA: usize = 0xC1703C;

fn model_info(e: GPtr<u8>) -> GPtr<u8> {
    let i = rd::<i16>(e, 0x22) as i32;
    g::<GPtr<u8>>((MODEL_INFOS as i32).wrapping_add(i.wrapping_mul(4)) as u32 as usize).read()
}

/// The ped hit-sphere depth helper 0x50D590 (cdecl `(CPed*)`, ST0): `AnimatePedColModelSkinnedWorld(ped clump)` on the
/// ped's model info, then over the 12 spheres of its hit col model (+0x34, data +0x2C, spheres +8, 0x14 each) the
/// minimum (start 1e6) of `dot(c, N) - d0 - radius - near` (summed `(z + x) + y` for spheres 0, 1, 6, 7, `(z + y) + x` for
/// the others), where `N` / `S` are the active cam's
/// front (+0x190) and source (+0x19C), `d0 = f32((N.z*S.z + N.y*S.y) + N.x*S.x)` and `near` = `Scene.m_pRwCamera`
/// +0x80. A smaller candidate replaces the minimum as its stored f32 (`test ah, 5; jp`). The original stores each
/// candidate in its own argument slot (the ped pointer is overwritten).
#[hook(addr = 0x50d590, conv = cdecl, name = "FUN_0050d590", expect = "83 EC 0C 8B 44 24 10",
       unverified = "mvpm-coll-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the candidate depths into its own argument slot (not modelled); animates the hit col model of the ped's model info")]
pub fn ped_hit_sphere_min_depth(ped: GPtr<u8>) -> f32 {
    let mi = model_info(ped);
    ped_mi_animate_col_model_skinned_world(mi, rd::<u32>(ped, 0x18));
    let cam = GPtr::<u8>::from_addr((CAMS + u32::from(g::<u8>(ACTIVE_CAM).read()) * 0x238) as usize);
    let (nx, ny, nz) = (rd::<f32>(cam, 0x190), rd::<f32>(cam, 0x194), rd::<f32>(cam, 0x198));
    let d0 = x(nz)
        .fmul_m(rd::<f32>(cam, 0x1A4))
        .fadd(x(ny).fmul_m(rd::<f32>(cam, 0x1A0)))
        .fadd(x(nx).fmul_m(rd::<f32>(cam, 0x19C)))
        .to_f32();
    let near = rd::<f32>(g::<GPtr<u8>>(SCENE_CAMERA).read(), 0x80);
    let spheres = rd::<GPtr<u8>>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(mi, 0x34), 0x2C), 8);
    let mut m: X87 = x(f32::from_bits(0x4974_23F0));
    for k in 0..12 {
        let s = spheres.byte_add(0x14 * k);
        let (cx, cy, cz) = (rd::<f32>(s, 0), rd::<f32>(s, 4), rd::<f32>(s, 8));
        // the unrolled body sums z, x, y for its first two spheres (the first loads N.y before c.y), z, y, x for the
        // other four
        let zz = x(cz).fmul_m(nz);
        let dot = match k % 6 {
            0 => zz.fadd(x(cx).fmul_m(nx)).fadd(x(ny).fmul_m(cy)),
            1 => zz.fadd(x(cx).fmul_m(nx)).fadd(x(cy).fmul_m(ny)),
            _ => zz.fadd(x(cy).fmul_m(ny)).fadd(x(cx).fmul_m(nx)),
        };
        let d = dot
            .fsub_m(d0)
            .fsub_m(rd::<f32>(s, 0xC))
            .fsub_m(near);
        if !d.fcom(m).test_parity(5) {
            m = x(d.to_f32());
        }
    }
    m.to_f32()
}

pub const HOOKS: &[HookReg] = &[HOOK_PED_HIT_SPHERE_MIN_DEPTH];
