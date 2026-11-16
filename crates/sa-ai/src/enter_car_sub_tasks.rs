//! `CTaskComplexEnterCar`'s sub-task logic (batch `std14-ai-1`): `CreateFirstSubTask` 0x643A60, `ControlSubTask`
//! 0x63A890, `CreateSubTask` 0x63E040, `CreateNextSubTask` 0x63E990, `CreateNextSubTask_AfterSimpleCarAlign`
//! 0x63F970, `ComputeTargetDoorOppositeToFlag` 0x63AB70 (an inline helper here) and `CTaskComplexEnterCarAsDriver`'s destructor 0x640330.
//! Ported from the 1.0 US bytes (gta-reversed as a reading aid; it differs in several branches, the bytes are
//! followed). Unverified (docs/verify/HOWTO.md).
//!
//! Layout (`CTaskComplexEnterCar`, 0x50 bytes): +8 sub-task, +0xC vehicle (a registered reference), +0x10 flags (0x1 as
//! driver, 0x2 quit after opening the door, 0x4 quit after dragging the ped out, 0x8 carry on after falling off),
//! +0x14 target door, +0x18 the door opposite to the flag, +0x1C target seat, +0x20 time, +0x24 move state, +0x28
//! number getting in set, +0x2C target door position, +0x38 the line-up utility, +0x3C aborting, +0x40 the dragged
//! ped (a registered reference), +0x44 door flags set, +0x48 cruise speed, +0x4C start time.
#![allow(clippy::unreadable_literal)]

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

