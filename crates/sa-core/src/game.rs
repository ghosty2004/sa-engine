//! CORE ports: `CGame` / `Game::` (app_game) leaves and short sequences (PORT-CORE).
//!
//! Ported from the 1.0 US bytes (gta-reversed `Game.cpp`, `app/app_game.cpp`, `Draw.cpp` as reading
//! aid). The big orchestrators (`CGame::Initialise`, `Init1/2/3`, `Process`, `Shutdown`, `Game::Idle`,
//! `RenderScene`, …) are deferred until the subsystems they sequence are ported; see
//! `docs/arch/core-port.md`. Every callee goes through `orig!` (phase A: the original address, so its
//! own hook decides; phases B/C: `unported` until the callee's subsystem exists).

use crate::mem::{GBool, GPtr};
use crate::platform::{RS_MAXIMUM_HEIGHT, RS_MAXIMUM_WIDTH, RS_QUIT, SCENE_CAMERA};
use crate::{crt, globals, hook, orig};
use sa_math::x87::X87;

globals! {
    list GLOBALS;
    /// `CGame::currArea` (eAreaCodes, int32)
    pub static CURR_AREA @ 0xB72914: i32 = 0;
    /// `CGame::version_number` (int16; InitialiseGame stores 78)
    pub static VERSION_NUMBER @ 0xB72C68: i16 = 0;
    /// `version_name` (char[64], ValidateVersion copies the peds.col version text here)
    pub static VERSION_NAME @ 0xB72C28: [u8; 64] = [0; 64];
    /// `FrontEndMenuManager.m_bActivateMenuNextFrame` (+0x33)
    pub static MENU_ACTIVATE_NEXT_FRAME @ 0xBA677B: GBool = GBool::FALSE;
    /// `FrontEndMenuManager.m_bMenuActive` (+0x5C)
    pub static MENU_ACTIVE @ 0xBA67A4: GBool = GBool::FALSE;
    /// `FrontEndMenuManager.m_bStartGameLoading` (+0x5D)
    pub static MENU_START_GAME_LOADING @ 0xBA67A5: GBool = GBool::FALSE;
    /// `FrontEndMenuManager.m_bLoadingData` (+0x60)
    pub static MENU_LOADING_DATA @ 0xBA67A8: GBool = GBool::FALSE;
    /// `FrontEndMenuManager` +0xE9 (byte tested by `CGame::InitAfterLostFocus`; no gta-reversed name)
    pub static MENU_FIELD_E9 @ 0xBA6831: GBool = GBool::FALSE;
    /// `s_PrevD3DDeviceRestoreCallback` (the RW D3D9 restore callback chained by the game)
    pub static PREV_D3D_RESTORE_CALLBACK @ 0xBD0114: u32 = 0;
    /// `g_realTimeShadowMan.m_bInitialised`
    pub static RT_SHADOW_INITIALISED @ 0xC40350: GBool = GBool::FALSE;
    /// `g_realTimeShadowMan.m_bNeedsReinit`
    pub static RT_SHADOW_NEEDS_REINIT @ 0xC40351: GBool = GBool::FALSE;
    /// `CMirrors::d3dRestored`
    pub static MIRRORS_D3D_RESTORED @ 0xC7C729: GBool = GBool::FALSE;
    /// gamma-ramp flag tested by `Game::RwTerminate` (gta-reversed `ResetGammaWhenExiting`)
    pub static GAMMA_CHANGED @ 0xC920C8: GBool = GBool::FALSE;
    /// `JustLoadedDontFadeInYet`
    pub static JUST_LOADED_DONT_FADE_IN_YET @ 0xC16EDC: GBool = GBool::FALSE;
    /// `StillToFadeOut`
    pub static STILL_TO_FADE_OUT @ 0xC16EDD: GBool = GBool::FALSE;
    /// `FadeTimer`
    pub static FADE_TIMER @ 0xC16EE0: u32 = 0;
    /// `FadeOutTime` (.data: 1750 ms)
    pub static FADE_OUT_TIME @ 0x8D2BD4: u32 = 1750;
    /// `CDraw::FadeRed`, `FadeGreen`, `FadeBlue`, `FadeValue`
    pub static FADE_RGBV @ 0xC3EFA8: [u8; 4] = [0; 4];
    /// `TheCamera.m_bFadeTargetIsSplashScreen` (TheCamera 0xB6F028 + 0x55)
    pub static CAMERA_FADE_TARGET_IS_SPLASH @ 0xB6F07D: GBool = GBool::FALSE;
    /// pad 0 button word read by `NoDiskInDriveMessage` (new state)
    pub static PAD0_NEW_CROSS @ 0xB733A8: i16 = 0;
    /// pad 0 button word read by `NoDiskInDriveMessage` (old state)
    pub static PAD0_OLD_CROSS @ 0xB73138: i16 = 0;
    // .rdata strings the ports pass by address
    /// "models\\coll\\peds.col"
    pub static STR_PEDS_COL @ 0x86A51C: [u8; 21] = *b"models\\coll\\peds.col\0";
    /// "rb"
    pub static STR_RB @ 0x859C6C: [u8; 3] = *b"rb\0";
    /// "grandtheftauto3"
    pub static STR_GTA3 @ 0x86A50C: [u8; 16] = *b"grandtheftauto3\0";
    /// "Invalid version"
    pub static STR_INVALID_VERSION @ 0x86A4FC: [u8; 16] = *b"Invalid version\0";
    /// "Into TheGame!!!\n"
    pub static STR_INTO_THE_GAME @ 0x863B20: [u8; 17] = *b"Into TheGame!!!\n\0";
    /// "DATA\\GTA.DAT"
    pub static STR_GTA_DAT @ 0x863B10: [u8; 13] = *b"DATA\\GTA.DAT\0";
    /// "Starting Game"
    pub static STR_STARTING_GAME @ 0x863B00: [u8; 14] = *b"Starting Game\0";
    /// "NO_PCCD" (GXT key)
    pub static STR_NO_PCCD @ 0x86566C: [u8; 8] = *b"NO_PCCD\0";
    /// `Scene.world` (RpWorld*)
    pub static SCENE_WORLD @ 0xC17038: GPtr<u8> = GPtr::NULL;
    /// `CGame::m_pWorkingMatrix1` (RwMatrix*)
    pub static WORKING_MATRIX_1 @ 0xB72920: GPtr<u8> = GPtr::NULL;
    /// `CGame::m_pWorkingMatrix2` (RwMatrix*)
    pub static WORKING_MATRIX_2 @ 0xB72924: GPtr<u8> = GPtr::NULL;
    // D3DResourceSystem state written by `D3DResourceSystem::Init` (no gta-reversed names)
    /// 0xC87C60 (dword, Init stores 1)
    pub static D3D_RES_C87C60 @ 0xC87C60: u32 = 0;
    /// 0x8D6084 (byte, .data 1; Init stores 1)
    pub static D3D_RES_BUFFERING @ 0x8D6084: u8 = 1;
    /// 0xC87C68
    pub static D3D_RES_C87C68 @ 0xC87C68: u32 = 0;
    /// 0xC87C6C
    pub static D3D_RES_C87C6C @ 0xC87C6C: u32 = 0;
    /// 0xC87C70 (Init stores -1)
    pub static D3D_RES_C87C70 @ 0xC87C70: i32 = 0;
    /// 0xC87C74 (Init stores 0x10)
    pub static D3D_RES_C87C74 @ 0xC87C74: u32 = 0;
    /// 0xC87C78
    pub static D3D_RES_C87C78 @ 0xC87C78: u32 = 0;
    /// 0xC87C7C
    pub static D3D_RES_C87C7C @ 0xC87C7C: u32 = 0;
    /// 0xC87C80 (the `new(0x40)` array)
    pub static D3D_RES_C87C80 @ 0xC87C80: GPtr<u8> = GPtr::NULL;
    /// 0xC87E48: 17 lists of 6 dwords
    pub static D3D_RES_LISTS @ 0xC87E48: [u32; 102] = [0; 102];
}

/// `TheCamera` (CCamera, thiscall `this`).
pub const THE_CAMERA: u32 = 0xB6F028;
/// `FrontEndMenuManager` (CMenuManager).
pub const FRONTEND_MENU_MANAGER: u32 = 0xBA6748;
/// `ThePaths` (CPathFind).
pub const THE_PATHS: u32 = 0x96F050;
/// `TheText` (CText).
pub const THE_TEXT: u32 = 0xC1B340;
/// `AudioEngine` (CAudioEngine).
pub const AUDIO_ENGINE: u32 = 0xB6BC90;
/// `gammaTable`-style ramp passed to `IDirect3DDevice9::SetGammaRamp` by `Game::RwTerminate`.
pub const SAVED_GAMMA_RAMP: u32 = 0xC8D4C8;

fn at<T>(a: u32) -> GPtr<T> {
    GPtr::from_addr(a as usize)
}

