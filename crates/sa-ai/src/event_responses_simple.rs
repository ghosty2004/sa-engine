//! The short `CEventHandler::Compute*Response` functions (batch `intro-ai-2`) and the static helpers they share
//! (`RegisterKill`, `IsKillTaskAppropriate`). Ported from the 1.0 US bytes (gta-reversed `EventHandler.cpp` as a
//! reading aid; the bytes win: several responses leave +0x24 untouched where gta-reversed writes NULL, and a few
//! constructors are inlined). Unverified (docs/verify/HOWTO.md).
//!
//! `CEventHandler`: +0 the ped, +0x24 the event response task, +0x2C a "say" task, +0x30 the physical response
//! task. Every response is `thiscall (event, active, simplest)`, `ret 0xC`. Event +0xE is the response task type
//! (i16). A failed task allocation writes NULL where the original writes the response.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEvent::GetEventType` (vtable +4)
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CEvent::GetSourceEntity` (vtable +0x28)
    fn v_event_source(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x28 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::IsGoToTask(CTask*)` (0x61A360, static)
    fn task_is_go_to_task(t: GPtr<u8>) -> u8 = cdecl 0x61A360 => unported;
    /// `CTaskSimple::CTaskSimple` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::Say(context, offset, volume, b, b, b)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, ctx: u32, offset: u32, vol: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CPed::RestartNonPartialAnims` (0x5DED50)
    fn ped_restart_non_partial_anims(p: GPtr<u8>) = thiscall 0x5DED50 => unported;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0, `ret 0xC`; the vector by value)
    fn physical_apply_move_force(p: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x5429F0 => unported;
    /// `CVehicle::IsDriver(CPed*)` (0x6D1C40, `ret 4`)
    fn vehicle_is_driver(v: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x6D1C40 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0, `ret 4`)
    fn intel_find_task_by_type(i: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `CPedIntelligence::GetTaskDuck(bool)` (0x6010A0, `ret 4`)
    fn intel_get_task_duck(i: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x6010A0 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810, `ret 4`)
    fn tm_get_task_secondary(tm: GPtr<u8>, slot: i32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0, `ret 0xC`)
    fn tm_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: i32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CTaskComplexWander::GetWanderTaskByPedType(CPed*)` (0x673D00, static)
    fn wander_get_wander_task_by_ped_type(ped: GPtr<u8>) -> GPtr<u8> = cdecl 0x673D00 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CGeneral::GetRandomNumberInRange<int32>(min, max)` (0x407180; draws its own rand)
    fn general_random_int_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CInterestingEvents::Add(int32 type, CEntity*)` (0x602590, `ret 8`)
    fn interesting_events_add(ie: GPtr<u8>, ty: u32, e: GPtr<u8>) = thiscall 0x602590 => unported;
    /// `IKChainManager_c::LookAt(...)` (0x618970, `ret 0x2C`)
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, pos: u32, torso: u32,
                  speed: u32, blend: i32, prio: i32, b: u32) = thiscall 0x618970 => unported;
    /// `CPedGeometryAnalyser::ComputePedHitSide(const CPed&, const CVector&)` (0x5F1E70, cdecl)
    fn geometry_compute_ped_hit_side(ped: GPtr<u8>, pos: GPtr<u8>) -> i32 = cdecl 0x5F1E70 => unported;
    /// `CPedAttractorManager* GetPedAttractorManager()` (0x5EE190)
    fn get_ped_attractor_manager() -> GPtr<u8> = cdecl 0x5EE190 => unported;
    /// `CPedAttractorManager::HasEmptySlot(const C2dEffect*, const CEntity*)` (0x5EBB00, `ret 8`)
    fn attractor_manager_has_empty_slot(m: GPtr<u8>, effect: GPtr<u8>, entity: GPtr<u8>) -> u8 = thiscall 0x5EBB00 => unported;
    /// `CWeapon::IsTypeMelee` (0x73B1C0)
    fn weapon_is_type_melee(w: GPtr<u8>) -> u8 = thiscall 0x73B1C0 => unported;
    /// `CDarkel::RegisterKillByPlayer(const CPed*, eWeaponType, bool, int32)` (0x43DCD0, cdecl)
    fn darkel_register_kill_by_player(ped: GPtr<u8>, w: u32, head: u32, n: u32) = cdecl 0x43DCD0 => unported;
    /// `CDarkel::RegisterKillNotByPlayer(const CPed*, eWeaponType)` (0x43D210, cdecl)
    fn darkel_register_kill_not_by_player(ped: GPtr<u8>, w: u32) = cdecl 0x43D210 => unported;
    /// `CPedGroups::RegisterKillByPlayer(CPed*, CPlayerPed*)` (0x5F7E30, cdecl; reached by a tail jump with the
    /// caller's own arguments)
    fn ped_groups_register_kill_by_player(ped: GPtr<u8>, killer: GPtr<u8>, w: u32, head: u32) = cdecl 0x5F7E30 => unported;
    /// `CStats::IncrementStat(uint16, float)` (0x55C180, cdecl)
    fn stats_increment_stat(stat: u32, v: f32) = cdecl 0x55C180 => unported;
    /// `CWanted::SetWantedLevelNoDrop(int32)` (0x562570, `ret 4`)
    fn wanted_set_wanted_level_no_drop(w: GPtr<u8>, l: i32) = thiscall 0x562570 => unported;
    /// `CTaskSimpleDuck::RestartTask(CPed*)` (0x692390, `ret 4`)
    fn duck_restart_task(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x692390 => unported;
    /// `CEventHandler::ComputeKnockOffBikeResponse(CEvent*, CTask*, CTask*)` (0x4B9FF0, `ret 0xC`)
    fn compute_knock_off_bike_response(h: GPtr<u8>, e: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = thiscall 0x4B9FF0 => unported;
    /// `FUN_004ace80(CPed*)` (thiscall on the event, `ret 4`; result unused here)
    fn object_collision_4ace80(e: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x4ACE80 => unported;
    // Task constructors (thiscall; the task first).
    fn walk_round_object_ctor(t: GPtr<u8>, ms: i32, target: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = thiscall 0x655020 => unported;
    fn simple_dead_ctor(t: GPtr<u8>, time: u32, b: u32) -> GPtr<u8> = thiscall 0x630590 => unported;
    fn get_up_and_stand_still_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x678130 => unported;
    fn stand_still_ctor(t: GPtr<u8>, time: i32, a: u32, b: u32, f: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    fn smart_flee_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, b: u32, d: u32, time: u32, period: u32, f: u32) -> GPtr<u8> = thiscall 0x65C430 => unported;
    fn use_effect_ctor(t: GPtr<u8>, effect: GPtr<u8>, entity: GPtr<u8>) -> GPtr<u8> = thiscall 0x6321F0 => unported;
    fn investigate_disturbance_ctor(t: GPtr<u8>, pos: GPtr<u8>, e: GPtr<u8>) -> GPtr<u8> = thiscall 0x66C720 => unported;
    fn use_attractor_ctor(t: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x6346F0 => unported;
    fn prostitute_solicit_ctor(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x661A60 => unported;
    fn steal_car_ctor(t: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> = thiscall 0x63B7D0 => unported;
    fn die_ctor(t: GPtr<u8>, wt: u32, group: u32, anim: u32, delta: u32, speed: u32, stealth: u32, falling: u32, dir: u32, rail: u32) -> GPtr<u8> = thiscall 0x630040 => unported;
    fn dive_from_attached_ctor(t: GPtr<u8>, time: u32) -> GPtr<u8> = thiscall 0x492E20 => unported;
    fn duck_ctor(t: GPtr<u8>, ty: u32, time: u32, counter: i32) -> GPtr<u8> = thiscall 0x691FC0 => unported;
    fn kill_ped_on_foot_ctor(t: GPtr<u8>, ped: GPtr<u8>, time: i32, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x620E30 => unported;
    fn player_on_fire_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x633560 => unported;
    fn complex_on_fire_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x633390 => unported;
    fn fall_and_get_up_ctor2(t: GPtr<u8>, dir: i32, time: i32) -> GPtr<u8> = thiscall 0x678700 => unported;
    fn in_water_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x6350D0 => unported;
    fn interior_use_info_ctor(t: GPtr<u8>, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x675A50 => unported;
    fn pass_object_ctor(t: GPtr<u8>, e: u32, b: u32) -> GPtr<u8> = thiscall 0x6611B0 => unported;
    fn police_pursuit_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x68BA70 => unported;
    fn leave_car_ctor(t: GPtr<u8>, veh: GPtr<u8>, a: i32, b: i32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    fn walk_round_fire_ctor(t: GPtr<u8>, ms: i32, pos: GPtr<u8>, radius: u32, target: GPtr<u8>) -> GPtr<u8> = thiscall 0x655720 => unported;
    fn flee_point_ctor(t: GPtr<u8>, pos: GPtr<u8>, b: u32, a: u32, c: u32) -> GPtr<u8> = thiscall 0x65B390 => unported;
    fn smart_flee_point_ctor(t: GPtr<u8>, pos: GPtr<u8>, b: u32, a: u32, c: u32) -> GPtr<u8> = thiscall 0x65BD20 => unported;
    fn wait_until_area_codes_match_ctor(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x635540 => unported;
    fn extinguish_fires_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x6975C0 => unported;
    fn gang_join_respond_ctor(t: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x6616F0 => unported;
    fn car_drive_mission_flee_scene_ctor(t: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B8920 => unported;
    fn seek_entity_standard_ctor(t: GPtr<u8>, e: GPtr<u8>, a: u32, b: u32, c: u32, d: u32, f: u32, g: u32, h: u32) -> GPtr<u8> = thiscall 0x46AC10 => unported;
}

/// `g_ikChainMan`
const IK_CHAIN_MANAGER: usize = 0xC15448;
/// `g_InterestingEvents`
const INTERESTING_EVENTS: usize = 0xC0B058;
/// `CWorld::PlayerInFocus` (u8) and `CWorld::Players` (0x190 bytes each)
const PLAYER_IN_FOCUS: usize = 0xB7CD74;
const PLAYERS: usize = 0xB7CD98;
/// 5.0
const K_FIVE: usize = 0x858C80;
/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7CB5C;
/// The water cannon's speed factor (0x858CC8) and the 2D speed cap (0x858CC4)
const K_CANNON_SPEED: usize = 0x858CC8;
const K_CANNON_CAP: usize = 0x858CC4;
/// 0.5
const K_HALF: usize = 0x858B8C;
/// The flee tasks' default arguments (data / static-initialised globals, passed as the original reads them).
const FLEE_SAFE_DIST: usize = 0xC18CEC;
const FLEE_TIME: usize = 0x86F674;
const FLEE_PERIOD: usize = 0x86F678;
const FLEE_SHIFT: usize = 0xC18CF0;
const FLEE_POINT_A: usize = 0x86F648;
const FLEE_POINT_B: usize = 0x86F64C;
const SMART_FLEE_POINT_A: usize = 0xC18CE8;
const SMART_FLEE_POINT_B: usize = 0x86F66C;
/// `CTaskComplexSeekEntity` defaults read by `ComputePedToChaseResponse`
const SEEK_A: usize = 0x85BACC;
const SEEK_B: usize = 0x859E30;
const SEEK_C: usize = 0x859E34;
/// Vtables of the constructors the original inlines
const VT_SAY_LIKE: u32 = 0x85A1DC;
const VT_ATTRACTOR_RUN: u32 = 0x85B904;
const VT_ATTRACTOR_SPRINT: u32 = 0x85B930;
const VT_STAND_STILL_ESCALATOR: u32 = 0x85B8DC;
/// "CompSexyVehResp"-style look-at names
const S_SEXY_VEHICLE: u32 = 0x85BA14;

fn gu(a: usize) -> u32 {
    g::<u32>(a).read()
}

fn ped(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0)
}

fn intel(p: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(p, 0x47C)
}

fn task_type(e: GPtr<u8>) -> i32 {
    i32::from(rd::<i16>(e, 0xE))
}

fn in_vehicle(p: GPtr<u8>) -> bool {
    rd::<u32>(p, 0x46C) & 0x100 != 0
}

fn new_task(size: u32, ctor: impl FnOnce(GPtr<u8>) -> GPtr<u8>) -> GPtr<u8> {
    let t = task_new(size);
    if t.is_null() { t } else { ctor(t) }
}

fn set(this: GPtr<u8>, t: GPtr<u8>) {
    wr::<GPtr<u8>>(this, 0x24, t);
}

fn smart_flee_default(other: GPtr<u8>) -> GPtr<u8> {
    new_task(0x40, |t| smart_flee_entity_ctor(t, other, 0, gu(FLEE_SAFE_DIST), gu(FLEE_TIME), gu(FLEE_PERIOD), gu(FLEE_SHIFT)))
}

fn kill_on_foot(other: GPtr<u8>) -> GPtr<u8> {
    new_task(0x38, |t| kill_ped_on_foot_ctor(t, other, -1, 0, 0, 0, 1))
}

/// `CEventHandler::ComputeObjectCollisionResponse` (0x4B92B0). With a go-to `simplest` and a move state (+0xE) other
/// than 1: `FUN_004ace80(event, ped)` (result unused), then with the object (+0x14) `CTaskComplexWalkRoundObject(ms,
/// simplest +0xC, object)`. Else no write.
#[hook(addr = 0x4b92b0, conv = thiscall, name = "CEventHandler::ComputeObjectCollisionResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_object_collision_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, simplest: GPtr<u8>) {
    if simplest.is_null() || task_is_go_to_task(simplest) == 0 {
        return;
    }
    let ms = i32::from(rd::<i16>(e, 0xE));
    if ms == 1 {
        return;
    }
    let obj = rd::<GPtr<u8>>(e, 0x14);
    object_collision_4ace80(e, ped(this));
    if obj.is_null() {
        return;
    }
    set(this, new_task(0x54, |t| walk_round_object_ctor(t, ms, simplest.byte_add(0xC), obj)));
}

/// `CEventHandler::RegisterKill(const CPed*, const CEntity*, eWeaponType, bool)` (0x4B9340, static cdecl). Nothing
/// without the ped. A player ped killer other than the ped: the focused player's +0x140 (money) += 10 and +0x148
/// += 5.0 (x87), `CDarkel::RegisterKillByPlayer(ped, weapon, headShot, killer +0x598)`, then (tail jump with the
/// same arguments) `CPedGroups::RegisterKillByPlayer`. A vehicle killer that is the player's vehicle:
/// `CStats::IncrementStat(0x79, 1.0)`. Otherwise (and without a killer) `CDarkel::RegisterKillNotByPlayer(ped,
/// weapon)`.
#[hook(addr = 0x4b9340, conv = cdecl, name = "CEventHandler::RegisterKill", expect = "57 8B 7C 24 08",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_register_kill(p: GPtr<u8>, killer: GPtr<u8>, weapon: u32, head_shot: u32) {
    if p.is_null() {
        return;
    }
    if !killer.is_null() {
        if rd::<u8>(killer, 0x36) & 7 == 3 && ped_is_player(killer) != 0 && killer != p {
            let pl = GPtr::<u8>::from_addr(PLAYERS + 0x190 * usize::from(g::<u8>(PLAYER_IN_FOCUS).read()));
            wr::<u32>(pl, 0x140, rd::<u32>(pl, 0x140).wrapping_add(10));
            wr::<f32>(pl, 0x148, f32_of(x(rd::<f32>(pl, 0x148)).fadd_m(g::<f32>(K_FIVE).read())));
            darkel_register_kill_by_player(p, weapon, head_shot, rd::<u32>(killer, 0x598));
            ped_groups_register_kill_by_player(p, killer, weapon, head_shot);
            return;
        }
        if rd::<u8>(killer, 0x36) & 7 == 2 && killer == find_player_vehicle(-1, 0) {
            stats_increment_stat(0x79, 1.0);
            return;
        }
    }
    darkel_register_kill_not_by_player(p, weapon);
}

/// `CEventHandler::ComputeDeathResponse` (0x4B9400): +0x24 = `CTaskSimpleDead(event +0x10, event +0xC)`. The bool
/// is the event's byte stored over the low byte of the event argument slot and pushed as that whole dword (its upper
/// bytes are the event pointer's), as the original.
#[hook(addr = 0x4b9400, conv = thiscall, name = "CEventHandler::ComputeDeathResponse", expect = "6A FF 68 4B A9 83 00",
       this = "CEventHandler", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_death_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let time = rd::<u32>(e, 0x10);
    let b = (e.addr() as u32 & 0xFFFF_FF00) | u32::from(rd::<u8>(e, 0xC));
    set(this, new_task(0x10, |t| simple_dead_ctor(t, time, b)));
}

