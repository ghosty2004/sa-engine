//! Typed calls to functions that may still be original (ADR-001 §4).
//!
//! A port never calls another catalogued function by its Rust name directly; it calls the function
//! declared with [`orig!`](crate::orig!):
//!
//! ```ignore
//! sa_core::orig! {
//!     /// CRT `rand()`
//!     pub fn rand() -> i32 = cdecl 0x821B1E => crate::crt::rand_owned;
//!     /// QueryPerformanceCounter as `CTimer::GetCurrentTimeInCycles` calls it
//!     pub fn qpc(out: *mut i64) -> i32 = stdcall site 0x561A88 => unported;
//! }
//! ```
//!
//! - Phase A: the call goes to the original address with the original convention. If that function
//!   is hooked, its hook table entry decides whether the Rust port or the original runs — one switch
//!   per function, so the hook-off check (PLAN §5 L3) also covers calls made from Rust.
//!   `site ADDR` names a `call dword ptr [cell]` instruction of the original (an import call): the
//!   cell is read from that instruction at call time, so call-site redirections made by sa-hook's
//!   determinism mode (virtual clock) apply to the Rust port exactly as to the original code.
//!   `callsite ADDR` names a `call rel32` instruction of the original the same way (its target is
//!   read at call time; determinism mode retargets the `Game::Idle` frame-limiter calls).
//!   `cell ADDR` calls the pointer stored at ADDR now (an IAT slot used by `jmp/mov reg, [cell]`
//!   forms, or a global function pointer). `vslot OFF` calls slot OFF (bytes) of the vtable of the
//!   first argument (C++ virtual call with `thiscall`, COM method with `stdcall`). `table CELL OFF`
//!   calls slot OFF of the function table the global pointer at CELL points to (RwEngineInstance).
//!   `reg("spec") ADDR` (HARNESS-5, [`crate::regconv`]) calls a register-convention function: the
//!   arguments are placed per the spec and [`crate::regconv::call_regs`] loads EAX..EDI and the stack
//!   words (`fn f(c: u8, p: GPtr<u32>) = reg("args = al stack") 0x6FD7C0 => path;`); an `X87`/`f32`
//!   result is read from ST0 with all 80 bits.
//! - Phases B/C: the call goes straight to the Rust path after `=>`. `unported` makes it a panic
//!   naming the function (phase B starts only when F2 holds, i.e. nothing is unported).
//!
//! Every declared function is a safe Rust function (trust model of [`crate::mem`]).
#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

static SITE: AtomicU32 = AtomicU32::new(0);

/// True in phase A (in-process with the original exe): only there does a fallback hook's
/// `<fn>_original` reach original code (ADR-002). Ports use it to panic with their own context
/// (e.g. the opcode of an unported script command) before calling a fallback in phases B/C.
pub const PHASE_A: bool = cfg!(sa_phase_a);

/// The original entry recorded by sa-hook for a `#[hook(fallback)]` port (ADR-002): the trampoline
/// (relocated prologue + `jmp` back) or a `.HOODLUM` stub's old target. Panics when the hook was not
/// installed: a fallback runs only below the hook's own shim, which exists only once installed.
#[cfg(sa_phase_a)]
#[doc(hidden)]
#[inline]
pub fn fallback_entry(cell: &AtomicU32, name: &str) -> usize {
    match cell.load(Ordering::Acquire) {
        0 => panic!("{name}: fallback to the original body before the hook was installed"),
        a => a as usize,
    }
}

/// Runs `f` with `ret_addr` recorded as the original return address of the call it makes.
///
/// Calls from Rust to an original function have a Rust return address. Where an observer keys on the
/// caller — sa-hook's determinism mode hashes `(caller, value)` of every `rand()` call — the port
/// names the call instruction's original return address (the address after the `call` in the
/// original function), so original and Rust runs hash identically. Nesting restores the outer value.
pub fn at_site<R>(ret_addr: u32, f: impl FnOnce() -> R) -> R {
    let prev = SITE.swap(ret_addr, Ordering::Relaxed);
    let r = f();
    SITE.store(prev, Ordering::Relaxed);
    r
}

/// The return address recorded by the innermost [`at_site`], if any.
pub fn current_site() -> Option<u32> {
    match SITE.load(Ordering::Relaxed) {
        0 => None,
        a => Some(a),
    }
}

/// The function a `call dword ptr [cell]` (`FF 15 cell`) at `site` calls right now.
#[cfg(sa_phase_a)]
pub fn import_at_call_site(site: u32) -> usize {
    use crate::mem::GPtr;
    let op = GPtr::<[u8; 2]>::from_addr(site as usize).read();
    assert!(
        op == [0xFF, 0x15],
        "orig: {site:#x} is not a `call dword ptr [abs]`"
    );
    let cell = GPtr::<u32>::from_addr(site as usize + 2).read();
    GPtr::<u32>::from_addr(cell as usize).read() as usize
}

/// The function a `call rel32` (`E8 rel32`) at `site` calls right now. sa-hook's determinism mode
/// retargets some of these sites (the `Game::Idle` frame-limiter calls of `GetCurrentTimeInCycles`).
#[cfg(sa_phase_a)]
pub fn rel_call_target(site: u32) -> usize {
    use crate::mem::GPtr;
    let op = GPtr::<u8>::from_addr(site as usize).read();
    assert!(op == 0xE8, "orig: {site:#x} is not a `call rel32`");
    let rel = GPtr::<u32>::from_addr(site as usize + 1).read();
    site.wrapping_add(5).wrapping_add(rel) as usize
}

