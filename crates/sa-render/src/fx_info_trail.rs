//! WRITER-NATIVE (batch `native-5`): `FxInfoTrail_c::GetValue` (0x4A62B0, vtable slot 8 of the trail info),
//! reached by spark particles on the native walk. Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{rd, wr};

orig! {
    /// `FxInterpInfoU255_c::GetVal(float* out, float t)` (0x4A8800)
    fn interp_get_val(this: GPtr<u8>, out: GPtr<u8>, t: u32) = thiscall 0x4A8800 => unported;
}

/// `FxInfoTrail_c::GetValue(float currTime, float mult, float timeStep, float totalTime, bool useConst,
/// void* out)` (0x4A62B0, `ret 0x18`): the key time (`mult` with byte +6 set, else `(float)(currTime /
/// totalTime)`), the interp member's `GetVal`; `out +0x30 = key[0]` (fld / fstp), `out +0x2C = 1`, or 2 when
/// `key[1] > 0.1` (ordered).
#[hook(addr = 0x4A62B0, conv = thiscall, name = "FxInfoTrail_c::GetValue",
       expect = "8A 41 06 83 EC 40",
       unverified = "native-5", route = "L2", hazards = "heap-writes", recs = "grove-walk,driving",
       note = "writes the particle render info")]
pub fn fx_info_trail_get_value(this: GPtr<u8>, curr: u32, mult: u32, _step: u32, total: u32, _use_const: u32, out: GPtr<u8>) {
    let t = if rd::<u8>(this, 6) != 0 {
        mult
    } else {
        X87::from_f32(f32::from_bits(curr)).fdiv_m(f32::from_bits(total)).to_f32().to_bits()
    };
    let mut b = [0u32; 16];
    interp_get_val(this.byte_add(8), GPtr::from_raw(b.as_mut_ptr()).cast(), t);
    wr::<f32>(out, 0x30, X87::from_f32(f32::from_bits(b[0])).to_f32());
    wr::<u32>(out, 0x2C, 1);
    if !X87::from_f32(f32::from_bits(b[1])).fcom(X87::from_f32(f32::from_bits(0x3DCC_CCCD))).test_any(0x41) {
        wr::<u32>(out, 0x2C, 2);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FX_INFO_TRAIL_GET_VALUE];
