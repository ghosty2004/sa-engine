//! PORT-AI-4 (helper B): `CTaskComplexWander` / `CTaskComplexWanderStandard` / `CTaskComplexWanderCop`,
//! `CTaskComplexAvoidOtherPedWhileWandering`, `CTaskComplexFollowNodeRoute` and `CTaskComplexFollowPointRoute`.
//! Ported from the 1.0 US bytes (the EXE wins over gta-reversed and the specs). Every call to another
//! catalogued function goes through `orig!` at the original address; task objects keep their original
//! vtables and come from the original task pool (`common::task_new`).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_math::x87::Cmp;

use crate::common::*;

orig! {
    // ---- virtual slots ----
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskComplex::CreateFirstSubTask(CPed*)` (vtable +0x24)
    fn v_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;

    // ---- CTaskComplexFollowPointRoute ----
    /// `CTaskComplexFollowPointRoute::GetSubTaskType()` (0x668FE0)
    fn fpr_get_sub_task_type_orig(t: GPtr<u8>) -> u32 = thiscall 0x668FE0 => unported;
    /// `CTaskComplexFollowPointRoute::CreateTask(eTaskType, CPed*)` (0x669090, `ret 8`)
    fn fpr_create_task_orig(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x669090 => unported;
    /// `CTaskComplexFollowPointRoute::CalcGoToTaskType(CPed*, eTaskType)` (0x66E150, `ret 8`)
    fn fpr_calc_go_to_task_type(t: GPtr<u8>, ped: GPtr<u8>, ty: u32) -> u32 = thiscall 0x66E150 => unported;
    /// `CTaskComplexFollowPointRoute::CalcBlendRatio(CPed*, bool)` (0x66E710, `ret 8`, float in ST0)
    fn fpr_calc_blend_ratio(t: GPtr<u8>, ped: GPtr<u8>, slowing: u32) -> f64 = thiscall 0x66E710 => unported;
    /// 0x667720 (`CPointRoute` reversal, `this` = the route, `ret`)
    fn point_route_667720(r: GPtr<u8>) = thiscall 0x667720 => unported;
    /// `CPointRoute::operator new(uint32)` (0x41B7C0, cdecl)
    fn point_route_new(size: u32) -> GPtr<u8> = cdecl 0x41B7C0 => unported;
    /// 0x63A040 (`CPointRoute` copy from another route, `ret 4`)
    fn point_route_copy_63a040(r: GPtr<u8>, src: GPtr<u8>) = thiscall 0x63A040 => unported;

    // ---- other tasks ----
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CTaskComplexLeaveCar` constructor (0x63B8C0, `ret 0x14`)
    fn leave_car_ctor(t: GPtr<u8>, veh: u32, a1: u32, a2: u32, a3: u32, a4: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    /// `CTaskSimpleStandStill::Constructor(time, looped, useAnimIdleStance, blend)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, time: u32, looped: u32, idle: u32, blend: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskSimpleLeaveGroup::Constructor` (0x463540)
    fn leave_group_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x463540 => unported;
    /// `CTaskSimpleGoToPoint::Constructor(moveState, const CVector*, radius, bool, bool)` (0x667CD0, `ret 0x14`)
    fn go_to_point_ctor(t: GPtr<u8>, ms: u32, pt: GPtr<u8>, radius: u32, a3: u32, a4: u32) -> GPtr<u8> = thiscall 0x667CD0 => unported;
    /// `CTaskSimpleGoToPointFine::Constructor(ratio, CVector by value, radius, entity)` (0x65EEB0, `ret 0x18`)
    fn go_to_point_fine_ctor(t: GPtr<u8>, ratio: u32, vx: u32, vy: u32, vz: u32, radius: u32, a5: u32) -> GPtr<u8> = thiscall 0x65EEB0 => unported;
    /// `CTaskSimpleGoToPointFine::BaseRatio(eMoveState)` (0x65EF30, cdecl, float in ST0)
    fn go_to_point_fine_base_ratio(ms: u32) -> f64 = cdecl 0x65EF30 => unported;
    /// `CTaskSimpleGoToPointFine::SetMoveRatio(float)` (0x65F3A0, `ret 4`)
    fn go_to_point_fine_set_move_ratio(t: GPtr<u8>, ratio: u32) = thiscall 0x65F3A0 => unported;
    /// `CTaskComplexGoToPointAndStandStill::Constructor(ms, const CVector*, radius, slowDown, bool, bool)`
    /// (0x668120, `ret 0x18`)
    fn go_to_point_and_stand_still_ctor(t: GPtr<u8>, ms: u32, pt: GPtr<u8>, radius: u32, slow: u32, a4: u32, a5: u32) -> GPtr<u8> = thiscall 0x668120 => unported;
    /// `CTaskComplexWalkRoundCar::GoingForDoor()` (0x654720)
    fn walk_round_car_going_for_door(t: GPtr<u8>) -> u8 = thiscall 0x654720 => unported;

    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CEvent::GetEventType` (CEvent vtable +4)
    fn v_event_get_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CEntity::Teleport(CVector by value, bool)` (vtable +0x38)
    fn v_entity_teleport(e: GPtr<u8>, vx: u32, vy: u32, vz: u32, clear: u32) = thiscall vslot 0x38 => unported;
    /// `CPedGeometryAnalyser::IsEntityBlockingTarget(CEntity*, const CVector&, float)` (0x5F3970, cdecl)
    fn is_entity_blocking_target(e: GPtr<u8>, target: GPtr<u8>, radius: u32) -> u8 = cdecl 0x5F3970 => unported;

    /// `CTaskComplexWander::UpdatePathNodes(CPed*, int8 dir, CNodeAddress*, CNodeAddress*, int8*)` (vtable +0x38)
    fn v_wander_update_path_nodes(t: GPtr<u8>, ped: GPtr<u8>, dir: u32, last: GPtr<u8>, cur: GPtr<u8>, out_dir: GPtr<u8>) = thiscall vslot 0x38 => unported;
    /// `CPathFind::FindNextNodeWandering(type, CVector by value, CNodeAddress*, CNodeAddress*, dir, int8*)`
    /// (0x451B70, `ret 0x20`)
    fn path_find_next_node_wandering(pf: GPtr<u8>, ty: u32, x: u32, y: u32, z: u32, last: GPtr<u8>, cur: GPtr<u8>, dir: u32, out_dir: GPtr<u8>) = thiscall 0x451B70 => unported;
    /// `CPathFind::TakeWidthIntoAccountForWandering(CVector* out, CNodeAddress, uint16 seed)` (0x4509A0, `ret 0xC`)
    fn path_take_width_into_account_for_wandering(pf: GPtr<u8>, out: GPtr<u8>, node: u32, seed: u32) -> GPtr<u8> = thiscall 0x4509A0 => unported;
    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0, `ret 4`)
    fn ik_is_looking(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::AbortLookAt(CPed*, int32 blendTime)` (0x618280, `ret 8`)
    fn ik_abort_look_at(m: GPtr<u8>, ped: GPtr<u8>, blend: u32) = thiscall 0x618280 => unported;

    /// `CTaskComplexFollowNodeRoute::CreateSubTask(eTaskType, CPed*)` (0x669690, `ret 8`)
    fn fnr_create_sub_task_orig(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x669690 => unported;
    /// `CTaskComplexFollowNodeRoute::GetSubTaskType(int32, bool, const CPointRoute*)` (0x669650, cdecl)
    fn fnr_get_sub_task_type_orig(idx: i32, flag: u32, route: GPtr<u8>) -> u32 = cdecl 0x669650 => unported;
    /// `CTaskComplexFollowNodeRoute::CalcGoToTaskType(CPed*, eTaskType)` (0x66EBE0, `ret 8`)
    fn fnr_calc_go_to_task_type(t: GPtr<u8>, ped: GPtr<u8>, ty: u32) -> u32 = thiscall 0x66EBE0 => unported;
    /// `CTaskComplexFollowNodeRoute::CalcBlendRatio(CPed*, bool)` (0x66EDC0, `ret 8`, float in ST0)
    fn fnr_calc_blend_ratio(t: GPtr<u8>, ped: GPtr<u8>, slowing: u32) -> f64 = thiscall 0x66EDC0 => unported;
    /// `CTaskComplexFollowNodeRoute::ComputePathNodes(const CPed*)` (0x66EFA0, `ret 4`)
    fn fnr_compute_path_nodes_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x66EFA0 => unported;
    /// `CTaskComplexFollowNodeRoute::ComputeRoute()` (0x6699E0)
    fn fnr_compute_route_orig(t: GPtr<u8>) = thiscall 0x6699E0 => unported;
    /// `CTaskComplexFollowNodeRoute::SetTarget(CPed*, const CVector*, float, float, float, bool)` (0x671750, `ret 0x18`)
    fn fnr_set_target_orig(t: GPtr<u8>, ped: GPtr<u8>, target: GPtr<u8>, radius: u32, slow: u32, follow: u32, force: u32) = thiscall 0x671750 => unported;
    /// 0x509760 (cdecl, two `CVector*`; AL = the vectors differ)
    fn vectors_differ_509760(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x509760 => unported;

    /// `CTaskComplexWander::UpdateDir(CPed*)` (vtable +0x34)
    fn v_wander_update_dir(t: GPtr<u8>, ped: GPtr<u8>) = thiscall vslot 0x34 => unported;
    /// `CTaskComplexWander::ScanForStuff(CPed*)` (vtable +0x30)
    fn v_wander_scan_for_stuff(t: GPtr<u8>, ped: GPtr<u8>) = thiscall vslot 0x30 => unported;
    /// `CTaskComplexWander::CreateSubTask(CPed*, eTaskType)` (0x671CB0, `ret 8`)
    fn wander_create_sub_task_orig(t: GPtr<u8>, ped: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x671CB0 => unported;
    /// `CTaskComplexWander::ComputeTargetHeading(CPed*)` (0x66F530, `ret 4`, float in ST0)
    fn wander_compute_target_heading(t: GPtr<u8>, ped: GPtr<u8>) -> f64 = thiscall 0x66F530 => unported;
    /// `CTaskComplexWander::ComputeTargetPos(CPed*, CVector*, const CNodeAddress*)` (0x669F60, `ret 0xC`)
    fn wander_compute_target_pos_orig(t: GPtr<u8>, ped: GPtr<u8>, out: GPtr<u8>, node: GPtr<u8>) = thiscall 0x669F60 => unported;
    /// `CTaskComplexWander::ValidNodes()` (0x669F30)
    fn wander_valid_nodes_orig(t: GPtr<u8>) -> u32 = thiscall 0x669F30 => unported;
    /// `CTaskComplexWander::ScanForBlockedNodes(CPed*)` (0x674560, `ret 4`)
    fn wander_scan_for_blocked_nodes_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x674560 => unported;
    /// `CTaskComplexWander::ScanForBlockedNode-(CPed*, const CNodeAddress*)` (0x671EF0, `ret 8`)
    fn wander_scan_for_blocked_node_orig(t: GPtr<u8>, ped: GPtr<u8>, node: GPtr<u8>) -> u8 = thiscall 0x671EF0 => unported;
    /// `CTaskComplexWander::ScanForBlockedNode-1(const CVector*, CEntity*)` (0x66F4C0, `ret 8`)
    fn wander_scan_for_blocked_node_1_orig(t: GPtr<u8>, pos: GPtr<u8>, e: GPtr<u8>) -> u32 = thiscall 0x66F4C0 => unported;
    /// 0x48DF30 (`.HOODLUM` stub: the `CTaskSimpleScratchHead` (0x1A5) constructor)
    fn scratch_head_ctor_48df30(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x48DF30 => unported;
    /// `CTaskComplexCrossRoadLookAndAchieveHeading` constructor (0x631AE0, `ret 8`)
    fn cross_road_look_ctor(t: GPtr<u8>, time: u32, heading: u32) -> GPtr<u8> = thiscall 0x631AE0 => unported;
    /// `CTaskComplexObserveTrafficLightsAndAchieveHeading` constructor (0x631910, `ret 8`)
    fn observe_traffic_lights_heading_ctor(t: GPtr<u8>, time: u32, heading: u32) -> GPtr<u8> = thiscall 0x631910 => unported;
    /// `CTaskSimpleCarDriveTimed` constructor (0x5FF940, `ret 8`)
    fn car_drive_timed_ctor(t: GPtr<u8>, veh: u32, time: u32) -> GPtr<u8> = thiscall 0x5FF940 => unported;
    /// `CTaskComplexSequence::Constructor` (0x632BD0)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    /// `CTaskComplexSequence::AddTask-0(CTask*)` (0x632D10, `ret 4`)
    fn sequence_add_task(s: GPtr<u8>, t: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CTaskSimpleRunAnim::Constructor(group, anim, blend, bool)` (0x61A8B0, `ret 0x10`)
    fn run_anim_ctor(t: GPtr<u8>, group: u32, anim: u32, blend: u32, a3: u32) -> GPtr<u8> = thiscall 0x61A8B0 => unported;
    /// `CPathFind::TestForPedTrafficLight(CNodeAddress, CNodeAddress)` (0x44D480, `ret 8`)
    fn path_test_for_ped_traffic_light(pf: GPtr<u8>, a: u32, b: u32) -> u8 = thiscall 0x44D480 => unported;
    /// `CPathFind::TestCrossesRoad(CNodeAddress, CNodeAddress)` (0x44D790, `ret 8`)
    fn path_test_crosses_road(pf: GPtr<u8>, a: u32, b: u32) -> u8 = thiscall 0x44D790 => unported;
    /// 0x420920 (`.HOODLUM` stub: `CNodeAddress` equality with the argument, EAX 0/1, `ret 4`)
    fn node_address_eq_420920(a: GPtr<u8>, b: GPtr<u8>) -> u32 = thiscall 0x420920 => unported;
    /// `CTaskSimpleGoToPoint::UpdatePoint(const CVector*, float radius, bool)` (0x645700, `ret 0xC`)
    fn go_to_point_update_point(t: GPtr<u8>, p: GPtr<u8>, radius: u32, a2: u32) = thiscall 0x645700 => unported;
    /// `CPed::Say(uint16 id, uint32, float, uint8, uint8, uint8)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32) = thiscall 0x5EFFE0 => unported;
    /// `CPedScanner::GetClosestPedInRange()` (0x5FFF20)
    fn ped_scanner_get_closest_ped_in_range(s: GPtr<u8>) -> GPtr<u8> = thiscall 0x5FFF20 => unported;
    /// 0x5FFEE0 (the same getter of the vehicle scanner at intelligence +0xD4)
    fn scanner_get_closest_5ffee0(s: GPtr<u8>) -> GPtr<u8> = thiscall 0x5FFEE0 => unported;

    /// `CTask::Clone()` (vtable +4)
    fn v_clone(t: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x4 => unported;
    /// `CTaskComplexWander::CreateFirstSubTask(CPed*)` (0x6740E0, `ret 4`)
    fn wander_create_first_sub_task_orig(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x6740E0 => unported;
    /// `CTaskComplexWander::CreateNextSubTask(CPed*)` (0x674140, `ret 4`)
    fn wander_create_next_sub_task_orig(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x674140 => unported;
    /// `CTaskComplexWander::ControlSubTask(CPed*)` (0x674C30, `ret 4`)
    fn wander_control_sub_task_orig(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x674C30 => unported;
    /// `CTaskComplexWanderCop::ShouldPursuePlayer(CPed*)` (0x66B160, `ret 4`)
    fn cop_should_pursue_player_orig(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x66B160 => unported;
    /// `CTaskComplexWanderCop::LookForCarAlarms(CPed*)` (0x66B1B0, `ret 4`)
    fn cop_look_for_car_alarms_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x66B1B0 => unported;
    /// `CTaskComplexWanderCop::LookForStolenCopCars(CPed*)` (0x66B290, `ret 4`)
    fn cop_look_for_stolen_cop_cars_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x66B290 => unported;
    /// `CTaskComplexWanderCop::LookForCriminals(CPed*)` (0x66B300, `ret 4`)
    fn cop_look_for_criminals_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x66B300 => unported;
    /// `CTaskComplexPolicePursuit::Constructor` (0x68BA70)
    fn police_pursuit_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x68BA70 => unported;
    /// `CTaskTimer::IsOutOfTime` (0x420E30)
    fn task_timer_is_out_of_time(t: GPtr<u8>) -> u8 = thiscall 0x420E30 => unported;
    /// `CWorld::FindPlayerWanted(int32)` (0x56E230, cdecl)
    fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0, cdecl)
    fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CWanted::CanCopJoinPursuit-method(CCopPed*)` (0x562FB0, `ret 4`)
    fn wanted_can_cop_join_pursuit(w: GPtr<u8>, cop: GPtr<u8>) -> u8 = thiscall 0x562FB0 => unported;
    /// `CPlayerPed::SetWantedLevelNoDrop(int32)` (0x609F30, `ret 4` in the tail-jumped callee)
    fn player_set_wanted_level_no_drop(p: GPtr<u8>, level: i32) = thiscall 0x609F30 => unported;

    /// `CTaskComplexFollowPointRoute::GetLastWaypoint(CVector* out, CPed*)` (0x669340, `ret 8`)
    fn fpr_get_last_waypoint(t: GPtr<u8>, out: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x669340 => unported;
    /// `CTaskComplexFollowPointRoute::GetNextWaypoint(CVector* out, CPed*)` (0x669420, `ret 8`)
    fn fpr_get_next_waypoint(t: GPtr<u8>, out: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x669420 => unported;
    /// `CTaskComplexFollowNodeRoute::GetLastWaypoint(CVector* out, CPed*)` (0x6698E0, `ret 8`)
    fn fnr_get_last_waypoint(t: GPtr<u8>, out: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x6698E0 => unported;
    /// `CTaskComplexFollowNodeRoute::GetNextWaypoint(CVector* out, CPed*)` (0x669980, `ret 8`)
    fn fnr_get_next_waypoint(t: GPtr<u8>, out: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x669980 => unported;
    /// `CPedGeometryAnalyser::IsWanderPathClear(const CVector*, const CVector*, float, int32)` (0x5F2F70, cdecl)
    fn is_wander_path_clear(a: GPtr<u8>, b: GPtr<u8>, h: u32, n: i32) -> i32 = cdecl 0x5F2F70 => unported;

    // ---- math ----
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60, cdecl): `out = a - b`
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;

    // ---- avoid other ped (PORT-AI-4 main agent) ----
    /// `CColSphere::Set(float radius, const CVector& center, uint8 material, uint8 flags, uint8 lighting)` (0x40FD10,
    /// `ret 0x14`)
    fn col_sphere_set(s: GPtr<u8>, r: u32, c: GPtr<u8>, m: u32, f: u32, l: u32) = thiscall 0x40FD10 => unported;
    /// `CColSphere::IntersectSphere(const CColSphere&)` (0x410090, `ret 4`, AL)
    fn col_sphere_intersect_sphere(s: GPtr<u8>, o: GPtr<u8>) -> u8 = thiscall 0x410090 => unported;
    /// `CPedGroups::AreInSameGroup(const CPed*, const CPed*)` (0x5F7F40, AL)
    fn ped_groups_are_in_same_group(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x5F7F40 => unported;
    /// `CTaskComplexAvoidOtherPedWhileWandering::NearbyPedsInSphere(CPed*, const CColSphere&, CPed** cand, CPed** out)`
    /// (0x671FE0, `ret 0x10`)
    fn aop_nearby_peds_in_sphere(t: GPtr<u8>, ped: GPtr<u8>, s: GPtr<u8>, cand: GPtr<u8>, out: GPtr<u8>) -> u8 = thiscall 0x671FE0 => unported;
    /// `CTaskComplexAvoidOtherPedWhileWandering::ComputeSphere(CColSphere&, CPed**)` (0x66A320, `ret 8`)
    fn aop_compute_sphere(t: GPtr<u8>, s: GPtr<u8>, peds: GPtr<u8>) = thiscall 0x66A320 => unported;
    /// `CTaskComplexAvoidOtherPedWhileWandering::ComputeAvoidSphere(CPed*, CColSphere&)` (0x672080, `ret 8`)
    fn aop_compute_avoid_sphere(t: GPtr<u8>, ped: GPtr<u8>, s: GPtr<u8>) = thiscall 0x672080 => unported;
    /// `CTaskComplexAvoidOtherPedWhileWandering::ComputeRouteRoundSphere(CPed*, CColSphere&)` (0x66A7B0, `ret 8`)
    fn aop_compute_route_round_sphere(t: GPtr<u8>, ped: GPtr<u8>, s: GPtr<u8>) -> u32 = thiscall 0x66A7B0 => unported;
    /// `CPedGeometryAnalyser::ComputeRouteRoundSphere(ped, sphere, from, to, out1, out2)` (0x5F1890)
    fn geom_compute_route_round_sphere(
        ped: GPtr<u8>, s: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, o1: GPtr<u8>, o2: GPtr<u8>
    ) -> u32 = cdecl 0x5F1890 => unported;
    /// `CTaskComplexWanderStandard::LookForGangMembers(CPed*)` (0x670100, `ret 4`, AL)
    fn ws_look_for_gang_members(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x670100 => unported;
    /// `CTaskComplexWanderStandard::LookForChatPartners(CPed*)` (0x66FDA0, `ret 4`, AL)
    fn ws_look_for_chat_partners(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x66FDA0 => unported;
    /// `CTaskComplexWanderStandard::LookForSexyCars(CPed*)` (0x66AFD0, `ret 4`, AL)
    fn ws_look_for_sexy_cars(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x66AFD0 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn gen_random_in_range(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CTaskComplexWanderCop::CTaskComplexWanderCop(int32 moveState, uint8 dir)` (0x460C80, `ret 8`)
    fn wander_cop_ctor(t: GPtr<u8>, ms: u32, dir: u32) -> GPtr<u8> = thiscall 0x460C80 => unported;
    /// `CTaskComplexWanderMedic::CTaskComplexWanderMedic(int32, uint8, bool)` (0x658770, `ret 0xC`)
    fn wander_medic_ctor(t: GPtr<u8>, ms: u32, dir: u32, b: u32) -> GPtr<u8> = thiscall 0x658770 => unported;
    /// `CTaskComplexWanderCriminal::CTaskComplexWanderCriminal(int32, uint8, bool)` (0x48E610, `ret 0xC`)
    fn wander_criminal_ctor(t: GPtr<u8>, ms: u32, dir: u32, b: u32) -> GPtr<u8> = thiscall 0x48E610 => unported;
    /// `CTaskComplexWanderProstitute::CTaskComplexWanderProstitute(int32, uint8, bool)` (0x672690, `ret 0xC`)
    fn wander_prostitute_ctor(t: GPtr<u8>, ms: u32, dir: u32, b: u32) -> GPtr<u8> = thiscall 0x672690 => unported;
    /// `CTaskComplexWanderStandard::CTaskComplexWanderStandard(int32, uint8, bool)` (0x48E4F0, `ret 0xC`)
    fn wander_standard_ctor(t: GPtr<u8>, ms: u32, dir: u32, b: u32) -> GPtr<u8> = thiscall 0x48E4F0 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    fn ped_groups_get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupMembership::CountMembersExcludingLeader()` (0x5F6AA0)
    fn membership_count_members_excluding_leader(m: GPtr<u8>) -> i32 = thiscall 0x5F6AA0 => unported;
    /// `CEventAcquaintancePed` constructor (0x4AF820, `ret 4`)
    fn acquaintance_ped_ctor(e: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AF820 => unported;
    /// `FUN_004af890`: the acquaintance event destructor the original calls on its stack event
    fn acquaintance_ped_dtor(e: GPtr<u8>) = thiscall 0x4AF890 => unported;
    /// `FUN_004af010`: the sexy-vehicle event constructor (`ret 4`)
    fn sexy_vehicle_ctor(e: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AF010 => unported;
    /// `FUN_004af0a0`: its destructor
    fn sexy_vehicle_dtor(e: GPtr<u8>) = thiscall 0x4AF0A0 => unported;
    /// `CTaskComplexWanderStandard::SetNextMinScanTime(CPed*)` (0x66AF60, `ret 4`)
    fn ws_set_next_min_scan_time(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x66AF60 => unported;
    /// `CWorld::GetIsLineOfSightClear(a, b, buildings, vehicles, peds, objects, dummies, seeThrough, camera)` (0x56A490)
    fn world_los_clear(a: GPtr<u8>, b: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32, st: u32, ca: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add_ws(g: GPtr<u8>, e: GPtr<u8>, f: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CTaskManager::GetActiveTask()` (0x681720)
    fn tm_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_type_ws(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CEventPedToChase::CEventPedToChase(CPed*)` (0x4AF130, `ret 4`)
    fn ped_to_chase_ctor(e: GPtr<u8>, p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AF130 => unported;
    /// `CEventPedToChase::~CEventPedToChase` (0x4AF1C0)
    fn ped_to_chase_dtor(e: GPtr<u8>) = thiscall 0x4AF1C0 => unported;
    /// `CEventPedToFlee::CEventPedToFlee(CPed*)` (0x4AF240, `ret 4`)
    fn ped_to_flee_ctor(e: GPtr<u8>, p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AF240 => unported;
    /// `CEventPedToFlee::~CEventPedToFlee` (0x4AF2D0)
    fn ped_to_flee_dtor(e: GPtr<u8>) = thiscall 0x4AF2D0 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad_ws(id: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CEntity::GetIsOnScreen()` (0x534540)
    fn entity_get_is_on_screen_ws(e: GPtr<u8>) -> u8 = thiscall 0x534540 => unported;
    /// `IKChainManager_c::GetLookAtEntity(CPed*)` (0x6181D0, `ret 4`)
    fn ik_get_look_at_entity_ws(m: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x6181D0 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810, `ret 4`)
    fn tm_get_task_secondary_ws(tm: GPtr<u8>, slot: u32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CVector::MagnitudeSqr()` (0x406DA0, ST0)
    fn vector_magnitude_sqr_ws(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `IKChainManager_c::LookAt(...)` (0x618970, `ret 0x2C`)
    fn ik_look_at_ws(
        m: GPtr<u8>, purpose: u32, ped: GPtr<u8>, target: u32, time: u32, bone: u32, posn: GPtr<u8>,
        torso: u32, speed: u32, blend: u32, prio: u32, force: u32
    ) -> u32 = thiscall 0x618970 => unported;
    /// `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18)
    fn v_make_abortable_ws(t: GPtr<u8>, ped: GPtr<u8>, prio: u32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplexAvoidOtherPedWhileWandering::SetUpIK(CPed*)` (0x66A850, `ret 4`)
    fn aop_set_up_ik_call(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x66A850 => unported;
    /// `CTaskComplexAvoidOtherPedWhileWandering::QuitIK(CPed*)` (0x66A230, `ret 4`)
    fn aop_quit_ik_call(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x66A230 => unported;
    /// `CTaskComplexAvoidOtherPedWhileWandering::ComputeDetourTarget(CPed*)` (0x672180, `ret 4`, AL)
    fn aop_compute_detour_target_call(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x672180 => unported;
    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(int32, bool, bool, float)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor_ws(t: GPtr<u8>, time: u32, looped: u32, keep: u32, delta: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    fn tm_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTaskTimer::IsOutOfTime()` (0x420E30, AL)
    fn task_timer_is_out_of_time_ws(t: GPtr<u8>) -> u8 = thiscall 0x420E30 => unported;
    /// `CTask::IsGoToTask(CTask*)` (0x61A360, cdecl, AL)
    fn task_is_go_to_task(t: GPtr<u8>) -> u8 = cdecl 0x61A360 => unported;
    /// `CVector::DotProduct(const CVector*, const CVector*)` (0x40FDB0, ST0)
    fn vector_dot_product_ws(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, returns `out`)
    fn vector_mul_ws(out: GPtr<u8>, v: GPtr<u8>, f: f32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `FUN_0040fe30` (`operator+(const CVector&, const CVector&)`, returns `out`)
    fn vector_add_ws(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0, `ret 4`)
    fn intel_find_task_by_type_ws(i: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `CTaskComplexGangFollower::CalculateOffsetPosition(CVector*)` (0x65ED40, `ret 4`)
    fn gang_follower_calculate_offset_position(t: GPtr<u8>, out: GPtr<u8>) = thiscall 0x65ED40 => unported;
    /// `CTaskSimpleGoToPoint::UpdatePoint(const CVector&, float, bool)` (0x645700, `ret 0xC`)
    fn goto_point_update_point_ws(t: GPtr<u8>, p: GPtr<u8>, r: u32, force: u32) = thiscall 0x645700 => unported;
}

/// A local buffer's address as a `GPtr` (the original's stack object).
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// Entity position: matrix +0x30 when the matrix pointer (+0x14) is set, else the placement at +4.
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `fcomp` + `fnstsw` + `test ah, 0x41` + `jne` (taken unless ST0 > m).
fn gt(a: X87, b: X87) -> bool {
    a.fcom(b) == Cmp::Greater
}

/// `fcomp` + `test ah, 5` + `jp` not taken (ST0 < m).
fn lt(a: X87, b: X87) -> bool {
    a.fcom(b) == Cmp::Less
}

// =====================================================================================================
// CTaskComplexFollowPointRoute
// =====================================================================================================

/// `CTaskComplexFollowPointRoute::GetSubTaskType()` (0x668FE0, `ret`): 0x516 for an empty route, 0x384
/// while points remain, 0x387/0x384 (flag 4) at the last point; past the end it counts a traversal (+0x10)
/// and switches on the mode (+0xC, jump table 0x66907C): 0 clears flags 0x70 when flag 8 is set and
/// returns 0x516; 1 returns 0x516 after the first traversal; 1/2 reverse the route (0x667720), 2/3 reset
/// the index and tail-jump to the entry (re-entering through the hook); an index beyond the end returns the
/// 0xC8 the protection island at 0x401163 loads. The island is straight code.
#[hook(addr = 0x668FE0, conv = thiscall, name = "CTaskComplexFollowPointRoute::GetSubTaskType", expect = "56 8B F1 8B 4E 20")]
pub fn fpr_get_sub_task_type(this: GPtr<u8>) -> u32 {
    let route = rd::<GPtr<u8>>(this, 0x20);
    let n = rd::<i32>(route, 0);
    if n == 0 {
        return 0x516;
    }
    let idx = rd::<i32>(this, 0x24);
    let nx = idx.wrapping_add(1);
    if nx < n {
        return 0x384;
    }
    if nx == n {
        return if rd::<u8>(this, 0x28) & 4 != 0 { 0x387 } else { 0x384 };
    }
    if idx != n {
        return 0xC8;
    }
    let c = rd::<i32>(this, 0x10).wrapping_add(1);
    wr::<i32>(this, 0x10, c);
    match rd::<u32>(this, 0xC) {
        0 => {
            let f = rd::<u32>(this, 0x28);
            if f & 8 != 0 {
                wr::<u32>(this, 0x28, f & 0xFFFF_FF8F);
            }
            0x516
        }
        1 if c != 1 => 0x516,
        1 | 2 => {
            point_route_667720(route);
            wr::<u32>(this, 0x24, 0);
            fpr_get_sub_task_type_orig(this)
        }
        3 => {
            wr::<u32>(this, 0x24, 0);
            fpr_get_sub_task_type_orig(this)
        }
        _ => 0xC8,
    }
}

/// The current route point (`route + 4 + 12 * index`).
fn fpr_point(this: GPtr<u8>) -> GPtr<u8> {
    let i = rd::<i32>(this, 0x24);
    rd::<GPtr<u8>>(this, 0x20).byte_add((i as isize).wrapping_mul(12).wrapping_add(4))
}

/// `CTaskComplexFollowPointRoute::CreateTask(eTaskType, CPed*)` (0x669090, `ret 8`): 0x2C0 leave car
/// (`CTaskComplexLeaveCar(veh, 0, 0, 1, 0)`), 0xC8 leave group, 0xCB stand still (`[0x86DB24]`, 8.0 blend),
/// 0x384 go to point (radius `[0x86FC78]`, flag bit 0), 0x387 go to point and stand still (slow-down
/// distance zeroed by mode/traversals; the bool is the ped argument slot with its low byte replaced, pushed
/// as a dword, kept), 0x39E go to point fine (copies the low nibble of byte +0x1C from a 0x384 sub-task when
/// flag 0x20 is set; that write happens through a null pointer when the pool is full, kept), else null.
#[hook(addr = 0x669090, conv = thiscall, name = "CTaskComplexFollowPointRoute::CreateTask", expect = "6A FF 68 D2 4F 84 00")]
pub fn fpr_create_task(this: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> {
    match ty {
        0x2C0 => {
            let t = task_new(0x34);
            if t.is_null() {
                return t;
            }
            leave_car_ctor(t, rd::<u32>(ped, 0x58C), 0, 0, 1, 0)
        }
        0xC8 => {
            let t = task_new(8);
            if t.is_null() {
                return t;
            }
            leave_group_ctor(t)
        }
        0xCB => {
            let t = task_new(0x20);
            if t.is_null() {
                return t;
            }
            stand_still_ctor(t, g::<u32>(0x86DB24).read(), 0, 0, 0x41000000)
        }
        0x384 => {
            let t = task_new(0x30);
            if t.is_null() {
                return t;
            }
            let flag = u32::from(rd::<u8>(this, 0x28)) & 1;
            go_to_point_ctor(t, rd::<u32>(this, 0x14), fpr_point(this), g::<u32>(0x86FC78).read(), 0, flag)
        }
        0x387 => {
            let mut slow = rd::<u32>(this, 0x1C);
            match rd::<u32>(this, 0xC) {
                1 => {
                    if rd::<i32>(this, 0x10) == 0 {
                        slow = 0;
                    }
                }
                2 | 3 => slow = 0,
                _ => {}
            }
            let mut door = (ped.addr() as u32) & 0xFFFF_FF00;
            let parent = rd::<GPtr<u8>>(this, 4);
            if !parent.is_null() && v_get_task_type(parent) == 0x1FC && walk_round_car_going_for_door(rd::<GPtr<u8>>(this, 4)) != 0 {
                door |= 1;
            }
            let t = task_new(0x28);
            if t.is_null() {
                return t;
            }
            go_to_point_and_stand_still_ctor(t, rd::<u32>(this, 0x14), fpr_point(this), rd::<u32>(this, 0x18), slow, 0, door)
        }
        0x39E => {
            let t = task_new(0x24);
            let t = if t.is_null() {
                t
            } else {
                let p = fpr_point(this);
                let (vx, vy, vz) = (rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8));
                let r = f32_of(X87::from_f64_exact(go_to_point_fine_base_ratio(rd::<u32>(this, 0x14))));
                go_to_point_fine_ctor(t, r.to_bits(), vx, vy, vz, g::<u32>(0x86FC78).read(), 0)
            };
            if rd::<u8>(this, 0x28) & 0x20 != 0 {
                let sub = rd::<GPtr<u8>>(this, 8);
                if !sub.is_null() && v_get_task_type(sub) == 0x384 {
                    let c = rd::<u8>(rd::<GPtr<u8>>(this, 8), 0x1C);
                    let a = rd::<u8>(t, 0x1C);
                    wr::<u8>(t, 0x1C, (a & 0xF0) | (c & 0x0F));
                }
            }
            t
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexFollowPointRoute::CreateFirstSubTask(CPed*)` (0x66E320, `ret 4`): clears flag 2 (and the
/// blend fields when flag 8 is set); empty route → 0xC8, in a car → 0x2C0; for a mode other than 0 picks
/// the start index: the end point of the first segment the ped projects onto (inside the segment), the
/// closest by squared distance; else the closest point. Both minimum searches store the squared distance
/// and compare the next one against its square (original behaviour, kept). The search loops follow the
/// x87 sequence (registers vs stored f32 values) of the original; the second loop is unrolled by four in
/// the original, the order of the compares is the same.
#[hook(addr = 0x66E320, conv = thiscall, name = "CTaskComplexFollowPointRoute::CreateFirstSubTask", expect = "83 EC 3C 53 8B D9")]
pub fn fpr_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let f = rd::<u32>(this, 0x28) & !2;
    wr::<u32>(this, 0x28, f);
    if f & 8 != 0 {
        for o in [0x2C, 0x30, 0x34, 0x38] {
            wr::<u32>(this, o, 0);
        }
    }
    let n = rd::<i32>(rd::<GPtr<u8>>(this, 0x20), 0);
    if n == 0 {
        return fpr_create_task_orig(this, 0xC8, ped);
    }
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        return fpr_create_task_orig(this, 0x2C0, ped);
    }
    if rd::<u32>(this, 0xC) != 0 {
        let pos = ent_pos(ped);
        let (qx, qy, qz) = (rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8));
        let mut best: i32 = -1;
        let mut best_d = f32::from_bits(0x7F7F_FFFF);
        let mut i: i32 = 0;
        while i < n {
            let a = i.wrapping_add(1).wrapping_rem(n);
            let b = i.wrapping_rem(n);
            let route = rd::<GPtr<u8>>(this, 0x20);
            let pb = route.byte_add((b as isize) * 12 + 4);
            let pa = route.byte_add((a as isize) * 12 + 4);
            let mut d = [0u32; 3];
            let dv = buf(&mut d);
            let dx = f32_of(x(rd::<f32>(pa, 0)).fsub_m(rd::<f32>(pb, 0)));
            let dy = f32_of(x(rd::<f32>(pa, 4)).fsub_m(rd::<f32>(pb, 4)));
            let dzr = x(rd::<f32>(pa, 8)).fsub_m(rd::<f32>(pb, 8));
            let dz = f32_of(dzr);
            let len2 = f32_of(dzr.fmul_m(dz).fadd(x(dx).fmul_m(dx)).fadd(x(dy).fmul_m(dy)));
            wr::<f32>(dv, 0, dx);
            wr::<f32>(dv, 4, dy);
            wr::<f32>(dv, 8, dz);
            vector_normalise(dv);
            let (nx, ny, nz) = (rd::<f32>(dv, 0), rd::<f32>(dv, 4), rd::<f32>(dv, 8));
            let ra = x(qx).fsub_m(rd::<f32>(pb, 0));
            let rb = x(qy).fsub_m(rd::<f32>(pb, 4));
            let rc = x(qz).fsub_m(rd::<f32>(pb, 8));
            let t = rc.fmul_m(nz).fadd(rb.fmul_m(ny)).fadd(ra.fmul_m(nx));
            let tf = f32_of(t);
            if gt(t, x(g::<f32>(0x858B50).read())) && gt(x(len2).fsqrt(), x(tf)) {
                let sx = x(nx).fmul_m(tf);
                let sy = x(ny).fmul_m(tf);
                let sz = f32_of(x(nz).fmul_m(tf));
                let px = f32_of(sx.fadd_m(rd::<f32>(pb, 0)));
                let py = sy.fadd_m(rd::<f32>(pb, 4));
                let pz = x(sz).fadd_m(rd::<f32>(pb, 8));
                let ex = f32_of(x(qx).fsub_m(px));
                let ey = f32_of(x(qy).fsub(py));
                let ez = x(qz).fsub(pz);
                let d2 = ez.fmul(ez).fadd(x(ey).fmul_m(ey)).fadd(x(ex).fmul_m(ex));
                if lt(d2, x(best_d).fmul_m(best_d)) {
                    best_d = f32_of(d2);
                    best = a;
                }
            }
            i += 1;
        }
        if best == -1 {
            let mut k = x(g::<f32>(0x863A3C).read());
            let route = rd::<GPtr<u8>>(this, 0x20);
            let mut j: i32 = 0;
            while j < n {
                let p = route.byte_add((j as isize) * 12 + 4);
                let ra = x(qx).fsub_m(rd::<f32>(p, 0));
                let rb = x(qy).fsub_m(rd::<f32>(p, 4));
                let rc = x(qz).fsub_m(rd::<f32>(p, 8));
                let d2 = rc.fmul(rc).fadd(rb.fmul(rb)).fadd(ra.fmul(ra));
                let d2f = f32_of(d2);
                if lt(d2, k.fmul(k)) {
                    best = j;
                    k = x(d2f);
                }
                j += 1;
            }
        }
        wr::<i32>(this, 0x24, best);
    }
    let mut ty = fpr_get_sub_task_type_orig(this);
    if rd::<u8>(this, 0x28) & 8 != 0 {
        ty = fpr_calc_go_to_task_type(this, ped, ty);
    }
    fpr_create_task_orig(this, ty, ped)
}

/// `CTaskComplexFollowPointRoute::ControlSubTask(CPed*)` (0x66E860, `ret 4`): keeps a leave-car sub-task;
/// null when the ped's intelligence +0x274 is above `[0x86C938]` or the route is empty; restarts on flag 2;
/// with flag 8 drives the speed blending of a `CTaskSimpleGoToPointFine` sub-task (`CalcBlendRatio`,
/// `SetMoveRatio`, switching between 0x384 and 0x39E). The ratio lives in the original's ped argument slot
/// (a local here).
#[hook(addr = 0x66E860, conv = thiscall, name = "CTaskComplexFollowPointRoute::ControlSubTask", expect = "56 8B F1 8B 4E 08")]
pub fn fpr_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = || rd::<GPtr<u8>>(this, 8);
    if v_get_task_type(sub()) == 0x2C0 {
        return sub();
    }
    let lim = g::<i32>(0x86C938).read();
    if rd::<i32>(rd::<GPtr<u8>>(ped, 0x47C), 0x274) > lim {
        return GPtr::NULL;
    }
    if rd::<i32>(rd::<GPtr<u8>>(this, 0x20), 0) <= 0 {
        return GPtr::NULL;
    }
    let f = rd::<u32>(this, 0x28);
    if f & 2 != 0 {
        return v_create_first_sub_task(this, ped);
    }
    if f & 8 != 0 && v_get_task_type(sub()) == 0xC8 {
        wr::<u32>(this, 0x28, rd::<u32>(this, 0x28) & 0xFFFF_FF8F);
        return sub();
    }
    let f = rd::<u32>(this, 0x28);
    if f & 8 == 0 {
        return sub();
    }
    if f & 0x20 != 0 && v_get_task_type(sub()) == 0x39E {
        let r = X87::from_f64_exact(fpr_calc_blend_ratio(this, ped, 1));
        let mut ratio = f32_of(r);
        if lt(r, x(g::<f32>(0x858B50).read())) {
            ratio = f32_of(X87::from_f64_exact(go_to_point_fine_base_ratio(rd::<u32>(this, 0x14))));
        }
        go_to_point_fine_set_move_ratio(sub(), ratio.to_bits());
    } else if rd::<u8>(this, 0x28) & 0x40 != 0 && v_get_task_type(sub()) == 0x39E {
        let r = X87::from_f64_exact(fpr_calc_blend_ratio(this, ped, 0));
        let ratio = f32_of(r);
        if r.fcom(x(g::<f32>(0x858B50).read())).c0() {
            let f = rd::<u32>(this, 0x28);
            let ms = rd::<u32>(this, 0x14);
            wr::<u32>(this, 0x28, f & 0xFFFF_FFBF);
            let b = f32_of(X87::from_f64_exact(go_to_point_fine_base_ratio(ms)));
            go_to_point_fine_set_move_ratio(sub(), b.to_bits());
            return fpr_create_task_orig(this, 0x384, ped);
        }
        go_to_point_fine_set_move_ratio(sub(), ratio.to_bits());
    }
    // 0x66E933
    let f = rd::<u32>(this, 0x28);
    if f & 0x10 == 0 || f & 0x20 != 0 {
        return sub();
    }
    let r = X87::from_f64_exact(fpr_calc_blend_ratio(this, ped, 1));
    let ratio = f32_of(r);
    if r.fcom(x(g::<f32>(0x858B50).read())).c0() {
        return sub();
    }
    wr::<u32>(this, 0x28, (rd::<u32>(this, 0x28) & 0xFFFF_FFAF) | 0x20);
    if v_get_task_type(sub()) == 0x39E {
        go_to_point_fine_set_move_ratio(sub(), ratio.to_bits());
        return sub();
    }
    let t = fpr_create_task_orig(this, 0x39E, ped);
    go_to_point_fine_set_move_ratio(t, ratio.to_bits());
    t
}

/// `CTaskComplexFollowPointRoute::CTaskComplexFollowPointRoute(moveState, const CPointRoute&, mode, radius,
/// slowDown, overshoot, blending, standStill)` (0x671510, `ret 0x20`): the flags dword (+0x28) keeps the
/// pool slot's old bits outside 0x0D (and 0x02/0x20/0x40 as set below); a new `CPointRoute` (0x64 bytes,
/// count zeroed) is copied from the argument by 0x63A040 (with a null `this` when the allocation fails,
/// kept). The entry continues through the island at 0x401D9D (`pushfd; stc; sbb`: the SEH handler push).
/// Not hookable: its entry bytes are read as data by the protection (0x4022BB, `catalog/code-data-reads.csv`).
#[allow(clippy::too_many_arguments)]
pub fn fpr_ctor(this: GPtr<u8>, ms: u32, route: GPtr<u8>, mode: u32, radius: u32, slow: u32, overshoot: u32, blending: u32, stand_still: u32) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0xC, mode);
    wr::<u32>(this, 0x14, ms);
    let bits = ((((blending & 1) << 1) | (stand_still & 1)) << 2) | (overshoot & 1);
    let f = bits | (rd::<u32>(this, 0x28) & 0xFFFF_FFF2);
    wr::<u32>(this, 0, 0x870078);
    wr::<u32>(this, 0x28, f);
    if f & 8 != 0 {
        wr::<u32>(this, 0x28, f & 0xFFFF_FF9F);
        for o in [0x2C, 0x30, 0x34, 0x38] {
            wr::<u32>(this, o, 0);
        }
    }
    let r = point_route_new(0x64);
    if !r.is_null() {
        wr::<u32>(r, 0, 0);
    }
    wr::<GPtr<u8>>(this, 0x20, r);
    point_route_copy_63a040(r, route);
    wr::<u32>(this, 0x1C, slow);
    let f = rd::<u32>(this, 0x28) | 2;
    wr::<u32>(this, 0x18, radius);
    wr::<u32>(this, 0x24, 0);
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0x28, f);
    if f & 8 != 0 {
        for o in [0x2C, 0x30, 0x34, 0x38] {
            wr::<u32>(this, o, 0);
        }
        wr::<u32>(this, 0x28, (f & 0xFFFF_FFDF) | 0x40);
    }
    this
}

/// `CTaskComplexFollowPointRoute::CreateNextSubTask(CPed*)` (0x671630, `ret 4`): one `GetTaskType` call per
/// compare (0x516 → null, 0x2C0 → null in a car else `CreateFirstSubTask`, 0xC8/0xCB → 0x516, 0x384 at the
/// last point without flag 4 → 0x516); otherwise advances the index and creates the next go-to task.
#[hook(addr = 0x671630, conv = thiscall, name = "CTaskComplexFollowPointRoute::CreateNextSubTask", expect = "56 8B F1 8B 4E 08")]
pub fn fpr_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = || rd::<GPtr<u8>>(this, 8);
    if v_get_task_type(sub()) == 0x516 {
        return GPtr::NULL;
    }
    if v_get_task_type(sub()) == 0x2C0 {
        if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
            return GPtr::NULL;
        }
        return v_create_first_sub_task(this, ped);
    }
    if v_get_task_type(sub()) == 0xC8 {
        return fpr_create_task_orig(this, 0x516, ped);
    }
    if v_get_task_type(sub()) == 0xCB {
        return fpr_create_task_orig(this, 0x516, ped);
    }
    if v_get_task_type(sub()) == 0x384
        && rd::<i32>(this, 0x24).wrapping_add(1) == rd::<i32>(rd::<GPtr<u8>>(this, 0x20), 0)
        && rd::<u8>(this, 0x28) & 4 == 0
    {
        return fpr_create_task_orig(this, 0x516, ped);
    }
    wr::<i32>(this, 0x24, rd::<i32>(this, 0x24).wrapping_add(1));
    let mut ty = fpr_get_sub_task_type_orig(this);
    if rd::<u8>(this, 0x28) & 8 != 0 {
        ty = fpr_calc_go_to_task_type(this, ped, ty);
    }
    fpr_create_task_orig(this, ty, ped)
}

/// Shared body of the two route `MakeAbortable`s: for a damage-type event (type 1 or 0x38) whose
/// entity (+0x14) moves slower than `[0x86FC74]` (squared speed) and blocks a go-to-point sub-task's
/// target, the sub-task's radius (+0x18) becomes the 2D distance between the ped and that entity. The
/// original overwrites its own event argument slot with the radius it passes (not read again).
fn route_abort_widen_radius(this: GPtr<u8>, ped: GPtr<u8>, event: GPtr<u8>) {
    let ty = v_event_get_type(event);
    let ent = if ty == 1 || ty == 0x38 { rd::<GPtr<u8>>(event, 0x14) } else { GPtr::NULL };
    let st = v_get_task_type(rd::<GPtr<u8>>(this, 8));
    if st != 0x384 && st != 0x39E {
        return;
    }
    let sub = rd::<GPtr<u8>>(this, 8);
    if sub.is_null() || ent.is_null() {
        return;
    }
    let vz = x(rd::<f32>(ent, 0x4C));
    let vy = x(rd::<f32>(ent, 0x48));
    let vx = x(rd::<f32>(ent, 0x44));
    let s2 = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz));
    if !lt(s2, x(g::<f32>(0x86FC74).read())) {
        return;
    }
    if is_entity_blocking_target(ent, sub.byte_add(0xC), rd::<u32>(sub, 0x18)) == 0 {
        return;
    }
    let ep = ent_pos(ent);
    let pp = ent_pos(ped);
    let mut d = [0u32; 3];
    let r = vector_sub(buf(&mut d), pp, ep);
    let y = x(rd::<f32>(r, 4));
    let xx = x(rd::<f32>(r, 0));
    wr::<f32>(sub, 0x18, f32_of(xx.fmul(xx).fadd(y.fmul(y)).fsqrt()));
}

/// `CTaskComplexFollowPointRoute::MakeAbortable(CPed*, int32, const CEvent*)` (0x668EC0, `ret 0xC`): with
/// an event and points left (index + 1 < count) widens a blocked go-to-point radius
/// (`route_abort_widen_radius`); priority 0 empties the route (count = 0); then the sub-task's
/// `MakeAbortable`.
#[hook(addr = 0x668EC0, conv = thiscall, name = "CTaskComplexFollowPointRoute::MakeAbortable", expect = "83 EC 0C 53 55")]
pub fn fpr_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    if !event.is_null() && rd::<i32>(this, 0x24).wrapping_add(1) < rd::<i32>(rd::<GPtr<u8>>(this, 0x20), 0) {
        route_abort_widen_radius(this, ped, event);
    }
    if priority == 0 {
        wr::<i32>(rd::<GPtr<u8>>(this, 0x20), 0, 0);
    }
    v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, priority, event)
}

// =====================================================================================================
// CTaskComplexFollowNodeRoute
// =====================================================================================================

/// `CTaskComplexFollowNodeRoute::MakeAbortable(CPed*, int32, const CEvent*)` (0x669520, `ret 0xC`): with an
/// event and the index not at the last point (index + 1 != count) widens a blocked go-to-point radius
/// (`route_abort_widen_radius`), then the sub-task's `MakeAbortable`.
#[hook(addr = 0x669520, conv = thiscall, name = "CTaskComplexFollowNodeRoute::MakeAbortable", expect = "83 EC 0C 53 55")]
pub fn fnr_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    if !event.is_null() && rd::<i32>(this, 0x38).wrapping_add(1) != rd::<i32>(rd::<GPtr<u8>>(this, 0x30), 0) {
        route_abort_widen_radius(this, ped, event);
    }
    v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, priority, event)
}

/// `CTaskComplexFollowNodeRoute::GetSubTaskType(int32 index, bool, const CPointRoute*)` (0x669650, static
/// cdecl): 0x384 when the bool is false; else 0x516 at index == count, 0x387 at index + 1 == count, 0x384.
#[hook(addr = 0x669650, conv = cdecl, name = "CTaskComplexFollowNodeRoute::GetSubTaskType", expect = "8A 44 24 08 84 C0", preserve = "edx")]
pub fn fnr_get_sub_task_type(idx: i32, flag: u32, route: GPtr<u8>) -> u32 {
    if flag as u8 == 0 {
        return 0x384;
    }
    let n = rd::<i32>(route, 0);
    if idx == n {
        return 0x516;
    }
    if idx.wrapping_add(1) == n { 0x387 } else { 0x384 }
}

/// The current node-route point (`route(+0x30) + 4 + 12 * index(+0x38)`).
fn fnr_point(this: GPtr<u8>) -> GPtr<u8> {
    let i = rd::<i32>(this, 0x38);
    rd::<GPtr<u8>>(this, 0x30).byte_add((i as isize).wrapping_mul(12).wrapping_add(4))
}

/// `CTaskComplexFollowNodeRoute::CreateSubTask(eTaskType, CPed*)` (0x669690, `ret 8`): 0x384 go to point
/// (radius `[0x86FC78]`), 0x2C0 leave car, 0xCB stand still (then teleports the ped to the target +0xC
/// through the ped's vtable +0x38, also when the pool is full), 0x387 go to point and stand still, 0x39E
/// go to point fine; anything else null.
#[hook(addr = 0x669690, conv = thiscall, name = "CTaskComplexFollowNodeRoute::CreateSubTask", expect = "64 A1 00 00 00 00")]
pub fn fnr_create_sub_task(this: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> {
    match ty {
        0x384 => {
            let t = task_new(0x30);
            if t.is_null() {
                return t;
            }
            go_to_point_ctor(t, rd::<u32>(this, 0x18), fnr_point(this), g::<u32>(0x86FC78).read(), 0, 0)
        }
        0x2C0 => {
            let t = task_new(0x34);
            if t.is_null() {
                return t;
            }
            leave_car_ctor(t, rd::<u32>(ped, 0x58C), 0, 0, 1, 0)
        }
        0xCB => {
            let t = task_new(0x20);
            let t = if t.is_null() { t } else { stand_still_ctor(t, 0, 0, 0, 0x41000000) };
            v_entity_teleport(ped, rd::<u32>(this, 0xC), rd::<u32>(this, 0x10), rd::<u32>(this, 0x14), 0);
            t
        }
        0x387 => {
            let t = task_new(0x28);
            if t.is_null() {
                return t;
            }
            go_to_point_and_stand_still_ctor(t, rd::<u32>(this, 0x18), fnr_point(this), rd::<u32>(this, 0x1C), rd::<u32>(this, 0x20), 0, 0)
        }
        0x39E => {
            let t = task_new(0x24);
            if t.is_null() {
                return t;
            }
            let p = fnr_point(this);
            let (vx, vy, vz) = (rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8));
            let r = f32_of(X87::from_f64_exact(go_to_point_fine_base_ratio(rd::<u32>(this, 0x18))));
            go_to_point_fine_ctor(t, r.to_bits(), vx, vy, vz, g::<u32>(0x86FC78).read(), 0)
        }
        _ => GPtr::NULL,
    }
}

/// `ThePaths` (`CPathFind`, 0x96F050)
const THE_PATHS: usize = 0x96F050;
/// `ThePaths.m_pPathNodes[area]` (0x96F854)
const PATH_NODES: usize = 0x96F854;

/// `ThePaths.m_pPathNodes[area]` of a `CNodeAddress` dword (area = low word).
fn area_nodes(addr: u32) -> GPtr<u8> {
    g::<GPtr<u8>>(PATH_NODES + ((addr & 0xFFFF) as usize) * 4).read()
}

/// The path node of a `CNodeAddress` dword (node = high word, 0x1C bytes each); the area must be loaded.
fn path_node(addr: u32) -> GPtr<u8> {
    area_nodes(addr).byte_add(((addr >> 16) as isize) * 0x1C)
}

/// `CTaskComplexFollowNodeRoute::ComputeRoute()` (0x6699E0, `ret`): empties the point route (+0x30) and
/// appends the position of every loaded node of the node route (+0x2C; `int16` coordinates times
/// `[0x858C48]`) while it holds fewer than 8 points, then the target (+0xC) if there is room; bit 1 of
/// +0x4C = target appended. The original unrolls the node loop by four (same order). EAX is left as a
/// masked scratch value (not a result).
#[hook(addr = 0x6699E0, conv = thiscall, name = "CTaskComplexFollowNodeRoute::ComputeRoute", expect = "8B 41 30 83 EC 10")]
pub fn fnr_compute_route(this: GPtr<u8>) {
    wr::<u32>(rd::<GPtr<u8>>(this, 0x30), 0, 0);
    let n = rd::<i32>(rd::<GPtr<u8>>(this, 0x2C), 0);
    let k = g::<f32>(0x858C48).read();
    let mut i: i32 = 0;
    while i < n {
        let a = rd::<u32>(rd::<GPtr<u8>>(this, 0x2C), 4 + (i as isize) * 4);
        if !area_nodes(a).is_null() {
            let node = path_node(a);
            let px = f32_of(xi(i32::from(rd::<i16>(node, 8))).fmul_m(k));
            let py = f32_of(xi(i32::from(rd::<i16>(node, 0xA))).fmul_m(k));
            let pz = f32_of(xi(i32::from(rd::<i16>(node, 0xC))).fmul_m(k));
            let r = rd::<GPtr<u8>>(this, 0x30);
            let c = rd::<i32>(r, 0);
            if c < 8 {
                let p = r.byte_add((c as isize) * 12 + 4);
                wr::<f32>(p, 0, px);
                wr::<f32>(p, 4, py);
                wr::<f32>(p, 8, pz);
                wr::<i32>(r, 0, rd::<i32>(r, 0).wrapping_add(1));
            }
        }
        i += 1;
    }
    let r = rd::<GPtr<u8>>(this, 0x30);
    let c = rd::<i32>(r, 0);
    let added = if c < 8 {
        let p = r.byte_add((c as isize) * 12 + 4);
        for o in [0, 4, 8] {
            wr::<u32>(p, o, rd::<u32>(this, 0xC + o));
        }
        wr::<i32>(r, 0, rd::<i32>(r, 0).wrapping_add(1));
        2
    } else {
        0
    };
    wr::<u32>(this, 0x4C, (rd::<u32>(this, 0x4C) & !2) | added);
}

// =====================================================================================================
// CTaskComplexWander
// =====================================================================================================

/// `CTaskComplexWander::UpdateDir(CPed*)` (0x669DA0, `ret 4`): at a current node (+0x1C) whose flags byte
/// (+0x18) has a low nibble >= 3, once per frame (`[0xB7CB4C]` vs +0x20) and with bit 0 of +0x24, rolls
/// `(ped +0x20 + 3 * frame) % 100` (no `rand`): > 90 turns the direction (+0x10) by +6, > 80 by +2 (mod 8).
/// A changed direction asks the virtual `UpdatePathNodes` (+0x38) for the node's direction (the out byte is
/// the original's ped argument slot, which holds the node address dword at that point; the direction
/// argument is pushed as a dword whose upper bytes are a stale stack slot, passed zero-extended here) and
/// keeps the new direction only when it is within 3 steps of the returned one.
#[hook(addr = 0x669DA0, conv = thiscall, name = "CTaskComplexWander::UpdateDir", expect = "83 EC 0C 53 56")]
pub fn wander_update_dir(this: GPtr<u8>, ped: GPtr<u8>) {
    let dir = rd::<u8>(this, 0x10);
    let mut nd = dir;
    let mut slot = [ped.addr() as u32];
    if rd::<u16>(this, 0x1C) != 0xFFFF {
        let a = rd::<u32>(this, 0x1C);
        if !area_nodes(a).is_null() {
            slot[0] = a;
            let node = path_node(a);
            if !node.is_null() && rd::<u8>(node, 0x18) & 0xF >= 3 {
                let frame = g::<u32>(0xB7CB4C).read();
                if frame != rd::<u32>(this, 0x20) && rd::<u8>(this, 0x24) & 1 != 0 {
                    wr::<u32>(this, 0x20, frame);
                    let v = u32::from(rd::<u16>(ped, 0x20)).wrapping_add(frame.wrapping_mul(3));
                    let m = (v % 100) as u8;
                    if m > 0x5A {
                        nd = dir.wrapping_add(6) & 7;
                    } else if m > 0x50 {
                        nd = dir.wrapping_add(10) & 7;
                    }
                }
            }
        }
    }
    if nd == dir {
        return;
    }
    // [0] = current node (the original's [esp+0x10]), [1] = last node ([esp+0x14])
    let mut nodes = [rd::<u32>(this, 0x1C), rd::<u32>(this, 0x18)];
    let np = buf(&mut nodes);
    let sp = buf(&mut slot);
    v_wander_update_path_nodes(this, ped, u32::from(nd), np.byte_add(4), np, sp);
    let dl = rd::<u8>(sp, 0);
    let al = rd::<u8>(this, 0x10);
    let (hi, lo) = if dl > al { (dl, al) } else { (al, dl) };
    let diff = (i32::from(hi) - i32::from(lo) + 8) & 7;
    if diff <= 3 {
        wr::<u8>(this, 0x10, nd);
    }
}

/// `CTaskComplexWander::UpdatePathNodes(CPed*, int8 dir, CNodeAddress* last, CNodeAddress* cur, int8* outDir)`
/// (0x669ED0, `ret 0x14`): `*last = *cur`, clears the current node's area word, then
/// `ThePaths.FindNextNodeWandering(1, ped position, last, cur, dir, outDir)` (the direction dword passed
/// on as received). `this` is not read.
#[hook(addr = 0x669ED0, conv = thiscall, name = "CTaskComplexWander::UpdatePathNodes", expect = "8B 4C 24 10 8B 01")]
pub fn wander_update_path_nodes(_this: GPtr<u8>, ped: GPtr<u8>, dir: u32, last: GPtr<u8>, cur: GPtr<u8>, out_dir: GPtr<u8>) {
    wr::<u32>(last, 0, rd::<u32>(cur, 0));
    wr::<u16>(cur, 0, 0xFFFF);
    let p = ent_pos(ped);
    path_find_next_node_wandering(GPtr::from_addr(THE_PATHS), 1, rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8), last, cur, dir, out_dir);
}

/// `CTaskComplexWander::ValidNodes()` (0x669F30, `ret`): 1 when both node addresses (+0x18 last, +0x1C
/// current) have an area word other than 0xFFFF and differ; else 0 (EAX set in full).
#[hook(addr = 0x669F30, conv = thiscall, name = "CTaskComplexWander::ValidNodes", expect = "B8 FF FF 00 00")]
pub fn wander_valid_nodes(this: GPtr<u8>) -> u32 {
    if rd::<u16>(this, 0x1C) == 0xFFFF || rd::<u16>(this, 0x18) == 0xFFFF {
        return 0;
    }
    if rd::<u16>(this, 0x1C) == rd::<u16>(this, 0x18) && rd::<u16>(this, 0x1E) == rd::<u16>(this, 0x1A) {
        return 0;
    }
    1
}

/// `CTaskComplexWander::ComputeTargetPos(CPed*, CVector* out, const CNodeAddress*)` (0x669F60, `ret 0xC`):
/// `ThePaths.TakeWidthIntoAccountForWandering(node, ped +0x20)` into a local, copied to `out`, then
/// `out.z += [0x858624]`. `this` is not read.
#[hook(addr = 0x669F60, conv = thiscall, name = "CTaskComplexWander::ComputeTargetPos", expect = "8B 44 24 04 8B 54 24 0C")]
pub fn wander_compute_target_pos(_this: GPtr<u8>, ped: GPtr<u8>, out: GPtr<u8>, node: GPtr<u8>) {
    let seed = u32::from(rd::<u16>(ped, 0x20));
    let mut l = [0u32; 3];
    let r = path_take_width_into_account_for_wandering(GPtr::from_addr(THE_PATHS), buf(&mut l), rd::<u32>(node, 0), seed);
    for o in [0, 4, 8] {
        wr::<u32>(out, o, rd::<u32>(r, o));
    }
    wr::<f32>(out, 8, f32_of(x(rd::<f32>(out, 8)).fadd_m(g::<f32>(0x858624).read())));
}

// =====================================================================================================
// CTaskComplexAvoidOtherPedWhileWandering
// =====================================================================================================

/// `IKChainManager_c` singleton (0xC15448)
const IK_CHAIN_MANAGER: usize = 0xC15448;

/// `CTaskComplexAvoidOtherPedWhileWandering::QuitIK(CPed*)` (0x66A230, `ret 4`): with bit 0 of +0x5C, aborts
/// the ped's look-at (blend 250) when it is looking.
#[hook(addr = 0x66A230, conv = thiscall, name = "CTaskComplexAvoidOtherPedWhileWandering::QuitIK", expect = "F6 41 5C 01 74 25")]
pub fn avoid_quit_ik(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u8>(this, 0x5C) & 1 == 0 {
        return;
    }
    let m = GPtr::from_addr(IK_CHAIN_MANAGER);
    if ik_is_looking(m, ped) != 0 {
        ik_abort_look_at(m, ped, 0xFA);
    }
}

/// `CTaskComplexFollowNodeRoute::SetTarget(CPed*, const CVector* target, float radius, float slowDown,
/// float followDist, bool force)` (0x671750, `ret 0x18`): unless forced, returns when 0x509760 says the
/// target (+0xC) is unchanged and the three floats (+0x1C/+0x20/+0x24) compare equal; otherwise stores
/// them (resetting the blend fields when flag 8 is set), recomputes the path nodes and the route and sets
/// flag 4.
#[hook(addr = 0x671750, conv = thiscall, name = "CTaskComplexFollowNodeRoute::SetTarget", expect = "8A 44 24 18 53")]
pub fn fnr_set_target(this: GPtr<u8>, ped: GPtr<u8>, target: GPtr<u8>, radius: u32, slow: u32, follow: u32, force: u32) {
    if force as u8 == 0
        && vectors_differ_509760(this.byte_add(0xC), target) == 0
        && x(rd::<f32>(this, 0x1C)).fcom(x(f32::from_bits(radius))) == Cmp::Equal
        && x(rd::<f32>(this, 0x20)).fcom(x(f32::from_bits(slow))) == Cmp::Equal
        && x(rd::<f32>(this, 0x24)).fcom(x(f32::from_bits(follow))) == Cmp::Equal
    {
        return;
    }
    for o in [0, 4, 8] {
        wr::<u32>(this, 0xC + o, rd::<u32>(target, o));
    }
    wr::<u32>(this, 0x1C, radius);
    let f = rd::<u32>(this, 0x4C);
    wr::<u32>(this, 0x20, slow);
    wr::<u32>(this, 0x24, follow);
    if f & 8 != 0 {
        for o in [0x50, 0x54, 0x58, 0x5C] {
            wr::<u32>(this, o, 0);
        }
        wr::<u32>(this, 0x4C, (f & 0xFFFF_FFDF) | 0x40);
    }
    fnr_compute_path_nodes_orig(this, ped);
    fnr_compute_route_orig(this);
    wr::<u32>(this, 0x4C, rd::<u32>(this, 0x4C) | 4);
}

/// `CTaskComplexFollowNodeRoute::CreateFirstSubTask(CPed*)` (0x671800, `ret 4`): starts the give-up timer
/// (+0x40 start, +0x44 duration, +0x48 set) when +0x3C >= 0, resets the blend fields on flag 8; in a car
/// → 0x2C0; otherwise forces `SetTarget` with the stored target, clears flag 4 and creates the sub-task
/// type of the (inlined) `GetSubTaskType`, through `CalcGoToTaskType` on flag 8.
#[hook(addr = 0x671800, conv = thiscall, name = "CTaskComplexFollowNodeRoute::CreateFirstSubTask", expect = "56 8B F1 8B 46 3C")]
pub fn fnr_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let d = rd::<i32>(this, 0x3C);
    if d >= 0 {
        wr::<u32>(this, 0x40, g::<u32>(0xB7CB84).read());
        wr::<i32>(this, 0x44, d);
        wr::<u8>(this, 0x48, 1);
    }
    if rd::<u8>(this, 0x4C) & 8 != 0 {
        for o in [0x50, 0x54, 0x58, 0x5C] {
            wr::<u32>(this, o, 0);
        }
    }
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        return fnr_create_sub_task_orig(this, 0x2C0, ped);
    }
    fnr_set_target_orig(this, ped, this.byte_add(0xC), rd::<u32>(this, 0x1C), rd::<u32>(this, 0x20), rd::<u32>(this, 0x24), 1);
    let f = rd::<u32>(this, 0x4C) & !4;
    let idx = rd::<i32>(this, 0x38);
    wr::<u32>(this, 0x4C, f);
    let route = rd::<GPtr<u8>>(this, 0x30);
    let mut ty = if (f >> 1) & 1 != 0 {
        let n = rd::<i32>(route, 0);
        if idx == n {
            0x516
        } else if idx.wrapping_add(1) == n {
            0x387
        } else {
            0x384
        }
    } else {
        0x384
    };
    if f & 8 != 0 {
        ty = fnr_calc_go_to_task_type(this, ped, ty);
    }
    fnr_create_sub_task_orig(this, ty, ped)
}

