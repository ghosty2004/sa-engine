//! Native replacements for the statically linked MSVC CRT functions the ports call (catalogue
//! class `excluded:crt`: phase B replaces them instead of porting them). Registered at their
//! original addresses in `orig!`'s address registry ([`crate::native`]), with the original cdecl
//! convention, so every declaration of them reaches these.

#![allow(unsafe_code)]

use sa_core::mem::GPtr;

/// The C heap: libc natively; on wasm32 the Rust allocator with a 16-byte size header (the blocks
/// lie above the reserved legacy address range, see [`crate::arena::WASM_LOW_END`]).
#[cfg(not(target_arch = "wasm32"))]
use libc as sys;

#[cfg(target_arch = "wasm32")]
mod sys {
    use std::alloc::{Layout, alloc_zeroed, dealloc};

    const HDR: usize = 16;

    fn layout(n: usize) -> Layout {
        Layout::from_size_align(n + HDR, 16).expect("C heap block layout")
    }

    unsafe fn finish(p: *mut u8, n: usize) -> *mut u8 {
        if p.is_null() {
            return p;
        }
        // SAFETY: a fresh block of at least HDR bytes.
        unsafe {
            p.cast::<usize>().write(n);
            p.add(HDR)
        }
    }

    /// Zeroed, like the fresh pages a native heap mostly hands out: dlmalloc on wasm32 reuses freed blocks at once,
    /// and a port that reads a field the original leaves uninitialised (an FX blueprint list link, seen as an
    /// out-of-bounds trap in `FindFxSystemBP` / `CreateEntityFx`) must not see stale pointers.
    pub unsafe fn malloc(n: usize) -> *mut u8 {
        // SAFETY: a non-zero layout.
        unsafe { finish(alloc_zeroed(layout(n)), n) }
    }

    pub unsafe fn calloc(k: usize, n: usize) -> *mut u8 {
        let n = k.saturating_mul(n);
        // SAFETY: a non-zero layout.
        unsafe { finish(alloc_zeroed(layout(n)), n) }
    }

    pub unsafe fn free(p: *mut u8) {
        if p.is_null() {
            return;
        }
        // SAFETY: `p` came from `malloc` / `calloc` / `realloc`: its header holds the size.
        unsafe {
            let b = p.sub(HDR);
            dealloc(b, layout(b.cast::<usize>().read()));
        }
    }

    pub unsafe fn realloc(p: *mut u8, n: usize) -> *mut u8 {
        if p.is_null() {
            // SAFETY: as malloc.
            return unsafe { malloc(n) };
        }
        // SAFETY: as free; the copy stays inside both blocks.
        unsafe {
            let old = p.sub(HDR).cast::<usize>().read();
            let q = malloc(n);
            if !q.is_null() {
                std::ptr::copy_nonoverlapping(p, q, old.min(n));
                free(p);
            }
            q
        }
    }
}

/// `malloc` (0x824257) / `operator new` (0x82119A) / `operator new[]` (0x821195). MSVC's
/// `malloc(0)` returns a unique pointer; so does glibc's.
pub extern "C-unwind" fn malloc(size: u32) -> GPtr<u8> {
    if crate::det::fixed_heap() {
        return crate::detheap::alloc(size as usize);
    }
    // SAFETY: plain C allocation; the game owns the block until `free`.
    GPtr::from_raw(unsafe { sys::malloc(size as usize) }.cast())
}

/// `free` (0x82413F) / `operator delete` (0x8214BD).
pub extern "C-unwind" fn free(p: GPtr<u8>) {
    if crate::det::fixed_heap() {
        return crate::detheap::free(p);
    }
    // SAFETY: `p` came from `malloc`/`calloc`/`realloc` above (or is null).
    unsafe { sys::free(p.raw().cast()) }
}

/// `calloc` (0x824416).
pub extern "C-unwind" fn calloc(n: u32, size: u32) -> GPtr<u8> {
    if crate::det::fixed_heap() {
        return crate::detheap::calloc(n as usize * size as usize);
    }
    // SAFETY: plain C allocation.
    GPtr::from_raw(unsafe { sys::calloc(n as usize, size as usize) }.cast())
}

/// `realloc` (0x824269).
pub extern "C-unwind" fn realloc(p: GPtr<u8>, size: u32) -> GPtr<u8> {
    if crate::det::fixed_heap() {
        return crate::detheap::realloc(p, size as usize);
    }
    // SAFETY: `p` came from this allocator (or is null).
    GPtr::from_raw(unsafe { sys::realloc(p.raw().cast(), size as usize) }.cast())
}

/// `strncmp` (0x8214D0): MSVC returns -1/0/1.
pub extern "C-unwind" fn strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 {
    for i in 0..n as isize {
        let (x, y) = (a.byte_add(i).read(), b.byte_add(i).read());
        if x != y {
            return if x < y { -1 } else { 1 };
        }
        if x == 0 {
            break;
        }
    }
    0
}

