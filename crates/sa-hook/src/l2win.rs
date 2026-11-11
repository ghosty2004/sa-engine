//! L2 oracle, Windows side (INF-8): runs the original and the Rust implementation of a hooked function
//! at its real call site in the running game, from the same state, and compares them.
//!
//! A hook in mode `l2` jumps (thunk `jmp [TARGET]`) to its per-hook stub `push i; jmp l2_common`.
//! `l2_common` saves the caller's registers, FPU environment and MXCSR into a [`Frame`] on the stack
//! and calls [`l2_dispatch`]. Not sampled (or not the game thread, or already inside a case): the frame
//! is unwound and the call tail-jumps to the leave target with every register as the caller left it.
//! Sampled: one case runs on the game thread:
//!
//! 1. snapshot: the read/write-set globals (spec), extra globals, `this` (size from layouts.csv via the
//!    hook signature), pointed-to arguments, and copies of the watched windows (writable exe sections,
//!    the caller's stack above the arguments, optionally the used heap arena); plus side state outside
//!    game memory (rand counters, `_holdrand`, the arena bookkeeping, checkpointed lazily);
//! 2. run the ORIGINAL via [`l2_invoke`] (all registers, FPU environment and MXCSR as at the call site,
//!    arguments copied): capture EAX..EBP, bytes popped, the whole x87 stack (80 bits), CW/SW/MXCSR,
//!    the regions, every changed byte in the windows, the hooked callees entered and the arena blocks
//!    allocated;
//! 3. restore the snapshot (regions, changed window bytes, side state), run the RUST implementation the
//!    same way, capture the same;
//! 4. restore again and apply the leave run's results (`SA_L2_LEAVE`, default the original's), so the
//!    game continues exactly as if only that implementation had been called; the call returns that
//!    run's registers, x87 stack, FPU environment, MXCSR and argument words;
//! 5. compare (`sa_oracle_format::l2::compare`, normaliser in `sa_oracle_format::normalise`), report
//!    writes outside the declared write set (`undeclared_writes`), append the case to the spool.
//!
//! Split-run mode (`SA_L2_SPLIT`, INF-17, [`run_split`]): a sampled call runs one implementation only,
//! on the live state, and is recorded for `sa-oracle l2 split-report` (docs/re/l2-oracle.md).
//! The spool stops at `SA_L2_SPOOL_MAX` bytes with a trailer (INF-18, [`spool_write`]).
//!
//! Limits: memory outside the regions and windows that a run writes (heap objects other than `this` and
//! the pointer arguments, unless `SA_L2_WINDOW=all`) is neither restored nor compared; other threads
//! writing a watched window during a run would show up as writes of the function.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use sa_oracle_format::l2::{self as fmt, Case, Outcome, Region, RegionKind, Write};
use sa_oracle_format::subsys::{Mem, PoolMap};

use crate::l2::{Config, FnStats, Split, SpoolQuota, Window};
use crate::table::{self, HOOKS, Mode};
use crate::win::{self, log};
use sa_oracle_format::l2::OsProbe;

// ------------------------------------------------------------------ frame / invoke layouts ------

/// What `l2_common` builds on the stack (low to high addresses); the arguments follow.
#[repr(C)]
struct Frame {
    // pushad
    edi: u32,
    esi: u32,
    ebp: u32,
    esp: u32,
    ebx: u32,
    edx: u32,
    ecx: u32,
    eax: u32,
    eflags: u32,
    // out block (96 bytes), offsets used by l2_common
    st: [u8; 10], // +0
    _p0: [u8; 2],
    has_st0: u32,      // +12
    pop: u32,          // +16
    final_ecx: u32,    // +20
    mxcsr_out: u32,    // +24
    env_out: [u8; 28], // +28
    env_in: [u8; 28],  // +56
    mxcsr_in: u32,     // +84
    tail: u32,         // +88: nonzero = tail-jump there with the entry state
    _p1: u32,
    hook: u32,
    ret: u32,
}
const _: () = assert!(std::mem::size_of::<Frame>() == 140);
const _: () = assert!(std::mem::offset_of!(Frame, st) == 36);
const _: () = assert!(std::mem::offset_of!(Frame, tail) == 36 + 88);
const _: () = assert!(std::mem::offset_of!(Frame, hook) == 36 + 96);

/// The generic invoker's in/out block (offsets used by `l2_invoke`).
#[repr(C)]
struct Invoke {
    saved_esp: u32,      // 0
    nwords: u32,         // 4
    args: [u32; 16],     // 8
    regs_in: [u32; 7],   // 72: eax ecx edx ebx esi edi ebp
    regs_out: [u32; 7],  // 100
    esp_after: u32,      // 128
    cw: u16,             // 132
    sw: u16,             // 134
    mxcsr: u32,          // 136
    depth: u32,          // 140
    st: [[u8; 10]; 8],   // 144
    args_out: [u32; 16], // 224
    env_in: [u8; 28],    // 288
    mxcsr_in: u32,       // 316
    env_out: [u8; 28],   // 320
}
const _: () = assert!(std::mem::offset_of!(Invoke, regs_in) == 72);
const _: () = assert!(std::mem::offset_of!(Invoke, regs_out) == 100);
const _: () = assert!(std::mem::offset_of!(Invoke, esp_after) == 128);
const _: () = assert!(std::mem::offset_of!(Invoke, st) == 144);
const _: () = assert!(std::mem::offset_of!(Invoke, args_out) == 224);
const _: () = assert!(std::mem::offset_of!(Invoke, env_in) == 288);
const _: () = assert!(std::mem::offset_of!(Invoke, mxcsr_in) == 316);
const _: () = assert!(std::mem::offset_of!(Invoke, env_out) == 320);

const MAX_ARGS: usize = 16;

/// Target of the current invoke, the current invoke block and a scratch cell. Used only by the game
/// thread inside a case (cases never nest: calls made inside a case run take the tail path).
static INV_TARGET: AtomicU32 = AtomicU32::new(0);
static INV_CUR: AtomicU32 = AtomicU32::new(0);
static INV_TMP: AtomicU32 = AtomicU32::new(0);
/// `SA_L2_STACKFILL` (split-run mode): dwords below the invoker's stack pointer zeroed right before every
/// sampled call, so original code that reads uninitialised stack (e.g. `CWeapon::FireInstantHit`'s gunshell
/// direction at 0x740DAC) sees the same bytes in both runs instead of residue whose position depends on
/// the Rust frames above it. 0 = off.
pub(crate) static STACK_FILL_DWORDS: AtomicU32 = AtomicU32::new(0);

/// Shared entry of every hook's L2 stub (`push i; jmp l2_common`).
#[unsafe(naked)]
unsafe extern "C" fn l2_common() {
    std::arch::naked_asm!(
        "sub esp, 96",
        "fnstenv [esp+56]",
        "fldenv [esp+56]",
        "stmxcsr [esp+84]",
        "pushfd",
        "pushad",
        "cld",
        "push esp",
        "call {d}",
        "add esp, 4",
        "popad",
        "popfd",
        "cmp dword ptr [esp+88], 0",
        "jne 3f",
        "fldenv [esp+28]",
        "ldmxcsr [esp+24]",
        "cmp dword ptr [esp+12], 0",
        "je 2f",
        "fld tbyte ptr [esp]",
        "2:",
        // return to the caller, popping `pop` argument bytes: the return address moves up to the last
        // popped slot, ECX gets its final value from the slot below it.
        "mov ecx, [esp+16]",
        "lea ecx, [esp+ecx+100]",
        "push dword ptr [esp+100]",
        "pop dword ptr [ecx]",
        "push dword ptr [esp+20]",
        "pop dword ptr [ecx-4]",
        "lea esp, [ecx-4]",
        "pop ecx",
        "ret",
        // tail path: registers, flags, FPU and MXCSR are the caller's; jump to the target with the
        // stack exactly as at entry (the hook index slot receives the target, `ret` jumps there).
        "3:",
        "fldenv [esp+56]",
        "ldmxcsr [esp+84]",
        "push dword ptr [esp+88]",
        "pop dword ptr [esp+96]",
        "lea esp, [esp+96]",
        "ret",
        d = sym l2_dispatch,
    )
}

