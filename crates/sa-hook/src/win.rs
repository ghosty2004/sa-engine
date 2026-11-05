//! Windows side: DllMain, hook installation, the frame boundary, the control server and the log.
//!
//! Per hook, one 64-byte slot in an RWX block:
//! ```text
//! thunk  (+0):  [pushad; pushfd; call frame_boundary; popfd; popad]   only for the frame hook
//!               jmp dword [TARGET+i*4]          → Rust implementation or original
//! tramp (+32):  <relocated entry bytes>; jmp addr+n      (inline hooks only)
//! ```
//! The function entry gets `jmp thunk` (+ NOPs up to the stolen length); a `.HOODLUM` stub gets its
//! `jmp` retargeted to the thunk and its old target becomes the original. Toggling a hook is one
//! aligned 32-bit store into `TARGET[i]`, done by the game thread inside the frame hook.

use std::ffi::c_void;
use std::fs::File;
use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{HINSTANCE, TRUE};
use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows_sys::Win32::System::LibraryLoader::DisableThreadLibraryCalls;
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_RESERVE, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE_READWRITE, PAGE_GUARD, PAGE_NOACCESS,
    VirtualAlloc, VirtualProtect, VirtualQuery,
};
use windows_sys::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use crate::control::{self, Command};
use crate::game;
use crate::table::{self, Group, HOOKS, Id, Kind, Mode};
use crate::x86;

const SLOT: usize = 96;
const TRAMP_OFF: usize = 32;
/// Per-hook L2 entry stub (`push i; jmp l2_common`).
const L2_OFF: usize = 80;
/// Bytes swept after an entry for branches into the patched range.
const INBOUND_SWEEP: usize = 0x400;

// ------------------------------------------------------------------ log -----------------------

static LOG: Mutex<Option<File>> = Mutex::new(None);
static START: OnceLock<Instant> = OnceLock::new();

pub(crate) fn log(msg: &str) {
    let t = START.get().map(|s| s.elapsed().as_millis()).unwrap_or(0);
    if let Ok(mut g) = LOG.lock()
        && let Some(f) = g.as_mut()
    {
        let _ = writeln!(f, "[{t:>8} ms] {msg}");
        let _ = f.flush();
    }
}

// ------------------------------------------------------------------ memory --------------------

pub(crate) fn readable(addr: u32, len: usize) -> bool {
    if addr < 0x10000 {
        return false;
    }
    let end = addr as u64 + len as u64;
    let mut p = addr as u64;
    while p < end {
        // SAFETY: VirtualQuery only inspects the address space.
        let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
        let n = unsafe {
            VirtualQuery(
                p as usize as *const c_void,
                &mut mbi,
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };
        if n == 0 || mbi.State != MEM_COMMIT || mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD) != 0 {
            return false;
        }
        p = mbi.BaseAddress as u64 + mbi.RegionSize as u64;
    }
    true
}

pub(crate) fn read(addr: u32, len: usize) -> Option<Vec<u8>> {
    if !readable(addr, len) {
        return None;
    }
    let mut v = vec![0u8; len];
    // SAFETY: the range was just checked to be committed and readable.
    unsafe { std::ptr::copy_nonoverlapping(addr as usize as *const u8, v.as_mut_ptr(), len) };
    Some(v)
}

