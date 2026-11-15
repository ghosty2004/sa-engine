//! `CTaskComplexKillPedOnFootArmed` (FRONTIER-2 part 3, batch `frontier2-6`): the armed attack the police run on a
//! criminal (crossing scenario, 4000 frames). `CreateFirstSubTask` 0x62BF00, `CreateSubTask` 0x626FC0 and
//! `LineOfSightClearForAttack` 0x621500, ported from the 1.0 US bytes (gta-reversed reverses the first and the last
//! with differences noted on each function; it does not reverse `CreateSubTask`).
//!
//! Fields: +8 sub-task, +0xC target, +0x10 shoot timer, +0x14 last attack time, +0x20 duck length, +0x30 competence
//! (signed byte), +0x34 new target, +0x35 aim immediately, +0x38 our position and +0x44 the target's position when
//! the line of sight was last blocked, +0x50 the time it was blocked, +0x54 the time it was last clear, +0x58 the
//! time the ped started to move in closer.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::fpu::{ge_or_nan, gt, le_or_nan, lt, st0};

use crate::bindings::intel_set_task_duck_secondary;
use crate::bindings::ped_groups_get_peds_group as peds_group;
use crate::bindings::stand_still_dtor;
use crate::bindings::stand_still_process_ped;
use crate::bindings::throw_control_ctor;
use sa_core::crt::rand_at;
use crate::common::*;
use sa_core::bindings::general_random_int_in_range as random_in_range;
use sa_core::bindings::get_weapon_info;
use sa_core::bindings::vector_normalise;
use sa_core::bindings::{intel_get_task_use_gun, vector_difference, vector_dot_product, vector_magnitude};
use crate::bindings::{duck_control_duck_move, use_gun_control_gun_move};
use sa_world::bindings::{membership_get_leader, vec_mul_assign, vector_add_assign, vector_magnitude_sqr, vector_mul_scalar, vector_sub};
use crate::bindings::{intel_clear_task_duck_secondary, tm_get_task_secondary};
use sa_math::x87::X87;
use sa_world::bindings::{
    duck_ctor, go_to_point_and_stand_still_ctor, pause_ctor, ped_get_transformed_bone_position, ped_get_weapon_skill,
    ped_say, seek_entity_standard_ctor, stand_still_ctor, task_gun_control_ctor, weapon_is_type_melee,
};
use sa_core::field::gf as fc;
use sa_world::world::common::{entity_pos, entity_pos3 as pos3};
use sa_world::world::garage_update::now;

orig! {
    /// `CPed::ReleaseCoverPoint()` (0x5E0270)
    fn release_cover_point(ped: GPtr<u8>) = thiscall 0x5E0270 => unported;
    /// `CCover::FindAndReserveCoverPoint(CPed*, const CVector&, bool)` (0x6992B0)
    fn find_and_reserve_cover_point(ped: GPtr<u8>, pos: GPtr<u8>, b: u32) -> GPtr<u8> = cdecl 0x6992B0 => unported;
    /// `CCover::FindCoordinatesCoverPoint(CCoverPoint*, CPed*, const CVector&, CVector&)` (0x699570)
    fn find_coordinates_cover_point(cp: GPtr<u8>, ped: GPtr<u8>, pos: GPtr<u8>, out: GPtr<u8>) -> u8 = cdecl 0x699570 => unported;
    /// `CTaskComplexKillPedOnFootArmed::CreateSubTask(eTaskType, CPed*)` (0x626FC0, `ret 8`)
    fn create_sub_task_o(t: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x626FC0 => armed_create_sub_task;
    /// `CTaskComplexKillPedOnFootArmed::LineOfSightClearForAttack(CPed*)` (0x621500, `ret 4`, AL)
    fn los_clear_for_attack_o(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x621500 => armed_line_of_sight_clear_for_attack;
    /// `CTaskComplexKillPedOnFootArmed::IsPedInLeaderFiringLine(CPed*)` (0x621300, `ret 4`, AL)
    fn is_ped_in_leader_firing_line_o(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x621300 => armed_is_ped_in_leader_firing_line;
    /// `CTaskComplex::CreateFirstSubTask(CPed*)` (vtable +0x24)
    fn v_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CTaskSimpleDuck::SetDuckTimer(uint16)` (0x692530, `ret 4`)
    fn duck_set_duck_timer(d: GPtr<u8>, t: u32) = thiscall 0x692530 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CCover::DoesCoverPointStillProvideCover(CCoverPoint*, CVector)` (0x698DD0, the vector by value)
    fn cover_still_provides(cp: GPtr<u8>, x: u32, y: u32, z: u32) -> u8 = cdecl 0x698DD0 => unported;
}



fn target(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0xC)
}


/// The `CWeaponInfo` of the ped's current weapon at its current skill.
fn current_weapon_info(ped: GPtr<u8>) -> GPtr<u8> {
    let slot = isize::from(rd::<i8>(ped, 0x718));
    let wt = rd::<i32>(ped, 0x5A0 + slot * 0x1C);
    get_weapon_info(wt, ped_get_weapon_skill(ped))
}


/// `ped.Say(id, offset, 1.0, 0, 0, 0)`
fn say(ped: GPtr<u8>, id: u32, offset: u32) -> i16 {
    ped_say(ped, id, offset, 1.0, 0, 0, 0)
}

/// `CTaskComplexKillPedOnFootArmed::CreateFirstSubTask(CPed*)` (0x62BF00, `ret 4`): clears the new-target flag, takes
/// the ped's weapon info (before the target test), then with a target: the distance `d` (`sqrt` of the x87 sum
/// `(dx² + dy²) + dz²`, the squared sum stored as `d²`) and whether the target's weapon is not melee. A ped not told to
/// stay in place, `rand() & 1` (0x62BFDC), competence > 0 and `d > 30` (or `d > 6` against a non-melee weapon) looks
/// for cover: `ReleaseCoverPoint`, `FindAndReserveCoverPoint` (stored at +0x788), `FindCoordinatesCoverPoint` (its
/// result ignored), z + 1, and when the cover point sees the ped (buildings, vehicles) a duck secondary task (6000) and
/// a `CTaskSimpleGoToPoint(run, cover, 0.5, true)` is returned (a failed allocation falls through); a blocked line
/// releases the cover point. Otherwise the sub-task: gun control when `d² <= (range² * 0.25)` and the line is clear;
/// seek when the ped may move; else pause when the line is clear or the weapon is thrown, gun control otherwise.
/// gta-reversed compares squared distances against 30² / 6² and reads the cover coordinates only when found; the bytes
/// compare the root and leave them uninitialised (zero here) when not.
#[hook(addr = 0x62BF00, conv = thiscall, name = "CTaskComplexKillPedOnFootArmed::CreateFirstSubTask", expect = "6A FF 64 A1 00 00 00 00",
       unverified = "frontier2-6", route = "L2", hazards = "heap-writes,uninit-locals",
       note = "rand site 0x62BFDC; crossing 4000 (police kill a criminal on foot)")]
