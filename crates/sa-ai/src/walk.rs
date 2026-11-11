//! PORT-AI-4: locomotion tasks — `CTaskSimpleStandStill`, `CTaskSimpleGoTo` / `CTaskSimpleGoToPoint`,
//! `CTaskSimpleAchieveHeading`, `CTaskComplexGoToPointAndStandStill`, `CTaskComplexTurnToFaceEntityOrCoord`.
//! Ported from the 1.0 US bytes (gta-reversed and `docs/re/specs/AI` as reading aids only).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7CB5C;
/// `g_ikChainMan` (`IKChainManager_c`)
const IK_CHAIN_MAN: usize = 0xC15448;

orig! {
    /// `CTaskSimple::CTaskSimple` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CPed::SetMoveState(eMoveState)` (0x5DEC00, `ret 4`)
    fn ped_set_move_state(p: GPtr<u8>, s: u32) = thiscall 0x5DEC00 => unported;
    /// `CPedIntelligence::GetTaskDuck(bool)` (0x6010A0, `ret 4`)
    fn intel_get_task_duck(i: GPtr<u8>, ignore: u32) -> GPtr<u8> = thiscall 0x6010A0 => unported;
    /// `CPedIntelligence::GetTaskHold(bool)` (0x600FF0, `ret 4`)
    fn intel_get_task_hold(i: GPtr<u8>, ignore: u32) -> GPtr<u8> = thiscall 0x600FF0 => unported;
    /// `CTaskSimpleDuck::ControlDuckMove(float moveX, float moveY)` (0x6923F0, `ret 8`)
    fn duck_control_duck_move(d: GPtr<u8>, mx: f32, my: f32) = thiscall 0x6923F0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610)
    fn anim_blend_animation(clump: u32, group: u32, id: u32, delta: f32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    fn clump_get_association(clump: u32, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0, `ret 4`)
    fn ik_is_looking(this: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::AbortLookAt(CPed*, int32 blendOutTime)` (0x618280, `ret 8`)
    fn ik_abort_look_at(this: GPtr<u8>, ped: GPtr<u8>, time: u32) = thiscall 0x618280 => unported;
    /// `IKChainManager_c::GetLookAtEntity(CPed*)` (0x6181D0, `ret 4`)
    fn ik_get_look_at_entity(this: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x6181D0 => unported;
    /// `IKChainManager_c::LookAt(purpose, ped, target, time, bone, posn, useTorso, speed, blendTime, priority,
    /// forceLooking)` (0x618970, `ret 0x2C`)
    fn ik_look_at(
        this: GPtr<u8>, purpose: u32, ped: GPtr<u8>, target: u32, time: u32, bone: u32, posn: GPtr<u8>,
        torso: u32, speed: u32, blend: u32, prio: u32, force: u32
    ) -> u32 = thiscall 0x618970 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(id: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CEntity::GetIsOnScreen()` (0x534540)
    fn entity_get_is_on_screen(e: GPtr<u8>) -> u8 = thiscall 0x534540 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810, `ret 4`)
    fn tm_get_task_secondary(tm: GPtr<u8>, slot: u32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): out = a - b
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CVector::MagnitudeSqr()` (0x406DA0, ST0)
    fn vector_magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CVector2D::Normalise()` (0x44E480)
    fn vector2d_normalise(v: GPtr<u8>) = thiscall 0x44E480 => unported;
    /// `FUN_00509760(const CVector*, const CVector*)` (cdecl, AL): true unless the three components compare
    /// equal (`operator!=`)
    fn vector_ne(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x509760 => unported;
    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(int32, bool, bool, float)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, time: u32, looped: u32, keep: u32, delta: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskSimpleStandStill::ProcessPed(CPed*)` (0x62F370, `ret 4`)
    fn stand_still_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x62F370 => unported;
    /// `CTaskSimpleStandStill::~CTaskSimpleStandStill` (0x62F360)
    fn stand_still_dtor(t: GPtr<u8>) = thiscall 0x62F360 => unported;
    /// `CTaskSimpleGoTo::HasCircledTarget(CPed*)` (0x667A10, `ret 4`)
    fn goto_has_circled_target(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x667A10 => unported;
    /// `CTaskSimpleGoTo::QuitIK(CPed*)` (0x667CA0, `ret 4`)
    fn goto_quit_ik(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x667CA0 => unported;
    /// `CTaskSimpleGoTo::SetUpIK(CPed*)` (0x667AD0, `ret 4`)
    fn goto_set_up_ik(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x667AD0 => unported;
    /// `CEventScanner::ScanForEventsNow(const CPed&, bool)` (0x6053D0, `ret 8`)
    fn scanner_scan_for_events_now(s: GPtr<u8>, ped: GPtr<u8>, dont_scan_cars: u32) = thiscall 0x6053D0 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPlayerPed::SetPlayerMoveBlendRatio(CVector*)` (0x60C520, `ret 4`)
    fn player_set_move_blend_ratio(p: GPtr<u8>, v: GPtr<u8>) = thiscall 0x60C520 => unported;
    /// `CPlayerPed::SetRealMoveAnim()` (0x60A9C0)
    fn player_set_real_move_anim(p: GPtr<u8>) = thiscall 0x60A9C0 => unported;
    /// `CPlayerPed::ControlButtonSprint(eSprintType)` (0x60A610, `ret 4`, ST0)
    fn player_control_button_sprint(p: GPtr<u8>, ty: u32) -> f64 = thiscall 0x60A610 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn weapon_info_get(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `SurfaceInfos_c::CantSprintOn(SurfaceId)` (0x55E870, `ret 4`)
    fn surface_cant_sprint_on(s: GPtr<u8>, id: u32) -> u8 = thiscall 0x55E870 => unported;
    /// `CAnimBlendAssocGroup::GetAnimation(AnimationId)` (0x4CE090, `ret 4`)
    fn assoc_group_get_animation(g: GPtr<u8>, id: u32) -> GPtr<u8> = thiscall 0x4CE090 => unported;
    /// `CPed` vtable +0x5C (`SetMoveAnim`, no stack arguments)
    fn v_ped_set_move_anim(p: GPtr<u8>) = thiscall vslot 0x5C => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CBE0, ST0)
    fn general_get_radian_angle_between_points(x1: f32, y1: f32, x2: f32, y2: f32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn general_limit_radian_angle(a: f32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CPed::RestoreHeadingRate()` (0x5DFD60)
    fn ped_restore_heading_rate(p: GPtr<u8>) = thiscall 0x5DFD60 => unported;
    /// `CTaskSimpleAchieveHeading::SetUpIK(CPed*)` (0x667F20, `ret 4`)
    fn achieve_heading_set_up_ik(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x667F20 => unported;
    /// `CTaskComplexGoToPointAndStandStill::CreateSubTask(int32 type, CPed*)` (0x6682D0, `ret 8`)
    fn gtpss_create_sub_task(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x6682D0 => unported;
    /// `CTaskComplexGoToPointAndStandStill::SelectMoveState(CTaskSimpleGoToPoint*, CPed*, float, float)` (0x668250,
    /// `ret 0x10`)
    fn gtpss_select_move_state(t: GPtr<u8>, task: GPtr<u8>, ped: GPtr<u8>, r: u32, c: u32) = thiscall 0x668250 => unported;
    /// `CVector::DotProduct(const CVector*, const CVector*)` (0x40FDB0, ST0)
    fn vector_dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `CTaskSimplePause::CTaskSimplePause(int32)` (0x48E750, `ret 4`)
    fn pause_ctor(t: GPtr<u8>, time: u32) -> GPtr<u8> = thiscall 0x48E750 => unported;
    /// `CTaskComplexLeaveCar::CTaskComplexLeaveCar(CVehicle*, int32, int32, bool, bool)` (0x63B8C0, `ret 0x14`)
    fn leave_car_ctor(t: GPtr<u8>, veh: u32, a1: u32, a2: u32, a3: u32, a4: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    /// `CTaskSimpleGoToPoint::CTaskSimpleGoToPoint(...)` (0x667CD0, `ret 0x14`)
    fn goto_point_ctor_call(t: GPtr<u8>, state: u32, target: GPtr<u8>, radius: u32, over: u32, exact: u32) -> GPtr<u8> = thiscall 0x667CD0 => unported;
    /// `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: u32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplex::CreateFirstSubTask(CPed*)` (vtable +0x24)
    fn v_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CTaskComplexTurnToFaceEntityOrCoord::ComputeTargetHeading(CPed*)` (0x66B9D0, `ret 4`, ST0)
    fn turn_compute_target_heading(t: GPtr<u8>, ped: GPtr<u8>) -> f64 = thiscall 0x66B9D0 => unported;
    /// `CTaskComplexTurnToFaceEntityOrCoord` (entity) constructor (0x66B890, `ret 0xC`)
    fn turn_ctor_entity(t: GPtr<u8>, e: u32, rate: u32, tol: u32) -> GPtr<u8> = thiscall 0x66B890 => unported;
    /// `CTaskComplexTurnToFaceEntityOrCoord` (coords) constructor (0x66B910, `ret 0xC`)
    fn turn_ctor_coords(t: GPtr<u8>, pos: GPtr<u8>, rate: u32, tol: u32) -> GPtr<u8> = thiscall 0x66B910 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CTaskSimpleCarWaitForDoorNotToBeInUse` constructor (0x646B70, `ret 0xC`)
    fn car_wait_for_door_ctor(t: GPtr<u8>, veh: u32, door: u32, a3: u32) -> GPtr<u8> = thiscall 0x646B70 => unported;
    /// `CTaskSimpleCarGoToPointNearDoorUntilDoorNotInUse` constructor (0x646D70, `ret 0x10`)
    fn car_go_near_door_ctor(t: GPtr<u8>, veh: u32, door: u32, pos: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x646D70 => unported;
    /// `CTaskComplexFollowPointRoute` constructor (0x671510, `ret 0x20`)
    fn follow_point_route_ctor(
        t: GPtr<u8>, ms: u32, route: u32, mode: u32, r1: u32, r2: u32, a6: u32, a7: u32, a8: u32
    ) -> GPtr<u8> = thiscall 0x671510 => unported;
    /// `CCarEnterExit::GetPositionToOpenCarDoor(CVector* out, CVehicle*, int32 door)` (0x64E740, returns `out`)
    fn get_position_to_open_car_door(out: GPtr<u8>, veh: u32, door: u32) -> GPtr<u8> = cdecl 0x64E740 => unported;
    /// `CTaskComplexGoToCarDoorAndStandStill::CreateSubTask(int32, CPed*)` (0x64A5F0, `ret 8`)
    fn car_door_create_sub_task(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x64A5F0 => unported;
    /// `CTaskComplexGoToCarDoorAndStandStill::IsVehicleInRange(const CPed&)` (0x6458A0, `ret 4`)
    fn car_door_is_vehicle_in_range(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6458A0 => unported;
    /// `CTaskComplexGoToCarDoorAndStandStill::ComputeRouteToDoor(const CPed&)` (0x645910, `ret 4`)
    fn car_door_compute_route_to_door(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x645910 => unported;
    /// `CCarEnterExit::IsPathToDoorBlockedByVehicleCollisionModel(const CPed*, CVehicle*, const CVector&)` (0x651210)
    fn path_to_door_blocked(ped: GPtr<u8>, veh: u32, pos: GPtr<u8>) -> u8 = cdecl 0x651210 => unported;
    /// `CCarEnterExit::GetNearestCarDoor(const CPed*, const CVehicle*, CVector&, int32&)` (0x6528F0)
    fn get_nearest_car_door(ped: GPtr<u8>, veh: u32, pos: GPtr<u8>, door: GPtr<u8>) -> u8 = cdecl 0x6528F0 => unported;
    /// `CCarEnterExit::GetNearestCarPassengerDoor(ped, veh, CVector*, int32*, bool, bool, bool)` (0x650BB0)
    fn get_nearest_car_passenger_door(
        ped: GPtr<u8>, veh: u32, pos: GPtr<u8>, door: GPtr<u8>, a: u32, b: u32, c: u32
    ) -> u8 = cdecl 0x650BB0 => unported;
    /// `CCarEnterExit::ComputePassengerIndexFromCarDoor(const CVehicle*, int32)` (0x64F1E0)
    fn compute_passenger_index_from_car_door(veh: u32, door: u32) -> i32 = cdecl 0x64F1E0 => unported;
}

/// Pointer to a Rust stack buffer (stands in for an object the original builds on its own stack).
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// Entity position: matrix +0x30, else the placement at +4.
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

fn time_ms() -> u32 {
    g::<u32>(TIME_MS).read()
}

/// `fld dword [addr]` of a float constant/global.
fn gf(addr: usize) -> X87 {
    x(g::<f32>(addr).read())
}

/// `fabs`
fn fabs(v: X87) -> X87 {
    X87::from_f64_exact(v.to_f64().abs())
}

fn ped_intel(ped: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(ped, 0x47C)
}

fn ik() -> GPtr<u8> {
    GPtr::from_addr(IK_CHAIN_MAN)
}

/// The inlined `QuitIK` body: bit 0x10 of +0x1C set and the ped is looking → abort the look-at (250 ms).
fn quit_ik_inline(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u8>(this, 0x1C) & 0x10 != 0 && ik_is_looking(ik(), ped) != 0 {
        ik_abort_look_at(ik(), ped, 0xFA);
    }
}

// ---- CTaskSimpleStandStill ------------------------------------------------------------------------------

/// `CTaskSimpleStandStill::CTaskSimpleStandStill(int32 time, bool looped, bool keepAnim, float delta)` (0x62F310,
/// `ret 0x10`): `CTaskSimple` base, vtable 0x86DD2C, +8 = time, timer +0xC/+0x10 = 0 and its flags +0x14/+0x15
/// clear, +0x18/+0x19 = the bools, +0x1C = delta.
#[hook(addr = 0x62F310, conv = thiscall, name = "CTaskSimpleStandStill::Constructor", expect = "56 8B F1 E8 78 B0 FE FF")]
pub fn stand_still_ctor_port(this: GPtr<u8>, time: u32, looped: u8, keep: u8, delta: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 8, time);
    wr::<u32>(this, 0, 0x86DD2C);
    wr::<u32>(this, 0xC, 0);
    wr::<u32>(this, 0x10, 0);
    wr::<u8>(this, 0x14, 0);
    wr::<u8>(this, 0x15, 0);
    wr::<u32>(this, 0x1C, delta);
    wr::<u8>(this, 0x18, looped);
    wr::<u8>(this, 0x19, keep);
    this
}

/// Ducking (ped +0x46C bit 0x4000000) with a duck task: `ControlDuckMove(0, 0)` on a second `GetTaskDuck(0)`;
/// returns false when the ped is not ducking or has no duck task.
fn stand_still_duck(ped: GPtr<u8>) -> bool {
    if rd::<u32>(ped, 0x46C) & 0x0400_0000 == 0 || intel_get_task_duck(ped_intel(ped), 0).is_null() {
        return false;
    }
    let d = intel_get_task_duck(ped_intel(ped), 0);
    duck_control_duck_move(d, 0.0, 0.0);
    true
}

/// `CTaskSimpleStandStill::ProcessPed(CPed*)` (0x62F370, `ret 4`; the timer start continues through the island
/// 0x403793, which loads `CTimer::m_snTimeInMilliseconds` through `[0x42B7D3] ^ [0x156074C]` = 0xB7CB84). First
/// call with time >= 0 (signed): timer = (now, time), started; unless ped +0x46C bit 0x100: `SetMoveState(1)`, ped
/// +0x538 = 1, duck move or `BlendAnimation(clump, group, 3, delta)`, and +0x14 of ped +0x480 = 0. Every call:
/// duck move or `SetMoveState(1)`; a kept anim (+0x19) whose association 3 has blend amount > [0x862CD0] returns
/// AL = 1; an unlooped (+0x18) started timer returns EAX = 1 once expired (restart flag +0x15 resets the start).
#[hook(addr = 0x62F370, conv = thiscall, name = "CTaskSimpleStandStill::ProcessPed", expect = "83 EC 08 56 8B 74 24 10")]
pub fn stand_still_process_ped_port(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x14) == 0 && rd::<i32>(this, 8) >= 0 {
        let t = rd::<u32>(this, 8);
        wr::<u32>(this, 0xC, time_ms());
        wr::<u32>(this, 0x10, t);
        wr::<u8>(this, 0x14, 1);
        if rd::<u32>(ped, 0x46C) & 0x100 == 0 {
            ped_set_move_state(ped, 1);
            let ducking = rd::<u32>(ped, 0x46C) & 0x0400_0000 != 0;
            wr::<u32>(ped, 0x538, 1);
            let ducked = ducking && {
                if intel_get_task_duck(ped_intel(ped), 0).is_null() {
                    false
                } else {
                    let d = intel_get_task_duck(ped_intel(ped), 0);
                    duck_control_duck_move(d, 0.0, 0.0);
                    true
                }
            };
            if !ducked {
                anim_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), 3, rd::<f32>(this, 0x1C));
            }
            let pd = rd::<GPtr<u8>>(ped, 0x480);
            if !pd.is_null() {
                wr::<u32>(pd, 0x14, 0);
            }
        }
    }
    if !stand_still_duck(ped) {
        ped_set_move_state(ped, 1);
    }
    if rd::<u8>(this, 0x19) != 0 {
        let a = clump_get_association(rd::<u32>(ped, 0x18), 3);
        if !a.is_null() && !x(rd::<f32>(a, 0x18)).fcom(gf(0x862CD0)).test_any(0x41) {
            return 1;
        }
    }
    if rd::<u8>(this, 0x18) == 0 && rd::<u8>(this, 0x14) != 0 {
        if rd::<u8>(this, 0x15) != 0 {
            wr::<u32>(this, 0xC, time_ms());
            wr::<u8>(this, 0x15, 0);
        }
        let end = rd::<u32>(this, 0x10).wrapping_add(rd::<u32>(this, 0xC));
        if time_ms() >= end {
            return 1;
        }
    }
    0
}

// ---- CTaskSimpleGoTo / CTaskSimpleGoToPoint -------------------------------------------------------------

/// `CTaskSimpleGoTo::HasCircledTarget(CPed*)` (0x667A10, `ret 4`): copies the ped position; unless ped +0x478 bit
/// 0x40 it requires |z - target z (+0x14)| < [0x858CA0]. Sets quadrant bits of +0x1C: 1 (x < tx) or 2 (x > tx),
/// 4 (y < ty) or 8 (y > ty); true when all four were seen.
#[hook(addr = 0x667A10, conv = thiscall, name = "CTaskSimpleGoTo::HasCircledTarget", expect = "8B 54 24 04 8B 42 14")]
pub fn goto_has_circled_target_port(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let p = ent_pos(ped);
    let (px, py, pz) = (rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8));
    if rd::<u8>(ped, 0x478) & 0x40 == 0 {
        let dz = fabs(x(pz).fsub_m(rd::<f32>(this, 0x14)));
        if dz.fcom(gf(0x858CA0)).test_parity(5) {
            return 0;
        }
    }
    let c = x(px).fcom(x(rd::<f32>(this, 0xC)));
    if !c.test_parity(5) {
        wr::<u8>(this, 0x1C, rd::<u8>(this, 0x1C) | 1);
    } else if !x(px).fcom(x(rd::<f32>(this, 0xC))).test_any(0x41) {
        wr::<u8>(this, 0x1C, rd::<u8>(this, 0x1C) | 2);
    }
    let c = x(py).fcom(x(rd::<f32>(this, 0x10)));
    if !c.test_parity(5) {
        wr::<u8>(this, 0x1C, rd::<u8>(this, 0x1C) | 4);
    } else if !x(py).fcom(x(rd::<f32>(this, 0x10))).test_any(0x41) {
        wr::<u8>(this, 0x1C, rd::<u8>(this, 0x1C) | 8);
    }
    (rd::<u8>(this, 0x1C) & 0xF == 0xF) as u8
}

/// `CTaskSimpleGoTo::SetUpIK(CPed*)` (0x667AD0, `ret 4`): a look-at in the walking direction for an on-screen ped
/// that has no IK yet (+0x1C bit 0x10), no look-at entity, no secondary task 5, is not the player with pad word
/// +0x10E = 0 and whose parent task (+4) is neither 0x395 nor 0x3AB (two `GetTaskType` calls). The direction
/// target - pos must have squared length > [0x85EED4]; normalised, its dot with the matrix forward (+0x10, no
/// null check) must be < [0xC18D48]. Look-at point: target + 2·dir, z = (2·dir.z + pos.z) + [0x86FD40].
#[hook(addr = 0x667AD0, conv = thiscall, name = "CTaskSimpleGoTo::SetUpIK", expect = "83 EC 24 55 56 57 6A 00")]
pub fn goto_set_up_ik_port(this: GPtr<u8>, ped: GPtr<u8>) {
    let pad = pad_get_pad(0);
    if entity_get_is_on_screen(ped) == 0 || rd::<u8>(this, 0x1C) & 0x10 != 0 {
        return;
    }
    if ik_get_look_at_entity(ik(), ped) != 0 {
        return;
    }
    if !tm_get_task_secondary(ped_intel(ped).byte_add(4), 5).is_null() {
        return;
    }
    if ped == find_player_ped(-1) && rd::<u16>(pad, 0x10E) == 0 {
        return;
    }
    let parent = rd::<GPtr<u8>>(this, 4);
    if !parent.is_null() {
        if v_get_task_type(parent) == 0x395 {
            return;
        }
        if v_get_task_type(rd::<GPtr<u8>>(this, 4)) == 0x3AB {
            return;
        }
    }
    let mut d = [0u32; 3];
    let dp = buf(&mut d);
    vector_sub(dp, this.byte_add(0xC), ent_pos(ped));
    let m2 = X87::from_f64_exact(vector_magnitude_sqr(dp));
    if m2.fcom(gf(0x85EED4)).test_any(0x41) {
        return;
    }
    vector_normalise(dp);
    let (vx, vy, vz) = (rd::<f32>(dp, 0), rd::<f32>(dp, 4), rd::<f32>(dp, 8));
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let dot = x(vz)
        .fmul_m(rd::<f32>(m, 0x18))
        .fadd(x(vy).fmul_m(rd::<f32>(m, 0x14)))
        .fadd(x(vx).fmul_m(rd::<f32>(m, 0x10)));
    if dot.fcom(gf(0xC18D48)).test_parity(5) {
        return;
    }
    let (tx, ty) = (rd::<f32>(this, 0xC), rd::<f32>(this, 0x10));
    let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    let pz = x(rd::<f32>(pos, 8));
    let vz2 = f32_of(x(vz).fadd(x(vz)));
    let nx = f32_of(x(vx).fadd(x(vx)).fadd_m(tx));
    let ny = f32_of(x(ty).fadd(x(vy).fadd(x(vy))));
    let nz = f32_of(x(vz2).fadd(pz).fadd_m(g::<f32>(0x86FD40).read()));
    let mut posn = [nx.to_bits(), ny.to_bits(), nz.to_bits()];
    ik_look_at(ik(), 0x86FD44, ped, 0, 0x1388, 0xFFFF_FFFF, buf(&mut posn), 0, 0x3E80_0000, 0x1F4, 3, 0);
    wr::<u8>(this, 0x1C, rd::<u8>(this, 0x1C) | 0x10);
}

/// `CTaskSimpleGoTo::QuitIK(CPed*)` (0x667CA0, `ret 4`; continues through the island 0x40586C, which loads ECX =
/// `g_ikChainMan`): with +0x1C bit 0x10 and the ped looking, `AbortLookAt(ped, 250)`.
#[hook(addr = 0x667CA0, conv = thiscall, name = "CTaskSimpleGoTo::QuitIK", expect = "F6 41 1C 10 74 25")]
pub fn goto_quit_ik_port(this: GPtr<u8>, ped: GPtr<u8>) {
    quit_ik_inline(this, ped);
}

/// `CTaskSimpleGoToPoint::UpdatePoint(const CVector& target, float radius, bool force)` (0x645700, `ret 0xC`):
/// unless forced, keeps the task when the target compares equal (`FUN_00509760`) and the radius is the same
/// (an unordered radius compare updates); otherwise copies target and radius and sets +0x1C = (+0x1C & 0xF0) |
/// 0x20 (the quadrant bits and the IK bit cleared, "new target" set).
#[hook(addr = 0x645700, conv = thiscall, name = "CTaskSimpleGoToPoint::UpdatePoint", expect = "8A 44 24 0C 84 C0")]
pub fn goto_point_update_point(this: GPtr<u8>, target: GPtr<u8>, radius: f32, force: u8) {
    if force == 0
        && vector_ne(this.byte_add(0xC), target) == 0
        && !x(rd::<f32>(this, 0x18)).fcom(x(radius)).test_parity(0x44)
    {
        return;
    }
    wr::<u32>(this, 0xC, rd::<u32>(target, 0));
    wr::<u32>(this, 0x10, rd::<u32>(target, 4));
    wr::<u32>(this, 0x14, rd::<u32>(target, 8));
    let b = rd::<u8>(this, 0x1C);
    wr::<f32>(this, 0x18, radius);
    wr::<u8>(this, 0x1C, (b & 0xF0) | 0x20);
}

/// `CTaskSimpleGoToPoint::CTaskSimpleGoToPoint(int32 moveState, const CVector& target, float radius, bool
/// mustOverShoot, bool stopExactly)` (0x667CD0, `ret 0x14`): `CTaskSimple` base, vtable 0x86FD1C (`CTaskSimpleGoTo`)
/// then 0x86FD50; +8 = move state, +0xC = target, +0x18 = radius, +0x1C &= 0xC0, +0x20 = (+0x20 & 0xE0) |
/// (stopExactly & 1) << 3 | (mustOverShoot & 1).
#[hook(addr = 0x667CD0, conv = thiscall, name = "CTaskSimpleGoToPoint::Constructor", expect = "56 8B F1 E8 B8 26 FB FF")]
pub fn goto_point_ctor(this: GPtr<u8>, state: u32, target: GPtr<u8>, radius: u32, over: u8, exact: u8) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 8, state);
    wr::<u32>(this, 0, 0x86FD1C);
    wr::<u32>(this, 0xC, rd::<u32>(target, 0));
    wr::<u32>(this, 0x10, rd::<u32>(target, 4));
    wr::<u32>(this, 0x14, rd::<u32>(target, 8));
    let b1c = rd::<u8>(this, 0x1C);
    let b20 = rd::<u8>(this, 0x20);
    wr::<u8>(this, 0x1C, b1c & 0xC0);
    wr::<u32>(this, 0x18, radius);
    wr::<u32>(this, 0, 0x86FD50);
    wr::<u8>(this, 0x20, ((exact & 1) << 3) | (b20 & 0xE0) | (over & 1));
    this
}

/// `CTaskSimpleGoToPoint::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x667D60, `ret 0xC`): quits the IK
/// (inlined `QuitIK`); priority 1 (immediate) → true; priority 2 → a stack `CTaskSimpleStandStill(0, false, false,
/// 8.0)` runs `ProcessPed` once and is destroyed, true; otherwise +0x20 |= 2 (quit at the next point), false.
#[hook(addr = 0x667D60, conv = thiscall, name = "CTaskSimpleGoToPoint::MakeAbortable", expect = "64 A1 00 00 00 00")]
pub fn goto_point_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, prio: i32, _ev: GPtr<u8>) -> u8 {
    quit_ik_inline(this, ped);
    match prio {
        1 => 1,
        2 => {
            let mut s = [0u32; 8];
            let sp = buf(&mut s);
            stand_still_ctor(sp, 0, 0, 0, 0x4100_0000);
            stand_still_process_ped(sp, ped);
            stand_still_dtor(sp);
            1
        }
        _ => {
            wr::<u8>(this, 0x20, rd::<u8>(this, 0x20) | 2);
            0
        }
    }
}

/// The player's move control in `CTaskSimpleGoToPoint::ProcessPed` (0x66D885..0x66D9D6, the non-duck branch for a
/// player ped): sprint (state 7) when the weapon (+0x718 slot) allows it, no held object, the ped data +0x84 byte
/// is 0, the surface (+0xBA) allows sprinting, walk/run anims (1, 2) of the ped's group differ and
/// `ControlButtonSprint(0)` >= [0x858624]; otherwise a sprinting ped (+0x534 == 7) goes back to run (6). Then the
/// blend ratio from the target (+0x20 bit 1) or none, and `SetRealMoveAnim`.
fn goto_point_player_move(this: GPtr<u8>, ped: GPtr<u8>) {
    let pad = pad_get_pad(0);
    if rd::<u16>(pad, 0x10E) != 0 {
        player_set_move_blend_ratio(ped, GPtr::NULL);
        player_set_real_move_anim(ped);
        return;
    }
    let slot = rd::<i8>(ped, 0x718) as i32;
    let wt = rd::<u32>(ped, (slot * 0x1C + 0x5A0) as isize);
    let wi = weapon_info_get(wt, 1);
    let mut sprint = false;
    let mut check = (rd::<u32>(wi, 0x18) >> 9) & 1 == 0;
    if check && !intel_get_task_hold(ped_intel(ped), 0).is_null() {
        let h = intel_get_task_hold(ped_intel(ped), 0);
        if !rd::<GPtr<u8>>(h, 0x38).is_null() {
            check = false;
        }
    }
    if check {
        let grp = g::<GPtr<u8>>(0xB4EA34).read().byte_add((rd::<u32>(ped, 0x4D4).wrapping_mul(20)) as i32 as isize);
        let pd = rd::<GPtr<u8>>(ped, 0x480);
        if rd::<u8>(pd, 0x84) == 0
            && surface_cant_sprint_on(GPtr::from_addr(0xB79538), rd::<u8>(ped, 0xBA) as u32) == 0
        {
            let a1 = rd::<u32>(assoc_group_get_animation(grp, 1), 0x10);
            let a2 = rd::<u32>(assoc_group_get_animation(grp, 2), 0x10);
            if a1 != a2 {
                let s = X87::from_f64_exact(player_control_button_sprint(ped, 0));
                if !s.fcom(gf(0x858624)).test_any(1) {
                    ped_set_move_state(ped, 7);
                    sprint = true;
                }
            }
        }
    }
    if !sprint && rd::<u32>(ped, 0x534) == 7 {
        ped_set_move_state(ped, 6);
    }
    if rd::<u8>(this, 0x20) & 1 != 0 {
        player_set_move_blend_ratio(ped, this.byte_add(0xC));
    } else {
        player_set_move_blend_ratio(ped, GPtr::NULL);
    }
    player_set_real_move_anim(ped);
}

/// The common tail of `CTaskSimpleGoToPoint::ProcessPed` (0x66DB16): a pending new target (+0x1C bit 0x20) quits
/// the IK and clears the bit; `SetUpIK`; +0x20 bit 0x10 cleared; false.
fn goto_point_tail(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let b = rd::<u8>(this, 0x1C);
    if b & 0x20 != 0 {
        if b & 0x10 != 0 && ik_is_looking(ik(), ped) != 0 {
            ik_abort_look_at(ik(), ped, 0xFA);
        }
        wr::<u8>(this, 0x1C, rd::<u8>(this, 0x1C) & 0xDF);
    }
    goto_set_up_ik(this, ped);
    wr::<u8>(this, 0x20, rd::<u8>(this, 0x20) & 0xEF);
    0
}

/// `CTaskSimpleGoToPoint::ProcessPed(CPed*)` (0x66D710, `ret 4`). Ped +0x528 |= 8. Circled the target (and +0x20
/// bit 0x10 clear) → +0x20 |= 4, quit IK, true. Pending scans (ped +0x474 bit 8 / bit 7) run
/// `ScanForEventsNow(ped, false/true)` and skip the movement. Otherwise: the 2D direction to the target (unrounded
/// copy kept for the dot products), |dz|, squared 2D distance; move control (duck move with y = -[0x858F14] or
/// -[0x858F34] for state 4; `SetMoveState(+8)` then the player's sprint / blend ratio, or the ped's `SetMoveAnim`);
/// arrival when the squared distance < radius² (and |dz| < [0x858CA0] unless ped +0x478 bit 0x40, and for
/// +0x20 bit 8 only after passing the point); a predicted overshoot at the next step (dot <= [0x858B50]) ends the
/// task unless bit 8; +0x20 bit 2 (abort requested) quits without setting bit 4; else the heading +0x55C =
/// `LimitRadianAngle(GetRadianAngleBetweenPoints(dir, 0, 0))`. The original keeps the position pointer and then a
/// flag byte in its own argument slot (Rust locals here).
#[hook(addr = 0x66D710, conv = thiscall, name = "CTaskSimpleGoToPoint::ProcessPed", expect = "83 EC 28 53 56 8B 74 24 34")]
pub fn goto_point_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    wr::<u32>(ped, 0x528, rd::<u32>(ped, 0x528) | 8);
    if goto_has_circled_target(this, ped) != 0 {
        let f = rd::<u8>(this, 0x20);
        if f & 0x10 == 0 {
            wr::<u8>(this, 0x20, f | 4);
            quit_ik_inline(this, ped);
            return 1;
        }
    }
    let fl = rd::<u32>(ped, 0x474);
    if fl & 8 != 0 {
        scanner_scan_for_events_now(ped_intel(ped).byte_add(0x18C), ped, 0);
        wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) & 0xFFFF_FFF7);
        return goto_point_tail(this, ped);
    }
    if fl & 0x80 != 0 {
        scanner_scan_for_events_now(ped_intel(ped).byte_add(0x18C), ped, 1);
        wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) & 0xFFFF_FF7F);
        return goto_point_tail(this, ped);
    }
    let pos = ent_pos(ped);
    let tgt = this.byte_add(0xC);
    let dx = f32_of(x(rd::<f32>(tgt, 0)).fsub_m(rd::<f32>(pos, 0)));
    let dy = f32_of(x(rd::<f32>(tgt, 4)).fsub_m(rd::<f32>(pos, 4)));
    let mut dir = [dx.to_bits(), dy.to_bits()];
    let dz = f32_of(fabs(x(rd::<f32>(pos, 8)).fsub_m(rd::<f32>(tgt, 8))));
    let d2 = f32_of(x(dy).fmul_m(dy).fadd(x(dx).fmul_m(dx)));
    let dirp = buf(&mut dir);
    vector2d_normalise(dirp);
    if !intel_get_task_duck(ped_intel(ped), 0).is_null() {
        let c = if rd::<u32>(this, 8) == 4 { gf(0x858F34) } else { gf(0x858F14) };
        let my = f32_of(c.fchs());
        let d = intel_get_task_duck(ped_intel(ped), 0);
        duck_control_duck_move(d, 0.0, my);
    } else {
        ped_set_move_state(ped, rd::<u32>(this, 8));
        if ped_is_player(ped) != 0 {
            goto_point_player_move(this, ped);
        } else {
            v_ped_set_move_anim(ped);
        }
    }
    let r = x(rd::<f32>(this, 0x18));
    let inside = !x(d2).fcom(r.fmul(r)).test_parity(5);
    // dot of the step since the last position with the unrounded direction: < [0x858B50] = moving away
    let ldx = x(rd::<f32>(tgt, 0)).fsub_m(rd::<f32>(this, 0x24));
    let ldy = x(rd::<f32>(tgt, 4)).fsub_m(rd::<f32>(this, 0x28));
    let dot = x(dy).fmul(ldy).fadd(ldx.fmul_m(dx));
    let passed = !dot.fcom(gf(0x858B50)).test_parity(5);
    wr::<u32>(this, 0x24, rd::<u32>(pos, 0));
    wr::<u32>(this, 0x28, rd::<u32>(pos, 4));
    wr::<u32>(this, 0x2C, rd::<u32>(pos, 8));
    if inside {
        let cl = rd::<u8>(this, 0x20);
        if cl & 0x10 == 0
            && (rd::<u8>(ped, 0x478) & 0x40 != 0 || !x(dz).fcom(gf(0x858CA0)).test_parity(5))
            && (cl & 8 == 0 || passed)
        {
            wr::<u8>(this, 0x20, cl | 4);
            goto_quit_ik(this, ped);
            return 1;
        }
    }
    let cl = rd::<u8>(this, 0x20);
    if cl & 8 == 0 {
        let ts = gf(TIME_STEP);
        let nx = f32_of(ts.fmul_m(rd::<f32>(ped, 0x44)).fadd_m(rd::<f32>(pos, 0)));
        let ny = gf(TIME_STEP).fmul_m(rd::<f32>(ped, 0x48)).fadd_m(rd::<f32>(pos, 4));
        let ex = x(rd::<f32>(tgt, 0)).fsub_m(nx);
        let ey = x(rd::<f32>(tgt, 4)).fsub(ny);
        let dot2 = x(dy).fmul(ey).fadd(ex.fmul_m(dx));
        if !dot2.fcom(gf(0x858B50)).test_parity(0x41) && cl & 0x10 == 0 {
            wr::<u8>(this, 0x20, cl | 4);
            goto_quit_ik(this, ped);
            return 1;
        }
    }
    if cl & 2 != 0 {
        // abort requested: quit without setting bit 4
        goto_quit_ik(this, ped);
        return 1;
    }
    let a = general_get_radian_angle_between_points(f32::from_bits(dir[0]), f32::from_bits(dir[1]), 0.0, 0.0);
    let a = f32_of(X87::from_f64_exact(a));
    let h = general_limit_radian_angle(a);
    wr::<f32>(ped, 0x55C, f32_of(X87::from_f64_exact(h)));
    goto_point_tail(this, ped)
}

