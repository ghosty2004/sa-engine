//! `CTaskComplexKillPedOnFoot` (batch `carjack-1`): the base task a ped runs when it fights another ped, e.g. a driver
//! who gets back up after being jacked and goes for CJ. Destructor, `CreateSubTask`, `ControlSubTask`,
//! `CreateNextSubTask`, `CreateFirstSubTask` and the taunt helper `FUN_0065e9a0`. Ported from the 1.0 US bytes (not
//! reversed by gta-reversed). Unverified (docs/verify/HOWTO.md).
//!
//! Fields: +8 sub-task, +0xC flags (bit 0 "first control", 1 "restart", 2 "room to drag out", 3 "target dead",
//! 4 copied to the armed task's +0x35, 5/6 "wanted-level stand-off"), +0x10 target ped, +0x14 .. +0x20 the armed
//! task's arguments, +0x24 start time, +0x28 time limit, +0x2C a `CTaskTimer` (start, interval, started).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_math::x87::Cmp;

use crate::common::*;

orig! {
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplex::CreateFirstSubTask(CPed*)` (vtable +0x24)
    fn v_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CTaskComplex::~CTaskComplex()` (0x61A3D0)
    fn task_complex_dtor(t: GPtr<u8>) = thiscall 0x61A3D0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CTaskComplexKillPedOnFoot::CreateSubTask(int32, CPed*)` (0x625E70, `ret 8`)
    fn create_sub_task_o(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x625E70 => kill_ped_on_foot_create_sub_task;
    /// `FUN_0065e9a0(CPed*, CPed*)` (cdecl)
    fn taunt_o(ped: GPtr<u8>, target: GPtr<u8>) = cdecl 0x65E9A0 => kill_ped_on_foot_taunt;

    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(time, looped, idle, blend)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, time: u32, looped: u32, idle: u32, blend: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskSimpleStandStill::ProcessPed(CPed*)` (0x62F370, `ret 4`)
    fn stand_still_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x62F370 => unported;
    /// `CTaskSimpleStandStill::~CTaskSimpleStandStill()` (0x62F360)
    fn stand_still_dtor(t: GPtr<u8>) = thiscall 0x62F360 => unported;
    /// `CTaskSimplePause::CTaskSimplePause(int32 time)` (0x48E750, `ret 4`)
    fn pause_ctor(t: GPtr<u8>, time: u32) -> GPtr<u8> = thiscall 0x48E750 => unported;
    /// `CTaskSimpleLeaveGroup::CTaskSimpleLeaveGroup()` (0x463540; an 8-byte simple task)
    fn leave_group_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x463540 => unported;
    /// `CTaskComplexLeaveCar::CTaskComplexLeaveCar(veh, door, delay, sensible, forceGetOut)` (0x63B8C0, `ret 0x14`)
    fn leave_car_ctor(t: GPtr<u8>, veh: u32, door: u32, delay: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    /// `CTaskComplexDragPedFromCar::CTaskComplexDragPedFromCar(CPed*, int32)` (0x640430, `ret 8`)
    fn drag_ped_from_car_ctor(t: GPtr<u8>, ped: u32, a: u32) -> GPtr<u8> = thiscall 0x640430 => unported;
    /// `CTaskSimpleCarDriveTimed::CTaskSimpleCarDriveTimed(veh, time)` (0x5FF940, `ret 8`)
    fn car_drive_timed_ctor(t: GPtr<u8>, veh: u32, time: u32) -> GPtr<u8> = thiscall 0x5FF940 => unported;
    /// `CTaskComplexKillPedOnFootMelee::CTaskComplexKillPedOnFootMelee(CPed*)` (0x621000, `ret 4`)
    fn kill_melee_ctor(t: GPtr<u8>, target: u32) -> GPtr<u8> = thiscall 0x621000 => unported;
    /// `CTaskComplexKillPedOnFootArmed::CTaskComplexKillPedOnFootArmed(...)` (0x621190, `ret 0x14`)
    fn kill_armed_ctor(t: GPtr<u8>, target: u32, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x621190 => unported;
    /// `CTaskComplexDestroyCar::CTaskComplexDestroyCar(veh, a, b, c)` (0x621C00, `ret 0x10`)
    fn destroy_car_ctor(t: GPtr<u8>, veh: u32, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x621C00 => unported;
    /// `CTaskComplexSignalAtPed::CTaskComplexSignalAtPed(CPed*, int32 time, bool)` (0x660A30, `ret 0xC`)
    fn signal_at_ped_ctor(t: GPtr<u8>, ped: u32, time: u32, b: u32) -> GPtr<u8> = thiscall 0x660A30 => unported;

    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWorld::FindPlayerWanted(int32)` (0x56E230, cdecl)
    fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CPedGroups::GetPedsGroup(CPed*)` (0x5F7E80, cdecl)
    fn peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupMembership::GetLeader()` (0x5F69A0)
    fn membership_get_leader(m: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F69A0 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180, cdecl)
    fn random_in_range(a: i32, b: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CPed::Say(uint16, uint32, float, uint8, uint8, uint8)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, offset: u32, vol: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::GiveWeapon(eWeaponType, uint32 ammo, bool)` (0x5E6080, `ret 0xC`)
    fn ped_give_weapon(p: GPtr<u8>, ty: u32, ammo: u32, b: u32) -> u32 = thiscall 0x5E6080 => unported;
    /// `CPed::SetCurrentWeapon(eWeaponType)` (0x5E6280, `ret 4`)
    fn ped_set_current_weapon_type(p: GPtr<u8>, ty: u32) = thiscall 0x5E6280 => unported;
    /// `CPed::SetCurrentWeapon(int32 slot)` (0x5E61F0, `ret 4`)
    fn ped_set_current_weapon_slot(p: GPtr<u8>, slot: u32) = thiscall 0x5E61F0 => unported;
    /// `CPed::SetMoveState(eMoveState)` (0x5DEC00, `ret 4`)
    fn ped_set_move_state(p: GPtr<u8>, s: u32) = thiscall 0x5DEC00 => unported;
    /// `CPed::DropEntityThatThisPedIsHolding(bool)` (0x5E0360, `ret 4`)
    fn ped_drop_entity_held(p: GPtr<u8>, b: u32) = thiscall 0x5E0360 => unported;
    /// `FUN_005fe9c0(int32 pedType)` (cdecl; a gang ped type test)
    fn is_gang_ped_type(t: u32) -> u8 = cdecl 0x5FE9C0 => unported;
    /// `CCullZones::NoPolice()` (0x72DD50)
    fn cull_zones_no_police() -> u8 = cdecl 0x72DD50 => unported;
    /// `CWeapon::IsTypeMelee()` (0x73B1C0)
    fn weapon_is_type_melee(w: GPtr<u8>) -> u8 = thiscall 0x73B1C0 => unported;
    /// `CVehicle::IsDriver(CPed*)` (0x6D1C40, `ret 4`)
    fn vehicle_is_driver(v: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x6D1C40 => unported;
    /// `CVehicle::IsPassenger(CPed*)` (0x6D1BD0, `ret 4`)
    fn vehicle_is_passenger(v: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x6D1BD0 => unported;
    /// `CVehicle::CanPedOpenLocks(CPed*)` (0x6D1E20, `ret 4`)
    fn vehicle_can_ped_open_locks(v: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x6D1E20 => unported;
    /// `CCarEnterExit::ComputeTargetDoorToExit(CVehicle*, CPed*)` (0x64F110, cdecl)
    fn compute_target_door_to_exit(v: GPtr<u8>, p: GPtr<u8>) -> u32 = cdecl 0x64F110 => unported;
    /// `CCarEnterExit::IsRoomForPedToLeaveCar(CVehicle*, int32 door, CVector*)` (0x6504C0, cdecl)
    fn is_room_for_ped_to_leave_car(v: GPtr<u8>, door: u32, offset: u32) -> u8 = cdecl 0x6504C0 => unported;
    /// `CCarEnterExit::IsVehicleHealthy(CVehicle*)` (0x64EEC0, cdecl)
    fn is_vehicle_healthy(v: GPtr<u8>) -> u8 = cdecl 0x64EEC0 => unported;
    /// `CTaskTimer::IsOutOfTime()` (0x420E30)
    fn timer_is_out_of_time(t: GPtr<u8>) -> u8 = thiscall 0x420E30 => unported;
    /// `CEventAreaCodes::CEventAreaCodes(CPed*)` (0x4B2190, `ret 4`)
    fn area_codes_ctor(e: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B2190 => unported;
    /// `CEventAreaCodes` destructor (0x4B2210)
    fn area_codes_dtor(e: GPtr<u8>) = thiscall 0x4B2210 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> u32 = thiscall 0x4AB420 => unported;
}

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// 0.0, 0.1 (a vehicle "slow enough" to drag its occupant out), 5.0 (the gang-gun distance)
const ZERO: usize = 0x858B50;
const SLOW_SPEED: usize = 0x858B1C;
const GANG_GUN_DIST: usize = 0x858C80;

fn now() -> u32 {
    g::<u32>(TIME_MS).read()
}

fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

fn target(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0x10)
}

fn sub(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 8)
}

fn in_vehicle(p: GPtr<u8>) -> bool {
    rd::<u32>(p, 0x46C) & 0x100 != 0
}

/// The ped's current weapon (`ped + 0x5A0 + slot (+0x718, signed byte) * 0x1C`) is a melee one.
fn current_weapon_melee(ped: GPtr<u8>) -> bool {
    let slot = isize::from(rd::<i8>(ped, 0x718));
    weapon_is_type_melee(ped.byte_add(0x5A0 + slot * 0x1C)) != 0
}

/// `sub->MakeAbortable(ped, 1 (urgent), NULL)`.
fn abort_urgent(this: GPtr<u8>, ped: GPtr<u8>) -> bool {
    v_make_abortable(sub(this), ped, 1, GPtr::NULL) != 0
}

/// The wanted flags byte (+0x1E) of the player data's (+0x480) wanted record (NULL data reads address 0x1E, as the
/// original).
fn player_wanted_flags(p: GPtr<u8>) -> u8 {
    let data = rd::<GPtr<u8>>(p, 0x480);
    let w = if data.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(data, 0) };
    rd::<u8>(w, 0x1E)
}

/// `CreateSubTask(ty, ped)` through the original address.
fn create(this: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> {
    create_sub_task_o(this, ty, ped)
}

/// The target's door room test the original repeats: `door = ComputeTargetDoorToExit(veh, target)`, flag bit 2 :=
/// `IsRoomForPedToLeaveCar(veh, door, NULL)`; returns the bit.
fn update_room_flag(this: GPtr<u8>) -> bool {
    let t = target(this);
    let door = compute_target_door_to_exit(rd::<GPtr<u8>>(t, 0x58C), t);
    let room = is_room_for_ped_to_leave_car(rd::<GPtr<u8>>(target(this), 0x58C), door, 0);
    let fl = rd::<u8>(this, 0xC);
    let fl = fl ^ (((room << 2) ^ fl) & 4);
    wr::<u8>(this, 0xC, fl);
    fl & 4 != 0
}

/// The timer at +0x2C restarted for 2000 ms.
fn restart_timer(this: GPtr<u8>) {
    wr::<u32>(this, 0x2C, now());
    wr::<u32>(this, 0x30, 2000);
    wr::<u8>(this, 0x34, 1);
}

/// `sqrt(a * a + b * b)` of the two floats (x87) compared with `c`.
fn speed_cmp(a: f32, b: f32, c: f32) -> Cmp {
    x(a).fmul_m(a).fadd(x(b).fmul_m(b)).fsqrt().fcom(x(c))
}

/// A gang or cop ped (`FUN_005fe9c0(type) || type == 6`) not created by a mission script that is near a player
/// target arms itself: `GiveWeapon(0x16, 1000, true)`, `SetCurrentWeapon(0x16)`, `SetMoveState(1)`. `player` is the
/// target when it is a player, else NULL; `dist` is the target distance (f32), checked against 5.0 unless the cull
/// zones forbid police.
/// `player_first`: `CreateFirstSubTask` tests the player before the zone/distance, `CreateNextSubTask` after.
fn maybe_arm(this: GPtr<u8>, ped: GPtr<u8>, player: GPtr<u8>, player_first: bool, dist: impl Fn() -> Cmp) {
    if player_first && player.is_null() {
        return;
    }
    if cull_zones_no_police() == 0 {
        let g = rd::<u32>(target(this), 0x568);
        if g == 0 || g == rd::<u32>(ped, 0x568) || dist() != Cmp::Less {
            return;
        }
    }
    if player.is_null() {
        return;
    }
    if rd::<u8>(rd::<GPtr<u8>>(player, 0x480), 0x34) & 1 == 0 {
        return;
    }
    let ty = rd::<u32>(ped, 0x598);
    if (is_gang_ped_type(ty) == 0 && ty != 6) || rd::<u8>(ped, 0x484) == 2 {
        return;
    }
    ped_give_weapon(ped, 0x16, 1000, 1);
    ped_set_current_weapon_type(ped, 0x16);
    ped_set_move_state(ped, 1);
}

/// `CTaskComplexKillPedOnFoot::~CTaskComplexKillPedOnFoot()` (0x620EF0, catalogue `FUN_00620ef0`, SEH frame): vtable
/// 0x86D894, `CleanUpOldReference` of the target (+0x10) when set, `CTaskComplex::~CTaskComplex()`.
#[hook(addr = 0x620EF0, conv = thiscall, name = "FUN_00620ef0", expect = "6A FF 68 68 F6 83 00",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
pub fn kill_ped_on_foot_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86D894);
    let t = target(this);
    if !t.is_null() {
        entity_clean_up_old_reference(t, this.byte_add(0x10));
    }
    task_complex_dtor(this);
}

/// `CTaskComplexKillPedOnFoot::CreateSubTask(int32 type, CPed*)` (0x625E70, `ret 8`; entry `nop; jmp` into the
/// protection's relocated `mov eax, fs:[0]`). 0xC8 → `CTaskSimpleLeaveGroup`; 0xCA → a stack `CTaskSimpleStandStill(0,
/// false, false, 8.0)` processed once, then `CTaskSimplePause(10000)` when flags 0x20 and 0x40 are set, else
/// `Pause(2000)`; 0xCB → `StandStill(0, false, false, 8.0)`; 0x2BF → `CTaskComplexDragPedFromCar(target, 0)` and the
/// timer restarted (2000 ms); 0x2C0 → `CTaskComplexLeaveCar(ped's vehicle, 0, 0, true, true)`; 0x2CF →
/// `CTaskSimpleCarDriveTimed(ped's vehicle, 2000)`; 0x3E9 → `CTaskComplexKillPedOnFootMelee(target)` + timer; 0x3EA
/// → `CTaskComplexKillPedOnFootArmed(+0x10, +0x14, +0x18, +0x1C, (int8)+0x20)` whose +0x35 gets flag bit 4 (written
/// without a NULL check, as the original), flag bit 4 cleared, timer; 0x3EB → `CTaskComplexDestroyCar(target's
/// vehicle, 0, 0, 0)`; 0x4BA → the ped says 0x103 when the target is player 0, else 0x102 unless the target's group
/// leader is player 0, then `CTaskComplexSignalAtPed(target, GetRandomNumberInRange(0, 1500), true)`; 0x516 → ped
/// +0x474 loses 0x400000, NULL; others NULL.
#[hook(addr = 0x625E70, conv = thiscall, name = "CTaskComplexKillPedOnFoot::CreateSubTask", expect = "90 E9 82 42 DE FF",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
#[allow(clippy::too_many_lines)]
pub fn kill_ped_on_foot_create_sub_task(this: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> {
    let alloc = |size: u32, ctor: &dyn Fn(GPtr<u8>) -> GPtr<u8>| {
        let t = task_new(size);
        if t.is_null() { t } else { ctor(t) }
    };
    match ty {
        0xC8 => alloc(8, &|t| leave_group_ctor(t)),
        0xCA => {
            let mut local = [0u32; 8];
            let l = buf(&mut local);
            stand_still_ctor(l, 0, 0, 0, 0x4100_0000);
            stand_still_process_ped(l, ped);
            let fl = rd::<u8>(this, 0xC);
            let time = if fl & 0x20 != 0 && fl & 0x40 != 0 { 10_000 } else { 2000 };
            let r = alloc(0x18, &|t| pause_ctor(t, time));
            stand_still_dtor(l);
            r
        }
        0xCB => alloc(0x20, &|t| stand_still_ctor(t, 0, 0, 0, 0x4100_0000)),
        0x2BF => {
            let r = alloc(0x54, &|t| drag_ped_from_car_ctor(t, rd::<u32>(this, 0x10), 0));
            restart_timer(this);
            r
        }
        0x2C0 => alloc(0x34, &|t| leave_car_ctor(t, rd::<u32>(ped, 0x58C), 0, 0, 1, 1)),
        0x2CF => alloc(0x70, &|t| car_drive_timed_ctor(t, rd::<u32>(ped, 0x58C), 2000)),
        0x3E9 => {
            let r = alloc(0x38, &|t| kill_melee_ctor(t, rd::<u32>(this, 0x10)));
            restart_timer(this);
            r
        }
        0x3EA => {
            let r = alloc(0x5C, &|t| {
                kill_armed_ctor(
                    t,
                    rd::<u32>(this, 0x10),
                    rd::<u32>(this, 0x14),
                    rd::<u32>(this, 0x18),
                    rd::<u32>(this, 0x1C),
                    rd::<i8>(this, 0x20) as i32 as u32,
                )
            });
            wr::<u8>(r, 0x35, (rd::<u8>(this, 0xC) >> 4) & 1);
            wr::<u8>(this, 0xC, rd::<u8>(this, 0xC) & 0xEF);
            restart_timer(this);
            r
        }
        0x3EB => alloc(0x20, &|t| destroy_car_ctor(t, rd::<u32>(target(this), 0x58C), 0, 0, 0)),
        0x4BA => {
            let t = target(this);
            if t == find_player_ped(0) {
                ped_say(ped, 0x103, 0, 0x3F80_0000, 0, 0, 0);
            } else {
                let grp = peds_group(t);
                let leader_is_player = !grp.is_null() && {
                    let leader = membership_get_leader(peds_group(target(this)).byte_add(8));
                    leader == find_player_ped(0)
                };
                if !leader_is_player {
                    ped_say(ped, 0x102, 0, 0x3F80_0000, 0, 0, 0);
                }
            }
            let time = random_in_range(0, 1500) as u32;
            alloc(0x18, &|t| signal_at_ped_ctor(t, rd::<u32>(this, 0x10), time, 1))
        }
        0x516 => {
            wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) & 0xFFBF_FFFF);
            GPtr::NULL
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexKillPedOnFoot::ControlSubTask(CPed*)` (0x626260, `ret 4`; catalogue `FUN_00626260`). Sets ped
/// +0x474 bit 0x400000. Without a target the next type is 0x516. A stand-off sub-task (0xCA) is kept (aborted urgently
/// first when flags 0x20/0x40 are set and the player is no longer wanted-flagged, `FindPlayerWanted(-1)` +0x1E bit
/// 2). A time limit (+0x28 > 0) past `+0x24 + +0x28` → an aborted sub-task is replaced by `StandStill(0, false,
/// false, 8.0)`. Flag 2 → an aborted sub-task is replaced by the task's own `CreateFirstSubTask`. Unless flag 8: a
/// dead target (health +0x540 ≤ 0) → after an urgent abort flag 8, a cop says 0xE unless the target is a player,
/// then `CreateSubTask(0x4BA)` when ped +0x478 bit 12, else `CTaskSimpleLeaveGroup` (a sub-task already 0x4BA is
/// kept); a live player target that is wanted-flagged (or a cop whose +0x79C is set while the wanted level +0x1C is
/// 0) → next 0xCA (flag 0x40 added when flag 0x20 is set and the player is wanted-flagged). Otherwise by sub-task
/// type: melee (0x3E9; +0x30 = flag 0): a non-melee weapon → 0x3EA; a target in a car/bike-like vehicle (type 3/4) →
/// 0x3EB; a target in an unlocked non-boat vehicle → room to get out ? 0x2BF : 0x3EB. Drag-out (0x2BF): a target
/// still in the vehicle going faster than 0.1 → 0x3EB; once the timer runs out, the room flag decides (no room →
/// 0x3EB) and the timer restarts. Armed (0x3EA; +0x33 = flag 0): a melee weapon (non-player) → 0x3E9, an empty
/// weapon → the first slot with ammo (none: fists, 0x3E9); then the target-in-vehicle checks as melee (slow,
/// unlocked, not a boat / type 9). Destroy-car (0x3EB): a target on foot → 0x3E9 / 0x3EA by weapon; an unhealthy
/// vehicle → 0x4BA; a slow unlocked vehicle → 0x2BF with the room flag, or when the timer runs out and there is room
/// (timer restarted). Flag 0 is cleared. When the target and the ped both have an area entity (+0x584) with
/// different area codes (+0x2F) a `CEventAreaCodes(target)` is added to the ped's event group (+0x47C → +0x68). A
/// new type other than 0xC8 replaces an urgently aborted sub-task through `CreateSubTask`; otherwise the gang taunt
/// `FUN_0065e9a0(ped, target)` runs and the sub-task stays.
#[hook(addr = 0x626260, conv = thiscall, name = "CTaskComplexKillPedOnFoot::ControlSubTask", expect = "6A FF 64 A1 00 00 00 00",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes,x87-transcendental", recs = "carjack")]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub fn kill_ped_on_foot_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 0x40_0000);
    let mut next: u32 = 0xC8;
    'decide: {
        if target(this).is_null() {
            next = 0x516;
            break 'decide;
        }
        if v_get_task_type(sub(this)) == 0xCA {
            let fl = rd::<u8>(this, 0xC);
            if fl & 0x20 != 0 && fl & 0x40 != 0 && rd::<u8>(find_player_wanted(-1), 0x1E) & 4 == 0 {
                v_make_abortable(sub(this), ped, 1, GPtr::NULL);
            }
            return sub(this);
        }
        let limit = rd::<i32>(this, 0x28);
        if limit > 0 && now() > rd::<u32>(this, 0x24).wrapping_add(limit as u32) {
            if !abort_urgent(this, ped) {
                break 'decide;
            }
            let t = task_new(0x20);
            return if t.is_null() { t } else { stand_still_ctor(t, 0, 0, 0, 0x4100_0000) };
        }
        let fl = rd::<u8>(this, 0xC);
        if fl & 2 != 0 {
            if !abort_urgent(this, ped) {
                break 'decide;
            }
            return v_create_first_sub_task(this, ped);
        }
        'types: {
            if fl & 8 != 0 {
                break 'types;
            }
            let health = x(rd::<f32>(target(this), 0x540)).fcom(x(g::<f32>(ZERO).read()));
            if matches!(health, Cmp::Less | Cmp::Equal) {
                if !abort_urgent(this, ped) {
                    break 'types;
                }
                wr::<u8>(this, 0xC, rd::<u8>(this, 0xC) | 8);
                if v_get_task_type(sub(this)) == 0x4BA {
                    return sub(this);
                }
                if rd::<u32>(ped, 0x598) == 6 && ped_is_player(target(this)) == 0 {
                    ped_say(ped, 0xE, 0, 0x3F80_0000, 0, 0, 0);
                }
                if rd::<u32>(ped, 0x478) & 0x1000 != 0 {
                    return create(this, 0x4BA, ped);
                }
                let t = task_new(8);
                return if t.is_null() { t } else { leave_group_ctor(t) };
            }
            if ped_is_player(target(this)) == 0 {
                break 'types;
            }
            if player_wanted_flags(target(this)) & 4 == 0
                && (rd::<u32>(ped, 0x598) != 6 || rd::<u8>(ped, 0x79C) == 0 || rd::<u16>(find_player_wanted(-1), 0x1C) != 0)
            {
                break 'types;
            }
            if v_get_task_type(sub(this)) == 0xCA {
                break 'types;
            }
            let cl = rd::<u8>(this, 0xC);
            next = 0xCA;
            if cl & 0x20 != 0 && player_wanted_flags(target(this)) & 4 != 0 {
                wr::<u8>(this, 0xC, cl | 0x40);
            }
            wr::<u8>(this, 0xC, rd::<u8>(this, 0xC) & 0xFE);
            break 'decide;
        }
        match v_get_task_type(sub(this)) {
            0x3E9 => {
                wr::<u8>(sub(this), 0x30, rd::<u8>(this, 0xC) & 1);
                if !current_weapon_melee(ped) {
                    next = 0x3EA;
                } else {
                    let t = target(this);
                    let inveh = in_vehicle(t);
                    if inveh && matches!(rd::<u32>(rd::<GPtr<u8>>(t, 0x58C), 0x594), 3 | 4) {
                        next = 0x3EB;
                    } else if inveh {
                        let v = rd::<GPtr<u8>>(t, 0x58C);
                        if !v.is_null()
                            && rd::<u32>(v, 0x590) != 5
                            && rd::<u32>(ped, 0x46C) & 0x20_0000 == 0
                            && vehicle_can_ped_open_locks(v, ped) != 0
                        {
                            next = if update_room_flag(this) { 0x2BF } else { 0x3EB };
                        }
                    }
                }
            }
            0x2BF => {
                let t = target(this);
                let v = rd::<GPtr<u8>>(t, 0x58C);
                if in_vehicle(t)
                    && !v.is_null()
                    && (vehicle_is_driver(v, t) != 0 || {
                        let t = target(this);
                        vehicle_is_passenger(rd::<GPtr<u8>>(t, 0x58C), t) != 0
                    })
                {
                    let v = rd::<GPtr<u8>>(target(this), 0x58C);
                    // sqrt(x * x + y * y) of the move speed (+0x44, +0x48), x87
                    let (sx, sy) = (rd::<f32>(v, 0x44), rd::<f32>(v, 0x48));
                    if speed_cmp(sx, sy, g::<f32>(SLOW_SPEED).read()) == Cmp::Greater {
                        next = 0x3EB;
                    } else if timer_is_out_of_time(this.byte_add(0x2C)) != 0 {
                        let t = target(this);
                        let v = rd::<GPtr<u8>>(t, 0x58C);
                        if !v.is_null()
                            && (vehicle_is_passenger(v, t) != 0 || {
                                let t = target(this);
                                vehicle_is_driver(rd::<GPtr<u8>>(t, 0x58C), t) != 0
                            })
                        {
                            if !update_room_flag(this) {
                                next = 0x3EB;
                            }
                            restart_timer(this);
                        }
                    }
                }
            }
            0x3EA => {
                wr::<u8>(sub(this), 0x33, rd::<u8>(this, 0xC) & 1);
                if current_weapon_melee(ped) && ped_is_player(ped) == 0 {
                    next = 0x3E9;
                } else {
                    let slot = isize::from(rd::<i8>(ped, 0x718));
                    if rd::<u32>(ped, 0x5AC + slot * 0x1C) == 0 && ped_is_player(ped) == 0 {
                        let mut i = 0u32;
                        while i < 13 {
                            if rd::<i32>(ped, 0x5AC + i as isize * 0x1C) > 0 {
                                ped_set_current_weapon_slot(ped, i);
                                break;
                            }
                            i += 1;
                        }
                        if i == 13 {
                            ped_set_current_weapon_type(ped, 0);
                            next = 0x3E9;
                        }
                    }
                }
                let t = target(this);
                let v = rd::<GPtr<u8>>(t, 0x58C);
                if in_vehicle(t)
                    && !v.is_null()
                    && (vehicle_is_driver(v, t) != 0 || {
                        let t = target(this);
                        vehicle_is_passenger(rd::<GPtr<u8>>(t, 0x58C), t) != 0
                    })
                {
                    let v = rd::<GPtr<u8>>(target(this), 0x58C);
                    let (sx, sy) = (rd::<f32>(v, 0x44), rd::<f32>(v, 0x48));
                    let vt = rd::<u32>(v, 0x594);
                    let class = rd::<u32>(v, 0x590);
                    if vt == 4 || vt == 3 {
                        next = 0x3EB;
                    } else if class != 9
                        && class != 5
                        && rd::<u32>(ped, 0x46C) & 0x20_0000 == 0
                        && matches!(speed_cmp(sx, sy, g::<f32>(SLOW_SPEED).read()), Cmp::Less | Cmp::Equal)
                        && vehicle_can_ped_open_locks(v, ped) != 0
                    {
                        next = if update_room_flag(this) { 0x2BF } else { 0x3EB };
                    }
                }
            }
            0x3EB => {
                let t = target(this);
                if !in_vehicle(t) {
                    next = if current_weapon_melee(ped) { 0x3E9 } else { 0x3EA };
                } else {
                    let v = rd::<GPtr<u8>>(t, 0x58C);
                    if !v.is_null() && is_vehicle_healthy(v) == 0 {
                        next = 0x4BA;
                    } else if rd::<u32>(target(this), 0x46C) & 0x2000_0000 == 0 {
                        let v = rd::<GPtr<u8>>(target(this), 0x58C);
                        let (sx, sy) = (rd::<f32>(v, 0x44), rd::<f32>(v, 0x48));
                        let vt = rd::<u32>(v, 0x594);
                        let class = rd::<u32>(v, 0x590);
                        if vt != 4
                            && vt != 3
                            && class != 9
                            && class != 5
                            && rd::<u32>(ped, 0x46C) & 0x20_0000 == 0
                            && matches!(speed_cmp(sx, sy, g::<f32>(SLOW_SPEED).read()), Cmp::Less | Cmp::Equal)
                            && vehicle_can_ped_open_locks(v, ped) != 0
                        {
                            if rd::<u8>(this, 0xC) & 4 != 0 {
                                next = 0x2BF;
                            } else if timer_is_out_of_time(this.byte_add(0x2C)) != 0 {
                                if update_room_flag(this) {
                                    next = 0x2BF;
                                }
                                restart_timer(this);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        wr::<u8>(this, 0xC, rd::<u8>(this, 0xC) & 0xFE);
    }
    let t = target(this);
    if !t.is_null() {
        let a = rd::<GPtr<u8>>(t, 0x584);
        if !a.is_null() {
            let b = rd::<GPtr<u8>>(ped, 0x584);
            if !b.is_null() && rd::<u8>(a, 0x2F) != rd::<u8>(b, 0x2F) {
                let mut ev = [0u32; 8];
                let e = buf(&mut ev);
                area_codes_ctor(e, t);
                event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e, 0);
                area_codes_dtor(e);
            }
        }
    }
    if next != 0xC8 && abort_urgent(this, ped) {
        return create(this, next, ped);
    }
    let t = target(this);
    if !t.is_null() {
        taunt_o(ped, t);
    }
    sub(this)
}

/// `CTaskComplexKillPedOnFoot::CreateNextSubTask(CPed*)` (0x62B150, `ret 4`). No target → 0x516. By the finished
/// sub-task: 0xC8 / 0xCB / 0x3E9 / 0x3EA / 0x3EB → 0x516; 0x2CF → 0x2C0; 0x4BA and others → NULL. 0xCA: a cop with
/// +0x79C set → 0x3EA while the player's wanted level (+0x1C, 16 bits) is above 0, else 0xCA again; flags 0x20 and
/// 0x40 with the player no longer wanted-flagged → flag 0x40 cleared and the task's `CreateFirstSubTask`; else 0x516.
/// 0x2BF: the drag task's +0x10 bit 2 → 0x3EB, else 0x3E9 / 0x3EA by the ped's weapon. 0x2C0 (left the car): still in
/// a vehicle → 0x2CF; else a gang/cop ped near a player target arms itself (see `maybe_arm`; the distance is the
/// float `|target - ped|`, z² + y² + x² in x87 order) and 0x3E9 / 0x3EA by weapon.
#[hook(addr = 0x62B150, conv = thiscall, name = "CTaskComplexKillPedOnFoot::CreateNextSubTask", expect = "83 EC 0C 53 56 8B F1",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes,x87-transcendental", recs = "carjack")]
pub fn kill_ped_on_foot_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if target(this).is_null() {
        return create(this, 0x516, ped);
    }
    let by_weapon = |this: GPtr<u8>| if current_weapon_melee(ped) { create(this, 0x3E9, ped) } else { create(this, 0x3EA, ped) };
    match v_get_task_type(sub(this)) {
        0xC8 | 0xCB | 0x3E9 | 0x3EA | 0x3EB => create(this, 0x516, ped),
        0x2CF => create(this, 0x2C0, ped),
        0xCA => {
            if rd::<u32>(ped, 0x598) == 6 && rd::<u8>(ped, 0x79C) != 0 {
                return if rd::<u16>(find_player_wanted(-1), 0x1C) > 0 { create(this, 0x3EA, ped) } else { create(this, 0xCA, ped) };
            }
            let fl = rd::<u8>(this, 0xC);
            if fl & 0x20 != 0 && fl & 0x40 != 0 && rd::<u8>(find_player_wanted(-1), 0x1E) & 4 == 0 {
                wr::<u8>(this, 0xC, rd::<u8>(this, 0xC) & 0xBF);
                return v_create_first_sub_task(this, ped);
            }
            create(this, 0x516, ped)
        }
        0x2BF => {
            if rd::<u8>(sub(this), 0x10) & 4 != 0 {
                create(this, 0x3EB, ped)
            } else {
                by_weapon(this)
            }
        }
        0x2C0 => {
            let player = if ped_is_player(target(this)) != 0 { target(this) } else { GPtr::NULL };
            if in_vehicle(ped) {
                return create(this, 0x2CF, ped);
            }
            let (tp, pp) = (pos_ptr(target(this)), pos_ptr(ped));
            let d: [f32; 3] = core::array::from_fn(|k| f32_of(x(rd::<f32>(tp, k as isize * 4)).fsub_m(rd::<f32>(pp, k as isize * 4))));
            maybe_arm(this, ped, player, false, || {
                x(d[2]).fmul_m(d[2]).fadd(x(d[1]).fmul_m(d[1])).fadd(x(d[0]).fmul_m(d[0])).fsqrt().fcom(x(g::<f32>(GANG_GUN_DIST).read()))
            });
            by_weapon(this)
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexKillPedOnFoot::CreateFirstSubTask(CPed*)` (0x62B490, `ret 4`; catalogue `FUN_0062b490`). Flags: bit 1
/// cleared, bit 2 set. No target: ped +0x474 loses 0x400000 (through the protection fragment 0x409BE9), NULL. A dead
/// target (health ≤ 0, `fcomp` C0/C3) sets flag 8. The distance `|target - ped|` (x87, rounded to a float); a ped in
/// a vehicle → 0x2C0. A gang/cop ped near a player target arms itself (`maybe_arm`). A target in a vehicle it drives or
/// rides in: flag 4 set; a vehicle of type 3/4, a ped with +0x46C bit 0x200000, a vehicle faster than 0.1, locks the
/// ped cannot open, a class-9 vehicle with a non-melee weapon, a boat, or no room to leave by the target's exit door
/// (flag 4 := room) → 0x3EB; else 0x3EB when the target has +0x46C bit 0x20000000, 0x2BF otherwise. A target on foot:
/// 0x3E9 / 0x3EA by the ped's weapon. Every created sub-task is followed by `ped->DropEntityThatThisPedIsHolding(true)`.
#[hook(addr = 0x62B490, conv = thiscall, name = "CTaskComplexKillPedOnFoot::CreateFirstSubTask", expect = "83 EC 08 53 56 8B F1",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes,x87-transcendental", recs = "carjack")]
pub fn kill_ped_on_foot_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let fl = (rd::<u8>(this, 0xC) & 0xFD) | 4;
    wr::<u8>(this, 0xC, fl);
    let t = target(this);
    if t.is_null() {
        wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) & 0xFFBF_FFFF);
        return GPtr::NULL;
    }
    if matches!(x(rd::<f32>(t, 0x540)).fcom(x(g::<f32>(ZERO).read())), Cmp::Less | Cmp::Equal) {
        wr::<u8>(this, 0xC, fl | 8);
    }
    let (tp, pp) = (pos_ptr(t), pos_ptr(ped));
    let d: [X87; 3] = core::array::from_fn(|k| x(rd::<f32>(tp, k as isize * 4)).fsub_m(rd::<f32>(pp, k as isize * 4)));
    let dist = f32_of(d[2].fmul(d[2]).fadd(d[1].fmul(d[1])).fadd(d[0].fmul(d[0])).fsqrt());
    let player = if ped_is_player(target(this)) != 0 { target(this) } else { GPtr::NULL };
    let ty = 'ty: {
        if rd::<u32>(ped, 0x58C) != 0 && in_vehicle(ped) {
            break 'ty 0x2C0;
        }
        maybe_arm(this, ped, player, true, || x(dist).fcom(x(g::<f32>(GANG_GUN_DIST).read())));
        let t = target(this);
        let v = rd::<GPtr<u8>>(t, 0x58C);
        if !in_vehicle(t)
            || v.is_null()
            || (vehicle_is_driver(v, t) == 0 && {
                let t = target(this);
                vehicle_is_passenger(rd::<GPtr<u8>>(t, 0x58C), t) == 0
            })
        {
            break 'ty if current_weapon_melee(ped) { 0x3E9 } else { 0x3EA };
        }
        let v = rd::<GPtr<u8>>(target(this), 0x58C);
        let (sx, sy) = (rd::<f32>(v, 0x44), rd::<f32>(v, 0x48));
        wr::<u8>(this, 0xC, rd::<u8>(this, 0xC) | 4);
        let vt = rd::<u32>(rd::<GPtr<u8>>(target(this), 0x58C), 0x594);
        if vt == 3 || vt == 4 || rd::<u32>(ped, 0x46C) & 0x20_0000 != 0 {
            break 'ty 0x3EB;
        }
        // sqrt(y * y + x * x), x87
        if x(sy).fmul_m(sy).fadd(x(sx).fmul_m(sx)).fsqrt().fcom(x(g::<f32>(SLOW_SPEED).read())) == Cmp::Greater {
            break 'ty 0x3EB;
        }
        if vehicle_can_ped_open_locks(rd::<GPtr<u8>>(target(this), 0x58C), ped) == 0 {
            break 'ty 0x3EB;
        }
        let v = rd::<GPtr<u8>>(target(this), 0x58C);
        if rd::<u32>(v, 0x590) == 9 && !current_weapon_melee(ped) {
            break 'ty 0x3EB;
        }
        if rd::<u32>(rd::<GPtr<u8>>(target(this), 0x58C), 0x590) == 5 || !update_room_flag(this) {
            break 'ty 0x3EB;
        }
        if rd::<u32>(target(this), 0x46C) & 0x2000_0000 != 0 { 0x3EB } else { 0x2BF }
    };
    let r = create(this, ty, ped);
    ped_drop_entity_held(ped, 1);
    r
}

/// `FUN_0065e9a0(CPed* ped, CPed* target)` (0x65E9A0, cdecl): with both peds, a gang ped (type 7..0x10) facing another
/// gang ped or player 0 says, by the target's type, 0x11 (7), 0x12 (9) or 0x13 (0xE) (`Say(id, 0, 1.0, 0, 0, 0)`).
#[hook(addr = 0x65E9A0, conv = cdecl, name = "FUN_0065e9a0", expect = "57 8B 7C 24 08 85 FF",
       unverified = "carjack-1", route = "L2", hazards = "none", recs = "carjack")]
pub fn kill_ped_on_foot_taunt(ped: GPtr<u8>, target: GPtr<u8>) {
    if ped.is_null() || target.is_null() {
        return;
    }
    let pt = rd::<i32>(ped, 0x598);
    if !(7..=0x10).contains(&pt) {
        return;
    }
    let tt = rd::<i32>(target, 0x598);
    if !(7..=0x10).contains(&tt) && target != find_player_ped(0) {
        return;
    }
    let id = match rd::<i32>(target, 0x598) {
        7 => 0x11,
        9 => 0x12,
        0xE => 0x13,
        _ => return,
    };
    ped_say(ped, id, 0, 0x3F80_0000, 0, 0, 0);
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_KILL_PED_ON_FOOT_DTOR,
    HOOK_KILL_PED_ON_FOOT_CREATE_SUB_TASK,
    HOOK_KILL_PED_ON_FOOT_CONTROL_SUB_TASK,
    HOOK_KILL_PED_ON_FOOT_CREATE_NEXT_SUB_TASK,
    HOOK_KILL_PED_ON_FOOT_CREATE_FIRST_SUB_TASK,
    HOOK_KILL_PED_ON_FOOT_TAUNT,
];
