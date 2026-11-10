//! `CKeyGen` (0x53CED0–0x53CF9C): the CRC-32 string keys of model, animation and archive names.
//!
//! Ported from the 1.0 US bytes. The key is the reflected CRC-32 (polynomial 0xEDB88320) with the initial
//! value `0xFFFFFFFF` and **no final xor**, computed through the 256-entry table `CKeyGen::keyTable`
//! (0x8CD068, `.data`; a test checks the owned copy against the exe image). Bytes are unsigned
//! (`movzx`/`mov dl, [ecx]` into a cleared register). `GetUppercaseKey` uppercases each byte with the
//! CRT's locale-aware `toupper` (0x823A0E) called once per byte, so it goes through `orig!`.
//! gta-reversed agrees with the bytes here (no entries in catalog/gr-deviations.csv).

use crate::mem::GPtr;
use crate::{globals, hook, orig};

/// The reflected CRC-32 table (what the exe holds at 0x8CD068).
pub const fn crc_table() -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        t[i] = c;
        i += 1;
    }
    t
}

globals! {
    list GLOBALS;
    /// `CKeyGen::keyTable` (`.data`, never written by the game)
    pub static KEY_TABLE @ 0x8CD068: [u32; 256] = crc_table();
}

/// One table step: `(key >> 8) ^ table[(key ^ byte) & 0xFF]`.
#[inline]
fn step(table: &[u32; 256], key: u32, b: u8) -> u32 {
    (key >> 8) ^ table[((key ^ b as u32) & 0xFF) as usize]
}

/// The key of `bytes` continued from `key` (the loops of all four functions).
pub fn key_of(table: &[u32; 256], key: u32, bytes: impl IntoIterator<Item = u8>) -> u32 {
    bytes.into_iter().fold(key, |k, b| step(table, k, b))
}

/// The bytes of a C string, without the terminator.
fn c_bytes(s: GPtr<u8>) -> impl Iterator<Item = u8> {
    let mut i = 0i32;
    core::iter::from_fn(move || {
        let b = s.wrapping_index(i).read();
        i += 1;
        (b != 0).then_some(b)
    })
}

orig! {
    /// CRT `toupper` (0x823A0E, locale-aware: `__getptd` / `___updatetlocinfo`).
    pub fn toupper(c: i32) -> i32 = cdecl 0x823A0E => toupper_c_locale;
}

/// Phases B/C `toupper`: the "C" locale (the game never calls `setlocale`).
pub fn toupper_c_locale(c: i32) -> i32 {
    if (b'a' as i32..=b'z' as i32).contains(&c) {
        c - 0x20
    } else {
        c
    }
}

/// `CKeyGen::GetKey(const char* str, int32 size)` (0x53CED0, cdecl): `size` bytes (none if
/// `size <= 0`, signed `jle`), embedded NULs included.
#[hook(addr = 0x53CED0, conv = cdecl, name = "CKeyGen::GetKey-sized", expect = "56 8B 74 24 0C")]
pub fn get_key_sized(s: GPtr<u8>, size: i32) -> u32 {
    get_key_sized_with(&KEY_TABLE.get(), s, size)
}

/// [`get_key_sized`] with a given table (the L1 spec passes the image's).
pub fn get_key_sized_with(table: &[u32; 256], s: GPtr<u8>, size: i32) -> u32 {
    let n = size.max(0);
    key_of(table, 0xFFFF_FFFF, (0..n).map(|i| s.wrapping_index(i).read()))
}

/// `CKeyGen::GetKey(const char* str)` (0x53CF00, cdecl).
#[hook(addr = 0x53CF00, conv = cdecl, name = "CKeyGen::GetKey-null-terminated", expect = "8B 54 24 04 33 C9")]
pub fn get_key(s: GPtr<u8>) -> u32 {
    get_key_with(&KEY_TABLE.get(), s)
}

/// [`get_key`] with a given table.
pub fn get_key_with(table: &[u32; 256], s: GPtr<u8>) -> u32 {
    key_of(table, 0xFFFF_FFFF, c_bytes(s))
}

/// `CKeyGen::GetUppercaseKey(const char* str)` (0x53CF30, cdecl): `toupper` per byte (low byte of
/// its result used).
#[hook(addr = 0x53CF30, conv = cdecl, name = "CKeyGen::GetUppercaseKey-", expect = "56 57 8B 7C 24 0C")]
pub fn get_uppercase_key(s: GPtr<u8>) -> u32 {
    let table = KEY_TABLE.get();
    let mut key = 0xFFFF_FFFFu32;
    for b in c_bytes(s) {
        let u = toupper(b as i32) as u8;
        key = step(&table, key, u);
    }
    key
}

/// `CKeyGen::AppendStringToKey(uint32 key, const char* str)` (0x53CF70, cdecl; one instruction of
/// the loop sits in a protection chunk at 0x404785).
#[hook(addr = 0x53CF70, conv = cdecl, name = "CKeyGen::AppendStringToKey", expect = "8B 54 24 08 8B 44 24 04")]
pub fn append_string_to_key(key: u32, s: GPtr<u8>) -> u32 {
    append_string_to_key_with(&KEY_TABLE.get(), key, s)
}

/// [`append_string_to_key`] with a given table.
pub fn append_string_to_key_with(table: &[u32; 256], key: u32, s: GPtr<u8>) -> u32 {
    key_of(table, key, c_bytes(s))
}

/// This module's registrations (collected by [`crate::hooks`]).
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_GET_KEY_SIZED,
    HOOK_GET_KEY,
    HOOK_GET_UPPERCASE_KEY,
    HOOK_APPEND_STRING_TO_KEY,
];

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    fn p(s: &[u8]) -> GPtr<u8> {
        GPtr::from_raw(s.as_ptr() as *mut u8)
    }

    #[test]
    fn keys() {
        let t = crc_table();
        assert_eq!(&t[..4], &[0, 0x7707_3096, 0xEE0E_612C, 0x9909_51BA]);
        // CRC-32("123456789") = 0xCBF43926; without the final xor: !0xCBF43926
        assert_eq!(get_key(p(b"123456789\0")), !0xCBF4_3926);
        assert_eq!(get_key_sized(p(b"123456789"), 9), !0xCBF4_3926);
        assert_eq!(get_key_sized(p(b"x"), 0), 0xFFFF_FFFF);
        assert_eq!(get_key_sized(p(b"x"), -5), 0xFFFF_FFFF);
        assert_eq!(get_key(p(b"\0")), 0xFFFF_FFFF);
        let k = get_key(p(b"abc\0"));
        assert_eq!(append_string_to_key(get_key(p(b"a\0")), p(b"bc\0")), k);
        assert_eq!(get_uppercase_key(p(b"aBc\0")), get_key(p(b"ABC\0")));
        assert_eq!(get_uppercase_key(p(b"\xE9\0")), get_key(p(b"\xE9\0")));
    }
}
