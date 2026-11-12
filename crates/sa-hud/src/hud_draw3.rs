//! `CHud`: `DrawPlayerInfo`, `DrawCrossHairs`, `DrawVitalStats` (PORT-HUD-3). Ported from the 1.0 US bytes;
//! gta-reversed as reading aid (the bytes win; gta-reversed has no body for any of the three, only
//! `plugin::Call` stubs, and the dead C++ after `DrawCrossHairs`' stub differs from the bytes).
//!
//! Only the orchestration is Rust: conditions, coordinates, x87 arithmetic in the original order, global reads
//! and writes, the fade state machines. Every call into RenderWare, `CSprite2d`, `CSprite`, `CFont`, `CStats`,
//! `CPed*`, `CWeapon*` and the other HUD functions stays an `orig!` call. The `CHud` state-machine globals
//! (0xBAA3FC..0xBAA440) are addressed directly, as `hud_draw.rs` does.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, rd, x, xi};

/// No globals of its own (the `CHud` statics it writes are addressed directly).
pub const GLOBALS: &[GlobalDesc] = &[];

// Foreign globals (read / written by address).
/// `CWorld::PlayerInFocus` (byte) and `CWorld::Players[]` (stride 0x190: +0 `m_pPed`, +0xBC display money,
/// +0xE4 last time energy lost)
const PLAYER_IN_FOCUS: usize = 0xB7CD74;
const PLAYERS: usize = 0xB7CD98;
/// `CWorld::Players[1].m_pPed`
const PLAYER2_PED: usize = 0xB7CF28;
/// `RsGlobal.maximumWidth` / `maximumHeight`
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// `CTimer::ms_fTimeStep`, `CTimer::m_snTimeInMilliseconds`
const TIME_STEP: usize = 0xB7CB5C;
const TIME_MS: usize = 0xB7CB84;
/// `CHud::bDrawClock` (int)
const DRAW_CLOCK: usize = 0xBAA400;
/// `CHud::m_LastBreathTime`
const LAST_BREATH_TIME: usize = 0xBAA3FC;
/// Fade machines (state, fade timer, timer, last key): weapon, display score, energy lost
const WEAPON_MACHINE: usize = 0xBAA404;
const SCORE_MACHINE: usize = 0xBAA424;
const ENERGY_MACHINE: usize = 0xBAA434;
/// `CClock::ms_nGameClockHours` / `ms_nGameClockMinutes` (bytes), `CClock::CurrentDay` (byte)
const CLOCK_HOURS: usize = 0xB70153;
const CLOCK_MINUTES: usize = 0xB70152;
const CLOCK_DAY: usize = 0xB7014E;
/// `HudColour` (`CHudColours`)
const HUD_COLOUR: usize = 0xBAB22C;
/// `TheText` (`CText`)
const THE_TEXT: usize = 0xC1B340;
/// `FrontEndMenuManager`
const FRONT_END_MENU_MANAGER: usize = 0xBA6748;
/// `gString`
const G_STRING: usize = 0xB71670;
/// `CReplay::Mode`, `TheCamera.m_bWideScreenOn` (byte 0xB6F065)
const REPLAY_MODE: usize = 0xA43088;
const WIDE_SCREEN_ON: usize = 0xB6F065;
/// `TheCamera.m_nActiveCam` (byte), `TheCamera.m_aCams[i].m_nMode` (u16, stride 0x238), `TheCamera` +0x58
/// (byte 0xB6F080, gr `m_bTransitionState`)
const CAMERA_ACTIVE_CAM: usize = 0xB6F081;
const CAM_MODE: usize = 0xB6F1A8;
const CAMERA_TRANSITION_STATE: usize = 0xB6F080;
/// `CCamera::m_f3rdPersonCHairMultX` / `MultY`
const CHAIR_MULT_X: usize = 0xB6EC14;
const CHAIR_MULT_Y: usize = 0xB6EC10;
/// `CTheScripts::bDrawCrossHair` (int)
const SCRIPTS_DRAW_CROSS_HAIR: usize = 0xA44490;
/// `CHud::Sprites[1]` (crosshair quarter), `Sprites[2].m_pTexture` (dword 0xBAB204)
const SPRITE_CROSSHAIR: usize = 0xBAB200;
const SPRITE2_TEXTURE: usize = 0xBAB204;
/// `CModelInfo::ms_modelInfoPtrs`, `CTxdStore::ms_pTxdPool`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
const TXD_POOL: usize = 0xC8800C;
/// `CStats` float tables `DrawVitalStats` reads for the weapon skill row (0xB78EB4 + 4k, 0xB78F40 + 4(k - 0x45))
const STAT_TABLE_A: u32 = 0xB78EB4;
const STAT_TABLE_B: u32 = 0xB78F40;

// .rdata strings
/// `"%02d:%02d"`, `"$%08d"`, `"-$%07d"`, `"DAY_%d"`, `"FEH_STA"`, `"CROSSHAIR"`
const STR_CLOCK_FMT: usize = 0x859A6C;
const STR_MONEY_FMT: usize = 0x866C94;
const STR_MONEY_NEG_FMT: usize = 0x866C8C;
const STR_DAY_FMT: usize = 0x866BC4;
const STR_FEH_STA: usize = 0x86613C;
const STR_CROSSHAIR: usize = 0x866C68;
/// `"STAT068"`, `"STAT225"`, `"CURWSKL"`, `"STAT022"`, `"STAT023"`, `"STAT021"`, `"STAT025"`
const STR_STAT068: usize = 0x866C00;
const STR_STAT225: usize = 0x866BF8;
const STR_CURWSKL: usize = 0x866BF0;
const STR_STAT022: usize = 0x866BE4;
const STR_STAT023: usize = 0x866BDC;
const STR_STAT021: usize = 0x866BD4;
const STR_STAT025: usize = 0x866BCC;

