//! WRITER-NATIVE (batch `native-11`): `CPedGroup::GetClosestGroupPed` (0x5FACD0). Ported from the 1.0 US
//! bytes (the eight member slots are unrolled there).

use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

/// The entity's position (`m_matrix` +0x30, else the placement position +4).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CPedGroup::GetClosestGroupPed(CPed* ped, float* outDistSq)` (0x5FACD0, `ret 8`).
///
/// Over the member slots +0xC..+0x28 other than `ped`: the squared distance `(dx^2 + dz^2) + dy^2` of
/// `ped - member`; a member strictly closer than the best so far (initially 1e8, 0x86C848) becomes the result
/// and the best is reloaded from its f32 copy (kept in the `ped` argument slot). `*outDistSq` (when given)
/// receives the best, 1e8 when nobody qualified. Returns the member or null.
#[hook(addr = 0x5FACD0, conv = thiscall, name = "CPedGroup::GetClosestGroupPed", this = "CPedGroup",
       expect = "8B 51 0C D9 05 48 C8 86 00",
       unverified = "native-11", route = "L2", hazards = "stack-residue", recs = "grove-walk",
       note = "the original keeps the f32 distance in the ped argument slot")]
pub fn group_get_closest_group_ped(this: GPtr<u8>, ped: GPtr<u8>, out: GPtr<f32>) -> GPtr<u8> {
    let mut best = x(f32::from_bits(0x4CBE_BC20));
    let mut closest: GPtr<u8> = GPtr::NULL;
    for k in 0..8 {
        let m = rd::<GPtr<u8>>(this, 0xC + 4 * k);
        if m.is_null() || m == ped {
            continue;
        }
        let a = pos(ped);
        let b = pos(m);
        let dx = x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(b, 0));
        let dy = x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(b, 4));
        let dz = x(rd::<f32>(a, 8)).fsub_m(rd::<f32>(b, 8));
        let s = dx.fmul(dx).fadd(dz.fmul(dz)).fadd(dy.fmul(dy));
        if !s.fcom(best).test_parity(0x05) {
            closest = m;
            best = x(s.to_f32());
        }
    }
    if !out.is_null() {
        out.write(best.to_f32());
    }
    closest
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_GROUP_GET_CLOSEST_GROUP_PED];
