//! FRONTIER-2 turn 3, lane A (batch `frontier2-a5`): `CTaskSimpleGangDriveBy::ProcessPed` (0x62D3B0, the police /
//! gang drive-by of a wanted-level chase) and its helpers (the player's and the AI's anim choice 0x621960 /
//! 0x627600, the line-of-sight test 0x621B10, the shot 0x627CC0 and the gun pointing 0x628350), ported from the
//! 1.0 US bytes (gta-reversed reverses none of them). FPATAN / FSIN / FCOS go through `sa_math::x87`'s models,
//! `_CIasin` through `orig::x87_unary`.
//!
//! Task layout (0x44): +8 finished, +9 anims referenced, +0xA seat RHS, +0xB in range to shoot, +0xC in weapon range,
//! +0xD reached abort range, +0xF next command, +0x10 last command, +0x11 burst shots, +0x12 drive-by style, +0x13
//! frequency percentage, +0x14 fake shoot direction, +0x16 attack timer (i16), +0x18 LOS check time, +0x1C LOS
//! blocked, +0x20 abort range, +0x24 required anim id, +0x28 anim group, +0x2C anim, +0x30 weapon info, +0x34 target
//! entity, +0x38 target coords.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::field::vec3f_ptr;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::weapon_fire;
use crate::common::*;
use sa_anim::bindings::pedik_point_gun_in_direction;
use sa_core::bindings::{
    general_random_int_in_range, get_anim_hierarchy_from_skin_clump, get_weapon_info, matrix_mul_vec, ped_get_bone_position,
    rp_hanim_hierarchy_get_matrix_array, rp_hanim_id_get_index, rw_v3d_transform_points, vector_normalise,
};
use sa_math::fpu::{fbits, fcom_jnp, ge_or_nan, gt, le_or_nan, lt};
use sa_world::bindings::{
    cam_get_two_player_aim_vector, camera_find_3rd_person_cam_target_vector, camera_set_new_player_weapon_mode,
    get_pad_from_player, get_player_info, is_coop_game_going_on, pad_get_weapon, ped_do_gun_flash,
    ped_get_weapon_skill, vector_add, vector_scale, weapon_update,
};
use sa_world::collision::sphere_cast::ensure_matrix;
use sa_world::world::common::{entity_pos, ped_active_weapon as current_weapon, placeable_heading};

use crate::bindings::drive_by_start_anim;
use sa_world::bindings::vehicle_can_ped_lean_out;
use crate::bindings::gang_drive_by_fire as drive_by_fire;
use crate::bindings::vehicle_set_window_open_flag;
use sa_world::bindings::add_weapon_model as ped_add_weapon_model;
orig! {
    /// the player's anim choice (0x621960, `ret 4`)
    fn drive_by_player_choose(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x621960 => gang_drive_by_player_choose_anim;
    /// the AI's anim choice (0x627600, `ret 4`)
    fn drive_by_ai_choose(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x627600 => gang_drive_by_ai_choose_anim;
    /// the line-of-sight test (0x621B10, `ret 8`)
    fn drive_by_los_clear(t: GPtr<u8>, ped: GPtr<u8>, pos: GPtr<u8>) -> u32 = thiscall 0x621B10 => gang_drive_by_los_clear;
    /// the gun pointing (0x628350, `ret 4`)
    fn drive_by_point_gun(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x628350 => gang_drive_by_point_gun;
}

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7CB5C;
/// `TheCamera` (0xB6F028), its `ActiveCam` byte (+0x59) and `Cams[]` (+0x174, stride 0x238)
const THE_CAMERA: usize = 0xB6F028;
const ACTIVE_CAM: usize = 0xB6F081;
const CAMS: usize = 0xB6F19C;
/// `CWorld::pIgnoreEntity`
const IGNORE_ENTITY: usize = 0xB7CD68;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;

const PI: f32 = fbits(0x40490FDB);
const TWO_PI: f32 = fbits(0x40C90FDB);
const MINUS_PI: f32 = fbits(0xC0490FDB);
const PI_4: f32 = fbits(0x3F490FDB);
const MINUS_PI_4: f32 = fbits(0xBF490FDB);
const PI_2: f32 = fbits(0x3FC90FDB);
const TWO_OVER_PI: f32 = fbits(0x3F22F983);
const PI_3_4: f32 = fbits(0x4016CBE4);
const MINUS_PI_3_4: f32 = fbits(0xC016CBE4);
/// the anim times 0.2667 (0x8D2E7C), 0.4333 (0x8D2E80), 0.3 (0x8D2E84)
const T_RESTART: f32 = fbits(0x3E888CE7);
const T_FIRE: f32 = fbits(0x3EDDD97F);
const T_AIM: f32 = fbits(0x3E99999A);

/// The active camera (`TheCamera.Cams[ActiveCam]`).
fn active_cam() -> GPtr<u8> {
    GPtr::from_addr(CAMS + usize::from(g::<u8>(ACTIVE_CAM).read()) * 0x238)
}


/// `CTimer::GetTimeStepInMS()` as the bytes inline it: `ftol(ms_fTimeStep * 0.02 * 1000)`.
fn time_step_ms() -> i32 {
    ftol32(x(g::<f32>(TIME_STEP).read()).fmul_m(fbits(0x3CA3D70A)).fmul_m(1000.0))
}

/// `fld a; fcomp b; test ah, 0x41; jp skip`: taken (true) when `a <= b` (ordered).
fn le_ordered(a: X87, b: X87) -> bool {
    fcom_jnp(a, b, 0x41)
}

/// `fcomp 0; test ah, 0x44; jp`: false when the f32 is exactly ±0.
fn is_zero(v: f32) -> bool {
    !x(v).fcom(X87::zero()).test_parity(0x44)
}

/// The first drive-by anim (+0x24) of a quadrant (0 front, 1 left, 2 back, 3 right), by the seat side (+0xA).
fn quadrant_anim(this: GPtr<u8>, q: i8) -> Option<u32> {
    let rhs = rd::<u8>(this, 0xA) != 0;
    match q {
        0 => Some(if rhs { 0xED } else { 0xE9 }),
        1 => Some(if rhs { 0xEC } else { 0xE7 }),
        2 => Some(if rhs { 0xEE } else { 0xEA }),
        3 => Some(if rhs { 0xEB } else { 0xE8 }),
        _ => None,
    }
}

/// The 20 m (+ the camera's distance in the two-player mode 0x31) range of the player's aim; `None` outside the
/// aim-from-car modes 0x31 / 0x37.
fn player_aim_range(ped: GPtr<u8>) -> Option<f32> {
    let cam = active_cam();
    let mode = rd::<u16>(cam, 0xC);
    if mode == 0x31 {
        let p = entity_pos(ped);
        let a = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(cam, 0x19C));
        let b = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(cam, 0x1A0));
        let c = x(rd::<f32>(p, 8)).fsub_m(rd::<f32>(cam, 0x1A4));
        Some(c.fmul(c).fadd(b.fmul(b)).fadd(a.fmul(a)).fsqrt().fadd_m(20.0).to_f32())
    } else if mode == 0x37 {
        Some(20.0)
    } else {
        None
    }
}

/// `target.matrix * coords` (+0x38) with the original's inline `GetMatrix` (allocated and updated when missing).
fn target_world_coords(this: GPtr<u8>, target: GPtr<u8>, out: &mut [f32; 3]) {
    let m = ensure_matrix(target);
    matrix_mul_vec(vec3f_ptr(out), m, this.byte_add(0x38));
}

/// `CTaskSimpleGangDriveBy::ProcessPed(CPed*)` (0x62D3B0, `ret 4`, AL). Done (true) when the current weapon's info
/// is not instant-hit, the ped cannot lean out of its vehicle or the task finished. The LOS check time (+0x18) starts
/// at now + rand(0, 1000), the weapon info (+0x30) is taken once. Out of a vehicle, or below 1.0 health: the task
/// aborts itself (`MakeAbortable(ped, 1, null)`), true. A non-player gets ped flag 0x100000 (+0x478). Without an anim
/// group: 0x4B for vehicle sub-type 2, 0x4A for a bike's driver with +0x640 == 2 (else 0x49), 0x48 otherwise, with the
/// passenger seats' window opened (doors 8 / 0xB / 9) and the seat side; a non-player without a weapon model gets the
/// current weapon's model. Then the player's / the AI's anim choice, the bike flag +0x7B4, the anim start, the shot
/// when the anim crosses 0.3, the command state machine (+0xF / +0x10 / +0x11 / +0x16, the anim crossing 0.2667 and
/// 0.4333), the gun pointing, the attack timer, the player's pad, the finished flag and the player's weapon update;
/// last `Say(0x1F)` above a squared horizontal speed of 0.5 or `Say(0x1E)` below 0.01; false.
#[hook(addr = 0x62D3B0, conv = thiscall, name = "CTaskSimpleGangDriveBy::ProcessPed", expect = "55 8B 6C 24 08 0F BE 85",
       unverified = "frontier2-a5", route = "L2", hazards = "heap-writes,ax-al-return", recs = "police-gang",
       note = "calls the x87-transcendental helpers 0x621960 / 0x627600 / 0x628350")]
