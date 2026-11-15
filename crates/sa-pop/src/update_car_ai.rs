//! FRONTIER-2: `CCarAI::UpdateCarAI` (0x41DA30) as a whole port, replacing the ADR-002 partial port of PORT-POP-3 /
//! STD-12 / STD-14 / INTRO-2 (its cruise / goto-coordinates paths are the same code, now reached through the mission
//! switch), and `CCarAI::TellOccupantsToLeaveCar` (0x41C760), which the police path calls. Ported from the 1.0 US
//! bytes; gta-reversed read as an aid. Where it differs the bytes are followed (the differences are named at the
//! cases: mission 0x3B also switches to mission 0x16, the switch distances are 2D lengths, not squares, compared
//! with the straight-line distance byte or `FindSwitchDistanceFar`, mission 18 switches to 17 when far, the
//! cop bike's exit time is 20 s and the other bikes' 2.5 s, the chase steering uses magnitudes, not squares).

use sa_core::abi::HookReg;
use sa_core::bindings::{get_atan_of_xy, task_new, vector_magnitude, vector_normalise};
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::fpu::{gt, lt};
use sa_math::x87::X87;
use sa_world::bindings::{
    cull_zones_no_police, cull_zones_police_abandon_cars, enter_car_as_driver_ctor, event_group_add,
    event_script_command_ctor, event_script_command_dtor, find_player_entity, find_player_speed,
    find_player_wanted, gang_drive_by_ctor, get_has_collided_with, get_right_direction,
    get_task_swim, get_vehicle_appearance, intel_find_task_by_type, join_car_with_road_system_goto_coors,
    leave_car_ctor, ped_set_current_weapon, placeable_get_top_direction, player_get_wanted_level,
    sequence_add_task, sequence_ctor, tm_set_task, uses_siren, vector_add, vector_mul_scalar, vector_sub,
    vehicle_is_upside_down,
};
use sa_world::world::common::entity_pos_f as entity_pos;
use sa_world::world::garage_update::player_vehicle as fpv;

use crate::bindings::{
    find_max_steer_angle, find_police_boat_mission, find_police_car_mission, find_police_car_speed,
    find_speed_multiplier_with_speed_from_nodes, join_car_with_road_system, should_car_stop_for_bridge,
    switch_vehicle_to_real_physics, tell_occupants_to_leave_car as tell,
};
use crate::carctrl3::time_ms as now;
use crate::common::{find_player_coors, find_player_ped, ftol32, g, rand_at, rd, wr, x, xi};
use sa_core::bindings::vector_dot_product as dot_product;
use sa_core::field::gf as kf;

/// Globals owned by this module: none.
pub const GLOBALS: &[GlobalDesc] = &[];

orig! {
    /// `CCarAI::BackToCruisingIfNoWantedLevel(CVehicle*)` (0x41BFA0)
    fn back_to_cruising(v: GPtr<u8>) = cdecl 0x41BFA0 => crate::carctrl4::back_to_cruising_if_no_wanted_level;
    /// `CCarAI::MellowOutChaseSpeed(CVehicle*)` (0x41D3D0)
    fn mellow_out(v: GPtr<u8>) = cdecl 0x41D3D0 => crate::carctrl4::mellow_out_chase_speed;
    /// `CCarAI::MellowOutChaseSpeedBoat(CVehicle*)` (0x41CB70)
    fn mellow_out_boat(v: GPtr<u8>) = cdecl 0x41CB70 => crate::carctrl2::mellow_out_chase_speed_boat;
    /// `CCarAI::EntitiesGoHeadOn(CEntity*, CEntity*)` (0x41CD00, AL)
    fn entities_go_head_on(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x41CD00 => crate::carctrl4::entities_go_head_on;
    /// `CCarAI::FindSwitchDistanceFar(CVehicle*)` (0x41BF70, ST0; ported unhookable in carctrl2)
    fn switch_distance_far(v: GPtr<u8>) -> X87 = reg("args = stack") 0x41BF70 => crate::carctrl2::find_switch_distance_far;
    /// `CTaskComplexSequence::Contains(eTaskType)` (0x41BF10, AL)
    fn sequence_contains(s: GPtr<u8>, ty: i32) -> u8 = thiscall 0x41BF10 => unported;
    /// virtual `CTask::GetTaskType()` (vtable +0x10)
    fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// virtual `CVehicle::PlayCarHorn()` (vtable +0xD8)
    fn play_car_horn(v: GPtr<u8>) = thiscall vslot 0xD8 => unported;
}

fn lp(v: &mut [f32; 3]) -> GPtr<f32> {
    GPtr::from_raw(v.as_mut_ptr())
}

fn law(veh: GPtr<u8>) -> bool {
    rd::<u8>(veh, 0x428) & 1 != 0
}

/// `FindPlayerCoors(&tmp, -1)` into a fresh local.
fn fpc(buf: &mut [f32; 3]) -> GPtr<f32> {
    find_player_coors(lp(buf), -1)
}

/// The player's wanted level read inline: `FindPlayerPed(-1) + 0x480` (null → 0), its first dword, +0x2C.
fn wanted_inline() -> i32 {
    let pd = rd::<GPtr<u8>>(find_player_ped(-1), 0x480);
    let w = if pd.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(pd, 0) };
    rd::<i32>(w, 0x2C)
}

/// `CPlayerPed::GetWantedLevel` of `FindPlayerPed(-1)` (0x41BE60).
fn wanted_level() -> i32 {
    player_get_wanted_level(find_player_ped(-1))
}

/// `CVector::Magnitude` (0x4082C0) of the vector at `v`.
fn mag(v: GPtr<u8>) -> X87 {
    X87::from_f64_exact(vector_magnitude(v.cast::<u32>()))
}

fn straight(veh: GPtr<u8>) -> X87 {
    xi(i32::from(rd::<u8>(veh, 0x3DD)))
}

fn clear_siren(veh: GPtr<u8>) {
    wr(veh, 0x42D, rd::<u8>(veh, 0x42D) & 0x7F);
}

fn siren_on_if_used(veh: GPtr<u8>) {
    if uses_siren(veh) != 0 {
        wr(veh, 0x42D, rd::<u8>(veh, 0x42D) | 0x80);
    }
}