// .rdata float constants (bits)
/// `1/640` (0x859520), `1/448` (0x859524)
const K_1_640: u32 = 0x3ACC_CCCD;
const K_1_448: u32 = 0x3B12_4925;
/// `0.0` (0x858B50), `0.5` (0x858B8C), `1.0` (0x858624), `3.0` (0x858B3C), `5.0` (0x858C80), `10.0` (0x85862C)
const K_ZERO: u32 = 0;
const K_HALF: u32 = 0x3F00_0000;
const K_1: u32 = 0x3F80_0000;
const K_3: u32 = 0x4040_0000;
const K_5: u32 = 0x40A0_0000;
const K_10: u32 = 0x4120_0000;
/// `15.0` (0x858B48), `20.0` (0x858BA4), `22.0` (0x858F8C), `24.0` (0x85F114), `32.0` (0x85950C), `40.0` (0x858A10)
const K_15: u32 = 0x4170_0000;
const K_20: u32 = 0x41A0_0000;
const K_22: u32 = 0x41B0_0000;
const K_24: u32 = 0x41C0_0000;
const K_32: u32 = 0x4200_0000;
const K_40: u32 = 0x4220_0000;
/// `43.0` (0x866C78), `47.0` (0x866C4C), `48.0` (0x862D38), `62.0` (0x86503C), `64.0` (0x859A44), `70.0` (0x858CE0)
const K_43: u32 = 0x422C_0000;
const K_47: u32 = 0x423C_0000;
const K_48: u32 = 0x4240_0000;
const K_62: u32 = 0x4278_0000;
const K_64: u32 = 0x4280_0000;
const K_70: u32 = 0x428C_0000;
/// `77.0` (0x866CA8), `89.0` (0x866C88), `90.0` (0x85991C), `94.0` (0x866B78), `100.0` (0x858628), `111.0` (0x866C7C)
const K_77: u32 = 0x429A_0000;
const K_89: u32 = 0x42B2_0000;
const K_90: u32 = 0x42B4_0000;
const K_94: u32 = 0x42BC_0000;
const K_100: u32 = 0x42C8_0000;
const K_111: u32 = 0x42DE_0000;
/// `127.0` (0x86532C), `138.0` (0x866C80), `140.0` (0x865374), `141.0` (0x86535C), `160.0` (0x858970), `164.0` (0x866CA0)
const K_127: u32 = 0x42FE_0000;
const K_138: u32 = 0x430A_0000;
const K_140: u32 = 0x430C_0000;
const K_141: u32 = 0x430D_0000;
const K_160: u32 = 0x4320_0000;
const K_164: u32 = 0x4324_0000;
/// `170.0` (0x858F98), `179.0` (0x866C9C), `192.0` (0x866C74), `194.0` (0x866CA4), `210.0` (0x863B34), `255.0` (0x859AAC)
const K_170: u32 = 0x432A_0000;
const K_179: u32 = 0x4333_0000;
const K_192: u32 = 0x4340_0000;
const K_194: u32 = 0x4342_0000;
const K_210: u32 = 0x4352_0000;
const K_255: u32 = 0x437F_0000;
/// `256.0` (0x858FB4), `640.0` (0x85951C), `999.0` (0x866BEC), `1000.0` (0x858C4C), `-1000.0` (0x859948),
/// `10000.0` (0x859AA4)
const K_256: u32 = 0x4380_0000;
const K_640: u32 = 0x4420_0000;
const K_999: u32 = 0x4479_C000;
const K_1000: u32 = 0x447A_0000;
const K_M1000: u32 = 0xC47A_0000;
const K_10000: u32 = 0x461C_4000;
/// `0.001` (0x858CDC), `0.02` (0x858B38), `0.1` (0x858B1C), `0.2` (0x858CC4), `0.35` (0x858F9C), `0.55` (0x866CAC)
const K_0_001: u32 = 0x3A83_126F;
const K_0_02: u32 = 0x3CA3_D70A;
const K_0_1: u32 = 0x3DCC_CCCD;
const K_0_2: u32 = 0x3E4C_CCCD;
const K_0_35: u32 = 0x3EB3_3333;
const K_0_55: u32 = 0x3F0C_CCCD;
/// `0.7` (0x858CB0), `0.9` (0x858C20), `1.1` (0x858F14), `0.17343046` (0x866C84)
const K_0_7: u32 = 0x3F33_3333;
const K_0_9: u32 = 0x3F66_6666;
const K_1_1: u32 = 0x3F8C_CCCD;
const K_0_1734: u32 = 0x3E31_97C1;
/// `0.01` and `1.0` pushed as immediates (`RenderOneXLUSprite`'s recipZ and z)
const RECIP_Z_0_01: u32 = 0x3C23_D70A;
const ONE_BITS: u32 = 0x3F80_0000;
/// `40.0` stored by `mov dword` (the stats window's left edge)
const FORTY_BITS: u32 = 0x4220_0000;

fn k(bits: u32) -> f32 {
    f32::from_bits(bits)
}

fn b(addr: usize) -> u8 {
    g::<u8>(addr).read()
}

fn max_w() -> X87 {
    xi(g::<i32>(RS_MAX_WIDTH).read())
}

fn max_h() -> X87 {
    xi(g::<i32>(RS_MAX_HEIGHT).read())
}

/// A local buffer as a game pointer (the original's stack temporaries).
fn local<T, const N: usize>(buf: &mut [T; N]) -> GPtr<T> {
    GPtr::from_raw(buf.as_mut_ptr())
}

