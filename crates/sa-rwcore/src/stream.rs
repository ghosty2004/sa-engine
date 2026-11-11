//! STREAMMODULE (engine plugin 0x404) and `RwStream` (PORT-RW-1): open / initialise / read / write /
//! skip / close, chunk headers and the string chunk reader. Ported from the 1.0 US bytes;
//! `docs/re/specs/RW/RwStream.md` is the reading aid.
//!
//! `RwStream` (0x24): +0 type (1 FILE, 2 FILENAME, 3 MEMORY, 4 CUSTOM), +4 access (1 READ, 2 WRITE,
//! 3 APPEND), +8 position (unused here), +0xC fp / memory position / custom sfnclose, +0x10 memory nSize /
//! sfnread, +0x14 memory block / sfnwrite, +0x18 sfnskip, +0x1C custom data, +0x20 rwOwned.
//! Module `G + [0xC9792C]`: +0 the stream freelist. File functions are `G+0xC8..` (`RwFileFunctions`).
//!
//! Stack temporaries the originals pass to callees (`RwError`, chunk header locals, the 0x80-byte
//! string buffer) are Rust locals. `RwStreamFindChunk` and `RwStreamReadChunkHeaderInfo` use one of
//! their own argument slots as an out-parameter of `_rwStreamReadChunkHeader` (the length, resp. the
//! type); that stack write is not reproduced.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{
    error1, error2, g, module, p8, rd, rw_free, rw_malloc, rw_memory_alloc, rw_memory_free, rw_realloc, wr,
};

/// STREAMMODULE offset global (`module = G + [0xC9792C]`).
pub const STREAM_MODULE: usize = 0xC9792C;
/// STREAMMODULE instance counter.
const STREAM_INSTANCES: usize = 0xC97930;
/// Freelist entries per block (`.data`, 0x10) and blocks to preallocate (`.data`, 1).
const STREAM_FL_PER_BLOCK: usize = 0x8E2218;
const STREAM_FL_PREALLOC: usize = 0x8E221C;
/// The freelist's static storage.
const STREAM_FL_STATIC: u32 = 0xC97908;
/// Mode strings "rb", "wb", "ab".
const MODE_RB: u32 = 0x8E2220;
const MODE_WB: u32 = 0x8E2224;
const MODE_AB: u32 = 0x8E2228;

pub const TYPE_FILE: u32 = 1;
pub const TYPE_FILENAME: u32 = 2;
pub const TYPE_MEMORY: u32 = 3;
pub const TYPE_CUSTOM: u32 = 4;

/// Error codes (`_rwerror` arguments).
const E_ENDOFSTREAM: u32 = 5;
const E_INVSTREAMACCESSTYPE: u32 = 0xD;
const E_INVSTREAMTYPE: u32 = 0xE;
const E_RW_BADOPEN: u32 = 0x8000_0002;
const E_RW_BADVERSION: u32 = 0x8000_0004;
const E_RW_NOMEM: u32 = 0x8000_0013;
const E_RW_READ: u32 = 0x8000_001A;
const E_RW_WRITE: u32 = 0x8000_001C;

