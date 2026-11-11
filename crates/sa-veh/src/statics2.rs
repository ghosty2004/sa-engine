//! VEH static initialisers, second pass (PORT-VEH-2): the 109 `__init_g_*` functions that RE-SPEC-UNK
//! proposed as VEH. Every one is `fld m32; (fmul|fdiv|fdivr m32)+; fstp m32; ret` (generated from the
//! 1.0 US bytes by the PORT-VEH-2 agent; same shape as `statics.rs`). Every operand is read from memory
//! at run time (`.rdata` constants and `.data` values alike), so the order of the initialisers does not
//! matter; arithmetic goes through the x87 model (PC = 24).

use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::{g, x};

/// `fld dword [a]`.
fn ld(a: usize) -> sa_math::x87::X87 {
    x(g::<f32>(a).read())
}

/// `fstp dword [a]`.
fn st(a: usize, v: sa_math::x87::X87) {
    g::<f32>(a).write(v.to_f32());
}

/// The f32 at `a` (an `m32` operand).
fn m(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// `__init_g_97d648` (0x84A240): `[0x97D648] = [0x859AF8] * [0x858620]`.
#[hook(addr = 0x84A240, conv = cdecl, name = "__init_g_97d648", expect = "D9 05 F8 9A 85 00")]
pub fn init_g_97d648() {
    st(0x97D648, ld(0x859AF8).fmul_m(m(0x858620)));
}

/// `__init_g_97d658` (0x84A2C0): `[0x97D658] = [0x97D654] * [0x85862C]`.
#[hook(addr = 0x84A2C0, conv = cdecl, name = "__init_g_97d658", expect = "D9 05 54 D6 97 00")]
pub fn init_g_97d658() {
    st(0x97D658, ld(0x97D654).fmul_m(m(0x85862C)));
}

/// `__init_g_97d65c` (0x84A2E0): `[0x97D65C] = [0x858630] / [0x859B00]`.
#[hook(addr = 0x84A2E0, conv = cdecl, name = "__init_g_97d65c", expect = "D9 05 30 86 85 00")]
pub fn init_g_97d65c() {
    st(0x97D65C, ld(0x858630).fdiv_m(m(0x859B00)));
}

/// `__init_g_97f638` (0x84A320): `[0x97F638] = [0x858624] / [0x859BC8]`.
#[hook(addr = 0x84A320, conv = cdecl, name = "__init_g_97f638", expect = "D9 05 24 86 85 00")]
pub fn init_g_97f638() {
    st(0x97F638, ld(0x858624).fdiv_m(m(0x859BC8)));
}

/// `__init_g_97f644` (0x84A380): `[0x97F644] = [0x97F640] * [0x85862C]`.
#[hook(addr = 0x84A380, conv = cdecl, name = "__init_g_97f644", expect = "D9 05 40 F6 97 00")]
pub fn init_g_97f644() {
    st(0x97F644, ld(0x97F640).fmul_m(m(0x85862C)));
}

/// `__init_g_97f648` (0x84A3A0): `[0x97F648] = [0x858630] / [0x859BC8]`.
#[hook(addr = 0x84A3A0, conv = cdecl, name = "__init_g_97f648", expect = "D9 05 30 86 85 00")]
pub fn init_g_97f648() {
    st(0x97F648, ld(0x858630).fdiv_m(m(0x859BC8)));
}

/// `__init_g_c18c08` (0x852200): `[0xC18C08] = [0x86EC24] * [0x858620]`.
#[hook(addr = 0x852200, conv = cdecl, name = "__init_g_c18c08", expect = "D9 05 24 EC 86 00")]
pub fn init_g_c18c08() {
    st(0xC18C08, ld(0x86EC24).fmul_m(m(0x858620)));
}

/// `__init_g_c18c0c` (0x852220): `[0xC18C0C] = [0x858624] / [0x86EC2C]`.
#[hook(addr = 0x852220, conv = cdecl, name = "__init_g_c18c0c", expect = "D9 05 24 86 85 00")]
pub fn init_g_c18c0c() {
    st(0xC18C0C, ld(0x858624).fdiv_m(m(0x86EC2C)));
}

/// `__init_g_c18c10` (0x852240): `[0xC18C10] = [0x858628] / [0x86EC28]`.
#[hook(addr = 0x852240, conv = cdecl, name = "__init_g_c18c10", expect = "D9 05 28 86 85 00")]
pub fn init_g_c18c10() {
    st(0xC18C10, ld(0x858628).fdiv_m(m(0x86EC28)));
}

/// `__init_g_c18c18` (0x852280): `[0xC18C18] = [0xC18C14] * [0x85862C]`.
#[hook(addr = 0x852280, conv = cdecl, name = "__init_g_c18c18", expect = "D9 05 14 8C C1 00")]
pub fn init_g_c18c18() {
    st(0xC18C18, ld(0xC18C14).fmul_m(m(0x85862C)));
}

/// `__init_g_c18c1c` (0x8522A0): `[0xC18C1C] = [0x858630] / [0x86EC2C]`.
#[hook(addr = 0x8522A0, conv = cdecl, name = "__init_g_c18c1c", expect = "D9 05 30 86 85 00")]
pub fn init_g_c18c1c() {
    st(0xC18C1C, ld(0x858630).fdiv_m(m(0x86EC2C)));
}

/// `__init_g_c18c24` (0x8522C0): `[0xC18C24] = [0x86F0EC] * [0x858620]`.
#[hook(addr = 0x8522C0, conv = cdecl, name = "__init_g_c18c24", expect = "D9 05 EC F0 86 00")]
pub fn init_g_c18c24() {
    st(0xC18C24, ld(0x86F0EC).fmul_m(m(0x858620)));
}

/// `__init_g_c18c28` (0x8522E0): `[0xC18C28] = [0x858624] / [0x86F0F4]`.
#[hook(addr = 0x8522E0, conv = cdecl, name = "__init_g_c18c28", expect = "D9 05 24 86 85 00")]
pub fn init_g_c18c28() {
    st(0xC18C28, ld(0x858624).fdiv_m(m(0x86F0F4)));
}

/// `__init_g_c18c2c` (0x852300): `[0xC18C2C] = [0x858628] / [0x86F0F0]`.
#[hook(addr = 0x852300, conv = cdecl, name = "__init_g_c18c2c", expect = "D9 05 28 86 85 00")]
pub fn init_g_c18c2c() {
    st(0xC18C2C, ld(0x858628).fdiv_m(m(0x86F0F0)));
}

/// `__init_g_c18c34` (0x852340): `[0xC18C34] = [0xC18C30] * [0x85862C]`.
#[hook(addr = 0x852340, conv = cdecl, name = "__init_g_c18c34", expect = "D9 05 30 8C C1 00")]
pub fn init_g_c18c34() {
    st(0xC18C34, ld(0xC18C30).fmul_m(m(0x85862C)));
}

/// `__init_g_c18c38` (0x852360): `[0xC18C38] = [0x858630] / [0x86F0F4]`.
#[hook(addr = 0x852360, conv = cdecl, name = "__init_g_c18c38", expect = "D9 05 30 86 85 00")]
pub fn init_g_c18c38() {
    st(0xC18C38, ld(0x858630).fdiv_m(m(0x86F0F4)));
}

/// `__init_g_c1aeb0` (0x853360): `[0xC1AEB0] = [0x870F44] * [0x858620]`.
#[hook(addr = 0x853360, conv = cdecl, name = "__init_g_c1aeb0", expect = "D9 05 44 0F 87 00")]
pub fn init_g_c1aeb0() {
    st(0xC1AEB0, ld(0x870F44).fmul_m(m(0x858620)));
}

/// `__init_g_c1b328` (0x8533A0): `[0xC1B328] = [0x870F4C] * [0x858620]`.
#[hook(addr = 0x8533A0, conv = cdecl, name = "__init_g_c1b328", expect = "D9 05 4C 0F 87 00")]
pub fn init_g_c1b328() {
    st(0xC1B328, ld(0x870F4C).fmul_m(m(0x858620)));
}

/// `__init_g_c1b32c` (0x8533C0): `[0xC1B32C] = [0x858624] / [0x870F54]`.
#[hook(addr = 0x8533C0, conv = cdecl, name = "__init_g_c1b32c", expect = "D9 05 24 86 85 00")]
pub fn init_g_c1b32c() {
    st(0xC1B32C, ld(0x858624).fdiv_m(m(0x870F54)));
}

/// `__init_g_c1b330` (0x8533E0): `[0xC1B330] = [0x858628] / [0x870F50]`.
#[hook(addr = 0x8533E0, conv = cdecl, name = "__init_g_c1b330", expect = "D9 05 28 86 85 00")]
pub fn init_g_c1b330() {
    st(0xC1B330, ld(0x858628).fdiv_m(m(0x870F50)));
}

/// `__init_g_c1b338` (0x853420): `[0xC1B338] = [0xC1B334] * [0x85862C]`.
#[hook(addr = 0x853420, conv = cdecl, name = "__init_g_c1b338", expect = "D9 05 34 B3 C1 00")]
pub fn init_g_c1b338() {
    st(0xC1B338, ld(0xC1B334).fmul_m(m(0x85862C)));
}

/// `__init_g_c1b33c` (0x853440): `[0xC1B33C] = [0x858630] / [0x870F54]`.
#[hook(addr = 0x853440, conv = cdecl, name = "__init_g_c1b33c", expect = "D9 05 30 86 85 00")]
pub fn init_g_c1b33c() {
    st(0xC1B33C, ld(0x858630).fdiv_m(m(0x870F54)));
}

/// `__init_g_c1bfdc` (0x853480): `[0xC1BFDC] = [0x870FC0] * [0x858620]`.
#[hook(addr = 0x853480, conv = cdecl, name = "__init_g_c1bfdc", expect = "D9 05 C0 0F 87 00")]
pub fn init_g_c1bfdc() {
    st(0xC1BFDC, ld(0x870FC0).fmul_m(m(0x858620)));
}

/// `__init_g_c1bfe0` (0x8534A0): `[0xC1BFE0] = [0x858624] / [0x870FC8]`.
#[hook(addr = 0x8534A0, conv = cdecl, name = "__init_g_c1bfe0", expect = "D9 05 24 86 85 00")]
pub fn init_g_c1bfe0() {
    st(0xC1BFE0, ld(0x858624).fdiv_m(m(0x870FC8)));
}

/// `__init_g_c1bfe4` (0x8534C0): `[0xC1BFE4] = [0x858628] / [0x870FC4]`.
#[hook(addr = 0x8534C0, conv = cdecl, name = "__init_g_c1bfe4", expect = "D9 05 28 86 85 00")]
pub fn init_g_c1bfe4() {
    st(0xC1BFE4, ld(0x858628).fdiv_m(m(0x870FC4)));
}

/// `__init_g_c1bff0` (0x853520): `[0xC1BFF0] = [0x858630] / [0x870FC8]`.
#[hook(addr = 0x853520, conv = cdecl, name = "__init_g_c1bff0", expect = "D9 05 30 86 85 00")]
pub fn init_g_c1bff0() {
    st(0xC1BFF0, ld(0x858630).fdiv_m(m(0x870FC8)));
}

/// `__init_g_c1c808` (0x853580): `[0xC1C808] = [0x871274] * [0x858620]`.
#[hook(addr = 0x853580, conv = cdecl, name = "__init_g_c1c808", expect = "D9 05 74 12 87 00")]
pub fn init_g_c1c808() {
    st(0xC1C808, ld(0x871274).fmul_m(m(0x858620)));
}

/// `__init_g_c1c80c` (0x8535A0): `[0xC1C80C] = [0x858624] / [0x87127C]`.
#[hook(addr = 0x8535A0, conv = cdecl, name = "__init_g_c1c80c", expect = "D9 05 24 86 85 00")]
pub fn init_g_c1c80c() {
    st(0xC1C80C, ld(0x858624).fdiv_m(m(0x87127C)));
}

/// `__init_g_c1c810` (0x8535C0): `[0xC1C810] = [0x858628] / [0x871278]`.
#[hook(addr = 0x8535C0, conv = cdecl, name = "__init_g_c1c810", expect = "D9 05 28 86 85 00")]
pub fn init_g_c1c810() {
    st(0xC1C810, ld(0x858628).fdiv_m(m(0x871278)));
}

/// `__init_g_c1c824` (0x853640): `[0xC1C824] = [0x8714B0] * [0x858620]`.
#[hook(addr = 0x853640, conv = cdecl, name = "__init_g_c1c824", expect = "D9 05 B0 14 87 00")]
pub fn init_g_c1c824() {
    st(0xC1C824, ld(0x8714B0).fmul_m(m(0x858620)));
}

/// `__init_g_c1c828` (0x853660): `[0xC1C828] = [0x858624] / [0x8714B8]`.
#[hook(addr = 0x853660, conv = cdecl, name = "__init_g_c1c828", expect = "D9 05 24 86 85 00")]
pub fn init_g_c1c828() {
    st(0xC1C828, ld(0x858624).fdiv_m(m(0x8714B8)));
}

/// `__init_g_c1c82c` (0x853680): `[0xC1C82C] = [0x858628] / [0x8714B4]`.
#[hook(addr = 0x853680, conv = cdecl, name = "__init_g_c1c82c", expect = "D9 05 28 86 85 00")]
pub fn init_g_c1c82c() {
    st(0xC1C82C, ld(0x858628).fdiv_m(m(0x8714B4)));
}

/// `__init_g_c1c834` (0x8536C0): `[0xC1C834] = [0xC1C830] * [0x85862C]`.
#[hook(addr = 0x8536C0, conv = cdecl, name = "__init_g_c1c834", expect = "D9 05 30 C8 C1 00")]
pub fn init_g_c1c834() {
    st(0xC1C834, ld(0xC1C830).fmul_m(m(0x85862C)));
}

/// `__init_g_c1c838` (0x8536E0): `[0xC1C838] = [0x858630] / [0x8714B8]`.
#[hook(addr = 0x8536E0, conv = cdecl, name = "__init_g_c1c838", expect = "D9 05 30 86 85 00")]
pub fn init_g_c1c838() {
    st(0xC1C838, ld(0x858630).fdiv_m(m(0x8714B8)));
}

/// `__init_g_c1c878` (0x8537C0): `[0xC1C878] = [0x871648] * [0x858620]`.
#[hook(addr = 0x8537C0, conv = cdecl, name = "__init_g_c1c878", expect = "D9 05 48 16 87 00")]
pub fn init_g_c1c878() {
    st(0xC1C878, ld(0x871648).fmul_m(m(0x858620)));
}

/// `__init_g_c1c87c` (0x8537E0): `[0xC1C87C] = [0x858624] / [0x871650]`.
#[hook(addr = 0x8537E0, conv = cdecl, name = "__init_g_c1c87c", expect = "D9 05 24 86 85 00")]
pub fn init_g_c1c87c() {
    st(0xC1C87C, ld(0x858624).fdiv_m(m(0x871650)));
}

/// `__init_g_c1c880` (0x853800): `[0xC1C880] = [0x858628] / [0x87164C]`.
#[hook(addr = 0x853800, conv = cdecl, name = "__init_g_c1c880", expect = "D9 05 28 86 85 00")]
pub fn init_g_c1c880() {
    st(0xC1C880, ld(0x858628).fdiv_m(m(0x87164C)));
}

/// `__init_g_c1c888` (0x853840): `[0xC1C888] = [0xC1C884] * [0x85862C]`.
#[hook(addr = 0x853840, conv = cdecl, name = "__init_g_c1c888", expect = "D9 05 84 C8 C1 00")]
pub fn init_g_c1c888() {
    st(0xC1C888, ld(0xC1C884).fmul_m(m(0x85862C)));
}

/// `__init_g_c1c88c` (0x853860): `[0xC1C88C] = [0x858630] / [0x871650]`.
#[hook(addr = 0x853860, conv = cdecl, name = "__init_g_c1c88c", expect = "D9 05 30 86 85 00")]
pub fn init_g_c1c88c() {
    st(0xC1C88C, ld(0x858630).fdiv_m(m(0x871650)));
}

/// `__init_g_c1c974` (0x8538A0): `[0xC1C974] = [0x871668] * [0x858620]`.
#[hook(addr = 0x8538A0, conv = cdecl, name = "__init_g_c1c974", expect = "D9 05 68 16 87 00")]
pub fn init_g_c1c974() {
    st(0xC1C974, ld(0x871668).fmul_m(m(0x858620)));
}

/// `__init_g_c1c978` (0x8538C0): `[0xC1C978] = [0x858624] / [0x871670]`.
#[hook(addr = 0x8538C0, conv = cdecl, name = "__init_g_c1c978", expect = "D9 05 24 86 85 00")]
pub fn init_g_c1c978() {
    st(0xC1C978, ld(0x858624).fdiv_m(m(0x871670)));
}

/// `__init_g_c1c97c` (0x8538E0): `[0xC1C97C] = [0x858628] / [0x87166C]`.
#[hook(addr = 0x8538E0, conv = cdecl, name = "__init_g_c1c97c", expect = "D9 05 28 86 85 00")]
pub fn init_g_c1c97c() {
    st(0xC1C97C, ld(0x858628).fdiv_m(m(0x87166C)));
}

/// `__init_g_c1c984` (0x853920): `[0xC1C984] = [0xC1C980] * [0x85862C]`.
#[hook(addr = 0x853920, conv = cdecl, name = "__init_g_c1c984", expect = "D9 05 80 C9 C1 00")]
pub fn init_g_c1c984() {
    st(0xC1C984, ld(0xC1C980).fmul_m(m(0x85862C)));
}

/// `__init_g_c1c988` (0x853940): `[0xC1C988] = [0x858630] / [0x871670]`.
#[hook(addr = 0x853940, conv = cdecl, name = "__init_g_c1c988", expect = "D9 05 30 86 85 00")]
pub fn init_g_c1c988() {
    st(0xC1C988, ld(0x858630).fdiv_m(m(0x871670)));
}

/// `__init_g_c1cac0` (0x853960): `[0xC1CAC0] = [0x8717C8] * [0x858620]`.
#[hook(addr = 0x853960, conv = cdecl, name = "__init_g_c1cac0", expect = "D9 05 C8 17 87 00")]
pub fn init_g_c1cac0() {
    st(0xC1CAC0, ld(0x8717C8).fmul_m(m(0x858620)));
}

/// `__init_g_c1cac4` (0x853980): `[0xC1CAC4] = [0x858624] / [0x8717D0]`.
#[hook(addr = 0x853980, conv = cdecl, name = "__init_g_c1cac4", expect = "D9 05 24 86 85 00")]
pub fn init_g_c1cac4() {
    st(0xC1CAC4, ld(0x858624).fdiv_m(m(0x8717D0)));
}

/// `__init_g_c1cac8` (0x8539A0): `[0xC1CAC8] = [0x858628] / [0x8717CC]`.
#[hook(addr = 0x8539A0, conv = cdecl, name = "__init_g_c1cac8", expect = "D9 05 28 86 85 00")]
pub fn init_g_c1cac8() {
    st(0xC1CAC8, ld(0x858628).fdiv_m(m(0x8717CC)));
}

/// `__init_g_c1cad0` (0x8539E0): `[0xC1CAD0] = [0xC1CACC] * [0x85862C]`.
#[hook(addr = 0x8539E0, conv = cdecl, name = "__init_g_c1cad0", expect = "D9 05 CC CA C1 00")]
pub fn init_g_c1cad0() {
    st(0xC1CAD0, ld(0xC1CACC).fmul_m(m(0x85862C)));
}

/// `__init_g_c1cad4` (0x853A00): `[0xC1CAD4] = [0x858630] / [0x8717D0]`.
#[hook(addr = 0x853A00, conv = cdecl, name = "__init_g_c1cad4", expect = "D9 05 30 86 85 00")]
pub fn init_g_c1cad4() {
    st(0xC1CAD4, ld(0x858630).fdiv_m(m(0x8717D0)));
}

/// `__init_g_c1cae4` (0x853A20): `[0xC1CAE4] = [0x8718F4] * [0x858620]`.
#[hook(addr = 0x853A20, conv = cdecl, name = "__init_g_c1cae4", expect = "D9 05 F4 18 87 00")]
pub fn init_g_c1cae4() {
    st(0xC1CAE4, ld(0x8718F4).fmul_m(m(0x858620)));
}

/// `__init_g_c1cae8` (0x853A40): `[0xC1CAE8] = [0x858624] / [0x8718FC]`.
#[hook(addr = 0x853A40, conv = cdecl, name = "__init_g_c1cae8", expect = "D9 05 24 86 85 00")]
pub fn init_g_c1cae8() {
    st(0xC1CAE8, ld(0x858624).fdiv_m(m(0x8718FC)));
}

/// `__init_g_c1caec` (0x853A60): `[0xC1CAEC] = [0x858628] / [0x8718F8]`.
#[hook(addr = 0x853A60, conv = cdecl, name = "__init_g_c1caec", expect = "D9 05 28 86 85 00")]
pub fn init_g_c1caec() {
    st(0xC1CAEC, ld(0x858628).fdiv_m(m(0x8718F8)));
}

/// `__init_g_c1caf4` (0x853AA0): `[0xC1CAF4] = [0xC1CAF0] * [0x85862C]`.
#[hook(addr = 0x853AA0, conv = cdecl, name = "__init_g_c1caf4", expect = "D9 05 F0 CA C1 00")]
pub fn init_g_c1caf4() {
    st(0xC1CAF4, ld(0xC1CAF0).fmul_m(m(0x85862C)));
}

/// `__init_g_c1caf8` (0x853AC0): `[0xC1CAF8] = [0x858630] / [0x8718FC]`.
#[hook(addr = 0x853AC0, conv = cdecl, name = "__init_g_c1caf8", expect = "D9 05 30 86 85 00")]
pub fn init_g_c1caf8() {
    st(0xC1CAF8, ld(0x858630).fdiv_m(m(0x8718FC)));
}

/// `__init_g_c1cb00` (0x853AE0): `[0xC1CB00] = [0x871AB0] * [0x858620]`.
#[hook(addr = 0x853AE0, conv = cdecl, name = "__init_g_c1cb00", expect = "D9 05 B0 1A 87 00")]
pub fn init_g_c1cb00() {
    st(0xC1CB00, ld(0x871AB0).fmul_m(m(0x858620)));
}

/// `__init_g_c1cb04` (0x853B00): `[0xC1CB04] = [0x858624] / [0x871AB8]`.
#[hook(addr = 0x853B00, conv = cdecl, name = "__init_g_c1cb04", expect = "D9 05 24 86 85 00")]
pub fn init_g_c1cb04() {
    st(0xC1CB04, ld(0x858624).fdiv_m(m(0x871AB8)));
}

/// `__init_g_c1cb08` (0x853B20): `[0xC1CB08] = [0x858628] / [0x871AB4]`.
#[hook(addr = 0x853B20, conv = cdecl, name = "__init_g_c1cb08", expect = "D9 05 28 86 85 00")]
pub fn init_g_c1cb08() {
    st(0xC1CB08, ld(0x858628).fdiv_m(m(0x871AB4)));
}

/// `__init_g_c1cb10` (0x853B60): `[0xC1CB10] = [0xC1CB0C] * [0x85862C]`.
#[hook(addr = 0x853B60, conv = cdecl, name = "__init_g_c1cb10", expect = "D9 05 0C CB C1 00")]
pub fn init_g_c1cb10() {
    st(0xC1CB10, ld(0xC1CB0C).fmul_m(m(0x85862C)));
}

/// `__init_g_c1cb14` (0x853B80): `[0xC1CB14] = [0x858630] / [0x871AB8]`.
#[hook(addr = 0x853B80, conv = cdecl, name = "__init_g_c1cb14", expect = "D9 05 30 86 85 00")]
pub fn init_g_c1cb14() {
    st(0xC1CB14, ld(0x858630).fdiv_m(m(0x871AB8)));
}

/// `__init_g_c1cb18` (0x853BA0): `[0xC1CB18] = [0x871C04] * [0x858620]`.
#[hook(addr = 0x853BA0, conv = cdecl, name = "__init_g_c1cb18", expect = "D9 05 04 1C 87 00")]
pub fn init_g_c1cb18() {
    st(0xC1CB18, ld(0x871C04).fmul_m(m(0x858620)));
}

/// `__init_g_c1cb1c` (0x853BC0): `[0xC1CB1C] = [0x858624] / [0x871C0C]`.
#[hook(addr = 0x853BC0, conv = cdecl, name = "__init_g_c1cb1c", expect = "D9 05 24 86 85 00")]
pub fn init_g_c1cb1c() {
    st(0xC1CB1C, ld(0x858624).fdiv_m(m(0x871C0C)));
}

/// `__init_g_c1cb20` (0x853BE0): `[0xC1CB20] = [0x858628] / [0x871C08]`.
#[hook(addr = 0x853BE0, conv = cdecl, name = "__init_g_c1cb20", expect = "D9 05 28 86 85 00")]
pub fn init_g_c1cb20() {
    st(0xC1CB20, ld(0x858628).fdiv_m(m(0x871C08)));
}

/// `__init_g_c1cb2c` (0x853C40): `[0xC1CB2C] = [0x858630] / [0x871C0C]`.
#[hook(addr = 0x853C40, conv = cdecl, name = "__init_g_c1cb2c", expect = "D9 05 30 86 85 00")]
pub fn init_g_c1cb2c() {
    st(0xC1CB2C, ld(0x858630).fdiv_m(m(0x871C0C)));
}

/// `__init_g_c1cb44` (0x853C60): `[0xC1CB44] = [0x858624] / [0x871D48]`.
#[hook(addr = 0x853C60, conv = cdecl, name = "__init_g_c1cb44", expect = "D9 05 24 86 85 00")]
pub fn init_g_c1cb44() {
    st(0xC1CB44, ld(0x858624).fdiv_m(m(0x871D48)));
}

/// `__init_g_c1cb48` (0x853C80): `[0xC1CB48] = [0x858628] / [0x871D44]`.
#[hook(addr = 0x853C80, conv = cdecl, name = "__init_g_c1cb48", expect = "D9 05 28 86 85 00")]
pub fn init_g_c1cb48() {
    st(0xC1CB48, ld(0x858628).fdiv_m(m(0x871D44)));
}

/// `__init_g_c1cb50` (0x853CC0): `[0xC1CB50] = [0xC1CB4C] * [0x85862C]`.
#[hook(addr = 0x853CC0, conv = cdecl, name = "__init_g_c1cb50", expect = "D9 05 4C CB C1 00")]
pub fn init_g_c1cb50() {
    st(0xC1CB50, ld(0xC1CB4C).fmul_m(m(0x85862C)));
}

/// `__init_g_c1cc5c` (0x853D00): `[0xC1CC5C] = [0x871D88] * [0x858620]`.
#[hook(addr = 0x853D00, conv = cdecl, name = "__init_g_c1cc5c", expect = "D9 05 88 1D 87 00")]
pub fn init_g_c1cc5c() {
    st(0xC1CC5C, ld(0x871D88).fmul_m(m(0x858620)));
}

/// `__init_g_c1cc60` (0x853D20): `[0xC1CC60] = [0x858624] / [0x871D90]`.
#[hook(addr = 0x853D20, conv = cdecl, name = "__init_g_c1cc60", expect = "D9 05 24 86 85 00")]
pub fn init_g_c1cc60() {
    st(0xC1CC60, ld(0x858624).fdiv_m(m(0x871D90)));
}

/// `__init_g_c1cc64` (0x853D40): `[0xC1CC64] = [0x858628] / [0x871D8C]`.
#[hook(addr = 0x853D40, conv = cdecl, name = "__init_g_c1cc64", expect = "D9 05 28 86 85 00")]
pub fn init_g_c1cc64() {
    st(0xC1CC64, ld(0x858628).fdiv_m(m(0x871D8C)));
}

/// `__init_g_c1cc6c` (0x853D80): `[0xC1CC6C] = [0xC1CC68] * [0x85862C]`.
#[hook(addr = 0x853D80, conv = cdecl, name = "__init_g_c1cc6c", expect = "D9 05 68 CC C1 00")]
pub fn init_g_c1cc6c() {
    st(0xC1CC6C, ld(0xC1CC68).fmul_m(m(0x85862C)));
}

/// `__init_g_c1cc70` (0x853DA0): `[0xC1CC70] = [0x858630] / [0x871D90]`.
#[hook(addr = 0x853DA0, conv = cdecl, name = "__init_g_c1cc70", expect = "D9 05 30 86 85 00")]
pub fn init_g_c1cc70() {
    st(0xC1CC70, ld(0x858630).fdiv_m(m(0x871D90)));
}

/// `__init_g_c228f4` (0x853F30): `[0xC228F4] = [0x871FEC] * [0x858620]`.
#[hook(addr = 0x853F30, conv = cdecl, name = "__init_g_c228f4", expect = "D9 05 EC 1F 87 00")]
pub fn init_g_c228f4() {
    st(0xC228F4, ld(0x871FEC).fmul_m(m(0x858620)));
}

/// `__init_g_c228f8` (0x853F50): `[0xC228F8] = [0x858624] / [0x871FF4]`.
#[hook(addr = 0x853F50, conv = cdecl, name = "__init_g_c228f8", expect = "D9 05 24 86 85 00")]
pub fn init_g_c228f8() {
    st(0xC228F8, ld(0x858624).fdiv_m(m(0x871FF4)));
}

/// `__init_g_c228fc` (0x853F70): `[0xC228FC] = [0x858628] / [0x871FF0]`.
#[hook(addr = 0x853F70, conv = cdecl, name = "__init_g_c228fc", expect = "D9 05 28 86 85 00")]
pub fn init_g_c228fc() {
    st(0xC228FC, ld(0x858628).fdiv_m(m(0x871FF0)));
}

/// `__init_g_c22904` (0x853FB0): `[0xC22904] = [0xC22900] * [0x85862C]`.
#[hook(addr = 0x853FB0, conv = cdecl, name = "__init_g_c22904", expect = "D9 05 00 29 C2 00")]
pub fn init_g_c22904() {
    st(0xC22904, ld(0xC22900).fmul_m(m(0x85862C)));
}

/// `__init_g_c22908` (0x853FD0): `[0xC22908] = [0x858630] / [0x871FF4]`.
#[hook(addr = 0x853FD0, conv = cdecl, name = "__init_g_c22908", expect = "D9 05 30 86 85 00")]
pub fn init_g_c22908() {
    st(0xC22908, ld(0x858630).fdiv_m(m(0x871FF4)));
}

/// `__init_g_c2b978` (0x854190): `[0xC2B978] = [0x8722EC] * [0x858620]`.
#[hook(addr = 0x854190, conv = cdecl, name = "__init_g_c2b978", expect = "D9 05 EC 22 87 00")]
pub fn init_g_c2b978() {
    st(0xC2B978, ld(0x8722EC).fmul_m(m(0x858620)));
}

/// `__init_g_c2b97c` (0x8541B0): `[0xC2B97C] = [0x858624] / [0x8722F4]`.
#[hook(addr = 0x8541B0, conv = cdecl, name = "__init_g_c2b97c", expect = "D9 05 24 86 85 00")]
pub fn init_g_c2b97c() {
    st(0xC2B97C, ld(0x858624).fdiv_m(m(0x8722F4)));
}

/// `__init_g_c2b980` (0x8541D0): `[0xC2B980] = [0x858628] / [0x8722F0]`.
#[hook(addr = 0x8541D0, conv = cdecl, name = "__init_g_c2b980", expect = "D9 05 28 86 85 00")]
pub fn init_g_c2b980() {
    st(0xC2B980, ld(0x858628).fdiv_m(m(0x8722F0)));
}

/// `__init_g_c2b988` (0x854210): `[0xC2B988] = [0xC2B984] * [0x85862C]`.
#[hook(addr = 0x854210, conv = cdecl, name = "__init_g_c2b988", expect = "D9 05 84 B9 C2 00")]
pub fn init_g_c2b988() {
    st(0xC2B988, ld(0xC2B984).fmul_m(m(0x85862C)));
}

/// `__init_g_c2b98c` (0x854230): `[0xC2B98C] = [0x858630] / [0x8722F4]`.
#[hook(addr = 0x854230, conv = cdecl, name = "__init_g_c2b98c", expect = "D9 05 30 86 85 00")]
pub fn init_g_c2b98c() {
    st(0xC2B98C, ld(0x858630).fdiv_m(m(0x8722F4)));
}

/// `__init_g_c2b990` (0x854250): `[0xC2B990] = [0x8722F8] * [0x858620]`.
#[hook(addr = 0x854250, conv = cdecl, name = "__init_g_c2b990", expect = "D9 05 F8 22 87 00")]
pub fn init_g_c2b990() {
    st(0xC2B990, ld(0x8722F8).fmul_m(m(0x858620)));
}

/// `__init_g_c2b994` (0x854270): `[0xC2B994] = [0x858624] / [0x872300]`.
#[hook(addr = 0x854270, conv = cdecl, name = "__init_g_c2b994", expect = "D9 05 24 86 85 00")]
pub fn init_g_c2b994() {
    st(0xC2B994, ld(0x858624).fdiv_m(m(0x872300)));
}

/// `__init_g_c2b998` (0x854290): `[0xC2B998] = [0x858628] / [0x8722FC]`.
#[hook(addr = 0x854290, conv = cdecl, name = "__init_g_c2b998", expect = "D9 05 28 86 85 00")]
pub fn init_g_c2b998() {
    st(0xC2B998, ld(0x858628).fdiv_m(m(0x8722FC)));
}

/// `__init_g_c2b9a0` (0x8542D0): `[0xC2B9A0] = [0xC2B99C] * [0x85862C]`.
#[hook(addr = 0x8542D0, conv = cdecl, name = "__init_g_c2b9a0", expect = "D9 05 9C B9 C2 00")]
pub fn init_g_c2b9a0() {
    st(0xC2B9A0, ld(0xC2B99C).fmul_m(m(0x85862C)));
}

/// `__init_g_c2b9a4` (0x8542F0): `[0xC2B9A4] = [0x858630] / [0x872300]`.
#[hook(addr = 0x8542F0, conv = cdecl, name = "__init_g_c2b9a4", expect = "D9 05 30 86 85 00")]
pub fn init_g_c2b9a4() {
    st(0xC2B9A4, ld(0x858630).fdiv_m(m(0x872300)));
}

/// `__init_g_c2b9ac` (0x854310): `[0xC2B9AC] = [0x858624] / [0x872344]`.
#[hook(addr = 0x854310, conv = cdecl, name = "__init_g_c2b9ac", expect = "D9 05 24 86 85 00")]
pub fn init_g_c2b9ac() {
    st(0xC2B9AC, ld(0x858624).fdiv_m(m(0x872344)));
}

/// `__init_g_c2b9b0` (0x854330): `[0xC2B9B0] = [0x858628] / [0x872340]`.
#[hook(addr = 0x854330, conv = cdecl, name = "__init_g_c2b9b0", expect = "D9 05 28 86 85 00")]
pub fn init_g_c2b9b0() {
    st(0xC2B9B0, ld(0x858628).fdiv_m(m(0x872340)));
}

/// `__init_g_c2b9b8` (0x854370): `[0xC2B9B8] = [0xC2B9B4] * [0x85862C]`.
#[hook(addr = 0x854370, conv = cdecl, name = "__init_g_c2b9b8", expect = "D9 05 B4 B9 C2 00")]
pub fn init_g_c2b9b8() {
    st(0xC2B9B8, ld(0xC2B9B4).fmul_m(m(0x85862C)));
}

/// `__init_g_c2b9c0` (0x8543B0): `[0xC2B9C0] = [0x87234C] * [0x858620]`.
#[hook(addr = 0x8543B0, conv = cdecl, name = "__init_g_c2b9c0", expect = "D9 05 4C 23 87 00")]
pub fn init_g_c2b9c0() {
    st(0xC2B9C0, ld(0x87234C).fmul_m(m(0x858620)));
}

/// `__init_g_97d654` (0x84A2A0): `[0x97D654] = [0x858624] / ([0x859B00] * [0x859B00])`.
#[hook(addr = 0x84A2A0, conv = cdecl, name = "__init_g_97d654", expect = "D9 05 00 9B 85 00")]
pub fn init_g_97d654() {
    st(0x97D654, ld(0x859B00).fmul_m(m(0x859B00)).fdivr_m(m(0x858624)));
}

/// `__init_g_97f640` (0x84A360): `[0x97F640] = [0x858624] / ([0x859BC8] * [0x859BC8])`.
#[hook(addr = 0x84A360, conv = cdecl, name = "__init_g_97f640", expect = "D9 05 C8 9B 85 00")]
pub fn init_g_97f640() {
    st(0x97F640, ld(0x859BC8).fmul_m(m(0x859BC8)).fdivr_m(m(0x858624)));
}

/// `__init_g_c18c14` (0x852260): `[0xC18C14] = [0x858624] / ([0x86EC2C] * [0x86EC2C])`.
#[hook(addr = 0x852260, conv = cdecl, name = "__init_g_c18c14", expect = "D9 05 2C EC 86 00")]
pub fn init_g_c18c14() {
    st(0xC18C14, ld(0x86EC2C).fmul_m(m(0x86EC2C)).fdivr_m(m(0x858624)));
}

/// `__init_g_c18c30` (0x852320): `[0xC18C30] = [0x858624] / ([0x86F0F4] * [0x86F0F4])`.
#[hook(addr = 0x852320, conv = cdecl, name = "__init_g_c18c30", expect = "D9 05 F4 F0 86 00")]
pub fn init_g_c18c30() {
    st(0xC18C30, ld(0x86F0F4).fmul_m(m(0x86F0F4)).fdivr_m(m(0x858624)));
}

/// `__init_g_c1b334` (0x853400): `[0xC1B334] = [0x858624] / ([0x870F54] * [0x870F54])`.
#[hook(addr = 0x853400, conv = cdecl, name = "__init_g_c1b334", expect = "D9 05 54 0F 87 00")]
pub fn init_g_c1b334() {
    st(0xC1B334, ld(0x870F54).fmul_m(m(0x870F54)).fdivr_m(m(0x858624)));
}

/// `__init_g_c1bfe8` (0x8534E0): `[0xC1BFE8] = [0x858624] / ([0x870FC8] * [0x870FC8])`.
#[hook(addr = 0x8534E0, conv = cdecl, name = "__init_g_c1bfe8", expect = "D9 05 C8 0F 87 00")]
pub fn init_g_c1bfe8() {
    st(0xC1BFE8, ld(0x870FC8).fmul_m(m(0x870FC8)).fdivr_m(m(0x858624)));
}

/// `__init_g_c1c814` (0x8535E0): `[0xC1C814] = [0x858624] / ([0x87127C] * [0x87127C])`.
#[hook(addr = 0x8535E0, conv = cdecl, name = "__init_g_c1c814", expect = "D9 05 7C 12 87 00")]
pub fn init_g_c1c814() {
    st(0xC1C814, ld(0x87127C).fmul_m(m(0x87127C)).fdivr_m(m(0x858624)));
}

/// `__init_g_c1c830` (0x8536A0): `[0xC1C830] = [0x858624] / ([0x8714B8] * [0x8714B8])`.
#[hook(addr = 0x8536A0, conv = cdecl, name = "__init_g_c1c830", expect = "D9 05 B8 14 87 00")]
pub fn init_g_c1c830() {
    st(0xC1C830, ld(0x8714B8).fmul_m(m(0x8714B8)).fdivr_m(m(0x858624)));
}

/// `__init_g_c1c884` (0x853820): `[0xC1C884] = [0x858624] / ([0x871650] * [0x871650])`.
#[hook(addr = 0x853820, conv = cdecl, name = "__init_g_c1c884", expect = "D9 05 50 16 87 00")]
pub fn init_g_c1c884() {
    st(0xC1C884, ld(0x871650).fmul_m(m(0x871650)).fdivr_m(m(0x858624)));
}

/// `__init_g_c1c980` (0x853900): `[0xC1C980] = [0x858624] / ([0x871670] * [0x871670])`.
#[hook(addr = 0x853900, conv = cdecl, name = "__init_g_c1c980", expect = "D9 05 70 16 87 00")]
pub fn init_g_c1c980() {
    st(0xC1C980, ld(0x871670).fmul_m(m(0x871670)).fdivr_m(m(0x858624)));
}

/// `__init_g_c1cacc` (0x8539C0): `[0xC1CACC] = [0x858624] / ([0x8717D0] * [0x8717D0])`.
#[hook(addr = 0x8539C0, conv = cdecl, name = "__init_g_c1cacc", expect = "D9 05 D0 17 87 00")]
pub fn init_g_c1cacc() {
    st(0xC1CACC, ld(0x8717D0).fmul_m(m(0x8717D0)).fdivr_m(m(0x858624)));
}

/// `__init_g_c1caf0` (0x853A80): `[0xC1CAF0] = [0x858624] / ([0x8718FC] * [0x8718FC])`.
#[hook(addr = 0x853A80, conv = cdecl, name = "__init_g_c1caf0", expect = "D9 05 FC 18 87 00")]
pub fn init_g_c1caf0() {
    st(0xC1CAF0, ld(0x8718FC).fmul_m(m(0x8718FC)).fdivr_m(m(0x858624)));
}

/// `__init_g_c1cb0c` (0x853B40): `[0xC1CB0C] = [0x858624] / ([0x871AB8] * [0x871AB8])`.
#[hook(addr = 0x853B40, conv = cdecl, name = "__init_g_c1cb0c", expect = "D9 05 B8 1A 87 00")]
pub fn init_g_c1cb0c() {
    st(0xC1CB0C, ld(0x871AB8).fmul_m(m(0x871AB8)).fdivr_m(m(0x858624)));
}

/// `__init_g_c1cb24` (0x853C00): `[0xC1CB24] = [0x858624] / ([0x871C0C] * [0x871C0C])`.
#[hook(addr = 0x853C00, conv = cdecl, name = "__init_g_c1cb24", expect = "D9 05 0C 1C 87 00")]
pub fn init_g_c1cb24() {
    st(0xC1CB24, ld(0x871C0C).fmul_m(m(0x871C0C)).fdivr_m(m(0x858624)));
}

/// `__init_g_c1cb4c` (0x853CA0): `[0xC1CB4C] = [0x858624] / ([0x871D48] * [0x871D48])`.
#[hook(addr = 0x853CA0, conv = cdecl, name = "__init_g_c1cb4c", expect = "D9 05 48 1D 87 00")]
pub fn init_g_c1cb4c() {
    st(0xC1CB4C, ld(0x871D48).fmul_m(m(0x871D48)).fdivr_m(m(0x858624)));
}

/// `__init_g_c1cc68` (0x853D60): `[0xC1CC68] = [0x858624] / ([0x871D90] * [0x871D90])`.
#[hook(addr = 0x853D60, conv = cdecl, name = "__init_g_c1cc68", expect = "D9 05 90 1D 87 00")]
pub fn init_g_c1cc68() {
    st(0xC1CC68, ld(0x871D90).fmul_m(m(0x871D90)).fdivr_m(m(0x858624)));
}

/// `__init_g_c22900` (0x853F90): `[0xC22900] = [0x858624] / ([0x871FF4] * [0x871FF4])`.
#[hook(addr = 0x853F90, conv = cdecl, name = "__init_g_c22900", expect = "D9 05 F4 1F 87 00")]
pub fn init_g_c22900() {
    st(0xC22900, ld(0x871FF4).fmul_m(m(0x871FF4)).fdivr_m(m(0x858624)));
}

/// `__init_g_c2b984` (0x8541F0): `[0xC2B984] = [0x858624] / ([0x8722F4] * [0x8722F4])`.
#[hook(addr = 0x8541F0, conv = cdecl, name = "__init_g_c2b984", expect = "D9 05 F4 22 87 00")]
pub fn init_g_c2b984() {
    st(0xC2B984, ld(0x8722F4).fmul_m(m(0x8722F4)).fdivr_m(m(0x858624)));
}

/// `__init_g_c2b99c` (0x8542B0): `[0xC2B99C] = [0x858624] / ([0x872300] * [0x872300])`.
#[hook(addr = 0x8542B0, conv = cdecl, name = "__init_g_c2b99c", expect = "D9 05 00 23 87 00")]
pub fn init_g_c2b99c() {
    st(0xC2B99C, ld(0x872300).fmul_m(m(0x872300)).fdivr_m(m(0x858624)));
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_G_97D648,
    HOOK_INIT_G_97D658,
    HOOK_INIT_G_97D65C,
    HOOK_INIT_G_97F638,
    HOOK_INIT_G_97F644,
    HOOK_INIT_G_97F648,
    HOOK_INIT_G_C18C08,
    HOOK_INIT_G_C18C0C,
    HOOK_INIT_G_C18C10,
    HOOK_INIT_G_C18C18,
    HOOK_INIT_G_C18C1C,
    HOOK_INIT_G_C18C24,
    HOOK_INIT_G_C18C28,
    HOOK_INIT_G_C18C2C,
    HOOK_INIT_G_C18C34,
    HOOK_INIT_G_C18C38,
    HOOK_INIT_G_C1AEB0,
    HOOK_INIT_G_C1B328,
    HOOK_INIT_G_C1B32C,
    HOOK_INIT_G_C1B330,
    HOOK_INIT_G_C1B338,
    HOOK_INIT_G_C1B33C,
    HOOK_INIT_G_C1BFDC,
    HOOK_INIT_G_C1BFE0,
    HOOK_INIT_G_C1BFE4,
    HOOK_INIT_G_C1BFF0,
    HOOK_INIT_G_C1C808,
    HOOK_INIT_G_C1C80C,
    HOOK_INIT_G_C1C810,
    HOOK_INIT_G_C1C824,
    HOOK_INIT_G_C1C828,
    HOOK_INIT_G_C1C82C,
    HOOK_INIT_G_C1C834,
    HOOK_INIT_G_C1C838,
    HOOK_INIT_G_C1C878,
    HOOK_INIT_G_C1C87C,
    HOOK_INIT_G_C1C880,
    HOOK_INIT_G_C1C888,
    HOOK_INIT_G_C1C88C,
    HOOK_INIT_G_C1C974,
    HOOK_INIT_G_C1C978,
    HOOK_INIT_G_C1C97C,
    HOOK_INIT_G_C1C984,
    HOOK_INIT_G_C1C988,
    HOOK_INIT_G_C1CAC0,
    HOOK_INIT_G_C1CAC4,
    HOOK_INIT_G_C1CAC8,
    HOOK_INIT_G_C1CAD0,
    HOOK_INIT_G_C1CAD4,
    HOOK_INIT_G_C1CAE4,
    HOOK_INIT_G_C1CAE8,
    HOOK_INIT_G_C1CAEC,
    HOOK_INIT_G_C1CAF4,
    HOOK_INIT_G_C1CAF8,
    HOOK_INIT_G_C1CB00,
    HOOK_INIT_G_C1CB04,
    HOOK_INIT_G_C1CB08,
    HOOK_INIT_G_C1CB10,
    HOOK_INIT_G_C1CB14,
    HOOK_INIT_G_C1CB18,
    HOOK_INIT_G_C1CB1C,
    HOOK_INIT_G_C1CB20,
    HOOK_INIT_G_C1CB2C,
    HOOK_INIT_G_C1CB44,
    HOOK_INIT_G_C1CB48,
    HOOK_INIT_G_C1CB50,
    HOOK_INIT_G_C1CC5C,
    HOOK_INIT_G_C1CC60,
    HOOK_INIT_G_C1CC64,
    HOOK_INIT_G_C1CC6C,
    HOOK_INIT_G_C1CC70,
    HOOK_INIT_G_C228F4,
    HOOK_INIT_G_C228F8,
    HOOK_INIT_G_C228FC,
    HOOK_INIT_G_C22904,
    HOOK_INIT_G_C22908,
    HOOK_INIT_G_C2B978,
    HOOK_INIT_G_C2B97C,
    HOOK_INIT_G_C2B980,
    HOOK_INIT_G_C2B988,
    HOOK_INIT_G_C2B98C,
    HOOK_INIT_G_C2B990,
    HOOK_INIT_G_C2B994,
    HOOK_INIT_G_C2B998,
    HOOK_INIT_G_C2B9A0,
    HOOK_INIT_G_C2B9A4,
    HOOK_INIT_G_C2B9AC,
    HOOK_INIT_G_C2B9B0,
    HOOK_INIT_G_C2B9B8,
    HOOK_INIT_G_C2B9C0,
    HOOK_INIT_G_97D654,
    HOOK_INIT_G_97F640,
    HOOK_INIT_G_C18C14,
    HOOK_INIT_G_C18C30,
    HOOK_INIT_G_C1B334,
    HOOK_INIT_G_C1BFE8,
    HOOK_INIT_G_C1C814,
    HOOK_INIT_G_C1C830,
    HOOK_INIT_G_C1C884,
    HOOK_INIT_G_C1C980,
    HOOK_INIT_G_C1CACC,
    HOOK_INIT_G_C1CAF0,
    HOOK_INIT_G_C1CB0C,
    HOOK_INIT_G_C1CB24,
    HOOK_INIT_G_C1CB4C,
    HOOK_INIT_G_C1CC68,
    HOOK_INIT_G_C22900,
    HOOK_INIT_G_C2B984,
    HOOK_INIT_G_C2B99C,
];

/// This module's globals (the destinations are file statics owned by other VEH sources; written through `g`).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
