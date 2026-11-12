//! `CEventKnockOffBike::CalcForcesAndAnims` / `SetPedSafePosition` and `CEventDamage::ComputeDeathAnim`, executed by
//! the original under SA:MP 0.3.7 (`catalog/samp-priority.csv`) (PORT-AI-8). Ported from the 1.0 US bytes (the
//! exe wins over gta-reversed). Every call to another catalogued function goes through `orig!` at the original
//! address.
//!
//! `CEventKnockOffBike`: +0xC move speed, +0x18 / +0x1C / +0x24 collision impulse x / y / damage scale, +0x30
//! damage intensity, +0x34 direction (i8), +0x35 flags, +0x36 knock-off type (i8), +0x37 exit door (i8), +0x38
//! vehicle. `CEventDamage`: +0x14 source, +0x1C weapon, +0x20 piece, +0x24 direction (u8), +0x25 flags, +0x28 anim
//! group, +0x2C anim id, +0x30 blend delta, +0x34 anim speed, +0x41 jumped-out flag.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// 0.0f
const K_ZERO: usize = 0x858B50;
/// pi (f32)
const K_PI: usize = 0x858CB8;

orig! {
    /// `CGeneral::GetRandomNumberInRange(float min, float max)` (0x41BD90, cdecl, ST0)
    fn general_random_float_in_range(lo: u32, hi: u32) -> f64 = cdecl 0x41BD90 => unported;
    /// `CPhysical::ApplyMoveForce(float x, float y, float z)` (0x4ABBA0, `ret 0xC`)
    fn physical_apply_move_force(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x4ABBA0 => unported;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0, `ret 0xC`; the vector by value)
    fn physical_apply_move_force_v(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x5429F0 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CTaskManager::GetActiveTask` (0x681720)
    fn tm_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTaskManager::GetSimplestActiveTask` (0x6819D0)
    fn tm_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18, `ret 0xC`)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CPedIntelligence::GetTaskFighting` (0x600F30)
    fn intel_get_task_fighting(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F30 => unported;
    /// `CPed::GetBonePosition(RwV3d&, uint32 bone, bool)` (0x5E4280, `ret 0xC`)
    fn ped_get_bone_position(p: GPtr<u8>, out: GPtr<u8>, bone: u32, b: u32) = thiscall 0x5E4280 => unported;
    /// `RpAnimBlendClumpGetFirstAssociation(RpClump*, uint32 flags)` (0x4D6A70, cdecl)
    fn clump_get_first_association(clump: u32, flags: u32) -> GPtr<u8> = cdecl 0x4D6A70 => unported;
    /// `CTaskSimpleFight::IsComboSet` (0x4ABDC0)
    fn fight_is_combo_set(t: GPtr<u8>) -> u8 = thiscall 0x4ABDC0 => unported;
    /// `CTaskSimpleFight::GetComboAnimGroupID` (0x4ABDA0)
    fn fight_get_combo_anim_group_id(t: GPtr<u8>) -> u32 = thiscall 0x4ABDA0 => unported;
    /// `CStats::GetFatAndMuscleModifier(eStatModAbilities)` (0x559AF0, cdecl, ST0)
    fn stats_get_fat_and_muscle_modifier(i: u32) -> f64 = cdecl 0x559AF0 => unported;
    /// `CVector2D::Normalise` (0x44E480)
    fn vector2d_normalise(v: GPtr<u8>) = thiscall 0x44E480 => unported;
    /// `CPed::SetPedPositionInCar` (0x5DF910)
    fn ped_set_ped_position_in_car(p: GPtr<u8>) = thiscall 0x5DF910 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, cdecl, ST0)
    fn general_limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CMatrix::SetRotateZOnly(float)` (0x59B020, `ret 4`)
    fn matrix_set_rotate_z_only(m: GPtr<u8>, a: u32) = thiscall 0x59B020 => unported;
    /// `CPhysical::TestCollision(bool applySpeed)` (vtable +0x34, `ret 4`)
    fn v_test_collision(p: GPtr<u8>, b: u32) -> u8 = thiscall vslot 0x34 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool buildings, bool vehicles, bool peds,
    /// bool objects, bool dummies, bool seeThrough, bool ignoreSomeObjects)` (0x56A490, cdecl)
    #[allow(clippy::too_many_arguments)]
    fn world_los_clear(a: GPtr<u8>, b: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32, st: u32, ig: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0, `ret 4`)
    fn placeable_set_posn(p: GPtr<u8>, v: GPtr<u8>) = thiscall 0x4241C0 => unported;
    /// `CPed::PositionPedOutOfCollision(int32 door, CVehicle*, bool)` (0x5E0820, `ret 0xC`)
    fn ped_position_ped_out_of_collision(p: GPtr<u8>, door: i32, veh: GPtr<u8>, b: u32) -> u8 = thiscall 0x5E0820 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
}

