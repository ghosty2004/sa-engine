//! WRITER-NATIVE (batch `native-33`): `CVehicle::CanPedEnterCar` (0x6D1E80), asked by peds and the player near
//! cars on the native and web builds. Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use crate::common::{rd, x};

/// `|v|^2` of the three floats at `p + off` as `(x^2 + y^2) + z^2`.
fn len2(p: GPtr<u8>, off: isize) -> X87 {
    let (vx, vy, vz) = (x(rd::<f32>(p, off)), x(rd::<f32>(p, off + 4)), x(rd::<f32>(p, off + 8)));
    vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz))
}

/// `bool CVehicle::CanPedEnterCar()` (0x6D1E80, AL).
///
/// Not a bike (+0x590 = 9) lying on its side: false when `up.z` (matrix +0x28) is neither above 0.1 (ordered) nor
/// below -0.1. Then false when the move speed (+0x44) or the turn speed (+0x50) squared is above 0.04 (ordered).
#[hook(addr = 0x6D1E80, conv = thiscall, name = "CVehicle::CanPedEnterCar", this = "CVehicle",
       expect = "83 EC 0C 8B 41 14 83 C0 20",
       unverified = "native-33", route = "L2", hazards = "ax-al-return", recs = "grove-walk,driving")]
pub fn vehicle_can_ped_enter_car(this: GPtr<u8>) -> u8 {
    let up_z = rd::<f32>(rd::<GPtr<u8>>(this, 0x14), 0x28);
    if rd::<u32>(this, 0x590) != 9
        && !x(up_z).fcom(x(f32::from_bits(0x3DCC_CCCD))).test_parity(0x41)
        && !x(up_z).fcom(x(f32::from_bits(0xBDCC_CCCD))).test_any(0x01)
    {
        return 0;
    }
    let lim = x(f32::from_bits(0x3D23_D70B));
    if !len2(this, 0x44).fcom(lim).test_any(0x41) {
        return 0;
    }
    u8::from(len2(this, 0x50).fcom(lim).test_any(0x41))
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_VEHICLE_CAN_PED_ENTER_CAR];
