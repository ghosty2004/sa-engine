//! The streamed audio track readers (batch `mvpl-audio-2`): `CAEDataStream` (a Win32 file window over a track of the
//! stream archives, optionally XOR-encrypted), `CAEStreamTransformer::TransformBuffer`, `CAEStreamingDecoder`'s
//! destructor and `CAEVorbisDecoder` (the vorbisfile callbacks and wrappers). Ported from the 1.0 US bytes
//! (gta-reversed `AEDataStream.cpp` / `AEVorbisDecoder.cpp` as reading aids; the bytes win). Unverified
//! (docs/verify/HOWTO.md).
//!
//! `CAEDataStream` (0x28): +0 vtable, +4 file handle, +8 file name (heap), +0xC open, +0x10 current position, +0x14
//! start position, +0x18 length, +0x1C track id, +0x20 encrypted, +0x24 reference count. `CAEVorbisDecoder`: +4 the
//! data stream (`CAEStreamingDecoder`), +8 the four `ov_callbacks` (read, seek, close, tell), +0x18 the
//! `OggVorbis_File` (heap, 0x2D0 bytes), +0x1C initialised, +0x1D check the length, +0x20 the data source, +0x24 the
//! `vorbis_info`.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// KERNEL32 `CreateFileA` (IAT 0x8580A0)
    fn create_file_a(name: GPtr<u8>, access: u32, share: u32, sec: u32, disp: u32, flags: u32, tmpl: u32) -> u32 = stdcall cell 0x8580A0 => unported;
    /// KERNEL32 `SetFilePointer` (IAT 0x858090)
    fn set_file_pointer(h: u32, lo: u32, hi: u32, method: u32) -> u32 = stdcall cell 0x858090 => unported;
    /// KERNEL32 `ReadFile` (IAT 0x858098)
    fn read_file(h: u32, buf: GPtr<u8>, n: u32, read: GPtr<u32>, ov: u32) -> i32 = stdcall cell 0x858098 => unported;
    /// KERNEL32 `GetLastError` (IAT 0x858094)
    fn get_last_error() -> u32 = stdcall cell 0x858094 => unported;
    /// KERNEL32 `GetFileSize` (IAT 0x858074)
    fn get_file_size(h: u32, hi: u32) -> u32 = stdcall cell 0x858074 => unported;
    /// KERNEL32 `CloseHandle` (IAT 0x858078)
    fn close_handle(h: u32) -> i32 = stdcall cell 0x858078 => unported;
    /// CRT `free` (0x8214BD)
    fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// CRT `operator new` (0x82119A)
    fn crt_operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `CAEStreamingDecoder::CAEStreamingDecoder(CAEDataStream*)` (0x4F2810, `ret 4`)
    fn streaming_decoder_ctor(d: GPtr<u8>, stream: GPtr<u8>) -> GPtr<u8> = thiscall 0x4F2810 => streaming_decoder_constructor;
    /// `CAEVorbisDecoder::GetStreamLengthMs()` (vtable +8)
    fn v_get_stream_length_ms(d: GPtr<u8>) -> i32 = thiscall vslot 0x8 => unported;
    /// `CAEStreamTransformer::TransformBuffer(void*, uint32, uint32)` (0x4F17D0, `ret 0xC`)
    fn transform_buffer_call(t: GPtr<u8>, buf: GPtr<u8>, size: u32, pos: u32) = thiscall 0x4F17D0 => stream_transformer_transform_buffer;
    /// `CAEDataStream::FillBuffer(void*, uint32)` (0x4DC1C0, `ret 8`)
    fn data_stream_fill_buffer_call(s: GPtr<u8>, dest: GPtr<u8>, size: u32) -> u32 = thiscall 0x4DC1C0 => data_stream_fill_buffer;
    /// `CAEDataStream::GetCurrentPosition()` (0x4DC230)
    fn data_stream_get_current_position_call(s: GPtr<u8>) -> u32 = thiscall 0x4DC230 => data_stream_get_current_position;
    /// `CAEDataStream::Seek(int32, int32)` (0x4DC250, `ret 8`)
    fn data_stream_seek_call(s: GPtr<u8>, offset: u32, whence: u32) -> u32 = thiscall 0x4DC250 => data_stream_seek;
    /// `CAEDataStream::~CAEDataStream()` (0x4DC490)
    fn data_stream_destructor_call(s: GPtr<u8>) = thiscall 0x4DC490 => data_stream_destructor;
    /// `CAEStreamingDecoder::~CAEStreamingDecoder()` (0x4F2830)
    fn streaming_decoder_destructor_call(d: GPtr<u8>) = thiscall 0x4F2830 => streaming_decoder_destructor;
    /// `CAEVorbisDecoder::~CAEVorbisDecoder()` (0x5023F0)
    fn vorbis_destructor_call(d: GPtr<u8>) = thiscall 0x5023F0 => vorbis_destructor;
    /// KERNEL32 `lstrlenA` (IAT 0x8580DC, by its use here)
    fn lstrlen_a(s: GPtr<u8>) -> u32 = stdcall cell 0x8580DC => unported;
    /// KERNEL32 `lstrcpyA` (IAT 0x8580D4, by its use here)
    fn lstrcpy_a(d: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> = stdcall cell 0x8580D4 => unported;
    /// KERNEL32 `lstrcatA` (IAT 0x8580D8, by its use here)
    fn lstrcat_a(d: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> = stdcall cell 0x8580D8 => unported;
    /// `CAEDataStream::CAEDataStream(int32 track, char* name, uint32 start, uint32 length, bool encrypted)` (0x4DC620,
    /// `ret 0x14`)
    fn data_stream_ctor(s: GPtr<u8>, track: i32, name: GPtr<u8>, start: u32, length: u32, encrypted: u32) -> GPtr<u8> = thiscall 0x4DC620 => data_stream_constructor;
    /// KERNEL32 `InterlockedIncrement` (IAT 0x8580CC)
    fn interlocked_increment(p: GPtr<u32>) -> u32 = stdcall cell 0x8580CC => unported;
    /// KERNEL32 `InterlockedDecrement` (IAT 0x8580D0)
    fn interlocked_decrement(p: GPtr<u32>) -> u32 = stdcall cell 0x8580D0 => unported;
    /// `IUnknown::AddRef` through the object's vtable (slot +4)
    fn v_add_ref(this: GPtr<u8>) -> u32 = stdcall vslot 0x4 => unported;
    /// `CFileMgr::SetDir(const char*)` (0x5387D0)
    fn file_mgr_set_dir(dir: GPtr<u8>) -> i32 = cdecl 0x5387D0 => unported;
    /// `CAEDataStream::Initialise()` (0x4DC2B0), AL
    fn data_stream_initialise_call(s: GPtr<u8>) -> u8 = thiscall 0x4DC2B0 => data_stream_initialise;
    /// `ov_clear(OggVorbis_File*)` (VORBISFILE.DLL thunk 0x818062)
    fn ov_clear(vf: GPtr<u8>) -> i32 = cdecl 0x818062 => unported;
    /// `ov_read(vf, buffer, length, bigendian, word, signed, int* bitstream)` (thunk 0x818068)
    fn ov_read(vf: GPtr<u8>, buf: GPtr<u8>, len: u32, big: u32, word: u32, sgn: u32, bitstream: GPtr<i32>) -> i32 = cdecl 0x818068 => unported;
    /// `ov_info(vf, link)` (thunk 0x81806E)
    fn ov_info(vf: GPtr<u8>, link: i32) -> GPtr<u8> = cdecl 0x81806E => unported;
    /// `ov_open_callbacks(datasource, vf, initial, ibytes, ov_callbacks (16 bytes by value))` (thunk 0x818074)
    fn ov_open_callbacks(ds: GPtr<u8>, vf: GPtr<u8>, initial: u32, ibytes: u32, read: u32, seek: u32, close: u32, tell: u32) -> i32 = cdecl 0x818074 => unported;
    /// `ov_time_total(vf, link)` (thunk 0x81807A, double in ST0)
    fn ov_time_total(vf: GPtr<u8>, link: i32) -> f64 = cdecl 0x81807A => unported;
    /// `ov_time_tell(vf)` (thunk 0x818080, double in ST0)
    fn ov_time_tell(vf: GPtr<u8>) -> f64 = cdecl 0x818080 => unported;
    /// `ov_time_seek(vf, double seconds)` (thunk 0x818086)
    fn ov_time_seek(vf: GPtr<u8>, s: f64) -> i32 = cdecl 0x818086 => unported;
}

/// The stream transformer (0xB612D8) used for encrypted tracks
const STREAM_TRANSFORMER: usize = 0xB612D8;
/// 1000.0 (double, 0x85A318): seconds to milliseconds
const K_MS_PER_S: f64 = 1000.0;
/// 2^32 (0x858C54) and 0.001 (0x858CDC): `SetCursor`'s unsigned-to-seconds conversion
const K_TWO_32: usize = 0x858C54;
const K_S_PER_MS: usize = 0x858CDC;

fn k(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// `CAEDataStream::FillBuffer(void* dest, uint32 size)` (0x4DC1C0, thiscall, `ret 8`). `n = min(size, cur - length -
/// start)` (unsigned, as the original computes it); `ReadFile(handle, dest, n, &read, NULL)` with `read` in the
/// original's own `size` argument slot (so it keeps `size` if ReadFile does not write it); a failed ReadFile only calls
/// `GetLastError`. Encrypted → `TransformBuffer(dest, read, cur)` on the transformer 0xB612D8 (ECX set in the chunk
/// 0x40233E). `cur += read`; returns `read`.
#[hook(addr = 0x4dc1c0, conv = thiscall, name = "CAEDataStream::FillBuffer", expect = "56 8B F1 8B 4E 18",
       route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "reads the track file (ReadFile, file position): one run per OS state; writes the caller's buffer")]
