//! Small world helpers reached through the original by ported code (FRONTIER-2 lane C). Ported from the 1.0 US bytes.
//! Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;

/// The female ped-type test (stub 0x43A6A0 → .HOODLUM 0x15697D0, cdecl; unnamed): EAX = 1 for ped types 22
/// (prostitute) and 5 (civilian female), else 0.
#[hook(addr = 0x43A6A0, conv = cdecl, name = "thunk_FUN_015697d0", expect = "E9 2B F1 12 01",
       unverified = "frontier2-c2", route = "L2", hazards = "none")]
pub fn is_female_ped_type(ty: u32) -> u32 {
    u32::from(ty == 0x16 || ty == 5)
}

/// `WaterCreatureManager_c::~WaterCreatureManager_c()` (0x6E3F20, thiscall; `__dtor_g_waterCreatureMan`'s target): the
/// `List_c`s at +0x1A0C and +0x1A00 (0x4A8DE0), then the 0x80 creatures of 0x34 (0x6E3E50) through
/// `eh_vector_destructor_iterator`. The SEH frame is not modelled (FRONTIER-2 turn 2).
#[hook(addr = 0x6E3F20, conv = thiscall, name = "WaterCreatureManager_c::~WaterCreatureManager_c",
       expect = "6A FF 68 84 88 84 00", unverified = "frontier2-c5", route = "L2", hazards = "heap-writes")]
pub fn water_creature_manager_dtor(this: GPtr<u8>) {
    list_dtor(this.byte_add(0x1A0C));
    list_dtor(this.byte_add(0x1A00));
    eh_vector_dtor(this, 0x34, 0x80, 0x6E3E50);
}

/// `CMatrixLinkList::~CMatrixLinkList()` (0x54EB70, thiscall; `__dtor_gMatrixList`'s target): the six sentinel links'
/// `CMatrix` destructors (0x59ACD0), +0x1A4, +0x150, +0xFC, +0xA8, +0x54, +0 in that order. The SEH frame is not
/// modelled (FRONTIER-2 turn 2).
#[hook(addr = 0x54EB70, conv = thiscall, name = "CMatrixLinkList::~CMatrixLinkList", expect = "6A FF 68 FD CA 83 00",
       unverified = "frontier2-c5", route = "L2", hazards = "heap-writes")]
pub fn matrix_link_list_dtor(this: GPtr<u8>) {
    for off in [0x1A4, 0x150, 0xFC, 0xA8, 0x54, 0] {
        matrix_dtor(this.byte_add(off));
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_IS_FEMALE_PED_TYPE, HOOK_WATER_CREATURE_MANAGER_DTOR, HOOK_MATRIX_LINK_LIST_DTOR];

use sa_core::mem::GPtr;
use sa_core::orig;
use super::common::{g, rd, wr, xi};
use crate::bindings::{find_player_ped, find_player_vehicle, matrix_dtor};
use sa_core::bindings::{eh_vector_dtor, list_dtor};

orig! {
    /// virtual slot +0xC8 (`CVehicle::Fix()`)
    fn v_fix(v: GPtr<u8>) = thiscall vslot 0xC8 => unported;
}

/// `CCheat::HealthCheat()` (0x438D60, cdecl): the player ped's health (+0x540) = player 0's max health (byte 0xB7CEE7,
/// `fild`); in a vehicle (every access through `FindPlayerVehicle(-1, false)` again, as the original): health (+0x4C0)
/// = 1000.0; an automobile (type +0x590 == 0) gets +0x8E4 = 0, a bike (type 9) +0x7BC = 0, and either is `Fix()`ed
/// (vtable +0xC8, tail call); other vehicle types stop after the health. Not in the catalogue (the tail of
/// `CCheat::MoneyArmourHealthCheat`'s extent, CAT-12): a phase B native in turn 1, a hook since it is a catalogue row.
#[hook(addr = 0x438D60, conv = cdecl, name = "CCheat::HealthCheat", expect = "51 0F B6 05 E7 CE B7 00",
       unverified = "frontier2-c8", route = "L2", hazards = "heap-writes",
       note = "FRONTIER-2 lane C turn 3: was a phase B native until the body became a catalogue row (CAT-12 tail target)")]
pub fn health_cheat() {
    let h = xi(i32::from(g::<u8>(0xB7CEE7).read())).to_f32();
    wr::<f32>(find_player_ped(-1), 0x540, h);
    if find_player_vehicle(-1, 0).is_null() {
        return;
    }
    wr::<u32>(find_player_vehicle(-1, 0), 0x4C0, 0x447A_0000);
    if rd::<u32>(find_player_vehicle(-1, 0), 0x590) != 0 && rd::<u32>(find_player_vehicle(-1, 0), 0x590) != 9 {
        return;
    }
    if rd::<u32>(find_player_vehicle(-1, 0), 0x590) == 0 {
        wr::<u32>(find_player_vehicle(-1, 0), 0x8E4, 0);
    } else {
        wr::<u32>(find_player_vehicle(-1, 0), 0x7BC, 0);
    }
    v_fix(find_player_vehicle(-1, 0));
}

/// The water sync stamp (0x6E76E0, cdecl; gta-reversed `CWaterLevel::SyncWater`): `[0xC228A4] =
/// CTimer::m_snTimeInMilliseconds`. Not in the catalogue (CAT-12), so a phase B native.
#[sa_core::native(addr = 0x6E76E0, conv = cdecl, name = "CWaterLevel::SyncWater")]
pub fn sync_water() {
    g::<u32>(0xC228A4).write(g::<u32>(0xB7CB84).read());
}

/// Every `#[native]` of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_SYNC_WATER];

