//! `CTaskComplexProstituteSolicit` (PEDHIT: reached by the wander scan of a ped near the player's car, then her
//! task's sub-tasks): `IsTaskValid`, `CreateSubTask` and (FRONTIER-1) `ControlSubTask`. Ported from the 1.0 US bytes (`IsTaskValid` 0x661BB0 with
//! the displaced fragment 0x40160B, which only loads `ped+0x46C`; `CreateSubTask` 0x666360 leaves through the
//! fragment 0x401BAF, which only reloads the SEH link).

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use sa_core::orig::at_site;

use crate::common::*;
use sa_world::world::common::entity_pos as pos;

use sa_world::bindings::car_enter_exit_get_position_to_open_car_door as position_to_open_car_door;
use crate::bindings::intel_get_ped_entities;
use crate::bindings::pad_conversation_no_just_down;
use crate::bindings::pad_conversation_yes_just_down;
use sa_world::bindings::find_player_ped;
use sa_input::bindings::get_pad as pad_get_pad;
use sa_world::bindings::matrix_dtor;
use sa_core::bindings::matrix_invert as matrix_invert_into;
use sa_core::bindings::matrix_mul_vec;
use sa_core::bindings::messages_clear_messages;
use sa_core::bindings::tm_get_simplest_active_task;
use sa_world::bindings::apply_turn_force_6 as physical_apply_turn_force;
use sa_world::bindings::audio_add_event as vehicle_audio_add_audio_event;
use sa_world::bindings::car_drive_ctor;
use sa_world::bindings::enter_car_as_passenger_ctor;
use sa_world::bindings::get_player_info as player_ped_get_player_info;
use sa_world::bindings::get_vehicle_appearance as vehicle_get_appearance;
use sa_world::bindings::is_coop_game_going_on;
use sa_world::bindings::is_player_on_a_mission;
use sa_world::bindings::leave_car_ctor;
use sa_world::bindings::max_f;
use sa_world::bindings::min_f;
use sa_world::bindings::pad_get_accelerate;
use sa_world::bindings::pad_get_brake;
use sa_world::bindings::pad_start_shake;
use sa_world::bindings::player_info_add_health;
use sa_world::bindings::speech_set_cj_mood;
use sa_world::bindings::stand_still_ctor;
use sa_world::bindings::turn_to_face_entity_ctor as turn_ctor_entity;
use sa_world::bindings::vector_mul_scalar;
use sa_world::bindings::vehicle_is_on_its_side;
use sa_world::bindings::vehicle_is_upside_down;
use crate::bindings::is_task_valid;
use sa_world::bindings::wanted_set_wanted_level;
use crate::bindings::seek_entity_xy_offset_ctor as seek_entity_xy_ctor;
orig! {
    /// `CTaskComplexProstituteSolicit::CreateSubTask(int32, CPed*)` (0x666360)
    fn create_sub_task(t: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x666360 => prostitute_solicit_create_sub_task;
}

/// `CTaskComplexProstituteSolicit::IsTaskValid(CPed* prostitute, CPed* client)` (0x661BB0, cdecl; AL): true only
/// when `client` is the player (`FindPlayerPed(-1)`, non-null) in a vehicle (+0x46C bit 8, +0x58C), without
/// player-data flag +0x474 bit 6, whose player data's +0xA4 is unset or `prostitute`; the vehicle is a car
/// (`GetVehicleAppearance() == 1`), neither upside down nor on its side; the client's simplest active task is
/// 0x2C5 (the active task is not null-checked); the client drives it (+0x460); the prostitute is not in a vehicle
/// and its +0x484 byte is 0, or she is in this one and it is 1; the vehicle's +0x488 byte is non-zero and its
/// handling flags +0xCC bit 9 are clear; `|client - prostitute|² <= 100.0f` (0x86F840; NaN passes); no mission
/// and no co-op game.
#[hook(addr = 0x661BB0, conv = cdecl, name = "CTaskComplexProstituteSolicit::IsTaskValid",
       expect = "83 EC 0C 56 57 6A FF",
       unverified = "pedhit-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn prostitute_solicit_is_task_valid(prostitute: GPtr<u8>, client: GPtr<u8>) -> u8 {
    if find_player_ped(-1) != client || client.is_null() {
        return 0;
    }
    if rd::<u32>(client, 0x46C) & 0x100 == 0 {
        return 0;
    }
    let veh = rd::<GPtr<u8>>(client, 0x58C);
    if veh.is_null() || rd::<u8>(client, 0x474) & 0x40 != 0 {
        return 0;
    }
    let target = rd::<GPtr<u8>>(rd::<GPtr<u8>>(client, 0x480), 0xA4);
    if !target.is_null() && target != prostitute {
        return 0;
    }
    if vehicle_get_appearance(veh) as i32 != 1
        || vehicle_is_upside_down(rd::<GPtr<u8>>(client, 0x58C)) != 0
        || vehicle_is_on_its_side(rd::<GPtr<u8>>(client, 0x58C)) != 0
    {
        return 0;
    }
    let t = tm_get_simplest_active_task(rd::<GPtr<u8>>(client, 0x47C).byte_add(4));
    if task_get_type(t) != 0x2C5 {
        return 0;
    }
    let veh = rd::<GPtr<u8>>(client, 0x58C);
    if rd::<GPtr<u8>>(veh, 0x460) != client {
        return 0;
    }
    let pv = rd::<GPtr<u8>>(prostitute, 0x58C);
    if pv.is_null() {
        if rd::<u8>(veh, 0x484) != 0 {
            return 0;
        }
    } else if pv != veh || rd::<u8>(pv, 0x484) != 1 {
        return 0;
    }
    if rd::<u8>(veh, 0x488) == 0 || (rd::<u32>(rd::<GPtr<u8>>(veh, 0x384), 0xCC) >> 9) & 1 != 0 {
        return 0;
    }
    let mut d = [0f32; 3];
    vector_sub(GPtr::from_raw(d.as_mut_ptr()).cast(), pos(client), pos(prostitute));
    let sq = X87::from_f64_exact(vector_magnitude_sqr(GPtr::from_raw(d.as_mut_ptr()).cast()).to_f64());
    if !sq.fcom(x(100.0)).test_any(0x41) {
        return 0;
    }
    if is_player_on_a_mission() != 0 || is_coop_game_going_on() != 0 {
        return 0;
    }
    1
}

