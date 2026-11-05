//! Game memory: globals, game pointers, plain-data types (ADR-001 §2–§3).
//!
//! Every raw memory access of the engine goes through this module: [`Global`] (a game global) and
//! [`GPtr`] (a pointer into game memory). In phase A both refer to the original process's memory
//! (globals at their fixed 1.0 US addresses, objects in the game's heap and pools); in phases B/C
//! globals live in Rust-owned statics declared by the same [`globals!`] line and pointers point into
//! engine-owned memory. Port code is written once against these types.
//!
//! Trust model. Phase A shares the process with original code, so Rust cannot prove these accesses
//! sound: a game pointer is valid because the original program's invariants say so, exactly as in the
//! C++ it replaces. The accessors are therefore safe functions with one documented contract, kept in
//! one place that can be audited and instrumented (e.g. read/write-set recording for L2):
//! - only the game thread uses them (the original game loop is single-threaded; sa-hook runs Rust
//!   ports on the thread that called the original function);
//! - no Rust reference (`&`/`&mut`) to game memory outlives a single access, so original code that
//!   runs in between can never invalidate one;
//! - values are read and written as [`GameData`]: types for which every bit pattern is valid, so a
//!   value written by original code can never be an invalid Rust value.
#![allow(unsafe_code)]

use core::marker::PhantomData;

/// Plain data: `Copy`, no padding requirements on reads, and every bit pattern is a valid value.
///
/// # Safety
/// Implement only for types where any byte content is a valid value (integers, floats, raw/game
/// pointers, `Option` of a function pointer, arrays of these, `#[repr(C)]` structs of these).
pub unsafe trait GameData: Copy + 'static {}

macro_rules! game_data {
    ($($t:ty),*) => {$(
        // SAFETY: every bit pattern is a valid value of these types.
        unsafe impl GameData for $t {}
    )*};
}
game_data!(u8, i8, u16, i16, u32, i32, u64, i64, f32, f64, GBool);
// SAFETY: raw pointers have no validity invariant.
unsafe impl<T: 'static> GameData for *mut T {}
// SAFETY: as above.
unsafe impl<T: 'static> GameData for *const T {}
// SAFETY: transparent over a raw pointer.
unsafe impl<T: 'static> GameData for GPtr<T> {}
// SAFETY: arrays of plain data are plain data.
unsafe impl<T: GameData, const N: usize> GameData for [T; N] {}
// SAFETY: a function pointer's only validity requirement is non-null, which `Option` absorbs.
unsafe impl GameData for Option<TimerFn> {}

/// `CTimer::ms_fnTimerFunction`'s type: `uint64 (*)()`, result in EDX:EAX.
pub type TimerFn = unsafe extern "C" fn() -> u64;

/// A C++ `bool` in game memory: one byte, `0` = false, anything else = true (MSVC tests it with
/// `test al, al`). Stores write 0 or 1, as MSVC does.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GBool(pub u8);

impl GBool {
    pub const FALSE: GBool = GBool(0);
    pub const TRUE: GBool = GBool(1);
    pub const fn from_bool(b: bool) -> GBool {
        GBool(b as u8)
    }
    pub const fn get(self) -> bool {
        self.0 != 0
    }
}

/// A pointer into game memory (a C++ `T*` field or argument). `#[repr(transparent)]` over `*mut T`,
/// so it has the original's 4-byte layout on i686 and wasm32 and passes in registers like `T*`.
#[repr(transparent)]
pub struct GPtr<T>(*mut T, PhantomData<T>);

impl<T> Clone for GPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for GPtr<T> {}
impl<T> PartialEq for GPtr<T> {
    fn eq(&self, o: &Self) -> bool {
        core::ptr::eq(self.0, o.0)
    }
}
impl<T> Eq for GPtr<T> {}
impl<T> core::fmt::Debug for GPtr<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "GPtr({:#x})", self.addr())
    }
}

