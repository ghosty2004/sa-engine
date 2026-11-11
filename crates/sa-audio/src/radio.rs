//! `CAERadioTrackManager`, `CAEAmbienceTrackManager`, `CAECutsceneTrackManager`, `CAEUserRadioTrackManager` logic (PORT-AUDIO).
//! Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/AUDIO/*.md` are reading aids.
//!
//! Not hookable (entry < 5 bytes, `mov/lea; ret`): `CAERadioTrackManager::HasRadioRetuneJustStarted`
//! 0x4E8370 (`this[+6]`), `CAERadioTrackManager::GetRadioStationListenTimes` 0x4E83E0 (`this + 0x18`).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{AE_AUDIO_HARDWARE, g, rd, wr};

sa_core::globals! {
    list GLOBALS;
    /// `CAERadioTrackManager::m_nMusicTrackIndexHistory[14]` (20 `int8` each, newest first)
    pub static MUSIC_TRACK_INDEX_HISTORY @ 0xB62B40: [u8; 14 * 20] = [0; 14 * 20];
    /// `CAERadioTrackManager::m_nDJBanterIndexHistory[14]` (15 `int32` each)
    pub static DJ_BANTER_INDEX_HISTORY @ 0xB61D78: [u8; 14 * 15 * 4] = [0; 14 * 15 * 4];
    /// `CAERadioTrackManager::m_nAdvertIndexHistory[14]` (40 `int32` each)
    pub static ADVERT_INDEX_HISTORY @ 0xB620C0: [u8; 14 * 40 * 4] = [0; 14 * 40 * 4];
    /// `CAERadioTrackManager::m_nStats*` (0xB62C58..0xB62C73: last-hit hours / clock, mission flags)
    pub static RADIO_STATS @ 0xB62C58: [u8; 0x1C] = [0; 0x1C];
    /// `CAERadioTrackManager::m_nIdentIndexHistory[14]` (8 `int32` each)
    pub static IDENT_INDEX_HISTORY @ 0xB62980: [u8; 14 * 8 * 4] = [0; 14 * 8 * 4];
}

/// `FrontEndMenuManager.m_nRadioMode` (0xBA67F8, owned by the menu)
const MENU_RADIO_MODE: usize = 0xBA67F8;
/// `TheText` (`CText`)
const THE_TEXT: usize = 0xC1B340;
/// "STA%02d"-style station key format (0x85F3A0)
const FMT_STATION_KEY: usize = 0x85F3A0;
/// Station key of radio id 12 (0x85F3A8, 8 bytes)
const KEY_STATION_12: usize = 0x85F3A8;
/// Station key of radio id 13 (0x85F3B0, 8 bytes)
const KEY_STATION_13: usize = 0x85F3B0;

