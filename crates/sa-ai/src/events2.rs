//! More `CEvent*` methods (PORT-AI-4, helper A): editable-response informers, decision-maker response
//! computation, collision/sound/damage events' `AffectsPed`, constructors and clones. Ported from the
//! 1.0 US bytes (`docs/re/specs/AI/CEvent*.md` as reading aid); the exe wins over gta-reversed.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEventPool::New` (0x4B5570, thiscall on the pool)
    fn event_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B5570 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(grp: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CPed::IsAlive` (0x5E0170)
    fn ped_is_alive(ped: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPedIntelligence::IsThreatenedBy(const CPed&)` (0x601C30)
    fn intel_is_threatened_by(intel: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x601C30 => unported;
    /// `CPedGroups::GetPedsGroup(CPed*)` (0x5F7E80, cdecl)
    fn ped_groups_get_peds_group(ped: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupMembership::IsLeader(CPed*)` (0x5F69C0)
    fn membership_is_leader(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5F69C0 => unported;
    /// `CPedGroupMembership::GetLeader` (0x5F69A0)
    fn membership_get_leader(m: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F69A0 => unported;
    /// `CPedGroupMembership::GetMember(int)` (0x5F69B0)
    fn membership_get_member(m: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x5F69B0 => unported;
    /// `CVector::DotProduct(const CVector*, const CVector*)` (0x40FDB0, cdecl, ST0)
    fn vector_dot_product_st0(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `CVector::Normalise` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CVector::Difference` (0x40FE00, `ecx = out`, `ret 8`): `out = a - b`
    fn vector_difference(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = thiscall 0x40FE00 => unported;
    /// `FUN_00406da0` (0x406DA0): `CVector::MagnitudeSqr` (ST0)
    fn vector_magnitude_sqr_st0(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CTaskManager::FindActiveTaskByType(int)` (0x681740)
    fn tm_find_active_task_by_type(tm: GPtr<u8>, t: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CTaskManager::GetActiveTask` (0x681720)
    fn tm_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CEventHandler::GetCurrentEventType` (0x4B8CC0)
    fn handler_get_current_event_type(h: GPtr<u8>) -> i32 = thiscall 0x4B8CC0 => unported;
    /// `CEventHandlerHistory::GetCurrentEvent` (0x4B8CA0)
    fn history_get_current_event(h: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B8CA0 => unported;
    /// `CWorld::FindPlayerPed(int)` (0x56E210, cdecl)
    fn find_player_ped(i: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CAcquaintance::GetAcquaintances(int)` (0x608970)
    fn acquaintance_get(a: GPtr<u8>, i: i32) -> u32 = thiscall 0x608970 => unported;
    /// `CPedType::GetPedFlag(int)` (0x608830, cdecl)
    fn ped_type_get_ped_flag(t: u32) -> u32 = cdecl 0x608830 => unported;
    /// `CInformFriendsEventQueue::Add(CEntity*, CEvent*)` (0x4AC1E0, cdecl)
    fn inform_friends_queue_add(ped: GPtr<u8>, e: GPtr<u8>) -> u8 = cdecl 0x4AC1E0 => unported;
    /// `CInformGroupEventQueue::Add(CEntity*, CPedGroup*, CEvent*)` (0x4B7CD0, cdecl)
    fn inform_group_queue_add(ped: GPtr<u8>, grp: GPtr<u8>, e: GPtr<u8>) -> u8 = cdecl 0x4B7CD0 => unported;
    /// `CEvent::GetSoundLevel(CEntity*, CVector&)` (0x4B2850, `ret 8`, ST0)
    fn event_get_sound_level_st0(ev: GPtr<u8>, ent: GPtr<u8>, pos: GPtr<u8>) -> f64 = thiscall 0x4B2850 => unported;
    /// `CEventSource::ComputeEventSourceType(const CEvent&, const CPed&)` (0x4ABAC0, cdecl)
    fn compute_event_source_type(ev: GPtr<u8>, ped: GPtr<u8>) -> u32 = cdecl 0x4ABAC0 => unported;
    /// `CDecisionMakerTypes::GetInstance` (0x4684F0, cdecl)
    fn decision_maker_types_get_instance() -> GPtr<u8> = cdecl 0x4684F0 => unported;
    /// `CDecisionMakerTypes::MakeDecision(CPed*, ...)` (0x606E70, `ret 0x2C`)
    fn make_decision_ped(dm: GPtr<u8>, ped: GPtr<u8>, ev: i32, src: u32, b: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, task: GPtr<u8>, facial: GPtr<u8>) = thiscall 0x606E70 => unported;
    /// `CDecisionMakerTypes::MakeDecision(CPedGroup*, ...)` (0x606F80, `ret 0x20`, AX = task)
    fn make_decision_group(dm: GPtr<u8>, grp: GPtr<u8>, ev: i32, src: u32, b: u32, a4: u32, a5: u32, a6: u32, a7: u32) -> u32 = thiscall 0x606F80 => unported;
    /// `CEventEditableResponse::Constructor` (0x4AC450)
    fn editable_response_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AC450 => unported;

    /// `FUN_004abca0` (0x4ABCA0, AL)
    fn fun_004abca0(ev: GPtr<u8>) -> u8 = thiscall 0x4ABCA0 => unported;
    /// `CEvent::TakesPriorityOver(const CEvent&)` (0x4ABC30, AL)
    fn event_takes_priority_over(ev: GPtr<u8>, other: GPtr<u8>) -> u8 = thiscall 0x4ABC30 => unported;
    /// `CGeneral::GetRandomNumberInRange(int, int)` (0x407180, cdecl)
    fn general_get_random_number_in_range_int(a: i32, b: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CTaskSimpleCarSetPedOut::CTaskSimpleCarSetPedOut(CVehicle*, int, bool)` (0x6478B0)
    fn task_car_set_ped_out_ctor(t: GPtr<u8>, veh: GPtr<u8>, door: i32, b: u32) -> GPtr<u8> = thiscall 0x6478B0 => unported;
    /// `CTaskSimpleCarSetPedOut::ProcessPed(CPed*)` (0x647D10)
    fn task_car_set_ped_out_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x647D10 => unported;
    /// `FUN_00647950` (0x647950): `CTaskSimpleCarSetPedOut` destructor
    fn task_car_set_ped_out_dtor(t: GPtr<u8>) = thiscall 0x647950 => unported;
    /// `CCarEnterExit::RemoveCarSitAnim(const CPed*)` (0x64F680, cdecl)
    fn car_enter_exit_remove_car_sit_anim(ped: GPtr<u8>) = cdecl 0x64F680 => unported;
    /// `CEvent::GetEventPriority` (vtable +8)
    fn ev_get_event_priority(ev: GPtr<u8>) -> i32 = thiscall vslot 0x8 => unported;

    /// `CPed::IsAlive` (0x5E0170) with its whole EAX
    fn ped_is_alive_eax(ped: GPtr<u8>) -> u32 = thiscall 0x5E0170 => unported;
    /// `CEventGunShot::Constructor(CEntity*, CVector, CVector, bool)` (0x4AC610, `ret 0x20`)
    fn gun_shot_ctor(this: GPtr<u8>, ent: u32, a0: u32, a1: u32, a2: u32, b0: u32, b1: u32, b2: u32, f: u32) -> GPtr<u8> = thiscall 0x4AC610 => unported;
    /// `CPedIntelligence::FindTaskByType(int)` (0x600EE0)
    fn intel_find_task_by_type(intel: GPtr<u8>, t: i32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `CPedGroups::AreInSameGroup(const CPed*, const CPed*)` (0x5F7F40, cdecl)
    fn ped_groups_are_in_same_group(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x5F7F40 => unported;
    /// `CVehicle::IsPassenger(const CPed*)` (0x6D1BD0)
    fn vehicle_is_passenger(v: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6D1BD0 => unported;
    /// `CWorld::FindPlayerWanted(int)` (0x56E230, cdecl)
    fn find_player_wanted(i: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CWanted::IsInPursuit(CCopPed*)` (0x562330)
    fn wanted_is_in_pursuit(w: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x562330 => unported;
    /// `CWanted::CanCopJoinPursuit(CCopPed*)` (0x562FB0)
    fn wanted_can_cop_join_pursuit(w: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x562FB0 => unported;
    /// `CWanted::RegisterCrime(eCrimeType, const CVector&, CPed*, bool)` (0x562410)
    fn wanted_register_crime(w: GPtr<u8>, crime: u32, pos: GPtr<u8>, ped: GPtr<u8>, b: u32) = thiscall 0x562410 => unported;
    /// `CPedIntelligence::GetPedFOVRange` (0x4ABEA0, ST0)
    fn intel_get_ped_fov_range_st0(intel: GPtr<u8>) -> f64 = thiscall 0x4ABEA0 => unported;
    /// `VectorSub(CVector* out, const CVector*, const CVector*)` (0x40FE60, cdecl)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = cdecl 0x40FE60 => unported;
    /// `CWorld::GetIsLineOfSightClear(...)` (0x56A490, cdecl)
    fn world_get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, c1: u32, c2: u32, c3: u32, c4: u32, c5: u32, c6: u32, c7: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CPedType::PoliceDontCareAboutCrimesAgainstPedType(int)` (0x608A40, cdecl, AL)
    fn ped_type_police_dont_care(t: u32) -> u8 = cdecl 0x608A40 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60, cdecl)
    fn weapon_info_get(w: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CPedIntelligence::CanSeeEntityWithLights(const CEntity*, bool)` (0x605550, ST0)
    fn intel_can_see_entity_with_lights_st0(intel: GPtr<u8>, e: GPtr<u8>, b: u32) -> f64 = thiscall 0x605550 => unported;
    /// `CPhysical::CanPhysicalBeDamaged(eWeaponType, bool*)` (0x5448B0, AL)
    fn physical_can_be_damaged(p: GPtr<u8>, w: u32, out: u32) -> u8 = thiscall 0x5448B0 => unported;
    /// `CPedIntelligence::GetTaskJetPack` (0x601110)
    fn intel_get_task_jet_pack(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x601110 => unported;
    /// `CPed::RemoveBodyPart(int, char)` (0x5F0140)
    fn ped_remove_body_part(ped: GPtr<u8>, part: u32, dir: u32) = thiscall 0x5F0140 => unported;
    /// `CEventDamage::ComputeBodyPartToRemove(int&)` (0x4ADC10)
    fn damage_compute_body_part_to_remove(ev: GPtr<u8>, out: GPtr<u8>) = thiscall 0x4ADC10 => unported;
    /// `CEvent::GetLifeTime` (vtable +0xC)
    fn ev_get_life_time(ev: GPtr<u8>) -> i32 = thiscall vslot 0xC => unported;
    /// `CEvent::IsCriminalEvent` (vtable +0x1C)
    fn ev_is_criminal_event(ev: GPtr<u8>) -> u8 = thiscall vslot 0x1C => unported;

    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0)
    fn ik_is_looking(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::GetLookAtEntity(CPed*)` (0x6181D0)
    fn ik_get_look_at_entity(m: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x6181D0 => unported;
    /// `CPedIntelligence::IsInSeeingRange(const CVector&)` (0x600C60, AL)
    fn intel_is_in_seeing_range(intel: GPtr<u8>, pos: GPtr<u8>) -> u8 = thiscall 0x600C60 => unported;
    /// `CCrime::ReportCrime(eCrimeType, CEntity*, CPed*)` (0x532010, cdecl)
    fn crime_report_crime(ty: i32, victim: GPtr<u8>, ped: GPtr<u8>) = cdecl 0x532010 => unported;
    /// `CEventDamage::CEventDamage(const CEventDamage&)` (0x4B33B0)
    fn damage_copy_ctor(this: GPtr<u8>, other: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B33B0 => unported;
    /// `FUN_004ade90` (0x4ADE90): `CEventDeath` destructor body
    fn death_dtor(this: GPtr<u8>) = thiscall 0x4ADE90 => unported;
    /// `CEventKnockOffBike::Constructor-first` (0x4AFCF0)
    fn knock_off_bike_ctor(this: GPtr<u8>, veh: u32, v1: GPtr<u8>, v2: GPtr<u8>, f24: u32, f28: u32, b36: u32, b34: u32, i2c: u32, ped: u32, b1: u32, b2: u32) -> GPtr<u8> = thiscall 0x4AFCF0 => unported;
    /// `CPedGroupIntelligence::AddEvent(CEvent*)` (0x5F7470)
    fn pgi_add_event(gi: GPtr<u8>, e: GPtr<u8>) = thiscall 0x5F7470 => unported;

    /// `IKChainManager_c::LookAt(const char*, CPed*, CEntity*, int, ePedBones, RwV3d*, bool, float, int, int, bool)` (0x618970)
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, e: GPtr<u8>, time: u32, bone: u32, pos: u32, b: u32, speed: u32, blend: u32, prio: u32, f: u32) -> u8 = thiscall 0x618970 => unported;
    /// `CTaskManager::GetSimplestActiveTask` (0x6819D0)
    fn tm_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTask::IsGoToTask(CTask*)` (0x61A360, cdecl, AL)
    fn task_is_go_to_task(t: GPtr<u8>) -> u8 = cdecl 0x61A360 => unported;
    /// `CTaskComplexWalkRoundCar::SetNewVehicle(CVehicle*, int8)` (0x654290)
    fn walk_round_car_set_new_vehicle(t: GPtr<u8>, v: GPtr<u8>, f: i32) = thiscall 0x654290 => unported;
    /// `CTaskComplexEnterCar::GetTargetPos(CVector* out)` (0x63A300): returns `out`
    fn enter_car_get_target_pos(t: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x63A300 => unported;
    /// `CPedGeometryAnalyser::ComputeClosestSurfacePoint(const CPed&, CEntity&, CVector&)` (0x5F3B70, cdecl)
    fn geom_compute_closest_surface_point(ped: GPtr<u8>, e: GPtr<u8>, out: GPtr<u8>) = cdecl 0x5F3B70 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityHitSide(const CVector&, CEntity&)` (0x5F3AC0, cdecl)
    fn geom_compute_entity_hit_side(pos: GPtr<u8>, e: GPtr<u8>) -> i32 = cdecl 0x5F3AC0 => unported;
    /// `CPedGroupMembership::IsMember(const CPed*)` (0x5F6A10)
    fn membership_is_member(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5F6A10 => unported;

    /// `CVehiclePotentialCollisionScanner::ScanForVehiclePotentialCollisionEvents(const CPed&, CEntity**, int)` (0x603720)
    fn scan_vehicle_potential_collision(s: GPtr<u8>, ped: GPtr<u8>, ents: GPtr<u8>, n: i32) = thiscall 0x603720 => unported;
    /// `FUN_00606580` (0x606580, `ret 8`): an empty-class scanner method (ECX unused by its body)
    fn fun_00606580(s: GPtr<u8>, ped: GPtr<u8>, e: u32) = thiscall 0x606580 => unported;
    /// `FUN_00606890` (0x606890)
    fn fun_00606890(s: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x606890 => unported;
    /// `FUN_00607d80` (0x607D80)
    fn fun_00607d80(s: GPtr<u8>, ped: GPtr<u8>, ents: GPtr<u8>, n: i32) = thiscall 0x607D80 => unported;
    /// `FUN_006060a0` (0x6060A0)
    fn fun_006060a0(s: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x6060A0 => unported;
    /// `FUN_00603e70` (0x603E70)
    fn fun_00603e70(s: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x603E70 => unported;
    /// `CMentalState::Process(const CPed&)` (0x6008A0)
    fn mental_state_process(m: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x6008A0 => unported;
    /// `CPedGeometryAnalyser::IsInAir(const CPed&)` (0x5F1CB0, cdecl, AL)
    fn geom_is_in_air(ped: GPtr<u8>) -> u8 = cdecl 0x5F1CB0 => unported;
    /// `CEventInAir::Constructor` (0x4B0CB0)
    fn in_air_ctor(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B0CB0 => unported;
    /// `FUN_004b0d00` (0x4B0D00): `CEventInAir` destructor
    fn in_air_dtor(e: GPtr<u8>) = thiscall 0x4B0D00 => unported;
    /// `CEvent::Constructor` (0x4ABFC0)
    fn event_ctor(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x4ABFC0 => unported;
    /// `FUN_004ac010` (0x4AC010): `CEvent` base destructor
    fn event_base_dtor(e: GPtr<u8>) = thiscall 0x4AC010 => unported;
    /// `CWeapon::GenerateDamageEvent(CPed*, CEntity*, eWeaponType, int, ePedPieceTypes, int)` (0x73A530, cdecl)
    fn weapon_generate_damage_event(ped: GPtr<u8>, e: u32, w: u32, dmg: u32, piece: u32, dir: u32) -> u8 = cdecl 0x73A530 => unported;
    /// `FUN_00603bf0` (0x603BF0)
    fn fun_00603bf0(s: GPtr<u8>, ped: GPtr<u8>, ents: GPtr<u8>, n: i32) = thiscall 0x603BF0 => unported;
    /// `CTaskManager::FindTaskByType(int slot, int type)` (0x6817D0)
    fn tm_find_task_by_type(tm: GPtr<u8>, slot: i32, t: i32) -> GPtr<u8> = thiscall 0x6817D0 => unported;
    /// `CEventAreaCodes::Constructor(CPed*)` (0x4B2190)
    fn area_codes_ctor(e: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B2190 => unported;
    /// `FUN_004b2210` (0x4B2210): `CEventAreaCodes` destructor
    fn area_codes_dtor(e: GPtr<u8>) = thiscall 0x4B2210 => unported;
    /// `FUN_004aeac0` (0x4AEAC0): vehicle-event constructor `(CPed*, CVehicle*, int)`
    fn fun_004aeac0(e: GPtr<u8>, ped: GPtr<u8>, v: GPtr<u8>, n: i32) -> GPtr<u8> = thiscall 0x4AEAC0 => unported;
    /// `FUN_004aeb90` (0x4AEB90): its destructor
    fn fun_004aeb90(e: GPtr<u8>) = thiscall 0x4AEB90 => unported;
    /// `CEventDamage::Constructor-1` (0x4AD830)
    fn damage_ctor(e: GPtr<u8>, src: u32, time: u32, w: u32, piece: u32, dir: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x4AD830 => unported;
    /// `CEventGroup::HasEventOfType(const CEvent*)` (0x4AB5E0, AL)
    fn event_group_has_event_of_type(grp: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x4AB5E0 => unported;
    /// `FUN_004ad960` (0x4AD960): `CEventDamage` destructor
    fn damage_dtor(e: GPtr<u8>) = thiscall 0x4AD960 => unported;
    /// `CTask::MakeAbortable(CPed*, int, const CEvent*)` (vtable +0x18, AL)
    fn task_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, e: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;

    /// scalar deleting destructor (vtable +0)
    fn ev_delete(ev: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `CEvent::GetEventType` (vtable +4)
    fn ev_get_event_type(ev: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CEvent::Clone` (vtable +0x10)
    fn ev_clone(ev: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x10 => unported;
    /// `CEvent::GetSourceEntity` (vtable +0x28)
    fn ev_get_source_entity(ev: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x28 => unported;
    /// `CEvent::DoInformVehicleOccupants(CPed*)` (vtable +0x34)
    fn ev_vslot_34(ev: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall vslot 0x34 => unported;
    /// `CEventEditableResponse::CloneEditable` (vtable +0x40)
    fn ev_clone_editable(ev: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x40 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn task_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
}

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;

/// The event pool (`CPools::ms_pEventPool`).
fn event_pool() -> GPtr<u8> {
    g::<GPtr<u8>>(EVENT_POOL).read()
}

/// `fld dword [addr]` of a float constant/global.
fn gf(addr: usize) -> X87 {
    x(g::<f32>(addr).read())
}

/// Pointer to a Rust stack buffer (stands in for an object the original builds on its own stack).
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// The ped's intelligence (`+0x47C`).
fn intel(ped: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(ped, 0x47C)
}

/// Entity position: matrix +0x30, else the placement at +4.
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// Copies a 12-byte vector dword by dword.
fn copy_vec(dst: GPtr<u8>, src: GPtr<u8>) {
    for o in [0, 4, 8] {
        wr::<u32>(dst, o, rd::<u32>(src, o));
    }
}

// ---- informers ----------------------------------------------------------------------------------

/// `CEventEditableResponse::Clone` (0x420ED0): `CloneEditable()` (vtable +0x40), then copies
/// +0xE, +0x10 (words) and +0xC (byte) into the clone (not null-checked, as the original).
#[hook(addr = 0x420ED0, conv = thiscall, name = "CEventEditableResponse::Clone", expect = "56 8B F1 8B 06")]
pub fn event_editable_response_clone_420ed0(this: GPtr<u8>) -> GPtr<u8> {
    let e = ev_clone_editable(this);
    wr::<u16>(e, 0xE, rd::<u16>(this, 0xE));
    wr::<u16>(e, 0x10, rd::<u16>(this, 0x10));
    wr::<u8>(e, 0xC, rd::<u8>(this, 0xC));
    e
}

/// `CEventGlobalGroup::AddEventsToPed(CPed*)` (0x4AB9C0, `ret 4`): for each of the `+8` events at
/// `+0xC..`, `Clone()`, add it to the ped's event group (`intel + 0x68`, `Add(e, false)`), then delete
/// the clone when non-null. The count is re-read every iteration; the group pointer once.
#[hook(addr = 0x4AB9C0, conv = thiscall, name = "CEventGlobalGroup::AddEventsToPed", expect = "8B 44 24 04 8B 80 7C 04 00 00")]
pub fn event_global_group_add_events_to_ped_4ab9c0(this: GPtr<u8>, ped: GPtr<u8>) {
    let grp = intel(ped);
    if rd::<i32>(this, 8) <= 0 {
        return;
    }
    let grp = grp.byte_add(0x68);
    let mut i = 0i32;
    loop {
        let c = ev_clone(rd::<GPtr<u8>>(this, 0xC + 4 * i as isize));
        event_group_add(grp, c, 0);
        if !c.is_null() {
            ev_delete(c, 1);
        }
        i += 1;
        if i >= rd::<i32>(this, 8) {
            break;
        }
    }
}

/// Clones `this` for an informed ped: `Clone()`, `+0xE = 200`, `+0xC = 0` (not null-checked), adds it
/// to `ped`'s event group and deletes the clone.
fn inform_occupant(this: GPtr<u8>, p: GPtr<u8>) {
    let c = ev_clone(this);
    wr::<u16>(c, 0xE, 200);
    wr::<u8>(c, 0xC, 0);
    event_group_add(intel(p).byte_add(0x68), c, 0);
    ev_delete(c, 1);
}

/// `CEventEditableResponse::InformVehicleOccupants(CPed*)` (0x4AC4A0, `ret 4`): when the ped is in a
/// vehicle (`+0x46C` bit 8), `vtable +0x34(ped)` holds and `+0xC` is set, every other occupant
/// (driver `+0x460`, then passengers `+0x464[i]` for `i < +0x488` (byte, re-read)) gets a clone.
/// Always returns AL = 1.
#[hook(addr = 0x4AC4A0, conv = thiscall, name = "CEventEditableResponse::InformVehicleOccupants", expect = "83 EC 08 53 55 56")]
pub fn event_editable_response_inform_vehicle_occupants_4ac4a0(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 || ev_vslot_34(this, ped) == 0 || rd::<u8>(this, 0xC) == 0 {
        return 1;
    }
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    let d = rd::<GPtr<u8>>(veh, 0x460);
    if !d.is_null() && d != ped {
        inform_occupant(this, d);
    }
    if rd::<u8>(veh, 0x488) == 0 {
        return 1;
    }
    let mut i = 0i32;
    loop {
        let p = rd::<GPtr<u8>>(veh, 0x464 + 4 * i as isize);
        if !p.is_null() && p != ped {
            inform_occupant(this, p);
        }
        i += 1;
        if i >= i32::from(rd::<u8>(veh, 0x488)) {
            break;
        }
    }
    1
}

/// `CEventEditableResponse::InformRespectedFriends(CPed*)` (0x4B2B00, `ret 4`): when `+0xC` is set
/// and the ped's intelligence has a non-zero "friends to inform" count (`+0xC4`), walks the 16
/// nearby peds (`intel +0x130`) while fewer than that many were informed: cops (type 6) inform
/// cops while the player's wanted level (`FindPlayerPed(0)->+0x480->[0]->+0x2C`) is 0; a player ped
/// informs peds whose acquaintances(0) respect its ped type; others inform peds whose ped-type flag
/// is in the ped's respect set. Peds within `intel +0xC8` (squared distances compared, x87 order
/// `(dz² + dx²) + dy²`) get a clone (`+0xE = 200`, `+0xC = 0`) through `CInformFriendsEventQueue::Add`;
/// rejected clones are deleted. Returns AL = (count != 0). The original keeps the count in its
/// argument slot.
#[hook(addr = 0x4B2B00, conv = thiscall, name = "CEventEditableResponse::InformRespectedFriends", expect = "83 EC 18 56 8B F1")]
pub fn event_editable_response_inform_respected_friends_4b2b00(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0xC) == 0 {
        return 0;
    }
    let it = intel(ped);
    let max = rd::<i32>(it, 0xC4);
    let range = rd::<f32>(it, 0xC8);
    if max == 0 {
        return 0;
    }
    let respect = acquaintance_get(ped.byte_add(0x4E0), 0);
    let list = intel(ped).byte_add(0x130);
    let mut count = 0i32;
    let mut i = 0isize;
    while count < max {
        let p = rd::<GPtr<u8>>(list, 4 * i);
        if !p.is_null() {
            let ok = if rd::<u32>(ped, 0x598) == 6 {
                if rd::<u32>(p, 0x598) != 6 {
                    false
                } else {
                    let pd = rd::<GPtr<u8>>(find_player_ped(0), 0x480);
                    let w = if pd.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(pd, 0) };
                    rd::<u32>(w, 0x2C) == 0
                }
            } else if ped_is_player(ped) != 0 {
                let a = acquaintance_get(p.byte_add(0x4E0), 0);
                ped_type_get_ped_flag(rd::<u32>(ped, 0x598)) & a != 0
            } else {
                ped_type_get_ped_flag(rd::<u32>(p, 0x598)) & respect != 0
            };
            if ok {
                let b = ent_pos(p);
                let a = ent_pos(ped);
                let dx = x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(b, 0));
                let dy = x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(b, 4));
                let dz = x(rd::<f32>(a, 8)).fsub_m(rd::<f32>(b, 8));
                let d2 = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
                let r2 = x(range).fmul_m(range);
                if !r2.fcom(d2).test_any(0x41) {
                    let c = ev_clone(this);
                    wr::<u16>(c, 0xE, 200);
                    wr::<u8>(c, 0xC, 0);
                    if inform_friends_queue_add(p, c) != 0 {
                        count += 1;
                    } else {
                        ev_delete(c, 1);
                    }
                }
            }
        }
        i += 1;
        if i >= 0x10 {
            break;
        }
    }
    u8::from(count != 0)
}

/// `CEventEditableResponse::InformGroup(CPed*)` (0x4B7DF0, `ret 4`): the ped's group (or AL = 0);
/// `Clone()`, `+0xE = 200`, `+0xC = 0`, `CInformGroupEventQueue::Add(ped, group, clone)`, deleting the
/// clone when refused; AL = 1.
#[hook(addr = 0x4B7DF0, conv = thiscall, name = "CEventEditableResponse::InformGroup", expect = "53 8B 5C 24 08")]
pub fn event_editable_response_inform_group_4b7df0(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let grp = ped_groups_get_peds_group(ped);
    if grp.is_null() {
        return 0;
    }
    let c = ev_clone(this);
    wr::<u16>(c, 0xE, 200);
    wr::<u8>(c, 0xC, 0);
    if inform_group_queue_add(ped, grp, c) == 0 {
        ev_delete(c, 1);
    }
    1
}

// ---- response computation ------------------------------------------------------------------------

/// MSVC passes the `bool` "ped in a group" (`+0x46C` bit 8) by overwriting the low byte of a stack
/// dword (here the argument slot holding `slot`); the callee reads the low byte.
fn bool_in_slot(slot: u32, ped: GPtr<u8>) -> u32 {
    (slot & !0xFF) | ((rd::<u32>(ped, 0x46C) >> 8) & 1)
}

/// `CEventEditableResponse::ComputeResponseTaskType(CPed*, bool)` (0x4B56C0, `ret 8`): when the task
/// (+0xE) is still 200, `MakeDecision(ped, GetEventType(), sourceType, inGroup, 0x6A4, 0x4B0, 0x12C,
/// -1, arg2, &+0xE, &+0x10)`. The bool dword is the ped argument slot with its low byte replaced.
#[hook(addr = 0x4B56C0, conv = thiscall, name = "CEventEditableResponse::ComputeResponseTaskType-ped", expect = "55 56 8B F1 66 81 7E 0E C8 00")]
pub fn event_editable_response_compute_response_task_type_ped_4b56c0(this: GPtr<u8>, ped: GPtr<u8>, b: u32) {
    if rd::<u16>(this, 0xE) != 0xC8 {
        return;
    }
    let ty = ev_get_event_type(this);
    let src = compute_event_source_type(this, ped);
    let bd = bool_in_slot(ped.addr() as u32, ped);
    let dm = decision_maker_types_get_instance();
    make_decision_ped(dm, ped, ty, src, bd, 0x6A4, 0x4B0, 0x12C, 0xFFFF_FFFF, b, this.byte_add(0xE), this.byte_add(0x10));
}

/// `CEventEditableResponse::ComputeResponseTaskOfType(CPed*, int)` (0x4B5730, `ret 8`):
/// `MakeDecision(ped, GetEventType(), sourceType, inGroup, -1, -1, -1, type, 0, &task, &facial)` into
/// two stack words (zeroed here, uninitialised in the original); AL = `type == (int16) task`.
#[hook(addr = 0x4B5730, conv = thiscall, name = "CEventEditableResponse::ComputeResponseTaskOfType", expect = "83 EC 08 53 56 8B F1")]
pub fn event_editable_response_compute_response_task_of_type_4b5730(this: GPtr<u8>, ped: GPtr<u8>, ty: i32) -> u8 {
    let et = ev_get_event_type(this);
    let src = compute_event_source_type(this, ped);
    let bd = bool_in_slot(ped.addr() as u32, ped);
    let mut l = [0u32; 2];
    let p = buf(&mut l);
    let dm = decision_maker_types_get_instance();
    make_decision_ped(dm, ped, et, src, bd, 0xFFFF_FFFF, 0xFFFF_FFFF, 0xFFFF_FFFF, ty as u32, 0, p, p.byte_add(4));
    u8::from(ty == i32::from(rd::<i16>(p, 0)))
}

/// `CEventEditableResponse::ComputeResponseTaskType(CPedGroup*)` (0x4B57A0, `ret 4`): when +0xE is
/// 200: the deciding ped is the leader unless it is a player, else the first of members 0..6; with
/// one, `+0xE = MakeDecision(group, GetEventType(), sourceType(ped), inGroup(ped), 0x4B0, 0x6A4,
/// 0x12C, -1)`, else `+0xE = 200`. Still 200 and the leader (asked again each time) is a player →
/// the same decision from the leader. The bool dword is a stack local whose upper bytes are
/// uninitialised in the original (zero here).
#[hook(addr = 0x4B57A0, conv = thiscall, name = "CEventEditableResponse::ComputeResponseTaskType-group", expect = "83 EC 08 57 8B F9")]
pub fn event_editable_response_compute_response_task_type_group_4b57a0(this: GPtr<u8>, grp: GPtr<u8>) {
    if rd::<u16>(this, 0xE) != 0xC8 {
        return;
    }
    let ty = ev_get_event_type(this);
    let m = grp.byte_add(8);
    let mut p = membership_get_leader(m);
    if !p.is_null() && ped_is_player(p) != 0 {
        p = GPtr::NULL;
    }
    if p.is_null() {
        let mut i = 0;
        while i < 7 {
            p = membership_get_member(m, i);
            i += 1;
            if !p.is_null() {
                break;
            }
        }
    }
    let mut bslot = 0u32;
    if p.is_null() {
        wr::<u16>(this, 0xE, 0xC8);
    } else {
        let src = compute_event_source_type(this, p);
        bslot = bool_in_slot(bslot, p);
        let dm = decision_maker_types_get_instance();
        let t = make_decision_group(dm, grp, ty, src, bslot, 0x4B0, 0x6A4, 0x12C, 0xFFFF_FFFF);
        wr::<u16>(this, 0xE, t as u16);
    }
    if rd::<u16>(this, 0xE) != 0xC8 {
        return;
    }
    if membership_get_leader(m).is_null() || ped_is_player(membership_get_leader(m)) == 0 {
        return;
    }
    let l = membership_get_leader(m);
    let src = compute_event_source_type(this, l);
    bslot = bool_in_slot(bslot, l);
    let dm = decision_maker_types_get_instance();
    let t = make_decision_group(dm, grp, ty, src, bslot, 0x4B0, 0x6A4, 0x12C, 0xFFFF_FFFF);
    wr::<u16>(this, 0xE, t as u16);
}

// ---- pool, clones, constructors ------------------------------------------------------------------

/// `CEventPool::New` (0x4B5570): `CPool<CEvent>::New` (slot 0x44): scans from `firstFree + 1`
/// (stored back every step); at the end it wraps once to 0 (the store goes through the straight
/// protection stub 0x4014B5), a second wrap returns null. The found slot's flag byte: free bit
/// cleared, then the 7-bit reference id incremented (two separate read-modify-writes, as the original).
#[hook(addr = 0x4B5570, conv = thiscall, name = "CEventPool::New", expect = "8B 51 08 56 32 C0")]
pub fn event_pool_new_4b5570(pool: GPtr<u8>) -> GPtr<u8> {
    let size = rd::<i32>(pool, 8);
    let mut wrapped = false;
    loop {
        let i = rd::<i32>(pool, 0xC).wrapping_add(1);
        wr::<i32>(pool, 0xC, i);
        if i == size {
            wr::<i32>(pool, 0xC, 0);
            if wrapped {
                return GPtr::NULL;
            }
            wrapped = true;
        }
        let flags = rd::<GPtr<u8>>(pool, 4);
        let idx = rd::<i32>(pool, 0xC) as isize;
        if rd::<u8>(flags, idx) & 0x80 != 0 {
            let f = at::<u8>(flags, idx);
            f.write(f.read() & 0x7F);
            let f = at::<u8>(rd::<GPtr<u8>>(pool, 4), rd::<i32>(pool, 0xC) as isize);
            let b = f.read();
            f.write(b ^ ((b.wrapping_add(1) ^ b) & 0x7F));
            let idx = rd::<i32>(pool, 0xC);
            return rd::<GPtr<u8>>(pool, 0).byte_add(idx.wrapping_mul(0x44) as isize);
        }
    }
}

/// `CEventScriptCommand::Clone` (0x4B6490): `vtable +0x40` (the task clone) first, then the inlined
/// pool `New`; on success the `CEventScriptCommand` ctor inlined (`+0x10 = task`, `+0xC`, `+4 = 0`,
/// `+8 = 0`, `vtbl = 0x85B378`, byte `+0x14`). A failed `New` leaks the task, as the original.
#[hook(addr = 0x4B6490, conv = thiscall, name = "CEventScriptCommand::Clone", expect = "56 8B F1 8B 06 57")]
pub fn event_script_command_clone_4b6490(this: GPtr<u8>) -> GPtr<u8> {
    let t = ev_clone_editable(this);
    let e = event_pool_new(event_pool());
    if e.is_null() {
        return GPtr::NULL;
    }
    let b = rd::<u8>(this, 0x14);
    wr::<u32>(e, 0x10, t.addr() as u32);
    wr::<u32>(e, 0xC, rd::<u32>(this, 0xC));
    wr::<u32>(e, 4, 0);
    wr::<u8>(e, 8, 0);
    wr::<u32>(e, 0, 0x85B378);
    wr::<u8>(e, 0x14, b);
    e
}

/// `CEventPedCollisionWithPed::Constructor` (0x4AC990, `ret 0x1C`): `+4 = 0`, `+8 = 0`,
/// `+0xC = (u16) a`, `+0x10 = f`, `vtbl = 0x85AC68`, `+0x14 = ped`, vectors `+0x18`/`+0x24` copied
/// dword by dword, `+0x30`/`+0x32` words; registers the ped reference when set.
#[hook(addr = 0x4AC990, conv = thiscall, name = "CEventPedCollisionWithPed::Constructor", expect = "6A FF 68 C8 99 83 00")]
#[allow(clippy::too_many_arguments)]
pub fn event_ped_collision_with_ped_ctor_4ac990(this: GPtr<u8>, a: u32, f: u32, ped: GPtr<u8>, v1: GPtr<u8>, v2: GPtr<u8>, p30: u32, p32: u32) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u16>(this, 0xC, a as u16);
    wr::<u32>(this, 0x10, f);
    wr::<u32>(this, 0, 0x85AC68);
    wr::<u32>(this, 0x14, ped.addr() as u32);
    copy_vec(this.byte_add(0x18), v1);
    copy_vec(this.byte_add(0x24), v2);
    wr::<u16>(this, 0x30, p30 as u16);
    wr::<u16>(this, 0x32, p32 as u16);
    if !ped.is_null() {
        entity_register_reference(ped, this.byte_add(0x14));
    }
    this
}

/// `CEventBuildingCollision::Constructor` (0x4ACF00, `ret 0x18`): `+4 = 0`, `+8 = 0`,
/// `+0xC = (u16) a`, `+0xE = (u16) e`, `+0x10 = f`, `vtbl = 0x85ACE8`, `+0x14 = building`, vectors
/// `+0x18`/`+0x24` copied dword by dword; registers the building reference when set.
#[hook(addr = 0x4ACF00, conv = thiscall, name = "CEventBuildingCollision::Constructor", expect = "6A FF 68 48 9A 83 00")]
pub fn event_building_collision_ctor_4acf00(this: GPtr<u8>, a: u32, f: u32, b: GPtr<u8>, v1: GPtr<u8>, v2: GPtr<u8>, e: u32) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u16>(this, 0xC, a as u16);
    wr::<u16>(this, 0xE, e as u16);
    wr::<u32>(this, 0x10, f);
    wr::<u32>(this, 0, 0x85ACE8);
    wr::<u32>(this, 0x14, b.addr() as u32);
    copy_vec(this.byte_add(0x18), v1);
    copy_vec(this.byte_add(0x24), v2);
    if !b.is_null() {
        entity_register_reference(b, this.byte_add(0x14));
    }
    this
}

/// `CEventSoundQuiet::Constructor(CEntity*, float, int, const CVector&)` (0x5E05B0, `ret 0x10`;
/// entry `push -1; jmp` to a straight protection stub that pushes the SEH handler):
/// `CEventEditableResponse` ctor, `+0x18 = level`, `+0x1C = time`, `vtbl = 0x86C2B0`,
/// `+0x14 = entity`, `+0x20 = pos`; registers the entity reference when set; time -1 → now, and the
/// position from the entity (matrix +0x30 or placement; the entity is not null-checked).
#[hook(addr = 0x5E05B0, conv = thiscall, name = "CEventSoundQuiet::Constructor", expect = "6A FF E9 EE 3A E2 FF")]
pub fn event_sound_quiet_ctor_5e05b0(this: GPtr<u8>, ent: GPtr<u8>, level: u32, time: i32, pos: GPtr<u8>) -> GPtr<u8> {
    editable_response_ctor(this);
    wr::<u32>(this, 0x18, level);
    wr::<i32>(this, 0x1C, time);
    wr::<u32>(this, 0, 0x86C2B0);
    wr::<u32>(this, 0x14, ent.addr() as u32);
    copy_vec(this.byte_add(0x20), pos);
    if !ent.is_null() {
        entity_register_reference(ent, this.byte_add(0x14));
    }
    if rd::<i32>(this, 0x1C) == -1 {
        let e = rd::<GPtr<u8>>(this, 0x14);
        wr::<u32>(this, 0x1C, g::<u32>(TIME_MS).read());
        copy_vec(this.byte_add(0x20), ent_pos(e));
    }
    this
}

// ---- AffectsPed / priorities ---------------------------------------------------------------------

/// `CEventPedCollisionWithPed::AffectsPed(CPed*)` (0x4ACB10, `ret 4`, AL): alive, not attached
/// (`+0xFC`), the other ped (+0x14) set, neither in a vehicle (`+0x46C` bit 8), the other's
/// `intel +0x274 <= [0x86C938]`, not threatened by it; a move state (+0x30) below 4 needs the other
/// to lead its group; `DotProduct(&+0x18, ped forward) <= [0x858F40]` (NaN passes); both doing
/// the same task of type 0x4B2/0x4B3 (other's type asked first) → false; active task 0x395 aimed at
/// the other (`GetActiveTask` called three times) → false; a 0x3E8 task on the other plus a 0x3FB
/// task → false. The other ped is re-read from +0x14 at every use.
#[hook(addr = 0x4ACB10, conv = thiscall, name = "CEventPedCollisionWithPed::AffectsPed", expect = "53 56 8B 74 24 0C")]
pub fn event_ped_collision_with_ped_affects_ped_4acb10(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if ped_is_alive(ped) == 0 || rd::<u32>(ped, 0xFC) != 0 {
        return 0;
    }
    let o = rd::<GPtr<u8>>(this, 0x14);
    if o.is_null() || rd::<u32>(ped, 0x46C) & 0x100 != 0 || rd::<u32>(o, 0x46C) & 0x100 != 0 {
        return 0;
    }
    if rd::<i32>(intel(o), 0x274) > g::<i32>(0x86C938).read() {
        return 0;
    }
    if intel_is_threatened_by(intel(ped), o) != 0 {
        return 0;
    }
    if rd::<i16>(this, 0x30) < 4 {
        let grp = ped_groups_get_peds_group(rd::<GPtr<u8>>(this, 0x14));
        if grp.is_null() || membership_is_leader(grp.byte_add(8), rd::<GPtr<u8>>(this, 0x14)) == 0 {
            return 0;
        }
    }
    let d = vector_dot_product(this.byte_add(0x18), rd::<GPtr<u8>>(ped, 0x14).byte_add(0x10));
    if !d.fcom(gf(0x858F40)).test_any(0x41) {
        return 0;
    }
    let mut t = tm_find_active_task_by_type(intel(ped).byte_add(4), 0x4B2);
    if t.is_null() {
        t = tm_find_active_task_by_type(intel(ped).byte_add(4), 0x4B3);
    }
    if !t.is_null() {
        let oi = intel(rd::<GPtr<u8>>(this, 0x14)).byte_add(4);
        let mut ot = tm_find_active_task_by_type(oi, 0x4B2);
        if ot.is_null() {
            ot = tm_find_active_task_by_type(intel(rd::<GPtr<u8>>(this, 0x14)).byte_add(4), 0x4B3);
        }
        if !ot.is_null() {
            let oty = task_get_task_type(ot);
            if task_get_task_type(t) == oty {
                return 0;
            }
        }
    }
    if !tm_get_active_task(intel(ped).byte_add(4)).is_null()
        && task_get_task_type(tm_get_active_task(intel(ped).byte_add(4))) == 0x395
        && rd::<GPtr<u8>>(tm_get_active_task(intel(ped).byte_add(4)), 0xC) == rd::<GPtr<u8>>(this, 0x14)
    {
        return 0;
    }
    let t = tm_find_active_task_by_type(intel(ped).byte_add(4), 0x3E8);
    if !t.is_null()
        && rd::<GPtr<u8>>(t, 0x10) == rd::<GPtr<u8>>(this, 0x14)
        && !tm_find_active_task_by_type(intel(ped).byte_add(4), 0x3FB).is_null()
    {
        return 0;
    }
    1
}

/// `CEventBuildingCollision::AffectsPed(CPed*)` (0x4AD070, `ret 4`, AL): not a player, alive, move
/// state (+0xE) != 1; impact speed (+0x20) above `[0x86C8F4]` needs the building's `+0x1C` bit 22;
/// not attached; `d = normalised (n.x, n.y, 0) · ped forward` (x87 order `y·fy + z·fz + x·fx`, kept
/// as f32 on the stack) must not exceed `[0x85AD28]`; a current event 6 or 0x3C about the same
/// building needs `d <= [0x858F40]`; no active task 0x10B; and the current event type != 0x1F.
#[hook(addr = 0x4AD070, conv = thiscall, name = "CEventBuildingCollision::AffectsPed", expect = "83 EC 0C 56 8B 74 24 14")]
pub fn event_building_collision_affects_ped_4ad070(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if ped_is_player(ped) != 0 || ped_is_alive(ped) == 0 || rd::<u16>(this, 0xE) == 1 {
        return 0;
    }
    if !x(rd::<f32>(this, 0x20)).fcom(gf(0x86C8F4)).test_any(0x41)
        && rd::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x1C) & 0x400000 == 0
    {
        return 0;
    }
    if rd::<u32>(ped, 0xFC) != 0 {
        return 0;
    }
    let mut v = [rd::<u32>(this, 0x18), rd::<u32>(this, 0x1C), 0u32];
    let vp = buf(&mut v);
    vector_normalise(vp);
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let dot = x(rd::<f32>(vp, 4))
        .fmul_m(rd::<f32>(m, 0x14))
        .fadd(x(rd::<f32>(vp, 8)).fmul_m(rd::<f32>(m, 0x18)))
        .fadd(x(rd::<f32>(vp, 0)).fmul_m(rd::<f32>(m, 0x10)));
    let d = f32_of(dot);
    if !dot.fcom(gf(0x85AD28)).test_any(0x41) {
        return 0;
    }
    for ty in [6, 0x3C] {
        if handler_get_current_event_type(intel(ped).byte_add(0x34)) == ty {
            let ce = history_get_current_event(intel(ped).byte_add(0x38));
            if rd::<u32>(ce, 0x14) == rd::<u32>(this, 0x14) && !x(d).fcom(gf(0x858F40)).test_any(0x41) {
                return 0;
            }
        }
    }
    if !tm_find_active_task_by_type(intel(ped).byte_add(4), 0x10B).is_null() {
        return 0;
    }
    u8::from(handler_get_current_event_type(intel(ped).byte_add(0x34)) != 0x1F)
}

/// `CEventScriptCommand::GetEventPriority` (0x4B0B20, EAX): `+0x14` set → 0x4B; no task (+0x10) →
/// 0x35; task type 0x191 with flag `+0xC` bit 1, or 0xD4 / 0x518 / 0x640 / 0xD9 → 0x47; else 0x35.
/// `GetTaskType` is asked again for every comparison (task re-read from +0x10), as the original.
#[hook(addr = 0x4B0B20, conv = thiscall, name = "CEventScriptCommand::GetEventPriority", expect = "56 8B F1 8A 46 14")]
pub fn event_script_command_get_event_priority_4b0b20(this: GPtr<u8>) -> u32 {
    if rd::<u8>(this, 0x14) != 0 {
        return 0x4B;
    }
    if rd::<GPtr<u8>>(this, 0x10).is_null() {
        return 0x35;
    }
    let t = || rd::<GPtr<u8>>(this, 0x10);
    if task_get_task_type(t()) == 0x191 && rd::<u8>(t(), 0xC) & 2 != 0 {
        return 0x47;
    }
    if task_get_task_type(t()) == 0xD4 || task_get_task_type(t()) == 0x518 || task_get_task_type(t()) == 0x640 {
        return 0x47;
    }
    if task_get_task_type(t()) == 0xD9 {
        0x47
    } else {
        0x35
    }
}

/// `CEventSoundQuiet::AffectsPed(CPed*)` (0x4B5240, `ret 4`, AL): not a player, alive, a source
/// entity; `level = GetSoundLevel(null, pedPos)` (kept as f32 in the argument slot) must not be below
/// `[0x858CA4]`. When the ped's current event is also a 0x3E from the same source: if it is at least
/// 2000 ms older and `|pos - other.pos|² >= [0x858624]` → true; else true unless
/// `level - other.GetSoundLevel(null, pedPos) < [0x858B3C]`. Comparisons pass on NaN (`test ah, 5; jp`).
#[hook(addr = 0x4B5240, conv = thiscall, name = "CEventSoundQuiet::AffectsPed", expect = "83 EC 0C 53 57")]
pub fn event_sound_quiet_affects_ped_4b5240(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if ped_is_player(ped) != 0 || ped_is_alive(ped) == 0 || ev_get_source_entity(this).is_null() {
        return 0;
    }
    let l = event_get_sound_level(this, GPtr::NULL, ent_pos(ped));
    let level = f32_of(l);
    if !l.fcom(gf(0x858CA4)).test_parity(5) {
        return 0;
    }
    let o = history_get_current_event(intel(ped).byte_add(0x38));
    if o.is_null() || ev_get_event_type(o) != 0x3E {
        return 1;
    }
    let os = ev_get_source_entity(o);
    if os != ev_get_source_entity(this) {
        return 1;
    }
    let mut v = [0u32; 3];
    let vp = buf(&mut v);
    vector_difference(vp, this.byte_add(0x20), o.byte_add(0x20));
    if rd::<i32>(this, 0x1C).wrapping_sub(rd::<i32>(o, 0x1C)) >= 0x7D0
        && vector_magnitude_sqr(vp).fcom(gf(0x858624)).test_parity(5)
    {
        return 1;
    }
    let ol = event_get_sound_level(o, GPtr::NULL, ent_pos(ped));
    if ol.fsubr_m(level).fcom(gf(0x858B3C)).test_parity(5) {
        return 1;
    }
    0
}

/// `CEventObjectCollision::Constructor` (0x4ACCF0, `ret 0x18`): as the building one with
/// `vtbl = 0x85ACA8` (`+0xE = (u16) e`, `+0x14 = object`, reference registered when set).
#[hook(addr = 0x4ACCF0, conv = thiscall, name = "CEventObjectCollision::Constructor", expect = "6A FF 68 08 9A 83 00")]
pub fn event_object_collision_ctor_4accf0(this: GPtr<u8>, a: u32, f: u32, o: GPtr<u8>, v1: GPtr<u8>, v2: GPtr<u8>, e: u32) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u16>(this, 0xC, a as u16);
    wr::<u16>(this, 0xE, e as u16);
    wr::<u32>(this, 0x10, f);
    wr::<u32>(this, 0, 0x85ACA8);
    wr::<u32>(this, 0x14, o.addr() as u32);
    copy_vec(this.byte_add(0x18), v1);
    copy_vec(this.byte_add(0x24), v2);
    if !o.is_null() {
        entity_register_reference(o, this.byte_add(0x14));
    }
    this
}

/// `CEventDamage::CEventDamage(CEntity*, uint32 time, eWeaponType, ePedPieceTypes, uint8 dir,
/// bool a, bool b)` (0x4AD830, `ret 0x1C`; entry `push -1; jmp` to a straight protection stub that
/// pushes the SEH handler): editable-response fields (`+0xC = 1`, `+0xE = 200`, `+0x10 = -1`),
/// `+0x14..+0x24` from the arguments, `+0x25 = (old & 0xE0) | b << 3 | a`, `vtbl = 0x85AD78`,
/// `+0x28 = 0`, `+0x2C = 0xBF`, `+0x30 = 8.0`, `+0x34 = 1.0`, `+0x38..+0x43 = 0`; registers the
/// source; finally `+0x25 |= 8` (so bit 3 always ends set).
#[hook(addr = 0x4AD830, conv = thiscall, name = "CEventDamage::Constructor-1", expect = "6A FF E9 EE 9F F5 FF")]
#[allow(clippy::too_many_arguments)]
pub fn event_damage_ctor_4ad830(this: GPtr<u8>, src: GPtr<u8>, time: u32, weapon: u32, piece: u32, dir: u32, a: u32, b: u32) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u8>(this, 0xC, 1);
    wr::<u16>(this, 0xE, 0xC8);
    wr::<u16>(this, 0x10, 0xFFFF);
    wr::<u32>(this, 0x14, src.addr() as u32);
    wr::<u32>(this, 0x18, time);
    wr::<u32>(this, 0x1C, weapon);
    wr::<u32>(this, 0x20, piece);
    wr::<u8>(this, 0x24, dir as u8);
    let f = ((b as u8 & 1) << 3) | (rd::<u8>(this, 0x25) & 0xE0) | (a as u8 & 1);
    wr::<u32>(this, 0, 0x85AD78);
    wr::<u32>(this, 0x28, 0);
    wr::<u32>(this, 0x2C, 0xBF);
    wr::<u32>(this, 0x30, 0x4100_0000);
    wr::<u32>(this, 0x34, 0x3F80_0000);
    wr::<u8>(this, 0x25, f);
    wr::<u32>(this, 0x38, 0);
    wr::<u32>(this, 0x3C, 0);
    for o in 0x40..0x44 {
        wr::<u8>(this, o, 0);
    }
    let s = rd::<GPtr<u8>>(this, 0x14);
    if !s.is_null() {
        entity_register_reference(s, this.byte_add(0x14));
    }
    wr::<u8>(this, 0x25, rd::<u8>(this, 0x25) | 8);
    this
}

/// Copies bit `m` of `other +0x25` into `this +0x25` (`this ^ ((this ^ other) & m)`).
fn merge_bit(this: GPtr<u8>, other: GPtr<u8>, m: u8) {
    let t = rd::<u8>(this, 0x25);
    wr::<u8>(this, 0x25, t ^ ((t ^ rd::<u8>(other, 0x25)) & m));
}

/// `CEventDamage::From(const CEventDamage&)` (0x4AD9C0, `ret 4`): `+0x14 = other.+0x14` (registered
/// when set, the old reference is not cleaned up), `+0x18..+0x24`, then the `+0x25` bits 0, 2, 1, 3,
/// 4 one at a time around the copies of `+0x38..+0x43` and `+0x28..+0x34` (the original's order).
#[hook(addr = 0x4AD9C0, conv = thiscall, name = "CEventDamage::From", expect = "56 57 8B 7C 24 0C")]
pub fn event_damage_from_4ad9c0(this: GPtr<u8>, other: GPtr<u8>) {
    let s = rd::<GPtr<u8>>(other, 0x14);
    wr::<u32>(this, 0x14, s.addr() as u32);
    if !s.is_null() {
        entity_register_reference(s, this.byte_add(0x14));
    }
    for o in [0x18, 0x1C, 0x20] {
        wr::<u32>(this, o, rd::<u32>(other, o));
    }
    wr::<u8>(this, 0x24, rd::<u8>(other, 0x24));
    merge_bit(this, other, 1);
    copy_vec(this.byte_add(0x38), other.byte_add(0x38));
    for o in [0x28, 0x2C, 0x30, 0x34] {
        wr::<u32>(this, o, rd::<u32>(other, o));
    }
    for m in [4, 2, 8, 0x10] {
        merge_bit(this, other, m);
    }
}

/// `CEventDamage::IsCriminalEvent` (0x4ADA90, AL): the source (+0x14) is a player ped, or a vehicle
/// whose driver is `FindPlayerPed(-1)` (called before the source is re-read).
#[hook(addr = 0x4ADA90, conv = thiscall, name = "CEventDamage::IsCriminalEvent", expect = "56 8B F1 8B 4E 14")]
pub fn event_damage_is_criminal_event_4ada90(this: GPtr<u8>) -> u8 {
    let s = rd::<GPtr<u8>>(this, 0x14);
    if s.is_null() {
        return 0;
    }
    if rd::<u8>(s, 0x36) & 7 == 3 && ped_is_player(s) != 0 {
        return 1;
    }
    if rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 0x36) & 7 != 2 {
        return 0;
    }
    let p = find_player_ped(-1);
    u8::from(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x14), 0x460) == p)
}

/// `CEventDamage::TakesPriorityOver(const CEvent&)` (0x4ADB00, `ret 4`, AL): `+0x40` and `+0xC` set
/// beat events 0x42, 0x3A and 8 (type asked for each); damage by a player ped: another damage
/// event (9) from a different source, or `FUN_004abca0()`, wins, else `CEvent::TakesPriorityOver`;
/// otherwise a damage event from another source (other's `GetSourceEntity` first) loses to this, and
/// the rest compare priorities (`this >= other`, other's asked first).
#[hook(addr = 0x4ADB00, conv = thiscall, name = "CEventDamage::TakesPriorityOver", expect = "56 57 8B 7C 24 0C")]
pub fn event_damage_takes_priority_over_4adb00(this: GPtr<u8>, other: GPtr<u8>) -> u8 {
    for ty in [0x42, 0x3A, 8] {
        if ev_get_event_type(other) == ty && rd::<u8>(this, 0x40) != 0 && rd::<u8>(this, 0xC) != 0 {
            return 1;
        }
    }
    let s = rd::<GPtr<u8>>(this, 0x14);
    if !s.is_null() && rd::<u8>(s, 0x36) & 7 == 3 && ped_is_player(s) != 0 && ev_get_event_type(other) == 9 {
        if ev_get_source_entity(other) != rd::<GPtr<u8>>(this, 0x14) {
            return 1;
        }
        if fun_004abca0(this) != 0 {
            return 1;
        }
        return event_takes_priority_over(this, other);
    }
    if ev_get_event_type(other) == 9 {
        let os = ev_get_source_entity(other);
        if ev_get_source_entity(this) != os {
            return 1;
        }
    }
    let op = ev_get_event_priority(other);
    u8::from(ev_get_event_priority(this) >= op)
}

/// `CEventDamage::ComputeBodyPartToRemove(int& part)` (0x4ADC10, `ret 4`): `part = 0`; when `+0x41`
/// (headshot-capable) is set, by weapon (+0x1C, switch tables 0x4ADD0C/0x4ADCFC read from the exe):
/// 0..15 and 46 → 2; 16, 35, 36, 51 → one of `{3, 8, 2, 4, 7}` by
/// `GetRandomNumberInRange(0, 5)` (a stack array indexed by the result); 24..27, 30, 31, 33, 34,
/// 38 → by piece (+0x20) 5→3, 6→4, 7→7, 8→8, 9→2 (table 0x4ADD40); others nothing.
#[hook(addr = 0x4ADC10, conv = thiscall, name = "CEventDamage::ComputeBodyPartToRemove", expect = "83 EC 14 56 8B 74 24 1C")]
#[allow(clippy::collapsible_match)]
pub fn event_damage_compute_body_part_to_remove_4adc10(this: GPtr<u8>, part: GPtr<u8>) {
    wr::<u32>(part, 0, 0);
    match rd::<u32>(this, 0x1C) {
        0..=15 | 46 => {
            if rd::<u8>(this, 0x41) != 0 {
                wr::<u32>(part, 0, 2);
            }
        }
        16 | 35 | 36 | 51 => {
            if rd::<u8>(this, 0x41) != 0 {
                let mut a = [3u32, 8, 2, 4, 7];
                let r = general_get_random_number_in_range_int(0, 5);
                let v = rd::<u32>(buf(&mut a), 4 * r as isize);
                wr::<u32>(part, 0, v);
            }
        }
        24..=27 | 30 | 31 | 33 | 34 | 38 => {
            if rd::<u8>(this, 0x41) != 0 {
                let v = match rd::<u32>(this, 0x20).wrapping_sub(5) {
                    0 => 3,
                    1 => 4,
                    2 => 7,
                    3 => 8,
                    4 => 2,
                    _ => return,
                };
                wr::<u32>(part, 0, v);
            }
        }
        _ => {}
    }
}

/// `CEventDamage::WasHeadShot(bool& out)` (0x4ADD60, `ret 4`): `out = 0`; with `+0x41` set, weapons
/// 0..15 and 46 → 1; 22..34, 38 and 52 → 1 when the piece (+0x20) is 9 (switch tables
/// 0x4ADDAC/0x4ADDA0 read from the exe).
#[hook(addr = 0x4ADD60, conv = thiscall, name = "CEventDamage::WasHeadShot", expect = "8B 54 24 04 C6 02 00")]
#[allow(clippy::collapsible_match)]
pub fn event_damage_was_head_shot_4add60(this: GPtr<u8>, out: GPtr<u8>) {
    wr::<u8>(out, 0, 0);
    match rd::<u32>(this, 0x1C) {
        0..=15 | 46 => {
            if rd::<u8>(this, 0x41) != 0 {
                wr::<u8>(out, 0, 1);
            }
        }
        22..=34 | 38 | 52 => {
            if rd::<u8>(this, 0x41) != 0 && rd::<u32>(this, 0x20) == 9 {
                wr::<u8>(out, 0, 1);
            }
        }
        _ => {}
    }
}

/// `CEventDeadPed::CEventDeadPed(CPed*, bool, uint32)` (0x4ADEA0, `ret 0xC`): editable-response
/// fields (`+0xC = 1`, `+0xE = 200`, `+0x10 = -1`), `vtbl = 0x85AE00`, `+0x14 = ped`, byte `+0x18`,
/// `+0x1C`; registers the ped when set.
#[hook(addr = 0x4ADEA0, conv = thiscall, name = "CEventDeadPed::CEventDeadPed", expect = "6A FF 68 08 9B 83 00")]
pub fn event_dead_ped_ctor_4adea0(this: GPtr<u8>, ped: GPtr<u8>, b: u32, c: u32) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u8>(this, 0xC, 1);
    wr::<u16>(this, 0xE, 0xC8);
    wr::<u16>(this, 0x10, 0xFFFF);
    wr::<u32>(this, 0, 0x85AE00);
    wr::<u32>(this, 0x14, ped.addr() as u32);
    wr::<u8>(this, 0x18, b as u8);
    wr::<u32>(this, 0x1C, c);
    if !ped.is_null() {
        entity_register_reference(ped, this.byte_add(0x14));
    }
    this
}