// ------------------------------------------------------------------ variadic functions ---
//
// `sprintf`/`printf`/`sscanf` take a variable number of stack arguments; their declarations differ
// in arity. A naked cdecl entry passes the address of the first stack argument to a Rust body that
// reads the words it needs (no unwinding through the naked frame: the bodies catch panics).

macro_rules! vararg_entry {
    ($entry:ident => $body:ident) => {
        #[cfg(target_arch = "x86")]
        #[unsafe(naked)]
        extern "C" fn $entry() {
            core::arch::naked_asm!(
                "lea eax, [esp + 4]",
                "push eax",
                "call {body}",
                "add esp, 4",
                "ret",
                body = sym $body,
            )
        }
        /// wasm32: the type-erased entry ([`sa_core::wabi`]); its argument words are the x86 stack words.
        #[cfg(target_arch = "wasm32")]
        extern "C" fn $entry(_callee: u32, args: *const u32, ret: *mut sa_core::wabi::Ret) {
            let r = $body(args);
            // SAFETY: the dispatcher passes its `Ret` slot.
            sa_core::wabi::put_ret(r, unsafe { &mut *ret });
        }
        /// Other targets: no original-ABI dispatch exists, the entry is never called.
        #[cfg(not(any(target_arch = "x86", target_arch = "wasm32")))]
        extern "C" fn $entry() {
            let _ = $body;
        }
    };
}

fn word(args: *const u32, i: usize) -> u32 {
    // SAFETY: a stack word of the caller's frame (the caller pushed at least the arguments the
    // format string consumes; words beyond that are the caller's frame, readable).
    unsafe { args.add(i).read() }
}

extern "C" fn sprintf_body(args: *const u32) -> i32 {
    std::panic::catch_unwind(|| {
        let (buf, fmt) = (word(args, 0), word(args, 1));
        let w = |i: usize| word(args, 2 + i);
        let s = crate::crt_fmt::format(&crate::crt_fmt::cbytes(fmt), &mut crate::crt_fmt::Args::new(&w));
        for (i, c) in s.iter().chain(std::iter::once(&0)).enumerate() {
            GPtr::<u8>::from_addr(buf as usize + i).write(*c);
        }
        s.len() as i32
    })
    .unwrap_or(-1)
}
vararg_entry!(sprintf => sprintf_body);

extern "C" fn printf_body(args: *const u32) -> i32 {
    std::panic::catch_unwind(|| {
        let w = |i: usize| word(args, 1 + i);
        let s = crate::crt_fmt::format(&crate::crt_fmt::cbytes(word(args, 0)), &mut crate::crt_fmt::Args::new(&w));
        if crate::env::var_os("SA_GAME_PRINTF").is_some() {
            eprint!("{}", String::from_utf8_lossy(&s));
        }
        s.len() as i32
    })
    .unwrap_or(-1)
}
vararg_entry!(printf => printf_body);

/// `fprintf(f, fmt, ...)` (0x823A30): formatted as `sprintf` does, written with `fwrite` (text mode
/// turns `\n` into `\r\n` in sa-platform's file table, as the MSVC CRT does). Returns the number of
/// characters formatted, -1 when the write is short.
extern "C" fn fprintf_body(args: *const u32) -> i32 {
    std::panic::catch_unwind(|| {
        let f = GPtr::<u8>::from_addr(word(args, 0) as usize);
        let w = |i: usize| word(args, 2 + i);
        let s = crate::crt_fmt::format(&crate::crt_fmt::cbytes(word(args, 1)), &mut crate::crt_fmt::Args::new(&w));
        let mut buf = s;
        let n = buf.len() as u32;
        if n == 0 {
            return 0;
        }
        let p = GPtr::<u8>::from_raw(buf.as_mut_ptr());
        if crate::bind::fwrite(p, 1, n, f) == n { n as i32 } else { -1 }
    })
    .unwrap_or(-1)
}
vararg_entry!(fprintf => fprintf_body);

extern "C" fn sscanf_body(args: *const u32) -> i32 {
    std::panic::catch_unwind(|| {
        use crate::crt_fmt::Store;
        let input = crate::crt_fmt::cbytes(word(args, 0));
        let fmt = crate::crt_fmt::cbytes(word(args, 1));
        crate::crt_fmt::scan(&input, &fmt, &mut |i, v| {
            let p = word(args, 2 + i) as usize;
            match v {
                Store::I32(x) => GPtr::<i32>::from_addr(p).write(x),
                Store::I16(x) => GPtr::<i16>::from_addr(p).write(x),
                Store::F32(x) => GPtr::<f32>::from_addr(p).write(x),
                Store::F64(x) => GPtr::<f64>::from_addr(p).write(x),
                Store::Bytes(b, nul) => {
                    for (k, c) in b.iter().enumerate() {
                        GPtr::<u8>::from_addr(p + k).write(*c);
                    }
                    if nul {
                        GPtr::<u8>::from_addr(p + b.len()).write(0);
                    }
                }
            }
        })
    })
    .unwrap_or(-1)
}
vararg_entry!(sscanf => sscanf_body);