/// The give-up timer of the node route (+0x40 start, +0x44 duration, +0x48 set, +0x49 restart):
/// a pending restart takes the current time; true when `time >= start + duration` (unsigned).
fn fnr_timer_out(this: GPtr<u8>) -> bool {
    if rd::<u8>(this, 0x49) != 0 {
        wr::<u32>(this, 0x40, g::<u32>(0xB7CB84).read());
        wr::<u8>(this, 0x49, 0);
    }
    let end = rd::<u32>(this, 0x44).wrapping_add(rd::<u32>(this, 0x40));
    g::<u32>(0xB7CB84).read() >= end
}

/// `CTaskComplexFollowNodeRoute::CreateNextSubTask(CPed*)` (0x6718D0, `ret 4`): when the timer ran out
/// with a stand-still sub-task, teleports the ped to the target (vtable +0x38) and finishes (0x516); a
/// leave-car sub-task finishes in a car or restarts; with a built route (flag 2) advances the index
/// (+0x38, and the node +0x34 while in range) per sub-task type (0x387 / 0x384 / 0x39E; 0xCB → 0x516;
/// other types null); without it, the end of the route forces `SetTarget` and restarts, else advances
/// (node read without a bounds check) and asks `GetSubTaskType` with the shifted flags dword (low bit 0).
#[hook(addr = 0x6718D0, conv = thiscall, name = "CTaskComplexFollowNodeRoute::CreateNextSubTask", expect = "53 56 8B F1 8A 46 48")]
pub fn fnr_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = || rd::<GPtr<u8>>(this, 8);
    if rd::<u8>(this, 0x48) != 0 && fnr_timer_out(this) && v_get_task_type(sub()) == 0xCB {
        v_entity_teleport(ped, rd::<u32>(this, 0xC), rd::<u32>(this, 0x10), rd::<u32>(this, 0x14), 0);
        return fnr_create_sub_task_orig(this, 0x516, ped);
    }
    if v_get_task_type(sub()) == 0x2C0 {
        if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
            return fnr_create_sub_task_orig(this, 0x516, ped);
        }
        return v_create_first_sub_task(this, ped);
    }
    let f = rd::<u32>(this, 0x4C);
    let (mut ty, flags) = if (f >> 1) & 1 != 0 {
        let idx = match v_get_task_type(sub()) {
            0x387 => {
                let i = rd::<i32>(this, 0x38).wrapping_add(1);
                wr::<i32>(this, 0x38, i);
                i
            }
            0xCB => return fnr_create_sub_task_orig(this, 0x516, ped),
            0x384 | 0x39E => {
                let i = rd::<i32>(this, 0x38).wrapping_add(1);
                wr::<i32>(this, 0x38, i);
                let nodes = rd::<GPtr<u8>>(this, 0x2C);
                if i < rd::<i32>(nodes, 0) {
                    wr::<u32>(this, 0x34, rd::<u32>(nodes, 4 + (i as isize) * 4));
                }
                i
            }
            _ => return GPtr::NULL,
        };
        let route = rd::<GPtr<u8>>(this, 0x30);
        let f2 = rd::<u32>(this, 0x4C);
        (fnr_get_sub_task_type_orig(idx, (f2 >> 1) & 0xFFFF_FF01, route), f2)
    } else {
        let i = rd::<i32>(this, 0x38).wrapping_add(1);
        let route = rd::<GPtr<u8>>(this, 0x30);
        if i == rd::<i32>(route, 0) {
            fnr_set_target_orig(this, ped, this.byte_add(0xC), rd::<u32>(this, 0x1C), rd::<u32>(this, 0x20), rd::<u32>(this, 0x24), 1);
            return v_create_first_sub_task(this, ped);
        }
        wr::<i32>(this, 0x38, i);
        let nodes = rd::<GPtr<u8>>(this, 0x2C);
        wr::<u32>(this, 0x34, rd::<u32>(nodes, 4 + (i as isize) * 4));
        (fnr_get_sub_task_type_orig(i, (f >> 1) & 0xFFFF_FF01, route), f)
    };
    if flags & 8 != 0 {
        ty = fnr_calc_go_to_task_type(this, ped, ty);
    }
    fnr_create_sub_task_orig(this, ty, ped)
}

