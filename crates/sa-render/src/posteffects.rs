//! PORT-RENDER: `CPostEffects` (static class). Ported from the 1.0 US bytes; gta-reversed
//! `PostEffects.cpp` and `docs/re/specs/RENDER/CPostEffects*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, jne, k, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// `CPostEffects::m_bInCutscene`
    pub static IN_CUTSCENE @ 0xC402B7: u8 = 0;
    /// `CPostEffects::m_bHeatHazeFX`
    pub static HEAT_HAZE_FX @ 0xC402BA: u8 = 0;
    /// `CPostEffects::m_bDarknessFilter`
    pub static DARKNESS_FILTER @ 0xC402C4: u8 = 0;
    /// `CPostEffects::m_bCCTV`
    pub static CCTV @ 0xC402C5: u8 = 0;
    /// `CPostEffects::m_bRainEnable`
    pub static RAIN_ENABLE @ 0xC402D1: u8 = 0;
    /// `CPostEffects::pRasterFrontBuffer`
    pub static RASTER_FRONT_BUFFER @ 0xC402D8: u32 = 0;
    /// The ten render states `ImmediateModeRenderStatesStore` saves (src / dest blend, fog, cull, z-test,
    /// z-write, shade, alpha blend, texture filter, texture address: the `ReStore` order)
    pub static SAVED_RENDER_STATES @ 0xC40280: [u32; 10] = [0; 10];
    /// `CPostEffects::m_pGrainRaster`
    pub static GRAIN_RASTER @ 0xC402B0: u32 = 0;
    /// `CPostEffects::m_bSpeedFX` (initially true)
    pub static SPEED_FX @ 0x8D37D5: u8 = 1;
}

/// `CPostEffects::m_bNightVision` (0xC402B8; also read by sa-ped / the renderer through its address).
pub const NIGHT_VISION: usize = 0xC402B8;
/// `CPostEffects::m_bInfraredVision` (0xC402B9).
pub const INFRARED_VISION: usize = 0xC402B9;
/// `CWeather::Rain` (0xC81324, owned by the weather port).
const WEATHER_RAIN: usize = 0xC81324;
/// 0.0 (0x858B50)
const K_ZERO: u32 = 0;