pub fn armed_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    wr::<u8>(this, 0x34, 0);
    let wi = current_weapon_info(ped);
    let tg = target(this);
    if tg.is_null() {
        return GPtr::NULL;
    }
    let (o, t) = (pos3(ped), pos3(tg));
    let dz = x(t[2]).fsub_m(o[2]);
    let dy = x(t[1]).fsub_m(o[1]);
    let dx = x(t[0]).fsub_m(o[0]);
    let sum = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz));
    let dsq = f32_of(sum);
    let dist = f32_of(sum.fsqrt());
    let tslot = isize::from(rd::<i8>(tg, 0x718));
    let not_melee = weapon_is_type_melee(tg.byte_add(0x5A0 + tslot * 0x1C)) == 0;
    if rd::<u32>(ped, 0x46C) & 0x200000 == 0 && rand_at(0x62BFDC) & 1 != 0 && rd::<i8>(this, 0x30) > 0 {
        let go = if gt(x(dist), x(fc(0x858CA4))) {
            true
        } else {
            !le_or_nan(x(dist), x(fc(0x858B44))) && not_melee
        };
        if go {
            release_cover_point(ped);
            let cp = find_and_reserve_cover_point(ped, entity_pos(target(this)), 0);
            wr::<GPtr<u8>>(ped, 0x788, cp);
            if !cp.is_null() {
                let mut cover = [0f32; 3];
                let cv: GPtr<u8> = GPtr::from_raw(cover.as_mut_ptr()).cast();
                find_coordinates_cover_point(cp, ped, entity_pos(target(this)), cv);
                wr::<f32>(cv, 8, f32_of(x(rd::<f32>(cv, 8)).fadd_m(fc(0x858624))));
                if world_get_is_line_of_sight_clear(cv, entity_pos(ped), 1, 1, 0, 0, 0, 0, 0) != 0 {
                    intel_set_task_duck_secondary(rd::<GPtr<u8>>(ped, 0x47C), 6000);
                    let t = task_new(0x30);
                    let t = if t.is_null() { GPtr::NULL } else { crate::bindings::go_to_point_ctor(t, 6, cv, 0x3F00_0000, 1, 0) };
                    if !t.is_null() {
                        return t;
                    }
                } else {
                    release_cover_point(ped);
                }
            }
        }
    }
    let r = rd::<f32>(wi, 4);
    let lim = x(r).fmul(x(r)).fmul_m(fc(0x858C84));
    if le_or_nan(x(dsq), lim) && los_clear_for_attack_o(this, ped) != 0 {
        return create_sub_task_o(this, 0x3FC, ped);
    }
    let ty = if rd::<u32>(ped, 0x46C) & 0x600000 == 0 {
        0x38B
    } else if los_clear_for_attack_o(this, ped) != 0 || (rd::<u32>(wi, 0x18) >> 8) & 1 != 0 {
        0xCA
    } else {
        0x3FC
    };
    create_sub_task_o(this, ty, ped)
}

/// A `CTaskComplexSeekEntity<CEntitySeekPosCalculatorStandard>(target, 50000, 1000, radius, [0x859E30], [0x859E34],
/// true, true)` (0x4C bytes).
fn seek(this: GPtr<u8>, radius: f32) -> GPtr<u8> {
    let t = task_new(0x4C);
    if t.is_null() {
        return GPtr::NULL;
    }
    seek_entity_standard_ctor(t, target(this), 50000, 1000, radius, g::<u32>(0x859E30).read(), g::<u32>(0x859E34).read(), 1, 1)
}

/// A `CTaskComplexGoToPointAndStandStill(run, pos, [0x86FC84], [0x86FC88], false, false)` (0x28 bytes).
fn go_to(pos: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0x28);
    if t.is_null() {
        return GPtr::NULL;
    }
    go_to_point_and_stand_still_ctor(t, 6, pos.cast(), fc(0x86FC84), fc(0x86FC88), 0, 0)
}

/// `CTaskComplexKillPedOnFootArmed::CreateSubTask(eTaskType, CPed*)` (0x626FC0, `ret 8`; the tail passes through the
/// island 0x401C26). The ped's weapon info first, then by type:
/// - 0xCA pause: a stack `CTaskSimpleStandStill(0, false, false, 8.0)` runs `ProcessPed` once and is destroyed, then a
///   new `CTaskSimplePause(100)`;
/// - 0xCB stand still: `CTaskSimpleStandStill([0x86DB24], false, false, 8.0)`; 0x19F duck: `CTaskSimpleDuck(0, duck
///   length +0x20 (word), -1)`;
/// - 0x3FC gun control: a thrown weapon (info +0x18 bit 8) gets `CTaskSimpleThrowControl(target, null)`; otherwise
///   `CTaskSimpleGunControl(target, null, null, command, 5, -1)` with command 3 when the line of sight is clear, else 0
///   (the byte written over the ped argument's slot: the upper three bytes are the ped pointer's), its +0x3A = aim
///   immediately (+0x35, then cleared), the shoot timer +0x10 = now + `GetRandomNumberInRange(4000, 8000)` and a group
///   member says 0xCF; both set +0x14 = now and +0x50 = 0;
/// - 0x38B seek: without a move-in-closer time (+0x58), or less than [0x859A94] ms after it (unsigned difference), a
///   seek with radius 6; at most [0x859C38] ms after it a seek with radius `6 - (t - [0x859A94]) * [0x858CDC]`; later
///   the points beside the target (the normalised 2D direction turned by 90°, times [0x858CE8], on either side) are
///   tried in turn with a line of sight from the target (buildings, vehicles, dummies) and the first clear one is the
///   destination of a go-to-point-and-stand-still, else a seek with radius 1. A player target then hears a cop (type
///   6) say 0x2B (wanted level byte +0x18 > 1) or 0xC7 and answers 0x2C / 0xDAC when it did; another ped in a group
///   says 0x2B;
/// - any other type: null.
#[hook(addr = 0x626FC0, conv = thiscall, name = "CTaskComplexKillPedOnFootArmed::CreateSubTask", expect = "6A FF 64 A1 00 00 00 00",
       unverified = "frontier2-6", route = "L2", hazards = "heap-writes")]