/// `CTaskComplexFollowNodeRoute::ControlSubTask(CPed*)` (0x671AB0, `ret 4`): a new target (flag 4)
/// restarts unless leaving a car; a run-out timer aborts the sub-task (`MakeAbortable(ped, 1, null)`) for a
/// stand-still; with flag 8 drives the speed blending like the point route (no blend-in while the
/// sub-task is a 0x387). The ratio lives in the original's ped argument slot (a local here).
#[hook(addr = 0x671AB0, conv = thiscall, name = "CTaskComplexFollowNodeRoute::ControlSubTask", expect = "56 8B F1 F6 46 4C 04")]
pub fn fnr_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = || rd::<GPtr<u8>>(this, 8);
    if rd::<u8>(this, 0x4C) & 4 != 0 && v_get_task_type(sub()) != 0x2C0 {
        return v_create_first_sub_task(this, ped);
    }
    if rd::<u8>(this, 0x48) != 0 && fnr_timer_out(this) && v_get_task_type(sub()) != 0xCB {
        if v_make_abortable(sub(), ped, 1, GPtr::NULL) != 0 {
            return fnr_create_sub_task_orig(this, 0xCB, ped);
        }
        return sub();
    }
    let f = rd::<u32>(this, 0x4C);
    if f & 8 == 0 {
        return sub();
    }
    if f & 0x20 != 0 && v_get_task_type(sub()) == 0x39E {
        let r = X87::from_f64_exact(fnr_calc_blend_ratio(this, ped, 1));
        let mut ratio = f32_of(r);
        if lt(r, x(g::<f32>(0x858B50).read())) {
            ratio = f32_of(X87::from_f64_exact(go_to_point_fine_base_ratio(rd::<u32>(this, 0x18))));
        }
        go_to_point_fine_set_move_ratio(sub(), ratio.to_bits());
    } else if rd::<u8>(this, 0x4C) & 0x40 != 0 && v_get_task_type(sub()) == 0x39E {
        let r = X87::from_f64_exact(fnr_calc_blend_ratio(this, ped, 0));
        let ratio = f32_of(r);
        if r.fcom(x(g::<f32>(0x858B50).read())).c0() {
            wr::<u32>(this, 0x4C, rd::<u32>(this, 0x4C) & 0xFFFF_FFBF);
            let b = f32_of(X87::from_f64_exact(go_to_point_fine_base_ratio(rd::<u32>(this, 0x18))));
            go_to_point_fine_set_move_ratio(sub(), b.to_bits());
            return fnr_create_sub_task_orig(this, 0x384, ped);
        }
        go_to_point_fine_set_move_ratio(sub(), ratio.to_bits());
    }
    // 0x671BA8
    let f = rd::<u32>(this, 0x4C);
    if f & 0x10 == 0 || f & 0x20 != 0 || v_get_task_type(sub()) == 0x387 {
        return sub();
    }
    let r = X87::from_f64_exact(fnr_calc_blend_ratio(this, ped, 1));
    let ratio = f32_of(r);
    if r.fcom(x(g::<f32>(0x858B50).read())).c0() {
        return sub();
    }
    wr::<u32>(this, 0x4C, (rd::<u32>(this, 0x4C) & 0xFFFF_FFAF) | 0x20);
    if v_get_task_type(sub()) == 0x39E {
        go_to_point_fine_set_move_ratio(sub(), ratio.to_bits());
        return sub();
    }
    let t = fnr_create_sub_task_orig(this, 0x39E, ped);
    go_to_point_fine_set_move_ratio(t, ratio.to_bits());
    t
}