pub fn gang_drive_by_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let wt = rd::<i32>(current_weapon(ped), 0);
    let wi = get_weapon_info(wt, ped_get_weapon_skill(ped));
    if !wi.is_null() && rd::<u32>(wi, 0) != 1 {
        return 1;
    }
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    if !veh.is_null() && vehicle_can_ped_lean_out(veh, ped) == 0 {
        return 1;
    }
    if rd::<u8>(this, 8) != 0 {
        return 1;
    }
    if rd::<u32>(this, 0x18) == 0 {
        let now = g::<u32>(TIME_MS).read();
        let r = general_random_int_in_range(0, 1000);
        wr::<u32>(this, 0x18, (r as u32).wrapping_add(now));
    }
    if rd::<u32>(this, 0x30) == 0 {
        let wt = rd::<i32>(current_weapon(ped), 0);
        wr::<GPtr<u8>>(this, 0x30, get_weapon_info(wt, ped_get_weapon_skill(ped)));
    }
    if rd::<u32>(ped, 0x46C) & 0x100 == 0
        || rd::<u32>(ped, 0x58C) == 0
        || lt(x(rd::<f32>(ped, 0x540)), x(1.0))
    {
        task_make_abortable(this, ped, 1, GPtr::NULL);
        return 1;
    }
    if ped_is_player(ped) == 0 {
        wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) | 0x100000);
    }
    if rd::<u32>(this, 0x28) == 0 {
        let veh = rd::<GPtr<u8>>(ped, 0x58C);
        let sub = rd::<u32>(veh, 0x594);
        if sub == 2 {
            wr::<u32>(this, 0x28, 0x4B);
            wr::<u8>(this, 0xA, 0);
        } else if rd::<u32>(veh, 0x590) == 9 {
            let grp = if rd::<u32>(veh, 0x640) == 2 && rd::<GPtr<u8>>(veh, 0x460) == ped { 0x4A } else { 0x49 };
            wr::<u32>(this, 0x28, grp);
            wr::<u8>(this, 0xA, 0);
        } else {
            if sub == 0 {
                let door = if rd::<GPtr<u8>>(veh, 0x464) == ped {
                    wr::<u8>(this, 0xA, 1);
                    Some(8)
                } else if rd::<GPtr<u8>>(veh, 0x468) == ped {
                    wr::<u8>(this, 0xA, 0);
                    Some(0xB)
                } else if rd::<GPtr<u8>>(veh, 0x46C) == ped {
                    wr::<u8>(this, 0xA, 1);
                    Some(9)
                } else {
                    None
                };
                if let Some(d) = door {
                    vehicle_set_window_open_flag(rd::<GPtr<u8>>(ped, 0x58C), d);
                }
            }
            wr::<u32>(this, 0x28, 0x48);
        }
        if ped_is_player(ped) == 0 && rd::<u32>(ped, 0x4F4) == 0 {
            let wi = get_weapon_info(rd::<i32>(current_weapon(ped), 0), 1);
            ped_add_weapon_model(ped, rd::<i32>(wi, 0xC));
        }
    }
    if ped_is_player(ped) != 0 {
        drive_by_player_choose(this, ped);
    } else {
        drive_by_ai_choose(this, ped);
    }
    if rd::<u32>(this, 0x28) == 0x4A {
        let veh = rd::<GPtr<u8>>(ped, 0x58C);
        if rd::<u32>(veh, 0x590) == 9 {
            wr::<u8>(veh, 0x7B4, 1);
        }
    }
    if drive_by_start_anim(this, ped) != 0 {
        let a = rd::<GPtr<u8>>(this, 0x2C);
        if rd::<u8>(a, 0x2E) & 1 != 0
            && gt(x(rd::<f32>(a, 0x20)), x(T_AIM))
            && le_ordered(x(rd::<f32>(a, 0x20)).fsub_m(rd::<f32>(a, 0x28)), x(T_AIM))
            && matches!(rd::<i8>(this, 0x10), 2 | 3)
        {
            drive_by_fire(this, ped);
        }
        let a = rd::<GPtr<u8>>(this, 0x2C);
        let held = rd::<u8>(a, 0x2E) & 1;
        let mut handled = false;
        if held == 0 && rd::<i16>(this, 0x16) <= 0 && matches!(rd::<i8>(this, 0xF), 2 | 3) {
            let len = x(rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10));
            if gt(len, x(rd::<f32>(a, 0x20))) || !x(rd::<f32>(a, 0x1C)).fcom(x(0.0)).test_any(1) {
                wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
                let wi = rd::<GPtr<u8>>(this, 0x30);
                let n = ftol32(xi(i32::from(rd::<i16>(wi, 0x20))).fmul_m(0.5));
                wr::<u8>(this, 0x11, general_random_int_in_range(1, n) as u8);
                wr::<u8>(this, 0x10, rd::<u8>(this, 0xF));
                wr::<u8>(this, 0xF, 0);
                handled = true;
            }
        }
        if !handled {
            if held == 0 && rd::<i8>(this, 0xF) == 7 {
                wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
                wr::<u8>(this, 0x10, rd::<u8>(this, 0xF));
                wr::<u8>(this, 0xF, 7);
            } else if held == 0 && i32::from(rd::<i16>(a, 0x2C)) != rd::<i32>(this, 0x24) {
                wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
            } else if rd::<i8>(this, 0x10) == 1
                && held != 0
                && i32::from(rd::<i16>(a, 0x2C)) == rd::<i32>(this, 0x24)
                && gt(x(rd::<f32>(a, 0x20)), x(T_RESTART))
                && le_ordered(x(rd::<f32>(a, 0x20)).fsub_m(rd::<f32>(a, 0x28)), x(T_RESTART))
            {
                wr::<u16>(a, 0x2E, rd::<u16>(a, 0x2E) & 0xFFFE);
                assoc_set_current_time(rd::<GPtr<u8>>(this, 0x2C), T_RESTART);
            }
        }
        let a = rd::<GPtr<u8>>(this, 0x2C);
        if i32::from(rd::<i16>(a, 0x2C)) == rd::<i32>(this, 0x24)
            && gt(x(rd::<f32>(a, 0x20)), x(T_FIRE))
            && le_ordered(x(rd::<f32>(a, 0x20)).fsub_m(rd::<f32>(a, 0x28)), x(T_FIRE))
        {
            if rd::<u32>(current_weapon(ped), 4) == 2 && rd::<i8>(this, 0xF) <= 2 {
                wr::<u8>(this, 0xF, 2);
                wr::<u16>(this, 0x16, 2000);
            } else {
                let next = rd::<i8>(this, 0xF);
                let shoot = matches!(next, 2 | 3) || (rd::<i8>(this, 0x10) == 3 && rd::<i8>(this, 0x11) > 0);
                if shoot && rd::<u8>(this, 0xB) != 0 {
                    assoc_set_current_time(a, T_RESTART);
                    let a2 = rd::<GPtr<u8>>(this, 0x2C);
                    wr::<u8>(a2, 0x2E, rd::<u8>(a2, 0x2E) | 1);
                    let next = rd::<i8>(this, 0xF);
                    if next > rd::<i8>(this, 0x10) {
                        wr::<i8>(this, 0x10, next);
                    }
                    wr::<u8>(this, 0xF, 0);
                    if rd::<i8>(this, 0x10) == 3 {
                        wr::<u8>(this, 0x11, rd::<u8>(this, 0x11).wrapping_sub(1));
                    } else {
                        wr::<u8>(this, 0x11, 0);
                    }
                } else if next == 1 {
                    assoc_set_current_time(a, T_RESTART);
                    let a2 = rd::<GPtr<u8>>(this, 0x2C);
                    wr::<u16>(a2, 0x2E, rd::<u16>(a2, 0x2E) & 0xFFFE);
                    wr::<u8>(this, 0x10, 1);
                    wr::<u8>(this, 0xF, 0);
                    wr::<u16>(this, 0x16, 0xFFFF);
                }
            }
        }
    }
    let last = rd::<i8>(this, 0x10);
    if last != 0 && last < 4 {
        let a = rd::<GPtr<u8>>(this, 0x2C);
        if !a.is_null()
            && (gt(x(rd::<f32>(a, 0x18)), x(0.5))
                || (gt(x(rd::<f32>(a, 0x1C)), x(0.0)) && rd::<u8>(a, 0x2E) & 1 == 0))
        {
            drive_by_point_gun(this, ped);
        }
    }
    let timer = rd::<i16>(this, 0x16);
    if timer < 0 {
        let hi = (100 - i32::from(rd::<i8>(this, 0x13))).wrapping_mul(100);
        wr::<u16>(this, 0x16, general_random_int_in_range(200, hi) as u16);
    } else {
        wr::<u16>(this, 0x16, (timer as u16).wrapping_sub(time_step_ms() as u16));
    }
    if ped_is_player(ped) != 0 && rd::<i8>(this, 0xF) < 4 {
        wr::<u16>(this, 0x16, 0);
        let pad = get_pad_from_player(ped);
        wr::<u8>(this, 0xF, u8::from(pad_get_weapon(pad, ped) != 0) + 1);
    } else if rd::<i8>(this, 0xF) == 0 {
        match rd::<i8>(this, 0x10) {
            1 => {
                if rd::<u8>(this, 0x13) == 0 && rd::<u8>(this, 0xC) != 0 {
                    wr::<u8>(this, 0xF, 1);
                } else if rd::<u8>(this, 0xB) != 0 {
                    wr::<u8>(this, 0xF, 3);
                } else if rd::<i16>(this, 0x16) <= 0 {
                    wr::<u16>(this, 0x16, (time_step_ms() << 1) as u16);
                }
            }
            6 | 7 => wr::<u8>(this, 0xF, 7),
            _ => {
                if rd::<u8>(this, 0xC) != 0 {
                    wr::<u8>(this, 0xF, 1);
                }
            }
        }
    }
    let last = rd::<i8>(this, 0x10);
    if last != 4 && rd::<u32>(this, 0x2C) == 0 && (last == 7 || rd::<i8>(this, 0xF) == 7) {
        wr::<u8>(this, 8, 1);
    }
    if ped_is_player(ped) != 0 {
        weapon_update(current_weapon(ped), ped);
    }
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    let vx = x(rd::<f32>(veh, 0x44));
    let vy = x(rd::<f32>(veh, 0x48));
    let sq = vx.fmul(vx).fadd(vy.fmul(vy));
    if gt(sq, x(0.5)) {
        ped_say(ped, 0x1F, 0, 1.0, 0, 0, 0);
    } else if lt(sq, x(fbits(0x3C23D70A))) {
        ped_say(ped, 0x1E, 0, 1.0, 0, 0, 0);
    }
    0
}