/// `(x² + y²) + z²` of `pos - p` with `pos` read from memory and the x difference stored as f32 first
/// (`CreateSubTask`'s inline door distances).
fn dist_sq(pos: GPtr<u8>, p: &[f32; 3], store_x: bool) -> X87 {
    let dz = x(rd::<f32>(pos, 8)).fsub_m(p[2]);
    let dy = x(rd::<f32>(pos, 4)).fsub_m(p[1]);
    let dx = x(rd::<f32>(pos, 0)).fsub_m(p[0]);
    let xx = if store_x {
        let s = dx.to_f32();
        x(s).fmul_m(s)
    } else {
        dx.fmul(dx)
    };
    xx.fadd(dy.fmul(dy)).fadd(dz.fmul(dz))
}

/// `CTaskComplexProstituteSolicit::CreateSubTask(int32 taskType, CPed* ped)` (0x666360, thiscall, `ret 8`; the
/// client at +0xC, its vehicle at client +0x58C):
/// - 0x2C0: `CTaskComplexLeaveCar(vehicle, 0, 0, true, false)` (0x34 bytes);
/// - 0x2BC: `CTaskComplexEnterCarAsPassenger(vehicle, 8, false)` (0x50);
/// - 0xCB: `CTaskSimpleStandStill(5000, false, false, 8.0f)` (0x20);
/// - 0x2D5: flags word +0x2C = (w & ~2) | 1, then `CTaskComplexCarDrive(vehicle)` (0x24);
/// - 0x398: `CTaskComplexTurnToFaceEntityOrCoord(client, [0x86FC7C], [0x86FC80])` (0x28);
/// - 0x38B: the passenger (door 10) or driver (door 8) door position (`GetPositionToOpenCarDoor`), whichever is
///   nearer the ped (door 8 unless door 8 is strictly farther: `(dx² + dy²) + dz²`, door 10's x difference stored
///   as f32), turned into the vehicle's frame by the inverse matrix; then
///   `CTaskComplexSeekEntity<XYOffset>(vehicle, 50000, 1000, [0x86FC2C], [0x86FC28], [0x86FC30], false, false)` (0x58)
///   with that offset at +0x44 (written even when the allocation failed, as the original).
/// Any other type, or a failed allocation (except the last case): null.
#[hook(addr = 0x666360, conv = thiscall, name = "CTaskComplexProstituteSolicit::CreateSubTask",
       expect = "64 A1 00 00 00 00 6A FF",
       unverified = "pedhit-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "the CMatrix local's unwritten bytes are 0 here; the SEH frame is not reproduced")]