/// Typed calls the ports make (all into other subsystems or not yet ported).
pub mod call {
    use super::*;
    orig! {
        pub fn memory_mgr_init() = cdecl 0x72F3B0 => unported;
        pub fn localisation_initialise() = cdecl 0x56D180 => unported;
        pub fn file_mgr_initialise() = cdecl 0x5386F0 => unported;
        pub fn cd_stream_init(n: i32) = cdecl 0x406B70 => unported;
        pub fn pad_initialise() = cdecl 0x541D90 => unported;
        pub fn path_find_shutdown(this: GPtr<u8>) = thiscall 0x450950 => unported;
        pub fn temp_col_models_shutdown() = cdecl 0x41B360 => unported;
        pub fn txd_store_shutdown() = cdecl 0x732000 => unported;
        pub fn ped_stats_shutdown() = cdecl 0x608850 => unported;
        pub fn text_unload(this: GPtr<u8>, unload_mission: u32) = thiscall 0x69FF20 => unported;
        pub fn cd_stream_shutdown() = cdecl 0x406370 => unported;
        pub fn find_player_ped(player: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
        pub fn tidy_up_d3d_textures(n: u32) = cdecl 0x7305E0 => super::tidy_up_d3d_textures;
        pub fn tidy_up_d3d_index_buffers(n: u32) = cdecl 0x730740 => super::tidy_up_d3d_index_buffers;
        /// `IUnknown::Release()` (COM slot 8)
        pub fn com_release(obj: GPtr<u8>) -> u32 = stdcall vslot 0x8 => unported;
        pub fn game_shutdown_render_ware() = cdecl 0x53BB80 => unported;
        pub fn rw_d3d9_get_current_d3d_device() -> GPtr<u32> = cdecl 0x7F9D50 => unported;
        /// `IDirect3DDevice9::SetGammaRamp(swapchain, flags, ramp)` (COM slot 0x54)
        pub fn d3d_device_set_gamma_ramp(dev: GPtr<u32>, swapchain: u32, flags: u32, ramp: GPtr<u8>) = stdcall vslot 0x54 => unported;
        pub fn rs_rw_terminate() = cdecl 0x6195E0 => unported;
        /// CRT `printf` (one argument at its only call site here)
        pub fn printf(fmt: GPtr<u8>) -> i32 = cdecl 0x821982 => unported;
        pub fn timer_initialise() = cdecl 0x5617E0 => crate::timer::initialise;
        pub fn game_initialise(dat_file: GPtr<u8>) -> u8 = cdecl 0x53BC80 => unported;
        pub fn loading_screen_set_loading_bar_msg(msg: GPtr<u8>, msg2: GPtr<u8>) = cdecl 0x590240 => unported;
        pub fn loading_screen_new_chunk_loaded() = cdecl 0x590D00 => unported;
        pub fn menu_draw_front_end(this: GPtr<u8>) = thiscall 0x57C290 => unported;
        pub fn audio_engine_restart(this: GPtr<u8>) = thiscall 0x506DB0 => unported;
        /// `CCamera::Fade(float time, short direction)` (direction pushed as a dword)
        pub fn camera_fade(this: GPtr<u8>, time: f32, direction: u32) = thiscall 0x50AC20 => unported;
        pub fn camera_process_fade(this: GPtr<u8>) = thiscall 0x50B5D0 => unported;
        pub fn camera_process_music_fade(this: GPtr<u8>) = thiscall 0x50B6D0 => unported;
        /// `CCamera::SetFadeColour(r, g, b)` (bytes pushed as dwords)
        pub fn camera_set_fade_colour(this: GPtr<u8>, r: u32, g: u32, b: u32) = thiscall 0x50BF00 => unported;
        /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)`
        pub fn sprite2d_draw_rect(rect: GPtr<[f32; 4]>, colour: GPtr<[u8; 4]>) = cdecl 0x727B60 => unported;
        pub fn file_mgr_open_file(path: GPtr<u8>, mode: GPtr<u8>) -> u32 = cdecl 0x538900 => unported;
        pub fn file_mgr_seek(file: u32, offset: i32, origin: i32) -> i32 = cdecl 0x538990 => unported;
        pub fn file_mgr_read(file: u32, buf: GPtr<u8>, size: i32) -> i32 = cdecl 0x538950 => unported;
        pub fn file_mgr_close_file(file: u32) -> i32 = cdecl 0x5389D0 => unported;
        /// CRT `strncmp`
        pub fn strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
        /// CRT `strncpy`
        pub fn strncpy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
        /// `CLoadingScreen::LoadingScreen(msg1, msg2, splash)` (address 0x53DED0)
        pub fn loading_screen(msg1: GPtr<u8>, msg2: GPtr<u8>, splash: GPtr<u8>) = cdecl 0x53DED0 => unported;
        /// `DoRWStuffStartOfFrame(int16 TopRed, TopGreen, TopBlue, BottomRed, BottomGreen, BottomBlue, int16 Alpha)`
        /// (seven dword pushes)
        pub fn do_rw_stuff_start_of_frame(a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, g: u32) -> bool = cdecl 0x53D690 => crate::platform::do_rw_stuff_start_of_frame;
        pub fn message_loop() = cdecl 0x746870 => unported;
        pub fn pad_update_pads() = cdecl 0x541DD0 => unported;
        /// `CMenuManager::MessageScreen(const char* key, bool blackBg, bool cameraUpdateStarted)`
        pub fn menu_message_screen(this: GPtr<u8>, key: GPtr<u8>, black_bg: u32, camera_update_started: u32) = thiscall 0x579330 => unported;
        pub fn font_draw_fonts() = cdecl 0x71A210 => unported;
        pub fn do_rw_stuff_end_of_frame() = cdecl 0x53D840 => unported;
        pub fn pad_get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
        pub fn rs_event_handler(event: i32, param: GPtr<u8>) -> i32 = cdecl 0x619B60 => unported;
        /// `CText::Load(bool keepMissionPack)`
        pub fn text_load(this: GPtr<u8>, keep_mission_pack: u32) = thiscall 0x6A01A0 => unported;
        pub fn car_fx_renderer_initialise() -> i32 = cdecl 0x5D5AC0 => unported;
        pub fn grass_renderer_initialise() -> u8 = cdecl 0x5DD6B0 => unported;
        pub fn custom_building_renderer_initialise() -> i32 = cdecl 0x5D7EC0 => unported;
        pub fn rs_rw_initialize(param: GPtr<u8>) -> i32 = cdecl 0x619C90 => unported;
        pub fn game_initialise_render_ware() -> bool = cdecl 0x5BD600 => super::initialise_render_ware;
        pub fn d3d_resource_system_init() = cdecl 0x730830 => super::d3d_resource_system_init;
        pub fn txd_store_initialise() = cdecl 0x731F20 => unported;
        pub fn visibility_plugins_initialise() = cdecl 0x733A20 => unported;
        pub fn rw_camera_create() -> GPtr<u8> = cdecl 0x7EE4F0 => unported;
        pub fn rw_frame_create() -> GPtr<u8> = cdecl 0x7F0410 => unported;
        /// `RwRasterCreate(width, height, depth, flags)`
        pub fn rw_raster_create(w: i32, h: i32, depth: i32, flags: i32) -> GPtr<u8> = cdecl 0x7FB230 => unported;
        pub fn camera_init(this: GPtr<u8>) = thiscall 0x5BC520 => unported;
        pub fn camera_set_rw_camera(this: GPtr<u8>, cam: GPtr<u8>) = thiscall 0x50C100 => unported;
        pub fn rw_camera_set_far_clip_plane(cam: GPtr<u8>, v: f32) -> GPtr<u8> = cdecl 0x7EE2A0 => unported;
        pub fn rw_camera_set_near_clip_plane(cam: GPtr<u8>, v: f32) -> GPtr<u8> = cdecl 0x7EE1D0 => unported;
        pub fn camera_size(camera: GPtr<u8>, rect: GPtr<u8>, view_window: f32, aspect: f32) = cdecl 0x72FC70 => crate::camerasize::camera_size;
        pub fn rp_world_create(bbox: GPtr<u8>) -> GPtr<u8> = cdecl 0x74F760 => unported;
        pub fn rp_world_add_camera(world: GPtr<u8>, cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x750F20 => unported;
        pub fn lights_create(world: GPtr<u8>) -> GPtr<u8> = cdecl 0x5BA520 => unported;
        pub fn create_debug_font() = cdecl 0x734610 => crate::platform::create_debug_font;
        pub fn font_initialise() = cdecl 0x5BA690 => unported;
        pub fn hud_initialise() = cdecl 0x5BA850 => unported;
        pub fn player_skin_initialise() = cdecl 0x6FF8A0 => unported;
        pub fn post_effects_initialise() = cdecl 0x704630 => unported;
        pub fn rw_matrix_create() -> GPtr<u8> = cdecl 0x7F2A50 => unported;
        pub fn rw_d3d9_device_get_restore_callback() -> u32 = cdecl 0x7FAE30 => unported;
        pub fn rw_d3d9_device_set_restore_callback(cb: u32) = cdecl 0x7FAE20 => unported;
        /// `operator new` (0x82119A)
        pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
        pub fn rs_timer() -> u32 = cdecl 0x619410 => unported;
        pub fn rw_object_has_frame_set_frame(object: GPtr<u8>, frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x804EF0 => unported;
        pub fn rw_frame_destroy(frame: GPtr<u8>) -> i32 = cdecl 0x7F05A0 => unported;
        pub fn rw_raster_destroy(raster: GPtr<u8>) -> i32 = cdecl 0x7FB020 => unported;
        pub fn rw_camera_destroy(camera: GPtr<u8>) -> i32 = cdecl 0x7EE4B0 => unported;
        /// the chained restore callback stored at 0xBD0114
        pub fn prev_d3d_restore_callback() = cdecl cell 0xBD0114 => unported;
        pub fn temp_col_models_initialise() = cdecl 0x5BA8C0 => unported;
        pub fn handling_load_handling_data(this: GPtr<u8>) = thiscall 0x5BD830 => unported;
        pub fn surface_infos_init(this: GPtr<u8>) = thiscall 0x55F420 => unported;
        pub fn ped_stats_initialise() = cdecl 0x5BF9D0 => unported;
        pub fn time_cycle_initialise(is_second: u32) = cdecl 0x5BBAC0 => unported;
        pub fn pop_cycle_initialise() = cdecl 0x5BC090 => unported;
        pub fn vehicle_recording_init_at_start_of_game() = cdecl 0x45A1B0 => unported;
        pub fn audio_engine_initialise(this: GPtr<u8>) -> u8 = thiscall 0x5B9C60 => unported;
        pub fn break_manager_init(this: GPtr<u8>) -> u8 = thiscall 0x59E650 => unported;
        pub fn bone_node_manager_init(this: GPtr<u8>) -> u8 = thiscall 0x6173F0 => unported;
        pub fn ik_chain_manager_init(this: GPtr<u8>) -> u8 = thiscall 0x6180A0 => unported;
        /// `CDraw::CalculateAspectRatio()` (result in ST0)
        pub fn draw_calculate_aspect_ratio() -> f64 = cdecl 0x6FF420 => unported;
        pub fn timer_update() = cdecl 0x561B10 => unported;
        pub fn sprite2d_set_recip_near_clip() = cdecl 0x727260 => unported;
        pub fn sprite2d_init_per_frame() = cdecl 0x727350 => unported;
        pub fn font_init_per_frame() = cdecl 0x719800 => unported;
        pub fn menu_process(this: GPtr<u8>) = thiscall 0x57B440 => unported;
        pub fn audio_engine_service(this: GPtr<u8>) = thiscall 0x507750 => unported;
        pub fn visibility_plugins_set_render_ware_camera(camera: GPtr<u8>) = cdecl 0x7328C0 => unported;
        pub fn rw_camera_clear(camera: GPtr<u8>, colour: GPtr<u8>, mode: i32) -> GPtr<u8> = cdecl 0x7EE340 => unported;
        pub fn rs_camera_begin_update(camera: GPtr<u8>) -> i32 = cdecl 0x619450 => unported;
        pub fn defined_state() = cdecl 0x734650 => unported;
        pub fn securom_state_display() = cdecl 0x744AE0 => unported;
        pub fn do_fade() = cdecl 0x53E600 => unported;
        pub fn hud_draw_after_fade() = cdecl 0x58D490 => unported;
        pub fn messages_display(flag: u32) = cdecl 0x69EFC0 => unported;
        pub fn credits_render_credits() = cdecl 0x5A87F0 => unported;
        pub fn debug_display_text_buffer() = cdecl 0x532260 => unported;
        pub fn flush_obrs_printfs() = cdecl 0x734640 => unported;
        pub fn rw_camera_end_update(camera: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EE180 => unported;
        pub fn rs_camera_show_raster(camera: GPtr<u8>) -> GPtr<u8> = cdecl 0x619440 => unported;
    }
}

// ------------------------------------------------------------------ CGame -----------------------

/// `CGame::InitialiseOnceBeforeRW` (0x53BB50, cdecl, returns true in AL).
#[hook(addr = 0x53BB50, conv = cdecl, name = "CGame::InitialiseOnceBeforeRW", expect = "E8 5B 38 1F 00")]
pub fn initialise_once_before_rw() -> bool {
    call::memory_mgr_init();
    call::localisation_initialise();
    call::file_mgr_initialise();
    call::cd_stream_init(5);
    call::pad_initialise();
    true
}

/// `CGame::FinalShutdown` (0x53BC30, cdecl). Ends with a tail jump into `CdStreamShutdown`.
#[hook(addr = 0x53BC30, conv = cdecl, name = "CGame::FinalShutdown", expect = "B9 50 F0 96 00")]
pub fn final_shutdown() {
    call::path_find_shutdown(at(THE_PATHS));
    call::temp_col_models_shutdown();
    call::txd_store_shutdown();
    call::ped_stats_shutdown();
    call::text_unload(at(THE_TEXT), 0);
    call::cd_stream_shutdown();
}

/// `CGame::InitAfterLostFocus` (0x53BC60, cdecl): if neither menu byte +0xE9 nor `m_bMenuActive` is set,
/// `m_bActivateMenuNextFrame = true`.
#[hook(addr = 0x53BC60, conv = cdecl, name = "CGame::InitAfterLostFocus", expect = "A0 31 68 BA 00")]
pub fn init_after_lost_focus() {
    if !MENU_FIELD_E9.get().get() && !MENU_ACTIVE.get().get() {
        MENU_ACTIVATE_NEXT_FRAME.set(GBool::TRUE);
    }
}

/// `CGame::ReloadIPLs` (0x53BED0, cdecl): an empty function (`ret`, nop padding).
// Not hooked: a lone `ret` cannot be relocated (the installer needs 5 bytes of straight-line code).
pub fn reload_ipls() {}

/// `CGame::CanSeeOutSideFromCurrArea` (0x53C4A0, cdecl): `xor eax,eax; sete al` — the whole EAX is 0/1.
#[hook(addr = 0x53C4A0, conv = cdecl, name = "CGame::CanSeeOutSideFromCurrArea", expect = "8B 0D 14 29 B7 00")]
pub fn can_see_outside_from_curr_area() -> u32 {
    (CURR_AREA.get() == 0) as u32
}

/// `CGame::CanSeeWaterFromCurrArea` (0x53C4B0, cdecl): EAX = 1 for area 0 or 5, else 0.
#[hook(addr = 0x53C4B0, conv = cdecl, name = "CGame::CanSeeWaterFromCurrArea", expect = "A1 14 29 B7 00")]
pub fn can_see_water_from_curr_area() -> u32 {
    let a = CURR_AREA.get();
    (a == 0 || a == 5) as u32
}

/// `CGame::TidyUpMemory(bool, bool clearD3D)` (0x53C500, cdecl). The original calls the two D3D tidy
/// functions itself (gta-reversed calls `DrasticTidyUpMemory`); `FindPlayerPed(0)` is called first.
#[hook(addr = 0x53C500, conv = cdecl, name = "CGame::TidyUpMemory", expect = "6A 00 E8 09 1D 03 00")]
pub fn tidy_up_memory(_unused: u8, clear_d3d: u8) {
    if !call::find_player_ped(0).is_null() && clear_d3d != 0 {
        call::tidy_up_d3d_textures(100);
        call::tidy_up_d3d_index_buffers(200);
    }
}

/// `CGame::DrasticTidyUpMemory(bool)` (0x53C810, cdecl; the second call is a tail jump).
#[hook(addr = 0x53C810, conv = cdecl, name = "CGame::DrasticTidyUpMemory", expect = "6A 64 E8 C9 3D 1F 00")]
pub fn drastic_tidy_up_memory(_unused: u8) {
    call::tidy_up_d3d_textures(100);
    call::tidy_up_d3d_index_buffers(200);
}

/// `CGame::D3DDeviceRestoreCallback` (0x5BA120, cdecl): chain the previous callback, then flag the
/// real-time shadows for re-initialisation (only when initialised) and the mirrors as restored.
#[hook(addr = 0x5BA120, conv = cdecl, name = "CGame::D3DDeviceRestoreCallback", expect = "A1 14 01 BD 00")]
pub fn d3d_device_restore_callback() {
    if PREV_D3D_RESTORE_CALLBACK.get() != 0 {
        call::prev_d3d_restore_callback();
    }
    if RT_SHADOW_INITIALISED.get().get() {
        RT_SHADOW_NEEDS_REINIT.set(GBool::TRUE);
    }
    MIRRORS_D3D_RESTORED.set(GBool::TRUE);
}

/// `CGame::InitialiseEssentialsAfterRW` (0x5BA160, cdecl, bool in AL). Short-circuits like the
/// original: `CCarFXRenderer` result tested as EAX, `CGrassRenderer` as AL, `CCustomBuildingRenderer`
/// as EAX; `CTimer::Initialise` only on success.
#[hook(addr = 0x5BA160, conv = cdecl, name = "CGame::InitialiseEssentialsAfterRW", expect = "6A 00 B9 40 B3 C1 00")]
pub fn initialise_essentials_after_rw() -> bool {
    call::text_load(at(THE_TEXT), 0);
    if call::car_fx_renderer_initialise() == 0
        || call::grass_renderer_initialise() == 0
        || call::custom_building_renderer_initialise() == 0
    {
        return false;
    }
    call::timer_initialise();
    true
}

/// `CGame::InitialiseCoreDataAfterRW` (0x5BFA90, cdecl, bool in AL). `CTempColModels::Initialise`,
/// `gHandlingDataMgr.LoadHandlingData()` (0xC2B9C8) and then the manager's five leading floats (0.1,
/// 0.9, 1.0, 0.8, 0.98, stored after the load as the original does), `g_surfaceInfos.Init()`,
/// `CPedStats::Initialise`, `CTimeCycle::Initialise(false)`, `CPopCycle::Initialise`,
/// `CVehicleRecording::InitAtStartOfGame`; then `AudioEngine.Initialise() && g_breakMan.Init() &&
/// g_boneNodeMan.Init()` (each AL) and finally `!g_ikChainMan.Init()` (`neg al; sbb al, al; inc al`).
/// The early failures return `xor al, al` (the upper EAX bits are the callee's).
///
/// Not registered: 0x5BFA90 is hooked by sa-core mvpa_game.rs (duplicate port left unregistered when STD-5 merged both).
pub fn initialise_core_data_after_rw() -> bool {
    call::temp_col_models_initialise();
    call::handling_load_handling_data(GPtr::from_addr(0xC2B9C8));
    for (i, bits) in [0x3DCC_CCCDu32, 0x3F66_6666, 0x3F80_0000, 0x3F4C_CCCD, 0x3F7A_E148].into_iter().enumerate() {
        GPtr::<u32>::from_addr(0xC2B9C8 + 4 * i).write(bits);
    }
    call::surface_infos_init(GPtr::from_addr(0xB79538));
    call::ped_stats_initialise();
    call::time_cycle_initialise(0);
    call::pop_cycle_initialise();
    call::vehicle_recording_init_at_start_of_game();
    if call::audio_engine_initialise(GPtr::from_addr(0xB6BC90)) == 0
        || call::break_manager_init(GPtr::from_addr(0xBB4240)) == 0
        || call::bone_node_manager_init(GPtr::from_addr(0xC10820)) == 0
    {
        return false;
    }
    call::ik_chain_manager_init(GPtr::from_addr(0xC15448)) == 0
}

/// `Game::FrontendIdle` (0x53E770, cdecl): one frame of the front end. `CDraw::CalculateAspectRatio`
/// (ST0 popped), `CTimer::Update`, `CSprite2d::SetRecipNearClip`/`InitPerFrame`, `CFont::InitPerFrame`,
/// `CPad::UpdatePads`; `m_bActivateMenuNextFrame` → `m_bMenuActive = 1`; `FrontEndMenuManager.Process()`;
/// then, unless `RsGlobal.quit` (0xC17050): `AudioEngine.Service()`, `CameraSize(Scene.camera, NULL,
/// tan(CDraw::ms_fFOV * [0x8631D4]), CDraw::ms_fAspectRatio)` (`fmul` at PC=24, `fptan`, the tangent
/// stored as f32), `CVisibilityPlugins::SetRenderWareCamera`, `RwCameraClear(camera, &gColourTop,
/// rwCAMERACLEARZ)`, and when `RsCameraBeginUpdate` succeeds: `DefinedState`, `DrawFrontEnd` when the
/// menu is active, `SecuromStateDisplay`, `DoFade`, `CHud::DrawAfterFade`, `CMessages::Display(false)`,
/// `CFont::DrawFonts`, `CCredits::RenderCredits` when credits run and the menu is not active,
/// `CFont::DrawFonts`, `CDebug::DebugDisplayTextBuffer`, `FlushObrsPrintfs`, `RwCameraEndUpdate`,
/// `RsCameraShowRaster`.
///
/// Not registered: 0x53E770 is hooked by sa-core mvpb_app.rs (duplicate port left unregistered when STD-5 merged both).
pub fn frontend_idle() {
    let _ = call::draw_calculate_aspect_ratio();
    call::timer_update();
    call::sprite2d_set_recip_near_clip();
    call::sprite2d_init_per_frame();
    call::font_init_per_frame();
    call::pad_update_pads();
    if MENU_ACTIVATE_NEXT_FRAME.get().get() {
        MENU_ACTIVE.set(GBool::TRUE);
    }
    call::menu_process(at(FRONTEND_MENU_MANAGER));
    if GPtr::<u32>::from_addr(0xC17050).read() != 0 {
        return;
    }
    call::audio_engine_service(GPtr::from_addr(0xB6BC90));
    let fov = GPtr::<f32>::from_addr(0x8D5038).read();
    let k = crate::platform::rdata::<f32>(0x8631D4).read();
    let x = X87::from_f32(fov).fmul_m(k);
    let tan = x.fptan().map_or(x, |(t, _one)| t);
    let camera = SCENE_CAMERA.get();
    call::camera_size(camera, GPtr::NULL, tan.to_f32(), GPtr::<f32>::from_addr(0xC3EFA4).read());
    call::visibility_plugins_set_render_ware_camera(SCENE_CAMERA.get());
    call::rw_camera_clear(SCENE_CAMERA.get(), GPtr::from_addr(0xB72CA0), 2);
    if call::rs_camera_begin_update(SCENE_CAMERA.get()) == 0 {
        return;
    }
    call::defined_state();
    if MENU_ACTIVE.get().get() {
        call::menu_draw_front_end(at(FRONTEND_MENU_MANAGER));
    }
    call::securom_state_display();
    call::do_fade();
    call::hud_draw_after_fade();
    call::messages_display(0);
    call::font_draw_fonts();
    if GPtr::<u8>::from_addr(0xC6E97C).read() != 0 && !MENU_ACTIVE.get().get() {
        call::credits_render_credits();
    }
    call::font_draw_fonts();
    call::debug_display_text_buffer();
    call::flush_obrs_printfs();
    call::rw_camera_end_update(SCENE_CAMERA.get());
    call::rs_camera_show_raster(SCENE_CAMERA.get());
}

/// `CVector::Normalise` (0x59C910, thiscall). Phase B entry only: in phase A sa-hook keeps its own
/// (verified) hook here; both use `sa_math::vector::normalise` (x, y, z stored in that order; a zero
/// vector gets x = 1 only).
#[crate::native(addr = 0x59C910, conv = thiscall, name = "CVector::Normalise")]
pub fn vector_normalise(this: GPtr<[f32; 3]>) {
    let v = this.read();
    let r = sa_math::vector::normalise(v);
    this.write(r);
}

/// `BreakManager_c::Init` (0x59E650, thiscall): `mov al, 1; ret` (3 bytes, not hookable).
#[crate::native(addr = 0x59E650, conv = thiscall, name = "BreakManager_c::Init")]
pub fn break_manager_init(_this: GPtr<u8>) -> bool {
    true
}

/// `CDebug::DebugDisplayTextBuffer` (0x532260): a lone `ret` in 1.0 US.
#[crate::native(addr = 0x532260, conv = cdecl, name = "CDebug::DebugDisplayTextBuffer")]
pub fn debug_display_text_buffer() {}

/// `FlushObrsPrintfs` (0x734640): a lone `ret` in 1.0 US.
#[crate::native(addr = 0x734640, conv = cdecl, name = "FlushObrsPrintfs")]
pub fn flush_obrs_printfs() {}

/// Releases a D3D resource held by a `D3DTextureBuffer`/index-buffer slot: `IUnknown::Release` (+8).
fn release_com(p: u32) {
    call::com_release(GPtr::from_addr(p as usize));
}

/// `D3DResourceSystem::TidyUpD3DIndexBuffers(count)` (0x730740, cdecl). Releases up to `count` index
/// buffers: first from the dynamic list (count 0xC87FD8, size 0xC87FD4, array 0xC87FDC; popped from
/// the back, a NULL entry ends that phase), then round-robin over the 16 per-size buffers at 0xC87E48
/// (0x18 each: +0xC size, +0x10 count, +0x14 array) starting at slot 0xC87FE4, stopping after 16
/// consecutive empty slots; the slot index advances by `(i + 1) % 16` (signed, as the original).
#[hook(addr = 0x730740, conv = cdecl, name = "D3DResourceSystem::TidyUpD3DIndexBuffers", expect = "56 8B 74 24 08 57 33 FF",
       route = "L2", hazards = "none", recs = "boot")]
