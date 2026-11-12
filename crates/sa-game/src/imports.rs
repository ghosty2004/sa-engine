//! Native implementations of gta_sa.exe's Win32 imports, keyed by import (`DLL!name`).
//!
//! sa-game writes a trap address into every IAT cell of the arena ([`crate::arena`]) and registers
//! each function below at its import's trap in `orig!`'s address registry, so every `orig!`
//! declaration that calls the import through its IAT cell (`site`, `cell` forms), in any module,
//! reaches it. Declarations bound by path in [`crate::bind`] keep precedence (they are looked up
//! first); most entries here wrap those same functions. Each entry has the import's stdcall
//! signature with 32-bit arguments.

use sa_core::mem::GPtr;

use crate::bind::{self, with};

/// `(import, native entry)`: an `extern "stdcall-unwind"` wrapper per line.
macro_rules! natives {
    ($( $imp:literal => $f:path : fn($($a:ident : $t:ty),*) $(-> $r:ty)? ;)*) => {
        /// `(import, native entry)` of every implemented import.
        pub fn table() -> Vec<(&'static str, usize)> {
            let mut v: Vec<(&'static str, usize)> = Vec::new();
            $({
                #[cfg(target_arch = "x86")]
                extern "stdcall-unwind" fn w($($a: $t),*) $(-> $r)? {
                    $f($($a),*)
                }
                // other targets have no stdcall and no original-ABI dispatch (never called)
                #[cfg(not(target_arch = "x86"))]
                extern "C-unwind" fn w($($a: $t),*) $(-> $r)? {
                    $f($($a),*)
                }
                v.push(($imp, w as *const () as usize));
            })*
            v
        }
    };
}

natives! {
    "KERNEL32.dll!GetCommandLineA" => get_command_line_a: fn() -> GPtr<u8>;
    "KERNEL32.dll!GetModuleHandleA" => get_module_handle_a: fn(name: GPtr<u8>) -> u32;
    "KERNEL32.dll!SetErrorMode" => set_error_mode: fn(mode: u32) -> u32;
    "KERNEL32.dll!GetTickCount" => bind::time_get_time: fn() -> u32;
    "KERNEL32.dll!Sleep" => sleep: fn(ms: u32);
    "KERNEL32.dll!QueryPerformanceFrequency" => bind::qpf: fn(out: GPtr<u64>) -> i32;
    "KERNEL32.dll!QueryPerformanceCounter" => bind::qpc: fn(out: GPtr<u64>) -> i32;
    "KERNEL32.dll!CreateEventA" => bind::create_event_a: fn(attr: u32, manual: i32, initial: i32, name: GPtr<u8>) -> u32;
    "KERNEL32.dll!GetLastError" => bind::get_last_error: fn() -> u32;
    "KERNEL32.dll!SetLastError" => bind::set_last_error: fn(e: u32);
    "KERNEL32.dll!CreateDirectoryA" => bind::create_directory_a: fn(name: GPtr<u8>, sec: u32) -> i32;
    "KERNEL32.dll!CreateFileA" => bind::cd_create_file_a: fn(name: GPtr<u8>, access: u32, share: u32, sec: u32, disp: u32, flags: u32, t: u32) -> GPtr<u8>;
    "KERNEL32.dll!CloseHandle" => bind::cd_close_handle: fn(h: GPtr<u8>) -> i32;
    "KERNEL32.dll!ReadFile" => bind::read_file: fn(h: GPtr<u8>, buf: GPtr<u8>, n: u32, read: GPtr<u32>, ov: GPtr<u32>) -> i32;
    "KERNEL32.dll!GetOverlappedResult" => bind::get_overlapped_result: fn(h: GPtr<u8>, ov: GPtr<u32>, n: GPtr<u32>, wait: i32) -> i32;
    "KERNEL32.dll!SetFilePointer" => bind::set_file_pointer: fn(h: GPtr<u8>, lo: u32, hi: GPtr<u32>, method: u32) -> u32;
    "KERNEL32.dll!OutputDebugStringA" => bind::output_debug_string_a: fn(s: GPtr<u8>);
    "KERNEL32.dll!LocalAlloc" => bind::local_alloc: fn(flags: u32, n: u32) -> GPtr<u8>;
    "KERNEL32.dll!LocalFree" => bind::local_free: fn(p: GPtr<u8>) -> GPtr<u8>;
    "KERNEL32.dll!CreateSemaphoreA" => bind::create_semaphore_a: fn(sec: u32, initial: i32, max: i32, name: GPtr<u8>) -> GPtr<u8>;
    "KERNEL32.dll!ReleaseSemaphore" => bind::release_semaphore: fn(h: GPtr<u8>, n: i32, prev: GPtr<i32>) -> i32;
    "KERNEL32.dll!WaitForSingleObject" => bind::wait_for_single_object: fn(h: GPtr<u8>, ms: u32) -> u32;
    "KERNEL32.dll!WaitForSingleObjectEx" => bind::wait_for_single_object_ex: fn(h: GPtr<u8>, ms: u32, alertable: i32) -> u32;
    "KERNEL32.dll!CreateThread" => bind::create_thread: fn(sec: u32, stack: u32, entry: u32, param: u32, flags: u32, id: GPtr<u32>) -> GPtr<u8>;
    "KERNEL32.dll!ResumeThread" => bind::resume_thread: fn(h: GPtr<u8>) -> u32;
    "KERNEL32.dll!GetCurrentThread" => bind::get_current_thread: fn() -> GPtr<u8>;
    "KERNEL32.dll!GetThreadPriority" => bind::get_thread_priority: fn(h: GPtr<u8>) -> i32;
    "KERNEL32.dll!SetThreadPriority" => bind::set_thread_priority: fn(h: GPtr<u8>, p: i32) -> i32;
    "KERNEL32.dll!GetUserDefaultLCID" => bind::lcid: fn() -> u32;
    "KERNEL32.dll!GetSystemDefaultLCID" => bind::lcid: fn() -> u32;
    "KERNEL32.dll!LoadLibraryA" => crate::dx::load_library_a: fn(name: GPtr<u8>) -> u32;
    "KERNEL32.dll!GetProcAddress" => crate::dx::get_proc_address: fn(module: u32, name: GPtr<u8>) -> u32;
    "KERNEL32.dll!FreeLibrary" => crate::dx::free_library: fn(module: u32) -> i32;
    "ole32.dll!CoInitialize" => crate::dx::co_initialize: fn(reserved: u32) -> i32;
    "ole32.dll!CoCreateInstance" => crate::dx::co_create_instance: fn(clsid: u32, outer: u32, ctx: u32, iid: u32, out: GPtr<u32>) -> i32;
    "KERNEL32.dll!MultiByteToWideChar" => multi_byte_to_wide_char: fn(cp: u32, flags: u32, src: GPtr<u8>, n: i32, dst: GPtr<u16>, wn: i32) -> i32;
    "KERNEL32.dll!GetVersionExA" => get_version_ex_a: fn(info: GPtr<u32>) -> i32;
    "KERNEL32.dll!GlobalMemoryStatus" => global_memory_status: fn(out: GPtr<u32>);
    "USER32.dll!SystemParametersInfoA" => system_parameters_info_a: fn(action: u32, p1: u32, p2: u32, win_ini: u32) -> i32;
    "USER32.dll!ShowWindow" => show_window: fn(hwnd: u32, cmd: i32) -> i32;
    "USER32.dll!UpdateWindow" => update_window: fn(hwnd: u32) -> i32;
    "USER32.dll!SetWindowLongA" => set_window_long_a: fn(hwnd: u32, index: i32, value: i32) -> i32;
    "USER32.dll!SetWindowPos" => set_window_pos: fn(hwnd: u32, after: u32, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
    "USER32.dll!DestroyWindow" => destroy_window: fn(hwnd: u32) -> i32;
    "USER32.dll!GetWindowPlacement" => get_window_placement: fn(hwnd: u32, wp: GPtr<u32>) -> i32;
    "USER32.dll!FindWindowA" => bind::find_window_a: fn(class: GPtr<u8>, name: GPtr<u8>) -> u32;
    "USER32.dll!SetForegroundWindow" => bind::set_foreground_window: fn(hwnd: u32) -> i32;
    "USER32.dll!LoadIconA" => bind::load_icon_a: fn(instance: u32, id: u32) -> u32;
    "USER32.dll!LoadCursorA" => bind::load_cursor_a: fn(instance: u32, id: u32) -> u32;
    "USER32.dll!RegisterClassA" => bind::register_class_a: fn(wc: GPtr<[u32; 10]>) -> u32;
    "USER32.dll!AdjustWindowRect" => bind::adjust_window_rect: fn(rect: GPtr<[i32; 4]>, style: u32, menu: i32) -> i32;
    "USER32.dll!ShowCursor" => bind::show_cursor: fn(show: i32) -> i32;
    "USER32.dll!GetWindowRect" => bind::get_window_rect: fn(hwnd: u32, rect: GPtr<[i32; 4]>) -> i32;
    "USER32.dll!GetClientRect" => bind::get_window_rect: fn(hwnd: u32, rect: GPtr<[i32; 4]>) -> i32;
    "USER32.dll!IsIconic" => is_iconic: fn(hwnd: u32) -> i32;
    "USER32.dll!SetWindowTextA" =>bind::set_window_text_a: fn(hwnd: u32, text: GPtr<u8>) -> i32;
    "USER32.dll!ClientToScreen" => bind::client_to_screen: fn(hwnd: u32, pt: GPtr<[i32; 2]>) -> i32;
    "USER32.dll!SetCursorPos" => bind::set_cursor_pos: fn(x: i32, y: i32) -> i32;
    "USER32.dll!GetKeyboardLayout" => bind::keyboard_layout: fn(thread: u32) -> u32;
    "USER32.dll!MessageBoxA" => bind::message_box_a: fn(hwnd: u32, text: GPtr<u8>, caption: GPtr<u8>, ty: u32) -> i32;
    "USER32.dll!PeekMessageA" => bind::peek_message_a: fn(msg: GPtr<u8>, hwnd: u32, min: u32, max: u32, remove: u32) -> i32;
    "USER32.dll!TranslateMessage" => bind::translate_message: fn(msg: GPtr<u8>) -> i32;
    "USER32.dll!DispatchMessageA" => bind::dispatch_message_a: fn(msg: GPtr<u8>) -> i32;
    "USER32.dll!GetKeyState" => bind::get_key_state: fn(vk: i32) -> i32;
    "USER32.dll!PostQuitMessage" => bind::post_quit_message: fn(code: i32) -> ();
    "USER32.dll!DefWindowProcA" => bind::def_window_proc_a: fn(hwnd: u32, msg: u32, wparam: u32, lparam: u32) -> i32;
    "USER32.dll!SetCapture" => bind::set_capture: fn(hwnd: u32) -> u32;
    "USER32.dll!SetFocus" => bind::set_capture: fn(hwnd: u32) -> u32;
    "USER32.dll!ReleaseCapture" => bind::release_capture: fn() -> i32;
    "USER32.dll!ClipCursor" => bind::clip_cursor: fn(rect: u32) -> i32;
    "USER32.dll!SetCursor" => bind::set_cursor: fn(cursor: u32) -> u32;
    "USER32.dll!SetTimer" => bind::set_timer: fn(hwnd: u32, id: u32, elapse: u32, func: u32) -> u32;
    "USER32.dll!SendMessageA" => bind::send_message_a: fn(hwnd: u32, msg: u32, wparam: u32, lparam: u32) -> i32;
    "USER32.dll!MapVirtualKeyA" => bind::map_virtual_key_a: fn(code: u32, kind: u32) -> u32;
    "WINMM.dll!timeGetTime" => bind::time_get_time: fn() -> u32;
    "WINMM.dll!timeGetDevCaps" => bind::time_get_dev_caps: fn(caps: GPtr<[u32; 2]>, size: u32) -> u32;
    "WINMM.dll!timeBeginPeriod" => bind::time_period: fn(ms: u32) -> u32;
    "WINMM.dll!timeEndPeriod" => bind::time_period: fn(ms: u32) -> u32;
    "ADVAPI32.dll!RegOpenKeyExA" => bind::reg_open_key_ex_a: fn(key: u32, sub: GPtr<u8>, opts: u32, sam: u32, out: GPtr<u32>) -> i32;
    "ADVAPI32.dll!RegQueryValueExA" => bind::reg_query_value_ex_a: fn(key: u32, name: GPtr<u8>, r: u32, ty: GPtr<u32>, data: GPtr<u8>, size: GPtr<u32>) -> i32;
    "ADVAPI32.dll!RegCloseKey" => bind::reg_close_key: fn(key: u32) -> i32;
    "EAX.DLL!#6" => crate::dsound::eax_direct_sound_create: fn(guid: u32, out: GPtr<u32>, outer: u32) -> i32;
    "KERNEL32.dll!lstrlenA" => lstrlen_a: fn(s: GPtr<u8>) -> i32;
    "KERNEL32.dll!lstrcpyA" => lstrcpy_a: fn(d: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8>;
    "KERNEL32.dll!lstrcatA" => lstrcat_a: fn(d: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8>;
    "KERNEL32.dll!GetFileSize" => get_file_size: fn(h: GPtr<u8>, hi: GPtr<u32>) -> u32;
    "KERNEL32.dll!GetLogicalDriveStringsA" => get_logical_drive_strings_a: fn(n: u32, buf: GPtr<u8>) -> u32;
    "KERNEL32.dll!GetDriveTypeA" => get_drive_type_a: fn(root: GPtr<u8>) -> u32;
    "KERNEL32.dll!InitializeCriticalSection" => initialize_critical_section: fn(cs: GPtr<u32>);
    "KERNEL32.dll!DeleteCriticalSection" => delete_critical_section: fn(cs: GPtr<u32>);
    "KERNEL32.dll!EnterCriticalSection" => enter_critical_section: fn(cs: GPtr<u32>);
    "KERNEL32.dll!LeaveCriticalSection" => leave_critical_section: fn(cs: GPtr<u32>);
}

/// The command line `GetCommandLineA` returns: the program name only (no arguments), so
/// `WinMain`'s argument loops (`rsPREINITCOMMANDLINE`, `rsCOMMANDLINE`) see `argc = 1`.
static COMMAND_LINE: &[u8] = b"gta_sa.exe\0";

fn get_command_line_a() -> GPtr<u8> {
    with("GetCommandLineA", |_| ());
    GPtr::from_raw(COMMAND_LINE.as_ptr().cast_mut())
}
/// `SystemParametersInfoA`: WinMain's foreground-lock, screen-saver, power and accessibility-key
/// settings; recorded, success.
fn system_parameters_info_a(_action: u32, _p1: u32, _p2: u32, _win_ini: u32) -> i32 {
    with("SystemParametersInfoA", |_| ());
    1
}
fn show_window(_hwnd: u32, _cmd: i32) -> i32 {
    with("ShowWindow", |_| ());
    0
}
/// `IsIconic`: the native window is never minimised from the game's point of view (the RW camera
/// functions skip a frame for a minimised window). Platform value (STD-4).
fn is_iconic(_hwnd: u32) -> i32 {
    with("IsIconic", |_| ());
    0
}
fn update_window(_hwnd: u32) -> i32 {
    with("UpdateWindow", |_| ());
    1
}
fn set_window_long_a(_hwnd: u32, _index: i32, _value: i32) -> i32 {
    with("SetWindowLongA", |_| ());
    0
}
fn set_window_pos(_hwnd: u32, _after: u32, _x: i32, _y: i32, _cx: i32, _cy: i32, _flags: u32) -> i32 {
    with("SetWindowPos", |_| ());
    1
}
fn destroy_window(_hwnd: u32) -> i32 {
    with("DestroyWindow", |_| ());
    1
}
/// `GetWindowPlacement`: the window is shown normally (`showCmd = SW_SHOWNORMAL`), at the client
/// rectangle of the native window.
fn get_window_placement(_hwnd: u32, wp: GPtr<u32>) -> i32 {
    with("GetWindowPlacement", |_| ());
    let (w, h) = bind::WINDOW_SIZE;
    let v = [0x2C, 0, 1, 0, 0, 0, 0, 0, 0, w as u32, h as u32];
    for (i, x) in v.iter().enumerate() {
        wp.wrapping_index(i as i32).write(*x);
    }
    1
}
/// `MultiByteToWideChar(cp, flags, src, n, dst, wn)`: the ANSI code page as Latin-1 (every byte one
/// UTF-16 unit, what Windows-1252 gives for the ASCII paths the game converts); `n = -1` includes
/// the terminator; `wn = 0` returns the length needed; 0 when `dst` is too short.
fn multi_byte_to_wide_char(_cp: u32, _flags: u32, src: GPtr<u8>, n: i32, dst: GPtr<u16>, wn: i32) -> i32 {
    with("MultiByteToWideChar", |_| ());
    let len = if n < 0 { crate::bind::cstr(src).len() as i32 + 1 } else { n };
    if wn == 0 {
        return len;
    }
    if len > wn {
        return 0;
    }
    for i in 0..len {
        dst.wrapping_index(i).write(src.byte_add(i as isize).read() as u16);
    }
    len
}
/// `GetModuleHandleA(NULL)`: the image base, as Windows returns it for the exe.
fn get_module_handle_a(name: GPtr<u8>) -> u32 {
    with("GetModuleHandleA", |_| ());
    if name.is_null() { 0x0040_0000 } else { 0 }
}
fn set_error_mode(_mode: u32) -> u32 {
    with("SetErrorMode", |_| ());
    0
}
fn sleep(ms: u32) {
    with("Sleep", |_| ());
    crate::det::slept(ms);
    std::thread::sleep(std::time::Duration::from_millis(ms.min(1000) as u64));
}
/// `GetVersionExA(OSVERSIONINFOA*)`: Windows 10 (NT 10.0, build 19045), what the phase A Wine
/// prefix reports by default (Wine's default version); `szCSDVersion` empty.
fn get_version_ex_a(info: GPtr<u32>) -> i32 {
    with("GetVersionExA", |_| ());
    for (i, v) in [10u32, 0, 19045, 2].iter().enumerate() {
        info.wrapping_index(1 + i as i32).write(*v);
    }
    info.byte_add(20).cast::<u8>().write(0);
    1
}
/// `GlobalMemoryStatus(MEMORYSTATUS*)`: this machine's memory, capped at 2 GiB - 1 per field as
/// Windows does for a process without the large-address-aware flag (gta_sa.exe has none).
fn global_memory_status(out: GPtr<u32>) {
    with("GlobalMemoryStatus", |_| ());
    let (total, avail) = meminfo();
    let cap = |v: u64| v.min(0x7FFF_FFFF) as u32;
    let load = (avail * 100).checked_div(total).map_or(0, |p| (100 - p) as u32);
    let v = [
        32,
        load,
        cap(total),
        cap(avail),
        cap(total),
        cap(avail),
        0x7FFE_0000,
        0x7000_0000,
    ];
    for (i, x) in v.iter().enumerate() {
        out.wrapping_index(i as i32).write(*x);
    }
}
/// `(MemTotal, MemAvailable)` in bytes from /proc/meminfo (0 if unreadable).
fn meminfo() -> (u64, u64) {
    let s = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let get = |k: &str| {
        s.lines()
            .find(|l| l.starts_with(k))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
            .map_or(0, |kb| kb * 1024)
    };
    (get("MemTotal:"), get("MemAvailable:"))
}

/// `lstrlenA`: bytes before the terminator (0 for NULL).
fn lstrlen_a(s: GPtr<u8>) -> i32 {
    if s.is_null() {
        return 0;
    }
    let mut n = 0;
    while s.byte_add(n).read() != 0 {
        n += 1;
    }
    n as i32
}
/// `lstrcpyA`: copies with the terminator; returns `d`.
fn lstrcpy_a(d: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> {
    let mut i = 0;
    loop {
        let c = s.byte_add(i).read();
        d.byte_add(i).write(c);
        if c == 0 {
            break;
        }
        i += 1;
    }
    d
}
/// `lstrcatA`: appends `s`; returns `d`.
fn lstrcat_a(d: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> {
    lstrcpy_a(d.byte_add(lstrlen_a(d) as isize), s);
    d
}
/// `GetFileSize(h, &hi)`: the open file's size (seek to the end and back).
fn get_file_size(h: GPtr<u8>, hi: GPtr<u32>) -> u32 {
    let h = h.addr() as u32;
    let size = with("GetFileSize", |p| {
        let pos = p.files.tell(h);
        if p.files.seek(h, 0, 2) != 0 {
            return None;
        }
        let end = p.files.tell(h);
        p.files.seek(h, pos, 0);
        Some(end as u64)
    });
    match size {
        Some(s) => {
            if !hi.is_null() {
                hi.write((s >> 32) as u32);
            }
            s as u32
        }
        None => 0xFFFF_FFFF,
    }
}
/// `GetLogicalDriveStringsA`: no drives (an empty, double-terminated list). Phase B has no drive
/// letters; `getDvdGamePath` therefore finds no DVD.
fn get_logical_drive_strings_a(n: u32, buf: GPtr<u8>) -> u32 {
    with("GetLogicalDriveStringsA", |_| ());
    if n >= 1 && !buf.is_null() {
        buf.write(0);
    }
    0
}
/// `GetDriveTypeA`: `DRIVE_NO_ROOT_DIR` (1).
fn get_drive_type_a(_root: GPtr<u8>) -> u32 {
    1
}

/// Critical sections, keyed by the `CRITICAL_SECTION` address: `(owner thread, recursion count)`.
type CsMap = std::collections::HashMap<u32, (u64, u32)>;
static CRITICAL: std::sync::Mutex<Option<CsMap>> = std::sync::Mutex::new(None);
static CRITICAL_CV: std::sync::Condvar = std::sync::Condvar::new();

fn thread_key() -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    std::thread::current().id().hash(&mut h);
    h.finish() | 1
}
/// `InitializeCriticalSection`: the 0x18-byte structure is zeroed (the game never reads it);
/// the lock state lives in sa-game, keyed by the address.
fn initialize_critical_section(cs: GPtr<u32>) {
    for i in 0..6 {
        cs.wrapping_index(i).write(0);
    }
    let mut m = CRITICAL.lock().unwrap_or_else(|e| e.into_inner());
    m.get_or_insert_with(Default::default)
        .insert(cs.addr() as u32, (0, 0));
}
fn delete_critical_section(cs: GPtr<u32>) {
    let mut m = CRITICAL.lock().unwrap_or_else(|e| e.into_inner());
    m.get_or_insert_with(Default::default).remove(&(cs.addr() as u32));
}
/// `EnterCriticalSection`: recursive per thread, blocks while another thread owns it.
fn enter_critical_section(cs: GPtr<u32>) {
    let me = thread_key();
    let key = cs.addr() as u32;
    let mut m = CRITICAL.lock().unwrap_or_else(|e| e.into_inner());
    loop {
        let e = m
            .get_or_insert_with(Default::default)
            .entry(key)
            .or_insert((0, 0));
        if e.1 == 0 || e.0 == me {
            e.0 = me;
            e.1 += 1;
            return;
        }
        m = CRITICAL_CV.wait(m).unwrap_or_else(|e| e.into_inner());
    }
}
fn leave_critical_section(cs: GPtr<u32>) {
    let key = cs.addr() as u32;
    let mut m = CRITICAL.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(e) = m.get_or_insert_with(Default::default).get_mut(&key)
        && e.1 > 0
    {
        e.1 -= 1;
        if e.1 == 0 {
            e.0 = 0;
            CRITICAL_CV.notify_all();
        }
    }
}
