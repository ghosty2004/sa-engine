//! Helpers shared by the PORT-RENDER modules: raw field access (sa-world's), the x87 machine and compare
//! shorthands.

pub use sa_world::collision::fpu::Fpu;
pub use sa_world::world::common::{at, f32_of, ftol32, g, rd, wr, x, xi};

/// `TheCamera` (CCamera, 0xD78 bytes).
pub const THE_CAMERA: usize = 0xB6F028;

pub use sa_math::fpu::fbits as k;

pub use sa_math::fpu::fcom_parity as jp;

pub use sa_math::fpu::fcom_any as jne;

pub use sa_world::bindings::localisation_blood;
pub use sa_world::bindings::add_permanent_shadow as shadows_add_permanent_shadow;
pub use crate::bindings::fx_add_particle;
pub use sa_world::bindings::fx_prt_mult_ctor;