/// A dword-aligned stack object of `N` dwords, as a game pointer.
fn local<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// The f32 constant at `a`.
fn k(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// An entity's position: its matrix +0x30, or the placement at +4 without a matrix.
fn pos_of(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// Copies the vector at `src` to `dst` (three dwords).
fn copy3(dst: GPtr<u8>, src: GPtr<u8>) {
    for i in 0..3 {
        wr::<u32>(dst, 4 * i, rd::<u32>(src, 4 * i));
    }
}

/// `|v|^2` of the vector at `v` in the original's order (`x x + y y + z z`).
fn mag_sq(v: GPtr<u8>) -> X87 {
    let (vx, vy, vz) = (x(rd::<f32>(v, 0)), x(rd::<f32>(v, 4)), x(rd::<f32>(v, 8)));
    vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz))
}

/// The knock-off force `up * su + vec * sv` of the vehicle matrix `m` (`vec` at +`voff`), summed as the
/// original does (x both in registers, y with `vec.y` through an f32 slot, z with both through f32 slots), applied
/// with `CPhysical::ApplyMoveForce(CVector)`.
fn apply_bike_force(ped: GPtr<u8>, m: GPtr<u8>, su: f32, voff: isize, sv: f32) {
    let ux = x(rd::<f32>(m, 0x20)).fmul_m(su);
    let uy = x(rd::<f32>(m, 0x24)).fmul_m(su);
    let uz = f32_of(x(rd::<f32>(m, 0x28)).fmul_m(su));
    let vx = x(rd::<f32>(m, voff)).fmul_m(sv);
    let vy = f32_of(x(rd::<f32>(m, voff + 4)).fmul_m(sv));
    let vz = f32_of(x(rd::<f32>(m, voff + 8)).fmul_m(sv));
    let fx = f32_of(vx.fadd(ux)).to_bits();
    let fy = f32_of(x(vy).fadd(uy)).to_bits();
    let fz = f32_of(x(vz).fadd_m(uz)).to_bits();
    physical_apply_move_force_v(ped, fx, fy, fz);
}

/// The ped's move speed (+0x44) as the vehicle's times `s`, each through an f32 slot.
fn scaled_move_speed(ped: GPtr<u8>, vms: GPtr<u8>, s: f32) {
    for i in 0..3 {
        wr::<f32>(ped, 0x44 + 4 * i, f32_of(x(rd::<f32>(vms, 4 * i)).fmul_m(s)));
    }
}

/// `CEventKnockOffBike::CalcForcesAndAnims(CPed*)` (0x4B0020, `ret 4`): returns the fall anim id. The mass ratio
/// is ped mass / vehicle mass. Types 0 / 0x37: the ped takes the event's move speed, damage entity = vehicle, anim
/// 0xBF. 0x36: the event's move speed, a force of `-0.3 * impulse * ratio * scale` with a random 3.0..5.0 up, damage
/// entity, anim by direction (0x1A / 0x17 / 0x6A / 0x16, else 0xF); a vehicle without the bike / car "keep" byte
/// (+0x804 bikes, +0x960 cars) also sets flag 0x80000000 of +0x470. 0x31: the same force at -0.3 (-0.2 for vehicle
/// class 10) without up, plus a random 3.0..5.0 (3.0..6.0) / 70 move speed z, damage entity, anim 0x1A / 0x17 / 0x18
/// / 0x16, flag 0x80000000 (and an unused `IsPlayer`). 0x35: 0.2 of the vehicle's move speed in x/y and z = 0,
/// damage entity, anim 0x78. Other types by direction: front (0) a force of 5 up - 6 forward while the vehicle moves
/// slower than 0.3 (anim 0x6A, damage entity); sides (1, 2) 0.3 of the vehicle's speed with 5 up + 6 right above
/// 0.3 (anim 0x19), else its speed with 4 up + 8 right (0x16); back (3) the same with -6 / -8 right (0x1B / 0x17);
/// other directions anim 0xF without changes.
#[hook(addr = 0x4B0020, conv = thiscall, name = "CEventKnockOffBike::CalcForcesAndAnims", expect = "83 EC 28 53 55 56 57",
       unverified = "ai8-3", route = "L2", hazards = "heap-writes", recs = "driving,car-ai")]