/// `CEventHandler::ComputeReviveResponse` (0x4B97B0): the ped: health (+0x540) = 100.0, +0x1C |= 1, +0x470 &=
/// ~0x10 and ~0x80000000, +0x474 &= ~0x100, state (+0x530) = 1, +0x40 &= ~0x20000000, `RestartNonPartialAnims`.
/// +0x24 = `CTaskComplexGetUpAndStandStill`; +0x2C = an inlined 0x1C-byte simple task (`CTaskSimple` constructor,
/// vtable 0x85A1DC, +8 = 0xB3, +0xC = -1, +0x10 / +0x14 = 0, bytes +0x18 / +0x19 = 0). With no primary tasks in
/// slots 3 and 4 (intelligence +0x10 / +0x14): `SetTask(GetWanderTaskByPedType(ped), 4, false)`.
#[hook(addr = 0x4b97b0, conv = thiscall, name = "CEventHandler::ComputeReviveResponse", expect = "6A FF 68 C6 A9 83 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_revive_response(this: GPtr<u8>, _e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    wr::<u32>(ped(this), 0x540, 0x42C8_0000);
    let p = ped(this);
    wr::<u32>(p, 0x1C, rd::<u32>(p, 0x1C) | 1);
    wr::<u32>(p, 0x470, rd::<u32>(p, 0x470) & !0x10);
    wr::<u32>(p, 0x470, rd::<u32>(p, 0x470) & 0x7FFF_FFFF);
    wr::<u32>(p, 0x474, rd::<u32>(p, 0x474) & !0x100);
    wr::<u32>(p, 0x530, 1);
    wr::<u32>(p, 0x40, rd::<u32>(p, 0x40) & !0x2000_0000);
    ped_restart_non_partial_anims(ped(this));
    set(this, new_task(0xC, get_up_and_stand_still_ctor));
    let t = task_new(0x1C);
    if !t.is_null() {
        task_simple_ctor(t);
        wr::<u32>(t, 0, VT_SAY_LIKE);
        wr::<u32>(t, 8, 0xB3);
        wr::<i32>(t, 0xC, -1);
        wr::<u32>(t, 0x10, 0);
        wr::<u32>(t, 0x14, 0);
        wr::<u8>(t, 0x18, 0);
        wr::<u8>(t, 0x19, 0);
    }
    wr::<GPtr<u8>>(this, 0x2C, t);
    let p = ped(this);
    let i = intel(p);
    if rd::<u32>(i, 0x10) == 0 && rd::<u32>(i, 0x14) == 0 {
        let w = wander_get_wander_task_by_ped_type(p);
        tm_set_task(i.byte_add(4), w, 4, 0);
    }
}