pub fn data_stream_fill_buffer(this: GPtr<u8>, dest: GPtr<u8>, size: u32) -> u32 {
    let avail = rd::<u32>(this, 0x10).wrapping_sub(rd::<u32>(this, 0x18)).wrapping_sub(rd::<u32>(this, 0x14));
    let n = if size >= avail { avail } else { size };
    let mut read = size;
    if read_file(rd::<u32>(this, 4), dest, n, GPtr::from_raw(&mut read as *mut u32), 0) == 0 {
        get_last_error();
    }
    if rd::<u8>(this, 0x20) != 0 {
        transform_buffer_call(GPtr::from_addr(STREAM_TRANSFORMER), dest, read, rd::<u32>(this, 0x10));
    }
    wr::<u32>(this, 0x10, rd::<u32>(this, 0x10).wrapping_add(read));
    read
}

/// `CAEDataStream::GetCurrentPosition()` (0x4DC230, thiscall): `SetFilePointer(handle, 0, NULL, FILE_CURRENT) -
/// start`.
#[hook(addr = 0x4dc230, conv = thiscall, name = "CAEDataStream::GetCurrentPosition", expect = "56 6A 01 6A 00",
       route = "split", hazards = "none", recs = "grove-walk",
       note = "queries the OS file position")]
pub fn data_stream_get_current_position(this: GPtr<u8>) -> u32 {
    set_file_pointer(rd::<u32>(this, 4), 0, 0, 1).wrapping_sub(rd::<u32>(this, 0x14))
}