pub fn tidy_up_d3d_index_buffers(count: u32) {
    let mut n = count;
    if n == 0 {
        return;
    }
    let g32 = |a: usize| GPtr::<u32>::from_addr(a);
    loop {
        let c = g32(0xC87FD8).read();
        if c == 0 {
            break;
        }
        g32(0xC87FD4).write(g32(0xC87FD4).read().wrapping_sub(1));
        let c = c - 1;
        g32(0xC87FD8).write(c);
        let e = g32(g32(0xC87FDC).read() as usize + 4 * c as usize).read();
        if e == 0 {
            break;
        }
        release_com(e);
        n -= 1;
        if n == 0 {
            return;
        }
    }
    let mut empty = 0;
    let mut slot = g32(0xC87FE4).read() as i32;
    while empty < 0x10 {
        let b = 0xC87E48 + 0x18 * slot as usize;
        let c = g32(b + 0x10).read();
        let mut released = false;
        if c != 0 {
            g32(b + 0xC).write(g32(b + 0xC).read().wrapping_sub(1));
            let c = c - 1;
            g32(b + 0x10).write(c);
            let e = g32(g32(b + 0x14).read() as usize + 4 * c as usize).read();
            if e != 0 {
                release_com(e);
                slot = g32(0xC87FE4).read() as i32;
                empty = 0;
                n -= 1;
                released = true;
            }
        }
        if !released {
            empty += 1;
        }
        slot = slot.wrapping_add(1) % 16;
        g32(0xC87FE4).write(slot as u32);
        if n == 0 {
            break;
        }
    }
}

