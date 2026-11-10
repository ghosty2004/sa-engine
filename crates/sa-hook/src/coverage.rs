//! Execution coverage of the original code (INF-10): first-hit breakpoints.
//!
//! The host (`sa-oracle coverage points`) writes a list of code addresses (catalogue function entries
//! and the basic-block starts found by decoding each function from its entry) together with the byte
//! the exe has there. At attach, after the hooks and determinism patches are in place, every listed
//! address whose current byte matches gets an `int3`. A vectored exception handler counts the trap,
//! writes the original byte back and resumes at the same address, so each point costs one exception
//! the first time it executes and nothing afterwards. With `SA_COV_CAP` > 1 the point is re-armed
//! after its first instruction (single step) until it has trapped CAP times, so the hit column is
//! `min(executions, CAP)` (lower bound when two threads race on one point).
//!
//! | variable | meaning |
//! |---|---|
//! | `SA_COV` | points file (`ADDR BYTE [anything]` per line, hex; `#` comments) |
//! | `SA_COV_OUT` | output CSV (`address,hits`, every armed point), default `coverage.csv` next to gta_sa.exe; rewritten every 10 s and at the end frame |
//! | `SA_COV_CAP` | traps counted per point (default 1 = first hit only) |
//! | `SA_COV_SCM` | output CSV of SCM opcodes executed (`opcode,count`), written with the hits (SAMP-1D) |
//!
//! `SA_COV_SCM` detours `CRunningScript::ProcessOneCommand` (0x469EB0): a stub counts the command
//! word at `m_pCurrentIP` (`& 0x7FFF`, the high bit is the negation flag) in a 32768-entry table, runs
//! the relocated first instruction (`inc word ptr [0xA447F4]`, position independent) and jumps back to
//! 0x469EB7. The entry point is then no longer armed (byte mismatch); the opcode counts stand for it.
//!
//! `OUT.last` lists the last 256 first hits in order (where a run that hangs was last seen). At init a
//! throw-away `int3; ret` calibrates how the platform reports a breakpoint (ExceptionAddress at the
//! int3 or after it; Wine's WoW64 may differ from Windows).
//!
//! A point is skipped (and counted in the log) when its byte differs from the expected one, unless the
//! address is a hook entry (patched to `jmp thunk` by sa-hook; arming it counts every entry, the
//! trampoline is never armed).

/// One point of the points file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub addr: u32,
    /// Byte the unpatched exe has at `addr`.
    pub byte: u8,
}

/// Parses a points file. Lines: `ADDR BYTE [rest ignored]`, hex with or without `0x`; blank lines
/// and `#` comments are skipped. The result is sorted by address and deduplicated.
pub fn parse_points(text: &str) -> Result<Vec<Point>, String> {
    let hex = |s: &str| u32::from_str_radix(s.trim_start_matches("0x").trim_start_matches("0X"), 16);
    let mut v = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split_whitespace();
        let bad = || format!("points line {}: {line:?} (ADDR BYTE)", n + 1);
        let addr = it.next().and_then(|s| hex(s).ok()).ok_or_else(bad)?;
        let byte = it
            .next()
            .and_then(|s| hex(s).ok())
            .filter(|&b| b <= 0xFF)
            .ok_or_else(bad)? as u8;
        v.push(Point { addr, byte });
    }
    v.sort_by_key(|p| p.addr);
    v.dedup_by_key(|p| p.addr);
    Ok(v)
}

/// The output CSV: `address,hits` for every armed point, in address order.
pub fn csv(points: &[u32], hits: impl Fn(usize) -> u32) -> String {
    let mut s = String::with_capacity(points.len() * 16 + 16);
    s += "address,hits\n";
    for (i, a) in points.iter().enumerate() {
        s += &format!("{a:#x},{}\n", hits(i));
    }
    s
}

#[cfg(windows)]
pub use imp::{exit_flush, finish, init, summary};

