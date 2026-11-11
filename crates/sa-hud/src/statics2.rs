//! HUD static initialisers, second pass (PORT-HUD-2): the header-constant `__xc` initialisers of the HUD
//! translation units (RE-SPEC-UNK, `docs/re/specs/HUD/_static-init-unk.md`), `__init_TheText`, the two
//! initialisers that only register an empty destructor, and the `atexit` destructors of the HUD globals.
//! Header constants are generated from the 1.0 US bytes by `tools/gen/hud_statics.py` (each body is
//! exactly `fld; fmul|fdiv [; fdivr]; fstp; ret`); x87 order, stored as f32. Operands in `.rdata` are
//! their exe bits; operands in writable data (set by an earlier initialiser) are read at run time.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, x};

/// No globals of its own.
pub const GLOBALS: &[GlobalDesc] = &[];

// ---- header constants (generated) ----

/// `__init_g_b72cac` (0x84E110): `[0xB72CAC] = 0.005 * 0.0625`.
#[hook(addr = 0x84E110, conv = cdecl, name = "__init_g_b72cac", expect = "D9 05 38 3B 86 00")]
pub fn init_g_b72cac() {
    g::<f32>(0xB72CAC).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_ba176c` (0x84EBE0): `[0xBA176C] = 0.005 * 0.0625`.
#[hook(addr = 0x84EBE0, conv = cdecl, name = "__init_g_ba176c", expect = "D9 05 44 50 86 00")]
pub fn init_g_ba176c() {
    g::<f32>(0xBA176C).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_ba1770` (0x84EC00): `[0xBA1770] = 1.0 / 50.0`.
#[hook(addr = 0x84EC00, conv = cdecl, name = "__init_g_ba1770", expect = "D9 05 24 86 85 00")]
pub fn init_g_ba1770() {
    g::<f32>(0xBA1770).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_ba1774` (0x84EC20): `[0xBA1774] = 100.0 / 3.6`.
#[hook(addr = 0x84EC20, conv = cdecl, name = "__init_g_ba1774", expect = "D9 05 28 86 85 00")]
pub fn init_g_ba1774() {
    g::<f32>(0xBA1774).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_ba1778` (0x84EC40): `[0xBA1778] = 1.0 / (50.0 * 50.0)`.
#[hook(addr = 0x84EC40, conv = cdecl, name = "__init_g_ba1778", expect = "D9 05 4C 50 86 00")]
pub fn init_g_ba1778() {
    g::<f32>(0xBA1778).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_ba177c` (0x84EC60): `[0xBA177C] = [0xBA1778] * 10.0`.
#[hook(addr = 0x84EC60, conv = cdecl, name = "__init_g_ba177c", expect = "D9 05 78 17 BA 00")]
pub fn init_g_ba177c() {
    g::<f32>(0xBA177C).write(x(g::<f32>(0xBA1778).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_ba1780` (0x84EC80): `[0xBA1780] = 0.277778 / 50.0`.
#[hook(addr = 0x84EC80, conv = cdecl, name = "__init_g_ba1780", expect = "D9 05 30 86 85 00")]
pub fn init_g_ba1780() {
    g::<f32>(0xBA1780).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_ba671c` (0x84ED60): `[0xBA671C] = 0.005 * 0.0625`.
#[hook(addr = 0x84ED60, conv = cdecl, name = "__init_g_ba671c", expect = "D9 05 5C 50 86 00")]
pub fn init_g_ba671c() {
    g::<f32>(0xBA671C).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_ba6730` (0x84EDA0): `[0xBA6730] = 1.0 / 50.0`.
#[hook(addr = 0x84EDA0, conv = cdecl, name = "__init_g_ba6730", expect = "D9 05 24 86 85 00")]
pub fn init_g_ba6730() {
    g::<f32>(0xBA6730).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_ba6734` (0x84EDC0): `[0xBA6734] = 100.0 / 3.6`.
#[hook(addr = 0x84EDC0, conv = cdecl, name = "__init_g_ba6734", expect = "D9 05 28 86 85 00")]
pub fn init_g_ba6734() {
    g::<f32>(0xBA6734).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_ba6738` (0x84EDE0): `[0xBA6738] = 1.0 / (50.0 * 50.0)`.
#[hook(addr = 0x84EDE0, conv = cdecl, name = "__init_g_ba6738", expect = "D9 05 B0 51 86 00")]
pub fn init_g_ba6738() {
    g::<f32>(0xBA6738).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_ba673c` (0x84EE00): `[0xBA673C] = [0xBA6738] * 10.0`.
#[hook(addr = 0x84EE00, conv = cdecl, name = "__init_g_ba673c", expect = "D9 05 38 67 BA 00")]
pub fn init_g_ba673c() {
    g::<f32>(0xBA673C).write(x(g::<f32>(0xBA6738).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_ba6740` (0x84EE20): `[0xBA6740] = 0.277778 / 50.0`.
#[hook(addr = 0x84EE20, conv = cdecl, name = "__init_g_ba6740", expect = "D9 05 30 86 85 00")]
pub fn init_g_ba6740() {
    g::<f32>(0xBA6740).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_ba82c0` (0x84EE60): `[0xBA82C0] = 0.005 * 0.0625`.
#[hook(addr = 0x84EE60, conv = cdecl, name = "__init_g_ba82c0", expect = "D9 05 8C 55 86 00")]
pub fn init_g_ba82c0() {
    g::<f32>(0xBA82C0).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_ba82c4` (0x84EE80): `[0xBA82C4] = 1.0 / 50.0`.
#[hook(addr = 0x84EE80, conv = cdecl, name = "__init_g_ba82c4", expect = "D9 05 24 86 85 00")]
pub fn init_g_ba82c4() {
    g::<f32>(0xBA82C4).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_ba82c8` (0x84EEA0): `[0xBA82C8] = 100.0 / 3.6`.
#[hook(addr = 0x84EEA0, conv = cdecl, name = "__init_g_ba82c8", expect = "D9 05 28 86 85 00")]
pub fn init_g_ba82c8() {
    g::<f32>(0xBA82C8).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_ba82cc` (0x84EEC0): `[0xBA82CC] = 1.0 / (50.0 * 50.0)`.
#[hook(addr = 0x84EEC0, conv = cdecl, name = "__init_g_ba82cc", expect = "D9 05 94 55 86 00")]
pub fn init_g_ba82cc() {
    g::<f32>(0xBA82CC).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_ba82d0` (0x84EEE0): `[0xBA82D0] = [0xBA82CC] * 10.0`.
#[hook(addr = 0x84EEE0, conv = cdecl, name = "__init_g_ba82d0", expect = "D9 05 CC 82 BA 00")]
pub fn init_g_ba82d0() {
    g::<f32>(0xBA82D0).write(x(g::<f32>(0xBA82CC).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_ba82d4` (0x84EF00): `[0xBA82D4] = 0.277778 / 50.0`.
#[hook(addr = 0x84EF00, conv = cdecl, name = "__init_g_ba82d4", expect = "D9 05 30 86 85 00")]
pub fn init_g_ba82d4() {
    g::<f32>(0xBA82D4).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_ba82e4` (0x84EF20): `[0xBA82E4] = 0.005 * 0.0625`.
#[hook(addr = 0x84EF20, conv = cdecl, name = "__init_g_ba82e4", expect = "D9 05 B0 65 86 00")]
pub fn init_g_ba82e4() {
    g::<f32>(0xBA82E4).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_ba82e8` (0x84EF40): `[0xBA82E8] = 1.0 / 50.0`.
#[hook(addr = 0x84EF40, conv = cdecl, name = "__init_g_ba82e8", expect = "D9 05 24 86 85 00")]
pub fn init_g_ba82e8() {
    g::<f32>(0xBA82E8).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_ba82ec` (0x84EF60): `[0xBA82EC] = 100.0 / 3.6`.
#[hook(addr = 0x84EF60, conv = cdecl, name = "__init_g_ba82ec", expect = "D9 05 28 86 85 00")]
pub fn init_g_ba82ec() {
    g::<f32>(0xBA82EC).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_ba82f0` (0x84EF80): `[0xBA82F0] = 1.0 / (50.0 * 50.0)`.
#[hook(addr = 0x84EF80, conv = cdecl, name = "__init_g_ba82f0", expect = "D9 05 B8 65 86 00")]
pub fn init_g_ba82f0() {
    g::<f32>(0xBA82F0).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_ba82f4` (0x84EFA0): `[0xBA82F4] = [0xBA82F0] * 10.0`.
#[hook(addr = 0x84EFA0, conv = cdecl, name = "__init_g_ba82f4", expect = "D9 05 F0 82 BA 00")]
pub fn init_g_ba82f4() {
    g::<f32>(0xBA82F4).write(x(g::<f32>(0xBA82F0).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_ba82f8` (0x84EFC0): `[0xBA82F8] = 0.277778 / 50.0`.
#[hook(addr = 0x84EFC0, conv = cdecl, name = "__init_g_ba82f8", expect = "D9 05 30 86 85 00")]
pub fn init_g_ba82f8() {
    g::<f32>(0xBA82F8).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_ba86bc` (0x84EFE0): `[0xBA86BC] = 0.005 * 0.0625`.
#[hook(addr = 0x84EFE0, conv = cdecl, name = "__init_g_ba86bc", expect = "D9 05 5C 69 86 00")]
pub fn init_g_ba86bc() {
    g::<f32>(0xBA86BC).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_ba86c0` (0x84F000): `[0xBA86C0] = 1.0 / 50.0`.
#[hook(addr = 0x84F000, conv = cdecl, name = "__init_g_ba86c0", expect = "D9 05 24 86 85 00")]
pub fn init_g_ba86c0() {
    g::<f32>(0xBA86C0).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_ba86c4` (0x84F020): `[0xBA86C4] = 100.0 / 3.6`.
#[hook(addr = 0x84F020, conv = cdecl, name = "__init_g_ba86c4", expect = "D9 05 28 86 85 00")]
pub fn init_g_ba86c4() {
    g::<f32>(0xBA86C4).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_ba86c8` (0x84F040): `[0xBA86C8] = 1.0 / (50.0 * 50.0)`.
#[hook(addr = 0x84F040, conv = cdecl, name = "__init_g_ba86c8", expect = "D9 05 64 69 86 00")]
pub fn init_g_ba86c8() {
    g::<f32>(0xBA86C8).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_ba86cc` (0x84F060): `[0xBA86CC] = [0xBA86C8] * 10.0`.
#[hook(addr = 0x84F060, conv = cdecl, name = "__init_g_ba86cc", expect = "D9 05 C8 86 BA 00")]
pub fn init_g_ba86cc() {
    g::<f32>(0xBA86CC).write(x(g::<f32>(0xBA86C8).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_ba86d0` (0x84F080): `[0xBA86D0] = 0.277778 / 50.0`.
#[hook(addr = 0x84F080, conv = cdecl, name = "__init_g_ba86d0", expect = "D9 05 30 86 85 00")]
pub fn init_g_ba86d0() {
    g::<f32>(0xBA86D0).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_bab1e4` (0x84F100): `[0xBAB1E4] = 0.005 * 0.0625`.
#[hook(addr = 0x84F100, conv = cdecl, name = "__init_g_bab1e4", expect = "D9 05 A8 6B 86 00")]
pub fn init_g_bab1e4() {
    g::<f32>(0xBAB1E4).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_bab1e8` (0x84F120): `[0xBAB1E8] = 1.0 / 50.0`.
#[hook(addr = 0x84F120, conv = cdecl, name = "__init_g_bab1e8", expect = "D9 05 24 86 85 00")]
pub fn init_g_bab1e8() {
    g::<f32>(0xBAB1E8).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_bab1ec` (0x84F140): `[0xBAB1EC] = 100.0 / 3.6`.
#[hook(addr = 0x84F140, conv = cdecl, name = "__init_g_bab1ec", expect = "D9 05 28 86 85 00")]
pub fn init_g_bab1ec() {
    g::<f32>(0xBAB1EC).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_bab1f0` (0x84F160): `[0xBAB1F0] = 1.0 / (50.0 * 50.0)`.
#[hook(addr = 0x84F160, conv = cdecl, name = "__init_g_bab1f0", expect = "D9 05 B0 6B 86 00")]
pub fn init_g_bab1f0() {
    g::<f32>(0xBAB1F0).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_bab1f4` (0x84F180): `[0xBAB1F4] = [0xBAB1F0] * 10.0`.
#[hook(addr = 0x84F180, conv = cdecl, name = "__init_g_bab1f4", expect = "D9 05 F0 B1 BA 00")]
pub fn init_g_bab1f4() {
    g::<f32>(0xBAB1F4).write(x(g::<f32>(0xBAB1F0).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_bab1f8` (0x84F1A0): `[0xBAB1F8] = 0.277778 / 50.0`.
#[hook(addr = 0x84F1A0, conv = cdecl, name = "__init_g_bab1f8", expect = "D9 05 30 86 85 00")]
pub fn init_g_bab1f8() {
    g::<f32>(0xBAB1F8).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_c1a558` (0x8532A0): `[0xC1A558] = 0.005 * 0.0625`.
#[hook(addr = 0x8532A0, conv = cdecl, name = "__init_g_c1a558", expect = "D9 05 24 0E 87 00")]
pub fn init_g_c1a558() {
    g::<f32>(0xC1A558).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_c1a55c` (0x8532C0): `[0xC1A55C] = 1.0 / 50.0`.
#[hook(addr = 0x8532C0, conv = cdecl, name = "__init_g_c1a55c", expect = "D9 05 24 86 85 00")]
pub fn init_g_c1a55c() {
    g::<f32>(0xC1A55C).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_c1a560` (0x8532E0): `[0xC1A560] = 100.0 / 3.6`.
#[hook(addr = 0x8532E0, conv = cdecl, name = "__init_g_c1a560", expect = "D9 05 28 86 85 00")]
pub fn init_g_c1a560() {
    g::<f32>(0xC1A560).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_c1a564` (0x853300): `[0xC1A564] = 1.0 / (50.0 * 50.0)`.
#[hook(addr = 0x853300, conv = cdecl, name = "__init_g_c1a564", expect = "D9 05 2C 0E 87 00")]
pub fn init_g_c1a564() {
    g::<f32>(0xC1A564).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_c1a568` (0x853320): `[0xC1A568] = [0xC1A564] * 10.0`.
#[hook(addr = 0x853320, conv = cdecl, name = "__init_g_c1a568", expect = "D9 05 64 A5 C1 00")]
pub fn init_g_c1a568() {
    g::<f32>(0xC1A568).write(x(g::<f32>(0xC1A564).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_c1a56c` (0x853340): `[0xC1A56C] = 0.277778 / 50.0`.
#[hook(addr = 0x853340, conv = cdecl, name = "__init_g_c1a56c", expect = "D9 05 30 86 85 00")]
pub fn init_g_c1a56c() {
    g::<f32>(0xC1A56C).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_c1aeb4` (0x853380): `[0xC1AEB4] = 0.005 * 0.0625`.
#[hook(addr = 0x853380, conv = cdecl, name = "__init_g_c1aeb4", expect = "D9 05 48 0F 87 00")]
pub fn init_g_c1aeb4() {
    g::<f32>(0xC1AEB4).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_c6e990` (0x854C90): `[0xC6E990] = 0.005 * 0.0625`.
#[hook(addr = 0x854C90, conv = cdecl, name = "__init_g_c6e990", expect = "D9 05 84 28 87 00")]
pub fn init_g_c6e990() {
    g::<f32>(0xC6E990).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

/// `__init_g_c6e994` (0x854CB0): `[0xC6E994] = 1.0 / 50.0`.
#[hook(addr = 0x854CB0, conv = cdecl, name = "__init_g_c6e994", expect = "D9 05 24 86 85 00")]
pub fn init_g_c6e994() {
    g::<f32>(0xC6E994).write(x(f32::from_bits(0x3F800000)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_c6e998` (0x854CD0): `[0xC6E998] = 100.0 / 3.6`.
#[hook(addr = 0x854CD0, conv = cdecl, name = "__init_g_c6e998", expect = "D9 05 28 86 85 00")]
pub fn init_g_c6e998() {
    g::<f32>(0xC6E998).write(x(f32::from_bits(0x42C80000)).fdiv_m(f32::from_bits(0x40666666)).to_f32());
}

/// `__init_g_c6e99c` (0x854CF0): `[0xC6E99C] = 1.0 / (50.0 * 50.0)`.
#[hook(addr = 0x854CF0, conv = cdecl, name = "__init_g_c6e99c", expect = "D9 05 8C 28 87 00")]
pub fn init_g_c6e99c() {
    g::<f32>(0xC6E99C).write(x(f32::from_bits(0x42480000)).fmul_m(f32::from_bits(0x42480000)).fdivr_m(f32::from_bits(0x3F800000)).to_f32());
}

/// `__init_g_c6e9a0` (0x854D10): `[0xC6E9A0] = [0xC6E99C] * 10.0`.
#[hook(addr = 0x854D10, conv = cdecl, name = "__init_g_c6e9a0", expect = "D9 05 9C E9 C6 00")]
pub fn init_g_c6e9a0() {
    g::<f32>(0xC6E9A0).write(x(g::<f32>(0xC6E99C).read()).fmul_m(f32::from_bits(0x41200000)).to_f32());
}

/// `__init_g_c6e9a4` (0x854D30): `[0xC6E9A4] = 0.277778 / 50.0`.
#[hook(addr = 0x854D30, conv = cdecl, name = "__init_g_c6e9a4", expect = "D9 05 30 86 85 00")]
pub fn init_g_c6e9a4() {
    g::<f32>(0xC6E9A4).write(x(f32::from_bits(0x3E8E38EB)).fdiv_m(f32::from_bits(0x42480000)).to_f32());
}

/// `__init_g_c71a58` (0x854D80): `[0xC71A58] = 0.005 * 0.0625`.
#[hook(addr = 0x854D80, conv = cdecl, name = "__init_g_c71a58", expect = "D9 05 9C 28 87 00")]
pub fn init_g_c71a58() {
    g::<f32>(0xC71A58).write(x(f32::from_bits(0x3BA3D70A)).fmul_m(f32::from_bits(0x3D800000)).to_f32());
}

// ---- other initialisers and the atexit destructors ----

orig! {
    /// CRT `_atexit` (0x821D1E)
    pub fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// CRT `eh vector destructor iterator` (0x821E02, stdcall)
    pub fn eh_vector_dtor(arr: GPtr<u8>, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// `CText::CText()` (0x6A00F0)
    pub fn text_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A00F0 => crate::text::text_constructor;
    /// `CText::~CText()` (0x6A0140)
    pub fn text_dtor(this: GPtr<u8>) = thiscall 0x6A0140 => crate::text::text_destructor;
    /// `CMenuManager::~CMenuManager()` (0x579440)
    pub fn menu_manager_dtor(this: GPtr<u8>) = thiscall 0x579440 => unported;
    /// `CHudColours::~CHudColours()` (0x58FD90)
    pub fn hud_colours_dtor(this: GPtr<u8>) = thiscall 0x58FD90 => unported;
}

/// `TheText` (0xC1B340)
const THE_TEXT: usize = 0xC1B340;
/// `CSprite2d::~CSprite2d()` (0x7281E0)
const SPRITE2D_DTOR: u32 = 0x7281E0;

/// `__init_TheText` (0x853460): `TheText.CText()`, `atexit(__dtor_TheText)`.
#[hook(addr = 0x853460, conv = cdecl, name = "__init_TheText", expect = "B9 40 B3 C1 00")]
pub fn init_the_text() {
    text_ctor(g(THE_TEXT));
    atexit(0x856A00);
}

/// `__init_854da0` (0x854DA0): only `atexit(0x856B80)` (an object with an empty user destructor).
#[hook(addr = 0x854DA0, conv = cdecl, name = "__init_854da0", expect = "68 80 6B 85 00")]
pub fn init_854da0() {
    atexit(0x856B80);
}

/// `__init_854db0` (0x854DB0): only `atexit(0x856B90)`.
#[hook(addr = 0x854DB0, conv = cdecl, name = "__init_854db0", expect = "68 90 6B 85 00")]
pub fn init_854db0() {
    atexit(0x856B90);
}

/// `__dtor_TheText` (0x856A00): `mov ecx, TheText; jmp CText::~CText`.
#[hook(addr = 0x856A00, conv = cdecl, name = "__dtor_TheText", expect = "B9 40 B3 C1 00")]
pub fn dtor_the_text() {
    text_dtor(g(THE_TEXT));
}

/// `__dtor_FrontEndMenuManager` (0x856560): `FrontEndMenuManager.~CMenuManager()` (tail jump).
#[hook(addr = 0x856560, conv = cdecl, name = "__dtor_FrontEndMenuManager", expect = "B9 48 67 BA 00")]
pub fn dtor_front_end_menu_manager() {
    menu_manager_dtor(g(0xBA6748));
}

/// `__dtor_HudColour` (0x8565D0): `HudColour.~CHudColours()` (tail jump).
#[hook(addr = 0x8565D0, conv = cdecl, name = "__dtor_HudColour", expect = "B9 2C B2 BA 00")]
pub fn dtor_hud_colour() {
    hud_colours_dtor(g(0xBAB22C));
}

/// `__dtor_CRadar__ArrowBlipColour` (0x856570): `eh vector destructor iterator(0xBA86D4, 4, 6, 0x43CE70)`.
#[hook(addr = 0x856570, conv = cdecl, name = "__dtor_CRadar__ArrowBlipColour", expect = "68 70 CE 43 00")]
pub fn dtor_radar_arrow_blip_colour() {
    eh_vector_dtor(g(0xBA86D4), 4, 6, 0x43CE70);
}

/// `__dtor_CRadar__RadarBlipSprites` (0x856590): the 64 `CSprite2d` at 0xBAA250.
#[hook(addr = 0x856590, conv = cdecl, name = "__dtor_CRadar__RadarBlipSprites", expect = "68 E0 81 72 00")]
pub fn dtor_radar_blip_sprites() {
    eh_vector_dtor(g(0xBAA250), 4, 0x40, SPRITE2D_DTOR);
}

/// `__dtor_CHud__Sprites` (0x8565B0): the 6 `CSprite2d` at 0xBAB1FC.
#[hook(addr = 0x8565B0, conv = cdecl, name = "__dtor_CHud__Sprites", expect = "68 E0 81 72 00")]
pub fn dtor_hud_sprites() {
    eh_vector_dtor(g(0xBAB1FC), 4, 6, SPRITE2D_DTOR);
}

/// `__dtor_CFont__Sprite` (0x856B40): the 2 `CSprite2d` at 0xC71AD0.
#[hook(addr = 0x856B40, conv = cdecl, name = "__dtor_CFont__Sprite", expect = "68 E0 81 72 00")]
pub fn dtor_font_sprite() {
    eh_vector_dtor(g(0xC71AD0), 4, 2, SPRITE2D_DTOR);
}

/// `__dtor_CFont__ButtonSprite` (0x856B60): the 15 `CSprite2d` at 0xC71AD8.
#[hook(addr = 0x856B60, conv = cdecl, name = "__dtor_CFont__ButtonSprite", expect = "68 E0 81 72 00")]
pub fn dtor_font_button_sprite() {
    eh_vector_dtor(g(0xC71AD8), 4, 0xF, SPRITE2D_DTOR);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_G_B72CAC,
    HOOK_INIT_G_BA176C,
    HOOK_INIT_G_BA1770,
    HOOK_INIT_G_BA1774,
    HOOK_INIT_G_BA1778,
    HOOK_INIT_G_BA177C,
    HOOK_INIT_G_BA1780,
    HOOK_INIT_G_BA671C,
    HOOK_INIT_G_BA6730,
    HOOK_INIT_G_BA6734,
    HOOK_INIT_G_BA6738,
    HOOK_INIT_G_BA673C,
    HOOK_INIT_G_BA6740,
    HOOK_INIT_G_BA82C0,
    HOOK_INIT_G_BA82C4,
    HOOK_INIT_G_BA82C8,
    HOOK_INIT_G_BA82CC,
    HOOK_INIT_G_BA82D0,
    HOOK_INIT_G_BA82D4,
    HOOK_INIT_G_BA82E4,
    HOOK_INIT_G_BA82E8,
    HOOK_INIT_G_BA82EC,
    HOOK_INIT_G_BA82F0,
    HOOK_INIT_G_BA82F4,
    HOOK_INIT_G_BA82F8,
    HOOK_INIT_G_BA86BC,
    HOOK_INIT_G_BA86C0,
    HOOK_INIT_G_BA86C4,
    HOOK_INIT_G_BA86C8,
    HOOK_INIT_G_BA86CC,
    HOOK_INIT_G_BA86D0,
    HOOK_INIT_G_BAB1E4,
    HOOK_INIT_G_BAB1E8,
    HOOK_INIT_G_BAB1EC,
    HOOK_INIT_G_BAB1F0,
    HOOK_INIT_G_BAB1F4,
    HOOK_INIT_G_BAB1F8,
    HOOK_INIT_G_C1A558,
    HOOK_INIT_G_C1A55C,
    HOOK_INIT_G_C1A560,
    HOOK_INIT_G_C1A564,
    HOOK_INIT_G_C1A568,
    HOOK_INIT_G_C1A56C,
    HOOK_INIT_G_C1AEB4,
    HOOK_INIT_G_C6E990,
    HOOK_INIT_G_C6E994,
    HOOK_INIT_G_C6E998,
    HOOK_INIT_G_C6E99C,
    HOOK_INIT_G_C6E9A0,
    HOOK_INIT_G_C6E9A4,
    HOOK_INIT_G_C71A58,
    HOOK_INIT_THE_TEXT,
    HOOK_INIT_854DA0,
    HOOK_INIT_854DB0,
    HOOK_DTOR_THE_TEXT,
    HOOK_DTOR_FRONT_END_MENU_MANAGER,
    HOOK_DTOR_HUD_COLOUR,
    HOOK_DTOR_RADAR_ARROW_BLIP_COLOUR,
    HOOK_DTOR_RADAR_BLIP_SPRITES,
    HOOK_DTOR_HUD_SPRITES,
    HOOK_DTOR_FONT_SPRITE,
    HOOK_DTOR_FONT_BUTTON_SPRITE,
];
