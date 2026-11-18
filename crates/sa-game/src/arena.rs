//! The legacy address arena and the crash reporter.
//!
//! Ported code still reaches game state by its 1.0 US address (`GPtr::from_addr(CONTROLS_MANAGER)`,
//! `rdata(0x8747A8)`, `g::<T>(addr)`, vtable addresses stored in objects, `globals!` declarations;
//! ADR-001 §3 plans an engine context later). Phase B runs as a 32-bit process, so `sa-game` maps
//! the original's data sections at their original virtual addresses: `.rdata` (read-only, as in the
//! original), `.data` with its zero-fill `.bss` extent, `_TEXT_HA`, `_rwdseg` and `.rsrc`
//! (0x858000–0xCB1000). Their initial contents come from an [`Image`]: the built-in image
//! (`sa_image`, generated from the exe's data sections) or the user's own `gta_sa.exe` (1.0 US,
//! verified by its SHA-256), read at start-up only. The import table and the import call sites
//! always come from `sa_image`. No code section is executable: every function runs as its Rust port.
//!
//! `globals!` storage aliases the arena (`sa_core::mem::Global::ptr` returns the original address
//! once [`sa_core::orig::registry::set_arena`] is called), so raw-address and `globals!` accesses
//! see one copy.
//!
//! [`install_crash_reporter`] turns a SIGSEGV/SIGBUS into a one-line report naming the boot step
//! that was running and the section of the faulting address, instead of a bare crash.

#![allow(unsafe_code)]

use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

/// First and one-past-last address of the arena (`.rdata` … `.rsrc`).
pub const ARENA: (usize, usize) = (sa_image::RANGE.0 as usize, sa_image::RANGE.1 as usize);

/// SHA-256 of `gta_sa.exe` 1.0 US (CLAUDE.md, docs/PLAN.md §1).
pub const EXE_SHA256: &str = "a559aa772fd136379155efa71f00c47aad34bbfeae6196b0fe1047d0645cbd26";

/// `.rdata`: mapped read-only after loading, as the original's loader does.
const RDATA: (usize, usize) = (0x0085_8000, 0x008A_4000);

/// The section of the 1.0 US image an address belongs to.
pub fn section_of(a: usize) -> &'static str {
    match a {
        0x0040_1000..0x0085_7000 => ".text (game code: never executed in phase B)",
        0x0085_7000..0x0085_8000 => "_rwcseg (code: never executed in phase B)",
        0x0085_8000..0x008A_4000 => ".rdata (read-only)",
        0x008A_4000..0x008E_4000 => ".data",
        0x008E_4000..0x00C9_E000 => ".bss",
        0x00C9_E000..0x00CB_1000 => "_TEXT_HA/_rwdseg/.rsrc",
        0x00CB_1000..0x0155_6000 => "no-CD image (.text/.data copy, not mapped)",
        0x0155_6000..0x0157_7000 => ".HOODLUM (never executed in phase B)",
        0..0x1_0000 => "null page (a null game pointer)",
        _ => "outside the original image (a host pointer)",
    }
}

/// Where the arena's initial contents come from.
pub enum Image {
    /// The built-in image (`sa_image`).
    Builtin,
    /// The data sections of these bytes of `gta_sa.exe` 1.0 US.
    Exe(Vec<u8>),
    /// The built-in image, after comparing it byte for byte with the data sections of these exe bytes.
    Check(Vec<u8>),
}

/// The image source the command line asks for (`--image exe|builtin|check`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageKind {
    Exe,
    Builtin,
    Check,
}

impl ImageKind {
    pub fn parse(s: &str) -> Result<ImageKind, String> {
        match s {
            "exe" => Ok(ImageKind::Exe),
            "builtin" => Ok(ImageKind::Builtin),
            "check" => Ok(ImageKind::Check),
            _ => Err(format!("--image {s}: expected exe, builtin or check")),
        }
    }

    /// The image of this kind, reading the exe through `read` when it needs one.
    pub fn image(self, read: impl FnOnce() -> Result<Vec<u8>, String>) -> Result<Image, String> {
        Ok(match self {
            ImageKind::Builtin => Image::Builtin,
            ImageKind::Exe => Image::Exe(read()?),
            ImageKind::Check => Image::Check(read()?),
        })
    }
}

/// One PE section header.
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

/// Copies the initialised bytes of the exe's sections inside the arena into `dst` (the arena's extent, zeroed).
fn copy_exe_sections(exe: &[u8], dst: &mut [u8]) -> Result<usize, String> {
    let mut bytes = 0;
    for s in all_sections(exe)?.iter().filter(|s| s.va >= ARENA.0 && s.va < ARENA.1) {
        if s.va + s.vsize.max(s.raw_size) > ARENA.1 || s.raw_off + s.raw_size > exe.len() {
            return Err(format!("gta_sa.exe: section at {:#x} does not fit", s.va));
        }
        // the rest of the section stays zero (`.bss`)
        let n = if s.vsize == 0 { s.raw_size } else { s.raw_size.min(s.vsize) };
        let o = s.va - ARENA.0;
        dst[o..o + n].copy_from_slice(&exe[s.raw_off..s.raw_off + n]);
        bytes += n;
    }
    Ok(bytes)
}

