//! `CWeaponEffects` (PORT-WEAPON-PED): the cross-hair array helpers. Ported from the 1.0 US bytes
//! (gta-reversed `WeaponEffects.cpp`, `docs/re/specs/WEAPON/CWeaponEffects.md`).
//!
//! `gCrossHair[2]` (0x2C) at 0xC8A838: +0 active, +4 clear time, +8 position, +0x14..+0x17 colour,
//! +0x18 size, +0x1C ..., +0x24 rotation / lock (f32), +0x28 byte. Textures at 0xC8A810..0xC8A818.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

pub const CROSS_HAIR_SIZE: usize = 0x2C;
pub const NUM_CROSS_HAIRS: usize = 2;
const _: () = assert!(CROSS_HAIR_SIZE == sa_core::layouts::size::CWeaponEffects);

sa_core::globals! {
    list GLOBALS;
    /// the three cross-hair textures (`RwTexture*`): 0xC8A810, 0xC8A814, 0xC8A818
    pub static TEXTURES @ 0xC8A810: [GPtr<u8>; 3] = [GPtr::NULL; 3];
    /// `gCrossHair[2]`
    pub static CROSS_HAIRS @ 0xC8A838: [u8; NUM_CROSS_HAIRS * CROSS_HAIR_SIZE] = [0; NUM_CROSS_HAIRS * CROSS_HAIR_SIZE];
}

