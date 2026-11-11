//! `CRunningScript::ProcessCommands2400To2499` (0x478000): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-4). Every opcode of the range has a jump-table case (table 0x479AFC, 100 entries) and is in
//! [`PORTED`]; the default (-1, `or al,0xFF` at 0x479ADC) is unreachable for 2400..2499 and kept for other command
//! values. The common exit 0x479ADE only restores the SEH link (no destructor tail). The catalogue's extent of
//! the function stops at the case jump (0x478037); the cases run to 0x479AF6.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands2400To2499.md`.
//!
//! Divergence notes (ledger classes of PORT-SCRIPT-2/3): the 8-byte labels (0964, 098E, 09A4, 09AA, 09B7) and the
//! 15-byte label of 09A9 are read into zeroed buffers; `setcc` bools pushed with stale upper bytes are passed as
//! 0/1; the 0985 object list and count start zeroed (the callee writes the count); 099F builds its task with
//! `CTask::operator new` + constructor (the SEH state stores are not ported).

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x0960, 0x0961, 0x0962, 0x0963, 0x0964, 0x0965, 0x0966, 0x0967, 0x0968, 0x0969, 0x096A, 0x096B, 0x096C,
    0x096D, 0x096E, 0x096F, 0x0970, 0x0971, 0x0972, 0x0973, 0x0974, 0x0975, 0x0976, 0x0977, 0x0978, 0x0979,
    0x097A, 0x097B, 0x097C, 0x097D, 0x097E, 0x097F, 0x0980, 0x0981, 0x0982, 0x0983, 0x0984, 0x0985, 0x0986,
    0x0987, 0x0988, 0x0989, 0x098A, 0x098B, 0x098C, 0x098D, 0x098E, 0x098F, 0x0990, 0x0991, 0x0992, 0x0993,
    0x0994, 0x0995, 0x0996, 0x0997, 0x0998, 0x0999, 0x099A, 0x099B, 0x099C, 0x099D, 0x099E, 0x099F, 0x09A0,
    0x09A1, 0x09A2, 0x09A3, 0x09A4, 0x09A5, 0x09A6, 0x09A7, 0x09A8, 0x09A9, 0x09AA, 0x09AB, 0x09AC, 0x09AD,
    0x09AE, 0x09AF, 0x09B0, 0x09B1, 0x09B2, 0x09B3, 0x09B4, 0x09B5, 0x09B6, 0x09B7, 0x09B8, 0x09B9, 0x09BA,
    0x09BB, 0x09BC, 0x09BD, 0x09BE, 0x09BF, 0x09C0, 0x09C1, 0x09C2, 0x09C3,
];

/// `CHud::bScriptDontDisplayAreaName` 0xBAA3F8 / `bScriptDontDisplayVehicleName` 0xBAA3F9 (owned by the HUD; written
/// by address as the original does).
const SCRIPT_DONT_DISPLAY_AREA_NAME: usize = 0xBAA3F8;
const SCRIPT_DONT_DISPLAY_VEHICLE_NAME: usize = 0xBAA3F9;
/// `CClumpModelInfo::SetAtomicRendererCB` (0x4C5280), passed as a callback
const SET_ATOMIC_RENDERER_CB: u32 = 0x4C5280;
/// `CVisibilityPlugins::RenderFadingClumpCB` (0x733630), passed as the callback data
const RENDER_FADING_CLUMP_CB: u32 = 0x733630;
/// `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8.
const MODEL_INFO_PTRS: u32 = 0xA9B0C8;
/// `mod_HandlingManager.m_aVehicleHandling[0]` dword +0xCC (0xC2BAAC; 0xE0-byte entries): the model flags.
const HANDLING_MODEL_FLAGS: u32 = 0xC2BAAC;
/// `RsGlobal.maximumWidth` / `maximumHeight` (int32) 0xC17044 / 0xC17048.
const MAXIMUM_WIDTH: usize = 0xC17044;
const MAXIMUM_HEIGHT: usize = 0xC17048;
/// `.rdata` 0x859520: 1/640; 0x859524: 1/448
const K_1_640: u32 = 0x3ACC_CCCD;
const K_1_448: u32 = 0x3B12_4925;
/// `.rdata` 0x859EE0: "DUMMY"
const DUMMY: usize = 0x859EE0;
/// `AudioEngine` 0xB6BC90; `gFireManager` 0xB71F80; `ThePaths` 0x96F050; `g_fxMan` 0xA9AE80; `g_fx` 0xA9AE00.
const AUDIO_ENGINE: usize = 0xB6BC90;
const FIRE_MANAGER: usize = 0xB71F80;
const THE_PATHS: usize = 0x96F050;
const FX_MANAGER: usize = 0xA9AE80;
const FX: usize = 0xA9AE00;
/// `CTheScripts::ScriptsForBrains` 0xA90CF0; `CTheScripts::ScriptResourceManager` 0xA485A8.
const SCRIPTS_FOR_BRAINS: usize = 0xA90CF0;
const SCRIPT_RESOURCE_MANAGER: usize = 0xA485A8;
/// `CTheScripts::ScriptEffectSystemArray[0]` (8 bytes) dword +4 (0xA44114): the `FxSystem_c*`.
const SCRIPT_EFFECT_SYSTEMS: u32 = 0xA44114;
/// `CTheScripts::ScriptCheckpointArray[0]` (8 bytes) dword +4 (0xA44074): the checkpoint.
const SCRIPT_CHECKPOINTS: u32 = 0xA44074;
/// `CWorld::PlayerInFocus` (byte) 0xB7CD74.
const PLAYER_IN_FOCUS: usize = 0xB7CD74;
/// `CLoadedCarGroup` of the population (`CPopulation::m_LoadedCars`) 0xC0E9F8.
const LOADED_CARS: usize = 0xC0E9F8;
/// `CTheZones::ZoneInfoArray[0]` (0x11 bytes) byte +0x10 (0xBA1DFF); `CTheZones::NavigationZoneArray[0]` (0x20 bytes)
/// u16 +0x1C (0xBA37B4): the zone info index.
const ZONE_INFO_FLAGS: u32 = 0xBA1DFF;
const NAV_ZONE_INFO_INDEX: u32 = 0xBA37B4;