/// `D3DResourceSystem::TidyUpD3DTextures(count)` (0x7305E0, cdecl). Releases up to `count` textures
/// round-robin over the `[0xC87C60]` texture buffers at 0xC87C68 (0x1C each: +0x10 size, +0x14 count,
/// +0x18 array) starting at slot 0xC87FE0, stopping after a full round of empty buffers; the slot
/// advances by `(i + 1) % count` (`idiv`). The entry's no-CD chunk (0x40A16B) loads the slot as
/// `[[0x1560904] + [0x685002]]`: both dwords are static (the `.HOODLUM` one is read only there), their
/// sum is 0xC87FE0, so the port reads that global.
#[hook(addr = 0x7305E0, conv = cdecl, name = "D3DResourceSystem::TidyUpD3DTextures", expect = "56 8B 74 24 08 57 33 FF",
       route = "L2", hazards = "entry-protection-read", recs = "boot")]
pub fn tidy_up_d3d_textures(count: u32) {
    let mut n = count;
    if n == 0 {
        return;
    }
    let g32 = |a: usize| GPtr::<u32>::from_addr(a);
    let mut slot = g32(0xC87FE0).read() as i32;
    let mut total = g32(0xC87C60).read() as i32;
    let mut empty = 0i32;
    while empty < total {
        let b = 0xC87C68 + 0x1C * slot as usize;
        let c = g32(b + 0x14).read();
        let mut released = false;
        if c != 0 {
            g32(b + 0x10).write(g32(b + 0x10).read().wrapping_sub(1));
            let c = c - 1;
            g32(b + 0x14).write(c);
            let e = g32(g32(b + 0x18).read() as usize + 4 * c as usize).read();
            if e != 0 {
                release_com(e);
                total = g32(0xC87C60).read() as i32;
                slot = g32(0xC87FE0).read() as i32;
                empty = 0;
                n -= 1;
                released = true;
            }
        }
        if !released {
            empty += 1;
        }
        slot = slot.wrapping_add(1) % total;
        g32(0xC87FE0).write(slot as u32);
        if n == 0 {
            break;
        }
    }
}

/// `CPad::StopPadsShaking` (0x541D70): a lone `ret` in 1.0 US.
#[crate::native(addr = 0x541D70, conv = cdecl, name = "CPad::StopPadsShaking")]
pub fn stop_pads_shaking() {}

/// `CStreaming::ImGonnaUseStreamingMemory` (0x407BE0): a lone `ret` in 1.0 US.
#[crate::native(addr = 0x407BE0, conv = cdecl, name = "CStreaming::ImGonnaUseStreamingMemory")]
pub fn im_gonna_use_streaming_memory() {}

/// `CStreaming::IHaveUsedStreamingMemory` (0x407BF0): a lone `ret` in 1.0 US.
#[crate::native(addr = 0x407BF0, conv = cdecl, name = "CStreaming::IHaveUsedStreamingMemory")]
pub fn i_have_used_streaming_memory() {}

