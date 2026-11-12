//! The legacy address arena and the crash reporter.
//!
//! Ported code still reaches game state by its 1.0 US address (`GPtr::from_addr(CONTROLS_MANAGER)`,
//! `rdata(0x8747A8)`, `g::<T>(addr)`, vtable addresses stored in objects, `globals!` declarations;
//! ADR-001 §3 plans an engine context later). Phase B runs as a 32-bit process, so `sa-game` maps
//! the original's data sections at their original virtual addresses: `.rdata` (read-only, as in the
//! original), `.data` with its zero-fill `.bss` extent, `_TEXT_HA`, `_rwdseg` and `.rsrc`
//! (0x858000–0xCB1000). Their initial contents come from the user's own `gta_sa.exe` in the game
//! folder, read at start-up only (1.0 US, verified by its SHA-256): nothing of it is copied into
//! the repository or the build. No code section is executable: every function runs as its Rust
//! port. Game `.text`/`_rwcseg` and `.HOODLUM` are mapped read-only as data, because the no-CD
//! protection chunks read code bytes and `.HOODLUM` dwords as data ([`map_read_only_code`]).
//!
//! `globals!` storage aliases the arena (`sa_core::mem::Global::ptr` returns the original address
//! once [`sa_core::orig::registry::set_arena`] is called), so raw-address and `globals!` accesses
//! see one copy.
//!
//! [`install_crash_reporter`] turns a SIGSEGV/SIGBUS into a one-line report naming the boot step
//! that was running and the section of the faulting address, instead of a bare crash.

#![allow(unsafe_code)]

use std::path::Path;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

/// First and one-past-last address of the arena (`.rdata` … `.rsrc`).
pub const ARENA: (usize, usize) = (0x0085_8000, 0x00CB_1000);

/// SHA-256 of `gta_sa.exe` 1.0 US (CLAUDE.md, docs/PLAN.md §1).
pub const EXE_SHA256: &str = "a559aa772fd136379155efa71f00c47aad34bbfeae6196b0fe1047d0645cbd26";

/// `.rdata`: mapped read-only after loading, as the original's loader does.
const RDATA: (usize, usize) = (0x0085_8000, 0x008A_4000);

/// The section of the 1.0 US image an address belongs to.
pub fn section_of(a: usize) -> &'static str {
    match a {
        0x0040_1000..0x0085_7000 => ".text (game code: read-only data in phase B, never executed)",
        0x0085_7000..0x0085_8000 => "_rwcseg (code: read-only data in phase B)",
        0x0085_8000..0x008A_4000 => ".rdata (read-only)",
        0x008A_4000..0x008E_4000 => ".data",
        0x008E_4000..0x00C9_E000 => ".bss",
        0x00C9_E000..0x00CB_1000 => "_TEXT_HA/_rwdseg/.rsrc",
        0x00CB_1000..0x0155_6000 => "no-CD image (.text/.data copy, not mapped)",
        0x0155_6000..0x0157_7000 => ".HOODLUM (read-only data in phase B)",
        0..0x1_0000 => "null page (a null game pointer)",
        _ => "outside the original image (a host pointer)",
    }
}

/// One PE section header of interest.
#[derive(Clone, Copy, Debug)]
struct Section {
    va: usize,
    vsize: usize,
    raw_off: usize,
    raw_size: usize,
}

fn u16_at(d: &[u8], o: usize) -> Result<u16, String> {
    d.get(o..o + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| "gta_sa.exe: truncated PE header".to_owned())
}
fn u32_at(d: &[u8], o: usize) -> Result<u32, String> {
    d.get(o..o + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| "gta_sa.exe: truncated PE header".to_owned())
}

