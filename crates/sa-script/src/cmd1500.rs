//! `CRunningScript::ProcessCommands1500To1599` (0x490DB0): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-4). Every opcode with a jump-table case (0x5DC..0x639, table 0x492AC8, 94 entries) is in [`PORTED`];
//! the default (-1, `or al,0xFF` at 0x492AA9; 063A..063F and other command values) is kept. The common exit
//! 0x492AAB only restores the SEH link (no destructor tail).
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands1500To1599.md`.
//!
//! The task cases build the task with `CTask::operator new` + the constructor (EAX = the task, null when `new`
//! fails) and give it with `GivePedScriptedTask(ped, task, command)` (the shared tails 0x492A90 / 0x490ED4). The SEH
//! state stores around `new` + ctor are not ported. Stack objects (the `CEventScriptCommand`s, 0631's
//! `CEventLeaderEnteredCarAsDriver` / `CEventGroupEvent` / `CTaskComplexEnterCarAsPassenger` /
//! `CTaskComplexSequence`, 0638's `CTaskSimpleSetStayInSamePlace`) are built on zeroed Rust buffers (PORT-SCRIPT-3
//! decision).
//!
//! Divergence notes (ledger classes of PORT-SCRIPT-2/3): the 24-byte animation names (0611..0614, 061A) are read into
//! zeroed buffers; `setcc` bools pushed with stale upper bytes are passed as 0/1; 060A's decision-maker name buffer
//! starts zeroed (the callee writes it); FSIN/FCOS (061D) at 53 bits (`X87::fsin/fcos`); FPATAN (0604, the
//! `GetATanOfXY` ST0 read as f64) at 53 bits.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x05DC, 0x05DD, 0x05DE, 0x05DF, 0x05E0, 0x05E1, 0x05E2, 0x05E3, 0x05E4, 0x05E5, 0x05E6, 0x05E7, 0x05E8,
    0x05E9, 0x05EA, 0x05EB, 0x05EC, 0x05ED, 0x05EE, 0x05EF, 0x05F0, 0x05F1, 0x05F2, 0x05F3, 0x05F4, 0x05F5,
    0x05F6, 0x05F7, 0x05F8, 0x05F9, 0x05FA, 0x05FB, 0x05FC, 0x05FD, 0x05FE, 0x05FF, 0x0600, 0x0601, 0x0602,
    0x0603, 0x0604, 0x0605, 0x0606, 0x0607, 0x0608, 0x0609, 0x060A, 0x060B, 0x060C, 0x060D, 0x060E, 0x060F,
    0x0610, 0x0611, 0x0612, 0x0613, 0x0614, 0x0615, 0x0616, 0x0617, 0x0618, 0x0619, 0x061A, 0x061B, 0x061C,
    0x061D, 0x061E, 0x061F, 0x0620, 0x0621, 0x0622, 0x0623, 0x0624, 0x0625, 0x0626, 0x0627, 0x0628, 0x0629,
    0x062A, 0x062B, 0x062C, 0x062D, 0x062E, 0x062F, 0x0630, 0x0631, 0x0632, 0x0633, 0x0634, 0x0635, 0x0636,
    0x0637, 0x0638, 0x0639,
];

/// `.rdata`: 0x859878 57.295776f (180/pi), 0x85991C 90.0f, 0x859E2C 360.0f, 0x858B50 0.0f, 0x8595EC pi/180,
/// 0x85A318 1000.0 (double).
const K_RAD_TO_DEG: u32 = 0x4265_2EE0;
const K_90: u32 = 0x42B4_0000;
const K_360: u32 = 0x43B4_0000;
const K_ZERO: u32 = 0;
const K_DEG_TO_RAD: u32 = 0x3C8E_FA35;
/// `ThePaths` 0x96F050.
const THE_PATHS: usize = 0x96F050;
/// Task-constructor defaults read at run time as the original does: 0x86FCAC (int 50000, the follow-route time
/// for -1), 0x86FCA0/A4/A8 (0.5f, 3.0f, 2.0f), 0xC18DB4, 0xC18CF0, 0x86F678 (int 1000), 0x86FC7C/0x86FC80.
const FOLLOW_ROUTE_DEFAULT_TIME: usize = 0x86FCAC;
/// `CTaskSequences::ms_taskSequence[0]` (`CTaskComplexSequence`, 0x40 bytes) 0xC178F0; `ms_bIsOpened[i]` 0xC17898;
/// `ms_iActiveSequence` 0x8D2E98.
const TASK_SEQUENCES: u32 = 0xC178F0;
const SEQUENCE_OPENED: u32 = 0xC17898;
const ACTIVE_SEQUENCE: usize = 0x8D2E98;
/// `CPedGroups::ms_groups[0]` (`CPedGroup`, 0x2D4 bytes) 0xC09920: membership +8 (0xC09928), intelligence +0x30
/// (0xC09950), byte +0x2D0 (0xC09BF0); `ms_activeGroups[i]` 0xC098E0.
const PED_GROUPS: u32 = 0xC09920;
const ACTIVE_GROUPS: u32 = 0xC098E0;
/// `CScripted2dEffects::ms_effects[0]` (0x40 bytes) 0xC3AB00; `ms_effectSequenceTaskIDs[i]` 0xC3A020; `ms_activated[i]`
/// 0xC3A1A0.
const SCRIPTED_EFFECTS: u32 = 0xC3AB00;
const EFFECT_SEQUENCE_IDS: u32 = 0xC3A020;
const EFFECT_ACTIVATED: u32 = 0xC3A1A0;
/// `CPedScriptedTaskRecord::ms_scriptedTasks[0]` (0x14 bytes) 0xC0B1E8.
const SCRIPTED_TASK_RECORDS: u32 = 0xC0B1E8;
/// The mission-ped counter 0xC0EC24.
const MISSION_PED_COUNT: usize = 0xC0EC24;
/// The list head 0xB9ACC8 walked by SET_SENSE_RANGE (nodes: +0 the entity, +4 the next node).
const SENSE_RANGE_LIST: usize = 0xB9ACC8;
/// `CTheScripts::IntroTextLines[0]` (0x44 bytes): drop-shadow colour bytes at 0xA91405..0xA91408, shadow size byte
/// 0xA91409; `NumberOfIntroTextLinesThisFrame` (u16) 0xA44B68.
const INTRO_TEXT_SHADOW: u32 = 0xA91405;
const NUM_INTRO_TEXT_LINES: usize = 0xA44B68;
/// The per-sequence byte array cleared by CLEAR_SEQUENCE_TASK (0xA43F68, 4-byte stride).
const SEQUENCE_ID_BYTES: u32 = 0xA43F68;

orig! {
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CRunningScript::GivePedScriptedTask(int32 ped, CTask*, int32 command)` (0x465C20)
    fn give_ped_scripted_task(s: Scr, ped: u32, task: GPtr<u8>, command: u32) = thiscall 0x465C20 => unported;
    /// `CTaskComplexSmartFleePoint::CTaskComplexSmartFleePoint(const CVector&, bool, float, int32)` (0x65BD20)
    fn task_smart_flee_point_ctor(t: GPtr<u8>, pos: GPtr<u32>, b: u32, range: u32, time: u32) -> GPtr<u8> = thiscall 0x65BD20 => unported;
    /// `CTaskComplexSmartFleeEntity::CTaskComplexSmartFleeEntity(CEntity*, bool, float, int32, int32, float)`
    /// (0x65C430); floats raw
    fn task_smart_flee_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, b: u32, range: u32, time: u32, a: u32, c: u32) -> GPtr<u8> = thiscall 0x65C430 => unported;
    /// `CTaskComplexWanderStandard::CTaskComplexWanderStandard(int32 moveState, uint8 dir, bool)` (0x48E4F0); the
    /// direction dword as pushed
    fn task_wander_standard_ctor(t: GPtr<u8>, move_state: u32, dir: u32, b: u32) -> GPtr<u8> = thiscall 0x48E4F0 => unported;
    /// `CTaskComplexWanderCriminal::CTaskComplexWanderCriminal(int32, uint8)` (0x48E610)
    fn task_wander_criminal_ctor(t: GPtr<u8>, move_state: u32, dir: u32, b: u32) -> GPtr<u8> = thiscall 0x48E610 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn random_int_in_range(lo: i32, hi: i32) -> u32 = cdecl 0x407180 => unported;
    /// `CTaskComplexKillPedOnFoot::CTaskComplexKillPedOnFoot(CPed*, int32, uint32, uint32, uint32, uint8)` (0x620E30)
    fn task_kill_ped_on_foot_ctor(t: GPtr<u8>, p: GPtr<u8>, a: u32, b: u32, c: u32, d: u32, e: u32) -> GPtr<u8> = thiscall 0x620E30 => unported;
    /// `CVehicleRecording::StartPlaybackRecordedCar(CVehicle*, int32, bool, bool)` (0x45A980)
    fn start_playback_recorded_car(v: GPtr<u8>, n: u32, a: u32, b: u32) = cdecl 0x45A980 => unported;
    /// `CVehicleRecording::StopPlaybackRecordedCar(CVehicle*)` (0x45A280) / `PausePlaybackRecordedCar` (0x459740) /
    /// `UnpausePlaybackRecordedCar` (0x459850)
    fn stop_playback_recorded_car(v: GPtr<u8>) = cdecl 0x45A280 => unported;
    fn pause_playback_recorded_car(v: GPtr<u8>) = cdecl 0x459740 => unported;
    fn unpause_playback_recorded_car(v: GPtr<u8>) = cdecl 0x459850 => unported;
    /// `CVehicleRecording::IsPlaybackGoingOnForCar(CVehicle*)` (0x4594C0), AL
    fn is_playback_going_on_for_car(v: GPtr<u8>) -> u8 = cdecl 0x4594C0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CTaskComplexFollowNodeRoute::CTaskComplexFollowNodeRoute(int32 mode, const CVector&, float, float, float,
    /// bool, int32 time, bool)` (0x66EA30); floats raw
    fn task_follow_node_route_ctor(
        t: GPtr<u8>, mode: u32, pos: GPtr<u32>, a: u32, b: u32, c: u32, d: u32, time: u32, e: u32
    ) -> GPtr<u8> = thiscall 0x66EA30 => unported;
    /// `CRunningScript::CharInAngledAreaCheckCommand(int32 command)` (0x487F60)
    fn char_in_angled_area_check_command(s: Scr, command: u32) = thiscall 0x487F60 => unported;
    /// `CTaskComplexGoToPointAnyMeans::CTaskComplexGoToPointAnyMeans(int32, const CVector&, CVehicle*, float, int32)`
    /// (0x66B790); the float raw
    fn task_go_to_point_any_means_ctor(t: GPtr<u8>, mode: u32, pos: GPtr<u32>, v: GPtr<u8>, r: u32, m: u32) -> GPtr<u8> = thiscall 0x66B790 => unported;
    /// `CGeneral::GetATanOfXY(float, float)` (0x53CC70), ST0 (read as f64)
    fn get_atan_of_xy(x: u32, y: u32) -> f64 = cdecl 0x53CC70 => unported;
    /// `CRunningScript::PlayAnimScriptCommand(int32 command)` (0x470150; reads its own parameters)
    fn play_anim_script_command(s: Scr, command: u32) = thiscall 0x470150 => unported;
    /// `CPathFind::MakeRequestForNodesToBeLoaded(float, float, float, float)` (0x450D70); raw floats
    fn make_request_for_nodes_to_be_loaded(p: GPtr<u8>, x1: u32, x2: u32, y1: u32, y2: u32) = thiscall 0x450D70 => unported;
    /// `CPathFind::ReleaseRequestedNodes()` (0x44DD00)
    fn release_requested_nodes(p: GPtr<u8>) = thiscall 0x44DD00 => unported;
    /// `CDecisionMakerTypesFileLoader::GetPedDMName(int32, char*)` (0x600860)
    fn get_ped_dm_name(i: u32, out: GPtr<u8>) = cdecl 0x600860 => unported;
    /// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0)
    fn get_actual_script_thing_index(r: u32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `CDecisionMakerTypesFileLoader::LoadDecisionMaker(const char*, eDecisionTypes, bool)` (0x607D30)
    fn load_decision_maker(name: GPtr<u8>, ty: u32, cleanup: u32) -> u32 = cdecl 0x607D30 => unported;
    /// `CTheScripts::GetNewUniqueScriptThingIndex(int32 index, uint8 type)` (0x483720)
    fn get_new_unique_script_thing_index(i: u32, ty: u32) -> u32 = cdecl 0x483720 => unported;
    /// `CPedIntelligence::SetPedDecisionMakerType(int32)` (0x600B50)
    fn set_ped_decision_maker_type(intel: GPtr<u8>, ty: i32) = thiscall 0x600B50 => unported;
    /// `CRGBA::CRGBA(uint8, uint8, uint8, uint8)` (0x7170C0), returns this
    fn rgba_ctor(c: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CPedIntelligence::SetSeeingRange(float)` (0x600BF0) / `SetHearingRange(float)` (0x600BE0); raw
    fn set_seeing_range(i: GPtr<u8>, r: u32) = thiscall 0x600BF0 => unported;
    fn set_hearing_range(i: GPtr<u8>, r: u32) = thiscall 0x600BE0 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, const char*)` (0x4D6870)
    fn rp_anim_blend_clump_get_association(clump: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D6870 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80)
    fn set_current_time(a: GPtr<u8>, t: f32) = thiscall 0x4CEA80 => unported;
    /// `CTaskSequences::GetAvailableSlot(bool)` (0x632E00)
    fn get_available_sequence_slot(b: u32) -> i32 = cdecl 0x632E00 => unported;
    /// `CTaskComplexSequence::Flush()` (0x632C10)
    fn sequence_flush(seq: GPtr<u8>) = thiscall 0x632C10 => unported;
    /// `CTaskComplexUseSequence::CTaskComplexUseSequence(int32)` (0x635450)
    fn task_use_sequence_ctor(t: GPtr<u8>, i: u32) -> GPtr<u8> = thiscall 0x635450 => unported;
    /// `CScripted2dEffects::AddScripted2DEffect(float)` (0x6FA7C0); the float raw
    fn add_scripted_2d_effect(r: u32) -> i32 = cdecl 0x6FA7C0 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u32>) = thiscall 0x59C910 => unported;
    /// `CRunningScript::GetCorrectPedModelIndexForEmergencyServiceType(ePedType, int32*)` (0x464F50)
    fn get_correct_ped_model_index(s: Scr, ped_type: u32, model: GPtr<u32>) = thiscall 0x464F50 => unported;
    /// `CPed::operator new(uint32)` (0x5E4720)
    fn ped_new(size: u32) -> GPtr<u8> = cdecl 0x5E4720 => unported;
    /// `CCopPed::CCopPed(eCopType)` (0x5DDC60) / `CCivilianPed::CCivilianPed(ePedType, uint32)` (0x5DDB70) /
    /// `CEmergencyPed::CEmergencyPed(ePedType, uint32)` (0x5DE340), each returns this
    fn cop_ped_ctor(p: GPtr<u8>, cop_type: u32) -> GPtr<u8> = thiscall 0x5DDC60 => unported;
    fn civilian_ped_ctor(p: GPtr<u8>, ped_type: u32, model: u32) -> GPtr<u8> = thiscall 0x5DDB70 => unported;
    fn emergency_ped_ctor(p: GPtr<u8>, ped_type: u32, model: u32) -> GPtr<u8> = thiscall 0x5DE340 => unported;
    /// `CPed::SetCharCreatedBy(uint8)` (0x5E47E0)
    fn set_char_created_by(p: GPtr<u8>, by: u32) = thiscall 0x5E47E0 => unported;
    /// `CPedAttractorPedPlacer::PlacePedAtEffect(const C2dEffect&, CEntity*, CPed*, float)` (0x5EA390); the float raw
    fn place_ped_at_effect(fx: GPtr<u8>, e: u32, p: GPtr<u8>, d: u32) = cdecl 0x5EA390 => unported;
    /// `CTheScripts::ClearSpaceForMissionEntity(const CVector&, CEntity*)` (0x486B00)
    fn clear_space_for_mission_entity(pos: GPtr<u32>, e: GPtr<u8>) = cdecl 0x486B00 => unported;
    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(int32, bool, bool, float)` (0x62F310); the float raw
    fn task_stand_still_ctor(t: GPtr<u8>, time: u32, looped: u32, hold: u32, delta: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0)
    fn task_manager_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: u32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CTaskComplexUseEffect::CTaskComplexUseEffect(C2dEffect*, CEntity*)` (0x6321F0)
    fn task_use_effect_ctor(t: GPtr<u8>, effect: GPtr<u8>, e: u32) -> GPtr<u8> = thiscall 0x6321F0 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32 primaryTaskIndex, CTask*, bool)` (0x4B0A00)
    fn event_script_command_ctor(e: GPtr<u8>, index: u32, task: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventScriptCommand::~CEventScriptCommand()` (0x4B0A50, unnamed in the catalogue)
    fn event_script_command_dtor(e: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CPedScriptedTaskRecord::GetVacantSlot()` (0x608540)
    fn scripted_task_record_get_vacant_slot() -> u32 = cdecl 0x608540 => unported;
    /// `CPedScriptedTaskRecordData::Set(CPed*, int32 opcode, CEventScriptCommand*)` (0x608390)
    fn scripted_task_record_set(r: GPtr<u8>, p: GPtr<u8>, command: u32, e: GPtr<u8>) = thiscall 0x608390 => unported;
    /// `CTaskComplexLeaveCar::CTaskComplexLeaveCar(CVehicle*, int32 door, int32 delay, bool, bool)` (0x63B8C0)
    fn task_leave_car_ctor(t: GPtr<u8>, v: GPtr<u8>, door: u32, delay: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    /// `CStats::IncrementStat(uint16, float)` (0x55C180) / `DecrementStat` (0x559FA0) / `SetNewRecordStat` (0x55C410)
    /// / `SetStatValue` (0x55A070); the float raw
    fn increment_stat(stat: u32, v: u32) = cdecl 0x55C180 => unported;
    fn decrement_stat(stat: u32, v: u32) = cdecl 0x559FA0 => unported;
    fn set_new_record_stat(stat: u32, v: u32) = cdecl 0x55C410 => unported;
    fn set_stat_value(stat: u32, v: u32) = cdecl 0x55A070 => unported;
    /// `CStats::DisplayScriptStatUpdateMessage(uint8 state, uint32 stat, float value)` (0x55B980); the float raw
    fn display_script_stat_update_message(state: u32, stat: u32, v: u32) = cdecl 0x55B980 => unported;
    /// `CPedScriptedTaskRecord::GetStatus(const CPed*, int32 opcode)` (0x608710)
    fn scripted_task_record_get_status(p: GPtr<u8>, command: u32) -> i32 = cdecl 0x608710 => unported;
    /// `CPedGroups::AddGroup()` (0x5FB800)
    fn ped_groups_add_group() -> u32 = cdecl 0x5FB800 => unported;
    /// `CPedGroupIntelligence::SetDefaultTaskAllocatorType(int32)` (0x5FBB70)
    fn set_default_task_allocator_type(gi: GPtr<u8>, ty: u32) = thiscall 0x5FBB70 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0), AL
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CTaskComplexBeInGroup::CTaskComplexBeInGroup(int32 group, bool isLeader)` (0x632E50)
    fn task_be_in_group_ctor(t: GPtr<u8>, g: u32, leader: u32) -> GPtr<u8> = thiscall 0x632E50 => unported;
    /// `CPedGroupMembership::SetLeader(CPed*)` (0x5FB9C0) / `AddFollower(CPed*)` (0x5F8020)
    fn membership_set_leader(m: GPtr<u8>, p: GPtr<u8>) = thiscall 0x5FB9C0 => unported;
    fn membership_add_follower(m: GPtr<u8>, p: GPtr<u8>) = thiscall 0x5F8020 => unported;
    /// `CPedGroup::Process()` (0x5FC7E0)
    fn ped_group_process(g: GPtr<u8>) = thiscall 0x5FC7E0 => unported;
    /// `CPedGroupMembership::CountMembersExcludingLeader()` (0x5F6AA0) / `GetLeader()` (0x5F69A0) /
    /// `RemoveNFollowers(int32)` (0x5FB1D0)
    fn count_members_excluding_leader(m: GPtr<u8>) -> i32 = thiscall 0x5F6AA0 => unported;
    fn membership_get_leader(m: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F69A0 => unported;
    fn remove_n_followers(m: GPtr<u8>, n: u32) = thiscall 0x5FB1D0 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0)
    fn intel_find_task_by_type(i: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `CEventLeaderEnteredCarAsDriver::CEventLeaderEnteredCarAsDriver(CVehicle*)` (0x48E1C0), returns this
    fn event_leader_entered_car_ctor(e: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x48E1C0 => unported;
    /// The `CEventLeaderEnteredCarAsDriver` destructor at 0x48E220 (unnamed in the catalogue)
    fn event_leader_entered_car_dtor_48e220(e: GPtr<u8>) = thiscall 0x48E220 => unported;
    /// `CEventEditableResponse::ComputeResponseTaskType(CPedGroup*)` (0x4B57A0)
    fn compute_response_task_type_group(e: GPtr<u8>, g: GPtr<u8>) = thiscall 0x4B57A0 => unported;
    /// `CEvent::operator new(uint32)` (0x4B5620)
    fn event_new(size: u32) -> GPtr<u8> = cdecl 0x4B5620 => unported;
    /// `CEventGroupEvent::CEventGroupEvent(CPed*, CEvent*)` (0x4ADFD0) / `~CEventGroupEvent()` (0x4AE070)
    fn event_group_event_ctor(e: GPtr<u8>, p: GPtr<u8>, ev: GPtr<u8>) -> GPtr<u8> = thiscall 0x4ADFD0 => unported;
    fn event_group_event_dtor(e: GPtr<u8>) = thiscall 0x4AE070 => unported;
    /// `CPedGroupIntelligence::AddEvent(CEvent*)` (0x5F7470), AL (discarded)
    fn group_intel_add_event(gi: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x5F7470 => unported;
    /// `CTaskComplexEnterCarAsPassenger::CTaskComplexEnterCarAsPassenger(CVehicle*, int32 seat, bool)` (0x640340)
    fn task_enter_car_as_passenger_ctor(t: GPtr<u8>, v: GPtr<u8>, seat: u32, b: u32) -> GPtr<u8> = thiscall 0x640340 => unported;
    /// The `CTaskComplexEnterCarAsPassenger` destructor at 0x640380 (unnamed in the catalogue)
    fn task_enter_car_as_passenger_dtor_640380(t: GPtr<u8>) = thiscall 0x640380 => unported;
    /// `CPedGroupIntelligence::SetScriptCommandTask(CPed*, const CTask&)` (0x5F8560) / `SetDefaultTask` (0x5F8580)
    fn group_intel_set_script_command_task(gi: GPtr<u8>, p: GPtr<u8>, t: GPtr<u8>) = thiscall 0x5F8560 => unported;
    fn group_intel_set_default_task(gi: GPtr<u8>, p: GPtr<u8>, t: GPtr<u8>) = thiscall 0x5F8580 => unported;
    /// `CPedGroupIntelligence::GetTaskDefault(CPed*)` (0x5F86C0)
    fn group_intel_get_task_default(gi: GPtr<u8>, p: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F86C0 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence()` (0x632BD0) / `~CTaskComplexSequence()` (0x6389F0) /
    /// `AddTask(CTask*)` (0x632D10)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    fn sequence_dtor(t: GPtr<u8>) = thiscall 0x6389F0 => unported;
    fn sequence_add_task(seq: GPtr<u8>, t: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CTaskSimpleCarDrive::CTaskSimpleCarDrive(CVehicle*, CTaskUtilityLineUpPedWithCar*, bool)` (0x63C340)
    fn task_car_drive_ctor(t: GPtr<u8>, v: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x63C340 => unported;
    /// `CTask` virtual slot 1 (+4, `Clone()`)
    fn task_clone(t: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x4 => unported;
    /// `CPedGroups::RemoveAllFollowersFromGroup(int32)` (0x5FB8A0) / `RemoveGroup(int32)` (0x5FB870)
    fn remove_all_followers_from_group(g: u32) = cdecl 0x5FB8A0 => unported;
    fn remove_group(g: u32) = cdecl 0x5FB870 => unported;
    /// `CTaskComplexLeaveAnyCar::CTaskComplexLeaveAnyCar(int32, bool, bool)` (0x421150)
    fn task_leave_any_car_ctor(t: GPtr<u8>, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x421150 => unported;
    /// `CTaskSimpleGunControl::CTaskSimpleGunControl(CEntity*, const CVector*, const CVector*, eGunCommand, int16,
    /// int32)` (0x61F3F0)
    fn task_gun_control_ctor(t: GPtr<u8>, e: GPtr<u8>, a: u32, b: u32, c: u32, d: u32, e2: u32) -> GPtr<u8> = thiscall 0x61F3F0 => unported;
    /// `CTaskComplexGoToPointShooting::CTaskComplexGoToPointShooting(int32, const CVector&, CEntity*, CVector, float,
    /// float)` (0x668C70); the by-value vector and floats raw
    fn task_go_to_point_shooting_ctor(
        t: GPtr<u8>, mode: u32, pos: GPtr<u32>, e: GPtr<u8>, ox: u32, oy: u32, oz: u32, a: u32, b: u32
    ) -> GPtr<u8> = thiscall 0x668C70 => unported;
    /// `CTaskSimpleSetStayInSamePlace::CTaskSimpleSetStayInSamePlace(bool)` (0x62F590) / `ProcessPed(CPed*)` (0x62F5E0)
    /// / the destructor at 0x62F5D0
    fn task_stay_in_same_place_ctor(t: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x62F590 => unported;
    fn task_stay_in_same_place_process_ped(t: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x62F5E0 => unported;
    fn task_stay_in_same_place_dtor_62f5d0(t: GPtr<u8>) = thiscall 0x62F5D0 => unported;
    /// `CTaskComplexTurnToFaceEntityOrCoord::CTaskComplexTurnToFaceEntityOrCoord(CEntity*, float, float)` (0x66B890);
    /// floats raw
    fn task_turn_to_face_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x66B890 => unported;
}

/// `PED(h)` / `VEH(h)`.
#[inline]
fn ped(h: u32) -> GPtr<u8> {
    ped_at_ref(pool(PED_POOL), h as i32)
}
#[inline]
fn veh(h: u32) -> GPtr<u8> {
    vehicle_at_ref(pool(VEHICLE_POOL), h as i32)
}

/// A dword read from the image's data at run time.
#[inline]
fn dword(a: usize) -> u32 {
    GPtr::<u32>::from_addr(a).read()
}

/// `new(size)` then the constructor when non-null (EAX = the task, null when `new` fails).
#[inline]
fn new_task(size: u32, ctor: impl FnOnce(GPtr<u8>) -> GPtr<u8>) -> GPtr<u8> {
    let t = task_new(size);
    if t.is_null() { GPtr::NULL } else { ctor(t) }
}

/// An entity's position: `m_matrix (+0x14) ? &m_matrix->pos (+0x30) : &m_placement.pos (+4)`.
#[inline]
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = e.byte_add(0x14).cast::<GPtr<u8>>().read();
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `p->m_pIntelligence` (+0x47C).
#[inline]
fn intel(p: GPtr<u8>) -> GPtr<u8> {
    p.byte_add(0x47C).cast::<GPtr<u8>>().read()
}

/// The min/max ordering of 0606: the two dwords are swapped when `a > b` (ordered, `test ah,0x41; jne keep`).
#[inline]
fn order(a: u32, b: u32) -> (u32, u32) {
    if f(a) > f(b) { (b, a) } else { (a, b) }
}

/// The animation association of 0611..0614 / 061A: `p = PED(SP0)`; 24-byte name (zeroed buffer);
/// `RpAnimBlendClumpGetAssociation(p->m_pRwClump (+0x18), name)`.
fn char_anim_association(s: Scr) -> GPtr<u8> {
    collect(s, 1);
    let p = ped(sp(0));
    let mut name = [0u8; 32];
    let np = GPtr::from_raw(name.as_mut_ptr());
    read_text_label(s, np, 0x18);
    rp_anim_blend_clump_get_association(p.byte_add(0x18).cast::<GPtr<u8>>().read(), np)
}

/// 05F1..05F4 SET_CAR_ESCORT_CAR_LEFT/RIGHT/REAR/FRONT (0x49106E, 0x4910CE, 0x49112E, 0x49118F): `a = VEH(SP0)`, `b =
/// VEH(SP1)` (the pool pointer read once); `a->+0x41C = b`, `b->RegisterReference(&a->+0x41C)`; the mission byte
/// +0x3BA unless 0x39 / 0x3A = `mission` (0x1D..0x20).
fn set_car_escort(s: Scr, mission: u8) {
    collect(s, 2);
    let vp = pool(VEHICLE_POOL);
    let a = vehicle_at_ref(vp, spi(0));
    let b = vehicle_at_ref(vp, spi(1));
    let slot = a.byte_add(0x41C);
    slot.cast::<GPtr<u8>>().write(b);
    register_reference(b, slot);
    let m = a.byte_add(0x3BA);
    let cur = m.read();
    if cur != 0x39 && cur != 0x3A {
        m.write(mission);
    }
}

/// 0604 GET_HEADING_FROM_VECTOR_2D (0x49137C): `h = GetATanOfXY(SP0, SP1) * 57.295776f - 90.0f` (x87, ST0 at 53
/// bits, `fmul m32; fsub m32`), stored f32 (`fst`); when `h < 0` (`test ah,5; jp`): `h += 360.0f` in ST0 while `h <
/// 0`, then `fstp`. SP0 = h; `StoreParameters(1)`.
fn get_heading_from_vector_2d(s: Scr) {
    collect(s, 2);
    let a = get_atan_of_xy(sp(0), sp(1));
    let mut h = X87::from_f64_exact(a).fmul_m(f(K_RAD_TO_DEG)).fsub_m(f(K_90));
    let mut out = h.to_f32();
    if h.fcom(X87::constant(K_ZERO)) == Cmp::Less {
        loop {
            h = h.fadd_m(f(K_360));
            if h.fcom(X87::constant(K_ZERO)) != Cmp::Less {
                break;
            }
        }
        out = h.to_f32();
    }
    set_sp(0, out.to_bits());
    store(s, 1);
}

/// 060A LOAD_CHAR_DECISION_MAKER (0x4914A5): `GetPedDMName(SP0, name)` (zeroed buffer); the discarded
/// `GetActualScriptThingIndex(CollectNextParameterWithoutIncreasingPC(), 7)`; `SP0 = GetNewUniqueScriptThingIndex(
/// LoadDecisionMaker(name, 0, m_UsesMissionCleanup), 7)`; `StoreParameters(1)`; mission cleanup `AddEntityToList(SP0,
/// 9)`.
fn load_char_decision_maker(s: Scr) {
    collect(s, 1);
    let mut name = [0u8; 0x100];
    let np = GPtr::from_raw(name.as_mut_ptr());
    get_ped_dm_name(sp(0), np);
    get_actual_script_thing_index(collect_next_parameter_without_increasing_pc(s) as u32, 7);
    let cleanup = u32::from(s.byte_add(crate::vm::USES_MISSION_CLEANUP).read());
    let dm = load_decision_maker(np, 0, cleanup);
    set_sp(0, get_new_unique_script_thing_index(dm, 7));
    store(s, 1);
    if uses_mission_cleanup(s) {
        mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), spi(0), 9);
    }
}

/// 060D SET_TEXT_DROPSHADOW (0x491575): a stack `CRGBA((u8)SP1, (u8)SP2, (u8)SP3, (u8)SP4)`; `IntroTextLines[u16
/// [0xA44B68]]` drop-shadow colour (+0x1D..+0x20) = its bytes, shadow size (+0x21) = (u8)SP0.
fn set_text_dropshadow(s: Scr) {
    collect(s, 5);
    let mut c = [0u8; 4];
    let cp = rgba_ctor(GPtr::from_raw(c.as_mut_ptr()), sp(1) & 0xFF, sp(2) & 0xFF, sp(3) & 0xFF, sp(4) & 0xFF);
    let (r, g, b, a) = (cp.read(), cp.byte_add(1).read(), cp.byte_add(2).read(), cp.byte_add(3).read());
    let n = u32::from(GPtr::<u16>::from_addr(NUM_INTRO_TEXT_LINES).read());
    let line = GPtr::<u8>::from_addr(INTRO_TEXT_SHADOW.wrapping_add(n * 0x44) as usize);
    line.write(r);
    line.byte_add(1).write(g);
    line.byte_add(2).write(b);
    line.byte_add(3).write(a);
    line.byte_add(4).write(sp(0) as u8);
}

/// 060F SET_SENSE_RANGE (0x49162A): SP0 != -1: `PED(SP0)->m_pIntelligence->SetSeeingRange(SP1)`, then
/// `SetHearingRange(SP1)`; SP0 == -1: every entity of the list at 0xB9ACC8 that is a ped (`(+0x36 & 7) == 3`) created
/// by a mission (+0x484 == 2) gets both.
fn set_sense_range(s: Scr) {
    collect(s, 2);
    let r = sp(1);
    if sp(0) != u32::MAX {
        let p = ped(sp(0));
        set_seeing_range(intel(p), r);
        set_hearing_range(intel(p), r);
        return;
    }
    let mut node = GPtr::<GPtr<u8>>::from_addr(SENSE_RANGE_LIST).read();
    while !node.is_null() {
        let e = node.cast::<GPtr<u8>>().read();
        node = node.byte_add(4).cast::<GPtr<u8>>().read();
        if e.byte_add(0x36).read() & 7 == 3 && e.byte_add(0x484).read() == 2 {
            set_seeing_range(intel(e), r);
            set_hearing_range(intel(e), r);
        }
    }
}

/// 0615 OPEN_SEQUENCE_TASK (0x49186F): the discarded `GetActualScriptThingIndex(CollectNextParameterWithoutIncreasingPC(),
/// 4)`; `i = GetAvailableSlot(m_UsesMissionCleanup)`; 0 <= i < 64: `ms_bIsOpened[i] = 1`, `ms_taskSequence[i].Flush()`,
/// `ms_iActiveSequence = i`, `SP0 = GetNewUniqueScriptThingIndex(i, 4)`, mission cleanup `AddEntityToList(SP0, 8)`;
/// else SP0 = -1. `StoreParameters(1)`.
fn open_sequence_task(s: Scr) {
    get_actual_script_thing_index(collect_next_parameter_without_increasing_pc(s) as u32, 4);
    let cleanup = u32::from(s.byte_add(crate::vm::USES_MISSION_CLEANUP).read());
    let i = get_available_sequence_slot(cleanup);
    if (0..0x40).contains(&i) {
        GPtr::<u8>::from_addr(SEQUENCE_OPENED.wrapping_add(i as u32) as usize).write(1);
        sequence_flush(GPtr::from_addr(TASK_SEQUENCES.wrapping_add((i as u32) << 6) as usize));
        GPtr::<u32>::from_addr(ACTIVE_SEQUENCE).write(i as u32);
        let h = get_new_unique_script_thing_index(i as u32, 4);
        set_sp(0, h);
        if uses_mission_cleanup(s) {
            mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), h as i32, 8);
        }
    } else {
        set_sp(0, u32::MAX);
    }
    store(s, 1);
}

/// 061B CLEAR_SEQUENCE_TASK (0x491A4A): `i = GetActualScriptThingIndex(SP0, 4)`; 0 <= i < 64: `ms_bIsOpened[i] = 0`;
/// sequence dword +0x3C == 0: byte +0x38 = 0 and `Flush()`, else byte +0x38 = 1; byte [0xA43F68 + i * 4] = 0. Mission
/// cleanup `RemoveEntityFromList(SP0, 8)` (also when out of range).
fn clear_sequence_task(s: Scr) {
    collect(s, 1);
    let h = sp(0);
    let i = get_actual_script_thing_index(h, 4);
    if (0..0x40).contains(&i) {
        let seq = GPtr::<u8>::from_addr(TASK_SEQUENCES.wrapping_add((i as u32) << 6) as usize);
        let refs = seq.byte_add(0x3C).cast::<u32>().read();
        GPtr::<u8>::from_addr(SEQUENCE_OPENED.wrapping_add(i as u32) as usize).write(0);
        if refs == 0 {
            seq.byte_add(0x38).write(0);
            sequence_flush(seq);
        } else {
            seq.byte_add(0x38).write(1);
        }
        GPtr::<u8>::from_addr(SEQUENCE_ID_BYTES.wrapping_add(i as u32 * 4) as usize).write(0);
    }
    if uses_mission_cleanup(s) {
        mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), h as i32, 8);
    }
}

/// The `(-sin a, cos a, 0)` direction of 061D, normalised (`fld; fmul m32` kept in ST0, `fld st0; fsin; fchs; fstp`,
/// `fcos; fstp`, `CVector::Normalise`).
fn heading_dir(deg: f32) -> [u32; 3] {
    let a = X87::from_f32(deg).fmul_m(f(K_DEG_TO_RAD));
    let mut v = [a.fsin().fchs().to_f32().to_bits(), a.fcos().to_f32().to_bits(), 0];
    vector_normalise(GPtr::from_raw(v.as_mut_ptr()));
    v
}

/// 061D ADD_ATTRACTOR (0x491ABC): `CollectParameters(6)` (x, y, z, heading, queue heading, sequence); the discarded
/// `GetActualScriptThingIndex(CollectNextParameterWithoutIncreasingPC(), 6)`; `i = AddScripted2DEffect(-1.0f)`; `q =
/// GetActualScriptThingIndex(SP5, 4)`; both in [0, 64): effect `ms_effects[i]`: +0 pos, +0x10 and +0x1C the heading
/// direction, +0x28 the queue direction, +0xC = 3, +0x34 = 7, `ms_effectSequenceTaskIDs[i] = q`; `SP0 =
/// GetNewUniqueScriptThingIndex(i, 6)`, mission cleanup `AddEntityToList(SP0, 7)`; otherwise SP0 = -1.
/// `StoreParameters(1)`.
fn add_attractor(s: Scr) {
    collect(s, 6);
    get_actual_script_thing_index(collect_next_parameter_without_increasing_pc(s) as u32, 6);
    let i = add_scripted_2d_effect(0xBF80_0000);
    let q = get_actual_script_thing_index(sp(5), 4);
    if !(0..0x40).contains(&i) || !(0..0x40).contains(&q) {
        set_sp(0, u32::MAX);
        store(s, 1);
        return;
    }
    let pos = [sp(0), sp(1), sp(2)];
    let d1 = heading_dir(spf(3));
    let d2 = heading_dir(spf(4));
    let e = GPtr::<u32>::from_addr(SCRIPTED_EFFECTS.wrapping_add((i as u32) << 6) as usize);
    let w = |off: isize, v: u32| e.byte_add(off).write(v);
    w(0, pos[0]);
    w(4, pos[1]);
    w(8, pos[2]);
    w(0x10, d1[0]);
    w(0x14, d1[1]);
    GPtr::<u32>::from_addr(EFFECT_SEQUENCE_IDS.wrapping_add(i as u32 * 4) as usize).write(q as u32);
    w(0x18, d1[2]);
    w(0x1C, d1[0]);
    w(0x20, d1[1]);
    w(0x28, d2[0]);
    w(0x2C, d2[1]);
    e.cast::<u8>().byte_add(0xC).write(3);
    w(0x24, d1[2]);
    w(0x30, d2[2]);
    e.cast::<u8>().byte_add(0x34).write(7);
    let h = get_new_unique_script_thing_index(i as u32, 6);
    set_sp(0, h);
    if uses_mission_cleanup(s) {
        mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), h as i32, 7);
    }
    store(s, 1);
}

/// 0621 CREATE_CHAR_AT_ATTRACTOR (0x491C7F): `CollectParameters(4)` (ped type, model, attractor, task); `a =
/// GetActualScriptThingIndex(SP2, 6)` outside [0, 64) → SP0 = -1. `GetCorrectPedModelIndexForEmergencyServiceType(SP0,
/// &SP1)`; type 6 → `new CCopPed(SP1)` (0x7C4 bytes), 0x12/0x13 → `new CEmergencyPed(SP0, SP1)`, else `new
/// CCivilianPed(SP0, SP1)` (0x79C bytes; null when `new` fails); `SetCharCreatedBy(2)`; +0x46C bit 31 cleared;
/// `PlacePedAtEffect(effect, null, ped, 0.01f)`; `ClearSpaceForMissionEntity(pos, ped)`; mission cleanup: +0x1C |=
/// 0x40000; `CWorld::Add(ped)`. Task by `SP3 - 0x5BA` (inner tables 0x492C40 / 0x492C54): 0 → `CTaskSimpleStandStill(0,
/// true, false, 8.0f)`; 0x24 → wander standard, 0x26 → wander criminal (`(4, GetRandomNumberInRange(0, 8))`); 0x25 → no
/// task; any other → wander standard; the task is set with `m_TaskMgr.SetTask(task, 4, false)`. Then `new
/// CTaskComplexUseEffect(effect, null)` in a stack `CEventScriptCommand(3, task, false)` added to the ped's event
/// group, `ms_scriptedTasks[GetVacantSlot()].Set(ped, command, added)`, mission ped count + 1, `SP0 = PEDREF(ped)`,
/// mission cleanup `AddEntityToList(SP0, 2)`, the event's destructor. `StoreParameters(1)`.
fn create_char_at_attractor(s: Scr, command: u32) {
    collect(s, 4);
    let a = get_actual_script_thing_index(sp(2), 6);
    if !(0..0x40).contains(&a) {
        set_sp(0, u32::MAX);
        store(s, 1);
        return;
    }
    get_correct_ped_model_index(s, sp(0), param(1));
    let ty = sp(0);
    let p = if ty == 6 {
        let m = ped_new(0x7C4);
        if m.is_null() { GPtr::NULL } else { cop_ped_ctor(m, sp(1)) }
    } else {
        let m = ped_new(0x79C);
        if m.is_null() {
            GPtr::NULL
        } else if ty == 0x12 || ty == 0x13 {
            emergency_ped_ctor(m, sp(0), sp(1))
        } else {
            civilian_ped_ctor(m, sp(0), sp(1))
        }
    };
    set_char_created_by(p, 2);
    let fl = p.byte_add(0x46C).cast::<u32>();
    fl.write(fl.read() & 0x7FFF_FFFF);
    let effect = GPtr::<u8>::from_addr(SCRIPTED_EFFECTS.wrapping_add((a as u32) << 6) as usize);
    place_ped_at_effect(effect, 0, p, 0x3C23_D70A);
    let q = entity_pos(p).cast::<u32>();
    let mut pos = [q.read(), q.byte_add(4).read(), q.byte_add(8).read()];
    clear_space_for_mission_entity(GPtr::from_raw(pos.as_mut_ptr()), p);
    if uses_mission_cleanup(s) {
        let e = p.byte_add(0x1C).cast::<u32>();
        e.write(e.read() | 0x4_0000);
    }
    world_add(p);
    let k = sp(3).wrapping_sub(0x5BA);
    // The byte table 0x492C54 (39 entries): 0 → 0, 1..=0x23 → 4, 0x24 → 1, 0x25 → 2, 0x26 → 3; above → 4.
    let sel = match k {
        0 => 0,
        0x24 => 1,
        0x25 => 2,
        0x26 => 3,
        _ => 4,
    };
    if sel != 2 {
        let t = if sel == 0 {
            new_task(0x20, |t| task_stand_still_ctor(t, 0, 1, 0, 0x4100_0000))
        } else if sel == 3 {
            new_task(0x38, |t| task_wander_criminal_ctor(t, 4, random_int_in_range(0, 8), 1))
        } else {
            new_task(0x38, |t| task_wander_standard_ctor(t, 4, random_int_in_range(0, 8), 1))
        };
        task_manager_set_task(intel(p).byte_add(4), t, 4, 0);
    }
    let t = new_task(0x24, |t| task_use_effect_ctor(t, effect, 0));
    let mut ev = [0u32; 8];
    let e = GPtr::from_raw(ev.as_mut_ptr()).cast::<u8>();
    event_script_command_ctor(e, 3, t, 0);
    let added = event_group_add(intel(p).byte_add(0x68), e, 0);
    let slot = scripted_task_record_get_vacant_slot();
    let rec = GPtr::<u8>::from_addr(SCRIPTED_TASK_RECORDS.wrapping_add(slot.wrapping_mul(20)) as usize);
    scripted_task_record_set(rec, p, command, added);
    let c = GPtr::<u32>::from_addr(MISSION_PED_COUNT);
    c.write(c.read().wrapping_add(1));
    let h = ped_get_ref(pool(PED_POOL), p);
    set_sp(0, h as u32);
    if uses_mission_cleanup(s) {
        mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), h, 2);
    }
    event_script_command_dtor(e);
    store(s, 1);
}