/// The function pointer stored at `cell` right now (an IAT slot read by `call/jmp dword ptr [cell]`,
/// or a global function pointer such as `RwEngineInstance`-style tables).
#[cfg(sa_phase_a)]
pub fn fn_at_cell(cell: u32) -> usize {
    crate::mem::GPtr::<u32>::from_addr(cell as usize).read() as usize
}

/// Slot `off` (bytes) of the vtable of the object at `obj` (a C++ virtual call or a COM method:
/// `mov eax, [obj]; call [eax+off]`).
#[cfg(sa_phase_a)]
pub fn vtable_slot(obj: usize, off: u32) -> usize {
    let vt = crate::mem::GPtr::<u32>::from_addr(obj).read();
    crate::mem::GPtr::<u32>::from_addr(vt as usize + off as usize).read() as usize
}

/// Phases B/C: what a `call dword ptr [cell]` at `site` calls, as the registry's site resolver
/// reports it (sa-game: the import trap address sa-game wrote into the IAT cell), 0 without one.
#[cfg(not(sa_phase_a))]
pub fn import_at_call_site(site: u32) -> usize {
    registry::resolve_site(site)
}

/// Phases B/C: the function pointer stored at `cell` in the legacy address arena (sa-game maps the
/// original's data sections there), 0 when nothing is mapped there (e.g. host tests).
#[cfg(not(sa_phase_a))]
pub fn fn_at_cell(cell: u32) -> usize {
    if !registry::arena_contains(cell as usize, 4) {
        return 0;
    }
    crate::mem::GPtr::<u32>::from_addr(cell as usize).read() as usize
}

/// Phases B/C: slot `off` of the vtable of the object at `obj` (objects made by ported code carry
/// the original vtable addresses, whose slots are read from the arena; sa-game's native COM objects
/// carry vtables of trap addresses); 0 for a null object or vtable, or with no registry installed.
#[cfg(not(sa_phase_a))]
pub fn vtable_slot(obj: usize, off: u32) -> usize {
    if obj == 0 || !registry::is_installed() {
        return 0;
    }
    let vt = crate::mem::GPtr::<u32>::from_addr(obj).read() as usize;
    if vt == 0 {
        return 0;
    }
    crate::mem::GPtr::<u32>::from_addr(vt + off as usize).read() as usize
}

/// Phases B/C: a `#[hook(fallback)]`'s original body does not exist. 0 makes `<fn>_original` fail
/// loudly (the registry never maps 0, and must not map the hook's own address back to the port).
#[cfg(not(sa_phase_a))]
#[doc(hidden)]
#[inline]
pub fn fallback_entry(_cell: &AtomicU32, _name: &str) -> usize {
    0
}

