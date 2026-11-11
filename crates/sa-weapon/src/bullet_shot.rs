//! `CBulletInfo` and `CShotInfo` (PORT-WEAPON-PED). Ported from the 1.0 US bytes (gta-reversed
//! `BulletInfo.cpp`/`ShotInfo.cpp`, `docs/re/specs/WEAPON/C{Bullet,Shot}Info.deviations.md`).
//!
//! `CBulletInfo::aBulletInfos[8]` (0x2C) at 0xC88740: +0 weapon type, +4 creator, +8 destruction time
//! (f32), +0xC active, +0x10 position, +0x1C velocity, +0x28 damage (int16).
//! `CShotInfo::aShotInfos[100]` (0x2C) at 0xC89690: +0 weapon type, +4 origin, +0x10 direction
//! (velocity), +0x1C spread, +0x20 creator, +0x24 destroy time (f32), +0x28 active, +0x29 executed.
//! `CShotInfo::ms_afRandTable[20]` at 0xC89628.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

pub const BULLET_SIZE: usize = 0x2C;
pub const NUM_BULLETS: usize = 8;
pub const SHOT_SIZE: usize = 0x2C;
pub const NUM_SHOTS: usize = 100;
const _: () = assert!(BULLET_SIZE == sa_core::layouts::size::CBulletInfo);
const _: () = assert!(SHOT_SIZE == sa_core::layouts::size::CShotInfo);

sa_core::globals! {
    list GLOBALS;
    /// `CBulletInfo::aBulletInfos[8]`
    pub static BULLETS @ 0xC88740: [u8; NUM_BULLETS * BULLET_SIZE] = [0; NUM_BULLETS * BULLET_SIZE];
    /// `CShotInfo::ms_afRandTable[20]`
    pub static SHOT_RAND_TABLE @ 0xC89628: [f32; 20] = [0.0; 20];
    /// `CShotInfo::aShotInfos[100]`
    pub static SHOTS @ 0xC89690: [u8; NUM_SHOTS * SHOT_SIZE] = [0; NUM_SHOTS * SHOT_SIZE];
}

