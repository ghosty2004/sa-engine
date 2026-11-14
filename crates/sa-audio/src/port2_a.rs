//! PORT-AUDIO-2 hand ports, helper A (see `docs/arch/audio-port.md`, PORT-AUDIO-2).
//! Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/AUDIO/*.md` are reading aids (the exe wins).
//!
//! AUDIO static initialisers: the `__init_g_*` functions compute derived float constants
//! (`fld m32; f<op> m32; fstp m32`). They run before D3D sets precision control 24, i.e. under the CRT's
//! PC=53. For one operation on f32 operands the double rounding (to 53, then to 24 bits on `fstp m32`)
//! equals the single PC=24 rounding `X87` models (53 >= 2*24+2). The `fld; fmul; fdivr; fstp` ones keep
//! the product in ST0: under PC=53 it is exact (48 significant bits), the quotient is rounded to 53 bits
//! and then to 24 by the store, which `div_of_product` reproduces with f64 arithmetic.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::{AE_AUDIO_HARDWARE, GPtr, g, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
}

orig! {
    /// CRT `_atexit` (0x821D1E)
    fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
}

/// `n / (a * b)` as `fld a; fmul b; fdivr n; fstp m32` computes it under PC=53: the f32 * f32 product is
/// exact in a double, the division rounds to 53 bits and the store to 24 bits.
fn div_of_product(n: f32, a: f32, b: f32) -> f32 {
    let p = f64::from(a) * f64::from(b);
    (f64::from(n) / p) as f32
}

// ---------------------------------------------------------------------------------------------------
// AUDIO static initialisers
// ---------------------------------------------------------------------------------------------------

/// `__init_g_b5f880` (0x84C0E0): `[0xB5F880] = [0x85EEC0] * [0x858620]`.
#[hook(addr = 0x84C0E0, conv = cdecl, name = "__init_g_b5f880", expect = "D9 05 C0 EE 85 00")]
pub fn init_g_b5f880() {
    let v = x(g::<f32>(0x85EEC0).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB5F880).write(v.to_f32());
}

/// `__init_g_b5f884` (0x84C100): `[0xB5F884] = [0x858624] / [0x85EEC8]`.
#[hook(addr = 0x84C100, conv = cdecl, name = "__init_g_b5f884", expect = "D9 05 24 86 85 00")]
pub fn init_g_b5f884() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x85EEC8).read());
    g::<f32>(0xB5F884).write(v.to_f32());
}

/// `__init_g_b5f888` (0x84C120): `[0xB5F888] = [0x858628] / [0x85EEC4]`.
#[hook(addr = 0x84C120, conv = cdecl, name = "__init_g_b5f888", expect = "D9 05 28 86 85 00")]
pub fn init_g_b5f888() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x85EEC4).read());
    g::<f32>(0xB5F888).write(v.to_f32());
}

/// `__init_g_b5f88c` (0x84C140): `[0xB5F88C] = [0x858624] / ([0x85EEC8] * [0x85EEC8])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84C140, conv = cdecl, name = "__init_g_b5f88c", expect = "D9 05 C8 EE 85 00")]
pub fn init_g_b5f88c() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x85EEC8).read(),
        g::<f32>(0x85EEC8).read(),
    );
    g::<f32>(0xB5F88C).write(v);
}

/// `__init_g_b5f890` (0x84C160): `[0xB5F890] = [0xB5F88C] * [0x85862C]`.
#[hook(addr = 0x84C160, conv = cdecl, name = "__init_g_b5f890", expect = "D9 05 8C F8 B5 00")]
pub fn init_g_b5f890() {
    let v = x(g::<f32>(0xB5F88C).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB5F890).write(v.to_f32());
}

/// `__init_g_b5f894` (0x84C180): `[0xB5F894] = [0x858630] / [0x85EEC8]`.
#[hook(addr = 0x84C180, conv = cdecl, name = "__init_g_b5f894", expect = "D9 05 30 86 85 00")]
pub fn init_g_b5f894() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x85EEC8).read());
    g::<f32>(0xB5F894).write(v.to_f32());
}

/// `__init_84c1a0` (0x84C1A0): `atexit(0x8562E0)`.
#[hook(addr = 0x84C1A0, conv = cdecl, name = "__init_84c1a0", expect = "68 E0 62 85 00")]
pub fn init_84c1a0() {
    atexit(0x8562E0);
}

/// `__init_g_b5f89c` (0x84C1B0): `[0xB5F89C] = [0x85F060] * [0x858620]`.
#[hook(addr = 0x84C1B0, conv = cdecl, name = "__init_g_b5f89c", expect = "D9 05 60 F0 85 00")]
pub fn init_g_b5f89c() {
    let v = x(g::<f32>(0x85F060).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB5F89C).write(v.to_f32());
}

/// `__init_g_b5f8a0` (0x84C1D0): `[0xB5F8A0] = [0x858624] / [0x85F068]`.
#[hook(addr = 0x84C1D0, conv = cdecl, name = "__init_g_b5f8a0", expect = "D9 05 24 86 85 00")]
pub fn init_g_b5f8a0() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x85F068).read());
    g::<f32>(0xB5F8A0).write(v.to_f32());
}

/// `__init_g_b5f8a4` (0x84C1F0): `[0xB5F8A4] = [0x858628] / [0x85F064]`.
#[hook(addr = 0x84C1F0, conv = cdecl, name = "__init_g_b5f8a4", expect = "D9 05 28 86 85 00")]
pub fn init_g_b5f8a4() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x85F064).read());
    g::<f32>(0xB5F8A4).write(v.to_f32());
}

/// `__init_g_b5f8a8` (0x84C210): `[0xB5F8A8] = [0x858624] / ([0x85F068] * [0x85F068])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84C210, conv = cdecl, name = "__init_g_b5f8a8", expect = "D9 05 68 F0 85 00")]
pub fn init_g_b5f8a8() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x85F068).read(),
        g::<f32>(0x85F068).read(),
    );
    g::<f32>(0xB5F8A8).write(v);
}

/// `__init_g_b5f8ac` (0x84C230): `[0xB5F8AC] = [0xB5F8A8] * [0x85862C]`.
#[hook(addr = 0x84C230, conv = cdecl, name = "__init_g_b5f8ac", expect = "D9 05 A8 F8 B5 00")]
pub fn init_g_b5f8ac() {
    let v = x(g::<f32>(0xB5F8A8).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB5F8AC).write(v.to_f32());
}

/// `__init_g_b5f8b0` (0x84C250): `[0xB5F8B0] = [0x858630] / [0x85F068]`.
#[hook(addr = 0x84C250, conv = cdecl, name = "__init_g_b5f8b0", expect = "D9 05 30 86 85 00")]
pub fn init_g_b5f8b0() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x85F068).read());
    g::<f32>(0xB5F8B0).write(v.to_f32());
}

/// `__init_g_b61290` (0x84C2B0): `[0xB61290] = [0x85F084] * [0x858620]`.
#[hook(addr = 0x84C2B0, conv = cdecl, name = "__init_g_b61290", expect = "D9 05 84 F0 85 00")]
pub fn init_g_b61290() {
    let v = x(g::<f32>(0x85F084).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB61290).write(v.to_f32());
}

/// `__init_g_b61294` (0x84C2D0): `[0xB61294] = [0x858624] / [0x85F08C]`.
#[hook(addr = 0x84C2D0, conv = cdecl, name = "__init_g_b61294", expect = "D9 05 24 86 85 00")]
pub fn init_g_b61294() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x85F08C).read());
    g::<f32>(0xB61294).write(v.to_f32());
}

/// `__init_g_b61298` (0x84C2F0): `[0xB61298] = [0x858628] / [0x85F088]`.
#[hook(addr = 0x84C2F0, conv = cdecl, name = "__init_g_b61298", expect = "D9 05 28 86 85 00")]
pub fn init_g_b61298() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x85F088).read());
    g::<f32>(0xB61298).write(v.to_f32());
}

/// `__init_g_b6129c` (0x84C310): `[0xB6129C] = [0x858624] / ([0x85F08C] * [0x85F08C])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84C310, conv = cdecl, name = "__init_g_b6129c", expect = "D9 05 8C F0 85 00")]
pub fn init_g_b6129c() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x85F08C).read(),
        g::<f32>(0x85F08C).read(),
    );
    g::<f32>(0xB6129C).write(v);
}

/// `__init_g_b612a0` (0x84C330): `[0xB612A0] = [0xB6129C] * [0x85862C]`.
#[hook(addr = 0x84C330, conv = cdecl, name = "__init_g_b612a0", expect = "D9 05 9C 12 B6 00")]
pub fn init_g_b612a0() {
    let v = x(g::<f32>(0xB6129C).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB612A0).write(v.to_f32());
}

/// `__init_g_b612a4` (0x84C350): `[0xB612A4] = [0x858630] / [0x85F08C]`.
#[hook(addr = 0x84C350, conv = cdecl, name = "__init_g_b612a4", expect = "D9 05 30 86 85 00")]
pub fn init_g_b612a4() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x85F08C).read());
    g::<f32>(0xB612A4).write(v.to_f32());
}

/// `__init_g_b612a8` (0x84C370): `[0xB612A8] = [0x85F090] * [0x858620]`.
#[hook(addr = 0x84C370, conv = cdecl, name = "__init_g_b612a8", expect = "D9 05 90 F0 85 00")]
pub fn init_g_b612a8() {
    let v = x(g::<f32>(0x85F090).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB612A8).write(v.to_f32());
}

/// `__init_g_b612ac` (0x84C390): `[0xB612AC] = [0x858624] / [0x85F098]`.
#[hook(addr = 0x84C390, conv = cdecl, name = "__init_g_b612ac", expect = "D9 05 24 86 85 00")]
pub fn init_g_b612ac() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x85F098).read());
    g::<f32>(0xB612AC).write(v.to_f32());
}

/// `__init_g_b612b0` (0x84C3B0): `[0xB612B0] = [0x858628] / [0x85F094]`.
#[hook(addr = 0x84C3B0, conv = cdecl, name = "__init_g_b612b0", expect = "D9 05 28 86 85 00")]
pub fn init_g_b612b0() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x85F094).read());
    g::<f32>(0xB612B0).write(v.to_f32());
}

/// `__init_g_b612b4` (0x84C3D0): `[0xB612B4] = [0x858624] / ([0x85F098] * [0x85F098])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84C3D0, conv = cdecl, name = "__init_g_b612b4", expect = "D9 05 98 F0 85 00")]
pub fn init_g_b612b4() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x85F098).read(),
        g::<f32>(0x85F098).read(),
    );
    g::<f32>(0xB612B4).write(v);
}

/// `__init_g_b612b8` (0x84C3F0): `[0xB612B8] = [0xB612B4] * [0x85862C]`.
#[hook(addr = 0x84C3F0, conv = cdecl, name = "__init_g_b612b8", expect = "D9 05 B4 12 B6 00")]
pub fn init_g_b612b8() {
    let v = x(g::<f32>(0xB612B4).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB612B8).write(v.to_f32());
}

/// `__init_g_b612bc` (0x84C410): `[0xB612BC] = [0x858630] / [0x85F098]`.
#[hook(addr = 0x84C410, conv = cdecl, name = "__init_g_b612bc", expect = "D9 05 30 86 85 00")]
pub fn init_g_b612bc() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x85F098).read());
    g::<f32>(0xB612BC).write(v.to_f32());
}

/// `__init_g_b612c0` (0x84C430): `[0xB612C0] = [0x85F0B0] * [0x858620]`.
#[hook(addr = 0x84C430, conv = cdecl, name = "__init_g_b612c0", expect = "D9 05 B0 F0 85 00")]
pub fn init_g_b612c0() {
    let v = x(g::<f32>(0x85F0B0).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB612C0).write(v.to_f32());
}

/// `__init_g_b612c4` (0x84C450): `[0xB612C4] = [0x858624] / [0x85F0B8]`.
#[hook(addr = 0x84C450, conv = cdecl, name = "__init_g_b612c4", expect = "D9 05 24 86 85 00")]
pub fn init_g_b612c4() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x85F0B8).read());
    g::<f32>(0xB612C4).write(v.to_f32());
}

/// `__init_g_b612c8` (0x84C470): `[0xB612C8] = [0x858628] / [0x85F0B4]`.
#[hook(addr = 0x84C470, conv = cdecl, name = "__init_g_b612c8", expect = "D9 05 28 86 85 00")]
pub fn init_g_b612c8() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x85F0B4).read());
    g::<f32>(0xB612C8).write(v.to_f32());
}

/// `__init_g_b612cc` (0x84C490): `[0xB612CC] = [0x858624] / ([0x85F0B8] * [0x85F0B8])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84C490, conv = cdecl, name = "__init_g_b612cc", expect = "D9 05 B8 F0 85 00")]
pub fn init_g_b612cc() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x85F0B8).read(),
        g::<f32>(0x85F0B8).read(),
    );
    g::<f32>(0xB612CC).write(v);
}

/// `__init_g_b612d0` (0x84C4B0): `[0xB612D0] = [0xB612CC] * [0x85862C]`.
#[hook(addr = 0x84C4B0, conv = cdecl, name = "__init_g_b612d0", expect = "D9 05 CC 12 B6 00")]
pub fn init_g_b612d0() {
    let v = x(g::<f32>(0xB612CC).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB612D0).write(v.to_f32());
}

/// `__init_g_b612d4` (0x84C4D0): `[0xB612D4] = [0x858630] / [0x85F0B8]`.
#[hook(addr = 0x84C4D0, conv = cdecl, name = "__init_g_b612d4", expect = "D9 05 30 86 85 00")]
pub fn init_g_b612d4() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x85F0B8).read());
    g::<f32>(0xB612D4).write(v.to_f32());
}

/// `__init_84c4f0` (0x84C4F0): `atexit(0x856310)`.
#[hook(addr = 0x84C4F0, conv = cdecl, name = "__init_84c4f0", expect = "68 10 63 85 00")]
pub fn init_84c4f0() {
    atexit(0x856310);
}

/// `__init_g_b612e8` (0x84C520): `[0xB612E8] = [0x85F0F4] * [0x858620]`.
#[hook(addr = 0x84C520, conv = cdecl, name = "__init_g_b612e8", expect = "D9 05 F4 F0 85 00")]
pub fn init_g_b612e8() {
    let v = x(g::<f32>(0x85F0F4).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB612E8).write(v.to_f32());
}

/// `__init_g_b61300` (0x84C540): `[0xB61300] = [0x85F108] * [0x858620]`.
#[hook(addr = 0x84C540, conv = cdecl, name = "__init_g_b61300", expect = "D9 05 08 F1 85 00")]
pub fn init_g_b61300() {
    let v = x(g::<f32>(0x85F108).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB61300).write(v.to_f32());
}

/// `__init_g_b61304` (0x84C560): `[0xB61304] = [0x858624] / [0x85F110]`.
#[hook(addr = 0x84C560, conv = cdecl, name = "__init_g_b61304", expect = "D9 05 24 86 85 00")]
pub fn init_g_b61304() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x85F110).read());
    g::<f32>(0xB61304).write(v.to_f32());
}

/// `__init_g_b61308` (0x84C580): `[0xB61308] = [0x858628] / [0x85F10C]`.
#[hook(addr = 0x84C580, conv = cdecl, name = "__init_g_b61308", expect = "D9 05 28 86 85 00")]
pub fn init_g_b61308() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x85F10C).read());
    g::<f32>(0xB61308).write(v.to_f32());
}

/// `__init_g_b6130c` (0x84C5A0): `[0xB6130C] = [0x858624] / ([0x85F110] * [0x85F110])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84C5A0, conv = cdecl, name = "__init_g_b6130c", expect = "D9 05 10 F1 85 00")]
pub fn init_g_b6130c() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x85F110).read(),
        g::<f32>(0x85F110).read(),
    );
    g::<f32>(0xB6130C).write(v);
}

/// `__init_g_b61310` (0x84C5C0): `[0xB61310] = [0xB6130C] * [0x85862C]`.
#[hook(addr = 0x84C5C0, conv = cdecl, name = "__init_g_b61310", expect = "D9 05 0C 13 B6 00")]
pub fn init_g_b61310() {
    let v = x(g::<f32>(0xB6130C).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB61310).write(v.to_f32());
}

/// `__init_g_b61314` (0x84C5E0): `[0xB61314] = [0x858630] / [0x85F110]`.
#[hook(addr = 0x84C5E0, conv = cdecl, name = "__init_g_b61314", expect = "D9 05 30 86 85 00")]
pub fn init_g_b61314() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x85F110).read());
    g::<f32>(0xB61314).write(v.to_f32());
}

/// `__init_g_b6136c` (0x84C600): `[0xB6136C] = [0x85F178] * [0x858620]`.
#[hook(addr = 0x84C600, conv = cdecl, name = "__init_g_b6136c", expect = "D9 05 78 F1 85 00")]
pub fn init_g_b6136c() {
    let v = x(g::<f32>(0x85F178).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB6136C).write(v.to_f32());
}

/// `__init_g_b61370` (0x84C620): `[0xB61370] = [0x858624] / [0x85F180]`.
#[hook(addr = 0x84C620, conv = cdecl, name = "__init_g_b61370", expect = "D9 05 24 86 85 00")]
pub fn init_g_b61370() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x85F180).read());
    g::<f32>(0xB61370).write(v.to_f32());
}

/// `__init_g_b61374` (0x84C640): `[0xB61374] = [0x858628] / [0x85F17C]`.
#[hook(addr = 0x84C640, conv = cdecl, name = "__init_g_b61374", expect = "D9 05 28 86 85 00")]
pub fn init_g_b61374() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x85F17C).read());
    g::<f32>(0xB61374).write(v.to_f32());
}

/// `__init_g_b61378` (0x84C660): `[0xB61378] = [0x858624] / ([0x85F180] * [0x85F180])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84C660, conv = cdecl, name = "__init_g_b61378", expect = "D9 05 80 F1 85 00")]
pub fn init_g_b61378() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x85F180).read(),
        g::<f32>(0x85F180).read(),
    );
    g::<f32>(0xB61378).write(v);
}

/// `__init_g_b6137c` (0x84C680): `[0xB6137C] = [0xB61378] * [0x85862C]`.
#[hook(addr = 0x84C680, conv = cdecl, name = "__init_g_b6137c", expect = "D9 05 78 13 B6 00")]
pub fn init_g_b6137c() {
    let v = x(g::<f32>(0xB61378).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB6137C).write(v.to_f32());
}

/// `__init_g_b61380` (0x84C6A0): `[0xB61380] = [0x858630] / [0x85F180]`.
#[hook(addr = 0x84C6A0, conv = cdecl, name = "__init_g_b61380", expect = "D9 05 30 86 85 00")]
pub fn init_g_b61380() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x85F180).read());
    g::<f32>(0xB61380).write(v.to_f32());
}

/// `__init_g_b6138c` (0x84C6C0): `[0xB6138C] = [0x85F1CC] * [0x858620]`.
#[hook(addr = 0x84C6C0, conv = cdecl, name = "__init_g_b6138c", expect = "D9 05 CC F1 85 00")]
pub fn init_g_b6138c() {
    let v = x(g::<f32>(0x85F1CC).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB6138C).write(v.to_f32());
}

/// `__init_g_b61390` (0x84C6E0): `[0xB61390] = [0x858624] / [0x85F1D4]`.
#[hook(addr = 0x84C6E0, conv = cdecl, name = "__init_g_b61390", expect = "D9 05 24 86 85 00")]
pub fn init_g_b61390() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x85F1D4).read());
    g::<f32>(0xB61390).write(v.to_f32());
}

/// `__init_g_b61394` (0x84C700): `[0xB61394] = [0x858628] / [0x85F1D0]`.
#[hook(addr = 0x84C700, conv = cdecl, name = "__init_g_b61394", expect = "D9 05 28 86 85 00")]
pub fn init_g_b61394() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x85F1D0).read());
    g::<f32>(0xB61394).write(v.to_f32());
}

/// `__init_g_b61398` (0x84C720): `[0xB61398] = [0x858624] / ([0x85F1D4] * [0x85F1D4])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84C720, conv = cdecl, name = "__init_g_b61398", expect = "D9 05 D4 F1 85 00")]
pub fn init_g_b61398() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x85F1D4).read(),
        g::<f32>(0x85F1D4).read(),
    );
    g::<f32>(0xB61398).write(v);
}

/// `__init_g_b6139c` (0x84C740): `[0xB6139C] = [0xB61398] * [0x85862C]`.
#[hook(addr = 0x84C740, conv = cdecl, name = "__init_g_b6139c", expect = "D9 05 98 13 B6 00")]
pub fn init_g_b6139c() {
    let v = x(g::<f32>(0xB61398).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB6139C).write(v.to_f32());
}

/// `__init_g_b613a0` (0x84C760): `[0xB613A0] = [0x858630] / [0x85F1D4]`.
#[hook(addr = 0x84C760, conv = cdecl, name = "__init_g_b613a0", expect = "D9 05 30 86 85 00")]
pub fn init_g_b613a0() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x85F1D4).read());
    g::<f32>(0xB613A0).write(v.to_f32());
}

/// `__init_g_b613a4` (0x84C780): `[0xB613A4] = [0x8AE5F4] - [0x8AE5F0]`.
#[hook(addr = 0x84C780, conv = cdecl, name = "__init_g_b613a4", expect = "D9 05 F4 E5 8A 00")]
pub fn init_g_b613a4() {
    let v = x(g::<f32>(0x8AE5F4).read()).fsub_m(g::<f32>(0x8AE5F0).read());
    g::<f32>(0xB613A4).write(v.to_f32());
}

/// `__init_g_b613a8` (0x84C7A0): `[0xB613A8] = [0x8AE5FC] - [0x8AE5F8]`.
#[hook(addr = 0x84C7A0, conv = cdecl, name = "__init_g_b613a8", expect = "D9 05 FC E5 8A 00")]
pub fn init_g_b613a8() {
    let v = x(g::<f32>(0x8AE5FC).read()).fsub_m(g::<f32>(0x8AE5F8).read());
    g::<f32>(0xB613A8).write(v.to_f32());
}

/// `__init_g_b613ac` (0x84C7C0): `[0xB613AC] = [0x8AE604] - [0x8AE600]`.
#[hook(addr = 0x84C7C0, conv = cdecl, name = "__init_g_b613ac", expect = "D9 05 04 E6 8A 00")]
pub fn init_g_b613ac() {
    let v = x(g::<f32>(0x8AE604).read()).fsub_m(g::<f32>(0x8AE600).read());
    g::<f32>(0xB613AC).write(v.to_f32());
}

/// `__init_g_b613b4` (0x84C800): `[0xB613B4] = [0x8AE614] - [0x8AE610]`.
#[hook(addr = 0x84C800, conv = cdecl, name = "__init_g_b613b4", expect = "D9 05 14 E6 8A 00")]
pub fn init_g_b613b4() {
    let v = x(g::<f32>(0x8AE614).read()).fsub_m(g::<f32>(0x8AE610).read());
    g::<f32>(0xB613B4).write(v.to_f32());
}

/// `__init_g_b613b8` (0x84C820): `[0xB613B8] = [0x8AE61C] - [0x8AE618]`.
#[hook(addr = 0x84C820, conv = cdecl, name = "__init_g_b613b8", expect = "D9 05 1C E6 8A 00")]
pub fn init_g_b613b8() {
    let v = x(g::<f32>(0x8AE61C).read()).fsub_m(g::<f32>(0x8AE618).read());
    g::<f32>(0xB613B8).write(v.to_f32());
}

/// `__init_g_b613c4` (0x84C880): `[0xB613C4] = [0x8AE680] - [0x8AE67C]`.
#[hook(addr = 0x84C880, conv = cdecl, name = "__init_g_b613c4", expect = "D9 05 80 E6 8A 00")]
pub fn init_g_b613c4() {
    let v = x(g::<f32>(0x8AE680).read()).fsub_m(g::<f32>(0x8AE67C).read());
    g::<f32>(0xB613C4).write(v.to_f32());
}

/// `__init_g_b613c8` (0x84C8A0): `[0xB613C8] = [0x8AE690] - [0x8AE68C]`.
#[hook(addr = 0x84C8A0, conv = cdecl, name = "__init_g_b613c8", expect = "D9 05 90 E6 8A 00")]
pub fn init_g_b613c8() {
    let v = x(g::<f32>(0x8AE690).read()).fsub_m(g::<f32>(0x8AE68C).read());
    g::<f32>(0xB613C8).write(v.to_f32());
}

/// `__init_g_b61c1c` (0x84C8E0): `[0xB61C1C] = [0x85F258] * [0x858620]`.
#[hook(addr = 0x84C8E0, conv = cdecl, name = "__init_g_b61c1c", expect = "D9 05 58 F2 85 00")]
pub fn init_g_b61c1c() {
    let v = x(g::<f32>(0x85F258).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB61C1C).write(v.to_f32());
}

/// `__init_g_b61c20` (0x84C900): `[0xB61C20] = [0x858624] / [0x85F260]`.
#[hook(addr = 0x84C900, conv = cdecl, name = "__init_g_b61c20", expect = "D9 05 24 86 85 00")]
pub fn init_g_b61c20() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x85F260).read());
    g::<f32>(0xB61C20).write(v.to_f32());
}

/// `__init_g_b61c24` (0x84C920): `[0xB61C24] = [0x858628] / [0x85F25C]`.
#[hook(addr = 0x84C920, conv = cdecl, name = "__init_g_b61c24", expect = "D9 05 28 86 85 00")]
pub fn init_g_b61c24() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x85F25C).read());
    g::<f32>(0xB61C24).write(v.to_f32());
}

/// `__init_g_b61c28` (0x84C940): `[0xB61C28] = [0x858624] / ([0x85F260] * [0x85F260])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84C940, conv = cdecl, name = "__init_g_b61c28", expect = "D9 05 60 F2 85 00")]
pub fn init_g_b61c28() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x85F260).read(),
        g::<f32>(0x85F260).read(),
    );
    g::<f32>(0xB61C28).write(v);
}

/// `__init_g_b61c2c` (0x84C960): `[0xB61C2C] = [0xB61C28] * [0x85862C]`.
#[hook(addr = 0x84C960, conv = cdecl, name = "__init_g_b61c2c", expect = "D9 05 28 1C B6 00")]
pub fn init_g_b61c2c() {
    let v = x(g::<f32>(0xB61C28).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB61C2C).write(v.to_f32());
}

/// `__init_g_b61c30` (0x84C980): `[0xB61C30] = [0x858630] / [0x85F260]`.
#[hook(addr = 0x84C980, conv = cdecl, name = "__init_g_b61c30", expect = "D9 05 30 86 85 00")]
pub fn init_g_b61c30() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x85F260).read());
    g::<f32>(0xB61C30).write(v.to_f32());
}

/// `__init_g_b61ce0` (0x84C9D0): `[0xB61CE0] = [0x85F350] * [0x858620]`.
#[hook(addr = 0x84C9D0, conv = cdecl, name = "__init_g_b61ce0", expect = "D9 05 50 F3 85 00")]
pub fn init_g_b61ce0() {
    let v = x(g::<f32>(0x85F350).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB61CE0).write(v.to_f32());
}

/// `__init_g_b61ce4` (0x84C9F0): `[0xB61CE4] = [0x858624] / [0x85F358]`.
#[hook(addr = 0x84C9F0, conv = cdecl, name = "__init_g_b61ce4", expect = "D9 05 24 86 85 00")]
pub fn init_g_b61ce4() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x85F358).read());
    g::<f32>(0xB61CE4).write(v.to_f32());
}

/// `__init_g_b61ce8` (0x84CA10): `[0xB61CE8] = [0x858628] / [0x85F354]`.
#[hook(addr = 0x84CA10, conv = cdecl, name = "__init_g_b61ce8", expect = "D9 05 28 86 85 00")]
pub fn init_g_b61ce8() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x85F354).read());
    g::<f32>(0xB61CE8).write(v.to_f32());
}

/// `__init_g_b61cec` (0x84CA30): `[0xB61CEC] = [0x858624] / ([0x85F358] * [0x85F358])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84CA30, conv = cdecl, name = "__init_g_b61cec", expect = "D9 05 58 F3 85 00")]
pub fn init_g_b61cec() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x85F358).read(),
        g::<f32>(0x85F358).read(),
    );
    g::<f32>(0xB61CEC).write(v);
}

/// `__init_g_b61cf0` (0x84CA50): `[0xB61CF0] = [0xB61CEC] * [0x85862C]`.
#[hook(addr = 0x84CA50, conv = cdecl, name = "__init_g_b61cf0", expect = "D9 05 EC 1C B6 00")]
pub fn init_g_b61cf0() {
    let v = x(g::<f32>(0xB61CEC).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB61CF0).write(v.to_f32());
}

/// `__init_g_b61cf4` (0x84CA70): `[0xB61CF4] = [0x858630] / [0x85F358]`.
#[hook(addr = 0x84CA70, conv = cdecl, name = "__init_g_b61cf4", expect = "D9 05 30 86 85 00")]
pub fn init_g_b61cf4() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x85F358).read());
    g::<f32>(0xB61CF4).write(v.to_f32());
}

/// `__init_g_b61d58` (0x84CA90): `[0xB61D58] = [0x858624] / [0x85F360]`.
#[hook(addr = 0x84CA90, conv = cdecl, name = "__init_g_b61d58", expect = "D9 05 24 86 85 00")]
pub fn init_g_b61d58() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x85F360).read());
    g::<f32>(0xB61D58).write(v.to_f32());
}

/// `__init_g_b61d5c` (0x84CAB0): `[0xB61D5C] = [0x858628] / [0x85F35C]`.
#[hook(addr = 0x84CAB0, conv = cdecl, name = "__init_g_b61d5c", expect = "D9 05 28 86 85 00")]
pub fn init_g_b61d5c() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x85F35C).read());
    g::<f32>(0xB61D5C).write(v.to_f32());
}

/// `__init_g_b61d60` (0x84CAD0): `[0xB61D60] = [0x858624] / ([0x85F360] * [0x85F360])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84CAD0, conv = cdecl, name = "__init_g_b61d60", expect = "D9 05 60 F3 85 00")]
pub fn init_g_b61d60() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x85F360).read(),
        g::<f32>(0x85F360).read(),
    );
    g::<f32>(0xB61D60).write(v);
}

/// `__init_g_b61d64` (0x84CAF0): `[0xB61D64] = [0xB61D60] * [0x85862C]`.
#[hook(addr = 0x84CAF0, conv = cdecl, name = "__init_g_b61d64", expect = "D9 05 60 1D B6 00")]
pub fn init_g_b61d64() {
    let v = x(g::<f32>(0xB61D60).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB61D64).write(v.to_f32());
}

/// `__init_g_b61d68` (0x84CB10): `[0xB61D68] = [0x858630] / [0x85F360]`.
#[hook(addr = 0x84CB10, conv = cdecl, name = "__init_g_b61d68", expect = "D9 05 30 86 85 00")]
pub fn init_g_b61d68() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x85F360).read());
    g::<f32>(0xB61D68).write(v.to_f32());
}

/// `__init_g_b61d6c` (0x84CB30): `[0xB61D6C] = [0x85F364] * [0x858620]`.
#[hook(addr = 0x84CB30, conv = cdecl, name = "__init_g_b61d6c", expect = "D9 05 64 F3 85 00")]
pub fn init_g_b61d6c() {
    let v = x(g::<f32>(0x85F364).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB61D6C).write(v.to_f32());
}

/// `__init_g_b61d74` (0x84CB50): `[0xB61D74] = [0x85F36C] * [0x858620]`.
#[hook(addr = 0x84CB50, conv = cdecl, name = "__init_g_b61d74", expect = "D9 05 6C F3 85 00")]
pub fn init_g_b61d74() {
    let v = x(g::<f32>(0x85F36C).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB61D74).write(v.to_f32());
}

/// `__init_g_b62c74` (0x84CB70): `[0xB62C74] = [0x858624] / [0x85F398]`.
#[hook(addr = 0x84CB70, conv = cdecl, name = "__init_g_b62c74", expect = "D9 05 24 86 85 00")]
pub fn init_g_b62c74() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x85F398).read());
    g::<f32>(0xB62C74).write(v.to_f32());
}

/// `__init_g_b62c78` (0x84CB90): `[0xB62C78] = [0x858628] / [0x85F394]`.
#[hook(addr = 0x84CB90, conv = cdecl, name = "__init_g_b62c78", expect = "D9 05 28 86 85 00")]
pub fn init_g_b62c78() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x85F394).read());
    g::<f32>(0xB62C78).write(v.to_f32());
}

/// `__init_g_b62c7c` (0x84CBB0): `[0xB62C7C] = [0x858624] / ([0x85F398] * [0x85F398])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84CBB0, conv = cdecl, name = "__init_g_b62c7c", expect = "D9 05 98 F3 85 00")]
pub fn init_g_b62c7c() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x85F398).read(),
        g::<f32>(0x85F398).read(),
    );
    g::<f32>(0xB62C7C).write(v);
}

/// `__init_g_b62c80` (0x84CBD0): `[0xB62C80] = [0xB62C7C] * [0x85862C]`.
#[hook(addr = 0x84CBD0, conv = cdecl, name = "__init_g_b62c80", expect = "D9 05 7C 2C B6 00")]
pub fn init_g_b62c80() {
    let v = x(g::<f32>(0xB62C7C).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB62C80).write(v.to_f32());
}

/// `__init_g_b62c84` (0x84CBF0): `[0xB62C84] = [0x858630] / [0x85F398]`.
#[hook(addr = 0x84CBF0, conv = cdecl, name = "__init_g_b62c84", expect = "D9 05 30 86 85 00")]
pub fn init_g_b62c84() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x85F398).read());
    g::<f32>(0xB62C84).write(v.to_f32());
}

/// `__init_g_b62c88` (0x84CC10): `[0xB62C88] = [0x85F39C] * [0x858620]`.
#[hook(addr = 0x84CC10, conv = cdecl, name = "__init_g_b62c88", expect = "D9 05 9C F3 85 00")]
pub fn init_g_b62c88() {
    let v = x(g::<f32>(0x85F39C).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB62C88).write(v.to_f32());
}

/// `__init_84cc30` (0x84CC30): `atexit(0x856350)`.
#[hook(addr = 0x84CC30, conv = cdecl, name = "__init_84cc30", expect = "68 50 63 85 00")]
pub fn init_84cc30() {
    atexit(0x856350);
}

/// `__init_g_b62c8c` (0x84CC40): `[0xB62C8C] = [0x85F3B8] * [0x858620]`.
#[hook(addr = 0x84CC40, conv = cdecl, name = "__init_g_b62c8c", expect = "D9 05 B8 F3 85 00")]
pub fn init_g_b62c8c() {
    let v = x(g::<f32>(0x85F3B8).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB62C8C).write(v.to_f32());
}