orig! {
    /// `CTxdStore::PushCurrentTxd()` (0x7316A0)
    pub fn txd_push_current() = cdecl 0x7316A0 => unported;
    /// `CTxdStore::PopCurrentTxd()` (0x7316B0)
    pub fn txd_pop_current() = cdecl 0x7316B0 => unported;
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    pub fn txd_find_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => unported;
    /// `CTxdStore::SetCurrentTxd(int32)` (0x7319C0)
    pub fn txd_set_current(slot: i32) = cdecl 0x7319C0 => unported;
    /// `RwTextureRead(const char* name, const char* mask)` (0x7F3AC0)
    pub fn rw_texture_read(name: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3AC0 => unported;
    /// `RwTextureDestroy(RwTexture*)` (0x7F3820)
    pub fn rw_texture_destroy(t: GPtr<u8>) -> i32 = cdecl 0x7F3820 => unported;
}

pub fn cross_hair(i: i32) -> GPtr<u8> {
    CROSS_HAIRS.ptr().cast::<u8>().byte_add((i as isize).wrapping_mul(CROSS_HAIR_SIZE as isize))
}

fn texture(k: usize) -> GPtr<GPtr<u8>> {
    TEXTURES.ptr().cast::<GPtr<u8>>().wrapping_index(k as i32)
}

/// `CWeaponEffects::IsLockedOn(int32 i)` (0x742BD0): `_ftol(+0x24)` (tail jump; EAX of `_ftol`).
#[hook(addr = 0x742BD0, conv = cdecl, name = "CWeaponEffects::IsLockedOn", expect = "8B 44 24 04 6B C0 2C")]
pub fn is_locked_on(i: i32) -> u32 {
    ftol32(x(rf(cross_hair(i), 0x24))) as u32
}

/// `CWeaponEffects::MarkTarget(int32 i, CVector pos, uint8 r, g, b, a, float size, uint8 flag)`
/// (0x742BF0; split chunk 0x401E73): position, colour, active, size, clear time -1, +0x28.
#[hook(addr = 0x742BF0, conv = cdecl, name = "CWeaponEffects::MarkTarget", expect = "8B 44 24 04 8B 54 24 08")]
#[allow(clippy::too_many_arguments)]
pub fn mark_target(i: i32, px: u32, py: u32, pz: u32, r: u8, gg: u8, b: u8, a: u8, size: u32, flag: u8) {
    let c = cross_hair(i);
    wu(c, 8, px);
    wu(c, 0xC, py);
    wu(c, 0x10, pz);
    wb(c, 0x14, r);
    wb(c, 0x15, gg);
    wb(c, 0x16, b);
    wb(c, 0x17, a);
    wb(c, 0, 1);
    wu(c, 0x18, size);
    wu(c, 4, u32::MAX);
    wb(c, 0x28, flag);
}

/// `CWeaponEffects::ClearCrossHair(int32 i)` (0x742C60): clear time = now + 400.
#[hook(addr = 0x742C60, conv = cdecl, name = "CWeaponEffects::ClearCrossHair", expect = "8B 4C 24 04 A1 84 CB B7 00")]
pub fn clear_cross_hair(i: i32) {
    wu(cross_hair(i), 4, time_ms().wrapping_add(400));
}

/// `CWeaponEffects::ClearCrossHairs()` (0x742C80): both inactive.
#[hook(addr = 0x742C80, conv = cdecl, name = "CWeaponEffects::ClearCrossHairs", expect = "B8 38 A8 C8 00")]
pub fn clear_cross_hairs() {
    for i in 0..NUM_CROSS_HAIRS as i32 {
        wb(cross_hair(i), 0, 0);
    }
}

/// `CWeaponEffects::ClearCrossHairImmediately(int32 i)` (0x742CA0): clear time now - 100, inactive.
#[hook(addr = 0x742CA0, conv = cdecl, name = "CWeaponEffects::ClearCrossHairImmediately", expect = "8B 44 24 04 8B 0D 84 CB B7 00")]
pub fn clear_cross_hair_immediately(i: i32) {
    let c = cross_hair(i);
    wu(c, 4, time_ms().wrapping_sub(100));
    wb(c, 0, 0);
}

/// `CWeaponEffects::ClearCrossHairsImmediately()` (0x742CC0).
#[hook(addr = 0x742CC0, conv = cdecl, name = "CWeaponEffects::ClearCrossHairsImmediately", expect = "8B 0D 84 CB B7 00")]
pub fn clear_cross_hairs_immediately() {
    let t = time_ms().wrapping_sub(100);
    for i in 0..NUM_CROSS_HAIRS as i32 {
        let c = cross_hair(i);
        wu(c, 4, t);
        wb(c, 0, 0);
    }
}

/// `CWeaponEffects::Init()` (0x742AB0): both cross-hairs reset (position 0, inactive, colour
/// (255, 0, 0, 127), size 1.0, +0x1C/+4/+0x24 0, +0x28 0), then the three textures from the
/// "particle" TXD (0x85C648).
#[hook(addr = 0x742AB0, conv = cdecl, name = "CWeaponEffects::Init", expect = "83 EC 0C 53 56")]
pub fn init() {
    for i in 0..NUM_CROSS_HAIRS as i32 {
        let c = cross_hair(i);
        wu(c, 8, 0);
        wb(c, 0, 0);
        wb(c, 0x14, 0xFF);
        wb(c, 0x15, 0);
        wb(c, 0x16, 0);
        wb(c, 0x17, 0x7F);
        wu(c, 0x18, 0x3F80_0000);
        wu(c, 0x1C, 0);
        wu(c, 4, 0);
        wb(c, 0x28, 0);
        wu(c, 0x24, 0);
        wu(c, 0xC, 0);
        wu(c, 0x10, 0);
    }
    txd_push_current();
    let slot = txd_find_slot(g(0x85C648));
    txd_set_current(slot);
    texture(2).write(rw_texture_read(g(0x872CC4), g(0x872CD0)));
    texture(0).write(rw_texture_read(g(0x872CB4), g(0x872CBC)));
    texture(1).write(rw_texture_read(g(0x872C9C), g(0x872CA8)));
    txd_pop_current();
}

/// `CWeaponEffects::Shutdown()` (0x742B80): destroys the textures 0xC8A814, 0xC8A810, 0xC8A818.
#[hook(addr = 0x742B80, conv = cdecl, name = "CWeaponEffects::Shutdown", expect = "A1 14 A8 C8 00")]
pub fn shutdown() {
    for k in [1usize, 0, 2] {
        rw_texture_destroy(texture(k).read());
        texture(k).write(GPtr::NULL);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_IS_LOCKED_ON,
    HOOK_MARK_TARGET,
    HOOK_CLEAR_CROSS_HAIR,
    HOOK_CLEAR_CROSS_HAIRS,
    HOOK_CLEAR_CROSS_HAIR_IMMEDIATELY,
    HOOK_CLEAR_CROSS_HAIRS_IMMEDIATELY,
    HOOK_INIT,
    HOOK_SHUTDOWN,
];
