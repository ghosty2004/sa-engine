//! PORT-RENDER: `CSpecialFX` (static class). Ported from the 1.0 US bytes; gta-reversed
//! `SpecialFX.cpp` and `docs/re/specs/RENDER/CSpecialFX.deviations.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd};

/// `CMotionBlurStreaks::aStreaks[4]` (CRegisteredMotionBlurStreak, 0x54 bytes; owned by that port).
const STREAKS: usize = 0xC7CA08;

orig! {
    /// `CRegisteredMotionBlurStreak::Update()` (0x721EE0)
    pub fn streak_update(this: GPtr<u8>) = thiscall 0x721EE0 => unported;
    /// `CBulletTraces::Update()` (0x723FB0)
    pub fn bullet_traces_update() = cdecl 0x723FB0 => unported;
    /// `CCheckpoints::Update()` (0x7229C0)
    pub fn checkpoints_update() = cdecl 0x7229C0 => unported;
    /// `C3dMarkers::Update()` (0x7227B0)
    pub fn markers_update() = cdecl 0x7227B0 => unported;
    /// `CMotionBlurStreaks::Render()` (0x7240E0)
    pub fn streaks_render() = cdecl 0x7240E0 => unported;
    /// `CBulletTraces::Render()` (0x723C10)
    pub fn bullet_traces_render() = cdecl 0x723C10 => unported;
    /// `CBrightLights::Render()` (0x7241C0)
    pub fn bright_lights_render() = cdecl 0x7241C0 => unported;
    /// `CShinyTexts::Render()` (0x724890)
    pub fn shiny_texts_render() = cdecl 0x724890 => unported;
    /// `C3dMarkers::Render()` (0x725040)
    pub fn markers_render() = cdecl 0x725040 => unported;
    /// `CCheckpoints::Render()` (0x726060)
    pub fn checkpoints_render() = cdecl 0x726060 => unported;
}

/// `CSpecialFX::Update()` (0x726AA0): the four motion blur streaks with an id (+0) updated (inlined
/// `CMotionBlurStreaks::Update`), then `CBulletTraces::Update`, `CCheckpoints::Update`,
/// `C3dMarkers::Update` (tail jump).
#[hook(addr = 0x726AA0, conv = cdecl, name = "CSpecialFX::Update", expect = "56 BE 08 CA C7 00")]
pub fn update() {
    for i in 0..4 {
        let s = g::<u8>(STREAKS + i * 0x54);
        if rd::<u32>(s, 0) != 0 {
            streak_update(s);
        }
    }
    bullet_traces_update();
    checkpoints_update();
    markers_update();
}

/// `CSpecialFX::Render()` (0x726AD0): motion blur streaks, bullet traces, bright lights, shiny texts,
/// 3D markers, checkpoints (tail jump).
#[hook(addr = 0x726AD0, conv = cdecl, name = "CSpecialFX::Render", expect = "E8 0B D6 FF FF")]
pub fn render() {
    streaks_render();
    bullet_traces_render();
    bright_lights_render();
    shiny_texts_render();
    markers_render();
    checkpoints_render();
}

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];

pub const HOOKS: &[HookReg] = &[HOOK_UPDATE, HOOK_RENDER];
