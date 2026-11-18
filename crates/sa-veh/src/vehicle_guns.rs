//! FRONTIER-2 turn 10, lane B (batch `frontier2-b23`): the vehicles' guns, `CVehicle::FireFixedMachineGuns`
//! (0x73DF00), the heat-seeking missile `CVehicle::FireHeatSeakingMissile` (0x6E05C0) and its target, `CVehicle::ScanAndMarkTargetForHeatSeekingMissile` (0x6E0400).
//! Ported from the 1.0 US bytes with docs/re/specs/VEH/CVehicle.md as reading aid (gta-reversed has no
//! body).

use sa_core::abi::HookReg;
use sa_core::bindings::matrix_mul_vec;
use sa_core::field::{gf, gr32, ptr_of as vp};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{f32_of, g, rand_at, rd, wr, x, xi};
use crate::mvpd_v1::read_vec;

use crate::bindings::weapon_do_tank_doom_aiming as do_tank_doom_aiming;
use crate::bindings::weapon_fire_one_instant_hit_round;
use crate::bindings::get_plane_num_guns;
use crate::frontier_b2_veh2::{get_plane_ordnance_position, get_plane_ordnance_rate_of_fire, gun_dummy_position, player_driver_pad, shake_and_stamp};
use sa_core::bindings::{crt_new_821195 as operator_new, fx_play, get_pad};
use sa_world::bindings::{add_gunshell, add_light, fire_instant_hit, fx_create_system, fx_set_offset_pos, get_modelling_matrix, pad_start_shake};
use sa_math::fpu::lt;
use sa_world::bindings::add_projectile;
use sa_world::bindings::{weapon_ctor, weapon_fire_projectile};
use sa_core::bindings::world_get_is_line_of_sight_clear;
use sa_pop::bindings::clear_cross_hair_immediately;
use sa_world::bindings::pick_target_for_heat_seeking_missile;
use sa_world::bindings::report_weapon_event;
use sa_world::bindings::weapon_effects_mark_target;
use sa_world::world::common::entity_pos;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7_CB84;
/// `AudioEngine`
const AUDIO_ENGINE: usize = 0xB6_BC90;
/// `g_fxMan`
const FX_MAN: usize = 0xA9_AE80;
/// "gunflash"
const S_GUNFLASH: usize = 0x85_A728;

/// `((rand() & 0xFF) - 128)` as the bytes load it (`fild` of the int).
fn spread(site: u32) -> X87 {
    xi((rand_at(site) & 0xFF) - 0x80)
}

/// One fixed gun: `start = matrix * offset`, `end = start + (dx, dy, 0)` (the caller's sum), the rand() spread
/// (x from the second call, y from the first, z from the third; x / y f32 offsets, z unrounded), the aim
/// correction and the shot.
fn fire_gun(this: GPtr<u8>, offset: [f32; 3], end_xy: impl Fn(CVector) -> (f32, f32), sites: [u32; 3]) {
    let mut v = CVector::new(offset[0], offset[1], offset[2]);
    let mut out = CVector::default();
    let mut start = read_vec(matrix_mul_vec(vp(&mut out), rd::<GPtr<u8>>(this, 0x14), vp(&mut v)));
    let (ex, ey) = end_xy(start);
    let r1 = rand_at(sites[0]);
    let r2 = rand_at(sites[1]);
    let k = gf(0x85_9F00);
    let ox = f32_of(xi((r2 & 0xFF) - 0x80).fmul_m(k));
    let oy = f32_of(xi((r1 & 0xFF) - 0x80).fmul_m(k));
    let oz = spread(sites[2]).fmul_m(gf(0x85_8B38));
    let mut end = CVector::new(f32_of(x(ox).fadd_m(ex)), f32_of(x(ey).fadd_m(oy)), f32_of(x(start.z).fadd(oz)));
    do_tank_doom_aiming(this, rd::<GPtr<u8>>(this, 0x460), vp(&mut start), vp(&mut end));
    weapon_fire_one_instant_hit_round(vp(&mut start).cast(), vp(&mut end).cast(), 15);
}

