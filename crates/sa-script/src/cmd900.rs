//! `CRunningScript::ProcessCommands900To999` (0x483BD0): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-3). Every jump-table case is in [`PORTED`]; the other opcodes of the range reach the
//! original default (-1). Cases follow the 1.0 US bytes and
//! `docs/re/specs/SCRIPT/commands/ProcessCommands900To999.md`.
//!
//! Divergence notes: text labels are read into zeroed buffers one byte longer than the label (8 → 16,
//! 15 → 16, 24 → 25), so a label that fills its buffer is terminated where the original reads stale
//! stack bytes; out-parameters the original leaves uninitialised on the stack (03D3 `found`, 038A/03B6
//! counts and lists) start zeroed.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x0384, 0x038A, 0x038B, 0x038C, 0x038D, 0x038E, 0x038F, 0x0390, 0x0391, 0x0392, 0x0393, 0x0394, 0x0395,
    0x0396, 0x0397, 0x039C, 0x039E, 0x039F, 0x03A1, 0x03A2, 0x03A3, 0x03A4, 0x03A7, 0x03A8, 0x03A9, 0x03AA,
    0x03AB, 0x03AD, 0x03AF, 0x03B0, 0x03B1, 0x03B6, 0x03B7, 0x03BA, 0x03BC, 0x03BD, 0x03BF, 0x03C0, 0x03C3,
    0x03C4, 0x03C5, 0x03C7, 0x03C8, 0x03C9, 0x03CA, 0x03CB, 0x03CC, 0x03CD, 0x03CE, 0x03CF, 0x03D0, 0x03D1,
    0x03D2, 0x03D3, 0x03D4, 0x03D5, 0x03D6, 0x03D7, 0x03D8, 0x03D9, 0x03DA, 0x03DC, 0x03DE, 0x03DF, 0x03E0,
    0x03E1, 0x03E3, 0x03E4, 0x03E5, 0x03E6, 0x03E7,
];

/// `AudioEngine` (`CAudioEngine`) 0xB6BC90.
const AUDIO_ENGINE: usize = 0xB6BC90;
/// Byte 0xBA18D9 (FREEZE_ONSCREEN_TIMER's flag, `CUserDisplay::OnscnTimer` + 0x151).
const ONSCN_TIMER_FROZEN: usize = 0xBA18D9;
/// `CStreaming::ms_disableStreaming` (byte) 0x9654B0.
const DISABLE_STREAMING: usize = 0x9654B0;
/// `CWorld::bProcessCutsceneOnly` (byte) 0xB7CD6D.
const PROCESS_CUTSCENE_ONLY: usize = 0xB7CD6D;
/// `CTheScripts::StuckCars` (`CStuckCarCheck`) 0xA90AB0.
const STUCK_CARS: usize = 0xA90AB0;
/// Bytes read by HAS_SAVE_GAME_FINISHED (0xBA67A4, 0xBA67A7).
const SAVE_BUSY_A: usize = 0xBA67A4;
const SAVE_BUSY_B: usize = 0xBA67A7;
/// `CTheScripts::NumberOfIntroTextLinesThisFrame` (u16) 0xA44B68.
const NUM_INTRO_TEXT_LINES: usize = 0xA44B68;
/// `CTheScripts::IntroTextLines` (0x44 bytes each) 0xA913E8.
const INTRO_TEXT_LINES: u32 = 0xA913E8;
/// u16 index 0xA44B5C and the 0x3C-byte array whose byte +0 starts at 0xA92D6C (SET_SPRITES_DRAW_BEFORE_FADE).
const NUM_SCRIPT_RECTS: usize = 0xA44B5C;
const SCRIPT_RECTS_DRAW_BEFORE_FADE: u32 = 0xA92D6C;
/// `CHud::m_ItemToFlash` (int16) 0xBAB1DC.
const ITEM_TO_FLASH: usize = 0xBAB1DC;

/// `CPopulation::PedDensityMultiplier` 0x8D2530 (written by address as the original does).
const PED_DENSITY_MULTIPLIER: usize = 0x8D2530;

