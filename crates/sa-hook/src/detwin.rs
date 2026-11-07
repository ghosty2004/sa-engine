//! Windows side of the oracle features (INF-4/5/6). See `oracle.rs` for the configuration and
//! `docs/re/determinism.md` for every nondeterminism source and how it is handled.
//!
//! - Determinism mode patches the game's own time call sites (never the IAT, so Windows/Wine and the
//!   game's other threads keep real time) to a virtual clock that advances by a fixed step at every
//!   frame boundary, makes the WinMain frame limiter never wait, and turns `CdStreamRead` into a
//!   synchronous read on the calling thread.
//! - rand/srand are logged with their caller: the main thread's calls feed the `randcalls` hash,
//!   other threads' calls are counted per (thread, caller).
//! - `CPad::UpdateMouse` / `CPad::Update(0)` capture and inject the per-tick pad input.
//! - At each frame boundary the frame-state hashes are computed and written to the trace.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::ffi::c_void;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use sa_oracle_format::fnv;
use sa_oracle_format::pad::{self, PadInput};
use sa_oracle_format::program::Program;
use sa_oracle_format::recording::Recording;
use sa_oracle_format::subsys::{self, COUNT, Extra, Mem, Subsystem};
use sa_oracle_format::trace;

use windows_sys::Win32::Foundation::{
    ERROR_IO_PENDING, FILETIME, GetLastError, HANDLE, INVALID_HANDLE_VALUE, SYSTEMTIME, SetLastError,
};
use windows_sys::Win32::Storage::FileSystem::{FILE_BEGIN, ReadFile, SetFilePointer};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
};
use windows_sys::Win32::System::IO::{GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows_sys::Win32::System::SystemInformation::{GetLocalTime, GetSystemTimeAsFileTime, GetTickCount};
use windows_sys::Win32::System::Threading::{ExitProcess, GetCurrentProcessId, GetCurrentThreadId};
use windows_sys::Win32::System::Time::FileTimeToSystemTime;

use crate::arena::Arena;
use crate::oracle::{Clock, Config, InputState, Source, rand_seq_step};
use crate::table::{self, Id};
use crate::win::{self, log};
use crate::x86;

// ------------------------------------------------------------------ state ---------------------

static CFG: OnceLock<Config> = OnceLock::new();
static ENABLED: AtomicBool = AtomicBool::new(false);
static DET: AtomicBool = AtomicBool::new(false);
static MAIN_TID: AtomicU32 = AtomicU32::new(0);

static QPF: AtomicU64 = AtomicU64::new(1);
static STEP_TICKS: AtomicU64 = AtomicU64::new(0);
/// Virtual QueryPerformanceCounter value seen by the main thread.
static NOW: AtomicU64 = AtomicU64::new(0);

static INPUT: Mutex<Option<InputState>> = Mutex::new(None);
static CAPTURE: Mutex<Option<PadInput>> = Mutex::new(None);
static RECORD_PATH: Mutex<Option<String>> = Mutex::new(None);
static RECORD_START: Mutex<String> = Mutex::new(String::new());

static RAND_SEQ: AtomicU64 = AtomicU64::new(fnv::OFFSET);
static RAND_MAIN: AtomicU32 = AtomicU32::new(0);
static SRAND_MAIN: AtomicU32 = AtomicU32::new(0);
/// (thread id, kind 0=rand 1=srand, caller) → calls, for threads other than the main thread.
static RAND_OTHER: Mutex<BTreeMap<(u32, u8, u32), u32>> = Mutex::new(BTreeMap::new());
static RANDLOG_ON: AtomicBool = AtomicBool::new(false);
static RANDLOG: Mutex<Option<BufWriter<File>>> = Mutex::new(None);

static TRACE: Mutex<Option<BufWriter<File>>> = Mutex::new(None);
static HASHES: Mutex<Vec<(u32, u32, [u64; COUNT])>> = Mutex::new(Vec::new());
static DONE: AtomicBool = AtomicBool::new(false);
static POOLS_LOGGED: AtomicBool = AtomicBool::new(false);
static PERTURBED: Mutex<String> = Mutex::new(String::new());

static SYNC_READS: AtomicU32 = AtomicU32::new(0);
static OFFTHREAD_TIME: AtomicU32 = AtomicU32::new(0);
static PAGES: Mutex<Option<HashSet<u32>>> = Mutex::new(None);
static IMAGE: Mutex<(u32, u32)> = Mutex::new((0, 0));

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Acquire)
}

pub(crate) fn det_on() -> bool {
    DET.load(Ordering::Acquire)
}

pub(crate) fn virtual_now() -> u64 {
    NOW.load(Ordering::Acquire)
}

pub(crate) fn qpf() -> u64 {
    QPF.load(Ordering::Acquire)
}

/// The game's main thread as known so far (guessed at attach, corrected at frame 1); 0 = unknown.
pub(crate) fn main_tid() -> u32 {
    MAIN_TID.load(Ordering::Acquire)
}

fn is_main() -> bool {
    // SAFETY: plain Win32 call.
    unsafe { GetCurrentThreadId() == MAIN_TID.load(Ordering::Relaxed) }
}

fn frame() -> u32 {
    win::frames()
}

// ------------------------------------------------------------------ raw game memory -----------

fn put(addr: u32, bytes: &[u8]) {
    // SAFETY: only called for fixed .data/.bss addresses of the exe (CPad blocks), always mapped RW.
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), addr as usize as *mut u8, bytes.len()) }
}

fn get(addr: u32, out: &mut [u8]) {
    // SAFETY: as `put`.
    unsafe { std::ptr::copy_nonoverlapping(addr as usize as *const u8, out.as_mut_ptr(), out.len()) }
}

fn get_u32(addr: u32) -> u32 {
    let mut b = [0u8; 4];
    get(addr, &mut b);
    u32::from_le_bytes(b)
}

/// The hasher's view of the process: the exe image is always readable; other pages are checked
/// once with VirtualQuery and remembered.
struct GameMem<'a> {
    image: (u32, u32),
    pages: RefCell<&'a mut HashSet<u32>>,
}

impl Mem for GameMem<'_> {
    fn read(&self, addr: u32, out: &mut [u8]) -> bool {
        if out.is_empty() {
            return true;
        }
        let end = addr as u64 + out.len() as u64;
        if addr < 0x10000 || end > 0x8000_0000 {
            return false;
        }
        if !(addr >= self.image.0 && end <= self.image.1 as u64) {
            let mut pages = self.pages.borrow_mut();
            for page in (addr >> 12)..=((end - 1) >> 12) as u32 {
                if !pages.contains(&page) {
                    if !win::readable(page << 12, 0x1000) {
                        return false;
                    }
                    pages.insert(page);
                }
            }
        }
        get(addr, out);
        true
    }
}

// ------------------------------------------------------------------ init ----------------------

fn main_thread_guess() -> (u32, Vec<u32>) {
    let mut ids = Vec::new();
    // SAFETY: toolhelp snapshot of our own process' threads.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snap != INVALID_HANDLE_VALUE {
            let mut te: THREADENTRY32 = std::mem::zeroed();
            te.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
            let pid = GetCurrentProcessId();
            if Thread32First(snap, &mut te) != 0 {
                loop {
                    if te.th32OwnerProcessID == pid {
                        ids.push(te.th32ThreadID);
                    }
                    if Thread32Next(snap, &mut te) == 0 {
                        break;
                    }
                }
            }
            windows_sys::Win32::Foundation::CloseHandle(snap);
        }
        let me = GetCurrentThreadId();
        let main = ids.iter().copied().find(|&t| t != me).unwrap_or(0);
        (main, ids)
    }
}

