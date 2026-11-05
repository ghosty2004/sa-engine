//! `CMaths`: the 256-entry sine table and its lookups, ported from the 1.0 US machine code.
//!
//! The table lives in the game's `.bss` (0xBB3E00, filled by `InitMathsTables`); these ports take it as
//! an argument.

use crate::consts as k;
use crate::x87::X87;

/// `CMaths::InitMathsTables` (0x59AC90): `table[i] = f32(fsin(fild(i) * (2pi/256)))`.
pub fn init_maths_tables() -> [f32; 256] {
    let mut t = [0f32; 256];
    for (i, v) in t.iter_mut().enumerate() {
        *v = X87::from_i32(i as i32)
            .fmul_m(f32::from_bits(k::SIN_TABLE_STEP))
            .fsin()
            .to_f32();
    }
    t
}

/// `CMaths::GetSinFast(float)` (0x4A1340): `table[ftol(a * 256/(2pi)) & 0xFF]`.
pub fn get_sin_fast(table: &[f32; 256], a: f32) -> f32 {
    let i = X87::from_f32(a).fmul_m(f32::from_bits(k::SIN_TABLE_SCALE)).ftol() as u32 & 0xFF;
    table[i as usize]
}

/// `CMaths::GetCosFast(float)` (0x4A1360): `table[ftol(a * 256/(2pi) + 64) & 0xFF]`.
pub fn get_cos_fast(table: &[f32; 256], a: f32) -> f32 {
    let i = X87::from_f32(a)
        .fmul_m(f32::from_bits(k::SIN_TABLE_SCALE))
        .fadd_m(f32::from_bits(k::F64))
        .ftol() as u32
        & 0xFF;
    table[i as usize]
}
