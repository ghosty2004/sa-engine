//! STD-15 (batch `std15-render-2`): `Fx_c::AddDebris` (0x49F750), `Fx_c::AddWheelGrass` (0x49FF20) and the
//! stunt-jump pool's `GetAt` (0x49C3B0, a stub into `.HOODLUM` 0x156ACC0), reached while driving. Unreversed in
//! gta-reversed; ported from the 1.0 US bytes, `rand()` at its original call sites. Unverified (docs/verify/HOWTO.md).
//! FRONTIER-2 (batch `frontier2-5`, user-hit driving) adds `AddWheelGravel` / `AddWheelMud` (the grass code with other
//! colours) and `AddWheelSand` / `AddWheelDust` (0x4A0170, 0x4A03C0, 0x4A0610, 0x4A09C0).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;

use sa_core::field::gu as g;
fn gf(a: u32) -> f32 {
    g::<f32>(a).read()
}
use sa_math::fpu::x;
use sa_math::fpu::xi;
use sa_math::fpu::ftol32;
use sa_core::crt::rand_at;

use crate::bindings::fx_prt_mult_default_ctor;
use sa_world::bindings::find_player_ped;
use sa_world::bindings::find_player_vehicle;
use sa_pop::bindings::fx_enable_prim;
use crate::bindings::fx_add_particle;
use sa_world::bindings::fx_prt_mult_ctor;

/// `TheCamera` matrix (+0x14); null: the placement position `TheCamera` +4 (0xB6F02C).
fn camera_pos() -> GPtr<f32> {
    let m = g::<u32>(0xB6F03C).read();
    if m != 0 {
        GPtr::from_addr(m as usize + 0x30)
    } else {
        g::<f32>(0xB6F02C)
    }
}

/// `Fx_c::AddDebris(const CVector& pos, const RwRGBA& colour, float scale, int32 amount)` (0x49F750, thiscall,
/// `ret 0x10`, `this` unused). Nothing beyond the squared camera distance 0x85A6E8. A default `FxPrtMult_c` gets
/// the colour / 255, the size `scale`, the spin `(rand() % 10000 * 0x858FC4 + 1) * 0.5` and the life 0.2; each of
/// `amount` particles has the velocity `(rand * k * 0.5 * 0x858BA4 - 0x858C80` twice, `rand * k * 0x858FCC *
/// 0x858BA4 + 2)`, enables the primitives 0..3 off and the current one (0xA9ADE4) on, and is added to the debris
/// system (0xA9AE0C) at `pos` (time 0, rotation -1, brightness 1.2, camera 0.6); the current primitive cycles 0..3.
#[hook(addr = 0x49F750, conv = thiscall, name = "Fx_c::AddDebris", this = "Fx_c",
       expect = "A1 3C F0 B6 00 83 EC 28", unverified = "std15-render-2", route = "L2",
       hazards = "heap-writes,stack-residue", recs = "car-drive",
       note = "the caller's pos / scale argument slots hold rand temporaries in the original")]