orig! {
    /// `RwRenderStateSet` through `RwEngineInstance` (`mov r, [0xC97B24]; call [r+0x20]`)
    fn rw_render_state_set(state: u32, value: u32) -> u32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CRGBA::CRGBA(r, g, b, a)` (0x7170C0)
    fn rgba_ctor(this: GPtr<u8>, r: u32, gg: u32, bb: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CHudColours::GetRGB(CRGBA* out, uint8 colour)` (0x58FEA0)
    fn hud_colours_get_rgb(this: GPtr<u8>, out: GPtr<u8>, colour: u32) -> GPtr<u8> = thiscall 0x58FEA0 => crate::hud::get_rgb;
    /// `CHudColours::GetRGBA(CRGBA* out, uint8 colour, uint8 alpha)` (0x58FEE0)
    fn hud_colours_get_rgba(this: GPtr<u8>, out: GPtr<u8>, colour: u32, alpha: u32) -> GPtr<u8> = thiscall 0x58FEE0 => crate::hud::get_rgba;
    /// CRT `sprintf(buf, fmt, a)` (0x821BB5)
    fn sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, a: i32) -> i32 = cdecl 0x821BB5 => unported;
    /// CRT `sprintf(buf, fmt, a, b)` (0x821BB5)
    fn sprintf2(buf: GPtr<u8>, fmt: GPtr<u8>, a: i32, bb: i32) -> i32 = cdecl 0x821BB5 => unported;
    /// `AsciiToGxtChar(const char*, GxtChar*)` (0x718600)
    fn ascii_to_gxt_char(src: GPtr<u8>, dst: GPtr<u8>) = cdecl 0x718600 => unported;
    /// `CText::Get(const char*)` (0x6A0050)
    fn text_get(this: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => crate::text::text_get;

    /// `CFont::SetBackground(bool, bool)` (0x7195C0)
    fn font_set_background(on: u32, outline: u32) = cdecl 0x7195C0 => crate::font::set_background;
    /// `CFont::SetScale(float w, float h)` (0x719380)
    fn font_set_scale(w: u32, h: u32) = cdecl 0x719380 => crate::font::set_scale;
    /// `CFont::SetProportional(bool)` (0x7195B0)
    fn font_set_proportional(v: u32) = cdecl 0x7195B0 => crate::font::set_proportional;
    /// `CFont::SetFontStyle(eFontStyle)` (0x719490)
    fn font_set_font_style(v: u32) = cdecl 0x719490 => crate::font::set_font_style;
    /// `CFont::SetOrientation(eFontAlignment)` (0x719610; not hookable, see hud-port.md)
    fn font_set_orientation(v: u32) = cdecl 0x719610 => crate::font::set_orientation;
    /// `CFont::SetRightJustifyWrap(float)` (0x7194F0)
    fn font_set_right_justify_wrap(v: u32) = cdecl 0x7194F0 => crate::font::set_right_justify_wrap;
    /// `CFont::SetWrapx(float)` (0x7194D0)
    fn font_set_wrapx(v: u32) = cdecl 0x7194D0 => crate::font::set_wrapx;
    /// `CFont::SetEdge(int8)` (0x719590)
    fn font_set_edge(v: u32) = cdecl 0x719590 => crate::font::set_edge;
    /// `CFont::SetDropColor(CRGBA)` (0x719510)
    fn font_set_drop_color(c: u32) = cdecl 0x719510 => crate::font::set_drop_color;
    /// `CFont::SetDropShadowPosition(int8)` (0x719570)
    fn font_set_drop_shadow_position(v: u32) = cdecl 0x719570 => crate::font::set_drop_shadow_position;
    /// `CFont::SetColor(CRGBA)` (0x719430)
    fn font_set_color(c: u32) = cdecl 0x719430 => crate::font::set_color;
    /// `CFont::PrintString(float x, float y, const GxtChar*)` (0x71A700)
    fn font_print_string(px: f32, py: f32, text: GPtr<u8>) = cdecl 0x71A700 => unported;

    /// `CHud::GetYPosBasedOnHealth(uint8, float, int8)` (0x588B60, ST0)
    fn get_y_pos_based_on_health_st0(player: u8, y: f32, offset: i8) -> f64 = cdecl 0x588B60 => get_y_pos_st0_rust;
    /// `CHud::RenderHealthBar(int32 player, int32 x, int32 y)` (0x589270)
    fn render_health_bar_call(id: i32, px: i32, py: i32) = cdecl 0x589270 => crate::hud_draw::render_health_bar;
    /// `CHud::RenderArmorBar(int32 player, int32 x, int32 y)` (0x5890A0)
    fn render_armor_bar_call(id: i32, px: i32, py: i32) = cdecl 0x5890A0 => crate::hud_draw::render_armor_bar;
    /// `CHud::RenderBreathBar(int32 player, int32 x, int32 y)` (0x589190)
    fn render_breath_bar_call(id: i32, px: i32, py: i32) = cdecl 0x589190 => crate::hud_draw::render_breath_bar;
    /// `CHud::DrawWeaponIcon(CPed*, int32 x, int32 y, float alpha)` (0x58D7D0)
    fn draw_weapon_icon_call(ped: GPtr<u8>, px: i32, py: i32, alpha: f32) = cdecl 0x58D7D0 => crate::hud_draw::draw_weapon_icon;
    /// `CHud::DrawAmmo(CPed*, int32 x, int32 y, float alpha)` (0x5893B0)
    fn draw_ammo_call(ped: GPtr<u8>, px: i32, py: i32, alpha: f32) = cdecl 0x5893B0 => crate::hud_draw::draw_ammo;
    /// `CPedIntelligence::GetTaskSwim()` (0x601070)
    fn intelligence_get_task_swim(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x601070 => unported;
    /// `CPedIntelligence::GetTaskUseGun()` (0x600F70)
    fn intelligence_get_task_use_gun(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F70 => unported;
    /// `CStats::GetFatAndMuscleModifier(eStatModAbilities)` (0x559AF0, ST0)
    fn stats_get_fat_and_muscle_modifier_st0(stat: u32) -> f64 = cdecl 0x559AF0 => unported;
    /// `CStats::GetStatValue(eStats)` (0x558E40, ST0)
    fn stats_get_stat_value_st0(stat: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `CWeaponInfo::GetSkillStatIndex(eWeaponType)` (0x743CD0)
    fn weapon_info_get_skill_stat_index(ty: u32) -> i32 = cdecl 0x743CD0 => unported;
    /// `floorf` wrapper (0x406CE0, cdecl (float) -> ST0)
    fn floorf_st0(v: f32) -> f64 = cdecl 0x406CE0 => unported;
    /// `CMenuManager::DrawWindow(const CRect&, const char* key, uint8 colour, CRGBA back, bool unused, bool
    /// background)` (0x573EE0, thiscall)
    fn draw_window_call(this: GPtr<u8>, rect: GPtr<u8>, key: GPtr<u8>, colour: u32, back: u32, unused: u32, bg: u32) = thiscall 0x573EE0 => crate::menu_mgr::draw_window;
    /// `CSprite2d::DrawBarChart(x, y, width, height, progress, progressAdd, drawPercentage, drawBlackBorder,
    /// CRGBA colour, CRGBA addColour)` (0x728640)
    fn sprite2d_draw_bar_chart(px: f32, py: f32, w: u32, h: u32, progress: f32, add: u32, pct: u32, border: u32, colour: u32, add_colour: u32) = cdecl 0x728640 => unported;

    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => sa_world::world::cworld::find_player_vehicle_port;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
    /// `CWeapon::IsTypeMelee()` (0x73B1C0)
    fn weapon_is_type_melee(this: GPtr<u8>) -> u8 = thiscall 0x73B1C0 => unported;
    /// `CPlayerPed::GetWeaponRadiusOnScreen()` (0x609CD0, ST0)
    fn player_ped_get_weapon_radius_on_screen_st0(this: GPtr<u8>) -> f64 = thiscall 0x609CD0 => unported;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727B60)
    fn sprite2d_draw_rect_static(rect: GPtr<f32>, colour: GPtr<u8>) = cdecl 0x727B60 => unported;
    /// `CSprite2d::Draw(const CRect&, const CRGBA&)` (0x728350)
    fn sprite2d_draw_rect(this: GPtr<u8>, rect: GPtr<f32>, colour: GPtr<u8>) = thiscall 0x728350 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn get_weapon_info(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CKeyGen::AppendStringToKey(uint32, const char*)` (0x53CF70)
    fn append_string_to_key(key: u32, s: GPtr<u8>) -> u32 = cdecl 0x53CF70 => sa_core::keygen::append_string_to_key;
    /// `RwTexDictionaryFindHashNamedTexture(RwTexDictionary*, uint32)` (0x734E50)
    fn rw_tex_dictionary_find_hash_named_texture(dict: GPtr<u8>, hash: u32) -> GPtr<u8> = cdecl 0x734E50 => unported;
    /// `CSprite::RenderOneXLUSprite(x, y, z, halfW, halfH, r, g, b, intensity, rhw, a, uDir, vDir)` (0x70D000)
    fn sprite_render_one_xlu_sprite(px: f32, py: f32, pz: u32, hw: u32, hh: u32, r: u32, gg: u32, bb: u32, intensity: u32, rhw: u32, a: u32, u_dir: u32, v_dir: u32) = cdecl 0x70D000 => unported;
}

/// The Rust path of the ST0 declaration above (phases B/C).
#[allow(dead_code)]
fn get_y_pos_st0_rust(player: u8, y: f32, offset: i8) -> f64 {
    crate::hud::get_y_pos_based_on_health(player, y, offset).to_f64()
}

/// `CHud::GetYPosBasedOnHealth` as an x87 value (ST0).
fn gy(player: u8, y: f32, offset: i8) -> X87 {
    X87::from_f64_exact(get_y_pos_based_on_health_st0(player, y, offset))
}

/// `CStats::GetFatAndMuscleModifier` as an x87 value (ST0).
fn fat_and_muscle_modifier(stat: u32) -> X87 {
    X87::from_f64_exact(stats_get_fat_and_muscle_modifier_st0(stat))
}

/// `CStats::GetStatValue` as an x87 value (ST0).
fn stat_value(stat: u32) -> X87 {
    X87::from_f64_exact(stats_get_stat_value_st0(stat))
}

/// `CRGBA(r, g, b, a)` on a stack temporary; returns the dword the original passes on.
fn rgba(r: u32, gg: u32, bb: u32, a: u32) -> u32 {
    let mut c = [0u8; 4];
    let p = rgba_ctor(local(&mut c), r, gg, bb, a);
    rd::<u32>(p, 0)
}

/// `ftol(CTimer::ms_fTimeStep * 0.02 * c)`: the fade steps of the HUD state machines.
fn fade_step(c: u32) -> i32 {
    ftol32(x(g::<f32>(TIME_STEP).read()).fmul_m(k(K_0_02)).fmul_m(k(c)))
}

/// The active weapon type of `ped` (`ped + 0x5A0 + (i8)ped[0x718] * 0x1C`).
fn weapon_type(ped: GPtr<u8>) -> u32 {
    let slot = i32::from(rd::<i8>(ped, 0x718));
    rd::<u32>(ped, slot.wrapping_mul(0x1C) as isize + 0x5A0)
}

/// The fade state machine `DrawPlayerInfo` inlines three times (state, fade timer, timer at `base`, `+4`,
/// `+8`; the last key at `+0xC`), as `DrawWanted` / `DrawFadeState` run it: a changed key restarts a hidden
/// or fading machine (state 0: fade 0; states 0, 1, 3: timer 5, state 2); states 1..3 step (1: fade 1000,
/// to 3 / fade 3000 once the timer passes 10000; 2: fade in, to 1 at 1000; 3: fade out, to 0 below 0) and
/// the timer advances (not in states 0 and 5). The alpha (`fade * 0.001 * 255`, 255 in state 1 and the
/// other states, clamped to [0, 255]) is computed only `with_alpha` (the energy machine has none).
/// `new_last` gives the value stored as the last key when it changed (the weapon machine re-reads it).
/// Returns (state, alpha).
fn fade_machine(base: usize, key: i32, with_alpha: bool, new_last: impl FnOnce() -> i32) -> (i32, X87) {
    let mut state = g::<i32>(base).read();
    let mut fade = g::<i32>(base + 4).read();
    let mut timer = g::<i32>(base + 8).read();
    let last = g::<i32>(base + 0xC).read();
    let changed = last != key;
    let mut tmp = timer;
    let mut alpha = x(k(K_255));
    if changed {
        match state {
            0 => {
                fade = 0;
                timer = 5;
                state = 2;
                tmp = 5;
            }
            1 | 3 => {
                timer = 5;
                state = 2;
                tmp = 5;
            }
            _ => {}
        }
    }
    if state != 0 && state != 5 {
        match state {
            1 => {
                fade = 1000;
                alpha = x(k(K_255));
                if !xi(tmp).fcom(x(k(K_10000))).test_any(0x41) {
                    state = 3;
                    fade = 3000;
                }
            }
            2 => {
                fade = fade.wrapping_add(fade_step(K_1000));
                if !xi(fade).fcom(x(k(K_1000))).test_any(0x41) {
                    fade = 1000;
                    state = 1;
                }
                alpha = xi(fade).fmul_m(k(K_0_001)).fmul_m(k(K_255));
            }
            3 => {
                fade = fade.wrapping_add(fade_step(K_M1000));
                if !xi(fade).fcom(x(k(K_ZERO))).test_parity(0x05) {
                    fade = 0;
                    state = 0;
                }
                alpha = xi(fade).fmul_m(k(K_0_001)).fmul_m(k(K_255));
            }
            _ => {}
        }
        timer = timer.wrapping_add(fade_step(K_1000));
    }
    if changed && !with_alpha {
        // energy machine: the last key is stored before the state block (0x58ED59)
        g::<i32>(base + 0xC).write(key);
    }
    g::<i32>(base).write(state);
    g::<i32>(base + 8).write(timer);
    g::<i32>(base + 4).write(fade);
    if with_alpha {
        if !alpha.fcom(x(k(K_ZERO))).test_parity(0x05) {
            alpha = x(k(K_ZERO));
        } else if !alpha.fcom(x(k(K_255))).test_any(0x41) {
            alpha = x(k(K_255));
        }
        if changed {
            g::<i32>(base + 0xC).write(new_last());
        }
    }
    (state, alpha)
}

/// The breath-bar test of one player: swimming, or in a vehicle whose `+0x40 & 0x100` and `+0x42B & 0x40`
/// are set (`true` = show), else `None` to fall through to the breath compare.
fn breath_forced(ped: GPtr<u8>) -> bool {
    if !intelligence_get_task_swim(rd::<GPtr<u8>>(ped, 0x47C)).is_null() {
        return true;
    }
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        if !v.is_null() && rd::<u32>(v, 0x40) & 0x100 != 0 && rd::<u8>(v, 0x42B) & 0x40 != 0 {
            return true;
        }
    }
    false
}

/// `GetFatAndMuscleModifier(8) > ped->m_pPlayerData->+0x44` (breath not full; equal or NaN: no).
fn breath_not_full(ped: GPtr<u8>) -> bool {
    let data = rd::<GPtr<u8>>(ped, 0x480);
    !fat_and_muscle_modifier(8).fcom(x(rd::<f32>(data, 0x44))).test_any(0x41)
}

/// `ftol(Wf - Wf * (1/640) * c)` with `Wf` the f32 of `maximumWidth` (the bar x positions).
fn bar_x(wf: f32, c: u32) -> i32 {
    ftol32(x(wf).fmul_m(k(K_1_640)).fmul_m(k(c)).fsubr_m(wf))
}

/// `y = H * (1/448) * c` as the f32 argument `GetYPosBasedOnHealth` gets.
fn sy_times(c: u32) -> f32 {
    f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(c)))
}

/// One player-2 bar: `render(1, bar_x(Wf, xc), ftol(GetYPos(1, GetYPos(focus, H/448 * yc, 12), 12)))`.
fn player2_bar(render: fn(i32, i32, i32), xc: u32, yc: u32) {
    let wf = f32_of(max_w());
    let focus = b(PLAYER_IN_FOCUS);
    let y1 = f32_of(gy(focus, sy_times(yc), 12));
    let py = ftol32(gy(1, y1, 12));
    let px = bar_x(wf, xc);
    render(1, px, py);
}

/// `CHud::DrawPlayerInfo()` (0x58EAF0): clock, health / armour / breath bars (only while the energy-lost
/// machine is not in state 0), money (score machine) and the weapon icon and ammo (weapon machine), for the
/// focused player and, where the bytes do, player 2 (`CWorld::Players[1].m_pPed`).
///
/// Byte findings (gta-reversed has no body): the clock is `"%02d:%02d"` in `HudColour` 4 at
/// `(W - W/640 * 32, H/448 * 22)`; the health bar is at `x = W - W/640 * 141`, `y = GetYPos(focus, H/448 * 77,
/// 10)`; armour `(W - W/640 * 94, GetYPos(focus, H/448 * 48, 3))`; breath `(.. 94, GetYPos(focus, H/448 * 62,
/// 6))`; player 2's bars use `GetYPos(1, GetYPos(focus, H/448 * {194, 164, 179}, 12), 12)`. The focused
/// player's breath bar shows while swimming, in a vehicle with `+0x40 & 0x100` and `+0x42B & 0x40`, or with
/// the breath below `GetFatAndMuscleModifier(8)` for 500 ms after `m_LastBreathTime`; player 2's without the
/// 500 ms window. Money: `"$%08d"` in colour 1 or `"-$%07d"` in colour 0 (of `abs(money)`, re-read), at
/// `(W - W/640 * 32, GetYPos(focus, H/448 * 89, 12))`. Weapon icon at `x = W - (W/640 * 32 + W * 0.17343046)`,
/// `y = H/448 * 20`; player 2's at `W - (W/640 * 32 + 111)` (111 unscaled), `GetYPos(focus, H/448 * 138, 12)`;
/// ammo at `x = (W - (W * 0.17343046 + W/640 * 32)) + W/640 * 47 * 0.5`, `y = H/448 * 20 + H/448 * 43`
/// (player 2: `GetYPos(focus, H/448 * 138 + H/448 * 43, 12)`). The weapon machine's last key is the type
/// re-read after the step. Player 1's ped pointer is read once at entry; no null test (as the bytes).
#[hook(addr = 0x58EAF0, conv = cdecl, name = "CHud::DrawPlayerInfo", expect = "81 EC A0 01 00 00")]
pub fn draw_player_info() {
    let ped = g::<GPtr<u8>>(PLAYERS + usize::from(b(PLAYER_IN_FOCUS)) * 0x190).read();
    let mut buf = [0u8; 0xC8];
    let mut gbuf = [0u8; 0xC8];
    if g::<i32>(DRAW_CLOCK).read() == 1 {
        font_set_background(0, 0);
        let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_1_1)));
        let w = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_0_55)));
        font_set_scale(w.to_bits(), h.to_bits());
        font_set_proportional(0);
        font_set_font_style(3);
        font_set_orientation(2);
        font_set_right_justify_wrap(0);
        font_set_edge(2);
        font_set_drop_color(rgba(0, 0, 0, 0xFF));
        let minutes = i32::from(b(CLOCK_MINUTES));
        let hours = i32::from(b(CLOCK_HOURS));
        // samp's clock-off patch zeroes the format: the clock prints as an empty string.
        let mut no_fmt = [0u8; 12];
        let fmt = if sa_core::samp::toggled(sa_core::samp::toggle::NO_GAME_CLOCK) {
            local(&mut no_fmt)
        } else {
            g(STR_CLOCK_FMT)
        };
        sprintf2(local(&mut buf), fmt, hours, minutes);
        ascii_to_gxt_char(local(&mut buf), local(&mut gbuf));
        let mut c = [0u8; 4];
        let cp = hud_colours_get_rgb(g(HUD_COLOUR), local(&mut c), 4);
        font_set_color(rd::<u32>(cp, 0));
        let wv = max_w();
        let py = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_22)));
        let px = f32_of(x(k(K_1_640)).fmul(wv).fmul_m(k(K_32)).fsubr(wv));
        font_print_string(px, py, local(&mut gbuf));
        font_set_edge(0);
    }

    let focus_i = i32::from(b(PLAYER_IN_FOCUS));
    let energy_key = g::<i32>(PLAYERS + focus_i as usize * 0x190 + 0xE4).read();
    let (energy_state, _) = fade_machine(ENERGY_MACHINE, energy_key, false, || 0);
    if energy_state != 0 {
        // health
        let wf = f32_of(max_w());
        let focus = b(PLAYER_IN_FOCUS);
        let py = ftol32(gy(focus, sy_times(K_77), 10));
        let px = bar_x(wf, K_141);
        render_health_bar_call(focus_i, px, py);
        if !g::<GPtr<u8>>(PLAYER2_PED).read().is_null() {
            player2_bar(render_health_bar_call, K_141, K_194);
        }
        // armour
        let wf = f32_of(max_w());
        let focus = b(PLAYER_IN_FOCUS);
        let py = ftol32(gy(focus, sy_times(K_48), 3));
        let px = bar_x(wf, K_94);
        render_armor_bar_call(i32::from(focus), px, py);
        if !g::<GPtr<u8>>(PLAYER2_PED).read().is_null() {
            player2_bar(render_armor_bar_call, K_94, K_164);
        }
        // breath
        let mut show1 = false;
        let mut show2 = false;
        let now_hit = || {
            g::<u32>(LAST_BREATH_TIME).write(g::<u32>(TIME_MS).read());
        };
        if breath_forced(ped)
            || (breath_not_full(ped)
                && g::<u32>(LAST_BREATH_TIME).read().wrapping_add(500) > g::<u32>(TIME_MS).read())
        {
            show1 = true;
            now_hit();
        }
        let p2 = g::<GPtr<u8>>(PLAYER2_PED).read();
        if !p2.is_null() {
            let hit = breath_forced(p2) || {
                let p2 = g::<GPtr<u8>>(PLAYER2_PED).read();
                breath_not_full(p2)
            };
            if hit {
                show2 = true;
                now_hit();
            }
        }
        if show1 {
            let wf = f32_of(max_w());
            let focus = b(PLAYER_IN_FOCUS);
            let py = ftol32(gy(focus, sy_times(K_62), 6));
            let px = bar_x(wf, K_94);
            render_breath_bar_call(i32::from(focus), px, py);
        }
        if show2 && !g::<GPtr<u8>>(PLAYER2_PED).read().is_null() {
            player2_bar(render_breath_bar_call, K_94, K_179);
        }
    }

    // money
    let money_addr = PLAYERS + usize::from(b(PLAYER_IN_FOCUS)) * 0x190 + 0xBC;
    let money = g::<i32>(money_addr).read();
    let (score_state, alpha) = fade_machine(SCORE_MACHINE, money, true, || money);
    if score_state != 0 {
        let a = u32::from(ftol32(alpha) as u8);
        let neg = money < 0;
        let mut c = [0u8; 4];
        let cp = hud_colours_get_rgba(g(HUD_COLOUR), local(&mut c), if neg { 0 } else { 1 }, a);
        font_set_color(rd::<u32>(cp, 0));
        let m = g::<i32>(PLAYERS + usize::from(b(PLAYER_IN_FOCUS)) * 0x190 + 0xBC).read();
        let m = if m < 0 { m.wrapping_neg() } else { m };
        sprintf1(local(&mut buf), g(if neg { STR_MONEY_NEG_FMT } else { STR_MONEY_FMT }), m);
        ascii_to_gxt_char(local(&mut buf), local(&mut gbuf));
        font_set_proportional(0);
        font_set_background(0, 0);
        let h = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_1_1)));
        let w = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_0_55)));
        font_set_scale(w.to_bits(), h.to_bits());
        font_set_orientation(2);
        font_set_right_justify_wrap(0);
        font_set_font_style(3);
        font_set_drop_shadow_position(0);
        font_set_edge(2);
        font_set_drop_color(rgba(0, 0, 0, a));
        let wf = f32_of(max_w());
        let focus = b(PLAYER_IN_FOCUS);
        let py = f32_of(gy(focus, sy_times(K_89), 12));
        let px = f32_of(x(wf).fmul_m(k(K_1_640)).fmul_m(k(K_32)).fsubr_m(wf));
        font_print_string(px, py, local(&mut gbuf));
        font_set_edge(0);
    }

    // weapon
    let wkey = weapon_type(ped) as i32;
    let (weapon_state, alpha) = fade_machine(WEAPON_MACHINE, wkey, true, || weapon_type(ped) as i32);
    let alpha = f32_of(alpha);
    if weapon_state == 0 {
        return;
    }
    let wv = max_w();
    let py = ftol32(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_20)));
    let px = ftol32(x(k(K_1_640)).fmul(wv).fmul_m(k(K_32)).fadd(wv.fmul_m(k(K_0_1734))).fsubr(wv));
    draw_weapon_icon_call(ped, px, py, alpha);
    let p2 = g::<GPtr<u8>>(PLAYER2_PED).read();
    if !p2.is_null() {
        let wf = f32_of(max_w());
        let focus = b(PLAYER_IN_FOCUS);
        let py = ftol32(gy(focus, sy_times(K_138), 12));
        let px = ftol32(x(wf).fmul_m(k(K_1_640)).fmul_m(k(K_32)).fadd_m(k(K_111)).fsubr_m(wf));
        draw_weapon_icon_call(p2, px, py, alpha);
    }
    let (px, py) = ammo_pos(K_20, None);
    draw_ammo_call(ped, px, py, alpha);
    let p2 = g::<GPtr<u8>>(PLAYER2_PED).read();
    if !p2.is_null() {
        let focus = b(PLAYER_IN_FOCUS);
        let (px, py) = ammo_pos(K_138, Some(focus));
        draw_ammo_call(p2, px, py, alpha);
    }
}