/// Reads the environment, prepares tracing/recording/replay and, in determinism mode, patches the
/// time call sites. Runs in DllMain while the game's main thread is still suspended. Returns whether
/// the Oracle hook group must be installed.
pub fn init(dir: &std::path::Path) -> bool {
    let cfg = match Config::from_env(|k| std::env::var(k).ok()) {
        Ok(c) => c,
        Err(e) => {
            log(&format!("oracle: bad configuration, oracle features off: {e}"));
            return false;
        }
    };
    if !cfg.enabled() {
        return false;
    }
    let (main, all) = main_thread_guess();
    MAIN_TID.store(main, Ordering::Release);
    log(&format!("oracle: config {cfg:?}"));
    log(&format!("oracle: threads at attach {all:?}, main thread {main}"));

    // Exe image range for the hasher's fast path.
    let base = 0x400000u32;
    let size = get_u32(base + get_u32(base + 0x3C) + 0x50);
    if win::readable(base, size as usize) {
        *IMAGE.lock().unwrap_or_else(|e| e.into_inner()) = (base, base + size);
    }
    log(&format!("oracle: image {base:#x}..{:#x}", base + size));
    *PAGES.lock().unwrap_or_else(|e| e.into_inner()) = Some(HashSet::new());

    let mut qpf = 0i64;
    // SAFETY: plain Win32 call.
    unsafe { QueryPerformanceFrequency(&mut qpf) };
    let clock = Clock::new(qpf as u64, cfg.step_us, cfg.clock_base);
    QPF.store(clock.qpf, Ordering::Release);
    STEP_TICKS.store(clock.step_ticks, Ordering::Release);
    NOW.store(clock.now, Ordering::Release);

    if cfg.hash {
        let path = cfg
            .trace
            .clone()
            .unwrap_or_else(|| dir.join("sa-trace.bin").to_string_lossy().into_owned());
        match File::create(&path) {
            Ok(f) => {
                let mut w = BufWriter::with_capacity(1 << 16, f);
                let _ = w.write_all(&trace::header());
                *TRACE.lock().unwrap_or_else(|e| e.into_inner()) = Some(w);
                log(&format!("oracle: trace {path}"));
            }
            Err(e) => log(&format!("oracle: trace {path}: {e}")),
        }
    }
    if let Some(p) = &cfg.randlog {
        match File::create(p) {
            Ok(f) => {
                *RANDLOG.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(BufWriter::with_capacity(1 << 20, f));
                RANDLOG_ON.store(true, Ordering::Release);
            }
            Err(e) => log(&format!("oracle: randlog {p}: {e}")),
        }
    }

    if let Ok(p) = std::env::var("SA_DET_HEAPLOG")
        && !p.is_empty()
    {
        match File::create(&p) {
            Ok(f) => {
                *HEAPLOG.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(BufWriter::with_capacity(1 << 20, f));
                HEAPLOG_ON.store(true, Ordering::Release);
            }
            Err(e) => log(&format!("oracle: heaplog {p}: {e}")),
        }
    }

    // Input source from process start.
    let source = if let Some(p) = &cfg.program {
        match std::fs::read_to_string(p)
            .map_err(|e| e.to_string())
            .and_then(|s| Program::parse(&s))
        {
            Ok(prog) => {
                log(&format!(
                    "oracle: program {p} end={:?} fnv={:016x}",
                    prog.end, prog.fnv
                ));
                Some(Source::Program(prog))
            }
            Err(e) => {
                log(&format!("oracle: program {p}: {e}"));
                None
            }
        }
    } else if let Some(p) = &cfg.replay {
        match std::fs::read(p)
            .map_err(|e| e.to_string())
            .and_then(|b| Recording::parse(&b))
        {
            Ok(rec) => {
                log(&format!(
                    "oracle: replay {p}: {} ticks, header {:?}",
                    rec.ticks.len(),
                    rec.header.fields
                ));
                Some(Source::Replay { rec, pos: 0 })
            }
            Err(e) => {
                log(&format!("oracle: replay {p}: {e}"));
                None
            }
        }
    } else {
        None
    };
    let mut cfg = cfg;
    if cfg.end_frame.is_none()
        && let Some(Source::Program(p)) = &source
    {
        cfg.end_frame = p.end;
    }
    if cfg.end_frame.is_none()
        && let Some(Source::Replay { rec, .. }) = &source
    {
        cfg.end_frame = rec.header.get("frames").and_then(|f| f.parse().ok());
    }
    let record = cfg.record.is_some();
    if source.is_some() || record {
        *INPUT.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(InputState::new(source.unwrap_or(Source::Live), record));
        *RECORD_PATH.lock().unwrap_or_else(|e| e.into_inner()) = cfg.record.clone();
        *RECORD_START.lock().unwrap_or_else(|e| e.into_inner()) = "boot".into();
    }

    if cfg.det {
        match arena_init() {
            Ok(base) => log(&format!(
                "oracle: deterministic heap arena at {base:#x}..{:#x} for main-thread CRT allocations",
                base as u64 + ARENA_LEN as u64
            )),
            Err(e) => log(&format!("oracle: deterministic heap OFF: {e}")),
        }
        match patch_time_sites() {
            Ok(n) => {
                DET.store(true, Ordering::Release);
                log(&format!(
                    "oracle: determinism mode on: {n} time sites patched, qpf {}, step {} ticks, base {}",
                    clock.qpf, clock.step_ticks, clock.now
                ));
            }
            Err(e) => log(&format!(
                "oracle: determinism mode OFF, time site patch failed: {e}"
            )),
        }
    }
    let _ = CFG.set(cfg);
    ENABLED.store(true, Ordering::Release);
    true
}

// ------------------------------------------------------------------ virtual time --------------

extern "system" fn det_qpc(out: *mut i64) -> i32 {
    if is_main() {
        // SAFETY: the caller passes a LARGE_INTEGER*.
        unsafe { *out = NOW.load(Ordering::Acquire) as i64 };
        1
    } else {
        OFFTHREAD_TIME.fetch_add(1, Ordering::Relaxed);
        // SAFETY: as above.
        unsafe { QueryPerformanceCounter(out) }
    }
}

fn clock_now() -> Clock {
    Clock {
        qpf: QPF.load(Ordering::Acquire),
        step_ticks: STEP_TICKS.load(Ordering::Acquire),
        now: NOW.load(Ordering::Acquire),
    }
}

extern "system" fn det_time_get_time() -> u32 {
    if is_main() {
        clock_now().ms()
    } else {
        OFFTHREAD_TIME.fetch_add(1, Ordering::Relaxed);
        // SAFETY: plain Win32 call.
        unsafe { GetTickCount() }
    }
}

extern "system" fn det_get_system_time_as_file_time(out: *mut FILETIME) {
    if is_main() {
        let ft = clock_now().filetime();
        // SAFETY: the caller passes a FILETIME*.
        unsafe {
            *out = FILETIME {
                dwLowDateTime: ft as u32,
                dwHighDateTime: (ft >> 32) as u32,
            }
        };
    } else {
        OFFTHREAD_TIME.fetch_add(1, Ordering::Relaxed);
        // SAFETY: as above.
        unsafe { GetSystemTimeAsFileTime(out) }
    }
}

extern "system" fn det_get_local_time(out: *mut SYSTEMTIME) {
    if is_main() {
        let ft = clock_now().filetime();
        let f = FILETIME {
            dwLowDateTime: ft as u32,
            dwHighDateTime: (ft >> 32) as u32,
        };
        // SAFETY: the caller passes a SYSTEMTIME*. The virtual local time zone is UTC.
        unsafe { FileTimeToSystemTime(&f, out) };
    } else {
        OFFTHREAD_TIME.fetch_add(1, Ordering::Relaxed);
        // SAFETY: as above.
        unsafe { GetLocalTime(out) }
    }
}