/// `CVehicle::FireFixedMachineGuns()` (0x73DF00, thiscall, plain `ret`).
///
/// Not before `m_nGunFiringTime (+0x4D4) + 150` (unsigned). The 2D forward direction `(fx, fy) / max(|(fx, fy)|,
/// 0.1)` (stored f32) times 60 is the reach: the right gun (2.0, 2.5, 1.0) adds the unrounded products, the left
/// gun (-2.0, 2.5, 1.0) their f32 stores. Each gun: three rand() spreads (x / y * 0.015, z * 0.02),
/// `CWeapon::DoTankDoomAiming(this, driver, start, end)` and `FireOneInstantHitRound(start, end, 15)`. Then
/// `ReportWeaponEvent(0x95, 31, this)`; the clip (+0x4AB) empties after 20 shots: 20 again and a 1400 ms pause.
#[hook(addr = 0x73DF00, conv = thiscall, name = "CVehicle::FireFixedMachineGuns", this = "CVehicle",
       expect = "8B 15 84 CB B7 00", unverified = "frontier2-b23", route = "L2", hazards = "none",
       note = "consumes six rand() values")]
pub fn fire_fixed_machine_guns(this: GPtr<u8>) {
    let now = gr32(TIME_MS);
    if now <= rd::<u32>(this, 0x4D4).wrapping_add(0x96) {
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let (fx, fy) = (x(rd::<f32>(m, 0x10)), x(rd::<f32>(m, 0x14)));
    let mut len = fy.fmul(fy).fadd(fx.fmul(fx)).fsqrt();
    let tenth = x(gf(0x85_8B1C));
    // fld 0.1; fcomp st(1); `test ah, 0x41; jne` keeps the length when 0.1 <= it (or NaN)
    if !tenth.fcom(len).test_any(0x41) {
        len = tenth;
    }
    let inv = len.fdivr_m(1.0);
    wr::<u32>(this, 0x4D4, now);
    let dx = f32_of(inv.fmul(fx));
    let dy = f32_of(inv.fmul(fy));
    let sixty = gf(0x85_8B34);
    let dx60 = x(dx).fmul_m(sixty);
    let dy60 = x(dy).fmul_m(sixty);
    let (dx60f, dy60f) = (f32_of(dx60), f32_of(dy60));
    fire_gun(
        this,
        [2.0, 2.5, 1.0],
        |s| (f32_of(dx60.fadd_m(s.x)), f32_of(x(s.y).fadd(dy60))),
        [0x73_E015, 0x73_E01C, 0x73_E05B],
    );
    fire_gun(
        this,
        [-2.0, 2.5, 1.0],
        |s| (f32_of(x(dx60f).fadd_m(s.x)), f32_of(x(dy60f).fadd_m(s.y))),
        [0x73_E153, 0x73_E15A, 0x73_E199],
    );
    report_weapon_event(GPtr::from_addr(AUDIO_ENGINE), 0x95, 0x1F, this.addr() as u32);
    let clip = rd::<u8>(this, 0x4AB).wrapping_sub(1);
    wr::<u8>(this, 0x4AB, clip);
    if clip == 0 {
        wr::<u8>(this, 0x4AB, 0x14);
        wr::<u32>(this, 0x4D4, gr32(TIME_MS).wrapping_add(0x578));
    }
}

/// `CVehicle::ScanAndMarkTargetForHeatSeekingMissile(CEntity* preferred)` (0x6E0400, thiscall, `ret 4`).
///
/// `target = CWeapon::PickTargetForHeatSeekingMissile(position, forward, 1.2, this, true, preferred)`. A plane
/// (sub-type 4) keeps a bit field at +0x9E4: bits 0..30 the "checked" mark, bit 31 the line of sight. With a target
/// and `now - (mark == 0) > 1000` (unsigned; the mark only says whether it is zero) the line of sight from this
/// vehicle to the target is tested with both collision flags (+0x1C bit 0) off (`GetIsLineOfSightClear(.., 1, 1, 0,
/// 1, 0, 1, 0)`, the flags restored); a plane stores `now > 1` as the mark and the result as bit 31. Otherwise a
/// plane takes bit 31, any other vehicle a clear line. A blocked line, no target or a vehicle not driven by the
/// player (status +0x36 & 0xF8 not 0) clears crosshair 0 and returns null (gta-reversed returns the target for a
/// non-player); else `CWeaponEffects::MarkTarget(0, target position, 255, 255, 255, 100, 1.3, true)` and the target.
#[hook(addr = 0x6E0400, conv = thiscall, name = "CVehicle::ScanAndMarkTargetForHeatSeekingMissile", this = "CVehicle",
       expect = "53 55 56 8B F1", unverified = "frontier2-b23", route = "L2", hazards = "heap-writes",
       note = "the crosshair statics of CWeaponEffects 0")]
