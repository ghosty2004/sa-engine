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

/// What phases B/C do when an `unported` declaration is called and nothing is registered for its address: a
/// [`set_unported_handler`] handler, else a panic with `msg`. Natively the panic unwinds to the main loop, which
/// reports a blocked iteration; wasm32 cannot unwind, so the web build installs a handler that leaves the iteration
/// another way (a JS exception) and reports the same blocked iteration.
#[cold]
#[track_caller]
pub fn unported(msg: core::fmt::Arguments<'_>) -> ! {
    let h = UNPORTED_HANDLER.load(Ordering::Relaxed);
    if h != 0 {
        // SAFETY: only `set_unported_handler` stores a non-zero value, an `UnportedHandler`.
        let f: UnportedHandler = unsafe { core::mem::transmute::<usize, UnportedHandler>(h as usize) };
        f(msg);
    }
    panic!("{msg}")
}

/// A handler for [`unported`] (it must not return).
pub type UnportedHandler = fn(core::fmt::Arguments<'_>) -> !;

static UNPORTED_HANDLER: AtomicU32 = AtomicU32::new(0);

/// Installs `f` as [`unported`]'s handler (32-bit targets only: the handler is kept as a 32-bit word).
#[cfg(target_pointer_width = "32")]
pub fn set_unported_handler(f: UnportedHandler) {
    UNPORTED_HANDLER.store(f as usize as u32, Ordering::Relaxed);
}

/// Called by ports inside the original's busy-wait loops on another thread's progress (e.g.
/// `CAECutsceneTrackManager::PreloadCutsceneTrack` spinning until the audio stream thread loaded the track). A no-op
/// unless the host installed a handler ([`set_busy_wait_handler`]): sa-game's deterministic mode runs the audio
/// threads cooperatively on the main thread and steps them from here.
pub fn busy_wait() {
    let h = BUSY_WAIT_HANDLER.load(Ordering::Relaxed);
    if h != 0 {
        // SAFETY: only `set_busy_wait_handler` stores a non-zero value, a `fn()`.
        let f: fn() = unsafe { core::mem::transmute::<usize, fn()>(h as usize) };
        f();
    }
}

static BUSY_WAIT_HANDLER: AtomicU32 = AtomicU32::new(0);

/// Installs `f` as [`busy_wait`]'s handler (32-bit targets only: the handler is kept as a 32-bit word).
#[cfg(target_pointer_width = "32")]
pub fn set_busy_wait_handler(f: fn()) {
    BUSY_WAIT_HANDLER.store(f as usize as u32, Ordering::Relaxed);
}

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

/// Phases B/C: before / after observation of the calls of registry functions (the mod host's hooks), with the
/// observer's decisions: in the before phase it may change the arguments or skip the call with a result of its
/// choosing, in the after phase change the result. Compiled in only with the feature `observe`; without it
/// [`watched`](observe::watched) is a constant `false`, nothing is ever observed and `orig!` calls exactly what it
/// did. Observed are the calls `orig!` dispatches through the [`registry`] and, while any function is watched, the
/// calls of declarations bound to a port directly (`=> path`, a port's calls into its own crate), which then take
/// the same path for a watched callee.
#[cfg(not(sa_phase_a))]
pub mod observe {
    use core::sync::atomic::{AtomicUsize, Ordering};

    use super::registry::Entry;
    use crate::wabi::{Args, Ret};

    /// When the observer is called.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Phase {
        /// Before the call (the arguments).
        Before,
        /// After the call returned (the arguments and the result).
        After,
    }

    /// An argument or result as its declared type gives it.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Raw {
        /// An integer type (sign-extended).
        Int(i64),
        /// A float type.
        Float(f64),
        /// A pointer type (`GPtr`, raw pointer, reference): the address.
        Address(u32),
        /// Anything else (a struct passed by value).
        Other,
    }

    /// What a declared type is, for reading and writing its words.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Kind {
        Int { bits: u32, signed: bool },
        F32,
        F64,
        X87,
        Address,
        Other,
    }

    fn kind(ty: &str) -> Kind {
        let int = |bits, signed| Kind::Int { bits, signed };
        match ty {
            "f32" => Kind::F32,
            "f64" => Kind::F64,
            "i8" => int(8, true),
            "i16" => int(16, true),
            "i32" | "isize" => int(32, true),
            "i64" => int(64, true),
            "bool" | "u8" => int(8, false),
            "u16" => int(16, false),
            "u32" | "usize" | "char" => int(32, false),
            "u64" => int(64, false),
            _ if ty.starts_with('*') || ty.starts_with('&') || ty.contains("GPtr<") => Kind::Address,
            _ if ty.ends_with("x87::X87") => Kind::X87,
            _ => Kind::Other,
        }
    }

    /// The value of a type of kind `k` from its first two words (`f` the float of a float kind).
    fn read(k: Kind, f: f64, lo: u32, hi: u32) -> Raw {
        let v = u64::from(lo) | u64::from(hi) << 32;
        match k {
            Kind::Int { bits, signed: true } => Raw::Int(((v << (64 - bits)) as i64) >> (64 - bits)),
            Kind::Int { bits, signed: false } => Raw::Int((v & (u64::MAX >> (64 - bits))) as i64),
            Kind::F32 | Kind::F64 | Kind::X87 => Raw::Float(f),
            Kind::Address => Raw::Address(lo),
            Kind::Other => Raw::Other,
        }
    }

    /// The words of `v` as a value of kind `k` (an integer truncated to its width, `bool` to 0 / 1), or `None` when
    /// `v` does not fit the kind.
    fn words(k: Kind, v: Raw) -> Option<[u32; 4]> {
        Some(match (k, v) {
            (Kind::Int { bits: 8, signed: false }, Raw::Int(i)) => [i as u8 as u32, 0, 0, 0],
            (Kind::Int { bits, .. }, Raw::Int(i)) => {
                let m = u64::MAX >> (64 - bits);
                let u = i as u64 & m;
                [u as u32, (u >> 32) as u32, 0, 0]
            }
            (Kind::F32, Raw::Float(f)) => [(f as f32).to_bits(), 0, 0, 0],
            (Kind::F64, Raw::Float(f)) => {
                let b = f.to_bits();
                [b as u32, (b >> 32) as u32, 0, 0]
            }
            (Kind::X87, Raw::Float(f)) => {
                let e = crate::wabi::ext_from_f64(f);
                let w = |i: usize| u32::from_le_bytes([e[i], e[i + 1], *e.get(i + 2).unwrap_or(&0), *e.get(i + 3).unwrap_or(&0)]);
                [w(0), w(4), u32::from(e[8]) | u32::from(e[9]) << 8, 0]
            }
            (Kind::Address, Raw::Address(a)) => [a, 0, 0, 0],
            _ => return None,
        })
    }

    /// One observed call.
    pub struct Observed<'a> {
        /// The registry entry of the callee.
        pub entry: &'static Entry,
        /// Before or after.
        pub phase: Phase,
        args: &'a mut Args,
        params: &'a [(&'static str, usize)],
        /// The declared result type and size.
        ret_ty: (&'static str, usize),
        ret: &'a mut Ret,
        skipped: bool,
        result_set: bool,
    }

    impl Observed<'_> {
        /// The number of declared arguments.
        pub fn arg_count(&self) -> usize {
            self.params.len()
        }

        fn arg_at(&self, i: usize) -> Option<(Kind, usize)> {
            let (ty, _) = *self.params.get(i)?;
            let at: usize = self.params[..i].iter().map(|p| p.1.div_ceil(4)).sum();
            Some((kind(ty), at))
        }

        /// Argument `i`.
        pub fn arg(&self, i: usize) -> Option<Raw> {
            let (k, at) = self.arg_at(i)?;
            let words = self.args.words();
            let w = |n: usize| words.get(at + n).copied().unwrap_or(0);
            let f = match k {
                Kind::F64 => f64::from_bits(u64::from(w(0)) | u64::from(w(1)) << 32),
                _ => f64::from(f32::from_bits(w(0))),
            };
            Some(read(k, f, w(0), w(1)))
        }

        /// The result (after the call, of a function that returns a value that fits [`Ret`]).
        pub fn result(&self) -> Option<Raw> {
            if self.phase != Phase::After || !self.returns() {
                return None;
            }
            let b = &self.ret.bytes;
            let w = |k: usize| u32::from_le_bytes([b[4 * k], b[4 * k + 1], b[4 * k + 2], b[4 * k + 3]]);
            Some(read(kind(self.ret_ty.0), self.ret.st0, w(0), w(1)))
        }

        fn returns(&self) -> bool {
            self.ret_ty.1 > 0 && self.ret_ty.1 <= crate::wabi::RET_BYTES
        }

        /// Replaces argument `i` (before the call); false when there is no such argument or `v` does not fit its
        /// declared type (a struct passed by value cannot be replaced).
        pub fn set_arg(&mut self, i: usize, v: Raw) -> bool {
            let Some((k, at)) = self.arg_at(i) else { return false };
            let Some(w) = words(k, v) else { return false };
            if self.phase != Phase::Before {
                return false;
            }
            let n = self.params[i].1.div_ceil(4).min(4);
            let dst = &mut self.args.words_mut()[at..at + n];
            dst.copy_from_slice(&w[..n]);
            true
        }

        fn put_result(&mut self, v: Option<Raw>) -> bool {
            match v {
                None => !self.returns() && self.ret_ty.1 == 0,
                Some(v) => {
                    let k = kind(self.ret_ty.0);
                    if !self.returns() {
                        return false;
                    }
                    match (k, v) {
                        (Kind::F32 | Kind::F64 | Kind::X87, Raw::Float(f)) => self.ret.st0 = f,
                        _ => {
                            let Some(w) = words(k, v) else { return false };
                            let n = self.ret_ty.1.min(8);
                            for (i, b) in w.iter().flat_map(|w| w.to_le_bytes()).take(n).enumerate() {
                                self.ret.bytes[i] = b;
                            }
                        }
                    }
                    true
                }
            }
        }

        /// Skips the call (before it): the caller gets `result` (`None` for a function without a result). False
        /// (nothing skipped) when `result` does not fit the declared result type.
        pub fn skip(&mut self, result: Option<Raw>) -> bool {
            if self.phase != Phase::Before || !self.put_result(result) {
                return false;
            }
            self.skipped = true;
            true
        }

        /// Replaces the result (after the call); false when it does not fit the declared result type.
        pub fn set_result(&mut self, v: Raw) -> bool {
            if self.phase != Phase::After || !self.put_result(Some(v)) {
                return false;
            }
            self.result_set = true;
            true
        }
    }

    static HANDLER: AtomicUsize = AtomicUsize::new(0);
    /// Entries watched now.
    #[cfg(feature = "observe")]
    static WATCHED: AtomicUsize = AtomicUsize::new(0);

    /// Sets the observer (the mod host). It runs on the game thread inside the observed call.
    pub fn set_handler(f: fn(&mut Observed<'_>)) {
        HANDLER.store(f as usize, Ordering::Release);
    }

    /// Whether calls through `e` are observed.
    #[inline(always)]
    pub fn watched(e: &Entry) -> bool {
        #[cfg(feature = "observe")]
        {
            e.watch.load(Ordering::Relaxed)
        }
        #[cfg(not(feature = "observe"))]
        {
            let _ = e;
            false
        }
    }

    /// The watched entry of `addr`, for a declaration bound to a port directly: one relaxed load while nothing is
    /// watched (and nothing at all without the feature `observe`); the registry is not consulted and its call counts
    /// are not touched.
    #[inline(always)]
    pub fn watched_at(addr: usize) -> Option<&'static Entry> {
        #[cfg(feature = "observe")]
        {
            if WATCHED.load(Ordering::Relaxed) == 0 {
                return None;
            }
            super::registry::find(addr).filter(|e| watched(e))
        }
        #[cfg(not(feature = "observe"))]
        {
            let _ = addr;
            None
        }
    }

    /// Turns observation of calls through `e` on or off (a no-op without the feature `observe`).
    pub fn watch(e: &Entry, on: bool) {
        #[cfg(feature = "observe")]
        if e.watch.swap(on, Ordering::Relaxed) != on {
            if on {
                WATCHED.fetch_add(1, Ordering::Relaxed);
            } else {
                WATCHED.fetch_sub(1, Ordering::Relaxed);
            }
        }
        #[cfg(not(feature = "observe"))]
        let _ = (e, on);
    }

    fn notify(o: &mut Observed<'_>) {
        let h = HANDLER.load(Ordering::Acquire);
        if h != 0 {
            // SAFETY: `HANDLER` holds 0 or a `fn(&mut Observed)` stored by `set_handler`.
            let f = unsafe { core::mem::transmute::<usize, fn(&mut Observed<'_>)>(h) };
            f(o);
        }
    }

    /// Runs `call` (the callee, with the argument words it is given) between the observer's before and after calls.
    /// `args` holds the arguments' words, `params` their declared types and sizes. The observer may change the
    /// words, skip the call (the caller gets the result it chose) or replace the result.
    #[inline(never)]
    pub fn around<R>(e: &'static Entry, args: &mut Args, params: &[(&'static str, usize)], call: impl FnOnce(&Args) -> R) -> R {
        let ret_ty = (core::any::type_name::<R>(), core::mem::size_of::<R>());
        let mut ret = Ret::new();
        let mut o = Observed { entry: e, phase: Phase::Before, args, params, ret_ty, ret: &mut ret, skipped: false, result_set: false };
        notify(&mut o);
        if o.skipped {
            return crate::wabi::get_ret(&ret);
        }
        let r = call(args);
        let mut ret = Ret::new();
        if ret_ty.1 <= crate::wabi::RET_BYTES {
            crate::wabi::put_ret_ref(&r, &mut ret);
        }
        let mut o = Observed { entry: e, phase: Phase::After, args, params, ret_ty, ret: &mut ret, skipped: false, result_set: false };
        notify(&mut o);
        if o.result_set {
            return crate::wabi::get_ret(&ret);
        }
        r
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn params() -> [(&'static str, usize); 5] {
            [
                ("i16", 2),
                ("f32", 4),
                (core::any::type_name::<crate::mem::GPtr<u8>>(), core::mem::size_of::<crate::mem::GPtr<u8>>()),
                ("f64", 8),
                ("u8", 1),
            ]
        }

        fn args() -> Args {
            let mut a = Args::new();
            a.push_ref(&-3i16);
            a.push_ref(&2.5f32);
            a.push_ref(&crate::mem::GPtr::<u8>::from_addr(0x1234));
            a.push_ref(&1.25f64);
            a.push_ref(&200u8);
            a
        }

        static E: Entry = Entry::new(0x401000, 1, "f");

        #[test]
        fn classifies_by_declared_type() {
            let mut a = args();
            let p = params();
            let mut ret = Ret::new();
            let o = Observed { entry: &E, phase: Phase::Before, args: &mut a, params: &p, ret_ty: ("()", 0), ret: &mut ret, skipped: false, result_set: false };
            assert_eq!(o.arg_count(), 5);
            assert_eq!(o.arg(0), Some(Raw::Int(-3)));
            assert_eq!(o.arg(1), Some(Raw::Float(2.5)));
            assert_eq!(o.arg(2), Some(Raw::Address(0x1234)));
            assert_eq!(o.arg(3), Some(Raw::Float(1.25)));
            assert_eq!(o.arg(4), Some(Raw::Int(200)));
            assert_eq!(o.arg(5), None);
        }

        #[test]
        fn decisions_change_arguments_skip_and_results() {
            let p = params();
            let callee = |a: &Args| {
                let mut r = crate::wabi::Reader::over(a);
                let (x, y) = (r.take::<i16>(), r.take::<f32>());
                f32::from(x) * y
            };
            let mut a = args();
            assert_eq!(around(&E, &mut a, &p, callee), -7.5);
            // the observer of this thread's test: doubles argument 0, skips when argument 4 is 7, sets 99 after
            static MODE: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);
            fn observer(o: &mut Observed<'_>) {
                match (MODE.load(Ordering::Relaxed), o.phase) {
                    (1, Phase::Before) => {
                        let Some(Raw::Int(v)) = o.arg(0) else { panic!() };
                        assert!(o.set_arg(0, Raw::Int(2 * v)));
                        assert!(!o.set_arg(1, Raw::Int(1)));
                    }
                    (2, Phase::Before) => assert!(o.skip(Some(Raw::Float(4.0)))),
                    (3, Phase::After) => {
                        assert_eq!(o.result(), Some(Raw::Float(-7.5)));
                        assert!(o.set_result(Raw::Float(99.0)));
                    }
                    _ => {}
                }
            }
            set_handler(observer);
            for (mode, want) in [(1u8, -15.0f32), (2, 4.0), (3, 99.0)] {
                MODE.store(mode, Ordering::Relaxed);
                let mut a = args();
                assert_eq!(around(&E, &mut a, &p, callee), want, "mode {mode}");
            }
            MODE.store(0, Ordering::Relaxed);
        }
    }
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
    use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicUsize, Ordering};

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
        /// Calls through the registry are reported to the observer ([`super::observe`]).
        #[cfg(feature = "observe")]
        pub(super) watch: AtomicBool,
    }

    impl Entry {
        pub const fn new(addr: u32, entry: usize, name: &'static str) -> Entry {
            Entry {
                addr,
                entry,
                name,
                calls: AtomicU32::new(0),
                #[cfg(feature = "observe")]
                watch: AtomicBool::new(false),
            }
        }
    }

    static TABLE: AtomicPtr<Entry> = AtomicPtr::new(core::ptr::null_mut());
    static LEN: AtomicUsize = AtomicUsize::new(0);
    /// The address index of the installed table (see [`Index`]).
    static INDEX: AtomicPtr<Index> = AtomicPtr::new(core::ptr::null_mut());
    static ARENA_LO: AtomicUsize = AtomicUsize::new(0);
    static ARENA_HI: AtomicUsize = AtomicUsize::new(0);
    static MISSES: AtomicU32 = AtomicU32::new(0);
    /// Whether [`lookup`] counts calls and misses (off by default: only the per-frame measurement
    /// and sa-game's run report read the counts, and they turn it on).
    static COUNTING: AtomicBool = AtomicBool::new(false);

    /// An open-addressing hash index over the installed table (PERF-1): the slot `hash(addr)` and
    /// the slots after it hold `table position + 1` (0 = empty). At a load factor of at most 1/2 a
    /// lookup costs one multiply and about one probe instead of a binary search over the whole
    /// table. Built once per [`install`]; an installed table never changes.
    struct Index {
        slots: &'static [u32],
        shift: u32,
    }

    /// Fibonacci hashing of a 32-bit address into `32 - shift` bits.
    #[inline(always)]
    fn hash(addr: u32, shift: u32) -> usize {
        (addr.wrapping_mul(0x9E37_79B1) >> shift) as usize
    }

    impl Index {
        fn build(t: &[Entry]) -> Index {
            // at least twice as many slots as entries, at least 4 (so `shift` stays below 32)
            let bits = (t.len().max(2) * 2).next_power_of_two().trailing_zeros();
            let shift = 32 - bits;
            let mask = (1usize << bits) - 1;
            let mut slots = alloc::vec![0u32; 1 << bits];
            for (i, e) in t.iter().enumerate() {
                let mut h = hash(e.addr, shift);
                while slots[h] != 0 {
                    h = (h + 1) & mask;
                }
                slots[h] = i as u32 + 1;
            }
            Index {
                slots: alloc::boxed::Box::leak(slots.into_boxed_slice()),
                shift,
            }
        }

        /// The table position of `addr`, if it has an entry.
        #[inline(always)]
        fn find(&self, t: &[Entry], addr: u32) -> Option<usize> {
            let mask = self.slots.len() - 1;
            let mut h = hash(addr, self.shift);
            loop {
                match self.slots[h] {
                    0 => return None,
                    s => {
                        let i = s as usize - 1;
                        if t[i].addr == addr {
                            return Some(i);
                        }
                    }
                }
                h = (h + 1) & mask;
            }
        }
    }

    /// Installs the table (sorted by address, one entry per address), replacing an earlier one.
    pub fn install(t: &'static [Entry]) {
        assert!(
            t.windows(2).all(|w| w[0].addr < w[1].addr),
            "orig::registry: the table must be sorted by address without duplicates"
        );
        let index: &'static Index = alloc::boxed::Box::leak(alloc::boxed::Box::new(Index::build(t)));
        LEN.store(0, Ordering::SeqCst);
        TABLE.store(t.as_ptr().cast_mut(), Ordering::SeqCst);
        INDEX.store((index as *const Index).cast_mut(), Ordering::SeqCst);
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

    /// The position of `addr` in `t` (the installed table), if it has an entry.
    #[inline(always)]
    fn position(t: &[Entry], addr: usize) -> Option<usize> {
        if t.is_empty() {
            return None;
        }
        let addr = u32::try_from(addr).ok()?;
        // SAFETY: `install` stores the index (a leaked `Box`, never freed) before the non-zero
        // length that made `t` non-empty, and the index of a table is never replaced while that
        // table is installed.
        let index = unsafe { &*INDEX.load(Ordering::Acquire) };
        index.find(t, addr)
    }

    /// Turns the call and miss counting of [`lookup`] on or off (off by default).
    pub fn set_counting(on: bool) {
        COUNTING.store(on, Ordering::SeqCst);
    }

    /// Whether [`lookup`] counts calls and misses.
    pub fn counting() -> bool {
        COUNTING.load(Ordering::Relaxed)
    }

    /// The entry for `addr`, counting the call (or the miss) when counting is on
    /// ([`set_counting`]).
    #[inline]
    pub fn lookup(addr: usize) -> Option<&'static Entry> {
        let t = table();
        match position(t, addr) {
            Some(i) if addr != 0 => {
                if COUNTING.load(Ordering::Relaxed) {
                    t[i].calls.fetch_add(1, Ordering::Relaxed);
                }
                Some(&t[i])
            }
            _ => {
                if COUNTING.load(Ordering::Relaxed) {
                    MISSES.fetch_add(1, Ordering::Relaxed);
                }
                None
            }
        }
    }

    /// The entry for `addr`, if any (not counted).
    pub fn find(addr: usize) -> Option<&'static Entry> {
        let t = table();
        position(t, addr).filter(|_| addr != 0).map(|i| &t[i])
    }

    /// The name of the native implementation at `addr`, if any (not counted).
    pub fn name_of(addr: usize) -> Option<&'static str> {
        let t = table();
        position(t, addr).map(|i| t[i].name)
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

    #[cfg(test)]
    mod tests {
        use super::{Entry, Index};

        // The index finds exactly the table's addresses (dense runs, clustered and spread ones, hash
        // collisions), at any table size.
        #[test]
        fn index_matches_binary_search() {
            for n in [1usize, 2, 3, 7, 64, 1000, 20_000] {
                let mut addrs: std::vec::Vec<u32> = (0..n as u32)
                    .map(|i| match i % 3 {
                        0 => 0x401000 + i * 0x10,
                        1 => 0x1556000 + i * 4,
                        _ => 0x8000_0000u32.wrapping_add(i.wrapping_mul(0x9E37_79B1)),
                    })
                    .collect();
                addrs.sort_unstable();
                addrs.dedup();
                let t: std::vec::Vec<Entry> = addrs.iter().map(|&a| Entry::new(a, a as usize, "e")).collect();
                let ix = Index::build(&t);
                assert!(ix.slots.len() >= 2 * t.len());
                for (i, e) in t.iter().enumerate() {
                    assert_eq!(ix.find(&t, e.addr), Some(i));
                    for probe in [e.addr.wrapping_add(1), e.addr.wrapping_sub(1), e.addr ^ 0x10_0000] {
                        let want = t.binary_search_by_key(&probe, |e| e.addr).ok();
                        assert_eq!(ix.find(&t, probe), want, "{probe:#x}");
                    }
                }
                assert_eq!(ix.find(&t, 0), t.binary_search_by_key(&0, |e| e.addr).ok());
            }
        }
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
            $crate::__orig_body!(ported_at $imp, $conv, ($addr as usize), ($($a: $t),*) $(-> $r)?)
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

/// The result type of an `orig!` declaration, `()` without one (a bare `($($r)?)` trips
/// `unused_parens` when the macro is expanded inside sa-core).
#[doc(hidden)]
#[macro_export]
macro_rules! __orig_ret_ty {
    () => { () };
    ($r:ty) => { $r };
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
        fr.want_st0 = <$crate::__orig_ret_ty!($($r)?) as $crate::regconv::FromRegFrame>::ST0 as u8;
        // SAFETY: the declaration states the original's register convention.
        #[allow(unsafe_code)]
        $crate::abi::call_original(|| unsafe { $crate::regconv::call_regs(&mut fr) });
        <$crate::__orig_ret_ty!($($r)?) as $crate::regconv::FromRegFrame>::from_frame(&fr)
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
    use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    const SLOTS: usize = 512;

    /// Bits of the key filter (PERF-1): a key whose filter bit is clear has no stand-in, so the
    /// call skips the lock and the table scan. sa-game registers ~80 stand-ins and dispatches
    /// ~32,000 calls per frame through declarations that have none.
    const FILTER_BITS: usize = 4096;
    /// One bit per `key_hash(key) % FILTER_BITS` of every key ever registered (bits are never
    /// cleared, so a cleared key only costs the slow path again).
    static FILTER: [AtomicU32; FILTER_BITS / 32] = [const { AtomicU32::new(0) }; FILTER_BITS / 32];

    /// FNV-1a of a stand-in key (`const`, so the declarations hash their keys at compile time).
    pub const fn key_hash(key: &str) -> u32 {
        let b = key.as_bytes();
        let mut h: u32 = 0x811C_9DC5;
        let mut i = 0;
        while i < b.len() {
            h = (h ^ b[i] as u32).wrapping_mul(0x0100_0193);
            i += 1;
        }
        h
    }

    /// Whether a stand-in may be registered for a key of hash `h` (false: certainly none).
    #[inline(always)]
    pub fn may_have(h: u32) -> bool {
        let bit = h as usize % FILTER_BITS;
        FILTER[bit / 32].load(Ordering::Relaxed) & (1 << (bit % 32)) != 0
    }

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
        let bit = key_hash(key) as usize % FILTER_BITS;
        FILTER[bit / 32].fetch_or(1 << (bit % 32), Ordering::SeqCst);
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
        get_hashed(key, key_hash(key))
    }

    /// [`get`] with the key's [`key_hash`] precomputed (the `orig!` declarations' fast path).
    #[inline(always)]
    pub fn get_hashed<F: Copy + 'static>(key: &'static str, h: u32) -> Option<F> {
        if !may_have(h) {
            return None;
        }
        lookup(key)
    }

    #[inline(never)]
    fn lookup<F: Copy + 'static>(key: &'static str) -> Option<F> {
        let s = with(|t| t.iter().flatten().find(|s| s.key == key).copied())?;
        assert!(
            s.ty == TypeId::of::<F>(),
            "orig::host: the stand-in for {key} has another signature"
        );
        // SAFETY: registered by `set` from a value of type F (same TypeId).
        Some(unsafe { core::mem::transmute_copy::<usize, F>(&s.f) })
    }
}

