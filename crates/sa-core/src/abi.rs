//! Calling conventions, hook registration records and the hook shims' runtime support (ADR-001 §4).
#![allow(unsafe_code)]

/// Calling convention of an original function (MSVC 7.1, 32-bit x86).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conv {
    /// Arguments on the stack, caller cleans (`extern "C"`).
    Cdecl,
    /// Arguments on the stack, callee cleans.
    Stdcall,
    /// `this` in ECX, arguments on the stack, callee cleans.
    Thiscall,
    /// First two 32-bit integer arguments in ECX/EDX, rest on the stack, callee cleans.
    Fastcall,
}

/// One ported function, as `#[hook]` registers it. `sa-hook` turns every record of
/// [`crate::hooks::ALL`] into a hook-table entry (patched at attach, toggled Rust/original at frame
/// boundaries).
#[derive(Clone, Copy)]
pub struct HookReg {
    /// Address in gta_sa.exe 1.0 US (for a `.HOODLUM` stub, the stub address).
    pub addr: u32,
    /// Catalogue name (`catalog/functions.csv` `owner::name`), the control channel's key.
    pub name: &'static str,
    pub conv: Conv,
    /// Bytes that must be found at `addr` before patching (at least the 5 replaced by the entry jmp).
    pub expect: &'static [u8],
    /// Address of the shim with the original convention (0 on targets other than 32-bit x86).
    pub entry: fn() -> usize,
    /// Calls that came in through the shim (i.e. from original code while the hook is on Rust).
    pub calls: fn() -> u32,
    /// 32-bit stack argument words (after `this` in ECX for thiscall, ECX/EDX for fastcall).
    pub stack_words: u8,
    /// Where the result is returned.
    pub ret: AbiRet,
    /// For thiscall: the `this` type (catalog/layouts.csv name) and its size, when declared.
    pub this_type: Option<&'static str>,
    pub this_size: u32,
    /// Caller registers the phase-A entry returns unchanged although the convention lets a callee
    /// clobber them ([`PRESERVE_ECX`] | [`PRESERVE_EDX`]): original callers that rely on the original
    /// leaving them alone (whole-program-optimised call sites, `#[hook(preserve = "edx")]`).
    pub preserve: u8,
    /// `#[hook(fallback)]` (ADR-002): the installer passes the original entry (the relocated-prologue
    /// trampoline, or a `.HOODLUM` stub's old target) here before patching, so the port can call the
    /// original body for the paths it has not ported (`<fn>_original`). `None` for whole-function ports.
    pub set_original: Option<fn(u32)>,
    /// Stack argument words (bit k = word k) holding a 1-byte (`u8`/`i8`/`bool`) resp. 2-byte argument:
    /// the original's callers leave the upper bytes of such a slot undefined (MSVC pushes a whole
    /// register), so split-run L2 compares only the argument's own bytes (HARNESS-2).
    pub arg8: u32,
    pub arg16: u32,
    /// `#[hook(unverified = "BATCH")]` (docs/PLAN.md "Writer/verifier split"): the verification batch of
    /// a port that has not passed L2/L3 yet, `None` once verified. Phase A installs such a port only
    /// when it is selected (`SA_UNVERIFIED`, or named in `SA_HOOK_MODE` / `SA_L2` / `SA_HOOK_PORTS`), so
    /// unverified code never changes the default behaviour. Phases B/C use it like any other port.
    pub batch: Option<&'static str>,
}

/// EAX as the caller left it at the entry of the last `#[hook(entry_eax)]` port (PORT-ANIM-INPUT).
///
/// Some originals return a 16-bit value with `xor ax, ax` / `mov ax, ...` on a path where the upper
/// half of EAX is still the caller's EAX (e.g. `CPad::GetSteeringLeftRight` when the controls are
/// disabled). L2 compares the whole EAX, so such a port reads this value first thing and returns
/// `(entry_eax() & 0xFFFF_0000) | ax`. Phase A: the naked entry wrapper stores EAX here before
/// jumping to the shim (single game thread, ADR-001 §3 rule 3). Elsewhere it is whatever
/// [`set_entry_eax`] stored (L1 specs set the value their harness passes: the call target address).
pub static ENTRY_EAX: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