/// Calls `INV_TARGET` with the registers, stack arguments, FPU environment and MXCSR of `inv`, and
/// stores the registers, the x87 stack (drained), CW/SW/MXCSR, the FPU environment after draining and
/// the argument words afterwards.
#[unsafe(naked)]
unsafe extern "C" fn l2_invoke(inv: *mut Invoke) {
    std::arch::naked_asm!(
        "push ebp",
        "push ebx",
        "push esi",
        "push edi",
        "mov eax, [esp+20]",
        "mov [{cur}], eax",
        "mov [eax], esp",
        // optional stack fill (STACK_FILL_DWORDS): zero downwards from the current ESP, so the guard
        // page is touched in order
        "mov ecx, [{fill}]",
        "test ecx, ecx",
        "jz 8f",
        "push eax",
        "lea edi, [esp-4]",
        "xor eax, eax",
        "std",
        "rep stosd",
        "cld",
        "pop eax",
        "8:",
        "mov ecx, [eax+4]",
        "2:",
        "test ecx, ecx",
        "jz 3f",
        "push dword ptr [eax+ecx*4+4]",
        "dec ecx",
        "jmp 2b",
        "3:",
        "fldenv [eax+288]",
        "ldmxcsr [eax+316]",
        "mov ecx, [eax+76]",
        "mov edx, [eax+80]",
        "mov ebx, [eax+84]",
        "mov esi, [eax+88]",
        "mov edi, [eax+92]",
        "mov ebp, [eax+96]",
        "mov eax, [eax+72]",
        "call dword ptr [{target}]",
        "mov [{tmp}], eax",
        "mov eax, [{cur}]",
        "mov [eax+104], ecx",
        "mov [eax+108], edx",
        "mov [eax+112], ebx",
        "mov [eax+116], esi",
        "mov [eax+120], edi",
        "mov [eax+124], ebp",
        "mov ecx, [{tmp}]",
        "mov [eax+100], ecx",
        "mov [eax+128], esp",
        "fnstcw [eax+132]",
        "fnstsw [eax+134]",
        "stmxcsr [eax+136]",
        "mov esi, eax",
        "xor edx, edx",
        "lea ebx, [esi+144]",
        "4:",
        "cmp edx, 8",
        "je 5f",
        "fxam",
        "fnstsw ax",
        "and ax, 0x4500",
        "cmp ax, 0x4100",
        "je 5f",
        "fstp tbyte ptr [ebx]",
        "add ebx, 10",
        "inc edx",
        "jmp 4b",
        "5:",
        "mov [esi+140], edx",
        "fnstenv [esi+320]",
        "fldenv [esi+320]",
        // args_out[k] = [saved_esp - 4 * (n - k)]
        "mov ecx, [esi+4]",
        "mov edx, [esi]",
        "xor eax, eax",
        "6:",
        "cmp eax, ecx",
        "je 7f",
        "mov ebx, ecx",
        "sub ebx, eax",
        "neg ebx",
        "mov ebx, [edx+ebx*4]",
        "mov [esi+eax*4+224], ebx",
        "inc eax",
        "jmp 6b",
        "7:",
        "mov esp, [esi]",
        "pop edi",
        "pop esi",
        "pop ebx",
        "pop ebp",
        "ret",
        cur = sym INV_CUR,
        target = sym INV_TARGET,
        tmp = sym INV_TMP,
        fill = sym STACK_FILL_DWORDS,
    )
}

pub fn common_addr() -> u32 {
    l2_common as *const () as usize as u32
}

/// The 10-byte per-hook stub: `push i; jmp l2_common`.
pub fn stub_bytes(at: u32, i: usize) -> [u8; 10] {
    let mut b = [0u8; 10];
    b[0] = 0x68;
    b[1..5].copy_from_slice(&(i as u32).to_le_bytes());
    b[5..10].copy_from_slice(&crate::x86::jmp_rel32(at + 5, common_addr()));
    b
}

// ------------------------------------------------------------------ state ---------------------

static CFG: OnceLock<Config> = OnceLock::new();
static SPECS: OnceLock<BTreeMap<u32, Vec<fmt::Global>>> = OnceLock::new();
static STATS: Mutex<Vec<FnStats>> = Mutex::new(Vec::new());
static OFFTHREAD: Mutex<Vec<u64>> = Mutex::new(Vec::new());
static SPOOL: Mutex<Option<BufWriter<File>>> = Mutex::new(None);
static REPORT: Mutex<Option<BufWriter<File>>> = Mutex::new(None);
static IN_CASE: AtomicBool = AtomicBool::new(false);
static GAME_TID: AtomicU32 = AtomicU32::new(0);
static WINDOWS: OnceLock<Vec<(u32, u32)>> = OnceLock::new();
/// Free window snapshot buffer sets (one set per active case; split-run cases nest).
static BUFS: Mutex<Vec<Vec<Vec<u8>>>> = Mutex::new(Vec::new());
static QUOTA: Mutex<SpoolQuota> = Mutex::new(SpoolQuota {
    max: crate::l2::SPOOL_MAX_DEFAULT,
    written: 0,
    records: 0,
    hit: None,
});
/// Split-run: global sequence number of the next case.
static SEQ: AtomicU32 = AtomicU32::new(0);
static CASES_TOTAL: AtomicU32 = AtomicU32::new(0);
static POOLS: Mutex<(u32, PoolMap)> = Mutex::new((u32::MAX, PoolMap { pools: Vec::new() }));

fn cfg() -> &'static Config {
    CFG.get_or_init(Config::default)
}

fn tid() -> u32 {
    // SAFETY: plain Win32 call.
    unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() }
}

/// Writable sections of the loaded exe (from its PE headers): (start, end, name).
fn writable_sections() -> Vec<(u32, u32, String)> {
    let mut out = Vec::new();
    let base = 0x400000u32;
    let Some(pe) = win::read(base + 0x3C, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])) else {
        return out;
    };
    let Some(h) = win::read(base + pe, 24) else {
        return out;
    };
    let n = u16::from_le_bytes([h[6], h[7]]) as u32;
    let opt = u16::from_le_bytes([h[20], h[21]]) as u32;
    let sec = base + pe + 24 + opt;
    for k in 0..n {
        let Some(s) = win::read(sec + k * 40, 40) else {
            break;
        };
        let name = String::from_utf8_lossy(&s[..8])
            .trim_end_matches('\0')
            .to_string();
        let vs = u32::from_le_bytes([s[8], s[9], s[10], s[11]]);
        let va = u32::from_le_bytes([s[12], s[13], s[14], s[15]]);
        let ch = u32::from_le_bytes([s[36], s[37], s[38], s[39]]);
        if ch & 0x8000_0000 != 0 {
            out.push((base + va, base + va + ((vs + 0xFFF) & !0xFFF), name));
        }
    }
    out
}