pub fn fx_add_debris(_this: GPtr<u8>, pos: GPtr<f32>, colour: GPtr<u8>, scale: f32, amount: i32) {
    let cam = camera_pos();
    let dx = x(cam.wrapping_index(0).read()).fsub_m(pos.wrapping_index(0).read());
    let dy = x(cam.wrapping_index(1).read()).fsub_m(pos.wrapping_index(1).read());
    let dz = x(cam.wrapping_index(2).read()).fsub_m(pos.wrapping_index(2).read());
    let d2 = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
    if !d2.fcom(x(gf(0x85A6E8))).test_any(0x41) {
        return;
    }
    let mut mult = [0u32; 8];
    let mp: GPtr<u8> = GPtr::from_raw(mult.as_mut_ptr()).cast();
    fx_prt_mult_default_ctor(mp.addr() as u32);
    let k255 = gf(0x859A3C);
    let c = |i: isize| i32::from(colour.byte_add(i).read());
    let mf = |o: isize, v: f32| mp.byte_add(o).cast::<f32>().write(v);
    mf(0, xi(c(0)).fmul_m(k255).to_f32());
    mf(0x10, scale);
    mf(4, xi(c(1)).fmul_m(k255).to_f32());
    mf(8, xi(c(2)).fmul_m(k255).to_f32());
    mf(0xC, xi(c(3)).fmul_m(k255).to_f32());
    let r = rand_at(0x49_F814) % 10000;
    mf(0x18, f32::from_bits(0x3E4C_CCCD));
    mf(
        0x14,
        xi(r)
            .fmul_m(gf(0x858FC4))
            .fadd_m(gf(0x858624))
            .fmul_m(gf(0x858B8C))
            .to_f32(),
    );
    let k = gf(0x858C7C);
    let mut n = amount;
    while n > 0 {
        let vx = xi(rand_at(0x49_F855))
            .fmul_m(k)
            .fmul_m(gf(0x858B8C))
            .fmul_m(gf(0x858BA4))
            .fsub_m(gf(0x858C80))
            .to_f32();
        let vy = xi(rand_at(0x49_F87E))
            .fmul_m(k)
            .fmul_m(gf(0x858B8C))
            .fmul_m(gf(0x858BA4))
            .fsub_m(gf(0x858C80))
            .to_f32();
        let r3 = rand_at(0x49_F8A7);
        let sys = g::<GPtr<u8>>(0xA9AE0C).read();
        let vz = xi(r3)
            .fmul_m(k)
            .fmul_m(gf(0x858FCC))
            .fmul_m(gf(0x858BA4))
            .fadd_m(gf(0x858CA0))
            .to_f32();
        let mut vel = [vx, vy, vz];
        fx_enable_prim(sys, 0, 0);
        fx_enable_prim(g::<GPtr<u8>>(0xA9AE0C).read(), 1, 0);
        fx_enable_prim(g::<GPtr<u8>>(0xA9AE0C).read(), 2, 0);
        fx_enable_prim(g::<GPtr<u8>>(0xA9AE0C).read(), 3, 0);
        let cur = g::<i32>(0xA9ADE4).read();
        fx_enable_prim(g::<GPtr<u8>>(0xA9AE0C).read(), cur, 1);
        fx_add_particle(
            g::<GPtr<u8>>(0xA9AE0C).read(),
            pos.cast(),
            GPtr::from_raw(vel.as_mut_ptr()).cast(),
            0.0f32.to_bits(),
            mp,
            (-1.0f32).to_bits(),
            f32::from_bits(0x3F99_999A),
            f32::from_bits(0x3F19_999A),
            0,
        );
        let next = g::<i32>(0xA9ADE4).read().wrapping_add(1) & 3;
        g::<i32>(0xA9ADE4).write(next);
        n -= 1;
    }
}

/// `Fx_c::AddWheelGrass(CVehicle*, CVector pos, bool spinning, float brightness)` (0x49FF20, thiscall, `ret 0x18`,
/// `this` unused). Only for a vehicle driven by player 0 or 1. Beyond the squared camera distance 0x85A6E8 nothing;
/// beyond 0x85A700 only when `(model id low byte + frame counter) & 3 == 0`; beyond 0x859A44, or within it when
/// the player has no vehicle, only on even `(id byte + frame)`. Otherwise an `FxPrtMult_c(0.03, 0.09, 0.03, 1, 0,
/// 0, 0.05)` and three grass particles in the system 0xA9AE3C: size `rand * k * 0x858B10 + 0x858B10`, velocity
/// `(rand * k * move.x * 0x85A704, rand * k * move.y * 0x85A704, rand * k * 0x858CE8 + 2)`, position `pos` with
/// x / y moved by `rand * k * 0x858EE8 - 0.2`.
#[hook(addr = 0x49FF20, conv = thiscall, name = "Fx_c::AddWheelGrass", this = "Fx_c",
       expect = "83 EC 34 56 8B 74 24 3C 6A 00", unverified = "std15-render-2", route = "L2",
       hazards = "heap-writes,stack-residue", recs = "car-drive",
       note = "the caller's vehicle / brightness argument slots hold rand temporaries in the original")]
#[allow(clippy::too_many_arguments)]
pub fn fx_add_wheel_grass(
    _this: GPtr<u8>,
    veh: GPtr<u8>,
    px: f32,
    py: f32,
    pz: f32,
    _spin: u32,
    bright: f32,
) {
    wheel_bits(veh, [px, py, pz], bright, [0x3CF5_C28F, 0x3DB8_51EC, 0x3CF5_C28F], 0);
}