orig! {
    /// CRT `sprintf(buf, fmt, a)` (0x821BB5)
    pub fn sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, a: i32) -> i32 = cdecl 0x821BB5 => unported;
    /// `CText::Get(const char* key)` (0x6A0050)
    pub fn text_get(text: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CAEVehicleAudioEntity::StaticGetPlayerVehicleAudioSettingsForRadio()` (0x4F4ED0)
    pub fn player_vehicle_audio_settings_for_radio() -> GPtr<u8> = cdecl 0x4F4ED0 => unported;
    /// `CAEAudioHardware::SetChannelFlags(int16 slot, int16 channel, int16 flags)` (0x4D9500)
    pub fn hw_set_channel_flags(hw: GPtr<u8>, a: u32, b: u32, flags: u32) = thiscall 0x4D9500 => unported;
    /// `CAEAudioHardware::StartTrackPlayback()` (0x4D8F30; EAX passed through by tail calls)
    pub fn hw_start_track_playback(hw: GPtr<u8>) -> u32 = thiscall 0x4D8F30 => unported;
    /// `CAERadioTrackManager::UpdateRadioVolumes()` (0x4EA010; EAX passed through by tail calls)
    pub fn update_radio_volumes(this: GPtr<u8>) -> u32 = thiscall 0x4EA010 => unported;
    /// `CAEAudioHardware::DisableBassEq()` (0x4D94D0)
    pub fn hw_disable_bass_eq(hw: GPtr<u8>) = thiscall 0x4D94D0 => unported;
    /// `CAEAudioHardware::SetChannelVolume(int16 slot, uint16 channel, float volume, uint8)` (0x4D8870)
    pub fn hw_set_channel_volume(hw: GPtr<u8>, slot: u32, channel: u32, volume_bits: u32, b: u32) = thiscall 0x4D8870 => unported;
    /// `CAEAudioHardware::GetTrackLengthMs()` (0x4D8F70)
    pub fn hw_get_track_length_ms(hw: GPtr<u8>) -> u32 = thiscall 0x4D8F70 => unported;
    /// `CAEAudioHardware::GetPlayingTrackID()` (0x4D8F90)
    pub fn hw_get_playing_track_id(hw: GPtr<u8>) -> u32 = thiscall 0x4D8F90 => unported;
    /// `CAEAudioHardware::PlayTrack(uint32 id, int32 next, uint32 offsetMs, uint8 flags, bool, bool)` (0x4D8F10)
    pub fn hw_play_track(hw: GPtr<u8>, id: u32, next: i32, offset: u32, flags: u32, user: u32, user_next: u32) = thiscall 0x4D8F10 => unported;
    /// `CAEAudioHardware::StopTrack()` (0x4D8F50)
    pub fn hw_stop_track(hw: GPtr<u8>) = thiscall 0x4D8F50 => unported;
    /// `CAEAudioHardware::StopSound(int16 channel, uint16 slot)` (0x4D88E0)
    pub fn hw_stop_sound(hw: GPtr<u8>, channel: u32, slot: u32) = thiscall 0x4D88E0 => unported;
    /// `CAECutsceneTrackManager::StartTrackPlayback()` (0x4DBF10)
    pub fn cutscene_start_track_playback(this: GPtr<u8>) = thiscall 0x4DBF10 => unported;
    /// `CAECutsceneTrackManager::CheckForPause()` (0x4DBF60)
    pub fn cutscene_check_for_pause(this: GPtr<u8>) = thiscall 0x4DBF60 => unported;
    /// `FindPlayerVehicle(int32 playerId, bool bIncludeRemote)` (0x56E0D0)
    pub fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CAERadioTrackManager::IsVehicleRadioActive()` (0x4E9800; called with ECX = this)
    pub fn is_vehicle_radio_active_call() -> u8 = cdecl 0x4E9800 => is_vehicle_radio_active;
    /// `CAERadioTrackManager::GetRadioStationName(eRadioID)` (0x4E9E10)
    pub fn get_radio_station_name_call(this: GPtr<u8>, id: u32) -> GPtr<u8> = thiscall 0x4E9E10 => get_radio_station_name;
    /// `CFont::SetFontStyle(eFontStyle)` (0x719490)
    pub fn font_set_font_style(style: u32) = cdecl 0x719490 => unported;
    /// `CFont::SetJustify(bool)` (0x719600)
    pub fn font_set_justify(on: u32) = cdecl 0x719600 => unported;
    /// `CFont::SetBackground(bool, bool)` (0x7195C0)
    pub fn font_set_background(on: u32, proportional: u32) = cdecl 0x7195C0 => unported;
    /// `CFont::SetScale(float w, float h)` (0x719380)
    pub fn font_set_scale(w_bits: u32, h_bits: u32) = cdecl 0x719380 => unported;
    /// `CFont::SetProportional(bool)` (0x7195B0)
    pub fn font_set_proportional(on: u32) = cdecl 0x7195B0 => unported;
    /// `CFont::SetOrientation(eFontAlignment)` (0x719610)
    pub fn font_set_orientation(a: u32) = cdecl 0x719610 => unported;
    /// `CFont::SetRightJustifyWrap(float)` (0x7194F0)
    pub fn font_set_right_justify_wrap(v_bits: u32) = cdecl 0x7194F0 => unported;
    /// `CFont::SetEdge(int8)` (0x719590)
    pub fn font_set_edge(e: u32) = cdecl 0x719590 => unported;
    /// `CFont::SetDropColor(CRGBA)` (0x719510)
    pub fn font_set_drop_color(c: u32) = cdecl 0x719510 => unported;
    /// `CFont::SetColor(CRGBA)` (0x719430)
    pub fn font_set_color(c: u32) = cdecl 0x719430 => unported;
    /// `CFont::PrintString(float x, float y, const GxtChar*)` (0x71A700)
    pub fn font_print_string(x_bits: u32, y_bits: u32, text: GPtr<u8>) = cdecl 0x71A700 => unported;
    /// `CFont::DrawFonts()` (0x71A210; EAX passed through by the tail jump)
    pub fn font_draw_fonts() -> u32 = cdecl 0x71A210 => unported;
    /// `CRGBA::CRGBA(uint8 r, uint8 g, uint8 b, uint8 a)` (0x7170C0)
    pub fn rgba_ctor(this: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CHudColours::GetRGB(eHudColours)` (0x58FEA0; CRGBA through the hidden pointer)
    pub fn hud_colours_get_rgb(this: GPtr<u8>, out: GPtr<u8>, colour: u32) -> GPtr<u8> = thiscall 0x58FEA0 => unported;
    /// `CAudioEngine::IsCutsceneTrackActive()` (0x507150)
    pub fn engine_is_cutscene_track_active(engine: GPtr<u8>) -> u8 = thiscall 0x507150 => unported;
    /// `CAEAmbienceTrackManager::UpdateAmbienceTrackAndVolume()` (0x4D6E60)
    pub fn ambience_update_track_and_volume(this: GPtr<u8>) = thiscall 0x4D6E60 => unported;
    /// `CAEAmbienceTrackManager::StartTrackPlayback()` (0x4D6DA0)
    pub fn ambience_start_track_playback_call(this: GPtr<u8>) -> u32 = thiscall 0x4D6DA0 => ambience_start_track_playback;
    /// `CAEAmbienceTrackManager::CheckForPause()` (0x4D6DF0)
    pub fn ambience_check_for_pause_call(this: GPtr<u8>) = thiscall 0x4D6DF0 => ambience_check_for_pause;
    /// `CAudioEngine::GetCutsceneTrackStatus()` (0x507160)
    pub fn engine_get_cutscene_track_status(engine: GPtr<u8>) -> u8 = thiscall 0x507160 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn pad_get_pad(id: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::NextStationJustUp()` (0x5405B0)
    pub fn pad_next_station_just_up(pad: GPtr<u8>) -> u8 = thiscall 0x5405B0 => unported;
    /// `CPad::LastStationJustUp()` (0x5405E0)
    pub fn pad_last_station_just_up(pad: GPtr<u8>) -> u8 = thiscall 0x5405E0 => unported;
    /// `CAERadioTrackManager::QueueUpTracksForStation(eRadioID, int8* index, eTrackType, tRadioSettings*)` (0x4EA670)
    pub fn queue_up_tracks_for_station(this: GPtr<u8>, station: u32, index: GPtr<u8>, ty: u32, settings: GPtr<u8>) -> u8 = thiscall 0x4EA670 => unported;
    /// `CAEAudioUtility::ResolveProbability(float)` (0x4D9C80)
    pub fn resolve_probability(p: f32) -> u8 = cdecl 0x4D9C80 => crate::utility::resolve_probability;
    /// `CAERadioTrackManager::AddMusicTrackIndexToHistory(eRadioID, int8)` (0x4E96C0)
    pub fn add_music_track_index_to_history_call(this: GPtr<u8>, id: u32, index: u32) = thiscall 0x4E96C0 => add_music_track_index_to_history;
    /// `CStats::GetFullFavoriteRadioStationList()` (0x558F90)
    pub fn stats_get_full_favorite_radio_station_list() -> GPtr<u8> = cdecl 0x558F90 => unported;
    /// `CAEUserRadioTrackManager::GetUserTrackPlayMode()` (0x4F3330)
    pub fn get_user_track_play_mode_call(this: GPtr<u8>) -> u8 = thiscall 0x4F3330 => get_user_track_play_mode;
    /// `CStats::GetStatValue(uint16 stat)` (0x558E40, ST0)
    pub fn stats_get_stat_value(stat: u32) -> f32 = cdecl 0x558E40 => unported;
    /// `CAERadioTrackManager::GetRadioStationNameKey(eRadioID, char*)` (0x4E8380)
    /// `CAEAudioHardware::SetBassSetting(int8 setting, float gain)` (0x4D94A0)
    pub fn hw_set_bass_setting(hw: GPtr<u8>, setting: u32, gain: f32) = thiscall 0x4D94A0 => unported;
    /// `CAEUserRadioTrackManager::ReadUserTracks()` (0x4F2FD0)
    pub fn read_user_tracks(this: GPtr<u8>) -> u8 = thiscall 0x4F2FD0 => read_user_tracks_port;
    /// `CFileMgr::SetDirMyDocuments()` (0x538860)
    pub fn file_set_dir_my_documents() = cdecl 0x538860 => unported;
    /// `CFileMgr::SetDir(const char*)` (0x5387D0)
    pub fn file_set_dir(dir: GPtr<u8>) = cdecl 0x5387D0 => unported;
    /// `CFileMgr::OpenFile(const char* name, const char* mode)` (0x538900)
    pub fn file_open(name: GPtr<u8>, mode: GPtr<u8>) -> i32 = cdecl 0x538900 => unported;
    /// `CFileMgr::GetTotalSize(int32 file)` (0x5389E0)
    pub fn file_get_total_size(file: i32) -> u32 = cdecl 0x5389E0 => unported;
    /// `CFileMgr::Read(int32 file, void* buf, int32 size)` (0x538950)
    pub fn file_read(file: i32, buf: GPtr<u8>, size: u32) -> i32 = cdecl 0x538950 => unported;
    /// `CFileMgr::CloseFile(int32 file)` (0x5389D0)
    pub fn file_close(file: i32) -> i32 = cdecl 0x5389D0 => unported;
    /// `CMemoryMgr::Malloc(uint32 size, uint32 hint)` (0x72F420)
    pub fn memory_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// `CAEWMADecoder::InitLibrary()` (0x502B80)
    pub fn wma_init_library() -> u8 = cdecl 0x502B80 => unported;
    /// `CAEMFDecoder::InitLibrary()` (0x4E7C70)
    pub fn mf_init_library() -> u8 = cdecl 0x4E7C70 => unported;
    /// `CAEAudioHardware::SetChannelFrequencyScalingFactor(int16 slot, int16 channel, float factor)` (0x4D8960)
    pub fn hw_set_channel_frequency_scaling_factor(hw: GPtr<u8>, slot: u32, channel: u32, factor_bits: u32) = thiscall 0x4D8960 => unported;
    /// `CAudioEngine::IsAmbienceRadioActive()` (0x507280)
    pub fn engine_is_ambience_radio_active(engine: GPtr<u8>) -> u8 = thiscall 0x507280 => unported;
    /// `CAERadioTrackManager::StopRadio(tRadioSettings*, bool)` (0x4E9820)
    pub fn stop_radio(this: GPtr<u8>, settings: GPtr<u8>, b: u32) = thiscall 0x4E9820 => unported;
    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float volumeChange, float speed)` (0x506EA0)
    pub fn report_frontend_audio_event(engine: GPtr<u8>, event: i32, volume: f32, speed: f32) = thiscall 0x506EA0 => unported;
    /// `CAERadioTrackManager::ChooseTalkRadioShow()` (0x4E8E40)
    pub fn choose_talk_radio_show(this: GPtr<u8>) -> i32 = thiscall 0x4E8E40 => unported;
    /// `CAEAudioUtility::GetRandomNumberInRange<int32>(int32, int32)` (0x4D9C10)
    pub fn ae_random_int(min: i32, max: i32) -> i32 = cdecl 0x4D9C10 => crate::utility::get_random_number_in_range_int;
    /// `CAERadioTrackManager::StartRadio(eRadioID, eBassSetting, float bassGain, uint8)` (0x4EB3C0)
    pub fn start_radio_manual(this: GPtr<u8>, station: u32, bass: u32, gain_bits: u32, b: u32) = thiscall 0x4EB3C0 => start_radio_manual_port;
    /// `CAudioEngine::IsAmbienceTrackActive()` (0x507210)
    pub fn engine_is_ambience_track_active(engine: GPtr<u8>) -> u8 = thiscall 0x507210 => unported;
    /// `CAudioEngine::DoesAmbienceTrackOverrideRadio()` (0x507270)
    pub fn engine_does_ambience_track_override_radio(engine: GPtr<u8>) -> u8 = thiscall 0x507270 => unported;
    /// `CAudioEngine::StopAmbienceTrack(bool)` (0x507220)
    pub fn engine_stop_ambience_track(engine: GPtr<u8>, b: u32) = thiscall 0x507220 => unported;
    /// `CAERadioTrackManager::TrackRadioStation(eRadioID, uint8)` (0x4EAC30)
    pub fn track_radio_station(this: GPtr<u8>, station: u32, b: u32) -> u8 = thiscall 0x4EAC30 => unported;
    /// `CAERadioTrackManager::ChooseTracksForStation(eRadioID)` (0x4EB180)
    pub fn choose_tracks_for_station(this: GPtr<u8>, station: u32) = thiscall 0x4EB180 => choose_tracks_for_station_port;
    pub fn get_radio_station_name_key_call(this: GPtr<u8>, id: u32, out: GPtr<u8>) = thiscall 0x4E8380 => get_radio_station_name_key;
}

fn local(buf: &mut [u8]) -> GPtr<u8> {
    GPtr::from_raw(buf.as_mut_ptr())
}

// ---------------------------------------------------------------------------------------------------
// CAERadioTrackManager
// ---------------------------------------------------------------------------------------------------

/// `CAERadioTrackManager::SetRadioAutoRetuneOnOff(bool)` (0x4E82E0): `this[+7]`.
#[hook(addr = 0x4E82E0, conv = thiscall, name = "CAERadioTrackManager::SetRadioAutoRetuneOnOff", expect = "8A 44 24 04 88 41 07")]
pub fn set_radio_auto_retune_on_off(this: GPtr<u8>, on: u8) {
    wr::<u8>(this, 7, on);
}

/// `CAERadioTrackManager::IsRadioOn()` (0x4E8350): false only when the mode (+0x68) is 7 (stopped), the
/// initialised byte (+0) is 0 and the station list counters (+0x6C, +0x70) are 0.
#[hook(addr = 0x4E8350, conv = thiscall, name = "CAERadioTrackManager::IsRadioOn", expect = "83 79 68 07 75 16")]
pub fn is_radio_on(this: GPtr<u8>) -> u8 {
    let off = rd::<u32>(this, 0x68) == 7
        && rd::<u8>(this, 0) == 0
        && rd::<u32>(this, 0x6C) == 0
        && rd::<u32>(this, 0x70) == 0;
    u8::from(!off)
}

/// `CAERadioTrackManager::IsVehicleRadioActive()` (0x4E9800, static): the player vehicle's radio
/// settings exist and their radio type (+0x1B) is 0, 2 or 3.
#[hook(addr = 0x4E9800, conv = cdecl, name = "CAERadioTrackManager::IsVehicleRadioActive", expect = "E8 CB B6 00 00")]
pub fn is_vehicle_radio_active() -> u8 {
    let s = player_vehicle_audio_settings_for_radio();
    if s.is_null() {
        return 0;
    }
    u8::from(matches!(rd::<u8>(s, 0x1B), 0 | 2 | 3))
}

/// `CAERadioTrackManager::GetRadioStationNameKey(eRadioID id, char* out)` (0x4E8380): ids 13 and 12
/// copy fixed 8-byte keys, others `sprintf(out, fmt, id - 1)`.
#[hook(addr = 0x4E8380, conv = thiscall, name = "CAERadioTrackManager::GetRadioStationNameKey", expect = "8A 44 24 04 3C 0D")]
pub fn get_radio_station_name_key(_this: GPtr<u8>, id: u32, out: GPtr<u8>) {
    let id = id as u8;
    let src = match id {
        13 => KEY_STATION_13,
        12 => KEY_STATION_12,
        _ => {
            sprintf1(out, g(FMT_STATION_KEY), i32::from(id as i8) - 1);
            return;
        }
    };
    wr::<u32>(out, 0, g::<u32>(src).read());
    wr::<u32>(out, 4, g::<u32>(src + 4).read());
}

/// `CAERadioTrackManager::GetRadioStationName(eRadioID id)` (0x4E9E10): null for ids <= 0, otherwise
/// `TheText.Get(key)` with the key built in an 8-byte local.
#[hook(addr = 0x4E9E10, conv = thiscall, name = "CAERadioTrackManager::GetRadioStationName", expect = "8B 44 24 04 83 EC 08")]
pub fn get_radio_station_name(this: GPtr<u8>, id: u32) -> GPtr<u8> {
    if (id as u8 as i8) <= 0 {
        return GPtr::from_addr(0);
    }
    let mut key = [0u8; 8];
    get_radio_station_name_key_call(this, id, local(&mut key));
    text_get(g(THE_TEXT), local(&mut key))
}

/// `CAERadioTrackManager::AddMusicTrackIndexToHistory(eRadioID id, int8 index)` (0x4E96C0): negative
/// indices and a repeat of the newest entry are ignored; otherwise the station's 20-entry history
/// shifts by one, `index` goes first and the station's tracks-in-a-row counter (`this[+8 + id]`) grows.
#[hook(addr = 0x4E96C0, conv = thiscall, name = "CAERadioTrackManager::AddMusicTrackIndexToHistory", expect = "57 8B F9 8A 4C 24 0C")]
pub fn add_music_track_index_to_history(this: GPtr<u8>, id: u32, index: u32) {
    let index = index as u8;
    if (index as i8) < 0 {
        return;
    }
    let id = isize::from(id as u8 as i8);
    let h = MUSIC_TRACK_INDEX_HISTORY.ptr().cast::<u8>().byte_add(id * 20);
    if rd::<u8>(h, 0) == index {
        return;
    }
    for k in (1..20).rev() {
        let v = rd::<u8>(h, k - 1);
        wr::<u8>(h, k, v);
    }
    wr::<u8>(h, 0, index);
    let c = this.byte_add(id + 8);
    c.cast::<u8>().write(rd::<u8>(c, 0).wrapping_add(1));
}

/// `CAERadioTrackManager::StartTrackPlayback()` (0x4EA640): `SetChannelFlags((uint16)m_HwClientHandle,
/// 0, 0x37)`, `StartTrackPlayback()` on the hardware, then `UpdateRadioVolumes()`.
#[hook(addr = 0x4EA640, conv = thiscall, name = "CAERadioTrackManager::StartTrackPlayback", expect = "56 8B F1 33 C0")]
pub fn start_track_playback(this: GPtr<u8>) -> u32 {
    let hw = g::<u8>(AE_AUDIO_HARDWARE);
    hw_set_channel_flags(hw, u32::from(rd::<u16>(this, 0x64)), 0, 0x37);
    hw_start_track_playback(hw);
    update_radio_volumes(this)
}

/// `CAERadioTrackManager::SetBassEnhanceOnOff(bool)` (0x4E9DB0): `m_bBassEnhance` (+4); while the
/// radio plays (mode +0x68 == 2) the active settings' bass (+0xEA) and gain (+0xEC) are copied to the
/// requested settings (+0xAE, +0xB0) and applied with `SetBassSetting(on ? bass : 0, gain)`.
#[hook(addr = 0x4E9DB0, conv = thiscall, name = "CAERadioTrackManager::SetBassEnhanceOnOff", expect = "51 83 79 68 02")]
pub fn set_bass_enhance_on_off(this: GPtr<u8>, on: u32) {
    let on = on as u8;
    wr::<u8>(this, 4, on);
    if rd::<u32>(this, 0x68) != 2 {
        return;
    }
    let gain = crate::common::x(rd(this, 0xEC)).to_f32(); // fld / fst: an SNaN is quieted
    let bass = rd::<u8>(this, 0xEA);
    wr::<f32>(this, 0xB0, gain);
    wr::<u8>(this, 0xAE, bass);
    let hw = g::<u8>(AE_AUDIO_HARDWARE);
    if on != 0 {
        // the setting is pushed from the `push ecx` slot (this) with its low byte replaced
        let setting = (this.addr() as u32 & 0xFFFF_FF00) | u32::from(bass);
        hw_set_bass_setting(hw, setting, gain);
    } else {
        hw_set_bass_setting(hw, 0, gain);
    }
}

/// `CTimer::m_UserPause` / `CTimer::m_CodePause` (owned by sa-core's timer)
const USER_PAUSE: usize = 0xB7CB49;
const CODE_PAUSE: usize = 0xB7CB48;
/// `FrontEndMenuManager.m_bMenuActive` (0xBA6831)
const MENU_ACTIVE: usize = 0xBA6831;
/// `CAERadioTrackManager::m_nNumMusicTracks...` per-station track counts (0x8C9010, int32[])
const STATION_TRACK_COUNTS: u32 = 0x8C9010;

fn paused() -> bool {
    g::<u8>(USER_PAUSE).read() != 0 || g::<u8>(CODE_PAUSE).read() != 0
}

/// `CAERadioTrackManager::CheckForPause()` (0x4EA590): paused: frequency factor 1.0 when enabled in
/// pause mode (+3), else 0.0, and the pause flag (+5) set; the player vehicle's radio active (type 0/2/3)
/// or the ambience radio active: factor 1.0, flag cleared; otherwise `StopRadio(null, false)` and
/// front-end event 0x23, flag cleared.
#[hook(addr = 0x4EA590, conv = thiscall, name = "CAERadioTrackManager::CheckForPause", expect = "51 A0 49 CB B7 00")]
pub fn radio_check_for_pause(this: GPtr<u8>) {
    let factor: f32;
    if paused() {
        factor = if rd::<u8>(this, 3) != 0 { 1.0 } else { 0.0 };
        wr::<u8>(this, 5, 1);
    } else {
        let s = player_vehicle_audio_settings_for_radio();
        let active = !s.is_null() && matches!(rd::<u8>(s, 0x1B), 0 | 2 | 3);
        if !active && engine_is_ambience_radio_active(g(crate::common::AUDIO_ENGINE)) == 0 {
            stop_radio(this, GPtr::from_addr(0), 0);
            report_frontend_audio_event(g(crate::common::AUDIO_ENGINE), 0x23, 0.0, 1.0);
            wr::<u8>(this, 5, 0);
            return;
        }
        factor = 1.0;
        wr::<u8>(this, 5, 0);
    }
    hw_set_channel_frequency_scaling_factor(
        g(AE_AUDIO_HARDWARE),
        u32::from(rd::<u16>(this, 0x64)),
        0,
        factor.to_bits(),
    );
}

/// `CAERadioTrackManager::ChooseMusicTrackIndex(eRadioID id)` (0x4EA270, not reversed by
/// gta-reversed): id 11 asks `ChooseTalkRadioShow()`; otherwise draws `GetRandomNumberInRange(0,
/// count - 1)` until its low byte is none of the station's newest `min(20, count - 2)` history entries.
#[hook(addr = 0x4EA270, conv = thiscall, name = "CAERadioTrackManager::ChooseMusicTrackIndex", expect = "8A 44 24 04 3C 0B")]
pub fn choose_music_track_index(this: GPtr<u8>, id: u32) -> i32 {
    let id = id as u8;
    if id == 11 {
        return choose_talk_radio_show(this);
    }
    let st = isize::from(id as i8);
    let count_at =
        || g::<i32>(STATION_TRACK_COUNTS.wrapping_add((st as u32).wrapping_mul(4)) as usize).read();
    let h = MUSIC_TRACK_INDEX_HISTORY.ptr().cast::<u8>().byte_add(st * 20);
    'draw: loop {
        let r = ae_random_int(0, count_at().wrapping_sub(1));
        let n = count_at().wrapping_sub(2).min(20);
        let mut k: i8 = 0;
        while i32::from(k) < n {
            if r as u8 == rd::<u8>(h, isize::from(k)) {
                continue 'draw;
            }
            k = k.wrapping_add(1);
        }
        return r;
    }
}

/// Ident index ranges per station (0x8C8FB0: start, end int32 pairs; start 0x782 = none)
const IDENT_RANGES: u32 = 0x8C8FB0;
/// DJ banter index ranges per station (0x8C8CB0, same layout), sizing the banter history check
const DJ_BANTER_RANGES: u32 = 0x8C8CB0;
/// "No entries" marker of the range tables
const NO_RANGE: i32 = 0x782;

/// `(start, end)` of entry `id` of an int32-pair table at `base` (32-bit wrapping address).
fn range(base: u32, id: isize) -> (i32, i32) {
    let a = base.wrapping_add((id as u32).wrapping_mul(8)) as usize;
    (g::<i32>(a).read(), g::<i32>(a + 4).read())
}

/// `CAERadioTrackManager::ChooseIdentIndex(eRadioID id)` (0x4E94C0, not reversed by gta-reversed): -1
/// without idents; otherwise draws `GetRandomNumberInRange(start, end)` until it is not one of the
/// station's newest `min(8, end - start - 1)` idents, and (station 6) not ident 0x44C while stat 0x152
/// is 0.
#[hook(addr = 0x4E94C0, conv = thiscall, name = "CAERadioTrackManager::ChooseIdentIndex", expect = "53 8A 5C 24 08")]
pub fn choose_ident_index(_this: GPtr<u8>, id: u32) -> i32 {
    let id = id as u8;
    let st = isize::from(id as i8);
    if range(IDENT_RANGES, st).0 == NO_RANGE {
        return -1;
    }
    let h = IDENT_INDEX_HISTORY.ptr().cast::<u8>().byte_add(st * 32);
    'draw: loop {
        let (start, end) = range(IDENT_RANGES, st);
        let r = ae_random_int(start, end);
        // fcomp 0.0; test ah, 44h; jnp -> draw again (equal)
        if id == 6
            && r == 0x44C
            && !crate::common::x(stats_get_stat_value(0x152))
                .fcom(crate::common::x(0.0))
                .test_parity(0x44)
        {
            continue 'draw;
        }
        let (start, end) = range(IDENT_RANGES, st);
        let n = end.wrapping_sub(start).wrapping_sub(1).min(8);
        let mut k = 0i32;
        while k < n {
            if r == rd::<i32>(h, k as isize * 4) {
                continue 'draw;
            }
            k += 1;
        }
        return r;
    }
}