pub fn armed_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    let wi = current_weapon_info(ped);
    match ty {
        0x38B => {
            let started = rd::<u32>(this, 0x58);
            let t = 'pick: {
                if started == 0 {
                    break 'pick seek(this, 6.0);
                }
                let d = now().wrapping_sub(started) as i32;
                let mut v = xi(d);
                if d < 0 {
                    v = v.fadd_m(fc(0x858C54));
                }
                let tf = f32_of(v);
                if !ge_or_nan(v, x(fc(0x859A94))) {
                    break 'pick seek(this, 6.0);
                }
                if le_or_nan(x(tf), x(fc(0x859C38))) {
                    let r = f32_of(x(tf).fsub_m(fc(0x859A94)).fmul_m(fc(0x858CDC)).fsubr_m(fc(0x858B44)));
                    break 'pick seek(this, r);
                }
                let (o, t) = (pos3(ped), pos3(target(this)));
                let mut dir = [f32_of(x(t[0]).fsub_m(o[0])), f32_of(x(t[1]).fsub_m(o[1])), 0.0];
                vector_normalise(GPtr::from_raw(&mut dir));
                let k = fc(0x858CE8);
                let ox = f32_of(x(dir[1]).fchs().fmul_m(k));
                let oy = x(dir[0]).fmul_m(k);
                let t = pos3(target(this));
                let mut p1 = [f32_of(x(ox).fadd_m(t[0])), f32_of(oy.fadd_m(t[1])), t[2]];
                let t = pos3(target(this));
                let mut p2 = [f32_of(x(t[0]).fsub_m(ox)), f32_of(x(t[1]).fsub(oy)), t[2]];
                let a: GPtr<u8> = GPtr::from_raw(p1.as_mut_ptr()).cast();
                let b: GPtr<u8> = GPtr::from_raw(p2.as_mut_ptr()).cast();
                if world_get_is_line_of_sight_clear(entity_pos(target(this)), a, 1, 1, 0, 0, 1, 0, 0) != 0 {
                    break 'pick go_to(a);
                }
                if world_get_is_line_of_sight_clear(entity_pos(target(this)), b, 1, 1, 0, 0, 1, 0, 0) != 0 {
                    break 'pick go_to(b);
                }
                seek(this, 1.0)
            };
            let tg = target(this);
            if tg.is_null() || ped_is_player(tg) == 0 {
                return t;
            }
            if rd::<i32>(ped, 0x598) == 6 {
                let id = if rd::<u8>(find_player_wanted(-1), 0x18) > 1 { 0x2B } else { 0xC7 };
                if say(ped, id, 0) >= 0 {
                    say(target(this), 0x2C, 0xDAC);
                }
            } else if peds_group(ped.addr() as u32) != 0 {
                say(ped, 0x2B, 0);
            }
            t
        }
        0xCA => {
            let mut buf = [0u32; 8];
            let local: GPtr<u8> = GPtr::from_raw(buf.as_mut_ptr()).cast();
            stand_still_ctor(local, 0, 0, 0, 8.0);
            stand_still_process_ped(local, ped);
            stand_still_dtor(local);
            let t = task_new(0x18);
            if t.is_null() { GPtr::NULL } else { pause_ctor(t, 100) }
        }
        0xCB => {
            let t = task_new(0x20);
            if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, g::<i32>(0x86DB24).read(), 0, 0, 8.0) }
        }
        0x19F => {
            let t = task_new(0x28);
            if t.is_null() { GPtr::NULL } else { duck_ctor(t, 0, u32::from(rd::<u16>(this, 0x20)), 0xFFFF_FFFF) }
        }
        0x3FC => {
            let t = if (rd::<u32>(wi, 0x18) >> 8) & 1 != 0 {
                let t = task_new(0x1C);
                if t.is_null() { GPtr::NULL } else { throw_control_ctor(t, target(this).addr() as u32, GPtr::NULL) }
            } else {
                let mut cmd = (ped.addr() as u32 & 0xFFFF_FF00) | 3;
                if los_clear_for_attack_o(this, ped) == 0 {
                    cmd &= 0xFFFF_FF00;
                }
                let t = task_new(0x3C);
                let t = if t.is_null() { GPtr::NULL } else { task_gun_control_ctor(t, target(this), GPtr::NULL, 0, cmd, 5, 0xFFFF_FFFF) };
                wr::<u8>(t, 0x3A, u8::from(rd::<u8>(this, 0x35) != 0));
                wr::<u8>(this, 0x35, 0);
                let start = now();
                wr::<u32>(this, 0x10, (random_in_range(4000, 8000) as u32).wrapping_add(start));
                if peds_group(ped.addr() as u32) != 0 {
                    say(ped, 0xCF, 0);
                }
                t
            };
            wr::<u32>(this, 0x14, now());
            wr::<u32>(this, 0x50, 0);
            t
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexKillPedOnFootArmed::LineOfSightClearForAttack(CPed*)` (0x621500, `ret 4`, AL; the prologue passes
/// through the island 0x405932). The ped's weapon info is fetched (unused). Window and squared distance: 5000 ms (and
/// 4.0 instead of 9.0 when [0xB7298C] < 2) for a ped with +0x2F set, or a player target when [0xB72914] != 0 or the
/// ped's +0x484 byte is 2; 10000 ms and 9.0 otherwise. Clear within the window of the last clear time (+0x54) → true.
/// Blocked within the window of the last blocked time (+0x50) with both the target and the ped within the distance
/// of where they were (+0x44 / +0x38; a null target skips its test) → false. Otherwise a line of sight (buildings,
/// vehicles, objects, see-through) from the ped (its head bone 5 from (0.1, 0, 0) when not ducking; its position +
/// 0.25 z when ducking: gta-reversed has these the other way round) to the target's head, with the
/// target's vehicle's collision flag (+0x1C bit 0) cleared meanwhile and restored after; clear → +0x54 = now, +0x50 =
/// 0; blocked → +0x54 = 0, +0x50 = now and both positions saved.
#[hook(addr = 0x621500, conv = thiscall, name = "CTaskComplexKillPedOnFootArmed::LineOfSightClearForAttack", expect = "83 EC 18 53 55 56 57",
       unverified = "frontier2-6", route = "L2", hazards = "heap-writes")]
pub fn armed_line_of_sight_clear_for_attack(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let _ = current_weapon_info(ped);
    let mut window = 10000u32;
    let mut dsq = f32::from_bits(0x4110_0000);
    let special = rd::<u8>(ped, 0x2F) != 0 || {
        let tg = target(this);
        !tg.is_null()
            && ped_is_player(tg) != 0
            && (g::<u32>(0xB72914).read() != 0 || rd::<u8>(ped, 0x484) == 2)
    };
    if special {
        window = 5000;
        if g::<i32>(0xB7298C).read() < 2 {
            dsq = 4.0;
        }
    }
    let t0 = now();
    if t0.wrapping_sub(rd::<u32>(this, 0x54)) < window {
        return 1;
    }
    let near = |p: [f32; 3], off: isize| {
        let dx = x(p[0]).fsub_m(rd::<f32>(this, off));
        let dy = x(p[1]).fsub_m(rd::<f32>(this, off + 4));
        let dz = x(p[2]).fsub_m(rd::<f32>(this, off + 8));
        le_or_nan(dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz)), x(dsq))
    };
    if t0.wrapping_sub(rd::<u32>(this, 0x50)) <= window {
        let tg = target(this);
        if (tg.is_null() || near(pos3(tg), 0x44)) && near(pos3(ped), 0x38) {
            return 0;
        }
    }
    let mut a = [f32::from_bits(0x3DCC_CCCD), 0.0, 0.0];
    let ap: GPtr<u8> = GPtr::from_raw(a.as_mut_ptr()).cast();
    if rd::<u32>(ped, 0x46C) & 0x0400_0000 != 0 {
        let p = pos3(ped);
        wr::<f32>(ap, 0, p[0]);
        wr::<f32>(ap, 4, p[1]);
        wr::<f32>(ap, 8, f32_of(x(p[2]).fadd_m(fc(0x858C84))));
    } else {
        ped_get_transformed_bone_position(ped, ap, 5, 0);
    }
    let mut b = [f32::from_bits(0x3DCC_CCCD), 0.0, 0.0];
    let bp: GPtr<u8> = GPtr::from_raw(b.as_mut_ptr()).cast();
    ped_get_transformed_bone_position(target(this), bp, 5, 0);
    let tg = target(this);
    let mut saved: Option<u8> = None;
    if !tg.is_null() && rd::<u32>(tg, 0x46C) & 0x100 != 0 {
        let veh = rd::<GPtr<u8>>(tg, 0x58C);
        if !veh.is_null() {
            saved = Some(rd::<u8>(veh, 0x1C) & 1);
            wr::<u32>(veh, 0x1C, rd::<u32>(veh, 0x1C) & !1);
        }
    }
    let clear = world_get_is_line_of_sight_clear(ap, bp, 1, 1, 0, 1, 0, 1, 0) != 0;
    let r = if clear {
        wr::<u32>(this, 0x54, now());
        wr::<u32>(this, 0x50, 0);
        1
    } else {
        wr::<u32>(this, 0x54, 0);
        wr::<u32>(this, 0x50, now());
        let t = pos3(target(this));
        let p = pos3(ped);
        for i in 0..3 {
            wr::<f32>(this, 0x44 + 4 * i as isize, t[i]);
            wr::<f32>(this, 0x38 + 4 * i as isize, p[i]);
        }
        0
    };
    if let Some(bit) = saved {
        let veh = rd::<GPtr<u8>>(target(this), 0x58C);
        let v = rd::<u32>(veh, 0x1C);
        wr::<u32>(veh, 0x1C, v ^ ((u32::from(bit) ^ v) & 1));
    }
    r
}

/// `ped.m_pIntelligence`
fn intel(ped: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(ped, 0x47C)
}

/// `CTaskComplexKillPedOnFootArmed::ControlSubTask(CPed*)` (0x62CCE0, `ret 4`). The sub-task is noted and the weapon
/// info fetched first. A new target restarts with `CreateFirstSubTask` (virtual). No target, or one whose health
/// (+0x540) is not above 0, → null. Ducking: with condition bit 4, more than duck length + last duck + 2000 ms after
/// the last duck and the ped not ducking, a target closer than 20 (squared magnitude < 400) gets a duck secondary
/// task when `GetRandomNumberInRange(0, 100)` < the chance, and the duck time is noted either way; otherwise with
/// condition bit 1 and a shot by the player the same draw restarts the current duck's timer or starts one. A
/// running duck gets +0x1B = 1. When the sub-task changed meanwhile, or is neither gun control nor seek, it is
/// returned. Gun control: the 2D distance `d` decides whether the use-gun task (+0x2E) is set to 6 (fire, +0x34 = 0)
/// — out of range, beyond half the range and 2500 ms after the last attack (unless told to stay), beyond 4 and past
/// the shoot timer (unless told to stay), or the line blocked; then a group member says 0xCF and the strafe of the
/// running use-gun task (or the duck) is chosen: none while the target has +0x40 bit 8, the stored direction (+0x31:
/// left, right, back, forward) until +0x2C, forward when closer than 4, else the strafe direction flips (+0x32) with
/// a new 2500 ms period; a duck moves only for sideways strafes or every 3000 ms (+0x18), else (0, 0). Seek: gun
/// control when the ped is in its leader's firing line, or (after closing within half the range and the target
/// moving towards it, or 2000 ms after the last attack, or told to stay) the line is clear, or the target has +0x40
/// bit 8 and the line is clear; otherwise a group member says 0x41.
#[hook(addr = 0x62CCE0, conv = thiscall, name = "CTaskComplexKillPedOnFootArmed::ControlSubTask", expect = "83 EC 30 53 55 56 57",
       unverified = "frontier2-6", route = "L2", hazards = "heap-writes")]
pub fn armed_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let orig = rd::<GPtr<u8>>(this, 8);
    let wi = current_weapon_info(ped);
    if rd::<u8>(this, 0x34) != 0 {
        return v_create_first_sub_task(this, ped);
    }
    let tg = target(this);
    if tg.is_null() || !x(rd::<f32>(tg, 0x540)).fcom(x(fc(0x858B50))).test_parity(0x41) {
        return GPtr::NULL;
    }
    let flags = rd::<u32>(this, 0x1C);
    let len = || u32::from(rd::<u16>(this, 0x20));
    let staying = || rd::<u32>(ped, 0x46C) & 0x200000 != 0;
    if flags & 4 != 0
        && now() > rd::<u32>(this, 0x20).wrapping_add(rd::<u32>(this, 0x28)).wrapping_add(2000)
        && rd::<u32>(ped, 0x46C) & 0x0400_0000 == 0
    {
        let mut tmp = [0f32; 3];
        let d = vector_sub(GPtr::from_raw(tmp.as_mut_ptr()).cast(), entity_pos(tg), entity_pos(ped));
        if !ge_or_nan(vector_magnitude_sqr(d), x(fc(0x85A700)))
            && (random_in_range(0, 100) as u32) < rd::<u32>(this, 0x24)
        {
            intel_set_task_duck_secondary(intel(ped), len());
        }
        wr::<u32>(this, 0x28, now());
    } else if flags & 1 != 0 && rd::<u8>(this, 0x33) != 0 && (random_in_range(0, 100) as u32) < rd::<u32>(this, 0x24) {
        if !intel_get_task_duck(intel(ped), 1).is_null() {
            duck_set_duck_timer(intel_get_task_duck(intel(ped), 1), len());
        } else {
            intel_set_task_duck_secondary(intel(ped), len());
        }
    }
    let duck = intel_get_task_duck(intel(ped), 1);
    if !duck.is_null() {
        wr::<u8>(duck, 0x1B, 1);
    }
    if rd::<GPtr<u8>>(this, 8) != orig {
        return orig;
    }
    match task_get_type(orig) {
        0x3FC => {
            let mut tmp = [0f32; 3];
            let d = vector_sub(GPtr::from_raw(tmp.as_mut_ptr()).cast(), entity_pos(target(this)), entity_pos(ped));
            let dist = st0(vector_magnitude(d.cast()));
            let d2 = f32_of(dist);
            let range = rd::<f32>(wi, 4);
            let half_late = !ge_or_nan(x(range).fmul_m(fc(0x858B8C)), x(d2))
                && !staying()
                && now() > rd::<u32>(this, 0x14).wrapping_add(2500);
            let timer_out = !le_or_nan(x(d2), x(fc(0x858B90))) && !staying() && now() > rd::<u32>(this, 0x10);
            let fire = gt(dist, x(range)) || half_late || timer_out || los_clear_for_attack_o(this, ped) == 0;
            if fire {
                let sub = rd::<GPtr<u8>>(this, 8);
                if rd::<u8>(sub, 0x2E) != 6 {
                    wr::<u8>(sub, 0x2E, 6);
                    wr::<u32>(sub, 0x34, 0);
                }
            }
            let gun = intel_get_task_use_gun(intel(ped));
            if gun.is_null() {
                return orig;
            }
            if peds_group(ped.addr() as u32) != 0 {
                say(ped, 0xCF, 0);
            }
            let mut strafe = [0f32; 2];
            if rd::<u32>(target(this), 0x40) & 0x100 != 0 {
            } else if now() < rd::<u32>(this, 0x2C) && !staying() {
                match rd::<u8>(this, 0x31) {
                    0 => strafe = [-1.0, 0.0],
                    1 => strafe = [1.0, 0.0],
                    2 => strafe = [0.0, -1.0],
                    3 => strafe = [0.0, 1.0],
                    _ => {}
                }
            } else if !ge_or_nan(x(d2), x(fc(0x858B90))) && !staying() {
                strafe = [0.0, 1.0];
            } else if rd::<u8>(this, 0x32) != 0 {
                let dir = rd::<u8>(this, 0x31);
                if dir <= 3 {
                    wr::<u8>(this, 0x31, [1, 0, 3, 2][usize::from(dir)]);
                }
                wr::<u8>(this, 0x32, 0);
                wr::<u32>(this, 0x2C, now().wrapping_add(2500));
            }
            if !duck.is_null() {
                let dir = rd::<u8>(this, 0x31);
                if dir != 0 && dir != 1 {
                    duck_control_duck_move(duck, strafe[0], strafe[1]);
                } else if now() > rd::<u32>(this, 0x18).wrapping_add(3000) {
                    duck_control_duck_move(duck, strafe[0], strafe[1]);
                    wr::<u32>(this, 0x18, now());
                } else {
                    duck_control_duck_move(duck, 0.0, 0.0);
                }
            } else {
                use_gun_control_gun_move(gun, GPtr::from_raw(strafe.as_mut_ptr()).cast());
            }
            orig
        }
        0x38B => {
            let mut closing = false;
            if !staying() {
                let m = rd::<GPtr<u8>>(ped, 0x14);
                let tg = target(this);
                let mut tmp = [0f32; 3];
                let d = vector_sub(GPtr::from_raw(tmp.as_mut_ptr()).cast(), entity_pos(tg), entity_pos(ped));
                let dist = st0(vector_magnitude(d.cast()));
                if gt(x(rd::<f32>(wi, 4)).fmul_m(fc(0x858B8C)), dist) {
                    let dot = vector_dot_product(tg.byte_add(0x44), GPtr::from_addr(m.addr() + 0x10));
                    closing = lt(dot, x(fc(0x858B50))) || now().wrapping_sub(rd::<u32>(this, 0x14)) > 2000;
                }
            }
            if is_ped_in_leader_firing_line_o(this, ped) != 0
                || ((closing || staying()) && los_clear_for_attack_o(this, ped) != 0)
                || (rd::<u32>(target(this), 0x40) & 0x100 != 0 && los_clear_for_attack_o(this, ped) != 0)
            {
                return create_sub_task_o(this, 0x3FC, ped);
            }
            if peds_group(ped.addr() as u32) != 0 {
                say(ped, 0x41, 0);
            }
            orig
        }
        _ => orig,
    }
}

/// `CTaskComplexKillPedOnFootArmed::IsPedInLeaderFiringLine(CPed*)` (0x621300, `ret 4`, AL): false unless the ped's
/// group has a leader who is a player with a target (+0x71C) and a non-melee weapon. Then, with `a` the 2D unit
/// vector from the leader to its target and `b` = ped - leader, the closest point `leader + a (a·b)` (dot as `(ay by +
/// bz az) + ax bx`, rounded to f32) must lie within 2 (squared 2D distance < 4) of the ped, the ped must be in front
/// ((target - leader)·b > 0) and within 10 of the leader (squared distance < 100).
#[hook(addr = 0x621300, conv = thiscall, name = "CTaskComplexKillPedOnFootArmed::IsPedInLeaderFiringLine", expect = "83 EC 24 53 55",
       unverified = "frontier2-6", route = "L2", hazards = "none")]
