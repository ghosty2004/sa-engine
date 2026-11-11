//! `CRunningScript::ProcessCommands1700To1799` (0x496E00): complete port (PORT-SCRIPT-2..4, ADR-002): every jump-table case
//! is in [`PORTED`]; other opcodes reach the original default (-1).
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands1700To1799.md`.
//!
//! Every ported case returns 0. The "protected store" stub 0x4021A4 (`jmp 0x4021A4` at 0x498C8F, anti-tamper
//! code that computes 0xA43C78 and writes EAX there) is ported as `ScriptParams[0] = eax; StoreParameters(1)`.
//! The 0/1 compare scratch byte the original keeps in a stack slot before `UpdateCompareFlag` is not ported.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x06A4, 0x06A5, 0x06A6, 0x06A7, 0x06A8, 0x06A9, 0x06AA, 0x06AB, 0x06AC, 0x06AD, 0x06AE, 0x06AF, 0x06B0,
    0x06B1, 0x06B2, 0x06B3, 0x06B4, 0x06B5, 0x06B6, 0x06B7, 0x06B9, 0x06BA, 0x06BB, 0x06BC, 0x06BD, 0x06BE,
    0x06BF, 0x06C0, 0x06C1, 0x06C2, 0x06C3, 0x06C4, 0x06C5, 0x06C6, 0x06C7, 0x06C8, 0x06C9, 0x06CA, 0x06CB,
    0x06CE, 0x06CF, 0x06D0, 0x06D1, 0x06D2, 0x06D5, 0x06D6, 0x06D7, 0x06D8, 0x06D9, 0x06DA, 0x06DB, 0x06DC,
    0x06DD, 0x06DE, 0x06DF, 0x06E0, 0x06E1, 0x06E2, 0x06E3, 0x06E4, 0x06E5, 0x06E6, 0x06E7, 0x06E8, 0x06E9,
    0x06EA, 0x06EB, 0x06EC, 0x06ED, 0x06EE, 0x06EF, 0x06F0, 0x06F1, 0x06F2, 0x06F3, 0x06F4, 0x06F5, 0x06F6,
    0x06F7, 0x06F8, 0x06F9, 0x06FA, 0x06FB, 0x06FC, 0x06FD, 0x06FE, 0x06FF, 0x0700, 0x0701, 0x0702, 0x0703,
    0x0704, 0x0705, 0x0706, 0x0707,
];

/// `CTheScripts::UsedObjectArray[0].nModelIndex` + 0x1C (the base the `m < 0` model rule indexes from).
const USED_OBJECT_ARRAY_RULE_BASE: u32 = 0xA44B88;

orig! {
    /// `CTrain::CreateMissionTrain(CVector, bool clockwise, uint32 type, CTrain** first, CTrain** last, int32
    /// node, int32 track, bool missionTrain)` (0x6F7550), 10 dwords; the vector as raw dwords
    fn create_mission_train(
        x: u32, y: u32, z: u32, dir: u32, ty: u32, first: GPtr<u32>, last: GPtr<u32>, node: i32, track: i32, mission: u32
    ) = cdecl 0x6F7550 => unported;
    /// `CPool<CVehicle>::GetRef(CVehicle*)` (0x424160), ECX = the vehicle pool
    fn vehicle_get_ref_(pool: GPtr<u8>, v: u32) -> i32 = thiscall 0x424160 => unported;
    /// `CRunningScript::CollectNextParameterWithoutIncreasingPC()` (0x464250)
    fn collect_next(s: Scr) -> i32 = thiscall 0x464250 => crate::vm::collect_next_parameter_without_increasing_pc;
    /// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0)
    fn get_actual_script_thing_index(r: i32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490), AL
    fn get_is_line_of_sight_clear(a: GPtr<u32>, b: GPtr<u32>, buildings: u32, vehicles: u32, peds: u32, objects: u32, dummies: u32, see_through: u32, ignore_some: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CTheScripts::AddScriptCheckpoint(CVector at, CVector pointTo, float radius, int32 type)` (0x4935A0)
    fn add_script_checkpoint(x: u32, y: u32, z: u32, px: u32, py: u32, pz: u32, radius: u32, ty: u32) -> i32 = cdecl 0x4935A0 => unported;
    /// `CTheScripts::RemoveScriptCheckpoint(int32)` (0x4936C0)
    fn remove_script_checkpoint(h: i32) = cdecl 0x4936C0 => unported;
    /// `CTrain::DisableRandomTrains(bool)` (0x6F5DB0)
    fn disable_random_trains(b: u32) = cdecl 0x6F5DB0 => unported;
    /// `CVehicle::AddVehicleUpgrade(int32 modelId)` (0x6E3290), EAX
    fn add_vehicle_upgrade(v: GPtr<u8>, m: i32) -> i32 = thiscall 0x6E3290 => unported;
    /// `CVehicle::RemoveVehicleUpgrade(int32 modelId)` (0x6DF930)
    fn remove_vehicle_upgrade(v: GPtr<u8>, m: i32) = thiscall 0x6DF930 => unported;
    /// `CStreaming::RequestVehicleUpgrade(int32 modelId, int32 flags)` (0x408C70)
    fn request_vehicle_upgrade(m: i32, flags: i32) = cdecl 0x408C70 => unported;
    /// `CStreaming::HasVehicleUpgradeLoaded(int32 modelId)` (0x407820), AL
    fn has_vehicle_upgrade_loaded(m: i32) -> u8 = cdecl 0x407820 => unported;
    /// `CTrain::RemoveMissionTrains()` (0x6F6A20)
    fn remove_mission_trains() = cdecl 0x6F6A20 => unported;
    /// `CTrain::ReleaseMissionTrains()` (0x6F6B60)
    fn release_mission_trains() = cdecl 0x6F6B60 => unported;
    /// `CTrain::RemoveAllTrains()` (0x6F6AA0)
    fn remove_all_trains() = cdecl 0x6F6AA0 => unported;
    /// `CPlayerPed::RemovePlayerPed(int32)` (0x6094A0)
    fn remove_player_ped(i: i32) = cdecl 0x6094A0 => unported;
    /// `CCamera::StartCooperativeCamMode()` (0x50C260)
    fn start_cooperative_cam_mode(cam: GPtr<u8>) = thiscall 0x50C260 => unported;
    /// `CVehicle::SetRemap(int32)` (0x6D0C00)
    fn set_remap(v: GPtr<u8>, remap: i32) = thiscall 0x6D0C00 => unported;
}

orig! {
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CRunningScript::GivePedScriptedTask(int32 pedHandle, CTask*, int32 opcode)` (0x465C20)
    fn give(s: Scr, ped: u32, task: GPtr<u8>, command: u32) = thiscall 0x465C20 => unported;
    /// `CTaskComplexDiveFromAttachedEntityAndGetUp::CTaskComplexDiveFromAttachedEntityAndGetUp(int32)` (0x492E20)
    fn task_dive_ctor(t: GPtr<u8>, a: u32) -> GPtr<u8> = thiscall 0x492E20 => unported;
    /// `CPed::AttachPedToBike(CEntity*, CVector offset, uint16 pos, float, float, eWeaponType)` (0x5E7E60), 8 dwords
    fn attach_ped_to_bike(p: GPtr<u8>, v: GPtr<u8>, x: u32, y: u32, z: u32, pos: u32, a: u32, b: u32, w: u32) = thiscall 0x5E7E60 => unported;
    /// `CTaskComplexSeekEntityRadiusAngleOffset::CTaskComplexSeekEntityRadiusAngleOffset(CEntity*, int32 time,
    /// int32 period, float, float, float, bool, bool)` (0x493730)
    fn task_seek_radius_angle_ctor(t: GPtr<u8>, e: GPtr<u8>, time: u32, period: u32, a: u32, b: u32, c: u32, d: u32, f: u32) -> GPtr<u8> = thiscall 0x493730 => unported;
    /// `CIKChainManager_c::LookAt(...)` (0x618970) on `g_ikChainMan` 0xC15448
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: u32, time: u32, bone: u32, pos: GPtr<u32>, b: u32, speed: u32, blend: u32, prio: u32, c: u32) = thiscall 0x618970 => unported;
    /// unnamed task ctor 0x634440 (`CTaskSimpleTriggerLookAt`), 10 dwords
    fn task_trigger_look_at_ctor(t: GPtr<u8>, e: u32, time: u32, bone: u32, x: u32, y: u32, z: u32, b: u32, speed: u32, blend: u32, prio: u32) -> GPtr<u8> = thiscall 0x634440 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10)
    fn sequence_add_task(seq: GPtr<u8>, t: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CPed::ReplaceWeaponForScriptedCutscene()` (0x5E6530)
    fn replace_weapon_for_scripted_cutscene(p: GPtr<u8>) = thiscall 0x5E6530 => unported;
    /// `CPed::RemoveWeaponForScriptedCutscene()` (0x5E6550)
    fn remove_weapon_for_scripted_cutscene(p: GPtr<u8>) = thiscall 0x5E6550 => unported;
    /// `CPed::RemoveWeaponWhenEnteringVehicle(int32)` (0x5E6370)
    fn remove_weapon_when_entering_vehicle(p: GPtr<u8>, a: u32) = thiscall 0x5E6370 => unported;
    /// `CVector::Magnitude()` (0x4082C0), ST0 (exact as f64)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CPedGroupIntelligence::SetGroupDecisionMakerType(int32)` (0x5F7340)
    fn set_group_decision_maker_type(gi: GPtr<u8>, dm: i32) = thiscall 0x5F7340 => unported;
    /// `CDecisionMakerTypesFileLoader::GetGrpDMName(int32, char*)` (0x600880)
    fn get_grp_dm_name(i: u32, buf: GPtr<u8>) = cdecl 0x600880 => unported;
    /// `CDecisionMakerTypesFileLoader::LoadDecisionMaker(const char*, int32 type, bool mission)` (0x607D30)
    fn load_decision_maker(name: GPtr<u8>, ty: u32, mission: u32) -> i32 = cdecl 0x607D30 => unported;
    /// `CTheScripts::GetNewUniqueScriptThingIndex(int32, uint8)` (0x483720)
    fn get_new_unique_script_thing_index(i: i32, ty: u32) -> i32 = cdecl 0x483720 => unported;
    /// `CTaskComplexSitDownThenIdleThenStandUp::CTaskComplexSitDownThenIdleThenStandUp(int32, bool, bool)` (0x631460)
    fn task_sit_down_ctor(t: GPtr<u8>, time: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x631460 => unported;
    /// `CTheScripts::AddScriptSearchLight(CVector start, CEntity*, CVector target, float, float)` (0x493000), 9 dwords
    fn add_script_search_light(x: u32, y: u32, z: u32, e: GPtr<u8>, tx: u32, ty: u32, tz: u32, r1: u32, r2: u32) -> i32 = cdecl 0x493000 => unported;
    /// `CTheScripts::RemoveScriptSearchLight(int32)` (0x493160)
    fn remove_script_search_light(h: u32) = cdecl 0x493160 => unported;
    /// `CTheScripts::MoveSearchLightBetweenTwoPoints(int32, CVector, CVector, float)` (0x493360), 8 dwords
    fn move_search_light_between_two_points(h: u32, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, g: u32) = cdecl 0x493360 => unported;
    /// `CTheScripts::MoveSearchLightToPointAndStop(int32, CVector, float)` (0x493480), 5 dwords
    fn move_search_light_to_point_and_stop(h: u32, a: u32, b: u32, c: u32, d: u32) = cdecl 0x493480 => unported;
    /// `CTheScripts::MoveSearchLightToEntity(int32, CEntity*, float)` (0x493420)
    fn move_search_light_to_entity(h: u32, e: GPtr<u8>, k: u32) = cdecl 0x493420 => unported;
    /// `CTheScripts::IsEntityWithinSearchLight(int32, CEntity*)` (0x493900), AL
    fn is_entity_within_search_light(h: u32, e: GPtr<u8>) -> u8 = cdecl 0x493900 => unported;
    /// `CTaskComplexTurnToFaceEntityOrCoord` ctor (0x66B910; the coordinate form by its arguments: `const CVector&,
    /// float, float`)
    fn task_turn_to_face_ctor(t: GPtr<u8>, p: GPtr<u32>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x66B910 => unported;
    /// `CTaskComplexDrivePointRoute::CTaskComplexDrivePointRoute(CVehicle*, const CPointRoute&, float, int32,
    /// int32, float, int32)` (0x6433E0)
    fn task_drive_point_route_ctor(t: GPtr<u8>, v: GPtr<u8>, r: u32, speed: u32, mode: u32, model: u32, radius: u32, style: u32) -> GPtr<u8> = thiscall 0x6433E0 => unported;
    /// `CWeapon::FireOneInstantHitRound(CVector*, CVector*, int32)` (0x73AF00)
    fn fire_one_instant_hit_round(a: GPtr<u32>, b: GPtr<u32>, dmg: u32) = cdecl 0x73AF00 => unported;
    /// `CAutomobile::GetCarRoll()` (0x6A6010), ST0 (read as f64)
    fn get_car_roll(v: GPtr<u8>) -> f64 = thiscall 0x6A6010 => unported;
    /// `CTaskComplexGoToPointAiming::CTaskComplexGoToPointAiming(int32 moveState, const CVector&, CEntity*, CVector,
    /// float, float)` (0x668790), 9 dwords
    fn task_go_to_point_aiming_ctor(t: GPtr<u8>, ms: u32, p: GPtr<u32>, aim: GPtr<u8>, x: u32, y: u32, z: u32, r: u32, d: u32) -> GPtr<u8> = thiscall 0x668790 => unported;
    /// `CFireManager::GetNumFiresInRange(const CVector&, float)` (0x5397F0) on `gFireManager` 0xB71F80
    fn get_num_fires_in_range(m: GPtr<u8>, p: GPtr<u32>, r: u32) -> i32 = thiscall 0x5397F0 => unported;
    /// `CRadar::GetActualBlipArrayIndex(int32)` (0x582870)
    fn blip_index(h: i32) -> i32 = cdecl 0x582870 => unported;
    /// `CRadar::SetEntityBlip(eBlipType, int32, uint32, eBlipDisplay, char*)` (0x5839A0)
    fn set_entity_blip(ty: u32, h: u32, colour: u32, display: u32, name: GPtr<u8>) -> i32 = cdecl 0x5839A0 => unported;
    /// `CRadar::ChangeBlipScale(int32, int32)` (0x583CC0)
    fn change_blip_scale(b: i32, scale: u32) = cdecl 0x583CC0 => unported;
    /// `CVehicleRecording::SkipToEndAndStopPlaybackRecordedCar(CVehicle*)` (0x45A4A0)
    fn skip_to_end_and_stop_playback(v: GPtr<u8>) = cdecl 0x45A4A0 => unported;
    /// `CTaskSimpleCarSetTempAction::CTaskSimpleCarSetTempAction(CVehicle*, int32, int32)` (0x63D6F0)
    fn task_car_temp_action_ctor(t: GPtr<u8>, v: GPtr<u8>, a: u32, time: u32) -> GPtr<u8> = thiscall 0x63D6F0 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    fn get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupMembership::IsLeader(const CPed*)` (0x5F69C0), AL
    fn membership_is_leader(m: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x5F69C0 => unported;
    /// `CPedGroupMembership::IsMember(const CPed*)` (0x5F6A10), AL
    fn membership_is_member(m: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x5F6A10 => unported;
    /// `CPedGroupMembership::RemoveMember(CPed*)` (0x5FB210)
    fn membership_remove_member(m: GPtr<u8>, p: GPtr<u8>) = thiscall 0x5FB210 => unported;
    /// `CPedGroup::Process()` (0x5FC7E0)
    fn ped_group_process(g: GPtr<u8>) = thiscall 0x5FC7E0 => unported;
    /// `CTheScripts::AttachSearchlightToSearchlightObject(int32, CObject*, CObject*, CObject*, CVector)` (0x4934F0)
    fn attach_searchlight(h: u32, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, x: u32, y: u32, z: u32) = cdecl 0x4934F0 => unported;
    /// `CTrain::SetTrainSpeed(CTrain*, float)` (0x6F5E20)
    fn set_train_speed(v: GPtr<u8>, k: u32) = cdecl 0x6F5E20 => unported;
    /// `CTrain::SetTrainCruiseSpeed(CTrain*, float)` (0x6F5E50)
    fn set_train_cruise_speed(v: GPtr<u8>, k: u32) = cdecl 0x6F5E50 => unported;
    /// `CTrain::FindCaboose(CTrain*)` (0x6F5E70)
    fn find_caboose(v: GPtr<u8>) -> u32 = cdecl 0x6F5E70 => unported;
    /// `CTaskComplexCarDriveMission::CTaskComplexCarDriveMission(CVehicle*, CEntity*, int32, int32, float)` (0x63CC30)
    fn task_car_drive_mission_ctor(t: GPtr<u8>, v: GPtr<u8>, target: GPtr<u8>, mission: u32, style: u32, speed: u32) -> GPtr<u8> = thiscall 0x63CC30 => unported;
    /// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorStandard>` ctor (0x46AC10)
    fn task_seek_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, time: u32, period: u32, a: u32, b: u32, c: u32, d: u32, f: u32) -> GPtr<u8> = thiscall 0x46AC10 => unported;
    /// `CTaskSimpleRunAnim::CTaskSimpleRunAnim(AssocGroupId, AnimationId, float, bool)` (0x61A8B0)
    fn task_run_anim_ctor(t: GPtr<u8>, g: u32, a: u32, blend: u32, b: u32) -> GPtr<u8> = thiscall 0x61A8B0 => unported;
    /// `CTaskComplexArrestPed::CTaskComplexArrestPed(CPed*)` (0x68B990)
    fn task_arrest_ped_ctor(t: GPtr<u8>, p: GPtr<u8>) -> GPtr<u8> = thiscall 0x68B990 => unported;
    /// `CStreaming::SetMissionDoesntRequireModel(int32)` (0x409C90)
    fn set_mission_doesnt_require_model(m: i32) = cdecl 0x409C90 => unported;
    /// `CLinkedUpgradeList::FindOtherUpgrade(int16)` (0x4C74D0) on `CVehicleModelInfo::ms_linkedUpgrades` 0xB4E6D8, AX
    fn find_other_upgrade(l: GPtr<u8>, m: i32) -> u16 = thiscall 0x4C74D0 => unported;
    /// `CVehicleModelInfo::GetNumRemaps()` (0x4C86B0)
    fn get_num_remaps(mi: GPtr<u8>) -> i32 = thiscall 0x4C86B0 => unported;
    /// `CFireManager::GetScriptFireCoords(int32)` (0x5397E0), returns a `CVector*`
    fn get_script_fire_coords(m: GPtr<u8>, i: i32) -> GPtr<u32> = thiscall 0x5397E0 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), raw dwords, ST0 (exact as f64)
    fn find_ground_z(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CPathFind::FindNthNodeClosestToCoors(...)` (0x44F8C0), the `CNodeAddress` through the hidden pointer
    fn find_nth_node_closest_to_coors(paths: GPtr<u8>, out: GPtr<u32>, x: u32, y: u32, z: u32, ty: u32, max: u32, a: u32, b: u32, n: u32, c: u32, d: u32, e: u32) -> GPtr<u32> = thiscall 0x44F8C0 => unported;
    /// `CPathFind::FindNodeCoorsForScript(CNodeAddress, bool* found)` (0x4505E0), hidden-pointer vector
    fn find_node_coors_for_script(paths: GPtr<u8>, out: GPtr<u32>, addr: u32, found: GPtr<u8>) -> GPtr<u32> = thiscall 0x4505E0 => unported;
    /// `CPathFind::FindNodeOrientationForCarPlacement(CNodeAddress)` (0x450320), ST0 (read as f64)
    fn find_node_orientation_for_car_placement(paths: GPtr<u8>, addr: u32) -> f64 = thiscall 0x450320 => unported;
    /// `CStuckCarCheck::IsCarInStuckCarArray(int32)` (0x463C70) on `CTheScripts::StuckCars` 0xA90AB0, AL
    fn is_car_in_stuck_car_array(sc: GPtr<u8>, h: u32) -> u8 = thiscall 0x463C70 => unported;
    /// `CVehicleRecording::SetPlaybackSpeed(CVehicle*, float)` (0x459660)
    fn set_playback_speed(v: GPtr<u8>, k: u32) = cdecl 0x459660 => unported;
    /// `CPool<CPed>::GetAt(int32 index)` (0x4082A0), ECX = the ped pool
    fn ped_pool_get_at(pool: GPtr<u8>, i: u32) -> GPtr<u8> = thiscall 0x4082A0 => unported;
    /// `CVector::SquaredMagnitude()` (0x406DA0), ST0 (exact as f64)
    fn vector_squared_magnitude(v: GPtr<u32>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CTagManager::GetPercentageTaggedInArea(CRect*)` (0x49D0B0)
    fn get_percentage_tagged_in_area(r: GPtr<u32>) -> i32 = cdecl 0x49D0B0 => unported;
    /// `CTagManager::SetAlphaInArea(CRect*, uint8)` (0x49CFE0)
    fn set_alpha_in_area(r: GPtr<u32>, a: u32) = cdecl 0x49CFE0 => unported;
    /// `CEntity::GetDistanceFromCentreOfMassToBaseOfModel()` (0x536BE0), ST0 (exact as f64)
    fn get_distance_from_centre_of_mass_to_base_of_model(e: GPtr<u8>) -> f64 = thiscall 0x536BE0 => unported;
    /// `CCarCtrl::JoinCarWithRoadSystemGotoCoors(CVehicle*, const CVector&, bool, bool)` (0x42F870), AL
    fn join_car_with_road_system_goto_coors(v: GPtr<u8>, pos: GPtr<u32>, a: u32, b: u32) -> u8 = cdecl 0x42F870 => unported;
    /// unnamed thiscall on the vehicle (0x41BDD0; see ProcessCommands700To799.md, `SetEngineOn`)
    fn vehicle_41bdd0(v: GPtr<u8>, a: u32) = thiscall 0x41BDD0 => unported;
    /// `CVehicleRecording::StartPlaybackRecordedCar(CVehicle*, int32, bool, bool)` (0x45A980)
    fn start_playback_recorded_car(v: GPtr<u8>, n: u32, ai: u32, b: u32) = cdecl 0x45A980 => unported;
    /// `CVehicleRecording::SkipForwardInRecording(CVehicle*, float)` (0x459D10)
    fn skip_forward_in_recording(v: GPtr<u8>, k: u32) = cdecl 0x459D10 => unported;
}

/// `ms_modelInfoPtrs` 0xA9B0C8.
const MODEL_INFO_PTRS: u32 = 0xA9B0C8;
/// `.rdata` 0x8595EC: 0.017453292f; 0x858B40: 50.0f.
const K_DEG2RAD: u32 = 0x3C8E_FA35;
const K_50: u32 = 0x4248_0000;

/// The dword at `a` (`.rdata` constants pushed as raw dwords, read at run time as the original does).
#[inline]
fn rd(a: usize) -> u32 {
    GPtr::<u32>::from_addr(a).read()
}

/// `t = CTask::operator new(size)`; null -> null, else the constructor's result (EAX).
#[inline]
fn new_task(size: u32, ctor: impl FnOnce(GPtr<u8>) -> GPtr<u8>) -> GPtr<u8> {
    let t = task_new(size);
    if t.is_null() {
        GPtr::NULL
    } else {
        ctor(t)
    }
}

#[inline]
fn ped(h: u32) -> GPtr<u8> {
    ped_at_ref(pool(PED_POOL), h as i32)
}

/// `ms_modelInfoPtrs[m]`.
#[inline]
fn model_info(m: i32) -> GPtr<u8> {
    GPtr::<GPtr<u8>>::from_addr(MODEL_INFO_PTRS.wrapping_add((m as u32).wrapping_mul(4)) as usize).read()
}

/// `CPlaceable::GetPosition()` inlined: `m_matrix` (+0x14) ? `+0x30` of it : `m_placement.pos` (+0x4).
fn entity_pos(e: GPtr<u8>) -> [u32; 3] {
    let m = e.byte_add(0x14).cast::<GPtr<u8>>().read();
    let p = if m.is_null() { e.byte_add(0x4) } else { m.byte_add(0x30) }.cast::<u32>();
    [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()]
}

/// The "-100 ground rule" (`fld z; fcom [-100.0]; test ah,0x41; jp keep`, else ST0 = `FindGroundZForCoord(x, y)`;
/// ST0 then stored as f32).
fn ground_rule(x: u32, y: u32, z: u32) -> u32 {
    let st = X87::from_f32(f(z));
    let st = if st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
        st
    } else {
        X87::from_f64_exact(find_ground_z(x, y))
    };
    st.to_f32().to_bits()
}