// ---- CTaskSimpleAchieveHeading --------------------------------------------------------------------------

/// `CTaskSimpleAchieveHeading::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x667EB0, `ret 0xC`): quits
/// the look-at (+0x14 bit 0); priority 2: ped wanted heading +0x55C = current +0x558, `RestoreHeadingRate`, true;
/// otherwise tolerance +0x10 = 2π (the task finishes next time), +0x55C = +0x558, false.
#[hook(addr = 0x667EB0, conv = thiscall, name = "CTaskSimpleAchieveHeading::MakeAbortable", expect = "56 8B 74 24 08 57")]
pub fn achieve_heading_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, prio: i32, _ev: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x14) & 1 != 0 && ik_is_looking(ik(), ped) != 0 {
        ik_abort_look_at(ik(), ped, 0xFA);
    }
    if prio == 2 {
        wr::<f32>(ped, 0x55C, f32_of(x(rd::<f32>(ped, 0x558))));
        ped_restore_heading_rate(ped);
        return 1;
    }
    wr::<u32>(this, 0x10, 0x40C9_0FDB);
    wr::<f32>(ped, 0x55C, f32_of(x(rd::<f32>(ped, 0x558))));
    0
}

/// `CTaskSimpleAchieveHeading::SetUpIK(CPed*)` (0x667F20, `ret 4`): unless the ped is the player with pad word
/// +0x10E = 0, off screen, already looking (+0x14 bit 0), has a look-at entity or secondary task 5, or the parent
/// is 0x395/0x3AB: look at (pos.x + 2·fsin(h), pos.y + 2·fcos(h), pos.z + [0x86FD40]) for 5 s; +0x14 |= 1.
#[hook(addr = 0x667F20, conv = thiscall, name = "CTaskSimpleAchieveHeading::SetUpIK", expect = "83 EC 18 53 56 57")]
pub fn achieve_heading_set_up_ik_port(this: GPtr<u8>, ped: GPtr<u8>) {
    let pad = pad_get_pad(0);
    if ped == find_player_ped(-1) && rd::<u16>(pad, 0x10E) == 0 {
        return;
    }
    if entity_get_is_on_screen(ped) == 0 || rd::<u8>(this, 0x14) & 1 != 0 {
        return;
    }
    if ik_get_look_at_entity(ik(), ped) != 0 || !tm_get_task_secondary(ped_intel(ped).byte_add(4), 5).is_null() {
        return;
    }
    let parent = rd::<GPtr<u8>>(this, 4);
    if !parent.is_null()
        && (v_get_task_type(parent) == 0x395 || v_get_task_type(rd::<GPtr<u8>>(this, 4)) == 0x3AB)
    {
        return;
    }
    let pos = ent_pos(ped);
    let px = f32_of(x(rd::<f32>(this, 8)).fsin().fdouble().fadd_m(rd::<f32>(pos, 0)));
    let py = f32_of(x(rd::<f32>(this, 8)).fcos().fdouble().fadd_m(rd::<f32>(pos, 4)));
    let pz = f32_of(x(rd::<f32>(pos, 8)).fadd_m(g::<f32>(0x86FD40).read()));
    let mut posn = [px.to_bits(), py.to_bits(), pz.to_bits()];
    ik_look_at(ik(), 0x86FD98, ped, 0, 0x1388, 0xFFFF_FFFF, buf(&mut posn), 0, 0x3E80_0000, 0x1F4, 3, 0);
    wr::<u8>(this, 0x14, rd::<u8>(this, 0x14) | 1);
}

