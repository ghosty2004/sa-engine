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

/// `f32` from `.rdata` bits.
#[inline]
pub fn cf(bits: u32) -> f32 {
    f32::from_bits(bits)
}

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

orig! {
    /// CRT `rand()` (0x821B1E)
    pub fn rand() -> i32 = cdecl 0x821B1E => sa_core::crt::rand_owned;
    /// `CGeneral::GetRandomNumberInRange<int32>(min, max)` (0x407180)
    pub fn random_int(min: i32, max: i32) -> i32 = cdecl 0x407180 => sa_core::general::get_random_number_in_range_i32;
    /// `CGeneral::GetRandomNumberInRange<float>(min, max)` (0x41BD90, ST0)
    pub fn random_float(min: f32, max: f32) -> f64 = cdecl 0x41BD90 => random_float_port;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    pub fn get_weapon_info(ty: i32, skill: u8) -> GPtr<u8> = cdecl 0x743C60 => crate::weapon_info::get_weapon_info;
    /// CRT `_stricmp` (0x8229B6)
    pub fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// CRT `eh vector destructor iterator` (0x821E02, stdcall)
    pub fn eh_vector_dtor(arr: GPtr<u8>, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// CRT `_atexit` (0x821D1E)
    pub fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// `FxSystem_c::Kill` (0x4AA3F0, thiscall)
    pub fn fx_kill(fx: GPtr<u8>) = thiscall 0x4AA3F0 => unported;
    /// `FxSystem_c::Play` (0x4AA2F0, thiscall)
    pub fn fx_play(fx: GPtr<u8>) = thiscall 0x4AA2F0 => unported;
    /// `FxSystem_c::PlayAndKill` (0x4AA3D0, thiscall)
    pub fn fx_play_and_kill(fx: GPtr<u8>) = thiscall 0x4AA3D0 => unported;
}

/// Phases B/C path of `GetRandomNumberInRange<float>`: the full ST0 as an f64.
pub fn random_float_port(min: f32, max: f32) -> f64 {
    sa_core::general::get_random_number_in_range_f32(min, max).to_f64()
}

/// ST0 of an `orig!` call returning `f64`, as an x87 value (exact).
#[inline]
pub fn st0(v: f64) -> X87 {
    X87::from_f64_exact(v)
}
