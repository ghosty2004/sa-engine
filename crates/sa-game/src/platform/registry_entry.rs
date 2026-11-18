//! The registry entries of native replacements: a Rust function wrapped with the original's calling convention on
//! x86 and type-erased on wasm32 ([`sa_core::wabi`]), as `(address, name, entry)` rows for [`crate::native`].

/// The registry row of a cdecl replacement: `$f` wrapped as an `extern "C-unwind"` function. A function without a
/// result is written `-> ()`.
macro_rules! cdecl_entry {
    ($addr:expr, $name:literal, $f:path; fn($($a:ident : $t:ty),*) -> $r:ty) => {{
        #[allow(clippy::too_many_arguments, clippy::unused_unit)]
        extern "C-unwind" fn w($($a: $t),*) -> $r {
            $f($($a),*)
        }
        #[cfg(target_arch = "wasm32")]
        let e = sa_core::wabi_wrap!(w; $($a)*);
        #[cfg(not(target_arch = "wasm32"))]
        let e = w as *const () as usize;
        ($addr, $name, e)
    }};
}
pub(crate) use cdecl_entry;

/// As [`cdecl_entry`] for a thiscall method (`this` first); C on hosts without thiscall (tests).
macro_rules! thiscall_entry {
    ($addr:expr, $name:literal, $f:path; fn($($a:ident : $t:ty),*) -> $r:ty) => {{
        #[cfg(target_arch = "x86")]
        #[allow(clippy::unused_unit)]
        extern "thiscall-unwind" fn w($($a: $t),*) -> $r {
            $f($($a),*)
        }
        #[cfg(not(target_arch = "x86"))]
        #[allow(clippy::unused_unit)]
        extern "C-unwind" fn w($($a: $t),*) -> $r {
            $f($($a),*)
        }
        #[cfg(target_arch = "wasm32")]
        let e = sa_core::wabi_wrap!(w; $($a)*);
        #[cfg(not(target_arch = "wasm32"))]
        let e = w as *const () as usize;
        ($addr, $name, e)
    }};
}
pub(crate) use thiscall_entry;