/// Reads the configuration (attach, game threads suspended). Returns the hooks to switch to L2.
pub fn init(dir: &std::path::Path) -> Vec<usize> {
    let c = match Config::from_env(|k| std::env::var(k).ok()) {
        Ok(Some(c)) => c,
        // L2 off; mutants may still be requested (INF-9 L3 self-test).
        Ok(None) => match crate::l2::mutants_from_env(&|k: &str| std::env::var(k).ok()) {
            Ok(mutants) => {
                if !mutants.is_empty() {
                    log(&format!(
                        "l2: off; mutant Rust implementations {:?}",
                        mutants.iter().map(|&i| HOOKS[i].name).collect::<Vec<_>>()
                    ));
                }
                Config {
                    mutants,
                    ..Config::default()
                }
            }
            Err(e) => {
                log(&format!("l2: bad SA_L2_MUTANT ignored: {e}"));
                Config::default()
            }
        },
        Err(e) => {
            log(&format!("l2: bad configuration, L2 off: {e}"));
            Config::default()
        }
    };
    *STATS.lock().unwrap_or_else(|e| e.into_inner()) = vec![FnStats::default(); HOOKS.len()];
    *OFFTHREAD.lock().unwrap_or_else(|e| e.into_inner()) = vec![0; HOOKS.len()];
    crate::l2::set_mutants(&c.mutants);
    // The game thread until the first frame boundary confirms it: determinism mode's attach-time guess
    // (the process' only thread besides the injector's), so start-up calls before frame 1 (e.g.
    // CTimer::Initialise) are compared instead of passed through as off-thread.
    GAME_TID.store(crate::detwin::main_tid(), Ordering::Relaxed);
    // read/write sets
    let mut specs: BTreeMap<u32, Vec<fmt::Global>> = BTreeMap::new();
    if let Some(p) = &c.spec {
        match std::fs::read_to_string(p)
            .map_err(|e| e.to_string())
            .and_then(|t| fmt::parse_spec(&t))
        {
            Ok(v) => {
                for f in v {
                    let g: Vec<fmt::Global> = f
                        .globals
                        .into_iter()
                        .filter(|g| !c.drop.contains(&g.addr))
                        .collect();
                    specs.insert(f.addr, g);
                }
                log(&format!("l2: spec {p}: {} functions", specs.len()));
            }
            Err(e) => log(&format!("l2: spec {p}: {e}")),
        }
    }
    let _ = SPECS.set(specs);
    // windows
    let mut w: Vec<(u32, u32)> = Vec::new();
    let secs = writable_sections();
    match c.window {
        Window::None => {}
        Window::Data => {
            // The game's own data: .data/.bss (0x8A4000), _TEXT_HA, _rwdseg; not the no-CD sections.
            for (a, b, name) in &secs {
                if *a < 0xCB1000 {
                    let _ = name;
                    w.push((*a, *b));
                }
            }
        }
        Window::All => w.extend(secs.iter().map(|(a, b, _)| (*a, *b))),
    }
    w.extend(c.windows.iter().copied());
    w.retain(|&(a, b)| win::readable(a, (b - a) as usize));
    log(&format!(
        "l2: windows {:?} (sections {:?})",
        w.iter()
            .map(|(a, b)| format!("{a:#x}-{b:#x}"))
            .collect::<Vec<_>>(),
        secs
    ));
    let _ = WINDOWS.set(w);
    if !c.hooks.is_empty() {
        let out = c
            .out
            .clone()
            .unwrap_or_else(|| dir.join("l2.spool").to_string_lossy().into_owned());
        match File::create(&out) {
            Ok(f) => {
                let mut b = BufWriter::with_capacity(1 << 20, f);
                let _ = b.write_all(fmt::SPOOL_MAGIC);
                *SPOOL.lock().unwrap_or_else(|e| e.into_inner()) = Some(b);
                *QUOTA.lock().unwrap_or_else(|e| e.into_inner()) =
                    SpoolQuota::new(c.spool_max, fmt::SPOOL_MAGIC.len() as u64);
            }
            Err(e) => log(&format!("l2: spool {out}: {e}")),
        }
        match File::create(format!("{out}.txt")) {
            Ok(f) => *REPORT.lock().unwrap_or_else(|e| e.into_inner()) = Some(BufWriter::new(f)),
            Err(e) => log(&format!("l2: report {out}.txt: {e}")),
        }
        log(&format!(
            "l2: on for {:?}, max {} rate {} per_frame {} from {} leave {} mutants {:?}{}; spool {out} (quota {} bytes)",
            c.hooks.iter().map(|&i| HOOKS[i].name).collect::<Vec<_>>(),
            c.max,
            c.rate,
            c.per_frame,
            c.from_frame,
            if c.leave_rust { "rust" } else { "orig" },
            c.mutants.iter().map(|&i| HOOKS[i].name).collect::<Vec<_>>(),
            match c.split {
                Some(Split::Orig) => "; SPLIT-RUN recording of the original",
                Some(Split::Rust) => "; SPLIT-RUN recording of the Rust implementations",
                None => "",
            },
            c.spool_max
        ));
    }
    let hooks = c.hooks.clone();
    if c.split.is_some() && c.stack_fill > 0 {
        STACK_FILL_DWORDS.store(c.stack_fill / 4, Ordering::Release);
        log(&format!("l2: split-run stack fill: {:#x} bytes below the invoker zeroed before every sampled call", c.stack_fill));
    }
    let _ = CFG.set(c);
    // INF-10 coverage arms its breakpoints here: this runs after every hook and determinism patch is
    // installed and before the game's threads resume (kept out of win.rs, see coverage.rs).
    crate::coverage::init(dir);
    hooks
}

/// Frame boundary (game thread): remembers (confirms) the game thread.
pub fn on_boundary() {
    GAME_TID.store(tid(), Ordering::Relaxed);
}

fn leave_mode() -> Mode {
    if cfg().leave_rust {
        Mode::Rust
    } else {
        Mode::Original
    }
}

fn leave_target(i: usize) -> u32 {
    if cfg().leave_rust {
        table::rust_entry(i)
    } else {
        table::original_at(i)
    }
}

// ------------------------------------------------------------------ dispatch --------------------

extern "C" fn l2_dispatch(f: *mut Frame) {
    // SAFETY: l2_common passes its stack frame.
    let f = unsafe { &mut *f };
    let i = f.hook as usize;
    f.tail = 0;
    let r = catch_unwind(AssertUnwindSafe(|| dispatch(f, i)));
    if r.is_err() {
        log(&format!(
            "l2: {}: dispatch failed; passing the call through",
            HOOKS[i].name
        ));
        IN_CASE.store(false, Ordering::Release);
        f.tail = leave_target(i);
    }
}

fn dispatch(f: &mut Frame, i: usize) {
    let Some(sig) = HOOKS.get(i).and_then(|h| h.sig) else {
        f.tail = table::original_at(i);
        return;
    };
    let me = tid();
    if me != GAME_TID.load(Ordering::Relaxed) {
        if let Ok(mut v) = OFFTHREAD.lock() {
            v[i] += 1;
        }
        f.tail = leave_target(i);
        return;
    }
    let c = cfg();
    if c.split.is_none() && IN_CASE.load(Ordering::Acquire) {
        f.tail = leave_target(i);
        return;
    }
    let frame = win::frames();
    let (sample, retire, ordinal) = {
        let mut st = STATS.lock().unwrap_or_else(|e| e.into_inner());
        let s = &mut st[i];
        let sample = s.sample(c, frame);
        let ordinal = s.cases;
        if sample && c.split.is_some() && (sig.args as usize) <= MAX_ARGS {
            // Split-run cases may nest: the ordinal is taken at entry.
            s.cases += 1;
        }
        (sample, s.cases >= c.max && !s.retired, ordinal)
    };
    if retire {
        // Budget used up: the hook goes to the leave mode for the rest of the run.
        STATS.lock().unwrap_or_else(|e| e.into_inner())[i].retired = true;
        table::set_mode_now(i, leave_mode());
        log(&format!(
            "l2: frame {frame}: {} retired after {} cases",
            HOOKS[i].name, c.max
        ));
    }
    if !sample || sig.args as usize > MAX_ARGS {
        f.tail = leave_target(i);
        return;
    }
    if let Some(split) = c.split {
        run_split(f, i, &sig, frame, ordinal, split);
        return;
    }
    IN_CASE.store(true, Ordering::Release);
    run_case(f, i, &sig, frame);
    IN_CASE.store(false, Ordering::Release);
}

fn new_invoke(f: &Frame, args: &[u32]) -> Invoke {
    let mut a = [0u32; MAX_ARGS];
    a[..args.len()].copy_from_slice(args);
    Invoke {
        saved_esp: 0,
        nwords: args.len() as u32,
        args: a,
        regs_in: [f.eax, f.ecx, f.edx, f.ebx, f.esi, f.edi, f.ebp],
        regs_out: [0; 7],
        esp_after: 0,
        cw: 0,
        sw: 0,
        mxcsr: 0,
        depth: 0,
        st: [[0; 10]; 8],
        args_out: [0; MAX_ARGS],
        env_in: f.env_in,
        mxcsr_in: f.mxcsr_in,
        env_out: [0; 28],
    }
}

fn invoke(inv: &mut Invoke, target: u32) {
    // A nested split-run case runs inside this call and sets INV_CUR to its own block; the outer
    // `l2_invoke` reads INV_CUR after its call returns, so the outer block is put back afterwards.
    let outer = INV_CUR.load(Ordering::Acquire);
    INV_TARGET.store(target, Ordering::Release);
    // SAFETY: `target` has the hook's signature; `inv` carries its arguments.
    unsafe { l2_invoke(inv) };
    INV_CUR.store(outer, Ordering::Release);
}

