//! WRITER-WORLD-2 (batch `world6-3`): `CEntity::LivesInThisNonOverlapSector`. Ported from the 1.0
//! US bytes; gta-reversed as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{ftol32, x};

orig! {
    /// `CEntity::GetBoundRect()` (vtable +0x24): the `CRect` through the hidden pointer
    fn v_get_bound_rect(e: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// CRT `floor(double)` (0x8219F0), ST0
    fn crt_floor(v: f64) -> f64 = cdecl 0x8219F0 => unported;
}

/// `0.02` (0x858B38), `60` (0x858B34), `0.5` (0x858B8C).
const F_0_02: f32 = f32::from_bits(0x3CA3_D70A);
const F_60: f32 = 60.0;

/// `floor(v * 0.02 + 60)` as the CRT computes it from the double.
fn sector_floor(v: X87) -> X87 {
    X87::from_f64_exact(crt_floor(v.fmul_m(F_0_02).fadd_m(F_60).to_f64()))
}

/// `CEntity::LivesInThisNonOverlapSector(int32 x, int32 y)` (0x533050): the sector of the bound
/// rect's centre (`ftol(floor(((right + left) * 0.5) * 0.02 + 60))`, likewise bottom / top) is
/// (x, y). The original first computes and discards the floors of the four edges' sectors (left,
/// top, right, bottom); the calls are kept.
#[hook(addr = 0x533050, conv = thiscall, name = "CEntity::LivesInThisNonOverlapSector", this = "CEntity",
       expect = "55 8B EC 83 E4 F8",
       unverified = "world6-3", route = "L2", hazards = "none")]
pub fn lives_in_this_non_overlap_sector(this: GPtr<u8>, sx: i32, sy: i32) -> u32 {
    let mut r = [0f32; 4]; // left, bottom, right, top
    v_get_bound_rect(this, GPtr::from_raw(&mut r as *mut [f32; 4]).cast());
    for k in [0usize, 3, 2, 1] {
        sector_floor(x(r[k]));
    }
    let cx = ftol32(sector_floor(x(r[2]).fadd_m(r[0]).fmul_m(0.5)));
    let cy = ftol32(sector_floor(x(r[1]).fadd_m(r[3]).fmul_m(0.5)));
    u32::from(sx == cx && sy == cy)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_LIVES_IN_THIS_NON_OVERLAP_SECTOR];