/// `CAERadioTrackManager::ChooseDJBanterIndexFromList(eRadioID id, const int32 (*list)[2])` (0x4E95E0,
/// not reversed by gta-reversed): -1 without entries; otherwise one draw `r` in `[0, count)` and the
/// first of `start + (r + k) % count` (k = 0..count) that is none of the station's newest
/// `min(15, banterEnd - banterStart - 1)` banter indices (ranges of 0x8C8CB0); -1 when all are.
#[hook(addr = 0x4E95E0, conv = thiscall, name = "CAERadioTrackManager::ChooseDJBanterIndexFromList", expect = "53 0F BE 5C 24 08")]
pub fn choose_dj_banter_index_from_list(_this: GPtr<u8>, id: u32, list: GPtr<u8>) -> i32 {
    let st = isize::from(id as u8 as i8);
    let e = list.byte_add(st.wrapping_mul(8));
    let start = rd::<i32>(e, 0);
    if start == NO_RANGE {
        return -1;
    }
    let count = rd::<i32>(e, 4).wrapping_sub(start).wrapping_add(1);
    let r = ae_random_int(0, count.wrapping_sub(1));
    if count <= 0 {
        return -1;
    }
    let (b0, b1) = range(DJ_BANTER_RANGES, st);
    let m = b1.wrapping_sub(b0).wrapping_sub(1);
    let start = rd::<i32>(e, 0);
    let h = DJ_BANTER_INDEX_HISTORY.ptr().cast::<u8>();
    let mut k = 0i32;
    while k < count {
        let cand = k.wrapping_add(r).wrapping_rem(count).wrapping_add(start);
        let n = m.min(15);
        let mut c = 0i32;
        let mut hit = false;
        while c < n {
            let idx = (st as i32).wrapping_mul(15).wrapping_add(c);
            if cand == rd::<i32>(h, idx as isize * 4) {
                hit = true;
                break;
            }
            c += 1;
        }
        if !hit {
            return cand;
        }
        k += 1;
    }
    -1
}

