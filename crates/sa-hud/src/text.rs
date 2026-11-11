//! `CText` / `CKeyArray` / `CData` (PORT-HUD): the GXT text tables (`TheText` 0xC1B340). Ported from the
//! 1.0 US bytes; `docs/re/specs/HUD/CText.deviations.md` and gta-reversed `Text/*.cpp` as reading aids.
//!
//! `CText` (0xA90): +0 main `CKeyArray` {entries, size}, +8 main `CData` {data, size}, +0x10 mission
//! `CKeyArray`, +0x18 mission `CData`, +0x20 encoding byte ('e'), +0x21 mission-offsets-loaded,
//! +0x22 / +0x23 flags (+0x23 = mission text loaded), +0x24 mission name[8], +0xA8C offsets count (u16).
//! `CKeyArray` entries are { GxtChar* string; uint32 hash } (8 bytes).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr};

sa_core::globals! {
    list GLOBALS;
    /// `CText::Get`'s static result for a missing key (25 bytes, copied from a stack buffer)
    pub static GET_ERROR_TEXT @ 0xC1AEB8: [u8; 25] = [0; 25];
}

/// "" (the format `CText::Get` prints for a missing key)
const FMT_EMPTY: usize = 0x858B54;
/// byte tested by `CText::Get` before searching the mission table (with +0x23)
const GET_MISSION_FLAG: usize = 0xB72910;

