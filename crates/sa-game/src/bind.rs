//! Binding of the ported code's Win32/CRT/DirectInput declarations to sa-platform.
//!
//! Every import the ports call is an `orig!` declaration `=> unported` (ADR-001 §4). In phase B
//! such a call looks up a stand-in registered with `sa_core::orig::host::set` under the
//! declaration's path (`module_path!()::name`) and the declared signature; without one it panics
//! with the function's name, which the boot runner reports. This module registers the stand-ins
//! for the imports on the boot → menu → walking path whose Win32 semantics sa-platform
//! implements. They use the [`Platform`] behind a global mutex (the `CdStream` streaming thread
//! calls the file and kernel stand-ins too) and the lock-free [`kernel()`] table for waits.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use sa_platform::kernel::Kernel;

use sa_core::input::ComObj;
use sa_core::mem::GPtr;
use sa_core::orig::host;
use sa_platform::input::{DiJoyState2, InputState};
use sa_platform::vfs::{FileTable, Vfs};
use sa_platform::win32::{MessageQueue, Win32Time, map_virtual_key};

/// The window handle the stand-ins hand out (`CreateWindowExA`).
pub const HWND: u32 = 0x0001_0010;
/// Pseudo handles for objects that only need to exist (events, registry keys, COM objects).
const HKEY_SHELL_FOLDERS: u32 = 0x0002_0001;
const EVENT_HANDLE: u32 = 0x0002_0010;
/// Fake COM objects: DirectInput mouse and pads (never dereferenced: every method is bound).
pub const DI_MOUSE: usize = 0x0003_0000;
pub const DI_PAD1: usize = 0x0003_0010;
pub const DI_PAD2: usize = 0x0003_0020;

/// Native platform state behind the stand-ins.
pub struct Platform {
    pub vfs: Vfs,
    pub files: FileTable,
    pub time: Win32Time<crate::det::GameClock>,
    pub queue: MessageQueue,
    pub input: InputState,
    pub last_error: u32,
    /// Messages `DispatchMessageA` could not deliver because `MainWndProc` (0x747EB0) is not
    /// ported, per message id.
    pub undelivered: BTreeMap<u32, u32>,
    /// Calls of bound stand-ins, per declaration (for the report).
    pub calls: BTreeMap<&'static str, u64>,
}

static PLATFORM: Mutex<Option<Platform>> = Mutex::new(None);
static KERNEL: OnceLock<Kernel> = OnceLock::new();

/// The kernel object table (semaphores, threads, `LocalAlloc`).
pub fn kernel() -> &'static Kernel {
    KERNEL.get_or_init(Kernel::default)
}

pub fn install(p: Platform) {
    *PLATFORM.lock().unwrap_or_else(|e| e.into_inner()) = Some(p);
    register();
}

/// Runs `f` on the platform state. Stand-ins never call back into game code while holding it.
pub fn with<R>(key: &'static str, f: impl FnOnce(&mut Platform) -> R) -> R {
    let mut b = PLATFORM.lock().unwrap_or_else(|e| e.into_inner());
    let p = b.as_mut().expect("sa-game: platform not installed");
    *p.calls.entry(key).or_default() += 1;
    f(p)
}

/// Reads a NUL-terminated string from game memory (at most 4 KiB).
pub fn cstr(p: GPtr<u8>) -> String {
    let mut v = Vec::new();
    if p.is_null() {
        return String::new();
    }
    for i in 0..4096 {
        let c = p.byte_add(i).read();
        if c == 0 {
            break;
        }
        v.push(c);
    }
    // the game's strings are ANSI (CP1252); paths and literals are ASCII
    v.iter().map(|&b| b as char).collect()
}

/// Writes `s` + NUL into a `cap`-byte buffer; false if it does not fit.
pub(crate) fn write_cstr(dst: GPtr<u8>, s: &str, cap: usize) -> bool {
    let b = s.as_bytes();
    if b.len() + 1 > cap {
        return false;
    }
    for (i, c) in b.iter().chain(std::iter::once(&0)).enumerate() {
        dst.byte_add(i as isize).write(*c);
    }
    true
}

