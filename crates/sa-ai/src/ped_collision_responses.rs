//! `CEventHandler` responses to peds colliding with or threatening other peds (batch `intro-ai-1`): the
//! intro's airport peds reach them in the first in-world frames. Ported from the 1.0 US bytes (gta-reversed
//! `EventHandler.cpp` as a reading aid where it has the function; the bytes win). Unverified
//! (docs/verify/HOWTO.md).
//!
//! `CEventHandler` (see `handler.rs`): +0 the ped, +0x24 the event response task, +0x2C the "say" task the
//! collision responses create next to it. Every task comes from the task pool (`task_new`); an allocation
//! failure writes NULL where the original does (and dereferences NULL where the original does).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEvent::GetEventType` (vtable +4)
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CTask::IsGoToTask(CTask*)` (0x61A360, static)
    fn task_is_go_to_task(t: GPtr<u8>) -> u8 = cdecl 0x61A360 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740, `ret 4`)
    fn tm_find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CTaskManager::GetSimplestActiveTask` (0x6819D0)
    fn tm_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0, `ret 4`)
    fn intel_find_task_by_type(i: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `CPedIntelligence::GetMoveStateFromGoToTask` (0x601D70)
    fn intel_get_move_state_from_go_to_task(i: GPtr<u8>) -> i32 = thiscall 0x601D70 => unported;
    /// `CPedIntelligence::GetTaskDuck(bool)` (0x6010A0, `ret 4`)
    fn intel_get_task_duck(i: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x6010A0 => unported;
    /// `CPedIntelligence::GetPedFOVRange` (0x4ABEA0, ST0)
    fn intel_get_ped_fov_range(i: GPtr<u8>) -> f64 = thiscall 0x4ABEA0 => unported;
    /// `CMentalState::IncrementAnger(int32)` (0x421050, `ret 4`; called with the ped's intelligence)
    fn mental_state_increment_anger(m: GPtr<u8>, n: i32) = thiscall 0x421050 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CVehicle::IsDriver(CPed*)` (0x6D1C40, `ret 4`)
    fn vehicle_is_driver(v: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x6D1C40 => unported;
    /// `CPedGroups::GetPedsGroup(CPed*)` (0x5F7E80, cdecl)
    fn ped_groups_get_peds_group(ped: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupMembership::GetLeader` (0x5F69A0)
    fn membership_get_leader(m: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F69A0 => unported;
    /// `CGeneral::GetRandomNumberInRange<float>(min, max)` (0x41BD90; draws its own rand, ST0)
    fn general_random_float_in_range(min: f32, max: f32) -> f64 = cdecl 0x41BD90 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): out = a - b
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CVector::Magnitude` (0x4082C0, ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CVector::SquaredMagnitude` (0x406DA0, ST0)
    fn vector_magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CVector::Normalise` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `IKChainManager_c::LookAt(name, ped, target, time, bone, pos, useTorso, speed, blend, priority, b)`
    /// (0x618970, `ret 0x2C`)
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, pos: u32, torso: u32,
                  speed: u32, blend: i32, prio: i32, b: u32) = thiscall 0x618970 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityHitSide(const CEntity&, const CEntity&)` (0x5F3BC0, cdecl)
    fn geometry_compute_entity_hit_side(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x5F3BC0 => unported;
    /// `CPedGeometryAnalyser::ComputeRouteRoundEntityBoundingBox(const CPed&, CEntity&, const CVector&,
    /// CPointRoute&, int32)` (0x5F6110, cdecl)
    fn geometry_compute_route_round_entity_bbox(ped: GPtr<u8>, e: GPtr<u8>, target: GPtr<u8>, route: GPtr<u8>, n: i32) -> u8 = cdecl 0x5F6110 => unported;
    /// `CEventHandler::IsKillTaskAppropriate(CPed*, CPed*, const CEvent&)` (0x4BC3E0, static)
    fn handler_is_kill_task_appropriate(ped: GPtr<u8>, other: GPtr<u8>, e: GPtr<u8>) -> u8 = cdecl 0x4BC3E0 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60, cdecl)
    fn weapon_info_get(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CTaskSimpleSay::CTaskSimpleSay(int32 context, int32 time)` (0x48E360, `ret 8`)
    fn say_ctor(t: GPtr<u8>, ctx: i32, time: i32) -> GPtr<u8> = thiscall 0x48E360 => unported;
    /// `CTaskComplexEvasiveStep::CTaskComplexEvasiveStep(CEntity*, const CVector&)` (0x6532D0, `ret 8`)
    fn evasive_step_ctor(t: GPtr<u8>, e: GPtr<u8>, dir: GPtr<u8>) -> GPtr<u8> = thiscall 0x6532D0 => unported;
    /// `CTaskComplexFollowPointRoute::CTaskComplexFollowPointRoute(moveState, const CPointRoute&, mode, targetRadius,
    /// slowDownDist, overshootTarget, useBlending, standStillAfterMove)` (0x671510, `ret 0x20`)
    fn follow_point_route_ctor(t: GPtr<u8>, ms: i32, route: GPtr<u8>, mode: i32, radius: u32, slow: u32, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x671510 => unported;
    /// `CTaskComplexFallAndGetUp::CTaskComplexFallAndGetUp(int32 direction, int32 time)` (0x678700, `ret 8`)
    fn fall_and_get_up_ctor2(t: GPtr<u8>, dir: i32, time: i32) -> GPtr<u8> = thiscall 0x678700 => unported;
    /// `CTaskComplexHitResponse::CTaskComplexHitResponse(int32 side)` (0x631D70, `ret 4`)
    fn hit_response_ctor(t: GPtr<u8>, side: i32) -> GPtr<u8> = thiscall 0x631D70 => unported;
    /// `CTaskComplexPartnerShove::CTaskComplexPartnerShove(name, CPed*, bool, float, int32, CVector)` (0x6846F0, `ret 0x20`)
    fn partner_shove_ctor(t: GPtr<u8>, name: u32, ped: GPtr<u8>, b: u32, f: u32, n: i32, vx: u32, vy: u32, vz: u32) -> GPtr<u8> = thiscall 0x6846F0 => unported;
    /// `CTaskComplexAvoidOtherPedWhileWandering(CPed*, const CVector& target, int32 moveState)` (0x66A100, `ret 0xC`)
    fn avoid_other_ped_ctor(t: GPtr<u8>, ped: GPtr<u8>, target: GPtr<u8>, ms: i32) -> GPtr<u8> = thiscall 0x66A100 => unported;
    /// `CTaskSimpleHandsUp::CTaskSimpleHandsUp(int32 time)` (0x48E970, `ret 4`)
    fn hands_up_ctor(t: GPtr<u8>, time: i32) -> GPtr<u8> = thiscall 0x48E970 => unported;
    /// `CTaskSimpleDuck::CTaskSimpleDuck(eDuckControlType, uint16 time, int16 shotWhizzingCounter)` (0x691FC0, `ret 0xC`)
    fn duck_ctor(t: GPtr<u8>, ty: u32, time: u32, counter: i32) -> GPtr<u8> = thiscall 0x691FC0 => unported;
    /// `CTaskComplexLeaveCar::CTaskComplexLeaveCar(CVehicle*, int32, int32, bool, bool)` (0x63B8C0, `ret 0x14`)
    fn leave_car_ctor(t: GPtr<u8>, veh: GPtr<u8>, a: i32, b: i32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    /// `CTaskComplexSmartFleeEntity::CTaskComplexSmartFleeEntity(CEntity*, bool, float, int32, int32, float)` (0x65C430, `ret 0x18`)
    fn smart_flee_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, b: u32, d: u32, time: u32, period: u32, f: u32) -> GPtr<u8> = thiscall 0x65C430 => unported;
    /// `CTaskComplexFleeEntity::CTaskComplexFleeEntity(CEntity*, bool, float, int32, int32, float)` (0x65B930, `ret 0x18`)
    fn flee_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, b: u32, d: u32, time: u32, period: u32, f: u32) -> GPtr<u8> = thiscall 0x65B930 => unported;
    /// `CTaskComplexFleeAnyMeans::CTaskComplexFleeAnyMeans(CEntity*, bool, float, int32, int32, int32, float, float, float)`
    /// (0x65CC60, `ret 0x24`)
    fn flee_any_means_ctor(t: GPtr<u8>, e: GPtr<u8>, b: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32) -> GPtr<u8> = thiscall 0x65CC60 => unported;
    /// `CTaskComplexKillPedOnFoot::CTaskComplexKillPedOnFoot(CPed*, int32 time, uint32, int32, int32, bool)` (0x620E30, `ret 0x18`)
    fn kill_ped_on_foot_ctor(t: GPtr<u8>, ped: GPtr<u8>, time: i32, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x620E30 => unported;
    /// `CTaskComplexKillPedOnFootStealth::CTaskComplexKillPedOnFootStealth(CPed*)` (0x620F70, `ret 4`)
    fn kill_ped_on_foot_stealth_ctor(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x620F70 => unported;
    /// `CTaskComplexKillCriminal::CTaskComplexKillCriminal(CPed*, bool)` (0x68BE70, `ret 8`)
    fn kill_criminal_ctor(t: GPtr<u8>, ped: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x68BE70 => unported;
    /// `CTaskComplexCarDriveMissionFleeScene::CTaskComplexCarDriveMissionFleeScene(CVehicle*)` (0x4B8920, `ret 4`)
    fn car_drive_mission_flee_scene_ctor(t: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B8920 => unported;
    /// `CTaskComplexCarDriveWander::CTaskComplexCarDriveWander(CVehicle*, eCarDrivingStyle, float speed)` (0x63CB10, `ret 0xC`)
    fn car_drive_wander_ctor(t: GPtr<u8>, veh: GPtr<u8>, style: u32, speed: u32) -> GPtr<u8> = thiscall 0x63CB10 => unported;
    /// `CTaskSimpleGunControl::CTaskSimpleGunControl(CEntity*, const CVector*, const CVector*, eGunCommand, int16, int32)`
    /// (0x61F3F0, `ret 0x18`)
    fn gun_control_ctor(t: GPtr<u8>, e: GPtr<u8>, a: u32, b: u32, cmd: u32, n: u32, time: i32) -> GPtr<u8> = thiscall 0x61F3F0 => unported;
    /// `CTaskComplexSeekEntityAiming::CTaskComplexSeekEntityAiming(CEntity*, float, float)` (0x694B90, `ret 0xC`)
    fn seek_entity_aiming_ctor(t: GPtr<u8>, e: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x694B90 => unported;
    /// `CTaskSimpleGangDriveBy::CTaskSimpleGangDriveBy(CEntity*, const CVector*, float range, int8 freq, int8 style, bool)`
    /// (0x6217D0, `ret 0x18`)
    fn gang_drive_by_ctor(t: GPtr<u8>, e: GPtr<u8>, pos: u32, range: u32, freq: u32, style: u32, b: u32) -> GPtr<u8> = thiscall 0x6217D0 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence` (0x632BD0)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10, `ret 4`)
    fn sequence_add_task(t: GPtr<u8>, task: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CTaskComplexUseSequence::CTaskComplexUseSequence(int32)` (0x635450, `ret 4`)
    fn use_sequence_ctor(t: GPtr<u8>, idx: i32) -> GPtr<u8> = thiscall 0x635450 => unported;
    /// `CTaskSimpleCower::CTaskSimpleCower` (0x48DE70)
    fn cower_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x48DE70 => unported;
    /// `CTaskComplexUseClosestFreeScriptedAttractor::CTaskComplexUseClosestFreeScriptedAttractor(bool)` (0x6346F0, `ret 4`)
    fn use_attractor_ctor(t: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x6346F0 => unported;
    /// `CTaskComplexUseClosestFreeScriptedAttractorRun` constructor (0x4B8790)
    fn use_attractor_run_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B8790 => unported;
    /// `CTaskComplexUseClosestFreeScriptedAttractorSprint` constructor (0x4B8850)
    fn use_attractor_sprint_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B8850 => unported;
    /// `FUN_004bc470` (the seek task's move-state radius setter; `misc.rs`)
    fn seek_set_move_state_radius(t: GPtr<u8>, r: f32) = thiscall 0x4BC470 => crate::misc::fun_004bc470;
}

/// `g_ikChainMan` (`IKChainManager_c`)
const IK_CHAIN_MANAGER: usize = 0xC15448;
/// "CompPedCollPedResp"
const S_COMP_PED_COLL_PED_RESP: u32 = 0x85BA54;
/// "CompPotPedCollResp"
const S_COMP_POT_PED_COLL_RESP: u32 = 0x85BAD0;
/// `CTaskSequences::ms_taskSequence` (0x40 bytes each; +0 the first task)
const TASK_SEQUENCES: usize = 0xC17900;
/// 0.0 (`float`)
const K_ZERO: usize = 0x858B50;
/// 0.25
const K_LOOK_CHANCE: usize = 0x858C84;
/// 0.33
const K_LOOK_ONLY: usize = 0x85AD74;
/// 0.66
const K_SAY: usize = 0x85BA50;
/// 0.1, added to the follower distance for the seek radius
const K_RADIUS_PAD: usize = 0x858B1C;
/// 8.0, the follower's catch-up distance
const K_FOLLOW_DIST: usize = 0x86F828;
/// 1.5, the stat (+0x2C of `CPedStat`) above which a ped knocks another over
const K_SHOVE_STAT: usize = 0x85AAE0;
/// `CTaskComplexFollowPointRoute`'s default target radius (0.5) and slow-down distance (5.0)
const K_ROUTE_RADIUS: usize = 0x86FC98;
const K_ROUTE_SLOW: usize = 0x86FC9C;
/// 5.0, added to the FOV range for the drive-by range
const K_FOV_PAD: usize = 0x858C80;
/// The flee tasks' default arguments (static-initialised / data globals, passed as the original reads them).
const FLEE_SAFE_DIST: usize = 0xC18CEC;
const FLEE_TIME: usize = 0x86F674;
const FLEE_PERIOD: usize = 0x86F678;
const FLEE_SHIFT: usize = 0xC18CF0;
const FLEE_ANY_A: usize = 0x86F684;
const FLEE_ANY_B: usize = 0x86F688;
const FLEE_ANY_C: usize = 0x86F68C;
const FLEE_ENT_DIST: usize = 0xC18CE4;
const FLEE_ENT_A: usize = 0x86F660;
const FLEE_ENT_B: usize = 0x86F664;
const FLEE_ENT_C: usize = 0x86F668;

fn gu(a: usize) -> u32 {
    g::<u32>(a).read()
}

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// An entity's position (matrix +0x30, else the placement +4).
fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

fn ped(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0)
}

fn intel(p: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(p, 0x47C)
}

fn new_task(size: u32, ctor: impl FnOnce(GPtr<u8>) -> GPtr<u8>) -> GPtr<u8> {
    let t = task_new(size);
    if t.is_null() { t } else { ctor(t) }
}

fn look_at_ped(p: GPtr<u8>, other: GPtr<u8>) {
    ik_look_at(GPtr::from_addr(IK_CHAIN_MANAGER), S_COMP_PED_COLL_PED_RESP, p, other, 2000, 5, 0, 1, 0x3E80_0000, 500, 3, 0);
}

fn new_say() -> GPtr<u8> {
    new_task(0x1C, |t| say_ctor(t, 0x1C, -1))
}

/// The evasive step both group branches build: direction `(forward.x, -forward.y, 0)` of the ped's matrix (as the
/// bytes: not a perpendicular), normalised; `CTaskComplexEvasiveStep(other, &dir)`, NULL on a failed allocation.
fn evasive_step(p: GPtr<u8>, other: GPtr<u8>) -> GPtr<u8> {
    let f = rd::<GPtr<u8>>(p, 0x14).byte_add(0x10);
    let mut v = [rd::<u32>(f, 0), f32_of(x(rd::<f32>(f, 4)).fchs()).to_bits(), 0];
    let vp: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr()).cast();
    vector_normalise(vp);
    new_task(0x1C, |t| evasive_step_ctor(t, other, vp))
}

/// The "knock over or shout" choice of the no-group branch: with both stats (+0x59C, `CPedStat` +0x2C) above 1.5
/// the ped with the higher stat knocks the other over; else the ped knocks the other over when its own stat is above
/// 1.5 (x87 compares, `test ah, 0x41` / `jne`: not greater = no).
fn knocks_over(p: GPtr<u8>, other: GPtr<u8>) -> bool {
    let mine = rd::<f32>(rd::<GPtr<u8>>(p, 0x59C), 0x2C);
    let k = x(gf(K_SHOVE_STAT));
    let gt = |a: f32, b: X87| !x(a).fcom(b).test_any(0x41);
    if gt(mine, k) {
        let theirs = rd::<f32>(rd::<GPtr<u8>>(other, 0x59C), 0x2C);
        if gt(theirs, k) {
            return gt(mine, x(theirs));
        }
    }
    gt(mine, k)
}

/// `CEventHandler::ComputePedCollisionWithPedResponse(CEventPedCollisionWithPed*, CTask* active, CTask* simplest)`
/// (0x4BDB80, thiscall, `ret 0xC`; not reversed in gta-reversed). Event: +0x14 the other ped, +0x30 / +0x32 the
/// ped's and the other's move states (i16). Nothing without the other ped; else +0x24 = the response (NULL when no
/// branch makes one) and, when it is not NULL and the other ped is a player, `IncrementAnger(1)` on the ped's
/// intelligence. Read from the bytes:
///
/// - `GetSimplestActiveTask` of the other's task manager (unused). `d = otherPos - pos`; `inFront = d·myForward >=
///   0` (x87, unordered = no), `facing = d·otherForward <= 0` (unordered = no).
/// - **The ped has a group** (`GetPedsGroup`): `leader` = its leader; the other's group and leader. `same` = same
///   group; then the other (if it is the leader) and the ped (if it is the leader) get flag 0x80000000 at +0x474.
///   - The other is a player: +0x2C = `CTaskSimpleSay(0x1C, -1)`.
///   - The other has no group: either move state 1 → the evasive step (`evasive_step`), else nothing.
///   - Else neither move state 1 → nothing. `same`: unless the other is its group's leader, the ped's leader must
///     exist with `GetMoveStateFromGoToTask` 1, else nothing. Not `same`: `rand(0,1) < 0.25` → a second
///     `rand(0,1)`: below 0.33 `LookAt`, else below 0.66 +0x2C = a say task.
///   - Then the gang follower (`FindTaskByType` 0x4B7), the active avoid-ped task (0x395), task 0x384 and the seek
///     task (0x38B, `FindTaskByType`). `same` with a follower, its leader (+0x10), a seek task and one of the
///     other two: `dist` = 2D distance ped − leader (f32); avoid task +0x5C |= 2; task 0x384 +0x18 = dist + 0.1;
///     the seek task's radius = dist + 0.1 (`FUN_004bc470`); `dist < 8` → follower +0x20 = the 2D vector. A
///     follower in a `same` group without the rest → nothing. Otherwise the evasive step.
/// - **No group**: move states (mine, other's) as below; every `LookAt` is `("CompPedCollPedResp", ped, other, 2000,
///   5, NULL, true, 0.25, 500, 3, false)`, every say task `(0x1C, -1)` goes to +0x2C.
///   - (4, 0|1): `inFront` → with a go-to `simplest`, `ComputeRouteRoundEntityBoundingBox` into a stack route and
///     `CTaskComplexFollowPointRoute(4, route, 0, 0.5, 5, false, true, true)`; then a say task. Else `LookAt` and a
///     say task.
///   - (4, 4): `inFront` → the route task when `simplest` (not NULL-checked) is a go-to task; `LookAt`; say. Else
///     `facing` → `LookAt`, say.
///   - (6|7, 6|7): not `inFront`: `facing` → say, `LookAt`. `inFront` and not `facing` → say, `LookAt`. Both:
///     `knocks_over` → `CTaskComplexFallAndGetUp(ComputeEntityHitSide(other, ped), 0)`, else say, `LookAt`,
///     `CTaskComplexHitResponse(ComputeEntityHitSide(other, ped))`.
///   - (0|1, 4|6|7) with `facing`: other 6/7 → as the last case (fall / say + hit response); other 4 → say,
///     `LookAt`.
///   - (4|6|7, 0|1) with `facing`: say, `LookAt`, then (mine not 4) the hit response.
#[hook(addr = 0x4bdb80, conv = thiscall, name = "CEventHandler::ComputePedCollisionWithPedResponse", expect = "6A FF 68 88 B3 83 00",
       this = "CEventHandler", unverified = "intro-ai-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "the evasive direction (forward.x, -forward.y, 0) is the original's")]
pub fn event_handler_compute_ped_collision_with_ped_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, simplest: GPtr<u8>) {
    let other = rd::<GPtr<u8>>(e, 0x14);
    if other.is_null() {
        return;
    }
    tm_get_simplest_active_task(intel(other).byte_add(4));
    let p = ped(this);
    let mp = pos_ptr(p);
    let op = pos_ptr(other);
    let dx = x(rd::<f32>(op, 0)).fsub_m(rd::<f32>(mp, 0));
    let dy = x(rd::<f32>(op, 4)).fsub_m(rd::<f32>(mp, 4));
    let dz = x(rd::<f32>(op, 8)).fsub_m(rd::<f32>(mp, 8));
    let dot = |m: GPtr<u8>| {
        let s = dz.fmul_m(rd::<f32>(m, 0x18));
        let s = s.fadd(dy.fmul_m(rd::<f32>(m, 0x14)));
        s.fadd(dx.fmul_m(rd::<f32>(m, 0x10)))
    };
    let mut flag_a = !dot(rd::<GPtr<u8>>(p, 0x14)).fcom(x(gf(K_ZERO))).c0();
    let facing = !dot(rd::<GPtr<u8>>(other, 0x14)).fcom(x(gf(K_ZERO))).test_parity(0x41);
    let my_ms = i32::from(rd::<i16>(e, 0x30));
    let other_ms = i32::from(rd::<i16>(e, 0x32));
    let mut result = GPtr::<u8>::NULL;
    let set_say = |t: GPtr<u8>| wr::<GPtr<u8>>(this, 0x2C, t);

    let group = ped_groups_get_peds_group(p);
    if !group.is_null() {
        let leader = membership_get_leader(group.byte_add(8));
        let other_group = ped_groups_get_peds_group(other);
        let other_leader = if other_group.is_null() { GPtr::NULL } else { membership_get_leader(other_group.byte_add(8)) };
        flag_a = false;
        if group == other_group {
            flag_a = true;
            if other == leader {
                wr::<u32>(other, 0x474, rd::<u32>(other, 0x474) | 0x8000_0000);
            }
            let p = ped(this);
            if p == leader {
                wr::<u32>(p, 0x474, rd::<u32>(p, 0x474) | 0x8000_0000);
            }
        }
        'body: {
            if ped_is_player(other) != 0 {
                set_say(new_say());
                break 'body;
            }
            if other_group.is_null() {
                if my_ms == 1 || other_ms == 1 {
                    result = evasive_step(ped(this), other);
                }
                break 'body;
            }
            if my_ms != 1 && other_ms != 1 {
                break 'body;
            }
            if flag_a {
                if other != other_leader
                    && (leader.is_null() || intel_get_move_state_from_go_to_task(intel(leader)) != 1)
                {
                    break 'body;
                }
            } else if X87::from_f64_exact(general_random_float_in_range(0.0, 1.0)).fcom(x(gf(K_LOOK_CHANCE))).test_parity(0x05) {
                // not below 0.25: nothing more here
            } else {
                let r = X87::from_f64_exact(general_random_float_in_range(0.0, 1.0));
                if !r.fcom(x(gf(K_LOOK_ONLY))).test_parity(0x05) {
                    look_at_ped(ped(this), other);
                } else if !r.fcom(x(gf(K_SAY))).test_parity(0x05) {
                    set_say(new_say());
                }
            }
            let follower = intel_find_task_by_type(intel(ped(this)), 0x4B7);
            let avoid = tm_find_active_task_by_type(intel(ped(this)).byte_add(4), 0x395);
            let t384 = intel_find_task_by_type(intel(ped(this)), 0x384);
            let seek = intel_find_task_by_type(intel(ped(this)), 0x38B);
            if flag_a && !follower.is_null() {
                let lead = rd::<GPtr<u8>>(follower, 0x10);
                if !lead.is_null() && !seek.is_null() && (!avoid.is_null() || !t384.is_null()) {
                    let mut v = [0u32; 3];
                    let vp: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr()).cast();
                    vector_sub(vp, pos_ptr(ped(this)), pos_ptr(lead));
                    wr::<u32>(vp, 8, 0);
                    let dist = f32_of(X87::from_f64_exact(vector_magnitude(vp)));
                    if !avoid.is_null() {
                        wr::<u8>(avoid, 0x5C, rd::<u8>(avoid, 0x5C) | 2);
                    }
                    if !t384.is_null() {
                        wr::<f32>(t384, 0x18, f32_of(x(dist).fadd_m(gf(K_RADIUS_PAD))));
                    }
                    seek_set_move_state_radius(seek, f32_of(x(dist).fadd_m(gf(K_RADIUS_PAD))));
                    if !x(dist).fcom(x(gf(K_FOLLOW_DIST))).test_parity(0x05) {
                        for (i, c) in v.iter().enumerate() {
                            wr::<u32>(follower, 0x20 + 4 * i as isize, *c);
                        }
                    }
                }
                break 'body;
            }
            result = evasive_step(ped(this), other);
        }
    } else {
        no_group_response(this, other, simplest, my_ms, other_ms, flag_a, facing, &mut result);
    }
    wr::<GPtr<u8>>(this, 0x24, result);
    if !result.is_null() && ped_is_player(other) != 0 {
        mental_state_increment_anger(intel(ped(this)), 1);
    }
}

/// The route task of the no-group (4, x) cases: `ComputeRouteRoundEntityBoundingBox(ped, other, simplest + 0xC,
/// route, 0)` into a zeroed stack `CPointRoute` (0x64 bytes, count 0), then `CTaskComplexFollowPointRoute`.
fn route_round(this: GPtr<u8>, other: GPtr<u8>, simplest: GPtr<u8>) -> GPtr<u8> {
    let mut route = [0u32; 25];
    let rp: GPtr<u8> = GPtr::from_raw(route.as_mut_ptr()).cast();
    geometry_compute_route_round_entity_bbox(ped(this), other, simplest.byte_add(0xC), rp, 0);
    new_task(0x3C, |t| follow_point_route_ctor(t, 4, rp, 0, gu(K_ROUTE_RADIUS), gu(K_ROUTE_SLOW), 0, 1, 1))
}

/// The no-group half of `ComputePedCollisionWithPedResponse` (see the hook's doc comment); `result` is the
/// response written to +0x24 afterwards.
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
fn no_group_response(this: GPtr<u8>, other: GPtr<u8>, simplest: GPtr<u8>, my_ms: i32, other_ms: i32, in_front: bool,
                     facing: bool, result: &mut GPtr<u8>) {
    let set_say = |t: GPtr<u8>| wr::<GPtr<u8>>(this, 0x2C, t);
    let fall = |result: &mut GPtr<u8>| {
        let side = geometry_compute_entity_hit_side(other, ped(this));
        *result = new_task(0x18, |t| fall_and_get_up_ctor2(t, side, 0));
    };
    let hit = |result: &mut GPtr<u8>| {
        let side = geometry_compute_entity_hit_side(other, ped(this));
        *result = new_task(0x10, |t| hit_response_ctor(t, side));
    };
    let fast = |ms: i32| ms == 6 || ms == 7;
    let slow = |ms: i32| ms == 0 || ms == 1;
    if my_ms == 4 {
        if slow(other_ms) {
            if in_front {
                if !simplest.is_null() && task_is_go_to_task(simplest) != 0 {
                    *result = route_round(this, other, simplest);
                }
                set_say(new_say());
            } else {
                look_at_ped(ped(this), other);
                set_say(new_say());
            }
            return;
        }
        if other_ms == 4 {
            if in_front {
                if task_is_go_to_task(simplest) != 0 {
                    *result = route_round(this, other, simplest);
                }
                look_at_ped(ped(this), other);
                set_say(new_say());
            } else if facing {
                look_at_ped(ped(this), other);
                set_say(new_say());
            }
            return;
        }
    } else {
        if fast(my_ms) && fast(other_ms) {
            if !in_front {
                if facing {
                    set_say(new_say());
                    look_at_ped(ped(this), other);
                }
            } else if !facing {
                set_say(new_say());
                look_at_ped(ped(this), other);
            } else if knocks_over(ped(this), other) {
                fall(result);
            } else {
                set_say(new_say());
                look_at_ped(ped(this), other);
                hit(result);
            }
            return;
        }
        if slow(my_ms) && (other_ms == 4 || fast(other_ms)) && facing {
            if fast(other_ms) && knocks_over(ped(this), other) {
                fall(result);
                return;
            }
            set_say(new_say());
            look_at_ped(ped(this), other);
            if other_ms != 4 {
                hit(result);
            }
            return;
        }
    }
    if slow(other_ms) && (my_ms == 4 || fast(my_ms)) && facing {
        set_say(new_say());
        look_at_ped(ped(this), other);
        if my_ms != 4 {
            hit(result);
        }
    }
}

/// `CEventHandler::ComputePotentialPedCollideResponse(CEventPotentialWalkIntoPed*, CTask* active, CTask* simplest)`
/// (0x4C2610, thiscall, `ret 0xC`). Event: +0xE the response task type (i16), +0x14 the target point, +0x20 the
/// other ped, +0x24 the move state. Nothing without the other ped. Move state: the event's when
/// `IsGoToTask(simplest)` (not NULL-checked), else 4 (walk). By task type (others: no write):
///
/// - 200 (`TASK_NONE`): +0x24 = NULL.
/// - 0x4B9: `CTaskComplexPartnerShove("CompPotPedCollResp", other, true, 0.5, 1, (0, 0, 0))`.
/// - 0x395: `follower` = the active gang-follower task (0x4B7). Above walk with a follower whose leader (+0x10) is
///   set: the leader's move state (a player's +0x534, else `GetMoveStateFromGoToTask`); below 6 (run) with an active
///   seek task (0x38B) and `|leader - ped|²` (2D, `SquaredMagnitude`) minus the seek radius² (+0x1C, x87) below the
///   radius² → walk. Then `CTaskComplexAvoidOtherPedWhileWandering(other, &event+0x14, moveState)`; with a follower
///   the new task's +0x5C |= 4 (NULL task: written through NULL, as the original).
#[hook(addr = 0x4c2610, conv = thiscall, name = "CEventHandler::ComputePotentialPedCollideResponse", expect = "6A FF 68 A6 B9 83 00",
       this = "CEventHandler", unverified = "intro-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn event_handler_compute_potential_ped_collide_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, simplest: GPtr<u8>) {
    let other = rd::<GPtr<u8>>(e, 0x20);
    if other.is_null() {
        return;
    }
    let mut ms = 4;
    if task_is_go_to_task(simplest) != 0 {
        ms = rd::<i32>(e, 0x24);
    }
    match i32::from(rd::<i16>(e, 0xE)) {
        200 => wr::<GPtr<u8>>(this, 0x24, GPtr::NULL),
        0x4B9 => {
            let t = new_task(0x74, |t| partner_shove_ctor(t, S_COMP_POT_PED_COLL_RESP, other, 1, 0x3F00_0000, 1, 0, 0, 0));
            wr::<GPtr<u8>>(this, 0x24, t);
        }
        0x395 => {
            let follower = tm_find_active_task_by_type(intel(ped(this)).byte_add(4), 0x4B7);
            if ms > 4 && !follower.is_null() {
                let leader = rd::<GPtr<u8>>(follower, 0x10);
                if !leader.is_null() {
                    let leader_ms = if rd::<u32>(leader, 0x480) != 0 {
                        rd::<i32>(leader, 0x534)
                    } else {
                        intel_get_move_state_from_go_to_task(intel(leader))
                    };
                    let seek = tm_find_active_task_by_type(intel(ped(this)).byte_add(4), 0x38B);
                    if !seek.is_null() && leader_ms < 6 {
                        let mut v = [0u32; 3];
                        let vp: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr()).cast();
                        vector_sub(vp, pos_ptr(leader), pos_ptr(ped(this)));
                        wr::<u32>(vp, 8, 0);
                        let d2 = X87::from_f64_exact(vector_magnitude_sqr(vp));
                        let r = rd::<f32>(seek, 0x1C);
                        let r2 = x(r).fmul_m(r);
                        if !d2.fsub(r2).fcom(r2).test_parity(0x05) {
                            ms = 4;
                        }
                    }
                }
            }
            let t = new_task(0x60, |t| avoid_other_ped_ctor(t, other, e.byte_add(0x14), ms));
            wr::<GPtr<u8>>(this, 0x24, t);
            if !follower.is_null() {
                wr::<u8>(t, 0x5C, rd::<u8>(t, 0x5C) | 4);
            }
        }
        _ => {}
    }
}

/// `CTaskComplexSmartFleeEntity(other, false, <defaults>)` (0x40 bytes) with the original's default-argument globals.
fn smart_flee(other: GPtr<u8>) -> GPtr<u8> {
    new_task(0x40, |t| smart_flee_entity_ctor(t, other, 0, gu(FLEE_SAFE_DIST), gu(FLEE_TIME), gu(FLEE_PERIOD), gu(FLEE_SHIFT)))
}

/// `CTaskComplexFleeAnyMeans(other, true, <defaults>)` (0x54 bytes).
fn flee_any_means(other: GPtr<u8>) -> GPtr<u8> {
    new_task(0x54, |t| {
        flee_any_means_ctor(t, other, 1, gu(FLEE_SAFE_DIST), gu(FLEE_TIME), gu(FLEE_ANY_A), gu(FLEE_ANY_B), gu(FLEE_ANY_C),
                            gu(FLEE_PERIOD), gu(FLEE_SHIFT))
    })
}

fn kill_on_foot(other: GPtr<u8>, time: i32) -> GPtr<u8> {
    new_task(0x38, |t| kill_ped_on_foot_ctor(t, other, time, 0, 0, 0, 1))
}

/// Wanted by the police (+0x478 bit 0x800) and the other ped is a cop (ped type +0x598 == 6).
fn wanted_and_cop(p: GPtr<u8>, other: GPtr<u8>) -> bool {
    rd::<u32>(p, 0x478) & 0x800 != 0 && rd::<i32>(other, 0x598) == 6
}

/// The kill-on-foot cases' fallback: wanted and a cop → flee any means; in a vehicle (+0x46C bit 0x100, +0x58C)
/// as its driver → `CTaskComplexCarDriveMissionFleeScene(vehicle)` (only when `drive` is set); else smart flee.
fn kill_fallback(p: GPtr<u8>, other: GPtr<u8>, drive: bool) -> GPtr<u8> {
    if wanted_and_cop(p, other) {
        return flee_any_means(other);
    }
    if drive && rd::<u32>(p, 0x46C) & 0x100 != 0 {
        let veh = rd::<GPtr<u8>>(p, 0x58C);
        if !veh.is_null() && vehicle_is_driver(veh, p) != 0 {
            return new_task(0x2C, |t| car_drive_mission_flee_scene_ctor(t, rd::<GPtr<u8>>(p, 0x58C)));
        }
    }
    smart_flee(other)
}

/// `CEventHandler::ComputePedThreatResponse(CEventAcquaintancePedHate*, CTask* active, CTask* simplest)` (0x4C19A0,
/// thiscall, `ret 0xC`). Event: +0xE the response task type (i16), +0x14 the other ped (nothing without it). The
/// cases write +0x24 (a failed allocation writes NULL); "nothing" leaves it untouched, as do the unlisted types:
///
/// - 200: NULL. 0x19C: `CTaskSimpleCower`. 0x102 / 0x105 / 0x106: the closest-free-scripted-attractor tasks (the
///   first with `false`). 0xF4: the ped's next response sequence (intelligence +0xD0, i8): negative or an empty
///   `CTaskSequences::ms_taskSequence` slot → nothing, else `CTaskComplexUseSequence(index)`.
/// - 0x19D: `CTaskSimpleHandsUp(5000)`. 0x1AB: `CTaskSimpleDuck(0, 0x967F, -1)`. 0x19F: `GetTaskDuck(true)` set →
///   nothing, else `CTaskSimpleDuck(0, 5000, -1)`. 0x2C0: in a vehicle (+0x58C and +0x46C bit 0x100) →
///   `CTaskComplexLeaveCar(vehicle, 0, 0, false, true)`, else nothing.
/// - 0x38D: `CTaskComplexFleeEntity(other, false, <defaults>)`. 0x38F: wanted and a cop → flee any means; else smart
///   flee, and when the event type (vtable +4) is 0x25 the response's +0x30 = 4. 0x39F: flee any means. 0x3AC:
///   smart flee with +0x30 = 4 (through NULL on a failed allocation, as the original).
/// - 0x3E8 / 0x3FF: `IsKillTaskAppropriate(ped, other, event)` → `CTaskComplexKillPedOnFoot(other, -1 / 10000, 0, 0,
///   0, true)`, else `kill_fallback` with the driver case. 0x400 / 0x405: appropriate → +0x46C |= 0x200000 /
///   0x400000 and the kill task (-1); else `kill_fallback` without it. 0x406: in a vehicle (+0x58C) → a sequence of
///   the kill task (-1) and `CTaskComplexCarDriveWander(vehicle, 0, 10.0)` (a NULL sequence is used as the original
///   does); else the kill task. 0x404: `CTaskComplexKillPedOnFootStealth`. 0x451: a player → NULL, else
///   `CTaskComplexKillCriminal(other, true)`.
/// - 0x3FC: `CTaskSimpleGunControl(other, NULL, NULL, 0, 1, 10000)`. 0x3A5: `CTaskComplexSeekEntityAiming(other,
///   10.0, 5.0)`.
/// - 0x3FE: in a vehicle and not its driver (+0x460), with an active task 0x2C5 and an instant-hit weapon (the
///   current slot's `GetWeaponInfo(type, 1)` fire type 1): `range` = the weapon's target range (+4), raised to
///   `GetPedFOVRange() + 5` when not above it (stored as f32); the 3D distance to the other ped not below `range` →
///   NULL; else `CTaskSimpleGangDriveBy(other, NULL, range, 90, 8, false)`. Any failed condition: nothing.
#[hook(addr = 0x4c19a0, conv = thiscall, name = "CEventHandler::ComputePedThreatResponse", expect = "6A FF 64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
#[allow(clippy::too_many_lines)]
pub fn event_handler_compute_ped_threat_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let other = rd::<GPtr<u8>>(e, 0x14);
    if other.is_null() {
        return;
    }
    let set = |t: GPtr<u8>| wr::<GPtr<u8>>(this, 0x24, t);
    let p = || ped(this);
    match i32::from(rd::<i16>(e, 0xE)) {
        200 => set(GPtr::NULL),
        0x19C => set(new_task(0x20, cower_ctor)),
        0x102 => set(new_task(0x10, |t| use_attractor_ctor(t, 0))),
        0x105 => set(new_task(0x10, use_attractor_run_ctor)),
        0x106 => set(new_task(0x10, use_attractor_sprint_ctor)),
        0xF4 => {
            let idx = i32::from(rd::<i8>(intel(p()), 0xD0));
            if idx < 0 || g::<u32>(TASK_SEQUENCES + (idx as usize) * 0x40).read() == 0 {
                return;
            }
            set(new_task(0x1C, |t| use_sequence_ctor(t, idx)));
        }
        0x19D => set(new_task(0x34, |t| hands_up_ctor(t, 5000))),
        0x1AB => set(new_task(0x28, |t| duck_ctor(t, 0, 0x967F, -1))),
        0x19F => {
            if !intel_get_task_duck(intel(p()), 1).is_null() {
                return;
            }
            set(new_task(0x28, |t| duck_ctor(t, 0, 5000, -1)));
        }
        0x2C0 => {
            if rd::<GPtr<u8>>(p(), 0x58C).is_null() || rd::<u32>(p(), 0x46C) & 0x100 == 0 {
                return;
            }
            set(new_task(0x34, |t| leave_car_ctor(t, rd::<GPtr<u8>>(p(), 0x58C), 0, 0, 0, 1)));
        }
        0x38D => set(new_task(0x3C, |t| {
            flee_entity_ctor(t, other, 0, gu(FLEE_ENT_DIST), gu(FLEE_ENT_A), gu(FLEE_ENT_B), gu(FLEE_ENT_C))
        })),
        0x38F => {
            if wanted_and_cop(p(), other) {
                set(flee_any_means(other));
                return;
            }
            set(smart_flee(other));
            if v_event_type(e) == 0x25 {
                let t = rd::<GPtr<u8>>(this, 0x24);
                wr::<u32>(t, 0x30, 4);
            }
        }
        0x39F => set(flee_any_means(other)),
        0x3AC => {
            let t = smart_flee(other);
            set(t);
            wr::<u32>(t, 0x30, 4);
        }
        ty @ (0x3E8 | 0x3FF) => {
            let time = if ty == 0x3E8 { -1 } else { 10000 };
            if handler_is_kill_task_appropriate(p(), other, e) != 0 {
                set(kill_on_foot(other, time));
            } else {
                set(kill_fallback(p(), other, true));
            }
        }
        ty @ (0x400 | 0x405) => {
            if handler_is_kill_task_appropriate(p(), other, e) != 0 {
                let bit = if ty == 0x400 { 0x20_0000 } else { 0x40_0000 };
                wr::<u32>(p(), 0x46C, rd::<u32>(p(), 0x46C) | bit);
                set(kill_on_foot(other, -1));
            } else {
                set(kill_fallback(p(), other, false));
            }
        }
        0x406 => {
            if rd::<GPtr<u8>>(p(), 0x58C).is_null() {
                set(kill_on_foot(other, -1));
                return;
            }
            let seq = new_task(0x40, sequence_ctor);
            let kill = kill_on_foot(other, -1);
            sequence_add_task(seq, kill);
            let wander = new_task(0x24, |t| car_drive_wander_ctor(t, rd::<GPtr<u8>>(p(), 0x58C), 0, 0x4120_0000));
            sequence_add_task(seq, wander);
            set(seq);
        }
        0x404 => set(new_task(0x48, |t| kill_ped_on_foot_stealth_ctor(t, other))),
        0x451 => {
            if ped_is_player(other) != 0 {
                set(GPtr::NULL);
            } else {
                set(new_task(0x20, |t| kill_criminal_ctor(t, other, 1)));
            }
        }
        0x3FC => set(new_task(0x3C, |t| gun_control_ctor(t, other, 0, 0, 0, 1, 10000))),
        0x3A5 => set(new_task(0x18, |t| seek_entity_aiming_ctor(t, other, 0x4120_0000, 0x40A0_0000))),
        0x3FE => {
            let pd = p();
            if rd::<u32>(pd, 0x46C) & 0x100 == 0 {
                return;
            }
            let veh = rd::<GPtr<u8>>(pd, 0x58C);
            if veh.is_null() || rd::<GPtr<u8>>(veh, 0x460) == pd {
                return;
            }
            if tm_find_active_task_by_type(intel(pd).byte_add(4), 0x2C5).is_null() {
                return;
            }
            let weapon_type = || {
                let pd = p();
                let slot = i32::from(rd::<i8>(pd, 0x718)) as isize;
                rd::<u32>(pd, 0x5A0 + slot * 0x1C)
            };
            if rd::<u32>(weapon_info_get(weapon_type(), 1), 0) != 1 {
                return;
            }
            let mut range = rd::<u32>(weapon_info_get(weapon_type(), 1), 4);
            let pd = p();
            let fov = X87::from_f64_exact(intel_get_ped_fov_range(intel(pd))).fadd_m(gf(K_FOV_PAD));
            if x(f32::from_bits(range)).fcom(fov).test_any(0x41) {
                range = f32_of(fov).to_bits();
            }
            let mut v = [0u32; 3];
            let vp: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr()).cast();
            vector_sub(vp, pos_ptr(other), pos_ptr(pd));
            if X87::from_f64_exact(vector_magnitude(vp)).fcom(x(f32::from_bits(range))).test_parity(0x05) {
                set(GPtr::NULL);
                return;
            }
            set(new_task(0x44, |t| gang_drive_by_ctor(t, other, 0, range, 0x5A, 8, 0)));
        }
        _ => {}
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_EVENT_HANDLER_COMPUTE_PED_COLLISION_WITH_PED_RESPONSE,
    HOOK_EVENT_HANDLER_COMPUTE_POTENTIAL_PED_COLLIDE_RESPONSE,
    HOOK_EVENT_HANDLER_COMPUTE_PED_THREAT_RESPONSE,
];