pub fn armed_is_ped_in_leader_firing_line(_this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let grp = peds_group(ped.addr() as u32);
    if grp == 0 {
        return 0;
    }
    let leader = membership_get_leader(GPtr::from_addr(grp as usize + 8));
    if leader.is_null() || ped_is_player(leader) == 0 || rd::<u32>(leader, 0x71C) == 0 {
        return 0;
    }
    let slot = isize::from(rd::<i8>(leader, 0x718));
    if weapon_is_type_melee(leader.byte_add(0x5A0 + slot * 0x1C)) != 0 {
        return 0;
    }
    let lp = entity_pos(leader);
    let tpos = || entity_pos(rd::<GPtr<u8>>(leader, 0x71C));
    let mut a = [0f32; 3];
    let mut b = [0f32; 3];
    let mut c = [0f32; 3];
    let ap: GPtr<u8> = GPtr::from_raw(a.as_mut_ptr()).cast();
    let bp: GPtr<u8> = GPtr::from_raw(b.as_mut_ptr()).cast();
    let cp: GPtr<u8> = GPtr::from_raw(c.as_mut_ptr()).cast();
    vector_difference(ap, tpos(), lp);
    wr::<f32>(ap, 8, 0.0);
    vector_normalise(ap.cast());
    vector_difference(bp, entity_pos(ped), lp);
    let (ax, ay, az) = (rd::<f32>(ap, 0), rd::<f32>(ap, 4), rd::<f32>(ap, 8));
    let (bx, by, bz) = (rd::<f32>(bp, 0), rd::<f32>(bp, 4), rd::<f32>(bp, 8));
    let dot = f32_of(x(ay).fmul_m(by).fadd(x(bz).fmul_m(az)).fadd(x(ax).fmul_m(bx)));
    for (i, v) in [ax, ay, az].into_iter().enumerate() {
        wr::<f32>(cp, 4 * i as isize, v);
    }
    vec_mul_assign(cp, dot);
    vector_add_assign(cp, lp);
    let pp = entity_pos(ped);
    wr::<f32>(ap, 0, f32_of(x(rd::<f32>(cp, 0)).fsub_m(rd::<f32>(pp, 0))));
    wr::<f32>(ap, 4, f32_of(x(rd::<f32>(cp, 4)).fsub_m(rd::<f32>(pp, 4))));
    wr::<f32>(ap, 8, 0.0);
    if ge_or_nan(vector_magnitude_sqr(ap), x(fc(0x858B90))) {
        return 0;
    }
    vector_difference(cp, tpos(), lp);
    vector_difference(bp, entity_pos(ped), lp);
    let d = x(rd::<f32>(bp, 8))
        .fmul_m(rd::<f32>(cp, 8))
        .fadd(x(rd::<f32>(bp, 4)).fmul_m(rd::<f32>(cp, 4)))
        .fadd(x(rd::<f32>(bp, 0)).fmul_m(rd::<f32>(cp, 0)));
    if le_or_nan(d, x(fc(0x858B50))) {
        return 0;
    }
    vector_difference(bp, entity_pos(ped), lp);
    if ge_or_nan(vector_magnitude_sqr(bp), x(fc(0x858628))) {
        return 0;
    }
    1
}

