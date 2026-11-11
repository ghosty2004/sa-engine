//! PORT-RENDER: the `FxInfo*_c` constructors. Two shapes (identical code apart from the constants):
//! vtable, the `FxInterpInfo*_c` member at +8 constructed, type word +4, key count byte +0xE; the
//! infos with keys also get `+0x14 = g_fxMan pool GetMem(4 * keys, 4)` (the key buffer). Each runs inside an SEH frame (`push -1; push handler`) the
//! port does not need (no destructor can run: nothing between the frame set-up and tear-down throws).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use super::{FX_POOL, pool_get_mem};
use crate::common::{g, wr};

sa_core::orig! {
    /// `FxInterpInfoFloat_c::FxInterpInfoFloat_c()` (0x4A8440)
    pub fn interp_float_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A8440 => super::fx_interp_info_float_ctor;
    /// `FxInterpInfoU255_c::FxInterpInfoU255_c()` (0x4A87D0)
    pub fn interp_u255_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A87D0 => super::fx_interp_info_u255_ctor;
    /// `FxInterpInfo32_c::FxInterpInfo32_c()` (0x4A8990)
    pub fn interp_32_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A8990 => super::fx_interp_info_32_ctor;
    /// `FxInterpInfo255_c::FxInterpInfo255_c()` (0x4A8B50)
    pub fn interp_255_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A8B50 => super::fx_interp_info_255_ctor;
}

/// The common part: vtable, the interp member at +8, type word +4, key count byte +0xE; returns `this`.
fn info_ctor(this: GPtr<u8>, vt: u32, interp: fn(GPtr<u8>) -> GPtr<u8>, ty: u16, keys: u8) -> GPtr<u8> {
    wr::<u32>(this, 0, vt);
    interp(this.byte_add(8));
    wr::<u16>(this, 4, ty);
    wr::<u8>(this, 0xE, keys);
    this
}

/// `FxInfoEmSize_c::FxInfoEmSize_c()` (0x4A4C20): vtable 0x85A7E8, interp 0x4A8990 at +8, type 0x1004, +0xE = 7 (key count),
/// +0x14 = `GetMem(28, 4)`.
#[hook(addr = 0x4A4C20, conv = thiscall, name = "FxInfoEmSize_c::FxInfoEmSize_c", expect = "6A FF 68 C3 8E 83 00")]
pub fn fx_info_em_size_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A7E8, interp_32_ctor, 0x1004, 7);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 28, 4));
    r
}

/// `FxInfoEmSpeed_c::FxInfoEmSpeed_c()` (0x4A4D00): vtable 0x85A7F4, interp 0x4A8990 at +8, type 0x1008, +0xE = 2 (key count),
/// +0x14 = `GetMem(8, 4)`.
#[hook(addr = 0x4A4D00, conv = thiscall, name = "FxInfoEmSpeed_c::FxInfoEmSpeed_c", expect = "6A FF 68 E3 8E 83 00")]
pub fn fx_info_em_speed_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A7F4, interp_32_ctor, 0x1008, 2);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 8, 4));
    r
}

/// `FxInfoEmDir_c::FxInfoEmDir_c()` (0x4A4DC0): vtable 0x85A800, interp 0x4A8990 at +8, type 0x1010, +0xE = 3 (key count),
/// +0x14 = `GetMem(12, 4)`.
#[hook(addr = 0x4A4DC0, conv = thiscall, name = "FxInfoEmDir_c::FxInfoEmDir_c", expect = "6A FF 68 03 8F 83 00")]
pub fn fx_info_em_dir_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A800, interp_32_ctor, 0x1010, 3);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 12, 4));
    r
}

/// `FxInfoEmAngle_c::FxInfoEmAngle_c()` (0x4A4E90): vtable 0x85A80C, interp 0x4A8440 at +8, type 0x1020, +0xE = 2 (key count),
/// +0x14 = `GetMem(8, 4)`.
#[hook(addr = 0x4A4E90, conv = thiscall, name = "FxInfoEmAngle_c::FxInfoEmAngle_c", expect = "6A FF 68 23 8F 83 00")]
pub fn fx_info_em_angle_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A80C, interp_float_ctor, 0x1020, 2);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 8, 4));
    r
}