/// The stat cases 0623..062A (0x492058..0x4921CD): `op((u16)SP0, v)` with `v = (float)SP1` (`fild; fstp`, the int
/// forms) or SP1 raw (the float forms), then `DisplayScriptStatUpdateMessage(state, SP0, v)` (state 1, 0 for the
/// decrements).
fn stat_case(s: Scr, op: fn(u32, u32), int: bool, state: u32) {
    collect(s, 2);
    let v = if int { X87::from_i32(spi(1)).to_f32().to_bits() } else { sp(1) };
    op(sp(0) & 0xFFFF, v);
    let v = if int { X87::from_i32(spi(1)).to_f32().to_bits() } else { sp(1) };
    display_script_stat_update_message(state, sp(0), v);
}

/// A stack `CEventScriptCommand(3, new CTaskComplexBeInGroup(g, leader), false)` added to `p`'s event group
/// (`Add(&ev, false)`); the caller runs the destructor.
fn give_be_in_group_event(e: GPtr<u8>, p: GPtr<u8>, g: u32, leader: u32) {
    let t = new_task(0x28, |t| task_be_in_group_ctor(t, g, leader));
    event_script_command_ctor(e, 3, t, 0);
    event_group_add(intel(p).byte_add(0x68), e, 0);
}

/// `CPedGroups::ms_groups[g]` + `off`.
#[inline]
fn group(g: u32, off: u32) -> GPtr<u8> {
    GPtr::from_addr(PED_GROUPS.wrapping_add(g.wrapping_mul(0x2D4)).wrapping_add(off) as usize)
}

