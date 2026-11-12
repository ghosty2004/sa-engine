//! `CPedType` initialisation (batch `mvpf-ped-2`): `CPedType::Initialise` and `LoadPedData`. Ported from the 1.0 US
//! bytes; unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// CRT `operator new` (0x821195)
    fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
    /// CRT `eh_vector_constructor_iterator(void* p, size_t size, int count, ctor, dtor)` (0x821D30, stdcall)
    fn eh_vector_constructor_iterator(p: GPtr<u8>, size: u32, count: u32, ctor: u32, dtor: u32) = stdcall 0x821d30 => unported;
    /// `CPedType::LoadPedData()` (0x608B30)
    fn load_ped_data_call() = cdecl 0x608b30 => load_ped_data;
    /// `CFileMgr::OpenFile(const char*, const char*)` (0x538900)
    fn file_open(name: GPtr<u8>, mode: GPtr<u8>) -> i32 = cdecl 0x538900 => unported;
    /// `CFileMgr::CloseFile(FILE*)` (0x5389D0)
    fn file_close(f: i32) -> i32 = cdecl 0x5389d0 => unported;
    /// `CFileLoader::LoadLine(FILE*)` (0x536F80)
    fn load_line(f: i32) -> GPtr<u8> = cdecl 0x536f80 => unported;
    /// CRT `sscanf(s, "%s", buf)` (0x8220AD)
    fn sscanf1(s: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>) -> i32 = cdecl 0x8220ad => unported;
    /// CRT `strtok(char*, const char*)` (0x82244B)
    fn strtok(s: GPtr<u8>, delim: GPtr<u8>) -> GPtr<u8> = cdecl 0x82244b => unported;
    /// `CPedType::FindPedType(const char*)` (0x608790)
    fn find_ped_type(name: GPtr<u8>) -> i32 = cdecl 0x608790 => crate::ped_type::find_ped_type;
}

/// `CPedType::ms_apPedTypes` (0xC0BBE8)
const PED_TYPES: usize = 0xC0BBE8;

/// `CPedType::Initialise()` (0x608E40, SEH frame for the `new[]`): `operator new(0x284)`; when it
/// succeeds the array cookie 0x20 is stored and the 32 `CPedType`s (0x14 bytes) at +4 are constructed
/// with `eh_vector_constructor_iterator` (ctor 0x608AE0, dtor 0x608780); `ms_apPedTypes` = the array (or
/// null when `new` failed). Then every entry's five dwords are zeroed (through the re-read global, so a
/// failed `new` writes from address 0, as the original) and `LoadPedData()` runs.
#[hook(addr = 0x608e40, conv = cdecl, name = "CPedType::Initialise", expect = "6A FF 68 1B EB 83 00",
       route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "one-time boot initialiser (allocates ms_apPedTypes, then loads ped.dat)")]
pub fn ped_type_initialise() {
    let p = operator_new(0x284);
    let arr = if p.is_null() {
        GPtr::NULL
    } else {
        p.cast::<u32>().write(0x20);
        let a = p.byte_add(4);
        eh_vector_constructor_iterator(a, 0x14, 0x20, 0x608AE0, 0x608780);
        a
    };
    GPtr::<GPtr<u8>>::from_addr(PED_TYPES).write(arr);
    let mut off = 0isize;
    while off < 0x280 {
        let e = GPtr::<GPtr<u8>>::from_addr(PED_TYPES).read().byte_add(off);
        for k in 0..5 {
            e.byte_add(4 * k).cast::<u32>().write(0);
        }
        off += 0x14;
    }
    load_ped_data_call();
}

/// The relationship keywords (`"Hate"` 0x86D06C, `"Dislike"` 0x86D064, `"Like"` 0x86D05C, `"Respect"`
/// 0x86D054; compared with their terminators, `repe cmpsb`) and the `CPedType` dword each one sets.
const KEYWORDS: [(usize, usize, isize); 4] = [(0x86D06C, 5, 0x10), (0x86D064, 8, 0xC), (0x86D05C, 5, 4), (0x86D054, 8, 0)];

/// `CPedType::LoadPedData()` (0x608B30): reads `DATA\PED.DAT` (0x86D074, mode `"r"` 0x85A53C) line by
/// line (`CFileLoader::LoadLine`), skipping lines starting with `#` or empty. `sscanf(line, "%s", buf)`;
/// a relationship keyword ORs `1 << FindPedType(token)` (0 for types >= 32; negative types shift by
/// their low 5 bits) over the `strtok(" ,\t")` tokens after the keyword into the current type's dword
/// (Hate +0x10, Dislike +0xC, Like +4, Respect +0; entries of 0x14 bytes at `ms_apPedTypes`); any
/// other word selects the current type (`FindPedType`). The current type starts at 32 (one past the
/// array, as the original). The file is closed at the end.
#[hook(addr = 0x608b30, conv = cdecl, name = "CPedType::LoadPedData", expect = "83 EC 24 53 55 56",
       route = "split", hazards = "uninit-locals,heap-writes", recs = "grove-walk",
       note = "file I/O; a blank-but-whitespace line compares the previous (first: uninitialised) sscanf buffer")]
pub fn load_ped_data() {
    let delim = GPtr::<u8>::from_addr(0x86A8C8);
    let f = file_open(GPtr::from_addr(0x86D074), GPtr::from_addr(0x85A53C));
    let mut cur: i32 = 0x20;
    let mut buf = [0u32; 8];
    let bp = GPtr::<u32>::from_raw(buf.as_mut_ptr()).cast::<u8>();
    let mut line = load_line(f);
    while !line.is_null() {
        let c = line.read();
        if c != b'#' && c != 0 {
            sscanf1(line, GPtr::from_addr(0x85C714), bp);
            let kw = KEYWORDS.iter().find(|(s, n, _)| (0..*n).all(|k| bp.byte_add(k as isize).read() == GPtr::<u8>::from_addr(s + k).read()));
            if let Some(&(_, _, off)) = kw {
                strtok(line, delim);
                let mut flags = 0u32;
                let mut t = strtok(GPtr::NULL, delim);
                while !t.is_null() {
                    let ty = find_ped_type(t);
                    flags |= if ty < 0x20 { 1u32 << (ty & 31) } else { 0 };
                    t = strtok(GPtr::NULL, delim);
                }
                let arr = GPtr::<GPtr<u8>>::from_addr(PED_TYPES).read();
                arr.byte_add(cur as isize * 0x14 + off).cast::<u32>().write(flags);
            } else {
                cur = find_ped_type(bp);
            }
        }
        line = load_line(f);
    }
    file_close(f);
}

pub const HOOKS: &[HookReg] = &[HOOK_PED_TYPE_INITIALISE, HOOK_LOAD_PED_DATA];