orig! {
    /// `CPed::IsPointerValid()` (0x5E4220, thiscall)
    pub fn ped_is_pointer_valid(ped: GPtr<u8>) -> u8 = thiscall 0x5E4220 => unported;
    /// `CPed::IsPedInControl()` (0x5E3960, thiscall)
    pub fn ped_is_ped_in_control(ped: GPtr<u8>) -> u8 = thiscall 0x5E3960 => unported;
    /// `CPed::GetLocalDirection(const CVector2D&)` (0x5DEF60, thiscall)
    pub fn ped_get_local_direction(ped: GPtr<u8>, v: GPtr<u8>) -> i32 = thiscall 0x5DEF60 => unported;
    /// `CWeapon::GenerateDamageEvent(CPed*, CEntity*, eWeaponType, int32 damage, ePedPieceTypes, uint8 dir)` (0x73A530)
    pub fn generate_damage_event(ped: GPtr<u8>, creator: GPtr<u8>, ty: i32, damage: i32, piece: i32, dir: i32) = cdecl 0x73A530 => unported;
    /// `CFireManager::StartFire(CEntity* target, CEntity* creator, float, uint8, uint32 time, int8 gens)` (0x53A050, thiscall)
    pub fn start_fire_target(man: GPtr<u8>, target: GPtr<u8>, creator: GPtr<u8>, size: f32, unk: u8, time: u32, gens: i8) -> GPtr<u8> = thiscall 0x53A050 => unported;
    /// `CWorld::SprayPaintWorld(CVector&, CVector&, float, bool)` (0x565B70)
    pub fn spray_paint_world(pos: GPtr<u8>, out: GPtr<u8>, range: f32, b: u8) -> i32 = cdecl 0x565B70 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float volumeChange, float speed)` (0x506EA0, thiscall)
    pub fn report_frontend_audio_event(ae: GPtr<u8>, ev: i32, vol: f32, speed: f32) = thiscall 0x506EA0 => unported;
    /// `CFireManager::ExtinguishPointWithWater(CVector, float, float)` (0x5394C0, thiscall)
    pub fn extinguish_point_with_water(man: GPtr<u8>, x: f32, y: f32, z: f32, r: f32, w: f32) -> u8 = thiscall 0x5394C0 => crate::fire::extinguish_point_with_water;
    /// `CWorld::SetCarsOnFire(float x, y, z, float radius, CEntity*)` (0x5659F0)
    pub fn set_cars_on_fire(x: f32, y: f32, z: f32, r: f32, e: GPtr<u8>) = cdecl 0x5659F0 => unported;
    /// `CWorld::SetWorldOnFire(float x, y, z, float radius, CEntity*)` (0x56B910)
    pub fn set_world_on_fire(x: f32, y: f32, z: f32, r: f32, e: GPtr<u8>) = cdecl 0x56B910 => unported;
    /// `CVector::Normalise()` (0x59C910, thiscall)
    pub fn normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
}

pub fn bullet(i: usize) -> GPtr<u8> {
    BULLETS.ptr().cast::<u8>().byte_add((i * BULLET_SIZE) as isize)
}

pub fn shot(i: usize) -> GPtr<u8> {
    SHOTS.ptr().cast::<u8>().byte_add((i * SHOT_SIZE) as isize)
}

fn rand_table(i: i32) -> f32 {
    SHOT_RAND_TABLE.ptr().cast::<f32>().wrapping_index(i).read()
}

/// The "first free slot" scan of `AddBullet` / `AddShot`: slot 0 if inactive, else the loop
/// `while (p < end) { c = p[next].active; p = next; i++; if (!c) break; }`, which reads the active
/// byte one element past the array when every slot is in use (original out-of-bounds read).
fn first_free(active0: GPtr<u8>, size: usize, n: usize) -> usize {
    if active0.read() == 0 {
        return 0;
    }
    let mut i = 0;
    while i < n {
        let c = active0.byte_add(((i + 1) * size) as isize).read();
        i += 1;
        if c == 0 {
            break;
        }
    }
    i
}

/// `CBulletInfo::Initialise()` (0x735FD0): every bullet inactive, type 22, time and creator 0.
#[hook(addr = 0x735FD0, conv = cdecl, name = "CBulletInfo::Initialise", expect = "B8 40 87 C8 00")]
pub fn bullet_initialise() {
    for i in 0..NUM_BULLETS {
        let b = bullet(i);
        wb(b, 0xC, 0);
        wu(b, 0, 0x16);
        wu(b, 8, 0);
        wu(b, 4, 0);
    }
}

/// `CBulletInfo::AddBullet(CEntity* creator, eWeaponType, CVector pos, CVector velocity)` (0x736010):
/// false when all 8 are in use; else type, creator, damage (`GetWeaponInfo(type, 1)` +0x22), position,
/// velocity, time `(float)(now + 1000)`, active.
#[hook(addr = 0x736010, conv = cdecl, name = "CBulletInfo::AddBullet", expect = "51 A0 4C 87 C8 00")]
#[allow(clippy::too_many_arguments)]
pub fn add_bullet(creator: GPtr<u8>, ty: i32, px: f32, py: f32, pz: f32, vx: f32, vy: f32, vz: f32) -> u8 {
    let i = first_free(bullet(0).byte_add(0xC), BULLET_SIZE, NUM_BULLETS);
    if i == NUM_BULLETS {
        return 0;
    }
    let b = bullet(i);
    wp(b, 4, creator);
    wu(b, 0, ty as u32);
    wh(b, 0x28, rh(get_weapon_info(ty, 1), 0x22));
    wv(b, 0x10, [px, py, pz]);
    wv(b, 0x1C, [vx, vy, vz]);
    wb(b, 0xC, 1);
    wf(b, 8, xi(time_ms().wrapping_add(1000) as i32).to_f32());
    1
}

/// `CShotInfo::Initialise()` (0x739B60): every shot: origin and direction 0 (zeroed temporaries),
/// inactive, type 22, spread 1.0, creator 0, time 0; then `ms_afRandTable[i] = -0.05 + 0.005*i`
/// accumulated on the x87 stack.
#[hook(addr = 0x739B60, conv = cdecl, name = "CShotInfo::Initialise", expect = "83 EC 18 53 55")]
pub fn shot_initialise() {
    for i in 0..NUM_SHOTS {
        let s = shot(i);
        wu(s, 4, 0);
        wu(s, 8, 0);
        wu(s, 0xC, 0);
        wu(s, 0x10, 0);
        wb(s, 0x28, 0);
        wb(s, 0x29, 0);
        wu(s, 0, 0x16);
        wu(s, 0x14, 0);
        wu(s, 0x1C, 0x3F80_0000);
        wu(s, 0x20, 0);
        wu(s, 0x24, 0);
        wu(s, 0x18, 0);
    }
    let mut v = X87::from_f32(f32::from_bits(0xBD4C_CCCD));
    let t = SHOT_RAND_TABLE.ptr().cast::<f32>();
    for i in 0..20 {
        t.wrapping_index(i).write(v.to_f32());
        v = v.fadd_m(f32::from_bits(0x3BA3_D70A));
    }
}

/// `CShotInfo::AddShot(CEntity* creator, eWeaponType, CVector origin, CVector target)` (0x739C30):
/// false when all 100 are in use; else type, origin, direction `target - origin` (f32 per axis),
/// active, not executed, spread = info+0x60; with info+0x68 != 0 (or NaN) the direction is jittered
/// by `ms_afRandTable[rand() % 20]` (x, y scaled by info+0x68, z unscaled); normalised; times the
/// speed info+0x5C (+ a table entry with flag 0x20000); creator; destroy time `(float)_ftol(info+0x64
/// + (float)now)`.
#[hook(addr = 0x739C30, conv = cdecl, name = "CShotInfo::AddShot", expect = "83 EC 10 53 56")]
#[allow(clippy::too_many_arguments)]
pub fn add_shot(creator: GPtr<u8>, ty: i32, ox: f32, oy: f32, oz: f32, tx: f32, ty_: f32, tz: f32) -> u8 {
    let info = get_weapon_info(ty, 1);
    let i = first_free(shot(0).byte_add(0x28), SHOT_SIZE, NUM_SHOTS);
    if i == NUM_SHOTS {
        return 0;
    }
    let s = shot(i);
    let dx = x(tx).fsub_m(ox).to_f32();
    let dy = x(ty_).fsub_m(oy).to_f32();
    wv(s, 4, [ox, oy, oz]);
    wu(s, 0, ty as u32);
    let dz = x(tz).fsub_m(oz).to_f32();
    wv(s, 0x10, [dx, dy, dz]);
    wb(s, 0x28, 1);
    wb(s, 0x29, 0);
    wf(s, 0x1C, rf(info, 0x60));
    let mut f = Fpu::new();
    f.fld(rf(info, 0x68));
    f.fcomp_m(0.0);
    if f.jp(0x44) {
        let r = sa_core::orig::at_site(0x739D06, rand) % 20;
        wf(s, 0x10, x(rand_table(r)).fmul_m(rf(info, 0x68)).fadd_m(rf(s, 0x10)).to_f32());
        let r = sa_core::orig::at_site(0x739D21, rand) % 20;
        wf(s, 0x14, x(rand_table(r)).fmul_m(rf(info, 0x68)).fadd_m(rf(s, 0x14)).to_f32());
        let r = sa_core::orig::at_site(0x739D44, rand) % 20;
        wf(s, 0x18, x(rand_table(r)).fadd_m(rf(s, 0x18)).to_f32());
    }
    normalise(s.byte_add(0x10));
    let speed = if ru(info, 0x18) & 0x20000 != 0 {
        let r = sa_core::orig::at_site(0x739D74, rand) % 20;
        x(rf(info, 0x5C)).fadd_m(rand_table(r))
    } else {
        x(rf(info, 0x5C))
    };
    wf(s, 0x10, speed.fmul_m(rf(s, 0x10)).to_f32());
    wf(s, 0x14, speed.fmul_m(rf(s, 0x14)).to_f32());
    wf(s, 0x18, speed.fmul_m(rf(s, 0x18)).to_f32());
    wp(s, 0x20, creator);
    let now = time_ms();
    let mut t = xi(now as i32);
    if (now as i32) < 0 {
        t = t.fadd_m(4_294_967_296.0);
    }
    let t = x(rf(info, 0x64)).fadd(t);
    wf(s, 0x24, xi(ftol32(t)).to_f32());
    1
}

/// `CShotInfo::GetFlameThrowerShotPosn(uint8 id, CVector* out)` (0x739DE0): `out` = 0, then the
/// origin of an active flamethrower (type 37) shot: true.
#[hook(addr = 0x739DE0, conv = cdecl, name = "CShotInfo::GetFlameThrowerShotPosn", expect = "83 EC 0C 8B 4C 24 14")]
pub fn get_flame_thrower_shot_posn(id: u8, out: GPtr<u8>) -> u8 {
    wv_bits(out, 0, [0, 0, 0]);
    let s = shot(usize::from(id));
    if rb(s, 0x28) != 0 && ru(s, 0) == 0x25 {
        wv_bits(out, 0, rv_bits(s, 4));
        return 1;
    }
    0
}

/// `CShotInfo::Update()` (0x739E60): per shot (counter `i`): an invalid ped creator is cleared; for
/// active shots: deactivate once `(float)now > destroyTime` (or NaN), slow down by `_CIpow(0.96, ts)`
/// (flag 0x10000), widen by `ts * 0.075` (flag 0x80000; the original constant, gr has 0.0075), move;
/// with a creator, the peds of its scanner (entries 16..1, entry 0 never read) within `max(range, 1)`
/// of the squared distance get set on fire (or, for spray can / extinguisher and once per shot, a
/// damage event); then the spray can paints the world (direction reflected and doubled as the bytes
/// do, a frontend sound for the player on result 2), the extinguisher puts out fires, and other types
/// set cars (every 4th frame by `frame + i`) and the world on fire.
#[hook(addr = 0x739E60, conv = cdecl, name = "CShotInfo::Update", expect = "55 8B EC 83 E4 F8")]
pub fn shot_update() {
    for i in 0..NUM_SHOTS {
        let s = shot(i);
        let c = rp(s, 0x20);
        if !c.is_null() && etype(c) == 3 && ped_is_pointer_valid(c) == 0 {
            wu(s, 0x20, 0);
        }
        if rb(s, 0x28) == 0 {
            continue;
        }
        let info = get_weapon_info(ru(s, 0) as i32, 1);
        let flags = ru(info, 0x18);
        let now = time_ms();
        let mut f = Fpu::new();
        f.fild(now as i32);
        if (now as i32) < 0 {
            f.fadd_m(4_294_967_296.0);
        }
        f.fcomp_m(rf(s, 0x24));
        if !f.jne(0x41) {
            wb(s, 0x28, 0);
        }
        if flags & 0x10000 != 0 {
            let k = sa_world::physical::shared::ci_pow(X87::from_f64_exact(f64::from(f32::from_bits(0x3F75_C28F))), x(time_step()));
            wf(s, 0x10, k.fmul_m(rf(s, 0x10)).to_f32());
            wf(s, 0x14, k.fmul_m(rf(s, 0x14)).to_f32());
            wf(s, 0x18, k.fmul_m(rf(s, 0x18)).to_f32());
        }
        if flags & 0x80000 != 0 {
            wf(s, 0x1C, x(time_step()).fmul_m(f32::from_bits(0x3D99_999A)).fadd_m(rf(s, 0x1C)).to_f32());
        }
        let ts = time_step();
        let mx = x(ts).fmul_m(rf(s, 0x10));
        let my = x(ts).fmul_m(rf(s, 0x14));
        let mz = x(ts).fmul_m(rf(s, 0x18)).to_f32();
        wf(s, 4, mx.fadd_m(rf(s, 4)).to_f32());
        wf(s, 8, my.fadd_m(rf(s, 8)).to_f32());
        wf(s, 0xC, x(mz).fadd_m(rf(s, 0xC)).to_f32());
        if !rp(s, 0x20).is_null() {
            shot_hit_peds(s, info);
        }
        let ty = ru(s, 0);
        if ty == 0x29 {
            if rb(s, 0x29) != 0 {
                continue;
            }
            let mut n = [0f32; 3];
            let res = spray_paint_world(s.byte_add(4), lp(&mut n).cast(), rf(s, 0x1C), 1);
            if res == 0 {
                continue;
            }
            wb(s, 0x29, 1);
            let mut f = Fpu::new();
            f.fld(n[1]);
            f.fmul_m(rf(s, 0x14));
            f.fld(n[2]);
            f.fmul_m(rf(s, 0x18));
            f.faddp(1);
            f.fld(n[0]);
            f.fmul_m(rf(s, 0x10));
            f.faddp(1);
            f.fld(n[0]);
            f.fmul_st(1);
            f.fld(n[1]);
            f.fmul_st(2);
            let t44 = f.fstp();
            f.fld(n[2]);
            f.fmul_st(2);
            let t48 = f.fstp();
            f.fld(rf(s, 0x10));
            f.fsub_st(1);
            wf(s, 0x10, f.fstp());
            wf(s, 0x14, x(rf(s, 0x14)).fsub_m(t44).to_f32());
            wf(s, 0x18, x(rf(s, 0x18)).fsub_m(t48).to_f32());
            for off in [0x10, 0x14, 0x18] {
                let v = x(rf(s, off));
                wf(s, off, v.fadd(v).to_f32());
            }
            if res == 2 && rp(s, 0x20) == find_player_ped(-1) {
                report_frontend_audio_event(g(0xB6BC90), 0x2B, 0.0, 1.0);
            }
        } else if ty == 0x2A {
            if rb(s, 0x29) != 0 {
                continue;
            }
            let o = rv(s, 4);
            if extinguish_point_with_water(g(crate::fire::FIRE_MANAGER), o[0], o[1], o[2], rf(s, 0x1C), 2.0) != 0 {
                wb(s, 0x29, 1);
            }
        } else {
            let o = rv(s, 4);
            if frame_counter().wrapping_add(i as u32) & 3 == 0 {
                set_cars_on_fire(o[0], o[1], o[2], 4.0, rp(s, 0x20));
            }
            let o = rv(s, 4);
            set_world_on_fire(o[0], o[1], o[2], f32::from_bits(0x3DCC_CCCD), rp(s, 0x20));
        }
    }
}

/// The scanner loop of [`shot_update`] for a shot with a creator.
fn shot_hit_peds(s: GPtr<u8>, info: GPtr<u8>) {
    let mut r = rf(s, 0x1C);
    let mut f = Fpu::new();
    f.fld(r);
    f.fcomp_m(1.0);
    if !f.jp(5) {
        r = 1.0;
    }
    let scan = rp(rp(s, 0x20), 0x47C).byte_add(0x130);
    for n in (1..=16isize).rev() {
        let p = rp(scan, n * 4);
        if p.is_null() || ped_is_pointer_valid(p) == 0 || ru(p, 0x46C) & 0x100 != 0 {
            continue;
        }
        let pp = pos_ptr(p);
        let dx = x(rf(pp, 0)).fsub_m(rf(s, 4)).to_f32();
        let dy = x(rf(pp, 4)).fsub_m(rf(s, 8)).to_f32();
        let dz = x(rf(pp, 8)).fsub_m(rf(s, 0xC)).to_f32();
        if ru(p, 0x40) & 0x80000 != 0 || ped_is_ped_in_control(p) == 0 {
            continue;
        }
        let mut f = Fpu::new();
        f.fld(dz);
        f.fmul_m(dz);
        f.fld(dy);
        f.fmul_m(dy);
        f.faddp(1);
        f.fld(dx);
        f.fmul_m(dx);
        f.faddp(1);
        f.fcomp_m(r);
        if f.jp(5) {
            continue;
        }
        let ty = ru(s, 0);
        if ty != 0x29 && ty != 0x2A {
            start_fire_target(g(crate::fire::FIRE_MANAGER), p, rp(s, 0x20), f32::from_bits(0x3F4C_CCCD), 1, 7000, 100);
            continue;
        }
        if rb(s, 0x29) != 0 {
            continue;
        }
        let pp = pos_ptr(p);
        let cp = pos_ptr(rp(s, 0x20));
        let mut v = [x(rf(cp, 0)).fsub_m(rf(pp, 0)).to_f32(), x(rf(cp, 4)).fsub_m(rf(pp, 4)).to_f32()];
        let dir = ped_get_local_direction(p, lp(&mut v).cast());
        let dmg = i32::from(rh(info, 0x22) as i16);
        generate_damage_event(p, rp(s, 0x20), ru(s, 0) as i32, dmg, 3, dir);
        wb(s, 0x29, 1);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_BULLET_INITIALISE,
    HOOK_ADD_BULLET,
    HOOK_SHOT_INITIALISE,
    HOOK_ADD_SHOT,
    HOOK_GET_FLAME_THROWER_SHOT_POSN,
    HOOK_SHOT_UPDATE,
];
