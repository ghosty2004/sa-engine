//! `CRunningScript::ProcessCommands2300To2399` (0x4762D0): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-4). Every opcode of the range has a jump-table case and is in [`PORTED`]; the default (-1,
//! `or al,0xFF` at 0x477BFC, no common tail) is unreachable for 2300..2399 and kept for other command values.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands2300To2399.md`.
//!
//! The "protected store" (`jmp 0x403A72` at 0x477BE0, an obfuscated `SP0 = EDX`, then `StoreParameters(1)` at
//! 0x477BE5) is ported as `SP0 = edx; Store(1)` (0926, 0945, 0947, 0953, 095F).
//!
//! Divergence notes (ledger classes of PORT-SCRIPT-2/3): the 8-byte labels (090C, 0917, 091C, 0937, 093A) are
//! read into zeroed 16-byte buffers (the original's are not cleared; differs only for 8-character labels);
//! `setcc` bools pushed with stale upper bytes are passed as 0/1; 092E's water level out-parameter, 094C's
//! position and the 095C `FxPrtMult_c` start zeroed (the original's start as the case writes them, or are fully
//! written by the callee).

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::{self, Scr};
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x08FC, 0x08FD, 0x08FE, 0x08FF, 0x0900, 0x0901, 0x0902, 0x0903, 0x0904, 0x0905, 0x0906, 0x0907, 0x0908,
    0x0909, 0x090A, 0x090B, 0x090C, 0x090D, 0x090E, 0x090F, 0x0910, 0x0911, 0x0912, 0x0913, 0x0914, 0x0915,
    0x0916, 0x0917, 0x0918, 0x0919, 0x091A, 0x091B, 0x091C, 0x091D, 0x091E, 0x091F, 0x0920, 0x0921, 0x0922,
    0x0923, 0x0924, 0x0925, 0x0926, 0x0927, 0x0928, 0x0929, 0x092A, 0x092B, 0x092C, 0x092D, 0x092E, 0x092F,
    0x0930, 0x0931, 0x0932, 0x0933, 0x0934, 0x0935, 0x0936, 0x0937, 0x0938, 0x0939, 0x093A, 0x093B, 0x093C,
    0x093D, 0x093E, 0x093F, 0x0940, 0x0941, 0x0942, 0x0943, 0x0944, 0x0945, 0x0946, 0x0947, 0x0948, 0x0949,
    0x094A, 0x094B, 0x094C, 0x094D, 0x094E, 0x094F, 0x0950, 0x0951, 0x0952, 0x0953, 0x0954, 0x0955, 0x0956,
    0x0957, 0x0958, 0x0959, 0x095A, 0x095B, 0x095C, 0x095D, 0x095E, 0x095F,
];

/// `TheCamera.m_bCameraPersistTrack` (0xB6F028 + 0xCEF).
const CAMERA_PERSIST_TRACK: usize = 0xB6FD17;
/// `TheCamera.m_bCameraPersistPosition` (0xB6F028 + 0xCEE).
const CAMERA_PERSIST_POSITION: usize = 0xB6FD16;
/// `TheCamera` byte +0xCED (CAMERA_PERSIST_FOV) and +0xCF0 (SET_CINEMA_CAMERA).
const CAMERA_PERSIST_FOV: usize = 0xB6FD15;
const CAMERA_CINEMA: usize = 0xB6FD18;
/// `CTheScripts::StreamedScripts` (`CStreamedScripts`) 0xA47B60; `m_aScripts[i]` (0x20 bytes) byte +4 = instance count.
const STREAMED_SCRIPTS: usize = 0xA47B60;
const STREAMED_SCRIPT_INSTANCES: u32 = 0xA47B64;
/// `CTheScripts::ScriptsForBrains` 0xA90CF0.
const SCRIPTS_FOR_BRAINS: usize = 0xA90CF0;
/// `CHudColours HudColour` 0xBAB22C.
const HUD_COLOUR: usize = 0xBAB22C;
/// `ThePaths` 0x96F050.
const THE_PATHS: usize = 0x96F050;
/// `AudioEngine` 0xB6BC90.
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `CPedGroups::ms_groups[0]` (`CPedGroup`, 0x2D4 bytes): membership at +4 (0xC09928), follow-status byte at +0
/// (0xC09924).
const PED_GROUPS: u32 = 0xC09924;
/// `CTheScripts::ScriptSearchLights[0]` (0x7C bytes) byte +1 (0xA94D69): clip-if-colliding.
const SEARCHLIGHT_CLIP: u32 = 0xA94D69;
/// `CGarages::aGarages[0]` (0xD8 bytes) byte +0x4E (0x96C096): bit 7 respray free.
const GARAGE_FLAGS: u32 = 0x96C096;
/// `CTheScripts::NumberOfIntroRectanglesThisFrame` (u16) 0xA44B5C; `IntroRectangles[0]` (0x3C bytes) 0xA92D68.
const NUM_SCRIPT_RECTS: usize = 0xA44B5C;
const INTRO_RECTANGLES: u32 = 0xA92D68;
/// `RsGlobal.maximumWidth` / `maximumHeight` (int32) 0xC17044 / 0xC17048.
const MAXIMUM_WIDTH: usize = 0xC17044;
const MAXIMUM_HEIGHT: usize = 0xC17048;
/// `.rdata` 0x859520: 1/640; 0x859524: 1/448; 0x8595EC: pi/180 (0x3C8EFA35); 0x858B8C: 0.5f; 0x858B50: 0.0f
const K_1_640: u32 = 0x3ACC_CCCD;
const K_1_448: u32 = 0x3B12_4925;
const K_DEG_TO_RAD: u32 = 0x3C8E_FA35;
const K_HALF: u32 = 0x3F00_0000;
const K_ZERO: u32 = 0;
/// `.rdata` 0x859EE0: "DUMMY"
const DUMMY: usize = 0x859EE0;
/// `FxSystem_c* g_fxMan`-style smoke system pointer read by 095C (0xA9AE20).
const SMOKE_FX_SYSTEM: usize = 0xA9AE20;
/// `MI_PICKUP_CAMERA` / `MI_PICKUP_HORSESHOE` / `MI_PICKUP_OYSTER` (u16 model ids in `.data`).
const MI_SNAPSHOT: usize = 0x8CD5D8;
const MI_HORSESHOE: usize = 0x8CD754;
const MI_OYSTER: usize = 0x8CD750;
/// Script text-message formatting globals written by 0912 (u16 0xA44B64, byte 0xA44B66, u16 0xA44B60).
const MSG_FORMAT_WIDTH: usize = 0xA44B64;
const MSG_FORMAT_FLAG: usize = 0xA44B66;
const MSG_FORMAT_X: usize = 0xA44B60;