/// `CAEDataStream::Seek(int32 offset, int32 whence)` (0x4DC250, thiscall, `ret 8`). Whence 0: `start + offset`, method
/// 0; whence 2: `(length + start) - offset`, method 0; otherwise `offset` with method `whence`. `cur =
/// SetFilePointer(handle, that, NULL, method)`; returns `cur - start`.
#[hook(addr = 0x4dc250, conv = thiscall, name = "CAEDataStream::Seek-OG", expect = "56 8B F1 8B 4C 24 0C",
       route = "split", hazards = "none", recs = "grove-walk",
       note = "moves the OS file position")]
pub fn data_stream_seek(this: GPtr<u8>, offset: u32, whence: u32) -> u32 {
    let (pos, method) = match whence {
        0 => (offset.wrapping_add(rd::<u32>(this, 0x14)), 0),
        2 => (rd::<u32>(this, 0x18).wrapping_add(rd::<u32>(this, 0x14)).wrapping_sub(offset), 0),
        _ => (offset, whence),
    };
    let cur = set_file_pointer(rd::<u32>(this, 4), pos, 0, method);
    wr::<u32>(this, 0x10, cur);
    cur.wrapping_sub(rd::<u32>(this, 0x14))
}

/// `CAEDataStream::Initialise()` (0x4DC2B0, thiscall), AL. Open → 1. Else `handle = CreateFileA(name, GENERIC_READ,
/// FILE_SHARE_READ, NULL, OPEN_EXISTING, 0, NULL)`; valid → `cur = SetFilePointer(handle, start, NULL, FILE_BEGIN)`,
/// open, result 1. Then a zero length becomes `GetFileSize(handle, NULL)` (also after a failed open, with the invalid
/// handle). AL only: the upper EAX bytes are the caller's (open) or the last call's.
#[hook(addr = 0x4dc2b0, conv = thiscall, name = "CAEDataStream::Initialise", expect = "56 8B F1 8A 46 0C",
       route = "split", hazards = "ax-al-return", recs = "grove-walk",
       note = "opens the track file (CreateFileA): one run per OS state")]
pub fn data_stream_initialise(this: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0xC) != 0 {
        return 1;
    }
    let mut ok = 0;
    let h = create_file_a(rd::<GPtr<u8>>(this, 8), 0x8000_0000, 1, 0, 3, 0, 0);
    wr::<u32>(this, 4, h);
    if h != u32::MAX {
        let cur = set_file_pointer(h, rd::<u32>(this, 0x14), 0, 0);
        ok = 1;
        wr::<u32>(this, 0x10, cur);
        wr::<u8>(this, 0xC, 1);
    }
    if rd::<u32>(this, 0x18) == 0 {
        let n = get_file_size(rd::<u32>(this, 4), 0);
        wr::<u32>(this, 0x18, n);
    }
    ok
}

/// `CAEDataStream::~CAEDataStream()` (0x4DC490, thiscall): vtable 0x85F0BC; open → cleared, `CloseHandle(handle)`; a
/// file name → `free` and cleared.
#[hook(addr = 0x4dc490, conv = thiscall, name = "CAEDataStream::Destructor", expect = "56 8B F1 8A 46 0C",
       route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "closes the OS file handle and frees the name")]
pub fn data_stream_destructor(this: GPtr<u8>) {
    let open = rd::<u8>(this, 0xC);
    wr::<u32>(this, 0, 0x85F0BC);
    if open != 0 {
        let h = rd::<u32>(this, 4);
        wr::<u8>(this, 0xC, 0);
        close_handle(h);
    }
    let name = rd::<GPtr<u8>>(this, 8);
    if !name.is_null() {
        crt_free(name);
        wr::<u32>(this, 8, 0);
    }
}

/// `CAEStreamingDecoder::~CAEStreamingDecoder()` (0x4F2830, thiscall): vtable 0x85F418; a data stream (+4) →
/// `CAEDataStream::~CAEDataStream`, `free`, cleared.
#[hook(addr = 0x4f2830, conv = thiscall, name = "CAEStreamingDecoder::Destructor", expect = "56 8B F1 57 8B 7E 04",
       route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "closes the stream's OS file handle and frees it")]
pub fn streaming_decoder_destructor(this: GPtr<u8>) {
    let s = rd::<GPtr<u8>>(this, 4);
    wr::<u32>(this, 0, 0x85F418);
    if !s.is_null() {
        data_stream_destructor_call(s);
        crt_free(s);
        wr::<u32>(this, 4, 0);
    }
}

