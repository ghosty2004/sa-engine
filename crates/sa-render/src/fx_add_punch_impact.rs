//! WRITER-NATIVE (batch `native-24`): `Fx_c::AddPunchImpact` (0x49F670), the dust puff of a landed punch.
//! Unreversed in gta-reversed; ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GameData};
use sa_core::{hook, orig};
use sa_math::x87::X87;

fn g<T: GameData>(a: u32) -> GPtr<T> {
    GPtr::from_addr(a as usize)
}

fn x(v: f32) -> X87 {
    X87::from_f32(v)
}

orig! {
    /// `FxPrtMult_c::FxPrtMult_c(r, g, b, a, size, spin, life)` (0x4AB290)
    #[allow(clippy::too_many_arguments)]
    fn fx_prt_mult_ctor(p: GPtr<u8>, r: f32, g: f32, b: f32, a: f32, size: f32, spin: f32, life: f32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(RwV3d*, RwV3d*, float, FxPrtMult_c*, float, float, float, uint8)` (0x4AA440)
    #[allow(clippy::too_many_arguments)]
    fn fx_add_particle(fx: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: f32, mult: GPtr<u8>, rot: f32, bright: f32, cam: f32, local: u32) = thiscall 0x4AA440 => unported;
}

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
    fx_add_particle(g::<GPtr<u8>>(0xA9_AE24).read(), pos.cast(), vel.cast(), 0.0, mp, -1.0, bright, camf, 0);
    fx_add_particle(g::<GPtr<u8>>(0xA9_AE24).read(), pos.cast(), vel.cast(), f32::from_bits(0x3D4C_CCCD), mp, -1.0, bright, camf, 0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FX_ADD_PUNCH_IMPACT];