/// `CTaskSimpleAchieveHeading::ProcessPed(CPed*)` (0x668060, `ret 4`): done at once for ped +0x46C bit 0x100.
/// Otherwise `RestoreHeadingRate`, heading rate +0x560 *= +0xC, `SetMoveState(1)`, the ped's `SetMoveAnim`, wanted
/// heading +0x55C = +8; when |LimitRadianAngle(|h - current|)| < tolerance +0x10: +0x55C = current, restore the
/// rate, quit the look-at, true; else `SetUpIK`, false.
#[hook(addr = 0x668060, conv = thiscall, name = "CTaskSimpleAchieveHeading::ProcessPed", expect = "56 8B 74 24 08")]
pub fn achieve_heading_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        return 1;
    }
    ped_restore_heading_rate(ped);
    wr::<f32>(ped, 0x560, f32_of(x(rd::<f32>(this, 0xC)).fmul_m(rd::<f32>(ped, 0x560))));
    ped_set_move_state(ped, 1);
    v_ped_set_move_anim(ped);
    wr::<u32>(ped, 0x55C, rd::<u32>(this, 8));
    let d = f32_of(fabs(x(rd::<f32>(this, 8)).fsub_m(rd::<f32>(ped, 0x558))));
    let a = fabs(X87::from_f64_exact(general_limit_radian_angle(d)));
    if a.fcom(x(rd::<f32>(this, 0x10))).test_parity(5) {
        achieve_heading_set_up_ik(this, ped);
        return 0;
    }
    wr::<u32>(ped, 0x55C, rd::<u32>(ped, 0x558));
    ped_restore_heading_rate(ped);
    if rd::<u8>(this, 0x14) & 1 != 0 && ik_is_looking(ik(), ped) != 0 {
        ik_abort_look_at(ik(), ped, 0xFA);
    }
    1
}

