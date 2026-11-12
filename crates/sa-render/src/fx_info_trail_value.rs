//! `FxInfoTrail_c::GetValue` (0x4A62B0, batch `attr-5`): the trail keyframe evaluation (reached by sparks near the
//! Grove Street attractors). Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{Fpu, rd, wr};

orig! {
    /// `FxInterpInfoU255_c::GetVal(float* out, float t)` (0x4A8800, `ret 8`)
    fn interp_u255_get_val(this: GPtr<u8>, out: GPtr<u8>, t: u32) = thiscall 0x4A8800 => unported;
}

/// `FxInfoTrail_c::GetValue(float currTime, float mult, float timeStep, float totalTime, bool useConst, void* out)`
/// (0x4A62B0, thiscall, `ret 0x18`): key time `mult` with byte +6 set, else `(float)(currTime / totalTime)`;
/// `FxInterpInfoU255_c::GetVal` into a 16-float buffer; `out +0x30 = key[0]` (fld / fstp), `out +0x2C = 1`, or 2 when
/// `key[1] > 0.1` ([0x858B1C]).
#[hook(addr = 0x4A62B0, conv = thiscall, name = "FxInfoTrail_c::GetValue", expect = "8A 41 06 83 EC 40",
       unverified = "attr-5", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the particle render info")]
pub fn fx_info_trail_get_value(this: GPtr<u8>, curr: u32, mult: u32, _step: u32, total: u32, _use_const: u32, out: GPtr<u8>) {
    let t = if rd::<u8>(this, 6) != 0 {
        mult
    } else {
        let mut f = Fpu::new();
        f.fld(f32::from_bits(curr));
        f.fdiv_m(f32::from_bits(total));
        f.fstp().to_bits()
    };
    let mut b = [0f32; 16];
    interp_u255_get_val(this.byte_add(8), GPtr::from_raw(b.as_mut_ptr()).cast(), t);
    wr::<f32>(out, 0x30, X87::from_f32(b[0]).to_f32());
    wr::<u32>(out, 0x2C, 1);
    let lim = sa_core::mem::GPtr::<f32>::from_addr(0x858B1C).read();
    if !X87::from_f32(b[1]).fcom(X87::from_f32(lim)).test_any(0x41) {
        wr::<u32>(out, 0x2C, 2);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_FX_INFO_TRAIL_GET_VALUE];
