//! `CWeaponInfo` (PORT-WEAPON-PED). Ported from the 1.0 US bytes (gta-reversed `WeaponInfo.cpp` and
//! `docs/re/specs/WEAPON/CWeaponInfo*.md` as reading aids).
//!
//! `CWeaponInfo` (0x70): +0 fire type, +4 target range, +8 weapon range, +0xC/+0x10 model ids, +0x14
//! slot, +0x18 flags (bit 11 "reload 2 start", 12 twin pistol... as read below), +0x1C/+0x20 ammo /
//! damage, ..., +0x30 skill-ish int used by `GetTargetHeadRange`, +0x6C anim group (int16, index into
//! `ms_aWeaponAimOffsets`), +0x6E/+0x6F bytes. `aWeaponInfo[80]` at 0xC8AAB8; skill 0 (poor) = index
//! `type + 25`, 1 (std) = `type`, 2 (pro) = `type + 36`, 3 (special) = `type + 47`.
//! `ms_aWeaponAimOffsets[21]` (0x18 each) at 0xC8A8A8: +0x10/+0x12/+0x14/+0x16 the reload anim
//! loop times (int16).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

pub const WEAPON_INFO_SIZE: usize = 0x70;
pub const NUM_WEAPON_INFOS: usize = 80;
pub const AIM_OFFSET_SIZE: usize = 0x18;
pub const NUM_AIM_OFFSETS: usize = 21;

const _: () = assert!(WEAPON_INFO_SIZE == sa_core::layouts::size::CWeaponInfo);

sa_core::globals! {
    list GLOBALS;
    /// `CWeaponInfo::ms_aWeaponAimOffsets` (21 x 0x18)
    pub static AIM_OFFSETS @ 0xC8A8A8: [u8; NUM_AIM_OFFSETS * AIM_OFFSET_SIZE] = [0; NUM_AIM_OFFSETS * AIM_OFFSET_SIZE];
    /// 0xC8AAA4: `1.0f / 50.0f` (set by the static initialiser 0x855D70)
    pub static G_C8AAA4 @ 0xC8AAA4: f32 = 0.0;
    /// `aWeaponInfo[80]`
    pub static WEAPON_INFOS @ 0xC8AAB8: [u8; NUM_WEAPON_INFOS * WEAPON_INFO_SIZE] = [0; NUM_WEAPON_INFOS * WEAPON_INFO_SIZE];
}

/// `aWeaponInfo[i]` (no bounds check, as the original's `imul`/`add`).
pub fn info(i: i32) -> GPtr<u8> {
    WEAPON_INFOS.ptr().cast::<u8>().byte_add((i as isize).wrapping_mul(WEAPON_INFO_SIZE as isize))
}

fn aim_offsets() -> GPtr<u8> {
    AIM_OFFSETS.ptr().cast::<u8>()
}

/// 0x86A908.. the fire-type names as `.rdata` holds them (with their NUL; compared with `repe cmpsb`).
const FIRE_TYPE_NAMES: [&[u8]; 6] =
    [b"MELEE\0", b"INSTANT_HIT\0", b"PROJECTILE\0", b"AREA_EFFECT\0", b"CAMERA\0", b"USE\0"];

orig! {
    /// `CWeaponInfo::LoadWeaponData()` (0x5BE670)
    pub fn load_weapon_data() = cdecl 0x5BE670 => crate::weapon_data::load_weapon_data;
}

/// `repe cmpsb` of `n = name.len()` bytes: equal iff every byte matches (stops at the first mismatch).
fn cmps(s: GPtr<u8>, name: &[u8]) -> bool {
    for (k, &c) in name.iter().enumerate() {
        if s.byte_add(k as isize).read() != c {
            return false;
        }
    }
    true
}

/// `CWeaponInfo::FindWeaponFireType(const char*)` (0x5BCF30, cdecl): MELEE 0, INSTANT_HIT 1,
/// PROJECTILE 2, AREA_EFFECT 3, CAMERA 4, USE 5, anything else 1.
#[hook(addr = 0x5BCF30, conv = cdecl, name = "CWeaponInfo::FindWeaponFireType", expect = "8B 44 24 04 56")]
pub fn find_weapon_fire_type(s: GPtr<u8>) -> u32 {
    for (i, n) in FIRE_TYPE_NAMES.iter().enumerate().take(5) {
        if cmps(s, n) {
            return i as u32;
        }
    }
    // `sete al; lea eax, [eax*4 + 1]`
    if cmps(s, FIRE_TYPE_NAMES[5]) { 5 } else { 1 }
}

/// `CWeaponInfo::GetCrouchReloadAnimationID()` (0x685700, thiscall): 0xE3 when flags bits 13 and 12
/// are both set, else 0.
#[hook(addr = 0x685700, conv = thiscall, name = "CWeaponInfo::GetCrouchReloadAnimationID", expect = "8B 41 18 8B C8", this = "CWeaponInfo")]
pub fn get_crouch_reload_animation_id(this: GPtr<u8>) -> u32 {
    let f = ru(this, 0x18);
    if (f >> 13) & 1 != 0 && (f >> 12) & 1 != 0 { 0xE3 } else { 0 }
}

/// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60, cdecl; split entry: `jmp
/// 0x4075C9`, which loads EAX = 0x2F before jumping back). Index `(int16)(type + k)` with k = 25 / 0 /
/// 36 / 47 for skill 0..3; any other skill uses the EAX of the split chunk: index 0x2F (original bug).
#[hook(addr = 0x743C60, conv = cdecl, name = "CWeaponInfo::GetWeaponInfo-original", expect = "8A 4C 24 08 84 C9")]
pub fn get_weapon_info(ty: i32, skill: u8) -> GPtr<u8> {
    let idx = match skill {
        0 => ty.wrapping_add(0x19),
        1 => ty,
        2 => ty.wrapping_add(0x24),
        3 => ty.wrapping_add(0x2F),
        _ => 0x2F,
    };
    info(i32::from(idx as i16))
}

/// `CWeaponInfo::GetSkillStatIndex(eWeaponType)` (0x743CD0, cdecl): 22..31 → type - 22 + 69; 32 → 75;
/// 33 → 81; otherwise -1.
#[hook(addr = 0x743CD0, conv = cdecl, name = "CWeaponInfo::GetSkillStatIndex", expect = "8B 44 24 04 83 F8 16")]
pub fn get_skill_stat_index(ty: i32) -> i32 {
    if !(0x16..=0x20).contains(&ty) {
        return -1;
    }
    if ty <= 0x1F {
        return ty - 0x16 + 0x45;
    }
    match ty - 0x20 {
        0 => 6 + 0x45,
        1 => 0xC + 0x45,
        _ => ty + 0x45, // unreachable for 0x16..=0x20, kept as the bytes
    }
}

/// `CWeaponInfo::FindWeaponType(const char*)` (0x743D10, cdecl): the first of the 49 names
/// (0x8D6150) equal to `name` under `_stricmp`, else 0.
#[hook(addr = 0x743D10, conv = cdecl, name = "CWeaponInfo::FindWeaponType", expect = "56 57 8B 7C 24 0C")]
pub fn find_weapon_type(name: GPtr<u8>) -> i32 {
    for i in 0..=0x30i32 {
        let s = g::<GPtr<u8>>(0x8D6150 + i as usize * 4).read();
        if stricmp(name, s) == 0 {
            return i;
        }
    }
    0
}

/// `CWeaponInfo::GetTargetHeadRange()` (0x743D50, thiscall, ST0): `(float)(+0x30 + 2) * +8 * 0.04f`.
#[hook(addr = 0x743D50, conv = thiscall, name = "CWeaponInfo::GetTargetHeadRange", expect = "51 8B 41 30 83 C0 02", this = "CWeaponInfo")]
pub fn get_target_head_range(this: GPtr<u8>) -> X87 {
    xi(ru(this, 0x30).wrapping_add(2) as i32).fmul_m(rf(this, 8)).fmul_m(cf(0x3D23_D70A))
}

/// `CWeaponInfo::GetWeaponReloadTime()` (0x743D70, thiscall): flag 12 → 1000 (+1000 with flag 11);
/// flag 15 → 1000; else the largest of the anim group's four reload loop times + 100 that exceeds
/// 400 in the order +0x10, +0x14, +0x12, +0x16, else 400.
#[hook(addr = 0x743D70, conv = thiscall, name = "CWeaponInfo::GetWeaponReloadTime", expect = "8B 41 18 8B D0", this = "CWeaponInfo")]
pub fn get_weapon_reload_time(this: GPtr<u8>) -> u32 {
    let f = ru(this, 0x18);
    if (f >> 12) & 1 != 0 {
        return if (f >> 11) & 1 != 0 { 2000 } else { 1000 };
    }
    if (f >> 15) & 1 != 0 {
        return 1000;
    }
    let a = aim_offsets().byte_add((i32::from(rh(this, 0x6C) as i16) * AIM_OFFSET_SIZE as i32) as isize);
    for off in [0x10, 0x14, 0x12] {
        let t = u32::from(rh(a, off)) + 100;
        if t as i32 > 400 {
            return t;
        }
    }
    let t = u32::from(rh(a, 0x16)) + 100;
    if t as i32 > 400 { t } else { 400 }
}