// ---------------------------------------------------------------------- string functions ---

fn len(s: GPtr<u8>) -> usize {
    let mut n = 0;
    while s.byte_add(n as isize).read() != 0 {
        n += 1;
    }
    n
}
pub extern "C-unwind" fn strlen(s: GPtr<u8>) -> u32 {
    len(s) as u32
}
pub extern "C-unwind" fn strchr(s: GPtr<u8>, c: i32) -> GPtr<u8> {
    let c = c as u8;
    let mut p = s;
    loop {
        let v = p.read();
        if v == c {
            return p;
        }
        if v == 0 {
            return GPtr::NULL;
        }
        p = p.byte_add(1);
    }
}
pub extern "C-unwind" fn strrchr(s: GPtr<u8>, c: i32) -> GPtr<u8> {
    let c = c as u8;
    let n = len(s);
    (0..=n)
        .rev()
        .map(|i| s.byte_add(i as isize))
        .find(|p| p.read() == c)
        .unwrap_or(GPtr::NULL)
}
pub extern "C-unwind" fn strstr(a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> {
    let (ha, nb) = (crate::crt_fmt::cbytes(a.addr() as u32), crate::crt_fmt::cbytes(b.addr() as u32));
    if nb.is_empty() {
        return a;
    }
    ha.windows(nb.len())
        .position(|w| w == nb.as_slice())
        .map_or(GPtr::NULL, |i| a.byte_add(i as isize))
}
/// `strncpy`: copies at most `n` bytes, pads with NULs, no terminator when `src` is longer.
pub extern "C-unwind" fn strncpy(d: GPtr<u8>, s: GPtr<u8>, n: u32) -> GPtr<u8> {
    let mut end = false;
    for i in 0..n as isize {
        let c = if end { 0 } else { s.byte_add(i).read() };
        if c == 0 {
            end = true;
        }
        d.byte_add(i).write(c);
    }
    d
}
fn lower(c: u8) -> u8 {
    c.to_ascii_lowercase()
}
/// `_stricmp` (C locale): compares lower-cased bytes, -1/0/1.
pub extern "C-unwind" fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 {
    strnicmp(a, b, u32::MAX)
}
pub extern "C-unwind" fn strnicmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 {
    // (a u32 counter: `u32::MAX as isize` is -1 in the 32-bit process, an empty range)
    for i in 0..n {
        let i = i as usize as isize;
        let (x, y) = (lower(a.byte_add(i).read()), lower(b.byte_add(i).read()));
        if x != y {
            return if x < y { -1 } else { 1 };
        }
        if x == 0 {
            break;
        }
    }
    0
}
pub extern "C-unwind" fn strlwr(s: GPtr<u8>) -> GPtr<u8> {
    for i in 0..len(s) as isize {
        let p = s.byte_add(i);
        p.write(lower(p.read()));
    }
    s
}
static STRTOK: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// `strtok` (one static position; MSVC keeps it per thread, the game calls it from one thread).
pub extern "C-unwind" fn strtok(s: GPtr<u8>, delim: GPtr<u8>) -> GPtr<u8> {
    let d = crate::crt_fmt::cbytes(delim.addr() as u32);
    let mut p = if s.is_null() { STRTOK.load(std::sync::atomic::Ordering::Relaxed) } else { s.addr() };
    if p == 0 {
        return GPtr::NULL;
    }
    let at = |a: usize| GPtr::<u8>::from_addr(a).read();
    while at(p) != 0 && d.contains(&at(p)) {
        p += 1;
    }
    if at(p) == 0 {
        STRTOK.store(0, std::sync::atomic::Ordering::Relaxed);
        return GPtr::NULL;
    }
    let start = p;
    while at(p) != 0 && !d.contains(&at(p)) {
        p += 1;
    }
    if at(p) != 0 {
        GPtr::<u8>::from_addr(p).write(0);
        p += 1;
    }
    STRTOK.store(p, std::sync::atomic::Ordering::Relaxed);
    GPtr::from_addr(start)
}
pub extern "C-unwind" fn atol(s: GPtr<u8>) -> i32 {
    let mut r = 0;
    crate::crt_fmt::scan(&crate::crt_fmt::cbytes(s.addr() as u32), b"%d", &mut |_, x| {
        if let crate::crt_fmt::Store::I32(x) = x {
            r = x;
        }
    });
    r
}
pub extern "C-unwind" fn atof(s: GPtr<u8>) -> f64 {
    let mut r = 0.0;
    crate::crt_fmt::scan(&crate::crt_fmt::cbytes(s.addr() as u32), b"%lf", &mut |_, x| {
        if let crate::crt_fmt::Store::F64(x) = x {
            r = x;
        }
    });
    r
}
pub extern "C-unwind" fn isdigit(c: i32) -> i32 {
    if (b'0' as i32..=b'9' as i32).contains(&c) { 4 } else { 0 }
}
pub extern "C-unwind" fn floor(v: f64) -> f64 {
    v.floor()
}
pub extern "C-unwind" fn strcpy(d: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> {
    let n = len(s);
    for i in 0..=n as isize {
        d.byte_add(i).write(s.byte_add(i).read());
    }
    d
}
pub extern "C-unwind" fn strcat(d: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> {
    strcpy(d.byte_add(len(d) as isize), s);
    d
}
/// `strncat`: appends at most `n` bytes and always a terminator.
pub extern "C-unwind" fn strncat(d: GPtr<u8>, s: GPtr<u8>, n: u32) -> GPtr<u8> {
    let e = d.byte_add(len(d) as isize);
    let mut i = 0isize;
    while (i as u32) < n {
        let c = s.byte_add(i).read();
        if c == 0 {
            break;
        }
        e.byte_add(i).write(c);
        i += 1;
    }
    e.byte_add(i).write(0);
    d
}
/// `strcmp`: unsigned byte compare, -1/0/1 (MSVC's result).
pub extern "C-unwind" fn strcmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 {
    let mut i = 0isize;
    loop {
        let (x, y) = (a.byte_add(i).read(), b.byte_add(i).read());
        if x != y {
            return if x < y { -1 } else { 1 };
        }
        if x == 0 {
            return 0;
        }
        i += 1;
    }
}
pub extern "C-unwind" fn strupr(s: GPtr<u8>) -> GPtr<u8> {
    for i in 0..len(s) as isize {
        let p = s.byte_add(i);
        p.write(p.read().to_ascii_uppercase());
    }
    s
}
pub extern "C-unwind" fn fopen(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> {
    crate::bind::fopen(path, mode)
}
pub extern "C-unwind" fn fclose(f: GPtr<u8>) -> i32 {
    crate::bind::fclose(f)
}
pub extern "C-unwind" fn fread(buf: GPtr<u8>, size: u32, n: u32, f: GPtr<u8>) -> u32 {
    crate::bind::fread(buf, size, n, f)
}
pub extern "C-unwind" fn fwrite(buf: GPtr<u8>, size: u32, n: u32, f: GPtr<u8>) -> u32 {
    crate::bind::fwrite(buf, size, n, f)
}
pub extern "C-unwind" fn fgets(buf: GPtr<u8>, n: i32, f: GPtr<u8>) -> GPtr<u8> {
    crate::bind::fgets(buf, n, f)
}
pub extern "C-unwind" fn fseek(f: GPtr<u8>, offset: i32, origin: i32) -> i32 {
    crate::bind::fseek(f, offset, origin)
}
pub extern "C-unwind" fn ftell(f: GPtr<u8>) -> i32 {
    crate::bind::rwftell(f)
}
pub extern "C-unwind" fn feof(f: GPtr<u8>) -> i32 {
    crate::bind::with("feof", |p| p.files.eof(crate::bind::handle(f))) as i32 * 0x10
}
pub extern "C-unwind" fn fputs(s: GPtr<u8>, f: GPtr<u8>) -> i32 {
    let n = len(s) as u32;
    if crate::bind::fwrite(s, 1, n, f) == n { 0 } else { -1 }
}
pub extern "C-unwind" fn fflush(f: GPtr<u8>) -> i32 {
    crate::bind::with("fflush", |p| {
        let crate::bind::Platform { files, vfs, .. } = p;
        files.flush(vfs, crate::bind::handle(f)).map_or(-1, |()| 0)
    })
}
/// RW's `rwfexist(name)` (0x804200): `fopen(name, "r")` succeeds (closed again).
pub extern "C-unwind" fn rwfexist(name: GPtr<u8>) -> i32 {
    let mut mode = *b"r\0";
    let f = crate::bind::fopen(name, GPtr::from_raw(mode.as_mut_ptr()));
    if f.is_null() {
        return 0;
    }
    crate::bind::fclose(f);
    1
}

/// `pow(x, y)` (0x8220F0): the host's `powf` on f64. The MSVC CRT picks an SSE2 or x87 kernel
/// at run time and their last bits are not modelled (exact for the identity and integral cases
/// the boot uses, e.g. `RwImageSetGamma(1.0)`); a recorded approximation of phase B.
pub extern "C-unwind" fn pow(x: f64, y: f64) -> f64 {
    x.powf(y)
}
/// `acos(x)` (0x822394, cdecl, ST0): the model phase B uses for `_CIacos` (0x822380, `sa_math::libm::acos`); the
/// CRT's SSE2 / x87 kernel choice is not modelled (a recorded approximation of phase B, as for `pow`).
pub extern "C-unwind" fn acos(x: f64) -> f64 {
    sa_core::orig::x87_unary(0x0082_2380, x)
}
/// `modf(v, ip)` (0x8232F0): exact (`*ip` = the integral part, the fraction with v's sign returned;
/// infinities give a signed 0 fraction, NaN propagates), as the CRT computes it.
///
/// # Safety
/// `ip` is the caller's valid double pointer (the original signature).
pub unsafe extern "C-unwind" fn modf(v: f64, ip: *mut f64) -> f64 {
    let i = v.trunc();
    // SAFETY: see above.
    unsafe { ip.write_unaligned(i) };
    if v.is_infinite() { 0f64.copysign(v) } else { (v - i).copysign(v) }
}
pub extern "C-unwind" fn ceil(v: f64) -> f64 {
    v.ceil()
}
/// `_strdate(buf)` (0x823A8E): `MM/DD/YY` and a NUL into `buf` (9 bytes), returns `buf`. The original
/// takes the local date from `GetLocalTime`; natively the date is the host's UTC date (sa-platform has no
/// time zone).
pub extern "C-unwind" fn strdate(buf: GPtr<u8>) -> GPtr<u8> {
    let secs = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    let text = [
        b'0' + (m / 10) as u8,
        b'0' + (m % 10) as u8,
        b'/',
        b'0' + (d / 10) as u8,
        b'0' + (d % 10) as u8,
        b'/',
        b'0' + ((y % 100) / 10) as u8,
        b'0' + (y % 10) as u8,
        0,
    ];
    for (i, c) in text.iter().enumerate() {
        buf.byte_add(i as isize).write(*c);
    }
    buf
}

/// Days since 1970-01-01 to (year, month, day) in the proleptic Gregorian calendar.
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// `atexit`: the native process never runs the game's exit handlers (recorded only).
pub extern "C-unwind" fn atexit(_f: u32) -> i32 {
    0
}
extern "C-unwind" fn rand() -> i32 {
    sa_core::crt::rand_owned()
}
extern "C-unwind" fn srand(seed: u32) {
    sa_core::crt::srand_owned(seed)
}

/// The statically linked Media Foundation loader's startup (0x8195C0, cdecl, one argument): it
/// loads the platform DLL by name and resolves its entry points. Phase B has no Media Foundation:
/// the result is the original's failure value when the DLL cannot be loaded (0xFFFFF7D3, the
/// `esi` it returns after `LoadLibraryA` fails), so `CAEMFDecoder::InitLibrary` reports no
/// MF support (user tracks in MF formats unavailable).
extern "C-unwind" fn mf_library_startup(_flags: u32) -> u32 {
    0xFFFF_F7D3
}

/// The registry entry of a replacement: the function itself on x86 (its ABI is the original's), its type-erased
/// entry on wasm32 ([`sa_core::wabi`]).
macro_rules! ent {
    (raw $f:ident) => {
        $f as *const () as usize
    };
    ($f:ident; $($a:ident)*) => {{
        #[cfg(target_arch = "wasm32")]
        let e = sa_core::wabi_wrap!($f; $($a)*);
        #[cfg(not(target_arch = "wasm32"))]
        let e = $f as *const () as usize;
        e
    }};
}

/// Calls the registry entry `e` (an element constructor / destructor, thiscall) with `this`.
fn call_this(e: &sa_core::orig::registry::Entry, this: u32) -> u32 {
    #[cfg(target_arch = "x86")]
    {
        // SAFETY: a registered thiscall entry `T* T::T()` / `void T::~T()` (EAX ignored for the latter).
        let f = unsafe { std::mem::transmute::<usize, extern "thiscall-unwind" fn(u32) -> u32>(e.entry) };
        f(this)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut a = sa_core::wabi::Args::new();
        a.push(this);
        sa_core::wabi::call(e.entry, e.addr as usize, &a)
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "wasm32")))]
    {
        let _ = (e, this);
        unreachable!("no original-ABI dispatch on this target")
    }
}

/// Calls the registry entry `e` (a cdecl `int cmp(const void*, const void*)`).
fn call_cmp(e: &sa_core::orig::registry::Entry, a: u32, b: u32) -> i32 {
    #[cfg(target_arch = "x86")]
    {
        // SAFETY: a registered cdecl entry `int cmp(const void*, const void*)`.
        let f = unsafe { std::mem::transmute::<usize, extern "C-unwind" fn(u32, u32) -> i32>(e.entry) };
        f(a, b)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut x = sa_core::wabi::Args::new();
        x.push(a);
        x.push(b);
        sa_core::wabi::call(e.entry, e.addr as usize, &x)
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "wasm32")))]
    {
        let _ = (e, a, b);
        unreachable!("no original-ABI dispatch on this target")
    }
}