// ---- CTaskComplexGoToPointAndStandStill -----------------------------------------------------------------

/// `CTaskComplexGoToPointAndStandStill::GoToPoint(const CVector& target, float radius, float moveStateRadius, bool
/// force)` (0x46FE60, `ret 0x10`): unless forced, keeps the task when target, radius and move-state radius are
/// unchanged; the radius test compares +0x1C with itself (only a NaN differs), as the bytes do. Otherwise copies
/// them and sets +0x24 bit 2 (new target).
#[hook(addr = 0x46FE60, conv = thiscall, name = "CTaskComplexGoToPointAndStandStill::GoToPoint", expect = "8A 44 24 10 84 C0")]
pub fn gtpss_go_to_point(this: GPtr<u8>, target: GPtr<u8>, radius: u32, ms_radius: u32, force: u8) {
    if force == 0 {
        let ne = |a: X87, b: X87| a.fcom(b).test_parity(0x44);
        let same = !ne(x(rd::<f32>(this, 0x10)), x(rd::<f32>(target, 0)))
            && !ne(x(rd::<f32>(this, 0x14)), x(rd::<f32>(target, 4)))
            && !ne(x(rd::<f32>(this, 0x18)), x(rd::<f32>(target, 8)))
            && !ne(x(rd::<f32>(this, 0x1C)), x(rd::<f32>(this, 0x1C)))
            && !ne(x(rd::<f32>(this, 0x20)), x(f32::from_bits(ms_radius)));
        if same {
            return;
        }
    }
    wr::<u32>(this, 0x10, rd::<u32>(target, 0));
    wr::<u32>(this, 0x14, rd::<u32>(target, 4));
    wr::<u32>(this, 0x18, rd::<u32>(target, 8));
    wr::<u32>(this, 0x1C, radius);
    let b = rd::<u8>(this, 0x24);
    wr::<u32>(this, 0x20, ms_radius);
    wr::<u8>(this, 0x24, b | 4);
}

