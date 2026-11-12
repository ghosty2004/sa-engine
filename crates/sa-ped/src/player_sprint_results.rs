//! WRITER-NATIVE (batch `native-21`): `CPlayerPed::GetButtonSprintResults` (0x60A820), reached when the player
//! sprints. Unreversed in gta-reversed; ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

/// `CPlayerPed::GetButtonSprintResults(eSprintType)` (0x60A820, `ret 4`, ST0; EDX untouched, which the caller at
/// 0x60B44C relies on).
///
/// `v` = the player data's (+0x480) sprint button counter +0x1C. Above `[0x8D2458]` (ordered): `max(v / limit - 1,
/// 0) * [0x8D247C + type * 32] + 1`; otherwise 1 when `v > 0`, else 0.
#[hook(addr = 0x60A820, conv = thiscall, name = "CPlayerPed::GetButtonSprintResults", this = "CPlayerPed",
       expect = "8B 89 80 04 00 00 D9 41 1C",
       unverified = "native-21", route = "L2", hazards = "preserved-regs", preserve = "edx", recs = "grove-walk",
       note = "ST0 result; the caller at 0x60B44C keeps EDX across the call")]
pub fn get_button_sprint_results(this: GPtr<u8>, ty: i32) -> X87 {
    let pd = rd::<GPtr<u8>>(this, 0x480);
    let v = x(rd::<f32>(pd, 0x1C));
    let limit = g::<f32>(0x8D_2458).read();
    if !v.fcom(x(limit)).test_any(0x41) {
        let mut r = v.fdiv_m(limit).fsub_m(1.0);
        if !x(0.0).fcom(r).test_any(0x41) {
            r = x(0.0);
        }
        let k = g::<f32>(0x8D_247Cu32.wrapping_add((ty as u32).wrapping_shl(5)) as usize).read();
        return r.fmul_m(k).fadd_m(1.0);
    }
    if v.fcom(x(0.0)).test_any(0x41) { x(0.0) } else { x(1.0) }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_GET_BUTTON_SPRINT_RESULTS];
