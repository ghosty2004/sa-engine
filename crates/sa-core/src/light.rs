//! CORE ports: `app_light` (the world lights: one ambient, one directional, six extra directionals) and
//! `GetDayNightBalance`. See `docs/arch/core-port.md`.
//!
//! Ported from the 1.0 US bytes; gta-reversed `source/app/app_light.cpp` is a reading aid. Every
//! float operation is the original's x87 sequence under PC=24 ([`X87`]); RenderWare calls go through
//! [`orig!`]. `RpLightSetFlags` is inlined in the original as a byte store to `light + 2`.
//!
//! Original UB kept visible: several functions pass a stack `RwRGBAReal` whose `alpha` word (and in
//! `SetLightsWithTimeOfDayColour` an `RwMatrix` whose `flags`, pads and `pos`) the original never
//! initialises; the port passes 0 there (recorded per function in `docs/arch/core-port.md`).

use crate::mem::{GBool, GPtr, GameData};
use crate::{globals, hook, orig};
use sa_math::x87::X87;

/// Opaque RenderWare light (`RpLight`): only its `flags` byte (+2) and frame pointer (+4) are touched.
pub enum RpLight {}
/// Opaque `RpWorld`.
pub enum RpWorld {}
/// Opaque `RwFrame` (its modelling matrix is at +0x10).
pub enum RwFrame {}

/// `RwRGBAReal`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RwRGBAReal {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

/// `RwV3d` / `CVector`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RwV3d {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// `RwMatrix` (0x40 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RwMatrix {
    pub right: RwV3d,
    pub flags: u32,
    pub up: RwV3d,
    pub pad1: u32,
    pub at: RwV3d,
    pub pad2: u32,
    pub pos: RwV3d,
    pub pad3: u32,
}

// SAFETY: plain floats and integers.
#[allow(unsafe_code)]
unsafe impl GameData for RwRGBAReal {}
// SAFETY: plain floats.
#[allow(unsafe_code)]
unsafe impl GameData for RwV3d {}
// SAFETY: plain floats and integers.
#[allow(unsafe_code)]
unsafe impl GameData for RwMatrix {}

const _: () = {
    assert!(core::mem::size_of::<RwRGBAReal>() == 0x10);
    assert!(core::mem::size_of::<RwMatrix>() == 0x40);
};

const C0: RwRGBAReal = RwRGBAReal {
    red: 0.0,
    green: 0.0,
    blue: 0.0,
    alpha: 0.0,
};

/// `FullLight` (0x8D60C0, `{1,1,1,1}`) followed by the road-group indicator tables R (0x8D60D0), G
/// (0x8D60D8), B (0x8D60E0). One block because `SetAmbientColoursToIndicateRoadGroup` indexes the
/// tables with a signed `idx % 7` that reaches back into `FullLight`.
pub const ROAD_GROUP_BLOCK_INIT: [u8; 40] = [
    0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x80, 0x3F, // FullLight
    0x00, 0xFF, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, // R
    0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, // G
    0x00, 0x00, 0x00, 0xFF, 0x00, 0xFF, 0xFF, 0x00, // B
];

globals! {
    list GLOBALS;
    /// `DirectAmbientLight[0]` (saved directional colour, infrared vision)
    pub static DIRECT_AMBIENT_LIGHT_0 @ 0xC8865C: RwRGBAReal = C0;
    /// `DirectAmbientLight[1]` (saved ambient colour, infrared vision)
    pub static DIRECT_AMBIENT_LIGHT_1 @ 0xC8866C: RwRGBAReal = C0;
    /// `LightStrengths[6]`
    pub static LIGHT_STRENGTHS @ 0xC8867C: [f32; 6] = [0.0; 6];
    /// `DirectionalLightColour`
    pub static DIRECTIONAL_LIGHT_COLOUR @ 0xC88694: RwRGBAReal = C0;
    /// `AmbientLightColour`
    pub static AMBIENT_LIGHT_COLOUR @ 0xC886A4: RwRGBAReal = C0;
    /// `DirectionalLightColourForFrame`
    pub static DIRECTIONAL_LIGHT_COLOUR_FOR_FRAME @ 0xC886B4: RwRGBAReal = C0;
    /// `AmbientLightColourForFrame_PedsCarsAndObjects`
    pub static AMBIENT_FOR_FRAME_PCO @ 0xC886C4: RwRGBAReal = C0;
    /// `AmbientLightColourForFrame`
    pub static AMBIENT_LIGHT_COLOUR_FOR_FRAME @ 0xC886D4: RwRGBAReal = C0;
    /// `pAmbient`
    pub static P_AMBIENT @ 0xC886E8: GPtr<RpLight> = GPtr::NULL;
    /// `pDirect`
    pub static P_DIRECT @ 0xC886EC: GPtr<RpLight> = GPtr::NULL;
    /// `pExtraDirectionals[6]`
    pub static P_EXTRA_DIRECTIONALS @ 0xC886F0: [GPtr<RpLight>; 6] = [GPtr::NULL; 6];
    /// `NumExtraDirectionalLights`
    pub static NUM_EXTRA_DIRECTIONAL_LIGHTS @ 0xC88708: i32 = 0;
    /// `FullLight` + road-group indicator tables (see [`ROAD_GROUP_BLOCK_INIT`])
    pub static ROAD_GROUP_BLOCK @ 0x8D60C0: [u8; 40] = ROAD_GROUP_BLOCK_INIT;
    /// `CPostEffects::m_fInfraredVisionHeatObjectCol`
    pub static IR_HEAT_OBJECT_COL @ 0x8D50BC: RwRGBAReal = RwRGBAReal { red: 1.0, green: 0.0, blue: 0.0, alpha: 1.0 };
    /// `CCoronas::LightsMult`
    pub static CORONAS_LIGHTS_MULT @ 0x8D4B5C: f32 = 1.0;
    /// `CTimeCycle::m_BrightnessAddedToAmbientBlue`
    pub static TC_BRIGHTNESS_ADDED_BLUE @ 0xB79E30: f32 = 0.0;
    /// `CTimeCycle::m_BrightnessAddedToAmbientGreen`
    pub static TC_BRIGHTNESS_ADDED_GREEN @ 0xB79E34: f32 = 0.0;
    /// `CTimeCycle::m_BrightnessAddedToAmbientRed`
    pub static TC_BRIGHTNESS_ADDED_RED @ 0xB79E38: f32 = 0.0;
    /// `CTimeCycle::m_CurrentColours.m_fIllumination`
    pub static TC_ILLUMINATION @ 0xB7C544: f32 = 0.0;
    /// `CTimeCycle::m_vecDirnLightToSun`
    pub static TC_DIRN_LIGHT_TO_SUN @ 0xB7CB14: RwV3d = RwV3d { x: 0.0, y: 0.0, z: 0.0 };
    /// `CWeather::LightningFlash`
    pub static WEATHER_LIGHTNING_FLASH @ 0xC812CC: GBool = GBool::FALSE;
    /// `CClock::ms_nGameClockSeconds`
    pub static CLOCK_SECONDS @ 0xB70150: u16 = 0;
    /// `CClock::ms_nGameClockMinutes`
    pub static CLOCK_MINUTES @ 0xB70152: u8 = 0;
    /// `CClock::ms_nGameClockHours`
    pub static CLOCK_HOURS @ 0xB70153: u8 = 0;
}