impl<T> GPtr<T> {
    pub const NULL: GPtr<T> = GPtr(core::ptr::null_mut(), PhantomData);

    pub const fn from_raw(p: *mut T) -> GPtr<T> {
        GPtr(p, PhantomData)
    }
    /// A game address (phase A) or an engine address as an integer.
    pub fn from_addr(a: usize) -> GPtr<T> {
        GPtr(core::ptr::without_provenance_mut(a), PhantomData)
    }
    pub const fn raw(self) -> *mut T {
        self.0
    }
    pub fn addr(self) -> usize {
        self.0 as usize
    }
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
    pub const fn cast<U>(self) -> GPtr<U> {
        GPtr(self.0.cast(), PhantomData)
    }
    /// `self + i` elements, wrapping like the original's 32-bit `imul`/`add` (`base + i * sizeof(T)`).
    pub fn wrapping_index(self, i: i32) -> GPtr<T> {
        let bytes = (i as isize).wrapping_mul(core::mem::size_of::<T>() as isize);
        GPtr(self.0.wrapping_byte_offset(bytes), PhantomData)
    }
    /// `self + n` bytes (wrapping).
    pub fn byte_add(self, n: isize) -> GPtr<T> {
        GPtr(self.0.wrapping_byte_offset(n), PhantomData)
    }
    /// The C++ pointer difference `(self - base)` in elements, as MSVC computes it for 32-bit
    /// pointers: a wrapping 32-bit byte difference divided (signed, truncating) by `sizeof(T)`.
    pub fn index_from(self, base: GPtr<T>) -> i32 {
        let diff = (self.addr() as u32).wrapping_sub(base.addr() as u32) as i32;
        diff / core::mem::size_of::<T>() as i32
    }
}

impl<T: GameData> GPtr<T> {
    /// Reads the value (see the module's trust model).
    #[inline]
    pub fn read(self) -> T {
        // SAFETY: trust model (module docs): a game pointer the original program would dereference.
        unsafe { self.0.read_unaligned() }
    }
    /// Writes the value (see the module's trust model).
    #[inline]
    pub fn write(self, v: T) {
        // SAFETY: as for `read`.
        unsafe { self.0.write_unaligned(v) }
    }
}

/// A game global: `addr` in phase A, a Rust-owned static otherwise. Declared with [`globals!`].
pub struct Global<T: 'static> {
    addr: u32,
    name: &'static str,
    #[cfg_attr(sa_phase_a, allow(dead_code))]
    store: fn() -> *mut T,
}

// SAFETY: accessed only from the game thread (trust model); the descriptor itself is immutable.
unsafe impl<T> Sync for Global<T> {}

impl<T: GameData> Global<T> {
    #[doc(hidden)]
    pub const fn new(addr: u32, name: &'static str, store: fn() -> *mut T) -> Global<T> {
        Global { addr, name, store }
    }
    /// Address in gta_sa.exe 1.0 US.
    pub const fn addr(&self) -> u32 {
        self.addr
    }
    pub const fn name(&self) -> &'static str {
        self.name
    }
    /// Where the value lives in this build.
    #[inline]
    pub fn ptr(&self) -> GPtr<T> {
        #[cfg(sa_phase_a)]
        {
            GPtr::from_addr(self.addr as usize)
        }
        #[cfg(not(sa_phase_a))]
        {
            GPtr::from_raw((self.store)())
        }
    }
    #[inline]
    pub fn get(&self) -> T {
        self.ptr().read()
    }
    #[inline]
    pub fn set(&self, v: T) {
        self.ptr().write(v)
    }
}

/// Static description of one global (for tests: initial value against the exe's image).
#[derive(Clone, Copy, Debug)]
pub struct GlobalDesc {
    pub name: &'static str,
    pub addr: u32,
    pub size: usize,
    /// Initial value as bytes (phase B/C storage starts with it; the exe's image must agree).
    pub init: &'static [u8],
}