/// `CAEStreamTransformer::TransformBuffer(void* buf, uint32 size, uint32 pos)` (0x4F17D0, thiscall, `ret 0xC`):
/// `buf[i] ^= this[(pos + i) & 15]` for each byte.
#[hook(addr = 0x4f17d0, conv = thiscall, name = "CAEStreamTransformer::TransformBuffer", expect = "56 8B 74 24 0C",
       unverified = "mvpl-audio-2", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the caller's stream buffer")]
pub fn stream_transformer_transform_buffer(this: GPtr<u8>, buf: GPtr<u8>, size: u32, pos: u32) {
    for i in 0..size {
        let b = buf.byte_add(i as isize);
        let key = rd::<u8>(this, (pos.wrapping_add(i) & 0xF) as isize);
        b.write(b.read() ^ key);
    }
}

/// `CAEVorbisDecoder::ReadCallback(void* ptr, size_t size, size_t n, void* ds)` (0x502580, cdecl): no data source → 0;
/// else `FillBuffer(ds, ptr, size * n) / size` (unsigned; a zero `size` faults like the original's `div`).
#[hook(addr = 0x502580, conv = cdecl, name = "CAEVorbisDecoder::ReadCallback", expect = "8B 4C 24 10 85 C9",
       route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "reads the track file through CAEDataStream::FillBuffer")]
pub fn vorbis_read_callback(ptr: GPtr<u8>, size: u32, n: u32, ds: GPtr<u8>) -> u32 {
    if ds.is_null() {
        return 0;
    }
    data_stream_fill_buffer_call(ds, ptr, size.wrapping_mul(n)) / size
}

/// `CAEVorbisDecoder::SeekCallback(void* ds, int64 offset, int32 whence)` (0x5025B0, cdecl): no data source → -1; else
/// `Seek(ds, low dword of offset, whence)`.
#[hook(addr = 0x5025b0, conv = cdecl, name = "CAEVorbisDecoder::SeekCallback", expect = "8B 4C 24 04 85 C9",
       route = "split", hazards = "none", recs = "grove-walk",
       note = "moves the OS file position")]
pub fn vorbis_seek_callback(ds: GPtr<u8>, off_lo: u32, _off_hi: u32, whence: u32) -> u32 {
    if ds.is_null() {
        return u32::MAX;
    }
    data_stream_seek_call(ds, off_lo, whence)
}

/// `CAEVorbisDecoder::CloseCallback(void* ds)` (0x5025D0, cdecl): no data source → -1; else the tail jump into the
/// uncatalogued `CAEDataStream::Close` (0x4DC2A0): open cleared, `CloseHandle(handle)`'s result.
#[hook(addr = 0x5025d0, conv = cdecl, name = "CAEVorbisDecoder::CloseCallback", expect = "8B 4C 24 04 85 C9",
       route = "split", hazards = "none", recs = "grove-walk",
       note = "closes the OS file handle")]
pub fn vorbis_close_callback(ds: GPtr<u8>) -> i32 {
    if ds.is_null() {
        return -1;
    }
    let h = rd::<u32>(ds, 4);
    wr::<u8>(ds, 0xC, 0);
    close_handle(h)
}

/// `CAEVorbisDecoder::TellCallback(void* ds)` (0x5025F0, cdecl): no data source → -1; else the tail jump into
/// `CAEDataStream::GetCurrentPosition`.
#[hook(addr = 0x5025f0, conv = cdecl, name = "CAEVorbisDecoder::TellCallback", expect = "8B 4C 24 04 85 C9",
       route = "split", hazards = "none", recs = "grove-walk",
       note = "queries the OS file position")]
pub fn vorbis_tell_callback(ds: GPtr<u8>) -> u32 {
    if ds.is_null() {
        return u32::MAX;
    }
    data_stream_get_current_position_call(ds)
}

/// `CAEVorbisDecoder::GetSampleRate()` (0x502560, thiscall): initialised → `vorbis_info->rate` (+0x24 → +8), else -1.
#[hook(addr = 0x502560, conv = thiscall, name = "CAEVorbisDecoder::GetSampleRate", expect = "8A 41 1C 84 C0",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn vorbis_get_sample_rate(this: GPtr<u8>) -> i32 {
    if rd::<u8>(this, 0x1C) == 0 {
        return -1;
    }
    rd::<i32>(rd::<GPtr<u8>>(this, 0x24), 8)
}

/// `CAEVorbisDecoder::GetStreamLengthMs()` (0x502610, thiscall): initialised → `_ftol(ov_time_total(vf, -1) *
/// 1000.0)` (x87, the double constant 0x85A318 as a memory operand; the low dword), else -1.
#[hook(addr = 0x502610, conv = thiscall, name = "CAEVorbisDecoder::GetStreamLengthMs", expect = "8A 41 1C 84 C0",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn vorbis_get_stream_length_ms(this: GPtr<u8>) -> i32 {
    if rd::<u8>(this, 0x1C) == 0 {
        return -1;
    }
    let t = X87::from_f64_exact(ov_time_total(rd::<GPtr<u8>>(this, 0x18), -1));
    ftol32(t.fmul(X87::from_f64_exact(K_MS_PER_S)))
}

