//! MVP-C AI ports, batch `mvpc-ai-1`: the event handler's dispatch (`CEventHandler::ComputeEventResponseTask`,
//! `SetEventResponseTask`, the passive vehicle-collision and script-command responses), `CInformGroupEventQueue::Add`
//! the pedestrian-acquaintance scanner's entry points (0x607D80, 0x603A30) and
//! the decision-maker response probe 0x6042B0, the conversation setters, the anim-blend association destructors, and
//! the boot-time model-info store / vehicle-structure helpers reached from the CRT initialisers. Ported from the 1.0
//! US bytes (protection fragments inlined; the exe wins over gta-reversed). Every call to another catalogued
//! function goes through `orig!` at the original address.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEventHandler::ComputeVehicleCollisionResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BD6A0, `ret 0xC`)
    fn r_vehicle_collision(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BD6A0 => unported;
    /// `CEventHandler::ComputePedCollisionWithPedResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BDB80, `ret 0xC`)
    fn r_ped_collision_with_ped(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BDB80 => unported;
    /// `CEventHandler::ComputePedCollisionWithPlayerResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BE7D0, `ret 0xC`)
    fn r_ped_collision_with_player(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BE7D0 => unported;
    /// `CEventHandler::ComputePlayerCollisionWithPedResponse(CEvent*, CTask* active, CTask* simplest)` (0x4B8CE0, `ret 0xC`)
    fn r_player_collision_with_ped(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4B8CE0 => unported;
    /// `CEventHandler::ComputeObjectCollisionResponse(CEvent*, CTask* active, CTask* simplest)` (0x4B92B0, `ret 0xC`)
    fn r_object_collision(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4B92B0 => unported;
    /// `CEventHandler::ComputeBuildingCollisionResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BF2B0, `ret 0xC`)
    fn r_building_collision(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BF2B0 => unported;
    /// `CEventHandler::ComputeDraggedOutCarResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BCC30, `ret 0xC`)
    fn r_dragged_out_car(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BCC30 => unported;
    /// `CEventHandler::ComputeDamageResponse(CEvent*, CTask* active, CTask* simplest, CTask*)` (0x4C0170, `ret 0x10`)
    fn r_damage(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>, extra: GPtr<u8>) = thiscall 0x4C0170 => crate::handler::event_handler_compute_damage_response;
    /// `CEventHandler::ComputeDeathResponse(CEvent*, CTask* active, CTask* simplest)` (0x4B9400, `ret 0xC`)
    fn r_death(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4B9400 => unported;
    /// `CEventHandler::ComputeDeadPedResponse(CEvent*, CTask* active, CTask* simplest)` (0x4B9470, `ret 0xC`)
    fn r_dead_ped(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4B9470 => unported;
    /// `CEventHandler::ComputeVehiclePotentialCollisionResponse(CEvent*, CTask* active, CTask* simplest)` (0x4C0BD0, `ret 0xC`)
    fn r_vehicle_potential_collision(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4C0BD0 => unported;
    /// `CEventHandler::ComputeVehiclePotentialPassiveCollisionResponse(CEvent*, CTask* active, CTask* simplest)` (0x4B96D0, `ret 0xC`)
    fn r_vehicle_potential_passive_collision(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4B96D0 => crate::mvpc_ai1::event_handler_compute_vehicle_potential_passive_collision_response;
    /// `CEventHandler::ComputeShotFiredResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BC710, `ret 0xC`)
    fn r_shot_fired(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BC710 => unported;
    /// `CEventHandler::ComputeReviveResponse(CEvent*, CTask* active, CTask* simplest)` (0x4B97B0, `ret 0xC`)
    fn r_revive(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4B97B0 => unported;
    /// `CEventHandler::ComputeChatPartnerResponse(CEvent*, CTask* active, CTask* simplest)` (0x4B98E0, `ret 0xC`)
    fn r_chat_partner(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4B98E0 => unported;
    /// `CEventHandler::ComputeSexyPedResponse(CEvent*, CTask* active, CTask* simplest)` (0x4B99F0, `ret 0xC`)
    fn r_sexy_ped(h: GPtr<u8>, ev: GPtr<u8>, active: u32, simplest: u32) = thiscall 0x4B99F0 => crate::events::event_handler_compute_sexy_ped_response_4b99f0;
    /// `CEventHandler::ComputeSexyVehicleResponse(CEvent*, CTask* active, CTask* simplest)` (0x4B9AA0, `ret 0xC`)
    fn r_sexy_vehicle(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4B9AA0 => unported;
    /// `CEventHandler::ComputePedToChaseResponse(CEvent*, CTask* active, CTask* simplest)` (0x4C1910, `ret 0xC`)
    fn r_ped_to_chase(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4C1910 => unported;
    /// `CEventHandler::ComputePedToFleeResponse(CEvent*, CTask* active, CTask* simplest)` (0x4B9B50, `ret 0xC`)
    fn r_ped_to_flee(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4B9B50 => unported;
    /// `CEventHandler::ComputeAttractorResponse(CEvent*, CTask* active, CTask* simplest)` (0x4B9BE0, `ret 0xC`)
    fn r_attractor(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4B9BE0 => unported;
    /// `CEventHandler::ComputePotentialPedCollideResponse(CEvent*, CTask* active, CTask* simplest)` (0x4C2610, `ret 0xC`)
    fn r_potential_ped_collide(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4C2610 => unported;
    /// `CEventHandler::ComputePedEnteredVehicleResponse(CEvent*, CTask* active, CTask* simplest)` (0x4C1590, `ret 0xC`)
    fn r_ped_entered_vehicle(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4C1590 => unported;
    /// `CEventHandler::ComputeVehicleToStealResponse(CEvent*, CTask* active, CTask* simplest)` (0x4B9F80, `ret 0xC`)
    fn r_vehicle_to_steal(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4B9F80 => unported;
    /// `CEventHandler::ComputeKnockOffBikeResponse(CEvent*, CTask* active, CTask* simplest)` (0x4B9FF0, `ret 0xC`)
    fn r_knock_off_bike(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4B9FF0 => crate::handler::event_handler_compute_knock_off_bike_response;
    /// `CEventHandler::ComputeGunAimedAtResponse(CEvent*, CTask* active, CTask* simplest)` (0x4C2840, `ret 0xC`)
    fn r_gun_aimed_at(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4C2840 => unported;
    /// `CEventHandler::ComputeScriptCommandResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BA7C0, `ret 0xC`)
    fn r_script_command(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BA7C0 => crate::mvpc_ai1::event_handler_compute_script_command_response;
    /// `CEventHandler::ComputeVehicleDiedResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BA8B0, `ret 0xC`)
    fn r_vehicle_died(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BA8B0 => unported;
    /// `CEventHandler::ComputePedFriendResponse(CEvent*, CTask* active, CTask* simplest)` (0x4B9DD0, `ret 0xC`)
    fn r_ped_friend(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4B9DD0 => unported;
    /// `CEventHandler::ComputePedThreatResponse(CEvent*, CTask* active, CTask* simplest)` (0x4C19A0, `ret 0xC`)
    fn r_ped_threat(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4C19A0 => unported;
    /// `CEventHandler::ComputeSeenCopResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BC050, `ret 0xC`)
    fn r_seen_cop(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BC050 => unported;
    /// `CEventHandler::ComputePedThreatBadlyLitResponse(CEvent*, CTask* active, CTask* simplest)` (0x4B9C90, `ret 0xC`)
    fn r_ped_threat_badly_lit(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4B9C90 => unported;
    /// `CEventHandler::ComputePedSoundQuietResponse(CEvent*, CTask* active, CTask* simplest)` (0x4B9D40, `ret 0xC`)
    fn r_ped_sound_quiet(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4B9D40 => unported;
    /// `CEventHandler::ComputeLowHealthResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BA990, `ret 0xC`)
    fn r_low_health(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BA990 => unported;
    /// `CEventHandler::ComputeReallyLowHealthResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BAA30, `ret 0xC`)
    fn r_really_low_health(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BAA30 => unported;
    /// `CEventHandler::ComputeLowAngerAtPlayerResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BAAD0, `ret 0xC`)
    fn r_low_anger_at_player(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BAAD0 => unported;
    /// `CEventHandler::ComputeHighAngerAtPlayerResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BAC10, `ret 0xC`)
    fn r_high_anger_at_player(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BAC10 => unported;
    /// `CEventHandler::ComputeOnFireResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BAD50, `ret 0xC`)
    fn r_on_fire(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BAD50 => unported;
    /// `CEventHandler::ComputeWaterCannonResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BAE30, `ret 0xC`)
    fn r_water_cannon(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BAE30 => unported;
    /// `CEventHandler::ComputeInWaterResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BAF80, `ret 0xC`)
    fn r_in_water(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BAF80 => unported;
    /// `CEventHandler::ComputeInteriorUseInfoResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BAFE0, `ret 0xC`)
    fn r_interior_use_info(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BAFE0 => unported;
    /// `CEventHandler::ComputeSignalAtPedResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BB050, `ret 0xC`)
    fn r_signal_at_ped(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BB050 => unported;
    /// `CEventHandler::ComputePassObjectResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BB0C0, `ret 0xC`)
    fn r_pass_object(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BB0C0 => unported;
    /// `CEventHandler::ComputeCreatePartnerTaskResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BB130, `ret 0xC`)
    fn r_create_partner_task(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BB130 => unported;
    /// `CEventHandler::ComputePlayerWantedLevelResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BB280, `ret 0xC`)
    fn r_player_wanted_level(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BB280 => unported;
    /// `CEventHandler::ComputeVehicleOnFireResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BB2E0, `ret 0xC`)
    fn r_vehicle_on_fire(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BB2E0 => unported;
    /// `CEventHandler::ComputeCopCarBeingStolenResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BB740, `ret 0xC`)
    fn r_cop_car_being_stolen(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BB740 => unported;
    /// `CEventHandler::ComputeVehicleDamageResponse(CEvent*, CTask* active, CTask* simplest)` (0x4C2FC0, `ret 0xC`)
    fn r_vehicle_damage(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4C2FC0 => unported;
    /// `CEventHandler::ComputeSpecialResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BB800, `ret 0xC`)
    fn r_special(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BB800 => unported;
    /// `CEventHandler::ComputeGotKnockedOverByCarResponse(CEvent*, CTask* active, CTask* simplest)` (0x4C3430, `ret 0xC`)
    fn r_got_knocked_over_by_car(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4C3430 => unported;
    /// `CEventHandler::ComputeObjectCollisionPassiveResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BBB90, `ret 0xC`)
    fn r_object_collision_passive(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BBB90 => unported;
    /// `CEventHandler::ComputeCarUpsideDownResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BBC30, `ret 0xC`)
    fn r_car_upside_down(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BBC30 => unported;
    /// `CEventHandler::ComputePotentialWalkIntoFireResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BBCD0, `ret 0xC`)
    fn r_potential_walk_into_fire(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BBCD0 => unported;
    /// `CEventHandler::ComputeShotFiredWhizzedByResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BBE30, `ret 0xC`)
    fn r_shot_fired_whizzed_by(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BBE30 => unported;
    /// `CEventHandler::ComputeFireNearbyResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BBFB0, `ret 0xC`)
    fn r_fire_nearby(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BBFB0 => unported;
    /// `CEventHandler::ComputeSeenPanickedPedResponse(CEvent*, CTask* active, CTask* simplest)` (0x4C35F0, `ret 0xC`)
    fn r_seen_panicked_ped(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4C35F0 => unported;
    /// `CEventHandler::ComputeAreaCodesResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BBF50, `ret 0xC`)
    fn r_area_codes(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BBF50 => unported;
    /// `CEventHandler::ComputeOnEscalatorResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BC150, `ret 0xC`)
    fn r_on_escalator(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BC150 => unported;
    /// `CEventHandler::ComputeDangerResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BC230, `ret 0xC`)
    fn r_danger(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BC230 => unported;
    /// `CEventHandler::ComputeDontJoinGroupResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BC1D0, `ret 0xC`)
    fn r_dont_join_group(h: GPtr<u8>, ev: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) = thiscall 0x4BC1D0 => unported;
    /// `CTaskManager::GetActiveTask` (0x681720)
    fn tm_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTaskManager::GetSimplestActiveTask` (0x6819D0)
    fn tm_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810, `ret 4`)
    fn tm_get_task_secondary(tm: GPtr<u8>, slot: i32) -> GPtr<u8> = thiscall 0x681810 => crate::tasks::task_manager_get_task_secondary;
    /// `CTaskManager::SetTaskSecondary(CTask*, int32)` (0x681B60, `ret 8`)
    fn tm_set_task_secondary(tm: GPtr<u8>, t: GPtr<u8>, slot: i32) = thiscall 0x681B60 => crate::taskmgr::task_manager_set_task_secondary;
    /// `CTaskManager::SetTask(CTask*, int32, bool)` (0x681AF0, `ret 0xC`)
    fn tm_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: i32, unused: u8) = thiscall 0x681AF0 => crate::taskmgr::task_manager_set_task;
    /// `CTaskManager::ClearTaskEventResponse` (0x681BD0)
    fn tm_clear_task_event_response(tm: GPtr<u8>) = thiscall 0x681BD0 => crate::taskmgr::task_manager_clear_task_event_response;
    /// `CEvent::GetEventType` (vtable +4)
    fn v_get_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CEvent` vtable +0x24 (`HasEditableResponse`, AL)
    fn v_has_editable_response(e: GPtr<u8>) -> u8 = thiscall vslot 0x24 => unported;
    /// `CEventScriptCommand` vtable +0x40 (`CloneScriptTask`)
    fn v_clone_script_task(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x40 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18, `ret 0xC`, AL)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new_o(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => crate::mvpa_ai1::task_operator_new;
    /// `CTaskComplexInAirAndLand::CTaskComplexInAirAndLand(bool, bool)` (0x678C80, `ret 8`)
    fn in_air_and_land_ctor(t: GPtr<u8>, glide: u8, vault: u8) -> GPtr<u8> = thiscall 0x678C80 => crate::fall::in_air_and_land_ctor;
    /// `CTaskComplexStuckInAir::CTaskComplexStuckInAir` (0x67BA40)
    fn stuck_in_air_ctor(t: u32) -> u32 = thiscall 0x67BA40 => crate::gen_ctors::g_ctask_complex_stuck_in_air_67ba40;
    /// `CPedIntelligence::AddTaskPhysResponse(CTask*, bool)` (0x600DC0, `ret 8`)
    fn intel_add_task_phys_response(i: GPtr<u8>, t: GPtr<u8>, b: u32) = thiscall 0x600DC0 => unported;
    /// `CPedIntelligence::AddTaskEventResponseTemp(CTask*, bool)` (0x600DE0, `ret 8`)
    fn intel_add_task_event_response_temp(i: GPtr<u8>, t: GPtr<u8>, b: u32) = thiscall 0x600DE0 => crate::intel::intel_add_task_event_response_temp;
    /// `CPedIntelligence::AddTaskEventResponseNonTemp(CTask*, bool)` (0x600E00, `ret 8`)
    fn intel_add_task_event_response_non_temp(i: GPtr<u8>, t: GPtr<u8>, b: u32) = thiscall 0x600E00 => crate::intel::intel_add_task_event_response_non_temp;
    /// `CEventHandler::IsTemporaryEvent(const CEvent&)` (0x4BC370, cdecl, AL)
    fn is_temporary_event(e: GPtr<u8>) -> u32 = cdecl 0x4BC370 => crate::handler::handler_is_temporary_event_port;
    /// `CTaskComplexFacial::SetRequest(eFacialExpression, int32 duration, int32, int32)` (0x691230, `ret 0x10`)
    fn facial_set_request(t: u32, a0: u32, a1: u32, a2: u32, a3: u32) -> u32 = thiscall 0x691230 => crate::gen_leaves::g_task_complex_facial_set_request_691230;
    /// `CTask::IsGoToTask(CTask*)` (0x61A360, cdecl, AL)
    fn task_is_go_to_task(t: GPtr<u8>) -> u8 = cdecl 0x61A360 => crate::tasks::task_is_go_to_task;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80, cdecl)
    fn get_peds_group(ped: u32) -> u32 = cdecl 0x5F7E80 => crate::misc::ped_groups_get_peds_group_5f7e80;
    /// `CPedIntelligence::IsPedGoingForCarDoor` (0x602350, AL)
    fn intel_is_ped_going_for_car_door(i: GPtr<u8>) -> u8 = thiscall 0x602350 => crate::intelrest::intel_is_ped_going_for_car_door;
    /// `CTaskComplexWalkRoundCar::CTaskComplexWalkRoundCar(int32 moveState, const CVector&, CVehicle*, bool, bool)`
    /// (0x6541B0, `ret 0x14`)
    fn walk_round_car_ctor(t: GPtr<u8>, ms: i32, target: GPtr<u8>, veh: GPtr<u8>, door: u32, b: u32) -> GPtr<u8> = thiscall 0x6541B0 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, valid: u8) -> GPtr<u8> = thiscall 0x4AB420 => crate::group::event_group_add;
    /// `CPedScriptedTaskRecord::GetRecordAssociatedWithEvent(CEvent*)` (0x608580, cdecl)
    fn record_associated_with_event(e: u32) -> GPtr<u8> = cdecl 0x608580 => crate::small::scripted_task_record_get_record_associated_with_event;
    /// `CPedScriptedTaskRecordData::AssociateWithEvent(CEvent*)` (0x608500, `ret 4`)
    fn record_associate_with_event(r: u32, e: u32) -> u32 = thiscall 0x608500 => crate::gen_leaves::g_cped_scripted_task_record_data_associate_with_event_608500;
    /// `CPedScriptedTaskRecordData::AssociateWithTask(CTask*)` (0x608520, `ret 4`)
    fn record_associate_with_task(r: GPtr<u8>, t: u32) = thiscall 0x608520 => crate::small::record_data_associate_with_task;
    /// `CEventEditableResponse::ComputeResponseTaskType(CPedGroup*)` (0x4B57A0, `ret 4`)
    fn editable_compute_response_task_type_group(e: GPtr<u8>, g: GPtr<u8>) = thiscall 0x4B57A0 => crate::events2::event_editable_response_compute_response_task_type_group_4b57a0;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped;
    /// `CPedGroupMembership::IsMember(const CPed*)` (0x5F6A10, `ret 4`, AL)
    fn membership_is_member(m: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x5F6A10 => crate::small::membership_is_member;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => sa_world::world::entity::register_reference;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCornersUncached(float z, CEntity&, CVector*)` (0x5F1FA0, cdecl)
    fn bbox_corners_uncached(z: u32, e: GPtr<u8>, corners: GPtr<u8>) = cdecl 0x5F1FA0 => unported;
    /// `CAnimBlendAssociation::~CAnimBlendAssociation` (0x4CECF0)
    fn anim_assoc_dtor(a: GPtr<u8>) = thiscall 0x4CECF0 => crate::mvpc_ai1::anim_blend_association_dtor;
    /// CRT `free` (0x8214BD, cdecl)
    fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `CMemoryMgr::FreeAlign(void*)` (0x72F4F0, cdecl)
    fn free_align(p: GPtr<u8>) = cdecl 0x72F4F0 => sa_core::memory::free_align;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0, cdecl)
    fn remove_anim_block_ref(idx: u32) = cdecl 0x4D3FD0 => unported;
    /// `CTaskSimpleRunAnim::CTaskSimpleRunAnim(AssocGroupId, AnimationId, float, eTaskType, const char*, bool)`
    /// (0x61A900, `ret 0x18`)
    fn run_anim_ctor2(t: GPtr<u8>, group: u32, anim: u32, delta: u32, ty: u32, name: u32, hold: u8) -> GPtr<u8> = thiscall 0x61A900 => crate::mvpa_ai2::run_anim_ctor2;
    /// `CTaskSimpleAnim::~CTaskSimpleAnim` (0x61A6F0)
    fn simple_anim_dtor(t: GPtr<u8>) = thiscall 0x61A6F0 => crate::tasks::simple_anim_dtor;
    /// `CBaseModelInfo::CBaseModelInfo` (0x4C4A60)
    fn base_model_info_ctor(m: GPtr<u8>) -> GPtr<u8> = thiscall 0x4C4A60 => unported;
    /// `RpAtomicDestroy(RpAtomic*)` (0x749DC0, cdecl)
    fn rp_atomic_destroy(a: GPtr<u8>) -> u32 = cdecl 0x749DC0 => unported;
    /// `RwFrameDestroy(RwFrame*)` (0x7F05A0, cdecl)
    fn rw_frame_destroy(f: GPtr<u8>) -> u32 = cdecl 0x7F05A0 => unported;
    /// CRT `eh vector constructor iterator(void*, size, count, ctor, dtor)` (0x821D30, stdcall)
    fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// CRT `eh vector destructor iterator(void*, size, count, dtor)` (0x821E02, stdcall)
    fn eh_vector_dtor(arr: GPtr<u8>, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// `CPed::IsAlive` (0x5E0170, AL)
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CPedScriptedTaskRecord::GetStatus(const CPed*)` (0x608750, cdecl)
    fn scripted_task_get_status(p: GPtr<u8>) -> i32 = cdecl 0x608750 => unported;
    /// `CEventHandlerHistory::GetCurrentEvent` (0x4B8CA0)
    fn history_get_current_event(h: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B8CA0 => crate::handler::history_get_current_event;
    /// `CEventEditableResponse::ComputeResponseTaskOfType(const CPed*, int32)` (0x4B5730, `ret 8`, AL)
    fn editable_compute_response_task_of_type(e: GPtr<u8>, ped: GPtr<u8>, ty: i32) -> u8 = thiscall 0x4B5730 => crate::events2::event_editable_response_compute_response_task_of_type_4b5730;
    /// `CPedIntelligence::FindRespectedFriendInInformRange` (0x600CF0, AL)
    fn intel_find_respected_friend_in_inform_range(i: GPtr<u8>) -> u8 = thiscall 0x600CF0 => unported;
    /// `FUN_00603a30` (0x603A30, `ret 4`, AL): may the acquaintance scanner scan this ped
    fn acq_scanner_can_scan_o(s: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x603A30 => crate::mvpc_ai1::acq_scanner_can_scan;
    /// `FUN_00607a90` (0x607A90, `ret 0x18`): the acquaintance scan over the nearby entities
    #[allow(clippy::too_many_arguments)]
    fn acq_scanner_scan_entities(s: GPtr<u8>, ped: GPtr<u8>, ty: i32, ents: GPtr<u8>, n: i32, out_a: GPtr<u8>, out_b: GPtr<u8>) = thiscall 0x607A90 => unported;
}

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;

fn local<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// `CEventHandler::ComputeEventResponseTask(CEvent*, CTask*)` (0x4C3870, `ret 8`; the entry is `nop` + a protection
/// fragment that loads the SEH head): the five response slots +0x20..+0x30 are cleared; `active` = the task manager's
/// (ped +0x47C, +4) active task and `simplest` = its simplest active task (null when there is no active task). The
/// event type is fetched twice (the first result is dropped) and dispatched through the byte-indexed table at
/// 0x4C3DC8: most types call `Compute*Response(event, active, simplest)`; 9 (damage) also passes the second
/// argument; 33 creates `CTaskComplexInAirAndLand(false, false)` into +0x24 unless ped flag 1 at +0x46C is set; 93
/// creates `CTaskComplexStuckInAir` into +0x24 (null when the pool is full). Unlisted types do nothing.
#[hook(addr = 0x4C3870, conv = thiscall, name = "CEventHandler::ComputeEventResponseTask-0", expect = "90 E9 00 E1 F3 FF",
       unverified = "mvpc-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_handler_compute_event_response_task(this: GPtr<u8>, ev: GPtr<u8>, extra: GPtr<u8>) {
    for off in [0x20isize, 0x24, 0x28, 0x2C, 0x30] {
        wr::<u32>(this, off, 0);
    }
    let ped = rd::<GPtr<u8>>(this, 0);
    let active = tm_get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
    let simplest = if active.is_null() {
        GPtr::NULL
    } else {
        tm_get_simplest_active_task(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C).byte_add(4))
    };
    v_get_event_type(ev);
    match v_get_event_type(ev) {
        1 => r_vehicle_collision(this, ev, active, simplest),
        2 => r_ped_collision_with_ped(this, ev, active, simplest),
        3 => r_ped_collision_with_player(this, ev, active, simplest),
        4 => r_player_collision_with_ped(this, ev, active, simplest),
        5 => r_object_collision(this, ev, active, simplest),
        6 => r_building_collision(this, ev, active, simplest),
        7 => r_dragged_out_car(this, ev, active, simplest),
        9 => r_damage(this, ev, active, simplest, extra),
        10 => r_death(this, ev, active, simplest),
        11 => r_dead_ped(this, ev, active, simplest),
        12 => r_vehicle_potential_collision(this, ev, active, simplest),
        56 => r_vehicle_potential_passive_collision(this, ev, active, simplest),
        15 => r_shot_fired(this, ev, active, simplest),
        18 => r_revive(this, ev, active, simplest),
        19 => r_chat_partner(this, ev, active, simplest),
        20 => r_sexy_ped(this, ev, active.addr() as u32, simplest.addr() as u32),
        21 => r_sexy_vehicle(this, ev, active, simplest),
        25 => r_ped_to_chase(this, ev, active, simplest),
        26 => r_ped_to_flee(this, ev, active, simplest),
        27 | 48 => r_attractor(this, ev, active, simplest),
        13 => r_potential_ped_collide(this, ev, active, simplest),
        17 => r_ped_entered_vehicle(this, ev, active, simplest),
        28 => r_vehicle_to_steal(this, ev, active, simplest),
        8 => r_knock_off_bike(this, ev, active, simplest),
        31 => r_gun_aimed_at(this, ev, active, simplest),
        32 => r_script_command(this, ev, active, simplest),
        35 => r_vehicle_died(this, ev, active, simplest),
        38 | 39 => r_ped_friend(this, ev, active, simplest),
        36 | 37 => r_ped_threat(this, ev, active, simplest),
        72 => r_seen_cop(this, ev, active, simplest),
        63 => r_ped_threat_badly_lit(this, ev, active, simplest),
        62 => r_ped_sound_quiet(this, ev, active, simplest),
        53 => r_low_health(this, ev, active, simplest),
        52 => r_really_low_health(this, ev, active, simplest),
        50 => r_low_anger_at_player(this, ev, active, simplest),
        51 => r_high_anger_at_player(this, ev, active, simplest),
        58 => r_on_fire(this, ev, active, simplest),
        64 => r_water_cannon(this, ev, active, simplest),
        66 => r_in_water(this, ev, active, simplest),
        81 => r_interior_use_info(this, ev, active, simplest),
        88 => r_signal_at_ped(this, ev, active, simplest),
        89 => r_pass_object(this, ev, active, simplest),
        70 => r_create_partner_task(this, ev, active, simplest),
        69 => r_player_wanted_level(this, ev, active, simplest),
        79 => r_vehicle_on_fire(this, ev, active, simplest),
        16 => r_cop_car_being_stolen(this, ev, active, simplest),
        41 | 73 => r_vehicle_damage(this, ev, active, simplest),
        42 => r_special(this, ev, active, simplest),
        43 => r_got_knocked_over_by_car(this, ev, active, simplest),
        44 => r_object_collision_passive(this, ev, active, simplest),
        45 => r_car_upside_down(this, ev, active, simplest),
        46 => r_potential_walk_into_fire(this, ev, active, simplest),
        49 => r_shot_fired_whizzed_by(this, ev, active, simplest),
        59 => r_fire_nearby(this, ev, active, simplest),
        65 => r_seen_panicked_ped(this, ev, active, simplest),
        67 => r_area_codes(this, ev, active, simplest),
        74 => r_on_escalator(this, ev, active, simplest),
        75 => r_danger(this, ev, active, simplest),
        94 => r_dont_join_group(this, ev, active, simplest),
        33 => {
            if rd::<u32>(rd::<GPtr<u8>>(this, 0), 0x46C) & 1 == 0 {
                let t = task_new_o(0x10);
                let r = if t.is_null() { GPtr::NULL } else { in_air_and_land_ctor(t, 0, 0) };
                wr::<GPtr<u8>>(this, 0x24, r);
            }
        }
        93 => {
            let t = task_new_o(0xC);
            let r = if t.is_null() { 0 } else { stuck_in_air_ctor(t.addr() as u32) };
            wr::<u32>(this, 0x24, r);
        }
        _ => {}
    }
}

/// `CEventHandler::SetEventResponseTask(const CEvent&)` (0x4BC600, `ret 4`): with `intel` = ped +0x47C: a physical
/// response (+0x20) goes to `AddTaskPhysResponse(t, true)`. An event response (+0x24) goes to
/// `AddTaskEventResponseTemp(t, false)` for a temporary event, else `AddTaskEventResponseTemp(null, false)` then
/// `AddTaskEventResponseNonTemp(t, false)`. A secondary attack response (+0x28) first makes the current secondary
/// slot-0 task abortable (`MakeAbortable(ped, 1, null)`, looked up twice) and then replaces it; +0x2C goes to
/// secondary slot 2 and +0x30 to slot 4. Last, for an event with an editable response, a facial task in secondary
/// slot 3 and a facial expression (i16 +0x10) other than -1: `SetRequest(expr, 10000, -1, 0)`.
#[hook(addr = 0x4BC600, conv = thiscall, name = "CEventHandler::SetEventResponseTask", expect = "53 55 56 8B F1 8B 06",
       unverified = "mvpc-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_handler_set_event_response_task(this: GPtr<u8>, ev: GPtr<u8>) {
    let intel = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C);
    let phys = rd::<GPtr<u8>>(this, 0x20);
    if !phys.is_null() {
        intel_add_task_phys_response(intel, phys, 1);
    }
    if !rd::<GPtr<u8>>(this, 0x24).is_null() {
        if is_temporary_event(ev) as u8 != 0 {
            intel_add_task_event_response_temp(intel, rd::<GPtr<u8>>(this, 0x24), 0);
        } else {
            intel_add_task_event_response_temp(intel, GPtr::NULL, 0);
            intel_add_task_event_response_non_temp(intel, rd::<GPtr<u8>>(this, 0x24), 0);
        }
    }
    if !rd::<GPtr<u8>>(this, 0x28).is_null() {
        let tm = intel.byte_add(4);
        if !tm_get_task_secondary(tm, 0).is_null() {
            let t = tm_get_task_secondary(tm, 0);
            v_make_abortable(t, rd::<GPtr<u8>>(this, 0), 1, GPtr::NULL);
        }
        tm_set_task_secondary(tm, rd::<GPtr<u8>>(this, 0x28), 0);
    }
    let t = rd::<GPtr<u8>>(this, 0x2C);
    if !t.is_null() {
        tm_set_task_secondary(intel.byte_add(4), t, 2);
    }
    let t = rd::<GPtr<u8>>(this, 0x30);
    if !t.is_null() {
        tm_set_task_secondary(intel.byte_add(4), t, 4);
    }
    if v_has_editable_response(ev) != 0 {
        let tm = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C).byte_add(4);
        if !tm_get_task_secondary(tm, 3).is_null() {
            let expr = rd::<i16>(ev, 0x10) as i32;
            if expr != -1 {
                let tm = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C).byte_add(4);
                let facial = tm_get_task_secondary(tm, 3);
                facial_set_request(facial.addr() as u32, expr as u32, 10000, 0xFFFF_FFFF, 0);
            }
        }
    }
}

/// `CEventHandler::ComputeVehiclePotentialPassiveCollisionResponse(CEvent*, CTask*, CTask* simplest)` (0x4B96D0,
/// `ret 0xC`): nothing without the event's vehicle (+0x14), for a ped with flag 0x100 at +0x46C, or unless the
/// simplest task is a go-to task. Move state = event +0x18; nothing when it is 1 (still); 6 (sprint) when the ped
/// has a group. Then +0x24 = `new CTaskComplexWalkRoundCar(moveState, simplest +0xC (target), vehicle, door, false)`
/// (null when the pool is full). The original stores `IsPedGoingForCarDoor`'s AL into the low byte of its first
/// argument slot and passes that whole dword as the `bool`; the pool slot pointer goes to its second argument slot.
#[hook(addr = 0x4B96D0, conv = thiscall, name = "CEventHandler::ComputeVehiclePotentialPassiveCollisionResponse", expect = "6A FF 68 9B A9 83 00",
       unverified = "mvpc-ai-1", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "writes into its own argument slots 1 (AL of IsPedGoingForCarDoor) and 2 (the new task)")]
pub fn event_handler_compute_vehicle_potential_passive_collision_response(this: GPtr<u8>, ev: GPtr<u8>, _active: GPtr<u8>, simplest: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(ev, 0x14);
    if veh.is_null() || rd::<u32>(rd::<GPtr<u8>>(this, 0), 0x46C) & 0x100 != 0 {
        return;
    }
    if simplest.is_null() || task_is_go_to_task(simplest) == 0 {
        return;
    }
    let target = simplest.byte_add(0xC);
    let mut ms = rd::<i32>(ev, 0x18);
    if ms == 1 {
        return;
    }
    if get_peds_group(rd::<u32>(this, 0)) != 0 {
        ms = 6;
    }
    let door = intel_is_ped_going_for_car_door(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C));
    let flag = (ev.addr() as u32 & !0xFF) | door as u32;
    let t = task_new_o(0x58);
    let r = if t.is_null() { GPtr::NULL } else { walk_round_car_ctor(t, ms, target, veh, flag, 0) };
    wr::<GPtr<u8>>(this, 0x24, r);
}

/// `CEventHandler::ComputeScriptCommandResponse(CEvent*, CTask*, CTask*)` (0x4BA7C0, `ret 0xC`): the event's primary
/// slot (+0xC) selects task manager slot 3 (value 3; current task at intel +0x10) or 4 (intel +0x14). When that slot
/// is empty or its task agrees to `MakeAbortable(ped, 1, event)`: the cloned script task (event vtable +0x40) is set
/// there after `ClearTaskEventResponse` (`SetTask(t, slot, true)`) and associated with the event's scripted-task
/// record, if any. Otherwise the task gets `MakeAbortable(ped, 0, event)`, the event is re-added to the ped's event
/// group (`Add(event, false)`) and the record, if any, is associated with the added copy.
#[hook(addr = 0x4BA7C0, conv = thiscall, name = "CEventHandler::ComputeScriptCommandResponse", expect = "53 56 8B 74 24 0C",
       unverified = "mvpc-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_handler_compute_script_command_response(this: GPtr<u8>, ev: GPtr<u8>, _a: GPtr<u8>, _b: GPtr<u8>) {
    let ped = rd::<GPtr<u8>>(this, 0);
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    let slot = if rd::<i32>(ev, 0xC) == 3 { 3 } else { 4 };
    let cur = rd::<GPtr<u8>>(intel, if slot == 3 { 0x10 } else { 0x14 });
    if !cur.is_null() && v_make_abortable(cur, ped, 1, ev) == 0 {
        v_make_abortable(cur, rd::<GPtr<u8>>(this, 0), 0, ev);
        let intel = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C);
        let added = event_group_add(intel.byte_add(0x68), ev, 0);
        let rec = record_associated_with_event(ev.addr() as u32);
        if !rec.is_null() {
            record_associate_with_event(rec.addr() as u32, added.addr() as u32);
        }
        return;
    }
    let task = v_clone_script_task(ev);
    tm_clear_task_event_response(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C).byte_add(4));
    tm_set_task(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C).byte_add(4), task, slot, 1);
    let rec = record_associated_with_event(ev.addr() as u32);
    if !rec.is_null() {
        record_associate_with_task(rec, task.addr() as u32);
    }
}

/// `CInformGroupEventQueue::Add(CEntity*, CPedGroup*, CEventEditableResponse*)` (0x4B7CD0, cdecl, AL): over the 8
/// entries (0xA9B018, 0x10 each: entity, group, event, time): an entry for the same entity whose event has the same
/// type rejects the add (false; an entry for the same entity with another type is skipped without being considered
/// free); the first entry with a null event and another entity is the free slot. No free slot: false. For event
/// types 0x24/0x25 the event computes its group response type, and the delay is 5000 ms when the entity is not a
/// member of the player's group, not a mission ped (+0x484 != 2), the event's source ped (+0x14) has flag 0x100 at
/// +0x46C and the event's response (u16 +0xE) is not 200; else 0. The slot gets the entity (with a reference
/// registered when non-null), the event, the group and `now + delay`. True.
#[hook(addr = 0x4B7CD0, conv = cdecl, name = "CInformGroupEventQueue::Add", expect = "51 53 55 56 57 8B 7C 24",
       unverified = "mvpc-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn inform_group_event_queue_add(ent: GPtr<u8>, group: GPtr<u8>, ev: GPtr<u8>) -> u8 {
    let mut free = -1i32;
    for i in 0..8i32 {
        let e = g::<u8>(0xA9B018 + 0x10 * i as usize);
        if rd::<GPtr<u8>>(e, 0) == ent {
            let ty = v_get_event_type(rd::<GPtr<u8>>(e, 8));
            if v_get_event_type(ev) == ty {
                return 0;
            }
        } else if free == -1 && rd::<u32>(e, 8) == 0 {
            free = i;
        }
    }
    if free == -1 {
        return 0;
    }
    let mut delay = 0u32;
    if v_get_event_type(ev) == 0x24 || v_get_event_type(ev) == 0x25 {
        editable_compute_response_task_type_group(ev, group);
        let gid = rd::<i32>(rd::<GPtr<u8>>(find_player_ped(-1), 0x480), 0x38);
        let m = g::<u8>((0xC09928 + gid * 0x2D4) as u32 as usize);
        delay = if membership_is_member(m, ent) as u8 == 0
            && rd::<u8>(ent, 0x484) != 2
            && rd::<u32>(rd::<GPtr<u8>>(ev, 0x14), 0x46C) & 0x100 != 0
            && rd::<u16>(ev, 0xE) != 0xC8
        {
            5000
        } else {
            0
        };
    }
    let s = g::<u8>(0xA9B018 + 0x10 * free as usize);
    wr::<GPtr<u8>>(s, 0, ent);
    if !ent.is_null() {
        entity_register_reference(ent, s);
    }
    wr::<GPtr<u8>>(s, 8, ev);
    wr::<GPtr<u8>>(s, 4, group);
    wr::<u32>(s, 0xC, g::<u32>(TIME_MS).read().wrapping_add(delay));
    1
}

/// `CConversations::StartSettingUpConversation(CPed*)` (0x43A840, a `.HOODLUM` stub; body 0x1565550, cdecl):
/// `m_pSettingUpConversationPed` (0x9691CC) = ped with a reference registered (no null test),
/// `m_SettingUpConversationNumNodes` (0x9691C8) = 0, `m_bSettingUpConversation` (0x9691D0) = 1.
#[hook(addr = 0x43A840, conv = cdecl, name = "CConversations::StartSettingUpConversation", expect = "E9 0B AD 12 01",
       unverified = "mvpc-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn conversations_start_setting_up_conversation(ped: GPtr<u8>) {
    g::<GPtr<u8>>(0x9691CC).write(ped);
    entity_register_reference(ped, g::<u8>(0x9691CC));
    g::<u32>(0x9691C8).write(0);
    g::<u8>(0x9691D0).write(1);
}

/// `CConversations::EnableConversation(CPed*, bool)` (0x43AA40, cdecl): the first of the 14 conversations (0x9691D8,
/// 0x1C each) whose ped (+8) is the argument gets enabled byte +0x18 = the flag. Original bug kept: with no match
/// the byte is written to address 0x18.
#[hook(addr = 0x43AA40, conv = cdecl, name = "CConversations::EnableConversation", expect = "8B 54 24 04 33 C9",
       unverified = "mvpc-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn conversations_enable_conversation(ped: GPtr<u8>, enable: u8) {
    for i in 0..14usize {
        let c = g::<u8>(0x9691D8 + 0x1C * i);
        if rd::<GPtr<u8>>(c, 8) == ped {
            wr::<u8>(c, 0x18, enable);
            return;
        }
    }
    wr::<u8>(GPtr::NULL, 0x18, enable);
}

/// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCorners(float z, CEntity&, CVector*)` (0x5F3650, cdecl): a tail
/// jump to the uncached version.
#[hook(addr = 0x5F3650, conv = cdecl, name = "CPedGeometryAnalyser::ComputeEntityBoundingBoxCorners", expect = "E9 4B E9 FF FF 90 90 90",
       unverified = "mvpc-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn compute_entity_bounding_box_corners(z: u32, e: GPtr<u8>, corners: GPtr<u8>) {
    bbox_corners_uncached(z, e, corners);
}

/// `CAnimBlendAssociation::~CAnimBlendAssociation` (0x4CECF0): vtable 0x85C6D0; frees the aligned node array (+0x10)
/// when set; unlinks the association from its list (`prev` +8 / `next` +4: `*prev = next`, `next->prev = prev`) and
/// clears both links; when flag 0x4000 of u16 +0x2E is set, releases the anim block of the hierarchy (+0x14, block
/// index +0xC).
#[hook(addr = 0x4CECF0, conv = thiscall, name = "CAnimBlendAssociation", expect = "56 8B F1 8B 46 10 85 C0",
       unverified = "mvpc-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn anim_blend_association_dtor(this: GPtr<u8>) {
    let nodes = rd::<GPtr<u8>>(this, 0x10);
    wr::<u32>(this, 0, 0x85C6D0);
    if !nodes.is_null() {
        free_align(nodes);
    }
    let prev = rd::<GPtr<u8>>(this, 8);
    if !prev.is_null() {
        wr::<u32>(prev, 0, rd::<u32>(this, 4));
    }
    let next = rd::<GPtr<u8>>(this, 4);
    if !next.is_null() {
        wr::<u32>(next, 4, rd::<u32>(this, 8));
    }
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0);
    if (rd::<u16>(this, 0x2E) >> 14) & 1 != 0 {
        remove_anim_block_ref(rd::<u32>(rd::<GPtr<u8>>(this, 0x14), 0xC));
    }
}

/// `CAnimBlendAssociation` scalar deleting destructor (0x4CEFA0, `ret 4`): destructor, then CRT `free` when bit 0
/// of the flags is set; returns `this`.
#[hook(addr = 0x4CEFA0, conv = thiscall, name = "CAnimBlendAssociation", expect = "56 8B F1 E8 48 FD FF FF",
       unverified = "mvpc-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn anim_blend_association_deleting_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    anim_assoc_dtor(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// 0x48DF30 (, a `.HOODLUM` stub; body 0x1562470): the `CTaskSimpleScratchHead` constructor:
/// `CTaskSimpleRunAnim(group 0, anim 0x87, 4.0, type 0x1A5, "ScratchHead" (0x85A128), false)`, vtable 0x85A100.
#[hook(addr = 0x48DF30, conv = thiscall, name = "thunk_FUN_01562470", expect = "E9 3B 45 0D 01",
       unverified = "mvpc-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn task_simple_scratch_head_ctor(this: GPtr<u8>) -> GPtr<u8> {
    run_anim_ctor2(this, 0, 0x87, 0x4080_0000, 0x1A5, 0x85A128, 0);
    wr::<u32>(this, 0, 0x85A100);
    this
}

/// 0x48E930: the `CTaskSimpleScratchHead` destructor, a tail jump to `~CTaskSimpleAnim`.
#[hook(addr = 0x48E930, conv = thiscall, name = "thunk_FUN_0061a6f0", expect = "E9 BB BD 18 00 90 90 90",
       unverified = "mvpc-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn task_simple_scratch_head_dtor(this: GPtr<u8>) {
    simple_anim_dtor(this);
}

/// `FUN_004c75e0` (0x4C75E0): the `CVehicleModelInfo` constructor: `CBaseModelInfo`, then +0x5C = 0, bytes +0x2D0
/// and +0x4E = 0, vtable 0x85C5C8, +0x304 = -1, the nine (unaligned) dwords +0x2D6..+0x2F6 = -1, +0x2FA and
/// +0x2FE = -1. Returns `this`.
#[hook(addr = 0x4C75E0, conv = thiscall, name = "FUN_004c75e0", expect = "56 57 8B F1 E8 77 D4 FF FF",
       unverified = "mvpc-ai-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "boot-time element constructor run by the CRT initialiser of the vehicle model-info store")]
pub fn vehicle_model_info_ctor(this: GPtr<u8>) -> GPtr<u8> {
    base_model_info_ctor(this);
    wr::<u32>(this, 0x5C, 0);
    wr::<u8>(this, 0x2D0, 0);
    wr::<u8>(this, 0x4E, 0);
    wr::<u32>(this, 0, 0x85C5C8);
    wr::<u32>(this, 0x304, 0xFFFF_FFFF);
    for k in 0..9isize {
        wr::<u32>(this, 0x2D6 + 4 * k, 0xFFFF_FFFF);
    }
    wr::<u32>(this, 0x2FA, 0xFFFF_FFFF);
    wr::<u32>(this, 0x2FE, 0xFFFF_FFFF);
    this
}

/// `FUN_004c94c0` (0x4C94C0): `CPool<CVehicleStructure>::New` (0x314-byte slots), the same probe as
/// `CTaskPool::New`: advance the cursor (+0xC) to the next free slot (flag bit 0x80), wrapping once to 0 (a second
/// wrap returns null); the slot's flag byte is then marked used and its 7-bit ID incremented. Returns the slot.
#[hook(addr = 0x4C94C0, conv = thiscall, name = "FUN_004c94c0", expect = "8B 51 08 56 32 C0",
       unverified = "mvpc-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn vehicle_structure_pool_new(this: GPtr<u8>) -> GPtr<u8> {
    let size = rd::<i32>(this, 8);
    let mut wrapped = false;
    loop {
        let i = rd::<i32>(this, 0xC).wrapping_add(1);
        wr::<i32>(this, 0xC, i);
        if i == size {
            wr::<i32>(this, 0xC, 0);
            if wrapped {
                return GPtr::NULL;
            }
            wrapped = true;
        }
        let flags = rd::<GPtr<u8>>(this, 4);
        let i = rd::<i32>(this, 0xC) as isize;
        if rd::<u8>(flags, i) & 0x80 != 0 {
            break;
        }
    }
    let i = rd::<i32>(this, 0xC) as isize;
    let flags = rd::<GPtr<u8>>(this, 4);
    wr::<u8>(flags, i, rd::<u8>(flags, i) & 0x7F);
    let i = rd::<i32>(this, 0xC) as isize;
    let flags = rd::<GPtr<u8>>(this, 4);
    let b = rd::<u8>(flags, i);
    wr::<u8>(flags, i, (b & 0x80) | (b.wrapping_add(1) & 0x7F));
    let i = rd::<i32>(this, 0xC);
    rd::<GPtr<u8>>(this, 0).byte_add(i.wrapping_mul(0x314) as isize)
}

/// `FUN_004c7410` (0x4C7410): the `CVehicleStructure` destructor: for each of the i8 count (+0x30C, re-read every
/// pass) extra atomics (+0x2F4): `RpAtomicDestroy(atomic)` then `RwFrameDestroy` of its frame (read first); then the
/// 0x12 sub-objects at +0xB4 (0x20 each) through the CRT vector destructor with the empty 0x4C7400.
#[hook(addr = 0x4C7410, conv = thiscall, name = "FUN_004c7410", expect = "55 8B E9 8A 85 0C 03 00 00",
       unverified = "mvpc-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn vehicle_structure_dtor(this: GPtr<u8>) {
    let mut i = 0isize;
    while (i as i32) < rd::<i8>(this, 0x30C) as i32 {
        let a = rd::<GPtr<u8>>(this, 0x2F4 + 4 * i);
        let f = rd::<GPtr<u8>>(a, 4);
        rp_atomic_destroy(a);
        rw_frame_destroy(f);
        i += 1;
    }
    eh_vector_dtor(this.byte_add(0xB4), 0x20, 0x12, 0x4C7400);
}

/// The CRT-initialiser store constructors (`CStore<T, N>::CStore`): the CRT vector constructor over the N elements at
/// +4, then the count (+0) = 0. Returns `this`.
fn store_ctor(this: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) -> GPtr<u8> {
    eh_vector_ctor(this.byte_add(4), size, n, ctor, dtor);
    wr::<u32>(this, 0, 0);
    this
}

/// `FUN_004c5cb0` (0x4C5CB0): `CStore<CAtomicModelInfo, 14000>` (0x20 each; ctor 0x4C5540, dtor 0x4C5590).
#[hook(addr = 0x4C5CB0, conv = thiscall, name = "FUN_004c5cb0", expect = "56 68 90 55 4C 00",
       unverified = "mvpc-ai-1", route = "split", hazards = "none", recs = "grove-walk", note = "one-time CRT initialiser")]
pub fn store_atomic_ctor(this: GPtr<u8>) -> GPtr<u8> {
    store_ctor(this, 0x20, 0x36B0, 0x4C5540, 0x4C5590)
}

/// `FUN_004c5d10` (0x4C5D10): `CStore<CDamageAtomicModelInfo, 70>` (0x24 each; ctor 0x4C55A0, dtor 0x4C55D0).
#[hook(addr = 0x4C5D10, conv = thiscall, name = "FUN_004c5d10", expect = "56 68 D0 55 4C 00",
       unverified = "mvpc-ai-1", route = "split", hazards = "none", recs = "grove-walk", note = "one-time CRT initialiser")]
pub fn store_damage_atomic_ctor(this: GPtr<u8>) -> GPtr<u8> {
    store_ctor(this, 0x24, 0x46, 0x4C55A0, 0x4C55D0)
}

/// `FUN_004c5d70` (0x4C5D70): `CStore<CLodAtomicModelInfo, 1>` (0x24; ctor 0x4C55E0, dtor 0x4C5620).
#[hook(addr = 0x4C5D70, conv = thiscall, name = "FUN_004c5d70", expect = "56 68 20 56 4C 00",
       unverified = "mvpc-ai-1", route = "split", hazards = "none", recs = "grove-walk", note = "one-time CRT initialiser")]
pub fn store_lod_atomic_ctor(this: GPtr<u8>) -> GPtr<u8> {
    store_ctor(this, 0x24, 1, 0x4C55E0, 0x4C5620)
}

/// `FUN_004c5dd0` (0x4C5DD0): `CStore<CTimeModelInfo, 169>` (0x24 each; ctor 0x4C5640, dtor 0x4C5680).
#[hook(addr = 0x4C5DD0, conv = thiscall, name = "FUN_004c5dd0", expect = "56 68 80 56 4C 00",
       unverified = "mvpc-ai-1", route = "split", hazards = "none", recs = "grove-walk", note = "one-time CRT initialiser")]
pub fn store_time_ctor(this: GPtr<u8>) -> GPtr<u8> {
    store_ctor(this, 0x24, 0xA9, 0x4C5640, 0x4C5680)
}

/// `FUN_004c5e30` (0x4C5E30): `CStore<CLodTimeModelInfo, 1>` (0x28; ctor 0x4C5690, dtor 0x4C56E0).
#[hook(addr = 0x4C5E30, conv = thiscall, name = "FUN_004c5e30", expect = "56 68 E0 56 4C 00",
       unverified = "mvpc-ai-1", route = "split", hazards = "none", recs = "grove-walk", note = "one-time CRT initialiser")]
pub fn store_lod_time_ctor(this: GPtr<u8>) -> GPtr<u8> {
    store_ctor(this, 0x28, 1, 0x4C5690, 0x4C56E0)
}

/// `FUN_004c5e90` (0x4C5E90): `CStore<CWeaponModelInfo, 51>` (0x28 each; ctor 0x4C5760, dtor 0x4C5790).
#[hook(addr = 0x4C5E90, conv = thiscall, name = "FUN_004c5e90", expect = "56 68 90 57 4C 00",
       unverified = "mvpc-ai-1", route = "split", hazards = "none", recs = "grove-walk", note = "one-time CRT initialiser")]
pub fn store_weapon_ctor(this: GPtr<u8>) -> GPtr<u8> {
    store_ctor(this, 0x28, 0x33, 0x4C5760, 0x4C5790)
}

/// `FUN_004c5ef0` (0x4C5EF0): `CStore<CClumpModelInfo, 92>` (0x24 each; ctor 0x4C56F0, dtor 0x4C5750).
#[hook(addr = 0x4C5EF0, conv = thiscall, name = "FUN_004c5ef0", expect = "56 68 50 57 4C 00",
       unverified = "mvpc-ai-1", route = "split", hazards = "none", recs = "grove-walk", note = "one-time CRT initialiser")]
pub fn store_clump_ctor(this: GPtr<u8>) -> GPtr<u8> {
    store_ctor(this, 0x24, 0x5C, 0x4C56F0, 0x4C5750)
}

/// `FUN_004c5f50` (0x4C5F50): `CStore<CVehicleModelInfo, 212>` (0x308 each; ctor 0x4C75E0, dtor 0x4C5920).
#[hook(addr = 0x4C5F50, conv = thiscall, name = "FUN_004c5f50", expect = "56 68 20 59 4C 00",
       unverified = "mvpc-ai-1", route = "split", hazards = "none", recs = "grove-walk", note = "one-time CRT initialiser")]
pub fn store_vehicle_ctor(this: GPtr<u8>) -> GPtr<u8> {
    store_ctor(this, 0x308, 0xD4, 0x4C75E0, 0x4C5920)
}

/// `FUN_004c67d0` (0x4C67D0): `CStore<CPedModelInfo, 278>` (0x44 each; ctor 0x4C57A0, dtor 0x4C62F0).
#[hook(addr = 0x4C67D0, conv = thiscall, name = "FUN_004c67d0", expect = "56 68 F0 62 4C 00",
       unverified = "mvpc-ai-1", route = "split", hazards = "none", recs = "grove-walk", note = "one-time CRT initialiser")]
pub fn store_ped_ctor(this: GPtr<u8>) -> GPtr<u8> {
    store_ctor(this, 0x44, 0x116, 0x4C57A0, 0x4C62F0)
}

/// `FUN_0066d440` (0x66D440): the constructor run by `__init_g_c18db8`: +0 = 0, the CRT vector constructor over 8
/// elements of 0x28 at +4 (ctor 0x46AE30, dtor 0x46A380), then the bytes +4, +0x1C, +0x2C, +0x44, +0x54, +0x6C,
/// +0x7C, +0x94, +0xA4, +0xBC, +0xCC, +0xE4, +0xF4, +0x10C, +0x11C and +0x134 = 0. Returns `this`.
#[hook(addr = 0x66D440, conv = thiscall, name = "FUN_0066d440", expect = "53 56 57 68 80 A3 46 00",
       unverified = "mvpc-ai-1", route = "split", hazards = "none", recs = "grove-walk", note = "one-time CRT initialiser")]
pub fn init_c18db8_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0);
    eh_vector_ctor(this.byte_add(4), 0x28, 8, 0x46AE30, 0x46A380);
    for off in [4isize, 0x1C, 0x2C, 0x44, 0x54, 0x6C, 0x7C, 0x94, 0xA4, 0xBC, 0xCC, 0xE4, 0xF4, 0x10C, 0x11C, 0x134] {
        wr::<u8>(this, off, 0);
    }
    this
}

/// `FUN_00607d80` (0x607D80, `ret 0xC`): the pedestrian-acquaintance scanner (`CEventScanner` +0xAC) over the ped's
/// nearby peds (`CPedIntelligence` +0x130, 16 entries). Its `CTaskTimer` (+0 start, +4 interval, +8 started, +9
/// reset): not started -> started now with interval [0x8D2358]; a pending reset restarts it now. Unless `now >=
/// start + interval` (unsigned): nothing. Then restart (start = now, interval = [0x8D2358]) and, when
/// `FUN_00603a30(ped)` allows, scan with `FUN_00607a90(ped, -1, entities, count, &0, &-1)`. The two out
/// parameters are the original's scratch slot (0) and its own first argument slot (overwritten with -1).
#[hook(addr = 0x607D80, conv = thiscall, name = "FUN_00607d80", expect = "51 56 8B F1 8A 46 08",
       unverified = "mvpc-ai-1", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "overwrites its own first argument slot with -1 and passes its address on")]
pub fn acq_scanner_scan(this: GPtr<u8>, ped: GPtr<u8>, ents: GPtr<u8>, n: i32) {
    if rd::<u8>(this, 8) == 0 {
        wr::<u32>(this, 4, g::<u32>(0x8D2358).read());
        wr::<u32>(this, 0, g::<u32>(TIME_MS).read());
        wr::<u8>(this, 8, 1);
    }
    if rd::<u8>(this, 9) != 0 {
        wr::<u32>(this, 0, g::<u32>(TIME_MS).read());
        wr::<u8>(this, 9, 0);
    }
    let end = rd::<u32>(this, 4).wrapping_add(rd::<u32>(this, 0));
    let now = g::<u32>(TIME_MS).read();
    if now < end {
        return;
    }
    wr::<u32>(this, 0, now);
    wr::<u32>(this, 4, g::<u32>(0x8D2358).read());
    wr::<u8>(this, 8, 1);
    if acq_scanner_can_scan_o(this, ped) != 0 {
        let mut outs = [0u32, 0xFFFF_FFFF];
        let o = local(&mut outs);
        acq_scanner_scan_entities(this, ped, -1, ents, n, o, o.byte_add(4));
    }
}

/// `FUN_00603a30` (0x603A30, `ret 4`, AL): may the acquaintance scanner (flags +0xC script peds, +0xD in vehicle, +0xE
/// scripted task) scan `ped`. A dead ped: false (`IsAlive`'s EAX). A non-mission ped (+0x484 != 2) or flag +0xC:
/// allowed = true; else allowed = (ped flag 0x100 at +0x46C and +0xD), forced true by a scripted-task status
/// (`CPedScriptedTaskRecord::GetStatus` != 0) with +0xE; not allowed: false. An allowed ped whose current event
/// (history at intel +0x38) is type 0x24 is refused when intel +0xC4 is 0, when the event has no response of type
/// 0x6A4, or when no respected friend is in inform range.
#[hook(addr = 0x603A30, conv = thiscall, name = "FUN_00603a30", expect = "56 8B 74 24 08 57 8B F9",
       unverified = "mvpc-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn acq_scanner_can_scan(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let alive = ped_is_alive(ped);
    if alive == 0 {
        return alive;
    }
    let allowed = if rd::<u8>(ped, 0x484) != 2 || rd::<u8>(this, 0xC) != 0 {
        1u8
    } else {
        let mut b = (rd::<u32>(ped, 0x46C) & 0x100 != 0 && rd::<u8>(this, 0xD) != 0) as u8;
        if scripted_task_get_status(ped) != 0 && rd::<u8>(this, 0xE) != 0 {
            b = 1;
        }
        if b == 0 {
            return 0;
        }
        b
    };
    let ev = history_get_current_event(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x38));
    if ev.is_null() || v_get_event_type(ev) != 0x24 {
        return allowed;
    }
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    if rd::<u32>(intel, 0xC4) == 0 || editable_compute_response_task_of_type(ev, ped, 0x6A4) == 0 {
        return 0;
    }
    if intel_find_respected_friend_in_inform_range(rd::<GPtr<u8>>(ped, 0x47C)) != 0 {
        return allowed;
    }
    0
}

/// `FUN_006042b0` (0x6042B0, `ret 0xC`, AL): on the decision-maker store (`CDecisionMakerTypes`), does `ped`'s
/// decision maker (intel +0xB4) give any of the `count` event types in `events` a response: per event, the
/// decision index is `this[0xC034 + 4*event]`; the 0x3C-byte decision is the group default (+0xD4EC) for -2, the
/// mission-ped (+0x484 == 2, +0xCB50) or random-ped (+0xC1B4) default for -1, else `this + 4 + 0x3C*(0x29*dm +
/// index)`; it responds when one of its six task types is not 200. Stops at the first event that responds.
#[hook(addr = 0x6042B0, conv = thiscall, name = "FUN_006042b0", expect = "8B 44 24 04 55 56 8B F1",
       unverified = "mvpc-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn decision_maker_types_has_response(this: GPtr<u8>, ped: GPtr<u8>, events: GPtr<u8>, count: i32) -> u8 {
    let dm = rd::<i32>(rd::<GPtr<u8>>(ped, 0x47C), 0xB4);
    let mut found = 0u8;
    let mut k = 0i32;
    while k < count && found == 0 {
        let ev = rd::<i32>(events, 4 * k as isize);
        let idx = rd::<i32>(this, 0xC034 + 4 * ev as isize);
        k += 1;
        let base = match dm {
            -2 => 0xD4EC + idx * 0x3C,
            -1 if rd::<u8>(ped, 0x484) == 2 => 0xCB50 + idx * 0x3C,
            -1 => 0xC1B4 + idx * 0x3C,
            _ => 4 + (dm * 0x29 + idx) * 0x3C,
        };
        let d = this.byte_add(base as isize);
        found = (0..6isize).any(|j| rd::<i32>(d, 4 * j) != 0xC8) as u8;
    }
    found
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_EVENT_HANDLER_COMPUTE_EVENT_RESPONSE_TASK,
    HOOK_EVENT_HANDLER_SET_EVENT_RESPONSE_TASK,
    HOOK_EVENT_HANDLER_COMPUTE_VEHICLE_POTENTIAL_PASSIVE_COLLISION_RESPONSE,
    HOOK_EVENT_HANDLER_COMPUTE_SCRIPT_COMMAND_RESPONSE,
    HOOK_INFORM_GROUP_EVENT_QUEUE_ADD,
    HOOK_CONVERSATIONS_START_SETTING_UP_CONVERSATION,
    HOOK_CONVERSATIONS_ENABLE_CONVERSATION,
    HOOK_COMPUTE_ENTITY_BOUNDING_BOX_CORNERS,
    HOOK_ANIM_BLEND_ASSOCIATION_DTOR,
    HOOK_ANIM_BLEND_ASSOCIATION_DELETING_DTOR,
    HOOK_TASK_SIMPLE_SCRATCH_HEAD_CTOR,
    HOOK_TASK_SIMPLE_SCRATCH_HEAD_DTOR,
    HOOK_VEHICLE_MODEL_INFO_CTOR,
    HOOK_VEHICLE_STRUCTURE_POOL_NEW,
    HOOK_VEHICLE_STRUCTURE_DTOR,
    HOOK_STORE_ATOMIC_CTOR,
    HOOK_STORE_DAMAGE_ATOMIC_CTOR,
    HOOK_STORE_LOD_ATOMIC_CTOR,
    HOOK_STORE_TIME_CTOR,
    HOOK_STORE_LOD_TIME_CTOR,
    HOOK_STORE_WEAPON_CTOR,
    HOOK_STORE_CLUMP_CTOR,
    HOOK_STORE_VEHICLE_CTOR,
    HOOK_STORE_PED_CTOR,
    HOOK_INIT_C18DB8_CTOR,
    HOOK_ACQ_SCANNER_SCAN,
    HOOK_ACQ_SCANNER_CAN_SCAN,
    HOOK_DECISION_MAKER_TYPES_HAS_RESPONSE,
];