/// `TheCamera` byte +0x3D (0xB6F065), tested by the HUD-style draws
const CAM_BYTE_B6F065: usize = 0xB6F065;
/// `RsGlobal.maximumWidth` / `maximumHeight`
const RS_MAXIMUM_WIDTH: usize = 0xC17044;
const RS_MAXIMUM_HEIGHT: usize = 0xC17048;
/// `HudColour` (`CHudColours`)
const HUD_COLOUR: usize = 0xBAB22C;

/// `fild dword [g]; fmul [a]; fmul [b]` as an f32.
fn scaled(g_addr: usize, a: usize, b: usize) -> f32 {
    crate::common::xi(g::<i32>(g_addr).read())
        .fmul_m(g::<f32>(a).read())
        .fmul_m(g::<f32>(b).read())
        .to_f32()
}

/// `CAERadioTrackManager::DisplayRadioStationName()` (0x4E9E50): not while paused, with the camera byte
/// 0xB6F065 set, without a player vehicle or during replay playback. A pending display request (+1)
/// with an active vehicle radio starts a 2.5 s display (+0x54). While it lasts, the station `requested
/// (+0xAD) + list offset (+0x6C)` (0 = none; wrapped into 1..13) is printed at the top centre in font
/// style 2 with the HUD colour 6 (8 while the station list moves).
#[hook(addr = 0x4E9E50, conv = thiscall, name = "CAERadioTrackManager::DisplayRadioStationName", expect = "51 A0 49 CB B7 00")]
pub fn display_radio_station_name(this: GPtr<u8>) {
    if paused() || g::<u8>(CAM_BYTE_B6F065).read() != 0 {
        return;
    }
    if find_player_vehicle(-1, 0).is_null() || g::<u8>(REPLAY_MODE).read() == 1 {
        return;
    }
    if rd::<u8>(this, 1) != 0 && is_vehicle_radio_active_call() != 0 {
        wr::<u32>(this, 0x54, sa_core::timer::TIME_MS.get().wrapping_add(2500));
        wr::<u8>(this, 1, 0);
    }
    if sa_core::timer::TIME_MS.get() >= rd::<u32>(this, 0x54) {
        return;
    }
    let mut id = i32::from(rd::<u8>(this, 0xAD) as i8).wrapping_add(rd::<i32>(this, 0x6C));
    if id == 0 {
        return;
    }
    if id <= 0 {
        id += 13;
    } else if id >= 14 {
        id -= 13;
    }
    let text = get_radio_station_name_call(this, id as u32);
    font_set_font_style(2);
    font_set_justify(0);
    font_set_background(0, 0);
    let sy = scaled(RS_MAXIMUM_HEIGHT, 0x859524, 0x858C20);
    let sx = scaled(RS_MAXIMUM_WIDTH, 0x859520, 0x858CC8);
    font_set_scale(sx.to_bits(), sy.to_bits());
    font_set_proportional(1);
    font_set_orientation(0);
    font_set_right_justify_wrap(0);
    font_set_edge(1);
    let mut rgba = [0u8; 4];
    let p = rgba_ctor(local(&mut rgba), 0, 0, 0, 0xFF);
    font_set_drop_color(rd::<u32>(p, 0));
    let colour = if rd::<u32>(this, 0x6C) == 0 && rd::<u32>(this, 0x70) == 0 {
        6
    } else {
        8
    };
    let mut out = [0u8; 4];
    let c = hud_colours_get_rgb(g(HUD_COLOUR), local(&mut out), colour);
    font_set_color(rd::<u32>(c, 0));
    let py = scaled(RS_MAXIMUM_HEIGHT, 0x859524, 0x858F8C);
    let px = crate::common::xi(g::<i32>(RS_MAXIMUM_WIDTH).read() / 2).to_f32();
    font_print_string(px.to_bits(), py.to_bits(), text);
    font_draw_fonts();
}