orig! {
    /// `CTheScripts::CleanUpThisObject(CObject*)` (0x4866C0)
    fn clean_up_this_object(o: GPtr<u8>) = cdecl 0x4866C0 => unported;
    /// `RpClumpForAllAtomics(RpClump*, RpAtomicCallBack, void*)` (0x749B70)
    fn rp_clump_for_all_atomics(clump: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: u32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// CRT `_stricmp` (0x8229B6)
    fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// `CMenuSystem::CreateNewMenu(eMenuType, const char*, float x, float y, float width, int8 columns, bool
    /// interactive, bool background, eFontAlignment)` (0x582300), AL; floats as raw dwords
    fn create_new_menu(
        ty: u32, title: GPtr<u8>, x: u32, y: u32, w: u32, columns: u32, interactive: u32, background: u32, align: u32
    ) -> u8 = cdecl 0x582300 => unported;
    /// `CPedIntelligence::GetTaskSwim()` (0x601070)
    fn get_task_swim(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601070 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810)
    fn get_task_secondary(tm: GPtr<u8>, slot: u32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CTaskComplexFacial::SetRequest(eFacialExpression, int32, eFacialExpression, int32)` (0x691230)
    fn facial_set_request(t: GPtr<u8>, a: u32, b: u32, c: u32, d: u32) = thiscall 0x691230 => unported;
    /// `CTaskComplexFacial::StopAll()` (0x691250)
    fn facial_stop_all(t: GPtr<u8>) = thiscall 0x691250 => unported;
    /// `CHeli::SwitchPoliceHelis(bool)` (0x6C4800)
    fn switch_police_helis(on: u32) = cdecl 0x6C4800 => unported;
    /// `CShopping::StoreVehicleMods()` (0x49B280) / `RestoreVehicleMods()` (0x49B3C0)
    fn store_vehicle_mods() = cdecl 0x49B280 => unported;
    fn restore_vehicle_mods() = cdecl 0x49B3C0 => unported;
    /// `CVehicle::GetUpgrade(int32)` (0x6D3650) / `GetReplacementUpgrade(int32)` (0x6D3A50)
    fn get_upgrade(v: GPtr<u8>, slot: u32) -> u32 = thiscall 0x6D3650 => unported;
    fn get_replacement_upgrade(v: GPtr<u8>, node: u32) -> u32 = thiscall 0x6D3A50 => unported;
    /// `CGameLogic::ForceDeathRestart()` (0x441240) / `ResetStuffUponResurrection()` (0x442980)
    fn force_death_restart() = cdecl 0x441240 => unported;
    fn reset_stuff_upon_resurrection() = cdecl 0x442980 => unported;
    /// 0x6E76E0 (cdecl, no arguments; gr `CWaterLevel::SyncWater`)
    fn sync_water_6e76e0() = cdecl 0x6E76E0 => unported;
    /// `CRunningScript::SetCharCoordinates(CPed*, float x, y, z, bool warp, bool offset)` (0x464DC0); raw floats
    fn set_char_coordinates(s: Scr, p: GPtr<u8>, x: u32, y: u32, z: u32, warp: u32, offset: u32) = thiscall 0x464DC0 => unported;
    /// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0)
    fn get_actual_script_thing_index(r: u32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `CVehicle::IsLawEnforcementVehicle()` (0x6D2370), AL
    fn is_law_enforcement_vehicle(v: GPtr<u8>) -> u8 = thiscall 0x6D2370 => unported;
    /// `FxManager_c::DestroyFxSystem(FxSystem_c*)` (0x4A9810)
    fn destroy_fx_system(m: GPtr<u8>, fx: u32) = thiscall 0x4A9810 => unported;
    /// `CTheScripts::RemoveScriptEffectSystem(int32)` (0x492FD0)
    fn remove_script_effect_system(h: u32) = cdecl 0x492FD0 => unported;
    /// `CWorld::FindPlayerCentreOfWorld(int32)` (0x56E250): a pointer to the position
    fn find_player_centre_of_world(id: u32) -> GPtr<u32> = cdecl 0x56E250 => unported;
    /// `CScriptsForBrains::IsObjectWithinBrainActivationRange(CObject*, const CVector&)` (0x46B3D0), AL
    fn is_object_within_brain_activation_range(b: GPtr<u8>, o: GPtr<u8>, pos: GPtr<u32>) -> u8 = thiscall 0x46B3D0 => unported;
    /// `CDecisionMakerTypes::GetInstance()` (0x4684F0)
    fn decision_maker_types_get_instance() -> GPtr<u8> = cdecl 0x4684F0 => unported;
    /// 0x6070F0 (thiscall on the `CDecisionMakerTypes` instance, 3 dwords `(int32 src, int32 0, bool cleanup)`, returns
    /// the new decision maker index; gr `CDecisionMakerTypes::CopyDecisionMaker`)
    fn copy_decision_maker_6070f0(t: GPtr<u8>, src: u32, ty: u32, cleanup: u32) -> u32 = thiscall 0x6070F0 => unported;
    /// `CTheScripts::GetNewUniqueScriptThingIndex(int32 index, uint8 type)` (0x483720)
    fn get_new_unique_script_thing_index(i: u32, ty: u32) -> u32 = cdecl 0x483720 => unported;
    /// `CScriptResourceManager::AddToResourceManager(int32, uint32 type, CRunningScript*)` (0x4704B0)
    fn add_to_resource_manager(m: GPtr<u8>, id: u32, ty: u32, s: Scr) = thiscall 0x4704B0 => unported;
    /// `CAudioEngine::ReportMissionAudioEvent(uint16, CVector&)` (0x507340)
    fn report_mission_audio_event_pos(a: GPtr<u8>, ev: u32, pos: GPtr<u32>) = thiscall 0x507340 => unported;
    /// `CAudioEngine::ReportMissionAudioEvent(uint16, CObject*)` (0x507350)
    fn report_mission_audio_event_obj(a: GPtr<u8>, ev: u32, o: GPtr<u8>) = thiscall 0x507350 => unported;
    /// `CAudioEngine::AttachMissionAudioToObject(uint8 slot, CObject*)` (0x507320)
    fn attach_mission_audio_to_object(a: GPtr<u8>, slot: u32, o: GPtr<u8>) = thiscall 0x507320 => unported;
    /// `CFireManager::ExtinguishPoint(CVector, float)` (0x539450); the vector by value, raw
    fn extinguish_point(m: GPtr<u8>, x: u32, y: u32, z: u32, r: u32) = thiscall 0x539450 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), raw floats, ST0 (read as f64: exact)
    fn find_ground_z_raw(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CWorld::FindObjectsOfTypeInRange(uint32 model, const CVector&, float radius, bool 2d, int16* count,
    /// int16 max, CEntity** out, bool buildings, bool vehicles, bool peds, bool objects, bool dummies)`
    /// (0x564C70); the float raw
    fn find_objects_of_type_in_range(model: u32, pos: GPtr<u32>, radius: u32, two_d: u32, count: GPtr<u16>, max_count: u32, out: GPtr<u32>, b: u32, v: u32, p: u32, o: u32, d: u32) = cdecl 0x564C70 => unported;
    /// `CVector::Magnitude()` (0x4082C0), ST0 (an `fsqrt` result under PC=24: exact as f64)
    fn vector_magnitude(v: GPtr<u32>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CFireManager::ClearAllScriptFireFlags()` (0x5397A0)
    fn clear_all_script_fire_flags(m: GPtr<u8>) = thiscall 0x5397A0 => unported;
    /// `CPool<CVehicle>::GetRef(CVehicle*)` (0x424160), ECX = the vehicle pool
    fn vehicle_get_ref(pool: GPtr<u8>, v: GPtr<u8>) -> u32 = thiscall 0x424160 => unported;
    /// `CVehicle::GetRemapIndex()` (0x6D0B70)
    fn get_remap_index(v: GPtr<u8>) -> u32 = thiscall 0x6D0B70 => unported;
    /// `CAutomobile::GetMovingCollisionOffset()` (0x6A2150), ST0 (a loaded float: exact as f64)
    fn get_moving_collision_offset(v: GPtr<u8>) -> f64 = thiscall 0x6A2150 => unported;
    /// `CEntryExitManager::SetEntryExitFlag(const char*, uint32, bool)` (0x43EF20)
    fn set_entry_exit_flag(name: GPtr<u8>, flag: u32, on: u32) = cdecl 0x43EF20 => unported;
    /// `CAudioEngine::PauseBeatTrack(bool)` (0x507200)
    fn pause_beat_track(a: GPtr<u8>, b: u32) = thiscall 0x507200 => unported;
    /// `CPathFind::MarkRoadNodeAsDontWander(float, float, float)` (0x450560); raw floats
    fn mark_road_node_as_dont_wander(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x450560 => unported;
    /// `CPathFind::UnMarkAllRoadNodesAsDontWander()` (0x44D400)
    fn unmark_all_road_nodes_as_dont_wander(p: GPtr<u8>) = thiscall 0x44D400 => unported;
    /// `CCheckpoints::SetHeading(uint32 id, float)` (0x722970); raw float
    fn checkpoints_set_heading(id: u32, h: u32) = cdecl 0x722970 => unported;
    /// `CStats::SetStatValue(uint16, float)` (0x55A070) / `IncrementStat(uint16, float)` (0x55C180)
    fn set_stat_value(stat: u32, v: f32) = cdecl 0x55A070 => unported;
    fn increment_stat(stat: u32, v: f32) = cdecl 0x55C180 => unported;
    /// `CVehicleRecording::ChangeCarPlaybackToUseAI(CVehicle*)` (0x45A360)
    fn change_car_playback_to_use_ai(v: GPtr<u8>) = cdecl 0x45A360 => unported;
    /// `CCamera::AddShakeSimple(float, int32, float)` (0x50D240); the three operand dwords raw
    fn add_shake_simple(c: GPtr<u8>, a: u32, b: u32, d: u32) = thiscall 0x50D240 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskSimpleSetCharIgnoreWeaponRangeFlag::CTaskSimpleSetCharIgnoreWeaponRangeFlag(bool)` (0x474620)
    fn task_ignore_weapon_range_ctor(t: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x474620 => unported;
    /// `CRunningScript::GivePedScriptedTask(int32 ped, CTask*, int32 command)` (0x465C20)
    fn give_ped_scripted_task(s: Scr, ped: u32, task: GPtr<u8>, command: u32) = thiscall 0x465C20 => unported;
    /// `CRunningScript::ScriptTaskPickUpObject(int32 command)` (0x46AF50; reads its own parameters)
    fn script_task_pick_up_object(s: Scr, command: u32) = thiscall 0x46AF50 => unported;
    /// `CTask` virtual slot 4 (+0x10, `GetTaskType()`)
    fn task_get_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CTask` virtual slot 6 (+0x18, `MakeAbortable(CPed*, eAbortPriority, const CEvent*)`), AL (discarded)
    fn task_make_abortable(t: GPtr<u8>, p: GPtr<u8>, prio: u32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskSimpleHoldEntity::DropEntity(CPed*, bool)` (0x6930F0)
    fn hold_entity_drop_entity(t: GPtr<u8>, p: GPtr<u8>, b: u32) = thiscall 0x6930F0 => unported;
    /// `CConversations::SetUpConversationNode(char*, char*, char*, int32, int32, int32)` (0x43A870)
    fn set_up_conversation_node(a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, x: u32, y: u32, z: u32) = cdecl 0x43A870 => unported;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    fn get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => unported;
    /// `CVehicle::PickRandomPassenger()` (0x6D2A10)
    fn pick_random_passenger(v: GPtr<u8>) -> GPtr<u8> = thiscall 0x6D2A10 => unported;
    /// `CPed::Say(uint16, uint32, float, uint8, uint8, uint8)` (0x5EFFE0), AX (discarded); the float raw
    fn ped_say(p: GPtr<u8>, id: u32, a: u32, vol: u32, b: u32, c: u32, d: u32) -> u16 = thiscall 0x5EFFE0 => unported;
    /// `CGameLogic::SetUpSkip(CVector, float, bool, CVehicle*, bool)` (0x4423C0); the vector by value, raw
    fn set_up_skip(x: u32, y: u32, z: u32, heading: u32, a: u32, b: u32, c: u32) = cdecl 0x4423C0 => unported;
    /// `CLoadedCarGroup::PickRandomCar(bool, bool)` (0x611C50)
    fn pick_random_car(g: GPtr<u8>, a: u32, b: u32) -> i32 = thiscall 0x611C50 => unported;
    /// `CEntryExitManager::FindNearestEntryExit(const CVector2D&, float range, int32 ignore)` (0x43F4B0)
    fn find_nearest_entry_exit(pos: GPtr<u32>, range: u32, ignore: i32) -> u32 = cdecl 0x43F4B0 => unported;
    /// 0x43EF90 (cdecl `(int32 index, uint32 flag, bool on)`; gr `CEntryExitManager::SetEntryExitFlagWithIndex`)
    fn set_entry_exit_flag_with_index_43ef90(i: u32, flag: u32, on: u32) = cdecl 0x43EF90 => unported;
    /// `CTheZones::FindZoneByLabel(const char*, eZoneType)` (0x572C40), AX
    fn find_zone_by_label(label: GPtr<u8>, ty: u32) -> u16 = cdecl 0x572C40 => unported;
    /// `Fx_c::AddBlood(CVector& pos, CVector& dir, int32 amount, float lightMult)` (0x49EB00); the float raw
    fn fx_add_blood(fx: GPtr<u8>, pos: GPtr<u32>, dir: GPtr<u32>, amount: u32, light: u32) = thiscall 0x49EB00 => unported;
    /// `CDamageManager::GetDoorStatus(eDoors)` (0x6C2230)
    fn get_door_status(dm: GPtr<u8>, door: u32) -> u32 = thiscall 0x6C2230 => unported;
    /// The vehicle pool's slot accessor at 0x41CC10 (thiscall on the pool, slot index; a `.HOODLUM` thunk,
    /// `CPool<CVehicle>::GetAt(int32)` in gr)
    fn vehicle_pool_get_at(pool: GPtr<u8>, i: u32) -> GPtr<u8> = thiscall 0x41CC10 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn get_vehicle_appearance(v: GPtr<u8>) -> u32 = thiscall 0x6D1080 => unported;
    /// `CVehicle::CanBeDeleted()` (0x6D1180), AL
    fn can_be_deleted(v: GPtr<u8>) -> u8 = thiscall 0x6D1180 => unported;
    /// 0x477DA0 (cdecl `(const CVector* pos, float x1, float y1, float x2, float y2, float width)`, AL: the angled
    /// area test; unnamed in the catalogue)
    fn is_in_angled_area_477da0(pos: GPtr<u8>, x1: u32, y1: u32, x2: u32, y2: u32, w: u32) -> u8 = cdecl 0x477DA0 => unported;
    /// `CDarkel::FailKillFrenzy()` (0x43DC60)
    fn fail_kill_frenzy() = cdecl 0x43DC60 => unported;
}

/// `PED(h)`.
#[inline]
fn ped(h: u32) -> GPtr<u8> {
    ped_at_ref(pool(PED_POOL), h as i32)
}

/// `VEH(h)`.
#[inline]
fn veh(h: u32) -> GPtr<u8> {
    vehicle_at_ref(pool(VEHICLE_POOL), h as i32)
}

/// `OBJ(h)`.
#[inline]
fn obj(h: u32) -> GPtr<u8> {
    object_at_ref(pool(OBJECT_POOL), h as i32)
}

/// `CModelInfo::ms_modelInfoPtrs[(int16) e->m_nModelIndex (+0x22)]`.
#[inline]
fn model_info_of(e: GPtr<u8>) -> GPtr<u8> {
    let m = i32::from(e.byte_add(0x22).cast::<i16>().read());
    GPtr::<GPtr<u8>>::from_addr(MODEL_INFO_PTRS.wrapping_add((m as u32).wrapping_mul(4)) as usize).read()
}

/// An entity's position: `m_matrix (+0x14) ? &m_matrix->pos (+0x30) : &m_placement.pos (+4)`.
#[inline]
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = e.byte_add(0x14).cast::<GPtr<u8>>().read();
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `p` byte bit `mask` set to `on` (`setne; shl; xor; and; xor`).
#[inline]
fn set_byte_bit(p: GPtr<u8>, mask: u8, on: bool) {
    let old = p.read();
    p.write(if on { old | mask } else { old & !mask });
}

/// The ped flag dword +0x478: `flags ^= ((v << k) ^ flags) & mask`, i.e. bit `k` = bit 0 of `v` (the tail
/// 0x479600 `xor [eax+0x478],edx` of 0982/099F/09B5/09B6 and the inline forms of 0961/09A7).
#[inline]
fn set_ped_flag_bit(p: GPtr<u8>, k: u32, v: u32) {
    let fl = p.byte_add(0x478).cast::<u32>();
    let old = fl.read();
    fl.write(old ^ (((v << k) ^ old) & (1 << k)));
}

/// The min/max ordering of 09C3: `fld a; fcomp b; fnstsw; test ah,0x41; jne keep`, i.e. the two dwords are swapped
/// when `a > b` (ordered; the `fld/fstp` copy of a non-NaN is exact).
#[inline]
fn order(a: u32, b: u32) -> (u32, u32) {
    if f(a) > f(b) { (b, a) } else { (a, b) }
}

/// 0964 CREATE_MENU_GRID (0x4780D1): 8-byte label (zeroed buffer); title = label unless "DUMMY" (`_stricmp`; byte
/// loop up to the terminator); `CollectParameters(7)`; x87 (PC=24): `A = fild maxWidth * 1/640` (register), x =
/// SP0 * A, w = A * SP2, y = `fild maxHeight * 1/448 * SP1` (stored f32); `SP0 = (u8)CreateNewMenu(1, title, x,
/// y, w, SP3, SP4, SP5, SP6)`; `StoreParameters(1)`.
fn create_menu_grid(s: Scr) {
    let mut label = [0u8; 16];
    let lp = GPtr::from_raw(label.as_mut_ptr());
    read_text_label(s, lp, 8);
    let mut title = [0u8; 16];
    if stricmp(lp, GPtr::from_addr(DUMMY)) != 0 {
        let mut i = 0;
        loop {
            let c = label[i];
            title[i] = c;
            i += 1;
            if c == 0 {
                break;
            }
        }
    }
    collect(s, 7);
    let a = X87::from_i32(GPtr::<i32>::from_addr(MAXIMUM_WIDTH).read()).fmul_m(f(K_1_640));
    let x = X87::from_f32(spf(0)).fmul(a).to_f32().to_bits();
    let w = a.fmul_m(spf(2)).to_f32().to_bits();
    let y = X87::from_i32(GPtr::<i32>::from_addr(MAXIMUM_HEIGHT).read())
        .fmul_m(f(K_1_448))
        .fmul_m(spf(1))
        .to_f32()
        .to_bits();
    let id = create_new_menu(1, GPtr::from_raw(title.as_mut_ptr()), x, y, w, sp(3), sp(4), sp(5), sp(6));
    set_sp(0, u32::from(id));
    store(s, 1);
}

/// 096D GET_CURRENT_CAR_MOD (0x478315): `v = VEH(SP0)`; by slot SP1 (inner jump table 0x479C8C, unsigned > 16 →
/// -1): 0 → GetUpgrade(0); 1 → GetUpgrade(1), or GetUpgrade(2) when that is -1; 2 → 6; 3 → GetUpgrade(8), or
/// GetUpgrade(9) when -1; 4..10 → 10, 11, 12, 14, 15, 16, 17; 11 → -1; 12..16 → GetReplacementUpgrade(2, 0x13,
/// 0xC, 0xD, 0x14). SP0 = the result; `StoreParameters(1)`.
fn get_current_car_mod(s: Scr) {
    collect(s, 2);
    let v = veh(sp(0));
    let r = match sp(1) {
        0 => get_upgrade(v, 0),
        1 => {
            let r = get_upgrade(v, 1);
            if r == u32::MAX { get_upgrade(v, 2) } else { r }
        }
        2 => get_upgrade(v, 6),
        3 => {
            let r = get_upgrade(v, 8);
            if r == u32::MAX { get_upgrade(v, 9) } else { r }
        }
        4 => get_upgrade(v, 10),
        5 => get_upgrade(v, 11),
        6 => get_upgrade(v, 12),
        7 => get_upgrade(v, 14),
        8 => get_upgrade(v, 15),
        9 => get_upgrade(v, 16),
        10 => get_upgrade(v, 17),
        12 => get_replacement_upgrade(v, 2),
        13 => get_replacement_upgrade(v, 0x13),
        14 => get_replacement_upgrade(v, 0xC),
        15 => get_replacement_upgrade(v, 0xD),
        16 => get_replacement_upgrade(v, 0x14),
        _ => u32::MAX,
    };
    set_sp(0, r);
    store(s, 1);
}

/// 0976 KILL_FX_SYSTEM_NOW (0x47866F): `i = GetActualScriptThingIndex(SP0, 1)`; i >= 0 and `fx = [0xA44114 + i *
/// 8]` non-null: `g_fxMan.DestroyFxSystem(fx)`, `RemoveScriptEffectSystem(SP0)`, mission cleanup
/// `RemoveEntityFromList(SP0, 4)`.
fn kill_fx_system_now(s: Scr) {
    collect(s, 1);
    let h = sp(0);
    let i = get_actual_script_thing_index(h, 1);
    if i < 0 {
        return;
    }
    let fx = GPtr::<u32>::from_addr(SCRIPT_EFFECT_SYSTEMS.wrapping_add(i as u32 * 8) as usize).read();
    if fx == 0 {
        return;
    }
    destroy_fx_system(GPtr::from_addr(FX_MANAGER), fx);
    remove_script_effect_system(h);
    if uses_mission_cleanup(s) {
        mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), h as i32, 4);
    }
}

/// 0977 IS_OBJECT_WITHIN_BRAIN_ACTIVATION_RANGE (0x4786D6): `o = OBJ(SP0)`; `i = (u8)CWorld::PlayerInFocus`; flag 0
/// unless `Players[i].m_pPed` and `o`: `pos = *FindPlayerCentreOfWorld(i)` (copied), flag = AL of
/// `ScriptsForBrains.IsObjectWithinBrainActivationRange(o, pos) != 0`.
fn is_object_within_brain_activation_range_case(s: Scr) {
    collect(s, 1);
    let o = obj(sp(0));
    let i = u32::from(GPtr::<u8>::from_addr(PLAYER_IN_FOCUS).read());
    let mut b = false;
    if player_info(i).cast::<u32>().read() != 0 && !o.is_null() {
        let c = find_player_centre_of_world(i);
        let mut pos = [c.read(), c.byte_add(4).read(), c.byte_add(8).read()];
        b = is_object_within_brain_activation_range(GPtr::from_addr(SCRIPTS_FOR_BRAINS), o, GPtr::from_raw(pos.as_mut_ptr())) != 0;
    }
    set_flag(s, b);
}

/// 0978 COPY_SHARED_CHAR_DECISION_MAKER (0x47875E): `src = SP0 == -1 ? -1 : GetActualScriptThingIndex(SP0, 7)`; `h =
/// CollectNextParameterWithoutIncreasingPC()`; `GetActualScriptThingIndex(h, 7) != -1` → SP0 = h; otherwise `SP0 =
/// GetNewUniqueScriptThingIndex(0x6070F0(GetInstance(), src, 0, m_UsesMissionCleanup), 7)` and with mission cleanup
/// `AddEntityToList(SP0, 9)`. Then `ScriptResourceManager.AddToResourceManager(SP0, 3, this)`; `StoreParameters(1)`.
fn copy_shared_char_decision_maker(s: Scr) {
    collect(s, 1);
    let mut src = u32::MAX;
    if sp(0) != u32::MAX {
        src = get_actual_script_thing_index(sp(0), 7) as u32;
    }
    let h = collect_next_parameter_without_increasing_pc(s) as u32;
    let r = if get_actual_script_thing_index(h, 7) != -1 {
        set_sp(0, h);
        h
    } else {
        let cleanup = u32::from(s.byte_add(crate::vm::USES_MISSION_CLEANUP).read());
        let inst = decision_maker_types_get_instance();
        let dm = copy_decision_maker_6070f0(inst, src, 0, cleanup);
        let n = get_new_unique_script_thing_index(dm, 7);
        set_sp(0, n);
        if uses_mission_cleanup(s) {
            mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), n as i32, 9);
            sp(0)
        } else {
            n
        }
    };
    add_to_resource_manager(GPtr::from_addr(SCRIPT_RESOURCE_MANAGER), r, 3, s);
    store(s, 1);
}

