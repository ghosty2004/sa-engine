//! WRITER-MVP-B (batch `mvpb-core-1`): the per-frame orchestrators of `app_game.cpp` / `app.cpp`
//! (`Game::FrontendIdle`, `Game::Idle`, `CGame::Process`, `Game::RenderScene`, `Game::RenderEffects`,
//! `Game::Render2dStuff`, `DoRWStuffStartOfFrame(_Horizon)`) and the small CORE leaves they reach
//! (`CPool` instantiations, `CLinkList::Clear`, `List_c::GetNumItems`).
//!
//! Ported from the 1.0 US bytes (gta-reversed `app/app_game.cpp`, `app/app.cpp`, `Game.cpp` as reading
//! aid). Differences to gta-reversed that the bytes show and the ports keep:
//! - `Game::Idle` starts with the 14 ms frame limiter (busy loop on `CTimer::GetCurrentTimeInCycles /
//!   GetCyclesPerMillisecond` against `g_LastTickTime` 0xB72CA8), which gta-reversed comments out;
//! - `Game::Idle` and `Game::FrontendIdle` call `CCredits::RenderCredits` under the same test twice
//!   in a row (Idle) / once (FrontendIdle) and `CFont::DrawFonts` twice (FrontendIdle);
//! - `DoRWStuffStartOfFrame` calls `CFont::SetAlphaFade(255.0)` first;
//! - `Game::Render2dStuff` reads its weapon type from an uninitialised local when there is no
//!   player ped (see the port);
//! - `CGame::Process` measures `CStreaming::Update` (and, when that took < 4 ms, `CPopulation::Update`)
//!   with the cycle timer and skips `CCarCtrl::GenerateRandomCars` when the measured time is ≥ 4 ms.
//!
//! Every callee goes through `orig!`; the view window `tan(FOV * pi/360)` is FPTAN on the x87 (model
//! `sa_math::x87::X87::fptan`), rounded to f32 when stored as the `CameraSize` argument.

use crate::mem::{GPtr, GameData};
use crate::pool;
use crate::pools::{PoolPtr, Slot};
use crate::{hook, orig};
use sa_math::x87::X87;

fn g<T: GameData>(a: u32) -> GPtr<T> {
    GPtr::from_addr(a as usize)
}

/// `Scene.m_pRwCamera` (0xC1703C).
const SCENE_CAMERA: u32 = 0xC1703C;
/// `RsGlobal.maximumWidth` / `maximumHeight`.
const RS_MAX_WIDTH: u32 = 0xC17044;
const RS_MAX_HEIGHT: u32 = 0xC17048;
/// `CDraw::ms_fFOV` (0x8D5038) and `CDraw::ms_fAspectRatio` (0xC3EFA4).
const DRAW_FOV: u32 = 0x8D5038;
const DRAW_ASPECT: u32 = 0xC3EFA4;
/// `gColourTop` (RwRGBA, 0xB72CA0).
const COLOUR_TOP: u32 = 0xB72CA0;
/// `g_LastTickTime` (0xB72CA8).
const LAST_TICK_TIME: u32 = 0xB72CA8;
/// `FrontEndMenuManager` (0xBA6748): `m_bActivateMenuNextFrame` +0x33, `m_bMenuActive` +0x5C,
/// `m_bWidescreenOn` +0x4B.
const MENU: u32 = 0xBA6748;
const MENU_ACTIVATE_NEXT_FRAME: u32 = 0xBA677B;
const MENU_ACTIVE: u32 = 0xBA67A4;
const MENU_WIDESCREEN_ON: u32 = 0xBA6793;
/// `RsGlobal.quit` (0xC17050).
const RS_QUIT: u32 = 0xC17050;
/// `CCredits::bCreditsGoing` (0xC6E97C).
const CREDITS_GOING: u32 = 0xC6E97C;
/// `TheCamera` (0xB6F028) and fields: `m_pMat` +0x14 (0xB6F03C), placement position +0x4 (0xB6F02C),
/// `m_nActiveCam` +0x59 (0xB6F081), `m_bWideScreenOn` +0x3D (0xB6F065), `m_nBlurType` +0x90
/// (0xB6F0B8), `m_fScreenReductionPercentage` +0x144 (0xB6F16C), `m_mViewMatrix` +0xA04 (0xB6FA2C),
/// `m_pRwCamera` +0x954 (0xB6F97C), `Cams[]` +0x174 (0x238 each; `Front.z` +0x30C from the camera,
/// `Mode` +0x180).
const THE_CAMERA: u32 = 0xB6F028;
const CAMERA_MAT: u32 = 0xB6F03C;
const CAMERA_PLACEMENT_POS: u32 = 0xB6F02C;
const CAMERA_ACTIVE_CAM: u32 = 0xB6F081;
const CAMERA_WIDESCREEN_ON: u32 = 0xB6F065;
const CAMERA_BLUR_TYPE: u32 = 0xB6F0B8;
const CAMERA_SCREEN_REDUCTION: u32 = 0xB6F16C;
const CAMERA_VIEW_MATRIX: u32 = 0xB6FA2C;
const CAMERA_RW_CAMERA: u32 = 0xB6F97C;
/// `TheCamera.Cams[m_nActiveCam].Front.z` base (0xB6F334 + 0x238 * active).
const CAM_FRONT_Z: u32 = 0xB6F334;
/// `TheCamera.Cams[m_nActiveCam].Mode` base (int16, 0xB6F1A8 + 0x238 * active).
const CAM_MODE: u32 = 0xB6F1A8;
/// `AudioEngine` (0xB6BC90), `g_fx` (0xA9AE00), `g_breakMan` (0xBB4240), `g_interiorMan` (0xBAF670),
/// `g_procObjMan` (0xBB7CB0), `g_waterCreatureMan` (0xC1DF30), `g_realTimeShadowMan` (0xC40350),
/// `CLoadMonitor` instance (0xB72978), `CFireManager` `gFireManager` (0xB71F80), `ThePaths` (0x96F050),
/// `CInterestingEvents` instance (0xC0B058), `CUserDisplay::OnscnTimer` (0xBA1788),
/// `CVisibilityPlugins::ms_weaponPedsForPC` (0xC88224).
const AUDIO_ENGINE: u32 = 0xB6BC90;
const G_FX: u32 = 0xA9AE00;
const G_BREAK_MAN: u32 = 0xBB4240;
const G_INTERIOR_MAN: u32 = 0xBAF670;
const G_PROC_OBJ_MAN: u32 = 0xBB7CB0;
const G_WATER_CREATURE_MAN: u32 = 0xC1DF30;
const G_REAL_TIME_SHADOW_MAN: u32 = 0xC40350;
const LOAD_MONITOR: u32 = 0xB72978;
const FIRE_MANAGER: u32 = 0xB71F80;
const THE_PATHS: u32 = 0x96F050;
const INTERESTING_EVENTS: u32 = 0xC0B058;
const ONSCREEN_TIMER: u32 = 0xBA1788;
const WEAPON_PEDS_FOR_PC: u32 = 0xC88224;
/// `CReplay::Mode` (0xA43088; 1 = playback).
const REPLAY_MODE: u32 = 0xA43088;
/// `CCutsceneMgr::ms_running` (0xB5F851), `CCutsceneMgr::ms_cutsceneProcessing` (0xB5F852).
const CUTSCENE_RUNNING: u32 = 0xB5F851;
const CUTSCENE_PROCESSING: u32 = 0xB5F852;
/// `CTimer::m_CodePause` (0xB7CB48), `m_UserPause` (0xB7CB49), `bSkipProcessThisFrame` (0xB7CB89),
/// `ms_fTimeStep` (0xB7CB5C).
const TIMER_CODE_PAUSE: u32 = 0xB7CB48;
const TIMER_USER_PAUSE: u32 = 0xB7CB49;
const TIMER_SKIP_PROCESS: u32 = 0xB7CB89;
const TIMER_TIME_STEP: u32 = 0xB7CB5C;
/// `CPointLights::NumLights` (0xC3F0D0), `CTheScripts::NumberOfIntroRectanglesThisFrame`-style counter
/// reset by `CGame::Process` (0xC81450).
const POINT_LIGHTS_NUM: u32 = 0xC3F0D0;
const PROCESS_RESET_C81450: u32 = 0xC81450;
/// `CInterestingEvents` flags byte (bit 0 = enabled, 0xC0B184).
const INTERESTING_EVENTS_FLAGS: u32 = 0xC0B184;
/// `CTimeCycle::m_CurrentStoredValue` (0xB79FD0) and the two per-entry float tables `CGame::Process`
/// doubles into the stencil-shadow light vector (0xB79F90 x, 0xB79F50 y).
const TC_CURRENT_STORED: u32 = 0xB79FD0;
const TC_TABLE_X: u32 = 0xB79F90;
const TC_TABLE_Y: u32 = 0xB79F50;
/// `CWeather::LightningFlash` (0xC812CC), `CWeather::UnderWaterness` (0xC8132C).
const WEATHER_LIGHTNING_FLASH: u32 = 0xC812CC;
const WEATHER_UNDERWATERNESS: u32 = 0xC8132C;
/// `CTimeCycle` current sky colours (int16 each: top R/G/B 0xB7C4C4.., bottom R/G/B ..0xB7C4CE),
/// `CTimeCycle::m_CurrentColours.m_fFarClip` (0xB7C4F0), `m_fFogStart` (0xB7C4F4).
const TC_SKY_TOP_RED: u32 = 0xB7C4C4;
const TC_FAR_CLIP: u32 = 0xB7C4F0;
const TC_FOG_START: u32 = 0xB7C4F4;
/// `CMirrors::TypeOfMirror` (0xC7C724), `CMirrors::bRenderingReflection` (0xC7C728).
const MIRRORS_TYPE: u32 = 0xC7C724;
const MIRRORS_RENDERING_REFLECTION: u32 = 0xC7C728;
/// `CHeli::NumberOfSearchLights` (0xC1C96C), `CTheScripts::NumberOfScriptSearchLights` (int16 0xA90830).
const HELI_NUM_SEARCHLIGHTS: u32 = 0xC1C96C;
const SCRIPT_NUM_SEARCHLIGHTS: u32 = 0xA90830;