/// `new` + `CTaskSimpleScratchHead` (0x1A5) through the 0x48DF30 stub; null when the pool is full.
fn new_scratch_head() -> GPtr<u8> {
    let t = task_new(0x20);
    if t.is_null() { t } else { scratch_head_ctor_48df30(t) }
}

/// `CTaskComplexWander::CreateSubTask(CPed*, eTaskType)` (0x671CB0, `ret 8`; the ped comes first): 0x2C0
/// leave car, 0xE1 observe traffic lights / 0xE3 cross-road look (2000 ms, `ComputeTargetHeading` after
/// the allocation), 0x1A5 scratch head, 0x2CF timed car drive (2000 ms), 0x384 go to the node target
/// (`ComputeTargetPos` into a local before the allocation, move state +0xC, radius +0x14); else null.
#[hook(addr = 0x671CB0, conv = thiscall, name = "CTaskComplexWander::CreateSubTask", expect = "64 A1 00 00 00 00")]
pub fn wander_create_sub_task(this: GPtr<u8>, ped: GPtr<u8>, ty: u32) -> GPtr<u8> {
    match ty {
        0x2C0 => {
            let t = task_new(0x34);
            if t.is_null() {
                return t;
            }
            leave_car_ctor(t, rd::<u32>(ped, 0x58C), 0, 0, 1, 0)
        }
        0xE1 | 0xE3 => {
            let t = task_new(0x14);
            if t.is_null() {
                return t;
            }
            let h = f32_of(X87::from_f64_exact(wander_compute_target_heading(this, ped)));
            if ty == 0xE1 {
                observe_traffic_lights_heading_ctor(t, 0x7D0, h.to_bits())
            } else {
                cross_road_look_ctor(t, 0x7D0, h.to_bits())
            }
        }
        0x1A5 => new_scratch_head(),
        0x2CF => {
            let t = task_new(0x70);
            if t.is_null() {
                return t;
            }
            car_drive_timed_ctor(t, rd::<u32>(ped, 0x58C), 0x7D0)
        }
        0x384 => {
            let mut l = [0u32; 3];
            let lp = buf(&mut l);
            wander_compute_target_pos_orig(this, ped, lp, this.byte_add(0x1C));
            let t = task_new(0x30);
            if t.is_null() {
                return t;
            }
            go_to_point_ctor(t, rd::<u32>(this, 0xC), lp, rd::<u32>(this, 0x14), 0, 0)
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexWander::CreateFirstSubTask(CPed*)` (0x6740E0, `ret 4`): in a car → 0x2C0; else the
/// virtual `UpdateDir` and `UpdatePathNodes(ped, dir, &last, &cur, &dir)` (+0x34 / +0x38), then 0x384.
#[hook(addr = 0x6740E0, conv = thiscall, name = "CTaskComplexWander::CreateFirstSubTask", expect = "56 57 8B 7C 24 0C")]
pub fn wander_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        return wander_create_sub_task_orig(this, ped, 0x2C0);
    }
    v_wander_update_dir(this, ped);
    let d = u32::from(rd::<u8>(this, 0x10));
    v_wander_update_path_nodes(this, ped, d, this.byte_add(0x18), this.byte_add(0x1C), this.byte_add(0x10));
    wander_create_sub_task_orig(this, ped, 0x384)
}

/// `CTaskComplexWander::CreateNextSubTask(CPed*)` (0x674140, `ret 4`; re-read from the bytes, the spec
/// row is medium): one `GetTaskType` call, then: 0xE1/0xE3 → 0x384; 0x1A5 turns the direction (+0x10)
/// by one, asks `UpdatePathNodes` and walks on (0x384) with valid nodes (inlined `ValidNodes`), else
/// scratches the head again; 0x2C0 → timed car drive in a car, else `CreateFirstSubTask`; 0x2CF →
/// `CreateFirstSubTask`; 0x384 / 0xF4: a pending flag 8 (+0x24) is cleared for a scratch head; else
/// `UpdateDir`, `UpdatePathNodes`, and a new current node equal to the old last node (dead end) builds a
/// `CTaskComplexSequence` (stand still 500 ms, run anim 4 of the ped's anim group, scratch head, go to the
/// node target; `AddTask` also with null tasks, the sequence pointer is not checked), invalid nodes
/// scratch the head, a ped traffic light (flag 1) → 0xE1, a road crossing (flag 1 re-read) → cross-road
/// look (`ComputeTargetHeading` after the allocation), else 0x384; other types null. The epilogue passes
/// through the island at 0x407E8D (the SEH frame restore).
#[hook(addr = 0x674140, conv = thiscall, name = "CTaskComplexWander::CreateNextSubTask", expect = "6A FF 64 A1 00 00 00 00")]
pub fn wander_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    match v_get_task_type(rd::<GPtr<u8>>(this, 8)) {
        0xE1 | 0xE3 => wander_create_sub_task_orig(this, ped, 0x384),
        0x1A5 => {
            let d = rd::<u8>(this, 0x10).wrapping_add(1);
            wr::<u8>(this, 0x10, d);
            let d = u32::from(rd::<u8>(this, 0x10));
            v_wander_update_path_nodes(this, ped, d, this.byte_add(0x18), this.byte_add(0x1C), this.byte_add(0x10));
            let valid = rd::<u16>(this, 0x1C) != 0xFFFF
                && rd::<u16>(this, 0x18) != 0xFFFF
                && (rd::<u16>(this, 0x1C) != rd::<u16>(this, 0x18) || rd::<u16>(this, 0x1E) != rd::<u16>(this, 0x1A));
            if valid {
                wander_create_sub_task_orig(this, ped, 0x384)
            } else {
                new_scratch_head()
            }
        }
        0x2C0 => {
            if rd::<u32>(ped, 0x46C) & 0x100 == 0 {
                return v_create_first_sub_task(this, ped);
            }
            let t = task_new(0x70);
            if t.is_null() {
                return t;
            }
            car_drive_timed_ctor(t, rd::<u32>(ped, 0x58C), 0x7D0)
        }
        0x2CF => v_create_first_sub_task(this, ped),
        0x384 | 0xF4 => wander_next_walk(this, ped),
        _ => GPtr::NULL,
    }
}

/// The 0x384 / 0xF4 branch of `CTaskComplexWander::CreateNextSubTask` (0x67425A..).
fn wander_next_walk(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let fl = rd::<u8>(this, 0x24);
    if fl & 8 != 0 {
        wr::<u8>(this, 0x24, fl & 0xF7);
        return new_scratch_head();
    }
    v_wander_update_dir(this, ped);
    let old_last = rd::<u32>(this, 0x18);
    let d = u32::from(rd::<u8>(this, 0x10));
    v_wander_update_path_nodes(this, ped, d, this.byte_add(0x18), this.byte_add(0x1C), this.byte_add(0x10));
    if rd::<u16>(this, 0x1C) == old_last as u16 && rd::<u16>(this, 0x1E) == (old_last >> 16) as u16 {
        let s = task_new(0x40);
        let seq = if s.is_null() { s } else { sequence_ctor(s) };
        let t = task_new(0x20);
        let t = if t.is_null() { t } else { stand_still_ctor(t, 0x1F4, 0, 0, 0x41000000) };
        sequence_add_task(seq, t);
        let t = task_new(0x20);
        let t = if t.is_null() { t } else { run_anim_ctor(t, rd::<u32>(ped, 0x4D4), 4, 0x40800000, 0) };
        sequence_add_task(seq, t);
        sequence_add_task(seq, new_scratch_head());
        let mut l = [0u32; 3];
        let lp = buf(&mut l);
        wander_compute_target_pos_orig(this, ped, lp, this.byte_add(0x1C));
        let t = task_new(0x30);
        let t = if t.is_null() { t } else { go_to_point_ctor(t, rd::<u32>(this, 0xC), lp, rd::<u32>(this, 0x14), 0, 0) };
        sequence_add_task(seq, t);
        return seq;
    }
    if wander_valid_nodes_orig(this) as u8 == 0 {
        return new_scratch_head();
    }
    let paths = GPtr::from_addr(THE_PATHS);
    if rd::<u8>(this, 0x24) & 1 != 0 {
        if path_test_for_ped_traffic_light(paths, rd::<u32>(this, 0x18), rd::<u32>(this, 0x1C)) != 0 {
            return wander_create_sub_task_orig(this, ped, 0xE1);
        }
        if rd::<u8>(this, 0x24) & 1 != 0 && path_test_crosses_road(paths, rd::<u32>(this, 0x18), rd::<u32>(this, 0x1C)) != 0 {
            let t = task_new(0x14);
            if t.is_null() {
                return t;
            }
            let h = f32_of(X87::from_f64_exact(wander_compute_target_heading(this, ped)));
            return cross_road_look_ctor(t, 0x7D0, h.to_bits());
        }
    }
    wander_create_sub_task_orig(this, ped, 0x384)
}

/// `CTaskComplexWander::ScanForBlockedNodes(CPed*)` (0x674560, `ret 4`): while walking to a node (0x384)
/// whose current node is blocked (`ScanForBlockedNode-`), aborts the sub-task (`MakeAbortable(ped, 0,
/// null)`, result unused), asks `UpdatePathNodes` for the next node into locals (current = the original's
/// ped argument slot; the direction is pushed as a dword whose upper bytes are a stale stack slot, passed
/// zero-extended here) and sets flag 8 (+0x24) when that node is blocked too or equals the last node
/// (0x420920). EAX is not a result.
#[hook(addr = 0x674560, conv = thiscall, name = "CTaskComplexWander::ScanForBlockedNodes", expect = "83 EC 08 56 8B F1")]
pub fn wander_scan_for_blocked_nodes(this: GPtr<u8>, ped: GPtr<u8>) {
    if v_get_task_type(rd::<GPtr<u8>>(this, 8)) != 0x384 || rd::<u16>(this, 0x1C) == 0xFFFF {
        return;
    }
    if wander_scan_for_blocked_node_orig(this, ped, this.byte_add(0x1C)) == 0 {
        return;
    }
    v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 0, GPtr::NULL);
    // [0] = direction byte ([esp+0x10]), [1] = last node, [2] = current node (the ped argument slot)
    let mut l = [0u32; 3];
    let lp = buf(&mut l);
    let cur = rd::<u32>(this, 0x1C);
    let d = rd::<u8>(this, 0x10);
    let last = rd::<u32>(this, 0x18);
    wr::<u32>(lp, 8, cur);
    wr::<u8>(lp, 0, d);
    wr::<u32>(lp, 4, last);
    v_wander_update_path_nodes(this, ped, u32::from(d), lp.byte_add(4), lp.byte_add(8), lp);
    if wander_scan_for_blocked_node_orig(this, ped, lp.byte_add(8)) != 0 || node_address_eq_420920(lp.byte_add(8), this.byte_add(0x18)) as u8 != 0 {
        wr::<u8>(this, 0x24, rd::<u8>(this, 0x24) | 8);
    }
}

/// `CTaskComplexWander::ScanForBlockedNode-(CPed*, const CNodeAddress*)` (0x671EF0, `ret 8`): when the
/// node's wander position (`TakeWidthIntoAccountForWandering`, z + `[0x858624]`) is within `[0x86FCBC]`
/// of the ped in 2D (`K*K < d*d` → false; unordered continues), true when the closest ped (intelligence
/// +0x124) or the closest entity of the scanner at +0xD4 blocks it (`ScanForBlockedNode-1`).
#[hook(addr = 0x671EF0, conv = thiscall, name = "CTaskComplexWander::ScanForBlockedNode-", expect = "83 EC 18 56 8B 74 24 20")]
pub fn wander_scan_for_blocked_node(this: GPtr<u8>, ped: GPtr<u8>, node: GPtr<u8>) -> u8 {
    let seed = u32::from(rd::<u16>(ped, 0x20));
    let mut w = [0u32; 3];
    let r = path_take_width_into_account_for_wandering(GPtr::from_addr(THE_PATHS), buf(&mut w), rd::<u32>(node, 0), seed);
    let mut v = [0u32; 3];
    let vp = buf(&mut v);
    wr::<u32>(vp, 0, rd::<u32>(r, 0));
    wr::<u32>(vp, 4, rd::<u32>(r, 4));
    wr::<f32>(vp, 8, f32_of(x(rd::<f32>(r, 8)).fadd_m(g::<f32>(0x858624).read())));
    let p = ent_pos(ped);
    let dx = x(rd::<f32>(vp, 0)).fsub_m(rd::<f32>(p, 0));
    let dy = x(rd::<f32>(vp, 4)).fsub_m(rd::<f32>(p, 4));
    let d2 = dy.fmul(dy).fadd(dx.fmul(dx));
    let k = g::<f32>(0x86FCBC).read();
    if lt(x(k).fmul_m(k), d2) {
        return 0;
    }
    let e = ped_scanner_get_closest_ped_in_range(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x124));
    if wander_scan_for_blocked_node_1_orig(this, vp, e) as u8 != 0 {
        return 1;
    }
    let e = scanner_get_closest_5ffee0(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0xD4));
    if wander_scan_for_blocked_node_1_orig(this, vp, e) as u8 != 0 {
        return 1;
    }
    0
}

/// `CTaskComplexWander::ScanForBlockedNode-1(const CVector* pos, CEntity*)` (0x66F4C0, `ret 8`): 0 for a
/// null entity; else 1 when the 2D distance from the entity to `pos` is below its collision bounding
/// radius (model info `[0xA9B0C8 + 4 * model]` +0x14 → +0x24) + `[0x858624]` (`r*r > d*d`). EAX is 0/1 in
/// full on every path (the null path's `xor al, al` leaves the null pointer's upper bytes). `this` is not
/// read.
#[hook(addr = 0x66F4C0, conv = thiscall, name = "CTaskComplexWander::ScanForBlockedNode-1", expect = "8B 44 24 08 85 C0")]
pub fn wander_scan_for_blocked_node_1(_this: GPtr<u8>, pos: GPtr<u8>, e: GPtr<u8>) -> u32 {
    if e.is_null() {
        return 0;
    }
    let mi = g::<GPtr<u8>>(0xA9B0C8u32.wrapping_add((i32::from(rd::<i16>(e, 0x22)) as u32).wrapping_mul(4)) as usize).read();
    let r = x(rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x24)).fadd_m(g::<f32>(0x858624).read());
    let ep = ent_pos(e);
    let a = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(ep, 0));
    let b = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(ep, 4));
    let d2 = a.fmul(a).fadd(b.fmul(b));
    u32::from(gt(r.fmul(r), d2))
}