/// The ammo position: `y = H/448 * yc + H/448 * 43` (through `GetYPos(focus, .., 12)` for player 2),
/// `x = (Wf - (Wf * 0.17343046 + S * 32)) + S * 47 * 0.5` with `Wf` and `S = W/640` stored as f32.
fn ammo_pos(yc: u32, focus: Option<u8>) -> (i32, i32) {
    let hy = max_h().fmul_m(k(K_1_448));
    let wv = max_w();
    let wf = f32_of(wv);
    let s = f32_of(wv.fmul_m(k(K_1_640)));
    let yv = x(k(yc)).fmul(hy).fadd(hy.fmul_m(k(K_43)));
    let py = match focus {
        None => ftol32(yv),
        Some(f) => ftol32(gy(f, f32_of(yv), 12)),
    };
    let px = ftol32(
        x(wf)
            .fmul_m(k(K_0_1734))
            .fadd(x(s).fmul_m(k(K_32)))
            .fsubr_m(wf)
            .fadd(x(s).fmul_m(k(K_47)).fmul_m(k(K_HALF))),
    );
    (px, py)
}

/// The camera mode of the active camera (u16 at `0xB6F1A8 + 0x238 * activeCam`).
fn cam_mode() -> u16 {
    g::<u16>(CAM_MODE + usize::from(b(CAMERA_ACTIVE_CAM)) * 0x238).read()
}