pub fn prostitute_solicit_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    let veh = || rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 0x58C);
    let alloc = task_new;
    match ty {
        0x2C0 => {
            let t = alloc(0x34);
            if t.is_null() { t } else { leave_car_ctor(t, veh(), 0, 0, 1, 0) }
        }
        0x2BC => {
            let t = alloc(0x50);
            if t.is_null() { t } else { enter_car_as_passenger_ctor(t, veh(), 8, 0) }
        }
        0xCB => {
            let t = alloc(0x20);
            if t.is_null() { t } else { stand_still_ctor(t, 5000, 0, 0, f32::from_bits(8.0f32.to_bits())) }
        }
        0x2D5 => {
            wr::<u16>(this, 0x2C, (rd::<u16>(this, 0x2C) & 0xFFFD) | 1);
            let t = alloc(0x24);
            if t.is_null() { t } else { car_drive_ctor(t, veh()) }
        }
        0x398 => {
            let t = alloc(0x28);
            if t.is_null() {
                t
            } else {
                turn_ctor_entity(t, rd::<GPtr<u8>>(this, 0xC), g::<u32>(0x86_FC7C).read(), g::<u32>(0x86_FC80).read())
            }
        }
        0x38B => {
            let mut m = [0u32; 18];
            let mp: GPtr<u8> = GPtr::from_raw(m.as_mut_ptr()).cast();
            matrix_invert_into(rd::<GPtr<u8>>(veh(), 0x14), mp);
            let (mut p10, mut p8) = ([0f32; 3], [0f32; 3]);
            position_to_open_car_door(GPtr::from_raw(p10.as_mut_ptr()).cast(), veh(), 10);
            position_to_open_car_door(GPtr::from_raw(p8.as_mut_ptr()).cast(), veh(), 8);
            let s10 = dist_sq(pos(ped), &p10, true);
            let s8 = dist_sq(pos(ped), &p8, false);
            let mut door = if !s8.fcom(s10).test_any(0x41) { p10 } else { p8 };
            let mut off = [0f32; 3];
            matrix_mul_vec(GPtr::from_raw(off.as_mut_ptr()).cast(), mp, GPtr::from_raw(door.as_mut_ptr()).cast());
            let mut t = alloc(0x58);
            if !t.is_null() {
                t = seek_entity_xy_ctor(
                    t,
                    veh(),
                    50000,
                    1000,
                    g::<u32>(0x86_FC2C).read(),
                    g::<u32>(0x86_FC28).read(),
                    g::<u32>(0x86_FC30).read(),
                    0,
                    0,
                );
            }
            for (i, v) in off.iter().enumerate() {
                wr::<f32>(t, 0x44 + 4 * i as isize, *v);
            }
            matrix_dtor(mp);
            t
        }
        _ => GPtr::NULL,
    }
}

/// `TheText` (0xC1B340)
const THE_TEXT: usize = 0xC1_B340;
/// The first pad (`CPad::Pads[0]`, 0xB73458)
const PAD0: usize = 0xB7_3458;

