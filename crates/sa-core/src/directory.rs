//! `CDirectory` (0x532290–0x53252F): the entry table of an IMG archive.
//!
//! Ported from the 1.0 US bytes; gta-reversed's `Directory.cpp` differs in these points (the original
//! is kept):
//! - `AddItem` (0x532310): no duplicate check (gr `#ifdef FIX_BUGS`, catalog/gr-deviations.csv
//!   `Directory.cpp:52`); a full table prints "Too many objects without modelinfo structures\n" with
//!   the CRT `printf` (0x821982).
//! - the empty constructor (0x532290) writes `m_pEntries`, `m_nCapacity`, `m_nNumEntries` only; the
//!   ownership byte at +0x0C is left as it was (gr's default member initialiser clears it).
//! - `FindItem(name, pos, size)` / `FindItem(hash, pos, size)` store `*size` (zero-extended `uint16`)
//!   before `*pos` and re-read the entry for `pos` (visible only when the outputs alias the entry).
//! - `ReadDirFile` inlines `AddItem` (same message on overflow) and reads the entry count as a
//!   `uint32` counted down to 0.

use crate::layouts::size;
use crate::mem::{GBool, GPtr, GameData};
use crate::{globals, hook, orig};

/// `CDirectory::DirectoryInfo` (0x20 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectoryInfo {
    /// +0x00 `Pos` (offset in sectors, stream id in the top byte)
    pub pos: u32,
    /// +0x04 `Size` (sectors)
    pub size: u16,
    /// +0x06 `SizeInArchive`
    pub size_in_archive: u16,
    /// +0x08 `Name` (NUL-terminated)
    pub name: [u8; 24],
}

/// `CDirectory` (0x10 bytes on 32-bit targets).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CDirectory {
    /// +0x00 `m_pEntries`
    pub entries: GPtr<DirectoryInfo>,
    /// +0x04 `m_nCapacity`
    pub capacity: u32,
    /// +0x08 `m_nNumEntries`
    pub num_entries: u32,
    /// +0x0C `m_bOwnsEntries`
    pub owns_entries: GBool,
    pub _pad: [u8; 3],
}

// SAFETY: integers, bytes and game pointers only.
#[allow(unsafe_code)]
unsafe impl GameData for DirectoryInfo {}
// SAFETY: as above.
#[allow(unsafe_code)]
unsafe impl GameData for CDirectory {}

const _: () = assert!(core::mem::size_of::<DirectoryInfo>() == size::CDirectory_DirectoryInfo);
#[cfg(target_pointer_width = "32")]
const _: () = assert!(core::mem::size_of::<CDirectory>() == size::CDirectory);

globals! {
    list GLOBALS;
}

/// "Too many objects without modelinfo structures\n" (0x863880)
pub const MSG_TOO_MANY: u32 = 0x863880;
/// "rb" (0x859C6C)
pub const MODE_RB: u32 = 0x859C6C;

