//! Pure Rust ports of `gta_sa.exe` 1.0 US math functions (MATH subsystem: `CGeneral`, `CMaths`,
//! `CVector`, ...). No unsafe, no host float behaviour that differs between targets (wasm included):
//! x87 semantics come from [`x87`], transcendentals from `libm`.
//!
//! Each function names its original address. They are verified against the original bytes by
//! `sa-oracle` (L1 runner, see `docs/re/l1-harness.md`).

/// The deterministic libm (phase B stands in with it for the CRT's math intrinsics).
pub use libm;

pub mod col;
pub mod compressed;
pub mod consts;
pub mod general;
pub mod maths;
pub mod matrix;
pub mod matrix_link;
pub mod quaternion;
pub mod rect;
pub mod vector;
pub mod x87;
mod x87_wide;
