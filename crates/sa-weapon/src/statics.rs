//! Static initialisers / `atexit` destructors of the WEAPON globals (CRT `__xc` entries; names from
//! CAT-6 `__init_*` / `__dtor_*`), as sa-world's `world::statics`: array constructions through the
//! CRT's `eh vector constructor iterator` with the original constructor/destructor addresses.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

sa_core::globals! {
    list GLOBALS;
    /// 0xC89190: `0.005f * 0.0625f`
    pub static G_C89190 @ 0xC89190: f32 = 0.0;
    /// 0xC89678: `0.005f * 0.0625f`
    pub static G_C89678 @ 0xC89678: f32 = 0.0;
    /// 0xC8A7D8: `0.277778f / 50.0f`
    pub static G_C8A7D8 @ 0xC8A7D8: f32 = 0.0;
}

orig! {
    /// `CAEExplosionAudioEntity::CAEExplosionAudioEntity()` (0x5074B0, thiscall)
    pub fn explosion_audio_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x5074B0 => unported;
    /// the `CAEExplosionAudioEntity` destructor body (0x506B90, thiscall)
    pub fn explosion_audio_dtor(this: GPtr<u8>) = thiscall 0x506B90 => unported;
    /// `CFireManager::Constructor()` (0x539DA0)
    pub fn fire_manager_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x539DA0 => crate::fire::manager_constructor;
    /// `CFireManager::Destructor()` (0x538BB0)
    pub fn fire_manager_dtor(this: GPtr<u8>) = thiscall 0x538BB0 => crate::fire::manager_destructor;
    /// `CColModel::CColModel()` (0x40FB60, thiscall)
    pub fn col_model_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x40FB60 => unported;
    /// `CColModel::~CColModel()` (0x40F700, thiscall)
    pub fn col_model_dtor(this: GPtr<u8>) = thiscall 0x40F700 => unported;
}

const K_005: f32 = f32::from_bits(0x3BA3_D70A);

/// `__init_CWaterCannons__aCannons` (0x855420).
#[hook(addr = 0x855420, conv = cdecl, name = "__init_CWaterCannons__aCannons", expect = "68 30 8B 72 00")]
pub fn init_water_cannons() {
    eh_vector_ctor(g(0xC80740), 0x3CC, 3, 0x728B10, 0x728B30);
    atexit(0x856BF0);
}

/// `__dtor_CWaterCannons__aCannons` (0x856BF0).
#[hook(addr = 0x856BF0, conv = cdecl, name = "__dtor_CWaterCannons__aCannons", expect = "68 30 8B 72 00")]
pub fn dtor_water_cannons() {
    eh_vector_dtor(g(0xC80740), 0x3CC, 3, 0x728B30);
}

/// `__init_g_c89190` (0x855A00): `0.005 * 0.0625`.
#[hook(addr = 0x855A00, conv = cdecl, name = "__init_g_c89190", expect = "D9 05 DC 2B 87 00")]
pub fn init_g_c89190() {
    G_C89190.set(x(K_005).fmul_m(0.0625).to_f32());
}

/// `__init_g_c89678` (0x855AC0): `0.005 * 0.0625`.
#[hook(addr = 0x855AC0, conv = cdecl, name = "__init_g_c89678", expect = "D9 05 04 2C 87 00")]
pub fn init_g_c89678() {
    G_C89678.set(x(K_005).fmul_m(0.0625).to_f32());
}

/// `__init_g_c8a7d8` (0x855C20): `[0x858630] / 50.0`.
#[hook(addr = 0x855C20, conv = cdecl, name = "__init_g_c8a7d8", expect = "D9 05 30 86 85 00")]
pub fn init_g_c8a7d8() {
    G_C8A7D8.set(x(f32::from_bits(0x3E8E_38EB)).fdiv_m(50.0).to_f32());
}

/// `__init_gCrossHair` (0x855D20): 2 `CWeaponEffects` (0x2C) at 0xC8A838.
#[hook(addr = 0x855D20, conv = cdecl, name = "__init_gCrossHair", expect = "68 A0 2A 74 00")]
pub fn init_cross_hair() {
    eh_vector_ctor(g(0xC8A838), 0x2C, 2, 0x742A90, 0x742AA0);
    atexit(0x856C40);
}

/// `__dtor_gCrossHair` (0x856C40).
#[hook(addr = 0x856C40, conv = cdecl, name = "__dtor_gCrossHair", expect = "68 A0 2A 74 00")]
pub fn dtor_cross_hair() {
    eh_vector_dtor(g(0xC8A838), 0x2C, 2, 0x742AA0);
}

