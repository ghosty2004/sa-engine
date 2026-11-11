//! PORT-RENDER: `CWeather` (catalogue subsystem RENDER rows): weather state globals, the weather-type
//! lists and regions, forcing / releasing a weather. Ported from the 1.0 US bytes; gta-reversed
//! `Weather.cpp` and `docs/re/specs/RENDER/CWeather*.md` as reading aids.
//!
//! `NewWeatherType` / `OldWeatherType` are declared by sa-stream and `WetRoads` by sa-world
//! (re-exported here); `LightningFlash` by sa-core.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_world::world::common::{CAM_ENTITY, CAM_VECTOR, find_player_coors};

use crate::common::{k, x};

pub use sa_stream::modelinfo::vehicle::{NEW_WEATHER_TYPE, OLD_WEATHER_TYPE};
pub use sa_world::collision::surface::WET_ROADS;

sa_core::globals! {
    list GLOBALS;
    /// `CWeather::bScriptsForceRain`
    pub static SCRIPTS_FORCE_RAIN @ 0xC812AC: u8 = 0;
    /// `CWeather::CurrentRainParticleStrength`
    pub static CURRENT_RAIN_PARTICLE_STRENGTH @ 0xC812B0: u32 = 0;
    /// `CWeather::LightningStartY`
    pub static LIGHTNING_START_Y @ 0xC812B4: u32 = 0;
    /// `CWeather::LightningStartX`
    pub static LIGHTNING_START_X @ 0xC812B8: u32 = 0;
    /// `CWeather::WhenToPlayLightningSound`
    pub static WHEN_TO_PLAY_LIGHTNING_SOUND @ 0xC812C0: u32 = 0;
    /// `CWeather::Wind`
    pub static WIND @ 0xC812F0: f32 = 0.0;
    /// `CWeather::Sandstorm`
    pub static SANDSTORM @ 0xC812F4: f32 = 0.0;
    /// `CWeather::Foggyness_SF`
    pub static FOGGYNESS_SF @ 0xC812FC: f32 = 0.0;
    /// `CWeather::InterpolationValue`
    pub static INTERPOLATION_VALUE @ 0xC8130C: f32 = 0.0;
    /// `CWeather::WeatherTypeInList`
    pub static WEATHER_TYPE_IN_LIST @ 0xC81310: i32 = 0;
    /// `CWeather::WeatherRegion` (i16)
    pub static WEATHER_REGION @ 0xC81314: i16 = 0;
    /// `CWeather::ForcedWeatherType` (i16)
    pub static FORCED_WEATHER_TYPE @ 0xC81318: i16 = 0;
    /// `CWeather::Rain`
    pub static RAIN @ 0xC81324: f32 = 0.0;
    /// `CWeather::InTunnelness`
    pub static IN_TUNNELNESS @ 0xC81334: f32 = 0.0;
}

/// `CWeather::StreamAfterRainTimer` (0x8D5EAC, initial value 800 in `.data`; owner: CWeather, written
/// by `Init`).
const STREAM_AFTER_RAIN_TIMER: usize = 0x8D5EAC;
/// The weather-type lists (64 i8 each): default 0x8D5EB0, then LA, SF, LV, desert at +0x40 steps.
const WEATHER_LISTS: usize = 0x8D5EB0;

/// 0.2 (0x858CC4) and the region bounds: 1000 (0x858C4C), 910 (0x872A8C), -850 (0x872A88), 1280
/// (0x872A84), -1430 (0x872A80), -580 (0x872A7C), 1430 (0x872A78), 250 (0x859F80), 3000 (0x859A94),
/// -3000 (0x859A90)
const P02: u32 = 0x3E4C_CCCD;
const B1000: u32 = 0x447A_0000;
const B910: u32 = 0x4463_8000;
const BM850: u32 = 0xC454_8000;
const B1280: u32 = 0x44A0_0000;
const BM1430: u32 = 0xC4B2_C000;
const BM580: u32 = 0xC411_0000;
const B1430: u32 = 0x44B2_C000;
const B250: u32 = 0x437A_0000;
const B3000: u32 = 0x453B_8000;
const BM3000: u32 = 0xC53B_8000;