/// `CAEVorbisDecoder::GetStreamPlayTimeMs()` (0x502640, thiscall): initialised → `_ftol(ov_time_tell(vf) * 1000.0)`,
/// else -1.
#[hook(addr = 0x502640, conv = thiscall, name = "CAEVorbisDecoder::GetStreamPlayTimeMs", expect = "8A 41 1C 84 C0",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn vorbis_get_stream_play_time_ms(this: GPtr<u8>) -> i32 {
    if rd::<u8>(this, 0x1C) == 0 {
        return -1;
    }
    let t = X87::from_f64_exact(ov_time_tell(rd::<GPtr<u8>>(this, 0x18)));
    ftol32(t.fmul(X87::from_f64_exact(K_MS_PER_S)))
}

/// `CAEVorbisDecoder::SetCursor(uint32 ms)` (0x502670, thiscall, `ret 4`): initialised → `ov_time_seek(vf, (ms as
/// unsigned, `fild` + 2^32 when negative) * 0.001)` (x87, passed as a double).
#[hook(addr = 0x502670, conv = thiscall, name = "CAEVorbisDecoder::SetCursor", expect = "8A 41 1C 84 C0",
       route = "split", hazards = "none", recs = "grove-walk",
       note = "seeks the track file through vorbisfile")]
pub fn vorbis_set_cursor(this: GPtr<u8>, ms: i32) {
    if rd::<u8>(this, 0x1C) == 0 {
        return;
    }
    let mut v = X87::from_i32(ms);
    if ms < 0 {
        v = v.fadd_m(k(K_TWO_32));
    }
    let s = v.fmul_m(k(K_S_PER_MS));
    ov_time_seek(rd::<GPtr<u8>>(this, 0x18), s.to_f64());
}

/// `CAEVorbisDecoder::~CAEVorbisDecoder()` (0x5023F0, thiscall): vtable 0x862D70; an `OggVorbis_File` (+0x18) →
/// `ov_clear`, `free`; then the `CAEStreamingDecoder` part (0x4F2830). The SEH frame of the original is not needed.
#[hook(addr = 0x5023f0, conv = thiscall, name = "CAEVorbisDecoder::Destructor", expect = "6A FF 68 E8 C4 83 00",
       route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "frees the vorbisfile state and closes the stream's file handle")]
pub fn vorbis_destructor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x862D70);
    let vf = rd::<GPtr<u8>>(this, 0x18);
    if !vf.is_null() {
        ov_clear(vf);
        crt_free(rd::<GPtr<u8>>(this, 0x18));
    }
    streaming_decoder_destructor_call(this);
}

/// `CAEVorbisDecoder` scalar deleting destructor (0x502700, thiscall, `ret 4`): the destructor, `free(this)` when
/// `flags & 1`; returns `this`.
#[hook(addr = 0x502700, conv = thiscall, name = "CAEVorbisDecoder::~CAEVorbisDecoder", expect = "56 8B F1 E8 E8 FC FF FF",
       route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "frees the decoder")]
pub fn vorbis_deleting_destructor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    vorbis_destructor_call(this);
    if flags as u8 & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `CAEVorbisDecoder::FillBuffer(void* dest, uint32 size)` (0x502470, thiscall, `ret 8`): `ov_read(vf, dest + n, size
/// - n, little endian, 16-bit, signed, &bitstream)` (bitstream starts at -1) while it returns above 0 and `n < size`
/// (unsigned); returns `n`.
#[hook(addr = 0x502470, conv = thiscall, name = "CAEVorbisDecoder::FillBuffer", expect = "51 53 55 56 57",
       route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "decodes through vorbisfile, which reads the track file")]
pub fn vorbis_fill_buffer(this: GPtr<u8>, dest: GPtr<u8>, size: u32) -> u32 {
    let mut bitstream: i32 = -1;
    let mut n: u32 = 0;
    while n < size {
        let r = ov_read(
            rd::<GPtr<u8>>(this, 0x18),
            dest.byte_add(n as isize),
            size - n,
            0,
            2,
            1,
            GPtr::from_raw(&mut bitstream as *mut i32),
        );
        if r <= 0 {
            break;
        }
        n = n.wrapping_add(r as u32);
    }
    n
}

/// `CAEVorbisDecoder::Initialise()` (0x5024D0, thiscall), AL. `vf = operator new(0x2D0)` (+0x18);
/// `ov_open_callbacks(data source +0x20, vf, NULL, 0, the callbacks +8..+0x17)` non-zero → 0. Else `vorbis_info =
/// ov_info(vf, -1)` (+0x24), initialised; with the length check (+0x1D) a stream shorter than 7000 ms (virtual
/// `GetStreamLengthMs`) clears initialised again and gives 0; else 1. AL only: the upper EAX bytes are the last
/// call's.
#[hook(addr = 0x5024d0, conv = thiscall, name = "CAEVorbisDecoder::Initialise", expect = "53 56 57 68 D0 02 00 00",
       route = "split", hazards = "ax-al-return,heap-writes", recs = "grove-walk",
       note = "allocates the vorbisfile state and reads the stream header (file position)")]