/// Replaces `CTimer::GetCurrentTimeInCycles()` at the WinMain frame-limiter check (0x748D1F): the
/// limiter never waits, frames are paced by the virtual clock.
extern "C" fn det_limiter_elapsed() -> u32 {
    0x7FFF_FFFF
}

/// Pointer cells the patched `call dword ptr [cell]` sites jump through.
static CELL_QPC: AtomicU32 = AtomicU32::new(0);
static CELL_TGT: AtomicU32 = AtomicU32::new(0);
static CELL_FILETIME: AtomicU32 = AtomicU32::new(0);
static CELL_LOCALTIME: AtomicU32 = AtomicU32::new(0);

/// Every game call site of a time source on the main thread (IAT cross-references of gta_sa.exe 1.0
/// US, docs/re/determinism.md §1). Audio-thread sites (CAEAudioUtility::GetCurrentTimeInMS 0x4D9E88)
/// and the CRT start-up cookie (0x8339CA) keep real time.
fn patch_time_sites() -> Result<usize, String> {
    CELL_QPC.store(det_qpc as *const () as usize as u32, Ordering::Release);
    CELL_TGT.store(det_time_get_time as *const () as usize as u32, Ordering::Release);
    CELL_FILETIME.store(
        det_get_system_time_as_file_time as *const () as usize as u32,
        Ordering::Release,
    );
    CELL_LOCALTIME.store(det_get_local_time as *const () as usize as u32, Ordering::Release);
    let cell = |c: &AtomicU32| c as *const AtomicU32 as usize as u32;
    // (site, expected bytes, new disp32 for `FF 15` sites / call target for `E8`)
    const QPC: [u8; 6] = [0xFF, 0x15, 0xC8, 0x80, 0x85, 0x00];
    const TGT: [u8; 6] = [0xFF, 0x15, 0x10, 0x83, 0x85, 0x00];
    const FT: [u8; 6] = [0xFF, 0x15, 0x00, 0x82, 0x85, 0x00];
    const LT: [u8; 6] = [0xFF, 0x15, 0x1C, 0x81, 0x85, 0x00];
    let iat_sites: [(u32, &[u8; 6], u32, &str); 7] = [
        (
            0x5617A8,
            &QPC,
            cell(&CELL_QPC),
            "CTimer timer function 0x5617A0 (QPC)",
        ),
        (
            0x561A88,
            &QPC,
            cell(&CELL_QPC),
            "CTimer::GetCurrentTimeInCycles (QPC)",
        ),
        (
            0x5900FF,
            &QPC,
            cell(&CELL_QPC),
            "CLoadingScreen::LoadSplashes srand seed (QPC)",
        ),
        (0x74528C, &TGT, cell(&CELL_TGT), "psTimer/RsTimer (timeGetTime)"),
        (
            0x82410F,
            &FT,
            cell(&CELL_FILETIME),
            "CRT time() (GetSystemTimeAsFileTime)",
        ),
        (
            0x5D1E54,
            &LT,
            cell(&CELL_LOCALTIME),
            "CSimpleVariablesSaveStructure::Construct (GetLocalTime)",
        ),
        (
            0x823A9B,
            &LT,
            cell(&CELL_LOCALTIME),
            "CRT local time helper 0x823A8E (GetLocalTime)",
        ),
    ];
    // `call CTimer::GetCurrentTimeInCycles` (E8 rel32 → 0x561A80) sites of the two main-thread
    // busy-wait loops, retargeted to constant answers that end the wait at once.
    let wait_sites: [(u32, [u8; 5], u32, &str); 3] = [
        (
            0x748D1F,
            [0xE8, 0x5C, 0x8D, 0xE1, 0xFF],
            det_limiter_elapsed as *const () as usize as u32,
            "WinMain frame limiter (idle when elapsed > 1000/frameLimit) -> never waits",
        ),
        (
            0x53E930,
            [0xE8, 0x4B, 0x31, 0x02, 0x00],
            det_limiter_elapsed as *const () as usize as u32,
            "Game::Idle busy-wait (elapsed - g_LastTickTime >= 14) -> never waits",
        ),
        (
            0x53E94F,
            [0xE8, 0x2C, 0x31, 0x02, 0x00],
            det_idle_last_tick as *const () as usize as u32,
            "Game::Idle g_LastTickTime (0xB72CA8, read only by this loop) -> constant 0",
        ),
    ];
    for (site, expect, _, what) in &iat_sites {
        let cur = win::read(*site, 6).ok_or_else(|| format!("{site:#x} not readable"))?;
        if cur != expect[..] {
            return Err(format!("{site:#x} ({what}): unexpected bytes {cur:02x?}"));
        }
    }
    for (site, expect, _, what) in &wait_sites {
        let cur = win::read(*site, 5).ok_or_else(|| format!("{site:#x} not readable"))?;
        if cur != expect[..] {
            return Err(format!("{site:#x} ({what}): unexpected bytes {cur:02x?}"));
        }
    }
    for (site, _, disp, what) in &iat_sites {
        win::patch(site + 2, &disp.to_le_bytes())?;
        log(&format!("oracle: time site {site:#x} {what} -> virtual"));
    }
    for (site, _, target, what) in &wait_sites {
        let rel = x86::rel32_to(*site, 5, *target);
        win::patch(site + 1, &rel.to_le_bytes())?;
        log(&format!("oracle: wait site {site:#x} {what}"));
    }
    crate::detaudio::patch_sites(cell(&CELL_QPC))?;
    Ok(iat_sites.len() + wait_sites.len() + 3)
}

/// Replaces the post-loop `GetCurrentTimeInCycles()` of `Game::Idle` (0x53E94F), whose quotient
/// becomes `g_LastTickTime`: with 0 the busy-wait condition stays satisfied every frame.
extern "C" fn det_idle_last_tick() -> u32 {
    0
}

// ------------------------------------------------------------------ CdStream ------------------

const CD_STREAMS: u32 = 0x8E3FFC;
const CD_STREAMING_INITIALIZED: u32 = 0x8E3FE4;
const CD_OVERLAPPED_IO: u32 = 0x8E3FE8;
const CD_FILE_HANDLES: u32 = 0x8E4010;
const CD_LAST_POSN: u32 = 0x8E4898;