orig! {
    /// `CCamera::VectorTrackLinear(CVector& to, CVector& from, float duration, bool ease)` (0x50D1D0)
    fn vector_track_linear(cam: GPtr<u8>, to: GPtr<u32>, from: GPtr<u32>, duration: f32, ease: u32) = thiscall 0x50D1D0 => unported;
    /// `CCamera::VectorMoveLinear(CVector& to, CVector& from, float duration, bool ease)` (0x50D160)
    fn vector_move_linear(cam: GPtr<u8>, to: GPtr<u32>, from: GPtr<u32>, duration: f32, ease: u32) = thiscall 0x50D160 => unported;
    /// `CCamera::InitialiseScriptableComponents()` (0x50D2D0)
    fn initialise_scriptable_components(cam: GPtr<u8>) = thiscall 0x50D2D0 => unported;
    /// `CExplosion::AddExplosion(CEntity* victim, CPed* creator, eExplosionType, CVector pos, uint32 lifetime,
    /// bool makeSound, float camShake, bool visibility)` (0x736A50; the vector by value)
    fn add_explosion(victim: u32, creator: u32, ty: u32, x: u32, y: u32, z: u32, lifetime: u32, make_sound: u32, cam_shake: u32, visibility: u32) = cdecl 0x736A50 => unported;
    /// `CPostEffects::ScriptHeatHazeFXSwitch(bool)` (0x701160)
    fn script_heat_haze_fx_switch(on: u32) = cdecl 0x701160 => unported;
    /// `CHud::HelpMessageDisplayed()` (0x588B50), AL
    fn help_message_displayed() -> u8 = cdecl 0x588B50 => unported;
    /// `CDarkel::CheckDamagedWeaponType(int32 damageWeaponId, int32 expectedDamageWeaponId)` (0x43D9E0), AL
    fn check_damaged_weapon_type(got: i32, want: u32) -> u8 = cdecl 0x43D9E0 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: u32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CHudColours::GetRGB(uint8)` (0x58FEA0): the CRGBA through the hidden result pointer
    fn hud_get_rgb(h: GPtr<u8>, out: GPtr<u8>, idx: u32) -> GPtr<u8> = thiscall 0x58FEA0 => unported;
    /// `CObject::LockDoor()` (0x59F5C0)
    fn object_lock_door(o: GPtr<u8>) = thiscall 0x59F5C0 => unported;
    /// `CEntity` virtual slot 4 (+0x10, `SetIsStatic(bool)`)
    fn entity_set_is_static(e: GPtr<u8>, b: u32) = thiscall vslot 0x10 => unported;
    /// `CTheZones::FindZoneByLabel(const char*, eZoneType)` (0x572C40), AX
    fn find_zone_by_label(label: GPtr<u8>, ty: u32) -> u16 = cdecl 0x572C40 => unported;
    /// `CGangWars::SetSpecificZoneToTriggerGangWar(int32)` (0x444010)
    fn set_specific_zone_to_trigger_gang_war(z: i32) = cdecl 0x444010 => unported;
    /// `CGangWars::ClearSpecificZonesToTriggerGangWar()` (0x443FF0)
    fn clear_specific_zones_to_trigger_gang_war() = cdecl 0x443FF0 => unported;
    /// `CMenuSystem::SetActiveMenuItem(uint8, uint8)` (0x5820C0)
    fn set_active_menu_item(menu: u32, item: u32) = cdecl 0x5820C0 => unported;
    /// `CStreamedScripts::GetProperIndexFromIndexUsedByScript(int16)` (0x470810), AX
    fn get_proper_index_from_index_used_by_script(m: GPtr<u8>, i: u32) -> u16 = thiscall 0x470810 => unported;
    /// 0x4700E0 (cdecl (index); tail jump to `CStreaming::SetMissionDoesntRequireModel(index + 0x6676)`)
    fn set_mission_doesnt_require_streamed_script(i: i32) = cdecl 0x4700E0 => unported;
    /// 0x40C1C0 (cdecl (index); tail jump to `CStreaming::RemoveModel(index + 0x6676)`)
    fn remove_streamed_script(i: i32) = cdecl 0x40C1C0 => unported;
    /// `CStreamedScripts::StartNewStreamedScript(int32)` (0x470890)
    fn start_new_streamed_script(m: GPtr<u8>, i: i32) -> Scr = thiscall 0x470890 => unported;
    /// `CWeather::SetWeatherToAppropriateTypeNow()` (0x72A790)
    fn set_weather_to_appropriate_type_now() = cdecl 0x72A790 => unported;
    /// `CAudioZones::SwitchAudioZone(char*, bool)` (0x508320)
    fn switch_audio_zone(name: GPtr<u8>, on: u32) = cdecl 0x508320 => unported;
    /// Unnamed thiscall at 0x41BDD0 (a `.HOODLUM` stub; the vehicle's engine-on setter) taking a bool
    fn vehicle_41bdd0(v: GPtr<u8>, b: u32) = thiscall 0x41BDD0 => unported;
    /// The closest-attractor search at 0x6002F0 (cdecl, 9 dwords: `(CVector* pos, float range, int32 model,
    /// int32 5, char* name, bool, C2dEffect* inUse, C2dEffect** outEffect, CEntity** outEntity)`)
    fn find_closest_attractor(
        pos: GPtr<u32>, range: u32, model: i32, five: u32, name: GPtr<u8>, b: u32, in_use: u32,
        out_effect: GPtr<u32>, out_entity: GPtr<u32>
    ) = cdecl 0x6002F0 => unported;
    /// `GetPedAttractorManager()` (0x5EE190)
    fn get_ped_attractor_manager() -> GPtr<u8> = cdecl 0x5EE190 => unported;
    /// `CPedAttractorManager::GetPedUsingEffect(const C2dEffect*, const CEntity*)` (0x5EBE50)
    fn get_ped_using_effect(m: GPtr<u8>, fx: u32, e: u32) -> GPtr<u8> = thiscall 0x5EBE50 => unported;
    /// `CPathFind::SwitchRoadsOffInArea(x1, x2, y1, y2, z1, z2, bool off, bool cars, bool force)` (0x452C80); raw floats
    fn switch_roads_off_in_area(
        p: GPtr<u8>, x1: u32, x2: u32, y1: u32, y2: u32, z1: u32, z2: u32, off: u32, cars: u32, force: u32
    ) = thiscall 0x452C80 => unported;
    /// `CPathFind::SwitchPedRoadsOffInArea(x1, x2, y1, y2, z1, z2, bool off, bool)` (0x452F00); raw floats
    fn switch_ped_roads_off_in_area(
        p: GPtr<u8>, x1: u32, x2: u32, y1: u32, y2: u32, z1: u32, z2: u32, off: u32, b: u32
    ) = thiscall 0x452F00 => unported;
    /// `CCamera::LerpFOV(float from, float to, float duration, bool ease)` (0x50D280); from/to raw
    fn lerp_fov(cam: GPtr<u8>, from: u32, to: u32, duration: f32, ease: u32) = thiscall 0x50D280 => unported;
    /// `CPlane::SwitchAmbientPlanes(bool)` (0x6CCC50)
    fn switch_ambient_planes(on: u32) = cdecl 0x6CCC50 => unported;
    /// `CPostEffects::ScriptDarknessFilterSwitch(bool, int32)` (0x701170)
    fn script_darkness_filter_switch(on: u32, alpha: u32) = cdecl 0x701170 => unported;
    /// `CScriptsForBrains::AddNewScriptBrain(int16 index, int16 model, uint16 priority, int8 attachType, int8
    /// type, float radius)` (0x46A930); the radius raw
    fn add_new_script_brain(b: GPtr<u8>, i: i32, model: u32, priority: u32, attach: u32, ty: u32, radius: u32) = thiscall 0x46A930 => unported;
    /// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0)
    fn get_actual_script_thing_index(r: u32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `CPedGroupMembership::GetMember(int32)` (0x5F69B0)
    fn group_get_member(m: GPtr<u8>, i: u32) -> GPtr<u8> = thiscall 0x5F69B0 => unported;
    /// `CWaterLevel::GetWaterLevel(float x, float y, float z, float* level, bool touching, CVector* normal)`
    /// (0x6EB690), AL; floats raw
    fn get_water_level(x: u32, y: u32, z: u32, level: GPtr<u32>, touching: u32, normal: u32) -> u8 = cdecl 0x6EB690 => unported;
    /// `CWaterLevel::GetWaterLevelNoWaves(float x, float y, float z, float* level, float*, float*)` (0x6E8580), AL
    fn get_water_level_no_waves(x: u32, y: u32, z: u32, level: GPtr<u32>, a: u32, b: u32) -> u8 = cdecl 0x6E8580 => unported;
    /// `CCamera::VectorMoveRunning()` (0x4748A0) / `VectorTrackRunning()` (0x474870); the whole EAX is pushed
    fn vector_move_running(cam: GPtr<u8>) -> u32 = thiscall 0x4748A0 => unported;
    fn vector_track_running(cam: GPtr<u8>) -> u32 = thiscall 0x474870 => unported;
    /// CRT `_stricmp` (0x8229B6)
    fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// `CRGBA::CRGBA(uint8, uint8, uint8, uint8)` (0x7170C0), returns this
    fn rgba_ctor(c: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// Unnamed thiscall at 0x463370 (a `.HOODLUM` stub) taking `const CRGBA*`: the script rectangle's colour copy
    fn rgba_set_463370(c: GPtr<u8>, from: GPtr<u8>) = thiscall 0x463370 => unported;
    /// `CPhysical::AttachEntityToEntity(CEntity*, CVector offset, CVector rotation)` (0x54D570); vectors by value
    fn attach_entity_to_entity(p: GPtr<u8>, e: GPtr<u8>, ox: u32, oy: u32, oz: u32, rx: u32, ry: u32, rz: u32) = thiscall 0x54D570 => unported;
    /// `CGarages::GetGarageNumberByName(const char*)` (0x447680), AX
    fn get_garage_number_by_name(name: GPtr<u8>) -> u16 = cdecl 0x447680 => unported;
    /// `CShopping::HasPlayerBought(uint32)` (0x49B5E0); the whole EAX is pushed
    fn has_player_bought(item: u32) -> u32 = cdecl 0x49B5E0 => unported;
    /// `CCamera::SetCameraDirectlyInFrontForFollowPed_ForAPed_CamOnAString(CPed*)` (0x50BE30)
    fn set_camera_directly_in_front(cam: GPtr<u8>, p: GPtr<u8>) = thiscall 0x50BE30 => unported;
    /// `CPed::Say(uint16, uint32, float, uint8, uint8, uint8)` (0x5EFFE0), AX; the float raw
    fn ped_say(p: GPtr<u8>, id: u32, a: u32, vol: u32, b: u32, c: u32, d: u32) -> u16 = thiscall 0x5EFFE0 => unported;
    /// `CAudioEngine::AttachMissionAudioToPed(uint8 slot, CPed*)` (0x507310)
    fn attach_mission_audio_to_ped(a: GPtr<u8>, slot: u32, p: GPtr<u8>) = thiscall 0x507310 => unported;
    /// `CPickups::UpdateMoneyPerDay(int32 handle, uint16 money)` (0x455680)
    fn update_money_per_day(h: u32, money: u32) = cdecl 0x455680 => unported;
    /// 0x43E090 (thiscall on the `CEntryExit`, `CVector*` out; gr `GetPositionRelativeToOutsideWorld`)
    fn entry_exit_position_43e090(e: GPtr<u8>, out: GPtr<u32>) = thiscall 0x43E090 => unported;
    /// `CPed::GetPedTalking()` (0x5EFF50), AL
    fn get_ped_talking(p: GPtr<u8>) -> u8 = thiscall 0x5EFF50 => unported;
    /// `CPed::DisablePedSpeech(int16)` (0x5EFF60) / `EnablePedSpeech()` (0x5EFF70)
    fn disable_ped_speech(p: GPtr<u8>, b: u32) = thiscall 0x5EFF60 => unported;
    fn enable_ped_speech(p: GPtr<u8>) = thiscall 0x5EFF70 => unported;
    /// `CGameLogic::SetUpSkip(CVector, float, bool, CVehicle*, bool)` (0x4423C0); the vector by value, raw
    fn set_up_skip(x: u32, y: u32, z: u32, heading: u32, a: u32, b: u32, c: u32) = cdecl 0x4423C0 => unported;
    /// `CGameLogic::ClearSkip(bool)` (0x441560)
    fn clear_skip(b: u32) = cdecl 0x441560 => unported;
    /// `CAudioEngine::PreloadBeatTrack(int16)` (0x507F40)
    fn preload_beat_track(a: GPtr<u8>, t: u32) = thiscall 0x507F40 => unported;
    /// `CAudioEngine::GetBeatTrackStatus()` (0x507170), AL
    fn get_beat_track_status(a: GPtr<u8>) -> u8 = thiscall 0x507170 => unported;
    /// `CAudioEngine::PlayPreloadedBeatTrack(bool)` (0x507180) / `StopBeatTrack()` (0x5071A0)
    fn play_preloaded_beat_track(a: GPtr<u8>, b: u32) = thiscall 0x507180 => unported;
    fn stop_beat_track(a: GPtr<u8>) = thiscall 0x5071A0 => unported;
    /// `CStats::FindMaxNumberOfGroupMembers()` (0x559A50)
    fn find_max_number_of_group_members() -> u32 = cdecl 0x559A50 => unported;
    /// `CStats::IncrementStat(uint16 stat, float value)` (0x55C180); the float raw
    fn increment_stat(stat: u32, value: u32) = cdecl 0x55C180 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), raw floats, ST0 (read as f64: exact)
    fn find_ground_z_raw(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CPickups::GenerateNewOne(CVector, uint32 model, uint8 type, uint32 ammo, uint32 money, bool empty, const
    /// char* msg)` (0x456F20); the vector by value
    fn generate_new_one(x: u32, y: u32, z: u32, model: u32, ty: u32, ammo: u32, money: u32, empty: u32, msg: u32) -> u32 = cdecl 0x456F20 => unported;
    /// `CEntity::GetIsStatic()` (0x4633E0), AL
    fn entity_get_is_static(e: GPtr<u8>) -> u8 = thiscall 0x4633E0 => unported;
    /// `FxPrtMult_c::FxPrtMult_c(float r, g, b, a, size, angle, life)` (0x4AB290); raw floats
    fn fx_prt_mult_ctor(p: GPtr<u8>, r: u32, g: u32, b: u32, a: u32, size: u32, angle: u32, life: u32) = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(CVector* pos, CVector* vel, float time, FxPrtMult_c*, float, float, float, bool)`
    /// (0x4AA440); raw floats
    fn fx_add_particle(fx: GPtr<u8>, pos: GPtr<u32>, vel: GPtr<u32>, time: u32, prt: GPtr<u8>, a: u32, b: u32, c: u32, d: u32) = thiscall 0x4AA440 => unported;
    /// `CDoor::Open(float ratio)` (0x6F4790); raw
    fn door_open(d: GPtr<u8>, ratio: u32) = thiscall 0x6F4790 => unported;
    /// `CDamageManager::SetDoorStatus(eDoors, uint32)` (0x6C21C0)
    fn set_door_status(dm: GPtr<u8>, door: u32, status: u32) = thiscall 0x6C21C0 => unported;
    /// `CAutomobile::SetDoorDamage(eDoors, bool)` (0x6B1600)
    fn set_door_damage(v: GPtr<u8>, door: u32, b: u32) = thiscall 0x6B1600 => unported;
}

/// The VECTOR_TRACK / VECTOR_MOVE argument shape (0920 0x476BDE, 0936 0x476F9D): `CollectParameters(8)`; `from =
/// (SP0, SP1, SP2)`, `to = (SP3, SP4, SP5)`; `duration = (float)SP6` (`fild`: an integer operand, gr reads a float
/// for 0936); `ease = SP7 != 0` (pushed with stale upper bytes of ECX = SP0; the callee reads the low byte, the port
/// passes 0/1). The callee gets `(&to, &from, duration, ease)`.
fn vector_args(s: Scr) -> ([u32; 3], [u32; 3], f32, u32) {
    collect(s, 8);
    let from = [sp(0), sp(1), sp(2)];
    let to = [sp(3), sp(4), sp(5)];
    let duration = X87::from_i32(spi(6)).to_f32();
    (to, from, duration, u32::from(sp(7) != 0))
}

/// The min/max ordering of the SWITCH_*ROADS_BACK_TO_ORIGINAL cases: `fld a; fcomp b; fnstsw; test ah,0x41; jne
/// keep`, i.e. the two dwords are swapped when `a > b` (ordered; the `fld/fstp` copy of a non-NaN is exact).
#[inline]
fn order(a: u32, b: u32) -> (u32, u32) {
    if f(a) > f(b) { (b, a) } else { (a, b) }
}

/// The streamed-script index of 090F/0910/0913/0926/0928/0929: `(int32)(int16)
/// StreamedScripts.GetProperIndexFromIndexUsedByScript((u16)SP0)`.
fn streamed_index() -> i32 {
    i32::from(get_proper_index_from_index_used_by_script(GPtr::from_addr(STREAMED_SCRIPTS), sp(0) & 0xFFFF) as i16)
}

/// The "UsedObjectArray" model rule: a negative model indexes `CTheScripts::UsedObjectArray` backwards.
#[inline]
fn model_operand(v: u32) -> u32 {
    if (v as i32) < 0 {
        GPtr::<u32>::from_addr(0xA44B88u32.wrapping_sub(v.wrapping_mul(0x1C)) as usize).read()
    } else {
        v
    }
}

/// The protected store path (0x477BDF): `SP0 = edx; StoreParameters(1)`.
#[inline]
fn store_sp0(s: Scr, v: u32) {
    set_sp(0, v);
    store(s, 1);
}

/// `p.byte_add(off)` bit `mask` of a byte set to `on` (`setne; shl; xor; and; xor`).
#[inline]
fn set_byte_bit(p: GPtr<u8>, mask: u8, on: bool) {
    let old = p.read();
    p.write(if on { old | mask } else { old & !mask });
}

/// `p` dword bit `mask` set to `on`.
#[inline]
fn set_dword_bit(p: GPtr<u32>, mask: u32, on: bool) {
    let old = p.read();
    p.write(if on { old | mask } else { old & !mask });
}

/// 08FF HAS_OBJECT_BEEN_DAMAGED_BY_WEAPON (0x476346): `obj = OBJ(SP0)`; flag 0 when null; weapon SP1 0x38/0x39
/// (any melee / any weapon): `CheckDamagedWeaponType((i8)obj->m_nLastWeaponDamage (+0x148), SP1)`; otherwise
/// `(i8)+0x148 == SP1`.
fn has_object_been_damaged_by_weapon(s: Scr) {
    collect(s, 2);
    let o = object_at_ref(pool(OBJECT_POOL), spi(0));
    let mut b = false;
    if !o.is_null() {
        let w = sp(1);
        let last = i32::from(o.byte_add(0x148).read() as i8);
        b = if w == 0x38 || w == 0x39 { check_damaged_weapon_type(last, w) != 0 } else { last as u32 == w };
    }
    set_flag(s, b);
}

/// 0905 LOCK_DOOR (0x4764AF): `obj = OBJ(SP0)` (no null check); SP1 != 0: `obj->LockDoor()`; else physical flags
/// +0x40 `&= ~0xC`, turn speed (+0x50) = 0, `SetIsStatic(true)` (virtual slot 4).
fn lock_door(s: Scr) {
    collect(s, 2);
    let o = object_at_ref(pool(OBJECT_POOL), spi(0));
    if sp(1) != 0 {
        object_lock_door(o);
        return;
    }
    let fl = o.byte_add(0x40).cast::<u32>();
    fl.write(fl.read() & 0xFFFF_FFF3);
    for i in 0..3 {
        o.byte_add(0x50 + 4 * i).cast::<u32>().write(0);
    }
    entity_set_is_static(o, 1);
}

/// 091C GET_USER_OF_CLOSEST_MAP_ATTRACTOR (0x4768EC): `CollectParameters(5)` (pos, range, model), 8-byte name
/// (zeroed 16-byte buffer); model rule (0 → -1, < 0 → used-object model); both outputs zeroed; `0x6002F0(&pos,
/// SP3, model, 5, name, 0, 0, &effect, &entity)`; `SP0 = -1`; with an effect, `ped = GetPedAttractorManager()->
/// GetPedUsingEffect(effect, entity)` and `SP0 = PEDREF(ped)` when non-null; `StoreParameters(1)`.
fn get_user_of_closest_map_attractor(s: Scr) {
    collect(s, 5);
    let mut pos = [sp(0), sp(1), sp(2)];
    let range = sp(3);
    let mut model = spi(4);
    let mut name = [0u8; 16];
    let np = GPtr::from_raw(name.as_mut_ptr());
    read_text_label(s, np, 8);
    if model == 0 {
        model = -1;
    } else if model < 0 {
        model = model_operand(model as u32) as i32;
    }
    let mut effect = [0u32; 1];
    let mut entity = [0u32; 1];
    find_closest_attractor(
        GPtr::from_raw(pos.as_mut_ptr()),
        range,
        model,
        5,
        np,
        0,
        0,
        GPtr::from_raw(effect.as_mut_ptr()),
        GPtr::from_raw(entity.as_mut_ptr()),
    );
    set_sp(0, u32::MAX);
    if effect[0] != 0 {
        let m = get_ped_attractor_manager();
        let p = get_ped_using_effect(m, effect[0], entity[0]);
        if !p.is_null() {
            set_sp(0, ped_get_ref(pool(PED_POOL), p) as u32);
        }
    }
    store(s, 1);
}

/// 0937 DRAW_WINDOW (0x477017): `CollectParameters(4)` (x1, y1, x2, y2), 8-byte label (zeroed buffer), text = the
/// label unless "DUMMY" (`_stricmp`; byte loop up to the terminator), `CollectParameters(1)` (style). Rectangle
/// `r = IntroRectangles[n]`, `n = u16 [0xA44B5C]`: +0 = 2, +6 (u16) = 0xFFFF; x87 (PC=24): `A = fild maxWidth *
/// 1/640`, `B = fild maxHeight * 1/448` (registers); +8 = x1 * A, +0xC = y1 * B, +0x10 = A * x2, +0x14 = B * y2;
/// +0x18 = 0; a stack `CRGBA(0, 0, 0, 0xBE)` copied with 0x463370 into +0x1C; the text copied to +0x20 up to the
/// terminator; `n += 1`; byte +0x2A = 0; +0x34 = 1; +0x38 = style.
fn draw_window(s: Scr) {
    collect(s, 4);
    let (x1, y1, x2, y2) = (spf(0), spf(1), spf(2), spf(3));
    let mut label = [0u8; 16];
    let lp = GPtr::from_raw(label.as_mut_ptr());
    read_text_label(s, lp, 8);
    let mut text = [0u8; 16];
    if stricmp(lp, GPtr::from_addr(DUMMY)) != 0 {
        let mut i = 0;
        loop {
            let c = label[i];
            text[i] = c;
            i += 1;
            if c == 0 {
                break;
            }
        }
    }
    collect(s, 1);
    let n = GPtr::<u16>::from_addr(NUM_SCRIPT_RECTS);
    let r = GPtr::<u8>::from_addr(INTRO_RECTANGLES.wrapping_add(u32::from(n.read()) * 0x3C) as usize);
    let a = X87::from_i32(GPtr::<i32>::from_addr(MAXIMUM_WIDTH).read()).fmul_m(f(K_1_640));
    r.cast::<u32>().write(2);
    r.byte_add(6).cast::<u16>().write(0xFFFF);
    let style = sp(0);
    r.byte_add(8).cast::<f32>().write(X87::from_f32(x1).fmul(a).to_f32());
    let b = X87::from_i32(GPtr::<i32>::from_addr(MAXIMUM_HEIGHT).read()).fmul_m(f(K_1_448));
    r.byte_add(0xC).cast::<f32>().write(X87::from_f32(y1).fmul(b).to_f32());
    r.byte_add(0x10).cast::<f32>().write(a.fmul_m(x2).to_f32());
    r.byte_add(0x14).cast::<f32>().write(b.fmul_m(y2).to_f32());
    r.byte_add(0x18).cast::<u32>().write(0);
    let mut col = [0u8; 4];
    let cp = rgba_ctor(GPtr::from_raw(col.as_mut_ptr()), 0, 0, 0, 0xBE);
    rgba_set_463370(r.byte_add(0x1C), cp);
    let mut i = 0;
    loop {
        let c = text[i];
        r.byte_add(0x20 + i as isize).write(c);
        i += 1;
        if c == 0 {
            break;
        }
    }
    n.write(n.read().wrapping_add(1));
    r.byte_add(0x2A).write(0);
    r.byte_add(0x34).cast::<u32>().write(1);
    r.byte_add(0x38).cast::<u32>().write(style);
}

/// 0939 ATTACH_CAR_TO_OBJECT (0x477175): `CollectParameters(8)`; `VEH(SP0)->AttachEntityToEntity(OBJ(SP1),
/// CVector(SP2, SP3, SP4), CVector(SP5 * k, SP6 * k, SP7 * k))`, `k` = pi/180 (`fld; fmul m32; fstp`).
fn attach_car_to_object(s: Scr) {
    collect(s, 8);
    let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
    let o = object_at_ref(pool(OBJECT_POOL), spi(1));
    let rot = |i| X87::from_f32(spf(i)).fmul_m(f(K_DEG_TO_RAD)).to_f32().to_bits();
    let (rx, ry, rz) = (rot(5), rot(6), rot(7));
    attach_entity_to_entity(v, o, sp(2), sp(3), sp(4), rx, ry, rz);
}

/// 094C GET_POSITION_OF_ENTRY_EXIT_CHAR_USED (0x47760C): `ee = PED(SP0)->m_pEnex (+0x78C)`; pos and heading start
/// 0; with `ee`: dword +0x38 != 0 → pos = (+0x20, +0x24, +0x28), heading = +0x2C * pi/180 (`fstp` f32); else
/// `0x43E090(ee, &pos)` (heading stays 0); `SP0..SP3 = pos, heading`. Without `ee` the parameters are not written.
/// `StoreParameters(4)`.
fn get_position_of_entry_exit_char_used(s: Scr) {
    collect(s, 1);
    let p = ped_at_ref(pool(PED_POOL), spi(0));
    let ee = p.byte_add(0x78C).cast::<GPtr<u8>>().read();
    let mut pos = [0u32; 3];
    let mut heading = 0u32;
    if !ee.is_null() {
        if ee.byte_add(0x38).cast::<u32>().read() != 0 {
            heading = X87::from_f32(ee.byte_add(0x2C).cast::<f32>().read())
                .fmul_m(f(K_DEG_TO_RAD))
                .to_f32()
                .to_bits();
            for (i, v) in pos.iter_mut().enumerate() {
                *v = ee.byte_add(0x20 + 4 * i as isize).cast::<u32>().read();
            }
        } else {
            entry_exit_position_43e090(ee, GPtr::from_raw(pos.as_mut_ptr()));
        }
        set_sp(0, pos[0]);
        set_sp(1, pos[1]);
        set_sp(2, pos[2]);
        set_sp(3, heading);
    }
    store(s, 4);
}

/// 0958 CREATE_SNAPSHOT_PICKUP / 0959 CREATE_HORSESHOE_PICKUP / 095A CREATE_OYSTER_PICKUP (0x4778AA, shared):
/// `CollectParameters(3)`; by `command - 0x958`: 0 → model u16 [0x8CD5D8], type 0x14, stat 0xE8; 1 → u16
/// [0x8CD754], type 3, stat 0xF2; otherwise → u16 [0x8CD750], type 3, stat 0xF4; `IncrementStat(stat, 1.0f)`;
/// z = ground z + 0.5 (`fadd m32`, stored f32) when `z <= -100` (ordered, `test ah,0x41; jp`); `SP0 =
/// GenerateNewOne(CVector(x, y, z), model, type, 0, 0, 0, null)`; `StoreParameters(1)`.
fn create_collectable_pickup(s: Scr, command: u32) {
    collect(s, 3);
    let (x, y) = (sp(0), sp(1));
    let mut z = sp(2);
    let (model, ty, stat) = match command.wrapping_sub(0x958) {
        0 => (GPtr::<u16>::from_addr(MI_SNAPSHOT).read(), 0x14, 0xE8),
        1 => (GPtr::<u16>::from_addr(MI_HORSESHOE).read(), 3, 0xF2),
        _ => (GPtr::<u16>::from_addr(MI_OYSTER).read(), 3, 0xF4),
    };
    increment_stat(stat, 0x3F80_0000);
    if f(z) <= f(K_MINUS_100) {
        z = X87::from_f64_exact(find_ground_z_raw(x, y)).fadd_m(f(K_HALF)).to_f32().to_bits();
    }
    let h = generate_new_one(x, y, z, u32::from(model), ty, 0, 0, 0, 0);
    set_sp(0, h);
    store(s, 1);
}

/// 095C ADD_SMOKE_PARTICLE (0x4779EC): `CollectParameters(12)`; pos = (SP0, SP1, SP2), vel = (SP3, SP4, SP5); a
/// stack `FxPrtMult_c(SP6, SP7, SP8, SP9, SP10, 1.0f, SP11)` (zeroed buffer); `[0xA9AE20]->AddParticle(&pos, &vel,
/// 0, &prt, -1.0f, 1.2f, 0.6f, 0)`.
fn add_smoke_particle(s: Scr) {
    collect(s, 12);
    let mut pos = [sp(0), sp(1), sp(2)];
    let mut vel = [sp(3), sp(4), sp(5)];
    let mut prt = [0u32; 8];
    let pp = GPtr::from_raw(prt.as_mut_ptr()).cast::<u8>();
    fx_prt_mult_ctor(pp, sp(6), sp(7), sp(8), sp(9), sp(10), 0x3F80_0000, sp(11));
    let fx = GPtr::<GPtr<u8>>::from_addr(SMOKE_FX_SYSTEM).read();
    fx_add_particle(
        fx,
        GPtr::from_raw(pos.as_mut_ptr()),
        GPtr::from_raw(vel.as_mut_ptr()),
        0,
        pp,
        0xBF80_0000,
        0x3F99_999A,
        0x3F19_999A,
        0,
    );
}

/// 095E CONTROL_CAR_DOOR (0x477AE5): `CollectParameters(4)`; `veh = VEH(SP0)` (no null check); `ratio = (float)SP3`
/// (`fild; fst`); door SP1 < 6 (signed): `m_doors[door] (+0x5B8 + 0x18 * door).Open(ratio)` unless `ratio < 0`
/// or unordered (`test ah,1`), `m_damageManager (+0x5A0).SetDoorStatus(door, SP2)`, `SetDoorDamage(door, false)`;
/// otherwise the extra door +0x70C `.Open(ratio)` under the same test and byte +0x717 = (u8)SP2.
fn control_car_door(s: Scr) {
    collect(s, 4);
    let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
    let r = X87::from_i32(spi(3));
    let ratio = r.to_f32().to_bits();
    let open = !matches!(r.fcom(X87::constant(K_ZERO)), Cmp::Less | Cmp::Unordered);
    let door = sp(1);
    if (door as i32) < 6 {
        if open {
            door_open(v.byte_add(0x5B8 + 0x18 * door as i32 as isize), ratio);
        }
        set_door_status(v.byte_add(0x5A0), door, sp(2));
        set_door_damage(v, door, 0);
    } else {
        if open {
            door_open(v.byte_add(0x70C), ratio);
        }
        v.byte_add(0x717).write(sp(2) as u8);
    }
}

/// `CRunningScript::ProcessCommands2300To2399(int32 command)` (0x4762D0, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x4762D0, conv = thiscall, name = "CRunningScript::ProcessCommands2300To2399", expect = "81 EC 84 00 00 00", this = "CRunningScript")]
pub fn process_commands_2300_to_2399(s: Scr, command: u32) -> i8 {
    match command {
        // Cases the jump table sends straight to the epilogue 0x477BEE (AL = 0, no operand read): 08FC
        // GET_MENU_POSITION, 0902 SET_OBJECT_BEEN_PHOTOGRAPHED_FLAG, 0903 SET_CHAR_BEEN_PHOTOGRAPHED_FLAG, 090A
        // IS_PLAYBACK_FOR_CAR_PAUSED, 090B TRIGGER_PED_BOUNCE, 0911 REGISTER_STREAMED_SCRIPT, 091B
        // RESET_LATEST_CONSOLE_COMMAND, 0921 CAMERA_SET_SHAKE_SIMULATION, 0927
        // ALLOCATE_STREAMED_SCRIPT_TO_PED_GENERATOR, 092A SET_PLAYER_CAN_BE_DAMAGED, 092C/092D
        // GET/SET_PLAYERS_GANG_IN_CAR_ACTIVE, 0932 CAMERA_IS_FOV_RUNNING, 0935 CAMERA_IS_SHAKE_RUNNING, 0938
        // CLEAR_ALL_QUEUED_DIALOGUE, 093C SET_ONSCREEN_COUNTER_COLOUR, 093F IS_FIRE_BUTTON_PRESSED, 0943
        // SET_CAMERA_BEHIND_CHAR. gr: 0903/0921/0932/0935/0943 `unimplemented` (-1), the exe returns 0.
        0x08FC | 0x0902 | 0x0903 | 0x090A | 0x090B | 0x0911 | 0x091B | 0x0921 | 0x0927 => {}
        0x092A | 0x092C | 0x092D | 0x0932 | 0x0935 | 0x0938 | 0x093C | 0x093F | 0x0943 => {}
        // 08FD SET_HEATHAZE_EFFECT (0x4762F8): `ScriptHeatHazeFXSwitch(SP0 != 0)`
        0x08FD => {
            collect(s, 1);
            script_heat_haze_fx_switch(u32::from(sp(0) != 0));
        }
        // 08FE IS_HELP_MESSAGE_BEING_DISPLAYED (0x476323): flag = AL of `CHud::HelpMessageDisplayed()`
        0x08FE => flag(s, u32::from(help_message_displayed())),
        // 08FF HAS_OBJECT_BEEN_DAMAGED_BY_WEAPON (0x476346)
        0x08FF => has_object_been_damaged_by_weapon(s),
        // 0900 CLEAR_OBJECT_LAST_WEAPON_DAMAGE (0x4763BB): `OBJ(SP0)->m_nLastWeaponDamage (+0x148) = -1` when non-null
        0x0900 => {
            collect(s, 1);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            if !o.is_null() {
                o.byte_add(0x148).write(0xFF);
            }
        }
        // 0901 SET_PLAYER_JUMP_BUTTON (0x4763F2): `CPad::GetPad(SP0)` byte +0x11E = SP1 == 0
        0x0901 => {
            collect(s, 2);
            let on = sp(1) != 0;
            get_pad(sp(0)).byte_add(0x11E).write(u8::from(!on));
        }
        // 0904 GET_HUD_COLOUR (0x47644C): `c = HudColour.GetRGB((u8)SP0)`; SP0..SP3 = r, g, b, a; StoreParameters(4)
        0x0904 => {
            collect(s, 1);
            let mut c = [0u8; 4];
            hud_get_rgb(GPtr::from_addr(HUD_COLOUR), GPtr::from_raw(c.as_mut_ptr()), sp(0) & 0xFF);
            set_sp(1, u32::from(c[1]));
            set_sp(0, u32::from(c[0]));
            set_sp(2, u32::from(c[2]));
            set_sp(3, u32::from(c[3]));
            store(s, 4);
        }
        // 0905 LOCK_DOOR (0x4764AF)
        0x0905 => lock_door(s),
        // 0906 SET_OBJECT_MASS (0x47652F) / 0908 SET_OBJECT_TURN_MASS (0x4765A0): `OBJ(SP0)` +0x8C / +0x90 = SP1
        // (no null check)
        0x0906 | 0x0908 => {
            collect(s, 2);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            let off = if command == 0x0906 { 0x8C } else { 0x90 };
            o.byte_add(off).cast::<u32>().write(sp(1));
        }
        // 0907 GET_OBJECT_MASS (0x476563) / 0909 GET_OBJECT_TURN_MASS (0x4765D5): SP0 = `OBJ(SP0)` +0x8C / +0x90
        0x0907 | 0x0909 => {
            collect(s, 1);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            let off = if command == 0x0907 { 0x8C } else { 0x90 };
            set_sp(0, o.byte_add(off).cast::<u32>().read());
            store(s, 1);
        }
        // 090C SET_SPECIFIC_ZONE_TO_TRIGGER_GANG_WAR (0x476612): `SetSpecificZoneToTriggerGangWar((i16)
        // FindZoneByLabel(label, 2))`
        0x090C => {
            let mut label = [0u8; 16];
            let lp = GPtr::from_raw(label.as_mut_ptr());
            read_text_label(s, lp, 8);
            let z = find_zone_by_label(lp, 2);
            set_specific_zone_to_trigger_gang_war(i32::from(z as i16));
        }
        // 090D CLEAR_SPECIFIC_ZONES_TO_TRIGGER_GANG_WAR (0x476646)
        0x090D => clear_specific_zones_to_trigger_gang_war(),
        // 090E SET_ACTIVE_MENU_ITEM (0x476659): `SetActiveMenuItem((u8)SP0, (u8)SP1)`
        0x090E => {
            collect(s, 2);
            set_active_menu_item(sp(0) & 0xFF, sp(1) & 0xFF);
        }
        // 090F MARK_STREAMED_SCRIPT_AS_NO_LONGER_NEEDED (0x47668A): `SP0 = i`; `0x4700E0(i)`
        0x090F => {
            collect(s, 1);
            let i = streamed_index();
            set_sp(0, i as u32);
            set_mission_doesnt_require_streamed_script(i);
        }
        // 0910 REMOVE_STREAMED_SCRIPT (0x4766C5): `SP0 = i`; `0x40C1C0(i)`
        0x0910 => {
            collect(s, 1);
            let i = streamed_index();
            set_sp(0, i as u32);
            remove_streamed_script(i);
        }
        // 0912 SET_MESSAGE_FORMATTING (0x476701): u16 [0xA44B64] = SP1, byte [0xA44B66] = SP0 != 0, u16 [0xA44B60] = SP2
        0x0912 => {
            collect(s, 3);
            GPtr::<u16>::from_addr(MSG_FORMAT_WIDTH).write(sp(1) as u16);
            GPtr::<u8>::from_addr(MSG_FORMAT_FLAG).write(u8::from(sp(0) != 0));
            GPtr::<u16>::from_addr(MSG_FORMAT_X).write(sp(2) as u16);
        }
        // 0913 START_NEW_STREAMED_SCRIPT (0x476742): `SP0 = i`; `ReadParametersForNewlyStartedScript(
        // StreamedScripts.StartNewStreamedScript(i))`
        0x0913 => {
            collect(s, 1);
            let i = streamed_index();
            set_sp(0, i as u32);
            let n = start_new_streamed_script(GPtr::from_addr(STREAMED_SCRIPTS), i);
            vm::read_parameters_for_newly_started_script_call(s, n);
        }
        // 0914 REGISTER_STREAMED_SCRIPT_INTERNAL (0x476788): the operand is read, nothing else
        0x0914 => collect(s, 1),
        // 0915 SET_WEATHER_TO_APPROPRIATE_TYPE_NOW (0x47679F)
        0x0915 => set_weather_to_appropriate_type_now(),
        // 0916 WINCH_CAN_PICK_OBJECT_UP (0x4767B2): `OBJ(SP0)` +0x140 bit 0x40000 = SP1 != 0 (no null check)
        0x0916 => {
            collect(s, 2);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            set_dword_bit(o.byte_add(0x140).cast::<u32>(), 0x4_0000, sp(1) != 0);
        }
        // 0917 SWITCH_AUDIO_ZONE (0x476810): label, `CollectParameters(1)`, `SwitchAudioZone(label, SP0 != 0)`
        0x0917 => {
            let mut label = [0u8; 16];
            let lp = GPtr::from_raw(label.as_mut_ptr());
            read_text_label(s, lp, 8);
            collect(s, 1);
            switch_audio_zone(lp, u32::from(sp(0) != 0));
        }
        // 0918 SET_CAR_ENGINE_ON (0x47684E): `0x41BDD0(VEH(SP0), SP1 != 0)`
        0x0918 => {
            collect(s, 2);
            let on = u32::from(sp(1) != 0);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            vehicle_41bdd0(v, on);
        }
        // 0919 SET_CAR_LIGHTS_ON (0x47688A): `VEH(SP0)` byte +0x428 bit 6 = SP1 != 0 (no null check)
        0x0919 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            set_byte_bit(v.byte_add(0x428), 0x40, sp(1) != 0);
        }
        // 091A GET_LATEST_CONSOLE_COMMAND (0x4768D5): `GetPointerToScriptVariable(2)`, result unused
        0x091A => {
            var_ptr(s, 2);
        }
        // 091C GET_USER_OF_CLOSEST_MAP_ATTRACTOR (0x4768EC)
        0x091C => get_user_of_closest_map_attractor(s),
        // 091D SWITCH_ROADS_BACK_TO_ORIGINAL (0x4769F3): each axis ordered, `ThePaths.SwitchRoadsOffInArea(x1, x2,
        // y1, y2, z1, z2, false, true, true)`
        0x091D => {
            collect(s, 6);
            let (x1, x2) = order(sp(0), sp(3));
            let (y1, y2) = order(sp(1), sp(4));
            let (z1, z2) = order(sp(2), sp(5));
            switch_roads_off_in_area(GPtr::from_addr(THE_PATHS), x1, x2, y1, y2, z1, z2, 0, 1, 1);
        }
        // 091E SWITCH_PED_ROADS_BACK_TO_ORIGINAL (0x476ACB): `ThePaths.SwitchPedRoadsOffInArea(x1, x2, y1, y2, z1,
        // z2, false, true)`
        0x091E => {
            collect(s, 6);
            let (x1, x2) = order(sp(0), sp(3));
            let (y1, y2) = order(sp(1), sp(4));
            let (z1, z2) = order(sp(2), sp(5));
            switch_ped_roads_off_in_area(GPtr::from_addr(THE_PATHS), x1, x2, y1, y2, z1, z2, 0, 1);
        }
        // 091F GET_PLANE_UNDERCARRIAGE_POSITION (0x476BA1): SP0 = `VEH(SP0)` dword +0x9CC
        0x091F => {
            collect(s, 1);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            set_sp(0, v.byte_add(0x9CC).cast::<u32>().read());
            store(s, 1);
        }
        // 0920 CAMERA_SET_VECTOR_TRACK (0x476BDE)
        0x0920 => {
            let (to, from, d, ease) = vector_args(s);
            let (tp, fp) = (
                GPtr::from_raw(to.as_ptr().cast_mut()),
                GPtr::from_raw(from.as_ptr().cast_mut()),
            );
            vector_track_linear(GPtr::from_addr(THE_CAMERA), tp, fp, d, ease);
        }
        // 0922 CAMERA_SET_LERP_FOV (0x476C58): `TheCamera.LerpFOV(SP0, SP1, (float)SP2 (fild), SP3 != 0)`
        0x0922 => {
            collect(s, 4);
            let d = X87::from_i32(spi(2)).to_f32();
            lerp_fov(GPtr::from_addr(THE_CAMERA), sp(0), sp(1), d, u32::from(sp(3) != 0));
        }
        // 0923 SWITCH_AMBIENT_PLANES (0x476C9B)
        0x0923 => {
            collect(s, 1);
            switch_ambient_planes(u32::from(sp(0) != 0));
        }
        // 0924 SET_DARKNESS_EFFECT (0x476CC6): `ScriptDarknessFilterSwitch(SP0 != 0, SP1)`
        0x0924 => {
            collect(s, 2);
            script_darkness_filter_switch(u32::from(sp(0) != 0), sp(1));
        }
        // 0925 CAMERA_RESET_NEW_SCRIPTABLES (0x476CF8)
        0x0925 => initialise_scriptable_components(GPtr::from_addr(THE_CAMERA)),
        // 0926 GET_NUMBER_OF_INSTANCES_OF_STREAMED_SCRIPT (0x476D10): SP0 = byte [0xA47B64 + i * 0x20] (SP0 not
        // written with `i`), protected store
        0x0926 => {
            collect(s, 1);
            let i = streamed_index();
            let n = GPtr::<u8>::from_addr(STREAMED_SCRIPT_INSTANCES.wrapping_add((i as u32).wrapping_shl(5)) as usize);
            store_sp0(s, u32::from(n.read()));
        }
        // 0928 ALLOCATE_STREAMED_SCRIPT_TO_RANDOM_PED (0x476D3F): `SP0 = i`; `ScriptsForBrains.AddNewScriptBrain(i,
        // (u16)SP1, (u16)SP2, 0, -1, -1.0f)`
        0x0928 => {
            collect(s, 3);
            let i = streamed_index();
            set_sp(0, i as u32);
            add_new_script_brain(GPtr::from_addr(SCRIPTS_FOR_BRAINS), i, sp(1) & 0xFFFF, sp(2) & 0xFFFF, 0, u32::MAX, 0xBF80_0000);
        }
        // 0929 ALLOCATE_STREAMED_SCRIPT_TO_OBJECT (0x476D99): `SP0 = i`; model rule on SP1; `AddNewScriptBrain(i, m,
        // (u16)SP2, 1, (u8)SP4, SP3)`
        0x0929 => {
            collect(s, 5);
            let i = streamed_index();
            set_sp(0, i as u32);
            let m = model_operand(sp(1));
            add_new_script_brain(GPtr::from_addr(SCRIPTS_FOR_BRAINS), i, m, sp(2) & 0xFFFF, 1, sp(4) & 0xFF, sp(3));
        }
        // 092B GET_GROUP_MEMBER (0x476E09): `m = ms_groups[GetActualScriptThingIndex(SP0, 8)].m_groupMembership
        // .GetMember(SP1)` (no range check); SP0 = m ? PEDREF(m) : -1
        0x092B => {
            collect(s, 2);
            let g = get_actual_script_thing_index(sp(0), 8);
            let mship = GPtr::<u8>::from_addr(PED_GROUPS.wrapping_add(4).wrapping_add((g as u32).wrapping_mul(0x2D4)) as usize);
            let m = group_get_member(mship, sp(1));
            let h = if m.is_null() { u32::MAX } else { ped_get_ref(pool(PED_POOL), m) as u32 };
            set_sp(0, h);
            store(s, 1);
        }
        // 092E GET_WATER_HEIGHT_AT_COORDS (0x476E6E): SP2 != 0: `GetWaterLevel(x, y, 0, &h, true, null)`; else
        // `GetWaterLevelNoWaves(x, y, 0, &h, null, null)`; false → h = -1000.0f; SP0 = h
        0x092E => {
            collect(s, 3);
            let (x, y) = (sp(0), sp(1));
            let mut h = [0u32; 1];
            let hp = GPtr::from_raw(h.as_mut_ptr());
            let ok = if sp(2) != 0 {
                get_water_level(x, y, 0, hp, 1, 0)
            } else {
                get_water_level_no_waves(x, y, 0, hp, 0, 0)
            };
            if ok == 0 {
                h[0] = 0xC47A_0000;
            }
            set_sp(0, h[0]);
            store(s, 1);
        }
        // 092F CAMERA_PERSIST_TRACK (0x476EE9) / 0930 CAMERA_PERSIST_POS (0x476F10) / 0931 CAMERA_PERSIST_FOV
        // (0x476F37) / 093D SET_CINEMA_CAMERA (0x4772E5): the camera byte = SP0 != 0
        0x092F | 0x0930 | 0x0931 | 0x093D => {
            collect(s, 1);
            let a = if command == 0x092F {
                CAMERA_PERSIST_TRACK
            } else if command == 0x0930 {
                CAMERA_PERSIST_POSITION
            } else if command == 0x0931 {
                CAMERA_PERSIST_FOV
            } else {
                CAMERA_CINEMA
            };
            GPtr::<u8>::from_addr(a).write(u8::from(sp(0) != 0));
        }
        // 0933 CAMERA_IS_VECTOR_MOVE_RUNNING (0x476F5D) / 0934 CAMERA_IS_VECTOR_TRACK_RUNNING (0x476F7D): the whole
        // EAX is pushed to UpdateCompareFlag
        0x0933 => flag(s, vector_move_running(GPtr::from_addr(THE_CAMERA))),
        0x0934 => flag(s, vector_track_running(GPtr::from_addr(THE_CAMERA))),
        // 0936 CAMERA_SET_VECTOR_MOVE (0x476F9D): the duration is `fild SP6` (gr: a float parameter)
        0x0936 => {
            let (to, from, d, ease) = vector_args(s);
            let (tp, fp) = (
                GPtr::from_raw(to.as_ptr().cast_mut()),
                GPtr::from_raw(from.as_ptr().cast_mut()),
            );
            vector_move_linear(GPtr::from_addr(THE_CAMERA), tp, fp, d, ease);
        }
        // 0937 DRAW_WINDOW (0x477017)
        0x0937 => draw_window(s),
        // 0939 ATTACH_CAR_TO_OBJECT (0x477175)
        0x0939 => attach_car_to_object(s),
        // 093A SET_GARAGE_RESPRAY_FREE (0x477239): label, `CollectParameters(1)`; `g = (i16)GetGarageNumberByName`;
        // g >= 0: `aGarages[g]` byte +0x4E bit 7 = SP0 != 0
        0x093A => {
            let mut label = [0u8; 16];
            let lp = GPtr::from_raw(label.as_mut_ptr());
            read_text_label(s, lp, 8);
            collect(s, 1);
            let g = i32::from(get_garage_number_by_name(lp) as i16);
            if g >= 0 {
                let b = GPtr::<u8>::from_addr(GARAGE_FLAGS.wrapping_add((g as u32) * 0xD8) as usize);
                set_byte_bit(b, 0x80, sp(0) != 0);
            }
        }
        // 093B SET_CHAR_BULLETPROOF_VEST (0x47729D) / 0946 SET_CHAR_USES_UPPERBODY_DAMAGE_ANIMS_ONLY (0x47744C):
        // `PED(SP0)` dword +0x478 bit 0 / bit 2 = SP1 != 0 (no null check)
        0x093B | 0x0946 => {
            collect(s, 2);
            let p = ped_at_ref(pool(PED_POOL), spi(0));
            let mask = if command == 0x093B { 1 } else { 4 };
            set_dword_bit(p.byte_add(0x478).cast::<u32>(), mask, sp(1) != 0);
        }
        // 093E SET_CHAR_FIRE_DAMAGE_MULTIPLIER (0x47730C): the operands are read, nothing else
        0x093E => collect(s, 2),
        // 0940 SET_GROUP_FOLLOW_STATUS (0x477323): `g = GetActualScriptThingIndex(SP0, 8)`; 0 <= g < 8: byte
        // [0xC09924 + g * 0x2D4] = SP1 != 0
        0x0940 => {
            collect(s, 2);
            let g = get_actual_script_thing_index(sp(0), 8);
            if (0..8).contains(&g) {
                GPtr::<u8>::from_addr(PED_GROUPS.wrapping_add(g as u32 * 0x2D4) as usize).write(u8::from(sp(1) != 0));
            }
        }
        // 0941 SET_SEARCHLIGHT_CLIP_IF_COLLIDING (0x477372): `i = GetActualScriptThingIndex(SP0, 2)`; i >= 0: byte
        // [0xA94D69 + i * 0x7C] = SP1 != 0
        0x0941 => {
            collect(s, 2);
            let i = get_actual_script_thing_index(sp(0), 2);
            if i >= 0 {
                GPtr::<u8>::from_addr(SEARCHLIGHT_CLIP.wrapping_add(i as u32 * 0x7C) as usize).write(u8::from(sp(1) != 0));
            }
        }
        // 0942 HAS_PLAYER_BOUGHT_ITEM (0x4773CB): the whole EAX of `CShopping::HasPlayerBought(SP0)` is pushed
        0x0942 => {
            collect(s, 1);
            let r = has_player_bought(sp(0));
            flag(s, r);
        }
        // 0944 SET_CAMERA_IN_FRONT_OF_CHAR (0x4773F8)
        0x0944 => {
            collect(s, 1);
            let p = ped_at_ref(pool(PED_POOL), spi(0));
            set_camera_directly_in_front(GPtr::from_addr(THE_CAMERA), p);
        }
        // 0945 GET_PLAYER_MAX_ARMOUR (0x47742B): SP0 = byte `Players[SP0]` +0x150, protected store
        0x0945 => {
            collect(s, 1);
            store_sp0(s, u32::from(player_info(sp(0)).byte_add(0x150).read()));
        }
        // 0947 SET_CHAR_SAY_CONTEXT (0x47749A): SP0 = (i16)`PED(SP0)->Say((u16)SP1, 0, 1.0f, 0, 0, 0)`, protected
        // store
        0x0947 => {
            collect(s, 2);
            let p = ped_at_ref(pool(PED_POOL), spi(0));
            let r = ped_say(p, sp(1) & 0xFFFF, 0, 0x3F80_0000, 0, 0, 0);
            store_sp0(s, i32::from(r as i16) as u32);
        }
        // 0948 ADD_EXPLOSION_VARIABLE_SHAKE (0x4774DA): `AddExplosion(null, null, SP3, CVector(SP0, SP1, SP2), 0,
        // true, SP4 (raw f32), false)`
        0x0948 => {
            collect(s, 5);
            add_explosion(0, 0, sp(3), sp(0), sp(1), sp(2), 0, 1, sp(4), 0);
        }
        // 0949 ATTACH_MISSION_AUDIO_TO_CHAR (0x477545): `AudioEngine.AttachMissionAudioToPed((u8)(SP0 - 1), PED(SP1))`
        0x0949 => {
            collect(s, 2);
            let p = ped_at_ref(pool(PED_POOL), spi(1));
            attach_mission_audio_to_ped(GPtr::from_addr(AUDIO_ENGINE), sp(0).wrapping_sub(1) & 0xFF, p);
        }
        // 094A UPDATE_PICKUP_MONEY_PER_DAY (0x477584): `CPickups::UpdateMoneyPerDay(SP0, (u16)SP1)`
        0x094A => {
            collect(s, 2);
            update_money_per_day(sp(0), sp(1) & 0xFFFF);
        }
        // 094B GET_NAME_OF_ENTRY_EXIT_CHAR_USED (0x4775B3): `dst = GetPointerToScriptVariable(2)`; `ee = PED(SP0)
        // ->m_pEnex (+0x78C)`; ee: byte copy of its name (+0) up to the terminator; else `dst[0] = 0`
        0x094B => {
            collect(s, 1);
            let p = ped_at_ref(pool(PED_POOL), spi(0));
            let dst = var_ptr(s, 2).cast::<u8>();
            let ee = p.byte_add(0x78C).cast::<GPtr<u8>>().read();
            if ee.is_null() {
                dst.write(0);
            } else {
                let mut i = 0isize;
                loop {
                    let c = ee.byte_add(i).read();
                    dst.byte_add(i).write(c);
                    i += 1;
                    if c == 0 {
                        break;
                    }
                }
            }
        }
        // 094C GET_POSITION_OF_ENTRY_EXIT_CHAR_USED (0x47760C)
        0x094C => get_position_of_entry_exit_char_used(s),
        // 094D IS_CHAR_TALKING (0x4776C3): flag = AL of `PED(SP0)->GetPedTalking()`
        0x094D => {
            collect(s, 1);
            let p = ped_at_ref(pool(PED_POOL), spi(0));
            flag(s, u32::from(get_ped_talking(p)));
        }
        // 094E DISABLE_CHAR_SPEECH (0x477703): `PED(SP0)->DisablePedSpeech((u16)SP1)`
        0x094E => {
            collect(s, 2);
            let b = sp(1) & 0xFFFF;
            let p = ped_at_ref(pool(PED_POOL), spi(0));
            disable_ped_speech(p, b);
        }
        // 094F ENABLE_CHAR_SPEECH (0x47773D)
        0x094F => {
            collect(s, 1);
            enable_ped_speech(ped_at_ref(pool(PED_POOL), spi(0)));
        }
        // 0950 SET_UP_SKIP (0x47776C): `CGameLogic::SetUpSkip(CVector(SP0, SP1, SP2), SP3, 0, null, 0)`
        0x0950 => {
            collect(s, 4);
            set_up_skip(sp(0), sp(1), sp(2), sp(3), 0, 0, 0);
        }
        // 0951 CLEAR_SKIP (0x4777CE): `CGameLogic::ClearSkip(false)`
        0x0951 => clear_skip(0),
        // 0952 PRELOAD_BEAT_TRACK (0x4777E6): `AudioEngine.PreloadBeatTrack((u16)SP0)`
        0x0952 => {
            collect(s, 1);
            preload_beat_track(GPtr::from_addr(AUDIO_ENGINE), sp(0) & 0xFFFF);
        }
        // 0953 GET_BEAT_TRACK_STATUS (0x477811): SP0 = (i8)`GetBeatTrackStatus()`, protected store
        0x0953 => {
            let r = get_beat_track_status(GPtr::from_addr(AUDIO_ENGINE));
            store_sp0(s, i32::from(r as i8) as u32);
        }
        // 0954 PLAY_BEAT_TRACK (0x477823): `PlayPreloadedBeatTrack(false)`
        0x0954 => play_preloaded_beat_track(GPtr::from_addr(AUDIO_ENGINE), 0),
        // 0955 STOP_BEAT_TRACK (0x47783D)
        0x0955 => stop_beat_track(GPtr::from_addr(AUDIO_ENGINE)),
        // 0956 FIND_MAX_NUMBER_OF_GROUP_MEMBERS (0x477855): SP0 = `CStats::FindMaxNumberOfGroupMembers()`
        0x0956 => {
            let n = find_max_number_of_group_members();
            set_sp(0, n);
            store(s, 1);
        }
        // 0957 VEHICLE_DOES_PROVIDE_COVER (0x47785F): `VEH(SP0)` byte +0x42E bit 2 = SP1 != 0
        0x0957 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            set_byte_bit(v.byte_add(0x42E), 4, sp(1) != 0);
        }
        // 0958 CREATE_SNAPSHOT_PICKUP / 0959 CREATE_HORSESHOE_PICKUP / 095A CREATE_OYSTER_PICKUP (0x4778AA)
        0x0958..=0x095A => create_collectable_pickup(s, command),
        // 095B HAS_OBJECT_BEEN_UPROOTED (0x4779A7): flag = `!OBJ(SP0)->GetIsStatic()` (`neg; sbb; inc`)
        0x095B => {
            collect(s, 1);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            set_flag(s, entity_get_is_static(o) == 0);
        }
        // 095C ADD_SMOKE_PARTICLE (0x4779EC)
        0x095C => add_smoke_particle(s),
        // 095D IS_CHAR_STUCK_UNDER_CAR (0x477AA6): pushes `(PED(SP0) dword +0x478 >> 3) & 0xFFFFFF01` (exact dword)
        0x095D => {
            collect(s, 1);
            let p = ped_at_ref(pool(PED_POOL), spi(0));
            flag(s, (p.byte_add(0x478).cast::<u32>().read() >> 3) & 0xFFFF_FF01);
        }
        // 095E CONTROL_CAR_DOOR (0x477AE5)
        0x095E => control_car_door(s),
        // 095F GET_DOOR_ANGLE_RATIO (0x477B8C): door SP1 < 6 (signed): SP0 = `VEH(SP0)` +0x5C4 + 0x18 * door (`fld;
        // fstp`), StoreParameters(1); else SP0 = dword +0x718, protected store
        0x095F => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let door = spi(1);
            if door < 6 {
                let a = v.byte_add(0x5C4 + 0x18 * door as isize).cast::<f32>().read();
                set_sp(0, fld_fst(a).to_bits());
                store(s, 1);
            } else {
                store_sp0(s, v.byte_add(0x718).cast::<u32>().read());
            }
        }
        _ => return -1,
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_2300_TO_2399];