/// `CTaskComplexGoToPointAndStandStill::CTaskComplexGoToPointAndStandStill(int32 moveState, const CVector& target,
/// float radius, float moveStateRadius, bool mustOverShoot, bool stopExactly)` (0x668120, `ret 0x18`): `CTaskComplex`
/// base, vtable 0x86FDA8, +0xC = move state, +0x24 = (+0x24 & 0xE4) | bools; the target is compared with the
/// uninitialised pool contents (radius against itself) before it is copied (+0x24 |= 4); with bit 1 (stop
/// exactly) bit 0 is cleared and the radius raised to [0x858624] (move state > 4) or [0x858B8C] (fld/fstp), the
/// move-state radius set to 0.
#[hook(addr = 0x668120, conv = thiscall, name = "CTaskComplexGoToPointAndStandStill::Constructor", expect = "53 55 56 57 8B F1")]
pub fn gtpss_ctor(this: GPtr<u8>, state: i32, target: GPtr<u8>, radius: u32, ms_radius: u32, over: u8, exact: u8) -> GPtr<u8> {
    task_complex_ctor(this);
    let b = rd::<u8>(this, 0x24);
    let mut bl = ((exact & 1) << 1) | (b & 0xE4) | (over & 1);
    wr::<u32>(this, 0, 0x86FDA8);
    wr::<i32>(this, 0xC, state);
    wr::<u8>(this, 0x24, bl);
    let update = vector_ne(this.byte_add(0x10), target) != 0
        || x(rd::<f32>(this, 0x1C)).fcom(x(rd::<f32>(this, 0x1C))).test_parity(0x44)
        || x(rd::<f32>(this, 0x20)).fcom(x(f32::from_bits(ms_radius))).test_parity(0x44);
    if update {
        wr::<u32>(this, 0x10, rd::<u32>(target, 0));
        wr::<u32>(this, 0x14, rd::<u32>(target, 4));
        bl |= 4;
        wr::<u32>(this, 0x18, rd::<u32>(target, 8));
        wr::<u32>(this, 0x1C, radius);
        wr::<u32>(this, 0x20, ms_radius);
        wr::<u8>(this, 0x24, bl);
    }
    let al = rd::<u8>(this, 0x24);
    if al & 2 != 0 {
        wr::<u8>(this, 0x24, al & 0xFE);
        let c = if state > 4 { 0x858624 } else { 0x858B8C };
        if !gf(c).fcom(x(rd::<f32>(this, 0x1C))).test_any(0x41) {
            wr::<f32>(this, 0x1C, f32_of(gf(c)));
            wr::<u32>(this, 0x20, 0);
        } else {
            let r = rd::<f32>(this, 0x1C);
            wr::<u32>(this, 0x20, 0);
            wr::<f32>(this, 0x1C, f32_of(x(r)));
        }
    }
    this
}