pub fn scan_and_mark_target_for_heat_seeking_missile(this: GPtr<u8>, preferred: GPtr<u8>) -> GPtr<u8> {
    let p = read_vec(entity_pos(this));
    let f = read_vec(rd::<GPtr<u8>>(this, 0x14).byte_add(0x10));
    let target = pick_target_for_heat_seeking_missile(p.x, p.y, p.z, f.x, f.y, f.z, f32::from_bits(0x3F99_999A), this, 1, preferred);
    let plane = rd::<u32>(this, 0x594) == 4;
    let last = if plane { u32::from(rd::<u32>(this, 0x9E4) & 0x7FFF_FFFF == 0) } else { 0 };
    let clear = if !target.is_null() && gr32(TIME_MS).wrapping_sub(last) > 1000 {
        let own = rd::<u8>(this, 0x1C) & 1;
        let theirs = rd::<u8>(target, 0x1C) & 1;
        wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) & !1);
        wr::<u32>(target, 0x1C, rd::<u32>(target, 0x1C) & !1);
        let los = world_get_is_line_of_sight_clear(entity_pos(this), entity_pos(target), 1, 1, 0, 1, 0, 1, 0);
        wr::<u32>(this, 0x1C, (rd::<u32>(this, 0x1C) & !1) | u32::from(own));
        wr::<u32>(target, 0x1C, (rd::<u32>(target, 0x1C) & !1) | u32::from(theirs));
        if plane {
            let mark = u32::from(gr32(TIME_MS) > 1);
            wr::<u32>(this, 0x9E4, mark | (u32::from(los) << 31));
        }
        los != 0
    } else if plane {
        rd::<u32>(this, 0x9E4) >> 31 != 0
    } else {
        true
    };
    if !clear || target.is_null() || rd::<u8>(this, 0x36) & 0xF8 != 0 {
        clear_cross_hair_immediately(0);
        return GPtr::NULL;
    }
    let t = read_vec(entity_pos(target));
    weapon_effects_mark_target(0, t.x.to_bits(), t.y.to_bits(), t.z.to_bits(), 0xFF, 0xFF, 0xFF, 0x64, 0x3FA6_6666, 1);
    target
}

/// `CVehicle::FireHeatSeakingMissile(CEntity* target, eOrdnanceType type, bool useRateLimit)` (0x6E05C0, thiscall,
/// `ret 0xC`).
///
/// With the rate limit, nothing until the type's rate of fire has passed since its last shot (+0x4FC for type 1, else
/// +0x500; unsigned). The ordnance offset (`GetPlaneOrdnancePosition`, x negated while the side bits 2..3 of +0x4A9 are
/// set; the bits become 1 when they were 0, else 0) is added to the position (not transformed), plus this frame's
/// forward travel `max(0, moveSpeed . forward) * forward * timeStep` (x and y of the product stored as f32 before the
/// time step, the sums kept in x87 as the bytes order them); `AddProjectile(this, WEAPON_ROCKET_HS, position, 0,
/// &forward, target)`; then the pad shake and the shot time as for the free-fall bomb.
#[hook(addr = 0x6E05C0, conv = thiscall, name = "CVehicle::FireHeatSeakingMissile", this = "CVehicle",
       expect = "83 EC 24 53 8B 5C 24 30", unverified = "frontier2-b23", route = "L2", hazards = "heap-writes",
       note = "AddProjectile (projectile pool)")]