/// 0985 SET_CHAR_USES_COLLISION_CLOSEST_OBJECT_OF_TYPE (0x478A13): `CollectParameters(7)` (x, y, z, radius, model,
/// flag, ped); z = ground z when `z <= -100` (ordered); best = radius + radius (`fadd st,st`, stored f32); model
/// rule; `FindObjectsOfTypeInRange(model, &pos, radius, true, &n, 16, list, true, false, false, true, true)`; for
/// each of the `(i16)n` entries (u16 index): `d = Magnitude(e.pos - pos)` (components `fsub`, stored f32); taken
/// when `d < best` (`test ah,5; jp`), then best = d (stored f32). `p = PED(SP6)` (called always, no null check);
/// with a closest entity: `p->+0x128 = SP5 != 0 ? null : entity` (`setne; dec; and`).
fn set_char_uses_collision_closest_object_of_type(s: Scr) {
    collect(s, 7);
    let (x, y) = (sp(0), sp(1));
    let mut z = sp(2);
    if f(z) <= f(K_MINUS_100) {
        z = X87::from_f64_exact(find_ground_z_raw(x, y)).to_f32().to_bits();
    }
    let radius = sp(3);
    let mut best = X87::from_f32(f(radius)).fdouble().to_f32();
    let mut model = sp(4);
    if (model as i32) < 0 {
        model = GPtr::<u32>::from_addr(0xA44B88u32.wrapping_sub(model.wrapping_mul(0x1C)) as usize).read();
    }
    let mut pos = [x, y, z];
    let mut n = [0u16; 2];
    let mut list = [0u32; 16];
    find_objects_of_type_in_range(
        model,
        GPtr::from_raw(pos.as_mut_ptr()),
        radius,
        1,
        GPtr::from_raw(n.as_mut_ptr()),
        16,
        GPtr::from_raw(list.as_mut_ptr()),
        1,
        0,
        0,
        1,
        1,
    );
    let count = i32::from(n[0] as i16);
    let mut closest = 0u32;
    let mut i: u16 = 0;
    if count > 0 {
        loop {
            let e = GPtr::<u8>::from_addr(list[usize::from(i)] as usize);
            let p = entity_pos(e).cast::<f32>();
            let mut d = [
                X87::from_f32(p.read()).fsub_m(f(x)).to_f32().to_bits(),
                X87::from_f32(p.byte_add(4).read()).fsub_m(f(y)).to_f32().to_bits(),
                X87::from_f32(p.byte_add(8).read()).fsub_m(f(z)).to_f32().to_bits(),
            ];
            let m = X87::from_f64_exact(vector_magnitude(GPtr::from_raw(d.as_mut_ptr())));
            if m.fcom(X87::from_f32(best)) == Cmp::Less {
                best = m.to_f32();
                closest = e.addr() as u32;
            }
            i = i.wrapping_add(1);
            if i32::from(i) >= count {
                break;
            }
        }
    }
    let p = ped(sp(6));
    if closest != 0 {
        let v = if sp(5) != 0 { 0 } else { closest };
        p.byte_add(0x128).cast::<u32>().write(v);
    }
}