/// `FxInfoEmLife_c::FxInfoEmLife_c()` (0x4A4F50): vtable 0x85A818, interp 0x4A87D0 at +8, type 0x1040, +0xE = 2 (key count),
/// +0x14 = `GetMem(8, 4)`.
#[hook(addr = 0x4A4F50, conv = thiscall, name = "FxInfoEmLife_c::FxInfoEmLife_c", expect = "6A FF 68 43 8F 83 00")]
pub fn fx_info_em_life_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A818, interp_u255_ctor, 0x1040, 2);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 8, 4));
    r
}

/// `FxInfoEmPos_c::FxInfoEmPos_c()` (0x4A5010): vtable 0x85A824, interp 0x4A8440 at +8, type 0x1080, +0xE = 3 (key count),
/// +0x14 = `GetMem(12, 4)`.
#[hook(addr = 0x4A5010, conv = thiscall, name = "FxInfoEmPos_c::FxInfoEmPos_c", expect = "6A FF 68 63 8F 83 00")]
pub fn fx_info_em_pos_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A824, interp_float_ctor, 0x1080, 3);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 12, 4));
    r
}

/// `FxInfoEmWeather_c::FxInfoEmWeather_c()` (0x4A50E0): vtable 0x85A830, interp 0x4A8440 at +8, type 0x1100, +0xE = 4 (key count),
/// +0x14 = `GetMem(16, 4)`.
#[hook(addr = 0x4A50E0, conv = thiscall, name = "FxInfoEmWeather_c::FxInfoEmWeather_c", expect = "6A FF 68 83 8F 83 00")]
pub fn fx_info_em_weather_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A830, interp_float_ctor, 0x1100, 4);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 16, 4));
    r
}

/// `FxInfoEmRotation_c::FxInfoEmRotation_c()` (0x4A51B0): vtable 0x85A83C, interp 0x4A8440 at +8, type 0x1200, +0xE = 2 (key count),
/// +0x14 = `GetMem(8, 4)`.
#[hook(addr = 0x4A51B0, conv = thiscall, name = "FxInfoEmRotation_c::FxInfoEmRotation_c", expect = "6A FF 68 A3 8F 83 00")]
pub fn fx_info_em_rotation_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A83C, interp_float_ctor, 0x1200, 2);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 8, 4));
    r
}

/// `FxInfoForce_c::FxInfoForce_c()` (0x4A5450): vtable 0x85A854, interp 0x4A8990 at +8, type 0x2002, +0xE = 3 (key count),
/// +0x14 = `GetMem(12, 4)`.
#[hook(addr = 0x4A5450, conv = thiscall, name = "FxInfoForce_c::FxInfoForce_c", expect = "6A FF 68 E3 8F 83 00")]
pub fn fx_info_force_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A854, interp_32_ctor, 0x2002, 3);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 12, 4));
    r
}

/// `FxInfoFriction_c::FxInfoFriction_c()` (0x4A5550): vtable 0x85A860, interp 0x4A8990 at +8, type 0x2004, +0xE = 1 (key count),
/// +0x14 = `GetMem(4, 4)`.
#[hook(addr = 0x4A5550, conv = thiscall, name = "FxInfoFriction_c::FxInfoFriction_c", expect = "6A FF 68 03 90 83 00")]
pub fn fx_info_friction_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A860, interp_32_ctor, 0x2004, 1);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 4, 4));
    r
}

/// `FxInfoAttractPt_c::FxInfoAttractPt_c()` (0x4A5650): vtable 0x85A86C, interp 0x4A8990 at +8, type 0x2008, +0xE = 4 (key count),
/// +0x14 = `GetMem(16, 4)`.
#[hook(addr = 0x4A5650, conv = thiscall, name = "FxInfoAttractPt_c::FxInfoAttractPt_c", expect = "6A FF 68 23 90 83 00")]
pub fn fx_info_attract_pt_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A86C, interp_32_ctor, 0x2008, 4);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 16, 4));
    r
}

/// `FxInfoAttractLine_c::FxInfoAttractLine_c()` (0x4A57C0): vtable 0x85A878, interp 0x4A8990 at +8, type 0x2010, +0xE = 7 (key count),
/// +0x14 = `GetMem(28, 4)`.
#[hook(addr = 0x4A57C0, conv = thiscall, name = "FxInfoAttractLine_c::FxInfoAttractLine_c", expect = "6A FF 68 43 90 83 00")]
pub fn fx_info_attract_line_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A878, interp_32_ctor, 0x2010, 7);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 28, 4));
    r
}