/// The player's drive-by anim choice (0x621960, `ret 4`). Outside the camera modes 0x37 (aim from car) / 0x31 (two
/// players): +0xB = 0, +0xC = 1. Else +0xB = +0xC = 1 and the anim (+0x24) is the quadrant of `atan2(-front.x,
/// front.y) - vehicle heading` (the camera's front +0x190, or `Get_TwoPlayer_AimVector` in mode 0x31; wrapped into
/// (-pi, pi], plus pi/4, made non-negative, times 2/pi, `_ftol`). Then the player weapon camera mode 0x37 unless a
/// co-op game runs, the player data's flag 8 (+0x34) and the player info byte +0x153 = 0.
#[hook(addr = 0x621960, conv = thiscall, name = "FUN_00621960", expect = "0F B6 05 81 F0 B6 00 69",
       unverified = "frontier2-a5", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "FPATAN of the camera front and the vehicle heading")]
pub fn gang_drive_by_player_choose_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let cam = active_cam();
    let mode = rd::<u16>(cam, 0xC);
    if mode != 0x37 && mode != 0x31 {
        wr::<u8>(this, 0xB, 0);
        wr::<u8>(this, 0xC, 1);
    } else {
        wr::<u8>(this, 0xB, 1);
        wr::<u8>(this, 0xC, 1);
        let cam = active_cam();
        let mut front = [rd::<f32>(cam, 0x190), rd::<f32>(cam, 0x194), rd::<f32>(cam, 0x198)];
        if rd::<u16>(cam, 0xC) == 0x31 {
            cam_get_two_player_aim_vector(cam, vec3f_ptr(&mut front));
        }
        let a = x(front[0]).fchs().fpatan(x(front[1]));
        let veh = rd::<GPtr<u8>>(ped, 0x58C);
        let mut d = a.fsub(placeable_heading(veh));
        if gt(d, x(PI)) {
            d = d.fsub_m(TWO_PI);
        } else if lt(d, x(MINUS_PI)) {
            d = d.fadd_m(TWO_PI);
        }
        d = d.fadd_m(PI_4);
        if lt(d, x(0.0)) {
            d = d.fadd_m(TWO_PI);
        }
        let q = ftol32(d.fmul_m(TWO_OVER_PI)) as i8;
        if let Some(id) = quadrant_anim(this, q) {
            wr::<u32>(this, 0x24, id);
        }
    }
    if is_coop_game_going_on() == 0 {
        camera_set_new_player_weapon_mode(GPtr::from_addr(THE_CAMERA), 0x37, 0, 0);
    }
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    if !pd.is_null() {
        wr::<u32>(pd, 0x34, rd::<u32>(pd, 0x34) | 8);
    }
    if ped_is_player(ped) != 0 {
        wr::<u8>(get_player_info(ped), 0x153, 0);
    }
}