pub fn knock_off_bike_calc_forces_and_anims(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    let veh = rd::<GPtr<u8>>(this, 0x38);
    let ratio = f32_of(x(rd::<f32>(ped, 0x8C)).fdiv_m(rd::<f32>(veh, 0x8C)));
    let keep = match rd::<u32>(veh, 0x590) {
        9 => rd::<u8>(veh, 0x804),
        0 => rd::<u8>(veh, 0x960),
        _ => 0,
    };
    let ty = i32::from(rd::<i8>(this, 0x36)) as u32;
    let dir = i32::from(rd::<i8>(this, 0x34)) as u32;
    let impulse = |kk: X87| {
        let y = f32_of(kk.fmul_m(rd::<f32>(this, 0x1C)).fmul_m(ratio).fmul_m(rd::<f32>(this, 0x24)));
        let xx = f32_of(kk.fmul_m(rd::<f32>(this, 0x18)).fmul_m(ratio).fmul_m(rd::<f32>(this, 0x24)));
        (xx.to_bits(), y.to_bits())
    };
    match ty {
        0 | 0x37 => {
            copy3(ped.byte_add(0x44), this.byte_add(0xC));
            wr::<GPtr<u8>>(ped, 0x128, rd::<GPtr<u8>>(this, 0x38));
            0xBF
        }
        0x36 => {
            copy3(ped.byte_add(0x44), this.byte_add(0xC));
            let z = f32_of(X87::from_f64_exact(general_random_float_in_range(g::<u32>(0x85AB14).read(), g::<u32>(0x85AB18).read())));
            let (fx, fy) = impulse(x(k(0x85AB10)));
            physical_apply_move_force(ped, fx, fy, z.to_bits());
            wr::<GPtr<u8>>(ped, 0x128, rd::<GPtr<u8>>(this, 0x38));
            let anim = match dir {
                0 => 0x1A,
                1 => 0x17,
                2 => 0x6A,
                3 => 0x16,
                _ => 0xF,
            };
            if keep == 0 {
                wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) | 0x8000_0000);
            }
            anim
        }
        0x31 => {
            copy3(ped.byte_add(0x44), this.byte_add(0xC));
            let class10 = rd::<u32>(rd::<GPtr<u8>>(this, 0x38), 0x594) == 10;
            let kk = if class10 { x(k(0x85AB28)) } else { x(k(0x85AB1C)) };
            let (fx, fy) = impulse(kk);
            physical_apply_move_force(ped, fx, fy, 0);
            let class10 = rd::<u32>(rd::<GPtr<u8>>(this, 0x38), 0x594) == 10;
            let lo = g::<u32>(if class10 { 0x85AB2C } else { 0x85AB20 }).read();
            let hi = g::<u32>(if class10 { 0x85AB30 } else { 0x85AB24 }).read();
            let r = X87::from_f64_exact(general_random_float_in_range(lo, hi));
            wr::<f32>(ped, 0x4C, f32_of(r.fmul_m(k(0x85B328)).fadd_m(rd::<f32>(ped, 0x4C))));
            wr::<GPtr<u8>>(ped, 0x128, rd::<GPtr<u8>>(this, 0x38));
            let anim = match dir {
                0 => 0x1A,
                1 => 0x17,
                2 => 0x18,
                3 => 0x16,
                _ => 0xF,
            };
            wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) | 0x8000_0000);
            ped_is_player(ped);
            anim
        }
        0x35 => {
            let vms = veh.byte_add(0x44);
            let v = [
                f32_of(x(rd::<f32>(vms, 0)).fmul_m(k(0x858CC4))),
                f32_of(x(rd::<f32>(vms, 4)).fmul_m(k(0x858CC4))),
                f32_of(x(rd::<f32>(vms, 8)).fmul_m(k(0x858CC4))),
            ];
            for (i, f) in v.into_iter().enumerate() {
                wr::<f32>(ped, 0x44 + 4 * i as isize, f);
            }
            wr::<u32>(ped, 0x4C, 0);
            wr::<GPtr<u8>>(ped, 0x128, rd::<GPtr<u8>>(this, 0x38));
            0x78
        }
        _ => {
            let vms = veh.byte_add(0x44);
            let m = rd::<GPtr<u8>>(veh, 0x14);
            let fast = || !mag_sq(vms).fcom(x(k(0x85B324))).test_any(0x41);
            match dir {
                0 => {
                    if !mag_sq(vms).fcom(x(k(0x85B324))).test_parity(0x05) {
                        apply_bike_force(ped, m, k(0x858C80), 0x10, k(0x85A5C0));
                    }
                    wr::<GPtr<u8>>(ped, 0x128, rd::<GPtr<u8>>(this, 0x38));
                    0x6A
                }
                1 | 2 => {
                    if fast() {
                        scaled_move_speed(ped, vms, k(0x858C24));
                        apply_bike_force(ped, rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x38), 0x14), k(0x858C80), 0, k(0x858B44));
                        0x19
                    } else {
                        copy3(ped.byte_add(0x44), vms);
                        apply_bike_force(ped, rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x38), 0x14), k(0x858B90), 0, k(0x859000));
                        0x16
                    }
                }
                3 => {
                    if fast() {
                        scaled_move_speed(ped, vms, k(0x858C24));
                        apply_bike_force(ped, rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x38), 0x14), k(0x858C80), 0, k(0x85A5C0));
                        0x1B
                    } else {
                        copy3(ped.byte_add(0x44), vms);
                        apply_bike_force(ped, rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x38), 0x14), k(0x858B90), 0, k(0x85B320));
                        0x17
                    }
                }
                _ => 0xF,
            }
        }
    }
}