/// `.rdata` constants (f32 bits).
pub mod k {
    /// 0x858F34: 0.75 (`SetFlashyColours`)
    pub const FLASHY: u32 = 0x3F40_0000;
    /// 0x858C20: 0.9 (`SetFlashyColours_Mild`)
    pub const FLASHY_MILD: u32 = 0x3F66_6666;
    /// 0x859A3C: 1/255
    pub const INV_255: u32 = 0x3B80_8081;
    /// 0x86C2F4: 0.99609375
    pub const ILLUMINATION_SCALE: u32 = 0x3F7F_0000;
    /// 0x859044: 1/60
    pub const INV_60: u32 = 0x3C88_8889;
    /// 0x859E2C: 360.0
    pub const M360: u32 = 0x43B4_0000;
    /// 0x86BE9C: 420.0
    pub const M420: u32 = 0x43D2_0000;
    /// 0x86BE98: 1200.0
    pub const M1200: u32 = 0x4496_0000;
    /// 0x86BE94: 1260.0
    pub const M1260: u32 = 0x449D_8000;
}

fn kf(bits: u32) -> f32 {
    f32::from_bits(bits)
}

orig! {
    /// `RpLightSetColor` (0x751A90)
    pub fn rp_light_set_color(light: GPtr<RpLight>, color: GPtr<RwRGBAReal>) -> GPtr<RpLight> = cdecl 0x751A90 => unported;
    /// `RpLightCreate` (0x752110)
    pub fn rp_light_create(kind: i32) -> GPtr<RpLight> = cdecl 0x752110 => unported;
    /// `RpLightSetRadius` (0x751A70)
    pub fn rp_light_set_radius(light: GPtr<RpLight>, radius: f32) -> GPtr<RpLight> = cdecl 0x751A70 => unported;
    /// `RpLightDestroy` (0x7520D0)
    pub fn rp_light_destroy(light: GPtr<RpLight>) -> i32 = cdecl 0x7520D0 => unported;
    /// `RpWorldAddLight` (0x751910)
    pub fn rp_world_add_light(world: GPtr<RpWorld>, light: GPtr<RpLight>) -> GPtr<RpWorld> = cdecl 0x751910 => unported;
    /// `RpWorldRemoveLight` (0x751960)
    pub fn rp_world_remove_light(world: GPtr<RpWorld>, light: GPtr<RpLight>) -> GPtr<RpWorld> = cdecl 0x751960 => unported;
    /// `RwFrameCreate` (0x7F0410)
    pub fn rw_frame_create() -> GPtr<RwFrame> = cdecl 0x7F0410 => unported;
    /// `RwFrameDestroy` (0x7F05A0)
    pub fn rw_frame_destroy(frame: GPtr<RwFrame>) -> i32 = cdecl 0x7F05A0 => unported;
    /// `_rwObjectHasFrameSetFrame` (0x804EF0)
    pub fn rw_object_has_frame_set_frame(object: GPtr<RpLight>, frame: GPtr<RwFrame>) = cdecl 0x804EF0 => unported;
    /// `RwFrameRotate` (0x7F1010)
    pub fn rw_frame_rotate(frame: GPtr<RwFrame>, axis: GPtr<RwV3d>, angle: f32, combine: i32) -> GPtr<RwFrame> = cdecl 0x7F1010 => unported;
    /// `RwFrameTransform` (0x7F0F70)
    pub fn rw_frame_transform(frame: GPtr<RwFrame>, m: GPtr<RwMatrix>, combine: i32) -> GPtr<RwFrame> = cdecl 0x7F0F70 => unported;
    /// `RwFrameUpdateObjects` (0x7F0910)
    pub fn rw_frame_update_objects(frame: GPtr<RwFrame>) -> GPtr<RwFrame> = cdecl 0x7F0910 => unported;
    /// `RwMatrixUpdate` (0x7F18A0)
    pub fn rw_matrix_update(m: GPtr<RwMatrix>) -> GPtr<RwMatrix> = cdecl 0x7F18A0 => unported;
    /// `CVector::CrossProduct(out, a, b)` (0x59C730): `*out = a x b`
    pub fn cross_product(out: GPtr<RwV3d>, a: GPtr<RwV3d>, b: GPtr<RwV3d>) -> GPtr<RwV3d> = cdecl 0x59C730 => unported;
    /// `CVector::Normalise` (0x59C910)
    pub fn vector_normalise(v: GPtr<RwV3d>) = thiscall 0x59C910 => unported;
    /// `CTimeCycle::GetAmbientRed` (0x560330, ST0)
    pub fn tc_get_ambient_red() -> f64 = cdecl 0x560330 => unported;
    /// `CTimeCycle::GetAmbientGreen` (0x560340, ST0)
    pub fn tc_get_ambient_green() -> f64 = cdecl 0x560340 => unported;
    /// `CTimeCycle::GetAmbientBlue` (0x560350, ST0)
    pub fn tc_get_ambient_blue() -> f64 = cdecl 0x560350 => unported;
    /// `CTimeCycle::GetAmbientRed_Obj` (0x560360, ST0)
    pub fn tc_get_ambient_red_obj() -> f64 = cdecl 0x560360 => unported;
    /// `CTimeCycle::GetAmbientGreen_Obj` (0x560370, ST0)
    pub fn tc_get_ambient_green_obj() -> f64 = cdecl 0x560370 => unported;
    /// `CTimeCycle::GetAmbientBlue_Obj` (0x560380, ST0)
    pub fn tc_get_ambient_blue_obj() -> f64 = cdecl 0x560380 => unported;
    /// `Light::SetAmbientAndDirectionalColours` (0x735A20)
    pub fn call_set_ambient_and_directional_colours(mult: f32) = cdecl 0x735A20 => set_ambient_and_directional_colours;
}

