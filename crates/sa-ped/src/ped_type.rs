//! `CPedType` / `CPedStats` leaves (PORT-WEAPON-PED). Ported from the 1.0 US bytes (gta-reversed
//! `PedType.cpp`, `PedStats.cpp`, `docs/re/specs/PED/CPedType.deviations.md`, `CPedStats.md`).
//!
//! `CPedType::ms_apPedTypes` (0xC0BBE8): `CAcquaintance[32]` (0x14 each: respect, like, ignore, dislike,
//! hate bit masks) allocated by `Initialise`; `CPedStats::ms_apPedStats` (0xC0BBEC): `CPedStat[43]`
//! (0x34 each: +0 id, +4 name[24], ...).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

sa_core::globals! {
    list GLOBALS;
    /// `CPedType::ms_apPedTypes`
    pub static PED_TYPES @ 0xC0BBE8: GPtr<u8> = GPtr::NULL;
    /// `CPedStats::ms_apPedStats`
    pub static PED_STATS @ 0xC0BBEC: GPtr<u8> = GPtr::NULL;
}

/// The ped type name table (`const char*[32]` at 0x8D23B8, .data).
const PED_TYPE_NAMES: usize = 0x8D23B8;
const NUM_PED_TYPES: u32 = 32;
const NUM_PED_STATS: u32 = 43;

orig! {
    /// `CAcquaintance::SetAsAcquaintance(AcquaintanceId, uint32 mask)` (0x608DA0)
    pub fn acquaintance_set_as_acquaintance(this: GPtr<u8>, id: i32, mask: u32) = thiscall 0x608DA0 => unported;
    /// CRT `free` (0x8213AE)
    pub fn crt_free(p: GPtr<u8>) = cdecl 0x8213AE => unported;
    /// CRT `eh vector destructor iterator(void*, size, count, dtor)` (0x821E02)
    pub fn eh_vector_destructor_iterator(p: GPtr<u8>, size: u32, count: u32, dtor: u32) = stdcall 0x821E02 => unported;
}

/// `strcmp(a, b) == 0` as the inlined MSVC loop compares (bytes, stops at the first difference or NUL).
pub fn str_eq(a: GPtr<u8>, b: GPtr<u8>) -> bool {
    let mut i = 0isize;
    loop {
        let (x, y) = (a.byte_add(i).read(), b.byte_add(i).read());
        if x != y {
            return false;
        }
        if x == 0 {
            return true;
        }
        i += 1;
    }
}

/// `repe cmpsb` of `n` bytes: true when all equal.
fn mem_eq(a: GPtr<u8>, b: usize, n: isize) -> bool {
    (0..n).all(|i| a.byte_add(i).read() == g::<u8>(b).byte_add(i).read())
}

/// `CPedType::FindPedType(const char*)` (0x608790): index of the name in the type table; else 2 for
/// "PLAYER_NETWORK", 3 for "PLAYER_UNUSED", 0x20 otherwise.
#[hook(addr = 0x608790, conv = cdecl, name = "CPedType::FindPedType", expect = "53 55 8B 6C 24 0C")]
pub fn find_ped_type(name: GPtr<u8>) -> i32 {
    for i in 0..NUM_PED_TYPES {
        let s = g::<GPtr<u8>>(PED_TYPE_NAMES + 4 * i as usize).read();
        if str_eq(name, s) {
            return i as i32;
        }
    }
    if mem_eq(name, 0x86D028, 15) {
        return 2;
    }
    if mem_eq(name, 0x86D018, 14) { 3 } else { 0x20 }
}

/// `CPedType::GetPedFlag(ePedType)` (0x608830): `1 << type` for type < 32 (signed), else 0.
#[hook(addr = 0x608830, conv = cdecl, name = "CPedType::GetPedFlag", expect = "8B 4C 24 04 83 F9 20")]
pub fn get_ped_flag(ty: i32) -> u32 {
    if ty < 0x20 { 1u32.wrapping_shl(ty as u32) } else { 0 }
}

/// `CPedStats::GetPedStatType(const char*)` (0x6088D0): index (u16) of the stat whose name matches; 0x10
/// when none does.
#[hook(addr = 0x6088D0, conv = cdecl, name = "CPedStats::GetPedStatType", expect = "8B 15 EC BB C0 00")]
pub fn get_ped_stat_type(name: GPtr<u8>) -> u32 {
    let base = PED_STATS.get();
    for i in 0..NUM_PED_STATS {
        if str_eq(base.byte_add(i as isize * 0x34 + 4), name) {
            return i;
        }
    }
    0x10
}

/// `CPedStats::FindIndexWithPedStat(CPedStat*)` (0x608940): the index of the pointer in the table, 0 when
/// not found.
#[hook(addr = 0x608940, conv = cdecl, name = "CPedStats::FindIndexWithPedStat", expect = "8B 0D EC BB C0 00")]
pub fn find_index_with_ped_stat(stat: GPtr<u8>) -> i32 {
    let base = PED_STATS.get();
    for i in 0..NUM_PED_STATS {
        if base.byte_add(i as isize * 0x34) == stat {
            return i as i32;
        }
    }
    0
}

fn acq(ty: i32) -> GPtr<u8> {
    PED_TYPES.get().byte_add((ty as isize).wrapping_mul(0x14))
}

