//! `CLoadingScreen` (0x53DED0, 0x58FF10–0x590DDD, 0x744DB5) and its static initialisers
//! (0x84F2B0, 0x84F2D0, 0x8565E0).
//!
//! Ported from the 1.0 US bytes (gta-reversed `LoadingScreen.cpp` as reading aid,
//! `docs/re/specs/STREAM/CLoadingScreen.deviations.md`, `_static-init.md`). The clock is
//! `(float)(uint32)RsTimer() * 0.001f` (x87 PC=24, unsigned via `fild` + 2^32) minus `m_PauseTime`,
//! kept in ST0 by `GetClockTime` and by every inlined copy. Original behaviour kept:
//! - `LoadSplashes`: the TXD slot is "loadscs"; `srand(QPC.LowPart)` then a 15-entry draw with 14
//!   `rand()` calls picks splashes 1..6 from loadsc1..loadsc14 (gr-deviations `LoadingScreen.cpp:127`);
//!   any logo id other than 1 is "eax" (`:142`);
//! - `RenderLoadingBar`: `m_TimeBarAppeared` is set only after the bar was drawn and only while 0
//!   (spec); the bar geometry is computed (and `_ftol`ed) before the legal-screen test;
//! - `DoPCScreenChange`: no wrap of `m_currDisplayedSplash` (`:334`); tune amplitude
//!   `(255 - alpha) / 255` when fading out, else 1, for `isLastOne || fadeIn` (spec); inlined Shutdown;
//! - `NewChunkLoaded`: any splash other than 0 (also -1) uses the 5.0 s limit (`:395`);
//! - *new*: `Update` uses the 2.0 s fade only when `m_currDisplayedSplash >= 0` and (legal screen or
//!   fading in from black); gr also for splash -1. Its percent is `min(100, ((now - pause - appeared)
//!   * 100) / (36 - (appeared - started)))` (constant 36.0 at 0x863204), and the legal screen
//!   advances when `now - pause - started > 5.5`, otherwise the next splash is shown once
//!   `splash * 16.666666 < percent` (gr computes neither of these);
//! - `RenderSplash` with `m_currDisplayedSplash == 0` while cross-fading draws "splash -1", i.e. the
//!   f32 global 0xBAB358 in front of `m_aSplashes` read as a `CSprite2d` (original behaviour, reproduced
//!   by address arithmetic);
//! - `CLoadingScreen::IsActive` (0x744DB5) and `Pause` (0x590310) are a lone `ret` in 1.0 US.

use sa_core::mem::GPtr;
/// `CLoadingScreen::m_bActive` (0xBAB318), declared by sa-core.
pub use sa_core::platform::LOADING_SCREEN_ACTIVE as ACTIVE;
use sa_core::platform::{RS_MAXIMUM_HEIGHT, RS_MAXIMUM_WIDTH, SCENE_CAMERA};
use sa_core::{crt, hook, orig};
use sa_math::x87::X87;

use crate::shared::image;

sa_core::globals! {
    list GLOBALS;
    /// `m_PopUpMessage[16]`
    pub static POP_UP_MESSAGE @ 0xBAB268: [u8; 16] = [0; 16];
    /// `m_LoadingGxtMsg2[80]` (GxtChar = 1 byte)
    pub static LOADING_GXT_MSG2 @ 0xBAB278: [u8; 80] = [0; 80];
    /// `m_LoadingGxtMsg1[80]`
    pub static LOADING_GXT_MSG1 @ 0xBAB2C8: [u8; 80] = [0; 80];
    /// `m_bWantToPause`
    pub static WANT_TO_PAUSE @ 0xBAB319: u8 = 0;
    /// `m_bPaused`
    pub static PAUSED @ 0xBAB31A: u8 = 0;
    /// `m_bFading`
    pub static FADING @ 0xBAB31C: u8 = 0;
    /// `m_bLegalScreen`
    pub static LEGAL_SCREEN @ 0xBAB31D: u8 = 0;
    /// `m_bFadeInNextSplashFromBlack`
    pub static FADE_IN_FROM_BLACK @ 0xBAB31E: u8 = 0;
    /// `m_bFadeOutCurrSplashToBlack`
    pub static FADE_OUT_TO_BLACK @ 0xBAB31F: u8 = 0;
    /// `m_FadeAlpha`
    pub static FADE_ALPHA @ 0xBAB320: u8 = 0;
    /// `m_StartFadeTime`
    pub static START_FADE_TIME @ 0xBAB324: f32 = 0.0;
    /// `m_ClockTimeOnPause`
    pub static CLOCK_TIME_ON_PAUSE @ 0xBAB328: f32 = 0.0;
    /// `m_PauseTime`
    pub static PAUSE_TIME @ 0xBAB32C: f32 = 0.0;
    /// `m_PercentLoaded`
    pub static PERCENT_LOADED @ 0xBAB330: f32 = 0.0;
    /// `m_TimeStartedLoading`
    pub static TIME_STARTED_LOADING @ 0xBAB334: f32 = 0.0;
    /// `m_TimeBarAppeared`
    pub static TIME_BAR_APPEARED @ 0xBAB338: f32 = 0.0;
    /// `m_bSignalDelete`
    pub static SIGNAL_DELETE @ 0xBAB33C: u8 = 0;
    /// `m_bReadyToDelete`
    pub static READY_TO_DELETE @ 0xBAB33D: u8 = 0;
    /// `m_timeSinceLastScreen`
    pub static TIME_SINCE_LAST_SCREEN @ 0xBAB340: f32 = 0.0;
    /// unnamed f32 set by `__init_g_bab358` (0.277778 / 50)
    pub static G_BAB358 @ 0xBAB358: f32 = 0.0;
    /// `m_aSplashes[7]` (CSprite2d = one RwTexture*)
    pub static SPLASHES @ 0xBAB35C: [GPtr<u8>; 7] = [GPtr::NULL; 7];
    /// `m_currDisplayedSplash`
    pub static CURR_DISPLAYED_SPLASH @ 0x8D093C: i32 = -1;
    /// `m_numChunksLoaded`
    pub static NUM_CHUNKS_LOADED @ 0x8D0940: i32 = -1;
    /// `m_chunkBarAppeared`
    pub static CHUNK_BAR_APPEARED @ 0x8D0944: i32 = -1;
}