use crate::bindings::dragged_out_car_dtor as event_dragged_out_car_dtor;
use crate::bindings::event_cop_car_being_stolen_ctor;
use crate::bindings::event_cop_car_being_stolen_dtor;
use crate::bindings::event_leader_exited_car_as_driver_ctor as event_leader_exited_car_ctor;
use crate::bindings::is_player_to_quit_car_enter;
use crate::bindings::is_vehicle_healthy;
use crate::bindings::leave_group_ctor as t_none;
use crate::bindings::vehicle_clear_getting_in_flags;
use sa_pop::bindings::vehicle_is_driver;
use sa_core::bindings::car_set_ped_out_ctor as t_set_ped_out;
use sa_world::bindings::entity_register_reference;
use sa_world::bindings::find_player_ped;
use sa_core::bindings::hud_set_help_message;
use sa_core::bindings::ped_is_alive;
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::compute_target_door_to_exit;
use sa_world::bindings::enter_car_dtor;
use crate::bindings::event_group_add;
use sa_world::bindings::event_group_event_ctor;
use sa_world::bindings::event_group_event_dtor;
use sa_world::bindings::event_leader_entered_car_as_driver_ctor;
use sa_world::bindings::get_event_global_group;
use sa_world::bindings::get_nearest_car_door;
use crate::bindings::group_intelligence_add_event;
use sa_world::bindings::intel_is_friendly_with as intelligence_is_friendly_with;
use sa_world::bindings::is_law_enforcement_vehicle as vehicle_is_law_enforcement;
use sa_world::bindings::leave_car_ctor as t_leave_car_5;
use sa_world::bindings::membership_is_leader as group_membership_is_leader;
use sa_world::bindings::pad_get_accelerate;
use sa_world::bindings::pad_get_brake;
use sa_world::bindings::ped_groups_are_in_same_group;
use crate::bindings::ped_groups_get_peds_group;
use sa_world::bindings::player_set_real_move_anim as player_ped_set_real_move_anim;
use sa_world::bindings::stand_still_ctor as t_stand_still;
use crate::bindings::car_drive_timed_ctor as t_drive_timed;
use crate::bindings::car_enter_exit_car_has_door_to_close as car_has_door_to_close;
use crate::bindings::car_enter_exit_car_has_door_to_open as car_has_door_to_open;
use crate::bindings::car_enter_exit_car_has_openable_door as car_has_openable_door;
use crate::bindings::car_set_ped_out_position_ped_out_of_collision as position_ped_out_of_collision;
use crate::bindings::car_wait_for_door_ctor as t_wait_for_door;
use crate::bindings::close_door_from_outside_ctor as t_close_door_outside;
use crate::bindings::create_sub_task_o_63e040 as create_sub_task_o;
use crate::bindings::damage_get_door_status_component as damage_get_door_status;
use crate::bindings::damage_manager_set_door_status as damage_set_door_status;
use crate::bindings::dragged_out_car_ctor as event_dragged_out_car_ctor;
use crate::bindings::fall_and_get_up_ctor as t_fall_and_get_up;
use sa_core::bindings::text_get;
use sa_world::bindings::car_enter_exit_compute_passenger_index_from_car_door as compute_passenger_index_from_car_door;
use sa_world::bindings::ik_look_at as ped_chat_say;
use sa_world::bindings::ped_set_move_state;
use sa_world::bindings::set_ped_in_as_driver_ctor as t_set_in_driver;
use sa_world::bindings::set_ped_in_as_passenger_ctor as t_set_in_passenger;
use crate::bindings::car_shuffle_ctor as t_shuffle;
use crate::bindings::t_wait_until_ped_out;
orig! {
    /// `CTaskComplex::CreateFirstSubTask` (vtable +0x24)
    fn v_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CTaskComplexEnterCar::CreateNextSubTask_AfterSimpleCarAlign` (vtable +0x2C)
    fn v_after_simple_car_align(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x2C => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, pri: i32, ev: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CPed` vtable +0x5C (`SetMoveAnim`)
    fn v_ped_set_move_anim(ped: GPtr<u8>) = thiscall vslot 0x5C => unported;
    /// `CVehicle` vtable +0x94 (door missing, door argument)
    fn v_veh_slot_94(v: GPtr<u8>, door: u32) -> u8 = thiscall vslot 0x94 => unported;
    fn player_info_set_last_target_vehicle(pi: GPtr<u8>, v: GPtr<u8>) = thiscall 0x56DA80 => unported;
    fn is_ped_healthy(p: GPtr<u8>) -> u8 = cdecl 0x64EEE0 => unported;
    fn compute_slow_jacked_ped(v: GPtr<u8>, door: i32) -> GPtr<u8> = cdecl 0x64F070 => unported;
    fn is_car_slow_jack_required(v: GPtr<u8>, door: i32, ped: GPtr<u8>) -> u8 = cdecl 0x64EF70 => unported;
    fn is_car_quick_jack_possible(v: GPtr<u8>, door: i32, ped: GPtr<u8>) -> u8 = cdecl 0x64EF00 => unported;
    fn car_has_partially_open_door(v: GPtr<u8>, door: i32) -> u8 = cdecl 0x64EE70 => unported;
    fn is_car_door_ready(v: GPtr<u8>, door: i32) -> u8 = cdecl 0x64ED90 => unported;
    fn is_car_door_in_use(v: GPtr<u8>, a: i32, b: i32) -> u8 = cdecl 0x64EC90 => unported;
    fn make_undragged_passenger_peds_leave_car(v: GPtr<u8>, dragged: GPtr<u8>, ped: GPtr<u8>) = cdecl 0x64F540 => unported;
    fn set_vehicle_flags(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x63AB90 => unported;
    fn prepare_vehicle_for_ped_enter(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x63AC10 => unported;
    fn prepare_ped_for_vehicle_enter(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x63AC80 => unported;
    fn create_line_up_utility(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x63ACC0 => unported;
    fn intelligence_add_task_primary_maybe_in_group(i: GPtr<u8>, t: GPtr<u8>, b: u32) = thiscall 0x600E20 => unported;
    fn ped_groups_is_group_leader(ped: GPtr<u8>) -> u8 = cdecl 0x5F7E40 => unported;
    fn event_leader_quit_entering_car_ctor(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x63A110 => unported;
    fn t_enter_boat_as_driver(t: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x63B5E0 => unported;
    fn t_leave_car_3(t: GPtr<u8>, v: GPtr<u8>, door: u32, delay: u32) -> GPtr<u8> = thiscall 0x62F1A0 => unported;
    fn t_go_to_car_door(t: GPtr<u8>, v: GPtr<u8>, ms: u32, driver: u32, seat: u32, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x645780 => unported;
    fn t_car_align(t: GPtr<u8>, v: GPtr<u8>, pos: GPtr<u8>, door: u32, lu: u32) -> GPtr<u8> = thiscall 0x645B40 => unported;
    fn t_open_door_outside(t: GPtr<u8>, v: GPtr<u8>, door: u32, lu: u32, quit: u32) -> GPtr<u8> = thiscall 0x645E50 => unported;
    fn t_veh_door_lu(t: GPtr<u8>, v: GPtr<u8>, door: u32, lu: u32) -> GPtr<u8> = thiscall 0x646070 => unported;
    fn t_bike_pick_up(t: GPtr<u8>, v: GPtr<u8>, door: u32, lu: u32) -> GPtr<u8> = thiscall 0x6461B0 => unported;
    fn t_close_door_inside(t: GPtr<u8>, v: GPtr<u8>, door: u32, lu: u32) -> GPtr<u8> = thiscall 0x646300 => unported;
    fn t_get_in(t: GPtr<u8>, v: GPtr<u8>, door: u32, lu: u32) -> GPtr<u8> = thiscall 0x646690 => unported;
    fn t_slow_drag_ped_out(t: GPtr<u8>, v: GPtr<u8>, door: u32, lu: u32, quit: u32) -> GPtr<u8> = thiscall 0x647FE0 => unported;
    fn t_slow_be_dragged_out(t: GPtr<u8>, v: GPtr<u8>, door: u32) -> GPtr<u8> = thiscall 0x648620 => unported;
    fn t_0x648b90(t: GPtr<u8>, v: GPtr<u8>, door: u32, time: u32, ped: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x648B90 => unported;
}

const FINISHED: u32 = 0x516;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `CPad::Pads[0]`
const PAD0: usize = 0xB73458;

fn veh(t: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(t, 0xC)
}
fn flags(t: GPtr<u8>) -> u8 {
    rd::<u8>(t, 0x10)
}
fn door(t: GPtr<u8>) -> i32 {
    rd::<i32>(t, 0x14)
}
fn lu(t: GPtr<u8>) -> u32 {
    rd::<u32>(t, 0x38)
}
use sa_core::field::gf as kf;
use sa_world::world::cworld::in_vehicle;
fn sub(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> {
    create_sub_task_o(t, ty, ped)
}
/// `as driver` without either quit flag (`test al, 1; je; test al, 6; jne`)
fn driver_no_quit(t: GPtr<u8>) -> bool {
    flags(t) & 1 != 0 && flags(t) & 6 == 0
}

/// A task pool slot of `size`, constructed by `ctor` when the pool had one (null otherwise, as MSVC's `new`).
fn new_task(size: u32, ctor: impl FnOnce(GPtr<u8>) -> GPtr<u8>) -> GPtr<u8> {
    let t = task_new(size);
    if t.is_null() { t } else { ctor(t) }
}

/// The leader event the original builds inline: when `ped` leads its group, an event of `size` bytes from the event
/// pool (`ctor`; null without a slot) wrapped in a stack `CEventGroupEvent(ped, event)` is added to the group's
/// intelligence and the wrapper destroyed.
fn leader_event(ped: GPtr<u8>, size: u32, ctor: impl FnOnce(GPtr<u8>) -> GPtr<u8>) {
    let grp = GPtr::from_addr(ped_groups_get_peds_group(ped.addr() as u32) as usize);
    if grp.is_null() || group_membership_is_leader(grp.byte_add(8), ped) == 0 {
        return;
    }
    let e = event_new(size);
    let e = if e.is_null() { e } else { ctor(e) };
    let mut local = [0u32; 16];
    let lp = GPtr::from_raw(local.as_mut_ptr()).cast::<u8>();
    event_group_event_ctor(lp, ped, e);
    group_intelligence_add_event(grp.byte_add(0x30), lp);
    event_group_event_dtor(lp);
}

/// The door-status bump the original inlines: automobiles (+0x590 == 0) whose door status is 0 or 2 get status + 1.
fn bump_door_status(v: GPtr<u8>, d: i32) {
    if rd::<u32>(v, 0x590) != 0 {
        return;
    }
    let dm = v.byte_add(0x5A0);
    let st = damage_get_door_status(dm, d as u32);
    if st == 0 || damage_get_door_status(dm, d as u32) == 2 {
        let s = damage_get_door_status(dm, d as u32);
        damage_set_door_status(dm, d as u32, s.wrapping_add(1));
    }
}

/// The 2D speed test of the open-door branches: `sqrt(vy² + vx²)` above 0.1 (not a player) or 0.2 (player).
fn too_fast(ped: GPtr<u8>, v: GPtr<u8>) -> bool {
    let speed = || {
        let (vx, vy) = (rd::<f32>(v, 0x44), rd::<f32>(v, 0x48));
        x(vx).fmul_m(vx).fadd(x(vy).fmul_m(vy)).fsqrt()
    };
    if ped_is_player(ped) == 0 && !speed().fcom(x(kf(0x86F104))).test_any(0x41) {
        return true;
    }
    ped_is_player(ped) != 0 && !speed().fcom(x(kf(0x86F108))).test_any(0x41)
}

/// The cruise speed write: `veh +0x3D0 = (u8)_ftol(+0x48)` unless +0x48 < 0.
fn set_cruise_speed(t: GPtr<u8>, v: GPtr<u8>) {
    let cs = x(rd::<f32>(t, 0x48));
    if !cs.fcom(x(0.0)).c0() {
        wr::<u8>(v, 0x3D0, ftol32(cs) as u8);
    }
}

/// `CTaskComplexEnterCarAsDriver::~CTaskComplexEnterCarAsDriver` (0x640330): vtable 0x86EAAC, then (tail jump) the
/// `CTaskComplexEnterCar` destructor.
#[hook(addr = 0x640330, conv = thiscall, name = "FUN_00640330", expect = "C7 01 AC EA 86 00 E9",
       unverified = "std14-ai-1", route = "L2", hazards = "heap-writes", recs = "enter-car")]
pub fn enter_car_as_driver_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86EAAC);
    enter_car_dtor(this);
}

/// `CTaskComplexEnterCar::ComputeTargetDoorOppositeToFlag()` (0x63AB70, not in the catalogue, so not hooked; called
/// directly): 10 for a driver at door 8, else 0.
pub fn compute_target_door_opposite_to_flag(this: GPtr<u8>) -> i32 {
    if flags(this) & 1 != 0 && door(this) == 8 {
        10
    } else {
        0
    }
}

/// `CTaskComplexEnterCar::CreateFirstSubTask(CPed*)` (0x643A60). Start time = now. Finished without a vehicle, with
/// a fire, an unhealthy vehicle or ped, a passenger entry into a car without passenger seats, or a derailed train.
/// A driver entry (no quit flags) by a group leader sends the group `CEventLeaderEnteredCarAsDriver`. In a vehicle:
/// drive timed (0x2CF) in this one, leave car (0x2C0) otherwise; boats: 0x2CB; else go to the door (0x320), whose
/// target door / position / in-water flag are set by `GetNearestCarDoor` for a submerged, not standing ped and the
/// seaplanes 0x1CC / 0x21B / 0x1BF / 0x1A1.
#[hook(addr = 0x643A60, conv = thiscall, name = "CTaskComplexEnterCar::CreateFirstSubTask", expect = "6A FF 64 A1 00 00 00 00",
       unverified = "std14-ai-1", route = "L2", hazards = "heap-writes", recs = "enter-car")]
pub fn create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0x4C, g::<u32>(TIME_MS).read());
    let v = veh(this);
    if v.is_null()
        || rd::<u32>(v, 0x490) != 0
        || is_vehicle_healthy(v) == 0
        || is_ped_healthy(ped) == 0
        || (flags(this) & 1 == 0 && rd::<u8>(veh(this), 0x488) == 0)
        || (rd::<u32>(veh(this), 0x590) == 6 && rd::<u8>(veh(this), 0x5B9) & 1 != 0)
    {
        return sub(this, FINISHED, ped);
    }
    let grp = GPtr::from_addr(ped_groups_get_peds_group(ped.addr() as u32) as usize);
    if driver_no_quit(this) && !grp.is_null() && group_membership_is_leader(grp.byte_add(8), ped) != 0 {
        let e = event_new(0x18);
        let e = if e.is_null() {
            e
        } else {
            event_leader_entered_car_as_driver_ctor(e, veh(this))
        };
        let mut local = [0u32; 16];
        let lp = GPtr::from_raw(local.as_mut_ptr()).cast::<u8>();
        event_group_event_ctor(lp, ped, e);
        group_intelligence_add_event(grp.byte_add(0x30), lp);
        event_group_event_dtor(lp);
    }
    if in_vehicle(ped) {
        let ty = if rd::<GPtr<u8>>(ped, 0x58C) == veh(this) {
            0x2CF
        } else {
            0x2C0
        };
        return sub(this, ty, ped);
    }
    if rd::<u32>(veh(this), 0x590) == 5 {
        return sub(this, 0x2CB, ped);
    }
    let t = sub(this, 0x320, ped);
    if !t.is_null() && rd::<u32>(ped, 0x40) & 0x100 != 0 && rd::<u8>(ped, 0x46C) & 1 == 0 {
        let v = veh(this);
        if matches!(rd::<u16>(v, 0x22), 0x1CC | 0x21B | 0x1BF | 0x1A1)
            && get_nearest_car_door(ped, v, this.byte_add(0x2C), this.byte_add(0x14)) != 0
        {
            wr::<u32>(t, 0x34, rd::<u32>(this, 0x14));
            for k in 0..3 {
                wr::<u32>(t, 0x38 + 4 * k, rd::<u32>(this, 0x2C + 4 * k));
            }
            wr::<u8>(t, 0x48, 1);
        }
    }
    t
}

/// `CTaskComplexEnterCar::ControlSubTask(CPed*)` (0x63A890). A player (no drag-out quit) with a vehicle who wants to
/// quit (`IsPlayerToQuitCarEnter`): a leading driver entry (not while getting in, 0x327) sends
/// `CEventLeaderQuitEnteringCarAsDriver`; a slow-jacked ped flagged +0x46C & 0x20000000 sets quit-after-opening
/// (0x2), anything else quit-after-dragging (0x4). Without a healthy vehicle and ped: aborting, the sub-task asked
/// to abort (priority 0). Otherwise a driver entry or a player, during 0x320 / 0xC8 / 0x2CB / 0x32A, quits (null
/// result, with the leader event) when the player wants to. The go-to-door sub-task's position is copied to +0x2C
/// when it has a target door.
#[hook(addr = 0x63A890, conv = thiscall, name = "CTaskComplexEnterCar::ControlSubTask", expect = "6A FF 68 D6 13 84 00 64",
       unverified = "std14-ai-1", route = "L2", hazards = "heap-writes", recs = "enter-car")]