/// `CTaskComplexWander::ControlSubTask(CPed*)` (0x674C30, `ret 4`): keeps a leave-car / timed-drive
/// sub-task (two `GetTaskType` calls); flag 2 (+0x24) with a 0x384 sub-task clears the nodes and restarts;
/// flag 4 with a 0x384 sub-task re-targets it (`ComputeTargetPos`, `UpdatePoint` radius `[0x86FC78]`),
/// else `ScanForBlockedNodes` and the virtual `ScanForStuff` (+0x30); a 0x384 sub-task gets the move
/// state (+0xC → sub +8) and a walking ped (state <= 4) says 0x2D; above the intelligence limit
/// (`[0x86C938]`) an abortable sub-task becomes a scratch head. The returned sub-task is the one read
/// before the scans (the original keeps it in EBP).
#[hook(addr = 0x674C30, conv = thiscall, name = "CTaskComplexWander::ControlSubTask", expect = "83 EC 0C 55 56")]
pub fn wander_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = || rd::<GPtr<u8>>(this, 8);
    if v_get_task_type(sub()) == 0x2C0 || v_get_task_type(sub()) == 0x2CF {
        return sub();
    }
    let mut ret = sub();
    if rd::<u8>(this, 0x24) & 2 != 0 && v_get_task_type(ret) == 0x384 {
        let f = rd::<u8>(this, 0x24);
        wr::<u16>(this, 0x1C, 0xFFFF);
        wr::<u16>(this, 0x18, 0xFFFF);
        wr::<u8>(this, 0x24, f & 0xFD);
        return v_create_first_sub_task(this, ped);
    }
    if rd::<u8>(this, 0x24) & 4 != 0 && v_get_task_type(sub()) == 0x384 {
        wr::<u8>(this, 0x24, rd::<u8>(this, 0x24) & 0xFB);
        let mut l = [0u32; 3];
        let lp = buf(&mut l);
        wander_compute_target_pos_orig(this, ped, lp, this.byte_add(0x1C));
        go_to_point_update_point(sub(), lp, g::<u32>(0x86FC78).read(), 0);
        ret = sub();
    } else {
        wander_scan_for_blocked_nodes_orig(this, ped);
        v_wander_scan_for_stuff(this, ped);
    }
    if v_get_task_type(sub()) == 0x384 {
        wr::<u32>(sub(), 8, rd::<u32>(this, 0xC));
        if rd::<i32>(this, 0xC) <= 4 {
            ped_say(ped, 0x2D, 0, 0x3DCCCCCD, 0, 0, 0);
        }
    }
    if rd::<i32>(rd::<GPtr<u8>>(ped, 0x47C), 0x274) > g::<i32>(0x86C938).read() && v_make_abortable(sub(), ped, 1, GPtr::NULL) != 0 {
        ret = wander_create_sub_task_orig(this, ped, 0x1A5);
    }
    ret
}

