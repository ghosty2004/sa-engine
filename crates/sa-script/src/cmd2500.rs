//! `CRunningScript::ProcessCommands2500To2599` (0x47A760): partial port (PORT-SCRIPT-2, ADR-002; all cases but two
//! ported in PORT-SCRIPT-4). Opcodes in [`PORTED`] run in Rust; every other opcode (and the default, -1) runs the
//! original body through the fallback. Cases follow the 1.0 US bytes and
//! `docs/re/specs/SCRIPT/commands/ProcessCommands2500To2599.md`.
//!
//! The switch stays partial on purpose: SA:MP writes its own opcodes 09C6 / 09D3 into this switch's jump table
//! (0x47BF54 / 0x47BF88, `docs/samp/game-adapter.md`). In the unmodified exe both entries point at the empty case
//! 0x47BF2B, but only the original body dispatches through the (patched) table, so these two opcodes stay on the
//! fallback; every other jump-table case (95 of 97) is Rust. 09CD / 09CE / 09DF reach the default (-1) through the
//! table and are not cases.
//!
//! The dispatcher zeroes EBP and EBX before the jump (0x47A78D/0x47A78F); cases that use them as constant 0 (or,
//! 0A1B, EBX as the "no vehicle" initial value) are ported with literal zeros. ECX still holds the command when a
//! case starts (0A1A pushes it).
//!
//! Divergence notes (ledger classes of PORT-SCRIPT-2/3): the 8-byte labels (09FD, 0A08, 0A18, 0A19) and the 9-byte
//! plate of 09E2 are read into zeroed buffers; `setcc` bools pushed with stale upper bytes are passed as 0/1; the
//! 09C9 `CTaskSimpleCarSetPedOut` is built on a zeroed Rust buffer, 09DA's vector temporaries and 0A08's string
//! buffer start zeroed (the callees write them); task constructions (09EB, 0A1D) use `CTask::operator new` + the
//! constructor (the SEH state stores are not ported).

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x09C4, 0x09C5, 0x09C7, 0x09C8, 0x09C9, 0x09CA, 0x09CB, 0x09CC, 0x09CF, 0x09D0, 0x09D1, 0x09D2, 0x09D4,
    0x09D5, 0x09D6, 0x09D7, 0x09D8, 0x09D9, 0x09DA, 0x09DB, 0x09DC, 0x09DD, 0x09DE, 0x09E0, 0x09E1, 0x09E2,
    0x09E3, 0x09E4, 0x09E5, 0x09E6, 0x09E7, 0x09E8, 0x09E9, 0x09EA, 0x09EB, 0x09EC, 0x09ED, 0x09EE, 0x09EF,
    0x09F0, 0x09F1, 0x09F2, 0x09F3, 0x09F4, 0x09F5, 0x09F6, 0x09F7, 0x09F8, 0x09F9, 0x09FA, 0x09FB, 0x09FC,
    0x09FD, 0x09FE, 0x09FF, 0x0A00, 0x0A01, 0x0A02, 0x0A03, 0x0A04, 0x0A05, 0x0A06, 0x0A07, 0x0A08, 0x0A09,
    0x0A0A, 0x0A0B, 0x0A0C, 0x0A0D, 0x0A0E, 0x0A0F, 0x0A10, 0x0A11, 0x0A12, 0x0A13, 0x0A14, 0x0A15, 0x0A16,
    0x0A17, 0x0A18, 0x0A19, 0x0A1A, 0x0A1B, 0x0A1C, 0x0A1D, 0x0A1E, 0x0A1F, 0x0A20, 0x0A21, 0x0A22, 0x0A23,
    0x0A24, 0x0A25, 0x0A26, 0x0A27,
];

/// `.rdata` 0x859520: 0.0015625f (1/640)
const K_ONE_640TH: u32 = 0x3ACC_CCCD;
/// `.rdata` 0x858B8C: 0.5f; 0x859F00: 0.015f; 0x859A3C: 1/255 (0x3B808081); 0x8595EC: pi/180
const K_HALF: u32 = 0x3F00_0000;
const K_0_015: u32 = 0x3C75_C28F;
const K_1_255: u32 = 0x3B80_8081;
const K_DEG_TO_RAD: u32 = 0x3C8E_FA35;
/// `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8; `mod_HandlingManager.m_aVehicleHandling[0]` dword +0xD4 (0xC2BAB4,
/// 0xE0-byte entries): the monetary value.
const MODEL_INFO_PTRS: u32 = 0xA9B0C8;
const HANDLING_VALUE: u32 = 0xC2BAB4;
/// `CPickups::aPickUps[0]` (0x20 bytes, 620 entries) 0x9788C0; byte +0x1C = the pickup type (8 = money).
const PICKUPS: u32 = 0x9788C0;
/// `TheText` 0xC1B340; `AudioEngine` 0xB6BC90; `FrontEndMenuManager` 0xBA6748; `CSpecialPlateHandler` 0xC279D8.
const THE_TEXT: usize = 0xC1B340;
const AUDIO_ENGINE: usize = 0xB6BC90;
const FRONT_END_MENU_MANAGER: usize = 0xBA6748;
const SPECIAL_PLATE_HANDLER: usize = 0xC279D8;
/// `CTheCarGenerators::CarGeneratorArray[0]` (0x20 bytes) 0xC27AD0.
const CAR_GENERATORS: u32 = 0xC27AD0;
/// `CTheScripts::ScriptSearchLights[0]` (0x7C bytes) byte +2 (0xA94D6A).
const SEARCHLIGHT_ON_GROUND: u32 = 0xA94D6A;
/// `CPedGroups::ms_groups[0].m_groupMembership` (0x2D4-byte groups) 0xC09928.
const GROUP_MEMBERSHIP: u32 = 0xC09928;
/// `CDecisionMakerTypes` "used" bytes 0xC0B01C (20 entries).
const DECISION_MAKER_USED: u32 = 0xC0B01C;
/// `TheCamera.ActiveCam` (byte) 0xB6F081; `TheCamera.Cams[0]` (`CCam`, 0x238 bytes) 0xB6F19C.
const ACTIVE_CAM: usize = 0xB6F081;
const CAMS: u32 = 0xB6F19C;