/// 098B STRING_CAT16 / 098C STRING_CAT8 (0x478C89, shared): `a = GetPointerToScriptVariable(2)`, `b =
/// GetPointerToScriptVariable(2)`; `strlen(a) + strlen(b)` (signed) >= 16 (098B) / >= 8 (098C): return 0 without
/// reading the destination operand; else `dst = GetPointerToScriptVariable(2)`, byte copy of `a` with its
/// terminator, then `b` with its terminator appended at the end of `dst` (`rep movsd` dwords, then `rep movsb`).
fn string_cat(s: Scr, command: u32) {
    let a = var_ptr(s, 2).cast::<u8>();
    let b = var_ptr(s, 2).cast::<u8>();
    let strlen = |p: GPtr<u8>| {
        let mut n = 0isize;
        while p.byte_add(n).read() != 0 {
            n += 1;
        }
        n
    };
    let total = strlen(a) as i32 + strlen(b) as i32;
    let max = if command == 0x098B { 16 } else { 8 };
    if total >= max {
        return;
    }
    let dst = var_ptr(s, 2).cast::<u8>();
    let mut i = 0isize;
    loop {
        let c = a.byte_add(i).read();
        dst.byte_add(i).write(c);
        i += 1;
        if c == 0 {
            break;
        }
    }
    let n = strlen(b) + 1;
    let mut end = dst;
    while end.read() != 0 {
        end = end.byte_add(1);
    }
    let words = n / 4;
    for k in 0..words {
        end.byte_add(4 * k).cast::<u32>().write(b.byte_add(4 * k).cast::<u32>().read());
    }
    for k in 4 * words..n {
        end.byte_add(k).write(b.byte_add(k).read());
    }
}

/// 099F TASK_SET_IGNORE_WEAPON_RANGE_FLAG (0x478FD2): `CollectParameters(2)`; SP0 != -1: `PED(SP0)` flags +0x478
/// bit 9 = bit 0 of (u8)SP1 (no null check); SP0 == -1: `new CTaskSimpleSetCharIgnoreWeaponRangeFlag((u8)SP1)`
/// (0xC bytes; null when `new` fails), `GivePedScriptedTask(-1, task, command)`.
fn task_set_ignore_weapon_range_flag(s: Scr, command: u32) {
    collect(s, 2);
    if sp(0) != u32::MAX {
        set_ped_flag_bit(ped(sp(0)), 9, sp(1) & 0xFF);
        return;
    }
    let t = task_new(0xC);
    let t = if t.is_null() { GPtr::NULL } else { task_ignore_weapon_range_ctor(t, sp(1) & 0xFF) };
    give_ped_scripted_task(s, u32::MAX, t, command);
}

