//! WRITER-WORLD-2 (batch `world6-4`): `InteriorGroup_c::FindClosestInteriorInfo`. Ported from the
//! 1.0 US bytes; gta-reversed has it unreversed.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{rd, x};

orig! {
    /// `Interior_c::IsPtInside(const CVector&, float, float, float)` (0x5913E0)
    fn is_pt_inside(it: GPtr<u8>, pt: GPtr<f32>, mx: f32, my: f32, mz: f32) -> u8 = thiscall 0x5913E0 => super::misc2::interior_is_pt_inside;
}

/// `InteriorGroup_c::FindClosestInteriorInfo(int32 type, CVector pos, float maxDist,
/// InteriorInfo_t** outInfo, Interior_c** outInterior, float* outDistSq)` (0x594A50): among the
/// unused infos (+1 == 0) of `type` (-1: any) of the group's interiors that contain `pos`, the
/// nearest one closer than `maxDist` (squared distance `(dz² + dy²) + dx²`, compared unrounded,
/// kept as f32; start 1e6). Found: the outputs (the squared distance in `outDistSq`) and true.
#[hook(addr = 0x594A50, conv = thiscall, name = "InteriorGroup_c::FindClosestInteriorInfo", this = "InteriorGroup_c",
       expect = "83 EC 18 D9 44 24 2C",
       unverified = "world6-4", route = "L2", hazards = "none")]
#[allow(clippy::too_many_arguments)]
pub fn find_closest_interior_info(this: GPtr<u8>, ty: i32, px: f32, py: f32, pz: f32, max_dist: f32,
                                  out_info: GPtr<GPtr<u8>>, out_interior: GPtr<GPtr<u8>>, out_dist: GPtr<f32>) -> u8 {
    let max2 = x(max_dist).fmul_m(max_dist).to_f32();
    let mut best = f32::from_bits(0x4974_23F0);
    let (mut found, mut owner) = (GPtr::<u8>::NULL, GPtr::<u8>::NULL);
    let mut pos = [px, py, pz];
    for k in 0..8 {
        let it = rd::<GPtr<u8>>(this, 0x10 + 4 * k);
        if it.is_null() || is_pt_inside(it, GPtr::from_raw(&mut pos as *mut [f32; 3]).cast(), 0.0, 0.0, 0.0) == 0 {
            continue;
        }
        let n = i32::from(rd::<i8>(it, 0x40D));
        let mut info = it.byte_add(0x590);
        for _ in 0..n.max(0) {
            if (ty == -1 || i32::from(rd::<i8>(info, 0)) == ty) && rd::<u8>(info, 1) == 0 {
                let dx = x(px).fsub_m(rd::<f32>(info, 4));
                let dy = x(py).fsub_m(rd::<f32>(info, 8));
                let dz = x(pz).fsub_m(rd::<f32>(info, 0xC));
                let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
                if !d2.fcom(x(max2)).test_parity(0x05) && !d2.fcom(x(best)).test_parity(0x05) {
                    best = d2.to_f32();
                    found = info;
                    owner = it;
                }
            }
            info = info.byte_add(0x20);
        }
    }
    if found.is_null() {
        return 0;
    }
    out_info.write(found);
    out_interior.write(owner);
    out_dist.write(best);
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FIND_CLOSEST_INTERIOR_INFO];