/// The 2D vehicle–player distance as the chase cases compute it: four `FindPlayerCoors` calls, `(c.y - p.y)` through
/// an f32 slot times `(e.y - p.y)`, plus `(a.x - p.x) * (b.x - p.x)`, square root.
fn dist_player4(veh: GPtr<u8>) -> X87 {
    let p = entity_pos(veh);
    let (mut a, mut b, mut c, mut e) = ([0f32; 3], [0f32; 3], [0f32; 3], [0f32; 3]);
    let a = fpc(&mut a);
    let b = fpc(&mut b);
    let c = fpc(&mut c);
    let t = x(c.wrapping_index(1).read()).fsub_m(p.wrapping_index(1).read()).to_f32();
    let e = fpc(&mut e);
    let yy = x(e.wrapping_index(1).read()).fsub_m(p.wrapping_index(1).read()).fmul_m(t);
    let xx = x(a.read()).fsub_m(p.read()).fmul(x(b.read()).fsub_m(p.read()));
    yy.fadd(xx).fsqrt()
}

/// `sqrt((t.y - v.y)^2 + (t.x - v.x)^2)` of the points `t` and `v` (x87).
fn dist2d(t: GPtr<f32>, v: GPtr<f32>) -> X87 {
    let dy = x(t.wrapping_index(1).read()).fsub_m(v.wrapping_index(1).read());
    let dx = x(t.read()).fsub_m(v.read());
    dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt()
}

/// The 2D distance from the vehicle to its destination (+0x3EC).
fn dist_dest(veh: GPtr<u8>) -> X87 {
    dist2d(veh.byte_add(0x3EC).cast(), entity_pos(veh))
}

/// The 2D distance from the vehicle to its target entity (+0x41C).
fn dist_target(veh: GPtr<u8>, t: GPtr<u8>) -> X87 {
    dist2d(entity_pos(t), entity_pos(veh))
}

fn set_temp(veh: GPtr<u8>, action: u8, ms: u32) {
    wr::<u8>(veh, 0x3BB, action);
    wr(veh, 0x3BC, now().wrapping_add(ms));
}

/// TellOccupantsToLeaveCar, cruise speed 0, mission 0, and the siren off at wanted level <= 1.
fn give_up_chase(veh: GPtr<u8>) {
    tell(veh);
    wr::<u8>(veh, 0x3D0, 0);
    wr::<u8>(veh, 0x3BA, 0);
    if wanted_level() <= 1 {
        clear_siren(veh);
    }
}

/// `CCarAI::UpdateCarAI(CVehicle*)` (0x41DA30; entry `nop; jmp` into its SEH prologue). A vehicle recording whose
/// `bUseCarAI` (0x97D6C0) is off and vehicle type 3 return at once. Mission 0x3B with wanted level < 3 or the player on
/// foot gets the destination (-6000, 10000, 500) and mission 0x16. Law enforcers on missions 2..5, 0x2B, 0x2C, 0x36,
/// 0x37 take `FindPoliceCarSpeedForWantedLevel` as cruise speed. Status 2 / 3 / 12 runs the mission switch
/// ([`mission_switch`]), status 4 / 5 sets cruise speed 0 and mission 0. Law enforcers with wanted level >= 1 give up
/// where `CCullZones::PoliceAbandonCars()`. Then [`tail`].
#[hook(addr = 0x41DA30, conv = cdecl, name = "CCarAI::UpdateCarAI", expect = "90 E9 12 62 FE FF",
       unverified = "frontier2-3", route = "L2", hazards = "heap-writes", recs = "grove-walk,driving",
       note = "whole port (was the std12-pop-1 partial port); the police path is new; TellOccupantsToLeaveCar 0x41C760 is ported with it")]
pub fn update_car_ai(veh: GPtr<u8>) {
    let rec = rd::<i8>(veh, 0x424);
    if rec >= 0 && g::<u8>(0x97D6C0 + rec as usize).read() == 0 {
        return;
    }
    if rd::<u32>(veh, 0x594) == 3 {
        return;
    }
    if rd::<u8>(veh, 0x3BA) == 0x3B && (wanted_inline() < 3 || fpv().is_null()) {
        wr::<u32>(veh, 0x3EC, 0xC5BB_8000);
        wr::<u32>(veh, 0x3F0, 0x461C_4000);
        wr::<u8>(veh, 0x3BA, 0x16);
        wr::<u32>(veh, 0x3F4, 0x43FA_0000);
    }
    if law(veh) && matches!(rd::<u8>(veh, 0x3BA), 4 | 2 | 5 | 3 | 0x2B | 0x2C | 0x36 | 0x37) {
        wr(veh, 0x3D0, find_police_car_speed(veh) as u8);
    }
    match rd::<u8>(veh, 0x36) >> 3 {
        2 | 3 | 12 => mission_switch(veh),
        4 | 5 => {
            wr::<u8>(veh, 0x3D0, 0);
            wr::<u8>(veh, 0x3BA, 0);
        }
        _ => {}
    }
    // 0x41FA92 (the law-enforcer test through the no-CD chunk at 0x40A4AD)
    if law(veh) && wanted_inline() >= 1 && cull_zones_police_abandon_cars() != 0 {
        tell(veh);
        wr::<u8>(veh, 0x3D0, 0);
        wr::<u8>(veh, 0x3BA, 0);
    }
    tail(veh);
}