/// Word tested before retuning to station 12 (0xB6B976: user tracks available)
const USER_TRACKS_WORD: usize = 0xB6B976;
/// `TheCamera` float +0x124 (0xB6F14C) compared with the constant at 0x858C20 for the retune delay
const CAM_FLOAT_B6F14C: usize = 0xB6F14C;
const RETUNE_DELAY_LIMIT: usize = 0x858C20;

/// `CAERadioTrackManager::CheckForStationRetune()` (0x4EB660): with a station (+0xE9) and the radio
/// on or starting, no cutscene track and an active player-vehicle radio of type 0 (not in replay), a
/// script request (+0x7C) or the pad's next / last station buttons move the list offset (+0x6C, old
/// value +0x70, time +0x50, flags +1 / +6). A pending offset retunes: to station `requested + offset`
/// (wrapped into 1..13; the byte is built in the `push ecx` slot, so the argument's upper bytes are
/// `this`'s) after stopping the radio (station 13 / 12 without user tracks: at once, events 0x26 /
/// 0x23), else after 1.5 s and 2 s / 4 s (camera value at 0xB6F14C above 0x858C20) since +0x60.
#[hook(addr = 0x4EB660, conv = thiscall, name = "CAERadioTrackManager::CheckForStationRetune", expect = "51 56 8B F1 8A 8E E9 00 00 00")]
pub fn check_for_station_retune(this: GPtr<u8>) {
    let cur = rd::<u8>(this, 0xE9);
    if cur == 0 {
        return;
    }
    let engine = g::<u8>(crate::common::AUDIO_ENGINE);
    let mode = rd::<u32>(this, 0x68);
    wr::<u8>(this, 6, 0);
    let now = || sa_core::timer::TIME_MS.get();
    let input = mode <= 2
        || rd::<u8>(this, 0) != 0
        || rd::<u32>(this, 0x6C) != 0
        || rd::<u32>(this, 0x70) != 0
        || cur == 13;
    'input: {
        if !input || engine_get_cutscene_track_status(engine) != 0 {
            break 'input;
        }
        let s = player_vehicle_audio_settings_for_radio();
        if s.is_null() || !matches!(rd::<u8>(s, 0x1B), 0 | 2 | 3) || g::<u8>(REPLAY_MODE).read() == 1 {
            break 'input;
        }
        if rd::<u8>(player_vehicle_audio_settings_for_radio(), 0x1B) != 0 {
            return;
        }
        let req = rd::<i32>(this, 0x7C);
        if req >= 0 {
            let old = rd::<i32>(this, 0x6C);
            let off = req.wrapping_sub(i32::from(rd::<u8>(this, 0xAD) as i8));
            wr::<i32>(this, 0x70, old);
            wr::<i32>(this, 0x6C, off);
            wr::<i32>(this, 0x7C, -1);
        } else if pad_next_station_just_up(pad_get_pad(0)) != 0 {
            let old = rd::<i32>(this, 0x6C);
            wr::<i32>(this, 0x70, old);
            wr::<i32>(this, 0x6C, old.wrapping_add(1));
        } else if pad_last_station_just_up(pad_get_pad(0)) != 0 {
            let old = rd::<i32>(this, 0x6C);
            wr::<i32>(this, 0x70, old);
            wr::<i32>(this, 0x6C, old.wrapping_sub(1));
        } else {
            break 'input;
        }
        wr::<u32>(this, 0x50, now());
        wr::<u8>(this, 1, 1);
        wr::<u8>(this, 6, 1);
    }
    if rd::<u32>(this, 0x6C) == 0 && rd::<u32>(this, 0x70) == 0 {
        return;
    }
    let mut st = rd::<u8>(this, 0xAD).wrapping_add(rd::<u8>(this, 0x6C));
    if (st as i8) <= 0 {
        st = st.wrapping_add(13);
    } else if (st as i8) >= 14 {
        st = st.wrapping_add(0xF3);
    }
    let slot = (this.addr() as u32 & 0xFFFF_FF00) | u32::from(st);
    if st == 13 || (st == 12 && g::<u16>(USER_TRACKS_WORD).read() == 0) {
        stop_radio(this, GPtr::from_addr(0), 0);
        report_frontend_audio_event(engine, 0x26, 0.0, 1.0);
        report_frontend_audio_event(engine, 0x23, 0.0, 1.0);
    } else {
        if rd::<u8>(this, 0xE9) == 13 {
            report_frontend_audio_event(engine, 0x25, 0.0, 1.0);
            wr::<u8>(this, 0xE9, 0xFF);
        } else {
            stop_radio(this, GPtr::from_addr(0), 0);
        }
        report_frontend_audio_event(engine, 0x22, 0.0, 1.0);
        let far = crate::common::x(g::<f32>(CAM_FLOAT_B6F14C).read())
            .fcom(crate::common::x(g::<f32>(RETUNE_DELAY_LIMIT).read()));
        let delay: u32 = if far.test_any(0x41) { 2000 } else { 4000 };
        let t = now();
        if t <= rd::<u32>(this, 0x50).wrapping_add(1500) || t <= rd::<u32>(this, 0x60).wrapping_add(delay) {
            return;
        }
    }
    start_radio_manual(
        this,
        slot,
        u32::from(rd::<u8>(this, 0xEA)),
        rd::<u32>(this, 0xEC),
        0,
    );
    wr::<u32>(this, 0x6C, 0);
    wr::<u32>(this, 0x70, 0);
}

/// `CAERadioTrackManager::ChooseTracksForStation(eRadioID station)` (0x4EB180): resets the requested
/// queue (+0x88: 5 track ids -1, types 6, indices -1) and queues tracks with `QueueUpTracksForStation`
/// (types: 0 ident?, 1, 2, 3, 4, 5 music, as the original passes them) chosen by `ResolveProbability`
/// draws (0.95, 0.17, 0.9, 0.5), in exactly the original's branch order. The station argument is passed
/// on as the full dword it arrived in.
#[hook(addr = 0x4EB180, conv = thiscall, name = "CAERadioTrackManager::ChooseTracksForStation", expect = "51 53 56 8B F1 57")]
pub fn choose_tracks_for_station_port(this: GPtr<u8>, station: u32) {
    let settings = this.byte_add(0x88);
    for k in 0..5isize {
        wr::<i32>(this, 0x88 + 4 * k, -1);
        wr::<u8>(this, 0xB4 + k, 6);
        wr::<u8>(this, 0xBB + k, 0xFF);
    }
    let mut idx = [0u8; 1];
    let ip = local(&mut idx);
    let q = |st: u32, ty: u32| queue_up_tracks_for_station(this, st, ip, ty, settings);
    let rp = |p: u32| resolve_probability(f32::from_bits(p)) != 0;
    const P95: u32 = 0x3F73_3333;
    const P17: u32 = 0x3E2E_147B;
    const P90: u32 = 0x3F66_6666;
    const P50: u32 = 0x3F00_0000;
    let bl = station as u8;
    if rp(P95) {
        if bl == 12 {
            q(12, 4);
            q(12, 4);
            if g::<u8>(MENU_RADIO_MODE).read() == 0 && rp(P17) {
                q(12, 1);
            }
            return;
        }
        if rp(P90) {
            q(station, 4);
            return;
        }
        if !rp(P50) {
            q(station, 5);
            let i = isize::from(rd::<u8>(ip, 0) as i8);
            add_music_track_index_to_history_call(this, station, u32::from(rd::<u8>(this, 0xBA + i)));
            if bl == 0 {
                q(0, 2);
                return;
            }
            if !rp(P50) {
                if rp(P50) {
                    q(station, 0);
                }
                if q(station, 2) == 0 {
                    q(station, 1);
                }
                return;
            }
        }
        // 0x4EB26B
        if rp(P50) {
            q(station, 0);
        }
        q(station, 3);
        return;
    }
    if bl == 0 {
        q(0, 2);
        q(station, 3);
        return;
    }
    if rp(P50) {
        q(station, 0);
    }
    if q(station, 2) == 0 {
        q(station, 1);
    }
    if bl == 12 {
        q(12, 4);
    } else {
        q(station, 3);
    }
}

/// `AEUserRadioTrackManager` (`CAEUserRadioTrackManager`)
const AE_USER_RADIO_TRACK_MANAGER: usize = 0xB6B970;

