//! `CPathFind::Find2NodesForCarCreation` (batch `std8-pop-1`), ported from the 1.0 US bytes (`.HOODLUM` body
//! 0x156D150). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `CPathFind::FindNodeClosestToCoors(CVector, type, count, CNodeAddress* out, maxDist, lowTraffic, unused,
    /// boats, ignoreInterior)` (0x44FA30, `ret 0x2C`)
    fn record_nodes_closest_to_coors(this: GPtr<u8>, px: f32, py: f32, pz: f32, node_type: u32, count: i32, out: GPtr<u32>, max_dist: f32, a: u32, b: u32, c: u32, d: u32) = thiscall 0x44FA30 => crate::pathfind::record_nodes_closest_to_coors;
    /// `CPathFind::These2NodesAreAdjacent(CNodeAddress, CNodeAddress)` (0x44D230, `ret 8`, AL)
    fn these_2_nodes_are_adjacent(this: GPtr<u8>, a: u32, b: u32) -> u8 = thiscall 0x44D230 => crate::pathfind::these_2_nodes_are_adjacent;
}

/// `ThePaths` (0x96F050)
const THE_PATHS: usize = 0x96F050;

/// `CPathFind::Find2NodesForCarCreation(CVector pos, CNodeAddress* out1, CNodeAddress* out2, bool lowTraffic)`
/// (stub 0x452090 → 0x156D150, thiscall, `ret 0x18`). Four node slots get the area word -1 (their upper words
/// are stack leftovers in the original, 0 here); `this->FindNodeClosestToCoors(pos, 0, 4, slots, 999999.9,
/// lowTraffic, false, false, true)`. Slot 0's area word -1: that word to both outputs (16-bit stores), done.
/// Otherwise `*out1 = slot 0`; for the slots `i = 1..3` with an area word other than -1, the first one
/// `ThePaths.These2NodesAreAdjacent(slot 0, slot i)` reports not adjacent goes to `*out2`; none: `*out2` is not
/// written.
///
/// The loop start index comes out of the no-CD protection VM (0xCB73E0 with descriptor 0x1560064, into EDI); the
/// loop walks the slots from slot 1 and indexes them with EDI, so the VM's value is 1 (decoded from the loop's own
/// pointer, not by evaluating the VM).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x452090, conv = thiscall, name = "CPathFind::Find2NodesForCarCreation", expect = "E9 BB B0 11 01",
       this = "CPathFind", route = "L2", hazards = "uninit-locals", recs = "grove-walk",
       note = "the original computes the loop start (1) in the no-CD VM 0xCB73E0")]
pub fn find_2_nodes_for_car_creation(
    this: GPtr<u8>,
    px: f32,
    py: f32,
    pz: f32,
    out1: GPtr<u32>,
    out2: GPtr<u32>,
    low_traffic: u32,
) {
    let mut slots = [0xFFFFu32; 4];
    let s = GPtr::<u32>::from_raw(slots.as_mut_ptr());
    record_nodes_closest_to_coors(
        this,
        px,
        py,
        pz,
        0,
        4,
        s,
        f32::from_bits(0x4974_23FE),
        low_traffic,
        0,
        0,
        1,
    );
    let first = s.read();
    if first as u16 == 0xFFFF {
        out1.cast::<u16>().write(first as u16);
        out2.cast::<u16>().write(first as u16);
        return;
    }
    out1.write(first);
    for i in 1..4 {
        let n = s.wrapping_index(i).read();
        if n as u16 != 0xFFFF && these_2_nodes_are_adjacent(GPtr::from_addr(THE_PATHS), first, n) == 0 {
            out2.write(n);
            return;
        }
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_FIND_2_NODES_FOR_CAR_CREATION];