/// The AI's drive-by anim choice (0x627600, `ret 4`). The target (+0x34's matrix times +0x38, or +0x38) relative to
/// the ped: its distance, and its angle to the vehicle heading (`atan2(-dx, dy)`, stored as f32 and wrapped into
/// (-pi, pi]). +0xB = 0, +0xC = distance below the weapon's target range (info +8). The abort range (+0x20): reached
/// once (+0xD) below it, then above it the next command is 7 (also for a target ped / vehicle without health); then
/// the line-of-sight test (0x621B10) on the target position; failed: +0xB = 0. Else the drive-by style (+0x12) picks
/// a quadrant (front 0, left 1, back 2, right 3; styles 2..4 / 7 keep the current anim unless it is none (0xBF)),
/// which sets the anim (+0x24) and +0xB when in weapon range and the angle lies in that quadrant.
#[hook(addr = 0x627600, conv = thiscall, name = "FUN_00627600", expect = "83 EC 34 53 55 8B 6C 24",
       unverified = "frontier2-a5", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "FPATAN / FSQRT of the target offset; the original keeps its side flag in its ped argument slot")]
pub fn gang_drive_by_ai_choose_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let range = rd::<f32>(rd::<GPtr<u8>>(this, 0x30), 8);
    let target = rd::<GPtr<u8>>(this, 0x34);
    let pp = entity_pos(ped);
    let (dx, dy, dz) = if !target.is_null() {
        let mut w = [0f32; 3];
        target_world_coords(this, target, &mut w);
        (
            x(w[0]).fsub_m(rd::<f32>(pp, 0)).to_f32(),
            x(w[1]).fsub_m(rd::<f32>(pp, 4)).to_f32(),
            x(w[2]).fsub_m(rd::<f32>(pp, 8)).to_f32(),
        )
    } else {
        (
            x(rd::<f32>(this, 0x38)).fsub_m(rd::<f32>(pp, 0)).to_f32(),
            x(rd::<f32>(this, 0x3C)).fsub_m(rd::<f32>(pp, 4)).to_f32(),
            x(rd::<f32>(this, 0x40)).fsub_m(rd::<f32>(pp, 8)).to_f32(),
        )
    };
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    let dist = x(dz).fmul_m(dz).fadd(x(dx).fmul_m(dx)).fadd(x(dy).fmul_m(dy)).fsqrt().to_f32();
    let ang = x(dx).fchs().fpatan(x(dy));
    let mut rel = ang.fsub(placeable_heading(veh)).to_f32();
    if gt(x(rel), x(PI)) {
        rel = x(rel).fsub_m(TWO_PI).to_f32();
    } else if lt(x(rel), x(MINUS_PI)) {
        rel = x(rel).fadd_m(TWO_PI).to_f32();
    }
    wr::<u8>(this, 0xB, 0);
    let in_range = lt(x(dist), x(range));
    wr::<u8>(this, 0xC, u8::from(in_range));
    let reached = rd::<u8>(this, 0xD);
    let abort = rd::<f32>(this, 0x20);
    let mut give_up = false;
    if reached == 0 {
        if lt(x(dist), x(abort)) {
            wr::<u8>(this, 0xD, 1);
        }
    } else if gt(x(dist), x(abort)) {
        give_up = true;
    }
    if !give_up {
        let t = rd::<GPtr<u8>>(this, 0x34);
        if !t.is_null() {
            let ty = rd::<u8>(t, 0x36) & 7;
            // a dead target ped (health +0x540) or vehicle (health +0x4C0)
            give_up = (ty == 3 && le_ordered(x(rd::<f32>(t, 0x540)), x(0.0)))
                || (ty == 2 && le_ordered(x(rd::<f32>(t, 0x4C0)), x(0.0)));
        }
    }
    if give_up {
        wr::<u8>(this, 0xF, 7);
        return;
    }
    let pp = entity_pos(ped);
    let mut pt = [
        x(dx).fadd_m(rd::<f32>(pp, 0)).to_f32(),
        x(dy).fadd_m(rd::<f32>(pp, 4)).to_f32(),
        x(dz).fadd_m(rd::<f32>(pp, 8)).to_f32(),
    ];
    if drive_by_los_clear(this, ped, vec3f_ptr(&mut pt)) as u8 == 0 {
        wr::<u8>(this, 0xB, 0);
        return;
    }
    let style = rd::<i8>(this, 0x12);
    let mut quad: i8 = -1;
    let mut side: i8 = 0;
    let mut fwd: i8 = 0;
    if !matches!(style, 0 | 1 | 5 | 6) {
        let mut t = x(rel).fadd_m(PI_4);
        if lt(t, x(0.0)) {
            t = t.fadd_m(TWO_PI);
        }
        quad = ftol32(t.fmul_m(TWO_OVER_PI)) as i8;
        let r = x(rel);
        if gt(r, x(fbits(0x3E860A92))) && lt(r, x(fbits(0x40384E89))) {
            side = -1;
        } else if lt(r, x(fbits(0xBE860A92))) && gt(r, x(fbits(0xC0384E89))) {
            side = 1;
        }
        if lt(r, x(fbits(0x3FA78D36))) && gt(r, x(fbits(0xBFA78D36))) {
            fwd = -1;
        } else if gt(r, x(fbits(0x3FEA9280))) && lt(r, x(fbits(0xBFEA9280))) {
            fwd = 1;
        }
    }
    let none = rd::<u32>(this, 0x24) == 0xBF;
    let by_quad = |keep: &[i8]| if keep.contains(&quad) { quad } else { -1 };
    let pick: i8 = match style {
        0 => 1,
        1 => 3,
        2 if none => 1,
        3 if none => 3,
        4 if none => match side {
            -1 => 1,
            1 => 3,
            _ => -1,
        },
        2..=4 => by_quad(&[1, 3]),
        5 => 0,
        6 => 2,
        7 if none => match fwd {
            -1 => 0,
            1 => 2,
            _ => -1,
        },
        7 => by_quad(&[0, 2]),
        8 => quad,
        _ => -1,
    };
    let Some(id) = quadrant_anim(this, pick) else {
        return;
    };
    wr::<u32>(this, 0x24, id);
    if ge_or_nan(x(dist), x(range)) {
        return;
    }
    let r = x(rel);
    let hit = match pick {
        0 => !r.fcom(x(MINUS_PI_4)).test_any(1) && le_ordered(r, x(PI_4)),
        1 => !r.fcom(x(PI_4)).test_any(1) && le_ordered(r, x(PI_3_4)),
        2 => !r.fcom(x(MINUS_PI_3_4)).test_any(1) || le_ordered(r, x(PI_3_4)),
        _ => !r.fcom(x(MINUS_PI_3_4)).test_any(1) && le_ordered(r, x(MINUS_PI_4)),
    };
    if hit {
        wr::<u8>(this, 0xB, 1);
    }
}