/// `.rdata` constants (f32 bits): 0x8631D4 pi/360, 0x858B8C 0.5, 0x858B50 0.0, 0x858C84 0.25,
/// 0x858B38 0.02, 0x859524 1/448, 0x859520 1/640, 0x85A994 180.0, 0x858F98 170.0, 0x863B34 210.0.
mod k {
    pub const PI_OVER_360: u32 = 0x3C0E_FA35;
    pub const HALF: u32 = 0x3F00_0000;
    pub const ZERO: u32 = 0x0000_0000;
    pub const QUARTER: u32 = 0x3E80_0000;
    pub const FIFTIETH: u32 = 0x3CA3_D70A;
    pub const INV_448: u32 = 0x3B12_4925;
    pub const INV_640: u32 = 0x3ACC_CCCD;
    pub const K180: u32 = 0x4334_0000;
    pub const K170: u32 = 0x432A_0000;
    pub const K210: u32 = 0x4352_0000;
    pub const MINUS_1_5: u32 = 0xBFC0_0000;
}

/// `.data` floats `RenderScene` multiplies (2.0 at 0x8CD4F0, 2^-24 at 0x8CD4EC).
const NEAR_K1: u32 = 0x8CD4F0;
const NEAR_K2: u32 = 0x8CD4EC;

orig! {
    /// `CDraw::CalculateAspectRatio()` (0x6FF420; float in ST0, popped by every caller here)
    fn draw_calculate_aspect_ratio() -> f32 = cdecl 0x6FF420 => unported;
    /// `CTimer::Update()` (0x561B10)
    fn timer_update() = cdecl 0x561B10 => unported;
    /// `CTimer::GetCurrentTimeInCycles()` (0x561A80)
    fn timer_get_current_time_in_cycles() -> u32 = cdecl 0x561A80 => crate::timer::get_current_time_in_cycles;
    /// `CTimer::GetCurrentTimeInCycles()` as the `Game::Idle` busy-wait calls it (0x53E930; a
    /// determinism-mode site: retargeted to a constant that ends the wait)
    fn timer_cycles_idle_wait() -> u32 = cdecl callsite 0x53E930 => crate::timer::get_current_time_in_cycles;
    /// `CTimer::GetCurrentTimeInCycles()` as `Game::Idle` calls it for `g_LastTickTime` (0x53E94F; a
    /// determinism-mode site: retargeted to 0)
    fn timer_cycles_idle_last_tick() -> u32 = cdecl callsite 0x53E94F => crate::timer::get_current_time_in_cycles;
    /// `CTimer::GetCyclesPerMillisecond()` (0x561A40)
    fn timer_get_cycles_per_millisecond() -> u32 = cdecl 0x561A40 => crate::timer::get_cycles_per_millisecond;
    /// `CSprite2d::SetRecipNearClip()` (0x727260)
    fn sprite2d_set_recip_near_clip() = cdecl 0x727260 => unported;
    /// `CSprite2d::InitPerFrame()` (0x727350)
    fn sprite2d_init_per_frame() = cdecl 0x727350 => unported;
    /// `CFont::InitPerFrame()` (0x719800)
    fn font_init_per_frame() = cdecl 0x719800 => unported;
    /// `CFont::SetAlphaFade(float)` (0x719500)
    fn font_set_alpha_fade(a: u32) = cdecl 0x719500 => unported;
    /// `CFont::DrawFonts()` (0x71A210)
    fn font_draw_fonts() = cdecl 0x71A210 => unported;
    /// `CPad::UpdatePads()` (0x541DD0)
    fn pad_update_pads() = cdecl 0x541DD0 => unported;
    /// `CPad::GetPad(int)` (0x53FB70)
    fn pad_get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CMenuManager::Process()` (0x57B440)
    fn menu_process(this: GPtr<u8>) = thiscall 0x57B440 => unported;
    /// `CMenuManager::DrawFrontEnd()` (0x57C290)
    fn menu_draw_front_end(this: GPtr<u8>) = thiscall 0x57C290 => unported;
    /// `CAudioEngine::Service()` (0x507750)
    fn audio_engine_service(this: GPtr<u8>) = thiscall 0x507750 => unported;
    /// `CAudioEngine::DisplayRadioStationName()` (0x507030)
    fn audio_engine_display_radio_station_name(this: GPtr<u8>) = thiscall 0x507030 => unported;
    /// `CameraSize(RwCamera*, RwRect*, float viewWindow, float aspectRatio)` (0x72FC70)
    fn camera_size(cam: GPtr<u8>, rect: GPtr<u8>, view_window: u32, aspect: u32) = cdecl 0x72FC70 => unported;
    /// `CVisibilityPlugins::SetRenderWareCamera(RwCamera*)` (0x7328C0)
    fn vis_set_render_ware_camera(cam: GPtr<u8>) = cdecl 0x7328C0 => unported;
    /// `RwCameraClear(RwCamera*, RwRGBA*, int clearMode)` (0x7EE340)
    fn rw_camera_clear(cam: GPtr<u8>, colour: GPtr<u8>, mode: u32) -> GPtr<u8> = cdecl 0x7EE340 => unported;
    /// `RsCameraBeginUpdate(RwCamera*)` (0x619450)
    fn rs_camera_begin_update(cam: GPtr<u8>) -> i32 = cdecl 0x619450 => crate::platform::rs_camera_begin_update;
    /// `RsCameraShowRaster(RwCamera*)` (0x619440)
    fn rs_camera_show_raster(cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x619440 => unported;
    /// `RwCameraEndUpdate(RwCamera*)` (0x7EE180)
    fn rw_camera_end_update(cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EE180 => unported;
    /// `RwCameraBeginUpdate(RwCamera*)` (0x7EE190)
    fn rw_camera_begin_update(cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EE190 => unported;
    /// `RwCameraSetNearClipPlane(RwCamera*, float)` (0x7EE1D0)
    fn rw_camera_set_near_clip_plane(cam: GPtr<u8>, near: u32) -> GPtr<u8> = cdecl 0x7EE1D0 => unported;
    /// `RwCameraSetFarClipPlane(RwCamera*, float)` (0x7EE2A0)
    fn rw_camera_set_far_clip_plane(cam: GPtr<u8>, far: u32) -> GPtr<u8> = cdecl 0x7EE2A0 => unported;
    /// `RwRenderStateSet(state, value)` (RwEngineInstance 0xC97B24, slot 0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `DefinedState()` (0x734650)
    fn defined_state() = cdecl 0x734650 => crate::input::defined_state;
    /// `DoFade()` (0x53E600)
    fn do_fade() = cdecl 0x53E600 => unported;
    /// `SecuromStateDisplay()` (0x744AE0)
    fn securom_state_display() = cdecl 0x744AE0 => unported;
    /// `CHud::DrawAfterFade()` (0x58D490)
    fn hud_draw_after_fade() = cdecl 0x58D490 => unported;
    /// `CHud::Draw()` (0x58FAE0)
    fn hud_draw() = cdecl 0x58FAE0 => unported;
    /// `CMessages::Display(bool)` (0x69EFC0)
    fn messages_display(flag: u32) = cdecl 0x69EFC0 => unported;
    /// `CCredits::RenderCredits()` (0x5A87F0)
    fn credits_render_credits() = cdecl 0x5A87F0 => unported;
    /// `CDebug::DebugDisplayTextBuffer()` (0x532260)
    fn debug_display_text_buffer() = cdecl 0x532260 => unported;
    /// `FlushObrsPrintfs()` (0x734640)
    fn flush_obrs_printfs() = cdecl 0x734640 => unported;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&, const CRGBA&, const CRGBA&, const CRGBA&)` (0x727C10)
    fn sprite2d_draw_rect4(rect: GPtr<u8>, c1: GPtr<u8>, c2: GPtr<u8>, c3: GPtr<u8>, c4: GPtr<u8>) = cdecl 0x727C10 => unported;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727B60)
    fn sprite2d_draw_rect(rect: GPtr<u8>, c: GPtr<u8>) = cdecl 0x727B60 => unported;
    /// `CRGBA::CRGBA(r, g, b, a)` (0x7170C0, thiscall; bytes pushed as dwords)
    fn rgba_ctor(this: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `Light::SetLightsWithTimeOfDayColour(RpWorld*)` (0x7354E0)
    fn set_lights_with_time_of_day_colour(world: GPtr<u8>) = cdecl 0x7354E0 => unported;
    /// `CCamera::GetScreenFadeStatus()` (0x50AE20)
    fn camera_get_screen_fade_status(this: GPtr<u8>) -> i32 = thiscall 0x50AE20 => unported;
    /// `RsMouseSetPos(RwV2d*)` (0x6194A0)
    fn rs_mouse_set_pos(pos: GPtr<u8>) = cdecl 0x6194A0 => unported;
    /// `CRenderer::ConstructRenderList()` (0x5556E0)
    fn renderer_construct_render_list() = cdecl 0x5556E0 => unported;
    /// `CRenderer::PreRender()` (0x553910)
    fn renderer_pre_render() = cdecl 0x553910 => unported;
    /// `CWorld::ProcessPedsAfterPreRender()` (0x563430)
    fn world_process_peds_after_pre_render() = cdecl 0x563430 => unported;
    /// `CRealTimeShadowManager::Update()` (0x706AB0)
    fn real_time_shadow_man_update(this: GPtr<u8>) = thiscall 0x706AB0 => unported;
    /// `CMirrors::BeforeMainRender()` (0x727140)
    fn mirrors_before_main_render() = cdecl 0x727140 => unported;
    /// `DoRWStuffStartOfFrame_Horizon(...)` (0x53D7A0; seven dword pushes, bool in AL)
    fn do_rw_stuff_start_of_frame_horizon_o(a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, g: u32) -> u8 = cdecl 0x53D7A0 => crate::mvpb_app::do_rw_stuff_start_of_frame_horizon;
    /// `CMirrors::RenderMirrorBuffer()` (0x726090)
    fn mirrors_render_mirror_buffer() = cdecl 0x726090 => unported;
    /// `RenderScene()` (0x53DF40)
    fn render_scene_o() = cdecl 0x53DF40 => crate::mvpb_app::render_scene;
    /// `CVisibilityPlugins::RenderWeaponPedsForPC()` (0x732F30)
    fn vis_render_weapon_peds_for_pc() = cdecl 0x732F30 => unported;
    /// `CLinkList<CPed*>::Clear()` of `ms_weaponPedsForPC` (0x53E8D0)
    fn link_list_clear_o(this: GPtr<u8>) = thiscall 0x53E8D0 => crate::mvpb_app::link_list_clear;
    /// `RenderEffects()` (0x53E170)
    fn render_effects_o() = cdecl 0x53E170 => crate::mvpb_app::render_effects;
    /// `Render2dStuff()` (0x53E230)
    fn render_2d_stuff_o() = cdecl 0x53E230 => crate::mvpb_app::render_2d_stuff;
    /// `CGame::Process()` (0x53BEE0)
    fn game_process_o() = cdecl 0x53BEE0 => crate::mvpb_app::game_process;
    /// `CCamera::SetMotionBlurAlpha(int)` (0x50BF80)
    fn camera_set_motion_blur_alpha(this: GPtr<u8>, a: i32) = thiscall 0x50BF80 => unported;
    /// `CCamera::RenderMotionBlur()` (0x50B8F0)
    fn camera_render_motion_blur(this: GPtr<u8>) = thiscall 0x50B8F0 => unported;
    /// `CMatrix::Update()` (0x59BB60)
    fn matrix_update(this: GPtr<u8>) = thiscall 0x59BB60 => unported;
    /// `CClouds::RenderSkyPolys()` (0x714650)
    fn clouds_render_sky_polys() = cdecl 0x714650 => unported;
}