fn outcome(inv: &Invoke, calls: Vec<u32>, allocs: Vec<(u32, u32)>, writes: Vec<Write>) -> Outcome {
    let n = inv.nwords as usize;
    let first_arg = inv.saved_esp.wrapping_sub(4 * n as u32);
    Outcome {
        regs: inv.regs_out,
        pop: inv.esp_after.wrapping_sub(first_arg),
        st: inv.st[..(inv.depth as usize).min(8)].to_vec(),
        cw: inv.cw,
        sw: inv.sw,
        mxcsr: inv.mxcsr,
        writes,
        calls,
        allocs,
        args_out: inv.args_out[..n].to_vec(),
    }
}

/// Fills the frame so `l2_common` returns `inv`'s results to the caller.
fn finish_frame(f: &mut Frame, inv: &Invoke, o: &Outcome, args_ptr: u32) {
    let [eax, ecx, edx, ebx, esi, edi, ebp] = inv.regs_out;
    (f.eax, f.ecx, f.edx, f.ebx, f.esi, f.edi, f.ebp) = (eax, ecx, edx, ebx, esi, edi, ebp);
    f.final_ecx = ecx;
    f.pop = o.pop;
    if inv.depth > 1 {
        log(&format!(
            "l2: x87 depth {} after the call; only ST0 is returned",
            inv.depth
        ));
    }
    f.has_st0 = (inv.depth > 0) as u32;
    f.st = inv.st[0];
    f.env_out = inv.env_out;
    f.mxcsr_out = inv.mxcsr;
    // The argument words as the callee left them (cdecl callees may write their arguments).
    for (k, w) in o.args_out.iter().enumerate() {
        put(args_ptr + 4 * k as u32, &w.to_le_bytes());
    }
}

/// Exe image (all sections mapped for the whole run): reads there need no VirtualQuery.
const IMAGE: (u32, u32) = (0x400000, 0x1577000);

/// `win::read` with a fast path for the exe image.
fn mem_read(addr: u32, len: usize) -> Option<Vec<u8>> {
    if addr >= IMAGE.0 && addr as u64 + len as u64 <= IMAGE.1 as u64 {
        let mut v = vec![0u8; len];
        // SAFETY: inside the mapped exe image.
        unsafe { std::ptr::copy_nonoverlapping(addr as usize as *const u8, v.as_mut_ptr(), len) };
        return Some(v);
    }
    win::read(addr, len)
}

fn put(addr: u32, b: &[u8]) {
    // SAFETY: only ranges that were read before (regions, windows, argument slots) are written back.
    unsafe { std::ptr::copy_nonoverlapping(b.as_ptr(), addr as usize as *mut u8, b.len()) };
}

/// Writes `want` at `addr` only where it differs (read-only pages are never touched by a no-op).
fn put_diff(addr: u32, want: &[u8]) {
    if let Some(cur) = mem_read(addr, want.len())
        && cur != want
    {
        put(addr, want);
    }
}

struct GameMem;
impl Mem for GameMem {
    fn read(&self, addr: u32, out: &mut [u8]) -> bool {
        match mem_read(addr, out.len()) {
            Some(b) => {
                out.copy_from_slice(&b);
                true
            }
            None => false,
        }
    }
}

/// Changed bytes of `[start, start + snap.len())` against the snapshot, as merged runs (new values).
fn window_diff(start: u32, snap: &[u8]) -> Vec<Write> {
    // SAFETY: the window was readable when it was snapshotted (image sections / committed stack /
    // arena range); it stays mapped.
    let live = unsafe { std::slice::from_raw_parts(start as usize as *const u8, snap.len()) };
    let mut out: Vec<Write> = Vec::new();
    let mut i = 0usize;
    let n = snap.len();
    while i < n {
        // memcmp over 4 KiB blocks, then 64-byte lines, then bytes
        if i.is_multiple_of(4096) && i + 4096 <= n && live[i..i + 4096] == snap[i..i + 4096] {
            i += 4096;
            continue;
        }
        if i + 64 <= n && live[i..i + 64] == snap[i..i + 64] {
            i += 64;
            continue;
        }
        let end = (i + 64).min(n);
        for k in i..end {
            if live[k] != snap[k] {
                let a = start + k as u32;
                match out.last_mut() {
                    Some(w) if w.addr + w.bytes.len() as u32 == a => w.bytes.push(live[k]),
                    _ => out.push(Write {
                        addr: a,
                        bytes: vec![live[k]],
                    }),
                }
            }
        }
        i = end;
    }
    out
}

struct Snap {
    windows: Vec<(u32, u32)>,
    bufs: Vec<Vec<u8>>,
}

impl Drop for Snap {
    fn drop(&mut self) {
        let b = std::mem::take(&mut self.bufs);
        BUFS.lock().unwrap_or_else(|e| e.into_inner()).push(b);
    }
}

impl Snap {
    fn take(windows: Vec<(u32, u32)>) -> Snap {
        let mut bufs = BUFS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pop()
            .unwrap_or_default();
        bufs.resize_with(windows.len(), Vec::new);
        for (k, &(a, b)) in windows.iter().enumerate() {
            let len = (b - a) as usize;
            let buf = &mut bufs[k];
            buf.resize(len, 0);
            // SAFETY: the window is readable (checked by the caller).
            unsafe { std::ptr::copy_nonoverlapping(a as usize as *const u8, buf.as_mut_ptr(), len) };
        }
        Snap { windows, bufs }
    }
    fn diff(&self) -> Vec<Write> {
        let mut out = Vec::new();
        for (k, &(a, _)) in self.windows.iter().enumerate() {
            out.extend(window_diff(a, &self.bufs[k]));
        }
        out
    }
    /// Hashes of every 4 KiB page of the first `n` windows (split-run input fingerprint). Words whose
    /// value lies in the OS heap range (`mask`: below the determinism arena, outside the image and the
    /// stack; the same decision in both runs, it depends on the value only) or in the thread's stack
    /// (`stack`) are replaced by a marker first. Those that point into committed OS memory now (`fm`,
    /// OS object addresses) and every stack address are listed in `foreign_words` as (address, value)
    /// pairs: the report shows which OS words differed and compares the stack words through
    /// normalise.rs `Stack` (a Rust caller's frame layout moves them; HARNESS-3: the `CEntity*` slots
    /// that CReferences 0xB9B9A8 points to, PORT-WEAPON-2 DoBulletImpact PASS-FP).
    fn page_hashes(
        &self,
        n: usize,
        mask: &impl Fn(u32) -> bool,
        stack: (u32, u32),
        fm: &ForeignMap,
        foreign_words: &mut Vec<u32>,
    ) -> Vec<(u32, u64)> {
        let mut out = Vec::new();
        let mut tmp = vec![0u8; 4096];
        for (k, &(a, _)) in self.windows.iter().enumerate().take(n) {
            for (j, page) in self.bufs[k].chunks(4096).enumerate() {
                let pa = a + 4096 * j as u32;
                let mut hit = false;
                for (w, b) in page.as_chunks::<4>().0.iter().enumerate() {
                    let v = u32::from_le_bytes(*b);
                    let on_stack = (stack.0..stack.1).contains(&v);
                    if mask(v) || on_stack {
                        if !hit {
                            tmp[..page.len()].copy_from_slice(page);
                            hit = true;
                        }
                        tmp[4 * w..4 * w + 4].copy_from_slice(&FOREIGN_MARK.to_le_bytes());
                        if on_stack || fm.contains(v) {
                            foreign_words.push(pa + 4 * w as u32);
                            foreign_words.push(v);
                        }
                    }
                }
                let h = if hit {
                    fmt::page_hash(pa, &tmp[..page.len()])
                } else {
                    fmt::page_hash(pa, page)
                };
                out.push((pa, h));
            }
        }
        out
    }
    /// Diagnostic dump of the first `n` windows: per window `[u32 start][u32 len][bytes]`.
    fn dump(&self, n: usize, path: &str) {
        let mut b = Vec::new();
        for (k, &(a, _)) in self.windows.iter().enumerate().take(n) {
            b.extend_from_slice(&a.to_le_bytes());
            b.extend_from_slice(&(self.bufs[k].len() as u32).to_le_bytes());
            b.extend_from_slice(&self.bufs[k]);
        }
        match std::fs::write(path, b) {
            Ok(()) => log(&format!("l2: window dump {path}")),
            Err(e) => log(&format!("l2: window dump {path}: {e}")),
        }
    }
    /// The snapshot's bytes for `[addr, addr + len)` (inside one window).
    fn pre(&self, addr: u32, len: usize) -> Vec<u8> {
        for (k, &(a, b)) in self.windows.iter().enumerate() {
            if addr >= a && addr as u64 + len as u64 <= b as u64 {
                let o = (addr - a) as usize;
                return self.bufs[k][o..o + len].to_vec();
            }
        }
        Vec::new()
    }
    fn restore(&self, writes: &[Write]) {
        for w in writes {
            let pre = self.pre(w.addr, w.bytes.len());
            if pre.len() == w.bytes.len() {
                put(w.addr, &pre);
            }
        }
    }
}

