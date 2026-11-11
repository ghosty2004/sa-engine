//! CdStream (0x406370–0x406C8B, bodies partly in `.HOODLUM`): the IMG archive reader with its
//! request queue and streaming thread.
//!
//! Ported from the 1.0 US bytes; gta-reversed `CdStreamInfo.cpp` as reading aid
//! (`docs/re/specs/STREAM/CdStream.deviations.md`). Win32 through the IAT cells (`orig!` `cell`).
//! Several bodies call their imports through the no-CD protection's import thunk 0x1141980, which
//! loads `[[cellptr] ^ 0x214D4C48]` (decoded: CreateFileA 0x8580A0, ReleaseSemaphore 0x85808C,
//! ReadFile 0x858098, SetFilePointer 0x858090, GetOverlappedResult 0x858084); the port calls the
//! same IAT cells directly. Original behaviour kept:
//! - `CdStreamRead` stores `gLastCdStreamPosn = sectors + pos` with the file id byte included
//!   (gr-deviations `CdStreamInfo.cpp:164`, keep-original), overwrites its `pos` argument slot with the
//!   file handle and its `streamId` slot with the byte count of the synchronous ReadFile; the
//!   non-threaded paths return ReadFile's BOOL unchanged (synchronous) or 1/0 (overlapped,
//!   `ERROR_IO_PENDING` = success);
//! - `CdStreamInit`: `FILE_FLAG_NO_BUFFERING` when `bytesPerSector <= 2048` (gr-deviations `:286`,
//!   keep-original); `bytesPerSector` is an uninitialised local when GetDiskFreeSpaceA fails (the port
//!   passes 0 then; L2 may show it);
//! - `CdStreamOpen`: `strcpy` of the name into the 64-byte slot (gr: `strncpy_s`, *new*);
//! - `CdStreamThread`: `GetFirstInQueue` of an empty queue gives stream -1 (no check); a ReadFile
//!   result other than 1 takes the failure path in the overlapped branch (*new*: gr tests nonzero);
//!   the result of GetOverlappedResult must be exactly 1 for success (also in `CdStreamSync`).
//!
//! Deliberate divergence (ROADMAP divergence ledger, INF-15): `CdStreamSync` does not reproduce the
//! original's lost wake-up. The original waits `INFINITE` on the stream's semaphore after setting
//! `bLocked`; when the streaming thread finished in between (it releases only if it saw `bLocked`),
//! the game hangs forever. The port waits in 20 ms slices and ends the wait once the thread is done
//! with the stream (`nSectorsToRead == 0 && bInUse == 0`), consuming a release that raced with the
//! check — exactly sa-hook's INF-15 fix (`cdsync.rs`). Everything observable of a completed read is
//! unchanged.
//!
//! Not reproduced (stdout only): `CdStreamRead`'s `printf` when `ReleaseSemaphore` fails — its format
//! string is computed by the protection VM (0xCB73E0) and was not decoded.
//!
//! Threads: `CdStreamThread` runs on the streaming thread and uses `Global`/`GPtr` there, as the
//! original does (an exception to ADR-001's single-thread rule, recorded in docs/arch/stream-port.md).
//! Its hook takes effect when the thread starts (the function never returns).

use sa_core::mem::{GPtr, GameData};
use sa_core::{hook, orig};

use crate::shared::image;

/// `CdStream` (0x30).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CdStream {
    /// +0x00 `nSectorOffset`
    pub sector_offset: u32,
    /// +0x04 `nSectorsToRead`
    pub sectors_to_read: u32,
    /// +0x08 `lpBuffer`
    pub buffer: GPtr<u8>,
    pub field_c: u8,
    /// +0x0D `bLocked`
    pub locked: u8,
    /// +0x0E `bInUse`
    pub in_use: u8,
    pub field_f: u8,
    /// +0x10 `status`
    pub status: u32,
    /// +0x14 sync semaphore
    pub semaphore: GPtr<u8>,
    /// +0x18 `hFile`
    pub file: GPtr<u8>,
    /// +0x1C `OVERLAPPED` (Internal, InternalHigh, Offset +0x24, OffsetHigh, hEvent)
    pub overlapped: [u32; 5],
}
// SAFETY: integers and game pointers.
#[allow(unsafe_code)]
unsafe impl GameData for CdStream {}
#[cfg(target_pointer_width = "32")]
const _: () = assert!(core::mem::size_of::<CdStream>() == sa_core::layouts::size::CdStream);