/// The sections of the image that lie inside the arena.
fn arena_sections(exe: &[u8]) -> Result<Vec<Section>, String> {
    let pe = u32_at(exe, 0x3C)? as usize;
    if exe.get(pe..pe + 4) != Some(b"PE\0\0") {
        return Err("gta_sa.exe: not a PE image".into());
    }
    let (n, opt) = (u16_at(exe, pe + 6)? as usize, u16_at(exe, pe + 20)? as usize);
    let base = u32_at(exe, pe + 24 + 28)? as usize;
    let mut out = Vec::new();
    for i in 0..n {
        let h = pe + 24 + opt + i * 40;
        let s = Section {
            vsize: u32_at(exe, h + 8)? as usize,
            va: base + u32_at(exe, h + 12)? as usize,
            raw_size: u32_at(exe, h + 16)? as usize,
            raw_off: u32_at(exe, h + 20)? as usize,
        };
        if s.va >= ARENA.0 && s.va < ARENA.1 {
            if s.va + s.vsize.max(s.raw_size) > ARENA.1 || s.raw_off + s.raw_size > exe.len() {
                return Err(format!("gta_sa.exe: section at {:#x} does not fit", s.va));
            }
            out.push(s);
        }
    }
    Ok(out)
}

/// Reads `<game>/gta_sa.exe`, checks it is 1.0 US, maps the arena (once) and fills it with the
/// image's data sections. Returns a one-line summary.
pub fn map(game: &Path) -> Result<String, String> {
    let path = game.join("gta_sa.exe");
    let exe = std::fs::read(&path).map_err(|e| format!("{}: {e} (phase B reads the data sections of your own 1.0 US exe at start-up)", path.display()))?;
    map_image(exe, &path.display().to_string())
}

/// [`map`] from the exe's bytes (`label` names it in messages).
pub fn map_image(exe: Vec<u8>, label: &str) -> Result<String, String> {
    let digest = crate::sha256::hex(&exe);
    if digest != EXE_SHA256 {
        return Err(format!(
            "{label}: SHA-256 {digest} is not gta_sa.exe 1.0 US ({EXE_SHA256})"
        ));
    }
    let secs = arena_sections(&exe)?;
    let (lo, hi) = ARENA;
    map_fixed(lo, hi - lo).map_err(|e| format!("cannot map the legacy address arena: {e}"))?;
    let mut bytes = 0;
    for s in &secs {
        let n = if s.vsize == 0 { s.raw_size } else { s.raw_size.min(s.vsize) };
        // SAFETY: inside the mapping made above (checked by `arena_sections`); the source range is
        // inside the file (checked too). The rest of the section stays zero (`.bss`).
        unsafe { std::ptr::copy_nonoverlapping(exe[s.raw_off..].as_ptr(), s.va as *mut u8, n) };
        bytes += n;
    }
    // Imports: every IAT cell gets a trap address naming its import (the shipped IAT is pre-bound by
    // the packer to addresses of another machine's DLLs), so `orig!` declarations that call an
    // import (`site`, `cell`) reach the native implementation registered for it ([`crate::imports`]).
    let all = all_sections(&exe)?;
    let imports = parse_imports(&exe, &all)?;
    let mut traps = Vec::with_capacity(imports.len());
    for (i, (cell, name)) in imports.into_iter().enumerate() {
        let trap = TRAP_BASE + TRAP_STRIDE * i as u32;
        if !(RDATA.0..RDATA.1).contains(&(cell as usize)) {
            return Err(format!("gta_sa.exe: IAT cell {cell:#x} outside .rdata"));
        }
        // SAFETY: the cell lies in `.rdata`, inside the mapping, still writable here.
        unsafe { (cell as *mut u32).write_unaligned(trap) };
        traps.push(Import {
            trap,
            name: Box::leak(name.into_boxed_str()),
        });
    }
    let nimp = traps.len();
    let _ = IMPORTS.set(traps);
    // The game `.text` bytes (not mapped, not executed): only to read the IAT cell of an import
    // call site (`call dword ptr [cell]`) for `orig!`'s `site` form.
    let text = all
        .iter()
        .find(|s| s.va == 0x0040_1000)
        .ok_or("gta_sa.exe: no .text at 0x401000")?;
    let _ = TEXT.set(exe[text.raw_off..text.raw_off + text.raw_size].to_vec());
    sa_core::orig::registry::set_site_resolver(resolve_site);
    let ro = map_read_only_code(&exe, &all)?;
    drop(exe);
    // `.rdata` read-only, as the original's loader does
    protect_read_only(RDATA.0, RDATA.1 - RDATA.0).map_err(|e| format!("mprotect(.rdata) failed: {e}"))?;
    sa_core::orig::registry::set_arena(lo, hi);
    Ok(format!(
        "{} sections, {bytes} initialised bytes at {lo:#x}..{hi:#x} from {} (SHA-256 verified; .bss zero), {nimp} IAT cells trapped, {ro} code bytes readable (no-CD protection reads)",
        secs.len(),
        label
    ))
}

