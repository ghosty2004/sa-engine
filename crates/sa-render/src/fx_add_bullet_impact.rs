//! `Fx_c::AddBulletImpact` (0x49F3D0, batch `samp2-fx-1`): the impact particles of every bullet hit.
//! Unreversed in gta-reversed; ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{ftol32, g, k, rd, wr, x, xi};

orig! {
    /// `SurfaceInfos_c::GetBulletFx(uint32 surface)` (0x55E670, thiscall on `g_surfaceInfos` 0xB79538)
    fn surface_get_bullet_fx(si: GPtr<u8>, surface: u32) -> i32 = thiscall 0x55E670 => unported;
    /// `Fx_c::AddSparks(...)` (0x49F040, thiscall `ret 0x28`; the across vector by value)
    #[allow(clippy::too_many_arguments)]
    fn fx_add_sparks(fx: GPtr<u8>, origin: GPtr<f32>, dir: GPtr<f32>, force: f32, amount: i32, ax: f32, ay: f32, az: f32, kind: u32, spread: f32, life: f32) = thiscall 0x49F040 => crate::fx_add_sparks::fx_add_sparks;
    /// `Fx_c::AddWood(CVector& pos, CVector& dir, int32 amount, float brightness)` (0x49EE10, thiscall)
    fn fx_add_wood(fx: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<u8>, amount: i32, b: f32) = thiscall 0x49EE10 => unported;
    /// `FxPrtMult_c::FxPrtMult_c(r, g, b, a, size, spin, life)` (0x4AB290)
    #[allow(clippy::too_many_arguments)]
    fn fx_prt_mult_ctor(p: GPtr<u8>, r: f32, g: f32, b: f32, a: f32, size: f32, spin: f32, life: f32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(RwV3d*, RwV3d*, float, FxPrtMult_c*, float, float, float, uint8)` (0x4AA440)
    #[allow(clippy::too_many_arguments)]
    fn fx_add_particle(sys: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: f32, mult: GPtr<u8>, rot: f32, bright: f32, near: f32, local: u32) = thiscall 0x4AA440 => unported;
}

/// `g_fx` (0xA9AE00) and its particle systems: sparks-impact smoke 0xA9AE24, dust 0xA9AE18.
const G_FX: usize = 0xA9_AE00;
const SYS_SMOKE: usize = 0xA9_AE24;
const SYS_DUST: usize = 0xA9_AE18;

/// `Fx_c::AddBulletImpact(CVector& pos, CVector& dir, int32 surface, int32 amount, float lighting)`
/// (0x49F3D0, thiscall `ret 0x14`; `this` unused).
///
/// `fx = g_surfaceInfos.GetBulletFx(surface)`; nothing for 0 or when the squared camera distance
/// `(dz² + dx²) + dy²` is above 22500 (NaN passes). Kind 1 (metal): `AddSparks(pos, dir, 3, amount, (0, 0, 0),
/// 1, 0.4, 1)`, a mult (1, 1, 1, 0.15, 0.4, 0, 0.075) whose alpha doubles for `amount >= 8` (one particle,
/// else two), particles `AddParticle(pos, dir, i * 0.05, mult, -1, lighting, 0.6, 0)` on 0xA9AE24. Kind 3
/// (wood): `AddWood(pos, dir, ftol(amount * 0.5), 1)`. Kinds 2 / 4 (dust, sand): a mult (0.81, 0.67, 0.57,
/// 0.15, 0.4, 0, 0.3), grey 0.6 for kind 4, alpha doubled for `amount >= 8` (two particles, else four),
/// particles with velocity `dir * 0.3` on 0xA9AE18. The original keeps the particle index in the `amount`
/// argument slot (stack only).
#[hook(addr = 0x49F3D0, conv = thiscall, name = "Fx_c::AddBulletImpact", this = "Fx_c",
       expect = "8B 44 24 0C 83 EC 28",
       unverified = "samp2-fx-1", route = "L2", hazards = "heap-writes", recs = "weapon-combat",
       note = "particles go to the FX pools; writes its own argument slot")]
pub fn fx_add_bullet_impact(_this: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<u8>, surface: u32, amount: i32, lighting: f32) {
    let fx = surface_get_bullet_fx(g(0xB7_9538), surface);
    let m = g::<u32>(0xB6_F03C).read();
    let cam = if m != 0 { GPtr::<u8>::from_addr(m as usize + 0x30) } else { g::<u8>(0xB6_F02C) };
    let dx = x(rd::<f32>(cam, 0)).fsub_m(rd::<f32>(pos, 0));
    let dy = x(rd::<f32>(cam, 4)).fsub_m(rd::<f32>(pos, 4));
    let dz = x(rd::<f32>(cam, 8)).fsub_m(rd::<f32>(pos, 8));
    if fx == 0 {
        return;
    }
    let d2 = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
    if !d2.fcom(x(k(0x46AF_C800))).test_any(0x41) {
        return;
    }
    let mut mult = [0u32; 8];
    let mp: GPtr<u8> = GPtr::from_raw(mult.as_mut_ptr()).cast();
    match fx {
        1 => {
            fx_add_sparks(g(G_FX), pos.cast(), dir.cast(), 3.0, amount, 0.0, 0.0, 0.0, 1, k(0x3ECC_CCCD), 1.0);
            fx_prt_mult_ctor(mp, 1.0, 1.0, 1.0, k(0x3E19_999A), k(0x3ECC_CCCD), 0.0, k(0x3D99_999A));
            let n = if amount >= 8 {
                wr::<f32>(mp, 0xC, x(rd::<f32>(mp, 0xC)).fadd(x(rd::<f32>(mp, 0xC))).to_f32());
                1
            } else {
                2
            };
            for i in 0..n {
                let t = xi(i).fmul_m(k(0x3D4C_CCCD)).to_f32();
                fx_add_particle(g::<GPtr<u8>>(SYS_SMOKE).read(), pos, dir, t, mp, -1.0, lighting, k(0x3F19_999A), 0);
            }
        }
        2 | 4 => {
            fx_prt_mult_ctor(mp, k(0x3F4F_5C29), k(0x3F2B_851F), k(0x3F11_EB85), k(0x3E19_999A), k(0x3ECC_CCCD), 0.0,
                k(0x3E99_999A));
            if fx == 4 {
                wr::<f32>(mp, 8, k(0x3F19_999A));
                wr::<f32>(mp, 4, k(0x3F19_999A));
                wr::<f32>(mp, 0, k(0x3F19_999A));
            }
            let n = if amount >= 8 {
                wr::<f32>(mp, 0xC, x(rd::<f32>(mp, 0xC)).fadd(x(rd::<f32>(mp, 0xC))).to_f32());
                2
            } else {
                4
            };
            for i in 0..n {
                let mut vel = [
                    x(rd::<f32>(dir, 0)).fmul_m(k(0x3E99_999A)).to_f32(),
                    x(rd::<f32>(dir, 4)).fmul_m(k(0x3E99_999A)).to_f32(),
                    x(rd::<f32>(dir, 8)).fmul_m(k(0x3E99_999A)).to_f32(),
                ];
                let t = xi(i).fmul_m(k(0x3D4C_CCCD)).to_f32();
                fx_add_particle(g::<GPtr<u8>>(SYS_DUST).read(), pos, GPtr::from_raw(vel.as_mut_ptr()).cast(), t, mp,
                    -1.0, lighting, k(0x3F19_999A), 0);
            }
        }
        3 => {
            let n = ftol32(xi(amount).fmul_m(0.5));
            fx_add_wood(g(G_FX), pos, dir, n, 1.0);
        }
        _ => {}
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_FX_ADD_BULLET_IMPACT];