fn read_regions(regions: &mut [Region], orig: bool) {
    for r in regions.iter_mut().filter(|r| r.readable && r.has_pre_memory()) {
        let b = mem_read(r.addr, r.pre.len()).unwrap_or_default();
        if orig {
            r.post_orig = b;
        } else {
            r.post_rust = b;
        }
    }
}

fn restore_regions(regions: &[Region]) {
    for r in regions.iter().filter(|r| r.readable && r.has_pre_memory()) {
        put_diff(r.addr, &r.pre);
    }
}

/// The case's regions before the call: spec globals, extra regions evaluated at entry, `this`,
/// pointed-to arguments, and the pre-call OS probes (`os:` extras without `post:`).
fn build_regions(f: &Frame, i: usize, sig: &fmt::Sig, args: &[u32]) -> Vec<Region> {
    let c = cfg();
    let h = &HOOKS[i];
    let mut regions: Vec<Region> = Vec::new();
    fn add(regions: &mut Vec<Region>, addr: u32, len: u32, kind: RegionKind, access: u8) {
        let pre = if len > 0 {
            mem_read(addr, len as usize)
        } else {
            None
        };
        regions.push(Region {
            addr,
            kind,
            access,
            readable: pre.is_some(),
            pre: pre.unwrap_or_default(),
            post_orig: Vec::new(),
            post_rust: Vec::new(),
        });
    }
    if let Some(gs) = SPECS.get().and_then(|s| s.get(&h.addr)) {
        for g in gs {
            add(
                &mut regions,
                g.addr,
                g.len,
                RegionKind::Global { exact: g.exact },
                g.access,
            );
        }
    }
    let regs_in = [f.eax, f.ecx, f.edx, f.ebx, f.esi, f.edi, f.ebp];
    for e in c
        .extra
        .iter()
        .filter(|e| e.hook.is_none_or(|h| h == i) && !e.post)
    {
        if let Some(p) = e.os {
            let a = e.addr.eval(&regs_in, args, &rd32).unwrap_or(0);
            regions.push(Region {
                addr: a,
                kind: RegionKind::Os(p),
                access: 0,
                readable: true,
                pre: probe(p, a),
                post_orig: Vec::new(),
                post_rust: Vec::new(),
            });
            continue;
        }
        match (
            e.addr.eval(&regs_in, args, &rd32),
            e.len.eval(&regs_in, args, &rd32),
        ) {
            // declared write range: no bytes recorded
            (Some(a), Some(l)) if e.acc == crate::l2::ACC_DECL => regions.push(Region {
                addr: a,
                kind: RegionKind::Decl(l),
                access: fmt::ACC_W,
                readable: true,
                pre: Vec::new(),
                post_orig: Vec::new(),
                post_rust: Vec::new(),
            }),
            // at most 16 MiB per region
            (Some(a), Some(l)) => add(&mut regions, a, l.min(1 << 24), RegionKind::Extra, e.acc),
            _ => add(&mut regions, 0, 0, RegionKind::Extra, e.acc), // counted as unreadable
        }
    }
    if sig.conv == fmt::Conv::Thiscall && sig.this_size > 0 {
        add(
            &mut regions,
            f.ecx,
            sig.this_size,
            RegionKind::This,
            fmt::ACC_R | fmt::ACC_W,
        );
    }
    for p in sig.ptr_args {
        if let Some(&a) = args.get(p.arg as usize) {
            add(
                &mut regions,
                a,
                p.size,
                RegionKind::ArgPtr(p.arg),
                fmt::ACC_R | fmt::ACC_W,
            );
        }
    }
    regions
}