/// Phases B/C: the address-keyed registry of native implementations (decision 3 of
/// docs/arch/platform.md §8).
///
/// Every `#[hook]` port has a native entry with the original calling convention (the macro's
/// phase B shim, an `-unwind` ABI so that a panic inside reaches the caller's `catch_unwind`).
/// sa-game installs the table of all of them (`sa_ports::ALL`) plus native replacements of
/// excluded CRT functions, keyed by original address. An `orig!` declaration `=> unported` then
/// resolves its callee address at call time (a literal address, a vtable slot read from the arena,
/// a function pointer cell) and, when the registry has that address, calls the native entry with
/// the declaration's signature, exactly as phase A calls the original address. Calls to addresses
/// without an entry panic with the declaration's name and the address (the live MVP gap list).
///
/// Dispatch exists only on 32-bit x86, where the entries have the original's ABI.
#[cfg(not(sa_phase_a))]
pub mod registry {
    use core::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering};

    /// One native implementation of an original function.
    pub struct Entry {
        /// Original address (for a `.HOODLUM` stub, the stub address).
        pub addr: u32,
        /// Native entry with the original calling convention (`-unwind` ABI).
        pub entry: usize,
        /// Catalogue name of the port (or of the replaced CRT function).
        pub name: &'static str,
        /// Calls dispatched through the registry.
        pub calls: AtomicU32,
    }

    impl Entry {
        pub const fn new(addr: u32, entry: usize, name: &'static str) -> Entry {
            Entry {
                addr,
                entry,
                name,
                calls: AtomicU32::new(0),
            }
        }
    }

    static TABLE: AtomicPtr<Entry> = AtomicPtr::new(core::ptr::null_mut());
    static LEN: AtomicUsize = AtomicUsize::new(0);
    static ARENA_LO: AtomicUsize = AtomicUsize::new(0);
    static ARENA_HI: AtomicUsize = AtomicUsize::new(0);
    static MISSES: AtomicU32 = AtomicU32::new(0);

    /// Installs the table (sorted by address, one entry per address), replacing an earlier one.
    pub fn install(t: &'static [Entry]) {
        assert!(
            t.windows(2).all(|w| w[0].addr < w[1].addr),
            "orig::registry: the table must be sorted by address without duplicates"
        );
        LEN.store(0, Ordering::SeqCst);
        TABLE.store(t.as_ptr().cast_mut(), Ordering::SeqCst);
        LEN.store(t.len(), Ordering::SeqCst);
    }

    /// True once a table is installed.
    pub fn is_installed() -> bool {
        LEN.load(Ordering::Acquire) != 0
    }

    /// The installed table.
    pub fn table() -> &'static [Entry] {
        let n = LEN.load(Ordering::Acquire);
        if n == 0 {
            return &[];
        }
        // SAFETY: `install` stores a `&'static [Entry]`'s pointer and length.
        unsafe { core::slice::from_raw_parts(TABLE.load(Ordering::Acquire), n) }
    }

    /// The entry for `addr`, counting the call (or the miss).
    pub fn lookup(addr: usize) -> Option<&'static Entry> {
        let t = table();
        match t.binary_search_by_key(&addr, |e| e.addr as usize) {
            Ok(i) if addr != 0 => {
                t[i].calls.fetch_add(1, Ordering::Relaxed);
                Some(&t[i])
            }
            _ => {
                MISSES.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// The name of the native implementation at `addr`, if any (not counted).
    pub fn name_of(addr: usize) -> Option<&'static str> {
        let t = table();
        t.binary_search_by_key(&addr, |e| e.addr as usize)
            .ok()
            .map(|i| t[i].name)
    }

    /// Calls that found no entry.
    pub fn misses() -> u32 {
        MISSES.load(Ordering::Relaxed)
    }

    /// Records the legacy address arena `[lo, hi)` (sa-game maps the original's data sections
    /// there) so that vtable and cell reads stay inside mapped memory.
    pub fn set_arena(lo: usize, hi: usize) {
        ARENA_LO.store(lo, Ordering::SeqCst);
        ARENA_HI.store(hi, Ordering::SeqCst);
    }

    /// True if `[a, a + n)` lies in the arena.
    pub fn arena_contains(a: usize, n: usize) -> bool {
        let (lo, hi) = (ARENA_LO.load(Ordering::Relaxed), ARENA_HI.load(Ordering::Relaxed));
        a >= lo && a < hi && a.saturating_add(n) <= hi
    }

    static SITE_RESOLVER: AtomicUsize = AtomicUsize::new(0);
    static NAMES: AtomicPtr<(u32, &'static str)> = AtomicPtr::new(core::ptr::null_mut());
    static NAMES_LEN: AtomicUsize = AtomicUsize::new(0);

    /// Sets how an import call site (`call dword ptr [cell]`, `orig!`'s `site` form) is resolved to
    /// the address to look up: sa-game reads the cell from the instruction bytes and returns the
    /// value it put into that IAT cell (a per-import trap address).
    pub fn set_site_resolver(f: fn(u32) -> usize) {
        SITE_RESOLVER.store(f as usize, Ordering::SeqCst);
    }

    /// The address the `call dword ptr [cell]` at `site` reaches (0 without a resolver).
    pub fn resolve_site(site: u32) -> usize {
        match SITE_RESOLVER.load(Ordering::Acquire) {
            0 => 0,
            f => {
                // SAFETY: stored from a `fn(u32) -> usize` by `set_site_resolver`.
                let f = unsafe { core::mem::transmute::<usize, fn(u32) -> usize>(f) };
                f(site)
            }
        }
    }

    /// Names for addresses that have no entry of their own (the import trap addresses: `DLL!name`),
    /// sorted by address; used only in the failure message.
    pub fn install_names(t: &'static [(u32, &'static str)]) {
        assert!(t.windows(2).all(|w| w[0].0 < w[1].0), "orig::registry: names must be sorted");
        NAMES_LEN.store(0, Ordering::SeqCst);
        NAMES.store(t.as_ptr().cast_mut(), Ordering::SeqCst);
        NAMES_LEN.store(t.len(), Ordering::SeqCst);
    }

    /// The name of an address (an entry or an import trap), else `""`.
    pub fn describe(addr: usize) -> &'static str {
        if let Some(n) = name_of(addr) {
            return n;
        }
        let len = NAMES_LEN.load(Ordering::Acquire);
        if len == 0 {
            return "";
        }
        // SAFETY: `install_names` stores a `&'static` slice's pointer and length.
        let t = unsafe { core::slice::from_raw_parts(NAMES.load(Ordering::Acquire), len) };
        t.binary_search_by_key(&addr, |e| e.0 as usize)
            .map_or("", |i| t[i].1)
    }
}

/// Calls an original function that takes two operands on the x87 stack (`st1`, `st0`, loaded with
/// `fld qword`, exact for f32-loaded values) and returns its result in ST0 (stored with `fstp qword`):
/// the CRT's `_CIpow` (0x822130) and friends, which no `extern` ABI describes. Runs under the game's
/// MXCSR like every `orig!` call ([`crate::abi::call_original`]); the x87 control word is the game's.
#[cfg(sa_phase_a)]
pub fn x87_binary(addr: usize, st1: f64, st0: f64) -> f64 {
    let mut out: f64 = 0.0;
    let (a, b, o) = (&raw const st1, &raw const st0, &raw mut out);
    crate::abi::call_original(|| {
        // SAFETY: `addr` is an original CRT intrinsic with the `_CI*` convention (operands on the x87
        // stack, result in ST0, EAX/ECX/EDX clobbered); the operand and output pointers are locals.
        // EDI and EBX are callee-saved, so the output pointer survives the call.
        #[allow(unsafe_code)]
        unsafe {
            core::arch::asm!(
                "fld qword ptr [{a}]",
                "fld qword ptr [{b}]",
                "call edi",
                "fstp qword ptr [ebx]",
                a = in(reg) a,
                b = in(reg) b,
                in("edi") addr,
                in("ebx") o,
                clobber_abi("C"),
            );
        }
    });
    out
}