/// The body shared by `AddWheelGrass` / `AddWheelGravel` / `AddWheelMud` (0x49FF20 / 0x4A0170 / 0x4A03C0: the same
/// code, 0x250 bytes apart, differing only in the particle colour `rgb`; `site` is the function's offset from 0x49FF20,
/// added to the `rand()` return addresses).
fn wheel_bits(veh: GPtr<u8>, p: [f32; 3], bright: f32, rgb: [u32; 3], site: u32) {
    let [px, py, pz] = p;
    let driver = veh.byte_add(0x460).cast::<GPtr<u8>>();
    if driver.read() != find_player_ped(0) && driver.read() != find_player_ped(1) {
        return;
    }
    let pv = find_player_vehicle(-1, 0);
    let cam = camera_pos();
    let dx = x(cam.wrapping_index(0).read()).fsub_m(px);
    let dy = x(cam.wrapping_index(1).read()).fsub_m(py);
    let dz = x(cam.wrapping_index(2).read()).fsub_m(pz);
    let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    if d2.fcom(x(gf(0x85A6E8))).test_parity(0x41) {
        return;
    }
    let tick = |mask: u8| {
        let id = veh.byte_add(0x22).read();
        id.wrapping_add(g::<u32>(0xB7CB4C).read() as u8) & mask != 0
    };
    if !d2.fcom(x(gf(0x85A700))).test_any(0x41) {
        if tick(3) {
            return;
        }
    } else {
        let far = !d2.fcom(x(gf(0x859A44))).test_any(0x41);
        if (far || pv.is_null()) && tick(1) {
            return;
        }
    }
    let mut mult = [0u32; 8];
    let mp: GPtr<u8> = GPtr::from_raw(mult.as_mut_ptr()).cast();
    fx_prt_mult_ctor(
        mp,
        f32::from_bits(rgb[0]),
        f32::from_bits(rgb[1]),
        f32::from_bits(rgb[2]),
        1.0,
        0.0,
        0.0,
        f32::from_bits(0x3D4C_CCCD),
    );
    let k = gf(0x858C7C);
    let mf = |o: isize, v: f32| mp.byte_add(o).cast::<f32>().write(v);
    for _ in 0..3 {
        mf(
            0x10,
            xi(rand_at(0x4A_0035 + site))
                .fmul_m(k)
                .fmul_m(gf(0x858B10))
                .fadd_m(gf(0x858B10))
                .to_f32(),
        );
        let sx = x(veh.byte_add(0x44).cast::<f32>().read())
            .fmul_m(gf(0x85A704))
            .to_f32();
        let vx = xi(rand_at(0x4A_0065 + site)).fmul_m(k).fmul_m(sx).to_f32();
        let sy = x(veh.byte_add(0x48).cast::<f32>().read())
            .fmul_m(gf(0x85A704))
            .to_f32();
        let vy = xi(rand_at(0x4A_008D + site)).fmul_m(k).fmul_m(sy).to_f32();
        let vz = xi(rand_at(0x4A_00A8 + site))
            .fmul_m(k)
            .fmul_m(gf(0x858CE8))
            .fadd_m(gf(0x858CA0))
            .to_f32();
        let p0 = xi(rand_at(0x4A_00DF + site))
            .fmul_m(k)
            .fmul_m(gf(0x858EE8))
            .fadd_m(px)
            .fsub_m(gf(0x858CC4))
            .to_f32();
        let r6 = rand_at(0x4A_0106 + site);
        let sys = g::<GPtr<u8>>(0xA9AE3C).read();
        let p1 = xi(r6)
            .fmul_m(k)
            .fmul_m(gf(0x858EE8))
            .fadd_m(py)
            .fsub_m(gf(0x858CC4))
            .to_f32();
        let mut pos = [p0, p1, pz];
        let mut vel = [vx, vy, vz];
        fx_add_particle(
            sys,
            GPtr::from_raw(pos.as_mut_ptr()).cast(),
            GPtr::from_raw(vel.as_mut_ptr()).cast(),
            0.0f32.to_bits(),
            mp,
            (-1.0f32).to_bits(),
            bright,
            f32::from_bits(0x3F19_999A),
            0,
        );
    }
}