/// The ped's cover point (`CPed::m_pCoverPoint`, +0x788).
fn cover_point(ped: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(ped, 0x788)
}

/// `CTaskComplexKillPedOnFootArmed::CreateNextSubTask(CPed*)` (0x62C190, `ret 4`; the epilogue passes through the
/// island 0x401EAE; FRONTIER-2 part 4, crossing 4000 and the shoot-a-driver run). Unreversed in gta-reversed; from the
/// bytes. Null without a target. `notMelee` = the target's weapon is not melee, `wi` / `range` = the ped's weapon info
/// and its target range (+4), `dist` = `sqrt((dx² + dy²) + dz²)` (target - ped, rounded to a float). By the finished
/// sub-task's type:
/// - 0x19F duck: gun control (0x3FC) when the line of sight is clear, else pause (0xCA); clears ped +0x46C bit 26;
/// - 0xCA / 0xCB: a ped that may not move (+0x46C & 0x600000) takes gun control when `range² > |target - ped|²` and the
///   line is clear, else pause; any other says 0x90 and seeks (0x38B), starting the move-in-closer time (+0x58) if unset;
/// - 0x384: gun control when the line is clear, else pause; 0x387 / 0x38B: gun control when the line is clear, else
///   pause through the seek tail (starts +0x58);
/// - 0x516 and any other type: null; every path above clears +0x58 except the ones through the seek tail;
/// - 0x385, 0x3FC, 0x3FD (the attack, [`attack_next`] then [`attack_tail`]).
#[hook(addr = 0x62C190, conv = thiscall, name = "CTaskComplexKillPedOnFootArmed::CreateNextSubTask", expect = "6A FF 64 A1 00 00 00 00",
       unverified = "frontier2-8", route = "L2", hazards = "heap-writes",
       note = "rand sites 0x62C6F6, 0x62C777, 0x62C876, 0x62C9FF, 0x62CAE4, 0x62CAF5, 0x62CC19, 0x62CC22, 0x62CC71; crossing 4000")]