/// `__init_g_b62c90` (0x84CC60): `[0xB62C90] = [0x858624] / [0x85F3C0]`.
#[hook(addr = 0x84CC60, conv = cdecl, name = "__init_g_b62c90", expect = "D9 05 24 86 85 00")]
pub fn init_g_b62c90() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x85F3C0).read());
    g::<f32>(0xB62C90).write(v.to_f32());
}

/// `__init_g_b62c94` (0x84CC80): `[0xB62C94] = [0x858628] / [0x85F3BC]`.
#[hook(addr = 0x84CC80, conv = cdecl, name = "__init_g_b62c94", expect = "D9 05 28 86 85 00")]
pub fn init_g_b62c94() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x85F3BC).read());
    g::<f32>(0xB62C94).write(v.to_f32());
}

/// `__init_g_b62c98` (0x84CCA0): `[0xB62C98] = [0x858624] / ([0x85F3C0] * [0x85F3C0])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84CCA0, conv = cdecl, name = "__init_g_b62c98", expect = "D9 05 C0 F3 85 00")]
pub fn init_g_b62c98() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x85F3C0).read(),
        g::<f32>(0x85F3C0).read(),
    );
    g::<f32>(0xB62C98).write(v);
}

/// `__init_g_b62c9c` (0x84CCC0): `[0xB62C9C] = [0xB62C98] * [0x85862C]`.
#[hook(addr = 0x84CCC0, conv = cdecl, name = "__init_g_b62c9c", expect = "D9 05 98 2C B6 00")]
pub fn init_g_b62c9c() {
    let v = x(g::<f32>(0xB62C98).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB62C9C).write(v.to_f32());
}

/// `__init_g_b62ca0` (0x84CCE0): `[0xB62CA0] = [0x858630] / [0x85F3C0]`.
#[hook(addr = 0x84CCE0, conv = cdecl, name = "__init_g_b62ca0", expect = "D9 05 30 86 85 00")]
pub fn init_g_b62ca0() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x85F3C0).read());
    g::<f32>(0xB62CA0).write(v.to_f32());
}

/// `__init_g_b62ca8` (0x84CD00): `[0xB62CA8] = [0x85F3C8] * [0x858620]`.
#[hook(addr = 0x84CD00, conv = cdecl, name = "__init_g_b62ca8", expect = "D9 05 C8 F3 85 00")]
pub fn init_g_b62ca8() {
    let v = x(g::<f32>(0x85F3C8).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB62CA8).write(v.to_f32());
}

/// `__init_g_b6b96c` (0x84CD40): `[0xB6B96C] = [0x85F43C] * [0x858620]`.
#[hook(addr = 0x84CD40, conv = cdecl, name = "__init_g_b6b96c", expect = "D9 05 3C F4 85 00")]
pub fn init_g_b6b96c() {
    let v = x(g::<f32>(0x85F43C).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB6B96C).write(v.to_f32());
}

/// `__init_84cd60` (0x84CD60): `atexit(0x856370)`.
#[hook(addr = 0x84CD60, conv = cdecl, name = "__init_84cd60", expect = "68 70 63 85 00")]
pub fn init_84cd60() {
    atexit(0x856370);
}

/// `__init_g_b6ba08` (0x84CD70): `[0xB6BA08] = [0x860AE0] * [0x858620]`.
#[hook(addr = 0x84CD70, conv = cdecl, name = "__init_g_b6ba08", expect = "D9 05 E0 0A 86 00")]
pub fn init_g_b6ba08() {
    let v = x(g::<f32>(0x860AE0).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB6BA08).write(v.to_f32());
}

/// `__init_g_b6ba0c` (0x84CD90): `[0xB6BA0C] = [0x858624] / [0x860AE8]`.
#[hook(addr = 0x84CD90, conv = cdecl, name = "__init_g_b6ba0c", expect = "D9 05 24 86 85 00")]
pub fn init_g_b6ba0c() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x860AE8).read());
    g::<f32>(0xB6BA0C).write(v.to_f32());
}

/// `__init_g_b6ba10` (0x84CDB0): `[0xB6BA10] = [0x858628] / [0x860AE4]`.
#[hook(addr = 0x84CDB0, conv = cdecl, name = "__init_g_b6ba10", expect = "D9 05 28 86 85 00")]
pub fn init_g_b6ba10() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x860AE4).read());
    g::<f32>(0xB6BA10).write(v.to_f32());
}

/// `__init_g_b6ba14` (0x84CDD0): `[0xB6BA14] = [0x858624] / ([0x860AE8] * [0x860AE8])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84CDD0, conv = cdecl, name = "__init_g_b6ba14", expect = "D9 05 E8 0A 86 00")]
pub fn init_g_b6ba14() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x860AE8).read(),
        g::<f32>(0x860AE8).read(),
    );
    g::<f32>(0xB6BA14).write(v);
}

/// `__init_g_b6ba18` (0x84CDF0): `[0xB6BA18] = [0xB6BA14] * [0x85862C]`.
#[hook(addr = 0x84CDF0, conv = cdecl, name = "__init_g_b6ba18", expect = "D9 05 14 BA B6 00")]
pub fn init_g_b6ba18() {
    let v = x(g::<f32>(0xB6BA14).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB6BA18).write(v.to_f32());
}

/// `__init_g_b6ba1c` (0x84CE10): `[0xB6BA1C] = [0x858630] / [0x860AE8]`.
#[hook(addr = 0x84CE10, conv = cdecl, name = "__init_g_b6ba1c", expect = "D9 05 30 86 85 00")]
pub fn init_g_b6ba1c() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x860AE8).read());
    g::<f32>(0xB6BA1C).write(v.to_f32());
}

/// `__init_g_b6ba30` (0x84CEB0): `[0xB6BA30] = [0x8CBC64] - [0x8CBC60]`.
#[hook(addr = 0x84CEB0, conv = cdecl, name = "__init_g_b6ba30", expect = "D9 05 64 BC 8C 00")]
pub fn init_g_b6ba30() {
    let v = x(g::<f32>(0x8CBC64).read()).fsub_m(g::<f32>(0x8CBC60).read());
    g::<f32>(0xB6BA30).write(v.to_f32());
}

/// `__init_g_b6ba34` (0x84CED0): `[0xB6BA34] = [0xB6B9D4] - [0x8CBC68]`.
#[hook(addr = 0x84CED0, conv = cdecl, name = "__init_g_b6ba34", expect = "D9 05 D4 B9 B6 00")]
pub fn init_g_b6ba34() {
    let v = x(g::<f32>(0xB6B9D4).read()).fsub_m(g::<f32>(0x8CBC68).read());
    g::<f32>(0xB6BA34).write(v.to_f32());
}

/// `__init_g_b6ba48` (0x84CF60): `[0xB6BA48] = [0x8CBC84] - [0x8CBC80]`.
#[hook(addr = 0x84CF60, conv = cdecl, name = "__init_g_b6ba48", expect = "D9 05 84 BC 8C 00")]
pub fn init_g_b6ba48() {
    let v = x(g::<f32>(0x8CBC84).read()).fsub_m(g::<f32>(0x8CBC80).read());
    g::<f32>(0xB6BA48).write(v.to_f32());
}

/// `__init_g_b6ba4c` (0x84CF80): `[0xB6BA4C] = [0x8CBC88] - [0xB6B9D8]`.
#[hook(addr = 0x84CF80, conv = cdecl, name = "__init_g_b6ba4c", expect = "D9 05 88 BC 8C 00")]
pub fn init_g_b6ba4c() {
    let v = x(g::<f32>(0x8CBC88).read()).fsub_m(g::<f32>(0xB6B9D8).read());
    g::<f32>(0xB6BA4C).write(v.to_f32());
}

/// `__init_g_b6ba60` (0x84D010): `[0xB6BA60] = [0x8CBCA0] - [0x8CBC9C]`.
#[hook(addr = 0x84D010, conv = cdecl, name = "__init_g_b6ba60", expect = "D9 05 A0 BC 8C 00")]
pub fn init_g_b6ba60() {
    let v = x(g::<f32>(0x8CBCA0).read()).fsub_m(g::<f32>(0x8CBC9C).read());
    g::<f32>(0xB6BA60).write(v.to_f32());
}

/// `__init_g_b6ba7c` (0x84D0F0): `[0xB6BA7C] = [0x8CBF0C] - [0xB6BA00]`.
#[hook(addr = 0x84D0F0, conv = cdecl, name = "__init_g_b6ba7c", expect = "D9 05 0C BF 8C 00")]
pub fn init_g_b6ba7c() {
    let v = x(g::<f32>(0x8CBF0C).read()).fsub_m(g::<f32>(0xB6BA00).read());
    g::<f32>(0xB6BA7C).write(v.to_f32());
}

/// `__init_g_b6ba8c` (0x84D170): `[0xB6BA8C] = [0x8CBF78] - [0x8CBF74]`.
#[hook(addr = 0x84D170, conv = cdecl, name = "__init_g_b6ba8c", expect = "D9 05 78 BF 8C 00")]
pub fn init_g_b6ba8c() {
    let v = x(g::<f32>(0x8CBF78).read()).fsub_m(g::<f32>(0x8CBF74).read());
    g::<f32>(0xB6BA8C).write(v.to_f32());
}

/// `__init_g_b6ba98` (0x84D1F0): `[0xB6BA98] = [0x8CBFCC] - [0x8CBFC8]`.
#[hook(addr = 0x84D1F0, conv = cdecl, name = "__init_g_b6ba98", expect = "D9 05 CC BF 8C 00")]
pub fn init_g_b6ba98() {
    let v = x(g::<f32>(0x8CBFCC).read()).fsub_m(g::<f32>(0x8CBFC8).read());
    g::<f32>(0xB6BA98).write(v.to_f32());
}

/// `__init_g_b6badc` (0x84D330): `[0xB6BADC] = [0x862DE4] * [0x858620]`.
#[hook(addr = 0x84D330, conv = cdecl, name = "__init_g_b6badc", expect = "D9 05 E4 2D 86 00")]
pub fn init_g_b6badc() {
    let v = x(g::<f32>(0x862DE4).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB6BADC).write(v.to_f32());
}

/// `__init_g_b6bae0` (0x84D350): `[0xB6BAE0] = [0x862E08] * [0x858620]`.
#[hook(addr = 0x84D350, conv = cdecl, name = "__init_g_b6bae0", expect = "D9 05 08 2E 86 00")]
pub fn init_g_b6bae0() {
    let v = x(g::<f32>(0x862E08).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB6BAE0).write(v.to_f32());
}

/// `__init_g_b6bae4` (0x84D370): `[0xB6BAE4] = [0x858624] / [0x862E10]`.
#[hook(addr = 0x84D370, conv = cdecl, name = "__init_g_b6bae4", expect = "D9 05 24 86 85 00")]
pub fn init_g_b6bae4() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x862E10).read());
    g::<f32>(0xB6BAE4).write(v.to_f32());
}

/// `__init_g_b6bae8` (0x84D390): `[0xB6BAE8] = [0x858628] / [0x862E0C]`.
#[hook(addr = 0x84D390, conv = cdecl, name = "__init_g_b6bae8", expect = "D9 05 28 86 85 00")]
pub fn init_g_b6bae8() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x862E0C).read());
    g::<f32>(0xB6BAE8).write(v.to_f32());
}

/// `__init_g_b6baec` (0x84D3B0): `[0xB6BAEC] = [0x858624] / ([0x862E10] * [0x862E10])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84D3B0, conv = cdecl, name = "__init_g_b6baec", expect = "D9 05 10 2E 86 00")]
pub fn init_g_b6baec() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x862E10).read(),
        g::<f32>(0x862E10).read(),
    );
    g::<f32>(0xB6BAEC).write(v);
}

/// `__init_g_b6baf0` (0x84D3D0): `[0xB6BAF0] = [0xB6BAEC] * [0x85862C]`.
#[hook(addr = 0x84D3D0, conv = cdecl, name = "__init_g_b6baf0", expect = "D9 05 EC BA B6 00")]
pub fn init_g_b6baf0() {
    let v = x(g::<f32>(0xB6BAEC).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB6BAF0).write(v.to_f32());
}

/// `__init_g_b6baf4` (0x84D3F0): `[0xB6BAF4] = [0x858630] / [0x862E10]`.
#[hook(addr = 0x84D3F0, conv = cdecl, name = "__init_g_b6baf4", expect = "D9 05 30 86 85 00")]
pub fn init_g_b6baf4() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x862E10).read());
    g::<f32>(0xB6BAF4).write(v.to_f32());
}

/// `__init_g_b6bb00` (0x84D410): `[0xB6BB00] = [0x862E20] * [0x858620]`.
#[hook(addr = 0x84D410, conv = cdecl, name = "__init_g_b6bb00", expect = "D9 05 20 2E 86 00")]
pub fn init_g_b6bb00() {
    let v = x(g::<f32>(0x862E20).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB6BB00).write(v.to_f32());
}

/// `__init_g_b6bb04` (0x84D430): `[0xB6BB04] = [0x858624] / [0x862E28]`.
#[hook(addr = 0x84D430, conv = cdecl, name = "__init_g_b6bb04", expect = "D9 05 24 86 85 00")]
pub fn init_g_b6bb04() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x862E28).read());
    g::<f32>(0xB6BB04).write(v.to_f32());
}

/// `__init_g_b6bb08` (0x84D450): `[0xB6BB08] = [0x858628] / [0x862E24]`.
#[hook(addr = 0x84D450, conv = cdecl, name = "__init_g_b6bb08", expect = "D9 05 28 86 85 00")]
pub fn init_g_b6bb08() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x862E24).read());
    g::<f32>(0xB6BB08).write(v.to_f32());
}

/// `__init_g_b6bb0c` (0x84D470): `[0xB6BB0C] = [0x858624] / ([0x862E28] * [0x862E28])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84D470, conv = cdecl, name = "__init_g_b6bb0c", expect = "D9 05 28 2E 86 00")]
pub fn init_g_b6bb0c() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x862E28).read(),
        g::<f32>(0x862E28).read(),
    );
    g::<f32>(0xB6BB0C).write(v);
}

/// `__init_g_b6bb10` (0x84D490): `[0xB6BB10] = [0xB6BB0C] * [0x85862C]`.
#[hook(addr = 0x84D490, conv = cdecl, name = "__init_g_b6bb10", expect = "D9 05 0C BB B6 00")]
pub fn init_g_b6bb10() {
    let v = x(g::<f32>(0xB6BB0C).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB6BB10).write(v.to_f32());
}

/// `__init_g_b6bb14` (0x84D4B0): `[0xB6BB14] = [0x858630] / [0x862E28]`.
#[hook(addr = 0x84D4B0, conv = cdecl, name = "__init_g_b6bb14", expect = "D9 05 30 86 85 00")]
pub fn init_g_b6bb14() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x862E28).read());
    g::<f32>(0xB6BB14).write(v.to_f32());
}

/// `__init_g_b6bc78` (0x84D510): `[0xB6BC78] = [0x862E48] * [0x858620]`.
#[hook(addr = 0x84D510, conv = cdecl, name = "__init_g_b6bc78", expect = "D9 05 48 2E 86 00")]
pub fn init_g_b6bc78() {
    let v = x(g::<f32>(0x862E48).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xB6BC78).write(v.to_f32());
}

/// `__init_g_b6bc7c` (0x84D530): `[0xB6BC7C] = [0x858624] / [0x862E50]`.
#[hook(addr = 0x84D530, conv = cdecl, name = "__init_g_b6bc7c", expect = "D9 05 24 86 85 00")]
pub fn init_g_b6bc7c() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x862E50).read());
    g::<f32>(0xB6BC7C).write(v.to_f32());
}

/// `__init_g_b6bc80` (0x84D550): `[0xB6BC80] = [0x858628] / [0x862E4C]`.
#[hook(addr = 0x84D550, conv = cdecl, name = "__init_g_b6bc80", expect = "D9 05 28 86 85 00")]
pub fn init_g_b6bc80() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x862E4C).read());
    g::<f32>(0xB6BC80).write(v.to_f32());
}

/// `__init_g_b6bc84` (0x84D570): `[0xB6BC84] = [0x858624] / ([0x862E50] * [0x862E50])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x84D570, conv = cdecl, name = "__init_g_b6bc84", expect = "D9 05 50 2E 86 00")]
pub fn init_g_b6bc84() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x862E50).read(),
        g::<f32>(0x862E50).read(),
    );
    g::<f32>(0xB6BC84).write(v);
}

/// `__init_g_b6bc88` (0x84D590): `[0xB6BC88] = [0xB6BC84] * [0x85862C]`.
#[hook(addr = 0x84D590, conv = cdecl, name = "__init_g_b6bc88", expect = "D9 05 84 BC B6 00")]
pub fn init_g_b6bc88() {
    let v = x(g::<f32>(0xB6BC84).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xB6BC88).write(v.to_f32());
}

/// `__init_g_b6bc8c` (0x84D5B0): `[0xB6BC8C] = [0x858630] / [0x862E50]`.
#[hook(addr = 0x84D5B0, conv = cdecl, name = "__init_g_b6bc8c", expect = "D9 05 30 86 85 00")]
pub fn init_g_b6bc8c() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x862E50).read());
    g::<f32>(0xB6BC8C).write(v.to_f32());
}

/// `__init_g_bd00e0` (0x850200): `[0xBD00E0] = [0x868B34] * [0x858620]`.
#[hook(addr = 0x850200, conv = cdecl, name = "__init_g_bd00e0", expect = "D9 05 34 8B 86 00")]
pub fn init_g_bd00e0() {
    let v = x(g::<f32>(0x868B34).read()).fmul_m(g::<f32>(0x858620).read());
    g::<f32>(0xBD00E0).write(v.to_f32());
}

/// `__init_g_bd00e4` (0x850220): `[0xBD00E4] = [0x858624] / [0x868B3C]`.
#[hook(addr = 0x850220, conv = cdecl, name = "__init_g_bd00e4", expect = "D9 05 24 86 85 00")]
pub fn init_g_bd00e4() {
    let v = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0x868B3C).read());
    g::<f32>(0xBD00E4).write(v.to_f32());
}

/// `__init_g_bd00e8` (0x850240): `[0xBD00E8] = [0x858628] / [0x868B38]`.
#[hook(addr = 0x850240, conv = cdecl, name = "__init_g_bd00e8", expect = "D9 05 28 86 85 00")]
pub fn init_g_bd00e8() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x868B38).read());
    g::<f32>(0xBD00E8).write(v.to_f32());
}

/// `__init_g_bd00ec` (0x850260): `[0xBD00EC] = [0x858624] / ([0x868B3C] * [0x868B3C])` (the product stays in ST0, see `div_of_product`).
#[hook(addr = 0x850260, conv = cdecl, name = "__init_g_bd00ec", expect = "D9 05 3C 8B 86 00")]
pub fn init_g_bd00ec() {
    let v = div_of_product(
        g::<f32>(0x858624).read(),
        g::<f32>(0x868B3C).read(),
        g::<f32>(0x868B3C).read(),
    );
    g::<f32>(0xBD00EC).write(v);
}

/// `__init_g_bd00f0` (0x850280): `[0xBD00F0] = [0xBD00EC] * [0x85862C]`.
#[hook(addr = 0x850280, conv = cdecl, name = "__init_g_bd00f0", expect = "D9 05 EC 00 BD 00")]
pub fn init_g_bd00f0() {
    let v = x(g::<f32>(0xBD00EC).read()).fmul_m(g::<f32>(0x85862C).read());
    g::<f32>(0xBD00F0).write(v.to_f32());
}

/// `__init_g_bd00f4` (0x8502A0): `[0xBD00F4] = [0x858630] / [0x868B3C]`.
#[hook(addr = 0x8502A0, conv = cdecl, name = "__init_g_bd00f4", expect = "D9 05 30 86 85 00")]
pub fn init_g_bd00f4() {
    let v = x(g::<f32>(0x858630).read()).fdiv_m(g::<f32>(0x868B3C).read());
    g::<f32>(0xBD00F4).write(v.to_f32());
}

/// `__init_g_c0b1d8` (0x851600): `[0xC0B1D8] = [0x858628] / [0x86CF0C]`.
#[hook(addr = 0x851600, conv = cdecl, name = "__init_g_c0b1d8", expect = "D9 05 28 86 85 00")]
pub fn init_g_c0b1d8() {
    let v = x(g::<f32>(0x858628).read()).fdiv_m(g::<f32>(0x86CF0C).read());
    g::<f32>(0xC0B1D8).write(v.to_f32());
}

// ---------------------------------------------------------------------------------------------------
// Game-side members
// ---------------------------------------------------------------------------------------------------

/// `CTimer::m_UserPause` (0xB7CB49)
const TIMER_USER_PAUSE: usize = 0xB7CB49;
/// `CTimer::m_CodePause` (0xB7CB48)
const TIMER_CODE_PAUSE: usize = 0xB7CB48;
/// `CTimer::m_snTimeInMillisecondsPauseMode` (0xB7CB7C)
const TIMER_MS_PAUSE_MODE: usize = 0xB7CB7C;
/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const TIMER_MS: usize = 0xB7CB84;
/// `CAEPoliceScannerAudioEntity::s_pPSControlling` (0xB61D48)
const PS_CONTROLLING: usize = 0xB61D48;
/// `CAEPoliceScannerAudioEntity::s_nScannerPlaybackState` (0xB61D4C, int16)
const PS_PLAYBACK_STATE: usize = 0xB61D4C;
/// `CAEPoliceScannerAudioEntity::s_bScannerDisabled` (0xB61D4E)
const PS_SCANNER_DISABLED: usize = 0xB61D4E;
/// `CAEPoliceScannerAudioEntity::s_NextNewScannerDialogueTime` (0xB61D50)
const PS_NEXT_DIALOGUE_TIME: usize = 0xB61D50;

fn game_paused() -> bool {
    g::<u8>(TIMER_USER_PAUSE).read() != 0 || g::<u8>(TIMER_CODE_PAUSE).read() != 0
}

orig! {
    /// `CAEAudioHardware::SetChannelFrequencyScalingFactor(int16 slot, int16 channel, float factor)` (0x4D8960)
    fn hw_set_channel_frequency_scaling_factor(hw: GPtr<u8>, slot: u32, channel: u32, factor_bits: u32) = thiscall 0x4D8960 => unported;
    /// `CAEPoliceScannerAudioEntity::StopScanner(bool)` (0x4E6DC0)
    fn ps_stop_scanner(b: u32) = cdecl 0x4E6DC0 => unported;
}

/// `CAEAmbienceTrackManager::Reset()` (0x4D6CA0): +0xC, +0x10 = -1; +0, +1, +2 = 0; +0x14 = 0; +3 = 0xFF.
/// Returns the original's EAX (0); EDX (-1) is not modelled (the only caller, `CAudioEngine::Reset`,
/// overwrites it).
#[hook(addr = 0x4D6CA0, conv = thiscall, name = "CAEAmbienceTrackManager::Reset", expect = "33 C0 83 CA FF")]
pub fn ambience_reset(this: GPtr<u8>) -> u32 {
    wr::<i32>(this, 0xC, -1);
    wr::<u8>(this, 0, 0);
    wr::<u8>(this, 1, 0);
    wr::<i32>(this, 0x10, -1);
    wr::<u32>(this, 0x14, 0);
    wr::<u8>(this, 2, 0);
    wr::<u8>(this, 3, 0xFF);
    0
}

/// `CAECutsceneTrackManager::PauseTrack(bool)` (0x4DBF00): `this[+1] = pause`.
#[hook(addr = 0x4DBF00, conv = thiscall, name = "CAECutsceneTrackManager::PauseTrack", expect = "8A 44 24 04 88 41 01")]
pub fn cutscene_pause_track(this: GPtr<u8>, pause: u8) {
    wr::<u8>(this, 1, pause);
}

/// `CAECutsceneTrackManager::CheckForPause()` (0x4DBF60): frequency factor 0.0 while the game or the
/// track (+1) is paused, else 1.0, on channel 0 of slot `(int16)this[+4]`.
#[hook(addr = 0x4DBF60, conv = thiscall, name = "CAECutsceneTrackManager::CheckForPause", expect = "51 A0 49 CB B7 00")]
pub fn cutscene_check_for_pause(this: GPtr<u8>) {
    let factor: u32 = if game_paused() || rd::<u8>(this, 1) != 0 {
        0
    } else {
        0x3F80_0000
    };
    let slot = rd::<i16>(this, 4) as i32 as u32;
    hw_set_channel_frequency_scaling_factor(g(AE_AUDIO_HARDWARE), slot, 0, factor);
}

/// `CAEFrontendAudioEntity::IsRadioTuneSoundActive()` (0x4DD480): byte +0x1CC while the game is paused,
/// else +0x124 (the `m_bRadioTuneSound*` flags).
#[hook(addr = 0x4DD480, conv = thiscall, name = "CAEFrontendAudioEntity::IsRadioTuneSoundActive", expect = "A0 49 CB B7 00")]
pub fn frontend_is_radio_tune_sound_active(this: GPtr<u8>) -> u8 {
    if game_paused() {
        rd::<u8>(this, 0x1CC)
    } else {
        rd::<u8>(this, 0x124)
    }
}

/// `CAERadioTrackManager::GetCurrentRadioStationID()` (0x4E83F0; entry `nop; jmp` into the `.text` chunk
/// 0x405FF5): station byte +0xE9, 0xFF (none) reported as 13 (radio off).
#[hook(addr = 0x4E83F0, conv = thiscall, name = "CAERadioTrackManager::GetCurrentRadioStationID", expect = "90 E9 FF DB F1 FF")]
pub fn radio_get_current_radio_station_id(this: GPtr<u8>) -> u8 {
    let id = rd::<u8>(this, 0xE9);
    if id == 0xFF { 13 } else { id }
}

/// `CAERadioTrackManager::InitialiseRadioStationID(eRadioID)` (0x4E8330): bytes +0xAD and +0xE9.
#[hook(addr = 0x4E8330, conv = thiscall, name = "CAERadioTrackManager::InitialiseRadioStationID", expect = "8A 44 24 04 88 81 AD 00 00 00")]
pub fn radio_initialise_radio_station_id(this: GPtr<u8>, id: u8) {
    wr::<u8>(this, 0xAD, id);
    wr::<u8>(this, 0xE9, id);
}

/// `CAERadioTrackManager::RetuneRadio(eRadioID)` (0x4E8290): station 12 becomes 13 while the word at
/// 0xB6B976 is 0; while the game is paused it stores the station at +0x78 and the pause-mode time at
/// +0x5C, else at +0x7C (sign-extended).
#[hook(addr = 0x4E8290, conv = thiscall, name = "CAERadioTrackManager::RetuneRadio", expect = "8A 44 24 04 3C 0C")]
pub fn radio_retune_radio(this: GPtr<u8>, station: u8) {
    let mut s = station;
    if s == 12 && g::<u16>(0xB6B976).read() == 0 {
        s = 13;
    }
    if game_paused() {
        wr::<i32>(this, 0x78, i32::from(s as i8));
        wr::<u32>(this, 0x5C, g::<u32>(TIMER_MS_PAUSE_MODE).read());
    } else {
        wr::<i32>(this, 0x7C, i32::from(s as i8));
    }
}

/// `CAERadioTrackManager::ResetStatistics()` (0x4E8200): stats bytes 0xB62C58..0xB62C5A = 0xFF,
/// 0xB62C5B..0xB62C71 = 0 (store order of the bytes: 0xB62C71, 5A, 59, 58, then 70 down to 5B).
#[hook(addr = 0x4E8200, conv = thiscall, name = "CAERadioTrackManager::ResetStatistics", expect = "32 C0 80 C9 FF")]
pub fn radio_reset_statistics(_this: GPtr<u8>) {
    g::<u8>(0xB62C71).write(0);
    g::<u8>(0xB62C5A).write(0xFF);
    g::<u8>(0xB62C59).write(0xFF);
    g::<u8>(0xB62C58).write(0xFF);
    for a in (0xB62C5B..=0xB62C70usize).rev() {
        g::<u8>(a).write(0);
    }
}

/// `CAEPoliceScannerAudioEntity::DisableScanner(bool, bool)` (0x4E71B0): sets `s_bScannerDisabled`; when
/// asked to stop and a dialogue is playing for a controlling entity, `StopScanner(arg2)` (the argument
/// dword passed through).
#[hook(addr = 0x4E71B0, conv = cdecl, name = "CAEPoliceScannerAudioEntity::DisableScanner", expect = "8A 44 24 04 84 C0")]
pub fn ps_disable_scanner(stop: u32, b: u32) {
    g::<u8>(PS_SCANNER_DISABLED).write(1);
    if stop as u8 != 0 && g::<i16>(PS_PLAYBACK_STATE).read() != 0 && g::<u32>(PS_CONTROLLING).read() != 0 {
        ps_stop_scanner(b);
    }
}

/// `CAEPoliceScannerAudioEntity::CanWePlayNewScannerDialogue()` (0x4E6C00): no playback, the next
/// dialogue time reached (unsigned), the byte 0xB6F065 clear and the scanner enabled.
#[hook(addr = 0x4E6C00, conv = cdecl, name = "CAEPoliceScannerAudioEntity::CanWePlayNewScannerDialogue", expect = "66 83 3D 4C 1D B6 00 00")]
pub fn ps_can_we_play_new_scanner_dialogue() -> u8 {
    if g::<i16>(PS_PLAYBACK_STATE).read() != 0 {
        return 0;
    }
    if g::<u32>(TIMER_MS).read() < g::<u32>(PS_NEXT_DIALOGUE_TIME).read() {
        return 0;
    }
    if g::<u8>(0xB6F065).read() != 0 {
        return 0;
    }
    u8::from(g::<u8>(PS_SCANNER_DISABLED).read() == 0)
}

