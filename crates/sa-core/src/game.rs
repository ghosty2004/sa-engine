//! CORE ports: `CGame` / `Game::` (app_game) leaves and short sequences (PORT-CORE).
//!
//! Ported from the 1.0 US bytes (gta-reversed `Game.cpp`, `app/app_game.cpp`, `Draw.cpp` as reading
//! aid). The big orchestrators (`CGame::Initialise`, `Init1/2/3`, `Process`, `Shutdown`, `Game::Idle`,
//! `RenderScene`, …) are deferred until the subsystems they sequence are ported; see
//! `docs/arch/core-port.md`. Every callee goes through `orig!` (phase A: the original address, so its
//! own hook decides; phases B/C: `unported` until the callee's subsystem exists).

use crate::mem::{GBool, GPtr};
use crate::platform::{RS_MAXIMUM_HEIGHT, RS_MAXIMUM_WIDTH, RS_QUIT};
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
        pub fn tidy_up_d3d_textures(n: u32) = cdecl 0x7305E0 => unported;
        pub fn tidy_up_d3d_index_buffers(n: u32) = cdecl 0x730740 => unported;
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
        pub fn do_rw_stuff_start_of_frame(a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, g: u32) = cdecl 0x53D690 => unported;
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
        pub fn game_initialise_render_ware() -> u8 = cdecl 0x5BD600 => unported;
        pub fn rs_timer() -> u32 = cdecl 0x619410 => unported;
        pub fn rw_object_has_frame_set_frame(object: GPtr<u8>, frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x804EF0 => unported;
        pub fn rw_frame_destroy(frame: GPtr<u8>) -> i32 = cdecl 0x7F05A0 => unported;
        pub fn rw_raster_destroy(raster: GPtr<u8>) -> i32 = cdecl 0x7FB020 => unported;
        pub fn rw_camera_destroy(camera: GPtr<u8>) -> i32 = cdecl 0x7EE4B0 => unported;
        /// the chained restore callback stored at 0xBD0114
        pub fn prev_d3d_restore_callback() = cdecl cell 0xBD0114 => unported;
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
    HOOK_RW_TERMINATE,
    HOOK_THE_GAME,
    HOOK_RENDER_MENUS,
    HOOK_INITIALISE_GAME,
    HOOK_RW_INITIALIZE,
    HOOK_GAME_INIT,
    HOOK_DO_FADE,
    HOOK_NO_DISK_IN_DRIVE_MESSAGE,
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