/// The drive-by's line-of-sight test (0x621B10, `ret 8`, EAX = !blocked). Once the time passes +0x18: +0x1C
/// (blocked) = 0, and with a target (+0x34) the line from the ped's position + 0.5 up to `pos` (buildings and
/// see-through only, the target ignored through `CWorld::pIgnoreEntity`) sets it when not clear; the next check at
/// now - ftol(rand16 * 2^-15 * -500) + 1750.
#[hook(addr = 0x621B10, conv = thiscall, name = "FUN_00621b10", expect = "A1 84 CB B7 00 83 EC 0C",
       unverified = "frontier2-a5", route = "L2", hazards = "heap-writes")]
pub fn gang_drive_by_los_clear(this: GPtr<u8>, ped: GPtr<u8>, pos: GPtr<u8>) -> u32 {
    if g::<u32>(TIME_MS).read() > rd::<u32>(this, 0x18) {
        let t = rd::<GPtr<u8>>(this, 0x34);
        wr::<u8>(this, 0x1C, 0);
        if !t.is_null() {
            let p = entity_pos(ped);
            let mut from = [rd::<f32>(p, 0), x(rd::<f32>(p, 4)).to_f32(), x(rd::<f32>(p, 8)).fadd_m(0.5).to_f32()];
            g::<GPtr<u8>>(IGNORE_ENTITY).write(t);
            if world_get_is_line_of_sight_clear(vec3f_ptr(&mut from), pos, 1, 0, 0, 0, 0, 1, 0) == 0 {
                wr::<u8>(this, 0x1C, 1);
            }
            g::<u32>(IGNORE_ENTITY).write(0);
        }
        let r = sa_core::orig::at_site(0x621B99, rand) as u32 & 0xFFFF;
        let d = ftol32(xi(r as i32).fmul_m(fbits(0x38000000)).fmul_m(-500.0));
        let now = g::<u32>(TIME_MS).read();
        wr::<u32>(this, 0x18, now.wrapping_sub(d as u32).wrapping_add(1750));
    }
    u32::from(rd::<u8>(this, 0x1C) == 0)
}