/// The code-section ranges mapped read-only (never executable): game `.text` + `_rwcseg`, and
/// `.HOODLUM`.
const READ_ONLY_CODE: [(usize, usize); 2] = [(0x0040_1000, 0x0085_8000), (0x0155_6000, 0x0157_7000)];

/// Maps the bytes of game `.text`/`_rwcseg` and `.HOODLUM` at their addresses, read-only and not
/// executable. The no-CD build's protection chunks read code bytes and `.HOODLUM` dwords as data
/// (`adc eax, [0x685002]`, `mov eax, [0x1560904]`, ...) and turn them into addresses or constants;
/// with these bytes present the ports compute the same values in phase B as in phase A. Nothing in
/// these ranges ever runs: a call or jump into them still faults (no `PROT_EXEC`). Same MVP-only
/// exe dependency as the arena (platform.md §8, decision 1).
fn map_read_only_code(exe: &[u8], all: &[Section]) -> Result<usize, String> {
    let mut total = 0;
    for &(lo, hi) in &READ_ONLY_CODE {
        map_fixed(lo, hi - lo).map_err(|e| format!("cannot map the code bytes: {e}"))?;
        for s in all.iter().filter(|s| s.va >= lo && s.va < hi) {
            let n = s.raw_size.min(s.vsize.max(s.raw_size)).min(hi - s.va);
            let src = exe
                .get(s.raw_off..s.raw_off + n)
                .ok_or_else(|| format!("gta_sa.exe: section at {:#x} does not fit", s.va))?;
            // SAFETY: inside the mapping made above (`n` is clipped to it).
            unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), s.va as *mut u8, n) };
            total += n;
        }
        protect_read_only(lo, hi - lo)?;
    }
    Ok(total)
}

/// wasm32: everything below this address of linear memory is the legacy image's address space (the module is
/// linked with its data, heap and stack above it: `--global-base`, see `tools/web/build.sh`).
#[cfg(target_arch = "wasm32")]
pub const WASM_LOW_END: usize = 0x0160_0000;

/// Makes `[lo, lo + len)` zeroed, writable memory at exactly that address: a fixed anonymous mapping natively (never
/// over an existing one); on wasm32 the reserved low linear memory, which is already there and zero.
pub fn map_fixed(lo: usize, len: usize) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        static PROBE: u8 = 0;
        let base = std::ptr::addr_of!(PROBE) as usize;
        if lo == 0 || lo + len > WASM_LOW_END || base < WASM_LOW_END {
            return Err(format!("{lo:#x}..{:#x} is not in the reserved low linear memory (data at {base:#x})", lo + len));
        }
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // SAFETY: an anonymous private mapping at a fixed address that MAP_FIXED_NOREPLACE refuses to place over
        // anything already mapped; it is never unmapped.
        let p = unsafe {
            libc::mmap(
                lo as *mut libc::c_void,
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_FIXED_NOREPLACE,
                -1,
                0,
            )
        };
        if p as usize != lo {
            return Err(format!("{lo:#x}..{:#x}: mmap returned {p:?}", lo + len));
        }
        Ok(())
    }
}

/// Makes a [`map_fixed`] range read-only (natively; wasm32 has no page protection).
pub fn protect_read_only(lo: usize, len: usize) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (lo, len);
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // SAFETY: a range [`map_fixed`] mapped.
        if unsafe { libc::mprotect(lo as *mut libc::c_void, len, libc::PROT_READ) } != 0 {
            return Err(format!("mprotect({lo:#x}) failed"));
        }
        Ok(())
    }
}