/// `CEventHandler::ComputeSexyVehicleResponse` (0x4B9AA0): with the vehicle (+0xC): `g_InterestingEvents.Add(8,
/// vehicle)`, +0x24 = `CTaskSimpleStandStill(5000, false, false, 8.0)`, `LookAt(0x85BA14, ped, vehicle, 5000, -1,
/// NULL, true, 0.25, 500, 3, false)`.
#[hook(addr = 0x4b9aa0, conv = thiscall, name = "CEventHandler::ComputeSexyVehicleResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_sexy_vehicle_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(e, 0xC);
    if veh.is_null() {
        return;
    }
    interesting_events_add(GPtr::from_addr(INTERESTING_EVENTS), 8, veh);
    set(this, new_task(0x20, |t| stand_still_ctor(t, 5000, 0, 0, 0x4100_0000)));
    ik_look_at(GPtr::from_addr(IK_CHAIN_MANAGER), S_SEXY_VEHICLE, ped(this), veh, 5000, -1, 0, 1, 0x3E80_0000, 500, 3, 0);
}

/// `CEventHandler::ComputePedToFleeResponse` (0x4B9B50): with the ped (+0xC): `CPed::Say(0x45, 0, 1.0, 0, 0, 0)`,
/// +0x24 = `CTaskComplexSmartFleeEntity(ped, true, 100.0, -1, <period>, <shift>)`.
#[hook(addr = 0x4b9b50, conv = thiscall, name = "CEventHandler::ComputePedToFleeResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_ped_to_flee_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let other = rd::<GPtr<u8>>(e, 0xC);
    if other.is_null() {
        return;
    }
    ped_say(ped(this), 0x45, 0, 0x3F80_0000, 0, 0, 0);
    set(this, new_task(0x40, |t| smart_flee_entity_ctor(t, other, 1, 0x42C8_0000, u32::MAX, gu(FLEE_PERIOD), gu(FLEE_SHIFT))));
}