orig! {
    /// `CAEAudioHardware::GetSoundLoadingStatus(uint16 bank, uint16 sound, int16 slot)` (0x4D8F00)
    fn hw_get_sound_loading_status(hw: GPtr<u8>, bank: u32, sound: u32, slot: u32) -> u8 = thiscall 0x4D8F00 => unported;
    /// `CAEAudioHardware::GetSoundBankLoadingStatus(uint16 bank, int16 slot)` (0x4D88D0)
    fn hw_get_sound_bank_loading_status(hw: GPtr<u8>, bank: u32, slot: u32) -> u8 = thiscall 0x4D88D0 => unported;
    /// `CAESoundManager::CancelSoundsInBankSlot(int16 slot, bool)` (0x4EFC60)
    fn sm_cancel_sounds_in_bank_slot(sm: GPtr<u8>, slot: u32, b: u32) = thiscall 0x4EFC60 => unported;
    /// `CAEScriptAudioEntity::ClearMissionAudio(uint8 slot)` (0x4EC040)
    fn script_clear_mission_audio_call(this: GPtr<u8>, slot: u32) = thiscall 0x4EC040 => script_clear_mission_audio;
    /// `CEntity::CleanUpOldReference(CEntity** ref)` (0x571A00)
    fn entity_clean_up_old_reference(entity: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
}

/// `CAEScriptAudioEntity::GetMissionAudioLoadingStatus(uint8 slot)` (0x4EBF60): 1 for slots >= 4 or a
/// negative bank (+0xA8 of the slot's 32-byte record); else the hardware's sound loading status when
/// the sound id (+0xAC) is >= 0, its bank loading status otherwise (bank slot `slot + 26`). The
/// original's pushed slot words carry leftover upper halves (EAX's / the caller's EDX); both callees
/// read only the low word.
#[hook(addr = 0x4EBF60, conv = thiscall, name = "CAEScriptAudioEntity::GetMissionAudioLoadingStatus", expect = "8A 54 24 04 80 FA 04")]
pub fn script_get_mission_audio_loading_status(this: GPtr<u8>, slot: u8) -> u8 {
    if slot >= 4 {
        return 1;
    }
    let rec = this.byte_add(isize::from(slot) * 32);
    if rd::<i32>(rec, 0xA8) < 0 {
        return 1;
    }
    let bank = u32::from(rd::<u16>(rec, 0xA8));
    let bank_slot = u32::from(slot) + 0x1A;
    if rd::<i32>(rec, 0xAC) >= 0 {
        let sound = u32::from(rd::<u16>(rec, 0xAC));
        hw_get_sound_loading_status(g(AE_AUDIO_HARDWARE), bank, sound, bank_slot)
    } else {
        hw_get_sound_bank_loading_status(g(AE_AUDIO_HARDWARE), bank, bank_slot)
    }
}

/// `CAEScriptAudioEntity::ClearMissionAudio(uint8 slot)` (0x4EC040): for slots < 4 cancels the sounds
/// of bank slot `slot + 26` (with the flag 1), then clears the slot record: +0x9C = 0, position
/// (+0x90, +0x94, +0x98) = -1000.0, +0xA0 = 0 (the bytes' store order). The original's bank-slot dword
/// carries EAX's leftover upper half; the callee reads only the low word.
#[hook(addr = 0x4EC040, conv = thiscall, name = "CAEScriptAudioEntity::ClearMissionAudio", expect = "83 EC 0C 53 8A 5C 24 14")]
pub fn script_clear_mission_audio(this: GPtr<u8>, slot: u32) {
    let slot = slot as u8;
    if slot >= 4 {
        return;
    }
    sm_cancel_sounds_in_bank_slot(g(crate::common::AE_SOUND_MANAGER), u32::from(slot) + 0x1A, 1);
    let rec = this.byte_add(isize::from(slot) * 32);
    const MINUS_1000: u32 = 0xC47A_0000;
    wr::<u32>(rec, 0x9C, 0);
    wr::<u32>(rec, 0x90, MINUS_1000);
    wr::<u32>(rec, 0xA0, 0);
    wr::<u32>(rec, 0x94, MINUS_1000);
    wr::<u32>(rec, 0x98, MINUS_1000);
}

/// `CAEScriptAudioEntity::Reset()` (0x4EC150): `ClearMissionAudio(0..3)`, then +0x7C = 0, +0x110 = 0,
/// +0x7D = 0, +0x8C = 2.0.
#[hook(addr = 0x4EC150, conv = thiscall, name = "CAEScriptAudioEntity::Reset", expect = "56 57 8B F9 33 F6")]
pub fn script_reset(this: GPtr<u8>) {
    for i in 0..4u32 {
        script_clear_mission_audio_call(this, i);
    }
    wr::<u8>(this, 0x7C, 0);
    wr::<u32>(this, 0x110, 0);
    wr::<u8>(this, 0x7D, 0);
    wr::<u32>(this, 0x8C, 0x4000_0000);
}

/// `CAESoundManager::Reset()` (0x4EF4D0): for each of the 300 sounds (+0xC, 0x74 bytes each) in use
/// (word +0x50): word +0x60 = 1 (stop request), then a registered physical entity (+0) is released
/// (`CleanUpOldReference(&sound.entity)`) and cleared.
#[hook(addr = 0x4EF4D0, conv = thiscall, name = "CAESoundManager::Reset", expect = "56 57 8D 71 0C")]
pub fn sound_manager_reset(this: GPtr<u8>) {
    let mut s = this.byte_add(0xC);
    for _ in 0..300 {
        if rd::<u16>(s, 0x50) != 0 {
            wr::<u16>(s, 0x60, 1);
            let e = rd::<GPtr<u8>>(s, 0);
            if !e.is_null() {
                entity_clean_up_old_reference(e, s);
                wr::<u32>(s, 0, 0);
            }
        }
        s = s.byte_add(0x74);
    }
}

/// `CAESoundManager::GetVirtualChannelForPhysicalChannel(int16 physical)` (0x4EF630): -1 when
/// `physical - first channel (+2)` is negative (16-bit) or not below the channel count (+0); else the
/// word entry of the table at +0x87F4. AX only (the upper half of EAX is path-dependent).
/// AX-only result (the upper EAX is the caller's leftover on some paths): hooked by PORT-AUDIO-3 through an
/// `sa_core::abi::Ax` wrapper in `port3_lead.rs`.
pub fn sm_get_virtual_channel_for_physical_channel(this: GPtr<u8>, physical: u32) -> u16 {
    let idx = (physical as u16).wrapping_sub(rd::<u16>(this, 2)) as i16;
    if idx < 0 || i32::from(idx) >= i32::from(rd::<u16>(this, 0)) {
        return 0xFFFF;
    }
    let table = rd::<GPtr<u8>>(this, 0x87F4);
    rd::<u16>(table, isize::from(idx) * 2)
}

/// Shifts a radio index history (`n` dwords, newest first) down by one and stores `index` at the front,
/// unless it is already the newest entry. Returns whether it stored.
fn push_history(base: usize, n: usize, index: u32) -> bool {
    if g::<u32>(base).read() == index {
        return false;
    }
    for i in (1..n).rev() {
        let v = g::<u32>(base + (i - 1) * 4).read();
        g::<u32>(base + i * 4).write(v);
    }
    g::<u32>(base).write(index);
    true
}

/// `CAERadioTrackManager::AddIdentIndexToHistory(eRadioID, int32)` (0x4E9720): the station's 8-entry
/// history at 0xB62980 (`this` unused).
#[hook(addr = 0x4E9720, conv = thiscall, name = "CAERadioTrackManager::AddIdentIndexToHistory", expect = "0F BE 4C 24 04")]
pub fn radio_add_ident_index_to_history(_this: GPtr<u8>, station: u32, index: u32) {
    let base = (0xB62980 + (i32::from(station as i8) << 5)) as u32 as usize;
    push_history(base, 8, index);
}

/// `CAERadioTrackManager::AddAdvertIndexToHistory(eRadioID, int32)` (0x4E9760): the station's 40-entry
/// history at 0xB620C0; after a store `this[station + 8] = 0`.
#[hook(addr = 0x4E9760, conv = thiscall, name = "CAERadioTrackManager::AddAdvertIndexToHistory", expect = "53 8B 5C 24 0C")]
pub fn radio_add_advert_index_to_history(this: GPtr<u8>, station: u32, index: u32) {
    let st = i32::from(station as i8);
    let base = (0xB620C0 + st * 160) as u32 as usize;
    if push_history(base, 40, index) {
        wr::<u8>(this, st as isize + 8, 0);
    }
}

/// `CAERadioTrackManager::AddDJBanterIndexToHistory(eRadioID, int32)` (0x4E97B0): the station's
/// 15-entry history at 0xB61D78; after a store `this[station + 8] = 0`.
#[hook(addr = 0x4E97B0, conv = thiscall, name = "CAERadioTrackManager::AddDJBanterIndexToHistory", expect = "53 8B 5C 24 0C")]
pub fn radio_add_dj_banter_index_to_history(this: GPtr<u8>, station: u32, index: u32) {
    let st = i32::from(station as i8);
    let base = (0xB61D78 + st * 60) as u32 as usize;
    if push_history(base, 15, index) {
        wr::<u8>(this, st as isize + 8, 0);
    }
}

/// `CAEMP3BankLoader::GetSoundBankLoadingStatus(uint16 bank, int16 slot)` (0x4E0250): initialised (+0x14)
/// and the slot's bank (`(int16)` word +0x10 of the 0x12D4-byte slot record at `*this`) equals `bank`.
#[hook(addr = 0x4E0250, conv = thiscall, name = "CAEMP3BankLoader::GetSoundBankLoadingStatus", expect = "8A 41 14 84 C0")]
pub fn mp3_get_sound_bank_loading_status(this: GPtr<u8>, bank: u32, slot: u32) -> u8 {
    if rd::<u8>(this, 0x14) == 0 {
        return 0;
    }
    let slots = rd::<GPtr<u8>>(this, 0);
    let rec = slots.byte_add((i32::from(slot as i16).wrapping_mul(0x12D4)) as isize);
    u8::from(i32::from(rd::<i16>(rec, 0x10)) == i32::from(bank as u16))
}

/// `CAEMP3BankLoader::GetSoundLoadingStatus(uint16 bank, uint16 sound, int16 slot)` (0x4E0400): the bank
/// is loaded in the slot (as `GetSoundBankLoadingStatus`) and the slot's sound word (+0x66C + 2*slot)
/// equals `sound` (signed vs zero-extended compares, as the bytes).
#[hook(addr = 0x4E0400, conv = thiscall, name = "CAEMP3BankLoader::GetSoundLoadingStatus", expect = "8A 41 14 84 C0")]
pub fn mp3_get_sound_loading_status(this: GPtr<u8>, bank: u32, sound: u32, slot: u32) -> u8 {
    if rd::<u8>(this, 0x14) == 0 {
        return 0;
    }
    let s = i32::from(slot as i16);
    let slots = rd::<GPtr<u8>>(this, 0);
    let rec = slots.byte_add(s.wrapping_mul(0x12D4) as isize);
    if i32::from(rd::<i16>(rec, 0x10)) != i32::from(bank as u16) {
        return 0;
    }
    u8::from(i32::from(rd::<i16>(this, (0x66C + s * 2) as isize)) == i32::from(sound as u16))
}

/// Slot record (0x12D4 bytes) `slot` of the MP3 bank loader (array at `*this`).
fn mp3_slot(this: GPtr<u8>, slot: i32) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0).byte_add(slot.wrapping_mul(0x12D4) as isize)
}

/// `CAEBankLoader::GetBankLookup(uint16 bank)` (0x4E01B0): `&lookups(+4)[bank]` (12 bytes each) when
/// initialised (+0x14) and `bank < (int16)` count (+0xE), else null.
#[hook(addr = 0x4E01B0, conv = thiscall, name = "CAEBankLoader::GetBankLookup", expect = "8A 41 14 84 C0")]
pub fn bank_get_bank_lookup(this: GPtr<u8>, bank: u32) -> u32 {
    if rd::<u8>(this, 0x14) == 0 {
        return 0;
    }
    let b = i32::from(bank as u16);
    if b >= i32::from(rd::<i16>(this, 0xE)) {
        return 0;
    }
    rd::<u32>(this, 4).wrapping_add((b * 12) as u32)
}

/// `CAEMP3BankLoader::GetSoundHeadroom(uint16 sound, int16 slot)` (0x4E01E0): `[0x858B50]` when not
/// initialised; else the sound's int16 headroom (+0x1E of its 12-byte entry in the slot record) * 0.01
/// (`[0x858C58]`, ST0). The original also stores the int in its own argument slot (`[esp+8]`, popped by
/// the return; not modelled).
#[hook(addr = 0x4E01E0, conv = thiscall, name = "CAEMP3BankLoader::GetSoundHeadroom", expect = "8A 41 14 84 C0")]
pub fn mp3_get_sound_headroom(this: GPtr<u8>, sound: u32, slot: u32) -> f32 {
    if rd::<u8>(this, 0x14) == 0 {
        return g::<f32>(0x858B50).read();
    }
    let rec = mp3_slot(this, i32::from(slot as i16));
    let v = i32::from(rd::<i16>(rec, i32::from(sound as u16) as isize * 12 + 0x1E));
    crate::common::xi(v).fmul_m(g::<f32>(0x858C58).read()).to_f32()
}

/// `CAEMP3BankLoader::IsSoundBankLoaded(uint16 bank, int16 slot)` (0x4E0220): initialised and the slot's
/// `(int16)` bank word (+0x10) equals `bank`.
#[hook(addr = 0x4E0220, conv = thiscall, name = "CAEMP3BankLoader::IsSoundBankLoaded", expect = "8A 41 14 84 C0")]
pub fn mp3_is_sound_bank_loaded(this: GPtr<u8>, bank: u32, slot: u32) -> u8 {
    if rd::<u8>(this, 0x14) == 0 {
        return 0;
    }
    let rec = mp3_slot(this, i32::from(slot as i16));
    u8::from(i32::from(rd::<i16>(rec, 0x10)) == i32::from(bank as u16))
}

/// `CAEMP3BankLoader::GetLoopOffset(uint16 sound, int16 slot)` (0x4E0380): -1 when not initialised, else
/// the dword at +0x18 of the sound's 12-byte entry in the slot record.
#[hook(addr = 0x4E0380, conv = thiscall, name = "CAEMP3BankLoader::GetLoopOffset", expect = "8A 41 14 84 C0")]
pub fn mp3_get_loop_offset(this: GPtr<u8>, sound: u32, slot: u32) -> u32 {
    if rd::<u8>(this, 0x14) == 0 {
        return 0xFFFF_FFFF;
    }
    let rec = mp3_slot(this, i32::from(slot as i16));
    rd::<u32>(rec, (i32::from(sound as u16) + 2) as isize * 12)
}

/// `CAEMP3BankLoader::IsSoundLoaded(uint16 bank, uint16 sound, int16 slot)` (0x4E03B0): same bytes as
/// `GetSoundLoadingStatus` (0x4E0400).
#[hook(addr = 0x4E03B0, conv = thiscall, name = "CAEMP3BankLoader::IsSoundLoaded", expect = "8A 41 14 84 C0")]
pub fn mp3_is_sound_loaded(this: GPtr<u8>, bank: u32, sound: u32, slot: u32) -> u8 {
    mp3_get_sound_loading_status(this, bank, sound, slot)
}

/// `CAEMP3BankLoader::GetBankSlotBuffer(uint16 slot, uint32* size)` (0x4DFDE0): null (and `*size`
/// untouched) when not initialised; null and `*size = 0` when `slot >= (int16)` count (+0xC); else
/// `*size` = the slot record's +4 and the result its +0 (re-read after the store) + the buffer base +0x1C.
#[hook(addr = 0x4DFDE0, conv = thiscall, name = "CAEMP3BankLoader::GetBankSlotBuffer", expect = "8A 41 14 84 C0")]
pub fn mp3_get_bank_slot_buffer(this: GPtr<u8>, slot: u32, size: GPtr<u32>) -> u32 {
    if rd::<u8>(this, 0x14) == 0 {
        return 0;
    }
    let s = i32::from(slot as u16);
    if s >= i32::from(rd::<i16>(this, 0xC)) {
        size.write(0);
        return 0;
    }
    size.write(rd::<u32>(mp3_slot(this, s), 4));
    rd::<u32>(mp3_slot(this, s), 0).wrapping_add(rd::<u32>(this, 0x1C))
}

/// `FrontEndMenuManager.m_nRadioMode` (0xBA67F8)
const MENU_RADIO_MODE: usize = 0xBA67F8;
/// `CAEUserRadioTrackManager` current user track index (0xBA67F4, in the menu manager)
const USER_TRACK_INDEX: usize = 0xBA67F4;

/// `CAEUserRadioTrackManager::SelectUserTrackIndex()` (0x4F3250): -1 without tracks (word +6) or for
/// play modes other than 0..2; mode 2 (sequential): `(index + 1) % count`; modes 0/1 (random):
/// `CAEAudioUtility::GetRandomNumberInRange(0, count - 1)` redrawn while it repeats the current index and
/// more than one track exists. The result is stored at 0xBA67F4.
#[hook(addr = 0x4F3250, conv = thiscall, name = "CAEUserRadioTrackManager::SelectUserTrackIndex", expect = "56 8B F1 66 8B 4E 06")]
pub fn user_select_user_track_index(this: GPtr<u8>) -> u32 {
    let count = rd::<u16>(this, 6);
    if count == 0 {
        return 0xFFFF_FFFF;
    }
    let mode = g::<i8>(MENU_RADIO_MODE).read();
    if mode < 0 {
        return 0xFFFF_FFFF;
    }
    if mode <= 1 {
        let mut r;
        let mut n = count;
        loop {
            r = crate::common::ae_random_int(0, i32::from(n) - 1) as u32;
            let c = rd::<u16>(this, 6);
            if c <= 1 || r != g::<u32>(USER_TRACK_INDEX).read() {
                break;
            }
            n = c;
        }
        g::<u32>(USER_TRACK_INDEX).write(r);
        return r;
    }
    if mode != 2 {
        return 0xFFFF_FFFF;
    }
    let r = g::<u32>(USER_TRACK_INDEX).read().wrapping_add(1) % u32::from(count);
    g::<u32>(USER_TRACK_INDEX).write(r);
    r
}

/// `CAEPoliceScannerAudioEntity` statics (gta-reversed names where known)
const PS_STOPPING: usize = 0xB61CFC; // byte
const PS_CF8: usize = 0xB61CF8; // dword
const PS_SOUND: usize = 0xB61D00; // CAESound*
const PS_D04: usize = 0xB61D04; // word
const PS_D08: usize = 0xB61D08; // dword (time)
const PS_D0C: usize = 0xB61D0C; // dword (time)
const PS_SLOT_LIST: usize = 0xB61D10; // int16 (*)[2] list pointer
const PS_SLOTS: usize = 0xB61D14; // int16[5..]
const PS_LIST_A: usize = 0xB61D34; // 5 dwords (s_SlotState / bank-sound pairs)
const PS_LIST_B: usize = 0xB61D20; // 5 dwords

orig! {
    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float volumeChange, float speed)` (0x506EA0)
    fn engine_report_frontend_audio_event(engine: GPtr<u8>, event: u32, volume_bits: u32, speed_bits: u32) = thiscall 0x506EA0 => unported;
    /// `CAESound::StopSoundAndForget()` (0x4EF850)
    fn sound_stop_sound_and_forget(sound: GPtr<u8>) = thiscall 0x4EF850 => unported;
    /// `CAESound::SetFlags(uint16 flags, uint16 value)` (0x4EF2B0)
    fn sound_set_flags(sound: GPtr<u8>, flags: u32, value: u32) = thiscall 0x4EF2B0 => unported;
    /// `CAEPoliceScannerAudioEntity::PlayLoadedDialogue()` (0x4E6F60)
    fn ps_play_loaded_dialogue(this: GPtr<u8>) = thiscall 0x4E6F60 => unported;
    /// `CAEPoliceScannerAudioEntity::FinishedPlayingScannerDialogue()` (0x4E6C30)
    fn ps_finished_playing_call() = cdecl 0x4E6C30 => ps_finished_playing_scanner_dialogue;
    /// `CAEPoliceScannerAudioEntity::PopulateScannerDialogueLists(const int16* banks, const int16* sounds)` (0x4E6B60)
    fn ps_populate_lists_call(a: u32, b: u32) = cdecl 0x4E6B60 => ps_populate_scanner_dialogue_lists;
    /// `CAEPoliceScannerAudioEntity::PrepSlots()` (0x4E6BC0)
    fn ps_prep_slots_call() = cdecl 0x4E6BC0 => ps_prep_slots;
}

/// `CAEPoliceScannerAudioEntity::PrepSlots()` (0x4E6BC0): when a list is set (0xB61D10), for each of its 5
/// int16 pairs the slot word (0xB61D14 + 2i) is 1 if either word is negative, else 0.
#[hook(addr = 0x4E6BC0, conv = cdecl, name = "CAEPoliceScannerAudioEntity::PrepSlots", expect = "A1 10 1D B6 00")]
pub fn ps_prep_slots() {
    let list = g::<u32>(PS_SLOT_LIST).read();
    if list == 0 {
        return;
    }
    for i in 0..5usize {
        let e = g::<i16>(list as usize + i * 4);
        let neg = e.read() < 0 || e.byte_add(2).read() < 0;
        g::<u16>(PS_SLOTS + i * 2).write(u16::from(neg));
    }
}

/// `CAEPoliceScannerAudioEntity::FinishedPlayingScannerDialogue()` (0x4E6C30): reports frontend event
/// 0x30 (volume 0.0, speed 1.0), then resets the scanner state: slot words (0xB61D14..0xB61D1D) = 1, next
/// dialogue time = now + `[0x8C8140]`, state/controller/list/flags = 0 and the two dialogue lists
/// (0xB61D20, 0xB61D34; 5 int16 pairs each) = -1, in the bytes' store order.
#[hook(addr = 0x4E6C30, conv = cdecl, name = "CAEPoliceScannerAudioEntity::FinishedPlayingScannerDialogue", expect = "53 68 00 00 80 3F")]
pub fn ps_finished_playing_scanner_dialogue() {
    engine_report_frontend_audio_event(g(crate::common::AUDIO_ENGINE), 0x30, 0, 0x3F80_0000);
    let delay = g::<u32>(0x8C8140).read();
    let now = g::<u32>(TIMER_MS).read();
    g::<u32>(PS_SLOTS).write(0x10001);
    g::<u32>(PS_SLOTS + 4).write(0x10001);
    g::<u32>(PS_NEXT_DIALOGUE_TIME).write(now.wrapping_add(delay));
    g::<u16>(PS_PLAYBACK_STATE).write(0);
    g::<u32>(PS_CONTROLLING).write(0);
    g::<u32>(PS_SLOT_LIST).write(0);
    g::<u8>(PS_STOPPING).write(0);
    g::<u32>(PS_CF8).write(0);
    g::<u16>(PS_SLOTS + 8).write(1);
    for i in 0..5usize {
        g::<u16>(PS_LIST_A + 2 + i * 4).write(0xFFFF);
        g::<u16>(PS_LIST_A + i * 4).write(0xFFFF);
        g::<u16>(PS_LIST_B + 2 + i * 4).write(0xFFFF);
        g::<u16>(PS_LIST_B + i * 4).write(0xFFFF);
    }
}

/// `CAEPoliceScannerAudioEntity::PopulateScannerDialogueLists(a, b)` (0x4E6B60): while no dialogue plays,
/// copies 5 dwords from `a` to 0xB61D34 and from `b` to 0xB61D20 (interleaved per index, as the bytes).
#[hook(addr = 0x4E6B60, conv = cdecl, name = "CAEPoliceScannerAudioEntity::PopulateScannerDialogueLists", expect = "66 83 3D 4C 1D B6 00 00")]
pub fn ps_populate_scanner_dialogue_lists(a: u32, b: u32) {
    if g::<i16>(PS_PLAYBACK_STATE).read() != 0 {
        return;
    }
    for i in 0..5u32 {
        let v = g::<u32>(a.wrapping_add(i * 4) as usize).read();
        g::<u32>(PS_LIST_A + i as usize * 4).write(v);
        let w = g::<u32>(b.wrapping_add(i * 4) as usize).read();
        g::<u32>(PS_LIST_B + i as usize * 4).write(w);
    }
}

/// `CAEPoliceScannerAudioEntity::Reset()` (0x4E6E90): while a dialogue plays, sets the stopping flag,
/// stops and forgets the scanner sound (0xB61D00) and runs `FinishedPlayingScannerDialogue`; then (tail
/// jump) runs it once more.
#[hook(addr = 0x4E6E90, conv = cdecl, name = "CAEPoliceScannerAudioEntity::Reset", expect = "66 83 3D 4C 1D B6 00 00")]
pub fn ps_reset() {
    if g::<i16>(PS_PLAYBACK_STATE).read() != 0 {
        let sound = g::<GPtr<u8>>(PS_SOUND).read();
        g::<u8>(PS_STOPPING).write(1);
        if !sound.is_null() {
            sound_stop_sound_and_forget(sound);
            g::<u32>(PS_SOUND).write(0);
        }
        ps_finished_playing_call();
    }
    ps_finished_playing_call();
}

/// `CAEPoliceScannerAudioEntity::PlayPoliceScannerDialogue(a, b)` (0x4E6ED0): when no dialogue plays, the
/// next dialogue time is reached, the byte 0xB6F065 is clear and the scanner is enabled: fills the lists,
/// takes control (`this`), state 2 and the two start times (now + `[0x8C8144]` / `[0x8C8148]`, `now` read
/// before the calls).
#[hook(addr = 0x4E6ED0, conv = thiscall, name = "CAEPoliceScannerAudioEntity::PlayPoliceScannerDialogue", expect = "66 83 3D 4C 1D B6 00 00")]
pub fn ps_play_police_scanner_dialogue(this: GPtr<u8>, a: u32, b: u32) {
    if g::<i16>(PS_PLAYBACK_STATE).read() != 0 {
        return;
    }
    let now = g::<u32>(TIMER_MS).read();
    if now < g::<u32>(PS_NEXT_DIALOGUE_TIME).read()
        || g::<u8>(0xB6F065).read() != 0
        || g::<u8>(PS_SCANNER_DISABLED).read() != 0
    {
        return;
    }
    ps_populate_lists_call(a, b);
    g::<GPtr<u8>>(PS_CONTROLLING).write(this);
    g::<u16>(PS_D04).write(0);
    g::<u32>(PS_SLOT_LIST).write(PS_LIST_A as u32);
    ps_prep_slots_call();
    let d0 = g::<u32>(0x8C8144).read();
    let d1 = g::<u32>(0x8C8148).read();
    g::<u16>(PS_PLAYBACK_STATE).write(2);
    g::<u32>(PS_D0C).write(now.wrapping_add(d0));
    g::<u32>(PS_D08).write(now.wrapping_add(d1));
}

/// `CAEPoliceScannerAudioEntity::UpdateParameters(CAESound*, int16 curPlayPos)` (0x4E7590): sound ended
/// (-1): forget it, then either play the next loaded dialogue, or (stopping and still playing) finish the
/// dialogue. Playing while stopping: volume (+0x14) -= `[0x8C8150]`. Otherwise, past `length (+0x70) -
/// [0x8C814C]` and not sound 0x25: `SetFlags(4, 0)`, forget the sound, play the next loaded dialogue.
#[hook(addr = 0x4E7590, conv = thiscall, name = "CAEPoliceScannerAudioEntity::UpdateParameters", expect = "66 8B 54 24 08")]
pub fn ps_update_parameters(this: GPtr<u8>, sound: GPtr<u8>, pos: u32) {
    let pos = pos as i16;
    let stopping = g::<u8>(PS_STOPPING).read();
    if pos == -1 {
        g::<u32>(PS_SOUND).write(0);
        if stopping == 0 {
            ps_play_loaded_dialogue(this);
            return;
        }
        if g::<i16>(PS_PLAYBACK_STATE).read() == 0 {
            return;
        }
        g::<u8>(PS_STOPPING).write(1);
        ps_finished_playing_call();
        return;
    }
    if stopping != 0 {
        let v = x(rd::<f32>(sound, 0x14)).fsub_m(g::<f32>(0x8C8150).read());
        wr::<f32>(sound, 0x14, v.to_f32());
        return;
    }
    let len = rd::<i16>(sound, 0x70);
    if len <= 0 {
        return;
    }
    if i32::from(pos) <= i32::from(len) - i32::from(g::<i16>(0x8C814C).read()) {
        return;
    }
    if rd::<u16>(sound, 0) == 0x25 {
        return;
    }
    sound_set_flags(sound, 4, 0);
    g::<u32>(PS_SOUND).write(0);
    ps_play_loaded_dialogue(this);
}

orig! {
    /// `CAEAudioEnvironment::GetPositionRelativeToCamera(CVector* out, const CVector* pos)` (0x4D80B0)
    fn env_get_position_relative_to_camera(out: u32, pos: GPtr<u8>) = cdecl 0x4D80B0 => unported;
    /// `CAEAudioUtility::GetRandomNumberInRange<float>(float, float)` (0x4D9C50, ST0)
    fn ae_random_float(min: f32, max: f32) -> f32 = cdecl 0x4D9C50 => crate::utility::random_float_f32;
    /// `CAERadioTrackManager::StartRadio(eRadioID, eBassSetting, float bassGain, uint8)` (0x4EB3C0)
    fn radio_start_radio_manual(this: GPtr<u8>, station: u32, bass: u32, gain_bits: u32, b: u32) = thiscall 0x4EB3C0 => crate::radio::start_radio_manual_port;
    /// `CAudioEngine::StopRadio(tRadioSettings*, bool)` (0x506F70)
    fn engine_stop_radio(engine: GPtr<u8>, settings: u32, b: u32) = thiscall 0x506F70 => unported;
}

/// `CAESound::UnregisterWithPhysicalEntity()` (0x4EF1A0): releases the reference at +8
/// (`CleanUpOldReference(&m_pPhysicalEntity)`) and clears it.
#[hook(addr = 0x4EF1A0, conv = thiscall, name = "CAESound::UnregisterWithPhysicalEntity", expect = "56 8D 71 08 8B 0E")]
pub fn sound_unregister_with_physical_entity(this: GPtr<u8>) {
    let r = this.byte_add(8);
    let e = rd::<GPtr<u8>>(r, 0);
    if !e.is_null() {
        entity_clean_up_old_reference(e, r);
        wr::<u32>(r, 0, 0);
    }
}

/// `CAESound::UpdatePlayTime(int16 length, int16 loopStart, int16 timeElapsed)` (0x4EF2E0): length at
/// +0x70; unless the sound is paused (+0x5E): when it is stopped (+0x68) the play time (+0x5C) is -1,
/// else `ftol(elapsed * speed(+0x64) + playTime)` (x87, PC=24); at or past the length it wraps to
/// `playTime % length + loopStart` or becomes -1 without a loop. The original reuses its argument slot
/// for the `fild` operands (not modelled). A zero length past the check faults in the original (`idiv`);
/// the port stores `loopStart` instead.
#[hook(addr = 0x4EF2E0, conv = thiscall, name = "CAESound::UpdatePlayTime", expect = "56 8B F1 66 83 7E 5E 00")]
pub fn sound_update_play_time(this: GPtr<u8>, length: u32, loop_start: u32, elapsed: u32) {
    let len = length as i16;
    wr::<i16>(this, 0x70, len);
    if rd::<u16>(this, 0x5E) != 0 {
        return;
    }
    if rd::<u16>(this, 0x68) != 0 {
        wr::<u16>(this, 0x5C, 0xFFFF);
        return;
    }
    let v = crate::common::xi(i32::from(elapsed as i16))
        .fmul_m(rd::<f32>(this, 0x64))
        .fadd(crate::common::xi(i32::from(rd::<i16>(this, 0x5C))));
    let t = crate::common::ftol32(v) as i16;
    wr::<i16>(this, 0x5C, t);
    if t < len {
        return;
    }
    if loop_start as u16 == 0xFFFF {
        wr::<u16>(this, 0x5C, 0xFFFF);
        return;
    }
    let rem = i32::from(t).checked_rem(i32::from(len)).unwrap_or(0);
    wr::<u16>(this, 0x5C, (rem as u32).wrapping_add(loop_start) as u16);
}

/// `CAESound::CalculateFrequency()` (0x4EF390): with a frequency variance (+0x20) above `[0x858B50]` and
/// below the frequency (+0x1C): `freq = GetRandomNumberInRange<float>(var * [0x858C1C], var) + freq`,
/// else `freq`; stored at +0x64.
#[hook(addr = 0x4EF390, conv = thiscall, name = "CAESound::CalculateFrequency", expect = "56 8B F1 D9 46 20")]
pub fn sound_calculate_frequency(this: GPtr<u8>) {
    let var = rd::<f32>(this, 0x20);
    let c1 = x(var).fcom(x(g::<f32>(0x858B50).read()));
    if !c1.test_any(0x41) {
        let c2 = x(var).fcom(x(rd::<f32>(this, 0x1C)));
        if !c2.test_parity(0x05) {
            let lo = x(var).fmul_m(g::<f32>(0x858C1C).read()).to_f32();
            let r = ae_random_float(lo, var);
            let f = x(r).fadd_m(rd::<f32>(this, 0x1C));
            wr::<f32>(this, 0x64, f.to_f32());
            return;
        }
    }
    wr::<u32>(this, 0x64, rd::<u32>(this, 0x1C));
}

/// `CAESound::UpdateFrequency()` (0x4EF3E0): when the variance (+0x20) equals `[0x858B50]` the speed
/// (+0x64) becomes the frequency (+0x1C).
#[hook(addr = 0x4EF3E0, conv = thiscall, name = "CAESound::UpdateFrequency", expect = "D9 41 20 D8 1D 50 8B 85 00")]
pub fn sound_update_frequency(this: GPtr<u8>) {
    let c = x(rd::<f32>(this, 0x20)).fcom(x(g::<f32>(0x858B50).read()));
    if !c.test_parity(0x44) {
        wr::<u32>(this, 0x64, rd::<u32>(this, 0x1C));
    }
}

/// `CAESound::GetRelativePosition(CVector* out)` (0x4EF350, catalogue `-orginal`): the position (+0x24)
/// itself when flag bit 0 of +0x56 (relative) is set, else `GetPositionRelativeToCamera(out, &pos)`.
#[hook(addr = 0x4EF350, conv = thiscall, name = "CAESound::GetRelativePosition-orginal", expect = "8A 41 56 83 C1 24")]
pub fn sound_get_relative_position(this: GPtr<u8>, out: u32) {
    let pos = this.byte_add(0x24);
    if rd::<u8>(this, 0x56) & 1 == 0 {
        env_get_position_relative_to_camera(out, pos);
        return;
    }
    let o = g::<u8>(out as usize);
    wr::<u32>(o, 0, rd::<u32>(pos, 0));
    wr::<u32>(o, 4, rd::<u32>(pos, 4));
    wr::<u32>(o, 8, rd::<u32>(pos, 8));
}

/// `CAECutsceneTrackManager::Initialise(int32 slot)` (0x5B9630): slot at +4, flags +0/+1/+0x1C = 0,
/// +0x14/+0x18 = 0, status +8 = 8, track ids +0xC/+0x10 = -1; returns 1 (EAX).
#[hook(addr = 0x5B9630, conv = thiscall, name = "CAECutsceneTrackManager::Initialise", expect = "8B 44 24 04 89 41 04")]
pub fn cutscene_initialise(this: GPtr<u8>, slot: u32) -> u32 {
    wr::<u32>(this, 4, slot);
    wr::<u8>(this, 0, 0);
    wr::<u8>(this, 1, 0);
    wr::<u8>(this, 0x1C, 0);
    wr::<u32>(this, 0x14, 0);
    wr::<u32>(this, 0x18, 0);
    wr::<u32>(this, 8, 8);
    wr::<i32>(this, 0xC, -1);
    wr::<i32>(this, 0x10, -1);
    1
}

/// `CAERadioTrackManager::CheckForStationRetuneDuringPause()` (0x4EB890): with a station (+0xE9) or the
/// radio fully off (mode +0x68 == 7, +0, +0x6C, +0x70 all 0): a pending retune (+0x78) to 13 (off)
/// reports events 0x26 and 0x23 and restarts on 13; any other pending station reports 0x25 and clears the
/// station (when it was 13) or stops the radio (`CAudioEngine::StopRadio(null, true)`), reports 0x22 and,
/// 700 ms (pause-mode clock) after the retune time +0x5C, starts the pending station. Each restart uses
/// the bass byte +0xEA and gain +0xEC and clears +0x78 to -1.
#[hook(addr = 0x4EB890, conv = thiscall, name = "CAERadioTrackManager::CheckForStationRetuneDuringPause", expect = "56 8B F1 8A 8E E9 00 00 00")]
pub fn radio_check_for_station_retune_during_pause(this: GPtr<u8>) {
    let engine = g::<u8>(crate::common::AUDIO_ENGINE);
    let cur = rd::<u8>(this, 0xE9);
    if cur == 0
        && (rd::<u32>(this, 0x68) != 7
            || rd::<u8>(this, 0) != 0
            || rd::<u32>(this, 0x6C) != 0
            || rd::<u32>(this, 0x70) != 0)
    {
        return;
    }
    let pending = rd::<i32>(this, 0x78);
    if pending == 13 {
        engine_report_frontend_audio_event(engine, 0x26, 0, 0x3F80_0000);
        engine_report_frontend_audio_event(engine, 0x23, 0, 0x3F80_0000);
        let gain = rd::<u32>(this, 0xEC);
        let bass = u32::from(rd::<u8>(this, 0xEA));
        radio_start_radio_manual(this, 13, bass, gain, 0);
        wr::<i32>(this, 0x78, -1);
        return;
    }
    if pending < 0 {
        return;
    }
    if cur == 13 {
        engine_report_frontend_audio_event(engine, 0x25, 0, 0x3F80_0000);
        wr::<u8>(this, 0xE9, 0xFF);
    } else {
        engine_stop_radio(engine, 0, 1);
    }
    engine_report_frontend_audio_event(engine, 0x22, 0, 0x3F80_0000);
    let due = rd::<u32>(this, 0x5C).wrapping_add(700);
    if g::<u32>(TIMER_MS_PAUSE_MODE).read() > due {
        let gain = rd::<u32>(this, 0xEC);
        let bass = u32::from(rd::<u8>(this, 0xEA));
        let st = u32::from(rd::<u8>(this, 0x78));
        radio_start_radio_manual(this, st, bass, gain, 0);
        wr::<i32>(this, 0x78, -1);
    }
}

/// `CAEScriptAudioEntity::GetMissionAudioPosition(uint8 slot)` (0x4EC4D0): for an attached entity
/// (+0x9C of the slot's record) its matrix position (`m_matrix + 0x30`) or else its placement position
/// (+4); otherwise the record's position (+0x90) unless it is the "unset" (-1000, -1000, -1000, bitwise)
/// or all three components compare equal to `[0x858B50]` (0.0), which give null.
#[hook(addr = 0x4EC4D0, conv = thiscall, name = "CAEScriptAudioEntity::GetMissionAudioPosition", expect = "0F B6 44 24 04")]
pub fn script_get_mission_audio_position(this: GPtr<u8>, slot: u32) -> u32 {
    let rec = this.byte_add(isize::from(slot as u8) * 32);
    let ent = rd::<u32>(rec, 0x9C);
    if ent != 0 {
        let m = rd::<u32>(g::<u8>(ent as usize), 0x14);
        return if m != 0 {
            m.wrapping_add(0x30)
        } else {
            ent.wrapping_add(4)
        };
    }
    let p = rec.byte_add(0x90);
    const UNSET: u32 = 0xC47A_0000;
    if rd::<u32>(p, 0) == UNSET && rd::<u32>(p, 4) == UNSET && rd::<u32>(p, 8) == UNSET {
        return 0;
    }
    let zero = x(g::<f32>(0x858B50).read());
    for off in [0isize, 4, 8] {
        if x(rd::<f32>(p, off)).fcom(zero).test_parity(0x44) {
            return p.addr() as u32;
        }
    }
    0
}

