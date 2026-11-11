//! `CMenuManager` (PORT-HUD-2): `FrontEndMenuManager` (0xBA6748, 0x1B78) settings, screen switching,
//! input helpers, the message screens and the small `CPad` mouse / escape tests. Ported from the 1.0 US
//! bytes; `docs/re/specs/HUD/CMenuManager.deviations.md` and gta-reversed `Frontend/MenuManager*.cpp` as
//! reading aids. Rendering, RenderWare, audio, file IO and allocation stay original (`orig!`).
//!
//! `this` fields used here (offsets from the bytes): +0x21 / +0x24 / +0x3C brightness / +0x40 draw
//! distance / +0x44..+0x52 display and audio prefs, +0x54 current item, +0x5C, +0x84 language, +0x85
//! previous language, +0xC0 / +0xBC mouse position, +0xD0 control method, +0xE9, +0xEA pad, +0x15D
//! current screen, +0x15E previous screen, +0x160 mission pack id, +0x1AEA, +0x1AEC helper fade,
//! +0x1B18 helper text. `aScreens` (0x8CE008, 0xE2 per screen: +0 name[8], +8 parent screen, +9
//! parent item, items of 0x12 from +0xA: +0 action, +1 text[8], ..., +0xA target screen).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, wr, x, xi};

/// No globals of its own: `FrontEndMenuManager` fields are reached through `this` or by address
/// (radar.rs / radar_draw.rs read some of them by address too).
pub const GLOBALS: &[GlobalDesc] = &[];

/// `RsGlobal.maximumWidth` / `maximumHeight` (0xC17044 / 0xC17048)
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// `RsGlobal.ps` (0xC17054)
const RS_PS: usize = 0xC17054;
/// `CPad::NewKeyState` / `OldKeyState` key words (esc 0xB733A8 / 0xB73138, up 0xB733B6 / 0xB73146, down
/// 0xB733B8 / 0xB73148, left 0xB733BA / 0xB7314A, right 0xB733BC / 0xB7314C)
const KEY_NEW_ESC: usize = 0xB733A8;
const KEY_OLD_ESC: usize = 0xB73138;
/// `CPad::NewMouseControllerState` / `OldMouseControllerState` (rmb +2, wheel up +4, wheel down +5,
/// bmx1 +6, bmx2 +7; new at 0xB73417, old at 0xB7342B)
const MOUSE_NEW: usize = 0xB73417;
const MOUSE_OLD: usize = 0xB7342B;
/// `AudioEngine` (0xB6BC90)
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `TheText` (0xC1B340)
const THE_TEXT: usize = 0xC1B340;
/// `TheCamera` (0xB6F028)
const THE_CAMERA: usize = 0xB6F028;
/// `g_fx` (0xA9AE00)
const G_FX: usize = 0xA9AE00;
/// `ControlsManager` (0xB70198)
const CONTROLS_MANAGER: usize = 0xB70198;
/// `aScreens` (0x8CE008)
const SCREENS: usize = 0x8CE008;
/// The selected mission pack (0xB72910)
const MISSION_PACK: usize = 0xB72910;
/// "" (0x858B54)
const S_EMPTY: usize = 0x858B54;

/// 1/640 (0x859520), 1/448 (0x859524)
const K_INV_640: f32 = f32::from_bits(0x3ACC_CCCD);
const K_INV_448: f32 = f32::from_bits(0x3B12_4925);
/// 0.5 (0x858B8C), 0.0 (0x858B50)
const K_HALF: f32 = f32::from_bits(0x3F00_0000);
const K_ZERO: f32 = 0.0;