pub fn armed_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let tg = target(this);
    if tg.is_null() {
        return GPtr::NULL;
    }
    let tslot = isize::from(rd::<i8>(tg, 0x718));
    let not_melee = weapon_is_type_melee(tg.byte_add(0x5A0 + tslot * 0x1C)) == 0;
    let wi = current_weapon_info(ped);
    let range = rd::<f32>(wi, 4);
    let (o, t) = (pos3(ped), pos3(tg));
    let dz = x(t[2]).fsub_m(o[2]);
    let dy = x(t[1]).fsub_m(o[1]);
    let dx = x(t[0]).fsub_m(o[0]);
    let dist = f32_of(dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz)).fsqrt());
    let clear_58 = |r: GPtr<u8>| {
        wr::<u32>(this, 0x58, 0);
        r
    };
    match v_task_type(rd::<GPtr<u8>>(this, 8)) {
        0x19F => {
            let ty = if los_clear_for_attack_o(this, ped) != 0 { 0x3FC } else { 0xCA };
            let r = create_sub_task_o(this, ty, ped);
            wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFBFF_FFFF);
            clear_58(r)
        }
        0xCA | 0xCB => {
            if rd::<u32>(ped, 0x46C) & 0x600000 != 0 {
                let wi2 = current_weapon_info(ped);
                let mut d = [0f32; 3];
                let dp: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
                vector_sub(dp, entity_pos(target(this)), entity_pos(ped));
                let r2 = rd::<f32>(wi2, 4);
                let m = vector_magnitude_sqr(dp);
                let ty = if gt(x(r2).fmul_m(r2), m) && los_clear_for_attack_o(this, ped) != 0 { 0x3FC } else { 0xCA };
                clear_58(create_sub_task_o(this, ty, ped))
            } else {
                say(ped, 0x90, 0);
                seek_tail(this, create_sub_task_o(this, 0x38B, ped))
            }
        }
        0x384 => {
            let ty = if los_clear_for_attack_o(this, ped) != 0 { 0x3FC } else { 0xCA };
            clear_58(create_sub_task_o(this, ty, ped))
        }
        0x387 | 0x38B => {
            if los_clear_for_attack_o(this, ped) != 0 {
                clear_58(create_sub_task_o(this, 0x3FC, ped))
            } else {
                seek_tail(this, create_sub_task_o(this, 0xCA, ped))
            }
        }
        0x385 | 0x3FC | 0x3FD => {
            let r = attack_next(this, ped, wi, range, dist, not_melee);
            attack_tail(this, ped, r)
        }
        _ => clear_58(GPtr::NULL),
    }
}