/// `(address, name, entry)` of every replacement.
pub fn entries() -> Vec<(u32, &'static str, usize)> {
    let mut v = vec![
        (0x0082_13AE, "CRT free (operator delete[])", ent!(free; p)),
        (0x0082_13D0, "CRT strchr", ent!(strchr; s c)),
        (0x0082_1490, "CRT strrchr", ent!(strrchr; s c)),
        (0x0082_1982, "CRT printf", ent!(raw printf)),
        (0x0082_19F0, "CRT floor", ent!(floor; v)),
        (0x0082_20F0, "CRT pow", ent!(pow; x y)),
        (0x0082_2394, "CRT acos", ent!(acos; x)),
        (0x0082_6590, "CRT strcpy", ent!(strcpy; d s)),
        (0x0082_65A0, "CRT strcat", ent!(strcat; d s)),
        (0x0082_6450, "CRT strncat", ent!(strncat; d s n)),
        (0x0082_63C0, "CRT strcmp", ent!(strcmp; a b)),
        (0x0080_A420, "RW rwstrrchr", ent!(strrchr; s c)),
        (0x0080_A400, "RW rwstrchr", ent!(strchr; s c)),
        (0x0080_A350, "RW rwstricmp", ent!(stricmp; a b)),
        (0x0080_A3A0, "RW rwstrupr", ent!(strupr; s)),
        (0x0080_A3D0, "RW rwstrlwr", ent!(strlwr; s)),
        (0x0080_4200, "RW rwfexist", ent!(rwfexist; name)),
        (0x0082_32D8, "CRT fopen", ent!(fopen; path mode)),
        (0x0082_318B, "CRT fclose", ent!(fclose; f)),
        (0x0082_3521, "CRT fread", ent!(fread; buf size n f)),
        (0x0082_3674, "CRT fwrite", ent!(fwrite; buf size n f)),
        (0x0082_3798, "CRT fgets", ent!(fgets; buf n f)),
        (0x0082_62B8, "CRT fputs", ent!(fputs; s f)),
        (0x0082_62A2, "CRT feof", ent!(feof; f)),
        (0x0082_374F, "CRT fseek", ent!(fseek; f offset origin)),
        (0x0082_3E86, "CRT fflush", ent!(fflush; f)),
        (0x0082_6261, "CRT ftell", ent!(ftell; f)),
        (0x0082_1B11, "CRT srand", ent!(srand; seed)),
        (0x0082_1B1E, "CRT rand", ent!(rand;)),
        (0x0082_1BB5, "CRT sprintf", ent!(raw sprintf)),
        (0x0082_3A30, "CRT fprintf", ent!(raw fprintf)),
        (0x0082_3A8E, "CRT _strdate", ent!(strdate; buf)),
        (0x0083_6FBA, "CRT _strupr", ent!(strupr; s)),
        (0x0082_1D1E, "CRT atexit", ent!(atexit; _f)),
        (0x0082_1F40, "CRT strncpy", ent!(strncpy; d s n)),
        (0x0082_20AD, "CRT sscanf", ent!(raw sscanf)),
        (0x0082_244B, "CRT strtok", ent!(strtok; s delim)),
        (0x0082_258E, "CRT atol", ent!(atol; s)),
        (0x0082_2650, "CRT strstr", ent!(strstr; a b)),
        (0x0082_2789, "CRT isdigit", ent!(isdigit; c)),
        (0x0082_29B6, "CRT _stricmp", ent!(stricmp; a b)),
        (0x0082_3820, "CRT ceil", ent!(ceil; v)),
        (0x0082_32F0, "CRT modf", ent!(modf; v ip)),
        (0x0082_3CEE, "CRT atof", ent!(atof; s)),
        (0x0082_45A0, "CRT strncpy (2)", ent!(strncpy; d s n)),
        (0x0082_6330, "CRT strlen", ent!(strlen; s)),
        (0x0083_5AC7, "CRT _strnicmp", ent!(strnicmp; a b n)),
        (0x0083_6C58, "CRT _strlwr", ent!(strlwr; s)),
        (
            0x0081_95C0,
            "MF library startup (statically linked loader; no Media Foundation natively)",
            ent!(mf_library_startup; _flags),
        ),
    ];
    v.extend(base_entries());
    v
}