pub const STATUS_SUCCESS: u32 = 0;
pub const STATUS_WAITING: u32 = 0xFA;
pub const STATUS_FAILURE: u32 = 0xFE;
pub const STATUS_READING: u32 = 0xFF;
const ERROR_IO_PENDING: u32 = 0x3E5;
const INFINITE: u32 = 0xFFFF_FFFF;
/// Poll slice of the lost-wake-up-free wait (as sa-hook's INF-15 fix).
const POLL_MS: u32 = 20;
const WAIT_OBJECT_0: u32 = 0;
const WAIT_TIMEOUT: u32 = 0x102;

sa_core::globals! {
    list GLOBALS;
    /// `gStreamFileCreateFlags`
    pub static STREAM_FILE_CREATE_FLAGS @ 0x8E3FE0: u32 = 0;
    /// `gStreamingInitialized` (the thread is running)
    pub static STREAMING_INITIALIZED @ 0x8E3FE4: i32 = 0;
    /// `gOverlappedIO`
    pub static OVERLAPPED_IO @ 0x8E3FE8: i32 = 0;
    /// `gStreamQueue.data`
    pub static QUEUE_DATA @ 0x8E3FEC: GPtr<i32> = GPtr::NULL;
    /// `gStreamQueue.head`
    pub static QUEUE_HEAD @ 0x8E3FF0: i32 = 0;
    /// `gStreamQueue.tail`
    pub static QUEUE_TAIL @ 0x8E3FF4: i32 = 0;
    /// `gStreamQueue.size`
    pub static QUEUE_SIZE @ 0x8E3FF8: i32 = 0;
    /// `gCdStreams`
    pub static CD_STREAMS @ 0x8E3FFC: GPtr<CdStream> = GPtr::NULL;
    /// `gStreamingThreadId`
    pub static STREAMING_THREAD_ID @ 0x8E4000: u32 = 0;
    /// `gStreamSemaphore`
    pub static STREAM_SEMAPHORE @ 0x8E4004: GPtr<u8> = GPtr::NULL;
    /// `gStreamingThread`
    pub static STREAMING_THREAD @ 0x8E4008: GPtr<u8> = GPtr::NULL;
    /// `gStreamFileHandles[32]`
    pub static STREAM_FILE_HANDLES @ 0x8E4010: [GPtr<u8>; 32] = [GPtr::NULL; 32];
    /// `gStreamCount`
    pub static STREAM_COUNT @ 0x8E4090: i32 = 0;
    /// `gOpenStreamCount`
    pub static OPEN_STREAM_COUNT @ 0x8E4094: i32 = 0;
    /// `gCdImageNames[32][64]`
    pub static CD_IMAGE_NAMES @ 0x8E4098: [[u8; 64]; 32] = [[0; 64]; 32];
    /// `gLastCdStreamPosn`
    pub static LAST_CD_STREAM_POSN @ 0x8E4898: u32 = 0;
}

/// "MODELS\\GTA3.IMG" (0x858AF4)
pub const NAME_GTA3_IMG: u32 = 0x858AF4;
/// "CdStream" (0x858AD4), the stream semaphore's name
pub const NAME_CDSTREAM: u32 = 0x858AD4;
/// printf arguments of CdStreamInitThread's failures: "%s" message strings and the prefix 0x858AC8
pub const MSG_SYNC_SEMA: u32 = 0x858A78;
pub const MSG_STREAM_SEMA: u32 = 0x858AA0;
pub const MSG_THREAD: u32 = 0x858A50;
pub const MSG_PREFIX: u32 = 0x858AC8;
/// `CdStreamThread` (thread entry passed to CreateThread)
pub const THREAD_ENTRY: u32 = 0x406560;