pub fn vorbis_initialise(this: GPtr<u8>) -> u8 {
    let vf = crt_operator_new(0x2D0);
    let cb = [rd::<u32>(this, 8), rd::<u32>(this, 0xC), rd::<u32>(this, 0x10), rd::<u32>(this, 0x14)];
    wr::<GPtr<u8>>(this, 0x18, vf);
    if ov_open_callbacks(rd::<GPtr<u8>>(this, 0x20), vf, 0, 0, cb[0], cb[1], cb[2], cb[3]) != 0 {
        return 0;
    }
    let info = ov_info(rd::<GPtr<u8>>(this, 0x18), -1);
    wr::<GPtr<u8>>(this, 0x24, info);
    wr::<u8>(this, 0x1C, 1);
    if rd::<u8>(this, 0x1D) != 0 && v_get_stream_length_ms(this) < 7000 {
        wr::<u8>(this, 0x1C, 0);
        return 0;
    }
    1
}

/// `CAEVorbisDecoder::CAEVorbisDecoder(CAEDataStream*, bool checkLength)` (0x5026B0, thiscall, `ret 8`): the
/// `CAEStreamingDecoder` part, data source (+0x20) = the stream, +0x1C, +0x24 and +0x18 cleared, vtable 0x862D70 (in
/// the chunk 0x40A1D0), +0x1D = the flag, callbacks read 0x502580 (+8), close 0x5025D0 (+0x10), seek 0x5025B0
/// (+0xC), tell 0x5025F0 (+0x14); returns `this`.
#[hook(addr = 0x5026b0, conv = thiscall, name = "CAEVorbisDecoder::Constructor", expect = "56 57 8B 7C 24 0C",
       unverified = "mvpl-audio-2", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "constructs a heap decoder")]
pub fn vorbis_constructor(this: GPtr<u8>, stream: GPtr<u8>, check: u32) -> GPtr<u8> {
    streaming_decoder_ctor(this, stream);
    wr::<GPtr<u8>>(this, 0x20, stream);
    wr::<u8>(this, 0x1C, 0);
    wr::<u32>(this, 0x24, 0);
    wr::<u32>(this, 0x18, 0);
    wr::<u32>(this, 0, 0x862D70);
    wr::<u8>(this, 0x1D, check as u8);
    wr::<u32>(this, 8, 0x502580);
    wr::<u32>(this, 0x10, 0x5025D0);
    wr::<u32>(this, 0xC, 0x5025B0);
    wr::<u32>(this, 0x14, 0x5025F0);
    this
}

/// `CAETrackLoader::GetDataStream(uint32 track)` (0x4E0D20, thiscall, `ret 4`). Layout: +4 track count, +8 the last
/// valid stream pack index, +0xC the track table (12 bytes: pack byte +0, offset +4, length +8), +0x10 the pack names
/// (16 bytes each), +0x14 has a root path, +0x18 the root path. `track >= count` (unsigned) or its pack above +8 → NULL.
/// The name buffer (`operator new`) holds `lstrlenA(pack name) [+ lstrlenA(root)] + 0x15` bytes: `[root +]
/// "AUDIO\\STREAMS\\"` (0x85F1BC) + the pack name (`lstrcpyA` / `lstrcatA`). A `CAEDataStream(track, name, offset +
/// 0x1F84, length, true)` (`operator new(0x28)`, NULL when that fails); `CFileMgr::SetDir("")`; `Initialise` on it (on
/// NULL too, as the original); a failed `Initialise` destroys and frees it and gives NULL. The SEH frame of the
/// original is not needed.
#[hook(addr = 0x4e0d20, conv = thiscall, name = "CAETrackLoader::GetDataStream", expect = "6A FF 68 9B BF 83 00",
       route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "allocates the stream and its name and opens the stream archive (CreateFileA, file position)")]
pub fn track_loader_get_data_stream(this: GPtr<u8>, track: u32) -> GPtr<u8> {
    if track >= rd::<u32>(this, 4) {
        return GPtr::NULL;
    }
    let row = (track.wrapping_mul(12)) as isize;
    let pack = u32::from(rd::<GPtr<u8>>(this, 0xC).byte_add(row).read());
    if pack > rd::<u32>(this, 8) {
        return GPtr::NULL;
    }
    let pack_name = |p: u32| rd::<GPtr<u8>>(this, 0x10).byte_add((p << 4) as isize);
    let has_root = rd::<u8>(this, 0x14) != 0;
    let size = if has_root {
        let a = lstrlen_a(pack_name(pack));
        let b = lstrlen_a(rd::<GPtr<u8>>(this, 0x18));
        b.wrapping_add(a).wrapping_add(0x15)
    } else {
        lstrlen_a(pack_name(pack)).wrapping_add(0x15)
    };
    let name = crt_operator_new(size);
    let streams_dir: GPtr<u8> = GPtr::from_addr(0x85F1BC);
    if rd::<u8>(this, 0x14) != 0 {
        lstrcpy_a(name, rd::<GPtr<u8>>(this, 0x18));
        lstrcat_a(name, streams_dir);
    } else {
        lstrcpy_a(name, streams_dir);
    }
    let pack = u32::from(rd::<GPtr<u8>>(this, 0xC).byte_add(row).read());
    lstrcat_a(name, pack_name(pack));
    let mem = crt_operator_new(0x28);
    let stream = if mem.is_null() {
        GPtr::NULL
    } else {
        let t = rd::<GPtr<u8>>(this, 0xC).byte_add(row);
        let length = rd::<u32>(t, 8);
        let offset = rd::<u32>(t, 4).wrapping_add(0x1F84);
        data_stream_ctor(mem, track as i32, name, offset, length, 1)
    };
    file_mgr_set_dir(GPtr::from_addr(0x858B54));
    if data_stream_initialise_call(stream) != 0 {
        return stream;
    }
    if !stream.is_null() {
        data_stream_destructor_call(stream);
        crt_free(stream);
    }
    GPtr::NULL
}