/// `CEventHandler::ComputeAttractorResponse` (0x4B9BE0): event +0x14 the effect, +0x18 the entity. An event of type
/// 0x1B without an entity: nothing. Task 200: NULL. Task 0xE9: `HasEmptySlot(effect, entity)` on the attractor
/// manager → `CTaskComplexUseEffect(effect, entity)`, else NULL. Others: nothing.
#[hook(addr = 0x4b9be0, conv = thiscall, name = "CEventHandler::ComputeAttractorResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_attractor_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let effect = rd::<GPtr<u8>>(e, 0x14);
    let entity = rd::<GPtr<u8>>(e, 0x18);
    if v_event_type(e) == 0x1B && entity.is_null() {
        return;
    }
    match task_type(e) {
        200 => set(this, GPtr::NULL),
        0xE9 => {
            let m = get_ped_attractor_manager();
            if attractor_manager_has_empty_slot(m, effect, entity) == 0 {
                set(this, GPtr::NULL);
            } else {
                set(this, new_task(0x24, |t| use_effect_ctor(t, effect, entity)));
            }
        }
        _ => {}
    }
}

/// `CEventHandler::ComputePedThreatBadlyLitResponse` (0x4B9C90): with the ped (+0x14): task 200 → NULL; 0x3A7 →
/// `CTaskComplexInvestigateDisturbance(&event +0x1C, ped)`; others: nothing.
#[hook(addr = 0x4b9c90, conv = thiscall, name = "CEventHandler::ComputePedThreatBadlyLitResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_ped_threat_badly_lit_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let other = rd::<GPtr<u8>>(e, 0x14);
    if other.is_null() {
        return;
    }
    match task_type(e) {
        200 => set(this, GPtr::NULL),
        0x3A7 => set(this, new_task(0x1C, |t| investigate_disturbance_ctor(t, e.byte_add(0x1C), other))),
        _ => {}
    }
}

/// `CEventHandler::ComputePedSoundQuietResponse` (0x4B9D40): with a source entity (vtable +0x28): task 200 → NULL;
/// 0x3A7 → `CTaskComplexInvestigateDisturbance(&event +0x20, source)` (the source read again); others: nothing.
#[hook(addr = 0x4b9d40, conv = thiscall, name = "CEventHandler::ComputePedSoundQuietResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_ped_sound_quiet_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    if v_event_source(e).is_null() {
        return;
    }
    match task_type(e) {
        200 => set(this, GPtr::NULL),
        0x3A7 => set(this, new_task(0x1C, |t| investigate_disturbance_ctor(t, e.byte_add(0x20), v_event_source(e)))),
        _ => {}
    }
}

/// The closest-free-scripted-attractor tasks: the base constructor (`false`), then for the run / sprint variants the
/// inlined derived constructor (vtable and +0xC move state).
fn use_attractor(vt: Option<(u32, u32)>) -> GPtr<u8> {
    let t = task_new(0x10);
    if t.is_null() {
        return t;
    }
    use_attractor_ctor(t, 0);
    if let Some((vt, ms)) = vt {
        wr::<u32>(t, 0, vt);
        wr::<u32>(t, 0xC, ms);
    }
    t
}

/// `CEventHandler::ComputePedFriendResponse` (0x4B9DD0): task 200 → NULL; 0x102 / 0x105 / 0x106 → the
/// closest-free-scripted-attractor task (walk / run (move state 6) / sprint (7)); 0x51D → unless the ped has a task
/// 0x51D or is in a vehicle, `CTaskComplexProstituteSolicit(event +0x14)`; others: nothing.
#[hook(addr = 0x4b9dd0, conv = thiscall, name = "CEventHandler::ComputePedFriendResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_ped_friend_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    match task_type(e) {
        200 => set(this, GPtr::NULL),
        0x102 => set(this, use_attractor(None)),
        0x105 => set(this, use_attractor(Some((VT_ATTRACTOR_RUN, 6)))),
        0x106 => set(this, use_attractor(Some((VT_ATTRACTOR_SPRINT, 7)))),
        0x51D => {
            if !intel_find_task_by_type(intel(ped(this)), 0x51D).is_null() || in_vehicle(ped(this)) {
                return;
            }
            let other = rd::<GPtr<u8>>(e, 0x14);
            set(this, new_task(0x30, |t| prostitute_solicit_ctor(t, other)));
        }
        _ => {}
    }
}

