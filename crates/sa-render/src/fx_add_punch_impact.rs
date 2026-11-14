//! WRITER-NATIVE (batch `native-24`): `Fx_c::AddPunchImpact` (0x49F670), the dust puff of a landed punch.
//! Unreversed in gta-reversed; ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;

use sa_core::field::gu as g;

use sa_math::fpu::x;

use crate::bindings::fx_add_particle;
use sa_world::bindings::fx_prt_mult_ctor;

/// `Fx_c::AddPunchImpact(const CVector& pos, const CVector& velocity, int32 num)` (0x49F670, `ret 0xC`; `this`
/// and `num` unused).
///
/// Nothing when the camera (`TheCamera.m_matrix` 0xB6F03C, else its placement position) is farther than 25 m
/// (`(dx^2 + dz^2) + dy^2 > 625`, ordered). Otherwise `FxPrtMult_c(1, 1, 1, 0.4, 0.1, 0, 0.1)` and two particles
/// of the system at 0xA9AE24 at `pos` with `velocity`, times 0 and 0.05 (rotation -1, brightness 1.2, camera 0.6).
#[hook(addr = 0x49F670, conv = thiscall, name = "Fx_c::AddPunchImpact", this = "Fx_c",
       expect = "A1 3C F0 B6 00",
       unverified = "native-24", route = "L2", hazards = "heap-writes", recs = "weapon-combat",
       note = "particles go to the FX pools")]
pub fn fx_add_punch_impact(_this: GPtr<u8>, pos: GPtr<f32>, vel: GPtr<f32>, _num: i32) {
    let m = g::<u32>(0xB6_F03C).read();
    let cam: GPtr<f32> = if m != 0 { GPtr::from_addr(m as usize + 0x30) } else { g::<f32>(0xB6_F02C) };
    let dx = x(cam.read()).fsub_m(pos.read());
    let dy = x(cam.wrapping_index(1).read()).fsub_m(pos.wrapping_index(1).read());
    let dz = x(cam.wrapping_index(2).read()).fsub_m(pos.wrapping_index(2).read());
    let d2 = dx.fmul(dx).fadd(dz.fmul(dz)).fadd(dy.fmul(dy));
    if !d2.fcom(x(625.0)).test_any(0x41) {
        return;
    }
    let mut mult = [0u32; 8];
    let mp: GPtr<u8> = GPtr::from_raw(&mut mult as *mut [u32; 8]).cast();
    let tenth = f32::from_bits(0x3DCC_CCCD);
    fx_prt_mult_ctor(mp, 1.0, 1.0, 1.0, f32::from_bits(0x3ECC_CCCD), tenth, 0.0, tenth);
    let (bright, camf) = (f32::from_bits(0x3F99_999A), f32::from_bits(0x3F19_999A));
    fx_add_particle(g::<GPtr<u8>>(0xA9_AE24).read(), pos.cast(), vel.cast(), 0.0f32.to_bits(), mp, (-1.0f32).to_bits(), bright, camf, 0);
    fx_add_particle(g::<GPtr<u8>>(0xA9_AE24).read(), pos.cast(), vel.cast(), f32::from_bits(0x3D4C_CCCD).to_bits(), mp, (-1.0f32).to_bits(), bright, camf, 0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FX_ADD_PUNCH_IMPACT];