/// 0630 SET_GROUP_LEADER (0x4922D8): `g = GetActualScriptThingIndex(SP0, 8)`, `p = PED(SP1)`; 0 <= g < 8: a
/// non-player gets the BE_IN_GROUP(g, leader) script-command event (its destructor right after);
/// `membership.SetLeader(p)`, `CPedGroup::Process()`.
fn set_group_leader(s: Scr) {
    collect(s, 2);
    let g = get_actual_script_thing_index(sp(0), 8);
    let p = ped(sp(1));
    if !(0..8).contains(&g) {
        return;
    }
    let g = g as u32;
    if ped_is_player(p) == 0 {
        let mut ev = [0u32; 8];
        let e = GPtr::from_raw(ev.as_mut_ptr()).cast::<u8>();
        give_be_in_group_event(e, p, g, 1);
        event_script_command_dtor(e);
    }
    membership_set_leader(group(g, 8), p);
    ped_group_process(group(g, 0));
}

/// 0631 SET_GROUP_MEMBER (0x4923BA): `g`, `p = PED(SP1)`; 0 <= g < 8: the BE_IN_GROUP(g, member) event; a group with
/// at least 7 followers and a player leader drops one (`RemoveNFollowers(1)`); `AddFollower(p)`, `Process()`. With a leader
/// whose vehicle (in a vehicle: +0x58C; else the vehicle +0xC of its task 0x2BD) has a free passenger seat (+0x464,
/// `(u8)+0x488` seats): a stack `CEventLeaderEnteredCarAsDriver(veh)` whose `ComputeResponseTaskType(group)` gives the
/// response (i16 +0xE): 0x5E8 → a stack `CTaskComplexEnterCarAsPassenger(veh, 0, true)` set as the member's script
/// command task, and unless +0x474 bit 25 (then set) a stack sequence (`new CTaskSimpleCarDrive(veh, null, true)`,
/// the clone of the member's default task when any) set as its default task (sequence destructor), then the
/// enter-car destructor; 0x5E9 → `new CEventLeaderEnteredCarAsDriver(veh)` (0x18 bytes, `CEvent::operator new`) in a
/// stack `CEventGroupEvent(leader, event)` given to `AddEvent` (its destructor). Then the leader event's destructor;
/// the script-command event's destructor last.
fn set_group_member(s: Scr) {
    collect(s, 2);
    let g = get_actual_script_thing_index(sp(0), 8);
    let p = ped(sp(1));
    if !(0..8).contains(&g) {
        return;
    }
    let g = g as u32;
    let mut ev1 = [0u32; 8];
    let e1 = GPtr::from_raw(ev1.as_mut_ptr()).cast::<u8>();
    give_be_in_group_event(e1, p, g, 0);
    let m = group(g, 8);
    if count_members_excluding_leader(m) >= 7
        && !membership_get_leader(m).is_null()
        && ped_is_player(membership_get_leader(m)) != 0
    {
        remove_n_followers(m, 1);
    }
    membership_add_follower(m, p);
    ped_group_process(group(g, 0));
    let leader = membership_get_leader(m);
    if !leader.is_null() {
        let mut v = GPtr::<u8>::NULL;
        if leader.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 {
            v = leader.byte_add(0x58C).cast::<GPtr<u8>>().read();
        }
        if v.is_null() {
            let t = intel_find_task_by_type(intel(leader), 0x2BD);
            if !t.is_null() {
                v = t.byte_add(0xC).cast::<GPtr<u8>>().read();
            }
        }
        if !v.is_null() {
            let n = i32::from(v.byte_add(0x488).read());
            let free = (0..n).any(|i| v.byte_add(0x464 + 4 * i as isize).cast::<u32>().read() == 0);
            if free {
                leader_entered_car_response(g, p, leader, v);
            }
        }
    }
    event_script_command_dtor(e1);
}

