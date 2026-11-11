//! PORT-RENDER: `CPostEffects` (static class). Ported from the 1.0 US bytes; gta-reversed
//! `PostEffects.cpp` and `docs/re/specs/RENDER/CPostEffects*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, jne, k, rd, wr, x, xi};

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
    /// The back buffer quad `SetupBackBufferVertex` fills (4 RwIm2DVertex of 0x1C bytes, as dwords)
    pub static BACK_BUFFER_VERTICES @ 0xC400D8: [u32; 28] = [0; 28];
    /// `CPostEffects::ms_imf` (0xC40150..0xC40248, as dwords)
    pub static IMF @ 0xC40150: [u32; 62] = [0; 62];
    /// `CPostEffects::m_HeatHazeFXType`
    pub static HEAT_HAZE_FX_TYPE @ 0xC402BC: i32 = 0;
    /// `CPostEffects::m_bHeatHazeMaskModeTest`
    pub static HEAT_HAZE_MASK_MODE_TEST @ 0xC402C0: u32 = 0;
    /// `m_HeatHazeFXScanSizeX`, `ScanSizeY`, `RenderSizeX`, `RenderSizeY`
    pub static HEAT_HAZE_SIZES @ 0xC40304: [i32; 4] = [0; 4];
    /// `m_HeatHazeFXTypeLast` (-1), `m_HeatHazeFXIntensity` (0x96), `m_HeatHazeFXSpeedMin` (6),
    /// `m_HeatHazeFXSpeedMax` (10)
    pub static HEAT_HAZE_PARAMS @ 0x8D50E4: [i32; 4] = [-1, 0x96, 6, 0xA];
    /// `hpS[180]` (heat haze speeds)
    pub static HP_S @ 0xC3F868: [i32; 180] = [0; 180];
    /// `hpY[180]`
    pub static HP_Y @ 0xC3FB38: [i32; 180] = [0; 180];
    /// `hpX[180]`
    pub static HP_X @ 0xC3FE08: [i32; 180] = [0; 180];
}

/// `Scene.camera` (RwCamera*: +0x60 frame buffer raster, +0x80 near plane).
const SCENE_CAMERA: usize = 0xC1703C;
/// `RwEngineInstance` (+0x18: the Im2D near screen z).
const RW_ENGINE_INSTANCE: usize = 0xC97B24;
/// `RsGlobal.maximumWidth` / `maximumHeight`.
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// "Error subrastering\n" (0x872630)
const STR_ERROR_SUBRASTERING: usize = 0x872630;

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
    pub fn setup_back_buffer_vertex_call() = cdecl 0x7043D0 => setup_back_buffer_vertex;
    /// `CPostEffects::ImmediateModeFilterStuffInitialize()` (0x703CC0)
    pub fn immediate_mode_filter_stuff_initialize_call() = cdecl 0x703CC0 => immediate_mode_filter_stuff_initialize;
    /// `CPostEffects::HeatHazeFXInit()` (0x701450)
    pub fn heat_haze_fx_init_call() = cdecl 0x701450 => heat_haze_fx_init;
    /// `RwRasterDestroy(RwRaster*)` (0x7FB020)
    pub fn rw_raster_destroy(r: GPtr<u8>) -> i32 = cdecl 0x7FB020 => unported;
    /// CRT `printf` (one argument here)
    pub fn printf(fmt: GPtr<u8>) -> i32 = cdecl 0x821982 => unported;
    /// CRT `pow(double, double)` (0x8220F0)
    pub fn crt_pow(a: f64, b: f64) -> f64 = cdecl 0x8220F0 => crt_pow_host;
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
        setup_back_buffer_vertex_call();
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
    setup_back_buffer_vertex_call();
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

/// Phase B/C path of [`crt_pow`]: only `pow(2.0, integer)` is used here, which is exact
/// (overflow → +inf, underflow → +0).
pub fn crt_pow_host(a: f64, b: f64) -> f64 {
    assert!(a == 2.0 && b == (b as i64) as f64, "crt_pow_host: only pow(2, integer)");
    let e = b as i64;
    if e > 1023 {
        f64::INFINITY
    } else if e >= -1022 {
        f64::from_bits(((e + 1023) as u64) << 52)
    } else if e >= -1074 {
        f64::from_bits(1u64 << (e + 1074))
    } else {
        0.0
    }
}

/// `ftol(pow(2.0, (double)n))`: `pow`'s ST0 result goes straight to `_ftol`.
fn pow2_ftol(n: i32) -> i32 {
    ftol32(X87::from_f64_exact(crt_pow(2.0, f64::from(n))))
}