/// A `CRect` {left, bottom, right, top} drawn with `Sprites[1]` in white.
fn crosshair_quarter(l: f32, bt: f32, r: f32, t: f32) {
    let mut c = [0u8; 4];
    let cp = rgba_ctor(local(&mut c), 0xFF, 0xFF, 0xFF, 0xFF);
    let mut rect = [l, bt, r, t];
    sprite2d_draw_rect(g(SPRITE_CROSSHAIR), local(&mut rect), cp);
}

/// `int / 2` as `cdq; sub eax, edx; sar eax, 1` (truncation toward zero).
fn half(v: i32) -> i32 {
    v / 2
}

/// `CHud::DrawCrossHairs()` (0x58E020). Flag A (camera modes 7, 0x10, 8, 0x33, 0x22, 0x2D, 0x2E: in mode
/// 0x10 with a Hydra / Hunter, else with a non-melee weapon) and flag B (modes 0x2A, 0x28, 0x34, 0x27; or free
/// aiming without a target in modes 0x35 / 0x37 / 0x41, not in ped states 0x3B / 0x39, with weapon type
/// 0x16..0x1F, 0x20, 0x21, 0x26 or 0x25 - except mode 0x35 during a camera transition). Nothing when neither
/// is set and `CTheScripts::bDrawCrossHair` is 0. Then: the radius crosshair (B and modes 0x35 / 0x37 /
/// 0x41: four `Sprites[1]` quarters around `(W * MultX, H * MultY)` scaled by
/// `GetWeaponRadiusOnScreen()`, plus a 2x2 dot when the radius is exactly 0.2), the fixed centred crosshair
/// (`bDrawCrossHair != 2`, modes 0x22, 0x2A, 0x29, 0x2D), or the textured scope / crosshair (four
/// `RenderOneXLUSprite`s: `Sprites[2]` in modes 8, 0x10, 0x28, 0x33, 0x34 for weapons other than the
/// camera and the sniper rifle, else the weapon model's `"CROSSHAIR"` texture, 210 x 210 or for the camera /
/// `bDrawCrossHair == 2` 256 x 192). Render states as the bytes (`ZTESTENABLE` set in the texture path is
/// not restored).
///
/// Byte findings: the ped pointer (`Players[focus].m_pPed`) is not null-tested in the free-aim test; in the
/// texture path a TXD slot whose pool flag byte is negative makes the original read address 0 (an access
/// violation; the port performs the same read).
#[hook(addr = 0x58E020, conv = cdecl, name = "CHud::DrawCrossHairs", expect = "83 EC 30 0F B6 05 74 CD B7 00")]
pub fn draw_cross_hairs() {
    let ped = g::<GPtr<u8>>(PLAYERS + usize::from(b(PLAYER_IN_FOCUS)) * 0x190).read();
    let mode = cam_mode();
    let mut a = false;
    let mut bflag = false;
    if matches!(mode, 7 | 0x10 | 8 | 0x33 | 0x22 | 0x2D | 0x2E) {
        if mode == 0x10
            && !find_player_vehicle(-1, 0).is_null()
            && (rd::<u16>(find_player_vehicle(-1, 0), 0x22) == 0x208
                || rd::<u16>(find_player_vehicle(-1, 0), 0x22) == 0x1A9)
        {
            a = true;
        }
        if cam_mode() != 0x10 && !ped.is_null() {
            let slot = i32::from(rd::<i8>(ped, 0x718));
            let weapon = ped.byte_add(slot.wrapping_mul(0x1C) as isize + 0x5A0);
            if weapon_is_type_melee(weapon) == 0 {
                a = true;
            }
        }
    }
    if matches!(cam_mode(), 0x2A | 0x28 | 0x34 | 0x27) {
        bflag = true;
    }
    if rd::<u32>(ped, 0x71C) == 0 && rd::<u8>(rd::<GPtr<u8>>(ped, 0x480), 0x34) & 8 != 0 && {
        let intel = rd::<GPtr<u8>>(ped, 0x47C);
        intelligence_get_task_use_gun(intel).is_null() || {
            let t = intelligence_get_task_use_gun(rd::<GPtr<u8>>(ped, 0x47C));
            rd::<u8>(t, 0xE) == 0
        }
    } {
        let cx = cam_mode();
        if matches!(cx, 0x35 | 0x37 | 0x41) {
            let st = rd::<u32>(ped, 0x530);
            if st != 0x3B && st != 0x39 {
                let wt = weapon_type(ped) as i32;
                if (0x16..=0x1F).contains(&wt) || matches!(wt, 0x20 | 0x21 | 0x26 | 0x25) {
                    bflag = !(cx == 0x35 && b(CAMERA_TRANSITION_STATE) != 0);
                }
            }
        }
    }
    if !a && !bflag && g::<i32>(SCRIPTS_DRAW_CROSS_HAIR).read() == 0 {
        return;
    }
    rw_render_state_set(9, 2);
    rw_render_state_set(8, 0);
    let mode = cam_mode();
    if bflag && matches!(mode, 0x35 | 0x37 | 0x41) {
        // radius crosshair (0x58E24A)
        let cx = f32_of(max_w().fmul_m(g::<f32>(CHAIR_MULT_X).read()));
        let cy = f32_of(max_h().fmul_m(g::<f32>(CHAIR_MULT_Y).read()));
        let r = X87::from_f64_exact(player_ped_get_weapon_radius_on_screen_st0(ped));
        let rf = f32_of(r);
        if !r.fcom(x(k(K_0_2))).test_parity(0x44) {
            let l = f32_of(x(cx).fsub_m(k(K_1)));
            let mut c = [0u8; 4];
            let cp = rgba_ctor(local(&mut c), 0xFF, 0xFF, 0xFF, 0xFF);
            let t = f32_of(x(cy).fsub_m(k(K_1)));
            let rr = f32_of(x(cx).fadd_m(k(K_1)));
            let bt = f32_of(x(cy).fadd_m(k(K_1)));
            let mut rect = [l, bt, rr, t];
            sprite2d_draw_rect_static(local(&mut rect), cp);
        }
        let sx = max_w().fmul_m(k(K_1_640)).fmul_m(k(K_64)).fmul_m(rf);
        let sx_f = f32_of(sx);
        let sy = max_h().fmul_m(k(K_1_448)).fmul_m(k(K_64)).fmul_m(rf);
        let sy_f = f32_of(sy);
        let hx = x(sx_f).fmul_m(k(K_HALF));
        let hx_f = f32_of(hx);
        let l1 = f32_of(hx.fadd_m(cx).fsub(sx));
        let hy = x(sy_f).fmul_m(k(K_HALF));
        let hy_f = f32_of(hy);
        let t1 = f32_of(hy.fadd_m(cy).fsub(sy));
        let r1 = f32_of(x(hx_f).fadd_m(l1));
        let b1 = f32_of(x(hy_f).fadd_m(t1));
        crosshair_quarter(l1, b1, r1, t1);
        let l2 = f32_of(x(l1).fadd_m(sx_f));
        crosshair_quarter(l2, b1, r1, t1);
        let t2 = f32_of(x(t1).fadd_m(sy_f));
        crosshair_quarter(l1, b1, r1, t2);
        crosshair_quarter(l2, b1, r1, t2);
    } else if g::<i32>(SCRIPTS_DRAW_CROSS_HAIR).read() != 2 && matches!(mode, 0x22 | 0x2A | 0x29 | 0x2D) {
        // fixed centred crosshair (0x58E4DF)
        let w2 = half(g::<i32>(RS_MAX_WIDTH).read());
        let sx = max_w().fmul_m(k(K_1_640)).fmul_m(k(K_64));
        let sx_f = f32_of(sx);
        let h2 = half(g::<i32>(RS_MAX_HEIGHT).read());
        let sy = max_h().fmul_m(k(K_1_448)).fmul_m(k(K_64));
        let sy_f = f32_of(sy);
        let l = f32_of(xi(w2).fsub(sx.fmul_m(k(K_HALF))));
        let t = f32_of(xi(h2).fsub(sy.fmul_m(k(K_HALF))));
        let r = f32_of(x(sx_f).fmul_m(k(K_HALF)).fadd_m(l));
        let bt = f32_of(x(sy_f).fmul_m(k(K_HALF)).fadd_m(t));
        crosshair_quarter(l, bt, r, t);
        let l2 = f32_of(x(l).fadd_m(sx_f));
        crosshair_quarter(l2, bt, r, t);
        let t2 = f32_of(x(t).fadd_m(sy_f));
        crosshair_quarter(l, bt, r, t2);
        crosshair_quarter(l2, bt, r, t2);
    } else {
        // textured crosshair (0x58E6B2)
        rw_render_state_set(9, 2);
        let tex: GPtr<u8>;
        let (hx, hy, ox, oy): (f32, f32, f32, f32);
        let wt1 = weapon_type(find_player_ped(-1));
        if wt1 != 0x2B
            && weapon_type(find_player_ped(-1)) != 0x22
            && g::<i32>(SCRIPTS_DRAW_CROSS_HAIR).read() != 2
        {
            if !matches!(cam_mode(), 8 | 0x10 | 0x28 | 0x33 | 0x34) {
                draw_cross_hairs_end();
                return;
            }
            tex = g::<GPtr<u8>>(SPRITE2_TEXTURE).read();
            let sxv = max_w().fmul_m(k(K_1_640));
            hx = f32_of(x(k(K_24)).fmul(sxv));
            let syv = max_h().fmul_m(k(K_1_448));
            hy = f32_of(x(k(K_24)).fmul(syv));
            ox = f32_of(sxv.fmul_m(k(K_20)));
            oy = f32_of(syv.fmul_m(k(K_20)));
        } else {
            if weapon_type(find_player_ped(-1)) != 0x2B && g::<i32>(SCRIPTS_DRAW_CROSS_HAIR).read() != 2 {
                hx = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_210)));
                hy = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_210)));
            } else {
                hx = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_256)));
                hy = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_192)));
            }
            oy = 0.0;
            ox = 0.0;
            let info = get_weapon_info(weapon_type(ped), 1);
            let model = rd::<i32>(info, 0xC);
            if model <= 0 {
                draw_cross_hairs_end();
                return;
            }
            let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS.wrapping_add((model as usize).wrapping_mul(4))).read();
            let t = i32::from(rd::<i16>(mi, 0xA));
            let pool = g::<GPtr<u8>>(TXD_POOL).read();
            let flags = rd::<GPtr<u8>>(pool, 4);
            // `black_box`: the null read must stay a real read (the original faults there), not be
            // optimised on the assumption that it cannot happen
            let slot_addr = if rd::<i8>(flags, t as isize) < 0 {
                core::hint::black_box(0usize)
            } else {
                rd::<GPtr<u8>>(pool, 0).addr().wrapping_add((t as usize).wrapping_mul(12))
            };
            // with a free slot the original reads address 0 here (access violation)
            let dict = g::<GPtr<u8>>(slot_addr).read();
            if dict.is_null() {
                draw_cross_hairs_end();
                return;
            }
            let key = append_string_to_key(rd::<u32>(mi, 4), g(STR_CROSSHAIR));
            tex = rw_tex_dictionary_find_hash_named_texture(dict, key);
        }
        if tex.is_null() {
            draw_cross_hairs_end();
            return;
        }
        rw_render_state_set(6, 0);
        rw_render_state_set(2, 3);
        rw_render_state_set(1, rd::<u32>(tex, 0));
        let hhy = f32_of(x(hy).fmul_m(k(K_HALF)));
        let hhx = f32_of(x(hx).fmul_m(k(K_HALF)));
        for (ux, vy) in [(0u32, 0u32), (1, 0), (0, 1), (1, 1)] {
            let h2 = half(g::<i32>(RS_MAX_HEIGHT).read());
            let yv = if vy == 0 {
                xi(h2).fsub_m(hhy).fsub_m(oy)
            } else {
                xi(h2).fadd_m(hhy).fadd_m(oy)
            };
            let py = f32_of(yv);
            let w2 = half(g::<i32>(RS_MAX_WIDTH).read());
            let xv = if ux == 0 {
                xi(w2).fsub_m(hhx).fsub_m(ox)
            } else {
                xi(w2).fadd_m(hhx).fadd_m(ox)
            };
            let px = f32_of(xv);
            sprite_render_one_xlu_sprite(
                px,
                py,
                ONE_BITS,
                hhx.to_bits(),
                hhy.to_bits(),
                0xFF,
                0xFF,
                0xFF,
                0xFF,
                RECIP_Z_0_01,
                0xFF,
                ux,
                vy,
            );
        }
        rw_render_state_set(8, 0);
    }
    draw_cross_hairs_end();
}

