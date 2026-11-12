//! `CEventHandler::ComputeShotFiredResponse`, `ComputeGunAimedAtResponse`, `ComputePersonalityResponseToDamage`,
//! `ComputePedEnteredVehicleResponse`, `ComputeVehicleCollisionResponse` and `ComputeVehicleDamageResponse` (batch
//! `intro-ai-4`). Ported
//! from the 1.0 US bytes (gta-reversed `EventHandler.cpp` as a reading aid; the bytes win). Unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CVehicle::IsDriver(CPed*)` (0x6D1C40, `ret 4`)
    fn vehicle_is_driver(v: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x6D1C40 => unported;
    /// `CPedIntelligence::GetTaskDuck(bool)` (0x6010A0, `ret 4`)
    fn intel_get_task_duck(i: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x6010A0 => unported;
    /// `CEventHandler::IsKillTaskAppropriate(CPed*, CPed*, const CEvent&)` (0x4BC3E0, static)
    fn handler_is_kill_task_appropriate(ped: GPtr<u8>, other: GPtr<u8>, e: GPtr<u8>) -> u8 = cdecl 0x4BC3E0 => crate::event_responses_simple::handler_is_kill_task_appropriate;
    /// `CInterestingEvents::Add(int32 type, CEntity*)` (0x602590, `ret 8`)
    fn interesting_events_add(ie: GPtr<u8>, ty: u32, e: GPtr<u8>) = thiscall 0x602590 => unported;
    /// `CGeneral::GetRandomNumberInRange<int32>(min, max)` (0x407180; draws its own rand)
    fn general_random_int_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CTheScripts::IsPlayerOnAMission()` (0x464D50)
    fn scripts_is_player_on_a_mission() -> u8 = cdecl 0x464D50 => unported;
    fn smart_flee_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, b: u32, d: u32, time: u32, period: u32, f: u32) -> GPtr<u8> = thiscall 0x65C430 => unported;
    fn duck_ctor(t: GPtr<u8>, ty: u32, time: u32, counter: i32) -> GPtr<u8> = thiscall 0x691FC0 => unported;
    fn car_drive_mission_flee_scene_ctor(t: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B8920 => unported;
    fn kill_ped_on_foot_ctor(t: GPtr<u8>, ped: GPtr<u8>, time: i32, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x620E30 => unported;
    fn kill_ped_on_foot_stealth_ctor(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x620F70 => unported;
    fn kill_criminal_ctor(t: GPtr<u8>, ped: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x68BE70 => unported;
    fn turn_to_face_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x66B890 => unported;
    fn leave_car_ctor(t: GPtr<u8>, veh: GPtr<u8>, a: i32, b: i32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    fn leave_car_and_flee_ctor(t: GPtr<u8>, veh: GPtr<u8>, pos: GPtr<u8>, a: i32, b: i32, c: u32) -> GPtr<u8> = thiscall 0x63BF90 => unported;
    fn leave_car_and_wander_ctor(t: GPtr<u8>, veh: GPtr<u8>, a: i32, b: i32, c: u32) -> GPtr<u8> = thiscall 0x63BDD0 => unported;
    fn scream_in_car_then_leave_ctor(t: GPtr<u8>, veh: GPtr<u8>, a: i32) -> GPtr<u8> = thiscall 0x63C240 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::IsGoToTask(CTask*)` (0x61A360, static)
    fn task_is_go_to_task(t: GPtr<u8>) -> u8 = cdecl 0x61A360 => unported;
    /// `CMentalState::IncrementAnger(int32)` (0x421050, `ret 4`; called with the ped's intelligence)
    fn mental_state_increment_anger(m: GPtr<u8>, n: i32) = thiscall 0x421050 => unported;
    /// `CTaskComplexHitPedWithCar::ComputeEvasiveStepMoveDir(CVector* out, const CPed&, const CVehicle&)` (0x653B40)
    fn hit_ped_with_car_compute_evasive_step_move_dir(out: GPtr<u8>, ped: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> = cdecl 0x653B40 => unported;
    /// `operator*(CVector* out, float, const CVector&)` (0x40FE90, cdecl)
    fn vector_scale(out: GPtr<u8>, f: u32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `DotProduct(const CVector&, const CVector&)` (0x40FDB0, ST0)
    fn vector_dot(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `CPedGeometryAnalyser::CanPedJumpObstacle(const CPed&, const CEntity&)` (0x5F1B00)
    fn geometry_can_ped_jump_obstacle_los(ped: GPtr<u8>, e: GPtr<u8>) -> u8 = cdecl 0x5F1B00 => unported;
    /// `CPedGroups::GetPedsGroup(CPed*)` (0x5F7E80)
    fn ped_groups_get_peds_group(ped: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedIntelligence::IsPedGoingForCarDoor` (0x602350)
    fn intel_is_ped_going_for_car_door(i: GPtr<u8>) -> u8 = thiscall 0x602350 => unported;
    /// `CPlayerPed::AnnoyPlayerPed(bool)` (0x60A040, `ret 4`)
    fn player_ped_annoy_player_ped(p: GPtr<u8>, b: u32) = thiscall 0x60A040 => unported;
    /// `CPed::GetWeaponSlot(eWeaponType)` (0x5DF200, `ret 4`)
    fn ped_get_weapon_slot(p: GPtr<u8>, ty: u32) -> i32 = thiscall 0x5DF200 => unported;
    /// `IKChainManager_c::LookAt(...)` (0x618970, `ret 0x2C`)
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, pos: u32, torso: u32,
                  speed: u32, blend: i32, prio: i32, b: u32) = thiscall 0x618970 => unported;
    fn evasive_step_ctor(t: GPtr<u8>, e: GPtr<u8>, dir: GPtr<u8>) -> GPtr<u8> = thiscall 0x6532D0 => unported;
    fn hit_ped_with_car_ctor(t: GPtr<u8>, veh: GPtr<u8>, impulse: u32) -> GPtr<u8> = thiscall 0x6539A0 => unported;
    fn complex_jump_ctor(t: GPtr<u8>, a: i32) -> GPtr<u8> = thiscall 0x67A030 => unported;
    fn walk_round_car_ctor(t: GPtr<u8>, ms: i32, target: GPtr<u8>, veh: GPtr<u8>, door: u32, n: i32) -> GPtr<u8> = thiscall 0x6541B0 => unported;
    fn shake_fist_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x690B80 => unported;
    /// `CEvent::GetSourceEntity` (vtable +0x28)
    fn v_event_source(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x28 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    fn use_sequence_ctor(t: GPtr<u8>, idx: i32) -> GPtr<u8> = thiscall 0x635450 => unported;
    fn car_drive_mission_kill_ped_ctor(t: GPtr<u8>, veh: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B8A00 => unported;
    fn road_rage_ctor(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x6220A0 => unported;
    /// `CWeapon::IsTypeMelee` (0x73B1C0)
    fn weapon_is_type_melee(w: GPtr<u8>) -> u8 = thiscall 0x73B1C0 => unported;
    /// `CPedIntelligence::IsFriendlyWith(const CPed&)` (0x601BC0, `ret 4`)
    fn intel_is_friendly_with(i: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x601BC0 => unported;
    fn flee_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, b: u32, d: u32, time: u32, period: u32, f: u32) -> GPtr<u8> = thiscall 0x65B930 => unported;
    fn flee_any_means_ctor(t: GPtr<u8>, e: GPtr<u8>, b: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32) -> GPtr<u8> = thiscall 0x65CC60 => unported;
    fn cower_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x48DE70 => unported;
    fn hands_up_ctor(t: GPtr<u8>, time: i32) -> GPtr<u8> = thiscall 0x48E970 => unported;
    fn use_attractor_ctor(t: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x6346F0 => unported;
    fn use_attractor_run_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B8790 => unported;
    fn use_attractor_sprint_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B8850 => unported;
    fn react_to_gun_aimed_at_ctor(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x6971B0 => unported;
    /// `CPed::Say(context, offset, volume, b, b, b)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, ctx: u32, offset: u32, vol: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    fn dive_from_attached_ctor(t: GPtr<u8>, time: u32) -> GPtr<u8> = thiscall 0x492E20 => unported;
    fn destroy_car_ctor(t: GPtr<u8>, veh: GPtr<u8>, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x621C00 => unported;
}

/// `g_InterestingEvents`
const INTERESTING_EVENTS: usize = 0xC0B058;
const FLEE_SAFE_DIST: usize = 0xC18CEC;
const FLEE_TIME: usize = 0x86F674;
const FLEE_PERIOD: usize = 0x86F678;
const FLEE_SHIFT: usize = 0xC18CF0;
/// `CTaskSequences::ms_taskSequence` (0x40 bytes each; +0 the first task)
const TASK_SEQUENCES: usize = 0xC17900;
/// `g_ikChainMan`
const IK_CHAIN_MANAGER: usize = 0xC15448;
/// The look-at name of the vehicle collision response
const S_VEHICLE_COLLISION: u32 = 0x85BA3C;
/// 0.0
const K_ZERO: usize = 0x858B50;
/// The vehicle speed above which a collision is a hit (0x85AAD0) and the slower threshold (0x858CDC)
const K_HIT_SPEED: usize = 0x85AAD0;
const K_HIT_SPEED_SLOW: usize = 0x858CDC;
/// A player's impulse cap
const K_PLAYER_IMPULSE_CAP: usize = 0x85AAD4;
/// The height gap below which a ped jumps the vehicle it stands against
const K_JUMP_GAP: usize = 0x858C58;
const FLEE_ENT_DIST: usize = 0xC18CE4;
const FLEE_ENT_A: usize = 0x86F660;
const FLEE_ENT_B: usize = 0x86F664;
const FLEE_ENT_C: usize = 0x86F668;
const FLEE_ANY_A: usize = 0x86F684;
const FLEE_ANY_B: usize = 0x86F688;
const FLEE_ANY_C: usize = 0x86F68C;
/// `CTaskComplexTurnToFaceEntityOrCoord` defaults
const TURN_A: usize = 0x86FC7C;
const TURN_B: usize = 0x86FC80;

fn gu(a: usize) -> u32 {
    g::<u32>(a).read()
}

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn ped(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0)
}

fn new_task(size: u32, ctor: impl FnOnce(GPtr<u8>) -> GPtr<u8>) -> GPtr<u8> {
    let t = task_new(size);
    if t.is_null() { t } else { ctor(t) }
}

fn set(this: GPtr<u8>, t: GPtr<u8>) {
    wr::<GPtr<u8>>(this, 0x24, t);
}

fn smart_flee(other: GPtr<u8>, b: u32) -> GPtr<u8> {
    new_task(0x40, |t| smart_flee_entity_ctor(t, other, b, gu(FLEE_SAFE_DIST), gu(FLEE_TIME), gu(FLEE_PERIOD), gu(FLEE_SHIFT)))
}

fn in_vehicle(p: GPtr<u8>) -> bool {
    rd::<u32>(p, 0x46C) & 0x100 != 0
}

/// The "as the driver of its own vehicle, flee the scene; else smart flee from the own vehicle" answer.
fn flee_scene_or_own_vehicle(this: GPtr<u8>) -> GPtr<u8> {
    let p = ped(this);
    let own = rd::<GPtr<u8>>(p, 0x58C);
    if !own.is_null() && vehicle_is_driver(own, p) != 0 {
        return new_task(0x2C, |t| car_drive_mission_flee_scene_ctor(t, rd::<GPtr<u8>>(ped(this), 0x58C)));
    }
    new_task(0x40, |t| {
        let own = rd::<GPtr<u8>>(ped(this), 0x58C);
        smart_flee_entity_ctor(t, own, 0, gu(FLEE_SAFE_DIST), gu(FLEE_TIME), gu(FLEE_PERIOD), gu(FLEE_SHIFT))
    })
}

/// `CEventHandler::ComputeShotFiredResponse` (0x4BC710): with the shooter (+0x14). By task (others: nothing):
/// - 200 → NULL. 0x19F → unless the intelligence has a duck task, `CTaskSimpleDuck(0, 3000, -1)`. 0x1AB →
///   `CTaskSimpleDuck(0, 0x967F, -1)`. 0x2D6 → driver: flee the scene, else smart flee from the own vehicle.
///   0x38F → smart flee `(shooter, true, <defaults>)`. 0x398 → `CTaskComplexTurnToFaceEntityOrCoord(shooter,
///   <defaults>)`. 0x404 → `CTaskComplexKillPedOnFootStealth(shooter)`.
/// - 0x3E8 (a ped shooter of another ped type, +0x598): `IsKillTaskAppropriate` → `CTaskComplexKillPedOnFoot(shooter,
///   -1, 0, 0, 0, true)`; else as the driver of its vehicle → flee the scene; else a passenger of a driven vehicle →
///   NULL; else smart flee `(shooter, false)`.
/// - 0x451 (same shooter conditions): a player → NULL; appropriate → `CTaskComplexKillCriminal(shooter, false)`; else
///   smart flee `(shooter, false)` with its +0x30 = 4 (through NULL on a failed allocation, as the original).
#[hook(addr = 0x4bc710, conv = thiscall, name = "CEventHandler::ComputeShotFiredResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_shot_fired_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let shooter = rd::<GPtr<u8>>(e, 0x14);
    if shooter.is_null() {
        return;
    }
    let other_kind = |this: GPtr<u8>| {
        rd::<u8>(shooter, 0x36) & 7 == 3 && rd::<u32>(shooter, 0x598) != rd::<u32>(ped(this), 0x598)
    };
    match i32::from(rd::<i16>(e, 0xE)) {
        200 => set(this, GPtr::NULL),
        0x19F => {
            if !intel_get_task_duck(rd::<GPtr<u8>>(ped(this), 0x47C), 1).is_null() {
                return;
            }
            set(this, new_task(0x28, |t| duck_ctor(t, 0, 3000, -1)));
        }
        0x1AB => set(this, new_task(0x28, |t| duck_ctor(t, 0, 0x967F, -1))),
        0x2D6 => set(this, flee_scene_or_own_vehicle(this)),
        0x38F => set(this, smart_flee(shooter, 1)),
        0x398 => set(this, new_task(0x28, |t| turn_to_face_entity_ctor(t, shooter, gu(TURN_A), gu(TURN_B)))),
        0x404 => set(this, new_task(0x48, |t| kill_ped_on_foot_stealth_ctor(t, shooter))),
        0x3E8 => {
            if !other_kind(this) {
                return;
            }
            if handler_is_kill_task_appropriate(ped(this), shooter, e) != 0 {
                set(this, new_task(0x38, |t| kill_ped_on_foot_ctor(t, shooter, -1, 0, 0, 0, 1)));
                return;
            }
            let p = ped(this);
            if in_vehicle(p) {
                let own = rd::<GPtr<u8>>(p, 0x58C);
                if !own.is_null() && vehicle_is_driver(own, p) != 0 {
                    set(this, new_task(0x2C, |t| car_drive_mission_flee_scene_ctor(t, rd::<GPtr<u8>>(ped(this), 0x58C))));
                    return;
                }
            }
            let p = ped(this);
            if in_vehicle(p) {
                let own = rd::<GPtr<u8>>(p, 0x58C);
                if !own.is_null() && !rd::<GPtr<u8>>(own, 0x460).is_null() {
                    set(this, GPtr::NULL);
                    return;
                }
            }
            set(this, smart_flee(shooter, 0));
        }
        0x451 => {
            if !other_kind(this) {
                return;
            }
            if ped_is_player(shooter) != 0 {
                set(this, GPtr::NULL);
                return;
            }
            if handler_is_kill_task_appropriate(ped(this), shooter, e) != 0 {
                set(this, new_task(0x20, |t| kill_criminal_ctor(t, shooter, 0)));
                return;
            }
            let t = smart_flee(shooter, 0);
            set(this, t);
            wr::<u32>(t, 0x30, 4);
        }
        _ => {}
    }
}

/// `CTaskComplexFleeEntity(other, false, <defaults>)` (0x3C bytes) with the original's default-argument globals.
fn flee_entity_default(other: GPtr<u8>) -> GPtr<u8> {
    new_task(0x3C, |t| flee_entity_ctor(t, other, 0, gu(FLEE_ENT_DIST), gu(FLEE_ENT_A), gu(FLEE_ENT_B), gu(FLEE_ENT_C)))
}

/// `CTaskComplexFleeAnyMeans(other, true, <defaults>)` (0x54 bytes).
fn flee_any_means(other: GPtr<u8>) -> GPtr<u8> {
    new_task(0x54, |t| {
        flee_any_means_ctor(t, other, 1, gu(FLEE_SAFE_DIST), gu(FLEE_TIME), gu(FLEE_ANY_A), gu(FLEE_ANY_B), gu(FLEE_ANY_C),
                            gu(FLEE_PERIOD), gu(FLEE_SHIFT))
    })
}

/// Wanted by the police (+0x478 bit 0x800) and the aimer is a cop (+0x598 == 6): flee any means, else smart flee.
fn flee_aimer(this: GPtr<u8>, aimer: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(ped(this), 0x478) & 0x800 != 0 && rd::<i32>(aimer, 0x598) == 6 {
        flee_any_means(aimer)
    } else {
        smart_flee(aimer, 0)
    }
}

/// `CEventHandler::ComputeGunAimedAtResponse` (0x4C2840): the aimer is event +0x14 (not NULL-checked where the
/// original does not). Nothing for a player. An active flee task (type 0x38D / 0x38C → `CTaskComplexFleeEntity(aimer,
/// false, <defaults>)`; 0x38F / 0x38E → smart flee `(aimer, false)`) is renewed. A mission ped (+0x484 == 1) on foot
/// without a melee weapon, aimed at by a player it is not friendly with (`IsFriendlyWith`), gets the event's task
/// type (+0xE) set to 0x3E8. Then by task (others: nothing):
/// - 200 → NULL; 0x19C cower; 0x19D `CTaskSimpleHandsUp(5000)`; 0x19F → unless ducking, `CTaskSimpleDuck(0, 5000,
///   -1)`; 0x1AB → `CTaskSimpleDuck(0, 0x967F, -1)`; 0x102 / 0x105 / 0x106 the closest-free-scripted-attractor tasks;
///   0x259 → `CTaskComplexReactToGunAimedAt(aimer)`.
/// - 0x2D6 → as a driver flee the scene; else wanted and a cop aimer → flee any means, else smart flee. 0x38D → flee
///   entity. 0x38F → wanted and a cop → flee any means, else smart flee. 0x39F → flee any means.
/// - 0x3E8 → `IsKillTaskAppropriate` → `CTaskComplexKillPedOnFoot(aimer, -1, 0, 0, 0, true)`, else flee entity. 0x404
///   → `CTaskComplexKillPedOnFootStealth(aimer)`. 0x451 → a player aimer → NULL; appropriate →
///   `CTaskComplexKillCriminal(aimer, false)`, else flee entity.
#[hook(addr = 0x4c2840, conv = thiscall, name = "CEventHandler::ComputeGunAimedAtResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-4", route = "L2", hazards = "none", recs = "grove-walk")]
#[allow(clippy::too_many_lines)]
pub fn handler_compute_gun_aimed_at_response(this: GPtr<u8>, e: GPtr<u8>, active: GPtr<u8>, _simplest: GPtr<u8>) {
    let aimer = rd::<GPtr<u8>>(e, 0x14);
    if ped_is_player(ped(this)) != 0 {
        return;
    }
    if !active.is_null() {
        if v_task_type(active) == 0x38D || v_task_type(active) == 0x38C {
            set(this, flee_entity_default(aimer));
            return;
        }
        if v_task_type(active) == 0x38F || v_task_type(active) == 0x38E {
            set(this, smart_flee(aimer, 0));
            return;
        }
    }
    let p = ped(this);
    if rd::<u8>(p, 0x484) == 1
        && !in_vehicle(p)
        && weapon_is_type_melee(p.byte_add(0x5A0 + 0x1C * isize::from(rd::<i8>(p, 0x718)))) == 0
        && !aimer.is_null()
        && ped_is_player(aimer) != 0
        && intel_is_friendly_with(rd::<GPtr<u8>>(ped(this), 0x47C), aimer) == 0
    {
        wr::<i16>(e, 0xE, 0x3E8);
    }
    match i32::from(rd::<i16>(e, 0xE)) {
        200 => set(this, GPtr::NULL),
        0x19C => set(this, new_task(0x20, cower_ctor)),
        0x19D => set(this, new_task(0x34, |t| hands_up_ctor(t, 5000))),
        0x19F => {
            if !intel_get_task_duck(rd::<GPtr<u8>>(ped(this), 0x47C), 1).is_null() {
                return;
            }
            set(this, new_task(0x28, |t| duck_ctor(t, 0, 5000, -1)));
        }
        0x1AB => set(this, new_task(0x28, |t| duck_ctor(t, 0, 0x967F, -1))),
        0x102 => set(this, new_task(0x10, |t| use_attractor_ctor(t, 0))),
        0x105 => set(this, new_task(0x10, use_attractor_run_ctor)),
        0x106 => set(this, new_task(0x10, use_attractor_sprint_ctor)),
        0x259 => set(this, new_task(0x1C, |t| react_to_gun_aimed_at_ctor(t, aimer))),
        0x2D6 => {
            let p = ped(this);
            let own = rd::<GPtr<u8>>(p, 0x58C);
            if !own.is_null() && vehicle_is_driver(own, p) != 0 {
                set(this, new_task(0x2C, |t| car_drive_mission_flee_scene_ctor(t, rd::<GPtr<u8>>(ped(this), 0x58C))));
            } else {
                set(this, flee_aimer(this, aimer));
            }
        }
        0x38D => set(this, flee_entity_default(aimer)),
        0x38F => set(this, flee_aimer(this, aimer)),
        0x39F => set(this, flee_any_means(aimer)),
        0x3E8 => {
            if handler_is_kill_task_appropriate(ped(this), aimer, e) != 0 {
                set(this, new_task(0x38, |t| kill_ped_on_foot_ctor(t, aimer, -1, 0, 0, 0, 1)));
            } else {
                set(this, flee_entity_default(aimer));
            }
        }
        0x404 => set(this, new_task(0x48, |t| kill_ped_on_foot_stealth_ctor(t, aimer))),
        0x451 => {
            if ped_is_player(aimer) != 0 {
                set(this, GPtr::NULL);
            } else if handler_is_kill_task_appropriate(ped(this), aimer, e) != 0 {
                set(this, new_task(0x20, |t| kill_criminal_ctor(t, aimer, 0)));
            } else {
                set(this, flee_entity_default(aimer));
            }
        }
        _ => {}
    }
}

/// `CEventHandler::ComputePersonalityResponseToDamage(CEventDamage*, CPed* source)` (0x4BF9B0, thiscall, `ret 8`):
/// by the event's task type (others: nothing). "+say" = after writing +0x24, `CPed::Say(0xB2, 0, 1.0, 0, 0, 0)`.
/// - 200 / 0xFB → NULL. 0x102 / 0x105 / 0x106 the closest-free-scripted-attractor tasks. 0x19F → unless ducking,
///   `CTaskSimpleDuck(0, GetRandomNumberInRange(2000, 5000), -1)` (drawn after the allocation). 0x1AB →
///   `CTaskSimpleDuck(0, 0xE0FF, -1)`. 0x201 → `CTaskComplexDiveFromAttachedEntityAndGetUp(0)`.
/// - 0x2D6 → as a driver `CTaskComplexCarDriveMissionFleeScene(own)`; else wanted and a cop source → flee any
///   means, else smart flee `(source, false)`.
/// - 0x2D9 → in a vehicle `CTaskComplexCarDriveMissionKillPed(own, source)`, else `CTaskComplexKillPedOnFoot(source,
///   -1, 0, 0, 0, true)`.
/// - 0x38D → wanted → flee any means, else `CTaskComplexFleeEntity(source, false, <defaults>)`; +say. 0x38F → wanted
///   and a cop → flee any means, else smart flee; +say. 0x39F → flee any means; +say.
/// - 0x3E8 → `IsKillTaskAppropriate(ped, source, event)` → kill on foot; else wanted and a cop → flee any means;
///   else as a driver flee the scene; else smart flee. 0x404 → `CTaskComplexKillPedOnFootStealth(source)`. 0x451 → a
///   player → NULL; appropriate → `CTaskComplexKillCriminal(source, false)`; else smart flee.
#[hook(addr = 0x4bf9b0, conv = thiscall, name = "CEventHandler::ComputePersonalityResponseToDamage", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", route = "L2", hazards = "none", recs = "grove-walk")]
#[allow(clippy::too_many_lines)]
pub fn handler_compute_personality_response_to_damage(this: GPtr<u8>, e: GPtr<u8>, src: GPtr<u8>) {
    let wanted = |this: GPtr<u8>| rd::<u32>(ped(this), 0x478) & 0x800 != 0;
    let cop = |p: GPtr<u8>| rd::<i32>(p, 0x598) == 6;
    let say_after = |this: GPtr<u8>, t: GPtr<u8>| {
        set(this, t);
        ped_say(ped(this), 0xB2, 0, 0x3F80_0000, 0, 0, 0);
    };
    let driver_of_own = |this: GPtr<u8>| {
        let p = ped(this);
        let own = rd::<GPtr<u8>>(p, 0x58C);
        !own.is_null() && vehicle_is_driver(own, p) != 0
    };
    let flee_scene = |this: GPtr<u8>| new_task(0x2C, |t| car_drive_mission_flee_scene_ctor(t, rd::<GPtr<u8>>(ped(this), 0x58C)));
    match i32::from(rd::<i16>(e, 0xE)) {
        200 | 0xFB => set(this, GPtr::NULL),
        0x102 => set(this, new_task(0x10, |t| use_attractor_ctor(t, 0))),
        0x105 => set(this, new_task(0x10, use_attractor_run_ctor)),
        0x106 => set(this, new_task(0x10, use_attractor_sprint_ctor)),
        0x19F => {
            if !intel_get_task_duck(rd::<GPtr<u8>>(ped(this), 0x47C), 1).is_null() {
                return;
            }
            set(this, new_task(0x28, |t| {
                let time = general_random_int_in_range(2000, 5000);
                duck_ctor(t, 0, time as u32, -1)
            }));
        }
        0x1AB => set(this, new_task(0x28, |t| duck_ctor(t, 0, 0xE0FF, -1))),
        0x201 => set(this, new_task(0x24, |t| dive_from_attached_ctor(t, 0))),
        0x2D6 => {
            if driver_of_own(this) {
                set(this, flee_scene(this));
            } else if wanted(this) && cop(src) {
                set(this, flee_any_means(src));
            } else {
                set(this, smart_flee(src, 0));
            }
        }
        0x2D9 => {
            let p = ped(this);
            if rd::<u32>(p, 0x46C) & 0x100 != 0 && !rd::<GPtr<u8>>(p, 0x58C).is_null() {
                set(this, new_task(0x2C, |t| car_drive_mission_kill_ped_ctor(t, rd::<GPtr<u8>>(ped(this), 0x58C), src)));
            } else {
                set(this, new_task(0x38, |t| kill_ped_on_foot_ctor(t, src, -1, 0, 0, 0, 1)));
            }
        }
        0x38D => {
            let t = if wanted(this) { flee_any_means(src) } else { flee_entity_default(src) };
            say_after(this, t);
        }
        0x38F => {
            let t = if wanted(this) && cop(src) { flee_any_means(src) } else { smart_flee(src, 0) };
            say_after(this, t);
        }
        0x39F => {
            let t = flee_any_means(src);
            say_after(this, t);
        }
        0x3E8 => {
            if handler_is_kill_task_appropriate(ped(this), src, e) != 0 {
                set(this, new_task(0x38, |t| kill_ped_on_foot_ctor(t, src, -1, 0, 0, 0, 1)));
            } else if wanted(this) && cop(src) {
                set(this, flee_any_means(src));
            } else if rd::<u32>(ped(this), 0x46C) & 0x100 != 0 && driver_of_own(this) {
                set(this, flee_scene(this));
            } else {
                set(this, smart_flee(src, 0));
            }
        }
        0x404 => set(this, new_task(0x48, |t| kill_ped_on_foot_stealth_ctor(t, src))),
        0x451 => {
            if ped_is_player(src) != 0 {
                set(this, GPtr::NULL);
            } else if handler_is_kill_task_appropriate(ped(this), src, e) != 0 {
                set(this, new_task(0x20, |t| kill_criminal_ctor(t, src, 0)));
            } else {
                set(this, smart_flee(src, 0));
            }
        }
        _ => {}
    }
}

/// `CEventHandler::ComputePedEnteredVehicleResponse` (0x4C1590): event +0x14 the intruder, +0x18 the vehicle, +0x1C
/// the leave delay. Needs both and the ped in a vehicle; `IsDriver(own vehicle, ped)` (result unused). A vehicle
/// whose handling (+0x384) byte +0xDE is 0xF → +0x24 = smart flee `(intruder, false, <defaults>)`. Else
/// `g_InterestingEvents.Add(0x14, vehicle)`, `r = GetRandomNumberInRange(300, 600)`; by task (others: nothing): 200
/// → NULL; 0x2C0 → `CTaskComplexLeaveCar(vehicle, delay, r, false, true)`; 0x2C2 →
/// `CTaskComplexLeaveCarAndFlee(vehicle, &vehiclePos, delay, r, false)`; 0x2C3 →
/// `CTaskComplexLeaveCarAndWander(vehicle, delay, r, false)`; 0x2C4 → a player intruder during a mission → leave and
/// flee, else `CTaskComplexScreamInCarThenLeave(vehicle, delay)`; 0x3E8 → `IsKillTaskAppropriate(ped, intruder,
/// event)` → `CTaskComplexKillPedOnFoot(intruder, -1, 0, 0, 0, true)`, else leave and flee; 0x3EB →
/// `CTaskComplexDestroyCar(vehicle, 0, 0, 0)`; 0x451 → a player → NULL, else `CTaskComplexKillCriminal(intruder,
/// false)`.
#[hook(addr = 0x4c1590, conv = thiscall, name = "CEventHandler::ComputePedEnteredVehicleResponse", expect = "6A FF 68 AE B7 83 00",
       this = "CEventHandler", unverified = "intro-ai-4", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_ped_entered_vehicle_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(e, 0x18);
    if veh.is_null() {
        return;
    }
    let intruder = rd::<GPtr<u8>>(e, 0x14);
    if intruder.is_null() {
        return;
    }
    let p = ped(this);
    if !in_vehicle(p) {
        return;
    }
    vehicle_is_driver(rd::<GPtr<u8>>(p, 0x58C), p);
    if rd::<u8>(rd::<GPtr<u8>>(veh, 0x384), 0xDE) == 0xF {
        set(this, new_task(0x40, |t| {
            smart_flee_entity_ctor(t, intruder, 0, gu(FLEE_SAFE_DIST), gu(FLEE_TIME), gu(FLEE_PERIOD), gu(FLEE_SHIFT))
        }));
        return;
    }
    interesting_events_add(GPtr::from_addr(INTERESTING_EVENTS), 0x14, veh);
    let delay = rd::<i32>(e, 0x1C);
    let r = general_random_int_in_range(300, 600);
    let veh_pos = || {
        let m = rd::<GPtr<u8>>(veh, 0x14);
        if m.is_null() { veh.byte_add(4) } else { m.byte_add(0x30) }
    };
    let leave_and_flee = || new_task(0x28, |t| leave_car_and_flee_ctor(t, veh, veh_pos(), delay, r, 0));
    match i32::from(rd::<i16>(e, 0xE)) {
        200 => set(this, GPtr::NULL),
        0x2C0 => set(this, new_task(0x34, |t| leave_car_ctor(t, veh, delay, r, 0, 1))),
        0x2C2 => set(this, leave_and_flee()),
        0x2C3 => set(this, new_task(0x1C, |t| leave_car_and_wander_ctor(t, veh, delay, r, 0))),
        0x2C4 => {
            if ped_is_player(intruder) != 0 && scripts_is_player_on_a_mission() != 0 {
                set(this, leave_and_flee());
            } else {
                set(this, new_task(0x20, |t| scream_in_car_then_leave_ctor(t, veh, delay)));
            }
        }
        0x3E8 => {
            if handler_is_kill_task_appropriate(ped(this), intruder, e) != 0 {
                set(this, new_task(0x38, |t| kill_ped_on_foot_ctor(t, intruder, -1, 0, 0, 0, 1)));
            } else {
                set(this, leave_and_flee());
            }
        }
        0x3EB => set(this, new_task(0x20, |t| destroy_car_ctor(t, veh, 0, 0, 0))),
        0x451 => {
            if ped_is_player(intruder) != 0 {
                set(this, GPtr::NULL);
            } else {
                set(this, new_task(0x20, |t| kill_criminal_ctor(t, intruder, 0)));
            }
        }
        _ => {}
    }
}

/// `CEventHandler::ComputeVehicleCollisionResponse(CEventVehicleCollision*, CTask* active, CTask* simplest)`
/// (0x4BD6A0). Event: +0xE (i16), +0x10 the impulse, +0x14 the vehicle, +0x18 the collision normal, +0x30 the move
/// state (i8), +0x31 (i8). Nothing without the vehicle. The ped's +0x470 &= ~0x08000000; `speed` = the vehicle's 3D
/// move speed (`fsqrt`, f32). The original keeps `impulse` in its event argument slot (written over the pointer).
/// - +0xE == 1: `CTaskComplexEvasiveStep(vehicle, ComputeEvasiveStepMoveDir(&tmp, ped, vehicle))`.
/// - `speed` above 0x85AAD0: a player on the ground (+0x46C bit 0): `d = (ped +0x4DC · forward) · normal`; below 0,
///   `impulse = max(0, d · mass(+0x8C) + impulse)` (x87; `fld 0; fcomp`: not above 0 → 0). A player's impulse is
///   capped at 0x85AAD4. `CTaskComplexHitPedWithCar(vehicle, impulse)` (a non-player: the event's impulse); the ped's
///   +0x470 |= 0x08000000 unless its +0x128 is the vehicle.
/// - Slower: a non-player, or a player with a primary task in slot 3 (intelligence +0x10), with a go-to `simplest`:
///   - the vehicle is the ped's +0x568: `|+0x558 - +0x55C|` (x87 `fabs`) below 0x858C58 and `CanPedJumpObstacle(ped,
///     vehicle)` → `CTaskComplexJump(0)`, else no write;
///   - else move state 1 → nothing (also skips the anger check); in a group → move state 6;
///     `CTaskComplexWalkRoundCar(ms, simplest +0xC, vehicle, IsPedGoingForCarDoor(), +0x31)` (the bool goes as the
///     argument-slot dword: its low byte over the impulse's upper bytes, as the original).
/// - Otherwise a player not flagged +0x46C bit 0x200: player data +0x14 == 0 and a driver → `AnnoyPlayerPed(false)`,
///   `LookAt(0x85BA3C, ped, vehicle, 1300, -1, NULL, true, 0.25, 500, 3, false)` and, with no weapon or a weapon in
///   slot 2 / 4 / 1, +0x30 = `CTaskSimpleShakeFist`; else `LookAt(..., 800, ...)`. A non-player (or flagged player)
///   faster than 0x858CDC → `CTaskComplexHitPedWithCar(vehicle, event impulse)`.
/// - Afterwards (except the move-state-1 case): a response of type 0x1F9 with a player driver → `IncrementAnger(2)`.
#[hook(addr = 0x4bd6a0, conv = thiscall, name = "CEventHandler::ComputeVehicleCollisionResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", route = "L2", hazards = "x87-transcendental", recs = "grove-walk",
       note = "the WalkRoundCar bool dword carries the impulse's upper bytes (see doc)")]
#[allow(clippy::too_many_lines)]
pub fn handler_compute_vehicle_collision_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, simplest: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(e, 0x14);
    if veh.is_null() {
        return;
    }
    let p = ped(this);
    wr::<u32>(p, 0x470, rd::<u32>(p, 0x470) & 0xF7FF_FFFF);
    let vx = x(rd::<f32>(veh, 0x44));
    let vy = x(rd::<f32>(veh, 0x48));
    let vz = x(rd::<f32>(veh, 0x4C));
    let speed = f32_of(vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt());
    let mut impulse = rd::<u32>(e, 0x10);
    if rd::<i16>(e, 0xE) == 1 {
        let t = task_new(0x1C);
        if !t.is_null() {
            let mut tmp = [0u32; 3];
            let dir = hit_ped_with_car_compute_evasive_step_move_dir(GPtr::from_raw(tmp.as_mut_ptr()).cast(), ped(this), veh);
            evasive_step_ctor(t, veh, dir);
        }
        set(this, t);
        anger_check(this, veh);
        return;
    }
    if !x(speed).fcom(x(gf(K_HIT_SPEED))).test_any(0x41) {
        let p = ped(this);
        if ped_is_player(p) != 0 && rd::<u8>(ped(this), 0x46C) & 1 != 0 {
            let p = ped(this);
            let mut tmp = [0u32; 3];
            let scaled = vector_scale(GPtr::from_raw(tmp.as_mut_ptr()).cast(), rd::<u32>(p, 0x4DC), rd::<GPtr<u8>>(p, 0x14).byte_add(0x10));
            let d = X87::from_f64_exact(vector_dot(scaled, e.byte_add(0x18)));
            if !d.fcom(x(gf(K_ZERO))).test_parity(0x05) {
                let v = d.fmul_m(rd::<f32>(p, 0x8C)).fadd_m(f32::from_bits(impulse));
                impulse = if x(gf(K_ZERO)).fcom(v).test_any(0x41) { f32_of(v).to_bits() } else { 0 };
            }
        }
        let t = if ped_is_player(ped(this)) != 0 {
            if !x(f32::from_bits(impulse)).fcom(x(gf(K_PLAYER_IMPULSE_CAP))).test_any(0x41) {
                impulse = gu(K_PLAYER_IMPULSE_CAP);
            }
            new_task(0x28, |t| hit_ped_with_car_ctor(t, veh, impulse))
        } else {
            new_task(0x28, |t| hit_ped_with_car_ctor(t, veh, rd::<u32>(e, 0x10)))
        };
        set(this, t);
        let p = ped(this);
        if rd::<GPtr<u8>>(p, 0x128) != veh {
            wr::<u32>(p, 0x470, rd::<u32>(p, 0x470) | 0x0800_0000);
        }
        anger_check(this, veh);
        return;
    }
    let use_route = if ped_is_player(ped(this)) != 0 && rd::<u32>(rd::<GPtr<u8>>(ped(this), 0x47C), 0x10) == 0 {
        ped_is_player(ped(this)) == 0
    } else {
        true
    };
    if use_route && !simplest.is_null() && task_is_go_to_task(simplest) != 0 {
        let p = ped(this);
        if veh == rd::<GPtr<u8>>(p, 0x568) {
            let gap = x(rd::<f32>(p, 0x558)).fsub_m(rd::<f32>(p, 0x55C)).fabs();
            if gap.fcom(x(gf(K_JUMP_GAP))).test_parity(0x05) || geometry_can_ped_jump_obstacle_los(p, veh) == 0 {
                anger_check(this, veh);
                return;
            }
            set(this, new_task(0x14, |t| complex_jump_ctor(t, 0)));
            anger_check(this, veh);
            return;
        }
        let target = simplest.byte_add(0xC);
        let mut ms = i32::from(rd::<i8>(e, 0x30));
        if ms == 1 {
            return;
        }
        if !ped_groups_get_peds_group(p).is_null() {
            ms = 6;
        }
        let door = intel_is_ped_going_for_car_door(rd::<GPtr<u8>>(ped(this), 0x47C));
        let flag = (impulse & 0xFFFF_FF00) | u32::from(door);
        set(this, new_task(0x58, |t| walk_round_car_ctor(t, ms, target, veh, flag, i32::from(rd::<i8>(e, 0x31)))));
        anger_check(this, veh);
        return;
    }
    let p = ped(this);
    if ped_is_player(p) != 0 && rd::<u32>(ped(this), 0x46C) & 0x200 == 0 {
        let p = ped(this);
        let pd = rd::<GPtr<u8>>(p, 0x480);
        let zero = !x(rd::<f32>(pd, 0x14)).fcom(x(gf(K_ZERO))).test_parity(0x44);
        if zero && !rd::<GPtr<u8>>(veh, 0x460).is_null() {
            player_ped_annoy_player_ped(p, 0);
            ik_look_at(GPtr::from_addr(IK_CHAIN_MANAGER), S_VEHICLE_COLLISION, ped(this), veh, 1300, -1, 0, 1, 0x3E80_0000, 500, 3, 0);
            let wtype = || rd::<u32>(p, 0x5A0 + 0x1C * isize::from(rd::<i8>(p, 0x718)));
            if wtype() == 0
                || ped_get_weapon_slot(p, wtype()) == 2
                || ped_get_weapon_slot(p, wtype()) == 4
                || ped_get_weapon_slot(p, wtype()) == 1
            {
                wr::<GPtr<u8>>(this, 0x30, new_task(0x10, shake_fist_ctor));
            }
        } else {
            ik_look_at(GPtr::from_addr(IK_CHAIN_MANAGER), S_VEHICLE_COLLISION, p, veh, 800, -1, 0, 1, 0x3E80_0000, 500, 3, 0);
        }
        anger_check(this, veh);
        return;
    }
    if !x(speed).fcom(x(gf(K_HIT_SPEED_SLOW))).test_any(0x41) {
        set(this, new_task(0x28, |t| hit_ped_with_car_ctor(t, veh, rd::<u32>(e, 0x10))));
    }
    anger_check(this, veh);
}