/// `CdStreamRead(int32 stream, void* buf, CdStreamPos pos, int32 sectors)` (cdecl, 0x406A20 →
/// .HOODLUM 0x156C2C0). Determinism mode: the read that `CdStreamThread` (0x406560) would do later is
/// done now, on the calling thread, and the stream is left in the state the thread leaves it in
/// (`nSectorsToRead = 0`, `bInUse = 0`, status 0 or 0xFE). Everything before the queueing is the
/// original's (0x156C2C0..0x156C344).
pub extern "C" fn cd_stream_read_hook(stream: i32, buf: *mut c_void, pos: u32, sectors: i32) -> u32 {
    table::count(Id::CdStreamRead);
    // SAFETY: the old stub target has this signature.
    let orig: extern "C" fn(i32, *mut c_void, u32, i32) -> u32 =
        unsafe { std::mem::transmute(table::original(Id::CdStreamRead) as usize) };
    if !DET.load(Ordering::Acquire) || get_u32(CD_STREAMING_INITIALIZED) == 0 {
        return orig(stream, buf, pos, sectors);
    }
    let s = get_u32(CD_STREAMS).wrapping_add((stream as u32).wrapping_mul(0x30));
    put(CD_LAST_POSN, &(sectors as u32).wrapping_add(pos).to_le_bytes());
    let h = get_u32(CD_FILE_HANDLES + (pos >> 24) * 4);
    put(s + 0x18, &h.to_le_bytes());
    // SAFETY: plain Win32 calls on the game's own handles/buffers.
    unsafe {
        SetLastError(0);
        let mut b = [0u8; 1];
        get(s + 0xE, &mut b);
        if get_u32(s + 4) != 0 || b[0] != 0 {
            return 0;
        }
        let off = (pos & 0x00FF_FFFF) << 11;
        let len = (sectors as u32) << 11;
        put(s + 0x10, &0u32.to_le_bytes());
        put(s, &(pos & 0x00FF_FFFF).to_le_bytes());
        put(s + 4, &(sectors as u32).to_le_bytes());
        put(s + 8, &(buf as usize as u32).to_le_bytes());
        put(s + 0xD, &[0]);
        let hfile = h as usize as HANDLE;
        let status: u32 = if get_u32(CD_OVERLAPPED_IO) != 0 {
            let ov = (s + 0x1C) as usize as *mut OVERLAPPED;
            put(s + 0x24, &off.to_le_bytes());
            if ReadFile(hfile, buf as *mut u8, len, std::ptr::null_mut(), ov) == 1 {
                0
            } else if GetLastError() != ERROR_IO_PENDING {
                0xFE
            } else {
                let mut n = 0u32;
                if GetOverlappedResult(hfile, ov, &mut n, 1) == 1 {
                    0
                } else {
                    0xFE
                }
            }
        } else {
            SetFilePointer(hfile, off as i32, std::ptr::null_mut(), FILE_BEGIN);
            let mut n = 0u32;
            if ReadFile(hfile, buf as *mut u8, len, &mut n, std::ptr::null_mut()) == 1 {
                0
            } else {
                0xFE
            }
        };
        put(s + 0x10, &status.to_le_bytes());
        put(s + 4, &0u32.to_le_bytes());
    }
    SYNC_READS.fetch_add(1, Ordering::Relaxed);
    1
}

// ------------------------------------------------------------------ heap ----------------------

static ARENA: Mutex<Option<Arena>> = Mutex::new(None);
static HEAP_ON: AtomicBool = AtomicBool::new(false);
/// Set when the CRT start-up reaches `_cinit` (static constructors): from then on the main thread
/// allocates from the arena. Everything before (I/O tables, the environment block and argv, whose
/// sizes depend on the environment variables of the run) stays in the CRT heap.
static HEAP_ACTIVE: AtomicBool = AtomicBool::new(false);
static ARENA_BASE: AtomicU32 = AtomicU32::new(0);
static ARENA_END: AtomicU32 = AtomicU32::new(0);
/// Arena blocks freed/reallocated by a thread other than the main thread (perturbs the arena).
static HEAP_XTHREAD: AtomicU32 = AtomicU32::new(0);
/// Main-thread allocations served by the CRT heap because the arena was exhausted.
static HEAP_OOM: AtomicU32 = AtomicU32::new(0);
/// Other threads' allocations (CRT heap, not deterministic, counted for the record).
static HEAP_OTHER_ALLOCS: AtomicU32 = AtomicU32::new(0);
static HEAP_BAD_FREE: AtomicU32 = AtomicU32::new(0);

/// Candidate fixed bases for the arena (first free one wins; the same one every run as long as the
/// address-space layout at attach is the same, which the log records).
const ARENA_BASES: [u32; 3] = [0x2000_0000, 0x3000_0000, 0x1800_0000];
const ARENA_LEN: u32 = 0x3000_0000;

fn arena_init() -> Result<u32, String> {
    use windows_sys::Win32::System::Memory::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc};
    for base in ARENA_BASES {
        // SAFETY: reserves and commits a fresh region at a fixed address (fails if anything is there).
        let p = unsafe {
            VirtualAlloc(
                base as usize as *const c_void,
                ARENA_LEN as usize,
                MEM_RESERVE | MEM_COMMIT,
                PAGE_READWRITE,
            )
        } as usize as u32;
        if p == base {
            *ARENA.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arena::new(base, ARENA_LEN));
            ARENA_BASE.store(base, Ordering::Release);
            ARENA_END.store(base + ARENA_LEN, Ordering::Release);
            HEAP_ON.store(true, Ordering::Release);
            // mainCRTStartup's `call _cinit` (0x8246AC → 0x823B76) goes through the activation shim.
            const SITE: u32 = 0x8246AC;
            if win::read(SITE, 5).as_deref() != Some(&[0xE8, 0xC5, 0xF4, 0xFF, 0xFF][..]) {
                HEAP_ON.store(false, Ordering::Release);
                return Err(format!("{SITE:#x}: unexpected bytes (call _cinit)"));
            }
            let rel = x86::rel32_to(SITE, 5, cinit_shim as *const () as usize as u32);
            win::patch(SITE + 1, &rel.to_le_bytes())?;
            return Ok(base);
        }
    }
    Err("no fixed arena base available".into())
}

extern "C" fn heap_activate() {
    HEAP_ACTIVE.store(true, Ordering::Release);
    if DET.load(Ordering::Acquire) {
        match crate::detaudio::virtualise_dsound() {
            Ok(s) => log(&format!("oracle: DirectSound {s}")),
            Err(e) => log(&format!("oracle: DirectSound virtualisation FAILED: {e}")),
        }
    }
}

/// Replaces mainCRTStartup's call of `_cinit` (0x823B76): activates the arena, then tail-jumps into
/// `_cinit` with the stack exactly as the original call left it.
#[unsafe(naked)]
unsafe extern "C" fn cinit_shim() {
    std::arch::naked_asm!(
        "call {act}",
        "mov eax, 0x823B76",
        "jmp eax",
        act = sym heap_activate,
    )
}

fn in_arena(p: u32) -> bool {
    p >= ARENA_BASE.load(Ordering::Relaxed) && p < ARENA_END.load(Ordering::Relaxed)
}

fn arena<T>(f: impl FnOnce(&mut Arena) -> T) -> Option<T> {
    ARENA.lock().ok().and_then(|mut g| g.as_mut().map(f))
}

/// `__heap_alloc(size)` (cdecl, 0x8241B0): reached by malloc, `_nh_malloc` and operator new.
pub extern "C" fn heap_alloc_hook(size: u32) -> u32 {
    table::count(Id::HeapAlloc);
    if HEAP_ACTIVE.load(Ordering::Acquire) {
        if is_main() {
            l2_heap_touch();
            if let Some(Some(p)) = arena(|a| a.alloc(size)) {
                if HEAPLOG_ON.load(Ordering::Relaxed) {
                    heaplog('a', size, p);
                }
                crate::l2::note_alloc(p, size);
                return p;
            }
            HEAP_OOM.fetch_add(1, Ordering::Relaxed);
        } else {
            HEAP_OTHER_ALLOCS.fetch_add(1, Ordering::Relaxed);
        }
    }
    // SAFETY: the trampoline has this signature.
    let orig: extern "C" fn(u32) -> u32 =
        unsafe { std::mem::transmute(table::original(Id::HeapAlloc) as usize) };
    orig(size)
}

/// `free(p)` (cdecl, 0x82413F).
pub extern "C" fn free_hook(p: u32) {
    table::count(Id::Free);
    if p != 0 && in_arena(p) {
        if !is_main() {
            HEAP_XTHREAD.fetch_add(1, Ordering::Relaxed);
        }
        l2_heap_touch();
        if arena(|a| a.free(p)) != Some(true) {
            HEAP_BAD_FREE.fetch_add(1, Ordering::Relaxed);
        }
        return;
    }
    // SAFETY: the trampoline has this signature.
    let orig: extern "C" fn(u32) = unsafe { std::mem::transmute(table::original(Id::Free) as usize) };
    orig(p)
}