/// `CAEDataStream::CAEDataStream(int32 track, char* name, uint32 start, uint32 length, bool encrypted)` (0x4DC620,
/// thiscall, `ret 0x14`): name +8, open +0xC cleared, current position +0x10 cleared, start +0x14, length +0x18,
/// vtable 0x85F0BC, track +0x1C, encrypted +0x20 (the byte), reference count +0x24 = 1; returns `this`. The file
/// handle (+4) is left as it was.
#[hook(addr = 0x4dc620, conv = thiscall, name = "CAEDataStream::Constructor", expect = "8B 54 24 0C 8B C1",
       unverified = "stream-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "constructs a heap stream")]
pub fn data_stream_constructor(this: GPtr<u8>, track: i32, name: GPtr<u8>, start: u32, length: u32, encrypted: u32) -> GPtr<u8> {
    wr::<GPtr<u8>>(this, 8, name);
    wr::<u8>(this, 0xC, 0);
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0x14, start);
    wr::<u32>(this, 0x18, length);
    wr::<u32>(this, 0, 0x85F0BC);
    wr::<i32>(this, 0x1C, track);
    wr::<u8>(this, 0x20, encrypted as u8);
    wr::<u32>(this, 0x24, 1);
    this
}

/// `CAEDataStream::Read(void* pv, ULONG cb, ULONG* read)` (0x4DC320, IStream, stdcall `ret 0x10`): `*read =
/// FillBuffer(pv, cb)` (no NULL check, as the original); `S_OK`.
#[hook(addr = 0x4dc320, conv = stdcall, name = "CAEDataStream::Read", expect = "8B 44 24 0C 8B 4C 24 08",
       unverified = "stream-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "reads the track file through FillBuffer")]
pub fn data_stream_read(this: GPtr<u8>, pv: GPtr<u8>, cb: u32, read: GPtr<u32>) -> u32 {
    let n = data_stream_fill_buffer_call(this, pv, cb);
    read.write(n);
    0
}

/// `CAEDataStream::Seek(LARGE_INTEGER move, DWORD origin, ULARGE_INTEGER* pos)` (0x4DC340, IStream, stdcall `ret
/// 0x14`). Origin 0: `start + move.lo`, method 0; origin 2: `(length + start) - move.lo`, method 0; otherwise
/// `move.lo` with method `origin` (the high dword is ignored). `cur = SetFilePointer(handle, that, NULL, method)`; a
/// `pos` gets `(cur - start, 0)`; `S_OK`.
#[hook(addr = 0x4dc340, conv = stdcall, name = "CAEDataStream::Seek-istream", expect = "8B 4C 24 10 56",
       unverified = "stream-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "moves the OS file position")]
pub fn data_stream_seek_istream(this: GPtr<u8>, move_lo: u32, _move_hi: u32, origin: u32, pos: GPtr<u32>) -> u32 {
    let (to, method) = match origin {
        0 => (rd::<u32>(this, 0x14).wrapping_add(move_lo), 0),
        2 => (rd::<u32>(this, 0x18).wrapping_add(rd::<u32>(this, 0x14)).wrapping_sub(move_lo), 0),
        _ => (move_lo, origin),
    };
    let cur = set_file_pointer(rd::<u32>(this, 4), to, 0, method);
    wr::<u32>(this, 0x10, cur);
    let rel = cur.wrapping_sub(rd::<u32>(this, 0x14));
    if !pos.is_null() {
        pos.write(0);
        pos.wrapping_index(1).write(0);
        pos.write(rel);
    }
    0
}

/// `CAEDataStream::Stat(STATSTG* stat, DWORD flags)` (0x4DC3A0, IStream, stdcall `ret 0xC`). No `stat` or `flags !=
/// STATFLAG_NONAME` → `E_INVALIDARG`. `size = GetFileSize(handle, NULL)`; `INVALID_FILE_SIZE` → `GetLastError()` is
/// read twice: a value ≤ 0 → the second read itself, else `HRESULT_FROM_WIN32` of a third read. Else the 0x48 bytes
/// zeroed, type `STGTY_STREAM` (+4), `cbSize.lo` (+8) = the file size; `S_OK`.
#[hook(addr = 0x4dc3a0, conv = stdcall, name = "CAEDataStream::Stat", expect = "56 8B 74 24 0C 85 F6",
       unverified = "stream-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "queries the OS file size")]
pub fn data_stream_stat(this: GPtr<u8>, stat: GPtr<u32>, flags: u32) -> u32 {
    if stat.is_null() || flags != 1 {
        return 0x8007_0057;
    }
    let size = get_file_size(rd::<u32>(this, 4), 0);
    if size == u32::MAX {
        if (get_last_error() as i32) <= 0 {
            return get_last_error();
        }
        return (get_last_error() & 0xFFFF) | 0x8007_0000;
    }
    for i in 0..0x12 {
        stat.wrapping_index(i).write(0);
    }
    stat.wrapping_index(1).write(2);
    stat.wrapping_index(2).write(size);
    0
}