/// The common end of `DrawCrossHairs` (0x58EABE): `RS(10, 5)`, `RS(11, 6)`, `RS(8, 1)`.
fn draw_cross_hairs_end() {
    rw_render_state_set(10, 5);
    rw_render_state_set(11, 6);
    rw_render_state_set(8, 1);
}

/// One label of the stats box: `PrintString(XL, (float)(i16)y, TheText.Get(key))`; returns `yF`.
fn vital_label(xl: f32, y: i32, key: usize) -> f32 {
    let yf = f32_of(xi(i32::from(y as i16)));
    let text = text_get(g(THE_TEXT), g(key));
    font_print_string(xl, yf, text);
    yf
}

/// One bar of the stats box: `DrawBarChart(XB, (float)((i16)gap + (i16)y), ftol(W/640 * 70),
/// ftol(H/448 * 10), progress, 0, 0, 1, CRGBA(0xC8, 0xC8, 0xC8, 0xFF), CRGBA(0, 0, 0, 0))`.
fn vital_bar(xb: f32, gap: i32, y: i32, progress: impl FnOnce() -> f32) {
    let zero = rgba(0, 0, 0, 0);
    let grey = rgba(0xC8, 0xC8, 0xC8, 0xFF);
    let p = progress();
    let h = ftol32(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_10))) as u32;
    let w = ftol32(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_70))) as u32;
    let by = f32_of(xi(i32::from(gap as i16).wrapping_add(i32::from(y as i16))));
    sprite2d_draw_bar_chart(xb, by, w, h, p, 0, 0, 1, grey, zero);
}