/// `light->flags = v` (`RpLightSetFlags`, inlined by the original as `mov byte [light+2], v`).
fn set_light_flags(light: GPtr<RpLight>, v: u8) {
    light.cast::<u8>().byte_add(2).write(v)
}

/// `RpLightGetFrame(light)`: `light->object.parent` at +4.
fn light_frame(light: GPtr<RpLight>) -> GPtr<RwFrame> {
    light.cast::<GPtr<RwFrame>>().byte_add(4).read()
}

fn set_color(light: GPtr<RpLight>, c: GPtr<RwRGBAReal>) {
    let _ = rp_light_set_color(light, c);
}

fn set_color_local(light: GPtr<RpLight>, mut c: RwRGBAReal) {
    set_color(light, GPtr::from_raw(&raw mut c));
}

/// `fld a; fmul m32 b; fstp m32` under PC=24.
fn mul(a: f32, b: f32) -> f32 {
    X87::from_f32(a).fmul_m(b).to_f32()
}

/// `fld v; fchs; fstp m32` (an SNaN is quieted by the load).
fn neg(v: f32) -> f32 {
    X87::from_f32(v).fchs().to_f32()
}

// ------------------------------------------------------------------------------------ hooks ---

/// `ActivateDirectional` (0x735C80): `pDirect->flags = 1`.
#[hook(addr = 0x735C80, conv = cdecl, name = "Light::ActivateDirectional", expect = "A1 EC 86 C8 00")]
pub fn activate_directional() {
    set_light_flags(P_DIRECT.get(), 1)
}

/// `DeActivateDirectional` (0x735C70): `jmp` into an obfuscation chunk at 0x405BCF that computes
/// `*(dword[0x15607FC] + dword[0x6D1E24] + CF=1)` = `*0xC886EC` = `pDirect`, then `flags = 0`.
#[hook(addr = 0x735C70, conv = cdecl, name = "Light::DeActivateDirectional", expect = "E9 5A FF CC FF C6 40 02 00 C3")]
pub fn deactivate_directional() {
    set_light_flags(P_DIRECT.get(), 0)
}

/// `LightsCreate(world)` (0x5BA520). Returns `world` (EAX; gta-reversed: void). Colours are stack
/// `RwRGBAReal`s whose `alpha` is never written (original UB: port passes 0).
#[hook(addr = 0x5BA520, conv = cdecl, name = "Light::LightsCreate", expect = "83 EC 1C 57 8B 7C 24 24")]
pub fn lights_create(world: GPtr<RpWorld>) -> GPtr<RpWorld> {
    if world.is_null() {
        return GPtr::NULL;
    }
    let amb = rp_light_create(2);
    P_AMBIENT.set(amb);
    set_light_flags(amb, 1);
    set_color_local(P_AMBIENT.get(), RwRGBAReal { red: 0.25, green: 0.25, blue: f32::from_bits(0x3E4C_CCCD), alpha: 0.0 });
    let dir = rp_light_create(1);
    P_DIRECT.set(dir);
    set_light_flags(dir, 1);
    set_color_local(
        P_DIRECT.get(),
        RwRGBAReal { red: 1.0, green: f32::from_bits(0x3F59_999A), blue: f32::from_bits(0x3EE6_6666), alpha: 0.0 },
    );
    let _ = rp_light_set_radius(P_DIRECT.get(), 2.0);
    let frame = rw_frame_create();
    rw_object_has_frame_set_frame(P_DIRECT.get(), frame);
    let mut axis = RwV3d { x: 1.0, y: 1.0, z: 0.0 };
    let _ = rw_frame_rotate(frame, GPtr::from_raw(&raw mut axis), 160.0, 1);
    let _ = rp_world_add_light(world, P_AMBIENT.get());
    let _ = rp_world_add_light(world, P_DIRECT.get());
    let extra = P_EXTRA_DIRECTIONALS.ptr().cast::<GPtr<RpLight>>();
    for i in 0..6 {
        let slot = extra.wrapping_index(i);
        let l = rp_light_create(1);
        slot.write(l);
        set_light_flags(l, 0);
        set_color_local(slot.read(), RwRGBAReal { red: 1.0, green: 0.5, blue: 0.0, alpha: 0.0 });
        let _ = rp_light_set_radius(slot.read(), 2.0);
        let f = rw_frame_create();
        rw_object_has_frame_set_frame(slot.read(), f);
        let _ = rp_world_add_light(world, slot.read());
    }
    world
}