/// `CEventHandler::ComputeVehicleToStealResponse` (0x4B9F80): on foot with the vehicle (+0xC):
/// `CTaskComplexStealCar(vehicle)`.
#[hook(addr = 0x4b9f80, conv = thiscall, name = "CEventHandler::ComputeVehicleToStealResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_vehicle_to_steal_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    if in_vehicle(ped(this)) {
        return;
    }
    let veh = rd::<GPtr<u8>>(e, 0xC);
    if veh.is_null() {
        return;
    }
    set(this, new_task(0x20, |t| steal_car_ctor(t, veh)));
}

/// `CEventHandler::ComputeVehicleDiedResponse` (0x4BA8B0): with the vehicle (+0xC): the ped's +0x40 |= 0x20000000,
/// health (+0x540) = 0. In a bike-like vehicle (+0x590 == 9 or +0x594 == 2) → `ComputeKnockOffBikeResponse(event,
/// active, simplest)`; else +0x24 = `CTaskComplexDie(0x33, 0, 0xF, 4.0, 0, 0, 0, 0, 0)`.
#[hook(addr = 0x4ba8b0, conv = thiscall, name = "CEventHandler::ComputeVehicleDiedResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_vehicle_died_response(this: GPtr<u8>, e: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) {
    if rd::<GPtr<u8>>(e, 0xC).is_null() {
        return;
    }
    wr::<u32>(ped(this), 0x40, rd::<u32>(ped(this), 0x40) | 0x2000_0000);
    wr::<u32>(ped(this), 0x540, 0);
    let p = ped(this);
    if in_vehicle(p) {
        let veh = rd::<GPtr<u8>>(p, 0x58C);
        if !veh.is_null() && (rd::<i32>(veh, 0x590) == 9 || rd::<i32>(veh, 0x594) == 2) {
            compute_knock_off_bike_response(this, e, active, simplest);
            return;
        }
    }
    set(this, new_task(0x28, |t| die_ctor(t, 0x33, 0, 0xF, 0x4080_0000, 0, 0, 0, 0, 0)));
}

fn health_response(this: GPtr<u8>, e: GPtr<u8>) {
    match task_type(e) {
        200 => set(this, GPtr::NULL),
        0x201 => set(this, new_task(0x24, |t| dive_from_attached_ctor(t, 0))),
        _ => {}
    }
}

/// `CEventHandler::ComputeLowHealthResponse` (0x4BA990): task 200 → NULL; 0x201 →
/// `CTaskComplexDiveFromAttachedEntityAndGetUp(0)`; others: nothing.
#[hook(addr = 0x4ba990, conv = thiscall, name = "CEventHandler::ComputeLowHealthResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_low_health_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    health_response(this, e);
}

/// `CEventHandler::ComputeReallyLowHealthResponse` (0x4BAA30): the same code as `ComputeLowHealthResponse`.
#[hook(addr = 0x4baa30, conv = thiscall, name = "CEventHandler::ComputeReallyLowHealthResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_really_low_health_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    health_response(this, e);
}

fn anger_response(this: GPtr<u8>, e: GPtr<u8>) {
    match task_type(e) {
        200 => set(this, GPtr::NULL),
        0x1AB => set(this, new_task(0x28, |t| duck_ctor(t, 0, 0xE0FF, -1))),
        0x38F => set(this, new_task(0x40, |t| {
            let player = find_player_ped(-1);
            smart_flee_entity_ctor(t, player, 0, gu(FLEE_SAFE_DIST), gu(FLEE_TIME), gu(FLEE_PERIOD), gu(FLEE_SHIFT))
        })),
        0x3E8 => set(this, new_task(0x38, |t| {
            let player = find_player_ped(-1);
            kill_ped_on_foot_ctor(t, player, -1, 0, 0, 0, 1)
        })),
        _ => {}
    }
}

/// `CEventHandler::ComputeLowAngerAtPlayerResponse` (0x4BAAD0): task 200 → NULL; 0x1AB → `CTaskSimpleDuck(0, 0xE0FF,
/// -1)`; 0x38F → smart flee from `FindPlayerPed(-1)` (defaults); 0x3E8 → `CTaskComplexKillPedOnFoot(player, -1, 0, 0,
/// 0, true)` (the player looked up after the allocation); others: nothing.
#[hook(addr = 0x4baad0, conv = thiscall, name = "CEventHandler::ComputeLowAngerAtPlayerResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_low_anger_at_player_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    anger_response(this, e);
}

/// `CEventHandler::ComputeHighAngerAtPlayerResponse` (0x4BAC10): the same code as the low-anger response.
#[hook(addr = 0x4bac10, conv = thiscall, name = "CEventHandler::ComputeHighAngerAtPlayerResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_high_anger_at_player_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    anger_response(this, e);
}

/// `CEventHandler::ComputeOnFireResponse` (0x4BAD50): a player without a secondary task in slot 4 → +0x30 (the
/// physical response) = `CTaskSimplePlayerOnFire`; a player with one: nothing; another ped → +0x24 =
/// `CTaskComplexOnFire`.
#[hook(addr = 0x4bad50, conv = thiscall, name = "CEventHandler::ComputeOnFireResponse", expect = "6A FF 68 96 AC 83 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_on_fire_response(this: GPtr<u8>, _e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    if ped_is_player(ped(this)) != 0 {
        if !tm_get_task_secondary(intel(ped(this)).byte_add(4), 4).is_null() {
            return;
        }
        wr::<GPtr<u8>>(this, 0x30, new_task(0x18, player_on_fire_ctor));
        return;
    }
    set(this, new_task(0xC, complex_on_fire_ctor));
}

