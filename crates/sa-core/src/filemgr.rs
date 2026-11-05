//! `CFileMgr` (0x5386F0–0x538A5A, not `InitUserDirectories`): directory strings and thin wrappers
//! over the statically linked CRT's stdio (`fopen` 0x8232D8, `fread` 0x823521, `fwrite` 0x823674,
//! `fseek` 0x82374F, `fgets` 0x823798, `fclose` 0x82318B, `_getcwd` 0x836E91, `_chdir` 0x836F1E) and
//! RenderWare's file interface (`RwOsGetFileInterface` = `RwEngineInstance + 0xC4`: `rwfseek` at
//! +0x20, `rwftell` at +0x28).
//!
//! Ported from the 1.0 US bytes. gta-reversed's `FileMgr.cpp` differs and the original is kept:
//! - `ChangeDir` does **not** clear `ms_dirName`: a path without a leading `\` is appended to the
//!   current `ms_dirName` (gr clears it first, so a relative path replaces it); no length checks
//!   (gr returns -1/EINVAL); the `\` is appended with `strcat(ms_dirName, "\\")`.
//! - `SetDir`, `SetDirMyDocuments`: no length checks; plain `_chdir` (gr: wide-char variants).
//! - `LoadFile` ignores `size`, reads 0x4000-byte chunks until a short read, NUL-terminates the
//!   buffer, returns the byte count; -1 when `fopen` fails, and -1 **without closing** if `fread`
//!   returned a negative count (gr: one `fread` of `size` bytes, no terminator; catalog
//!   gr-deviations.csv `FileMgr.cpp:238`).
//! - `Seek` returns `fseek(...) != 0` (true on failure), as gr does.
//! - `GetTotalSize`/`Tell` use the RW file interface (gr: `ftell`/`fseek`).
//! - `GetErrorReadWrite` returns `file->_flag & _IOERR` (0x20) as an int (gr: `(bool)ferror`).

use crate::mem::GPtr;
use crate::{globals, hook, orig};

globals! {
    list GLOBALS;
    /// `CFileMgr::ms_dirName`
    pub static DIR_NAME @ 0xB71A60: [u8; 128] = [0; 128];
    /// `CFileMgr::ms_rootDirName`
    pub static ROOT_DIR_NAME @ 0xB71AE0: [u8; 128] = [0; 128];
}

/// `"\\"` (`.rdata` 0x859F7C, two bytes copied with a word move)
pub const BACKSLASH_STR: [u8; 2] = *b"\\\0";
/// `"wb"` (0x860ADC)
pub const MODE_WB: u32 = 0x860ADC;
/// `"a"` (0x863A2C)
pub const MODE_A: u32 = 0x863A2C;
/// `RwEngineInstance` (0xC97B24); the file functions start at +0xC4.
pub const RW_ENGINE_INSTANCE: u32 = 0xC97B24;