/// Copies the built-in image into `dst` (the arena's extent, zeroed).
fn copy_builtin(dst: &mut [u8]) -> usize {
    let mut bytes = 0;
    for (a, b) in sa_image::runs() {
        let o = a as usize - ARENA.0;
        dst[o..o + b.len()].copy_from_slice(b);
        bytes += b.len();
    }
    bytes
}

/// Maps the arena (once) and fills it from `image` (`label` names an exe in messages). Returns a one-line
/// summary.
pub fn map_image(image: Image, label: &str) -> Result<String, String> {
    if let Image::Exe(exe) | Image::Check(exe) = &image {
        let digest = crate::sha256::hex(exe);
        if digest != EXE_SHA256 {
            return Err(format!("{label}: SHA-256 {digest} is not gta_sa.exe 1.0 US ({EXE_SHA256})"));
        }
    }
    let (lo, hi) = ARENA;
    map_fixed(lo, hi - lo).map_err(|e| format!("cannot map the legacy address arena: {e}"))?;
    // SAFETY: the mapping made above, writable, owned by nothing else yet.
    let arena = unsafe { std::slice::from_raw_parts_mut(lo as *mut u8, hi - lo) };
    let (source, bytes) = match &image {
        Image::Exe(exe) => (format!("{label} (SHA-256 verified)"), copy_exe_sections(exe, arena)?),
        Image::Builtin | Image::Check(_) => ("the built-in image".to_owned(), copy_builtin(arena)),
    };
    let checked = match &image {
        Image::Check(exe) => check_against_exe(arena, exe, label)?,
        _ => String::new(),
    };
    let nimp = install_imports()?;
    sa_core::orig::registry::set_site_resolver(resolve_site);
    let ro = match &image {
        Image::Exe(exe) => map_read_only_code(exe)?,
        _ => 0,
    };
    // `.rdata` read-only, as the original's loader does
    protect_read_only(RDATA.0, RDATA.1 - RDATA.0).map_err(|e| format!("mprotect(.rdata) failed: {e}"))?;
    sa_core::orig::registry::set_arena(lo, hi);
    Ok(format!(
        "{bytes} initialised bytes at {lo:#x}..{hi:#x} from {source} (.bss zero){checked}, {nimp} IAT cells trapped{}",
        if ro > 0 { format!(", {ro} code bytes of the exe readable") } else { String::new() }
    ))
}

