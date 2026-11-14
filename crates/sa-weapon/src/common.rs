//! Helpers shared by the WEAPON modules: byte-offset field access (sa-world's PHYS accessors), x87
//! shorthands, the stack-model FPU of COLL/PHYS, and the original callees several modules call.

pub use sa_core::mem::GPtr;
pub use sa_math::x87::X87;
pub use sa_world::collision::fpu::Fpu;
pub use sa_world::physical::shared::{
    Ent, etype, lp, pos_ptr, rb, rf, rh, rp, ru, rv, rv_bits, wb, wf, wh, wp, wu, wv, wv_bits,
};
pub use sa_world::world::common::{ftol32, g, x, xi};

use sa_core::orig;

pub use sa_math::fpu::fbits as cf;

/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84), owned by sa-core.
#[inline]
pub fn time_ms() -> u32 {
    sa_core::timer::TIME_MS.get()
}

/// `CTimer::ms_fTimeStep` (0xB7CB5C), owned by sa-core.
#[inline]
pub fn time_step() -> f32 {
    sa_core::timer::TIME_STEP.get()
}

/// `CTimer::m_FrameCounter` (0xB7CB4C).
#[inline]
pub fn frame_counter() -> u32 {
    sa_core::timer::FRAME_COUNTER.get()
}

/// `TheCamera` position as the original reads it: `[0xB6F03C]` (the camera's matrix) `+ 0x30`, else
/// the placement vector at 0xB6F02C.
#[inline]
pub fn cam_pos() -> GPtr<f32> {
    sa_world::world::common::cam_pos()
}

pub use sa_core::bindings::atexit;
pub use sa_core::bindings::eh_vector_ctor;
pub use sa_core::bindings::eh_vector_dtor;
pub use sa_core::bindings::fx_play;
pub use sa_core::bindings::stricmp;
pub use sa_stream::bindings::rand;
pub use sa_world::bindings::fx_kill;
pub use sa_world::bindings::fx_play_and_kill;
orig! {
    /// `CGeneral::GetRandomNumberInRange<int32>(min, max)` (0x407180)
    pub fn random_int(min: i32, max: i32) -> i32 = cdecl 0x407180 => sa_core::general::get_random_number_in_range_i32;
    /// `CGeneral::GetRandomNumberInRange<float>(min, max)` (0x41BD90, ST0)
    pub fn random_float(min: f32, max: f32) -> f64 = cdecl 0x41BD90 => random_float_port;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    pub fn get_weapon_info(ty: i32, skill: u8) -> GPtr<u8> = cdecl 0x743C60 => crate::weapon_info::get_weapon_info;
}

/// Phases B/C path of `GetRandomNumberInRange<float>`: the full ST0 as an f64.
pub fn random_float_port(min: f32, max: f32) -> f64 {
    sa_core::general::get_random_number_in_range_f32(min, max).to_f64()
}

pub use sa_math::fpu::st0;