/// "loadscs" (0x866CC4), "loadscs.txd" (0x866CF0), "MODELS\\TXD\\" (0x866CFC), "" (0x858B54),
/// "nvidia" (0x866CE8), "eax" (0x866CE4), "title_pc_US" (0x866CD8), "loadsc%d" (0x866CCC)
pub const STR_LOADSCS: u32 = 0x866CC4;
pub const STR_LOADSCS_TXD: u32 = 0x866CF0;
pub const STR_MODELS_TXD: u32 = 0x866CFC;
pub const STR_EMPTY: u32 = 0x858B54;
pub const STR_NVIDIA: u32 = 0x866CE8;
pub const STR_EAX: u32 = 0x866CE4;
pub const STR_TITLE: u32 = 0x866CD8;
pub const STR_LOADSC_D: u32 = 0x866CCC;
/// `HudColour` (CHudColours), `AudioEngine`
const HUD_COLOUR: u32 = 0xBAB22C;
const AUDIO_ENGINE: u32 = 0xB6BC90;
/// Return address of LoadSplashes' `call rand` (0x590128 + 5).
pub const RAND_RET_LOAD_SPLASHES: u32 = 0x59012D;
/// `CSprite2d::Constructor` / `Destructor` (static init iterators)
pub const SPRITE_CTOR: u32 = 0x727230;
pub const SPRITE_DTOR: u32 = 0x7281E0;
/// `__dtor_CLoadingScreen__m_aSplashes` (registered with `_atexit`)
pub const DTOR_SPLASHES: u32 = 0x8565E0;

