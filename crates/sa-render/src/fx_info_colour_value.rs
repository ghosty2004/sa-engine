//! STD-15 (batch `std15-render-3`): the colour-type `FxInfo` `GetValue` (0x4A6860, vtable slot 2), reached by the
//! wheel / crash particles while driving. Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{rd, wr, Fpu};

orig! {
    /// `FxInterpInfoU255_c::GetVal(float* out, float t)` (0x4A8800, `ret 8`)
    fn interp_u255_get_val(this: GPtr<u8>, out: GPtr<u8>, t: u32) = thiscall 0x4A8800 => unported;
}

/// `FUN_004a6860(float currTime, float mult, float timeStep, float totalTime, bool useConst, void* out)` (0x4A6860,
/// thiscall, `ret 0x18`): key time `mult` with byte +6 set, else `(float)(currTime / totalTime)`;
/// `FxInterpInfoU255_c::GetVal` into a 16-float buffer; `out[0..4] = _ftol(key[0..4])` (low bytes), `out[7] =
/// _ftol(key[4])`, `out +8 = 2`.
#[hook(addr = 0x4A6860, conv = thiscall, name = "FxInfoColourBright_c::GetValue", expect = "8A 41 06 83 EC 40",
       unverified = "std15-render-3", route = "L2", hazards = "heap-writes", recs = "car-drive",
       note = "writes the particle render info")]
pub fn fx_info_colour_get_value(
    this: GPtr<u8>,
    curr: u32,
    mult: u32,
    _step: u32,
    total: u32,
    _use_const: u32,
    out: GPtr<u8>,
) {
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
    let l = |v: f32| X87::from_f32(v).ftol() as u8;
    wr::<u8>(out, 0, l(b[0]));
    wr::<u8>(out, 1, l(b[1]));
    wr::<u8>(out, 2, l(b[2]));
    wr::<u8>(out, 3, l(b[3]));
    wr::<u8>(out, 7, l(b[4]));
    wr::<u32>(out, 8, 2);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_FX_INFO_COLOUR_GET_VALUE];