/// Result of the peds.col version check, as the bytes decode it (host-testable part of
/// `ValidateVersion`): each byte read is decremented by 23 (`add al, 0E9h`); the loop stops after the
/// byte that decodes to 0 or after 128 bytes.
pub fn decode_version_byte(b: u8) -> u8 {
    b.wrapping_add(0xE9)
}

/// `CGame::ValidateVersion` (0x5BA060, cdecl).
///
/// Original behaviour kept: the open check compares the `FILE*` with **-1** (`cmp ebx, -1`), not with
/// null (gta-reversed: `!file`; a missing file therefore reaches `Seek(NULL)` in the original);
/// failure shows `LoadingScreen("Invalid version")` and then spins forever (`jmp $`); the version text
/// is copied with `strncpy(version_name, buf + 15, 64)` (gta-reversed: `strncpy_s`, which always
/// terminates). The local buffer is uninitialised in the original; here it starts zeroed, which only
/// matters for a file whose text has no terminator within the bytes `strncmp`/`strncpy` read.
#[hook(addr = 0x5BA060, conv = cdecl, name = "CGame::ValidateVersion", expect = "81 EC 80 00 00 00")]
pub fn validate_version() {
    let mut buf = [0u8; 128];
    let file = call::file_mgr_open_file(STR_PEDS_COL.ptr().cast(), STR_RB.ptr().cast());
    if file != u32::MAX {
        call::file_mgr_seek(file, 100, 0);
        let base: GPtr<u8> = GPtr::from_raw(buf.as_mut_ptr());
        let mut i = 0i32;
        loop {
            let p = base.wrapping_index(i);
            call::file_mgr_read(file, p, 1);
            let v = decode_version_byte(p.read());
            p.write(v);
            if v == 0 {
                break;
            }
            i += 1;
            call::file_mgr_seek(file, 99, 1);
            if i >= 0x80 {
                break;
            }
        }
        if call::strncmp(base, STR_GTA3.ptr().cast(), 15) == 0 {
            call::strncpy(VERSION_NAME.ptr().cast(), base.wrapping_index(15), 0x40);
            call::file_mgr_close_file(file);
            return;
        }
    }
    call::loading_screen(STR_INVALID_VERSION.ptr().cast(), GPtr::NULL, GPtr::NULL);
    // `jmp $` at 0x5BA110
    loop {
        core::hint::spin_loop();
    }
}

/// `CGame::CameraDestroy(RwCamera*)` (0x72FD90, cdecl). RwCamera: `object.parent` +0x04,
/// `frameBuffer` +0x60, `zBuffer` +0x64; a raster's parent is its +0x00.
#[hook(addr = 0x72FD90, conv = cdecl, name = "CGame::CameraDestroy", expect = "53 8B 5C 24 08")]
pub fn camera_destroy(camera: GPtr<u8>) {
    if camera.is_null() {
        return;
    }
    let parent: GPtr<u8> = camera.byte_add(4).cast::<GPtr<u8>>().read();
    if !parent.is_null() {
        call::rw_object_has_frame_set_frame(camera, GPtr::NULL);
        call::rw_frame_destroy(parent);
    }
    for off in [0x60isize, 0x64] {
        let raster: GPtr<u8> = camera.byte_add(off).cast::<GPtr<u8>>().read();
        if !raster.is_null() {
            let rparent: GPtr<u8> = raster.cast::<GPtr<u8>>().read();
            call::rw_raster_destroy(raster);
            if !rparent.is_null() && rparent != raster {
                call::rw_raster_destroy(rparent);
            }
        }
    }
    call::rw_camera_destroy(camera);
}

/// `CGame::InitialiseRenderWare` (0x5BD600, cdecl, bool in AL).
///
/// Original behaviour kept (gta-reversed differs): when `RwCameraCreate` fails, `CameraDestroy(NULL)`
/// is still called and `Scene.camera` is set to NULL; the camera is accepted only when its frame,
/// both rasters and both rasters' parents (+0x00) are non-null, otherwise it is destroyed and
/// `Scene.camera = NULL` (gta-reversed checks the frame only); `RsGlobal.maximumWidth/Height` are read
/// once before `RwCameraCreate`. The world bbox is `sup = (1e4, 1e4, 1e4)`, `inf = -sup`.
#[hook(addr = 0x5BD600, conv = cdecl, name = "CGame::InitialiseRenderWare", expect = "83 EC 18 53 56")]
pub fn initialise_render_ware() -> bool {
    validate_version();
    call::d3d_resource_system_init();
    call::txd_store_initialise();
    call::visibility_plugins_initialise();
    let h = RS_MAXIMUM_HEIGHT.get();
    let w = RS_MAXIMUM_WIDTH.get();
    let mut cam = call::rw_camera_create();
    if !cam.is_null() {
        let frame = call::rw_frame_create();
        call::rw_object_has_frame_set_frame(cam, frame);
        let fb = call::rw_raster_create(w, h, 0, 2);
        cam.byte_add(0x60).cast::<GPtr<u8>>().write(fb);
        let zb = call::rw_raster_create(w, h, 0, 1);
        cam.byte_add(0x64).cast::<GPtr<u8>>().write(zb);
        let ok = !cam.byte_add(4).cast::<GPtr<u8>>().read().is_null() && {
            let fb = cam.byte_add(0x60).cast::<GPtr<u8>>().read();
            !fb.is_null()
                && fb.cast::<u32>().read() != 0
                && !zb.is_null()
                && zb.cast::<u32>().read() != 0
        };
        if !ok {
            camera_destroy(cam);
            cam = GPtr::NULL;
        }
    } else {
        camera_destroy(cam);
    }
    SCENE_CAMERA.set(cam);
    if cam.is_null() {
        return false;
    }
    call::camera_init(at(THE_CAMERA));
    call::camera_set_rw_camera(at(THE_CAMERA), SCENE_CAMERA.get());
    call::rw_camera_set_far_clip_plane(SCENE_CAMERA.get(), f32::from_bits(0x44FA_0000));
    call::rw_camera_set_near_clip_plane(SCENE_CAMERA.get(), f32::from_bits(0x3F66_6666));
    call::camera_size(SCENE_CAMERA.get(), GPtr::NULL, f32::from_bits(0x3F33_3333), f32::from_bits(0x3FAA_AAAB));
    let s = 0x461C_4000u32;
    let i = 0xC61C_4000u32;
    let mut bbox = [s, s, s, i, i, i];
    let world = call::rp_world_create(GPtr::from_raw(bbox.as_mut_ptr()).cast());
    SCENE_WORLD.set(world);
    if world.is_null() {
        camera_destroy(SCENE_CAMERA.get());
        SCENE_CAMERA.set(GPtr::NULL);
        return false;
    }
    call::rp_world_add_camera(world, SCENE_CAMERA.get());
    call::lights_create(SCENE_WORLD.get());
    call::create_debug_font();
    call::font_initialise();
    call::hud_initialise();
    call::player_skin_initialise();
    call::post_effects_initialise();
    WORKING_MATRIX_1.set(call::rw_matrix_create());
    WORKING_MATRIX_2.set(call::rw_matrix_create());
    PREV_D3D_RESTORE_CALLBACK.set(call::rw_d3d9_device_get_restore_callback());
    call::rw_d3d9_device_set_restore_callback(0x5BA120);
    true
}

/// One `D3DResourceSystem` buffer list (6 dwords: +0 type, +4 size, +8 capacity, +0xC / +0x10
/// counters, +0x14 the `new(0x40)` array) as the `Init` loop fills it, in the loop's store order.
fn d3d_res_list(base: GPtr<u32>, ty: u32, size: u32) {
    base.write(ty);
    base.wrapping_index(1).write(size);
    let arr = call::operator_new(0x40);
    base.wrapping_index(5).cast::<GPtr<u8>>().write(arr);
    base.wrapping_index(2).write(0x10);
    base.wrapping_index(4).write(0);
    base.wrapping_index(3).write(0);
}

/// `D3DResourceSystem::Init` (0x730830, cdecl; not reversed by gta-reversed, ported from the bytes).
/// Writes, in the original's order: 0xC87C60 = 1, the byte 0x8D6084 = 1, 0xC87C68 = 0, 0xC87C6C = 0,
/// 0xC87C70 = -1; then `new(0x40)` → 0xC87C80, 0xC87C74 = 0x10, 0xC87C7C = 0, 0xC87C78 = 0. Then 16 lists
/// of 6 dwords at 0xC87E48 (type 0x65, size 100, 200, … 1600, `new(0x40)`, capacity 0x10, two zeroed
/// counters) and a 17th at 0xC87FC8 (type 0x65, size **0**, array stored last).
#[hook(addr = 0x730830, conv = cdecl, name = "D3DResourceSystem::Init", expect = "53 55 56 B8 01 00 00 00")]
pub fn d3d_resource_system_init() {
    D3D_RES_C87C60.set(1);
    D3D_RES_BUFFERING.set(1);
    D3D_RES_C87C68.set(0);
    D3D_RES_C87C6C.set(0);
    D3D_RES_C87C70.set(-1);
    let arr = call::operator_new(0x40);
    D3D_RES_C87C80.set(arr);
    D3D_RES_C87C74.set(0x10);
    D3D_RES_C87C7C.set(0);
    D3D_RES_C87C78.set(0);
    let lists = D3D_RES_LISTS.ptr().cast::<u32>();
    for k in 0..16i32 {
        d3d_res_list(lists.wrapping_index(k * 6), 0x65, (100 * (k + 1)) as u32);
    }
    // the 17th entry: size 0, the array is stored after the counters
    let last = lists.wrapping_index(16 * 6);
    last.write(0x65);
    last.wrapping_index(1).write(0);
    let arr = call::operator_new(0x40);
    last.wrapping_index(2).write(0x10);
    last.wrapping_index(4).write(0);
    last.wrapping_index(3).write(0);
    last.wrapping_index(5).cast::<GPtr<u8>>().write(arr);
}