pub fn control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let mut res = rd::<GPtr<u8>>(this, 8);
    if ped_is_player(ped) != 0
        && flags(this) & 4 == 0
        && !veh(this).is_null()
        && is_player_to_quit_car_enter(ped, veh(this), rd::<u32>(this, 0x4C), rd::<GPtr<u8>>(this, 8)) != 0
    {
        if driver_no_quit(this)
            && ped_groups_is_group_leader(ped) != 0
            && v_task_type(rd::<GPtr<u8>>(this, 8)) != 0x327
        {
            leader_event(ped, 0xC, event_leader_quit_entering_car_ctor);
        }
        let d = door(this);
        let f = if d > 0 && {
            let j = compute_slow_jacked_ped(veh(this), d);
            !j.is_null() && rd::<u32>(j, 0x46C) & 0x2000_0000 != 0
        } {
            2
        } else {
            4
        };
        wr::<u8>(this, 0x10, flags(this) | f);
    }
    let v = veh(this);
    if v.is_null() || is_vehicle_healthy(v) == 0 || is_ped_healthy(ped) == 0 {
        wr::<u8>(this, 0x3C, 1);
        v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 0, GPtr::NULL);
    } else if (flags(this) & 1 != 0 || ped_is_player(ped) != 0)
        && matches!(v_task_type(rd::<GPtr<u8>>(this, 8)), 0x320 | 0xC8 | 0x2CB | 0x32A)
        && ped_is_player(ped) != 0
        && is_player_to_quit_car_enter(ped, veh(this), rd::<u32>(this, 0x4C), rd::<GPtr<u8>>(this, 8)) != 0
    {
        if driver_no_quit(this) && ped_groups_is_group_leader(ped) != 0 {
            leader_event(ped, 0xC, event_leader_quit_entering_car_ctor);
        }
        res = GPtr::NULL;
    }
    let st = rd::<GPtr<u8>>(this, 8);
    if v_task_type(st) == 0x320 && rd::<u32>(st, 0x34) != 0 {
        for k in 0..3 {
            wr::<u32>(this, 0x2C + 4 * k, rd::<u32>(st, 0x38 + 4 * k));
        }
    }
    res
}