/// One value of a [`stack_footprint`] write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fp {
    /// a value the port computed
    Val(u32),
    /// the caller's ESI / EDI / EBX / EBP (callee-saved: the values the original pushed)
    Esi,
    Edi,
    Ebx,
    Ebp,
    /// the address `entry ESP + offset` (e.g. a pushed pointer to one of the original's locals)
    Addr(i32),
}

/// Most writes one call records (see [`stack_footprint`]).
pub const FOOTPRINT_MAX: usize = 16;
/// Largest snapshot window below the entry ESP, bytes (see [`stack_footprint`]).
pub const FOOTPRINT_WINDOW_MAX: usize = 0x1000;

/// State shared with the naked `footprint` entry wrapper of `#[hook]` (field offsets are part of
/// the wrapper's code: count 0, active 4, scratch 8 (EAX, EDX, ECX, ESI, EDI, spare), entries 32 as
/// [offset, kind, value] triples (kinds 0 value, 1 esi, 2 edi, 3 ebx, 4 ebp, 5 entry ESP + value),
/// snapshot 224).
#[repr(C)]
#[doc(hidden)]
pub struct FootprintState {
    pub count: u32,
    pub active: u32,
    pub scratch: [u32; 6],
    pub entries: [[u32; 3]; FOOTPRINT_MAX],
    pub snapshot: [u32; FOOTPRINT_WINDOW_MAX / 4],
}

#[doc(hidden)]
pub struct FootprintCell(pub core::cell::UnsafeCell<FootprintState>);
// SAFETY: written only by the game's main thread (the hooked functions run there, ADR-001 §3 rule 3)
// and by the naked wrapper around those calls.
unsafe impl Sync for FootprintCell {}

const _: () = assert!(core::mem::offset_of!(FootprintState, entries) == 32);
const _: () = assert!(core::mem::offset_of!(FootprintState, snapshot) == 224);

/// See [`stack_footprint`].
#[doc(hidden)]
pub static FOOTPRINT: FootprintCell = FootprintCell(core::cell::UnsafeCell::new(FootprintState {
    count: 0,
    active: 0,
    scratch: [0; 6],
    entries: [[0; 3]; FOOTPRINT_MAX],
    snapshot: [0; FOOTPRINT_WINDOW_MAX / 4],
}));

/// The stack residue the original leaves (ADR-001 §5b): original code that runs later may read
/// uninitialised stack slots, and so depend on what an earlier callee left below its entry ESP.
/// A port registered with `#[hook(..., footprint = W)]` runs behind a naked wrapper that snapshots
/// the `W` bytes below the entry ESP (`W` must cover the whole depth the Rust shim and its callees
/// use, so none of their frames survive), calls the shim, puts the snapshot back (the original
/// leaves the slots it never writes as they were), then applies the writes the port recorded here, in
/// order: `(offset from the entry ESP, value)` (negative = below the return address; 4, 8, ... =
/// the argument slots). The port lists every slot the original writes on the path it took (derived
/// from the disassembly). Recorded only while that wrapper is active (a direct Rust call or a host
/// test records nothing); not reentrant (a footprint hook must not call another one). Phases B/C:
/// no wrapper, no effect.
pub fn stack_footprint(writes: &[(i32, Fp)]) {
    // SAFETY: main thread only (see FootprintCell); the wrapper is not running between its call
    // and return of the shim, which is where this executes.
    #[allow(unsafe_code)]
    let st = unsafe { &mut *FOOTPRINT.0.get() };
    if st.active == 0 {
        return;
    }
    for &(off, v) in writes {
        let n = st.count as usize;
        assert!(n < FOOTPRINT_MAX, "stack_footprint: more than {FOOTPRINT_MAX} writes");
        let (kind, val) = match v {
            Fp::Val(x) => (0, x),
            Fp::Esi => (1, 0),
            Fp::Edi => (2, 0),
            Fp::Ebx => (3, 0),
            Fp::Ebp => (4, 0),
            Fp::Addr(o) => (5, o as u32),
        };
        st.entries[n] = [off as u32, kind, val];
        st.count += 1;
    }
}