/// `CAERadioTrackManager::ChooseAdvertIndex(eRadioID)` (0x4E9570): draws
/// `GetRandomNumberInRange([0x8C8B88], [0x8C8B8C])` (bounds re-read per draw) until the index is neither in
/// the station's 23-entry exclusion list (0x8CADD0 + station * 0x5C) nor in its 40-entry advert history
/// (0xB620C0 + station * 160). `this` is unused.
#[hook(addr = 0x4E9570, conv = thiscall, name = "CAERadioTrackManager::ChooseAdvertIndex", expect = "56 0F BE 74 24 08")]
pub fn radio_choose_advert_index(_this: GPtr<u8>, station: u32) -> u32 {
    let st = i32::from(station as i8);
    let excl = (0x8CADD0 + st * 0x5C) as u32 as usize;
    let hist = (0xB620C0 + st * 160) as u32 as usize;
    'draw: loop {
        let min = g::<i32>(0x8C8B88).read();
        let max = g::<i32>(0x8C8B8C).read();
        let r = crate::common::ae_random_int(min, max) as u32;
        for i in 0..23 {
            if g::<u32>(excl + i * 4).read() == r {
                continue 'draw;
            }
        }
        for i in 0..40 {
            if g::<u32>(hist + i * 4).read() == r {
                continue 'draw;
            }
        }
        return r;
    }
}

/// `CClock::ms_nGameClockDays` (0xB70154) and
/// `CClock::ms_nGameClockHours` (0xB70153)
const CLOCK_DAYS: usize = 0xB70154;
const CLOCK_HOURS: usize = 0xB70153;

/// `CAERadioTrackManager::StopRadio(tRadioSettings* settings, bool)` (0x4E9820; the entry chunk continues
/// at 0x406AAD). For modes 0..2: mode 3 unless paused without the flag; the current station's 44-byte
/// record (+0x100 + station * 0x2C, station re-read before every store as the bytes do) is reset and
/// stamped (time, clock), its listen time (+0x18 + 4 * station) accumulated, and the record filled from
/// the playing tracks according to the track type +0xF5 (jump table 0x4E9D8C; 13 instead clears
/// +0x33C). Then +0/+3 = 0; paused without the flag: +0x78 = -1, +0x5C = 0. With `settings`: reset the
/// queue, report event 0x23, store station (-1 becomes 13) and bass into `settings` (+0x1A, +6), record
/// the stop time/clock/station unless the radio is fully off, and replace station 0 by
/// `GetRandomNumberInRange(1, 13)`. Without `settings` and with the flag: reset the queue counters.
#[hook(addr = 0x4E9820, conv = thiscall, name = "CAERadioTrackManager::StopRadio", expect = "53 55 56 8B F1")]
pub fn radio_stop_radio(this: GPtr<u8>, settings: GPtr<u8>, flag: u32) {
    let flag = flag as u8;
    let st = || i32::from(rd::<i8>(this, 0xE9));
    let rec = |off: i32| (st() * 0x2C + off) as isize;
    let mode = rd::<u32>(this, 0x68);
    if mode <= 2 {
        if !game_paused() || flag != 0 {
            wr::<u32>(this, 0x68, 3);
        }
        wr::<u32>(this, rec(0x100), 0);
        wr::<i32>(this, rec(0x114), -1);
        wr::<i32>(this, rec(0x118), -1);
        wr::<u8>(this, rec(0x124), 6);
        wr::<u32>(this, rec(0x104), 0);
        wr::<i32>(this, rec(0x114), -1);
        wr::<i32>(this, rec(0x11C), -1);
        wr::<u8>(this, rec(0x125), 6);
        wr::<u32>(this, rec(0x108), 0);
        wr::<i32>(this, rec(0x114), -1);
        wr::<i32>(this, rec(0x120), -1);
        wr::<u8>(this, rec(0x126), 6);
        wr::<u32>(this, rec(0x110), g::<u32>(TIMER_MS).read());
        wr::<u8>(this, rec(0x127), g::<u8>(CLOCK_DAYS).read());
        wr::<u8>(this, rec(0x128), g::<u8>(CLOCK_HOURS).read());
        let s = rd::<u8>(this, 0xE9);
        let d = rd::<i32>(this, rec(0x10C));
        if d >= 0 && s != 0 && s != 13 {
            let off = 0x18 + i32::from(s as i8) * 4;
            let v = rd::<u32>(this, off as isize)
                .wrapping_add(g::<u32>(TIMER_MS_PAUSE_MODE).read().wrapping_sub(d as u32));
            wr::<u32>(this, off as isize, v);
        }
        if rd::<u8>(this, 0xE9) == 13 {
            wr::<u32>(this, 0x33C, 0);
        } else {
            let v = rd::<u32>(this, 0xE4)
                .wrapping_sub(rd::<u32>(this, 0xE0))
                .wrapping_sub(100);
            wr::<u32>(this, rec(0x100), v);
            let ty = i32::from(rd::<i8>(this, 0xF5)) as u32;
            // The record is filled from the queue: ids +0xC4/+0xC8/+0xCC/+0xD8/+0xE0, types +0xF0..+0xF2/+0xF5.
            let fill_from_queue = |three: bool| {
                let first = rd::<u32>(this, 0xC4);
                if first == rd::<u32>(this, 0xD8) {
                    wr::<u32>(this, rec(0x118), first);
                    wr::<u8>(this, rec(0x124), rd::<u8>(this, 0xF0));
                    wr::<u32>(this, rec(0x11C), rd::<u32>(this, 0xC8));
                    wr::<u8>(this, rec(0x125), rd::<u8>(this, 0xF1));
                    if three {
                        wr::<u32>(this, rec(0x120), rd::<u32>(this, 0xCC));
                        wr::<u8>(this, rec(0x126), rd::<u8>(this, 0xF2));
                    }
                } else {
                    wr::<u32>(this, rec(0x118), rd::<u32>(this, 0xD8));
                    wr::<u8>(this, rec(0x124), rd::<u8>(this, 0xF5));
                    wr::<u32>(this, rec(0x11C), rd::<u32>(this, 0xC4));
                    wr::<u8>(this, rec(0x125), rd::<u8>(this, 0xF0));
                    if three {
                        wr::<u32>(this, rec(0x120), rd::<u32>(this, 0xC8));
                        wr::<u8>(this, rec(0x126), rd::<u8>(this, 0xF1));
                    }
                }
            };
            match ty {
                0 | 1 | 2 | 5 => {
                    wr::<u32>(this, rec(0x114), rd::<u32>(this, 0xE0));
                    wr::<u32>(this, rec(0x118), rd::<u32>(this, 0xD8));
                    wr::<u8>(this, rec(0x124), rd::<u8>(this, 0xF5));
                }
                3 => {
                    if rd::<u8>(this, 0xE9) == 11 {
                        let i = isize::from(rd::<i8>(this, 0xF7));
                        wr::<u32>(this, 0x2E8, g::<u32>((0x8CAD50 + i * 4) as u32 as usize).read());
                    } else {
                        wr::<u32>(this, rec(0x104), 0x249F0);
                    }
                    wr::<u32>(this, rec(0x108), 0x1388);
                    wr::<u32>(this, rec(0x114), rd::<u32>(this, 0xE0));
                    fill_from_queue(true);
                }
                4 | 7 => {
                    wr::<u32>(this, rec(0x104), 0x1388);
                    wr::<u32>(this, rec(0x114), rd::<u32>(this, 0xE0));
                    fill_from_queue(false);
                }
                _ => {}
            }
        }
    }
    wr::<u8>(this, 0, 0);
    wr::<u8>(this, 3, 0);
    if game_paused() && flag == 0 {
        wr::<i32>(this, 0x78, -1);
        wr::<u32>(this, 0x5C, 0);
    }
    if settings.is_null() {
        if flag != 0 {
            wr::<u32>(this, 0x6C, 0);
            wr::<u32>(this, 0x70, 0);
            wr::<u8>(this, 6, 0);
        }
        return;
    }
    wr::<u32>(this, 0x6C, 0);
    wr::<u32>(this, 0x70, 0);
    wr::<i32>(this, 0x7C, -1);
    wr::<u8>(this, 1, 0);
    wr::<u8>(this, 6, 0);
    engine_report_frontend_audio_event(g(crate::common::AUDIO_ENGINE), 0x23, 0, 0x3F80_0000);
    if rd::<u8>(this, 0xE9) == 0xFF {
        wr::<u8>(this, 0xE9, 13);
    }
    wr::<u8>(settings, 0x1A, rd::<u8>(this, 0xE9));
    wr::<u8>(settings, 6, rd::<u8>(this, 0xEA));
    let off = rd::<u32>(this, 0x68) == 7
        && rd::<u8>(this, 0) == 0
        && rd::<u32>(this, 0x6C) == 0
        && rd::<u32>(this, 0x70) == 0;
    if !off {
        wr::<u32>(this, 0x58, g::<u32>(TIMER_MS).read());
        wr::<u8>(this, 0x16, g::<u8>(CLOCK_DAYS).read());
        wr::<u8>(this, 0x17, g::<u8>(CLOCK_HOURS).read());
        wr::<i32>(this, 0x74, i32::from(rd::<i8>(this, 0xE9)));
    }
    if rd::<u8>(this, 0xE9) == 0 {
        let r = crate::common::ae_random_int(1, 13);
        wr::<u8>(this, 0xE9, r as u8);
    }
}

orig! {
    /// `CGameLogic::LaRiotsActiveHere()` (0x441C10)
    fn game_logic_la_riots_active_here() -> u8 = cdecl 0x441C10 => unported;
    /// `CWeather::ForecastWeather(eWeatherType, int32 numSteps)` (0x72A590)
    fn weather_forecast_weather(ty: u32, steps: u32) -> u8 = cdecl 0x72A590 => unported;
    /// `CAERadioTrackManager::ChooseDJBanterIndexFromList(eRadioID, int32 (*list)[15])` (0x4E95E0)
    fn radio_choose_dj_banter_index_from_list(this: GPtr<u8>, station: u32, list: u32) -> u32 = thiscall 0x4E95E0 => unported;
}

/// `CAERadioTrackManager::ChooseDJBanterIndex(eRadioID)` (0x4EA2D0): first a mission/stat-driven banter
/// chosen by `m_nSpecialDJBanterPending` (0xB62C73) and `m_nSpecialDJBanterIndex` (0xB62C72) from the tables
/// 0x8C8BF0 (8 bytes per station), 0x8C8C50 (2 per station) or 0x8CB280 (22 per station), rejected when
/// 0x782 or already in the station's DJ banter history; otherwise station 0 picks from the riot list
/// (0x8C8E30) or the generic list (0x8C8CB0) with station argument 0, and other stations (60% chance, not
/// during a mission flag 0xB72914) try weather lists (fog/sandstorm 0x8C8E90, rain 0x8C8F50; the bytes'
/// sunny-weather test `hours >= 8 && hours < 6` can never pass), then time-of-day lists (morning
/// 0x8C8DD0, evening 0x8C8D70, night 0x8C8E30) and finally the generic list. Returns the full EAX.
#[hook(addr = 0x4EA2D0, conv = thiscall, name = "CAERadioTrackManager::ChooseDJBanterIndex", expect = "0F BE 05 73 2C B6 00")]
pub fn radio_choose_dj_banter_index(this: GPtr<u8>, station: u32) -> u32 {
    let st = i32::from(station as i8);
    let mut idx: i32 = -1;
    let pending = i32::from(g::<i8>(0xB62C73).read());
    let special = g::<i8>(0xB62C72).read();
    let mut candidate: Option<i32> = None;
    match pending {
        0 => candidate = Some(g::<i32>((0x8C8BF0 + st * 8) as u32 as usize).read()),
        1 => {
            let use_it = special == 0
                || (special == 1
                    && g::<u32>((0x8C8C50 + st * 8) as u32 as usize).read()
                        != g::<u32>((0x8C8C54 + st * 8) as u32 as usize).read());
            if use_it {
                let a = (0x8C8C50 + (i32::from(special) + st * 2) * 4) as u32 as usize;
                candidate = Some(g::<i32>(a).read());
            }
        }
        2 => {
            let a = (0x8CB280 + (st * 0x16 + i32::from(special)) * 4) as u32 as usize;
            candidate = Some(g::<i32>(a).read());
        }
        _ => {}
    }
    if let Some(c) = candidate {
        idx = c;
        if c == 0x782 {
            idx = -1;
        } else if c >= 0 {
            let hist = (0xB61D78 + st * 0x3C) as u32 as usize;
            if (0..15).any(|i| g::<i32>(hist + i * 4).read() == c) {
                idx = -1;
            }
        }
        if idx != -1 {
            return idx as u32;
        }
    }
    let from_list = |list: u32| radio_choose_dj_banter_index_from_list(this, station, list);
    if station as u8 == 0 {
        let list = if game_logic_la_riots_active_here() != 0 {
            0x8C8E30
        } else {
            0x8C8CB0
        };
        return radio_choose_dj_banter_index_from_list(this, 0, list);
    }
    let p = |bits: u32| crate::common::ae_resolve_probability(f32::from_bits(bits)) != 0;
    if !p(0x3F19_999A) || g::<u32>(0xB72914).read() != 0 {
        return idx as u32;
    }
    let hours = g::<u8>(CLOCK_HOURS).read();
    let mut tried = false;
    if weather_forecast_weather(0x10, 3) != 0 || weather_forecast_weather(8, 3) != 0 {
        if p(0x3F00_0000) {
            let r = from_list(0x8C8E90);
            if r != 0xFFFF_FFFF {
                return r;
            }
        }
        tried = true;
    } else {
        // The sunny-weather branch: evaluated for its calls, its `hours >= 8 && hours < 6` never holds.
        let _ = weather_forecast_weather(0, 3) != 0
            || weather_forecast_weather(2, 3) != 0
            || weather_forecast_weather(0xD, 3) != 0
            || weather_forecast_weather(6, 3) != 0
            || weather_forecast_weather(0xB, 3) != 0
            || weather_forecast_weather(0x11, 3) != 0;
    }
    if !tried && weather_forecast_weather(9, 3) != 0 && p(0x3F00_0000) {
        let r = from_list(0x8C8F50);
        if r != 0xFFFF_FFFF {
            return r;
        }
    }
    let tod = if (6..9).contains(&hours) {
        Some(0x8C8DD0)
    } else if (0x12..0x15).contains(&hours) {
        Some(0x8C8D70)
    } else if !(3..0x16).contains(&hours) {
        Some(0x8C8E30)
    } else {
        None
    };
    if let Some(list) = tod
        && p(0x3E99_999A)
    {
        let r = from_list(list);
        if r != 0xFFFF_FFFF {
            return r;
        }
    }
    from_list(0x8C8CB0)
}

orig! {
    /// `CStreaming::IsVeryBusy()` (0x4076A0)
    fn streaming_is_very_busy() -> u8 = cdecl 0x4076A0 => unported;
    /// `CAEAudioHardware::LoadSound(uint16 bank, uint16 sound, int16 slot)` (0x4D8ED0)
    fn hw_load_sound(hw: GPtr<u8>, bank: u32, sound: u32, slot: u32) = thiscall 0x4D8ED0 => unported;
    /// `CAEAudioHardware::IsSoundLoaded(uint16 bank, uint16 sound, int16 slot)` (0x4D8EF0)
    fn hw_is_sound_loaded(hw: GPtr<u8>, bank: u32, sound: u32, slot: u32) -> u8 = thiscall 0x4D8EF0 => unported;
}

/// `CAEPoliceScannerAudioEntity::LoadSlots()` (0x4E6CD0): for each of the 5 slots (state words 0xB61D14,
/// bank slots 33..37) of the current list (0xB61D10, (bank, sound) int16 pairs): state 2 (loading)
/// becomes 3 once `IsSoundLoaded`; state 0 becomes 1 for an empty pair, else (unless streaming is very
/// busy) requests `LoadSound` and becomes 2. When every slot was done, the playback state (0xB61D4C)
/// becomes 4. The list pointer is re-read after each hardware call, as the bytes do.
#[hook(addr = 0x4E6CD0, conv = cdecl, name = "CAEPoliceScannerAudioEntity::LoadSlots", expect = "51 8B 0D 10 1D B6 00")]
pub fn ps_load_slots() {
    let mut list = g::<u32>(PS_SLOT_LIST).read();
    if list == 0 {
        return;
    }
    let hw = g::<u8>(AE_AUDIO_HARDWARE);
    let mut done = true;
    for i in 0..5usize {
        let st = g::<i16>(PS_SLOTS + i * 2);
        let slot = 0x21 + i as u32;
        let pair = |l: u32| g::<u8>(l as usize + i * 4);
        match st.read() {
            2 => {
                let p = pair(list);
                let loaded =
                    hw_is_sound_loaded(hw, u32::from(rd::<u16>(p, 0)), u32::from(rd::<u16>(p, 2)), slot);
                list = g::<u32>(PS_SLOT_LIST).read();
                if loaded != 0 {
                    st.write(3);
                } else {
                    done = false;
                }
            }
            0 => {
                let p = pair(list);
                if rd::<i16>(p, 0) < 0 || rd::<i16>(p, 2) < 0 {
                    st.write(1);
                } else {
                    if streaming_is_very_busy() == 0 {
                        let p = pair(g::<u32>(PS_SLOT_LIST).read());
                        hw_load_sound(hw, u32::from(rd::<u16>(p, 0)), u32::from(rd::<u16>(p, 2)), slot);
                        st.write(2);
                    }
                    list = g::<u32>(PS_SLOT_LIST).read();
                    done = false;
                }
            }
            _ => {}
        }
    }
    if done {
        g::<u16>(PS_PLAYBACK_STATE).write(4);
    }
}

orig! {
    /// `CAEPoliceScannerAudioEntity::LoadSlots()` (0x4E6CD0)
    fn ps_load_slots_call() = cdecl 0x4E6CD0 => ps_load_slots;
}

/// `CAEPoliceScannerAudioEntity::Service()` (0x4E7630): the byte 0xB6F065 during a dialogue sets the
/// stopping flag. By playback state (jump table 0x4E774C): 2 = loading (finish when stopping or past the
/// load deadline 0xB61D08, else `LoadSlots`); 4 = loaded (finish when stopping; at 0xB61D0C: the
/// volume offset 0xB61CF8 = 0 with the vehicle-radio pointer 0xB6B98C set, else `[0x8C815C]`; report events
/// 0x31 (`[0xB61D54]` + offset) and 0x2F (`[0x8C8158]` + offset); next time now + `[0x8C8154]`; state 5);
/// 5 = (finish when stopping; at 0xB61D0C with a controller: state 7 and its `PlayLoadedDialogue`).
#[hook(addr = 0x4E7630, conv = cdecl, name = "CAEPoliceScannerAudioEntity::Service", expect = "A0 65 F0 B6 00")]
pub fn ps_service() {
    let state = g::<i16>(PS_PLAYBACK_STATE).read();
    let stopping = if g::<u8>(0xB6F065).read() != 0 && state != 0 {
        g::<u8>(PS_STOPPING).write(1);
        1
    } else {
        g::<u8>(PS_STOPPING).read()
    };
    match state {
        2 => {
            if g::<u32>(TIMER_MS).read() > g::<u32>(PS_D08).read() || stopping != 0 {
                ps_finished_playing_call();
            } else {
                ps_load_slots_call();
            }
        }
        4 => {
            if stopping != 0 {
                ps_finished_playing_call();
                return;
            }
            if g::<u32>(TIMER_MS).read() < g::<u32>(PS_D0C).read() {
                return;
            }
            let off = if g::<u32>(0xB6B98C).read() != 0 {
                0
            } else {
                g::<u32>(0x8C815C).read()
            };
            g::<u32>(PS_CF8).write(off);
            let engine = g::<u8>(crate::common::AUDIO_ENGINE);
            let v = x(g::<f32>(0xB61D54).read())
                .fadd_m(g::<f32>(PS_CF8).read())
                .to_f32();
            engine_report_frontend_audio_event(engine, 0x31, v.to_bits(), 0x3F80_0000);
            let v = x(g::<f32>(0x8C8158).read())
                .fadd_m(g::<f32>(PS_CF8).read())
                .to_f32();
            engine_report_frontend_audio_event(engine, 0x2F, v.to_bits(), 0x3F80_0000);
            let t = g::<u32>(TIMER_MS).read().wrapping_add(g::<u32>(0x8C8154).read());
            g::<u32>(PS_D0C).write(t);
            g::<u16>(PS_PLAYBACK_STATE).write(5);
        }
        5 => {
            if stopping != 0 {
                ps_finished_playing_call();
                return;
            }
            if g::<u32>(TIMER_MS).read() < g::<u32>(PS_D0C).read() {
                return;
            }
            let c = g::<GPtr<u8>>(PS_CONTROLLING).read();
            if c.is_null() {
                return;
            }
            g::<u16>(PS_PLAYBACK_STATE).write(7);
            ps_play_loaded_dialogue(c);
        }
        _ => {}
    }
}

orig! {
    /// `CStats::GetStatValue(uint16 stat)` (0x558E40, ST0)
    fn stats_get_stat_value(stat: u32) -> f32 = cdecl 0x558E40 => unported;
}

/// The mission-progress stats `CheckForMissionStatsChanges` watches, in the bytes' order: stat id, the
/// last-seen byte (0xB62C71 down to 0xB62C5B) and the special banter index stored on a change to 1
/// (`None` = the stat value itself).
const MISSION_STATS: [(u32, Option<u8>); 22] = [
    (0x147, Some(0)),
    (0x148, None),
    (0x149, Some(2)),
    (0x13E, Some(3)),
    (0x140, Some(4)),
    (0x14A, Some(5)),
    (0x14B, Some(6)),
    (0x14C, Some(7)),
    (0x14D, Some(8)),
    (0x137, Some(9)),
    (0x143, Some(10)),
    (0x134, Some(11)),
    (0x136, Some(12)),
    (0x14F, Some(13)),
    (0x150, Some(14)),
    (0x151, Some(15)),
    (0x12E, Some(16)),
    (0x152, Some(17)),
    (0x153, Some(18)),
    (0x154, Some(19)),
    (0x155, Some(20)),
    (0x156, Some(21)),
];

/// `CAERadioTrackManager::CheckForMissionStatsChanges()` (0x4E8410): first expires a pending special
/// banter (0xB62C73 = 3) once `days * 24 - hour0 + hour >= duration` (days since 0xB62C5A, wrapped with
/// the previous month's length from 0x8CCF24); then for stat 0xB5 and the 22 `MISSION_STATS`: when the
/// last-seen byte (int8) is below the stat value, the byte becomes `ftol(stat)`; stat 0xB5 at 1 or 2
/// arms banter type 1 (index value - 1, 24 h), the others at 1 arm type 2 (168 h) with their index; each
/// arming stamps the day (0xB62C5A) and hour (0xB62C59). `CStats::GetStatValue` is called twice per
/// changed stat (compare, then `_ftol`), as the bytes do; its ST0 is taken as an f32 (exact for the
/// stats' integer values below 2^24).
#[hook(addr = 0x4E8410, conv = thiscall, name = "CAERadioTrackManager::CheckForMissionStatsChanges", expect = "51 80 3D 73 2C B6 00 03")]
pub fn radio_check_for_mission_stats_changes(_this: GPtr<u8>) {
    const PENDING: usize = 0xB62C73;
    const INDEX: usize = 0xB62C72;
    const DAY0: usize = 0xB62C5A;
    const HOUR0: usize = 0xB62C59;
    const DURATION: usize = 0xB62C58;
    if g::<u8>(PENDING).read() != 3 {
        let mut d = i32::from(g::<u8>(CLOCK_DAYS).read()) - i32::from(g::<i8>(DAY0).read());
        if d < 0 {
            let mut m = g::<u8>(0xB70155).read().wrapping_sub(1) as i8;
            if m < 0 {
                m = m.wrapping_add(12);
            }
            d += i32::from(g::<u8>((0x8CCF24 + i32::from(m)) as u32 as usize).read());
        }
        let h = d * 24 - i32::from(g::<i8>(HOUR0).read()) + i32::from(g::<u8>(CLOCK_HOURS).read());
        if h >= i32::from(g::<u8>(DURATION).read()) {
            g::<u8>(PENDING).write(3);
        }
    }
    let arm = |duration: u8, pending: u8, index: u8| {
        g::<u8>(DAY0).write(g::<u8>(CLOCK_DAYS).read());
        g::<u8>(HOUR0).write(g::<u8>(CLOCK_HOURS).read());
        g::<u8>(DURATION).write(duration);
        g::<u8>(PENDING).write(pending);
        g::<u8>(INDEX).write(index);
    };
    // Returns the new last-seen byte when the stat moved past it.
    let changed = |stat: u32, byte: usize| -> Option<u8> {
        let v = stats_get_stat_value(stat);
        let seen = crate::common::xi(i32::from(g::<i8>(byte).read()));
        if seen.fcom(x(v)).test_parity(0x05) {
            return None;
        }
        let n = crate::common::ftol32(x(stats_get_stat_value(stat))) as u8;
        g::<u8>(byte).write(n);
        Some(n)
    };
    if let Some(n) = changed(0xB5, 0xB62C71)
        && (n == 1 || n == 2)
    {
        arm(0x18, 1, n - 1);
    }
    for (k, &(stat, index)) in MISSION_STATS.iter().enumerate() {
        if let Some(n) = changed(stat, 0xB62C70 - k)
            && n == 1
        {
            arm(0xA8, 2, index.unwrap_or(n));
        }
    }
}

orig! {
    /// `CAERadioTrackManager::QueueUpTracksForStation(eRadioID, int8* index, eTrackType, tRadioSettings*)` (0x4EA670)
    fn radio_queue_up_tracks_for_station(this: GPtr<u8>, station: u32, index: GPtr<u8>, ty: u32, settings: GPtr<u8>) -> u8 = thiscall 0x4EA670 => unported;
    /// `CAERadioTrackManager::AddMusicTrackIndexToHistory(eRadioID, int8)` (0x4E96C0)
    fn radio_add_music_track_index_to_history(this: GPtr<u8>, station: u32, index: u32) = thiscall 0x4E96C0 => crate::radio::add_music_track_index_to_history;
    /// `CAEUserRadioTrackManager::SelectUserTrackIndex()` (0x4F3250)
    fn user_select_user_track_index_call(this: GPtr<u8>) -> u32 = thiscall 0x4F3250 => user_select_user_track_index;
}

/// Days since the stamp `day0` with the previous month's length (0x8CCF24) when the day number wrapped,
/// as `CheckForMissionStatsChanges` / `TrackRadioStation` compute it.
fn days_since(day0: i8) -> i32 {
    let mut d = i32::from(g::<u8>(CLOCK_DAYS).read()) - i32::from(day0);
    if d < 0 {
        let mut m = g::<u8>(0xB70155).read().wrapping_sub(1) as i8;
        if m < 0 {
            m = m.wrapping_add(12);
        }
        d += i32::from(g::<u8>((0x8CCF24 + i32::from(m)) as u32 as usize).read());
    }
    d
}

/// `CAERadioTrackManager::TrackRadioStation(eRadioID, uint8)` (0x4EAC30): resumes a station from its
/// history record (+0x100 + station * 0x2C) as if it had kept playing. Fails (0) when the record was
/// stamped more than 5 game hours ago, the elapsed time is past the record + 160 s, or the resumed track
/// type is 6. Otherwise it clears the 5-entry queue (+0x88 ids, +0xB4 types, +0xBB indices), sets the
/// play offset (+0xA4) and refills the queue from the record (type jump table 0x4EB158) or through
/// `QueueUpTracksForStation` (the local `int8` index lives on the Rust stack), and returns 1. The elapsed
/// time is at least 7000 ms, and at least record +0x100 + 1 with the flag. The station argument dword is
/// passed through to the callees except where the bytes push a literal 12 or 0.
#[hook(addr = 0x4EAC30, conv = thiscall, name = "CAERadioTrackManager::TrackRadioStation", expect = "83 EC 10 53 8B 5C 24 18")]
pub fn radio_track_radio_station(this: GPtr<u8>, station: u32, flag: u32) -> u8 {
    let st8 = station as u8;
    let r = i32::from(st8 as i8) * 0x2C;
    let ri = |off: i32| (r + off) as isize;
    let mut idx_buf = [0u8; 4];
    let idx = GPtr::from_raw(idx_buf.as_mut_ptr());
    let queue = this.byte_add(0x88);
    let q = |stn: u32, ty: u32| radio_queue_up_tracks_for_station(this, stn, idx, ty, queue);
    let hours0 = rd::<i8>(this, ri(0x128));
    if hours0 >= 0 {
        let day0 = rd::<i8>(this, ri(0x127));
        if day0 >= 0 {
            let h = days_since(day0) * 24 - i32::from(hours0) + i32::from(g::<u8>(CLOCK_HOURS).read());
            if h > 5 {
                return 0;
            }
        }
    }
    let mut elapsed = g::<i32>(TIMER_MS).read().wrapping_sub(rd::<i32>(this, ri(0x110)));
    if elapsed <= 7000 {
        elapsed = 7000;
    }
    if flag as u8 != 0 {
        let t = rd::<i32>(this, ri(0x100)).wrapping_add(1);
        if elapsed <= t {
            elapsed = t;
        }
    }
    for i in 0..5isize {
        wr::<i32>(this, 0x88 + i * 4, -1);
        wr::<u8>(this, 0xB4 + i, 6);
        wr::<u8>(this, 0xBB + i, 0xFF);
    }
    let mut total: i32 = 0;
    let mut hit = None;
    for i in 0..3i32 {
        total = total.wrapping_add(rd::<i32>(this, ri(0x100 + 4 * i)));
        if elapsed <= total {
            hit = Some(i);
            break;
        }
    }
    let Some(i) = hit else {
        if elapsed <= total.wrapping_add(7000) {
            if st8 == 12 {
                q(12, 4);
                q(12, 4);
            } else {
                q(station, 3);
            }
            let v = elapsed.wrapping_sub(total);
            wr::<i32>(this, 0xA4, if v < 5000 { v } else { 5000 });
            return 1;
        }
        if elapsed <= total.wrapping_add(155_000) {
            q(station, 4);
            wr::<i32>(this, 0xA4, elapsed.wrapping_sub(total).wrapping_sub(5000));
            return 1;
        }
        if elapsed > total.wrapping_add(160_000) {
            return 0;
        }
        if st8 == 12 {
            q(12, 4);
            if g::<u8>(MENU_RADIO_MODE).read() == 0
                && crate::common::ae_resolve_probability(f32::from_bits(0x3E2E_147B)) != 0
            {
                q(12, 1);
            }
        } else {
            q(station, 5);
            let k = isize::from(rd::<i8>(idx, 0));
            radio_add_music_track_index_to_history(this, station, u32::from(rd::<u8>(this, 0xBA + k)));
            if st8 == 0 {
                q(0, 2);
            } else {
                let p1 = crate::common::ae_resolve_probability(0.5) != 0;
                let p2 = crate::common::ae_resolve_probability(0.5) != 0;
                if p2 {
                    q(station, 0);
                }
                if p1 {
                    q(station, 3);
                } else if q(station, 2) == 0 {
                    q(station, 1);
                }
            }
        }
        wr::<i32>(this, 0xA4, elapsed.wrapping_sub(total).wrapping_sub(155_000));
        return 1;
    };
    let a4 = if i == 0 {
        rd::<i32>(this, ri(0x114)).wrapping_add(elapsed)
    } else {
        rd::<i32>(this, ri(0x100 + 4 * i))
            .wrapping_sub(total)
            .wrapping_add(elapsed)
    };
    wr::<i32>(this, 0xA4, a4);
    let ty = i32::from(rd::<i8>(this, ri(0x124 + i))) as u32;
    let copy = |n: i32| {
        for j in 0..n {
            wr::<u32>(queue, (j * 4) as isize, rd::<u32>(this, ri(0x118 + 4 * (i + j))));
            wr::<u8>(this, (0xB4 + j) as isize, rd::<u8>(this, ri(0x124 + i + j)));
        }
    };
    match ty {
        0..=2 => {
            copy(1);
            wr::<u8>(idx, 0, 1);
            if st8 == 12 {
                q(12, 4);
            } else {
                q(station, 3);
            }
        }
        5 => {
            copy(1);
            wr::<u8>(idx, 0, 1);
            if st8 == 0 {
                q(0, 2);
            } else {
                if crate::common::ae_resolve_probability(0.5) != 0 {
                    q(station, 0);
                }
                if q(station, 2) == 0 {
                    q(station, 1);
                }
            }
        }
        3 => copy(3),
        4 => copy(2),
        7 => {
            copy(2);
            if rd::<i32>(this, 0x8C) == -1 {
                let u = user_select_user_track_index_call(g(0xB6B970));
                wr::<u32>(queue, 4, u);
                wr::<u8>(queue, 0x2D, 7);
                wr::<u8>(queue, 0x34, rd::<u8>(queue, 4));
            }
        }
        _ => return 0,
    }
    1
}

/// `CStats::GetStatValue(stat)` compared with the f32 constant at `c` (`fcomp m32`).
fn stat_cmp(stat: u32, c: usize) -> sa_math::x87::Cmp {
    x(stats_get_stat_value(stat)).fcom(x(g::<f32>(c).read()))
}