/// 0x62C310: the result, starting the move-in-closer time (+0x58) when it is not set.
fn seek_tail(this: GPtr<u8>, r: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(this, 0x58) == 0 {
        wr::<u32>(this, 0x58, now());
    }
    r
}

/// The strafe timer: +0x2C = now + 2000, +0x32 = 0.
fn strafe_timer(this: GPtr<u8>) {
    wr::<u32>(this, 0x2C, now().wrapping_add(2000));
    wr::<u8>(this, 0x32, 0);
}

/// 0x62CC55: pause (0xCA), +0x14 = now.
fn pause_now(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let r = create_sub_task_o(this, 0xCA, ped);
    wr::<u32>(this, 0x14, now());
    r
}

/// 0x62CC42: gun control, pause when it fails.
fn gun_or_pause(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let r = create_sub_task_o(this, 0x3FC, ped);
    if r.is_null() { pause_now(this, ped) } else { r }
}

/// 2D length `sqrt(x² + y²)` of a vector (x87, not rounded).
fn len2d(v: GPtr<u8>) -> X87 {
    let (vy, vx) = (x(rd::<f32>(v, 4)), x(rd::<f32>(v, 0)));
    vx.fmul(vx).fadd(vy.fmul(vy)).fsqrt()
}

/// The attack case of [`armed_create_next_sub_task`] (0x62C411), up to the tail: pause when the ped stays in place
/// (+0x46C bit 21); gun control (pause when it fails) closer than 3; beyond `min(range * 0.8, 23)` a ped that may move
/// seeks with radius `min(range * 0.6, 20)` (pause otherwise or when it fails). Else:
/// 1. a finished gun control with its +0x39 flag (and a ped that may move) says 0x90 and seeks with radius 2; otherwise,
///    with a non-melee target and a cover point that still covers and whose coordinates lie within 0.75 (2D) of the ped:
///    a crouching point (+1 == 0) closer than 12, the ped not ducking, `rand() & 3 == 0` and no secondary task says 0x55
///    and ducks (2500 ms); a standing point, on `rand() & 1 == 0` and a weapon that is not thrown, gets a gun control
///    (command 3, burst 5) aiming at once (+0x35, cleared) with the strafe timers (+0x10 now + 5000, +0x14 now, +0x2C
///    now + 2500, +0x31 1 (0 for type 1), +0x32 1) and clears the duck secondary task; failures release the cover point;
/// 2. competence > 0, dist > 6, a non-melee target and `rand() & 1`: a new cover point (`FindAndReserveCoverPoint(ped,
///    target, competence == 2)`, its coordinates' z + 1) within `range * 0.75` (2D) of the target and in sight of the
///    ped: a group member says 0x41, a duck secondary task (6000) and a run to it; else it is released;
/// 3. beyond 10, `rand() & 3`: 0 says 0x90 and seeks with radius `dist - 4`, 1 starts a strafe (+0x31 = 2) — both only
///    for a ped that may move;
/// 4. beyond 5 and `rand() & 3 == 0`: a strafe side (+0x31 = `rand() & 1`) checked with a line of sight from the ped to
///    2.5 m along its right axis (side 1 to the right, else to the left; a blocked side switches), the strafe timer;
/// 5. without a result: gun control when the line is clear (pause when it fails); else on `rand() & 3 == 0` a random
///    strafe side (`rand() < 0x3FFF`) and the timer; then pause, with +0x14 = now.
fn attack_next(this: GPtr<u8>, ped: GPtr<u8>, wi: GPtr<u8>, range: f32, dist: f32, not_melee: bool) -> GPtr<u8> {
    let flags = rd::<u32>(ped, 0x46C);
    if flags & 0x200000 != 0 {
        return pause_now(this, ped);
    }
    if lt(x(dist), x(fc(0x858B3C))) {
        return gun_or_pause(this, ped);
    }
    let mut lim = x(range).fmul_m(fc(0x858C98));
    if !lt(lim, x(fc(0x858F84))) {
        lim = x(fc(0x858F84));
    }
    if gt(x(dist), lim) {
        if flags & 0x400000 != 0 {
            return pause_now(this, ped);
        }
        let l2 = x(range).fmul_m(fc(0x858CC8));
        let radius = if lt(l2, x(fc(0x858BA4))) { f32_of(l2) } else { 20.0 };
        let r = seek(this, radius);
        return if r.is_null() { pause_now(this, ped) } else { r };
    }
    // 1. the current cover point
    'cover: {
        let sub = rd::<GPtr<u8>>(this, 8);
        if v_task_type(sub) == 0x3FC && rd::<u8>(rd::<GPtr<u8>>(this, 8), 0x39) != 0 && rd::<u32>(ped, 0x46C) & 0x600000 == 0 {
            say(ped, 0x90, 0);
            let r = seek(this, 2.0);
            if !r.is_null() {
                return r;
            }
            break 'cover;
        }
        let cp = cover_point(ped);
        if !not_melee || cp.is_null() {
            break 'cover;
        }
        let tp = entity_pos(target(this));
        if cover_still_provides(cp, rd::<u32>(tp, 0), rd::<u32>(tp, 4), rd::<u32>(tp, 8)) == 0 {
            break 'cover;
        }
        let mut cov = [0f32; 3];
        let cv: GPtr<u8> = GPtr::from_raw(cov.as_mut_ptr()).cast();
        if find_coordinates_cover_point(cover_point(ped), ped, entity_pos(target(this)), cv) == 0 {
            break 'cover;
        }
        let mut d = [0f32; 3];
        let dp: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
        let near = lt(len2d(vector_sub(dp, entity_pos(ped), cv)), x(fc(0x858F34)));
        let task = 'pick: {
            if !near {
                break 'pick GPtr::NULL;
            }
            if rd::<u8>(cover_point(ped), 1) == 0 {
                if !lt(x(dist), x(fc(0x858CCC))) || rd::<u8>(ped, 0x46F) & 1 != 0 || rand_at(0x62C6F6) & 3 != 0 {
                    break 'pick GPtr::NULL;
                }
                if !tm_get_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 1).is_null() {
                    break 'pick GPtr::NULL;
                }
                say(ped, 0x55, 0);
                let t = task_new(0x28);
                if t.is_null() { GPtr::NULL } else { duck_ctor(t, 0, 0x9C4, 0xFFFF_FFFF) }
            } else {
                if rand_at(0x62C777) & 1 != 0 || (rd::<u32>(wi, 0x18) >> 8) & 1 != 0 {
                    break 'pick GPtr::NULL;
                }
                let t = task_new(0x3C);
                let t = if t.is_null() { GPtr::NULL } else { task_gun_control_ctor(t, target(this), GPtr::NULL, 0, 3, 5, 0xFFFF_FFFF) };
                wr::<u8>(t, 0x3A, u8::from(rd::<u8>(this, 0x35) != 0));
                wr::<u8>(this, 0x35, 0);
                wr::<u32>(this, 0x10, now().wrapping_add(5000));
                wr::<u32>(this, 0x14, now());
                wr::<u32>(this, 0x2C, now().wrapping_add(2500));
                wr::<u8>(this, 0x31, 1);
                wr::<u8>(this, 0x32, 1);
                if rd::<u8>(cover_point(ped), 1) == 1 {
                    wr::<u8>(this, 0x31, 0);
                }
                intel_clear_task_duck_secondary(rd::<GPtr<u8>>(ped, 0x47C));
                t
            }
        };
        if !task.is_null() {
            return task;
        }
        release_cover_point(ped);
    }
    // 2. a new cover point (0x62C849)
    if rd::<i8>(this, 0x30) > 0 && gt(x(dist), x(fc(0x858B44))) && not_melee && rand_at(0x62C876) & 1 != 0 {
        release_cover_point(ped);
        let cp = find_and_reserve_cover_point(ped, entity_pos(target(this)), u32::from(rd::<u8>(this, 0x30) == 2));
        wr::<GPtr<u8>>(ped, 0x788, cp);
        if !cp.is_null() {
            let mut cov = [0f32; 3];
            let cv: GPtr<u8> = GPtr::from_raw(cov.as_mut_ptr()).cast();
            find_coordinates_cover_point(cp, ped, entity_pos(target(this)), cv);
            wr::<f32>(cv, 8, f32_of(x(rd::<f32>(cv, 8)).fadd_m(fc(0x858624))));
            let mut d = [0f32; 3];
            let dp: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
            let len = len2d(vector_sub(dp, cv, entity_pos(target(this))));
            if gt(x(range).fmul_m(fc(0x858F34)), len) && world_get_is_line_of_sight_clear(cv, entity_pos(ped), 1, 1, 0, 0, 0, 0, 0) != 0 {
                if peds_group(ped.addr() as u32) != 0 {
                    say(ped, 0x41, 0);
                }
                intel_set_task_duck_secondary(rd::<GPtr<u8>>(ped, 0x47C), 6000);
                let t = task_new(0x30);
                let t = if t.is_null() { GPtr::NULL } else { crate::bindings::go_to_point_ctor(t, 6, cv, 0x3F00_0000, 1, 0) };
                if !t.is_null() {
                    return t;
                }
            } else {
                release_cover_point(ped);
            }
        }
    }
    // 3. (0x62C9E5)
    let mut res = GPtr::NULL;
    if gt(x(dist), x(fc(0x85862C))) {
        match rand_at(0x62C9FF) & 3 {
            0 if rd::<u32>(ped, 0x46C) & 0x400000 == 0 => {
                let radius = f32_of(x(dist).fsub_m(fc(0x858B90)));
                say(ped, 0x90, 0);
                res = seek(this, radius);
            }
            1 if rd::<u32>(ped, 0x46C) & 0x400000 == 0 => {
                wr::<u8>(this, 0x31, 2);
                strafe_timer(this);
            }
            _ => {}
        }
    }
    // 4. (0x62CACA)
    if gt(x(dist), x(fc(0x858C80))) && rand_at(0x62CAE4) & 3 == 0 {
        wr::<u8>(this, 0x31, 0);
        if rand_at(0x62CAF5) & 1 != 0 {
            wr::<u8>(this, 0x31, 1);
        }
        let mut p = [0f32; 3];
        let pp: GPtr<u8> = GPtr::from_raw(p.as_mut_ptr()).cast();
        let src = entity_pos(ped);
        for i in 0..3 {
            wr::<u32>(pp, 4 * i, rd::<u32>(src, 4 * i));
        }
        let mut side = [0f32; 3];
        let sp: GPtr<u8> = GPtr::from_raw(side.as_mut_ptr()).cast();
        vector_mul_scalar(sp, rd::<GPtr<u8>>(ped, 0x14), 2.5);
        let right = rd::<u8>(this, 0x31) == 1;
        for i in 0..3 {
            let (a, b) = (rd::<f32>(sp, 4 * i), rd::<f32>(pp, 4 * i));
            wr::<f32>(sp, 4 * i, f32_of(if right { x(a).fadd_m(b) } else { x(b).fsub_m(a) }));
        }
        if world_get_is_line_of_sight_clear(pp, sp, 1, 1, 0, 1, 0, 0, 0) == 0 {
            wr::<u8>(this, 0x31, u8::from(!right));
        }
        strafe_timer(this);
    }
    // 5. (0x62CC04)
    if !res.is_null() {
        return res;
    }
    if los_clear_for_attack_o(this, ped) != 0 {
        return gun_or_pause(this, ped);
    }
    if rand_at(0x62CC19) & 3 == 0 {
        wr::<u8>(this, 0x31, u8::from(rand_at(0x62CC22) < 0x3FFF));
        strafe_timer(this);
    }
    pause_now(this, ped)
}