/// `CWeaponInfo::Initialise()` (0x5BF750, cdecl): resets the 80 infos and the 21 aim offsets, then
/// tail-calls `LoadWeaponData`.
#[hook(addr = 0x5BF750, conv = cdecl, name = "CWeaponInfo::Initialise", expect = "83 EC 0C 53 55")]
pub fn initialise() {
    for i in 0..NUM_WEAPON_INFOS as i32 {
        let e = info(i);
        wu(e, 0x24, 0);
        wu(e, 0x28, 0);
        wu(e, 0x2C, 0);
        wu(e, 0, 0);
        wu(e, 4, 0);
        wu(e, 8, 0);
        wu(e, 0xC, u32::MAX);
        wu(e, 0x10, u32::MAX);
        wu(e, 0x14, u32::MAX);
        wu(e, 0x1C, 0);
        wh(e, 0x20, 0);
        wh(e, 0x22, 0);
        wu(e, 0x30, 1);
        wu(e, 0x34, 0);
        wu(e, 0x38, 0x3F80_0000);
        wu(e, 0x3C, 0x3F80_0000);
        for off in (0x40..=0x68).step_by(4) {
            wu(e, off, 0);
        }
        wh(e, 0x6C, 0);
        wu(e, 0x18, 0);
        wb(e, 0x6E, 4);
        wb(e, 0x6F, 1);
    }
    for i in 0..NUM_AIM_OFFSETS {
        let a = aim_offsets().byte_add((i * AIM_OFFSET_SIZE) as isize);
        for off in [0, 4, 8, 0xC] {
            wu(a, off, 0);
        }
        for off in [0x10, 0x12, 0x14, 0x16] {
            wh(a, off, 0);
        }
    }
    load_weapon_data();
}

/// `CWeaponInfo::Shutdown()` (0x743C50): `ret` (1-byte body, not hookable).
pub fn shutdown() {}

/// `FUN_0073a280` (0x73A280, thiscall `ret 4`; split chunk 0x4039E1): the reload loop time word of
/// the info's anim group: `+0x14` when `flag`, else `+0x10`, in AX over the upper half of the index
/// arithmetic (EAX = `idx*3` resp. `idx`).
#[hook(addr = 0x73A280, conv = thiscall, name = "FUN_0073a280", expect = "8A 44 24 04 84 C0", this = "CWeaponInfo")]
pub fn reload_loop_time_a(this: GPtr<u8>, flag: u8) -> u32 {
    let idx = i32::from(rh(this, 0x6C) as i16) as u32;
    let i3 = idx.wrapping_mul(3);
    let base = aim_offsets().byte_add((i3.wrapping_mul(8)) as i32 as isize);
    if flag != 0 {
        (i3 & 0xFFFF_0000) | u32::from(rh(base, 0x14))
    } else {
        (idx & 0xFFFF_0000) | u32::from(rh(base, 0x10))
    }
}

/// `FUN_0073a2b0` (0x73A2B0, thiscall `ret 4`): as [`reload_loop_time_a`] with `+0x16` / `+0x12`.
#[hook(addr = 0x73A2B0, conv = thiscall, name = "FUN_0073a2b0", expect = "8A 44 24 04 84 C0", this = "CWeaponInfo")]
pub fn reload_loop_time_b(this: GPtr<u8>, flag: u8) -> u32 {
    let idx = i32::from(rh(this, 0x6C) as i16) as u32;
    let i3 = idx.wrapping_mul(3);
    let base = aim_offsets().byte_add((i3.wrapping_mul(8)) as i32 as isize);
    if flag != 0 {
        (i3 & 0xFFFF_0000) | u32::from(rh(base, 0x16))
    } else {
        (idx & 0xFFFF_0000) | u32::from(rh(base, 0x12))
    }
}

/// 0x86D6F8.. the melee anim group names (with their NUL) and the group each selects.
const MELEE_GROUPS: [(&[u8], u32); 7] = [
    (b"UNARMED\0", 4),
    (b"BBALLBAT\0", 8),
    (b"KNIFE\0", 9),
    (b"GOLFCLUB\0", 10),
    (b"SWORD\0", 11),
    (b"CHAINSAW\0", 12),
    (b"DILDO\0", 13),
];

/// `FUN_0061db30` (0x61DB30, cdecl; called by `LoadWeaponData`): the melee anim group of a name:
/// UNARMED 4, BBALLBAT 8, KNIFE 9, GOLFCLUB 10, SWORD 11, CHAINSAW 12, DILDO 13, FLOWERS 14, else 4.
#[hook(addr = 0x61DB30, conv = cdecl, name = "FUN_0061db30", expect = "8B 44 24 04 56")]
pub fn melee_anim_group(s: GPtr<u8>) -> u32 {
    for (n, v) in MELEE_GROUPS {
        if cmps(s, n) {
            return v;
        }
    }
    if cmps(s, b"FLOWERS\0") { 14 } else { 4 }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_FIND_WEAPON_FIRE_TYPE,
    HOOK_GET_CROUCH_RELOAD_ANIMATION_ID,
    HOOK_GET_WEAPON_INFO,
    HOOK_GET_SKILL_STAT_INDEX,
    HOOK_FIND_WEAPON_TYPE,
    HOOK_GET_TARGET_HEAD_RANGE,
    HOOK_GET_WEAPON_RELOAD_TIME,
    HOOK_INITIALISE,
    HOOK_RELOAD_LOOP_TIME_A,
    HOOK_RELOAD_LOOP_TIME_B,
    HOOK_MELEE_ANIM_GROUP,
];