/// `CAERadioTrackManager::ChooseTalkRadioShow()` (0x4E8E40): builds a candidate list (stack array of 31
/// bytes initialised to -1, count `bl`) from mission-progress stats compared with `[0x858B50]` (0.0) /
/// `[0x858B90]` (`CStats::GetStatValue`, the bytes' decision chain transcribed as a state machine over the
/// original addresses), then draws `GetRandomNumberInRange(0, count - 1)` until the candidate is not
/// among the first `count - 1` entries of the talk show history (0xB62C1C). Returns the original's EAX:
/// the drawn index with its low byte replaced by the candidate. The port's array is 256 bytes of -1 (the
/// original's bytes past 31 would be other stack slots; no path stores that many).
#[hook(addr = 0x4E8E40, conv = thiscall, name = "CAERadioTrackManager::ChooseTalkRadioShow", expect = "83 EC 20 53 56")]
pub fn radio_choose_talk_radio_show(_this: GPtr<u8>) -> u32 {
    let mut arr = [0xFFu8; 256];
    let mut bl: u8 = 0;
    let mut pc: u32 = 0x4E8E56;
    loop {
        pc = match pc {
            0x4E8E56 => {
                if !stat_cmp(0x136, 0x858B50).test_parity(0x44) {
                    0x4E8E94
                } else {
                    0x4E8E73
                }
            }
            0x4E8E73 => {
                if stat_cmp(0x137, 0x858B50).test_parity(0x44) {
                    0x4E8E94
                } else {
                    0x4E8E8D
                }
            }
            0x4E8E8D => {
                arr[0] = 0xe;
                0x4E8E92
            }
            0x4E8E92 => 0x4E8EB3,
            0x4E8E94 => {
                if !stat_cmp(0x138, 0x858B50).test_parity(0x44) {
                    0x4E8EB5
                } else {
                    0x4E8EAE
                }
            }
            0x4E8EAE => {
                arr[0] = 0xf;
                0x4E8EB3
            }
            0x4E8EB3 => {
                bl = 1;
                0x4E8EB5
            }
            0x4E8EB5 => {
                if stat_cmp(0x139, 0x858B50).test_parity(0x44) {
                    0x4E8ED9
                } else {
                    0x4E8ECF
                }
            }
            0x4E8ECF => {
                arr[usize::from(bl)] = 0xc;
                0x4E8ED7
            }
            0x4E8ED7 => 0x4E8F15,
            0x4E8ED9 => {
                if !stat_cmp(0x139, 0x858B50).test_parity(0x44) {
                    0x4E8F17
                } else {
                    0x4E8EF3
                }
            }
            0x4E8EF3 => {
                if stat_cmp(0x138, 0x858B50).test_parity(0x44) {
                    0x4E8F17
                } else {
                    0x4E8F0D
                }
            }
            0x4E8F0D => {
                arr[usize::from(bl)] = 0xd;
                0x4E8F15
            }
            0x4E8F15 => {
                bl = bl.wrapping_add(1);
                0x4E8F17
            }
            0x4E8F17 => {
                if !stat_cmp(0x13B, 0x858B50).test_parity(0x44) {
                    0x4E8F55
                } else {
                    0x4E8F31
                }
            }
            0x4E8F31 => {
                if stat_cmp(0x13C, 0x858B50).test_parity(0x44) {
                    0x4E8F55
                } else {
                    0x4E8F4B
                }
            }
            0x4E8F4B => {
                arr[usize::from(bl)] = 0x6;
                0x4E8F53
            }
            0x4E8F53 => {
                bl = bl.wrapping_add(1);
                0x4E8F55
            }
            0x4E8F55 => {
                if !stat_cmp(0x12E, 0x858B50).test_parity(0x44) {
                    0x4E8F93
                } else {
                    0x4E8F6F
                }
            }
            0x4E8F6F => {
                if stat_cmp(0x13A, 0x858B50).test_parity(0x44) {
                    0x4E8F93
                } else {
                    0x4E8F89
                }
            }
            0x4E8F89 => {
                arr[usize::from(bl)] = 0x3;
                0x4E8F91
            }
            0x4E8F91 => 0x4E8FF3,
            0x4E8F93 => {
                if !stat_cmp(0x13C, 0x858B50).test_parity(0x44) {
                    0x4E8FD1
                } else {
                    0x4E8FAD
                }
            }
            0x4E8FAD => {
                if stat_cmp(0x13D, 0x858B50).test_parity(0x44) {
                    0x4E8FD1
                } else {
                    0x4E8FC7
                }
            }
            0x4E8FC7 => {
                arr[usize::from(bl)] = 0x4;
                0x4E8FCF
            }
            0x4E8FCF => 0x4E8FF3,
            0x4E8FD1 => {
                if !stat_cmp(0x13D, 0x858B50).test_parity(0x44) {
                    0x4E8FF5
                } else {
                    0x4E8FEB
                }
            }
            0x4E8FEB => {
                arr[usize::from(bl)] = 0x5;
                0x4E8FF3
            }
            0x4E8FF3 => {
                bl = bl.wrapping_add(1);
                0x4E8FF5
            }
            0x4E8FF5 => {
                if stat_cmp(0x13E, 0x858B50).test_parity(0x44) {
                    0x4E9019
                } else {
                    0x4E900F
                }
            }
            0x4E900F => {
                arr[usize::from(bl)] = 0x7;
                0x4E9017
            }
            0x4E9017 => 0x4E903B,
            0x4E9019 => {
                if !stat_cmp(0x13E, 0x858B50).test_parity(0x44) {
                    0x4E903D
                } else {
                    0x4E9033
                }
            }
            0x4E9033 => {
                arr[usize::from(bl)] = 0x8;
                0x4E903B
            }
            0x4E903B => {
                bl = bl.wrapping_add(1);
                0x4E903D
            }
            0x4E903D => {
                if stat_cmp(0x13F, 0x858B50).test_parity(0x44) {
                    0x4E9061
                } else {
                    0x4E9057
                }
            }
            0x4E9057 => {
                arr[usize::from(bl)] = 0x9;
                0x4E905F
            }
            0x4E905F => 0x4E90DB,
            0x4E9061 => {
                if !stat_cmp(0x13F, 0x858B50).test_parity(0x44) {
                    0x4E90B9
                } else {
                    0x4E907B
                }
            }
            0x4E907B => {
                if stat_cmp(0x130, 0x858B50).test_parity(0x44) {
                    0x4E90B9
                } else {
                    0x4E9095
                }
            }
            0x4E9095 => {
                if stat_cmp(0x140, 0x858B50).test_parity(0x44) {
                    0x4E90B9
                } else {
                    0x4E90AF
                }
            }
            0x4E90AF => {
                arr[usize::from(bl)] = 0xa;
                0x4E90B7
            }
            0x4E90B7 => 0x4E90DB,
            0x4E90B9 => {
                if !stat_cmp(0x140, 0x858B50).test_parity(0x44) {
                    0x4E90DD
                } else {
                    0x4E90D3
                }
            }
            0x4E90D3 => {
                arr[usize::from(bl)] = 0xb;
                0x4E90DB
            }
            0x4E90DB => {
                bl = bl.wrapping_add(1);
                0x4E90DD
            }
            0x4E90DD => {
                if stat_cmp(0x141, 0x858B50).test_parity(0x44) {
                    0x4E9101
                } else {
                    0x4E90F7
                }
            }
            0x4E90F7 => {
                arr[usize::from(bl)] = 0x1b;
                0x4E90FF
            }
            0x4E90FF => 0x4E9123,
            0x4E9101 => {
                if !stat_cmp(0x141, 0x858B50).test_parity(0x44) {
                    0x4E9125
                } else {
                    0x4E911B
                }
            }
            0x4E911B => {
                arr[usize::from(bl)] = 0x1c;
                0x4E9123
            }
            0x4E9123 => {
                bl = bl.wrapping_add(1);
                0x4E9125
            }
            0x4E9125 => {
                if stat_cmp(0x142, 0x858B50).test_parity(0x44) {
                    0x4E9149
                } else {
                    0x4E913F
                }
            }
            0x4E913F => {
                arr[usize::from(bl)] = 0x1d;
                0x4E9147
            }
            0x4E9147 => 0x4E916B,
            0x4E9149 => {
                if !stat_cmp(0x142, 0x858B50).test_parity(0x44) {
                    0x4E916D
                } else {
                    0x4E9163
                }
            }
            0x4E9163 => {
                arr[usize::from(bl)] = 0x1e;
                0x4E916B
            }
            0x4E916B => {
                bl = bl.wrapping_add(1);
                0x4E916D
            }
            0x4E916D => {
                if stat_cmp(0x143, 0x858B50).test_parity(0x44) {
                    0x4E9191
                } else {
                    0x4E9187
                }
            }
            0x4E9187 => {
                arr[usize::from(bl)] = 0x0;
                0x4E918F
            }
            0x4E918F => 0x4E91F1,
            0x4E9191 => {
                if !stat_cmp(0x144, 0x858B50).test_parity(0x44) {
                    0x4E91CF
                } else {
                    0x4E91AB
                }
            }
            0x4E91AB => {
                if stat_cmp(0x145, 0x858B50).test_parity(0x44) {
                    0x4E91CF
                } else {
                    0x4E91C5
                }
            }
            0x4E91C5 => {
                arr[usize::from(bl)] = 0x1;
                0x4E91CD
            }
            0x4E91CD => 0x4E91F1,
            0x4E91CF => {
                if !stat_cmp(0x146, 0x858B50).test_parity(0x44) {
                    0x4E91F3
                } else {
                    0x4E91E9
                }
            }
            0x4E91E9 => {
                arr[usize::from(bl)] = 0x2;
                0x4E91F1
            }
            0x4E91F1 => {
                bl = bl.wrapping_add(1);
                0x4E91F3
            }
            0x4E91F3 => {
                if stat_cmp(0x12E, 0x858B50).test_parity(0x44) {
                    0x4E921A
                } else {
                    0x4E920D
                }
            }
            0x4E920D => {
                arr[usize::from(bl)] = 0x10;
                0x4E9215
            }
            0x4E9215 => 0x4E947F,
            0x4E921A => {
                if !stat_cmp(0x12E, 0x858B50).test_parity(0x44) {
                    0x4E925B
                } else {
                    0x4E9234
                }
            }
            0x4E9234 => {
                if stat_cmp(0x12F, 0x858B50).test_parity(0x44) {
                    0x4E925B
                } else {
                    0x4E924E
                }
            }
            0x4E924E => {
                arr[usize::from(bl)] = 0x11;
                0x4E9256
            }
            0x4E9256 => 0x4E947F,
            0x4E925B => {
                if !stat_cmp(0x12F, 0x858B50).test_parity(0x44) {
                    0x4E929C
                } else {
                    0x4E9275
                }
            }
            0x4E9275 => {
                if stat_cmp(0x143, 0x858B50).test_parity(0x44) {
                    0x4E929C
                } else {
                    0x4E928F
                }
            }
            0x4E928F => {
                arr[usize::from(bl)] = 0x12;
                0x4E9297
            }
            0x4E9297 => 0x4E947F,
            0x4E929C => {
                if !stat_cmp(0x143, 0x858B50).test_parity(0x44) {
                    0x4E92DD
                } else {
                    0x4E92B6
                }
            }
            0x4E92B6 => {
                if stat_cmp(0x130, 0x858B50).test_parity(0x44) {
                    0x4E92DD
                } else {
                    0x4E92D0
                }
            }
            0x4E92D0 => {
                arr[usize::from(bl)] = 0x13;
                0x4E92D8
            }
            0x4E92D8 => 0x4E947F,
            0x4E92DD => {
                if !stat_cmp(0x130, 0x858B50).test_parity(0x44) {
                    0x4E931E
                } else {
                    0x4E92F7
                }
            }
            0x4E92F7 => {
                if stat_cmp(0x131, 0x858B50).test_parity(0x44) {
                    0x4E931E
                } else {
                    0x4E9311
                }
            }
            0x4E9311 => {
                arr[usize::from(bl)] = 0x14;
                0x4E9319
            }
            0x4E9319 => 0x4E947F,
            0x4E931E => {
                if !stat_cmp(0x131, 0x858B50).test_parity(0x44) {
                    0x4E935F
                } else {
                    0x4E9338
                }
            }
            0x4E9338 => {
                if stat_cmp(0x132, 0x858B50).test_parity(0x44) {
                    0x4E935F
                } else {
                    0x4E9352
                }
            }
            0x4E9352 => {
                arr[usize::from(bl)] = 0x15;
                0x4E935A
            }
            0x4E935A => 0x4E947F,
            0x4E935F => {
                if !stat_cmp(0x132, 0x858B50).test_parity(0x44) {
                    0x4E93A0
                } else {
                    0x4E9379
                }
            }
            0x4E9379 => {
                if stat_cmp(0x133, 0x858B50).test_parity(0x44) {
                    0x4E93A0
                } else {
                    0x4E9393
                }
            }
            0x4E9393 => {
                arr[usize::from(bl)] = 0x16;
                0x4E939B
            }
            0x4E939B => 0x4E947F,
            0x4E93A0 => {
                if !stat_cmp(0x133, 0x858B50).test_parity(0x44) {
                    0x4E93E1
                } else {
                    0x4E93BA
                }
            }
            0x4E93BA => {
                if stat_cmp(0x134, 0x858B50).test_parity(0x44) {
                    0x4E93E1
                } else {
                    0x4E93D4
                }
            }
            0x4E93D4 => {
                arr[usize::from(bl)] = 0x17;
                0x4E93DC
            }
            0x4E93DC => 0x4E947F,
            0x4E93E1 => {
                if !stat_cmp(0x134, 0x858B50).test_parity(0x44) {
                    0x4E941F
                } else {
                    0x4E93FB
                }
            }
            0x4E93FB => {
                if stat_cmp(0x135, 0x858B50).test_parity(0x44) {
                    0x4E941F
                } else {
                    0x4E9415
                }
            }
            0x4E9415 => {
                arr[usize::from(bl)] = 0x18;
                0x4E941D
            }
            0x4E941D => 0x4E947F,
            0x4E941F => {
                if !stat_cmp(0x135, 0x858B50).test_parity(0x44) {
                    0x4E945D
                } else {
                    0x4E9439
                }
            }
            0x4E9439 => {
                if !stat_cmp(0xB5, 0x858B90).test_parity(0x44) {
                    0x4E945D
                } else {
                    0x4E9453
                }
            }
            0x4E9453 => {
                arr[usize::from(bl)] = 0x19;
                0x4E945B
            }
            0x4E945B => 0x4E947F,
            0x4E945D => {
                if stat_cmp(0xB5, 0x858B90).test_parity(0x44) {
                    0x4E9481
                } else {
                    0x4E9477
                }
            }
            0x4E9477 => {
                arr[usize::from(bl)] = 0x1a;
                0x4E947F
            }
            0x4E947F => {
                bl = bl.wrapping_add(1);
                0x4E9481
            }
            _ => break,
        };
    }
    let n = i32::from(bl as i8) - 1;
    loop {
        let r = crate::common::ae_random_int(0, n) as u32;
        let v = arr.get(r as usize).copied().unwrap_or(0xFF);
        let mut repeat = false;
        if n > 0 {
            for i in 0..n {
                if g::<u8>(0xB62C1C + i as usize).read() == v {
                    repeat = true;
                    break;
                }
            }
        }
        if !repeat {
            return (r & !0xFF) | u32::from(v);
        }
    }
}

orig! {
    /// `CAEScriptAudioEntity::IsMissionAudioSampleFinished(uint8 slot)` (0x4EBFE0)
    fn script_is_mission_audio_sample_finished(this: GPtr<u8>, slot: u32) -> u8 = thiscall 0x4EBFE0 => unported;
    /// `CAEAudioUtility::GetBankAndSoundFromScriptSlotAudioEvent(const int32*, int32* bank, int32* sound, int32 slot)` (0x4D9CC0)
    fn util_get_bank_and_sound_from_script_slot_audio_event(event: GPtr<u8>, bank: GPtr<u8>, sound: GPtr<u8>, slot: u32) -> u8 = cdecl 0x4D9CC0 => unported;
    /// `CAEAudioHardware::LoadSoundBank(uint16 bank, int16 slot)` (0x4D88A0)
    fn hw_load_sound_bank(hw: GPtr<u8>, bank: u32, slot: u32) = thiscall 0x4D88A0 => unported;
    /// `CAESoundManager::AreSoundsOfThisEventPlayingForThisEntity(int16 event, CAEAudioEntity*)` (0x4EF570; AX)
    fn sm_are_sounds_of_this_event_playing_for_this_entity(sm: GPtr<u8>, event: u32, entity: GPtr<u8>) -> u16 = thiscall 0x4EF570 => unported;
    /// `CAEScriptAudioEntity::PlayResidentSoundEvent(int16 bank, int16 slot, int16 sfx, int32 event, CVector& pos, CPhysical*, float vol, float speed, int16 playTime, float maxDistance)` (0x4EC550)
    fn script_play_resident_sound_event(this: GPtr<u8>, bank: u32, slot: u32, sfx: u32, event: u32, pos: GPtr<u8>, physical: u32, vol_bits: u32, speed_bits: u32, play_time: u32, max_dist_bits: u32) = thiscall 0x4EC550 => unported;
}

/// `CAEScriptAudioEntity::PreloadMissionAudio(uint8 slot, int32 event)` (0x4EC190): for slots < 4 whose
/// previous sample finished: resolves the event to (bank +0xA8, sound +0xAC) of the slot's 32-byte record
/// and loads the sound (sound >= 0, `LoadSound`) or the whole bank (`LoadSoundBank`) into bank slot
/// `slot + 26`; then the record's event (+0xA4) = `event` (re-read from the argument after the call, which
/// receives its address), entity (+0x9C) = 0 and position (+0x90..+0x98) = -1000. The bank-slot words
/// carry leftover upper halves in the original; the callees read only the low word.
#[hook(addr = 0x4EC190, conv = thiscall, name = "CAEScriptAudioEntity::PreloadMissionAudio", expect = "83 EC 0C 53 8B 5C 24 14")]
pub fn script_preload_mission_audio(this: GPtr<u8>, slot: u32, event: u32) {
    let s = slot as u8;
    if s >= 4 || script_is_mission_audio_sample_finished(this, slot) == 0 {
        return;
    }
    let rec = this.byte_add(isize::from(s) * 32);
    let mut ev = [event];
    let evp = GPtr::from_raw(ev.as_mut_ptr().cast::<u8>());
    if util_get_bank_and_sound_from_script_slot_audio_event(
        evp,
        rec.byte_add(0xA8),
        rec.byte_add(0xAC),
        u32::from(s),
    ) == 0
    {
        return;
    }
    let hw = g::<u8>(AE_AUDIO_HARDWARE);
    let bank_slot = u32::from(s) + 0x1A;
    if rd::<i32>(rec, 0xAC) >= 0 {
        hw_load_sound(
            hw,
            u32::from(rd::<u16>(rec, 0xA8)),
            u32::from(rd::<u16>(rec, 0xAC)),
            bank_slot,
        );
    } else {
        hw_load_sound_bank(hw, u32::from(rd::<u16>(rec, 0xA8)), bank_slot);
    }
    wr::<u32>(rec, 0xA4, rd::<u32>(evp, 0));
    wr::<u32>(rec, 0x9C, 0);
    wr::<u32>(rec, 0x90, 0xC47A_0000);
    wr::<u32>(rec, 0x94, 0xC47A_0000);
    wr::<u32>(rec, 0x98, 0xC47A_0000);
}

/// `CAEScriptAudioEntity::Service()` (0x4EC900): with an attached entity (+0x110) and no event 0x3FB sound
/// playing for this entity, plays resident sound event 0x3FB (bank 0x28, slot 0x2C, sfx 0) on that entity
/// with volume 0.0, speed 1.0, play time 0 and max distance 2.5 at the position (-1000, -1000, -1000)
/// (a local `CVector`).
#[hook(addr = 0x4EC900, conv = thiscall, name = "CAEScriptAudioEntity::Service", expect = "83 EC 0C 56 8B F1")]
pub fn script_service(this: GPtr<u8>) {
    let mut pos = [0xC47A_0000u32; 3];
    let entity = rd::<u32>(this, 0x110);
    if entity == 0 {
        return;
    }
    if sm_are_sounds_of_this_event_playing_for_this_entity(g(crate::common::AE_SOUND_MANAGER), 0x3FB, this)
        != 0
    {
        return;
    }
    let entity = rd::<u32>(this, 0x110);
    let p = GPtr::from_raw(pos.as_mut_ptr().cast::<u8>());
    script_play_resident_sound_event(
        this,
        0x28,
        0x2C,
        0,
        0x3FB,
        p,
        entity,
        0,
        0x3F80_0000,
        0,
        0x4020_0000,
    );
}

orig! {
    /// `CAESound::SetPosition(CVector)` (0x4EF880; the vector by value)
    fn sound_set_position(sound: GPtr<u8>, x_bits: u32, y_bits: u32, z_bits: u32) = thiscall 0x4EF880 => unported;
    /// `CAEScriptAudioEntity::PlayMissionBankSound(event, pos, physical, sfx, slot, skip, volDelta, maxDist,
    /// speed)` (0x4EC6D0, `ret 0x24`)
    fn script_play_mission_bank_sound(this: GPtr<u8>, event: u32, pos: GPtr<u8>, physical: u32, sfx: u32, slot: u32, skip: u32, vol_bits: u32, max_dist_bits: u32, speed_bits: u32) = thiscall 0x4EC6D0 => script_play_mission_bank_sound_port;
}

/// `CAEScriptAudioEntity::UpdateParameters(CAESound*, int16 curPlayPos)` (0x4EC970, vtable slot 0). For
/// each of the 4 mission-audio records: the record's own sound (+0xA0) is cleared and the function returns
/// when it ended (-1), else follows the record's entity (+0x9C; `SetPosition`). Any other sound is
/// handled by its event (+0xC), once per record (the bytes run the switch inside the record loop):
/// 0x68 / 0x412 / 0x48F / 0x499 / 0x47B stop after 300 / 300 / 300 / 300 / 500 ms past `this[+0x80]`
/// (0x68 then plays resident event 0x3FD at the sound's position, else keeps volume/speed from +0x84/+0x88);
/// 0x43E fades volume and speed after 4500 ms and plays mission bank sound 0x488 after 4800 ms (once,
/// flag +0x7C); 0x459 / 0x45E move the volume towards the int8 at `[0xBD00F8] + event` (mode +0x8C = 1.0
/// up, 2.0 down then stop). The 0x488 sound: slot 3, sfx `GetRandomNumberInRange(1, 3)`, position a
/// (-1000, -1000, -1000) local, volume delta 0, max distance 2.0, speed 1.0.
#[hook(addr = 0x4EC970, conv = thiscall, name = "CAEScriptAudioEntity::UpdateParameters", expect = "83 EC 14 53 55 56")]
pub fn script_update_parameters(this: GPtr<u8>, sound: GPtr<u8>, pos: u32) {
    let mut local = [0xC47A_0000u32; 3];
    if sound.is_null() {
        return;
    }
    const ADJ: usize = 0x858B1C;
    for i in 0..4isize {
        let rec = this.byte_add(0x9C + i * 32);
        if rd::<u32>(rec, 4) == sound.addr() as u32 {
            if pos as u16 == 0xFFFF {
                wr::<u32>(this, (i + 5) * 32, 0);
                return;
            }
            let ent = rd::<u32>(rec, 0);
            if ent != 0 {
                let e = g::<u8>(ent as usize);
                let m = rd::<u32>(e, 0x14);
                let p = if m != 0 {
                    g::<u8>(m as usize + 0x30)
                } else {
                    e.byte_add(4)
                };
                sound_set_position(sound, rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8));
            }
            continue;
        }
        let ev = rd::<i32>(sound, 0xC);
        let now = g::<u32>(TIMER_MS).read();
        let since = |ms: u32| now > rd::<u32>(this, 0x80).wrapping_add(ms);
        match ev {
            0x68 => {
                if since(300) {
                    sound_stop_sound_and_forget(sound);
                    wr::<u32>(this, 0x80, 0);
                    let phys = rd::<u32>(sound, 8);
                    script_play_resident_sound_event(
                        this,
                        0x28,
                        0x2C,
                        2,
                        0x3FD,
                        sound.byte_add(0x24),
                        phys,
                        0xC140_0000,
                        0x3F80_0000,
                        0,
                        0x4020_0000,
                    );
                } else {
                    wr::<f32>(sound, 0x14, x(rd::<f32>(this, 0x84)).to_f32());
                    wr::<f32>(sound, 0x1C, x(rd::<f32>(this, 0x88)).to_f32());
                }
            }
            0x43E => {
                if since(4500) {
                    let v = x(rd::<f32>(sound, 0x14));
                    if v.fcom(x(g::<f32>(0x85A708).read())).test_any(0x41) {
                        sound_stop_sound_and_forget(sound);
                        wr::<u32>(this, 0x80, 0);
                    } else {
                        wr::<f32>(sound, 0x14, v.fsub_m(g::<f32>(ADJ).read()).to_f32());
                    }
                    let zero = x(g::<f32>(0x858B50).read());
                    let s = x(rd::<f32>(sound, 0x1C));
                    if !s.fcom(zero).test_any(0x41) {
                        let t = s.fsub_m(g::<f32>(0x858CDC).read());
                        let r = if zero.fcom(t).test_any(0x41) { t } else { zero };
                        wr::<f32>(sound, 0x1C, r.to_f32());
                    }
                }
                if since(0x12C0) && rd::<u8>(this, 0x7C) == 0 {
                    let r = crate::common::ae_random_int(1, 3) as u32;
                    let p = GPtr::from_raw(local.as_mut_ptr().cast::<u8>());
                    script_play_mission_bank_sound(this, 0x488, p, 0, r, 3, 0, 0, 0x4000_0000, 0x3F80_0000);
                    wr::<u8>(this, 0x7C, 1);
                }
            }
            0x459 | 0x45E => {
                let t = i32::from(rd::<i8>(g::<u8>(g::<u32>(0xBD00F8).read() as usize), ev as isize));
                let target = crate::common::xi(t).to_f32();
                let mode = rd::<u32>(this, 0x8C);
                if mode == 0x3F80_0000 {
                    let s = x(rd::<f32>(sound, 0x14));
                    if !s.fcom(x(target)).test_parity(0x05) {
                        let t = s.fadd_m(g::<f32>(ADJ).read());
                        let r = if t.fcom(x(target)).test_parity(0x05) {
                            x(target)
                        } else {
                            t
                        };
                        wr::<f32>(sound, 0x14, r.to_f32());
                    }
                } else if mode == 0x4000_0000 {
                    let target = x(target).fsub_m(g::<f32>(0x859008).read()).to_f32();
                    let s = x(rd::<f32>(sound, 0x14));
                    if s.fcom(x(target)).test_any(0x41) {
                        sound_stop_sound_and_forget(sound);
                    } else {
                        let t = s.fsub_m(g::<f32>(ADJ).read());
                        let r = if t.fcom(x(target)).test_any(0x41) {
                            x(target)
                        } else {
                            t
                        };
                        wr::<f32>(sound, 0x14, r.to_f32());
                    }
                }
            }
            0x412 | 0x48F | 0x499 | 0x47B if since(if ev == 0x47B { 500 } else { 300 }) => {
                sound_stop_sound_and_forget(sound);
                wr::<u32>(this, 0x80, 0);
            }
            _ => {}
        }
    }
}

/// `CAEFrontendAudioEntity::UpdateParameters(CAESound*, int16 curPlayPos)` (0x4DEDA0): by event (jump
/// table 0x4DEF30 / index bytes 0x4DEF4C): 0x1B..0x1E pan the sound with the play position (0..350):
/// `SetPosition((±0.1, 0, ±(2 * pos * [0x85F104] - [0x858624])))` (x87 PC=24, x = -0.1 for 0x1B/0x1C,
/// 0.1 for 0x1D/0x1E; z negated as `fsubr` for 0x1C/0x1E); 0x10 / 0x21 stop the sound once playing past
/// `this[+0x90] + 1900` / `this[+0x94] + 100`. Finally the tracked sound (+0x98) ending (-1) clears it
/// and the flag +0x7C.
#[hook(addr = 0x4DEDA0, conv = thiscall, name = "CAEFrontendAudioEntity::UpdateParameters", expect = "83 EC 0C 55 56 57")]
pub fn frontend_update_parameters(this: GPtr<u8>, sound: GPtr<u8>, pos: u32) {
    if sound.is_null() {
        return;
    }
    let p = pos as i16;
    let ev = rd::<i32>(sound, 0xC);
    match ev {
        0x1B..=0x1E if (0..=0x15E).contains(&p) => {
            let xb: u32 = if ev <= 0x1C { 0xBDCC_CCCD } else { 0x3DCC_CCCD };
            let v = crate::common::xi(i32::from(p)).fmul_m(g::<f32>(0x85F104).read());
            let v = v.fadd(v);
            let c = g::<f32>(0x858624).read();
            let z = if ev == 0x1B || ev == 0x1D {
                v.fsub_m(c)
            } else {
                v.fsubr_m(c)
            };
            sound_set_position(sound, xb, 0, z.to_f32().to_bits());
        }
        0x10 | 0x21 if p > 0 => {
            let (off, ms) = if ev == 0x10 { (0x90, 0x76C) } else { (0x94, 0x64) };
            if g::<u32>(TIMER_MS).read() > rd::<u32>(this, off).wrapping_add(ms) {
                sound_stop_sound_and_forget(sound);
            }
        }
        _ => {}
    }
    if rd::<u32>(this, 0x98) == sound.addr() as u32 && p == -1 {
        wr::<u32>(this, 0x98, 0);
        wr::<u8>(this, 0x7C, 0);
    }
}

orig! {
    /// `CAESoundManager::Service()` (0x4F0000)
    fn sm_service(sm: GPtr<u8>) = thiscall 0x4F0000 => unported;
    /// `CAECollisionAudioEntity::Reset()` (0x4DA320)
    fn collision_reset(this: GPtr<u8>) = thiscall 0x4DA320 => unported;
    /// `CAEFrontendAudioEntity::AddAudioEvent(eAudioEvents, float volume, float speed)` (0x4DD4A0)
    fn frontend_add_audio_event(this: GPtr<u8>, event: u32, volume_bits: u32, speed_bits: u32) = thiscall 0x4DD4A0 => unported;
    /// `CAEFrontendAudioEntity::Reset()` (0x4DD440)
    fn frontend_reset(this: GPtr<u8>) = thiscall 0x4DD440 => unported;
    /// `CAEScriptAudioEntity::Reset()` (0x4EC150)
    fn script_reset_call(this: GPtr<u8>) = thiscall 0x4EC150 => script_reset;
    /// `CAEWeatherAudioEntity::StaticReset()` (0x5052B0)
    fn weather_static_reset() = cdecl 0x5052B0 => unported;
    /// `CAEPedSpeechAudioEntity::Reset()` (0x4E37B0)
    fn ped_speech_reset() = cdecl 0x4E37B0 => unported;
    /// `CAESoundManager::Reset()` (0x4EF4D0)
    fn sm_reset_call(sm: GPtr<u8>) = thiscall 0x4EF4D0 => sound_manager_reset;
    /// `CAESoundManager::PauseManually(uint8)` (0x4EF510)
    fn sm_pause_manually(sm: GPtr<u8>, b: u32) = thiscall 0x4EF510 => unported;
    /// `CAEAudioHardware::GetEffectsFaderScalingFactor()` (0x4D9590, ST0 = the f32 at +0x41C)
    fn hw_get_effects_fader_scaling_factor(hw: GPtr<u8>) -> f32 = thiscall 0x4D9590 => unported;
    /// `CAEAudioHardware::SetEffectsFaderScalingFactor(float)` (0x4D9560)
    fn hw_set_effects_fader_scaling_factor(hw: GPtr<u8>, f_bits: u32) = thiscall 0x4D9560 => unported;
    /// `CAERadioTrackManager::IsRadioOn()` (0x4E8350)
    fn radio_is_radio_on(this: GPtr<u8>) -> u8 = thiscall 0x4E8350 => crate::radio::is_radio_on;
    /// `CAERadioTrackManager::GetCurrentRadioStationID()` (0x4E83F0)
    fn radio_get_current_radio_station_id_call(this: GPtr<u8>) -> u8 = thiscall 0x4E83F0 => radio_get_current_radio_station_id;
    /// `CAEVehicleAudioEntity::StaticGetPlayerVehicleAudioSettingsForRadio()` (0x4F4ED0)
    fn player_vehicle_audio_settings_for_radio() -> GPtr<u8> = cdecl 0x4F4ED0 => unported;
    /// `CAERadioTrackManager::StopRadio(tRadioSettings*, bool)` (0x4E9820)
    fn radio_stop_radio_call(this: GPtr<u8>, settings: GPtr<u8>, b: u32) = thiscall 0x4E9820 => radio_stop_radio;
    /// `CAEAudioHardware::GetTrackPlayTime()` (0x4D8F60)
    fn hw_get_track_play_time(hw: GPtr<u8>) -> u32 = thiscall 0x4D8F60 => unported;
    /// `CAERadioTrackManager::Service(int32 playTime)` (0x4EB9A0)
    fn radio_service(this: GPtr<u8>, t: u32) = thiscall 0x4EB9A0 => unported;
    /// `CAEAudioHardware::Service()` (0x4D9870)
    fn hw_service(hw: GPtr<u8>) = thiscall 0x4D9870 => unported;
    /// `CAEAmbienceTrackManager::IsAmbienceTrackActive()` (0x4D6CF0)
    fn ambience_is_ambience_track_active(this: GPtr<u8>) -> u8 = thiscall 0x4D6CF0 => unported;
    /// `CAudioEngine::StopAmbienceTrack(bool)` (0x507220)
    fn engine_stop_ambience_track(engine: GPtr<u8>, b: u32) = thiscall 0x507220 => unported;
    /// `CAECutsceneTrackManager::IsCutsceneTrackActive()` (0x4DBEB0)
    fn cutscene_is_cutscene_track_active(this: GPtr<u8>) -> u8 = thiscall 0x4DBEB0 => unported;
    /// `CAECutsceneTrackManager::StopCutsceneTrack()` (0x4DBE80)
    fn cutscene_stop_cutscene_track(this: GPtr<u8>) = thiscall 0x4DBE80 => unported;
    /// `CAECutsceneTrackManager::Service(int32 playTime)` (0x4DBFB0)
    fn cutscene_service(this: GPtr<u8>, t: u32) = thiscall 0x4DBFB0 => unported;
    /// `CAECutsceneTrackManager::PreloadCutsceneTrack(int16 track, bool wait)` (0x4DC0E0)
    fn cutscene_preload_cutscene_track(this: GPtr<u8>, track: u32, wait: u32) = thiscall 0x4DC0E0 => unported;
    /// `CAECutsceneTrackManager` 0x4DC130 (beat-track preload, `(int16 track, bool)`)
    fn cutscene_preload_beat_track(this: GPtr<u8>, track: u32, b: u32) = thiscall 0x4DC130 => unported;
}

/// `CAudioEngine::ResetSoundEffects()` (0x507C30): sound manager service, collision entity (+0x4BC) reset,
/// frontend (+0xB4) events 0x24 and 0x23 (volume 0.0, speed 1.0) and reset, script entity (+0x2A0) reset,
/// weather and ped speech static resets, sound manager reset, service and `PauseManually(1)`.
#[hook(addr = 0x507C30, conv = thiscall, name = "CAudioEngine::ResetSoundEffects", expect = "56 8B F1 57 B9 B0 2C B6 00")]
pub fn engine_reset_sound_effects(this: GPtr<u8>) {
    let sm = g::<u8>(crate::common::AE_SOUND_MANAGER);
    sm_service(sm);
    collision_reset(this.byte_add(0x4BC));
    let fe = this.byte_add(0xB4);
    frontend_add_audio_event(fe, 0x24, 0, 0x3F80_0000);
    frontend_add_audio_event(fe, 0x23, 0, 0x3F80_0000);
    frontend_reset(fe);
    script_reset_call(this.byte_add(0x2A0));
    weather_static_reset();
    ped_speech_reset();
    sm_reset_call(sm);
    sm_service(sm);
    sm_pause_manually(sm, 1);
}