/// `CPedType::GetPedTypeAcquaintances(ePedType)` (0x6089B0): `&ms_apPedTypes[type]`.
#[hook(addr = 0x6089B0, conv = cdecl, name = "CPedType::GetPedTypeAcquaintances-CAcquaintance", expect = "8B 44 24 04 8B 0D E8 BB C0 00")]
pub fn get_ped_type_acquaintances(ty: i32) -> GPtr<u8> {
    acq(ty)
}

/// `CPedType::GetPedTypeAcquaintances(AcquaintanceId, ePedType)` (0x6089D0): the mask word.
#[hook(addr = 0x6089D0, conv = cdecl, name = "CPedType::GetPedTypeAcquaintances-uint32", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn get_ped_type_acquaintances_id(id: i32, ty: i32) -> u32 {
    rd::<u32>(PED_TYPES.get(), ((ty.wrapping_mul(5).wrapping_add(id)) as isize).wrapping_mul(4))
}

/// `CPedType::ClearPedTypeAsAcquaintance(AcquaintanceId, ePedType, uint32 mask)` (0x6089F0): clears the
/// bits when any is set.
#[hook(addr = 0x6089F0, conv = cdecl, name = "CPedType::ClearPedTypeAsAcquaintance", expect = "8B 44 24 08 8B 0D E8 BB C0 00")]
pub fn clear_ped_type_as_acquaintance(id: i32, ty: i32, mask: u32) {
    let p = acq(ty).byte_add((id as isize).wrapping_mul(4));
    let v = rd::<u32>(p, 0);
    if v & mask != 0 {
        wr::<u32>(p, 0, !mask & v);
    }
}

/// `CPedType::ClearPedTypeAcquaintances(AcquaintanceId, ePedType)` (0x608A20)
#[hook(addr = 0x608A20, conv = cdecl, name = "CPedType::ClearPedTypeAcquaintances", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn clear_ped_type_acquaintances(id: i32, ty: i32) {
    wr::<u32>(PED_TYPES.get(), ((ty.wrapping_mul(5).wrapping_add(id)) as isize).wrapping_mul(4), 0);
}

/// `CPedType::PoliceDontCareAboutCrimesAgainstPedType(ePedType)` (0x608A40): gangs 7..16, 0x11, 0x14
/// (EAX 0/1).
#[hook(addr = 0x608A40, conv = cdecl, name = "CPedType::PoliceDontCareAboutCrimesAgainstPedType", expect = "8B 44 24 04 83 F8 07")]
pub fn police_dont_care_about_crimes_against_ped_type(ty: i32) -> u8 {
    ((7..=0x10).contains(&ty) || ty == 0x14 || ty == 0x11) as u8
}

/// `CPedType::SetPedTypeAsAcquaintance(AcquaintanceId, ePedType, uint32 mask)` (0x608E20, cdecl):
/// `ms_apPedTypes[type].SetAsAcquaintance(id, mask)`.
#[hook(addr = 0x608E20, conv = cdecl, name = "CPedType::SetPedTypeAsAcquaintance", expect = "8B 44 24 0C 8B 4C 24 04")]
pub fn set_ped_type_as_acquaintance(id: i32, ty: i32, mask: u32) {
    acquaintance_set_as_acquaintance(acq(ty), id, mask);
}

/// `CPedStats::Shutdown` (0x608850): `free(ms_apPedStats)` (the pointer is left as is).
#[hook(addr = 0x608850, conv = cdecl, name = "CPedStats::Shutdown", expect = "A1 EC BB C0 00")]
pub fn ped_stats_shutdown() {
    crt_free(PED_STATS.get());
}

/// `CPedType::Shutdown` (0x608B00): `delete[] ms_apPedTypes` (the `eh vector destructor iterator` with
/// `CAcquaintance::~CAcquaintance` 0x608780 over the count stored before the array, then `free` of the
/// block); the pointer is left as is.
#[hook(addr = 0x608B00, conv = cdecl, name = "CPedType::Shutdown", expect = "A1 E8 BB C0 00")]
pub fn ped_type_shutdown() {
    let a = PED_TYPES.get();
    if a.is_null() {
        return;
    }
    let count = rd::<u32>(a, -4);
    eh_vector_destructor_iterator(a, 0x14, count, 0x608780);
    crt_free(a.byte_add(-4));
}

// --- hook list (generated) ---

pub const HOOKS: &[HookReg] = &[
    HOOK_FIND_PED_TYPE,
    HOOK_GET_PED_FLAG,
    HOOK_GET_PED_STAT_TYPE,
    HOOK_FIND_INDEX_WITH_PED_STAT,
    HOOK_GET_PED_TYPE_ACQUAINTANCES,
    HOOK_GET_PED_TYPE_ACQUAINTANCES_ID,
    HOOK_CLEAR_PED_TYPE_AS_ACQUAINTANCE,
    HOOK_CLEAR_PED_TYPE_ACQUAINTANCES,
    HOOK_POLICE_DONT_CARE_ABOUT_CRIMES_AGAINST_PED_TYPE,
    HOOK_SET_PED_TYPE_AS_ACQUAINTANCE,
    HOOK_PED_STATS_SHUTDOWN,
    HOOK_PED_TYPE_SHUTDOWN,
];
