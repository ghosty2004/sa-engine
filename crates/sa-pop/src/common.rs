//! Helpers shared by the PORT-POP modules: raw field access (sa-world's), x87 shorthands, `rand()` with
//! its original call site, and the `orig!` declarations several modules call.

use sa_core::mem::GPtr;
use sa_core::orig;
use sa_math::x87::X87;

pub use sa_world::world::common::{at, f32_of, find_player_coors, find_player_ped, ftol32, g, rd, wr, x, xi};

/// `rand()` as the original call whose return address is `site` calls it (determinism mode hashes
/// `(caller, value)`).
#[inline]
pub fn rand_at(site: u32) -> i32 {
    sa_core::orig::at_site(site, sa_core::crt::rand)
}

/// `fild` of `(rand() & 0xFFFF)` times 3.0517578e-05 (0x858B14, 1/32768): the `[0, 2)` fraction the
/// inlined `CGeneral::GetRandomNumberInRange` sequences compute.
#[inline]
pub fn rand_frac(r: i32) -> X87 {
    xi(r & 0xFFFF).fmul_m(f32::from_bits(0x3800_0000))
}

/// `fcomp` + `fnstsw ax` + `test ah, 5` + `jp` taken: `a >= b` (or unordered never: C2|C0 both set).
#[inline]
pub fn jp5(a: X87, b: X87) -> bool {
    a.fcom(b).test_parity(0x05)
}

/// `fcomp` + `fnstsw ax` + `test ah, mask` + `jne` taken.
#[inline]
pub fn jne_mask(a: X87, b: X87, mask: u8) -> bool {
    a.fcom(b).test_any(mask)
}

/// `CModelInfo::ms_modelInfoPtrs[mi]`.
#[inline]
pub fn model_info(mi: i32) -> GPtr<u8> {
    sa_core::pools::MODEL_INFO_PTRS
        .ptr()
        .cast::<GPtr<u8>>()
        .wrapping_index(mi)
        .read()
}

orig! {
    /// `_ftol` (0x821B40) is modelled by `X87::ftol`; `CGeneral::GetATanOfXY(float, float)` (0x53CC70)
    pub fn get_atan_of_xy(x: f32, y: f32) -> f64 = cdecl 0x53CC70 => unported;
}