/// `CAudioEngine::ServiceLoadingTune(float volume)` (0x5078A0): above the current effects fader factor
/// the volume is limited to `factor + [0x858B4C]` (x87 PC=24); sets the fader factor to it and services
/// the sound manager.
#[hook(addr = 0x5078A0, conv = thiscall, name = "CAudioEngine::ServiceLoadingTune", expect = "B9 B8 F8 B5 00")]
pub fn engine_service_loading_tune(_this: GPtr<u8>, volume: u32) {
    let hw = g::<u8>(AE_AUDIO_HARDWARE);
    let f = x(hw_get_effects_fader_scaling_factor(hw));
    let arg = x(f32::from_bits(volume));
    let mut out = volume;
    if !arg.fcom(f).test_any(0x41) {
        let t = f.fadd_m(g::<f32>(0x858B4C).read());
        if !t.fcom(arg).test_parity(0x05) {
            out = t.to_f32().to_bits();
        }
    }
    hw_set_effects_fader_scaling_factor(hw, out);
    sm_service(g(crate::common::AE_SOUND_MANAGER));
}

/// `CAudioEngine::PreloadCutsceneTrack(int16 track, bool wait)` (0x507E30): stops whatever plays on the
/// music stream first: the radio (remembering its station at +2, `StopRadio(player vehicle settings,
/// true)`, frontend event 0x23, then services radio and hardware until it is off), else an ambience
/// track (`StopAmbienceTrack(true)`), else an active cutscene track (stop, then service until inactive);
/// then tail-calls `CAECutsceneTrackManager::PreloadCutsceneTrack` with the argument words.
#[hook(addr = 0x507E30, conv = thiscall, name = "CAudioEngine::PreloadCutsceneTrack", expect = "56 8B F1 B9 F8 B6 8C 00")]
pub fn engine_preload_cutscene_track(this: GPtr<u8>, track: u32, wait: u32) {
    stop_music_stream_for_preload(this);
    cutscene_preload_cutscene_track(g(crate::common::AE_CUTSCENE_TRACK_MANAGER), track, wait);
}

/// The common head of `CAudioEngine::PreloadCutsceneTrack` / `PreloadBeatTrack`.
fn stop_music_stream_for_preload(this: GPtr<u8>) {
    let radio = g::<u8>(crate::common::AE_RADIO_TRACK_MANAGER);
    let hw = g::<u8>(AE_AUDIO_HARDWARE);
    let cut = g::<u8>(crate::common::AE_CUTSCENE_TRACK_MANAGER);
    if radio_is_radio_on(radio) != 0 {
        let id = radio_get_current_radio_station_id_call(radio);
        wr::<u8>(this, 2, id);
        let settings = player_vehicle_audio_settings_for_radio();
        radio_stop_radio_call(radio, settings, 1);
        frontend_add_audio_event(this.byte_add(0xB4), 0x23, 0, 0x3F80_0000);
        while radio_is_radio_on(radio) != 0 {
            let t = hw_get_track_play_time(hw);
            radio_service(radio, t);
            hw_service(hw);
            sa_core::orig::busy_wait();
        }
    } else if ambience_is_ambience_track_active(g(crate::common::AE_AMBIENCE_TRACK_MANAGER)) != 0 {
        engine_stop_ambience_track(this, 1);
    } else {
        if cutscene_is_cutscene_track_active(cut) != 0 {
            cutscene_stop_cutscene_track(cut);
        }
        while cutscene_is_cutscene_track_active(cut) != 0 {
            let t = hw_get_track_play_time(hw);
            cutscene_service(cut, t);
            hw_service(hw);
            sa_core::orig::busy_wait();
        }
    }
}

/// `CAudioEngine::PreloadBeatTrack(int16 track)` (0x507F40): stops the music stream as
/// `PreloadCutsceneTrack` does, then `CAECutsceneTrackManager` 0x4DC130 `(track, 0)` (the argument word
/// passed through).
#[hook(addr = 0x507F40, conv = thiscall, name = "CAudioEngine::PreloadBeatTrack", expect = "56 8B F1 B9 F8 B6 8C 00")]
pub fn engine_preload_beat_track(this: GPtr<u8>, track: u32) {
    stop_music_stream_for_preload(this);
    cutscene_preload_beat_track(g(crate::common::AE_CUTSCENE_TRACK_MANAGER), track, 0);
}

orig! {
    /// `CAEAmbienceTrackManager::StopSpecialMissionAmbienceTrack()` (0x4D6D60)
    fn ambience_stop_special_mission_ambience_track(this: GPtr<u8>) = thiscall 0x4D6D60 => unported;
    /// `CAERadioTrackManager::Reset()` (0x4E7F80)
    fn radio_reset(this: GPtr<u8>) = thiscall 0x4E7F80 => unported;
    /// `CAEAmbienceTrackManager::Reset()` (0x4D6CA0)
    fn ambience_reset_call(this: GPtr<u8>) -> u32 = thiscall 0x4D6CA0 => ambience_reset;
    /// `CAEWeaponAudioEntity::Reset()` (0x503490)
    fn weapon_reset(this: GPtr<u8>) = thiscall 0x503490 => unported;
    /// `CAEPoliceScannerAudioEntity::Reset()` (0x4E6E90)
    fn ps_reset_call() = cdecl 0x4E6E90 => ps_reset;
}

/// `CAudioEngine::Reset()` (0x507A90): services the sound manager, then stops the music stream: the
/// radio (`StopRadio(null, true)` and service radio + hardware until off), else an ambience track
/// (`StopAmbienceTrack(true)`, `StopSpecialMissionAmbienceTrack`), else an active cutscene track (stop,
/// service until inactive). Then resets collision (+0x4BC), radio, ambience, frontend (+0xB4, events
/// 0x24 / 0x23 first), script (+0x2A0), the weapon entity (+0x1E34, when set), weather, ped speech,
/// police scanner and the sound manager (reset, service, `PauseManually(1)`), and clears +2 (0xFF), +0, +1.
#[hook(addr = 0x507A90, conv = thiscall, name = "CAudioEngine::Reset", expect = "56 8B F1 B9 B0 2C B6 00")]
pub fn engine_reset(this: GPtr<u8>) {
    let sm = g::<u8>(crate::common::AE_SOUND_MANAGER);
    let radio = g::<u8>(crate::common::AE_RADIO_TRACK_MANAGER);
    let hw = g::<u8>(AE_AUDIO_HARDWARE);
    let cut = g::<u8>(crate::common::AE_CUTSCENE_TRACK_MANAGER);
    let amb = g::<u8>(crate::common::AE_AMBIENCE_TRACK_MANAGER);
    sm_service(sm);
    if radio_is_radio_on(radio) != 0 {
        radio_stop_radio_call(radio, GPtr::from_addr(0), 1);
        while radio_is_radio_on(radio) != 0 {
            let t = hw_get_track_play_time(hw);
            radio_service(radio, t);
            hw_service(hw);
            sa_core::orig::busy_wait();
        }
    } else if ambience_is_ambience_track_active(amb) != 0 {
        engine_stop_ambience_track(this, 1);
        ambience_stop_special_mission_ambience_track(amb);
    } else if cutscene_is_cutscene_track_active(cut) != 0 {
        cutscene_stop_cutscene_track(cut);
        while cutscene_is_cutscene_track_active(cut) != 0 {
            let t = hw_get_track_play_time(hw);
            cutscene_service(cut, t);
            hw_service(hw);
            sa_core::orig::busy_wait();
        }
    }
    collision_reset(this.byte_add(0x4BC));
    radio_reset(radio);
    ambience_reset_call(amb);
    let fe = this.byte_add(0xB4);
    frontend_add_audio_event(fe, 0x24, 0, 0x3F80_0000);
    frontend_add_audio_event(fe, 0x23, 0, 0x3F80_0000);
    frontend_reset(fe);
    script_reset_call(this.byte_add(0x2A0));
    let weapon = rd::<GPtr<u8>>(this, 0x1E34);
    if !weapon.is_null() {
        weapon_reset(weapon);
    }
    weather_static_reset();
    ped_speech_reset();
    ps_reset_call();
    sm_reset_call(sm);
    sm_service(sm);
    sm_pause_manually(sm, 1);
    wr::<u8>(this, 2, 0xFF);
    wr::<u8>(this, 0, 0);
    wr::<u8>(this, 1, 0);
}

orig! {
    /// CRT `_stricmp` (0x8229B6)
    fn crt_stricmp(a: u32, b: u32) -> i32 = cdecl 0x8229B6 => unported;
}

/// `CAudioZones::SwitchAudioZone(const char* name, bool enable)` (0x508320): sets bit 0 of the flags word
/// (+0xA) of every audio box (0xB6EBA8, 0x1C bytes, count 0xB6DCC8) and then every audio sphere (0xB6DCD0,
/// 0x18 bytes, count 0xB6DCC4) whose name equals `name` (`_stricmp`), to `enable & 1`. Counts are re-read
/// each iteration.
#[hook(addr = 0x508320, conv = cdecl, name = "CAudioZones::SwitchAudioZone", expect = "A1 C8 DC B6 00")]
pub fn audio_zones_switch_audio_zone(name: u32, enable: u32) {
    let bit = (enable as u8 & 1) as u16;
    for (count, base, stride) in [
        (0xB6DCC8usize, 0xB6EBA8usize, 0x1Cusize),
        (0xB6DCC4, 0xB6DCD0, 0x18),
    ] {
        let mut i = 0usize;
        while (i as i32) < g::<i32>(count).read() {
            let zone = base + i * stride;
            if crt_stricmp(name, zone as u32) == 0 {
                let f = g::<u16>(zone + 0xA);
                f.write((f.read() & 0xFFFE) | bit);
            }
            i += 1;
        }
    }
}

/// `FUN_004dc670` (0x4DC670, a `CVector ==` helper, cdecl): 1 when the three components of `*a` and `*b`
/// compare equal (x87 `fcomp`, so -0.0 == 0.0 and NaN != NaN), else 0 (full EAX).
#[hook(addr = 0x4DC670, conv = cdecl, name = "FUN_004dc670", expect = "8B 4C 24 04 D9 01")]
pub fn vector_equal(a: GPtr<u8>, b: GPtr<u8>) -> u32 {
    for off in [0isize, 4, 8] {
        if x(rd::<f32>(a, off)).fcom(x(rd::<f32>(b, off))).test_parity(0x44) {
            return 0;
        }
    }
    1
}

/// `FUN_004e3350` (0x4E3350, a ped-speech slot initialiser, thiscall): word +0 = 0, +4 / +8 / +0x10 = 0,
/// words +0xC / +0xE / +0x14 / +0x16 = -1, bytes +0x19 / +0x1A / +0x18 = 0 (store order of the bytes).
/// Returns the original's EAX (0); EDX = -1 is not modelled (both callers overwrite it).
#[hook(addr = 0x4E3350, conv = thiscall, name = "FUN_004e3350", expect = "33 C0 83 CA FF")]
pub fn ped_speech_slot_init(this: GPtr<u8>) -> u32 {
    wr::<u16>(this, 0, 0);
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0);
    wr::<u16>(this, 0xC, 0xFFFF);
    wr::<u16>(this, 0xE, 0xFFFF);
    wr::<u32>(this, 0x10, 0);
    wr::<u16>(this, 0x14, 0xFFFF);
    wr::<u16>(this, 0x16, 0xFFFF);
    wr::<u8>(this, 0x19, 0);
    wr::<u8>(this, 0x1A, 0);
    wr::<u8>(this, 0x18, 0);
    0
}

/// `CAECutsceneTrackManager::PreloadCutsceneTrack(int16 track, bool wait)` (0x4DC0E0): track id +0xC
/// (sign-extended), +0x14 = 0, status +8 = 0; with `wait` services itself (track play time) and the
/// hardware until the status is 2.
#[hook(addr = 0x4DC0E0, conv = thiscall, name = "CAECutsceneTrackManager::PreloadCutsceneTrack", expect = "56 8B F1 0F BF 4C 24 08")]
pub fn cutscene_preload_cutscene_track_port(this: GPtr<u8>, track: u32, wait: u32) {
    wr::<i32>(this, 0xC, i32::from(track as i16));
    wr::<u32>(this, 0x14, 0);
    wr::<u32>(this, 8, 0);
    if wait as u8 == 0 {
        return;
    }
    let hw = g::<u8>(AE_AUDIO_HARDWARE);
    loop {
        let t = hw_get_track_play_time(hw);
        cutscene_service(this, t);
        hw_service(hw);
        if rd::<u32>(this, 8) == 2 {
            break;
        }
        // the stream thread loads the track meanwhile (phase B's deterministic mode steps it here)
        sa_core::orig::busy_wait();
    }
}

orig! {
    /// `CAEScriptAudioEntity::ProcessMissionAudioEvent(eAudioEvents, CVector& pos, CPhysical*, float, float)` (0x4ECCF0)
    fn script_process_mission_audio_event(this: GPtr<u8>, event: u32, pos: GPtr<u8>, physical: u32, a: u32, b: u32) = thiscall 0x4ECCF0 => unported;
    /// `CAEAudioEnvironment::GetDopplerRelativeFrequency(float, float, uint32, uint32, float)` (0x4D7E40, ST0)
    fn env_get_doppler_relative_frequency(a: u32, b: u32, c: u32, d: u32, e: u32) -> f32 = cdecl 0x4D7E40 => unported;
}

/// `CAEScriptAudioEntity::ReportMissionAudioEvent(eAudioEvents, CPhysical*, float, float)` (0x4EE960,
/// catalogue `-1`): `ProcessMissionAudioEvent(event, (-1000, -1000, -1000), physical, a, b)` with a local
/// position, the argument words passed through.
#[hook(addr = 0x4EE960, conv = thiscall, name = "CAEScriptAudioEntity::ReportMissionAudioEvent-1", expect = "83 EC 0C 8B 44 24 1C")]
pub fn script_report_mission_audio_event_physical(this: GPtr<u8>, event: u32, physical: u32, a: u32, b: u32) {
    let mut pos = [0xC47A_0000u32; 3];
    let p = GPtr::from_raw(pos.as_mut_ptr().cast::<u8>());
    script_process_mission_audio_event(this, event, p, physical, a, b);
}

/// `CAESound::GetRelativePlaybackFrequencyWithDoppler()` (0x4EF400, ST0): the speed (+0x64) for
/// relative (frontend) sounds, else `GetDopplerRelativeFrequency(+0x4C, +0x48, +0x44, +0x40, +0x50) *
/// speed` (x87 PC=24; the floats pass through `fld`/`fstp`).
#[hook(addr = 0x4EF400, conv = thiscall, name = "CAESound::GetRelativePlaybackFrequencyWithDoppler", expect = "56 8B F1 F6 46 56 01")]
pub fn sound_get_relative_playback_frequency_with_doppler(this: GPtr<u8>) -> f32 {
    if rd::<u8>(this, 0x56) & 1 != 0 {
        return x(rd::<f32>(this, 0x64)).to_f32();
    }
    let fb = |off: isize| x(rd::<f32>(this, off)).to_f32().to_bits();
    let d = env_get_doppler_relative_frequency(
        fb(0x4C),
        fb(0x48),
        rd::<u32>(this, 0x44),
        rd::<u32>(this, 0x40),
        fb(0x50),
    );
    x(d).fmul_m(rd::<f32>(this, 0x64)).to_f32()
}

orig! {
    /// `CTheZones::FindSmallestZoneForPosition(const CVector&, bool)` (0x572360)
    fn zones_find_smallest_zone_for_position(pos: u32, b: u32) -> u32 = cdecl 0x572360 => unported;
    /// `CWorld::FindPlayerWanted(int32)` (0x56E230)
    fn world_find_player_wanted(id: i32) -> u32 = cdecl 0x56E230 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn world_find_player_ped(id: i32) -> u32 = cdecl 0x56E210 => unported;
    /// `CVehicle::GetRemapIndex()` (0x6D0B70)
    fn vehicle_get_remap_index(veh: GPtr<u8>) -> i32 = thiscall 0x6D0B70 => unported;
    /// `CPedIntelligence::GetTaskSwim()` (0x601070)
    fn ped_intelligence_get_task_swim(intel: u32) -> u32 = thiscall 0x601070 => unported;
    /// `CPedIntelligence::GetTaskJetPack()` (0x601110)
    fn ped_intelligence_get_task_jet_pack(intel: u32) -> u32 = thiscall 0x601110 => unported;
    /// `CAEPoliceScannerAudioEntity::PlayPoliceScannerDialogue(int16* a, int16* b)` (0x4E6ED0)
    fn ps_play_police_scanner_dialogue_call(this: GPtr<u8>, a: u32, b: u32) = thiscall 0x4E6ED0 => ps_play_police_scanner_dialogue;
}

/// `CAEPoliceScannerAudioEntity::AddAudioEvent(eAudioEvents, eCrimeType, const CVector&)` (0x4E71E0): for
/// event 0xA4 and crime types 2..22 builds two 10-entry (bank, sound) int16 lists (initialised to -1) and
/// plays them as a scanner dialogue. List A: an intro (0x96, `[0x8C8160 + 2 * GetRandomNumberInRange(0,
/// 3)]`), the crime (0x97, `[0x8C8168 + 2 * crime]`), then for the smallest zone at the position whose
/// 8-byte label matches one of the 194 entries of 0x8C8198 (with a sound in 0x8C87A8 >= 0): when
/// 0x8C8930 flags it, north/south (0x95, 2 / 3) and east/west/centre (0x95, 1 / 4 / 0, skipped after a
/// north/south word) from the zone box (x87 PC=24: centre = min + size * [0x858B8C], margin = size *
/// [0x858C84]), and the zone name (0x93, `[0x8C87A8]`); unknown zones end the event. List B describes the
/// player whose wanted record (+0x21C) is this entity: in a vehicle with a known model class (+0x1D4,
/// 0..45, `[0x8C89F8] >= 0`): (0x96, 7), (0x96, 1 or 3 by class), a colour from the remap index or the
/// paint byte +0x434 (0x94, ...) when 0x8C8A54 allows, and the model (0x98, ...); on foot: swimming (2) or
/// not jetpacking (4) as (0x96, 7) ... (0x96, n).
#[hook(addr = 0x4E71E0, conv = thiscall, name = "CAEPoliceScannerAudioEntity::AddAudioEvent", expect = "83 EC 30 8B 44 24 34")]
pub fn ps_add_audio_event(this: GPtr<u8>, event: u32, crime: u32, pos: GPtr<u8>) {
    let mut a = [0xFFFFu16; 10];
    let mut b = [0xFFFFu16; 10];
    if event != 0xA4 {
        return;
    }
    let crime = crime as i32;
    if crime <= 1 || crime >= 0x17 {
        return;
    }
    a[0] = 0x96;
    let r = crate::common::ae_random_int(0, 3);
    let intro = g::<u16>((0x8C8160 + r * 2) as u32 as usize).read();
    let crime_word = g::<u16>((0x8C8168 + crime * 2) as usize).read();
    a[1] = intro;
    a[2] = 0x97;
    a[3] = crime_word;
    let zone = zones_find_smallest_zone_for_position(pos.addr() as u32, 1);
    if zone == 0 {
        return;
    }
    let z = g::<u8>(zone as usize);
    let (l0, l1) = (rd::<u32>(z, 8), rd::<u32>(z, 0xC));
    let Some(idx) = (0..0xC2usize).find(|&i| {
        g::<u32>(0x8C8198 + i * 8).read() == l0
            && g::<u32>(0x8C819C + i * 8).read() == l1
            && g::<i16>(0x8C87A8 + i * 2).read() >= 0
    }) else {
        return;
    };
    if g::<u8>(0x8C8930 + idx).read() != 0 {
        let x1 = i32::from(rd::<i16>(z, 0x10));
        let y1 = i32::from(rd::<i16>(z, 0x12));
        let w = i32::from(rd::<i16>(z, 0x16)) - x1;
        let h = i32::from(rd::<i16>(z, 0x18)) - y1;
        let k = g::<f32>(0x858B8C).read();
        let k2 = g::<f32>(0x858C84).read();
        let xi = crate::common::xi;
        let cx = xi(w).fmul_m(k).fadd(xi(x1)).to_f32();
        let cy = x(k).fmul(xi(h)).fadd(xi(y1)).to_f32();
        let qx = xi(w).fmul_m(k2).to_f32();
        let hk = xi(h).fmul_m(k2);
        let py = x(rd::<f32>(pos, 4));
        let mut ns = false;
        if !py.fcom(x(cy).fadd(hk)).test_any(0x41) {
            a[5] = 2;
            ns = true;
        } else if !py.fcom(x(cy).fsub(hk)).test_parity(0x05) {
            a[5] = 3;
            ns = true;
        }
        if ns {
            a[4] = 0x95;
        }
        let px = rd::<f32>(pos, 0);
        if !x(qx).fadd_m(cx).fcom(x(px)).test_parity(0x05) {
            a[7] = 1;
            a[6] = 0x95;
        } else if !x(cx).fsub_m(qx).fcom(x(px)).test_any(0x41) {
            a[7] = 4;
            a[6] = 0x95;
        } else if !ns {
            a[7] = 0;
            a[6] = 0x95;
        }
    }
    a[8] = 0x93;
    a[9] = g::<u16>(0x8C87A8 + idx * 2).read();
    let mut ped = 0u32;
    for i in 0..2 {
        if world_find_player_wanted(i).wrapping_add(0x21C) == this.addr() as u32 {
            ped = world_find_player_ped(i);
        }
    }
    if ped != 0 {
        let p = g::<u8>(ped as usize);
        if rd::<u32>(p, 0x46C) & 0x100 != 0 {
            let veh = rd::<GPtr<u8>>(p, 0x58C);
            if !veh.is_null() {
                let class = i32::from(rd::<i8>(veh, 0x1D4));
                if (0..0x2E).contains(&class) && g::<i16>(0x8C89F8 + class as usize * 2).read() >= 0 {
                    let c = class as usize;
                    b[0] = 0x96;
                    b[1] = 7;
                    b[2] = 0x96;
                    b[3] = if matches!(class, 0x19 | 0x1D | 0x1E | 0x25 | 0x29 | 0x22) {
                        3
                    } else {
                        1
                    };
                    if g::<u8>(0x8C8A54 + c).read() != 0 {
                        if vehicle_get_remap_index(veh) != -1 {
                            b[7] = 4;
                            b[6] = 0x94;
                        } else {
                            let paint = rd::<u8>(veh, 0x434);
                            if paint != 0 && paint < 0x7F {
                                let colour = g::<i16>(0x8C8A88 + usize::from(paint) * 2).read();
                                if colour >= 0 {
                                    b[7] = colour as u16;
                                    b[6] = 0x94;
                                }
                            }
                        }
                    }
                    b[8] = 0x98;
                    b[9] = g::<u16>(0x8C89F8 + c * 2).read();
                }
            }
        } else {
            let intel = rd::<u32>(p, 0x47C);
            let foot = if ped_intelligence_get_task_swim(intel) != 0 {
                Some(2)
            } else if ped_intelligence_get_task_jet_pack(intel) == 0 {
                Some(4)
            } else {
                None
            };
            if let Some(n) = foot {
                b[9] = n;
                b[0] = 0x96;
                b[1] = 7;
                b[8] = 0x96;
            }
        }
    }
    let pa = GPtr::from_raw(a.as_mut_ptr().cast::<u8>()).addr() as u32;
    let pb = GPtr::from_raw(b.as_mut_ptr().cast::<u8>()).addr() as u32;
    ps_play_police_scanner_dialogue_call(this, pa, pb);
}

orig! {
    /// `CAESound::Initialise(int16 bankSlot, int16 sfx, CAEAudioEntity*, CVector pos, float volume, float
    /// maxDistance, float speed, float doppler, uint8 frameDelay, uint32 flags, float freqVariance, int16
    /// playTime)` (0x4EFE50, `ret 0x38`)
    fn sound_initialise(sound: GPtr<u8>, slot: u32, sfx: u32, entity: GPtr<u8>, px: u32, py: u32, pz: u32, vol: u32, max_dist: u32, speed: u32, doppler: u32, delay: u32, flags: u32, freq_var: u32, play_time: u32) = thiscall 0x4EFE50 => unported;
    /// `CAESoundManager::RequestNewSound(CAESound*)` (0x4EFB10)
    fn sm_request_new_sound(sm: GPtr<u8>, sound: GPtr<u8>) -> u32 = thiscall 0x4EFB10 => unported;
    /// `CAESound::~CAESound()` (0x4EF660)
    fn sound_dtor(sound: GPtr<u8>) = thiscall 0x4EF660 => unported;
}

/// `CAEPoliceScannerAudioEntity::PlayLoadedDialogue()` (0x4E6F60): picks the first loaded slot (state 3)
/// at or after the first playing one (state 5; from slot 0 when none) and plays it: a stack `CAESound`
/// (only its entity pointer +8 pre-cleared, then `Initialise(slot + 33, sound id of the list pair, this,
/// (0, 1, 0), volume = (int8) [[0xBD00F8] + 0xA4] + [0xB61CF8], 1, 1, 1, 0, 0, 0, 0)`, +0x10 = (float) slot
/// index, event +0xC = 0xA4, flags +0x56 = 0x107), `RequestNewSound` into 0xB61D00; a granted sound marks
/// the slot 5 and the state 7. Without a loaded slot: reports events 0x31 (`[0xB61D54]` + offset) and
/// 0x30; a second list already used (0xB61D04) finishes the dialogue, else switches to list B (0xB61D20),
/// re-preps the slots, clears 0xB61D0C, sets 0xB61D08 = now + `[0x8C8148]` and the state 2. The stack sound
/// is destroyed (`~CAESound`) on every path. Its bytes not written by `Initialise` are zero here and
/// stack leftovers in the original (copied by `RequestNewSound`).
#[hook(addr = 0x4E6F60, conv = thiscall, name = "CAEPoliceScannerAudioEntity::PlayLoadedDialogue", expect = "6A FF 68 78 C0 83 00", peek = 0x80)]
pub fn ps_play_loaded_dialogue_port(this: GPtr<u8>) {
    // the original's `CAESound` local (entry ESP - 0x80) is uninitialised: its stack residue (`peek`)
    let mut buf = crate::port3_lead::stack_local::<0x1D>(0x4E6F60, -0x80);
    let sound = GPtr::from_raw(buf.as_mut_ptr().cast::<u8>());
    wr::<u32>(sound, 8, 0);
    let slot_state = |i: usize| g::<u16>(PS_SLOTS + i * 2).read();
    let start = (0..5usize).find(|&i| slot_state(i) == 5).unwrap_or(0);
    let loaded = (start..5usize).find(|&i| slot_state(i) == 3);
    if let Some(i) = loaded {
        let v = i32::from(rd::<i8>(g::<u8>(g::<u32>(0xBD00F8).read() as usize), 0xA4));
        let vol = crate::common::xi(v)
            .fadd_m(g::<f32>(PS_CF8).read())
            .to_f32()
            .to_bits();
        let list = g::<u32>(PS_SLOT_LIST).read();
        let sfx = u32::from(g::<u16>(list as usize + i * 4 + 2).read());
        sound_initialise(
            sound,
            i as u32 + 0x21,
            sfx,
            this,
            0,
            0x3F80_0000,
            0,
            vol,
            0x3F80_0000,
            0x3F80_0000,
            0x3F80_0000,
            0,
            0,
            0,
            0,
        );
        wr::<f32>(sound, 0x10, crate::common::xi(i as i32).to_f32());
        wr::<u16>(sound, 0x56, 0x107);
        wr::<u32>(sound, 0xC, 0xA4);
        let got = sm_request_new_sound(g(crate::common::AE_SOUND_MANAGER), sound);
        g::<u32>(PS_SOUND).write(got);
        if got != 0 {
            g::<u16>(PS_SLOTS + i * 2).write(5);
            g::<u16>(PS_PLAYBACK_STATE).write(7);
        }
    } else {
        let engine = g::<u8>(crate::common::AUDIO_ENGINE);
        let v = x(g::<f32>(0xB61D54).read())
            .fadd_m(g::<f32>(PS_CF8).read())
            .to_f32();
        engine_report_frontend_audio_event(engine, 0x31, v.to_bits(), 0x3F80_0000);
        engine_report_frontend_audio_event(engine, 0x30, 0, 0x3F80_0000);
        if g::<u16>(PS_D04).read() != 0 {
            ps_finished_playing_call();
        } else {
            g::<u32>(PS_SLOT_LIST).write(PS_LIST_B as u32);
            g::<u16>(PS_D04).write(1);
            for k in 0..5usize {
                let e = g::<i16>(PS_LIST_B + k * 4);
                let neg = e.read() < 0 || e.byte_add(2).read() < 0;
                g::<u16>(PS_SLOTS + k * 2).write(u16::from(neg));
            }
            let t = g::<u32>(TIMER_MS).read().wrapping_add(g::<u32>(0x8C8148).read());
            g::<u32>(PS_D0C).write(0);
            g::<u32>(PS_D08).write(t);
            g::<u16>(PS_PLAYBACK_STATE).write(2);
        }
    }
    sound_dtor(sound);
}

orig! {
    /// `CAEAudioHardware::IsSoundBankLoaded(uint16 bank, int16 slot)` (0x4D88C0)
    fn hw_is_sound_bank_loaded(hw: GPtr<u8>, bank: u32, slot: u32) -> u8 = thiscall 0x4D88C0 => unported;
    /// `FUN_004dc670` (0x4DC670, `CVector ==`)
    fn vector_equal_call(a: GPtr<u8>, b: GPtr<u8>) -> u32 = cdecl 0x4DC670 => vector_equal;
    /// `CAESound::RegisterWithPhysicalEntity(CEntity*)` (0x4EF820)
    fn sound_register_with_physical_entity(sound: GPtr<u8>, entity: u32) = thiscall 0x4EF820 => unported;
}

/// `CAEScriptAudioEntity::PlayResidentSoundEvent(int16 slot, int16 bank, int16 sfx, int32 event, CVector*
/// pos, CPhysical*, float volumeDelta, float speed, int16 playTime, float maxDistance)` (0x4EC550): when
/// the bank is loaded in the slot, (re)initialises the entity's own sound (+8) at the physical's position,
/// else at `pos`, or frontend-relative at (0, 1, 0) when `pos` is unset (-1000 bitwise) or (0, 0, 0)
/// (`FUN_004dc670`); volume = `(int8) [[0xBD00F8] + event] + volumeDelta` (x87 PC=24); then play time
/// (+0x64 = sound +0x5C) = `playTime`, flags word +0x5E = 0x26, `SetFlags(1, relative)`, with a physical
/// `SetFlags(0x80, 1)` + `RegisterWithPhysicalEntity`, event (+0x14) = `(uint16) event` and
/// `RequestNewSound`. The volume reuses the original's second argument slot (not modelled).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x4EC550, conv = thiscall, name = "CAEScriptAudioEntity::PlayResidentSoundEvent", expect = "83 EC 10 8B 44 24 14")]
pub fn script_play_resident_sound_event_port(
    this: GPtr<u8>,
    slot: u32,
    bank: u32,
    sfx: u32,
    event: u32,
    pos: GPtr<u8>,
    physical: u32,
    vol_delta: u32,
    speed: u32,
    play_time: u32,
    max_dist: u32,
) {
    if hw_is_sound_bank_loaded(g(AE_AUDIO_HARDWARE), bank, slot) == 0 {
        return;
    }
    let ev = event as u16;
    let base = i32::from(rd::<i8>(g::<u8>(g::<u32>(0xBD00F8).read() as usize), ev as isize));
    let vol = crate::common::xi(base)
        .fadd_m(f32::from_bits(vol_delta))
        .to_f32()
        .to_bits();
    let mut relative = 0u32;
    let (px, py, pz);
    if physical != 0 {
        let e = g::<u8>(physical as usize);
        let m = rd::<u32>(e, 0x14);
        let p = if m != 0 {
            g::<u8>(m as usize + 0x30)
        } else {
            e.byte_add(4)
        };
        (px, py, pz) = (rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8));
    } else {
        let unset = (0..3).all(|k| rd::<u32>(pos, k * 4) == 0xC47A_0000);
        let mut zero = [0u32; 3];
        let at_origin =
            !unset && vector_equal_call(pos, GPtr::from_raw(zero.as_mut_ptr().cast::<u8>())) as u8 != 0;
        if unset || at_origin {
            (px, py, pz) = (0, 0x3F80_0000, 0);
            relative = 1;
        } else {
            (px, py, pz) = (rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8));
        }
    }
    let sound = this.byte_add(8);
    sound_initialise(
        sound,
        slot,
        sfx,
        this,
        px,
        py,
        pz,
        vol,
        max_dist,
        speed,
        0x3F80_0000,
        0,
        0,
        0,
        0,
    );
    wr::<u16>(this, 0x64, play_time as u16);
    wr::<u16>(this, 0x5E, 0x26);
    sound_set_flags(sound, 1, relative);
    if physical != 0 {
        sound_set_flags(sound, 0x80, 1);
        sound_register_with_physical_entity(sound, physical);
    }
    wr::<u32>(this, 0x14, u32::from(ev));
    sm_request_new_sound(g(crate::common::AE_SOUND_MANAGER), sound);
}