/// First trap address written into an IAT cell; import `i` gets `TRAP_BASE + TRAP_STRIDE * i`
/// (below the image, never a port address).
pub const TRAP_BASE: u32 = 0x0002_0000;
pub const TRAP_STRIDE: u32 = 0x10;

/// One import of the exe.
pub struct Import {
    pub trap: u32,
    /// `DLL!name` (`DLL!#ordinal` for an ordinal import), DLL as the import table spells it.
    pub name: &'static str,
}

static IMPORTS: std::sync::OnceLock<Vec<Import>> = std::sync::OnceLock::new();
static TEXT: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();

/// The imports, in IAT order (empty before [`map`]).
pub fn imports() -> &'static [Import] {
    IMPORTS.get().map_or(&[], |v| v.as_slice())
}

/// `orig!`'s `site` form: the trap address in the IAT cell a `call dword ptr [cell]` at `site`
/// reads (0 if `site` is not such an instruction).
fn resolve_site(site: u32) -> usize {
    let Some(text) = TEXT.get() else { return 0 };
    let o = (site as usize).wrapping_sub(0x0040_1000);
    match text.get(o..o + 6) {
        Some([0xFF, 0x15, c @ ..]) => {
            let cell = u32::from_le_bytes([c[0], c[1], c[2], c[3]]) as usize;
            if !sa_core::orig::registry::arena_contains(cell, 4) {
                return 0;
            }
            sa_core::mem::GPtr::<u32>::from_addr(cell).read() as usize
        }
        _ => 0,
    }
}

/// Every section header (VA, sizes, file offset).
fn all_sections(exe: &[u8]) -> Result<Vec<Section>, String> {
    let pe = u32_at(exe, 0x3C)? as usize;
    let (n, opt) = (u16_at(exe, pe + 6)? as usize, u16_at(exe, pe + 20)? as usize);
    let base = u32_at(exe, pe + 24 + 28)? as usize;
    (0..n)
        .map(|i| {
            let h = pe + 24 + opt + i * 40;
            Ok(Section {
                vsize: u32_at(exe, h + 8)? as usize,
                va: base + u32_at(exe, h + 12)? as usize,
                raw_size: u32_at(exe, h + 16)? as usize,
                raw_off: u32_at(exe, h + 20)? as usize,
            })
        })
        .collect()
}

/// The import table: `(IAT cell VA, "DLL!name")`, names from the OriginalFirstThunk arrays.
fn parse_imports(exe: &[u8], secs: &[Section]) -> Result<Vec<(u32, String)>, String> {
    const BASE: usize = 0x0040_0000;
    let off = |rva: usize| -> Result<usize, String> {
        secs.iter()
            .find(|s| BASE + rva >= s.va && BASE + rva < s.va + s.vsize.max(s.raw_size))
            .map(|s| s.raw_off + BASE + rva - s.va)
            .ok_or_else(|| format!("gta_sa.exe: RVA {rva:#x} outside the image"))
    };
    let cstr = |o: usize| -> String {
        exe[o..].iter().take_while(|&&c| c != 0).map(|&c| c as char).collect()
    };
    let pe = u32_at(exe, 0x3C)? as usize;
    let dir = u32_at(exe, pe + 24 + 104)? as usize;
    let mut out = Vec::new();
    let mut d = off(dir)?;
    loop {
        let oft = u32_at(exe, d)? as usize;
        let name = u32_at(exe, d + 12)? as usize;
        let ft = u32_at(exe, d + 16)? as usize;
        if name == 0 {
            break;
        }
        let dll = cstr(off(name)?);
        let names = off(if oft != 0 { oft } else { ft })?;
        for i in 0.. {
            let t = u32_at(exe, names + 4 * i)?;
            if t == 0 {
                break;
            }
            let f = if t & 0x8000_0000 != 0 {
                format!("#{}", t & 0xFFFF)
            } else {
                cstr(off(t as usize)? + 2)
            };
            out.push(((BASE + ft + 4 * i) as u32, format!("{dll}!{f}")));
        }
        d += 20;
    }
    Ok(out)
}