/// The byte the original would find at `entry ESP + off` (`-w <= off < 0`) in a `#[hook(..., footprint = w)]`
/// port: the snapshot the wrapper took before the shim ran (an uninitialised local of the original reads
/// the residue of earlier calls there). 0 when no footprint wrapper is active (direct Rust calls, host
/// tests, phases B/C) or `off` is outside the window.
pub fn footprint_residue_u8(off: i32, w: u32) -> u8 {
    // SAFETY: main thread only (see FootprintCell); read while the wrapper is inside the shim.
    #[allow(unsafe_code)]
    let st = unsafe { &*FOOTPRINT.0.get() };
    let i = w as i64 + i64::from(off);
    if st.active == 0 || off >= 0 || i < 0 || w as usize > FOOTPRINT_WINDOW_MAX {
        return 0;
    }
    let i = i as usize;
    (st.snapshot[i / 4] >> (8 * (i % 4))) as u8
}

/// See [`ENTRY_EAX`].
#[inline]
pub fn entry_eax() -> u32 {
    ENTRY_EAX.load(core::sync::atomic::Ordering::Relaxed)
}

/// See [`ENTRY_EAX`] (host-side specs and tests).
#[inline]
pub fn set_entry_eax(v: u32) {
    ENTRY_EAX.store(v, core::sync::atomic::Ordering::Relaxed)
}

/// `(entry EAX & 0xFFFF0000) | ax`: a 16-bit result written with `mov ax, ...` / `xor ax, ax` while
/// the upper half still holds the caller's EAX.
#[inline]
pub fn with_entry_hi(ax: u16) -> u32 {
    (entry_eax() & 0xFFFF_0000) | u32::from(ax)
}

/// [`HookReg::preserve`]: ECX is returned unchanged.
pub const PRESERVE_ECX: u8 = 1;
/// [`HookReg::preserve`]: EDX is returned unchanged.
pub const PRESERVE_EDX: u8 = 2;

/// Where a function returns its result (MSVC 32-bit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbiRet {
    Void,
    /// `bool`/8-bit result in AL
    Al,
    /// 32-bit integer or pointer in EAX
    Eax,
    /// 64-bit integer in EDX:EAX
    EaxEdx,
    /// float in ST0
    St0,
}

/// 32-bit stack words an argument of type `T` occupies.
pub const fn stack_words<T>() -> u8 {
    core::mem::size_of::<T>().div_ceil(4) as u8
}

/// Bit `word` when an argument of type `T` (at stack word `word`) is `bytes` wide, else 0.
pub const fn narrow_bit<T>(word: u8, bytes: usize) -> u32 {
    if core::mem::size_of::<T>() == bytes && word < 32 {
        1u32 << word
    } else {
        0
    }
}

impl HookReg {
    /// A 5-byte `jmp rel32` entry: one of the 501 stubs into `.HOODLUM` (CLAUDE.md), hooked by
    /// retargeting the stub instead of relocating code.
    pub const fn is_hoodlum_stub(&self) -> bool {
        self.expect.len() == 5 && self.expect[0] == 0xE9
    }
}

/// Converts a port's return value into what the original returns in registers.
///
/// Identity for integers, pointers and `f32`; [`sa_math::x87::X87`] goes back as `f64`, which the
/// i686 C ABI returns in ST0 through `fld qword` — exact for every value the 24-bit-significand,
/// extended-exponent model holds, so callers that keep computing on ST0 see the original's value.
pub trait IntoAbi {
    type Abi;
    /// Where the original returns it.
    const RET: AbiRet;
    fn into_abi(self) -> Self::Abi;
}