/// The mission switch of status 2 / 3 / 12 (jump tables 0x420694 / 0x420708, `movsx` of the mission: 0, 1 and
/// missions >= 0x80 take the default). Every case ends at 0x41FA92 (or 0x41FADB, which skips the law-enforcer test
/// only for vehicles that are not law enforcers, so the two are the same).
fn mission_switch(veh: GPtr<u8>) {
    let m = rd::<u8>(veh, 0x3BA);
    match m {
        2 => ram_or_block_far(veh, 3),
        4 => ram_or_block_far(veh, 5),
        3 => ram_or_block_close(veh, true),
        5 => ram_or_block_close(veh, false),
        8 | 12 => {
            if gt(straight(veh), dist_dest(veh)) {
                wr::<u8>(veh, 0x3BA, m + 1);
            }
        }
        9 | 13 => goto_straight_line(veh, m),
        15 | 17 | 52 | 54 | 61 => target_far(veh, m),
        16 | 18 | 53 | 55 | 60 => target_close(veh, m),
        24 => {
            // fcomp 1.5; test ah, 5; jp: only less switches
            if lt(dist_dest(veh), x(kf(0x858CE8))) {
                wr::<u8>(veh, 0x3BA, 0x19);
            }
            back_to_cruising(veh);
        }
        25 => drive_towards_player_2(veh),
        26 => block_forward_and_back(veh),
        21 => {
            if !law(veh) {
                return;
            }
            if fpv().is_null() {
                wr::<u8>(veh, 0x3BA, 0x40);
            }
            mellow_out_boat(veh);
            back_to_cruising(veh);
        }
        64 => {
            if !law(veh) {
                return;
            }
            if !fpv().is_null() {
                wr::<u8>(veh, 0x3BA, 0x15);
            }
            wr::<u8>(veh, 0x3D0, 0xA);
            back_to_cruising(veh);
        }
        43 => {
            if gt(straight(veh), dist_player4(veh)) {
                wr::<u8>(veh, 0x3BA, 0x2C);
                siren_on_if_used(veh);
            }
            back_to_cruising(veh);
        }
        44 => approach_player_close(veh),
        65..=68 => {
            // add byte [esi+0x3BA], 0xDC: back to 29..32
            if gt(straight(veh), dist_player4(veh)) {
                wr(veh, 0x3BA, m.wrapping_add(0xDC));
            }
        }
        29..=32 => {
            if rd::<u8>(veh, 0x3DD) >= 0xF0 {
                return;
            }
            let d = dist_player4(veh).to_f32();
            if lt(switch_distance_far(veh), x(d)) {
                let mut c = [0f32; 3];
                if join_car_with_road_system_goto_coors(veh, fpc(&mut c).cast(), 1, 0) == 0 {
                    wr(veh, 0x3BA, m.wrapping_add(0x24));
                }
            }
        }
        10 | 27 | 28 => {}
        _ => police_default(veh),
    }
}

/// Missions 2 / 4 (ram / block the player, far away, 0x41DB72 / 0x41DF54): closer than the straight-line distance
/// byte, or `EntitiesGoHeadOn(FindPlayerEntity(-1), veh)` and closer than 40 → the close mission, siren on when the
/// vehicle uses one. Then `BackToCruisingIfNoWantedLevel`.
fn ram_or_block_far(veh: GPtr<u8>, close: u8) {
    let d = dist_player4(veh);
    let df = d.to_f32();
    // fild straight; fxch; fcompp; test ah, 5; jnp: d < straight switches
    let switch = lt(d, straight(veh))
        || (entities_go_head_on(find_player_entity(-1), veh) != 0 && lt(x(df), x(kf(0x858A10))));
    if switch {
        wr::<u8>(veh, 0x3BA, close);
        siren_on_if_used(veh);
    }
    back_to_cruising(veh);
}

/// Missions 3 / 5 (ram / block the player, close, 0x41DC92 / 0x41E07D). Further than `FindSwitchDistanceFar`: when
/// `JoinCarWithRoadSystemGotoCoors(veh, player, true, false)` fails, the far mission (2 / 4), horn counter 0 and the
/// siren off. Otherwise (ram only) a collision with the player's vehicle starts reversing (temp action 3) for 800 ms
/// when that vehicle is slower than 0.05, else 50 ms, unless the temp action is 7 / 8; the cops-in-car timer (+0x4DC)
/// counts `timestep * 16.666666` ms while the player's vehicle is slower than 0.05 (ram) / 0.04 (block), else 0;
/// with no player vehicle, an upside-down one, or a slow one for more than 2.5 s, a law enforcer (not a Rhino with
/// seed <= 10000) closer than 10 gives up the chase. Law enforcers then `MellowOutChaseSpeed` and
/// `BackToCruisingIfNoWantedLevel`.
fn ram_or_block_close(veh: GPtr<u8>, ram: bool) {
    let d = dist_player4(veh).to_f32();
    // fcomp d; test ah, 5; jp: far < d goes on
    if lt(switch_distance_far(veh), x(d)) {
        let mut c = [0f32; 3];
        if join_car_with_road_system_goto_coors(veh, fpc(&mut c).cast(), 1, 0) == 0 {
            wr::<u8>(veh, 0x3BA, if ram { 2 } else { 4 });
            wr::<u32>(veh, 0x514, 0);
            clear_siren(veh);
        }
    } else {
        let k = kf(if ram { 0x858C28 } else { 0x858CEC });
        if ram && !fpv().is_null() && get_has_collided_with(veh, fpv()) != 0 {
            let t = rd::<u8>(veh, 0x3BB);
            if t != 7 && t != 8 {
                let slow = lt(mag(fpv().byte_add(0x44)), x(k));
                wr::<u8>(veh, 0x3BB, 3);
                wr(veh, 0x3BC, now().wrapping_add(if slow { 800 } else { 50 }));
            }
        }
        if !fpv().is_null() && lt(mag(fpv().byte_add(0x44)), x(k)) {
            let t = x(g::<f32>(0xB7CB5C).read())
                .fmul_m(kf(0x858CF0))
                .fadd(xi(i32::from(rd::<u16>(veh, 0x4DC))));
            wr(veh, 0x4DC, ftol32(t) as u16);
        } else {
            wr::<u16>(veh, 0x4DC, 0);
        }
        let stop = fpv().is_null()
            || vehicle_is_upside_down(fpv()) != 0
            || (lt(mag(fpv().byte_add(0x44)), x(k)) && rd::<u16>(veh, 0x4DC) > 0x9C4);
        if stop {
            if !law(veh) {
                return;
            }
            if (rd::<u16>(veh, 0x22) != 0x1B0 || rd::<u16>(veh, 0x20) > 0x2710) && lt(x(d), x(kf(0x85862C))) {
                give_up_chase(veh);
            }
        }
    }
    if !law(veh) {
        return;
    }
    mellow_out(veh);
    back_to_cruising(veh);
}

/// Missions 9 / 13 (go to coordinates in a straight line, plain / accurate; 0x41E3B1 / 0x41E473): closer than 3 (1
/// for 13) → temp action 0 and mission 0 (13: parking cars (+0x42C bit 2) tell their occupants to leave and clear
/// the bit). Otherwise further than the straight-line distance + 5 on frames with `(frameCounter & 7) == 0` → temp
/// action 0 and mission `8 / 12 + (JoinCarWithRoadSystemGotoCoors(veh, &destination, true, false) != 0)`.
fn goto_straight_line(veh: GPtr<u8>, m: u8) {
    let d = dist_dest(veh);
    let near = kf(if m == 9 { 0x858C74 } else { 0x858C78 });
    if lt(d, x(near)) {
        let parking = rd::<u8>(veh, 0x42C);
        wr::<u8>(veh, 0x3BB, 0);
        wr::<u8>(veh, 0x3BA, 0);
        if m == 13 && parking & 4 != 0 {
            tell(veh);
            wr(veh, 0x42C, rd::<u8>(veh, 0x42C) & 0xFB);
        }
        return;
    }
    // fild straight; fadd 5.0; fxch; fcompp; test ah, 0x41; jne: only greater goes on
    if gt(d, straight(veh).fadd_m(kf(0x858C80))) && g::<u8>(0xB7CB4C).read() & 7 == 0 {
        wr::<u8>(veh, 0x3BB, 0);
        let r = join_car_with_road_system_goto_coors(veh, veh.byte_add(0x3EC).cast(), 1, 0);
        wr::<u8>(veh, 0x3BA, u8::from(r != 0) + m - 1);
    }
}