orig! {
    /// `CMenuSystem::SetColumnWidth(uint8 menu, uint8 column, uint16 width)` (0x582050)
    fn set_column_width(menu: u32, column: u32, width: u32) = cdecl 0x582050 => unported;
    /// `GetPedAttractorManager()` (0x5EE190)
    fn get_ped_attractor_manager() -> GPtr<u8> = cdecl 0x5EE190 => unported;
    /// `CPedAttractorManager::IsPedRegisteredWithEffect(CPed*)` (0x5EBCB0), AL
    fn is_ped_registered_with_effect(m: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x5EBCB0 => unported;
    /// `CEntity` virtual slot 8 (+0x20, `DeleteRwObject()`) / slot 5 (+0x14, `SetModelIndex(uint32)`)
    fn entity_delete_rw_object(e: GPtr<u8>) = thiscall vslot 0x20 => unported;
    fn entity_set_model_index(e: GPtr<u8>, m: u32) = thiscall vslot 0x14 => unported;
    /// `CCarEnterExit::ComputeTargetDoorToExit(const CVehicle*, const CPed*)` (0x64F110)
    fn compute_target_door_to_exit(v: GPtr<u8>, p: GPtr<u8>) -> u32 = cdecl 0x64F110 => unported;
    /// `CTaskSimpleCarSetPedOut::CTaskSimpleCarSetPedOut(CVehicle*, int32 door, bool)` (0x6478B0)
    fn task_car_set_ped_out_ctor(t: GPtr<u8>, v: GPtr<u8>, door: u32, b: u32) -> GPtr<u8> = thiscall 0x6478B0 => unported;
    /// `CTaskSimpleCarSetPedOut::ProcessPed(CPed*)` (0x647D10), AL (discarded)
    fn task_car_set_ped_out_process_ped(t: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x647D10 => unported;
    /// The `CTaskSimpleCarSetPedOut` destructor at 0x647950 (unnamed in the catalogue)
    fn task_car_set_ped_out_dtor_647950(t: GPtr<u8>) = thiscall 0x647950 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0)
    fn set_posn_v(e: GPtr<u8>, pos: GPtr<u32>) = thiscall 0x4241C0 => unported;
    /// `CPhysical::GetHasCollidedWith(CEntity*)` (0x543540), AL
    fn get_has_collided_with(p: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x543540 => unported;
    /// `CPickups::GetActualPickupIndex(int32)` (0x4552A0)
    fn get_actual_pickup_index(h: u32) -> i32 = cdecl 0x4552A0 => unported;
    /// `CWorld::FindPlayerWanted(int32)` (0x56E230)
    fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CWanted::ClearWantedLevelAndGoOnParole()` (0x5625A0)
    fn clear_wanted_level_and_go_on_parole(w: GPtr<u8>) = thiscall 0x5625A0 => unported;
    /// `CPed::Say(uint16, uint32, float, uint8, uint8, uint8)` (0x5EFFE0), AX; the float raw
    fn ped_say(p: GPtr<u8>, id: u32, a: u32, vol: u32, b: u32, c: u32, d: u32) -> u16 = thiscall 0x5EFFE0 => unported;
    /// `CPed::SayScript(int32, uint8, uint8, uint8)` (0x5EFFB0)
    fn ped_say_script(p: GPtr<u8>, id: u32, a: u32, b: u32, c: u32) = thiscall 0x5EFFB0 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWorld::UseDetonator(CPed*)` (0x5660B0)
    fn use_detonator(p: GPtr<u8>) = cdecl 0x5660B0 => unported;
    /// `CPickup::GetPosn()` (0x4549A0), the vector returned through the hidden pointer
    fn pickup_get_posn(p: GPtr<u8>, out: GPtr<u32>) -> GPtr<u32> = thiscall 0x4549A0 => unported;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, `VectorSub`), the result through the hidden pointer
    fn vector_sub(out: GPtr<u32>, a: GPtr<u32>, b: GPtr<u32>) -> GPtr<u32> = cdecl 0x40FE60 => unported;
    /// `CVector::Magnitude()` (0x4082C0), ST0 (an `fsqrt` result under PC=24: exact as f64)
    fn vector_magnitude(v: GPtr<u32>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CPedGroupMembership::CountMembersExcludingLeader()` (0x5F6AA0)
    fn count_members_excluding_leader(m: GPtr<u8>) -> i32 = thiscall 0x5F6AA0 => unported;
    /// `CPedGroupMembership::RemoveNFollowers(int32)` (0x5FB1D0)
    fn remove_n_followers(m: GPtr<u8>, n: i32) = thiscall 0x5FB1D0 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740)
    fn find_active_task_by_type(tm: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CGameLogic::SetUpSkip(CVector, float, bool, CVehicle*, bool)` (0x4423C0); the vector by value, raw
    fn set_up_skip(x: u32, y: u32, z: u32, heading: u32, a: u32, v: GPtr<u8>, c: u32) = cdecl 0x4423C0 => unported;
    /// `CTheCarGenerators::CreateCarGenerator(CVector, float angle, int32 model, int16 c1, int16 c2, uint8 force,
    /// uint8 alarm, uint8 lock, uint16 minDelay, uint16 maxDelay, uint8 iplId, bool owned)` (0x6F31A0); raw dwords
    fn create_car_generator(
        x: u32, y: u32, z: u32, angle: u32, model: u32, c1: u32, c2: u32, force: u32, alarm: u32, lock: u32,
        min_delay: u32, max_delay: u32, ipl: u32, owned: u32
    ) -> u32 = cdecl 0x6F31A0 => unported;
    /// `CSpecialPlateHandler::Add(int32 generator, const char* plate)` (0x6F2D90)
    fn special_plate_add(h: GPtr<u8>, generator: u32, plate: GPtr<u8>) = thiscall 0x6F2D90 => unported;
    /// `CPointLights::AddLight(uint8 type, CVector pos, CVector dir, float radius, float r, float g, float b, uint8
    /// fog, bool shadows, CEntity*)` (0x7000E0); vectors by value, raw floats
    fn add_light(
        ty: u32, x: u32, y: u32, z: u32, dx: u32, dy: u32, dz: u32, radius: u32, r: f32, g: f32, b: f32, fog: u32,
        shadows: u32, e: u32
    ) = cdecl 0x7000E0 => unported;
    /// `CEntryExitManager::EnableBurglaryHouses(bool)` (0x43F180)
    fn enable_burglary_houses(on: u32) = cdecl 0x43F180 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: u32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CAutomobile::NitrousControl(int8)` (0x6A3EA0)
    fn nitrous_control(v: GPtr<u8>, n: u32) = thiscall 0x6A3EA0 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexUseGoggles::CTaskComplexUseGoggles()` (0x634EF0)
    fn task_use_goggles_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x634EF0 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0)
    fn task_manager_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: u32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CPed::TakeOffGoggles()` (0x5E6010)
    fn take_off_goggles(p: GPtr<u8>) = thiscall 0x5E6010 => unported;
    /// `CPed::OurPedCanSeeThisEntity(CEntity*, bool)` (0x5E1660), AL
    fn our_ped_can_see_this_entity(p: GPtr<u8>, e: GPtr<u8>, b: u32) -> u8 = thiscall 0x5E1660 => unported;
    /// `CAudioEngine::ReportMissionAudioEvent(uint16, CPed*)` (0x507370) / `(uint16, CVehicle*)` (0x507390)
    fn report_mission_audio_event_ped(a: GPtr<u8>, ev: u32, p: GPtr<u8>) = thiscall 0x507370 => unported;
    fn report_mission_audio_event_veh(a: GPtr<u8>, ev: u32, v: GPtr<u8>) = thiscall 0x507390 => unported;
    /// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0)
    fn get_actual_script_thing_index(r: u32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `CDecisionMakerTypes::GetInstance()` (0x4684F0)
    fn decision_maker_types_get_instance() -> GPtr<u8> = cdecl 0x4684F0 => unported;
    /// `CAEPedSpeechAudioEntity::DisableAllPedSpeech()` (0x4E3EB0) / `EnableAllPedSpeech()` (0x4E3EC0)
    fn disable_all_ped_speech() = cdecl 0x4E3EB0 => unported;
    fn enable_all_ped_speech() = cdecl 0x4E3EC0 => unported;
    /// `CGameLogic::DoWeaponStuffAtStartOf2PlayerGame(bool)` (0x4428B0)
    fn do_weapon_stuff_at_start_of_2p_game(b: u32) = cdecl 0x4428B0 => unported;
    /// `CEntity` virtual slot 13 (+0x34, `TestCollision(bool)`), AL
    fn entity_test_collision(e: GPtr<u8>, b: u32) -> u8 = thiscall vslot 0x34 => unported;
    /// `CText::Get(const char*)` (0x6A0050), ECX = `TheText`
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CFont::GetStringWidth(const char*, bool, bool)` (0x71A0E0), ST0 (read as f64: exact)
    fn get_string_width(s: GPtr<u8>, a: u32, b: u32) -> f64 = cdecl 0x71A0E0 => unported;
    /// `CRestart::SetRespawnPointForDurationOfMission(CVector)` (0x460810); the vector by value
    fn set_respawn_point_for_duration_of_mission(x: u32, y: u32, z: u32) = cdecl 0x460810 => unported;
    /// `CModelInfo::IsCarModel(int32)` (0x4C5AA0), AL
    fn is_car_model(m: u32) -> u8 = cdecl 0x4C5AA0 => unported;
    /// `CGangWars::GangWarFightingGoingOn()` (0x443AC0), AL
    fn gang_war_fighting_going_on() -> u8 = cdecl 0x443AC0 => unported;
    /// `CTrain::IsNextStationAllowed(CTrain*)` (0x6F7260), AL / `SkipToNextAllowedStation(CTrain*)` (0x6F72F0)
    fn is_next_station_allowed(v: GPtr<u8>) -> u8 = cdecl 0x6F7260 => unported;
    fn skip_to_next_allowed_station(v: GPtr<u8>) = cdecl 0x6F72F0 => unported;
    /// `CMessages::InsertNumberInString(const char*, int32 x6, char* out)` (0x69DE90)
    fn insert_number_in_string(s: GPtr<u8>, n1: u32, n2: u32, n3: u32, n4: u32, n5: u32, n6: u32, out: GPtr<u8>) = cdecl 0x69DE90 => unported;
    /// `CPed::DisablePedSpeechForScriptSpeech(bool)` (0x5EFF80) / `EnablePedSpeechForScriptSpeech()` (0x5EFF90)
    fn disable_ped_speech_for_script_speech(p: GPtr<u8>, b: u32) = thiscall 0x5EFF80 => unported;
    fn enable_ped_speech_for_script_speech(p: GPtr<u8>) = thiscall 0x5EFF90 => unported;
    /// `CTimer::Stop()` (0x561AA0) / `CTimer::Update()` (0x561B10)
    fn timer_stop() = cdecl 0x561AA0 => unported;
    fn timer_update() = cdecl 0x561B10 => unported;
    /// `CRenderer::RequestObjectsInDirection(const CVector&, float, int32)` (0x555CB0); the float raw
    fn request_objects_in_direction(pos: GPtr<u32>, angle: u32, flags: u32) = cdecl 0x555CB0 => unported;
    /// `CStreaming::LoadScene(const CVector&)` (0x40EB70)
    fn load_scene(pos: GPtr<u32>) = cdecl 0x40EB70 => unported;
    /// `CPedIntelligence::GetTaskJetPack()` (0x601110)
    fn get_task_jet_pack(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601110 => unported;
    /// `CMessages::ClearThisPrintBigNow(eMessageStyle)` (0x69ED80)
    fn clear_this_print_big_now(style: u32) = cdecl 0x69ED80 => unported;
    /// `CMenuManager::HasLanguageChanged()` (0x573CD0), AL
    fn has_language_changed(m: GPtr<u8>) -> u8 = thiscall 0x573CD0 => unported;
    /// `CStats::IncrementStat(uint16, float)` (0x55C180); the float raw
    fn increment_stat(stat: u32, v: u32) = cdecl 0x55C180 => unported;
    /// `CPopulation::ManageAllPopulation()` (0x6160A0)
    fn manage_all_population() = cdecl 0x6160A0 => unported;
    /// `CGarages::AllRespraysCloseOrOpen(bool)` (0x448B30)
    fn all_resprays_close_or_open(b: u32) = cdecl 0x448B30 => unported;
    /// `CAudioEngine::AttachMissionAudioToPhysical(uint8 slot, CPhysical*)` (0x507330)
    fn attach_mission_audio_to_physical(a: GPtr<u8>, slot: u32, p: GPtr<u8>) = thiscall 0x507330 => unported;
    /// `CConversations::SetUpConversationNode(char*, char*, char*, int32, int32, int32)` (0x43A870)
    fn set_up_conversation_node(a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, x: u32, y: u32, z: u32) = cdecl 0x43A870 => unported;
    /// `CHud::SetZoneName(const char*, bool)` (0x588BB0)
    fn set_zone_name(s: GPtr<u8>, b: u32) = cdecl 0x588BB0 => unported;
    /// `CRunningScript::PlayAnimScriptCommand(int32 command)` (0x470150; reads its own parameters)
    fn play_anim_script_command(s: Scr, command: u32) = thiscall 0x470150 => unported;
    /// `CAEVehicleAudioEntity::EnableHelicoptor()` (0x4F5C00) / `DisableHelicoptor()` (0x4F5BF0)
    fn enable_helicoptor(a: GPtr<u8>) = thiscall 0x4F5C00 => unported;
    fn disable_helicoptor(a: GPtr<u8>) = thiscall 0x4F5BF0 => unported;
    /// `CTaskComplexSignalAtPed::CTaskComplexSignalAtPed(CPed*, int32, bool)` (0x660A30)
    fn task_signal_at_ped_ctor(t: GPtr<u8>, p: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x660A30 => unported;
    /// `CRunningScript::GivePedScriptedTask(int32 ped, CTask*, int32 command)` (0x465C20)
    fn give_ped_scripted_task(s: Scr, ped: u32, task: GPtr<u8>, command: u32) = thiscall 0x465C20 => unported;
    /// `CPlayerPed::ForceGroupToAlwaysFollow(bool)` (0x60C7C0)
    fn force_group_to_always_follow(p: GPtr<u8>, b: u32) = thiscall 0x60C7C0 => unported;
    /// `CMenuSystem::GetCarColourFromGrid(uint8, uint8)` (0x5822B0), AL
    fn get_car_colour_from_grid(menu: u32, item: u32) -> u8 = cdecl 0x5822B0 => unported;
    /// `CMenuSystem::HighlightOneItem(uint8, uint8, bool)` (0x581C10)
    fn highlight_one_item(menu: u32, item: u32, b: u32) = cdecl 0x581C10 => unported;
    /// `CCam::Process_FollowPed_SA(const CVector&, float, float, float, bool)` (0x522D40); raw floats
    fn cam_process_follow_ped_sa(c: GPtr<u8>, pos: GPtr<u8>, a: u32, b: u32, d: u32, e: u32) = thiscall 0x522D40 => unported;
    /// `CCam::Process_FollowCar_SA(const CVector&, float, float, float, bool)` (0x5245B0); raw floats
    fn cam_process_follow_car_sa(c: GPtr<u8>, pos: GPtr<u8>, a: u32, b: u32, d: u32, e: u32) = thiscall 0x5245B0 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, b: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CStats::FindMostFavoriteRadioStation()` (0x558FA0)
    fn find_most_favorite_radio_station() -> u32 = cdecl 0x558FA0 => unported;
    /// `CAudioEngine::RetuneRadio(int8)` (0x507E10); the whole EAX is pushed
    fn retune_radio(a: GPtr<u8>, station: u32) = thiscall 0x507E10 => unported;
}

/// `PED(h)` / `VEH(h)` / `OBJ(h)`.
#[inline]
fn ped(h: u32) -> GPtr<u8> {
    ped_at_ref(pool(PED_POOL), h as i32)
}
#[inline]
fn veh(h: u32) -> GPtr<u8> {
    vehicle_at_ref(pool(VEHICLE_POOL), h as i32)
}
#[inline]
fn obj(h: u32) -> GPtr<u8> {
    object_at_ref(pool(OBJECT_POOL), h as i32)
}

/// An entity's position: `m_matrix (+0x14) ? &m_matrix->pos (+0x30) : &m_placement.pos (+4)`.
#[inline]
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = e.byte_add(0x14).cast::<GPtr<u8>>().read();
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `p` byte bit `mask` set to `on`.
#[inline]
fn set_byte_bit(p: GPtr<u8>, mask: u8, on: bool) {
    let old = p.read();
    p.write(if on { old | mask } else { old & !mask });
}

/// A dword `flags ^= ((v << k) ^ flags) & (1 << k)`: bit `k` = bit 0 of `v`.
#[inline]
fn set_dword_bit(p: GPtr<u32>, k: u32, v: u32) {
    let old = p.read();
    p.write(old ^ (((v << k) ^ old) & (1 << k)));
}

/// An 8-byte label read into a zeroed 16-byte buffer.
#[inline]
fn label8(s: Scr, buf: &mut [u8; 16]) -> GPtr<u8> {
    let p = GPtr::from_raw(buf.as_mut_ptr());
    read_text_label(s, p, 8);
    p
}

/// 09C7 SET_PLAYER_MODEL (0x47A820): `p = Players[SP0].m_pPed`; `keep = p->+0x4D4`; `p->DeleteRwObject()`;
/// `p->m_nModelIndex (+0x22) = -1`; `p->SetModelIndex(SP1)`; `p->+0x4D4 = keep`.
fn set_player_model(s: Scr) {
    collect(s, 2);
    let m = sp(1);
    let p = player_info(sp(0)).cast::<GPtr<u8>>().read();
    let keep = p.byte_add(0x4D4).cast::<u32>().read();
    entity_delete_rw_object(p);
    p.byte_add(0x22).cast::<u16>().write(0xFFFF);
    entity_set_model_index(p, m);
    p.byte_add(0x4D4).cast::<u32>().write(keep);
}

/// 09C9 REMOVE_CHAR_FROM_CAR_MAINTAIN_POSITION (0x47A880): `p = PED(SP0)`, `v = VEH(SP1)`; pos = p's position
/// (copied); a stack `CTaskSimpleCarSetPedOut(v, ComputeTargetDoorToExit(v, p), false)` (zeroed buffer),
/// `ProcessPed(p)`, `p->SetPosn(pos)`, the destructor 0x647950.
fn remove_char_from_car_maintain_position(s: Scr) {
    collect(s, 2);
    let p = ped(sp(0));
    let v = veh(sp(1));
    let q = entity_pos(p).cast::<u32>();
    let mut pos = [q.read(), q.byte_add(4).read(), q.byte_add(8).read()];
    let door = compute_target_door_to_exit(v, p);
    let mut task = [0u32; 16];
    let t = GPtr::from_raw(task.as_mut_ptr()).cast::<u8>();
    task_car_set_ped_out_ctor(t, v, door, 0);
    task_car_set_ped_out_process_ped(t, p);
    set_posn_v(p, GPtr::from_raw(pos.as_mut_ptr()));
    task_car_set_ped_out_dtor_647950(t);
}

/// 09CA SET_OBJECT_PROOFS (0x47A921): `OBJ(SP0)` physical flags +0x40: bit 18 = SP1, bit 19 = SP2, bit 23 = SP3, bit
/// 20 = SP4, bit 21 = SP5 (bit 0 of each raw operand, `shl; xor; and; xor`, in this order; no null check).
fn set_object_proofs(s: Scr) {
    collect(s, 6);
    let o = obj(sp(0));
    let fl = o.byte_add(0x40).cast::<u32>();
    for (i, k) in [(1, 18), (2, 19), (3, 23), (4, 20), (5, 21)] {
        set_dword_bit(fl, k, sp(i));
    }
}

/// 09D0 IS_VEHICLE_ON_ALL_WHEELS (0x47AA8D): `v = VEH(SP0)`; flag = (v->+0x590 == 9 (bike) and byte +0x804 == 4) or
/// (v->+0x590 == 0 (automobile) and byte +0x960 == 4) (no null check).
fn is_vehicle_on_all_wheels(s: Scr) {
    collect(s, 1);
    let v = veh(sp(0));
    let ty = v.byte_add(0x590).cast::<u32>().read();
    let mut b = false;
    if ty == 9 && v.byte_add(0x804).read() == 4 {
        b = true;
    }
    if ty == 0 && v.byte_add(0x960).read() == 4 {
        b = true;
    }
    set_flag(s, b);
}

/// 09DA IS_MONEY_PICKUP_AT_COORDS (0x47AC65): over the 620 pickups while none is found (the counter advances past
/// the hit): type byte +0x1C == 8 (money): `d = GetPosn() - pos` (`VectorSub`), taken when `Magnitude(d) < 0.5`
/// (`test ah,5; jp`).
fn is_money_pickup_at_coords(s: Scr) {
    collect(s, 3);
    let mut pos = [sp(0), sp(1), sp(2)];
    let mut found = false;
    let mut i = 0u32;
    while !found && i < 620 {
        let pk = GPtr::<u8>::from_addr(PICKUPS.wrapping_add(i * 0x20) as usize);
        i += 1;
        if pk.byte_add(0x1C).read() != 8 {
            continue;
        }
        let mut posn = [0u32; 3];
        let pp = pickup_get_posn(pk, GPtr::from_raw(posn.as_mut_ptr()));
        let mut diff = [0u32; 3];
        let r = vector_sub(GPtr::from_raw(diff.as_mut_ptr()), pp, GPtr::from_raw(pos.as_mut_ptr()));
        let mut d = [r.read(), r.byte_add(4).read(), r.byte_add(8).read()];
        let m = X87::from_f64_exact(vector_magnitude(GPtr::from_raw(d.as_mut_ptr())));
        if m.fcom(X87::constant(K_HALF)) == Cmp::Less {
            found = true;
        }
    }
    set_flag(s, found);
}

/// 09DD MAKE_ROOM_IN_PLAYER_GANG_FOR_MISSION_PEDS (0x47AD5A): `g = FindPlayerPed(-1)->m_pPlayerData (+0x480)->
/// m_nPlayerGroup (+0x38)`; `m = &ms_groups[g].m_groupMembership`; `pd = FindPlayerPed(-1)->m_pPlayerData`; `n =
/// m.CountMembersExcludingLeader() - (u8)pd->+0x43 - SP0`; n > 0: `m.RemoveNFollowers(n)`.
fn make_room_in_player_gang(s: Scr) {
    collect(s, 1);
    let g = find_player_ped(-1).byte_add(0x480).cast::<GPtr<u8>>().read().byte_add(0x38).cast::<u32>().read();
    let pd = find_player_ped(-1).byte_add(0x480).cast::<GPtr<u8>>().read();
    let m = GPtr::<u8>::from_addr(GROUP_MEMBERSHIP.wrapping_add(g.wrapping_mul(0x2D4)) as usize);
    let n = count_members_excluding_leader(m)
        .wrapping_sub(i32::from(pd.byte_add(0x43).read()))
        .wrapping_sub(spi(0));
    if n > 0 {
        remove_n_followers(m, n);
    }
}

/// 09E2 CREATE_CAR_GENERATOR_WITH_PLATE (0x47AECF): `CollectParameters(12)`; z += 0.015f when `z > -100` (`fadd m32`,
/// stored f32); `generator = CreateCarGenerator(CVector(SP0, SP1, z), SP3, SP4, (u16)SP5, (u16)SP6, (u8)SP7, (u8)SP8,
/// (u8)SP9, (u16)SP10, (u16)SP11, 0, true)`; a 9-byte plate label (zeroed buffer) whose first 8 bytes have '_' and
/// NUL replaced by ' ', byte 8 = 0; `SpecialPlateHandler.Add(generator, plate)`; SP0 = generator; `StoreParameters(1)`.
fn create_car_generator_with_plate(s: Scr) {
    collect(s, 12);
    let mut z = sp(2);
    if f(z) > f(K_MINUS_100) {
        z = X87::from_f32(f(z)).fadd_m(f(K_0_015)).to_f32().to_bits();
    }
    let generator = create_car_generator(
        sp(0),
        sp(1),
        z,
        sp(3),
        sp(4),
        sp(5) & 0xFFFF,
        sp(6) & 0xFFFF,
        sp(7) & 0xFF,
        sp(8) & 0xFF,
        sp(9) & 0xFF,
        sp(10) & 0xFFFF,
        sp(11) & 0xFFFF,
        0,
        1,
    );
    let mut plate = [0u8; 16];
    let pp = GPtr::from_raw(plate.as_mut_ptr());
    read_text_label(s, pp, 9);
    for c in plate.iter_mut().take(8) {
        if *c == b'_' || *c == 0 {
            *c = b' ';
        }
    }
    plate[8] = 0;
    special_plate_add(GPtr::from_addr(SPECIAL_PLATE_HANDLER), generator, GPtr::from_raw(plate.as_mut_ptr()));
    set_sp(0, generator);
    store(s, 1);
}

/// 09E5 DRAW_LIGHT_WITH_RANGE (0x47B02B): `CollectParameters(7)`; colour components `(float)SPi * (1/255)` (`fild;
/// fmul m32; fstp`, b, g, r in this order); `CPointLights::AddLight(0, CVector(SP0, SP1, SP2), CVector(0, 0, 0),
/// SP6, r, g, b, 0, true, null)`.
fn draw_light_with_range(s: Scr) {
    collect(s, 7);
    let c = |i| X87::from_i32(spi(i)).fmul_m(f(K_1_255)).to_f32();
    let b = c(5);
    let g = c(4);
    let r = c(3);
    add_light(0, sp(0), sp(1), sp(2), 0, 0, 0, sp(6), r, g, b, 0, 1, 0);
}

/// 09EB PLAYER_TAKE_OFF_GOGGLES (0x47B1A4): `p = Players[SP0].m_pPed`; SP1 == 0: `p->TakeOffGoggles()`; otherwise,
/// with goggles (+0x4FC) and an empty `m_pIntelligence->+0x10`: `new CTaskComplexUseGoggles` (0xC bytes; null when
/// `new` fails), `m_TaskMgr.SetTask(task, 3, false)`, and `p->m_pPlayerData (+0x480)->+0x85 = 1` when non-null.
fn player_take_off_goggles(s: Scr) {
    collect(s, 2);
    let p = player_info(sp(0)).cast::<GPtr<u8>>().read();
    if sp(1) == 0 {
        take_off_goggles(p);
        return;
    }
    if p.byte_add(0x4FC).cast::<u32>().read() == 0 {
        return;
    }
    let intel = p.byte_add(0x47C).cast::<GPtr<u8>>().read();
    if intel.byte_add(0x10).cast::<u32>().read() != 0 {
        return;
    }
    let t = task_new(0xC);
    let t = if t.is_null() { GPtr::NULL } else { task_use_goggles_ctor(t) };
    let intel = p.byte_add(0x47C).cast::<GPtr<u8>>().read();
    task_manager_set_task(intel.byte_add(4), t, 3, 0);
    let pd = p.byte_add(0x480).cast::<GPtr<u8>>().read();
    if !pd.is_null() {
        pd.byte_add(0x85).write(1);
    }
}

/// 0A08 GET_STRING_WIDTH_WITH_NUMBER (0x47B6EB): label (zeroed buffer); `t = TheText.Get(label)`;
/// `CollectParameters(1)`; `InsertNumberInString(t, SP0, -1, -1, -1, -1, -1, buf)` (zeroed buffer); SP0 =
/// `_ftol(CFont::GetStringWidth(buf, true, true))`; `StoreParameters(1)`.
fn get_string_width_with_number(s: Scr) {
    let mut label = [0u8; 16];
    let lp = label8(s, &mut label);
    let t = text_get(GPtr::from_addr(THE_TEXT), lp);
    collect(s, 1);
    let mut buf = [0u16; 0x100];
    let bp = GPtr::from_raw(buf.as_mut_ptr()).cast::<u8>();
    let m = u32::MAX;
    insert_number_in_string(t, sp(0), m, m, m, m, m, bp);
    let w = get_string_width(bp, 1, 1);
    set_sp(0, X87::from_f64_exact(w).ftol() as u32);
    store(s, 1);
}

/// 0A0B LOAD_SCENE_IN_DIRECTION (0x47B7E4): `CollectParameters(4)`; `CTimer::Stop()`;
/// `CRenderer::RequestObjectsInDirection(pos, SP3 * pi/180 (stored f32), 0x20)`; `CStreaming::LoadScene(pos)`;
/// `CTimer::Update()`.
fn load_scene_in_direction(s: Scr) {
    collect(s, 4);
    let mut pos = [sp(0), sp(1), sp(2)];
    let heading = sp(3);
    timer_stop();
    let a = X87::from_f32(f(heading)).fmul_m(f(K_DEG_TO_RAD)).to_f32().to_bits();
    let pp = GPtr::from_raw(pos.as_mut_ptr());
    request_objects_in_direction(pp, a, 0x20);
    load_scene(pp);
    timer_update();
}

/// 0A1B IS_CHAR_TOUCHING_CHAR (0x47BB17): `p1 = PED(SP0)`, `p2 = PED(SP1)` (the pool pointer read once); v1/v2 =
/// the ped's vehicle (+0x58C) when in a vehicle (+0x46C bit 8), else null (EBX starts at the dispatcher's 0).
/// v1 and v2: `v1->GetHasCollidedWith(v2) || v2->GetHasCollidedWith(v1)`; v1 only: `p2->GetHasCollidedWith(v1)`;
/// v2 only: `p1->GetHasCollidedWith(v2)`; neither: `p1->GetHasCollidedWith(p2)`.
fn is_char_touching_char(s: Scr) {
    collect(s, 2);
    let pp = pool(PED_POOL);
    let p1 = ped_at_ref(pp, spi(0));
    let p2 = ped_at_ref(pp, spi(1));
    let veh_of = |p: GPtr<u8>| {
        if p.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 {
            p.byte_add(0x58C).cast::<GPtr<u8>>().read()
        } else {
            GPtr::NULL
        }
    };
    let v1 = veh_of(p1);
    let v2 = veh_of(p2);
    let r = if !v1.is_null() {
        if !v2.is_null() {
            get_has_collided_with(v1, v2) != 0 || get_has_collided_with(v2, v1) != 0
        } else {
            get_has_collided_with(p2, v1) != 0
        }
    } else if !v2.is_null() {
        get_has_collided_with(p1, v2) != 0
    } else {
        get_has_collided_with(p1, p2) != 0
    };
    set_flag(s, r);
}

/// 0A1D TASK_HAND_GESTURE (0x47BBF7): `new CTaskComplexSignalAtPed(PED(SP1), -1, false)` (0x18 bytes; null when `new`
/// fails); `GivePedScriptedTask(SP0, task, command)`.
fn task_hand_gesture(s: Scr, command: u32) {
    collect(s, 2);
    let target = ped(sp(1));
    let h = sp(0);
    let t = task_new(0x18);
    let t = if t.is_null() { GPtr::NULL } else { task_signal_at_ped_ctor(t, target, u32::MAX, 0) };
    give_ped_scripted_task(s, h, t, command);
}

/// The active camera `TheCamera.Cams[(u8)ActiveCam]` (the index is re-read at every use, as the original does).
#[inline]
fn active_cam() -> GPtr<u8> {
    let i = u32::from(GPtr::<u8>::from_addr(ACTIVE_CAM).read());
    GPtr::from_addr(CAMS.wrapping_add(i * 0x238) as usize)
}

/// 0A25 SET_CAMERA_POSITION_UNFIXED (0x47BDD2): active cam +0xAC = SP0, +0xBC = SP1; mode (u16 +0xC) == 4:
/// `Process_FollowPed_SA(FindPlayerPed(-1)'s position, 0, 0, 0, true)`; else with `FindPlayerVehicle(-1, false)` and
/// mode 0x12, 0x16 or 3: `Process_FollowCar_SA(FindPlayerVehicle(-1, false)'s position, 0, 0, 0, true)`.
fn set_camera_position_unfixed(s: Scr) {
    collect(s, 2);
    active_cam().byte_add(0xAC).cast::<u32>().write(sp(0));
    active_cam().byte_add(0xBC).cast::<u32>().write(sp(1));
    if active_cam().byte_add(0xC).cast::<u16>().read() == 4 {
        let pos = entity_pos(find_player_ped(-1));
        cam_process_follow_ped_sa(active_cam(), pos, 0, 0, 0, 1);
        return;
    }
    if find_player_vehicle(-1, 0).is_null() {
        return;
    }
    let mode = active_cam().byte_add(0xC).cast::<u16>().read();
    if mode == 0x12 || mode == 0x16 || mode == 3 {
        let pos = entity_pos(find_player_vehicle(-1, 0));
        cam_process_follow_car_sa(active_cam(), pos, 0, 0, 0, 1);
    }
}

/// `CRunningScript::ProcessCommands2500To2599(int32 command)` (0x47A760, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x47A760, conv = thiscall, name = "CRunningScript::ProcessCommands2500To2599", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript", fallback)]
pub fn process_commands_2500_to_2599(s: Scr, command: u32) -> i8 {
    match command {
        // Cases the jump table sends to the shared `xor al,al` 0x47BF2B (AL = 0, no operand read): 09DC
        // SET_CHAR_CAN_CLIMB_OUT_WATER, 09EA PLAYER_PUT_ON_GOGGLES, 09F3 GET_RANDOM_TRAIN_IN_SPHERE_NO_SAVE, 09F9
        // SET_MENU_HEADER_ORIENTATION, 0A00 IS_THIS_MODEL_A_BIKE, 0A04/0A05 SET/GET_VEHICLE_FIRING_RATE_MULTIPLIER,
        // 0A0D BLOCK_VEHICLE_MODEL. gr: 09DC `unimplemented` (-1), the exe returns 0. (09C6 / 09D3 also point there
        // in the unmodified exe but stay on the fallback: SA:MP patches their table entries.)
        0x09DC | 0x09EA | 0x09F3 | 0x09F9 | 0x0A00 | 0x0A04 | 0x0A05 | 0x0A0D => {}
        // 09C4 SET_PETROL_TANK_WEAKPOINT (0x47A7A2): `VEH(SP0)` +0x42E bit 6 = bit 0 of SP1 (`shl cl,6; xor; and 0x40;
        // xor`, no null check)
        0x09C4 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let b = v.byte_add(0x42E);
            let old = b.read();
            let cl = ((sp(1) as u8) << 6 ^ old) & 0x40;
            b.write(old ^ cl);
        }
        // 09C5 IS_CHAR_USING_MAP_ATTRACTOR (0x47A7E1): flag = AL of `GetPedAttractorManager()->
        // IsPedRegisteredWithEffect(PED(SP0))`
        0x09C5 => {
            collect(s, 1);
            let p = ped(sp(0));
            let m = get_ped_attractor_manager();
            flag(s, u32::from(is_ped_registered_with_effect(m, p)));
        }
        // 09C7 SET_PLAYER_MODEL (0x47A820)
        0x09C7 => set_player_model(s),
        // 09C8 ARE_SUBTITLES_SWITCHED_ON (0x47A869): flag = byte [0xBA678C] (zero-extended)
        0x09C8 => flag(s, u32::from(GPtr::<u8>::from_addr(0xBA678C).read())),
        // 09C9 REMOVE_CHAR_FROM_CAR_MAINTAIN_POSITION (0x47A880)
        0x09C9 => remove_char_from_car_maintain_position(s),
        // 09CA SET_OBJECT_PROOFS (0x47A921)
        0x09CA => set_object_proofs(s),
        // 09CB IS_CAR_TOUCHING_CAR (0x47A9B6): flag = AL of `VEH(SP0)->GetHasCollidedWith(VEH(SP1))` (the pool
        // pointer read once)
        0x09CB => {
            collect(s, 2);
            let vp = pool(VEHICLE_POOL);
            let a = vehicle_at_ref(vp, spi(0));
            let b = vehicle_at_ref(vp, spi(1));
            flag(s, u32::from(get_has_collided_with(a, b)));
        }
        // 09CC DOES_OBJECT_HAVE_THIS_MODEL (0x47AA01): model rule on SP1; flag = `(i16)OBJ(SP0)->+0x22 == m`
        0x09CC => {
            collect(s, 2);
            let o = obj(sp(0));
            let mut m = sp(1);
            if (m as i32) < 0 {
                m = GPtr::<u32>::from_addr(0xA44B88u32.wrapping_sub(m.wrapping_mul(0x1C)) as usize).read();
            }
            set_flag(s, i32::from(o.byte_add(0x22).cast::<i16>().read()) as u32 == m);
        }
        // 09CF SET_TRAIN_FORCED_TO_SLOW_DOWN (0x47AA49): `VEH(SP0)` byte +0x5B9 bit 1 = SP1 != 0
        0x09CF => {
            collect(s, 2);
            set_byte_bit(veh(sp(0)).byte_add(0x5B9), 2, sp(1) != 0);
        }
        // 09D0 IS_VEHICLE_ON_ALL_WHEELS (0x47AA8D)
        0x09D0 => is_vehicle_on_all_wheels(s),
        // 09D1 DOES_PICKUP_EXIST (0x47AAEB): flag = `CPickups::GetActualPickupIndex(SP0) != -1`
        0x09D1 => {
            collect(s, 1);
            set_flag(s, get_actual_pickup_index(sp(0)) != -1);
        }
        // 09D2 ENABLE_AMBIENT_CRIME (0x47AB20): byte [0xB72994] = SP0 != 0
        0x09D2 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0xB72994).write(u8::from(sp(0) != 0));
        }
        // 09D4 CLEAR_WANTED_LEVEL_IN_GARAGE (0x47AB3F): `FindPlayerWanted(-1)->ClearWantedLevelAndGoOnParole()`
        0x09D4 => clear_wanted_level_and_go_on_parole(find_player_wanted(-1)),
        // 09D5 SET_CHAR_SAY_CONTEXT_IMPORTANT (0x47AB57): SP0 = (i16)`PED(SP0)->Say((u16)SP1, 0, 1.0f, (u8)SP2,
        // (u8)SP3, (u8)SP4)`; `StoreParameters(1)`
        0x09D5 => {
            collect(s, 5);
            let p = ped(sp(0));
            let r = ped_say(p, sp(1) & 0xFFFF, 0, 0x3F80_0000, sp(2) & 0xFF, sp(3) & 0xFF, sp(4) & 0xFF);
            set_sp(0, i32::from(r as i16) as u32);
            store(s, 1);
        }
        // 09D6 SET_CHAR_SAY_SCRIPT (0x47ABBC): `PED(SP0)->SayScript(SP1, (u8)SP2, (u8)SP3, (u8)SP4)`
        0x09D6 => {
            collect(s, 5);
            let p = ped(sp(0));
            ped_say_script(p, sp(1), sp(2) & 0xFF, sp(3) & 0xFF, sp(4) & 0xFF);
        }
        // 09D7 FORCE_INTERIOR_LIGHTING_FOR_PLAYER (0x47AC06): `Players[SP0].m_pPed->m_pPlayerData (+0x480)` byte +0x86
        // = SP1 != 0
        0x09D7 => {
            collect(s, 2);
            let p = player_info(sp(0)).cast::<GPtr<u8>>().read();
            let pd = p.byte_add(0x480).cast::<GPtr<u8>>().read();
            pd.byte_add(0x86).write(u8::from(sp(1) != 0));
        }
        // 09D8 DISABLE_2ND_PAD_FOR_DEBUG (0x47AC3E): the operand is read, nothing else
        0x09D8 => collect(s, 1),
        // 09D9 USE_DETONATOR (0x47AC4E): `CWorld::UseDetonator(FindPlayerPed(-1))`
        0x09D9 => use_detonator(find_player_ped(-1)),
        // 09DA IS_MONEY_PICKUP_AT_COORDS (0x47AC65)
        0x09DA => is_money_pickup_at_coords(s),
        // 09DB SET_MENU_COLUMN_WIDTH (0x47AD10): `w = _ftol(fild RsGlobal.maximumWidth * 0.0015625f * (int)(u16)SP2)`
        // (x87 PC=24: `fild; fmul m32; fimul m32`, `_ftol` 0x821B40 inlined as `X87::ftol`, low dword);
        // `SetColumnWidth((u8)SP0, (u8)SP1, w)`
        0x09DB => {
            collect(s, 3);
            let width = sa_core::platform::RS_MAXIMUM_WIDTH.get();
            let w = X87::from_i32(width)
                .fmul_m(f(K_ONE_640TH))
                .fmul(X87::from_i32(i32::from(sp(2) as u16)))
                .ftol() as u32;
            set_column_width(sp(0) & 0xFF, sp(1) & 0xFF, w);
        }
        // 09DD MAKE_ROOM_IN_PLAYER_GANG_FOR_MISSION_PEDS (0x47AD5A)
        0x09DD => make_room_in_player_gang(s),
        // 09DE IS_CHAR_GETTING_IN_TO_A_CAR (0x47ADB9): flag = (active task 0x2BD or 0x2BC) and no active task 0x320
        // (`m_TaskMgr.FindActiveTaskByType`, the intelligence pointer re-read each time)
        0x09DE => {
            collect(s, 1);
            let p = ped(sp(0));
            let tm = || p.byte_add(0x47C).cast::<GPtr<u8>>().read().byte_add(4);
            let entering = !find_active_task_by_type(tm(), 0x2BD).is_null()
                || !find_active_task_by_type(tm(), 0x2BC).is_null();
            let b = entering && find_active_task_by_type(tm(), 0x320).is_null();
            set_flag(s, b);
        }
        // 09E0 SET_UP_SKIP_FOR_SPECIFIC_VEHICLE (0x47AE2C): `CGameLogic::SetUpSkip(CVector(SP0, SP1, SP2), SP3, false,
        // VEH(SP4), false)`
        0x09E0 => {
            collect(s, 5);
            let v = veh(sp(4));
            set_up_skip(sp(0), sp(1), sp(2), sp(3), 0, v, 0);
        }
        // 09E1 GET_CAR_MODEL_VALUE (0x47AE95): SP0 = `handling[(u8)ms_modelInfoPtrs[SP0]->+0x4A]` dword +0xD4
        0x09E1 => {
            collect(s, 1);
            let mi = GPtr::<GPtr<u8>>::from_addr(MODEL_INFO_PTRS.wrapping_add(sp(0).wrapping_mul(4)) as usize).read();
            let h = u32::from(mi.byte_add(0x4A).read());
            set_sp(0, GPtr::<u32>::from_addr(HANDLING_VALUE.wrapping_add(h * 0xE0) as usize).read());
            store(s, 1);
        }
        // 09E2 CREATE_CAR_GENERATOR_WITH_PLATE (0x47AECF)
        0x09E2 => create_car_generator_with_plate(s),
        // 09E3 FIND_TRAIN_DIRECTION (0x47AFD2): pushes `((u8)VEH(SP0)->+0x5B8 >> 6) & 0xFFFFFF01` (exact dword)
        0x09E3 => {
            collect(s, 1);
            flag(s, u32::from(veh(sp(0)).byte_add(0x5B8).read() >> 6) & 0xFFFF_FF01);
        }
        // 09E4 SET_AIRCRAFT_CARRIER_SAM_SITE (0x47B00C): byte [0x8D0A24] = SP0 == 0
        0x09E4 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0x8D0A24).write(u8::from(sp(0) == 0));
        }
        // 09E5 DRAW_LIGHT_WITH_RANGE (0x47B02B)
        0x09E5 => draw_light_with_range(s),
        // 09E6 ENABLE_BURGLARY_HOUSES (0x47B0E7)
        0x09E6 => {
            collect(s, 1);
            enable_burglary_houses(u32::from(sp(0) != 0));
        }
        // 09E7 IS_PLAYER_CONTROL_ON (0x47B109): flag = `!(CPad::GetPad(SP0) byte +0x10E bit 5)`
        0x09E7 => {
            collect(s, 1);
            let b = get_pad(sp(0)).byte_add(0x10E).read();
            set_flag(s, (b >> 5) & 1 == 0);
        }
        // 09E8 GET_CHAR_AREA_VISIBLE (0x47B145): SP0 = byte `PED(SP0)` +0x2F
        0x09E8 => {
            collect(s, 1);
            set_sp(0, u32::from(ped(sp(0)).byte_add(0x2F).read()));
            store(s, 1);
        }
        // 09E9 GIVE_NON_PLAYER_CAR_NITRO (0x47B179): `VEH(SP0)->NitrousControl(1)`
        0x09E9 => {
            collect(s, 1);
            nitrous_control(veh(sp(0)), 1);
        }
        // 09EB PLAYER_TAKE_OFF_GOGGLES (0x47B1A4)
        0x09EB => player_take_off_goggles(s),
        // 09EC ALLOW_FIXED_CAMERA_COLLISION (0x47B250): byte [0xB6EC2C] = SP0 != 0
        0x09EC => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0xB6EC2C).write(u8::from(sp(0) != 0));
        }
        // 09ED HAS_CHAR_SPOTTED_CHAR_IN_FRONT (0x47B26F): flag = AL of `PED(SP0)->OurPedCanSeeThisEntity(PED(SP1),
        // false)` (the pool pointer read once)
        0x09ED => {
            collect(s, 2);
            let pp = pool(PED_POOL);
            let a = ped_at_ref(pp, spi(0));
            let b = ped_at_ref(pp, spi(1));
            flag(s, u32::from(our_ped_can_see_this_entity(a, b, 0)));
        }
        // 09EE FORCE_BIG_MESSAGE_AND_COUNTER (0x47B2BC): byte [0xBAA3FA] = (u8)SP0
        0x09EE => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0xBAA3FA).write(sp(0) as u8);
        }
        // 09EF SET_VEHICLE_CAMERA_TWEAK (0x47B2D8): [0xB6FD5C] = SP0, [0xB6FD68] = SP3, [0xB6FD60] = SP1, [0xB6FD64] = SP2
        0x09EF => {
            collect(s, 4);
            GPtr::<u32>::from_addr(0xB6FD5C).write(sp(0));
            GPtr::<u32>::from_addr(0xB6FD68).write(sp(3));
            GPtr::<u32>::from_addr(0xB6FD60).write(sp(1));
            GPtr::<u32>::from_addr(0xB6FD64).write(sp(2));
        }
        // 09F0 RESET_VEHICLE_CAMERA_TWEAK (0x47B314): -1, 1.0f, 1.0f, 0
        0x09F0 => {
            GPtr::<u32>::from_addr(0xB6FD5C).write(u32::MAX);
            GPtr::<u32>::from_addr(0xB6FD60).write(0x3F80_0000);
            GPtr::<u32>::from_addr(0xB6FD64).write(0x3F80_0000);
            GPtr::<u32>::from_addr(0xB6FD68).write(0);
        }
        // 09F1 REPORT_MISSION_AUDIO_EVENT_AT_CHAR (0x47B343) / 09F7 REPORT_MISSION_AUDIO_EVENT_AT_CAR (0x47B479):
        // `AudioEngine.ReportMissionAudioEvent((u16)SP1, PED(SP0) / VEH(SP0))`
        0x09F1 => {
            collect(s, 2);
            let p = ped(sp(0));
            report_mission_audio_event_ped(GPtr::from_addr(AUDIO_ENGINE), sp(1) & 0xFFFF, p);
        }
        0x09F7 => {
            collect(s, 2);
            let v = veh(sp(0));
            report_mission_audio_event_veh(GPtr::from_addr(AUDIO_ENGINE), sp(1) & 0xFFFF, v);
        }
        // 09F2 DOES_DECISION_MAKER_EXIST (0x47B37A): `i = GetActualScriptThingIndex(SP0, 7)`; flag = 0 <= i < 20 and
        // (after a discarded `CDecisionMakerTypes::GetInstance()`) byte [0xC0B01C + i] != 0
        0x09F2 => {
            collect(s, 1);
            let i = get_actual_script_thing_index(sp(0), 7);
            let mut b = false;
            if (0..0x14).contains(&i) {
                decision_maker_types_get_instance();
                b = GPtr::<u8>::from_addr(DECISION_MAKER_USED.wrapping_add(i as u32) as usize).read() != 0;
            }
            set_flag(s, b);
        }
        // 09F4 IGNORE_HEIGHT_DIFFERENCE_FOLLOWING_NODES (0x47B3BB) / 09F6 SET_CHAR_GET_OUT_UPSIDE_DOWN_CAR (0x47B42F) /
        // 0A27 SET_DEATH_WEAPONS_PERSIST (0x47BEE7): `PED(SP0)` flags +0x478 bit 15 / 17 / 19 = SP1 != 0
        0x09F4 | 0x09F6 | 0x0A27 => {
            collect(s, 2);
            let k = if command == 0x09F4 {
                15
            } else if command == 0x09F6 {
                17
            } else {
                19
            };
            set_dword_bit(ped(sp(0)).byte_add(0x478).cast::<u32>(), k, u32::from(sp(1) != 0));
        }
        // 09F5 SHUT_ALL_CHARS_UP (0x47B406)
        0x09F5 => {
            collect(s, 1);
            if sp(0) != 0 {
                disable_all_ped_speech();
            } else {
                enable_all_ped_speech();
            }
        }
        // 09F8 DO_WEAPON_STUFF_AT_START_OF_2P_GAME (0x47B4AF): `DoWeaponStuffAtStartOf2PlayerGame(true)`
        0x09F8 => do_weapon_stuff_at_start_of_2p_game(1),
        // 09FA HAS_GAME_JUST_RETURNED_FROM_FRONTEND (0x47B4C0): flag = `CPad::GetPad(0)` byte +0x117 == 1
        0x09FA => {
            let b = get_pad(0).byte_add(0x117).read();
            set_flag(s, b == 1);
        }
        // 09FB GET_CURRENT_LANGUAGE (0x47B4EC): SP0 = (i8) byte [0xBA67CC]
        0x09FB => {
            set_sp(0, i32::from(GPtr::<u8>::from_addr(0xBA67CC).read() as i8) as u32);
            store(s, 1);
        }
        // 09FC IS_OBJECT_INTERSECTING_WORLD (0x47B509): flag = AL of `OBJ(SP0)->TestCollision(false)` (virtual slot 13)
        0x09FC => {
            collect(s, 1);
            let o = obj(sp(0));
            flag(s, u32::from(entity_test_collision(o, 0)));
        }
        // 09FD GET_STRING_WIDTH (0x47B543): SP0 = `_ftol(CFont::GetStringWidth(TheText.Get(label), true, true))`
        0x09FD => {
            let mut label = [0u8; 16];
            let lp = label8(s, &mut label);
            let t = text_get(GPtr::from_addr(THE_TEXT), lp);
            let w = get_string_width(t, 1, 1);
            set_sp(0, X87::from_f64_exact(w).ftol() as u32);
            store(s, 1);
        }
        // 09FE RESET_VEHICLE_HYDRAULICS (0x47B587): `v = VEH(SP0)`; dword +0x38C bit 17 and +0x590 == 0: u16 +0x86C = 0
        0x09FE => {
            collect(s, 1);
            let v = veh(sp(0));
            if v.byte_add(0x38C).cast::<u32>().read() & 0x2_0000 != 0 && v.byte_add(0x590).cast::<u32>().read() == 0 {
                v.byte_add(0x86C).cast::<u16>().write(0);
            }
        }
        // 09FF SET_RESPAWN_POINT_FOR_DURATION_OF_MISSION (0x47B5CB)
        0x09FF => {
            collect(s, 3);
            set_respawn_point_for_duration_of_mission(sp(0), sp(1), sp(2));
        }
        // 0A01 IS_THIS_MODEL_A_CAR (0x47B619): flag = `CModelInfo::IsCarModel(SP0) != 0`
        0x0A01 => {
            collect(s, 1);
            set_flag(s, is_car_model(sp(0)) != 0);
        }
        // 0A02 SWITCH_ON_GROUND_SEARCHLIGHT (0x47B63A): `i = GetActualScriptThingIndex(SP0, 2)`; i >= 0: byte
        // [0xA94D6A + i * 0x7C] = (u8)SP1
        0x0A02 => {
            collect(s, 2);
            let i = get_actual_script_thing_index(sp(0), 2);
            if i >= 0 {
                GPtr::<u8>::from_addr(SEARCHLIGHT_ON_GROUND.wrapping_add(i as u32 * 0x7C) as usize).write(sp(1) as u8);
            }
        }
        // 0A03 IS_GANG_WAR_FIGHTING_GOING_ON (0x47B672): flag = `GangWarFightingGoingOn() != 0`
        0x0A03 => set_flag(s, gang_war_fighting_going_on() != 0),
        // 0A06 IS_NEXT_STATION_ALLOWED (0x47B698): flag = AL of `CTrain::IsNextStationAllowed(VEH(SP0))`
        0x0A06 => {
            collect(s, 1);
            flag(s, u32::from(is_next_station_allowed(veh(sp(0)))));
        }
        // 0A07 SKIP_TO_NEXT_ALLOWED_STATION (0x47B6C0)
        0x0A07 => {
            collect(s, 1);
            skip_to_next_allowed_station(veh(sp(0)));
        }
        // 0A08 GET_STRING_WIDTH_WITH_NUMBER (0x47B6EB)
        0x0A08 => get_string_width_with_number(s),
        // 0A09 SHUT_CHAR_UP_FOR_SCRIPTED_SPEECH (0x47B75C): `p = PED(SP0)`; p non-null: SP1 != 0 →
        // `DisablePedSpeechForScriptSpeech(false)`, else `EnablePedSpeechForScriptSpeech()`
        0x0A09 => {
            collect(s, 2);
            let p = ped(sp(0));
            if !p.is_null() {
                if sp(1) != 0 {
                    disable_ped_speech_for_script_speech(p, 0);
                } else {
                    enable_ped_speech_for_script_speech(p);
                }
            }
        }
        // 0A0A ENABLE_DISABLED_ATTRACTORS_ON_OBJECT (0x47B7A2): `OBJ(SP0)` dword +0x140 bit 24 = bit 0 of SP1
        0x0A0A => {
            collect(s, 2);
            set_dword_bit(obj(sp(0)).byte_add(0x140).cast::<u32>(), 24, sp(1));
        }
        // 0A0B LOAD_SCENE_IN_DIRECTION (0x47B7E4)
        0x0A0B => load_scene_in_direction(s),
        // 0A0C IS_PLAYER_USING_JETPACK (0x47B84C): flag = `Players[SP0].m_pPed->m_pIntelligence->GetTaskJetPack() !=
        // null`
        0x0A0C => {
            collect(s, 1);
            let p = player_info(sp(0)).cast::<GPtr<u8>>().read();
            let t = get_task_jet_pack(p.byte_add(0x47C).cast::<GPtr<u8>>().read());
            set_flag(s, !t.is_null());
        }
        // 0A0E CLEAR_THIS_PRINT_BIG_NOW (0x47B88E): `CMessages::ClearThisPrintBigNow(SP0 - 1)`
        0x0A0E => {
            collect(s, 1);
            clear_this_print_big_now(sp(0).wrapping_sub(1));
        }
        // 0A0F HAS_LANGUAGE_CHANGED (0x47B8AD): flag = AL of `FrontEndMenuManager.HasLanguageChanged()`
        0x0A0F => flag(s, u32::from(has_language_changed(GPtr::from_addr(FRONT_END_MENU_MANAGER)))),
        // 0A10 INCREMENT_INT_STAT_NO_MESSAGE (0x47B8CE): `IncrementStat((u16)SP0, (float)SP1)` (`fild; fstp`)
        0x0A10 => {
            collect(s, 2);
            let v = X87::from_i32(spi(1)).to_f32().to_bits();
            increment_stat(sp(0) & 0xFFFF, v);
        }
        // 0A11 SET_EXTRA_CAR_COLOURS (0x47B8FA): `VEH(SP0)` bytes +0x436 = (u8)SP1, +0x437 = (u8)SP2
        0x0A11 => {
            collect(s, 3);
            let v = veh(sp(0));
            v.byte_add(0x436).write(sp(1) as u8);
            v.byte_add(0x437).write(sp(2) as u8);
        }
        // 0A12 GET_EXTRA_CAR_COLOURS (0x47B933): SP0, SP1 = bytes `VEH(SP0)` +0x436, +0x437; `StoreParameters(2)`
        0x0A12 => {
            collect(s, 1);
            let v = veh(sp(0));
            set_sp(0, u32::from(v.byte_add(0x436).read()));
            set_sp(1, u32::from(v.byte_add(0x437).read()));
            store(s, 2);
        }
        // 0A13 MANAGE_ALL_POPULATION (0x47B977)
        0x0A13 => manage_all_population(),
        // 0A14 SET_NO_RESPRAYS (0x47B983): `b = SP0 != 0`; byte [0x96C008] = b; `AllRespraysCloseOrOpen(!b)`
        0x0A14 => {
            collect(s, 1);
            let b = sp(0) != 0;
            GPtr::<u8>::from_addr(0x96C008).write(u8::from(b));
            all_resprays_close_or_open(u32::from(!b));
        }
        // 0A15 HAS_CAR_BEEN_RESPRAYED (0x47B9AF): `VEH(SP0)` byte +0x42F bit 0: flag = the bit, cleared when set
        0x0A15 => {
            collect(s, 1);
            let c = veh(sp(0)).byte_add(0x42F);
            let v = c.read();
            let b = v & 1 != 0;
            if b {
                c.write(v & 0xFE);
            }
            set_flag(s, b);
        }
        // 0A16 ATTACH_MISSION_AUDIO_TO_CAR (0x47B9FB): `AttachMissionAudioToPhysical((u8)(SP0 - 1), VEH(SP1))`
        0x0A16 => {
            collect(s, 2);
            let v = veh(sp(1));
            attach_mission_audio_to_physical(GPtr::from_addr(AUDIO_ENGINE), sp(0).wrapping_sub(1) & 0xFF, v);
        }
        // 0A17 SET_HAS_BEEN_OWNED_FOR_CAR_GENERATOR (0x47BA32): `CarGeneratorArray[(u16)SP0]` byte +0xD bit 3 = SP1 != 0
        0x0A17 => {
            collect(s, 2);
            let g = GPtr::<u8>::from_addr(CAR_GENERATORS.wrapping_add((sp(0) & 0xFFFF) << 5) as usize);
            set_byte_bit(g.byte_add(0xD), 8, sp(1) != 0);
        }
        // 0A18 SET_UP_CONVERSATION_NODE_WITH_SCRIPTED_SPEECH (0x47BA6E): labels A, B, C; `CollectParameters(3)`;
        // `SetUpConversationNode(A, B, C, -SP0, -SP1, -SP2)`
        0x0A18 => {
            let (mut a, mut b, mut c) = ([0u8; 16], [0u8; 16], [0u8; 16]);
            let ap = label8(s, &mut a);
            let bp = label8(s, &mut b);
            let cp = label8(s, &mut c);
            collect(s, 3);
            let n = |i| sp(i).wrapping_neg();
            set_up_conversation_node(ap, bp, cp, n(0), n(1), n(2));
        }
        // 0A19 SET_AREA_NAME (0x47BAD9): `CHud::SetZoneName(TheText.Get(label), true)`
        0x0A19 => {
            let mut label = [0u8; 16];
            let lp = label8(s, &mut label);
            let t = text_get(GPtr::from_addr(THE_TEXT), lp);
            set_zone_name(t, 1);
        }
        // 0A1A TASK_PLAY_ANIM_SECONDARY (0x47BB08): `PlayAnimScriptCommand(command)` (ECX = the command)
        0x0A1A => play_anim_script_command(s, command),
        // 0A1B IS_CHAR_TOUCHING_CHAR (0x47BB17)
        0x0A1B => is_char_touching_char(s),
        // 0A1C DISABLE_HELI_AUDIO (0x47BBB6): `VEH(SP0)->m_vehicleAudio (+0x138)`: SP1 != 0 → EnableHelicoptor, else
        // DisableHelicoptor
        0x0A1C => {
            collect(s, 2);
            let a = veh(sp(0)).byte_add(0x138);
            if sp(1) != 0 {
                enable_helicoptor(a);
            } else {
                disable_helicoptor(a);
            }
        }
        // 0A1D TASK_HAND_GESTURE (0x47BBF7)
        0x0A1D => task_hand_gesture(s, command),
        // 0A1E TAKE_PHOTO (0x47BC69): byte [0xC8A7C1] = 1, byte [0xC402D0] = SP0 != 0
        0x0A1E => {
            collect(s, 1);
            let b = sp(0) != 0;
            GPtr::<u8>::from_addr(0xC8A7C1).write(1);
            GPtr::<u8>::from_addr(0xC402D0).write(u8::from(b));
        }
        // 0A1F INCREMENT_FLOAT_STAT_NO_MESSAGE (0x47BC8F): `IncrementStat((u16)SP0, SP1)` (raw float)
        0x0A1F => {
            collect(s, 2);
            increment_stat(sp(0) & 0xFFFF, sp(1));
        }
        // 0A20 SET_PLAYER_GROUP_TO_FOLLOW_ALWAYS (0x47BCB7): `FindPlayerPed(SP0)->ForceGroupToAlwaysFollow(SP1 != 0)`
        0x0A20 => {
            collect(s, 2);
            let b = u32::from(sp(1) != 0);
            let p = find_player_ped(spi(0));
            force_group_to_always_follow(p, b);
        }
        // 0A21 IMPROVE_CAR_BY_CHEATING (0x47BCE8): `VEH(SP0)` byte +0x42F bit 1 = bit 0 of (u8)SP1 (`shl dl,1`)
        0x0A21 => {
            collect(s, 2);
            let c = veh(sp(0)).byte_add(0x42F);
            let old = c.read();
            c.write(old ^ ((((sp(1) as u8) << 1) ^ old) & 2));
        }
        // 0A22 CHANGE_CAR_COLOUR_FROM_MENU (0x47BD27): `c = GetCarColourFromGrid((u8)SP0, (u8)SP3)`; SP2 == 1 →
        // `VEH(SP1)` byte +0x434 = c, else +0x435 = c
        0x0A22 => {
            collect(s, 4);
            let v = veh(sp(1));
            let c = get_car_colour_from_grid(sp(0) & 0xFF, sp(3) & 0xFF);
            let off = if sp(2) == 1 { 0x434 } else { 0x435 };
            v.byte_add(off).write(c);
        }
        // 0A23 HIGHLIGHT_MENU_ITEM (0x47BD82): `HighlightOneItem((u8)SP0, (u8)SP1, (u8)SP2)`
        0x0A23 => {
            collect(s, 3);
            highlight_one_item(sp(0) & 0xFF, sp(1) & 0xFF, sp(2) & 0xFF);
        }
        // 0A24 SET_DISABLE_MILITARY_ZONES (0x47BDB4): byte [0xC87ACD] = SP0 != 0
        0x0A24 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0xC87ACD).write(u8::from(sp(0) != 0));
        }
        // 0A25 SET_CAMERA_POSITION_UNFIXED (0x47BDD2)
        0x0A25 => set_camera_position_unfixed(s),
        // 0A26 SET_RADIO_TO_PLAYERS_FAVOURITE_STATION (0x47BED3): `AudioEngine.RetuneRadio(
        // CStats::FindMostFavoriteRadioStation())` (the whole EAX pushed)
        0x0A26 => {
            let st = find_most_favorite_radio_station();
            retune_radio(GPtr::from_addr(AUDIO_ENGINE), st);
        }
        _ => {
            return fallback(
                process_commands_2500_to_2599_original,
                "CRunningScript::ProcessCommands2500To2599",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_2500_TO_2599];