/// `CEventDamage::ComputeDeathAnim(CPed*, bool makeActiveTaskAbortable)` (0x4B3A60, `ret 8`). Defaults: group 0,
/// anim 0xF, delta 4.0, speed 1.0. A ped in a vehicle, or (with the flag) whose active task refuses an urgent
/// abort, gets anim 0xBF. Drowning (weapon 0x35) or a ped in water (+0x40 bit 8) that is not standing gets 0x8C.
/// A ped whose head (bone 5) is below its position - 0.2 while in a getting-up / falling task (0xCF / 0xCD) sets
/// flag bit 1 of +0x25 and gets 0x27 when an anim with flag 0x800 plays, else 0x24. A front hit by a fighter whose
/// combo is set (stage >= 4, hit <= 2) uses the combo's group and anim 0xDB + hit at delta 16.0 with force 1.0.
/// Otherwise by weapon class (table 0x4B3F3C): fall anims 0x18..0x1B by direction (or 0x13 / 0xF when jumped out
/// of a car), head/torso variants 0x16..0x1A by body piece, 0x11, 0x8C, 0xF, with a force of 0.5 / 1.5 / 1.0 / 2.0
/// (a player source scales it by its fat-and-muscle modifier, or 5.0 with player data flag 2 of +0x34). With the
/// cheat byte 0x969173 set, fists, knuckles and the camera (0, 1, 0x2E) multiply it by 20. A positive force
/// pushes the ped away from the source in the ground plane by `-5.0 * force` with 5.0 up and clears its standing
/// flags (+0x46C bits 0..1).
#[hook(addr = 0x4B3A60, conv = thiscall, name = "CEventDamage::ComputeDeathAnim", expect = "83 EC 24 53 55 8B 6C 24 30",
       unverified = "ai8-3", route = "L2", hazards = "heap-writes", recs = "weapon-combat,police-gang")]