/// `CEventKnockOffBike::CEventKnockOffBike()` (0x4AFC70): `+4 = 0`, `+8 = 0`, `vtbl = 0x85B2E0`,
/// `+0x24..+0x30 = 0`, `+0x34 = 0`, `+0x35` bit 0 cleared (read first), `+0x36 = +0x37 = 0`,
/// `+0x38 = 0`.
#[hook(addr = 0x4AFC70, conv = thiscall, name = "CEventKnockOffBike::Constructor-second", expect = "8B C1 8A 50 35")]
pub fn event_knock_off_bike_ctor_4afc70(this: GPtr<u8>) -> GPtr<u8> {
    let f = rd::<u8>(this, 0x35) & 0xFE;
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u32>(this, 0, 0x85B2E0);
    for o in [0x24, 0x28, 0x2C, 0x30] {
        wr::<u32>(this, o, 0);
    }
    wr::<u8>(this, 0x34, 0);
    wr::<u8>(this, 0x35, f);
    wr::<u8>(this, 0x36, 0);
    wr::<u8>(this, 0x37, 0);
    wr::<u32>(this, 0x38, 0);
    this
}

/// `CEventKnockOffBike::CEventKnockOffBike(CVehicle*, CVector*, CVector*, float, float, uint8,
/// uint8, int, CPed*, bool, bool)` (0x4AFCF0, `ret 0x2C`; entry `push -1; jmp` to a straight
/// protection stub that pushes the SEH handler): `vtbl = 0x85B2E0`, vectors at +0xC/+0x18, `+0x24`,
/// `+0x28`, `+0x2C`, ped `+0x30`, bytes `+0x34`/`+0x36`, `+0x35 = (old & 0xFC) | b2 << 1 | b1`,
/// `+0x37 = 0`, vehicle `+0x38`; registers the vehicle, then the ped (re-read from +0x30).
#[hook(addr = 0x4AFCF0, conv = thiscall, name = "CEventKnockOffBike::Constructor-first", expect = "6A FF E9 4C A5 F5 FF")]
#[allow(clippy::too_many_arguments)]
pub fn event_knock_off_bike_ctor_4afcf0(
    this: GPtr<u8>,
    veh: GPtr<u8>,
    v1: GPtr<u8>,
    v2: GPtr<u8>,
    f24: u32,
    f28: u32,
    b36: u32,
    b34: u32,
    i2c: u32,
    ped: GPtr<u8>,
    b1: u32,
    b2: u32,
) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u32>(this, 0, 0x85B2E0);
    copy_vec(this.byte_add(0xC), v1);
    copy_vec(this.byte_add(0x18), v2);
    wr::<u32>(this, 0x2C, i2c);
    wr::<u32>(this, 0x24, f24);
    wr::<u32>(this, 0x28, f28);
    wr::<u32>(this, 0x30, ped.addr() as u32);
    let f = ((b2 as u8 & 1) << 1) | (rd::<u8>(this, 0x35) & 0xFC);
    wr::<u8>(this, 0x34, b34 as u8);
    wr::<u8>(this, 0x35, f | (b1 as u8 & 1));
    wr::<u8>(this, 0x36, b36 as u8);
    wr::<u8>(this, 0x37, 0);
    wr::<u32>(this, 0x38, veh.addr() as u32);
    if !veh.is_null() {
        entity_register_reference(veh, this.byte_add(0x38));
    }
    let p = rd::<GPtr<u8>>(this, 0x30);
    if !p.is_null() {
        entity_register_reference(p, this.byte_add(0x30));
    }
    this
}