/// `FxInfoGroundCollide_c::FxInfoGroundCollide_c()` (0x4A5980): vtable 0x85A884, interp 0x4A8990 at +8, type 0x2020, +0xE = 3 (key count),
/// +0x14 = `GetMem(12, 4)`.
#[hook(addr = 0x4A5980, conv = thiscall, name = "FxInfoGroundCollide_c::FxInfoGroundCollide_c", expect = "6A FF 68 63 90 83 00")]
pub fn fx_info_ground_collide_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A884, interp_32_ctor, 0x2020, 3);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 12, 4));
    r
}

/// `FxInfoWind_c::FxInfoWind_c()` (0x4A5A10): vtable 0x85A890, interp 0x4A8990 at +8, type 0x2040, +0xE = 1 (key count),
/// +0x14 = `GetMem(4, 4)`.
#[hook(addr = 0x4A5A10, conv = thiscall, name = "FxInfoWind_c::FxInfoWind_c", expect = "6A FF 68 83 90 83 00")]
pub fn fx_info_wind_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A890, interp_32_ctor, 0x2040, 1);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 4, 4));
    r
}

/// `FxInfoJitter_c::FxInfoJitter_c()` (0x4A5B20): vtable 0x85A89C, interp 0x4A8990 at +8, type 0x2080, +0xE = 1 (key count),
/// +0x14 = `GetMem(4, 4)`.
#[hook(addr = 0x4A5B20, conv = thiscall, name = "FxInfoJitter_c::FxInfoJitter_c", expect = "6A FF 68 A3 90 83 00")]
pub fn fx_info_jitter_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A89C, interp_32_ctor, 0x2080, 1);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 4, 4));
    r
}

/// `FxInfoRotSpeed_c::FxInfoRotSpeed_c()` (0x4A5CB0): vtable 0x85A8A8, interp 0x4A8440 at +8, type 0x2100, +0xE = 4 (key count),
/// +0x14 = `GetMem(16, 4)`.
#[hook(addr = 0x4A5CB0, conv = thiscall, name = "FxInfoRotSpeed_c::FxInfoRotSpeed_c", expect = "6A FF 68 C3 90 83 00")]
pub fn fx_info_rot_speed_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A8A8, interp_float_ctor, 0x2100, 4);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 16, 4));
    r
}

/// `FxInfoFloat_c::FxInfoFloat_c()` (0x4A5DA0): vtable 0x85A8B4, interp 0x4A8440 at +8, type 0x2200, +0xE = 0.
#[hook(addr = 0x4A5DA0, conv = thiscall, name = "FxInfoFloat_c::FxInfoFloat_c", expect = "6A FF 68 D8 90 83 00")]
pub fn fx_info_float_ctor(this: GPtr<u8>) -> GPtr<u8> {
    info_ctor(this, 0x85A8B4, interp_float_ctor, 0x2200, 0)
}

/// `FxInfoUnderwater_c::FxInfoUnderwater_c()` (0x4A5E20): vtable 0x85A8C0, interp 0x4A87D0 at +8, type 0x2400, +0xE = 0.
#[hook(addr = 0x4A5E20, conv = thiscall, name = "FxInfoUnderwater_c::FxInfoUnderwater_c", expect = "6A FF 68 F8 90 83 00")]
pub fn fx_info_underwater_ctor(this: GPtr<u8>) -> GPtr<u8> {
    info_ctor(this, 0x85A8C0, interp_u255_ctor, 0x2400, 0)
}

/// `FxInfoColour_c::FxInfoColour_c()` (0x4A5EA0): vtable 0x85A8CC, interp 0x4A87D0 at +8, type 0x4001, +0xE = 4 (key count),
/// +0x14 = `GetMem(16, 4)`.
#[hook(addr = 0x4A5EA0, conv = thiscall, name = "FxInfoColour_c::FxInfoColour_c", expect = "6A FF 68 23 91 83 00")]
pub fn fx_info_colour_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A8CC, interp_u255_ctor, 0x4001, 4);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 16, 4));
    r
}

/// `FxInfoSize_c::FxInfoSize_c()` (0x4A5FA0): vtable 0x85A8D8, interp 0x4A8990 at +8, type 0x4002, +0xE = 4 (key count),
/// +0x14 = `GetMem(16, 4)`.
#[hook(addr = 0x4A5FA0, conv = thiscall, name = "FxInfoSize_c::FxInfoSize_c", expect = "6A FF 68 43 91 83 00")]
pub fn fx_info_size_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A8D8, interp_32_ctor, 0x4002, 4);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 16, 4));
    r
}