/// `CAERadioTrackManager::Initialise(int32 hwClientHandle)` (0x5B9390): every field and static reset in
/// the original's order: listen times copied from `CStats`, the per-station histories (music, ident,
/// advert, DJ banter) and radio states (+0x100 + 0x2C * station) cleared to -1 / type 6, a random
/// requested station `GetRandomNumberInRange(1, 13)`, both settings blocks reset (the active one a copy
/// of the requested one), the mission-stat statics, and the user track play mode (+0x36C). Returns true.
#[hook(addr = 0x5B9390, conv = thiscall, name = "CAERadioTrackManager::Initialise", expect = "83 EC 0C 8B 44 24 10")]
pub fn radio_initialise(this: GPtr<u8>, handle: u32) -> u8 {
    wr::<u32>(this, 0x64, handle);
    wr::<u32>(this, 0x68, 7);
    wr::<u8>(this, 0, 0);
    wr::<u8>(this, 1, 0);
    let fav = stats_get_full_favorite_radio_station_list();
    for k in 0..14isize {
        wr::<u32>(this, 0x18 + 4 * k, rd::<u32>(fav, 4 * k));
    }
    let music = MUSIC_TRACK_INDEX_HISTORY.ptr().cast::<u8>();
    let ident = IDENT_INDEX_HISTORY.ptr().cast::<u8>();
    let advert = ADVERT_INDEX_HISTORY.ptr().cast::<u8>();
    let dj = DJ_BANTER_INDEX_HISTORY.ptr().cast::<u8>();
    for st in 0..14isize {
        for k in 0..5 {
            wr::<i32>(music, st * 20 + 4 * k, -1);
        }
        for k in 0..8 {
            wr::<i32>(ident, st * 32 + 4 * k, -1);
        }
        for k in 0..40 {
            wr::<i32>(advert, st * 0xA0 + 4 * k, -1);
        }
        for k in 0..15 {
            wr::<i32>(dj, st * 0x3C + 4 * k, -1);
        }
        let e = this.byte_add(0x110 + st * 0x2C);
        wr::<i32>(e, -4, -1);
        wr::<i32>(e, 0, -1);
        wr::<u8>(e, 0x17, 0xFF);
        wr::<u8>(e, 0x18, 0xFF);
        wr::<i32>(e, 4, -1);
        for k in 0..3 {
            wr::<u32>(e, 4 * k - 0x10, 0);
            wr::<i32>(e, 8 + 4 * k, -1);
            wr::<u8>(e, 0x14 + k, 6);
        }
        wr::<u8>(this, 8 + st, 0);
    }
    let r = ae_random_int(1, 13);
    wr::<u8>(this, 0xAD, r as u8);
    wr::<u8>(this, 0xAE, 0);
    wr::<u8>(this, 0xAC, 2);
    for k in 0..5isize {
        wr::<u8>(this, 0xBB + k, 0xFF);
        wr::<i32>(this, 0x88 + 4 * k, -1);
        wr::<u8>(this, 0xB4 + k, 6);
    }
    wr::<i32>(this, 0x9C, -1);
    wr::<i32>(this, 0xA0, -1);
    wr::<u8>(this, 0xB9, 6);
    wr::<u8>(this, 0xC0, 0xFF);
    wr::<u8>(this, 0xBA, 6);
    wr::<u8>(this, 0xC1, 0xFF);
    for off in [0xA4isize, 0xA8, 0x6C, 0x70, 0x50, 0x54, 0x58] {
        wr::<u32>(this, off, 0);
    }
    wr::<u8>(this, 0x16, 0xFF);
    wr::<u8>(this, 0x17, 0xFF);
    wr::<u8>(this, 2, 0);
    wr::<u32>(this, 0x60, 0);
    wr::<u32>(this, 0x5C, 0);
    wr::<u8>(this, 3, 0);
    wr::<u8>(this, 7, 1);
    wr::<u8>(this, 4, 1);
    wr::<i32>(this, 0x74, -1);
    wr::<i32>(this, 0x78, -1);
    wr::<i32>(this, 0x7C, -1);
    for k in 0..15isize {
        wr::<u32>(this, 0xC4 + 4 * k, rd::<u32>(this, 0x88 + 4 * k));
    }
    let stats = RADIO_STATS.ptr().cast::<u8>();
    wr::<u8>(stats, 0x1B, 3);
    wr::<u8>(stats, 0x1A, 0xFF);
    wr::<u8>(this, 5, 0);
    wr::<u8>(this, 6, 0);
    wr::<u32>(this, 0x80, 0);
    wr::<u32>(this, 0x84, 0);
    wr::<u8>(stats, 0x19, 0);
    wr::<u8>(stats, 2, 0xFF);
    wr::<u8>(stats, 1, 0xFF);
    wr::<u8>(stats, 0, 0xFF);
    for k in (3..=0x18isize).rev() {
        wr::<u8>(stats, k, 0);
    }
    wr::<u32>(this, 0x368, 0);
    let mode = get_user_track_play_mode_call(g(AE_USER_RADIO_TRACK_MANAGER));
    wr::<u8>(this, 0x36C, mode);
    1
}

/// `CTimer::m_snTimeInMillisecondsPauseMode` (0xB7CB7C)
const TIME_MS_PAUSE_MODE: usize = 0xB7CB7C;

/// `CAERadioTrackManager::StartRadio(eRadioID station, eBassSetting bass, float gain, uint8 b)`
/// (0x4EB3C0): stations >= 14 become 13 (also in the argument slot, low byte). While paused, the
/// enabled-in-pause flag (+3) is set and a playing radio already on that station only stamps its state's
/// pause time (+0x10C + 0x2C * station). Otherwise an active ambience track either keeps the radio off
/// (unpaused and overriding: front-end event 0x23) or is stopped; then the requested station / bass /
/// gain (+0xAD, +0xAE, +0xB0) are set, and the track queue reset (station 13), resumed
/// (`TrackRadioStation`) or rebuilt (`ChooseTracksForStation`, play time `GetRandomNumberInRange(0,
/// 300000)` at +0xA4); the radio becomes initialised, modes 0..2 become 3 and the state's pause time -1.
#[hook(addr = 0x4EB3C0, conv = thiscall, name = "CAERadioTrackManager::StartRadio-manual", expect = "53 8A 5C 24 08")]
pub fn start_radio_manual_port(this: GPtr<u8>, station: u32, bass: u32, gain_bits: u32, b: u32) {
    let mut slot = station;
    let mut st = station as u8 as i8;
    if st >= 14 {
        st = 13;
        slot = (slot & 0xFFFF_FF00) | 13;
    }
    let engine = g::<u8>(crate::common::AUDIO_ENGINE);
    let state = |s: i8| this.byte_add(0x10C + isize::from(s) * 0x2C);
    if paused() {
        wr::<u8>(this, 3, 1);
        if is_radio_on(this) != 0 && st == rd::<u8>(this, 0xE9) as i8 {
            wr::<u32>(state(st), 0, g::<u32>(TIME_MS_PAUSE_MODE).read());
            return;
        }
    }
    if st != 13 && engine_is_ambience_track_active(engine) != 0 {
        if !paused() && engine_does_ambience_track_override_radio(engine) != 0 {
            report_frontend_audio_event(engine, 0x23, 0.0, 1.0);
            return;
        }
        engine_stop_ambience_track(engine, 0);
    }
    wr::<u8>(this, 0xAD, st as u8);
    wr::<u8>(this, 0xAE, bass as u8);
    wr::<u32>(this, 0xB0, gain_bits);
    if st == 13 {
        for k in 0..5isize {
            wr::<i32>(this, 0x88 + 4 * k, -1);
            wr::<u8>(this, 0xB4 + k, 6);
            wr::<u8>(this, 0xBB + k, 0xFF);
        }
    } else if rd::<i32>(state(st), 4) < 0 || track_radio_station(this, slot, b) == 0 {
        choose_tracks_for_station(this, u32::from(rd::<u8>(this, 0xAD)));
        wr::<i32>(this, 0xA4, ae_random_int(0, 300_000));
    }
    wr::<u8>(this, 0, 1);
    if rd::<u32>(this, 0x68) <= 2 {
        wr::<u32>(this, 0x68, 3);
    }
    wr::<i32>(state(rd::<u8>(this, 0xAD) as i8), 0, -1);
}

/// `CReplay::Mode`
const REPLAY_MODE: usize = 0xA43088;
/// `CClock` hours / days / month (u8) and `CClock::daysInMonth` (u8[12])
const CLOCK_HOURS: usize = 0xB70153;
const CLOCK_DAYS: usize = 0xB70154;
const CLOCK_MONTH: usize = 0xB70155;
const DAYS_IN_MONTH: u32 = 0x8CCF24;

/// `CAERadioTrackManager::StartRadio(tVehicleAudioSettings* settings)` (0x4EB550: `nop; nop; jmp` to its first
/// compare at 0x407490): nothing during replay playback; radio type 3: `StartRadio(0, bass (+6),
/// gain (+8), 0)`; other non-zero types: nothing. Type 0: the vehicle's station (+0x1A), or, with
/// auto-retune on (+7) and a saved station (+0x74) that is valid (> 0, != 13, != the vehicle's) and
/// saved less than 60 s ago (+0x58) and at most 5 game hours ago (saved day +0x16 / hour +0x17 against
/// the clock, wrapping through the previous month's length), front-end event 0x22 and the saved station.
#[hook(addr = 0x4EB550, conv = thiscall, name = "CAERadioTrackManager::StartRadio-with-settings", expect = "90 90 E9 39 BF F1 FF")]
pub fn start_radio_with_settings(this: GPtr<u8>, settings: GPtr<u8>) {
    if g::<u8>(REPLAY_MODE).read() == 1 {
        return;
    }
    let ty = rd::<u8>(settings, 0x1B);
    if ty == 3 {
        start_radio_manual(
            this,
            0,
            u32::from(rd::<u8>(settings, 6)),
            rd::<u32>(settings, 8),
            0,
        );
        return;
    }
    if ty != 0 {
        return;
    }
    let station = if retune_to_saved(this, settings) {
        report_frontend_audio_event(g(crate::common::AUDIO_ENGINE), 0x22, 0.0, 1.0);
        u32::from(rd::<u8>(this, 0x74))
    } else {
        u32::from(rd::<u8>(settings, 0x1A))
    };
    start_radio_manual(
        this,
        station,
        u32::from(rd::<u8>(settings, 6)),
        rd::<u32>(settings, 8),
        0,
    );
}