/// The search light creation tail (0x4974D5): `SP0 = h`; mission cleanup -> `AddEntityToList(h, 0xB)`;
/// `StoreParameters(1)`.
fn searchlight_tail(s: Scr, h: i32) {
    set_sp(0, h as u32);
    if uses_mission_cleanup(s) {
        mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), h, 0xB);
    }
    store(s, 1);
}

/// 06E6 GET_VEHICLE_MOD_TYPE: the two jump tables (0x4990A0/0x4990BC for flag 0x100, 0x4990D4 otherwise);
/// `None` = the shared `StoreParameters(1)` without writing SP0.
fn vehicle_mod_type(flags: u16) -> Option<u32> {
    let t = u32::from(flags >> 10) & 0x1F;
    if flags & 0x100 != 0 {
        match t {
            1 => Some(11),
            2 => Some(12),
            12 => Some(14),
            13 => Some(15),
            19 => Some(13),
            20..=22 => Some(16),
            _t => None,
        }
    } else {
        match t {
            0 => Some(0),
            1 | 2 => Some(1),
            6 => Some(2),
            8 | 9 => Some(3),
            10 => Some(4),
            11 => Some(5),
            12 => Some(6),
            14 => Some(7),
            15 => Some(8),
            16 => Some(9),
            17 => Some(10),
            _t => None,
        }
    }
}