/// 09A1 DROP_SECOND_OBJECT (0x47906B): `p = PED(SP0)`; `tm = &p->m_pIntelligence->m_TaskMgr (+4)`; with a secondary
/// task in slot 0 whose type is 0x133 (HOLD_ENTITY): `t->DropEntity(p, SP1 != 0)`, `t->MakeAbortable(p, 1, null)`
/// (GetTaskSecondary is called again before each step, as the original does).
fn drop_second_object(s: Scr) {
    collect(s, 2);
    let p = ped(sp(0));
    let b = u32::from(sp(1) != 0);
    let tm = || p.byte_add(0x47C).cast::<GPtr<u8>>().read().byte_add(4);
    if get_task_secondary(tm(), 0).is_null() {
        return;
    }
    if task_get_type(get_task_secondary(tm(), 0)) != 0x133 {
        return;
    }
    let t = get_task_secondary(tm(), 0);
    hold_entity_drop_entity(t, p, b);
    task_make_abortable(t, p, 1, 0);
}

/// 09B2 GET_RANDOM_CAR_MODEL_IN_MEMORY (0x47949F): `SP0 = m = CPopulation::m_LoadedCars.PickRandomCar(false, SP0 !=
/// 0)`; SP1 = -1 when m == -1, else `(i8) ms_modelInfoPtrs[m]` byte +0x4D (the vehicle class); `StoreParameters(2)`.
fn get_random_car_model_in_memory(s: Scr) {
    collect(s, 1);
    let m = pick_random_car(GPtr::from_addr(LOADED_CARS), 0, u32::from(sp(0) != 0));
    set_sp(0, m as u32);
    if m == -1 {
        set_sp(1, m as u32);
    } else {
        let mi = GPtr::<GPtr<u8>>::from_addr(MODEL_INFO_PTRS.wrapping_add((m as u32).wrapping_mul(4)) as usize).read();
        set_sp(1, i32::from(mi.byte_add(0x4D).read() as i8) as u32);
    }
    store(s, 2);
}

/// 09B4 SET_CLOSEST_ENTRY_EXIT_FLAG (0x479536): `CollectParameters(5)`; `i = FindNearestEntryExit(CVector2D(SP0, SP1),
/// SP2, -1)`; `0x43EF90(i, SP3, SP4 != 0)`.
fn set_closest_entry_exit_flag(s: Scr) {
    collect(s, 5);
    let mut pos = [sp(0), sp(1)];
    let i = find_nearest_entry_exit(GPtr::from_raw(pos.as_mut_ptr()), sp(2), -1);
    set_entry_exit_flag_with_index_43ef90(i, sp(3), u32::from(sp(4) != 0));
}

/// 09B8 ADD_BLOOD (0x47966F): `CollectParameters(8)`; `g_fx.AddBlood(&(SP0, SP1, SP2), &(SP3, SP4, SP5), SP6,
/// PED(SP7)->+0x12C)` (no null check).
fn add_blood(s: Scr) {
    collect(s, 8);
    let mut pos = [sp(0), sp(1), sp(2)];
    let mut dir = [sp(3), sp(4), sp(5)];
    let p = ped(sp(7));
    let light = p.byte_add(0x12C).cast::<u32>().read();
    fx_add_blood(GPtr::from_addr(FX), GPtr::from_raw(pos.as_mut_ptr()), GPtr::from_raw(dir.as_mut_ptr()), sp(6), light);
}

/// 09C0 GET_RANDOM_CAR_OF_TYPE_IN_ANGLED_AREA_NO_SAVE (0x479831): `CollectParameters(6)` (x1, y1, x2, y2, width,
/// model); walks the vehicle pool from slot `m_nSize - 1` down while none is found (slot accessor 0x41CC10): a
/// non-null vehicle with appearance 1 or 2 (GetVehicleAppearance called a second time when the first is not 1),
/// model == SP5 or SP5 < 0, `CanBeDeleted()`, and `0x477DA0(&pos, x1, y1, x2, y2, width)` → handle =
/// `GetRef(v)`. SP0 = handle (-1 when none); `StoreParameters(1)`.
fn get_random_car_of_type_in_angled_area_no_save(s: Scr) {
    collect(s, 6);
    let (x1, y1, x2, y2, w) = (sp(0), sp(1), sp(2), sp(3), sp(4));
    let mut handle = u32::MAX;
    let mut n = pool(VEHICLE_POOL).byte_add(8).cast::<u32>().read();
    while n != 0 {
        n -= 1;
        if handle != u32::MAX {
            break;
        }
        let v = vehicle_pool_get_at(pool(VEHICLE_POOL), n);
        if v.is_null() {
            continue;
        }
        if get_vehicle_appearance(v) != 1 && get_vehicle_appearance(v) != 2 {
            continue;
        }
        let model = i32::from(v.byte_add(0x22).cast::<i16>().read());
        if model != spi(5) && spi(5) >= 0 {
            continue;
        }
        if can_be_deleted(v) == 0 {
            continue;
        }
        if is_in_angled_area_477da0(entity_pos(v), x1, y1, x2, y2, w) != 0 {
            handle = vehicle_get_ref(pool(VEHICLE_POOL), v);
        }
    }
    set_sp(0, handle);
    store(s, 1);
}

/// 09C3 IS_COP_VEHICLE_IN_AREA_3D_NO_SAVE (0x479957): `CollectParameters(6)`; each axis ordered min/max; flag 0, then
/// for every vehicle pool slot `0 <= i < m_nSize` (no early exit): a non-null `IsLawEnforcementVehicle()` vehicle
/// whose model is not 0x1AE with `x1 <= px <= x2`, `y1 <= py <= y2`, `z1 <= pz <= z2` (ordered: `test ah,1; jne` /
/// `test ah,0x41; jp`) sets the flag.
fn is_cop_vehicle_in_area_3d_no_save(s: Scr) {
    collect(s, 6);
    let (x1, x2) = order(sp(0), sp(3));
    let (y1, y2) = order(sp(1), sp(4));
    let (z1, z2) = order(sp(2), sp(5));
    let (x1, x2, y1, y2, z1, z2) = (f(x1), f(x2), f(y1), f(y2), f(z1), f(z2));
    let n = pool(VEHICLE_POOL).byte_add(8).cast::<i32>().read();
    let mut b = false;
    let mut i = 0;
    while i < n {
        let v = vehicle_pool_get_at(pool(VEHICLE_POOL), i as u32);
        i += 1;
        if v.is_null() || is_law_enforcement_vehicle(v) == 0 || v.byte_add(0x22).cast::<u16>().read() == 0x1AE {
            continue;
        }
        let p = entity_pos(v).cast::<f32>();
        let (px, py, pz) = (p.read(), p.byte_add(4).read(), p.byte_add(8).read());
        if px >= x1 && px <= x2 && py >= y1 && py <= y2 && pz >= z1 && pz <= z2 {
            b = true;
        }
    }
    set_flag(s, b);
}

