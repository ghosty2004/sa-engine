//! FRONTIER-2 turn 2, lane B (batch `frontier2-b5`): population / path functions on the phase B frontier. Ported
//! from the 1.0 US bytes (the `.HOODLUM` bodies followed through the no-CD protection's VM calls, evaluated on the exe
//! image); gta-reversed as a reading aid only. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::common::{g, rd, wr};

/// `CPathFind::RemoveInteriorLinks(int32 node)` (stub 0x44DF60 → .HOODLUM 0x1562390, thiscall (`this` unused: ECX is
/// replaced by the protection VM's result, descriptor 0x155FF20, evaluated: `ConnectsToGiven` 0x96EAC0), `ret 4`): for
/// each of the `NumNodesGiven` (0x96EF80) given nodes, the six link bytes (6 per node; gta-reversed declares 8): all set
/// to -1 for the node itself, else each that names the node (signed byte compare).
#[hook(addr = 0x44DF60, conv = thiscall, name = "CPathFind::RemoveInteriorLinks", expect = "E9 2B 44 11 01",
       this = "CPathFind", unverified = "frontier2-b5", route = "L2", hazards = "none",
       note = "VM 0xCB73E0 (descriptor 0x155FF20) evaluated: 0x96EAC0")]
pub fn path_find_remove_interior_links(_this: GPtr<u8>, node: i32) {
    let n = g::<i32>(0x96_EF80).read();
    let mut links = GPtr::<u8>::from_addr(0x96_EAC0);
    for i in 0..n.max(0) {
        for k in 0..6isize {
            if i == node || i32::from(rd::<i8>(links, k)) == node {
                wr::<u8>(links, k, 0xFF);
            }
        }
        links = links.byte_add(6);
    }
}

/// `CRoadBlocks::RegisterScriptRoadBlock(CVector cornerA, CVector cornerB, bool type)` (stub 0x460DF0 → .HOODLUM
/// 0x156AB70, cdecl): the first of the 16 script road blocks (0xA43AB8, 0x1C bytes) whose active byte (+0x18) is clear
/// (searched 8 at a time; the 8th index of a group is the protection VM's value, descriptor 0x1560010, evaluated: 7)
/// becomes active (+0x18 = +0x19 = 1) with the two corners (+0, +0xC) and the type (+0x1A); none when all are active.
#[hook(addr = 0x460DF0, conv = cdecl, name = "CRoadBlocks::RegisterScriptRoadBlock", expect = "E9 7B 9D 10 01",
       unverified = "frontier2-b5", route = "L2", hazards = "none",
       note = "VM 0xCB73E0 (descriptor 0x1560010) evaluated: 7")]
#[allow(clippy::too_many_arguments)]
pub fn road_blocks_register_script_road_block(ax: u32, ay: u32, az: u32, bx: u32, by: u32, bz: u32, ty: u32) {
    let Some(i) = (0..16isize).find(|i| g::<u8>((0xA4_3AD0 + 0x1C * i) as usize).read() == 0) else {
        return;
    };
    let e = GPtr::<u8>::from_addr((0xA4_3AB8 + 0x1C * i) as usize);
    wr::<u8>(e, 0x18, 1);
    wr::<u8>(e, 0x19, 1);
    for (o, v) in [(0isize, ax), (4, ay), (8, az), (0xC, bx), (0x10, by), (0x14, bz)] {
        wr::<u32>(e, o, v);
    }
    wr::<u8>(e, 0x1A, ty as u8);
}

/// `CCarAI::GetCarToGoToCoorsAccurate` (0x41D0E0, cdecl, ST0): the phase B entry of the port in `carctrl4` (unhookable:
/// the protection calls through its entry dwords as data). The result goes back as an f64.
#[sa_core::native(addr = 0x41D0E0, conv = cdecl, name = "CCarAI::GetCarToGoToCoorsAccurate")]
pub fn get_car_to_go_to_coors_accurate_native(veh: GPtr<u8>, c: GPtr<f32>, style: u8, cruise: u8) -> f64 {
    crate::carctrl4::get_car_to_go_to_coors_accurate(veh, c, style, cruise).to_f64()
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PATH_FIND_REMOVE_INTERIOR_LINKS, HOOK_ROAD_BLOCKS_REGISTER_SCRIPT_ROAD_BLOCK];

/// This module's phase B natives.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_GET_CAR_TO_GO_TO_COORS_ACCURATE_NATIVE];