/// `CEventHandler::ComputeWaterCannonResponse` (0x4BAE30): `Say(0x158, 0, 1.0, 0, 0, 0)`; +0x24 =
/// `CTaskComplexFallAndGetUp(ComputePedHitSide(ped, &event +0x18), 0)`; `ApplyMoveForce((0, 0, 2 * timeStep))`; move
/// speed x / y = `(event +0x18 / +0x1C * k + speed) * 0.5` (x87, stored f32); the 2D speed (`fsqrt`) above the cap
/// scales both by `cap / length`.
#[hook(addr = 0x4bae30, conv = thiscall, name = "CEventHandler::ComputeWaterCannonResponse", expect = "6A FF 68 AB AC 83 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "x87-transcendental", recs = "grove-walk")]
pub fn handler_compute_water_cannon_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let dir = e.byte_add(0x18);
    ped_say(ped(this), 0x158, 0, 0x3F80_0000, 0, 0, 0);
    let side = geometry_compute_ped_hit_side(ped(this), dir);
    set(this, new_task(0x18, |t| fall_and_get_up_ctor2(t, side, 0)));
    let ts = x(g::<f32>(TIME_STEP).read());
    physical_apply_move_force(ped(this), 0.0, 0.0, f32_of(ts.fadd(ts)));
    let k = g::<f32>(K_CANNON_SPEED).read();
    let half = g::<f32>(K_HALF).read();
    for (o, d) in [(0x44isize, 0isize), (0x48, 4)] {
        let p = ped(this);
        wr::<f32>(p, o, f32_of(x(rd::<f32>(dir, d)).fmul_m(k).fadd_m(rd::<f32>(p, o)).fmul_m(half)));
    }
    let p = ped(this);
    let vx = x(rd::<f32>(p, 0x44));
    let vy = x(rd::<f32>(p, 0x48));
    let len = vx.fmul(vx).fadd(vy.fmul(vy)).fsqrt();
    let cap = g::<f32>(K_CANNON_CAP).read();
    if len.fcom(x(cap)).test_any(0x41) {
        return;
    }
    let s = len.fdivr(x(cap));
    wr::<f32>(p, 0x44, f32_of(s.fmul_m(rd::<f32>(p, 0x44))));
    let p = ped(this);
    wr::<f32>(p, 0x48, f32_of(s.fmul_m(rd::<f32>(p, 0x48))));
}

/// `CEventHandler::ComputeInWaterResponse` (0x4BAF80): +0x24 = `CTaskComplexInWater`.
#[hook(addr = 0x4baf80, conv = thiscall, name = "CEventHandler::ComputeInWaterResponse", expect = "6A FF 68 CB AC 83 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_in_water_response(this: GPtr<u8>, _e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    set(this, new_task(0xC, in_water_ctor));
}

/// `CEventHandler::ComputeInteriorUseInfoResponse` (0x4BAFE0): +0x24 = `CTaskInteriorUseInfo(event +0xC, +0x10,
/// +0x14, byte +0x18)`.
#[hook(addr = 0x4bafe0, conv = thiscall, name = "CEventHandler::ComputeInteriorUseInfoResponse", expect = "6A FF 68 EB AC 83 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_interior_use_info_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    set(this, new_task(0x1C, |t| {
        interior_use_info_ctor(t, rd::<u32>(e, 0xC), rd::<u32>(e, 0x10), rd::<u32>(e, 0x14), u32::from(rd::<u8>(e, 0x18)))
    }));
}

/// `CEventHandler::ComputePassObjectResponse` (0x4BB0C0): +0x24 = `CTaskComplexPassObject(event +0xC, byte +0x10)`.
#[hook(addr = 0x4bb0c0, conv = thiscall, name = "CEventHandler::ComputePassObjectResponse", expect = "6A FF 68 2B AD 83 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_pass_object_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    set(this, new_task(0x30, |t| pass_object_ctor(t, rd::<u32>(e, 0xC), u32::from(rd::<u8>(e, 0x10)))));
}

/// `CEventHandler::ComputePlayerWantedLevelResponse` (0x4BB280): +0x24 = `CTaskComplexPolicePursuit`.
#[hook(addr = 0x4bb280, conv = thiscall, name = "CEventHandler::ComputePlayerWantedLevelResponse", expect = "6A FF 68 6B AD 83 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_player_wanted_level_response(this: GPtr<u8>, _e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    set(this, new_task(0x18, police_pursuit_ctor));
}

/// `CEventHandler::ComputeCopCarBeingStolenResponse` (0x4BB740): event +0xC the thief, +0x10 the vehicle; both set,
/// the ped in that vehicle: +0x24 = `CTaskComplexLeaveCar(vehicle, 0, 0, true, false)`; a player thief gets
/// `SetWantedLevelNoDrop(1)` on its wanted record (player data +0x480, `+0`; NULL without player data).
#[hook(addr = 0x4bb740, conv = thiscall, name = "CEventHandler::ComputeCopCarBeingStolenResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_cop_car_being_stolen_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(e, 0x10);
    let thief = rd::<GPtr<u8>>(e, 0xC);
    if thief.is_null() || veh.is_null() {
        return;
    }
    let p = ped(this);
    if rd::<GPtr<u8>>(p, 0x58C) != veh || !in_vehicle(p) {
        return;
    }
    set(this, new_task(0x34, |t| leave_car_ctor(t, veh, 0, 0, 1, 0)));
    if ped_is_player(thief) != 0 {
        let pd = rd::<GPtr<u8>>(thief, 0x480);
        let wanted = if pd.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(pd, 0) };
        wanted_set_wanted_level_no_drop(wanted, 1);
    }
}

/// `CEventHandler::ComputeObjectCollisionPassiveResponse` (0x4BBB90): with the object (+0x14), on foot, move state
/// (+0x18) not 1 and a go-to `simplest`: `CTaskComplexWalkRoundObject(ms, simplest +0xC, object)`.
#[hook(addr = 0x4bbb90, conv = thiscall, name = "CEventHandler::ComputeObjectCollisionPassiveResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_object_collision_passive_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, simplest: GPtr<u8>) {
    let obj = rd::<GPtr<u8>>(e, 0x14);
    if obj.is_null() || in_vehicle(ped(this)) {
        return;
    }
    let ms = rd::<i32>(e, 0x18);
    if ms == 1 || simplest.is_null() || task_is_go_to_task(simplest) == 0 {
        return;
    }
    set(this, new_task(0x54, |t| walk_round_object_ctor(t, ms, simplest.byte_add(0xC), obj)));
}

/// `CEventHandler::ComputeCarUpsideDownResponse` (0x4BBC30): in the event's vehicle (+0xC): +0x24 =
/// `CTaskComplexLeaveCar(vehicle, 0, GetRandomNumberInRange(100, 400), true, false)` (the delay drawn after the
/// allocation).
#[hook(addr = 0x4bbc30, conv = thiscall, name = "CEventHandler::ComputeCarUpsideDownResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_car_upside_down_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let ev_veh = rd::<GPtr<u8>>(e, 0xC);
    let p = ped(this);
    if !in_vehicle(p) {
        return;
    }
    let veh = rd::<GPtr<u8>>(p, 0x58C);
    if veh.is_null() || veh != ev_veh {
        return;
    }
    set(this, new_task(0x34, |t| {
        let delay = general_random_int_in_range(100, 400);
        leave_car_ctor(t, rd::<GPtr<u8>>(ped(this), 0x58C), 0, delay, 1, 0)
    }));
}

