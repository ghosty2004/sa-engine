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

orig! {
    /// CRT `operator new(size)` (0x821195)
    fn ps_operator_new(size: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
    /// `CFileMgr::OpenFile(path, mode)` (0x538900)
    fn ps_open_file(path: GPtr<u8>, mode: GPtr<u8>) -> u32 = cdecl 0x538900 => unported;
    /// `CFileMgr::CloseFile(file)` (0x5389D0)
    fn ps_close_file(file: u32) -> i32 = cdecl 0x5389D0 => unported;
    /// `CFileLoader::LoadLine(FILE*)` (0x536F80)
    fn ps_load_line(file: u32) -> GPtr<u8> = cdecl 0x536F80 => unported;
    /// CRT `sscanf(line, "%s %f %f %d %d %d %d %f %f %d %d", ...)` (0x8220AD)
    fn ps_sscanf(line: GPtr<u8>, fmt: GPtr<u8>, name: GPtr<u8>, a: GPtr<u32>, b: GPtr<u32>, c: GPtr<u32>, d: GPtr<u32>, e: GPtr<u32>, f: GPtr<u32>, gg: GPtr<u32>, h: GPtr<u32>, i: GPtr<u32>, j: GPtr<u32>) -> i32 = cdecl 0x8220AD => unported;
    /// `CDecisionMakerTypesFileLoader::LoadDefaultDecisionMaker()` (0x5BF400)
    fn ps_load_default_decision_maker() = cdecl 0x5BF400 => unported;
}

/// `CPedStats::Initialise` (0x5BF9D0): `ms_apPedStats = new CPedStat[43]` (0x8BC bytes), each entry
/// defaulted (+0 id 0, +0x1C flee distance 20.0, +0x20 heading change rate 15.0, +0x24..+0x27 fear,
/// temper, lawfulness, sexiness 50, +0x28 attack strength 1.0, +0x2C defend weakness 1.0, +0x30 shooting
/// rate 0, +0x32 default decision maker 0; the name is left as allocated), then `LoadPedStats` and a
/// tail jump to `CDecisionMakerTypesFileLoader::LoadDefaultDecisionMaker`.
#[hook(addr = 0x5BF9D0, conv = cdecl, name = "CPedStats::Initialise", expect = "53 55 56 68 BC 08 00 00",
       unverified = "std4-data-1", route = "L2", hazards = "heap-writes", recs = "boot")]
pub fn ped_stats_initialise() {
    let t = ps_operator_new(0x8BC);
    PED_STATS.set(t);
    for i in 0..NUM_PED_STATS as isize {
        let e = PED_STATS.get().byte_add(0x34 * i);
        wr::<u32>(e, 0, 0);
        wr::<u32>(e, 0x1C, 0x41A0_0000);
        wr::<u32>(e, 0x20, 0x4170_0000);
        for o in 0x24..0x28 {
            wr::<u8>(e, o, 0x32);
        }
        wr::<u32>(e, 0x28, 0x3F80_0000);
        wr::<u32>(e, 0x2C, 0x3F80_0000);
        wr::<u16>(e, 0x30, 0);
        wr::<u8>(e, 0x32, 0);
    }
    load_ped_stats();
    ps_load_default_decision_maker();
}

/// `CPedStats::LoadPedStats` (0x5BB890): `data\pedstats.dat` line by line (`CFileLoader::LoadLine`),
/// skipping lines starting with `#` or empty; each other line is `sscanf`'d with
/// `"%s %f %f %d %d %d %d %f %f %d %d"` and fills the next entry: +0 the running index, +4 the name
/// (strcpy), +0x1C/+0x20 floats, +0x24..+0x27 the low bytes of the four ints, +0x28/+0x2C floats, +0x30
/// the low word and +0x32 the low byte of the last two ints. The values are stack locals in the
/// original: fields `sscanf` does not fill keep the previous line's values (the port keeps them across
/// lines too; before the first line they are zero here, uninitialised there). The floats are copied
/// with `fld`/`fstp` (an SNaN would be quieted; the port copies bits).
#[hook(addr = 0x5BB890, conv = cdecl, name = "CPedStats::LoadPedStats", expect = "83 EC 48 53 57",
       unverified = "std4-data-1", route = "L2", hazards = "uninit-locals", recs = "boot")]
pub fn load_ped_stats() {
    let f = ps_open_file(g::<u8>(0x86A698), g::<u8>(0x85A53C));
    let mut v = [0u32; 10];
    let mut name = [0u8; 0x20];
    let mut index = 0u32;
    let mut off = 0isize;
    loop {
        let line = ps_load_line(f);
        if line.is_null() {
            break;
        }
        let c = line.read();
        if c == b'#' || c == 0 {
            continue;
        }
        let vb = v.as_mut_ptr();
        let vp = |i: usize| GPtr::<u32>::from_raw(vb.wrapping_add(i));
        ps_sscanf(
            line,
            g::<u8>(0x86A674),
            GPtr::from_raw(name.as_mut_ptr()),
            vp(0), vp(1), vp(2), vp(3), vp(4), vp(5), vp(6), vp(7), vp(8), vp(9),
        );
        let e = PED_STATS.get().byte_add(off);
        wr::<u32>(e, 0, index);
        let mut k = 0isize;
        loop {
            let b = name[k as usize];
            wr::<u8>(e, 4 + k, b);
            if b == 0 {
                break;
            }
            k += 1;
        }
        wr::<u32>(e, 0x1C, v[0]);
        index += 1;
        wr::<u32>(e, 0x20, v[1]);
        wr::<u8>(e, 0x24, v[2] as u8);
        wr::<u8>(e, 0x25, v[3] as u8);
        wr::<u8>(e, 0x26, v[4] as u8);
        wr::<u8>(e, 0x27, v[5] as u8);
        wr::<u32>(e, 0x28, v[6]);
        wr::<u32>(e, 0x2C, v[7]);
        wr::<u16>(e, 0x30, v[8] as u16);
        wr::<u8>(e, 0x32, v[9] as u8);
        off += 0x34;
    }
    ps_close_file(f);
}

// --- hook list (generated) ---

pub const HOOKS: &[HookReg] = &[
    HOOK_PED_STATS_INITIALISE,
    HOOK_LOAD_PED_STATS,
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