/// `ftol(ln(v) / div) + 1` (`fldln2; fild v; fyl2x; fdiv div; call _ftol; inc eax`).
fn log2_plus_one(v: i32, div: X87) -> i32 {
    ftol32(X87::ln2().fyl2x(xi(v)).fdiv(div)).wrapping_add(1)
}

fn camera() -> GPtr<u8> {
    g::<GPtr<u8>>(SCENE_CAMERA).read()
}

/// `1.0 / Scene.camera->nearPlane` (`fld 1.0; fdiv dword [camera+0x80]`).
fn recip_near() -> X87 {
    x(k(0x3F80_0000)).fdiv_m(rd::<f32>(camera(), 0x80))
}

/// `RwIm2DGetNearScreenZ()` as the dword the original copies with `mov`.
fn near_screen_z_bits() -> u32 {
    rd::<u32>(g::<GPtr<u8>>(RW_ENGINE_INSTANCE).read(), 0x18)
}

/// `CPostEffects::SetupBackBufferVertex()` (0x7043D0).
///
/// The camera frame buffer's width / height are rounded up to powers of two the original's way:
/// `n = ftol(ln(size) / ln2d) + 1`, `size2 = ftol(pow(2.0, n))`, where `ln2d` is `fldln2 * log2(2.0)`
/// stored as a **double** (the 64-bit FYL2X result rounded to 53 bits) and the division runs under
/// PC = 24. The front buffer raster (`pRasterFrontBuffer`) is kept when it already has that size,
/// otherwise destroyed (when set) and re-created (`RwRasterCreate(w2, h2, frameBuffer->depth, 5)`;
/// failure: `printf("Error subrastering\n")`, `RwRasterDestroy(NULL)`, raster = NULL). Then the four
/// back-buffer vertices (x, y, z = Im2D near screen z, rhw = 1 / near plane, u / v = `0.5 / size` and
/// `(size + 0.5) / size`; colours untouched), in the original's store order. With a raster:
/// `ImmediateModeFilterStuffInitialize()`, `HeatHazeFXInit()`.
#[hook(addr = 0x7043D0, conv = cdecl, name = "CPostEffects::SetupBackBufferVertex", expect = "55 8B EC 83 E4 F8")]
pub fn setup_back_buffer_vertex() {
    let fb = rd::<GPtr<u8>>(camera(), 0x60);
    let w0 = rd::<i32>(fb, 0xC);
    let h0 = rd::<i32>(fb, 0x10);
    let ln2d = X87::from_f64_exact(X87::ln2().fyl2x(X87::from_f64_exact(2.0)).to_f64());
    let n = log2_plus_one(w0, ln2d);
    let w = pow2_ftol(n);
    let n = log2_plus_one(h0, ln2d);
    let h = pow2_ftol(n);
    let mut r = GPtr::<u8>::from_addr(RASTER_FRONT_BUFFER.get() as usize);
    let keep = !r.is_null() && w == rd::<i32>(r, 0xC) && h == rd::<i32>(r, 0x10);
    if !keep {
        if !r.is_null() {
            rw_raster_destroy(r);
            RASTER_FRONT_BUFFER.set(0);
        }
        let depth = rd::<i32>(rd::<GPtr<u8>>(camera(), 0x60), 0x14);
        r = rw_raster_create(w, h, depth, 5);
        RASTER_FRONT_BUFFER.set(r.addr() as u32);
        if r.is_null() {
            printf(GPtr::from_addr(STR_ERROR_SUBRASTERING));
            rw_raster_destroy(GPtr::from_addr(RASTER_FRONT_BUFFER.get() as usize));
            r = GPtr::NULL;
            RASTER_FRONT_BUFFER.set(0);
        }
    }
    let half = k(0x3F00_0000);
    let v = BACK_BUFFER_VERTICES.ptr().cast::<u8>();
    let st = |o: isize, val: X87| wr::<f32>(v, o, f32_of(val));
    let wx = xi(w);
    let wf = f32_of(wx);
    wr::<u32>(v, 0, 0);
    wr::<u32>(v, 4, 0);
    let hx = xi(h);
    wr::<u32>(v, 8, near_screen_z_bits());
    let hf = f32_of(hx);
    let a = x(half).fadd(hx).fdiv_m(hf);
    let b = x(half).fdiv_m(hf);
    let c = wx.fadd_m(half).fdiv_m(wf);
    let d = x(half).fdiv_m(wf);
    let rn = recip_near();
    wr::<u32>(v, 0x1C, 0);
    st(0xC, rn);
    st(0x14, d);
    st(0x18, b);
    st(0x20, hx);
    wr::<u32>(v, 0x24, near_screen_z_bits());
    st(0x28, recip_near());
    st(0x30, d);
    st(0x34, a);
    st(0x38, wx);
    st(0x3C, hx);
    wr::<u32>(v, 0x40, near_screen_z_bits());
    let rn = recip_near();
    wr::<u32>(v, 0x58, 0);
    st(0x44, rn);
    st(0x4C, c);
    st(0x50, a);
    st(0x54, wx);
    wr::<u32>(v, 0x5C, near_screen_z_bits());
    let rn = recip_near();
    wr::<f32>(v, 0x68, f32_of(c));
    wr::<f32>(v, 0x6C, f32_of(b));
    st(0x60, rn);
    if !r.is_null() {
        immediate_mode_filter_stuff_initialize_call();
        heat_haze_fx_init_call();
    }
}