/// `CRunningScript::ProcessOneCommand` and its first instruction (`inc word ptr [0xA447F4]`).
pub const SCM_DISPATCH: u32 = 0x469EB0;
pub const SCM_FIRST: [u8; 7] = [0x66, 0xFF, 0x05, 0xF4, 0x47, 0xA4, 0x00];

/// The detour stub for `SCM_DISPATCH` at `at`, counting into the u32 table at `counts`.
pub fn scm_stub(at: u32, counts: u32) -> Vec<u8> {
    let mut v = vec![
        0x50, 0x8B, 0x41, 0x14, 0x0F, 0xB7, 0x00, 0x25, 0xFF, 0x7F, 0x00, 0x00,
    ];
    // inc dword ptr [eax*4 + counts]
    v.extend([0xFF, 0x04, 0x85]);
    v.extend(counts.to_le_bytes());
    v.push(0x58); // pop eax
    v.extend(SCM_FIRST);
    let back = SCM_DISPATCH + SCM_FIRST.len() as u32;
    let next = at + v.len() as u32 + 5;
    v.push(0xE9);
    v.extend(back.wrapping_sub(next).to_le_bytes());
    v
}

/// `opcode,count` lines for every opcode with a non-zero count.
pub fn scm_csv(count: impl Fn(usize) -> u32) -> String {
    let mut s = String::from("opcode,count\n");
    for op in 0..0x8000 {
        let n = count(op);
        if n > 0 {
            s += &format!("{op:04X},{n}\n");
        }
    }
    s
}

#[cfg(windows)]
mod imp {
    use std::cell::Cell;
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;

    use windows_sys::Win32::Foundation::{EXCEPTION_BREAKPOINT, EXCEPTION_SINGLE_STEP};
    use windows_sys::Win32::System::Diagnostics::Debug::{
        AddVectoredExceptionHandler, EXCEPTION_POINTERS, FlushInstructionCache,
    };
    use windows_sys::Win32::System::Memory::{PAGE_EXECUTE_READWRITE, PAGE_READWRITE, VirtualProtect};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    use super::{csv, parse_points};
    use crate::table::{self, HOOKS};
    use crate::win::{self, log};

    const CONTINUE_EXECUTION: i32 = -1;
    const CONTINUE_SEARCH: i32 = 0;
    const TRAP_FLAG: u32 = 0x100;

    struct State {
        addrs: Vec<u32>,
        orig: Vec<u8>,
        hits: Vec<AtomicU32>,
        cap: u32,
        out: String,
    }

    static STATE: OnceLock<State> = OnceLock::new();
    /// SA_COV_SCM: (output path, address of the 32768 x u32 count table).
    static SCM: OnceLock<(String, u32)> = OnceLock::new();

    fn install_scm(out: String) {
        use windows_sys::Win32::System::Memory::{MEM_COMMIT, MEM_RESERVE, VirtualAlloc};
        match win::read(super::SCM_DISPATCH, super::SCM_FIRST.len()) {
            Some(b) if b == super::SCM_FIRST => {}
            other => {
                log(&format!(
                    "coverage: SCM dispatch bytes {other:02x?} unexpected; no opcode counts"
                ));
                return;
            }
        }
        // SAFETY: plain allocations: the count table (RW) and the stub page (RWX).
        let (counts, stub) = unsafe {
            (
                VirtualAlloc(
                    std::ptr::null(),
                    0x8000 * 4,
                    MEM_COMMIT | MEM_RESERVE,
                    PAGE_READWRITE,
                ),
                VirtualAlloc(
                    std::ptr::null(),
                    64,
                    MEM_COMMIT | MEM_RESERVE,
                    PAGE_EXECUTE_READWRITE,
                ),
            )
        };
        let (counts, stub) = (counts as usize as u32, stub as usize as u32);
        if counts == 0 || stub == 0 {
            log("coverage: VirtualAlloc for the SCM counter failed");
            return;
        }
        let code = super::scm_stub(stub, counts);
        // SAFETY: the stub page was just allocated with room for the stub.
        unsafe { std::ptr::copy_nonoverlapping(code.as_ptr(), stub as usize as *mut u8, code.len()) };
        let mut jmp = vec![0xE9];
        jmp.extend(stub.wrapping_sub(super::SCM_DISPATCH + 5).to_le_bytes());
        jmp.extend([0x90, 0x90]);
        if let Err(e) = win::patch(super::SCM_DISPATCH, &jmp) {
            log(&format!("coverage: SCM detour failed: {e}"));
            return;
        }
        let _ = SCM.set((out, counts));
        log(&format!(
            "coverage: SCM opcode counter at {:#x} (stub {stub:#x})",
            super::SCM_DISPATCH
        ));
    }