/// The far-away target missions 15 (ram car), 17 (block car), 52 (follow car), 54 (kill ped), 61 (drive-by): no
/// target (+0x41C) → mission 0; closer than the straight-line distance byte → the close mission (16, 18, 53, 55, 60;
/// 17 also turns the siren on when the vehicle uses one).
fn target_far(veh: GPtr<u8>, m: u8) {
    let t = rd::<GPtr<u8>>(veh, 0x41C);
    if t.is_null() {
        wr::<u8>(veh, 0x3BA, 0);
        return;
    }
    if gt(straight(veh), dist_target(veh, t)) {
        let close = match m {
            15 => 0x10,
            17 => 0x12,
            52 => 0x35,
            54 => 0x37,
            _ => 0x3C,
        };
        wr::<u8>(veh, 0x3BA, close);
        if m == 17 {
            siren_on_if_used(veh);
        }
    }
}

/// The close target missions 16 (ram car), 18 (block car), 53 (follow car), 55 (kill ped), 60 (drive-by): no target →
/// mission 0 (16: mission 1). 16 first calls `BackToCruisingIfNoWantedLevel` when the target is the player's vehicle.
/// Further than `FindSwitchDistanceFar` → the far mission (15, 17, 52, 54, 61; 17 also turns the siren off and the
/// horn counter to 0) and `JoinCarWithRoadSystem`. 16 otherwise reverses (temp action 3) for 800 ms after a collision
/// with the target while slower than 0.04.
fn target_close(veh: GPtr<u8>, m: u8) {
    let t = rd::<GPtr<u8>>(veh, 0x41C);
    if t.is_null() {
        wr::<u8>(veh, 0x3BA, u8::from(m == 16));
        return;
    }
    if m == 16 && t == fpv() {
        back_to_cruising(veh);
    }
    let t = rd::<GPtr<u8>>(veh, 0x41C);
    let d = dist_target(veh, t).to_f32();
    if lt(switch_distance_far(veh), x(d)) {
        match m {
            16 => wr::<u8>(veh, 0x3BA, 0xF),
            18 => {
                let s = rd::<u8>(veh, 0x42D) & 0x7F;
                wr::<u8>(veh, 0x3BA, 0x11);
                wr(veh, 0x42D, s);
                wr::<u32>(veh, 0x514, 0);
            }
            53 => wr::<u8>(veh, 0x3BA, 0x34),
            55 => wr::<u8>(veh, 0x3BA, 0x36),
            _ => wr::<u8>(veh, 0x3BA, 0x3D),
        }
        join_car_with_road_system(veh);
    } else if m == 16 && get_has_collided_with(veh, t) != 0 && lt(mag(veh.byte_add(0x44)), x(kf(0x858CEC))) {
        set_temp(veh, 3, 800);
    }
}

/// Mission 25 (slowly drive towards the player 2, 0x41EBB9): closer than 13 → occupants out, cruise speed 0, mission
/// 0xB; then further than 70, the wanted flags +0x1E bit 2, or (law enforcers) wanted level 0, any of bits 0..2 or
/// `CCullZones::NoPolice()` → the same again.
fn drive_towards_player_2(veh: GPtr<u8>) {
    let d = dist_player4(veh);
    let df = d.to_f32();
    let stop = |veh: GPtr<u8>| {
        tell(veh);
        wr::<u8>(veh, 0x3D0, 0);
        wr::<u8>(veh, 0x3BA, 0xB);
    };
    if lt(d, x(kf(0x858CE4))) {
        stop(veh);
    }
    // fcomp 70; test ah, 0x41; je: greater stops
    if gt(x(df), x(kf(0x858CE0))) || rd::<u8>(find_player_wanted(-1), 0x1E) & 4 != 0 {
        stop(veh);
        return;
    }
    if !law(veh) {
        return;
    }
    if wanted_level() == 0 || rd::<u8>(find_player_wanted(-1), 0x1E) & 7 != 0 || cull_zones_no_police() != 0 {
        stop(veh);
    }
}

/// Mission 26 (block the player forward and back, 0x41ED1C): `d = player - veh` (2D, f32 slots), its length clamped
/// below at 0.001; on foot, or when the player's speed along `d / |d|` (`inv * dy * s.y + inv * dx * s.x`) is above
/// 0.05 → mission 5. Then `BackToCruisingIfNoWantedLevel`.
fn block_forward_and_back(veh: GPtr<u8>) {
    let p = entity_pos(veh);
    let mut c1 = [0f32; 3];
    let dx = x(fpc(&mut c1).read()).fsub_m(p.read()).to_f32();
    let p = entity_pos(veh);
    let mut c2 = [0f32; 3];
    let dyx = x(fpc(&mut c2).wrapping_index(1).read()).fsub_m(p.wrapping_index(1).read());
    let dy = dyx.to_f32();
    let len = dyx.fmul_m(dy).fadd(x(dx).fmul_m(dx)).fsqrt();
    // fld 0.001; fcomp st(1); test ah, 0x41; jne: 0.001 > len clamps
    let len = if gt(x(kf(0x858CDC)), len) { kf(0x858CDC) } else { len.to_f32() };
    let switch = fpv().is_null() || {
        let inv = x(kf(0x858624)).fdiv_m(len).to_f32();
        let s1 = find_player_speed(-1).cast::<f32>();
        let s2 = find_player_speed(-1).cast::<f32>();
        let a = x(inv).fmul_m(dy).fmul_m(s2.wrapping_index(1).read());
        let b = x(inv).fmul_m(dx).fmul_m(s1.read());
        gt(a.fadd(b), x(kf(0x858C28)))
    };
    if switch {
        wr::<u8>(veh, 0x3BA, 5);
    }
    back_to_cruising(veh);
}