/// The leader-in-car part of 0631 (0x492522..0x4926C9).
fn leader_entered_car_response(g: u32, p: GPtr<u8>, leader: GPtr<u8>, v: GPtr<u8>) {
    let mut ev2 = [0u32; 18];
    let e2 = GPtr::from_raw(ev2.as_mut_ptr()).cast::<u8>();
    event_leader_entered_car_ctor(e2, v);
    compute_response_task_type_group(e2, group(g, 0));
    let rt = i32::from(e2.byte_add(0xE).cast::<i16>().read());
    let gi = group(g, 0x30);
    if rt == 0x5E8 {
        let mut task = [0u32; 32];
        let t = GPtr::from_raw(task.as_mut_ptr()).cast::<u8>();
        task_enter_car_as_passenger_ctor(t, v, 0, 1);
        group_intel_set_script_command_task(gi, p, t);
        let fl = p.byte_add(0x474).cast::<u32>();
        let old = fl.read();
        if old & 0x200_0000 == 0 {
            fl.write(old | 0x200_0000);
            let mut seq = [0u32; 16];
            let sq = GPtr::from_raw(seq.as_mut_ptr()).cast::<u8>();
            sequence_ctor(sq);
            let drive = new_task(0x60, |d| task_car_drive_ctor(d, v, 0, 1));
            sequence_add_task(sq, drive);
            let dt = group_intel_get_task_default(gi, p);
            if !dt.is_null() {
                sequence_add_task(sq, task_clone(dt));
            }
            group_intel_set_default_task(gi, p, sq);
            sequence_dtor(sq);
        }
        task_enter_car_as_passenger_dtor_640380(t);
    } else if rt == 0x5E9 {
        let ne = event_new(0x18);
        let ne = if ne.is_null() { GPtr::NULL } else { event_leader_entered_car_ctor(ne, v) };
        let mut gev = [0u32; 8];
        let ge = GPtr::from_raw(gev.as_mut_ptr()).cast::<u8>();
        event_group_event_ctor(ge, leader, ne);
        group_intel_add_event(gi, ge);
        event_group_event_dtor(ge);
    }
    event_leader_entered_car_dtor_48e220(e2);
}