/// `FxInfoSpriteRect_c::FxInfoSpriteRect_c()` (0x4A60B0): vtable 0x85A8E4, interp 0x4A8B50 at +8, type 0x4004, +0xE = 4 (key count),
/// +0x14 = `GetMem(16, 4)`.
#[hook(addr = 0x4A60B0, conv = thiscall, name = "FxInfoSpriteRect_c::FxInfoSpriteRect_c", expect = "6A FF 68 63 91 83 00")]
pub fn fx_info_sprite_rect_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A8E4, interp_255_ctor, 0x4004, 4);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 16, 4));
    r
}

/// `FxInfoHeatHaze_c::FxInfoHeatHaze_c()` (0x4A61A0): vtable 0x85A8F0, interp 0x4A8B50 at +8, type 0x4008, +0xE = 0.
#[hook(addr = 0x4A61A0, conv = thiscall, name = "FxInfoHeatHaze_c::FxInfoHeatHaze_c", expect = "6A FF 68 78 91 83 00")]
pub fn fx_info_heat_haze_ctor(this: GPtr<u8>) -> GPtr<u8> {
    info_ctor(this, 0x85A8F0, interp_255_ctor, 0x4008, 0)
}

/// `FxInfoTrail_c::FxInfoTrail_c()` (0x4A6220): vtable 0x85A8FC, interp 0x4A87D0 at +8, type 0x4010, +0xE = 2 (key count),
/// +0x14 = `GetMem(8, 4)`.
#[hook(addr = 0x4A6220, conv = thiscall, name = "FxInfoTrail_c::FxInfoTrail_c", expect = "6A FF 68 A3 91 83 00")]
pub fn fx_info_trail_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A8FC, interp_u255_ctor, 0x4010, 2);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 8, 4));
    r
}

/// `FxInfoFlat_c::FxInfoFlat_c()` (0x4A6320): vtable 0x85A908, interp 0x4A8440 at +8, type 0x4020, +0xE = 9 (key count),
/// +0x14 = `GetMem(36, 4)`.
#[hook(addr = 0x4A6320, conv = thiscall, name = "FxInfoFlat_c::FxInfoFlat_c", expect = "6A FF 68 C3 91 83 00")]
pub fn fx_info_flat_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A908, interp_float_ctor, 0x4020, 9);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 36, 4));
    r
}

/// `FxInfoDir_c::FxInfoDir_c()` (0x4A6440): vtable 0x85A914, interp 0x4A8440 at +8, type 0x4040, +0xE = 3 (key count),
/// +0x14 = `GetMem(12, 4)`.
#[hook(addr = 0x4A6440, conv = thiscall, name = "FxInfoDir_c::FxInfoDir_c", expect = "6A FF 68 E3 91 83 00")]
pub fn fx_info_dir_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A914, interp_float_ctor, 0x4040, 3);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 12, 4));
    r
}

/// `FxInfoAnimTexture_c::FxInfoAnimTexture_c()` (0x4A6550): vtable 0x85A920, interp 0x4A8990 at +8, type 0x4080, +0xE = 1 (key count),
/// +0x14 = `GetMem(4, 4)`.
#[hook(addr = 0x4A6550, conv = thiscall, name = "FxInfoAnimTexture_c::FxInfoAnimTexture_c", expect = "6A FF 68 03 92 83 00")]
pub fn fx_info_anim_texture_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A920, interp_32_ctor, 0x4080, 1);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 4, 4));
    r
}

/// `FxInfoColourRange_c::FxInfoColourRange_c()` (0x4A6630): vtable 0x85A92C, interp 0x4A87D0 at +8, type 0x4100, +0xE = 7 (key count),
/// +0x14 = `GetMem(28, 4)`.
#[hook(addr = 0x4A6630, conv = thiscall, name = "FxInfoColourRange_c::FxInfoColourRange_c", expect = "6A FF 68 23 92 83 00")]
pub fn fx_info_colour_range_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A92C, interp_u255_ctor, 0x4100, 7);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 28, 4));
    r
}