/// The auto-retune condition of `StartRadio(tVehicleAudioSettings*)`.
fn retune_to_saved(this: GPtr<u8>, settings: GPtr<u8>) -> bool {
    if rd::<u8>(this, 7) == 0 {
        return false;
    }
    let saved = rd::<i32>(this, 0x74);
    if saved < 0 || saved == i32::from(rd::<u8>(settings, 0x1A) as i8) || saved == 13 || saved == 0 {
        return false;
    }
    let now = sa_core::timer::TIME_MS.get();
    if now > rd::<u32>(this, 0x58).wrapping_add(60_000) {
        return false;
    }
    let hour = rd::<u8>(this, 0x17) as i8;
    let day = rd::<u8>(this, 0x16) as i8;
    if hour < 0 || day < 0 {
        return false;
    }
    let mut d = i32::from(g::<u8>(CLOCK_DAYS).read()).wrapping_sub(i32::from(day));
    if d < 0 {
        let mut m = (g::<u8>(CLOCK_MONTH).read() as i8).wrapping_sub(1);
        if m < 0 {
            m = m.wrapping_add(12);
        }
        let len = g::<u8>(DAYS_IN_MONTH.wrapping_add(i32::from(m) as u32) as usize).read();
        d = d.wrapping_add(i32::from(len));
    }
    let h = d
        .wrapping_mul(24)
        .wrapping_sub(i32::from(hour))
        .wrapping_add(i32::from(g::<u8>(CLOCK_HOURS).read()));
    h <= 5
}

// ---------------------------------------------------------------------------------------------------
// CAEAmbienceTrackManager / CAECutsceneTrackManager / CAEUserRadioTrackManager
// ---------------------------------------------------------------------------------------------------

/// `CAEAmbienceTrackManager::IsAmbienceTrackActive()` (0x4D6CF0): status (+8) != 8.
#[hook(addr = 0x4D6CF0, conv = thiscall, name = "CAEAmbienceTrackManager::IsAmbienceTrackActive", expect = "83 79 08 08 0F 95 C0")]
pub fn is_ambience_track_active(this: GPtr<u8>) -> u8 {
    u8::from(rd::<u32>(this, 8) != 8)
}

/// `CAEAmbienceTrackManager::IsAmbienceRadioActive()` (0x4D6D40): the radio-active byte (+3) >= 0.
#[hook(addr = 0x4D6D40, conv = thiscall, name = "CAEAmbienceTrackManager::IsAmbienceRadioActive", expect = "8A 41 03 84 C0")]
pub fn is_ambience_radio_active(this: GPtr<u8>) -> u8 {
    u8::from(rd::<u8>(this, 3) as i8 >= 0)
}

/// `CAEAmbienceTrackManager::StopAmbienceTrack()` (0x4D6CC0): a status (+8) of 0..3 becomes 4; the
/// bytes +0, +1, +2 are cleared in every case.
#[hook(addr = 0x4D6CC0, conv = thiscall, name = "CAEAmbienceTrackManager::StopAmbienceTrack", expect = "8B 41 08 33 D2")]
pub fn stop_ambience_track(this: GPtr<u8>) {
    if rd::<u32>(this, 8) <= 3 {
        wr::<u32>(this, 8, 4);
    }
    wr::<u8>(this, 0, 0);
    wr::<u8>(this, 1, 0);
    wr::<u8>(this, 2, 0);
}

/// `CAEAmbienceTrackManager::CheckForPause()` (0x4D6DF0): the track's frequency factor is 0 while paused
/// or in the menu, else the stored factor (+0x1C); applied twice to channel 0 of slot (+4), as the bytes.
#[hook(addr = 0x4D6DF0, conv = thiscall, name = "CAEAmbienceTrackManager::CheckForPause", expect = "51 A0 49 CB B7 00")]
pub fn ambience_check_for_pause(this: GPtr<u8>) {
    let factor = if paused() || g::<u8>(MENU_ACTIVE).read() != 0 {
        0
    } else {
        rd::<u32>(this, 0x1C)
    };
    let hw = g::<u8>(AE_AUDIO_HARDWARE);
    for _ in 0..2 {
        hw_set_channel_frequency_scaling_factor(hw, u32::from(rd::<u16>(this, 4)), 0, factor);
    }
}

/// `CAEAmbienceTrackManager::StartTrackPlayback()` (0x4D6DA0): bass EQ off, channel volume (+0x18) on
/// slot (+4) channel 0, channel flags 0x21, then the hardware's `StartTrackPlayback()` (tail call, EAX
/// passed through).
#[hook(addr = 0x4D6DA0, conv = thiscall, name = "CAEAmbienceTrackManager::StartTrackPlayback", expect = "56 8B F1 B9 B8 F8 B5 00")]
pub fn ambience_start_track_playback(this: GPtr<u8>) -> u32 {
    let hw = g::<u8>(AE_AUDIO_HARDWARE);
    hw_disable_bass_eq(hw);
    hw_set_channel_volume(hw, u32::from(rd::<u16>(this, 4)), 0, rd::<u32>(this, 0x18), 0);
    hw_set_channel_flags(hw, u32::from(rd::<u16>(this, 4)), 0, 0x21);
    hw_start_track_playback(hw)
}

/// `CAEAmbienceTrackManager::Service(int32 playTime)` (0x4D76C0): the track and volume updated unless a
/// cutscene track plays, the game is paused or the menu is open; then the status (+8, jump table
/// 0x4D7854): 0 PlayTrack(+0x20, -1, offset, +0x28) -> 1 with offset 0 (restart flag +2), a random
/// `[0, 600000]` without a saved position (+0x10 < 0), else `max(saved + elapsed since +0x14,
/// saved + 7000)`; 1 -> 2 at play time -2; 2 StartTrackPlayback -> 3; 3 CheckForPause and the channel
/// volume (+0x18); 4 saves the position (or forgets it when the track (+0x20) is the stopped one
/// (+0xC)), volume -100, StopTrack -> 7; 5 StopSound(+4, 0) -> 6; 6 StopTrack -> 7; 7 -> 8 at -6,
/// StopTrack at -7 / -2.
#[hook(addr = 0x4D76C0, conv = thiscall, name = "CAEAmbienceTrackManager::Service", expect = "56 8B F1 57 B9 90 BC B6 00")]
pub fn ambience_service(this: GPtr<u8>, play_time: i32) {
    let engine = g::<u8>(crate::common::AUDIO_ENGINE);
    if engine_is_cutscene_track_active(engine) == 0 && !paused() && g::<u8>(MENU_ACTIVE).read() == 0 {
        ambience_update_track_and_volume(this);
    }
    let hw = g::<u8>(AE_AUDIO_HARDWARE);
    match rd::<u32>(this, 8) {
        0 => {
            let offset = if rd::<u8>(this, 2) != 0 {
                0
            } else {
                let saved = rd::<i32>(this, 0x10);
                if saved < 0 {
                    ae_random_int(0, 600_000)
                } else {
                    let since = rd::<u32>(this, 0x14);
                    let mut e = saved;
                    if since != 0 {
                        e = e.wrapping_add(sa_core::timer::TIME_MS.get().wrapping_sub(since) as i32);
                    }
                    let lim = saved.wrapping_add(7000);
                    if e > lim { e } else { lim }
                }
            };
            hw_play_track(
                hw,
                rd(this, 0x20),
                -1,
                offset as u32,
                u32::from(rd::<u8>(this, 0x28)),
                0,
                0,
            );
            wr::<u32>(this, 8, 1);
        }
        1 => {
            if play_time == -2 {
                wr::<u32>(this, 8, 2);
            }
        }
        2 => {
            ambience_start_track_playback_call(this);
            wr::<u32>(this, 8, 3);
        }
        3 => {
            ambience_check_for_pause_call(this);
            hw_set_channel_volume(hw, u32::from(rd::<u16>(this, 4)), 0, rd::<u32>(this, 0x18), 0);
        }
        4 => {
            if rd::<u32>(this, 0x20) == rd::<u32>(this, 0xC) {
                wr::<i32>(this, 0x10, -1);
                wr::<u32>(this, 0x14, 0);
            } else {
                wr::<i32>(this, 0x10, play_time);
                wr::<u32>(this, 0x14, sa_core::timer::TIME_MS.get());
            }
            wr::<u32>(this, 0x18, 0xC2C8_0000); // -100.0
            hw_stop_track(hw);
            wr::<u32>(this, 8, 7);
        }
        5 => {
            hw_stop_sound(hw, u32::from(rd::<u16>(this, 4)), 0);
            wr::<u32>(this, 8, 6);
        }
        6 => {
            hw_stop_track(hw);
            wr::<u32>(this, 8, 7);
        }
        7 => match play_time {
            -6 => wr::<u32>(this, 8, 8),
            -7 | -2 => hw_stop_track(hw),
            _ => {}
        },
        _ => {}
    }
}