/// `CEventHandler::ComputePotentialWalkIntoFireResponse` (0x4BBCD0): with a go-to `simplest` (target +0xC); event
/// +0x14 the fire position, +0x24 its radius, +0x28 the move state. Task 200 → NULL; 0x202 →
/// `CTaskComplexWalkRoundFire(ms, &pos, radius, &target)`; 0x38C → `CTaskComplexFleePoint(&pos, false, <defaults>)`;
/// 0x38E → `CTaskComplexSmartFleePoint(&pos, false, <defaults>)`; others: nothing.
#[hook(addr = 0x4bbcd0, conv = thiscall, name = "CEventHandler::ComputePotentialWalkIntoFireResponse", expect = "6A FF 68 21 AF 83 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_potential_walk_into_fire_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, simplest: GPtr<u8>) {
    let radius = rd::<u32>(e, 0x24);
    let pos = e.byte_add(0x14);
    if simplest.is_null() || task_is_go_to_task(simplest) == 0 {
        return;
    }
    let target = simplest.byte_add(0xC);
    let ms = rd::<i32>(e, 0x28);
    match task_type(e) {
        200 => set(this, GPtr::NULL),
        0x202 => set(this, new_task(0x38, |t| walk_round_fire_ctor(t, ms, pos, radius, target))),
        0x38C => set(this, new_task(0x3C, |t| flee_point_ctor(t, pos, 0, gu(FLEE_POINT_A), gu(FLEE_POINT_B)))),
        0x38E => set(this, new_task(0x44, |t| smart_flee_point_ctor(t, pos, 0, gu(SMART_FLEE_POINT_A), gu(SMART_FLEE_POINT_B)))),
        _ => {}
    }
}

/// `CEventHandler::ComputeShotFiredWhizzedByResponse` (0x4BBE30): with a source entity (vtable +0x28): task 200 →
/// NULL; task 0x137: an active duck task (`active`, type 0x19F, +0xE above -1) or else the intelligence's duck task
/// (`GetTaskDuck(true)`, read up to three times, +0xE above -1) is restarted (`RestartTask(ped)`) and +0x24 = NULL;
/// otherwise +0x24 = `CTaskSimpleDuck(1, GetRandomNumberInRange(3000, 5000), 1000)`. Others: nothing.
#[hook(addr = 0x4bbe30, conv = thiscall, name = "CEventHandler::ComputeShotFiredWhizzedByResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_shot_fired_whizzed_by_response(this: GPtr<u8>, e: GPtr<u8>, active: GPtr<u8>, _simplest: GPtr<u8>) {
    let ty = task_type(e);
    if v_event_source(e).is_null() {
        return;
    }
    match ty {
        200 => set(this, GPtr::NULL),
        0x137 => {
            if !active.is_null() && v_task_type(active) == 0x19F && rd::<i16>(active, 0xE) > -1 {
                duck_restart_task(active, ped(this));
                set(this, GPtr::NULL);
                return;
            }
            if !intel_get_task_duck(intel(ped(this)), 1).is_null()
                && rd::<i16>(intel_get_task_duck(intel(ped(this)), 1), 0xE) > -1
            {
                let p = ped(this);
                let duck = intel_get_task_duck(intel(p), 1);
                duck_restart_task(duck, p);
                set(this, GPtr::NULL);
                return;
            }
            set(this, new_task(0x28, |t| {
                let time = general_random_int_in_range(3000, 5000);
                duck_ctor(t, 1, time as u32, 1000)
            }));
        }
        _ => {}
    }
}

/// `CEventHandler::ComputeAreaCodesResponse` (0x4BBF50): with the ped (+0xC): +0x24 =
/// `CTaskSimpleWaitUntilAreaCodesMatch(ped)`.
#[hook(addr = 0x4bbf50, conv = thiscall, name = "CEventHandler::ComputeAreaCodesResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_area_codes_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let other = rd::<GPtr<u8>>(e, 0xC);
    if other.is_null() {
        return;
    }
    set(this, new_task(0x24, |t| wait_until_area_codes_match_ctor(t, other)));
}

/// `CEventHandler::ComputeFireNearbyResponse` (0x4BBFB0): task 200 → NULL; 0x25C → `CTaskComplexExtinguishFires`;
/// others: nothing.
#[hook(addr = 0x4bbfb0, conv = thiscall, name = "CEventHandler::ComputeFireNearbyResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_fire_nearby_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    match task_type(e) {
        200 => set(this, GPtr::NULL),
        0x25C => set(this, new_task(0x10, extinguish_fires_ctor)),
        _ => {}
    }
}

/// `CEventHandler::ComputeSeenCopResponse` (0x4BC050): with the cop (+0x14): task 200 → NULL; 0x38F → smart flee
/// (defaults); 0x3E8 → `CTaskComplexKillPedOnFoot(cop, -1, 0, 0, 0, true)`; others: nothing.
#[hook(addr = 0x4bc050, conv = thiscall, name = "CEventHandler::ComputeSeenCopResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_seen_cop_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let cop = rd::<GPtr<u8>>(e, 0x14);
    if cop.is_null() {
        return;
    }
    match task_type(e) {
        200 => set(this, GPtr::NULL),
        0x38F => set(this, smart_flee_default(cop)),
        0x3E8 => set(this, kill_on_foot(cop)),
        _ => {}
    }
}

/// `CEventHandler::ComputeOnEscalatorResponse` (0x4BC150): +0x24 = an inlined stand-still task
/// (`CTaskSimpleStandStill(0, true, false, 8.0)`, then vtable 0x85B8DC).
#[hook(addr = 0x4bc150, conv = thiscall, name = "CEventHandler::ComputeOnEscalatorResponse", expect = "6A FF 68 BB AF 83 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_on_escalator_response(this: GPtr<u8>, _e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let t = task_new(0x20);
    if !t.is_null() {
        stand_still_ctor(t, 0, 1, 0, 0x4100_0000);
        wr::<u32>(t, 0, VT_STAND_STILL_ESCALATOR);
    }
    set(this, t);
}

/// `CEventHandler::ComputeDontJoinGroupResponse` (0x4BC1D0): +0x24 = `CTaskComplexGangJoinRespond(false)`.
#[hook(addr = 0x4bc1d0, conv = thiscall, name = "CEventHandler::ComputeDontJoinGroupResponse", expect = "6A FF 68 DB AF 83 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_dont_join_group_response(this: GPtr<u8>, _e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    set(this, new_task(0x14, |t| gang_join_respond_ctor(t, 0)));
}