/// The common tail of `ComputeVehicleCollisionResponse` (0x4BDB25): a response of type 0x1F9 (hit ped with car)
/// whose vehicle has a player driver makes the ped angry (`IncrementAnger(2)` on its intelligence).
fn anger_check(this: GPtr<u8>, veh: GPtr<u8>) {
    let t = rd::<GPtr<u8>>(this, 0x24);
    if t.is_null() || v_task_type(t) != 0x1F9 {
        return;
    }
    let driver = rd::<GPtr<u8>>(veh, 0x460);
    if driver.is_null() || ped_is_player(driver) == 0 {
        return;
    }
    mental_state_increment_anger(rd::<GPtr<u8>>(ped(this), 0x47C), 2);
}

/// `CEventHandler::ComputeVehicleDamageResponse` (0x4C2FC0): event +0x14 the damaged vehicle, +0x18 the damager, +0x1C
/// the weapon. Only for the ped's own vehicle (+0x58C, set). By task (others and the damager cases without a
/// damager: nothing):
/// - 0xF4: the next response sequence (as `ComputePedThreatResponse`). 0x2C2: `CTaskComplexLeaveCarAndFlee(own,
///   &ownPos, 0, 0, 0)`. 0x2D6: driver → `CTaskComplexCarDriveMissionFleeScene(own)`, else smart flee from the own
///   vehicle.
/// - 0x2D9 (a ped or vehicle damager): the event's source entity is the player, a mission is running, the ped's +0x484
///   is 1 and the weapon is 0x31 → flee the scene; else `CTaskComplexCarDriveMissionKillPed(own, damager)`.
/// - 0x3E8: a ped damager for which `IsKillTaskAppropriate` fails → driver: flee the scene, else smart flee from the
///   own vehicle; otherwise `CTaskComplexKillPedOnFoot(damager, -1, 0, 0, 0, true)` for a ped, or the vehicle
///   damager's driver (+0x460, unchecked) for a vehicle.
/// - 0x407: `CTaskComplexRoadRage(damager)` (a vehicle: its driver).
/// - 0x451: a player ped (or a vehicle with a player driver) → NULL; else `CTaskComplexKillCriminal(ped or driver,
///   false)`.
#[hook(addr = 0x4c2fc0, conv = thiscall, name = "CEventHandler::ComputeVehicleDamageResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", route = "L2", hazards = "none", recs = "grove-walk")]
#[allow(clippy::too_many_lines)]
pub fn handler_compute_vehicle_damage_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let damager = rd::<GPtr<u8>>(e, 0x18);
    let own = rd::<GPtr<u8>>(ped(this), 0x58C);
    if own.is_null() || own != rd::<GPtr<u8>>(e, 0x14) {
        return;
    }
    let kind = || rd::<u8>(damager, 0x36) & 7;
    let own_veh = || rd::<GPtr<u8>>(ped(this), 0x58C);
    let flee_scene = || new_task(0x2C, |t| car_drive_mission_flee_scene_ctor(t, own_veh()));
    let flee_own = || new_task(0x40, |t| {
        smart_flee_entity_ctor(t, own_veh(), 0, gu(FLEE_SAFE_DIST), gu(FLEE_TIME), gu(FLEE_PERIOD), gu(FLEE_SHIFT))
    });
    match i32::from(rd::<i16>(e, 0xE)) {
        0xF4 => {
            let idx = i32::from(rd::<i8>(rd::<GPtr<u8>>(ped(this), 0x47C), 0xD0));
            if idx < 0 || g::<u32>(TASK_SEQUENCES + (idx as usize) * 0x40).read() == 0 {
                return;
            }
            set(this, new_task(0x1C, |t| use_sequence_ctor(t, idx)));
        }
        0x2C2 => set(this, new_task(0x28, |t| {
            let v = own_veh();
            let m = rd::<GPtr<u8>>(v, 0x14);
            let pos = if m.is_null() { v.byte_add(4) } else { m.byte_add(0x30) };
            leave_car_and_flee_ctor(t, v, pos, 0, 0, 0)
        })),
        0x2D6 => {
            if vehicle_is_driver(own, ped(this)) != 0 {
                set(this, flee_scene());
            } else {
                set(this, flee_own());
            }
        }
        0x2D9 => {
            if damager.is_null() || (kind() != 3 && kind() != 2) {
                return;
            }
            let src = v_event_source(e);
            if find_player_ped(-1) == src
                && scripts_is_player_on_a_mission() != 0
                && rd::<u8>(ped(this), 0x484) == 1
                && rd::<i32>(e, 0x1C) == 0x31
            {
                set(this, flee_scene());
                return;
            }
            set(this, new_task(0x2C, |t| car_drive_mission_kill_ped_ctor(t, own_veh(), damager)));
        }
        0x3E8 => {
            if damager.is_null() {
                return;
            }
            if kind() == 3 && handler_is_kill_task_appropriate(ped(this), damager, e) == 0 {
                let p = ped(this);
                let v = rd::<GPtr<u8>>(p, 0x58C);
                if !v.is_null() && vehicle_is_driver(v, p) != 0 {
                    set(this, flee_scene());
                } else {
                    set(this, flee_own());
                }
                return;
            }
            match kind() {
                3 => set(this, new_task(0x38, |t| kill_ped_on_foot_ctor(t, damager, -1, 0, 0, 0, 1))),
                2 => set(this, new_task(0x38, |t| kill_ped_on_foot_ctor(t, rd::<GPtr<u8>>(damager, 0x460), -1, 0, 0, 0, 1))),
                _ => {}
            }
        }
        0x407 => {
            if damager.is_null() {
                return;
            }
            match kind() {
                3 => set(this, new_task(0x10, |t| road_rage_ctor(t, damager))),
                2 => set(this, new_task(0x10, |t| road_rage_ctor(t, rd::<GPtr<u8>>(damager, 0x460)))),
                _ => {}
            }
        }
        0x451 => {
            if damager.is_null() {
                return;
            }
            match kind() {
                3 => {
                    if ped_is_player(damager) != 0 {
                        set(this, GPtr::NULL);
                    } else {
                        set(this, new_task(0x20, |t| kill_criminal_ctor(t, damager, 0)));
                    }
                }
                2 => {
                    let driver = rd::<GPtr<u8>>(damager, 0x460);
                    if !driver.is_null() && ped_is_player(driver) != 0 {
                        set(this, GPtr::NULL);
                    } else {
                        set(this, new_task(0x20, |t| kill_criminal_ctor(t, rd::<GPtr<u8>>(damager, 0x460), 0)));
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_HANDLER_COMPUTE_SHOT_FIRED_RESPONSE,
    HOOK_HANDLER_COMPUTE_GUN_AIMED_AT_RESPONSE,
    HOOK_HANDLER_COMPUTE_PERSONALITY_RESPONSE_TO_DAMAGE,
    HOOK_HANDLER_COMPUTE_PED_ENTERED_VEHICLE_RESPONSE,
    HOOK_HANDLER_COMPUTE_VEHICLE_COLLISION_RESPONSE,
    HOOK_HANDLER_COMPUTE_VEHICLE_DAMAGE_RESPONSE,
];