/// Phases B/C: a call of the watched entry `e` between the observer's before and after calls ([`observe::around`]):
/// the arguments go into words the observer may change, and `$call` runs with the arguments read back from them.
#[cfg(not(sa_phase_a))]
#[doc(hidden)]
#[macro_export]
macro_rules! __orig_observed {
    ($e:expr, ($($a:ident : $t:ty),*), $call:expr) => {{
        #[allow(unused_mut)]
        let mut __sa_args = $crate::wabi::Args::new();
        $(__sa_args.push::<$t>($a);)*
        let __sa_params = [$((::core::any::type_name::<$t>(), ::core::mem::size_of::<$t>())),*];
        $crate::orig::observe::around($e, &mut __sa_args, &__sa_params, |__sa_a| {
            #[allow(unused_mut, unused_variables)]
            let mut __sa_rd = $crate::wabi::Reader::over(__sa_a);
            $(let $a: $t = __sa_rd.take::<$t>();)*
            $call
        })
    }};
}

/// Phases B/C: call the Rust implementation, or fail loudly for an unported function (unless a host
/// stand-in is registered, see [`host`]).
#[cfg(not(sa_phase_a))]
#[doc(hidden)]
#[macro_export]
macro_rules! __orig_body {
    (ported $imp:path, reg($spec:literal), ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        const _: $crate::regconv::RegConv = $crate::regconv::RegConv::parse($spec);
        $crate::__orig_body!(ported_at $imp, cdecl, ($addr), ($($a: $t),*) $(-> $r)?)
    }};
    (unported $name:path, reg($spec:literal), ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        const SPEC: $crate::regconv::RegConv = $crate::regconv::RegConv::parse($spec);
        $crate::__orig_reg_dispatch!(SPEC, ($addr), ($($a: $t),*) $(-> $r)?);
        $crate::__orig_body!(unported $name, cdecl, ($addr), ($($a: $t),*) $(-> $r)?)
    }};
    (ported $imp:path, $conv:ident, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {
        $imp($($a),*)
    };
    (ported_at $imp:path, $conv:ident, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {
        if let Some(e) = $crate::orig::observe::watched_at($addr) {
            $crate::__orig_observed!(e, ($($a: $t),*), $imp($($a),*))
        } else {
            $imp($($a),*)
        }
    };
    (unported $name:path, $conv:ident, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        {
            const __SA_KEY: &str = concat!(module_path!(), "::", stringify!($name));
            const __SA_HASH: u32 = $crate::orig::host::key_hash(__SA_KEY);
            if let Some(f) = $crate::orig::host::get_hashed::<fn($($t),*) $(-> $r)?>(__SA_KEY, __SA_HASH) {
                return f($($a),*);
            }
        }
        let __sa_callee: usize = $addr;
        $crate::__orig_dispatch!($conv, __sa_callee, ($($a: $t),*) $(-> $r)?);
        $(let _ = $a;)*
        $crate::orig::unported(::core::format_args!(
            "{} is not ported; phases B/C have no original code to call ({:#x}{}{})",
            stringify!($name),
            __sa_callee,
            if $crate::orig::registry::describe(__sa_callee).is_empty() { "" } else { " = " },
            $crate::orig::registry::describe(__sa_callee)
        ))
    }};
}