/// Mission 44 (approach the player, close, 0x41F12D). Further than `FindSwitchDistanceFar`: when
/// `JoinCarWithRoadSystemGotoCoors` fails, the siren off, mission 0x2B and horn counter 0. Otherwise the cops-in-car
/// timer adds `ftol(timestep * 0.02 * 1000)` while the player's vehicle is slower than 0.05, else 0; with no player
/// vehicle, an upside-down one, or the timer above 2500 (20000 for the cop bike, model 523), a law enforcer closer
/// than 10 gives up the chase; otherwise a cop bike's driver whose primary task is not a sequence containing task
/// 701 or 1022 gets `CEventScriptCommand(3, sequence { EnterCarAsDriver(bike), GangDriveBy(player, null, 50, 50,
/// 8, false) })` and weapon 0x16. Law enforcers then `MellowOutChaseSpeed`; `BackToCruisingIfNoWantedLevel`.
fn approach_player_close(veh: GPtr<u8>) {
    let d = dist_player4(veh).to_f32();
    if lt(switch_distance_far(veh), x(d)) {
        let mut c = [0f32; 3];
        if join_car_with_road_system_goto_coors(veh, fpc(&mut c).cast(), 1, 0) == 0 {
            let s = rd::<u8>(veh, 0x42D) & 0x7F;
            wr::<u8>(veh, 0x3BA, 0x2B);
            wr(veh, 0x42D, s);
            wr::<u32>(veh, 0x514, 0);
        }
    } else {
        if !fpv().is_null() && lt(mag(fpv().byte_add(0x44)), x(kf(0x858C28))) {
            let t = x(g::<f32>(0xB7CB5C).read()).fmul_m(kf(0x858B38)).fmul_m(kf(0x858C4C));
            wr(veh, 0x4DC, rd::<u16>(veh, 0x4DC).wrapping_add(ftol32(t) as u16));
        } else {
            wr::<u16>(veh, 0x4DC, 0);
        }
        let limit = if rd::<u16>(veh, 0x22) == 0x20B { 20000.0f32 } else { 2500.0 };
        let exit = fpv().is_null()
            || vehicle_is_upside_down(fpv()) != 0
            || gt(xi(i32::from(rd::<u16>(veh, 0x4DC))), x(limit));
        if exit && law(veh) && lt(x(d), x(kf(0x85862C))) {
            give_up_chase(veh);
        } else {
            cop_bike_drive_by(veh);
        }
    }
    if law(veh) {
        mellow_out(veh);
    }
    back_to_cruising(veh);
}

/// The cop bike's drive-by order of mission 44 (0x41F34D..0x41F4B2).
fn cop_bike_drive_by(veh: GPtr<u8>) {
    if rd::<u16>(veh, 0x22) != 0x20B || rd::<GPtr<u8>>(veh, 0x460).is_null() {
        return;
    }
    let primary = rd::<GPtr<u8>>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(veh, 0x460), 0x47C), 0x10);
    if !primary.is_null()
        && task_get_type(primary) == 0xF4
        && (sequence_contains(primary, 0x2BD) != 0 || sequence_contains(primary, 0x3FE) != 0)
    {
        return;
    }
    let t = task_new(0x40);
    let seq = if t.is_null() { GPtr::NULL } else { sequence_ctor(t) };
    let t = task_new(0x50);
    let enter = if t.is_null() { GPtr::NULL } else { enter_car_as_driver_ctor(t, veh) };
    sequence_add_task(seq, enter);
    let t = task_new(0x44);
    let dby = if t.is_null() {
        GPtr::NULL
    } else {
        gang_drive_by_ctor(t, find_player_ped(-1), GPtr::NULL, 0x4248_0000, 0x32, 8, 0)
    };
    sequence_add_task(seq, dby);
    let mut ev = [0u32; 8];
    let e = GPtr::from_raw(ev.as_mut_ptr()).cast::<u8>();
    event_script_command_ctor(e, 3, seq, 0);
    let intel = rd::<GPtr<u8>>(rd::<GPtr<u8>>(veh, 0x460), 0x47C);
    event_group_add(intel.byte_add(0x68), e, 0);
    ped_set_current_weapon(rd::<GPtr<u8>>(veh, 0x460), 0x16);
    event_script_command_dtor(e.cast());
}

/// The default case (0x41F880; cruise, none and every mission without a case): law enforcers only, with wanted level
/// above 0 and police allowed in the zone. More than 10 from the player in x or y (each `|FindPlayerCoors - pos|`, the
/// sign test and the value from two calls) → cruise speed `FindPoliceCarSpeedForWantedLevel`, status 3 unless 12,
/// the police mission for the vehicle's appearance (boat: `FindPoliceBoatMissionForWantedLevel`, bike: 0x2B, else
/// `FindPoliceCarMissionForWantedLevel`), temp action 0, driving style 2. Within 10 while cruising: status 3 unless
/// 12 and the chase is given up.
fn police_default(veh: GPtr<u8>) {
    if !law(veh) || wanted_level() <= 0 || cull_zones_no_police() != 0 {
        return;
    }
    let far = |k: i32| {
        let p = entity_pos(veh);
        let mut c = [0f32; 3];
        let v = x(fpc(&mut c).wrapping_index(k).read()).fsub_m(p.wrapping_index(k).read());
        let p = entity_pos(veh);
        let mut c = [0f32; 3];
        let a = x(fpc(&mut c).wrapping_index(k).read()).fsub_m(p.wrapping_index(k).read());
        let a = if lt(v, x(0.0)) { a.fchs() } else { a };
        gt(a, x(kf(0x85862C)))
    };
    let to_physics = |veh: GPtr<u8>| {
        let s = rd::<u8>(veh, 0x36);
        if s & 0xF8 != 0x60 {
            wr(veh, 0x36, (s & 7) | 0x18);
        }
    };
    if far(0) || far(1) {
        wr(veh, 0x3D0, find_police_car_speed(veh) as u8);
        to_physics(veh);
        let m = if get_vehicle_appearance(veh) == 4 {
            find_police_boat_mission()
        } else if get_vehicle_appearance(veh) == 2 {
            0x2B
        } else {
            find_police_car_mission()
        };
        wr::<u8>(veh, 0x3BA, m);
        wr::<u8>(veh, 0x3BB, 0);
        wr::<u8>(veh, 0x3B9, 2);
    } else if rd::<u8>(veh, 0x3BA) == 1 {
        to_physics(veh);
        give_up_chase(veh);
    }
}