/// 0702 / 0703: the area rectangle at `[esp+0x74]` from SP0..SP3 (x1, y1, x2, y2): `+0 = minX, +4 = maxY,
/// +8 = maxX, +0xC = minY`. x swapped when x1 > x2, y when y1 > y2 (ordered compares; NaN keeps the order); the
/// low y comes from the x87 register (`fstp`: a NaN comes back quiet), the others are raw dwords.
fn tag_rect() -> [u32; 4] {
    let (x1, y1, x2, y2) = (sp(0), sp(1), sp(2), sp(3));
    let gt = |a: u32, b: u32| X87::from_f32(f(a)).fcom(X87::from_f32(f(b))) == sa_math::x87::Cmp::Greater;
    let (min_x, max_x) = if gt(x1, x2) { (x2, x1) } else { (x1, x2) };
    let (min_y, max_y) = if gt(y1, y2) { (y2, y1) } else { (y1, y2) };
    [min_x, max_y, max_x, fld_fst(f(min_y)).to_bits()]
}

/// The model id rule of this switch: `m < 0` (a `UsedObjectArray` index) reads
/// `*(int32*)(0xA44B88 - m * 0x1C)` (`imul m,0x1C; sub`).
#[inline]
fn model_id(m: i32) -> i32 {
    if m < 0 {
        GPtr::<i32>::from_addr(
            USED_OBJECT_ARRAY_RULE_BASE.wrapping_sub((m as u32).wrapping_mul(0x1C)) as usize,
        )
        .read()
    } else {
        m
    }
}