/// `CEventHandler::ComputeDangerResponse` (0x4BC230): with the source (+0x14): task 0x2D6 in a vehicle → as its
/// driver `CTaskComplexCarDriveMissionFleeScene(vehicle)`, else NULL; on foot (and task 0x38F) → smart flee `(source,
/// true, 100000.0, <time>, <period>, <shift>)`; others: nothing.
#[hook(addr = 0x4bc230, conv = thiscall, name = "CEventHandler::ComputeDangerResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_danger_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let src = rd::<GPtr<u8>>(e, 0x14);
    if src.is_null() {
        return;
    }
    let flee = || new_task(0x40, |t| smart_flee_entity_ctor(t, src, 1, 0x47C3_5000, gu(FLEE_TIME), gu(FLEE_PERIOD), gu(FLEE_SHIFT)));
    match task_type(e) {
        0x2D6 => {
            let p = ped(this);
            if !in_vehicle(p) {
                set(this, flee());
                return;
            }
            if vehicle_is_driver(rd::<GPtr<u8>>(p, 0x58C), p) == 0 {
                set(this, GPtr::NULL);
                return;
            }
            set(this, new_task(0x2C, |t| car_drive_mission_flee_scene_ctor(t, rd::<GPtr<u8>>(ped(this), 0x58C))));
        }
        0x38F => set(this, flee()),
        _ => {}
    }
}

/// `CEventHandler::IsKillTaskAppropriate(CPed* ped, CPed* other, const CEvent&)` (0x4BC3E0, static cdecl): 1 for a
/// mission ped (+0x484 == 2); else the current weapons (`+0x5A0 + 0x1C * slot(+0x718)`) decide: 0 when the ped's is
/// melee and the other's is not, else 1 (the NULL checks on the two weapon addresses are kept).
#[hook(addr = 0x4bc3e0, conv = cdecl, name = "CEventHandler::IsKillTaskAppropriate", expect = "8B 44 24 04 90 90",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_is_kill_task_appropriate(p: GPtr<u8>, other: GPtr<u8>, _e: GPtr<u8>) -> u8 {
    if rd::<u8>(p, 0x484) == 2 {
        return 1;
    }
    let weapon = |q: GPtr<u8>| q.byte_add(0x5A0 + 0x1C * isize::from(rd::<i8>(q, 0x718)));
    let mine = weapon(p);
    let theirs = weapon(other);
    if theirs.is_null() {
        return 1;
    }
    if mine.is_null() {
        return 0;
    }
    if weapon_is_type_melee(mine) != 0 && weapon_is_type_melee(theirs) == 0 {
        return 0;
    }
    1
}

/// `CEventHandler::ComputePedToChaseResponse` (0x4C1910): with the ped (+0xC): +0x24 =
/// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorStandard>(ped, 30000, 1000, <defaults>, true, true)`.
#[hook(addr = 0x4c1910, conv = thiscall, name = "CEventHandler::ComputePedToChaseResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_ped_to_chase_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let other = rd::<GPtr<u8>>(e, 0xC);
    if other.is_null() {
        return;
    }
    set(this, new_task(0x4C, |t| seek_entity_standard_ctor(t, other, 30000, 1000, gu(SEEK_A), gu(SEEK_B), gu(SEEK_C), 1, 1)));
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_HANDLER_COMPUTE_OBJECT_COLLISION_RESPONSE,
    HOOK_HANDLER_REGISTER_KILL,
    HOOK_HANDLER_COMPUTE_DEATH_RESPONSE,
    HOOK_HANDLER_COMPUTE_REVIVE_RESPONSE,
    HOOK_HANDLER_COMPUTE_SEXY_VEHICLE_RESPONSE,
    HOOK_HANDLER_COMPUTE_PED_TO_FLEE_RESPONSE,
    HOOK_HANDLER_COMPUTE_ATTRACTOR_RESPONSE,
    HOOK_HANDLER_COMPUTE_PED_THREAT_BADLY_LIT_RESPONSE,
    HOOK_HANDLER_COMPUTE_PED_SOUND_QUIET_RESPONSE,
    HOOK_HANDLER_COMPUTE_PED_FRIEND_RESPONSE,
    HOOK_HANDLER_COMPUTE_VEHICLE_TO_STEAL_RESPONSE,
    HOOK_HANDLER_COMPUTE_VEHICLE_DIED_RESPONSE,
    HOOK_HANDLER_COMPUTE_LOW_HEALTH_RESPONSE,
    HOOK_HANDLER_COMPUTE_REALLY_LOW_HEALTH_RESPONSE,
    HOOK_HANDLER_COMPUTE_LOW_ANGER_AT_PLAYER_RESPONSE,
    HOOK_HANDLER_COMPUTE_HIGH_ANGER_AT_PLAYER_RESPONSE,
    HOOK_HANDLER_COMPUTE_ON_FIRE_RESPONSE,
    HOOK_HANDLER_COMPUTE_WATER_CANNON_RESPONSE,
    HOOK_HANDLER_COMPUTE_IN_WATER_RESPONSE,
    HOOK_HANDLER_COMPUTE_INTERIOR_USE_INFO_RESPONSE,
    HOOK_HANDLER_COMPUTE_PASS_OBJECT_RESPONSE,
    HOOK_HANDLER_COMPUTE_PLAYER_WANTED_LEVEL_RESPONSE,
    HOOK_HANDLER_COMPUTE_COP_CAR_BEING_STOLEN_RESPONSE,
    HOOK_HANDLER_COMPUTE_OBJECT_COLLISION_PASSIVE_RESPONSE,
    HOOK_HANDLER_COMPUTE_CAR_UPSIDE_DOWN_RESPONSE,
    HOOK_HANDLER_COMPUTE_POTENTIAL_WALK_INTO_FIRE_RESPONSE,
    HOOK_HANDLER_COMPUTE_SHOT_FIRED_WHIZZED_BY_RESPONSE,
    HOOK_HANDLER_COMPUTE_AREA_CODES_RESPONSE,
    HOOK_HANDLER_COMPUTE_FIRE_NEARBY_RESPONSE,
    HOOK_HANDLER_COMPUTE_SEEN_COP_RESPONSE,
    HOOK_HANDLER_COMPUTE_ON_ESCALATOR_RESPONSE,
    HOOK_HANDLER_COMPUTE_DONT_JOIN_GROUP_RESPONSE,
    HOOK_HANDLER_COMPUTE_DANGER_RESPONSE,
    HOOK_HANDLER_IS_KILL_TASK_APPROPRIATE,
    HOOK_HANDLER_COMPUTE_PED_TO_CHASE_RESPONSE,
];