// =====================================================================================================
// CTaskComplexWanderCop
// =====================================================================================================

/// `CTaskComplexWanderCop::ShouldPursuePlayer(CPed*)` (0x66B160, `ret 4`): 1 when the player's wanted
/// level (+0x2C) is positive, the sub-task is not a police pursuit (0x44F; null sub-task allowed) and
/// `CWanted::CanCopJoinPursuit(ped)` (`FindPlayerWanted` called again); else 0 (EAX in full).
#[hook(addr = 0x66B160, conv = thiscall, name = "CTaskComplexWanderCop::ShouldPursuePlayer", expect = "56 6A FF 8B F1")]
pub fn cop_should_pursue_player(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    if rd::<i32>(find_player_wanted(-1), 0x2C) <= 0 {
        return 0;
    }
    let sub = rd::<GPtr<u8>>(this, 8);
    if !sub.is_null() && v_get_task_type(sub) == 0x44F {
        return 0;
    }
    u32::from(wanted_can_cop_join_pursuit(find_player_wanted(-1), ped) != 0)
}

/// `CTaskComplexWanderCop::LookForCarAlarms(CPed*)` (0x66B1B0, `ret 4`): when the player's vehicle (asked
/// four times, `FindPlayerVehicle(-1, false)`) is an automobile (+0x590 == 0) with a sounding alarm
/// (+0x45C not 0 / 0xFFFF), a status (+0x36 & 0xF8) other than 0x28, and within `[0x86FCEC]` of the cop
/// (`K*K > d*d`, 3D), sets the player's wanted level to at least 1.
#[hook(addr = 0x66B1B0, conv = thiscall, name = "CTaskComplexWanderCop::LookForCarAlarms", expect = "6A 00 6A FF E8 17 2F F0 FF")]
pub fn cop_look_for_car_alarms(_this: GPtr<u8>, ped: GPtr<u8>) {
    if find_player_vehicle(-1, 0).is_null() {
        return;
    }
    if rd::<u32>(find_player_vehicle(-1, 0), 0x590) != 0 {
        return;
    }
    let v = find_player_vehicle(-1, 0);
    let alarm = rd::<u16>(v, 0x45C);
    if alarm == 0 || alarm == 0xFFFF || rd::<u8>(v, 0x36) & 0xF8 == 0x28 {
        return;
    }
    let pp = ent_pos(ped);
    let vp = ent_pos(find_player_vehicle(-1, 0));
    let dx = x(rd::<f32>(vp, 0)).fsub_m(rd::<f32>(pp, 0));
    let dy = x(rd::<f32>(vp, 4)).fsub_m(rd::<f32>(pp, 4));
    let dz = x(rd::<f32>(vp, 8)).fsub_m(rd::<f32>(pp, 8));
    let d2 = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
    let k = g::<f32>(0x86FCEC).read();
    if !gt(x(k).fmul_m(k), d2) {
        return;
    }
    player_set_wanted_level_no_drop(find_player_ped(-1), 1);
}

/// `CTaskComplexWanderCop::LookForStolenCopCars(CPed*)` (0x66B290, `ret 4`): when the player (asked four
/// times) has no wanted level (wanted read through +0x480 → [0] without a check of the wanted pointer),
/// sits in a vehicle (+0x58C) whose byte +0x428 has bit 0 (a law-enforcement vehicle), sets the wanted
/// level to at least 1. Neither `this` nor the ped is read.
#[hook(addr = 0x66B290, conv = thiscall, name = "CTaskComplexWanderCop::LookForStolenCopCars", expect = "6A FF E8 79 2F F0 FF")]
pub fn cop_look_for_stolen_cop_cars(_this: GPtr<u8>, _ped: GPtr<u8>) {
    let pd = rd::<GPtr<u8>>(find_player_ped(-1), 0x480);
    let w = if pd.is_null() { pd } else { rd::<GPtr<u8>>(pd, 0) };
    if rd::<u32>(w, 0x2C) != 0 {
        return;
    }
    if rd::<u32>(find_player_ped(-1), 0x58C) == 0 {
        return;
    }
    if rd::<u8>(rd::<GPtr<u8>>(find_player_ped(-1), 0x58C), 0x428) & 1 == 0 {
        return;
    }
    player_set_wanted_level_no_drop(find_player_ped(-1), 1);
}

/// `CTaskComplexWanderCop::ScanForStuff(CPed*)` (0x6702B0, `ret 4`): a 50 ms scan timer (+0x2C start,
/// +0x30 duration, +0x34 set, +0x35 restart); when it runs out (unsigned) and the sub-task is not a police
/// pursuit, restarts it and, by the times at +0x3C / +0x38 (unsigned, the clock read again for each),
/// looks for car alarms and stolen cop cars, then for criminals.
#[hook(addr = 0x6702B0, conv = thiscall, name = "CTaskComplexWanderCop::ScanForStuff", expect = "56 8B F1 8A 46 34")]
pub fn cop_scan_for_stuff(this: GPtr<u8>, ped: GPtr<u8>) {
    let now = || g::<u32>(0xB7CB84).read();
    if rd::<u8>(this, 0x34) == 0 {
        wr::<u32>(this, 0x2C, now());
        wr::<u32>(this, 0x30, 0x32);
        wr::<u8>(this, 0x34, 1);
    }
    if v_get_task_type(rd::<GPtr<u8>>(this, 8)) == 0x44F || rd::<u8>(this, 0x34) == 0 {
        return;
    }
    if rd::<u8>(this, 0x35) != 0 {
        wr::<u32>(this, 0x2C, now());
        wr::<u8>(this, 0x35, 0);
    }
    let end = rd::<u32>(this, 0x30).wrapping_add(rd::<u32>(this, 0x2C));
    let t = now();
    if t < end {
        return;
    }
    wr::<u32>(this, 0x30, 0x32);
    wr::<u32>(this, 0x2C, t);
    wr::<u8>(this, 0x34, 1);
    if now() >= rd::<u32>(this, 0x3C) {
        cop_look_for_car_alarms_orig(this, ped);
        cop_look_for_stolen_cop_cars_orig(this, ped);
    }
    if now() >= rd::<u32>(this, 0x38) {
        cop_look_for_criminals_orig(this, ped);
    }
}

/// `new` + `CTaskComplexPolicePursuit` (0x18 bytes); null when the pool is full.
fn new_police_pursuit() -> GPtr<u8> {
    let t = task_new(0x18);
    if t.is_null() { t } else { police_pursuit_ctor(t) }
}

/// `CTaskComplexWanderCop::CreateFirstSubTask(CPed*)` (0x674750, `ret 4`): for a cop (ped +0x598 == 6):
/// a flagged cop (+0x79C) stands still (100000 ms, looped); a pursuing one gets a police pursuit; a stored
/// task (+0x28) is cloned; otherwise (and for other peds) `CTaskComplexWander::CreateFirstSubTask`.
#[hook(addr = 0x674750, conv = thiscall, name = "CTaskComplexWanderCop::CreateFirstSubTask", expect = "64 A1 00 00 00 00")]
pub fn cop_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<i32>(ped, 0x598) == 6 {
        if rd::<u8>(ped, 0x79C) != 0 {
            let t = task_new(0x20);
            if t.is_null() {
                return t;
            }
            return stand_still_ctor(t, 0x186A0, 1, 0, 0x41000000);
        }
        if cop_should_pursue_player_orig(this, ped) != 0 {
            return new_police_pursuit();
        }
        let s = rd::<GPtr<u8>>(this, 0x28);
        if !s.is_null() {
            return v_clone(s);
        }
    }
    wander_create_first_sub_task_orig(this, ped)
}

/// `CTaskComplexWanderCop::CreateNextSubTask(CPed*)` (0x674860, `ret 4`): other peds → the wander version;
/// a flagged cop (+0x79C) → null; after a police pursuit (0x44F) starts a 3000 ms timer (+0x44/+0x48/+0x4C)
/// and resumes the stored task (+0x28, cloned) or the first wander sub-task; a sub-task of the stored
/// task's type → null (two `GetTaskType` calls, sub-task first); else the stored task cloned or the next
/// wander sub-task.
#[hook(addr = 0x674860, conv = thiscall, name = "CTaskComplexWanderCop::CreateNextSubTask", expect = "53 8B 5C 24 08")]
pub fn cop_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<i32>(ped, 0x598) != 6 {
        return wander_create_next_sub_task_orig(this, ped);
    }
    if rd::<u8>(ped, 0x79C) != 0 {
        return GPtr::NULL;
    }
    if v_get_task_type(rd::<GPtr<u8>>(this, 8)) == 0x44F {
        wr::<u32>(this, 0x44, g::<u32>(0xB7CB84).read());
        wr::<u32>(this, 0x48, 0xBB8);
        wr::<u8>(this, 0x4C, 1);
        let s = rd::<GPtr<u8>>(this, 0x28);
        if !s.is_null() {
            return v_clone(s);
        }
        return wander_create_first_sub_task_orig(this, ped);
    }
    let s = rd::<GPtr<u8>>(this, 0x28);
    if !s.is_null() {
        let a = v_get_task_type(rd::<GPtr<u8>>(this, 8));
        if a == v_get_task_type(s) {
            return GPtr::NULL;
        }
    }
    let s = rd::<GPtr<u8>>(this, 0x28);
    if !s.is_null() {
        return v_clone(s);
    }
    wander_create_next_sub_task_orig(this, ped)
}

/// `CTaskComplexWanderCop::ControlSubTask(CPed*)` (0x674D80, `ret 4`): other peds → the wander version; a
/// cop that should pursue keeps the sub-task while the timer (+0x44, set flag +0x4C) runs, else aborts it
/// (`MakeAbortable(ped, 1, null)`) for a police pursuit; a pursuit sub-task is kept (re-read); with a
/// stored task (+0x28) the sub-task read at entry is returned; else the wander version.
#[hook(addr = 0x674D80, conv = thiscall, name = "CTaskComplexWanderCop::ControlSubTask", expect = "64 A1 00 00 00 00")]
pub fn cop_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub0 = rd::<GPtr<u8>>(this, 8);
    if rd::<i32>(ped, 0x598) != 6 {
        return wander_control_sub_task_orig(this, ped);
    }
    if cop_should_pursue_player_orig(this, ped) != 0 {
        if rd::<u8>(this, 0x4C) != 0 && task_timer_is_out_of_time(this.byte_add(0x44)) == 0 {
            return sub0;
        }
        if v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, GPtr::NULL) == 0 {
            return sub0;
        }
        return new_police_pursuit();
    }
    if v_get_task_type(rd::<GPtr<u8>>(this, 8)) == 0x44F {
        return rd::<GPtr<u8>>(this, 8);
    }
    if !rd::<GPtr<u8>>(this, 0x28).is_null() {
        return sub0;
    }
    wander_control_sub_task_orig(this, ped)
}

/// Field offsets of the shared corner-slowing code of the two routes' `CalcGoToTaskType`.
struct BlendLayout {
    ms: isize,
    route: isize,
    idx: isize,
    flags: isize,
    /// speed-decrease distance; +4 increase distance, +8 decrease amount, +0xC its copy
    blend: isize,
    last: fn(GPtr<u8>, GPtr<u8>, GPtr<u8>) -> GPtr<u8>,
    next: fn(GPtr<u8>, GPtr<u8>, GPtr<u8>) -> GPtr<u8>,
}

/// The shared body (0x66E157.. / 0x66EBF5..): copies the decrease amount to its copy, clears flag 0x10;
/// for a move state above 4 and a valid index (clamped to the last point; a negative one returns `None` =
/// the caller returns the type unchanged), compares the 2D turn at the current point (normalised last →
/// point and point → next) with `[0x863C1C]`: a sharper turn sets the blend distances (5.0 for state 7, else
/// 4.0), the amount `clamp(1 - (dot + 1) * [0x8700A4], 0, 1) * [0x858B3C] / [0x858CE8]` and flag 0x10,
/// otherwise clears flag 0x10.
fn route_corner_blend(this: GPtr<u8>, ped: GPtr<u8>, l: &BlendLayout) -> Option<()> {
    wr::<u32>(this, l.blend + 0xC, rd::<u32>(this, l.blend + 8));
    wr::<u32>(this, l.flags, rd::<u32>(this, l.flags) & 0xFFFF_FFEF);
    if rd::<i32>(this, l.ms) <= 4 {
        return Some(());
    }
    let route = rd::<GPtr<u8>>(this, l.route);
    let n = rd::<i32>(route, 0);
    let mut i = rd::<i32>(this, l.idx);
    if i >= n {
        i = n.wrapping_sub(1);
    }
    if i < 0 {
        return None;
    }
    let off = (i as isize) * 12;
    // [0..3] = v1, [3..6] = v2, [6..9] = waypoint out
    let mut v = [0u32; 9];
    let vp = buf(&mut v);
    let w = (l.last)(this, vp.byte_add(0x18), ped);
    let p = route.byte_add(off + 4);
    let dx = f32_of(x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(w, 0)));
    let dy = f32_of(x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(w, 4)));
    wr::<f32>(vp, 0, dx);
    wr::<f32>(vp, 4, dy);
    wr::<u32>(vp, 8, 0);
    vector_normalise(vp);
    let p = rd::<GPtr<u8>>(this, l.route).byte_add(off + 4);
    let w = (l.next)(this, vp.byte_add(0x18), ped);
    let ny = x(rd::<f32>(w, 4)).fsub_m(rd::<f32>(p, 4));
    let nx = x(rd::<f32>(w, 0)).fsub_m(rd::<f32>(p, 0));
    wr::<u32>(vp, 0x14, 0);
    wr::<f32>(vp, 0xC, f32_of(nx));
    wr::<f32>(vp, 0x10, f32_of(ny));
    vector_normalise(vp.byte_add(0xC));
    let f = |o: isize| rd::<f32>(vp, o);
    let dot = x(f(4)).fmul_m(f(0x10)).fadd(x(f(0)).fmul_m(f(0xC))).fadd(x(f(0x14)).fmul_m(f(8)));
    let one = g::<f32>(0x858624).read();
    let zero = g::<f32>(0x858B50).read();
    if dot.fcom(x(g::<f32>(0x863C1C).read())) != Cmp::Less {
        wr::<u32>(this, l.flags, rd::<u32>(this, l.flags) & 0xFFFF_FFEF);
        return Some(());
    }
    let mut s = dot.fadd_m(one).fmul_m(g::<f32>(0x8700A4).read()).fsubr_m(one);
    if s.fcom(x(zero)) == Cmp::Less {
        s = x(zero);
    } else if s.fcom(x(one)) == Cmp::Greater {
        s = x(one);
    }
    let (k, d) = if rd::<i32>(this, l.ms) == 7 { (0x858B3C, 0x40A0_0000u32) } else { (0x858CE8, 0x4080_0000u32) };
    let s = s.fmul_m(g::<f32>(k).read());
    wr::<u32>(this, l.blend, d);
    wr::<u32>(this, l.blend + 4, d);
    let fl = rd::<u32>(this, l.flags);
    wr::<f32>(this, l.blend + 8, f32_of(s));
    wr::<u32>(this, l.flags, fl | 0x10);
    Some(())
}

/// `CTaskComplexFollowPointRoute::CalcGoToTaskType(CPed*, eTaskType)` (0x66E150, `ret 8`): the corner
/// blending above (move state +0x14, route +0x20, index +0x24, flags +0x28, blend fields +0x2C..+0x38);
/// then flag 0x20 with a 0x384 / 0x387 type switches to 0x39E (flags 0x20 → 0x40), else clears 0x60. A
/// negative clamped index returns the type before any of that.
#[hook(addr = 0x66E150, conv = thiscall, name = "CTaskComplexFollowPointRoute::CalcGoToTaskType", expect = "83 EC 24 53 56 8B F1")]
pub fn fpr_calc_go_to_task_type_port(this: GPtr<u8>, ped: GPtr<u8>, ty: u32) -> u32 {
    let l = BlendLayout { ms: 0x14, route: 0x20, idx: 0x24, flags: 0x28, blend: 0x2C, last: fpr_get_last_waypoint, next: fpr_get_next_waypoint };
    if route_corner_blend(this, ped, &l).is_none() {
        return ty;
    }
    let f = rd::<u32>(this, 0x28);
    if f & 0x20 != 0 && (ty == 0x384 || ty == 0x387) {
        wr::<u32>(this, 0x28, (f & 0xFFFF_FFDF) | 0x40);
        return 0x39E;
    }
    wr::<u32>(this, 0x28, f & 0xFFFF_FF9F);
    ty
}

/// `CTaskComplexFollowNodeRoute::CalcGoToTaskType(CPed*, eTaskType)` (0x66EBE0, `ret 8`): only for 0x384
/// (other types are returned untouched): the corner blending (move state +0x18, route +0x30, index +0x38,
/// flags +0x4C, blend fields +0x50..+0x5C), then flag 0x20 switches to 0x39E (0x20 → 0x40), else clears
/// 0x60.
#[hook(addr = 0x66EBE0, conv = thiscall, name = "CTaskComplexFollowNodeRoute::CalcGoToTaskType", expect = "8B 44 24 08 83 EC 24")]
pub fn fnr_calc_go_to_task_type_port(this: GPtr<u8>, ped: GPtr<u8>, ty: u32) -> u32 {
    if ty != 0x384 {
        return ty;
    }
    let l = BlendLayout { ms: 0x18, route: 0x30, idx: 0x38, flags: 0x4C, blend: 0x50, last: fnr_get_last_waypoint, next: fnr_get_next_waypoint };
    if route_corner_blend(this, ped, &l).is_none() {
        return 0x384;
    }
    let f = rd::<u32>(this, 0x4C);
    if f & 0x20 != 0 {
        wr::<u32>(this, 0x4C, (f & 0xFFFF_FFDF) | 0x40);
        return 0x39E;
    }
    wr::<u32>(this, 0x4C, f & 0xFFFF_FF9F);
    ty
}

