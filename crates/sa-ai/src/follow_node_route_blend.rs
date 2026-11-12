//! WRITER-CLIMB (batch `climb-7`): `CTaskComplexFollowNodeRoute::CalcBlendRatio` (0x66EDC0), reached by wandering
//! peds in the native Grove Street runs. Ported from the 1.0 US bytes; x87 values stay unrounded where the original
//! keeps them in registers.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskComplexFollowNodeRoute::GetLastWaypoint(CPed*)` (0x6698E0, hidden result first)
    fn fnr_get_last_waypoint(t: GPtr<u8>, out: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x6698E0 => unported;
    /// `CTaskSimpleGoToPointFine::BaseRatio(eMoveState)` (0x65EF30, ST0)
    fn go_to_point_fine_base_ratio(ms: u32) -> f64 = cdecl 0x65EF30 => unported;
}

fn c(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// `|a - b|^2` as the original sums it: `(dz^2 + dy^2) + dx^2`, unrounded.
fn dist_sq(a: GPtr<u8>, b: GPtr<u8>) -> X87 {
    let dx = x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(b, 0));
    let dy = x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(b, 4));
    let dz = x(rd::<f32>(a, 8)).fsub_m(rd::<f32>(b, 8));
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx))
}

/// `float CTaskComplexFollowNodeRoute::CalcBlendRatio(CPed* ped, bool slowing)` (0x66EDC0, `ret 8`, ST0).
///
/// Slowing: `d^2` from the ped to the current route point (route +0x30, index +0x38) against the slow-down radius
/// +0x50; otherwise from the last waypoint to the ped against +0x54. Outside the radius (`d^2 >= r^2`, or
/// unordered): -1 ([0x858C1C]). Inside: `t = clamp(d^2 / r^2, 0, 1)`, `v = (cos(t pi) * 0.5 + 0.5) * (+0x58
/// slowing, else +0x5C)` stored as f32, `w = BaseRatio(move state +0x18) - v` stored; the result is `BaseRatio(4) +
/// 0.75` when that exceeds `w` (ordered, unrounded), else `w`.
#[hook(addr = 0x66EDC0, conv = thiscall, name = "CTaskComplexFollowNodeRoute::CalcBlendRatio",
       expect = "83 EC 0C 53 8A 5C 24 18",
       unverified = "climb-7", route = "L2", hazards = "x87-transcendental", recs = "grove-walk",
       note = "fcos; the result may be an unrounded ST0")]
pub fn fnr_calc_blend_ratio(this: GPtr<u8>, ped: GPtr<u8>, slowing: u32) -> X87 {
    let slowing = slowing as u8 != 0;
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    let (sq, r) = if slowing {
        let pt = rd::<GPtr<u8>>(this, 0x30).byte_add(rd::<i32>(this, 0x38) as isize * 12 + 4);
        (dist_sq(pt, pos), rd::<f32>(this, 0x50))
    } else {
        let mut out = [0u32; 3];
        let w = fnr_get_last_waypoint(this, GPtr::from_raw(out.as_mut_ptr()).cast(), ped);
        (dist_sq(pos, w), rd::<f32>(this, 0x54))
    };
    let r2 = x(r).fmul(x(r));
    if sq.fcom(r2).test_parity(0x05) {
        return x(c(0x85_8C1C));
    }
    let mut t = sq.fdiv(r2);
    if !t.fcom(x(c(0x85_8B50))).test_parity(0x05) {
        t = x(c(0x85_8B50));
    } else if !t.fcom(x(c(0x85_8624))).test_any(0x41) {
        t = x(c(0x85_8624));
    }
    let half = c(0x85_8B8C);
    let k = if slowing { rd::<f32>(this, 0x58) } else { rd::<f32>(this, 0x5C) };
    let v = t.fmul_m(c(0x85_8CB8)).fcos().fmul_m(half).fadd_m(half).fmul_m(k).to_f32();
    let w = X87::from_f64_exact(go_to_point_fine_base_ratio(rd::<u32>(this, 0x18))).fsub_m(v).to_f32();
    let top = X87::from_f64_exact(go_to_point_fine_base_ratio(4)).fadd_m(c(0x85_8F34));
    if top.fcom(x(w)).test_any(0x41) {
        return x(w);
    }
    top
}

pub const HOOKS: &[HookReg] = &[HOOK_FNR_CALC_BLEND_RATIO];
