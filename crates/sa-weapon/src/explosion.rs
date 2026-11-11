//! `CExplosion` (PORT-WEAPON-PED): the array helpers. Ported from the 1.0 US bytes (gta-reversed
//! `Explosion.cpp`, `docs/re/specs/WEAPON/CExplosion.deviations.md`).
//!
//! `CExplosion::aExplosions[16]` (0x7C each) at 0xC88950: +0 type, +4 position, +0x10 radius,
//! +0x14 expansion rate, +0x18 victim / +0x1C creator (entities), +0x20 time to live, +0x24 ...,
//! +0x28 active (counter), +0x29 active counter, +0x2A makes sound, +0x2C created time, +0x30 ...,
//! +0x34 ..., +0x38 visibility / damage, +0x3C ... (the fields written below are as the bytes do).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

pub const EXPLOSION_SIZE: usize = 0x7C;
pub const NUM_EXPLOSIONS: usize = 16;
const _: () = assert!(EXPLOSION_SIZE == sa_core::layouts::size::CExplosion);

sa_core::globals! {
    list GLOBALS;
    /// `CExplosion::aExplosions[16]`
    pub static EXPLOSIONS @ 0xC88950: [u8; NUM_EXPLOSIONS * EXPLOSION_SIZE] = [0; NUM_EXPLOSIONS * EXPLOSION_SIZE];
}