/// `realloc(p, size)` (cdecl, 0x824269); VC7.1 semantics: realloc(0, n) = malloc(n), realloc(p, 0) =
/// free(p) and NULL.
pub extern "C" fn realloc_hook(p: u32, size: u32) -> u32 {
    table::count(Id::Realloc);
    if HEAP_ACTIVE.load(Ordering::Acquire) && (in_arena(p) || (p == 0 && is_main())) {
        if p == 0 {
            return heap_alloc_hook(size);
        }
        if !is_main() {
            HEAP_XTHREAD.fetch_add(1, Ordering::Relaxed);
        }
        if size == 0 {
            free_hook(p);
            return 0;
        }
        l2_heap_touch();
        if arena(|a| a.resize_in_place(p, size)) == Some(true) {
            return p;
        }
        let old = arena(|a| a.block(p).map(|b| b.size)).flatten().unwrap_or(0);
        let q = heap_alloc_hook(size);
        if q != 0 {
            // SAFETY: both blocks are live; the copy stays inside both.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    p as usize as *const u8,
                    q as usize as *mut u8,
                    old.min(size) as usize,
                )
            };
            free_hook(p);
        }
        return q;
    }
    // SAFETY: the trampoline has this signature.
    let orig: extern "C" fn(u32, u32) -> u32 =
        unsafe { std::mem::transmute(table::original(Id::Realloc) as usize) };
    orig(p, size)
}

/// `calloc(n, size)` (cdecl, 0x824416): `n * size` without overflow check (imul at 0x824425), 0 → 1.
pub extern "C" fn calloc_hook(n: u32, size: u32) -> u32 {
    table::count(Id::Calloc);
    if HEAP_ACTIVE.load(Ordering::Acquire) && is_main() {
        let total = n.wrapping_mul(size);
        l2_heap_touch();
        if let Some(Some(p)) = arena(|a| a.alloc(total)) {
            crate::l2::note_alloc(p, total);
            // SAFETY: fresh block of at least `max(total, 1)` bytes.
            unsafe { std::ptr::write_bytes(p as usize as *mut u8, 0, total.max(1) as usize) };
            return p;
        }
        HEAP_OOM.fetch_add(1, Ordering::Relaxed);
    }
    // SAFETY: the trampoline has this signature.
    let orig: extern "C" fn(u32, u32) -> u32 =
        unsafe { std::mem::transmute(table::original(Id::Calloc) as usize) };
    orig(n, size)
}

/// `_msize(p)` (cdecl, 0x828C4A): the requested size, as HeapSize reports it.
pub extern "C" fn msize_hook(p: u32) -> u32 {
    table::count(Id::Msize);
    if in_arena(p)
        && let Some(Some(b)) = arena(|a| a.block(p))
    {
        return b.size;
    }
    // SAFETY: the trampoline has this signature.
    let orig: extern "C" fn(u32) -> u32 = unsafe { std::mem::transmute(table::original(Id::Msize) as usize) };
    orig(p)
}

static HEAPLOG_ON: AtomicBool = AtomicBool::new(false);
static HEAPLOG: Mutex<Option<BufWriter<File>>> = Mutex::new(None);
static HEAPLOG_SEQ: AtomicU32 = AtomicU32::new(0);

/// `SA_DET_HEAPLOG`: one line per main-thread arena allocation: seq, frame, size, address and the
/// first stack words that point into game code (call chain evidence).
#[inline(never)]
fn heaplog(kind: char, size: u32, p: u32) {
    let rets = stack_words(6);
    let seq = HEAPLOG_SEQ.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut g) = HEAPLOG.lock()
        && let Some(w) = g.as_mut()
    {
        let _ = writeln!(w, "{seq} {} {kind} {size} {p:x} {rets}", frame());
    }
}

/// The first `n` words above the current stack pointer that point into game code (candidate return
/// addresses; stale words can appear, so this is evidence, not a reliable backtrace).
#[inline(never)]
fn stack_words(n: usize) -> String {
    let esp: u32;
    // SAFETY: reads the stack pointer.
    unsafe { std::arch::asm!("mov {0}, esp", out(reg) esp, options(nomem, nostack)) };
    let mut rets = Vec::new();
    let mut a = esp;
    while rets.len() < n && a < esp + 0x600 {
        // SAFETY: our own thread's stack above the current frame.
        let v = unsafe { std::ptr::read_volatile(a as usize as *const u32) };
        if (0x401000..0x857000).contains(&v) || (0x1556000..0x1577000).contains(&v) {
            rets.push(format!("{v:x}"));
        }
        a += 4;
    }
    rets.join(",")
}

// ------------------------------------------------------------------ rand ----------------------

/// `rand()` entry: passes the caller's return address to `rand_impl`.
#[unsafe(naked)]
pub unsafe extern "C" fn rand_shim() -> i32 {
    std::arch::naked_asm!(
        "push dword ptr [esp]",
        "call {imp}",
        "add esp, 4",
        "ret",
        imp = sym rand_impl,
    )
}

/// `srand(seed)` entry: passes (caller, seed) to `srand_impl`.
#[unsafe(naked)]
pub unsafe extern "C" fn srand_shim() {
    std::arch::naked_asm!(
        "push dword ptr [esp+4]",
        "push dword ptr [esp+4]",
        "call {imp}",
        "add esp, 8",
        "ret",
        imp = sym srand_impl,
    )
}

/// The caller a rand/srand call is attributed to: its return address when that is game code,
/// otherwise (a call from a Rust port) the original return address the port declared with
/// `sa_core::orig::at_site`, so original and Rust runs log and hash the same caller.
fn attributed_caller(ret: u32) -> u32 {
    if (0x401000..0x857000).contains(&ret) || (0x1556000..0x1577000).contains(&ret) {
        ret
    } else {
        sa_core::orig::current_site().unwrap_or(ret)
    }
}

extern "C" fn rand_impl(caller: u32) -> i32 {
    let caller = attributed_caller(caller);
    table::count(Id::Rand);
    // SAFETY: the trampoline has rand's signature.
    let orig: extern "C" fn() -> i32 = unsafe { std::mem::transmute(table::original(Id::Rand) as usize) };
    let v = orig();
    on_rand(0, caller, v as u32);
    v
}

extern "C" fn srand_impl(caller: u32, seed: u32) {
    let caller = attributed_caller(caller);
    table::count(Id::Srand);
    // SAFETY: the trampoline has srand's signature.
    let orig: extern "C" fn(u32) = unsafe { std::mem::transmute(table::original(Id::Srand) as usize) };
    orig(seed);
    on_rand(1, caller, seed);
}

fn on_rand(kind: u8, caller: u32, value: u32) {
    if is_main() {
        let s = RAND_SEQ.load(Ordering::Relaxed);
        RAND_SEQ.store(rand_seq_step(s, kind, caller, value), Ordering::Relaxed);
        if kind == 0 {
            RAND_MAIN.fetch_add(1, Ordering::Relaxed);
        } else {
            SRAND_MAIN.fetch_add(1, Ordering::Relaxed);
        }
        if RANDLOG_ON.load(Ordering::Relaxed)
            && let Ok(mut g) = RANDLOG.lock()
            && let Some(w) = g.as_mut()
        {
            let k = if kind == 0 { 'r' } else { 's' };
            if (0x4D0000..0x500000).contains(&caller) {
                // audio callers: add the call chain evidence
                let _ = writeln!(w, "{} {k} {caller:x} {value} {}", frame(), stack_words(8));
            } else {
                let _ = writeln!(w, "{} {k} {caller:x} {value}", frame());
            }
        }
    } else if let Ok(mut m) = RAND_OTHER.lock() {
        // SAFETY: plain Win32 call.
        let tid = unsafe { GetCurrentThreadId() };
        *m.entry((tid, kind, caller)).or_insert(0) += 1;
    }
}

// ------------------------------------------------------------------ L2 side state ---------------