static STEP_PTR: AtomicPtr<u8> = AtomicPtr::new(std::ptr::null_mut());
static STEP_LEN: AtomicUsize = AtomicUsize::new(0);

/// Records the step name the crash reporter prints.
pub fn set_step(name: &'static str) {
    STEP_LEN.store(0, Ordering::SeqCst);
    STEP_PTR.store(name.as_ptr() as *mut u8, Ordering::SeqCst);
    STEP_LEN.store(name.len(), Ordering::SeqCst);
}

#[cfg(not(target_arch = "wasm32"))]
fn write_err(b: &[u8]) {
    // SAFETY: write(2) of a valid buffer; async-signal-safe.
    unsafe { libc::write(2, b.as_ptr().cast(), b.len()) };
}

#[cfg(not(target_arch = "wasm32"))]
fn hex(mut v: usize, out: &mut [u8; 18]) -> &[u8] {
    out[0] = b'0';
    out[1] = b'x';
    let mut digits = [0u8; 16];
    let mut n = 0;
    loop {
        digits[n] = b"0123456789abcdef"[v & 0xF];
        n += 1;
        v >>= 4;
        if v == 0 {
            break;
        }
    }
    for i in 0..n {
        out[2 + i] = digits[n - 1 - i];
    }
    &out[..2 + n]
}

#[cfg(not(target_arch = "wasm32"))]
extern "C" fn on_fault(sig: libc::c_int, info: *mut libc::siginfo_t, ctx: *mut libc::c_void) {
    // SAFETY: the kernel passes a valid siginfo for SA_SIGINFO handlers.
    let addr = unsafe { (*info).si_addr() } as usize;
    let mut buf = [0u8; 18];
    write_err(if sig == libc::SIGBUS {
        b"\nsa-game: SIGBUS"
    } else {
        b"\nsa-game: SIGSEGV"
    });
    write_err(b" at ");
    write_err(hex(addr, &mut buf));
    write_err(b" [");
    write_err(section_of(addr).as_bytes());
    write_err(b"] at eip ");
    // SAFETY: SA_SIGINFO handlers get the interrupted context as a ucontext_t.
    #[cfg(target_arch = "x86")]
    let eip = unsafe { (*(ctx as *const libc::ucontext_t)).uc_mcontext.gregs[libc::REG_EIP as usize] } as usize;
    #[cfg(not(target_arch = "x86"))]
    let eip = { let _ = ctx; 0usize };
    let mut buf2 = [0u8; 18];
    write_err(hex(eip, &mut buf2));
    // the reporter's own address: `eip - on_fault` + its link address locates the faulting code in a PIE build
    write_err(b" (on_fault at ");
    let mut buf3 = [0u8; 18];
    write_err(hex(on_fault as *const () as usize, &mut buf3));
    write_err(b") during step `");
    let (p, n) = (STEP_PTR.load(Ordering::SeqCst), STEP_LEN.load(Ordering::SeqCst));
    if !p.is_null() {
        // SAFETY: set_step stores a &'static str.
        write_err(unsafe { std::slice::from_raw_parts(p, n) });
    }
    write_err(b"`: a ported function dereferenced memory phase B does not provide (see docs/arch/platform.md, Legacy address arena)\n");
    // SAFETY: _exit is async-signal-safe.
    unsafe { libc::_exit(3) };
}

/// wasm32: a memory fault is a trap the page reports; nothing to install.
#[cfg(target_arch = "wasm32")]
pub fn install_crash_reporter() {}

/// Installs the SIGSEGV/SIGBUS reporter.
#[cfg(not(target_arch = "wasm32"))]
pub fn install_crash_reporter() {
    // SAFETY: a plain sigaction with an SA_SIGINFO handler that only calls async-signal-safe
    // functions.
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = on_fault as *const () as usize;
        sa.sa_flags = libc::SA_SIGINFO;
        libc::sigemptyset(&mut sa.sa_mask);
        libc::sigaction(libc::SIGSEGV, &sa, std::ptr::null_mut());
        libc::sigaction(libc::SIGBUS, &sa, std::ptr::null_mut());
    }
}