orig! {
    /// `CWorld::TriggerExplosion(const CVector&, float radius, float visibleDistance, CEntity* victim,
    /// CEntity* creator, bool processVehicleBombTimer, float damage)` (0x56B790); raw words as pushed
    pub fn trigger_explosion(pos: GPtr<u8>, radius: u32, vis: u32, victim: GPtr<u8>, creator: GPtr<u8>, b: u32, dmg: u32) = cdecl 0x56B790 => unported;
    /// `CPointLights::AddLight(uint8 type, CVector point, CVector dir, float radius, float r, g, b, uint8 fog,
    /// bool shadows, CEntity*)` (0x7000E0)
    #[allow(clippy::too_many_arguments)]
    pub fn add_light(ty: u32, px: f32, py: f32, pz: f32, dx: u32, dy: u32, dz: u32, radius: f32, r: f32, g: f32, b: f32, fog: u32, shadows: u32, e: GPtr<u8>) = cdecl 0x7000E0 => unported;
    /// `CVector::operator*=(float)` (0x40FEF0, thiscall)
    pub fn vec_mul_assign(v: GPtr<u8>, f: f32) = thiscall 0x40FEF0 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00, thiscall)
    pub fn vec_add_assign(v: GPtr<u8>, o: GPtr<u8>) = thiscall 0x411A00 => unported;
    /// `CVector::Normalise()` (0x59C910, thiscall)
    pub fn normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CCreepingFire::TryToStartFireAtCoors(CVector, int8, bool, bool, float)` (0x53A450)
    pub fn try_to_start_fire_at_coors(x: f32, y: f32, z: f32, gens: i8, a: u8, script: u8, zdist: f32) -> u8 = cdecl 0x53A450 => crate::fire::try_to_start_fire_at_coors;
    /// `CExplosion::AddExplosion(...)` (0x736A50)
    pub fn add_explosion(victim: GPtr<u8>, creator: GPtr<u8>, ty: i32, x: f32, y: f32, z: f32, life: u32, sound: u8, shake: f32, invisible: u8) = cdecl 0x736A50 => unported;
    /// `CWorld::SetPedsOnFire(float x, y, z, float radius, CEntity*)` (0x565610)
    pub fn set_peds_on_fire(x: u32, y: u32, z: u32, r: f32, e: GPtr<u8>) = cdecl 0x565610 => unported;
    /// `CWorld::SetCarsOnFire(...)` (0x5659F0)
    pub fn set_cars_on_fire(x: u32, y: u32, z: u32, r: f32, e: GPtr<u8>) = cdecl 0x5659F0 => unported;
    /// `CWorld::SetWorldOnFire(...)` (0x56B910)
    pub fn set_world_on_fire(x: u32, y: u32, z: u32, r: f32, e: GPtr<u8>) = cdecl 0x56B910 => unported;
    /// `CWorld::ProcessVerticalLine(...)` (0x5674E0)
    pub fn process_vertical_line(p: GPtr<u8>, z: f32, cp: GPtr<u8>, ent: GPtr<GPtr<u8>>, b: u8, v: u8, pd: u8, o: u8, d: u8, s: u8, poly: GPtr<u8>) -> u8 = cdecl 0x5674E0 => unported;
    /// `FxManager_c::CreateFxSystem(const char*, CVector*, RwMatrix*, bool)` (0x4A9BE0, thiscall)
    pub fn create_fx_system(man: GPtr<u8>, name: GPtr<u8>, pos: GPtr<u8>, m: GPtr<u8>, flag: u8) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `CExplosion::ClearAllExplosions()` (0x736840)
    pub fn clear_all_explosions_call() = cdecl 0x736840 => clear_all_explosions;
}

/// `aExplosions[i]`.
pub fn explosion(i: usize) -> GPtr<u8> {
    EXPLOSIONS.ptr().cast::<u8>().byte_add((i * EXPLOSION_SIZE) as isize)
}

/// `aExplosions[(uint8)i]` (the byte index of the accessors).
fn by_id(i: u8) -> GPtr<u8> {
    explosion(usize::from(i))
}

/// The absolute address `a` inside the array (the inner loop of `ClearAllExplosions` writes fixed
/// addresses).
fn at_addr(a: usize) -> GPtr<u8> {
    EXPLOSIONS.ptr().cast::<u8>().byte_add(a as isize - 0xC88950)
}

/// `CExplosion::ClearAllExplosions()` (0x736840): per explosion: position 0 (zeroed temporaries),
/// type 0, +0x10 1.0, +0x34/+0x14/+0x38/+0x18/+0x1C/+0x20/+0x2C/+0x3C 0, +0x28 byte 0, +0x2A byte 1;
/// then an inner loop that does not depend on the explosion (original bug: it writes the same three
/// pairs of fixed addresses 0xC88990.. / 0xC889B4.. every time).
#[hook(addr = 0x736840, conv = cdecl, name = "CExplosion::ClearAllExplosions", expect = "83 EC 0C 53 55")]
pub fn clear_all_explosions() {
    for i in 0..NUM_EXPLOSIONS {
        let e = explosion(i);
        wu(e, 4, 0);
        wu(e, 8, 0);
        wu(e, 0xC, 0);
        wu(e, 0, 0);
        wu(e, 0x10, 0x3F80_0000);
        wu(e, 0x34, 0);
        wu(e, 0x14, 0);
        wu(e, 0x38, 0);
        wu(e, 0x18, 0);
        wu(e, 0x1C, 0);
        wu(e, 0x20, 0);
        wb(e, 0x28, 0);
        wu(e, 0x2C, 0);
        wb(e, 0x2A, 1);
        wu(e, 0x3C, 0);
        for k in 0..3usize {
            let edx = 0xC88994 + k * 0x88;
            let esi = 0xC889C0 + k * 0x80;
            wu(at_addr(edx - 4), 0, 0);
            wu(at_addr(edx), 0, 0);
            wu(at_addr(edx + 4), 0, 0);
            wu(at_addr(esi - 0xC), 0, 0);
            wu(at_addr(esi), 0, 0);
        }
    }
}

/// `CExplosion::Initialise()` (0x736A40): `jmp ClearAllExplosions`.
#[hook(addr = 0x736A40, conv = cdecl, name = "CExplosion::Initialise", expect = "E9 FB FD FF FF 90")]
pub fn initialise() {
    clear_all_explosions_call();
}

/// `CExplosion::GetExplosionActiveCounter(uint8 id)` (0x736900): byte +0x29.
#[hook(addr = 0x736900, conv = cdecl, name = "CExplosion::GetExplosionActiveCounter", expect = "0F B6 44 24 04")]
pub fn get_explosion_active_counter(id: u8) -> u32 {
    (u32::from(id) * EXPLOSION_SIZE as u32) & 0xFFFF_FF00 | u32::from(rb(by_id(id), 0x29))
}

/// `CExplosion::ResetExplosionActiveCounter(uint8 id)` (0x736910).
#[hook(addr = 0x736910, conv = cdecl, name = "CExplosion::ResetExplosionActiveCounter", expect = "0F B6 44 24 04")]
pub fn reset_explosion_active_counter(id: u8) {
    wb(by_id(id), 0x29, 0);
}

/// `CExplosion::DoesExplosionMakeSound(uint8 id)` (0x736920): byte +0x2A.
#[hook(addr = 0x736920, conv = cdecl, name = "CExplosion::DoesExplosionMakeSound", expect = "0F B6 44 24 04")]
pub fn does_explosion_make_sound(id: u8) -> u32 {
    (u32::from(id) * EXPLOSION_SIZE as u32) & 0xFFFF_FF00 | u32::from(rb(by_id(id), 0x2A))
}

/// `CExplosion::GetExplosionType(uint8 id)` (0x736930).
#[hook(addr = 0x736930, conv = cdecl, name = "CExplosion::GetExplosionType", expect = "0F B6 44 24 04")]
pub fn get_explosion_type(id: u8) -> u32 {
    ru(by_id(id), 0)
}

/// `CExplosion::GetExplosionPosition(uint8 id)` (0x736940): `&aExplosions[id].pos`.
#[hook(addr = 0x736940, conv = cdecl, name = "CExplosion::GetExplosionPosition", expect = "0F B6 44 24 04")]
pub fn get_explosion_position(id: u8) -> GPtr<u8> {
    by_id(id).byte_add(4)
}

/// `CExplosion::TestForExplosionInArea(type, minX, maxX, minY, maxY, minZ, maxZ)` (0x736950): an
/// active explosion (+0x28) of `type` (or any for -1) with `min <= pos <= max` per axis.
#[hook(addr = 0x736950, conv = cdecl, name = "CExplosion::TestForExplosionInArea", expect = "8B 54 24 04 B9 54 89 C8 00")]
pub fn test_for_explosion_in_area(ty: i32, x1: f32, x2: f32, y1: f32, y2: f32, z1: f32, z2: f32) -> u8 {
    for i in 0..NUM_EXPLOSIONS {
        let e = explosion(i);
        if rb(e, 0x28) == 0 {
            continue;
        }
        if ru(e, 0) as i32 != ty && ty != -1 {
            continue;
        }
        if crate::fire::in_range(rf(e, 4), x1, x2)
            && crate::fire::in_range(rf(e, 8), y1, y2)
            && crate::fire::in_range(rf(e, 0xC), z1, z2)
        {
            return 1;
        }
    }
    0
}

/// `CExplosion::RemoveAllExplosionsInArea(CVector pos, float radius)` (0x7369E0): active explosions
/// with `radius*radius > (dx² + dz²) + dy²` are deactivated.
#[hook(addr = 0x7369E0, conv = cdecl, name = "CExplosion::RemoveAllExplosionsInArea", expect = "B9 58 89 C8 00")]
pub fn remove_all_explosions_in_area(px: f32, py: f32, pz: f32, r: f32) {
    for i in 0..NUM_EXPLOSIONS {
        let e = explosion(i);
        if rb(e, 0x28) == 0 {
            continue;
        }
        let mut f = Fpu::new();
        f.fld(pz);
        f.fsub_m(rf(e, 0xC));
        f.fld(py);
        f.fsub_m(rf(e, 8));
        f.fld(px);
        f.fsub_m(rf(e, 4));
        f.fld_st(0);
        f.fmul_st(1);
        f.fld_st(3);
        f.fmul_st(4);
        f.faddp(1);
        f.fld_st(2);
        f.fmul_st(3);
        f.faddp(1);
        f.fld(r);
        f.fmul_m(r);
        f.fcompp();
        if !f.jne(0x41) {
            wb(e, 0x28, 0);
        }
    }
}

/// `(float)(uint32)v` (`fild` + 2^32 for negative ints).
fn u32_x87(v: u32) -> X87 {
    let f = xi(v as i32);
    if (v as i32) < 0 { f.fadd_m(4_294_967_296.0) } else { f }
}

/// `CExplosion::Update()` (0x737620): per active explosion (fields relative to the entry):
/// - a pending trigger (+0x30): after its time it is cleared and, with +0x34 != 0 (or NaN),
///   `CWorld::TriggerExplosion(pos, radius, +0x34, victim, creator, type in {2,5,8,9,10}, +0x24)` (the
///   false flag is the type dword with AL cleared, as the bytes push it);
/// - else: radius += ts * rate; remaining = `_ftol(endTime - (float)now)`; by type (jump table
///   0x737B18): cars/heli/boat (4..6): with a victim and `(rand() & 31) == 0` a creeping fire near
///   the VICTIM (`v = normalise(b, a, 0) * GRN(1, 2) + victimPos`, z + 2; the first random value goes
///   to y), and every other frame a light (15, 1, 0.7, 0.5); types 0,2,3,7,8,9: every other frame a
///   light (20, 1, 1, 0.5), and for type 7 `GRN(0, 100) < 5` with a victim another explosion (type 2)
///   at the victim; molotov (1): peds/cars on fire (6.0) and the world (0.1), and on the first
///   counter value the ground height below `pos + 5` (else `pos.z`) into +0x38;
/// - the counter (+0x28) goes up while time remains, else 0 (inactive); the fuel timer (+0x3C) drops by
///   `_ftol(ts * 0.02 * -1000)`; for types 4..7 with the timer <= 200 each of the three fuel jets with a
///   positive speed emits "explosion_fuel_car" at `pos + dir * (timer * 0.001 * k + speed)`.
#[hook(addr = 0x737620, conv = cdecl, name = "CExplosion::Update", expect = "81 EC 8C 00 00 00")]
pub fn update() {
    for i in 0..NUM_EXPLOSIONS {
        let e = explosion(i);
        if rb(e, 0x28) == 0 {
            continue;
        }
        let trig = ru(e, 0x30);
        if trig != 0 {
            if time_ms() <= trig {
                continue;
            }
            wu(e, 0x30, 0);
            let mut f = Fpu::new();
            f.fld(rf(e, 0x34));
            f.fcomp_m(0.0);
            if !f.jp(0x44) {
                continue;
            }
            let ty = ru(e, 0);
            let flag = if matches!(ty, 2 | 5 | 8 | 9 | 10) { 1 } else { ty & 0xFFFF_FF00 };
            trigger_explosion(e.byte_add(4), ru(e, 0x10), ru(e, 0x34), rp(e, 0x1C), rp(e, 0x18), flag, ru(e, 0x24));
            continue;
        }
        wf(e, 0x10, x(time_step()).fmul_m(rf(e, 0x14)).fadd_m(rf(e, 0x10)).to_f32());
        let p0 = rv(e, 4);
        let remaining = ftol32(u32_x87(time_ms()).fsubr_m(rf(e, 0x20)));
        let ty = ru(e, 0);
        match ty {
            4..=6 => {
                let victim = rp(e, 0x1C);
                if !victim.is_null() && sa_core::orig::at_site(0x737717 + 5, rand) & 0x1F == 0 {
                    let a = st0(random_float(-0.5, 0.5)).to_f32();
                    let b = st0(random_float(-0.5, 0.5)).to_f32();
                    let mut v = [b, a, 0.0];
                    normalise(lp(&mut v).cast());
                    let k = st0(random_float(1.0, 2.0)).to_f32();
                    vec_mul_assign(lp(&mut v).cast(), k);
                    vec_add_assign(lp(&mut v).cast(), pos_ptr(rp(e, 0x1C)));
                    let z = x(v[2]).fadd_m(2.0).to_f32();
                    v[2] = z;
                    try_to_start_fire_at_coors(v[0], v[1], z, 0, 1, 0, 10.0);
                }
                if frame_counter() & 1 != 0 {
                    add_light(0, p0[0], p0[1], p0[2], 0, 0, 0, 15.0, 1.0, f32::from_bits(0x3F33_3333), 0.5, 0, 1, GPtr::NULL);
                }
            }
            0 | 2 | 3 | 7 | 8 | 9 => {
                if frame_counter() & 1 != 0 {
                    let p = rv(e, 4);
                    add_light(0, p[0], p[1], p[2], 0, 0, 0, 20.0, 1.0, 1.0, 0.5, 0, 1, GPtr::NULL);
                }
                if ty == 7 && random_int(0, 100) < 5 {
                    let victim = rp(e, 0x1C);
                    if !victim.is_null() {
                        let vp = rv(pos_ptr(victim), 0);
                        add_explosion(victim, rp(e, 0x18), 2, vp[0], vp[1], vp[2], 0, 1, -1.0, 0);
                    }
                }
            }
            1 => {
                let p = rv_bits(e, 4);
                set_peds_on_fire(p[0], p[1], p[2], 6.0, rp(e, 0x18));
                set_cars_on_fire(p[0], p[1], p[2], 6.0, rp(e, 0x18));
                set_world_on_fire(p[0], p[1], p[2], f32::from_bits(0x3DCC_CCCD), rp(e, 0x18));
                if rb(e, 0x28) == 1 {
                    let p = rv(e, 4);
                    let mut v = [p[0], p[1], x(p[2]).fadd_m(5.0).to_f32()];
                    let mut cp = [0u8; 0x2C];
                    let mut ent = GPtr::NULL;
                    if process_vertical_line(lp(&mut v).cast(), -1000.0, lp(&mut cp).cast(), lp(&mut ent), 1, 0, 0, 0, 1, 0, GPtr::NULL) != 0 {
                        wu(e, 0x38, u32::from_le_bytes([cp[8], cp[9], cp[10], cp[11]]));
                    } else {
                        wu(e, 0x38, ru(e, 0xC));
                    }
                }
            }
            _ => {}
        }
        if remaining > 0 {
            wb(e, 0x28, rb(e, 0x28).wrapping_add(1));
        } else {
            wb(e, 0x28, 0);
        }
        let d = ftol32(x(time_step()).fmul_m(f32::from_bits(0x3CA3_D70A)).fmul_m(-1000.0));
        let timer = ru(e, 0x3C).wrapping_sub(d as u32);
        wu(e, 0x3C, timer);
        if !matches!(ty, 4..=7) || timer > 200 {
            continue;
        }
        for k in 0..3isize {
            let speed = rf(e, 0x64 + 4 * k);
            let mut f = Fpu::new();
            f.fld(speed);
            f.fcomp_m(0.0);
            if f.jne(0x41) {
                continue;
            }
            let s = u32_x87(ru(e, 0x3C))
                .fmul_m(f32::from_bits(0x3A83_126F))
                .fmul_m(rf(e, 0x70 + 4 * k))
                .fadd_m(rf(e, 0x64 + 4 * k));
            let dir = e.byte_add(0x40 + 0xC * k);
            let sx = s.fmul_m(rf(dir, 0));
            let sy = s.fmul_m(rf(dir, 4)).to_f32();
            let sz = s.fmul_m(rf(dir, 8)).to_f32();
            let p = rv(e, 4);
            let mut pos = [sx.fadd_m(p[0]).to_f32(), x(p[1]).fadd_m(sy).to_f32(), x(p[2]).fadd_m(sz).to_f32()];
            let fx = create_fx_system(g(0xA9AE80), g(0x872BC8), lp(&mut pos).cast(), GPtr::NULL, 0);
            if !fx.is_null() {
                fx_play_and_kill(fx);
            }
        }
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_CLEAR_ALL_EXPLOSIONS,
    HOOK_INITIALISE,
    HOOK_GET_EXPLOSION_ACTIVE_COUNTER,
    HOOK_RESET_EXPLOSION_ACTIVE_COUNTER,
    HOOK_DOES_EXPLOSION_MAKE_SOUND,
    HOOK_GET_EXPLOSION_TYPE,
    HOOK_GET_EXPLOSION_POSITION,
    HOOK_TEST_FOR_EXPLOSION_IN_AREA,
    HOOK_REMOVE_ALL_EXPLOSIONS_IN_AREA,
    HOOK_UPDATE,
];