mod d3dres {
    use crate::mem::GPtr;
    crate::orig! {
        /// `IDirect3DBaseTexture9::GetLevelCount()` (+0x34)
        pub fn tex_get_level_count(tex: GPtr<u8>) -> u32 = stdcall vslot 0x34 => unported;
        /// `IDirect3DTexture9::GetLevelDesc(level, D3DSURFACE_DESC*)` (+0x44)
        pub fn tex_get_level_desc(tex: GPtr<u8>, level: u32, desc: GPtr<u32>) -> i32 = stdcall vslot 0x44 => unported;
        /// `IDirect3DDevice9::CreateTexture(w, h, levels, usage, format, pool, out, shared)` (+0x5C)
        pub fn dev_create_texture(dev: GPtr<u8>, w: u32, h: u32, levels: u32, usage: u32, format: u32, pool: u32, out: GPtr<u32>, shared: u32) -> i32 = stdcall vslot 0x5C => unported;
        /// `D3DTextureBuffer::Pop(format, width, height, levels)` (0x72FF60)
        pub fn texture_buffer_pop(this: GPtr<u8>, format: u32, w: u32, h: u32, levels: u32) -> GPtr<u8> = thiscall 0x72FF60 => super::texture_buffer_pop;
    }
}

/// The D3DResourceSystem texture buffers: 0xC87C68 + 0x1C·i (`{format +0, width +4, levels +8, ?,
/// popped +0x10, count +0x14, list +0x18}`), `[0xC87C60]` of them.
const D3D_TEXTURE_BUFFERS: usize = 0xC87C68;

/// `D3DTextureBuffer::Pop(format, width, height, levels)` (0x72FF60, thiscall, `ret 0x10`): from
/// the last pooled texture down, the first whose single-level-ness (`GetLevelCount() == 1`) equals
/// `levels` and whose level-0 desc has this format, width and height is removed (the last entry
/// moves into its slot unless it was the last) and returned; NULL when none. The desc local is
/// uninitialised in the original when `GetLevelDesc` fails (port: zero). Written by STD-3,
/// unverified.
#[hook(addr = 0x72FF60, conv = thiscall, name = "D3DTextureBuffer::Pop", expect = "83 EC 20 53 55 56 8B D9")]
pub fn texture_buffer_pop(this: GPtr<u8>, format: u32, w: u32, h: u32, levels: u32) -> GPtr<u8> {
    let count = this.byte_add(0x14).cast::<u32>();
    let mut i = count.read();
    while i != 0 {
        let list = this.byte_add(0x18).cast::<u32>().read();
        let tex = GPtr::<u8>::from_addr(GPtr::<u32>::from_addr(list as usize).wrapping_index(i as i32 - 1).read() as usize);
        i -= 1;
        let single = (d3dres::tex_get_level_count(tex) == 1) as u32;
        if single != levels {
            continue;
        }
        let mut desc = [0u32; 8];
        d3dres::tex_get_level_desc(tex, 0, GPtr::from_raw(desc.as_mut_ptr()));
        if desc[0] == format && desc[6] == w && desc[7] == h {
            let n = count.read().wrapping_sub(1);
            count.write(n);
            if (n as i32) > 0 && n != i {
                let l = GPtr::<u32>::from_addr(this.byte_add(0x18).cast::<u32>().read() as usize);
                l.wrapping_index(i as i32).write(l.wrapping_index(n as i32).read());
            }
            return tex;
        }
    }
    GPtr::NULL
}

/// `D3DResourceSystem::CreateTexture(width, height, levels, format, IDirect3DTexture9** out)`
/// (0x730510): `levels > 1` becomes 0 (full chain). A square texture first looks in buffers
/// 1..`[0xC87C60]` for one with this width (x87 compare of the integer against the width converted
/// to float), levels and format and takes its last pooled texture; otherwise buffer 0 is searched
/// (`D3DTextureBuffer::Pop`). A pooled texture is returned with `D3D_OK`; else
/// `device->CreateTexture(w, h, levels, 0, format, D3DPOOL_MANAGED, out, NULL)`'s result. Written by
/// STD-3, unverified.
#[hook(addr = 0x730510, conv = cdecl, name = "D3DResourceSystem::CreateTexture", expect = "53 55 56 57 8B 7C 24 1C")]
pub fn d3d_resource_create_texture(w: u32, h: u32, levels: u32, format: u32, out: GPtr<u32>) -> i32 {
    let levels = if (levels as i32) > 1 { 0 } else { levels };
    let mut found: Option<GPtr<u8>> = None;
    let n = D3D_RES_C87C60.get() as i32;
    if w == h && n > 1 {
        let wf = w as i32 as f32;
        for i in 1..n {
            let e = GPtr::<u8>::from_addr(D3D_TEXTURE_BUFFERS + 0x1C * i as usize);
            let ew = e.byte_add(4).cast::<i32>().read();
            if (ew as f64) == (wf as f64)
                && e.byte_add(8).cast::<u32>().read() == levels
                && e.cast::<u32>().read() == format
            {
                found = Some(e);
                break;
            }
        }
    }
    let tex = match found {
        Some(e) => {
            let c = e.byte_add(0x14).cast::<u32>();
            if c.read() != 0 {
                let p = e.byte_add(0x10).cast::<u32>();
                p.write(p.read().wrapping_sub(1));
                c.write(c.read() - 1);
                let list = GPtr::<u32>::from_addr(e.byte_add(0x18).cast::<u32>().read() as usize);
                list.wrapping_index(c.read() as i32).read()
            } else {
                0
            }
        }
        None => d3dres::texture_buffer_pop(GPtr::from_addr(D3D_TEXTURE_BUFFERS), format, w, h, levels).addr() as u32,
    };
    out.write(tex);
    if tex != 0 {
        return 0;
    }
    let dev = GPtr::<u8>::from_addr(GPtr::<u32>::from_addr(0xC97C28).read() as usize);
    d3dres::dev_create_texture(dev, w, h, levels, 0, format, 1, out, 0)
}


mod d3dres2 {
    use crate::mem::GPtr;
    crate::orig! {
        /// `IUnknown::Release()` (+8)
        pub fn release(o: GPtr<u8>) -> u32 = stdcall vslot 0x8 => unported;
        /// CRT `operator new` (0x82119A)
        pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
        /// CRT `operator delete` (0x8214BD)
        pub fn operator_delete(p: GPtr<u8>) = cdecl 0x8214BD => unported;
        /// `D3DTextureBuffer::Setup(format, width, levels, capacity)` (0x72FE80)
        pub fn setup(this: GPtr<u8>, format: u32, w: u32, levels: u32, cap: u32) = thiscall 0x72FE80 => super::texture_buffer_setup;
        /// `D3DTextureBuffer::Push(texture)` (0x72FFF0)
        pub fn push(this: GPtr<u8>, tex: GPtr<u8>) -> u8 = thiscall 0x72FFF0 => super::texture_buffer_push;
        /// `D3DTextureBuffer::Resize(capacity)` (0x730020)
        pub fn resize(this: GPtr<u8>, cap: u32) = thiscall 0x730020 => super::texture_buffer_resize;
        /// `D3DTextureBuffer::PushWithoutIncreasingCounter(texture)` (0x730AD0)
        pub fn push_without_increasing_counter(this: GPtr<u8>, tex: GPtr<u8>) -> u8 = thiscall 0x730AD0 => super::texture_buffer_push_without_increasing_counter;
    }
}

fn tb_u32(b: GPtr<u8>, off: isize) -> GPtr<u32> {
    b.byte_add(off).cast::<u32>()
}

/// `D3DTextureBuffer::Setup(format, width, levels, capacity)` (0x72FE80, thiscall, `ret 0x10`):
/// format, width and levels stored, a list of `capacity` pointers (`operator new`), capacity set,
/// count and pushed counter 0. Written by STD-4, unverified.
///
/// Not registered: 0x72FE80 is hooked by sa-render mvpd_d3d.rs (duplicate port left unregistered when STD-5 merged both).
pub fn texture_buffer_setup(this: GPtr<u8>, format: u32, w: u32, levels: u32, cap: u32) {
    tb_u32(this, 0).write(format);
    tb_u32(this, 4).write(w);
    tb_u32(this, 8).write(levels);
    let list = d3dres2::operator_new(cap.wrapping_mul(4));
    tb_u32(this, 0x18).write(list.addr() as u32);
    tb_u32(this, 0xC).write(cap);
    tb_u32(this, 0x14).write(0);
    tb_u32(this, 0x10).write(0);
}

/// `D3DTextureBuffer::Push(texture)` (0x72FFF0, thiscall, `ret 4`): the pushed counter (+0x10)
/// incremented; with room (count < capacity, signed) the texture appended, 1; else 0. Written by
/// STD-4, unverified.
#[hook(addr = 0x72FFF0, conv = thiscall, name = "D3DTextureBuffer::Push", expect = "8B 41 14 8B 51 0C 56",
       route = "L2", hazards = "ax-al-return", recs = "boot")]
pub fn texture_buffer_push(this: GPtr<u8>, tex: GPtr<u8>) -> u8 {
    let n = tb_u32(this, 0x14).read();
    tb_u32(this, 0x10).write(tb_u32(this, 0x10).read().wrapping_add(1));
    if (n as i32) >= (tb_u32(this, 0xC).read() as i32) {
        return 0;
    }
    GPtr::<u32>::from_addr(tb_u32(this, 0x18).read() as usize).wrapping_index(n as i32).write(tex.addr() as u32);
    tb_u32(this, 0x14).write(n.wrapping_add(1));
    1
}

/// `D3DTextureBuffer::Resize(capacity)` (0x730020, thiscall, `ret 4`): nothing for the current
/// capacity; else a new list (`operator new`) receives the first `min(count, capacity)` entries,
/// the entries from `capacity` up to the count are released (`IUnknown::Release`; the count is not
/// lowered), the old list freed (`operator delete`), the new one and the capacity stored. Written
/// by STD-4, unverified.
#[hook(addr = 0x730020, conv = thiscall, name = "D3DTextureBuffer::Resize", expect = "53 8B 5C 24 08 56 8B F1",
       unverified = "std4-d3d9-3", route = "L2", hazards = "heap-writes", recs = "boot")]
