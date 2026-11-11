//! VEH static initialisers (PORT-VEH). Ported from the 1.0 US bytes; gta-reversed as a reading aid.
//!
//! The f32 globals they write belong to file statics of the VEH sources that are not declared
//! anywhere yet, so they are written through `g`. Sources in `.data` are read at run time (another
//! initialiser or the game may have written them); `.rdata` constants are inlined.

use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::{g, x};

/// `[dst] = [src] * 10.0` (fld/fmul/fstp).
fn mul10(dst: usize, src: usize) {
    g::<f32>(dst).write(x(g::<f32>(src).read()).fmul_m(10.0).to_f32());
}

/// `[dst] = 0.277778 / div` (fld 0x858630, fdiv).
fn kmh(dst: usize, div: f32) {
    g::<f32>(dst).write(x(f32::from_bits(0x3E8E_38EB)).fdiv_m(div).to_f32());
}

/// `__init_g_97f634` (0x84A300): `0.005 * 0.0625`.
#[hook(addr = 0x84A300, conv = cdecl, name = "__init_g_97f634", expect = "D9 05 C0 9B 85 00")]
pub fn init_g_97f634() {
    g::<f32>(0x97F634).write(x(f32::from_bits(0x3BA3_D70A)).fmul_m(0.0625).to_f32());
}

/// `__init_g_97f63c` (0x84A340): `100.0 / 3.6`.
#[hook(addr = 0x84A340, conv = cdecl, name = "__init_g_97f63c", expect = "D9 05 28 86 85 00")]
pub fn init_g_97f63c() {
    g::<f32>(0x97F63C).write(x(100.0).fdiv_m(f32::from_bits(0x4066_6666)).to_f32());
}

/// `__init_g_c1bfec` (0x853500): `[0xC1BFE8] * 10.0`.
#[hook(addr = 0x853500, conv = cdecl, name = "__init_g_c1bfec", expect = "D9 05 E8 BF C1 00")]
pub fn init_g_c1bfec() {
    mul10(0xC1BFEC, 0xC1BFE8);
}

/// `__init_g_c1c818` (0x853600): `[0xC1C814] * 10.0`.
#[hook(addr = 0x853600, conv = cdecl, name = "__init_g_c1c818", expect = "D9 05 14 C8 C1 00")]
pub fn init_g_c1c818() {
    mul10(0xC1C818, 0xC1C814);
}

/// `__init_g_c1c81c` (0x853620): `0.277778 / 50.0`.
#[hook(addr = 0x853620, conv = cdecl, name = "__init_g_c1c81c", expect = "D9 05 30 86 85 00")]
pub fn init_g_c1c81c() {
    kmh(0xC1C81C, 50.0);
}

/// `__init_g_c1cb28` (0x853C20): `[0xC1CB24] * 10.0`.
#[hook(addr = 0x853C20, conv = cdecl, name = "__init_g_c1cb28", expect = "D9 05 24 CB C1 00")]
pub fn init_g_c1cb28() {
    mul10(0xC1CB28, 0xC1CB24);
}

/// `__init_g_c1cb54` (0x853CE0): `0.277778 / 50.0`.
#[hook(addr = 0x853CE0, conv = cdecl, name = "__init_g_c1cb54", expect = "D9 05 30 86 85 00")]
pub fn init_g_c1cb54() {
    kmh(0xC1CB54, 50.0);
}

/// `__init_fTimeMult` (0x8540B0): `fTimeMult = 1.2 / [0x8D3940]` (CBoat::WAKE_LIFETIME, .data 150.0).
#[hook(addr = 0x8540B0, conv = cdecl, name = "__init_fTimeMult", expect = "D9 05 08 8F 85 00")]
pub fn init_f_time_mult() {
    let v = x(f32::from_bits(0x3F99_999A)).fdiv_m(g::<f32>(0x8D3940).read());
    crate::boat::TIME_MULT.set(v.to_f32());
}

/// `__init_g_c2b9bc` (0x854390): `0.277778 / 50.0`.
#[hook(addr = 0x854390, conv = cdecl, name = "__init_g_c2b9bc", expect = "D9 05 30 86 85 00")]
pub fn init_g_c2b9bc() {
    kmh(0xC2B9BC, 50.0);
}

/// `__init_g_c3804c` (0x854410): `0.005 * 0.0625`.
#[hook(addr = 0x854410, conv = cdecl, name = "__init_g_c3804c", expect = "D9 05 58 23 87 00")]
pub fn init_g_c3804c() {
    g::<f32>(0xC3804C).write(x(f32::from_bits(0x3BA3_D70A)).fmul_m(0.0625).to_f32());
}

/// `__init_g_c2b9b4` (0x854350): `1.0 / (50.0 * 50.0)` (fdivr).
#[hook(addr = 0x854350, conv = cdecl, name = "__init_g_c2b9b4", expect = "D9 05 44 23 87 00")]
pub fn init_g_c2b9b4() {
    let v = x(50.0).fmul_m(50.0).fdivr_m(1.0);
    g::<f32>(0xC2B9B4).write(v.to_f32());
}

/// `__init_g_c1c208` (0x853540): the CVector `[0xC1C208] = [0x8D3198] - [0x8D318C]` (componentwise).
#[hook(addr = 0x853540, conv = cdecl, name = "__init_g_c1c208", expect = "D9 05 98 31 8D 00")]
pub fn init_g_c1c208() {
    for k in 0..3 {
        let v = x(g::<f32>(0x8D3198 + 4 * k).read()).fsub_m(g::<f32>(0x8D318C + 4 * k).read());
        g::<f32>(0xC1C208 + 4 * k).write(v.to_f32());
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_G_97F634,
    HOOK_INIT_G_97F63C,
    HOOK_INIT_G_C1BFEC,
    HOOK_INIT_G_C1C818,
    HOOK_INIT_G_C1C81C,
    HOOK_INIT_G_C1CB28,
    HOOK_INIT_G_C1CB54,
    HOOK_INIT_F_TIME_MULT,
    HOOK_INIT_G_C2B9BC,
    HOOK_INIT_G_C3804C,
    HOOK_INIT_G_C2B9B4,
    HOOK_INIT_G_C1C208,
];

/// This module's globals (replace with a `sa_core::globals! { list GLOBALS; ... }` block when it gets some).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