/// `__init_g_c8aaa4` (0x855D70): `1.0 / 50.0`.
#[hook(addr = 0x855D70, conv = cdecl, name = "__init_g_c8aaa4", expect = "D9 05 24 86 85 00")]
pub fn init_g_c8aaa4() {
    crate::weapon_info::G_C8AAA4.set(x(1.0).fdiv_m(50.0).to_f32());
}

/// `__init_CWeaponInfo__aWeaponInfo` (0x855E10): 80 `CWeaponInfo` (0x70) at 0xC8AAB8.
#[hook(addr = 0x855E10, conv = cdecl, name = "__init_CWeaponInfo__aWeaponInfo", expect = "68 40 3C 74 00")]
pub fn init_weapon_infos() {
    eh_vector_ctor(g(0xC8AAB8), 0x70, 0x50, 0x743C30, 0x743C40);
    atexit(0x856C60);
}

/// `__dtor_CWeaponInfo__aWeaponInfo` (0x856C60).
#[hook(addr = 0x856C60, conv = cdecl, name = "__dtor_CWeaponInfo__aWeaponInfo", expect = "68 40 3C 74 00")]
pub fn dtor_weapon_infos() {
    eh_vector_dtor(g(0xC8AAB8), 0x70, 0x50, 0x743C40);
}

/// `__init_CExplosion__m_ExplosionAudioEntity` (0x8559E0): the audio entity at 0xC888D0.
#[hook(addr = 0x8559E0, conv = cdecl, name = "__init_CExplosion__m_ExplosionAudioEntity", expect = "B9 D0 88 C8 00")]
pub fn init_explosion_audio() {
    explosion_audio_ctor(g(0xC888D0));
    atexit(0x856C20);
}

/// `__dtor_CExplosion__m_ExplosionAudioEntity` (0x856C20): tail jump to the destructor body.
#[hook(addr = 0x856C20, conv = cdecl, name = "__dtor_CExplosion__m_ExplosionAudioEntity", expect = "B9 D0 88 C8 00")]
pub fn dtor_explosion_audio() {
    explosion_audio_dtor(g(0xC888D0));
}

/// `__init_gFireManager` (0x84DE70).
#[hook(addr = 0x84DE70, conv = cdecl, name = "__init_gFireManager", expect = "B9 80 1F B7 00")]
pub fn init_fire_manager() {
    fire_manager_ctor(g(crate::fire::FIRE_MANAGER));
    atexit(0x856420);
}

/// `__dtor_gFireManager` (0x856420): tail jump to `CFireManager::Destructor`.
#[hook(addr = 0x856420, conv = cdecl, name = "__dtor_gFireManager", expect = "B9 80 1F B7 00")]
pub fn dtor_fire_manager() {
    fire_manager_dtor(g(crate::fire::FIRE_MANAGER));
}

/// `__init_CWeapon__ms_PelletTestCol` (0x855C40).
#[hook(addr = 0x855C40, conv = cdecl, name = "__init_CWeapon__ms_PelletTestCol", expect = "B9 DC A7 C8 00")]
pub fn init_pellet_test_col() {
    col_model_ctor(g(crate::weapon::PELLET_TEST_COL));
    atexit(0x856C30);
}

/// `__dtor_CWeapon__ms_PelletTestCol` (0x856C30): tail jump to `CColModel::~CColModel`.
#[hook(addr = 0x856C30, conv = cdecl, name = "__dtor_CWeapon__ms_PelletTestCol", expect = "B9 DC A7 C8 00")]
pub fn dtor_pellet_test_col() {
    col_model_dtor(g(crate::weapon::PELLET_TEST_COL));
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_WATER_CANNONS,
    HOOK_DTOR_WATER_CANNONS,
    HOOK_INIT_G_C89190,
    HOOK_INIT_G_C89678,
    HOOK_INIT_G_C8A7D8,
    HOOK_INIT_CROSS_HAIR,
    HOOK_DTOR_CROSS_HAIR,
    HOOK_INIT_G_C8AAA4,
    HOOK_INIT_WEAPON_INFOS,
    HOOK_DTOR_WEAPON_INFOS,
    HOOK_INIT_EXPLOSION_AUDIO,
    HOOK_DTOR_EXPLOSION_AUDIO,
    HOOK_INIT_FIRE_MANAGER,
    HOOK_DTOR_FIRE_MANAGER,
    HOOK_INIT_PELLET_TEST_COL,
    HOOK_DTOR_PELLET_TEST_COL,
];