orig! {
    // ---- RenderScene
    fn moving_things_render_before_clouds() = cdecl 0x7178F0 => unported;
    fn clouds_render() = cdecl 0x713950 => unported;
    fn car_fx_renderer_pre_render_update() = cdecl 0x5D5B10 => unported;
    fn renderer_render_roads() = cdecl 0x553A10 => unported;
    fn coronas_render_reflections() = cdecl 0x6FB630 => unported;
    fn renderer_render_everything_bar_roads() = cdecl 0x553AA0 => unported;
    /// `BreakManager_c::Render(bool)` (0x59E6A0)
    fn break_man_render(this: GPtr<u8>, flag: u32) = thiscall 0x59E6A0 => unported;
    fn renderer_render_fading_in_underwater_entities() = cdecl 0x553220 => unported;
    fn water_level_render_water() = cdecl 0x6EF650 => unported;
    fn renderer_render_fading_in_entities() = cdecl 0x5531E0 => unported;
    fn shadows_update_static_shadows() = cdecl 0x707F40 => unported;
    fn shadows_render_static_shadows() = cdecl 0x708300 => unported;
    fn shadows_render_stored_shadows() = cdecl 0x70A960 => unported;
    fn plant_mgr_render() = cdecl 0x5DBAE0 => unported;
    fn clouds_render_bottom_from_height() = cdecl 0x7154B0 => unported;
    fn weather_render_rain_streaks() = cdecl 0x72AF70 => unported;
    fn coronas_render_sun_reflection() = cdecl 0x6FBAA0 => unported;
    fn stencil_shadows_render_stencil_shadows() = cdecl 0x7113B0 => unported;
    // ---- RenderEffects
    fn birds_render() = cdecl 0x712810 => unported;
    fn skidmarks_render() = cdecl 0x720640 => unported;
    fn ropes_render() = cdecl 0x556AE0 => unported;
    fn glass_render() = cdecl 0x71CE20 => unported;
    fn moving_things_render() = cdecl 0x717150 => unported;
    fn vis_render_really_draw_last_objects() = cdecl 0x733800 => unported;
    fn coronas_render() = cdecl 0x6FAEC0 => unported;
    /// `Fx_c::Render(RwCamera*, bool)` (0x49E650)
    fn fx_render(this: GPtr<u8>, cam: GPtr<u8>, flag: u32) = thiscall 0x49E650 => unported;
    fn water_cannons_render() = cdecl 0x729B30 => unported;
    fn water_level_render_water_fog() = cdecl 0x6E7760 => unported;
    fn clouds_moving_fog_render() = cdecl 0x716C90 => unported;
    fn clouds_volumetric_clouds_render() = cdecl 0x716380 => unported;
    fn heli_pre_search_light_cone() = cdecl 0x6C4650 => unported;
    fn heli_render_all_heli_search_lights() = cdecl 0x6C7C50 => unported;
    fn scripts_render_all_search_lights() = cdecl 0x493E30 => unported;
    fn heli_post_search_light_cone() = cdecl 0x6C46E0 => unported;
    fn weapon_effects_render() = cdecl 0x742CF0 => unported;
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPed::DrawTriangleForMouseRecruitPed()` (0x60BA80)
    fn ped_draw_triangle_for_mouse_recruit_ped(this: GPtr<u8>) = thiscall 0x60BA80 => unported;
    fn special_fx_render() = cdecl 0x726AD0 => unported;
    fn vehicle_recording_render() = cdecl 0x459F70 => unported;
    fn point_lights_render_fog_effect() = cdecl 0x7002D0 => unported;
    fn renderer_render_first_person_vehicle() = cdecl 0x553D00 => unported;
    fn post_effects_render() = cdecl 0x7046E0 => unported;
}

orig! {
    // ---- Render2dStuff
    fn replay_display() = cdecl 0x45C210 => unported;
    /// 0x60F8C0 (no arguments; called between `CReplay::Display` and `CPickups::RenderPickUpText`)
    fn fun_0060f8c0() = cdecl 0x60F8C0 => unported;
    fn pickups_render_pick_up_text() = cdecl 0x455000 => unported;
    fn camera_draw_borders_for_wide_screen(this: GPtr<u8>) = thiscall 0x514860 => unported;
    fn special_fx_render_2d_fxs() = cdecl 0x721660 => unported;
    fn onscreen_timer_process_for_display(this: GPtr<u8>) = thiscall 0x44CC20 => unported;
    fn darkel_draw_messages() = cdecl 0x43CEC0 => unported;
    fn garages_print_messages() = cdecl 0x447790 => unported;
    // ---- CGame::Process
    fn load_monitor_begin_frame(this: GPtr<u8>) = thiscall 0x53D030 => unported;
    fn load_monitor_end_frame(this: GPtr<u8>) = thiscall 0x53D0B0 => unported;
    fn streaming_update() = cdecl 0x40E670 => unported;
    fn cutscene_mgr_update() = cdecl 0x4D5D00 => unported;
    fn zones_update() = cdecl 0x572D10 => unported;
    fn cover_update() = cdecl 0x6997E0 => unported;
    /// `CAudioZones::Update(bool, CVector)` (0x5083C0; the vector by value)
    fn audio_zones_update(force: u32, x: u32, y: u32, z: u32) = cdecl 0x5083C0 => unported;
    fn cheat_do_cheats() = cdecl 0x439AF0 => unported;
    fn clock_update() = cdecl 0x52CF10 => unported;
    fn weather_update() = cdecl 0x72B850 => unported;
    fn scripts_process() = cdecl 0x46A000 => unported;
    fn collision_update() = cdecl 0x411E20 => unported;
    /// `CPathFind::UpdateStreaming(bool)` (0x450A60)
    fn path_find_update_streaming(this: GPtr<u8>, force: u32) = thiscall 0x450A60 => unported;
    fn train_update_trains() = cdecl 0x6F5900 => unported;
    fn heli_update_helis() = cdecl 0x6C79A0 => unported;
    fn darkel_update() = cdecl 0x43DAC0 => unported;
    fn skidmarks_update() = cdecl 0x7205C0 => unported;
    fn glass_update() = cdecl 0x71B0D0 => unported;
    fn wanted_update_each_frame() = cdecl 0x562360 => unported;
    fn creeping_fire_update() = cdecl 0x539CE0 => unported;
    fn set_pieces_update() = cdecl 0x49AA00 => unported;
    fn fire_manager_update(this: GPtr<u8>) = thiscall 0x53AF00 => unported;
    /// `CPopulation::Update(bool generate)` (0x616650)
    fn population_update(generate: u32) = cdecl 0x616650 => unported;
    fn weapon_update_weapons() = cdecl 0x73A360 => unported;
    fn car_generators_process() = cdecl 0x6F3F40 => unported;
    fn cranes_update_cranes() = cdecl 0x6F3FE0 => unported;
    fn clouds_update() = cdecl 0x712FF0 => unported;
    fn moving_things_update() = cdecl 0x7185B0 => unported;
    fn water_cannons_update() = cdecl 0x72A3C0 => unported;
    fn user_display_process() = cdecl 0x5720A0 => unported;
    fn replay_update() = cdecl 0x460500 => unported;
    fn world_process() = cdecl 0x5684A0 => unported;
    fn pickups_update() = cdecl 0x458DE0 => unported;
}

orig! {
    fn car_ctrl_prune_vehicles_of_interest() = cdecl 0x423F10 => unported;
    fn garages_update() = cdecl 0x44C8C0 => unported;
    fn entry_exit_manager_update() = cdecl 0x440D10 => unported;
    fn stunt_jump_manager_update() = cdecl 0x49C490 => unported;
    fn birds_update() = cdecl 0x712330 => unported;
    fn special_fx_update() = cdecl 0x726AA0 => unported;
    fn ropes_update() = cdecl 0x558D70 => unported;
    fn post_effects_update() = cdecl 0x7046A0 => unported;
    fn time_cycle_update() = cdecl 0x561760 => unported;
    fn pop_cycle_update() = cdecl 0x610BF0 => unported;
    fn interesting_events_scan_for_nearby_entities(this: GPtr<u8>) = thiscall 0x605A30 => unported;
    /// `CReplay::ShouldStandardCameraBeProcessed()` (0x45C440, bool in AL)
    fn replay_should_standard_camera_be_processed() -> u8 = cdecl 0x45C440 => unported;
    fn camera_process(this: GPtr<u8>) = thiscall 0x52B730 => unported;
    fn camera_process_fade(this: GPtr<u8>) = thiscall 0x50B5D0 => unported;
    fn cull_zones_update() = cdecl 0x72DEC0 => unported;
    fn game_logic_update() = cdecl 0x442AD0 => unported;
    fn gang_wars_update() = cdecl 0x446610 => unported;
    fn conversations_update() = cdecl 0x43C590 => unported;
    fn ped_to_player_conversations_update() = cdecl 0x43B0F0 => unported;
    fn bridge_update() = cdecl 0x41BC80 => unported;
    fn coronas_do_sun_and_moon() = cdecl 0x6FC5A0 => unported;
    fn coronas_update() = cdecl 0x6FADF0 => unported;
    fn shadows_update_permanent_shadows() = cdecl 0x70C950 => unported;
    /// `CPlantMgr::Update(const CVector& camPos)` (0x5DCFA0)
    fn plant_mgr_update(cam_pos: GPtr<u8>) = cdecl 0x5DCFA0 => unported;
    fn custom_building_renderer_update() = cdecl 0x5D8050 => unported;
    /// 0x710B50 (`CStencilShadows` light-direction setter; `const CVector&`)
    fn stencil_shadows_set_light(v: GPtr<u8>) = cdecl 0x710B50 => unported;
    /// `CStencilShadows::Process(const CVector& camPos)` (0x711D90)
    fn stencil_shadows_process(cam_pos: GPtr<u8>) = cdecl 0x711D90 => unported;
    fn car_ctrl_generate_random_cars() = cdecl 0x4341C0 => unported;
    fn road_blocks_generate_road_blocks() = cdecl 0x4629E0 => unported;
    fn car_ctrl_remove_distant_cars() = cdecl 0x42CD10 => unported;
    fn car_ctrl_remove_cars_if_the_pool_gets_full() = cdecl 0x4322B0 => unported;
    /// `Fx_c::Update(RwCamera*, float timeStep)` (0x49E640)
    fn fx_update(this: GPtr<u8>, cam: GPtr<u8>, dt: u32) = thiscall 0x49E640 => unported;
    /// `BreakManager_c::Update(float timeStep)` (0x59E670)
    fn break_man_update(this: GPtr<u8>, dt: u32) = thiscall 0x59E670 => unported;
    fn interior_man_update(this: GPtr<u8>) = thiscall 0x598F50 => unported;
    fn proc_obj_man_update(this: GPtr<u8>) = thiscall 0x5A3110 => unported;
    /// `WaterCreatureManager_c::Update(float timeStep)` (0x6E4F10)
    fn water_creature_man_update(this: GPtr<u8>, dt: u32) = thiscall 0x6E4F10 => unported;
    fn water_level_pre_render_water() = cdecl 0x6EB710 => unported;
}

fn camera() -> GPtr<u8> {
    g::<GPtr<u8>>(SCENE_CAMERA).read()
}

/// `CameraSize(Scene.m_pRwCamera, NULL, tan(CDraw::ms_fFOV * pi/360), CDraw::ms_fAspectRatio)`: FPTAN of
/// the x87 product, the tangent rounded to f32 by `fstp dword [esp]` (the `1.0` FPTAN pushes is popped).
fn camera_size_from_fov(cam: GPtr<u8>) {
    let view = view_window();
    camera_size(cam, GPtr::NULL, view, g::<u32>(DRAW_ASPECT).read());
}

fn view_window() -> u32 {
    let arg = X87::from_f32(g::<f32>(DRAW_FOV).read()).fmul_m(f32::from_bits(k::PI_OVER_360));
    // |arg| < 2^63 always for a float FOV times pi/360 that is finite; FPTAN leaves ST0 unchanged
    // (and pushes nothing) otherwise, which this model treats as the argument itself.
    let t = arg.fptan().map_or(arg, |(t, _one)| t);
    t.to_f32().to_bits()
}

/// The camera position `TheCamera.GetPosition()`: `&m_pMat->pos` (+0x30) when `m_pMat`, else the
/// placement position (`TheCamera` +0x4).
fn camera_position_ptr() -> GPtr<u8> {
    let m = g::<GPtr<u8>>(CAMERA_MAT).read();
    if m.is_null() { g(CAMERA_PLACEMENT_POS) } else { m.byte_add(0x30) }
}

/// `Game::FrontendIdle()` (0x53E770): `CDraw::CalculateAspectRatio` (result popped), `CTimer::Update`,
/// `CSprite2d::SetRecipNearClip`, `CSprite2d::InitPerFrame`, `CFont::InitPerFrame`, `CPad::UpdatePads`;
/// `m_bMenuActive = 1` when `m_bActivateMenuNextFrame`; `CMenuManager::Process`. Returns when
/// `RsGlobal.quit`. Then `AudioEngine.Service()`, `CameraSize(cam, NULL, tan(FOV*pi/360), aspect)`,
/// `CVisibilityPlugins::SetRenderWareCamera(cam)`, `RwCameraClear(cam, &gColourTop, rwCAMERACLEARZ)`;
/// when `RsCameraBeginUpdate(cam)`: `DefinedState`, `DrawFrontEnd` if the menu is active,
/// `SecuromStateDisplay`, `DoFade`, `CHud::DrawAfterFade`, `CMessages::Display(false)`,
/// `CFont::DrawFonts`, `CCredits::RenderCredits` when credits are going and the menu is not active,
/// `CFont::DrawFonts` again, `CDebug::DebugDisplayTextBuffer`, `FlushObrsPrintfs`,
/// `RwCameraEndUpdate(cam)`, `RsCameraShowRaster(cam)`.
#[hook(addr = 0x53E770, conv = cdecl, name = "Game::FrontendIdle", expect = "E8 AB 0C 1C 00 DD D8",
       unverified = "mvpb-core-1", route = "L3-only", hazards = "x87-transcendental",
       recs = "grove-walk", note = "frame orchestrator: every callee is a nested call; FPTAN view window")]
pub fn frontend_idle() {
    let _ = draw_calculate_aspect_ratio();
    timer_update();
    sprite2d_set_recip_near_clip();
    sprite2d_init_per_frame();
    font_init_per_frame();
    pad_update_pads();
    if g::<u8>(MENU_ACTIVATE_NEXT_FRAME).read() != 0 {
        g::<u8>(MENU_ACTIVE).write(1);
    }
    menu_process(g(MENU));
    if g::<u32>(RS_QUIT).read() != 0 {
        return;
    }
    audio_engine_service(g(AUDIO_ENGINE));
    camera_size_from_fov(camera());
    vis_set_render_ware_camera(camera());
    rw_camera_clear(camera(), g(COLOUR_TOP), 2);
    if rs_camera_begin_update(camera()) == 0 {
        return;
    }
    defined_state();
    if g::<u8>(MENU_ACTIVE).read() != 0 {
        menu_draw_front_end(g(MENU));
    }
    securom_state_display();
    do_fade();
    hud_draw_after_fade();
    messages_display(0);
    font_draw_fonts();
    if g::<u8>(CREDITS_GOING).read() != 0 && g::<u8>(MENU_ACTIVE).read() == 0 {
        credits_render_credits();
    }
    font_draw_fonts();
    debug_display_text_buffer();
    flush_obrs_printfs();
    rw_camera_end_update(camera());
    rs_camera_show_raster(camera());
}

/// `DoRWStuffStartOfFrame(int16 TopRed, TopGreen, TopBlue, BottomRed, BottomGreen, BottomBlue, Alpha)`
/// (0x53D690, cdecl, bool in AL): `CDraw::CalculateAspectRatio` (popped), `CFont::SetAlphaFade(255.0)`,
/// then the stack colours `top = (TopRed, TopGreen, TopBlue, Alpha)` and `bottom = (BottomRed,
/// BottomGreen, BottomBlue, Alpha)` (low bytes), `CameraSize(cam, NULL, tan(FOV*pi/360), aspect)`,
/// `SetRenderWareCamera(cam)`, `RwCameraClear(cam, &top, rwCAMERACLEARZ)`. False when
/// `RsCameraBeginUpdate` fails. Else `CSprite2d::InitPerFrame`, and when the 16-bit Alpha is non-zero
/// `CSprite2d::DrawRect(CRect(0, maxHeight, maxWidth, 0), bottom, bottom, top, top)` (the two
/// dimensions `fild` → f32); true.
#[hook(addr = 0x53D690, conv = cdecl, name = "App::DoRWStuffStartOfFrame", expect = "83 EC 18 53 E8 87 1D 1C 00",
       route = "split", hazards = "x87-transcendental",
       note = "called by the loading screens; bool in AL (callers test AL only)")]