/// Process state outside game memory that a function run changes through the oracle hooks: the rand
/// counters/hash, the main thread's `_holdrand` (thread data, not in a watched window) and the heap
/// arena's bookkeeping. The L2 oracle saves it before a case and puts the leave run's version back.
#[derive(Clone)]
pub(crate) struct Side {
    rand: (u64, u32, u32),
    holdrand: Option<(u32, u32)>,
    pub arena: Option<Arena>,
}

static HOLDRAND_ADDR: AtomicU32 = AtomicU32::new(0);
static L2_ARENA_CHECKPOINT: Mutex<Option<Arena>> = Mutex::new(None);
static L2_CHECKPOINT_WANTED: AtomicBool = AtomicBool::new(false);

/// Main-thread `&_holdrand`, resolved once.
fn holdrand_addr() -> u32 {
    let a = HOLDRAND_ADDR.load(Ordering::Relaxed);
    if a != 0 {
        return a;
    }
    // SAFETY: called on the main thread; _getptd (0x827B3D) is cdecl without arguments.
    let getptd: extern "C" fn() -> u32 = unsafe { std::mem::transmute(0x827B3Dusize) };
    let p = getptd();
    let a = if p == 0 { 0 } else { p + 0x14 };
    HOLDRAND_ADDR.store(a, Ordering::Relaxed);
    a
}

pub(crate) fn side_save() -> Side {
    let h = if ENABLED.load(Ordering::Acquire) {
        holdrand_addr()
    } else {
        0
    };
    Side {
        rand: (
            RAND_SEQ.load(Ordering::Relaxed),
            RAND_MAIN.load(Ordering::Relaxed),
            SRAND_MAIN.load(Ordering::Relaxed),
        ),
        holdrand: (h != 0).then(|| (h, get_u32(h))),
        arena: None,
    }
}

pub(crate) fn side_restore(s: &Side) {
    RAND_SEQ.store(s.rand.0, Ordering::Relaxed);
    RAND_MAIN.store(s.rand.1, Ordering::Relaxed);
    SRAND_MAIN.store(s.rand.2, Ordering::Relaxed);
    if let Some((a, v)) = s.holdrand {
        put(a, &v.to_le_bytes());
    }
    if let Some(a) = &s.arena
        && let Ok(mut g) = ARENA.lock()
    {
        *g = Some(a.clone());
    }
}

/// Starts a case: the arena is checkpointed lazily, at the first heap change inside a run.
pub(crate) fn l2_case_begin() {
    if let Ok(mut g) = L2_ARENA_CHECKPOINT.lock() {
        *g = None;
    }
    L2_CHECKPOINT_WANTED.store(true, Ordering::Release);
}

/// Ends a case: the arena as it was before the case's first heap change (None: untouched).
pub(crate) fn l2_case_end() -> Option<Arena> {
    L2_CHECKPOINT_WANTED.store(false, Ordering::Release);
    L2_ARENA_CHECKPOINT.lock().ok().and_then(|mut g| g.take())
}

/// The checkpoint so far (a copy), without ending the case.
pub(crate) fn l2_checkpoint() -> Option<Arena> {
    L2_ARENA_CHECKPOINT.lock().ok().and_then(|g| g.clone())
}

/// The arena's reserved range [base, end) (0, 0 when off).
pub(crate) fn arena_range() -> (u32, u32) {
    (ARENA_BASE.load(Ordering::Acquire), ARENA_END.load(Ordering::Acquire))
}

/// The used part of the arena [base, base + high water), if the arena is on.
pub(crate) fn arena_extent() -> Option<(u32, u32)> {
    arena(|a| (a.base, a.base + a.high_water))
}

/// The arena now (for the leave run's state).
pub(crate) fn arena_clone() -> Option<Arena> {
    ARENA.lock().ok().and_then(|g| g.clone())
}

fn l2_heap_touch() {
    if L2_CHECKPOINT_WANTED.load(Ordering::Relaxed)
        && crate::l2::heap_change_in_run()
        && let Ok(mut c) = L2_ARENA_CHECKPOINT.lock()
        && c.is_none()
    {
        *c = arena_clone();
    }
}

/// The main thread's `_holdrand` (`_getptd()` 0x827B3D, +0x14 as read by rand 0x821B23).
fn holdrand() -> u32 {
    // SAFETY: called on the main thread; _getptd is cdecl, no arguments, returns the thread's ptd.
    let getptd: extern "C" fn() -> u32 = unsafe { std::mem::transmute(0x827B3Dusize) };
    let p = getptd();
    if p == 0 { 0 } else { get_u32(p + 0x14) }
}

// ------------------------------------------------------------------ pad ------------------------

fn capture_with(f: impl FnOnce(&mut PadInput)) {
    if let Ok(mut g) = CAPTURE.lock() {
        f(g.get_or_insert_with(PadInput::default));
    }
}

/// `CPad::UpdateMouse` (thiscall, 0x53F3C0): the first call of an input tick.
pub extern "thiscall" fn pad_update_mouse_hook(this: *mut c_void) {
    table::count(Id::PadUpdateMouse);
    // SAFETY: the trampoline has this signature.
    let orig: extern "thiscall" fn(*mut c_void) =
        unsafe { std::mem::transmute(table::original(Id::PadUpdateMouse) as usize) };
    let (active, inj) = match INPUT.lock() {
        Ok(mut g) => match g.as_mut() {
            Some(s) => (true, s.begin_tick(frame()).cloned()),
            None => (false, None),
        },
        Err(_) => (false, None),
    };
    if active {
        if let Some(i) = &inj {
            put(pad::addr::TEMP_KEY_STATE, &i.temp_key_state);
        }
        capture_with(|c| get(pad::addr::TEMP_KEY_STATE, &mut c.temp_key_state));
    }
    orig(this);
    if active {
        if let Some(i) = &inj {
            put(pad::addr::NEW_MOUSE_STATE, &i.new_mouse);
        }
        capture_with(|c| get(pad::addr::NEW_MOUSE_STATE, &mut c.new_mouse));
    }
}

/// `CPad::Update(int32 pad)` (thiscall, 0x541C40): for `Pads[0]` the end of an input tick.
pub extern "thiscall" fn pad_update_hook(this: *mut c_void, pad_id: i32) {
    table::count(Id::PadUpdate);
    // SAFETY: the trampoline has this signature.
    let orig: extern "thiscall" fn(*mut c_void, i32) =
        unsafe { std::mem::transmute(table::original(Id::PadUpdate) as usize) };
    if this as usize as u32 == pad::addr::PAD0
        && let Ok(mut g) = INPUT.lock()
        && let Some(s) = g.as_mut()
    {
        let base = pad::addr::PAD0;
        if let Some(i) = s.injected() {
            put(base + pad::addr::PC_TEMP_KEY, &i.pc_temp_key);
            put(base + pad::addr::PC_TEMP_JOY, &i.pc_temp_joy);
            put(base + pad::addr::PC_TEMP_MOUSE, &i.pc_temp_mouse);
        }
        let mut cap = CAPTURE.lock().ok().and_then(|mut c| c.take()).unwrap_or_default();
        get(base + pad::addr::PC_TEMP_KEY, &mut cap.pc_temp_key);
        get(base + pad::addr::PC_TEMP_JOY, &mut cap.pc_temp_joy);
        get(base + pad::addr::PC_TEMP_MOUSE, &mut cap.pc_temp_mouse);
        s.end_tick(frame(), cap);
    }
    orig(this, pad_id);
}

// ------------------------------------------------------------------ frame boundary -------------