pub fn damage_compute_death_anim(this: GPtr<u8>, ped: GPtr<u8>, make_abortable: u8) {
    wr::<u32>(this, 0x28, 0);
    wr::<u32>(this, 0x2C, 0xF);
    wr::<u32>(this, 0x30, 0x4080_0000);
    wr::<u32>(this, 0x34, 0x3F80_0000);
    let mut force = 0.0f32;
    let tm = || rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
    let refuse = rd::<u32>(ped, 0x46C) & 0x100 != 0
        || (make_abortable != 0
            && !tm_get_active_task(tm()).is_null()
            && v_make_abortable(tm_get_active_task(tm()), ped, 1, this) == 0);
    if refuse {
        wr::<u32>(this, 0x2C, 0xBF);
        return;
    }
    let weapon = rd::<u32>(this, 0x1C);
    if weapon == 0x35 || (rd::<u32>(ped, 0x40) & 0x100 != 0 && rd::<u8>(ped, 0x46C) & 1 == 0) {
        wr::<u32>(this, 0x2C, 0x8C);
        return;
    }
    let src = rd::<GPtr<u8>>(this, 0x14);
    let is_ped = |e: GPtr<u8>| !e.is_null() && rd::<u8>(e, 0x36) & 7 == 3;
    let fight = if is_ped(src) { intel_get_task_fighting(rd::<GPtr<u8>>(src, 0x47C)) } else { GPtr::NULL };
    let mut head = [0u32; 3];
    ped_get_bone_position(ped, local(&mut head), 5, 0);
    let low = !x(rd::<f32>(pos_of(ped), 8)).fsub_m(k(0x858CC4)).fcom(x(f32::from_bits(head[2]))).test_any(0x41);
    if low && !tm_get_simplest_active_task(tm()).is_null() {
        let t1 = v_task_type(tm_get_simplest_active_task(tm()));
        if t1 == 0xCF || v_task_type(tm_get_simplest_active_task(tm())) == 0xCD {
            wr::<u8>(this, 0x25, rd::<u8>(this, 0x25) | 2);
            let a = clump_get_first_association(rd::<u32>(ped, 0x18), 0x800);
            wr::<u32>(this, 0x2C, if a.is_null() { 0x24 } else { 0x27 });
            return;
        }
    }
    let dir = rd::<u8>(this, 0x24);
    let combo = dir == 0
        && !fight.is_null()
        && fight_is_combo_set(fight) != 0
        && rd::<i8>(fight, 0x24) >= 4
        && rd::<i8>(fight, 0x25) <= 2;
    if combo {
        wr::<u32>(this, 0x28, fight_get_combo_anim_group_id(fight));
        wr::<u32>(this, 0x2C, (i32::from(rd::<i8>(fight, 0x25)) + 0xDB) as u32);
        wr::<u32>(this, 0x30, 0x4180_0000);
        force = 1.0;
    } else if weapon <= 0x36 {
        let class = g::<u8>(0x4B3F3C + weapon as usize).read();
        let by_dir = |a: [u32; 4]| if dir <= 3 { Some(a[dir as usize]) } else { None };
        match class {
            0..=2 => {
                force = match class {
                    0 => 0.5,
                    1 => 1.5,
                    _ => 0.0,
                };
                if rd::<u8>(this, 0x41) != 0 {
                    wr::<u32>(this, 0x2C, 0x13);
                    force = 0.0;
                } else if let Some(a) = by_dir([0x18, 0x19, 0x1A, 0x1B]) {
                    wr::<u32>(this, 0x2C, a);
                }
                let s = rd::<GPtr<u8>>(this, 0x14);
                if is_ped(s) {
                    let pd = rd::<GPtr<u8>>(s, 0x480);
                    if !pd.is_null() {
                        force = if rd::<u8>(pd, 0x34) & 2 != 0 {
                            f32_of(x(force).fmul_m(k(0x858C80)))
                        } else {
                            f32_of(X87::from_f64_exact(stats_get_fat_and_muscle_modifier(4)).fmul_m(force))
                        };
                    }
                }
            }
            3 => {
                if let Some(a) = by_dir([0x18, 0x19, 0x1A, 0x1B]) {
                    wr::<u32>(this, 0x2C, a);
                }
            }
            4 => wr::<u32>(this, 0x2C, 0xF),
            5..=8 => {
                force = match class {
                    6 => 1.0,
                    7 => 2.0,
                    8 => 0.5,
                    _ => 0.0,
                };
                if rd::<u8>(this, 0x41) != 0 {
                    wr::<u32>(this, 0x2C, 0xF);
                    force = 0.0;
                } else if x(force).fcom(x(k(K_ZERO))).test_any(0x41) {
                    wr::<u32>(this, 0x2C, 0xF);
                } else if let Some(a) = by_dir([0x18, 0x19, 0x1A, 0x1B]) {
                    wr::<u32>(this, 0x2C, a);
                }
            }
            9 => {
                let piece = rd::<u32>(this, 0x20);
                let a = match dir {
                    0 => Some(if piece == 5 { 0x16 } else if piece == 6 { 0x17 } else { 0x1A }),
                    1 => Some(0x19),
                    2 => Some(if piece == 5 { 0x16 } else if piece == 6 { 0x17 } else { 0x18 }),
                    3 => Some(0x1B),
                    _ => None,
                };
                if let Some(a) = a {
                    wr::<u32>(this, 0x2C, a);
                }
            }
            10 => wr::<u32>(this, 0x2C, 0x8C),
            11 => wr::<u32>(this, 0x2C, 0x11),
            _ => {}
        }
    }
    if g::<u8>(0x969173).read() != 0 && matches!(rd::<u32>(this, 0x1C), 0 | 1 | 0x2E) {
        force = f32_of(x(force).fmul_m(k(0x858BA4)));
    }
    if x(force).fcom(x(k(K_ZERO))).test_any(0x41) {
        return;
    }
    let src = rd::<GPtr<u8>>(this, 0x14);
    if src.is_null() {
        return;
    }
    let sp = pos_of(src);
    let pp = pos_of(ped);
    let mut d = [
        f32_of(x(rd::<f32>(sp, 0)).fsub_m(rd::<f32>(pp, 0))).to_bits(),
        f32_of(x(rd::<f32>(sp, 4)).fsub_m(rd::<f32>(pp, 4))).to_bits(),
    ];
    vector2d_normalise(local(&mut d));
    let f = x(force).fmul_m(k(0x858FC8));
    wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFFFF_FFFC);
    let fx = f32_of(x(f32::from_bits(d[0])).fmul(f)).to_bits();
    let fy = f32_of(f.fmul_m(f32::from_bits(d[1]))).to_bits();
    physical_apply_move_force_v(ped, fx, fy, 0x40A0_0000);
}