pub fn do_rw_stuff_start_of_frame(
    top_red: u32,
    top_green: u32,
    top_blue: u32,
    bottom_red: u32,
    bottom_green: u32,
    bottom_blue: u32,
    alpha: u32,
) -> bool {
    let _ = draw_calculate_aspect_ratio();
    font_set_alpha_fade(0x437F_0000);
    let a = alpha as u8;
    let mut top = [top_red as u8, top_green as u8, top_blue as u8, a];
    let mut bottom = [bottom_red as u8, bottom_green as u8, bottom_blue as u8, a];
    camera_size_from_fov(camera());
    vis_set_render_ware_camera(camera());
    rw_camera_clear(camera(), GPtr::from_raw(top.as_mut_ptr()), 2);
    if rs_camera_begin_update(camera()) == 0 {
        return false;
    }
    sprite2d_init_per_frame();
    if alpha as u16 != 0 {
        let mut rect = [
            0.0f32,
            X87::from_i32(g::<i32>(RS_MAX_HEIGHT).read()).to_f32(),
            X87::from_i32(g::<i32>(RS_MAX_WIDTH).read()).to_f32(),
            0.0,
        ];
        let t = GPtr::from_raw(top.as_mut_ptr());
        let b = GPtr::from_raw(bottom.as_mut_ptr());
        sprite2d_draw_rect4(GPtr::from_raw(rect.as_mut_ptr()).cast(), b, b, t, t);
    }
    true
}