/// `FxInfoSelfLit_c::FxInfoSelfLit_c()` (0x4A6760): vtable 0x85A938, interp 0x4A87D0 at +8, type 0x4200, +0xE = 0.
#[hook(addr = 0x4A6760, conv = thiscall, name = "FxInfoSelfLit_c::FxInfoSelfLit_c", expect = "6A FF 68 38 92 83 00")]
pub fn fx_info_self_lit_ctor(this: GPtr<u8>) -> GPtr<u8> {
    info_ctor(this, 0x85A938, interp_u255_ctor, 0x4200, 0)
}

/// `FxInfoColourBright_c::FxInfoColourBright_c()` (0x4A67D0): vtable 0x85A944, interp 0x4A87D0 at +8, type 0x4400, +0xE = 5 (key count),
/// +0x14 = `GetMem(20, 4)`.
#[hook(addr = 0x4A67D0, conv = thiscall, name = "FxInfoColourBright_c::FxInfoColourBright_c", expect = "6A FF 68 63 92 83 00")]
pub fn fx_info_colour_bright_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A944, interp_u255_ctor, 0x4400, 5);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 20, 4));
    r
}

/// `FxInfoSmoke_c::FxInfoSmoke_c()` (0x4A68E0): vtable 0x85A950, interp 0x4A87D0 at +8, type 0x8001, +0xE = 8 (key count),
/// +0x14 = `GetMem(32, 4)`.
#[hook(addr = 0x4A68E0, conv = thiscall, name = "FxInfoSmoke_c::FxInfoSmoke_c", expect = "6A FF 68 83 92 83 00")]
pub fn fx_info_smoke_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A950, interp_u255_ctor, 0x8001, 8);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 32, 4));
    r
}

/// `FxInfoEmRate_c::FxInfoEmRate_c()` (0x4A6A00): vtable 0x85A95C, interp 0x4A8440 at +8, type 0x1001, +0xE = 1 (key count),
/// +0x14 = `GetMem(4, 4)`.
#[hook(addr = 0x4A6A00, conv = thiscall, name = "FxInfoEmRate_c::FxInfoEmRate_c", expect = "6A FF 68 A3 92 83 00")]
pub fn fx_info_em_rate_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let r = info_ctor(this, 0x85A95C, interp_float_ctor, 0x1001, 1);
    wr(this, 0x14, pool_get_mem(g(FX_POOL), 4, 4));
    r
}

pub const HOOKS: &[HookReg] = &[
    HOOK_FX_INFO_EM_SIZE_CTOR,
    HOOK_FX_INFO_EM_SPEED_CTOR,
    HOOK_FX_INFO_EM_DIR_CTOR,
    HOOK_FX_INFO_EM_ANGLE_CTOR,
    HOOK_FX_INFO_EM_LIFE_CTOR,
    HOOK_FX_INFO_EM_POS_CTOR,
    HOOK_FX_INFO_EM_WEATHER_CTOR,
    HOOK_FX_INFO_EM_ROTATION_CTOR,
    HOOK_FX_INFO_FORCE_CTOR,
    HOOK_FX_INFO_FRICTION_CTOR,
    HOOK_FX_INFO_ATTRACT_PT_CTOR,
    HOOK_FX_INFO_ATTRACT_LINE_CTOR,
    HOOK_FX_INFO_GROUND_COLLIDE_CTOR,
    HOOK_FX_INFO_WIND_CTOR,
    HOOK_FX_INFO_JITTER_CTOR,
    HOOK_FX_INFO_ROT_SPEED_CTOR,
    HOOK_FX_INFO_FLOAT_CTOR,
    HOOK_FX_INFO_UNDERWATER_CTOR,
    HOOK_FX_INFO_COLOUR_CTOR,
    HOOK_FX_INFO_SIZE_CTOR,
    HOOK_FX_INFO_SPRITE_RECT_CTOR,
    HOOK_FX_INFO_HEAT_HAZE_CTOR,
    HOOK_FX_INFO_TRAIL_CTOR,
    HOOK_FX_INFO_FLAT_CTOR,
    HOOK_FX_INFO_DIR_CTOR,
    HOOK_FX_INFO_ANIM_TEXTURE_CTOR,
    HOOK_FX_INFO_COLOUR_RANGE_CTOR,
    HOOK_FX_INFO_SELF_LIT_CTOR,
    HOOK_FX_INFO_COLOUR_BRIGHT_CTOR,
    HOOK_FX_INFO_SMOKE_CTOR,
    HOOK_FX_INFO_EM_RATE_CTOR,
];