orig! {
    /// KERNEL32 `CloseHandle` (IAT 0x858078)
    pub fn close_handle(h: GPtr<u8>) -> i32 = stdcall cell 0x858078 => unported;
    /// KERNEL32 `LocalFree` (IAT 0x85807C)
    pub fn local_free(p: GPtr<u8>) -> GPtr<u8> = stdcall cell 0x85807C => unported;
    /// KERNEL32 `WaitForSingleObjectEx` (IAT 0x858080)
    pub fn wait_for_single_object_ex(h: GPtr<u8>, ms: u32, alertable: i32) -> u32 = stdcall cell 0x858080 => unported;
    /// KERNEL32 `GetOverlappedResult` (IAT 0x858084)
    pub fn get_overlapped_result(h: GPtr<u8>, ov: GPtr<u32>, n: GPtr<u32>, wait: i32) -> i32 = stdcall cell 0x858084 => unported;
    /// KERNEL32 `WaitForSingleObject` (IAT 0x858088)
    pub fn wait_for_single_object(h: GPtr<u8>, ms: u32) -> u32 = stdcall cell 0x858088 => unported;
    /// KERNEL32 `ReleaseSemaphore` (IAT 0x85808C)
    pub fn release_semaphore(h: GPtr<u8>, n: i32, prev: GPtr<i32>) -> i32 = stdcall cell 0x85808C => unported;
    /// KERNEL32 `SetFilePointer` (IAT 0x858090)
    pub fn set_file_pointer(h: GPtr<u8>, lo: u32, hi: GPtr<u32>, method: u32) -> u32 = stdcall cell 0x858090 => unported;
    /// KERNEL32 `GetLastError` (IAT 0x858094)
    pub fn get_last_error() -> u32 = stdcall cell 0x858094 => unported;
    /// KERNEL32 `ReadFile` (IAT 0x858098)
    pub fn read_file(h: GPtr<u8>, buf: GPtr<u8>, n: u32, read: GPtr<u32>, ov: GPtr<u32>) -> i32 = stdcall cell 0x858098 => unported;
    /// KERNEL32 `SetLastError` (IAT 0x85809C)
    pub fn set_last_error(e: u32) = stdcall cell 0x85809C => unported;
    /// KERNEL32 `CreateFileA` (IAT 0x8580A0)
    pub fn create_file_a(name: GPtr<u8>, access: u32, share: u32, sec: u32, disp: u32, flags: u32, tmpl: u32) -> GPtr<u8> = stdcall cell 0x8580A0 => unported;
    /// KERNEL32 `ResumeThread` (IAT 0x8580A4)
    pub fn resume_thread(h: GPtr<u8>) -> u32 = stdcall cell 0x8580A4 => unported;
    /// KERNEL32 `SetThreadPriority` (IAT 0x8580A8)
    pub fn set_thread_priority(h: GPtr<u8>, p: i32) -> i32 = stdcall cell 0x8580A8 => unported;
    /// KERNEL32 `GetThreadPriority` (IAT 0x8580AC)
    pub fn get_thread_priority(h: GPtr<u8>) -> i32 = stdcall cell 0x8580AC => unported;
    /// KERNEL32 `GetCurrentThread` (IAT 0x8580B0)
    pub fn get_current_thread() -> GPtr<u8> = stdcall cell 0x8580B0 => unported;
    /// KERNEL32 `CreateThread` (IAT 0x8580B4)
    pub fn create_thread(sec: u32, stack: u32, entry: u32, param: u32, flags: u32, id: GPtr<u32>) -> GPtr<u8> = stdcall cell 0x8580B4 => unported;
    /// KERNEL32 `LocalAlloc` (IAT 0x8580B8)
    pub fn local_alloc(flags: u32, n: u32) -> GPtr<u8> = stdcall cell 0x8580B8 => unported;
    /// KERNEL32 `CreateSemaphoreA` (IAT 0x8580BC)
    pub fn create_semaphore_a(sec: u32, initial: i32, max: i32, name: GPtr<u8>) -> GPtr<u8> = stdcall cell 0x8580BC => unported;
    /// KERNEL32 `GetDiskFreeSpaceA` (IAT 0x8580C0)
    pub fn get_disk_free_space_a(root: GPtr<u8>, spc: GPtr<u32>, bps: GPtr<u32>, free: GPtr<u32>, total: GPtr<u32>) -> i32 = stdcall cell 0x8580C0 => unported;
    /// CRT `printf(fmt, s)` (0x821982)
    pub fn printf_s(fmt: GPtr<u8>, s: GPtr<u8>) -> i32 = cdecl 0x821982 => unported;
    /// `CMemoryMgr::MallocAlign(size, align, hint)` (0x72F4C0)
    pub fn malloc_align(size: u32, align: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F4C0 => unported;
    /// `CMemoryMgr::FreeAlign(p)` (0x72F4F0)
    pub fn free_align(p: GPtr<u8>) = cdecl 0x72F4F0 => unported;
    /// `CdStreamSync(id)` (0x406460)
    pub fn cd_stream_sync_call(id: i32) -> u32 = cdecl 0x406460 => cd_stream_sync;
    /// `CdStreamOpen(name, unused)` (0x4067B0; the caller in CdStreamInit pushes a second 0)
    pub fn cd_stream_open_call(name: GPtr<u8>, unused: i32) -> u32 = cdecl 0x4067B0 => cd_stream_open_2;
    /// `CdStreamRead(id, buf, pos, sectors)` (0x406A20)
    pub fn cd_stream_read_call(id: i32, buf: GPtr<u8>, pos: u32, sectors: u32) -> i32 = cdecl 0x406A20 => cd_stream_read;
    /// `CdStreamRemoveImages()` (0x406690)
    pub fn cd_stream_remove_images_call() = cdecl 0x406690 => cd_stream_remove_images;
    /// `CdStreamInitThread()` (0x4068F0)
    pub fn cd_stream_init_thread_call() = cdecl 0x4068F0 => cd_stream_init_thread;
}

/// `&gCdStreams[id]` (no bounds check; `lea id*3; shl 4`).
fn stream(id: i32) -> GPtr<CdStream> {
    CD_STREAMS.get().wrapping_index(id)
}

fn f_u32(s: GPtr<CdStream>, off: isize) -> GPtr<u32> {
    s.byte_add(off).cast()
}
fn f_u8(s: GPtr<CdStream>, off: isize) -> GPtr<u8> {
    s.byte_add(off).cast()
}
fn f_ptr(s: GPtr<CdStream>, off: isize) -> GPtr<GPtr<u8>> {
    s.byte_add(off).cast()
}
const SECTORS: isize = 4;
const BUFFER: isize = 8;
const LOCKED: isize = 0xD;
const IN_USE: isize = 0xE;
const STATUS: isize = 0x10;
const SEMA: isize = 0x14;
const FILE: isize = 0x18;
const OVERLAPPED: isize = 0x1C;
const OV_OFFSET: isize = 0x24;

fn handle_slot(i: i32) -> GPtr<GPtr<u8>> {
    STREAM_FILE_HANDLES.ptr().cast::<GPtr<u8>>().wrapping_index(i)
}
fn name_slot(i: i32) -> GPtr<u8> {
    CD_IMAGE_NAMES.ptr().cast::<u8>().byte_add(64 * i as isize)
}

/// `CdStreamShutdown()` (0x406370, cdecl).
#[hook(addr = 0x406370, conv = cdecl, name = "CdStream::CdStreamShutdown", expect = "A1 E4 3F 8E 00")]
pub fn cd_stream_shutdown() {
    if STREAMING_INITIALIZED.get() != 0 {
        local_free(QUEUE_DATA.get().cast());
        close_handle(STREAM_SEMAPHORE.get());
        close_handle(STREAMING_THREAD.get());
        let mut i = 0;
        while i < STREAM_COUNT.get() {
            close_handle(f_ptr(stream(i), SEMA).read());
            i += 1;
        }
    }
    local_free(CD_STREAMS.get().cast());
}

/// `CdStreamGetStatus(id)` (stub 0x4063E0 → .HOODLUM 0x1563850, cdecl).
#[hook(addr = 0x4063E0, conv = cdecl, name = "CdStream::CdStreamGetStatus", expect = "E9 6B D4 15 01")]
pub fn cd_stream_get_status(id: i32) -> u32 {
    let s = stream(id);
    if STREAMING_INITIALIZED.get() != 0 {
        if f_u8(s, IN_USE).read() != 0 {
            return STATUS_READING;
        }
        if f_u32(s, SECTORS).read() != 0 {
            return STATUS_WAITING;
        }
        if f_u32(s, STATUS).read() != 0 {
            let st = f_u32(s, STATUS).read();
            f_u32(s, STATUS).write(0);
            return st;
        }
        return STATUS_SUCCESS;
    }
    if OVERLAPPED_IO.get() != 0 {
        // alertable = [0x55B089] ^ [0x15604CC] = 1 (protection constants, decoded from the image)
        let r = wait_for_single_object_ex(f_ptr(s, FILE).read(), 0, 1);
        return if r != 0 { STATUS_READING } else { STATUS_SUCCESS };
    }
    STATUS_SUCCESS
}

/// The INF-15 wait: `WaitForSingleObject(sema, INFINITE)` without the lost wake-up (module docs).
fn wait_stream_done(s: GPtr<CdStream>, sema: GPtr<u8>) -> u32 {
    loop {
        let r = wait_for_single_object(sema, POLL_MS);
        if r != WAIT_TIMEOUT {
            return r;
        }
        if f_u32(s, SECTORS).read() == 0 && f_u8(s, IN_USE).read() == 0 {
            // the thread released (if at all) before `bInUse = 0`: consume a pending release
            let _ = wait_for_single_object(sema, 0);
            return WAIT_OBJECT_0;
        }
    }
}

/// `CdStreamSync(id)` (stub 0x406460 → .HOODLUM 0x156CD80, cdecl).
#[hook(addr = 0x406460, conv = cdecl, name = "CdStream::CdStreamSync", expect = "E9 1B 69 16 01")]
pub fn cd_stream_sync(id: i32) -> u32 {
    let s = stream(id);
    if STREAMING_INITIALIZED.get() != 0 {
        if f_u32(s, SECTORS).read() != 0 {
            f_u8(s, LOCKED).write(1);
            let sema = f_ptr(s, SEMA).read();
            wait_stream_done(s, sema);
        }
        f_u8(s, IN_USE).write(0);
        return f_u32(s, STATUS).read();
    }
    if OVERLAPPED_IO.get() != 0 && !f_ptr(s, FILE).read().is_null() {
        let mut n = 0u32;
        // GetOverlappedResult through the protection thunk (cell 0x401009 -> 0x858084), bWait =
        // [0x1560588] - [0x545994] = 1; success only for a result of exactly 1
        let r = get_overlapped_result(
            f_ptr(s, FILE).read(),
            s.byte_add(OVERLAPPED).cast(),
            GPtr::from_raw(&raw mut n),
            1,
        );
        return if r == 1 { STATUS_SUCCESS } else { STATUS_FAILURE };
    }
    STATUS_SUCCESS
}

/// `CdStreamThread(param)` (0x406560, the streaming thread's entry; never returns).
#[hook(addr = 0x406560, conv = stdcall, name = "CdStream::CdStreamThread", expect = "83 EC 08 53 55")]
pub fn cd_stream_thread(_param: u32) -> u32 {
    loop {
        wait_for_single_object(STREAM_SEMAPHORE.get(), INFINITE);
        // GetFirstInQueue: -1 when empty (not checked)
        let id = if QUEUE_HEAD.get() == QUEUE_TAIL.get() {
            -1
        } else {
            QUEUE_DATA.get().wrapping_index(QUEUE_HEAD.get()).read()
        };
        let s = stream(id);
        f_u8(s, IN_USE).write(1);
        if f_u32(s, STATUS).read() == 0 {
            let off = f_u32(s, 0).read() << 11;
            if OVERLAPPED_IO.get() != 0 {
                f_u32(s, OV_OFFSET).write(off);
                let n = f_u32(s, SECTORS).read() << 11;
                let ov: GPtr<u32> = s.byte_add(OVERLAPPED).cast();
                let r = read_file(f_ptr(s, FILE).read(), f_ptr(s, BUFFER).read(), n, GPtr::NULL, ov);
                let st = if r == 1 {
                    STATUS_SUCCESS
                } else if get_last_error() == ERROR_IO_PENDING {
                    let mut got = 0u32;
                    let g = get_overlapped_result(f_ptr(s, FILE).read(), ov, GPtr::from_raw(&raw mut got), 1);
                    if g == 1 { STATUS_SUCCESS } else { STATUS_FAILURE }
                } else {
                    STATUS_FAILURE
                };
                f_u32(s, STATUS).write(st);
            } else {
                set_file_pointer(f_ptr(s, FILE).read(), off, GPtr::NULL, 0);
                let mut got = 0u32;
                let n = f_u32(s, SECTORS).read() << 11;
                let r = read_file(f_ptr(s, FILE).read(), f_ptr(s, BUFFER).read(), n, GPtr::from_raw(&raw mut got), GPtr::NULL);
                f_u32(s, STATUS).write(if r != 0 { STATUS_SUCCESS } else { STATUS_FAILURE });
            }
        }
        // RemoveFirstInQueue (inlined)
        let h = QUEUE_HEAD.get();
        if h != QUEUE_TAIL.get() {
            QUEUE_HEAD.set(h.wrapping_add(1).wrapping_rem(QUEUE_SIZE.get()));
        }
        f_u32(s, SECTORS).write(0);
        if f_u8(s, LOCKED).read() != 0 {
            release_semaphore(f_ptr(s, SEMA).read(), 1, GPtr::NULL);
        }
        f_u8(s, IN_USE).write(0);
    }
}

/// `CdStreamRemoveImages()` (0x406690, cdecl): sync every stream, close every open image.
#[hook(addr = 0x406690, conv = cdecl, name = "CdStream::CdStreamRemoveImages", expect = "A1 90 40 8E 00")]
pub fn cd_stream_remove_images() {
    let mut i = 0;
    while i < STREAM_COUNT.get() {
        cd_stream_sync_call(i);
        i += 1;
    }
    let mut i = 0;
    while i < OPEN_STREAM_COUNT.get() {
        set_last_error(0);
        let h = handle_slot(i).read();
        if !h.is_null() {
            close_handle(h);
        }
        handle_slot(i).write(GPtr::NULL);
        name_slot(i).write(0);
        i += 1;
    }
    OPEN_STREAM_COUNT.set(0);
}

/// `CdStreamOpen(name)` (stub 0x4067B0 → .HOODLUM 0x1564A90, cdecl): the first free handle slot,
/// `CreateFileA(name, GENERIC_READ, FILE_SHARE_READ, 0, OPEN_EXISTING, flags | 0x10000001, 0)`,
/// `strcpy` of the name; returns `slot << 24` (0 on INVALID_HANDLE_VALUE).
///
/// The slot search is unrolled by eight: a free slot at group offset `k` = 1..6 is `add esi, k`
/// (0x1564AFA..0x1564B11); offset 7 (slots 7, 15, 23, 31) goes through the protection VM
/// (0xCB73E0, descriptor 0x155FF5C) and adds its result instead. Taken as 7 by consistency (the
/// compiler's `add esi, 7`, replaced by the protector; the shipped game opens more than eight
/// streams, e.g. the SFX paks, which reach slot 7 on every boot); not decoded from the VM, needs L2.
pub fn cd_stream_open(name: GPtr<u8>) -> u32 {
    let mut i = 0i32;
    while i < 32 {
        if handle_slot(i).read().is_null() {
            break;
        }
        i += 1;
    }
    set_last_error(0);
    let flags = STREAM_FILE_CREATE_FLAGS.get() | 0x1000_0001;
    let h = create_file_a(name, 0x8000_0000, 1, 0, 3, flags, 0);
    handle_slot(i).write(h);
    if h.addr() as u32 == 0xFFFF_FFFF {
        return 0;
    }
    let dst = name_slot(i);
    let mut k = 0;
    loop {
        let c = name.byte_add(k).read();
        dst.byte_add(k).write(c);
        k += 1;
        if c == 0 {
            break;
        }
    }
    (i as u32) << 24
}

/// [`cd_stream_open`] with the second argument CdStreamInit pushes (unused).
/// Phase B entry of `CdStreamOpen(name, unused)` (0x4067B0; no hook: the slot search's
/// protection-VM offset for slots 7/15/23/31 is taken as 7, not decoded).
#[sa_core::native(addr = 0x4067B0, conv = cdecl, name = "CdStream::CdStreamOpen")]
pub fn cd_stream_open_2(name: GPtr<u8>, _unused: i32) -> u32 {
    cd_stream_open(name)
}

/// `CdStreamInitThread()` (0x4068F0, cdecl).
#[hook(addr = 0x4068F0, conv = cdecl, name = "CdStream::CdStreamInitThread", expect = "53 56 57 6A 00")]
pub fn cd_stream_init_thread() {
    set_last_error(0);
    let mut n = STREAM_COUNT.get();
    let mut i = 0;
    while i < n {
        let sema = create_semaphore_a(0, 0, 2, GPtr::NULL);
        f_ptr(stream(i), SEMA).write(sema);
        if sema.is_null() {
            printf_s(image(MSG_SYNC_SEMA), image(MSG_PREFIX));
            return;
        }
        n = STREAM_COUNT.get();
        i += 1;
    }
    let q = local_alloc(0x40, (n as u32).wrapping_mul(4).wrapping_add(4));
    QUEUE_DATA.set(q.cast());
    QUEUE_HEAD.set(0);
    QUEUE_TAIL.set(0);
    QUEUE_SIZE.set(STREAM_COUNT.get().wrapping_add(1));
    let sema = create_semaphore_a(0, 0, 5, image(NAME_CDSTREAM));
    STREAM_SEMAPHORE.set(sema);
    if sema.is_null() {
        printf_s(image(MSG_STREAM_SEMA), image(MSG_PREFIX));
        return;
    }
    let t = create_thread(0, 0x10000, THREAD_ENTRY, 0, 4, STREAMING_THREAD_ID.ptr());
    STREAMING_THREAD.set(t);
    if t.is_null() {
        printf_s(image(MSG_THREAD), image(MSG_PREFIX));
        return;
    }
    let p = get_thread_priority(get_current_thread());
    set_thread_priority(STREAMING_THREAD.get(), p);
    resume_thread(STREAMING_THREAD.get());
}

/// `CdStreamRead(id, buf, pos, sectors)` (stub 0x406A20 → .HOODLUM 0x156C2C0, cdecl).
///
/// Deferred as a hook: 0x406A20 is sa-hook's own determinism-mode hook (`Id::CdStreamRead`, a
/// synchronous read); two hooks cannot share the entry. sa-hook can call this port as its
/// "original" once it is verified.
#[sa_core::native(addr = 0x406A20, conv = cdecl, name = "CdStream::CdStreamRead")]
pub fn cd_stream_read(id: i32, buf: GPtr<u8>, pos: u32, sectors: u32) -> i32 {
    let s = stream(id);
    let h = handle_slot((pos >> 24) as i32).read();
    LAST_CD_STREAM_POSN.set(sectors.wrapping_add(pos));
    let off = pos & 0xFF_FFFF;
    f_ptr(s, FILE).write(h);
    set_last_error(0);
    if STREAMING_INITIALIZED.get() != 0 {
        if f_u32(s, SECTORS).read() != 0 || f_u8(s, IN_USE).read() != 0 {
            return 0;
        }
        f_u32(s, STATUS).write(0);
        f_u32(s, 0).write(off);
        f_u32(s, SECTORS).write(sectors);
        f_ptr(s, BUFFER).write(buf);
        f_u8(s, LOCKED).write(0);
        // AddToQueue: data[tail] = id (tail address decoded: [0x1560578] ^ [0x527D09] = 0x8E3FF4)
        QUEUE_DATA.get().wrapping_index(QUEUE_TAIL.get()).write(id);
        QUEUE_TAIL.set(QUEUE_TAIL.get().wrapping_add(1).wrapping_rem(QUEUE_SIZE.get()));
        let _ok = release_semaphore(STREAM_SEMAPHORE.get(), 1, GPtr::NULL);
        // on failure the original printf()s a string from the protection VM (not reproduced)
        return 1;
    }
    let off = off << 11;
    if OVERLAPPED_IO.get() != 0 {
        f_u32(s, OV_OFFSET).write(off);
        let ov: GPtr<u32> = s.byte_add(OVERLAPPED).cast();
        if read_file(h, buf, sectors << 11, GPtr::NULL, ov) != 0 {
            return 1;
        }
        return (get_last_error() == ERROR_IO_PENDING) as i32;
    }
    set_file_pointer(h, off, GPtr::NULL, 0);
    let mut got = 0u32;
    read_file(h, buf, sectors << 11, GPtr::from_raw(&raw mut got), GPtr::NULL)
}

/// `CdStreamInit(count)` (0x406B70, cdecl).
#[hook(addr = 0x406B70, conv = cdecl, name = "CdStream::CdStreamInit", expect = "83 EC 10 57 B9 20 00 00 00")]
pub fn cd_stream_init(count: i32) {
    for i in 0..32 {
        handle_slot(i).write(GPtr::NULL);
    }
    for i in 0..32 {
        name_slot(i).write(0);
    }
    let (mut spc, mut bps, mut free, mut total) = (0u32, 0u32, 0u32, 0u32);
    get_disk_free_space_a(
        GPtr::NULL,
        GPtr::from_raw(&raw mut spc),
        GPtr::from_raw(&raw mut bps),
        GPtr::from_raw(&raw mut free),
        GPtr::from_raw(&raw mut total),
    );
    let flags = if bps <= 0x800 { 0x2000_0000 } else { 0 } | 0x4000_0000;
    OVERLAPPED_IO.set(1);
    STREAM_FILE_CREATE_FLAGS.set(flags);
    STREAMING_INITIALIZED.set(0);
    let mem = malloc_align(0x800, bps, 0);
    set_last_error(0);
    OPEN_STREAM_COUNT.set(0);
    STREAM_COUNT.set(count);
    let streams = local_alloc(0x40, (count as u32).wrapping_mul(0x30));
    CD_STREAMS.set(streams.cast());
    cd_stream_open_call(image(NAME_GTA3_IMG), 0);
    let ok = cd_stream_read_call(0, mem, 0, 1);
    cd_stream_remove_images_call();
    STREAMING_INITIALIZED.set(1);
    if ok != 1 {
        OVERLAPPED_IO.set(0);
        STREAM_FILE_CREATE_FLAGS.set(STREAM_FILE_CREATE_FLAGS.get() & 0xBFFF_FFFF);
    }
    cd_stream_init_thread_call();
    free_align(mem);
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[sa_core::abi::HookReg] = &[
    HOOK_CD_STREAM_SHUTDOWN,
    HOOK_CD_STREAM_GET_STATUS,
    HOOK_CD_STREAM_SYNC,
    HOOK_CD_STREAM_THREAD,
    HOOK_CD_STREAM_REMOVE_IMAGES,
    HOOK_CD_STREAM_INIT_THREAD,
    HOOK_CD_STREAM_INIT,
];

#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    #[test]
    fn decoded_protection_constants() {
        // GetStatus alertable flag, Sync bWait, Read's queue-tail address, thunk cell keys
        assert_eq!(0xFFEA_61B3u32 ^ 0xFFEA_61B2, 1);
        assert_eq!(0xEBC8_ACEAu32.wrapping_sub(0xEBC8_ACE9), 1);
        assert_eq!(0xFF63_A075u32 ^ 0xFFED_9F81, super::QUEUE_TAIL.addr());
        assert_eq!(0x21C8_CCCCu32 ^ 0x214D_4C48, 0x858084);
        assert_eq!(0x21C8_CCE8u32 ^ 0x214D_4C48, 0x8580A0);
    }
}
