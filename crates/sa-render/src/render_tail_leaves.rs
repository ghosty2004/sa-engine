//! A small in-world render leaf (batch `mvpl-render-1`): the roadsign material alpha callback. Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;

use crate::common::{rd, wr};

/// `FUN_006fe220(RpMaterial*, void* data)` (0x6FE220, cdecl; the `RpGeometryForAllMaterials` callback of the
/// roadsign atomic render callback): the material colour dword (+4) gets its top byte (alpha) replaced by the low
/// byte of `data`; returns the material. The original assembles the new dword in its own `material` argument slot
/// and writes it back whole.
#[hook(addr = 0x6fe220, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 44 24 04 8B 48 04",
       unverified = "mvpl-render-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the RW material colour (RW heap); its own argument slot holds the colour")]
pub fn roadsign_material_set_alpha_cb(material: GPtr<u8>, data: u32) -> GPtr<u8> {
    let c = rd::<u32>(material, 4);
    wr::<u32>(material, 4, (c & 0x00FF_FFFF) | ((data & 0xFF) << 24));
    material
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_ROADSIGN_MATERIAL_SET_ALPHA_CB];