/// `CTaskComplexEnterCar::CreateSubTask(int type, CPed*)` (0x63E040, `ret 8`): the sub-task of `type` from the task
/// pool (null for unknown types, 809 and a full pool), with the original's side effects for 0xD0 (fall and get up:
/// the player's last target vehicle, collision back on, the ped placed out of the car's collision, the getting-in
/// count and flags given back, the door status bumped when the door is not missing) and 0x516 (finished: collision
/// and placement, `CEventCopCarBeingStolen` for a player stealing a law car, `CEventLeaderQuitEnteringCarAsDriver`,
/// the cruise speed given back), and the one-time `JCK_HLP` help text for a player's slow drag-out.
#[hook(addr = 0x63E040, conv = thiscall, name = "CTaskComplexEnterCar::CreateSubTask", expect = "6A FF 68 9D 1D 84 00 64",
       unverified = "std14-ai-1", route = "L2", hazards = "heap-writes", recs = "enter-car")]
pub fn create_sub_task(this: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> {
    let v = veh(this);
    let d = door(this) as u32;
    match ty {
        0xCB => new_task(0x20, |t| t_stand_still(t, 0, 0, 0, f32::from_bits(0x4100_0000))),
        0xC8 => new_task(8, t_none),
        0xD0 => {
            let anim = if door(this) == 10 || door(this) == 11 {
                0x1B
            } else {
                0x19
            };
            let t = new_task(0x18, |t| t_fall_and_get_up(t, anim, 0, 2000));
            if find_player_ped(-1) == ped {
                let i = u32::from(g::<u8>(0xB7CD74).read());
                player_info_set_last_target_vehicle(
                    GPtr::from_addr((0xB7CD98 + 0x190 * i) as usize),
                    veh(this),
                );
            }
            if !ped.is_null() && !in_vehicle(ped) {
                wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) | 1);
            }
            if !veh(this).is_null() {
                position_ped_out_of_collision(ped, veh(this), door(this) as u32);
            }
            let v = veh(this);
            if v.is_null() {
                return t;
            }
            wr::<u8>(v, 0x485, rd::<u8>(v, 0x485).wrapping_sub(rd::<u8>(this, 0x28)));
            wr::<u8>(this, 0x28, 0);
            vehicle_clear_getting_in_flags(veh(this), u32::from(rd::<u8>(this, 0x44)));
            wr::<u8>(this, 0x44, 0);
            if v_veh_slot_94(veh(this), door(this) as u32) == 0 {
                bump_door_status(veh(this), door(this));
            }
            t
        }
        0x2C0 => new_task(0x34, |t| t_leave_car_5(t, rd::<GPtr<u8>>(ped, 0x58C), 0, 0, 1, 0)),
        0x2CB => new_task(0x10, |t| t_enter_boat_as_driver(t, v)),
        0x2CF => new_task(0x70, |t| t_drive_timed(t, v, -1i32 as u32)),
        0x320 => new_task(0x50, |t| {
            let k = |a: usize| g::<u32>(a).read();
            t_go_to_car_door(
                t,
                v,
                rd::<u32>(this, 0x24),
                u32::from(flags(this) & 1),
                rd::<u32>(this, 0x1C),
                k(0x86EC30),
                k(0x86EC34),
                k(0x86EC38),
                k(0x86EC3C),
            )
        }),
        0x321 => new_task(0x28, |t| t_car_align(t, v, this.byte_add(0x2C), d, lu(this))),
        0x322 => new_task(0x24, |t| {
            t_open_door_outside(t, v, d, lu(this), u32::from((flags(this) >> 1) & 1))
        }),
        0x323 => new_task(0x1C, |t| t_veh_door_lu(t, v, d, lu(this))),
        0x324 => new_task(0x1C, |t| t_bike_pick_up(t, v, d, lu(this))),
        0x325 => new_task(0x1C, |t| t_close_door_inside(t, v, d, lu(this))),
        0x326 => new_task(0x1C, |t| t_close_door_outside(t, v, d, lu(this))),
        0x327 => new_task(0x24, |t| t_get_in(t, v, d, lu(this))),
        0x328 => new_task(0x1C, |t| t_shuffle(t, v, d, lu(this))),
        0x32A => new_task(0x14, |t| t_wait_for_door(t, v, d, rd::<u32>(this, 0x18))),
        0x32B => new_task(0x20, |t| t_set_in_passenger(t, v, d, GPtr::from_addr(lu(this) as usize))),
        0x32C => new_task(0x1C, |t| t_set_in_driver(t, v, GPtr::from_addr(lu(this) as usize))),
        0x330 => new_task(0x18, |t| t_set_ped_out(t, v, d, 1)),
        0x334 => {
            if !ped.is_null() && ped_is_player(ped) != 0 && g::<u8>(0xC0BC15).read() == 0 {
                g::<u8>(0xC0BC15).write(1);
                let s = GPtr::<u8>::addr(text_get(GPtr::from_addr(0xC1B340), GPtr::from_addr(0x86EA90))) as u32;
                hud_set_help_message(GPtr::from_addr(s as usize), 0, 0, 0);
            }
            new_task(0x20, |t| {
                t_slow_drag_ped_out(t, v, d, lu(this), u32::from((flags(this) >> 2) & 1))
            })
        }
        0x33D => {
            let m = rd::<GPtr<u8>>(v, 0x14);
            let mut r = [rd::<f32>(m, 0), rd::<f32>(m, 4), rd::<f32>(m, 8)];
            if (10..=11).contains(&door(this)) {
                let k = kf(0x858C1C);
                for c in &mut r {
                    *c = x(*c).fmul_m(k).to_f32();
                }
            }
            new_task(0x1C, |t| {
                t_wait_until_ped_out(t, GPtr::<u8>::from_addr(rd::<u32>(this, 0x40) as usize), GPtr::from_raw(r.as_mut_ptr()).cast())
            })
        }
        FINISHED => {
            finished_side_effects(this, ped);
            GPtr::NULL
        }
        _ => GPtr::NULL,
    }
}