/// The lane C turn-1 natives that became hooks once their bodies were catalogue rows (FRONTIER-2 turn 3).
pub const HOOKS3: &[HookReg] = &[HOOK_HEALTH_CHEAT];

orig! {
    /// `CPlayerData::operator=` (stub 0x45C4B0 → .HOODLUM 0x1561CF0), with the caller's ESI
    fn player_data_assign_call(this: GPtr<u8>, src: GPtr<u8>, esi: u32) -> GPtr<u8> = reg("args = ecx stack esi; pop") 0x45C4B0 => player_data_assign;
}

/// Copies the field of `size` bytes (1, 2 or 4) at `off` from `src` to `dst`.
fn copy_field(dst: GPtr<u8>, src: GPtr<u8>, off: isize, size: u8) {
    match size {
        1 => wr::<u8>(dst, off, rd::<u8>(src, off)),
        2 => wr::<u16>(dst, off, rd::<u16>(src, off)),
        _ => wr::<u32>(dst, off, rd::<u32>(src, off)),
    }
}

/// Copies the fields `(first, last, size)` (inclusive, step `size`) from `src` to `dst` in order.
fn copy_fields(dst: GPtr<u8>, src: GPtr<u8>, runs: &[(isize, isize, u8)]) {
    for &(first, last, size) in runs {
        let mut off = first;
        while off <= last {
            copy_field(dst, src, off, size);
            off += isize::from(size);
        }
    }
}

/// `CPlayerData::operator=(const CPlayerData&)` (stub 0x45C4B0 → .HOODLUM 0x1561CF0, thiscall, `ret 4`; its only caller
/// is `CPlayerInfo::operator=`): the fields of the 0xAC-byte block one by one (padding +0x22, +0x87, +0x8F skipped).
/// The bit-field word +0x34: the no-CD body has ten NOPs where the source load of the first bit-field copy was, so that
/// step reads the caller's ESI: `+0x34 = ((+0x34 ^ esi) & 0x200) ^ esi`, then bits 0x400 / 0x800 / 0x1000 come from the
/// source one at a time. +0xA0 is stored twice (bits 0..30 first, then the whole source word). EAX = `this`.
#[hook(addr = 0x45C4B0, conv = reg("args = ecx stack esi; pop"), name = "thunk_FUN_01561cf0", expect = "E9 3B 58 10 01",
       unverified = "frontier2-c12", route = "L2", hazards = "heap-writes",
       note = "the +0x34 bit-field step reads the caller's ESI (NOPed source load in the no-CD body 0x1561D4C)")]
pub fn player_data_assign(this: GPtr<u8>, src: GPtr<u8>, esi: u32) -> GPtr<u8> {
    copy_fields(this, src, &[(0, 0x1C, 4), (0x20, 0x21, 1), (0x24, 0x30, 4)]);
    let mut d = rd::<u32>(this, 0x34);
    d = ((d ^ esi) & 0x200) ^ esi;
    wr::<u32>(this, 0x34, d);
    for m in [0x400u32, 0x800, 0x1000] {
        d ^= (rd::<u32>(src, 0x34) ^ d) & m;
        wr::<u32>(this, 0x34, d);
    }
    copy_fields(
        this,
        src,
        &[
            (0x38, 0x3C, 4),
            (0x40, 0x43, 1),
            (0x44, 0x80, 4),
            (0x84, 0x86, 1),
            (0x88, 0x8A, 2),
            (0x8C, 0x8E, 1),
            (0x90, 0x9C, 4),
        ],
    );
    let old = rd::<u32>(this, 0xA0);
    let first = ((old ^ rd::<u32>(src, 0xA0)) & 0x7FFF_FFFF) ^ old;
    wr::<u32>(this, 0xA0, first);
    let s = rd::<u32>(src, 0xA0);
    wr::<u32>(this, 0xA0, ((s ^ first) & 0x7FFF_FFFF) ^ s);
    copy_fields(this, src, &[(0xA4, 0xA8, 4)]);
    this
}

/// `CPlayerInfo::operator=(const CPlayerInfo&)` (stub 0x45DEF0 → .HOODLUM 0x156AE50, thiscall, `ret 4`; callers
/// `CReplay::RestorePlayerInfoVariables` / `RestoreStuffFromMem`): +0, then `CPlayerData::operator=` on +4 (with ESI =
/// `this`), then the remaining fields of the 0x190-byte block one by one (the 32 bytes at +0x164 by a byte loop).
/// EAX = `this`.
#[hook(addr = 0x45DEF0, conv = thiscall, name = "thunk_FUN_0156ae50", expect = "E9 5B CF 10 01",
       unverified = "frontier2-c12", route = "L2", hazards = "heap-writes")]
pub fn player_info_assign(this: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, rd::<u32>(src, 0));
    player_data_assign_call(this.byte_add(4), src.byte_add(4), this.addr() as u32);
    copy_fields(
        this,
        src,
        &[
            (0xB0, 0xD0, 4),
            (0xD4, 0xD5, 1),
            (0xD8, 0xD8, 4),
            (0xDC, 0xDF, 1),
            (0xE0, 0x12C, 4),
            (0x130, 0x130, 2),
            (0x134, 0x140, 4),
            (0x144, 0x144, 2),
            (0x148, 0x148, 4),
            (0x14C, 0x154, 1),
            (0x156, 0x156, 2),
            (0x158, 0x160, 4),
            (0x164, 0x183, 1),
            (0x184, 0x184, 4),
            (0x188, 0x188, 1),
            (0x18C, 0x18C, 4),
        ],
    );
    this
}

/// Lane C turn 6 hooks (batch `frontier2-c12`).
pub const HOOKS6: &[HookReg] = &[HOOK_PLAYER_DATA_ASSIGN, HOOK_PLAYER_INFO_ASSIGN];