/// `CEventKnockOffBike::SetPedSafePosition(CPed*)` (0x4B4AC0, `ret 4`; AL). A bike's rider fields (+0x648,
/// +0x5C8) are cleared and the ped is placed in its seat. The ped takes the vehicle's heading (`atan2(-fwd.x,
/// fwd.y)` or the placement heading; turned by pi through `LimitRadianAngle` when the vehicle is upside down) as
/// its current and aimed rotation. Off a bike (unless flag bit 0 of +0x35) it moves back along its forward axis by
/// `(1 - |bike fwd . ped fwd|) * 0.8`. With the vehicle ignored by the world (0xB7CD68) and as the damage entity,
/// the position is kept if clear (`TestCollision(false)` and a line of sight from the vehicle; type 0x31 first
/// tries 0.5 higher), else 0.6 higher, else 0.6 lower than the saved position, else
/// `PositionPedOutOfCollision(door +0x37, vehicle, bodyless || intensity)`; a failure returns its AL (0) as is.
/// Success sets flag 0x80000000 of +0x470, drops the attach reference +0x568, clears +0x46C bits 0..1 and returns
/// 1. The ignored entity is reset to null.
#[hook(addr = 0x4B4AC0, conv = thiscall, name = "CEventKnockOffBike::SetPedSafePosition", expect = "83 EC 1C 53 56 8B 74 24 28",
       unverified = "ai8-3", route = "L2", hazards = "ax-al-return,heap-writes,x87-transcendental,uninit-locals",
       recs = "driving,car-ai", note = "the original pushes the whole dword of a stack byte as the bool of PositionPedOutOfCollision")]