orig! {
    /// CRT `operator delete` / `free` (0x8213AE)
    pub fn crt_delete(p: GPtr<u8>) = cdecl 0x8213AE => unported;
    /// CRT `sprintf(buf, fmt)` (0x821BB5)
    pub fn sprintf0(buf: GPtr<u8>, fmt: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    pub fn get_uppercase_key(key: GPtr<u8>) -> u32 = cdecl 0x53CF30 => unported;
    /// `CKeyArray::BinarySearch(uint32 key, CKeyEntry* entries, int16 low, int16 high)` (0x69F570)
    pub fn binary_search_call(this: GPtr<u8>, key: u32, entries: GPtr<u8>, low: i32, high: i32) -> GPtr<u8> = thiscall 0x69F570 => binary_search;
    /// `CKeyArray::Search(const char* key, bool* found)` (0x6A0000)
    pub fn key_array_search_call(this: GPtr<u8>, key: GPtr<u8>, found: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0000 => key_array_search;
    /// `CMessages::ClearAllMessagesDisplayedByGame(bool)` (0x69EDC0)
    pub fn clear_all_messages_displayed_by_game_call(flag: u32) = cdecl 0x69EDC0 => crate::messages::clear_all_messages_displayed_by_game;
}

/// Frees `[p + off]` when set and nulls it, then zeroes the size at `p + off + 4` (`CKeyArray::Unload` /
/// `CData::Unload` inlined shape).
fn unload_pair(p: GPtr<u8>, off: isize) {
    let d = rd::<GPtr<u8>>(p, off);
    if !d.is_null() {
        crt_delete(d);
        wr(p, off, GPtr::<u8>::NULL);
    }
    wr::<u32>(p, off + 4, 0);
}

/// `CKeyArray::Unload()` (0x69F510).
#[hook(addr = 0x69F510, conv = thiscall, name = "CKeyArray::Unload", expect = "56 8B F1 8B 06")]
pub fn key_array_unload(this: GPtr<u8>) {
    unload_pair(this, 0);
}

/// `CKeyArray::BinarySearch(uint32 key, CKeyEntry* entries, int16 low, int16 high)` (0x69F570): int16
/// bounds, `mid = (low + high) / 2` truncated toward zero and used as an unsigned 16-bit index; unsigned
/// hash compare. Returns the entry or null.
#[hook(addr = 0x69F570, conv = thiscall, name = "CKeyArray::BinarySearch", expect = "53 55 56 8B 74 24 18")]
pub fn binary_search(_this: GPtr<u8>, key: u32, entries: GPtr<u8>, low: i32, high: i32) -> GPtr<u8> {
    let mut lo = low as i16;
    let mut hi = high as i16;
    while lo <= hi {
        let mid = (i32::from(lo) + i32::from(hi)) / 2;
        let e = entries.byte_add(((mid as u16) as isize) * 8);
        let h = rd::<u32>(e, 4);
        if key == h {
            return e;
        }
        if key > h {
            lo = mid.wrapping_add(1) as i16;
        } else {
            hi = mid.wrapping_sub(1) as i16;
        }
    }
    GPtr::NULL
}

/// `CData::Unload()` (0x69F640).
#[hook(addr = 0x69F640, conv = thiscall, name = "CData::Unload", expect = "56 8B F1 8B 06")]
pub fn data_unload(this: GPtr<u8>) {
    unload_pair(this, 0);
}

/// `CText::GetNameOfLoadedMissionText(char* out)` (0x69FBD0): unbounded copy of the mission name.
#[hook(addr = 0x69FBD0, conv = thiscall, name = "CText::GetNameOfLoadedMissionText", expect = "8B 54 24 04 8D 41 24")]
pub fn get_name_of_loaded_mission_text(this: GPtr<u8>, out: GPtr<u8>) {
    let mut i = 0isize;
    loop {
        let c = rd::<u8>(this, 0x24 + i);
        wr::<u8>(out, i, c);
        if c == 0 {
            break;
        }
        i += 1;
    }
}

/// `CText::Unload(bool unloadMissionData)` (0x69FF20): clears all game messages, frees the main tables
/// and, when the flag is clear, the mission tables, the loaded flag and the mission name.
#[hook(addr = 0x69FF20, conv = thiscall, name = "CText::Unload", expect = "53 56 33 DB 53")]
pub fn text_unload(this: GPtr<u8>, keep_mission: u32) {
    clear_all_messages_displayed_by_game_call(0);
    unload_pair(this, 0);
    unload_pair(this, 8);
    wr::<u8>(this, 0x22, 0);
    if keep_mission as u8 != 0 {
        return;
    }
    unload_pair(this, 0x10);
    unload_pair(this, 0x18);
    wr::<u8>(this, 0x23, 0);
    wr::<u32>(this, 0x24, 0);
    wr::<u32>(this, 0x28, 0);
}

/// `CKeyArray::Search(const char* key, bool* found)` (0x6A0000): binary search of the uppercase key hash
/// over `[0, size - 1]` (u16 size); `*found` set, returns the entry's string or null.
///
/// Not hooked: sa-hook's `every_hook_installs_on_the_exe` rejects the entry 0x6A0000 (instructions whose
/// absolute operand is 0x6A0000, e.g. `mov word [0x6A0000], es` at 0x5F7B7E / 0x62FDB5 / 0x1568F84 and
/// `fadd qword [0x6A0000]` at 0x4427E2, 58 keycells readers).
pub fn key_array_search(this: GPtr<u8>, key: GPtr<u8>, found: GPtr<u8>) -> GPtr<u8> {
    let h = get_uppercase_key(key);
    let entries = rd::<GPtr<u8>>(this, 0);
    let high = u32::from(rd::<u16>(this, 4).wrapping_sub(1));
    let e = binary_search_call(this, h, entries, 0, high as i32);
    if e.is_null() {
        wr::<u8>(found, 0, 0);
        return GPtr::NULL;
    }
    wr::<u8>(found, 0, 1);
    rd(e, 0)
}

/// `CText::Get(const char* key)` (0x6A0050): main table, then the mission table when mission text is
/// loaded (+0x23) and either the byte at 0xB72910 or +0x21 is set; a missing or empty / space-led key
/// gives the static 25-byte buffer 0xC1AEB8 filled from `sprintf(buf, "")` (bytes 1..24 are uninitialised
/// stack in the original, zeros here).
///
/// Not hooked: the protection reads the dword at 0x6A004D (covering the entry byte 0x6A0050) at 0xD723B1
/// (`mov edx, [0x6A004D]`) and 0xD72411 (`sub edx, [0x6A004D]`); hooking needs two sa-hook keycells
/// `READERS` entries first.
pub fn text_get(this: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> {
    let mut found = [0u8; 1];
    let fp = GPtr::from_raw(found.as_mut_ptr());
    let c0 = rd::<u8>(key, 0);
    if c0 != b' ' && c0 != 0 {
        let r = key_array_search_call(this, key, fp);
        if found[0] != 0 {
            return r;
        }
        let mission = rd::<u8>(this, 0x23) != 0;
        if mission && (g::<u8>(GET_MISSION_FLAG).read() != 0 || rd::<u8>(this, 0x21) != 0) {
            let r = key_array_search_call(this.byte_add(0x10), key, fp);
            if found[0] != 0 {
                return r;
            }
        }
    }
    let mut buf = [0u8; 25];
    sprintf0(GPtr::from_raw(buf.as_mut_ptr()), g(FMT_EMPTY));
    GET_ERROR_TEXT.set(buf);
    GET_ERROR_TEXT.ptr().cast()
}

/// `CText::CText()` (0x6A00F0): tables null, offsets count 0, flags clear, encoding 'e', mission name
/// cleared, and the 25-byte error buffer zeroed. Returns `this`.
#[hook(addr = 0x6A00F0, conv = thiscall, name = "CText::Constructor", expect = "8B D1 33 C0 89 02")]
pub fn text_constructor(this: GPtr<u8>) -> GPtr<u8> {
    for k in 0..8 {
        wr::<u32>(this, k * 4, 0);
    }
    wr::<u16>(this, 0xA8C, 0);
    wr::<u8>(this, 0x23, 0);
    wr::<u8>(this, 0x21, 0);
    wr::<u8>(this, 0x20, 0x65);
    wr::<u32>(this, 0x24, 0);
    wr::<u32>(this, 0x28, 0);
    GET_ERROR_TEXT.set([0; 25]);
    this
}

/// `CText::~CText()` (0x6A0140): mission data, mission keys, main data, main keys freed.
#[hook(addr = 0x6A0140, conv = thiscall, name = "CText::Destructor", expect = "56 8B F1 8B 46 18")]
pub fn text_destructor(this: GPtr<u8>) {
    unload_pair(this, 0x18);
    unload_pair(this, 0x10);
    unload_pair(this, 8);
    unload_pair(this, 0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_KEY_ARRAY_UNLOAD,
    HOOK_BINARY_SEARCH,
    HOOK_DATA_UNLOAD,
    HOOK_GET_NAME_OF_LOADED_MISSION_TEXT,
    HOOK_TEXT_UNLOAD,
    HOOK_TEXT_CONSTRUCTOR,
    HOOK_TEXT_DESTRUCTOR,
];