/// Phases B/C: the CRT intrinsics the ports call this way are served by libm (STD-5, an MVP choice
/// like `pow` in sa-game's `crt.rs`: the exe's SSE2 kernels can differ in the last bit): `_CIpow`
/// (0x822130) = `st1 ^ st0`.
#[cfg(not(sa_phase_a))]
pub fn x87_binary(addr: usize, st1: f64, st0: f64) -> f64 {
    match addr {
        0x822130 => sa_math::libm::pow(st1, st0),
        _ => panic!("orig::x87_binary({addr:#x}): unported"),
    }
}

/// Calls an original function that takes one operand in ST0 and returns its result in ST0: the CRT's
/// unary `_CI*` intrinsics such as `_CIacos` (0x821E70), `_CIasin`, `_CIsqrt`, which no `extern` ABI
/// describes. The operand is loaded with `fld qword` (exact for any value a port holds as an f64, in
/// particular every f32 and every 64-bit-mantissa value that fits a double) and the result is stored
/// with `fstp qword`, so the caller gets ST0 rounded to a double (exact whenever the intrinsic's result
/// carries at most 53 mantissa bits, which holds under the game's 24-bit precision control). Runs under
/// the game's MXCSR like every `orig!` call ([`crate::abi::call_original`]); the x87 control word is
/// the game's. The FPU stack is balanced on return (one push, one pop), checked by the host test.
#[cfg(sa_phase_a)]
pub fn x87_unary(addr: usize, st0: f64) -> f64 {
    // SAFETY: `addr` is an original CRT intrinsic with the `_CI*` convention (operand in ST0, result
    // in ST0, EAX/ECX/EDX clobbered).
    crate::abi::call_original(|| unsafe { raw::call_unary(addr, st0) })
}

/// Phases B/C: libm stands in for the CRT intrinsics (see [`x87_binary`]): `_CIasin` (0x821E70),
/// `_CIacos` (0x822380).
#[cfg(not(sa_phase_a))]
pub fn x87_unary(addr: usize, st0: f64) -> f64 {
    match addr {
        0x821E70 => sa_math::libm::asin(st0),
        0x822380 => sa_math::libm::acos(st0),
        _ => panic!("orig::x87_unary({addr:#x}): unported"),
    }
}

/// The calling sequence behind [`x87_unary`], separate so that a host test (x86_64) can run it
/// against hand-written intrinsics.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[doc(hidden)]
pub mod raw {
    /// `fld qword [&st0]; call f; fstp qword [&out]`.
    ///
    /// # Safety
    /// `f` must be a function that takes its operand in ST0, returns its result in ST0 (leaving the
    /// FPU stack depth unchanged), preserves the callee-saved registers and the stack pointer of the
    /// platform's C ABI, and clobbers at most the C ABI's caller-saved registers.
    pub unsafe fn call_unary(f: usize, st0: f64) -> f64 {
        let mut out: f64 = 0.0;
        let (a, o) = (&raw const st0, &raw mut out);
        // SAFETY: per the contract above; the output pointer is kept on the stack across the call
        // (pushed twice so that x86_64 keeps its 16-byte alignment at the call) and every
        // caller-saved register is declared clobbered.
        #[cfg(target_arch = "x86")]
        unsafe {
            core::arch::asm!(
                "push ecx",
                "push ecx",
                "fld qword ptr [{a}]",
                "call {f}",
                "pop ecx",
                "pop ecx",
                "fstp qword ptr [ecx]",
                a = in(reg) a,
                f = in(reg) f,
                inout("ecx") o => _,
                clobber_abi("C"),
            );
        }
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!(
                "push rcx",
                "push rcx",
                "fld qword ptr [{a}]",
                "call {f}",
                "pop rcx",
                "pop rcx",
                "fstp qword ptr [rcx]",
                a = in(reg) a,
                f = in(reg) f,
                inout("rcx") o => _,
                clobber_abi("C"),
            );
        }
        out
    }
}

