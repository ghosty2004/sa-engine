//! PORT-VEH-3 (batch `veh3-3`, SA:MP-executed rows): the particles of a car hitting something
//! (`CAutomobile::dmgDrawCarCollidingParticles`). Ported from the 1.0 US bytes; gta-reversed as a
//! reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{ftol32, g, rd, x, xi};

orig! {
    /// `CEntity::GetIsOnScreen()` (0x534540)
    fn entity_get_is_on_screen(e: GPtr<u8>) -> u8 = thiscall 0x534540 => unported;
    /// `CVector::NormaliseAndMag()` (0x59C970), ST0
    fn vector_normalise_and_mag(v: GPtr<u8>) -> f64 = thiscall 0x59C970 => unported;
    /// `Fx_c::AddSparks(CVector&, CVector&, float, int32, CVector, eSparkType, float, float)` (0x49F040) on `g_fx` (0xA9AE00)
    fn fx_add_sparks(fx: GPtr<u8>, origin: GPtr<u8>, dir: GPtr<u8>, force: f32, amount: i32, across: CVector, kind: u32, spread: f32, life: f32) = thiscall 0x49F040 => unported;
    /// `FxPrtMult_c::FxPrtMult_c(float, float, float, float, float, float, float)` (0x4AB290)
    fn fx_prt_mult_ctor(p: GPtr<u8>, r: f32, g: f32, b: f32, a: f32, size: f32, spin: f32, life: f32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(RwV3d*, RwV3d*, float, FxPrtMult_c*, float, float, float, uint8)` (0x4AA440)
    fn fx_add_particle(fx: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: f32, mult: GPtr<u8>, rot: f32, bright: f32, cam: f32, local: u32) = thiscall 0x4AA440 => unported;
    /// `Fx_c::AddDebris(CVector&, RwRGBA&, float, int32)` (0x49F750) on `g_fx`
    fn fx_add_debris(fx: GPtr<u8>, pos: GPtr<u8>, col: GPtr<u8>, scale: f32, amount: i32) = thiscall 0x49F750 => unported;
}

/// `CAutomobile::dmgDrawCarCollidingParticles(const CVector& pos, float force, eWeaponType)`
/// (0x6A6DC0): only on screen. Unless the weapon is 0 or 14: sparks at `pos` along the normalised
/// move speed (`amount = (ftol(force) / 10 + 4) & 63`, force = |move speed| x -10). Then smoke
/// particles (`max(1, ftol(|timestep x move speed| x 4))`, `g_fx` system 0xA9AE20) at 70% of the way
/// from the car to `pos`, and with |move speed|^2 above 0.0625 debris of the car's first colour
/// scaled by its lighting (+0x12C), `ftol(force) / 100 + 1` pieces.
#[hook(addr = 0x6A6DC0, conv = thiscall, name = "CAutomobile::dmgDrawCarCollidingParticles", this = "CAutomobile",
       expect = "83 EC 4C 53 55",
       unverified = "veh3-3", route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "driving",
       note = "fsqrt; creates particles (fx pools); the original keeps the spark force and the debris colour in its argument slots")]
pub fn automobile_dmg_draw_car_colliding_particles(this: GPtr<u8>, pos: GPtr<u8>, force: f32, weapon: u32) {
    if entity_get_is_on_screen(this) == 0 {
        return;
    }
    let f = ftol32(x(force));
    let fx: GPtr<u8> = GPtr::from_addr(0xA9_AE00);
    if weapon != 0 && weapon != 0xE {
        let mut dir = [rd::<u32>(this, 0x44), rd::<u32>(this, 0x48), rd::<u32>(this, 0x4C)];
        let dp: GPtr<u8> = GPtr::from_raw(&mut dir as *mut [u32; 3]).cast();
        let mag = X87::from_f64_exact(vector_normalise_and_mag(dp));
        let mut p = [rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8)];
        let across = CVector::new(rd::<f32>(this, 0x44), rd::<f32>(this, 0x48), rd::<f32>(this, 0x4C));
        let amount = ((f / 10) + 4) & 0x3F;
        let sf = mag.fmul_m(-10.0).to_f32();
        fx_add_sparks(fx, GPtr::from_raw(&mut p as *mut [u32; 3]).cast(), dp, sf, amount, across, 1, f32::from_bits(0x3E99_999A), 1.0);
    }
    let mut mult = [0u32; 8];
    let mp: GPtr<u8> = GPtr::from_raw(&mut mult as *mut [u32; 8]).cast();
    let c4 = f32::from_bits(0x3ECC_CCCD);
    fx_prt_mult_ctor(mp, c4, c4, c4, f32::from_bits(0x3F19_999A), c4, 1.0, f32::from_bits(0x3DCC_CCCD));
    let mut vel = [0u32; 3];
    let m = rd::<GPtr<u8>>(this, 0x14);
    let vp = if m.is_null() { this.byte_add(4) } else { m.byte_add(0x30) };
    let a = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(vp, 0));
    let b = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(vp, 4));
    let c = x(rd::<f32>(pos, 8)).fsub_m(rd::<f32>(vp, 8)).to_f32();
    let k = f32::from_bits(0x3F33_3333);
    let ax = a.fmul_m(k).to_f32();
    let by = b.fmul_m(k).to_f32();
    let cz = x(c).fmul_m(k);
    let mut pt = [
        x(ax).fadd_m(rd::<f32>(vp, 0)).to_f32(),
        x(by).fadd_m(rd::<f32>(vp, 4)).to_f32(),
        cz.fadd_m(rd::<f32>(vp, 8)).to_f32(),
    ];
    let ptp: GPtr<u8> = GPtr::from_raw(&mut pt as *mut [f32; 3]).cast();
    let ts = g::<f32>(0xB7_CB5C).read();
    let sx = x(ts).fmul_m(rd::<f32>(this, 0x44));
    let sy = x(ts).fmul_m(rd::<f32>(this, 0x48));
    let sz = x(ts).fmul_m(rd::<f32>(this, 0x4C));
    let len = sz.fmul(sz).fadd(sy.fmul(sy)).fadd(sx.fmul(sx)).fsqrt();
    let n = ftol32(len.fmul_m(4.0)).max(1);
    for _ in 0..n {
        fx_add_particle(
            g::<GPtr<u8>>(0xA9_AE20).read(),
            ptp,
            GPtr::from_raw(&mut vel as *mut [u32; 3]).cast(),
            0.0,
            mp,
            -1.0,
            f32::from_bits(0x3F99_999A),
            f32::from_bits(0x3F19_999A),
            0,
        );
    }
    let mx = x(rd::<f32>(this, 0x44));
    let my = x(rd::<f32>(this, 0x48));
    let mz = x(rd::<f32>(this, 0x4C));
    let s2 = mx.fmul(mx).fadd(my.fmul(my)).fadd(mz.fmul(mz));
    if s2.fcom(x(0.0625)).test_any(0x41) {
        return;
    }
    let mut col = g::<u32>(0xB4_E480 + 4 * usize::from(rd::<u8>(this, 0x434))).read().to_le_bytes();
    let light = rd::<f32>(this, 0x12C);
    for ch in col.iter_mut().take(3) {
        *ch = ftol32(xi(i32::from(*ch)).fmul_m(light)) as u8;
    }
    fx_add_debris(fx, ptp, GPtr::from_raw(&mut col as *mut [u8; 4]).cast(), f32::from_bits(0x3D75_C28F), f / 100 + 1);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_DMG_DRAW_CAR_COLLIDING_PARTICLES];
