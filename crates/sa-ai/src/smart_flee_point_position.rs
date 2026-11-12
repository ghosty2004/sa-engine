//! `CTaskComplexSmartFleePoint::SetFleePosition` (0x65C3C0, batch `std12-ai-1`): reached when an ambient ped flees
//! from a point near CJ. Unreversed in gta-reversed; ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

/// `CTaskComplexSmartFleePoint::SetFleePosition(const CVector& pos, float safeDistance, bool scream)` (0x65C3C0,
/// thiscall, `ret 0xC`). Unless the position (+0x18) and the distance (+0x2C) are all equal (`fcomp; test ah, 0x44`;
/// a NaN counts as different): position and distance stored, the new-flee-point flag (+0x41) set and the scream
/// byte (+0x24) stored.
#[hook(addr = 0x65C3C0, conv = thiscall, name = "CTaskComplexSmartFleePoint::SetFleePosition", expect = "D9 41 18 8B 54 24 04",
       unverified = "std12-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn smart_flee_point_set_flee_position(this: GPtr<u8>, pos: GPtr<u8>, dist: f32, scream: u32) {
    let ne = |a: f32, b: f32| x(a).fcom(x(b)).test_parity(0x44);
    let changed = ne(rd::<f32>(this, 0x18), rd::<f32>(pos, 0))
        || ne(rd::<f32>(this, 0x1C), rd::<f32>(pos, 4))
        || ne(rd::<f32>(this, 0x20), rd::<f32>(pos, 8))
        || ne(rd::<f32>(this, 0x2C), dist);
    if !changed {
        return;
    }
    for o in [0isize, 4, 8] {
        wr::<u32>(this, 0x18 + o, rd::<u32>(pos, o));
    }
    wr::<f32>(this, 0x2C, dist);
    wr::<u8>(this, 0x41, 1);
    wr::<u8>(this, 0x24, scream as u8);
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[HOOK_SMART_FLEE_POINT_SET_FLEE_POSITION];
