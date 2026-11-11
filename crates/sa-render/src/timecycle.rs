//! PORT-RENDER-2: `CTimeCycle` and `CColourSet`. Ported from the 1.0 US bytes; gta-reversed
//! `TimeCycle.cpp` / `ColourSet.cpp` and `docs/re/specs/RENDER/` as reading aids.
//!
//! `CColourSet` (0xAC bytes): +0x00..+0x14 ambient / ambient_obj (f32), +0x18..+0x20 ambient before
//! brightness (f32, not set by the constructor, not interpolated), +0x24..+0x3A sky top / bottom, sun
//! core / corona (u16), +0x3C sun size, +0x40 sprite size, +0x44 sprite brightness (f32), +0x48..+0x4C
//! shadow strengths (i16), +0x50 far clip, +0x54 fog start, +0x58 lights-on-ground brightness (f32),
//! +0x5C..+0x66 low / fluffy cloud colours (u16), +0x68..+0x98 water, post FX 1 / 2, cloud alpha (f32),
//! +0x9C high-light min intensity (i32), +0xA0 water fog alpha (u16), +0xA4 directional mult, +0xA8
//! (f32).
//!
//! `CTimeCycleBox` (0x28 bytes, `m_aBoxes[20]` at 0xB7C550): +0 CBox (min, max), +0x18 far clip (i16),
//! +0x1A LOD distance multiplier * 32 (u8), +0x1C extra colour (i32), +0x20 strength, +0x24 falloff (f32).
//!
//! `m_CurrentColours` (0xB7C4A0) is addressed through `g` (other crates declare globals inside it).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, jne, jp, k, rd, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// `CTimeCycle::m_NumBoxes`
    pub static NUM_BOXES @ 0xB7C480: u32 = 0;
    /// `CTimeCycle::m_bExtraColourOn`
    pub static EXTRA_COLOUR_ON @ 0xB7C484: u32 = 0;
    /// `CTimeCycle::m_ExtraColourInter`
    pub static EXTRA_COLOUR_INTER @ 0xB79E3C: u32 = 0;
    /// `CTimeCycle::m_ExtraColourWeatherType`
    pub static EXTRA_COLOUR_WEATHER_TYPE @ 0xB79E40: i32 = 0;
    /// `CTimeCycle::m_ExtraColour`
    pub static EXTRA_COLOUR @ 0xB79E44: i32 = 0;
    /// `CTimeCycle::m_aBoxes[20]` (`CTimeCycleBox`, 0x28 bytes)
    pub static BOXES @ 0xB7C550: [u8; 20 * 0x28] = [0; 20 * 0x28];
    /// `gfLaRiotsLightMult` (initially 1.0)
    pub static LA_RIOTS_LIGHT_MULT @ 0x8CD060: f32 = 1.0;
    /// `CTimeCycle::m_BelowHorizonGrey` (RwRGBA)
    pub static BELOW_HORIZON_GREY @ 0xB7CB10: [u8; 4] = [0; 4];
    /// `CTimeCycle::m_FogReduction`
    pub static FOG_REDUCTION @ 0xB79E48: i32 = 0;
    /// `CTimeCycle::m_CurrentStoredValue` (the sun-vector / shadow ring index)
    pub static CURRENT_STORED_VALUE @ 0xB79FD0: u32 = 0;
    /// `CTimeCycle::m_VectorToSun[16]`
    pub static VECTOR_TO_SUN @ 0xB7CA50: [f32; 48] = [0.0; 48];
    /// The old weather type `CalcColoursForPoint` used (word copy of `CWeather::OldWeatherType`)
    pub static CURRENT_OLD_WEATHER @ 0xB7CB20: i16 = 0;
    /// The new weather type `CalcColoursForPoint` used (word copy of `CWeather::NewWeatherType`)
    pub static CURRENT_NEW_WEATHER @ 0xB7CB24: i16 = 0;
    /// `CTimeCycle::m_fShadowDisplacementY[16]`, `..X`, `m_fShadowSideY`, `..X`, `m_fShadowFrontY`,
    /// `..X` (16 floats each, 0xB79E50..0xB79FD0; written by `CShadows::CalcPedShadowValues`)
    pub static SHADOW_VALUES @ 0xB79E50: [f32; 6 * 16] = [0.0; 6 * 16];
}

/// `CTimeCycle::m_CurrentColours` (CColourSet).
pub const CURRENT_COLOURS: usize = 0xB7C4A0;
const BOXES_BASE: u32 = 0xB7C550;

/// 0.01 (0x858C58), 4.0 (0x858B90), 32.0 (0x85950C), 1/3 (0x859040), 0.0 (0x858B50), 1.0 (0x858624),
/// 3.0 (0x858B3C)
const K_0_01: u32 = 0x3C23_D70A;
const K_FOUR: u32 = 0x4080_0000;
const K_32: u32 = 0x4200_0000;
const K_THIRD: u32 = 0x3EAA_AAAB;
const K_ZERO: u32 = 0;
const K_ONE: u32 = 0x3F80_0000;
const K_THREE: u32 = 0x4040_0000;
/// `CalcColoursForPoint` constants: 1/60 (0x859044), 1/3600 (0x864E20), 23.999 (0x864E1C), 20.0
/// (0x858BA4), 0.005 (0x858B4C), 0.25 (0x858C84), 10.15625 (0x864E18), pi/720 (0x864E14), 0.7 (0x858CB0),
/// 0.2 (0x858CC4), 1/32 (0x864E10), 1/120 (0x864E0C), 1/255 (0x859A3C), -0.9 (0x858CAC), 200.0
/// (0x858A48), 500.0 (0x858B58), 1000.0 (0x858C4C), 1/300 (0x864E08), 256.0 (0x858FB4), 1/256
/// (0x859AA0), 0.8 (0x858C98), 1/128 (0x858B88), 0.06 (0x859934), 0.001 (0x858CDC), -3000.0 (0x859A90),
/// 3000.0 (0x859A94)
const K_1_60: u32 = 0x3C88_8889;
const K_1_3600: u32 = 0x3991_A2B4;
const K_23_999: u32 = 0x41BF_FDF4;
const K_20: u32 = 0x41A0_0000;
const K_0_005: u32 = 0x3BA3_D70A;
const K_0_25: u32 = 0x3E80_0000;
const K_FOG_RED: u32 = 0x4122_8000;
const K_SUN_ANGLE: u32 = 0x3B8E_FA35;
const K_0_7: u32 = 0x3F33_3333;
const K_0_2: u32 = 0x3E4C_CCCD;
const K_1_32: u32 = 0x3D00_0000;
const K_1_120: u32 = 0x3C08_8889;
const K_1_255: u32 = 0x3B80_8081;
const K_M0_9: u32 = 0xBF66_6666;
const K_200: u32 = 0x4348_0000;
const K_500: u32 = 0x43FA_0000;
const K_1000: u32 = 0x447A_0000;
const K_1_300: u32 = 0x3B5A_740E;
const K_256: u32 = 0x4380_0000;
const K_1_256: u32 = 0x3B80_0000;
const K_0_8: u32 = 0x3F4C_CCCD;
const K_1_128: u32 = 0x3C00_0000;
const K_0_06: u32 = 0x3D75_C28F;
const K_0_001: u32 = 0x3A83_126F;
const K_0_001_F: u32 = 0x3A83_126F;
const K_M3000: u32 = 0xC53B_8000;
const K_3000: u32 = 0x453B_8000;
/// `Initialise` constants: 10.0 (0x85862C), 0.5 (0x858B8C), 100.0 (0x858628)
const K_TEN: u32 = 0x4120_0000;
const K_HALF: u32 = 0x3F00_0000;
const K_100: u32 = 0x42C8_0000;
/// `TimeSamples` (0x8CDECC, 9 bytes), `GreyValuesDuringDay` (0x8CDED8, 8 bytes), `TunnelWeather` (0x8CDEE0)
const TIME_TABLE: u32 = 0x8CDECC;
const HORIZON_TABLE: u32 = 0x8CDED8;
const TUNNEL_EXTRA_COLOUR: usize = 0x8CDEE0;
/// `CWeather::InterpolationValue` (0xC8130C), `OldWeatherType` (0xC81320), `NewWeatherType` (0xC8131C),
/// `UnderWaterness` (0xC8132C), `InTunnelness` (0xC81334)
const WEATHER_INTERPOLATION: usize = 0xC8130C;
const OLD_WEATHER: usize = 0xC81320;
const NEW_WEATHER: usize = 0xC8131C;
const UNDER_WATERNESS: usize = 0xC8132C;
const IN_TUNNELNESS: usize = 0xC81334;
/// The extra-colour box blend: sky words, then the float groups before / after far clip and fog start.
const CALC_BOX_WORDS: [(isize, u32); 6] =
    [(0x24, 0xB7BF78), (0x26, 0xB7BEC0), (0x28, 0xB7BE08), (0x2A, 0xB7BD50), (0x2C, 0xB7BC98), (0x2E, 0xB7BBE0)];
