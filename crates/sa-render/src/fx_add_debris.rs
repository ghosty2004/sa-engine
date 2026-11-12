//! `Fx_c::AddDebris` (batch `samp-render-3`), ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, k, rd};

orig! {
    /// `FxPrtMult_c::FxPrtMult_c()` (0x4AB270)
    fn fx_prt_mult_ctor(m: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AB270 => unported;
    /// `FxSystem_c::EnablePrim(int32 prim, bool enable)` (0x4AA610, thiscall)
    fn fx_system_enable_prim(s: GPtr<u8>, prim: u32, enable: u32) = thiscall 0x4AA610 => unported;
    /// `FxSystem_c::AddParticle(RwV3d* pos, RwV3d* vel, float timeSince, FxPrtMult_c*, float rotZ, float
    /// lightMult, float lightMultLimit, uint8 createLocal)` (0x4AA440, thiscall)
    fn fx_system_add_particle(s: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: u32, mult: GPtr<u8>, rot: u32,
        bright: u32, limit: u32, local: u32) = thiscall 0x4AA440 => unported;
}

/// `g_fx.m_fxSysDebris` (0xA9AE0C) and the debris primitive cursor `Fx_c::m_nDebrisCurrPrim`? (0xA9ADE4, 0..3).
const FX_SYS_DEBRIS: usize = 0xA9_AE0C;
const DEBRIS_PRIM: usize = 0xA9_ADE4;

/// `rand()` as the effect code scales it: `fild(rand()) * (1/32768)`; the call attributed to its original return
/// address.
fn rand_unit(ret: u32) -> X87 {
    X87::from_i32(sa_core::orig::at_site(ret, sa_core::crt::rand)).fmul_m(k(0x3800_0100))
}

/// `Fx_c::AddDebris(CVector* pos, RwRGBA& colour, float scale, int32 total)` (0x49F750, thiscall, `ret 0x10`):
/// nothing unless the squared camera distance `(dz² + dx²) + dy²` is at most 1265.0f (NaN passes). A
/// `FxPrtMult_c` with the colour bytes `* (1/255)`, size `scale`, +0x14 = `f32((rand() % 10000 * 0.0001 + 1) *
/// 0.5)` and +0x18 = 0.2f; then `total` times: velocity x and y `f32(rand() * (1/32768) * 0.5 * 20 - 5)`, z
/// `f32(rand() * (1/32768) * 0.15 * 20 + 2)`; on the debris system primitives 0..3 disabled and the current one
/// (`0xA9ADE4`) enabled; `AddParticle(pos, &vel, 0, &mult, -1.0f, 1.2f, 0.6f, 0)`; the current primitive steps to
/// the next of four.
#[hook(addr = 0x49F750, conv = thiscall, name = "Fx_c::AddDebris", expect = "A1 3C F0 B6 00",
       unverified = "samp-render-3", route = "L2", hazards = "heap-writes", recs = "weapon-combat",
       note = "rand calls named by return address; adds FX particles")]
pub fn fx_add_debris(_this: GPtr<u8>, pos: GPtr<u8>, colour: GPtr<u8>, scale: u32, total: i32) {
    let m = g::<u32>(0xB6_F03C).read();
    let cam = if m != 0 { GPtr::<u8>::from_addr(m as usize + 0x30) } else { g::<u8>(0xB6_F02C) };
    let dx = X87::from_f32(rd::<f32>(cam, 0)).fsub_m(rd::<f32>(pos, 0));
    let dy = X87::from_f32(rd::<f32>(cam, 4)).fsub_m(rd::<f32>(pos, 4));
    let dz = X87::from_f32(rd::<f32>(cam, 8)).fsub_m(rd::<f32>(pos, 8));
    if !dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy)).fcom(X87::from_f32(k(0x441C_4000))).test_any(0x41) {
        return;
    }
    let mut mult = [0u32; 7];
    let mp = GPtr::from_raw(mult.as_mut_ptr()).cast::<u8>();
    fx_prt_mult_ctor(mp);
    let field = |i: isize| mp.byte_add(4 * i).cast::<u32>();
    for i in 0..4 {
        field(i).write(X87::from_i32(i32::from(colour.byte_add(i).read())).fmul_m(k(0x3B80_8081)).to_f32().to_bits());
    }
    field(4).write(scale);
    field(6).write(0x3E4C_CCCD);
    let r = sa_core::orig::at_site(0x49_F814, sa_core::crt::rand).wrapping_rem(10000);
    field(5).write(X87::from_i32(r).fmul_m(k(0x38D1_B717)).fadd_m(k(0x3F80_0000)).fmul_m(k(0x3F00_0000)).to_f32().to_bits());
    if total <= 0 {
        return;
    }
    let mut n = total;
    loop {
        let mut vel = [0f32; 3];
        vel[0] = rand_unit(0x49_F855).fmul_m(k(0x3F00_0000)).fmul_m(k(0x41A0_0000)).fsub_m(k(0x40A0_0000)).to_f32();
        vel[1] = rand_unit(0x49_F87E).fmul_m(k(0x3F00_0000)).fmul_m(k(0x41A0_0000)).fsub_m(k(0x40A0_0000)).to_f32();
        vel[2] = rand_unit(0x49_F8A7).fmul_m(k(0x3E19_999A)).fmul_m(k(0x41A0_0000)).fadd_m(k(0x4000_0000)).to_f32();
        for prim in 0..4 {
            fx_system_enable_prim(g::<GPtr<u8>>(FX_SYS_DEBRIS).read(), prim, 0);
        }
        fx_system_enable_prim(g::<GPtr<u8>>(FX_SYS_DEBRIS).read(), g::<u32>(DEBRIS_PRIM).read(), 1);
        fx_system_add_particle(g::<GPtr<u8>>(FX_SYS_DEBRIS).read(), pos, GPtr::from_raw(vel.as_mut_ptr()).cast(), 0, mp,
            0xBF80_0000, 0x3F99_999A, 0x3F19_999A, 0);
        g::<u32>(DEBRIS_PRIM).write(g::<u32>(DEBRIS_PRIM).read().wrapping_add(1) & 3);
        n -= 1;
        if n == 0 {
            break;
        }
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_FX_ADD_DEBRIS];