/// `CEventKnockOffBike::AffectsPed(CPed*)` (0x4AFEE0, `ret 4`, AL): alive; the vehicle (+0x38) not
/// in status `+0x36 & 0xF8 == 0x60`; a ped with `+0x474 & 0x18000000 == 0x8000000` needs `+0x35`
/// bit 1; the ped must be the driver or in one of the three passenger slots; a driver sets `+0x35`
/// bit 0.
#[hook(addr = 0x4AFEE0, conv = thiscall, name = "CEventKnockOffBike::AffectsPed", expect = "56 8B 74 24 08")]
pub fn event_knock_off_bike_affects_ped_4afee0(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if ped_is_alive(ped) == 0 {
        return 0;
    }
    let v = rd::<GPtr<u8>>(this, 0x38);
    if !v.is_null() && rd::<u8>(v, 0x36) & 0xF8 == 0x60 {
        return 0;
    }
    if rd::<u32>(ped, 0x474) & 0x1800_0000 == 0x800_0000 && rd::<u8>(this, 0x35) & 2 == 0 {
        return 0;
    }
    if v.is_null() {
        return 0;
    }
    let d = rd::<GPtr<u8>>(v, 0x460);
    if d != ped && rd::<GPtr<u8>>(v, 0x464) != ped && rd::<GPtr<u8>>(v, 0x468) != ped && rd::<GPtr<u8>>(v, 0x46C) != ped {
        return 0;
    }
    if d == ped {
        wr::<u8>(this, 0x35, rd::<u8>(this, 0x35) | 1);
    }
    1
}