/// 0x62CC6C: the attack case's tail. On `rand() & 1`, competence > 0, a ped not ducking (+0x46F bit 0) and no cover
/// point of type 1 or 2: a duck secondary task (6000). +0x58 = 0.
fn attack_tail(this: GPtr<u8>, ped: GPtr<u8>, r: GPtr<u8>) -> GPtr<u8> {
    if rand_at(0x62CC71) & 1 != 0 && rd::<i8>(this, 0x30) > 0 && rd::<u8>(ped, 0x46F) & 1 == 0 {
        let cp = cover_point(ped);
        if cp.is_null() || !matches!(rd::<u8>(cp, 1), 1 | 2) {
            intel_set_task_duck_secondary(rd::<GPtr<u8>>(ped, 0x47C), 6000);
        }
    }
    wr::<u32>(this, 0x58, 0);
    r
}

pub const HOOKS: &[HookReg] = &[
    HOOK_ARMED_CREATE_NEXT_SUB_TASK,
    HOOK_ARMED_CONTROL_SUB_TASK,
    HOOK_ARMED_IS_PED_IN_LEADER_FIRING_LINE,
    HOOK_ARMED_CREATE_FIRST_SUB_TASK,
    HOOK_ARMED_CREATE_SUB_TASK,
    HOOK_ARMED_LINE_OF_SIGHT_CLEAR_FOR_ATTACK,
];