orig! {
    /// CRT `fopen` (0x8232D8)
    pub fn fopen(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x8232D8 => unported;
    /// CRT `fread` (0x823521)
    pub fn fread(buf: GPtr<u8>, size: u32, n: u32, f: GPtr<u8>) -> u32 = cdecl 0x823521 => unported;
    /// CRT `fwrite` (0x823674)
    pub fn fwrite(buf: GPtr<u8>, size: u32, n: u32, f: GPtr<u8>) -> u32 = cdecl 0x823674 => unported;
    /// CRT `fseek` (0x82374F)
    pub fn fseek(f: GPtr<u8>, offset: i32, origin: i32) -> i32 = cdecl 0x82374F => unported;
    /// CRT `fgets` (0x823798)
    pub fn fgets(buf: GPtr<u8>, n: i32, f: GPtr<u8>) -> GPtr<u8> = cdecl 0x823798 => unported;
    /// CRT `fclose` (0x82318B)
    pub fn fclose(f: GPtr<u8>) -> i32 = cdecl 0x82318B => unported;
    /// CRT `_getcwd` (0x836E91)
    pub fn getcwd(buf: GPtr<u8>, n: i32) -> GPtr<u8> = cdecl 0x836E91 => unported;
    /// CRT `_chdir` (0x836F1E)
    pub fn chdir(path: GPtr<u8>) -> i32 = cdecl 0x836F1E => unported;
    /// `InitUserDirectories` (0x744FB0, CFileMgr::InitUserDirectories in the catalogue)
    pub fn init_user_directories() -> GPtr<u8> = cdecl 0x744FB0 => unported;
    /// `RwOsGetFileInterface` (0x804130): `RwEngineInstance + 0xC4`
    pub fn rw_os_get_file_interface() -> GPtr<u8> = cdecl 0x804130 => unported;
    /// `RwEngineInstance->fileFuncs.rwftell` (interface +0x28)
    pub fn rwftell(f: GPtr<u8>) -> i32 = cdecl table 0xC97B24 0xEC => unported;
    /// `RwEngineInstance->fileFuncs.rwfseek` (interface +0x20)
    pub fn rwfseek(f: GPtr<u8>, offset: i32, whence: i32) -> i32 = cdecl table 0xC97B24 0xE4 => unported;
}

fn strlen(s: GPtr<u8>) -> i32 {
    let mut n = 0;
    while s.wrapping_index(n).read() != 0 {
        n += 1;
    }
    n
}

/// `strcpy(dst, src)`: copies up to and including the NUL, forward.
pub fn strcpy(dst: GPtr<u8>, src: GPtr<u8>) {
    let mut i = 0;
    loop {
        let c = src.wrapping_index(i).read();
        dst.wrapping_index(i).write(c);
        if c == 0 {
            break;
        }
        i += 1;
    }
}

/// `strcat(dst, src)` as MSVC inlines it: the length of `src` (with its NUL) is taken first, then
/// that many bytes are copied to the end of `dst`.
pub fn strcat(dst: GPtr<u8>, src: GPtr<u8>) {
    let n = strlen(src) + 1;
    let d = dst.wrapping_index(strlen(dst));
    for i in 0..n {
        d.wrapping_index(i).write(src.wrapping_index(i).read());
    }
}

/// The `strcat(ms_dirName, path)` + trailing `\` tail shared by `ChangeDir` and `SetDir`.
fn append_dir(path: GPtr<u8>) {
    let dir = DIR_NAME.ptr().cast::<u8>();
    strcat(dir, path);
    let len = strlen(path);
    if path.wrapping_index(len - 1).read() != b'\\' {
        // word copy of "\\\0"
        let end = dir.wrapping_index(strlen(dir));
        end.cast::<[u8; 2]>().write(BACKSLASH_STR);
    }
}

/// `CFileMgr::Initialise()` (0x5386F0, cdecl): `_getcwd(ms_rootDirName, 128)`, then `"\\"` appended.
#[hook(addr = 0x5386F0, conv = cdecl, name = "CFileMgr::Initialise", expect = "57 68 80 00 00 00")]
pub fn initialise() {
    let root = ROOT_DIR_NAME.ptr().cast::<u8>();
    getcwd(root, 0x80);
    let end = root.wrapping_index(strlen(root));
    end.cast::<[u8; 2]>().write(BACKSLASH_STR);
}

/// `CFileMgr::ChangeDir(path)` (0x538730, cdecl; part of the copy loop is a protection chunk at
/// 0x402353): `\path` starts from the root; otherwise `path` is appended to the current dir.
/// Returns `_chdir(ms_dirName)`.
#[hook(addr = 0x538730, conv = cdecl, name = "CFileMgr::ChangeDir", expect = "53 8B 5C 24 08")]
pub fn change_dir(path: GPtr<u8>) -> i32 {
    let mut path = path;
    if path.read() == b'\\' {
        strcpy(DIR_NAME.ptr().cast(), ROOT_DIR_NAME.ptr().cast());
        path = path.byte_add(1);
    }
    if path.read() != 0 {
        append_dir(path);
    }
    chdir(DIR_NAME.ptr().cast())
}

/// `CFileMgr::SetDir(path)` (0x5387D0, cdecl): `ms_dirName = root + path (+ "\\")`, `_chdir`.
/// Not hookable: the inlined `strcpy` loop branches back to 0x5387D2, inside the entry bytes an
/// inline hook would replace.
pub fn set_dir(path: GPtr<u8>) -> i32 {
    strcpy(DIR_NAME.ptr().cast(), ROOT_DIR_NAME.ptr().cast());
    if path.read() != 0 {
        append_dir(path);
    }
    chdir(DIR_NAME.ptr().cast())
}

/// `CFileMgr::SetDirMyDocuments()` (0x538860, cdecl): `strcpy(ms_dirName, InitUserDirectories())`
/// (no length check), `_chdir`.
#[hook(addr = 0x538860, conv = cdecl, name = "CFileMgr::SetDirMyDocuments", expect = "E8 4B C7 20 00")]
pub fn set_dir_my_documents() -> i32 {
    let user = init_user_directories();
    strcpy(DIR_NAME.ptr().cast(), user);
    chdir(DIR_NAME.ptr().cast())
}

/// `CFileMgr::LoadFile(path, buf, size, mode)` (0x538890, cdecl): see the module docs.
#[hook(addr = 0x538890, conv = cdecl, name = "CFileMgr::LoadFile", expect = "8B 44 24 10 8B 4C 24 04")]
pub fn load_file(path: GPtr<u8>, buf: GPtr<u8>, _size: u32, mode: GPtr<u8>) -> i32 {
    let f = fopen(path, mode);
    if f.is_null() {
        return -1;
    }
    let mut total = 0i32;
    loop {
        let n = fread(buf.wrapping_index(total), 1, 0x4000, f) as i32;
        if n < 0 {
            return -1;
        }
        total = total.wrapping_add(n);
        if n != 0x4000 {
            break;
        }
    }
    buf.wrapping_index(total).write(0);
    fclose(f);
    total
}

/// `CFileMgr::OpenFile(path, mode)` (0x538900, cdecl): `jmp fopen`.
#[hook(addr = 0x538900, conv = cdecl, name = "CFileMgr::OpenFile", expect = "E9 D3 A9 2E 00 90 90 90")]
pub fn open_file(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> {
    fopen(path, mode)
}

/// `CFileMgr::OpenFileForWriting(path)` (0x538910, cdecl): `fopen(path, "wb")`.
#[hook(addr = 0x538910, conv = cdecl, name = "CFileMgr::OpenFileForWriting", expect = "8B 44 24 04 68 DC 0A 86 00")]
pub fn open_file_for_writing(path: GPtr<u8>) -> GPtr<u8> {
    fopen(path, GPtr::from_addr(MODE_WB as usize))
}

/// `CFileMgr::OpenFileForAppending(path)` (0x538930, cdecl): `fopen(path, "a")`.
#[hook(addr = 0x538930, conv = cdecl, name = "CFileMgr::OpenFileForAppending", expect = "8B 44 24 04 68 2C 3A 86 00")]
pub fn open_file_for_appending(path: GPtr<u8>) -> GPtr<u8> {
    fopen(path, GPtr::from_addr(MODE_A as usize))
}

/// `CFileMgr::Read(file, buf, size)` (0x538950, cdecl): `fread(buf, 1, size, file)`.
#[hook(addr = 0x538950, conv = cdecl, name = "CFileMgr::Read", expect = "8B 44 24 04 8B 4C 24 0C")]
pub fn read(f: GPtr<u8>, buf: GPtr<u8>, size: u32) -> u32 {
    fread(buf, 1, size, f)
}

/// `CFileMgr::Write(file, buf, size)` (0x538970, cdecl): `fwrite(buf, 1, size, file)`.
#[hook(addr = 0x538970, conv = cdecl, name = "CFileMgr::Write", expect = "8B 44 24 04 8B 4C 24 0C")]
pub fn write(f: GPtr<u8>, buf: GPtr<u8>, size: u32) -> u32 {
    fwrite(buf, 1, size, f)
}

/// `CFileMgr::Seek(file, offset, origin)` (0x538990, cdecl, bool in AL): `fseek(...) != 0`.
#[hook(addr = 0x538990, conv = cdecl, name = "CFileMgr::Seek", expect = "8B 44 24 0C 8B 4C 24 08")]
pub fn seek(f: GPtr<u8>, offset: i32, origin: i32) -> bool {
    fseek(f, offset, origin) != 0
}

/// `CFileMgr::ReadLine(file, buf, n)` (0x5389B0, cdecl, bool in AL): `fgets(buf, n, file) != NULL`.
#[hook(addr = 0x5389B0, conv = cdecl, name = "CFileMgr::ReadLine", expect = "8B 44 24 04 8B 4C 24 0C")]
pub fn read_line(f: GPtr<u8>, buf: GPtr<u8>, n: i32) -> bool {
    !fgets(buf, n, f).is_null()
}

/// `CFileMgr::CloseFile(file)` (0x5389D0, cdecl): `jmp fclose`.
#[hook(addr = 0x5389D0, conv = cdecl, name = "CFileMgr::CloseFile", expect = "E9 B6 A7 2E 00 90 90 90")]
pub fn close_file(f: GPtr<u8>) -> i32 {
    fclose(f)
}

/// `CFileMgr::GetTotalSize(file)` (0x5389E0, cdecl): through the RW file interface, fetched anew
/// before each call: `pos = tell; seek(0, SEEK_END); size = tell; seek(pos, SEEK_SET)`.
#[hook(addr = 0x5389E0, conv = cdecl, name = "CFileMgr::GetTotalSize", expect = "53 56 57 E8 48 B7 2C 00")]
pub fn get_total_size(f: GPtr<u8>) -> i32 {
    rw_os_get_file_interface();
    let pos = rwftell(f);
    rw_os_get_file_interface();
    rwfseek(f, 0, 2);
    rw_os_get_file_interface();
    let size = rwftell(f);
    rw_os_get_file_interface();
    rwfseek(f, pos, 0);
    size
}

/// `CFileMgr::Tell(file)` (0x538A20, cdecl): `RwOsGetFileInterface()->rwftell(file)` (tail jump).
#[hook(addr = 0x538A20, conv = cdecl, name = "CFileMgr::Tell", expect = "E8 0B B7 2C 00")]
pub fn tell(f: GPtr<u8>) -> i32 {
    rw_os_get_file_interface();
    rwftell(f)
}

/// `CFileMgr::GetErrorReadWrite(file)` (0x538A50, cdecl): `file->_flag & 0x20` (EAX, not a bool).
#[hook(addr = 0x538A50, conv = cdecl, name = "CFileMgr::GetErrorReadWrite", expect = "8B 44 24 04 8B 40 0C")]
pub fn get_error_read_write(f: GPtr<u8>) -> u32 {
    f.byte_add(0xC).cast::<u32>().read() & 0x20
}

/// This module's registrations (collected by [`crate::hooks`]).
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_INITIALISE,
    HOOK_CHANGE_DIR,
    HOOK_SET_DIR_MY_DOCUMENTS,
    HOOK_LOAD_FILE,
    HOOK_OPEN_FILE,
    HOOK_OPEN_FILE_FOR_WRITING,
    HOOK_OPEN_FILE_FOR_APPENDING,
    HOOK_READ,
    HOOK_WRITE,
    HOOK_SEEK,
    HOOK_READ_LINE,
    HOOK_CLOSE_FILE,
    HOOK_GET_TOTAL_SIZE,
    HOOK_TELL,
    HOOK_GET_ERROR_READ_WRITE,
];

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    fn p(s: &[u8]) -> GPtr<u8> {
        GPtr::from_raw(s.as_ptr() as *mut u8)
    }

    #[test]
    fn string_helpers() {
        let mut b = [0u8; 16];
        let d = GPtr::from_raw(b.as_mut_ptr());
        strcpy(d, p(b"ab\0"));
        strcat(d, p(b"cd\0"));
        assert_eq!(&b[..5], b"abcd\0");
        let mut flag = [0u8; 16];
        flag[0xC] = 0x2F;
        assert_eq!(get_error_read_write(GPtr::from_raw(flag.as_mut_ptr())), 0x20);
    }
}