/// `CEventKnockOffBike::SetPedOutCar(CPed*)` (0x4AFF60, `ret 4`): door (+0x37) = 10 (driver, written
/// twice), 11 for a bike (`+0x590 == 9`) or the second passenger, 8 / 9 for the first / third;
/// then a stack `CTaskSimpleCarSetPedOut(vehicle, door, true)` (0x18 bytes) with `+0x13 = 1`,
/// `+0x10 = 0`, `ProcessPed(ped)`, `CCarEnterExit::RemoveCarSitAnim(ped)`, and its destructor.
#[hook(addr = 0x4AFF60, conv = thiscall, name = "CEventKnockOffBike::SetPedOutCar", expect = "64 A1 00 00 00 00")]
pub fn event_knock_off_bike_set_ped_out_car_4aff60(this: GPtr<u8>, ped: GPtr<u8>) {
    let v = rd::<GPtr<u8>>(this, 0x38);
    wr::<u8>(this, 0x37, 10);
    if rd::<GPtr<u8>>(v, 0x460) == ped {
        wr::<u8>(this, 0x37, 10);
    } else if rd::<u32>(v, 0x590) == 9 {
        wr::<u8>(this, 0x37, 0xB);
    } else if rd::<GPtr<u8>>(v, 0x464) == ped {
        wr::<u8>(this, 0x37, 8);
    } else if rd::<GPtr<u8>>(v, 0x468) == ped {
        wr::<u8>(this, 0x37, 0xB);
    } else if rd::<GPtr<u8>>(v, 0x46C) == ped {
        wr::<u8>(this, 0x37, 9);
    }
    let mut l = [0u32; 6];
    let t = buf(&mut l);
    task_car_set_ped_out_ctor(t, v, i32::from(rd::<i8>(this, 0x37)), 1);
    wr::<u8>(t, 0x13, 1);
    wr::<u8>(t, 0x10, 0);
    task_car_set_ped_out_process_ped(t, ped);
    car_enter_exit_remove_car_sit_anim(ped);
    task_car_set_ped_out_dtor(t);
}