/// `CTaskComplexFollowNodeRoute::CanGoStraightThere(CPed*, const CVector* from, const CVector* to, float
/// maxDist)` (0x66EF20, `ret 0x10`): false (AL) when the 3D distance exceeds `maxDist`; true (AL) for a
/// ped with bit 0x8000 of +0x478; else `IsWanderPathClear(from, to, +0x24, 4) == 4` (EAX 0/1).
#[hook(addr = 0x66EF20, conv = thiscall, name = "CTaskComplexFollowNodeRoute::CanGoStraightThere", expect = "8B 54 24 08 D9 02")]
pub fn fnr_can_go_straight_there(this: GPtr<u8>, ped: GPtr<u8>, from: GPtr<u8>, to: GPtr<u8>, max: u32) -> u8 {
    let a = x(rd::<f32>(from, 0)).fsub_m(rd::<f32>(to, 0));
    let b = x(rd::<f32>(from, 4)).fsub_m(rd::<f32>(to, 4));
    let c = x(rd::<f32>(from, 8)).fsub_m(rd::<f32>(to, 8));
    let d = c.fmul(c).fadd(a.fmul(a)).fadd(b.fmul(b)).fsqrt();
    if gt(d, x(f32::from_bits(max))) {
        return 0;
    }
    if rd::<u32>(ped, 0x478) & 0x8000 != 0 {
        return 1;
    }
    u8::from(is_wander_path_clear(from, to, rd::<u32>(this, 0x24), 4) == 4)
}

// ---- CTaskComplexAvoidOtherPedWhileWandering (PORT-AI-4 main agent) -------------------------------------

/// `CTaskComplexAvoidOtherPedWhileWandering::NearbyPedsInSphere(CPed*, const CColSphere&, CPed** cand, CPed** out)`
/// (0x671FE0, `ret 0x10`; `this` and the ped unused): every candidate (16 slots) whose [0x86FCC0]-radius sphere at
/// its position intersects the sphere moves to the first free slot of `out` (none free: it stays); true when one
/// moved.
#[hook(addr = 0x671FE0, conv = thiscall, name = "CTaskComplexAvoidOtherPedWhileWandering::NearbyPedsInSphere", expect = "83 EC 18 53 8B 5C 24 28")]
pub fn aop_nearby_peds_in_sphere_port(_this: GPtr<u8>, _ped: GPtr<u8>, s: GPtr<u8>, cand: GPtr<u8>, out: GPtr<u8>) -> u8 {
    let mut moved = 0u8;
    for i in 0..16isize {
        let p = rd::<GPtr<u8>>(cand, i * 4);
        if p.is_null() {
            continue;
        }
        let mut sp = [0u32; 5];
        let spp = buf(&mut sp);
        col_sphere_set(spp, g::<u32>(0x86FCC0).read(), ent_pos(p), 0, 0, 0xFF);
        if col_sphere_intersect_sphere(s, spp) == 0 {
            continue;
        }
        if let Some(j) = (0..16isize).find(|&j| rd::<u32>(out, j * 4) == 0) {
            wr::<u32>(cand, i * 4, 0);
            moved = 1;
            wr::<GPtr<u8>>(out, j * 4, p);
        }
    }
    moved
}

/// `CTaskComplexAvoidOtherPedWhileWandering::ComputeAvoidSphere(CPed*, CColSphere& out)` (0x672080, `ret 8`): the
/// sphere starts as the [0x86FCC0]-radius sphere round the ped to avoid (+0xC); the candidates are the ped's
/// scanned peds (intelligence +0x130, 16) except that ped and members of the ped's group (`AreInSameGroup`), the
/// in-sphere list starts with the avoided ped; while `NearbyPedsInSphere` moves peds in, `ComputeSphere` grows the
/// sphere round the list.
#[hook(addr = 0x672080, conv = thiscall, name = "CTaskComplexAvoidOtherPedWhileWandering::ComputeAvoidSphere", expect = "8B 44 24 04 81 EC 80 00 00 00")]
pub fn aop_compute_avoid_sphere_port(this: GPtr<u8>, ped: GPtr<u8>, s: GPtr<u8>) {
    let list = rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x130);
    col_sphere_set(s, g::<u32>(0x86FCC0).read(), ent_pos(rd::<GPtr<u8>>(this, 0xC)), 0, 0, 0xFF);
    let mut a = [0u32; 16];
    let mut b = [0u32; 16];
    for (i, slot) in b.iter_mut().enumerate() {
        let e = rd::<GPtr<u8>>(list, i as isize * 4);
        *slot = if e == rd::<GPtr<u8>>(this, 0xC) || ped_groups_are_in_same_group(ped, e) != 0 {
            0
        } else {
            rd::<u32>(list, i as isize * 4)
        };
    }
    a[0] = rd::<u32>(this, 0xC);
    let (ap, bp) = (buf(&mut a), buf(&mut b));
    if aop_nearby_peds_in_sphere(this, ped, s, bp, ap) != 0 {
        loop {
            aop_compute_sphere(this, s, ap);
            if aop_nearby_peds_in_sphere(this, ped, s, bp, ap) == 0 {
                break;
            }
        }
    }
}

/// `CTaskComplexAvoidOtherPedWhileWandering::ComputeRouteRoundSphere(CPed*, CColSphere&)` (0x66A7B0, `ret 8`): the
/// start (+0x10) and target (+0x1C) copies and the sphere centre get the ped's z (position read three times);
/// `CPedGeometryAnalyser::ComputeRouteRoundSphere(ped, sphere, start, target, +0x34, +0x28)`, its EAX returned.
#[hook(addr = 0x66A7B0, conv = thiscall, name = "CTaskComplexAvoidOtherPedWhileWandering::ComputeRouteRoundSphere", expect = "83 EC 18 8D 41 10")]
pub fn aop_compute_route_round_sphere_port(this: GPtr<u8>, ped: GPtr<u8>, s: GPtr<u8>) -> u32 {
    let mut from = [rd::<u32>(this, 0x10), rd::<u32>(this, 0x14), rd::<u32>(this, 0x18)];
    let mut to = [rd::<u32>(this, 0x1C), rd::<u32>(this, 0x20), rd::<u32>(this, 0x24)];
    from[2] = rd::<u32>(ent_pos(ped), 8);
    to[2] = rd::<u32>(ent_pos(ped), 8);
    wr::<u32>(s, 8, rd::<u32>(ent_pos(ped), 8));
    let (fp, tp) = (buf(&mut from), buf(&mut to));
    geom_compute_route_round_sphere(ped, s, fp, tp, this.byte_add(0x34), this.byte_add(0x28))
}

/// `CTaskComplexAvoidOtherPedWhileWandering::ComputeDetourTarget(CPed*)` (0x672180, `ret 4`): `ComputeAvoidSphere`
/// into a stack sphere, then `ComputeRouteRoundSphere` with it (its EAX returned).
#[hook(addr = 0x672180, conv = thiscall, name = "CTaskComplexAvoidOtherPedWhileWandering::ComputeDetourTarget", expect = "83 EC 14 56 57")]
pub fn aop_compute_detour_target(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    let mut sp = [0u32; 5];
    let spp = buf(&mut sp);
    aop_compute_avoid_sphere(this, ped, spp);
    aop_compute_route_round_sphere(this, ped, spp)
}

/// `CTaskComplexAvoidOtherPedWhileWandering::CreateFirstSubTask(CPed*)` (0x674610, `ret 4`): no ped to avoid (+0xC) →
/// ped +0x478 bit 6 cleared, null. Else the start +0x10 = ped position, the avoid sphere and the route round it;
/// no route → quit the look-at (+0x5C bit 0), clear bit 6, null; a route → bit 6 set, `CTaskSimpleGoToPoint(+0x58,
/// &+0x28, [0x86FC78], false, false)` and the 2 s timer (+0x4C/+0x50/+0x54, also when the pool is full).
#[hook(addr = 0x674610, conv = thiscall, name = "CTaskComplexAvoidOtherPedWhileWandering::CreateFirstSubTask", expect = "64 A1 00 00 00 00")]
pub fn aop_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(this, 0xC) == 0 {
        wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) & 0xFFFF_FFBF);
        return GPtr::NULL;
    }
    let p = ent_pos(ped);
    wr::<u32>(this, 0x10, rd::<u32>(p, 0));
    wr::<u32>(this, 0x14, rd::<u32>(p, 4));
    wr::<u32>(this, 0x18, rd::<u32>(p, 8));
    let mut sp = [0u32; 5];
    let spp = buf(&mut sp);
    aop_compute_avoid_sphere(this, ped, spp);
    if aop_compute_route_round_sphere(this, ped, spp) as u8 == 0 {
        if rd::<u8>(this, 0x5C) & 1 != 0 {
            let ik = GPtr::from_addr(0xC15448);
            if ik_is_looking(ik, ped) != 0 {
                ik_abort_look_at(ik, ped, 0xFA);
            }
        }
        wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) & 0xFFFF_FFBF);
        return GPtr::NULL;
    }
    wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) | 0x40);
    let t = task_new(0x30);
    let t = if t.is_null() {
        GPtr::NULL
    } else {
        let r = g::<u32>(0x86FC78).read();
        go_to_point_ctor(t, rd::<u32>(this, 0x58), this.byte_add(0x28), r, 0, 0)
    };
    wr::<u32>(this, 0x4C, g::<u32>(0xB7CB84).read());
    wr::<u32>(this, 0x50, 0x7D0);
    wr::<u8>(this, 0x54, 1);
    t
}

/// `CTaskComplexWanderStandard::ScanForStuff(CPed*)` (0x672600, `ret 4`): starts the 50 ms scan timer (+0x28/+0x2C/
/// +0x30) when stopped; from +0x34 on, an expired timer (restart flag +0x31) restarts and tries, in order,
/// `LookForGangMembers`, `LookForChatPartners`, `LookForSexyCars` until one succeeds.
#[hook(addr = 0x672600, conv = thiscall, name = "CTaskComplexWanderStandard::ScanForStuff", expect = "56 8B F1 8A 46 30")]
pub fn ws_scan_for_stuff(this: GPtr<u8>, ped: GPtr<u8>) {
    let now = || g::<u32>(0xB7CB84).read();
    if rd::<u8>(this, 0x30) == 0 {
        wr::<u32>(this, 0x28, now());
        wr::<u32>(this, 0x2C, 0x32);
        wr::<u8>(this, 0x30, 1);
    }
    let mut t = now();
    if t < rd::<u32>(this, 0x34) || rd::<u8>(this, 0x30) == 0 {
        return;
    }
    if rd::<u8>(this, 0x31) != 0 {
        wr::<u32>(this, 0x28, t);
        wr::<u8>(this, 0x31, 0);
        t = now();
    }
    if t < rd::<u32>(this, 0x2C).wrapping_add(rd::<u32>(this, 0x28)) {
        return;
    }
    wr::<u32>(this, 0x2C, 0x32);
    wr::<u32>(this, 0x28, t);
    wr::<u8>(this, 0x30, 1);
    if ws_look_for_gang_members(this, ped) == 0 && ws_look_for_chat_partners(this, ped) == 0 {
        ws_look_for_sexy_cars(this, ped);
    }
}

/// `CTaskComplexWander::GetWanderTaskByPedType(const CPed*)` (0x673D00, cdecl): by ped type (+0x598): 6 →
/// `CTaskComplexWanderCop(4, GetRandomNumberInRange(0, 8))`, 18 → `CTaskComplexWanderMedic(4, dir, true)`, 20 →
/// `CTaskComplexWanderCriminal(4, dir, true)`, 22 → `CTaskComplexWanderProstitute(4, dir, true)`, any other →
/// `CTaskComplexWanderStandard(4, _ftol((rand() & 0xFFFF)·[0x858B14]·[0x859000]), true)` (rand at 0x673E7A). The
/// direction is drawn only after a successful allocation; null when the pool is full.
#[hook(addr = 0x673D00, conv = cdecl, name = "CTaskComplexWander::GetWanderTaskByPedType", expect = "6A FF 68 A7 59 84 00")]
pub fn wander_get_wander_task_by_ped_type(ped: GPtr<u8>) -> GPtr<u8> {
    let ty = rd::<u32>(ped, 0x598);
    let size = match ty {
        6 => 0x50,
        18 => 0x28,
        20 => 0x38,
        22 => 0x3C,
        _ => 0x38,
    };
    let t = task_new(size);
    if t.is_null() {
        return GPtr::NULL;
    }
    match ty {
        6 => wander_cop_ctor(t, 4, gen_random_in_range(0, 8) as u32),
        18 => wander_medic_ctor(t, 4, gen_random_in_range(0, 8) as u32, 1),
        20 => wander_criminal_ctor(t, 4, gen_random_in_range(0, 8) as u32, 1),
        22 => wander_prostitute_ctor(t, 4, gen_random_in_range(0, 8) as u32, 1),
        _ => {
            let r = sa_core::orig::at_site(0x673E7A, rand) & 0xFFFF;
            let d = ftol32(xi(r).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(0x859000).read()));
            wander_standard_ctor(t, 4, d as u32, 1)
        }
    }
}

/// `CTaskComplexWanderStandard::LookForSexyCars(CPed*)` (0x66AFD0, `ret 4`): the first scanned vehicle (intelligence
/// +0xE0, 16) that is not the ped's own (+0x58C), is worth > 40000 (handling +0xD8, unsigned), has +0x4C0 >
/// [0x858B58], is closer than [0x86FCD8] ((dz² + dy²) + dx² against c·c, unrounded), ahead of the ped (dot with the
/// matrix forward > [0x858B50]) and in sight (`GetIsLineOfSightClear`, buildings + objects) raises a stack
/// sexy-vehicle event (0x4AF010) in the ped's event group and `SetNextMinScanTime`; true. Else false.
#[hook(addr = 0x66AFD0, conv = thiscall, name = "CTaskComplexWanderStandard::LookForSexyCars", expect = "6A FF 68 B8 50 84 00")]
pub fn ws_look_for_sexy_cars_port(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let pos = ent_pos(ped);
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    for i in 0..16isize {
        let v = rd::<GPtr<u8>>(intel, 0xE0 + i * 4);
        if v.is_null() || v == rd::<GPtr<u8>>(ped, 0x58C) {
            continue;
        }
        if rd::<u32>(rd::<GPtr<u8>>(v, 0x384), 0xD8) <= 0x9C40 {
            continue;
        }
        if x(rd::<f32>(v, 0x4C0)).fcom(x(g::<f32>(0x858B58).read())).test_any(0x41) {
            continue;
        }
        let vp = ent_pos(v);
        let dx = x(rd::<f32>(vp, 0)).fsub_m(rd::<f32>(pos, 0));
        let dy = x(rd::<f32>(vp, 4)).fsub_m(rd::<f32>(pos, 4));
        let dz = x(rd::<f32>(vp, 8)).fsub_m(rd::<f32>(pos, 8));
        let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
        let c = g::<f32>(0x86FCD8).read();
        if x(c).fmul_m(c).fcom(d2).test_any(0x41) {
            continue;
        }
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let dot = dz.fmul_m(rd::<f32>(m, 0x18)).fadd(dy.fmul_m(rd::<f32>(m, 0x14))).fadd(dx.fmul_m(rd::<f32>(m, 0x10)));
        if dot.fcom(x(g::<f32>(0x858B50).read())).test_any(0x41) {
            continue;
        }
        if world_los_clear(pos, vp, 1, 0, 0, 1, 0, 0, 0) == 0 {
            continue;
        }
        let mut ev = [0u32; 6];
        let ep = buf(&mut ev);
        sexy_vehicle_ctor(ep, v);
        event_group_add_ws(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ep, 0);
        ws_set_next_min_scan_time(this, ped);
        sexy_vehicle_dtor(ep);
        return 1;
    }
    0
}

/// `CTaskComplexWanderStandard::LookForGangMembers(CPed*)` (0x670100, `ret 4`): only for a ped without a group. With
/// the wander target T (`ComputeTargetPos(ped, &T, +0x1C)`), the first scanned ped (intelligence +0x130, 16) that
/// lies ahead ((e - T)·(pos - T) > [0x858B50], e - T stored as f32, its z kept unrounded for the dot) within
/// [0x858FE8] (|e - T|² from the f32 copies) and leads a group with members raises a stack
/// `CEventAcquaintancePed(e)` with vtable 0x86CAC8 and +0xE = 0x3AC in the ped's event group and
/// `SetNextMinScanTime`; true. Else false.
#[hook(addr = 0x670100, conv = thiscall, name = "CTaskComplexWanderStandard::LookForGangMembers", expect = "6A FF 68 A8 55 84 00")]
pub fn ws_look_for_gang_members_port(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if !ped_groups_get_peds_group(ped).is_null() {
        return 0;
    }
    let mut t = [0u32; 3];
    let tp = buf(&mut t);
    wander_compute_target_pos_orig(this, ped, tp, this.byte_add(0x1C));
    let (tx, ty, tz) = (f32::from_bits(t[0]), f32::from_bits(t[1]), f32::from_bits(t[2]));
    let pos = ent_pos(ped);
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    for i in 0..16isize {
        let e = rd::<GPtr<u8>>(intel, 0x130 + i * 4);
        if e.is_null() {
            continue;
        }
        let ax = x(rd::<f32>(pos, 0)).fsub_m(tx);
        let ay = x(rd::<f32>(pos, 4)).fsub_m(ty);
        let az = x(rd::<f32>(pos, 8)).fsub_m(tz);
        let ep = ent_pos(e);
        let bx = f32_of(x(rd::<f32>(ep, 0)).fsub_m(tx));
        let by = f32_of(x(rd::<f32>(ep, 4)).fsub_m(ty));
        let bz87 = x(rd::<f32>(ep, 8)).fsub_m(tz);
        let bz = f32_of(bz87);
        let dot = bz87.fmul(az).fadd(x(by).fmul(ay)).fadd(x(bx).fmul(ax));
        if dot.fcom(x(g::<f32>(0x858B50).read())).test_any(0x41) {
            continue;
        }
        let b2 = x(bz).fmul_m(bz).fadd(x(by).fmul_m(by)).fadd(x(bx).fmul_m(bx));
        if b2.fcom(x(g::<f32>(0x858FE8).read())).test_parity(5) {
            continue;
        }
        let grp = ped_groups_get_peds_group(e);
        if grp.is_null() || membership_count_members_excluding_leader(grp.byte_add(8)) <= 0 {
            continue;
        }
        let mut ev = [0u32; 6];
        let evp = buf(&mut ev);
        acquaintance_ped_ctor(evp, e);
        wr::<u32>(evp, 0, 0x86CAC8);
        wr::<u16>(evp, 0xE, 0x3AC);
        event_group_add_ws(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), evp, 0);
        ws_set_next_min_scan_time(this, ped);
        wr::<u32>(evp, 0, 0x86CAC8);
        acquaintance_ped_dtor(evp);
        return 1;
    }
    0
}