    fn write_scm() {
        let Some((out, counts)) = SCM.get() else { return };
        // SAFETY: the table holds 0x8000 u32 counters allocated in install_scm.
        let text =
            super::scm_csv(|i| unsafe { std::ptr::read_volatile((*counts as usize as *const u32).add(i)) });
        let tmp = format!("{out}.tmp");
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, out);
        }
    }
    static WRITE: Mutex<()> = Mutex::new(());
    static DONE: AtomicBool = AtomicBool::new(false);

    /// The last first-hits, in order (diagnostics: where a run that hangs was last seen).
    const RING: usize = 256;
    #[allow(clippy::declare_interior_mutable_const)]
    const Z: AtomicU32 = AtomicU32::new(0);
    static LAST: [AtomicU32; RING] = [Z; RING];
    static LAST_N: AtomicU32 = AtomicU32::new(0);

    /// int3 calibration: a throw-away `int3; ret` executed once at init tells how this Windows/Wine
    /// reports a breakpoint (offset of ExceptionAddress and of EIP from the int3).
    static CALIB_AT: AtomicU32 = AtomicU32::new(0);
    static CALIB_EXC: AtomicU32 = AtomicU32::new(u32::MAX);
    static CALIB_EIP: AtomicU32 = AtomicU32::new(u32::MAX);

    thread_local! {
        /// Index of the point this thread is single-stepping over (cap > 1), `usize::MAX` if none.
        static REARM: Cell<usize> = const { Cell::new(usize::MAX) };
    }

    fn index(s: &State, addr: u32) -> Option<usize> {
        s.addrs.binary_search(&addr).ok()
    }

    /// Writes one code byte (pages were made RWX at arm time).
    unsafe fn put(addr: u32, b: u8) {
        // SAFETY: caller guarantees `addr` is an armed point inside a section made writable.
        unsafe {
            std::ptr::write_volatile(addr as usize as *mut u8, b);
            FlushInstructionCache(GetCurrentProcess(), addr as usize as *const c_void, 1);
        }
    }

    unsafe extern "system" fn handler(info: *mut EXCEPTION_POINTERS) -> i32 {
        // SAFETY: the OS passes valid exception and context records.
        let (rec, ctx) = unsafe { (&*(*info).ExceptionRecord, &mut *(*info).ContextRecord) };
        let at = rec.ExceptionAddress as usize as u32;
        let c = CALIB_AT.load(Ordering::Acquire);
        if c != 0 && rec.ExceptionCode == EXCEPTION_BREAKPOINT && (c..c + 2).contains(&at) {
            CALIB_EXC.store(at - c, Ordering::Release);
            CALIB_EIP.store(ctx.Eip.wrapping_sub(c), Ordering::Release);
            ctx.Eip = c + 1; // the `ret`
            return CONTINUE_EXECUTION;
        }
        let Some(s) = STATE.get() else {
            return CONTINUE_SEARCH;
        };
        match rec.ExceptionCode {
            EXCEPTION_BREAKPOINT => {
                // The int3's address: ExceptionAddress minus the calibrated offset (0 on Windows).
                let off = CALIB_EXC.load(Ordering::Relaxed).min(1);
                let Some(i) = index(s, at.wrapping_sub(off)) else {
                    return CONTINUE_SEARCH;
                };
                let n = s.hits[i].fetch_add(1, Ordering::Relaxed) + 1;
                if n == 1 {
                    let k = LAST_N.fetch_add(1, Ordering::Relaxed) as usize;
                    LAST[k % RING].store(s.addrs[i], Ordering::Relaxed);
                }
                // SAFETY: point `i` was armed by us; the original byte goes back.
                unsafe { put(s.addrs[i], s.orig[i]) };
                ctx.Eip = s.addrs[i];
                if n < s.cap && !DONE.load(Ordering::Relaxed) {
                    REARM.with(|r| r.set(i));
                    ctx.EFlags |= TRAP_FLAG;
                }
                CONTINUE_EXECUTION
            }
            EXCEPTION_SINGLE_STEP => {
                let i = REARM.with(|r| r.replace(usize::MAX));
                if i == usize::MAX {
                    return CONTINUE_SEARCH;
                }
                ctx.EFlags &= !TRAP_FLAG;
                if s.hits[i].load(Ordering::Relaxed) < s.cap && !DONE.load(Ordering::Relaxed) {
                    // SAFETY: as above; re-arms the point.
                    unsafe { put(s.addrs[i], 0xCC) };
                }
                CONTINUE_EXECUTION
            }
            _ => CONTINUE_SEARCH,
        }
    }

    /// Image sections of gta_sa.exe as (start, end) (page-rounded), from the loaded headers.
    fn sections() -> Vec<(u32, u32)> {
        let base = 0x400000u32;
        let rd = |a: u32| win::read(a, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        let Some(pe) = rd(base + 0x3C) else {
            return Vec::new();
        };
        let Some(h) = win::read(base + pe, 24) else {
            return Vec::new();
        };
        let n = u16::from_le_bytes([h[6], h[7]]) as u32;
        let opt = u16::from_le_bytes([h[20], h[21]]) as u32;
        let sec = base + pe + 24 + opt;
        (0..n)
            .filter_map(|k| {
                let s = win::read(sec + k * 40, 40)?;
                let vs = u32::from_le_bytes([s[8], s[9], s[10], s[11]]);
                let va = u32::from_le_bytes([s[12], s[13], s[14], s[15]]);
                Some((base + va, base + va + ((vs + 0xFFF) & !0xFFF)))
            })
            .collect()
    }

    /// Reads the configuration and arms the points. Call after every hook and patch is installed,
    /// while the game's threads are still suspended.
    pub fn init(dir: &std::path::Path) {
        let Some(path) = std::env::var("SA_COV").ok().filter(|v| !v.is_empty()) else {
            return;
        };
        let points = match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|t| parse_points(&t))
        {
            Ok(p) => p,
            Err(e) => {
                log(&format!("coverage: {path}: {e}; coverage off"));
                return;
            }
        };
        let cap = std::env::var("SA_COV_CAP")
            .ok()
            .and_then(|v| v.trim().parse::<u32>().ok())
            .unwrap_or(1)
            .max(1);
        let out = std::env::var("SA_COV_OUT")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| dir.join("coverage.csv").to_string_lossy().into_owned());
        if let Some(o) = std::env::var("SA_COV_SCM").ok().filter(|v| !v.is_empty()) {
            install_scm(o);
        }
        let secs = sections();
        let hook_entries: Vec<u32> = (0..HOOKS.len())
            .filter(|&i| table::installed(i))
            .map(|i| HOOKS[i].addr)
            .collect();
        let (mut addrs, mut orig) = (Vec::new(), Vec::new());
        let (mut outside, mut mismatch, mut hooked) = (0usize, 0usize, 0usize);
        let mut first_mismatch = Vec::new();
        for p in &points {
            if !secs.iter().any(|&(a, b)| (a..b).contains(&p.addr)) {
                outside += 1;
                continue;
            }
            let Some(cur) = win::read(p.addr, 1).map(|b| b[0]) else {
                outside += 1;
                continue;
            };
            let is_hook = hook_entries.contains(&p.addr);
            if cur != p.byte && !is_hook {
                mismatch += 1;
                if first_mismatch.len() < 8 {
                    first_mismatch.push(format!("{:#x}:{:02x}!={:02x}", p.addr, cur, p.byte));
                }
                continue;
            }
            hooked += is_hook as usize;
            addrs.push(p.addr);
            orig.push(cur);
        }
        // Every section holding a point becomes RWX once (the handler restores bytes in place).
        for &(a, b) in &secs {
            if addrs.iter().any(|x| (a..b).contains(x)) {
                let mut old = 0u32;
                // SAFETY: changes the protection of an image section of the game.
                let ok = unsafe {
                    VirtualProtect(
                        a as usize as *const c_void,
                        (b - a) as usize,
                        PAGE_EXECUTE_READWRITE,
                        &mut old,
                    )
                };
                if ok == 0 {
                    log(&format!(
                        "coverage: VirtualProtect {a:#x}-{b:#x} failed; coverage off"
                    ));
                    return;
                }
            }
        }
        let n = addrs.len();
        let state = State {
            hits: (0..n).map(|_| AtomicU32::new(0)).collect(),
            addrs,
            orig,
            cap,
            out,
        };
        if STATE.set(state).is_err() {
            return;
        }
        let s = STATE.get().expect("just set");
        // SAFETY: registers our handler first in the chain.
        if unsafe { AddVectoredExceptionHandler(1, Some(handler)) }.is_null() {
            log("coverage: AddVectoredExceptionHandler failed; coverage off");
            return;
        }
        if let Err(e) = calibrate() {
            log(&format!("coverage: int3 calibration failed ({e}); coverage off"));
            return;
        }
        for &a in &s.addrs {
            // SAFETY: `a` is in a section made writable above; the game's threads are suspended.
            unsafe { std::ptr::write_volatile(a as usize as *mut u8, 0xCC) };
        }
        // SAFETY: flushes the whole image after the bulk write.
        unsafe { FlushInstructionCache(GetCurrentProcess(), std::ptr::null(), 0) };
        log(&format!(
            "coverage: {n} of {} points armed ({hooked} hook entries), {outside} outside the image, \
             {mismatch} byte mismatches {first_mismatch:?}; cap {cap}; out {}; int3 reported at \
             ExceptionAddress=int3+{} Eip=int3+{}",
            points.len(),
            s.out,
            CALIB_EXC.load(Ordering::Relaxed),
            CALIB_EIP.load(Ordering::Relaxed),
        ));
        let _ = std::thread::Builder::new().name("sa-hook-cov".into()).spawn(|| {
            while !DONE.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_secs(10));
                write();
            }
        });
    }

    /// Executes one `int3; ret` in a fresh executable page and records how the trap is reported.
    fn calibrate() -> Result<(), String> {
        use windows_sys::Win32::System::Memory::{MEM_COMMIT, MEM_RESERVE, VirtualAlloc};
        // SAFETY: a private RWX page for the probe.
        let p = unsafe {
            VirtualAlloc(
                std::ptr::null(),
                16,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            )
        } as usize as u32;
        if p == 0 {
            return Err("VirtualAlloc".into());
        }
        // SAFETY: the page was just allocated; `int3; ret` is a valid cdecl function.
        unsafe {
            std::ptr::copy_nonoverlapping([0xCCu8, 0xC3].as_ptr(), p as usize as *mut u8, 2);
            CALIB_AT.store(p, Ordering::Release);
            let f: extern "C" fn() = std::mem::transmute(p as usize);
            f();
        }
        CALIB_AT.store(0, Ordering::Release);
        match CALIB_EXC.load(Ordering::Acquire) {
            0 | 1 => Ok(()),
            v => Err(format!("ExceptionAddress offset {v:#x}")),
        }
    }

    fn write() {
        let _g = WRITE.lock().unwrap_or_else(|e| e.into_inner());
        write_locked();
    }

    fn write_locked() {
        write_scm();
        let Some(s) = STATE.get() else { return };
        let text = csv(&s.addrs, |i| s.hits[i].load(Ordering::Relaxed));
        let tmp = format!("{}.tmp", s.out);
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, &s.out);
        }
        let n = LAST_N.load(Ordering::Relaxed) as usize;
        let mut last = format!("# last first-hits, oldest first ({n} first hits in all)\n");
        for k in n.saturating_sub(RING)..n {
            last += &format!("{:#x}\n", LAST[k % RING].load(Ordering::Relaxed));
        }
        let _ = std::fs::write(format!("{}.last", s.out), last);
    }

    /// End of the run (end frame reached): stop re-arming and write the final CSV.
    pub fn finish() {
        if STATE.get().is_none() {
            return;
        }
        DONE.store(true, Ordering::Release);
        write();
        log(&format!("coverage: final {}", summary()));
    }

    /// Process exit (DLL_PROCESS_DETACH): one last write. The writer thread may have been ended
    /// while holding the lock, so it is only tried (a blocked exit would hang the game).
    pub fn exit_flush() {
        if STATE.get().is_none() {
            return;
        }
        DONE.store(true, Ordering::Release);
        if let Ok(_g) = WRITE.try_lock() {
            write_locked();
        }
    }

    pub fn summary() -> String {
        match STATE.get() {
            Some(s) => {
                let hit = s.hits.iter().filter(|h| h.load(Ordering::Relaxed) > 0).count();
                format!("cov points={} hit={hit} out={}", s.addrs.len(), s.out)
            }
            None => "cov off".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scm_stub_encoding() {
        let at = 0x0100_0000;
        let v = scm_stub(at, 0x0200_0000);
        // push eax; mov eax,[ecx+0x14]; movzx eax,word [eax]; and eax,0x7fff; inc dword [eax*4+T]; pop eax
        assert_eq!(
            &v[..12],
            &[
                0x50, 0x8B, 0x41, 0x14, 0x0F, 0xB7, 0x00, 0x25, 0xFF, 0x7F, 0x00, 0x00
            ]
        );
        assert_eq!(&v[12..19], &[0xFF, 0x04, 0x85, 0x00, 0x00, 0x00, 0x02]);
        assert_eq!(v[19], 0x58);
        assert_eq!(&v[20..27], &SCM_FIRST);
        assert_eq!(v[27], 0xE9);
        let rel = i32::from_le_bytes([v[28], v[29], v[30], v[31]]);
        assert_eq!((at + 32).wrapping_add(rel as u32), 0x469EB7);
        assert_eq!(scm_csv(|i| (i == 0x9C6) as u32 * 3), "opcode,count\n09C6,3\n");
    }

    #[test]
    fn points_and_csv() {
        let p = parse_points("# hdr\n0x401000 e9 F f\n\n401005 0x55\n0x401000 e9\n").unwrap();
        assert_eq!(
            p,
            vec![
                Point {
                    addr: 0x401000,
                    byte: 0xE9
                },
                Point {
                    addr: 0x401005,
                    byte: 0x55
                }
            ]
        );
        assert!(parse_points("0x401000").is_err());
        assert!(parse_points("0x401000 100").is_err());
        let a: Vec<u32> = p.iter().map(|p| p.addr).collect();
        assert_eq!(
            csv(&a, |i| i as u32 * 3),
            "address,hits\n0x401000,0\n0x401005,3\n"
        );
    }
}