orig! {
    /// `CTheScripts::RemoveScriptTextureDictionary()` (0x465A40)
    fn remove_script_texture_dictionary() = cdecl 0x465A40 => unported;
    /// `CAudioEngine::PreloadBeatTrack(int16)` (0x507F40)
    fn preload_beat_track(a: GPtr<u8>, track: u32) = thiscall 0x507F40 => unported;
    /// `CAudioEngine::PlayPreloadedBeatTrack(bool)` (0x507180)
    fn play_preloaded_beat_track(a: GPtr<u8>, b: u32) = thiscall 0x507180 => unported;
    /// Unnamed cdecl function of a ped type returning AL (0x43A6A0, a `.HOODLUM` stub; IS_CHAR_MALE's callee)
    fn ped_type_query_43a6a0(ped_type: u32) -> u8 = cdecl 0x43A6A0 => unported;
    /// CRT `strncpy` (0x821F40)
    fn strncpy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => crate::vm::strncpy_port;
    /// `CRunningScript::ReadTextLabelFromScript(char*, uint8)` (0x463D50)
    fn read_text_label(s: Scr, buf: GPtr<u8>, len: u32) = thiscall 0x463D50 => crate::vm::read_text_label_from_script;
    /// `CRubbish::SetVisibility(bool)` (0x7204C0)
    fn rubbish_set_visibility(v: u32) = cdecl 0x7204C0 => unported;
    /// `CTheScripts::RemoveScriptSphere(int32)` (0x483BA0)
    fn remove_script_sphere(h: i32) = cdecl 0x483BA0 => unported;
    /// `CWorld::FindPlayerWanted(int32)` (0x56E230)
    fn find_player_wanted(i: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CCamera::SetCameraDirectlyInFrontForFollowPed_CamOnAString()` (0x50BD70)
    fn set_camera_directly_in_front(cam: GPtr<u8>) = thiscall 0x50BD70 => unported;
    /// `CTimer::Stop()` (0x561AA0)
    fn timer_stop() = cdecl 0x561AA0 => unported;
    /// `CTimer::Update()` (0x561B10)
    fn timer_update() = cdecl 0x561B10 => unported;
    /// `CStreaming::LoadScene(const CVector&)` (0x40EB70)
    fn load_scene(v: GPtr<u32>) = cdecl 0x40EB70 => unported;
    /// `CStuckCarCheck::AddCarToCheck(int32, float, uint32, bool, bool, bool, bool, int8)` (0x465970)
    fn stuck_add_car_to_check(c: GPtr<u8>, h: i32, radius: u32, time: u32, a: u32, b: u32, c2: u32, d: u32, e: u32) = thiscall 0x465970 => unported;
    /// `CStuckCarCheck::RemoveCarFromCheck(int32)` (0x463B80)
    fn stuck_remove_car_from_check(c: GPtr<u8>, h: i32) = thiscall 0x463B80 => unported;
    /// `CStuckCarCheck::HasCarBeenStuckForAWhile(int32)` (0x463C00), AL
    fn stuck_has_car_been_stuck(c: GPtr<u8>, h: i32) -> u8 = thiscall 0x463C00 => unported;
    /// `CAudioEngine::PreloadMissionAudio(uint8 slot, int32 id)` (0x507290)
    fn preload_mission_audio(a: GPtr<u8>, slot: u32, id: i32) = thiscall 0x507290 => unported;
    /// `CAudioEngine::GetMissionAudioLoadingStatus(uint8)` (0x5072A0), AL
    fn get_mission_audio_loading_status(a: GPtr<u8>, slot: u32) -> u8 = thiscall 0x5072A0 => unported;
    /// `CAudioEngine::PlayLoadedMissionAudio(uint8)` (0x5072B0)
    fn play_loaded_mission_audio(a: GPtr<u8>, slot: u32) = thiscall 0x5072B0 => unported;
    /// `CAudioEngine::IsMissionAudioSampleFinished(uint8)` (0x5072C0), AL
    fn is_mission_audio_sample_finished(a: GPtr<u8>, slot: u32) -> u8 = thiscall 0x5072C0 => unported;
    /// `CHud::SetHelpMessage(const char*, bool quick, bool permanent, bool addToBrief)` (0x588BE0)
    fn set_help_message(text: GPtr<u8>, quick: u32, permanent: u32, brief: u32) = cdecl 0x588BE0 => unported;
    /// `CTimer::Suspend()` (0x5619D0)
    fn timer_suspend() = cdecl 0x5619D0 => unported;
    /// `CTimer::Resume()` (0x561A00)
    fn timer_resume() = cdecl 0x561A00 => unported;
    /// `CStreaming::LoadAllRequestedModels(bool onlyPriorityRequests)` (0x40EA10)
    fn load_all_requested_models(only_priority: u32) = cdecl 0x40EA10 => unported;
}

/// `byte [base + idx * stride + off] = v != 0` for the text/sprite setters (index re-read as compiled).
#[inline]
fn set_indexed_byte(idx_cell: usize, base: u32, stride: u32, v: u32) {
    let n = u32::from(GPtr::<u16>::from_addr(idx_cell).read());
    GPtr::<u8>::from_addr(base.wrapping_add(n.wrapping_mul(stride)) as usize).write(u8::from(v != 0));
}

/// `byte [e+off]` bit 7 = `v != 0`.
#[inline]
fn set_bit7(e: GPtr<u8>, off: isize, v: u32) {
    let p = e.byte_add(off);
    let b = p.read();
    p.write(if v != 0 { b | 0x80 } else { b & 0x7F });
}

/// The mission audio slot operand: `xor r,r; mov r8,[P0]; dec r8` (the low byte minus one, zero-extended).
#[inline]
fn audio_slot() -> u32 {
    u32::from((sp(0) as u8).wrapping_sub(1))
}

use sa_math::x87::X87;

orig! {
    /// `CText::Get(const char*)` (0x6A0050), ECX = `TheText`
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CMessages::AddMessageJumpQWithString(const char*, uint32 time, uint16 flag, const char* str, bool
    /// previousBrief)` (0x69E950)
    fn add_message_jump_q_with_string(text: GPtr<u8>, time: u32, flag: u32, s: GPtr<u8>, brief: u32) = cdecl 0x69E950 => unported;
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    fn find_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => unported;
    /// `CTxdStore::AddTxdSlot(const char*)` (0x731C80)
    fn add_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731C80 => unported;
    /// `CTxdStore::LoadTxd(int32, const char*)` (0x7320B0)
    fn load_txd(slot: i32, name: GPtr<u8>) -> u8 = cdecl 0x7320B0 => unported;
    /// `CTxdStore::AddRef(int32)` (0x731A00)
    fn txd_add_ref(slot: i32) = cdecl 0x731A00 => unported;
    /// `CTxdStore::PushCurrentTxd()` (0x7316A0)
    fn push_current_txd() = cdecl 0x7316A0 => unported;
    /// `CTxdStore::SetCurrentTxd(int32)` (0x7319C0)
    fn set_current_txd(slot: i32) = cdecl 0x7319C0 => unported;
    /// `CTxdStore::PopCurrentTxd()` (0x7316B0)
    fn pop_current_txd() = cdecl 0x7316B0 => unported;
    /// `CSprite2d::SetTexture(const char*)` (0x727270)
    fn sprite_set_texture(sp: GPtr<u8>, name: GPtr<u8>) = thiscall 0x727270 => unported;
    /// `CPhysical::AddToMovingList()` (0x542800)
    fn add_to_moving_list(e: GPtr<u8>) = thiscall 0x542800 => unported;
    /// `CPhysical::RemoveFromMovingList()` (0x542860)
    fn remove_from_moving_list(e: GPtr<u8>) = thiscall 0x542860 => unported;
    /// `CEntity::SetIsStatic(bool)` (vtable slot 0x10)
    fn set_is_static(e: GPtr<u8>, b: u32) = thiscall vslot 0x10 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, const char*)` (0x4D6870)
    fn clump_get_association(clump: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D6870 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), floats as raw dwords, ST0 (exact as f64)
    fn ground_z(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CWorld::ClearExcitingStuffFromArea(const CVector&, float radius, uint8)` (0x56A0D0)
    fn clear_exciting_stuff_from_area(v: GPtr<u32>, radius: u32, b: u32) = cdecl 0x56A0D0 => unported;
    /// `CGeneral::GetATanOfXY(float, float)` (0x53CC70), ST0 (read as f64)
    fn get_atan_of_xy(x: u32, y: u32) -> f64 = cdecl 0x53CC70 => unported;
    /// `CPlaceable::SetHeading(float)` (0x43E0C0); the float as its raw dword
    fn set_heading(e: GPtr<u8>, h: u32) = thiscall 0x43E0C0 => unported;
    /// `C3dMarkers::PlaceMarkerSet(uint32 id, uint16 type, CVector&, float size, uint8 r, uint8 g, uint8 b,
    /// uint8 a, uint16 pulsePeriod, float pulseFraction, int16 rotateRate)` (0x725BA0); floats raw
    fn place_marker_set(id: u32, ty: u32, pos: GPtr<u32>, size: u32, r: u32, g: u32, b: u32, a: u32, period: u32, fraction: u32, rot: u32) = cdecl 0x725BA0 => unported;
    /// `CCarCtrl::SwitchVehicleToRealPhysics(CVehicle*)` (0x423FC0)
    fn switch_vehicle_to_real_physics(v: GPtr<u8>) = cdecl 0x423FC0 => unported;
    /// `CGarages::GetGarageNumberByName(const char*)` (0x447680); the caller tests AX
    fn get_garage_number_by_name(name: GPtr<u8>) -> u32 = cdecl 0x447680 => unported;
    /// `CGarages::IsGarageOpen(int16)` (0x447D00), AL
    fn is_garage_open(g: u32) -> u8 = cdecl 0x447D00 => unported;
    /// `CGarages::IsGarageClosed(int16)` (0x447D30), AL
    fn is_garage_closed(g: u32) -> u8 = cdecl 0x447D30 => unported;
    /// `CWorld::ClearCarsFromArea(float x1, float y1, float z1, float x2, float y2, float z2)` (0x566610)
    fn clear_cars_from_area(x1: u32, y1: u32, z1: u32, x2: u32, y2: u32, z2: u32) = cdecl 0x566610 => unported;
    /// `CTheScripts::GetActualScriptThingIndex(int32, uint8 type)` (0x4839A0); result discarded here
    fn get_actual_script_thing_index(h: i32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `CTheScripts::AddScriptSphere(uint32 id, CVector, float radius)` (0x483B30); vector and float raw
    fn add_script_sphere(id: u32, x: u32, y: u32, z: u32, radius: u32) -> u32 = cdecl 0x483B30 => unported;
    /// `CWorld::StopAllLawEnforcersInTheirTracks()` (0x566C10)
    fn stop_all_law_enforcers_in_their_tracks() = cdecl 0x566C10 => unported;
    /// `CPool<CVehicle>::GetRef(CVehicle*)` (0x424160), ECX = the vehicle pool
    fn vehicle_get_ref(pool: GPtr<u8>, v: GPtr<u8>) -> u32 = thiscall 0x424160 => unported;
    /// `CRunningScript::GetIndexOfGlobalVariable()` (0x464700); AX used
    fn get_index_of_global_variable(s: Scr) -> u16 = thiscall 0x464700 => crate::vm::get_index_of_global_variable;
    /// `COnscreenTimer::AddClock(uint32 varId, const char* gxt, bool countDown)` (0x44CD50)
    fn add_clock(t: GPtr<u8>, var: u32, gxt: GPtr<u8>, down: u32) = thiscall 0x44CD50 => unported;
    /// `COnscreenTimer::AddCounter(int32 varId, int16 type, const char* gxt, uint16 index)` (0x44CDA0)
    fn add_counter(t: GPtr<u8>, var: u32, ty: u32, gxt: GPtr<u8>, index: u32) = thiscall 0x44CDA0 => unported;
    /// `CPathFind::FindNodeClosestToCoors(CVector, uint8 type, float maxDist, uint16, uint16, uint16, bool,
    /// bool)` (0x44F460): `CNodeAddress` through the hidden pointer (first argument), returned in EAX;
    /// vector and float raw
    fn find_node_closest_to_coors(paths: GPtr<u8>, ret: GPtr<u32>, x: u32, y: u32, z: u32, ty: u32, dist: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> GPtr<u32> = thiscall 0x44F460 => unported;
    /// `CPathFind::FindNodeCoorsForScript(CNodeAddress, bool* found)` (0x4505E0): CVector through the
    /// hidden pointer (first argument)
    fn find_node_coors_for_script(paths: GPtr<u8>, ret: GPtr<u32>, node: u32, found: GPtr<u8>) -> GPtr<u32> = thiscall 0x4505E0 => unported;
    /// `CPathFind::FindNodeOrientationForCarPlacement(CNodeAddress)` (0x450320), ST0 (read as f64)
    fn find_node_orientation_for_car_placement(paths: GPtr<u8>, node: u32) -> f64 = thiscall 0x450320 => unported;
    /// `CMessages::ClearThisPrint(const char*)` (0x69EA30)
    fn clear_this_print(text: GPtr<u8>) = cdecl 0x69EA30 => unported;
    /// `CMessages::ClearThisBigPrint(const char*)` (0x69EBE0)
    fn clear_this_big_print(text: GPtr<u8>) = cdecl 0x69EBE0 => unported;
    /// `CAudioEngine::SetMissionAudioPosition(uint8 slot, CVector&)` (0x507300)
    fn set_mission_audio_position(a: GPtr<u8>, slot: u32, v: GPtr<u32>) = thiscall 0x507300 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(i: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPhysical::SetVelocity(CVector)` (0x441130, thunk), vector raw
    fn physical_set_velocity(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x441130 => unported;
    /// Unnamed thiscall at 0x45AFB0 (thunk) taking a CVector by value (the call after SetVelocity,
    /// presumably `CPhysical::SetTurnSpeed`)
    fn physical_45afb0(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x45AFB0 => unported;
    /// `CPickups::GetActualPickupIndex(int32)` (0x4552A0); result discarded here
    fn get_actual_pickup_index(h: i32) -> i32 = cdecl 0x4552A0 => unported;
    /// `CRadar::SetEntityBlip(eBlipType, int32, uint32 colour, eBlipDisplay, char* name)` (0x5839A0)
    fn set_entity_blip(ty: u32, h: u32, colour: u32, display: u32, name: GPtr<u8>) -> u32 = cdecl 0x5839A0 => unported;
    /// `CRadar::ChangeBlipScale(int32, int32)` (0x583CC0)
    fn change_blip_scale(h: u32, scale: u32) = cdecl 0x583CC0 => unported;
    /// `CMessages::AddToPreviousBriefArray(const char*, int32 x6, const char*)` (0x69DD50)
    fn add_to_previous_brief_array(text: GPtr<u8>, a: i32, b: i32, c: i32, d: i32, e: i32, f6: i32, s: u32) = cdecl 0x69DD50 => unported;
}

/// `TheText` 0xC1B340.
const THE_TEXT: usize = 0xC1B340;
/// `ThePaths` 0x96F050.
const THE_PATHS: usize = 0x96F050;
/// `CUserDisplay::OnscnTimer` 0xBA1788.
const ONSCN_TIMER: usize = 0xBA1788;
/// `CTheScripts::ScriptSprites[0]` (`CSprite2d`, 4 bytes) 0xA94B64.
const SCRIPT_SPRITES: u32 = 0xA94B64;
/// "script" (`.rdata` 0x859D4C).
const S_SCRIPT: usize = 0x859D4C;
/// `CMessages`/brief "add to previous brief" byte 0xA4448B (`CTheScripts::bAddNextMessageToPreviousBriefs`).
const ADD_NEXT_TO_PREVIOUS_BRIEFS: usize = 0xA4448B;
/// The two PRINT_HELP gates 0xA444A7 / 0xA444A8 (`CTheScripts::bMiniGameInProgress` /
/// `bDisplayNonMiniGameHelpMessages`) and the script's +0xC8 byte (`m_bIsActive`... as read).
const HELP_GATE_A: usize = 0xA444A7;
const HELP_GATE_B: usize = 0xA444A8;
/// Byte 0xBA67A7 (ACTIVATE_SAVE_MENU's request flag).
const SAVE_MENU_REQUEST: usize = 0xBA67A7;
/// `.rdata` 0x858B38: 0.02f
const K_0_02: u32 = 0x3CA3_D70A;
/// `.rdata` 0x858FE4: pi/2
const K_HALF_PI: u32 = 0x3FC9_0FDB;
/// `.rdata` 0x858CBC: 2pi
const K_TWO_PI: u32 = 0x40C9_0FDB;

orig! {
    /// `CRGBA::CRGBA(uint8 r, uint8 g, uint8 b, uint8 a)` (0x7170C0), returns this
    fn rgba_ctor(c: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// Unnamed thiscall at 0x463370 (a `.HOODLUM` stub) taking `const CRGBA*`: the script rectangle's colour
    /// setter (presumably `CRGBA::Set(const CRGBA&)`)
    fn rgba_set_463370(c: GPtr<u8>, from: GPtr<u8>) = thiscall 0x463370 => unported;
}

/// `CTheScripts::IntroRectangles[0]` (`tScriptRectangle`, 0x3C bytes) 0xA92D68.
const INTRO_RECTANGLES: u32 = 0xA92D68;
/// `RsGlobal.maximumWidth` / `maximumHeight` (int) 0xC17044 / 0xC17048.
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// `.rdata` 0x858B8C: 0.5f; 0x859520: 1/640 (0x3ACCCCCD); 0x859524: 1/448 (0x3B124925)
const K_HALF: u32 = 0x3F00_0000;
const K_INV_640: u32 = 0x3ACC_CCCD;
const K_INV_448: u32 = 0x3B12_4925;

/// DRAW_SPRITE (038D) / DRAW_RECT (038E) after `CollectParameters`: the rectangle `r = IntroRectangles[n]`,
/// `n = u16 [0xA44B5C]`: `r.type (+0) = ty`, `r.sprite (+6, u16) = sprite`; x87 (PC=24, the order and
/// operand forms of the bytes): `w' = w * 0.5` (kept in a register), `h' = h * 0.5` (`fstp` local), `sx =
/// fild maxWidth * (1/640)` (register), `sy = fild maxHeight * (1/448)` (`fstp` local); left (+8) =
/// `(x - w') * sx`, top (+0xC) = `(y - h') * sy`, right (+0x10) = `(w' + x) * sx`, bottom (+0x14) =
/// `(h' + y) * sy`; +0x18 = 0; a stack `CRGBA(c0, c1, c2, c3)` copied with 0x463370 into +0x1C; `n += 1`; byte +0x20 of
/// the same rectangle = 0.
#[allow(clippy::too_many_arguments)]
fn script_rect(ty: u32, sprite: u16, x: u32, y: u32, w: u32, h: u32, c: [u32; 4]) {
    let idx = GPtr::<u16>::from_addr(NUM_SCRIPT_RECTS);
    let r = GPtr::<u8>::from_addr(INTRO_RECTANGLES.wrapping_add(u32::from(idx.read()) * 0x3C) as usize);
    let (x, y) = (X87::from_f32(f(x)), X87::from_f32(f(y)));
    let w2 = X87::from_f32(f(w)).fmul_m(f(K_HALF));
    let h2 = X87::from_f32(f(h)).fmul_m(f(K_HALF)).to_f32();
    let sx = X87::from_i32(GPtr::<i32>::from_addr(RS_MAX_WIDTH).read()).fmul_m(f(K_INV_640));
    r.cast::<u32>().write(ty);
    r.byte_add(6).cast::<u16>().write(sprite);
    let left = x.fsub(w2).fmul(sx).to_f32();
    r.byte_add(8).cast::<u32>().write(left.to_bits());
    let sy = X87::from_i32(GPtr::<i32>::from_addr(RS_MAX_HEIGHT).read()).fmul_m(f(K_INV_448)).to_f32();
    let top = y.fsub_m(h2).fmul_m(sy).to_f32();
    r.byte_add(0xC).cast::<u32>().write(top.to_bits());
    let right = w2.fadd(x).fmul(sx).to_f32();
    r.byte_add(0x10).cast::<u32>().write(right.to_bits());
    let bottom = X87::from_f32(h2).fadd(y).fmul_m(sy).to_f32();
    r.byte_add(0x14).cast::<u32>().write(bottom.to_bits());
    r.byte_add(0x18).cast::<u32>().write(0);
    let mut col = [0u8; 4];
    let cp = rgba_ctor(GPtr::from_raw(col.as_mut_ptr()), c[0], c[1], c[2], c[3]);
    rgba_set_463370(r.byte_add(0x1C), cp);
    idx.write(idx.read().wrapping_add(1));
    r.byte_add(0x20).write(0);
}

orig! {
    /// `CWorld::FindMissionEntitiesIntersectingCube(const CVector& min, const CVector& max, int16* count,
    /// int16 max, CEntity** out, bool vehicles, bool peds, bool objects)` (0x569240)
    fn find_mission_entities_intersecting_cube(min: GPtr<u32>, max: GPtr<u32>, count: GPtr<u16>, max_count: u32, out: u32, v: u32, p: u32, o: u32) = cdecl 0x569240 => unported;
    /// `CWorld::FindObjectsOfTypeInRange(uint32 model, const CVector&, float radius, bool 2d, int16* count,
    /// int16 max, CEntity** out, bool buildings, bool vehicles, bool peds, bool objects, bool dummies)`
    /// (0x564C70); the float raw
    fn find_objects_of_type_in_range(model: u32, pos: GPtr<u32>, radius: u32, two_d: u32, count: GPtr<u16>, max_count: u32, out: GPtr<u32>, b: u32, v: u32, p: u32, o: u32, d: u32) = cdecl 0x564C70 => unported;
    /// `CVector::Magnitude()` (0x4082C0), ST0 (an `fsqrt` result under PC=24: exact as f64)
    fn vector_magnitude(v: GPtr<u32>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CBuilding::ReplaceWithNewModel(int32)` (0x403EC0)
    fn replace_with_new_model(b: GPtr<u8>, model: u32) = thiscall 0x403EC0 => unported;
    /// `CTheScripts::AddToBuildingSwapArray(CBuilding*, int32 old, int32 new)` (0x481140)
    fn add_to_building_swap_array(b: GPtr<u8>, old: u32, new: u32) = cdecl 0x481140 => unported;
}

/// A model operand: a negative value indexes `CTheScripts::UsedObjectArray` backwards
/// (`imul v,0x1C; mov r,0xA44B88; sub r,v; mov v,[r]`).
#[inline]
fn model_operand(v: u32) -> u32 {
    if (v as i32) < 0 {
        GPtr::<u32>::from_addr(0xA44B88u32.wrapping_sub(v.wrapping_mul(0x1C)) as usize).read()
    } else {
        v
    }
}

orig! {
    /// `CLoadedCarGroup::PickRandomCar(bool, bool)` (0x611C50), ECX = `CPopulation::m_LoadedCars`... the
    /// group at 0xC0E9F8
    fn pick_random_car(g: GPtr<u8>, a: u32, b: u32) -> i32 = thiscall 0x611C50 => unported;
    /// `CVehicle::operator new(uint32)` (0x6E2D50)
    fn vehicle_new(size: u32) -> GPtr<u8> = cdecl 0x6E2D50 => unported;
    /// `CMonsterTruck::CMonsterTruck(int32 model, eVehicleCreatedBy)` (0x6C8D60)
    fn monster_truck_ctor(v: GPtr<u8>, model: i32, by: u32) -> GPtr<u8> = thiscall 0x6C8D60 => unported;
    /// `CQuadBike::CQuadBike(int32, eVehicleCreatedBy)` (0x6CE370)
    fn quad_bike_ctor(v: GPtr<u8>, model: i32, by: u32) -> GPtr<u8> = thiscall 0x6CE370 => unported;
    /// `CHeli::CHeli(int32, eVehicleCreatedBy)` (0x6C4190)
    fn heli_ctor(v: GPtr<u8>, model: i32, by: u32) -> GPtr<u8> = thiscall 0x6C4190 => unported;
    /// `CPlane::CPlane(int32, eVehicleCreatedBy)` (0x6C8E20)
    fn plane_ctor(v: GPtr<u8>, model: i32, by: u32) -> GPtr<u8> = thiscall 0x6C8E20 => unported;
    /// `CBike::CBike(int32, eVehicleCreatedBy)` (0x6BF430)
    fn bike_ctor(v: GPtr<u8>, model: i32, by: u32) -> GPtr<u8> = thiscall 0x6BF430 => unported;
    /// `CBmx::CBmx(int32, eVehicleCreatedBy)` (0x6BF820)
    fn bmx_ctor(v: GPtr<u8>, model: i32, by: u32) -> GPtr<u8> = thiscall 0x6BF820 => unported;
    /// `CTrailer::CTrailer(int32, eVehicleCreatedBy)` (0x6D03A0)
    fn trailer_ctor(v: GPtr<u8>, model: i32, by: u32) -> GPtr<u8> = thiscall 0x6D03A0 => unported;
    /// `CAutomobile::CAutomobile(int32, eVehicleCreatedBy, bool setupSuspensionLines)` (0x6B0A90)
    fn automobile_ctor(v: GPtr<u8>, model: i32, by: u32, susp: u32) -> GPtr<u8> = thiscall 0x6B0A90 => unported;
    /// `CEntity::GetDistanceFromCentreOfMassToBaseOfModel()` (vtable slot 0xD4), ST0 (a float function's
    /// result: exact as f64)
    fn distance_to_base(e: GPtr<u8>) -> f64 = thiscall vslot 0xD4 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0)
    fn set_posn_v(e: GPtr<u8>, pos: GPtr<u32>) = thiscall 0x4241C0 => unported;
    /// `CTheScripts::ClearSpaceForMissionEntity(const CVector&, CEntity*)` (0x486B00)
    fn clear_space_for_mission_entity(pos: GPtr<u32>, e: GPtr<u8>) = cdecl 0x486B00 => unported;
    /// `CCarCtrl::JoinCarWithRoadSystem(CVehicle*)` (0x42F5A0)
    fn join_car_with_road_system(v: GPtr<u8>) = cdecl 0x42F5A0 => unported;
}

/// `CCarCtrl::NumMissionCars`... the int 0x969094 CREATE_RANDOM_CAR_FOR_CAR_PARK compares with 45.
const CAR_PARK_CAR_COUNT: usize = 0x969094;
/// The `CLoadedCarGroup` 0xC0E9F8 (`CPopulation::m_LoadedCars`... as the case passes it).
const LOADED_CAR_GROUP: usize = 0xC0E9F8;
/// `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8.
const MODEL_INFO_PTRS: u32 = 0xA9B0C8;
/// `.rdata` 0x8595EC: pi/180
const K_DEG_TO_RAD: u32 = 0x3C8E_FA35;

/// An 8-byte text label read with `ReadTextLabelFromScript` into a zeroed 16-byte buffer (divergence
/// note: the original's stack buffer is not cleared, so an 8-character label is followed by stale stack
/// bytes when a callee reads it as a C string).
#[inline]
fn label8(s: Scr, buf: &mut [u8; 16]) -> GPtr<u8> {
    let b = GPtr::from_raw(buf.as_mut_ptr());
    read_text_label(s, b, 8);
    b
}

/// `TheText.Get(label)`.
#[inline]
fn text(b: GPtr<u8>) -> GPtr<u8> {
    text_get(GPtr::from_addr(THE_TEXT), b)
}

/// The coordinate triple of the CLEAR_AREA / DRAW_SPHERE / ADD_SPHERE / GET_CLOSEST_CAR_NODE_WITH_HEADING
/// cases: (P0, P1) raw; z = P2 through `fld; fcom -100.0; test ah,0x41; jp` (kept and `fstp`ed unless
/// `z <= -100`, ordered), else FindGroundZForCoord(P0, P1) `fstp`ed.
fn xyz_ground() -> [u32; 3] {
    let z = if spf(2) <= f(K_MINUS_100) {
        X87::from_f64_exact(ground_z(sp(0), sp(1))).to_f32().to_bits()
    } else {
        fld_fst(spf(2)).to_bits()
    };
    [sp(0), sp(1), z]
}

/// `wanted = Players[P0].m_pPed->m_pPlayerData (+0x480) ? ->m_Wanted : nullptr` (the null path still
/// writes `[0x1E]`, as the exe); P1 != 0: byte +0x1E |= bit, then StopAllLawEnforcersInTheirTracks();
/// else cleared (SET_POLICE_IGNORE_PLAYER / SET_EVERYONE_IGNORE_PLAYER).
fn wanted_back_off(bit: u8) {
    let ped = player_info(sp(0)).cast::<GPtr<u8>>().read();
    let pd = ped.byte_add(0x480).cast::<GPtr<u8>>().read();
    let wanted = if pd.is_null() { GPtr::NULL } else { pd.cast::<GPtr<u8>>().read() };
    let b = wanted.byte_add(0x1E);
    if sp(1) != 0 {
        b.write(b.read() | bit);
        stop_all_law_enforcers_in_their_tracks();
    } else {
        b.write(b.read() & !bit);
    }
}

/// `CRunningScript::ProcessCommands900To999(int32 command)` (0x483BD0, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x483BD0, conv = thiscall, name = "CRunningScript::ProcessCommands900To999", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript")]
pub fn process_commands_900_to_999(s: Scr, command: u32) -> i8 {
    match command {
        // 038B LOAD_ALL_MODELS_NOW (0x483DDB): CTimer::Suspend(); LoadAllRequestedModels(false); CTimer::Resume().
        0x038B => {
            timer_suspend();
            load_all_requested_models(0);
            timer_resume();
        }
        // 0391 REMOVE_TEXTURE_DICTIONARY (0x4841BB): RemoveScriptTextureDictionary(); when the script uses
        // the mission cleanup, MissionCleanUp.RemoveEntityFromList(1, 0xD).
        0x0391 => {
            remove_script_texture_dictionary();
            if uses_mission_cleanup(s) {
                mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), 1, 0xD);
            }
        }
        // 0394 PLAY_MISSION_PASSED_TUNE (0x4842AF): only for P0 == 1 or 2: AudioEngine.PreloadBeatTrack(P0 +
        // 10); AudioEngine.PlayPreloadedBeatTrack(true).
        0x0394 => {
            collect(s, 1);
            let t = sp(0);
            if t == 1 || t == 2 {
                preload_beat_track(GPtr::from_addr(AUDIO_ENGINE), t.wrapping_add(10));
                play_preloaded_beat_track(GPtr::from_addr(AUDIO_ENGINE), 1);
            }
        }
        // 0396 FREEZE_ONSCREEN_TIMER (0x484370): byte 0xBA18D9 = P0 != 0.
        0x0396 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(ONSCN_TIMER_FROZEN).write(u8::from(sp(0) != 0));
        }
        // 0397 SWITCH_CAR_SIREN (0x48438F) / 03AB SET_CAR_STRONG (0x48470F): no null check; byte +0x42D /
        // +0x429 bit 7 = P1 != 0.
        0x0397 | 0x03AB => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            set_bit7(veh, if command == 0x0397 { 0x42D } else { 0x429 }, sp(1));
        }
        // 039E SET_CHAR_CANT_BE_DRAGGED_OUT (0x484468): no null check; dword +0x46C bit 0x20000000 = P1 != 0.
        0x039E => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let p = ped.byte_add(0x46C).cast::<u32>();
            let v = p.read();
            p.write(if sp(1) != 0 {
                v | 0x2000_0000
            } else {
                v & 0xDFFF_FFFF
            });
        }
        // 03A3 IS_CHAR_MALE (0x48465B): no null check; flag = 0x43A6A0(m_nPedType (+0x598)) == 0 (`neg; sbb;
        // inc`).
        0x03A3 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let r = ped_type_query_43a6a0(ped.byte_add(0x598).cast::<u32>().read());
            set_flag(s, r == 0);
        }
        // 03A4 SCRIPT_NAME (0x4846A1): ReadTextLabelFromScript(buf, 8); the 8 bytes are lower-cased in place
        // ('A'..='Z' + 0x20); strncpy(m_szName (+8), buf, 8).
        0x03A4 => {
            let mut buf = [0u8; 16];
            let b = GPtr::from_raw(buf.as_mut_ptr());
            read_text_label(s, b, 8);
            for i in 0..8 {
                let p = b.byte_add(i);
                let c = p.read();
                if c.is_ascii_uppercase() {
                    p.write(c + 0x20);
                }
            }
            strncpy(s.byte_add(8), b, 8);
        }
        // 03A7 SAVE_INT_TO_DEBUG_FILE / 03A8 SAVE_FLOAT_TO_DEBUG_FILE (0x4846EF): 1 operand read.
        0x03A7 | 0x03A8 => collect(s, 1),
        // 03AA POLICE_RADIO_MESSAGE (0x4846FF): 3 operands read.
        0x03AA => collect(s, 3),
        // 03AD SWITCH_RUBBISH (0x48475A): CRubbish::SetVisibility(P0 != 0).
        0x03AD => {
            collect(s, 1);
            rubbish_set_visibility(u32::from(sp(0) != 0));
        }
        // 03AF SWITCH_STREAMING (0x48478E): CStreaming::ms_disableStreaming = P0 == 0.
        0x03AF => {
            collect(s, 1);
            GPtr::<u8>::from_addr(DISABLE_STREAMING).write(u8::from(sp(0) == 0));
        }
        // 03B7 SWITCH_WORLD_PROCESSING (0x4849CC): CWorld::bProcessCutsceneOnly = P0 == 0.
        0x03B7 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(PROCESS_CUTSCENE_ONLY).write(u8::from(sp(0) == 0));
        }
        // 03BD REMOVE_SPHERE (0x484B77)
        0x03BD => {
            collect(s, 1);
            remove_script_sphere(spi(0));
        }
        // 03C7 SET_WANTED_MULTIPLIER (0x485003): FindPlayerWanted(-1)->m_fMultiplier (+0x14) = P0 (raw).
        0x03C7 => {
            collect(s, 1);
            let w = find_player_wanted(-1);
            w.byte_add(0x14).cast::<u32>().write(sp(0));
        }
        // 03C8 SET_CAMERA_IN_FRONT_OF_PLAYER (0x485026)
        0x03C8 => set_camera_directly_in_front(GPtr::from_addr(THE_CAMERA)),
        // 03C9 IS_CAR_VISIBLY_DAMAGED (0x485037): no null check; flag = byte +0x42A bit 0.
        0x03C9 => {
            collect(s, 1);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            set_flag(s, veh.byte_add(0x42A).read() & 1 != 0);
        }
        // 03CA DOES_OBJECT_EXIST (0x485063): flag = ObjectPool.GetAtRef(P0) != null.
        0x03CA => {
            collect(s, 1);
            let b = !object_at_ref(pool(OBJECT_POOL), spi(0)).is_null();
            set_flag(s, b);
        }
        // 03CB LOAD_SCENE (0x48509F): the vector (P0, P1, P2) is copied to locals before CTimer::Stop();
        // CStreaming::LoadScene(&copy); CTimer::Update().
        0x03CB => {
            collect(s, 3);
            let v = [sp(0), sp(1), sp(2)];
            timer_stop();
            load_scene(GPtr::from_raw(v.as_ptr().cast_mut()));
            timer_update();
        }
        // 03CC ADD_STUCK_CAR_CHECK (0x485107): StuckCars.AddCarToCheck(P0, P1, P2, 0, 0, 0, 0, 0).
        0x03CC => {
            collect(s, 3);
            stuck_add_car_to_check(GPtr::from_addr(STUCK_CARS), spi(0), sp(1), sp(2), 0, 0, 0, 0, 0);
        }
        // 03CD REMOVE_STUCK_CAR_CHECK (0x48513F)
        0x03CD => {
            collect(s, 1);
            stuck_remove_car_from_check(GPtr::from_addr(STUCK_CARS), spi(0));
        }
        // 03CE IS_CAR_STUCK (0x485160): flag = StuckCars.HasCarBeenStuckForAWhile(P0) != 0.
        0x03CE => {
            collect(s, 1);
            let b = stuck_has_car_been_stuck(GPtr::from_addr(STUCK_CARS), spi(0)) != 0;
            set_flag(s, b);
        }
        // 03CF LOAD_MISSION_AUDIO (0x48519B): AudioEngine.PreloadMissionAudio((u8)P0 - 1, P1).
        0x03CF => {
            collect(s, 2);
            preload_mission_audio(GPtr::from_addr(AUDIO_ENGINE), audio_slot(), spi(1));
        }
        // 03D0 HAS_MISSION_AUDIO_LOADED (0x4851C7): flag = GetMissionAudioLoadingStatus((u8)P0 - 1) == 1.
        0x03D0 => {
            collect(s, 1);
            let st = get_mission_audio_loading_status(GPtr::from_addr(AUDIO_ENGINE), audio_slot());
            set_flag(s, st == 1);
        }
        // 03D1 PLAY_MISSION_AUDIO (0x485203)
        0x03D1 => {
            collect(s, 1);
            play_loaded_mission_audio(GPtr::from_addr(AUDIO_ENGINE), audio_slot());
        }
        // 03D2 HAS_MISSION_AUDIO_FINISHED (0x485228): flag = IsMissionAudioSampleFinished((u8)P0 - 1) != 0.
        0x03D2 => {
            collect(s, 1);
            let b = is_mission_audio_sample_finished(GPtr::from_addr(AUDIO_ENGINE), audio_slot()) != 0;
            set_flag(s, b);
        }
        // 03D9 HAS_SAVE_GAME_FINISHED (0x48548A): flag = byte 0xBA67A4 == 0 && byte 0xBA67A7 == 0.
        0x03D9 => {
            let b = GPtr::<u8>::from_addr(SAVE_BUSY_A).read() == 0
                && GPtr::<u8>::from_addr(SAVE_BUSY_B).read() == 0;
            set_flag(s, b);
        }
        // 03DE SET_PED_DENSITY_MULTIPLIER (0x485516): raw dword copy.
        0x03DE => {
            collect(s, 1);
            GPtr::<u32>::from_addr(PED_DENSITY_MULTIPLIER).write(sp(0));
        }
        // 03E0 SET_TEXT_DRAW_BEFORE_FADE (0x485532) / 03E4 SET_TEXT_RIGHT_JUSTIFY (0x4855B6): intro text
        // line byte +0x23 / +0x24 = P0 != 0.
        0x03E0 | 0x03E4 => {
            collect(s, 1);
            let off = if command == 0x03E0 { 0x23 } else { 0x24 };
            set_indexed_byte(NUM_INTRO_TEXT_LINES, INTRO_TEXT_LINES + off, 0x44, sp(0));
        }
        // 03E3 SET_SPRITES_DRAW_BEFORE_FADE (0x485574): byte [0xA92D6C + u16[0xA44B5C] * 0x3C] = P0 != 0.
        0x03E3 => {
            collect(s, 1);
            set_indexed_byte(NUM_SCRIPT_RECTS, SCRIPT_RECTS_DRAW_BEFORE_FADE, 0x3C, sp(0));
        }
        // 03E6 CLEAR_HELP (0x48566A): CHud::SetHelpMessage(nullptr, true, false, false).
        0x03E6 => set_help_message(GPtr::NULL, 1, 0, 0),
        // 03E7 FLASH_HUD_OBJECT (0x48567E): CHud::m_ItemToFlash = (u16)P0.
        0x03E7 => {
            collect(s, 1);
            GPtr::<u16>::from_addr(ITEM_TO_FLASH).write(sp(0) as u16);
        }
        // 0384 PRINT_STRING_IN_STRING_NOW (0x483C11): two labels (8) read into the same buffer, each through
        // `TheText.Get`; `CollectParameters(2)`; AddMessageJumpQWithString(text1, P0, (u16)P1, text2, byte
        // 0xA4448B != 0); then byte 0xA4448B = 1. gr: unimplemented. Divergence notes: zeroed label buffer;
        // the `setne` bool is passed as 0/1.
        0x0384 => {
            let mut buf = [0u8; 16];
            let b = label8(s, &mut buf);
            let t1 = text(b);
            read_text_label(s, b, 8);
            let t2 = text(b);
            collect(s, 2);
            let brief = GPtr::<u8>::from_addr(ADD_NEXT_TO_PREVIOUS_BRIEFS);
            add_message_jump_q_with_string(t1, sp(0), sp(1) & 0xFFFF, t2, u32::from(brief.read() != 0));
            brief.write(1);
        }
        // 038C ADD_TO_OBJECT_VELOCITY (0x483DF6): object not null-checked; the three `m_vecMoveSpeed` (+0x44)
        // components are copied first, then each `= fld Pn; fmul 0.02f; fadd old` (PC=24), written x, y, z.
        // gr: unregistered.
        0x038C => {
            collect(s, 4);
            let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
            let v = obj.byte_add(0x44).cast::<u32>();
            let old = [v.read(), v.wrapping_index(1).read(), v.wrapping_index(2).read()];
            for i in 0..3 {
                let n = X87::from_f32(spf(1 + i)).fmul_m(f(K_0_02)).fadd_m(f(old[i as usize])).to_f32();
                v.wrapping_index(i).write(n.to_bits());
            }
        }
        // 038D DRAW_SPRITE (0x483E80): `CollectParameters(9)`; script_rect(4, P0 - 1, P1, P2, P3, P4, (u8)P5..P8).
        0x038D => {
            collect(s, 9);
            let c = [sp(5) & 0xFF, sp(6) & 0xFF, sp(7) & 0xFF, sp(8) & 0xFF];
            script_rect(4, sp(0).wrapping_sub(1) as u16, sp(1), sp(2), sp(3), sp(4), c);
        }
        // 038E DRAW_RECT (0x483F8F): `CollectParameters(8)`; script_rect(3, 0xFFFF, P0, P1, P2, P3, (u8)P4..P7).
        0x038E => {
            collect(s, 8);
            let c = [sp(4) & 0xFF, sp(5) & 0xFF, sp(6) & 0xFF, sp(7) & 0xFF];
            script_rect(3, 0xFFFF, sp(0), sp(1), sp(2), sp(3), c);
        }
        // 038A IS_POINT_OBSCURED_BY_A_MISSION_ENTITY (0x483C8C): x87 (PC=24, operand forms of the bytes): xa =
        // `x - rx`, xb = `rx + x`, ya = `y - ry` (`fsubr`), yb = `ry + y` (all `fstp`ed), zlo = `z - rz`, zhi =
        // `rz + z` (both kept in registers, zhi also `fst`); the x and y pairs are ordered by `fcomp` (swap when
        // `a > b`, ordered); z: when `zlo > zhi` (registers, ordered) min z = (f32)zhi, max z = (f32)zlo, else
        // min = (f32)zlo, max = (f32)zhi; flag = `FindMissionEntitiesIntersectingCube(&min, &max, &n, 2,
        // nullptr, true, true, true)` leaves `(int16)n > 0`. gr: unregistered. Divergence note: `n` starts
        // zeroed (the original's stack word is stale until the callee writes it).
        0x038A => {
            collect(s, 6);
            let (x, y, z) = (X87::from_f32(spf(0)), X87::from_f32(spf(1)), spf(2));
            let (rx, ry, rz) = (X87::from_f32(spf(3)), X87::from_f32(spf(4)), spf(5));
            let xa = x.fsub(rx).to_f32().to_bits();
            let xb = rx.fadd(x).to_f32().to_bits();
            let ya = ry.fsubr(y).to_f32().to_bits();
            let yb = ry.fadd(y).to_f32().to_bits();
            let lo = X87::from_f32(z).fsub_m(rz);
            let hi = X87::from_f32(rz).fadd_m(z);
            let pair = |a: u32, b: u32| if f(a) > f(b) { (b, a) } else { (a, b) };
            let (x1, x2) = pair(xa, xb);
            let (y1, y2) = pair(ya, yb);
            let (z1, z2) = if lo.fcom(hi) == sa_math::x87::Cmp::Greater {
                (hi.to_f32().to_bits(), lo.to_f32().to_bits())
            } else {
                (lo.to_f32().to_bits(), hi.to_f32().to_bits())
            };
            let min = [x1, y1, z1];
            let max = [x2, y2, z2];
            let mut n = [0u16; 2];
            let np = GPtr::from_raw(n.as_mut_ptr());
            find_mission_entities_intersecting_cube(
                GPtr::from_raw(min.as_ptr().cast_mut()),
                GPtr::from_raw(max.as_ptr().cast_mut()),
                np,
                2,
                0,
                1,
                1,
                1,
            );
            set_flag(s, (np.read() as i16) > 0);
        }
        // 038F LOAD_SPRITE (0x484075): `CollectParameters(1)`; label (15 bytes) lower-cased in place (15 bytes);
        // `slot = FindTxdSlot("script")`; PushCurrentTxd(); SetCurrentTxd(slot); ScriptSprites[P0].SetTexture
        // (label); PopCurrentTxd(). Divergence note: zeroed 16-byte buffer (a 15-character label has its
        // terminator in stale stack bytes in the original).
        0x038F => {
            collect(s, 1);
            let mut buf = [0u8; 16];
            let b = GPtr::from_raw(buf.as_mut_ptr());
            read_text_label(s, b, 0xF);
            for i in 0..15 {
                let p = b.byte_add(i);
                let c = p.read();
                if c.is_ascii_uppercase() {
                    p.write(c + 0x20);
                }
            }
            let slot = find_txd_slot(GPtr::from_addr(S_SCRIPT));
            push_current_txd();
            set_current_txd(slot);
            let sprite = GPtr::<u8>::from_addr(SCRIPT_SPRITES.wrapping_add(sp(0).wrapping_mul(4)) as usize);
            sprite_set_texture(sprite, b);
            pop_current_txd();
        }
        // 0390 LOAD_TEXTURE_DICTIONARY (0x4840F2): label (8); path = "models\\txd\\" (12 bytes from 0x859F38),
        // `strncpy(path + 11, label, 8)`, ".txd" appended at `strlen(path)` (5 bytes from 0x859F30);
        // `slot = FindTxdSlot("script")`, `AddTxdSlot("script")` when -1; LoadTxd(slot, path); AddRef(slot);
        // with `m_UsesMissionCleanup`: MissionCleanUp.AddEntityToList(1, 0xD). Divergence note: zeroed label
        // and path buffers (the original's are stale stack bytes past the copied ones).
        0x0390 => {
            let mut buf = [0u8; 16];
            let b = label8(s, &mut buf);
            let mut path = [0u8; 48];
            path[..12].copy_from_slice(b"models\\txd\\\0");
            let pp = GPtr::from_raw(path.as_mut_ptr());
            strncpy(pp.byte_add(11), b, 8);
            let n = path.iter().position(|&c| c == 0).unwrap_or(path.len() - 5);
            path[n..n + 5].copy_from_slice(b".txd\0");
            let mut slot = find_txd_slot(GPtr::from_addr(S_SCRIPT));
            if slot == -1 {
                slot = add_txd_slot(GPtr::from_addr(S_SCRIPT));
            }
            load_txd(slot, GPtr::from_raw(path.as_mut_ptr()));
            txd_add_ref(slot);
            if uses_mission_cleanup(s) {
                mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), 1, 0xD);
            }
        }
        // 0392 SET_OBJECT_DYNAMIC (0x4841E3): object not null-checked; with `bIsStatic` (+0x1C bit 2) set and
        // P1 != 0: SetIsStatic(false) (virtual), AddToMovingList(); with it clear and P1 == 0:
        // SetIsStatic(true), RemoveFromMovingList(); otherwise nothing. gr: unregistered.
        0x0392 => {
            collect(s, 2);
            let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
            let is_static = obj.byte_add(0x1C).read() & 4 != 0;
            if sp(1) != 0 {
                if is_static {
                    set_is_static(obj, 0);
                    add_to_moving_list(obj);
                }
            } else if !is_static {
                set_is_static(obj, 1);
                remove_from_moving_list(obj);
            }
        }
        // 0393 SET_CHAR_ANIM_SPEED (0x48424A): ped looked up (not null-checked); label (24);
        // `a = RpAnimBlendClumpGetAssociation(ped->m_pRwClump (+0x18), label)`; `CollectParameters(1)`;
        // `a->m_fSpeed (+0x24) = P0` when `a`. Divergence note: zeroed 25-byte buffer.
        0x0393 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let mut buf = [0u8; 25];
            let b = GPtr::from_raw(buf.as_mut_ptr());
            read_text_label(s, b, 0x18);
            let a = clump_get_association(ped.byte_add(0x18).cast::<GPtr<u8>>().read(), b);
            collect(s, 1);
            if !a.is_null() {
                a.byte_add(0x24).cast::<u32>().write(sp(0));
            }
        }
        // 0395 CLEAR_AREA (0x4842EC): ClearExcitingStuffFromArea(&(x, y, z), P3, (u8)P4) (z: ground when
        // `<= -100`). gr: unregistered.
        0x0395 => {
            collect(s, 5);
            let v = xyz_ground();
            clear_exciting_stuff_from_area(GPtr::from_raw(v.as_ptr().cast_mut()), sp(3), sp(4) & 0xFF);
        }
        // 039C SET_CAR_WATERTIGHT (0x4843DA): vehicle not null-checked; `m_nVehicleSubType` (+0x590) 0
        // (automobile): byte +0x868 bit 2 = P1 != 0; 9 (bike): byte +0x614 bit 2; otherwise nothing.
        // gr: unregistered.
        0x039C => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let sub = veh.byte_add(0x590).cast::<u32>().read();
            if sub != 0 && sub != 9 {
                return 0;
            }
            let off = if sub == 0 { 0x868 } else { 0x614 };
            let p = veh.byte_add(off);
            let v = p.read();
            p.write(if sp(1) != 0 { v | 4 } else { v & 0xFB });
        }
        // 039F TURN_CAR_TO_FACE_COORD (0x4844B8): vehicle not null-checked; pos = `m_matrix ? m_matrix->pos :
        // m_placement.pos`; `h = GetATanOfXY(pos.x - P1, pos.y - P2) + pi/2` (x87 PC=24 `fsub`/`fadd`; the
        // callee's ST0 read as f64: its FPATAN is modelled at 53 bits); `fst` local; when `h > 2pi` (the
        // register, ordered) local = `local - 2pi`; SetHeading(local). gr: unregistered.
        0x039F => {
            collect(s, 3);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let m = veh.byte_add(0x14).cast::<GPtr<u8>>().read();
            let pos = if m.is_null() { veh.byte_add(4) } else { m.byte_add(0x30) }.cast::<u32>();
            let (px, py) = (pos.read(), pos.wrapping_index(1).read());
            let dx = X87::from_f32(f(px)).fsub_m(spf(1)).to_f32().to_bits();
            let dy = X87::from_f32(f(py)).fsub(X87::from_f32(spf(2))).to_f32().to_bits();
            let h = X87::from_f64_exact(get_atan_of_xy(dx, dy)).fadd_m(f(K_HALF_PI));
            let mut local = h.to_f32();
            if !h.fcom(X87::constant(K_TWO_PI)).test_any(0x41) {
                local = X87::from_f32(local).fsub_m(f(K_TWO_PI)).to_f32();
            }
            set_heading(veh, local.to_bits());
        }
        // 03A1 DRAW_SPHERE (0x484562): PlaceMarkerSet(m_pCurrentIP (+0x14) + this, 1, &(x, y, z), P3, 255, 0,
        // 0, 228, 2048, 0.1f, 0) (z: ground when `<= -100`). gr: unregistered.
        0x03A1 => {
            collect(s, 4);
            let v = xyz_ground();
            let id = s.byte_add(crate::vm::IP).cast::<u32>().read().wrapping_add(s.addr() as u32);
            place_marker_set(id, 1, GPtr::from_raw(v.as_ptr().cast_mut()), sp(3), 0xFF, 0, 0, 0xE4, 0x800, 0x3DCC_CCCD, 0);
        }
        // 03A2 SET_CAR_STATUS (0x484607): vehicle not null-checked; when the status (+0x36 bits 3..7) is 2
        // (`and 0xF8; cmp 0x10`) and P1 != 2: SwitchVehicleToRealPhysics(veh); then byte +0x36 = `(u8)(P1 << 3)
        // | (byte & 7)`. gr: unregistered.
        0x03A2 => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            if veh.byte_add(0x36).read() & 0xF8 == 0x10 && sp(1) != 2 {
                switch_vehicle_to_real_physics(veh);
            }
            let p = veh.byte_add(0x36);
            p.write(((sp(1) as u8) << 3) | (p.read() & 7));
        }
        // 03A9 SAVE_NEWLINE_TO_DEBUG_FILE, 03D4 HAS_IMPORT_GARAGE_SLOT_BEEN_FILLED, 03DA
        // NO_SPECIAL_CAMERA_FOR_THIS_GARAGE, 03DF FORCE_RANDOM_PED_TYPE, 03E1 GET_COLLECTABLE1S_COLLECTED
        // (0x485695): the jump table goes straight to the epilogue: 0 without reading the operands.
        0x03A9 | 0x03D4 | 0x03DA | 0x03DF | 0x03E1 => {}
        // 03B0 IS_GARAGE_OPEN (0x4847AD) / 03B1 IS_GARAGE_CLOSED (0x4847E4): label (8); `g =
        // GetGarageNumberByName(label)`; flag = `(int16)g >= 0 && IsGarageOpen/Closed(g) != 0` (EAX pushed
        // whole). gr: unregistered. Divergence note: zeroed label buffer.
        0x03B0 | 0x03B1 => {
            let mut buf = [0u8; 16];
            let b = label8(s, &mut buf);
            let g = get_garage_number_by_name(b);
            let r = (g as i16) >= 0 && {
                let v = if command == 0x03B0 { is_garage_open(g) } else { is_garage_closed(g) };
                v != 0
            };
            set_flag(s, r);
        }
        // 03B6 SWAP_NEAREST_BUILDING_MODEL (0x48481B): centre (P0, P1, P2 raw, or FindGroundZForCoord(P0, P1)
        // when `P2 <= -100`, ordered); `best = (f32)(r + r)` (`fadd st,st`), r = P3; model operands P4, P5
        // (negative: UsedObjectArray); `FindObjectsOfTypeInRange(P4, &centre, r, true, &n, 16, list, true,
        // false, false, false, false)`; nothing when `(int16)n <= 0`; over the `n` entries (u16 counter)
        // the distance `|pos - centre|` (pos = `m_matrix ? m_matrix->pos : m_placement.pos`, components
        // `fsub`ed and `fstp`ed, CVector::Magnitude) replaces `best` (and the building) when `< best`
        // (`test ah,5; jp`, ordered); with a building: ReplaceWithNewModel(P5);
        // AddToBuildingSwapArray(b, P4, P5). gr: unregistered. Divergence note: `n` and the list start
        // zeroed.
        0x03B6 => {
            collect(s, 6);
            let (x, y) = (sp(0), sp(1));
            let z = if spf(2) <= f(K_MINUS_100) {
                X87::from_f64_exact(ground_z(x, y)).to_f32().to_bits()
            } else {
                sp(2)
            };
            let r = sp(3);
            let mut best = X87::from_f32(f(r)).fdouble().to_f32();
            let m1 = model_operand(sp(4));
            let m2 = model_operand(sp(5));
            let centre = [x, y, z];
            let mut n = [0u16; 2];
            let np = GPtr::from_raw(n.as_mut_ptr());
            let mut list = [0u32; 16];
            let lp = GPtr::from_raw(list.as_mut_ptr());
            find_objects_of_type_in_range(m1, GPtr::from_raw(centre.as_ptr().cast_mut()), r, 1, np, 16, lp, 1, 0, 0, 0, 0);
            let count = i32::from(np.read() as i16);
            if count <= 0 {
                return 0;
            }
            let mut found = GPtr::<u8>::NULL;
            let mut i: u16 = 0;
            loop {
                let e = GPtr::<u8>::from_addr(lp.wrapping_index(i32::from(i)).read() as usize);
                let m = e.byte_add(0x14).cast::<GPtr<u8>>().read();
                let pos = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }.cast::<u32>();
                let mut d = [0u32; 3];
                for k in 0..3 {
                    d[k as usize] =
                        X87::from_f32(f(pos.wrapping_index(k).read())).fsub_m(f(centre[k as usize])).to_f32().to_bits();
                }
                let mag = X87::from_f64_exact(vector_magnitude(GPtr::from_raw(d.as_mut_ptr())));
                if mag.fcom(X87::from_f32(best)) == sa_math::x87::Cmp::Less {
                    best = mag.to_f32();
                    found = e;
                }
                i = i.wrapping_add(1);
                if i32::from(i) >= count {
                    break;
                }
            }
            if found.is_null() {
                return 0;
            }
            replace_with_new_model(found, m2);
            add_to_building_swap_array(found, m1, m2);
        }
        // 03BA CLEAR_AREA_OF_CARS (0x4849EB): each pair (P0,P3), (P1,P4), (P2,P5) ordered by `fld a; fcomp b;
        // test ah,0x41; jne` (swap only when `a > b`, ordered; the moved value goes through `fld; fstp`,
        // exact); ClearCarsFromArea(xmin, ymin, zmin, xmax, ymax, zmax). gr: unregistered.
        0x03BA => {
            collect(s, 6);
            let pair = |a: u32, b: u32| if f(a) > f(b) { (b, a) } else { (a, b) };
            let (x1, x2) = pair(sp(0), sp(3));
            let (y1, y2) = pair(sp(1), sp(4));
            let (z1, z2) = pair(sp(2), sp(5));
            clear_cars_from_area(x1, y1, z1, x2, y2, z2);
        }
        // 03BC ADD_SPHERE (0x484AB4): (x, y, z) (ground when `<= -100`), radius P3; the discarded
        // GetActualScriptThingIndex(CollectNextParameterWithoutIncreasingPC(), 0); P0 = AddScriptSphere(
        // m_pCurrentIP + this, (x, y, z), P3); StoreParameters(1). gr: unregistered.
        0x03BC => {
            collect(s, 4);
            let v = xyz_ground();
            let r = sp(3);
            get_actual_script_thing_index(collect_next_parameter_without_increasing_pc(s), 0);
            let id = s.byte_add(crate::vm::IP).cast::<u32>().read().wrapping_add(s.addr() as u32);
            let h = add_script_sphere(id, v[0], v[1], v[2], r);
            set_sp(0, h);
            store(s, 1);
        }
        // 03BF SET_EVERYONE_IGNORE_PLAYER (0x484B95): `m_bEverybodyBackOff` (wanted +0x1E bit 2).
        0x03BF => {
            collect(s, 2);
            wanted_back_off(4);
        }
        // 03C0 STORE_CAR_CHAR_IS_IN_NO_SAVE (0x484C06): ped not null-checked; P0 = VehiclePool.GetRef("in
        // vehicle" (+0x46C bit 8) ? m_pVehicle (+0x58C) : nullptr); StoreParameters(1).
        0x03C0 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let veh = if ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 {
                ped.byte_add(0x58C).cast::<GPtr<u8>>().read()
            } else {
                GPtr::NULL
            };
            set_sp(0, vehicle_get_ref(pool(VEHICLE_POOL), veh));
            store(s, 1);
        }
        // 03C3 DISPLAY_ONSCREEN_TIMER_WITH_STRING (0x484C57): `v = GetIndexOfGlobalVariable()`;
        // `CollectParameters(1)`; label (8); `TheText.Get(label)` (result unused); OnscnTimer.AddClock(v,
        // label, P0 != 0) (the label itself is passed). gr: unregistered. Divergence notes: zeroed label
        // buffer; the bool passed as 0/1.
        0x03C3 => {
            let v = get_index_of_global_variable(s);
            collect(s, 1);
            let down = sp(0) != 0;
            let mut buf = [0u8; 16];
            let b = label8(s, &mut buf);
            text(b);
            add_clock(GPtr::from_addr(ONSCN_TIMER), u32::from(v), b, u32::from(down));
        }
        // 03C4 DISPLAY_ONSCREEN_COUNTER_WITH_STRING (0x484CB3): `v = GetIndexOfGlobalVariable()`;
        // `CollectParameters(1)`; label (8); `TheText.Get(label)` (unused); OnscnTimer.AddCounter(v, (u16)P0,
        // label, 0). gr: unregistered. Divergence note: zeroed label buffer.
        0x03C4 => {
            let v = get_index_of_global_variable(s);
            collect(s, 1);
            let mut buf = [0u8; 16];
            let b = label8(s, &mut buf);
            text(b);
            add_counter(GPtr::from_addr(ONSCN_TIMER), u32::from(v), sp(0) & 0xFFFF, b, 0);
        }
        // 03C5 CREATE_RANDOM_CAR_FOR_CAR_PARK (0x484D08): `CollectParameters(4)`; nothing when int 0x969094 >= 45
        // (signed) or `model = PickRandomCar(0, 1)` (group 0xC0E9F8) is -1. By `ms_modelInfoPtrs[model]->
        // m_nVehicleType (+0x3C) - 1` (the inlined jump table 0x485828, unsigned, > 10 = automobile):
        // `operator new` 0x6E2D50 of 0x99C MonsterTruck / 0x9BC QuadBike / 0xA18 Heli / 0xA04 Plane / 0x814 Bike
        // / 0x838 Bmx / 0x9F4 Trailer (`ctor(model, 1)`) / 0x988 Automobile (`ctor(model, 1, 1)`); a failed new
        // gives null, and Bike/Bmx then set `+0x614 |= 0x10` on the result even when null (as the exe). Then
        // (no null checks): `pos = (fld/fstp P0, fld/fstp P1, GetDistanceFromCentreOfMassToBaseOfModel()
        // (virtual, ST0) fadd P2)` (PC=24); SetPosn(&pos); SetHeading((f32)(P3 * pi/180));
        // ClearSpaceForMissionEntity(&pos, veh); status (+0x36) = 4 (`& 7 | 0x20`), +0x428 bit 3 cleared, +0x42B
        // bit 1 set; JoinCarWithRoadSystem(veh); +0x3BA = 0 unless it is 0x39 or 0x3A; +0x3BB = 0, +0x3B9 = 0,
        // +0x3C8 = 9.0f, +0x3D0 = 9, +0x3B7 = 0, +0x3B8 = 0, +0x428 bit 4 cleared (re-read); CWorld::Add(veh).
        // gr: unregistered. Dead stack writes not ported: the EH state slot.
        0x03C5 => {
            collect(s, 4);
            if GPtr::<i32>::from_addr(CAR_PARK_CAR_COUNT).read() >= 0x2D {
                return 0;
            }
            let model = pick_random_car(GPtr::from_addr(LOADED_CAR_GROUP), 0, 1);
            if model == -1 {
                return 0;
            }
            let mi = GPtr::<GPtr<u8>>::from_addr(MODEL_INFO_PTRS.wrapping_add((model as u32).wrapping_mul(4)) as usize).read();
            let ty = mi.byte_add(0x3C).cast::<u32>().read().wrapping_sub(1);
            type Ctor = fn(GPtr<u8>, i32, u32) -> GPtr<u8>;
            let two: Option<(u32, Ctor, bool)> = match ty {
                0 => Some((0x99C, monster_truck_ctor, false)),
                1 => Some((0x9BC, quad_bike_ctor, false)),
                2 => Some((0xA18, heli_ctor, false)),
                3 => Some((0xA04, plane_ctor, false)),
                8 => Some((0x814, bike_ctor, true)),
                9 => Some((0x838, bmx_ctor, true)),
                10 => Some((0x9F4, trailer_ctor, false)),
                4..=7 | 11..=u32::MAX => None,
            };
            let veh = match two {
                Some((size, ctor, bike)) => {
                    let mem = vehicle_new(size);
                    let v = if mem.is_null() { GPtr::NULL } else { ctor(mem, model, 1) };
                    if bike {
                        let p = v.byte_add(0x614);
                        p.write(p.read() | 0x10);
                    }
                    v
                }
                None => {
                    let mem = vehicle_new(0x988);
                    if mem.is_null() { GPtr::NULL } else { automobile_ctor(mem, model, 1, 1) }
                }
            };
            let x = fld_fst(spf(0)).to_bits();
            let y = fld_fst(spf(1)).to_bits();
            let z = X87::from_f64_exact(distance_to_base(veh)).fadd_m(spf(2)).to_f32().to_bits();
            let pos = [x, y, z];
            let pp = GPtr::from_raw(pos.as_ptr().cast_mut());
            set_posn_v(veh, pp);
            set_heading(veh, X87::from_f32(spf(3)).fmul_m(f(K_DEG_TO_RAD)).to_f32().to_bits());
            clear_space_for_mission_entity(pp, veh);
            let st = veh.byte_add(0x36);
            let f428 = veh.byte_add(0x428);
            let f42b = veh.byte_add(0x42B);
            let (a, b, c) = ((st.read() & 7) | 0x20, f428.read() & 0xF7, f42b.read() | 2);
            st.write(a);
            f428.write(b);
            f42b.write(c);
            join_car_with_road_system(veh);
            let m = veh.byte_add(0x3BA);
            let mv = m.read();
            if mv != 0x39 && mv != 0x3A {
                m.write(0);
            }
            let b = f428.read() & 0xEF;
            veh.byte_add(0x3BB).write(0);
            veh.byte_add(0x3B9).write(0);
            veh.byte_add(0x3C8).cast::<u32>().write(0x4110_0000);
            veh.byte_add(0x3D0).write(9);
            veh.byte_add(0x3B7).write(0);
            veh.byte_add(0x3B8).write(0);
            f428.write(b);
            world_add(veh);
        }
        // 03D3 GET_CLOSEST_CAR_NODE_WITH_HEADING (0x485261): (x, y, z) (ground when `<= -100`); `node =
        // ThePaths.FindNodeClosestToCoors((x, y, z), 0, 999999.9f, 0, 1, 0, 0, 0)`; `c =
        // FindNodeCoorsForScript(node, &found)`; found: P0..P2 = c, P3 = `fstp
        // FindNodeOrientationForCarPlacement(node)` (ST0 read as f64), flag 1; else P0..P3 = 0, flag 0;
        // StoreParameters(4); UpdateCompareFlag. gr: unregistered. Divergence note: the `found` byte
        // starts zeroed (the original's stack byte is stale until the callee writes it).
        0x03D3 => {
            collect(s, 3);
            let v = xyz_ground();
            let paths = GPtr::from_addr(THE_PATHS);
            let mut ret = [0u32; 1];
            let n = find_node_closest_to_coors(paths, GPtr::from_raw(ret.as_mut_ptr()), v[0], v[1], v[2], 0, 0x4974_23FE, 0, 1, 0, 0, 0);
            let node = n.read();
            let mut found = [0u8; 4];
            let mut c = [0u32; 3];
            let fb = GPtr::from_raw(found.as_mut_ptr());
            find_node_coors_for_script(paths, GPtr::from_raw(c.as_mut_ptr()), node, fb);
            let ok = fb.read() != 0;
            if ok {
                set_sp(2, c[2]);
                set_sp(0, c[0]);
                set_sp(1, c[1]);
                let h = find_node_orientation_for_car_placement(paths, node);
                set_sp(3, X87::from_f64_exact(h).to_f32().to_bits());
            } else {
                for i in 0..4 {
                    set_sp(i, 0);
                }
            }
            store(s, 4);
            set_flag(s, ok);
        }
        // 03D5 CLEAR_THIS_PRINT (0x48537F) / 03D6 CLEAR_THIS_BIG_PRINT (0x4853AC): label (8);
        // ClearThisPrint / ClearThisBigPrint(TheText.Get(label)). Divergence note: zeroed label buffer.
        0x03D5 | 0x03D6 => {
            let mut buf = [0u8; 16];
            let b = label8(s, &mut buf);
            let t = text(b);
            if command == 0x03D5 {
                clear_this_print(t);
            } else {
                clear_this_big_print(t);
            }
        }
        // 03D7 SET_MISSION_AUDIO_POSITION (0x4853D9): AudioEngine.SetMissionAudioPosition((u8)P0 - 1, &(P1,
        // P2, P3)). gr: unregistered.
        0x03D7 => {
            collect(s, 4);
            let v = [sp(1), sp(2), sp(3)];
            set_mission_audio_position(GPtr::from_addr(AUDIO_ENGINE), audio_slot(), GPtr::from_raw(v.as_ptr().cast_mut()));
        }
        // 03D8 ACTIVATE_SAVE_MENU (0x48542C): no operands; FindPlayerPed(-1) is called again for every use: when
        // it is not null and not "in vehicle" (+0x46C bit 8): byte 0xBA67A7 = 1, `SetVelocity(0, 0, 0)` and
        // 0x45AFB0(0, 0, 0) on the player ped. gr: unregistered.
        0x03D8 => {
            if !find_player_ped(-1).is_null()
                && find_player_ped(-1).byte_add(0x46C).cast::<u32>().read() & 0x100 == 0
            {
                GPtr::<u8>::from_addr(SAVE_MENU_REQUEST).write(1);
                physical_set_velocity(find_player_ped(-1), 0, 0, 0);
                physical_45afb0(find_player_ped(-1), 0, 0, 0);
            }
        }
        // 03DC ADD_BLIP_FOR_PICKUP (0x4854B9): the discarded GetActualPickupIndex(P0) and
        // GetActualBlipArrayIndex(CollectNextParameterWithoutIncreasingPC()); `h = SetEntityBlip(7 (pickup),
        // P0, 6, 3 (both), m_szName (+8))`; ChangeBlipScale(h, 3); P0 = h; StoreParameters(1). gr: unregistered.
        0x03DC => {
            collect(s, 1);
            get_actual_pickup_index(spi(0));
            get_actual_blip_array_index(collect_next_parameter_without_increasing_pc(s));
            let h = set_entity_blip(7, sp(0), 6, 3, s.byte_add(8));
            change_blip_scale(h, 3);
            set_sp(0, h);
            store(s, 1);
        }
        // 03E5 PRINT_HELP (0x4855F8): label (8), `text = TheText.Get(label)`; when byte 0xA444A7 != 0, or the
        // script's byte +0xC8 != 0, or byte 0xA444A8 == 0: SetHelpMessage(text, false, false, false) and,
        // with byte 0xA4448B != 0, AddToPreviousBriefArray(text, -1 x6, nullptr); then byte 0xA4448B = 1.
        // Divergence note: zeroed label buffer.
        0x03E5 => {
            let mut buf = [0u8; 16];
            let b = label8(s, &mut buf);
            let t = text(b);
            if GPtr::<u8>::from_addr(HELP_GATE_A).read() != 0
                || s.byte_add(0xC8).read() != 0
                || GPtr::<u8>::from_addr(HELP_GATE_B).read() == 0
            {
                set_help_message(t, 0, 0, 0);
                if GPtr::<u8>::from_addr(ADD_NEXT_TO_PREVIOUS_BRIEFS).read() != 0 {
                    add_to_previous_brief_array(t, -1, -1, -1, -1, -1, -1, 0);
                }
            }
            GPtr::<u8>::from_addr(ADD_NEXT_TO_PREVIOUS_BRIEFS).write(1);
        }
        _ => return -1,
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_900_TO_999];
