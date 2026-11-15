//! FRONTIER-2 (batch `frontier2-2`): small `CPostEffects` switches and light hooks on the frontier (script night /
//! infrared vision, heat haze, the day-night balance store / restore, the vision light passes, the screen-mode
//! initialisation). Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::g;
use crate::posteffects::immediate_mode_filter_stuff_initialize_call;
use crate::bindings::heat_haze_fx_init;

/// `CPostEffects::m_bNightVision`, `m_bInfraredVision`, `m_bHeatHazeFX`, `m_bDisableAllPostEffect`.
const NIGHT_VISION: usize = 0xC4_02B8;
const INFRARED_VISION: usize = 0xC4_02B9;
const HEAT_HAZE_FX: usize = 0xC4_02BA;
const DISABLE_ALL: usize = 0xC4_02B7;
/// `CTimeCycle` day-night balance (0x8D12C0), its saved copy (0xC3F860) and the filter value (0x8D50A4).
const DAY_NIGHT_BALANCE: usize = 0x8D_12C0;
const SAVED_BALANCE: usize = 0xC3_F860;
const FILTER_BALANCE: usize = 0x8D_50A4;

orig! {
    /// `Light::SetLightsForNightVision()` (0x735F70)
    fn set_lights_for_night_vision() = cdecl 0x735F70 => unported;
    /// `Light::RestoreLightsForInfraredVisionHeatObjects()` (0x735EF0)
    fn restore_lights_for_infrared_heat_objects() = cdecl 0x735EF0 => unported;
    /// `Light::SetLightsForInfraredVisionDefaultObjects()` (0x735F20)
    fn set_lights_for_infrared_default_objects() = cdecl 0x735F20 => unported;
}

fn flag(a: usize) -> u8 {
    g::<u8>(a).read()
}

/// `CPostEffects::ScriptNightVisionSwitch(bool)` (0x701120): on sets night vision and clears infrared; off clears it.
#[hook(addr = 0x701120, conv = cdecl, name = "CPostEffects::ScriptNightVisionSwitch", expect = "8A 4C 24 04 32 C0 3A C8",
       unverified = "frontier2-2", route = "L2", hazards = "none")]
pub fn script_night_vision_switch(on: u32) {
    if on as u8 != 0 {
        g::<u8>(NIGHT_VISION).write(1);
        g::<u8>(INFRARED_VISION).write(0);
    } else {
        g::<u8>(NIGHT_VISION).write(0);
    }
}

/// `CPostEffects::ScriptInfraredVisionSwitch(bool)` (0x701140): on sets infrared and clears night vision; off clears it.
#[hook(addr = 0x701140, conv = cdecl, name = "CPostEffects::ScriptInfraredVisionSwitch", expect = "8A 4C 24 04 32 C0 3A C8",
       unverified = "frontier2-2", route = "L2", hazards = "none")]
pub fn script_infrared_vision_switch(on: u32) {
    if on as u8 != 0 {
        g::<u8>(INFRARED_VISION).write(1);
        g::<u8>(NIGHT_VISION).write(0);
    } else {
        g::<u8>(INFRARED_VISION).write(0);
    }
}

/// `CPostEffects::ScriptHeatHazeFXSwitch(bool)` (0x701160): the byte.
#[hook(addr = 0x701160, conv = cdecl, name = "CPostEffects::ScriptHeatHazeFXSwitch", expect = "8A 44 24 04 A2 BA 02 C4 00",
       unverified = "frontier2-2", route = "L2", hazards = "none")]
pub fn script_heat_haze_fx_switch(on: u32) {
    g::<u8>(HEAT_HAZE_FX).write(on as u8);
}

/// `CPostEffects::NightVisionSetLights()` (0x7012E0): with night vision on and post effects not disabled.
#[hook(addr = 0x7012E0, conv = cdecl, name = "CPostEffects::NightVisionSetLights", expect = "A0 B8 02 C4 00 84 C0 74",
       unverified = "frontier2-2", route = "L2", hazards = "none")]
pub fn night_vision_set_lights() {
    if flag(NIGHT_VISION) != 0 && flag(DISABLE_ALL) == 0 {
        set_lights_for_night_vision();
    }
}