/// The editable-response event ctor shape shared by `CEventGunAimedAt` and `CEventVehicleOnFire`:
/// `+4 = 0`, `+8 = 0`, `+0xC = 1`, `+0xE = 200`, `+0x10 = -1`, `vtbl`, `+0x14 = entity` (registered
/// when set).
fn editable_entity_ctor(this: GPtr<u8>, ent: GPtr<u8>, vtbl: u32) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u8>(this, 0xC, 1);
    wr::<u16>(this, 0xE, 0xC8);
    wr::<u16>(this, 0x10, 0xFFFF);
    wr::<u32>(this, 0, vtbl);
    wr::<u32>(this, 0x14, ent.addr() as u32);
    if !ent.is_null() {
        entity_register_reference(ent, this.byte_add(0x14));
    }
    this
}

/// `CEventGunAimedAt::CEventGunAimedAt(CPed*)` (0x4B0700, `ret 4`; entry `push -1; jmp` to a straight
/// protection stub that pushes the SEH handler): editable-response fields, `vtbl = 0x85B330`, the ped
/// at +0x14 (registered when set).
/// Not hookable: protection code reads the patched bytes (0x403A0D reads 0x4B0703).
#[sa_core::native(addr = 0x4B0700, conv = thiscall, name = "CEventGunAimedAt::CEventGunAimedAt")]
pub fn event_gun_aimed_at_ctor_4b0700(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    editable_entity_ctor(this, ped, 0x85B330)
}

/// `CEventVehicleOnFire::Constructor(CVehicle*)` (0x4B10C0, `ret 4`; entry `push -1; jmp` to a
/// straight protection stub that pushes the SEH handler): editable-response fields,
/// `vtbl = 0x85B440`, the vehicle at +0x14 (registered when set).
#[hook(addr = 0x4B10C0, conv = thiscall, name = "CEventVehicleOnFire::Constructor", expect = "6A FF E9 6D 10 F5 FF")]
pub fn event_vehicle_on_fire_ctor_4b10c0(this: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> {
    editable_entity_ctor(this, veh, 0x85B440)
}

/// `CEventScriptCommand::IsValid(CPed*)` (0x4B0AB0, `ret 4`): with a ped, `ped->IsAlive()` (its whole
/// EAX passed through); else EAX = `+8 || +4 <= GetLifeTime()` (1/0).
#[hook(addr = 0x4B0AB0, conv = thiscall, name = "CEventScriptCommand::IsValid", expect = "56 8B F1 8B 4C 24 08")]
pub fn event_script_command_is_valid_4b0ab0(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    if !ped.is_null() {
        return ped_is_alive_eax(ped);
    }
    if rd::<u8>(this, 8) != 0 {
        return 1;
    }
    u32::from(rd::<i32>(this, 4) <= ev_get_life_time(this))
}

/// `CEventCopCarBeingStolen::AffectsPed(CPed*)` (0x4B1860, `ret 4`): hijacker (+0xC) and vehicle
/// (+0x10) set, the hijacker alive, the ped alive and a cop (type 6), in a vehicle (`+0x46C` bit 8)
/// that is this one. The early rejections set AL only, the final answer full EAX (u32 here).
#[hook(addr = 0x4B1860, conv = thiscall, name = "CEventCopCarBeingStolen::AffectsPed", expect = "56 8B F1 8B 4E 0C")]
pub fn event_cop_car_being_stolen_affects_ped_4b1860(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    let h = rd::<GPtr<u8>>(this, 0xC);
    if h.is_null() || rd::<u32>(this, 0x10) == 0 || ped_is_alive(h) == 0 || ped_is_alive(ped) == 0 {
        return 0;
    }
    if rd::<u32>(ped, 0x598) != 6 {
        return 0;
    }
    u32::from(rd::<u32>(this, 0x10) == rd::<u32>(ped, 0x58C) && rd::<u32>(ped, 0x46C) & 0x100 != 0)
}

/// `CEventGunShotWhizzedBy::Constructor(CEntity*, const CVector&, const CVector&, bool)` (0x4B1FD0,
/// `ret 0x10`): `CEventGunShot::Constructor(entity, *a, *b, flag)` with both vectors copied by value
/// onto the stack, then `vtbl = 0x85B730`.
#[hook(addr = 0x4B1FD0, conv = thiscall, name = "CEventGunShotWhizzedBy::Constructor", expect = "8B 44 24 10 56")]
pub fn event_gun_shot_whizzed_by_ctor_4b1fd0(this: GPtr<u8>, ent: u32, a: GPtr<u8>, b: GPtr<u8>, f: u32) -> GPtr<u8> {
    let bv = [rd::<u32>(b, 0), rd::<u32>(b, 4), rd::<u32>(b, 8)];
    let av = [rd::<u32>(a, 0), rd::<u32>(a, 4), rd::<u32>(a, 8)];
    gun_shot_ctor(this, ent, av[0], av[1], av[2], bv[0], bv[1], bv[2], f);
    wr::<u32>(this, 0, 0x85B730);
    this
}

/// `CEventAreaCodes::AffectsPed(CPed*)` (0x4B2270, `ret 4`, AL): the target ped (+0xC) set; the ped's
/// active 0x3E8 task (or, when that one aims elsewhere, its 0x44D task) must target it; then the
/// target doing a 0x3A4 task → true; both in the same attractor/zone (`+0x584`, byte `+0x2F`) →
/// false; both alive; the ped's `+0x2F` == 0xD → false; the target's missing or != 0xD → true.
#[hook(addr = 0x4B2270, conv = thiscall, name = "CEventAreaCodes::AffectsPed", expect = "53 8B D9 8B 43 0C")]
pub fn event_area_codes_affects_ped_4b2270(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u32>(this, 0xC) == 0 {
        return 0;
    }
    let mut tgt = 0u32;
    let t = tm_find_active_task_by_type(intel(ped).byte_add(4), 0x3E8);
    if !t.is_null() {
        tgt = rd::<u32>(t, 0x10);
        if tgt != rd::<u32>(this, 0xC) {
            let t2 = tm_find_active_task_by_type(intel(ped).byte_add(4), 0x44D);
            if !t2.is_null() {
                tgt = rd::<u32>(t2, 0x10);
            }
        }
    }
    let p = rd::<GPtr<u8>>(this, 0xC);
    if tgt != p.addr() as u32 {
        return 0;
    }
    if !intel_find_task_by_type(intel(p), 0x3A4).is_null() {
        return 1;
    }
    let a = rd::<GPtr<u8>>(ped, 0x584);
    if !a.is_null() {
        let b = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 0x584);
        if !b.is_null() && rd::<u8>(a, 0x2F) == rd::<u8>(b, 0x2F) {
            return 0;
        }
    }
    if ped_is_alive(rd::<GPtr<u8>>(this, 0xC)) == 0 || ped_is_alive(ped) == 0 {
        return 0;
    }
    let a = rd::<GPtr<u8>>(ped, 0x584);
    if !a.is_null() && rd::<u8>(a, 0x2F) == 0xD {
        return 0;
    }
    let b = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 0x584);
    u8::from(b.is_null() || rd::<u8>(b, 0x2F) != 0xD)
}

/// `CEventGunShot::AffectsPed(CPed*)` (0x4B2CD0, `ret 4`, AL): a shooter (+0x14); not a ped of the
/// same group; not a passenger of a driven vehicle; a cop while the player is wanted and the cop is
/// (or may join) the pursuit only for the player's shots (`FindPlayerWanted(-1)` asked each time);
/// the ped alive, not the shooter, not a player; `|ped - shooter|²` (x87 order `(dz² + dy²) + dx²`,
/// stored as f32) within `r²`, `r` = `[0x8A625C]` (when above `[0x858B50]`) or the ped's FOV range
/// (otherwise) for a ped with `+0x484 == 2`, else `[0x858CB4]`; with `+0x30` set also
/// `(+0x18 - pedPos) · ped forward > [0x858B50]` (NaN passes; the matrix is not null-checked) and a
/// clear line of sight from +0x18 to the ped.
#[hook(addr = 0x4B2CD0, conv = thiscall, name = "CEventGunShot::AffectsPed", expect = "83 EC 0C 53 8B D9")]
pub fn event_gun_shot_affects_ped_4b2cd0(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let s = rd::<GPtr<u8>>(this, 0x14);
    if s.is_null() {
        return 0;
    }
    if rd::<u8>(s, 0x36) & 7 == 3 && ped_groups_are_in_same_group(ped, s) != 0 {
        return 0;
    }
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        if !v.is_null() && vehicle_is_passenger(v, ped) != 0 && rd::<u32>(rd::<GPtr<u8>>(ped, 0x58C), 0x460) != 0 {
            return 0;
        }
    }
    if rd::<u32>(ped, 0x598) == 6
        && rd::<i32>(find_player_wanted(-1), 0x2C) > 0
        && (wanted_is_in_pursuit(find_player_wanted(-1), ped) != 0
            || wanted_can_cop_join_pursuit(find_player_wanted(-1), ped) != 0)
    {
        let pp = find_player_ped(-1);
        if rd::<GPtr<u8>>(this, 0x14) != pp {
            return 0;
        }
    }
    if ped_is_alive(ped) == 0 || ped == rd::<GPtr<u8>>(this, 0x14) || ped_is_player(ped) != 0 {
        return 0;
    }
    let b = ent_pos(rd::<GPtr<u8>>(this, 0x14));
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let a = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    let dx = x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(b, 0));
    let dy = x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(b, 4));
    let dz = x(rd::<f32>(a, 8)).fsub_m(rd::<f32>(b, 8));
    let d2 = f32_of(dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)));
    let gr = gf(0x8A625C);
    let two = rd::<u8>(ped, 0x484) == 2;
    let r = if !gr.fcom(gf(0x858B50)).test_any(0x41) {
        if two {
            gf(0x8A625C)
        } else {
            gf(0x858CB4)
        }
    } else if two {
        intel_get_ped_fov_range(intel(ped))
    } else {
        gf(0x858CB4)
    };
    if !x(d2).fcom(r.fmul(r)).test_any(0x41) {
        return 0;
    }
    if rd::<u8>(this, 0x30) == 0 {
        return 1;
    }
    let mut l = [0u32; 3];
    let lp = buf(&mut l);
    vector_sub(lp, this.byte_add(0x18), a);
    let dot = x(rd::<f32>(lp, 4))
        .fmul_m(rd::<f32>(m, 0x14))
        .fadd(x(rd::<f32>(lp, 8)).fmul_m(rd::<f32>(m, 0x18)))
        .fadd(x(rd::<f32>(lp, 0)).fmul_m(rd::<f32>(m, 0x10)));
    if !dot.fcom(gf(0x858B50)).test_parity(0x41) {
        return 0;
    }
    u8::from(world_get_is_line_of_sight_clear(this.byte_add(0x18), a, 1, 1, 1, 1, 1, 0, 0) != 0)
}

/// `CEventDamage::ReportCriminalEvent(CPed*)` (0x4B3440, `ret 4`): a criminal event (vtable +0x1C)
/// with a source: `RegisterCrime` on `FindPlayerWanted(-1)` at the source's position with the victim;
/// crime by weapon (+0x1C, signed): <= 9 → 3 (cop victim) / 2; 10..0x28 and 0x29 → 5 / 4; 0x31, 0x32
/// → 0xB / 0xA; others none. The bool argument is the victim argument slot with its low byte
/// replaced by `PoliceDontCareAboutCrimesAgainstPedType(victim type)`.
#[hook(addr = 0x4B3440, conv = thiscall, name = "CEventDamage::ReportCriminalEvent", expect = "56 8B F1 8B 06")]
pub fn event_damage_report_criminal_event_4b3440(this: GPtr<u8>, ped: GPtr<u8>) {
    if ev_is_criminal_event(this) == 0 || rd::<u32>(this, 0x14) == 0 {
        return;
    }
    let dc = ped_type_police_dont_care(rd::<u32>(ped, 0x598));
    let bslot = (ped.addr() as u32 & !0xFF) | u32::from(dc);
    let w = rd::<i32>(this, 0x1C);
    let cop = rd::<u32>(ped, 0x598) == 6;
    let crime = if w <= 9 {
        if cop { 3 } else { 2 }
    } else if w <= 0x28 || w == 0x29 {
        if cop { 5 } else { 4 }
    } else if w == 0x31 || w == 0x32 {
        if cop { 0xB } else { 0xA }
    } else {
        return;
    };
    let pos = ent_pos(rd::<GPtr<u8>>(this, 0x14));
    wanted_register_crime(find_player_wanted(-1), crime, pos, ped, bslot);
}

