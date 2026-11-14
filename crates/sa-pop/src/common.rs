//! Helpers shared by the PORT-POP modules: raw field access (sa-world's), x87 shorthands, `rand()` with
//! its original call site, and the `orig!` declarations several modules call.

use sa_core::mem::GPtr;
use sa_math::x87::X87;

pub use sa_world::world::common::{at, f32_of, find_player_coors, find_player_ped, ftol32, g, rd, wr, x, xi};

pub use sa_core::crt::rand_at;

/// `fild` of `(rand() & 0xFFFF)` times 3.0517578e-05 (0x858B14, 1/32768): the `[0, 2)` fraction the
/// inlined `CGeneral::GetRandomNumberInRange` sequences compute.
#[inline]
pub fn rand_frac(r: i32) -> X87 {
    xi(r & 0xFFFF).fmul_m(f32::from_bits(0x3800_0000))
}

pub use sa_math::fpu::ge_or_nan as jp5;

pub use sa_math::fpu::fcom_any as jne_mask;

/// `CModelInfo::ms_modelInfoPtrs[mi]`.
#[inline]
pub fn model_info(mi: i32) -> GPtr<u8> {
    sa_core::pools::MODEL_INFO_PTRS
        .ptr()
        .cast::<GPtr<u8>>()
        .wrapping_index(mi)
        .read()
}

pub use sa_core::bindings::get_atan_of_xy;