fn finished_side_effects(this: GPtr<u8>, ped: GPtr<u8>) {
    if ped.is_null() {
        return;
    }
    if !in_vehicle(ped) && rd::<u8>(ped, 0x1C) & 1 == 0 {
        wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) | 1);
        if !veh(this).is_null() && door(this) != 0 {
            position_ped_out_of_collision(ped, veh(this), door(this) as u32);
        }
    }
    if ped_is_player(ped) != 0
        && !veh(this).is_null()
        && vehicle_is_law_enforcement(veh(this)) != 0
        && driver_no_quit(this)
        && rd::<u8>(this, 0x44) != 0
    {
        let mut local = [0u32; 16];
        let lp = GPtr::from_raw(local.as_mut_ptr()).cast::<u8>();
        event_cop_car_being_stolen_ctor(lp, ped, veh(this));
        event_group_add(get_event_global_group(), lp, 0);
        event_cop_car_being_stolen_dtor(lp);
    }
    if !in_vehicle(ped) && driver_no_quit(this) {
        leader_event(ped, 0xC, event_leader_quit_entering_car_ctor);
    }
    if ped_is_player(ped) != 0 {
        let v = veh(this);
        if !in_vehicle(ped) && !v.is_null() && rd::<u32>(v, 0x460) != 0 && flags(this) & 2 == 0 {
            set_cruise_speed(this, v);
        }
        return;
    }
    let v = veh(this);
    if in_vehicle(ped) && !v.is_null() {
        set_cruise_speed(this, v);
    }
}

/// The original's "open the car door, then align or not" step (0x63ECE0 / 0x63F264): `SetVehicleFlags`,
/// `PrepareVehicleForPedEnter`, `PreparePedForVehicleEnter`, the line-up utility; bikes (type 9) and quads
/// (sub-type 2) go on through vtable +0x2C, others align (0x321).
fn prepare_and_align(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    set_vehicle_flags(this, ped);
    prepare_vehicle_for_ped_enter(this, ped);
    prepare_ped_for_vehicle_enter(this, ped);
    create_line_up_utility(this, ped);
    let v = veh(this);
    if rd::<u32>(v, 0x590) == 9 || rd::<u32>(v, 0x594) == 2 {
        v_after_simple_car_align(this, ped)
    } else {
        sub(this, 0x321, ped)
    }
}

/// The driver being dragged out (0x63F530 / 0x63F6E1): a stack `CEventDraggedOutCar(vehicle, ped, true)` to the
/// driver's event group, and a `CTaskComplexLeaveCar` (`ComputeTargetDoorToExit`) as its primary task; `alive_long`
/// picks the five-argument constructor (0x63B8C0) when true, the three-argument one (0x62F1A0) otherwise.
fn drag_driver_out(this: GPtr<u8>, ped: GPtr<u8>, alive_long: impl FnOnce(GPtr<u8>) -> bool) {
    let v = veh(this);
    let mut local = [0u32; 16];
    let lp = GPtr::from_raw(local.as_mut_ptr()).cast::<u8>();
    event_dragged_out_car_ctor(lp, v, ped, 1);
    let drv = rd::<GPtr<u8>>(veh(this), 0x460);
    event_group_add(rd::<GPtr<u8>>(drv, 0x47C).byte_add(0x68), lp, 1);
    let drv = rd::<GPtr<u8>>(veh(this), 0x460);
    let long = alive_long(drv);
    let t = task_new(0x34);
    let t = if t.is_null() {
        t
    } else {
        let v = veh(this);
        let d = compute_target_door_to_exit(v, rd::<GPtr<u8>>(v, 0x460)) as u32;
        if long {
            t_leave_car_5(t, veh(this), d as i32, 0, 0, 1)
        } else {
            t_leave_car_3(t, veh(this), d, 0)
        }
    };
    let drv = rd::<GPtr<u8>>(veh(this), 0x460);
    intelligence_add_task_primary_maybe_in_group(rd::<GPtr<u8>>(drv, 0x47C), t, 1);
    event_dragged_out_car_dtor(lp);
}

/// The slow-jack branch shared by `CreateNextSubTask` (0x63EE7A, `align` false) and the align step (0x63FB46,
/// `align` true): the jacked ped is stored (+0x40, referenced without dropping the old one). A player victim during a
/// skip (`CGameLogic::SkipState` == 2) ends the entry with the leader-quit event (the align step tests this first,
/// `CreateNextSubTask` only on the drag path). Dragging (quit-after-dragging, or a victim outside the ped's group
/// and not flagged +0x46C & 0x20000000): the victim gets `CTaskSimpleCarSlowBeDraggedOut` and `CEventDraggedOutCar`
/// while the entering ped drags (0x334); without a victim the entry finishes (`CreateNextSubTask` first bumps the
/// door status). Otherwise a victim that is the driver (driver entry) or the door's passenger (passenger entry)
/// leads to closing the door from outside (or finished), anything else to getting in.
fn slow_jack(this: GPtr<u8>, ped: GPtr<u8>, align: bool) -> GPtr<u8> {
    let j = compute_slow_jacked_ped(veh(this), door(this));
    wr::<GPtr<u8>>(this, 0x40, j);
    if !j.is_null() {
        entity_register_reference(j, this.byte_add(0x40));
    }
    let skip = |this: GPtr<u8>| {
        let j = rd::<GPtr<u8>>(this, 0x40);
        if !j.is_null() && ped_is_player(j) != 0 && g::<u32>(0x96A8A0).read() == 2 {
            let r = sub(this, FINISHED, ped);
            leader_event(
                rd::<GPtr<u8>>(this, 0x40),
                0xC,
                event_leader_quit_entering_car_ctor,
            );
            Some(r)
        } else {
            None
        }
    };
    if align && let Some(r) = skip(this) {
        return r;
    }
    let j = rd::<GPtr<u8>>(this, 0x40);
    let drag = flags(this) & 4 != 0
        || (!j.is_null()
            && ped_groups_are_in_same_group(j, ped) == 0
            && rd::<u32>(j, 0x46C) & 0x2000_0000 == 0);
    if !drag {
        let j = rd::<GPtr<u8>>(this, 0x40);
        let keep = if !j.is_null() && flags(this) & 1 != 0 && vehicle_is_driver(veh(this), j) != 0 {
            true
        } else if j.is_null() || flags(this) & 1 != 0 {
            false
        } else {
            let v = veh(this);
            let idx = compute_passenger_index_from_car_door(v, (door(this) as u32) as i32);
            rd::<GPtr<u8>>(this, 0x40) == rd::<GPtr<u8>>(v, 0x464 + 4 * idx as isize)
        };
        if !keep {
            return sub(this, 0x327, ped);
        }
        return if car_has_door_to_close(veh(this), door(this) as u32) != 0 {
            sub(this, 0x326, ped)
        } else {
            sub(this, FINISHED, ped)
        };
    }
    if !align && let Some(r) = skip(this) {
        return r;
    }
    if !rd::<GPtr<u8>>(this, 0x40).is_null() {
        let r = sub(this, 0x334, ped);
        let t = new_task(0x14, |t| t_slow_be_dragged_out(t, veh(this), door(this) as u32));
        let j = rd::<GPtr<u8>>(this, 0x40);
        intelligence_add_task_primary_maybe_in_group(rd::<GPtr<u8>>(j, 0x47C), t, 1);
        drag_event(this, ped);
        return r;
    }
    if align {
        return sub(this, FINISHED, ped);
    }
    if flags(this) & 4 != 0 {
        bump_door_status(veh(this), door(this));
        return sub(this, FINISHED, ped);
    }
    sub(this, 0x327, ped)
}

