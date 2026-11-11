//! Header-constant static initialisers (`__init_g_*`, CRT `__xc` entries) that CAT-5 / the UNK
//! classification assign to WORLD (PORT-WORLD-2): 204 are `fld [a]; (fmul|fdiv|fdivr) [b]...; fstp [g]; ret`, six
//! construct a manager singleton or array and register its `atexit` destructor.
//! Generated from the 1.0 US bytes; the operands are read from their addresses (constants in `.rdata`
//! as their bit patterns, writable globals through `g()`), in the original's x87 order (PC=24 model; a
//! check at generation found no operand set whose PC=53 result differs).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, x};
use super::statics::{atexit, eh_vector_ctor};

sa_core::globals! {
    list GLOBALS;
}

/// `__init_g_9690d0` (0x849860): `fld fmul fstp` -> f32 `[0x9690D0]`.
#[hook(addr = 0x849860, conv = cdecl, name = "__init_g_9690d0", expect = "D9 05 F4 8C 85 00")]
pub fn init_g_9690d0() {
    g::<f32>(0x9690D0).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_9690d4` (0x849880): `fld fdiv fstp` -> f32 `[0x9690D4]`.
#[hook(addr = 0x849880, conv = cdecl, name = "__init_g_9690d4", expect = "D9 05 24 86 85 00")]
pub fn init_g_9690d4() {
    g::<f32>(0x9690D4).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_9690d8` (0x8498A0): `fld fdiv fstp` -> f32 `[0x9690D8]`.
#[hook(addr = 0x8498A0, conv = cdecl, name = "__init_g_9690d8", expect = "D9 05 28 86 85 00")]
pub fn init_g_9690d8() {
    g::<f32>(0x9690D8).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_9690dc` (0x8498C0): `fld fmul fdivr fstp` -> f32 `[0x9690DC]`.
#[hook(addr = 0x8498C0, conv = cdecl, name = "__init_g_9690dc", expect = "D9 05 FC 8C 85 00")]
pub fn init_g_9690dc() {
    g::<f32>(0x9690DC).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_9690e0` (0x8498E0): `fld fmul fstp` -> f32 `[0x9690E0]`.
#[hook(addr = 0x8498E0, conv = cdecl, name = "__init_g_9690e0", expect = "D9 05 DC 90 96 00")]
pub fn init_g_9690e0() {
    g::<f32>(0x9690E0).write(x(g::<f32>(0x9690DC).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_9690e4` (0x849900): `fld fdiv fstp` -> f32 `[0x9690E4]`.
#[hook(addr = 0x849900, conv = cdecl, name = "__init_g_9690e4", expect = "D9 05 30 86 85 00")]
pub fn init_g_9690e4() {
    g::<f32>(0x9690E4).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_96a708` (0x849AA0): `fld fmul fstp` -> f32 `[0x96A708]`.
#[hook(addr = 0x849AA0, conv = cdecl, name = "__init_g_96a708", expect = "D9 05 F4 94 85 00")]
pub fn init_g_96a708() {
    g::<f32>(0x96A708).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_96a70c` (0x849AC0): `fld fdiv fstp` -> f32 `[0x96A70C]`.
#[hook(addr = 0x849AC0, conv = cdecl, name = "__init_g_96a70c", expect = "D9 05 24 86 85 00")]
pub fn init_g_96a70c() {
    g::<f32>(0x96A70C).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_96a710` (0x849AE0): `fld fdiv fstp` -> f32 `[0x96A710]`.
#[hook(addr = 0x849AE0, conv = cdecl, name = "__init_g_96a710", expect = "D9 05 28 86 85 00")]
pub fn init_g_96a710() {
    g::<f32>(0x96A710).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_96a714` (0x849B00): `fld fmul fdivr fstp` -> f32 `[0x96A714]`.
#[hook(addr = 0x849B00, conv = cdecl, name = "__init_g_96a714", expect = "D9 05 FC 94 85 00")]
pub fn init_g_96a714() {
    g::<f32>(0x96A714).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_96a7e4` (0x849B60): `fld fmul fstp` -> f32 `[0x96A7E4]`.
#[hook(addr = 0x849B60, conv = cdecl, name = "__init_g_96a7e4", expect = "D9 05 30 95 85 00")]
pub fn init_g_96a7e4() {
    g::<f32>(0x96A7E4).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_96a7e8` (0x849B80): `fld fdiv fstp` -> f32 `[0x96A7E8]`.
#[hook(addr = 0x849B80, conv = cdecl, name = "__init_g_96a7e8", expect = "D9 05 24 86 85 00")]
pub fn init_g_96a7e8() {
    g::<f32>(0x96A7E8).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_96a7ec` (0x849BA0): `fld fdiv fstp` -> f32 `[0x96A7EC]`.
#[hook(addr = 0x849BA0, conv = cdecl, name = "__init_g_96a7ec", expect = "D9 05 28 86 85 00")]
pub fn init_g_96a7ec() {
    g::<f32>(0x96A7EC).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_96a7f0` (0x849BC0): `fld fmul fdivr fstp` -> f32 `[0x96A7F0]`.
#[hook(addr = 0x849BC0, conv = cdecl, name = "__init_g_96a7f0", expect = "D9 05 38 95 85 00")]
pub fn init_g_96a7f0() {
    g::<f32>(0x96A7F0).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_96a7f4` (0x849BE0): `fld fmul fstp` -> f32 `[0x96A7F4]`.
#[hook(addr = 0x849BE0, conv = cdecl, name = "__init_g_96a7f4", expect = "D9 05 F0 A7 96 00")]
pub fn init_g_96a7f4() {
    g::<f32>(0x96A7F4).write(x(g::<f32>(0x96A7F0).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_96a7f8` (0x849C00): `fld fdiv fstp` -> f32 `[0x96A7F8]`.
#[hook(addr = 0x849C00, conv = cdecl, name = "__init_g_96a7f8", expect = "D9 05 30 86 85 00")]
pub fn init_g_96a7f8() {
    g::<f32>(0x96A7F8).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_96aba4` (0x849D10): `fld fmul fstp` -> f32 `[0x96ABA4]`.
#[hook(addr = 0x849D10, conv = cdecl, name = "__init_g_96aba4", expect = "D9 05 A0 98 85 00")]
pub fn init_g_96aba4() {
    g::<f32>(0x96ABA4).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_96aba8` (0x849D30): `fld fdiv fstp` -> f32 `[0x96ABA8]`.
#[hook(addr = 0x849D30, conv = cdecl, name = "__init_g_96aba8", expect = "D9 05 24 86 85 00")]
pub fn init_g_96aba8() {
    g::<f32>(0x96ABA8).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_96abac` (0x849D50): `fld fdiv fstp` -> f32 `[0x96ABAC]`.
#[hook(addr = 0x849D50, conv = cdecl, name = "__init_g_96abac", expect = "D9 05 28 86 85 00")]
pub fn init_g_96abac() {
    g::<f32>(0x96ABAC).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_96abb0` (0x849D70): `fld fmul fdivr fstp` -> f32 `[0x96ABB0]`.
#[hook(addr = 0x849D70, conv = cdecl, name = "__init_g_96abb0", expect = "D9 05 A8 98 85 00")]
pub fn init_g_96abb0() {
    g::<f32>(0x96ABB0).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_96abb4` (0x849D90): `fld fmul fstp` -> f32 `[0x96ABB4]`.
#[hook(addr = 0x849D90, conv = cdecl, name = "__init_g_96abb4", expect = "D9 05 B0 AB 96 00")]
pub fn init_g_96abb4() {
    g::<f32>(0x96ABB4).write(x(g::<f32>(0x96ABB0).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_96abb8` (0x849DB0): `fld fdiv fstp` -> f32 `[0x96ABB8]`.
#[hook(addr = 0x849DB0, conv = cdecl, name = "__init_g_96abb8", expect = "D9 05 30 86 85 00")]
pub fn init_g_96abb8() {
    g::<f32>(0x96ABB8).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_96c02c` (0x849DD0): `fld fmul fstp` -> f32 `[0x96C02C]`.
#[hook(addr = 0x849DD0, conv = cdecl, name = "__init_g_96c02c", expect = "D9 05 7C 99 85 00")]
pub fn init_g_96c02c() {
    g::<f32>(0x96C02C).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_96c030` (0x849DF0): `fld fdiv fstp` -> f32 `[0x96C030]`.
#[hook(addr = 0x849DF0, conv = cdecl, name = "__init_g_96c030", expect = "D9 05 24 86 85 00")]
pub fn init_g_96c030() {
    g::<f32>(0x96C030).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_96c034` (0x849E10): `fld fdiv fstp` -> f32 `[0x96C034]`.
#[hook(addr = 0x849E10, conv = cdecl, name = "__init_g_96c034", expect = "D9 05 28 86 85 00")]
pub fn init_g_96c034() {
    g::<f32>(0x96C034).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_96c038` (0x849E30): `fld fmul fdivr fstp` -> f32 `[0x96C038]`.
#[hook(addr = 0x849E30, conv = cdecl, name = "__init_g_96c038", expect = "D9 05 84 99 85 00")]
pub fn init_g_96c038() {
    g::<f32>(0x96C038).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_96c03c` (0x849E50): `fld fmul fstp` -> f32 `[0x96C03C]`.
#[hook(addr = 0x849E50, conv = cdecl, name = "__init_g_96c03c", expect = "D9 05 38 C0 96 00")]
pub fn init_g_96c03c() {
    g::<f32>(0x96C03C).write(x(g::<f32>(0x96C038).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_96c040` (0x849E70): `fld fdiv fstp` -> f32 `[0x96C040]`.
#[hook(addr = 0x849E70, conv = cdecl, name = "__init_g_96c040", expect = "D9 05 30 86 85 00")]
pub fn init_g_96c040() {
    g::<f32>(0x96C040).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_977ba0` (0x84A0A0): `fld fmul fstp` -> f32 `[0x977BA0]`.
#[hook(addr = 0x84A0A0, conv = cdecl, name = "__init_g_977ba0", expect = "D9 05 CC 9A 85 00")]
pub fn init_g_977ba0() {
    g::<f32>(0x977BA0).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_977ba4` (0x84A0C0): `fld fdiv fstp` -> f32 `[0x977BA4]`.
#[hook(addr = 0x84A0C0, conv = cdecl, name = "__init_g_977ba4", expect = "D9 05 24 86 85 00")]
pub fn init_g_977ba4() {
    g::<f32>(0x977BA4).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_977ba8` (0x84A0E0): `fld fdiv fstp` -> f32 `[0x977BA8]`.
#[hook(addr = 0x84A0E0, conv = cdecl, name = "__init_g_977ba8", expect = "D9 05 28 86 85 00")]
pub fn init_g_977ba8() {
    g::<f32>(0x977BA8).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_977bac` (0x84A100): `fld fmul fdivr fstp` -> f32 `[0x977BAC]`.
#[hook(addr = 0x84A100, conv = cdecl, name = "__init_g_977bac", expect = "D9 05 D4 9A 85 00")]
pub fn init_g_977bac() {
    g::<f32>(0x977BAC).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_977bb0` (0x84A120): `fld fmul fstp` -> f32 `[0x977BB0]`.
#[hook(addr = 0x84A120, conv = cdecl, name = "__init_g_977bb0", expect = "D9 05 AC 7B 97 00")]
pub fn init_g_977bb0() {
    g::<f32>(0x977BB0).write(x(g::<f32>(0x977BAC).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_977bb4` (0x84A140): `fld fdiv fstp` -> f32 `[0x977BB4]`.
#[hook(addr = 0x84A140, conv = cdecl, name = "__init_g_977bb4", expect = "D9 05 30 86 85 00")]
pub fn init_g_977bb4() {
    g::<f32>(0x977BB4).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_977bcc` (0x84A160): `fld fmul fstp` -> f32 `[0x977BCC]`.
#[hook(addr = 0x84A160, conv = cdecl, name = "__init_g_977bcc", expect = "D9 05 E0 9A 85 00")]
pub fn init_g_977bcc() {
    g::<f32>(0x977BCC).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_977bd0` (0x84A180): `fld fdiv fstp` -> f32 `[0x977BD0]`.
#[hook(addr = 0x84A180, conv = cdecl, name = "__init_g_977bd0", expect = "D9 05 24 86 85 00")]
pub fn init_g_977bd0() {
    g::<f32>(0x977BD0).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_977bd4` (0x84A1A0): `fld fdiv fstp` -> f32 `[0x977BD4]`.
#[hook(addr = 0x84A1A0, conv = cdecl, name = "__init_g_977bd4", expect = "D9 05 28 86 85 00")]
pub fn init_g_977bd4() {
    g::<f32>(0x977BD4).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_977bd8` (0x84A1C0): `fld fmul fdivr fstp` -> f32 `[0x977BD8]`.
#[hook(addr = 0x84A1C0, conv = cdecl, name = "__init_g_977bd8", expect = "D9 05 E8 9A 85 00")]
pub fn init_g_977bd8() {
    g::<f32>(0x977BD8).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_977bdc` (0x84A1E0): `fld fmul fstp` -> f32 `[0x977BDC]`.
#[hook(addr = 0x84A1E0, conv = cdecl, name = "__init_g_977bdc", expect = "D9 05 D8 7B 97 00")]
pub fn init_g_977bdc() {
    g::<f32>(0x977BDC).write(x(g::<f32>(0x977BD8).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_977be0` (0x84A200): `fld fdiv fstp` -> f32 `[0x977BE0]`.
#[hook(addr = 0x84A200, conv = cdecl, name = "__init_g_977be0", expect = "D9 05 30 86 85 00")]
pub fn init_g_977be0() {
    g::<f32>(0x977BE0).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_97f64c` (0x84A3C0): `fld fmul fstp` -> f32 `[0x97F64C]`.
#[hook(addr = 0x84A3C0, conv = cdecl, name = "__init_g_97f64c", expect = "D9 05 04 9C 85 00")]
pub fn init_g_97f64c() {
    g::<f32>(0x97F64C).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_97f650` (0x84A3E0): `fld fdiv fstp` -> f32 `[0x97F650]`.
#[hook(addr = 0x84A3E0, conv = cdecl, name = "__init_g_97f650", expect = "D9 05 24 86 85 00")]
pub fn init_g_97f650() {
    g::<f32>(0x97F650).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_97f654` (0x84A400): `fld fdiv fstp` -> f32 `[0x97F654]`.
#[hook(addr = 0x84A400, conv = cdecl, name = "__init_g_97f654", expect = "D9 05 28 86 85 00")]
pub fn init_g_97f654() {
    g::<f32>(0x97F654).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_97f658` (0x84A420): `fld fmul fdivr fstp` -> f32 `[0x97F658]`.
#[hook(addr = 0x84A420, conv = cdecl, name = "__init_g_97f658", expect = "D9 05 0C 9C 85 00")]
pub fn init_g_97f658() {
    g::<f32>(0x97F658).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_97f65c` (0x84A440): `fld fmul fstp` -> f32 `[0x97F65C]`.
#[hook(addr = 0x84A440, conv = cdecl, name = "__init_g_97f65c", expect = "D9 05 58 F6 97 00")]
pub fn init_g_97f65c() {
    g::<f32>(0x97F65C).write(x(g::<f32>(0x97F658).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_97f660` (0x84A460): `fld fdiv fstp` -> f32 `[0x97F660]`.
#[hook(addr = 0x84A460, conv = cdecl, name = "__init_g_97f660", expect = "D9 05 30 86 85 00")]
pub fn init_g_97f660() {
    g::<f32>(0x97F660).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a43098` (0x84A480): `fld fmul fstp` -> f32 `[0xA43098]`.
#[hook(addr = 0x84A480, conv = cdecl, name = "__init_g_a43098", expect = "D9 05 10 9C 85 00")]
pub fn init_g_a43098() {
    g::<f32>(0xA43098).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_a4309c` (0x84A4A0): `fld fdiv fstp` -> f32 `[0xA4309C]`.
#[hook(addr = 0x84A4A0, conv = cdecl, name = "__init_g_a4309c", expect = "D9 05 24 86 85 00")]
pub fn init_g_a4309c() {
    g::<f32>(0xA4309C).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a430a0` (0x84A4C0): `fld fdiv fstp` -> f32 `[0xA430A0]`.
#[hook(addr = 0x84A4C0, conv = cdecl, name = "__init_g_a430a0", expect = "D9 05 28 86 85 00")]
pub fn init_g_a430a0() {
    g::<f32>(0xA430A0).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_a430a4` (0x84A4E0): `fld fmul fdivr fstp` -> f32 `[0xA430A4]`.
#[hook(addr = 0x84A4E0, conv = cdecl, name = "__init_g_a430a4", expect = "D9 05 18 9C 85 00")]
pub fn init_g_a430a4() {
    g::<f32>(0xA430A4).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_a430a8` (0x84A500): `fld fmul fstp` -> f32 `[0xA430A8]`.
#[hook(addr = 0x84A500, conv = cdecl, name = "__init_g_a430a8", expect = "D9 05 A4 30 A4 00")]
pub fn init_g_a430a8() {
    g::<f32>(0xA430A8).write(x(g::<f32>(0xA430A4).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_a430ac` (0x84A520): `fld fdiv fstp` -> f32 `[0xA430AC]`.
#[hook(addr = 0x84A520, conv = cdecl, name = "__init_g_a430ac", expect = "D9 05 30 86 85 00")]
pub fn init_g_a430ac() {
    g::<f32>(0xA430AC).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a43310` (0x84A560): `fld fmul fstp` -> f32 `[0xA43310]`.
#[hook(addr = 0x84A560, conv = cdecl, name = "__init_g_a43310", expect = "D9 05 70 9C 85 00")]
pub fn init_g_a43310() {
    g::<f32>(0xA43310).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_a95160` (0x84A8A0): `fld fmul fstp` -> f32 `[0xA95160]`.
#[hook(addr = 0x84A8A0, conv = cdecl, name = "__init_g_a95160", expect = "D9 05 74 9E 85 00")]
pub fn init_g_a95160() {
    g::<f32>(0xA95160).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_a95164` (0x84A8C0): `fld fdiv fstp` -> f32 `[0xA95164]`.
#[hook(addr = 0x84A8C0, conv = cdecl, name = "__init_g_a95164", expect = "D9 05 24 86 85 00")]
pub fn init_g_a95164() {
    g::<f32>(0xA95164).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a95168` (0x84A8E0): `fld fdiv fstp` -> f32 `[0xA95168]`.
#[hook(addr = 0x84A8E0, conv = cdecl, name = "__init_g_a95168", expect = "D9 05 28 86 85 00")]
pub fn init_g_a95168() {
    g::<f32>(0xA95168).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_a9516c` (0x84A900): `fld fmul fdivr fstp` -> f32 `[0xA9516C]`.
#[hook(addr = 0x84A900, conv = cdecl, name = "__init_g_a9516c", expect = "D9 05 7C 9E 85 00")]
pub fn init_g_a9516c() {
    g::<f32>(0xA9516C).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_a95170` (0x84A920): `fld fmul fstp` -> f32 `[0xA95170]`.
#[hook(addr = 0x84A920, conv = cdecl, name = "__init_g_a95170", expect = "D9 05 6C 51 A9 00")]
pub fn init_g_a95170() {
    g::<f32>(0xA95170).write(x(g::<f32>(0xA9516C).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_a95174` (0x84A940): `fld fdiv fstp` -> f32 `[0xA95174]`.
#[hook(addr = 0x84A940, conv = cdecl, name = "__init_g_a95174", expect = "D9 05 30 86 85 00")]
pub fn init_g_a95174() {
    g::<f32>(0xA95174).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a95194` (0x84AA20): `fld fmul fstp` -> f32 `[0xA95194]`.
#[hook(addr = 0x84AA20, conv = cdecl, name = "__init_g_a95194", expect = "D9 05 E8 9E 85 00")]
pub fn init_g_a95194() {
    g::<f32>(0xA95194).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_a95198` (0x84AA40): `fld fdiv fstp` -> f32 `[0xA95198]`.
#[hook(addr = 0x84AA40, conv = cdecl, name = "__init_g_a95198", expect = "D9 05 24 86 85 00")]
pub fn init_g_a95198() {
    g::<f32>(0xA95198).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a9519c` (0x84AA60): `fld fdiv fstp` -> f32 `[0xA9519C]`.
#[hook(addr = 0x84AA60, conv = cdecl, name = "__init_g_a9519c", expect = "D9 05 28 86 85 00")]
pub fn init_g_a9519c() {
    g::<f32>(0xA9519C).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_a951a0` (0x84AA80): `fld fmul fdivr fstp` -> f32 `[0xA951A0]`.
#[hook(addr = 0x84AA80, conv = cdecl, name = "__init_g_a951a0", expect = "D9 05 F0 9E 85 00")]
pub fn init_g_a951a0() {
    g::<f32>(0xA951A0).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_a951a4` (0x84AAA0): `fld fmul fstp` -> f32 `[0xA951A4]`.
#[hook(addr = 0x84AAA0, conv = cdecl, name = "__init_g_a951a4", expect = "D9 05 A0 51 A9 00")]
pub fn init_g_a951a4() {
    g::<f32>(0xA951A4).write(x(g::<f32>(0xA951A0).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_a951a8` (0x84AAC0): `fld fdiv fstp` -> f32 `[0xA951A8]`.
#[hook(addr = 0x84AAC0, conv = cdecl, name = "__init_g_a951a8", expect = "D9 05 30 86 85 00")]
pub fn init_g_a951a8() {
    g::<f32>(0xA951A8).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a951ac` (0x84AAE0): `fld fmul fstp` -> f32 `[0xA951AC]`.
#[hook(addr = 0x84AAE0, conv = cdecl, name = "__init_g_a951ac", expect = "D9 05 04 9F 85 00")]
pub fn init_g_a951ac() {
    g::<f32>(0xA951AC).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_a951b0` (0x84AB00): `fld fdiv fstp` -> f32 `[0xA951B0]`.
#[hook(addr = 0x84AB00, conv = cdecl, name = "__init_g_a951b0", expect = "D9 05 24 86 85 00")]
pub fn init_g_a951b0() {
    g::<f32>(0xA951B0).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a951b4` (0x84AB20): `fld fdiv fstp` -> f32 `[0xA951B4]`.
#[hook(addr = 0x84AB20, conv = cdecl, name = "__init_g_a951b4", expect = "D9 05 28 86 85 00")]
pub fn init_g_a951b4() {
    g::<f32>(0xA951B4).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_a951b8` (0x84AB40): `fld fmul fdivr fstp` -> f32 `[0xA951B8]`.
#[hook(addr = 0x84AB40, conv = cdecl, name = "__init_g_a951b8", expect = "D9 05 0C 9F 85 00")]
pub fn init_g_a951b8() {
    g::<f32>(0xA951B8).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_a951bc` (0x84AB60): `fld fmul fstp` -> f32 `[0xA951BC]`.
#[hook(addr = 0x84AB60, conv = cdecl, name = "__init_g_a951bc", expect = "D9 05 B8 51 A9 00")]
pub fn init_g_a951bc() {
    g::<f32>(0xA951BC).write(x(g::<f32>(0xA951B8).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_a951c0` (0x84AB80): `fld fdiv fstp` -> f32 `[0xA951C0]`.
#[hook(addr = 0x84AB80, conv = cdecl, name = "__init_g_a951c0", expect = "D9 05 30 86 85 00")]
pub fn init_g_a951c0() {
    g::<f32>(0xA951C0).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a951c4` (0x84ABA0): `fld fmul fstp` -> f32 `[0xA951C4]`.
#[hook(addr = 0x84ABA0, conv = cdecl, name = "__init_g_a951c4", expect = "D9 05 10 9F 85 00")]
pub fn init_g_a951c4() {
    g::<f32>(0xA951C4).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_a951c8` (0x84ABC0): `fld fdiv fstp` -> f32 `[0xA951C8]`.
#[hook(addr = 0x84ABC0, conv = cdecl, name = "__init_g_a951c8", expect = "D9 05 24 86 85 00")]
pub fn init_g_a951c8() {
    g::<f32>(0xA951C8).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a951cc` (0x84ABE0): `fld fdiv fstp` -> f32 `[0xA951CC]`.
#[hook(addr = 0x84ABE0, conv = cdecl, name = "__init_g_a951cc", expect = "D9 05 28 86 85 00")]
pub fn init_g_a951cc() {
    g::<f32>(0xA951CC).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_a951d0` (0x84AC00): `fld fmul fdivr fstp` -> f32 `[0xA951D0]`.
#[hook(addr = 0x84AC00, conv = cdecl, name = "__init_g_a951d0", expect = "D9 05 18 9F 85 00")]
pub fn init_g_a951d0() {
    g::<f32>(0xA951D0).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_a951d4` (0x84AC20): `fld fmul fstp` -> f32 `[0xA951D4]`.
#[hook(addr = 0x84AC20, conv = cdecl, name = "__init_g_a951d4", expect = "D9 05 D0 51 A9 00")]
pub fn init_g_a951d4() {
    g::<f32>(0xA951D4).write(x(g::<f32>(0xA951D0).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_a951d8` (0x84AC40): `fld fdiv fstp` -> f32 `[0xA951D8]`.
#[hook(addr = 0x84AC40, conv = cdecl, name = "__init_g_a951d8", expect = "D9 05 30 86 85 00")]
pub fn init_g_a951d8() {
    g::<f32>(0xA951D8).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a9577c` (0x84AD20): `fld fmul fstp` -> f32 `[0xA9577C]`.
#[hook(addr = 0x84AD20, conv = cdecl, name = "__init_g_a9577c", expect = "D9 05 44 9F 85 00")]
pub fn init_g_a9577c() {
    g::<f32>(0xA9577C).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_a95780` (0x84AD40): `fld fdiv fstp` -> f32 `[0xA95780]`.
#[hook(addr = 0x84AD40, conv = cdecl, name = "__init_g_a95780", expect = "D9 05 24 86 85 00")]
pub fn init_g_a95780() {
    g::<f32>(0xA95780).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a95784` (0x84AD60): `fld fdiv fstp` -> f32 `[0xA95784]`.
#[hook(addr = 0x84AD60, conv = cdecl, name = "__init_g_a95784", expect = "D9 05 28 86 85 00")]
pub fn init_g_a95784() {
    g::<f32>(0xA95784).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_a95788` (0x84AD80): `fld fmul fdivr fstp` -> f32 `[0xA95788]`.
#[hook(addr = 0x84AD80, conv = cdecl, name = "__init_g_a95788", expect = "D9 05 4C 9F 85 00")]
pub fn init_g_a95788() {
    g::<f32>(0xA95788).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_a9578c` (0x84ADA0): `fld fmul fstp` -> f32 `[0xA9578C]`.
#[hook(addr = 0x84ADA0, conv = cdecl, name = "__init_g_a9578c", expect = "D9 05 88 57 A9 00")]
pub fn init_g_a9578c() {
    g::<f32>(0xA9578C).write(x(g::<f32>(0xA95788).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_a95790` (0x84ADC0): `fld fdiv fstp` -> f32 `[0xA95790]`.
#[hook(addr = 0x84ADC0, conv = cdecl, name = "__init_g_a95790", expect = "D9 05 30 86 85 00")]
pub fn init_g_a95790() {
    g::<f32>(0xA95790).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a95794` (0x84ADE0): `fld fmul fstp` -> f32 `[0xA95794]`.
#[hook(addr = 0x84ADE0, conv = cdecl, name = "__init_g_a95794", expect = "D9 05 54 9F 85 00")]
pub fn init_g_a95794() {
    g::<f32>(0xA95794).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_a95798` (0x84AE00): `fld fdiv fstp` -> f32 `[0xA95798]`.
#[hook(addr = 0x84AE00, conv = cdecl, name = "__init_g_a95798", expect = "D9 05 24 86 85 00")]
pub fn init_g_a95798() {
    g::<f32>(0xA95798).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a9579c` (0x84AE20): `fld fdiv fstp` -> f32 `[0xA9579C]`.
#[hook(addr = 0x84AE20, conv = cdecl, name = "__init_g_a9579c", expect = "D9 05 28 86 85 00")]
pub fn init_g_a9579c() {
    g::<f32>(0xA9579C).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_a957a0` (0x84AE40): `fld fmul fdivr fstp` -> f32 `[0xA957A0]`.
#[hook(addr = 0x84AE40, conv = cdecl, name = "__init_g_a957a0", expect = "D9 05 5C 9F 85 00")]
pub fn init_g_a957a0() {
    g::<f32>(0xA957A0).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_a957a4` (0x84AE60): `fld fmul fstp` -> f32 `[0xA957A4]`.
#[hook(addr = 0x84AE60, conv = cdecl, name = "__init_g_a957a4", expect = "D9 05 A0 57 A9 00")]
pub fn init_g_a957a4() {
    g::<f32>(0xA957A4).write(x(g::<f32>(0xA957A0).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_a957a8` (0x84AE80): `fld fdiv fstp` -> f32 `[0xA957A8]`.
#[hook(addr = 0x84AE80, conv = cdecl, name = "__init_g_a957a8", expect = "D9 05 30 86 85 00")]
pub fn init_g_a957a8() {
    g::<f32>(0xA957A8).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a957ac` (0x84AEA0): `fld fmul fstp` -> f32 `[0xA957AC]`.
#[hook(addr = 0x84AEA0, conv = cdecl, name = "__init_g_a957ac", expect = "D9 05 84 9F 85 00")]
pub fn init_g_a957ac() {
    g::<f32>(0xA957AC).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_a957b0` (0x84AEC0): `fld fdiv fstp` -> f32 `[0xA957B0]`.
#[hook(addr = 0x84AEC0, conv = cdecl, name = "__init_g_a957b0", expect = "D9 05 24 86 85 00")]
pub fn init_g_a957b0() {
    g::<f32>(0xA957B0).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a957b4` (0x84AEE0): `fld fdiv fstp` -> f32 `[0xA957B4]`.
#[hook(addr = 0x84AEE0, conv = cdecl, name = "__init_g_a957b4", expect = "D9 05 28 86 85 00")]
pub fn init_g_a957b4() {
    g::<f32>(0xA957B4).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_a957b8` (0x84AF00): `fld fmul fdivr fstp` -> f32 `[0xA957B8]`.
#[hook(addr = 0x84AF00, conv = cdecl, name = "__init_g_a957b8", expect = "D9 05 8C 9F 85 00")]
pub fn init_g_a957b8() {
    g::<f32>(0xA957B8).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_a957bc` (0x84AF20): `fld fmul fstp` -> f32 `[0xA957BC]`.
#[hook(addr = 0x84AF20, conv = cdecl, name = "__init_g_a957bc", expect = "D9 05 B8 57 A9 00")]
pub fn init_g_a957bc() {
    g::<f32>(0xA957BC).write(x(g::<f32>(0xA957B8).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_a957c0` (0x84AF40): `fld fdiv fstp` -> f32 `[0xA957C0]`.
#[hook(addr = 0x84AF40, conv = cdecl, name = "__init_g_a957c0", expect = "D9 05 30 86 85 00")]
pub fn init_g_a957c0() {
    g::<f32>(0xA957C0).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a957c4` (0x84AF60): `fld fmul fstp` -> f32 `[0xA957C4]`.
#[hook(addr = 0x84AF60, conv = cdecl, name = "__init_g_a957c4", expect = "D9 05 C4 A0 85 00")]
pub fn init_g_a957c4() {
    g::<f32>(0xA957C4).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_a957c8` (0x84AF80): `fld fdiv fstp` -> f32 `[0xA957C8]`.
#[hook(addr = 0x84AF80, conv = cdecl, name = "__init_g_a957c8", expect = "D9 05 24 86 85 00")]
pub fn init_g_a957c8() {
    g::<f32>(0xA957C8).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a957cc` (0x84AFA0): `fld fdiv fstp` -> f32 `[0xA957CC]`.
#[hook(addr = 0x84AFA0, conv = cdecl, name = "__init_g_a957cc", expect = "D9 05 28 86 85 00")]
pub fn init_g_a957cc() {
    g::<f32>(0xA957CC).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_a957d0` (0x84AFC0): `fld fmul fdivr fstp` -> f32 `[0xA957D0]`.
#[hook(addr = 0x84AFC0, conv = cdecl, name = "__init_g_a957d0", expect = "D9 05 CC A0 85 00")]
pub fn init_g_a957d0() {
    g::<f32>(0xA957D0).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_a957d4` (0x84AFE0): `fld fmul fstp` -> f32 `[0xA957D4]`.
#[hook(addr = 0x84AFE0, conv = cdecl, name = "__init_g_a957d4", expect = "D9 05 D0 57 A9 00")]
pub fn init_g_a957d4() {
    g::<f32>(0xA957D4).write(x(g::<f32>(0xA957D0).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_a957d8` (0x84B000): `fld fdiv fstp` -> f32 `[0xA957D8]`.
#[hook(addr = 0x84B000, conv = cdecl, name = "__init_g_a957d8", expect = "D9 05 30 86 85 00")]
pub fn init_g_a957d8() {
    g::<f32>(0xA957D8).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a957dc` (0x84B020): `fld fmul fstp` -> f32 `[0xA957DC]`.
#[hook(addr = 0x84B020, conv = cdecl, name = "__init_g_a957dc", expect = "D9 05 20 A3 85 00")]
pub fn init_g_a957dc() {
    g::<f32>(0xA957DC).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_a957e0` (0x84B040): `fld fdiv fstp` -> f32 `[0xA957E0]`.
#[hook(addr = 0x84B040, conv = cdecl, name = "__init_g_a957e0", expect = "D9 05 24 86 85 00")]
pub fn init_g_a957e0() {
    g::<f32>(0xA957E0).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a957e4` (0x84B060): `fld fdiv fstp` -> f32 `[0xA957E4]`.
#[hook(addr = 0x84B060, conv = cdecl, name = "__init_g_a957e4", expect = "D9 05 28 86 85 00")]
pub fn init_g_a957e4() {
    g::<f32>(0xA957E4).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_a957e8` (0x84B080): `fld fmul fdivr fstp` -> f32 `[0xA957E8]`.
#[hook(addr = 0x84B080, conv = cdecl, name = "__init_g_a957e8", expect = "D9 05 28 A3 85 00")]
pub fn init_g_a957e8() {
    g::<f32>(0xA957E8).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_a957ec` (0x84B0A0): `fld fmul fstp` -> f32 `[0xA957EC]`.
#[hook(addr = 0x84B0A0, conv = cdecl, name = "__init_g_a957ec", expect = "D9 05 E8 57 A9 00")]
pub fn init_g_a957ec() {
    g::<f32>(0xA957EC).write(x(g::<f32>(0xA957E8).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_a957f0` (0x84B0C0): `fld fdiv fstp` -> f32 `[0xA957F0]`.
#[hook(addr = 0x84B0C0, conv = cdecl, name = "__init_g_a957f0", expect = "D9 05 30 86 85 00")]
pub fn init_g_a957f0() {
    g::<f32>(0xA957F0).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a957f4` (0x84B0E0): `fld fmul fstp` -> f32 `[0xA957F4]`.
#[hook(addr = 0x84B0E0, conv = cdecl, name = "__init_g_a957f4", expect = "D9 05 18 A4 85 00")]
pub fn init_g_a957f4() {
    g::<f32>(0xA957F4).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_a95800` (0x84B100): `fld fmul fstp` -> f32 `[0xA95800]`.
#[hook(addr = 0x84B100, conv = cdecl, name = "__init_g_a95800", expect = "D9 05 1C A4 85 00")]
pub fn init_g_a95800() {
    g::<f32>(0xA95800).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_a95804` (0x84B120): `fld fdiv fstp` -> f32 `[0xA95804]`.
#[hook(addr = 0x84B120, conv = cdecl, name = "__init_g_a95804", expect = "D9 05 24 86 85 00")]
pub fn init_g_a95804() {
    g::<f32>(0xA95804).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a95808` (0x84B140): `fld fdiv fstp` -> f32 `[0xA95808]`.
#[hook(addr = 0x84B140, conv = cdecl, name = "__init_g_a95808", expect = "D9 05 28 86 85 00")]
pub fn init_g_a95808() {
    g::<f32>(0xA95808).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_a9580c` (0x84B160): `fld fmul fdivr fstp` -> f32 `[0xA9580C]`.
#[hook(addr = 0x84B160, conv = cdecl, name = "__init_g_a9580c", expect = "D9 05 24 A4 85 00")]
pub fn init_g_a9580c() {
    g::<f32>(0xA9580C).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_a95810` (0x84B180): `fld fmul fstp` -> f32 `[0xA95810]`.
#[hook(addr = 0x84B180, conv = cdecl, name = "__init_g_a95810", expect = "D9 05 0C 58 A9 00")]
pub fn init_g_a95810() {
    g::<f32>(0xA95810).write(x(g::<f32>(0xA9580C).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_a95814` (0x84B1A0): `fld fdiv fstp` -> f32 `[0xA95814]`.
#[hook(addr = 0x84B1A0, conv = cdecl, name = "__init_g_a95814", expect = "D9 05 30 86 85 00")]
pub fn init_g_a95814() {
    g::<f32>(0xA95814).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a9a7f4` (0x84B1E0): `fld fmul fstp` -> f32 `[0xA9A7F4]`.
#[hook(addr = 0x84B1E0, conv = cdecl, name = "__init_g_a9a7f4", expect = "D9 05 D0 A4 85 00")]
pub fn init_g_a9a7f4() {
    g::<f32>(0xA9A7F4).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_a9a7f8` (0x84B200): `fld fdiv fstp` -> f32 `[0xA9A7F8]`.
#[hook(addr = 0x84B200, conv = cdecl, name = "__init_g_a9a7f8", expect = "D9 05 24 86 85 00")]
pub fn init_g_a9a7f8() {
    g::<f32>(0xA9A7F8).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_a9a7fc` (0x84B220): `fld fdiv fstp` -> f32 `[0xA9A7FC]`.
#[hook(addr = 0x84B220, conv = cdecl, name = "__init_g_a9a7fc", expect = "D9 05 28 86 85 00")]
pub fn init_g_a9a7fc() {
    g::<f32>(0xA9A7FC).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_a9a800` (0x84B240): `fld fmul fdivr fstp` -> f32 `[0xA9A800]`.
#[hook(addr = 0x84B240, conv = cdecl, name = "__init_g_a9a800", expect = "D9 05 D8 A4 85 00")]
pub fn init_g_a9a800() {
    g::<f32>(0xA9A800).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_a9a804` (0x84B260): `fld fmul fstp` -> f32 `[0xA9A804]`.
#[hook(addr = 0x84B260, conv = cdecl, name = "__init_g_a9a804", expect = "D9 05 00 A8 A9 00")]
pub fn init_g_a9a804() {
    g::<f32>(0xA9A804).write(x(g::<f32>(0xA9A800).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_a9a808` (0x84B280): `fld fdiv fstp` -> f32 `[0xA9A808]`.
#[hook(addr = 0x84B280, conv = cdecl, name = "__init_g_a9a808", expect = "D9 05 30 86 85 00")]
pub fn init_g_a9a808() {
    g::<f32>(0xA9A808).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_b717ec` (0x84DAB0): `fld fmul fstp` -> f32 `[0xB717EC]`.
#[hook(addr = 0x84DAB0, conv = cdecl, name = "__init_g_b717ec", expect = "D9 05 B0 38 86 00")]
pub fn init_g_b717ec() {
    g::<f32>(0xB717EC).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_b717f0` (0x84DAD0): `fld fdiv fstp` -> f32 `[0xB717F0]`.
#[hook(addr = 0x84DAD0, conv = cdecl, name = "__init_g_b717f0", expect = "D9 05 24 86 85 00")]
pub fn init_g_b717f0() {
    g::<f32>(0xB717F0).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_b717f4` (0x84DAF0): `fld fdiv fstp` -> f32 `[0xB717F4]`.
#[hook(addr = 0x84DAF0, conv = cdecl, name = "__init_g_b717f4", expect = "D9 05 28 86 85 00")]
pub fn init_g_b717f4() {
    g::<f32>(0xB717F4).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_b717f8` (0x84DB10): `fld fmul fdivr fstp` -> f32 `[0xB717F8]`.
#[hook(addr = 0x84DB10, conv = cdecl, name = "__init_g_b717f8", expect = "D9 05 B8 38 86 00")]
pub fn init_g_b717f8() {
    g::<f32>(0xB717F8).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_b717fc` (0x84DB30): `fld fmul fstp` -> f32 `[0xB717FC]`.
#[hook(addr = 0x84DB30, conv = cdecl, name = "__init_g_b717fc", expect = "D9 05 F8 17 B7 00")]
pub fn init_g_b717fc() {
    g::<f32>(0xB717FC).write(x(g::<f32>(0xB717F8).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_b71800` (0x84DB50): `fld fdiv fstp` -> f32 `[0xB71800]`.
#[hook(addr = 0x84DB50, conv = cdecl, name = "__init_g_b71800", expect = "D9 05 30 86 85 00")]
pub fn init_g_b71800() {
    g::<f32>(0xB71800).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_b7689c` (0x84E560): `fld fmul fstp` -> f32 `[0xB7689C]`.
#[hook(addr = 0x84E560, conv = cdecl, name = "__init_g_b7689c", expect = "D9 05 1C 3E 86 00")]
pub fn init_g_b7689c() {
    g::<f32>(0xB7689C).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_b768a0` (0x84E580): `fld fdiv fstp` -> f32 `[0xB768A0]`.
#[hook(addr = 0x84E580, conv = cdecl, name = "__init_g_b768a0", expect = "D9 05 24 86 85 00")]
pub fn init_g_b768a0() {
    g::<f32>(0xB768A0).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_b768a4` (0x84E5A0): `fld fdiv fstp` -> f32 `[0xB768A4]`.
#[hook(addr = 0x84E5A0, conv = cdecl, name = "__init_g_b768a4", expect = "D9 05 28 86 85 00")]
pub fn init_g_b768a4() {
    g::<f32>(0xB768A4).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_b768a8` (0x84E5C0): `fld fmul fdivr fstp` -> f32 `[0xB768A8]`.
#[hook(addr = 0x84E5C0, conv = cdecl, name = "__init_g_b768a8", expect = "D9 05 24 3E 86 00")]
pub fn init_g_b768a8() {
    g::<f32>(0xB768A8).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_b768ac` (0x84E5E0): `fld fmul fstp` -> f32 `[0xB768AC]`.
#[hook(addr = 0x84E5E0, conv = cdecl, name = "__init_g_b768ac", expect = "D9 05 A8 68 B7 00")]
pub fn init_g_b768ac() {
    g::<f32>(0xB768AC).write(x(g::<f32>(0xB768A8).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_b768b0` (0x84E600): `fld fdiv fstp` -> f32 `[0xB768B0]`.
#[hook(addr = 0x84E600, conv = cdecl, name = "__init_g_b768b0", expect = "D9 05 30 86 85 00")]
pub fn init_g_b768b0() {
    g::<f32>(0xB768B0).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_b7cb90` (0x84E800): `fld fmul fstp` -> f32 `[0xB7CB90]`.
#[hook(addr = 0x84E800, conv = cdecl, name = "__init_g_b7cb90", expect = "D9 05 24 4E 86 00")]
pub fn init_g_b7cb90() {
    g::<f32>(0xB7CB90).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_b7cb94` (0x84E820): `fld fdiv fstp` -> f32 `[0xB7CB94]`.
#[hook(addr = 0x84E820, conv = cdecl, name = "__init_g_b7cb94", expect = "D9 05 24 86 85 00")]
pub fn init_g_b7cb94() {
    g::<f32>(0xB7CB94).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_b7cb98` (0x84E840): `fld fdiv fstp` -> f32 `[0xB7CB98]`.
#[hook(addr = 0x84E840, conv = cdecl, name = "__init_g_b7cb98", expect = "D9 05 28 86 85 00")]
pub fn init_g_b7cb98() {
    g::<f32>(0xB7CB98).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_b7cb9c` (0x84E860): `fld fmul fdivr fstp` -> f32 `[0xB7CB9C]`.
#[hook(addr = 0x84E860, conv = cdecl, name = "__init_g_b7cb9c", expect = "D9 05 2C 4E 86 00")]
pub fn init_g_b7cb9c() {
    g::<f32>(0xB7CB9C).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_b7cba0` (0x84E880): `fld fmul fstp` -> f32 `[0xB7CBA0]`.
#[hook(addr = 0x84E880, conv = cdecl, name = "__init_g_b7cba0", expect = "D9 05 9C CB B7 00")]
pub fn init_g_b7cba0() {
    g::<f32>(0xB7CBA0).write(x(g::<f32>(0xB7CB9C).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_b7cba4` (0x84E8A0): `fld fdiv fstp` -> f32 `[0xB7CBA4]`.
#[hook(addr = 0x84E8A0, conv = cdecl, name = "__init_g_b7cba4", expect = "D9 05 30 86 85 00")]
pub fn init_g_b7cba4() {
    g::<f32>(0xB7CBA4).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_b7cd80` (0x84E8C0): `fld fmul fstp` -> f32 `[0xB7CD80]`.
#[hook(addr = 0x84E8C0, conv = cdecl, name = "__init_g_b7cd80", expect = "D9 05 8C 4E 86 00")]
pub fn init_g_b7cd80() {
    g::<f32>(0xB7CD80).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_b7cd84` (0x84E8E0): `fld fdiv fstp` -> f32 `[0xB7CD84]`.
#[hook(addr = 0x84E8E0, conv = cdecl, name = "__init_g_b7cd84", expect = "D9 05 24 86 85 00")]
pub fn init_g_b7cd84() {
    g::<f32>(0xB7CD84).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_b7cd88` (0x84E900): `fld fdiv fstp` -> f32 `[0xB7CD88]`.
#[hook(addr = 0x84E900, conv = cdecl, name = "__init_g_b7cd88", expect = "D9 05 28 86 85 00")]
pub fn init_g_b7cd88() {
    g::<f32>(0xB7CD88).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_b7cd8c` (0x84E920): `fld fmul fdivr fstp` -> f32 `[0xB7CD8C]`.
#[hook(addr = 0x84E920, conv = cdecl, name = "__init_g_b7cd8c", expect = "D9 05 94 4E 86 00")]
pub fn init_g_b7cd8c() {
    g::<f32>(0xB7CD8C).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_b7cd90` (0x84E940): `fld fmul fstp` -> f32 `[0xB7CD90]`.
#[hook(addr = 0x84E940, conv = cdecl, name = "__init_g_b7cd90", expect = "D9 05 8C CD B7 00")]
pub fn init_g_b7cd90() {
    g::<f32>(0xB7CD90).write(x(g::<f32>(0xB7CD8C).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_b7cd94` (0x84E960): `fld fdiv fstp` -> f32 `[0xB7CD94]`.
#[hook(addr = 0x84E960, conv = cdecl, name = "__init_g_b7cd94", expect = "D9 05 30 86 85 00")]
pub fn init_g_b7cd94() {
    g::<f32>(0xB7CD94).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_ba18dc` (0x84ECA0): `fld fmul fstp` -> f32 `[0xBA18DC]`.
#[hook(addr = 0x84ECA0, conv = cdecl, name = "__init_g_ba18dc", expect = "D9 05 50 50 86 00")]
pub fn init_g_ba18dc() {
    g::<f32>(0xBA18DC).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_ba18e0` (0x84ECC0): `fld fdiv fstp` -> f32 `[0xBA18E0]`.
#[hook(addr = 0x84ECC0, conv = cdecl, name = "__init_g_ba18e0", expect = "D9 05 24 86 85 00")]
pub fn init_g_ba18e0() {
    g::<f32>(0xBA18E0).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_ba18e4` (0x84ECE0): `fld fdiv fstp` -> f32 `[0xBA18E4]`.
#[hook(addr = 0x84ECE0, conv = cdecl, name = "__init_g_ba18e4", expect = "D9 05 28 86 85 00")]
pub fn init_g_ba18e4() {
    g::<f32>(0xBA18E4).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_ba18e8` (0x84ED00): `fld fmul fdivr fstp` -> f32 `[0xBA18E8]`.
#[hook(addr = 0x84ED00, conv = cdecl, name = "__init_g_ba18e8", expect = "D9 05 58 50 86 00")]
pub fn init_g_ba18e8() {
    g::<f32>(0xBA18E8).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_ba18ec` (0x84ED20): `fld fmul fstp` -> f32 `[0xBA18EC]`.
#[hook(addr = 0x84ED20, conv = cdecl, name = "__init_g_ba18ec", expect = "D9 05 E8 18 BA 00")]
pub fn init_g_ba18ec() {
    g::<f32>(0xBA18EC).write(x(g::<f32>(0xBA18E8).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_ba18f0` (0x84ED40): `fld fdiv fstp` -> f32 `[0xBA18F0]`.
#[hook(addr = 0x84ED40, conv = cdecl, name = "__init_g_ba18f0", expect = "D9 05 30 86 85 00")]
pub fn init_g_ba18f0() {
    g::<f32>(0xBA18F0).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_bab344` (0x84F210): `fld fmul fstp` -> f32 `[0xBAB344]`.
#[hook(addr = 0x84F210, conv = cdecl, name = "__init_g_bab344", expect = "D9 05 B8 6C 86 00")]
pub fn init_g_bab344() {
    g::<f32>(0xBAB344).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_bab348` (0x84F230): `fld fdiv fstp` -> f32 `[0xBAB348]`.
#[hook(addr = 0x84F230, conv = cdecl, name = "__init_g_bab348", expect = "D9 05 24 86 85 00")]
pub fn init_g_bab348() {
    g::<f32>(0xBAB348).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_bab34c` (0x84F250): `fld fdiv fstp` -> f32 `[0xBAB34C]`.
#[hook(addr = 0x84F250, conv = cdecl, name = "__init_g_bab34c", expect = "D9 05 28 86 85 00")]
pub fn init_g_bab34c() {
    g::<f32>(0xBAB34C).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_bab350` (0x84F270): `fld fmul fdivr fstp` -> f32 `[0xBAB350]`.
#[hook(addr = 0x84F270, conv = cdecl, name = "__init_g_bab350", expect = "D9 05 C0 6C 86 00")]
pub fn init_g_bab350() {
    g::<f32>(0xBAB350).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_bab354` (0x84F290): `fld fmul fstp` -> f32 `[0xBAB354]`.
#[hook(addr = 0x84F290, conv = cdecl, name = "__init_g_bab354", expect = "D9 05 50 B3 BA 00")]
pub fn init_g_bab354() {
    g::<f32>(0xBAB354).write(x(g::<f32>(0xBAB350).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_baf5f8` (0x84F380): `fld fmul fstp` -> f32 `[0xBAF5F8]`.
#[hook(addr = 0x84F380, conv = cdecl, name = "__init_g_baf5f8", expect = "D9 05 08 6D 86 00")]
pub fn init_g_baf5f8() {
    g::<f32>(0xBAF5F8).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_baf5fc` (0x84F3A0): `fld fdiv fstp` -> f32 `[0xBAF5FC]`.
#[hook(addr = 0x84F3A0, conv = cdecl, name = "__init_g_baf5fc", expect = "D9 05 24 86 85 00")]
pub fn init_g_baf5fc() {
    g::<f32>(0xBAF5FC).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_baf600` (0x84F3C0): `fld fdiv fstp` -> f32 `[0xBAF600]`.
#[hook(addr = 0x84F3C0, conv = cdecl, name = "__init_g_baf600", expect = "D9 05 28 86 85 00")]
pub fn init_g_baf600() {
    g::<f32>(0xBAF600).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_baf604` (0x84F3E0): `fld fmul fdivr fstp` -> f32 `[0xBAF604]`.
#[hook(addr = 0x84F3E0, conv = cdecl, name = "__init_g_baf604", expect = "D9 05 10 6D 86 00")]
pub fn init_g_baf604() {
    g::<f32>(0xBAF604).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_baf608` (0x84F400): `fld fmul fstp` -> f32 `[0xBAF608]`.
#[hook(addr = 0x84F400, conv = cdecl, name = "__init_g_baf608", expect = "D9 05 04 F6 BA 00")]
pub fn init_g_baf608() {
    g::<f32>(0xBAF608).write(x(g::<f32>(0xBAF604).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_baf60c` (0x84F420): `fld fdiv fstp` -> f32 `[0xBAF60C]`.
#[hook(addr = 0x84F420, conv = cdecl, name = "__init_g_baf60c", expect = "D9 05 30 86 85 00")]
pub fn init_g_baf60c() {
    g::<f32>(0xBAF60C).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_baf610` (0x84F440): `fld fmul fstp` -> f32 `[0xBAF610]`.
#[hook(addr = 0x84F440, conv = cdecl, name = "__init_g_baf610", expect = "D9 05 18 6D 86 00")]
pub fn init_g_baf610() {
    g::<f32>(0xBAF610).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_baf614` (0x84F460): `fld fdiv fstp` -> f32 `[0xBAF614]`.
#[hook(addr = 0x84F460, conv = cdecl, name = "__init_g_baf614", expect = "D9 05 24 86 85 00")]
pub fn init_g_baf614() {
    g::<f32>(0xBAF614).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_baf618` (0x84F480): `fld fdiv fstp` -> f32 `[0xBAF618]`.
#[hook(addr = 0x84F480, conv = cdecl, name = "__init_g_baf618", expect = "D9 05 28 86 85 00")]
pub fn init_g_baf618() {
    g::<f32>(0xBAF618).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_baf61c` (0x84F4A0): `fld fmul fdivr fstp` -> f32 `[0xBAF61C]`.
#[hook(addr = 0x84F4A0, conv = cdecl, name = "__init_g_baf61c", expect = "D9 05 20 6D 86 00")]
pub fn init_g_baf61c() {
    g::<f32>(0xBAF61C).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_baf620` (0x84F4C0): `fld fmul fstp` -> f32 `[0xBAF620]`.
#[hook(addr = 0x84F4C0, conv = cdecl, name = "__init_g_baf620", expect = "D9 05 1C F6 BA 00")]
pub fn init_g_baf620() {
    g::<f32>(0xBAF620).write(x(g::<f32>(0xBAF61C).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_baf624` (0x84F4E0): `fld fdiv fstp` -> f32 `[0xBAF624]`.
#[hook(addr = 0x84F4E0, conv = cdecl, name = "__init_g_baf624", expect = "D9 05 30 86 85 00")]
pub fn init_g_baf624() {
    g::<f32>(0xBAF624).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_baf628` (0x84F500): `fld fmul fstp` -> f32 `[0xBAF628]`.
#[hook(addr = 0x84F500, conv = cdecl, name = "__init_g_baf628", expect = "D9 05 54 6D 86 00")]
pub fn init_g_baf628() {
    g::<f32>(0xBAF628).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_baf62c` (0x84F520): `fld fdiv fstp` -> f32 `[0xBAF62C]`.
#[hook(addr = 0x84F520, conv = cdecl, name = "__init_g_baf62c", expect = "D9 05 24 86 85 00")]
pub fn init_g_baf62c() {
    g::<f32>(0xBAF62C).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_baf630` (0x84F540): `fld fdiv fstp` -> f32 `[0xBAF630]`.
#[hook(addr = 0x84F540, conv = cdecl, name = "__init_g_baf630", expect = "D9 05 28 86 85 00")]
pub fn init_g_baf630() {
    g::<f32>(0xBAF630).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_baf634` (0x84F560): `fld fmul fdivr fstp` -> f32 `[0xBAF634]`.
#[hook(addr = 0x84F560, conv = cdecl, name = "__init_g_baf634", expect = "D9 05 5C 6D 86 00")]
pub fn init_g_baf634() {
    g::<f32>(0xBAF634).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_baf638` (0x84F580): `fld fmul fstp` -> f32 `[0xBAF638]`.
#[hook(addr = 0x84F580, conv = cdecl, name = "__init_g_baf638", expect = "D9 05 34 F6 BA 00")]
pub fn init_g_baf638() {
    g::<f32>(0xBAF638).write(x(g::<f32>(0xBAF634).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_baf63c` (0x84F5A0): `fld fdiv fstp` -> f32 `[0xBAF63C]`.
#[hook(addr = 0x84F5A0, conv = cdecl, name = "__init_g_baf63c", expect = "D9 05 30 86 85 00")]
pub fn init_g_baf63c() {
    g::<f32>(0xBAF63C).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_baf640` (0x84F5C0): `fld fmul fstp` -> f32 `[0xBAF640]`.
#[hook(addr = 0x84F5C0, conv = cdecl, name = "__init_g_baf640", expect = "D9 05 60 6D 86 00")]
pub fn init_g_baf640() {
    g::<f32>(0xBAF640).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_baf644` (0x84F5E0): `fld fdiv fstp` -> f32 `[0xBAF644]`.
#[hook(addr = 0x84F5E0, conv = cdecl, name = "__init_g_baf644", expect = "D9 05 24 86 85 00")]
pub fn init_g_baf644() {
    g::<f32>(0xBAF644).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_baf648` (0x84F600): `fld fdiv fstp` -> f32 `[0xBAF648]`.
#[hook(addr = 0x84F600, conv = cdecl, name = "__init_g_baf648", expect = "D9 05 28 86 85 00")]
pub fn init_g_baf648() {
    g::<f32>(0xBAF648).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_baf64c` (0x84F620): `fld fmul fdivr fstp` -> f32 `[0xBAF64C]`.
#[hook(addr = 0x84F620, conv = cdecl, name = "__init_g_baf64c", expect = "D9 05 68 6D 86 00")]
pub fn init_g_baf64c() {
    g::<f32>(0xBAF64C).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_baf650` (0x84F640): `fld fmul fstp` -> f32 `[0xBAF650]`.
#[hook(addr = 0x84F640, conv = cdecl, name = "__init_g_baf650", expect = "D9 05 4C F6 BA 00")]
pub fn init_g_baf650() {
    g::<f32>(0xBAF650).write(x(g::<f32>(0xBAF64C).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_baf654` (0x84F660): `fld fdiv fstp` -> f32 `[0xBAF654]`.
#[hook(addr = 0x84F660, conv = cdecl, name = "__init_g_baf654", expect = "D9 05 30 86 85 00")]
pub fn init_g_baf654() {
    g::<f32>(0xBAF654).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_baf658` (0x84F680): `fld fmul fstp` -> f32 `[0xBAF658]`.
#[hook(addr = 0x84F680, conv = cdecl, name = "__init_g_baf658", expect = "D9 05 6C 6D 86 00")]
pub fn init_g_baf658() {
    g::<f32>(0xBAF658).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_baf65c` (0x84F6A0): `fld fdiv fstp` -> f32 `[0xBAF65C]`.
#[hook(addr = 0x84F6A0, conv = cdecl, name = "__init_g_baf65c", expect = "D9 05 24 86 85 00")]
pub fn init_g_baf65c() {
    g::<f32>(0xBAF65C).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_baf660` (0x84F6C0): `fld fdiv fstp` -> f32 `[0xBAF660]`.
#[hook(addr = 0x84F6C0, conv = cdecl, name = "__init_g_baf660", expect = "D9 05 28 86 85 00")]
pub fn init_g_baf660() {
    g::<f32>(0xBAF660).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_baf664` (0x84F6E0): `fld fmul fdivr fstp` -> f32 `[0xBAF664]`.
#[hook(addr = 0x84F6E0, conv = cdecl, name = "__init_g_baf664", expect = "D9 05 74 6D 86 00")]
pub fn init_g_baf664() {
    g::<f32>(0xBAF664).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_baf668` (0x84F700): `fld fmul fstp` -> f32 `[0xBAF668]`.
#[hook(addr = 0x84F700, conv = cdecl, name = "__init_g_baf668", expect = "D9 05 64 F6 BA 00")]
pub fn init_g_baf668() {
    g::<f32>(0xBAF668).write(x(g::<f32>(0xBAF664).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_baf66c` (0x84F720): `fld fdiv fstp` -> f32 `[0xBAF66C]`.
#[hook(addr = 0x84F720, conv = cdecl, name = "__init_g_baf66c", expect = "D9 05 30 86 85 00")]
pub fn init_g_baf66c() {
    g::<f32>(0xBAF66C).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_bb3dcc` (0x84F760): `fld fmul fstp` -> f32 `[0xBB3DCC]`.
#[hook(addr = 0x84F760, conv = cdecl, name = "__init_g_bb3dcc", expect = "D9 05 78 6D 86 00")]
pub fn init_g_bb3dcc() {
    g::<f32>(0xBB3DCC).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_bb3dd0` (0x84F780): `fld fdiv fstp` -> f32 `[0xBB3DD0]`.
#[hook(addr = 0x84F780, conv = cdecl, name = "__init_g_bb3dd0", expect = "D9 05 24 86 85 00")]
pub fn init_g_bb3dd0() {
    g::<f32>(0xBB3DD0).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_bb3dd4` (0x84F7A0): `fld fdiv fstp` -> f32 `[0xBB3DD4]`.
#[hook(addr = 0x84F7A0, conv = cdecl, name = "__init_g_bb3dd4", expect = "D9 05 28 86 85 00")]
pub fn init_g_bb3dd4() {
    g::<f32>(0xBB3DD4).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_bb3dd8` (0x84F7C0): `fld fmul fdivr fstp` -> f32 `[0xBB3DD8]`.
#[hook(addr = 0x84F7C0, conv = cdecl, name = "__init_g_bb3dd8", expect = "D9 05 80 6D 86 00")]
pub fn init_g_bb3dd8() {
    g::<f32>(0xBB3DD8).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_bb3ddc` (0x84F7E0): `fld fmul fstp` -> f32 `[0xBB3DDC]`.
#[hook(addr = 0x84F7E0, conv = cdecl, name = "__init_g_bb3ddc", expect = "D9 05 D8 3D BB 00")]
pub fn init_g_bb3ddc() {
    g::<f32>(0xBB3DDC).write(x(g::<f32>(0xBB3DD8).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_bb3de0` (0x84F800): `fld fdiv fstp` -> f32 `[0xBB3DE0]`.
#[hook(addr = 0x84F800, conv = cdecl, name = "__init_g_bb3de0", expect = "D9 05 30 86 85 00")]
pub fn init_g_bb3de0() {
    g::<f32>(0xBB3DE0).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_bb4200` (0x84F8E0): `fld fmul fstp` -> f32 `[0xBB4200]`.
#[hook(addr = 0x84F8E0, conv = cdecl, name = "__init_g_bb4200", expect = "D9 05 A4 6D 86 00")]
pub fn init_g_bb4200() {
    g::<f32>(0xBB4200).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_bb4204` (0x84F900): `fld fdiv fstp` -> f32 `[0xBB4204]`.
#[hook(addr = 0x84F900, conv = cdecl, name = "__init_g_bb4204", expect = "D9 05 24 86 85 00")]
pub fn init_g_bb4204() {
    g::<f32>(0xBB4204).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_bb4208` (0x84F920): `fld fdiv fstp` -> f32 `[0xBB4208]`.
#[hook(addr = 0x84F920, conv = cdecl, name = "__init_g_bb4208", expect = "D9 05 28 86 85 00")]
pub fn init_g_bb4208() {
    g::<f32>(0xBB4208).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_bb420c` (0x84F940): `fld fmul fdivr fstp` -> f32 `[0xBB420C]`.
#[hook(addr = 0x84F940, conv = cdecl, name = "__init_g_bb420c", expect = "D9 05 AC 6D 86 00")]
pub fn init_g_bb420c() {
    g::<f32>(0xBB420C).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_bb4210` (0x84F960): `fld fmul fstp` -> f32 `[0xBB4210]`.
#[hook(addr = 0x84F960, conv = cdecl, name = "__init_g_bb4210", expect = "D9 05 0C 42 BB 00")]
pub fn init_g_bb4210() {
    g::<f32>(0xBB4210).write(x(g::<f32>(0xBB420C).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_bb4214` (0x84F980): `fld fdiv fstp` -> f32 `[0xBB4214]`.
#[hook(addr = 0x84F980, conv = cdecl, name = "__init_g_bb4214", expect = "D9 05 30 86 85 00")]
pub fn init_g_bb4214() {
    g::<f32>(0xBB4214).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_bb4a74` (0x84FC20): `fld fmul fstp` -> f32 `[0xBB4A74]`.
#[hook(addr = 0x84FC20, conv = cdecl, name = "__init_g_bb4a74", expect = "D9 05 3C 6F 86 00")]
pub fn init_g_bb4a74() {
    g::<f32>(0xBB4A74).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_bb4a78` (0x84FC40): `fld fdiv fstp` -> f32 `[0xBB4A78]`.
#[hook(addr = 0x84FC40, conv = cdecl, name = "__init_g_bb4a78", expect = "D9 05 24 86 85 00")]
pub fn init_g_bb4a78() {
    g::<f32>(0xBB4A78).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_bb4a7c` (0x84FC60): `fld fdiv fstp` -> f32 `[0xBB4A7C]`.
#[hook(addr = 0x84FC60, conv = cdecl, name = "__init_g_bb4a7c", expect = "D9 05 28 86 85 00")]
pub fn init_g_bb4a7c() {
    g::<f32>(0xBB4A7C).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_bb4a80` (0x84FC80): `fld fmul fdivr fstp` -> f32 `[0xBB4A80]`.
#[hook(addr = 0x84FC80, conv = cdecl, name = "__init_g_bb4a80", expect = "D9 05 44 6F 86 00")]
pub fn init_g_bb4a80() {
    g::<f32>(0xBB4A80).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_bb4a84` (0x84FCA0): `fld fmul fstp` -> f32 `[0xBB4A84]`.
#[hook(addr = 0x84FCA0, conv = cdecl, name = "__init_g_bb4a84", expect = "D9 05 80 4A BB 00")]
pub fn init_g_bb4a84() {
    g::<f32>(0xBB4A84).write(x(g::<f32>(0xBB4A80).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_bb4a88` (0x84FCC0): `fld fdiv fstp` -> f32 `[0xBB4A88]`.
#[hook(addr = 0x84FCC0, conv = cdecl, name = "__init_g_bb4a88", expect = "D9 05 30 86 85 00")]
pub fn init_g_bb4a88() {
    g::<f32>(0xBB4A88).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_c40424` (0x854A00): `fld fmul fdivr fstp` -> f32 `[0xC40424]`.
#[hook(addr = 0x854A00, conv = cdecl, name = "__init_g_c40424", expect = "D9 05 7C 26 87 00")]
pub fn init_g_c40424() {
    g::<f32>(0xC40424).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

orig! {
    /// `FurnitureManager_c::FurnitureManager_c` (0x591260)
    pub fn furniture_manager_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x591260 => unported;
    /// `InteriorManager_c::InteriorManager_c` (0x5984C0)
    pub fn interior_manager_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x5984C0 => unported;
    /// `BreakManager_c::BreakManager_c` (0x59E620)
    pub fn break_manager_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x59E620 => unported;
    /// `ProcObjectMan_c::ProcObjectMan_c` (0x5A3E20)
    pub fn proc_object_man_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x5A3E20 => unported;
    /// `WaterCreatureManager_c::WaterCreatureManager_c` (0x6E3EB0)
    pub fn water_creature_manager_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x6E3EB0 => unported;
}

/// `__init_g_furnitureMan` (0x84F300): constructor on 0xBAB380, `atexit(0x856640)`.
#[hook(addr = 0x84F300, conv = cdecl, name = "__init_g_furnitureMan", expect = "B9 80 B3 BA 00")]
pub fn init_g_furniture_man() {
    furniture_manager_ctor(g(0xBAB380));
    atexit(0x856640);
}

/// `__init_g_interiorMan` (0x84F740): constructor on 0xBAF670, `atexit(0x856650)`.
#[hook(addr = 0x84F740, conv = cdecl, name = "__init_g_interiorMan", expect = "B9 70 F6 BA 00")]
pub fn init_g_interior_man() {
    interior_manager_ctor(g(0xBAF670));
    atexit(0x856650);
}

/// `__init_g_breakMan` (0x84FA80): constructor on 0xBB4240, `atexit(0x856660)`.
#[hook(addr = 0x84FA80, conv = cdecl, name = "__init_g_breakMan", expect = "B9 40 42 BB 00")]
pub fn init_g_break_man() {
    break_manager_ctor(g(0xBB4240));
    atexit(0x856660);
}

/// `__init_g_procObjMan` (0x84FDC0): constructor on 0xBB7CB0, `atexit(0x856670)`.
#[hook(addr = 0x84FDC0, conv = cdecl, name = "__init_g_procObjMan", expect = "B9 B0 7C BB 00")]
pub fn init_g_proc_obj_man() {
    proc_object_man_ctor(g(0xBB7CB0));
    atexit(0x856670);
}

/// `__init_g_waterCreatureMan` (0x853F10): constructor on 0xC1DF30, `atexit(0x856A80)`.
#[hook(addr = 0x853F10, conv = cdecl, name = "__init_g_waterCreatureMan", expect = "B9 30 DF C1 00")]
pub fn init_g_water_creature_man() {
    water_creature_manager_ctor(g(0xC1DF30));
    atexit(0x856A80);
}

/// `__init_g_bae1f8` (0x84F350): 256 `Furniture_c` (0x14) at 0xBAE1F8 through the CRT
/// `eh vector constructor iterator` (ctor 0x590EC0, dtor 0x590ED0), `atexit(0x856620)`.
#[hook(addr = 0x84F350, conv = cdecl, name = "__init_g_bae1f8", expect = "68 D0 0E 59 00")]
pub fn init_g_bae1f8() {
    eh_vector_ctor(g(0xBAE1F8), 0x14, 0x100, 0x590EC0, 0x590ED0);
    atexit(0x856620);
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_G_9690D0,
    HOOK_INIT_G_9690D4,
    HOOK_INIT_G_9690D8,
    HOOK_INIT_G_9690DC,
    HOOK_INIT_G_9690E0,
    HOOK_INIT_G_9690E4,
    HOOK_INIT_G_96A708,
    HOOK_INIT_G_96A70C,
    HOOK_INIT_G_96A710,
    HOOK_INIT_G_96A714,
    HOOK_INIT_G_96A7E4,
    HOOK_INIT_G_96A7E8,
    HOOK_INIT_G_96A7EC,
    HOOK_INIT_G_96A7F0,
    HOOK_INIT_G_96A7F4,
    HOOK_INIT_G_96A7F8,
    HOOK_INIT_G_96ABA4,
    HOOK_INIT_G_96ABA8,
    HOOK_INIT_G_96ABAC,
    HOOK_INIT_G_96ABB0,
    HOOK_INIT_G_96ABB4,
    HOOK_INIT_G_96ABB8,
    HOOK_INIT_G_96C02C,
    HOOK_INIT_G_96C030,
    HOOK_INIT_G_96C034,
    HOOK_INIT_G_96C038,
    HOOK_INIT_G_96C03C,
    HOOK_INIT_G_96C040,
    HOOK_INIT_G_977BA0,
    HOOK_INIT_G_977BA4,
    HOOK_INIT_G_977BA8,
    HOOK_INIT_G_977BAC,
    HOOK_INIT_G_977BB0,
    HOOK_INIT_G_977BB4,
    HOOK_INIT_G_977BCC,
    HOOK_INIT_G_977BD0,
    HOOK_INIT_G_977BD4,
    HOOK_INIT_G_977BD8,
    HOOK_INIT_G_977BDC,
    HOOK_INIT_G_977BE0,
    HOOK_INIT_G_97F64C,
    HOOK_INIT_G_97F650,
    HOOK_INIT_G_97F654,
    HOOK_INIT_G_97F658,
    HOOK_INIT_G_97F65C,
    HOOK_INIT_G_97F660,
    HOOK_INIT_G_A43098,
    HOOK_INIT_G_A4309C,
    HOOK_INIT_G_A430A0,
    HOOK_INIT_G_A430A4,
    HOOK_INIT_G_A430A8,
    HOOK_INIT_G_A430AC,
    HOOK_INIT_G_A43310,
    HOOK_INIT_G_A95160,
    HOOK_INIT_G_A95164,
    HOOK_INIT_G_A95168,
    HOOK_INIT_G_A9516C,
    HOOK_INIT_G_A95170,
    HOOK_INIT_G_A95174,
    HOOK_INIT_G_A95194,
    HOOK_INIT_G_A95198,
    HOOK_INIT_G_A9519C,
    HOOK_INIT_G_A951A0,
    HOOK_INIT_G_A951A4,
    HOOK_INIT_G_A951A8,
    HOOK_INIT_G_A951AC,
    HOOK_INIT_G_A951B0,
    HOOK_INIT_G_A951B4,
    HOOK_INIT_G_A951B8,
    HOOK_INIT_G_A951BC,
    HOOK_INIT_G_A951C0,
    HOOK_INIT_G_A951C4,
    HOOK_INIT_G_A951C8,
    HOOK_INIT_G_A951CC,
    HOOK_INIT_G_A951D0,
    HOOK_INIT_G_A951D4,
    HOOK_INIT_G_A951D8,
    HOOK_INIT_G_A9577C,
    HOOK_INIT_G_A95780,
    HOOK_INIT_G_A95784,
    HOOK_INIT_G_A95788,
    HOOK_INIT_G_A9578C,
    HOOK_INIT_G_A95790,
    HOOK_INIT_G_A95794,
    HOOK_INIT_G_A95798,
    HOOK_INIT_G_A9579C,
    HOOK_INIT_G_A957A0,
    HOOK_INIT_G_A957A4,
    HOOK_INIT_G_A957A8,
    HOOK_INIT_G_A957AC,
    HOOK_INIT_G_A957B0,
    HOOK_INIT_G_A957B4,
    HOOK_INIT_G_A957B8,
    HOOK_INIT_G_A957BC,
    HOOK_INIT_G_A957C0,
    HOOK_INIT_G_A957C4,
    HOOK_INIT_G_A957C8,
    HOOK_INIT_G_A957CC,
    HOOK_INIT_G_A957D0,
    HOOK_INIT_G_A957D4,
    HOOK_INIT_G_A957D8,
    HOOK_INIT_G_A957DC,
    HOOK_INIT_G_A957E0,
    HOOK_INIT_G_A957E4,
    HOOK_INIT_G_A957E8,
    HOOK_INIT_G_A957EC,
    HOOK_INIT_G_A957F0,
    HOOK_INIT_G_A957F4,
    HOOK_INIT_G_A95800,
    HOOK_INIT_G_A95804,
    HOOK_INIT_G_A95808,
    HOOK_INIT_G_A9580C,
    HOOK_INIT_G_A95810,
    HOOK_INIT_G_A95814,
    HOOK_INIT_G_A9A7F4,
    HOOK_INIT_G_A9A7F8,
    HOOK_INIT_G_A9A7FC,
    HOOK_INIT_G_A9A800,
    HOOK_INIT_G_A9A804,
    HOOK_INIT_G_A9A808,
    HOOK_INIT_G_B717EC,
    HOOK_INIT_G_B717F0,
    HOOK_INIT_G_B717F4,
    HOOK_INIT_G_B717F8,
    HOOK_INIT_G_B717FC,
    HOOK_INIT_G_B71800,
    HOOK_INIT_G_B7689C,
    HOOK_INIT_G_B768A0,
    HOOK_INIT_G_B768A4,
    HOOK_INIT_G_B768A8,
    HOOK_INIT_G_B768AC,
    HOOK_INIT_G_B768B0,
    HOOK_INIT_G_B7CB90,
    HOOK_INIT_G_B7CB94,
    HOOK_INIT_G_B7CB98,
    HOOK_INIT_G_B7CB9C,
    HOOK_INIT_G_B7CBA0,
    HOOK_INIT_G_B7CBA4,
    HOOK_INIT_G_B7CD80,
    HOOK_INIT_G_B7CD84,
    HOOK_INIT_G_B7CD88,
    HOOK_INIT_G_B7CD8C,
    HOOK_INIT_G_B7CD90,
    HOOK_INIT_G_B7CD94,
    HOOK_INIT_G_BA18DC,
    HOOK_INIT_G_BA18E0,
    HOOK_INIT_G_BA18E4,
    HOOK_INIT_G_BA18E8,
    HOOK_INIT_G_BA18EC,
    HOOK_INIT_G_BA18F0,
    HOOK_INIT_G_BAB344,
    HOOK_INIT_G_BAB348,
    HOOK_INIT_G_BAB34C,
    HOOK_INIT_G_BAB350,
    HOOK_INIT_G_BAB354,
    HOOK_INIT_G_BAF5F8,
    HOOK_INIT_G_BAF5FC,
    HOOK_INIT_G_BAF600,
    HOOK_INIT_G_BAF604,
    HOOK_INIT_G_BAF608,
    HOOK_INIT_G_BAF60C,
    HOOK_INIT_G_BAF610,
    HOOK_INIT_G_BAF614,
    HOOK_INIT_G_BAF618,
    HOOK_INIT_G_BAF61C,
    HOOK_INIT_G_BAF620,
    HOOK_INIT_G_BAF624,
    HOOK_INIT_G_BAF628,
    HOOK_INIT_G_BAF62C,
    HOOK_INIT_G_BAF630,
    HOOK_INIT_G_BAF634,
    HOOK_INIT_G_BAF638,
    HOOK_INIT_G_BAF63C,
    HOOK_INIT_G_BAF640,
    HOOK_INIT_G_BAF644,
    HOOK_INIT_G_BAF648,
    HOOK_INIT_G_BAF64C,
    HOOK_INIT_G_BAF650,
    HOOK_INIT_G_BAF654,
    HOOK_INIT_G_BAF658,
    HOOK_INIT_G_BAF65C,
    HOOK_INIT_G_BAF660,
    HOOK_INIT_G_BAF664,
    HOOK_INIT_G_BAF668,
    HOOK_INIT_G_BAF66C,
    HOOK_INIT_G_BB3DCC,
    HOOK_INIT_G_BB3DD0,
    HOOK_INIT_G_BB3DD4,
    HOOK_INIT_G_BB3DD8,
    HOOK_INIT_G_BB3DDC,
    HOOK_INIT_G_BB3DE0,
    HOOK_INIT_G_BB4200,
    HOOK_INIT_G_BB4204,
    HOOK_INIT_G_BB4208,
    HOOK_INIT_G_BB420C,
    HOOK_INIT_G_BB4210,
    HOOK_INIT_G_BB4214,
    HOOK_INIT_G_BB4A74,
    HOOK_INIT_G_BB4A78,
    HOOK_INIT_G_BB4A7C,
    HOOK_INIT_G_BB4A80,
    HOOK_INIT_G_BB4A84,
    HOOK_INIT_G_BB4A88,
    HOOK_INIT_G_C40424,
    HOOK_INIT_G_FURNITURE_MAN,
    HOOK_INIT_G_INTERIOR_MAN,
    HOOK_INIT_G_BREAK_MAN,
    HOOK_INIT_G_PROC_OBJ_MAN,
    HOOK_INIT_G_WATER_CREATURE_MAN,
    HOOK_INIT_G_BAE1F8,
];