/// 0632 REMOVE_GROUP (0x4926E9): `g = GetActualScriptThingIndex(SP0, 8)`; 0 <= g < 8: an active group whose leader is
/// a player → `RemoveAllFollowersFromGroup(g)` and return; otherwise `RemoveGroup(g)`. Mission cleanup
/// `RemoveEntityFromList(SP0, 5)`.
fn remove_group_case(s: Scr) {
    collect(s, 1);
    let h = sp(0);
    let g = get_actual_script_thing_index(h, 8);
    if (0..8).contains(&g) {
        let g = g as u32;
        let active = GPtr::<u8>::from_addr(ACTIVE_GROUPS.wrapping_add(g) as usize).read() != 0;
        if active {
            let m = group(g, 8);
            if !membership_get_leader(m).is_null() && ped_is_player(membership_get_leader(m)) != 0 {
                remove_all_followers_from_group(g);
                return;
            }
        }
        remove_group(g);
    }
    if uses_mission_cleanup(s) {
        mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), h as i32, 5);
    }
}

/// 0638 TASK_STAY_IN_SAME_PLACE (0x492975): SP0 != -1: a stack `CTaskSimpleSetStayInSamePlace(SP1 != 0)`
/// `ProcessPed(PED(SP0))`, its destructor; SP0 == -1: `new CTaskSimpleSetStayInSamePlace(SP1 != 0)` (0xC bytes)
/// added to the open sequence `ms_taskSequence[ms_iActiveSequence]`.
fn task_stay_in_same_place(s: Scr) {
    collect(s, 2);
    if sp(0) != u32::MAX {
        let p = ped(sp(0));
        let mut task = [0u32; 4];
        let t = GPtr::from_raw(task.as_mut_ptr()).cast::<u8>();
        task_stay_in_same_place_ctor(t, u32::from(sp(1) != 0));
        task_stay_in_same_place_process_ped(t, p);
        task_stay_in_same_place_dtor_62f5d0(t);
        return;
    }
    let t = new_task(0xC, |t| task_stay_in_same_place_ctor(t, u32::from(sp(1) != 0)));
    let i = GPtr::<u32>::from_addr(ACTIVE_SEQUENCE).read();
    sequence_add_task(GPtr::from_addr(TASK_SEQUENCES.wrapping_add(i << 6) as usize), t);
}