fn read_u32(addr: u32) -> Option<u32> {
    read(addr, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Writes code bytes (temporarily making the page writable) and flushes the instruction cache.
pub(crate) fn patch(addr: u32, bytes: &[u8]) -> Result<(), String> {
    let p = addr as usize as *mut c_void;
    let mut old = 0u32;
    // SAFETY: `addr` is verified game code; the game's threads are suspended (attach) or the caller
    // guarantees no thread executes these bytes.
    unsafe {
        if VirtualProtect(p, bytes.len(), PAGE_EXECUTE_READWRITE, &mut old) == 0 {
            return Err(format!(
                "VirtualProtect({addr:#x}) failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), p as *mut u8, bytes.len());
        VirtualProtect(p, bytes.len(), old, &mut old);
        FlushInstructionCache(GetCurrentProcess(), p, bytes.len());
    }
    Ok(())
}

// ------------------------------------------------------------------ frame boundary ------------

static FRAMES: AtomicU32 = AtomicU32::new(0);

/// Frame boundaries so far (1 = the first `CTimer::Update` of the process).
pub(crate) fn frames() -> u32 {
    FRAMES.load(Ordering::Acquire)
}

type Job = Box<dyn FnOnce() + Send>;
static JOBS: Mutex<Vec<Job>> = Mutex::new(Vec::new());

/// Called by the frame hook's thunk (all registers and flags saved), on the game's main thread,
/// before `CTimer::Update` runs: a frame boundary. Never blocks.
extern "C" fn frame_boundary() {
    let _ = catch_unwind(|| {
        let f = FRAMES.fetch_add(1, Ordering::AcqRel) + 1;
        for (i, m) in table::apply_pending() {
            log(&format!("frame {f}: {} -> {}", HOOKS[i].name, m.name()));
        }
        crate::l2win::on_boundary();
        // Hashes of the state the previous frame left, virtual clock step, end-of-run handling.
        crate::detwin::on_boundary(f);
        let jobs: Vec<Job> = match JOBS.try_lock() {
            Ok(mut g) if !g.is_empty() => std::mem::take(&mut *g),
            _ => Vec::new(),
        };
        for j in jobs {
            j();
        }
    });
}

/// Runs `f` on the game thread at the next frame boundary and waits for its result.
fn on_game_thread<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static, wait: Duration) -> Option<T> {
    let (tx, rx) = mpsc::channel();
    JOBS.lock().ok()?.push(Box::new(move || {
        let _ = tx.send(f());
    }));
    rx.recv_timeout(wait).ok()
}

// ------------------------------------------------------------------ install -------------------

fn thunk(slot: u32, i: usize, boundary: bool) -> Vec<u8> {
    let mut c = Vec::new();
    if boundary {
        c.extend_from_slice(&[0x60, 0x9C, 0xE8]); // pushad; pushfd; call
        let fb = frame_boundary as extern "C" fn() as usize as u32;
        c.extend_from_slice(&x86::rel32_to(slot + 2, 5, fb).to_le_bytes());
        c.extend_from_slice(&[0x9D, 0x61]); // popfd; popad
    }
    c.extend_from_slice(&[0xFF, 0x25]); // jmp dword [abs32]
    c.extend_from_slice(&(table::target_slot(i) as u32).to_le_bytes());
    c
}

fn install_one(i: usize, slot: u32) -> Result<String, String> {
    let h = &HOOKS[i];
    let cur = read(h.addr, h.expect.len()).ok_or("entry not readable")?;
    if cur != h.expect {
        return Err(format!("unexpected entry bytes {}", control::hex(&cur)));
    }
    let code = thunk(slot, i, h.frame_boundary);
    assert!(code.len() <= TRAMP_OFF);
    let (original, entry_patch, what) = match h.kind {
        Kind::HoodlumStub => {
            let insn = x86::decode(&cur).map_err(|e| e.to_string())?;
            let old = insn.target(&cur, h.addr).ok_or("stub is not a jmp rel32")?;
            (
                old,
                x86::jmp_rel32(h.addr, slot).to_vec(),
                format!("stub retargeted, original {old:#x}"),
            )
        }
        Kind::Inline => {
            let tramp = slot + TRAMP_OFF as u32;
            let src = read(h.addr, 32).ok_or("entry not readable")?;
            let r = x86::relocate(&src, h.addr, tramp, 5).map_err(|e| e.to_string())?;
            if r.code.len() > L2_OFF - TRAMP_OFF {
                return Err("trampoline too long".into());
            }
            let around = read(h.addr, INBOUND_SWEEP).unwrap_or_default();
            if let Some((at, t)) = x86::inbound_branch(&around, h.addr, h.addr, h.addr + r.consumed as u32) {
                return Err(format!("branch at {at:#x} into the patched range ({t:#x})"));
            }
            // SAFETY: the slot lies in our RWX block.
            unsafe {
                std::ptr::copy_nonoverlapping(r.code.as_ptr(), tramp as usize as *mut u8, r.code.len());
            }
            let mut p = x86::jmp_rel32(h.addr, slot).to_vec();
            p.resize(r.consumed, 0x90);
            (tramp, p, format!("{} bytes relocated to {tramp:#x}", r.consumed))
        }
    };
    // SAFETY: the slot lies in our RWX block.
    unsafe { std::ptr::copy_nonoverlapping(code.as_ptr(), slot as usize as *mut u8, code.len()) };
    if h.sig.is_some() {
        let at = slot + L2_OFF as u32;
        let stub = crate::l2win::stub_bytes(at, i);
        // SAFETY: the slot lies in our RWX block.
        unsafe { std::ptr::copy_nonoverlapping(stub.as_ptr(), at as usize as *mut u8, stub.len()) };
        table::set_l2_stub(i, at);
    }
    let mode = if h.start_rust { Mode::Rust } else { Mode::Original };
    table::set_installed(i, original, mode);
    patch(h.addr, &entry_patch)?;
    Ok(format!("{what}; thunk {slot:#x}; mode {}", mode.name()))
}

/// Installs every Core hook, and the Oracle group too when `oracle` is set. Returns (installed, wanted).
fn install_all(oracle: bool) -> (usize, usize) {
    // SAFETY: plain allocation of an executable block for thunks and trampolines.
    let block = unsafe {
        VirtualAlloc(
            std::ptr::null(),
            HOOKS.len() * SLOT,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        )
    } as usize as u32;
    if block == 0 {
        log("VirtualAlloc failed: no hooks installed");
        return (0, HOOKS.len());
    }
    let (mut ok, mut wanted) = (0, 0);
    for (i, h) in HOOKS.iter().enumerate() {
        if h.group == Group::Oracle && !oracle {
            continue;
        }
        wanted += 1;
        match install_one(i, block + (i * SLOT) as u32) {
            Ok(s) => {
                ok += 1;
                log(&format!("hook {:#x} {}: {s}", h.addr, h.name));
            }
            Err(e) => log(&format!("hook {:#x} {}: FAILED: {e}", h.addr, h.name)),
        }
    }
    (ok, wanted)
}

// ------------------------------------------------------------------ control -------------------

fn fpu_state() -> (u16, u32) {
    let mut cw: u16 = 0;
    let mut mx: u32 = 0;
    // SAFETY: stores the x87 control word and MXCSR into locals.
    unsafe {
        std::arch::asm!("fnstcw word ptr [{0}]", "stmxcsr dword ptr [{1}]", in(reg) &mut cw, in(reg) &mut mx,
            options(nostack));
    }
    (cw, mx)
}

/// Parity check on the game thread: every Rust port against its original over a set of inputs.
fn selftest() -> String {
    let mut out = Vec::new();
    let (cw, mx) = fpu_state();
    out.push(format!("fpu cw={cw:#06x} mxcsr={mx:#010x}"));
    if table::installed(Id::LimitRadianAngle as usize) {
        // SAFETY: the trampoline has the original's signature.
        let orig: extern "C" fn(f32) -> f32 =
            unsafe { std::mem::transmute(table::original(Id::LimitRadianAngle) as usize) };
        let mut inputs: Vec<f32> = vec![
            0.0,
            -0.0,
            1.0,
            -1.0,
            25.0,
            -25.0,
            24.999998,
            -24.999998,
            100.0,
            -100.0,
            1e-30,
            -1e-30,
            1e30,
            f32::MIN_POSITIVE,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            f32::MAX,
            f32::MIN,
            game::general::PI,
            -game::general::PI,
            game::general::TWO_PI,
            -game::general::TWO_PI,
        ];
        inputs.extend((-30_000..=30_000).map(|k| k as f32 / 1000.0));
        let mut s: u32 = 0x1234_5678;
        for _ in 0..100_000 {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            inputs.push(f32::from_bits(s));
        }
        let mut bad = 0usize;
        let mut first = String::new();
        for &x in &inputs {
            let (a, b) = (orig(x), game::general::limit_radian_angle(x));
            let same = a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan());
            if !same {
                if bad == 0 {
                    first = format!(
                        " first={:#010x}:orig={:#010x},rust={:#010x}",
                        x.to_bits(),
                        a.to_bits(),
                        b.to_bits()
                    );
                }
                bad += 1;
            }
        }
        out.push(format!(
            "{} n={} mismatches={bad}{first}",
            table::spec(Id::LimitRadianAngle).name,
            inputs.len()
        ));
    }
    if table::installed(Id::IsCoopGameGoingOn as usize) {
        // SAFETY: the old stub target has the original's signature.
        let orig: extern "C" fn() -> u32 =
            unsafe { std::mem::transmute(table::original(Id::IsCoopGameGoingOn) as usize) };
        let p = game::game_logic::PLAYERS;
        let (p1, p2) = (
            read_u32(p).unwrap_or(0),
            read_u32(p + game::game_logic::PLAYER_INFO_SIZE).unwrap_or(0),
        );
        let (a, b) = (orig(), game::game_logic::is_coop_game_going_on(p1, p2));
        out.push(format!(
            "{} n=1 mismatches={} orig={a} rust={b} ped1={p1:#x} ped2={p2:#x}",
            table::spec(Id::IsCoopGameGoingOn).name,
            (a != b) as u32
        ));
    }
    format!("selftest {}", out.join("; "))
}

fn handle(cmd: &Command) -> String {
    match cmd {
        Command::Ping => "pong".into(),
        Command::Frames => format!(
            "frames hook={} game={} state={} time={}",
            FRAMES.load(Ordering::Acquire),
            read_u32(game::timer::FRAME_COUNTER).unwrap_or(0),
            read_u32(game::GAME_STATE).unwrap_or(0),
            read_u32(game::timer::TIME_IN_MS).unwrap_or(0),
        ),
        Command::Hooks => {
            let mut s = format!("hooks {}", HOOKS.len());
            for (i, h) in HOOKS.iter().enumerate() {
                let state = if table::installed(i) {
                    table::mode(i).name()
                } else {
                    "not-installed"
                };
                s += &format!("; {:#x} {} {} calls={}", h.addr, h.name, state, table::calls(i));
            }
            s
        }
        Command::Hook { key, rust } => {
            let Some(i) = table::find(key) else {
                return format!("err unknown hook {key}");
            };
            if !table::installed(i) {
                return format!("err {} not installed", HOOKS[i].name);
            }
            let m = if *rust { Mode::Rust } else { Mode::Original };
            table::request(i, m);
            log(&format!("control: {} -> {} requested", HOOKS[i].name, m.name()));
            let t0 = Instant::now();
            while table::pending(i).is_some() {
                if t0.elapsed() > Duration::from_secs(5) {
                    return format!(
                        "pending {} {} (no frame boundary within 5 s)",
                        HOOKS[i].name,
                        m.name()
                    );
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            format!(
                "ok {} {} frame={}",
                HOOKS[i].name,
                table::mode(i).name(),
                FRAMES.load(Ordering::Acquire)
            )
        }
        Command::Peek { addr, len } => match read(*addr, *len) {
            Some(b) => format!("peek {addr:#x} {}", control::hex(&b)),
            None => format!("err {addr:#x} not readable"),
        },
        Command::Selftest => on_game_thread(selftest, Duration::from_secs(10))
            .unwrap_or_else(|| "err no frame boundary".into()),
        Command::Hashes(f) => crate::detwin::hashes(*f),
        Command::Det => crate::detwin::status(),
        Command::Record(arg) => crate::detwin::record(arg),
        Command::Replay(path) => crate::detwin::replay(path),
        Command::Stack => crate::detwin::main_stack(),
        Command::HookL2(key) => {
            let Some(i) = table::find(key) else {
                return format!("err unknown hook {key}");
            };
            if !table::installed(i) || table::l2_stub(i) == 0 {
                return format!("err {} has no L2 entry", HOOKS[i].name);
            }
            table::request(i, Mode::L2);
            let t0 = Instant::now();
            while table::pending(i).is_some() {
                if t0.elapsed() > Duration::from_secs(5) {
                    return format!("pending {} l2", HOOKS[i].name);
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            format!("ok {} {}", HOOKS[i].name, table::mode(i).name())
        }
        Command::L2(None) => crate::l2win::summary(),
        Command::L2(Some(a)) if a == "flush" => crate::l2win::flush(),
        Command::L2(Some(a)) => format!("err l2 {a}: unknown (l2 | l2 flush)"),
        Command::Unknown(s) => format!("err unknown command {s:?}"),
    }
}

fn control_thread(port: u16) {
    match std::net::TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => {
            log(&format!("control: listening on 127.0.0.1:{port}"));
            control::serve(l, |c| {
                catch_unwind(AssertUnwindSafe(|| handle(c))).unwrap_or_else(|_| "err panic".into())
            });
        }
        Err(e) => log(&format!("control: bind 127.0.0.1:{port} failed: {e}")),
    }
}

// ------------------------------------------------------------------ entry ---------------------

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(hinst: HINSTANCE, reason: u32, _reserved: *mut c_void) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        // SAFETY: valid module handle passed by the loader.
        unsafe { DisableThreadLibraryCalls(hinst) };
        let _ = catch_unwind(attach);
    }
    TRUE
}

fn attach() {
    let _ = START.set(Instant::now());
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_default();
    if let Ok(f) = File::create(dir.join("sa-hook.log")) {
        *LOG.lock().unwrap_or_else(|e| e.into_inner()) = Some(f);
    }
    std::panic::set_hook(Box::new(|info| log(&format!("PANIC: {info}"))));
    let port = control::port_from_env(std::env::var("SA_HOOK_PORT").ok().as_deref());
    log(&format!(
        "sa-hook {} attached in {}",
        env!("CARGO_PKG_VERSION"),
        dir.display()
    ));
    let oracle = crate::detwin::init(&dir);
    let (ok, wanted) = install_all(oracle);
    log(&format!(
        "{ok}/{wanted} hooks installed (oracle group {})",
        if oracle { "on" } else { "off" }
    ));
    // Start modes (game threads are still suspended, so the stores take effect before any call).
    if let Ok(v) = std::env::var("SA_HOOK_MODE") {
        match crate::l2::parse_hook_modes(&v) {
            Ok(list) => {
                for (i, m) in list {
                    table::set_mode_now(i, m);
                    log(&format!(
                        "start mode: {} -> {}",
                        HOOKS[i].name,
                        table::mode(i).name()
                    ));
                }
            }
            Err(e) => log(&format!("SA_HOOK_MODE ignored: {e}")),
        }
    }
    for i in crate::l2win::init(&dir) {
        if table::installed(i) {
            table::set_mode_now(i, Mode::L2);
        } else {
            log(&format!("l2: {} is not installed", HOOKS[i].name));
        }
    }
    let _ = std::thread::Builder::new()
        .name("sa-hook-ctl".into())
        .spawn(move || control_thread(port));
}