/// `DoRWStuffStartOfFrame_Horizon(...)` (0x53D7A0, cdecl, bool in AL; the colour arguments are unused):
/// `tan` argument `FOV * pi/360` stored as f32 first, `CDraw::CalculateAspectRatio` (popped), then
/// `CameraSize(cam, NULL, tan(arg), aspect)`, `SetRenderWareCamera(cam)`, `RwCameraClear(cam,
/// &gColourTop, rwCAMERACLEARZ | rwCAMERACLEARSTENCIL)`. False when `RsCameraBeginUpdate` fails; else
/// `DefinedState`, `TheCamera.m_mViewMatrix.Update()`, `CClouds::RenderSkyPolys`, true.
#[hook(addr = 0x53D7A0, conv = cdecl, name = "App::DoRWStuffStartOfFrame_Horizon", expect = "51 D9 05 38 50 8D 00",
       unverified = "mvpb-core-1", route = "L3-only", hazards = "x87-transcendental",
       recs = "grove-walk", note = "bool in AL (Game::Idle tests AL only)")]
pub fn do_rw_stuff_start_of_frame_horizon(_a: u32, _b: u32, _c: u32, _d: u32, _e: u32, _f: u32, _g: u32) -> u8 {
    let arg = X87::from_f32(g::<f32>(DRAW_FOV).read())
        .fmul_m(f32::from_bits(k::PI_OVER_360))
        .to_f32();
    let _ = draw_calculate_aspect_ratio();
    let a = X87::from_f32(arg);
    let view = a.fptan().map_or(a, |(t, _)| t).to_f32().to_bits();
    camera_size(camera(), GPtr::NULL, view, g::<u32>(DRAW_ASPECT).read());
    vis_set_render_ware_camera(camera());
    rw_camera_clear(camera(), g(COLOUR_TOP), 6);
    if rs_camera_begin_update(camera()) == 0 {
        return 0;
    }
    defined_state();
    matrix_update(g(CAMERA_VIEW_MATRIX));
    clouds_render_sky_polys();
    1
}

/// `Game::Idle(void* param)` (0x53E920): the frame limiter (spin until `cycles / cyclesPerMs -
/// g_LastTickTime >= 14`, unsigned, then `g_LastTickTime = cycles / cyclesPerMs`), `CTimer::Update`,
/// `CSprite2d::InitPerFrame`, `CFont::InitPerFrame`, `CPointLights::NumLights = 0`, `CGame::Process`,
/// `AudioEngine.Service()`, `SetLightsWithTimeOfDayColour(Scene.m_pRpWorld)`; nothing more without
/// `param`. Menu active or fade status 2 (fading in): `CalculateAspectRatio`, `CameraSize`,
/// `SetRenderWareCamera`, `RwCameraClear(cam, &gColourTop, rwCAMERACLEARZ)` and return unless
/// `RsCameraBeginUpdate`. Otherwise the world frame: mouse to the screen centre (`RsMouseSetPos`),
/// `ConstructRenderList`, `PreRender`, `ProcessPedsAfterPreRender`, `g_realTimeShadowMan.Update()`,
/// `CMirrors::BeforeMainRender`, `DoRWStuffStartOfFrame_Horizon` (all 255 and the fog colours set to
/// 255 during a lightning flash, else the time-cycle sky colours), return if it fails; `DefinedState`,
/// far clip and `fogPlane` from the time cycle, `RenderMirrorBuffer`, `RenderScene`,
/// `RenderWeaponPedsForPC`, `ms_weaponPedsForPC.Clear()`, `RenderEffects`, motion blur alpha 150 when
/// the blur type is 0 or 2 and the screen reduction > 0, `RenderMotionBlur`, `Render2dStuff`. Then
/// `DrawFrontEnd` if the menu is active, `RwRenderStateSet(rwRENDERSTATETEXTURERASTER, NULL)`,
/// `DoFade`, `CHud::DrawAfterFade`, `CMessages::Display(false)`, `CFont::DrawFonts`, the credits test
/// twice (`RenderCredits` each time it holds), `DebugDisplayTextBuffer`, `FlushObrsPrintfs`,
/// `RwCameraEndUpdate`, `RsCameraShowRaster`. SA:MP dispatch points (`crate::samp`, no-ops without a
/// registered handler) sit where samp retargets the calls: before `CGame::Process` and around `Render2dStuff`.
#[hook(addr = 0x53E920, conv = cdecl, name = "Game::Idle", expect = "83 EC 08 E8 58 31 02 00",
       unverified = "mvpb-core-1", route = "L3-only", hazards = "x87-transcendental",
       recs = "grove-walk", note = "frame orchestrator (determinism mode retargets the limiter call sites 0x53E930/0x53E94F; the port calls through them)")]