macro_rules! identity_abi {
    ($($t:ty => $r:ident),*) => {$(
        impl IntoAbi for $t {
            type Abi = $t;
            const RET: AbiRet = AbiRet::$r;
            #[inline(always)]
            fn into_abi(self) -> $t {
                self
            }
        }
    )*};
}
identity_abi!(() => Void, bool => Al, u8 => Al, i8 => Al, u16 => Eax, i16 => Eax, u32 => Eax, i32 => Eax,
    u64 => EaxEdx, i64 => EaxEdx, f32 => St0, f64 => St0);

impl<T> IntoAbi for *mut T {
    type Abi = *mut T;
    const RET: AbiRet = AbiRet::Eax;
    #[inline(always)]
    fn into_abi(self) -> *mut T {
        self
    }
}

impl<T> IntoAbi for *const T {
    type Abi = *const T;
    const RET: AbiRet = AbiRet::Eax;
    #[inline(always)]
    fn into_abi(self) -> *const T {
        self
    }
}

impl<T> IntoAbi for crate::mem::GPtr<T> {
    type Abi = crate::mem::GPtr<T>;
    const RET: AbiRet = AbiRet::Eax;
    #[inline(always)]
    fn into_abi(self) -> Self {
        self
    }
}

impl IntoAbi for sa_math::x87::X87 {
    type Abi = f64;
    const RET: AbiRet = AbiRet::St0;
    #[inline(always)]
    fn into_abi(self) -> f64 {
        self.to_f64()
    }
}

/// MXCSR for the duration of a hook shim (ADR-001 §5).
///
/// The game runs with MXCSR 0x9FA0 (flush-to-zero on, measured by INF-3). The original's code is x87,
/// which MXCSR does not affect, but Rust compiles f32/f64 arithmetic (including the `X87` model) to
/// SSE, where FTZ would flush subnormal results. The guard clears FTZ and DAZ for the Rust code; calls
/// into original code run under the game's MXCSR ([`call_original`]), and on exit the guard loads the
/// game's MXCSR as those callees left it, so a Rust port affects MXCSR exactly as the original would.
/// No-op off x86.
pub struct FpuGuard {
    #[cfg(target_arch = "x86")]
    slot: usize,
    #[cfg(target_arch = "x86")]
    prev: u32,
}

/// MXCSR flush-to-zero (bit 15) and denormals-are-zero (bit 6).
pub const MXCSR_FTZ_DAZ: u32 = 0x8040;

/// Per-thread "game MXCSR" while Rust code runs under a guard (FPU-GUARD, 2026-11-10).
///
/// The guard used to restore the entry MXCSR on exit, which (1) discarded MXCSR changes made by
/// original code that Rust called (D3D device creation inside `RwInitialize` sets 0x9FA0), and
/// (2) ran those original callees with FTZ/DAZ cleared. Now the game's MXCSR is kept per thread:
/// [`call_original`] loads it for the callee, stores back what the callee left, and re-clears
/// FTZ/DAZ for Rust; the guard's exit loads the current game value. `no_std`: a small table keyed
/// by the Win32 thread id (TEB +0x24), not `thread_local!`.
#[cfg(target_arch = "x86")]
mod game_mxcsr {
    use core::sync::atomic::{AtomicU32, Ordering};

    const N: usize = 32;
    /// Value of `prev` meaning "the slot was free before this guard".
    pub const FREE: u32 = u32::MAX;
    static TID: [AtomicU32; N] = [const { AtomicU32::new(0) }; N];
    static VAL: [AtomicU32; N] = [const { AtomicU32::new(FREE) }; N];