orig! {
    /// `CPostEffects::SetupBackBufferVertex()` (0x7043D0)
    pub fn setup_back_buffer_vertex() = cdecl 0x7043D0 => unported;
    /// `RwRenderStateSet` (`RwEngineInstance` slot 0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwRasterCreate(int32 w, int32 h, int32 depth, int32 flags)` (0x7FB230)
    pub fn rw_raster_create(w: i32, h: i32, depth: i32, flags: i32) -> GPtr<u8> = cdecl 0x7FB230 => unported;
    /// `RwRasterLock(RwRaster*, uint8 level, int32 mode)` (0x7FB2D0)
    pub fn rw_raster_lock(r: GPtr<u8>, level: u32, mode: i32) -> GPtr<u8> = cdecl 0x7FB2D0 => unported;
    /// `RwRasterUnlock(RwRaster*)` (0x7FAEC0)
    pub fn rw_raster_unlock(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7FAEC0 => unported;
}

/// `CPostEffects::ScriptResetForEffects()` (0x7010F0): night / infrared vision, heat haze, darkness
/// filter and CCTV off, speed FX on.
#[hook(addr = 0x7010F0, conv = cdecl, name = "CPostEffects::ScriptResetForEffects", expect = "32 C0 A2 B8 02 C4 00")]
pub fn script_reset_for_effects() {
    g::<u8>(NIGHT_VISION).write(0);
    g::<u8>(INFRARED_VISION).write(0);
    HEAT_HAZE_FX.set(0);
    DARKNESS_FILTER.set(0);
    CCTV.set(0);
    SPEED_FX.set(1);
}

/// `CPostEffects::IsVisionFXActive()` (0x7034F0): false in a cut-scene, else night || infrared vision.
#[hook(addr = 0x7034F0, conv = cdecl, name = "CPostEffects::IsVisionFXActive", expect = "A0 B7 02 C4 00")]
pub fn is_vision_fx_active() -> u8 {
    if IN_CUTSCENE.get() != 0 {
        return 0;
    }
    u8::from(g::<u8>(NIGHT_VISION).read() != 0 || g::<u8>(INFRARED_VISION).read() != 0)
}

/// `CPostEffects::Update()` (0x7046A0): rain on unless `CWeather::Rain <= 0.0` or unordered
/// (`fcomp` + `test ah, 0x41`), then `SetupBackBufferVertex()` (tail jump) while there is no front
/// buffer raster yet.
#[hook(addr = 0x7046A0, conv = cdecl, name = "CPostEffects::Update", expect = "D9 05 24 13 C8 00")]
pub fn update() {
    let rain = x(g::<f32>(WEATHER_RAIN).read());
    RAIN_ENABLE.set(1);
    if jne(rain, x(k(K_ZERO)), 0x41) {
        RAIN_ENABLE.set(0);
    }
    if RASTER_FRONT_BUFFER.get() == 0 {
        setup_back_buffer_vertex();
    }
}

/// `CPostEffects::ImmediateModeRenderStatesSet()` (0x700D70): ten `RwRenderStateSet` calls (src / dest
/// blend, fog off, cull none, z-test / z-write off, alpha blending on, vertex alpha on, texture filter,
/// texture address).
#[hook(addr = 0x700D70, conv = cdecl, name = "CPostEffects::ImmediateModeRenderStatesSet", expect = "A1 24 7B C9 00")]
pub fn immediate_mode_render_states_set() {
    for (st, v) in [
        (0xA, 5),
        (0xB, 6),
        (0xE, 0),
        (0x14, 1),
        (6, 0),
        (8, 0),
        (7, 2),
        (0xC, 1),
        (2, 3),
        (9, 1),
    ] {
        rw_render_state_set(st, v);
    }
}

/// `CPostEffects::Initialise()` (0x704630): `SetupBackBufferVertex()`, then the 256x256x32 grain
/// raster (flags 0x504); when created, every pixel's four bytes = the low byte of one `rand()` (65536
/// calls at 0x704670), locked level 0 / mode 1.
#[hook(addr = 0x704630, conv = cdecl, name = "CPostEffects::Initialise", expect = "E8 9B FD FF FF")]
pub fn initialise() {
    setup_back_buffer_vertex();
    let r = rw_raster_create(0x100, 0x100, 0x20, 0x504);
    GRAIN_RASTER.set(r.addr() as u32);
    if r.is_null() {
        return;
    }
    let p = rw_raster_lock(r, 0, 1);
    for i in 0..0x10000isize {
        let v = sa_core::orig::at_site(0x704675, sa_core::crt::rand) as u8;
        wr(p, i * 4 + 3, v);
        wr(p, i * 4 + 2, v);
        wr(p, i * 4 + 1, v);
        wr(p, i * 4, v);
    }
    rw_raster_unlock(GPtr::from_addr(GRAIN_RASTER.get() as usize));
}

/// `CPostEffects::ImmediateModeRenderStatesReStore()` (0x700E00): the ten saved states (0xC40280..)
/// set back.
#[hook(addr = 0x700E00, conv = cdecl, name = "CPostEffects::ImmediateModeRenderStatesReStore", expect = "A1 80 02 C4 00")]
pub fn immediate_mode_render_states_restore() {
    for (i, st) in [0xA, 0xB, 0xE, 0x14, 6, 8, 7, 0xC, 2, 9].into_iter().enumerate() {
        rw_render_state_set(st, SAVED_RENDER_STATES.get()[i]);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_IMMEDIATE_MODE_RENDER_STATES_RESTORE,
    HOOK_IMMEDIATE_MODE_RENDER_STATES_SET,
    HOOK_INITIALISE,
    HOOK_SCRIPT_RESET_FOR_EFFECTS,
    HOOK_IS_VISION_FX_ACTIVE,
    HOOK_UPDATE,
];
