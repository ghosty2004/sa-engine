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
    /// `CWorld::ProcessLineOfSight(start, end, colPoint, entity, buildings, vehicles, peds, objects, dummies,
    /// seeThrough, camera, shootThrough)` (0x56BA00)
    pub fn process_line_of_sight(a: GPtr<u8>, b: GPtr<u8>, cp: GPtr<u8>, e: GPtr<u8>, bd: u8, vh: u8, pd: u8, ob: u8, du: u8, st: u8, cam: u8, sh: u8) -> u8 = cdecl 0x56BA00 => unported;
    /// `CWeapon::CheckForShootingVehicleOccupant(CEntity**, CColPoint*, eWeaponType, const CVector&, const CVector&)` (0x73F480)
    pub fn check_for_shooting_vehicle_occupant(e: GPtr<u8>, cp: GPtr<u8>, ty: i32, a: GPtr<u8>, b: GPtr<u8>) = cdecl 0x73F480 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60, returns out)
    pub fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `operator*(CVector* out, float, const CVector*)` (0x40FE90, returns out)
    pub fn vector_scale(out: GPtr<u8>, k: f32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `CCrime::ReportCrime(eCrimeType, CEntity* victim, CPed* commitedBy)` (0x532010)
    pub fn report_crime(ty: i32, victim: GPtr<u8>, by: GPtr<u8>) = cdecl 0x532010 => unported;
    /// `CLocalisation::Blood()` (0x56D230)
    pub fn localisation_blood() -> u8 = cdecl 0x56D230 => unported;
    /// `Fx_c::AddBlood(CVector& pos, CVector& dir, int32 count, float brightness)` (0x49EB00, thiscall)
    pub fn fx_add_blood(fx: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<u8>, n: i32, b: f32) = thiscall 0x49EB00 => unported;
    /// `RpAnimBlendClumpGetFirstAssociation(RpClump*, uint32 flags)` (0x4D6A70)
    pub fn clump_get_first_association(clump: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x4D6A70 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610)
    pub fn blend_animation(clump: GPtr<u8>, group: i32, id: i32, delta: f32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80, thiscall)
    pub fn assoc_set_current_time(a: GPtr<u8>, t: f32) = thiscall 0x4CEA80 => unported;
    /// `CVehicle::BurstTyre(uint8 component, bool)` (vtable +0xB0, thiscall; the dword holding the piece byte is pushed)
    pub fn vehicle_burst_tyre(veh: GPtr<u8>, piece: u32, b: u8) = thiscall vslot 0xB0 => unported;
    /// `Fx_c::AddTyreBurst(CVector& pos, CVector& vel)` (0x49F300, thiscall)
    pub fn fx_add_tyre_burst(fx: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>) = thiscall 0x49F300 => unported;
    /// `CVehicle::InflictDamage(CEntity* damager, eWeaponType, float damage, CVector coords)` (0x6D7C90, thiscall)
    pub fn vehicle_inflict_damage(veh: GPtr<u8>, by: GPtr<u8>, ty: i32, dmg: f32, x: f32, y: f32, z: f32) = thiscall 0x6D7C90 => unported;
    /// `CCamera::IsSphereVisible(const CVector& center, float radius)` (0x420D40, thiscall)
    pub fn camera_is_sphere_visible(cam: GPtr<u8>, c: GPtr<u8>, r: f32) -> u8 = thiscall 0x420D40 => unported;
    /// `tColLighting::GetCurrentLighting(float)` (0x59F0C0, cdecl with the dword holding the lighting byte pushed; ST0)
    pub fn col_lighting_get_current(l: u32, scale: f32) -> f64 = cdecl 0x59F0C0 => unported;
    /// `Fx_c::AddBulletImpact(CVector& pos, CVector& dir, int32 surface, int32 count, float brightness)` (0x49F3D0, thiscall)
    pub fn fx_add_bullet_impact(fx: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<u8>, surface: u32, n: i32, b: f32) = thiscall 0x49F3D0 => unported;
    /// `CObject::ObjectDamage(float, const CVector* pos, const CVector* dir, CEntity*, eWeaponType)` (0x5A0D90, thiscall)
    pub fn object_damage(obj: GPtr<u8>, dmg: f32, pos: GPtr<u8>, dir: GPtr<u8>, by: GPtr<u8>, ty: i32) = thiscall 0x5A0D90 => unported;
    /// `CEntity::GetIsStatic()` (0x4633E0, thiscall)
    pub fn entity_get_is_static(e: GPtr<u8>) -> u8 = thiscall 0x4633E0 => unported;
    /// `CEntity::SetIsStatic(bool)` (vtable +0x10, thiscall)
    pub fn entity_set_is_static(e: GPtr<u8>, b: u8) = thiscall vslot 0x10 => unported;
    /// `CPhysical::AddToMovingList()` (0x542800, thiscall)
    pub fn physical_add_to_moving_list(e: GPtr<u8>) = thiscall 0x542800 => unported;
    /// `CPhysical::ApplyMoveForce(CVector force)` (0x5429F0, thiscall)
    pub fn physical_apply_move_force(e: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x5429F0 => unported;
    /// `CAudioEngine::ReportBulletHit(CEntity*, eSurfaceType, const CVector&, float angle)` (0x506EC0, thiscall)
    pub fn audio_report_bullet_hit(ae: GPtr<u8>, e: GPtr<u8>, surface: u32, pos: GPtr<u8>, angle: f32) = thiscall 0x506EC0 => unported;
    /// `CGlass::WasGlassHitByBullet(CEntity*, CVector)` (0x71C0D0)
    pub fn glass_was_hit_by_bullet(e: GPtr<u8>, x: f32, y: f32, z: f32) = cdecl 0x71C0D0 => unported;
    /// `CHeli::TestSniperCollision(CVector* start, CVector* end)` (0x6C6890)
    pub fn heli_test_sniper_collision(a: GPtr<u8>, b: GPtr<u8>) = cdecl 0x6C6890 => unported;
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

/// `g_fx` (Fx_c)
const G_FX: usize = 0xA9AE00;
/// `TheCamera`
const THE_CAMERA: usize = 0xB6F028;
/// `AudioEngine`
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `CWorld::pIgnoreEntity`
const IGNORE_ENTITY: usize = 0xB7CD68;
/// `CWorld::bIncludeBikers`, `bIncludeCarTyres`, `bIncludeDeadPeds` (write order of the original)
const INCLUDE_FLAGS: [usize; 3] = [0xB7CD71, 0xB7CD70, 0xB7CD6F];
/// `PlayerSniperBulletStart` / `PlayerSniperBulletEnd`
const SNIPER_START: usize = 0xC888A0;
const SNIPER_END: usize = 0xC888AC;
/// The CRT's `_CIasin` (0x821E70; operand and result in ST0). The bytes compute
/// `fpatan(x, sqrt((1+x)(1-x)))`, i.e. asin (gta-reversed: `asin`). It keeps the caller's precision
/// control (`fldcw (cw & 0x300) | 0x7F`), so under the game's 24-bit precision its result is exact as a
/// double and [`sa_core::orig::x87_unary`] returns it unchanged. Its |x| = 1 path returns the 80-bit
/// constant pi/2, which `x87_unary` rounds to a double; `pi/2 * 57.29578f` rounds to the same f32
/// (89.99999237) from both (checked with exact rationals).
const CI_ASIN: usize = 0x821E70;

/// `CBulletInfo::Update()` (0x7360D0), ported from the 1.0 US bytes. Differences from gta-reversed
/// (the original is kept):
/// - an expired bullet (`(float)now > destroyTime`) is marked inactive but still moved and tested this
///   frame (gr `continue`s);
/// - the hit entity that `ProcessLineOfSight` fills is ONE local for the whole loop, zeroed once at
///   entry, and a bullet is deactivated at the end of its step iff it is non-null (so a later bullet
///   sees an earlier bullet's hit whenever `ProcessLineOfSight` leaves the value alone);
/// - only x and y are bounds-checked (`-3000 < v < 3000`, NaN passes);
/// - the step `t = timeStep * 0.5f` stays on the x87 stack: x = `t*vx + px` in extended precision,
///   y and z round `t*v` to f32 first;
/// - both `IsSphereVisible` calls take the collision point (gr: the normal);
/// - the dead-ped hit animation sets the association flags to `(f & 0xFFF7) | 1`;
/// - the sniper dot product is `(nx*dx + nz*dz) + ny*dy`, compared with 0 in extended precision; its
///   f32 copy goes to asin; the angle is `asin(-dot) * 57.29578f`.
///
/// Stack-only differences: `GetCurrentLighting` and the vehicle damage path write their own argument /
/// scratch slots (not reproduced).
#[hook(addr = 0x7360D0, conv = cdecl, name = "CBulletInfo::Update", expect = "81 EC 90 00 00 00")]
pub fn bullet_update() {
    let mut hit: GPtr<u8> = GPtr::NULL;
    for i in 0..NUM_BULLETS {
        bullet_step(bullet(i), &mut hit);
    }
}

fn bullet_step(b: GPtr<u8>, hit: &mut GPtr<u8>) {
    let c = rp(b, 4);
    if !c.is_null() && etype(c) == 3 && ped_is_pointer_valid(c) == 0 {
        wp(b, 4, GPtr::NULL);
    }
    if rb(b, 0xC) == 0 {
        return;
    }
    let now = time_ms();
    let mut f = Fpu::new();
    f.fild(now as i32);
    if (now as i32) < 0 {
        f.fadd_m(cf(0x4F80_0000));
    }
    f.fcomp_m(rf(b, 8));
    if !f.jne(0x41) {
        wb(b, 0xC, 0);
    }
    let t = x(time_step()).fmul_m(cf(0x3F00_0000));
    let old = rv(b, 0x10);
    let mut oldpos: [f32; 3] = old;
    let ty_ = t.fmul_m(rf(b, 0x20)).to_f32();
    let tz = t.fmul_m(rf(b, 0x24)).to_f32();
    let mut newpos: [f32; 3] = [
        t.fmul_m(rf(b, 0x1C)).fadd_m(old[0]).to_f32(),
        x(ty_).fadd_m(rf(b, 0x14)).to_f32(),
        x(tz).fadd_m(rf(b, 0x18)).to_f32(),
    ];
    // x, then y: out iff `v <= -3000` (`test ah, 0x41; jnp`) or `!(v < 3000)` (`test ah, 1; je`; NaN
    // sets C0 and stays in)
    for v in [newpos[0], newpos[1]] {
        if !x(v).fcom(X87::constant(0xC53B_8000)).test_parity(0x41) || !x(v).fcom(X87::constant(0x453B_8000)).test_any(0x01) {
            wb(b, 0xC, 0);
            return;
        }
    }
    let ty = ru(b, 0) as i32;
    g::<GPtr<u8>>(IGNORE_ENTITY).write(rp(b, 4));
    for a in INCLUDE_FLAGS {
        g::<u8>(a).write(1);
    }
    let mut cp = [0u8; 0x2C];
    let (oldp, newp, cpp) = (lp(&mut oldpos).cast::<u8>(), lp(&mut newpos).cast::<u8>(), lp(&mut cp).cast::<u8>());
    if process_line_of_sight(oldp, newp, cpp, lp(hit).cast(), 1, 1, 1, 1, 1, 0, 0, 1) != 0 {
        bullet_hit(b, ty, hit, cpp, oldp, newp);
    }
    for a in INCLUDE_FLAGS {
        g::<u8>(a).write(0);
    }
    g::<GPtr<u8>>(IGNORE_ENTITY).write(GPtr::NULL);
    let np = rv(newp, 0);
    if ru(b, 0) == 0x22 {
        wv(g(SNIPER_START), 0, rv(b, 0x10));
        wv(g(SNIPER_END), 0, np);
        heli_test_sniper_collision(g(SNIPER_START), g(SNIPER_END));
    }
    wv(b, 0x10, np);
    if !hit.is_null() {
        wb(b, 0xC, 0);
    }
}

/// The hit handling of [`bullet_update`] (0x736257..0x736779). `cpp` = the local `CColPoint` (point
/// +0, normal +0x10, surfaceB +0x23, pieceB +0x24, lightingB +0x25); the dwords at +0x23 / +0x24 /
/// +0x25 are pushed whole where the original pushes them.
fn bullet_hit(b: GPtr<u8>, ty: i32, hit: &mut GPtr<u8>, cpp: GPtr<u8>, oldp: GPtr<u8>, newp: GPtr<u8>) {
    let fx = g::<u8>(G_FX);
    let normal = cpp.byte_add(0x10);
    check_for_shooting_vehicle_occupant(lp(hit).cast(), cpp, ty, oldp, newp);
    let e = *hit;
    let impact = || {
        if camera_is_sphere_visible(g(THE_CAMERA), cpp, 1.0) != 0 {
            let l = st0(col_lighting_get_current(ru(cpp, 0x25), 0.5)).to_f32();
            fx_add_bullet_impact(fx, cpp, normal, u32::from(rb(cpp, 0x23)), 8, l);
        }
    };
    let set_new_to_point = || wv(newp, 0, rv(cpp, 0));
    let attached_to = |e: GPtr<u8>| {
        let c = rp(b, 4);
        !c.is_null() && etype(c) == 3 && rp(c, 0xFC) == e
    };
    match etype(e) {
        3 => {
            let state = ru(e, 0x530);
            if state != 0x36 && state != 0x37 && e != rp(b, 4) {
                let m = rp(e, 0x14);
                let pos = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) };
                let mut d = [0f32; 3];
                vector_sub(lp(&mut d).cast(), pos, cpp);
                let mut d2 = [d[0], d[1]];
                let dir = ped_get_local_direction(e, lp(&mut d2).cast());
                let dmg = i32::from(rh(b, 0x28) as i16);
                generate_damage_event(e, rp(b, 4), ty, dmg, i32::from(rb(cpp, 0x24)), i32::from(dir as u8));
                let crime = if ru(e, 0x598) == 6 { 5 } else { 4 };
                report_crime(crime, *hit, rp(b, 4));
                set_new_to_point();
            }
            if localisation_blood() != 0 {
                fx_add_blood(fx, cpp, normal, 8, rf(e, 0x12C));
                if ru(e, 0x530) == 0x37 {
                    let id = if clump_get_first_association(rp(e, 0x18), 0x800).is_null() { 0x24 } else { 0x27 };
                    let assoc = blend_animation(rp(e, 0x18), 0, id, 8.0);
                    if !assoc.is_null() {
                        assoc_set_current_time(assoc, 0.0);
                        wh(assoc, 0x2E, (rh(assoc, 0x2E) & 0xFFF7) | 1);
                    }
                }
                set_new_to_point();
            }
        }
        2 => {
            if !attached_to(e) {
                let piece = rb(cpp, 0x24);
                if (0xD..=0x10).contains(&piece) {
                    vehicle_burst_tyre(e, ru(cpp, 0x24), 1);
                    fx_add_tyre_burst(fx, cpp, normal);
                } else {
                    let p = rv(cpp, 0);
                    let dmg = xi(i32::from(rh(b, 0x28) as i16)).to_f32();
                    vehicle_inflict_damage(e, rp(b, 4), ru(b, 0) as i32, dmg, p[0], p[1], p[2]);
                    if ru(b, 0) == 0x25 {
                        start_fire_target(g(crate::fire::FIRE_MANAGER), *hit, rp(b, 4), cf(0x3F4C_CCCD), 1, 7000, 100);
                    } else {
                        impact();
                    }
                }
            }
        }
        _ => {
            impact();
            let e = *hit;
            if !attached_to(e) {
                let creator = rp(b, 4);
                match etype(e) {
                    4 => object_hit(b, e, creator, cpp, normal),
                    1 if !creator.is_null() && etype(creator) == 3 => {
                        let pd = rp(creator, 0x480);
                        if !pd.is_null() {
                            wu(pd, 0x9C, i32::from(rh(e, 0x22) as i16) as u32);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let e = *hit;
    if ru(b, 0) == 0x22 {
        let o = rv(oldp, 0);
        let n = rv(newp, 0);
        let mut d = [x(n[0]).fsub_m(o[0]).to_f32(), x(n[1]).fsub_m(o[1]).to_f32(), x(n[2]).fsub_m(o[2]).to_f32()];
        normalise(lp(&mut d).cast());
        let nv = rv(normal, 0);
        let dot = x(nv[0]).fmul_m(d[0]).fadd(x(nv[2]).fmul_m(d[2])).fadd(x(nv[1]).fmul_m(d[1]));
        let dot32 = dot.to_f32();
        // `fcomp 0.0; test ah, 5; jp` skips unless C0 alone (dot < 0)
        if !dot.fcom(X87::zero()).test_parity(0x05) {
            let a = st0(sa_core::orig::x87_unary(CI_ASIN, x(dot32).fchs().to_f64()));
            let angle = a.fmul_m(cf(0x4265_2EE0)).to_f32();
            audio_report_bullet_hit(g(AUDIO_ENGINE), e, ru(cpp, 0x23), cpp, angle);
        }
    }
    let p = rv(cpp, 0);
    glass_was_hit_by_bullet(e, p[0], p[1], p[2]);
}

/// The object branch of the hit handling (0x736576..0x736697).
fn object_hit(b: GPtr<u8>, o: GPtr<u8>, creator: GPtr<u8>, cpp: GPtr<u8>, normal: GPtr<u8>) {
    let ty = ru(b, 0) as i32;
    if rb(o, 0x144) >= 200 {
        let info = rp(o, 0x160);
        match ru(info, 0x48) {
            1 => object_damage(o, 151.0, cpp, normal, creator, ty),
            2 => object_damage(o, x(rf(info, 0x34)).fmul_m(cf(0x4317_0000)).to_f32(), cpp, normal, creator, ty),
            _ => {}
        }
        object_damage(o, 50.0, cpp, normal, rp(b, 4), ru(b, 0) as i32);
        return;
    }
    if rb(o, 0x40) & 4 == 0 {
        let info = rp(o, 0x160);
        // `fcomp 99.9f; test ah, 5; jp`: only `< 99.9` continues
        if !x(rf(info, 0x18)).fcom(X87::constant(0x42C7_CCCD)).test_parity(0x05) {
            if entity_get_is_static(o) != 0 {
                // `fcomp 0.0; test ah, 0x41; jp`: `<= 0` continues
                if !x(rf(info, 0x14)).fcom(X87::zero()).test_parity(0x41) {
                    entity_set_is_static(o, 0);
                    physical_add_to_moving_list(o);
                }
            }
            if entity_get_is_static(o) == 0 {
                let mut f = [0f32; 3];
                vector_scale(lp(&mut f).cast(), cf(0xC0F0_0000), normal);
                physical_apply_move_force(o, f[0], f[1], f[2]);
            }
            object_damage(o, 50.0, cpp, normal, rp(b, 4), ru(b, 0) as i32);
            return;
        }
    }
    object_damage(o, 50.0, cpp, normal, creator, ty);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_BULLET_INITIALISE,
    HOOK_ADD_BULLET,
    HOOK_SHOT_INITIALISE,
    HOOK_ADD_SHOT,
    HOOK_GET_FLAME_THROWER_SHOT_POSN,
    HOOK_SHOT_UPDATE,
    HOOK_BULLET_UPDATE,
];