/// `eh vector destructor iterator(ptr, size, count, dtor)` (0x821E02, stdcall): `dtor` (thiscall) on each of
/// `count` elements from the last to the first (`ptr += size * count`, then `ptr -= size` before each call; the
/// signed count is decremented first and the loop stops when it goes negative), through `orig!`'s registry. The
/// SEH frame (`__ArrayUnwind` on a throwing destructor) has no counterpart: no game destructor throws.
#[cfg(any(target_arch = "x86", target_arch = "wasm32"))]
pub(crate) fn eh_vector_dtor(ptr: u32, size: u32, count: i32, dtor: u32) {
    let Some(e) = sa_core::orig::registry::lookup(dtor as usize) else {
        panic!(
            "eh_vector_dtor_element is not ported; phases B/C have no original code to call ({dtor:#x} {})",
            sa_core::orig::registry::describe(dtor as usize)
        );
    };
    let f = |p: u32| {
        call_this(e, p);
    };
    let mut p = ptr.wrapping_add(size.wrapping_mul(count as u32));
    let mut n = count;
    loop {
        n = n.wrapping_sub(1);
        if n < 0 {
            break;
        }
        p = p.wrapping_sub(size);
        f(p);
    }
}

/// `eh vector constructor iterator(ptr, size, count, ctor, dtor)` (0x821D30, stdcall): `ctor`
/// (thiscall) on each of `count` elements (signed count, `size` bytes apart), through `orig!`'s
/// registry. The original's unwind path (destroying the constructed elements when a constructor
/// throws) has no counterpart: no game constructor throws.
#[cfg(any(target_arch = "x86", target_arch = "wasm32"))]
pub(crate) fn eh_vector_ctor(ptr: u32, size: u32, count: i32, ctor: u32, _dtor: u32) {
    let Some(e) = sa_core::orig::registry::lookup(ctor as usize) else {
        panic!(
            "eh_vector_ctor_element is not ported; phases B/C have no original code to call ({ctor:#x} {})",
            sa_core::orig::registry::describe(ctor as usize)
        );
    };
    let f = |p: u32| call_this(e, p);
    let mut p = ptr;
    let mut i = 0;
    while i < count {
        f(p);
        p = p.wrapping_add(size);
        i += 1;
    }
}