pub fn idle(param: GPtr<u8>) {
    let _ = timer_get_current_time_in_cycles();
    let _ = timer_get_cycles_per_millisecond();
    loop {
        let c = timer_cycles_idle_wait();
        let per = timer_get_cycles_per_millisecond();
        if (c / per).wrapping_sub(g::<u32>(LAST_TICK_TIME).read()) >= 14 {
            break;
        }
    }
    let c = timer_cycles_idle_last_tick();
    let per = timer_get_cycles_per_millisecond();
    g::<u32>(LAST_TICK_TIME).write(c / per);
    timer_update();
    sprite2d_init_per_frame();
    font_init_per_frame();
    g::<u32>(POINT_LIGHTS_NUM).write(0);
    crate::samp::frame(crate::samp::FramePhase::BeforeGameProcess);
    game_process_o();
    audio_engine_service(g(AUDIO_ENGINE));
    set_lights_with_time_of_day_colour(g::<GPtr<u8>>(0xC17038).read());
    if param.is_null() {
        return;
    }
    let world_frame = g::<u8>(MENU_ACTIVE).read() == 0 && camera_get_screen_fade_status(g(THE_CAMERA)) != 2;
    if world_frame {
        let mut pos = [
            X87::from_i32(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(f32::from_bits(k::HALF)).to_f32(),
            X87::from_i32(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(f32::from_bits(k::HALF)).to_f32(),
        ];
        rs_mouse_set_pos(GPtr::from_raw(pos.as_mut_ptr()).cast());
        renderer_construct_render_list();
        renderer_pre_render();
        world_process_peds_after_pre_render();
        real_time_shadow_man_update(g(G_REAL_TIME_SHADOW_MAN));
        mirrors_before_main_render();
        let ok = if g::<u8>(WEATHER_LIGHTNING_FLASH).read() != 0 {
            // fog red/green/blue (0xB7C4CA/CC/CE) = 255
            g::<u16>(0xB7C4CA).write(0xFF);
            g::<u16>(0xB7C4CC).write(0xFF);
            g::<u16>(0xB7C4CE).write(0xFF);
            do_rw_stuff_start_of_frame_horizon_o(0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF)
        } else {
            let s = |i: u32| g::<u16>(TC_SKY_TOP_RED + 2 * i).read() as u32;
            do_rw_stuff_start_of_frame_horizon_o(s(0), s(1), s(2), s(3), s(4), s(5), 0xFF)
        };
        if ok == 0 {
            return;
        }
        defined_state();
        rw_camera_set_far_clip_plane(camera(), g::<u32>(TC_FAR_CLIP).read());
        // `fld [fogStart]; fstp [cam+0x88]`: a plain float copy
        let fog = g::<f32>(TC_FOG_START).read();
        camera().byte_add(0x88).cast::<f32>().write(X87::from_f32(fog).to_f32());
        mirrors_render_mirror_buffer();
        render_scene_o();
        vis_render_weapon_peds_for_pc();
        link_list_clear_o(g(WEAPON_PEDS_FOR_PC));
        render_effects_o();
        let blur = g::<u32>(CAMERA_BLUR_TYPE).read();
        if (blur == 0 || blur == 2)
            && !X87::from_f32(g::<f32>(CAMERA_SCREEN_REDUCTION).read())
                .fcom(X87::from_f32(f32::from_bits(k::ZERO)))
                .test_any(0x41)
        {
            camera_set_motion_blur_alpha(g(THE_CAMERA), 0x96);
        }
        camera_render_motion_blur(g(THE_CAMERA));
        crate::samp::frame(crate::samp::FramePhase::Before2d);
        render_2d_stuff_o();
        crate::samp::frame(crate::samp::FramePhase::After2d);
    } else {
        let _ = draw_calculate_aspect_ratio();
        let view = view_window();
        camera_size(camera(), GPtr::NULL, view, g::<u32>(DRAW_ASPECT).read());
        vis_set_render_ware_camera(camera());
        rw_camera_clear(camera(), g(COLOUR_TOP), 2);
        if rs_camera_begin_update(camera()) == 0 {
            return;
        }
    }
    if g::<u8>(MENU_ACTIVE).read() != 0 {
        menu_draw_front_end(g(MENU));
    }
    rw_render_state_set(1, 0);
    do_fade();
    hud_draw_after_fade();
    messages_display(0);
    font_draw_fonts();
    if g::<u8>(CREDITS_GOING).read() != 0 {
        if g::<u8>(MENU_ACTIVE).read() == 0 {
            credits_render_credits();
        }
        if g::<u8>(CREDITS_GOING).read() != 0 && g::<u8>(MENU_ACTIVE).read() == 0 {
            credits_render_credits();
        }
    }
    debug_display_text_buffer();
    flush_obrs_printfs();
    crate::samp::frame(crate::samp::FramePhase::Present);
    rw_camera_end_update(camera());
    rs_camera_show_raster(camera());
}

/// `ms` elapsed by the cycle timer: `cycles / cyclesPerMs` (unsigned `div`).
fn now_ms() -> u32 {
    let c = timer_get_current_time_in_cycles();
    c / timer_get_cycles_per_millisecond()
}

/// `CGame::Process()` (0x53BEE0): `CPad::UpdatePads`, `CLoadMonitor::BeginFrame`, `CStreaming::Update`
/// timed in ms (`t`), `CCutsceneMgr::Update`, `CMenuManager::Process` unless a cutscene is being
/// processed or the code pause is set, `CTheZones::Update`, `CCover::Update`,
/// `CAudioZones::Update(false, camera position)`, `0xC81450 = 0`. Nothing more under the user or code
/// pause. Then the per-frame update chain in the original order (see the body); `CPopulation::Update`
/// gets `true` and is itself timed into `t` when `t < 4`, else `false`; `CTheCarGenerators::Process`
/// unless a cutscene runs; `CCranes::UpdateCranes` unless replaying; the `!bSkipProcessThisFrame`
/// group; `CInterestingEvents::ScanForNearbyEntities` when its flag bit 0 is set; `CCamera::Process`
/// or `ProcessFade` by `CReplay::ShouldStandardCameraBeProcessed`; `CGameLogic`/`CGangWars` unless
/// replaying; `CPlantMgr::Update(camPos)`, the stencil-shadow light vector `(2*X[i], 2*Y[i], -1.5)` of
/// the current time-cycle entry, `CStencilShadows::Process(camPos)`; car generation/removal unless
/// replaying (`GenerateRandomCars` only when `t < 4`); `g_fx.Update(TheCamera.m_pRwCamera, timeStep *
/// 0.02)`, `g_breakMan.Update(timeStep)`, the interior/proc-object managers,
/// `g_waterCreatureMan.Update(timeStep * 0.02)`; finally `CWaterLevel::PreRenderWater` (also reached
/// on the paused path).
#[hook(addr = 0x53BEE0, conv = cdecl, name = "CGame::Process", expect = "83 EC 0C 53 56 57 E8 E5",
       unverified = "mvpb-core-1", route = "L3-only", hazards = "none",
       recs = "grove-walk", note = "the update orchestrator; timing via the cycle timer (virtual clock in determinism mode)")]
pub fn game_process() {
    pad_update_pads();
    load_monitor_begin_frame(g(LOAD_MONITOR));
    let start = now_ms();
    streaming_update();
    let mut t = now_ms().wrapping_sub(start);
    cutscene_mgr_update();
    if g::<u8>(CUTSCENE_PROCESSING).read() == 0 && g::<u8>(TIMER_CODE_PAUSE).read() == 0 {
        menu_process(g(MENU));
    }
    zones_update();
    cover_update();
    let p = camera_position_ptr().cast::<u32>();
    audio_zones_update(0, p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read());
    g::<u32>(PROCESS_RESET_C81450).write(0);
    if g::<u8>(TIMER_USER_PAUSE).read() == 0 && g::<u8>(TIMER_CODE_PAUSE).read() == 0 {
        sprite2d_set_recip_near_clip();
        sprite2d_init_per_frame();
        font_init_per_frame();
        cheat_do_cheats();
        clock_update();
        weather_update();
        scripts_process();
        collision_update();
        path_find_update_streaming(g(THE_PATHS), 0);
        train_update_trains();
        heli_update_helis();
        darkel_update();
        skidmarks_update();
        glass_update();
        wanted_update_each_frame();
        creeping_fire_update();
        set_pieces_update();
        fire_manager_update(g(FIRE_MANAGER));
        if t < 4 {
            let s = now_ms();
            population_update(1);
            t = now_ms().wrapping_sub(s);
        } else {
            population_update(0);
        }
        weapon_update_weapons();
        if g::<u8>(CUTSCENE_RUNNING).read() == 0 {
            car_generators_process();
        }
        if g::<u8>(REPLAY_MODE).read() != 1 {
            cranes_update_cranes();
        }
        clouds_update();
        moving_things_update();
        water_cannons_update();
        user_display_process();
        replay_update();
        world_process();
        load_monitor_end_frame(g(LOAD_MONITOR));
        if g::<u8>(TIMER_SKIP_PROCESS).read() == 0 {
            pickups_update();
            car_ctrl_prune_vehicles_of_interest();
            garages_update();
            entry_exit_manager_update();
            stunt_jump_manager_update();
            birds_update();
            special_fx_update();
            ropes_update();
        }
        post_effects_update();
        time_cycle_update();
        pop_cycle_update();
        if g::<u8>(INTERESTING_EVENTS_FLAGS).read() & 1 != 0 {
            interesting_events_scan_for_nearby_entities(g(INTERESTING_EVENTS));
        }
        if replay_should_standard_camera_be_processed() != 0 {
            crate::samp::frame(crate::samp::FramePhase::CameraProcess);
            camera_process(g(THE_CAMERA));
            crate::samp::frame(crate::samp::FramePhase::AfterGameProcess);
        } else {
            camera_process_fade(g(THE_CAMERA));
        }
        cull_zones_update();
        if g::<u8>(REPLAY_MODE).read() != 1 {
            game_logic_update();
            gang_wars_update();
        }
        conversations_update();
        ped_to_player_conversations_update();
        bridge_update();
        coronas_do_sun_and_moon();
        coronas_update();
        shadows_update_permanent_shadows();
        plant_mgr_update(camera_position_ptr());
        custom_building_renderer_update();
        let i = g::<i32>(TC_CURRENT_STORED).read();
        let xv = X87::from_f32(g::<f32>(TC_TABLE_X).wrapping_index(i).read());
        let yv = X87::from_f32(g::<f32>(TC_TABLE_Y).wrapping_index(i).read());
        let mut light = [
            xv.fadd(xv).to_f32(),
            yv.fadd(yv).to_f32(),
            f32::from_bits(k::MINUS_1_5),
        ];
        stencil_shadows_set_light(GPtr::from_raw(light.as_mut_ptr()).cast());
        stencil_shadows_process(camera_position_ptr());
        if g::<u8>(REPLAY_MODE).read() != 1 {
            if t < 4 {
                car_ctrl_generate_random_cars();
            }
            road_blocks_generate_road_blocks();
            car_ctrl_remove_distant_cars();
            car_ctrl_remove_cars_if_the_pool_gets_full();
        }
        let step = || {
            X87::from_f32(g::<f32>(TIMER_TIME_STEP).read())
                .fmul_m(f32::from_bits(k::FIFTIETH))
                .to_f32()
                .to_bits()
        };
        fx_update(g(G_FX), g::<GPtr<u8>>(CAMERA_RW_CAMERA).read(), step());
        break_man_update(g(G_BREAK_MAN), g::<u32>(TIMER_TIME_STEP).read());
        interior_man_update(g(G_INTERIOR_MAN));
        proc_obj_man_update(g(G_PROC_OBJ_MAN));
        water_creature_man_update(g(G_WATER_CREATURE_MAN), step());
    }
    water_level_pre_render_water();
}

/// `RwRenderStateSet` state ids used here.
mod rs {
    pub const TEXTURERASTER: u32 = 1;
    pub const ZTESTENABLE: u32 = 6;
    pub const SHADEMODE: u32 = 7;
    pub const ZWRITEENABLE: u32 = 8;
    pub const SRCBLEND: u32 = 10;
    pub const DESTBLEND: u32 = 11;
    pub const VERTEXALPHAENABLE: u32 = 12;
    pub const FOGENABLE: u32 = 14;
    pub const CULLMODE: u32 = 20;
}

