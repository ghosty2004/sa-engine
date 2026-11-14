//! `Fx_c::AddBlood` (0x49EB00, batch `frontier-1-2`): the blood particles and pools of a punch / shot that hurts a
//! ped. Unreversed in gta-reversed; ported from the 1.0 US bytes, `rand()` at its original call sites.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig::at_site;
use sa_world::world::common::cam_pos;

use crate::common::{
    fx_add_particle, fx_prt_mult_ctor, g, localisation_blood, rd, shadows_add_permanent_shadow, wr, x, xi,
};

/// `rand()` at its call site in 0x49EB00 (the return address).
fn rand_at(site: u32) -> i32 {
    at_site(site, sa_core::crt::rand)
}

/// `(rand() % 10000) * [0x858FC4]` as the x87 value the loop builds on.
fn frac(site: u32) -> sa_math::x87::X87 {
    xi(rand_at(site) % 10000).fmul_m(g::<f32>(0x85_8FC4).read())
}

/// `Fx_c::AddBlood(CVector& pos, CVector& dir, int32 amount, float lightMult)` (0x49EB00, thiscall, `ret 0x10`).
/// Nothing without `CLocalisation::Blood()` or beyond the squared camera distance `[0x85A6E8]`
/// (`(dx² + dz²) + dy²`). A `FxPrtMult_c(0.5, 0, 0, 1, 0.8, 0, 0.8)`; each of `amount` particles gets the size
/// `r * k * [0x858C24] + [0x858CB0]` and the velocity `dir * [0x858CE8]` plus `2 r k - 1` on x, y and z (four
/// draws), and goes to the blood system (`g_fx` +0, 0xA9AE00) at `pos` (time 0, rotation -1, brightness
/// `lightMult`, near 0.6). Then the pool position `pos + dir * 0.5`, x and y jittered by `r k [0x858CC4] -
/// [0x858B1C]`, z + 1; the counter +0x50 advances and on (counter & 7) == 5 / 2 a permanent blood shadow (texture
/// `[0xC40400]`, 0.1 / 0.2 wide, red 200, intensity 255, distance 4, life 2000 / 8000 + `rand() & 0xFFF`).
///
/// Stack: the original stores two rand temporaries into the caller's `pos` argument slot; not reproduced.
#[hook(addr = 0x49EB00, conv = thiscall, name = "Fx_c::AddBlood", this = "Fx_c",
       expect = "83 EC 38 55 8B E9",
       unverified = "frontier-1-2", route = "L2", hazards = "heap-writes,stack-residue",
       note = "rand at_site 0x49EBA5/0x49EBF6/0x49EC21/0x49EC4C per particle, 0x49ECDF/0x49ED0E, 0x49ED63 or 0x49EDBB; the pos argument slot holds rand temporaries in the original")]
pub fn fx_add_blood(this: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<u8>, amount: i32, light: f32) {
    if localisation_blood() == 0 {
        return;
    }
    let cam = cam_pos();
    let c = |i: i32| cam.wrapping_index(i).read();
    let dx = x(c(0)).fsub_m(rd::<f32>(pos, 0));
    let dy = x(c(1)).fsub_m(rd::<f32>(pos, 4));
    let dz = x(c(2)).fsub_m(rd::<f32>(pos, 8));
    let sq = dx.fmul(dx).fadd(dz.fmul(dz)).fadd(dy.fmul(dy));
    if !sq.fcom(x(g::<f32>(0x85_A6E8).read())).test_any(0x41) {
        return;
    }
    let mut mult = [0u32; 8];
    let mp: GPtr<u8> = GPtr::from_raw(mult.as_mut_ptr()).cast();
    fx_prt_mult_ctor(mp, 0.5, 0.0, 0.0, 1.0, f32::from_bits(0x3F4C_CCCD), 0.0, f32::from_bits(0x3F4C_CCCD));
    let one = g::<f32>(0x85_8624).read();
    for _ in 0..amount.max(0) {
        let size = frac(0x49_EBA5)
            .fmul_m(g::<f32>(0x85_8C24).read())
            .fadd_m(g::<f32>(0x85_8CB0).read())
            .to_f32();
        wr::<f32>(mp, 0x10, size);
        let k = g::<f32>(0x85_8CE8).read();
        let mut v = [
            x(rd::<f32>(dir, 0)).fmul_m(k).to_f32(),
            x(rd::<f32>(dir, 4)).fmul_m(k).to_f32(),
            x(rd::<f32>(dir, 8)).fmul_m(k).to_f32(),
        ];
        let f = frac(0x49_EBF6);
        v[0] = f.fadd(f).fsub_m(one).fadd_m(v[0]).to_f32();
        let f = frac(0x49_EC21);
        v[1] = f.fadd(f).fsub_m(one).fadd_m(v[1]).to_f32();
        let f = frac(0x49_EC4C);
        let sys = g::<GPtr<u8>>(0xA9_AE00).read();
        v[2] = f.fadd(f).fsub_m(one).fadd_m(v[2]).to_f32();
        let vp: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr()).cast();
        fx_add_particle(sys, pos, vp, 0.0, mp, -1.0, light, f32::from_bits(0x3F19_999A), 0);
    }
    let h = g::<f32>(0x85_8B8C).read();
    let hz = x(rd::<f32>(dir, 8)).fmul_m(h).to_f32();
    let mut p = [
        x(rd::<f32>(dir, 0)).fmul_m(h).fadd_m(rd::<f32>(pos, 0)).to_f32(),
        x(rd::<f32>(dir, 4)).fmul_m(h).fadd_m(rd::<f32>(pos, 4)).to_f32(),
        x(hz).fadd_m(rd::<f32>(pos, 8)).to_f32(),
    ];
    let (kj, kk) = (g::<f32>(0x85_8CC4).read(), g::<f32>(0x85_8B1C).read());
    p[0] = frac(0x49_ECDF).fmul_m(kj).fsub_m(kk).fadd_m(p[0]).to_f32();
    let r6 = rand_at(0x49_ED0E) % 10000;
    let n = rd::<u32>(this, 0x50).wrapping_add(1);
    wr::<u32>(this, 0x50, n);
    p[1] = xi(r6).fmul_m(g::<f32>(0x85_8FC4).read()).fmul_m(kj).fsub_m(kk).fadd_m(p[1]).to_f32();
    p[2] = x(p[2]).fadd_m(one).to_f32();
    let pp: GPtr<u8> = GPtr::from_raw(p.as_mut_ptr()).cast();
    let (w, life, site) = match n & 7 {
        5 => (0x3DCC_CCCD_u32, 2000u32, 0x49_ED63),
        2 => (0x3E4C_CCCD, 8000, 0x49_EDBB),
        _ => return,
    };
    let time = (rand_at(site) as u32 & 0xFFF) + life;
    shadows_add_permanent_shadow(
        1,
        g::<u32>(0xC4_0400).read(),
        pp,
        f32::from_bits(w),
        0.0,
        0.0,
        -f32::from_bits(w),
        0xFF,
        0xC8,
        0,
        0,
        4.0,
        time,
        1.0,
    );
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FX_ADD_BLOOD];