pub fn texture_buffer_resize(this: GPtr<u8>, cap: u32) {
    if cap == tb_u32(this, 0xC).read() {
        return;
    }
    let list = d3dres2::operator_new(cap.wrapping_mul(4)).cast::<u32>();
    let old = |i: i32| GPtr::<u32>::from_addr(tb_u32(this, 0x18).read() as usize).wrapping_index(i);
    let mut i: i32 = 0;
    while i < tb_u32(this, 0x14).read() as i32 && i < cap as i32 {
        list.wrapping_index(i).write(old(i).read());
        i += 1;
    }
    let mut j = cap as i32;
    while j < tb_u32(this, 0x14).read() as i32 {
        d3dres2::release(GPtr::from_addr(old(j).read() as usize));
        j += 1;
    }
    d3dres2::operator_delete(GPtr::from_addr(tb_u32(this, 0x18).read() as usize));
    tb_u32(this, 0x18).write(list.addr() as u32);
    tb_u32(this, 0xC).write(cap);
}

/// `D3DTextureBuffer::PushWithoutIncreasingCounter(texture)` (0x730AD0, thiscall, `ret 4`): with
/// room the texture appended, 1; else 0 (the pushed counter is rewritten unchanged). Written by
/// STD-4, unverified.
#[hook(addr = 0x730AD0, conv = thiscall, name = "D3DTextureBuffer::PushWithoutIncreasingCounter", expect = "8B 41 10 8B 51 0C 89 41 10",
       unverified = "std4-d3d9-3", route = "L2", hazards = "ax-al-return", recs = "boot")]
pub fn texture_buffer_push_without_increasing_counter(this: GPtr<u8>, tex: GPtr<u8>) -> u8 {
    tb_u32(this, 0x10).write(tb_u32(this, 0x10).read());
    let n = tb_u32(this, 0x14).read();
    if (n as i32) >= (tb_u32(this, 0xC).read() as i32) {
        return 0;
    }
    GPtr::<u32>::from_addr(tb_u32(this, 0x18).read() as usize).wrapping_index(n as i32).write(tex.addr() as u32);
    tb_u32(this, 0x14).write(n.wrapping_add(1));
    1
}

/// `D3DResourceSystem::DestroyTexture(texture)` (0x730B70). Without resource buffering (0x8D6084)
/// the texture is released. Else it is pooled by its level-0 desc and level count (single = one
/// level): a square one joins buffer 1..`[0xC87C60]` of its width (the integer width converted to
/// float, unsigned fix-up, stored as f32, compared with each buffer's `fild` width), format and
/// single-ness (pushed counter +1; appended, or the list doubled and appended without counting);
/// otherwise a square one of 32..256 with fewer than 17 buffers opens a new buffer (capacity 16) and
/// is pushed there (wider: released); any other texture up to 256x256 goes to buffer 0 (pushed
/// counter +1; appended, or the list doubled first; still full: neither stored nor released), larger
/// ones are released. Written by STD-4, unverified.
///
/// Not registered: 0x730B70 is hooked by sa-render mvpd_d3d.rs (duplicate port left unregistered when STD-5 merged both).
pub fn d3d_resource_destroy_texture(tex: GPtr<u8>) {
    if D3D_RES_BUFFERING.get() == 0 {
        d3dres2::release(tex);
        return;
    }
    let single: u32 = if d3dres::tex_get_level_count(tex) > 1 { 0 } else { 1 };
    let mut desc = [0u32; 8];
    d3dres::tex_get_level_desc(tex, 0, GPtr::from_raw(desc.as_mut_ptr()));
    let (format, w, h) = (desc[0], desc[6], desc[7]);
    let n = D3D_RES_C87C60.get() as i32;
    if w == h && n > 1 {
        let mut fw = w as i32 as f64;
        if (w as i32) < 0 {
            fw += 4_294_967_296.0;
        }
        let f = fw as f32;
        for i in 1..n {
            let b = GPtr::<u8>::from_addr(D3D_TEXTURE_BUFFERS + 0x1C * i as usize);
            if (tb_u32(b, 4).read() as i32 as f64) == f as f64 && format == tb_u32(b, 0).read() && single == tb_u32(b, 8).read() {
                tb_u32(b, 0x10).write(tb_u32(b, 0x10).read().wrapping_add(1));
                let (count, cap) = (tb_u32(b, 0x14).read(), tb_u32(b, 0xC).read());
                if (count as i32) < (cap as i32) {
                    GPtr::<u32>::from_addr(tb_u32(b, 0x18).read() as usize).wrapping_index(count as i32).write(tex.addr() as u32);
                    tb_u32(b, 0x14).write(count.wrapping_add(1));
                } else {
                    d3dres2::resize(b, cap.wrapping_mul(2));
                    d3dres2::push_without_increasing_counter(b, tex);
                }
                return;
            }
        }
    }
    let n = D3D_RES_C87C60.get();
    if (n as i32) < 0x11 && w == h && w >= 0x20 {
        if w > 0x100 {
            d3dres2::release(tex);
            return;
        }
        let b = GPtr::<u8>::from_addr(D3D_TEXTURE_BUFFERS + 0x1C * n as usize);
        d3dres2::setup(b, format, w, single, 0x10);
        let n = D3D_RES_C87C60.get();
        d3dres2::push(GPtr::from_addr(D3D_TEXTURE_BUFFERS + 0x1C * n as usize), tex);
        D3D_RES_C87C60.set(n.wrapping_add(1));
        return;
    }
    if w > 0x100 || h > 0x100 {
        d3dres2::release(tex);
        return;
    }
    let b = GPtr::<u8>::from_addr(D3D_TEXTURE_BUFFERS);
    tb_u32(b, 0x10).write(tb_u32(b, 0x10).read().wrapping_add(1));
    if (tb_u32(b, 0x14).read() as i32) >= (tb_u32(b, 0xC).read() as i32) {
        d3dres2::resize(b, tb_u32(b, 0xC).read().wrapping_mul(2));
        if (tb_u32(b, 0x14).read() as i32) >= (tb_u32(b, 0xC).read() as i32) {
            return;
        }
    }
    let count = tb_u32(b, 0x14).read();
    GPtr::<u32>::from_addr(tb_u32(b, 0x18).read() as usize).wrapping_index(count as i32).write(tex.addr() as u32);
    tb_u32(b, 0x14).write(count.wrapping_add(1));
}

// ------------------------------------------------------------------ Game:: (app_game) -----------

/// `Game::RwTerminate` (0x53D910, cdecl): `CGame::ShutdownRenderWare`; if the gamma flag is set,
/// `SetGammaRamp(0, D3DSGR_CALIBRATE = 1, saved ramp)` on the current device (when there is one) and clear
/// the flag; tail jump into `RsRwTerminate`.
#[hook(addr = 0x53D910, conv = cdecl, name = "Game::RwTerminate", expect = "E8 6B E2 FF FF")]
pub fn rw_terminate() {
    call::game_shutdown_render_ware();
    if GAMMA_CHANGED.get().get() {
        let dev = call::rw_d3d9_get_current_d3d_device();
        if !dev.is_null() {
            call::d3d_device_set_gamma_ramp(dev, 0, 1, at(SAVED_GAMMA_RAMP));
        }
        GAMMA_CHANGED.set(GBool::FALSE);
    }
    call::rs_rw_terminate();
}

/// `Game::TheGame` (0x53DF10, cdecl; unused by the game).
#[hook(addr = 0x53DF10, conv = cdecl, name = "Game::TheGame", expect = "68 20 3B 86 00")]
pub fn the_game() {
    call::printf(STR_INTO_THE_GAME.ptr().cast());
    call::timer_initialise();
    call::game_initialise(STR_GTA_DAT.ptr().cast());
    call::loading_screen_set_loading_bar_msg(STR_STARTING_GAME.ptr().cast(), GPtr::NULL);
    call::loading_screen_new_chunk_loaded();
}

/// `Game::RenderDebugShit` (0x53E160, cdecl): empty in 1.0 US (`ret`). gta-reversed calls
/// `CTheScripts::RenderTheScriptDebugLines` and `DefinedState` here (NOTSA / !FINAL); not followed.
// Not hooked: a lone `ret` cannot be relocated (the installer needs 5 bytes of straight-line code).
pub fn render_debug_shit() {}

/// `Game::RenderMenus` (0x53E530, cdecl): `if (m_bMenuActive) FrontEndMenuManager.DrawFrontEnd()`.
#[hook(addr = 0x53E530, conv = cdecl, name = "Game::RenderMenus", expect = "A0 A4 67 BA 00")]
pub fn render_menus() {
    if MENU_ACTIVE.get().get() {
        call::menu_draw_front_end(at(FRONTEND_MENU_MANAGER));
    }
}

/// `Game::InitialiseGame` (0x53E580, cdecl). The version number is stored before the call.
#[hook(addr = 0x53E580, conv = cdecl, name = "Game::InitialiseGame", expect = "68 10 3B 86 00")]
pub fn initialise_game() {
    VERSION_NUMBER.set(78);
    call::game_initialise(STR_GTA_DAT.ptr().cast());
    call::audio_engine_restart(at(AUDIO_ENGINE));
    if MENU_LOADING_DATA.get().get() {
        MENU_START_GAME_LOADING.set(GBool::TRUE);
    }
}

/// `Game::RwInitialize(void*)` (0x5BF390, cdecl): EAX = `RsRwInitialize(param) ?
/// (uint8)CGame::InitialiseRenderWare() : 0` (`movzx eax, al`).
#[hook(addr = 0x5BF390, conv = cdecl, name = "Game::RwInitialize", expect = "8B 44 24 04 50")]
pub fn rw_initialize(param: GPtr<u8>) -> u32 {
    if call::rs_rw_initialize(param) == 0 {
        return 0;
    }
    call::game_initialise_render_ware() as u32
}