/// `CAEDataStream::QueryInterface(REFIID riid, void** out)` (0x4DC410, stdcall `ret 0xC`): `riid` equal (16 bytes) to
/// the IID at 0x887BBC or the one at 0x887A4C → `*out = this`, `AddRef` through the vtable, `S_OK`; else
/// `E_NOINTERFACE` (`*out` untouched).
#[hook(addr = 0x4dc410, conv = stdcall, name = "CAEDataStream::QueryInterface", expect = "8B 44 24 08 56 57",
       unverified = "stream-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn data_stream_query_interface(this: GPtr<u8>, riid: GPtr<u32>, out: GPtr<u32>) -> u32 {
    let eq = |iid: usize| (0..4).all(|i| riid.wrapping_index(i).read() == g::<u32>(iid + 4 * i as usize).read());
    if !eq(0x887BBC) && !eq(0x887A4C) {
        return 0x8000_4002;
    }
    out.write(this.addr() as u32);
    v_add_ref(this);
    0
}

/// `CAEDataStream::AddRef()` (0x4DC460, stdcall): the tail jump into `InterlockedIncrement(&refs)` (+0x24).
#[hook(addr = 0x4dc460, conv = stdcall, name = "CAEDataStream::AddRef", expect = "8B 44 24 04 83 C0 24",
       unverified = "stream-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn data_stream_add_ref(this: GPtr<u8>) -> u32 {
    interlocked_increment(this.byte_add(0x24).cast())
}

/// `CAEDataStream::Release()` (0x4DC5B0, stdcall `ret 4`): `InterlockedDecrement(&refs)` non-zero → 0xBAD; else (a
/// non-NULL `this`) the destructor inlined (vtable 0x85F0BC, an open handle closed, the name freed) and `free(this)`;
/// 0.
#[hook(addr = 0x4dc5b0, conv = stdcall, name = "CAEDataStream::Release", expect = "56 8B 74 24 08 8D 46 24",
       unverified = "stream-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "closes the OS file handle and frees the stream")]
pub fn data_stream_release(this: GPtr<u8>) -> u32 {
    if interlocked_decrement(this.byte_add(0x24).cast()) != 0 {
        return 0xBAD;
    }
    if !this.is_null() {
        data_stream_destructor_call(this);
        crt_free(this);
    }
    0
}

/// `CAEVorbisDecoder::GetStreamID()` (0x502460, thiscall): the data source's (+0x20) track id (+0x1C).
#[hook(addr = 0x502460, conv = thiscall, name = "CAEVorbisDecoder::GetStreamID", expect = "8B 41 20 8B 40 1C",
       unverified = "stream-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn vorbis_get_stream_id(this: GPtr<u8>) -> i32 {
    rd::<i32>(rd::<GPtr<u8>>(this, 0x20), 0x1C)
}

/// `CAEStreamingDecoder::CAEStreamingDecoder(CAEDataStream* stream)` (0x4F2810, thiscall, `ret 4`): vtable 0x85F418,
/// the stream (+4); a stream → `CAEDataStream::Initialise` (result ignored); returns `this`.
#[hook(addr = 0x4f2810, conv = thiscall, name = "CAEStreamingDecoder::Constructor", expect = "56 8B F1 8B 4C 24 08",
       unverified = "stream-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "opens the track file (CreateFileA)")]
pub fn streaming_decoder_constructor(this: GPtr<u8>, stream: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0x85F418);
    wr::<GPtr<u8>>(this, 4, stream);
    if !stream.is_null() {
        data_stream_initialise_call(stream);
    }
    this
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_DATA_STREAM_FILL_BUFFER,
    HOOK_DATA_STREAM_GET_CURRENT_POSITION,
    HOOK_DATA_STREAM_SEEK,
    HOOK_DATA_STREAM_INITIALISE,
    HOOK_DATA_STREAM_DESTRUCTOR,
    HOOK_STREAMING_DECODER_DESTRUCTOR,
    HOOK_STREAM_TRANSFORMER_TRANSFORM_BUFFER,
    HOOK_VORBIS_READ_CALLBACK,
    HOOK_VORBIS_SEEK_CALLBACK,
    HOOK_VORBIS_CLOSE_CALLBACK,
    HOOK_VORBIS_TELL_CALLBACK,
    HOOK_VORBIS_GET_SAMPLE_RATE,
    HOOK_VORBIS_GET_STREAM_LENGTH_MS,
    HOOK_VORBIS_GET_STREAM_PLAY_TIME_MS,
    HOOK_VORBIS_SET_CURSOR,
    HOOK_VORBIS_DESTRUCTOR,
    HOOK_VORBIS_DELETING_DESTRUCTOR,
    HOOK_VORBIS_FILL_BUFFER,
    HOOK_VORBIS_INITIALISE,
    HOOK_VORBIS_CONSTRUCTOR,
    HOOK_TRACK_LOADER_GET_DATA_STREAM,
    HOOK_DATA_STREAM_CONSTRUCTOR,
    HOOK_DATA_STREAM_READ,
    HOOK_DATA_STREAM_SEEK_ISTREAM,
    HOOK_DATA_STREAM_STAT,
    HOOK_DATA_STREAM_QUERY_INTERFACE,
    HOOK_DATA_STREAM_ADD_REF,
    HOOK_DATA_STREAM_RELEASE,
    HOOK_STREAMING_DECODER_CONSTRUCTOR,
    HOOK_VORBIS_GET_STREAM_ID,
];