/// `r · axis` as the shot's plane tests compute it: `(a.y*m[1] + a.z*m[2]) + a.x*m[0]` or, with `zfirst`,
/// `(a.z*m[2] + a.y*m[1]) + a.x*m[0]` (the order of the x87 sums).
fn dot_axis(a: [f32; 3], m: GPtr<u8>, off: isize, zfirst: bool) -> X87 {
    let (ax, ay, az) = (x(rd::<f32>(m, off)), x(rd::<f32>(m, off + 4)), x(rd::<f32>(m, off + 8)));
    let s = if zfirst {
        x(a[2]).fmul(az).fadd(x(a[1]).fmul(ay))
    } else {
        x(a[1]).fmul(ay).fadd(x(a[2]).fmul(az))
    };
    s.fadd(x(a[0]).fmul(ax))
}

/// The drive-by shot (0x627CC0, `ret 4`). The muzzle (weapon info +0x24) through the ped's hand bone matrix
/// (`RwV3dTransformPoints`), relative to the vehicle. The direction: the player's camera target
/// (`Find3rdPersonCamTargetVector` with 20 m, + the camera distance in mode 0x31; nothing outside the modes 0x31 /
/// 0x37; player data +0x2C = 0), else the target (+0x34's matrix times +0x38, or +0x38), minus the muzzle,
/// normalised. The start point: where the direction crosses the vehicle's side / back / front plane (the vehicle
/// model's bounds +0x14 [0] / [3] / [1] / [4], 0.2 m out) for the anim's quadrant or the fake direction (+0x14), when
/// the direction leaves through that plane (dot above 0.1); else the muzzle. Ped flags +0x528 |= 1 (a no-CD island).
/// `CWeapon::Fire` from the muzzle (the target position null when +0x38 is zero), `DoGunFlash(0x8D2E90, false)`.
#[hook(addr = 0x627CC0, conv = thiscall, name = "FUN_00627cc0", expect = "83 EC 3C 53 55 8B E9 8B",
       unverified = "frontier2-a5", route = "L2", hazards = "heap-writes",
       note = "the original keeps the aim range in its ped argument slot; a no-CD island at 0x628234 reads +0x528")]