/// `CAEScriptAudioEntity::PlayMissionBankSound(int32 event, CVector* pos, CPhysical*, int16 sfx, uint8
/// slot, bool skipIfPlaying, float volumeDelta, float maxDistance, float speed)` (0x4EC6D0): for mission
/// slots 2..3 (and, with `skipIfPlaying`, no sound of this event playing for the entity) whose bank (+0xA8 of
/// the slot record) is loaded in bank slot `slot + 26`: (re)initialises the entity's sound (+8) like
/// `PlayResidentSoundEvent`, with these position rules: the physical's position; else `pos` compared
/// (`FUN_004dc670`) with (-1000, -1000, -1000) and (0, 0, 0) (relative at (0, 1, 0)), then (-1, 0, 0) and
/// (1, 0, 0) (relative at `pos`), else `pos`; flags word +0x5E = 6. The original reuses its argument slots
/// for the volume, bank slot and event (not modelled).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x4EC6D0, conv = thiscall, name = "CAEScriptAudioEntity::PlayMissionBankSound", expect = "83 EC 10 53 8A 5C 24 28")]
pub fn script_play_mission_bank_sound_port(
    this: GPtr<u8>,
    event: u32,
    pos: GPtr<u8>,
    physical: u32,
    sfx: u32,
    slot: u32,
    skip_if_playing: u32,
    vol_delta: u32,
    max_dist: u32,
    speed: u32,
) {
    let sl = slot as u8;
    if !(2..4).contains(&sl) {
        return;
    }
    if skip_if_playing as u8 != 0
        && sm_are_sounds_of_this_event_playing_for_this_entity(
            g(crate::common::AE_SOUND_MANAGER),
            event,
            this,
        ) != 0
    {
        return;
    }
    let bank_slot = u32::from(sl) + 0x1A;
    let bank = u32::from(rd::<u16>(this, isize::from(sl) * 32 + 0xA8));
    if hw_is_sound_bank_loaded(g(AE_AUDIO_HARDWARE), bank, bank_slot) == 0 {
        return;
    }
    let ev = event as u16;
    let base = i32::from(rd::<i8>(g::<u8>(g::<u32>(0xBD00F8).read() as usize), ev as isize));
    let vol = crate::common::xi(base)
        .fadd_m(f32::from_bits(vol_delta))
        .to_f32()
        .to_bits();
    let mut relative = 0u32;
    let (px, py, pz);
    if physical != 0 {
        let e = g::<u8>(physical as usize);
        let m = rd::<u32>(e, 0x14);
        let p = if m != 0 {
            g::<u8>(m as usize + 0x30)
        } else {
            e.byte_add(4)
        };
        (px, py, pz) = (rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8));
    } else {
        let mut v = [0u32; 3];
        let vp = GPtr::from_raw(v.as_mut_ptr().cast::<u8>());
        let mut is = |vals: [u32; 3]| {
            v = vals;
            vector_equal_call(pos, vp) as u8 != 0
        };
        let own = (rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8));
        if is([0xC47A_0000; 3]) || is([0; 3]) {
            (px, py, pz) = (0, 0x3F80_0000, 0);
            relative = 1;
        } else if is([0xBF80_0000, 0, 0]) || is([0x3F80_0000, 0, 0]) {
            (px, py, pz) = (rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8));
            relative = 1;
        } else {
            (px, py, pz) = own;
        }
    }
    let sound = this.byte_add(8);
    sound_initialise(
        sound,
        bank_slot,
        sfx,
        this,
        px,
        py,
        pz,
        vol,
        max_dist,
        speed,
        0x3F80_0000,
        0,
        0,
        0,
        0,
    );
    wr::<u16>(this, 0x5E, 6);
    sound_set_flags(sound, 1, relative);
    if physical != 0 {
        sound_set_flags(sound, 0x80, 1);
        sound_register_with_physical_entity(sound, physical);
    }
    wr::<u32>(this, 0x14, u32::from(ev));
    sm_request_new_sound(g(crate::common::AE_SOUND_MANAGER), sound);
}

orig! {
    /// `CStats::GetFullFavoriteRadioStationList()` (0x558F90)
    fn stats_get_full_favorite_radio_station_list() -> GPtr<u8> = cdecl 0x558F90 => unported;
}

/// `CAERadioTrackManager::Reset()` (0x4E7F80): clears the flags, copies the 14 favourite-station listen
/// times (`CStats::GetFullFavoriteRadioStationList`) to +0x18, resets per station (in this order) the music
/// (0xB62B40, 5), ident (0xB62980, 8), advert (0xB620C0, 40) and DJ banter (0xB61D78, 15) histories to -1,
/// the station record (+0x100 + 0x2C * station: +0xC/+0x10/+0x14 = -1, day/hour bytes = 0xFF, three
/// (length 0, track -1, type 6) entries) and the retune byte (+8 + station); then picks a random station
/// `GetRandomNumberInRange(1, 13)` (+0xAD), clears the queue (+0x88 ids -1, +0xB4 types 6, +0xBB indices
/// -1) and the current settings and copies them to the previous settings (+0xC4, 15 dwords); finally the
/// special banter state (0xB62C73 = 3, 0xB62C72 = -1) and the statistics bytes (as `ResetStatistics`).
#[hook(addr = 0x4E7F80, conv = thiscall, name = "CAERadioTrackManager::Reset", expect = "83 EC 10 53 55")]
pub fn radio_reset_port(this: GPtr<u8>) {
    wr::<u8>(this, 0, 0);
    wr::<u8>(this, 1, 0);
    let fav = stats_get_full_favorite_radio_station_list();
    for i in 0..14isize {
        wr::<u32>(this, 0x18 + i * 4, rd::<u32>(fav, i * 4));
    }
    let fill = |base: usize, n: usize| {
        for j in 0..n {
            g::<u32>(base + j * 4).write(0xFFFF_FFFF);
        }
    };
    for k in 0..14usize {
        fill(0xB62B40 + k * 0x14, 5);
        fill(0xB62980 + k * 0x20, 8);
        fill(0xB620C0 + k * 0xA0, 40);
        fill(0xB61D78 + k * 0x3C, 15);
        let r = 0x100 + k as isize * 0x2C;
        wr::<i32>(this, r + 0xC, -1);
        wr::<i32>(this, r + 0x10, -1);
        wr::<u8>(this, r + 0x27, 0xFF);
        wr::<u8>(this, r + 0x28, 0xFF);
        wr::<i32>(this, r + 0x14, -1);
        for j in 0..3isize {
            wr::<u32>(this, r + 4 * j, 0);
            wr::<i32>(this, r + 0x18 + 4 * j, -1);
            wr::<u8>(this, r + 0x24 + j, 6);
        }
        wr::<u8>(this, 8 + k as isize, 0);
    }
    let st = crate::common::ae_random_int(1, 13);
    wr::<u8>(this, 0xAD, st as u8);
    wr::<u8>(this, 0xAE, 0);
    wr::<u8>(this, 0xAC, 2);
    for i in 0..5isize {
        wr::<u8>(this, 0xBB + i, 0xFF);
        wr::<i32>(this, 0x88 + 4 * i, -1);
        wr::<u8>(this, 0xB4 + i, 6);
    }
    wr::<i32>(this, 0x9C, -1);
    wr::<i32>(this, 0xA0, -1);
    wr::<u32>(this, 0xA4, 0);
    wr::<u32>(this, 0xA8, 0);
    wr::<u8>(this, 0xB9, 6);
    wr::<u8>(this, 0xC0, 0xFF);
    wr::<u8>(this, 0xBA, 6);
    wr::<u8>(this, 0xC1, 0xFF);
    for off in [0x6C, 0x70, 0x50, 0x54, 0x58] {
        wr::<u32>(this, off, 0);
    }
    wr::<u8>(this, 2, 0);
    wr::<u32>(this, 0x60, 0);
    wr::<u32>(this, 0x5C, 0);
    wr::<u8>(this, 3, 0);
    wr::<u8>(this, 0x16, 0xFF);
    wr::<u8>(this, 0x17, 0xFF);
    wr::<u8>(this, 7, 1);
    wr::<u8>(this, 4, 1);
    for i in 0..15isize {
        wr::<u32>(this, 0xC4 + i * 4, rd::<u32>(this, 0x88 + i * 4));
    }
    wr::<i32>(this, 0x74, -1);
    wr::<i32>(this, 0x78, -1);
    wr::<i32>(this, 0x7C, -1);
    g::<u8>(0xB62C73).write(3);
    g::<u8>(0xB62C72).write(0xFF);
    wr::<u8>(this, 5, 0);
    wr::<u8>(this, 6, 0);
    wr::<u32>(this, 0x80, 0);
    wr::<u32>(this, 0x84, 0);
    for a in (0xB62C5B..=0xB62C71usize).rev() {
        g::<u8>(a).write(0);
    }
    g::<u8>(0xB62C5A).write(0xFF);
    g::<u8>(0xB62C59).write(0xFF);
    g::<u8>(0xB62C58).write(0xFF);
}

orig! {
    /// `CAEAudioHardware::GetTrackLengthMs()` (0x4D8F70)
    fn hw_get_track_length_ms(hw: GPtr<u8>) -> u32 = thiscall 0x4D8F70 => unported;
    /// `CAEAudioHardware::GetPlayingTrackID()` (0x4D8F90)
    fn hw_get_playing_track_id(hw: GPtr<u8>) -> u32 = thiscall 0x4D8F90 => unported;
    /// `CAEAudioHardware::GetActiveTrackID()` (0x4D8F80)
    fn hw_get_active_track_id(hw: GPtr<u8>) -> u32 = thiscall 0x4D8F80 => unported;
    /// `CAEAudioHardware::StopTrack()` (0x4D8F50)
    fn hw_stop_track(hw: GPtr<u8>) = thiscall 0x4D8F50 => unported;
    /// `CAEAudioHardware::SetBassSetting(int8, float)` (0x4D94A0)
    fn hw_set_bass_setting(hw: GPtr<u8>, setting: u32, gain_bits: u32) = thiscall 0x4D94A0 => unported;
    /// `CAEAudioHardware::PlayTrack(uint32, int32, uint32, uint8, bool, bool)` (0x4D8F10)
    fn hw_play_track(hw: GPtr<u8>, id: u32, next: u32, offset: u32, flags: u32, user: u32, user_next: u32) = thiscall 0x4D8F10 => unported;
    /// `CAERadioTrackManager::CheckForMissionStatsChanges()` (0x4E8410)
    fn radio_check_for_mission_stats_changes_call(this: GPtr<u8>) = thiscall 0x4E8410 => radio_check_for_mission_stats_changes;
    /// `CAERadioTrackManager::CheckForStationRetune()` (0x4EB660)
    fn radio_check_for_station_retune(this: GPtr<u8>) = thiscall 0x4EB660 => unported;
    /// `CAERadioTrackManager::CheckForStationRetuneDuringPause()` (0x4EB890)
    fn radio_check_for_station_retune_during_pause_call(this: GPtr<u8>) = thiscall 0x4EB890 => radio_check_for_station_retune_during_pause;
    /// `CAudioEngine::IsAmbienceTrackActive()` (0x507210)
    fn engine_is_ambience_track_active(engine: GPtr<u8>) -> u8 = thiscall 0x507210 => unported;
    /// `CAERadioTrackManager::IsVehicleRadioActive()` (0x4E9800; called with ECX = this)
    fn radio_is_vehicle_radio_active(this: GPtr<u8>) -> u8 = thiscall 0x4E9800 => unported;
    /// `CAERadioTrackManager::StartTrackPlayback()` (0x4EA640)
    fn radio_start_track_playback(this: GPtr<u8>) = thiscall 0x4EA640 => unported;
    /// `CAERadioTrackManager::ChooseAdvertIndex(eRadioID)` (0x4E9570)
    fn radio_choose_advert_index_call(this: GPtr<u8>, station: u32) -> u32 = thiscall 0x4E9570 => radio_choose_advert_index;
    /// `CAERadioTrackManager::AddIdentIndexToHistory(eRadioID, int32)` (0x4E9720)
    fn radio_add_ident_call(this: GPtr<u8>, station: u32, index: u32) = thiscall 0x4E9720 => radio_add_ident_index_to_history;
    /// `CAERadioTrackManager::AddAdvertIndexToHistory(eRadioID, int32)` (0x4E9760)
    fn radio_add_advert_call(this: GPtr<u8>, station: u32, index: u32) = thiscall 0x4E9760 => radio_add_advert_index_to_history;
    /// `CAERadioTrackManager::AddDJBanterIndexToHistory(eRadioID, int32)` (0x4E97B0)
    fn radio_add_dj_banter_call(this: GPtr<u8>, station: u32, index: u32) = thiscall 0x4E97B0 => radio_add_dj_banter_index_to_history;
    /// `CAEUserRadioTrackManager::GetUserTrackPlayMode()` (0x4F3330)
    fn user_get_user_track_play_mode(this: GPtr<u8>) -> u8 = thiscall 0x4F3330 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(id: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `FUN_004e7f20` (0x4E7F20, a `CPad` query used for the user-track skip)
    fn pad_query_4e7f20(pad: GPtr<u8>) -> u8 = thiscall 0x4E7F20 => unported;
    /// `CAERadioTrackManager::Service(int32 playTime)` (0x4EB9A0; the bytes' recursion)
    fn radio_service_call(this: GPtr<u8>, t: u32) = thiscall 0x4EB9A0 => radio_service_port;
    /// `CAERadioTrackManager::StartRadio(eRadioID, eBassSetting, float, uint8)` (0x4EB3C0)
    fn radio_start_radio_call(this: GPtr<u8>, station: u32, bass: u32, gain_bits: u32, b: u32) = thiscall 0x4EB3C0 => crate::radio::start_radio_manual_port;
    /// `CAERadioTrackManager::CheckForPause()` (0x4EA590)
    fn radio_check_for_pause(this: GPtr<u8>) = thiscall 0x4EA590 => unported;
    /// `CAERadioTrackManager::UpdateRadioVolumes()` (0x4EA010)
    fn radio_update_radio_volumes(this: GPtr<u8>) = thiscall 0x4EA010 => unported;
    /// `CAERadioTrackManager::CheckForTrackConcatenation()` (0x4EA930)
    fn radio_check_for_track_concatenation(this: GPtr<u8>) = thiscall 0x4EA930 => unported;
}

/// `CAERadioTrackManager::Service(int32 playTime)` (0x4EB9A0). Stores the play time (+0xE0; the previous
/// one at 0x8CBA68), track length (+0xE4) and playing track (+0xD8); runs the mission-stat and retune
/// checks (or the pause variant); handles a pending start (+0 set in mode 7: for station 13 the player
/// vehicle's radio type, else unless an ambience track plays: mode 0 and `IsVehicleRadioActive`); then by
/// mode +0x68 (jump table 0x4EBF10): 0 starts the queued track (bass, `PlayTrack`), mode 1; 1 waits for
/// the start (-2: event 0x23, `StartTrackPlayback`, time stamps, mode 2) or a failure (-8: the user-track
/// station refills its queue with an advert and a user track, mode 0); 2 plays: user-track rotation for
/// station 12 at -6, history adds by track type for a newly playing current/next track, the user-track
/// skip (pad query 0x4E7F20: `StopRadio`, recursive `Service` until off, `StartRadio`) and every frame
/// `CheckForPause`, `UpdateRadioVolumes`, `CheckForTrackConcatenation` and event 0x23; 3 and 5 stop the
/// track (mode 6); 4 goes to 5; 6 waits for the stop (-6 / -8: mode 7; -7 / -2: `StopTrack` again).
#[hook(addr = 0x4EB9A0, conv = thiscall, name = "CAERadioTrackManager::Service", expect = "55 8B E9 8B 85 E0 00 00 00")]
pub fn radio_service_port(this: GPtr<u8>, play_time: u32) {
    let hw = g::<u8>(AE_AUDIO_HARDWARE);
    let engine = g::<u8>(crate::common::AUDIO_ENGINE);
    let old = rd::<u32>(this, 0xE0);
    wr::<u32>(this, 0xE0, play_time);
    g::<u32>(0x8CBA68).write(old);
    wr::<u32>(this, 0xE4, hw_get_track_length_ms(hw));
    wr::<u32>(this, 0xD8, hw_get_playing_track_id(hw));
    if !game_paused() {
        radio_check_for_mission_stats_changes_call(this);
        radio_check_for_station_retune(this);
    } else {
        radio_check_for_station_retune_during_pause_call(this);
    }
    let copy_settings = || {
        for i in 0..15isize {
            wr::<u32>(this, 0xC4 + i * 4, rd::<u32>(this, 0x88 + i * 4));
        }
    };
    if rd::<u8>(this, 0) != 0 && rd::<u32>(this, 0x68) == 7 {
        if rd::<u8>(this, 0xAD) == 13 {
            copy_settings();
            let s = player_vehicle_audio_settings_for_radio();
            if !s.is_null() {
                let t = rd::<u8>(s, 0x1B);
                if t == 0 || t == 3 || t == 2 {
                    wr::<u8>(this, 1, 1);
                }
            }
            wr::<u8>(this, 0, 0);
        } else if engine_is_ambience_track_active(engine) == 0 {
            copy_settings();
            wr::<u32>(this, 0x68, 0);
            if radio_is_vehicle_radio_active(this) != 0 {
                wr::<u8>(this, 1, 1);
            }
            wr::<u8>(this, 0, 0);
        }
    }
    let one = 0x3F80_0000;
    match rd::<u32>(this, 0x68) {
        0 => {
            if rd::<i32>(this, 0xE0) < 0 {
                wr::<u32>(this, 0xE0, 0);
            }
            let bass = if rd::<u8>(this, 4) != 0 {
                u32::from(rd::<u8>(this, 0xEA))
            } else {
                0
            };
            hw_set_bass_setting(hw, bass, rd::<u32>(this, 0xEC));
            let user_next = u32::from(rd::<u8>(this, 0xF1) == 7);
            let user = u32::from(rd::<u8>(this, 0xF0) == 7);
            let flags = u32::from(rd::<u8>(this, 0xE8));
            hw_play_track(
                hw,
                rd::<u32>(this, 0xC4),
                rd::<u32>(this, 0xC8),
                rd::<u32>(this, 0xE0),
                flags,
                user,
                user_next,
            );
            wr::<u32>(this, 0x68, 1);
        }
        1 => {
            let e0 = rd::<i32>(this, 0xE0);
            if e0 == -2 {
                engine_report_frontend_audio_event(engine, 0x23, 0, one);
                radio_start_track_playback(this);
                let idx = isize::from(rd::<i8>(this, 0xE9));
                wr::<u32>(this, 0x60, g::<u32>(TIMER_MS).read());
                wr::<u32>(this, idx * 0x2C + 0x10C, g::<u32>(TIMER_MS_PAUSE_MODE).read());
                wr::<u32>(this, 0x68, 2);
            } else if e0 == -8 {
                let d8 = rd::<u32>(this, 0xD8);
                let c8 = rd::<u32>(this, 0xC8);
                if d8 == c8 || (d8 == rd::<u32>(this, 0xC4) && c8 == 0xFFFF_FFFF) {
                    let a = radio_choose_advert_index_call(this, 12);
                    wr::<u32>(this, 0xC4, a);
                    wr::<u8>(this, 0xF0, 1);
                    let u = user_select_user_track_index_call(g(0xB6B970));
                    wr::<u32>(this, 0xC8, u);
                    wr::<u8>(this, 0xF1, 7);
                    wr::<u8>(this, 0xF8, rd::<u8>(this, 0xC8));
                    wr::<u32>(this, 0x68, 0);
                }
            }
        }
        2 => {
            if rd::<u8>(this, 0xE9) == 12 && rd::<i32>(this, 0xE0) == -6 {
                let c8 = rd::<u32>(this, 0xC8);
                if hw_get_active_track_id(hw) == rd::<u32>(this, 0xC4) && c8 != 0xFFFF_FFFF {
                    let cl = rd::<u8>(this, 0xF8);
                    wr::<u32>(this, 0xC4, c8);
                    let al = rd::<u8>(this, 0xF1);
                    wr::<u8>(this, 0xF7, cl);
                    wr::<u8>(this, 0xF0, al);
                } else {
                    let u = user_select_user_track_index_call(g(0xB6B970));
                    wr::<u32>(this, 0xC4, u);
                    wr::<u8>(this, 0xF0, 7);
                    wr::<u8>(this, 0xF7, rd::<u8>(this, 0xC4));
                }
                let u = user_select_user_track_index_call(g(0xB6B970));
                wr::<u32>(this, 0xC8, u);
                wr::<u8>(this, 0xF1, 7);
                wr::<u8>(this, 0xF8, rd::<u8>(this, 0xC8));
                wr::<u32>(this, 0x68, 0);
            }
            let st = || u32::from(rd::<u8>(this, 0xE9));
            let add_by_type = |ty: u8, id: u32, music: u8| match ty {
                3 | 4 | 5 | 7 => radio_add_music_track_index_to_history(this, st(), u32::from(music)),
                0 => radio_add_ident_call(this, st(), id),
                1 => radio_add_advert_call(this, st(), id),
                2 => radio_add_dj_banter_call(this, st(), id),
                _ => {}
            };
            let c4 = rd::<u32>(this, 0xC4);
            let d8 = rd::<u32>(this, 0xD8);
            if c4 == d8 {
                add_by_type(rd::<u8>(this, 0xF0), c4, rd::<u8>(this, 0xF7));
                wr::<u8>(this, 0xF5, rd::<u8>(this, 0xF0));
                wr::<u8>(this, 0xFC, rd::<u8>(this, 0xF7));
            } else if rd::<u32>(this, 0xDC) == d8 {
                add_by_type(rd::<u8>(this, 0xF6), d8, rd::<u8>(this, 0xFD));
                wr::<u8>(this, 0xF5, rd::<u8>(this, 0xF6));
                wr::<u8>(this, 0xFC, rd::<u8>(this, 0xFD));
            }
            if rd::<u8>(this, 0xE9) == 12
                && (rd::<u8>(this, 0xF0) == 7 || user_get_user_track_play_mode(g(0xB6B970)) == 0)
                && pad_query_4e7f20(pad_get_pad(0)) != 0
            {
                radio_stop_radio_call(this, GPtr::from_addr(0), 1);
                while !(rd::<u32>(this, 0x68) == 7
                    && rd::<u8>(this, 0) == 0
                    && rd::<u32>(this, 0x6C) == 0
                    && rd::<u32>(this, 0x70) == 0)
                {
                    let t = hw_get_track_play_time(hw);
                    radio_service_call(this, t);
                    hw_service(hw);
                    sa_core::orig::busy_wait();
                }
                let gain = rd::<u32>(this, 0xEC);
                let bass = u32::from(rd::<u8>(this, 0xEA));
                radio_start_radio_call(this, st(), bass, gain, 1);
            }
            radio_check_for_pause(this);
            radio_update_radio_volumes(this);
            radio_check_for_track_concatenation(this);
            engine_report_frontend_audio_event(engine, 0x23, 0, one);
        }
        3 | 5 => {
            hw_stop_track(hw);
            wr::<u32>(this, 0x68, 6);
        }
        4 => wr::<u32>(this, 0x68, 5),
        6 => match rd::<i32>(this, 0xE0) {
            -6 | -8 => wr::<u32>(this, 0x68, 7),
            -7 | -2 => hw_stop_track(hw),
            _ => {}
        },
        _ => {}
    }
}

orig! {
    /// `CAEScriptAudioEntity::GetMissionAudioLoadingStatus(uint8 slot)` (0x4EBF60)
    fn script_get_mission_audio_loading_status_call(this: GPtr<u8>, slot: u8) -> u8 = thiscall 0x4EBF60 => script_get_mission_audio_loading_status;
}

/// `CAEScriptAudioEntity::PlayLoadedMissionAudio(uint8 slot)` (0x4EC270): for a slot < 4 with a bank
/// (+0xA8) and sound (+0xAC) that report loaded (status 1): a stack `CAESound` (entity pointer +8
/// pre-cleared) is initialised in bank slot `slot + 26` with the slot's sound, at the attached entity's
/// position, else at the record's position (+0x90) or frontend-relative (0, 1, 0) when that is unset
/// (-1000) or (0, 0, 0) (`FUN_004dc670`); volume -100 for event 0xFFFF, else the int8 at `[0xBD00F8] +
/// event` (slots 0..1: frontend flags 0x100 / 0x200 / 0x800 set, and a volume equal to `[0x85F3C4]` becomes
/// 6.0); max distance 2.0, speed 1.0; flags word +0x56 = 0xE, `SetFlags(1, relative)`, the three frontend
/// flags; `RequestNewSound` stored at the record's +0xA0; then `~CAESound`. The sound's bytes not written
/// by `Initialise` are zero here and stack leftovers in the original (copied by `RequestNewSound`).
#[hook(addr = 0x4EC270, conv = thiscall, name = "CAEScriptAudioEntity::PlayLoadedMissionAudio", expect = "6A FF 68 B8 C0 83 00", peek = 0x80)]
pub fn script_play_loaded_mission_audio(this: GPtr<u8>, slot: u32) {
    // the original's `CAESound` local (entry ESP - 0x80) is uninitialised: its stack residue (`peek`)
    let mut buf = crate::port3_lead::stack_local::<0x1D>(0x4EC270, -0x80);
    let sound = GPtr::from_raw(buf.as_mut_ptr().cast::<u8>());
    wr::<u32>(sound, 8, 0);
    let sl = slot as u8;
    let play = sl < 4 && {
        let rec = this.byte_add(isize::from(sl) * 32);
        rd::<i32>(rec, 0xA8) >= 0
            && rd::<i32>(rec, 0xAC) >= 0
            && script_get_mission_audio_loading_status_call(this, sl) == 1
    };
    if play {
        let rec = this.byte_add(isize::from(sl) * 32);
        let ev = rd::<u32>(rec, 0xA4);
        let mut frontend = 0u32;
        let vol = if ev == 0xFFFF {
            0xC2C8_0000
        } else {
            let v = crate::common::xi(i32::from(rd::<i8>(
                g::<u8>(g::<u32>(0xBD00F8).read() as usize),
                ev as isize,
            )))
            .to_f32();
            if sl < 2 {
                frontend = 1;
                if x(v).fcom(x(g::<f32>(0x85F3C4).read())).test_parity(0x44) {
                    v.to_bits()
                } else {
                    0x40C0_0000
                }
            } else {
                v.to_bits()
            }
        };
        let mut relative = 0u32;
        let (px, py, pz);
        let ent = rd::<u32>(rec, 0x9C);
        if ent != 0 {
            let e = g::<u8>(ent as usize);
            let m = rd::<u32>(e, 0x14);
            let p = if m != 0 {
                g::<u8>(m as usize + 0x30)
            } else {
                e.byte_add(4)
            };
            (px, py, pz) = (rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8));
        } else {
            let pos = rec.byte_add(0x90);
            let mut v = [0u32; 3];
            let vp = GPtr::from_raw(v.as_mut_ptr().cast::<u8>());
            let mut is = |vals: [u32; 3]| {
                v = vals;
                vector_equal_call(pos, vp) as u8 != 0
            };
            if is([0xC47A_0000; 3]) || is([0; 3]) {
                (px, py, pz) = (0, 0x3F80_0000, 0);
                relative = 1;
            } else {
                (px, py, pz) = (rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8));
            }
        }
        let sfx = u32::from(rd::<u16>(rec, 0xAC));
        sound_initialise(
            sound,
            u32::from(sl) + 0x1A,
            sfx,
            this,
            px,
            py,
            pz,
            vol,
            0x4000_0000,
            0x3F80_0000,
            0x3F80_0000,
            0,
            0,
            0,
            0,
        );
        wr::<u16>(sound, 0x56, 0xE);
        sound_set_flags(sound, 1, relative);
        sound_set_flags(sound, 0x100, frontend);
        sound_set_flags(sound, 0x200, frontend);
        sound_set_flags(sound, 0x800, frontend);
        let got = sm_request_new_sound(g(crate::common::AE_SOUND_MANAGER), sound);
        wr::<u32>(rec, 0xA0, got);
    }
    sound_dtor(sound);
}

/// `CAEAudioHardware::GetNumAvailableChannels()` (0x4D8810): the word +0x8C (AX only; the upper half of
/// EAX is the caller's).
/// AX-only result (the upper EAX is the caller's leftover on some paths): hooked by PORT-AUDIO-3 through an
/// `sa_core::abi::Ax` wrapper in `port3_lead.rs`.
pub fn hw_get_num_available_channels(this: GPtr<u8>) -> u16 {
    rd::<u16>(this, 0x8C)
}

orig! {
    /// `CAEMP3BankLoader::IsSoundBankLoaded(uint16 bank, int16 slot)` (0x4E0220)
    fn mp3_is_sound_bank_loaded_call(this: GPtr<u8>, bank: u32, slot: u32) -> u8 = thiscall 0x4E0220 => mp3_is_sound_bank_loaded;
    /// `CAEMP3BankLoader::IsSoundLoaded(uint16 bank, uint16 sound, int16 slot)` (0x4E03B0)
    fn mp3_is_sound_loaded_call(this: GPtr<u8>, bank: u32, sound: u32, slot: u32) -> u8 = thiscall 0x4E03B0 => mp3_is_sound_loaded;
    /// `CAEBankLoader::GetBankLookup(uint16 bank)` (0x4E01B0)
    fn bank_get_bank_lookup_call(this: GPtr<u8>, bank: u32) -> u32 = thiscall 0x4E01B0 => bank_get_bank_lookup;
}

/// Appends a load request at the queue tail (+0x668, 50 records of 0x20 bytes from +0x24): bank (+0x3C),
/// slot (+0x3E), sound (+0x40), slot buffer (+0x24 = `*this + slot * 0x12D4`), the bank lookup's pak byte
/// (+0x42), offset (+0x28) and, for whole banks, size (+0x2C), status 1 (+0x38); the tail index is re-read
/// before every store as the bytes do; then count +0x666 += 1 and tail = (tail + 1) % 50.
fn mp3_queue_request(this: GPtr<u8>, bank: u32, slot: u32, sound: u16, lookup: u32, with_size: bool) {
    let q = || isize::from(rd::<i16>(this, 0x668)) * 32;
    let lk = g::<u8>(lookup as usize);
    wr::<u16>(this, q() + 0x3C, bank as u16);
    wr::<u16>(this, q() + 0x3E, slot as u16);
    wr::<u16>(this, q() + 0x40, sound);
    let buf = rd::<u32>(this, 0).wrapping_add(i32::from(slot as i16).wrapping_mul(0x12D4) as u32);
    wr::<u32>(this, q() + 0x24, buf);
    wr::<u8>(this, q() + 0x42, rd::<u8>(lk, 0));
    wr::<u32>(this, q() + 0x28, rd::<u32>(lk, 4));
    if with_size {
        wr::<u32>(this, q() + 0x2C, rd::<u32>(lk, 8));
    }
    wr::<u32>(this, q() + 0x38, 1);
    let tail = i32::from(rd::<i16>(this, 0x668));
    wr::<u16>(this, 0x666, rd::<u16>(this, 0x666).wrapping_add(1));
    wr::<u16>(this, 0x668, ((tail + 1) % 50) as u16);
}

/// `CAEMP3BankLoader::LoadSoundBank(uint16 bank, int16 slot)` (0x4E0670): when initialised, the bank
/// (zero-extended) is within the bank count (+0xE, int16), the slot within 0..=slot count (+0xC), the bank
/// is not already in the slot and no queued request (+0x3C bank sign-extended, +0x3E slot) matches: queues
/// a whole-bank request (sound -1, with size).
#[hook(addr = 0x4E0670, conv = thiscall, name = "CAEMP3BankLoader::LoadSoundBank", expect = "56 8B F1 8A 46 14")]
pub fn mp3_load_sound_bank(this: GPtr<u8>, bank: u32, slot: u32) {
    if rd::<u8>(this, 0x14) == 0 {
        return;
    }
    let b = i32::from(bank as u16);
    if b > i32::from(rd::<i16>(this, 0xE)) {
        return;
    }
    let s = slot as i16;
    if s < 0 || s > rd::<i16>(this, 0xC) {
        return;
    }
    if mp3_is_sound_bank_loaded_call(this, bank, slot) != 0 {
        return;
    }
    for i in 0..50isize {
        if i32::from(rd::<i16>(this, 0x3C + i * 32)) == b && rd::<i16>(this, 0x3E + i * 32) == s {
            return;
        }
    }
    let lookup = bank_get_bank_lookup_call(this, bank);
    mp3_queue_request(this, bank, slot, 0xFFFF, lookup, true);
}

/// `CAEMP3BankLoader::LoadSound(uint16 bank, uint16 sound, int16 slot)` (0x4E07A0): as `LoadSoundBank`
/// for a single sound (< 400, unsigned): skipped when already loaded or queued (request sound and bank
/// sign-extended against the zero-extended arguments, slot words equal); the request carries the sound and
/// no size. The duplicate scan reuses the original's first argument slot (not modelled).
#[hook(addr = 0x4E07A0, conv = thiscall, name = "CAEMP3BankLoader::LoadSound", expect = "56 8B F1 8A 46 14")]
pub fn mp3_load_sound(this: GPtr<u8>, bank: u32, sound: u32, slot: u32) {
    if rd::<u8>(this, 0x14) == 0 {
        return;
    }
    if sound as u16 >= 0x190 {
        return;
    }
    let b = i32::from(bank as u16);
    if b > i32::from(rd::<i16>(this, 0xE)) {
        return;
    }
    let s = slot as i16;
    if s < 0 || s > rd::<i16>(this, 0xC) {
        return;
    }
    if mp3_is_sound_loaded_call(this, bank, sound, slot) != 0 {
        return;
    }
    let snd = i32::from(sound as u16);
    for i in 0..50isize {
        let r = 0x3C + i * 32;
        if i32::from(rd::<i16>(this, r + 4)) == snd
            && i32::from(rd::<i16>(this, r)) == b
            && rd::<i16>(this, r + 2) == s
        {
            return;
        }
    }
    let lookup = bank_get_bank_lookup_call(this, bank);
    mp3_queue_request(this, bank, slot, sound as u16, lookup, false);
}

/// `CAEMP3BankLoader::GetSoundBuffer(uint16 sound, int16 slot, uint32* size, uint16* sampleRate)` (0x4E0280):
/// null unless initialised, the slot holds a bank (+0x10 != -1) and the sound is loaded (below the sound
/// count +0x12, or equal to the single loaded sound `this[+0x66C + 2 * slot]` when the count is -1). The
/// size is the next entry's offset (index `(sound + 1) % 400`) minus this one's (+0x14 of the 12-byte
/// entry), or for the last sound (and for a sound >= 400 not below count - 1) the bank size from the lookup (`[this+4] +
/// 12 * bank + 8`) minus the offset; the sample rate is the entry's word +0x1C; the result is the entry
/// offset + the slot's buffer (+0) + the loader's base (+0x1C).
#[hook(addr = 0x4E0280, conv = thiscall, name = "CAEMP3BankLoader::GetSoundBuffer", expect = "8A 41 14 84 C0")]
pub fn mp3_get_sound_buffer(this: GPtr<u8>, sound: u32, slot: u32, size: GPtr<u32>, rate: GPtr<u16>) -> u32 {
    if rd::<u8>(this, 0x14) == 0 {
        return 0;
    }
    let base = rd::<GPtr<u8>>(this, 0);
    let rec_off = i32::from(slot as i16).wrapping_mul(0x12D4) as isize;
    let rec = base.byte_add(rec_off);
    if rd::<u16>(rec, 0x10) == 0xFFFF {
        return 0;
    }
    let count = rd::<i16>(rec, 0x12);
    let s = sound as u16;
    let si = i32::from(s);
    if count != -1 {
        if i32::from(count) <= si {
            return 0;
        }
    } else if i32::from(rd::<i16>(this, 0x66C + isize::from(slot as i16) * 2)) != si {
        return 0;
    }
    let entry = rec_off + si as isize * 12;
    let use_next = if s < 0x190 && count == -1 {
        true
    } else {
        si < i32::from(count) - 1
    };
    if use_next {
        let next = (si + 1) % 400;
        let v =
            rd::<u32>(base, rec_off + next as isize * 12 + 0x14).wrapping_sub(rd::<u32>(base, entry + 0x14));
        size.write(v);
    } else {
        let bank = i32::from(rd::<i16>(rec, 0x10));
        let lookups = rd::<GPtr<u8>>(this, 4);
        let v = rd::<u32>(lookups, (bank * 12 + 8) as isize).wrapping_sub(rd::<u32>(base, entry + 0x14));
        size.write(v);
    }
    let base2 = rd::<GPtr<u8>>(this, 0);
    rate.write(rd::<u16>(base2, entry + 0x1C));
    let base3 = rd::<GPtr<u8>>(this, 0);
    rd::<u32>(base3, entry + 0x14)
        .wrapping_add(rd::<u32>(base3, rec_off))
        .wrapping_add(rd::<u32>(this, 0x1C))
}

