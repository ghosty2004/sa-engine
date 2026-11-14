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

sa_core::orig! {
    /// `FxPrtMult_c::FxPrtMult_c(r, g, b, a, size, spin, life)` (0x4AB290)
    #[allow(clippy::too_many_arguments)]
    pub fn fx_prt_mult_ctor(p: sa_core::mem::GPtr<u8>, r: f32, g: f32, b: f32, a: f32, size: f32, spin: f32, life: f32) -> sa_core::mem::GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(RwV3d* pos, RwV3d* vel, float time, FxPrtMult_c*, float rot, float brightness,
    /// float zNear, uint8 local)` (0x4AA440)
    #[allow(clippy::too_many_arguments)]
    pub fn fx_add_particle(sys: sa_core::mem::GPtr<u8>, pos: sa_core::mem::GPtr<u8>, vel: sa_core::mem::GPtr<u8>, time: f32, mult: sa_core::mem::GPtr<u8>, rot: f32, bright: f32, near: f32, local: u32) = thiscall 0x4AA440 => unported;
    /// `CLocalisation::Blood()` (0x56D230, AL)
    pub fn localisation_blood() -> u8 = cdecl 0x56D230 => unported;
    /// `CShadows::AddPermanentShadow(type, RwTexture*, CVector*, frontX, frontY, sideX, sideY, intensity, r, g, b,
    /// drawDistance, time, upDistance)` (0x706F60)
    #[allow(clippy::too_many_arguments)]
    pub fn shadows_add_permanent_shadow(ty: u32, tex: u32, pos: sa_core::mem::GPtr<u8>, fx: f32, fy: f32, sx: f32, sy: f32, intensity: u32, r: u32, gr: u32, b: u32, dist: f32, time: u32, up: f32) = cdecl 0x706F60 => unported;
}