/// `LightsDestroy(world)` (0x735730). Returns `world` (EAX; gta-reversed: void). The ambient light's
/// frame is not destroyed (as in the original).
#[hook(addr = 0x735730, conv = cdecl, name = "Light::LightsDestroy", expect = "57 8B 7C 24 08 85 FF")]
pub fn lights_destroy(world: GPtr<RpWorld>) -> GPtr<RpWorld> {
    if world.is_null() {
        return world;
    }
    if !P_AMBIENT.get().is_null() {
        let _ = rp_world_remove_light(world, P_AMBIENT.get());
        let _ = rp_light_destroy(P_AMBIENT.get());
        P_AMBIENT.set(GPtr::NULL);
    }
    if !P_DIRECT.get().is_null() {
        let _ = rp_world_remove_light(world, P_DIRECT.get());
        let _ = rw_frame_destroy(light_frame(P_DIRECT.get()));
        let _ = rp_light_destroy(P_DIRECT.get());
        P_DIRECT.set(GPtr::NULL);
    }
    let extra = P_EXTRA_DIRECTIONALS.ptr().cast::<GPtr<RpLight>>();
    for i in 0..6 {
        let slot = extra.wrapping_index(i);
        if !slot.read().is_null() {
            let _ = rp_world_remove_light(world, slot.read());
            let _ = rw_frame_destroy(light_frame(slot.read()));
            let _ = rp_light_destroy(slot.read());
            slot.write(GPtr::NULL);
        }
    }
    world
}

/// `LightsEnable(int)` (0x735720): a lone `ret`; not hookable (5-byte patch would overrun).
pub fn lights_enable(_enable: i32) {}

/// `SetLightsWithTimeOfDayColour(world)` (0x7354E0). The directional frame's matrix is a stack
/// `RwMatrix` with `right = (Z x sun).Normalise() x sun`, `up = (Z x sun).Normalise()`, `at = -sun`;
/// its `flags`, pads and `pos` are uninitialised in the original (port: 0).
#[hook(addr = 0x7354E0, conv = cdecl, name = "Light::SetLightsWithTimeOfDayColour", expect = "A1 E8 86 C8 00 83 EC 74")]
pub fn set_lights_with_time_of_day_colour(_world: GPtr<RpWorld>) {
    if !P_AMBIENT.get().is_null() {
        let getters: [fn() -> f64; 6] = [
            tc_get_ambient_red,
            tc_get_ambient_green,
            tc_get_ambient_blue,
            tc_get_ambient_red_obj,
            tc_get_ambient_green_obj,
            tc_get_ambient_blue_obj,
        ];
        let dst: [GPtr<f32>; 6] = [
            AMBIENT_LIGHT_COLOUR_FOR_FRAME.ptr().cast(),
            AMBIENT_LIGHT_COLOUR_FOR_FRAME.ptr().cast::<f32>().wrapping_index(1),
            AMBIENT_LIGHT_COLOUR_FOR_FRAME.ptr().cast::<f32>().wrapping_index(2),
            AMBIENT_FOR_FRAME_PCO.ptr().cast(),
            AMBIENT_FOR_FRAME_PCO.ptr().cast::<f32>().wrapping_index(1),
            AMBIENT_FOR_FRAME_PCO.ptr().cast::<f32>().wrapping_index(2),
        ];
        for (g, d) in getters.iter().zip(dst) {
            // `mov [esp], LightsMult` before the call, `fmul dword [esp]` after it
            let mult = CORONAS_LIGHTS_MULT.get();
            d.write(X87::from_f64_exact(g()).fmul_m(mult).to_f32());
        }
        if WEATHER_LIGHTNING_FLASH.get().get() {
            for d in dst {
                d.write(1.0);
            }
        }
        set_color(P_AMBIENT.get(), AMBIENT_LIGHT_COLOUR_FOR_FRAME.ptr());
    }
    let dir = P_DIRECT.get();
    if dir.is_null() {
        return;
    }
    let c = X87::from_f32(TC_ILLUMINATION.get())
        .fmul_m(kf(k::ILLUMINATION_SCALE))
        .fmul_m(CORONAS_LIGHTS_MULT.get())
        .to_f32();
    let p = DIRECTIONAL_LIGHT_COLOUR_FOR_FRAME.ptr().cast::<f32>();
    p.write(c);
    p.wrapping_index(1).write(c);
    p.wrapping_index(2).write(c);
    set_color(dir, DIRECTIONAL_LIGHT_COLOUR_FOR_FRAME.ptr());

    let mut sun = TC_DIRN_LIGHT_TO_SUN.get();
    let mut z = RwV3d { x: 0.0, y: 0.0, z: 1.0 };
    let mut tmp = RwV3d::default();
    let r = cross_product(GPtr::from_raw(&raw mut tmp), GPtr::from_raw(&raw mut z), GPtr::from_raw(&raw mut sun)).read();
    let mut right = r;
    vector_normalise(GPtr::from_raw(&raw mut right));
    let u = cross_product(GPtr::from_raw(&raw mut tmp), GPtr::from_raw(&raw mut right), GPtr::from_raw(&raw mut sun)).read();
    let mut m = RwMatrix {
        right: u,
        up: right,
        at: RwV3d { x: neg(sun.x), y: neg(sun.y), z: neg(sun.z) },
        ..RwMatrix::default()
    };
    let _ = rw_frame_transform(light_frame(P_DIRECT.get()), GPtr::from_raw(&raw mut m), 0);
}

