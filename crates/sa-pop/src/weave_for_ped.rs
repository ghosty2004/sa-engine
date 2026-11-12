//! WRITER-NATIVE (batch `native-17`): 0x426970, the per-ped weave of `CCarCtrl::WeaveThroughPedsSectorList`
//! (gta-reversed's `WeaveForPed`, unnamed in the catalogue), reached when CJ walks in front of traffic on the
//! native walk. Ported from the 1.0 US bytes; x87 arithmetic follows the original's instruction order.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd};

orig! {
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CGeneral::GetATanOfXY(float x, float y)` (0x53CC70, ST0)
    fn general_get_atan_of_xy(x: f32, y: f32) -> f64 = cdecl 0x53CC70 => unported;
}

/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8)
const MODEL_INFO_PTRS: usize = 0xA9_B0C8;
/// -pi (0x858CC0), 2 pi (0x858CBC), pi (0x858CB8)
const NEG_PI: u32 = 0xC049_0FDB;
const TWO_PI: u32 = 0x40C9_0FDB;
const PI: u32 = 0x4049_0FDB;

fn x(v: f32) -> X87 {
    X87::from_f32(v)
}

/// The entity's position (`m_matrix` +0x30, else the placement position +4).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `(a - b)` wrapped into [-pi, pi] by the original's add / subtract loops, then made non-negative (`fchs`
/// below 0, ordered).
fn wrapped_abs(mut d: X87) -> X87 {
    let (neg_pi, two_pi, pi) = (f32::from_bits(NEG_PI), f32::from_bits(TWO_PI), f32::from_bits(PI));
    while !d.fcom(x(neg_pi)).test_parity(0x05) {
        d = d.fadd_m(two_pi);
    }
    while !d.fcom(x(pi)).test_any(0x41) {
        d = d.fsub_m(two_pi);
    }
    if !d.fcom(x(0.0)).test_parity(0x05) {
        d = d.fchs();
    }
    d
}

/// `CCarCtrl::WeaveForPed(CEntity* ped, CVehicle* veh, float* a, float* b)` (0x426970, cdecl).
///
/// Skips the player for mission 3 and the target (+0x41C) for mission 0x37. `angle = GetATanOfXY(dx, dy)` of
/// `ped - veh`; nothing within distance 1 (strictly less, ordered). `half = (bbox.max.x * 2.4 + 0.8) / dist *
/// 0.5`. When `|angle - *a|` (wrapped) is below `half`: `*a = angle - half`, raised by 2 pi while below -pi.
/// Then when `|angle - *b|` is below `half`: `*b = angle + half`, lowered by 2 pi while above pi. The original
/// keeps `dy`, then `half` in the vehicle argument slot.
#[hook(addr = 0x426970, conv = cdecl, name = "FUN_00426970",
       expect = "83 EC 08 53 8B 5C 24 14",
       unverified = "native-17", route = "L2", hazards = "x87-transcendental,stack-residue",
       recs = "grove-walk,car-ai",
       note = "fsqrt; GetATanOfXY; rewrites its vehicle argument slot")]
pub fn weave_for_ped(ped: GPtr<u8>, veh: GPtr<u8>, a: GPtr<f32>, b: GPtr<f32>) {
    let mission = rd::<u8>(veh, 0x3BA);
    if mission == 3 && ped == find_player_ped(-1) {
        return;
    }
    if rd::<u8>(veh, 0x3BA) == 0x37 && ped == rd::<GPtr<u8>>(veh, 0x41C) {
        return;
    }
    let (pp, vp) = (pos(ped), pos(veh));
    let dx = x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(vp, 0)).to_f32();
    let dy = x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(vp, 4)).to_f32();
    let angle = X87::from_f64_exact(general_get_atan_of_xy(dx, dy)).to_f32();
    let dist = x(dy).fmul_m(dy).fadd(x(dx).fmul_m(dx)).fsqrt();
    if !dist.fcom(x(1.0)).test_parity(0x05) {
        return;
    }
    let info = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(veh, 0x22))).read();
    let max_x = rd::<f32>(rd::<GPtr<u8>>(info, 0x14), 0xC);
    let ratio = x(max_x)
        .fmul_m(f32::from_bits(0x4019_999A))
        .fadd_m(f32::from_bits(0x3F4C_CCCD))
        .fdiv(dist)
        .to_f32();
    let half = x(ratio).fmul_m(0.5).to_f32();
    let (neg_pi, two_pi, pi) = (f32::from_bits(NEG_PI), f32::from_bits(TWO_PI), f32::from_bits(PI));
    if !wrapped_abs(x(angle).fsub_m(a.read())).fcom(x(half)).test_parity(0x05) {
        let na = x(angle).fsub_m(half);
        a.write(na.to_f32());
        if !na.fcom(x(neg_pi)).test_parity(0x05) {
            let mut v = x(a.read());
            loop {
                v = v.fadd_m(two_pi);
                if v.fcom(x(neg_pi)).test_parity(0x05) {
                    break;
                }
            }
            a.write(v.to_f32());
        }
    }
    if wrapped_abs(x(angle).fsub_m(b.read())).fcom(x(half)).test_parity(0x05) {
        return;
    }
    let nb = x(half).fadd_m(angle);
    b.write(nb.to_f32());
    if nb.fcom(x(pi)).test_any(0x41) {
        return;
    }
    let mut v = x(b.read());
    loop {
        v = v.fsub_m(two_pi);
        if v.fcom(x(pi)).test_any(0x41) {
            break;
        }
    }
    b.write(v.to_f32());
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_WEAVE_FOR_PED];