/// `GetStatValue(id) * 0.001 * 100` (stored f32): a stat row's progress.
fn stat_progress(id: u32) -> f32 {
    f32_of(stat_value(id).fmul_m(k(K_0_001)).fmul_m(k(K_100)))
}

/// `ftol(H/448 * 15 + yF)`: the next row's y.
fn vital_advance(yf: f32) -> i32 {
    ftol32(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_15)).fadd_m(yf))
}

/// `CHud::DrawVitalStats()` (0x589650): the on-foot stats box. Not with replay playback or wide screen. The
/// window (`FrontEndMenuManager.DrawWindow`, title `FEH_STA`, black 0xBE) spans `{40, T + H/448 * 127,
/// W/640 * 170 + 40, T + H/448 * 15}` with `T = H - H/448 * 140`, or up to `T` when a fifth row shows
/// (swimming, or a weapon of type 0x16..0x20; type 0x20 counts as 0x1C); the first row is at `T + H/448 * 30`
/// resp. `T + H/448 * 15`. Rows (label at `x = ftol(W/640 * 10 + 40)`, bar at `ftol(W/640 * 90 + 40)` and
/// `y + ftol(H/448 * 5)`, rows `ftol(H/448 * 15)` apart, all as int16): STAT068 (stat 0x40), then STAT225
/// (0xE1) when swimming or CURWSKL (the weapon skill: `floorf((tableB[k] * 0.1 + v) / maxV) * maxV * 0.001`
/// with `maxV = tableA[k] * tableB[k - 0x45]`, 1.0 above 999), STAT022 (0x16), STAT023 (0x17), STAT021
/// (0x15), STAT025 (0x19); then `DAY_n` (`CClock::CurrentDay`) at `(W/640 * 160 + 40, y + H/448 * 3)` in font
/// 3. gta-reversed has no body. Every value as the bytes compute it; `GetTaskSwim` is called twice.
#[hook(addr = 0x589650, conv = cdecl, name = "CHud::DrawVitalStats", expect = "A0 88 30 A4 00")]
pub fn draw_vital_stats() {
    if b(REPLAY_MODE) == 1 || b(WIDE_SCREEN_ON) != 0 {
        return;
    }
    let mut wt = weapon_type(find_player_ped(-1)) as i32;
    if wt == 0x20 {
        wt = 0x1C;
    }
    font_set_background(0, 0);
    font_set_color(rgba(0xE1, 0xE1, 0xE1, 0xFF));
    font_set_wrapx(f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_640))).to_bits());
    font_set_right_justify_wrap(0);
    font_set_proportional(1);
    let s = max_w().fmul_m(k(K_1_640));
    let xl = ftol32(x(k(K_10)).fmul(s).fadd_m(k(K_40)));
    let xb = ftol32(s.fmul_m(k(K_90)).fadd_m(k(K_40)));
    let gap = ftol32(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_5)));
    let focus_ped = g::<GPtr<u8>>(PLAYERS + usize::from(b(PLAYER_IN_FOCUS)) * 0x190).read();
    let swim = !intelligence_get_task_swim(rd::<GPtr<u8>>(focus_ped, 0x47C)).is_null();
    let small = !swim && !(0x16..=0x20).contains(&wt);
    // window
    let hv = max_h();
    let hy = x(k(K_1_448)).fmul(hv);
    let t = x(k(K_140)).fmul(hy).fsubr(hv);
    let top = if small { f32_of(hy.fmul_m(k(K_15)).fadd(t)) } else { f32_of(t) };
    let right = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_170)).fadd_m(k(K_40)));
    let bottom = f32_of(hy.fmul_m(k(K_127)).fadd(t));
    let back = rgba(0, 0, 0, 0xBE);
    let mut rect = [k(FORTY_BITS), bottom, right, top];
    draw_window_call(g(FRONT_END_MENU_MANAGER), local(&mut rect).cast(), g(STR_FEH_STA), 0, back, 0, 1);
    let hv = max_h();
    let hy = x(k(K_1_448)).fmul(hv);
    let t = x(k(K_140)).fmul(hy).fsubr(hv);
    let d = hy.fmul_m(k(K_15));
    let d = if small { d.fadd(d) } else { d };
    let mut y = ftol32(t.fadd(d));
    font_set_font_style(1);
    font_set_orientation(1);
    let sh = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_0_9)));
    let sw = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_0_35)));
    font_set_scale(sw.to_bits(), sh.to_bits());
    font_set_color(rgba(0xE1, 0xE1, 0xE1, 0xFF));
    font_set_edge(0);
    let xbf = f32_of(xi(i32::from(xb as i16)));
    let xlf = f32_of(xi(i32::from(xl as i16)));

    // STAT068
    let yf = vital_label(xlf, y, STR_STAT068);
    vital_bar(xbf, gap, y, || stat_progress(0x40));
    y = vital_advance(yf);

    let focus_ped = g::<GPtr<u8>>(PLAYERS + usize::from(b(PLAYER_IN_FOCUS)) * 0x190).read();
    if !intelligence_get_task_swim(rd::<GPtr<u8>>(focus_ped, 0x47C)).is_null() {
        let yf = vital_label(xlf, y, STR_STAT225);
        vital_bar(xbf, gap, y, || stat_progress(0xE1));
        y = vital_advance(yf);
    } else if (0x16..=0x20).contains(&wt) {
        let yf = vital_label(xlf, y, STR_CURWSKL);
        let kidx = weapon_info_get_skill_stat_index(wt as u32);
        let ta = g::<f32>(STAT_TABLE_A.wrapping_add((kidx as u32).wrapping_mul(4)) as usize).read();
        let tb_addr = STAT_TABLE_B.wrapping_add((kidx.wrapping_sub(0x45) as u32).wrapping_mul(4)) as usize;
        let max_v = f32_of(x(ta).fmul_m(g::<f32>(tb_addr).read()));
        let v = stat_value(kidx as u32);
        let r = if !v.fcom(x(k(K_999))).test_any(0x41) {
            k(K_1)
        } else {
            let q = f32_of(x(g::<f32>(tb_addr).read()).fmul_m(k(K_0_1)).fadd(v).fdiv_m(max_v));
            let fl = X87::from_f64_exact(floorf_st0(q));
            f32_of(fl.fmul_m(max_v).fmul_m(k(K_0_001)))
        };
        vital_bar(xbf, gap, y, || f32_of(x(r).fmul_m(k(K_100))));
        y = vital_advance(yf);
    }

    let yf = vital_label(xlf, y, STR_STAT022);
    vital_bar(xbf, gap, y, || stat_progress(0x16));
    y = vital_advance(yf);
    let yf = vital_label(xlf, y, STR_STAT023);
    vital_bar(xbf, gap, y, || stat_progress(0x17));
    y = vital_advance(yf);
    let yf = vital_label(xlf, y, STR_STAT021);
    vital_bar(xbf, gap, y, || stat_progress(0x15));
    y = vital_advance(yf);
    let yf = vital_label(xlf, y, STR_STAT025);
    vital_bar(xbf, gap, y, || stat_progress(0x19));
    let hy = max_h().fmul_m(k(K_1_448));
    let yd = ftol32(x(k(K_15)).fmul(hy).fadd(hy.fmul_m(k(K_3))).fadd_m(yf));

    font_set_font_style(3);
    font_set_orientation(2);
    let sh = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_0_7)));
    let sw = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_0_7)));
    font_set_scale(sw.to_bits(), sh.to_bits());
    font_set_edge(1);
    font_set_color(rgba(0xC8, 0xC8, 0xC8, 0xFF));
    font_set_drop_color(rgba(0, 0, 0, 0xFF));
    sprintf1(g(G_STRING), g(STR_DAY_FMT), i32::from(b(CLOCK_DAY)));
    let text = text_get(g(THE_TEXT), g(G_STRING));
    let py = f32_of(xi(i32::from(yd as i16)));
    let px = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_160)).fadd_m(k(K_40)));
    font_print_string(px, py, text);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_DRAW_PLAYER_INFO, HOOK_DRAW_CROSS_HAIRS, HOOK_DRAW_VITAL_STATS];