/// `WorldReplaceNormalLightsWithScorched(world, lighting)` (0x7357E0): ambient = `{l, l, l, ?}`
/// (alpha uninitialised in the original; port 0), then `pDirect->flags = 0` (inlined).
#[hook(addr = 0x7357E0, conv = cdecl, name = "Light::WorldReplaceNormalLightsWithScorched", expect = "83 EC 10 8B 44 24 18")]
pub fn world_replace_normal_lights_with_scorched(_world: GPtr<RpWorld>, lighting_bits: u32) {
    // copied with integer moves: the bits are kept (no x87 load)
    let l = f32::from_bits(lighting_bits);
    set_color_local(P_AMBIENT.get(), RwRGBAReal { red: l, green: l, blue: l, alpha: 0.0 });
    set_light_flags(P_DIRECT.get(), 0);
}

/// `WorldReplaceScorchedLightsWithNormal(world)` (0x735820).
#[hook(addr = 0x735820, conv = cdecl, name = "Light::WorldReplaceScorchedLightsWithNormal", expect = "A1 E8 86 C8 00")]
pub fn world_replace_scorched_lights_with_normal(_world: GPtr<RpWorld>) {
    set_color(P_AMBIENT.get(), AMBIENT_LIGHT_COLOUR_FOR_FRAME.ptr());
    set_light_flags(P_DIRECT.get(), 1);
}

/// Slot choice of `AddAnExtraDirectionalLight` (pure): `strength` = max of the colours with the
/// original's compare order; the slot is `num` while `num < limit`, else the index of the weakest
/// light weaker than `strength`, or none (gta-reversed starts that search at 0, the original at -1:
/// a light not stronger than every... weaker one is dropped).
pub fn extra_light_slot(red: f32, green: f32, blue: f32, num: i32, limit: i32, strengths: &[f32; 6]) -> (f32, Option<usize>) {
    // fld red; fcomp green; test ah,41h; jne -> green (also unordered)
    let m = if red > green { red } else { green };
    // fcom blue; test ah,41h; jne -> blue (copied by integer move)
    let strength = if m > blue { X87::from_f32(m).to_f32() } else { blue };
    if num < limit {
        return (strength, Some(num as usize));
    }
    let mut best = strength;
    let mut slot: i32 = -1;
    for (i, s) in strengths.iter().enumerate().take(limit.max(0) as usize) {
        if best > *s {
            best = *s;
            slot = i as i32;
        }
    }
    (strength, (slot >= 0).then_some(slot as usize))
}

/// `AddAnExtraDirectionalLight(world, x, y, z, r, g, b)` (0x735840). `CGame::CanSeeOutSideFromCurrArea`
/// is inlined (`currArea != 0` -> 6 lights, else 4). Original: the weakest-slot search starts at -1,
/// so a light weaker than all six is dropped (gta-reversed: replaces slot 0). The colour's alpha is
/// uninitialised (port 0).
#[hook(addr = 0x735840, conv = cdecl, name = "Light::AddAnExtraDirectionalLight", expect = "D9 44 24 14 83 EC 14")]
pub fn add_an_extra_directional_light(_world: GPtr<RpWorld>, x: f32, y: f32, z: f32, red: f32, green: f32, blue: f32) {
    let limit = if crate::game::CURR_AREA.get() != 0 { 6 } else { 4 };
    let num = NUM_EXTRA_DIRECTIONAL_LIGHTS.get();
    let (strength, slot) = extra_light_slot(red, green, blue, num, limit, &LIGHT_STRENGTHS.get());
    let Some(slot) = slot else { return };
    let lights = P_EXTRA_DIRECTIONALS.ptr().cast::<GPtr<RpLight>>();
    // colour words copied with integer moves (bits kept)
    set_color_local(lights.wrapping_index(slot as i32).read(), RwRGBAReal { red, green, blue, alpha: 0.0 });
    let frame = light_frame(lights.wrapping_index(slot as i32).read());
    let at = frame.cast::<f32>().byte_add(0x30);
    at.write(neg(x));
    at.wrapping_index(1).write(neg(y));
    at.wrapping_index(2).write(neg(z));
    let _ = rw_matrix_update(frame.cast::<RwMatrix>().byte_add(0x10));
    let _ = rw_frame_update_objects(frame);
    set_light_flags(lights.wrapping_index(slot as i32).read(), 1);
    LIGHT_STRENGTHS.ptr().cast::<f32>().wrapping_index(slot as i32).write(X87::from_f32(strength).to_f32());
    let n = NUM_EXTRA_DIRECTIONAL_LIGHTS.get().wrapping_add(1);
    NUM_EXTRA_DIRECTIONAL_LIGHTS.set(if n < limit { n } else { limit });
}

/// `RemoveExtraDirectionalLights(world)` (0x7359E0): flags of all six = 0, count = 0.
#[hook(addr = 0x7359E0, conv = cdecl, name = "Light::RemoveExtraDirectionalLights", expect = "8B 0D F0 86 C8 00")]
pub fn remove_extra_directional_lights(_world: GPtr<RpWorld>) {
    for l in P_EXTRA_DIRECTIONALS.get() {
        set_light_flags(l, 0);
    }
    NUM_EXTRA_DIRECTIONAL_LIGHTS.set(0);
}

fn set_amb_dir(amb: [f32; 3], dir: [f32; 3]) {
    let a = AMBIENT_LIGHT_COLOUR.ptr().cast::<f32>();
    let d = DIRECTIONAL_LIGHT_COLOUR.ptr().cast::<f32>();
    for i in 0..3 {
        a.wrapping_index(i).write(amb[i as usize]);
        d.wrapping_index(i).write(dir[i as usize]);
    }
}

fn rgb(c: RwRGBAReal) -> [f32; 3] {
    [c.red, c.green, c.blue]
}

/// `SetAmbientAndDirectionalColours(mult)` (0x735A20): `colour = forFrame * mult` (x87, PC=24).
#[hook(addr = 0x735A20, conv = cdecl, name = "Light::SetAmbientAndDirectionalColours", expect = "D9 05 D4 86 C8 00")]
pub fn set_ambient_and_directional_colours(mult: f32) {
    let a = rgb(AMBIENT_LIGHT_COLOUR_FOR_FRAME.get()).map(|v| mul(v, mult));
    let d = rgb(DIRECTIONAL_LIGHT_COLOUR_FOR_FRAME.get()).map(|v| mul(v, mult));
    set_amb_dir(a, d);
    set_color(P_AMBIENT.get(), AMBIENT_LIGHT_COLOUR.ptr());
    set_color(P_DIRECT.get(), DIRECTIONAL_LIGHT_COLOUR.ptr());
}