pub fn fire_heat_seaking_missile(this: GPtr<u8>, target: GPtr<u8>, ty: u32, use_rate: u32) {
    let last = if ty == 1 { rd::<u32>(this, 0x4FC) } else { rd::<u32>(this, 0x500) };
    if use_rate as u8 != 0 && gr32(TIME_MS) <= (get_plane_ordnance_rate_of_fire(this, ty) as u32).wrapping_add(last) {
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let p = read_vec(entity_pos(this));
    let mut o = CVector::default();
    get_plane_ordnance_position(this, vp(&mut o), ty);
    let flags = rd::<u8>(this, 0x4A9);
    let side = (flags >> 2) & 3;
    let ox = if side != 0 { x(o.x).fchs() } else { x(o.x) };
    wr::<u8>(this, 0x4A9, (((u8::from(side == 0) << 2) ^ flags) & 0xC) ^ flags);
    let f = read_vec(m.byte_add(0x10));
    let dot = || {
        x(rd::<f32>(this, 0x4C)).fmul_m(f.z).fadd(x(rd::<f32>(this, 0x48)).fmul_m(f.y)).fadd(x(rd::<f32>(this, 0x44)).fmul_m(f.x))
    };
    let k = if lt(dot(), x(0.0)) { x(0.0) } else { dot() };
    let ts = gf(0xB7_CB5C);
    let kfy = f32_of(k.fmul_m(f.y));
    let kfz = f32_of(k.fmul_m(f.z));
    let a = k.fmul_m(f.x).fmul_m(ts);
    let b = x(kfy).fmul_m(ts);
    let cz = f32_of(x(kfz).fmul_m(ts));
    let ax = f32_of(a.fadd(ox));
    let yy = x(o.y).fadd(b);
    let zz = x(o.z).fadd_m(cz);
    let px = f32_of(x(p.x).fadd_m(ax));
    let py = f32_of(x(p.y).fadd(yy));
    let pz = f32_of(x(p.z).fadd(zz));
    let mut fwd = f;
    add_projectile(this, 0x14, px, py, pz, 0.0, vp(&mut fwd), target);
    shake_and_stamp(this, ty);
}

/// This frame's forward travel as FireUnguidedMissile stores it: `k = max(0, moveSpeed . forward)` (x87; NaN keeps the
/// dot), `(f32(k * f.x * timeStep), f32(f32(k * f.y) * timeStep), f32(f32(k * f.z) * timeStep))`.
fn forward_travel(this: GPtr<u8>, f: CVector) -> CVector {
    let dot = || x(rd::<f32>(this, 0x4C)).fmul_m(f.z).fadd(x(rd::<f32>(this, 0x48)).fmul_m(f.y)).fadd(x(f.x).fmul_m(rd::<f32>(this, 0x44)));
    let k = if lt(dot(), x(0.0)) { x(0.0) } else { dot() };
    let ts = gf(0xB7_CB5C);
    let kfy = f32_of(k.fmul_m(f.y));
    let kfz = f32_of(k.fmul_m(f.z));
    CVector::new(f32_of(k.fmul_m(f.x).fmul_m(ts)), f32_of(x(kfy).fmul_m(ts)), f32_of(x(kfz).fmul_m(ts)))
}

/// One rocket of FireUnguidedMissile: `shell = matrix * pos + travel` (f32 per component) into the ordnance
/// position's slot, then `weapon.FireProjectile(this, &shell, null, null, 0.0)`.
fn fire_rocket(this: GPtr<u8>, weapon: GPtr<u8>, pos: &mut CVector) {
    let m = rd::<GPtr<u8>>(this, 0x14);
    let v = forward_travel(this, read_vec(m.byte_add(0x10)));
    let mut out = CVector::default();
    let t = read_vec(matrix_mul_vec(vp(&mut out), m, vp(pos)));
    *pos = CVector::new(f32_of(x(v.x).fadd_m(t.x)), f32_of(x(v.y).fadd_m(t.y)), f32_of(x(v.z).fadd_m(t.z)));
    weapon_fire_projectile(weapon, this, vp(pos), GPtr::NULL, GPtr::NULL, 0.0);
}

/// `CVehicle::FireUnguidedMissile(eOrdnanceType type, bool useRateLimit)` (0x6D5110, thiscall, `ret 8`).
///
/// With the rate limit, nothing until the type's rate of fire has passed since its last shot (+0x4FC for type 1, else
/// +0x500; unsigned). Only the Hunter (425), Hydra (520) and Tornado (576): a `CWeapon(WEAPON_ROCKET, 5000)` fires
/// a rocket from the ordnance position and one from its x mirror (the position recomputed), each moved by the matrix
/// and this frame's forward travel ([`forward_travel`], computed for each). Then the pad shake and the shot time as
/// for the free-fall bomb. The weapon is not destroyed (the bytes call no destructor).
#[hook(addr = 0x6D5110, conv = thiscall, name = "CVehicle::FireUnguidedMissile", this = "CVehicle",
       expect = "83 EC 4C 53 8B 5C 24 54", unverified = "frontier2-b25", route = "L2", hazards = "heap-writes",
       note = "FireProjectile (projectile pool)")]
