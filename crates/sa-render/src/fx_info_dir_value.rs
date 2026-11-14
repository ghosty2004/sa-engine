//! `FxInfoDir_c::GetValue` (PEDHIT: the water of a knocked-over fire hydrant). Ported from the 1.0 US bytes; the
//! layout follows the other `FxInfo*_c::GetValue` ports (`fx_info_get_value_samp`).
#![allow(clippy::too_many_arguments)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{rd, wr};

orig! {
    /// `FxInterpInfoFloat_c::GetVal(float* out, float t)` (0x4A8470)
    fn interp_float_get_val(this: GPtr<u8>, out: GPtr<u8>, t: u32) = thiscall 0x4A8470 => unported;
    /// `RwV3dLength(const RwV3d*)` (0x7EDAC0): the length in ST0
    fn rw_v3d_length(v: GPtr<u8>) -> f64 = cdecl 0x7EDAC0 => unported;
}

/// `FxInfoDir_c::GetValue(float currTime, float mult, float timeStep, float totalTime, bool useConst, void* out)`
/// (0x4A64D0, thiscall, `ret 0x18`; vtable 0x85A914 slot 2): the key time is `mult` when byte +6 is set, else
/// `(float)(currTime / totalTime)`; `FxInterpInfoFloat_c::GetVal` (+8) fills a 16-float buffer; the render info's
/// direction +0x7C..+0x84 = key[0..2] (dword copies), flag +0x79 = 1, and flag +0x7A = 1 when
/// `RwV3dLength(direction) < 0.001f` (0x858CDC; NaN does not set it).
#[hook(addr = 0x4A64D0, conv = thiscall, name = "FxInfoDir_c::GetValue", expect = "8A 41 06 83 EC 40 84 C0",
       unverified = "pedhit-1", route = "L2", hazards = "heap-writes", recs = "driving",
       note = "writes the particle render info")]
pub fn fx_info_dir_get_value(this: GPtr<u8>, curr: u32, mult: u32, _step: u32, total: u32, _use_const: u32, out: GPtr<u8>) {
    let t = if rd::<u8>(this, 6) != 0 {
        mult
    } else {
        X87::from_f32(f32::from_bits(curr)).fdiv_m(f32::from_bits(total)).to_f32().to_bits()
    };
    let mut b = [0f32; 16];
    interp_float_get_val(this.byte_add(8), GPtr::from_raw(b.as_mut_ptr()).cast(), t);
    wr::<u32>(out, 0x7C, b[0].to_bits());
    wr::<u8>(out, 0x79, 1);
    wr::<u32>(out, 0x80, b[1].to_bits());
    wr::<u32>(out, 0x84, b[2].to_bits());
    let len = X87::from_f64_exact(rw_v3d_length(out.byte_add(0x7C)));
    if !len.fcom(X87::from_f32(0.001)).test_parity(5) {
        wr::<u8>(out, 0x7A, 1);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FX_INFO_DIR_GET_VALUE];