/// CRT `qsort(base, num, width, comp)` (0x8247E0), MSVC 8's algorithm as the exe has it (not the
/// host's: equal elements must end in the same order): an explicit stack of sub-arrays; parts of at
/// most 8 elements by the selection `shortsort` (0x824770: the last maximum by `comp(p, max) > 0`
/// swapped to the end); else the median of three (`comp(lo, mid)`, `comp(lo, hi)`, `comp(mid, hi)`,
/// swapping on > 0), the `loguy`/`higuy` partition around `mid` (which follows the element when it is
/// swapped), the run of elements equal to `mid` skipped, and the larger part pushed. Swaps are byte
/// by byte and skipped for identical pointers. `comp` (cdecl) is called through `orig!`'s registry.
#[cfg(any(target_arch = "x86", target_arch = "wasm32"))]
pub(crate) fn qsort(base: u32, num: u32, width: u32, comp: u32) {
    if num < 2 || width == 0 {
        return;
    }
    let Some(e) = sa_core::orig::registry::lookup(comp as usize) else {
        panic!(
            "qsort comparator is not ported; phases B/C have no original code to call ({comp:#x} {})",
            sa_core::orig::registry::describe(comp as usize)
        );
    };
    let f = |a: u32, b: u32| call_cmp(e, a, b);
    let swap = |a: u32, b: u32| {
        if a != b {
            for i in 0..width {
                let pa = GPtr::<u8>::from_addr(a.wrapping_add(i) as usize);
                let pb = GPtr::<u8>::from_addr(b.wrapping_add(i) as usize);
                let (x, y) = (pa.read(), pb.read());
                pa.write(y);
                pb.write(x);
            }
        }
    };
    let mut lostk = [0u32; 30];
    let mut histk = [0u32; 30];
    let mut stkptr: i32 = 0;
    let mut lo = base;
    let mut hi = base.wrapping_add(width.wrapping_mul(num - 1));
    loop {
        let size = (hi - lo) / width + 1;
        if size <= 8 {
            // shortsort (0x824770)
            let mut h = hi;
            while h > lo {
                let mut max = lo;
                let mut p = lo + width;
                while p <= h {
                    if f(p, max) > 0 {
                        max = p;
                    }
                    p += width;
                }
                swap(max, h);
                h -= width;
            }
        } else {
            let mut mid = lo + (size / 2) * width;
            if f(lo, mid) > 0 {
                swap(lo, mid);
            }
            if f(lo, hi) > 0 {
                swap(lo, hi);
            }
            if f(mid, hi) > 0 {
                swap(mid, hi);
            }
            let mut loguy = lo;
            let mut higuy = hi;
            loop {
                if mid > loguy {
                    loop {
                        loguy += width;
                        if !(loguy < mid && f(loguy, mid) <= 0) {
                            break;
                        }
                    }
                }
                if mid <= loguy {
                    loop {
                        loguy += width;
                        if !(loguy <= hi && f(loguy, mid) <= 0) {
                            break;
                        }
                    }
                }
                loop {
                    higuy -= width;
                    if !(higuy > mid && f(higuy, mid) > 0) {
                        break;
                    }
                }
                if higuy < loguy {
                    break;
                }
                swap(loguy, higuy);
                if mid == higuy {
                    mid = loguy;
                }
            }
            higuy += width;
            if mid < higuy {
                loop {
                    higuy -= width;
                    if !(higuy > mid && f(higuy, mid) == 0) {
                        break;
                    }
                }
            }
            if mid >= higuy {
                loop {
                    higuy -= width;
                    if !(higuy > lo && f(higuy, mid) == 0) {
                        break;
                    }
                }
            }
            if (higuy.wrapping_sub(lo) as i32) >= (hi.wrapping_sub(loguy) as i32) {
                if lo < higuy {
                    lostk[stkptr as usize] = lo;
                    histk[stkptr as usize] = higuy;
                    stkptr += 1;
                }
                if loguy < hi {
                    lo = loguy;
                    continue;
                }
            } else {
                if loguy < hi {
                    lostk[stkptr as usize] = loguy;
                    histk[stkptr as usize] = hi;
                    stkptr += 1;
                }
                if lo < higuy {
                    hi = higuy;
                    continue;
                }
            }
        }
        stkptr -= 1;
        if stkptr < 0 {
            return;
        }
        lo = lostk[stkptr as usize];
        hi = histk[stkptr as usize];
    }
}