/// Ends the process with a report if an unprobed step runs longer than [`PROBE_TIMEOUT`]
/// (dropping it disarms it).
pub struct Watchdog {
    done: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Watchdog {
    #[cfg(target_arch = "wasm32")]
    pub fn start(_step: &'static str) -> Watchdog {
        Watchdog {
            done: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn start(step: &'static str) -> Watchdog {
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let d = done.clone();
        std::thread::spawn(move || {
            let t = web_time::Instant::now();
            while t.elapsed() < PROBE_TIMEOUT {
                if d.load(Ordering::SeqCst) {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            eprintln!(
                "\nsa-game: step `{step}` made no progress for {} s (a wait that phase B never satisfies); stopping",
                PROBE_TIMEOUT.as_secs()
            );
            std::process::exit(4);
        });
        Watchdog { done }
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.done.store(true, Ordering::SeqCst);
    }
}

/// How long a probed step may run.
pub const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Runs `f` in a forked child first. Returns `Some(report)` if the child died from a memory
/// fault (the crash reporter's line), `None` if it finished (the caller then runs the step for
/// real in this process). The child's side effects (game memory, open files) are discarded; only
/// files it wrote in the user directory remain.
#[cfg(target_arch = "wasm32")]
pub fn probe(_f: impl FnOnce()) -> Option<String> {
    None
}

/// See the native [`probe`].
#[cfg(not(target_arch = "wasm32"))]
pub fn probe(f: impl FnOnce()) -> Option<String> {
    let mut fds = [0i32; 2];
    // SAFETY: pipe(2) into a local array.
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return None;
    }
    // SAFETY: the game thread is the only thread running ported code; the child only runs `f`
    // and leaves with _exit.
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        // SAFETY: plain fd manipulation in the child.
        unsafe {
            libc::close(fds[0]);
            libc::dup2(fds[1], 2);
        }
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        // SAFETY: leave without running destructors or atexit handlers of the parent's state.
        unsafe { libc::_exit(0) };
    }
    // SAFETY: closing our copy of the write end.
    unsafe { libc::close(fds[1]) };
    if pid < 0 {
        // SAFETY: closing the read end.
        unsafe { libc::close(fds[0]) };
        return None;
    }
    let mut out = Vec::new();
    let mut buf = [0u8; 512];
    let mut status = 0;
    let start = web_time::Instant::now();
    // SAFETY: making our read end non-blocking.
    unsafe { libc::fcntl(fds[0], libc::F_SETFL, libc::O_NONBLOCK) };
    let hung = loop {
        // SAFETY: read(2) into a local buffer.
        let n = unsafe { libc::read(fds[0], buf.as_mut_ptr().cast(), buf.len()) };
        if n > 0 {
            out.extend_from_slice(&buf[..n as usize]);
            continue;
        }
        // SAFETY: polling our own child.
        if unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) } == pid {
            break false;
        }
        if start.elapsed() > PROBE_TIMEOUT {
            // SAFETY: killing and reaping our own child (by pid, nothing else).
            unsafe {
                libc::kill(pid, libc::SIGKILL);
                libc::waitpid(pid, &mut status, 0);
            }
            break true;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    // SAFETY: closing the read end.
    unsafe { libc::close(fds[0]) };
    if hung {
        return Some(format!(
            "sa-game: no progress for {} s (a wait that phase B never satisfies)",
            PROBE_TIMEOUT.as_secs()
        ));
    }
    let crashed = libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 3 || libc::WIFSIGNALED(status);
    crashed.then(|| {
        let s = String::from_utf8_lossy(&out);
        s.lines()
            .find(|l| l.starts_with("sa-game: SIG"))
            .unwrap_or("crashed")
            .trim()
            .to_owned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn hex_and_sections() {
        assert_eq!(hex(0xB70198, &mut [0; 18]), b"0xb70198");
        assert!(section_of(0xB70198).starts_with(".bss"));
        assert!(section_of(0x858A50).starts_with(".rdata"));
    }
}