/// `CEventDraggedOutCar(vehicle, ped, driver == victim)` to the victim (+0x40), and the leader-exited event to its
/// group when it was the driver.
fn drag_event(this: GPtr<u8>, ped: GPtr<u8>) {
    let v = veh(this);
    let j = rd::<GPtr<u8>>(this, 0x40);
    let mut local = [0u32; 16];
    let lp = GPtr::from_raw(local.as_mut_ptr()).cast::<u8>();
    event_dragged_out_car_ctor(lp, v, ped, u32::from(rd::<GPtr<u8>>(v, 0x460) == j));
    event_group_add(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x40), 0x47C).byte_add(0x68), lp, 1);
    let j = rd::<GPtr<u8>>(this, 0x40);
    if rd::<GPtr<u8>>(veh(this), 0x460) == j {
        leader_event(j, 0x14, event_leader_exited_car_ctor);
    }
    event_dragged_out_car_dtor(lp);
}

/// `CTaskComplexEnterCar::CreateNextSubTask(CPed*)` (0x63E990). Aborting: 811 / 812 → set ped out (0x330), the
/// listed types → finished, others null. Otherwise finished without a healthy vehicle, then by the sub-task's type:
/// 0x320 go to door (target door and position copied; door in use → stop, wait 0x32A; door neither ready nor
/// partially open → finished; else prepare and align), 0x321 → the align step (vtable +0x2C), 0x323 → cruise speed,
/// stand still, 0x322 / 0x324 → speed / quit / slow-jack tests then get in, 0x327 get in → close the door (unless a
/// player driver floors accelerate or brake: the door left open), passenger / driver / shuffle (dragging the driver
/// out at door 8), 0x325 close door → driver / passenger / shuffle, 0x328 → driver, 0x32A → prepare and align when
/// the door is ready, 0x331 → driver, 0x334 → get in or wait until out, 0xD0 → finished (and the first sub-task again
/// with carry-on), 0x2C0 → finished in a vehicle, else boat / go to door; the rest finished or null as the bytes.
#[hook(addr = 0x63E990, conv = thiscall, name = "CTaskComplexEnterCar::CreateNextSubTask", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "std14-ai-1", route = "L2", hazards = "heap-writes", recs = "enter-car",
       note = "0x2C0 (leave car) and the fall-and-get-up order follow the bytes, unlike gta-reversed")]
pub fn create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let st = || rd::<GPtr<u8>>(this, 8);
    if rd::<u8>(this, 0x3C) != 0 {
        let ty = v_task_type(st());
        return match ty {
            0x32B | 0x32C => sub(this, 0x330, ped),
            0x320..=0x328
            | 0x2C0
            | 0xC8
            | 0xCB
            | 0xD0
            | 0x2CB
            | 0x2CF
            | 0x32A
            | 0x330
            | 0x331
            | 0x334
            | 0x33D => sub(this, FINISHED, ped),
            _ => GPtr::NULL,
        };
    }
    if veh(this).is_null() || is_vehicle_healthy(veh(this)) == 0 {
        return sub(this, FINISHED, ped);
    }
    match v_task_type(st()) {
        0x2CF | 0xC8 | 0xCB | 0x2CB | 0x326 | 0x32B | 0x32C | 0x33D => sub(this, FINISHED, ped),
        0xD0 => {
            if find_player_ped(-1) == ped {
                let i = u32::from(g::<u8>(0xB7CD74).read());
                player_info_set_last_target_vehicle(
                    GPtr::from_addr((0xB7CD98 + 0x190 * i) as usize),
                    GPtr::NULL,
                );
            }
            if flags(this) & 8 != 0 {
                sub(this, FINISHED, ped);
                v_create_first_sub_task(this, ped)
            } else {
                sub(this, FINISHED, ped)
            }
        }
        0x2C0 => {
            if in_vehicle(ped) {
                sub(this, FINISHED, ped)
            } else if flags(this) & 1 != 0 && rd::<u32>(veh(this), 0x590) == 5 {
                sub(this, 0x2CB, ped)
            } else {
                sub(this, 0x320, ped)
            }
        }
        0x320 => {
            let t = st();
            if rd::<u8>(t, 0x49) == 0 {
                return sub(this, FINISHED, ped);
            }
            wr::<u32>(this, 0x14, rd::<u32>(t, 0x34));
            for k in 0..3 {
                wr::<u32>(this, 0x2C + 4 * k, rd::<u32>(t, 0x38 + 4 * k));
            }
            let opp = compute_target_door_opposite_to_flag(this);
            wr::<i32>(this, 0x18, opp);
            if is_car_door_in_use(veh(this), door(this), opp) != 0 {
                if ped_is_player(ped) != 0 {
                    wr::<f32>(rd::<GPtr<u8>>(ped, 0x480), 0x14, 0.0);
                    ped_set_move_state(ped, 1);
                    player_ped_set_real_move_anim(ped);
                } else {
                    ped_set_move_state(ped, 1);
                    v_ped_set_move_anim(ped);
                }
                return sub(this, 0x32A, ped);
            }
            if is_car_door_ready(veh(this), door(this)) == 0
                && car_has_partially_open_door(veh(this), door(this)) == 0
            {
                return sub(this, FINISHED, ped);
            }
            prepare_and_align(this, ped)
        }
        0x321 => v_after_simple_car_align(this, ped),
        0x323 => {
            let v = veh(this);
            if !v.is_null() {
                set_cruise_speed(this, v);
            }
            sub(this, 0xCB, ped)
        }
        0x322 | 0x324 => {
            if too_fast(ped, veh(this)) {
                wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) | 1);
                return if ped_is_player(ped) != 0 {
                    sub(this, FINISHED, ped)
                } else {
                    sub(this, 0xD0, ped)
                };
            }
            if flags(this) & 2 != 0 {
                wr::<u8>(this, 0x10, flags(this) & !2);
                return sub(this, FINISHED, ped);
            }
            if flags(this) & 4 == 0 && is_car_slow_jack_required(veh(this), door(this), ped) == 0 {
                return sub(this, 0x327, ped);
            }
            slow_jack(this, ped, false)
        }
        0x327 => get_in_next(this, ped),
        0x325 => {
            let v = veh(this);
            if v.is_null() || is_vehicle_healthy(v) == 0 || is_ped_healthy(ped) == 0 {
                return sub(this, FINISHED, ped);
            }
            if flags(this) & 1 == 0 {
                return sub(this, 0x32B, ped);
            }
            if door(this) != 8 {
                return sub(this, 0x32C, ped);
            }
            if rd::<u32>(veh(this), 0x460) != 0 {
                drag_driver_out(this, ped, |drv| {
                    ped_is_alive(drv) as u8 != 0 && x(rd::<f32>(drv, 0x540)).fcom(x(0.0)).test_parity(0x41)
                });
            }
            let hd = rd::<GPtr<u8>>(veh(this), 0x384);
            if (rd::<u32>(hd, 0xCC) >> 9) & 1 != 0 {
                sub(this, 0x32C, ped)
            } else {
                sub(this, 0x328, ped)
            }
        }
        0x328 => sub(this, 0x32C, ped),
        0x32A => {
            if is_car_door_ready(veh(this), door(this)) == 0
                && car_has_partially_open_door(veh(this), door(this)) == 0
            {
                return GPtr::NULL;
            }
            prepare_and_align(this, ped)
        }
        0x331 => {
            make_undragged_passenger_peds_leave_car(veh(this), rd::<GPtr<u8>>(this, 0x40), ped);
            sub(this, 0x32C, ped)
        }
        0x334 => {
            if flags(this) & 4 == 0 {
                return sub(this, 0x327, ped);
            }
            wr::<u8>(this, 0x10, flags(this) & !4);
            if rd::<u32>(this, 0x40) != 0 {
                sub(this, 0x33D, ped)
            } else {
                sub(this, FINISHED, ped)
            }
        }
        _ => GPtr::NULL,
    }
}