/// `CTaskComplexGoToPointAndStandStill::CreateSubTask(int32 type, CPed*)` (0x6682D0, `ret 8`; returns the task in
/// EAX and EDX). 0xCA: `CTaskSimplePause(1)`; 0xCB: stand still — after an arrival with stop-exactly (+0x24 bit 1)
/// from a go-to-point sub-task (0x384) a `CTaskSimpleStandStill(2000, false, true, delta)` whose blend delta is 8
/// for a slow ped (+0x4DC < [0x858C58]), 16 when the target is not ahead (dot <= [0x858C58]) or the computed
/// [0x858B40] / ((dot / (speed·[0x858B8C]) - [0x858624])·step) exceeds [0x8599D0]; else `CTaskSimpleStandStill(1,
/// false, false, 8)`; 0x2C0: `CTaskComplexLeaveCar(ped +0x58C, 0, 0, true, false)`; 0x384: `CTaskSimpleGoToPoint(
/// state, target, radius, false, +0x24 bit 0)`, +0x20 |= 0x10 for bit 3 (also through a null task, as the
/// original), and `SelectMoveState` for states 6 (1e8) / 7 (10); anything else null. The matrix forward vector is
/// used without a null check; the dot and the delta live in the original's ped argument slot (Rust locals).
#[hook(addr = 0x6682D0, conv = thiscall, name = "CTaskComplexGoToPointAndStandStill::CreateSubTask", expect = "64 A1 00 00 00 00")]
pub fn gtpss_create_sub_task_port(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    match ty {
        0xCA => {
            let t = task_new(0x18);
            if t.is_null() { GPtr::NULL } else { pause_ctor(t, 1) }
        }
        0xCB => {
            let sub = rd::<GPtr<u8>>(this, 8);
            if rd::<u8>(this, 0x24) & 2 != 0 && !sub.is_null() && v_get_task_type(sub) == 0x384 {
                let m = rd::<GPtr<u8>>(ped, 0x14);
                let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
                let fwd = m.byte_add(0x10);
                let mut d = [0u32; 3];
                let dp = vector_sub(buf(&mut d), this.byte_add(0x10), pos);
                let dot = f32_of(X87::from_f64_exact(vector_dot_product(dp, fwd)));
                let delta: f32 = if !x(rd::<f32>(ped, 0x4DC)).fcom(gf(0x858C58)).test_parity(5) {
                    8.0
                } else if x(dot).fcom(gf(0x858C58)).test_any(0x41) {
                    16.0
                } else {
                    let t = x(rd::<f32>(ped, 0x4DC))
                        .fmul_m(g::<f32>(0x858B8C).read())
                        .fdivr_m(dot)
                        .fsub_m(g::<f32>(0x858624).read())
                        .fmul_m(g::<f32>(TIME_STEP).read())
                        .fdivr_m(g::<f32>(0x858B40).read());
                    if t.fcom(gf(0x8599D0)).test_any(0x41) { f32_of(t) } else { 16.0 }
                };
                let t = task_new(0x20);
                if t.is_null() {
                    return GPtr::NULL;
                }
                stand_still_ctor(t, 0x7D0, 0, 1, delta.to_bits())
            } else {
                let t = task_new(0x20);
                if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, 1, 0, 0, 0x4100_0000) }
            }
        }
        0x2C0 => {
            let t = task_new(0x34);
            if t.is_null() { GPtr::NULL } else { leave_car_ctor(t, rd::<u32>(ped, 0x58C), 0, 0, 1, 0) }
        }
        0x384 => {
            let t = task_new(0x30);
            let t = if t.is_null() {
                GPtr::NULL
            } else {
                let r = rd::<u32>(this, 0x1C);
                let b = (rd::<u8>(this, 0x24) & 1) as u32;
                goto_point_ctor_call(t, rd::<u32>(this, 0xC), this.byte_add(0x10), r, 0, b)
            };
            if rd::<u8>(this, 0x24) & 8 != 0 {
                wr::<u8>(t, 0x20, rd::<u8>(t, 0x20) | 0x10);
            }
            match rd::<u32>(this, 0xC) {
                6 => gtpss_select_move_state(this, t, ped, rd::<u32>(this, 0x20), 0x4CBE_BC20),
                7 => gtpss_select_move_state(this, t, ped, rd::<u32>(this, 0x20), 0x4120_0000),
                _ => {}
            }
            t
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexGoToPointAndStandStill::ControlSubTask(CPed*)` (0x668570, `ret 4`): a new target (+0x24 bit 2)
/// asks the sub-task to abort (`MakeAbortable(ped, 1, null)`) and restarts with `CreateFirstSubTask`; a running
/// go-to-point (0x384) in state 6/7 gets `SelectMoveState`; else the current sub-task.
#[hook(addr = 0x668570, conv = thiscall, name = "CTaskComplexGoToPointAndStandStill::ControlSubTask", expect = "56 8B F1 8A 46 24")]
pub fn gtpss_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    if rd::<u8>(this, 0x24) & 4 != 0 {
        if v_make_abortable(sub, ped, 1, 0) != 0 {
            return v_create_first_sub_task(this, ped);
        }
        return rd::<GPtr<u8>>(this, 8);
    }
    if v_get_task_type(sub) == 0x384 {
        let st = rd::<u32>(this, 0xC);
        let s = rd::<GPtr<u8>>(this, 8);
        match st {
            6 => gtpss_select_move_state(this, s, ped, rd::<u32>(this, 0x20), 0x4CBE_BC20),
            7 => gtpss_select_move_state(this, s, ped, rd::<u32>(this, 0x20), 0x4120_0000),
            _ => {}
        }
    }
    rd::<GPtr<u8>>(this, 8)
}

/// `CTaskComplexGoToPointAndStandStill::CreateNextSubTask(CPed*)` (0x66DBA0, `ret 4`): after a stand still (0xCB)
/// → `CreateSubTask(0x516)` (finished: null); after leaving a car (0x2C0) → 0x516 when ped +0x46C bit 0x100, else
/// go to the point (0x384); after the go-to-point → +0x24 bit 4 = its +0x20 bit 2 (arrived), then stand still
/// (0xCB); anything else null.
#[hook(addr = 0x66DBA0, conv = thiscall, name = "CTaskComplexGoToPointAndStandStill::CreateNextSubTask", expect = "56 8B F1 8B 4E 08")]
pub fn gtpss_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    match v_get_task_type(rd::<GPtr<u8>>(this, 8)) {
        0xCB => gtpss_create_sub_task(this, 0x516, ped),
        0x2C0 => {
            if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
                gtpss_create_sub_task(this, 0x516, ped)
            } else {
                gtpss_create_sub_task(this, 0x384, ped)
            }
        }
        0x384 => {
            let dl = rd::<u8>(rd::<GPtr<u8>>(this, 8), 0x20);
            let al = rd::<u8>(this, 0x24);
            wr::<u8>(this, 0x24, al ^ (((dl << 2) ^ al) & 0x10));
            gtpss_create_sub_task(this, 0xCB, ped)
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexGoToPointAndStandStill::CreateFirstSubTask(CPed*)` (0x66DC40, `ret 4`): clears the new-target bit
/// (+0x24 bit 2); leave the car first (0x2C0) for a ped in a vehicle (+0x46C bit 0x100), else go to the point.
#[hook(addr = 0x66DC40, conv = thiscall, name = "CTaskComplexGoToPointAndStandStill::CreateFirstSubTask", expect = "80 61 24 FB 8B 44 24 04")]
pub fn gtpss_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    wr::<u8>(this, 0x24, rd::<u8>(this, 0x24) & 0xFB);
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        gtpss_create_sub_task(this, 0x2C0, ped)
    } else {
        gtpss_create_sub_task(this, 0x384, ped)
    }
}

// ---- CTaskComplexTurnToFaceEntityOrCoord ----------------------------------------------------------------

