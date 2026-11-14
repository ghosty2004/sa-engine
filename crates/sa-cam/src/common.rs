//! Helpers shared by the PORT-CAM modules: raw field access (sa-world's), the x87 machine and compare
//! shorthands, and the camera's fixed address.

use sa_core::mem::GPtr;

pub use sa_world::collision::fpu::Fpu;
pub use sa_world::world::common::{at, f32_of, ftol32, g, rd, wr, x, xi};

/// `TheCamera` (CCamera, 0xD78 bytes).
pub const THE_CAMERA: usize = 0xB6F028;

/// `TheCamera` as a `this` pointer.
#[inline]
pub fn the_camera() -> GPtr<u8> {
    GPtr::from_addr(THE_CAMERA)
}

/// Stride of `CCamera::m_aCams[3]` (sizeof(CCam)).
pub const CCAM_SIZE: isize = 0x238;
/// Offset of `CCamera::m_aCams[0]`.
pub const CAMS: isize = 0x174;
/// Offset of `CCamera::m_nActiveCam` (u8).
pub const ACTIVE_CAM: isize = 0x59;

/// `&this->m_aCams[i]`.
#[inline]
pub fn cam_of(this: GPtr<u8>, i: i32) -> GPtr<u8> {
    this.byte_add(CAMS + i as isize * CCAM_SIZE)
}

/// `&this->m_aCams[this->m_nActiveCam]`.
#[inline]
pub fn active_cam(this: GPtr<u8>) -> GPtr<u8> {
    cam_of(this, i32::from(rd::<u8>(this, ACTIVE_CAM)))
}

pub use sa_math::fpu::fbits as k;

pub use sa_math::fpu::fcom_parity as jp;

pub use sa_math::fpu::fcom_any as jne;