/// `vector constructor iterator(ptr, size, count, ctor)` (0x403D60, stdcall, protected entry): `ctor`
/// (thiscall) on each of `count` elements (signed; none when <= 0), `size` bytes apart.
#[cfg(any(target_arch = "x86", target_arch = "wasm32"))]
pub(crate) fn vector_ctor(ptr: u32, size: u32, count: i32, ctor: u32) {
    let Some(e) = sa_core::orig::registry::lookup(ctor as usize) else {
        panic!(
            "vector_ctor_element is not ported; phases B/C have no original code to call ({ctor:#x} {})",
            sa_core::orig::registry::describe(ctor as usize)
        );
    };
    let f = |p: u32| call_this(e, p);
    let mut p = ptr;
    for _ in 0..count.max(0) {
        f(p);
        p = p.wrapping_add(size);
    }
}

#[cfg(target_arch = "x86")]
mod abi_entries {
    pub extern "stdcall-unwind" fn eh_vector_ctor(ptr: u32, size: u32, count: i32, ctor: u32, dtor: u32) {
        super::eh_vector_ctor(ptr, size, count, ctor, dtor)
    }
    pub extern "C-unwind" fn qsort(base: u32, num: u32, width: u32, comp: u32) {
        super::qsort(base, num, width, comp)
    }
    pub extern "stdcall-unwind" fn eh_vector_dtor(ptr: u32, size: u32, count: i32, dtor: u32) {
        super::eh_vector_dtor(ptr, size, count, dtor)
    }
    pub extern "stdcall-unwind" fn vector_ctor(ptr: u32, size: u32, count: i32, ctor: u32) {
        super::vector_ctor(ptr, size, count, ctor)
    }
}
#[cfg(target_arch = "wasm32")]
mod abi_entries {
    pub(crate) use super::{eh_vector_ctor, eh_vector_dtor, qsort, vector_ctor};
}