/// `Game::RenderScene()` (0x53DF40): `underWater = !(CWeather::UnderWaterness > 0.0)` (x87 compare;
/// NaN counts as not above); texture raster NULL, z-test/z-write/vertex-alpha off; `Render_BeforeClouds`
/// and `CClouds::Render` without a mirror; z-test/z-write on, Gouraud; `CCarFXRenderer::PreRenderUpdate`,
/// `RenderRoads`, `CCoronas::RenderReflections`, `RenderEverythingBarRoads`, `g_breakMan.Render(false)`,
/// `RenderFadingInUnderwaterEntities`; under water `RenderWater` between cull none/back;
/// `RenderFadingInEntities`; unless rendering a reflection: the near plane is pushed out by
/// `(far - near) * ((k*0.25 - k) * v + k)` (`k = 2.0 * 2^-24` from .data, `v = -Front.z` when
/// `Front.z <= 0`, else 0; all on the x87, the offset rounded to f32 and re-added on the x87) around
/// `UpdateStaticShadows`/`RenderStaticShadows`/`RenderStoredShadows` (End/Begin camera updates), then
/// restored. Then `g_breakMan.Render(true)`, `CPlantMgr::Render`, cull none; without a mirror
/// `RenderBottomFromHeight`, `RenderRainStreaks`, `RenderSunReflection`; when not under water
/// `RenderWater` between cull none/back; finally `CStencilShadows::RenderStencilShadows`.
#[hook(addr = 0x53DF40, conv = cdecl, name = "Game::RenderScene", expect = "D9 05 2C 13 C8 00",
       unverified = "mvpb-core-1", route = "L3-only", hazards = "none",
       recs = "grove-walk", note = "render orchestrator; the near-plane offset is x87 arithmetic (X87 model)")]
pub fn render_scene() {
    let uw = X87::from_f32(g::<f32>(WEATHER_UNDERWATERNESS).read());
    let not_above = uw.fcom(X87::from_f32(f32::from_bits(k::ZERO))).test_any(0x41);
    // bl = 1 when UnderWaterness > 0 (i.e. NOT "under water" in gta-reversed's inverted naming)
    let above = !not_above;
    rw_render_state_set(rs::TEXTURERASTER, 0);
    rw_render_state_set(rs::ZTESTENABLE, 0);
    rw_render_state_set(rs::ZWRITEENABLE, 0);
    rw_render_state_set(rs::VERTEXALPHAENABLE, 0);
    if g::<u32>(MIRRORS_TYPE).read() == 0 {
        moving_things_render_before_clouds();
        clouds_render();
    }
    rw_render_state_set(rs::ZTESTENABLE, 1);
    rw_render_state_set(rs::ZWRITEENABLE, 1);
    rw_render_state_set(rs::SHADEMODE, 2);
    car_fx_renderer_pre_render_update();
    renderer_render_roads();
    coronas_render_reflections();
    renderer_render_everything_bar_roads();
    break_man_render(g(G_BREAK_MAN), 0);
    renderer_render_fading_in_underwater_entities();
    if !above {
        rw_render_state_set(rs::CULLMODE, 1);
        water_level_render_water();
        rw_render_state_set(rs::CULLMODE, 2);
    }
    renderer_render_fading_in_entities();
    if g::<u8>(MIRRORS_RENDERING_REFLECTION).read() == 0 {
        let active = g::<u8>(CAMERA_ACTIVE_CAM).read() as u32;
        let kk = X87::from_f32(g::<f32>(NEAR_K1).read()).fmul_m(g::<f32>(NEAR_K2).read());
        let cam = camera();
        let near = cam.byte_add(0x80).cast::<f32>().read();
        let near_f = X87::from_f32(near).to_f32();
        let fz = X87::from_f32(g::<f32>(CAM_FRONT_Z + active * 0x238).read());
        let v = if fz.fcom(X87::from_f32(f32::from_bits(k::ZERO))).test_any(0x41) {
            fz.fchs()
        } else {
            X87::from_f32(f32::from_bits(k::ZERO))
        };
        let e = kk
            .fmul_m(f32::from_bits(k::QUARTER))
            .fsub(kk)
            .fmul(v)
            .fadd(kk);
        let span = X87::from_f32(cam.byte_add(0x84).cast::<f32>().read()).fsub_m(near_f);
        let off = span.fmul(e).to_f32();
        rw_camera_end_update(cam);
        let new_near = X87::from_f32(off).fadd_m(near_f).to_f32();
        rw_camera_set_near_clip_plane(camera(), new_near.to_bits());
        rw_camera_begin_update(camera());
        shadows_update_static_shadows();
        shadows_render_static_shadows();
        shadows_render_stored_shadows();
        rw_camera_end_update(camera());
        rw_camera_set_near_clip_plane(camera(), near_f.to_bits());
        rw_camera_begin_update(camera());
    }
    break_man_render(g(G_BREAK_MAN), 1);
    plant_mgr_render();
    rw_render_state_set(rs::CULLMODE, 1);
    if g::<u32>(MIRRORS_TYPE).read() == 0 {
        clouds_render_bottom_from_height();
        weather_render_rain_streaks();
        coronas_render_sun_reflection();
    }
    if above {
        rw_render_state_set(rs::CULLMODE, 1);
        water_level_render_water();
        rw_render_state_set(rs::CULLMODE, 2);
    }
    stencil_shadows_render_stencil_shadows();
}

/// `Game::RenderEffects()` (0x53E170): `CBirds::Render`, `CSkidmarks::Render`, `CRopes::Render`,
/// `CGlass::Render`, `CMovingThings::Render`, `RenderReallyDrawLastObjects`, `CCoronas::Render`,
/// `g_fx.Render(TheCamera.m_pRwCamera, false)`, `CWaterCannons::Render`, `RenderWaterFog`,
/// `MovingFogRender`, `VolumetricCloudsRender`; the search-light block when `CHeli::NumberOfSearchLights`
/// or the 16-bit `CTheScripts::NumberOfScriptSearchLights` is non-zero; `CWeaponEffects::Render`;
/// unless replaying, `FindPlayerPed(-1)->DrawTriangleForMouseRecruitPed()` when pad 0's
/// `DisablePlayerControls` (+0x10E, int16) is 0; `CSpecialFX::Render`, `CVehicleRecording::Render`,
/// `RenderFogEffect`, `RenderFirstPersonVehicle`, `CPostEffects::Render` (tail call).
#[hook(addr = 0x53E170, conv = cdecl, name = "Game::RenderEffects", expect = "E8 9B 46 1D 00",
       unverified = "mvpb-core-1", route = "L3-only", hazards = "none", recs = "grove-walk")]
pub fn render_effects() {
    birds_render();
    skidmarks_render();
    ropes_render();
    glass_render();
    moving_things_render();
    vis_render_really_draw_last_objects();
    coronas_render();
    fx_render(g(G_FX), g::<GPtr<u8>>(CAMERA_RW_CAMERA).read(), 0);
    water_cannons_render();
    water_level_render_water_fog();
    clouds_moving_fog_render();
    clouds_volumetric_clouds_render();
    if g::<u32>(HELI_NUM_SEARCHLIGHTS).read() != 0 || g::<u16>(SCRIPT_NUM_SEARCHLIGHTS).read() != 0 {
        heli_pre_search_light_cone();
        heli_render_all_heli_search_lights();
        scripts_render_all_search_lights();
        heli_post_search_light_cone();
    }
    weapon_effects_render();
    if g::<u8>(REPLAY_MODE).read() != 1 && pad_get_pad(0).byte_add(0x10E).cast::<u16>().read() == 0 {
        ped_draw_triangle_for_mouse_recruit_ped(find_player_ped(-1));
    }
    special_fx_render();
    vehicle_recording_render();
    point_lights_render_fog_effect();
    renderer_render_first_person_vehicle();
    post_effects_render();
}

/// `Game::Render2dStuff()` (0x53E230): render states (z-test/z-write off, vertex alpha on, src/dest
/// blend SRCALPHA/INVSRCALPHA, fog off, cull none), `CReplay::Display`, 0x60F8C0,
/// `CPickups::RenderPickUpText`, `DrawBordersForWideScreen` when the camera's widescreen is on and the
/// menu's is off. The zoom box: weapon type = the player's `m_aWeapons[m_nActiveWeaponSlot].m_eWeaponType`
/// (+0x5A0 + slot*0x1C); camera mode (int16) 7/0x27/8/0x28/0x2E/0x2D = first person; for weapon 0x22
/// (sniper) or 0x13 (rocket) in first person four black rectangles (`CRGBA(0,0,0,255)`): rocket
/// `h/2 - h*(1/448)*180` and `h/2 + h*(1/448)*170`, sniper 210/210; then the sides
/// `w/2 ∓ w*(1/640)*210` (all x87, the halves `(n - (n<0)) >> 1` via `cdq; sub; sar`, `fiadd`).
/// Then `DisplayRadioStationName`, `CHud::Draw`, `Render2DFXs`, `OnscnTimer.ProcessForDisplay`,
/// `CMessages::Display(true)`, `CDarkel::DrawMessages`, `CGarages::PrintMessages`, `CFont::DrawFonts`.
///
/// Kept original bug: without a player ped the weapon type is an uninitialised local of the
/// original's frame. The port uses 0 (unarmed) there, so the zoom box can only differ when no
/// player exists, the residue happens to be 0x22/0x13 and the camera is in a first-person mode.
#[hook(addr = 0x53E230, conv = cdecl, name = "Game::Render2dStuff", expect = "A1 24 7B C9 00 83 EC 18",
       unverified = "mvpb-core-1", route = "L3-only", hazards = "uninit-locals",
       recs = "grove-walk", note = "weapon type is uninitialised stack ([esp+8] at 0x53E2DB = entry ESP - 0x14) without a player ped; port uses 0")]