/// `CPostEffects::ImmediateModeFilterStuffInitialize()` (0x703CC0): `ms_imf` from the Im2D near
/// screen z, `1 / near plane`, the front buffer raster and its size (two full-screen quads at
/// 0xC40174 and 0xC401C0 with extents `2 * size`, colour 0xFF00C800 on the second), then the u / v
/// scales `RsGlobal.maximumWidth / w2` and `maximumHeight / h2` (`fidiv`), where `w2` / `h2` are the
/// camera frame buffer's size rounded up to powers of two as in `SetupBackBufferVertex`, except that
/// the divisor is the **unrounded** 64-bit FYL2X `ln 2` kept on the x87 stack. Stores in the
/// original's order.
#[hook(addr = 0x703CC0, conv = cdecl, name = "CPostEffects::ImmediateModeFilterStuffInitialize", expect = "83 EC 08 A1 24 7B C9 00")]
pub fn immediate_mode_filter_stuff_initialize() {
    let m = IMF.ptr().cast::<u8>();
    let o = |a: usize| (a - 0xC40150) as isize;
    let z = x(f32::from_bits(near_screen_z_bits()));
    let zf = f32_of(z);
    wr::<f32>(m, o(0xC40150), zf);
    let raster = RASTER_FRONT_BUFFER.get();
    // `fld dword [camera+0x80]; fdivr dword [1.0]`
    let rnf = f32_of(x(rd::<f32>(camera(), 0x80)).fdivr_m(k(0x3F80_0000)));
    wr::<u32>(m, o(0xC40158), raster);
    wr::<u32>(m, o(0xC401C8), 0);
    wr::<u32>(m, o(0xC401CC), 0x4000_0000);
    wr::<u32>(m, o(0xC401D0), 0);
    wr::<u32>(m, o(0xC401D4), 0x4000_0000);
    wr::<f32>(m, o(0xC40154), rnf);
    let rp = GPtr::<u8>::from_addr(raster as usize);
    wr::<i32>(m, o(0xC4015C), rd::<i32>(rp, 0xC));
    let rh = rd::<i32>(rp, 0x10);
    wr::<f32>(m, o(0xC4017C), zf);
    wr::<i32>(m, o(0xC40160), rh);
    wr::<f32>(m, o(0xC40180), rnf);
    let col: u32 = 0xFF00_C800;
    let w2 = xi(rd::<i32>(m, o(0xC4015C))).fdouble();
    for a in [0xC40174, 0xC40178, 0xC40188, 0xC4018C, 0xC40194] {
        wr::<u32>(m, o(a), 0);
    }
    wr::<f32>(m, o(0xC40190), f32_of(w2));
    wr::<u32>(m, o(0xC401A4), 0x4000_0000);
    wr::<f32>(m, o(0xC40198), zf);
    wr::<u32>(m, o(0xC401A8), 0);
    wr::<u32>(m, o(0xC401AC), 0);
    wr::<f32>(m, o(0xC4019C), rnf);
    wr::<u32>(m, o(0xC401C0), 0);
    let h2 = xi(rd::<i32>(m, o(0xC40160))).fdouble();
    wr::<u32>(m, o(0xC401C4), 0x4000_0000);
    wr::<u32>(m, o(0xC401D8), 0);
    wr::<u32>(m, o(0xC401DC), 0);
    wr::<u32>(m, o(0xC401E8), col);
    wr::<u32>(m, o(0xC401F4), 0);
    wr::<f32>(m, o(0xC401B0), f32_of(h2));
    wr::<u32>(m, o(0xC401F8), 0);
    wr::<f32>(m, o(0xC401B4), zf);
    wr::<u32>(m, o(0xC40204), col);
    wr::<u32>(m, o(0xC40210), 0);
    wr::<f32>(m, o(0xC401B8), rnf);
    wr::<u32>(m, o(0xC40214), 0);
    wr::<u32>(m, o(0xC40220), col);
    wr::<f32>(m, o(0xC401E0), zf);
    wr::<u32>(m, o(0xC4022C), 0);
    wr::<u32>(m, o(0xC40230), 0);
    wr::<f32>(m, o(0xC401E4), rnf);
    wr::<u32>(m, o(0xC4023C), col);
    wr::<u32>(m, o(0xC401EC), 0);
    wr::<f32>(m, o(0xC401FC), zf);
    wr::<u32>(m, o(0xC401F0), 0);
    wr::<u32>(m, o(0xC40208), 0x3F80_0000);
    wr::<f32>(m, o(0xC40200), rnf);
    wr::<u32>(m, o(0xC4020C), 0);
    wr::<u32>(m, o(0xC40224), 0);
    wr::<f32>(m, o(0xC40218), zf);
    wr::<u32>(m, o(0xC40228), 0x3F80_0000);
    wr::<f32>(m, o(0xC4021C), rnf);
    wr::<f32>(m, o(0xC40234), zf);
    wr::<f32>(m, o(0xC40238), rnf);
    wr::<u32>(m, o(0xC40240), 0x3F80_0000);
    wr::<u32>(m, o(0xC40244), 0x3F80_0000);
    let ln2w = X87::ln2().fyl2x(X87::from_f64_exact(2.0));
    let fb = rd::<GPtr<u8>>(camera(), 0x60);
    let nw = ftol32(X87::ln2().fyl2x(xi(rd::<i32>(fb, 0xC))).fdiv(ln2w));
    let nh = ftol32(X87::ln2().fyl2x(xi(rd::<i32>(fb, 0x10))).fdiv(ln2w));
    let pw = pow2_ftol(nw.wrapping_add(1));
    let ph = pow2_ftol(nh.wrapping_add(1));
    let us = xi(g::<i32>(RS_MAX_WIDTH).read()).fdiv(xi(pw));
    wr::<u32>(m, o(0xC40164), 0);
    wr::<u32>(m, o(0xC40168), 0);
    wr::<f32>(m, o(0xC4016C), f32_of(us));
    let vs = xi(g::<i32>(RS_MAX_HEIGHT).read()).fdiv(xi(ph));
    wr::<f32>(m, o(0xC40170), f32_of(vs));
}