/// `--image check`: the built-in image in `arena` against the exe's data sections, the import table against the
/// exe's, and every call site against the exe's `.text`. Every difference is an error.
fn check_against_exe(arena: &[u8], exe: &[u8], label: &str) -> Result<String, String> {
    let mut want = vec![0u8; arena.len()];
    copy_exe_sections(exe, &mut want)?;
    let mut bad = Vec::new();
    let mut i = 0;
    while i < arena.len() {
        if arena[i] == want[i] {
            i += 1;
            continue;
        }
        // one entry per stretch of differences, until 16 equal bytes in a row
        let start = i;
        while i < arena.len() && arena[i..].iter().zip(&want[i..]).take(16).any(|(a, b)| a != b) {
            i += 1;
        }
        bad.push(format!("{:#x}..{:#x} ({})", ARENA.0 + start, ARENA.0 + i, section_of(ARENA.0 + start)));
    }
    let secs = all_sections(exe)?;
    let imports = parse_imports(exe, &secs)?;
    let generated: Vec<(u32, String)> = sa_image::IMPORTS.iter().map(|&(c, n)| (c, n.to_owned())).collect();
    if imports != generated {
        bad.push(format!("the import table ({} imports in the exe, {} built in)", imports.len(), generated.len()));
    }
    for &(site, cell) in &sa_image::CALL_SITES {
        let mut code = [0u8; 6];
        code[..2].copy_from_slice(&[0xFF, 0x15]);
        code[2..].copy_from_slice(&cell.to_le_bytes());
        if image_bytes(exe, &secs, site as usize, 6) != Some(&code[..]) {
            bad.push(format!("call site {site:#x} (built in: call [{cell:#x}])"));
        }
    }
    if !bad.is_empty() {
        return Err(format!(
            "--image check: the built-in image differs from {label} in {} places: {}",
            bad.len(),
            bad.iter().take(20).cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    Ok(format!(
        ", identical to {label} ({} bytes, {} imports, {} call sites checked)",
        arena.len(),
        generated.len(),
        sa_image::CALL_SITES.len()
    ))
}

/// The raw bytes of the exe at a virtual address (inside one section's raw data).
fn image_bytes<'a>(exe: &'a [u8], secs: &[Section], va: usize, n: usize) -> Option<&'a [u8]> {
    let s = secs.iter().find(|s| va >= s.va && va + n <= s.va + s.raw_size)?;
    exe.get(s.raw_off + va - s.va..s.raw_off + va - s.va + n)
}

/// The code-section ranges mapped read-only (never executable) when the exe is the image source:
/// game `.text` + `_rwcseg`, and `.HOODLUM`.
const READ_ONLY_CODE: [(usize, usize); 2] = [(0x0040_1000, 0x0085_8000), (0x0155_6000, 0x0157_7000)];

/// Maps the bytes of game `.text`/`_rwcseg` and `.HOODLUM` at their addresses, read-only and not
/// executable. The ports read the code bytes the no-CD protection chunks and MSVC's switch byte
/// tables use as data from constants, so the built-in image needs none of this; with the exe as
/// source a code read that is not a constant yet still sees the original bytes instead of faulting.
/// Nothing in these ranges ever runs: a call or jump into them still faults (no `PROT_EXEC`).
fn map_read_only_code(exe: &[u8]) -> Result<usize, String> {
    let all = all_sections(exe)?;
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

/// Writes `new` over `original` at `addr` inside the mapped image when the original bytes are there, lifting a
/// read-only protection for the write (what samp.dll does with `VirtualProtect` before its data patches).
/// Returns false when the bytes differ (nothing written).
pub fn patch_image(addr: usize, original: &[u8], new: &[u8]) -> Result<bool, String> {
    // SAFETY: callers pass addresses inside the image `map_image` mapped (read access is always allowed).
    let current = unsafe { std::slice::from_raw_parts(addr as *const u8, original.len()) };
    if current != original {
        return Ok(false);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let (page, len) = {
        let page = addr & !0xFFF;
        let len = (addr + new.len()).div_ceil(0x1000) * 0x1000 - page;
        // SAFETY: pages of the mapped image.
        if unsafe { libc::mprotect(page as *mut libc::c_void, len, libc::PROT_READ | libc::PROT_WRITE) } != 0 {
            return Err(format!("mprotect({page:#x}) failed"));
        }
        (page, len)
    };
    // SAFETY: as above; the range is writable now.
    unsafe { std::ptr::copy_nonoverlapping(new.as_ptr(), addr as *mut u8, new.len()) };
    #[cfg(not(target_arch = "wasm32"))]
    {
        let in_rdata = (RDATA.0..RDATA.1).contains(&page);
        if in_rdata {
            protect_read_only(page, len)?;
        }
    }
    Ok(true)
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

/// The imports, in IAT order (empty before [`map_image`]).
pub fn imports() -> &'static [Import] {
    IMPORTS.get().map_or(&[], |v| v.as_slice())
}

/// Writes a trap address naming its import into every IAT cell (the shipped IAT is pre-bound by the packer
/// to addresses of another machine's DLLs), so `orig!` declarations that call an import (`site`, `cell`)
/// reach the native implementation registered for it ([`crate::imports`]).
fn install_imports() -> Result<usize, String> {
    let mut traps = Vec::with_capacity(sa_image::IMPORTS.len());
    for (i, &(cell, name)) in sa_image::IMPORTS.iter().enumerate() {
        let trap = TRAP_BASE + TRAP_STRIDE * i as u32;
        if !(RDATA.0..RDATA.1).contains(&(cell as usize)) {
            return Err(format!("IAT cell {cell:#x} outside .rdata"));
        }
        // SAFETY: the cell lies in `.rdata`, inside the arena mapping, still writable here.
        unsafe { (cell as *mut u32).write_unaligned(trap) };
        traps.push(Import { trap, name });
    }
    let n = traps.len();
    let _ = IMPORTS.set(traps);
    Ok(n)
}

/// `orig!`'s `site` form: the trap address in the IAT cell the `call dword ptr [cell]` at `site`
/// reads (0 if `site` is not such a call site).
fn resolve_site(site: u32) -> usize {
    match sa_image::call_site_cell(site) {
        Some(cell) if sa_core::orig::registry::arena_contains(cell as usize, 4) => {
            sa_core::mem::GPtr::<u32>::from_addr(cell as usize).read() as usize
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

    #[test]
    fn builtin_image_equals_the_exe() {
        let Some(exe) = sa_testkit::exe() else {
            eprintln!("note: gta_sa.exe not found, the built-in image not checked");
            return;
        };
        let mut arena = vec![0u8; ARENA.1 - ARENA.0];
        copy_builtin(&mut arena);
        let summary = check_against_exe(&arena, &exe, "gta_sa.exe").unwrap();
        assert!(summary.contains("identical"), "{summary}");
    }
}
