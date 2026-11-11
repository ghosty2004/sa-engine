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
    pub fn add_explosion_call(victim: GPtr<u8>, creator: GPtr<u8>, ty: i32, x: f32, y: f32, z: f32, life: u32, sound: u8, shake: f32, invisible: u8) -> u8 = cdecl 0x736A50 => add_explosion;
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
                        add_explosion_call(victim, rp(e, 0x18), 2, vp[0], vp[1], vp[2], 0, 1, -1.0, 0);
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

orig! {
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, thiscall)
    pub fn clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, thiscall)
    pub fn register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CWorld::FindGroundZFor3DCoord(float x, float y, float z, bool* found, CEntity** e)` (0x5696C0, ST0)
    pub fn find_ground_z_for_3d_coord(x: f32, y: f32, z: f32, found: GPtr<u8>, e: GPtr<u8>) -> f64 = cdecl 0x5696C0 => unported;
    /// `CWaterLevel::GetWaterLevelNoWaves(float x, float y, float z, float* level, float*, float*)` (0x6E8580)
    pub fn get_water_level_no_waves(x: f32, y: f32, z: f32, level: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x6E8580 => unported;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, returns into `out`)
    pub fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CAEExplosionAudioEntity::AddAudioEvent(int32 event, CVector& pos, float volume)` (0x4DCBE0, thiscall)
    pub fn explosion_audio_add_event(ae: GPtr<u8>, ev: i32, pos: GPtr<u8>, vol: f32) = thiscall 0x4DCBE0 => unported;
    /// `CCrime::ReportCrime(eCrimeType, CEntity* victim, CPed* criminal)` (0x532010)
    pub fn report_crime(ty: i32, victim: GPtr<u8>, criminal: GPtr<u8>) = cdecl 0x532010 => unported;
    /// `CInterestingEvents::Add(EType, CEntity*)` (0x602590, thiscall)
    pub fn interesting_events_add(this: GPtr<u8>, ty: i32, e: GPtr<u8>) = thiscall 0x602590 => unported;
    /// `CStats::IncrementStat(eStats, float)` (0x55C180)
    pub fn stats_increment_stat(stat: u32, v: f32) = cdecl 0x55C180 => unported;
    /// `CShadows::AddPermanentShadow(uint8 type, RwTexture*, const CVector*, float frontX, float frontY,
    /// float sideX, float sideY, int16 intensity, uint8 r, g, b, float drawDistance, uint32 time,
    /// float upDistance)` (0x706F60); small integers as pushed words
    #[allow(clippy::too_many_arguments)]
    pub fn add_permanent_shadow(ty: u32, tex: u32, pos: GPtr<u8>, fx: f32, fy: f32, sx: f32, sy: f32, intensity: u32, r: u32, g: u32, b: u32, dist: f32, time: u32, up: f32) = cdecl 0x706F60 => unported;
    /// `CCamera::CamShake(float strength, float x, float y, float z)` (0x50A9F0, thiscall); raw position words
    pub fn cam_shake(cam: GPtr<u8>, strength: f32, x: u32, y: u32, z: u32) = thiscall 0x50A9F0 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn pad_get_pad(id: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::StartShake_Distance(int16 time, uint8 freq, float x, y, z)` (0x53F9A0, thiscall); raw words
    pub fn pad_start_shake_distance(pad: GPtr<u8>, time: u32, freq: u32, x: u32, y: u32, z: u32) = thiscall 0x53F9A0 => unported;
    /// `CGameLogic::IsCoopGameGoingOn()` (0x441390)
    pub fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
}

/// `g_fxMan`
const FX_MAN: usize = 0xA9AE80;
/// `CAEExplosionAudioEntity` of `CExplosion` (0xC888D0)
const EXPLOSION_AUDIO: usize = 0xC888D0;
/// `g_InterestingEvents`
const INTERESTING_EVENTS: usize = 0xC0B058;
/// `TheCamera`
const THE_CAMERA: usize = 0xB6F028;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// fx names in `.rdata`
const FX_SMALL: usize = 0x872BB8;
const FX_MOLOTOV: usize = 0x872BA4;
const FX_MEDIUM: usize = 0x872B90;
const FX_LARGE: usize = 0x872B80;
const FX_TINY: usize = 0x872B70;
/// `1 / 32767` (0x858C7C)
const RAND_K: u32 = 0x3800_0100;

/// The free-slot scan of `AddExplosion`: slot 0 when its active byte (+0x28) is 0, else `while (p <
/// &a[16].active) { c = p[next]; p = next; i++; if (!c) break; }`, which reads the active byte one element
/// past the array (0xC89138) when every slot is in use (original out-of-bounds read).
fn first_free() -> usize {
    let a0 = explosion(0).byte_add(0x28);
    if a0.read() == 0 {
        return 0;
    }
    let mut i = 0;
    while i < NUM_EXPLOSIONS {
        let c = a0.byte_add(((i + 1) * EXPLOSION_SIZE) as isize).read();
        i += 1;
        if c == 0 {
            break;
        }
    }
    i
}

/// `rand() * (1/32767)` as the x87 value (`fild; fmul [0x858C7C]`).
fn rand_unit(site: u32) -> X87 {
    xi(sa_core::orig::at_site(site, rand)).fmul_m(cf(RAND_K))
}

/// The fx matrix of `victim` (`RwFrame` modelling matrix: `[[victim+0x18]+4] + 0x10`), or null when the
/// entity has no RW object or frame (the original then skips the fx).
fn victim_matrix(victim: GPtr<u8>) -> GPtr<u8> {
    let rw = rp(victim, 0x18);
    if rw.is_null() {
        return GPtr::NULL;
    }
    let m = rp(rw, 4).byte_add(0x10);
    if m.is_null() { GPtr::NULL } else { m }
}

/// `CExplosion::AddExplosion(CEntity* victim, CEntity* creator, eExplosionType type, CVector pos, uint32
/// lifetime, bool makeSound, float camShake, bool invisible)` (0x736A50, cdecl, AL result). Follows the
/// bytes (gta-reversed `Explosion.cpp` and `CExplosion.deviations.md` as reading aids):
///
/// - created by player 0's ped: the focused player's info +0x140 += 5 and +0x148 += 7.0 (through the
///   protection chunk 0x40A13B, which loads `PlayerInFocus`);
/// - the first free slot (see [`first_free`]); none: false;
/// - type, pos, radius 1.0, +0x34 0, +0x38 0, +0x24 1.0; creator into +0x18 and victim into +0x1C with
///   `CleanUpOldReference` / `RegisterReference`; active 1, counter 1, sound, fuel timer 0;
/// - three fuel jets: jet 0 always, jets 1, 2 only when `rand() < 0x3FFF` (else speed 0): direction
///   `(2r-1, 2r-1, r*0.8+0.2)`, speed `r*1.5+0.5`, offset `r*10+20` (r = `rand()/32767`, five calls in
///   that order, x87 then stored as f32);
/// - trigger time `now + lifetime` (0 for no lifetime);
/// - by type: radius / +0x34 (0 / 0 when invisible), end time `(float)(uint32)(now + lifetime + d)`,
///   rate 0.5, fx: grenade, rocket (weak rocket: +0x34 200, +0x24 0.2 even when invisible), small
///   "explosion_small"; molotov: ground z under `z + 3` replaces z when found, below the water level the
///   explosion is marked (no fx, deactivated at the end), "explosion_molotov" with sound volume -36;
///   car / quick car: created time, crime 0x11 for the victim, "explosion_medium" at the vehicle model's
///   dummy +0x54 (needs a victim); boat / aircraft / tank fire: "explosion_large" (boat/aircraft also the
///   created time); mine: ground z under `z + 4` computed and discarded, sound only; object: sound only;
///   tiny "explosion_tiny". With a victim the fx is placed at `pos - victimPos` in the victim's frame
///   matrix and skipped when the victim has no RW object/frame; the sound (`AddAudioEvent(0x81)`) plays
///   only with `makeSound` and a created fx (mine/object: without fx);
/// - interesting event 0x1B for the victim; in water: inactive, false;
/// - fires: molotov `(rand() - 2) & 3`, rocket / weak rocket / object `(rand() + 1) & 3` fires around
///   the position (fires after the first moved by `r*8 - 4` in x then y), each where the ground (under
///   `z + 3`) is found within 10.0 of the explosion z: `StartFire(pos, 0.8, i != 0, creator,
///   _ftol(r*0.4*7000 + 5600), 3, 1)`; with any fire a player-0 ped creator increments stat 0xD8;
/// - permanent shadow at `pos + (0, 0, 5)`; with +0x34 != 0 (or NaN) and no trigger time
///   `CWorld::TriggerExplosion` now; camera shake `camShake` (-1.0: 0.2 for molotov, 0.6 else), and
///   except for molotov pad shakes for pad 0 and, in a coop game, pad 1; true.
///
/// Kept original: the out-of-bounds active-byte read when full; the weak-rocket fields set even for
/// invisible explosions; the mine's discarded ground probe; positions written back into the `pos`
/// argument (molotov ground z) are used by everything after. Not reproduced: the `StartFire` bool word's
/// upper bytes (ECX left by `_ftol`; the callee reads the byte).
#[hook(addr = 0x736A50, conv = cdecl, name = "CExplosion::AddExplosion", expect = "83 EC 1C 53 55 56 57")]
#[allow(clippy::too_many_arguments)]
pub fn add_explosion(victim: GPtr<u8>, creator: GPtr<u8>, ty: i32, px: f32, py: f32, pz: f32, life: u32, sound: u8, shake: f32, invisible: u8) -> u8 {
    let mut in_water: u8 = 0;
    let mut pos = [px, py, pz];
    let posp: GPtr<u8> = lp(&mut pos).cast();
    let mut shake = shake;
    if creator == find_player_ped(-1) {
        let n = usize::from(g::<u8>(0xB7CD74).read()) * 0x190;
        let c = g::<u32>(0xB7CED8 + n);
        c.write(c.read().wrapping_add(5));
        let f = g::<f32>(0xB7CEE0 + n);
        f.write(x(f.read()).fadd_m(7.0).to_f32());
    }
    let slot = first_free();
    if slot == NUM_EXPLOSIONS {
        return 0;
    }
    let e = explosion(slot);
    wu(e, 0, ty as u32);
    wv_bits(e, 4, [px.to_bits(), py.to_bits(), pz.to_bits()]);
    wu(e, 0x10, 0x3F80_0000);
    wu(e, 0x34, 0);
    wu(e, 0x38, 0);
    wu(e, 0x24, 0x3F80_0000);
    for (off, ent) in [(0x18isize, creator), (0x1C, victim)] {
        let old = rp(e, off);
        if !old.is_null() {
            clean_up_old_reference(old, e.byte_add(off));
        }
        wp(e, off, ent);
        if !ent.is_null() {
            register_reference(ent, e.byte_add(off));
        }
    }
    wb(e, 0x28, 1);
    wb(e, 0x29, 1);
    wb(e, 0x2A, sound);
    wu(e, 0x3C, 0);
    for i in 0..3isize {
        let speed = 0x64 + 4 * i;
        let dir = 0x40 + 0xC * i;
        if i != 0 && sa_core::orig::at_site(0x736B59, rand) >= 0x3FFF {
            wu(e, speed, 0);
            continue;
        }
        let r = rand_unit(0x736B71);
        wf(e, dir, r.fadd(r).fsub_m(1.0).to_f32());
        let r = rand_unit(0x736B8F);
        wf(e, dir + 4, r.fadd(r).fsub_m(1.0).to_f32());
        wf(e, dir + 8, rand_unit(0x736BAC).fmul_m(cf(0x3F4C_CCCD)).fadd_m(cf(0x3E4C_CCCD)).to_f32());
        wf(e, speed, rand_unit(0x736BCE).fmul_m(1.5).fadd_m(0.5).to_f32());
        wf(e, speed + 0xC, rand_unit(0x736BF0).fmul_m(10.0).fadd_m(20.0).to_f32());
    }
    let now = time_ms();
    wu(e, 0x30, if life != 0 { now.wrapping_add(life) } else { 0 });

    let set_size = |r10: u32, r34: u32| {
        if invisible != 0 {
            wu(e, 0x10, 0);
            wu(e, 0x34, 0);
        } else {
            wu(e, 0x10, r10);
            wu(e, 0x34, r34);
        }
    };
    let end_time = |d: u32| wf(e, 0x20, u32_x87(now.wrapping_add(life).wrapping_add(d)).to_f32());
    let created = || wf(e, 0x2C, xi(time_ms() as i32).to_f32());
    // (fx name, volume, at the vehicle dummy) or no fx
    let mut fx: Option<(usize, f32)> = None;
    let mut dummy = false;
    let mut sound_only = false;
    match ty {
        0 | 11 | 12 => {
            if ty == 0 {
                set_size(0x4110_0000, 0x4396_0000);
            } else {
                set_size(0x4040_0000, 0x42B4_0000);
            }
            end_time(0x2EE);
            wu(e, 0x14, 0x3F00_0000);
            fx = Some((if ty == 12 { FX_TINY } else { FX_SMALL }, 0.0));
        }
        1 => {
            wu(e, 0x34, 0);
            wu(e, 0x10, if invisible != 0 { 0 } else { 0x40C0_0000 });
            end_time(0xBB8);
            wu(e, 0x14, 0x3F00_0000);
            let mut found = [0u8; 1];
            let gz = st0(find_ground_z_for_3d_coord(pos[0], pos[1], x(pos[2]).fadd_m(3.0).to_f32(), lp(&mut found).cast(), GPtr::NULL));
            if found[0] != 0 {
                pos[2] = gz.to_f32();
            }
            let mut level = [0f32; 1];
            if get_water_level_no_waves(pos[0], pos[1], pos[2], lp(&mut level).cast(), GPtr::NULL, GPtr::NULL) != 0 && {
                let mut f = Fpu::new();
                f.fld(pos[2]);
                f.fcomp_m(level[0]);
                !f.jp(5)
            } {
                in_water = 1;
            } else {
                fx = Some((FX_MOLOTOV, -36.0));
            }
        }
        2 | 3 => {
            set_size(0x4120_0000, 0x4396_0000);
            end_time(0x2EE);
            wu(e, 0x14, 0x3F00_0000);
            if ty == 3 {
                wu(e, 0x34, 0x4348_0000);
                wu(e, 0x24, 0x3E4C_CCCD);
            }
            fx = Some((FX_SMALL, 0.0));
        }
        4 | 5 => {
            set_size(0x4110_0000, 0x4396_0000);
            end_time(0x109A);
            created();
            wu(e, 0x14, 0x3F00_0000);
            let v = rp(e, 0x1C);
            if !v.is_null() {
                report_crime(0x11, v, GPtr::NULL);
            }
            if !rp(e, 0x1C).is_null() {
                fx = Some((FX_MEDIUM, 0.0));
                dummy = true;
            }
        }
        6 | 7 | 10 => {
            if ty == 10 {
                set_size(0x4120_0000, 0x4316_0000);
                end_time(0x2EE);
            } else {
                set_size(0x41C8_0000, 0x4416_0000);
                end_time(0xBB8);
                created();
            }
            wu(e, 0x14, 0x3F00_0000);
            fx = Some((FX_LARGE, 0.0));
        }
        8 | 9 => {
            set_size(0x4120_0000, 0x4316_0000);
            end_time(0x2EE);
            if ty == 8 {
                wu(e, 0x14, 0x3F00_0000);
                find_ground_z_for_3d_coord(pos[0], pos[1], x(pos[2]).fadd_m(4.0).to_f32(), GPtr::NULL, GPtr::NULL);
            } else {
                wu(e, 0x14, 0x3F00_0000);
            }
            sound_only = true;
        }
        _ => {}
    }
    if sound_only && rb(e, 0x2A) != 0 {
        explosion_audio_add_event(g(EXPLOSION_AUDIO), 0x81, e.byte_add(4), 0.0);
    }
    if let Some((name, vol)) = fx {
        let mut rel = [0u32; 3];
        let relp: GPtr<u8> = lp(&mut rel).cast();
        let sys = if dummy {
            // the vehicle model's dummy position, then the RW checks of the victim argument
            let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + (i32::from(rh(rp(e, 0x1C), 0x22) as i16) as usize).wrapping_mul(4)).read();
            wv_bits(relp, 0, rv_bits(rp(mi, 0x5C), 0x54));
            let m = victim_matrix(victim);
            if m.is_null() { GPtr::NULL } else { create_fx_system(g(FX_MAN), g(FX_MEDIUM), relp, m, 0) }
        } else if !victim.is_null() {
            let m = victim_matrix(victim);
            if m.is_null() {
                GPtr::NULL
            } else {
                vector_sub(relp, posp, pos_ptr(victim));
                create_fx_system(g(FX_MAN), g(name), relp, m, 0)
            }
        } else {
            create_fx_system(g(FX_MAN), g(name), posp, GPtr::NULL, 0)
        };
        if !sys.is_null() {
            if rb(e, 0x2A) != 0 {
                explosion_audio_add_event(g(EXPLOSION_AUDIO), 0x81, e.byte_add(4), vol);
            }
            fx_play_and_kill(sys);
        }
    }
    if !victim.is_null() {
        interesting_events_add(g(INTERESTING_EVENTS), 0x1B, victim);
    }
    if in_water != 0 {
        wb(e, 0x28, 0);
        return 0;
    }
    let fires = match ty {
        1 => Some(sa_core::orig::at_site(0x7372B0, rand).wrapping_sub(2) & 3),
        2 | 3 | 9 => Some(sa_core::orig::at_site(0x7372BA, rand).wrapping_add(1) & 3),
        _ => None,
    };
    if let Some(n) = fires.filter(|&n| n != 0) {
        for i in 0..n {
            let mut t = rv(e, 4);
            if i != 0 {
                t[0] = rand_unit(0x7372F4).fmul_m(8.0).fadd_m(t[0]).fsub_m(4.0).to_f32();
                t[1] = rand_unit(0x73731B).fmul_m(8.0).fadd_m(t[1]).fsub_m(4.0).to_f32();
            }
            let mut found = [0u8; 1];
            t[2] = st0(find_ground_z_for_3d_coord(t[0], t[1], x(t[2]).fadd_m(3.0).to_f32(), lp(&mut found).cast(), GPtr::NULL)).to_f32();
            if found[0] == 0 {
                continue;
            }
            let mut f = Fpu::new();
            f.fld(t[2]);
            f.fsub_m(rf(e, 0xC));
            f.fabs();
            f.fcomp_m(10.0);
            if f.jp(5) {
                continue;
            }
            let s = rand_unit(0x73738B).fmul_m(cf(0x3ECC_CCCE)).fmul_m(7000.0).fadd_m(5600.0);
            let time = ftol32(s) as u32;
            crate::fire::start_fire_no_target_call(g(0xB71F80), t[0], t[1], t[2], cf(0x3F4C_CCCD), u8::from(i != 0), rp(e, 0x18), time, 3, 1);
        }
        if !creator.is_null() && etype(creator) == 3 && ru(creator, 0x598) == 0 {
            stats_increment_stat(0xD8, 1.0);
        }
    }
    let mut sp = [pos[0], pos[1], x(pos[2]).fadd_m(5.0).to_f32()];
    add_permanent_shadow(1, g::<u32>(0xC403E8).read(), lp(&mut sp).cast(), 8.0, 0.0, 0.0, -8.0, 0xC8, 0, 0, 0, 10.0, 0x7530, 1.0);
    let mut f = Fpu::new();
    f.fld(rf(e, 0x34));
    f.fcomp_m(0.0);
    if f.jp(0x44) && ru(e, 0x30) == 0 {
        let flag = u32::from(matches!(ty, 2 | 5 | 8 | 9 | 10));
        trigger_explosion(posp, ru(e, 0x10), ru(e, 0x34), victim, creator, flag, ru(e, 0x24));
    }
    let mut f = Fpu::new();
    f.fld(-1.0);
    f.fcomp_m(shake);
    let p = rv_bits(posp, 0);
    if ty == 1 {
        if !f.jp(0x44) {
            shake = cf(0x3E4C_CCCD);
        }
        cam_shake(g(THE_CAMERA), shake, p[0], p[1], p[2]);
        return 1;
    }
    if !f.jp(0x44) {
        shake = cf(0x3F19_999A);
    }
    cam_shake(g(THE_CAMERA), shake, p[0], p[1], p[2]);
    pad_start_shake_distance(pad_get_pad(0), 0x12C, 0x80, p[0], p[1], p[2]);
    if is_coop_game_going_on() != 0 {
        pad_start_shake_distance(pad_get_pad(1), 0x12C, 0x80, p[0], p[1], p[2]);
    }
    1
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
    HOOK_ADD_EXPLOSION,
];