pub(crate) fn handle(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

// ------------------------------------------------------------------------------- timers ---

pub(crate) fn qpf(out: GPtr<u64>) -> i32 {
    out.write(with("QueryPerformanceFrequency", |p| {
        p.time.query_performance_frequency()
    }));
    1
}
pub(crate) fn qpc(out: GPtr<u64>) -> i32 {
    out.write(with("QueryPerformanceCounter", |p| {
        p.time.query_performance_counter()
    }));
    1
}
pub(crate) fn time_get_time() -> u32 {
    with("timeGetTime", |p| p.time.time_get_time())
}
pub(crate) fn time_get_dev_caps(caps: GPtr<[u32; 2]>, _size: u32) -> u32 {
    caps.write([1, 1_000_000]);
    0
}
pub(crate) fn time_period(_ms: u32) -> u32 {
    0
}

// --------------------------------------------------------------------------- CRT stdio ---

pub(crate) fn fopen(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> {
    let (path, mode) = (cstr(path), cstr(mode));
    let h = with("fopen", |p| {
        let Platform { files, vfs, .. } = p;
        files.fopen(vfs, &path, &mode)
    });
    GPtr::from_addr(h.unwrap_or(0) as usize)
}
pub(crate) fn fread(buf: GPtr<u8>, size: u32, n: u32, f: GPtr<u8>) -> u32 {
    let total = size.saturating_mul(n) as usize;
    let mut tmp = vec![0u8; total];
    let got = with("fread", |p| p.files.read(handle(f), &mut tmp));
    for (i, b) in tmp[..got].iter().enumerate() {
        buf.byte_add(i as isize).write(*b);
    }
    if size == 0 {
        0
    } else {
        (got / size as usize) as u32
    }
}
pub(crate) fn fwrite(buf: GPtr<u8>, size: u32, n: u32, f: GPtr<u8>) -> u32 {
    let total = size.saturating_mul(n) as usize;
    let data: Vec<u8> = (0..total).map(|i| buf.byte_add(i as isize).read()).collect();
    let put = with("fwrite", |p| p.files.write(handle(f), &data));
    if size == 0 {
        0
    } else {
        (put / size as usize) as u32
    }
}
pub(crate) fn fseek(f: GPtr<u8>, offset: i32, origin: i32) -> i32 {
    with("fseek", |p| p.files.seek(handle(f), offset as i64, origin))
}
pub(crate) fn fgets(buf: GPtr<u8>, n: i32, f: GPtr<u8>) -> GPtr<u8> {
    match with("fgets", |p| p.files.gets(handle(f), n.max(0) as usize)) {
        Some(line) => {
            for (i, b) in line.iter().enumerate() {
                buf.byte_add(i as isize).write(*b);
            }
            buf
        }
        None => GPtr::from_addr(0),
    }
}
pub(crate) fn fclose(f: GPtr<u8>) -> i32 {
    with("fclose", |p| {
        let Platform { files, vfs, .. } = p;
        files.close(vfs, handle(f))
    })
}
pub(crate) fn getcwd(buf: GPtr<u8>, n: i32) -> GPtr<u8> {
    let cwd = with("_getcwd", |p| p.vfs.getcwd());
    if write_cstr(buf, &cwd, n.max(0) as usize) {
        buf
    } else {
        GPtr::from_addr(0)
    }
}
pub(crate) fn chdir(path: GPtr<u8>) -> i32 {
    let path = cstr(path);
    with("_chdir", |p| p.vfs.chdir(&path))
}
pub(crate) fn rwftell(f: GPtr<u8>) -> i32 {
    with("rwftell", |p| p.files.tell(handle(f)) as i32)
}
pub(crate) fn rwfseek(f: GPtr<u8>, offset: i32, whence: i32) -> i32 {
    with("rwfseek", |p| p.files.seek(handle(f), offset as i64, whence))
}

// ------------------------------------------------------------------- Win32 (kernel/user) ---

pub(crate) fn create_event_a(_attr: u32, _manual: i32, _initial: i32, _name: GPtr<u8>) -> u32 {
    // a fresh named event: the game is not already running
    with("CreateEventA", |p| p.last_error = 0);
    EVENT_HANDLE
}
pub(crate) fn get_last_error() -> u32 {
    with("GetLastError", |p| p.last_error)
}
pub(crate) fn find_window_a(_class: GPtr<u8>, _name: GPtr<u8>) -> u32 {
    0
}
pub(crate) fn set_foreground_window(_hwnd: u32) -> i32 {
    1
}
pub(crate) fn reg_open_key_ex_a(_key: u32, sub: GPtr<u8>, _opts: u32, _sam: u32, out: GPtr<u32>) -> i32 {
    if cstr(sub).eq_ignore_ascii_case("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Shell Folders")
    {
        out.write(HKEY_SHELL_FOLDERS);
        0
    } else {
        2 // ERROR_FILE_NOT_FOUND
    }
}
pub(crate) fn reg_query_value_ex_a(
    key: u32,
    name: GPtr<u8>,
    _r: u32,
    ty: GPtr<u32>,
    data: GPtr<u8>,
    size: GPtr<u32>,
) -> i32 {
    if key != HKEY_SHELL_FOLDERS || !cstr(name).eq_ignore_ascii_case("Personal") {
        return 2;
    }
    let docs = sa_platform::vfs::DOCUMENTS;
    let need = docs.len() as u32 + 1;
    if size.read() < need {
        size.write(need);
        return 234; // ERROR_MORE_DATA
    }
    if !ty.is_null() {
        ty.write(1); // REG_SZ
    }
    write_cstr(data, docs, need as usize);
    size.write(need);
    0
}
pub(crate) fn reg_close_key(_key: u32) -> i32 {
    0
}
pub(crate) fn create_file_a(name: GPtr<u8>, access: u32, _share: u32, _sec: u32, disp: u32, flags: u32, _t: u32) -> u32 {
    let path = cstr(name);
    with("CreateFileA", |p| {
        // FILE_FLAG_BACKUP_SEMANTICS on a directory: a directory handle (only closed again)
        if flags & 0x0200_0000 != 0 && p.vfs.is_dir(&path) {
            return EVENT_HANDLE + 4;
        }
        let write = access & 0x4000_0000 != 0;
        let mode = match (write, disp) {
            (false, _) => "rb",
            (true, 2) => "wb", // CREATE_ALWAYS
            (true, _) => "r+b",
        };
        let Platform {
            files,
            vfs,
            last_error,
            ..
        } = p;
        match files.fopen(vfs, &path, mode) {
            Some(h) => h,
            None => {
                *last_error = 2;
                0xFFFF_FFFF
            }
        }
    })
}
pub(crate) fn create_directory_a(name: GPtr<u8>, _sec: u32) -> i32 {
    let path = cstr(name);
    with("CreateDirectoryA", |p| p.vfs.mkdir(&path).is_ok() as i32)
}
pub(crate) fn close_handle(h: u32) -> i32 {
    with("CloseHandle", |p| {
        if (FileTable::FIRST_HANDLE..Kernel::FIRST_HANDLE).contains(&h) {
            let Platform { files, vfs, .. } = p;
            (files.close(vfs, h) == 0) as i32
        } else {
            1
        }
    })
}

// ------------------------------------------------------------------------ window (user32) ---

/// Client size of the native window (`RsGlobal.maximumWidth/Height` start at 640x480).
pub const WINDOW_SIZE: (i32, i32) = (640, 480);

pub(crate) fn load_icon_a(_instance: u32, _id: u32) -> u32 {
    0x0005_0001
}
pub(crate) fn load_cursor_a(_instance: u32, _id: u32) -> u32 {
    0x0005_0002
}
pub(crate) fn register_class_a(wc: GPtr<[u32; 10]>) -> u32 {
    WND_PROC.store(wc.read()[1], std::sync::atomic::Ordering::SeqCst);
    0xC001
}
pub(crate) fn adjust_window_rect(_rect: GPtr<[i32; 4]>, _style: u32, _menu: i32) -> i32 {
    // the native window has no Win32 frame: client rect = window rect
    1
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn create_window_ex_a(
    _ex: u32,
    _class: GPtr<u8>,
    _name: GPtr<u8>,
    _style: u32,
    _x: i32,
    _y: i32,
    _w: i32,
    _h: i32,
    _parent: u32,
    _menu: u32,
    _instance: u32,
    _param: u32,
) -> u32 {
    HWND
}
pub(crate) fn show_cursor(show: i32) -> i32 {
    show
}
/// `GetWindowRect`/`GetClientRect`: once the D3D device exists the window covers its back buffer
/// (the exclusive video mode the game selected), before that the initial window size.
pub(crate) fn get_window_rect(_hwnd: u32, rect: GPtr<[i32; 4]>) -> i32 {
    let (w, h) = crate::d3d9::with_device(|d| sa_platform::gpu::D3d9Device::back_buffer_size(d))
        .map_or(WINDOW_SIZE, |(w, h)| (w as i32, h as i32));
    rect.write([0, 0, w, h]);
    1
}
pub(crate) fn set_window_text_a(_hwnd: u32, _text: GPtr<u8>) -> i32 {
    1
}
pub(crate) fn client_to_screen(_hwnd: u32, _pt: GPtr<[i32; 2]>) -> i32 {
    1
}
pub(crate) fn set_cursor_pos(_x: i32, _y: i32) -> i32 {
    1
}
pub(crate) fn lcid() -> u32 {
    0x0409 // en-US
}
pub(crate) fn keyboard_layout(_thread: u32) -> u32 {
    0x0409_0409
}
pub(crate) fn message_box_a(_hwnd: u32, text: GPtr<u8>, caption: GPtr<u8>, _ty: u32) -> i32 {
    eprintln!("sa-game: MessageBox {:?}: {:?}", cstr(caption), cstr(text));
    1 // IDOK
}
pub(crate) fn output_debug_string_a(s: GPtr<u8>) {
    eprintln!("sa-game: OutputDebugString {:?}", cstr(s));
}

// ------------------------------------------------------------------------ messages/keys ---

pub(crate) fn peek_message_a(msg: GPtr<u8>, _hwnd: u32, _min: u32, _max: u32, remove: u32) -> i32 {
    match with("PeekMessageA", |p| p.queue.peek(remove & 1 != 0)) {
        Some(m) => {
            msg.cast::<[u32; 7]>().write(m.to_words());
            1
        }
        None => 0,
    }
}
/// `PostQuitMessage(code)`: `WM_QUIT` once the queue is empty.
pub(crate) fn post_quit_message(code: i32) {
    with("PostQuitMessage", |p| p.queue.post_quit(code as u32));
}
/// `DefWindowProcA`: no default processing a headless window needs; 0.
pub(crate) fn def_window_proc_a(_hwnd: u32, _msg: u32, _wparam: u32, _lparam: u32) -> i32 {
    with("DefWindowProcA", |_| ());
    0
}
/// `SetCapture`/`SetFocus`: no previous window (NULL).
pub(crate) fn set_capture(_hwnd: u32) -> u32 {
    0
}
/// `ReleaseCapture`, `ClipCursor`: success.
pub(crate) fn release_capture() -> i32 {
    1
}
pub(crate) fn clip_cursor(_rect: u32) -> i32 {
    1
}
/// `SetCursor`: no previous cursor.
pub(crate) fn set_cursor(_cursor: u32) -> u32 {
    0
}
/// `SetTimer(hwnd, id, elapse, func)`: the timer id (no `WM_TIMER` is generated; the game's handler
/// for it only returns 0).
pub(crate) fn set_timer(_hwnd: u32, id: u32, _elapse: u32, _func: u32) -> u32 {
    id
}
/// `SendMessageA`: the only sender is `WM_SIZE`'s maximised-window case (`WM_WINDOWPOSCHANGED`,
/// which the game's window procedure passes to `DefWindowProcA`): 0.
pub(crate) fn send_message_a(_hwnd: u32, _msg: u32, _wparam: u32, _lparam: u32) -> i32 {
    0
}
pub(crate) fn translate_message(_msg: GPtr<u8>) -> i32 {
    // WM_CHAR is queued from the layout's text events when the key event arrives
    1
}
/// `DispatchMessageA`: calls the window procedure `RegisterClassA` received (`Win::__MainWndProc`
/// 0x747EB0) through `orig!`'s registry. While it has no Rust port, the message is counted as
/// undelivered (per id, for the report) instead of failing the caller.
pub(crate) fn dispatch_message_a(msg: GPtr<u8>) -> i32 {
    let m = msg.cast::<[u32; 7]>().read();
    with("DispatchMessageA", |_| ());
    let proc_ = WND_PROC.load(std::sync::atomic::Ordering::SeqCst);
    let r = std::panic::catch_unwind(|| call_wnd_proc(proc_, m[0], m[1], m[2], m[3]));
    match r {
        Ok(v) => v,
        Err(_) => {
            with("report", |p| *p.undelivered.entry(m[1]).or_default() += 1);
            0
        }
    }
}

/// The `lpfnWndProc` of the registered window class.
static WND_PROC: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

sa_core::orig! {
    /// a window procedure (`LRESULT CALLBACK (HWND, UINT, WPARAM, LPARAM)`) at a run-time address
    fn call_wnd_proc(f: u32, hwnd: u32, msg: u32, wparam: u32, lparam: u32) -> i32 = stdcall dyn => unported;
}
pub(crate) fn get_key_state(vk: i32) -> i32 {
    with("GetKeyState", |p| p.queue.keys.get_key_state(vk))
}
pub(crate) fn map_virtual_key_a(code: u32, kind: u32) -> u32 {
    map_virtual_key(code, kind)
}

// ------------------------------------------------------------------- CdStream (kernel32) ---

/// `GetDiskFreeSpaceA`: 2048-byte sectors (the original then opens the IMGs with
/// `FILE_FLAG_NO_BUFFERING`, which changes nothing here).
pub(crate) fn get_disk_free_space_a(
    _root: GPtr<u8>,
    spc: GPtr<u32>,
    bps: GPtr<u32>,
    free: GPtr<u32>,
    total: GPtr<u32>,
) -> i32 {
    spc.write(8);
    bps.write(0x800);
    free.write(0x0100_0000);
    total.write(0x0200_0000);
    1
}
pub(crate) fn h32(h: GPtr<u8>) -> u32 {
    h.addr() as u32
}
pub(crate) fn cd_create_file_a(
    name: GPtr<u8>,
    access: u32,
    share: u32,
    sec: u32,
    disp: u32,
    flags: u32,
    t: u32,
) -> GPtr<u8> {
    GPtr::from_addr(create_file_a(name, access, share, sec, disp, flags, t) as usize)
}
pub(crate) fn cd_close_handle(h: GPtr<u8>) -> i32 {
    let h = h32(h);
    if Kernel::is_kernel_handle(h) {
        kernel().close(h) as i32
    } else {
        close_handle(h)
    }
}
pub(crate) fn set_last_error(e: u32) {
    with("SetLastError", |p| p.last_error = e);
}
pub(crate) fn local_alloc(_flags: u32, n: u32) -> GPtr<u8> {
    GPtr::from_addr(kernel().local_alloc(n as usize))
}
pub(crate) fn local_free(p: GPtr<u8>) -> GPtr<u8> {
    if kernel().local_free(p.addr()) {
        GPtr::from_addr(0)
    } else {
        p
    }
}
pub(crate) fn create_semaphore_a(_sec: u32, initial: i32, max: i32, _name: GPtr<u8>) -> GPtr<u8> {
    GPtr::from_addr(kernel().create_semaphore(initial, max) as usize)
}
pub(crate) fn release_semaphore(h: GPtr<u8>, n: i32, prev: GPtr<i32>) -> i32 {
    let r = match kernel().release_semaphore(h32(h), n) {
        Some(v) => {
            if !prev.is_null() {
                prev.write(v);
            }
            1
        }
        None => 0,
    };
    crate::det::released(h32(h));
    r
}
pub(crate) fn wait_for_single_object(h: GPtr<u8>, ms: u32) -> u32 {
    crate::det::wait(h32(h), ms, |h, ms| kernel().wait(h, ms))
}
pub(crate) fn wait_for_single_object_ex(h: GPtr<u8>, ms: u32, _alertable: i32) -> u32 {
    crate::det::wait(h32(h), ms, |h, ms| kernel().wait(h, ms))
}
/// `ReadFile`. With an `OVERLAPPED` the read happens at its offset and completes synchronously
/// (TRUE, `InternalHigh` = bytes), which Win32 allows for overlapped handles.
pub(crate) fn read_file(h: GPtr<u8>, buf: GPtr<u8>, n: u32, read: GPtr<u32>, ov: GPtr<u32>) -> i32 {
    let mut tmp = vec![0u8; n as usize];
    let got = with("ReadFile", |p| {
        if ov.is_null() {
            p.files.read(h32(h), &mut tmp)
        } else {
            let off = ov.wrapping_index(2).read() as u64 | (ov.wrapping_index(3).read() as u64) << 32;
            p.files.read_at(h32(h), off, &mut tmp)
        }
    });
    for (i, b) in tmp[..got].iter().enumerate() {
        buf.byte_add(i as isize).write(*b);
    }
    if !ov.is_null() {
        ov.write(0);
        ov.wrapping_index(1).write(got as u32);
    }
    if !read.is_null() {
        read.write(got as u32);
    }
    1
}
pub(crate) fn get_overlapped_result(_h: GPtr<u8>, ov: GPtr<u32>, n: GPtr<u32>, _wait: i32) -> i32 {
    n.write(ov.wrapping_index(1).read());
    1
}
pub(crate) fn set_file_pointer(h: GPtr<u8>, lo: u32, hi: GPtr<u32>, method: u32) -> u32 {
    let off = if hi.is_null() {
        lo as i32 as i64
    } else {
        ((hi.read() as u64) << 32 | lo as u64) as i64
    };
    with("SetFilePointer", |p| {
        if p.files.seek(h32(h), off, method as i32) != 0 {
            return 0xFFFF_FFFF;
        }
        let pos = p.files.tell(h32(h)) as u64;
        if !hi.is_null() {
            hi.write((pos >> 32) as u32);
        }
        pos as u32
    })
}
/// `CreateThread(…, entry, param, CREATE_SUSPENDED, &id)`: the entry is an original code
/// address, resolved to its Rust port.
pub(crate) fn create_thread(_sec: u32, _stack: u32, entry: u32, param: u32, _flags: u32, id: GPtr<u32>) -> GPtr<u8> {
    let f: Box<dyn FnOnce() + Send> = match entry {
        sa_stream::cdstream::THREAD_ENTRY => Box::new(move || {
            sa_stream::cdstream::cd_stream_thread(param);
        }),
        // the audio threads (sa-audio port4_hw): smooth volume fades and the stream thread
        0x4E_EE90 => Box::new(move || {
            sa_audio::port4_hw::smooth_fade_proc(param);
        }),
        0x4F_15C0 => Box::new(move || {
            sa_audio::port4_hw::stream_thread_main_loop(param);
        }),
        _ => return GPtr::from_addr(0),
    };
    let (h, tid) = kernel().create_thread(f);
    if !id.is_null() {
        id.write(tid);
    }
    GPtr::from_addr(h as usize)
}
pub(crate) fn resume_thread(h: GPtr<u8>) -> u32 {
    kernel().resume_thread(h32(h))
}
pub(crate) fn get_current_thread() -> GPtr<u8> {
    GPtr::from_addr(0xFFFF_FFFE) // the pseudo handle
}
pub(crate) fn get_thread_priority(h: GPtr<u8>) -> i32 {
    kernel().thread_priority(h32(h))
}
pub(crate) fn set_thread_priority(h: GPtr<u8>, p: i32) -> i32 {
    kernel().set_thread_priority(h32(h), p) as i32
}
/// `CMemoryMgr::MallocAlign` (0x72F4C0) / `FreeAlign` (0x72F4F0) stand-ins (not ported): plain
/// leaked-until-freed allocations through the kernel table.
pub(crate) fn malloc_align(size: u32, align: u32, _hint: u32) -> GPtr<u8> {
    if crate::det::fixed_heap() {
        // the deterministic heap: the block's own address in the dword before the aligned pointer
        let a = (align as usize).max(4);
        let raw = crate::detheap::alloc(size as usize + a + 4);
        let p = (raw.addr() + 4).div_ceil(a) * a;
        GPtr::<u32>::from_addr(p - 4).write(raw.addr() as u32);
        return GPtr::from_addr(p);
    }
    GPtr::from_addr(kernel().local_alloc(size as usize))
}
pub(crate) fn free_align(p: GPtr<u8>) {
    if crate::det::fixed_heap() {
        if !p.is_null() {
            crate::detheap::free(GPtr::from_addr(
                GPtr::<u32>::from_addr(p.addr() - 4).read() as usize
            ));
        }
        return;
    }
    kernel().local_free(p.addr());
}

// --------------------------------------------------------------------------- DirectInput ---

pub(crate) fn did_acquire(_this: GPtr<ComObj>) -> i32 {
    0
}
pub(crate) fn did_poll(_this: GPtr<ComObj>) -> i32 {
    0
}
pub(crate) fn did_get_device_state(this: GPtr<ComObj>, size: u32, data: GPtr<u8>) -> i32 {
    let bytes: Option<Vec<u8>> = with("IDirectInputDevice8::GetDeviceState", |p| match this.addr() {
        DI_MOUSE if size == 0x14 => Some(p.input.di_mouse_state().to_bytes().to_vec()),
        a @ (DI_PAD1 | DI_PAD2) if size == 0x110 => {
            let i = usize::from(a == DI_PAD2);
            p.input.pads.get(i).map(|g| {
                DiJoyState2::from_pad(g, DiJoyState2::GAME_RANGE)
                    .to_bytes()
                    .to_vec()
            })
        }
        _ => None,
    });
    match bytes {
        Some(b) => {
            for (i, v) in b.iter().enumerate() {
                data.byte_add(i as isize).write(*v);
            }
            0
        }
        None => 0x8007_001Eu32 as i32, // DIERR_INPUTLOST
    }
}

/// Registers every stand-in. The keys and signatures must match the `orig!` declarations
/// exactly (`host::get` asserts the signature).
pub(crate) fn register() {
    host::set::<fn(GPtr<u64>) -> i32>("sa_core::timer::query_performance_frequency", qpf);
    host::set::<fn(GPtr<u64>) -> i32>("sa_core::timer::query_performance_counter_cycles", qpc);
    host::set::<fn() -> u32>("sa_core::platform::call::time_get_time", time_get_time);
    host::set::<fn(GPtr<[u32; 2]>, u32) -> u32>(
        "sa_core::platform::call::time_get_dev_caps",
        time_get_dev_caps,
    );
    host::set::<fn(u32) -> u32>("sa_core::platform::call::time_begin_period", time_period);
    host::set::<fn(u32) -> u32>("sa_core::platform::call::time_end_period", time_period);

    host::set::<fn(GPtr<u8>, GPtr<u8>) -> GPtr<u8>>("sa_core::filemgr::fopen", fopen);
    host::set::<fn(GPtr<u8>, u32, u32, GPtr<u8>) -> u32>("sa_core::filemgr::fread", fread);
    host::set::<fn(GPtr<u8>, u32, u32, GPtr<u8>) -> u32>("sa_core::filemgr::fwrite", fwrite);
    host::set::<fn(GPtr<u8>, i32, i32) -> i32>("sa_core::filemgr::fseek", fseek);
    host::set::<fn(GPtr<u8>, i32, GPtr<u8>) -> GPtr<u8>>("sa_core::filemgr::fgets", fgets);
    host::set::<fn(GPtr<u8>) -> i32>("sa_core::filemgr::fclose", fclose);
    host::set::<fn(GPtr<u8>, i32) -> GPtr<u8>>("sa_core::filemgr::getcwd", getcwd);
    host::set::<fn(GPtr<u8>) -> i32>("sa_core::filemgr::chdir", chdir);
    host::set::<fn(GPtr<u8>) -> i32>("sa_core::filemgr::rwftell", rwftell);
    host::set::<fn(GPtr<u8>, i32, i32) -> i32>("sa_core::filemgr::rwfseek", rwfseek);

    host::set::<fn(u32, i32, i32, GPtr<u8>) -> u32>(
        "sa_core::platform::call::create_event_a",
        create_event_a,
    );
    host::set::<fn() -> u32>("sa_core::platform::call::get_last_error", get_last_error);
    host::set::<fn(GPtr<u8>, GPtr<u8>) -> u32>("sa_core::platform::call::find_window_a", find_window_a);
    host::set::<fn(u32) -> i32>(
        "sa_core::platform::call::set_foreground_window_found",
        set_foreground_window,
    );
    host::set::<fn(u32) -> i32>(
        "sa_core::platform::call::set_foreground_window_own",
        set_foreground_window,
    );
    host::set::<fn(u32, GPtr<u8>, u32, u32, GPtr<u32>) -> i32>(
        "sa_core::platform::call::reg_open_key_ex_a",
        reg_open_key_ex_a,
    );
    host::set::<fn(u32, GPtr<u8>, u32, GPtr<u32>, GPtr<u8>, GPtr<u32>) -> i32>(
        "sa_core::platform::call::reg_query_value_ex_a",
        reg_query_value_ex_a,
    );
    host::set::<fn(u32) -> i32>("sa_core::platform::call::reg_close_key_ok", reg_close_key);
    host::set::<fn(u32) -> i32>("sa_core::platform::call::reg_close_key_fail", reg_close_key);
    host::set::<fn(GPtr<u8>, u32, u32, u32, u32, u32, u32) -> u32>(
        "sa_core::platform::call::create_file_a",
        create_file_a,
    );
    host::set::<fn(GPtr<u8>, u32) -> i32>("sa_core::platform::call::create_directory_a", create_directory_a);
    host::set::<fn(u32) -> i32>("sa_core::platform::call::close_handle", close_handle);

    host::set::<fn(u32, u32) -> u32>("sa_core::platform::call::load_icon_a", load_icon_a);
    host::set::<fn(u32, u32) -> u32>("sa_core::platform::call::load_cursor_a", load_cursor_a);
    host::set::<fn(GPtr<[u32; 10]>) -> u32>("sa_core::platform::call::register_class_a", register_class_a);
    host::set::<fn(GPtr<[i32; 4]>, u32, i32) -> i32>(
        "sa_core::platform::call::adjust_window_rect",
        adjust_window_rect,
    );
    host::set::<fn(u32, GPtr<u8>, GPtr<u8>, u32, i32, i32, i32, i32, u32, u32, u32, u32) -> u32>(
        "sa_core::platform::call::create_window_ex_a",
        create_window_ex_a,
    );
    host::set::<fn(i32) -> i32>("sa_core::platform::call::show_cursor", show_cursor);
    host::set::<fn(u32, GPtr<[i32; 4]>) -> i32>("sa_core::platform::call::get_window_rect", get_window_rect);
    host::set::<fn(u32, GPtr<[i32; 4]>) -> i32>(
        "sa_core::platform::call::get_window_rect_grab",
        get_window_rect,
    );
    host::set::<fn(u32, GPtr<u8>) -> i32>("sa_core::platform::call::set_window_text_a", set_window_text_a);
    host::set::<fn(u32, GPtr<[i32; 2]>) -> i32>(
        "sa_core::platform::call::client_to_screen",
        client_to_screen,
    );
    host::set::<fn(i32, i32) -> i32>("sa_core::platform::call::set_cursor_pos", set_cursor_pos);
    host::set::<fn() -> u32>("sa_core::platform::call::get_system_default_lcid", lcid);
    host::set::<fn() -> u32>("sa_core::platform::call::get_user_default_lcid_1", lcid);
    host::set::<fn() -> u32>("sa_core::platform::call::get_user_default_lcid_2", lcid);
    host::set::<fn(u32) -> u32>("sa_core::platform::call::get_keyboard_layout_1", keyboard_layout);
    host::set::<fn(u32) -> u32>("sa_core::platform::call::get_keyboard_layout_2", keyboard_layout);
    host::set::<fn(u32, GPtr<u8>, GPtr<u8>, u32) -> i32>(
        "sa_core::platform::call::message_box_a_error",
        message_box_a,
    );
    host::set::<fn(u32, GPtr<u8>, GPtr<u8>, u32) -> i32>(
        "sa_core::platform::call::message_box_a_warning",
        message_box_a,
    );
    host::set::<fn(GPtr<u8>)>(
        "sa_core::platform::call::output_debug_string_a",
        output_debug_string_a,
    );

    host::set::<fn(GPtr<u8>, u32, u32, u32, u32) -> i32>("sa_core::input::peek_message_a", peek_message_a);
    host::set::<fn(GPtr<u8>) -> i32>("sa_core::input::translate_message", translate_message);
    host::set::<fn(GPtr<u8>) -> i32>("sa_core::input::dispatch_message_a", dispatch_message_a);
    host::set::<fn(i32) -> i32>("sa_core::input::get_key_state", get_key_state);
    host::set::<fn(u32, u32) -> u32>("sa_core::input::map_virtual_key_a", map_virtual_key_a);

    host::set::<fn(GPtr<u8>, GPtr<u32>, GPtr<u32>, GPtr<u32>, GPtr<u32>) -> i32>(
        "sa_stream::cdstream::get_disk_free_space_a",
        get_disk_free_space_a,
    );
    host::set::<fn(GPtr<u8>, u32, u32, u32, u32, u32, u32) -> GPtr<u8>>(
        "sa_stream::cdstream::create_file_a",
        cd_create_file_a,
    );
    host::set::<fn(GPtr<u8>) -> i32>("sa_stream::cdstream::close_handle", cd_close_handle);
    host::set::<fn() -> u32>("sa_stream::cdstream::get_last_error", get_last_error);
    host::set::<fn(u32)>("sa_stream::cdstream::set_last_error", set_last_error);
    host::set::<fn(u32, u32) -> GPtr<u8>>("sa_stream::cdstream::local_alloc", local_alloc);
    host::set::<fn(GPtr<u8>) -> GPtr<u8>>("sa_stream::cdstream::local_free", local_free);
    host::set::<fn(u32, i32, i32, GPtr<u8>) -> GPtr<u8>>(
        "sa_stream::cdstream::create_semaphore_a",
        create_semaphore_a,
    );
    host::set::<fn(GPtr<u8>, i32, GPtr<i32>) -> i32>(
        "sa_stream::cdstream::release_semaphore",
        release_semaphore,
    );
    host::set::<fn(GPtr<u8>, u32) -> u32>(
        "sa_stream::cdstream::wait_for_single_object",
        wait_for_single_object,
    );
    host::set::<fn(GPtr<u8>, u32, i32) -> u32>(
        "sa_stream::cdstream::wait_for_single_object_ex",
        wait_for_single_object_ex,
    );
    host::set::<fn(GPtr<u8>, GPtr<u8>, u32, GPtr<u32>, GPtr<u32>) -> i32>(
        "sa_stream::cdstream::read_file",
        read_file,
    );
    host::set::<fn(GPtr<u8>, GPtr<u32>, GPtr<u32>, i32) -> i32>(
        "sa_stream::cdstream::get_overlapped_result",
        get_overlapped_result,
    );
    host::set::<fn(GPtr<u8>, u32, GPtr<u32>, u32) -> u32>(
        "sa_stream::cdstream::set_file_pointer",
        set_file_pointer,
    );
    host::set::<fn(u32, u32, u32, u32, u32, GPtr<u32>) -> GPtr<u8>>(
        "sa_stream::cdstream::create_thread",
        create_thread,
    );
    host::set::<fn(GPtr<u8>) -> u32>("sa_stream::cdstream::resume_thread", resume_thread);
    host::set::<fn() -> GPtr<u8>>("sa_stream::cdstream::get_current_thread", get_current_thread);
    host::set::<fn(GPtr<u8>) -> i32>("sa_stream::cdstream::get_thread_priority", get_thread_priority);
    host::set::<fn(GPtr<u8>, i32) -> i32>("sa_stream::cdstream::set_thread_priority", set_thread_priority);
    host::set::<fn(u32, u32, u32) -> GPtr<u8>>("sa_stream::cdstream::malloc_align", malloc_align);
    host::set::<fn(GPtr<u8>)>("sa_stream::cdstream::free_align", free_align);

    host::set::<fn(GPtr<ComObj>) -> i32>("sa_input::pad::did_acquire", did_acquire);
    host::set::<fn(GPtr<ComObj>) -> i32>("sa_input::pad::did_poll", did_poll);
    host::set::<fn(GPtr<ComObj>, u32, GPtr<u8>) -> i32>(
        "sa_input::pad::did_get_device_state",
        did_get_device_state,
    );
}