/// `ftol((float)(rand() & 0xFFFF) * 2^-15 * range)` (`fimul` of the int range); `ret` is the return
/// address of the original's `call rand` (determinism attribution).
fn haze_random(ret: u32, range: i32) -> i32 {
    let r = sa_core::orig::at_site(ret, sa_core::crt::rand) & 0xFFFF;
    ftol32(xi(r).fmul_m(k(0x3800_0000)).fmul(xi(range)))
}

/// `CPostEffects::HeatHazeFXInit()` (0x701450), including the part the original reaches through the
/// protection chunk 0x401092 (it stores `m_HeatHazeFXTypeLast = type`; target `[0x62B4B2] ^
/// [0x15605E8]` = 0x8D50E4) and the loop at 0x7016A0 (not a separate function: the loop body, entered
/// with the caller's ESI). Returns at once when the type equals the last one. Types 0..4 set intensity,
/// speed min / max, the mask-mode flag (1 for type 4 only) and the scan / render sizes as `ftol` of
/// `RsGlobal size (fild) * 0.0015625f / 0.002232143f * k`, with the bytes' constants and operand order;
/// a type above 4 keeps them. Then 180 samples of three `rand()`s each
/// (`docs/re/specs/RENDER/CPostEffects.deviations.md`): `hpX = r * (raster width - scanX)`,
/// `hpY = r * (raster height - scanY)`, `hpS = r * (speedMax - speedMin) + speedMin`, the raster and
/// the sizes re-read every sample.
#[hook(addr = 0x701450, conv = cdecl, name = "CPostEffects::HeatHazeFXInit", expect = "A1 E4 50 8D 00")]
pub fn heat_haze_fx_init() {
    let ty = HEAT_HAZE_FX_TYPE.get();
    if ty == HEAT_HAZE_PARAMS.get()[0] {
        return;
    }
    let p = HEAT_HAZE_PARAMS.ptr().cast::<i32>();
    let sz = HEAT_HAZE_SIZES.ptr().cast::<i32>();
    let ws = || xi(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(k(0x3ACC_CCCD));
    let hs = || xi(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(k(0x3B12_4925));
    match ty as u32 {
        0 => {
            p.wrapping_index(1).write(0x50);
            HEAT_HAZE_MASK_MODE_TEST.set(0);
            p.wrapping_index(2).write(0xC);
            let w = ws();
            p.wrapping_index(3).write(0x12);
            sz.write(ftol32(x(k(0x423C_0000)).fmul(w)));
            let h = hs();
            let hf = f32_of(h);
            sz.wrapping_index(1).write(ftol32(h.fmul_m(k(0x423C_0000))));
            sz.wrapping_index(2).write(ftol32(w.fmul_m(k(0x4248_0000))));
            sz.wrapping_index(3).write(ftol32(x(hf).fmul_m(k(0x4248_0000))));
        }
        1 => {
            p.wrapping_index(1).write(0x20);
            HEAT_HAZE_MASK_MODE_TEST.set(0);
            p.wrapping_index(2).write(6);
            let w = ws();
            p.wrapping_index(3).write(0xA);
            let sx = ftol32(w.fmul_m(k(0x42C8_0000)));
            sz.write(sx);
            let h = hs();
            let sy = ftol32(x(k(0x4250_0000)).fmul(h));
            let ry = ftol32(h.fmul_m(k(0x4270_0000)));
            sz.wrapping_index(1).write(sy);
            sz.wrapping_index(2).write(sx);
            sz.wrapping_index(3).write(ry);
        }
        2 => {
            p.wrapping_index(1).write(0x20);
            HEAT_HAZE_MASK_MODE_TEST.set(0);
            p.wrapping_index(2).write(4);
            let w = ws();
            p.wrapping_index(3).write(8);
            sz.write(ftol32(x(k(0x428C_0000)).fmul(w)));
            let h = hs();
            let hf = f32_of(h);
            sz.wrapping_index(1).write(ftol32(h.fmul_m(k(0x428C_0000))));
            sz.wrapping_index(2).write(ftol32(w.fmul_m(k(0x42A0_0000))));
            sz.wrapping_index(3).write(ftol32(x(hf).fmul_m(k(0x42A0_0000))));
        }
        3 | 4 => {
            HEAT_HAZE_MASK_MODE_TEST.set(u32::from(ty == 4));
            let w = ws();
            p.wrapping_index(1).write(0x96);
            p.wrapping_index(2).write(5);
            p.wrapping_index(3).write(8);
            sz.write(ftol32(x(k(0x4270_0000)).fmul(w)));
            let sy = ftol32(hs().fmul_m(k(0x41C0_0000)));
            sz.wrapping_index(1).write(sy);
            sz.wrapping_index(2).write(ftol32(w.fmul_m(k(0x4278_0000))));
            sz.wrapping_index(3).write(sy);
        }
        _ => {}
    }
    p.write(ty);
    for i in 0..180i32 {
        let r = GPtr::<u8>::from_addr(RASTER_FRONT_BUFFER.get() as usize);
        let range = rd::<i32>(r, 0xC).wrapping_sub(sz.read());
        let v = haze_random(0x7016B7, range);
        let r = GPtr::<u8>::from_addr(RASTER_FRONT_BUFFER.get() as usize);
        let scan_y = sz.wrapping_index(1).read();
        HP_X.ptr().cast::<i32>().wrapping_index(i).write(v);
        let range = rd::<i32>(r, 0x10).wrapping_sub(scan_y);
        let v = haze_random(0x7016F3, range);
        let max = p.wrapping_index(3).read();
        let min = p.wrapping_index(2).read();
        HP_Y.ptr().cast::<i32>().wrapping_index(i).write(v);
        let v = haze_random(0x701726, max.wrapping_sub(min));
        HP_S.ptr().cast::<i32>().wrapping_index(i).write(v.wrapping_add(min));
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_SETUP_BACK_BUFFER_VERTEX,
    HOOK_IMMEDIATE_MODE_FILTER_STUFF_INITIALIZE,
    HOOK_HEAT_HAZE_FX_INIT,
    HOOK_IMMEDIATE_MODE_RENDER_STATES_RESTORE,
    HOOK_IMMEDIATE_MODE_RENDER_STATES_SET,
    HOOK_INITIALISE,
    HOOK_SCRIPT_RESET_FOR_EFFECTS,
    HOOK_IS_VISION_FX_ACTIVE,
    HOOK_UPDATE,
];
