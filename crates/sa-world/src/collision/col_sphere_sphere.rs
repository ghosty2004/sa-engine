//! `CColSphere::IntersectSphere` (0x410090, batch `std10-world-1`): ported from the 1.0 US bytes (gta-reversed's
//! version as a reading aid). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

fn f(p: GPtr<u8>, o: isize) -> f32 {
    p.byte_add(o).cast::<f32>().read()
}

/// `CColSphere::IntersectSphere(const CColSphere& other)` (0x410090, thiscall `ret 4`, bool in AL): with the centre
/// differences `d = this - other` and `r = other.r + this.r` kept on the x87 stack, true when
/// `r * r > (dz*dz + dx*dx) + dy*dy` (C0 and C3 clear).
#[hook(addr = 0x410090, conv = thiscall, name = "CColSphere::IntersectSphere", expect = "8B 44 24 04 D9 01 D8 20 32 D2",
       unverified = "std10-world-1", route = "L2", hazards = "ax-al-return", recs = "grove-walk",
       note = "AL result (upper EAX from fnstsw in the original)")]
pub fn col_sphere_intersect_sphere(this: GPtr<u8>, other: GPtr<u8>) -> u8 {
    let dx = X87::from_f32(f(this, 0)).fsub_m(f(other, 0));
    let dy = X87::from_f32(f(this, 4)).fsub_m(f(other, 4));
    let dz = X87::from_f32(f(this, 8)).fsub_m(f(other, 8));
    let r = X87::from_f32(f(other, 0xC)).fadd_m(f(this, 0xC));
    let sum = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
    u8::from(!r.fmul(r).fcom(sum).test_any(0x41))
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_COL_SPHERE_INTERSECT_SPHERE];