orig! {
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(n: u32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::GetAnaloguePadUp()` (0x540950, thiscall)
    pub fn analogue_pad_up(pad: GPtr<u8>) -> u8 = thiscall 0x540950 => unported;
    /// `CPad::GetAnaloguePadDown()` (0x540980, thiscall)
    pub fn analogue_pad_down(pad: GPtr<u8>) -> u8 = thiscall 0x540980 => unported;
    /// `CPad::GetAnaloguePadLeft()` (0x5409B0, thiscall)
    pub fn analogue_pad_left(pad: GPtr<u8>) -> u8 = thiscall 0x5409B0 => unported;
    /// `CPad::GetAnaloguePadRight()` (0x5409E0, thiscall)
    pub fn analogue_pad_right(pad: GPtr<u8>) -> u8 = thiscall 0x5409E0 => unported;
    /// `CAudioEngine::StopRadio(tVehicleAudioSettings*, bool)` (0x506F70, thiscall)
    pub fn audio_stop_radio(this: GPtr<u8>, a: u32, b: u32) = thiscall 0x506F70 => unported;
    /// `CAudioEngine::ReportFrontendAudioEvent(int32, float, float)` (0x506EA0, thiscall)
    pub fn audio_report_frontend_event(this: GPtr<u8>, id: u32, volume: u32, speed: u32) = thiscall 0x506EA0 => unported;
    /// `CAudioEngine::StartRadio(int8, int8)` (0x507DC0, thiscall)
    pub fn audio_start_radio(this: GPtr<u8>, station: u32, b: u32) = thiscall 0x507DC0 => unported;
    /// `CAudioEngine::IsCutsceneTrackActive()` (0x507150, thiscall)
    pub fn audio_is_cutscene_track_active(this: GPtr<u8>) -> u8 = thiscall 0x507150 => unported;
    /// `CAudioEngine::RetuneRadio(int8)` (0x507E10, thiscall)
    pub fn audio_retune_radio(this: GPtr<u8>, station: u32) = thiscall 0x507E10 => unported;
    /// `CAudioEngine::SetMusicMasterVolume(int8)` (0x506DE0, thiscall)
    pub fn audio_set_music_volume(this: GPtr<u8>, v: u32) = thiscall 0x506DE0 => unported;
    /// `CAudioEngine::SetEffectsMasterVolume(int8)` (0x506E10, thiscall)
    pub fn audio_set_effects_volume(this: GPtr<u8>, v: u32) = thiscall 0x506E10 => unported;
    /// `CAudioEngine::SetBassEnhanceOnOff(bool)` (0x506F90, thiscall)
    pub fn audio_set_bass_enhance(this: GPtr<u8>, v: u32) = thiscall 0x506F90 => unported;
    /// `CAudioEngine::SetRadioAutoRetuneOnOff(bool)` (0x506F80, thiscall)
    pub fn audio_set_radio_auto_retune(this: GPtr<u8>, v: u32) = thiscall 0x506F80 => unported;
    /// `CAudioEngine::Reset()` (0x507A90, thiscall)
    pub fn audio_reset(this: GPtr<u8>) = thiscall 0x507A90 => unported;
    /// `Fx_c::SetFxQuality(FxQuality_e)` (0x49EA40, thiscall)
    pub fn fx_set_quality(this: GPtr<u8>, q: u32) = thiscall 0x49EA40 => unported;
    /// `Fx_c::GetFxQuality()` (0x49EA50, thiscall)
    pub fn fx_get_quality(this: GPtr<u8>) -> u32 = thiscall 0x49EA50 => unported;
    /// `CGamma::SetGamma(float, bool)` (0x747200, thiscall)
    pub fn gamma_set(this: GPtr<u8>, v: u32, b: u32) = thiscall 0x747200 => unported;
    /// `RwRenderStateSet(state, value)` (`RwEngineInstance` +0x20)
    pub fn rw_render_state_set(state: i32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CMenuManager::SaveSettings()` (0x57C660, thiscall)
    pub fn save_settings_call(this: GPtr<u8>) = thiscall 0x57C660 => save_settings;
    /// `CMenuManager::ResetHelperText()` (0x57CD30, thiscall)
    pub fn reset_helper_text_call(this: GPtr<u8>) = thiscall 0x57CD30 => reset_helper_text;
    /// `CMenuManager::SwitchToNewScreen(int8)` (0x573680, thiscall)
    pub fn switch_to_new_screen_call(this: GPtr<u8>, screen: u32) = thiscall 0x573680 => switch_to_new_screen;
    /// `CMenuManager::DoSettingsBeforeStartingAGame()` (0x573330, thiscall)
    pub fn do_settings_before_starting_a_game_call(this: GPtr<u8>) = thiscall 0x573330 => do_settings_before_starting_a_game;
    /// CRT `strncpy` (0x821F40)
    pub fn crt_strncpy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727B60)
    pub fn sprite_draw_rect(rect: GPtr<u8>, colour: GPtr<u8>) = cdecl 0x727B60 => unported;
    /// `CRGBA::CRGBA(r, g, b, a)` (0x7170C0, thiscall)
    pub fn rgba_ctor(this: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CFont::SetWrapx(float)` (0x7194D0)
    pub fn font_set_wrapx(v: u32) = cdecl 0x7194D0 => crate::font::set_wrapx;
    /// `CFont::SetColor(CRGBA)` (0x719430)
    pub fn font_set_color(c: u32) = cdecl 0x719430 => crate::font::set_color;
    /// `CFont::SetDropColor(CRGBA)` (0x719510)
    pub fn font_set_drop_color(c: u32) = cdecl 0x719510 => crate::font::set_drop_color;
    /// `CFont::SetEdge(int8)` (0x719590)
    pub fn font_set_edge(v: u32) = cdecl 0x719590 => crate::font::set_edge;
    /// `CFont::SetOrientation(eFontAlignment)` (0x719610)
    pub fn font_set_orientation(v: u32) = cdecl 0x719610 => crate::font::set_orientation;
    /// `CFont::SetFontStyle(eFontStyle)` (0x719490)
    pub fn font_set_font_style(v: u32) = cdecl 0x719490 => crate::font::set_font_style;
    /// `CFont::SetScaleForCurrentLanguage(float, float)` (0x7193A0)
    pub fn font_set_scale_for_current_language(w: f32, h: u32) = cdecl 0x7193A0 => crate::font::set_scale_for_current_language;
    /// `CFont::PrintString(float, float, const GxtChar*)` (0x71A700)
    pub fn font_print_string(px: f32, py: f32, text: GPtr<u8>) = cdecl 0x71A700 => crate::font_print::print_string;
    /// `CText::Get(const char*)` (0x6A0050, thiscall)
    pub fn text_get(this: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => crate::text::text_get;
    /// `CText::GetNameOfLoadedMissionText(char*)` (0x69FBD0, thiscall)
    pub fn text_get_name_of_loaded_mission_text(this: GPtr<u8>, out: GPtr<u8>) = thiscall 0x69FBD0 => crate::text::get_name_of_loaded_mission_text;
    /// `CText::Load(bool)` (0x6A01A0, thiscall)
    pub fn text_load(this: GPtr<u8>, keep: u32) = thiscall 0x6A01A0 => crate::text_load::text_load;
    /// `CText::LoadMissionText(const char*)` (0x69FBF0, thiscall)
    pub fn text_load_mission_text(this: GPtr<u8>, name: GPtr<u8>) = thiscall 0x69FBF0 => crate::text_load::load_mission_text;
    /// `CTimer::Stop()` (0x561AA0)
    pub fn timer_stop() = cdecl 0x561AA0 => sa_core::timer::stop;
    /// `CTimer::Update()` (0x561B10)
    pub fn timer_update() = cdecl 0x561B10 => unported;
    /// `CLocalisation::SetNormalGame()` (0x56D1A0)
    pub fn localisation_set_normal_game() = cdecl 0x56D1A0 => crate::hud::localisation_set_normal_game;
    /// `CLocalisation::SetFrenchGame()` (0x56D1C0)
    pub fn localisation_set_french_game() = cdecl 0x56D1C0 => crate::hud::localisation_set_french_game;
    /// `CControllerConfigManager::ReinitControls()` (0x531F20, thiscall)
    pub fn ccm_reinit_controls(this: GPtr<u8>) = thiscall 0x531F20 => unported;
    /// `CControllerConfigManager::SaveSettings(FILE*)` (0x52D200, thiscall)
    pub fn ccm_save_settings(this: GPtr<u8>, f: GPtr<u8>) = thiscall 0x52D200 => unported;
    /// `CAEAudioUtility::GetRandomNumberInRange(int32, int32)` (0x4D9C10)
    pub fn audio_random_in_range(lo: i32, hi: i32) -> i32 = cdecl 0x4D9C10 => unported;
    /// `CRadar::ClearBlip(int32)` (0x587CE0)
    pub fn radar_clear_blip(h: i32) = cdecl 0x587CE0 => crate::radar::clear_blip;
    /// `CStats::UpdateRespectStat(uint8)` (0x55BC50)
    pub fn stats_update_respect_stat(v: u32) = cdecl 0x55BC50 => unported;
    /// `CHud::SetZoneName(const char*, bool)` (0x588BB0)
    pub fn hud_set_zone_name(name: u32, force: u8) = cdecl 0x588BB0 => crate::hud::set_zone_name;
    /// `CCamera::SetFadeColour(uint8, uint8, uint8)` (0x50BF00, thiscall)
    pub fn camera_set_fade_colour(this: GPtr<u8>, r: u32, g: u32, b: u32) = thiscall 0x50BF00 => unported;
    /// `CCamera::Fade(float, int16)` (0x50AC20, thiscall)
    pub fn camera_fade(this: GPtr<u8>, t: u32, dir: u32) = thiscall 0x50AC20 => unported;
    /// `CCamera::ProcessFade()` (0x50B5D0, thiscall)
    pub fn camera_process_fade(this: GPtr<u8>) = thiscall 0x50B5D0 => unported;
    /// `CRadar::RequestMapSection(int32, int32)` (0x584B50)
    pub fn radar_request_map_section(x: i32, y: i32) = cdecl 0x584B50 => crate::radar_draw::request_map_section;
    /// `CRadar::RemoveMapSection(int32, int32)` (0x584BB0)
    pub fn radar_remove_map_section(x: i32, y: i32) = cdecl 0x584BB0 => crate::radar_draw::remove_map_section;
    /// `CStreaming::LoadRequestedModels()` (0x40E3A0)
    pub fn streaming_load_requested_models() = cdecl 0x40E3A0 => sa_stream::streaming::load_requested_models;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    pub fn streaming_load_all_requested_models(only_priority: u8) = cdecl 0x40EA10 => sa_stream::streaming::load_all_requested_models;
    /// `CFileMgr::SetDir(const char*)` (0x5387D0)
    pub fn file_set_dir(dir: GPtr<u8>) -> i32 = cdecl 0x5387D0 => sa_core::filemgr::set_dir;
    /// `CFileMgr::SetDirMyDocuments()` (0x538860)
    pub fn file_set_dir_my_documents() -> i32 = cdecl 0x538860 => sa_core::filemgr::set_dir_my_documents;
    /// `CFileMgr::OpenFile(const char*, const char*)` (0x538900)
    pub fn file_open(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => sa_core::filemgr::open_file;
    /// `CFileMgr::Write(FILE*, const void*, int32)` (0x538970)
    pub fn file_write(f: GPtr<u8>, buf: GPtr<u8>, size: u32) -> u32 = cdecl 0x538970 => sa_core::filemgr::write;
    /// `CFileMgr::CloseFile(FILE*)` (0x5389D0)
    pub fn file_close(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => sa_core::filemgr::close_file;
    /// `RwEngineGetCurrentSubSystem()` (0x7F2C60)
    pub fn rw_engine_get_current_subsystem() -> i32 = cdecl 0x7F2C60 => unported;
    /// `ClientToScreen(HWND, POINT*)` (import slot 0x8582FC)
    pub fn client_to_screen(hwnd: u32, pt: GPtr<u8>) -> i32 = stdcall cell 0x8582FC => unported;
    /// `SetCursorPos(int, int)` (import slot 0x858300)
    pub fn set_cursor_pos(px: i32, py: i32) -> i32 = stdcall cell 0x858300 => unported;
}

fn local<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(core::ptr::from_mut(v)).cast()
}

fn b8(p: GPtr<u8>, off: isize) -> u8 {
    rd(p, off)
}

fn screen(i: i32) -> GPtr<u8> {
    g(SCREENS.wrapping_add(i.wrapping_mul(0xE2) as u32 as usize))
}

/// `just pressed`: the new state set and the old one clear (`cmp word new, 0; je; cmp word old, 0`).
fn key_just_pressed(new: usize, old: usize) -> bool {
    g::<u16>(new).read() != 0 && g::<u16>(old).read() == 0
}

fn mouse_just_pressed(off: usize) -> u8 {
    u8::from(g::<u8>(MOUSE_NEW + off).read() != 0 && g::<u8>(MOUSE_OLD + off).read() == 0)
}

// ---------------------------------------------------------------------------------------------------
// CPad helpers
// ---------------------------------------------------------------------------------------------------

/// `CPad::IsEscJustPressed()` (0x572DB0): AL.
#[hook(addr = 0x572DB0, conv = cdecl, name = "CPad::IsEscJustPressed", expect = "66 83 3D A8 33 B7 00 00")]
pub fn is_esc_just_pressed() -> u8 {
    u8::from(key_just_pressed(KEY_NEW_ESC, KEY_OLD_ESC))
}

/// `CPad::IsMouseRButtonPressed()` (0x572E70): right button just pressed (AL).
#[hook(addr = 0x572E70, conv = cdecl, name = "CPad::IsMouseRButtonPressed", expect = "A0 19 34 B7 00 84 C0")]
pub fn is_mouse_rbutton_pressed() -> u8 {
    mouse_just_pressed(2)
}

/// `CPad::IsMouseWheelUpPressed()` (0x57C400): AL.
#[hook(addr = 0x57C400, conv = cdecl, name = "CPad::IsMouseWheelUpPressed", expect = "A0 1B 34 B7 00 84 C0")]
pub fn is_mouse_wheel_up_pressed() -> u8 {
    mouse_just_pressed(4)
}

/// `CPad::IsMouseWheelDownPressed()` (0x57C420): AL.
#[hook(addr = 0x57C420, conv = cdecl, name = "CPad::IsMouseWheelDownPressed", expect = "A0 1C 34 B7 00 84 C0")]
pub fn is_mouse_wheel_down_pressed() -> u8 {
    mouse_just_pressed(5)
}

/// `CPad::IsMouseBmx1Pressed()` (0x57C440): AL.
#[hook(addr = 0x57C440, conv = cdecl, name = "CPad::IsMouseBmx1Pressed", expect = "A0 1D 34 B7 00 84 C0")]
pub fn is_mouse_bmx1_pressed() -> u8 {
    mouse_just_pressed(6)
}

/// `CPad::IsMouseBmx2Pressed()` (0x57C460): AL.
#[hook(addr = 0x57C460, conv = cdecl, name = "CPad::IsMouseBmx2Pressed", expect = "A0 1E 34 B7 00 84 C0")]
pub fn is_mouse_bmx2_pressed() -> u8 {
    mouse_just_pressed(7)
}

// ---------------------------------------------------------------------------------------------------
// Small CMenuManager helpers
// ---------------------------------------------------------------------------------------------------

/// `CMenuManager::StretchX(float)` (0x5733E0, thiscall, `this` unused; entry = 5 `nop` + `jmp` into the
/// `.text` chunk 0x4016C4): `x` when the width is 640, else `maxWidth * x * (1/640)` (ST0).
///
/// `preserve = "edx"` (PORT-HUD-3): callers keep EDX across the call (0x577FA6 / 0x578031 read it at
/// 0x57804B); the original never writes EDX. L1 spec `HudMenuStretchX*` compares EDX.
#[hook(addr = 0x5733E0, conv = thiscall, name = "CMenuManager::StretchX", expect = "90 90 90 90 90", preserve = "edx")]
pub fn stretch_x(_this: GPtr<u8>, v: f32) -> X87 {
    stretch_x_with(g::<i32>(RS_MAX_WIDTH).read(), v)
}

/// [`stretch_x`] with `RsGlobal.maximumWidth` as a parameter (L1 spec).
pub fn stretch_x_with(w: i32, v: f32) -> X87 {
    if w == 0x280 {
        x(v)
    } else {
        xi(w).fmul_m(v).fmul_m(K_INV_640)
    }
}

/// `CMenuManager::StretchY(float)` (0x573410, thiscall, `this` unused): `y` when the height is 448, else
/// `maxHeight * y * (1/448)` (ST0).
///
/// `preserve = "edx"` (PORT-HUD-3): callers keep EDX across the call (0x578106 / 0x578199 at 0x5781B3,
/// 0x57B8E9 at 0x57B906); the original never writes EDX. L1 spec `HudMenuStretchY*` compares EDX.
#[hook(addr = 0x573410, conv = thiscall, name = "CMenuManager::StretchY", expect = "81 3D 48 70 C1 00 C0 01 00 00", preserve = "edx")]
pub fn stretch_y(_this: GPtr<u8>, v: f32) -> X87 {
    stretch_y_with(g::<i32>(RS_MAX_HEIGHT).read(), v)
}

/// [`stretch_y`] with `RsGlobal.maximumHeight` as a parameter (L1 spec).
pub fn stretch_y_with(h: i32, v: f32) -> X87 {
    if h == 0x1C0 {
        x(v)
    } else {
        xi(h).fmul_m(v).fmul_m(K_INV_448)
    }
}

/// The four `CheckFrontEnd*Input` shapes: `GetPad(+0xEA)` (result unused), the key just pressed, the
/// analogue test, or the pad's d-pad word (`new != 0 && old == 0`). AL.
fn check_front_end(this: GPtr<u8>, key_new: usize, analogue: fn(GPtr<u8>) -> u8, dpad: isize) -> u8 {
    let pad_n = u32::from(b8(this, 0xEA));
    get_pad(pad_n);
    if key_just_pressed(key_new, key_new - 0x270) {
        return 1;
    }
    if analogue(get_pad(u32::from(b8(this, 0xEA)))) != 0 {
        return 1;
    }
    let pad = get_pad(u32::from(b8(this, 0xEA)));
    u8::from(rd::<u16>(pad, dpad) != 0 && rd::<u16>(pad, dpad + 0x30) == 0)
}

/// `CMenuManager::CheckFrontEndUpInput()` (0x573840, thiscall): up key, analogue up or d-pad up (AL).
#[hook(addr = 0x573840, conv = thiscall, name = "CMenuManager::CheckFrontEndUpInput", expect = "56 8B F1 0F B6 86 EA 00 00 00")]
pub fn check_front_end_up_input(this: GPtr<u8>) -> u8 {
    check_front_end(this, 0xB733B6, analogue_pad_up, 0x10)
}

/// `CMenuManager::CheckFrontEndDownInput()` (0x5738B0, thiscall)
#[hook(addr = 0x5738B0, conv = thiscall, name = "CMenuManager::CheckFrontEndDownInput", expect = "56 8B F1 0F B6 86 EA 00 00 00")]
pub fn check_front_end_down_input(this: GPtr<u8>) -> u8 {
    check_front_end(this, 0xB733B8, analogue_pad_down, 0x12)
}

/// `CMenuManager::CheckFrontEndLeftInput()` (0x573920, thiscall)
#[hook(addr = 0x573920, conv = thiscall, name = "CMenuManager::CheckFrontEndLeftInput", expect = "56 8B F1 0F B6 86 EA 00 00 00")]
pub fn check_front_end_left_input(this: GPtr<u8>) -> u8 {
    check_front_end(this, 0xB733BA, analogue_pad_left, 0x14)
}

/// `CMenuManager::CheckFrontEndRightInput()` (0x573990, thiscall)
#[hook(addr = 0x573990, conv = thiscall, name = "CMenuManager::CheckFrontEndRightInput", expect = "56 8B F1 0F B6 86 EA 00 00 00")]
pub fn check_front_end_right_input(this: GPtr<u8>) -> u8 {
    check_front_end(this, 0xB733BC, analogue_pad_right, 0x16)
}

/// `CMenuManager::ScrollRadioStations(int8)` (0x573A00, thiscall): with a station (+0x52) and no
/// cutscene track: `station += dir` (byte), `<= 0` (signed) -> 13, `>= 14` -> 1, `RetuneRadio`,
/// `SaveSettings`.
#[hook(addr = 0x573A00, conv = thiscall, name = "CMenuManager::ScrollRadioStations", expect = "56 8B F1 8A 46 52 84 C0")]
pub fn scroll_radio_stations(this: GPtr<u8>, dir: u32) {
    if b8(this, 0x52) == 0 || audio_is_cutscene_track_active(g(AUDIO_ENGINE)) != 0 {
        return;
    }
    let s = b8(this, 0x52).wrapping_add(dir as u8);
    wr(this, 0x52, s);
    if (s as i8) <= 0 {
        wr::<u8>(this, 0x52, 0xD);
    }
    if (b8(this, 0x52) as i8) >= 0xE {
        wr::<u8>(this, 0x52, 1);
    }
    audio_retune_radio(g(AUDIO_ENGINE), u32::from(b8(this, 0x52)));
    save_settings_call(this);
}

/// `CMenuManager::SetFrontEndRenderStates()` (0x573A60): nine `RwRenderStateSet` calls.
#[hook(addr = 0x573A60, conv = thiscall, name = "CMenuManager::SetFrontEndRenderStates", expect = "A1 24 7B C9 00 6A 00")]
pub fn set_front_end_render_states(_this: GPtr<u8>) {
    for (s, v) in [(6, 0), (8, 0), (0xE, 0), (0xA, 5), (0xB, 6), (0xC, 1), (9, 2), (2, 3), (1, 0)] {
        rw_render_state_set(s, v);
    }
}

/// `CMenuManager::HasLanguageChanged()` (0x573CD0, thiscall): +0x85 takes +0x84 when they differ (AL =
/// 1), else AL = 0.
#[hook(addr = 0x573CD0, conv = thiscall, name = "CMenuManager::HasLanguageChanged", expect = "8A 81 84 00 00 00")]
pub fn has_language_changed(this: GPtr<u8>) -> u8 {
    let l = b8(this, 0x84);
    if b8(this, 0x85) == l {
        return 0;
    }
    wr(this, 0x85, l);
    1
}

/// `CMenuManager::GetNumberOfMenuOptions()` (0x573E70, thiscall; entry `nop; jmp` into the `.text` chunk
/// 0x4022F1): 2 on screens 5 and 2; otherwise the count of items of the current screen before the first
/// empty action (actions 1 and 0x14 not counted), cached per screen in 0x8CDFF0 / 0xBA8269 (AL).
#[hook(addr = 0x573E70, conv = thiscall, name = "CMenuManager::GetNumberOfMenuOptions", expect = "90 E9 7B E4 E8 FF")]
pub fn get_number_of_menu_options(this: GPtr<u8>) -> u8 {
    let cur = b8(this, 0x15D);
    if cur == 5 || cur == 2 {
        return 2;
    }
    let s = i32::from(cur as i8);
    if g::<i32>(0x8CDFF0).read() == s {
        return g::<u8>(0xBA8269).read();
    }
    let mut n = 0u8;
    g::<u8>(0xBA8269).write(0);
    let sc = screen(s);
    let mut i = 0u8;
    loop {
        let a = b8(sc, 0xA + isize::from(i) * 0x12);
        if a == 0 {
            break;
        }
        if a != 1 && a != 0x14 {
            n = n.wrapping_add(1);
        }
        i += 1;
        if i >= 0xC {
            break;
        }
    }
    g::<i32>(0x8CDFF0).write(s);
    g::<u8>(0xBA8269).write(n);
    n
}

/// `CMenuManager::CheckHover(int32 left, int32 right, int32 top, int32 bottom)` (0x57C4F0, thiscall;
/// entry `nop; jmp` into the `.text` chunk 0x40178C): mouse x (+0xBC) strictly inside (left, right) and
/// y (+0xC0) strictly inside (top, bottom) (AL).
#[hook(addr = 0x57C4F0, conv = thiscall, name = "CMenuManager::CheckHover", expect = "90 E9 96 52 E8 FF")]
pub fn check_hover(this: GPtr<u8>, left: i32, right: i32, top: i32, bottom: i32) -> u8 {
    let mx: i32 = rd(this, 0xBC);
    if mx <= left || mx >= right {
        return 0;
    }
    let my: i32 = rd(this, 0xC0);
    u8::from(my > top && my < bottom)
}

/// `CMenuManager::SetHelperText(int32)` (0x57CD10, thiscall; the store sits in the `.text` chunk
/// 0x40A0E2): +0x1B18 = text, +0x1AEC = 300.
#[hook(addr = 0x57CD10, conv = thiscall, name = "CMenuManager::SetHelperText", expect = "8B 44 24 04 90 E9 C8 D3 E8 FF")]
pub fn set_helper_text(this: GPtr<u8>, text: u32) {
    wr(this, 0x1B18, text);
    wr::<u32>(this, 0x1AEC, 300);
}

/// `CMenuManager::ResetHelperText()` (0x57CD30, thiscall): +0x1B18 = 0, +0x1AEC = 300.
#[hook(addr = 0x57CD30, conv = thiscall, name = "CMenuManager::ResetHelperText", expect = "C7 81 18 1B 00 00 00 00 00 00")]
pub fn reset_helper_text(this: GPtr<u8>) {
    wr::<u32>(this, 0x1B18, 0);
    wr::<u32>(this, 0x1AEC, 300);
}

/// `CMenuManager::SaveLoadFileError_SetUpErrorScreen()` (0x57C490): by `PcSaveHelper.error`
/// (0xC17034) 1..10 (byte table 0x57C4DC): screens 0x1D, 0x1E, 0x1F, 0x20 through
/// `SwitchToNewScreen` (ECX passed on as the caller's).
#[hook(addr = 0x57C490, conv = thiscall, name = "CMenuManager::SaveLoadFileError_SetUpErrorScreen", expect = "A1 34 70 C1 00 48")]
pub fn save_load_file_error_set_up_error_screen(this: GPtr<u8>) {
    const TABLE: [u8; 10] = [0, 0, 1, 1, 2, 2, 3, 3, 4, 4];
    let e = g::<u32>(0xC17034).read().wrapping_sub(1);
    if e > 9 {
        return;
    }
    match TABLE[e as usize] {
        0 => switch_to_new_screen_call(this, 0x1D),
        1 => switch_to_new_screen_call(this, 0x1E),
        2 => switch_to_new_screen_call(this, 0x1F),
        3 => switch_to_new_screen_call(this, 0x20),
        _ => {}
    }
}

/// `CMenuManager::CentreMousePointer()` (0x57C520): nothing when `W * 0.5` or `H * 0.5` is 0 (x87
/// compare); else the cursor to `ClientToScreen({W/2, H/2})` and `RsGlobal.ps->lastMousePos` =
/// `(float)(W/2), (float)(H/2)` (signed halving).
#[hook(addr = 0x57C520, conv = thiscall, name = "CMenuManager::CentreMousePointer", expect = "83 EC 10 DB 05 44 70 C1 00")]
pub fn centre_mouse_pointer(_this: GPtr<u8>) {
    let w = g::<i32>(RS_MAX_WIDTH).read();
    let hx = xi(w).fmul_m(K_HALF);
    let hy = xi(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(K_HALF).to_f32();
    if !hx.fcom(x(K_ZERO)).test_parity(0x44) || !x(hy).fcom(x(K_ZERO)).test_parity(0x44) {
        return;
    }
    let ps: GPtr<u8> = g::<GPtr<u8>>(RS_PS).read();
    let mut pt = [w / 2, g::<i32>(RS_MAX_HEIGHT).read() / 2];
    let pp = local(&mut pt);
    client_to_screen(rd::<u32>(ps, 0), pp);
    set_cursor_pos(rd::<i32>(pp, 0), rd::<i32>(pp, 4));
    let px = g::<i32>(RS_MAX_WIDTH).read() / 2;
    let py = g::<i32>(RS_MAX_HEIGHT).read() / 2;
    wr(g::<GPtr<u8>>(RS_PS).read(), 0xC, xi(px).to_f32());
    wr(g::<GPtr<u8>>(RS_PS).read(), 0x10, xi(py).to_f32());
}

// ---------------------------------------------------------------------------------------------------
// Screens and settings
// ---------------------------------------------------------------------------------------------------

/// `CMenuManager::SwitchToNewScreen(int8 screen)` (0x573680, thiscall): previous = current, +0x1AEA
/// cleared, helper text reset; leaving screen 4 copies +0xC8 -> +0xCC and +0xD4 -> +0xD8. A screen
/// other than 0 / -2 is entered at item 0. -2 goes back to the parent screen and item (with +0xE9 set,
/// screens 1 / 0x21 / 0x23 return to items 0 / 1 / 2); 0 follows the current item's target screen
/// (item 0). Screen 0x10 selects item 9. Leaving screen 3 stops the radio and plays frontend event
/// 0x23; entering 3 starts station +0x52. With +0xE9 set: screen 6 / 8 start a game (mission pack 0 /
/// +0x160) and set +0x32, screen 11 becomes 13.
#[hook(addr = 0x573680, conv = thiscall, name = "CMenuManager::SwitchToNewScreen", expect = "56 8B F1 8A 86 5D 01 00 00")]
pub fn switch_to_new_screen(this: GPtr<u8>, screen_arg: u32) {
    wr(this, 0x15E, b8(this, 0x15D));
    wr::<u8>(this, 0x1AEA, 0);
    reset_helper_text_call(this);
    if b8(this, 0x15D) == 4 {
        wr(this, 0xCC, rd::<u32>(this, 0xC8));
        wr(this, 0xD8, rd::<u32>(this, 0xD4));
    }
    let s = screen_arg as u8;
    if s != 0 && s != 0xFE {
        wr::<u32>(this, 0x54, 0);
        wr(this, 0x15D, s);
    }
    if s == 0xFE {
        let cur = i32::from(b8(this, 0x15D) as i8);
        let sc = screen(cur);
        let item: i32 = if b8(this, 0xE9) == 0 {
            i32::from(b8(sc, 9) as i8)
        } else {
            match cur {
                1 => 0,
                0x21 => 1,
                0x23 => 2,
                _ => i32::from(b8(sc, 9) as i8),
            }
        };
        wr(this, 0x54, item);
        wr(this, 0x15D, b8(sc, 8));
    } else if s == 0 {
        let cur = i32::from(b8(this, 0x15D) as i8);
        let item: i32 = rd(this, 0x54);
        let t = rd::<u8>(screen(cur), 0x14 + (item.wrapping_mul(0x12)) as isize);
        wr::<u32>(this, 0x54, 0);
        wr(this, 0x15D, t);
    }
    if b8(this, 0x15D) == 0x10 {
        wr::<u32>(this, 0x54, 9);
    }
    if b8(this, 0x15E) == 3 {
        audio_stop_radio(g(AUDIO_ENGINE), 0, 0);
        audio_report_frontend_event(g(AUDIO_ENGINE), 0x23, 0, 0x3F80_0000);
    }
    if b8(this, 0x15D) == 3 {
        audio_start_radio(g(AUDIO_ENGINE), u32::from(b8(this, 0x52)), 0);
    }
    if b8(this, 0xE9) != 0 {
        match b8(this, 0x15D) as i8 {
            6 => {
                g::<u8>(MISSION_PACK).write(0);
                do_settings_before_starting_a_game_call(this);
                wr::<u8>(this, 0x32, 1);
            }
            8 => {
                g::<u8>(MISSION_PACK).write(b8(this, 0x160));
                do_settings_before_starting_a_game_call(this);
                wr::<u8>(this, 0x32, 1);
            }
            11 => wr::<u8>(this, 0x15D, 0xD),
            _ => {}
        }
    }
}

/// `CMenuManager::JumpToGenericMessageScreen(eMenuScreen, const char* title, const char* message)`
/// (0x576AE0, thiscall; part in the `.text` chunk 0x4067CC): `SwitchToNewScreen(screen)` (the dword
/// passed on); screens 0x14 / 0x15 set the current screen's second item target to 1 / 0x10; then
/// `strncpy` of the title (8) to the screen name and of the message (8) to the first item's text.
#[hook(addr = 0x576AE0, conv = thiscall, name = "CMenuManager::JumpToGenericMessageScreen", expect = "53 8B 5C 24 08 56 53")]
pub fn jump_to_generic_message_screen(this: GPtr<u8>, scr: u32, title: GPtr<u8>, message: GPtr<u8>) {
    switch_to_new_screen_call(this, scr);
    let cur = || screen(i32::from(b8(this, 0x15D) as i8));
    match scr as u8 as i8 {
        0x14 => wr::<u8>(cur(), 0x26, 1),
        0x15 => wr::<u8>(cur(), 0x26, 0x10),
        _ => {}
    }
    crt_strncpy(cur(), title, 8);
    crt_strncpy(cur().byte_add(0xB), message, 8);
}

/// `CMenuManager::DoSettingsBeforeStartingAGame()` (0x573330, thiscall): zeroes 14 dwords at
/// 0xB78E58, resets the audio engine when +0x5C, picks a random radio station 1..13 (+0x52), sets +0x32
/// / +0x5D, clears the blip +0x2C, `CStats::UpdateRespectStat(1)`, clears 0x96918C,
/// `CHud::SetZoneName(null, true)`, and fades the camera in from black (fade 0 out, process, fade 2 s
/// in).
#[hook(addr = 0x573330, conv = thiscall, name = "CMenuManager::DoSettingsBeforeStartingAGame", expect = "56 8B F1 57 33 C0 B9 0E 00 00 00")]
pub fn do_settings_before_starting_a_game(this: GPtr<u8>) {
    for k in 0..14 {
        g::<u32>(0xB78E58 + k * 4).write(0);
    }
    if b8(this, 0x5C) != 0 {
        audio_reset(g(AUDIO_ENGINE));
    }
    let r = audio_random_in_range(1, 0xD);
    wr(this, 0x52, r as u8);
    let blip: i32 = rd(this, 0x2C);
    wr::<u8>(this, 0x32, 1);
    wr::<u8>(this, 0x5D, 1);
    if blip != 0 {
        radar_clear_blip(blip);
        wr::<u32>(this, 0x2C, 0);
    }
    stats_update_respect_stat(1);
    g::<u8>(0x96918C).write(0);
    hud_set_zone_name(0, 1);
    let cam = g(THE_CAMERA);
    camera_set_fade_colour(cam, 0, 0, 0);
    camera_fade(cam, 0, 0);
    camera_process_fade(cam);
    camera_fade(cam, 0x4000_0000, 1);
}

/// `CMenuManager::InitialiseChangedLanguageSettings(bool reinitControls)` (0x573260, thiscall): when
/// +0xEB is set: cleared, timer stopped, the GXT reloaded (keeping the mission pack text when one is
/// selected) and the loaded mission text reloaded, timer updated; then the localisation by +0x88 (1 =
/// French game; 2 = nothing) or else by language (1 French, 0 / 2..4 normal, other nothing), and +0x88 ==
/// 1 French again; `ReinitControls` when asked.
#[hook(addr = 0x573260, conv = thiscall, name = "CMenuManager::InitialiseChangedLanguageSettings", expect = "83 EC 08 56 8B F1 8A 86 EB 00 00 00")]
pub fn initialise_changed_language_settings(this: GPtr<u8>, reinit: u32) {
    if b8(this, 0xEB) == 0 {
        return;
    }
    wr::<u8>(this, 0xEB, 0);
    timer_stop();
    // 8 bytes on the original's stack (unbounded copy); larger here.
    let mut name = [0u8; 0x20];
    let np: GPtr<u8> = local(&mut name);
    text_get_name_of_loaded_mission_text(g(THE_TEXT), np);
    text_load(g(THE_TEXT), u32::from(g::<u8>(MISSION_PACK).read() != 0));
    if b8(np, 0) != 0 {
        text_load_mission_text(g(THE_TEXT), np);
    }
    timer_update();
    let title: i32 = rd(this, 0x88);
    if title != 1 && title != 2 {
        match b8(this, 0x84) as i8 {
            1 => localisation_set_french_game(),
            0 | 2..=4 => localisation_set_normal_game(),
            _ => {}
        }
    }
    if rd::<i32>(this, 0x88) == 1 {
        localisation_set_french_game();
    }
    if reinit as u8 != 0 {
        ccm_reinit_controls(g(CONTROLS_MANAGER));
    }
}

/// `CMenuManager::SetDefaultPreferences(eMenuScreen)` (0x573AE0, thiscall): screen 3 (audio): volumes
/// 0x40, +0x4E / +0x51 / +0x4D on, +0xC5 / +0xB0 off, then the audio engine setters (music, effects,
/// bass, auto-retune: a tail jump). Screens 4 / 0x1B (display): FX quality 2, brightness 256 and
/// `SetGamma(0.5, true)`, draw distance 1.2 (also `ms_lodDistScale`), +0x4C / +0x21 / +0xE8 / +0xC4
/// on, +0xC8 / +0xCC = 1, +0x4B / +0x4A / +0x24 off, +0xD8 = +0xD4, the five blip toggles on, +0x44 off.
/// Screen 0x24 (controls): +0xD0 = 0, 0xBA6745 / 0xC1CC02 off, mouse sensitivity 0.0025, mouse steering
/// and 0xC1CC03 on, +0xB1..+0xB6 off.
#[hook(addr = 0x573AE0, conv = thiscall, name = "CMenuManager::SetDefaultPreferences", expect = "0F BE 44 24 04 83 C0 FD")]
pub fn set_default_preferences(this: GPtr<u8>, scr: u32) {
    let idx = i32::from(scr as u8 as i8).wrapping_sub(3) as u32;
    if idx > 0x21 {
        return;
    }
    match idx {
        0 => {
            wr::<u8>(this, 0x4F, 0x40);
            wr::<u8>(this, 0x50, 0x40);
            wr::<u8>(this, 0x4E, 1);
            wr::<u8>(this, 0x51, 1);
            wr::<u8>(this, 0x4D, 1);
            wr::<u8>(this, 0xC5, 0);
            wr::<u8>(this, 0xB0, 0);
            let ae = g(AUDIO_ENGINE);
            audio_set_music_volume(ae, u32::from(b8(this, 0x50)));
            audio_set_effects_volume(ae, u32::from(b8(this, 0x4F)));
            audio_set_bass_enhance(ae, u32::from(b8(this, 0x51)));
            audio_set_radio_auto_retune(ae, u32::from(b8(this, 0x4D)));
        }
        1 | 24 => {
            fx_set_quality(g(G_FX), 2);
            wr::<u32>(this, 0x3C, 0x100);
            gamma_set(g(0xC92134), 0x3F00_0000, 1);
            wr::<u32>(this, 0x40, 0x3F99_999A);
            g::<u32>(0x8CD800).write(0x3F99_999A);
            let d: u32 = rd(this, 0xD4);
            wr::<u8>(this, 0x4C, 1);
            wr::<u8>(this, 0x21, 1);
            wr::<u8>(this, 0xE8, 1);
            wr::<u8>(this, 0xC4, 1);
            wr::<u32>(this, 0xC8, 1);
            wr::<u32>(this, 0xCC, 1);
            wr::<u8>(this, 0x4B, 0);
            wr::<u8>(this, 0x4A, 0);
            wr::<u32>(this, 0x24, 0);
            wr(this, 0xD8, d);
            wr::<u32>(this, 0x45, 0x0101_0101);
            wr::<u8>(this, 0x49, 1);
            wr::<u8>(this, 0x44, 0);
        }
        33 => {
            wr::<u8>(this, 0xD0, 0);
            g::<u8>(0xBA6745).write(0);
            g::<u8>(0xC1CC02).write(0);
            g::<u32>(0xB6EC1C).write(0x3B23_D70A);
            g::<u8>(0xB6EC2E).write(1);
            g::<u8>(0xC1CC03).write(1);
            for k in 0xB1..=0xB6 {
                wr::<u8>(this, k, 0);
            }
        }
        _ => {}
    }
}

/// `CMenuManager::SaveSettings()` (0x57C660, thiscall): `gta_sa.set` in the user directory ("wb"): the
/// version dword (0x865588), the controls (`CControllerConfigManager::SaveSettings`), then the settings
/// in the original's order (mouse sensitivity, 0xBA6745, 0xC1CC02, 0xC1CC03, volumes, radio, ...,
/// the FX quality, constant bytes 0x54 / 0x1D / 0x5F and the RW subsystem); write results unchecked;
/// closed, directory reset. Nothing but the directory reset when the file does not open.
#[hook(addr = 0x57C660, conv = thiscall, name = "CMenuManager::SaveSettings", expect = "83 EC 0C 56 57 8B F9")]
pub fn save_settings(this: GPtr<u8>) {
    file_set_dir_my_documents();
    let f = file_open(g(0x865674), g(0x865680));
    let mut pad = [0u8, 0x54, 0x1D, 0x5F];
    let pp = local(&mut pad);
    if !f.is_null() {
        let w = |p: GPtr<u8>, n: u32| {
            file_write(f, p, n);
        };
        let t = |off: isize| this.byte_add(off);
        w(g(0x865588), 4);
        ccm_save_settings(g(CONTROLS_MANAGER), f);
        w(g(0xB6EC1C), 4);
        w(g(0xBA6745), 1);
        w(g(0xC1CC02), 1);
        w(g(0xC1CC03), 1);
        for (o, n) in [(0x4F, 1), (0x50, 1), (0x52, 1), (0x4D, 1), (0x51, 1), (0x3C, 4), (0xC4, 1), (0xC5, 1), (0xC8, 4)] {
            w(t(o), n);
        }
        let mut q = fx_get_quality(g(G_FX));
        w(local(&mut q), 4);
        w(pp.byte_add(1), 1);
        for (o, n) in [(0x40, 4), (0x44, 1), (0x4B, 1), (0x4C, 1), (0xD4, 4), (0xD0, 1), (0x84, 1), (0x21, 1), (0x24, 4), (0xB0, 1), (0xE8, 1)] {
            w(t(o), n);
        }
        w(pp.byte_add(2), 1);
        for (o, n) in [(0xB1, 1), (0xB2, 1), (0xB3, 1), (0xB4, 1), (0xB5, 1), (0xB6, 1), (0x4A, 1), (0xAC, 4)] {
            w(t(o), n);
        }
        let mut sub = rw_engine_get_current_subsystem();
        w(local(&mut sub), 4);
        w(pp.byte_add(3), 1);
        file_close(f);
    }
    file_set_dir(g(S_EMPTY));
}

/// 1.4 (0x859994), 16.0 (0x8599D0), 10.0 (0x85862C)
const K_1_4: f32 = f32::from_bits(0x3FB3_3333);
const K_16: f32 = f32::from_bits(0x4180_0000);
const K_10: f32 = f32::from_bits(0x4120_0000);

/// `CMenuManager::DrawWindow(const CRect& rect, const char* key, uint8 colour, CRGBA back, bool unused,
/// bool background)` (0x573EE0, thiscall): the background rect when asked; with a non-empty key the
/// title text: wrap at the rect's right, colour `(0xE1 - colour)` grey, black drop colour, edge 2,
/// left aligned, font 0, scale `(W/640, H/448 * 1.4)`, at `x = W/640 * 10 + left`, `y = top - H/448 *
/// 16` where top is +0xC when it is below +4 (else +4).
#[hook(addr = 0x573EE0, conv = thiscall, name = "CMenuManager::DrawWindow", expect = "8A 44 24 18 84 C0 56")]
pub fn draw_window(_this: GPtr<u8>, rect: GPtr<u8>, key: GPtr<u8>, colour: u32, back: u32, _unused: u32, bg: u32) {
    if bg as u8 != 0 {
        let mut c = back;
        sprite_draw_rect(rect, local(&mut c));
    }
    if key.is_null() || b8(key, 0) == 0 {
        return;
    }
    font_set_wrapx(rd(rect, 8));
    let grey = u32::from(0xE1u8.wrapping_sub(colour as u8));
    let mut tmp = 0u32;
    let tp = local(&mut tmp);
    font_set_color(rd(rgba_ctor(tp, grey, grey, grey, 0xFF), 0));
    font_set_drop_color(rd(rgba_ctor(tp, 0, 0, 0, 0xFF), 0));
    font_set_edge(2);
    font_set_orientation(1);
    font_set_font_style(0);
    let h = xi(g::<i32>(RS_MAX_HEIGHT).read())
        .fmul_m(K_INV_448)
        .fmul_m(K_1_4)
        .to_f32();
    let w = xi(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(K_INV_640).to_f32();
    font_set_scale_for_current_language(w, h.to_bits());
    let bottom: f32 = rd(rect, 0xC);
    let top: f32 = rd(rect, 4);
    let y = if x(bottom).fcom(x(top)).test_parity(0x05) { top } else { bottom };
    let text = text_get(g(THE_TEXT), key);
    let py = xi(g::<i32>(RS_MAX_HEIGHT).read())
        .fmul_m(K_INV_448)
        .fmul_m(K_16)
        .fsubr_m(y)
        .to_f32();
    let px = xi(g::<i32>(RS_MAX_WIDTH).read())
        .fmul_m(K_INV_640)
        .fmul_m(K_10)
        .fadd_m(rd(rect, 0))
        .to_f32();
    font_print_string(px, py, text);
}

/// 1/6 (0x85F0A0), 3.0 (0x858B3C), 60.0 (0x858B34), 580.0 (0x865260), 388.0 (0x86525C)
const K_SIXTH: f32 = f32::from_bits(0x3E2A_AAAB);
const K_3: f32 = f32::from_bits(0x4040_0000);
const K_60: f32 = f32::from_bits(0x4270_0000);
const K_580: f32 = f32::from_bits(0x4411_0000);
const K_388: f32 = f32::from_bits(0x43C2_0000);

/// `CMenuManager::ProcessStreaming(bool loadAll)` (0x573CF0, thiscall): on the map screen (menu active
/// +0x5C, screen 5, map loaded +0x78, +0x5A clear) every 12 x 12 radar section is requested when its
/// box (`origin - zoom + i * zoom/6`) lies inside (60 - zoom/2, 580 / 388 + zoom/2) and removed
/// otherwise; then all requested models (+0x5B cleared) or the requested models are loaded.
#[hook(addr = 0x573CF0, conv = thiscall, name = "CMenuManager::ProcessStreaming", expect = "83 EC 20 53 8B D9 8A 43 5C")]
pub fn process_streaming(this: GPtr<u8>, load_all: u32) {
    if b8(this, 0x5C) == 0 || b8(this, 0x15D) != 5 || b8(this, 0x78) == 0 || b8(this, 0x5A) != 0 {
        return;
    }
    let zoom: f32 = rd(this, 0x64);
    let s = x(zoom).fmul_m(K_SIXTH).to_f32();
    let ox = x(rd(this, 0x68)).fsub_m(zoom).to_f32();
    let oy = x(rd(this, 0x6C)).fsub_m(zoom).to_f32();
    let s3 = x(s).fmul_m(K_3).to_f32();
    let lim = x(K_60).fsub_m(s3).to_f32();
    for xs in 0..12i32 {
        let left = xi(xs).fmul_m(s).fadd_m(ox).to_f32();
        for ys in 0..12i32 {
            let inside = !x(left).fcom(x(lim)).test_any(0x41)
                && !xi(ys).fmul_m(s).fadd_m(oy).fcom(x(lim)).test_any(0x41)
                && !x(s3)
                    .fadd_m(K_580)
                    .fcom(xi(xs + 1).fmul_m(s).fadd_m(ox))
                    .test_any(0x41)
                && !x(s3)
                    .fadd_m(K_388)
                    .fcom(xi(ys + 1).fmul_m(s).fadd_m(oy))
                    .test_any(0x41);
            if inside {
                radar_request_map_section(xs, ys);
            } else {
                radar_remove_map_section(xs, ys);
            }
        }
    }
    if load_all as u8 != 0 {
        streaming_load_all_requested_models(0);
        wr::<u8>(this, 0x5B, 0);
    } else {
        streaming_load_requested_models();
    }
}

orig! {
    /// `CFileMgr::Read(FILE*, void*, int32)` (0x538950)
    pub fn file_read(f: GPtr<u8>, buf: GPtr<u8>, size: u32) -> u32 = cdecl 0x538950 => sa_core::filemgr::read;
    /// `CFileMgr::Seek(FILE*, int32, int32)` (0x538990)
    pub fn file_seek(f: GPtr<u8>, offset: i32, origin: i32) -> bool = cdecl 0x538990 => sa_core::filemgr::seek;
    /// CRT `strncmp` (0x8214D0)
    pub fn crt_strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
    /// CRT `_stricmp` (0x8229B6)
    pub fn crt_stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// `CControllerConfigManager::LoadSettings(FILE*)` (0x530530, thiscall)
    pub fn ccm_load_settings(this: GPtr<u8>, f: GPtr<u8>) -> u8 = thiscall 0x530530 => unported;
    /// `OutputDebugStringA` (import slot 0x858118)
    pub fn output_debug_string(s: GPtr<u8>) = stdcall cell 0x858118 => unported;
    /// `CMenuManager::SetDefaultPreferences(eMenuScreen)` (0x573AE0, thiscall)
    pub fn set_default_preferences_call(this: GPtr<u8>, scr: u32) = thiscall 0x573AE0 => set_default_preferences;
    /// `CMenuManager::InitialiseChangedLanguageSettings(bool)` (0x573260, thiscall)
    pub fn initialise_changed_language_settings_call(this: GPtr<u8>, reinit: u32) = thiscall 0x573260 => initialise_changed_language_settings;
    /// `CMenuManager::DrawWindow(...)` (0x573EE0, thiscall)
    pub fn draw_window_call(this: GPtr<u8>, rect: GPtr<u8>, key: GPtr<u8>, colour: u32, back: u32, unused: u32, bg: u32) = thiscall 0x573EE0 => draw_window;
    /// `CMenuManager::SmallMessageScreen(const char*)` (0x574010, thiscall)
    pub fn small_message_screen_call(this: GPtr<u8>, key: GPtr<u8>) = thiscall 0x574010 => small_message_screen;
    /// `CFont::SetBackground(bool, bool)` (0x7195C0)
    pub fn font_set_background(on: u32, outline: u32) = cdecl 0x7195C0 => crate::font::set_background;
    /// `CFont::SetProportional(bool)` (0x7195B0)
    pub fn font_set_proportional(v: u32) = cdecl 0x7195B0 => crate::font::set_proportional;
    /// `CFont::SetCentreSize(float)` (0x7194E0)
    pub fn font_set_centre_size(v: u32) = cdecl 0x7194E0 => crate::font::set_centre_size;
    /// `CFont::SetDropShadowPosition(int8)` (0x719570)
    pub fn font_set_drop_shadow_position(v: u32) = cdecl 0x719570 => crate::font::set_drop_shadow_position;
    /// `CFont::SetScale(float, float)` (0x719380)
    pub fn font_set_scale(w: u32, h: u32) = cdecl 0x719380 => crate::font::set_scale;
    /// `CFont::GetNumberLines(float, float, const GxtChar*)` (0x71A5E0)
    pub fn font_get_number_lines(px: f32, py: f32, text: GPtr<u8>) -> u32 = cdecl 0x71A5E0 => crate::font::get_number_lines;
    /// `CFont::DrawFonts()` (0x71A210)
    pub fn font_draw_fonts() = cdecl 0x71A210 => crate::font_print::draw_fonts;
    /// `CFont::InitPerFrame()` (0x719800)
    pub fn font_init_per_frame() = cdecl 0x719800 => crate::font::init_per_frame;
    /// `CHudColours::GetRGB(CRGBA*, uint8)` (0x58FEA0, thiscall)
    pub fn hud_colours_get_rgb(this: GPtr<u8>, out: GPtr<u8>, colour: u32) -> GPtr<u8> = thiscall 0x58FEA0 => crate::hud::get_rgb;
    /// `AsciiToGxtChar(const char*, GxtChar*)` (0x718600)
    pub fn ascii_to_gxt_char(src: GPtr<u8>, dst: GPtr<u8>) = cdecl 0x718600 => unported;
    /// `RsCameraBeginUpdate(RwCamera*)` (0x619450)
    pub fn rs_camera_begin_update(cam: u32) -> i32 = cdecl 0x619450 => unported;
    /// `CSprite2d::InitPerFrame()` (0x727260)
    pub fn sprite2d_init_per_frame() = cdecl 0x727260 => unported;
    /// `CSprite2d::SetRecipNearClip()` (0x727350)
    pub fn sprite2d_set_recip_near_clip() = cdecl 0x727350 => unported;
    /// `DefinedState()` (0x734750)
    pub fn defined_state() = cdecl 0x734750 => unported;
}

/// 1/512 (0x863A44)
const K_INV_512: f32 = f32::from_bits(0x3B00_0000);

/// `CMenuManager::LoadSettings()` (0x57C8F0, thiscall): `gta_sa.set` from the user directory ("rb");
/// 0xBA6745 is set first. A file starting with "THIS FILE IS NOT VALID YET" (26 bytes compared), a
/// version below 6, failing controls, or wrong check bytes 0x54 / 0x1D / 0x5F give the defaults (screens
/// 3, 4, 0x1B, 0x24; +0xDC = 0, the old language back, station 1). Otherwise the settings are read in
/// the save order but brightness mode into +0xCC (saved from +0xC8), draw distance mode into +0xD8
/// (saved from +0xD4) and the subsystem into +0xDC; then mouse steering = (+0xD0 == 0), the LOD scale,
/// FX quality, gamma (`+0x3C / 512`), +0xC8 = +0xCC, +0x1ADE set, the audio settings and radio; a
/// changed language reloads the text (+0x8C, +0xEB, `InitialiseChangedLanguageSettings(false)`,
/// `OutputDebugStringA`). The file is closed, the directory reset.
#[hook(addr = 0x57C8F0, conv = thiscall, name = "CMenuManager::LoadSettings", expect = "83 EC 44 56 57 8B F9")]
pub fn load_settings(this: GPtr<u8>) {
    file_set_dir_my_documents();
    let f = file_open(g(0x865674), g(0x859C6C));
    if f.is_null() {
        file_set_dir(g(S_EMPTY));
        return;
    }
    let old_lang = i32::from(b8(this, 0x84) as i8);
    g::<u8>(0xBA6745).write(1);
    let mut checks = [0u8; 4];
    let cp = local(&mut checks);
    let mut version: i32 = 0;
    let mut fxq: u32 = 0;
    let mut hdr = [0u8; 0x1D];
    let hp = local(&mut hdr);
    let r = |p: GPtr<u8>, n: u32| {
        file_read(f, p, n);
    };
    let t = |off: isize| this.byte_add(off);
    r(hp, 0x1D);
    let mut valid = crt_strncmp(hp, g(0x8D2BE4), 0x1A) != 0;
    if valid {
        file_seek(f, 0, 0);
        r(local(&mut version), 4);
        valid = version >= 6 && ccm_load_settings(g(CONTROLS_MANAGER), f) != 0;
    }
    if valid {
        r(g(0xB6EC1C), 4);
        r(g(0xBA6745), 1);
        r(g(0xC1CC02), 1);
        r(g(0xC1CC03), 1);
        for (o, n) in [(0x4F, 1), (0x50, 1), (0x52, 1), (0x4D, 1), (0x51, 1), (0x3C, 4), (0xC4, 1), (0xC5, 1), (0xCC, 4)] {
            r(t(o), n);
        }
        r(local(&mut fxq), 4);
        r(cp.byte_add(1), 1);
        for (o, n) in [(0x40, 4), (0x44, 1), (0x4B, 1), (0x4C, 1), (0xD8, 4), (0xD0, 1), (0x84, 1), (0x21, 1), (0x24, 4), (0xB0, 1), (0xE8, 1)] {
            r(t(o), n);
        }
        r(cp.byte_add(2), 1);
        for (o, n) in [(0xB1, 1), (0xB2, 1), (0xB3, 1), (0xB4, 1), (0xB5, 1), (0xB6, 1), (0x4A, 1), (0xAC, 4), (0xDC, 4)] {
            r(t(o), n);
        }
        r(cp.byte_add(3), 1);
        valid = b8(cp, 1) == 0x54 && b8(cp, 2) == 0x1D && b8(cp, 3) == 0x5F;
    }
    if valid {
        g::<u8>(0xB6EC2E).write(u8::from(b8(this, 0xD0) == 0));
        g::<u32>(0x8CD800).write(rd(this, 0x40));
        fx_set_quality(g(G_FX), fxq);
        let gamma = xi(rd(this, 0x3C)).fmul_m(K_INV_512).to_f32();
        gamma_set(g(0xC92134), gamma.to_bits(), 1);
        wr(this, 0xC8, rd::<u32>(this, 0xCC));
        wr::<u8>(this, 0x1ADE, 1);
        let ae = g(AUDIO_ENGINE);
        audio_set_music_volume(ae, u32::from(b8(this, 0x50)));
        audio_set_effects_volume(ae, u32::from(b8(this, 0x4F)));
        audio_set_bass_enhance(ae, u32::from(b8(this, 0x51)));
        audio_set_radio_auto_retune(ae, u32::from(b8(this, 0x4D)));
        audio_retune_radio(ae, u32::from(b8(this, 0x52)));
        if i32::from(b8(this, 0x84) as i8) != old_lang {
            wr::<u8>(this, 0x8C, 1);
            text_load(g(THE_TEXT), 0);
            wr::<u8>(this, 0xEB, 1);
            initialise_changed_language_settings_call(this, 0);
            output_debug_string(g(0x865684));
        } else {
            wr::<u8>(this, 0x8C, 0);
        }
    } else {
        for s in [3, 4, 0x1B, 0x24] {
            set_default_preferences_call(this, s);
        }
        wr::<u32>(this, 0xDC, 0);
        wr(this, 0x84, old_lang as u8);
        wr::<u8>(this, 0x52, 1);
    }
    file_close(f);
    file_set_dir(g(S_EMPTY));
}

/// `CMenuManager::SmallMessageScreen(const char* key)` (0x574010, thiscall): font state (no background,
/// proportional, style 1, centre size `W/640 * 430`, centred, HUD colour 4, no shadow, scale 0.56 x 1
/// at 640 x 448 else `W * 0.000875`, `H/448`), the black window {D, H - A, W - B, C} via `DrawWindow`
/// (A, B, C, D from fixed values at 640 x 448, else scaled), the line count at (320 / W*0.5, 135 /
/// H*0.3013); the text y `_ftol(200 - 9 * lines)` (scaled: `H*0.4464 - lines * H*0.0201`) for more than
/// one line, else `_ftol(190 / H*0.4241)`; a CD-error key (NOCD, OPENCD, WRONGCD, CDERROR,
/// case-insensitive) without `TheText.m_bCdErrorLoaded` converts the ASCII CD-error text INTO the GXT
/// string; then `PrintString(x, (float)y, text)`.
#[hook(addr = 0x574010, conv = thiscall, name = "CMenuManager::SmallMessageScreen", expect = "83 EC 18 56 57 6A 00 6A 00")]
pub fn small_message_screen(this: GPtr<u8>, key: GPtr<u8>) {
    font_set_background(0, 0);
    font_set_proportional(1);
    font_set_font_style(1);
    let w = || g::<i32>(RS_MAX_WIDTH).read();
    let h = || g::<i32>(RS_MAX_HEIGHT).read();
    font_set_centre_size(
        xi(w())
            .fmul_m(K_INV_640)
            .fmul_m(f32::from_bits(0x43D7_0000))
            .to_f32()
            .to_bits(),
    );
    font_set_orientation(0);
    let mut c = 0u32;
    let cp = local(&mut c);
    font_set_color(rd(hud_colours_get_rgb(g(0xBAB22C), cp, 4), 0));
    font_set_drop_shadow_position(0);
    let sy = if h() == 0x1C0 {
        0x3F80_0000
    } else {
        xi(h()).fmul_m(K_INV_448).to_f32().to_bits()
    };
    let sx = if w() == 0x280 {
        0x3F0F_5C29
    } else {
        xi(w()).fmul_m(f32::from_bits(0x3A65_6042)).to_f32().to_bits()
    };
    font_set_scale(sx, sy);
    let a = if h() == 0x1C0 { x(f32::from_bits(0x4325_0000)) } else { xi(h()).fmul_m(f32::from_bits(0x3EBC_9249)) };
    let bb = if w() == 0x280 { x(f32::from_bits(0x42BE_0000)) } else { xi(w()).fmul_m(f32::from_bits(0x3E18_0000)) };
    let cc = if h() == 0x1C0 { x(f32::from_bits(0x42FA_0000)) } else { xi(h()).fmul_m(f32::from_bits(0x3E8E_DB6E)) };
    let dd = if w() == 0x280 { x(f32::from_bits(0x42BE_0000)) } else { xi(w()).fmul_m(f32::from_bits(0x3E18_0000)) };
    let mut rect = [dd.to_f32(), 0.0, 0.0, cc.to_f32()];
    rect[2] = xi(w()).fsub(bb).to_f32();
    rect[1] = xi(h()).fsub(a).to_f32();
    let mut col = 0u32;
    let colp = local(&mut col);
    let back: u32 = rd(rgba_ctor(colp, 0, 0, 0, 0xFF), 0);
    draw_window_call(this, local(&mut rect), GPtr::NULL, 0, back, 0, 1);
    let ly = if h() == 0x1C0 {
        f32::from_bits(0x4307_0000)
    } else {
        xi(h()).fmul_m(f32::from_bits(0x3E9A_4925)).to_f32()
    };
    let lx = if w() == 0x280 {
        f32::from_bits(0x43A0_0000)
    } else {
        xi(w()).fmul_m(K_HALF).to_f32()
    };
    let lines = font_get_number_lines(lx, ly, text_get(g(THE_TEXT), key)) as u16 as i16;
    let yv = if i32::from(lines) > 1 {
        let (p, q) = if h() == 0x1C0 {
            (x(f32::from_bits(0x4348_0000)), x(f32::from_bits(0x4110_0000)))
        } else {
            let hf = xi(h()).to_f32();
            (
                xi(h()).fmul_m(f32::from_bits(0x3EE4_9249)),
                x(hf).fmul_m(f32::from_bits(0x3CA4_9249)),
            )
        };
        crate::common::ftol32(xi(i32::from(lines)).fmul(q).fsubr(p))
    } else if h() == 0x1C0 {
        crate::common::ftol32(x(f32::from_bits(0x433E_0000)))
    } else {
        crate::common::ftol32(xi(h()).fmul_m(f32::from_bits(0x3ED9_2492)))
    };
    let text = text_get(g(THE_TEXT), key);
    if g::<u8>(0xC1B362).read() == 0
        && (crt_stricmp(g(0x86527C), key) == 0
            || crt_stricmp(g(0x865274), key) == 0
            || crt_stricmp(g(0x86526C), key) == 0
            || crt_stricmp(g(0x865264), key) == 0)
    {
        ascii_to_gxt_char(g(0xC1B36C), text);
    }
    let px = if w() == 0x280 {
        f32::from_bits(0x43A0_0000)
    } else {
        xi(w()).fmul_m(K_HALF).to_f32()
    };
    font_print_string(px, xi(yv).to_f32(), text);
}

/// `Scene.m_pRwCamera` (0xC1703C): `MessageScreen` reads it through the protection chunk 0x40783C
/// (`[0x1560884] - [0x665DA4]`, values from the exe = 0xC1703C).
const SCENE_CAMERA: usize = 0xC1703C;

/// The black full-screen rect {0, H, W, 0} of `MessageScreen`.
fn black_screen() {
    let mut rect = [
        0.0,
        xi(g::<i32>(RS_MAX_HEIGHT).read()).to_f32(),
        xi(g::<i32>(RS_MAX_WIDTH).read()).to_f32(),
        0.0,
    ];
    let mut c = 0u32;
    let col = rgba_ctor(local(&mut c), 0, 0, 0, 0xFF);
    sprite_draw_rect(local(&mut rect), col);
}

/// `CMenuManager::MessageScreen(const char* key, bool blackBackground, bool cameraUpdateStarted)`
/// (0x579330, thiscall): unless the camera update was already started, `RsCameraBeginUpdate(Scene
/// camera)` (nothing more when it fails) and the black background when asked; then the per-frame sprite
/// / font state, `DefinedState`, the black background again when asked, `SmallMessageScreen(key)` and
/// `DrawFonts`.
#[hook(addr = 0x579330, conv = thiscall, name = "CMenuManager::MessageScreen", expect = "8A 44 24 0C 83 EC 10 84 C0")]
pub fn message_screen(this: GPtr<u8>, key: GPtr<u8>, black: u32, started: u32) {
    let black = black as u8 != 0;
    if started as u8 == 0 {
        if rs_camera_begin_update(g::<u32>(SCENE_CAMERA).read()) == 0 {
            return;
        }
        if black {
            black_screen();
        }
    }
    sprite2d_init_per_frame();
    sprite2d_set_recip_near_clip();
    font_init_per_frame();
    defined_state();
    let mut c = 0u32;
    rgba_ctor(local(&mut c), 0xFF, 0xFF, 0xFF, 0xFF);
    if black {
        black_screen();
    }
    small_message_screen_call(this, key);
    font_draw_fonts();
}

orig! {
    /// `CAudioEngine::IsRadioRetuneInProgress()` (0x506FF0, thiscall)
    pub fn audio_is_radio_retune_in_progress(this: GPtr<u8>) -> u8 = thiscall 0x506FF0 => unported;
    /// `CAudioEngine::Service()` (0x507750, thiscall)
    pub fn audio_service(this: GPtr<u8>) = thiscall 0x507750 => unported;
    /// `CMenuSystem::SwitchOffMenu(uint8)` (0x580750)
    pub fn menu_system_switch_off_menu(menu: u32) = cdecl 0x580750 => unported;
    /// `CPad::Clear(bool, bool)` (0x541A70, thiscall)
    pub fn pad_clear(pad: GPtr<u8>, a: u32, b: u32) = thiscall 0x541A70 => unported;
    /// `CPad::ClearKeyBoardHistory()` (0x53F1E0)
    pub fn pad_clear_keyboard_history() = cdecl 0x53F1E0 => unported;
    /// `CPad::ClearMouseHistory()` (0x541BD0, thiscall)
    pub fn pad_clear_mouse_history(pad: GPtr<u8>) = thiscall 0x541BD0 => unported;
    /// `VideoMode::IsVideoModeExclusive()` (0x745CA0)
    pub fn is_video_mode_exclusive() -> u8 = cdecl 0x745CA0 => unported;
    /// `DIReleaseMouse()` (0x746F70)
    pub fn di_release_mouse() = cdecl 0x746F70 => unported;
    /// `Input::diMouseInit(bool)` (0x7469A0)
    pub fn di_mouse_init(exclusive: u32) = cdecl 0x7469A0 => unported;
    /// `CTimer::EndUserPause()` (0x561B00)
    pub fn timer_end_user_pause() = cdecl 0x561B00 => unported;
    /// `CMenuManager::UnloadTextures()` (0x574630, thiscall)
    pub fn unload_textures_call(this: GPtr<u8>) = thiscall 0x574630 => unload_textures;
    /// `CMenuManager::Initialise()` (0x5744D0, thiscall)
    pub fn initialise_call(this: GPtr<u8>) = thiscall 0x5744D0 => initialise;
    /// `CMenuManager::LoadAllTextures()` (0x572EC0, thiscall)
    pub fn load_all_textures(this: GPtr<u8>) = thiscall 0x572EC0 => load_all_textures_port;
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
    /// `DoRWStuffStartOfFrame(int16 r, g, b, int16 r2, g2, b2, int16 alpha)` (0x53D690)
    pub fn do_rw_stuff_start_of_frame(r: u32, g: u32, b: u32, r2: u32, g2: u32, b2: u32, a: u32) -> u8 = cdecl 0x53D690 => unported;
    /// `DoRWStuffEndOfFrame()` (0x53D840)
    pub fn do_rw_stuff_end_of_frame() = cdecl 0x53D840 => sa_core::platform::do_rw_stuff_end_of_frame;
    /// `CMenuManager::CheckForMenuClosing()` (0x576B70, thiscall)
    pub fn check_for_menu_closing_call(this: GPtr<u8>) = thiscall 0x576B70 => check_for_menu_closing;
    /// `CMenuManager::ProcessStreaming(bool)` (0x573CF0, thiscall)
    pub fn process_streaming_call(this: GPtr<u8>, load_all: u32) = thiscall 0x573CF0 => process_streaming;
    /// `CMenuManager::UserInput()` (0x57FD70, thiscall)
    pub fn user_input(this: GPtr<u8>) = thiscall 0x57FD70 => crate::menu_input::user_input;
    /// `CMenuManager::ProcessFileActions()` (0x578D60, thiscall)
    pub fn process_file_actions(this: GPtr<u8>) = thiscall 0x578D60 => process_file_actions_port;
    /// `D3DResourceSystem::TidyUpD3DIndexBuffers(uint32)` (0x730740)
    pub fn tidy_up_d3d_index_buffers(n: u32) = cdecl 0x730740 => unported;
    /// `D3DResourceSystem::TidyUpD3DTextures(uint32)` (0x7305E0)
    pub fn tidy_up_d3d_textures(n: u32) = cdecl 0x7305E0 => unported;
}

/// `CPad::GetPad(m_nPlayerNumber (+0xEA))`
fn menu_pad(this: GPtr<u8>) -> GPtr<u8> {
    get_pad(u32::from(b8(this, 0xEA)))
}

/// `SetGamma((float)brightness * (1/512), true)`
fn apply_gamma(this: GPtr<u8>) {
    let v = xi(rd(this, 0x3C)).fmul_m(K_INV_512).to_f32();
    gamma_set(g(0xC92134), v.to_bits(), 1);
}

/// Two black frames (`DoRWStuffStartOfFrame(0, ..., 255)` + `DoRWStuffEndOfFrame`).
fn two_black_frames() {
    for _ in 0..2 {
        do_rw_stuff_start_of_frame(0, 0, 0, 0, 0, 0, 0xFF);
        do_rw_stuff_end_of_frame();
    }
}

/// Pad, keyboard and mouse history cleared, then the DirectInput mouse re-initialised (exclusive mode).
fn reset_input(this: GPtr<u8>, exclusive: u32) {
    pad_clear(menu_pad(this), 0, 1);
    menu_pad(this);
    pad_clear_keyboard_history();
    pad_clear_mouse_history(menu_pad(this));
    if is_video_mode_exclusive() != 0 {
        di_release_mouse();
        di_mouse_init(exclusive);
    }
}

/// `m_StatusDisablePlayerControls` (0xBA827C, `this + 0x1B34`)
const SAVED_DISABLE_CONTROLS: usize = 0xBA827C;

/// `CMenuManager::CheckForMenuClosing()` (0x576B70, thiscall). Escape (menu inactive, or on screens
/// 0x29 / 0x10 / 0x16; not with 0xA43088 == 1; with 0xB6F065 set only when +0x34 is clear) or a
/// requested start (+0x32) / pause (+0x33) toggles the menu unless a radio retune runs; screens 0x12 /
/// 0xE only clear +0x32. Closing: radio stopped, event 0x23, the script menu switched off, input
/// cleared (`diMouseInit(1)`), +0x4 = 150, settings saved, state cleared, textures unloaded, the user
/// pause ended, pads reset, gamma, a camera fade-in unless the player holds a camera that is still
/// timing (`m_nTimer` vs `m_snTimeInMilliseconds`), the saved `DisablePlayerControls` restored. Opening
/// (not when +0xF4): two black frames, the pad's `DisablePlayerControls` saved and set to 1 (gr omits
/// the set), input cleared (`diMouseInit(0)`), `Initialise`, `LoadAllTextures`, gamma. Then a pending
/// +0x5F opens the menu on screen 0x16 (0x96918C set) or 0x10.
#[hook(addr = 0x576B70, conv = thiscall, name = "CMenuManager::CheckForMenuClosing", expect = "53 56 8B F1 8A 46 5C 33 DB")]
pub fn check_for_menu_closing(this: GPtr<u8>) {
    let cur = b8(this, 0x15D);
    let mut toggle = false;
    if b8(this, 0x5C) == 0 || cur == 0x29 || cur == 0x10 || cur == 0x16 {
        menu_pad(this);
        if key_just_pressed(KEY_NEW_ESC, KEY_OLD_ESC)
            && g::<u8>(0xA43088).read() != 1
            && (g::<u8>(0xB6F065).read() == 0 || b8(this, 0x34) != 0)
        {
            toggle = true;
        }
    }
    if !toggle && (b8(this, 0x32) != 0 || b8(this, 0x33) != 0) {
        toggle = true;
    }
    if toggle && audio_is_radio_retune_in_progress(g(AUDIO_ENGINE)) == 0 {
        let cur = b8(this, 0x15D);
        if cur == 0x12 || cur == 0xE {
            wr::<u8>(this, 0x32, 0);
            return;
        }
        if (b8(this, 0x35) == 0 || b8(this, 0x33) == 0) && b8(this, 0x60) == 0 {
            audio_report_frontend_event(g(AUDIO_ENGINE), 0, 0, 0x3F80_0000);
            audio_service(g(AUDIO_ENGINE));
        }
        wr(this, 0x5C, u8::from(b8(this, 0x5C) == 0));
        if b8(this, 0x32) != 0 {
            wr::<u8>(this, 0x5C, 0);
        }
        if b8(this, 0x33) != 0 {
            wr::<u8>(this, 0x5C, 1);
        }
        if b8(this, 0x5C) == 0 {
            audio_stop_radio(g(AUDIO_ENGINE), 0, 0);
            audio_report_frontend_event(g(AUDIO_ENGINE), 0x23, 0, 0x3F80_0000);
            let menu = b8(this, 0x30);
            if (menu as i8) >= 0 {
                menu_system_switch_off_menu(u32::from(menu));
                wr::<u8>(this, 0x30, 0x9D);
            }
            reset_input(this, 1);
            wr::<u32>(this, 0x4, 0x4316_0000);
            save_settings_call(this);
            wr::<u32>(this, 0xF0, 0);
            wr::<u32>(this, 0xEC, 0);
            wr::<u8>(this, 0x1AE8, 0);
            wr::<u8>(this, 0x32, 0);
            wr::<u8>(this, 0x33, 0);
            wr::<u8>(this, 0x1B09, 0);
            wr::<u8>(this, 0x5F, 0);
            unload_textures_call(this);
            timer_end_user_pause();
            timer_update();
            wr::<u8>(menu_pad(this), 0x117, 1);
            wr::<u32>(menu_pad(this), 0x120, 0);
            wr::<u32>(get_pad(1), 0x120, 0);
            apply_gamma(this);
            if b8(this, 0xF4) != 0 {
                let ped = find_player_ped(-1);
                let slot = i32::from(rd::<u8>(ped, 0x718) as i8);
                let ty: i32 = rd(ped, (slot * 0x1C + 0x5A0) as isize);
                let timing = ty == 0x2B
                    && g::<u32>(0xB7CB84).read() < rd::<u32>(ped, ((slot + 0x34) * 0x1C) as isize);
                if !timing {
                    let cam = g(THE_CAMERA);
                    camera_set_fade_colour(cam, 0, 0, 0);
                    camera_fade(cam, 0, 0);
                    camera_process_fade(cam);
                    camera_fade(cam, 0x3E4C_CCCD, 1);
                }
            }
            wr::<u8>(this, 0xF4, 0);
            let pad = menu_pad(this);
            wr(pad, 0x10E, g::<u16>(SAVED_DISABLE_CONTROLS).read());
        } else if b8(this, 0xF4) == 0 {
            two_black_frames();
            let v = rd::<u16>(menu_pad(this), 0x10E);
            g::<u16>(SAVED_DISABLE_CONTROLS).write(v);
            wr::<u16>(menu_pad(this), 0x10E, 1);
            reset_input(this, 0);
            initialise_call(this);
            load_all_textures(this);
            apply_gamma(this);
        }
    }
    // 0x576EF1
    if b8(this, 0x5F) == 0 {
        wr::<u8>(this, 0x33, 0);
        wr::<u8>(this, 0x32, 0);
        return;
    }
    two_black_frames();
    audio_report_frontend_event(g(AUDIO_ENGINE), 0, 0, 0x3F80_0000);
    audio_service(g(AUDIO_ENGINE));
    let v = rd::<u16>(menu_pad(this), 0x10E);
    g::<u16>(SAVED_DISABLE_CONTROLS).write(v);
    wr::<u16>(menu_pad(this), 0x10E, 1);
    wr::<u8>(this, 0x5F, 0);
    wr::<u8>(this, 0x5C, 1);
    wr::<u8>(this, 0xF4, 1);
    if is_video_mode_exclusive() != 0 {
        di_release_mouse();
        di_mouse_init(0);
    }
    initialise_call(this);
    load_all_textures(this);
    let restart = g::<u8>(0x96918C).read() != 0;
    wr::<u32>(this, 0x54, 0);
    if restart {
        wr::<u8>(this, 0x33, 0);
        wr::<u8>(this, 0x32, 0);
        wr::<u8>(this, 0x15D, 0x16);
        return;
    }
    wr::<u8>(this, 0x15D, 0x10);
    wr::<u8>(this, 0x33, 0);
    wr::<u8>(this, 0x32, 0);
}

/// `CMenuManager::Process()` (0x57B440, thiscall): with the menu active, `ProcessStreaming(+0x5B)`,
/// `UserInput`, `ProcessFileActions` and the D3D index-buffer / texture tidy-ups; then (tail jump)
/// `CheckForMenuClosing`.
#[hook(addr = 0x57B440, conv = thiscall, name = "CMenuManager::Process", expect = "56 8B F1 8A 46 5C 84 C0")]
pub fn process(this: GPtr<u8>) {
    if b8(this, 0x5C) != 0 {
        process_streaming_call(this, u32::from(b8(this, 0x5B)));
        user_input(this);
        process_file_actions(this);
        tidy_up_d3d_index_buffers(1);
        tidy_up_d3d_textures(1);
    }
    check_for_menu_closing_call(this);
}

orig! {
    /// `RwD3D9ChangeMultiSamplingLevels(int32)` (0x7F8A90)
    pub fn rw_d3d9_change_multisampling_levels(n: u32) = cdecl 0x7F8A90 => unported;
    /// `VideoMode::SetVideoMode(int32)` (0x745C70)
    pub fn set_video_mode(mode: u32) = cdecl 0x745C70 => unported;
    /// `CMenuManager::CentreMousePointer()` (0x57C520, thiscall)
    pub fn centre_mouse_pointer_call(this: GPtr<u8>) = thiscall 0x57C520 => centre_mouse_pointer;
    /// `CRadar::SetMapCentreToPlayerCoords()` (0x585B20)
    pub fn radar_set_map_centre_to_player_coords() = cdecl 0x585B20 => unported;
    /// `CPad::StopPadsShaking()` (0x541D70)
    pub fn pad_stop_pads_shaking() = cdecl 0x541D70 => unported;
    /// `CAudioEngine::GetCurrentRadioStationID()` (0x507040, thiscall)
    pub fn audio_get_current_radio_station_id(this: GPtr<u8>) -> u8 = thiscall 0x507040 => unported;
    /// `C_PcSave::PopulateSlotInfo()` (0x619140, thiscall)
    pub fn pc_save_populate_slot_info(this: GPtr<u8>) = thiscall 0x619140 => unported;
    /// `CTimer::StartUserPause()` (0x561AF0)
    pub fn timer_start_user_pause() = cdecl 0x561AF0 => unported;
    /// `CMenuManager::SwapTexturesRound(bool)` (0x5730A0, thiscall)
    pub fn swap_textures_round(this: GPtr<u8>, b: u32) = thiscall 0x5730A0 => swap_textures_round_port;
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    pub fn find_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => sa_stream::txdstore::find_txd_slot_name;
    /// `CTxdStore::RemoveTxd(int32)` (0x731E90)
    pub fn remove_txd(slot: i32) = cdecl 0x731E90 => unported;
    /// `CSprite2d::Delete()` (0x727240, thiscall)
    pub fn sprite2d_delete(this: GPtr<u8>) = thiscall 0x727240 => unported;
}

/// 1100.0 (0x864F94), 640.0 (0x85951C), 448.0 (0x8652B0)
const K_1100: f32 = f32::from_bits(0x4489_8000);
const K_640: f32 = f32::from_bits(0x4420_0000);
const K_448: f32 = f32::from_bits(0x43E0_0000);

/// `CMenuManager::Initialise()` (0x5744D0, thiscall): pad 0, +0x1B1C / item / +0x15F cleared; a pending
/// video change (+0x1ADE) applies multisampling (+0xC8) and the video mode (+0xD4); the mouse is
/// centred; map zoom 1100 (+0x64), +0xB8 cleared, +0x8 = 3, and the map origin (+0x68, +0x6C) pulled
/// in so that `origin -/+ 1100` stays inside (0, 640) x (0, 448); the radar map centred on the player,
/// pad shaking stopped; outside the pause menu (+0xF4 clear) the start screen 0x2A and its state;
/// the current radio station (+0x52), no script menu (+0x30 = -99), the directory reset, save slots
/// populated, and (tail jump) `CTimer::StartUserPause`.
#[hook(addr = 0x5744D0, conv = thiscall, name = "CMenuManager::Initialise", expect = "83 EC 08 53 33 DB 56 8B F1")]
pub fn initialise(this: GPtr<u8>) {
    let pending = b8(this, 0x1ADE) != 0;
    wr::<u8>(this, 0xEA, 0);
    wr::<u32>(this, 0x1B1C, 0);
    wr::<u32>(this, 0x54, 0);
    wr::<u8>(this, 0x15F, 0);
    if pending {
        rw_d3d9_change_multisampling_levels(rd(this, 0xC8));
        set_video_mode(rd(this, 0xD4));
        wr::<u8>(this, 0x1ADE, 0);
    }
    centre_mouse_pointer_call(this);
    let y1 = x(rd(this, 0x6C)).fsub_m(K_1100);
    wr::<u8>(this, 0xB8, 0);
    let y2 = x(rd(this, 0x6C)).fadd_m(K_1100);
    wr::<u8>(this, 0x8, 3);
    wr::<u32>(this, 0x64, 0x4489_8000);
    let y2 = y2.to_f32();
    let x1 = x(rd(this, 0x68)).fsub_m(K_1100);
    let x2 = x(rd(this, 0x68)).fadd_m(K_1100).to_f32();
    if !x1.fcom(x(K_ZERO)).test_any(0x41) {
        wr(this, 0x68, x(rd(this, 0x68)).fsub(x1).to_f32());
    }
    if !x(x2).fcom(x(K_640)).test_parity(0x05) {
        wr(this, 0x68, x(K_640).fsub_m(x2).fadd_m(rd(this, 0x68)).to_f32());
    }
    if !y1.fcom(x(K_ZERO)).test_any(0x41) {
        wr(this, 0x6C, x(rd(this, 0x6C)).fsub(y1).to_f32());
    }
    if !x(y2).fcom(x(K_448)).test_parity(0x05) {
        wr(this, 0x6C, x(K_448).fsub_m(y2).fadd_m(rd(this, 0x6C)).to_f32());
    }
    radar_set_map_centre_to_player_coords();
    pad_stop_pads_shaking();
    if b8(this, 0xF4) == 0 {
        wr::<u8>(this, 0x15D, 0x2A);
        wr::<u8>(this, 0x78, 1);
        wr::<u32>(this, 0x1AF8, 0);
        wr::<u32>(this, 0x1AFC, 0);
        wr::<u8>(this, 0xB8, 0);
    }
    let st = audio_get_current_radio_station_id(g(AUDIO_ENGINE));
    wr(this, 0x52, st);
    wr::<u8>(this, 0x30, 0x9D);
    file_set_dir(g(S_EMPTY));
    pc_save_populate_slot_info(g(0xC17034));
    timer_start_user_pause();
}

/// `CMenuManager::UnloadTextures()` (0x574630, thiscall): when loaded (+0x15C): `SwapTexturesRound(0)`,
/// then per txd ("frontend1": 13 sprites at +0xF8, "frontend_pc": 2 at +0x154, "frontend2": 8 at
/// +0x12C) the slot is found first, the sprites deleted, the txd removed; +0x15C cleared.
#[hook(addr = 0x574630, conv = thiscall, name = "CMenuManager::UnloadTextures", expect = "56 8B F1 8A 86 5C 01 00 00")]
pub fn unload_textures(this: GPtr<u8>) {
    if b8(this, 0x15C) == 0 {
        return;
    }
    swap_textures_round(this, 0);
    for (name, off, n) in [(0x86520C, 0xF8, 13), (0x8651EC, 0x154, 2), (0x8651C8, 0x12C, 8)] {
        let slot = find_txd_slot(g(name));
        for k in 0..n {
            sprite2d_delete(this.byte_add(off + k * 4));
        }
        remove_txd(slot);
    }
    wr::<u8>(this, 0x15C, 0);
}

orig! {
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// CRT `eh vector destructor iterator` (0x821E02, stdcall)
    pub fn eh_vector_dtor(arr: GPtr<u8>, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
}

/// `CSprite2d::CSprite2d()` (0x727230) / `~CSprite2d()` (0x7281E0), passed to the CRT iterators.
const SPRITE2D_CTOR: u32 = 0x727230;
const SPRITE2D_DTOR: u32 = 0x7281E0;

/// `CMenuManager::CMenuManager()` (0x574350, thiscall; the SEH frame of the original is not modelled):
/// the 25 sprites at +0xF8 constructed, pad / video-change / +0x1B0A / +0x1B0B / +0xDC cleared, default
/// preferences for screens 0x1B and 0x24, the input state, mouse position = +0xE0 / +0xE4, +0xE9 set,
/// defaults for screens 3 and 4, a random radio station 1..13, the language and menu flags cleared,
/// +0x4 = 150, +0 = 1, +0x38 = -1, +0x20 = 1, map zoom 300 at (320, 224). Returns `this`.
#[hook(addr = 0x574350, conv = thiscall, name = "CMenuManager::Constructor", expect = "6A FF 68 18 CE 83 00")]
pub fn constructor(this: GPtr<u8>) -> GPtr<u8> {
    eh_vector_ctor(this.byte_add(0xF8), 4, 0x19, SPRITE2D_CTOR, SPRITE2D_DTOR);
    wr::<u8>(this, 0xEA, 0);
    wr::<u8>(this, 0x1ADE, 0);
    wr::<u8>(this, 0x1B0B, 0);
    wr::<u8>(this, 0x1B0A, 0);
    wr::<u32>(this, 0xDC, 0);
    set_default_preferences_call(this, 0x1B);
    set_default_preferences_call(this, 0x24);
    wr::<u32>(this, 0xEC, 0);
    wr::<u32>(this, 0xF0, 0);
    wr::<u8>(this, 0x1AE8, 0);
    wr::<u32>(this, 0x7C, 9);
    wr::<u32>(this, 0xAC, 0);
    wr::<u8>(this, 0xD0, 0);
    g::<u8>(0xB6EC2E).write(1);
    let mx: u32 = rd(this, 0xE0);
    let my: u32 = rd(this, 0xE4);
    wr(this, 0xBC, mx);
    wr::<u32>(this, 0x90, 0);
    wr::<u8>(this, 0xE9, 1);
    wr(this, 0xC0, my);
    wr::<u32>(this, 0x1AF8, 0);
    wr::<u32>(this, 0x1AFC, 0);
    wr::<u8>(this, 0xB8, 0);
    wr::<u32>(this, 0x1B00, 0x10);
    wr::<u32>(this, 0x2C, 0);
    wr::<u8>(this, 0x34, 0);
    set_default_preferences_call(this, 3);
    set_default_preferences_call(this, 4);
    let r = audio_random_in_range(1, 0xD);
    wr(this, 0x52, r as u8);
    for o in [0x5A, 0x5B, 0xEB, 0x84] {
        wr::<u8>(this, o, 0);
    }
    wr::<u32>(this, 0x80, 0);
    wr::<u8>(this, 0x85, 0);
    wr::<u32>(this, 0x88, 0);
    for o in [0x31, 0x32, 0x33, 0x5C, 0x5F, 0x60, 0xF4] {
        wr::<u8>(this, o, 0);
    }
    wr::<u32>(this, 0x4, 0x4316_0000);
    wr::<u8>(this, 0, 1);
    wr::<u32>(this, 0x38, 0xFFFF_FFFF);
    wr::<u8>(this, 0x20, 1);
    wr::<u32>(this, 0x64, 0x4396_0000);
    wr::<u32>(this, 0x68, 0x43A0_0000);
    wr::<u32>(this, 0x6C, 0x4360_0000);
    this
}

/// `CMenuManager::~CMenuManager()` (0x579440, thiscall; SEH frame not modelled): `UnloadTextures`, then
/// the 25 sprites at +0xF8 destroyed.
#[hook(addr = 0x579440, conv = thiscall, name = "CMenuManager::Destructor", expect = "6A FF 68 48 CE 83 00")]
pub fn destructor(this: GPtr<u8>) {
    unload_textures_call(this);
    eh_vector_dtor(this.byte_add(0xF8), 4, 0x19, SPRITE2D_DTOR);
}

/// `CMenuManager::CheckSliderMovement` constants: 24.19 (0x865258), 384.0 (0x865254), the draw
/// distance step 0.0546874963 (0x865250), 1.8 (0x859BB4), 0.925 (0x86524C), 0.005 (0x8651A8), 1/15
/// (0x863E0C).
const K_BRIGHT_STEP: f32 = f32::from_bits(0x41C1_851F);
const K_384: f32 = f32::from_bits(0x43C0_0000);
const K_DIST_STEP: f32 = f32::from_bits(0x3D5F_FFFF);
const K_1_8: f32 = f32::from_bits(0x3FE6_6666);
const K_0_925: f32 = f32::from_bits(0x3F6C_CCCD);
const K_0_005: f32 = f32::from_bits(0x3BA3_D70A);
const K_1_15: f32 = f32::from_bits(0x3D88_8889);
/// `CCamera::m_fMouseAccelHorzntl` (0xB6EC1C), the minimum mouse sensitivity (0xBA672C)
const MOUSE_ACCEL: usize = 0xB6EC1C;
const MOUSE_ACCEL_MIN: usize = 0xBA672C;

/// The volume sliders: `v = (int8)(v + (lr << 2))`, negative add result -> 0, `>= 64` -> 64.
fn slide_volume(this: GPtr<u8>, off: isize, lr: u32) -> u32 {
    let r = b8(this, off).wrapping_add((lr as u8).wrapping_shl(2));
    wr(this, off, r);
    let mut v: i32 = if (r as i8) < 0 { 0 } else { i32::from(r as i8) };
    wr(this, off, v as u8);
    if v as u8 as i8 >= 0x40 {
        v = 0x40;
    }
    wr(this, off, v as u8);
    v as u32
}

/// `CMenuManager::CheckSliderMovement(int8 lr)` (0x573440, thiscall): by the current item's action
/// (27 brightness, 28 radio volume, 29 SFX volume, 61 draw distance, 62 mouse sensitivity; others
/// return without saving) as in `CMenuManager.deviations.md`: brightness `_ftol(lr * 24.19 +
/// brightness)`, `max(0)`, `min(384)` (x87), `SetGamma(v / 512, false)`; volumes in steps of 4 clamped
/// to 0..64; draw distance `+/- 0.0546874963` clamped to 0.925..1.8 (NaN kept) into `ms_lodDistScale`;
/// mouse `lr * 0.005 * (1/15) + h`, at least the minimum, then 0.005 unless below it. Then
/// `SaveSettings`.
#[hook(addr = 0x573440, conv = thiscall, name = "CMenuManager::CheckSliderMovement", expect = "56 8B F1 0F BE 8E 5D 01 00 00")]
pub fn check_slider_movement(this: GPtr<u8>, lr: u32) {
    let cur = i32::from(b8(this, 0x15D) as i8);
    let item: i32 = rd(this, 0x54);
    let action = rd::<u8>(screen(cur), (0xA + item.wrapping_mul(0x12)) as isize);
    let lr8 = lr as u8 as i8;
    match action {
        27 => {
            let v = crate::common::ftol32(
                xi(i32::from(lr8))
                    .fmul_m(K_BRIGHT_STEP)
                    .fadd(xi(rd(this, 0x3C))),
            );
            wr(this, 0x3C, v);
            let v = if v < 0 { 0 } else { v };
            wr(this, 0x3C, v);
            let f = if xi(v).fcom(x(K_384)).test_parity(0x05) {
                x(K_384)
            } else {
                xi(v)
            };
            let v = crate::common::ftol32(f);
            let gamma = xi(v).fmul_m(K_INV_512).to_f32();
            wr(this, 0x3C, v);
            gamma_set(g(0xC92134), gamma.to_bits(), 0);
        }
        28 => {
            let v = slide_volume(this, 0x50, lr);
            audio_set_music_volume(g(AUDIO_ENGINE), v);
        }
        29 => {
            let v = slide_volume(this, 0x4F, lr);
            audio_set_effects_volume(g(AUDIO_ENGINE), v);
        }
        61 => {
            let d = x(rd(this, 0x40));
            let d = if lr8 > 0 { d.fadd_m(K_DIST_STEP) } else { d.fsub_m(K_DIST_STEP) };
            wr(this, 0x40, d.to_f32());
            if !x(rd(this, 0x40)).fcom(x(K_1_8)).test_any(0x41) {
                wr::<u32>(this, 0x40, 0x3FE6_6666);
            }
            if !x(rd(this, 0x40)).fcom(x(K_0_925)).test_parity(0x05) {
                wr::<u32>(this, 0x40, 0x3F6C_CCCD);
            }
            g::<u32>(0x8CD800).write(rd(this, 0x40));
        }
        62 => {
            let h = xi(i32::from(lr8))
                .fmul_m(K_0_005)
                .fmul_m(K_1_15)
                .fadd_m(g::<f32>(MOUSE_ACCEL).read());
            g::<f32>(MOUSE_ACCEL).write(h.to_f32());
            let min = g::<f32>(MOUSE_ACCEL_MIN).read();
            if !x(min).fcom(x(g::<f32>(MOUSE_ACCEL).read())).test_any(0x41) {
                g::<u32>(MOUSE_ACCEL).write(g::<u32>(MOUSE_ACCEL_MIN).read());
            }
            if x(g::<f32>(MOUSE_ACCEL).read()).fcom(x(K_0_005)).test_parity(0x05) {
                g::<u32>(MOUSE_ACCEL).write(K_0_005.to_bits());
            }
        }
        _ => return,
    }
    save_settings_call(this);
}

orig! {
    /// `CStreaming::MakeSpaceFor(int32)` (0x40E120)
    pub fn streaming_make_space_for(size: u32) = cdecl 0x40E120 => unported;
    /// `CStreaming::ImGonnaUseStreamingMemory()` (0x407BE0)
    pub fn streaming_im_gonna_use_streaming_memory() = cdecl 0x407BE0 => unported;
    /// `CStreaming::IHaveUsedStreamingMemory()` (0x407BF0)
    pub fn streaming_i_have_used_streaming_memory() = cdecl 0x407BF0 => unported;
    /// `CGame::TidyUpMemory(bool, bool)` (0x53C500)
    pub fn game_tidy_up_memory(a: u32, b: u32) = cdecl 0x53C500 => unported;
    /// `CTxdStore::AddTxdSlot(const char*)` (0x731C80)
    pub fn add_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731C80 => sa_stream::txdstore::add_txd_slot;
    /// `CTxdStore::LoadTxd(int32, const char*)` (0x7320B0)
    pub fn load_txd(slot: i32, file: GPtr<u8>) -> u8 = cdecl 0x7320B0 => sa_stream::txdstore::load_txd_filename;
    /// `CTxdStore::AddRef(int32)` (0x731A00)
    pub fn txd_add_ref(slot: i32) = cdecl 0x731A00 => sa_stream::txdstore::add_ref;
    /// `CTxdStore::PushCurrentTxd()` (0x7316A0)
    pub fn push_current_txd() = cdecl 0x7316A0 => sa_stream::txdstore::push_current_txd;
    /// `CTxdStore::SetCurrentTxd(int32)` (0x7319C0)
    pub fn set_current_txd(slot: i32) = cdecl 0x7319C0 => sa_stream::txdstore::set_current_txd;
    /// `CTxdStore::PopCurrentTxd()` (0x7316B0)
    pub fn pop_current_txd() = cdecl 0x7316B0 => sa_stream::txdstore::pop_current_txd;
    /// `CSprite2d::SetTexture(const char*, const char*)` (0x7272B0, thiscall)
    pub fn sprite2d_set_texture(this: GPtr<u8>, name: GPtr<u8>, mask: GPtr<u8>) = thiscall 0x7272B0 => unported;
    /// `CSprite2d::SetAddressing(RwTextureAddressMode)` (0x727320, thiscall)
    pub fn sprite2d_set_addressing(this: GPtr<u8>, mode: u32) = thiscall 0x727320 => unported;
}

/// One frontend txd load of `LoadAllTextures` / `SwapTexturesRound`: `MakeSpaceFor(size)`,
/// `ImGonnaUseStreamingMemory`, optionally `TidyUpMemory(false, true)`, push the current txd, the slot
/// (found, else added), `LoadTxd(slot, file)`, `AddRef`, `SetCurrentTxd`, then per (name, mask) pair of
/// the table `SetTexture` + `SetAddressing(3)` on consecutive sprites from `this + sprites`; pop the txd,
/// `IHaveUsedStreamingMemory`.
fn load_front_end_txd(this: GPtr<u8>, size: u32, tidy: bool, name: usize, file: usize, table: (usize, usize), sprites: isize) {
    streaming_make_space_for(size);
    streaming_im_gonna_use_streaming_memory();
    if tidy {
        game_tidy_up_memory(0, 1);
    }
    push_current_txd();
    let mut slot = find_txd_slot(g(name));
    if slot == -1 {
        slot = add_txd_slot(g(name));
    }
    load_txd(slot, g(file));
    txd_add_ref(slot);
    set_current_txd(slot);
    let mut e = table.0;
    let mut s = this.byte_add(sprites);
    while e < table.1 {
        sprite2d_set_texture(s, g::<GPtr<u8>>(e).read(), g::<GPtr<u8>>(e + 4).read());
        sprite2d_set_addressing(s, 3);
        e += 8;
        s = s.byte_add(4);
    }
    pop_current_txd();
    streaming_i_have_used_streaming_memory();
}

/// `CMenuManager::LoadAllTextures()` (0x572EC0, thiscall): unless loaded (+0x15C): timer stopped,
/// "frontend1" (0x21000, `MODELS\FRONTEN1.TXD`, 13 sprites at +0xF8, tidy), "frontend_pc" (0x3000, 2
/// at +0x154, tidy), "frontend2" (0x4C000, 8 at +0x12C, no tidy); loaded flag; (tail) `CTimer::Update`.
#[hook(addr = 0x572EC0, conv = thiscall, name = "CMenuManager::LoadAllTextures", expect = "53 8B D9 8A 83 5C 01 00 00")]
pub fn load_all_textures_port(this: GPtr<u8>) {
    if b8(this, 0x15C) != 0 {
        return;
    }
    timer_stop();
    load_front_end_txd(this, 0x21000, true, 0x86520C, 0x8651F8, (0x8CDF28, 0x8CDF90), 0xF8);
    load_front_end_txd(this, 0x3000, true, 0x8651EC, 0x8651D4, (0x8CDFE0, 0x8CDFF0), 0x154);
    load_front_end_txd(this, 0x4C000, false, 0x8651C8, 0x8651B4, (0x8CDF90, 0x8CDFD0), 0x12C);
    wr::<u8>(this, 0x15C, 1);
    timer_update();
}

/// `CMenuManager::SwapTexturesRound(bool slot)` (0x5730A0, thiscall): nothing when 0xBA8268 already
/// holds `slot`; stored. `false` with a "frontend3" slot index other than 0 (tested against 0, not -1):
/// its 2 sprites at +0x14C deleted, the txd removed, "frontend2" loaded (0x4C000, 8 at +0x12C).
/// Otherwise "frontend2"'s 8 sprites deleted and removed, "frontend3" loaded (0x6800, 2 at +0x14C).
#[hook(addr = 0x5730A0, conv = thiscall, name = "CMenuManager::SwapTexturesRound", expect = "8A 44 24 04 53 8B D9")]
pub fn swap_textures_round_port(this: GPtr<u8>, slot_arg: u32) {
    let s = slot_arg as u8;
    if g::<u8>(0xBA8268).read() == s {
        return;
    }
    g::<u8>(0xBA8268).write(s);
    if s == 0 && find_txd_slot(g(0x865240)) != 0 {
        let slot = find_txd_slot(g(0x865240));
        for k in 0..2 {
            sprite2d_delete(this.byte_add(0x14C + k * 4));
        }
        remove_txd(slot);
        load_front_end_txd(this, 0x4C000, false, 0x8651C8, 0x86522C, (0x8CDF90, 0x8CDFD0), 0x12C);
        return;
    }
    let slot = find_txd_slot(g(0x8651C8));
    for k in 0..8 {
        sprite2d_delete(this.byte_add(0x12C + k * 4));
    }
    remove_txd(slot);
    load_front_end_txd(this, 0x6800, false, 0x865240, 0x865218, (0x8CDFD0, 0x8CDFE0), 0x14C);
}

orig! {
    /// `CMenuManager::ProcessPCMenuOptions(int8 lr, bool accept)` (0x57CD50, thiscall)
    pub fn process_pc_menu_options(this: GPtr<u8>, lr: u32, accept: u32) -> u8 = thiscall 0x57CD50 => process_pc_menu_options_port;
    /// `CMenuManager::ProcessMissionPackNewGame()` (0x57D520, thiscall)
    pub fn process_mission_pack_new_game(this: GPtr<u8>) = thiscall 0x57D520 => process_mission_pack_new_game_port;
    /// `CMenuManager::ScrollRadioStations(int8)` (0x573A00, thiscall)
    pub fn scroll_radio_stations_call(this: GPtr<u8>, dir: u32) = thiscall 0x573A00 => scroll_radio_stations;
}

/// `PcSaveHelper` slot states (0xC16EBC, int32 per slot)
const SAVE_SLOT_STATE: usize = 0xC16EBC;

/// `byte = !byte` (`test; sete`).
fn toggle(p: GPtr<u8>, off: isize) -> u8 {
    let v = u8::from(b8(p, off) == 0);
    wr(p, off, v);
    v
}

/// `CMenuManager::ProcessMenuOptions(int8 lr, bool* goBack, bool accept)` (0x576FE0, thiscall): the PC
/// options first (`ProcessPCMenuOptions(lr, accept)`, nothing more when it handled the action), then by
/// the current item's action (byte table 0x5773A4): 2 back (`*goBack = 1`), 3..5 the item's target
/// screen, 10 mission pack new game, 11 mission pack (+0x160 = `m_MissionPacks[item - 2].m_Id`
/// unchecked, screen 8), 12 / 14 start a game (mission pack +0x160 / none, the latter sets +0x32), 13
/// load slot (screens 0xB / 0xC by slot state), 15 sets +0x32 only, 18 delete slot (screen 0x11), 21 the
/// +0 / +4 toggle (150 / 0 / 20), 22 0xB73402, 23 +0x31, 25 +0x44, 26 +0x4B, 30 bass, 31 auto-retune,
/// 32 radio stations, 33 +0x4A, 34 radar mode (+0x24 += lr, wraps 2 / 0), 35 +0x21, 36 language
/// (0..4 cyclic, reload). Toggles save the settings except 22, 23 and 33.
#[hook(addr = 0x576FE0, conv = thiscall, name = "CMenuManager::ProcessMenuOptions", expect = "8B 44 24 0C 53 8B 5C 24 08")]
pub fn process_menu_options(this: GPtr<u8>, lr: u32, go_back: GPtr<u8>, accept: u32) {
    if process_pc_menu_options(this, lr, accept) != 0 {
        return;
    }
    let cur = i32::from(b8(this, 0x15D) as i8);
    let item: i32 = rd(this, 0x54);
    let it = screen(cur).byte_add((item.wrapping_mul(0x12)) as isize);
    let lr8 = lr as u8 as i8;
    match b8(it, 0xA) {
        2 => wr::<u8>(go_back, 0, 1),
        3..=5 => switch_to_new_screen_call(this, i32::from(b8(it, 0x14) as i8) as u32),
        10 => process_mission_pack_new_game(this),
        11 => {
            let id = rd::<u8>(this, (item.wrapping_mul(0x105) - 0xA9) as isize);
            wr(this, 0x160, id);
            switch_to_new_screen_call(this, 8);
        }
        12 => {
            g::<u8>(MISSION_PACK).write(b8(this, 0x160));
            do_settings_before_starting_a_game_call(this);
        }
        13 => {
            let ty = b8(it, 0x13) as i8;
            if !(1..=8).contains(&ty) {
                return;
            }
            let slot = (item as u8).wrapping_sub(1);
            wr(this, 0x15F, slot);
            let state = g::<i32>(SAVE_SLOT_STATE.wrapping_add((i32::from(slot as i8) * 4) as u32 as usize)).read();
            match cur {
                9 if state != 1 && state != 2 => switch_to_new_screen_call(this, 0xB),
                10 if state != 1 => switch_to_new_screen_call(this, 0xC),
                _ => {}
            }
        }
        14 => {
            g::<u8>(MISSION_PACK).write(0);
            do_settings_before_starting_a_game_call(this);
            wr::<u8>(this, 0x32, 1);
        }
        15 => wr::<u8>(this, 0x32, 1),
        18 => {
            let ty = b8(it, 0x13) as i8;
            if (1..=8).contains(&ty) {
                wr(this, 0x15F, (item as u8).wrapping_sub(1));
                switch_to_new_screen_call(this, 0x11);
            }
        }
        21 => {
            // The `jp` branch of `fcomp 0.0; test ah, 0x44`: +4 != 0 (or unordered).
            let nz = x(rd(this, 4)).fcom(x(K_ZERO)).test_parity(0x44);
            let on = b8(this, 0) != 0;
            let flag = u8::from(lr8 == 1);
            if on == (lr8 == 1) {
                // al != 0 with lr == 1, or al == 0 with lr != 1: 0 -> 150, 150 -> 20.
                if !nz {
                    wr::<u32>(this, 4, 0x4316_0000);
                } else if rd::<u32>(this, 4) == 0x4316_0000 {
                    wr::<u32>(this, 4, 0x41A0_0000);
                }
            } else if nz {
                wr::<u32>(this, 4, 0);
            } else {
                wr::<u32>(this, 4, 0x4316_0000);
            }
            wr(this, 0, flag);
        }
        22 => {
            toggle(g(0xB73402), 0);
        }
        23 => {
            toggle(this, 0x31);
        }
        25 => {
            toggle(this, 0x44);
            save_settings_call(this);
        }
        26 => {
            toggle(this, 0x4B);
            save_settings_call(this);
        }
        30 => {
            let v = toggle(this, 0x51);
            audio_set_bass_enhance(g(AUDIO_ENGINE), u32::from(v));
            save_settings_call(this);
        }
        31 => {
            let v = toggle(this, 0x4D);
            audio_set_radio_auto_retune(g(AUDIO_ENGINE), u32::from(v));
            save_settings_call(this);
        }
        32 => scroll_radio_stations_call(this, lr),
        33 => {
            toggle(this, 0x4A);
        }
        34 => {
            let v = rd::<i32>(this, 0x24).wrapping_add(i32::from(lr8));
            wr(this, 0x24, v);
            if v < 0 {
                wr::<i32>(this, 0x24, 2);
            }
            if rd::<i32>(this, 0x24) > 2 {
                wr::<i32>(this, 0x24, 0);
            }
            save_settings_call(this);
        }
        35 => {
            toggle(this, 0x21);
            save_settings_call(this);
        }
        36 => {
            let l = b8(this, 0x84);
            let n = if lr8 > 0 {
                if l == 4 { 0 } else { l.wrapping_add(1) }
            } else if l == 0 {
                4
            } else {
                l.wrapping_sub(1)
            };
            wr(this, 0x84, n);
            wr::<u8>(this, 0x85, 0x9D);
            wr::<u8>(this, 0xEB, 1);
            initialise_changed_language_settings_call(this, 0);
            save_settings_call(this);
        }
        _ => {}
    }
}

orig! {
    /// `RwTextureSetMipmapping(bool)` (0x7F3530)
    pub fn rw_texture_set_mipmapping(on: u32) -> i32 = cdecl 0x7F3530 => unported;
    /// `RwD3D9EngineGetMaxMultiSamplingLevels()` (0x7F84E0)
    pub fn rw_d3d9_max_multisampling_levels() -> u32 = cdecl 0x7F84E0 => unported;
    /// `VideoMode::GetVideoModeList()` (0x745AF0)
    pub fn get_video_mode_list() -> GPtr<u8> = cdecl 0x745AF0 => unported;
    /// `GetNumVideoModes` thunk (0x745A70, `jmp 0x7F2CC0`)
    pub fn get_num_video_modes() -> i32 = cdecl 0x745A70 => unported;
    /// `CPostEffects::DoScreenModeDependentInitializations()` (0x7046D0)
    pub fn post_effects_screen_mode_init() = cdecl 0x7046D0 => unported;
}

/// `list[mode] == 0` (an unused video mode) for the mode at +0xD8.
fn video_mode_unused(list: GPtr<u8>, this: GPtr<u8>) -> bool {
    let m: i32 = rd(this, 0xD8);
    rd::<i32>(list, (m as isize).wrapping_mul(4)) == 0
}

/// `CMenuManager::ProcessPCMenuOptions(int8 lr, bool accept)` (0x57CD50, thiscall): by the current
/// item's action (byte table 0x57D4DC, actions 6..65); handled actions return AL = 1, others AL = 0.
/// Toggles (with `SaveSettings`): 24 +0x4C, 43 mipmapping +0xC4 (only with +0xE9), 46 0xBA6745,
/// 47..52 +0xB1..+0xB6, 59 / 60 0xC1CC02 / 0xC1CC03 (only with +0xD0 == 0), 64 +0xC5, 65 +0xE8.
/// 6 sets +0x1AEB; 7 controller screen 0x28 when +0xD0 == 1 else 0x27; 8 / 9 +0xB7 = 0 / 1, screen
/// 0x26, +0x90 = 0; 34 radar mode +1 (lr ignored, > 2 -> 0); 37..41 language 0 / 1 / 2 / 3 / 4 (reload
/// when it changes); 42 FX quality 3 -> 0 -> 1 -> 2 -> 3 (lr ignored); 44 anti-aliasing (+0xCC stepped
/// 1..4 / max levels, applied on accept); 45 redefine controls; 53 screen 0x2B; 54 +0x32; 55 back;
/// 56 video mode (unbounded scans of the mode list, applied on accept); 57 restore defaults of the
/// screen's +0x38 page; 58 control type; 63 user tracks mode (+0xB0 += lr, negative -> 2, > 2 -> 0).
#[hook(addr = 0x57CD50, conv = thiscall, name = "CMenuManager::ProcessPCMenuOptions", expect = "53 56 8B F1 0F BE 86 5D 01 00 00")]
pub fn process_pc_menu_options_port(this: GPtr<u8>, lr: u32, accept: u32) -> u8 {
    let cur = i32::from(b8(this, 0x15D) as i8);
    let item: i32 = rd(this, 0x54);
    let sc = screen(cur);
    let action = rd::<u8>(sc, (0xA + item.wrapping_mul(0x12)) as isize);
    let lr8 = lr as u8 as i8;
    let accept = accept as u8 != 0;
    let save = || save_settings_call(this);
    let set_language = |l: u8| {
        if b8(this, 0x84) == l {
            return;
        }
        wr(this, 0x84, l);
        wr::<u8>(this, 0xEB, 1);
        initialise_changed_language_settings_call(this, 0);
        save();
    };
    match action {
        6 => wr::<u8>(this, 0x1AEB, 1),
        7 => switch_to_new_screen_call(this, if b8(this, 0xD0) == 1 { 0x28 } else { 0x27 }),
        8 | 9 => {
            wr::<u8>(this, 0xB7, action - 8);
            switch_to_new_screen_call(this, 0x26);
            wr::<u32>(this, 0x90, 0);
        }
        24 => {
            toggle(this, 0x4C);
            save();
        }
        34 => {
            let v = rd::<i32>(this, 0x24).wrapping_add(1);
            wr(this, 0x24, v);
            if v > 2 {
                wr::<i32>(this, 0x24, 0);
            }
            save();
        }
        37 => set_language(0),
        38 => set_language(1),
        39 => set_language(2),
        40 => set_language(3),
        41 => set_language(4),
        42 => {
            let fx = g(G_FX);
            if fx_get_quality(fx) == 3 {
                fx_set_quality(fx, 0);
            } else if fx_get_quality(fx) == 2 {
                fx_set_quality(fx, 3);
            } else if fx_get_quality(fx) == 1 {
                fx_set_quality(fx, 2);
            } else if fx_get_quality(fx) == 0 {
                fx_set_quality(fx, 1);
            }
            save();
        }
        43 => {
            if b8(this, 0xE9) != 0 {
                let v = toggle(this, 0xC4);
                rw_texture_set_mipmapping(u32::from(v));
                save();
            }
        }
        44 => {
            if accept {
                let v: u32 = rd(this, 0xCC);
                if v != rd::<u32>(this, 0xC8) {
                    wr(this, 0xC8, v);
                    rw_d3d9_change_multisampling_levels(v);
                    set_video_mode(rd(this, 0xD4));
                    save();
                }
            } else if lr8 > 0 {
                let v = rd::<i32>(this, 0xCC).wrapping_add(1);
                wr(this, 0xCC, v);
                if v > 4 || rd::<u32>(this, 0xCC) > rw_d3d9_max_multisampling_levels() {
                    wr::<u32>(this, 0xCC, 1);
                }
            } else {
                let v = rd::<i32>(this, 0xCC).wrapping_sub(1);
                wr(this, 0xCC, v);
                if v < 1 {
                    if rw_d3d9_max_multisampling_levels() > 4 {
                        wr::<u32>(this, 0xCC, 4);
                    } else {
                        wr(this, 0xCC, rw_d3d9_max_multisampling_levels());
                    }
                }
            }
        }
        45 => {
            wr::<u8>(this, 0x1B14, 1);
            wr::<u8>(this, 0x1B09, 1);
            wr::<u8>(this, 0x1B08, 1);
            wr(this, 0x1B0C, item);
            wr(this, 0xF0, this.byte_add(0x38));
        }
        46 => {
            toggle(g(0xBA6745), 0);
            save();
        }
        47..=52 => {
            toggle(this, 0xB1 + isize::from(action - 47));
            save();
        }
        53 => switch_to_new_screen_call(this, 0x2B),
        54 => wr::<u8>(this, 0x32, 1),
        55 => switch_to_new_screen_call(this, 0xFFFF_FFFE),
        56 => {
            if accept {
                let m: u32 = rd(this, 0xD8);
                if m != rd::<u32>(this, 0xD4) {
                    wr(this, 0xD4, m);
                    set_video_mode(m);
                    centre_mouse_pointer_call(this);
                    wr::<u8>(this, 0xB8, 1);
                    wr::<u32>(this, 0x54, 5);
                    save();
                    post_effects_screen_mode_init();
                }
            } else {
                let list = get_video_mode_list();
                let step = |d: i32| wr(this, 0xD8, rd::<i32>(this, 0xD8).wrapping_add(d));
                if lr8 > 0 {
                    step(1);
                    while video_mode_unused(list, this) {
                        step(1);
                    }
                    if rd::<i32>(this, 0xD8) >= get_num_video_modes() {
                        wr::<i32>(this, 0xD8, 0);
                        while video_mode_unused(list, this) {
                            step(1);
                        }
                    }
                } else {
                    step(-1);
                    while video_mode_unused(list, this) {
                        step(-1);
                    }
                    if rd::<i32>(this, 0xD8) < 0 {
                        wr(this, 0xD8, get_num_video_modes().wrapping_sub(1));
                        while video_mode_unused(list, this) {
                            step(-1);
                        }
                    }
                }
            }
        }
        57 => {
            let page = || u32::from(b8(screen(i32::from(b8(this, 0x15D) as i8)), 0x38));
            set_default_preferences_call(this, page());
            match page() {
                0x1B => {
                    rw_d3d9_change_multisampling_levels(rd(this, 0xC8));
                    set_video_mode(rd(this, 0xD4));
                }
                0x24 => ccm_reinit_controls(g(CONTROLS_MANAGER)),
                _ => {}
            }
            save();
            switch_to_new_screen_call(this, page());
            wr::<u32>(this, 0x1B18, 2);
            wr::<u32>(this, 0x1AEC, 300);
        }
        58 => {
            if b8(this, 0xD0) == 1 {
                wr::<u8>(this, 0xD0, 0);
                g::<u8>(0xB6EC2E).write(1);
            } else {
                wr::<u8>(this, 0xD0, 1);
                g::<u8>(0xB6EC2E).write(0);
            }
            save();
        }
        59 | 60 => {
            if b8(this, 0xD0) == 0 {
                toggle(g(if action == 59 { 0xC1CC02 } else { 0xC1CC03 }), 0);
                save();
            }
        }
        63 => {
            let v = b8(this, 0xB0).wrapping_add(lr as u8);
            wr(this, 0xB0, v);
            if (v as i8) < 0 {
                wr::<u8>(this, 0xB0, 2);
            }
            if (b8(this, 0xB0) as i8) > 2 {
                wr::<u8>(this, 0xB0, 0);
            }
            save();
        }
        64 => {
            toggle(this, 0xC5);
            save();
        }
        65 => {
            toggle(this, 0xE8);
            save();
        }
        _ => return 0,
    }
    1
}

orig! {
    /// `CFileMgr::GetTotalSize(FILE*)` (0x5389E0)
    pub fn file_get_total_size(f: GPtr<u8>) -> i32 = cdecl 0x5389E0 => sa_core::filemgr::get_total_size;
    /// CRT `atol` (0x82258E)
    pub fn crt_atol(s: GPtr<u8>) -> i32 = cdecl 0x82258E => unported;
    /// CRT `sprintf(buf, fmt, a)` (0x821BB5)
    pub fn crt_sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, a: i32) -> i32 = cdecl 0x821BB5 => unported;
}

/// `CMenuManager::ProcessMissionPackNewGame()` (0x57D520, thiscall): as `CMenuManager.deviations.md`:
/// the 25 pack ids (+0x161, stride 0x105) cleared; for files `MPACK//MPACK%d//MPACK.DAT` 0..24 (user
/// directory set before each, through `gString`; a handle `<= 0` signed is skipped) a hand parser of
/// "id#name#" pairs (u8 position wrapping at 256, '\n' / '\r' ignored, 256-byte buffer with a u8
/// index) stores ids (`atol`) and names (`strcpy`) at a running index; then screen 7 when any file
/// opened, else screen 6 with "FESZ_MR" / "FESZ_QR" (mission pack selected or not) copied (8) into the
/// screen's first item text.
#[hook(addr = 0x57D520, conv = thiscall, name = "CMenuManager::ProcessMissionPackNewGame", expect = "81 EC 10 01 00 00 55 8B E9")]
pub fn process_mission_pack_new_game_port(this: GPtr<u8>) {
    let mut found = false;
    for k in 0..25 {
        wr::<u8>(this, 0x161 + k * 0x105, 0);
    }
    let mut pack: u8 = 0;
    let mut buf = [0u8; 0x100];
    let bp: GPtr<u8> = local(&mut buf);
    let gs: GPtr<u8> = g(0xB71670);
    // The read byte's stack slot keeps its value across short reads and files.
    let mut c = 0u8;
    for i in 0..25 {
        file_set_dir_my_documents();
        crt_sprintf1(gs, g(0x8656C0), i);
        let f = file_open(gs, g(0x859C6C));
        if (f.addr() as u32 as i32) <= 0 {
            continue;
        }
        found = true;
        let size = file_get_total_size(f);
        let mut len: u8 = 0;
        let mut is_id = true;
        let mut pos: u8 = 0;
        if size > 0 {
            loop {
                file_read(f, local(&mut c), 1);
                if c == b'#' {
                    if len != 0 {
                        wr::<u8>(bp, isize::from(len), 0);
                        if is_id {
                            let id = crt_atol(bp) as u8;
                            wr(this, isize::from(pack) * 0x105 + 0x161, id);
                        } else {
                            let dst = this.byte_add(isize::from(pack) * 0x105 + 0x162);
                            let mut j = 0isize;
                            loop {
                                let ch = b8(bp, j);
                                wr(dst, j, ch);
                                if ch == 0 {
                                    break;
                                }
                                j += 1;
                            }
                            pack = pack.wrapping_add(1);
                        }
                        is_id = !is_id;
                        len = 0;
                    }
                } else if c != b'\n' && c != b'\r' {
                    wr(bp, isize::from(len), c);
                    len = len.wrapping_add(1);
                }
                pos = pos.wrapping_add(1);
                if i32::from(pos) >= size {
                    break;
                }
            }
        }
        file_close(f);
    }
    if found {
        switch_to_new_screen_call(this, 7);
        return;
    }
    switch_to_new_screen_call(this, 6);
    let text = if g::<u8>(MISSION_PACK).read() != 0 { 0x8656B8 } else { 0x8656B0 };
    let sc = screen(i32::from(b8(this, 0x15D) as i8));
    crt_strncpy(sc.byte_add(0xB), g(text), 8);
}

orig! {
    /// `CMenuManager::StretchX(float)` (0x5733E0, thiscall)
    pub fn stretch_x_call_st0(this: GPtr<u8>, v: f32) -> f64 = thiscall 0x5733E0 => stretch_x_call_st0_rust;
    /// `CMenuManager::StretchY(float)` (0x573410, thiscall)
    pub fn stretch_y_call_st0(this: GPtr<u8>, v: f32) -> f64 = thiscall 0x573410 => stretch_y_call_st0_rust;
    /// `CFont::PrintStringFromBottom(float, float, const GxtChar*)` (0x71A820)
    pub fn font_print_string_from_bottom(px: f32, py: f32, text: GPtr<u8>) = cdecl 0x71A820 => crate::font_print::print_string_from_bottom;
}

/// `CMenuManager::DisplayHelperText(const char* text)` (0x57E240, thiscall; entry `push esi; mov esi,
/// ecx; nop; nop; jmp` into the `.text` chunk 0x40165E): scale `StretchX(0.4)` x `StretchY(0.5 on the
/// map screen, else 0.6)`, font 2, right aligned, no edge. A given key prints in white. Otherwise the
/// helper text +0x1B18 (2..6 fade by 2 every 10 ms of pause-mode time, 0xBA82A0, reset to none / 300 at
/// the end; alpha `min(fade, 255)`; 1 and none stay opaque): 1 FET_APP, 2 FET_HRD, 3 FET_RSO, 4 FEA_SCF,
/// 5 FEA_SCS, 6 FET_STS; none: by the current item's action 2 FEH_BPO, 5 / 7 / 8 / 9 FEH_JMP, 6 FEH_SNC,
/// 56 FET_MIG, else FEH_SSA on screen 0 or FET_MIG. Printed from the bottom at `(StretchX(610), H -
/// StretchY(10))` (`StretchY(2)` for the default text on the map screen).
#[hook(addr = 0x57E240, conv = thiscall, name = "CMenuManager::DisplayHelperText", expect = "56 8B F1 90 90 E9 14 34 E8 FF")]
pub fn display_helper_text(this: GPtr<u8>, key: GPtr<u8>) {
    let map = b8(this, 0x15D) == 5;
    let mut alpha: u32 = 0xFF;
    let h = stretch_y_call(this, f32::from_bits(if map { 0x3F00_0000 } else { 0x3F19_999A })).to_f32();
    let w = stretch_x_call(this, f32::from_bits(0x3ECC_CCCD)).to_f32();
    font_set_scale(w.to_bits(), h.to_bits());
    font_set_font_style(2);
    font_set_orientation(2);
    font_set_edge(0);
    let mut c = 0u32;
    let cp = local(&mut c);
    let mut offset = 0x4120_0000u32;
    let text;
    if !key.is_null() {
        text = text_get(g(THE_TEXT), key);
        font_set_color(rd(rgba_ctor(cp, 0xFF, 0xFF, 0xFF, 0xFF), 0));
    } else {
        let ht: i32 = rd(this, 0x1B18);
        if ht != 0 && ht != 1 {
            let now = g::<u32>(0xB7CB7C).read();
            if now.wrapping_sub(g::<u32>(0xBA82A0).read()) > 10 {
                g::<u32>(0xBA82A0).write(now);
                wr(this, 0x1AEC, rd::<i32>(this, 0x1AEC).wrapping_sub(2));
            }
            if rd::<i32>(this, 0x1AEC) < 1 {
                wr::<u32>(this, 0x1B18, 0);
                wr::<u32>(this, 0x1AEC, 300);
            }
            alpha = rd(this, 0x1AEC);
            if alpha as i32 > 0xFF {
                alpha = 0xFF;
            }
        }
        font_set_color(rd(rgba_ctor(cp, 0xFF, 0xFF, 0xFF, alpha), 0));
        let k = match rd::<i32>(this, 0x1B18) {
            1 => Some(0x8663E0),
            2 => Some(0x8663F8),
            3 => Some(0x8663F0),
            4 => Some(0x866400),
            5 => Some(0x866408),
            6 => Some(0x8663E8),
            _ => None,
        };
        text = match k {
            Some(k) => text_get(g(THE_TEXT), g(k)),
            None => {
                let cur = b8(this, 0x15D);
                let item: i32 = rd(this, 0x54);
                let action = rd::<u8>(screen(i32::from(cur as i8)), (0xA + item.wrapping_mul(0x12)) as isize);
                let k = match action {
                    2 => Some(0x8663C8),
                    5 | 7 | 8 | 9 => Some(0x8663D0),
                    6 => Some(0x8663D8),
                    56 => Some(0x8663C0),
                    _ => None,
                };
                match k {
                    Some(k) => text_get(g(THE_TEXT), g(k)),
                    None => {
                        let t = text_get(g(THE_TEXT), g(if cur == 0 { 0x8663B8 } else { 0x8663C0 }));
                        if b8(this, 0x15D) == 5 {
                            offset = 0x4000_0000;
                        }
                        t
                    }
                }
            }
        };
    }
    let hf = xi(g::<i32>(RS_MAX_HEIGHT).read()).to_f32();
    let py = stretch_y_call(this, f32::from_bits(offset)).fsubr_m(hf).to_f32();
    let px = stretch_x_call(this, f32::from_bits(0x4418_8000)).to_f32();
    font_print_string_from_bottom(px, py, text);
}

orig! {
    /// `CControllerConfigManager::ClearSettingsAssociatedWithAction(action, type)` (0x52FD70, thiscall)
    pub fn ccm_clear_settings(this: GPtr<u8>, action: u32, ty: u32) = thiscall 0x52FD70 => unported;
    /// `CControllerConfigManager::DeleteMatchingActionInitiators(action, key, type)` (0x531C90, thiscall)
    pub fn ccm_delete_matching(this: GPtr<u8>, action: u32, key: u32, ty: u32) = thiscall 0x531C90 => unported;
    /// `CControllerConfigManager::SetControllerKeyAssociatedWithAction(action, key, type)` (0x530490,
    /// thiscall)
    pub fn ccm_set_key(this: GPtr<u8>, action: u32, key: u32, ty: u32) = thiscall 0x530490 => unported;
    /// `CControllerConfigManager::GetControllerKeyAssociatedWithAction(action, type)` (0x52F4F0, thiscall)
    pub fn ccm_get_key(this: GPtr<u8>, action: u32, ty: u32) -> u32 = thiscall 0x52F4F0 => unported;
    /// `CControllerConfigManager::GetIsActionAButtonCombo(action)` (0x52F550, thiscall)
    pub fn ccm_is_combo(this: GPtr<u8>, action: u32) -> u8 = thiscall 0x52F550 => unported;
}

/// The keyboard keys `CheckCodesForControls` rejects (0, F1, F2, F3, F9, LWIN, PAD5, RWIN, RALT).
const INVALID_KEYS: [u32; 9] = [0, 0x3E9, 0x3EA, 0x3EB, 0x3F1, 0x41D, 0x408, 0x41E, 0x41C];

/// The redefine state reset of `CheckCodesForControls`.
fn reset_redefine(this: GPtr<u8>) {
    wr::<u32>(this, 0xF0, 0);
    wr::<u8>(this, 0x1B09, 0);
    wr::<i32>(this, 0x38, -1);
    wr::<u8>(this, 0x1B08, 0);
}

/// `CMenuManager::CheckCodesForControls(int32 type)` (0x57DB20, thiscall), as
/// `CMenuManager.deviations.md`: keyboard (0): escape (0x3E8) cancels (event 4), the nine invalid keys
/// return without the reset (event 4), other keys (event 1) are assigned with the "already bound" flag
/// (`GetControllerKeyAssociatedWithAction(action, 0)` neither rsNULL nor the key); mouse (2) / joystick
/// (3, a button combo sets +0x1AE8) (event 1). A combo or escape ends there (escape resets: +0x1B0B too);
/// else the action's settings (all four types once when +0x1B0B, then the flag's type) are cleared,
/// matching initiators deleted (keyboard: types 0 and 1) and the key set, the state reset, settings
/// saved.
#[hook(addr = 0x57DB20, conv = thiscall, name = "CMenuManager::CheckCodesForControls", expect = "51 53 55 8B 6C 24 10")]
pub fn check_codes_for_controls(this: GPtr<u8>, ty: u32) {
    let action: u32 = rd(this, 0x1B0C);
    let mut esc = false;
    let mut invalid = false;
    wr::<u8>(this, 0x1AE8, 0);
    let ae = g(AUDIO_ENGINE);
    let ccm = g(CONTROLS_MANAGER);
    let mut flag = ty;
    match ty {
        0 => {
            let key: u32 = rd(rd::<GPtr<u8>>(this, 0xF0), 0);
            if key == 0x3E8 {
                audio_report_frontend_event(ae, 4, 0, 0x3F80_0000);
                esc = true;
            } else if INVALID_KEYS.contains(&key) {
                audio_report_frontend_event(ae, 4, 0, 0x3F80_0000);
                invalid = true;
            } else {
                audio_report_frontend_event(ae, 1, 0, 0x3F80_0000);
                flag = 0;
                if ccm_get_key(ccm, action, 0) != 0x420 && ccm_get_key(ccm, action, 0) != key {
                    flag = 1;
                }
            }
        }
        2 => audio_report_frontend_event(ae, 1, 0, 0x3F80_0000),
        3 => {
            audio_report_frontend_event(ae, 1, 0, 0x3F80_0000);
            if ccm_is_combo(ccm, action) != 0 {
                wr::<u8>(this, 0x1AE8, 1);
            }
        }
        _ => {}
    }
    if b8(this, 0x1AE8) != 0 || esc {
        if esc {
            wr::<u8>(this, 0x1B0B, 0);
            reset_redefine(this);
        }
        return;
    }
    if invalid {
        return;
    }
    if b8(this, 0x1B0B) != 0 {
        for t in 0..4 {
            ccm_clear_settings(ccm, rd(this, 0x1B0C), t);
        }
        wr::<u8>(this, 0x1B0B, 0);
    }
    ccm_clear_settings(ccm, action, flag);
    match ty {
        0 => {
            let key = || rd::<u32>(rd::<GPtr<u8>>(this, 0xF0), 0);
            ccm_delete_matching(ccm, action, key(), 0);
            ccm_delete_matching(ccm, action, key(), 1);
            ccm_set_key(ccm, action, key(), flag);
        }
        2 => {
            ccm_delete_matching(ccm, action, rd(this, 0x1AE0), 2);
            ccm_set_key(ccm, action, rd(this, 0x1AE0), flag);
        }
        3 => {
            ccm_delete_matching(ccm, action, rd(this, 0x1AE4), 3);
            ccm_set_key(ccm, action, rd(this, 0x1AE4), flag);
        }
        _ => {}
    }
    reset_redefine(this);
    save_settings_call(this);
}

orig! {
    /// `CFont::SetAlphaFade(float)` (0x719500)
    pub fn font_set_alpha_fade(v: u32) = cdecl 0x719500 => crate::font::set_alpha_fade;
    /// `CMenuManager::SetFrontEndRenderStates()` (0x573A60, thiscall)
    pub fn set_front_end_render_states_call(this: GPtr<u8>) = thiscall 0x573A60 => set_front_end_render_states;
    /// `CMenuManager::DrawBackground()` (0x57B750, thiscall)
    pub fn draw_background(this: GPtr<u8>) = thiscall 0x57B750 => crate::menu_draw::draw_background;
    /// `RsEventHandler(RsEvent, void*)` (0x619B60)
    pub fn rs_event_handler(event: i32, param: GPtr<u8>) -> i32 = cdecl 0x619B60 => sa_core::platform::rs_event_handler;
    /// `Win::MessageLoop()` (0x746870)
    pub fn win_message_loop() = cdecl 0x746870 => sa_core::input::message_loop;
    /// `CPad::UpdatePads()` (0x541DD0)
    pub fn pad_update_pads() = cdecl 0x541DD0 => unported;
    /// `CMenuManager::MessageScreen(const char*, bool, bool)` (0x579330, thiscall)
    pub fn message_screen_call(this: GPtr<u8>, key: GPtr<u8>, black: u32, started: u32) = thiscall 0x579330 => message_screen;
}

/// `CMenuManager::DrawFrontEnd()` (0x57C290, thiscall): nothing while a game start is pending (+0x32).
/// Alpha fade 255, sprite / font per-frame state, `DefinedState`, the front-end render states; +0x1AE9
/// set unless a radio retune runs; the start screen 0x2A becomes 0x29 (+0xE9 clear) or 0x22; item 0
/// moves to 1 when the first item is a title (action 1); then (tail jump) `DrawBackground`.
#[hook(addr = 0x57C290, conv = thiscall, name = "CMenuManager::DrawFrontEnd", expect = "56 8B F1 8A 46 32 84 C0")]
pub fn draw_front_end(this: GPtr<u8>) {
    if b8(this, 0x32) != 0 {
        return;
    }
    font_set_alpha_fade(K_255_BITS);
    sprite2d_set_recip_near_clip();
    font_init_per_frame();
    defined_state();
    set_front_end_render_states_call(this);
    wr::<u8>(this, 0x1AE9, 1);
    if audio_is_radio_retune_in_progress(g(AUDIO_ENGINE)) != 0 {
        wr::<u8>(this, 0x1AE9, 0);
    }
    if b8(this, 0x15D) == 0x2A {
        wr::<u8>(this, 0x15D, if b8(this, 0xE9) == 0 { 0x29 } else { 0x22 });
    }
    if rd::<i32>(this, 0x54) == 0 && b8(screen(i32::from(b8(this, 0x15D) as i8)), 0xA) == 1 {
        wr::<u32>(this, 0x54, 1);
    }
    draw_background(this);
}

/// 255.0 (immediate 0x437F0000)
const K_255_BITS: u32 = 0x437F_0000;

/// `CMenuManager::DrawQuitGameScreen()` (0x57D860, thiscall): +0xB8 cleared, a black full-screen rect,
/// settings saved, `RsEventHandler(rsQUITAPP (0x1E), null)`.
#[hook(addr = 0x57D860, conv = thiscall, name = "CMenuManager::DrawQuitGameScreen", expect = "83 EC 14 56 8B F1")]
pub fn draw_quit_game_screen(this: GPtr<u8>) {
    wr::<u8>(this, 0xB8, 0);
    black_screen();
    save_settings_call(this);
    rs_event_handler(0x1E, GPtr::NULL);
}

/// `CMenuManager::CheckMissionPackValidMenu()` (0x57D720, thiscall; the pack byte read in the `.text`
/// chunk 0x4080F5): from the user directory opens `MPACK//MPACK%d//SCR.SCM` and `.../TEXT.GXT` of the
/// selected pack (through `gString`), resets the directory; both handles > 0 (signed): closed, AL = 1.
/// Otherwise the open one is closed and, in a user pause, the "NO_PAK" message screen of
/// `FrontEndMenuManager` loops (message loop, pads, black frame) until enter / return / the pad's
/// cross is pressed; then the pause ends, the mission pack is cleared, a game started, +0x33 cleared,
/// AL = 0.
#[hook(addr = 0x57D720, conv = thiscall, name = "CMenuManager::CheckMissionPackValidMenu", expect = "55 56 57 8B E9")]
pub fn check_mission_pack_valid_menu(this: GPtr<u8>) -> u8 {
    file_set_dir_my_documents();
    let gs: GPtr<u8> = g(0xB71670);
    crt_sprintf1(gs, g(0x859D78), i32::from(g::<u8>(MISSION_PACK).read()));
    let f1 = file_open(gs, g(0x859C6C));
    crt_sprintf1(gs, g(0x8656E4), i32::from(g::<u8>(MISSION_PACK).read()));
    let f2 = file_open(gs, g(0x859C6C));
    file_set_dir(g(S_EMPTY));
    let ok = |f: GPtr<u8>| (f.addr() as u32 as i32) > 0;
    if ok(f1) {
        file_close(f1);
        if ok(f2) {
            file_close(f2);
            return 1;
        }
    } else if ok(f2) {
        file_close(f2);
    }
    timer_start_user_pause();
    loop {
        win_message_loop();
        pad_update_pads();
        message_screen_call(g(FRONT_END_MANAGER), g(0x8656DC), 1, 0);
        do_rw_stuff_start_of_frame(0, 0, 0, 0, 0, 0, 0);
        do_rw_stuff_end_of_frame();
        get_pad(0);
        if key_just_pressed(0xB733CC, 0xB7315C) || key_just_pressed(0xB733EA, 0xB7317A) {
            break;
        }
        let pad = get_pad(0);
        if rd::<u16>(pad, 0x20) != 0 && rd::<u16>(pad, 0x50) == 0 {
            break;
        }
    }
    timer_end_user_pause();
    g::<u8>(MISSION_PACK).write(0);
    do_settings_before_starting_a_game_call(g(FRONT_END_MANAGER));
    wr::<u8>(this, 0x33, 0);
    0
}

/// `FrontEndMenuManager` (0xBA6748)
const FRONT_END_MANAGER: usize = 0xBA6748;

orig! {
    /// `CMenuManager::CheckRedefineControlInput()` (0x57E4D0, thiscall)
    pub fn check_redefine_control_input(this: GPtr<u8>) -> u8 = thiscall 0x57E4D0 => check_redefine_control_input_port;
    /// `CMenuManager::GetNumberOfMenuOptions()` (0x573E70, thiscall)
    pub fn get_number_of_menu_options_call(this: GPtr<u8>) -> u8 = thiscall 0x573E70 => get_number_of_menu_options;
    /// `CMenuManager::ProcessMenuOptions(int8, bool*, bool)` (0x576FE0, thiscall)
    pub fn process_menu_options_call(this: GPtr<u8>, lr: u32, go_back: GPtr<u8>, accept: u32) = thiscall 0x576FE0 => process_menu_options;
    /// `CMenuManager::CheckSliderMovement(int8)` (0x573440, thiscall)
    pub fn check_slider_movement_call(this: GPtr<u8>, lr: u32) = thiscall 0x573440 => check_slider_movement;
}

/// The action byte of item `i` on the current screen.
fn item_action(this: GPtr<u8>, i: i32) -> u8 {
    rd(screen(i32::from(b8(this, 0x15D) as i8)), (0xA + i.wrapping_mul(0x12)) as isize)
}

/// `CMenuManager::ProcessUserInput(bool down, bool up, bool accept, bool back, int8 lr)` (0x57B480,
/// thiscall): nothing on screen 0x2B or while a control is being redefined. With more than one option:
/// down / up move the item (event 3 off the map screen), skipping action 0x14 items, wrapping to the
/// first selectable item (1 when item 0 is a title) or, upwards, to the last item; accept (on screen
/// 0x26 the redefine state first) runs `ProcessMenuOptions(0, &back, accept)` and, unless that asked
/// to go back, plays event 4 for a used save slot outside the pause menu, else event 1; `lr` on an
/// item of type 0xC runs `ProcessMenuOptions(lr, &back, false)` and `CheckSliderMovement(lr)`; back
/// (the argument, or set by the options) plays event 2 and goes back (-2) when allowed (+0x1AE9), else
/// event 4.
#[hook(addr = 0x57B480, conv = thiscall, name = "CMenuManager::ProcessUserInput", expect = "56 8B F1 80 BE 5D 01 00 00 2B")]
pub fn process_user_input(this: GPtr<u8>, down: u32, up: u32, accept: u32, back: u32, lr: u32) {
    if b8(this, 0x15D) == 0x2B || check_redefine_control_input(this) != 0 {
        return;
    }
    let ae = g(AUDIO_ENGINE);
    if get_number_of_menu_options_call(this) > 1 && down as u8 != 0 {
        if b8(this, 0x15D) != 5 {
            audio_report_frontend_event(ae, 3, 0, 0x3F80_0000);
        }
        let mut i = rd::<i32>(this, 0x54).wrapping_add(1);
        wr(this, 0x54, i);
        while item_action(this, i) == 0x14 {
            i = i.wrapping_add(1);
            wr(this, 0x54, i);
        }
        if i >= 0xC || item_action(this, i) == 0 {
            wr(this, 0x54, i32::from(item_action(this, 0) == 1));
        }
    }
    if get_number_of_menu_options_call(this) > 1 && up as u8 != 0 {
        if b8(this, 0x15D) != 5 {
            audio_report_frontend_event(ae, 3, 0, 0x3F80_0000);
        }
        let first = i32::from(item_action(this, 0) == 1);
        let i: i32 = rd(this, 0x54);
        if i > first {
            let mut i = i - 1;
            wr(this, 0x54, i);
            while item_action(this, i) == 0x14 {
                i -= 1;
                wr(this, 0x54, i);
            }
        } else {
            while rd::<i32>(this, 0x54) < 0xB && item_action(this, rd::<i32>(this, 0x54) + 1) != 0 {
                wr(this, 0x54, rd::<i32>(this, 0x54) + 1);
            }
            while item_action(this, rd(this, 0x54)) == 0x14 {
                wr(this, 0x54, rd::<i32>(this, 0x54) - 1);
            }
        }
    }
    let mut back_slot = back;
    let bp = local(&mut back_slot);
    if accept as u8 != 0 {
        if b8(this, 0x15D) == 0x26 {
            wr::<u8>(this, 0x1B09, 1);
            wr::<u8>(this, 0x1B08, 1);
            wr(this, 0xF0, this.byte_add(0x38));
            wr::<u32>(this, 0x1B00, 0x10);
        }
        process_menu_options_call(this, 0, bp, accept);
        if b8(bp, 0) == 0 {
            let item: i32 = rd(this, 0x54);
            let ty = rd::<u8>(screen(i32::from(b8(this, 0x15D) as i8)), (0x13 + item.wrapping_mul(0x12)) as isize) as i8;
            let used = b8(this, 0xF4) == 0
                && (1..=8).contains(&ty)
                && g::<u32>(0xC16EB8u32.wrapping_add((item as u32).wrapping_mul(4)) as usize).read() != 0;
            audio_report_frontend_event(ae, if used { 4 } else { 1 }, 0, 0x3F80_0000);
        }
    }
    if lr as u8 != 0 {
        let item: i32 = rd(this, 0x54);
        if rd::<u8>(screen(i32::from(b8(this, 0x15D) as i8)), (0x13 + item.wrapping_mul(0x12)) as isize) == 0xC {
            process_menu_options_call(this, lr, bp, 0);
            check_slider_movement_call(this, lr);
        }
    }
    if b8(bp, 0) != 0 {
        if b8(this, 0x1AE9) != 0 {
            audio_report_frontend_event(ae, 2, 0, 0x3F80_0000);
            switch_to_new_screen_call(this, 0xFFFF_FFFE);
        } else {
            audio_report_frontend_event(ae, 4, 0, 0x3F80_0000);
        }
    }
}

orig! {
    /// `C_PcSave::SaveSlot(int32)` (0x619060, thiscall)
    pub fn pc_save_save_slot(this: GPtr<u8>, slot: u32) -> u8 = thiscall 0x619060 => unported;
    /// `C_PcSave::DeleteSlot(int32)` (0x6190D0, thiscall)
    pub fn pc_save_delete_slot(this: GPtr<u8>, slot: u32) -> u8 = thiscall 0x6190D0 => unported;
    /// `CGenericGameStorage::CheckSlotDataValid(int32)` (0x5D1380)
    pub fn storage_check_slot_data_valid(slot: i32) -> u8 = cdecl 0x5D1380 => unported;
    /// `CMenuManager::JumpToGenericMessageScreen(eMenuScreen, const char*, const char*)` (0x576AE0,
    /// thiscall)
    pub fn jump_to_generic_message_screen_call(this: GPtr<u8>, scr: u32, title: GPtr<u8>, message: GPtr<u8>) = thiscall 0x576AE0 => jump_to_generic_message_screen;
}

/// `PcSaveHelper` (0xC17034)
const PC_SAVE_HELPER: usize = 0xC17034;

/// `CMenuManager::ProcessFileActions()` (0x578D60, thiscall): each of the load (13), delete (14) and
/// save (18) screens first only arms its static (0xBA8284 / 0xBA8285 / 0xBA8286) so that one frame is
/// drawn. Load: a valid slot starts the game (`DoSettingsBeforeStartingAGame` unless +0xE9; +0x32,
/// +0x60 set, mission pack cleared), else the error screen 0x14. Delete: success repopulates the slots,
/// screen 0xF at item 1, else screen 0x14. Save: with a mission pack its `SCR.SCM` is probed from the
/// user directory (a handle > 0 signed is closed; the directory is not reset); `SaveSlot` 0 = success
/// -> screen 0x13 unless the probe failed for a selected pack; otherwise screen 0x15 with the pack or
/// the plain error text.
#[hook(addr = 0x578D60, conv = thiscall, name = "CMenuManager::ProcessFileActions", expect = "51 53 56 8B F1 0F BE 86 5D 01 00 00")]
pub fn process_file_actions_port(this: GPtr<u8>) {
    match b8(this, 0x15D) as i8 {
        0xD => {
            let armed = g::<u8>(0xBA8284);
            if armed.read() == 0 {
                armed.write(1);
                return;
            }
            if storage_check_slot_data_valid(i32::from(b8(this, 0x15F) as i8)) != 0 {
                if b8(this, 0xE9) == 0 {
                    do_settings_before_starting_a_game_call(this);
                }
                wr::<u8>(this, 0x32, 1);
                g::<u8>(MISSION_PACK).write(0);
                wr::<u8>(this, 0x60, 1);
            } else {
                jump_to_generic_message_screen_call(this, 0x14, g(0x8653F8), g(0x865400));
            }
            armed.write(0);
        }
        0xE => {
            let armed = g::<u8>(0xBA8285);
            if armed.read() == 0 {
                armed.write(1);
                return;
            }
            if pc_save_delete_slot(g(PC_SAVE_HELPER), u32::from(b8(this, 0x15F))) != 0 {
                pc_save_populate_slot_info(g(PC_SAVE_HELPER));
                switch_to_new_screen_call(this, 0xF);
                wr::<u32>(this, 0x54, 1);
            } else {
                jump_to_generic_message_screen_call(this, 0x14, g(0x865408), g(0x865410));
            }
            armed.write(0);
        }
        0x12 => {
            let armed = g::<u8>(0xBA8286);
            if armed.read() == 0 {
                armed.write(1);
                return;
            }
            let mut pack_ok = true;
            let mut error = false;
            if g::<u8>(MISSION_PACK).read() != 0 {
                file_set_dir_my_documents();
                crt_sprintf1(g(0xB71670), g(0x859D78), i32::from(g::<u8>(MISSION_PACK).read()));
                let f = file_open(g(0xB71670), g(0x859C6C));
                if (f.addr() as u32 as i32) > 0 {
                    file_close(f);
                } else {
                    pack_ok = false;
                    error = g::<u8>(MISSION_PACK).read() != 0;
                }
            }
            if !error {
                let r = pc_save_save_slot(g(PC_SAVE_HELPER), u32::from(b8(this, 0x15F)));
                pc_save_populate_slot_info(g(PC_SAVE_HELPER));
                if r == 0 && (pack_ok || g::<u8>(MISSION_PACK).read() == 0) {
                    switch_to_new_screen_call(this, 0x13);
                    armed.write(0);
                    return;
                }
            }
            let msg = if g::<u8>(MISSION_PACK).read() != 0 && !pack_ok { 0x865428 } else { 0x865420 };
            jump_to_generic_message_screen_call(this, 0x15, g(0x865418), g(msg));
            armed.write(0);
        }
        _ => {}
    }
}

orig! {
    /// `GetCurrentKeyPressed(RsKeyCodes*)` (0x541490)
    pub fn get_current_key_pressed_call(out: GPtr<u8>) -> GPtr<u8> = cdecl 0x541490 => get_current_key_pressed;
    /// `CPad::IsMouseWheelUpPressed()` (0x57C400)
    pub fn mouse_wheel_up_call() -> u8 = cdecl 0x57C400 => is_mouse_wheel_up_pressed;
    /// `CPad::IsMouseWheelDownPressed()` (0x57C420)
    pub fn mouse_wheel_down_call() -> u8 = cdecl 0x57C420 => is_mouse_wheel_down_pressed;
    /// `CPad::IsMouseBmx1Pressed()` (0x57C440)
    pub fn mouse_bmx1_call() -> u8 = cdecl 0x57C440 => is_mouse_bmx1_pressed;
    /// `CPad::IsMouseBmx2Pressed()` (0x57C460)
    pub fn mouse_bmx2_call() -> u8 = cdecl 0x57C460 => is_mouse_bmx2_pressed;
    /// `CControllerConfigManager::GetJoyButtonJustDown()` (0x52D1E0, thiscall)
    pub fn ccm_get_joy_button_just_down(this: GPtr<u8>) -> u32 = thiscall 0x52D1E0 => unported;
    /// `CMenuManager::CheckCodesForControls(int32)` (0x57DB20, thiscall)
    pub fn check_codes_for_controls_call(this: GPtr<u8>, ty: u32) = thiscall 0x57DB20 => check_codes_for_controls;
}

/// `CMenuManager::CheckRedefineControlInput()` (0x57E4D0, thiscall): while redefining (+0x1B09) and
/// not in the first frame (+0x1B08, cleared): the key pressed into `*(+0xF0)` (`GetCurrentKeyPressed`),
/// the mouse button just pressed (left 1, right 3, middle 2, wheel up 4 / down 5, bmx1 6, bmx2 7, the
/// pad fetched before each test) into +0x1AE0, the joystick button into +0x1AE4; the input type is 3
/// for a joystick button, 2 for a mouse button, 0 for a key. With +0x1B14: when +0x1B0A, event 1 and the
/// clear-first state; else `CheckCodesForControls(type)` when anything was pressed and +0x1B15 set.
/// Without +0x1B14 the redefine state is reset. AL = +0xEC or +0xF0 set.
#[hook(addr = 0x57E4D0, conv = thiscall, name = "CMenuManager::CheckRedefineControlInput", expect = "51 53 56 8B F1 8A 86 09 1B 00 00")]
pub fn check_redefine_control_input_port(this: GPtr<u8>) -> u8 {
    if b8(this, 0x1B09) != 0 {
        if b8(this, 0x1B08) != 0 {
            wr::<u8>(this, 0x1B08, 0);
        } else {
            let out = get_current_key_pressed_call(rd(this, 0xF0));
            wr(this, 0xF0, out);
            wr::<u32>(this, 0x1AE0, 0);
            wr::<u32>(this, 0x1AE4, 0);
            get_pad(0);
            // The mouse buttons in the original's test order, `GetPad(0)` before each but the first.
            let tests: [(fn() -> bool, u32); 7] = [
                (|| mouse_just_pressed(1) != 0, 1),
                (|| mouse_just_pressed(2) != 0, 3),
                (|| mouse_just_pressed(3) != 0, 2),
                (|| mouse_wheel_up_call() != 0, 4),
                (|| mouse_wheel_down_call() != 0, 5),
                (|| mouse_bmx1_call() != 0, 6),
                (|| mouse_bmx2_call() != 0, 7),
            ];
            let mut mouse = 0u32;
            for (k, (test, v)) in tests.into_iter().enumerate() {
                if k > 0 {
                    get_pad(0);
                }
                if test() {
                    mouse = v;
                    break;
                }
            }
            if mouse != 0 {
                wr::<u32>(this, 0x1AE0, mouse);
            }
            let joy = ccm_get_joy_button_just_down(g(CONTROLS_MANAGER));
            wr(this, 0x1AE4, joy);
            // With no joystick button the original takes the type from an uninitialised stack slot
            // (the entry ECX); it is then never used (no key and no mouse button either, or the key
            // sets 0 below).
            let mut ty: u32 = if joy == 0 { this.addr() as u32 } else { 3 };
            let m: u32 = rd(this, 0x1AE0);
            if m != 0 {
                ty = 2;
            }
            let key: u32 = rd(rd::<GPtr<u8>>(this, 0xF0), 0);
            if key != 0x420 {
                ty = 0;
            }
            if b8(this, 0x1B14) != 0 {
                if b8(this, 0x1B0A) != 0 {
                    audio_report_frontend_event(g(AUDIO_ENGINE), 1, 0, 0x3F80_0000);
                    wr::<u8>(this, 0x1B14, 0);
                    wr::<u8>(this, 0x1B0B, 1);
                    wr::<u8>(this, 0x1B0A, 0);
                } else {
                    if key != 0x420 || m != 0 || joy != 0 {
                        check_codes_for_controls_call(this, ty);
                    }
                    wr::<u8>(this, 0x1B15, 1);
                }
            } else {
                reset_redefine(this);
            }
        }
    }
    u8::from(rd::<u32>(this, 0xEC) != 0 || rd::<u32>(this, 0xF0) != 0)
}

/// The named keys of `GetCurrentKeyPressed` in the original's test order: (new state word, old state
/// word, `RsKeyCodes` value); the last pressed key wins.
const NAMED_KEYS: [(usize, u32); 44] = [
    (0xB733A8, 0x3E8), (0xB733AA, 0x3F5), (0xB733AC, 0x3F6), (0xB733AE, 0x3F7), (0xB733B0, 0x3F8),
    (0xB733B2, 0x3F9), (0xB733B4, 0x3FA), (0xB733B6, 0x3FB), (0xB733B8, 0x3FC), (0xB733BA, 0x3FD),
    (0xB733BC, 0x3FE), (0xB733BE, 0x410), (0xB733C0, 0x411), (0xB733C2, 0x409), (0xB733C4, 0x3FF),
    (0xB733C6, 0x400), (0xB733C8, 0x402), (0xB733CA, 0x401), (0xB733CC, 0x40F), (0xB733CE, 0x403),
    (0xB733D0, 0x404), (0xB733D2, 0x405), (0xB733D4, 0x406), (0xB733D6, 0x407), (0xB733D8, 0x408),
    (0xB733DA, 0x40A), (0xB733DC, 0x40B), (0xB733DE, 0x40C), (0xB733E0, 0x40D), (0xB733E2, 0x40E),
    (0xB733E4, 0x412), (0xB733E6, 0x413), (0xB733E8, 0x414), (0xB733EA, 0x415), (0xB733EC, 0x416),
    (0xB733F0, 0x418), (0xB733EE, 0x417), (0xB733F2, 0x419), (0xB733F4, 0x41A), (0xB733F6, 0x41B),
    (0xB733F8, 0x41C), (0xB733FA, 0x41D), (0xB733FC, 0x41E), (0xB733FE, 0x41F),
];

/// `GetCurrentKeyPressed(RsKeyCodes* out)` (0x541490, cdecl): `*out = rsNULL (0x420)`, then every key
/// just pressed (new word set, old word 0x270 bytes below clear) overwrites it: standard keys 0..254
/// (0xB731A8), F1..F12 (0xB73190, 0x3E9..), then the named keys in the original's order (0xB733F0 /
/// 0x418 is tested before 0xB733EE / 0x417, as the bytes). Returns `out` in EAX.
#[hook(addr = 0x541490, conv = cdecl, name = "GetCurrentKeyPressed", expect = "8B 44 24 04 56 C7 00 20 04 00 00")]
pub fn get_current_key_pressed(out: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(out, 0, 0x420);
    for c in 0..0xFFusize {
        if key_just_pressed(0xB731A8 + c * 2, 0xB72F38 + c * 2) {
            wr(out, 0, c as u32);
        }
    }
    for i in 0..12usize {
        if key_just_pressed(0xB73190 + i * 2, 0xB72F20 + i * 2) {
            wr(out, 0, 0x3E9 + i as u32);
        }
    }
    for (new, v) in NAMED_KEYS {
        if key_just_pressed(new, new - 0x270) {
            wr(out, 0, v);
        }
    }
    out
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_STRETCH_X,
    HOOK_STRETCH_Y,
    HOOK_IS_ESC_JUST_PRESSED,
    HOOK_IS_MOUSE_RBUTTON_PRESSED,
    HOOK_IS_MOUSE_WHEEL_UP_PRESSED,
    HOOK_IS_MOUSE_WHEEL_DOWN_PRESSED,
    HOOK_IS_MOUSE_BMX1_PRESSED,
    HOOK_IS_MOUSE_BMX2_PRESSED,
    HOOK_CHECK_FRONT_END_UP_INPUT,
    HOOK_CHECK_FRONT_END_DOWN_INPUT,
    HOOK_CHECK_FRONT_END_LEFT_INPUT,
    HOOK_CHECK_FRONT_END_RIGHT_INPUT,
    HOOK_SCROLL_RADIO_STATIONS,
    HOOK_SET_FRONT_END_RENDER_STATES,
    HOOK_HAS_LANGUAGE_CHANGED,
    HOOK_GET_NUMBER_OF_MENU_OPTIONS,
    HOOK_CHECK_HOVER,
    HOOK_SET_HELPER_TEXT,
    HOOK_RESET_HELPER_TEXT,
    HOOK_SAVE_LOAD_FILE_ERROR_SET_UP_ERROR_SCREEN,
    HOOK_CENTRE_MOUSE_POINTER,
    HOOK_SWITCH_TO_NEW_SCREEN,
    HOOK_JUMP_TO_GENERIC_MESSAGE_SCREEN,
    HOOK_DO_SETTINGS_BEFORE_STARTING_A_GAME,
    HOOK_INITIALISE_CHANGED_LANGUAGE_SETTINGS,
    HOOK_SET_DEFAULT_PREFERENCES,
    HOOK_SAVE_SETTINGS,
    HOOK_DRAW_WINDOW,
    HOOK_PROCESS_STREAMING,
    HOOK_LOAD_SETTINGS,
    HOOK_SMALL_MESSAGE_SCREEN,
    HOOK_MESSAGE_SCREEN,
    HOOK_CHECK_FOR_MENU_CLOSING,
    HOOK_PROCESS,
    HOOK_INITIALISE,
    HOOK_UNLOAD_TEXTURES,
    HOOK_CONSTRUCTOR,
    HOOK_DESTRUCTOR,
    HOOK_GET_CURRENT_KEY_PRESSED,
    HOOK_CHECK_SLIDER_MOVEMENT,
    HOOK_LOAD_ALL_TEXTURES_PORT,
    HOOK_SWAP_TEXTURES_ROUND_PORT,
    HOOK_PROCESS_MENU_OPTIONS,
    HOOK_PROCESS_PC_MENU_OPTIONS_PORT,
    HOOK_PROCESS_MISSION_PACK_NEW_GAME_PORT,
    HOOK_DISPLAY_HELPER_TEXT,
    HOOK_CHECK_CODES_FOR_CONTROLS,
    HOOK_DRAW_FRONT_END,
    HOOK_DRAW_QUIT_GAME_SCREEN,
    HOOK_CHECK_MISSION_PACK_VALID_MENU,
    HOOK_PROCESS_USER_INPUT,
    HOOK_PROCESS_FILE_ACTIONS_PORT,
    HOOK_CHECK_REDEFINE_CONTROL_INPUT_PORT,
];

/// The Rust path of [`stretch_x_call`] as an ST0 value.
#[allow(dead_code)]
fn stretch_x_call_st0_rust(this: GPtr<u8>, v: f32) -> f64 {
    stretch_x(this, v).to_f64()
}

/// [`stretch_x_call_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
pub fn stretch_x_call(this: GPtr<u8>, v: f32) -> X87 {
    X87::from_f64_exact(stretch_x_call_st0(this, v))
}

/// The Rust path of [`stretch_y_call`] as an ST0 value.
#[allow(dead_code)]
fn stretch_y_call_st0_rust(this: GPtr<u8>, v: f32) -> f64 {
    stretch_y(this, v).to_f64()
}

/// [`stretch_y_call_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
pub fn stretch_y_call(this: GPtr<u8>, v: f32) -> X87 {
    X87::from_f64_exact(stretch_y_call_st0(this, v))
}