/// Phase B on 32-bit x86, register convention: a registered entry has the original's convention
/// (registers, ST0 for an `X87`/`f32` result), so it is called through a `RegFrame` like the
/// original in phase A. (Dispatching it as `cdecl` returned an `X87` through a hidden pointer the
/// entry does not take: the pause menu's Stats page jumped into the host table.)
#[cfg(all(not(sa_phase_a), target_arch = "x86"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __orig_reg_dispatch {
    ($spec:ident, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {
        if let Some(e) = $crate::orig::registry::lookup(($addr) as usize) {
            let mut fr = $crate::regconv::RegFrame::new(e.entry as u32);
            #[allow(unused_mut, unused_variables)]
            let mut _i = 0usize;
            $( fr.arg(&$spec, _i, $a); _i += 1; )*
            fr.want_st0 = <$crate::__orig_ret_ty!($($r)?) as $crate::regconv::FromRegFrame>::ST0 as u8;
            // SAFETY: the registered entry has the original's register convention (the declaration's spec).
            #[allow(unsafe_code)]
            unsafe { $crate::regconv::call_regs(&mut fr) };
            return <$crate::__orig_ret_ty!($($r)?) as $crate::regconv::FromRegFrame>::from_frame(&fr);
        }
    };
}

/// Other phase B/C targets: the `cdecl` dispatch below (type-erased on wasm32).
#[cfg(all(not(sa_phase_a), not(target_arch = "x86")))]
#[doc(hidden)]
#[macro_export]
macro_rules! __orig_reg_dispatch {
    ($spec:ident, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {
        let _ = &$spec;
    };
}

/// Phase B on 32-bit x86: call the native implementation registered for the callee address
/// ([`registry`]) with the declaration's signature and the original convention (`-unwind`).
#[cfg(all(not(sa_phase_a), target_arch = "x86"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __orig_dispatch {
    ($conv:ident, $callee:ident, ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {
        if let Some(e) = $crate::orig::registry::lookup($callee) {
            if $crate::orig::observe::watched(e) {
                return $crate::__orig_observed!(e, ($($a: $t),*), {
                    $crate::__orig_dispatch_call!($conv, e.entry, ($($a: $t),*) $(-> $r)?);
                });
            }
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

/// Phases B/C on wasm32: every entry is type-erased ([`crate::wabi`]): the arguments go as the
/// original's stack words, the result comes back as EAX:EDX or ST0.
#[cfg(all(not(sa_phase_a), target_arch = "wasm32"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __orig_dispatch {
    ($conv:ident, $callee:ident, ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {
        if let Some(e) = $crate::orig::registry::lookup($callee) {
            #[allow(unused_mut)]
            let mut __sa_args = $crate::wabi::Args::new();
            $(__sa_args.push::<$t>($a);)*
            if $crate::orig::observe::watched(e) {
                let __sa_params = [$((::core::any::type_name::<$t>(), ::core::mem::size_of::<$t>())),*];
                return $crate::orig::observe::around(e, &mut __sa_args, &__sa_params, |__sa_a| {
                    $crate::wabi::call(e.entry, $callee, __sa_a)
                });
            }
            return $crate::wabi::call(e.entry, $callee, &__sa_args);
        }
    };
}

/// Other phase B/C targets: no native entries with the original ABI.
#[cfg(all(not(sa_phase_a), not(target_arch = "x86"), not(target_arch = "wasm32")))]
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