/// `CEventDamage::AffectsPed(CPed*)` (0x4B35A0, `ret 4`, AL): see the inline steps; `FindPlayerPed(-1)`
/// is called again at every use, the weapon (+0x1C) and source (+0x14) re-read, as the original.
/// Ends with `CanPhysicalBeDamaged(weapon, null)`, overridden to false for the jetpack weapon 0x36
/// when the ped is immune (`+0x40` bit 20), attached, or healthy (`+0x540 > [0x858B50]`) with a
/// jetpack task.
#[hook(addr = 0x4B35A0, conv = thiscall, name = "CEventDamage::AffectsPed", expect = "83 EC 0C 56 8B F1")]
pub fn event_damage_affects_ped_4b35a0(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    // already processed (+0x42) and not pending (+0x43)
    if rd::<u8>(this, 0x42) != 0 && rd::<u8>(this, 0x43) == 0 {
        return 1;
    }
    if ped_is_alive(ped) == 0 {
        return 0;
    }
    if ped == find_player_ped(-1) && rd::<u8>(rd::<GPtr<u8>>(find_player_ped(-1), 0x480), 0x34) & 0x10 == 0 {
        return 0;
    }
    if rd::<u32>(this, 0x1C) == 0x35 && rd::<u32>(ped, 0x470) & 0x10_0000 == 0 {
        return 0;
    }
    if ped == find_player_ped(-1) {
        let s = rd::<GPtr<u8>>(this, 0x14);
        if !s.is_null() && rd::<u8>(s, 0x36) & 7 == 3 && rd::<u32>(s, 0x598) == 8 && rd::<i32>(this, 0x1C) >= 0x10 {
            let t = tm_find_active_task_by_type(intel(s).byte_add(4), 0x3E8);
            if t.is_null() || rd::<GPtr<u8>>(t, 0x10) != ped {
                return 0;
            }
        }
    }
    if ped == find_player_ped(-1) {
        let i = usize::from(g::<u8>(0xB7CD74).read());
        if g::<u8>(0xB7CEE6 + 0x190 * i).read() != 0 {
            let w = rd::<u32>(this, 0x1C);
            if w == 0x12 || w == 0x25 {
                return 0;
            }
        }
    }
    if rd::<u32>(ped, 0x40) & 0x40_0000 != 0 {
        let s = ev_get_source_entity(this);
        if s != find_player_ped(-1) {
            let w = rd::<u32>(this, 0x1C);
            if w != 0x35 && w != 0x33 && !x(rd::<f32>(ped, 0x540)).fcom(gf(0x858B50)).test_any(0x41) {
                return 0;
            }
        }
    }
    if rd::<u8>(ped, 0x478) & 1 != 0 && rd::<u32>(weapon_info_get(rd::<u32>(this, 0x1C), 1), 0) == 1 {
        let p = rd::<u32>(this, 0x20);
        if p == 3 || p == 4 {
            return 0;
        }
    }
    let c = rd::<u8>(this, 0xC);
    if c == 0 {
        if rd::<u32>(this, 0x1C) == 0x17 {
            let s = rd::<GPtr<u8>>(this, 0x14);
            if s.is_null() || rd::<u8>(s, 0x36) & 7 != 3 || rd::<u8>(this, 0x25) & 8 == 0 || rd::<u32>(ped, 0x46C) & 0x100 == 0 {
                return 0;
            }
        }
        let s = rd::<GPtr<u8>>(this, 0x14);
        if !s.is_null()
            && rd::<u8>(s, 0x36) & 7 == 3
            && !tm_get_active_task(intel(s).byte_add(4)).is_null()
            && task_get_task_type(tm_get_active_task(intel(rd::<GPtr<u8>>(this, 0x14)).byte_add(4))) == 0x403
        {
            let pp = ent_pos(ped);
            let sp = ent_pos(rd::<GPtr<u8>>(this, 0x14));
            let mut l = [0u32; 3];
            let lp = buf(&mut l);
            vector_difference(lp, sp, pp);
            vector_normalise(lp);
            let r = intel_can_see_entity_with_lights(intel(ped), rd::<GPtr<u8>>(this, 0x14), 0);
            if !r.fcom(gf(0x858B50)).test_parity(0x41) {
                return 0;
            }
            let m = rd::<GPtr<u8>>(ped, 0x14);
            let dot = x(rd::<f32>(lp, 4))
                .fmul_m(rd::<f32>(m, 0x14))
                .fadd(x(rd::<f32>(lp, 8)).fmul_m(rd::<f32>(m, 0x18)))
                .fadd(x(rd::<f32>(lp, 0)).fmul_m(rd::<f32>(m, 0x10)));
            if !dot.fcom(gf(0xC0B034)).test_parity(5) {
                return 0;
            }
        }
    }
    let s = rd::<GPtr<u8>>(this, 0x14);
    if !s.is_null()
        && rd::<u8>(s, 0x36) & 7 == 3
        && ped_is_player(s) == 0
        && ped_groups_are_in_same_group(ped, rd::<GPtr<u8>>(this, 0x14)) != 0
        && rd::<u32>(this, 0x1C) != 0x33
    {
        return 0;
    }
    let mut bl = u8::from(physical_can_be_damaged(ped, rd::<u32>(this, 0x1C), 0) != 0);
    if rd::<u32>(this, 0x1C) == 0x36
        && (rd::<u32>(ped, 0x40) & 0x10_0000 != 0
            || rd::<u32>(ped, 0xFC) != 0
            || (!x(rd::<f32>(ped, 0x540)).fcom(gf(0x858B50)).test_any(0x41)
                && !intel_get_task_jet_pack(intel(ped)).is_null()))
    {
        bl = 0;
    }
    bl
}

/// `CEventDamage::ProcessDamage(CPed*)` (0x4B3A20, `ret 4`): `ComputeBodyPartToRemove(&part)` into a
/// zeroed stack int; a part → `ped->RemoveBodyPart(part, (u8) +0x24)`.
#[hook(addr = 0x4B3A20, conv = thiscall, name = "CEventDamage::ProcessDamage", expect = "51 56 8D 44 24 04")]
pub fn event_damage_process_damage_4b3a20(this: GPtr<u8>, ped: GPtr<u8>) {
    let mut l = [0u32; 1];
    let lp = buf(&mut l);
    damage_compute_body_part_to_remove(this, lp);
    let part = rd::<u32>(lp, 0);
    if part != 0 {
        ped_remove_body_part(ped, part, u32::from(rd::<u8>(this, 0x24)));
    }
}

/// `IKChainManager_c` instance `g_ikChainMan`.
const IK_CHAIN_MAN: usize = 0xC15448;

/// `CEventDeadPed::AffectsPed(CPed*)` (0x4B4830, `ret 4`): the dead ped (+0x14) set, not the ped
/// itself, the ped alive, not already looking at it (IK chain manager), the body not flagged
/// (`+0x40` bit 8); `d² = |dead - ped|²` (x87 order `(dz² + dy²) + dx²`, stored as f32). A dead ped
/// with `+0x474` bit 8 must also be in front (`(dz·fz + dy·fy) + dx·fx` not below `[0x858B50]`) and
/// within `[0x85AB0C]²`; then `d² < [0x85AB08]²` → EAX = 1. Early rejections set AL only, the last
/// test full EAX (u32 here).
#[hook(addr = 0x4B4830, conv = thiscall, name = "CEventDeadPed::AffectsPed", expect = "57 8B F9 8B 47 14")]
pub fn event_dead_ped_affects_ped_4b4830(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    let d = rd::<GPtr<u8>>(this, 0x14);
    if d.is_null() || ped == d || ped_is_alive(ped) == 0 {
        return 0;
    }
    let ik = GPtr::<u8>::from_addr(IK_CHAIN_MAN);
    if ik_is_looking(ik, ped) != 0 && ik_get_look_at_entity(ik, ped) == rd::<u32>(this, 0x14) {
        return 0;
    }
    let d = rd::<GPtr<u8>>(this, 0x14);
    if rd::<u32>(d, 0x40) & 0x100 != 0 {
        return 0;
    }
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let pp = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    let dp = ent_pos(d);
    let dx = x(rd::<f32>(dp, 0)).fsub_m(rd::<f32>(pp, 0));
    let dy = x(rd::<f32>(dp, 4)).fsub_m(rd::<f32>(pp, 4));
    let dz = x(rd::<f32>(dp, 8)).fsub_m(rd::<f32>(pp, 8));
    let d2 = f32_of(dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)));
    if rd::<u32>(d, 0x474) & 0x100 != 0 {
        let dot = dz
            .fmul_m(rd::<f32>(m, 0x18))
            .fadd(dy.fmul_m(rd::<f32>(m, 0x14)))
            .fadd(dx.fmul_m(rd::<f32>(m, 0x10)));
        if !dot.fcom(gf(0x858B50)).test_parity(5) {
            return 0;
        }
        let a = gf(0x85AB0C).fmul_m(g::<f32>(0x85AB0C).read());
        if !a.fcom(x(d2)).test_parity(5) {
            return 0;
        }
    }
    let b = gf(0x85AB08).fmul_m(g::<f32>(0x85AB08).read());
    u32::from(!b.fcom(x(d2)).test_any(0x41))
}

/// `CEventKnockOffBike::ReportCriminalEvent(CPed*)` (0x4B4E80, `ret 4`): a criminal event (vtable
/// +0x1C), a cop victim and a player culprit (+0x30) → `FindPlayerWanted(-1)->RegisterCrime(5,
/// victimPos, victim, false)`.
#[hook(addr = 0x4B4E80, conv = thiscall, name = "CEventKnockOffBike::ReportCriminalEvent", expect = "57 8B F9 8B 07")]
pub fn event_knock_off_bike_report_criminal_event_4b4e80(this: GPtr<u8>, ped: GPtr<u8>) {
    if ev_is_criminal_event(this) == 0 || rd::<u32>(ped, 0x598) != 6 {
        return;
    }
    let c = rd::<GPtr<u8>>(this, 0x30);
    if c.is_null() || ped_is_player(c) == 0 {
        return;
    }
    let pos = ent_pos(ped);
    wanted_register_crime(find_player_wanted(-1), 5, pos, ped, 0);
}

/// `CEventGunAimedAt::AffectsPed(CPed*)` (0x4B4EE0, `ret 4`, AL): not a player, not doing task 0x259
/// (`GetActiveTask` asked twice), an aimer (+0x14) whose position is in the ped's seeing range, the
/// ped alive, not in the aimer's group, not a passenger of a driven vehicle; a cop reports crime 0x16
/// against `FindPlayerPed(-1)`.
#[hook(addr = 0x4B4EE0, conv = thiscall, name = "CEventGunAimedAt::AffectsPed", expect = "56 8B 74 24 08")]
pub fn event_gun_aimed_at_affects_ped_4b4ee0(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if ped_is_player(ped) != 0 {
        return 0;
    }
    if !tm_get_active_task(intel(ped).byte_add(4)).is_null()
        && task_get_task_type(tm_get_active_task(intel(ped).byte_add(4))) == 0x259
    {
        return 0;
    }
    let a = rd::<GPtr<u8>>(this, 0x14);
    if a.is_null() || intel_is_in_seeing_range(intel(ped), ent_pos(a)) == 0 || ped_is_alive(ped) == 0 {
        return 0;
    }
    if ped_groups_are_in_same_group(ped, rd::<GPtr<u8>>(this, 0x14)) != 0 {
        return 0;
    }
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        if !v.is_null() && vehicle_is_passenger(v, ped) != 0 && rd::<u32>(rd::<GPtr<u8>>(ped, 0x58C), 0x460) != 0 {
            return 0;
        }
    }
    if rd::<u32>(ped, 0x598) == 6 {
        let p = find_player_ped(-1);
        crime_report_crime(0x16, ped, p);
    }
    1
}

/// `CEventVehicleOnFire::AffectsPed(CPed*)` (0x4B4FD0, `ret 4`, AL): a vehicle (+0x14); a ped in
/// another vehicle → false; not a player, alive; in a vehicle: true unless a non-leader group member;
/// on foot: `(dx² + dz²) + dy²` to the vehicle not above `[0x858628]`.
#[hook(addr = 0x4B4FD0, conv = thiscall, name = "CEventVehicleOnFire::AffectsPed", expect = "57 8B F9 8B 4F 14")]
pub fn event_vehicle_on_fire_affects_ped_4b4fd0(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let v = rd::<GPtr<u8>>(this, 0x14);
    if v.is_null() {
        return 0;
    }
    let pv = rd::<GPtr<u8>>(ped, 0x58C);
    if !pv.is_null() && rd::<u32>(ped, 0x46C) & 0x100 != 0 && pv != v {
        return 0;
    }
    if ped_is_player(ped) != 0 || ped_is_alive(ped) == 0 {
        return 0;
    }
    let grp = ped_groups_get_peds_group(ped);
    let follower = !grp.is_null() && membership_is_leader(grp.byte_add(8), ped) == 0;
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        return u8::from(!follower);
    }
    let vp = ent_pos(rd::<GPtr<u8>>(this, 0x14));
    let pp = ent_pos(ped);
    let dx = x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(vp, 0));
    let dy = x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(vp, 4));
    let dz = x(rd::<f32>(pp, 8)).fsub_m(rd::<f32>(vp, 8));
    let d2 = dx.fmul(dx).fadd(dz.fmul(dz)).fadd(dy.fmul(dy));
    u8::from(d2.fcom(gf(0x858628)).test_any(0x41))
}

/// `CEventGunShotWhizzedBy::AffectsPed(CPed*)` (0x4B5120, `ret 4`): not a player, alive; the shot
/// direction `normalise(+0x24 - +0x18)` (components stored as f32), `t = (pz·dz + py·dy) + px·dx`
/// of the ped relative to the start must be above `[0x858B50]` (else AL = 0); the closest point
/// `start + dir·t` (x kept on the x87 stack, y stored as f32) must lie within `[0x8A6264]` of the
/// ped in x/y (`ey² + ex²`) → EAX = 1, else EAX = 0 (u32 here).
#[hook(addr = 0x4B5120, conv = thiscall, name = "CEventGunShotWhizzedBy::AffectsPed", expect = "83 EC 18 56 57")]
pub fn event_gun_shot_whizzed_by_affects_ped_4b5120(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    if ped_is_player(ped) != 0 || ped_is_alive(ped) == 0 {
        return 0;
    }
    let mut l = [0u32; 3];
    let lp = buf(&mut l);
    for o in [0, 4, 8] {
        let v = x(rd::<f32>(this, 0x24 + o)).fsub_m(rd::<f32>(this, 0x18 + o));
        wr::<f32>(lp, o, f32_of(v));
    }
    vector_normalise(lp);
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let pp = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    let px = x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(this, 0x18));
    let py = x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(this, 0x1C));
    let pz = x(rd::<f32>(pp, 8)).fsub_m(rd::<f32>(this, 0x20));
    let t = pz
        .fmul_m(rd::<f32>(lp, 8))
        .fadd(py.fmul_m(rd::<f32>(lp, 4)))
        .fadd(px.fmul_m(rd::<f32>(lp, 0)));
    if t.fcom(gf(0x858B50)).test_any(0x41) {
        return 0;
    }
    for o in [0, 4, 8] {
        let v = x(rd::<f32>(lp, o)).fmul(t);
        wr::<f32>(lp, o, f32_of(v));
    }
    let cx = x(rd::<f32>(lp, 0)).fadd_m(rd::<f32>(this, 0x18));
    let cy = f32_of(x(rd::<f32>(lp, 4)).fadd_m(rd::<f32>(this, 0x1C)));
    let ex = cx.fsubr_m(rd::<f32>(pp, 0));
    let ey = x(rd::<f32>(pp, 4)).fsub_m(cy);
    let s = ey.fmul(ey).fadd(ex.fmul(ex));
    let r2 = gf(0x8A6264).fmul_m(g::<f32>(0x8A6264).read());
    u32::from(!r2.fcom(s).test_any(0x41))
}

/// `CEventDanger::AffectsPed(CPed*)` (0x4B5470, `ret 4`): a danger entity (+0x14) other than the
/// ped's vehicle, within radius `+0x18` in x/y (`r² < dx² + dy²` → AL = 0; NaN passes), then
/// `ped->IsAlive()` (its whole EAX).
#[hook(addr = 0x4B5470, conv = thiscall, name = "CEventDanger::AffectsPed", expect = "56 8B F1 8B 46 14")]
pub fn event_danger_affects_ped_4b5470(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    let e = rd::<GPtr<u8>>(this, 0x14);
    if e.is_null() || e == rd::<GPtr<u8>>(ped, 0x58C) {
        return 0;
    }
    let ep = ent_pos(e);
    let pp = ent_pos(ped);
    let dx = x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(ep, 0));
    let dy = x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(ep, 4));
    let r = x(rd::<f32>(this, 0x18));
    let s = dx.fmul(dx).fadd(dy.fmul(dy));
    if !r.fmul(r).fcom(s).test_parity(5) {
        return 0;
    }
    ped_is_alive_eax(ped)
}