/// `CRunningScript::ProcessCommands1500To1599(int32 command)` (0x490DB0, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x490DB0, conv = thiscall, name = "CRunningScript::ProcessCommands1500To1599", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript")]
pub fn process_commands_1500_to_1599(s: Scr, command: u32) -> i8 {
    match command {
        // Cases the jump table sends to the shared `xor al,al` 0x492AA5 (AL = 0, no operand read; all gr nop): 05DF
        // 05E0 05E1 05E3..05E8 05EA 05EF 05F0 0608 0609 060C 0610 0617 061C 061F 0620 062B 062C 062D.
        0x05DF | 0x05E0 | 0x05E1 | 0x05E3 | 0x05E4 | 0x05E5 | 0x05E6 | 0x05E7 | 0x05E8 | 0x05EA | 0x05EF => {}
        0x05F0 | 0x0608 | 0x0609 | 0x060C | 0x0610 | 0x0617 | 0x061C | 0x061F | 0x0620 | 0x062B | 0x062C => {}
        0x062D => {}
        // 05DC TASK_SMART_FLEE_POINT (0x490DEE): `new CTaskComplexSmartFleePoint(&(SP1, SP2, SP3), true, SP4, SP5)`
        // (0x44 bytes); `GivePedScriptedTask(SP0, task, command)`
        0x05DC => {
            collect(s, 6);
            let mut pos = [sp(1), sp(2), sp(3)];
            let pp = GPtr::from_raw(pos.as_mut_ptr());
            let t = new_task(0x44, |t| task_smart_flee_point_ctor(t, pp, 1, sp(4), sp(5)));
            give_ped_scripted_task(s, sp(0), t, command);
        }
        // 05DD TASK_SMART_FLEE_CHAR (0x490E63): `new CTaskComplexSmartFleeEntity(PED(SP1), true, (float)SP2 (fild),
        // SP3, [0x86F678], [0xC18CF0])` (0x40 bytes); `GivePedScriptedTask(SP0, task, command)`
        0x05DD => {
            collect(s, 4);
            let range = X87::from_i32(spi(2)).to_f32().to_bits();
            let target = ped(sp(1));
            let t = new_task(0x40, |t| {
                task_smart_flee_entity_ctor(t, target, 1, range, sp(3), dword(0x86F678), dword(0xC18CF0))
            });
            give_ped_scripted_task(s, sp(0), t, command);
        }
        // 05DE TASK_WANDER_STANDARD (0x490EF7): `new CTaskComplexWanderStandard(4, GetRandomNumberInRange(0, 8))` (0x38
        // bytes; the random call only when `new` succeeds)
        0x05DE => {
            collect(s, 1);
            let h = sp(0);
            let t = new_task(0x38, |t| task_wander_standard_ctor(t, 4, random_int_in_range(0, 8), 1));
            give_ped_scripted_task(s, h, t, command);
        }
        // 05E2 TASK_KILL_CHAR_ON_FOOT (0x490F46): `new CTaskComplexKillPedOnFoot(PED(SP1), -1, 0, 0, 0, 1)` (0x38 bytes)
        0x05E2 => {
            collect(s, 2);
            let target = ped(sp(1));
            let t = new_task(0x38, |t| task_kill_ped_on_foot_ctor(t, target, u32::MAX, 0, 0, 0, 1));
            give_ped_scripted_task(s, sp(0), t, command);
        }
        // 05E9 START_RECORDING_CAR (0x490FA0) / 0636 TASK_SIDE_STEP_AND_SHOOT (0x49289D): operands read only
        0x05E9 => collect(s, 2),
        0x0636 => collect(s, 9),
        // 05EB START_PLAYBACK_RECORDED_CAR (0x490FB0): `StartPlaybackRecordedCar(VEH(SP0), SP1, false, false)`
        0x05EB => {
            collect(s, 2);
            let v = veh(sp(0));
            start_playback_recorded_car(v, sp(1), 0, 0);
        }
        // 05EC STOP_PLAYBACK_RECORDED_CAR (0x490FE6): when `VEH(SP0)` is non-null
        0x05EC => {
            collect(s, 1);
            let v = veh(sp(0));
            if !v.is_null() {
                stop_playback_recorded_car(v);
            }
        }
        // 05ED PAUSE_PLAYBACK_RECORDED_CAR (0x491018) / 05EE UNPAUSE_PLAYBACK_RECORDED_CAR (0x491043)
        0x05ED => {
            collect(s, 1);
            pause_playback_recorded_car(veh(sp(0)));
        }
        0x05EE => {
            collect(s, 1);
            unpause_playback_recorded_car(veh(sp(0)));
        }
        // 05F1..05F4 SET_CAR_ESCORT_CAR_LEFT/RIGHT/REAR/FRONT
        0x05F1 => set_car_escort(s, 0x1D),
        0x05F2 => set_car_escort(s, 0x1E),
        0x05F3 => set_car_escort(s, 0x1F),
        0x05F4 => set_car_escort(s, 0x20),
        // 05F5 TASK_FOLLOW_PATH_NODES_TO_COORD (0x4911EF): time SP5 (-1 → [0x86FCAC], -2 → -1); `new
        // CTaskComplexFollowNodeRoute(SP4, &(SP1, SP2, SP3), [0x86FCA0], [0x86FCA4], [0x86FCA8], true, time, true)` (0x60
        // bytes); `GivePedScriptedTask(SP0, task, command)`
        0x05F5 => {
            collect(s, 6);
            let mut time = sp(5);
            if time == u32::MAX {
                time = dword(FOLLOW_ROUTE_DEFAULT_TIME);
            } else if time == 0xFFFF_FFFE {
                time = u32::MAX;
            }
            let mut pos = [sp(1), sp(2), sp(3)];
            let pp = GPtr::from_raw(pos.as_mut_ptr());
            let t = new_task(0x60, |t| {
                task_follow_node_route_ctor(t, sp(4), pp, dword(0x86FCA0), dword(0x86FCA4), dword(0x86FCA8), 1, time, 1)
            });
            give_ped_scripted_task(s, sp(0), t, command);
        }
        // 05F6..0601 IS_CHAR_(STOPPED_)IN_ANGLED_AREA_* (0x491289): `CharInAngledAreaCheckCommand(command)`. gr: 05F7
        // 05F9 05FA 05FF 0600 `unimplemented` (-1 without reading the operands).
        0x05F6..=0x0601 => char_in_angled_area_check_command(s, command),
        // 0602 IS_CHAR_IN_TAXI (0x491298): flag = `PED(SP0)` in a vehicle (+0x46C bit 8), vehicle +0x58C non-null with
        // model (u16 +0x22) 0x1A4 or 0x1B6
        0x0602 => {
            collect(s, 1);
            let p = ped(sp(0));
            let mut b = false;
            if p.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 {
                let v = p.byte_add(0x58C).cast::<GPtr<u8>>().read();
                if !v.is_null() {
                    let m = v.byte_add(0x22).cast::<u16>().read();
                    b = m == 0x1A4 || m == 0x1B6;
                }
            }
            set_flag(s, b);
        }
        // 0603 TASK_GO_TO_COORD_ANY_MEANS (0x4912F4): `v = SP5 >= 0 ? VEH(SP5) : null`; `new
        // CTaskComplexGoToPointAnyMeans(SP4, &(SP1, SP2, SP3), v, [0xC18DB4], -1)` (0x34 bytes)
        0x0603 => {
            collect(s, 6);
            let v = if spi(5) >= 0 { veh(sp(5)) } else { GPtr::NULL };
            let mut pos = [sp(1), sp(2), sp(3)];
            let pp = GPtr::from_raw(pos.as_mut_ptr());
            let t = new_task(0x34, |t| task_go_to_point_any_means_ctor(t, sp(4), pp, v, dword(0xC18DB4), u32::MAX));
            give_ped_scripted_task(s, sp(0), t, command);
        }
        // 0604 GET_HEADING_FROM_VECTOR_2D (0x49137C)
        0x0604 => get_heading_from_vector_2d(s),
        // 0605 TASK_PLAY_ANIM (0x4913F5): `PlayAnimScriptCommand(command)`
        0x0605 => play_anim_script_command(s, command),
        // 0606 LOAD_PATH_NODES_IN_AREA (0x491404): x and y ordered; `ThePaths.MakeRequestForNodesToBeLoaded(x1, x2, y1,
        // y2)`
        0x0606 => {
            collect(s, 4);
            let (x1, x2) = order(sp(0), sp(2));
            let (y1, y2) = order(sp(1), sp(3));
            make_request_for_nodes_to_be_loaded(GPtr::from_addr(THE_PATHS), x1, x2, y1, y2);
        }
        // 0607 RELEASE_PATH_NODES (0x491494)
        0x0607 => release_requested_nodes(GPtr::from_addr(THE_PATHS)),
        // 060A LOAD_CHAR_DECISION_MAKER (0x4914A5)
        0x060A => load_char_decision_maker(s),
        // 060B SET_CHAR_DECISION_MAKER (0x49152D): `dm = SP1 == -1 ? -1 : GetActualScriptThingIndex(SP1, 7)`;
        // `PED(SP0)->m_pIntelligence (+0x47C)->SetPedDecisionMakerType(dm)` (no null check)
        0x060B => {
            collect(s, 2);
            let p = ped(sp(0));
            let dm = if spi(1) == -1 { -1 } else { get_actual_script_thing_index(sp(1), 7) };
            set_ped_decision_maker_type(intel(p), dm);
        }
        // 060D SET_TEXT_DROPSHADOW (0x491575)
        0x060D => set_text_dropshadow(s),
        // 060E IS_PLAYBACK_GOING_ON_FOR_CAR (0x4915F5): flag = `IsPlaybackGoingOnForCar(VEH(SP0)) != 0`
        0x060E => {
            collect(s, 1);
            set_flag(s, is_playback_going_on_for_car(veh(sp(0))) != 0);
        }
        // 060F SET_SENSE_RANGE (0x49162A)
        0x060F => set_sense_range(s),
        // 0611 IS_CHAR_PLAYING_ANIM (0x4916CA): flag = the association is non-null
        0x0611 => {
            let a = char_anim_association(s);
            set_flag(s, !a.is_null());
        }
        // 0612 SET_CHAR_ANIM_PLAYING_FLAG (0x491727): the association; `CollectParameters(1)`; non-null: flags byte
        // +0x2E bit 0 = SP0 != 0
        0x0612 => {
            let a = char_anim_association(s);
            collect(s, 1);
            if !a.is_null() {
                let b = a.byte_add(0x2E);
                let v = b.read();
                b.write(if sp(0) != 0 { v | 1 } else { v & 0xFE });
            }
        }
        // 0613 GET_CHAR_ANIM_CURRENT_TIME (0x491794): SP0 = a ? `a->m_fCurrentTime (+0x20) / a->m_pHierarchy (+0x14)->
        // m_fTotalTime (+0x10)` (`fld; fdiv m32; fstp`) : 0
        0x0613 => {
            let a = char_anim_association(s);
            let mut v = 0u32;
            if !a.is_null() {
                let total = a.byte_add(0x14).cast::<GPtr<u8>>().read().byte_add(0x10).cast::<f32>().read();
                v = X87::from_f32(a.byte_add(0x20).cast::<f32>().read()).fdiv_m(total).to_f32().to_bits();
            }
            set_sp(0, v);
            store(s, 1);
        }
        // 0614 SET_CHAR_ANIM_CURRENT_TIME (0x491802): the association; `CollectParameters(1)`; non-null:
        // `a->SetCurrentTime(hier->m_fTotalTime * SP0)` (`fld; fmul m32; fstp`)
        0x0614 => {
            let a = char_anim_association(s);
            collect(s, 1);
            if !a.is_null() {
                let total = a.byte_add(0x14).cast::<GPtr<u8>>().read().byte_add(0x10).cast::<f32>().read();
                set_current_time(a, X87::from_f32(total).fmul_m(spf(0)).to_f32());
            }
        }
        // 0615 OPEN_SEQUENCE_TASK (0x49186F)
        0x0615 => open_sequence_task(s),
        // 0616 CLOSE_SEQUENCE_TASK (0x491908): `i = GetActualScriptThingIndex(SP0, 4)`; 0 <= i < 64: `ms_bIsOpened[i] =
        // 0`, `ms_iActiveSequence = -1`
        0x0616 => {
            collect(s, 1);
            let i = get_actual_script_thing_index(sp(0), 4);
            if (0..0x40).contains(&i) {
                GPtr::<u8>::from_addr(SEQUENCE_OPENED.wrapping_add(i as u32) as usize).write(0);
                GPtr::<u32>::from_addr(ACTIVE_SEQUENCE).write(u32::MAX);
            }
        }
        // 0618 PERFORM_SEQUENCE_TASK (0x49194B): `i = GetActualScriptThingIndex(SP1, 4)`; 0 <= i < 64: `new
        // CTaskComplexUseSequence(i)` (0x1C bytes), `GivePedScriptedTask(SP0, task, command)`
        0x0618 => {
            collect(s, 2);
            let h = sp(0);
            let i = get_actual_script_thing_index(sp(1), 4);
            if (0..0x40).contains(&i) {
                let t = new_task(0x1C, |t| task_use_sequence_ctor(t, i as u32));
                give_ped_scripted_task(s, h, t, command);
            }
        }
        // 0619 SET_CHAR_COLLISION (0x4919AC): `PED(SP0)` dword +0x1C bit 0 = SP1 != 0
        0x0619 => {
            collect(s, 2);
            let fl = ped(sp(0)).byte_add(0x1C).cast::<u32>();
            let v = fl.read();
            fl.write(if sp(1) != 0 { v | 1 } else { v & !1 });
        }
        // 061A GET_CHAR_ANIM_TOTAL_TIME (0x4919ED): SP0 = a ? `hier->m_fTotalTime * 1000.0` (`fmul m64`, `fstp` f32) : 0
        0x061A => {
            let a = char_anim_association(s);
            let mut v = 0u32;
            if !a.is_null() {
                let total = a.byte_add(0x14).cast::<GPtr<u8>>().read().byte_add(0x10).cast::<f32>().read();
                v = X87::from_f32(total).fmul(X87::from_f64_exact(1000.0)).to_f32().to_bits();
            }
            set_sp(0, v);
            store(s, 1);
        }
        // 061B CLEAR_SEQUENCE_TASK (0x491A4A)
        0x061B => clear_sequence_task(s),
        // 061D ADD_ATTRACTOR (0x491ABC)
        0x061D => add_attractor(s),
        // 061E CLEAR_ATTRACTOR (0x491C33): `i = GetActualScriptThingIndex(SP0, 6)`; 0 <= i < 64: `ms_activated[i] = 0`;
        // mission cleanup `RemoveEntityFromList(SP0, 7)`
        0x061E => {
            collect(s, 1);
            let h = sp(0);
            let i = get_actual_script_thing_index(h, 6);
            if (0..0x40).contains(&i) {
                GPtr::<u8>::from_addr(EFFECT_ACTIVATED.wrapping_add(i as u32) as usize).write(0);
            }
            if uses_mission_cleanup(s) {
                mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), h as i32, 7);
            }
        }
        // 0621 CREATE_CHAR_AT_ATTRACTOR (0x491C7F)
        0x0621 => create_char_at_attractor(s, command),
        // 0622 TASK_LEAVE_CAR_IMMEDIATELY (0x491FFF): `new CTaskComplexLeaveCar(VEH(SP1), 0, 0, false, false)` (0x34
        // bytes)
        0x0622 => {
            collect(s, 2);
            let v = veh(sp(1));
            let t = new_task(0x34, |t| task_leave_car_ctor(t, v, 0, 0, 0, 0));
            give_ped_scripted_task(s, sp(0), t, command);
        }
        // 0623 INCREMENT_INT_STAT / 0624 INCREMENT_FLOAT_STAT / 0625 DECREMENT_INT_STAT / 0626 DECREMENT_FLOAT_STAT /
        // 0627 REGISTER_INT_STAT / 0628 REGISTER_FLOAT_STAT / 0629 SET_INT_STAT / 062A SET_FLOAT_STAT
        0x0623 => stat_case(s, increment_stat, true, 1),
        0x0624 => stat_case(s, increment_stat, false, 1),
        0x0625 => stat_case(s, decrement_stat, true, 0),
        0x0626 => stat_case(s, decrement_stat, false, 0),
        0x0627 => stat_case(s, set_new_record_stat, true, 1),
        0x0628 => stat_case(s, set_new_record_stat, false, 1),
        0x0629 => stat_case(s, set_stat_value, true, 1),
        0x062A => stat_case(s, set_stat_value, false, 1),
        // 062E GET_SCRIPT_TASK_STATUS (0x49220A): `SP0 = CPedScriptedTaskRecord::GetStatus(PED(SP0), SP1)`, -1 → 7
        0x062E => {
            collect(s, 2);
            let p = ped(sp(0));
            let r = scripted_task_record_get_status(p, sp(1));
            set_sp(0, if r == -1 { 7 } else { r as u32 });
            store(s, 1);
        }
        // 062F CREATE_GROUP (0x492258): the discarded `GetActualScriptThingIndex(CollectNextParameterWithoutIncreasingPC(),
        // 8)`; `g = CPedGroups::AddGroup()`; `ms_groups[g].m_groupIntelligence.SetDefaultTaskAllocatorType(SP0)`; byte
        // +0x2D0 = 1; `SP0 = GetNewUniqueScriptThingIndex(g, 8)`; `StoreParameters(1)`; mission cleanup
        // `AddEntityToList(SP0, 5)`
        0x062F => {
            collect(s, 1);
            let ty = sp(0);
            get_actual_script_thing_index(collect_next_parameter_without_increasing_pc(s) as u32, 8);
            let g = ped_groups_add_group();
            set_default_task_allocator_type(group(g, 0x30), ty);
            group(g, 0x2D0).write(1);
            set_sp(0, get_new_unique_script_thing_index(g, 8));
            store(s, 1);
            if uses_mission_cleanup(s) {
                mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), spi(0), 5);
            }
        }
        // 0630 SET_GROUP_LEADER (0x4922D8) / 0631 SET_GROUP_MEMBER (0x4923BA) / 0632 REMOVE_GROUP (0x4926E9)
        0x0630 => set_group_leader(s),
        0x0631 => set_group_member(s),
        0x0632 => remove_group_case(s),
        // 0633 TASK_LEAVE_ANY_CAR (0x49277A): `new CTaskComplexLeaveAnyCar(0, true, false)` (0x14 bytes)
        0x0633 => {
            collect(s, 1);
            let h = sp(0);
            let t = new_task(0x14, |t| task_leave_any_car_ctor(t, 0, 1, 0));
            give_ped_scripted_task(s, h, t, command);
        }
        // 0634 TASK_KILL_CHAR_ON_FOOT_WHILE_DUCKING (0x4927D3): `new CTaskComplexKillPedOnFoot(PED(SP1), -1, SP2, SP3,
        // SP4, 1)` (0x38 bytes)
        0x0634 => {
            collect(s, 5);
            let target = ped(sp(1));
            let t = new_task(0x38, |t| task_kill_ped_on_foot_ctor(t, target, u32::MAX, sp(2), sp(3), sp(4), 1));
            give_ped_scripted_task(s, sp(0), t, command);
        }
        // 0635 TASK_AIM_GUN_AT_CHAR (0x49283D): `new CTaskSimpleGunControl(PED(SP1), null, null, 1, 0... )` with the
        // exe's arguments `(target, 0, 0, 0, 1, SP2)` (0x3C bytes)
        0x0635 => {
            collect(s, 3);
            let target = ped(sp(1));
            let t = new_task(0x3C, |t| task_gun_control_ctor(t, target, 0, 0, 0, 1, sp(2)));
            give_ped_scripted_task(s, sp(0), t, command);
        }
        // 0637 TASK_GO_TO_COORD_WHILE_SHOOTING (0x4928AD): `new CTaskComplexGoToPointShooting(SP4, &(SP1, SP2, SP3),
        // PED(SP7), CVector(0, 0, 0), SP5, SP6)` (0x38 bytes)
        0x0637 => {
            collect(s, 8);
            let target = ped(sp(7));
            let mut pos = [sp(1), sp(2), sp(3)];
            let pp = GPtr::from_raw(pos.as_mut_ptr());
            let t = new_task(0x38, |t| task_go_to_point_shooting_ctor(t, sp(4), pp, target, 0, 0, 0, sp(5), sp(6)));
            give_ped_scripted_task(s, sp(0), t, command);
        }
        // 0638 TASK_STAY_IN_SAME_PLACE (0x492975)
        0x0638 => task_stay_in_same_place(s),
        // 0639 TASK_TURN_CHAR_TO_FACE_CHAR (0x492A36): `new CTaskComplexTurnToFaceEntityOrCoord(PED(SP1), [0x86FC7C],
        // [0x86FC80])` (0x28 bytes)
        0x0639 => {
            collect(s, 2);
            let target = ped(sp(1));
            let t = new_task(0x28, |t| task_turn_to_face_entity_ctor(t, target, dword(0x86FC7C), dword(0x86FC80)));
            give_ped_scripted_task(s, sp(0), t, command);
        }
        _ => return -1,
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1500_TO_1599];