/// Declares typed calls into possibly-original functions (see the module docs).
#[macro_export]
macro_rules! orig {
    () => {};
    // `reg("spec") ADDR` (HARNESS-5, [`crate::regconv`]): a register convention.
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($($a:ident : $t:ty),* $(,)?) $(-> $r:ty)? = reg($spec:literal) $addr:literal => unported;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
        $vis fn $name($($a: $t),*) $(-> $r)? {
            $crate::__orig_body!(unported $name, reg($spec), ($addr as usize), ($($a: $t),*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($($a:ident : $t:ty),* $(,)?) $(-> $r:ty)? = reg($spec:literal) $addr:literal => $imp:path;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
        $vis fn $name($($a: $t),*) $(-> $r)? {
            $crate::__orig_body!(ported $imp, reg($spec), ($addr as usize), ($($a: $t),*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
    // `dyn`: the first argument is the address to call (a pointer computed at run time, e.g. a
    // `GetProcAddress` result); the remaining arguments are passed.
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($f0:ident : u32 $(, $a:ident : $t:ty)* $(,)?) $(-> $r:ty)? = $conv:ident dyn => unported;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
        $vis fn $name($f0: u32 $(, $a: $t)*) $(-> $r)? {
            let _ = $f0;
            $crate::__orig_body!(unported $name, $conv, ($f0 as usize), ($($a: $t),*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
    // `table CELL OFF`: slot OFF (bytes) of the function table a global pointer at CELL points to
    // (e.g. `RwEngineInstance` 0xC97B24: `mov eax, [0xC97B24]; call [eax+OFF]`).
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($($a:ident : $t:ty),* $(,)?) $(-> $r:ty)? = $conv:ident table $cell:literal $off:literal => unported;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
        $vis fn $name($($a: $t),*) $(-> $r)? {
            $crate::__orig_body!(unported $name, $conv, ($crate::orig::vtable_slot($cell as usize, $off)), ($($a: $t),*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($($a:ident : $t:ty),* $(,)?) $(-> $r:ty)? = $conv:ident table $cell:literal $off:literal => $imp:path;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
        $vis fn $name($($a: $t),*) $(-> $r)? {
            $crate::__orig_body!(ported $imp, $conv, ($crate::orig::vtable_slot($cell as usize, $off)), ($($a: $t),*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($($a:ident : $t:ty),* $(,)?) $(-> $r:ty)? = $conv:ident cell $cell:literal => unported;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
        $vis fn $name($($a: $t),*) $(-> $r)? {
            $crate::__orig_body!(unported $name, $conv, ($crate::orig::fn_at_cell($cell)), ($($a: $t),*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($($a:ident : $t:ty),* $(,)?) $(-> $r:ty)? = $conv:ident cell $cell:literal => $imp:path;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
        $vis fn $name($($a: $t),*) $(-> $r)? {
            $crate::__orig_body!(ported $imp, $conv, ($crate::orig::fn_at_cell($cell)), ($($a: $t),*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($a0:ident : $t0:ty $(, $a:ident : $t:ty)* $(,)?) $(-> $r:ty)? = $conv:ident vslot $off:literal => unported;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
        $vis fn $name($a0: $t0 $(, $a: $t)*) $(-> $r)? {
            $crate::__orig_body!(unported $name, $conv, ($crate::orig::vtable_slot($a0.addr(), $off)), ($a0: $t0 $(, $a: $t)*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($a0:ident : $t0:ty $(, $a:ident : $t:ty)* $(,)?) $(-> $r:ty)? = $conv:ident vslot $off:literal => $imp:path;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
        $vis fn $name($a0: $t0 $(, $a: $t)*) $(-> $r)? {
            $crate::__orig_body!(ported $imp, $conv, ($crate::orig::vtable_slot($a0.addr(), $off)), ($a0: $t0 $(, $a: $t)*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($($a:ident : $t:ty),* $(,)?) $(-> $r:ty)? = $conv:ident site $site:literal => unported;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
        $vis fn $name($($a: $t),*) $(-> $r)? {
            $crate::__orig_body!(unported $name, $conv, ($crate::orig::import_at_call_site($site)), ($($a: $t),*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($($a:ident : $t:ty),* $(,)?) $(-> $r:ty)? = $conv:ident site $site:literal => $imp:path;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
        $vis fn $name($($a: $t),*) $(-> $r)? {
            $crate::__orig_body!(ported $imp, $conv, ($crate::orig::import_at_call_site($site)), ($($a: $t),*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($($a:ident : $t:ty),* $(,)?) $(-> $r:ty)? = $conv:ident callsite $site:literal => unported;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
        $vis fn $name($($a: $t),*) $(-> $r)? {
            $crate::__orig_body!(unported $name, $conv, ($crate::orig::rel_call_target($site)), ($($a: $t),*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($($a:ident : $t:ty),* $(,)?) $(-> $r:ty)? = $conv:ident callsite $site:literal => $imp:path;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
        $vis fn $name($($a: $t),*) $(-> $r)? {
            $crate::__orig_body!(ported $imp, $conv, ($crate::orig::rel_call_target($site)), ($($a: $t),*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($($a:ident : $t:ty),* $(,)?) $(-> $r:ty)? = $conv:ident $addr:literal => unported;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
        $vis fn $name($($a: $t),*) $(-> $r)? {
            $crate::__orig_body!(unported $name, $conv, ($addr as usize), ($($a: $t),*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($($a:ident : $t:ty),* $(,)?) $(-> $r:ty)? = $conv:ident $addr:literal => $imp:path;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
        $vis fn $name($($a: $t),*) $(-> $r)? {
            $crate::__orig_body!(ported $imp, $conv, ($addr as usize), ($($a: $t),*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
}

/// Phase A: an `orig!` return type must be what the original returns in registers (at most 8 bytes:
/// EAX, EDX:EAX or ST0). A larger Rust type (e.g. `sa_math::x87::X87`, 16 bytes) would be returned
/// through a hidden pointer and read garbage: declare `f64` for an ST0 result and convert.
#[cfg(sa_phase_a)]
#[doc(hidden)]
#[macro_export]
macro_rules! __orig_ret_check {
    ($r:ty) => {
        const {
            assert!(
                ::core::mem::size_of::<$r>() <= 8,
                "orig! return type wider than a register pair (declare f64 for an ST0 result)"
            )
        }
    };
}

/// Phase A: call the original address with the original convention.
#[cfg(sa_phase_a)]
#[doc(hidden)]
#[macro_export]
macro_rules! __orig_body {
    // register convention: the arguments go into a `RegFrame` per the spec, `call_regs` loads them
    ($kind:ident $what:path, reg($spec:literal), ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        const SPEC: $crate::regconv::RegConv = $crate::regconv::RegConv::parse($spec);
        const {
            assert!(SPEC.input_regs() & (1 << 6) == 0, "orig! reg conv: no argument in EBP");
            let mut _i = 0usize;
            $( $crate::regconv::check_arg(&SPEC, _i, ::core::mem::size_of::<$t>()); _i += 1; )*
            $( $crate::regconv::check_ret(&SPEC, ::core::mem::size_of::<$r>()); )?
        }
        let mut fr = $crate::regconv::RegFrame::new(($addr) as u32);
        let mut _i = 0usize;
        $( fr.arg(&SPEC, _i, $a); _i += 1; )*
        fr.want_st0 = <($($r)?) as $crate::regconv::FromRegFrame>::ST0 as u8;
        // SAFETY: the declaration states the original's register convention.
        #[allow(unsafe_code)]
        $crate::abi::call_original(|| unsafe { $crate::regconv::call_regs(&mut fr) });
        <($($r)?) as $crate::regconv::FromRegFrame>::from_frame(&fr)
    }};
    ($kind:ident $what:path, cdecl, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        $($crate::__orig_ret_check!($r);)?
        #[allow(unsafe_code)]
        let f = unsafe {
            ::core::mem::transmute::<usize, unsafe extern "C" fn($($t),*) $(-> $r)?>($addr)
        };
        #[allow(unsafe_code)]
        $crate::abi::call_original(|| unsafe { f($($a),*) })
    }};
    ($kind:ident $what:path, stdcall, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        $($crate::__orig_ret_check!($r);)?
        #[allow(unsafe_code)]
        let f = unsafe {
            ::core::mem::transmute::<usize, unsafe extern "stdcall" fn($($t),*) $(-> $r)?>($addr)
        };
        #[allow(unsafe_code)]
        $crate::abi::call_original(|| unsafe { f($($a),*) })
    }};
    ($kind:ident $what:path, thiscall, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        $($crate::__orig_ret_check!($r);)?
        #[allow(unsafe_code)]
        let f = unsafe {
            ::core::mem::transmute::<usize, unsafe extern "thiscall" fn($($t),*) $(-> $r)?>($addr)
        };
        #[allow(unsafe_code)]
        $crate::abi::call_original(|| unsafe { f($($a),*) })
    }};
    ($kind:ident $what:path, fastcall, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        $($crate::__orig_ret_check!($r);)?
        #[allow(unsafe_code)]
        let f = unsafe {
            ::core::mem::transmute::<usize, unsafe extern "fastcall" fn($($t),*) $(-> $r)?>($addr)
        };
        #[allow(unsafe_code)]
        $crate::abi::call_original(|| unsafe { f($($a),*) })
    }};
}

/// Host builds (not phase A): stand-ins for functions declared `=> unported`, for the L1 harness.
///
/// A port whose hooked entry calls an unported function (an allocator, an import such as
/// `MapVirtualKeyA`, an original callee) cannot run on the host. The L1 specs run those entries
/// anyway: they register a stand-in that does what the spec's patched original does (return the
/// spec's value, record the arguments), keyed by the declaration's path
/// (`module_path!()::name`, e.g. `"sa_core::pools::call::operator_new_array"`) and checked against
/// the declared signature. Without a stand-in, the call panics as before. Phase A never consults
/// this table (the call goes to the original address).
#[cfg(not(sa_phase_a))]
pub mod host {
    use core::any::TypeId;
    use core::sync::atomic::{AtomicBool, Ordering};

    const SLOTS: usize = 512;

    #[derive(Clone, Copy)]
    struct Slot {
        key: &'static str,
        ty: TypeId,
        f: usize,
    }

    static LOCK: AtomicBool = AtomicBool::new(false);
    static mut TABLE: [Option<Slot>; SLOTS] = [None; SLOTS];

    fn with<R>(f: impl FnOnce(&mut [Option<Slot>; SLOTS]) -> R) -> R {
        while LOCK
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        // SAFETY: the table is only accessed with LOCK held.
        let r = f(unsafe { &mut *core::ptr::addr_of_mut!(TABLE) });
        LOCK.store(false, Ordering::Release);
        r
    }

    /// Registers `f` (a function pointer of the declared signature `fn(args) -> ret`) for the
    /// unported declaration `key`, replacing an earlier one.
    pub fn set<F: Copy + 'static>(key: &'static str, f: F) {
        assert_eq!(
            core::mem::size_of::<F>(),
            core::mem::size_of::<usize>(),
            "orig::host::set: not a function pointer"
        );
        // SAFETY: F is pointer-sized and Copy; it is turned back into the same type in `get`.
        let f = unsafe { core::mem::transmute_copy::<F, usize>(&f) };
        let slot = Slot {
            key,
            ty: TypeId::of::<F>(),
            f,
        };
        with(|t| {
            let i = t
                .iter()
                .position(|s| s.is_some_and(|s| s.key == key))
                .or_else(|| t.iter().position(Option::is_none))
                .expect("orig::host: table full");
            t[i] = Some(slot);
        });
    }

    /// Removes the stand-in for `key`.
    pub fn clear(key: &'static str) {
        with(|t| {
            for s in t.iter_mut() {
                if s.is_some_and(|x| x.key == key) {
                    *s = None;
                }
            }
        });
    }

    /// The stand-in for `key`, if one of exactly type `F` is registered.
    pub fn get<F: Copy + 'static>(key: &'static str) -> Option<F> {
        let s = with(|t| t.iter().flatten().find(|s| s.key == key).copied())?;
        assert!(
            s.ty == TypeId::of::<F>(),
            "orig::host: the stand-in for {key} has another signature"
        );
        // SAFETY: registered by `set` from a value of type F (same TypeId).
        Some(unsafe { core::mem::transmute_copy::<usize, F>(&s.f) })
    }
}

/// Phases B/C: call the Rust implementation, or fail loudly for an unported function (unless a host
/// stand-in is registered, see [`host`]).
#[cfg(not(sa_phase_a))]
#[doc(hidden)]
#[macro_export]
macro_rules! __orig_body {
    (ported $imp:path, reg($spec:literal), ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        const _: $crate::regconv::RegConv = $crate::regconv::RegConv::parse($spec);
        $imp($($a),*)
    }};
    (unported $name:path, reg($spec:literal), ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        const _: $crate::regconv::RegConv = $crate::regconv::RegConv::parse($spec);
        $crate::__orig_body!(unported $name, cdecl, ($addr), ($($a: $t),*) $(-> $r)?)
    }};
    (ported $imp:path, $conv:ident, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {
        $imp($($a),*)
    };
    (unported $name:path, $conv:ident, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        if let Some(f) = $crate::orig::host::get::<fn($($t),*) $(-> $r)?>(concat!(
            module_path!(),
            "::",
            stringify!($name)
        )) {
            return f($($a),*);
        }
        let __sa_callee: usize = $addr;
        $crate::__orig_dispatch!($conv, __sa_callee, ($($a: $t),*) $(-> $r)?);
        $(let _ = $a;)*
        ::core::panic!(
            "{} is not ported; phases B/C have no original code to call ({:#x}{}{})",
            stringify!($name),
            __sa_callee,
            if $crate::orig::registry::describe(__sa_callee).is_empty() { "" } else { " = " },
            $crate::orig::registry::describe(__sa_callee)
        )
    }};
}

/// Phase B on 32-bit x86: call the native implementation registered for the callee address
/// ([`registry`]) with the declaration's signature and the original convention (`-unwind`).
#[cfg(all(not(sa_phase_a), target_arch = "x86"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __orig_dispatch {
    ($conv:ident, $callee:ident, ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {
        if let Some(e) = $crate::orig::registry::lookup($callee) {
            $crate::__orig_dispatch_call!($conv, e.entry, ($($a: $t),*) $(-> $r)?);
        }
    };
}

#[cfg(all(not(sa_phase_a), target_arch = "x86"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __orig_dispatch_call {
    (cdecl, $e:expr, ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        #[allow(unsafe_code)]
        let f = unsafe { ::core::mem::transmute::<usize, extern "C-unwind" fn($($t),*) $(-> $r)?>($e) };
        return f($($a),*);
    }};
    (stdcall, $e:expr, ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        #[allow(unsafe_code)]
        let f = unsafe { ::core::mem::transmute::<usize, extern "stdcall-unwind" fn($($t),*) $(-> $r)?>($e) };
        return f($($a),*);
    }};
    (thiscall, $e:expr, ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        #[allow(unsafe_code)]
        let f = unsafe { ::core::mem::transmute::<usize, extern "thiscall-unwind" fn($($t),*) $(-> $r)?>($e) };
        return f($($a),*);
    }};
    (fastcall, $e:expr, ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        #[allow(unsafe_code)]
        let f = unsafe { ::core::mem::transmute::<usize, extern "fastcall-unwind" fn($($t),*) $(-> $r)?>($e) };
        return f($($a),*);
    }};
}

/// Other phase B/C targets: no native entries with the original ABI.
#[cfg(all(not(sa_phase_a), not(target_arch = "x86")))]
#[doc(hidden)]
#[macro_export]
macro_rules! __orig_dispatch {
    ($conv:ident, $callee:ident, ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {};
}

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    fn twice(x: i32) -> i32 {
        2 * x
    }

    crate::orig! {
        fn doubled(x: i32) -> i32 = cdecl 0x401000 => twice;
        fn missing(x: i32) = thiscall 0x401010 => unported;
        fn through_cell(x: i32) -> i32 = stdcall cell 0x858000 => twice;
        fn virtual_call(obj: crate::mem::GPtr<u32>, x: i32) -> i32 = thiscall vslot 0x0C => twice_obj;
        fn virtual_missing(obj: crate::mem::GPtr<u32>) = stdcall vslot 0x08 => unported;
        fn stand_in(x: i32) -> i32 = cdecl 0x401020 => unported;
    }

    fn twice_obj(_obj: crate::mem::GPtr<u32>, x: i32) -> i32 {
        2 * x
    }

    #[test]
    fn cell_and_vslot_forms() {
        assert_eq!(through_cell(4), 8);
        assert_eq!(virtual_call(crate::mem::GPtr::NULL, 5), 10);
        assert!(std::panic::catch_unwind(|| virtual_missing(crate::mem::GPtr::NULL)).is_err());
    }

    #[test]
    fn owned_builds_call_the_rust_implementation() {
        assert_eq!(doubled(21), 42);
        let r = std::panic::catch_unwind(|| missing(1));
        assert!(r.is_err());
    }

    // Phase B on 32-bit x86: an unported declaration whose callee address has a registry entry calls
    // it with the original convention; a panic inside unwinds to the caller; other addresses fail
    // loudly with the address.
    #[cfg(target_arch = "x86")]
    #[test]
    fn address_registry_dispatch() {
        extern "thiscall-unwind" fn add_this(this: u32, x: i32) -> i32 {
            this as i32 + x
        }
        extern "C-unwind" fn boom(_x: i32) -> i32 {
            panic!("inner")
        }
        crate::orig! {
            fn reg_this(this: u32, x: i32) -> i32 = thiscall 0x401040 => unported;
            fn reg_boom(x: i32) -> i32 = cdecl 0x401050 => unported;
            fn reg_none(x: i32) -> i32 = cdecl 0x401060 => unported;
        }
        let t: &'static [registry::Entry] = std::boxed::Box::leak(
            std::vec![
                registry::Entry::new(0x401040, add_this as *const () as usize, "add_this"),
                registry::Entry::new(0x401050, boom as *const () as usize, "boom"),
            ]
            .into_boxed_slice(),
        );
        registry::install(t);
        assert_eq!(reg_this(40, 2), 42);
        assert!(std::panic::catch_unwind(|| reg_boom(1)).is_err());
        let e = std::panic::catch_unwind(|| reg_none(1)).unwrap_err();
        let m = e.downcast_ref::<std::string::String>().unwrap();
        assert!(m.contains("reg_none is not ported") && m.contains("(0x401060)"), "{m}");
        assert_eq!(registry::name_of(0x401040), Some("add_this"));
    }

    #[test]
    fn host_stand_ins() {
        fn thrice(x: i32) -> i32 {
            3 * x
        }
        host::set::<fn(i32) -> i32>("sa_core::orig::tests::stand_in", thrice);
        assert_eq!(stand_in(2), 6);
        host::clear("sa_core::orig::tests::stand_in");
        assert!(std::panic::catch_unwind(|| stand_in(1)).is_err());
    }

    // Hand-written `_CI*`-convention intrinsics for the x87_unary calling sequence: one negates ST0
    // and uses stack scratch like the CRT's `_CIacos` (`sub esp, 0xC; fst qword [esp]`), one takes the
    // square root, one replaces ST0 by a constant through a push/pop pair.
    #[cfg(target_arch = "x86_64")]
    core::arch::global_asm!(
        ".globl sa_core_test_ci_neg",
        "sa_core_test_ci_neg:",
        "sub rsp, 0x18",
        "fst qword ptr [rsp]",
        "fchs",
        "add rsp, 0x18",
        "ret",
        ".globl sa_core_test_ci_sqrt",
        "sa_core_test_ci_sqrt:",
        "fsqrt",
        "ret",
        ".globl sa_core_test_ci_one",
        "sa_core_test_ci_one:",
        "fld1",
        "fstp st(1)",
        "ret",
    );

    #[cfg(target_arch = "x86_64")]
    unsafe extern "C" {
        fn sa_core_test_ci_neg();
        fn sa_core_test_ci_sqrt();
        fn sa_core_test_ci_one();
    }

    /// The x87 status word's TOP field (stack depth marker).
    #[cfg(target_arch = "x86_64")]
    fn fpu_top() -> u16 {
        let mut sw: u16 = 0;
        // SAFETY: fnstsw only stores the status word.
        unsafe { core::arch::asm!("fnstsw word ptr [{p}]", p = in(reg) &raw mut sw) };
        (sw >> 11) & 7
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn x87_unary_calling_sequence() {
        let top = fpu_top();
        for &(v, neg, root) in &[
            (2.0f64, -2.0f64, core::f64::consts::SQRT_2),
            (0.25, -0.25, 0.5),
            (f32::MAX as f64, -(f32::MAX as f64), (f32::MAX as f64).sqrt()),
            (1.0e-300, -1.0e-300, 1.0e-150),
        ] {
            // SAFETY: the three intrinsics follow the documented convention.
            unsafe {
                assert_eq!(raw::call_unary(sa_core_test_ci_neg as *const () as usize, v), neg);
                assert_eq!(raw::call_unary(sa_core_test_ci_sqrt as *const () as usize, v), root);
                assert_eq!(raw::call_unary(sa_core_test_ci_one as *const () as usize, v), 1.0);
            }
            assert_eq!(fpu_top(), top, "the FPU stack is balanced after each call");
        }
        // NaN payloads and signed zero pass through the fld/fstp pair unchanged.
        // SAFETY: as above.
        let z = unsafe { raw::call_unary(sa_core_test_ci_neg as *const () as usize, 0.0) };
        assert_eq!(z.to_bits(), (-0.0f64).to_bits());
        let q = f64::from_bits(0x7FF8_0000_0000_1234);
        // SAFETY: as above.
        let r = unsafe { raw::call_unary(sa_core_test_ci_sqrt as *const () as usize, q) };
        assert_eq!(r.to_bits(), 0x7FF8_0000_0000_1234);
        // Host: libm stands in for _CIasin/_CIacos; any other intrinsic address is unported.
        assert_eq!(x87_unary(0x821E70, 0.5), sa_math::libm::asin(0.5));
        assert_eq!(x87_unary(0x822380, 0.5), sa_math::libm::acos(0.5));
        assert!(std::panic::catch_unwind(|| x87_unary(0x401000, 0.5)).is_err());
        assert_eq!(fpu_top(), top);
    }

    #[test]
    fn sites_nest() {
        assert_eq!(current_site(), None);
        at_site(0x407187, || {
            assert_eq!(current_site(), Some(0x407187));
            at_site(0x41BD96, || assert_eq!(current_site(), Some(0x41BD96)));
            assert_eq!(current_site(), Some(0x407187));
        });
        assert_eq!(current_site(), None);
    }
}