pub fn gang_drive_by_fire(this: GPtr<u8>, ped: GPtr<u8>) {
    let wi = rd::<GPtr<u8>>(this, 0x30);
    let mut m = [rd::<f32>(wi, 0x24), rd::<f32>(wi, 0x28), rd::<f32>(wi, 0x2C)];
    let hier = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(ped, 0x18));
    let idx = rp_hanim_id_get_index(hier, rd::<u32>(rd::<GPtr<u8>>(ped, 0x4A0), 0x14));
    let mats = rp_hanim_hierarchy_get_matrix_array(hier);
    let mp = vec3f_ptr(&mut m);
    rw_v3d_transform_points(mp.cast(), mp.cast(), 1, mats.byte_add((idx << 6) as isize));
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + rd::<i16>(veh, 0x22) as isize as usize * 4).read();
    let bounds = rd::<GPtr<u8>>(mi, 0x14);
    let vp = entity_pos(veh);
    let r = [
        x(m[0]).fsub_m(rd::<f32>(vp, 0)).to_f32(),
        x(m[1]).fsub_m(rd::<f32>(vp, 4)).to_f32(),
        x(m[2]).fsub_m(rd::<f32>(vp, 8)).to_f32(),
    ];
    let mut t = [0f32; 3];
    if ped_is_player(ped) != 0 {
        let Some(range) = player_aim_range(ped) else {
            return;
        };
        let mut cam_pos = [0f32; 3];
        camera_find_3rd_person_cam_target_vector(
            GPtr::from_addr(THE_CAMERA),
            range,
            m[0],
            m[1],
            m[2],
            vec3f_ptr(&mut cam_pos),
            vec3f_ptr(&mut t),
        );
        t = [
            x(t[0]).fsub_m(m[0]).to_f32(),
            x(t[1]).fsub_m(m[1]).to_f32(),
            x(t[2]).fsub_m(m[2]).to_f32(),
        ];
        wr::<u32>(rd::<GPtr<u8>>(ped, 0x480), 0x2C, 0);
    } else {
        let target = rd::<GPtr<u8>>(this, 0x34);
        let w = if !target.is_null() {
            let mut w = [0f32; 3];
            target_world_coords(this, target, &mut w);
            w
        } else {
            [rd::<f32>(this, 0x38), rd::<f32>(this, 0x3C), rd::<f32>(this, 0x40)]
        };
        t = [
            x(w[0]).fsub_m(m[0]).to_f32(),
            x(w[1]).fsub_m(m[1]).to_f32(),
            x(w[2]).fsub_m(m[2]).to_f32(),
        ];
    }
    vector_normalise(vec3f_ptr(&mut t).cast());
    let anim = rd::<GPtr<u8>>(this, 0x2C);
    let id = if anim.is_null() { None } else { Some(rd::<u16>(anim, 0x2C)) };
    let fake = rd::<i8>(this, 0x14);
    let vm = rd::<GPtr<u8>>(veh, 0x14);
    let mut start = m;
    // the side planes: the plane offset k stays on the x87 stack, the point is t*k + muzzle
    let side = move |sign_neg: bool, bound: isize, out: bool| -> Option<[f32; 3]> {
        let mut dot = dot_axis(t, vm, 0, true);
        if sign_neg {
            dot = dot.fchs();
        }
        if le_or_nan(dot, x(0.1)) {
            return None;
        }
        let e = x(rd::<f32>(bounds, bound)).fsub(dot_axis(r, vm, 0, false));
        let e = if out { e.fadd_m(fbits(0x3E4CCCCD)) } else { e.fsub_m(fbits(0x3E4CCCCD)) };
        let k = e.fdiv(dot);
        let ty = x(t[1]).fmul(k).to_f32();
        let tz = x(t[2]).fmul(k).to_f32();
        Some([
            x(t[0]).fmul(k).fadd_m(m[0]).to_f32(),
            x(ty).fadd_m(m[1]).to_f32(),
            x(tz).fadd_m(m[2]).to_f32(),
        ])
    };
    // the front / back planes: k stored as f32, the point is muzzle + t*k through the CVector helpers
    let ends = move |sign_neg: bool, bound: isize, out: bool| -> Option<[f32; 3]> {
        let (mut t, mut m) = (t, m);
        let mut dot = dot_axis(t, vm, 0x10, true);
        if sign_neg {
            dot = dot.fchs();
        }
        if le_or_nan(dot, x(0.1)) {
            return None;
        }
        let e = x(rd::<f32>(bounds, bound)).fsub(dot_axis(r, vm, 0x10, false));
        let e = if out { e.fadd_m(fbits(0x3E4CCCCD)) } else { e.fsub_m(fbits(0x3E4CCCCD)) };
        let k = e.fdiv(dot).to_f32();
        let mut tk = [0f32; 3];
        vector_scale(vec3f_ptr(&mut tk), k, vec3f_ptr(&mut t));
        let mut s = [0f32; 3];
        vector_add(vec3f_ptr(&mut s), vec3f_ptr(&mut m), vec3f_ptr(&mut tk));
        Some(s)
    };
    let p = if matches!(id, Some(0xE7 | 0xEC)) || fake == 1 {
        side(true, 0, false)
    } else if matches!(id, Some(0xEB | 0xE8)) || fake == 3 {
        side(false, 0xC, true)
    } else if matches!(id, Some(0xEA | 0xEE)) || fake == 2 {
        ends(true, 4, false)
    } else {
        ends(false, 0x10, true)
    };
    if let Some(p) = p {
        start = p;
    }
    wr::<u32>(ped, 0x528, rd::<u32>(ped, 0x528) | 1);
    let target = rd::<GPtr<u8>>(this, 0x34);
    let weapon = current_weapon(ped);
    let mp = vec3f_ptr(&mut m);
    if is_zero(rd::<f32>(this, 0x38)) && is_zero(rd::<f32>(this, 0x3C)) && is_zero(rd::<f32>(this, 0x40)) {
        weapon_fire(weapon, ped, mp, mp, target, GPtr::NULL, vec3f_ptr(&mut start));
    } else {
        let mut v = [rd::<f32>(this, 0x38), rd::<f32>(this, 0x3C), rd::<f32>(this, 0x40)];
        if !target.is_null() {
            target_world_coords(this, target, &mut v);
        }
        weapon_fire(weapon, ped, mp, mp, target, vec3f_ptr(&mut v), vec3f_ptr(&mut start));
    }
    ped_do_gun_flash(ped, u32::from(g::<u16>(0x8D2E90).read()), 0);
}