orig! {
    /// `RwFreeListCreateAndPreallocateSpace(entrySize, entriesPerBlock, alignment, numBlocks, fl, hint)` (0x801B70)
    pub fn rw_free_list_create_and_preallocate(size: u32, per_block: u32, align: u32, blocks: u32, fl: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl 0x801B70 => unported;
    /// `RwFreeListDestroy(fl)` (0x801B80)
    pub fn rw_free_list_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => unported;
    /// `G->fileFuncs.rwfopen(name, mode)` (G+0xC8)
    pub fn rw_fopen(name: GPtr<u8>, mode: GPtr<u8>) -> u32 = cdecl table 0xC97B24 0xC8 => unported;
    /// `G->fileFuncs.rwfclose(fp)` (G+0xCC)
    pub fn rw_fclose(fp: u32) -> i32 = cdecl table 0xC97B24 0xCC => unported;
    /// `G->fileFuncs.rwfread(buf, size, n, fp)` (G+0xD0)
    pub fn rw_fread(buf: GPtr<u8>, size: u32, n: u32, fp: u32) -> u32 = cdecl table 0xC97B24 0xD0 => unported;
    /// `G->fileFuncs.rwfwrite(buf, size, n, fp)` (G+0xD4)
    pub fn rw_fwrite(buf: GPtr<u8>, size: u32, n: u32, fp: u32) -> u32 = cdecl table 0xC97B24 0xD4 => unported;
    /// `G->fileFuncs.rwfeof(fp)` (G+0xE0)
    pub fn rw_feof(fp: u32) -> i32 = cdecl table 0xC97B24 0xE0 => unported;
    /// `G->fileFuncs.rwfseek(fp, offset, whence)` (G+0xE4)
    pub fn rw_fseek(fp: u32, offset: u32, whence: u32) -> i32 = cdecl table 0xC97B24 0xE4 => unported;
    /// `G->fileFuncs.rwftell(fp)` (G+0xEC)
    pub fn rw_ftell(fp: u32) -> i32 = cdecl table 0xC97B24 0xEC => unported;
    /// Custom stream `sfnclose(data)` (stored pointer +0xC)
    pub fn call_sfnclose(f: u32, data: u32) -> u32 = cdecl dyn => unported;
    /// Custom stream `sfnread(data, buf, len)` (stored pointer +0x10)
    pub fn call_sfnread(f: u32, data: u32, buf: GPtr<u8>, len: u32) -> u32 = cdecl dyn => unported;
    /// Custom stream `sfnwrite(data, buf, len)` (stored pointer +0x14)
    pub fn call_sfnwrite(f: u32, data: u32, buf: GPtr<u8>, len: u32) -> u32 = cdecl dyn => unported;
    /// Custom stream `sfnskip(data, offset)` (stored pointer +0x18)
    pub fn call_sfnskip(f: u32, data: u32, offset: u32) -> u32 = cdecl dyn => unported;

    /// `RwStreamRead(stream, buf, len)` (0x7EC9D0)
    pub fn rw_stream_read_call(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => rw_stream_read;
    /// `RwStreamSkip(stream, offset)` (0x7ECD00)
    pub fn rw_stream_skip_call(s: GPtr<u8>, offset: u32) -> GPtr<u8> = cdecl 0x7ECD00 => rw_stream_skip;
    /// `_rwStreamReadChunkHeader(stream, type, length, version, build)` (0x7ED0F0)
    pub fn read_chunk_header_call(s: GPtr<u8>, ty: GPtr<u32>, len: GPtr<u32>, ver: GPtr<u32>, build: GPtr<u32>) -> u32 = cdecl 0x7ED0F0 => read_chunk_header;
    /// `_rwChunkIsComplex(info)` (0x7ED1E0)
    pub fn chunk_is_complex_call(info: GPtr<u8>) -> u32 = cdecl 0x7ED1E0 => chunk_is_complex;
}

/// The stream freelist `G + [0xC9792C]` +0.
fn stream_free_list() -> GPtr<u8> {
    p8(rd::<u32>(module(STREAM_MODULE), 0))
}

/// `rep movsd` of `n >> 2` dwords then `rep movsb` of `n & 3` bytes, forward.
fn movs_copy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) {
    let words = n >> 2;
    for i in 0..words as isize {
        dst.byte_add(i * 4)
            .cast::<u32>()
            .write(src.byte_add(i * 4).cast::<u32>().read());
    }
    let base = (words * 4) as isize;
    for i in 0..(n & 3) as isize {
        dst.byte_add(base + i).write(src.byte_add(base + i).read());
    }
}

/// `cb_STREAMMODULE_RwEngine_Open(instance, offset, size)` (0x7EC780): stores the module offset,
/// creates the 0x24-byte stream freelist (alignment 4, entries per block / preallocation from `.data`, static storage
/// 0xC97908, hint 0x40404) into `G+offset`; NULL -> NULL, else counts the instance and returns it.
#[hook(addr = 0x7EC780, conv = cdecl, name = "cb_STREAMMODULE_RwEngine_Open", expect = "8B 0D 1C 22 8E 00")]
pub fn stream_module_open(instance: GPtr<u8>, offset: u32, _size: u32) -> GPtr<u8> {
    let prealloc = g::<u32>(STREAM_FL_PREALLOC).read();
    let per_block = g::<u32>(STREAM_FL_PER_BLOCK).read();
    g::<u32>(STREAM_MODULE).write(offset);
    let fl = rw_free_list_create_and_preallocate(0x24, per_block, 4, prealloc, p8(STREAM_FL_STATIC), 0x40404);
    wr::<u32>(module(STREAM_MODULE), 0, fl.addr() as u32);
    if rd::<u32>(module(STREAM_MODULE), 0) == 0 {
        return GPtr::NULL;
    }
    let c = g::<u32>(STREAM_INSTANCES);
    c.write(c.read().wrapping_add(1));
    instance
}

/// `cb_STREAMMODULE_RwEngine_Close(instance, offset, size)` (0x7EC7E0): destroys the freelist when
/// set (the pointer is not cleared), decrements the instance counter, returns the instance.
#[hook(addr = 0x7EC7E0, conv = cdecl, name = "cb_STREAMMODULE_RwEngine_Close", expect = "A1 24 7B C9 00")]
pub fn stream_module_close(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    let fl = stream_free_list();
    if !fl.is_null() {
        rw_free_list_destroy(fl);
    }
    let c = g::<u32>(STREAM_INSTANCES);
    c.write(c.read().wrapping_sub(1));
    instance
}

/// The per-type initialisation shared by `_rwStreamInitialize` and `RwStreamOpen` (identical code in
/// both). Returns false on failure (errors already raised).
fn init_by_type(s: GPtr<u8>, ty: u32, access: u32, data: GPtr<u8>) -> bool {
    match ty {
        TYPE_FILE => {
            if rw_ftell(data.addr() as u32) == -1 {
                return false;
            }
            wr::<u32>(s, 0xC, data.addr() as u32);
            true
        }
        TYPE_FILENAME => {
            let mode = match access {
                1 => MODE_RB,
                2 => MODE_WB,
                3 => MODE_AB,
                _ => {
                    error1(E_INVSTREAMACCESSTYPE);
                    error2(E_RW_BADOPEN, data.addr() as u32);
                    return false;
                }
            };
            let fp = rw_fopen(data, p8(mode));
            if fp == 0 {
                error2(E_RW_BADOPEN, data.addr() as u32);
                return false;
            }
            wr::<u32>(s, 0xC, fp);
            true
        }
        TYPE_MEMORY => match access {
            1 => {
                wr::<u32>(s, 0xC, 0);
                wr::<u32>(s, 0x10, rd::<u32>(data, 4));
                wr::<u32>(s, 0x14, rd::<u32>(data, 0));
                true
            }
            2 => {
                wr::<u32>(s, 0xC, 0);
                wr::<u32>(s, 0x10, 0);
                wr::<u32>(s, 0x14, 0);
                true
            }
            3 => {
                wr::<u32>(s, 0xC, rd::<u32>(data, 4));
                wr::<u32>(s, 0x10, rd::<u32>(data, 4));
                wr::<u32>(s, 0x14, rd::<u32>(data, 0));
                true
            }
            _ => {
                error1(E_INVSTREAMACCESSTYPE);
                false
            }
        },
        TYPE_CUSTOM => {
            for i in 0..5 {
                wr::<u32>(s, 0xC + i * 4, rd::<u32>(data, i * 4));
            }
            true
        }
        _ => {
            error1(E_INVSTREAMTYPE);
            false
        }
    }
}

/// `_rwStreamInitialize(stream, rwOwned, type, access, data)` (0x7EC810): NULL stream -> NULL; stores
/// type, access and rwOwned before the type switch; per-type setup as in `RwStreamOpen`.
#[hook(addr = 0x7EC810, conv = cdecl, name = "_rwStreamInitialize", expect = "83 EC 10 53 8B 5C 24 18")]
pub fn rw_stream_initialize(s: GPtr<u8>, owned: u32, ty: u32, access: u32, data: GPtr<u8>) -> GPtr<u8> {
    if s.is_null() {
        return GPtr::NULL;
    }
    wr::<u32>(s, 0, ty);
    wr::<u32>(s, 4, access);
    wr::<u32>(s, 0x20, owned);
    if init_by_type(s, ty, access, data) {
        s
    } else {
        GPtr::NULL
    }
}

/// `RwStreamRead(stream, buf, len)` (0x7EC9D0): bytes read. File: `fread`, a short read raises
/// E_RW_ENDOFSTREAM when `feof` else E_RW_READ. Memory: clamps to `nSize - pos` (raising
/// E_RW_ENDOFSTREAM), copies, advances. Custom: `sfnread`. Other types: E_RW_INVSTREAMTYPE, 0.
#[hook(addr = 0x7EC9D0, conv = cdecl, name = "RwStreamRead", expect = "83 EC 08 53 8B 5C 24 10")]
pub fn rw_stream_read(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 {
    match rd::<u32>(s, 0) {
        TYPE_FILE | TYPE_FILENAME => {
            let fp = rd::<u32>(s, 0xC);
            let n = rw_fread(buf, 1, len, fp);
            if n != len {
                if rw_feof(fp) != 0 {
                    error1(E_ENDOFSTREAM);
                } else {
                    error1(E_RW_READ);
                }
            }
            n
        }
        TYPE_MEMORY => {
            let avail = rd::<u32>(s, 0x10).wrapping_sub(rd::<u32>(s, 0xC));
            let mut len = len;
            if len > avail {
                len = avail;
                error1(E_ENDOFSTREAM);
            }
            let src = p8(rd::<u32>(s, 0x14).wrapping_add(rd::<u32>(s, 0xC)));
            movs_copy(buf, src, len);
            wr::<u32>(s, 0xC, rd::<u32>(s, 0xC).wrapping_add(len));
            len
        }
        TYPE_CUSTOM => call_sfnread(rd::<u32>(s, 0x10), rd::<u32>(s, 0x1C), buf, len),
        _ => {
            error1(E_INVSTREAMTYPE);
            0
        }
    }
}

/// `RwStreamWrite(stream, buf, len)` (0x7ECB30): stream or NULL. File: `fwrite`, a short write raises
/// E_RW_WRITE (0x8000001C). Memory: allocates a 0x200-byte block on first write (hint 0x30404), grows
/// by `max(len, 0x200)` (realloc hint 0x1030404; on failure E_RW_NOMEM with the growth), copies,
/// advances. Custom: `sfnwrite` != 0 -> stream. Other types: E_RW_INVSTREAMTYPE, NULL.
#[hook(addr = 0x7ECB30, conv = cdecl, name = "RwStreamWrite", expect = "83 EC 08 53 8B 5C 24 10")]
pub fn rw_stream_write(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> GPtr<u8> {
    match rd::<u32>(s, 0) {
        TYPE_FILE | TYPE_FILENAME => {
            if rw_fwrite(buf, 1, len, rd::<u32>(s, 0xC)) != len {
                error1(E_RW_WRITE);
                return GPtr::NULL;
            }
            s
        }
        TYPE_MEMORY => {
            if rd::<u32>(s, 0x14) == 0 {
                let b = rw_malloc(0x200, 0x30404);
                wr::<u32>(s, 0x14, b.addr() as u32);
                if b.is_null() {
                    error2(E_RW_NOMEM, 0x200);
                    return GPtr::NULL;
                }
                wr::<u32>(s, 0x10, 0x200);
            }
            let size = rd::<u32>(s, 0x10);
            if size.wrapping_sub(rd::<u32>(s, 0xC)) < len {
                let new_size = if len < 0x200 {
                    size.wrapping_add(0x200)
                } else {
                    size.wrapping_add(len)
                };
                let b = rw_realloc(p8(rd::<u32>(s, 0x14)), new_size, 0x0103_0404);
                if b.is_null() {
                    error2(E_RW_NOMEM, new_size.wrapping_sub(rd::<u32>(s, 0x10)));
                    return GPtr::NULL;
                }
                wr::<u32>(s, 0x14, b.addr() as u32);
                wr::<u32>(s, 0x10, new_size);
            }
            let dst = p8(rd::<u32>(s, 0x14).wrapping_add(rd::<u32>(s, 0xC)));
            movs_copy(dst, buf, len);
            wr::<u32>(s, 0xC, rd::<u32>(s, 0xC).wrapping_add(len));
            s
        }
        TYPE_CUSTOM => {
            if call_sfnwrite(rd::<u32>(s, 0x14), rd::<u32>(s, 0x1C), buf, len) != 0 {
                s
            } else {
                GPtr::NULL
            }
        }
        _ => {
            error1(E_INVSTREAMTYPE);
            GPtr::NULL
        }
    }
}

/// `RwStreamSkip(stream, offset)` (0x7ECD00): offset 0 -> stream (type unchecked). File: `fseek(SEEK_CUR)`
/// failing -> E_RW_ENDOFSTREAM if `feof`, NULL. Memory: past `nSize` -> pos = nSize,
/// E_RW_ENDOFSTREAM, NULL. Custom: `sfnskip` != 0 -> stream. Other types: E_RW_INVSTREAMTYPE, NULL.
#[hook(addr = 0x7ECD00, conv = cdecl, name = "RwStreamSkip", expect = "8B 44 24 08 83 EC 08")]
pub fn rw_stream_skip(s: GPtr<u8>, offset: u32) -> GPtr<u8> {
    if offset == 0 {
        return s;
    }
    match rd::<u32>(s, 0) {
        TYPE_FILE | TYPE_FILENAME => {
            let fp = rd::<u32>(s, 0xC);
            if rw_fseek(fp, offset, 1) != 0 {
                if rw_feof(fp) != 0 {
                    error1(E_ENDOFSTREAM);
                }
                return GPtr::NULL;
            }
            s
        }
        TYPE_MEMORY => {
            let np = offset.wrapping_add(rd::<u32>(s, 0xC));
            let size = rd::<u32>(s, 0x10);
            if np > size {
                wr::<u32>(s, 0xC, size);
                error1(E_ENDOFSTREAM);
                return GPtr::NULL;
            }
            wr::<u32>(s, 0xC, np);
            s
        }
        TYPE_CUSTOM => {
            if call_sfnskip(rd::<u32>(s, 0x18), rd::<u32>(s, 0x1C), offset) != 0 {
                s
            } else {
                GPtr::NULL
            }
        }
        _ => {
            error1(E_INVSTREAMTYPE);
            GPtr::NULL
        }
    }
}

/// `RwStreamClose(stream, data)` (0x7ECE20): FILE 1 (the caller owns the FILE*); FILENAME
/// `fclose == 0`; MEMORY 1, and for non-READ access with `data` set, `data = {block, position}`;
/// CUSTOM calls `sfnclose` when set, 1. Then frees an rwOwned stream. Other types: E_RW_INVSTREAMTYPE, 0
/// (not freed).
#[hook(addr = 0x7ECE20, conv = cdecl, name = "RwStreamClose", expect = "83 EC 08 56 8B 74 24 10")]
pub fn rw_stream_close(s: GPtr<u8>, data: GPtr<u8>) -> u32 {
    let result = match rd::<u32>(s, 0) {
        TYPE_FILE => 1,
        TYPE_FILENAME => (rw_fclose(rd::<u32>(s, 0xC)) == 0) as u32,
        TYPE_MEMORY => {
            if rd::<u32>(s, 4) != 1 && !data.is_null() {
                wr::<u32>(data, 0, rd::<u32>(s, 0x14));
                wr::<u32>(data, 4, rd::<u32>(s, 0xC));
            }
            1
        }
        TYPE_CUSTOM => {
            let f = rd::<u32>(s, 0xC);
            if f != 0 {
                call_sfnclose(f, rd::<u32>(s, 0x1C));
            }
            1
        }
        _ => {
            error1(E_INVSTREAMTYPE);
            return 0;
        }
    };
    if rd::<u32>(s, 0x20) != 0 {
        rw_memory_free(stream_free_list(), s);
    }
    result
}

/// `RwStreamOpen(type, access, data)` (0x7ECEF0): a stream from the freelist (hint 0x30404; NULL ->
/// NULL, no error), rwOwned = 1, the per-type setup of `_rwStreamInitialize`; on failure the stream is
/// returned to the freelist and NULL is returned.
#[hook(addr = 0x7ECEF0, conv = cdecl, name = "RwStreamOpen", expect = "A1 24 7B C9 00")]
pub fn rw_stream_open(ty: u32, access: u32, data: GPtr<u8>) -> GPtr<u8> {
    let s = rw_memory_alloc(stream_free_list(), 0x30404);
    if s.is_null() {
        return GPtr::NULL;
    }
    wr::<u32>(s, 0, ty);
    wr::<u32>(s, 4, access);
    wr::<u32>(s, 0x20, 1);
    if init_by_type(s, ty, access, data) {
        return s;
    }
    rw_memory_free(stream_free_list(), s);
    GPtr::NULL
}

/// `_rwStreamReadChunkHeader(stream, type, length, version, build)` (0x7ED0F0): reads 12 bytes (a
/// short read raises E_RW_READ, returns 0); library id `id & 0xFFFF0000 != 0` decodes to version
/// `(((id >> 14) & 0x3FF00) + 0x30000) | ((id >> 16) & 0x3F)`, build `id & 0xFFFF`, else version
/// `id << 8`, build 0. Calls `_rwChunkIsComplex` on its local header (result unused), then writes the
/// non-NULL outputs in the order type, length, build, version. Returns 1.
#[hook(addr = 0x7ED0F0, conv = cdecl, name = "_rwStreamReadChunkHeader", expect = "8B 4C 24 04 83 EC 28")]
pub fn read_chunk_header(
    s: GPtr<u8>,
    ty: GPtr<u32>,
    len: GPtr<u32>,
    ver: GPtr<u32>,
    build: GPtr<u32>,
) -> u32 {
    let mut raw = [0u32; 3];
    if rw_stream_read_call(s, GPtr::from_raw(raw.as_mut_ptr()).cast(), 0xC) != 0xC {
        error1(E_RW_READ);
        return 0;
    }
    let id = raw[2];
    // RwChunkHeaderInfo {type, length, version, buildNum, isComplex}
    let mut info = [0u32; 5];
    info[0] = raw[0];
    info[1] = raw[1];
    if id & 0xFFFF_0000 != 0 {
        info[2] = (((id >> 14) & 0x3FF00) + 0x30000) | ((id >> 16) & 0x3F);
        info[3] = id & 0xFFFF;
    } else {
        info[2] = id << 8;
        info[3] = 0;
    }
    chunk_is_complex_call(GPtr::from_raw(info.as_mut_ptr()).cast());
    if !ty.is_null() {
        ty.write(info[0]);
    }
    if !len.is_null() {
        len.write(info[1]);
    }
    if !build.is_null() {
        build.write(info[3]);
    }
    if !ver.is_null() {
        ver.write(info[2]);
    }
    1
}

/// `_rwChunkIsComplex(info)` (0x7ED1E0): 1 for chunk types 5..0xB, 0xE, 0xF, 0x10, 0x12, 0x14, 0x1A
/// (jump table 0x7ED1FC, type - 1 < 0x1A), else 0.
///
/// Not hookable: its caller `RwStreamReadChunkHeaderInfo` (0x7ED5DE) keeps `info` in ECX across the
/// call and stores through it afterwards (`preserve = "ecx"` would need an L1 spec comparing ECX).
pub fn chunk_is_complex(info: GPtr<u8>) -> u32 {
    let t = rd::<u32>(info, 0);
    matches!(t, 5..=0xB | 0xE | 0xF | 0x10 | 0x12 | 0x14 | 0x1A) as u32
}

/// `RwStreamFindChunk(stream, type, lengthOut, versionOut)` (0x7ED2D0): skips chunks until one of
/// `type`; only that one's version is checked against [0x34000, 0x36003] (E_RW_BADVERSION, 0). Writes
/// the non-NULL outputs, returns 1; a failed header read or skip returns 0.
#[hook(addr = 0x7ED2D0, conv = cdecl, name = "RwStreamFindChunk", expect = "83 EC 0C 8D 44 24 04")]
pub fn rw_stream_find_chunk(s: GPtr<u8>, ty: u32, len_out: GPtr<u32>, ver_out: GPtr<u32>) -> u32 {
    let (mut t, mut len, mut ver) = (0u32, 0u32, 0u32);
    let (pt, pl, pv) = (
        GPtr::from_raw(&mut t),
        GPtr::from_raw(&mut len),
        GPtr::from_raw(&mut ver),
    );
    if read_chunk_header_call(s, pt, pl, pv, GPtr::NULL) == 0 {
        return 0;
    }
    while pt.read() != ty {
        if rw_stream_skip_call(s, pl.read()).is_null() {
            return 0;
        }
        if read_chunk_header_call(s, pt, pl, pv, GPtr::NULL) == 0 {
            return 0;
        }
    }
    let v = pv.read();
    if !(0x34000..=0x36003).contains(&v) {
        error1(E_RW_BADVERSION);
        return 0;
    }
    if !len_out.is_null() {
        len_out.write(pl.read());
    }
    if !ver_out.is_null() {
        ver_out.write(v);
    }
    1
}

/// The common body of `RwStreamReadInt16/Real/Int32` (x86: no byte swapping): `RwStreamRead == 0`
/// raises E_RW_READ and returns NULL; any other count (a short read too) returns the stream.
fn read_checked(s: GPtr<u8>, p: GPtr<u8>, n: u32) -> GPtr<u8> {
    if rw_stream_read_call(s, p, n) == 0 {
        error1(E_RW_READ);
        return GPtr::NULL;
    }
    s
}

/// `RwStreamReadInt16(stream, p, numBytes)` (0x7ED4A0).
#[hook(addr = 0x7ED4A0, conv = cdecl, name = "RwStreamReadInt16", expect = "8B 44 24 0C 8B 4C 24 08")]
pub fn rw_stream_read_int16(s: GPtr<u8>, p: GPtr<u8>, n: u32) -> GPtr<u8> {
    read_checked(s, p, n)
}

/// `RwStreamReadReal(stream, p, numBytes)` (0x7ED4F0).
#[hook(addr = 0x7ED4F0, conv = cdecl, name = "RwStreamReadReal", expect = "8B 44 24 0C 8B 4C 24 08")]
pub fn rw_stream_read_real(s: GPtr<u8>, p: GPtr<u8>, n: u32) -> GPtr<u8> {
    read_checked(s, p, n)
}

/// `RwStreamReadInt32(stream, p, numBytes)` (0x7ED540).
#[hook(addr = 0x7ED540, conv = cdecl, name = "RwStreamReadInt32", expect = "8B 44 24 0C 8B 4C 24 08")]
pub fn rw_stream_read_int32(s: GPtr<u8>, p: GPtr<u8>, n: u32) -> GPtr<u8> {
    read_checked(s, p, n)
}

/// `RwStreamReadChunkHeaderInfo(stream, info)` (0x7ED590): `info = {type, length, version, build,
/// _rwChunkIsComplex(info)}`; a failed header read returns NULL (info untouched).
#[hook(addr = 0x7ED590, conv = cdecl, name = "RwStreamReadChunkHeaderInfo", expect = "83 EC 0C 8D 44 24 08")]
pub fn rw_stream_read_chunk_header_info(s: GPtr<u8>, info: GPtr<u8>) -> GPtr<u8> {
    let (mut t, mut len, mut ver, mut build) = (0u32, 0u32, 0u32, 0u32);
    let (pt, pl, pv, pb) = (
        GPtr::from_raw(&mut t),
        GPtr::from_raw(&mut len),
        GPtr::from_raw(&mut ver),
        GPtr::from_raw(&mut build),
    );
    if read_chunk_header_call(s, pt, pl, pv, pb) == 0 {
        return GPtr::NULL;
    }
    wr::<u32>(info, 0, pt.read());
    wr::<u32>(info, 4, pl.read());
    wr::<u32>(info, 8, pv.read());
    wr::<u32>(info, 0xC, pb.read());
    let c = chunk_is_complex_call(info);
    wr::<u32>(info, 0x10, c);
    s
}

/// `_rwStringStreamFindAndRead(string, stream)` (0x80A510): finds the next rwID_STRING (2) or
/// rwID_UNICODESTRING (0x13) chunk, checking EVERY header's version against [0x34000, 0x36003]
/// (E_RW_BADVERSION, NULL). A NULL `string` is allocated with `length` bytes (hint 0x30002; failure
/// E_RW_NOMEM(length), NULL). ASCII: copied through a local buffer in 0x40-byte pieces (a short read
/// returns NULL and leaks an allocated buffer). Unicode: 0x80-byte pieces, the low byte of each UTF-16
/// unit kept (`n >> 1` bytes per piece); a short read frees an allocated buffer and returns NULL. No
/// terminator is added.
#[hook(addr = 0x80A510, conv = cdecl, name = "_rwStringStreamFindAndRead", expect = "81 EC 9C 00 00 00")]
pub fn string_stream_find_and_read(string: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> {
    let (mut t, mut len, mut ver) = (0u32, 0u32, 0u32);
    let (pt, pl, pv) = (
        GPtr::from_raw(&mut t),
        GPtr::from_raw(&mut len),
        GPtr::from_raw(&mut ver),
    );
    let mut local = [0u8; 0x80];
    let lp: GPtr<u8> = GPtr::from_raw(local.as_mut_ptr());
    if read_chunk_header_call(s, pt, pl, pv, GPtr::NULL) == 0 {
        return GPtr::NULL;
    }
    loop {
        let v = pv.read();
        if !(0x34000..=0x36003).contains(&v) {
            error1(E_RW_BADVERSION);
            return GPtr::NULL;
        }
        match pt.read() {
            2 => {
                let mut remaining = pl.read();
                let mut buf = string;
                if buf.is_null() {
                    buf = rw_malloc(remaining, 0x30002);
                    if buf.is_null() {
                        error2(E_RW_NOMEM, remaining);
                        return GPtr::NULL;
                    }
                }
                let mut dst = buf;
                while remaining > 0 {
                    let n = remaining.min(0x40);
                    if rw_stream_read_call(s, lp, n) != n {
                        return GPtr::NULL;
                    }
                    remaining -= n;
                    for i in 0..n as isize {
                        dst.byte_add(i).write(lp.byte_add(i).read());
                    }
                    dst = dst.byte_add(n as isize);
                }
                return buf;
            }
            0x13 => {
                let mut remaining = pl.read();
                let mut buf = string;
                let mut allocated = false;
                if buf.is_null() {
                    buf = rw_malloc(remaining, 0x30002);
                    if buf.is_null() {
                        error2(E_RW_NOMEM, remaining);
                        return GPtr::NULL;
                    }
                    allocated = true;
                }
                let mut dst = buf;
                while remaining > 0 {
                    let n = remaining.min(0x80);
                    if rw_stream_read_call(s, lp, n) != n {
                        if allocated {
                            rw_free(buf);
                        }
                        return GPtr::NULL;
                    }
                    remaining -= n;
                    let half = n >> 1;
                    for i in 0..half as isize {
                        dst.byte_add(i).write(lp.byte_add(2 * i).read());
                    }
                    dst = dst.byte_add(half as isize);
                }
                return buf;
            }
            _ => {
                if rw_stream_skip_call(s, pl.read()).is_null() {
                    return GPtr::NULL;
                }
                if read_chunk_header_call(s, pt, pl, pv, GPtr::NULL) == 0 {
                    return GPtr::NULL;
                }
            }
        }
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_STREAM_MODULE_OPEN,
    HOOK_STREAM_MODULE_CLOSE,
    HOOK_RW_STREAM_INITIALIZE,
    HOOK_RW_STREAM_READ,
    HOOK_RW_STREAM_WRITE,
    HOOK_RW_STREAM_SKIP,
    HOOK_RW_STREAM_CLOSE,
    HOOK_RW_STREAM_OPEN,
    HOOK_READ_CHUNK_HEADER,
    HOOK_RW_STREAM_FIND_CHUNK,
    HOOK_RW_STREAM_READ_INT16,
    HOOK_RW_STREAM_READ_REAL,
    HOOK_RW_STREAM_READ_INT32,
    HOOK_RW_STREAM_READ_CHUNK_HEADER_INFO,
    HOOK_STRING_STREAM_FIND_AND_READ,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