/// `Fx_c::AddWheelGravel(CVehicle*, CVector pos, bool spinning, float brightness)` (0x4A0170, thiscall, `ret 0x18`;
/// FRONTIER-2 batch `frontier2-5`, user-hit driving): [`fx_add_wheel_grass`]'s code with the colour (0.25, 0.25, 0.25).
#[hook(addr = 0x4A0170, conv = thiscall, name = "Fx_c::AddWheelGravel", this = "Fx_c",
       expect = "83 EC 34 56 8B 74 24 3C 6A 00", unverified = "frontier2-5", route = "L2",
       hazards = "heap-writes,stack-residue", recs = "car-drive",
       note = "the caller's vehicle / brightness argument slots hold rand temporaries in the original")]
#[allow(clippy::too_many_arguments)]
pub fn fx_add_wheel_gravel(_this: GPtr<u8>, veh: GPtr<u8>, px: f32, py: f32, pz: f32, _spin: u32, bright: f32) {
    wheel_bits(veh, [px, py, pz], bright, [0x3E80_0000, 0x3E80_0000, 0x3E80_0000], 0x250);
}

/// `Fx_c::AddWheelMud(CVehicle*, CVector pos, bool spinning, float brightness)` (0x4A03C0, thiscall, `ret 0x18`;
/// FRONTIER-2 batch `frontier2-5`): [`fx_add_wheel_grass`]'s code with the colour (0.25, 0.12, 0.06).
#[hook(addr = 0x4A03C0, conv = thiscall, name = "Fx_c::AddWheelMud", this = "Fx_c",
       expect = "83 EC 34 56 8B 74 24 3C 6A 00", unverified = "frontier2-5", route = "L2",
       hazards = "heap-writes,stack-residue", recs = "car-drive",
       note = "the caller's vehicle / brightness argument slots hold rand temporaries in the original")]
#[allow(clippy::too_many_arguments)]
pub fn fx_add_wheel_mud(_this: GPtr<u8>, veh: GPtr<u8>, px: f32, py: f32, pz: f32, _spin: u32, bright: f32) {
    wheel_bits(veh, [px, py, pz], bright, [0x3E80_0000, 0x3DF5_C28F, 0x3D75_C28F], 0x4A0);
}

/// `Fx_c::AddWheelSand(CVehicle*, CVector pos, bool spinning, float brightness)` (0x4A0610, thiscall, `ret 0x18`;
/// FRONTIER-2 batch `frontier2-5`): [`wheel_cloud`] with the sand colour and life `(1 + F) * 0.1`.
#[hook(addr = 0x4A0610, conv = thiscall, name = "Fx_c::AddWheelSand", this = "Fx_c",
       expect = "83 EC 58 56 6A 00 6A FF", unverified = "frontier2-5", route = "L2",
       hazards = "heap-writes,stack-residue", recs = "car-drive",
       note = "the original keeps temporaries in its vehicle / spinning / brightness argument slots")]
#[allow(clippy::too_many_arguments)]
pub fn fx_add_wheel_sand(this: GPtr<u8>, veh: GPtr<u8>, px: f32, py: f32, pz: f32, spin: u8, bright: f32) {
    wheel_cloud(this, veh, [px, py, pz], spin, bright, [0x3F4F_5C29, 0x3F2B_851F, 0x3F11_EB85], false);
}

/// `Fx_c::AddWheelDust(CVehicle*, CVector pos, bool spinning, float brightness)` (0x4A09C0, thiscall, `ret 0x18`;
/// FRONTIER-2 batch `frontier2-5`, user-hit driving): [`wheel_cloud`] with the dust colour and life `0.05 * F + 0.1`.
#[hook(addr = 0x4A09C0, conv = thiscall, name = "Fx_c::AddWheelDust", this = "Fx_c",
       expect = "83 EC 58 56 6A 00 6A FF", unverified = "frontier2-5", route = "L2",
       hazards = "heap-writes,stack-residue", recs = "car-drive",
       note = "the original keeps temporaries in its vehicle / spinning / brightness argument slots")]
#[allow(clippy::too_many_arguments)]
pub fn fx_add_wheel_dust(this: GPtr<u8>, veh: GPtr<u8>, px: f32, py: f32, pz: f32, spin: u8, bright: f32) {
    wheel_cloud(this, veh, [px, py, pz], spin, bright, [0x3F02_8F5C, 0x3EE1_47AE, 0x3E9E_B852], true);
}