fn base_entries() -> Vec<(u32, &'static str, usize)> {
    #[cfg(any(target_arch = "x86", target_arch = "wasm32"))]
    use abi_entries::{eh_vector_ctor, eh_vector_dtor, qsort, vector_ctor};
    vec![
        #[cfg(any(target_arch = "x86", target_arch = "wasm32"))]
        (
            0x0082_1D30,
            "CRT eh vector constructor iterator",
            ent!(eh_vector_ctor; ptr size count ctor _dtor),
        ),
        #[cfg(any(target_arch = "x86", target_arch = "wasm32"))]
        (0x0082_47E0, "CRT qsort", ent!(qsort; base num width comp)),
        #[cfg(any(target_arch = "x86", target_arch = "wasm32"))]
        (0x0082_1E02, "CRT eh vector destructor iterator", ent!(eh_vector_dtor; ptr size count dtor)),
        #[cfg(any(target_arch = "x86", target_arch = "wasm32"))]
        (0x0040_3D60, "CRT vector constructor iterator", ent!(vector_ctor; ptr size count ctor)),
        (0x0082_1195, "CRT operator new[]", ent!(malloc; n)),
        (0x0082_119A, "CRT operator new", ent!(malloc; n)),
        (0x0082_14BD, "CRT operator delete", ent!(free; p)),
        (0x0082_14D0, "CRT strncmp", ent!(strncmp; a b n)),
        (0x0082_413F, "CRT free", ent!(free; p)),
        (0x0082_4257, "CRT malloc", ent!(malloc; n)),
        (0x0082_4269, "CRT realloc", ent!(realloc; p n)),
        (0x0082_4416, "CRT calloc", ent!(calloc; k n)),
    ]
}

#[cfg(test)]
mod strdate_tests {
    #[test]
    fn civil_dates() {
        assert_eq!(super::civil_from_days(0), (1970, 1, 1));
        assert_eq!(super::civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(super::civil_from_days(20_768), (2026, 11, 11));
    }
}