/// `CTaskComplexWanderCop::LookForCriminals(CPed*)` (0x66B300, `ret 4`): the first scanned ped (intelligence +0x130,
/// 16) of type 7..16, or type 0x14 other than the last one chased (+0x40), whose active task has this task's type,
/// that is closer than [0x86FCE8] ((dz² + dy²) + dx² against c·c), ahead of the cop (dot with the matrix forward >
/// [0x858B50]) and in sight: a stack `CEventPedToChase(criminal)` goes to the cop's event group, a stack
/// `CEventPedToFlee(cop)` to the criminal's; +0x38 = now + 30000, +0x40 = the criminal; true. Else false.
#[hook(addr = 0x66B300, conv = thiscall, name = "CTaskComplexWanderCop::LookForCriminals", expect = "6A FF 64 A1 00 00 00 00")]
pub fn cop_look_for_criminals(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let pos = ent_pos(ped);
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    for i in 0..16isize {
        let e = rd::<GPtr<u8>>(intel, 0x130 + i * 4);
        if e.is_null() {
            continue;
        }
        let ty = rd::<i32>(e, 0x598);
        let ok = (7..=0x10).contains(&ty) || (ty == 0x14 && e != rd::<GPtr<u8>>(this, 0x40));
        if !ok {
            continue;
        }
        let tm = rd::<GPtr<u8>>(e, 0x47C).byte_add(4);
        if tm_get_active_task(tm).is_null() {
            continue;
        }
        let a = tm_get_active_task(rd::<GPtr<u8>>(e, 0x47C).byte_add(4));
        let at = v_task_type_ws(a);
        if at != v_task_type_ws(this) {
            continue;
        }
        let ep = ent_pos(e);
        let dx = x(rd::<f32>(ep, 0)).fsub_m(rd::<f32>(pos, 0));
        let dy = x(rd::<f32>(ep, 4)).fsub_m(rd::<f32>(pos, 4));
        let dz = x(rd::<f32>(ep, 8)).fsub_m(rd::<f32>(pos, 8));
        let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
        let c = g::<f32>(0x86FCE8).read();
        if x(c).fmul_m(c).fcom(d2).test_any(0x41) {
            continue;
        }
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let dot = dz.fmul_m(rd::<f32>(m, 0x18)).fadd(dy.fmul_m(rd::<f32>(m, 0x14))).fadd(dx.fmul_m(rd::<f32>(m, 0x10)));
        if dot.fcom(x(g::<f32>(0x858B50).read())).test_any(0x41) {
            continue;
        }
        if world_los_clear(pos, ep, 1, 0, 0, 1, 0, 0, 0) == 0 {
            continue;
        }
        let mut chase = [0u32; 6];
        let mut flee = [0u32; 6];
        let (cp, fp) = (buf(&mut chase), buf(&mut flee));
        ped_to_chase_ctor(cp, e);
        event_group_add_ws(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), cp, 0);
        ped_to_flee_ctor(fp, ped);
        event_group_add_ws(rd::<GPtr<u8>>(e, 0x47C).byte_add(0x68), fp, 0);
        wr::<u32>(this, 0x38, g::<u32>(0xB7CB84).read().wrapping_add(0x7530));
        wr::<GPtr<u8>>(this, 0x40, e);
        ped_to_flee_dtor(fp);
        ped_to_chase_dtor(cp);
        return 1;
    }
    0
}

/// `CTaskComplexAvoidOtherPedWhileWandering::SetUpIK(CPed*)` (0x66A850, `ret 4`): the `CTaskSimpleGoTo::SetUpIK`
/// shape with the checks in this order: the player with pad 0 word +0x10E = 0, off screen, already looking (+0x5C
/// bit 0), a look-at entity, secondary task 5, parent 0x395/0x3AB (two `GetTaskType` calls). Direction = detour
/// target (+0x1C) - position, squared length > [0x85EED4], normalised, dot with the matrix forward (no null check) <
/// [0xC18D48]; look at (t.x + 2d.x, t.y + 2d.y, (2d.z + pos.z) + [0x86FD40]) for 5 s ("0x86FEF0"); +0x5C |= 1.
#[hook(addr = 0x66A850, conv = thiscall, name = "CTaskComplexAvoidOtherPedWhileWandering::SetUpIK", expect = "83 EC 24 53 56 57")]
pub fn aop_set_up_ik(this: GPtr<u8>, ped: GPtr<u8>) {
    let pad = pad_get_pad_ws(0);
    if ped == find_player_ped(-1) && rd::<u16>(pad, 0x10E) == 0 {
        return;
    }
    if entity_get_is_on_screen_ws(ped) == 0 || rd::<u8>(this, 0x5C) & 1 != 0 {
        return;
    }
    let ik = GPtr::from_addr(0xC15448);
    if ik_get_look_at_entity_ws(ik, ped) != 0 {
        return;
    }
    if !tm_get_task_secondary_ws(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 5).is_null() {
        return;
    }
    let parent = rd::<GPtr<u8>>(this, 4);
    if !parent.is_null() && (v_task_type_ws(parent) == 0x395 || v_task_type_ws(rd::<GPtr<u8>>(this, 4)) == 0x3AB) {
        return;
    }
    let mut d = [0u32; 3];
    let dp = buf(&mut d);
    vector_sub(dp, this.byte_add(0x1C), ent_pos(ped));
    let m2 = X87::from_f64_exact(vector_magnitude_sqr_ws(dp));
    if m2.fcom(x(g::<f32>(0x85EED4).read())).test_any(0x41) {
        return;
    }
    vector_normalise(dp);
    let (vx, vy, vz) = (rd::<f32>(dp, 0), rd::<f32>(dp, 4), rd::<f32>(dp, 8));
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let dot = x(vz).fmul_m(rd::<f32>(m, 0x18)).fadd(x(vy).fmul_m(rd::<f32>(m, 0x14))).fadd(x(vx).fmul_m(rd::<f32>(m, 0x10)));
    if dot.fcom(x(g::<f32>(0xC18D48).read())).test_parity(5) {
        return;
    }
    let (tx, ty) = (rd::<f32>(this, 0x1C), rd::<f32>(this, 0x20));
    let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    let pz = x(rd::<f32>(pos, 8));
    let vz2 = f32_of(x(vz).fadd(x(vz)));
    let nx = f32_of(x(vx).fadd(x(vx)).fadd_m(tx));
    let ny = f32_of(x(ty).fadd(x(vy).fadd(x(vy))));
    let nz = f32_of(x(vz2).fadd(pz).fadd_m(g::<f32>(0x86FD40).read()));
    let mut posn = [nx.to_bits(), ny.to_bits(), nz.to_bits()];
    ik_look_at_ws(ik, 0x86FEF0, ped, 0, 0x1388, 0xFFFF_FFFF, buf(&mut posn), 0, 0x3E80_0000, 0x1F4, 3, 0);
    wr::<u8>(this, 0x5C, rd::<u8>(this, 0x5C) | 1);
}

/// `CTaskComplexAvoidOtherPedWhileWandering::ControlSubTask(CPed*)` (0x6721B0, `ret 4`; re-read from the bytes, the
/// spec row is medium). No ped to avoid: quit the look-at (+0x5C bit 0), clear ped +0x478 bit 6, null. A pending
/// abort (+0x5C bit 1) that the sub-task accepts (`MakeAbortable(ped, 1, null)`) → clear, null. Starts the 200 ms
/// timer (+0x40), `SetUpIK`; a stand-still sub-task (0xCB) is kept. A ped whose intelligence +0x274 > [0x86C938]
/// quits the IK and stands still for `GetRandomNumberInRange(0, 2000) + 500` ms. Too far apart in z (|dz| >
/// [0x858B3C]) → quit, clear, null. With D = f32(dy² + dx²) to the other ped: the other ped's simplest active task
/// being a go-to task once the timer +0x4C ran out, both facing the same way (forward dot >= [0x86CD98]), the
/// ped no faster than the other (|50·v|² <= |50·v_other|² + [0x870144], z dropped) and D > [0x870140] → quit; D >
/// [0xC18D24] (lazily set to [0x86C8C0]² + [0x858624], flag bit 0 of 0xC18D28) → quit. Else every 200 ms: a gang
/// follower (+0x5C bit 2, task 0x4B7 with a leader) re-targets +0x1C to leader position + offset, a new detour
/// (`ComputeDetourTarget`, none → quit) is passed to the go-to-point (`UpdatePoint(&+0x28, [0x86FC78], false)`).
/// The original keeps D in its ped argument slot (a local here).
#[hook(addr = 0x6721B0, conv = thiscall, name = "CTaskComplexAvoidOtherPedWhileWandering::ControlSubTask", expect = "64 A1 00 00 00 00")]
pub fn aop_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let ik = GPtr::from_addr(0xC15448);
    let flags = rd::<u8>(this, 0x5C);
    let clear = |ped: GPtr<u8>| wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) & 0xFFFF_FFBF);
    if rd::<u32>(this, 0xC) == 0 {
        if flags & 1 != 0 && ik_is_looking(ik, ped) != 0 {
            ik_abort_look_at(ik, ped, 0xFA);
        }
        clear(ped);
        return GPtr::NULL;
    }
    if flags & 2 != 0 && v_make_abortable_ws(rd::<GPtr<u8>>(this, 8), ped, 1, 0) != 0 {
        clear(ped);
        return GPtr::NULL;
    }
    if rd::<u8>(this, 0x48) == 0 {
        wr::<u32>(this, 0x40, g::<u32>(0xB7CB84).read());
        wr::<u32>(this, 0x44, 0xC8);
        wr::<u8>(this, 0x48, 1);
    }
    aop_set_up_ik_call(this, ped);
    if v_task_type_ws(rd::<GPtr<u8>>(this, 8)) == 0xCB {
        return rd::<GPtr<u8>>(this, 8);
    }
    if rd::<i32>(rd::<GPtr<u8>>(ped, 0x47C), 0x274) > g::<i32>(0x86C938).read() {
        aop_quit_ik_call(this, ped);
        let t = task_new(0x20);
        if t.is_null() {
            return GPtr::NULL;
        }
        let ms = gen_random_in_range(0, 0x7D0).wrapping_add(0x1F4);
        return stand_still_ctor_ws(t, ms as u32, 0, 0, 0x4100_0000);
    }
    let quit = |this: GPtr<u8>, ped: GPtr<u8>| -> GPtr<u8> {
        aop_quit_ik_call(this, ped);
        wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) & 0xFFFF_FFBF);
        GPtr::NULL
    };
    let pm = rd::<GPtr<u8>>(ped, 0x14);
    let pp = if pm.is_null() { ped.byte_add(4) } else { pm.byte_add(0x30) };
    let other = rd::<GPtr<u8>>(this, 0xC);
    let op = ent_pos(other);
    let dz = x(rd::<f32>(pp, 8)).fsub_m(rd::<f32>(op, 8));
    if !X87::from_f64_exact(dz.to_f64().abs()).fcom(x(g::<f32>(0x858B3C).read())).test_any(0x41) {
        return quit(this, ped);
    }
    let op = ent_pos(other);
    let pp = if pm.is_null() { ped.byte_add(4) } else { pm.byte_add(0x30) };
    let dx = x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(op, 0));
    let dy = x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(op, 4));
    let d = f32_of(dy.fmul(dy).fadd(dx.fmul(dx)));
    let simplest = tm_get_simplest_active_task(rd::<GPtr<u8>>(other, 0x47C).byte_add(4));
    if !simplest.is_null()
        && task_timer_is_out_of_time_ws(this.byte_add(0x4C)) != 0
        && task_is_go_to_task(simplest) != 0
    {
        let o = rd::<GPtr<u8>>(this, 0xC);
        let dot = X87::from_f64_exact(vector_dot_product_ws(
            rd::<GPtr<u8>>(ped, 0x14).byte_add(0x10),
            rd::<GPtr<u8>>(o, 0x14).byte_add(0x10),
        ));
        if !dot.fcom(x(g::<f32>(0x86CD98).read())).test_any(1) {
            let mut a = [0u32; 3];
            let mut b = [0u32; 3];
            let (ap, bp) = (buf(&mut a), buf(&mut b));
            vector_mul_ws(ap, ped.byte_add(0x44), 50.0);
            vector_mul_ws(bp, o.byte_add(0x44), 50.0);
            wr::<u32>(ap, 8, 0);
            wr::<u32>(bp, 8, 0);
            let m1 = f32_of(X87::from_f64_exact(vector_magnitude_sqr_ws(bp)).fadd_m(g::<f32>(0x870144).read()));
            let m2 = X87::from_f64_exact(vector_magnitude_sqr_ws(ap));
            if !m2.fcom(x(m1)).test_parity(0x41) && !x(d).fcom(x(g::<f32>(0x870140).read())).test_any(0x41) {
                return quit(this, ped);
            }
        }
    }
    let flag = g::<u32>(0xC18D28);
    if flag.read() & 1 == 0 {
        let c = g::<f32>(0x86C8C0).read();
        let f = flag.read();
        let v = x(c).fmul_m(c);
        flag.write(f | 1);
        g::<f32>(0xC18D24).write(f32_of(v.fadd_m(g::<f32>(0x858624).read())));
    }
    if !x(d).fcom(x(g::<f32>(0xC18D24).read())).test_any(0x41) {
        return quit(this, ped);
    }
    if task_timer_is_out_of_time_ws(this.byte_add(0x40)) == 0 {
        return rd::<GPtr<u8>>(this, 8);
    }
    wr::<u32>(this, 0x40, g::<u32>(0xB7CB84).read());
    wr::<u32>(this, 0x44, 0xC8);
    wr::<u8>(this, 0x48, 1);
    if rd::<u8>(this, 0x5C) & 4 != 0 {
        let t = intel_find_task_by_type_ws(rd::<GPtr<u8>>(ped, 0x47C), 0x4B7);
        if !t.is_null() {
            let leader = rd::<GPtr<u8>>(t, 0x10);
            if !leader.is_null() {
                let mut off = [0u32; 3];
                let mut w = [0u32; 3];
                let offp = buf(&mut off);
                gang_follower_calculate_offset_position(t, offp);
                let r = vector_add_ws(buf(&mut w), ent_pos(leader), offp);
                wr::<u32>(this, 0x1C, rd::<u32>(r, 0));
                wr::<u32>(this, 0x20, rd::<u32>(r, 4));
                wr::<u32>(this, 0x24, rd::<u32>(r, 8));
            }
        }
    }
    if aop_compute_detour_target_call(this, ped) == 0 {
        return quit(this, ped);
    }
    let r = g::<u32>(0x86FC78).read();
    goto_point_update_point_ws(rd::<GPtr<u8>>(this, 8), this.byte_add(0x28), r, 0);
    rd::<GPtr<u8>>(this, 8)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_FPR_GET_SUB_TASK_TYPE,
    HOOK_FPR_CREATE_TASK,
    HOOK_FPR_CREATE_FIRST_SUB_TASK,
    HOOK_FPR_CONTROL_SUB_TASK,
    HOOK_FPR_CREATE_NEXT_SUB_TASK,
    HOOK_FPR_MAKE_ABORTABLE,
    HOOK_FNR_MAKE_ABORTABLE,
    HOOK_FNR_GET_SUB_TASK_TYPE,
    HOOK_FNR_CREATE_SUB_TASK,
    HOOK_FNR_COMPUTE_ROUTE,
    HOOK_WANDER_UPDATE_DIR,
    HOOK_WANDER_UPDATE_PATH_NODES,
    HOOK_WANDER_VALID_NODES,
    HOOK_WANDER_COMPUTE_TARGET_POS,
    HOOK_AVOID_QUIT_IK,
    HOOK_FNR_SET_TARGET,
    HOOK_FNR_CREATE_FIRST_SUB_TASK,
    HOOK_FNR_CREATE_NEXT_SUB_TASK,
    HOOK_FNR_CONTROL_SUB_TASK,
    HOOK_WANDER_CREATE_SUB_TASK,
    HOOK_WANDER_CREATE_FIRST_SUB_TASK,
    HOOK_WANDER_CREATE_NEXT_SUB_TASK,
    HOOK_WANDER_SCAN_FOR_BLOCKED_NODES,
    HOOK_WANDER_SCAN_FOR_BLOCKED_NODE,
    HOOK_WANDER_SCAN_FOR_BLOCKED_NODE_1,
    HOOK_WANDER_CONTROL_SUB_TASK,
    HOOK_COP_SHOULD_PURSUE_PLAYER,
    HOOK_COP_LOOK_FOR_CAR_ALARMS,
    HOOK_COP_LOOK_FOR_STOLEN_COP_CARS,
    HOOK_COP_SCAN_FOR_STUFF,
    HOOK_COP_CREATE_FIRST_SUB_TASK,
    HOOK_COP_CREATE_NEXT_SUB_TASK,
    HOOK_COP_CONTROL_SUB_TASK,
    HOOK_FPR_CALC_GO_TO_TASK_TYPE_PORT,
    HOOK_FNR_CALC_GO_TO_TASK_TYPE_PORT,
    HOOK_FNR_CAN_GO_STRAIGHT_THERE,
    HOOK_AOP_NEARBY_PEDS_IN_SPHERE_PORT,
    HOOK_AOP_COMPUTE_AVOID_SPHERE_PORT,
    HOOK_AOP_COMPUTE_ROUTE_ROUND_SPHERE_PORT,
    HOOK_AOP_COMPUTE_DETOUR_TARGET,
    HOOK_AOP_CREATE_FIRST_SUB_TASK,
    HOOK_WS_SCAN_FOR_STUFF,
    HOOK_WANDER_GET_WANDER_TASK_BY_PED_TYPE,
    HOOK_WS_LOOK_FOR_SEXY_CARS_PORT,
    HOOK_WS_LOOK_FOR_GANG_MEMBERS_PORT,
    HOOK_COP_LOOK_FOR_CRIMINALS,
    HOOK_AOP_SET_UP_IK,
    HOOK_AOP_CONTROL_SUB_TASK,
];