/// The body of `AddWheelSand` / `AddWheelDust` (0x4A0610 / 0x4A09C0, 0x3B0 bytes apart). Nothing beyond the squared
/// camera distance 0x85A6E8. With the effects quality (`Fx_c` +0x54) >= 1: odd `(model id + frame counter)` bytes skip,
/// and beyond 0x859A44 (or without a player vehicle) also those with bits 0..1 set; quality 0 skips bits 0..1 and,
/// beyond 0x859A44 or without a player vehicle, bits 0..2; negative quality never skips. `F` = 1 when spinning, else
/// `min(2 |move speed|, 1)`. An `FxPrtMult_c(rgb, 0.5, 1.0, 0, 0)` gets life (sand `(1 + F) * 0.1`, dust `0.05 * F +
/// 0.1`) and size `S = F * [0x858C20] + 0.1` scaled by the vehicle type (+0x594: 10 → `* [0x858C84]`, 9 / 2 → `* 0.5`,
/// else `* [0x858CB0]`); `n = max(ftol(|move * timestep| * M), 1)` with M = 2 for those types, else 1.5. Particle i of n
/// in the system 0xA9AE18: velocity `(rand * k * (|gas| * move.x * [0x85A708]), the same in y, rand * k * ((F + 0.8) -
/// 0.2) + 0.2)`, position `pos - move * timestep * (1 - i / n)`, brightness, 0.7.
#[allow(clippy::too_many_arguments)]
fn wheel_cloud(this: GPtr<u8>, veh: GPtr<u8>, p: [f32; 3], spin: u8, bright: f32, rgb: [u32; 3], dust: bool) {
    let site: u32 = if dust { 0x3B0 } else { 0 };
    let pv = find_player_vehicle(-1, 0);
    let cam = camera_pos();
    let dx = x(cam.wrapping_index(0).read()).fsub_m(p[0]);
    let dy = x(cam.wrapping_index(1).read()).fsub_m(p[1]);
    let dz = x(cam.wrapping_index(2).read()).fsub_m(p[2]);
    let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).to_f32();
    if x(d2).fcom(x(gf(0x85A6E8))).test_parity(0x41) {
        return;
    }
    let q = this.byte_add(0x54).cast::<i32>().read();
    let tick = (i32::from(veh.byte_add(0x22).cast::<i16>().read()) as u32)
        .wrapping_add(g::<u32>(0xB7CB4C).read()) as u8;
    let near_with_vehicle = || x(d2).fcom(x(gf(0x859A44))).test_any(0x41) && !pv.is_null();
    let skip = if q >= 1 {
        tick & 1 != 0 || (!near_with_vehicle() && tick & 3 != 0)
    } else if q != 0 {
        false
    } else {
        tick & 3 != 0 || (!near_with_vehicle() && tick & 7 != 0)
    };
    if skip {
        return;
    }
    let mut mult = [0u32; 8];
    let mp: GPtr<u8> = GPtr::from_raw(mult.as_mut_ptr()).cast();
    fx_prt_mult_ctor(
        mp,
        f32::from_bits(rgb[0]),
        f32::from_bits(rgb[1]),
        f32::from_bits(rgb[2]),
        0.5,
        1.0,
        0.0,
        0.0,
    );
    let mf = |o: isize, v: f32| mp.byte_add(o).cast::<f32>().write(v);
    let vf = |o: isize| veh.byte_add(o).cast::<f32>().read();
    let gas = x(vf(0x49C)).fabs().to_f32();
    let speed2 = || {
        let (vx, vy, vz) = (x(vf(0x44)), x(vf(0x48)), x(vf(0x4C)));
        let s = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt();
        s.fadd(s)
    };
    let one = x(gf(0x858624));
    // spinning, or 2|v| > 1 (fcomp 1.0; test ah, 0x41; jne recomputes): 1.0
    let f = if spin != 0 || !speed2().fcom(one).test_any(0x41) { one } else { speed2() };
    let life = if dust {
        x(gf(0x858C28)).fmul(f).fadd_m(gf(0x858B1C))
    } else {
        one.fadd(f).fmul_m(gf(0x858B1C))
    };
    mf(0x18, life.to_f32());
    let s = x(gf(0x858C20)).fmul(f).fadd_m(gf(0x858B1C)).to_f32();
    let ty = veh.byte_add(0x594).cast::<i32>().read();
    let (m, size) = match ty {
        0xA => (x(gf(0x858CA0)), x(s).fmul_m(gf(0x858C84))),
        9 | 2 => (x(gf(0x858CA0)), x(s).fmul_m(gf(0x858B8C))),
        _ => (x(gf(0x858CE8)), x(s).fmul_m(gf(0x858CB0))),
    };
    mf(0x10, size.to_f32());
    let ts = gf(0xB7CB5C);
    let vxt = x(ts).fmul_m(vf(0x44)).to_f32();
    let vyt = x(ts).fmul_m(vf(0x48)).to_f32();
    let vzt_x = x(ts).fmul_m(vf(0x4C));
    let vzt = vzt_x.to_f32();
    let len = vzt_x.fmul_m(vzt).fadd(x(vyt).fmul_m(vyt)).fadd(x(vxt).fmul_m(vxt)).fsqrt();
    let n = ftol32(len.fmul(m)).max(1);
    let z = f.fadd_m(gf(0x858C98)).fsub_m(gf(0x858CC4)).to_f32();
    let nf = xi(n).to_f32();
    let k = gf(0x858C7C);
    for i in 0..n {
        let fx_ = x(gas).fmul_m(vf(0x44)).fmul_m(gf(0x85A708)).to_f32();
        let vx = xi(rand_at(0x4A_08A9 + site)).fmul_m(k).fmul_m(fx_).to_f32();
        let fy_ = x(gas).fmul_m(vf(0x48)).fmul_m(gf(0x85A708)).to_f32();
        let vy = xi(rand_at(0x4A_08DB + site)).fmul_m(k).fmul_m(fy_).to_f32();
        let vz = xi(rand_at(0x4A_08F9 + site))
            .fmul_m(k)
            .fmul_m(z)
            .fadd_m(gf(0x858CC4))
            .to_f32();
        let sys = g::<GPtr<u8>>(0xA9AE18).read();
        let t = xi(i).fdiv_m(nf).fsubr_m(gf(0x858624));
        let ox = x(vxt).fmul(t);
        let oy = x(vyt).fmul(t).to_f32();
        let oz = x(vzt).fmul(t).to_f32();
        let mut pos = [x(p[0]).fsub(ox).to_f32(), x(p[1]).fsub_m(oy).to_f32(), x(p[2]).fsub_m(oz).to_f32()];
        let mut vel = [vx, vy, vz];
        fx_add_particle(
            sys,
            GPtr::from_raw(pos.as_mut_ptr()).cast(),
            GPtr::from_raw(vel.as_mut_ptr()).cast(),
            0.0f32.to_bits(),
            mp,
            (-1.0f32).to_bits(),
            bright,
            f32::from_bits(0x3F33_3333),
            0,
        );
    }
}