pub fn fire_unguided_missile(this: GPtr<u8>, ty: u32, use_rate: u32) {
    let last = if ty == 1 { rd::<u32>(this, 0x4FC) } else { rd::<u32>(this, 0x500) };
    if use_rate as u8 != 0 && gr32(TIME_MS) <= (get_plane_ordnance_rate_of_fire(this, ty) as u32).wrapping_add(last) {
        return;
    }
    if !matches!(rd::<i16>(this, 0x22), 425 | 520 | 576) {
        return;
    }
    let mut weapon = [0u32; 7];
    let w = GPtr::from_raw(weapon.as_mut_ptr()).cast::<u8>();
    weapon_ctor(w, 0x23, 5000);
    let mut pos = CVector::default();
    get_plane_ordnance_position(this, vp(&mut pos), ty);
    fire_rocket(this, w, &mut pos);
    let mut p2 = CVector::default();
    let r = read_vec(get_plane_ordnance_position(this, vp(&mut p2), ty));
    pos = CVector::new(f32_of(x(r.x).fchs()), r.y, r.z);
    fire_rocket(this, w, &mut pos);
    shake_and_stamp(this, ty);
}

orig! {
    /// `CVehicle::GetPlaneGunsRateOfFire()` (0x6D40E0)
    fn get_plane_guns_rate_of_fire(v: GPtr<u8>) -> i32 = thiscall 0x6D40E0 => crate::vehicle_fire_rates_upgrades::vehicle_get_plane_guns_rate_of_fire;
    /// `CVehicle::GetPlaneGunsPosition(CVector* out, int32 gunId)` (0x6D4290)
    fn get_plane_guns_position_call(v: GPtr<u8>, out: GPtr<u8>, gun: i32) -> GPtr<u8> = thiscall 0x6D4290 => get_plane_guns_position;
    /// `CVehicle::DoPlaneGunFireFX(CWeapon*, CVector& particlePos, CVector& gunshellPos, int32 fxIdx)` (0x6D4AD0)
    fn do_plane_gun_fire_fx_call(v: GPtr<u8>, weapon: GPtr<u8>, particle: GPtr<u8>, shell: GPtr<u8>, idx: i32) = thiscall 0x6D4AD0 => do_plane_gun_fire_fx;
}