fn perturb_player() -> String {
    let mut pages = PAGES.lock().unwrap_or_else(|e| e.into_inner());
    let Some(pages) = pages.as_mut() else {
        return "no page cache".into();
    };
    let image = *IMAGE.lock().unwrap_or_else(|e| e.into_inner());
    let m = GameMem {
        image,
        pages: RefCell::new(pages),
    };
    let Some(a) = subsys::player_pos_x_addr(&m) else {
        return "no player ped".into();
    };
    let Some(bits) = m.u32(a) else {
        return format!("{a:#x} not readable");
    };
    let new = bits.wrapping_add(1);
    put(a, &new.to_le_bytes());
    format!(
        "player pos.x at {a:#x}: {:#010x} ({}) -> {new:#010x} ({})",
        bits,
        f32::from_bits(bits),
        f32::from_bits(new)
    )
}

fn hash_now() -> [u64; COUNT] {
    let extra = Extra {
        holdrand: holdrand(),
        rand_calls: RAND_SEQ.swap(fnv::OFFSET, Ordering::Relaxed),
    };
    let mut pages = PAGES.lock().unwrap_or_else(|e| e.into_inner());
    let pages = pages.get_or_insert_with(HashSet::new);
    let image = *IMAGE.lock().unwrap_or_else(|e| e.into_inner());
    let m = GameMem {
        image,
        pages: RefCell::new(pages),
    };
    subsys::hash_frame(&m, &extra)
}

/// Called by `win::frame_boundary` on the main thread before `CTimer::Update` runs.
pub fn on_boundary(f: u32) {
    if !enabled() {
        return;
    }
    let Some(cfg) = CFG.get() else { return };
    if MAIN_TID.load(Ordering::Relaxed) == 0 || (f == 1 && !is_main()) {
        // SAFETY: plain Win32 call.
        let me = unsafe { GetCurrentThreadId() };
        log(&format!(
            "oracle: main thread corrected {} -> {me} at frame {f}",
            MAIN_TID.load(Ordering::Relaxed)
        ));
        MAIN_TID.store(me, Ordering::Release);
    }
    if !POOLS_LOGGED.load(Ordering::Relaxed) && get_u32(subsys::addr::PED_POOL) != 0 {
        // Heap placement evidence: the pool storage addresses of this run.
        POOLS_LOGGED.store(true, Ordering::Relaxed);
        let mut s = String::new();
        for i in 0..subsys::addr::POOL_COUNT {
            let p = get_u32(subsys::addr::POOLS + i * 4);
            if p != 0 {
                s += &format!(" {i}:{p:#x}/{:#x}/{:#x}", get_u32(p), get_u32(p + 4));
            }
        }
        log(&format!("oracle: frame {f}: pools (header/storage/bytemap):{s}"));
    }
    if !DONE.load(Ordering::Acquire) {
        if cfg.perturb == Some(f) {
            let s = perturb_player();
            log(&format!("oracle: frame {f}: perturbed {s}"));
            *PERTURBED.lock().unwrap_or_else(|e| e.into_inner()) = format!("frame {f}: {s}");
        }
        if cfg.hash {
            let h = hash_now();
            let tick = INPUT
                .lock()
                .ok()
                .and_then(|g| g.as_ref().map(|s| s.ticks))
                .unwrap_or(0);
            if let Ok(mut g) = TRACE.lock()
                && let Some(w) = g.as_mut()
            {
                let _ = w.write_all(&trace::encode_record(f, tick, &h));
                if f.is_multiple_of(256) {
                    let _ = w.flush();
                }
            }
            if let Ok(mut v) = HASHES.lock() {
                v.push((f, tick, h));
            }
        }
        if cfg.end_frame == Some(f) {
            finish(f);
        }
    }

    if DET.load(Ordering::Acquire) {
        NOW.fetch_add(STEP_TICKS.load(Ordering::Relaxed), Ordering::AcqRel);
    }
}

/// `fnv64:size` fingerprint of a file.
fn fingerprint(path: &std::path::Path, hash: bool) -> String {
    match std::fs::metadata(path) {
        Ok(md) if !hash => format!("size:{}", md.len()),
        Ok(md) => match std::fs::read(path) {
            Ok(b) => format!("{:016x}:{}", fnv::hash(&b), md.len()),
            Err(e) => format!("unreadable:{}:{e}", md.len()),
        },
        Err(_) => "missing".into(),
    }
}

/// Files that define the game's behaviour, recorded in the recording header (relative to the game dir;
/// the two big IMG archives by size only).
const DATA_FILES: &[(&str, bool)] = &[
    ("data/script/main.scm", true),
    ("data/script/script.img", true),
    ("data/gta.dat", true),
    ("data/default.dat", true),
    ("data/default.ide", true),
    ("data/peds.ide", true),
    ("data/vehicles.ide", true),
    ("data/handling.cfg", true),
    ("data/timecyc.dat", true),
    ("data/weapon.dat", true),
    ("data/pedstats.dat", true),
    ("data/surfinfo.dat", true),
    ("data/object.dat", true),
    ("data/carcols.dat", true),
    ("data/popcycle.dat", true),
    ("models/gta3.img", false),
    ("models/gta_int.img", false),
];

fn write_recording(path: &str, mut rec: Recording, last_frame: u32) -> String {
    let cfg = CFG.get().cloned().unwrap_or_default();
    let h = &mut rec.header;
    h.set(
        "start",
        RECORD_START.lock().map(|s| s.clone()).unwrap_or_default(),
    );
    h.set("det", if DET.load(Ordering::Acquire) { 1 } else { 0 });
    h.set("det.step_us", cfg.step_us);
    h.set("det.clock_base", cfg.clock_base);
    h.set("det.qpf", QPF.load(Ordering::Acquire));
    h.set("frames", last_frame);
    if let Some(p) = &cfg.program
        && let Ok(s) = std::fs::read_to_string(p)
    {
        h.set("program", format!("{:016x}", fnv::hash(s.as_bytes())));
    }
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_default();
    h.set("exe", fingerprint(&dir.join("gta_sa.exe"), true));
    for (f, full) in DATA_FILES {
        h.set(&format!("data.{f}"), fingerprint(&dir.join(f), *full));
    }
    if let Ok(up) = std::env::var("USERPROFILE") {
        let set = std::path::Path::new(&up).join("Documents\\GTA San Andreas User Files\\gta_sa.set");
        h.set("settings", fingerprint(&set, true));
    }
    let n = rec.ticks.len();
    match std::fs::write(path, rec.encode()) {
        Ok(()) => format!("ok record {path} ticks={n} frames={last_frame}"),
        Err(e) => format!("err record {path}: {e}"),
    }
}

fn finish(f: u32) {
    if let Ok(mut g) = TRACE.lock()
        && let Some(w) = g.as_mut()
    {
        let _ = w.flush();
    }
    if let Ok(mut g) = RANDLOG.lock()
        && let Some(w) = g.as_mut()
    {
        let _ = w.flush();
    }
    if let Ok(mut g) = HEAPLOG.lock()
        && let Some(w) = g.as_mut()
    {
        let _ = w.flush();
    }
    let rec = INPUT
        .lock()
        .ok()
        .and_then(|mut g| g.as_mut().and_then(|s| s.recording.take()));
    let path = RECORD_PATH.lock().ok().and_then(|g| g.clone());
    if let (Some(rec), Some(path)) = (rec, path) {
        log(&format!("oracle: {}", write_recording(&path, rec, f)));
    }
    crate::l2win::finish();
    crate::coverage::finish();
    DONE.store(true, Ordering::Release);
    log(&format!("oracle: end frame {f} reached; {}", status()));
    if CFG.get().is_some_and(|c| c.exit_at_end) {
        log("oracle: exiting (SA_DET_EXIT=1)");
        // SAFETY: terminates the game process on purpose.
        unsafe { ExitProcess(0) };
    }
}

// ------------------------------------------------------------------ control commands -----------

