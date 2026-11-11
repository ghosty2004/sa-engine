//! The HUD static initialisers (PORT-HUD): CRT `__init_*` functions that construct HUD globals before
//! `WinMain`. Ported from the 1.0 US bytes. Constructors and the CRT helpers stay `orig!` calls.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, x};

/// No globals of its own (the constructed objects belong to their classes).
pub const GLOBALS: &[GlobalDesc] = &[];

orig! {
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// CRT `_atexit` (0x821D1E)
    pub fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// `CHudColours::CHudColours()` (0x58FDA0)
    pub fn hud_colours_ctor(this: GPtr<u8>) = thiscall 0x58FDA0 => unported;
    /// `CMenuManager::CMenuManager()` (0x574350)
    pub fn menu_manager_ctor(this: GPtr<u8>) = thiscall 0x574350 => unported;
}

/// `CSprite2d::CSprite2d()` (0x727230) / `CSprite2d::~CSprite2d()` (0x7281E0)
const SPRITE2D_CTOR: u32 = 0x727230;
const SPRITE2D_DTOR: u32 = 0x7281E0;

/// `eh vector constructor iterator(arr, 4, n, CSprite2d ctor, dtor)`, then `atexit(dtor_fn)`.
fn sprites(arr: usize, n: u32, ctor: u32, dtor: u32, exit_fn: u32) {
    eh_vector_ctor(g(arr), 4, n, ctor, dtor);
    atexit(exit_fn);
}

/// `__init_CHud__Sprites` (0x84F1C0): `CHud::Sprites[6]` (0xBAB1FC).
#[hook(addr = 0x84F1C0, conv = cdecl, name = "__init_CHud__Sprites", expect = "68 E0 81 72 00")]
pub fn init_hud_sprites() {
    sprites(0xBAB1FC, 6, SPRITE2D_CTOR, SPRITE2D_DTOR, 0x8565B0);
}

/// `__init_CFont__Sprite` (0x854DC0): `CFont::Sprite[2]` (0xC71AD0).
#[hook(addr = 0x854DC0, conv = cdecl, name = "__init_CFont__Sprite", expect = "68 E0 81 72 00")]
pub fn init_font_sprite() {
    sprites(0xC71AD0, 2, SPRITE2D_CTOR, SPRITE2D_DTOR, 0x856B40);
}

/// `__init_CFont__ButtonSprite` (0x854DF0): `CFont::ButtonSprite[15]` (0xC71AD8).
#[hook(addr = 0x854DF0, conv = cdecl, name = "__init_CFont__ButtonSprite", expect = "68 E0 81 72 00")]
pub fn init_font_button_sprite() {
    sprites(0xC71AD8, 0xF, SPRITE2D_CTOR, SPRITE2D_DTOR, 0x856B60);
}

/// `__init_CRadar__RadarBlipSprites` (0x84F0D0): `CRadar::RadarBlipSprites[64]` (0xBAA250).
#[hook(addr = 0x84F0D0, conv = cdecl, name = "__init_CRadar__RadarBlipSprites", expect = "68 E0 81 72 00")]
pub fn init_radar_blip_sprites() {
    sprites(0xBAA250, 0x40, SPRITE2D_CTOR, SPRITE2D_DTOR, 0x856590);
}

/// `__init_CRadar__ArrowBlipColour` (0x84F0A0): `CRadar::ArrowBlipColour[6]` (0xBA86D4, `CRGBA`
/// constructor 0x463360 / destructor 0x43CE70).
#[hook(addr = 0x84F0A0, conv = cdecl, name = "__init_CRadar__ArrowBlipColour", expect = "68 70 CE 43 00")]
pub fn init_radar_arrow_blip_colour() {
    sprites(0xBA86D4, 6, 0x463360, 0x43CE70, 0x856570);
}

/// `__init_HudColour` (0x84F1F0): `HudColour.CHudColours()` (0xBAB22C), `atexit`.
#[hook(addr = 0x84F1F0, conv = cdecl, name = "__init_HudColour", expect = "B9 2C B2 BA 00")]
pub fn init_hud_colour() {
    hud_colours_ctor(g(0xBAB22C));
    atexit(0x8565D0);
}

/// `__init_FrontEndMenuManager` (0x84EE40): `FrontEndMenuManager.CMenuManager()` (0xBA6748), `atexit`.
#[hook(addr = 0x84EE40, conv = cdecl, name = "__init_FrontEndMenuManager", expect = "B9 48 67 BA 00")]
pub fn init_front_end_menu_manager() {
    menu_manager_ctor(g(0xBA6748));
    atexit(0x856560);
}

/// `__init_g_ba672c` (0x84ED80): `[0xBA672C] = 0.005 * 0.0625`.
#[hook(addr = 0x84ED80, conv = cdecl, name = "__init_g_ba672c", expect = "D9 05 A8 51 86 00")]
pub fn init_g_ba672c() {
    let v = x(f32::from_bits(0x3BA3_D70A)).fmul_m(0.0625);
    g::<f32>(0xBA672C).write(v.to_f32());
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_HUD_SPRITES,
    HOOK_INIT_FONT_SPRITE,
    HOOK_INIT_FONT_BUTTON_SPRITE,
    HOOK_INIT_RADAR_BLIP_SPRITES,
    HOOK_INIT_RADAR_ARROW_BLIP_COLOUR,
    HOOK_INIT_HUD_COLOUR,
    HOOK_INIT_FRONT_END_MENU_MANAGER,
    HOOK_INIT_G_BA672C,
];