/// `CVehicle::GetPlaneGunsPosition(CVector* out, int32 gunId)` (0x6D4290, thiscall, `ret 8`, returns `out`).
///
/// The vehicle structure's gun dummy when any coordinate is non-zero, else by model (jump table 0x6D44C4 / 0x6D44EC):
/// 425 0x8D35D4, 447 0x8D35F8, 464 0x8D3634, 476 0x8D3610, 487 0x8D35E0, 497 0x8D35EC, 520 0x8D3628, 548 0x8D3604,
/// 576 the static 0xC1CC2C, any other model 0. The Rustler (476, the `cmp dx, 0x1DC` of the fragment 0x407A15) then
/// adds `(gunId - 1) * (0.2, 0, 0)` (0x8D361C): x through the unrounded product, y / z through their f32 stores.
#[hook(addr = 0x6D4290, conv = thiscall, name = "CVehicle::GetPlaneGunsPosition", this = "CVehicle",
       expect = "83 EC 18 66 8B 51 22", unverified = "frontier2-b26", route = "L2", hazards = "none")]
pub fn get_plane_guns_position(this: GPtr<u8>, out: GPtr<u8>, gun: i32) -> GPtr<u8> {
    let model = rd::<i16>(this, 0x22);
    let (mut v, set) = gun_dummy_position(this);
    if !set {
        let src = match model {
            425 => 0x8D_35D4,
            447 => 0x8D_35F8,
            464 => 0x8D_3634,
            476 => 0x8D_3610,
            487 => 0x8D_35E0,
            497 => 0x8D_35EC,
            520 => 0x8D_3628,
            548 => 0x8D_3604,
            576 => 0xC1_CC2C,
            _ => 0,
        };
        v = if src == 0 { [0; 3] } else { [gr32(src), gr32(src + 4), gr32(src + 8)] };
    }
    if model == 476 {
        let n = xi(gun.wrapping_sub(1));
        let k = |a: usize| n.fmul_m(gf(a));
        let fy = f32_of(k(0x8D_3620));
        let fz = f32_of(k(0x8D_3624));
        let f = |b: u32| f32::from_bits(b);
        v = [
            f32_of(k(0x8D_361C).fadd_m(f(v[0]))).to_bits(),
            f32_of(x(fy).fadd_m(f(v[1]))).to_bits(),
            f32_of(x(fz).fadd_m(f(v[2]))).to_bits(),
        ];
    }
    for (i, b) in v.iter().enumerate() {
        wr::<u32>(out, 4 * i as isize, *b);
    }
    out
}

/// `CVehicle::DoPlaneGunFireFX(CWeapon* weapon, CVector& particlePos, CVector& gunshellPos, int32 fxIdx)` (0x6D4AD0,
/// thiscall, `ret 0x10`).
///
/// Planes (sub-type 4) keep their gun flashes at +0x9D4, helis (3) at +0xA0C; any other vehicle returns at once. The
/// array (`new FxSystem_c*[GetPlaneNumGuns()]`, zeroed with the count re-read per entry) is made on first use; a
/// missing entry gets `CreateFxSystem("gunflash", (0, 0, 0), modelling matrix, false)`; an entry then gets
/// `SetOffsetPos(particlePos)` and `Play()`. Then (also without an array or a system) the shell casing when
/// `s_bPlaneGunsEjectShellCasings` (0xC1CC19): `weapon.AddGunshell(this, gunshellPos, (0, 0.1), 0.5)`, and
/// `CPointLights::AddLight(0, gunshellPos, 0, 3.0, 0.5, 0.4, 0, 0, 0, null)`.
#[hook(addr = 0x6D4AD0, conv = thiscall, name = "CVehicle::DoPlaneGunFireFX", this = "CVehicle",
       expect = "83 EC 18 56 8B F1", unverified = "frontier2-b26", route = "L2", hazards = "heap-writes",
       note = "the gun flash fx systems (fx manager pool)")]