pub fn status() -> String {
    let cfg = CFG.get().cloned().unwrap_or_default();
    let (mode, ticks, desyncs, first, replay_done) = match INPUT.lock() {
        Ok(g) => match g.as_ref() {
            Some(s) => (
                s.mode(),
                s.ticks,
                s.desyncs,
                s.first_desync.clone().unwrap_or_default(),
                s.replay_done,
            ),
            None => ("none", 0, 0, String::new(), false),
        },
        Err(_) => ("poisoned", 0, 0, String::new(), false),
    };
    let other: Vec<String> = RAND_OTHER
        .lock()
        .map(|m| {
            m.iter()
                .map(|((t, k, c), n)| format!("{t}:{}:{c:#x}:{n}", if *k == 0 { "rand" } else { "srand" }))
                .collect()
        })
        .unwrap_or_default();
    format!(
        "det enabled={} det={} hash={} frame={} ticks={ticks} input={mode} end={} done={} clock_ms={} \
         sync_reads={} rand_main={} srand_main={} rand_other=[{}] offthread_time={} desyncs={desyncs} \
         replay_done={replay_done} first_desync=\"{first}\" perturbed=\"{}\" {}",
        enabled() as u8,
        DET.load(Ordering::Acquire) as u8,
        cfg.hash as u8,
        frame(),
        cfg.end_frame.map(|e| e.to_string()).unwrap_or_else(|| "-".into()),
        DONE.load(Ordering::Acquire) as u8,
        clock_now().ms(),
        SYNC_READS.load(Ordering::Relaxed),
        RAND_MAIN.load(Ordering::Relaxed),
        SRAND_MAIN.load(Ordering::Relaxed),
        other.join(","),
        OFFTHREAD_TIME.load(Ordering::Relaxed),
        PERTURBED.lock().map(|s| s.clone()).unwrap_or_default(),
        heap_status(),
    ) + " "
        + &crate::detaudio::status()
}

fn heap_status() -> String {
    let a = arena(|a| {
        format!(
            "heap=on arena={:#x} used={} high={} live={} allocs={} frees={}",
            a.base,
            a.used(),
            a.high_water,
            a.live_count(),
            a.allocs,
            a.frees
        )
    })
    .unwrap_or_else(|| "heap=off".into());
    format!(
        "{a} xthread={} oom={} other_allocs={} bad_free={}",
        HEAP_XTHREAD.load(Ordering::Relaxed),
        HEAP_OOM.load(Ordering::Relaxed),
        HEAP_OTHER_ALLOCS.load(Ordering::Relaxed),
        HEAP_BAD_FREE.load(Ordering::Relaxed)
    )
}

/// `stack`: suspends the main thread for a moment and reports EIP and the stack words that point into
/// game code (candidate return addresses), to diagnose hangs. Works without the oracle features.
pub fn main_stack() -> String {
    use windows_sys::Win32::System::Diagnostics::Debug::{CONTEXT, CONTEXT_FULL_X86, GetThreadContext};
    use windows_sys::Win32::System::Threading::{
        OpenThread, ResumeThread, SuspendThread, THREAD_GET_CONTEXT, THREAD_SUSPEND_RESUME,
    };
    let tid = match MAIN_TID.load(Ordering::Acquire) {
        0 => main_thread_guess().0,
        t => t,
    };
    // SAFETY: suspends/inspects/resumes the game's main thread; the stack is read with `win::read`.
    unsafe {
        let h = OpenThread(THREAD_GET_CONTEXT | THREAD_SUSPEND_RESUME, 0, tid);
        if h.is_null() {
            return format!("err OpenThread({tid}) failed");
        }
        let mut ctx: CONTEXT = std::mem::zeroed();
        ctx.ContextFlags = CONTEXT_FULL_X86;
        SuspendThread(h);
        let ok = GetThreadContext(h, &mut ctx) != 0;
        let stack = if ok { win::read(ctx.Esp, 0x2000) } else { None };
        ResumeThread(h);
        windows_sys::Win32::Foundation::CloseHandle(h);
        if !ok {
            return "err GetThreadContext failed".into();
        }
        let mut rets = Vec::new();
        if let Some(s) = stack {
            for (i, w) in s.as_chunks::<4>().0.iter().enumerate() {
                let v = u32::from_le_bytes(*w);
                if (0x401000..0x857000).contains(&v) || (0x1556000..0x1577000).contains(&v) {
                    rets.push(format!("+{:#x}:{v:#x}", i * 4));
                }
                if rets.len() >= 40 {
                    break;
                }
            }
        }
        format!(
            "stack tid={tid} eip={:#x} esp={:#x} ebp={:#x} frame={} code_words=[{}]",
            ctx.Eip,
            ctx.Esp,
            ctx.Ebp,
            frame(),
            rets.join(" ")
        )
    }
}

/// `hashes [FRAME]`: the latest (or the given) frame's hashes.
pub fn hashes(want: Option<u32>) -> String {
    let Ok(v) = HASHES.lock() else {
        return "err poisoned".into();
    };
    let rec = match want {
        Some(f) => v.iter().rev().find(|r| r.0 == f),
        None => v.last(),
    };
    match rec {
        Some((f, t, h)) => {
            let mut s = format!("hashes frame={f} tick={t}");
            for &sub in Subsystem::ALL {
                s += &format!(" {}={:016x}", sub.name(), h[sub as usize]);
            }
            s
        }
        None if !enabled() => "err hashing off (start with SA_HASH=1 or SA_DET=1)".into(),
        None => "err no such frame".into(),
    }
}

/// `record PATH` (start recording the live input at the next tick) / `record stop`.
pub fn record(arg: &str) -> String {
    if !enabled() {
        return "err oracle features off (start with SA_HASH=1 or SA_DET=1)".into();
    }
    let Ok(mut g) = INPUT.lock() else {
        return "err poisoned".into();
    };
    if arg == "stop" {
        let rec = g.as_mut().and_then(|s| s.recording.take());
        drop(g);
        let path = RECORD_PATH.lock().ok().and_then(|mut p| p.take());
        return match (rec, path) {
            (Some(rec), Some(path)) => write_recording(&path, rec, frame()),
            _ => "err not recording".into(),
        };
    }
    match g.as_mut() {
        Some(s) if s.recording.is_some() => return "err already recording".into(),
        Some(s) => s.recording = Some(Recording::default()),
        None => *g = Some(InputState::new(Source::Live, true)),
    }
    *RECORD_PATH.lock().unwrap_or_else(|e| e.into_inner()) = Some(arg.to_string());
    *RECORD_START.lock().unwrap_or_else(|e| e.into_inner()) = format!("midrun:{}", frame());
    format!("ok recording {arg} from frame {}", frame())
}

/// `replay PATH`: inject a recording from the next tick on (frames rebased to the current frame).
pub fn replay(path: &str) -> String {
    if !enabled() {
        return "err oracle features off (start with SA_HASH=1 or SA_DET=1)".into();
    }
    let mut rec = match std::fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(|b| Recording::parse(&b))
    {
        Ok(r) => r,
        Err(e) => return format!("err {path}: {e}"),
    };
    let now = frame();
    if let Some(first) = rec.ticks.first().map(|t| t.frame) {
        for t in &mut rec.ticks {
            t.frame = t.frame.wrapping_sub(first).wrapping_add(now + 1);
        }
    }
    let n = rec.ticks.len();
    let Ok(mut g) = INPUT.lock() else {
        return "err poisoned".into();
    };
    let keep_rec = g.as_mut().and_then(|s| s.recording.take());
    let mut s = InputState::new(Source::Replay { rec, pos: 0 }, false);
    s.recording = keep_rec;
    *g = Some(s);
    format!("ok replaying {path} ticks={n} from frame {}", now + 1)
}
