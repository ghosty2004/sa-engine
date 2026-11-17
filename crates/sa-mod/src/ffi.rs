//! The binding layer: the host functions as wasm imports (generated from `sa_mod_api::for_each_import!`), and the
//! entry-point exports `entry!` generates (`__sa_mod_exports!`, from `sa_mod_api::for_each_export!`).
//!
//! Off wasm32 (host builds of a mod: its unit tests, `cargo clippy --workspace`) the host functions are stand-ins
//! that draw nothing and report a 0x0 screen.

macro_rules! imports {
    ($( $(#[$m:meta])* [$perm:ident] fn $name:ident( $($p:ident : $t:ident),* ) -> $r:ident; )*) => {
        #[cfg(target_arch = "wasm32")]
        #[link(wasm_import_module = "sa")]
        unsafe extern "C" {
            $( $(#[$m])* pub fn $name($($p: $t),*) -> $r; )*
        }

        $(
            /// Stand-in off wasm32.
            #[cfg(not(target_arch = "wasm32"))]
            #[allow(clippy::missing_safety_doc, clippy::too_many_arguments)]
            pub unsafe fn $name($($p: $t),*) -> $r {
                let _ = ($($p,)*);
                0
            }
        )*
    };
}
sa_mod_api::for_each_import!(imports);

/// The mod's exported entry points (used by [`crate::entry!`]): one `extern "C"` function per
/// `sa_mod_api::for_each_export!` item, forwarding to [`crate::rt`] with the mod type.
#[doc(hidden)]
#[macro_export]
macro_rules! __sa_mod_exports {
    ([$t:ty] $( $(#[$m:meta])* [$req:ident] fn $name:ident( $($p:ident : $pt:ident),* ) -> $r:ident; )*) => {
        $(
            $(#[$m])*
            #[unsafe(no_mangle)]
            pub extern "C" fn $name($($p: $pt),*) -> $r {
                $crate::rt::$name::<$t>($($p),*)
            }
        )*
    };
}