/// The tail of `UpdateCarAI` (0x41FADB..0x420660).
fn tail(veh: GPtr<u8>) {
    let (vx, vy) = (rd::<f32>(veh, 0x44), rd::<f32>(veh, 0x48));
    let ms2 = x(vx).fmul_m(vx).fadd(x(vy).fmul_m(vy)).to_f32();
    // fcomp 0.0025; test ah, 0x41; jne: only greater restarts the timers
    if gt(x(ms2), x(kf(0x858CD8))) {
        wr(veh, 0x3B0, now());
        wr(veh, 0x3AC, now());
    }
    let m = rd::<u8>(veh, 0x3BA);
    if rd::<u8>(veh, 0x3BB) == 0
        && !matches!(m, 0 | 0xB | 6)
        && rd::<u8>(veh, 0x3D0) != 0
        && !matches!(m, 0x1B..=0x20 | 0x35)
    {
        stuck_test(veh, m, ms2);
    }
    if rd::<u8>(veh, 0x20) & 7 == 0 {
        let sw = rd::<u32>(veh, 0x3B0);
        if now().wrapping_sub(sw) > 30000
            && g::<u32>(0xB7CB78).read().wrapping_sub(sw) <= 30000
            && rd::<u8>(veh, 0x3BA) == 1
            && should_car_stop_for_bridge(veh) == 0
        {
            switch_vehicle_to_real_physics(veh);
            wr::<u8>(veh, 0x3B9, 2);
            set_temp(veh, 3, 400);
        }
    }
    if law(veh) && matches!(rd::<u8>(veh, 0x3BA), 2 | 3) && !fpv().is_null() && get_vehicle_appearance(fpv()) == 2 {
        wr::<u8>(veh, 0x3BA, 4);
    }
    // fcomp -0.7; test ah, 5; jp: below waits 1 s
    if lt(x(rd::<f32>(rd::<GPtr<u8>>(veh, 0x14), 0x28)), x(kf(0x858CD0))) {
        set_temp(veh, 1, 1000);
    }
    if matches!(rd::<u8>(veh, 0x3BB), 0 | 0x17) && matches!(rd::<u8>(veh, 0x3BA), 2..=5 | 0x2B | 0x2C) && !fpv().is_null() {
        chase_steering(veh);
    }
    if rd::<u8>(veh, 0x42D) & 0x80 != 0 {
        let r = rand_at(0x4203AB) as u32 ^ u32::from(rd::<u16>(veh, 0x20));
        if r as u8 == 0xAD {
            wr::<u32>(veh, 0x514, 0x2D);
        }
    }
    speed_multiplier(veh);
    if law(veh) && wanted_inline() > 0 {
        player_vehicle_kind(veh);
    }
    police_gang_area_cruise_speed(veh);
}

/// The stuck test (0x41FB96): the last collision more than 0.5 s ago restarts +0x3AC; below 0.000144 for `((seed &
/// 0xF) + 40) * 500` ms with styles 0 / 4, else 1000 ms: the reverse mode cycles (reset after 10 s), a cruising
/// ambient car reverses for 0.7 s, others by mode 3 / 1.5 s, 3 / 4 s, 0xD / 2.5 s, 0xE / 2.5 s; random cars with
/// styles 0, 1, 4, 6 switch to style 2; simple status → physics; the horn.
fn stuck_test(veh: GPtr<u8>, m: u8, ms2: f32) {
    if now().wrapping_sub(rd::<u32>(veh, 0x3C)) > 500 {
        wr(veh, 0x3AC, now());
    }
    // fcomp 0.000144; test ah, 5; jp: only less
    if !lt(x(ms2), x(kf(0x858CD4))) {
        return;
    }
    let style = rd::<u8>(veh, 0x3B9);
    let interval = if style == 0 || style == 4 { ((rd::<u8>(veh, 0x20) & 0xF) as i32 + 40) * 500 } else { 1000 };
    let t = now();
    if (t.wrapping_sub(rd::<u32>(veh, 0x3AC)) as i32) <= interval {
        return;
    }
    if t < rd::<u32>(veh, 0x3C0).wrapping_add(10000) {
        wr(veh, 0x3C4, ((i32::from(rd::<u8>(veh, 0x3C4)) + 1) % 4) as u8);
    } else {
        wr::<u8>(veh, 0x3C4, 0);
    }
    wr(veh, 0x3C0, now());
    if m == 1 && rd::<u8>(veh, 0x4A4) != 2 && rd::<u8>(veh, 0x42E) & 8 == 0 {
        set_temp(veh, 3, 700);
    } else {
        match rd::<u8>(veh, 0x3C4) {
            0 => set_temp(veh, 3, 1500),
            1 => set_temp(veh, 3, 4000),
            2 => set_temp(veh, 0xD, 2500),
            3 => set_temp(veh, 0xE, 2500),
            _ => {}
        }
    }
    wr(veh, 0x3AC, now());
    if rd::<u8>(veh, 0x4A4) == 1 && matches!(style as i8, 0 | 1 | 4 | 6) {
        wr::<u8>(veh, 0x3B9, 2);
    }
    let s = rd::<u8>(veh, 0x36);
    if s & 0xF8 == 0x10 {
        wr(veh, 0x36, (s & 7) | 0x18);
    }
    play_car_horn(veh);
}