    #[inline(always)]
    pub fn tid() -> u32 {
        #[cfg(windows)]
        {
            let t: u32;
            // SAFETY: reads the current thread id from the TEB (fs:[0x24]) on 32-bit Windows.
            #[allow(unsafe_code)]
            unsafe {
                core::arch::asm!("mov {0}, fs:[0x24]", out(reg) t, options(nostack, readonly, preserves_flags));
            }
            t
        }
        #[cfg(not(windows))]
        {
            1
        }
    }

    /// The slot of the current thread, claiming a free one if needed.
    pub fn slot() -> usize {
        let t = tid();
        if let Some(i) = TID.iter().position(|x| x.load(Ordering::Acquire) == t) {
            return i;
        }
        loop {
            if let Some(i) = TID.iter().position(|x| {
                x.compare_exchange(0, t, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
            }) {
                return i;
            }
            core::hint::spin_loop();
        }
    }

    /// The slot of the current thread if it is inside a guard.
    pub fn find() -> Option<usize> {
        let t = tid();
        (0..N).find(|&i| TID[i].load(Ordering::Acquire) == t && VAL[i].load(Ordering::Acquire) != FREE)
    }

    pub fn get(i: usize) -> u32 {
        VAL[i].load(Ordering::Acquire)
    }
    pub fn set(i: usize, v: u32) {
        VAL[i].store(v, Ordering::Release)
    }
    pub fn release(i: usize) {
        VAL[i].store(FREE, Ordering::Release);
        TID[i].store(0, Ordering::Release);
    }
}

#[cfg(target_arch = "x86")]
#[inline(always)]
#[allow(unsafe_code)]
fn stmxcsr() -> u32 {
    let mut v: u32 = 0;
    // SAFETY: stores MXCSR into a local.
    unsafe { core::arch::asm!("stmxcsr [{0}]", in(reg) &mut v, options(nostack, preserves_flags)) };
    v
}

#[cfg(target_arch = "x86")]
#[inline(always)]
#[allow(unsafe_code)]
fn ldmxcsr(v: u32) {
    // SAFETY: loads MXCSR from a local; callers only pass values read from MXCSR (possibly with
    // FTZ/DAZ cleared).
    unsafe { core::arch::asm!("ldmxcsr [{0}]", in(reg) &v, options(nostack, preserves_flags)) };
}

impl FpuGuard {
    #[inline(always)]
    pub fn enter() -> FpuGuard {
        #[cfg(target_arch = "x86")]
        {
            let entry = stmxcsr();
            let slot = game_mxcsr::slot();
            let prev = game_mxcsr::get(slot);
            game_mxcsr::set(slot, entry);
            ldmxcsr(entry & !MXCSR_FTZ_DAZ);
            FpuGuard { slot, prev }
        }
        #[cfg(not(target_arch = "x86"))]
        FpuGuard {}
    }
}

impl Drop for FpuGuard {
    #[inline(always)]
    fn drop(&mut self) {
        #[cfg(target_arch = "x86")]
        {
            // The game's MXCSR as the original callees left it (sticky flags included).
            ldmxcsr(game_mxcsr::get(self.slot));
            if self.prev == game_mxcsr::FREE {
                game_mxcsr::release(self.slot);
            } else {
                game_mxcsr::set(self.slot, self.prev);
            }
        }
    }
}

/// Run a call into original code (every `orig!` call in phase A) under the game's MXCSR.
///
/// Inside a guard: load the thread's game MXCSR, call, record the MXCSR the callee left as the new
/// game value, then clear FTZ/DAZ again for the Rust code that continues. Outside a guard: just call.
#[inline(always)]
pub fn call_original<R>(f: impl FnOnce() -> R) -> R {
    #[cfg(target_arch = "x86")]
    {
        if let Some(slot) = game_mxcsr::find() {
            ldmxcsr(game_mxcsr::get(slot));
            let r = f();
            let after = stmxcsr();
            game_mxcsr::set(slot, after);
            ldmxcsr(after & !MXCSR_FTZ_DAZ);
            return r;
        }
    }
    f()
}