#[inline]
fn veh(h: u32) -> GPtr<u8> {
    vehicle_at_ref(pool(VEHICLE_POOL), h as i32)
}

/// `CRunningScript::ProcessCommands1700To1799(int32 command)` (0x496E00, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x496E00, conv = thiscall, name = "CRunningScript::ProcessCommands1700To1799", expect = "6A FF 68 7D 8D 83 00", this = "CRunningScript")]
pub fn process_commands_1700_to_1799(s: Scr, command: u32) -> i8 {
    match command {
        // 06BD IS_LINE_OF_SIGHT_CLEAR (0x497852)
        0x06BD => is_line_of_sight_clear(s),
        // 06D5 CREATE_CHECKPOINT (0x497E54)
        0x06D5 => create_checkpoint(s),
        // 06D6 DELETE_CHECKPOINT (0x497EF9)
        0x06D6 => delete_checkpoint(s),
        // 06D7 SWITCH_RANDOM_TRAINS (0x497F51): SP0 != 0 pushes 0, else 1
        0x06D7 => {
            collect(s, 1);
            disable_random_trains(u32::from(sp(0) == 0));
        }
        // 06E7 ADD_VEHICLE_MOD (0x4985A6): VEH(SP0) looked up before the model rule; result -> protected store
        0x06E7 => {
            collect(s, 2);
            let v = veh(sp(0));
            let r = add_vehicle_upgrade(v, model_id(spi(1)));
            set_sp(0, r as u32);
            store(s, 1);
        }
        // 06E8 REMOVE_VEHICLE_MOD (0x4985E4)
        0x06E8 => {
            collect(s, 2);
            let v = veh(sp(0));
            remove_vehicle_upgrade(v, model_id(spi(1)));
        }
        // 06E9 REQUEST_VEHICLE_MOD (0x498624)
        0x06E9 => {
            collect(s, 1);
            request_vehicle_upgrade(model_id(spi(0)), 0xC);
        }
        // 06EA HAS_VEHICLE_MOD_LOADED (0x498654): shared compare tail 0x497580/0x497585
        0x06EA => {
            collect(s, 1);
            let r = has_vehicle_upgrade_loaded(model_id(spi(0)));
            set_flag(s, r != 0);
        }
        // 06ED GIVE_VEHICLE_PAINTJOB (0x49870C): no null check
        0x06ED => {
            collect(s, 2);
            let v = veh(sp(0));
            set_remap(v, spi(1));
        }
        // 06D8 CREATE_MISSION_TRAIN (0x497F85): `CreateMissionTrain((P1, P2, P3), dir, P0, &first, nullptr, -1,
        // -1, true)`; P0 = VehiclePool.GetRef(first) (stored through the 0x4021A4 stub), StoreParameters(1).
        // `dir` is the exact dword the original pushes: `setne dl` (P4 != 0) over EDX = P3 (the callee reads
        // bit 0 of the low byte). `first` starts at 0: the original's slot is uninitialised, and
        // CreateMissionTrain writes it on its first carriage whenever the type's config (0x8D44F8 + 0x40 *
        // type) starts with a non-zero model, which holds for all 16 types 0..15 in the exe image (the
        // table is only read); an out-of-range type would leave the original's slot stale.
        0x06D8 => {
            collect(s, 5);
            let dir = (sp(3) & 0xFFFF_FF00) | u32::from(sp(4) != 0);
            let mut first = [0u32; 1];
            create_mission_train(
                sp(1),
                sp(2),
                sp(3),
                dir,
                sp(0),
                GPtr::from_raw(first.as_mut_ptr()),
                GPtr::NULL,
                -1,
                -1,
                1,
            );
            let h = vehicle_get_ref_(pool(VEHICLE_POOL), first[0]);
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 06A4, 06A6, 06C6, 06CB, 06CE, 06F4, 06F6, 06F7, 06F9, 06FB, 06FE, 0700 (0x498EED; gr: nop or
        // unregistered): straight to the epilogue, operands not read, return 0.
        0x06A4 | 0x06A6 | 0x06C6 | 0x06CB | 0x06CE | 0x06F4 => {}
        0x06F6 | 0x06F7 | 0x06F9 | 0x06FB | 0x06FE | 0x0700 => {}
        // 06AA IS_RECORDING_GOING_ON_FOR_CAR (0x4971B8, gr: nop): CollectParameters(1); UpdateCompareFlag(0).
        0x06AA => {
            collect(s, 1);
            set_flag(s, false);
        }
        // 06CF DISPLAY_TIMER_BARS (0x497DB8, gr: nop): CollectParameters(1), nothing else.
        0x06CF => collect(s, 1),
        // 06B9 HAS_CUTSCENE_LOADED (0x4976A2, gr: unregistered): no operands; flag =
        // `CCutsceneMgr::ms_cutsceneLoadStatus` (0xB5F84C) == 2.
        0x06B9 => {
            let b = GPtr::<u32>::from_addr(0xB5F84C).read() == 2;
            set_flag(s, b);
        }
        // 06C8 SET_LA_RIOTS (0x497CDB) / 06D0 SWITCH_EMERGENCY_SERVICES (0x497DC8), gr: unregistered:
        // byte 0xB72958 / 0x8A5B28 = P0 != 0.
        0x06C8 | 0x06D0 => {
            collect(s, 1);
            let a = if command == 0x06C8 { 0xB72958 } else { 0x8A5B28 };
            GPtr::<u8>::from_addr(a).write(u8::from(sp(0) != 0));
        }
        // 06D9 DELETE_MISSION_TRAINS (0x498001), 06DA MARK_MISSION_TRAINS_AS_NO_LONGER_NEEDED (0x49800D),
        // 06DB DELETE_ALL_TRAINS (0x498019), gr: unregistered: CTrain::RemoveMissionTrains /
        // ReleaseMissionTrains / RemoveAllTrains.
        0x06D9 => remove_mission_trains(),
        0x06DA => release_mission_trains(),
        0x06DB => remove_all_trains(),
        // 06DF DELETE_PLAYER (0x4980C9): CPlayerPed::RemovePlayerPed(P0).
        0x06DF => {
            collect(s, 1);
            remove_player_ped(spi(0));
        }
        // 06E0 SET_TWO_PLAYER_CAMERA_MODE (0x4980E7): CollectParameters(1) (unused);
        // TheCamera.StartCooperativeCamMode().
        0x06E0 => {
            collect(s, 1);
            start_cooperative_cam_mode(GPtr::from_addr(THE_CAMERA));
        }
        // 06F1 LIMIT_TWO_PLAYER_DISTANCE (0x49883D): byte 0x8A5E4A = 1, dword 0x8A5E4C = P0 (raw).
        0x06F1 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0x8A5E4A).write(1);
            GPtr::<u32>::from_addr(0x8A5E4C).write(sp(0));
        }
        // 06F2 RELEASE_TWO_PLAYER_DISTANCE (0x498860): no operands; byte 0x8A5E4A = 0.
        0x06F2 => GPtr::<u8>::from_addr(0x8A5E4A).write(0),
        // 06F3 SET_PLAYER_PLAYER_TARGETTING (0x49886E): byte 0x96A8B2 = P0 == 0.
        0x06F3 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0x96A8B2).write(u8::from(sp(0) == 0));
        }
        // 06FA SET_PLAYERS_CAN_BE_IN_SEPARATE_CARS (0x498A2C): byte 0x96A8B3 = P0 != 0.
        0x06FA => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0x96A8B3).write(u8::from(sp(0) != 0));
        }
        // 0701 SKIP_CUTSCENE_END (0x498BCF, gr: unregistered): no operands; `m_SceneSkipIP` (+0xD8) = 0.
        // 0707 SKIP_CUTSCENE_START_INTERNAL (0x498ED8, gr: unregistered): it = P0.
        0x0701 => s.byte_add(crate::vm::SCENE_SKIP_IP).cast::<u32>().write(0),
        0x0707 => {
            collect(s, 1);
            s.byte_add(crate::vm::SCENE_SKIP_IP).cast::<u32>().write(sp(0));
        }
        // 06A5 TASK_DIVE_FROM_ATTACHMENT_AND_GET_UP (0x496E61): `new(0x24) CTaskComplexDiveFromAttachedEntityAndGetUp(SP1)`;
        // give (GIVE tail 0x496E9F).
        0x06A5 => {
            collect(s, 2);
            let (p, a) = (sp(0), sp(1));
            let t = new_task(0x24, |t| task_dive_ctor(t, a));
            give(s, p, t, command);
        }
        // 06A7 ATTACH_CHAR_TO_BIKE (0x496EBB): `ped(SP0)->AttachPedToBike(veh(SP1), (SP2,SP3,SP4), (u16)SP5,
        // SP6 * 0.017453292, SP7 * 0.017453292, SP8)` (the radians f32; the vehicle looked up first; no null checks).
        0x06A7 => {
            collect(s, 9);
            let a2 = X87::from_f32(spf(7)).fmul_m(f(K_DEG2RAD)).to_f32().to_bits();
            let a1 = X87::from_f32(spf(6)).fmul_m(f(K_DEG2RAD)).to_f32().to_bits();
            let v = veh(sp(1));
            let p = ped(sp(0));
            attach_ped_to_bike(p, v, sp(2), sp(3), sp(4), sp(5) & 0xFFFF, a1, a2, sp(8));
        }
        // 06A8 TASK_GOTO_CHAR_OFFSET (0x496F5F): `target = ped(SP1)`; `a = SP4 * 0.017453292` (f32); `t = new(0x54)
        // CTaskComplexSeekEntityRadiusAngleOffset(target, SP2 < 0 ? 50000 : SP2, 1000, [0x85A40C], [0x85A410],
        // [0x85A414], true, true)`; then, t not null-checked: `t+0x44 = SP3`, `t+0x48 = a`, `t+0x4C = [0x8D237C]`,
        // `[0x8D237C] = 6` (the stack seek-pos calculator built at `[esp+0x54]` is dead); give.
        0x06A8 => {
            collect(s, 5);
            let a = X87::from_f32(spf(4)).fmul_m(f(K_DEG2RAD)).to_f32().to_bits();
            let (p, time, radius) = (sp(0), sp(2), sp(3));
            let target = ped(sp(1));
            let time = if (time as i32) < 0 { 0xC350 } else { time };
            let t = new_task(0x54, |t| {
                task_seek_radius_angle_ctor(t, target, time, 0x3E8, rd(0x85A40C), rd(0x85A410), rd(0x85A414), 1, 1)
            });
            t.byte_add(0x48).cast::<u32>().write(a);
            t.byte_add(0x44).cast::<u32>().write(radius);
            let g = GPtr::<u32>::from_addr(0x8D237C);
            t.byte_add(0x4C).cast::<u32>().write(g.read());
            g.write(6);
            give(s, p, t, command);
        }
        // 06A9 TASK_LOOK_AT_COORD (0x49707D): `time = SP4 == -1 ? 20000 : SP4 == -2 ? 999999 : SP4`; SP0 != -1:
        // `g_ikChainMan.LookAt("COMMAND_TASK_LOOK_AT_COO.." 0x85A3F0, ped(SP0), null, time, -1, &(SP1,SP2,SP3), false,
        // 0.25, 500, 6, true)`; SP0 == -1: `new(0x30) 0x634440(null, time, -1, (SP1,SP2,SP3), true, 0.25, 1000, 3)`,
        // `ms_taskSequence[ms_iActiveSequence].AddTask(t)` (index read directly from 0x8D2E98 here).
        0x06A9 => {
            collect(s, 5);
            let time = match spi(4) {
                -1 => 0x4E20,
                -2 => 0xF423F,
                v => v as u32,
            };
            let mut pos = [sp(1), sp(2), sp(3)];
            if spi(0) != -1 {
                let p = ped(sp(0));
                ik_look_at(
                    GPtr::from_addr(0xC15448),
                    0x85A3F0,
                    p,
                    0,
                    time,
                    u32::MAX,
                    GPtr::from_raw(pos.as_mut_ptr()),
                    0,
                    0x3E80_0000,
                    0x1F4,
                    6,
                    1,
                );
            } else {
                let t = new_task(0x30, |t| {
                    task_trigger_look_at_ctor(t, 0, time, u32::MAX, pos[0], pos[1], pos[2], 1, 0x3E80_0000, 0x3E8, 3)
                });
                let i = rd(0x8D2E98);
                sequence_add_task(GPtr::from_addr(0xC178F0u32.wrapping_add(i << 6) as usize), t);
            }
        }
        // 06AB HIDE_CHAR_WEAPON_FOR_SCRIPTED_CUTSCENE (0x4971D1): SP1 != 0: `ped->ReplaceWeaponForScriptedCutscene()`
        // (0x5E6530); else `ped->RemoveWeaponForScriptedCutscene()` (0x5E6550) and, when the ped is not in a vehicle
        // (`+0x46C` bit 8 clear), `ped->RemoveWeaponWhenEnteringVehicle(0)` (0x5E6370). No null check.
        0x06AB => {
            collect(s, 2);
            let p = ped(sp(0));
            if sp(1) != 0 {
                replace_weapon_for_scripted_cutscene(p);
            } else {
                remove_weapon_for_scripted_cutscene(p);
                if p.byte_add(0x46C).cast::<u32>().read() & 0x100 == 0 {
                    remove_weapon_when_entering_vehicle(p, 0);
                }
            }
        }
        // 06AC GET_CHAR_SPEED (0x497228): `v = ped in a vehicle (+0x46C bit 8) and +0x58C != null ? &vehicle->
        // m_vecMoveSpeed : &ped->m_vecMoveSpeed` (+0x44); `SP0 = v.Magnitude() * 50.0` (f32); StoreParameters(1).
        0x06AC => {
            collect(s, 1);
            let p = ped(sp(0));
            let car = p.byte_add(0x58C).cast::<GPtr<u8>>();
            let e = if p.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 && !car.read().is_null() {
                car.read()
            } else {
                p
            };
            let m = X87::from_f64_exact(vector_magnitude(e.byte_add(0x44)));
            set_sp(0, m.fmul_m(f(K_50)).to_f32().to_bits());
            store(s, 1);
        }
        // 06AD SET_GROUP_DECISION_MAKER (0x497281): `g = GetActualScriptThingIndex(SP0, 8)`; 0 <= g < 8: `dm = SP1 ==
        // -1 ? -1 : GetActualScriptThingIndex(SP1, 7)`; `ms_groups[g].m_groupIntelligence.SetGroupDecisionMakerType(dm)`.
        0x06AD => {
            collect(s, 2);
            let g = get_actual_script_thing_index(spi(0), 8);
            if (0..8).contains(&g) {
                let dm = if spi(1) == -1 { -1 } else { get_actual_script_thing_index(spi(1), 7) };
                set_group_decision_maker_type(GPtr::from_addr((0xC09950 + g * 0x2D4) as usize), dm);
            }
        }
        // 06AE LOAD_GROUP_DECISION_MAKER (0x4972E2): `GetGrpDMName(SP0, buf)`; the discarded
        // `GetActualScriptThingIndex(CollectNextParameterWithoutIncreasingPC(), 7)`; `i = LoadDecisionMaker(buf, 1,
        // (u8)m_UsesMissionCleanup)`; `SP0 = GetNewUniqueScriptThingIndex(i, 7)`; StoreParameters(1); mission cleanup ->
        // `AddEntityToList(SP0, 9)`. The name buffer starts zeroed (GetGrpDMName strcpy's a NUL-terminated name).
        0x06AE => {
            collect(s, 1);
            let mut buf = [0u8; 256];
            let b = GPtr::from_raw(buf.as_mut_ptr());
            get_grp_dm_name(sp(0), b);
            get_actual_script_thing_index(collect_next(s), 7);
            let mission = s.byte_add(crate::vm::USES_MISSION_CLEANUP).read();
            let i = load_decision_maker(b, 1, u32::from(mission));
            set_sp(0, get_new_unique_script_thing_index(i, 7) as u32);
            store(s, 1);
            if uses_mission_cleanup(s) {
                mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), spi(0), 9);
            }
        }
        // 06AF DISABLE_PLAYER_SPRINT (0x49736A): `Players[SP0].m_pPed->m_pPlayerData (+0x480)` byte +0x84 = SP1 != 0.
        0x06AF => {
            collect(s, 2);
            let p = player_info(sp(0)).cast::<GPtr<u8>>().read();
            let d = p.byte_add(0x480).cast::<GPtr<u8>>().read();
            d.byte_add(0x84).write(u8::from(sp(1) != 0));
        }
        // 06B0 TASK_SIT_DOWN (0x4973B9): `new(0x14) CTaskComplexSitDownThenIdleThenStandUp(SP1 == -1 ? [0x86DB28] :
        // SP1, false, false)`; give.
        0x06B0 => {
            collect(s, 2);
            let (p, time) = (sp(0), sp(1));
            let t = new_task(0x14, |t| {
                let time = if time as i32 == -1 { rd(0x86DB28) } else { time };
                task_sit_down_ctor(t, time, 0, 0)
            });
            give(s, p, t, command);
        }
        // 06B1 CREATE_SEARCHLIGHT (0x497444): the discarded `GetActualScriptThingIndex(CollectNext..(), 2)`; `h =
        // AddScriptSearchLight((SP0,SP1,SP2), null, (SP3,SP4,SP5), SP6, SP7)`; tail 0x4974D5.
        0x06B1 => {
            collect(s, 8);
            get_actual_script_thing_index(collect_next(s), 2);
            let h = add_script_search_light(sp(0), sp(1), sp(2), GPtr::NULL, sp(3), sp(4), sp(5), sp(6), sp(7));
            searchlight_tail(s, h);
        }
        // 06C1 CREATE_SEARCHLIGHT_ON_VEHICLE (0x4979B4): as 06B1 with `veh(SP0)` and the light at (SP1..SP3).
        0x06C1 => {
            collect(s, 9);
            get_actual_script_thing_index(collect_next(s), 2);
            let v = veh(sp(0));
            let h = add_script_search_light(sp(1), sp(2), sp(3), v, sp(4), sp(5), sp(6), sp(7), sp(8));
            searchlight_tail(s, h);
        }
        // 06B2 DELETE_SEARCHLIGHT (0x497505): `GetActualScriptThingIndex(SP0, 2) >= 0` -> `RemoveScriptSearchLight(SP0)`,
        // mission cleanup -> `RemoveEntityFromList(SP0, 0xB)`.
        0x06B2 => {
            collect(s, 1);
            if get_actual_script_thing_index(spi(0), 2) >= 0 {
                remove_script_search_light(sp(0));
                if uses_mission_cleanup(s) {
                    mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), spi(0), 0xB);
                }
            }
        }
        // 06B3 DOES_SEARCHLIGHT_EXIST (0x49755D): flag = `GetActualScriptThingIndex(SP0, 2) >= 0`.
        0x06B3 => {
            collect(s, 1);
            let b = get_actual_script_thing_index(spi(0), 2) >= 0;
            set_flag(s, b);
        }
        // 06B4 MOVE_SEARCHLIGHT_BETWEEN_COORDS (0x497598): `MoveSearchLightBetweenTwoPoints(SP0..SP7)`.
        0x06B4 => {
            collect(s, 8);
            move_search_light_between_two_points(sp(0), sp(1), sp(2), sp(3), sp(4), sp(5), sp(6), sp(7));
        }
        // 06B5 POINT_SEARCHLIGHT_AT_COORD (0x4975E5): `MoveSearchLightToPointAndStop(SP0..SP4)`.
        0x06B5 => {
            collect(s, 5);
            move_search_light_to_point_and_stop(sp(0), sp(1), sp(2), sp(3), sp(4));
        }
        // 06B6 POINT_SEARCHLIGHT_AT_CHAR (0x49761E) / 06BF ..._AT_VEHICLE (0x497940): `MoveSearchLightToEntity(SP0,
        // ped(SP1) / veh(SP1), SP2)`.
        0x06B6 | 0x06BF => {
            collect(s, 3);
            let k = sp(2);
            let e = if command == 0x06B6 { ped(sp(1)) } else { veh(sp(1)) };
            move_search_light_to_entity(sp(0), e, k);
        }
        // 06B7 IS_CHAR_IN_SEARCHLIGHT (0x497656) / 06C0 IS_VEHICLE_IN_SEARCHLIGHT (0x497978): flag =
        // `IsEntityWithinSearchLight(SP0, ped(SP1) / veh(SP1)) != 0`.
        0x06B7 | 0x06C0 => {
            collect(s, 2);
            let e = if command == 0x06B7 { ped(sp(1)) } else { veh(sp(1)) };
            let b = is_entity_within_search_light(sp(0), e) != 0;
            set_flag(s, b);
        }
        // 06BA TASK_TURN_CHAR_TO_FACE_COORD (0x4976C8): `new(0x28) CTaskComplexTurnToFaceEntityOrCoord(&(SP1,SP2,SP3),
        // [0x86FC7C], [0x86FC80])`; `GivePedScriptedTask(SP0 (read after the ctor), t, command)`.
        0x06BA => {
            collect(s, 4);
            let mut pos = [sp(1), sp(2), sp(3)];
            let t = new_task(0x28, |t| {
                task_turn_to_face_ctor(t, GPtr::from_raw(pos.as_mut_ptr()), rd(0x86FC7C), rd(0x86FC80))
            });
            give(s, sp(0), t, command);
        }
        // 06BB TASK_DRIVE_POINT_ROUTE (0x49776F): `v = SP1 < 0 ? null : veh(SP1)`; `new(0x2C)
        // CTaskComplexDrivePointRoute(v, route 0xC18D50, SP2, 2, -1, -1.0, 0)`; give.
        0x06BB => {
            collect(s, 3);
            let p = sp(0);
            let v = if spi(1) < 0 { GPtr::NULL } else { veh(sp(1)) };
            let speed = sp(2);
            let t = new_task(0x2C, |t| {
                task_drive_point_route_ctor(t, v, 0xC18D50, speed, 2, u32::MAX, 0xBF80_0000, 0)
            });
            give(s, p, t, command);
        }
        // 06BC FIRE_SINGLE_BULLET (0x4977E3): `CWeapon::FireOneInstantHitRound(&(SP0,SP1,SP2), &(SP3,SP4,SP5), SP6)`.
        0x06BC => {
            collect(s, 7);
            let mut a = [sp(0), sp(1), sp(2)];
            let mut b = [sp(3), sp(4), sp(5)];
            fire_one_instant_hit_round(GPtr::from_raw(a.as_mut_ptr()), GPtr::from_raw(b.as_mut_ptr()), sp(6));
        }
        // 06BE GET_CAR_ROLL (0x497908): `SP0 = CAutomobile::GetCarRoll(veh(SP0))` (ST0 stored f32; FPATAN class, ST0
        // read at 53 bits as for 0174/0176); StoreParameters(1).
        0x06BE => {
            collect(s, 1);
            let v = veh(sp(0));
            set_sp(0, X87::from_f64_exact(get_car_roll(v)).to_f32().to_bits());
            store(s, 1);
        }
        // 06C2 TASK_GO_TO_COORD_WHILE_AIMING (0x497A65): `aim = SP7 < 0 ? null : ped(SP7)`; `new(0x38)
        // CTaskComplexGoToPointAiming(SP4, &(SP1,SP2,SP3), aim, (SP8,SP9,SP10), SP5, SP6)`; give.
        0x06C2 => {
            collect(s, 11);
            let p = sp(0);
            let aim = if spi(7) < 0 { GPtr::NULL } else { ped(sp(7)) };
            let mut target = [sp(1), sp(2), sp(3)];
            let (ms, r, d, x, y, z) = (sp(4), sp(5), sp(6), sp(8), sp(9), sp(10));
            let t = new_task(0x38, |t| {
                task_go_to_point_aiming_ctor(t, ms, GPtr::from_raw(target.as_mut_ptr()), aim, x, y, z, r, d)
            });
            give(s, p, t, command);
        }
        // 06C3 GET_NUMBER_OF_FIRES_IN_RANGE (0x497B75): `SP0 = gFireManager.GetNumFiresInRange(&(SP0,SP1,SP2), SP3)`;
        // protected store.
        0x06C3 => {
            collect(s, 4);
            let mut pos = [sp(0), sp(1), sp(2)];
            let n = get_num_fires_in_range(GPtr::from_addr(0xB71F80), GPtr::from_raw(pos.as_mut_ptr()), sp(3));
            set_sp(0, n as u32);
            store(s, 1);
        }
        // 06C4 ADD_BLIP_FOR_SEARCHLIGHT (0x497BC7): the discarded `GetActualScriptThingIndex(SP0, 2)` and
        // `GetActualBlipArrayIndex(CollectNext..())`; `b = SetEntityBlip(6, SP0, 0, 3, this->m_szName)`;
        // `ChangeBlipScale(b, 3)`; `SP0 = b`; StoreParameters(1).
        0x06C4 => {
            collect(s, 1);
            get_actual_script_thing_index(spi(0), 2);
            blip_index(collect_next(s));
            let b = set_entity_blip(6, sp(0), 0, 3, s.byte_add(8));
            change_blip_scale(b, 3);
            set_sp(0, b as u32);
            store(s, 1);
        }
        // 06C5 SKIP_TO_END_AND_STOP_PLAYBACK_RECORDED_CAR (0x497C26).
        0x06C5 => {
            collect(s, 1);
            skip_to_end_and_stop_playback(veh(sp(0)));
        }
        // 06C7 TASK_CAR_TEMP_ACTION (0x497C50): `v = SP1 == -1 ? null : veh(SP1)`; `new(0x68)
        // CTaskSimpleCarSetTempAction(v, SP2, SP3)`; give.
        0x06C7 => {
            collect(s, 4);
            let p = sp(0);
            let v = if spi(1) == -1 { GPtr::NULL } else { veh(sp(1)) };
            let (a, time) = (sp(2), sp(3));
            let t = new_task(0x68, |t| task_car_temp_action_ctor(t, v, a, time));
            give(s, p, t, command);
        }
        // 06C9 REMOVE_CHAR_FROM_GROUP (0x497CFB): `g = CPedGroups::GetPedsGroup(ped)`; g and not the leader:
        // `g->m_groupMembership.RemoveMember(ped)`, `g->Process()`.
        0x06C9 => {
            collect(s, 1);
            let p = ped(sp(0));
            let g = get_peds_group(p);
            if !g.is_null() {
                let m = g.byte_add(8);
                if membership_is_leader(m, p) == 0 {
                    membership_remove_member(m, p);
                    ped_group_process(g);
                }
            }
        }
        // 06CA ATTACH_SEARCHLIGHT_TO_SEARCHLIGHT_OBJECT (0x497D54): objects looked up SP3, SP2, SP1;
        // `AttachSearchlightToSearchlightObject(SP0, obj(SP1), obj(SP2), obj(SP3), (SP4,SP5,SP6))`.
        0x06CA => {
            collect(s, 7);
            let op = pool(OBJECT_POOL);
            let o3 = object_at_ref(op, spi(3));
            let o2 = object_at_ref(op, spi(2));
            let o1 = object_at_ref(op, spi(1));
            attach_searchlight(sp(0), o1, o2, o3, sp(4), sp(5), sp(6));
        }
        // 06D1 SET_VAR_TEXT_LABEL16 (0x497DE8) / 06D2 SET_LVAR_TEXT_LABEL16 (0x497E1E): `p = var(2 / 1)`; 16-byte label;
        // `strncpy(p, label, 16)`.
        0x06D1 | 0x06D2 => {
            let p = var_ptr(s, if command == 0x06D1 { 2 } else { 1 });
            let mut buf = [0u8; 16];
            let b = GPtr::from_raw(buf.as_mut_ptr());
            read_text_label(s, b, 16);
            crate::vm::strncpy(p.cast(), b, 16);
        }
        // 06DC SET_TRAIN_SPEED (0x498025) / 06DD SET_TRAIN_CRUISE_SPEED (0x49805A): `CTrain::SetTrainSpeed /
        // SetTrainCruiseSpeed(veh(SP0), SP1)`.
        0x06DC | 0x06DD => {
            collect(s, 2);
            let v = veh(sp(0));
            if command == 0x06DC {
                set_train_speed(v, sp(1));
            } else {
                set_train_cruise_speed(v, sp(1));
            }
        }
        // 06DE GET_TRAIN_CABOOSE (0x498090): `SP0 = VehiclePool.GetRef(CTrain::FindCaboose(veh(SP0)))`; protected store.
        0x06DE => {
            collect(s, 1);
            let c = find_caboose(veh(sp(0)));
            let h = vehicle_get_ref_(pool(VEHICLE_POOL), c);
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 06E1 TASK_CAR_MISSION (0x498101): `v = SP1 >= 0 ? veh(SP1) : null`, `target = SP2 >= 0 ? veh(SP2) : null`;
        // `new(0x2C) CTaskComplexCarDriveMission(v, target, SP3, SP5, SP4)`; give.
        0x06E1 => {
            collect(s, 6);
            let p = sp(0);
            let v = if spi(1) < 0 { GPtr::NULL } else { veh(sp(1)) };
            let target = if spi(2) < 0 { GPtr::NULL } else { veh(sp(2)) };
            let (mission, speed, style) = (sp(3), sp(4), sp(5));
            let t = new_task(0x2C, |t| task_car_drive_mission_ctor(t, v, target, mission, style, speed));
            give(s, p, t, command);
        }
        // 06E2 TASK_GO_TO_OBJECT (0x4981BE): `new(0x4C) CTaskComplexSeekEntity<Standard>(obj(SP1), SP2, 1000, SP3,
        // [0x859E30], [0x859E34], true, true)`; give.
        0x06E2 => {
            collect(s, 4);
            let p = sp(0);
            let o = object_at_ref(pool(OBJECT_POOL), spi(1));
            let (time, radius) = (sp(2), sp(3));
            let t = new_task(0x4C, |t| {
                task_seek_entity_ctor(t, o, time, 0x3E8, radius, rd(0x859E30), rd(0x859E34), 1, 1)
            });
            give(s, p, t, command);
        }
        // 06E3 TASK_WEAPON_ROLL (0x498259): `new(0x20) CTaskSimpleRunAnim(0, SP1 != 0 ? 0x3B : 0x39, 4.0, false)`; give.
        0x06E3 => {
            collect(s, 2);
            let p = sp(0);
            let anim = if sp(1) != 0 { 0x3B } else { 0x39 };
            let t = new_task(0x20, |t| task_run_anim_ctor(t, 0, anim, 0x4080_0000, 0));
            give(s, p, t, command);
        }
        // 06E4 TASK_CHAR_ARREST_CHAR (0x4982EF): `new(0x24) CTaskComplexArrestPed(ped(SP1))`; give.
        0x06E4 => {
            collect(s, 2);
            let p = sp(0);
            let target = ped(sp(1));
            let t = new_task(0x24, |t| task_arrest_ped_ctor(t, target));
            give(s, p, t, command);
        }
        // 06E5 GET_AVAILABLE_VEHICLE_MOD (0x498340): `mi = ms_modelInfoPtrs[veh(SP0)->m_nModelIndex]`; `SP0 =
        // (int16) mi[+0x2D6 + SP1*2]` (no bounds check); StoreParameters(1).
        0x06E5 => {
            collect(s, 2);
            let v = veh(sp(0));
            let mi = model_info(i32::from(v.byte_add(0x22).cast::<u16>().read() as i16));
            let w = mi.byte_add(0x2D6).byte_add((spi(1) as isize).wrapping_mul(2)).cast::<u16>().read();
            set_sp(0, i32::from(w as i16) as u32);
            store(s, 1);
        }
        // 06E6 GET_VEHICLE_MOD_TYPE (0x498388): `f = (u16) ms_modelInfoPtrs[model rule(SP0)][+0x12]`; the type from
        // the jump tables (`vehicle_mod_type`); a type without a value stores SP0 unchanged (original quirk).
        0x06E6 => {
            collect(s, 1);
            let fl = model_info(model_id(spi(0))).byte_add(0x12).cast::<u16>().read();
            if let Some(v) = vehicle_mod_type(fl) {
                set_sp(0, v);
            }
            store(s, 1);
        }
        // 06EB MARK_VEHICLE_MOD_AS_NO_LONGER_NEEDED (0x49868D): `SetMissionDoesntRequireModel(m)`; `o = (int16)
        // ms_linkedUpgrades.FindOtherUpgrade(m)`; `o != -1` -> `SetMissionDoesntRequireModel(o)`.
        0x06EB => {
            collect(s, 1);
            let m = model_id(spi(0));
            set_mission_doesnt_require_model(m);
            let o = i32::from(find_other_upgrade(GPtr::from_addr(0xB4E6D8), m) as i16);
            if o != -1 {
                set_mission_doesnt_require_model(o);
            }
        }
        // 06EC GET_NUM_AVAILABLE_PAINTJOBS (0x4986DC): `SP0 = ms_modelInfoPtrs[veh(SP0)->m_nModelIndex]->GetNumRemaps()`;
        // protected store.
        0x06EC => {
            collect(s, 1);
            let v = veh(sp(0));
            let n = get_num_remaps(model_info(i32::from(v.byte_add(0x22).cast::<u16>().read() as i16)));
            set_sp(0, n as u32);
            store(s, 1);
        }
        // 06EE IS_GROUP_MEMBER (0x49873B) / 06EF IS_GROUP_LEADER (0x498796): `p = ped(SP0)`; `g =
        // GetActualScriptThingIndex(SP1, 8)`; flag = 0 <= g < 8 (06EE: and `CPedGroups::ms_activeGroups[g]` 0xC098E0) and
        // `ms_groups[g].m_groupMembership.IsMember(p)` / `.IsLeader(p)`.
        0x06EE | 0x06EF => {
            collect(s, 2);
            let p = ped(sp(0));
            let g = get_actual_script_thing_index(spi(1), 8);
            let mut b = false;
            if (0..8).contains(&g) {
                let m = GPtr::<u8>::from_addr((0xC09928 + g * 0x2D4) as usize);
                if command == 0x06EE {
                    if GPtr::<u8>::from_addr((0xC098E0 + g) as usize).read() != 0 {
                        b = membership_is_member(m, p) != 0;
                    }
                } else {
                    b = membership_is_leader(m, p) != 0;
                }
            }
            set_flag(s, b);
        }
        // 06F0 SET_GROUP_SEPARATION_RANGE (0x498801): `g = GetActualScriptThingIndex(SP0, 8)`; 0 <= g < 8 ->
        // `[0xC0994C + g*0x2D4] = SP1` (raw).
        0x06F0 => {
            collect(s, 2);
            let g = get_actual_script_thing_index(spi(0), 8);
            if (0..8).contains(&g) {
                GPtr::<u32>::from_addr((0xC0994C + g * 0x2D4) as usize).write(sp(1));
            }
        }
        // 06F5 GET_SCRIPT_FIRE_COORDS (0x49888D): `i = GetActualScriptThingIndex(SP0, 5)`; 0 <= i < 60 -> SP0..2 =
        // `*gFireManager.GetScriptFireCoords(i)`, else 0; StoreParameters(3).
        0x06F5 => {
            collect(s, 1);
            let i = get_actual_script_thing_index(spi(0), 5);
            let v = if (0..0x3C).contains(&i) {
                let p = get_script_fire_coords(GPtr::from_addr(0xB71F80), i);
                [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()]
            } else {
                [0; 3]
            };
            set_sp(1, v[1]);
            set_sp(2, v[2]);
            set_sp(0, v[0]);
            store(s, 3);
        }
        // 06F8 GET_NTH_CLOSEST_CAR_NODE_WITH_HEADING (0x498900): -100 ground rule on SP2; `node =
        // ThePaths.FindNthNodeClosestToCoors((SP0,SP1,z), 0, 999999.0, 0, 1, SP3 - 1, 0, 0, 0)`; `c =
        // FindNodeCoorsForScript(node, &found)`; found: SP0..2 = c, SP3 = FindNodeOrientationForCarPlacement(node)
        // (ST0 stored f32), flag 1; else SP0..3 = 0, flag 0; StoreParameters(4), then UpdateCompareFlag.
        0x06F8 => {
            collect(s, 4);
            let (x, y) = (sp(0), sp(1));
            let z = ground_rule(x, y, sp(2));
            let paths = GPtr::<u8>::from_addr(0x96F050);
            let mut ret = [0u32; 1];
            let node = find_nth_node_closest_to_coors(
                paths,
                GPtr::from_raw(ret.as_mut_ptr()),
                x,
                y,
                z,
                0,
                0x4974_23FE,
                0,
                1,
                sp(3).wrapping_sub(1),
                0,
                0,
                0,
            )
            .read();
            let mut out = [0u32; 3];
            let mut found = [0u8; 1];
            find_node_coors_for_script(paths, GPtr::from_raw(out.as_mut_ptr()), node, GPtr::from_raw(found.as_mut_ptr()));
            let ok = found[0] != 0;
            if ok {
                set_sp(2, out[2]);
                set_sp(0, out[0]);
                set_sp(1, out[1]);
                let h = X87::from_f64_exact(find_node_orientation_for_car_placement(paths, node));
                set_sp(3, h.to_f32().to_bits());
            } else {
                for i in 0..4 {
                    set_sp(i, 0);
                }
            }
            store(s, 4);
            set_flag(s, ok);
        }
        // 06FC DOES_CAR_HAVE_STUCK_CAR_CHECK (0x498A4B): flag = `CTheScripts::StuckCars.IsCarInStuckCarArray(SP0)`.
        0x06FC => {
            collect(s, 1);
            let b = is_car_in_stuck_car_array(GPtr::from_addr(0xA90AB0), sp(0)) != 0;
            set_flag(s, b);
        }
        // 06FD SET_PLAYBACK_SPEED (0x498A86): `CVehicleRecording::SetPlaybackSpeed(veh(SP0), SP1)`.
        0x06FD => {
            collect(s, 2);
            let k = sp(1);
            set_playback_speed(veh(sp(0)), k);
        }
        // 06FF ARE_ANY_CHARS_NEAR_CHAR (0x498AB7): `p = ped(SP0)`, `r2 = SP1 * SP1` (f32), `pos = p's position`; over
        // the ped pool from the last slot down (`m_nSize` read once): `e = GetAt(i)`, e != null and != p: `d = e's
        // position - pos` (f32 per component), `d.SquaredMagnitude() < r2` -> found; the loop stops at the next
        // iteration after a hit (index decremented first). flag = found.
        0x06FF => {
            collect(s, 2);
            let pp = pool(PED_POOL);
            let p = ped(sp(0));
            let pos = entity_pos(p);
            let r = X87::from_f32(spf(1));
            let r2 = r.fmul(r).to_f32();
            let mut i = pp.byte_add(8).cast::<u32>().read();
            let mut found = false;
            while i != 0 {
                i -= 1;
                if found {
                    break;
                }
                let e = ped_pool_get_at(pp, i);
                if !e.is_null() && e != p {
                    let ep = entity_pos(e);
                    let mut d = [0u32; 3];
                    for k in 0..3 {
                        d[k] = X87::from_f32(f(ep[k])).fsub_m(f(pos[k])).to_f32().to_bits();
                    }
                    let m = X87::from_f64_exact(vector_squared_magnitude(GPtr::from_raw(d.as_mut_ptr())));
                    if m.fcom(X87::from_f32(r2)) == sa_math::x87::Cmp::Less {
                        found = true;
                    }
                }
            }
            set_flag(s, found);
        }
        // 0702 GET_PERCENTAGE_TAGGED_IN_AREA (0x498BE0): `SP0 = CTagManager::GetPercentageTaggedInArea(&rect)`
        // (`tag_rect`); protected store.
        0x0702 => {
            collect(s, 4);
            let mut r = tag_rect();
            let v = get_percentage_tagged_in_area(GPtr::from_raw(r.as_mut_ptr()));
            set_sp(0, v as u32);
            store(s, 1);
        }
        // 0703 SET_TAG_STATUS_IN_AREA (0x498CA4): `CTagManager::SetAlphaInArea(&rect, SP4 != 0 ? 0xFF : 0)`.
        0x0703 => {
            collect(s, 5);
            let mut r = tag_rect();
            let a = if sp(4) != 0 { 0xFF } else { 0 };
            set_alpha_in_area(GPtr::from_raw(r.as_mut_ptr()), a);
        }
        // 0704 CAR_GOTO_COORDINATES_RACING (0x498D79): as 02C2 (ProcessCommands700To799) with the missions 9 (joined)
        // / 0x21, each skipped when the mission is 0x39 or 0x3A; then status `(b & 7) | 0x18`, `0x41BDD0(v, 1)`,
        // cruise speed at least 1, `+0x3AC = CTimer::m_snTimeInMilliseconds`.
        0x0704 => {
            collect(s, 4);
            let v = veh(sp(0));
            let (x, y) = (sp(1), sp(2));
            let zl = ground_rule(x, y, sp(3));
            let d = X87::from_f64_exact(get_distance_from_centre_of_mass_to_base_of_model(v));
            let mut pos = [x, y, d.fadd_m(f(zl)).to_f32().to_bits()];
            let ok = join_car_with_road_system_goto_coors(v, GPtr::from_raw(pos.as_mut_ptr()), 0, 0) != 0;
            let mp = v.byte_add(0x3BA);
            let c = mp.read();
            if c != 0x39 && c != 0x3A {
                mp.write(if ok { 9 } else { 0x21 });
            }
            let st = v.byte_add(0x36);
            st.write((st.read() & 7) | 0x18);
            vehicle_41bdd0(v, 1);
            let cs = v.byte_add(0x3D0);
            let c = cs.read();
            cs.write(if c > 1 { c } else { 1 });
            v.byte_add(0x3AC).cast::<u32>().write(rd(0xB7CB84));
        }
        // 0705 START_PLAYBACK_RECORDED_CAR_USING_AI (0x498E78): `StartPlaybackRecordedCar(veh(SP0), SP1, true, false)`.
        0x0705 => {
            collect(s, 2);
            let n = sp(1);
            start_playback_recorded_car(veh(sp(0)), n, 1, 0);
        }
        // 0706 SKIP_IN_PLAYBACK_RECORDED_CAR (0x498EAA): `SkipForwardInRecording(veh(SP0), SP1)`.
        0x0706 => {
            collect(s, 2);
            let k = sp(1);
            skip_forward_in_recording(veh(sp(0)), k);
        }
        _ => return -1,
    }
    0
}