/// The chase steering of missions 2..5, 0x2B, 0x2C (0x41FE46..0x42039C) with temp action 0 / 0x17 and the player in
/// a vehicle. P = the player vehicle's forward (x, y, 0) normalised, V = this vehicle's (`GetTopDirection`, z = 0)
/// normalised. When this vehicle is slower (magnitude) than the player, the player faster than 0.1, the raw
/// matrix forwards' 2D dot product positive and the player coming at this vehicle (`(veh - player) · speed2D >
/// |speed2D| * |veh - player|2D * 0.5`): further than 12 → wait 250 ms; closer than 20, `V · P > 0.8`, the player
/// faster than 0.6 and faster than this vehicle + 0.2 → temp action 0x17 for 250 ms and the steer angle (+0x494)
/// towards `player + speed * 120`: `GetATanOfXY` of that minus the matrix forward's, brought into [-pi, pi], clamped
/// to `FindMaxSteerAngle`. Then, head-on (`V · P < -0.8`), the player faster than 0.3, closer than 45 and this
/// vehicle faster than 0.5: a handbrake turn to the side of the player (temp action 4 left of the right vector, else
/// 5) when the side distance is below 15, and the temp action timer 2 s ahead in any case.
fn chase_steering(veh: GPtr<u8>) {
    let pm = rd::<GPtr<u8>>(fpv(), 0x14);
    let mut p = [0f32; 3];
    let pp = lp(&mut p).cast::<u32>();
    pp.write(rd::<u32>(pm, 0x10));
    pp.wrapping_index(1).write(rd::<u32>(pm, 0x14));
    pp.wrapping_index(2).write(rd::<u32>(pm, 0x18));
    pp.wrapping_index(2).write(0);
    vector_normalise(lp(&mut p).cast());
    let mut v = [0f32; 3];
    placeable_get_top_direction(veh, lp(&mut v));
    v[2] = 0.0;
    vector_normalise(lp(&mut v).cast());
    let pspd = mag(find_player_speed(-1)).to_f32();
    let dot3 = |v: &[f32; 3], p: &[f32; 3]| x(v[1]).fmul_m(p[1]).fadd(x(v[0]).fmul_m(p[0])).fadd(x(v[2]).fmul_m(p[2]));
    let mut tmp = [0f32; 3];
    let mut c = [0f32; 3];
    let to_player = |tmp: &mut [f32; 3], c: &mut [f32; 3]| {
        let pos = entity_pos(veh);
        mag(vector_sub(lp(tmp).cast(), fpc(c).cast(), pos.cast()))
    };
    'steer: {
        if !lt(mag(veh.byte_add(0x44)), x(pspd)) || !gt(mag(find_player_speed(-1)), x(kf(0x858B1C))) {
            break 'steer;
        }
        let pm = rd::<GPtr<u8>>(fpv(), 0x14);
        let vm = rd::<GPtr<u8>>(veh, 0x14);
        let fdot = x(rd::<f32>(pm, 0x14)).fmul_m(rd::<f32>(vm, 0x14)).fadd(x(rd::<f32>(vm, 0x10)).fmul_m(rd::<f32>(pm, 0x10)));
        if !gt(fdot, x(0.0)) {
            break 'steer;
        }
        let pos = entity_pos(veh);
        let mut c1 = [0f32; 3];
        let dx = x(pos.read()).fsub_m(fpc(&mut c1).read()).to_f32();
        let pos = entity_pos(veh);
        let mut c2 = [0f32; 3];
        let dy = x(pos.wrapping_index(1).read()).fsub_m(fpc(&mut c2).wrapping_index(1).read()).to_f32();
        let spx = find_player_speed(-1).cast::<f32>().read();
        let spy = x(find_player_speed(-1).cast::<f32>().wrapping_index(1).read());
        let along = x(dy).fmul(spy).fadd(x(spx).fmul_m(dx));
        let speed2d = spy.fmul(spy).fadd(x(spx).fmul_m(spx)).fsqrt();
        let dist = x(dy).fmul_m(dy).fadd(x(dx).fmul_m(dx)).fsqrt();
        // fcompp; test ah, 5; jp: only (speed2d * dist * 0.5) < along goes on
        if !lt(speed2d.fmul(dist).fmul_m(kf(0x858B8C)), along) {
            break 'steer;
        }
        if gt(to_player(&mut tmp, &mut c), x(kf(0x858CCC))) {
            set_temp(veh, 1, 250);
        }
        if !lt(to_player(&mut tmp, &mut c), x(kf(0x858BA4)))
            || !gt(dot3(&v, &p), x(kf(0x858C98)))
            || !gt(mag(find_player_speed(-1)), x(kf(0x858CC8)))
        {
            break 'steer;
        }
        let own = mag(veh.byte_add(0x44)).to_f32();
        if !gt(mag(find_player_speed(-1)).fsub_m(kf(0x858CC4)), x(own)) {
            break 'steer;
        }
        set_temp(veh, 0x17, 250);
        let mut lead = [0f32; 3];
        let lead = vector_mul_scalar(lp(&mut lead).cast(), find_player_speed(-1), f32::from_bits(0x42F0_0000));
        let mut c4 = [0f32; 3];
        let mut aim = [0f32; 3];
        let aim = vector_add(lp(&mut aim).cast(), fpc(&mut c4).cast(), lead).cast::<f32>();
        let pos = entity_pos(veh);
        let ay = x(aim.wrapping_index(1).read()).fsub_m(pos.wrapping_index(1).read()).to_f32();
        let ax = x(aim.read()).fsub_m(pos.read()).to_f32();
        let a1 = X87::from_f64_exact(get_atan_of_xy(ax, ay)).to_f32();
        let vm = rd::<GPtr<u8>>(veh, 0x14);
        let a2 = X87::from_f64_exact(get_atan_of_xy(rd::<f32>(vm, 0x10), rd::<f32>(vm, 0x14)));
        // fsubr a1; then brought into [-pi, pi] (fst after each loop, fstp only when the second loop ran)
        let mut a = a2.fsubr_m(a1);
        let mut angle = a.to_f32();
        if lt(a, x(kf(0x858CC0))) {
            loop {
                a = a.fadd_m(kf(0x858CBC));
                if !lt(a, x(kf(0x858CC0))) {
                    break;
                }
            }
            angle = a.to_f32();
        }
        if gt(a, x(kf(0x858CB8))) {
            loop {
                a = a.fsub_m(kf(0x858CBC));
                if !gt(a, x(kf(0x858CB8))) {
                    break;
                }
            }
            angle = a.to_f32();
        }
        let max = find_max_steer_angle(veh);
        let maxf = max.to_f32();
        let minf = max.fchs().to_f32();
        let mut s = x(angle);
        if lt(s, x(minf)) {
            s = x(minf);
        }
        if gt(s, x(maxf)) {
            s = x(maxf);
        }
        wr(veh, 0x494, s.to_f32());
    }
    if !lt(dot3(&v, &p), x(kf(0x858C9C)))
        || !gt(mag(find_player_speed(-1)), x(kf(0x858C24)))
        || !lt(to_player(&mut tmp, &mut c), x(kf(0x858CB4)))
    {
        return;
    }
    let pos = entity_pos(veh);
    let mut r = [0f32; 3];
    let right = get_right_direction(veh, lp(&mut r));
    let mut c3 = [0f32; 3];
    let mut diff = [0f32; 3];
    let d = vector_sub(lp(&mut diff).cast(), fpc(&mut c3).cast(), pos.cast());
    let rdot = dot_product(d, right.cast()).to_f32();
    if !gt(mag(veh.byte_add(0x44)), x(kf(0x858B8C))) {
        return;
    }
    if lt(x(rdot).fabs(), x(kf(0x858B48))) {
        wr::<u8>(veh, 0x3BB, if lt(x(rdot), x(0.0)) { 4 } else { 5 });
    }
    wr(veh, 0x3BC, now().wrapping_add(2000));
}

