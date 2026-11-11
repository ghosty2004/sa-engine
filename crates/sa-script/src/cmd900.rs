//! `CRunningScript::ProcessCommands900To999` (0x483BD0): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands900To999.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x038B, 0x0391, 0x0394, 0x0396, 0x0397, 0x039E, 0x03A3, 0x03A4, 0x03A7, 0x03A8, 0x03AA, 0x03AB, 0x03AD,
    0x03AF, 0x03B7, 0x03BD, 0x03C7, 0x03C8, 0x03C9, 0x03CA, 0x03CB, 0x03CC, 0x03CD, 0x03CE, 0x03CF, 0x03D0,
    0x03D1, 0x03D2, 0x03D9, 0x03DE, 0x03E0, 0x03E3, 0x03E4, 0x03E6, 0x03E7,
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

/// `CRunningScript::ProcessCommands900To999(int32 command)` (0x483BD0, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x483BD0, conv = thiscall, name = "CRunningScript::ProcessCommands900To999", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript", fallback)]
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
        _ => {
            return fallback(
                process_commands_900_to_999_original,
                "CRunningScript::ProcessCommands900To999",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_900_TO_999];