/// Declares game globals once for every phase.
///
/// ```ignore
/// sa_core::globals! {
///     list TIMER_GLOBALS;
///     /// `CTimer::m_snTimeInMilliseconds`
///     pub static TIME_IN_MS @ 0xB7CB84: u32 = 0;
/// }
/// ```
/// Each line gives the 1.0 US address, the type (a [`GameData`] type with the original's layout)
/// and the initial value of the original's image (`.data` contents, 0 for `.bss`). Phase A ignores
/// the initial value (the exe provides it); phases B/C start from it. `list NAME;` also emits
/// `NAME: &[GlobalDesc]` so tests can check the initial values against the exe.
#[macro_export]
macro_rules! globals {
    (
        list $list:ident;
        $(
            $(#[$m:meta])*
            $vis:vis static $name:ident @ $addr:literal : $ty:ty = $init:expr;
        )*
    ) => {
        $(
            $(#[$m])*
            $vis static $name: $crate::mem::Global<$ty> =
                $crate::mem::Global::new($addr, stringify!($name), $crate::__global_store!($ty, $init));
        )*
        #[allow(dead_code)]
        pub const $list: &[$crate::mem::GlobalDesc] = &[$(
            $crate::mem::GlobalDesc {
                name: stringify!($name),
                addr: $addr,
                size: ::core::mem::size_of::<$ty>(),
                init: {
                    const V: $ty = $init;
                    #[allow(unsafe_code, unknown_lints, unnecessary_transmutes, clippy::useless_transmute)]
                    const B: [u8; ::core::mem::size_of::<$ty>()] = unsafe {
                        ::core::mem::transmute::<$ty, [u8; ::core::mem::size_of::<$ty>()]>(V)
                    };
                    &B
                },
            },
        )*];
    };
}

#[cfg(sa_phase_a)]
#[doc(hidden)]
#[macro_export]
macro_rules! __global_store {
    ($ty:ty, $init:expr) => {{
        fn store() -> *mut $ty {
            ::core::ptr::null_mut()
        }
        store
    }};
}

#[cfg(not(sa_phase_a))]
#[doc(hidden)]
#[macro_export]
macro_rules! __global_store {
    ($ty:ty, $init:expr) => {{
        #[allow(unsafe_code)]
        fn store() -> *mut $ty {
            static mut S: $ty = $init;
            &raw mut S
        }
        store
    }};
}

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    crate::globals! {
        list TEST_GLOBALS;
        static A @ 0x1000: u32 = 7;
        static B @ 0x1004: f32 = 1.5;
        static P @ 0x1008: GPtr<u8> = GPtr::NULL;
    }

    #[test]
    fn owned_globals_start_with_their_initial_value() {
        const { assert!(!crate::PHASE_A) };
        assert_eq!(A.get(), 7);
        A.set(9);
        assert_eq!(A.get(), 9);
        assert_eq!(B.get(), 1.5);
        assert!(P.get().is_null());
        assert_eq!(A.addr(), 0x1000);
        assert_eq!(TEST_GLOBALS.len(), 3);
        assert_eq!(TEST_GLOBALS[0].init, &7u32.to_le_bytes());
        assert_eq!(TEST_GLOBALS[1].init, &1.5f32.to_le_bytes());
        assert_eq!(TEST_GLOBALS[2].init, &[0u8; core::mem::size_of::<usize>()][..]);
    }

    #[test]
    fn pointer_math_is_the_originals() {
        let base = GPtr::<[u8; 0x7C4]>::from_addr(0x1000_0000);
        assert_eq!(base.wrapping_index(3).addr(), 0x1000_0000 + 3 * 0x7C4);
        assert_eq!(base.wrapping_index(3).index_from(base), 3);
        // below the base: signed, truncating toward zero
        assert_eq!(base.byte_add(-1).index_from(base), 0);
        assert_eq!(base.byte_add(-0x7C5).index_from(base), -1);
    }
}