orig! {
    /// CRT `operator new` (0x82119A)
    pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// CRT `free` (0x8214BD; the destructor's `delete[]`)
    pub fn free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// CRT `printf` (0x821982), one-argument form
    pub fn printf(fmt: GPtr<u8>) -> i32 = cdecl 0x821982 => unported;
    /// CRT `_stricmp` (0x8229B6)
    pub fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// `CFileMgr::OpenFile` (0x538900)
    pub fn open_file(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => crate::filemgr::open_file;
    /// `CFileMgr::OpenFileForWriting` (0x538910)
    pub fn open_file_for_writing(path: GPtr<u8>) -> GPtr<u8> = cdecl 0x538910 => crate::filemgr::open_file_for_writing;
    /// `CFileMgr::Read` (0x538950)
    pub fn read(f: GPtr<u8>, buf: GPtr<u8>, size: u32) -> u32 = cdecl 0x538950 => crate::filemgr::read;
    /// `CFileMgr::Write` (0x538970)
    pub fn write(f: GPtr<u8>, buf: GPtr<u8>, size: u32) -> u32 = cdecl 0x538970 => crate::filemgr::write;
    /// `CFileMgr::CloseFile` (0x5389D0)
    pub fn close_file(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => crate::filemgr::close_file;
    /// `CDirectory::FindItem(name)` (0x532450)
    pub fn find_item_call(dir: GPtr<CDirectory>, name: GPtr<u8>) -> GPtr<DirectoryInfo> = thiscall 0x532450 => find_item;
    /// `CKeyGen::GetUppercaseKey` (0x53CF30)
    pub fn get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => crate::keygen::get_uppercase_key;
}

const ENTRIES: usize = core::mem::offset_of!(CDirectory, entries);
const CAPACITY: usize = core::mem::offset_of!(CDirectory, capacity);
const NUM: usize = core::mem::offset_of!(CDirectory, num_entries);
const OWNS: usize = core::mem::offset_of!(CDirectory, owns_entries);

fn field<T>(dir: GPtr<CDirectory>, off: usize) -> GPtr<T> {
    dir.byte_add(off as isize).cast()
}

/// `CDirectory::CDirectory()` (0x532290, thiscall, returns `this`).
#[hook(addr = 0x532290, conv = thiscall, name = "CDirectory::Constructor-Empty", expect = "8B C1 33 C9 89 48 08", this = "CDirectory")]
pub fn constructor_empty(dir: GPtr<CDirectory>) -> GPtr<CDirectory> {
    field::<u32>(dir, NUM).write(0);
    field::<u32>(dir, CAPACITY).write(0);
    field::<GPtr<DirectoryInfo>>(dir, ENTRIES).write(GPtr::NULL);
    dir
}

/// `CDirectory::CDirectory(capacity)` (0x5322A0, thiscall, `ret 4`): `operator new(capacity << 5)`
/// (32-bit wrapping, no overflow check).
#[hook(addr = 0x5322A0, conv = thiscall, name = "CDirectory::Constructor-Capacity", expect = "8B 44 24 04 56 8B F1", this = "CDirectory")]
pub fn constructor_capacity(dir: GPtr<CDirectory>, capacity: u32) -> GPtr<CDirectory> {
    field::<u32>(dir, CAPACITY).write(capacity);
    field::<u32>(dir, NUM).write(0);
    let p = operator_new(capacity << 5);
    field::<GPtr<u8>>(dir, ENTRIES).write(p);
    field::<GBool>(dir, OWNS).write(GBool::TRUE);
    dir
}

/// `CDirectory::~CDirectory()` (0x5322D0, thiscall): `free(m_pEntries)` when non-null and owned.
#[hook(addr = 0x5322D0, conv = thiscall, name = "CDirectory::Destructor", expect = "8B 01 85 C0 74 0E", this = "CDirectory")]
pub fn destructor(dir: GPtr<CDirectory>) {
    let d = dir.read();
    if !d.entries.is_null() && d.owns_entries.get() {
        free(d.entries.cast());
    }
}

/// `CDirectory::Init(capacity, entries)` (0x5322F0, thiscall, `ret 8`).
#[hook(addr = 0x5322F0, conv = thiscall, name = "CDirectory::Init", expect = "8B 54 24 04 33 C0", this = "CDirectory")]
pub fn init(dir: GPtr<CDirectory>, capacity: u32, entries: GPtr<DirectoryInfo>) {
    field::<u32>(dir, CAPACITY).write(capacity);
    field::<u32>(dir, NUM).write(0);
    field::<GPtr<DirectoryInfo>>(dir, ENTRIES).write(entries);
    field::<GBool>(dir, OWNS).write(GBool::FALSE);
}

/// The append both `AddItem` and the inlined copy in `ReadDirFile` perform: signed compare
/// `num < capacity`, a 32-byte copy, `++num`; otherwise the overflow message.
fn add(dir: GPtr<CDirectory>, info: DirectoryInfo) {
    let d = dir.read();
    if (d.num_entries as i32) < d.capacity as i32 {
        d.entries.wrapping_index(d.num_entries as i32).write(info);
        let n = field::<u32>(dir, NUM).read();
        field::<u32>(dir, NUM).write(n.wrapping_add(1));
    } else {
        printf(GPtr::from_addr(MSG_TOO_MANY as usize));
    }
}

/// `CDirectory::AddItem(const DirectoryInfo&)` (0x532310, thiscall, `ret 4`).
#[hook(addr = 0x532310, conv = thiscall, name = "CDirectory::AddItem-", expect = "8B C1 8B 48 04 57", this = "CDirectory")]
pub fn add_item(dir: GPtr<CDirectory>, info: GPtr<DirectoryInfo>) {
    add(dir, info.read())
}

/// `CDirectory::ReadDirFile(filename)` (0x532350, thiscall, `ret 4`): opens with "rb", reads a
/// 4-byte header (ignored) and a `uint32` count, then that many 32-byte entries (`AddItem` inlined).
#[hook(addr = 0x532350, conv = thiscall, name = "CDirectory::ReadDirFile", expect = "83 EC 28 8B 44 24 2C", this = "CDirectory")]
pub fn read_dir_file(dir: GPtr<CDirectory>, filename: GPtr<u8>) {
    let f = open_file(filename, GPtr::from_addr(MODE_RB as usize));
    let mut header = [0u8; 5];
    read(f, GPtr::from_raw(header.as_mut_ptr()), 4);
    // The count is read into the stack slot of the `filename` argument ([esp+4]); if the read
    // fails it keeps the pointer value, as in the original.
    let mut count = filename.addr() as u32;
    read(f, GPtr::from_raw((&raw mut count).cast()), 4);
    while count != 0 {
        count = count.wrapping_sub(1);
        let mut info = DirectoryInfo {
            pos: 0,
            size: 0,
            size_in_archive: 0,
            name: [0; 24],
        };
        read(f, GPtr::from_raw((&raw mut info).cast()), 0x20);
        add(dir, info);
    }
    close_file(f);
}

/// `CDirectory::WriteDirFile(filename)` (0x532410, thiscall, `ret 4`, bool in AL): writes only the
/// entries (no header) and returns `written == num * 32`.
#[hook(addr = 0x532410, conv = thiscall, name = "CDirectory::WriteDirFile", expect = "8B 44 24 04 53 56 57", this = "CDirectory")]
pub fn write_dir_file(dir: GPtr<CDirectory>, filename: GPtr<u8>) -> bool {
    let f = open_file_for_writing(filename);
    let d = dir.read();
    let written = write(f, d.entries.cast(), d.num_entries << 5);
    close_file(f);
    written == field::<u32>(dir, NUM).read() << 5
}

/// `CDirectory::FindItem(name)` (0x532450, thiscall, `ret 4`): the first entry whose name equals
/// `name` under `_stricmp`; `m_nNumEntries` is re-read on every iteration (signed compare).
#[hook(addr = 0x532450, conv = thiscall, name = "CDirectory::FindItem-", expect = "53 55 56 8B F1", this = "CDirectory")]
pub fn find_item(dir: GPtr<CDirectory>, name: GPtr<u8>) -> GPtr<DirectoryInfo> {
    let mut i = 0i32;
    while i < field::<i32>(dir, NUM).read() {
        let e = field::<GPtr<DirectoryInfo>>(dir, ENTRIES).read().wrapping_index(i);
        if stricmp(e.cast::<u8>().byte_add(8), name) == 0 {
            return e;
        }
        i += 1;
    }
    GPtr::NULL
}

/// `CDirectory::FindItem(name, pos, size)` (0x5324A0, thiscall, `ret 0Ch`, bool in AL).
#[hook(addr = 0x5324A0, conv = thiscall, name = "CDirectory::FindItem-ByName", expect = "8B 44 24 04 50", this = "CDirectory")]
pub fn find_item_by_name(dir: GPtr<CDirectory>, name: GPtr<u8>, pos: GPtr<u32>, size: GPtr<u32>) -> bool {
    let e = find_item_call(dir, name);
    if e.is_null() {
        return false;
    }
    size.write(e.cast::<u8>().byte_add(4).cast::<u16>().read() as u32);
    pos.write(e.cast::<u32>().read());
    true
}

/// `CDirectory::FindItem(hash, pos, size)` (0x5324D0, thiscall, `ret 0Ch`, bool in AL): compares
/// `CKeyGen::GetUppercaseKey(entry name)` with `hash`.
#[hook(addr = 0x5324D0, conv = thiscall, name = "CDirectory::FindItem-ByHash", expect = "53 55 56 8B F1", this = "CDirectory")]
pub fn find_item_by_hash(dir: GPtr<CDirectory>, hash: u32, pos: GPtr<u32>, size: GPtr<u32>) -> bool {
    let mut i = 0i32;
    while i < field::<i32>(dir, NUM).read() {
        let e = field::<GPtr<DirectoryInfo>>(dir, ENTRIES).read().wrapping_index(i);
        if get_uppercase_key(e.cast::<u8>().byte_add(8)) == hash {
            let e = field::<GPtr<DirectoryInfo>>(dir, ENTRIES).read().wrapping_index(i);
            size.write(e.cast::<u8>().byte_add(4).cast::<u16>().read() as u32);
            let e = field::<GPtr<DirectoryInfo>>(dir, ENTRIES).read().wrapping_index(i);
            pos.write(e.cast::<u32>().read());
            return true;
        }
        i += 1;
    }
    false
}

/// This module's registrations (collected by [`crate::hooks`]).
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_CONSTRUCTOR_EMPTY,
    HOOK_CONSTRUCTOR_CAPACITY,
    HOOK_DESTRUCTOR,
    HOOK_INIT,
    HOOK_ADD_ITEM,
    HOOK_READ_DIR_FILE,
    HOOK_WRITE_DIR_FILE,
    HOOK_FIND_ITEM,
    HOOK_FIND_ITEM_BY_NAME,
    HOOK_FIND_ITEM_BY_HASH,
];

#[cfg(target_pointer_width = "32")]
const _: () = {
    use core::mem::offset_of;
    assert!(offset_of!(CDirectory, entries) == 0);
    assert!(offset_of!(CDirectory, capacity) == 4);
    assert!(offset_of!(CDirectory, num_entries) == 8);
    assert!(offset_of!(CDirectory, owns_entries) == 0xC);
};

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;
    use std::vec;

    fn entry(pos: u32, size: u16, name: &str) -> DirectoryInfo {
        let mut n = [0u8; 24];
        n[..name.len()].copy_from_slice(name.as_bytes());
        DirectoryInfo { pos, size, size_in_archive: 0, name: n }
    }

    #[test]
    fn init_and_hash_lookup() {
        let mut storage = vec![entry(0, 0, ""); 2];
        let mut d = CDirectory { entries: GPtr::NULL, capacity: 9, num_entries: 9, owns_entries: GBool(7), _pad: [0; 3] };
        let dp = GPtr::from_raw(&raw mut d);
        constructor_empty(dp);
        assert_eq!((d.capacity, d.num_entries, d.owns_entries), (0, 0, GBool(7)));
        init(dp, 2, GPtr::from_raw(storage.as_mut_ptr()));
        assert_eq!((d.capacity, d.num_entries, d.owns_entries), (2, 0, GBool::FALSE));
        let a = entry(0x1234, 7, "car.dff");
        let b = entry(0x99, 0xFFFF, "ped.txd");
        let crate_ptr = |e: &DirectoryInfo| GPtr::from_raw(e as *const _ as *mut DirectoryInfo);
        add_item(dp, crate_ptr(&a));
        add_item(dp, crate_ptr(&b));
        assert_eq!(d.num_entries, 2);
        assert_eq!(storage[1], b);
        let (mut pos, mut size) = (0u32, 0u32);
        let key = crate::keygen::get_key(GPtr::from_raw(c"PED.TXD".as_ptr() as *mut u8));
        assert!(find_item_by_hash(dp, key, GPtr::from_raw(&raw mut pos), GPtr::from_raw(&raw mut size)));
        assert_eq!((pos, size), (0x99, 0xFFFF));
        assert!(!find_item_by_hash(dp, key ^ 1, GPtr::from_raw(&raw mut pos), GPtr::from_raw(&raw mut size)));
    }
}