/// `CTaskComplexTurnToFaceEntityOrCoord::Constructor-Coords` (0x66B890, `ret 0xC`; despite the catalogue name the
/// entity constructor: `(CEntity*, float changeRate, float tolerance)`; the SEH handler push continues through
/// the island 0x40823F): `CTaskComplex` base, vtable 0x86FF94, +0xC = entity (reference registered), +0x10 = 1,
/// +0x20 = rate, +0x24 = tolerance.
#[hook(addr = 0x66B890, conv = thiscall, name = "CTaskComplexTurnToFaceEntityOrCoord::Constructor-Coords", expect = "6A FF E9 A8 C9 D9 FF")]
pub fn turn_ctor_entity_port(this: GPtr<u8>, e: GPtr<u8>, rate: u32, tol: u32) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0, 0x86FF94);
    wr::<GPtr<u8>>(this, 0xC, e);
    wr::<u8>(this, 0x10, 1);
    if !e.is_null() {
        entity_register_reference(e, this.byte_add(0xC));
    }
    wr::<u32>(this, 0x24, tol);
    wr::<u32>(this, 0x20, rate);
    this
}

/// `CTaskComplexTurnToFaceEntityOrCoord::ComputeTargetHeading(CPed*)` (0x66B9D0, `ret 4`, ST0): the target (the
/// entity's position when +0x10, else +0x14) minus the ped position, each component stored as f32, normalised;
/// `GetRadianAngleBetweenPoints(d.x, d.y, 0, 0)`.
#[hook(addr = 0x66B9D0, conv = thiscall, name = "CTaskComplexTurnToFaceEntityOrCoord::ComputeTargetHeading", expect = "8A 41 10 83 EC 18")]
pub fn turn_compute_target_heading_port(this: GPtr<u8>, ped: GPtr<u8>) -> X87 {
    let t = if rd::<u8>(this, 0x10) != 0 { ent_pos(rd::<GPtr<u8>>(this, 0xC)) } else { this.byte_add(0x14) };
    let tv = [rd::<f32>(t, 0), rd::<f32>(t, 4), rd::<f32>(t, 8)];
    let p = ent_pos(ped);
    let mut d = [
        f32_of(x(tv[0]).fsub_m(rd::<f32>(p, 0))).to_bits(),
        f32_of(x(tv[1]).fsub_m(rd::<f32>(p, 4))).to_bits(),
        f32_of(x(tv[2]).fsub_m(rd::<f32>(p, 8))).to_bits(),
    ];
    vector_normalise(buf(&mut d));
    X87::from_f64_exact(general_get_radian_angle_between_points(f32::from_bits(d[0]), f32::from_bits(d[1]), 0.0, 0.0))
}

/// `CTaskComplexTurnToFaceEntityOrCoord::Clone()` (0x66D250): a pool copy through the entity constructor (+0x10)
/// or the coords constructor (0x66B910, `&+0x14`), with +0x20/+0x24; null when the pool is full.
#[hook(addr = 0x66D250, conv = thiscall, name = "CTaskComplexTurnToFaceEntityOrCoord::Clone", expect = "6A FF 68 96 54 84 00")]
pub fn turn_clone(this: GPtr<u8>) -> GPtr<u8> {
    let ent = rd::<u8>(this, 0x10) != 0;
    let t = task_new(0x28);
    if t.is_null() {
        return GPtr::NULL;
    }
    if ent {
        turn_ctor_entity(t, rd::<u32>(this, 0xC), rd::<u32>(this, 0x20), rd::<u32>(this, 0x24))
    } else {
        turn_ctor_coords(t, this.byte_add(0x14), rd::<u32>(this, 0x20), rd::<u32>(this, 0x24))
    }
}

/// `CTaskComplexTurnToFaceEntityOrCoord::CreateFirstSubTask(CPed*)` (0x670850, `ret 4`): null for an entity task
/// whose entity is gone; else the inlined `CTaskSimpleAchieveHeading(f32(ComputeTargetHeading), rate +0x20,
/// tolerance +0x24)` (pool slot 0x18: +0x14 bit 0 cleared, vtable 0x86FD74).
#[hook(addr = 0x670850, conv = thiscall, name = "CTaskComplexTurnToFaceEntityOrCoord::CreateFirstSubTask", expect = "64 A1 00 00 00 00")]
pub fn turn_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u8>(this, 0x10) != 0 && rd::<u32>(this, 0xC) == 0 {
        return GPtr::NULL;
    }
    let h = f32_of(X87::from_f64_exact(turn_compute_target_heading(this, ped)));
    let t = task_new(0x18);
    if t.is_null() {
        return GPtr::NULL;
    }
    let tol = rd::<u32>(this, 0x24);
    let rate = rd::<u32>(this, 0x20);
    task_simple_ctor(t);
    wr::<u8>(t, 0x14, rd::<u8>(t, 0x14) & 0xFE);
    wr::<f32>(t, 8, h);
    wr::<u32>(t, 0, 0x86FD74);
    wr::<u32>(t, 0xC, rate);
    wr::<u32>(t, 0x10, tol);
    t
}

/// `CTaskComplexTurnToFaceEntityOrCoord::ControlSubTask(CPed*)` (0x670920, `ret 4`): for an entity task, the heading
/// to the entity (or the ped's current heading +0x558 when the entity is gone) and the rate/tolerance are written
/// into the achieve-heading sub-task when any of them changed (exact compares); the current sub-task either way.
#[hook(addr = 0x670920, conv = thiscall, name = "CTaskComplexTurnToFaceEntityOrCoord::ControlSubTask", expect = "51 56 8B F1 8A 46 10")]
pub fn turn_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u8>(this, 0x10) == 0 {
        return rd::<GPtr<u8>>(this, 8);
    }
    let h = if rd::<u32>(this, 0xC) != 0 {
        X87::from_f64_exact(turn_compute_target_heading(this, ped))
    } else {
        x(rd::<f32>(ped, 0x558))
    };
    let tol = rd::<u32>(this, 0x24);
    let rate = rd::<u32>(this, 0x20);
    let s = rd::<GPtr<u8>>(this, 8);
    let ne = |a: X87, b: f32| a.fcom(x(b)).test_parity(0x44);
    if ne(h, rd::<f32>(s, 8))
        || ne(x(rd::<f32>(s, 0xC)), f32::from_bits(rate))
        || ne(x(rd::<f32>(s, 0x10)), f32::from_bits(tol))
    {
        wr::<f32>(s, 8, f32_of(h));
        wr::<u32>(s, 0xC, rate);
        wr::<u32>(s, 0x10, tol);
    }
    rd::<GPtr<u8>>(this, 8)
}

// ---- CTaskComplexGoToCarDoorAndStandStill ---------------------------------------------------------------

