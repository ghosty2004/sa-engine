//! STD-15 (batch `std15-render-2`): `Fx_c::AddDebris` (0x49F750), `Fx_c::AddWheelGrass` (0x49FF20) and the
//! stunt-jump pool's `GetAt` (0x49C3B0, a stub into `.HOODLUM` 0x156ACC0), reached while driving. Unreversed in
//! gta-reversed; ported from the 1.0 US bytes, `rand()` at its original call sites. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GameData};
use sa_core::{hook, orig};
use sa_math::x87::X87;

fn g<T: GameData>(a: u32) -> GPtr<T> {
    GPtr::from_addr(a as usize)
}
fn gf(a: u32) -> f32 {
    g::<f32>(a).read()
}
fn x(v: f32) -> X87 {
    X87::from_f32(v)
}
fn xi(v: i32) -> X87 {
    X87::from_i32(v)
}
fn rand_at(ret: u32) -> i32 {
    sa_core::orig::at_site(ret, sa_core::crt::rand)
}

orig! {
    /// `FxPrtMult_c::FxPrtMult_c()` (0x4AB270)
    fn fx_prt_mult_default_ctor(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AB270 => unported;
    /// `FxPrtMult_c::FxPrtMult_c(r, g, b, a, size, spin, life)` (0x4AB290)
    #[allow(clippy::too_many_arguments)]
    fn fx_prt_mult_ctor(p: GPtr<u8>, r: f32, g: f32, b: f32, a: f32, size: f32, spin: f32, life: f32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(RwV3d*, RwV3d*, float, FxPrtMult_c*, float, float, float, uint8)` (0x4AA440)
    #[allow(clippy::too_many_arguments)]
    fn fx_add_particle(fx: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: f32, mult: GPtr<u8>, rot: f32, bright: f32, cam: f32, local: u32) = thiscall 0x4AA440 => unported;
    /// `FxSystem_c::EnablePrim(int32 prim, bool on)` (0x4AA610)
    fn fx_enable_prim(fx: GPtr<u8>, prim: i32, on: u32) = thiscall 0x4AA610 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
}

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
    fx_prt_mult_default_ctor(mp);
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
            0.0,
            mp,
            -1.0,
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
        f32::from_bits(0x3CF5_C28F),
        f32::from_bits(0x3DB8_51EC),
        f32::from_bits(0x3CF5_C28F),
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
            xi(rand_at(0x4A_0035))
                .fmul_m(k)
                .fmul_m(gf(0x858B10))
                .fadd_m(gf(0x858B10))
                .to_f32(),
        );
        let sx = x(veh.byte_add(0x44).cast::<f32>().read())
            .fmul_m(gf(0x85A704))
            .to_f32();
        let vx = xi(rand_at(0x4A_0065)).fmul_m(k).fmul_m(sx).to_f32();
        let sy = x(veh.byte_add(0x48).cast::<f32>().read())
            .fmul_m(gf(0x85A704))
            .to_f32();
        let vy = xi(rand_at(0x4A_008D)).fmul_m(k).fmul_m(sy).to_f32();
        let vz = xi(rand_at(0x4A_00A8))
            .fmul_m(k)
            .fmul_m(gf(0x858CE8))
            .fadd_m(gf(0x858CA0))
            .to_f32();
        let p0 = xi(rand_at(0x4A_00DF))
            .fmul_m(k)
            .fmul_m(gf(0x858EE8))
            .fadd_m(px)
            .fsub_m(gf(0x858CC4))
            .to_f32();
        let r6 = rand_at(0x4A_0106);
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
            0.0,
            mp,
            -1.0,
            bright,
            f32::from_bits(0x3F19_999A),
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
    HOOK_STUNT_JUMP_POOL_GET_AT,
];