/// `CRunningScript::ProcessCommands2400To2499(int32 command)` (0x478000, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x478000, conv = thiscall, name = "CRunningScript::ProcessCommands2400To2499", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript")]
pub fn process_commands_2400_to_2499(s: Scr, command: u32) -> i8 {
    match command {
        // Cases the jump table sends to the shared `xor al,al` 0x479AD8 (AL = 0, no operand read): 0962
        // DOES_CAR_HAVE_ROOF, 0963 SET_BLIP_FADE, 0979 LOAD_SHARED_CHAR_DECISION_MAKER, 097E
        // IS_POLICE_VEHICLE_IN_PURSUIT, 097F GET_CAR_COLOUR_FROM_MENU_INDEX, 098F RADIANS_TO_DEGREES, 0990
        // DEGREES_TO_RADIANS, 0993 SET_CHAR_AIR_RESISTANCE_MULTIPLIER, 0999 SET_PLAYER_FIRE_WITH_SHOULDER_BUTTON,
        // 09A5 SET_CCTV_EFFECT, 09B1 GET_CPU_LEVEL. gr: 0979/0993 `unimplemented` (-1), the exe returns 0.
        0x0962 | 0x0963 | 0x0979 | 0x097E | 0x097F | 0x098F => {}
        0x0990 | 0x0993 | 0x0999 | 0x09A5 | 0x09B1 => {}
        // 0960 SET_PLAYER_DISPLAY_VITAL_STATS_BUTTON (0x47803E): `CPad::GetPad(SP0)` byte +0x11F = SP1 == 0
        0x0960 => {
            collect(s, 2);
            let on = sp(1) != 0;
            get_pad(sp(0)).byte_add(0x11F).write(u8::from(!on));
        }
        // 0961 SET_CHAR_KEEP_TASK (0x478089): `PED(SP0)` flags +0x478 bit 4 = SP1 != 0 (no null check)
        0x0961 => {
            collect(s, 2);
            set_ped_flag_bit(ped(sp(0)), 4, u32::from(sp(1) != 0));
        }
        // 0964 CREATE_MENU_GRID (0x4780D1)
        0x0964 => create_menu_grid(s),
        // 0965 IS_CHAR_SWIMMING (0x47819F): flag = `PED(SP0)->m_pIntelligence->GetTaskSwim() != null`
        0x0965 => {
            collect(s, 1);
            let t = get_task_swim(ped(sp(0)).byte_add(0x47C).cast::<GPtr<u8>>().read());
            set_flag(s, !t.is_null());
        }
        // 0966 GET_CHAR_SWIM_STATE (0x4781E1): SP0 = (i16) `GetTaskSwim()` +0xA (no null check)
        0x0966 => {
            collect(s, 1);
            let t = get_task_swim(ped(sp(0)).byte_add(0x47C).cast::<GPtr<u8>>().read());
            set_sp(0, i32::from(t.byte_add(0xA).cast::<i16>().read()) as u32);
            store(s, 1);
        }
        // 0967 START_CHAR_FACIAL_TALK (0x478221): `m_TaskMgr.GetTaskSecondary(3)->SetRequest(7, SP1, -1, 0)`
        0x0967 => {
            collect(s, 2);
            let tm = ped(sp(0)).byte_add(0x47C).cast::<GPtr<u8>>().read().byte_add(4);
            let t = get_task_secondary(tm, 3);
            facial_set_request(t, 7, sp(1), u32::MAX, 0);
        }
        // 0968 STOP_CHAR_FACIAL_TALK (0x478266): `GetTaskSecondary(3)->StopAll()`
        0x0968 => {
            collect(s, 1);
            let tm = ped(sp(0)).byte_add(0x47C).cast::<GPtr<u8>>().read().byte_add(4);
            facial_stop_all(get_task_secondary(tm, 3));
        }
        // 0969 IS_BIG_VEHICLE (0x47829F): pushes `((u8)VEH(SP0)->+0x429 >> 2) & 0xFFFFFF01` (exact dword)
        0x0969 => {
            collect(s, 1);
            let b = u32::from(veh(sp(0)).byte_add(0x429).read() >> 2);
            flag(s, b & 0xFFFF_FF01);
        }
        // 096A SWITCH_POLICE_HELIS (0x4782D9)
        0x096A => {
            collect(s, 1);
            switch_police_helis(u32::from(sp(0) != 0));
        }
        // 096B STORE_CAR_MOD_STATE (0x4782FD) / 096C RESTORE_CAR_MOD_STATE (0x478309)
        0x096B => store_vehicle_mods(),
        0x096C => restore_vehicle_mods(),
        // 096D GET_CURRENT_CAR_MOD (0x478315)
        0x096D => get_current_car_mod(s),
        // 096E IS_CAR_LOW_RIDER (0x4784E6) / 096F IS_CAR_STREET_RACER (0x478534): pushes `(handling[(u8)mi->+0x4A]
        // .m_nModelFlags >> 0x19 / 0x1A) & 0xFFFFFF01` (exact dword; no null check)
        0x096E | 0x096F => {
            collect(s, 1);
            let mi = model_info_of(veh(sp(0)));
            let h = u32::from(mi.byte_add(0x4A).read());
            let flags = GPtr::<u32>::from_addr(HANDLING_MODEL_FLAGS.wrapping_add(h * 0xE0) as usize).read();
            let sh = if command == 0x096E { 0x19 } else { 0x1A };
            flag(s, (flags >> sh) & 0xFFFF_FF01);
        }
        // 0970 FORCE_DEATH_RESTART (0x478581) / 0971 SYNC_WATER (0x47858D, 0x6E76E0) / 0974
        // RESET_STUFF_UPON_RESURRECTION (0x478605)
        0x0970 => force_death_restart(),
        0x0971 => sync_water_6e76e0(),
        0x0974 => reset_stuff_upon_resurrection(),
        // 0972 SET_CHAR_COORDINATES_NO_OFFSET (0x478599): `SetCharCoordinates(PED(SP0), SP1, SP2, SP3, true, false)`
        // (tail 0x47979C shared with 09BC)
        0x0972 => {
            collect(s, 4);
            let p = ped(sp(0));
            set_char_coordinates(s, p, sp(1), sp(2), sp(3), 1, 0);
        }
        // 0973 DOES_SCRIPT_FIRE_EXIST (0x4785C6): flag = `0 <= GetActualScriptThingIndex(SP0, 5) < 60`
        0x0973 => {
            collect(s, 1);
            let i = get_actual_script_thing_index(sp(0), 5);
            set_flag(s, (0..0x3C).contains(&i));
        }
        // 0975 IS_EMERGENCY_SERVICES_VEHICLE (0x478611): flag = `IsLawEnforcementVehicle()` or model 0x1A0 / 0x197 /
        // 0x220 (no null check)
        0x0975 => {
            collect(s, 1);
            let v = veh(sp(0));
            let b = is_law_enforcement_vehicle(v) != 0 || {
                let m = i32::from(v.byte_add(0x22).cast::<i16>().read());
                m == 0x1A0 || m == 0x197 || m == 0x220
            };
            set_flag(s, b);
        }
        // 0976 KILL_FX_SYSTEM_NOW (0x47866F)
        0x0976 => kill_fx_system_now(s),
        // 0977 IS_OBJECT_WITHIN_BRAIN_ACTIVATION_RANGE (0x4786D6)
        0x0977 => is_object_within_brain_activation_range_case(s),
        // 0978 COPY_SHARED_CHAR_DECISION_MAKER (0x47875E)
        0x0978 => copy_shared_char_decision_maker(s),
        // 097A REPORT_MISSION_AUDIO_EVENT_AT_POSITION (0x478805): `AudioEngine.ReportMissionAudioEvent((u16)SP3,
        // &(SP0, SP1, SP2))`
        0x097A => {
            collect(s, 4);
            let mut pos = [sp(0), sp(1), sp(2)];
            report_mission_audio_event_pos(GPtr::from_addr(AUDIO_ENGINE), sp(3) & 0xFFFF, GPtr::from_raw(pos.as_mut_ptr()));
        }
        // 097B REPORT_MISSION_AUDIO_EVENT_AT_OBJECT (0x47884B): `ReportMissionAudioEvent((u16)SP1, OBJ(SP0))`
        0x097B => {
            collect(s, 2);
            let o = obj(sp(0));
            report_mission_audio_event_obj(GPtr::from_addr(AUDIO_ENGINE), sp(1) & 0xFFFF, o);
        }
        // 097C ATTACH_MISSION_AUDIO_TO_OBJECT (0x478881): `AttachMissionAudioToObject((u8)(SP0 - 1), OBJ(SP1))`
        0x097C => {
            collect(s, 2);
            let o = obj(sp(1));
            attach_mission_audio_to_object(GPtr::from_addr(AUDIO_ENGINE), sp(0).wrapping_sub(1) & 0xFF, o);
        }
        // 097D GET_NUM_CAR_COLOURS (0x4788B8): SP0 = byte `ms_modelInfoPtrs[VEH(SP0) model]` +0x2D0
        0x097D => {
            collect(s, 1);
            let mi = model_info_of(veh(sp(0)));
            set_sp(0, u32::from(mi.byte_add(0x2D0).read()));
            store(s, 1);
        }
        // 0980 EXTINGUISH_FIRE_AT_POINT (0x4788FB): `gFireManager.ExtinguishPoint(CVector(SP0, SP1, SP2), SP3)`
        0x0980 => {
            collect(s, 4);
            extinguish_point(GPtr::from_addr(FIRE_MANAGER), sp(0), sp(1), sp(2), sp(3));
        }
        // 0981 HAS_TRAIN_DERAILED (0x478952): pushes `(u8)VEH(SP0)->+0x5B9 & 0xFFFFFF01` (exact dword)
        0x0981 => {
            collect(s, 1);
            flag(s, u32::from(veh(sp(0)).byte_add(0x5B9).read()) & 0xFFFF_FF01);
        }
        // 0982 SET_CHAR_FORCE_DIE_IN_CAR (0x478988): `PED(SP0)` flags +0x478 bit 7 = bit 0 of SP1 (`shl 7`, the raw
        // operand; no null check)
        0x0982 => {
            collect(s, 2);
            set_ped_flag_bit(ped(sp(0)), 7, sp(1));
        }
        // 0983 SET_ONLY_CREATE_GANG_MEMBERS (0x4789BF) / 099E SET_CREATE_RANDOM_COPS (0x478FB2): byte [0xC0FCB3] /
        // [0xC0FCB4] = SP0 == 0
        0x0983 | 0x099E => {
            collect(s, 1);
            let a = if command == 0x0983 { 0xC0FCB3 } else { 0xC0FCB4 };
            GPtr::<u8>::from_addr(a).write(u8::from(sp(0) == 0));
        }
        // 0984 GET_OBJECT_MODEL (0x4789DE): SP0 = (i16) `OBJ(SP0)` +0x22
        0x0984 => {
            collect(s, 1);
            set_sp(0, i32::from(obj(sp(0)).byte_add(0x22).cast::<i16>().read()) as u32);
            store(s, 1);
        }
        // 0985 SET_CHAR_USES_COLLISION_CLOSEST_OBJECT_OF_TYPE (0x478A13)
        0x0985 => set_char_uses_collision_closest_object_of_type(s),
        // 0986 CLEAR_ALL_SCRIPT_FIRE_FLAGS (0x478BB7)
        0x0986 => clear_all_script_fire_flags(GPtr::from_addr(FIRE_MANAGER)),
        // 0987 GET_CAR_BLOCKING_CAR (0x478BC8): `b = VEH(SP0)->+0x420`; SP0 = b and `(b->+0x36 & 7) == 2` (a vehicle)
        // ? `GetRef(b)` : -1 (the pool pointer read once)
        0x0987 => {
            collect(s, 1);
            let vp = pool(VEHICLE_POOL);
            let b = vehicle_at_ref(vp, spi(0)).byte_add(0x420).cast::<GPtr<u8>>().read();
            let mut h = u32::MAX;
            if !b.is_null() && b.byte_add(0x36).read() & 7 == 2 {
                h = vehicle_get_ref(vp, b);
            }
            set_sp(0, h);
            store(s, 1);
        }
        // 0988 GET_CURRENT_VEHICLE_PAINTJOB (0x478C1C): SP0 = `VEH(SP0)->GetRemapIndex()`
        0x0988 => {
            collect(s, 1);
            set_sp(0, get_remap_index(veh(sp(0))));
            store(s, 1);
        }
        // 0989 SET_HELP_MESSAGE_BOX_SIZE (0x478C53): float [0x8D0934] = (float)SP0 (`fild; fstp`)
        0x0989 => {
            collect(s, 1);
            GPtr::<f32>::from_addr(0x8D0934).write(X87::from_i32(spi(0)).to_f32());
        }
        // 098A SET_GUNSHOT_SENSE_RANGE_FOR_RIOT2 (0x478C6F): dword [0x8A625C] = SP0
        0x098A => {
            collect(s, 1);
            GPtr::<u32>::from_addr(0x8A625C).write(sp(0));
        }
        // 098B STRING_CAT16 / 098C STRING_CAT8 (0x478C89)
        0x098B | 0x098C => string_cat(s, command),
        // 098D GET_CAR_MOVING_COMPONENT_OFFSET (0x478D2F): SP0 = `VEH(SP0)->GetMovingCollisionOffset()` (`fstp` f32)
        0x098D => {
            collect(s, 1);
            let r = get_moving_collision_offset(veh(sp(0)));
            set_sp(0, X87::from_f64_exact(r).to_f32().to_bits());
            store(s, 1);
        }
        // 098E SET_NAMED_ENTRY_EXIT_FLAG (0x478D67): label, `CollectParameters(2)`, `SetEntryExitFlag(label, SP0,
        // SP1 != 0)`
        0x098E => {
            let mut label = [0u8; 16];
            let lp = GPtr::from_raw(label.as_mut_ptr());
            read_text_label(s, lp, 8);
            collect(s, 2);
            set_entry_exit_flag(lp, sp(0), u32::from(sp(1) != 0));
        }
        // 0991 PAUSE_CURRENT_BEAT_TRACK (0x478DA5): `AudioEngine.PauseBeatTrack((u8)SP0)`
        0x0991 => {
            collect(s, 1);
            pause_beat_track(GPtr::from_addr(AUDIO_ENGINE), sp(0) & 0xFF);
        }
        // 0992 SET_PLAYER_CYCLE_WEAPON_BUTTON (0x478DC7): `CPad::GetPad(SP0)` byte +0x11D = SP1 == 0
        0x0992 => {
            collect(s, 2);
            let on = sp(1) != 0;
            get_pad(sp(0)).byte_add(0x11D).write(u8::from(!on));
        }
        // 0994 MARK_ROAD_NODE_AS_DONT_WANDER (0x478E13) / 0995 UNMARK_ALL_ROAD_NODES_AS_DONT_WANDER (0x478E41)
        0x0994 => {
            collect(s, 3);
            mark_road_node_as_dont_wander(GPtr::from_addr(THE_PATHS), sp(0), sp(1), sp(2));
        }
        0x0995 => unmark_all_road_nodes_as_dont_wander(GPtr::from_addr(THE_PATHS)),
        // 0996 SET_CHECKPOINT_HEADING (0x478E52): `i = GetActualScriptThingIndex(SP0, 3)`; i >= 0 and `cp = [0xA44074
        // + i * 8]` non-null: `CCheckpoints::SetHeading(cp->+4, SP1)`
        0x0996 => {
            collect(s, 2);
            let h = sp(1);
            let i = get_actual_script_thing_index(sp(0), 3);
            if i >= 0 {
                let cp = GPtr::<GPtr<u8>>::from_addr(SCRIPT_CHECKPOINTS.wrapping_add(i as u32 * 8) as usize).read();
                if !cp.is_null() {
                    checkpoints_set_heading(cp.byte_add(4).cast::<u32>().read(), h);
                }
            }
        }
        // 0997 SET_MISSION_RESPECT_TOTAL (0x478EA4): `SetStatValue(0xE4, (float)SP0)` / 0998
        // AWARD_PLAYER_MISSION_RESPECT (0x478ECB): `IncrementStat(0xE0, (float)SP0)` (`fild; fstp`)
        0x0997 => {
            collect(s, 1);
            set_stat_value(0xE4, X87::from_i32(spi(0)).to_f32());
        }
        0x0998 => {
            collect(s, 1);
            increment_stat(0xE0, X87::from_i32(spi(0)).to_f32());
        }
        // 099A SET_CAR_COLLISION (0x478EF2): `VEH(SP0)` dword +0x1C bit 0 and dword +0x40 bit 1 = SP1 != 0
        0x099A => {
            collect(s, 2);
            let v = veh(sp(0));
            let (a, b) = (v.byte_add(0x1C).cast::<u32>(), v.byte_add(0x40).cast::<u32>());
            let (fa, fb) = (a.read(), b.read());
            if sp(1) != 0 {
                a.write(fa | 1);
                b.write(fb | 2);
            } else {
                a.write(fa & !1);
                b.write(fb & !2);
            }
        }
        // 099B CHANGE_PLAYBACK_TO_USE_AI (0x478F43)
        0x099B => {
            collect(s, 1);
            change_car_playback_to_use_ai(veh(sp(0)));
        }
        // 099C CAMERA_SET_SHAKE_SIMULATION_SIMPLE (0x478F6E): `TheCamera.AddShakeSimple(SP1, SP0, SP2)` (the exe's
        // argument order)
        0x099C => {
            collect(s, 3);
            add_shake_simple(GPtr::from_addr(THE_CAMERA), sp(1), sp(0), sp(2));
        }
        // 099D IS_NIGHT_VISION_ACTIVE (0x478F9C): flag = byte [0xC402B8] (zero-extended)
        0x099D => flag(s, u32::from(GPtr::<u8>::from_addr(0xC402B8).read())),
        // 099F TASK_SET_IGNORE_WEAPON_RANGE_FLAG (0x478FD2)
        0x099F => task_set_ignore_weapon_range_flag(s, command),
        // 09A0 TASK_PICK_UP_SECOND_OBJECT (0x47905C): `ScriptTaskPickUpObject(command)`
        0x09A0 => script_task_pick_up_object(s, command),
        // 09A1 DROP_SECOND_OBJECT (0x47906B)
        0x09A1 => drop_second_object(s),
        // 09A2 REMOVE_OBJECT_ELEGANTLY (0x479107): `obj = OBJ(SP0)`; `CleanUpThisObject(obj)` (also when null); if obj:
        // `obj->m_pRwObject (+0x18)` -> `RpClumpForAllAtomics(rw, SetAtomicRendererCB, RenderFadingClumpCB)` (no type
        // check), then `+0x140 |= 0x400000`; with mission cleanup `RemoveEntityFromList(SP0, 3)`.
        0x09A2 => {
            collect(s, 1);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            clean_up_this_object(o);
            if !o.is_null() {
                let rw = o.byte_add(0x18).cast::<GPtr<u8>>().read();
                if !rw.is_null() {
                    rp_clump_for_all_atomics(rw, SET_ATOMIC_RENDERER_CB, RENDER_FADING_CLUMP_CB);
                }
                let fl = o.byte_add(0x140).cast::<u32>();
                fl.write(fl.read() | 0x40_0000);
            }
            if uses_mission_cleanup(s) {
                mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), spi(0), 3);
            }
        }
        // 09A3 DRAW_CROSSHAIR (0x47917C): dword [0xA44490] = SP0 != 0
        0x09A3 => {
            collect(s, 1);
            GPtr::<u32>::from_addr(0xA44490).write(u32::from(sp(0) != 0));
        }
        // 09A4 SET_UP_CONVERSATION_NODE_WITH_SPEECH (0x47919E): labels A, B, C; `CollectParameters(3)`;
        // `SetUpConversationNode(A, B, C, SP0, SP1, SP2)`
        0x09A4 => {
            let mut a = [0u8; 16];
            let mut b = [0u8; 16];
            let mut c = [0u8; 16];
            let (ap, bp, cp) = (GPtr::from_raw(a.as_mut_ptr()), GPtr::from_raw(b.as_mut_ptr()), GPtr::from_raw(c.as_mut_ptr()));
            read_text_label(s, ap, 8);
            read_text_label(s, bp, 8);
            read_text_label(s, cp, 8);
            collect(s, 3);
            set_up_conversation_node(ap, bp, cp, sp(0), sp(1), sp(2));
        }
        // 09A6 SHOW_BLIPS_ON_ALL_LEVELS (0x479203) / 09AC HIDE_ALL_FRONTEND_BLIPS (0x47936D): byte [0xA444A2] /
        // [0xA444A1] = SP0 != 0
        0x09A6 | 0x09AC => {
            collect(s, 1);
            let a = if command == 0x09A6 { 0xA444A2 } else { 0xA444A1 };
            GPtr::<u8>::from_addr(a).write(u8::from(sp(0) != 0));
        }
        // 09A7 SET_CHAR_DRUGGED_UP (0x479222): `PED(SP0)` flags +0x478 bit 10 = SP1 != 0
        0x09A7 => {
            collect(s, 2);
            set_ped_flag_bit(ped(sp(0)), 10, u32::from(sp(1) != 0));
        }
        // 09A8 IS_CHAR_HEAD_MISSING (0x47926D): flag = `p = PED(SP0)` non-null, dword +0x46C bit 15, byte +0x754 == 2
        0x09A8 => {
            collect(s, 1);
            let p = ped(sp(0));
            let b = !p.is_null()
                && p.byte_add(0x46C).cast::<u32>().read() & 0x8000 != 0
                && p.byte_add(0x754).read() == 2;
            set_flag(s, b);
        }
        // 09A9 GET_HASH_KEY (0x4792AF): 15-byte label (zeroed buffer); SP0 = `CKeyGen::GetUppercaseKey(label)`
        0x09A9 => {
            let mut label = [0u8; 24];
            let lp = GPtr::from_raw(label.as_mut_ptr());
            read_text_label(s, lp, 15);
            set_sp(0, get_uppercase_key(lp));
            store(s, 1);
        }
        // 09AA SET_UP_CONVERSATION_END_NODE_WITH_SPEECH (0x4792E5): label, `CollectParameters(1)`;
        // `SetUpConversationNode(label, null, null, SP0, 0, 0)`
        0x09AA => {
            let mut label = [0u8; 16];
            let lp = GPtr::from_raw(label.as_mut_ptr());
            read_text_label(s, lp, 8);
            collect(s, 1);
            set_up_conversation_node(lp, GPtr::NULL, GPtr::NULL, sp(0), 0, 0);
        }
        // 09AB RANDOM_PASSENGER_SAY (0x47931F): `p = VEH(SP0)->PickRandomPassenger()`; p: `p->Say((u16)SP1, 0, 1.0f,
        // 0, 0, 0)`
        0x09AB => {
            collect(s, 2);
            let p = pick_random_passenger(veh(sp(0)));
            if !p.is_null() {
                ped_say(p, sp(1) & 0xFFFF, 0, 0x3F80_0000, 0, 0, 0);
            }
        }
        // 09AD SET_PLAYER_IN_CAR_CAMERA_MODE (0x47938D): SP0 != 4: dword [0xB6F0DC] = SP0
        0x09AD => {
            collect(s, 1);
            if sp(0) != 4 {
                GPtr::<u32>::from_addr(0xB6F0DC).write(sp(0));
            }
        }
        // 09AE IS_CHAR_IN_ANY_TRAIN (0x4793B0): flag = `PED(SP0)` dword +0x46C bit 8 and `m_pVehicle (+0x58C)` dword
        // +0x590 == 6 (no null checks)
        0x09AE => {
            collect(s, 1);
            let p = ped(sp(0));
            let b = p.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0
                && p.byte_add(0x58C).cast::<GPtr<u8>>().read().byte_add(0x590).cast::<u32>().read() == 6;
            set_flag(s, b);
        }
        // 09AF SET_UP_SKIP_AFTER_MISSION (0x479401): `CGameLogic::SetUpSkip(CVector(SP0, SP1, SP2), SP3, true, null,
        // false)`
        0x09AF => {
            collect(s, 4);
            set_up_skip(sp(0), sp(1), sp(2), sp(3), 1, 0, 0);
        }
        // 09B0 SET_VEHICLE_IS_CONSIDERED_BY_PLAYER (0x47945B): `VEH(SP0)` byte +0x42E bit 5 = SP1 != 0
        0x09B0 => {
            collect(s, 2);
            set_byte_bit(veh(sp(0)).byte_add(0x42E), 0x20, sp(1) != 0);
        }
        // 09B2 GET_RANDOM_CAR_MODEL_IN_MEMORY (0x47949F)
        0x09B2 => get_random_car_model_in_memory(s),
        // 09B3 GET_CAR_DOOR_LOCK_STATUS (0x479500): SP0 = `VEH(SP0)` dword +0x4F8
        0x09B3 => {
            collect(s, 1);
            set_sp(0, veh(sp(0)).byte_add(0x4F8).cast::<u32>().read());
            store(s, 1);
        }
        // 09B4 SET_CLOSEST_ENTRY_EXIT_FLAG (0x479536)
        0x09B4 => set_closest_entry_exit_flag(s),
        // 09B5 SET_CHAR_SIGNAL_AFTER_KILL (0x47958C) / 09B6 SET_CHAR_WANTED_BY_POLICE (0x4795C7): `PED(SP0)` flags
        // +0x478 bit 12 / bit 11 = SP1 != 0
        0x09B5 | 0x09B6 => {
            collect(s, 2);
            let k = if command == 0x09B5 { 12 } else { 11 };
            set_ped_flag_bit(ped(sp(0)), k, u32::from(sp(1) != 0));
        }
        // 09B7 SET_ZONE_NO_COPS (0x47960D): label, `CollectParameters(1)`; `z = (i16)FindZoneByLabel(label, 2)`; z >=
        // 0: `ZoneInfoArray[NavigationZoneArray[z].zoneInfoIndex]` byte +0x10 bit 7 = bit 0 of SP0 (`shl dl,7`)
        0x09B7 => {
            let mut label = [0u8; 16];
            let lp = GPtr::from_raw(label.as_mut_ptr());
            read_text_label(s, lp, 8);
            collect(s, 1);
            let z = find_zone_by_label(lp, 2) as i16;
            if z >= 0 {
                let zi = GPtr::<u16>::from_addr(NAV_ZONE_INFO_INDEX.wrapping_add((z as u32) << 5) as usize).read();
                let b = GPtr::<u8>::from_addr(ZONE_INFO_FLAGS.wrapping_add(u32::from(zi) * 0x11) as usize);
                b.write((b.read() & 0x7F) | ((sp(0) as u8) << 7));
            }
        }
        // 09B8 ADD_BLOOD (0x47966F)
        0x09B8 => add_blood(s),
        // 09B9 DISPLAY_CAR_NAMES (0x4796EC) / 09BA DISPLAY_ZONE_NAMES (0x47970C): `CHud::bScriptDontDisplayVehicleName`
        // / `bScriptDontDisplayAreaName` = SP0 == 0
        0x09B9 | 0x09BA => {
            collect(s, 1);
            let a = if command == 0x09B9 { SCRIPT_DONT_DISPLAY_VEHICLE_NAME } else { SCRIPT_DONT_DISPLAY_AREA_NAME };
            GPtr::<u8>::from_addr(a).write(u8::from(sp(0) == 0));
        }
        // 09BB IS_CAR_DOOR_DAMAGED (0x47972B): flag = `VEH(SP0)->m_damageManager (+0x5A0).GetDoorStatus(SP1) != 0`
        0x09BB => {
            collect(s, 2);
            let v = veh(sp(0));
            set_flag(s, get_door_status(v.byte_add(0x5A0), sp(1)) != 0);
        }
        // 09BC SET_CHAR_COORDINATES_DONT_WARP_GANG_NO_OFFSET (0x479774): `SetCharCoordinates(PED(SP0), SP1, SP2, SP3,
        // false, false)`
        0x09BC => {
            collect(s, 4);
            let p = ped(sp(0));
            set_char_coordinates(s, p, sp(1), sp(2), sp(3), 0, 0);
        }
        // 09BD SET_MINIGAME_IN_PROGRESS (0x4797B6): `this->+0xC8`, byte [0xA444A8] = SP0 != 0, byte [0xA444A7] = SP0 == 0
        0x09BD => {
            collect(s, 1);
            let on = sp(0) != 0;
            s.byte_add(0xC8).write(u8::from(on));
            GPtr::<u8>::from_addr(0xA444A8).write(u8::from(on));
            GPtr::<u8>::from_addr(0xA444A7).write(u8::from(!on));
        }
        // 09BE IS_MINIGAME_IN_PROGRESS (0x479800): flag = byte [0xA444A8] (zero-extended)
        0x09BE => flag(s, u32::from(GPtr::<u8>::from_addr(0xA444A8).read())),
        // 09BF SET_FORCE_RANDOM_CAR_MODEL (0x479817): dword [0xA4448C] = SP0
        0x09BF => {
            collect(s, 1);
            GPtr::<u32>::from_addr(0xA4448C).write(sp(0));
        }
        // 09C0 GET_RANDOM_CAR_OF_TYPE_IN_ANGLED_AREA_NO_SAVE (0x479831)
        0x09C0 => get_random_car_of_type_in_angled_area_no_save(s),
        // 09C1 ADD_NEXT_MESSAGE_TO_PREVIOUS_BRIEFS (0x479931): byte [0xA4448B] = (u8)SP0
        0x09C1 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0xA4448B).write(sp(0) as u8);
        }
        // 09C2 FAIL_KILL_FRENZY (0x47994B)
        0x09C2 => fail_kill_frenzy(),
        // 09C3 IS_COP_VEHICLE_IN_AREA_3D_NO_SAVE (0x479957)
        0x09C3 => is_cop_vehicle_in_area_3d_no_save(s),
        _ => return -1,
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_2400_TO_2499];