/// `TheText.Get(key)` added to the message queue (`AddMessageQ(text, time, 1, true)`).
fn message(key: usize, time: u32) {
    messages_add_message_q(text_get(g::<u8>(THE_TEXT), g::<u8>(key)), time, 1, 1);
}

use sa_pop::carctrl4::lt_k as below;
/// `CTaskComplexProstituteSolicit::ControlSubTask(CPed* prostitute)` (0x6669D0, thiscall, `ret 4`). Fields: +0x8 the
/// sub-task, +0xC the client, +0x10 the meeting point, +0x1C / +0x20 / +0x24 times, +0x28 / +0x2A word counters,
/// +0x2C flag word. Unreversed in gta-reversed; from the 1.0 US bytes:
/// - flag bit 8 is set to bit 1; an invalid solicit (`IsTaskValid(prostitute, client)` false) sets bit 2, clears 8;
/// - `dt = ftol(CTimer::ms_fTimeStep * [0x858B38] * [0x858C4C])`;
/// - bit 2 set (leaving): with +0x2A running, count it down by dt and when it ends set the CJ mood (9, 120000) and let
///   the client say 0xB (+0x2A = 0); with +0x2A at 0 start the leave-car sub-task 0x2C0 once the current one aborts;
/// - sub-task 0xCB (waiting at the window): conversation "yes" (bit 10, the client says 0xC4, next sub-task) or "no"
///   (the client says 0xC3, bit 2);
/// - 0x38B / 0x2BC (walking to / entering the car): bit 2 when the car is farther than `[0x86F85C]` (squared) from
///   the meeting point;
/// - 0x2D5 (in the car): unless `[0xB6F0D8] == 1` (then +0x1C = now, +0x2A = 8000, bits 7 and 11, bit 8 cleared):
///   "stopped" = `|car speed * 50|² < [0x86F84C]` (bit 7 and +0x1C restarted otherwise); once a second the scanner's 16
///   peds set bits 4 / 3 (`|ped - prostitute|²` below `[0x86F854]` / `[0x86F850]`), and a cop (type 6) with bit 1 and
///   no wanted level who sees the client (line of sight with the car's collision bit cleared) sets bit 5; then
///   bit 0 (parked): after 4 s stopped, the help text 0x86FC60 (bit 0 → 1) or, when bit 3, 0x86FC58 once (bit 12);
///   bit 1 (in action): bit 6 counts a stat 0xBE once; accelerate / brake, bit 4 or a cop who saw (wanted level 1)
///   stop it (bits 0 and 11, +0x1C = now; messages 0x86FC3C / 0x86FC58 by the random draw and +0x2A); otherwise the
///   rocking (+0x28 counts down, a random range `[-0.5, -0.9]` force by +0x2A, pad shake below 1000, a turn force on
///   the car on one side by the random's bit 0, vehicle audio event 0x6C, ped speech 0x70 / 0x74 with bit 9), then
///   +0x2A counts down (bit 2 at 0) and every second the client pays 2 (money cheat: earns 2; stat 0x21, the
///   prostitute's +0x756 counter) or, broke, the scene ends (messages 0x86FC44 / 0x86FC3C), and gets 2 health unless
///   bit 11.
///
/// Returns the current sub-task unless noted.
#[hook(addr = 0x6669D0, conv = thiscall, name = "CTaskComplexProstituteSolicit::ControlSubTask",
       this = "CTaskComplexProstituteSolicit", expect = "83 EC 30 53 56 8B F1",
       unverified = "frontier-1-1", route = "L2", hazards = "heap-writes",
       note = "rand at_site 0x666EA5 (and inside GetRandomNumberInRange); messages, stats, money, pad shake, turn force on the client's car")]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub fn prostitute_solicit_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = || rd::<GPtr<u8>>(this, 8);
    let client = || rd::<GPtr<u8>>(this, 0xC);
    let fl = || rd::<u16>(this, 0x2C);
    let set_fl = |v: u16| wr::<u16>(this, 0x2C, v);
    let f = fl();
    if f & 2 != 0 {
        if f & 0x100 == 0 {
            set_fl(f | 0x100);
        }
    } else if f & 0x100 != 0 {
        set_fl(f & 0xFEFF);
    }
    if is_task_valid(ped, client()) == 0 {
        set_fl((fl() & 0xFEFF) | 4);
    }
    let dt = ftol32(
        x(g::<f32>(0xB7_CB5C).read())
            .fmul_m(g::<f32>(0x85_8B38).read())
            .fmul_m(g::<f32>(0x85_8C4C).read()),
    );
    let pad = g::<u8>(PAD0);

    if rd::<u8>(this, 0x2C) & 4 != 0 {
        let c = rd::<u16>(this, 0x2A);
        if c != 0 {
            let n = u32::from(c).wrapping_sub(dt as u32) as u16;
            wr::<u16>(this, 0x2A, n);
            if n as i16 > 0 {
                return sub();
            }
            speech_set_cj_mood(9, 120_000, -1i32 as u32, -1i32 as u32, -1i32 as u32);
            ped_say(client(), 0xB, 0, 1.0, 0, 0, 0);
            let r = sub();
            wr::<u16>(this, 0x2A, 0);
            return r;
        }
        if task_get_type(sub()) == 0x2C0 {
            return sub();
        }
        if task_make_abortable(sub(), ped, 1, GPtr::NULL) == 0 {
            return sub();
        }
        return create_sub_task(this, 0x2C0, ped);
    }

    if task_get_type(sub()) == 0xCB {
        if pad_conversation_yes_just_down(pad) != 0 {
            wr::<u8>(this, 0x2D, rd::<u8>(this, 0x2D) | 4);
            let c = client();
            if !c.is_null() {
                ped_say(c, 0xC4, 0, 1.0, 0, 0, 0);
            }
            return task_create_next_sub_task(this, ped);
        }
        if pad_conversation_no_just_down(pad) == 0 {
            return sub();
        }
        let c = client();
        if !c.is_null() {
            ped_say(c, 0xC3, 0, 1.0, 0, 0, 0);
        }
        set_fl(fl() | 4);
        return sub();
    }
    if task_get_type(sub()) == 0x38B || task_get_type(sub()) == 0x2BC {
        let p = pos(rd::<GPtr<u8>>(client(), 0x58C));
        let dx = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(this, 0x10));
        let dy = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(this, 0x14));
        let dz = x(rd::<f32>(p, 8)).fsub_m(rd::<f32>(this, 0x18));
        let sq = dx.fmul(dx).fadd(dz.fmul(dz)).fadd(dy.fmul(dy));
        if !sq.fcom(x(g::<f32>(0x86_F85C).read())).test_any(0x41) {
            set_fl(fl() | 4);
        }
        return sub();
    }
    if task_get_type(sub()) != 0x2D5 {
        return sub();
    }

    let now = g::<u32>(0xB7_CB84).read();
    if g::<u32>(0xB6_F0D8).read() == 1 {
        let f = fl();
        let r = sub();
        wr::<u32>(this, 0x1C, now);
        wr::<u16>(this, 0x2A, 8000);
        set_fl((f & 0xFEFF) | 0x880);
        return r;
    }
    let mut v = [0u32; 3];
    let vp: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr()).cast();
    vector_mul_scalar(vp, rd::<GPtr<u8>>(client(), 0x58C).byte_add(0x44), 50.0);
    let slow = below(vector_magnitude_sqr_x(vp), g::<f32>(0x86_F84C).read());
    if !slow || rd::<u8>(this, 0x2C) & 0x80 != 0 {
        wr::<u8>(this, 0x2C, rd::<u8>(this, 0x2C) & 0x7F);
        wr::<u32>(this, 0x1C, now);
    }
    let wanted = find_player_wanted(-1);
    if now > rd::<u32>(this, 0x20) {
        wr::<u8>(this, 0x2C, rd::<u8>(this, 0x2C) & 0xC7);
        wr::<u32>(this, 0x20, now.wrapping_add(1000));
        let list = intel_get_ped_entities(rd::<GPtr<u8>>(ped, 0x47C));
        for i in 0..16isize {
            let e = rd::<GPtr<u8>>(list, i * 4);
            let c = client();
            if e.is_null() || e == c || rd::<u32>(e, 0x598) == 0x16 {
                continue;
            }
            let mut d = [0u32; 3];
            let dp: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
            vector_sub(dp, pos(e), pos(ped));
            let sq = vector_magnitude_sqr_x(dp);
            if below(sq, g::<f32>(0x86_F854).read()) {
                wr::<u8>(this, 0x2C, rd::<u8>(this, 0x2C) | 0x10);
            }
            if below(sq, g::<f32>(0x86_F850).read()) {
                wr::<u8>(this, 0x2C, rd::<u8>(this, 0x2C) | 8);
            }
            if rd::<u32>(e, 0x598) != 6
                || rd::<u8>(this, 0x2C) & 2 == 0
                || wanted.is_null()
                || rd::<i32>(wanted, 0x2C) >= 1
            {
                continue;
            }
            let veh = rd::<GPtr<u8>>(c, 0x58C);
            let saved = rd::<u32>(veh, 0x1C) & 1;
            wr::<u32>(veh, 0x1C, rd::<u32>(veh, 0x1C) & !1);
            let seen = world_get_is_line_of_sight_clear(pos(e), pos(client()), 1, 1, 0, 1, 0, 1, 0);
            let veh = rd::<GPtr<u8>>(client(), 0x58C);
            let f = rd::<u32>(veh, 0x1C);
            wr::<u32>(veh, 0x1C, f ^ ((saved ^ f) & 1));
            if seen != 0 {
                wr::<u8>(this, 0x2C, rd::<u8>(this, 0x2C) | 0x20);
            }
        }
    }

    let f = fl();
    if f & 1 != 0 {
        if slow && now.wrapping_sub(rd::<u32>(this, 0x1C)) > 4000 {
            if f & 8 == 0 {
                set_fl((f & 0xFFFE) | 2);
                wr::<u32>(this, 0x24, now);
                message(0x86_FC60, 2000);
            } else if f & 0x1000 == 0 {
                message(0x86_FC58, 3000);
                let r = sub();
                wr::<u8>(this, 0x2D, rd::<u8>(this, 0x2D) | 0x10);
                return r;
            }
        }
        return sub();
    }
    if f & 2 == 0 {
        return sub();
    }
    if f & 0x40 != 0 {
        set_fl(f & 0xFFBF);
        wr::<u16>(this, 0x2A, 15000);
        stats_increment_stat(0xBE, 1.0);
    }
    let stop = pad_get_accelerate(pad) as i16 != 0 || pad_get_brake(pad) as i16 != 0;
    let mut r = at_site(0x66_6EA5, rand) as u32;
    let mut cop = false;
    if rd::<u8>(this, 0x2C) & 0x20 != 0 && !wanted.is_null() && rd::<i32>(wanted, 0x2C) < 1 {
        wanted_set_wanted_level(find_player_wanted(-1), 1);
        cop = true;
    }

    if stop || rd::<u8>(this, 0x2C) & 0x10 != 0 || cop {
        wr::<u32>(this, 0x1C, now);
        let f = (fl() & 0xFFFD) | 0x801;
        set_fl(f);
        let low = rd::<i16>(this, 0x2A) < 3000;
        if stop {
            if r < 0x1FFF_FFFF || low {
                set_fl(f | 4);
                wr::<u16>(this, 0x2A, 0);
                message(0x86_FC3C, 3000);
                return sub();
            }
        } else {
            if low {
                set_fl(f | 4);
                let s = sub();
                wr::<u16>(this, 0x2A, 0);
                return s;
            }
            message(0x86_FC58, 3000);
        }
        let s = sub();
        wr::<u16>(this, 0x2A, 8000);
        return s;
    }

    let n = rd::<u16>(this, 0x28).wrapping_sub(dt as u16);
    wr::<u16>(this, 0x28, n);
    if n as i16 <= 0 {
        let mut rf = general_random_float_in_range_x(-0.5, -0.9).to_f32();
        let a = rd::<i16>(this, 0x2A);
        if a > 10000 {
            wr::<u16>(this, 0x28, 850);
        } else if a > 5000 {
            wr::<u16>(this, 0x28, 450);
        } else if a > 1000 {
            wr::<u16>(this, 0x28, 120);
        } else {
            rf = x(rf).fmul_m(g::<f32>(0x85_8B8C).read()).to_f32();
            wr::<u16>(this, 0x28, 850);
            pad_start_shake(pad_get_pad(0), 1000, 0x78, 0);
        }
        let c = client();
        let veh = rd::<GPtr<u8>>(c, 0x58C);
        let vp = pos(veh);
        let (vx, vy) = (rd::<f32>(vp, 0), rd::<f32>(vp, 4));
        let cp = pos(c);
        let (cx, cy) = (rd::<f32>(cp, 0), rd::<f32>(cp, 4));
        let cm = rd::<f32>(c, 0x8C);
        let vm = x(rd::<f32>(veh, 0x8C)).fmul_m(g::<f32>(0x86_3E0C).read()).to_f32();
        let lo = X87::from_f64_exact(min_f(150.0f32.to_bits(), vm.to_bits()).to_f64()).to_f32();
        let m = X87::from_f64_exact(max_f(lo, cm).to_f64());
        let side = r & 1 != 0;
        let ty = if side { cy } else { rd::<f32>(pos(ped), 4) };
        let dy = x(ty).fsub_m(vy).to_f32();
        let tx = if side { cx } else { rd::<f32>(pos(ped), 0) };
        let dx = x(tx).fsub_m(vx).to_f32();
        let fz = m.fmul_m(rf).to_f32();
        physical_apply_turn_force(veh, 0.0, 0.0, fz, dx, dy, 0.0);
        vehicle_audio_add_audio_event(rd::<GPtr<u8>>(client(), 0x58C).byte_add(0x138), 0x6C, 0.0);
        if rd::<u8>(this, 0x2D) & 2 != 0 && r > 0x0FFF_FFFF {
            r &= 0xFFFF;
            ped_say(ped, if r < 0xFF { 0x70 } else { 0x74 }, 0, 0.5, 0, 0, 0);
        }
    }

    let n = rd::<u16>(this, 0x2A).wrapping_sub(dt as u16);
    wr::<u16>(this, 0x2A, n);
    if n as i16 <= 0 {
        wr::<u16>(this, 0x2A, 3000);
        set_fl((fl() & 0xFFFD) | 4);
    }
    if now.wrapping_sub(rd::<u32>(this, 0x24)) <= 1000 {
        return sub();
    }
    wr::<u32>(this, 0x24, now);
    let pi = player_ped_get_player_info(client());
    if g::<u8>(0x96_918A).read() != 0 {
        wr::<i32>(pi, 0xB8, rd::<i32>(pi, 0xB8).wrapping_add(2));
    } else {
        let money = rd::<i32>(pi, 0xB8);
        if money >= 2 {
            wr::<i32>(pi, 0xB8, money - 2);
            stats_increment_stat(0x21, 2.0);
            wr::<u16>(ped, 0x756, rd::<u16>(ped, 0x756).wrapping_add(1));
        } else {
            wr::<i32>(pi, 0xB8, 0);
            wr::<u16>(this, 0x2A, 0);
            set_fl((fl() & 0xFFFD) | 4);
            messages_clear_messages(0);
            message(0x86_FC44, 2000);
            message(0x86_FC3C, 3000);
        }
    }
    if rd::<u8>(this, 0x2D) & 8 != 0 {
        return sub();
    }
    player_info_add_health(pi, 2);
    sub()
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_PROSTITUTE_SOLICIT_IS_TASK_VALID,
    HOOK_PROSTITUTE_SOLICIT_CREATE_SUB_TASK,
    HOOK_PROSTITUTE_SOLICIT_CONTROL_SUB_TASK,
];
