//! The POP static initialisers (PORT-POP): CRT `__init_*` functions that fill POP globals before
//! `WinMain`. Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::{g, x};

/// `__init_g_a9a8b8` (0x84B350): `[0xA9A8B8] = 0.277778 / 50.0` (a CTagManager-area f32).
#[hook(addr = 0x84B350, conv = cdecl, name = "__init_g_a9a8b8", expect = "D9 05 30 86 85 00")]
pub fn init_g_a9a8b8() {
    let v = x(f32::from_bits(0x3E8E_38EB)).fdiv_m(50.0);
    g::<f32>(0xA9A8B8).write(v.to_f32());
}

/// `__init_g_a9a8bc` (0x84B370): `[0xA9A8BC] = 0.005 * 0.0625`.
#[hook(addr = 0x84B370, conv = cdecl, name = "__init_g_a9a8bc", expect = "D9 05 8C A5 85 00")]
pub fn init_g_a9a8bc() {
    let v = x(f32::from_bits(0x3BA3_D70A)).fmul_m(0.0625);
    g::<f32>(0xA9A8BC).write(v.to_f32());
}

/// `__init_g_c2b950` (0x8540D0): `[0xC2B950] = 0.005 * 0.0625` (the constant at 0x8722DC).
#[hook(addr = 0x8540D0, conv = cdecl, name = "__init_g_c2b950", expect = "D9 05 DC 22 87 00")]
pub fn init_g_c2b950() {
    let v = x(f32::from_bits(0x3BA3_D70A)).fmul_m(0.0625);
    g::<f32>(0xC2B950).write(v.to_f32());
}

/// The `mov word [eax], 0xFFFF; add eax, 4` fills.
fn fill_words(base: usize, n: usize) {
    for k in 0..n {
        g::<u16>(base + 4 * k).write(0xFFFF);
    }
}

/// `__init_aNodesToBeCleared` (0x84A040): 5000 node addresses at 0x972CD0 (low word 0xFFFF).
#[hook(addr = 0x84A040, conv = cdecl, name = "__init_aNodesToBeCleared", expect = "B8 D0 2C 97 00")]
pub fn init_a_nodes_to_be_cleared() {
    fill_words(0x972CD0, 5000);
}

/// `__init_aExteriorNodeLinkedTo` (0x84A080): 8 node addresses at 0x977B7C.
#[hook(addr = 0x84A080, conv = cdecl, name = "__init_aExteriorNodeLinkedTo", expect = "B8 7C 7B 97 00")]
pub fn init_a_exterior_node_linked_to() {
    fill_words(0x977B7C, 8);
}

/// `__init_CRoadBlocks__RoadBlockNodes` (0x84A640): 0x145 node addresses at 0xA435A0.
#[hook(addr = 0x84A640, conv = cdecl, name = "__init_CRoadBlocks__RoadBlockNodes", expect = "B8 A0 35 A4 00")]
pub fn init_road_block_nodes() {
    fill_words(0xA435A0, 0x145);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_G_A9A8B8,
    HOOK_INIT_G_A9A8BC,
    HOOK_INIT_G_C2B950,
    HOOK_INIT_A_NODES_TO_BE_CLEARED,
    HOOK_INIT_A_EXTERIOR_NODE_LINKED_TO,
    HOOK_INIT_ROAD_BLOCK_NODES,
];