pub fn do_plane_gun_fire_fx(this: GPtr<u8>, weapon: GPtr<u8>, particle: GPtr<u8>, shell: GPtr<u8>, idx: i32) {
    let slot = match rd::<u32>(this, 0x594) {
        4 => 0x9D4,
        3 => 0xA0C,
        _ => return,
    };
    if rd::<GPtr<u8>>(this, slot).is_null() {
        let n = get_plane_num_guns(this);
        wr::<GPtr<u8>>(this, slot, operator_new(n.wrapping_shl(2)));
        let mut i = 0i32;
        while i < get_plane_num_guns(this) as i32 {
            wr::<u32>(rd::<GPtr<u8>>(this, slot), 4 * i as isize, 0);
            i += 1;
        }
    }
    if !rd::<GPtr<u8>>(this, slot).is_null() {
        let e = |this: GPtr<u8>| rd::<GPtr<u8>>(this, slot).byte_add(4 * idx as isize);
        if rd::<GPtr<u8>>(e(this), 0).is_null() {
            let mut zero = CVector::default();
            let fx = fx_create_system(GPtr::from_addr(FX_MAN), GPtr::from_addr(S_GUNFLASH), vp(&mut zero), get_modelling_matrix(this), 0);
            wr::<GPtr<u8>>(e(this), 0, fx);
        }
        if !rd::<GPtr<u8>>(e(this), 0).is_null() {
            fx_set_offset_pos(rd::<GPtr<u8>>(e(this), 0), particle);
            fx_play(rd::<GPtr<u8>>(e(this), 0));
        }
    }
    if g::<u8>(0xC1_CC19).read() != 0 {
        let mut dir = [0.0f32, f32::from_bits(0x3DCC_CCCD)];
        add_gunshell(weapon, this, shell, GPtr::from_raw(dir.as_mut_ptr()), 0.5);
    }
    let p = read_vec(shell);
    add_light(0, p.x, p.y, p.z, 0.0, 0.0, 0.0, 3.0, 0.5, f32::from_bits(0x3ECC_CCCD), 0.0, 0, 0, GPtr::NULL);
}

/// The shell position of a plane gun: `f32(f32(timeStep * moveSpeed) + matrix * pos)` per component.
fn gun_shell(this: GPtr<u8>, pos: &mut CVector) -> CVector {
    let ts = gf(0xB7_CB5C);
    let v = |o: isize| f32_of(x(ts).fmul_m(rd::<f32>(this, o)));
    let (vx, vy, vz) = (v(0x44), v(0x48), v(0x4C));
    let mut out = CVector::default();
    let t = read_vec(matrix_mul_vec(vp(&mut out), rd::<GPtr<u8>>(this, 0x14), vp(pos)));
    CVector::new(f32_of(x(vx).fadd_m(t.x)), f32_of(x(vy).fadd_m(t.y)), f32_of(x(vz).fadd_m(t.z)))
}

/// `CVehicle::FirePlaneGuns()` (0x6D4D30, thiscall, plain `ret`).
///
/// Nothing without guns or before `m_nGunFiringTime (+0x4D4) + GetPlaneGunsRateOfFire()` (unsigned). By model (jump
/// table 0x6D505C / 0x6D5070): 425 / 548 one minigun (38), shake frequency 160; 447 / 464 / 487 / 497 one M4 (31),
/// 92; 476 two M4s, 224, its gun cycle (+0x4A9 bits 0..1) stepped 0 -> 1 -> 2 -> 0; 520 / 576 two miniguns, 128;
/// any other model returns. A `CWeapon(type, 5000)` (not destroyed) fires `FireInstantHit(this, &shell, &shell)`
/// from the gun position (`GetPlaneGunsPosition(cycle)`) moved by the matrix and this frame's travel; one gun gets
/// `DoPlaneGunFireFX(cycle)`, two guns `DoPlaneGunFireFX(2 * cycle)`, then the position re-read with x negated, its
/// own shot and `DoPlaneGunFireFX(2 * cycle + 1)` (gta-reversed reuses the first position). A remote-controlled
/// vehicle (status 8) shakes pad 0, else a player driver's pad (0 / 1) shakes (240, frequency, 0). Then
/// `ReportWeaponEvent(0x95, minigun -> 31 else the type, this)` and the firing time = now.
#[hook(addr = 0x6D4D30, conv = thiscall, name = "CVehicle::FirePlaneGuns", this = "CVehicle",
       expect = "83 EC 5C 56 8B F1", unverified = "frontier2-b26", route = "L2", hazards = "heap-writes",
       note = "FireInstantHit (bullets, damage), the gun flash fx systems")]