/// 0x06BD IS_LINE_OF_SIGHT_CLEAR (0x497852): `CollectParameters(11)`;
/// `GetIsLineOfSightClear(&(SP0..SP2), &(SP3..SP5), SP6..SP10 != 0, false, false)`; the AL byte is the compare flag.
/// The pushed bools carry stale upper register bytes in the original (the callee reads the low byte); the
/// port passes 0/1.
fn is_line_of_sight_clear(s: Scr) {
    collect(s, 11);
    let a = [sp(0), sp(1), sp(2)];
    let b = [sp(3), sp(4), sp(5)];
    let nz = |i| u32::from(sp(i) != 0);
    let r = get_is_line_of_sight_clear(
        GPtr::from_raw(a.as_ptr().cast_mut()),
        GPtr::from_raw(b.as_ptr().cast_mut()),
        nz(6),
        nz(7),
        nz(8),
        nz(9),
        nz(10),
        0,
        0,
    );
    flag(s, u32::from(r));
}

/// 0x06D5 CREATE_CHECKPOINT (0x497E54): `CollectParameters(8)`; `GetActualScriptThingIndex(
/// CollectNextParameterWithoutIncreasingPC(), 3)` evaluated and discarded; `SP0 = AddScriptCheckpoint(SP1..SP3, SP4..SP6,
/// SP7, SP0)`; with mission cleanup `AddEntityToList(SP0, 0xC)` (0x4974EA); `StoreParameters(1)`.
fn create_checkpoint(s: Scr) {
    collect(s, 8);
    let n = collect_next(s);
    get_actual_script_thing_index(n, 3);
    let h = add_script_checkpoint(sp(1), sp(2), sp(3), sp(4), sp(5), sp(6), sp(7), sp(0));
    set_sp(0, h as u32);
    if uses_mission_cleanup(s) {
        mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), h, 0xC);
    }
    store(s, 1);
}

/// 0x06D6 DELETE_CHECKPOINT (0x497EF9): `CollectParameters(1)`; if `GetActualScriptThingIndex(SP0, 3) >= 0`:
/// `RemoveScriptCheckpoint(SP0)` (the handle, not the index) and, with mission cleanup, `RemoveEntityFromList(SP0, 0xC)`.
fn delete_checkpoint(s: Scr) {
    collect(s, 1);
    if get_actual_script_thing_index(spi(0), 3) < 0 {
        return;
    }
    remove_script_checkpoint(spi(0));
    if uses_mission_cleanup(s) {
        mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), spi(0), 0xC);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1700_TO_1799];