/// The stunt-jump pool's `GetAt(int32 i)` (0x49C3B0, thiscall, `ret 4`; a stub into `.HOODLUM` 0x156ACC0): null
/// when the slot's flag byte (`+4`)[i] is negative (free), else `objects + i * 0x44`.
#[hook(addr = 0x49C3B0, conv = thiscall, name = "thunk_FUN_0156acc0", expect = "E9 0B E9 0C 01",
       unverified = "std15-render-2", route = "L2", hazards = "none", recs = "car-drive")]
pub fn stunt_jump_pool_get_at(pool: GPtr<u8>, i: i32) -> GPtr<u8> {
    let flags = pool.byte_add(4).cast::<GPtr<u8>>().read();
    if (flags.byte_add(i as isize).read() as i8) < 0 {
        return GPtr::NULL;
    }
    let objs = pool.cast::<u32>().read();
    GPtr::from_addr(objs.wrapping_add((i as u32).wrapping_mul(0x44)) as usize)
}

pub const HOOKS: &[HookReg] = &[
    HOOK_FX_ADD_DEBRIS,
    HOOK_FX_ADD_WHEEL_GRASS,
    HOOK_FX_ADD_WHEEL_GRAVEL,
    HOOK_FX_ADD_WHEEL_MUD,
    HOOK_FX_ADD_WHEEL_SAND,
    HOOK_FX_ADD_WHEEL_DUST,
    HOOK_STUNT_JUMP_POOL_GET_AT,
];