/// `ReSetAmbientAndDirectionalColours()` (0x735C40).
#[hook(addr = 0x735C40, conv = cdecl, name = "Light::ReSetAmbientAndDirectionalColours", expect = "A1 E8 86 C8 00")]
pub fn reset_ambient_and_directional_colours() {
    set_color(P_AMBIENT.get(), AMBIENT_LIGHT_COLOUR_FOR_FRAME.ptr());
    set_color(P_DIRECT.get(), DIRECTIONAL_LIGHT_COLOUR_FOR_FRAME.ptr());
}

fn flashy(mult: f32, on: f32, factor: u32) {
    if crate::timer::TIME_MS.get() & 0x100 != 0 {
        // directional = forFrame rgb only (gta-reversed copies the alpha too)
        set_amb_dir([on; 3], rgb(DIRECTIONAL_LIGHT_COLOUR_FOR_FRAME.get()));
        set_color(P_AMBIENT.get(), AMBIENT_LIGHT_COLOUR.ptr());
        set_color(P_DIRECT.get(), DIRECTIONAL_LIGHT_COLOUR.ptr());
    } else {
        call_set_ambient_and_directional_colours(mul(mult, kf(factor)));
    }
}

/// `SetFlashyColours(mult)` (0x735AB0).
#[hook(addr = 0x735AB0, conv = cdecl, name = "Light::SetFlashyColours", expect = "A1 84 CB B7 00")]
pub fn set_flashy_colours(mult: f32) {
    flashy(mult, 1.0, k::FLASHY)
}

/// `SetFlashyColours_Mild(mult)` (0x735B40).
#[hook(addr = 0x735B40, conv = cdecl, name = "Light::SetFlashyColours_Mild", expect = "A1 84 CB B7 00")]
pub fn set_flashy_colours_mild(mult: f32) {
    flashy(mult, f32::from_bits(0x3F26_6666), k::FLASHY_MILD)
}

/// `SetBrightMarkerColours(lighting)` (0x735BD0): ambient 0.6, directional 1.0.
#[hook(addr = 0x735BD0, conv = cdecl, name = "Light::SetBrightMarkerColours", expect = "A1 E8 86 C8 00")]
pub fn set_bright_marker_colours(_lighting: f32) {
    set_amb_dir([f32::from_bits(0x3F19_999A); 3], [1.0; 3]);
    set_color(P_AMBIENT.get(), AMBIENT_LIGHT_COLOUR.ptr());
    set_color(P_DIRECT.get(), DIRECTIONAL_LIGHT_COLOUR.ptr());
}

/// Byte `idx % 7` (signed) of table `t` (0 = R, 1 = G, 2 = B) in [`ROAD_GROUP_BLOCK`].
pub fn road_group_offset(idx: i32, t: i32) -> isize {
    (0x10 + 8 * t + idx.wrapping_rem(7)) as isize
}

/// `SetAmbientColoursToIndicateRoadGroup(idx)` (0x735C90): the table index is `idx % 7` with a signed
/// remainder (gta-reversed: `idx & 8`); a negative idx reads before the R table (into `FullLight`).
#[hook(addr = 0x735C90, conv = cdecl, name = "Light::SetAmbientColoursToIndicateRoadGroup", expect = "8B 44 24 04 99")]
pub fn set_ambient_colours_to_indicate_road_group(idx: i32) {
    let base = ROAD_GROUP_BLOCK.ptr().cast::<u8>();
    let a = AMBIENT_LIGHT_COLOUR.ptr().cast::<f32>();
    for t in 0..3 {
        let b = base.byte_add(road_group_offset(idx, t)).read();
        a.wrapping_index(t).write(X87::from_i32(b as i32).fmul_m(kf(k::INV_255)).to_f32());
    }
    set_color(P_AMBIENT.get(), AMBIENT_LIGHT_COLOUR.ptr());
}

/// `SetFullAmbient()` (0x735D10).
#[hook(addr = 0x735D10, conv = cdecl, name = "Light::SetFullAmbient", expect = "A1 E8 86 C8 00")]
pub fn set_full_ambient() {
    set_color(P_AMBIENT.get(), ROAD_GROUP_BLOCK.ptr().cast());
}

/// `SetAmbientColours()` (0x735D30).
#[hook(addr = 0x735D30, conv = cdecl, name = "Light::SetAmbientColours-void", expect = "A1 E8 86 C8 00")]
pub fn set_ambient_colours() {
    set_color(P_AMBIENT.get(), AMBIENT_LIGHT_COLOUR_FOR_FRAME.ptr());
}

/// `SetAmbientColours(color)` (0x735D50).
#[hook(addr = 0x735D50, conv = cdecl, name = "Light::SetAmbientColours-color", expect = "8B 44 24 04 8B 0D E8 86 C8 00")]
pub fn set_ambient_colours_color(color: GPtr<RwRGBAReal>) {
    set_color(P_AMBIENT.get(), color);
}

/// `SetDirectionalColours(color)` (0x735D70).
#[hook(addr = 0x735D70, conv = cdecl, name = "Light::SetDirectionalColours", expect = "8B 44 24 04 8B 0D EC 86 C8 00")]
pub fn set_directional_colours(color: GPtr<RwRGBAReal>) {
    set_color(P_DIRECT.get(), color);
}