pub fn knock_off_bike_set_ped_safe_position(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let veh = || rd::<GPtr<u8>>(this, 0x38);
    if rd::<u32>(veh(), 0x590) == 9 {
        wr::<u32>(veh(), 0x648, 0);
        wr::<u8>(veh(), 0x5C8, 0);
        ped_set_ped_position_in_car(ped);
    }
    let v = veh();
    let m = rd::<GPtr<u8>>(v, 0x14);
    let heading = |m: GPtr<u8>| {
        if m.is_null() { x(rd::<f32>(v, 0x10)) } else { x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14))) }
    };
    let h = if !x(rd::<f32>(m, 0x28)).fcom(x(k(K_ZERO))).test_parity(0x05) {
        let a = heading(m).fadd_m(k(K_PI));
        X87::from_f64_exact(general_limit_radian_angle(f32_of(a).to_bits()))
    } else {
        heading(m)
    };
    wr::<f32>(ped, 0x55C, f32_of(h));
    let r = f32_of(x(rd::<f32>(ped, 0x55C)));
    wr::<f32>(ped, 0x558, r);
    let pm = rd::<GPtr<u8>>(ped, 0x14);
    if pm.is_null() {
        wr::<f32>(ped, 0x10, r);
    } else {
        matrix_set_rotate_z_only(pm, r.to_bits());
    }
    if rd::<u32>(veh(), 0x590) == 9 && rd::<u8>(this, 0x35) & 1 == 0 {
        let pm = rd::<GPtr<u8>>(ped, 0x14);
        let vf = rd::<GPtr<u8>>(veh(), 0x14).byte_add(0x10);
        let d = x(rd::<f32>(vf, 8))
            .fmul_m(rd::<f32>(pm, 0x18))
            .fadd(x(rd::<f32>(vf, 4)).fmul_m(rd::<f32>(pm, 0x14)))
            .fadd(x(rd::<f32>(vf, 0)).fmul_m(rd::<f32>(pm, 0x10)));
        let s = d.fabs().fsubr_m(k(0x858624)).fmul_m(k(0x858C98));
        let off = [
            f32_of(s.fmul_m(rd::<f32>(pm, 0x10))),
            f32_of(s.fmul_m(rd::<f32>(pm, 0x14))),
            f32_of(s.fmul_m(rd::<f32>(pm, 0x18))),
        ];
        let p = if pm.is_null() { ped.byte_add(4) } else { pm.byte_add(0x30) };
        let nx = f32_of(x(rd::<f32>(p, 0)).fsub_m(off[0]));
        let ny = f32_of(x(rd::<f32>(p, 4)).fsub_m(off[1]));
        let nz = f32_of(x(rd::<f32>(p, 8)).fsub_m(off[2]));
        wr::<f32>(p, 0, nx);
        wr::<f32>(p, 4, ny);
        wr::<f32>(p, 8, nz);
    }
    let mut result = 1u8;
    let flag = u32::from(
        !x(rd::<f32>(ped, 0x540)).fcom(x(k(K_ZERO))).test_parity(0x41) || rd::<u32>(this, 0x30) != 0,
    );
    g::<GPtr<u8>>(0xB7CD68).write(veh());
    wr::<GPtr<u8>>(ped, 0x128, veh());
    let p = pos_of(ped);
    let mut saved = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
    let clear = || {
        let v = veh();
        world_los_clear(pos_of(v), pos_of(ped), 1, 0, 0, 1, 0, 0, 0) != 0
    };
    let raise = |d: f32| {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        wr::<f32>(m, 0x38, f32_of(x(rd::<f32>(m, 0x38)).fadd_m(d)));
    };
    let mut ok = v_test_collision(ped, 0) == 0 && clear();
    if ok {
        if rd::<i8>(this, 0x36) == 0x31 {
            raise(k(0x858B8C));
            if v_test_collision(ped, 0) != 0 || !clear() {
                placeable_set_posn(ped, local(&mut saved));
            }
        }
    } else {
        raise(k(0x858CC8));
        ok = v_test_collision(ped, 0) == 0 && clear();
        if !ok {
            let pm = rd::<GPtr<u8>>(ped, 0x14);
            let p = if pm.is_null() { ped.byte_add(4) } else { pm.byte_add(0x30) };
            for (i, s) in saved.into_iter().enumerate() {
                wr::<u32>(p, 4 * i as isize, s);
            }
            let m = rd::<GPtr<u8>>(ped, 0x14);
            wr::<f32>(m, 0x38, f32_of(x(rd::<f32>(m, 0x38)).fsub_m(k(0x858CC8))));
            ok = v_test_collision(ped, 0) == 0 && clear();
            if !ok {
                result = ped_position_ped_out_of_collision(ped, i32::from(rd::<i8>(this, 0x37)), veh(), flag);
                if result == 0 {
                    g::<u32>(0xB7CD68).write(0);
                    return result;
                }
            }
        }
        wr::<GPtr<u8>>(ped, 0x128, veh());
    }
    wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) | 0x8000_0000);
    let slot = ped.byte_add(0x568);
    let e = rd::<GPtr<u8>>(slot, 0);
    if !e.is_null() {
        entity_clean_up_old_reference(e, slot);
        wr::<u32>(slot, 0, 0);
    }
    wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFFFF_FFFC);
    g::<u32>(0xB7CD68).write(0);
    result
}

pub const HOOKS: &[HookReg] = &[
    HOOK_KNOCK_OFF_BIKE_CALC_FORCES_AND_ANIMS,
    HOOK_DAMAGE_COMPUTE_DEATH_ANIM,
    HOOK_KNOCK_OFF_BIKE_SET_PED_SAFE_POSITION,
];