const CALC_BOX_FLOATS_A: [(isize, u32); 10] = [
    (0x68, 0xB7AAA0),
    (0x6C, 0xB7A9E8),
    (0x70, 0xB7A930),
    (0x74, 0xB7A878),
    (0x0, 0xB7C3C8),
    (0x4, 0xB7C310),
    (0x8, 0xB7C258),
    (0xC, 0xB7C1A0),
    (0x10, 0xB7C0E8),
    (0x14, 0xB7C030),
];
const CALC_BOX_FLOATS_B: [(isize, u32); 8] = [
    (0x78, 0xB7A7C0),
    (0x7C, 0xB7A708),
    (0x80, 0xB7A650),
    (0x84, 0xB7A598),
    (0x88, 0xB7A4E0),
    (0x8C, 0xB7A428),
    (0x90, 0xB7A370),
    (0x94, 0xB7A2B8),
];

orig! {
    /// `CTimeCycle::CalcColoursForPoint(CVector point, CColourSet* out)` (0x5603D0; the CVector by value)
    pub fn calc_colours_for_point(px: u32, py: u32, pz: u32, out: GPtr<u8>) = cdecl 0x5603D0 => unported;
    /// `CTimeCycle::FindTimeCycleBox(CVector, CTimeCycleBox**, float*, bool, bool, CTimeCycleBox*)` (0x55FFD0)
    pub fn find_time_cycle_box_call(x: u32, y: u32, z: u32, out: GPtr<u8>, interp: GPtr<u8>, lod: u32, far: u32, excl: u32) = cdecl 0x55FFD0 => unported;
    /// `CColourSet::CColourSet(int32 timeId, int32 weatherId)` (0x55F4B0)
    pub fn colour_set_ctor_call(s: GPtr<u8>, time_id: i32, weather_id: i32) -> GPtr<u8> = thiscall 0x55F4B0 => unported;
    /// `CColourSet::Interpolate(a, b, fa, fb, ignoreSky)` (0x55F870)
    pub fn colour_set_interpolate_call(s: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, fa: u32, fb: u32, ignore_sky: u32) = thiscall 0x55F870 => unported;
    /// `CVector::Normalise()` (0x59C910)
    pub fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CShadows::CalcPedShadowValues(CVector sunPos, float* frontX, frontY, sideX, sideY, dispX, dispY)` (0x7076C0)
    pub fn calc_ped_shadow_values(x: u32, y: u32, z: u32, fx: GPtr<u8>, fy: GPtr<u8>, sx: GPtr<u8>, sy: GPtr<u8>, dx: GPtr<u8>, dy: GPtr<u8>) = cdecl 0x7076C0 => unported;
    /// `CCullZones::PlayerNoRain()` (0x72DDC0)
    pub fn player_no_rain() -> u8 = cdecl 0x72DDC0 => unported;
    /// `CCullZones::CamNoRain()` (0x72DDB0)
    pub fn cam_no_rain() -> u8 = cdecl 0x72DDB0 => unported;
    /// `CTimeCycle::SetConstantParametersForPostFX()` (0x560210)
    pub fn set_constant_parameters_for_post_fx_call() = cdecl 0x560210 => unported;
    /// `CFileMgr::SetDir(const char*)` (0x5387D0)
    pub fn file_mgr_set_dir(d: u32) = cdecl 0x5387D0 => unported;
    /// `CFileMgr::OpenFile(const char* name, const char* mode)` (0x538900)
    pub fn file_mgr_open_file(name: u32, mode: u32) -> u32 = cdecl 0x538900 => unported;
    /// `CFileMgr::CloseFile(FILE*)` (0x5389D0)
    pub fn file_mgr_close_file(f: u32) -> i32 = cdecl 0x5389D0 => unported;
    /// `CFileLoader::LoadLine(FILE*)` (0x536F80)
    pub fn load_line(f: u32) -> GPtr<u8> = cdecl 0x536F80 => unported;
    /// CRT `sscanf(line, fmt, ...)` (0x8220AD) with the 52 pointers `CTimeCycle::Initialise` passes
    pub fn sscanf_52(line: GPtr<u8>, fmt: u32, p0: u32, p1: u32, p2: u32, p3: u32, p4: u32, p5: u32, p6: u32, p7: u32, p8: u32, p9: u32, p10: u32, p11: u32, p12: u32, p13: u32, p14: u32, p15: u32, p16: u32, p17: u32, p18: u32, p19: u32, p20: u32, p21: u32, p22: u32, p23: u32, p24: u32, p25: u32, p26: u32, p27: u32, p28: u32, p29: u32, p30: u32, p31: u32, p32: u32, p33: u32, p34: u32, p35: u32, p36: u32, p37: u32, p38: u32, p39: u32, p40: u32, p41: u32, p42: u32, p43: u32, p44: u32, p45: u32, p46: u32, p47: u32, p48: u32, p49: u32, p50: u32, p51: u32) -> i32 = cdecl 0x8220AD => unported;
    /// `CPostEffects::IsVisionFXActive()` (0x7034F0)
    pub fn is_vision_fx_active() -> u8 = cdecl 0x7034F0 => unported;
    /// `CRGBA::CRGBA(r, g, b, a)` (0x7170C0)
    pub fn rgba_ctor(c: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// CRT `_atexit` (0x821D1E)
    pub fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
}

fn cur(off: usize) -> usize {
    CURRENT_COLOURS + off
}

/// `fld dword [0x8CD060]` + `fmul dword [field]`.
fn riots_mul(off: usize) -> X87 {
    x(LA_RIOTS_LIGHT_MULT.get()).fmul_m(g::<f32>(cur(off)).read())
}

/// `CTimeCycle::GetAmbientRed()` (0x560330, ST0): `gfLaRiotsLightMult * m_CurrentColours.m_fAmbientRed`.
#[hook(addr = 0x560330, conv = cdecl, name = "CTimeCycle::GetAmbientRed", expect = "D9 05 60 D0 8C 00")]
pub fn get_ambient_red() -> X87 {
    riots_mul(0x0)
}

/// `CTimeCycle::GetAmbientGreen()` (0x560340, ST0).
#[hook(addr = 0x560340, conv = cdecl, name = "CTimeCycle::GetAmbientGreen", expect = "D9 05 60 D0 8C 00")]
pub fn get_ambient_green() -> X87 {
    riots_mul(0x4)
}

/// `CTimeCycle::GetAmbientBlue()` (0x560350, ST0).
#[hook(addr = 0x560350, conv = cdecl, name = "CTimeCycle::GetAmbientBlue", expect = "D9 05 60 D0 8C 00")]
pub fn get_ambient_blue() -> X87 {
    riots_mul(0x8)
}

/// `CTimeCycle::GetAmbientRed_Obj()` (0x560360, ST0): `m_CurrentColours.m_fAmbientRed_Obj`.
#[hook(addr = 0x560360, conv = cdecl, name = "CTimeCycle::GetAmbientRed_Obj", expect = "D9 05 AC C4 B7 00")]
pub fn get_ambient_red_obj() -> X87 {
    x(g::<f32>(cur(0xC)).read())
}

/// `CTimeCycle::GetAmbientGreen_Obj()` (0x560370, ST0).
#[hook(addr = 0x560370, conv = cdecl, name = "CTimeCycle::GetAmbientGreen_Obj", expect = "D9 05 B0 C4 B7 00")]
pub fn get_ambient_green_obj() -> X87 {
    x(g::<f32>(cur(0x10)).read())
}

/// `CTimeCycle::GetAmbientBlue_Obj()` (0x560380, ST0).
#[hook(addr = 0x560380, conv = cdecl, name = "CTimeCycle::GetAmbientBlue_Obj", expect = "D9 05 B4 C4 B7 00")]
pub fn get_ambient_blue_obj() -> X87 {
    x(g::<f32>(cur(0x14)).read())
}

/// `CTimeCycle::GetAmbientRed_BeforeBrightness()` (0x560390, ST0): riots mult * +0x18.
#[hook(addr = 0x560390, conv = cdecl, name = "CTimeCycle::GetAmbientRed_BeforeBrightness", expect = "D9 05 60 D0 8C 00")]
pub fn get_ambient_red_before_brightness() -> X87 {
    riots_mul(0x18)
}

/// `CTimeCycle::GetAmbientGreen_BeforeBrightness()` (0x5603A0, ST0).
#[hook(addr = 0x5603A0, conv = cdecl, name = "CTimeCycle::GetAmbientGreen_BeforeBrightness", expect = "D9 05 60 D0 8C 00")]
pub fn get_ambient_green_before_brightness() -> X87 {
    riots_mul(0x1C)
}

/// `CTimeCycle::GetAmbientBlue_BeforeBrightness()` (0x5603B0, ST0).
#[hook(addr = 0x5603B0, conv = cdecl, name = "CTimeCycle::GetAmbientBlue_BeforeBrightness", expect = "D9 05 60 D0 8C 00")]
pub fn get_ambient_blue_before_brightness() -> X87 {
    riots_mul(0x20)
}

/// `CTimeCycle::StartExtraColour(int32 colour, bool fade)` (0x55FEC0): `m_ExtraColour = colour % 8`
/// (C remainder), `m_ExtraColourWeatherType = colour / 8 + 21`, on, inter 0, or 1.0 without `fade`.
/// The entry jumps into the protection chunk 0x401D6D (`and ecx, 0x80000007`), which jumps back to
/// 0x55FECC.
#[hook(addr = 0x55FEC0, conv = cdecl, name = "CTimeCycle::StartExtraColour", expect = "8B 44 24 04 8B C8")]
pub fn start_extra_colour(colour: i32, fade: u32) {
    EXTRA_COLOUR_WEATHER_TYPE.set(colour.wrapping_div(8).wrapping_add(21));
    EXTRA_COLOUR.set(colour.wrapping_rem(8));
    EXTRA_COLOUR_ON.set(1);
    EXTRA_COLOUR_INTER.set(0);
    if fade as u8 == 0 {
        EXTRA_COLOUR_INTER.set(K_ONE);
    }
}

/// `CTimeCycle::StopExtraColour(bool fade)` (0x55FF20): off; the inter is cleared only without `fade`.
#[hook(addr = 0x55FF20, conv = cdecl, name = "CTimeCycle::StopExtraColour", expect = "8A 44 24 04 84 C0")]
pub fn stop_extra_colour(fade: u32) {
    EXTRA_COLOUR_ON.set(0);
    if fade as u8 == 0 {
        EXTRA_COLOUR_INTER.set(0);
    }
}

/// `CTimeCycle::Shutdown()` (0x5601E0): a single `ret` (not hookable).
pub fn shutdown() {}

/// `CTimeCycle::InitForRestart()` (0x5601F0): extra colour off, inter 0.
#[hook(addr = 0x5601F0, conv = cdecl, name = "CTimeCycle::InitForRestart", expect = "C7 05 84 C4 B7 00 00 00 00 00")]
pub fn init_for_restart() {
    EXTRA_COLOUR_ON.set(0);
    EXTRA_COLOUR_INTER.set(0);
}

/// `CTimeCycle::AddOne(CBox& box, int16 farClip, int32 extraColor, float strength, float falloff,
/// float lodDistMult)` (0x55FF40): box `m_NumBoxes` gets the CBox, far clip, extra colour, `strength *
/// 0.01`, falloff and `_ftol(min(lodDistMult, 4.0) * 32)` (the clamp only when `> 4.0`, ordered; the
/// original writes 4.0 into its own argument slot), then `m_NumBoxes++`.
#[hook(addr = 0x55FF40, conv = cdecl, name = "CTimeCycle::AddOne", expect = "D9 44 24 10 66 8B 44 24 08")]
pub fn add_one(bx: GPtr<u8>, far_clip: u32, extra_colour: i32, strength: u32, falloff: u32, lod_mult: u32) {
    let n = NUM_BOXES.get();
    let e = GPtr::<u8>::from_addr(BOXES_BASE.wrapping_add(n.wrapping_mul(0x28)) as usize);
    let s = x(f32::from_bits(strength)).fmul_m(k(K_0_01));
    for i in 0..6 {
        wr::<u32>(e, i * 4, rd::<u32>(bx, i * 4));
    }
    wr::<u16>(e, 0x18, far_clip as u16);
    wr::<i32>(e, 0x1C, extra_colour);
    wr::<f32>(e, 0x20, f32_of(s));
    wr::<u32>(e, 0x24, falloff);
    let mut lm = f32::from_bits(lod_mult);
    if !jne(x(lm), x(k(K_FOUR)), 0x41) {
        lm = k(K_FOUR);
    }
    let v = ftol32(x(lm).fmul_m(k(K_32)));
    NUM_BOXES.set(n.wrapping_add(1));
    wr::<u8>(e, 0x1A, v as u8);
}

/// `CTimeCycle::FindTimeCycleBox(CVector point, CTimeCycleBox** outBox, float* interp, bool checkLod,
/// bool checkFar, CTimeCycleBox* exclude)` (0x55FFD0). Over the boxes (count re-read each pass): skips a
/// LOD-checked box whose multiplier byte is exactly 32, a far-checked box with far clip 0, `exclude`, and
/// points outside the box grown by the falloff (z by falloff / 3, stored as f32). The distance to the box
/// (`dz` scaled by 3, sum `(dz^2 + dy^2) + dx^2`) gives `1 - dist / falloff`; a larger value than
/// `*interp` (or distance 0: 1.0) selects the box. The original uses its `outBox` / `interp` argument
/// slots as scratch (stack only).
#[hook(addr = 0x55FFD0, conv = cdecl, name = "CTimeCycle::FindTimeCycleBox", expect = "55 56 8B 74 24 1C")]
#[allow(clippy::too_many_arguments)]
pub fn find_time_cycle_box(
    px: u32,
    py: u32,
    pz: u32,
    out_box: GPtr<GPtr<u8>>,
    interp: GPtr<f32>,
    check_lod: u32,
    check_far: u32,
    exclude: GPtr<u8>,
) {
    let (px, py, pz) = (f32::from_bits(px), f32::from_bits(py), f32::from_bits(pz));
    out_box.write(GPtr::NULL);
    interp.cast::<u32>().write(0);
    let mut i: i32 = 0;
    while i < NUM_BOXES.get() as i32 {
        let b = GPtr::<u8>::from_addr(BOXES_BASE.wrapping_add((i as u32).wrapping_mul(0x28)) as usize);
        i += 1;
        let f = |o: isize| rd::<f32>(b, o);
        if check_lod as u8 != 0 && !jp(xi(i32::from(rd::<u8>(b, 0x1A))), x(k(K_32)), 0x44) {
            continue;
        }
        if check_far as u8 != 0 && rd::<u16>(b, 0x18) == 0 {
            continue;
        }
        if b == exclude {
            continue;
        }
        let fo = f(0x24);
        if jp(x(f(0)).fsub_m(fo), x(px), 0x41) || jp(x(f(4)).fsub_m(fo), x(py), 0x41) {
            continue;
        }
        let fz = f32_of(x(fo).fmul_m(k(K_THIRD)));
        if jp(x(f(8)).fsub_m(fz), x(pz), 0x41) {
            continue;
        }
        if jne(x(fo).fadd_m(f(0xC)), x(px), 0x01)
            || jne(x(fo).fadd_m(f(0x10)), x(py), 0x01)
            || jne(x(fz).fadd_m(f(0x14)), x(pz), 0x01)
        {
            continue;
        }
        // distance to the box along one axis: p > max: p - max; p < min: min - p; else 0
        let axis = |p: f32, min: f32, max: f32| -> X87 {
            if !jne(x(p), x(max), 0x41) {
                x(p).fsub_m(max)
            } else if jp(x(p), x(min), 0x05) {
                x(k(K_ZERO))
            } else {
                x(min).fsub_m(p)
            }
        };
        let dx = axis(px, f(0), f(0xC));
        let dy = axis(py, f(4), f(0x10));
        let dz = axis(pz, f(8), f(0x14)).fmul_m(k(K_THREE));
        let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
        if jp(d, x(k(K_ZERO)), 0x41) {
            let v = d.fdiv_m(fo).fsubr_m(k(K_ONE));
            if jne(v, x(interp.read()), 0x41) {
                continue;
            }
            out_box.write(b);
            interp.write(f32_of(v));
        } else {
            out_box.write(b);
            interp.cast::<u32>().write(K_ONE);
        }
    }
}

/// `CTimeCycle::SetConstantParametersForPostFX()` (0x560210): while a vision FX is active, night vision
/// sets shadow strengths 0, ambient / ambient_obj (0, 0.4, 0) and sky top / bottom (0, 0x80, 0);
/// infrared vision (checked after) sets shadow strengths, lights-on-ground, high-light intensity and
/// water fog alpha 0, ambient / ambient_obj (0, 0, 1) and sky top / bottom (0, 0, 0x80).
#[hook(addr = 0x560210, conv = cdecl, name = "CTimeCycle::SetConstantParametersForPostFX", expect = "E8 DB 32 1A 00")]
pub fn set_constant_parameters_for_post_fx() {
    if is_vision_fx_active() == 0 {
        return;
    }
    let w16 = |o: usize, v: u16| g::<u16>(cur(o)).write(v);
    let w32 = |o: usize, v: u32| g::<u32>(cur(o)).write(v);
    if g::<u8>(crate::posteffects::NIGHT_VISION).read() != 0 {
        w16(0x48, 0);
        w16(0x4A, 0);
        w16(0x4C, 0);
        w32(0x0, 0);
        w32(0x4, 0x3ECC_CCCD);
        w32(0x8, 0);
        w32(0xC, 0);
        w32(0x10, 0x3ECC_CCCD);
        w32(0x14, 0);
        w16(0x24, 0);
        w16(0x26, 0x80);
        w16(0x28, 0);
        w16(0x2A, 0);
        w16(0x2C, 0x80);
        w16(0x2E, 0);
    }
    if g::<u8>(crate::posteffects::INFRARED_VISION).read() != 0 {
        w16(0x48, 0);
        w16(0x4A, 0);
        w16(0x4C, 0);
        w32(0x58, 0);
        w32(0x9C, 0);
        w16(0xA0, 0);
        w32(0x0, 0);
        w32(0x4, 0);
        w32(0x8, K_ONE);
        w32(0xC, 0);
        w32(0x10, 0);
        w32(0x14, K_ONE);
        w16(0x24, 0);
        w16(0x26, 0);
        w16(0x28, 0x80);
        w16(0x2A, 0);
        w16(0x2C, 0);
        w16(0x2E, 0x80);
    }
}

/// `CTimeCycle::FindFarClipForCoors(CVector point)` (0x5616E0, ST0): `CalcColoursForPoint` into a local
/// colour set with the extra colour switched off, then the extra colour state restored (`m_bExtraColourOn`
/// as `!= 0`, i.e. 0 / 1); returns the set's far clip.
#[hook(addr = 0x5616E0, conv = cdecl, name = "CTimeCycle::FindFarClipForCoors", expect = "81 EC B0 00 00 00")]
pub fn find_far_clip_for_coors(px: u32, py: u32, pz: u32) -> X87 {
    let mut set = [0u32; 0xAC / 4];
    let inter = EXTRA_COLOUR_INTER.get();
    let on = EXTRA_COLOUR_ON.get() != 0;
    EXTRA_COLOUR_ON.set(0);
    EXTRA_COLOUR_INTER.set(0);
    calc_colours_for_point(px, py, pz, GPtr::from_raw(set.as_mut_ptr().cast()));
    let r = x(f32::from_bits(set[0x50 / 4]));
    EXTRA_COLOUR_ON.set(u32::from(on));
    EXTRA_COLOUR_INTER.set(inter);
    r
}

/// `CTimeCycle::Update()` (0x561760): `CalcColoursForPoint(camera position, &m_CurrentColours)`.
#[hook(addr = 0x561760, conv = cdecl, name = "CTimeCycle::Update", expect = "A1 3C F0 B6 00")]
pub fn update() {
    let p = sa_world::world::common::cam_pos().cast::<u32>();
    calc_colours_for_point(p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read(), g(CURRENT_COLOURS));
}

/// The larger of `[o0]` / `[o4]` / `[o8]` as the original computes it: `fld g; fld r; fcomp` keeps g
/// when `r <= g` (or unordered), then the result is replaced by b unless it is `> b`.
fn max3(out: GPtr<u8>) -> X87 {
    let r = rd::<f32>(out, 0);
    let gg = rd::<f32>(out, 4);
    let mut m = if jne(x(r), x(gg), 0x41) { x(gg) } else { x(r) };
    let b = rd::<f32>(out, 8);
    if jne(m, x(b), 0x41) {
        m = x(b);
    }
    m
}

/// `fild` of a `Colors<uint8>` entry.
fn tab8(t: u32, i: u32) -> X87 {
    xi(i32::from(g::<u8>(t.wrapping_add(i) as usize).read()))
}

/// `CTimeCycle::CalcColoursForPoint(CVector point, CColourSet* out)` (0x5603D0). The order of the
/// original, in short: the LOD box, the two nested
/// far-clip boxes (the wider one first: widths `max.x - min.x` compared, the swapped interpolation value
/// goes through the x87) and the extra-colour box; the game time `(min/60 + sec/3600) + hours` (seconds
/// read as a byte here) capped at 23.999 picks the time slots of the 9-entry table at 0x8CDECC; the four
/// colour sets (time slot x old / new weather), each pair pulled towards weather 0 (old/new weather 2) or
/// 1 (weather 3) by the camera height `(z - 20) * 0.005` clamped to (0, 1]; time, then weather
/// interpolation into `out`; sky colours scaled by `(1 / 0x8D4B5C + 3) / 4` and clamped to 255; far clip
/// raised to `FogReduction * 10.15625`; the next sun vector (16-entry ring) from the time of day; the
/// extra-colour box blend (strength * interpolation; far clip `min(box, current)`); LOD multiplier; the
/// far boxes; the scripted extra colour fade (`timeStep / 120`); underwater (weather 20) and tunnel
/// (extra colour 0x8CDEE0) blends; ambient quantised to 1/255; ped shadow values; `FogReduction`
/// counting; far clip above 200 / 500 m; the below-horizon grey; ambient-before-brightness copy and the
/// menu brightness; the map-border directional multiplier; tail call
/// `SetConstantParametersForPostFX`. Sky / ambient compares use the x87 idioms of the bytes; the
/// original's local colour sets live on its stack (here Rust locals passed to the original
/// `CColourSet` functions).
#[hook(addr = 0x5603D0, conv = cdecl, name = "CTimeCycle::CalcColoursForPoint", expect = "81 EC 60 04 00 00")]
pub fn calc_colours_for_point_port(px: u32, py: u32, pz: u32, out: GPtr<u8>) {
    let ptr = |v: &mut u32| GPtr::<u8>::from_raw(v as *mut u32 as *mut u8);
    let (mut lod_box, mut lod_i) = (0u32, 0u32);
    find_time_cycle_box_call(px, py, pz, ptr(&mut lod_box), ptr(&mut lod_i), 1, 0, 0);
    let (mut far1, mut fi1, mut far2, mut fi2) = (0u32, 0u32, 0u32, 0u32);
    find_time_cycle_box_call(px, py, pz, ptr(&mut far1), ptr(&mut fi1), 0, 1, 0);
    if far1 != 0 {
        find_time_cycle_box_call(px, py, pz, ptr(&mut far2), ptr(&mut fi2), 0, 1, far1);
        if far2 != 0 {
            let b1 = GPtr::<u8>::from_addr(far1 as usize);
            let b2 = GPtr::<u8>::from_addr(far2 as usize);
            let w1 = x(rd::<f32>(b1, 0xC)).fsub_m(rd::<f32>(b1, 0));
            let w2 = x(rd::<f32>(b2, 0xC)).fsub_m(rd::<f32>(b2, 0));
            if !jne(w2, w1, 0x41) {
                let i2 = fi2;
                fi2 = f32_of(x(f32::from_bits(fi1))).to_bits();
                core::mem::swap(&mut far1, &mut far2);
                fi1 = i2;
            }
        }
    } else {
        far2 = 0;
    }
    let (mut wbox, mut wi) = (0u32, 0u32);
    find_time_cycle_box_call(px, py, pz, ptr(&mut wbox), ptr(&mut wi), 0, 0, 0);
    let minutes = g::<u8>(0xB70152).read();
    let seconds = g::<u8>(0xB70150).read();
    let hours = g::<u8>(0xB70153).read();
    let mut time = xi(i32::from(minutes))
        .fmul_m(k(K_1_60))
        .fadd(xi(i32::from(seconds)).fmul_m(k(K_1_3600)))
        .fadd(xi(i32::from(hours)));
    if jp(time, x(k(K_23_999)), 0x05) {
        time = x(k(K_23_999));
    }
    let mut cur: u32 = 0;
    if !jne(time, tab8(TIME_TABLE, 1), 0x01) {
        loop {
            let t = tab8(TIME_TABLE, cur + 2);
            cur += 1;
            if jne(time, t, 0x01) {
                break;
            }
        }
    }
    let next = (cur + 1) & 7;
    let t0 = g::<u8>(TIME_TABLE as usize + cur as usize).read();
    let t1 = g::<u8>(TIME_TABLE as usize + cur as usize + 1).read();
    let ti = f32_of(time.fsub(xi(i32::from(t0))).fdiv(xi(i32::from(t1) - i32::from(t0))));
    let omti = f32_of(x(k(K_ONE)).fsub_m(ti));
    let wiv = g::<u32>(WEATHER_INTERPOLATION).read();
    let old_w = g::<i16>(OLD_WEATHER).read();
    let new_w = g::<i16>(NEW_WEATHER).read();
    CURRENT_OLD_WEATHER.set(old_w);
    CURRENT_NEW_WEATHER.set(new_w);
    let (mut box_weather, mut box_hour) = (0i32, 0i32);
    if wbox != 0 {
        let ec = rd::<i32>(GPtr::from_addr(wbox as usize), 0x1C);
        box_weather = i32::from(ec >= 8) + 21;
        box_hour = ec.wrapping_rem(8);
    }
    let omwi = f32_of(x(k(K_ONE)).fsub_m(f32::from_bits(wiv)));
    let mut sa = [0u32; 0x2C];
    let mut sb = [0u32; 0x2C];
    let mut sc = [0u32; 0x2C];
    let mut sd = [0u32; 0x2C];
    let mut stmp = [0u32; 0x2C];
    let mut sold = [0u32; 0x2C];
    let pa = GPtr::<u8>::from_raw(sa.as_mut_ptr().cast());
    let pb = GPtr::<u8>::from_raw(sb.as_mut_ptr().cast());
    let pc = GPtr::<u8>::from_raw(sc.as_mut_ptr().cast());
    let pd = GPtr::<u8>::from_raw(sd.as_mut_ptr().cast());
    let ptmp = GPtr::<u8>::from_raw(stmp.as_mut_ptr().cast());
    let pold = GPtr::<u8>::from_raw(sold.as_mut_ptr().cast());
    colour_set_ctor_call(pa, cur as i32, i32::from(old_w));
    colour_set_ctor_call(pb, next as i32, i32::from(old_w));
    colour_set_ctor_call(pc, cur as i32, i32::from(new_w));
    colour_set_ctor_call(pd, next as i32, i32::from(new_w));
    let cam = sa_world::world::common::cam_pos();
    let mut t = f32_of(x(cam.wrapping_index(2).read()).fsub_m(k(K_20)).fmul_m(k(K_0_005)));
    let apply = if !jne(x(0.0), x(t), 0x41) {
        false
    } else if !jp(x(k(K_ONE)), x(t), 0x05) {
        t = k(K_ONE);
        true
    } else {
        !jne(x(t), x(0.0), 0x41)
    };
    if apply {
        let fa = f32_of(x(k(K_ONE)).fsub_m(t)).to_bits();
        let fb = t.to_bits();
        for (w, s0, s1) in [(old_w, pa, pb), (new_w, pc, pd)] {
            let tw = match w {
                2 => 0,
                3 => 1,
                _ => continue,
            };
            colour_set_ctor_call(ptmp, cur as i32, tw);
            colour_set_interpolate_call(s0, s0, ptmp, fa, fb, 0);
            colour_set_ctor_call(ptmp, next as i32, tw);
            colour_set_interpolate_call(s1, s1, ptmp, fa, fb, 0);
        }
    }
    colour_set_interpolate_call(pold, pa, pb, omti.to_bits(), ti.to_bits(), 0);
    colour_set_interpolate_call(ptmp, pc, pd, omti.to_bits(), ti.to_bits(), 0);
    colour_set_interpolate_call(out, pold, ptmp, omwi.to_bits(), wiv, 0);

    // sky colours scaled and clamped
    let f = x(k(K_ONE)).fdiv_m(g::<f32>(0x8D4B5C).read()).fadd_m(k(K_THREE)).fmul_m(k(K_0_25));
    let mut v = [0u16; 6];
    for (i, vi) in v.iter_mut().enumerate() {
        let o = 0x24 + i as isize * 2;
        *vi = ftol32(xi(i32::from(rd::<u16>(out, o))).fmul(f)) as u16;
        wr::<u16>(out, o, *vi);
    }
    for (i, vi) in v.iter().enumerate() {
        wr::<u16>(out, 0x24 + i as isize * 2, if *vi < 0xFF { *vi } else { 0xFF });
    }
    let fr_n = FOG_REDUCTION.get();
    if fr_n != 0 {
        let fr = xi(fr_n).fmul_m(k(K_FOG_RED));
        let fc = rd::<f32>(out, 0x50);
        let r = if jne(x(fc), fr, 0x41) { fr } else { x(fc) };
        wr::<f32>(out, 0x50, f32_of(r));
    }

    // the next sun vector
    let hm = i32::from(hours) * 60 + i32::from(minutes);
    let secs16 = g::<u16>(0xB70150).read();
    let idx = CURRENT_STORED_VALUE.get().wrapping_add(1) & 0xF;
    CURRENT_STORED_VALUE.set(idx);
    let ang = xi(hm).fadd(xi(i32::from(secs16)).fmul_m(k(K_1_60))).fmul_m(k(K_SUN_ANGLE));
    let sun = GPtr::<u8>::from_addr(0xB7CA50 + idx as usize * 12);
    wr::<f32>(sun, 0, f32_of(ang.fsin().fadd_m(k(K_0_7))));
    wr::<u32>(sun, 4, 0xBF33_3333);
    wr::<f32>(sun, 8, f32_of(ang.fcos().fsubr_m(k(K_0_2))));
    vector_normalise(sun);

    // the extra-colour box
    let wb = GPtr::<u8>::from_addr(wbox as usize);
    if wbox != 0 && rd::<i32>(wb, 0x1C) >= 0 {
        let s = x(f32::from_bits(wi)).fmul_m(rd::<f32>(wb, 0x20));
        let oms = x(k(K_ONE)).fsub(s);
        let i = (box_hour.wrapping_mul(0x17)).wrapping_add(i32::from(box_weather as i16)) as u32;
        for (o, tab) in CALC_BOX_WORDS {
            let r = xi(i32::from(rd::<u16>(out, o))).fmul(oms).fadd(tab8(tab, i).fmul(s));
            wr::<u16>(out, o, ftol32(r) as u16);
        }
        let fl = |o: isize, tab: u32| {
            let r = tab8(tab, i).fmul(s).fadd(oms.fmul_m(rd::<f32>(out, o)));
            wr::<f32>(out, o, f32_of(r));
        };
        for (o, tab) in CALC_BOX_FLOATS_A {
            fl(o, tab);
        }
        let w = g::<i16>(0xB7B1D0u32.wrapping_add(i.wrapping_mul(2)) as usize).read();
        if w as u16 != 0xFFFF {
            let fc = rd::<f32>(out, 0x50);
            let mut vv = xi(i32::from(w));
            if jp(vv, x(fc), 0x05) {
                vv = x(fc);
            }
            wr::<f32>(out, 0x50, f32_of(oms.fmul_m(rd::<f32>(out, 0x50)).fadd(vv.fmul(s))));
        }
        let w = g::<i16>(0xB7B060u32.wrapping_add(i.wrapping_mul(2)) as usize).read();
        if w as u16 != 0xFFFF {
            let r = xi(i32::from(w)).fmul(s).fadd(oms.fmul_m(rd::<f32>(out, 0x54)));
            wr::<f32>(out, 0x54, f32_of(r));
        }
        for (o, tab) in CALC_BOX_FLOATS_B {
            fl(o, tab);
        }
    }
    if lod_box != 0 {
        let li = f32::from_bits(lod_i);
        let lm = rd::<u8>(GPtr::from_addr(lod_box as usize), 0x1A);
        let r = xi(i32::from(lm))
            .fmul_m(k(K_1_32))
            .fmul_m(li)
            .fadd(x(k(K_ONE)).fsub_m(li).fmul_m(rd::<f32>(out, 0xA8)));
        wr::<f32>(out, 0xA8, f32_of(r));
    }
    for (b, fi) in [(far1, fi1), (far2, fi2)] {
        if b == 0 {
            continue;
        }
        let fi = f32::from_bits(fi);
        let fc = rd::<f32>(out, 0x50);
        let mut vv = xi(i32::from(rd::<i16>(GPtr::from_addr(b as usize), 0x18)));
        if jp(vv, x(fc), 0x05) {
            vv = x(fc);
        }
        let r = x(k(K_ONE)).fsub_m(fi).fmul_m(fc).fadd(vv.fmul_m(fi));
        wr::<f32>(out, 0x50, f32_of(r));
    }

    // the scripted extra colour fade
    let ts = x(g::<f32>(0xB7CB5C).read()).fmul_m(k(K_1_120));
    let inter = f32::from_bits(EXTRA_COLOUR_INTER.get());
    let mut do_extra = false;
    let stored = if EXTRA_COLOUR_ON.get() != 0 {
        let vv = ts.fadd_m(inter);
        if !jp(x(k(K_ONE)), vv, 0x05) {
            EXTRA_COLOUR_INTER.set(K_ONE);
            do_extra = true;
            None
        } else {
            Some(vv)
        }
    } else {
        let vv = ts.fsubr_m(inter);
        if !jne(x(0.0), vv, 0x41) {
            EXTRA_COLOUR_INTER.set(0);
            None
        } else {
            Some(vv)
        }
    };
    if let Some(vv) = stored {
        EXTRA_COLOUR_INTER.set(f32_of(vv).to_bits());
        do_extra = !jne(x(f32::from_bits(EXTRA_COLOUR_INTER.get())), x(0.0), 0x41);
    }
    if do_extra {
        let ew = EXTRA_COLOUR_WEATHER_TYPE.get();
        let ec = EXTRA_COLOUR.get();
        let fa = f32_of(x(k(K_ONE)).fsub_m(f32::from_bits(EXTRA_COLOUR_INTER.get())));
        colour_set_ctor_call(pold, ec, ew);
        let i = ec.wrapping_mul(0x17).wrapping_add(ew) as u32;
        let arg = (i & 0xFFFF_FF00) | sky_top_zero(i);
        colour_set_interpolate_call(out, out, pold, fa.to_bits(), EXTRA_COLOUR_INTER.get(), arg);
    }

    // underwater
    let uw = g::<f32>(UNDER_WATERNESS).read();
    if !jne(x(uw), x(0.0), 0x41) {
        let mut s1 = [0u32; 0x2C];
        let p1 = GPtr::<u8>::from_raw(s1.as_mut_ptr().cast());
        colour_set_ctor_call(ptmp, cur as i32, 20);
        colour_set_ctor_call(pold, next as i32, 20);
        colour_set_interpolate_call(p1, ptmp, pold, omti.to_bits(), ti.to_bits(), 0);
        let fa = f32_of(x(k(K_ONE)).fsub_m(g::<f32>(UNDER_WATERNESS).read()));
        colour_set_interpolate_call(out, out, p1, fa.to_bits(), g::<u32>(UNDER_WATERNESS).read(), 0);
    }
    // tunnel
    let tn = g::<f32>(IN_TUNNELNESS).read();
    if !jne(x(tn), x(0.0), 0x41) {
        let fa = f32_of(x(k(K_ONE)).fsub_m(tn));
        let val = g::<i32>(TUNNEL_EXTRA_COLOUR).read();
        let ec = val.wrapping_rem(8);
        let adj = val.wrapping_add((val >> 31) & 7);
        let ew = (adj >> 3).wrapping_add(21);
        colour_set_ctor_call(pold, ec, ew);
        let i = ec.wrapping_mul(0x17).wrapping_add(ew) as u32;
        // EAX still holds the constructor's return value (the set's address) when AL is set
        let arg = (pold.addr() as u32 & 0xFFFF_FF00) | sky_top_zero(i);
        colour_set_interpolate_call(out, out, pold, fa.to_bits(), g::<u32>(IN_TUNNELNESS).read(), arg);
    }
    for o in [0x0, 0x4, 0x8, 0xC, 0x10, 0x14] {
        let q = ftol32(x(rd::<f32>(out, o)));
        wr::<f32>(out, o, f32_of(xi(q).fmul_m(k(K_1_255))));
    }
    let si = CURRENT_STORED_VALUE.get();
    let sv = GPtr::<u32>::from_addr(0xB7CA50 + si as usize * 12);
    let a4 = |base: usize| GPtr::<u8>::from_addr(base + si as usize * 4);
    calc_ped_shadow_values(
        sv.read(),
        sv.wrapping_index(1).read(),
        sv.wrapping_index(2).read(),
        a4(0xB79F90),
        a4(0xB79F50),
        a4(0xB79F10),
        a4(0xB79ED0),
        a4(0xB79E90),
        a4(0xB79E50),
    );
    let inc = !jp(x(g::<f32>(0xB6F9B4).read()), x(k(K_M0_9)), 0x05)
        || (g::<u8>(0xC812AC).read() == 0
            && (player_no_rain() != 0 || cam_no_rain() != 0 || g::<u8>(0xB5F851).read() != 0));
    if inc {
        let n = FOG_REDUCTION.get().wrapping_add(1);
        FOG_REDUCTION.set(if n >= 0x40 { 0x40 } else { n });
    } else {
        let n = FOG_REDUCTION.get().wrapping_sub(1);
        FOG_REDUCTION.set(if n <= 0 { 0 } else { n });
    }
    // far clip high above the map
    let cz = sa_world::world::common::cam_pos().wrapping_index(2).read();
    if jp(x(cz), x(k(K_200)), 0x05) {
        let fc = rd::<f32>(out, 0x50);
        if !jne(x(cz), x(k(K_500)), 0x41) {
            let r = if jp(x(fc), x(k(K_1000)), 0x05) { x(k(K_1000)) } else { x(fc) };
            wr::<f32>(out, 0x50, f32_of(r));
        } else if !jne(x(fc), x(k(K_1000)), 0x41) {
            let ff = x(cz).fsub_m(k(K_200)).fmul_m(k(K_1_300));
            let r = x(k(K_ONE)).fsub(ff).fmul_m(fc).fadd(ff.fmul_m(k(K_1000)));
            wr::<f32>(out, 0x50, f32_of(r));
        }
    }
    // the below-horizon grey
    let hg = ftol32(tab8(HORIZON_TABLE, cur).fmul_m(omti).fadd(tab8(HORIZON_TABLE, next).fmul_m(ti)));
    let base = x(k(K_ONE)).fsub_m(g::<f32>(UNDER_WATERNESS).read()).fmul(xi(i32::from(hg as u8)));
    for (j, a) in [0xB7C4CAusize, 0xB7C4CC, 0xB7C4CE].into_iter().enumerate() {
        let r = xi(i32::from(g::<u16>(a).read())).fmul_m(g::<f32>(UNDER_WATERNESS).read()).fadd(base);
        BELOW_HORIZON_GREY.ptr().cast::<u8>().wrapping_index(j as i32).write(ftol32(r) as u8);
    }
    let amb = [rd::<u32>(out, 0), rd::<u32>(out, 4), rd::<u32>(out, 8)];
    wr::<u32>(out, 0x18, amb[0]);
    wr::<u32>(out, 0x1C, amb[1]);
    wr::<u32>(out, 0x20, amb[2]);
    // menu brightness
    let bx = xi(g::<i32>(0xBA6784).read());
    let m = if !jp(bx, x(k(K_256)), 0x05) {
        let m = bx.fmul_m(k(K_1_256)).fmul_m(k(K_0_8)).fadd_m(k(K_0_2));
        for o in [0, 4, 8] {
            wr::<f32>(out, o, f32_of(m.fmul_m(rd::<f32>(out, o))));
        }
        m
    } else {
        let m = bx.fsub_m(k(K_256)).fmul_m(k(K_1_128)).fadd_m(k(K_ONE));
        let mx = max3(out);
        let d = mx.fmul(m).fsub(mx);
        for o in [0, 4, 8] {
            wr::<f32>(out, o, f32_of(d.fadd_m(rd::<f32>(out, o))));
        }
        m
    };
    if !jne(m, x(k(K_ONE)), 0x41) {
        let q = m.fsub_m(k(K_ONE)).fmul_m(k(K_0_06));
        let mut mx = max3(out);
        let saved = [rd::<f32>(out, 0), rd::<f32>(out, 4), rd::<f32>(out, 8)];
        if !jp(mx, x(0.0), 0x44) {
            mx = x(k(K_0_001));
            for o in [0, 4, 8] {
                wr::<u32>(out, o, K_0_001);
            }
        }
        if !jne(q, mx, 0x41) {
            let kq = q.fdiv(mx);
            for o in [0, 4, 8] {
                wr::<f32>(out, o, f32_of(kq.fmul_m(rd::<f32>(out, o))));
            }
        }
        for (j, a) in [0xB79E38usize, 0xB79E34, 0xB79E30].into_iter().enumerate() {
            g::<f32>(a).write(f32_of(x(rd::<f32>(out, j as isize * 4)).fsub_m(saved[j])));
        }
    }
    // the map-border directional multiplier
    let (fx, fy) = (f32::from_bits(px), f32::from_bits(py));
    let mut d = x(0.0);
    if !jp(x(fx), x(k(K_M3000)), 0x05) {
        d = x(k(K_M3000)).fsub_m(fx);
    } else if !jne(x(fx), x(k(K_3000)), 0x41) {
        d = x(fx).fsub_m(k(K_3000));
    }
    if !jp(x(fy), x(k(K_M3000)), 0x05) {
        d = d.fsub(x(fy).fadd_m(k(K_3000)));
    } else if !jne(x(fy), x(k(K_3000)), 0x41) {
        d = d.fadd(x(fy).fsub_m(k(K_3000)));
    }
    let a8 = rd::<f32>(out, 0xA8);
    if !jne(d, x(k(K_1000)), 0x01) {
        wr::<f32>(out, 0xA8, f32_of(x(a8).fadd(x(a8))));
    } else if !jne(d, x(0.0), 0x01) {
        wr::<f32>(out, 0xA8, f32_of(d.fmul_m(k(K_0_001_F)).fadd_m(k(K_ONE)).fmul_m(a8)));
    }
    set_constant_parameters_for_post_fx_call();
}

/// The `ignoreSky` flag of the extra-colour blends: the sky-top r, g, b table bytes of entry `i` are all 0.
fn sky_top_zero(i: u32) -> u32 {
    let z = |t: u32| g::<u8>(t.wrapping_add(i) as usize).read() == 0;
    u32::from(z(0xB7BF78) && z(0xB7BEC0) && z(0xB7BE08))
}

/// `_ftol(v * 10 + 0.5)` of a parsed float (sun size, sprite size / brightness, lights-on-ground).
fn tc_scaled(v: u32) -> u8 {
    ftol32(x(f32::from_bits(v)).fmul_m(k(K_TEN)).fadd_m(k(K_HALF))) as u8
}

/// `_ftol` of a parsed float.
fn tc_ftol(v: u32) -> i32 {
    ftol32(x(f32::from_bits(v)))
}

/// `CTimeCycle::Initialise()` (0x5BBAC0): reads "TIMECYC.DAT" from "DATA" (mode "rb"; directory reset to
/// "") for each weather 0..22 and hour slot 0..7 (index `hour * 23 + weather`): lines starting with '/' or
/// empty are skipped (a null line is passed on to `sscanf`, as in the original), then 52 fields
/// (`%d` x21, `%f` x3, `%d` x3, `%f` x3, `%d` x6, `%f` x13, `%d` x2, `%f`) are scanned into locals that
/// persist across lines. Fields 6..8 are unused; sun size / sprite size / sprite brightness /
/// lights-on-ground are `_ftol(v * 10 + 0.5)`; far clip / fog start `_ftol(v)` (16-bit); water and post FX
/// colours `_ftol(v)`, the two post FX alphas `_ftol(v + v)` (fields 40 and 44); directional mult
/// `_ftol(byte(_ftol(v)) * 100)`. After closing the file: `m_vecDirnLightToSun = (cos(-3pi/4) cos(pi/4),
/// sin(-3pi/4) cos(pi/4), sin(pi/4))` normalised (FSIN / FCOS of the qword constants 0x86A6C8 /
/// 0x86A6D0, sa-math's f64 model), fog reduction 0, extra colour off.
#[hook(addr = 0x5BBAC0, conv = cdecl, name = "CTimeCycle::Initialise", expect = "81 EC E0 00 00 00")]
pub fn initialise() {
    file_mgr_set_dir(0x86A784);
    let f = file_mgr_open_file(0x86A778, 0x859C6C);
    file_mgr_set_dir(0x858B54);
    let mut v = [0u32; 52];
    let vp = GPtr::<u32>::from_raw(v.as_mut_ptr());
    for w in 0..23u32 {
        for h in 0..8u32 {
            let i = h * 23 + w;
            let mut line = load_line(f);
            while !line.is_null() {
                let c = line.read();
                if c != b'/' && c != 0 {
                    break;
                }
                line = load_line(f);
            }
            let a = |n: i32| vp.wrapping_index(n).addr() as u32;
            sscanf_52(
                line, 0x86A6D8, a(0), a(1), a(2), a(3), a(4), a(5), a(6), a(7), a(8), a(9), a(10), a(11), a(12),
                a(13), a(14), a(15), a(16), a(17), a(18), a(19), a(20), a(21), a(22), a(23), a(24), a(25), a(26),
                a(27), a(28), a(29), a(30), a(31), a(32), a(33), a(34), a(35), a(36), a(37), a(38), a(39), a(40),
                a(41), a(42), a(43), a(44), a(45), a(46), a(47), a(48), a(49), a(50), a(51),
            );
            let r = |n: i32| vp.wrapping_index(n).read();
            let b = |t: u32, val: u8| g::<u8>(t.wrapping_add(i) as usize).write(val);
            for (t, n) in [
                (0xB7C3C8, 0),
                (0xB7C310, 1),
                (0xB7C258, 2),
                (0xB7C1A0, 3),
                (0xB7C0E8, 4),
                (0xB7C030, 5),
                (0xB7BF78, 9),
                (0xB7BEC0, 10),
                (0xB7BE08, 11),
                (0xB7BD50, 12),
                (0xB7BC98, 13),
                (0xB7BBE0, 14),
                (0xB7BB28, 15),
                (0xB7BA70, 16),
                (0xB7B9B8, 17),
                (0xB7B900, 18),
                (0xB7B848, 19),
                (0xB7B790, 20),
            ] {
                b(t, r(n) as u8);
            }
            b(0xB7B6D8, tc_scaled(r(21)));
            b(0xB7B620, tc_scaled(r(22)));
            b(0xB7B568, tc_scaled(r(23)));
            b(0xB7B4B0, r(24) as u8);
            b(0xB7B3F8, r(25) as u8);
            b(0xB7B340, r(26) as u8);
            g::<u16>(0xB7B1D0u32.wrapping_add(i * 2) as usize).write(tc_ftol(r(27)) as u16);
            g::<u16>(0xB7B060u32.wrapping_add(i * 2) as usize).write(tc_ftol(r(28)) as u16);
            b(0xB7AFA8, tc_scaled(r(29)));
            for (t, n) in [(0xB7AEF0, 30), (0xB7AE38, 31), (0xB7AD80, 32), (0xB7ACC8, 33), (0xB7AC10, 34), (0xB7AB58, 35)] {
                b(t, r(n) as u8);
            }
            for (t, n) in [
                (0xB7AAA0, 36),
                (0xB7A9E8, 37),
                (0xB7A930, 38),
                (0xB7A878, 39),
                (0xB7A7C0, 41),
                (0xB7A708, 42),
                (0xB7A650, 43),
                (0xB7A4E0, 45),
                (0xB7A428, 46),
                (0xB7A370, 47),
            ] {
                b(t, tc_ftol(r(n)) as u8);
            }
            let dbl = |n: i32| {
                let fv = x(f32::from_bits(r(n)));
                ftol32(fv.fadd(fv)) as u8
            };
            b(0xB7A598, dbl(40));
            b(0xB7A2B8, dbl(44));
            b(0xB7A200, tc_ftol(r(48)) as u8);
            b(0xB7A148, r(49) as u8);
            b(0xB7A090, r(50) as u8);
            let d = tc_ftol(r(51)) as u8;
            b(0xB79FD8, ftol32(xi(i32::from(d)).fmul_m(k(K_100))) as u8);
        }
    }
    file_mgr_close_file(f);
    let a1 = X87::from_f64_exact(f64::from_bits(0x3FE9_21FB_6000_0000));
    let a2 = X87::from_f64_exact(f64::from_bits(0xC002_D97C_8000_0000));
    let c1 = a1.fcos();
    let dir = g::<f32>(0xB7CB14);
    dir.write(f32_of(a2.fcos().fmul(c1)));
    dir.wrapping_index(1).write(f32_of(a2.fsin().fmul(c1)));
    dir.wrapping_index(2).write(f32_of(a1.fsin()));
    vector_normalise(dir.cast());
    FOG_REDUCTION.set(0);
    EXTRA_COLOUR_ON.set(0);
}

/// The `Colors<T>` tables (`[8 hours][23 weathers]`) the constructor reads, by destination offset.
/// f32 from u8: ambient, ambient_obj.
const CTOR_F_U8_A: [(usize, u32); 6] =
    [(0x0, 0xB7C3C8), (0x4, 0xB7C310), (0x8, 0xB7C258), (0xC, 0xB7C1A0), (0x10, 0xB7C0E8), (0x14, 0xB7C030)];
/// u16 from u8: sky top / bottom, sun core / corona.
const CTOR_W_A: [(usize, u32); 12] = [
    (0x24, 0xB7BF78),
    (0x26, 0xB7BEC0),
    (0x28, 0xB7BE08),
    (0x2A, 0xB7BD50),
    (0x2C, 0xB7BC98),
    (0x2E, 0xB7BBE0),
    (0x30, 0xB7BB28),
    (0x32, 0xB7BA70),
    (0x34, 0xB7B9B8),
    (0x36, 0xB7B900),
    (0x38, 0xB7B848),
    (0x3A, 0xB7B790),
];
/// f32 from i8: sun size, sprite size, sprite brightness.
const CTOR_F_I8: [(usize, u32); 3] = [(0x3C, 0xB7B6D8), (0x40, 0xB7B620), (0x44, 0xB7B568)];
/// u16 from u8: shadow strengths.
const CTOR_W_B: [(usize, u32); 3] = [(0x48, 0xB7B4B0), (0x4A, 0xB7B3F8), (0x4C, 0xB7B340)];
/// u16 from u8: low clouds, fluffy cloud bottom.
const CTOR_W_C: [(usize, u32); 6] =
    [(0x5C, 0xB7AEF0), (0x5E, 0xB7AE38), (0x60, 0xB7AD80), (0x62, 0xB7ACC8), (0x64, 0xB7AC10), (0x66, 0xB7AB58)];
/// f32 from u8: water, post FX 1 / 2, cloud alpha.
const CTOR_F_U8_B: [(usize, u32); 13] = [
    (0x68, 0xB7AAA0),
    (0x6C, 0xB7A9E8),
    (0x70, 0xB7A930),
    (0x74, 0xB7A878),
    (0x78, 0xB7A7C0),
    (0x7C, 0xB7A708),
    (0x80, 0xB7A650),
    (0x84, 0xB7A598),
    (0x88, 0xB7A4E0),
    (0x8C, 0xB7A428),
    (0x90, 0xB7A370),
    (0x94, 0xB7A2B8),
    (0x98, 0xB7A200),
];

/// `CColourSet::CColourSet(int32 timeId, int32 weatherId)` (0x55F4B0, thiscall, returns `this`): every
/// field from the `[timeId * 23 + weatherId]` entry of its table (bytes converted through `fild`;
/// far clip / fog start from i16 tables), +0x9C as an int, +0xA8 = 1.0, +0xA4 = directional mult * 0.01.
/// The entry jumps into the protection chunk 0x4058B7 (`movzx edi, [ecx+0xB7C3C8]`), which jumps back to
/// 0x55F4C9.
#[hook(addr = 0x55F4B0, conv = thiscall, name = "CColourSet::Constructor", expect = "8B 54 24 04 8B C1")]
pub fn colour_set_ctor(this: GPtr<u8>, time_id: i32, weather_id: i32) -> GPtr<u8> {
    let i = time_id.wrapping_mul(0x17).wrapping_add(weather_id) as u32;
    let b = |t: u32| g::<u8>(t.wrapping_add(i) as usize).read();
    let sb = |t: u32| g::<i8>(t.wrapping_add(i) as usize).read();
    let sw = |t: u32| g::<i16>(t.wrapping_add(i.wrapping_mul(2)) as usize).read();
    for (o, t) in CTOR_F_U8_A {
        wr::<f32>(this, o as isize, f32_of(xi(i32::from(b(t)))));
    }
    for (o, t) in CTOR_W_A {
        wr::<u16>(this, o as isize, u16::from(b(t)));
    }
    for (o, t) in CTOR_F_I8 {
        wr::<f32>(this, o as isize, f32_of(xi(i32::from(sb(t)))));
    }
    for (o, t) in CTOR_W_B {
        wr::<u16>(this, o as isize, u16::from(b(t)));
    }
    wr::<f32>(this, 0x50, f32_of(xi(i32::from(sw(0xB7B1D0)))));
    wr::<f32>(this, 0x54, f32_of(xi(i32::from(sw(0xB7B060)))));
    wr::<f32>(this, 0x58, f32_of(xi(i32::from(b(0xB7AFA8)))));
    for (o, t) in CTOR_W_C {
        wr::<u16>(this, o as isize, u16::from(b(t)));
    }
    for (o, t) in CTOR_F_U8_B {
        wr::<f32>(this, o as isize, f32_of(xi(i32::from(b(t)))));
    }
    wr::<u32>(this, 0x9C, u32::from(b(0xB7A148)));
    wr::<u16>(this, 0xA0, u16::from(b(0xB7A090)));
    let dm = b(0xB79FD8);
    wr::<u32>(this, 0xA8, K_ONE);
    wr::<f32>(this, 0xA4, f32_of(xi(i32::from(dm)).fmul_m(k(K_0_01))));
    this
}

/// `CColourSet::Interpolate(CColourSet* a, CColourSet* b, float fa, float fb, bool ignoreSky)`
/// (0x55F870, thiscall): every field `fa * a + fb * b` (`(a * fa) + (b * fb)` on the x87; integer fields
/// through `fild` and `_ftol`, stored truncated to their width; the shadow strengths sign-extended). The sky
/// group (sky / sun colours, sun size, cloud colours) only without `ignoreSky`; the ambient-before-
/// brightness floats (+0x18..+0x20) are not touched. Field order as the original's (matters when `this`
/// aliases an input). The original uses its `ignoreSky` argument slot as scratch (stack only).
#[hook(addr = 0x55F870, conv = thiscall, name = "CColourSet::Interpolate", expect = "8A 44 24 14 84 C0")]
pub fn colour_set_interpolate(this: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, fa: u32, fb: u32, ignore_sky: u32) {
    let (fa, fb) = (f32::from_bits(fa), f32::from_bits(fb));
    let wu = |o: isize| {
        let r = xi(i32::from(rd::<u16>(a, o))).fmul_m(fa).fadd(xi(i32::from(rd::<u16>(b, o))).fmul_m(fb));
        wr::<u16>(this, o, ftol32(r) as u16);
    };
    let ws = |o: isize| {
        let r = xi(i32::from(rd::<i16>(a, o))).fmul_m(fa).fadd(xi(i32::from(rd::<i16>(b, o))).fmul_m(fb));
        wr::<u16>(this, o, ftol32(r) as u16);
    };
    let fl = |o: isize| {
        let r = x(fa).fmul_m(rd::<f32>(a, o)).fadd(x(fb).fmul_m(rd::<f32>(b, o)));
        wr::<f32>(this, o, f32_of(r));
    };
    if ignore_sky as u8 == 0 {
        for o in (0x24..=0x3A).step_by(2) {
            wu(o);
        }
        fl(0x3C);
        for o in (0x5C..=0x66).step_by(2) {
            wu(o);
        }
    }
    for o in [0x0, 0x4, 0x8, 0xC, 0x10, 0x14, 0x40, 0x44] {
        fl(o);
    }
    for o in [0x48, 0x4A, 0x4C] {
        ws(o);
    }
    for o in [0x50, 0x54, 0x78, 0x7C, 0x80, 0x84, 0x88, 0x8C, 0x90, 0x94, 0x58, 0x98] {
        fl(o);
    }
    let r = xi(rd::<i32>(a, 0x9C)).fmul_m(fa).fadd(xi(rd::<i32>(b, 0x9C)).fmul_m(fb));
    wr::<i32>(this, 0x9C, ftol32(r));
    wu(0xA0);
    for o in [0xA4, 0xA8, 0x68, 0x6C, 0x70, 0x74] {
        fl(o);
    }
}

/// `__init_CTimeCycle__m_BelowHorizonGrey` (0x84E7B0, CRT initialiser): `m_BelowHorizonGrey =
/// CRGBA(70, 70, 70, 255)`, `atexit(__dtor_CTimeCycle__m_BelowHorizonGrey)`.
#[hook(addr = 0x84E7B0, conv = cdecl, name = "__init_CTimeCycle__m_BelowHorizonGrey", expect = "68 FF 00 00 00")]
pub fn init_below_horizon_grey() {
    rgba_ctor(BELOW_HORIZON_GREY.ptr().cast(), 0x46, 0x46, 0x46, 0xFF);
    atexit(0x8564B0);
}

/// `__dtor_CTimeCycle__m_BelowHorizonGrey` (0x8564B0): a single `ret` (not hookable).
pub fn dtor_below_horizon_grey() {}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_GET_AMBIENT_RED,
    HOOK_GET_AMBIENT_GREEN,
    HOOK_GET_AMBIENT_BLUE,
    HOOK_GET_AMBIENT_RED_OBJ,
    HOOK_GET_AMBIENT_GREEN_OBJ,
    HOOK_GET_AMBIENT_BLUE_OBJ,
    HOOK_GET_AMBIENT_RED_BEFORE_BRIGHTNESS,
    HOOK_GET_AMBIENT_GREEN_BEFORE_BRIGHTNESS,
    HOOK_GET_AMBIENT_BLUE_BEFORE_BRIGHTNESS,
    HOOK_START_EXTRA_COLOUR,
    HOOK_STOP_EXTRA_COLOUR,
    HOOK_INIT_FOR_RESTART,
    HOOK_ADD_ONE,
    HOOK_FIND_TIME_CYCLE_BOX,
    HOOK_SET_CONSTANT_PARAMETERS_FOR_POST_FX,
    HOOK_CALC_COLOURS_FOR_POINT_PORT,
    HOOK_INITIALISE,
    HOOK_FIND_FAR_CLIP_FOR_COORS,
    HOOK_UPDATE,
    HOOK_COLOUR_SET_CTOR,
    HOOK_COLOUR_SET_INTERPOLATE,
    HOOK_INIT_BELOW_HORIZON_GREY,
];