/// `CPostEffects::InfraredVisionRestoreLightsForHeatObjects()` (0x701410): with infrared on, not disabled.
#[hook(addr = 0x701410, conv = cdecl, name = "CPostEffects::InfraredVisionRestoreLightsForHeatObjects",
       expect = "A0 B9 02 C4 00 84 C0 74", unverified = "frontier2-2", route = "L2", hazards = "none")]
pub fn infrared_vision_restore_lights_for_heat_objects() {
    if flag(INFRARED_VISION) != 0 && flag(DISABLE_ALL) == 0 {
        restore_lights_for_infrared_heat_objects();
    }
}

/// `CPostEffects::InfraredVisionSetLightsForDefaultObjects()` (0x701430): with infrared on, not disabled.
#[hook(addr = 0x701430, conv = cdecl, name = "CPostEffects::InfraredVisionSetLightsForDefaultObjects",
       expect = "A0 B9 02 C4 00 84 C0 74", unverified = "frontier2-2", route = "L2", hazards = "none")]
pub fn infrared_vision_set_lights_for_default_objects() {
    if flag(INFRARED_VISION) != 0 && flag(DISABLE_ALL) == 0 {
        set_lights_for_infrared_default_objects();
    }
}

/// `CPostEffects::FilterFX_StoreAndSetDayNightBalance()` (0x7034B0; the first load runs in the no-CD chunk
/// 0x401682): unless post effects are disabled, the balance is saved and replaced by the filter value.
/// Registered for phase B only (`#[native]`): the no-CD chunk at 0x4084D2 reads the entry's jmp as a key.
#[sa_core::native(addr = 0x7034B0, conv = cdecl, name = "CPostEffects::FilterFX_StoreAndSetDayNightBalance")]
pub fn filter_fx_store_and_set_day_night_balance() {
    if flag(DISABLE_ALL) == 0 {
        let b = g::<u32>(DAY_NIGHT_BALANCE).read();
        let f = g::<u32>(FILTER_BALANCE).read();
        g::<u32>(SAVED_BALANCE).write(b);
        g::<u32>(DAY_NIGHT_BALANCE).write(f);
    }
}

/// `CPostEffects::FilterFX_RestoreDayNightBalance()` (0x7034D0): unless disabled, the saved balance back.
#[hook(addr = 0x7034D0, conv = cdecl, name = "CPostEffects::FilterFX_RestoreDayNightBalance", expect = "A0 B7 02 C4 00 84 C0 75",
       unverified = "frontier2-2", route = "L2", hazards = "none")]
pub fn filter_fx_restore_day_night_balance() {
    if flag(DISABLE_ALL) == 0 {
        g::<u32>(DAY_NIGHT_BALANCE).write(g::<u32>(SAVED_BALANCE).read());
    }
}

/// `CPostEffects::DoScreenModeDependentInitializations()` (0x7046D0): `ImmediateModeFilterStuffInitialize`, then a
/// tail jump to `HeatHazeFXInit`.
#[hook(addr = 0x7046D0, conv = cdecl, name = "CPostEffects::DoScreenModeDependentInitializations", expect = "E8 EB F5 FF FF E9 76 CD",
       unverified = "frontier2-2", route = "L2", hazards = "none")]
pub fn do_screen_mode_dependent_initializations() {
    immediate_mode_filter_stuff_initialize_call();
    heat_haze_fx_init();
}

pub const HOOKS: &[HookReg] = &[
    HOOK_SCRIPT_NIGHT_VISION_SWITCH,
    HOOK_SCRIPT_INFRARED_VISION_SWITCH,
    HOOK_SCRIPT_HEAT_HAZE_FX_SWITCH,
    HOOK_NIGHT_VISION_SET_LIGHTS,
    HOOK_INFRARED_VISION_RESTORE_LIGHTS_FOR_HEAT_OBJECTS,
    HOOK_INFRARED_VISION_SET_LIGHTS_FOR_DEFAULT_OBJECTS,
    HOOK_FILTER_FX_RESTORE_DAY_NIGHT_BALANCE,
    HOOK_DO_SCREEN_MODE_DEPENDENT_INITIALIZATIONS,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_FILTER_FX_STORE_AND_SET_DAY_NIGHT_BALANCE];