/// The get-in step of `CreateNextSubTask` (0x63F3A2).
fn get_in_next(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    make_undragged_passenger_peds_leave_car(veh(this), rd::<GPtr<u8>>(this, 0x40), ped);
    let mut keep_open = false;
    let v = veh(this);
    if flags(this) & 1 != 0
        && ped_is_player(ped) != 0
        && rd::<u32>(v, 0x590) == 0
        && !matches!(rd::<u32>(v, 0x594), 3 | 4)
    {
        let pad = GPtr::<u8>::from_addr(PAD0);
        let full = kf(0x859AAC);
        let floored = !xi(i32::from(pad_get_accelerate(pad) as i16)).fcom(x(full)).c0()
            || !xi(i32::from(pad_get_brake(pad) as i16)).fcom(x(full)).c0();
        if floored {
            let dm = veh(this).byte_add(0x5A0);
            let st = damage_get_door_status(dm, door(this) as u32);
            if (st == 0 || damage_get_door_status(dm, door(this) as u32) == 2)
                && car_has_door_to_close(veh(this), door(this) as u32) != 0
            {
                let s = damage_get_door_status(dm, door(this) as u32);
                damage_set_door_status(dm, door(this) as u32, s.wrapping_add(1));
            }
            keep_open = true;
        }
    }
    if car_has_door_to_close(veh(this), door(this) as u32) != 0 && !keep_open {
        return sub(this, 0x325, ped);
    }
    if flags(this) & 1 == 0 {
        return sub(this, 0x32B, ped);
    }
    let v = veh(this);
    if door(this) != 8
        || rd::<u32>(v, 0x590) != 0
        || (rd::<u32>(rd::<GPtr<u8>>(v, 0x384), 0xCC) >> 9) & 1 != 0
    {
        return sub(this, 0x32C, ped);
    }
    if rd::<u32>(v, 0x460) != 0 {
        drag_driver_out(this, ped, |drv| ped_is_alive(drv) as u8 == 0);
    }
    sub(this, 0x328, ped)
}

/// `CTaskComplexEnterCar::CreateNextSubTask_AfterSimpleCarAlign(CPed*)` (0x63F970). Too fast (as 0x322): finished
/// (player) or fall and get up. Quit after opening: open (0x322) or locked (0x323) door when the car has a door to
/// open, else the flag cleared and finished. A possible quick jack (no drag-out quit) leaves the result null. Cars:
/// a door to open → open (0x322; the anim group 0x13 driver-door case without a driver takes get in) or locked
/// (0x323); otherwise the slow-jack test as `CreateNextSubTask`. Bikes and quads: slow jack → the bike jack tasks
/// (0x648B90 for the victim and the passenger, not ported here: blocked), else finished (drag-out quit), upright
/// bikes with status 4 get in (+0x614 |= 8), others pick up (0x324) or get in. Then, unless a bus, every passenger and
/// the driver get a delayed chat record ("TaskEnterCar" store 0xC15448) with `rand()` delays.
#[hook(addr = 0x63F970, conv = thiscall, name = "CTaskComplexEnterCar::CreateNextSubTask_AfterSimpleCarAlign", expect = "6A FF 68 A5 1E 84 00 64",
       unverified = "std14-ai-1", route = "L2", hazards = "heap-writes", recs = "enter-car",
       note = "rand() at 0x640182 / 0x6401AC / 0x640249 / 0x640273")]
pub fn create_next_sub_task_after_simple_car_align(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let (res, chat) = after_align(this, ped);
    if chat {
        chat_reactions(this, ped);
    }
    res
}