/// The speed multiplier (+0x3D4) moves towards the node speed (cruise) or 1.0 by `timestep * 0.01`, snapping to the
/// target when closer (0x4203C1).
fn speed_multiplier(veh: GPtr<u8>) {
    let target = if rd::<u8>(veh, 0x3BA) == 1 {
        X87::from_f64_exact(find_speed_multiplier_with_speed_from_nodes(u32::from(rd::<u8>(veh, 0x3D1))))
    } else {
        x(kf(0x858624))
    };
    let step = x(g::<f32>(0xB7CB5C).read()).fmul_m(kf(0x858C58)).to_f32();
    let mult = rd::<f32>(veh, 0x3D4);
    let mut delta = x(mult).fsub(target);
    if lt(delta, x(0.0)) {
        delta = delta.fchs();
    }
    let new = if lt(delta, x(step)) {
        target
    } else if lt(target, x(mult)) {
        x(mult).fsub_m(step)
    } else {
        x(step).fadd_m(mult)
    };
    wr(veh, 0x3D4, new.to_f32());
}

/// Law enforcers with wanted level > 0 (0x420445): with the player in a car or bike, a police boat waits 1 s; in a
/// boat, a police car or bike waits 1 s. On foot: swimming (`CPedIntelligence::GetTaskSwim`) as in a boat, else as
/// in a car.
fn player_vehicle_kind(veh: GPtr<u8>) {
    let land = !fpv().is_null() && {
        get_vehicle_appearance(fpv()) == 1 || get_vehicle_appearance(fpv()) == 2
    };
    let water = if fpv().is_null() {
        !get_task_swim(rd::<GPtr<u8>>(find_player_ped(-1), 0x47C)).is_null()
    } else {
        get_vehicle_appearance(fpv()) == 4
    };
    let on_foot_dry = fpv().is_null() && !water;
    if land || on_foot_dry {
        if get_vehicle_appearance(veh) == 4 {
            set_temp(veh, 1, 1000);
        }
    } else if water && (get_vehicle_appearance(veh) == 1 || get_vehicle_appearance(veh) == 2) {
        set_temp(veh, 1, 1000);
    }
}

/// The law-enforcer tail of `UpdateCarAI` (0x420553..0x42065A): on frames with `(frameCounter & 0xF) == 0xE`, a
/// cruising (mission 1) law-enforcer automobile (+0x594 == 0) with status 2 or 3 in a gang area (`CPopCycle::
/// m_bCurrentZoneIsGangArea` 0xC0BC5C) counts the peds of types 7..=17 (gangs, dealers) closer than 10 (the pool
/// walked from the last slot down) and cruises at `max(10 - count, 1)`.
fn police_gang_area_cruise_speed(veh: GPtr<u8>) {
    if g::<u32>(0xB7CB4C).read() & 0xF != 0xE
        || !law(veh)
        || !matches!(rd::<u8>(veh, 0x36) >> 3, 2 | 3)
        || rd::<u8>(veh, 0x3BA) != 1
        || rd::<u32>(veh, 0x594) != 0
        || g::<u8>(0xC0BC5C).read() == 0
    {
        return;
    }
    let pool = g::<GPtr<u8>>(0xB74490).read();
    let mut i = rd::<i32>(pool, 8);
    let mut count = 0i32;
    let mut tmp = [0f32; 3];
    while i != 0 {
        let flag = rd::<i8>(rd::<GPtr<u8>>(pool, 4), (i - 1) as isize);
        i -= 1;
        if flag < 0 {
            continue;
        }
        let ped = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0x7C4);
        if ped.is_null() || !(7..=0x11).contains(&rd::<i32>(ped, 0x598)) {
            continue;
        }
        let d = vector_sub(lp(&mut tmp).cast(), entity_pos(veh).cast(), entity_pos(ped).cast());
        if lt(mag(d), x(kf(0x85862C))) {
            count += 1;
        }
    }
    wr::<u8>(veh, 0x3D0, (10 - count).max(1) as u8);
}

/// `CCarAI::TellOccupantsToLeaveCar(CVehicle*)` (stub 0x41C760 → 0x1569A30): the driver (+0x460) and every passenger
/// (+0x464, count +0x488 re-read each turn): a running `CTaskComplexCopInCar` (type 0x452, `FindTaskByType`) gets
/// +0x30 bit 1; otherwise a `CTaskComplexLeaveCar(veh, 0, 0, true, false)` (0x34 bytes) is set as the primary task
/// (slot 3). Each passenger given the task draws a `rand()` whose result is unused (0x1569B63). The driver's task
/// size and its `true` come through no-CD code ([0x1560550] - [0x609143] = 0x34; the bool from the decoder at
/// 0xCB73E0, taken as the passengers' constant 1, as gta-reversed has it).
#[hook(addr = 0x41C760, conv = cdecl, name = "CCarAI::TellOccupantsToLeaveCar", expect = "E9 CB D2 14 01",
       unverified = "frontier2-3", route = "L2", hazards = "heap-writes", recs = "driving")]
pub fn tell_occupants_to_leave_car(veh: GPtr<u8>) {
    let order = |ped: GPtr<u8>| -> bool {
        let cop = intel_find_task_by_type(rd::<GPtr<u8>>(ped, 0x47C), 0x452);
        if !cop.is_null() {
            wr(cop, 0x30, rd::<u8>(cop, 0x30) | 2);
            return false;
        }
        let t = task_new(0x34);
        let t = if t.is_null() { GPtr::NULL } else { leave_car_ctor(t, veh, 0, 0, 1, 0) };
        tm_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 3, 0);
        true
    };
    let drv = rd::<GPtr<u8>>(veh, 0x460);
    if !drv.is_null() {
        order(drv);
    }
    let mut i = 0i32;
    while i < i32::from(rd::<u8>(veh, 0x488)) {
        let p = rd::<GPtr<u8>>(veh, 0x464 + 4 * i as isize);
        if !p.is_null() && order(p) {
            rand_at(0x1569B68);
        }
        i += 1;
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_UPDATE_CAR_AI, HOOK_TELL_OCCUPANTS_TO_LEAVE_CAR];