pub fn fire_plane_guns(this: GPtr<u8>) {
    if get_plane_num_guns(this) == 0 {
        return;
    }
    let rate = get_plane_guns_rate_of_fire(this) as u32;
    if gr32(TIME_MS) <= rate.wrapping_add(rd::<u32>(this, 0x4D4)) {
        return;
    }
    let (double, freq, mut ty) = match rd::<i16>(this, 0x22) {
        425 | 548 => (false, 0xA0, 0x26),
        447 | 464 | 487 | 497 => (false, 0x5C, 0x1F),
        476 => {
            let c = rd::<u8>(this, 0x4A9);
            let mut a = (c.wrapping_add(1) ^ c) & 3 ^ c;
            wr::<u8>(this, 0x4A9, a);
            if a & 3 > 2 {
                a &= 0xFC;
                wr::<u8>(this, 0x4A9, a);
            }
            (true, 0xE0, 0x1F)
        }
        520 | 576 => (true, 0x80, 0x26),
        _ => return,
    };
    let mut weapon = [0u32; 7];
    let w = GPtr::from_raw(weapon.as_mut_ptr()).cast::<u8>();
    weapon_ctor(w, ty, 5000);
    let cycle = || i32::from(rd::<u8>(this, 0x4A9) & 3);
    let mut pos = CVector::default();
    get_plane_guns_position_call(this, vp(&mut pos), cycle());
    let mut shell = gun_shell(this, &mut pos);
    fire_instant_hit(w, this, vp(&mut shell), vp(&mut shell), GPtr::NULL, GPtr::NULL, GPtr::NULL, 0, 0);
    if double {
        do_plane_gun_fire_fx_call(this, w, vp(&mut pos), vp(&mut shell), cycle() * 2);
        let mut r = CVector::default();
        let r = read_vec(get_plane_guns_position_call(this, vp(&mut r), cycle()));
        pos = CVector::new(f32_of(x(r.x).fchs()), r.y, r.z);
        shell = gun_shell(this, &mut pos);
        fire_instant_hit(w, this, vp(&mut shell), vp(&mut shell), GPtr::NULL, GPtr::NULL, GPtr::NULL, 0, 0);
        do_plane_gun_fire_fx_call(this, w, vp(&mut pos), vp(&mut shell), cycle() * 2 + 1);
    } else {
        do_plane_gun_fire_fx_call(this, w, vp(&mut pos), vp(&mut shell), cycle());
    }
    let pad = if rd::<u8>(this, 0x36) & 0xF8 == 0x40 { Some(0) } else { player_driver_pad(this) };
    if let Some(n) = pad {
        pad_start_shake(get_pad(n), 0xF0, freq, 0);
    }
    if ty == 0x26 {
        ty = 0x1F;
    }
    report_weapon_event(GPtr::from_addr(AUDIO_ENGINE), 0x95, ty, this.addr() as u32);
    wr::<u32>(this, 0x4D4, gr32(TIME_MS));
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_GET_PLANE_GUNS_POSITION,
    HOOK_DO_PLANE_GUN_FIRE_FX,
    HOOK_FIRE_PLANE_GUNS,
    HOOK_FIRE_UNGUIDED_MISSILE,
    HOOK_FIRE_FIXED_MACHINE_GUNS,
    HOOK_SCAN_AND_MARK_TARGET_FOR_HEAT_SEEKING_MISSILE,
    HOOK_FIRE_HEAT_SEAKING_MISSILE,
];
