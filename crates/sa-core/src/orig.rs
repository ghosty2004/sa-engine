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
//! - Phases B/C: the call goes straight to the Rust path after `=>`. `unported` makes it a panic
//!   naming the function (phase B starts only when F2 holds, i.e. nothing is unported).
//!
//! Every declared function is a safe Rust function (trust model of [`crate::mem`]).
#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

static SITE: AtomicU32 = AtomicU32::new(0);

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

/// Declares typed calls into possibly-original functions (see the module docs).
#[macro_export]
macro_rules! orig {
    () => {};
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($($a:ident : $t:ty),* $(,)?) $(-> $r:ty)? = $conv:ident site $site:literal => unported;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
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
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        $vis fn $name($($a: $t),*) $(-> $r)? {
            $crate::__orig_body!(ported $imp, $conv, ($crate::orig::import_at_call_site($site)), ($($a: $t),*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
    (
        $(#[$m:meta])*
        $vis:vis fn $name:ident($($a:ident : $t:ty),* $(,)?) $(-> $r:ty)? = $conv:ident $addr:literal => unported;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
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
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        $vis fn $name($($a: $t),*) $(-> $r)? {
            $crate::__orig_body!(ported $imp, $conv, ($addr as usize), ($($a: $t),*) $(-> $r)?)
        }
        $crate::orig!($($rest)*);
    };
}

/// Phase A: call the original address with the original convention.
#[cfg(sa_phase_a)]
#[doc(hidden)]
#[macro_export]
macro_rules! __orig_body {
    ($kind:ident $what:path, cdecl, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        #[allow(unsafe_code)]
        let f = unsafe {
            ::core::mem::transmute::<usize, unsafe extern "C" fn($($t),*) $(-> $r)?>($addr)
        };
        #[allow(unsafe_code)]
        unsafe { f($($a),*) }
    }};
    ($kind:ident $what:path, stdcall, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        #[allow(unsafe_code)]
        let f = unsafe {
            ::core::mem::transmute::<usize, unsafe extern "stdcall" fn($($t),*) $(-> $r)?>($addr)
        };
        #[allow(unsafe_code)]
        unsafe { f($($a),*) }
    }};
    ($kind:ident $what:path, thiscall, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        #[allow(unsafe_code)]
        let f = unsafe {
            ::core::mem::transmute::<usize, unsafe extern "thiscall" fn($($t),*) $(-> $r)?>($addr)
        };
        #[allow(unsafe_code)]
        unsafe { f($($a),*) }
    }};
    ($kind:ident $what:path, fastcall, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        #[allow(unsafe_code)]
        let f = unsafe {
            ::core::mem::transmute::<usize, unsafe extern "fastcall" fn($($t),*) $(-> $r)?>($addr)
        };
        #[allow(unsafe_code)]
        unsafe { f($($a),*) }
    }};
}

/// Phases B/C: call the Rust implementation, or fail loudly for an unported function.
#[cfg(not(sa_phase_a))]
#[doc(hidden)]
#[macro_export]
macro_rules! __orig_body {
    (ported $imp:path, $conv:ident, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {
        $imp($($a),*)
    };
    (unported $name:path, $conv:ident, ($addr:expr), ($($a:ident : $t:ty),*) $(-> $r:ty)?) => {{
        $(let _ = $a;)*
        panic!(concat!(
            stringify!($name),
            " is not ported; phases B/C have no original code to call (",
            stringify!($addr),
            ")"
        ))
    }};
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
    }

    #[test]
    fn owned_builds_call_the_rust_implementation() {
        assert_eq!(doubled(21), 42);
        let r = std::panic::catch_unwind(|| missing(1));
        assert!(r.is_err());
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
