//! Helpers shared by the PORT-VEH modules: raw field access (sa-world's), x87 shorthands, `rand()` with
//! its original call site, and the `orig!` declarations several modules call.

use sa_core::mem::GPtr;

pub use sa_world::world::common::{at, f32_of, ftol32, g, rd, wr, x, xi};

pub use sa_core::crt::rand_at;

pub use sa_pop::common::rand_frac;
pub use sa_math::fpu::fcom_any as jne_mask;

pub use sa_math::fpu::ge_or_nan as jp5;

/// `CModelInfo::ms_modelInfoPtrs[mi]`.
#[inline]
pub fn model_info(mi: i32) -> GPtr<u8> {
    sa_core::pools::MODEL_INFO_PTRS
        .ptr()
        .cast::<GPtr<u8>>()
        .wrapping_index(mi)
        .read()
}

/// A C++ `bool` result in AL.
#[inline]
pub fn b(v: bool) -> u8 {
    u8::from(v)
}

pub use sa_world::bindings::entity_get_col_model;