fn rd32(a: u32) -> Option<u32> {
    mem_read(a, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// After a run: the OS probes again (same object as before the call) and the `post:` extras,
/// evaluated with the run's exit registers. The first run (`first`) appends the post regions; the
/// second run (double-run L2) reads them at the addresses the first run found.
fn capture_post(regions: &mut Vec<Region>, i: usize, regs_out: &[u32; 7], args: &[u32], first: bool) {
    for r in regions.iter_mut() {
        if let RegionKind::Os(p) = r.kind
            && !r.pre.is_empty()
        {
            let b = probe(p, r.addr);
            if first {
                r.post_orig = b;
            } else {
                r.post_rust = b;
            }
        }
    }
    let c = cfg();
    let posts: Vec<&crate::l2::Extra> = c
        .extra
        .iter()
        .filter(|e| e.hook.is_none_or(|h| h == i) && e.post)
        .collect();
    if posts.is_empty() {
        return;
    }
    if first {
        for e in posts {
            let a = e.addr.eval(regs_out, args, &rd32);
            if let Some(p) = e.os {
                let a = a.unwrap_or(0);
                regions.push(Region {
                    addr: a,
                    kind: RegionKind::Os(p),
                    access: 0,
                    readable: true,
                    pre: Vec::new(),
                    post_orig: probe(p, a),
                    post_rust: Vec::new(),
                });
                continue;
            }
            let l = e.len.eval(regs_out, args, &rd32);
            let b = match (a, l) {
                (Some(a), Some(l)) if l > 0 => mem_read(a, l.min(1 << 24) as usize),
                _ => None,
            };
            regions.push(Region {
                addr: a.unwrap_or(0),
                kind: RegionKind::Post,
                access: e.acc,
                readable: b.is_some(),
                pre: Vec::new(),
                post_orig: b.unwrap_or_default(),
                post_rust: Vec::new(),
            });
        }
    } else {
        for r in regions.iter_mut() {
            match r.kind {
                RegionKind::Post if r.readable => {
                    r.post_rust = mem_read(r.addr, r.post_orig.len()).unwrap_or_default();
                }
                RegionKind::Os(p) if r.pre.is_empty() => r.post_rust = probe(p, r.addr),
                _ => {}
            }
        }
    }
}

// ------------------------------------------------------------------ OS probes -----------------

/// `_nhandle` and `__pioinfo` of the exe's static CRT (read from `_get_osfhandle` 0x833D2C:
/// `cmp eax, [0xC9C2FC]`; `mov ecx, [ecx*4 + 0xC9C300]`; ioinfo stride 0x24, osfhnd +0, osfile +4).
const CRT_NHANDLE: u32 = 0xC9C2FC;
const CRT_PIOINFO: u32 = 0xC9C300;
const CRT_IOINFO_SIZE: u32 = 0x24;
/// sizeof(FILE) in the static CRT (_ptr _cnt _base _flag _file _charbuf _bufsiz _tmpfname).
const CRT_FILE_SIZE: usize = 0x20;

fn probe(p: OsProbe, addr: u32) -> Vec<u8> {
    match p {
        OsProbe::File => probe_file(addr).encode(),
        OsProbe::Cwd => std::env::current_dir()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_else(|e| format!("? {e}"))
            .into_bytes(),
        OsProbe::Cursor => {
            use windows_sys::Win32::UI::WindowsAndMessaging::{CURSORINFO, GetCursorInfo};
            // SAFETY: plain Win32 call with a sized out struct.
            let mut ci: CURSORINFO = unsafe { std::mem::zeroed() };
            ci.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
            let ok = unsafe { GetCursorInfo(&mut ci) } != 0;
            let mut v = vec![ok as u8];
            v.extend_from_slice(&ci.flags.to_le_bytes());
            v
        }
    }
}

/// The CRT stream at `fp` and the OS file behind its descriptor. Reads only: the OS position is
/// queried with a zero `FILE_CURRENT` move, the handle value is not recorded.
fn probe_file(fp: u32) -> fmt::FileProbe {
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_CURRENT, GetFileSizeEx, GetFinalPathNameByHandleW, SetFilePointerEx,
    };
    let mut out = fmt::FileProbe {
        fd: -1,
        ..Default::default()
    };
    let Some(file) = (fp != 0).then(|| mem_read(fp, CRT_FILE_SIZE)).flatten() else {
        return out;
    };
    out.fd = i32::from_le_bytes([file[16], file[17], file[18], file[19]]);
    out.file = file;
    let n = rd32(CRT_NHANDLE).unwrap_or(0);
    if out.fd < 0 || out.fd as u32 >= n {
        return out;
    }
    let fd = out.fd as u32;
    let Some(block) = rd32(CRT_PIOINFO + 4 * (fd >> 5)).filter(|&b| b != 0) else {
        return out;
    };
    let io = block + (fd & 31) * CRT_IOINFO_SIZE;
    out.osfile = mem_read(io + 4, 1).map(|b| b[0]).unwrap_or(0);
    if out.osfile & 1 == 0 {
        return out;
    }
    let Some(h) = rd32(io) else { return out };
    let h = h as usize as windows_sys::Win32::Foundation::HANDLE;
    let mut pos = 0i64;
    let mut size = 0i64;
    let mut buf = [0u16; 1024];
    // SAFETY: plain Win32 calls on the CRT's open handle; out pointers are local.
    let (okp, oks, n) = unsafe {
        (
            SetFilePointerEx(h, 0, &mut pos, FILE_CURRENT) != 0,
            GetFileSizeEx(h, &mut size) != 0,
            GetFinalPathNameByHandleW(h, buf.as_mut_ptr(), buf.len() as u32, 0),
        )
    };
    let path = if n > 0 && (n as usize) < buf.len() {
        String::from_utf16_lossy(&buf[..n as usize])
    } else {
        "?".into()
    };
    out.os = Some((
        if okp { pos as u64 } else { u64::MAX },
        if oks { size as u64 } else { u64::MAX },
        path,
    ));
    out
}

fn run_case(f: &mut Frame, i: usize, sig: &fmt::Sig, frame: u32) {
    let t0 = std::time::Instant::now();
    let c = cfg();
    let args_ptr = f as *mut Frame as usize as u32 + std::mem::size_of::<Frame>() as u32;
    let n = sig.args as usize;
    let args: Vec<u32> = mem_read(args_ptr, 4 * n)
        .map(|b| {
            b.as_chunks::<4>()
                .0
                .iter()
                .map(|w| u32::from_le_bytes(*w))
                .collect()
        })
        .unwrap_or_default();
    if args.len() != n {
        f.tail = leave_target(i);
        return;
    }

    // ---- regions
    let regions = build_regions(f, i, sig, &args);
    let unreadable = regions.iter().filter(|r| !r.readable).count() as u32;

    // ---- windows: static ones + the caller's stack above the arguments (+ the used arena)
    let mut windows = WINDOWS.get().cloned().unwrap_or_default();
    let stack_lo = args_ptr + 4 * n as u32;
    if c.stack > 0 && win::readable(stack_lo, c.stack as usize) {
        windows.push((stack_lo, stack_lo + c.stack));
    }
    if c.window == Window::All
        && let Some((a, b)) = crate::detwin::arena_extent()
        && b > a
        && win::readable(a, (b - a) as usize)
    {
        windows.push((a, b));
    }
    let snap = Snap::take(windows.clone());
    let side_pre = crate::detwin::side_save();
    crate::detwin::l2_case_begin();

    // ---- original
    let mut inv_o = new_invoke(f, &args);
    crate::l2::run_begin(i as u32);
    invoke(&mut inv_o, table::original_at(i));
    let (calls_o, allocs_o) = crate::l2::run_end();
    let mut regions = regions;
    read_regions(&mut regions, true);
    capture_post(&mut regions, i, &inv_o.regs_out, &args, true);
    let writes_o = snap.diff();
    let cp = crate::detwin::l2_checkpoint();
    let mut side_o = crate::detwin::side_save();
    side_o.arena = cp.as_ref().and_then(|_| crate::detwin::arena_clone());
    let out_o = outcome(&inv_o, calls_o, allocs_o, writes_o);
    // back to the snapshot
    snap.restore(&out_o.writes);
    restore_regions(&regions);
    let mut side_back = side_pre.clone();
    side_back.arena = cp;
    crate::detwin::side_restore(&side_back);

    // ---- Rust
    let mut inv_r = new_invoke(f, &args);
    crate::l2::run_begin(i as u32);
    invoke(&mut inv_r, table::rust_entry(i));
    let (calls_r, allocs_r) = crate::l2::run_end();
    read_regions(&mut regions, false);
    capture_post(&mut regions, i, &inv_r.regs_out, &args, false);
    let writes_r = snap.diff();
    let cp = crate::detwin::l2_case_end();
    let out_r = outcome(&inv_r, calls_r, allocs_r, writes_r);

    // ---- leave state
    if !c.leave_rust {
        snap.restore(&out_r.writes);
        restore_regions(&regions);
        for w in &out_o.writes {
            put(w.addr, &w.bytes);
        }
        for r in regions.iter().filter(|r| r.readable && r.has_pre_memory()) {
            put_diff(r.addr, &r.post_orig);
        }
        let mut s = side_o;
        if s.arena.is_none() {
            s.arena = cp; // the original left the arena untouched; Rust's changes are undone
        }
        crate::detwin::side_restore(&s);
        finish_frame(f, &inv_o, &out_o, args_ptr);
    } else {
        finish_frame(f, &inv_r, &out_r, args_ptr);
    }

    // ---- record
    let window_pre = pre_of_writes(&snap, &[&out_o.writes, &out_r.writes], false);
    let ordinal = STATS.lock().map(|s| s[i].cases).unwrap_or(0);
    let mut case = new_case(f, i, sig, frame, ordinal, &args, regions, windows, window_pre);
    case.orig = out_o;
    case.rust = out_r;
    case.leave = c.leave_rust as u8;
    case.mutant = c.mutants.contains(&i);
    drop(snap);
    record(case, i, t0, unreadable, true);
}

/// The window bytes before the call at every address one of the runs wrote (`words`: the whole
/// aligned words around them, for split-run records, whose two runs are compared word-wise later).
fn pre_of_writes(snap: &Snap, runs: &[&Vec<Write>], words: bool) -> Vec<Write> {
    let mut addrs: BTreeMap<u32, ()> = BTreeMap::new();
    for w in runs.iter().flat_map(|r| r.iter()) {
        for k in 0..w.bytes.len() as u32 {
            let a = w.addr + k;
            if words {
                for b in (a & !3)..(a & !3) + 4 {
                    if !snap.pre(b, 1).is_empty() {
                        addrs.insert(b, ());
                    }
                }
            } else {
                addrs.insert(a, ());
            }
        }
    }
    let mut window_pre: Vec<Write> = Vec::new();
    for &a in addrs.keys() {
        let b = snap.pre(a, 1).first().copied().unwrap_or(0);
        match window_pre.last_mut() {
            Some(w) if w.addr + w.bytes.len() as u32 == a => w.bytes.push(b),
            _ => window_pre.push(Write {
                addr: a,
                bytes: vec![b],
            }),
        }
    }
    window_pre
}

#[allow(clippy::too_many_arguments)]
fn new_case(
    f: &Frame,
    i: usize,
    sig: &fmt::Sig,
    frame: u32,
    ordinal: u32,
    args: &[u32],
    regions: Vec<Region>,
    windows: Vec<(u32, u32)>,
    window_pre: Vec<Write>,
) -> Case {
    let h = &HOOKS[i];
    // Pool headers, read once per frame (pools are only created at start-up).
    let pools = {
        let mut g = POOLS.lock().unwrap_or_else(|e| e.into_inner());
        if g.0 != frame || g.1.pools.is_empty() {
            *g = (frame, PoolMap::read(&GameMem));
        }
        g.1.clone()
    };
    let mut env = f.env_in.to_vec();
    env.truncate(28);
    Case {
        fn_addr: h.addr,
        name: h.name.to_string(),
        conv: sig.conv as u8,
        ret: sig.ret as u8,
        frame,
        ordinal,
        caller: f.ret,
        regs_in: [f.eax, f.ecx, f.edx, f.ebx, f.esi, f.edi, f.ebp],
        args: args.to_vec(),
        fpu_env: env,
        mxcsr: f.mxcsr_in,
        regions,
        windows,
        window_pre,
        image: (0x400000, 0x1577000),
        pools: pools
            .pools
            .iter()
            .map(|p| (p.objects, p.bytemap, p.size))
            .collect(),
        ..Case::default()
    }
}

/// Statistics, text report and spool for one case. `compare = false` for split-run records (one
/// outcome only; compared on the host by `sa-oracle l2 split-report`).
fn record(case: Case, i: usize, t0: std::time::Instant, unreadable: u32, compare: bool) {
    let h = &HOOKS[i];
    let (frame, ordinal) = (case.frame, case.ordinal);
    let diffs = if compare { fmt::compare(&case) } else { Vec::new() };
    let und = fmt::undeclared_writes(&case);
    let ledger = !diffs.is_empty() && is_ledger(h.addr, &case, &diffs);
    {
        let mut st = STATS.lock().unwrap_or_else(|e| e.into_inner());
        let s = &mut st[i];
        if compare {
            s.cases += 1; // split-run cases are counted at entry (they nest)
        }
        s.us += t0.elapsed().as_micros() as u64;
        s.unreadable_regions += unreadable;
        if !diffs.is_empty() {
            if ledger {
                s.ledger += 1;
            } else {
                s.mismatches += 1;
                if s.first_mismatch.is_none() {
                    s.first_mismatch = Some(format!(
                        "frame {frame} caller {:#x} args {}: {}",
                        case.caller,
                        args_str(&case.args),
                        diffs
                            .iter()
                            .map(|d| format!("{}: {}", d.field, d.detail))
                            .collect::<Vec<_>>()
                            .join(" | ")
                    ));
                }
            }
        }
        if !und.is_empty() {
            s.undeclared_cases += 1;
            for u in &und {
                let e = s.undeclared.entry((u.addr, u.len)).or_insert((0, String::new()));
                e.0 += 1;
                if !e.1.contains(u.run) {
                    if !e.1.is_empty() {
                        e.1.push('+');
                    }
                    e.1.push_str(u.run);
                }
            }
        }
    }
    CASES_TOTAL.fetch_add(1, Ordering::Relaxed);
    if (!diffs.is_empty() || !und.is_empty())
        && let Ok(mut g) = REPORT.lock()
        && let Some(w) = g.as_mut()
    {
        let _ = writeln!(
            w,
            "{} {} frame {frame} ordinal {ordinal} caller {:#x} args {} id {}",
            if diffs.is_empty() {
                "UNDECLARED"
            } else if ledger {
                "LEDGER"
            } else {
                "MISMATCH"
            },
            h.name,
            case.caller,
            args_str(&case.args),
            case.content_id()
        );
        for d in &diffs {
            let _ = writeln!(w, "    {}: {}", d.field, d.detail);
        }
        for u in &und {
            let _ = writeln!(
                w,
                "    write outside the declared write set ({}): {:#x}+{}{}",
                u.run,
                u.addr,
                u.len,
                u.near_unbounded
                    .map(|b| format!(" (within 64 KiB of unbounded {b:#x})"))
                    .unwrap_or_default()
            );
        }
    }
    spool_write(&case, h.name, frame);
}

/// Appends a case to the spool unless the quota (INF-18) is reached; at the first refusal the
/// trailer is written and the stop is logged.
fn spool_write(case: &Case, hook: &str, frame: u32) {
    let rec = fmt::spool_record(case);
    let Ok(mut g) = SPOOL.lock() else { return };
    let Some(w) = g.as_mut() else { return };
    let mut q = QUOTA.lock().unwrap_or_else(|e| e.into_inner());
    if q.admit(rec.len() as u64, hook, frame) {
        let _ = w.write_all(&rec);
        return;
    }
    if let Some(m) = q.message()
        && q.hit.as_ref().is_some_and(|h| h.0 == hook && h.1 == frame)
        && !QUOTA_LOGGED.swap(true, Ordering::AcqRel)
    {
        let _ = w.write_all(&fmt::spool_trailer(&m));
        let _ = w.flush();
        log(&format!("l2: {m}"));
    }
}

static QUOTA_LOGGED: AtomicBool = AtomicBool::new(false);

// ------------------------------------------------------------------ split-run (INF-17) ----------

/// Marker that replaces an OS object address in the page fingerprint.
const FOREIGN_MARK: u32 = 0xF0F0_F0F0;

/// Committed memory that holds OS-assigned objects: everything committed outside the exe image, the
/// determinism arena and the current thread's stack (Wine heaps, D3D/DirectInput/DirectSound objects,
/// DLL data). Built per split-run case (VirtualQuery walk).
struct ForeignMap {
    ranges: Vec<(u32, u32)>,
}

impl ForeignMap {
    fn build(stack: (u32, u32)) -> ForeignMap {
        use windows_sys::Win32::System::Memory::{MEM_COMMIT, MEMORY_BASIC_INFORMATION, VirtualQuery};
        let arena = crate::detwin::arena_range();
        let excluded = [IMAGE, arena, stack];
        let mut ranges: Vec<(u32, u32)> = Vec::new();
        let mut a: u64 = 0x10000;
        while a < 0xFFFF_0000 {
            // SAFETY: VirtualQuery only reads the address space description.
            let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
            let n = unsafe {
                VirtualQuery(
                    a as usize as *const std::ffi::c_void,
                    &mut mbi,
                    std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
                )
            };
            if n == 0 || mbi.RegionSize == 0 {
                break;
            }
            let lo = mbi.BaseAddress as usize as u64;
            let hi = lo + mbi.RegionSize as u64;
            if mbi.State == MEM_COMMIT && hi <= 0x1_0000_0000 {
                let (lo, hi) = (lo as u32, (hi - 1) as u32);
                let inside = |r: (u32, u32)| r.1 > r.0 && lo < r.1 && hi >= r.0;
                if !excluded.iter().any(|&r| inside(r)) {
                    match ranges.last_mut() {
                        Some(last) if last.1 == lo => last.1 = hi.wrapping_add(1),
                        _ => ranges.push((lo, hi.wrapping_add(1))),
                    }
                }
            }
            a = hi.max(a + 0x1000);
        }
        ForeignMap { ranges }
    }
    fn contains(&self, v: u32) -> bool {
        if v < 0x10000 || (v >= IMAGE.0 && v < IMAGE.1) {
            return false;
        }
        self.ranges
            .binary_search_by(|r| {
                if v < r.0 {
                    std::cmp::Ordering::Greater
                } else if r.1 != 0 && v >= r.1 {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .is_ok()
    }
}

/// The distinct OS object addresses among the case's arguments, registers, region bytes and the
/// window words around its writes (before and after the call).
fn foreign_values(fm: &ForeignMap, fm_post: &ForeignMap, c: &Case, snap: &Snap) -> Vec<u32> {
    let mut v: Vec<u32> = Vec::new();
    let mut add = |w: u32| {
        if fm.contains(w) || fm_post.contains(w) {
            v.push(w);
        }
    };
    for &w in c.args.iter().chain(&c.regs_in).chain(&c.orig.regs) {
        add(w);
    }
    let words = |b: &[u8], add: &mut dyn FnMut(u32)| {
        for x in b.as_chunks::<4>().0 {
            add(u32::from_le_bytes(*x));
        }
    };
    for r in c.regions.iter().filter(|r| !matches!(r.kind, RegionKind::Os(_))) {
        // the region's own address: an extra region read through a pointer (`[0x8E3FEC]:0x18`, the
        // LocalAlloc'd CdStream queue buffer) lies in OS-allocated memory whose address differs per process
        add(r.addr);
        let o = ((4 - (r.addr & 3)) & 3) as usize;
        for b in [&r.pre, &r.post_orig] {
            if b.len() > o {
                words(&b[o..], &mut add);
            }
        }
    }
    for w in &c.orig.writes {
        let (lo, hi) = (w.addr & !3, (w.addr + w.bytes.len() as u32 + 3) & !3);
        if let Some(post) = mem_read(lo, (hi - lo) as usize) {
            words(&post, &mut add);
        }
        let pre = snap.pre(lo, (hi - lo) as usize);
        words(&pre, &mut add);
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// The current thread's stack [StackLimit, StackBase) from its TIB.
fn thread_stack() -> (u32, u32) {
    let (base, limit): (u32, u32);
    // SAFETY: reads the current thread's NT_TIB through FS (i686 Windows).
    unsafe {
        std::arch::asm!("mov {0}, fs:[4]", "mov {1}, fs:[8]", out(reg) base, out(reg) limit, options(nostack, readonly, preserves_flags));
    }
    (limit, base)
}

/// One sampled call in split-run mode: runs only this run's implementation (the original in the
/// plain run, Rust in the rust run) on the live state, records inputs and outputs, and returns its
/// results to the caller. Nothing is restored: the game continues with what the call did, so both runs
/// stay identical up to the first differing output (determinism mode), which `split-report` checks
/// through the inputs of every later case.
fn run_split(f: &mut Frame, i: usize, sig: &fmt::Sig, frame: u32, ordinal: u32, split: Split) {
    let t0 = std::time::Instant::now();
    let c = cfg();
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let args_ptr = f as *mut Frame as usize as u32 + std::mem::size_of::<Frame>() as u32;
    let n = sig.args as usize;
    let args: Vec<u32> = mem_read(args_ptr, 4 * n)
        .map(|b| {
            b.as_chunks::<4>()
                .0
                .iter()
                .map(|w| u32::from_le_bytes(*w))
                .collect()
        })
        .unwrap_or_default();
    if args.len() != n {
        f.tail = leave_target(i);
        return;
    }
    let mut regions = build_regions(f, i, sig, &args);
    let unreadable = regions.iter().filter(|r| !r.readable).count() as u32;
    let mut windows = WINDOWS.get().cloned().unwrap_or_default();
    let nstatic = windows.len();
    let stack_lo = args_ptr + 4 * n as u32;
    if c.stack > 0 && win::readable(stack_lo, c.stack as usize) {
        windows.push((stack_lo, stack_lo + c.stack));
    }
    if c.window == Window::All
        && let Some((a, b)) = crate::detwin::arena_extent()
        && b > a
        && win::readable(a, (b - a) as usize)
    {
        windows.push((a, b));
    }
    let snap = Snap::take(windows.clone());
    let stack = thread_stack();
    let fm = ForeignMap::build(stack);
    let mut foreign_words = Vec::new();
    let arena_lo = match crate::detwin::arena_range().0 {
        0 => 0x1800_0000,
        a => a,
    };
    let mask = |v: u32| {
        (0x10000..arena_lo).contains(&v)
            && !(IMAGE.0..IMAGE.1).contains(&v)
            && !(stack.0..stack.1).contains(&v)
    };
    let page_hashes = snap.page_hashes(nstatic, &mask, stack, &fm, &mut foreign_words);
    let volatile: Vec<Write> = sa_oracle_format::normalise::volatile_ranges()
        .into_iter()
        .filter_map(|(addr, len, _)| mem_read(addr, len as usize).map(|bytes| Write { addr, bytes }))
        .collect();
    if c.dump_seq.contains(&seq) {
        let out = c.out.clone().unwrap_or_else(|| "l2.spool".into());
        snap.dump(nstatic, &format!("{out}.seq{seq}.win"));
    }

    let mut inv = new_invoke(f, &args);
    let outer = crate::l2::run_suspend();
    crate::l2::run_begin(i as u32);
    let target = match split {
        Split::Orig => table::original_at(i),
        Split::Rust => table::rust_entry(i),
    };
    invoke(&mut inv, target);
    let (calls, allocs) = crate::l2::run_end();
    crate::l2::run_resume(outer);
    read_regions(&mut regions, true);
    capture_post(&mut regions, i, &inv.regs_out, &args, true);
    let writes = snap.diff();
    let out = outcome(&inv, calls, allocs, writes);
    finish_frame(f, &inv, &out, args_ptr);

    let window_pre = pre_of_writes(&snap, &[&out.writes], true);
    let mut case = new_case(f, i, sig, frame, ordinal, &args, regions, windows, window_pre);
    // The frame's registers are the outputs now: the inputs were captured by build_regions/new_invoke.
    case.regs_in = inv.regs_in;
    case.orig = out;
    case.leave = match split {
        Split::Orig => fmt::LEAVE_SPLIT_ORIG,
        Split::Rust => fmt::LEAVE_SPLIT_RUST,
    };
    case.mutant = c.mutants.contains(&i);
    case.seq = seq;
    case.page_hashes = page_hashes;
    case.volatile = volatile;
    case.esp = args_ptr;
    case.stack = stack;
    // Objects the call itself created (D3D device, DirectInput devices) live in memory committed
    // during the call: the map is built again for the outputs.
    let fm_post = ForeignMap::build(stack);
    case.foreign = foreign_values(&fm, &fm_post, &case, &snap);
    case.foreign_words = foreign_words;
    drop(snap);
    record(case, i, t0, unreadable, false);
}

/// Pending ledger entry "GetATanOfXY raw FPATAN path": ST0 differs beyond f32 only.
fn is_ledger(addr: u32, c: &Case, diffs: &[fmt::Diff]) -> bool {
    addr == 0x53CC70
        && diffs.iter().all(|d| d.field == "st0")
        && c.orig.st.len() == 1
        && c.rust.st.len() == 1
        && fmt::ext_to_f32_bits(&c.orig.st[0]) == fmt::ext_to_f32_bits(&c.rust.st[0])
}

fn args_str(args: &[u32]) -> String {
    let v: Vec<String> = args
        .iter()
        .map(|&w| format!("{w:#010x}({})", f32::from_bits(w)))
        .collect();
    format!("[{}]", v.join(","))
}

// ------------------------------------------------------------------ control / end ---------------

pub fn summary() -> String {
    let c = cfg();
    let st = STATS.lock().unwrap_or_else(|e| e.into_inner());
    let off = OFFTHREAD.lock().map(|v| v.clone()).unwrap_or_default();
    let mut parts = vec![format!(
        "l2 cases={} leave={} max={} rate={} per_frame={} mutants=[{}]",
        CASES_TOTAL.load(Ordering::Relaxed),
        if c.leave_rust { "rust" } else { "orig" },
        c.max,
        c.rate,
        c.per_frame,
        c.mutants
            .iter()
            .map(|&i| HOOKS[i].name)
            .collect::<Vec<_>>()
            .join(",")
    )];
    if let Some((h, f, n)) = QUOTA.lock().ok().and_then(|q| q.hit.clone()) {
        parts[0] += &format!(" spool_quota_reached={h}@frame{f} spooled={n}");
    }
    if let Some(sp) = c.split {
        parts[0] += &format!(" split={}", if sp == Split::Orig { "orig" } else { "rust" });
    }
    for (i, s) in st.iter().enumerate() {
        if s.calls > 0 || c.hooks.contains(&i) {
            parts.push(format!(
                "{} offthread={} mode={}",
                s.line(HOOKS[i].name),
                off.get(i).copied().unwrap_or(0),
                table::mode(i).name()
            ));
        }
    }
    parts.join("; ")
}

pub fn flush() -> String {
    for m in [&SPOOL, &REPORT] {
        if let Ok(mut g) = m.lock()
            && let Some(w) = g.as_mut()
        {
            let _ = w.flush();
        }
    }
    "ok l2 flushed".into()
}

/// End of run (determinism mode end frame): summary into the log and the report, flush.
pub fn finish() {
    if cfg().hooks.is_empty() && CASES_TOTAL.load(Ordering::Relaxed) == 0 {
        return;
    }
    let s = summary();
    log(&format!("l2: summary: {s}"));
    if let Ok(mut g) = REPORT.lock()
        && let Some(w) = g.as_mut()
    {
        let _ = writeln!(w, "SUMMARY {s}");
    }
    flush();
}