/// The colour arithmetic of `SetLightColoursForPedsCarsAndObjects` (pure): returns (ambient,
/// directional). Ambient red/green add the brightness to the unrounded PC=24 products still on the
/// x87 stack; blue adds it to the already stored f32. Each channel uses its own brightness global
/// (gta-reversed adds the red one to all three).
pub fn peds_cars_objects_colours(mult: f32, pco: [f32; 3], dir_ff: [f32; 3], add: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let r = X87::from_f32(pco[0]).fmul_m(mult);
    let g = X87::from_f32(pco[1]).fmul_m(mult);
    let b = X87::from_f32(pco[2]).fmul_m(mult).to_f32();
    let dir = dir_ff.map(|v| mul(v, mult));
    let ar = X87::from_f32(add[0]).fadd(r).to_f32();
    let ag = X87::from_f32(add[1]).fadd(g).to_f32();
    let ab = X87::from_f32(add[2]).fadd_m(b).to_f32();
    ([ar, ag, ab], dir)
}

/// `SetLightColoursForPedsCarsAndObjects(mult)` (0x735D90).
#[hook(addr = 0x735D90, conv = cdecl, name = "Light::SetLightColoursForPedsCarsAndObjects", expect = "D9 05 C4 86 C8 00")]
pub fn set_light_colours_for_peds_cars_and_objects(mult: f32) {
    let (a, d) = peds_cars_objects_colours(
        mult,
        rgb(AMBIENT_FOR_FRAME_PCO.get()),
        rgb(DIRECTIONAL_LIGHT_COLOUR_FOR_FRAME.get()),
        [TC_BRIGHTNESS_ADDED_RED.get(), TC_BRIGHTNESS_ADDED_GREEN.get(), TC_BRIGHTNESS_ADDED_BLUE.get()],
    );
    set_amb_dir(a, d);
    set_color(P_AMBIENT.get(), AMBIENT_LIGHT_COLOUR.ptr());
    set_color(P_DIRECT.get(), DIRECTIONAL_LIGHT_COLOUR.ptr());
}

/// `SetLightsForInfraredVisionHeatObjects()` (0x735E40).
#[hook(addr = 0x735E40, conv = cdecl, name = "Light::SetLightsForInfraredVisionHeatObjects", expect = "A1 E8 86 C8 00")]
pub fn set_lights_for_infrared_vision_heat_objects() {
    set_color(P_AMBIENT.get(), IR_HEAT_OBJECT_COL.ptr());
    set_color(P_DIRECT.get(), IR_HEAT_OBJECT_COL.ptr());
}

/// `StoreAndSetLightsForInfraredVisionHeatObjects()` (0x735E70): saves both colours (all four words),
/// then the body of `SetLightsForInfraredVisionHeatObjects` inlined.
#[hook(addr = 0x735E70, conv = cdecl, name = "Light::StoreAndSetLightsForInfraredVisionHeatObjects", expect = "8B 15 AC 86 C8 00")]
pub fn store_and_set_lights_for_infrared_vision_heat_objects() {
    DIRECT_AMBIENT_LIGHT_1.set(AMBIENT_LIGHT_COLOUR.get());
    DIRECT_AMBIENT_LIGHT_0.set(DIRECTIONAL_LIGHT_COLOUR.get());
    set_color(P_AMBIENT.get(), IR_HEAT_OBJECT_COL.ptr());
    set_color(P_DIRECT.get(), IR_HEAT_OBJECT_COL.ptr());
}

/// `RestoreLightsForInfraredVisionHeatObjects()` (0x735EF0).
#[hook(addr = 0x735EF0, conv = cdecl, name = "Light::RestoreLightsForInfraredVisionHeatObjects", expect = "A1 E8 86 C8 00")]
pub fn restore_lights_for_infrared_vision_heat_objects() {
    set_color(P_AMBIENT.get(), DIRECT_AMBIENT_LIGHT_1.ptr());
    set_color(P_DIRECT.get(), DIRECT_AMBIENT_LIGHT_0.ptr());
}

/// `SetLightsForInfraredVisionDefaultObjects()` (0x735F20): `{0, 0, 1, 1}` for both.
#[hook(addr = 0x735F20, conv = cdecl, name = "Light::SetLightsForInfraredVisionDefaultObjects", expect = "83 EC 10 8B 0D E8 86 C8 00")]
pub fn set_lights_for_infrared_vision_default_objects() {
    let mut c = RwRGBAReal { red: 0.0, green: 0.0, blue: 1.0, alpha: 1.0 };
    set_color(P_AMBIENT.get(), GPtr::from_raw(&raw mut c));
    set_color(P_DIRECT.get(), GPtr::from_raw(&raw mut c));
}

/// `SetLightsForNightVision()` (0x735F70): `{0, 1, 0, 1}` for both.
#[hook(addr = 0x735F70, conv = cdecl, name = "Light::SetLightsForNightVision", expect = "83 EC 10 8B 0D E8 86 C8 00")]
pub fn set_lights_for_night_vision() {
    let mut c = RwRGBAReal { red: 0.0, green: 1.0, blue: 0.0, alpha: 1.0 };
    set_color(P_AMBIENT.get(), GPtr::from_raw(&raw mut c));
    set_color(P_DIRECT.get(), GPtr::from_raw(&raw mut c));
}

/// `GetDayNightBalance` arithmetic (pure): `t = (float)(h*60 + m) + (float)s * (1/60)` (x87, PC=24),
/// then `<360: 1`, `<420: (420 - t) * (1/60)`, `<1200: 0`, `<1260: 1 - (1260 - t) * (1/60)`, else 1.
/// The original multiplies by the f32 constant 1/60 where gta-reversed divides by 60. NaN cannot occur
/// (integer inputs).
pub fn day_night_balance(hours: u8, minutes: u8, seconds: u16) -> X87 {
    let t = X87::from_i32(hours as i32 * 60 + minutes as i32)
        .fadd(X87::from_i32(seconds as i32).fmul_m(kf(k::INV_60)));
    // `fst dword [esp]`; later compares reload the f32
    let tf = t.to_f32();
    if t.fcom(X87::from_f32(kf(k::M360))).c0() {
        return X87::one();
    }
    let lt = |c: u32| X87::from_f32(tf).fcom(X87::from_f32(kf(c))).c0();
    if lt(k::M420) {
        return X87::from_f32(kf(k::M420)).fsub_m(tf).fmul_m(kf(k::INV_60));
    }
    if lt(k::M1200) {
        return X87::zero();
    }
    if lt(k::M1260) {
        return X87::from_f32(kf(k::M1260))
            .fsub_m(tf)
            .fmul_m(kf(k::INV_60))
            .fsubr_m(1.0);
    }
    X87::one()
}