/// The drive-by gun pointing (0x628350, `ret 4`). Ped flags +0x528 bit 2 cleared. The aim point: the player's camera
/// target (from the ped + 0.7 up; nothing outside the modes 0x31 / 0x37), else the target ped's bone 3 (zero coords),
/// the target entity's position (zero coords), its matrix times +0x38, or +0x38. The yaw: `GetRadianAngleBetweenPoints`
/// from the ped (bone 0x16's height) to it; the pitch 0, on a bike `sin(yaw - heading) * lean(+0x648) +
/// angle(dz over the horizontal distance)` plus `asin(clamp(fwd.z)) * cos(yaw - heading)`. The anim's quadrant turns
/// the yaw by ∓pi/2 (flag set) or -pi, and flips the pitch for the left / back anims. `CPedIK::PointGunInDirection`.
#[hook(addr = 0x628350, conv = thiscall, name = "FUN_00628350", expect = "83 EC 34 53 55 56 57 8B",
       unverified = "frontier2-a5", route = "L2", hazards = "x87-transcendental,heap-writes,uninit-locals",
       note = "FPATAN / FSIN / FCOS, _CIasin; the flag argument's upper bytes are an uninitialised local in the original")]
pub fn gang_drive_by_point_gun(this: GPtr<u8>, ped: GPtr<u8>) {
    wr::<u32>(ped, 0x528, rd::<u32>(ped, 0x528) & 0xFFFF_FFFB);
    let mut v = [0f32; 3];
    if ped_is_player(ped) != 0 {
        let Some(range) = player_aim_range(ped) else {
            return;
        };
        let p = entity_pos(ped);
        let mut src = [
            f32::from_bits(rd::<u32>(p, 0)),
            x(rd::<f32>(p, 4)).to_f32(),
            x(rd::<f32>(p, 8)).fadd_m(fbits(0x3F333333)).to_f32(),
        ];
        let (sx, sy, sz) = (src[0], src[1], src[2]);
        camera_find_3rd_person_cam_target_vector(
            GPtr::from_addr(THE_CAMERA),
            range,
            sx,
            sy,
            sz,
            vec3f_ptr(&mut src),
            vec3f_ptr(&mut v),
        );
    } else {
        let t = rd::<GPtr<u8>>(this, 0x34);
        let zero = is_zero(rd::<f32>(this, 0x38)) && is_zero(rd::<f32>(this, 0x3C)) && is_zero(rd::<f32>(this, 0x40));
        if !t.is_null() {
            if zero {
                if rd::<u8>(t, 0x36) & 7 == 3 {
                    ped_get_bone_position(t, vec3f_ptr(&mut v), 3, 0);
                } else {
                    let p = entity_pos(t);
                    v = [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)];
                }
            } else {
                target_world_coords(this, t, &mut v);
            }
        } else if !zero {
            v = [rd::<f32>(this, 0x38), rd::<f32>(this, 0x3C), rd::<f32>(this, 0x40)];
        }
    }
    let mut bp = [0f32; 3];
    ped_get_bone_position(ped, vec3f_ptr(&mut bp), 0x16, 0);
    let pp = entity_pos(ped);
    bp[0] = rd::<f32>(pp, 0);
    bp[1] = rd::<f32>(pp, 4);
    let mut yaw = general_get_radian_angle_between_points(v[0], v[1], bp[0], bp[1]).to_f32();
    let mut pitch = 0f32;
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    if !veh.is_null() && rd::<u32>(veh, 0x590) == 9 {
        let a = x(bp[0]).fsub_m(v[0]);
        let b = x(bp[1]).fsub_m(v[1]);
        let s = b.fmul(b).fadd(a.fmul(a)).fsqrt().to_f32();
        let p = general_get_radian_angle_between_points(v[2], s, bp[2], 0.0);
        pitch = x(yaw)
            .fsub(placeable_heading(veh))
            .fsin()
            .fmul_m(rd::<f32>(veh, 0x648))
            .fadd(p)
            .to_f32();
        let c = x(yaw).fsub(placeable_heading(veh)).fcos().to_f32();
        let fz = rd::<f32>(rd::<GPtr<u8>>(veh, 0x14), 0x18);
        let clamped = if !x(-1.0).fcom(x(fz)).test_any(0x41) {
            -1.0
        } else if !x(1.0).fcom(x(fz)).test_parity(5) {
            1.0
        } else {
            fz
        };
        let asin = X87::from_f64_exact(sa_core::orig::x87_unary(0x82_1E70, x(clamped).to_f64()));
        pitch = asin.fmul_m(c).fadd_m(pitch).to_f32();
    }
    let id = rd::<u16>(rd::<GPtr<u8>>(this, 0x2C), 0x2C);
    let mut flag = 0u32;
    match id {
        0xE7 | 0xEC => {
            flag = 1;
            yaw = x(yaw).fsub_m(PI_2).to_f32();
            pitch = x(pitch).fmul_m(-1.0).to_f32();
        }
        0xEB | 0xE8 => {
            flag = 1;
            yaw = x(yaw).fadd_m(PI_2).to_f32();
        }
        0xEA | 0xEE => {
            yaw = x(yaw).fsub_m(PI).to_f32();
            pitch = x(pitch).fmul_m(-1.0).to_f32();
        }
        _ => {}
    }
    pedik_point_gun_in_direction(ped.byte_add(0x50C), yaw.to_bits(), pitch.to_bits(), flag, 0xBF80_0000);
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_GANG_DRIVE_BY_PROCESS_PED,
    HOOK_GANG_DRIVE_BY_PLAYER_CHOOSE_ANIM,
    HOOK_GANG_DRIVE_BY_AI_CHOOSE_ANIM,
    HOOK_GANG_DRIVE_BY_LOS_CLEAR,
    HOOK_GANG_DRIVE_BY_FIRE,
    HOOK_GANG_DRIVE_BY_POINT_GUN,
];