/// `CTaskComplexGoToCarDoorAndStandStill::CreateSubTask(int32 type, CPed*)` (0x64A5F0, `ret 8`; the entry is
/// `nop; jmp` into the island 0x40A1DB, which loads the SEH head and jumps back). 0xCA: `CTaskSimplePause(1)`;
/// 0xCB: `CTaskSimpleStandStill(1, false, false, 8)`; 0x32A: `CTaskSimpleCarWaitForDoorNotToBeInUse(vehicle +0xC,
/// door +0x34, 0)`; 0x33C: ped +0x474 |= 8, `CTaskSimpleCarGoToPointNearDoorUntilDoorNotInUse(+0xC, +0x34, &+0x38,
/// state +0x10)`; 0x384: ped +0x474 |= 8, `CTaskSimpleGoToPoint(+0x10, &+0x38, radius +0x18, true, false)`; 0x389:
/// `CTaskComplexFollowPointRoute(+0x10, route +0x4C, 0, [0x86FC98], [0x86FC9C], 0, 0, 0)`; anything else null.
/// Not hookable: the protection reads the patched entry bytes (`adc ebx, [0x64A5F2]` at 0x15630B8; sa-hook INF-20,
/// keycells and patch_refs tests).
#[sa_core::native(addr = 0x64A5F0, conv = thiscall, name = "CTaskComplexGoToCarDoorAndStandStill::CreateSubTask")]
pub fn car_door_create_sub_task_port(this: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> {
    let alloc = |n: u32| task_new(n);
    match ty {
        0xCA => {
            let t = alloc(0x18);
            if t.is_null() { GPtr::NULL } else { pause_ctor(t, 1) }
        }
        0xCB => {
            let t = alloc(0x20);
            if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, 1, 0, 0, 0x4100_0000) }
        }
        0x32A => {
            let t = alloc(0x14);
            if t.is_null() { GPtr::NULL } else { car_wait_for_door_ctor(t, rd::<u32>(this, 0xC), rd::<u32>(this, 0x34), 0) }
        }
        0x33C => {
            wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 8);
            let t = alloc(0x10);
            if t.is_null() {
                return GPtr::NULL;
            }
            let ms = rd::<u32>(this, 0x10);
            car_go_near_door_ctor(t, rd::<u32>(this, 0xC), rd::<u32>(this, 0x34), this.byte_add(0x38), ms)
        }
        0x384 => {
            wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 8);
            let t = alloc(0x30);
            if t.is_null() {
                return GPtr::NULL;
            }
            let r = rd::<u32>(this, 0x18);
            goto_point_ctor_call(t, rd::<u32>(this, 0x10), this.byte_add(0x38), r, 1, 0)
        }
        0x389 => {
            let t = alloc(0x3C);
            if t.is_null() {
                return GPtr::NULL;
            }
            let (k2, k1) = (g::<u32>(0x86FC9C).read(), g::<u32>(0x86FC98).read());
            let route = rd::<u32>(this, 0x4C);
            follow_point_route_ctor(t, rd::<u32>(this, 0x10), route, 0, k1, k2, 0, 0, 0)
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexGoToCarDoorAndStandStill::CreateNextSubTask(CPed*)` (0x64D2B0, `ret 4`): after waiting for the
/// door (0x32A) restart (`CreateFirstSubTask`); after standing still (0xCB) finish (0x516 = null); after a pause
/// (0xCA) finish, marking arrival (+0x49) when +0x48 and the 2D distance to the target (+0x38) < [0x858B8C];
/// after going near the door (0x33C) wait for it (0x32A); after the go-to-point (0x384) finish, marking arrival
/// when it arrived (+0x20 bit 2) within [0x858B3C] in z; after the point route (0x389) the target becomes
/// `GetPositionToOpenCarDoor(vehicle, door)` and a go-to-point follows; anything else null.
#[hook(addr = 0x64D2B0, conv = thiscall, name = "CTaskComplexGoToCarDoorAndStandStill::CreateNextSubTask", expect = "83 EC 18 56 8B F1")]
pub fn car_door_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    match v_get_task_type(rd::<GPtr<u8>>(this, 8)) {
        0x32A => v_create_first_sub_task(this, ped),
        0xCB => car_door_create_sub_task(this, 0x516, ped),
        0xCA => {
            if rd::<u8>(this, 0x48) != 0 {
                let mut d = [0u32; 3];
                let dp = vector_sub(buf(&mut d), this.byte_add(0x38), ent_pos(ped));
                let (dy, dx) = (x(rd::<f32>(dp, 4)), x(rd::<f32>(dp, 0)));
                let dist = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt();
                if !dist.fcom(gf(0x858B8C)).test_parity(5) {
                    wr::<u8>(this, 0x49, 1);
                }
            }
            car_door_create_sub_task(this, 0x516, ped)
        }
        0x33C => car_door_create_sub_task(this, 0x32A, ped),
        0x384 => {
            let pos = ent_pos(ped);
            let s = rd::<GPtr<u8>>(this, 8);
            if rd::<u8>(s, 0x20) & 4 != 0
                && !fabs(x(rd::<f32>(s, 0x14)).fsub_m(rd::<f32>(pos, 8))).fcom(gf(0x858B3C)).test_parity(5)
            {
                wr::<u8>(this, 0x49, 1);
            }
            car_door_create_sub_task(this, 0x516, ped)
        }
        0x389 => {
            let mut o = [0u32; 3];
            let r = get_position_to_open_car_door(buf(&mut o), rd::<u32>(this, 0xC), rd::<u32>(this, 0x34));
            wr::<u32>(this, 0x38, rd::<u32>(r, 0));
            wr::<u32>(this, 0x3C, rd::<u32>(r, 4));
            wr::<u32>(this, 0x40, rd::<u32>(r, 8));
            car_door_create_sub_task(this, 0x384, ped)
        }
        _ => GPtr::NULL,
    }
}

/// Start timer of the door search (+0x28 = now, +0x2C = +0x24, started).
fn car_door_start_timer(this: GPtr<u8>) {
    let t = rd::<u32>(this, 0x24);
    wr::<u32>(this, 0x28, g::<u32>(TIME_MS).read());
    wr::<u32>(this, 0x2C, t);
    wr::<u8>(this, 0x30, 1);
}

/// The target (+0x38) = `GetPositionToOpenCarDoor(vehicle, door)`.
fn car_door_set_target(this: GPtr<u8>, veh: u32, door: u32) {
    let mut o = [0u32; 3];
    let r = get_position_to_open_car_door(buf(&mut o), veh, door);
    wr::<u32>(this, 0x38, rd::<u32>(r, 0));
    wr::<u32>(this, 0x3C, rd::<u32>(r, 4));
    wr::<u32>(this, 0x40, rd::<u32>(r, 8));
}

/// A blocked path to the target door computes a route (0x389), else `open` follows.
fn car_door_route_or(this: GPtr<u8>, ped: GPtr<u8>, open: u32) -> GPtr<u8> {
    if path_to_door_blocked(ped, rd::<u32>(this, 0xC), this.byte_add(0x38)) != 0 {
        car_door_compute_route_to_door(this, ped);
        car_door_create_sub_task(this, 0x389, ped)
    } else {
        car_door_create_sub_task(this, open, ped)
    }
}

/// `CTaskComplexGoToCarDoorAndStandStill::CreateFirstSubTask(CPed*)` (0x64D440, `ret 4`): out of range → stand still
/// (0xCB); +0x48 → pause (0xCA). A driver (+0x14) goes to the given door (+0x44, target from
/// `GetPositionToOpenCarDoor`) or the nearest door (`GetNearestCarDoor`, none → 0xCB; starts the timer). A
/// passenger: the nearest passenger door (flags 1,1,1: timer, go to the point) or else (1,0,1: wait near the door,
/// 0x33C; none → 0xCB), or the given door unless its occupant has +0x46C bit 0x20000000 (→ 0xCB; else target,
/// timer). A path blocked by the vehicle's collision model computes a route (0x389) instead of going straight
/// (0x384). `IsVehicleInRange` keeps ECX (`preserve`), so the 0xCB call uses this task.
#[hook(addr = 0x64D440, conv = thiscall, name = "CTaskComplexGoToCarDoorAndStandStill::CreateFirstSubTask", expect = "83 EC 0C 55 56 57")]
pub fn car_door_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if car_door_is_vehicle_in_range(this, ped) == 0 {
        return car_door_create_sub_task(this, 0xCB, ped);
    }
    if rd::<u8>(this, 0x48) != 0 {
        return car_door_create_sub_task(this, 0xCA, ped);
    }
    let door = rd::<u32>(this, 0x44);
    if rd::<u8>(this, 0x14) != 0 {
        if door != 0 {
            wr::<u32>(this, 0x34, door);
            car_door_set_target(this, rd::<u32>(this, 0xC), door);
        } else {
            if get_nearest_car_door(ped, rd::<u32>(this, 0xC), this.byte_add(0x38), this.byte_add(0x34)) == 0 {
                return car_door_create_sub_task(this, 0xCB, ped);
            }
            car_door_start_timer(this);
        }
        return car_door_route_or(this, ped, 0x384);
    }
    if door == 0 {
        let (pos, dp) = (this.byte_add(0x38), this.byte_add(0x34));
        if get_nearest_car_passenger_door(ped, rd::<u32>(this, 0xC), pos, dp, 1, 1, 1) != 0 {
            car_door_start_timer(this);
            return car_door_route_or(this, ped, 0x384);
        }
        if get_nearest_car_passenger_door(ped, rd::<u32>(this, 0xC), pos, dp, 1, 0, 1) == 0 {
            return car_door_create_sub_task(this, 0xCB, ped);
        }
        return car_door_route_or(this, ped, 0x33C);
    }
    let idx = compute_passenger_index_from_car_door(rd::<u32>(this, 0xC), door);
    let veh = rd::<GPtr<u8>>(this, 0xC);
    let occ = rd::<GPtr<u8>>(veh, (idx.wrapping_mul(4) + 0x464) as isize);
    if !occ.is_null() && rd::<u32>(occ, 0x46C) & 0x2000_0000 != 0 {
        return car_door_create_sub_task(this, 0xCB, ped);
    }
    let d = rd::<u32>(this, 0x44);
    wr::<u32>(this, 0x34, d);
    car_door_set_target(this, veh.addr() as u32, d);
    car_door_start_timer(this);
    car_door_route_or(this, ped, 0x384)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_STAND_STILL_CTOR_PORT,
    HOOK_STAND_STILL_PROCESS_PED_PORT,
    HOOK_GOTO_HAS_CIRCLED_TARGET_PORT,
    HOOK_GOTO_SET_UP_IK_PORT,
    HOOK_GOTO_QUIT_IK_PORT,
    HOOK_GOTO_POINT_UPDATE_POINT,
    HOOK_GOTO_POINT_CTOR,
    HOOK_GOTO_POINT_MAKE_ABORTABLE,
    HOOK_GOTO_POINT_PROCESS_PED,
    HOOK_ACHIEVE_HEADING_MAKE_ABORTABLE,
    HOOK_ACHIEVE_HEADING_SET_UP_IK_PORT,
    HOOK_ACHIEVE_HEADING_PROCESS_PED,
    HOOK_GTPSS_GO_TO_POINT,
    HOOK_GTPSS_CTOR,
    HOOK_GTPSS_CREATE_SUB_TASK_PORT,
    HOOK_GTPSS_CONTROL_SUB_TASK,
    HOOK_GTPSS_CREATE_NEXT_SUB_TASK,
    HOOK_GTPSS_CREATE_FIRST_SUB_TASK,
    HOOK_TURN_CTOR_ENTITY_PORT,
    HOOK_TURN_COMPUTE_TARGET_HEADING_PORT,
    HOOK_TURN_CLONE,
    HOOK_TURN_CREATE_FIRST_SUB_TASK,
    HOOK_TURN_CONTROL_SUB_TASK,
    HOOK_CAR_DOOR_CREATE_NEXT_SUB_TASK,
    HOOK_CAR_DOOR_CREATE_FIRST_SUB_TASK,
];
