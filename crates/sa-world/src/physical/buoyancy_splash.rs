//! `cBuoyancy::AddSplashParticles` (0x6C34E0, batch `samp2-phys-1`): the water splash along a hull edge
//! and the swimming wake. Ported from the 1.0 US bytes (gta-reversed `Buoyancy.cpp` as a reading aid).
//! Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::shared::*;
use crate::world::common::{ftol32, x};

use sa_math::fpu::st0;

use crate::bindings::get_task_swim;
use sa_core::bindings::matrix_mul_vec;
use sa_core::bindings::vector_normalise as normalise;
orig! {
    /// `FxPrtMult_c::FxPrtMult_c(r, g, b, a, size, spin, life)` (0x4AB290)
    #[allow(clippy::too_many_arguments)]
    fn fx_prt_mult_ctor(p: GPtr<u8>, r: f32, g: f32, b: f32, a: f32, size: f32, spin: f32, life: f32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(RwV3d*, RwV3d*, float, FxPrtMult_c*, float, float, float, uint8)` (0x4AA440)
    #[allow(clippy::too_many_arguments)]
    fn fx_add_particle(sys: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: f32, mult: GPtr<u8>, rot: f32, bright: f32, near: f32, local: u32) = thiscall 0x4AA440 => unported;
    /// `CGeneral::GetAngleBetweenPoints(x1, y1, x2, y2)` (0x53CEA0, ST0)
    fn get_angle_between_points(x1: f32, y1: f32, x2: f32, y2: f32) -> f64 = cdecl 0x53CEA0 => unported;
    /// `CGeneral::LimitAngle(float)` (0x53CB00, ST0)
    fn limit_angle(a: f32) -> f64 = cdecl 0x53CB00 => unported;
    /// `CAEPedAudioEntity::AddAudioEvent(int32, float, float, CPhysical*, uint8, int32, uint32)` (0x4E2BB0)
    #[allow(clippy::too_many_arguments)]
    fn ped_audio_add_event(ae: GPtr<u8>, ev: i32, vol: f32, speed: f32, e: GPtr<u8>, a: u32, b: i32, c: u32) = thiscall 0x4E2BB0 => unported;
    /// `CAudioEngine::ReportWaterSplash(CPhysical*, float volume, bool)` (0x506F00, thiscall)
    fn audio_report_water_splash(ae: GPtr<u8>, e: GPtr<u8>, vol: f32, b: u32) = thiscall 0x506F00 => unported;
}

/// The speed cap 0.1 (0x8D3388, `.data`).
const SPEED_CAP: usize = 0x8D_3388;

use sa_core::field::gf;

use sa_core::field::ptr_of as p3;

/// `cBuoyancy::AddSplashParticles(CPhysical* e, CVector from, CVector to, CVector dir, uint8 reduce)` (0x6C34E0,
/// thiscall `ret 0x2C`; `this` unused; entry `nop; jmp` into the chunk 0x401C83, which does the `sub esp,
/// 0xE0`).
///
/// `d = to - from` (f32), `len = sqrt(dz*dz' + dy² + dx²)` (dz at 80 bits times its f32). The move speed s
/// (f32 copy) and `m = |s|`; above 0.1: `s = (s / m) * 0.1` per component (x at 80 bits), m = 0.1. Nothing
/// unless `m > 0.05`. A mult (1, 1, 1, 0.1, 0.5, 1, 0.2) (size and life halved with `reduce`); `vmod = (s.x * 0
/// * 120, s.y * 0 * 120, (f32)(s.z * -1) * 120)`; `zero = moveSpeed * 0` (the entity's own speed, signed
/// zeros / NaN kept). For `n = max(ftol(len * 2), 1)` points (i / (float)n): `P = M * (from + d * t)`; a
/// non-ped sets `dir = P - pos` (written over its argument); `dir.z = 0`, `Normalise(dir)`; `V = ((dir *
/// m) * 60 + vmod) + zero`; `f = rand() / 32767 * 0.5`, `P += dir * f` (x at 80 bits); `AddParticle(P, V,
/// 0, mult, -1, 1.2, 0.6, 0)` on 0xA9AE38. A ped: angle `LimitAngle(GetAngleBetweenPoints(fwd.x, fwd.y, 0,
/// 0)) + 180`, a mult (1, 1, 1, 0.2, 0.4, 0, 0.5), the point `pos + fwd * 0.4` (no matrix: `(-sin h, cos h,
/// 0)`); without a swim task: z = the player data's water height (else + 0.5), a wake particle on
/// 0xA9AE34 with the angle, ped audio event 0x4C. Finally `ReportWaterSplash(e, -100, false)`.
#[hook(addr = 0x6C34E0, conv = thiscall, name = "cBuoyancy::AddSplashParticles", this = "cBuoyancy",
       expect = "90 E9 9D E7 D3 FF",
       unverified = "samp2-phys-1", route = "L2", hazards = "heap-writes,x87-transcendental",
       recs = "boat",
       note = "entry nop + jmp into the chunk 0x401C83; rand at 0x6C38B4 per particle; writes its dir argument slots")]