orig! {
    /// `CWeather::UpdateWeatherRegion(CVector*)` (0x72A640)
    pub fn update_weather_region_call(pos: GPtr<f32>) = cdecl 0x72A640 => update_weather_region;
}

/// `CWeather::IsRainy()` (0x4ABF50): `!(Rain < 0.2f)` (EAX 0 / 1; NaN gives 0).
#[hook(addr = 0x4ABF50, conv = cdecl, name = "CWeather::IsRainy", expect = "D9 05 24 13 C8 00")]
pub fn is_rainy() -> u32 {
    u32::from(!x(RAIN.get()).fcom(x(k(P02))).test_any(0x01))
}

/// `CWeather::Init()` (0x72A480): weather types and region 0, interpolation and list position 0, no
/// forced weather (-1), lightning sound time, forced rain, rain, sandstorm, rain particle strength,
/// tunnelness, lightning start and `StreamAfterRainTimer` 0.
#[hook(addr = 0x72A480, conv = cdecl, name = "CWeather::Init", expect = "33 C0 66 A3 1C 13 C8 00")]
pub fn init() {
    NEW_WEATHER_TYPE.set(0);
    OLD_WEATHER_TYPE.set(0);
    WEATHER_REGION.set(0);
    INTERPOLATION_VALUE.set(0.0);
    WEATHER_TYPE_IN_LIST.set(0);
    FORCED_WEATHER_TYPE.set(-1);
    WHEN_TO_PLAY_LIGHTNING_SOUND.set(0);
    SCRIPTS_FORCE_RAIN.set(0);
    RAIN.set(0.0);
    SANDSTORM.set(0.0);
    CURRENT_RAIN_PARTICLE_STRENGTH.set(0);
    IN_TUNNELNESS.set(0.0);
    LIGHTNING_START_X.set(0);
    LIGHTNING_START_Y.set(0);
    crate::common::g::<i32>(STREAM_AFTER_RAIN_TIMER).write(0);
}

/// `CWeather::ForceWeather(int16 type)` (0x72A4E0): `ForcedWeatherType = type`.
#[hook(addr = 0x72A4E0, conv = cdecl, name = "CWeather::ForceWeather", expect = "66 8B 44 24 04")]
pub fn force_weather(ty: u32) {
    FORCED_WEATHER_TYPE.set(ty as i16);
}

/// `CWeather::ForceWeatherNow(int16 type)` (0x72A4F0): forced, old and new type = `type`. The entry
/// jumps to 0x402157 (`mov ax, [esp+4]`), which jumps back to 0x72A4F5.
#[hook(addr = 0x72A4F0, conv = cdecl, name = "CWeather::ForceWeatherNow", expect = "E9 62 7C CD FF 66 A3 18 13 C8 00")]
pub fn force_weather_now(ty: u32) {
    let t = ty as i16;
    FORCED_WEATHER_TYPE.set(t);
    OLD_WEATHER_TYPE.set(t);
    NEW_WEATHER_TYPE.set(t);
}

/// `CWeather::ReleaseWeather()` (0x72A510): `ForcedWeatherType = -1`.
#[hook(addr = 0x72A510, conv = cdecl, name = "CWeather::ReleaseWeather", expect = "66 C7 05 18 13 C8 00 FF FF")]
pub fn release_weather() {
    FORCED_WEATHER_TYPE.set(-1);
}

/// The weather-type list of a region (0..4; any other value, negative included, gives the default
/// list).
fn list_for_region(region: i32) -> GPtr<i8> {
    let r = if (region as u32) > 4 { 0 } else { region };
    GPtr::from_addr(WEATHER_LISTS + r as usize * 0x40)
}

/// `CWeather::FindWeatherTypesList()` (0x72A520): the list of `WeatherRegion`.
#[hook(addr = 0x72A520, conv = cdecl, name = "CWeather::FindWeatherTypesList", expect = "0F BF 05 14 13 C8 00", preserve = "ecx,edx")]
pub fn find_weather_types_list() -> GPtr<i8> {
    list_for_region(i32::from(WEATHER_REGION.get()))
}