/// `Game::GameInit` (0x5BF3B0, cdecl). The original inlines `RwInitialize(NULL)` (result ignored),
/// then `srand(RsTimer())`.
#[hook(addr = 0x5BF3B0, conv = cdecl, name = "Game::GameInit", expect = "6A 00 E8 D9 A8 05 00")]
pub fn game_init() {
    if call::rs_rw_initialize(GPtr::NULL) != 0 {
        call::game_initialise_render_ware();
    }
    let t = call::rs_timer();
    // return address of the original's `call srand` (0x5BF3C9), for the determinism rand log
    crate::orig::at_site(0x5BF3CE, || crt::srand(t));
}

/// Colour of the fade rectangle (`DoFade`): black while the fade target is the splash screen,
/// otherwise `(c * v) / v` per channel (signed `idiv`, v != 0 here) and alpha `v`.
pub fn fade_colour(rgbv: [u8; 4], target_is_splash: bool) -> [u8; 4] {
    let v = rgbv[3] as i32;
    if target_is_splash || v == 0 {
        return [0; 4];
    }
    let ch = |c: u8| ((c as i32 * v) / v) as u8;
    // alpha: `setl al; dec al; and eax, ecx` = v for every non-negative v
    [ch(rgbv[0]), ch(rgbv[1]), ch(rgbv[2]), rgbv[3]]
}

/// The fade rectangle `{left -5, top -5, right W + 5, bottom H + 5}` (memory order of `CRect`);
/// `fild` of the int then `fstp dword` (PC = 24).
pub fn fade_rect(width: i32, height: i32) -> [f32; 4] {
    let w = X87::from_i32(width.wrapping_add(5)).to_f32();
    let h = X87::from_i32(height.wrapping_add(5)).to_f32();
    [-5.0, -5.0, w, h]
}

/// `DoFade` (0x53E600, cdecl).
///
/// The bool globals are compared with `cmp byte, 1` (exactly 1, not "non-zero") for
/// `JustLoadedDontFadeInYet` and `StillToFadeOut`; the pause test is `m_UserPause || m_CodePause`
/// (byte != 0); the elapsed time is compared unsigned with `FadeOutTime`.
#[hook(addr = 0x53E600, conv = cdecl, name = "DoFade", expect = "A0 49 CB B7 00")]
pub fn do_fade() {
    if crate::timer::USER_PAUSE.get().0 != 0 || crate::timer::CODE_PAUSE.get().0 != 0 {
        return;
    }
    let now = crate::timer::TIME_MS.get();
    if JUST_LOADED_DONT_FADE_IN_YET.get().0 == 1 {
        FADE_TIMER.set(now);
        JUST_LOADED_DONT_FADE_IN_YET.set(GBool::FALSE);
    }
    if STILL_TO_FADE_OUT.get().0 == 1 {
        let cam = at::<u8>(THE_CAMERA);
        if now.wrapping_sub(FADE_TIMER.get()) > FADE_OUT_TIME.get() {
            STILL_TO_FADE_OUT.set(GBool::FALSE);
            call::camera_fade(cam, 3.0, 1);
            call::camera_process_fade(cam);
            call::camera_process_music_fade(cam);
        } else {
            call::camera_set_fade_colour(cam, 0, 0, 0);
            call::camera_fade(cam, 0.0, 0);
            call::camera_process_fade(cam);
        }
    }
    let rgbv = FADE_RGBV.get();
    if rgbv[3] == 0 {
        return;
    }
    let mut colour = fade_colour(rgbv, CAMERA_FADE_TARGET_IS_SPLASH.get().get());
    let mut rect = fade_rect(RS_MAXIMUM_WIDTH.get(), RS_MAXIMUM_HEIGHT.get());
    call::sprite2d_draw_rect(GPtr::from_raw(&raw mut rect), GPtr::from_raw(&raw mut colour));
}

/// `TestModelIndices` (0x53ED30, cdecl): empty in 1.0 US (`ret`, nop padding).
// Not hooked: a lone `ret` cannot be relocated (the installer needs 5 bytes of straight-line code).
pub fn test_model_indices() {}

/// `CMenuManager::NoDiskInDriveMessage` (0x57C5E0, thiscall). One frame of the "insert disc" screen:
/// `DoRWStuffStartOfFrame(0, …)`; unless `RsGlobal.quit`: message loop, pads, `MessageScreen("NO_PCCD",
/// true, false)`, fonts, end of frame; `CPad::GetPad(0)` (result unused), then if pad 0's button word
/// at 0xB733A8 is set and the one at 0xB73138 is clear: `m_bQuitGameNoDVD (+0x58) = true` and
/// `RsEventHandler(rsQUITAPP = 0x1E, NULL)`.
#[hook(addr = 0x57C5E0, conv = thiscall, name = "CMenuManager::NoDiskInDriveMessage", expect = "56 6A 00 6A 00", this = "CMenuManager")]
pub fn no_disk_in_drive_message(menu: GPtr<u8>) {
    call::do_rw_stuff_start_of_frame(0, 0, 0, 0, 0, 0, 0);
    if RS_QUIT.get() != 0 {
        return;
    }
    call::message_loop();
    call::pad_update_pads();
    call::menu_message_screen(menu, STR_NO_PCCD.ptr().cast(), 1, 0);
    call::font_draw_fonts();
    call::do_rw_stuff_end_of_frame();
    call::pad_get_pad(0);
    if PAD0_NEW_CROSS.get() != 0 && PAD0_OLD_CROSS.get() == 0 {
        menu.byte_add(0x58).write(1u8);
        call::rs_event_handler(0x1E, GPtr::NULL);
    }
}

/// This module's registrations (collected by [`crate::hooks`]).
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_INITIALISE_ONCE_BEFORE_RW,
    HOOK_TEXTURE_BUFFER_PUSH,
    HOOK_TEXTURE_BUFFER_RESIZE,
    HOOK_TEXTURE_BUFFER_PUSH_WITHOUT_INCREASING_COUNTER,
    HOOK_FINAL_SHUTDOWN,
    HOOK_INIT_AFTER_LOST_FOCUS,
    HOOK_CAN_SEE_OUTSIDE_FROM_CURR_AREA,
    HOOK_CAN_SEE_WATER_FROM_CURR_AREA,
    HOOK_TIDY_UP_MEMORY,
    HOOK_DRASTIC_TIDY_UP_MEMORY,
    HOOK_D3D_DEVICE_RESTORE_CALLBACK,
    HOOK_INITIALISE_ESSENTIALS_AFTER_RW,
    HOOK_VALIDATE_VERSION,
    HOOK_CAMERA_DESTROY,
    HOOK_INITIALISE_RENDER_WARE,
    HOOK_D3D_RESOURCE_SYSTEM_INIT,
    HOOK_TEXTURE_BUFFER_POP,
    HOOK_D3D_RESOURCE_CREATE_TEXTURE,
    HOOK_RW_TERMINATE,
    HOOK_THE_GAME,
    HOOK_RENDER_MENUS,
    HOOK_INITIALISE_GAME,
    HOOK_RW_INITIALIZE,
    HOOK_GAME_INIT,
    HOOK_DO_FADE,
    HOOK_NO_DISK_IN_DRIVE_MESSAGE,
    HOOK_TIDY_UP_D3D_INDEX_BUFFERS,
    HOOK_TIDY_UP_D3D_TEXTURES,
];

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    #[test]
    fn area_predicates() {
        for (a, out, water) in [(0, 1, 1), (5, 0, 1), (1, 0, 0), (-1, 0, 0)] {
            CURR_AREA.set(a);
            assert_eq!(can_see_outside_from_curr_area(), out);
            assert_eq!(can_see_water_from_curr_area(), water);
        }
        CURR_AREA.set(0);
    }

    #[test]
    fn lost_focus_and_menus() {
        MENU_ACTIVATE_NEXT_FRAME.set(GBool::FALSE);
        MENU_ACTIVE.set(GBool(2));
        init_after_lost_focus();
        assert_eq!(MENU_ACTIVATE_NEXT_FRAME.get(), GBool::FALSE);
        MENU_ACTIVE.set(GBool::FALSE);
        init_after_lost_focus();
        assert_eq!(MENU_ACTIVATE_NEXT_FRAME.get(), GBool::TRUE);
        MENU_ACTIVATE_NEXT_FRAME.set(GBool::FALSE);
        // menu inactive: no DrawFrontEnd call (it would panic as unported)
        render_menus();
    }

    #[test]
    fn restore_callback_without_previous() {
        PREV_D3D_RESTORE_CALLBACK.set(0);
        RT_SHADOW_INITIALISED.set(GBool::FALSE);
        RT_SHADOW_NEEDS_REINIT.set(GBool::FALSE);
        MIRRORS_D3D_RESTORED.set(GBool::FALSE);
        d3d_device_restore_callback();
        assert_eq!(RT_SHADOW_NEEDS_REINIT.get(), GBool::FALSE);
        assert_eq!(MIRRORS_D3D_RESTORED.get(), GBool::TRUE);
        RT_SHADOW_INITIALISED.set(GBool::TRUE);
        d3d_device_restore_callback();
        assert_eq!(RT_SHADOW_NEEDS_REINIT.get(), GBool::TRUE);
    }

    #[test]
    fn fade_parts() {
        assert_eq!(fade_colour([10, 20, 30, 0], false), [0; 4]);
        assert_eq!(fade_colour([10, 20, 30, 200], true), [0; 4]);
        assert_eq!(fade_colour([10, 20, 255, 200], false), [10, 20, 255, 200]);
        assert_eq!(fade_rect(640, 480), [-5.0, -5.0, 645.0, 485.0]);
        // fild + fstp dword rounds to 24 bits
        assert_eq!(fade_rect(16_777_216, 0)[2], 16_777_220.0);
        assert_eq!(decode_version_byte(23), 0);
        assert_eq!(decode_version_byte(b'g' + 23), b'g');
    }
}