#[allow(clippy::too_many_arguments)]
pub fn add_splash_particles(_this: GPtr<u8>, e: GPtr<u8>, fx: f32, fy: f32, fz: f32, tx: f32, ty: f32, tz: f32,
    dx_in: f32, dy_in: f32, dz_in: f32, reduce: u32) {
    let dx = x(tx).fsub_m(fx).to_f32();
    let dy = x(ty).fsub_m(fy).to_f32();
    let dz80 = x(tz).fsub_m(fz);
    let dz = dz80.to_f32();
    let len = dz80.fmul_m(dz).fadd(x(dy).fmul_m(dy)).fadd(x(dx).fmul_m(dx)).fsqrt().to_f32();
    let mut s = [rf(e, 0x44), rf(e, 0x48), rf(e, 0x4C)];
    let mm = x(s[2]).fmul_m(s[2]).fadd(x(s[1]).fmul_m(s[1])).fadd(x(s[0]).fmul_m(s[0])).fsqrt();
    let mut m = mm.to_f32();
    let mut mcmp = mm;
    if !mm.fcom(x(gf(SPEED_CAP))).test_any(0x41) {
        let inv = mm.fdivr_m(1.0);
        let sx = x(s[0]).fmul(inv);
        let sy = x(s[1]).fmul(inv).to_f32();
        let sz = x(s[2]).fmul(inv).to_f32();
        s[0] = sx.fmul_m(gf(SPEED_CAP)).to_f32();
        s[1] = x(sy).fmul_m(gf(SPEED_CAP)).to_f32();
        s[2] = x(sz).fmul_m(gf(SPEED_CAP)).to_f32();
        mcmp = x(gf(SPEED_CAP));
        m = mcmp.to_f32();
    }
    if mcmp.fcom(x(f32::from_bits(0x3D4C_CCCD))).test_any(0x41) {
        return;
    }
    let mut mult = [0u32; 8];
    let mp: GPtr<u8> = GPtr::from_raw(mult.as_mut_ptr()).cast();
    fx_prt_mult_ctor(mp, 1.0, 1.0, 1.0, cf(0x3DCC_CCCD), 0.5, 1.0, cf(0x3E4C_CCCD));
    if reduce as u8 != 0 {
        wf(mp, 0x10, x(rf(mp, 0x10)).fmul_m(0.5).to_f32());
        wf(mp, 0x18, x(rf(mp, 0x18)).fmul_m(0.5).to_f32());
    }
    let n0 = ftol32(x(len).fadd(x(len)));
    let n = if n0 < 1 { 1 } else { n0 };
    let a = x(s[0]).fmul_m(0.0);
    let b = x(s[1]).fmul_m(0.0);
    let c = x(s[2]).fmul_m(-1.0).to_f32();
    let vmod = [a.fmul_m(120.0).to_f32(), b.fmul_m(120.0).to_f32(), x(c).fmul_m(120.0).to_f32()];
    let zz = x(rf(e, 0x4C)).fmul_m(0.0).to_f32();
    let zero = [x(rf(e, 0x44)).fmul_m(0.0).to_f32(), x(rf(e, 0x48)).fmul_m(0.0).to_f32(), x(zz).fmul_m(0.0).to_f32()];
    let nf = X87::from_i32(n).to_f32();
    let mut dir = [dx_in, dy_in, dz_in];
    for i in 0..n {
        let t = X87::from_i32(i).fdiv(x(nf));
        let px = x(dx).fmul(t);
        let py = x(dy).fmul(t).to_f32();
        let pz = x(dz).fmul(t).to_f32();
        let mut v = [px.fadd_m(fx).to_f32(), x(py).fadd_m(fy).to_f32(), x(pz).fadd_m(fz).to_f32()];
        let mut p = [0f32; 3];
        matrix_mul_vec(p3(&mut p), rp(e, 0x14), p3(&mut v));
        if etype(e) != 3 {
            let pos = pos_ptr(e);
            dir = [x(p[0]).fsub_m(rf(pos, 0)).to_f32(), x(p[1]).fsub_m(rf(pos, 4)).to_f32(),
                x(p[2]).fsub_m(rf(pos, 8)).to_f32()];
        }
        dir[2] = 0.0;
        normalise(p3(&mut dir));
        let dxm = x(dir[0]).fmul_m(m);
        let dym = x(dir[1]).fmul_m(m);
        let dzm = x(dir[2]).fmul_m(m).to_f32();
        let dxm60 = dxm.fmul_m(60.0).to_f32();
        let dym60 = dym.fmul_m(60.0);
        let dzm60 = x(dzm).fmul_m(60.0);
        let vx = x(dxm60).fadd_m(vmod[0]).to_f32();
        let vy = dym60.fadd_m(vmod[1]).to_f32();
        let vz = dzm60.fadd_m(vmod[2]);
        let mut vel = [x(vx).fadd_m(zero[0]).to_f32(), x(vy).fadd_m(zero[1]).to_f32(), vz.fadd_m(zero[2]).to_f32()];
        let r = sa_core::orig::at_site(0x6C_38B4, sa_core::crt::rand);
        let f = X87::from_i32(r).fmul_m(cf(0x3800_0100)).fmul_m(0.5);
        let dxf = x(dir[0]).fmul(f);
        let dyf = x(dir[1]).fmul(f).to_f32();
        let dzf = x(dir[2]).fmul(f).to_f32();
        p[0] = dxf.fadd_m(p[0]).to_f32();
        p[1] = x(dyf).fadd_m(p[1]).to_f32();
        p[2] = x(dzf).fadd_m(p[2]).to_f32();
        fx_add_particle(GPtr::<GPtr<u8>>::from_addr(0xA9_AE38).read(), p3(&mut p), p3(&mut vel), 0.0, mp, -1.0,
            cf(0x3F99_999A), cf(0x3F19_999A), 0);
    }
    if etype(e) == 3 {
        let mt = rp(e, 0x14);
        let ang = st0(get_angle_between_points(rf(mt, 0x10), rf(mt, 0x14), 0.0, 0.0)).to_f32();
        let ang = st0(limit_angle(ang)).fadd_m(180.0).to_f32();
        let mut wake = [0u32; 8];
        let wp_: GPtr<u8> = GPtr::from_raw(wake.as_mut_ptr()).cast();
        fx_prt_mult_ctor(wp_, 1.0, 1.0, 1.0, cf(0x3E4C_CCCD), cf(0x3ECC_CCCD), 0.0, 0.5);
        let mut zero_v = [0f32; 3];
        let mt = rp(e, 0x14);
        let (fwx, fwy, fwz) = if !mt.is_null() {
            (x(rf(mt, 0x10)), x(rf(mt, 0x14)), rf(mt, 0x18))
        } else {
            (x(rf(e, 0x10)).fsin().fchs(), x(rf(e, 0x10)).fcos(), 0.0)
        };
        let ox = fwx.fmul_m(cf(0x3ECC_CCCD)).to_f32();
        let oy = fwy.fmul_m(cf(0x3ECC_CCCD)).to_f32();
        let oz = x(fwz).fmul_m(cf(0x3ECC_CCCD));
        let pos = pos_ptr(e);
        let mut p = [x(ox).fadd_m(rf(pos, 0)).to_f32(), x(oy).fadd_m(rf(pos, 4)).to_f32(), oz.fadd_m(rf(pos, 8)).to_f32()];
        if get_task_swim(rp(e, 0x47C)).is_null() {
            let pd = rp(e, 0x480);
            if !pd.is_null() {
                p[2] = rf(pd, 0x90);
            } else {
                p[2] = x(p[2]).fadd_m(0.5).to_f32();
            }
            fx_add_particle(GPtr::<GPtr<u8>>::from_addr(0xA9_AE34).read(), p3(&mut p), p3(&mut zero_v), 0.0, wp_,
                ang, cf(0x3F99_999A), cf(0x3F19_999A), 0);
            ped_audio_add_event(e.byte_add(0x138), 0x4C, 0.0, 1.0, GPtr::NULL, 0, 0, 0);
        }
    }
    audio_report_water_splash(GPtr::from_addr(0xB6_BC90), e, -100.0, 0);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_ADD_SPLASH_PARTICLES];