/// `CAECutsceneTrackManager::IsCutsceneTrackActive()` (0x4DBEB0): status (+8) != 8.
#[hook(addr = 0x4DBEB0, conv = thiscall, name = "CAECutsceneTrackManager::IsCutsceneTrackActive", expect = "83 79 08 08 0F 95 C0")]
pub fn is_cutscene_track_active(this: GPtr<u8>) -> u8 {
    u8::from(rd::<u32>(this, 8) != 8)
}

/// `CAECutsceneTrackManager::GetCutsceneTrackStatus()` (0x4DBEC0): the jump table at 0x4DBEE0 maps the
/// status (+8) 0,1 -> 1, 2 -> 2, 3 -> 3, 4..7 -> 4; above 7 -> 0.
#[hook(addr = 0x4DBEC0, conv = thiscall, name = "CAECutsceneTrackManager::GetCutsceneTrackStatus", expect = "8B 41 08 83 F8 07")]
pub fn get_cutscene_track_status(this: GPtr<u8>) -> u8 {
    match rd::<u32>(this, 8) {
        0 | 1 => 1,
        2 => 2,
        3 => 3,
        4..=7 => 4,
        _ => 0,
    }
}

/// `AECutsceneTrackManager` +0x34 (0x8AE588): the previous play time, written absolutely
const CUTSCENE_PREV_PLAY_TIME: usize = 0x8AE588;

/// `CAECutsceneTrackManager::Service(int32 playTime)` (0x4DBFB0): stores the play time (+0x28; the old
/// one at 0x8AE588), the hardware's track length (+0x2C) and playing track (+0x24), then steps the
/// status (+8, jump table 0x4DC0B4): 0 PlayTrack(+0xC, -1, +0x14, +0x1C) -> 1; 1 -> 2 at play time -2;
/// 2 -> 3 with StartTrackPlayback once requested (+0); 3 CheckForPause, -> 8 at -6; 4/6 StopTrack -> 7;
/// 5 StopSound(+4, 0) -> 6; 7 -> 8 at -6, StopTrack at -2 / -7.
#[hook(addr = 0x4DBFB0, conv = thiscall, name = "CAECutsceneTrackManager::Service", expect = "56 8B F1 8B 46 28")]
pub fn cutscene_service(this: GPtr<u8>, play_time: i32) {
    let old = rd::<u32>(this, 0x28);
    wr::<i32>(this, 0x28, play_time);
    g::<u32>(CUTSCENE_PREV_PLAY_TIME).write(old);
    let hw = g::<u8>(AE_AUDIO_HARDWARE);
    wr::<u32>(this, 0x2C, hw_get_track_length_ms(hw));
    wr::<u32>(this, 0x24, hw_get_playing_track_id(hw));
    match rd::<u32>(this, 8) {
        0 => {
            hw_play_track(
                hw,
                rd(this, 0xC),
                -1,
                rd(this, 0x14),
                u32::from(rd::<u8>(this, 0x1C)),
                0,
                0,
            );
            wr::<u32>(this, 8, 1);
        }
        1 => {
            if rd::<i32>(this, 0x28) == -2 {
                wr::<u32>(this, 8, 2);
            }
        }
        2 => {
            if rd::<u8>(this, 0) != 0 {
                cutscene_start_track_playback(this);
                wr::<u32>(this, 8, 3);
                wr::<u8>(this, 0, 0);
            }
        }
        3 => {
            cutscene_check_for_pause(this);
            if rd::<i32>(this, 0x28) == -6 {
                wr::<u32>(this, 8, 8);
            }
        }
        4 | 6 => {
            hw_stop_track(hw);
            wr::<u32>(this, 8, 7);
        }
        5 => {
            hw_stop_sound(hw, u32::from(rd::<u16>(this, 4)), 0);
            wr::<u32>(this, 8, 6);
        }
        7 => match rd::<i32>(this, 0x28) {
            -6 => wr::<u32>(this, 8, 8),
            -2 | -7 => hw_stop_track(hw),
            _ => {}
        },
        _ => {}
    }
}

/// `CAEUserRadioTrackManager::GetUserTrackPlayMode()` (0x4F3330): `FrontEndMenuManager.m_nRadioMode`
/// (AL only).
#[hook(addr = 0x4F3330, conv = thiscall, name = "CAEUserRadioTrackManager::GetUserTrackPlayMode", expect = "A0 F8 67 BA 00")]
pub fn get_user_track_play_mode(_this: GPtr<u8>) -> u8 {
    g::<u8>(MENU_RADIO_MODE).read()
}

/// User track list file name (0x860A88), its open mode (0x859C6C) and "" (0x858B54)
const USER_TRACKS_FILE: usize = 0x860A88;
const USER_TRACKS_MODE: usize = 0x859C6C;
const EMPTY_DIR: usize = 0x858B54;

/// `CAEUserRadioTrackManager::ReadUserTracks()` (0x4F2FD0): loads the user track list from My
/// Documents into a `CMemoryMgr::Malloc` buffer (+0) and counts its 12-byte entries (+6, u16); false
/// when the file cannot be opened (handle <= 0) or is empty.
#[hook(addr = 0x4F2FD0, conv = thiscall, name = "CAEUserRadioTrackManager::ReadUserTracks", expect = "53 57 8B D9 E8 87 58 04 00")]
pub fn read_user_tracks_port(this: GPtr<u8>) -> u8 {
    file_set_dir_my_documents();
    let f = file_open(g(USER_TRACKS_FILE), g(USER_TRACKS_MODE));
    file_set_dir(g(EMPTY_DIR));
    if f <= 0 {
        return 0;
    }
    let size = file_get_total_size(f);
    if size == 0 {
        file_close(f);
        return 0;
    }
    wr::<u16>(this, 6, (size / 12) as u16);
    let buf = memory_malloc(size, 0);
    wr::<GPtr<u8>>(this, 0, buf);
    file_read(f, buf, size);
    file_close(f);
    1
}

/// `audioExtensionTypes[".mp3"].type` (0x8CBB3C)
const MP3_EXTENSION_TYPE: usize = 0x8CBB3C;

/// `CAEUserRadioTrackManager::Initialise()` (0x4F35B0): user tracks read (+4, +5), the decoder flags
/// (+8..+0xC) = Vorbis, WAV, WMA (`InitLibrary`), MF (`InitLibrary`); without MF, ".mp3" goes to the
/// WMA decoder (3). Returns true.
#[hook(addr = 0x4F35B0, conv = thiscall, name = "CAEUserRadioTrackManager::Initialise", expect = "56 8B F1 E8 18 FA FF FF")]
pub fn user_radio_initialise(this: GPtr<u8>) -> u8 {
    let r = read_user_tracks(this);
    wr::<u8>(this, 5, r);
    wr::<u8>(this, 4, r);
    wr::<u32>(this, 8, 0);
    wr::<u8>(this, 0xC, 0);
    wr::<u8>(this, 9, 1);
    wr::<u8>(this, 0xA, 1);
    wr::<u8>(this, 0xB, wma_init_library());
    let mf = mf_init_library();
    wr::<u8>(this, 0xC, mf);
    if mf == 0 {
        g::<u32>(MP3_EXTENSION_TYPE).write(3);
    }
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_SET_RADIO_AUTO_RETUNE_ON_OFF,
    HOOK_IS_RADIO_ON,
    HOOK_IS_VEHICLE_RADIO_ACTIVE,
    HOOK_GET_RADIO_STATION_NAME_KEY,
    HOOK_GET_RADIO_STATION_NAME,
    HOOK_ADD_MUSIC_TRACK_INDEX_TO_HISTORY,
    HOOK_START_TRACK_PLAYBACK,
    HOOK_IS_AMBIENCE_TRACK_ACTIVE,
    HOOK_IS_AMBIENCE_RADIO_ACTIVE,
    HOOK_STOP_AMBIENCE_TRACK,
    HOOK_IS_CUTSCENE_TRACK_ACTIVE,
    HOOK_GET_CUTSCENE_TRACK_STATUS,
    HOOK_GET_USER_TRACK_PLAY_MODE,
    HOOK_SET_BASS_ENHANCE_ON_OFF,
    HOOK_USER_RADIO_INITIALISE,
    HOOK_RADIO_CHECK_FOR_PAUSE,
    HOOK_CHOOSE_MUSIC_TRACK_INDEX,
    HOOK_AMBIENCE_CHECK_FOR_PAUSE,
    HOOK_CHOOSE_IDENT_INDEX,
    HOOK_CHOOSE_DJ_BANTER_INDEX_FROM_LIST,
    HOOK_AMBIENCE_START_TRACK_PLAYBACK,
    HOOK_START_RADIO_WITH_SETTINGS,
    HOOK_START_RADIO_MANUAL_PORT,
    HOOK_READ_USER_TRACKS_PORT,
    HOOK_CUTSCENE_SERVICE,
    HOOK_DISPLAY_RADIO_STATION_NAME,
    HOOK_AMBIENCE_SERVICE,
    HOOK_CHECK_FOR_STATION_RETUNE,
    HOOK_CHOOSE_TRACKS_FOR_STATION_PORT,
    HOOK_RADIO_INITIALISE,
];