/// `CWeather::ForecastWeather(int type, int numSteps)` (0x72A590): whether `type` occurs at list
/// positions `(WeatherTypeInList + i) % 64` (C remainder) for `i = 0..=numSteps` (region and position
/// read once).
#[hook(addr = 0x72A590, conv = cdecl, name = "CWeather::ForecastWeather", expect = "53 55 8B 6C 24 10")]
pub fn forecast_weather(ty: i32, num_steps: i32) -> u8 {
    if num_steps < 0 {
        return 0;
    }
    let region = i32::from(WEATHER_REGION.get());
    let pos = WEATHER_TYPE_IN_LIST.get();
    let mut i = 0i32;
    loop {
        let idx = pos.wrapping_add(i).wrapping_rem(64);
        if i32::from(list_for_region(region).wrapping_index(idx).read()) == ty {
            return 1;
        }
        i += 1;
        if i > num_steps {
            return 0;
        }
    }
}

/// `CWeather::UpdateWeatherRegion(CVector* pos)` (0x72A640): `pos` or, when null, the camera position;
/// LV (3) when x > 1000 and y > 910; desert (4) when -850 < x < 1000 and y > 1280; SF (2) when x < -1430
/// and -580 < y < 1430; LA (1) when 250 < x < 3000 and -3000 < y < -850; else 0 (strict compares, NaN
/// fails them).
#[hook(addr = 0x72A640, conv = cdecl, name = "CWeather::UpdateWeatherRegion", expect = "A1 3C F0 B6 00")]
pub fn update_weather_region(pos: GPtr<f32>) {
    let ent = CAM_ENTITY.get();
    let cam: GPtr<f32> = if ent.is_null() {
        CAM_VECTOR.ptr().cast()
    } else {
        ent.byte_add(0x30).cast()
    };
    let p = if pos.is_null() { cam } else { pos };
    let (px, py) = (p.read(), p.wrapping_index(1).read());
    // `fld v; fcomp c`: greater = `test ah, 0x41; jne` not taken, less = `test ah, 5; jp` not taken
    let gt = |v: f32, c: u32| !x(v).fcom(x(k(c))).test_any(0x41);
    let lt = |v: f32, c: u32| !x(v).fcom(x(k(c))).test_parity(0x05);
    if gt(px, B1000) && gt(py, B910) {
        WEATHER_REGION.set(3);
        return;
    }
    if gt(px, BM850) && lt(px, B1000) && gt(py, B1280) {
        WEATHER_REGION.set(4);
        return;
    }
    if lt(px, BM1430) && gt(py, BM580) && lt(py, B1430) {
        WEATHER_REGION.set(2);
        return;
    }
    if gt(px, B250) && lt(px, B3000) && gt(py, BM3000) {
        WEATHER_REGION.set(1);
        if lt(py, BM850) {
            return;
        }
    }
    WEATHER_REGION.set(0);
}

/// `CWeather::SetWeatherToAppropriateTypeNow()` (0x72A790): the region at the player's position, then
/// old = new = the region list's FIRST entry (sign-extended), no forced weather.
#[hook(addr = 0x72A790, conv = cdecl, name = "CWeather::SetWeatherToAppropriateTypeNow", expect = "83 EC 0C 8D 44 24 00")]
pub fn set_weather_to_appropriate_type_now() {
    let mut v = [0f32; 3];
    find_player_coors(GPtr::from_raw(v.as_mut_ptr()), -1);
    update_weather_region_call(GPtr::from_raw(v.as_mut_ptr()));
    let t = i16::from(list_for_region(i32::from(WEATHER_REGION.get())).read());
    FORCED_WEATHER_TYPE.set(-1);
    OLD_WEATHER_TYPE.set(t);
    NEW_WEATHER_TYPE.set(t);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_IS_RAINY,
    HOOK_INIT,
    HOOK_FORCE_WEATHER,
    HOOK_FORCE_WEATHER_NOW,
    HOOK_RELEASE_WEATHER,
    HOOK_FIND_WEATHER_TYPES_LIST,
    HOOK_FORECAST_WEATHER,
    HOOK_UPDATE_WEATHER_REGION,
    HOOK_SET_WEATHER_TO_APPROPRIATE_TYPE_NOW,
];