/// `GetDayNightBalance()` (0x6FAB30, ST0). `CClock::GetMinutesToday` inlined.
#[hook(addr = 0x6FAB30, conv = cdecl, name = "Light::GetDayNightBalance", expect = "51 0F B6 05 53 01 B7 00")]
pub fn get_day_night_balance() -> X87 {
    day_night_balance(CLOCK_HOURS.get(), CLOCK_MINUTES.get(), CLOCK_SECONDS.get())
}

/// This module's registrations (collected by [`crate::hooks`]).
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_ACTIVATE_DIRECTIONAL,
    HOOK_DEACTIVATE_DIRECTIONAL,
    HOOK_LIGHTS_CREATE,
    HOOK_LIGHTS_DESTROY,
    HOOK_SET_LIGHTS_WITH_TIME_OF_DAY_COLOUR,
    HOOK_WORLD_REPLACE_NORMAL_LIGHTS_WITH_SCORCHED,
    HOOK_WORLD_REPLACE_SCORCHED_LIGHTS_WITH_NORMAL,
    HOOK_ADD_AN_EXTRA_DIRECTIONAL_LIGHT,
    HOOK_REMOVE_EXTRA_DIRECTIONAL_LIGHTS,
    HOOK_SET_AMBIENT_AND_DIRECTIONAL_COLOURS,
    HOOK_RESET_AMBIENT_AND_DIRECTIONAL_COLOURS,
    HOOK_SET_FLASHY_COLOURS,
    HOOK_SET_FLASHY_COLOURS_MILD,
    HOOK_SET_BRIGHT_MARKER_COLOURS,
    HOOK_SET_AMBIENT_COLOURS_TO_INDICATE_ROAD_GROUP,
    HOOK_SET_FULL_AMBIENT,
    HOOK_SET_AMBIENT_COLOURS,
    HOOK_SET_AMBIENT_COLOURS_COLOR,
    HOOK_SET_DIRECTIONAL_COLOURS,
    HOOK_SET_LIGHT_COLOURS_FOR_PEDS_CARS_AND_OBJECTS,
    HOOK_SET_LIGHTS_FOR_INFRARED_VISION_HEAT_OBJECTS,
    HOOK_STORE_AND_SET_LIGHTS_FOR_INFRARED_VISION_HEAT_OBJECTS,
    HOOK_RESTORE_LIGHTS_FOR_INFRARED_VISION_HEAT_OBJECTS,
    HOOK_SET_LIGHTS_FOR_INFRARED_VISION_DEFAULT_OBJECTS,
    HOOK_SET_LIGHTS_FOR_NIGHT_VISION,
    HOOK_GET_DAY_NIGHT_BALANCE,
];

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    #[test]
    fn day_night_balance_points() {
        assert_eq!(day_night_balance(0, 0, 0).to_f32(), 1.0);
        assert_eq!(day_night_balance(6, 30, 0).to_f32(), X87::from_f32(30.0).fmul_m(kf(k::INV_60)).to_f32());
        assert_eq!(day_night_balance(12, 0, 0).to_f32(), 0.0);
        let x = X87::from_f32(1260.0).fsub_m(1230.0).fmul_m(kf(k::INV_60)).fsubr_m(1.0);
        assert!(day_night_balance(20, 30, 0).same(x));
        assert_eq!(day_night_balance(23, 0, 0).to_f32(), 1.0);
    }

    #[test]
    fn extra_slot_search_starts_at_minus_one() {
        let s = [1.0, 0.5, 0.25, 2.0, 3.0, 4.0];
        // room left: the next slot
        assert_eq!(extra_light_slot(0.1, 0.2, 0.3, 2, 4, &s), (0.3, Some(2)));
        // full: weakest light weaker than the strength
        assert_eq!(extra_light_slot(0.6, 0.2, 0.3, 4, 4, &s), (0.6, Some(2)));
        // weaker than all: dropped (gta-reversed would take slot 0)
        assert_eq!(extra_light_slot(0.1, 0.2, 0.0, 4, 4, &s).1, None);
        // NaN red/green: falls to green / blue
        assert_eq!(extra_light_slot(f32::NAN, 0.7, 0.1, 0, 4, &s).0, 0.7);
    }

    #[test]
    fn road_group_index_is_a_signed_remainder() {
        assert_eq!(road_group_offset(9, 0), 0x12);
        assert_eq!(road_group_offset(-1, 0), 0x0F);
        assert_eq!(road_group_offset(-6, 2), 0x20 - 6);
        assert_eq!(road_group_offset(i32::MIN, 1), (0x18 + (i32::MIN % 7)) as isize);
    }

    #[test]
    fn peds_colours_use_three_brightness_globals() {
        let (a, d) = peds_cars_objects_colours(0.5, [0.2, 0.4, 0.6], [1.0, 0.5, 0.25], [0.1, 0.2, 0.3]);
        assert_eq!(d, [0.5, 0.25, 0.125]);
        assert_eq!(a[0], X87::from_f32(0.1).fadd(X87::from_f32(0.2).fmul_m(0.5)).to_f32());
        assert_eq!(a[2], X87::from_f32(0.3).fadd_m(0.3).to_f32());
    }
}