/// The result of the align step and whether the chat reactions follow (every path but a bike victim's skip).
fn after_align(this: GPtr<u8>, ped: GPtr<u8>) -> (GPtr<u8>, bool) {
    if too_fast(ped, veh(this)) {
        wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) | 1);
        return (
            if ped_is_player(ped) != 0 {
                sub(this, FINISHED, ped)
            } else {
                sub(this, 0xD0, ped)
            },
            true,
        );
    }
    if flags(this) & 2 != 0 {
        if car_has_door_to_open(veh(this), door(this) as u32) != 0 {
            let ty = if car_has_openable_door(veh(this), door(this) as u32, ped) != 0 {
                0x322
            } else {
                0x323
            };
            return (sub(this, ty, ped), true);
        }
        wr::<u8>(this, 0x10, flags(this) & !2);
        return (sub(this, FINISHED, ped), true);
    }
    if flags(this) & 4 == 0 && is_car_quick_jack_possible(veh(this), door(this), ped) != 0 {
        return (GPtr::NULL, true);
    }
    let v = veh(this);
    if rd::<u32>(v, 0x590) == 9 || rd::<u32>(v, 0x594) == 2 {
        return bike_next(this, ped);
    }
    let r = if car_has_door_to_open(veh(this), door(this) as u32) != 0 {
        if car_has_openable_door(veh(this), door(this) as u32, ped) != 0 {
            let v = veh(this);
            let grp = rd::<u8>(rd::<GPtr<u8>>(v, 0x384), 0xDE);
            if flags(this) & 6 == 0
                && u32::from(grp) + 0x58 == 0x6B
                && door(this) == 10
                && rd::<u32>(v, 0x460) == 0
            {
                sub(this, 0x327, ped)
            } else {
                sub(this, 0x322, ped)
            }
        } else {
            sub(this, 0x323, ped)
        }
    } else if is_car_slow_jack_required(veh(this), door(this), ped) != 0 {
        slow_jack(this, ped, true)
    } else if flags(this) & 4 != 0 {
        sub(this, FINISHED, ped)
    } else {
        sub(this, 0x327, ped)
    };
    (r, true)
}

/// The bike / quad part of the align step (0x63FE36).
fn bike_next(this: GPtr<u8>, ped: GPtr<u8>) -> (GPtr<u8>, bool) {
    if is_car_slow_jack_required(veh(this), door(this), ped) != 0 {
        let j = compute_slow_jacked_ped(veh(this), door(this));
        wr::<GPtr<u8>>(this, 0x40, j);
        if !j.is_null() {
            entity_register_reference(j, this.byte_add(0x40));
        }
        let j = rd::<GPtr<u8>>(this, 0x40);
        if !j.is_null() && ped_is_player(j) != 0 && g::<u32>(0x96A8A0).read() == 2 {
            let r = sub(this, FINISHED, ped);
            leader_event(
                rd::<GPtr<u8>>(this, 0x40),
                0xC,
                event_leader_quit_entering_car_ctor,
            );
            return (r, false);
        }
        let ty = if flags(this) & 4 == 0 && door(this) == 0x12 {
            0x327
        } else {
            0x334
        };
        let r = sub(this, ty, ped);
        let d = if door(this) == 0x12 && rd::<u32>(veh(this), 0x464) == 0 {
            0xB
        } else {
            0xA
        };
        let t = new_task(0x2C, |t| {
            t_0x648b90(t, veh(this), d, rd::<u32>(this, 0x20), ped, 1)
        });
        intelligence_add_task_primary_maybe_in_group(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x40), 0x47C), t, 1);
        let v = veh(this);
        if rd::<GPtr<u8>>(this, 0x40) == rd::<GPtr<u8>>(v, 0x460) && rd::<u32>(v, 0x464) != 0 {
            let t = new_task(0x2C, |t| {
                t_0x648b90(
                    t,
                    veh(this),
                    0xB,
                    rd::<u32>(this, 0x20).wrapping_add(0xC8),
                    ped,
                    0,
                )
            });
            let p = rd::<GPtr<u8>>(veh(this), 0x464);
            intelligence_add_task_primary_maybe_in_group(rd::<GPtr<u8>>(p, 0x47C), t, 1);
            let j = rd::<GPtr<u8>>(this, 0x40);
            if rd::<GPtr<u8>>(veh(this), 0x460) == j {
                leader_event(j, 0x14, event_leader_exited_car_ctor);
            }
        }
        return (r, true);
    }
    if flags(this) & 4 != 0 {
        return (sub(this, FINISHED, ped), true);
    }
    let v = veh(this);
    if rd::<u8>(v, 0x36) & 0xF8 != 0x20 || rd::<u32>(v, 0x590) != 9 {
        return (sub(this, 0x327, ped), true);
    }
    let m = rd::<GPtr<u8>>(v, 0x14);
    let rz = x(rd::<f32>(m, 8));
    if rz.fcom(x(kf(0x858B8C))).test_parity(0x05)
        || rz.fcom(x(kf(0x858F40))).test_any(0x41)
        || x(rd::<f32>(m, 0x28)).fcom(x(0.0)).test_any(0x41)
    {
        return (sub(this, 0x324, ped), true);
    }
    let r = sub(this, 0x327, ped);
    let v = veh(this);
    wr::<u8>(v, 0x614, rd::<u8>(v, 0x614) | 8);
    (r, true)
}

/// The chat reactions at the end of the align step (0x64013B): unless a bus, each passenger slot then the driver.
fn chat_reactions(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u8>(veh(this), 0x429) & 2 != 0 {
        return;
    }
    let react = |p: GPtr<u8>, sites: (u32, u32)| {
        let friendly = intelligence_is_friendly_with(rd::<GPtr<u8>>(ped, 0x47C), p) != 0
            || ped_groups_are_in_same_group(ped, p) != 0;
        let (site, k, base) = if friendly {
            (sites.1, 0x859948, 2000)
        } else {
            (sites.0, 0x86EAA8, 6000)
        };
        let r = at_site(site, rand) & 0xFFFF;
        let v = ftol32(xi(r).fmul_m(kf(0x858B14)).fmul_m(kf(k)));
        ped_chat_say(
            GPtr::from_addr(0xC15448),
            GPtr::from_addr(0x86EA98),
            p,
            ped,
            ((base - v) as u32) as i32,
            5,
            GPtr::NULL,
            0,
            f32::from_bits(0x3F00_0000),
            0x15E,
            3,
            0,
        );
    };
    for i in 0..8 {
        let p = rd::<GPtr<u8>>(veh(this), 0x464 + 4 * i);
        if !p.is_null() {
            react(p, (0x640187, 0x6401B1));
        }
    }
    let d = rd::<GPtr<u8>>(veh(this), 0x460);
    if !d.is_null() {
        react(d, (0x64024E, 0x640278));
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_ENTER_CAR_AS_DRIVER_DTOR,
    HOOK_CREATE_FIRST_SUB_TASK,
    HOOK_CONTROL_SUB_TASK,
    HOOK_CREATE_SUB_TASK,
    HOOK_CREATE_NEXT_SUB_TASK,
    HOOK_CREATE_NEXT_SUB_TASK_AFTER_SIMPLE_CAR_ALIGN,
];