pub fn render_2d_stuff() {
    rw_render_state_set(rs::ZTESTENABLE, 0);
    rw_render_state_set(rs::ZWRITEENABLE, 0);
    rw_render_state_set(rs::VERTEXALPHAENABLE, 1);
    rw_render_state_set(rs::SRCBLEND, 5);
    rw_render_state_set(rs::DESTBLEND, 6);
    rw_render_state_set(rs::FOGENABLE, 0);
    rw_render_state_set(rs::CULLMODE, 1);
    replay_display();
    fun_0060f8c0();
    pickups_render_pick_up_text();
    if g::<u8>(CAMERA_WIDESCREEN_ON).read() != 0 && g::<u8>(MENU_WIDESCREEN_ON).read() == 0 {
        camera_draw_borders_for_wide_screen(g(THE_CAMERA));
    }
    let ped = find_player_ped(-1);
    let weapon = if ped.is_null() {
        0
    } else {
        let slot = ped.byte_add(0x718).cast::<i8>().read() as i32;
        ped.byte_add(0x5A0 + slot as isize * 0x1C).cast::<u32>().read()
    };
    let active = g::<u8>(CAMERA_ACTIVE_CAM).read() as u32;
    let mode = g::<u16>(CAM_MODE + active * 0x238).read();
    let first_person = matches!(mode, 7 | 0x27 | 8 | 0x28 | 0x2E | 0x2D);
    if (weapon == 0x22 || weapon == 0x13) && first_person {
        let mut black = [0u8; 4];
        rgba_ctor(GPtr::from_raw(black.as_mut_ptr()), 0, 0, 0, 0xFF);
        let bl = GPtr::from_raw(black.as_mut_ptr());
        let half = |n: i32| (n - (n >> 31)) >> 1;
        let w = g::<i32>(RS_MAX_WIDTH).read();
        let h = g::<i32>(RS_MAX_HEIGHT).read();
        let (k_top, k_bottom) = if weapon == 0x13 { (k::K180, k::K170) } else { (k::K210, k::K210) };
        // top band: CRect(0, h/2 - h*(1/448)*k_top, w, 0)
        let top = X87::from_i32(half(h))
            .fsub(X87::from_i32(h).fmul_m(f32::from_bits(k::INV_448)).fmul_m(f32::from_bits(k_top)));
        let mut r = [0.0f32, top.to_f32(), X87::from_i32(w).to_f32(), 0.0];
        sprite2d_draw_rect(GPtr::from_raw(r.as_mut_ptr()).cast(), bl);
        // bottom band: CRect(0, h, w, h/2 + h*(1/448)*k_bottom)
        let hh = X87::from_i32(h);
        let b = X87::from_f32(f32::from_bits(k::INV_448))
            .fmul(hh)
            .fmul_m(f32::from_bits(k_bottom))
            .fadd(X87::from_i32(half(h)));
        let mut r = [0.0f32, hh.to_f32(), X87::from_i32(w).to_f32(), b.to_f32()];
        sprite2d_draw_rect(GPtr::from_raw(r.as_mut_ptr()).cast(), bl);
        // left: CRect(0, h, w/2 - w*(1/640)*210, 0)
        let l = X87::from_i32(half(w))
            .fsub(X87::from_i32(w).fmul_m(f32::from_bits(k::INV_640)).fmul_m(f32::from_bits(k::K210)));
        let mut r = [0.0f32, X87::from_i32(h).to_f32(), l.to_f32(), 0.0];
        sprite2d_draw_rect(GPtr::from_raw(r.as_mut_ptr()).cast(), bl);
        // right: CRect(w/2 + w*(1/640)*210, h, w, 0)
        let ww = X87::from_i32(w);
        let rr = X87::from_f32(f32::from_bits(k::INV_640))
            .fmul(ww)
            .fmul_m(f32::from_bits(k::K210))
            .fadd(X87::from_i32(half(w)));
        let mut r = [rr.to_f32(), X87::from_i32(h).to_f32(), ww.to_f32(), 0.0];
        sprite2d_draw_rect(GPtr::from_raw(r.as_mut_ptr()).cast(), bl);
    }
    audio_engine_display_radio_station_name(g(AUDIO_ENGINE));
    hud_draw();
    special_fx_render_2d_fxs();
    onscreen_timer_process_for_display(g(ONSCREEN_TIMER));
    messages_display(1);
    darkel_draw_messages();
    garages_print_messages();
    font_draw_fonts();
}

/// `CLinkList<T>::Clear()` (0x53E8D0, thiscall; here on `CVisibilityPlugins::ms_weaponPedsForPC`):
/// while `usedListHead.next` (+0x8) is not `&usedListTail` (+0xC), unlink that node and insert it after
/// `freeListHead` (+0x18): `node.next.prev = node.prev`... exactly the original's six stores per node.
#[hook(addr = 0x53E8D0, conv = thiscall, name = "FUN_0053e8d0", this = "CLinkList<void>", expect = "8B 41 08 56 8D 71 0C",
       route = "L2", hazards = "heap-writes",
       note = "list nodes live in the list's own heap node array (l2-extras: the node array)")]
pub fn link_list_clear(this: GPtr<u8>) {
    let used_tail = this.byte_add(0xC);
    let free_head = this.byte_add(0x18);
    let ptr = |p: GPtr<u8>, off: isize| p.byte_add(off).cast::<GPtr<u8>>();
    let mut n = ptr(this, 8).read();
    while n != used_tail {
        let next = ptr(n, 8).read();
        let prev = ptr(n, 4).read();
        ptr(next, 4).write(prev);
        let prev = ptr(n, 4).read();
        let next = ptr(n, 8).read();
        ptr(prev, 8).write(next);
        let fnext = ptr(free_head, 8).read();
        ptr(n, 8).write(fnext);
        let fnext = ptr(free_head, 8).read();
        ptr(fnext, 4).write(n);
        ptr(n, 4).write(free_head);
        ptr(free_head, 8).write(n);
        n = ptr(this, 8).read();
    }
}

/// 0x4082A0 (stub → .HOODLUM 0x1566860): `CPool<CPed>::GetAt(idx)` (slot 0x7C4): NULL when bit 7 of
/// `map[idx]` is set (`cmp byte, 0; jns`), else `objects + idx * 0x7C4`.
#[hook(addr = 0x4082A0, conv = thiscall, name = "thunk_FUN_01566860", this = "CPool<int32>", expect = "E9 BB E5 15 01",
       route = "L2", hazards = "none")]
pub fn ped_pool_get_at(this: PoolPtr<Slot<0x7C4>>, idx: i32) -> GPtr<Slot<0x7C4>> {
    pool::pool_at(this, idx)
}

/// 0x4A8EC0 `List_c::GetNumItems()`: `m_count` (+0x8). Not hooked: the body is 4 bytes
/// (`mov eax,[ecx+8]; ret`), shorter than the 5-byte entry patch.
pub fn list_get_num_items(this: GPtr<u8>) -> u32 {
    this.byte_add(8).cast::<u32>().read()
}

/// 0x4048E0 `CPool<CVehicle>::GetAtRef(ref)` (slot 0xA18; the `imul` lives in the protection-relocated
/// piece 0x40210A): `idx = ref >> 8` (arithmetic), NULL unless `map[idx] == (u8)ref`. No bounds check.
#[hook(addr = 0x4048E0, conv = thiscall, name = "FUN_004048e0", this = "CPool<int32>", expect = "8B 54 24 04 53 56",
       route = "L2", hazards = "none")]
pub fn vehicle_pool_get_at_ref(this: PoolPtr<Slot<0xA18>>, handle: i32) -> GPtr<Slot<0xA18>> {
    pool::pool_at_ref(this, handle)
}

/// 0x465040 `CPool<CObject>::GetAtRef(ref)` (slot 0x19C), as 0x4048E0.
#[hook(addr = 0x465040, conv = thiscall, name = "FUN_00465040", this = "CPool<int32>", expect = "8B 54 24 04 53 56",
       route = "L2", hazards = "none")]
pub fn object_pool_get_at_ref(this: PoolPtr<Slot<0x19C>>, handle: i32) -> GPtr<Slot<0x19C>> {
    pool::pool_at_ref(this, handle)
}

/// 0x552B50 `CPool<CQuadTreeNode>::New()` (slot 0x28; the `lea eax,[eax+eax*4]; lea eax,[ecx+eax*8]`
/// tail): the shared `CPool::New` algorithm.
#[hook(addr = 0x552B50, conv = thiscall, name = "FUN_00552b50", this = "CPool<int32>", expect = "8B 51 08 56 32 C0",
       route = "L2", hazards = "heap-writes")]
pub fn quad_tree_node_pool_new(this: PoolPtr<Slot<0x28>>) -> GPtr<Slot<0x28>> {
    pool::pool_new(this)
}

/// This module's registrations.
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_FRONTEND_IDLE,
    HOOK_DO_RW_STUFF_START_OF_FRAME,
    HOOK_DO_RW_STUFF_START_OF_FRAME_HORIZON,
    HOOK_IDLE,
    HOOK_GAME_PROCESS,
    HOOK_RENDER_SCENE,
    HOOK_RENDER_EFFECTS,
    HOOK_RENDER_2D_STUFF,
    HOOK_LINK_LIST_CLEAR,
    HOOK_PED_POOL_GET_AT,
    HOOK_VEHICLE_POOL_GET_AT_REF,
    HOOK_OBJECT_POOL_GET_AT_REF,
    HOOK_QUAD_TREE_NODE_POOL_NEW,
];