orig! {
    /// `RsTimer()` (0x619410)
    pub fn rs_timer() -> u32 = cdecl 0x619410 => unported;
    /// `RsCameraShowRaster(camera)` (0x619440)
    pub fn rs_camera_show_raster(cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x619440 => unported;
    /// `RwCameraBeginUpdate(camera)` (0x7EE190)
    pub fn rw_camera_begin_update(cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EE190 => unported;
    /// `RwCameraEndUpdate(camera)` (0x7EE180)
    pub fn rw_camera_end_update(cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EE180 => unported;
    /// `RwRenderStateSet(state, value)` (`RwEngineInstance` +0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `DefinedState2d()` (0x734750)
    pub fn defined_state_2d() = cdecl 0x734750 => unported;
    /// `CSprite2d::InitPerFrame()` (0x727350)
    pub fn sprite_init_per_frame() = cdecl 0x727350 => unported;
    /// `CSprite2d::Draw(const CRect&, const CRGBA&)` (0x728350)
    pub fn sprite_draw(s: GPtr<u8>, r: GPtr<u8>, c: GPtr<u8>) = thiscall 0x728350 => unported;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727B60)
    pub fn sprite_draw_rect(r: GPtr<u8>, c: GPtr<u8>) = cdecl 0x727B60 => unported;
    /// `CSprite2d::Delete()` (0x727240)
    pub fn sprite_delete(s: GPtr<u8>) = thiscall 0x727240 => unported;
    /// `CSprite2d::SetTexture(const char*)` (0x727270)
    pub fn sprite_set_texture(s: GPtr<u8>, name: GPtr<u8>) = thiscall 0x727270 => unported;
    /// `CSprite2d::DrawBarChart(x, y, width, height, progress, progressAdd, drawPercentage, drawBlackBorder, color, addColor)` (0x728640)
    pub fn sprite_draw_bar_chart(x: f32, y: f32, w: u32, h: u32, p: f32, add: u32, pct: u32, border: u32, color: u32, add_color: u32) = cdecl 0x728640 => unported;
    /// `CRGBA::CRGBA(r, g, b, a)` (0x7170C0)
    pub fn rgba_ctor(c: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CHudColours::GetRGB(CRGBA* out, uint8 colour)` (0x58FEA0, returns `out`)
    pub fn hud_get_rgb(h: GPtr<u8>, out: GPtr<u8>, colour: u32) -> GPtr<u8> = thiscall 0x58FEA0 => unported;
    /// `CAudioEngine::ServiceLoadingTune(float)` (0x5078A0)
    pub fn service_loading_tune(a: GPtr<u8>, amp: f32) = thiscall 0x5078A0 => unported;
    /// `AsciiToGxtChar(const char*, GxtChar*)` (0x718600)
    pub fn ascii_to_gxt_char(src: GPtr<u8>, dst: GPtr<u8>) = cdecl 0x718600 => unported;
    /// `CFileMgr::SetDir(const char*)` (0x5387D0)
    pub fn set_dir(path: GPtr<u8>) -> i32 = cdecl 0x5387D0 => sa_core::filemgr::set_dir;
    /// `CTxdStore::*`
    pub fn add_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731C80 => crate::txdstore::add_txd_slot;
    pub fn load_txd(slot: i32, name: GPtr<u8>) -> u8 = cdecl 0x7320B0 => crate::txdstore::load_txd_filename;
    pub fn txd_add_ref(slot: i32) = cdecl 0x731A00 => crate::txdstore::add_ref;
    pub fn push_current_txd() = cdecl 0x7316A0 => crate::txdstore::push_current_txd;
    pub fn pop_current_txd() = cdecl 0x7316B0 => crate::txdstore::pop_current_txd;
    pub fn set_current_txd(slot: i32) = cdecl 0x7319C0 => crate::txdstore::set_current_txd;
    pub fn find_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => crate::txdstore::find_txd_slot_name;
    pub fn remove_txd(slot: i32) = cdecl 0x731E90 => crate::txdstore::remove_txd;
    pub fn remove_txd_slot(slot: i32) = cdecl 0x731CD0 => crate::txdstore::remove_txd_slot;
    /// KERNEL32 `QueryPerformanceCounter` (IAT 0x8580C8)
    pub fn query_performance_counter(out: GPtr<u64>) -> i32 = stdcall cell 0x8580C8 => unported;
    /// CRT `sprintf(buf, fmt)` / `sprintf(buf, fmt, int)` (0x821BB5)
    pub fn sprintf0(buf: GPtr<u8>, fmt: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
    pub fn sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, v: u32) -> i32 = cdecl 0x821BB5 => unported;
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall) and `destructor iterator` (0x821E02, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    pub fn eh_vector_dtor(arr: GPtr<u8>, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// CRT `_atexit` (0x821D1E)
    pub fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// internal calls through the hook table
    pub fn set_loading_bar_msg_call(m1: GPtr<u8>, m2: GPtr<u8>) = cdecl 0x590240 => set_loading_bar_msg;
    pub fn new_chunk_loaded_call() = cdecl 0x590D00 => new_chunk_loaded;
    pub fn load_splashes_call(use_id: u8, id: u8) = cdecl 0x5900B0 => load_splashes;
    pub fn render_splash_call() = cdecl 0x58FF60 => render_splash;
    pub fn render_loading_bar_call() = cdecl 0x590370 => render_loading_bar;
    pub fn display_next_splash_call() = cdecl 0x5904D0 => display_next_splash;
    pub fn start_fading_call() = cdecl 0x590530 => start_fading;
    pub fn display_pc_screen_call() = cdecl 0x590570 => display_pc_screen;
    pub fn do_pc_screen_change_call(last: u8) = cdecl 0x590AC0 => do_pc_screen_change;
}

/// `fild` of a u32 as the original converts it (`+ 2^32` when negative as int32).
fn u32_x87(v: u32) -> X87 {
    let x = X87::from_i32(v as i32);
    if (v as i32) < 0 { x.fadd_m(4_294_967_296.0) } else { x }
}

/// `(float)(uint32)RsTimer() * 0.001f` in ST0.
fn clock_real() -> X87 {
    u32_x87(rs_timer()).fmul_m(f32::from_bits(0x3A83_126F))
}

/// `&m_aSplashes[i]` (no bounds check; -1 is 0xBAB358).
fn splash(i: i32) -> GPtr<u8> {
    SPLASHES.ptr().cast::<GPtr<u8>>().wrapping_index(i).cast()
}

/// `LoadingScreen(msg1, msg2, msg3)` (0x53DED0, cdecl).
#[hook(addr = 0x53DED0, conv = cdecl, name = "CLoadingScreen::LoadingScreen", expect = "8B 44 24 04 85 C0")]
pub fn loading_screen(msg1: GPtr<u8>, msg2: GPtr<u8>, _msg3: GPtr<u8>) {
    if !msg1.is_null() {
        set_loading_bar_msg_call(msg1, msg2);
    }
    new_chunk_loaded_call();
}

/// The release part of Shutdown (also inlined into DoPCScreenChange).
fn release_splashes() {
    ACTIVE.set(0);
    for i in 0..7 {
        let s = splash(i);
        if !s.cast::<GPtr<u8>>().read().is_null() {
            sprite_delete(s);
        }
    }
    let slot = find_txd_slot(image(STR_LOADSCS));
    if slot != -1 {
        remove_txd(slot);
        remove_txd_slot(slot);
    }
}

/// `CLoadingScreen::Shutdown()` (0x58FF10, cdecl).
#[hook(addr = 0x58FF10, conv = cdecl, name = "CLoadingScreen::Shutdown", expect = "56 C6 05 18 B3 BA 00 00")]
pub fn shutdown() {
    release_splashes();
}

/// `CLoadingScreen::RenderSplash()` (0x58FF60, cdecl).
#[hook(addr = 0x58FF60, conv = cdecl, name = "CLoadingScreen::RenderSplash", expect = "83 EC 18 E8 E8 73 19 00")]
pub fn render_splash() {
    sprite_init_per_frame();
    // CRect {-5, H + 5, W + 5, -5} (fild of the ints, stored as f32)
    let mut rect = [
        -5.0f32,
        RS_MAXIMUM_HEIGHT.get().wrapping_add(5) as f32,
        RS_MAXIMUM_WIDTH.get().wrapping_add(5) as f32,
        -5.0,
    ];
    let mut color = [0u8; 4];
    let c = GPtr::from_raw(color.as_mut_ptr());
    rgba_ctor(c, 255, 255, 255, 255);
    rw_render_state_set(2, 3);
    let r: GPtr<u8> = GPtr::from_raw(rect.as_mut_ptr()).cast();
    let curr = CURR_DISPLAYED_SPLASH.get();
    if FADING.get() == 0 {
        if READY_TO_DELETE.get() == 0 {
            sprite_draw(splash(curr), r, c);
        }
        return;
    }
    sprite_draw(splash(curr), r, c);
    let fade_in = FADE_IN_FROM_BLACK.get() != 0;
    if !fade_in && FADE_OUT_TO_BLACK.get() == 0 {
        c.byte_add(3).write(255u8.wrapping_sub(FADE_ALPHA.get()));
        sprite_draw(splash(CURR_DISPLAYED_SPLASH.get() - 1), r, c);
        return;
    }
    c.write(0);
    c.byte_add(1).write(0);
    c.byte_add(2).write(0);
    c.byte_add(3).write(if fade_in { 255u8.wrapping_sub(FADE_ALPHA.get()) } else { FADE_ALPHA.get() });
    sprite_draw_rect(r, c);
}

/// `CLoadingScreen::LoadSplashes(bool useSplashId, uint8 id)` (0x5900B0, cdecl).
#[hook(addr = 0x5900B0, conv = cdecl, name = "CLoadingScreen::LoadSplashes", expect = "83 EC 40 53 55")]
#[allow(clippy::needless_range_loop)] // k is also the draw counter of the original loop
pub fn load_splashes(use_id: u8, id: u8) {
    set_dir(image(STR_MODELS_TXD));
    let slot = add_txd_slot(image(STR_LOADSCS));
    load_txd(slot, image(STR_LOADSCS_TXD));
    txd_add_ref(slot);
    push_current_txd();
    set_current_txd(slot);
    let mut pool: [u8; 15] = core::array::from_fn(|k| k as u8);
    let mut qpc: u64 = 0;
    query_performance_counter(GPtr::from_raw(&raw mut qpc));
    crt::srand(qpc as u32);
    let mut out = [0u8; 15];
    let mut rem: i32 = 14;
    for k in 0..15 {
        let idx = if k == 0 {
            0
        } else {
            let r = sa_core::orig::at_site(RAND_RET_LOAD_SPLASHES, crt::rand);
            sa_math::general::get_random_number_in_range_i32(r, 0, rem + 1)
        };
        out[k] = pool[idx as usize];
        if idx < rem {
            pool.copy_within(idx as usize + 1..=rem as usize, idx as usize);
        }
        rem -= 1;
    }
    out[14] = pool[0];
    let mut name = [0u8; 20];
    let buf = GPtr::from_raw(name.as_mut_ptr());
    for i in 0..7 {
        if use_id != 0 {
            sprintf0(buf, image(if id == 1 { STR_NVIDIA } else { STR_EAX }));
        } else if i == 0 {
            sprintf0(buf, image(STR_TITLE));
        } else {
            sprintf1(buf, image(STR_LOADSC_D), out[i as usize] as u32);
        }
        sprite_set_texture(splash(i), buf);
    }
    pop_current_txd();
    set_dir(image(STR_EMPTY));
}

/// `CLoadingScreen::DisplayMessage(const char*)` (0x590220, cdecl): `strcpy` into the 16-byte
/// pop-up buffer (no length limit).
#[hook(addr = 0x590220, conv = cdecl, name = "CLoadingScreen::DisplayMessage", expect = "8B 44 24 04 BA 68 B2 BA 00")]
pub fn display_message(msg: GPtr<u8>) {
    let dst = POP_UP_MESSAGE.ptr().cast::<u8>();
    let mut k = 0;
    loop {
        let c = msg.byte_add(k).read();
        dst.byte_add(k).write(c);
        k += 1;
        if c == 0 {
            break;
        }
    }
}

/// `CLoadingScreen::SetLoadingBarMsg(const char*, const char*)` (0x590240, cdecl).
#[hook(addr = 0x590240, conv = cdecl, name = "CLoadingScreen::SetLoadingBarMsg", expect = "8B 44 24 04 85 C0")]
pub fn set_loading_bar_msg(m1: GPtr<u8>, m2: GPtr<u8>) {
    if !m1.is_null() {
        ascii_to_gxt_char(m1, LOADING_GXT_MSG1.ptr().cast());
    } else {
        LOADING_GXT_MSG1.ptr().cast::<u8>().write(0);
    }
    if !m2.is_null() {
        ascii_to_gxt_char(m2, LOADING_GXT_MSG2.ptr().cast());
    } else {
        LOADING_GXT_MSG2.ptr().cast::<u8>().write(0);
    }
}

/// `CLoadingScreen::GetClockTime(bool isRealTime)` (0x590280, cdecl, ST0).
#[hook(addr = 0x590280, conv = cdecl, name = "CLoadingScreen::GetClockTime", expect = "51 E8 8A 91 08 00")]
pub fn get_clock_time(real: u8) -> X87 {
    let t = clock_real();
    if real != 0 { t } else { t.fsub_m(PAUSE_TIME.get()) }
}

/// `CLoadingScreen::Init(bool, bool isReloadDisabled)` (0x5902B0, cdecl).
#[hook(addr = 0x5902B0, conv = cdecl, name = "CLoadingScreen::Init", expect = "51 A0 18 B3 BA 00")]
pub fn init(_legal: u8, reload_disabled: u8) {
    if ACTIVE.get() != 0 {
        return;
    }
    if reload_disabled == 0 {
        load_splashes_call(0, 0);
    }
    CURR_DISPLAYED_SPLASH.set(-1);
    let t = clock_real();
    ACTIVE.set(1);
    TIME_SINCE_LAST_SCREEN.set(t.to_f32());
}

/// `CLoadingScreen::Pause()` (0x590310): a lone `ret`. Not hookable.
pub fn pause() {}

/// `CLoadingScreen::IsActive()` (0x744DB5): a lone `ret` in 1.0 US (EAX unchanged). Not hookable.
pub fn is_active() {}

/// `CLoadingScreen::Continue()` (0x590320, cdecl).
#[hook(addr = 0x590320, conv = cdecl, name = "CLoadingScreen::Continue", expect = "51 8A 0D 18 B3 BA 00")]
pub fn continue_() {
    if ACTIVE.get() == 0 {
        return;
    }
    WANT_TO_PAUSE.set(0);
    if PAUSED.get() == 0 {
        return;
    }
    PAUSED.set(0);
    let v = clock_real().fsub_m(CLOCK_TIME_ON_PAUSE.get()).fadd_m(PAUSE_TIME.get());
    PAUSE_TIME.set(v.to_f32());
}

/// `CLoadingScreen::RenderLoadingBar()` (0x590370, cdecl).
#[hook(addr = 0x590370, conv = cdecl, name = "CLoadingScreen::RenderLoadingBar", expect = "83 EC 10 56 57")]
pub fn render_loading_bar() {
    let mut tmp = [0u8; 4];
    let t = GPtr::from_raw(tmp.as_mut_ptr());
    let mut color = [0u8; 4];
    color[0] = hud_get_rgb(image(HUD_COLOUR), t, 4).read();
    color[1] = hud_get_rgb(image(HUD_COLOUR), t, 4).byte_add(1).read();
    color[2] = hud_get_rgb(image(HUD_COLOUR), t, 4).byte_add(2).read();
    color[3] = 0xFF;
    let w = X87::from_i32(RS_MAXIMUM_WIDTH.get()).fmul_m(f32::from_bits(0x3ACC_CCCD)); // 1/640
    let x = X87::from_f32(50.0).fmul(w).to_f32();
    let h = X87::from_i32(RS_MAXIMUM_HEIGHT.get());
    let sy = X87::from_f32(f32::from_bits(0x3B12_4925)).fmul(h); // 1/448
    let sy_f = sy.to_f32();
    let y = h.fsub(sy.fmul_m(40.0)).to_f32();
    let width = w.fmul_m(180.0).ftol() as u32;
    let height = X87::from_f32(sy_f).fmul_m(10.0).ftol() as u32;
    if LEGAL_SCREEN.get() != 0 {
        return;
    }
    let p = X87::from_f32(PERCENT_LOADED.get());
    if !p.fcom(X87::zero()).test_any(0x41) {
        // percent > 0
    } else {
        return;
    }
    if p.fcom(X87::from_f32(100.0)).test_parity(5) {
        return;
    }
    let mut add = [0u8; 4];
    let addc = rgba_ctor(GPtr::from_raw(add.as_mut_ptr()), 0, 0, 0, 0).cast::<u32>().read();
    sprite_draw_bar_chart(
        x,
        y,
        width,
        height,
        PERCENT_LOADED.get(),
        0,
        0,
        1,
        u32::from_le_bytes(color),
        addc,
    );
    if X87::from_f32(TIME_BAR_APPEARED.get()).fcom(X87::zero()).test_parity(0x44) {
        return;
    }
    TIME_BAR_APPEARED.set(clock_real().fsub_m(PAUSE_TIME.get()).to_f32());
}

/// The body of DisplayPCScreen (0x590570), inlined into the fade loops of the original.
fn pc_screen_body() {
    if rw_camera_begin_update(SCENE_CAMERA.get()).is_null() {
        return;
    }
    defined_state_2d();
    rw_render_state_set(0xC, 1);
    render_splash_call();
    let curr = CURR_DISPLAYED_SPLASH.get();
    if curr > 0 && (FADING.get() == 0 || curr != 1) {
        render_loading_bar_call();
    }
    rw_camera_end_update(SCENE_CAMERA.get());
    rs_camera_show_raster(SCENE_CAMERA.get());
}

/// `CLoadingScreen::DisplayNextSplash()` (0x5904D0, cdecl).
#[hook(addr = 0x5904D0, conv = cdecl, name = "CLoadingScreen::DisplayNextSplash", expect = "83 3D 3C 09 8D 00 06")]
pub fn display_next_splash() {
    if CURR_DISPLAYED_SPLASH.get() == 6 || FADING.get() != 0 {
        return;
    }
    FADE_ALPHA.set(0xFF);
    if !rw_camera_begin_update(SCENE_CAMERA.get()).is_null() {
        defined_state_2d();
        render_splash_call();
        render_loading_bar_call();
        rw_camera_end_update(SCENE_CAMERA.get());
        rs_camera_show_raster(SCENE_CAMERA.get());
    }
    CURR_DISPLAYED_SPLASH.set(CURR_DISPLAYED_SPLASH.get().wrapping_add(1));
}

/// `CLoadingScreen::StartFading()` (0x590530, cdecl).
#[hook(addr = 0x590530, conv = cdecl, name = "CLoadingScreen::StartFading", expect = "51 C6 05 1C B3 BA 00 01")]
pub fn start_fading() {
    FADING.set(1);
    FADE_ALPHA.set(0);
    START_FADE_TIME.set(clock_real().fsub_m(PAUSE_TIME.get()).to_f32());
}

/// `CLoadingScreen::DisplayPCScreen()` (0x590570, cdecl).
#[hook(addr = 0x590570, conv = cdecl, name = "CLoadingScreen::DisplayPCScreen", expect = "A1 3C 70 C1 00")]
pub fn display_pc_screen() {
    pc_screen_body();
}

/// `CLoadingScreen::Update()` (0x5905E0, cdecl).
#[hook(addr = 0x5905E0, conv = cdecl, name = "CLoadingScreen::Update", expect = "51 D9 05 38 B3 BA 00")]
pub fn update() {
    let tba = X87::from_f32(TIME_BAR_APPEARED.get());
    if !tba.fcom(X87::zero()).test_any(0x41) {
        let a = clock_real()
            .fsub_m(PAUSE_TIME.get())
            .fsub_m(TIME_BAR_APPEARED.get())
            .fmul_m(100.0);
        let b = tba
            .fsub_m(TIME_STARTED_LOADING.get())
            .fsubr_m(36.0);
        let p = a.fdiv(b);
        PERCENT_LOADED.set(p.to_f32());
        if !p.fcom(X87::from_f32(100.0)).test_any(0x41) {
            PERCENT_LOADED.set(100.0);
        }
    }
    let mut go_fade = false;
    if READY_TO_DELETE.get() == 0 {
        if LEGAL_SCREEN.get() != 0 {
            let d = clock_real().fsub_m(PAUSE_TIME.get()).fsub_m(TIME_STARTED_LOADING.get());
            if !d.fcom(X87::from_f32(5.5)).test_any(0x41) {
                display_next_splash_call();
            }
        } else {
            let s = X87::from_i32(CURR_DISPLAYED_SPLASH.get()).fmul_m(f32::from_bits(0x4185_5555));
            if !s.fcom(X87::from_f32(PERCENT_LOADED.get())).test_parity(5) {
                display_next_splash_call();
            }
        }
        if SIGNAL_DELETE.get() != 0 {
            if FADING.get() != 0 {
                go_fade = true;
            } else if !X87::from_f32(PERCENT_LOADED.get()).fcom(X87::from_f32(100.0)).c0() {
                READY_TO_DELETE.set(1);
                FADE_OUT_TO_BLACK.set(1);
                start_fading_call();
                go_fade = FADING.get() != 0;
            }
        } else {
            go_fade = FADING.get() != 0;
        }
    } else {
        go_fade = FADING.get() != 0;
    }
    if go_fade {
        let elapsed = clock_real().fsub_m(PAUSE_TIME.get()).fsub_m(START_FADE_TIME.get());
        let curr = CURR_DISPLAYED_SPLASH.get();
        let dur = if curr >= 0 && (LEGAL_SCREEN.get() != 0 || FADE_IN_FROM_BLACK.get() != 0) { 2.0f32 } else { f32::from_bits(0x3F19_999A) };
        if !elapsed.fcom(X87::from_f32(dur)).test_any(0x41) {
            let legal = LEGAL_SCREEN.get();
            FADING.set(0);
            FADE_ALPHA.set(0xFF);
            if legal != 0 && FADE_OUT_TO_BLACK.get() != 0 {
                FADE_OUT_TO_BLACK.set(0);
                FADE_IN_FROM_BLACK.set(1);
                LEGAL_SCREEN.set(0);
                display_next_splash_call();
            } else {
                FADE_IN_FROM_BLACK.set(0);
                FADE_OUT_TO_BLACK.set(0);
            }
        } else {
            let a = elapsed.fdiv(X87::from_f32(dur)).fmul_m(255.0).ftol();
            FADE_ALPHA.set(a as u8);
        }
    } else {
        FADE_ALPHA.set(0xFF);
    }
    if LEGAL_SCREEN.get() != 0 {
        return;
    }
    let al = FADE_ALPHA.get();
    let amp = if FADE_IN_FROM_BLACK.get() != 0 {
        X87::from_i32(al as i32).fmul_m(f32::from_bits(0x3B80_8081)).to_f32()
    } else if FADE_OUT_TO_BLACK.get() != 0 {
        X87::from_i32(al as i32).fsubr_m(255.0).fmul_m(f32::from_bits(0x3B80_8081)).to_f32()
    } else {
        1.0
    };
    service_loading_tune(image(AUDIO_ENGINE), amp);
}

/// `(uint8)_ftol((float)(uint32)i * 5.0f)`
fn alpha_of(i: u32) -> u8 {
    u32_x87(i).fmul_m(5.0).ftol() as u8
}

/// `CLoadingScreen::DoPCTitleFadeIn()` (0x590860, cdecl; entry through the chunk 0x406465).
#[hook(addr = 0x590860, conv = cdecl, name = "CLoadingScreen::DoPCTitleFadeIn", expect = "51 56 90 90 E9 FC 5B E7 FF")]
pub fn do_pc_title_fade_in() {
    FADE_IN_FROM_BLACK.set(1);
    CURR_DISPLAYED_SPLASH.set(0);
    FADING.set(1);
    for i in 0..50u32 {
        FADE_ALPHA.set(alpha_of(i));
        pc_screen_body();
    }
    FADE_ALPHA.set(0xFF);
    pc_screen_body();
    FADING.set(0);
}

/// `CLoadingScreen::DoPCTitleFadeOut()` (0x590990, cdecl).
#[hook(addr = 0x590990, conv = cdecl, name = "CLoadingScreen::DoPCTitleFadeOut", expect = "51 56 C6 05 1E B3 BA 00 01")]
pub fn do_pc_title_fade_out() {
    FADE_IN_FROM_BLACK.set(1);
    CURR_DISPLAYED_SPLASH.set(0);
    FADING.set(1);
    for _ in 0..200 {
        FADE_ALPHA.set(0xFF);
        pc_screen_body();
    }
    for i in (1..=50u32).rev() {
        FADE_ALPHA.set(alpha_of(i));
        pc_screen_body();
    }
    FADE_IN_FROM_BLACK.set(1);
}

/// `CLoadingScreen::DoPCScreenChange(bool isLastOne)` (0x590AC0, cdecl).
#[hook(addr = 0x590AC0, conv = cdecl, name = "CLoadingScreen::DoPCScreenChange", expect = "51 53 56 57 8B 7C 24 14")]
pub fn do_pc_screen_change(last: u8) {
    let last = last as u32 != 0;
    FADING.set(1);
    if last {
        FADE_OUT_TO_BLACK.set(1);
    } else {
        CURR_DISPLAYED_SPLASH.set(CURR_DISPLAYED_SPLASH.get().wrapping_add(1));
    }
    for _ in 0..20 {
        FADE_ALPHA.set(0);
        pc_screen_body();
    }
    for i in 0..50u32 {
        let a = alpha_of(i);
        FADE_ALPHA.set(a);
        if last || FADE_IN_FROM_BLACK.get() != 0 {
            let amp = if FADE_OUT_TO_BLACK.get() != 0 {
                X87::from_i32(a as i32).fsubr_m(255.0).fmul_m(f32::from_bits(0x3B80_8081)).to_f32()
            } else {
                1.0
            };
            service_loading_tune(image(AUDIO_ENGINE), amp);
        }
        pc_screen_body();
    }
    FADE_ALPHA.set(0xFF);
    pc_screen_body();
    FADE_IN_FROM_BLACK.set(0);
    FADING.set(0);
    if last {
        release_splashes();
    }
}

/// `CLoadingScreen::NewChunkLoaded()` (0x590D00, cdecl).
#[hook(addr = 0x590D00, conv = cdecl, name = "CLoadingScreen::NewChunkLoaded", expect = "51 A0 18 B3 BA 00")]
pub fn new_chunk_loaded() {
    if ACTIVE.get() == 0 {
        return;
    }
    let n = NUM_CHUNKS_LOADED.get().wrapping_add(1);
    let appeared = CHUNK_BAR_APPEARED.get();
    NUM_CHUNKS_LOADED.set(n);
    if appeared != -1 {
        let p = X87::from_i32(n.wrapping_sub(appeared))
            .fmul_m(100.0)
            .fdiv(X87::from_i32(140i32.wrapping_sub(appeared)));
        PERCENT_LOADED.set(p.to_f32());
    }
    let now_x = clock_real();
    let now = now_x.to_f32();
    let delta = now_x.fsub_m(TIME_SINCE_LAST_SCREEN.get());
    if NUM_CHUNKS_LOADED.get() == 140 {
        do_pc_screen_change_call(1);
        return;
    }
    let limit = if CURR_DISPLAYED_SPLASH.get() == 0 { 5.5f32 } else { 5.0 };
    if delta.fcom(X87::from_f32(limit)).c0() {
        display_pc_screen_call();
        return;
    }
    do_pc_screen_change_call(0);
    TIME_SINCE_LAST_SCREEN.set(now);
    if CHUNK_BAR_APPEARED.get() == -1 {
        CHUNK_BAR_APPEARED.set(NUM_CHUNKS_LOADED.get());
    }
}

/// `__init_g_bab358` (0x84F2B0, CRT initialiser): `0.277778f / 50.0f` (f32 0x3BB60B6A).
#[hook(addr = 0x84F2B0, conv = cdecl, name = "__init_g_bab358", expect = "D9 05 30 86 85 00")]
pub fn init_g_bab358() {
    G_BAB358.set(g_bab358_value());
}

/// The value `__init_g_bab358` stores: `fld [0x858630] (0.277778f); fdiv [0x866CC0] (50.0f)`.
pub fn g_bab358_value() -> f32 {
    X87::from_f32(f32::from_bits(0x3E8E_38EB)).fdiv_m(50.0).to_f32()
}

/// `__init_CLoadingScreen__m_aSplashes` (0x84F2D0, CRT initialiser): construct the 7 sprites and
/// register the destructor with `_atexit`.
#[hook(addr = 0x84F2D0, conv = cdecl, name = "__init_CLoadingScreen__m_aSplashes", expect = "68 E0 81 72 00")]
pub fn init_splashes() {
    eh_vector_ctor(SPLASHES.ptr().cast(), 4, 7, SPRITE_CTOR, SPRITE_DTOR);
    atexit(DTOR_SPLASHES);
}

/// `__dtor_CLoadingScreen__m_aSplashes` (0x8565E0, `_atexit` callback): destroy the 7 sprites
/// (descending).
#[hook(addr = 0x8565E0, conv = cdecl, name = "__dtor_CLoadingScreen__m_aSplashes", expect = "68 E0 81 72 00")]
pub fn dtor_splashes() {
    eh_vector_dtor(SPLASHES.ptr().cast(), 4, 7, SPRITE_DTOR);
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[sa_core::abi::HookReg] = &[
    HOOK_LOADING_SCREEN,
    HOOK_SHUTDOWN,
    HOOK_RENDER_SPLASH,
    HOOK_LOAD_SPLASHES,
    HOOK_DISPLAY_MESSAGE,
    HOOK_SET_LOADING_BAR_MSG,
    HOOK_GET_CLOCK_TIME,
    HOOK_INIT,
    HOOK_CONTINUE_,
    HOOK_RENDER_LOADING_BAR,
    HOOK_DISPLAY_NEXT_SPLASH,
    HOOK_START_FADING,
    HOOK_DISPLAY_PC_SCREEN,
    HOOK_UPDATE,
    HOOK_DO_PC_TITLE_FADE_IN,
    HOOK_DO_PC_TITLE_FADE_OUT,
    HOOK_DO_PC_SCREEN_CHANGE,
    HOOK_NEW_CHUNK_LOADED,
    HOOK_INIT_G_BAB358,
    HOOK_INIT_SPLASHES,
    HOOK_DTOR_SPLASHES,
];

#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    #[test]
    fn g_bab358_value() {
        super::init_g_bab358();
        assert_eq!(super::G_BAB358.get().to_bits(), 0x3BB6_0B6A);
    }
}
