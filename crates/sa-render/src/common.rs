//! Helpers shared by the PORT-RENDER modules: raw field access (sa-world's), the x87 machine and compare
//! shorthands.

use sa_math::x87::X87;

pub use sa_world::collision::fpu::Fpu;
pub use sa_world::world::common::{at, f32_of, ftol32, g, rd, wr, x, xi};

/// `TheCamera` (CCamera, 0xD78 bytes).
pub const THE_CAMERA: usize = 0xB6F028;

/// A float constant of the exe's `.rdata` by its bit pattern (the comments name its address).
#[inline]
pub fn k(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// `fcom(p)` + `fnstsw ax` + `test ah, mask` + `jp` taken (parity of `ah & mask` even).
#[inline]
pub fn jp(a: X87, b: X87, mask: u8) -> bool {
    a.fcom(b).test_parity(mask)
}

/// `fcom(p)` + `fnstsw ax` + `test ah, mask` + `jne` taken (any bit of `ah & mask` set).
#[inline]
pub fn jne(a: X87, b: X87, mask: u8) -> bool {
    a.fcom(b).test_any(mask)
}