orig! {
    /// `CAEAudioUtility::ConvertFromBytesToMS(uint32 bytes, uint32 freq, uint16 channels)` (0x4D9EF0)
    fn util_convert_from_bytes_to_ms(bytes: u32, freq: u32, channels: u16) -> u32 = cdecl 0x4D9EF0 => crate::utility::convert_from_bytes_to_ms;
}

/// `CAEMP3BankLoader::UpdateVirtualChannels(tVirtualChannelSettings* settings, int16* lengths, int16*
/// loopTimes)` (0x4E0450): for the 300 virtual channels (bank slot words at `settings`, sound ids 600
/// bytes further): -1 / -1 for an empty channel, an empty slot (+0x10 == -1) or a sound outside a counted
/// bank (sound >= count +0x12 with count >= 0); else the length in ms (`ConvertFromBytesToMS(next offset -
/// offset, rate, 1)`, the bank size from the lookup for the last sound `count - 1`) and the loop time
/// (-1 for loop offset +0x18 == -1, else `ConvertFromBytesToMS(2 * loop, rate, 1)`) as words. The second
/// entry read re-reads the channel's slot and sound.
#[hook(addr = 0x4E0450, conv = thiscall, name = "CAEMP3BankLoader::UpdateVirtualChannels", expect = "8B 44 24 08 53")]
pub fn mp3_update_virtual_channels(this: GPtr<u8>, settings: GPtr<u8>, lengths: GPtr<u8>, loops: GPtr<u8>) {
    for i in 0..300isize {
        let a = rd::<i16>(settings, i * 2);
        let b = rd::<i16>(settings, 0x258 + i * 2);
        let mut out = None;
        if a >= 0 && b >= 0 {
            let base = rd::<GPtr<u8>>(this, 0);
            let rec_off = i32::from(a).wrapping_mul(0x12D4) as isize;
            let bank = rd::<u16>(base, rec_off + 0x10);
            let count = rd::<i16>(base, rec_off + 0x12);
            if bank != 0xFFFF && (b < count || count < 0) {
                let entry = base.byte_add(rec_off + isize::from(b) * 12);
                let end = if i32::from(b) == i32::from(count) - 1 {
                    let lookups = rd::<GPtr<u8>>(this, 4);
                    rd::<u32>(lookups, isize::from(bank as i16) * 12 + 8)
                } else {
                    rd::<u32>(entry, 0x20)
                };
                let bytes = end.wrapping_sub(rd::<u32>(entry, 0x14));
                let len = util_convert_from_bytes_to_ms(bytes, u32::from(rd::<u16>(entry, 0x1C)), 1);
                let b2 = isize::from(rd::<i16>(settings, 0x258 + i * 2));
                let a2 = i32::from(rd::<i16>(settings, i * 2));
                let base2 = rd::<GPtr<u8>>(this, 0);
                let entry2 = base2.byte_add(a2.wrapping_mul(0x12D4) as isize + b2 * 12);
                let lp = rd::<u32>(entry2, 0x18);
                let loop_ms = if lp == 0xFFFF_FFFF {
                    0xFFFF_FFFF
                } else {
                    util_convert_from_bytes_to_ms(lp.wrapping_add(lp), u32::from(rd::<u16>(entry2, 0x1C)), 1)
                };
                out = Some((len as u16, loop_ms as u16));
            }
        }
        let (l, p) = out.unwrap_or((0xFFFF, 0xFFFF));
        wr::<u16>(lengths, i * 2, l);
        wr::<u16>(loops, i * 2, p);
    }
}

/// `CAEBankLoader::CalculateBankSlotsInfosOffsets()` (0x4DFBA0): running offsets of the bank slots: slot i
/// (0x12D4 bytes each at `*this`) gets the sum of the sizes (+4) of the slots before it at +0; the slot
/// count (+0xC, int16) and `*this` are re-read every iteration.
#[hook(addr = 0x4DFBA0, conv = thiscall, name = "CAEBankLoader::CalculateBankSlotsInfosOffsets", expect = "56 57 33 FF 33 F6")]
pub fn bank_calculate_bank_slots_infos_offsets(this: GPtr<u8>) {
    let mut off: u32 = 0;
    let mut i: i32 = 0;
    while i < i32::from(rd::<i16>(this, 0xC)) {
        let rec = (i * 0x12D4) as isize;
        wr::<u32>(rd::<GPtr<u8>>(this, 0), rec, off);
        off = off.wrapping_add(rd::<u32>(rd::<GPtr<u8>>(this, 0), rec + 4));
        i += 1;
    }
}

/// `FUN_004dfb60` (0x4DFB60, `CAEMP3BankLoader` request-queue reset): the 50 requests get bank / slot /
/// sound words -1 and their +0 / +0x14 dwords 0 (records at +0x24); then +0x664, +0x666, +0x668 = 0.
/// Not hooked: its only caller (`CAEMP3BankLoader::Initialise`, 0x4E08F0) calls `LoadBankSlotFile` next
/// without reloading ECX, relying on this function leaving ECX = `this`.
pub fn mp3_reset_requests(this: GPtr<u8>) {
    for i in 0..50isize {
        let r = 0x3E + i * 32;
        wr::<u16>(this, r - 2, 0xFFFF);
        wr::<u16>(this, r, 0xFFFF);
        wr::<u16>(this, r + 2, 0xFFFF);
        wr::<u32>(this, r - 0x1A, 0);
        wr::<u32>(this, r - 6, 0);
    }
    wr::<u16>(this, 0x664, 0);
    wr::<u16>(this, 0x666, 0);
    wr::<u16>(this, 0x668, 0);
}

/// `CAESoundManager::CancelSoundsOfThisEventPlayingForThisEntityAndPhysical(int16 event, CAEAudioEntity*,
/// CPhysical*)` (0x4EFBF0): every sound in use (word +0x50 of the 0x74-byte records from +0xC) with this
/// event (+4, sign-extended), entity (-4) and physical (+0) has flag 4 of byte +0x4E cleared, the stop
/// request +0x60 = 1 and its entity cleared; a physical is released (`CleanUpOldReference`) and cleared.
#[hook(addr = 0x4EFBF0, conv = thiscall, name = "CAESoundManager::CancelSoundsOfThisEventPlayingForThisEntityAndPhysical", expect = "51 56 8D 71 0C")]
pub fn sm_cancel_sounds_of_this_event_playing_for_this_entity_and_physical(
    this: GPtr<u8>,
    event: u32,
    entity: u32,
    physical: u32,
) {
    let ev = i32::from(event as i16) as u32;
    let mut s = this.byte_add(0xC);
    for _ in 0..300 {
        if rd::<u16>(s, 0x50) != 0
            && rd::<u32>(s, 4) == ev
            && rd::<u32>(s, -4) == entity
            && rd::<u32>(s, 0) == physical
        {
            let p = rd::<GPtr<u8>>(s, 0);
            wr::<u8>(s, 0x4E, rd::<u8>(s, 0x4E) & 0xFB);
            wr::<u16>(s, 0x60, 1);
            wr::<u32>(s, -4, 0);
            if !p.is_null() {
                entity_clean_up_old_reference(p, s);
                wr::<u32>(s, 0, 0);
            }
        }
        s = s.byte_add(0x74);
    }
}

orig! {
    /// CRT `operator delete` / `_free` (0x8214BD)
    fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `CAEGlobalWeaponAudioEntity::Destructor` (0x506C50)
    fn global_weapon_dtor(this: GPtr<u8>) = thiscall 0x506C50 => unported;
    /// `CAEBankLoader::Deconstructor` (0x4DFB20)
    fn bank_loader_dtor(this: GPtr<u8>) = thiscall 0x4DFB20 => unported;
    /// `CAEMP3TrackLoader::Deconstructor` (0x4E0940)
    fn track_loader_dtor(this: GPtr<u8>) = thiscall 0x4E0940 => unported;
}

/// `FUN_00506dc0` (0x506DC0): the scalar deleting destructor of `CAEGlobalWeaponAudioEntity`: destructor
/// 0x506C50, `operator delete(this)` when bit 0 of the flags is set; returns `this`.
#[hook(addr = 0x506DC0, conv = thiscall, name = "FUN_00506dc0", expect = "56 8B F1 E8 88 FE FF FF")]
pub fn global_weapon_deleting_dtor(this: GPtr<u8>, flags: u32) -> u32 {
    global_weapon_dtor(this);
    if flags as u8 & 1 != 0 {
        crt_free(this);
    }
    this.addr() as u32
}

/// `FUN_004d8670` (0x4D8670): the scalar deleting destructor of `CAEMP3BankLoader` (destructor 0x4DFB20).
#[hook(addr = 0x4D8670, conv = thiscall, name = "FUN_004d8670", expect = "56 8B F1 E8 A8 74 00 00")]
pub fn bank_loader_deleting_dtor(this: GPtr<u8>, flags: u32) -> u32 {
    bank_loader_dtor(this);
    if flags as u8 & 1 != 0 {
        crt_free(this);
    }
    this.addr() as u32
}

/// `FUN_004d8690` (0x4D8690): the scalar deleting destructor of `CAEMP3TrackLoader` (destructor 0x4E0940).
#[hook(addr = 0x4D8690, conv = thiscall, name = "FUN_004d8690", expect = "56 8B F1 E8 A8 82 00 00")]
pub fn track_loader_deleting_dtor(this: GPtr<u8>, flags: u32) -> u32 {
    track_loader_dtor(this);
    if flags as u8 & 1 != 0 {
        crt_free(this);
    }
    this.addr() as u32
}

/// `CAEAudioHardware::AllocateChannels(uint16 num)` (0x5B9340): for 1 <= `num` <= the free channel count
/// (+0x8C): the first channel slot (words at +0x90, walked by their sizes below 64) with size 0 gets
/// `num`, the free count drops by `num` and the slot's first channel index is returned; else -1. AX only
/// (the upper half of EAX is the caller's or the walk index's).
#[hook(addr = 0x5B9340, conv = thiscall, name = "CAEAudioHardware::AllocateChannels", expect = "56 66 8B 74 24 08")]
pub fn hw_allocate_channels(this: GPtr<u8>, num: u32) -> u16 {
    let n = num as u16;
    if n > rd::<u16>(this, 0x8C) || n < 1 {
        return 0xFFFF;
    }
    let mut i: i32 = 0;
    loop {
        let size = i32::from(rd::<u16>(this, 0x90 + i as isize * 2));
        if size <= 0 {
            wr::<u16>(this, 0x90 + i as isize * 2, n);
            wr::<u16>(this, 0x8C, rd::<u16>(this, 0x8C).wrapping_sub(n));
            return i as u16;
        }
        i += size;
        if i >= 0x40 {
            return 0xFFFF;
        }
    }
}

orig! {
    /// CRT `eh vector destructor iterator(void* arr, uint32 size, int32 count, void (*dtor)(void*))` (0x821E02, stdcall)
    fn eh_vector_dtor(arr: u32, size: u32, count: u32, dtor: u32) = stdcall 0x821E02 => unported;
}

/// `CAESoundManager::Terminate()` (0x4EFAA0): destroys the virtual-channel array (+0x87F4: `eh vector
/// destructor iterator(arr, 2, count at arr - 4, 0x4EF070)`, then frees the block at arr - 4), frees +0x87F8
/// and +0x87FC and clears the three pointers.
#[hook(addr = 0x4EFAA0, conv = thiscall, name = "CAESoundManager::Terminate", expect = "56 8B F1 8B 86 F4 87 00 00")]
pub fn sm_terminate(this: GPtr<u8>) {
    let arr = rd::<u32>(this, 0x87F4);
    if arr != 0 {
        let block = arr.wrapping_sub(4);
        let count = g::<u32>(block as usize).read();
        eh_vector_dtor(arr, 2, count, 0x4EF070);
        crt_free(GPtr::from_addr(block as usize));
    }
    crt_free(rd::<GPtr<u8>>(this, 0x87F8));
    crt_free(rd::<GPtr<u8>>(this, 0x87FC));
    wr::<u32>(this, 0x87F4, 0);
    wr::<u32>(this, 0x87F8, 0);
    wr::<u32>(this, 0x87FC, 0);
}

orig! {
    /// `CAEStreamThread::CAEStreamThread()` (0x4F11D0)
    fn stream_thread_ctor(this: GPtr<u8>) = thiscall 0x4F11D0 => unported;
}

/// `CAEAudioHardware::Constructor` (0x4D83E0): constructs the stream thread (+0xE14), then for the 64
/// channels: slot size word +0x90 = 0, volume +0x110 = -1000.0, flags word +6 = 0 (per channel, in this
/// order); for the 300 virtual channels: words +0x430 and +0x688 = -1; the six scaling factors
/// +0x410..+0x424 = 1.0; free channel count +0x8C = 0, +0xDA8 / +1 / +0 = 0, +5 = 0xFF, +0x88 = -10000.
/// Returns `this` (EDX = -1 is not modelled; the static initialiser ignores it).
#[hook(addr = 0x4D83E0, conv = thiscall, name = "CAEAudioHardware::Constructor", expect = "53 56 8B F1 8D 8E 14 0E 00 00")]
pub fn hw_constructor(this: GPtr<u8>) -> u32 {
    stream_thread_ctor(this.byte_add(0xE14));
    for i in 0..64isize {
        wr::<u16>(this, 0x90 + i * 2, 0);
        wr::<u32>(this, 0x110 + i * 4, 0xC47A_0000);
        wr::<u16>(this, 6 + i * 2, 0);
    }
    for i in 0..300isize {
        wr::<u16>(this, 0x430 + i * 2, 0xFFFF);
        wr::<u16>(this, 0x688 + i * 2, 0xFFFF);
    }
    for off in [0x410, 0x414, 0x418, 0x41C, 0x420, 0x424] {
        wr::<u32>(this, off, 0x3F80_0000);
    }
    wr::<u16>(this, 0x8C, 0);
    wr::<u8>(this, 0xDA8, 0);
    wr::<u8>(this, 1, 0);
    wr::<u8>(this, 0, 0);
    wr::<u8>(this, 5, 0xFF);
    wr::<u32>(this, 0x88, 0xFFFF_D8F0);
    this.addr() as u32
}

/// `FUN_004defe0` (0x4DEFE0, a `CAEFrontendAudioEntity` helper choosing a random ambient pick): with
/// `GetRandomNumberInRange(0, 1)` (0x4D9C10): 0 -> `*a = rand(4, 14)`, `*b = 30`, `*c = rand(90, 350)`;
/// 1 -> `*a = rand(2, 4)`, `*b = 22`, `*c = rand(500, 1200)` (`this` unused). Returns the original's EAX
/// (the last draw, or the first draw - 1 when it is neither 0 nor 1).
#[hook(addr = 0x4DEFE0, conv = thiscall, name = "FUN_004defe0", expect = "6A 01 6A 00 E8 27 AC FF FF")]
pub fn frontend_random_pick(_this: GPtr<u8>, a: GPtr<u32>, b: GPtr<u32>, c: GPtr<u32>) -> u32 {
    let r = crate::common::ae_random_int(0, 1);
    match r {
        0 => {
            a.write(crate::common::ae_random_int(4, 14) as u32);
            b.write(0x1E);
            let v = crate::common::ae_random_int(90, 350) as u32;
            c.write(v);
            v
        }
        1 => {
            a.write(crate::common::ae_random_int(2, 4) as u32);
            b.write(0x16);
            let v = crate::common::ae_random_int(500, 1200) as u32;
            c.write(v);
            v
        }
        _ => r.wrapping_sub(1) as u32,
    }
}

orig! {
    /// `CAEScriptAudioEntity::Constructor` (0x5074D0)
    fn script_entity_ctor(this: GPtr<u8>) = thiscall 0x5074D0 => unported;
    /// `CAEPedlessSpeechAudioEntity::Constructor` (0x4E6070)
    fn pedless_speech_ctor(this: GPtr<u8>) = thiscall 0x4E6070 => unported;
}

/// `CAudioEngine::Constructor` (0x507670): the inline-constructed frontend entity (+0xB4: vtable 0x862E54,
/// +0xC4 = 0, its sounds' entity pointers +0x160 / +0x208 = 0, words +0x1D8 / +0x280 = 0, vtables 0x85F438
/// at +0x150 / +0x1F8, +0x1F0 / +0x1F4 / +0x298 / +0x29C = 0), the script entity (+0x2A0, its
/// constructor), the collision entity (+0x4BC: vtable 0x862E64, +0x4CC = 0), the pedless speech entity
/// (+0x1E38, its constructor) and +0x1F60 / +0x1FCC / +0x1FD0 / +0x1FD4 = 0, +0x1F50 = vtable 0x85998C, in
/// the bytes' order. The original's SEH frame (unwind state) is not modelled. Returns `this`.
#[hook(addr = 0x507670, conv = thiscall, name = "CAudioEngine::Constructor", expect = "6A FF 68 5A C6 83 00")]
pub fn engine_constructor(this: GPtr<u8>) -> u32 {
    wr::<u32>(this, 0xC4, 0);
    wr::<u32>(this, 0xB4, 0x862E54);
    wr::<u32>(this, 0x160, 0);
    wr::<u16>(this, 0x1D8, 0);
    wr::<u32>(this, 0x1F0, 0);
    wr::<u32>(this, 0x1F4, 0);
    wr::<u32>(this, 0x150, 0x85F438);
    wr::<u32>(this, 0x208, 0);
    wr::<u32>(this, 0x1F8, 0x85F438);
    wr::<u16>(this, 0x280, 0);
    wr::<u32>(this, 0x298, 0);
    wr::<u32>(this, 0x29C, 0);
    script_entity_ctor(this.byte_add(0x2A0));
    wr::<u32>(this, 0x4CC, 0);
    wr::<u32>(this, 0x4BC, 0x862E64);
    pedless_speech_ctor(this.byte_add(0x1E38));
    wr::<u32>(this, 0x1F60, 0);
    wr::<u32>(this, 0x1FCC, 0);
    wr::<u32>(this, 0x1FD0, 0);
    wr::<u32>(this, 0x1FD4, 0);
    wr::<u32>(this, 0x1F50, 0x85998C);
    this.addr() as u32
}

orig! {
    /// `CAudioEngine::Service()` (0x507750, called on the `AudioEngine` singleton)
    fn engine_service(engine: GPtr<u8>) = thiscall 0x507750 => unported;
    /// `CAECollisionAudioEntity::InitialisePostLoading()` (0x4DA050)
    fn collision_initialise_post_loading(this: GPtr<u8>) = thiscall 0x4DA050 => unported;
    /// CRT `operator new(uint32)` (0x82119A)
    fn crt_operator_new(size: u32) -> u32 = cdecl 0x82119A => unported;
    /// `CAEGlobalWeaponAudioEntity::Constructor` (0x5075B0; returns `this`)
    fn global_weapon_ctor(this: u32) -> u32 = thiscall 0x5075B0 => unported;
    /// `CAEWeaponAudioEntity::Initialise()` (0x503450; called with the possibly null entity)
    fn weapon_initialise(this: u32) = thiscall 0x503450 => unported;
    /// `FUN_00506b20` (0x506B20, `CAEFrontendAudioEntity` destructor)
    fn frontend_dtor(this: GPtr<u8>) = thiscall 0x506B20 => unported;
    /// `FUN_00506ba0` (0x506BA0, `CAEScriptAudioEntity` destructor)
    fn script_entity_dtor(this: GPtr<u8>) = thiscall 0x506BA0 => unported;
}

/// The (bank, bank slot) pairs `CAudioEngine::InitialisePostLoading` waits for, in the bytes' order.
const POST_LOADING_BANKS: [(u32, u32); 12] = [
    (0x27, 2),
    (0x1B, 3),
    (0x8A, 0x13),
    (0, 0x29),
    (0x3B, 0),
    (0x34, 4),
    (0x8F, 5),
    (0x33, 0x1F),
    (0x69, 6),
    (0x4A, 0x11),
    (0x80, 0x20),
    (0x0D, 0x12),
];

/// `CAudioEngine::InitialisePostLoading()` (0x5078F0): frontend event 0x33 (volume 0.0, speed 1.0),
/// `CAudioEngine::Service` on the singleton, collision entity post-loading init, a new
/// `CAEGlobalWeaponAudioEntity` (`operator new(0xA8)` + constructor, null when the allocation fails) at
/// +0x1E34 and its `Initialise`; then services the hardware until the 12 resident banks are loaded
/// (re-checked from the first after every service). The original's SEH unwind state is not modelled.
#[hook(addr = 0x5078F0, conv = thiscall, name = "CAudioEngine::InitialisePostLoading", expect = "6A FF 68 7B C6 83 00")]
pub fn engine_initialise_post_loading(this: GPtr<u8>) {
    frontend_add_audio_event(this.byte_add(0xB4), 0x33, 0, 0x3F80_0000);
    engine_service(g(crate::common::AUDIO_ENGINE));
    collision_initialise_post_loading(this.byte_add(0x4BC));
    let mem = crt_operator_new(0xA8);
    let weapon = if mem != 0 { global_weapon_ctor(mem) } else { 0 };
    wr::<u32>(this, 0x1E34, weapon);
    weapon_initialise(weapon);
    let hw = g::<u8>(AE_AUDIO_HARDWARE);
    while !POST_LOADING_BANKS
        .iter()
        .all(|&(bank, slot)| hw_is_sound_bank_loaded(hw, bank, slot) != 0)
    {
        hw_service(hw);
        sa_core::orig::busy_wait();
    }
}

/// `CAudioEngine::Destructor` (0x506CD0): resets the vtables of the members to `CAEAudioEntity`'s
/// (0x859988) and destroys their sounds in reverse construction order: +0x1F50 (sound +0x1F58), pedless
/// speech +0x1E38 (sound +0x1E40), collision +0x4BC (sound +0x4C4), then the script (+0x2A0, 0x506BA0) and
/// frontend (+0xB4, 0x506B20) entity destructors. The SEH unwind state is not modelled.
#[hook(addr = 0x506CD0, conv = thiscall, name = "CAudioEngine::Destructor", expect = "6A FF 68 F8 C5 83 00")]
pub fn engine_destructor(this: GPtr<u8>) {
    wr::<u32>(this, 0x1F50, 0x859988);
    sound_dtor(this.byte_add(0x1F58));
    wr::<u32>(this, 0x1E38, 0x859988);
    sound_dtor(this.byte_add(0x1E40));
    wr::<u32>(this, 0x4BC, 0x859988);
    sound_dtor(this.byte_add(0x4C4));
    script_entity_dtor(this.byte_add(0x2A0));
    frontend_dtor(this.byte_add(0xB4));
}

pub const HOOKS: &[HookReg] = &[
    HOOK_ENGINE_INITIALISE_POST_LOADING,
    HOOK_ENGINE_DESTRUCTOR,
    HOOK_FRONTEND_RANDOM_PICK,
    HOOK_ENGINE_CONSTRUCTOR,
    HOOK_HW_CONSTRUCTOR,
    HOOK_HW_ALLOCATE_CHANNELS,
    HOOK_SM_TERMINATE,
    HOOK_GLOBAL_WEAPON_DELETING_DTOR,
    HOOK_BANK_LOADER_DELETING_DTOR,
    HOOK_TRACK_LOADER_DELETING_DTOR,
    HOOK_SM_CANCEL_SOUNDS_OF_THIS_EVENT_PLAYING_FOR_THIS_ENTITY_AND_PHYSICAL,
    HOOK_BANK_CALCULATE_BANK_SLOTS_INFOS_OFFSETS,
    HOOK_MP3_UPDATE_VIRTUAL_CHANNELS,
    HOOK_MP3_GET_SOUND_BUFFER,
    HOOK_MP3_LOAD_SOUND_BANK,
    HOOK_MP3_LOAD_SOUND,
    HOOK_SCRIPT_PLAY_LOADED_MISSION_AUDIO,
    HOOK_RADIO_SERVICE_PORT,
    HOOK_RADIO_RESET_PORT,
    HOOK_SCRIPT_PLAY_MISSION_BANK_SOUND_PORT,
    HOOK_SCRIPT_PLAY_RESIDENT_SOUND_EVENT_PORT,
    HOOK_PS_PLAY_LOADED_DIALOGUE_PORT,
    HOOK_PS_ADD_AUDIO_EVENT,
    HOOK_VECTOR_EQUAL,
    HOOK_PED_SPEECH_SLOT_INIT,
    HOOK_CUTSCENE_PRELOAD_CUTSCENE_TRACK_PORT,
    HOOK_SCRIPT_REPORT_MISSION_AUDIO_EVENT_PHYSICAL,
    HOOK_SOUND_GET_RELATIVE_PLAYBACK_FREQUENCY_WITH_DOPPLER,
    HOOK_AUDIO_ZONES_SWITCH_AUDIO_ZONE,
    HOOK_ENGINE_PRELOAD_BEAT_TRACK,
    HOOK_ENGINE_RESET,
    HOOK_ENGINE_RESET_SOUND_EFFECTS,
    HOOK_ENGINE_SERVICE_LOADING_TUNE,
    HOOK_ENGINE_PRELOAD_CUTSCENE_TRACK,
    HOOK_FRONTEND_UPDATE_PARAMETERS,
    HOOK_SCRIPT_UPDATE_PARAMETERS,
    HOOK_SCRIPT_PRELOAD_MISSION_AUDIO,
    HOOK_SCRIPT_SERVICE,
    HOOK_RADIO_CHOOSE_TALK_RADIO_SHOW,
    HOOK_RADIO_TRACK_RADIO_STATION,
    HOOK_RADIO_CHECK_FOR_MISSION_STATS_CHANGES,
    HOOK_PS_SERVICE,
    HOOK_PS_LOAD_SLOTS,
    HOOK_RADIO_CHOOSE_DJ_BANTER_INDEX,
    HOOK_RADIO_CHOOSE_ADVERT_INDEX,
    HOOK_RADIO_STOP_RADIO,
    HOOK_SCRIPT_GET_MISSION_AUDIO_POSITION,
    HOOK_SOUND_UNREGISTER_WITH_PHYSICAL_ENTITY,
    HOOK_SOUND_UPDATE_PLAY_TIME,
    HOOK_SOUND_CALCULATE_FREQUENCY,
    HOOK_SOUND_UPDATE_FREQUENCY,
    HOOK_SOUND_GET_RELATIVE_POSITION,
    HOOK_CUTSCENE_INITIALISE,
    HOOK_RADIO_CHECK_FOR_STATION_RETUNE_DURING_PAUSE,
    HOOK_PS_PREP_SLOTS,
    HOOK_PS_FINISHED_PLAYING_SCANNER_DIALOGUE,
    HOOK_PS_POPULATE_SCANNER_DIALOGUE_LISTS,
    HOOK_PS_RESET,
    HOOK_PS_PLAY_POLICE_SCANNER_DIALOGUE,
    HOOK_PS_UPDATE_PARAMETERS,
    HOOK_BANK_GET_BANK_LOOKUP,
    HOOK_MP3_GET_SOUND_HEADROOM,
    HOOK_MP3_IS_SOUND_BANK_LOADED,
    HOOK_MP3_GET_LOOP_OFFSET,
    HOOK_MP3_IS_SOUND_LOADED,
    HOOK_MP3_GET_BANK_SLOT_BUFFER,
    HOOK_USER_SELECT_USER_TRACK_INDEX,
    HOOK_SCRIPT_GET_MISSION_AUDIO_LOADING_STATUS,
    HOOK_SCRIPT_CLEAR_MISSION_AUDIO,
    HOOK_SCRIPT_RESET,
    HOOK_SOUND_MANAGER_RESET,
    HOOK_RADIO_ADD_IDENT_INDEX_TO_HISTORY,
    HOOK_RADIO_ADD_ADVERT_INDEX_TO_HISTORY,
    HOOK_RADIO_ADD_DJ_BANTER_INDEX_TO_HISTORY,
    HOOK_MP3_GET_SOUND_BANK_LOADING_STATUS,
    HOOK_MP3_GET_SOUND_LOADING_STATUS,
    HOOK_AMBIENCE_RESET,
    HOOK_CUTSCENE_PAUSE_TRACK,
    HOOK_CUTSCENE_CHECK_FOR_PAUSE,
    HOOK_FRONTEND_IS_RADIO_TUNE_SOUND_ACTIVE,
    HOOK_RADIO_GET_CURRENT_RADIO_STATION_ID,
    HOOK_RADIO_INITIALISE_RADIO_STATION_ID,
    HOOK_RADIO_RETUNE_RADIO,
    HOOK_RADIO_RESET_STATISTICS,
    HOOK_PS_DISABLE_SCANNER,
    HOOK_PS_CAN_WE_PLAY_NEW_SCANNER_DIALOGUE,
    HOOK_INIT_G_B5F880,
    HOOK_INIT_G_B5F884,
    HOOK_INIT_G_B5F888,
    HOOK_INIT_G_B5F88C,
    HOOK_INIT_G_B5F890,
    HOOK_INIT_G_B5F894,
    HOOK_INIT_84C1A0,
    HOOK_INIT_G_B5F89C,
    HOOK_INIT_G_B5F8A0,
    HOOK_INIT_G_B5F8A4,
    HOOK_INIT_G_B5F8A8,
    HOOK_INIT_G_B5F8AC,
    HOOK_INIT_G_B5F8B0,
    HOOK_INIT_G_B61290,
    HOOK_INIT_G_B61294,
    HOOK_INIT_G_B61298,
    HOOK_INIT_G_B6129C,
    HOOK_INIT_G_B612A0,
    HOOK_INIT_G_B612A4,
    HOOK_INIT_G_B612A8,
    HOOK_INIT_G_B612AC,
    HOOK_INIT_G_B612B0,
    HOOK_INIT_G_B612B4,
    HOOK_INIT_G_B612B8,
    HOOK_INIT_G_B612BC,
    HOOK_INIT_G_B612C0,
    HOOK_INIT_G_B612C4,
    HOOK_INIT_G_B612C8,
    HOOK_INIT_G_B612CC,
    HOOK_INIT_G_B612D0,
    HOOK_INIT_G_B612D4,
    HOOK_INIT_84C4F0,
    HOOK_INIT_G_B612E8,
    HOOK_INIT_G_B61300,
    HOOK_INIT_G_B61304,
    HOOK_INIT_G_B61308,
    HOOK_INIT_G_B6130C,
    HOOK_INIT_G_B61310,
    HOOK_INIT_G_B61314,
    HOOK_INIT_G_B6136C,
    HOOK_INIT_G_B61370,
    HOOK_INIT_G_B61374,
    HOOK_INIT_G_B61378,
    HOOK_INIT_G_B6137C,
    HOOK_INIT_G_B61380,
    HOOK_INIT_G_B6138C,
    HOOK_INIT_G_B61390,
    HOOK_INIT_G_B61394,
    HOOK_INIT_G_B61398,
    HOOK_INIT_G_B6139C,
    HOOK_INIT_G_B613A0,
    HOOK_INIT_G_B613A4,
    HOOK_INIT_G_B613A8,
    HOOK_INIT_G_B613AC,
    HOOK_INIT_G_B613B4,
    HOOK_INIT_G_B613B8,
    HOOK_INIT_G_B613C4,
    HOOK_INIT_G_B613C8,
    HOOK_INIT_G_B61C1C,
    HOOK_INIT_G_B61C20,
    HOOK_INIT_G_B61C24,
    HOOK_INIT_G_B61C28,
    HOOK_INIT_G_B61C2C,
    HOOK_INIT_G_B61C30,
    HOOK_INIT_G_B61CE0,
    HOOK_INIT_G_B61CE4,
    HOOK_INIT_G_B61CE8,
    HOOK_INIT_G_B61CEC,
    HOOK_INIT_G_B61CF0,
    HOOK_INIT_G_B61CF4,
    HOOK_INIT_G_B61D58,
    HOOK_INIT_G_B61D5C,
    HOOK_INIT_G_B61D60,
    HOOK_INIT_G_B61D64,
    HOOK_INIT_G_B61D68,
    HOOK_INIT_G_B61D6C,
    HOOK_INIT_G_B61D74,
    HOOK_INIT_G_B62C74,
    HOOK_INIT_G_B62C78,
    HOOK_INIT_G_B62C7C,
    HOOK_INIT_G_B62C80,
    HOOK_INIT_G_B62C84,
    HOOK_INIT_G_B62C88,
    HOOK_INIT_84CC30,
    HOOK_INIT_G_B62C8C,
    HOOK_INIT_G_B62C90,
    HOOK_INIT_G_B62C94,
    HOOK_INIT_G_B62C98,
    HOOK_INIT_G_B62C9C,
    HOOK_INIT_G_B62CA0,
    HOOK_INIT_G_B62CA8,
    HOOK_INIT_G_B6B96C,
    HOOK_INIT_84CD60,
    HOOK_INIT_G_B6BA08,
    HOOK_INIT_G_B6BA0C,
    HOOK_INIT_G_B6BA10,
    HOOK_INIT_G_B6BA14,
    HOOK_INIT_G_B6BA18,
    HOOK_INIT_G_B6BA1C,
    HOOK_INIT_G_B6BA30,
    HOOK_INIT_G_B6BA34,
    HOOK_INIT_G_B6BA48,
    HOOK_INIT_G_B6BA4C,
    HOOK_INIT_G_B6BA60,
    HOOK_INIT_G_B6BA7C,
    HOOK_INIT_G_B6BA8C,
    HOOK_INIT_G_B6BA98,
    HOOK_INIT_G_B6BADC,
    HOOK_INIT_G_B6BAE0,
    HOOK_INIT_G_B6BAE4,
    HOOK_INIT_G_B6BAE8,
    HOOK_INIT_G_B6BAEC,
    HOOK_INIT_G_B6BAF0,
    HOOK_INIT_G_B6BAF4,
    HOOK_INIT_G_B6BB00,
    HOOK_INIT_G_B6BB04,
    HOOK_INIT_G_B6BB08,
    HOOK_INIT_G_B6BB0C,
    HOOK_INIT_G_B6BB10,
    HOOK_INIT_G_B6BB14,
    HOOK_INIT_G_B6BC78,
    HOOK_INIT_G_B6BC7C,
    HOOK_INIT_G_B6BC80,
    HOOK_INIT_G_B6BC84,
    HOOK_INIT_G_B6BC88,
    HOOK_INIT_G_B6BC8C,
    HOOK_INIT_G_BD00E0,
    HOOK_INIT_G_BD00E4,
    HOOK_INIT_G_BD00E8,
    HOOK_INIT_G_BD00EC,
    HOOK_INIT_G_BD00F0,
    HOOK_INIT_G_BD00F4,
    HOOK_INIT_G_C0B1D8,
];