/// `CEventDamage::CloneEditable` (0x4B5D40): inlined `operator new` + copy ctor (0x4B33B0); then
/// `+0x2C` and `+0x25` bit 4 copied into the result, which the original does even when the pool is
/// full (a null write, kept).
#[hook(addr = 0x4B5D40, conv = thiscall, name = "CEventDamage::CloneEditable", expect = "6A FF 68 7B A3 83 00")]
pub fn event_damage_clone_editable_4b5d40(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_pool_new(event_pool());
    let e = if e.is_null() { GPtr::NULL } else { damage_copy_ctor(e, this) };
    wr::<u32>(e, 0x2C, rd::<u32>(this, 0x2C));
    merge_bit(e, this, 0x10);
    e
}

/// `CEventDeath::~CEventDeath` (0x4B5DB0, scalar deleting): `FUN_004ade90`, then the inlined pool
/// delete when `flags & 1`. The original keeps using ECX after the call (the callee leaves it =
/// `this`); the port uses `this`.
#[hook(addr = 0x4B5DB0, conv = thiscall, name = "CEventDeath::~CEventDeath", expect = "56 E8 DA 80 FF FF")]
pub fn event_death_sdd_4b5db0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    death_dtor(this);
    if flags & 1 != 0 {
        crate::events::pool_delete_inline(EVENT_POOL, this, 0x44);
    }
    this
}

/// `CEventGunShot::CloneEditable` (0x4B6B20): inlined `operator new`;
/// `CEventGunShot::Constructor(+0x14, *(+0x18), *(+0x24), (u8) +0x30)` (vectors by value).
#[hook(addr = 0x4B6B20, conv = thiscall, name = "CEventGunShot::CloneEditable", expect = "6A FF 68 9B A3 83 00")]
pub fn event_gun_shot_clone_editable_4b6b20(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_pool_new(event_pool());
    if e.is_null() {
        return GPtr::NULL;
    }
    gun_shot_ctor(
        e,
        rd::<u32>(this, 0x14),
        rd::<u32>(this, 0x18),
        rd::<u32>(this, 0x1C),
        rd::<u32>(this, 0x20),
        rd::<u32>(this, 0x24),
        rd::<u32>(this, 0x28),
        rd::<u32>(this, 0x2C),
        u32::from(rd::<u8>(this, 0x30)),
    )
}

/// `CEventDeath::Clone` (0x4B6E30): inlined `operator new` and ctor: `+4 = 0`, `+8 = 0`,
/// `vtbl = 0x85ADC0`, byte `+0xC`, `+0x10`.
#[hook(addr = 0x4B6E30, conv = thiscall, name = "CEventDeath::Clone", expect = "56 8B F1 8B 0D AC 44 B7 00")]
pub fn event_death_clone_4b6e30(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_pool_new(event_pool());
    if e.is_null() {
        return GPtr::NULL;
    }
    let v = rd::<u32>(this, 0x10);
    let b = rd::<u8>(this, 0xC);
    wr::<u32>(e, 4, 0);
    wr::<u8>(e, 8, 0);
    wr::<u32>(e, 0, 0x85ADC0);
    wr::<u8>(e, 0xC, b);
    wr::<u32>(e, 0x10, v);
    e
}

/// `CEventKnockOffBike::Clone` (0x4B7590): inlined `operator new`; the full ctor (0x4AFCF0) with
/// `(+0x38, &+0xC, &+0x18, +0x24, +0x28, (i8) +0x36, (u8) +0x34, +0x2C, +0x30, +0x35 & 1,
/// (+0x35 & 2) >> 1)`.
#[hook(addr = 0x4B7590, conv = thiscall, name = "CEventKnockOffBike::Clone", expect = "6A FF 68 5B A6 83 00")]
pub fn event_knock_off_bike_clone_4b7590(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_pool_new(event_pool());
    if e.is_null() {
        return GPtr::NULL;
    }
    let f = u32::from(rd::<u8>(this, 0x35));
    knock_off_bike_ctor(
        e,
        rd::<u32>(this, 0x38),
        this.byte_add(0xC),
        this.byte_add(0x18),
        rd::<u32>(this, 0x24),
        rd::<u32>(this, 0x28),
        i32::from(rd::<i8>(this, 0x36)) as u32,
        u32::from(rd::<u8>(this, 0x34)),
        rd::<u32>(this, 0x2C),
        rd::<u32>(this, 0x30),
        f & 0xFFFF_FF01,
        (f & 2) >> 1,
    )
}

/// `CEventGlobalGroup::AddEventsToGroup(CPedGroup*)` (0x4AB8A0, `ret 4`): for each of the `+8` events
/// at `+0xC..`, `Clone()`, `CPedGroupIntelligence::AddEvent` on `group + 0x30`, then delete the clone
/// when non-null (count re-read every iteration).
#[hook(addr = 0x4AB8A0, conv = thiscall, name = "CEventGlobalGroup::AddEventsToGroup", expect = "8B 44 24 04 53")]
pub fn event_global_group_add_events_to_group_4ab8a0(this: GPtr<u8>, grp: GPtr<u8>) {
    let gi = grp.byte_add(0x30);
    if rd::<i32>(this, 8) <= 0 {
        return;
    }
    let mut i = 0i32;
    loop {
        let c = ev_clone(rd::<GPtr<u8>>(this, 0xC + 4 * i as isize));
        pgi_add_event(gi, c);
        if !c.is_null() {
            ev_delete(c, 1);
        }
        i += 1;
        if i >= rd::<i32>(this, 8) {
            break;
        }
    }
}

/// `CEventEditableResponse::TriggerLookAt(CPed*)` (0x4AC5A0, `ret 4`): with a source entity,
/// `g_ikChainMan.LookAt("CEventEditableResponse" (0x85ABC4), ped, source, 2000, bone, null, false,
/// 0.25, 500, 3, false)` with bone 5 for a ped source, -1 otherwise; AL = 1.
#[hook(addr = 0x4AC5A0, conv = thiscall, name = "CEventEditableResponse::TriggerLookAt", expect = "8B 01 FF 50 28")]
pub fn event_editable_response_trigger_look_at_4ac5a0(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let s = ev_get_source_entity(this);
    if !s.is_null() {
        let bone = if rd::<u8>(s, 0x36) & 7 == 3 { 5 } else { 0xFFFF_FFFF };
        ik_look_at(GPtr::from_addr(IK_CHAIN_MAN), 0x85ABC4, ped, s, 0x7D0, bone, 0, 1, 0x3E80_0000, 0x1F4, 3, 0);
    }
    1
}

/// `CEventBuildingCollision::IsHeadOnCollision(const CPed&)` (0x4AD1E0, `ret 4`, AL): the normal
/// `(n.x, n.y, 0)` normalised, `-((y·fy + z·fz) + x·fx)` against the ped's forward must exceed
/// `[0x85AAF0]` (NaN → false).
#[hook(addr = 0x4AD1E0, conv = thiscall, name = "CEventBuildingCollision::IsHeadOnCollision", expect = "83 EC 0C 8B 44 24 10")]
pub fn event_building_collision_is_head_on_collision_4ad1e0(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let f = rd::<GPtr<u8>>(ped, 0x14).byte_add(0x10);
    let mut v = [rd::<u32>(this, 0x18), rd::<u32>(this, 0x1C), 0u32];
    let vp = buf(&mut v);
    vector_normalise(vp);
    let dot = x(rd::<f32>(vp, 4))
        .fmul_m(rd::<f32>(f, 4))
        .fadd(x(rd::<f32>(vp, 8)).fmul_m(rd::<f32>(f, 8)))
        .fadd(x(rd::<f32>(vp, 0)).fmul_m(rd::<f32>(f, 0)));
    u8::from(!dot.fchs().fcom(gf(0x85AAF0)).test_any(0x41))
}

/// `CEventPotentialWalkIntoVehicle::AffectsPed(CPed*)` (0x4AE420, `ret 4`, AL): the enter-car task
/// (0x2BD) and the simplest active task first; a player without enter-car task needs a go-to task;
/// alive, on foot, `+0x18 != 1`. An active walk-round-car task (0x1FC) for another vehicle switches
/// to this one when its vehicle is this one's trailer/tractor (`+0x4C8`, then `+0x4C4`;
/// `SetNewVehicle(v, bits 2..5 of +0xD, signed)`) and the answer is false. Otherwise: not
/// attached, a vehicle whose `+0x594` is not -1/7/8; the target is the enter-car task's target
/// (`GetTargetPos`, false when entering this type-4 vehicle) or the simplest task's (0x384)
/// position at +0xC, none → true; false when the closest surface point and the target hit the same
/// side. Stack vectors: target and surface point (zeroed here).
#[hook(addr = 0x4AE420, conv = thiscall, name = "CEventPotentialWalkIntoVehicle::AffectsPed", expect = "83 EC 18 53 55 56")]
pub fn event_potential_walk_into_vehicle_affects_ped_4ae420(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let enter = tm_find_active_task_by_type(intel(ped).byte_add(4), 0x2BD);
    let simplest = tm_get_simplest_active_task(intel(ped).byte_add(4));
    if ped_is_player(ped) != 0 && enter.is_null() && task_is_go_to_task(simplest) == 0 {
        return 0;
    }
    if ped_is_alive(ped) == 0 || rd::<u32>(ped, 0x46C) & 0x100 != 0 || rd::<u32>(this, 0x18) == 1 {
        return 0;
    }
    let w = tm_find_active_task_by_type(intel(ped).byte_add(4), 0x1FC);
    if !w.is_null() {
        let c = rd::<u32>(w, 0x1C);
        let v = rd::<GPtr<u8>>(this, 0x14);
        if c == v.addr() as u32 {
            return 0;
        }
        let t = rd::<u32>(v, 0x4C8);
        if t == 0 || t != c {
            let t = rd::<u32>(v, 0x4C4);
            if t == 0 || t != c {
                return 0;
            }
        }
        let f = i32::from(((rd::<u8>(w, 0xD) << 2) as i8) >> 4);
        walk_round_car_set_new_vehicle(w, v, f);
        return 0;
    }
    if rd::<u32>(ped, 0xFC) != 0 {
        return 0;
    }
    let v = rd::<GPtr<u8>>(this, 0x14);
    if v.is_null() {
        return 0;
    }
    let k = rd::<i32>(v, 0x594);
    if k == -1 || k == 7 || k == 8 {
        return 0;
    }
    let mut l = [0u32; 6];
    let lp = buf(&mut l);
    let (tp, sp) = (lp, lp.byte_add(0xC));
    let mut have = false;
    if task_get_task_type(simplest) == 0x384 {
        copy_vec(tp, simplest.byte_add(0xC));
        have = true;
    }
    if !enter.is_null() {
        let v = rd::<GPtr<u8>>(this, 0x14);
        if v.addr() as u32 == rd::<u32>(enter, 0xC) && rd::<u32>(v, 0x594) == 4 {
            return 0;
        }
        let r = enter_car_get_target_pos(enter, sp);
        copy_vec(tp, r);
    } else if !have {
        return 1;
    }
    geom_compute_closest_surface_point(ped, rd::<GPtr<u8>>(this, 0x14), sp);
    let a = geom_compute_entity_hit_side(sp, rd::<GPtr<u8>>(this, 0x14));
    let b = geom_compute_entity_hit_side(tp, rd::<GPtr<u8>>(this, 0x14));
    u8::from(a != b)
}

/// The first active task of types 0x4B2, 0x4BF, 0x4B3 of a ped (intelligence re-read per lookup).
fn find_walk_task(ped: impl Fn() -> GPtr<u8>) -> GPtr<u8> {
    for ty in [0x4B2, 0x4BF, 0x4B3] {
        let t = tm_find_active_task_by_type(intel(ped()).byte_add(4), ty);
        if !t.is_null() {
            return t;
        }
    }
    GPtr::NULL
}

/// `CEventPotentialWalkIntoPed::AffectsPed(CPed*)` (0x4AE800, `ret 4`, AL): alive, the other ped
/// (+0x20) set, `+0x24 != 1`; both doing the same 0x4B2/0x4BF/0x4B3 task (other's type asked
/// first) → false; a 0x3A8 task at the other (+0xC) or a 0x3E8 task (`CPedIntelligence::
/// FindTaskByType`) at it (+0x10) → false; true when the simplest active task is a go-to task.
#[hook(addr = 0x4AE800, conv = thiscall, name = "CEventPotentialWalkIntoPed::AffectsPed", expect = "53 57 8B 7C 24 0C")]
pub fn event_potential_walk_into_ped_affects_ped_4ae800(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if ped_is_alive(ped) == 0 || rd::<u32>(this, 0x20) == 0 || rd::<u32>(this, 0x24) == 1 {
        return 0;
    }
    let t = find_walk_task(|| ped);
    if !t.is_null() {
        let o = find_walk_task(|| rd::<GPtr<u8>>(this, 0x20));
        if !o.is_null() {
            let oty = task_get_task_type(o);
            if task_get_task_type(t) == oty {
                return 0;
            }
        }
    }
    let a = tm_find_active_task_by_type(intel(ped).byte_add(4), 0x3A8);
    if !a.is_null() && rd::<u32>(a, 0xC) == rd::<u32>(this, 0x20) {
        return 0;
    }
    let b = intel_find_task_by_type(intel(ped), 0x3E8);
    if !b.is_null() && rd::<u32>(b, 0x10) == rd::<u32>(this, 0x20) {
        return 0;
    }
    let s = tm_get_simplest_active_task(intel(ped).byte_add(4));
    u8::from(!s.is_null() && task_is_go_to_task(s) != 0)
}

/// `CEventAcquaintancePed::AffectsPedGroup(CPedGroup*)` (0x4AF970, `ret 4`, AL): a ped (+0x14) not in
/// the group; for events 0x27/0x26 (type asked again for each test) the ped must not be the group's
/// first field, the leader must not be a player, and the ped must not be in the player's group
/// (`CPedGroups::ms_groups[player->+0x480->+0x38]`, membership at 0xC09928 + 0x2D4·id).
#[hook(addr = 0x4AF970, conv = thiscall, name = "CEventAcquaintancePed::AffectsPedGroup", expect = "56 8B F1 8B 46 14")]
pub fn event_acquaintance_ped_affects_ped_group_4af970(this: GPtr<u8>, grp: GPtr<u8>) -> u8 {
    let p = rd::<GPtr<u8>>(this, 0x14);
    if p.is_null() {
        return 0;
    }
    let m = grp.byte_add(8);
    if membership_is_member(m, p) != 0 {
        return 0;
    }
    let is_2x = || ev_get_event_type(this) == 0x27 || ev_get_event_type(this) == 0x26;
    if is_2x() && rd::<u32>(this, 0x14) == rd::<u32>(grp, 0) {
        return 0;
    }
    if !is_2x() {
        return 1;
    }
    if !membership_get_leader(m).is_null() && ped_is_player(membership_get_leader(m)) != 0 {
        return 0;
    }
    let pd = rd::<GPtr<u8>>(find_player_ped(-1), 0x480);
    let p = rd::<GPtr<u8>>(this, 0x14);
    let id = rd::<i32>(pd, 0x38);
    let pm = GPtr::<u8>::from_addr(id.wrapping_mul(0x2D4).wrapping_add(0xC09928) as u32 as usize);
    u8::from(membership_is_member(pm, p) == 0)
}

/// `CEventAcquaintancePed::AffectsPed(CPed*)` (0x4AFA30, `ret 4`, AL): a ped (+0x14), the target's
/// `+0x730` clear, both alive, and not in the same group.
#[hook(addr = 0x4AFA30, conv = thiscall, name = "CEventAcquaintancePed::AffectsPed", expect = "56 8B F1 8B 46 14")]
pub fn event_acquaintance_ped_affects_ped_4afa30(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u32>(this, 0x14) == 0 || rd::<u32>(ped, 0x730) != 0 {
        return 0;
    }
    if ped_is_alive(ped) == 0 || ped_is_alive(rd::<GPtr<u8>>(this, 0x14)) == 0 {
        return 0;
    }
    u8::from(ped_groups_are_in_same_group(ped, rd::<GPtr<u8>>(this, 0x14)) == 0)
}

/// `fld dword; fabs; fstp dword` of an f32 (the load quiets a signalling NaN).
fn fabs_f32(v: f32) -> f32 {
    f32::from_bits(f32_of(x(v)).to_bits() & 0x7FFF_FFFF)
}

