//! `CGeneral::GetNodeHeadingFromVector` (0x53CDC0, batch `std11-ai-1`): the 0..7 compass node of a direction (ped
//! wander and flee paths). Ported from the 1.0 US bytes; unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CBE0, ST0)
    fn general_get_radian_angle_between_points(x1: u32, y1: u32, x2: u32, y2: u32) -> f64 = cdecl 0x53CBE0 => unported;
    /// CRT `floor(double)` (0x8219F0, ST0)
    fn crt_floor(v: f64) -> f64 = cdecl 0x8219F0 => unported;
}

fn k(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

/// `CGeneral::GetNodeHeadingFromVector(float x, float y)` (0x53CDC0, cdecl), x87: a = `GetRadianAngleBetweenPoints(x,
/// y, 0, 0)`; below 0 → a + T (T = [0x858CBC]); a = T - a + [0x859F50]; not below T → a - T; then `_ftol(floor(double(a
/// * [0x8594F0] * [0x859000])))` in EAX (`_ftol`'s EDX is not reproduced).
#[hook(addr = 0x53CDC0, conv = cdecl, name = "CGeneral::GetNodeHeadingFromVector", expect = "8B 44 24 08 8B 4C 24 04 6A 00",
       unverified = "std11-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn general_get_node_heading_from_vector(x_: u32, y_: u32) -> u32 {
    let t = k(0x858CBC);
    let mut a = X87::from_f64_exact(general_get_radian_angle_between_points(x_, y_, 0, 0));
    if !a.fcom(x(k(0x858B50))).test_parity(0x05) {
        a = a.fadd_m(t);
    }
    a = a.fsubr_m(t).fadd_m(k(0x859F50));
    if !a.fcom(x(t)).test_any(0x01) {
        a = a.fsub_m(t);
    }
    let v = a.fmul_m(k(0x8594F0)).fmul_m(k(0x859000)).to_f64();
    ftol32(X87::from_f64_exact(crt_floor(v))) as u32
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_GENERAL_GET_NODE_HEADING_FROM_VECTOR];