/// `CEventBuildingCollision::CanTreatBuildingAsObject(CBuilding*)` (0x4B3120, cdecl, AL): `+0x1C`
/// bit 22 → true; else from the model's collision bounding box (`CModelInfo::ms_modelInfoPtrs`
/// 0xA9B0C8, `+0x14`): `|dx|`, `|dy|` stored as f32, `dz` on the x87 stack; a low box
/// (`dz < [0x85AAEC]`) is an object when `|dx|` or `|dy|` is below `[0x85AAE8]`, then (and for a tall
/// box) when both are.
#[hook(addr = 0x4B3120, conv = cdecl, name = "CEventBuildingCollision::CanTreatBuildingAsObject", expect = "8B 44 24 04 8B 48 1C")]
pub fn event_building_collision_can_treat_building_as_object_4b3120(b: GPtr<u8>) -> u8 {
    if rd::<u32>(b, 0x1C) & 0x40_0000 != 0 {
        return 1;
    }
    let mi = g::<GPtr<u8>>((0xA9B0C8 + 4 * i32::from(rd::<i16>(b, 0x22))) as u32 as usize).read();
    let cm = rd::<GPtr<u8>>(mi, 0x14);
    let dx = f32_of(x(rd::<f32>(cm, 0xC)).fsub_m(rd::<f32>(cm, 0)));
    let dy = f32_of(x(rd::<f32>(cm, 0x10)).fsub_m(rd::<f32>(cm, 4)));
    let dz = x(rd::<f32>(cm, 0x14)).fsub_m(rd::<f32>(cm, 8));
    let adx = fabs_f32(dx);
    let ady = fabs_f32(dy);
    let lim = || gf(0x85AAE8);
    let below = |v: f32| !x(v).fcom(lim()).test_parity(5);
    if !dz.fcom(gf(0x85AAEC)).test_parity(5) && (below(adx) || below(ady)) {
        return 1;
    }
    u8::from(below(adx) && below(ady))
}

/// `CEventScanner::ScanForEvents(CPed&)` (0x607E30, `ret 4`): only when `now > this[0]` (unsigned).
/// The intelligence is read once for the scanner calls (vehicle collisions over `intel +0xE0`,
/// `FUN_00606580` with ECX = the address of the ped argument slot (an empty-class `this`; a local
/// holding the ped here), `+0x10`, `+0xAC` over `intel +0x130`, `+0x1C`, `+0xC8`, the mental state
/// `intel +0x174`). Then, all events built in one 0x64-byte stack area (event at +0, damage event at
/// +0x20) and added with `CEventGroup::Add(e, false)`:
/// - in air (`+0x46C` bit 9, or not on the ground and `IsInAir`) → `CEventInAir`; on the ground with
///   bit 9 set, the bit is cleared unless the current event is 0x21;
/// - `+0x730` set → a `CEvent` (vtbl 0x86CCC0): a non-player whose simplest task refuses
///   `MakeAbortable(ped, 1, &e)` gets `GenerateDamageEvent(ped, [+0x730]+0x14, 0x25, 5, 3, 0)`
///   instead; the vtable is written again before the base dtor;
/// - `+0xBC` scanner over `intel +0x130`;
/// - an active 0x3E8 task (slots 4, 3, 1, 2) whose target has another area code (`+0x584` → byte
///   `+0x2F`) → `CEventAreaCodes(target)`;
/// - standing on model `[0x8CD674]` / `[0x8CD678]` (u16, compared with the sign-extended id) → a
///   `CEvent` with vtbl 0x86CD00;
/// - a non-player in a type-5 vehicle the player stands on → `FUN_004aeac0(player, vehicle, 10)`
///   with `+0xE = 0x2C2` (`FindPlayerPed(-1)` called twice);
/// - health not above `[0x858B50]` (NaN skips), alive and `now > [0xC0B038]`: unless the group holds
///   a `CEventDamage(0, 0, 0x37, 3, 0, 0, 0)` type, `GenerateDamageEvent(ped, 0, 0x36, 10, 3, 0)` and
///   `[0xC0B038] = now + 2000`.
#[hook(addr = 0x607E30, conv = thiscall, name = "CEventScanner::ScanForEvents", expect = "64 A1 00 00 00 00")]
pub fn event_scanner_scan_for_events_607e30(this: GPtr<u8>, ped: GPtr<u8>) {
    if g::<u32>(TIME_MS).read() <= rd::<u32>(this, 0) {
        return;
    }
    let it = intel(ped);
    scan_vehicle_potential_collision(this.byte_add(4), ped, it.byte_add(0xE0), 0x10);
    let mut slot = [ped.addr() as u32];
    fun_00606580(buf(&mut slot), ped, rd::<u32>(it, 0x170));
    fun_00606890(this.byte_add(0x10), ped);
    let ents = it.byte_add(0x130);
    fun_00607d80(this.byte_add(0xAC), ped, ents, 0x10);
    fun_006060a0(this.byte_add(0x1C), ped);
    fun_00603e70(this.byte_add(0xC8), ped);
    mental_state_process(it.byte_add(0x174), ped);

    let mut l = [0u32; 25];
    let e1 = buf(&mut l);
    let e3 = e1.byte_add(0x20);

    let in_air = rd::<u32>(ped, 0x46C) & 1 == 0
        && (rd::<u32>(ped, 0x46C) & 0x200 != 0 || geom_is_in_air(ped) != 0);
    if in_air {
        in_air_ctor(e1);
        event_group_add(it.byte_add(0x68), e1, 0);
        in_air_dtor(e1);
    } else if rd::<u32>(ped, 0x46C) & 0x200 != 0 {
        let keep = !history_get_current_event(it.byte_add(0x38)).is_null()
            && ev_get_event_type(history_get_current_event(it.byte_add(0x38))) == 0x21;
        if !keep {
            wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFFFF_FDFF);
        }
    }

    if rd::<u32>(ped, 0x730) != 0 {
        event_ctor(e1);
        wr::<u32>(e1, 0, 0x86CCC0);
        let mut add = true;
        if ped_is_player(ped) == 0 {
            let s = tm_get_simplest_active_task(it.byte_add(4));
            if !s.is_null() && task_make_abortable(s, ped, 1, e1) == 0 {
                let src = rd::<u32>(rd::<GPtr<u8>>(ped, 0x730), 0x14);
                weapon_generate_damage_event(ped, src, 0x25, 5, 3, 0);
                add = false;
            }
        }
        if add {
            event_group_add(it.byte_add(0x68), e1, 0);
        }
        wr::<u32>(e1, 0, 0x86CCC0);
        event_base_dtor(e1);
    }

    fun_00603bf0(this.byte_add(0xBC), ped, ents, 0x10);

    let tm = intel(ped).byte_add(4);
    let mut t = GPtr::NULL;
    for slot in [4, 3, 1, 2] {
        t = tm_find_task_by_type(tm, slot, 0x3E8);
        if !t.is_null() {
            break;
        }
    }
    if !t.is_null() {
        let tgt = rd::<GPtr<u8>>(t, 0x10);
        if !tgt.is_null() {
            let a = rd::<GPtr<u8>>(tgt, 0x584);
            let b = rd::<GPtr<u8>>(ped, 0x584);
            if !a.is_null() && !b.is_null() && rd::<u8>(a, 0x2F) != rd::<u8>(b, 0x2F) {
                area_codes_ctor(e1, tgt);
                event_group_add(intel(ped).byte_add(0x68), e1, 0);
                area_codes_dtor(e1);
            }
        }
    }

    let a = rd::<GPtr<u8>>(ped, 0x584);
    if !a.is_null() {
        let id = i32::from(rd::<i16>(a, 0x22)) as u32;
        if u32::from(g::<u16>(0x8CD674).read()) == id || u32::from(g::<u16>(0x8CD678).read()) == id {
            event_ctor(e1);
            wr::<u32>(e1, 0, 0x86CD00);
            event_group_add(intel(ped).byte_add(0x68), e1, 0);
            event_base_dtor(e1);
        }
    }

    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        if !v.is_null() && rd::<u32>(v, 0x590) == 5 && ped_is_player(ped) == 0 {
            let v = rd::<GPtr<u8>>(ped, 0x58C);
            if rd::<GPtr<u8>>(find_player_ped(-1), 0x584) == v {
                let p = find_player_ped(-1);
                fun_004aeac0(e1, p, v, 0xA);
                let grp = intel(ped).byte_add(0x68);
                wr::<u16>(e1, 0xE, 0x2C2);
                event_group_add(grp, e1, 0);
                fun_004aeb90(e1);
            }
        }
    }

    if !x(rd::<f32>(ped, 0x540)).fcom(gf(0x858B50)).test_parity(0x41) && ped_is_alive(ped) != 0 {
        if g::<u32>(TIME_MS).read() <= g::<u32>(0xC0B038).read() {
            return;
        }
        damage_ctor(e3, 0, 0, 0x37, 3, 0, 0, 0);
        if event_group_has_event_of_type(intel(ped).byte_add(0x68), e3) == 0 {
            weapon_generate_damage_event(ped, 0, 0x36, 0xA, 3, 0);
            g::<u32>(0xC0B038).write(g::<u32>(TIME_MS).read().wrapping_add(0x7D0));
        }
        damage_dtor(e3);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_EVENT_EDITABLE_RESPONSE_CLONE_420ED0,
    HOOK_EVENT_GLOBAL_GROUP_ADD_EVENTS_TO_PED_4AB9C0,
    HOOK_EVENT_EDITABLE_RESPONSE_INFORM_VEHICLE_OCCUPANTS_4AC4A0,
    HOOK_EVENT_EDITABLE_RESPONSE_INFORM_RESPECTED_FRIENDS_4B2B00,
    HOOK_EVENT_EDITABLE_RESPONSE_INFORM_GROUP_4B7DF0,
    HOOK_EVENT_EDITABLE_RESPONSE_COMPUTE_RESPONSE_TASK_TYPE_PED_4B56C0,
    HOOK_EVENT_EDITABLE_RESPONSE_COMPUTE_RESPONSE_TASK_OF_TYPE_4B5730,
    HOOK_EVENT_EDITABLE_RESPONSE_COMPUTE_RESPONSE_TASK_TYPE_GROUP_4B57A0,
    HOOK_EVENT_POOL_NEW_4B5570,
    HOOK_EVENT_SCRIPT_COMMAND_CLONE_4B6490,
    HOOK_EVENT_PED_COLLISION_WITH_PED_CTOR_4AC990,
    HOOK_EVENT_BUILDING_COLLISION_CTOR_4ACF00,
    HOOK_EVENT_SOUND_QUIET_CTOR_5E05B0,
    HOOK_EVENT_PED_COLLISION_WITH_PED_AFFECTS_PED_4ACB10,
    HOOK_EVENT_BUILDING_COLLISION_AFFECTS_PED_4AD070,
    HOOK_EVENT_SCRIPT_COMMAND_GET_EVENT_PRIORITY_4B0B20,
    HOOK_EVENT_SOUND_QUIET_AFFECTS_PED_4B5240,
    HOOK_EVENT_OBJECT_COLLISION_CTOR_4ACCF0,
    HOOK_EVENT_DAMAGE_CTOR_4AD830,
    HOOK_EVENT_DAMAGE_FROM_4AD9C0,
    HOOK_EVENT_DAMAGE_IS_CRIMINAL_EVENT_4ADA90,
    HOOK_EVENT_DAMAGE_TAKES_PRIORITY_OVER_4ADB00,
    HOOK_EVENT_DAMAGE_COMPUTE_BODY_PART_TO_REMOVE_4ADC10,
    HOOK_EVENT_DAMAGE_WAS_HEAD_SHOT_4ADD60,
    HOOK_EVENT_DEAD_PED_CTOR_4ADEA0,
    HOOK_EVENT_KNOCK_OFF_BIKE_CTOR_4AFC70,
    HOOK_EVENT_KNOCK_OFF_BIKE_CTOR_4AFCF0,
    HOOK_EVENT_KNOCK_OFF_BIKE_AFFECTS_PED_4AFEE0,
    HOOK_EVENT_KNOCK_OFF_BIKE_SET_PED_OUT_CAR_4AFF60,
    HOOK_EVENT_SCRIPT_COMMAND_IS_VALID_4B0AB0,
    HOOK_EVENT_VEHICLE_ON_FIRE_CTOR_4B10C0,
    HOOK_EVENT_COP_CAR_BEING_STOLEN_AFFECTS_PED_4B1860,
    HOOK_EVENT_GUN_SHOT_WHIZZED_BY_CTOR_4B1FD0,
    HOOK_EVENT_AREA_CODES_AFFECTS_PED_4B2270,
    HOOK_EVENT_GUN_SHOT_AFFECTS_PED_4B2CD0,
    HOOK_EVENT_DAMAGE_REPORT_CRIMINAL_EVENT_4B3440,
    HOOK_EVENT_DAMAGE_AFFECTS_PED_4B35A0,
    HOOK_EVENT_DAMAGE_PROCESS_DAMAGE_4B3A20,
    HOOK_EVENT_DEAD_PED_AFFECTS_PED_4B4830,
    HOOK_EVENT_KNOCK_OFF_BIKE_REPORT_CRIMINAL_EVENT_4B4E80,
    HOOK_EVENT_GUN_AIMED_AT_AFFECTS_PED_4B4EE0,
    HOOK_EVENT_VEHICLE_ON_FIRE_AFFECTS_PED_4B4FD0,
    HOOK_EVENT_GUN_SHOT_WHIZZED_BY_AFFECTS_PED_4B5120,
    HOOK_EVENT_DANGER_AFFECTS_PED_4B5470,
    HOOK_EVENT_DAMAGE_CLONE_EDITABLE_4B5D40,
    HOOK_EVENT_DEATH_SDD_4B5DB0,
    HOOK_EVENT_GUN_SHOT_CLONE_EDITABLE_4B6B20,
    HOOK_EVENT_DEATH_CLONE_4B6E30,
    HOOK_EVENT_KNOCK_OFF_BIKE_CLONE_4B7590,
    HOOK_EVENT_GLOBAL_GROUP_ADD_EVENTS_TO_GROUP_4AB8A0,
    HOOK_EVENT_EDITABLE_RESPONSE_TRIGGER_LOOK_AT_4AC5A0,
    HOOK_EVENT_BUILDING_COLLISION_IS_HEAD_ON_COLLISION_4AD1E0,
    HOOK_EVENT_POTENTIAL_WALK_INTO_VEHICLE_AFFECTS_PED_4AE420,
    HOOK_EVENT_POTENTIAL_WALK_INTO_PED_AFFECTS_PED_4AE800,
    HOOK_EVENT_ACQUAINTANCE_PED_AFFECTS_PED_GROUP_4AF970,
    HOOK_EVENT_ACQUAINTANCE_PED_AFFECTS_PED_4AFA30,
    HOOK_EVENT_BUILDING_COLLISION_CAN_TREAT_BUILDING_AS_OBJECT_4B3120,
    HOOK_EVENT_SCANNER_SCAN_FOR_EVENTS_607E30,
];

/// `vector_dot_product_st0` with its ST0 result as `X87` (orig! returns stay register-sized).
fn vector_dot_product(a: GPtr<u8>, b: GPtr<u8>) -> X87 {
    X87::from_f64_exact(vector_dot_product_st0(a, b))
}

/// `vector_magnitude_sqr_st0` with its ST0 result as `X87` (orig! returns stay register-sized).
fn vector_magnitude_sqr(v: GPtr<u8>) -> X87 {
    X87::from_f64_exact(vector_magnitude_sqr_st0(v))
}

/// `event_get_sound_level_st0` with its ST0 result as `X87` (orig! returns stay register-sized).
fn event_get_sound_level(ev: GPtr<u8>, ent: GPtr<u8>, pos: GPtr<u8>) -> X87 {
    X87::from_f64_exact(event_get_sound_level_st0(ev, ent, pos))
}

/// `intel_get_ped_fov_range_st0` with its ST0 result as `X87` (orig! returns stay register-sized).
fn intel_get_ped_fov_range(intel: GPtr<u8>) -> X87 {
    X87::from_f64_exact(intel_get_ped_fov_range_st0(intel))
}

/// `intel_can_see_entity_with_lights_st0` with its ST0 result as `X87` (orig! returns stay register-sized).
fn intel_can_see_entity_with_lights(intel: GPtr<u8>, e: GPtr<u8>, b: u32) -> X87 {
    X87::from_f64_exact(intel_can_see_entity_with_lights_st0(intel, e, b))
}
