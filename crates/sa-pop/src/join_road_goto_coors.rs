//! `CCarCtrl::JoinCarWithRoadSystemGotoCoors` (INTRO-2: the police car of the intro after the cutscene) and the
//! `CPathFind::RemoveBadStartNode` it calls. Ported from the 1.0 US bytes (stub 0x42F870 → `.HOODLUM` 0x156A4A0);
//! gta-reversed `CarCtrl.cpp` / `PathFind.cpp` as reading aids.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr, x};

orig! {
    /// `CPathFind::DoPathSearch(...)` (0x4515D0, thiscall, `ret 0x4C`)
    #[allow(clippy::too_many_arguments)]
    fn do_path_search(this: GPtr<u8>, path_type: u32, ox: f32, oy: f32, oz: f32, origin_addr: u32, tx: f32, ty: f32, tz: f32,
        out_nodes: GPtr<u32>, out_count: GPtr<i16>, max_nodes: i32, out_distance: GPtr<f32>, max_search: f32,
        target_addr: GPtr<u32>, max_depth: f32, same_lane: u32, forbidden: u32, allow_water: u32, for_boats: u32)
        = thiscall 0x4515D0 => crate::pathfind::do_path_search;
    /// `CPathNode::GetPosition(CVector* out)` (0x420A10, `ret 4`)
    fn node_get_position(node: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x420A10 => crate::pathfind::get_position;
    /// `CAutoPilot::RemoveOnePathNode()` (0x41B950)
    fn auto_pilot_remove_one_path_node(ap: GPtr<u8>) = thiscall 0x41B950 => unported;
    /// `CCarCtrl::FindLinksToGoWithTheseNodes(CVehicle*)` (0x42B470)
    fn find_links_to_go_with_these_nodes(v: GPtr<u8>) = cdecl 0x42B470 => unported;
    /// `CCarCtrl::JoinCarWithRoadSystem(CVehicle*)` (0x42F5A0)
    fn join_car_with_road_system(v: GPtr<u8>) = cdecl 0x42F5A0 => unported;
}

/// `ThePaths`: the `this` of `RemoveBadStartNode` comes out of the no-CD VM 0xCB73E0 (descriptor 0x155FFEC, called
/// at 0x156A590 with the flags and EAX saved around it). Evaluated on the exe image (unicorn, 293 instructions, no
/// write outside the stack): 0x96F050.
const THE_PATHS_FROM_VM: usize = 0x96F050;

/// `CNodeAddress` "empty" (0x8A5F44, 0xFFFF in the image)
const EMPTY_NODE_ADDRESS: usize = 0x8A5F44;

/// The position of an entity: the matrix's (+0x14) translation (+0x30) when it has one, else the placement (+4).
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `CPathFind::RemoveBadStartNode(CVector pos, CNodeAddress* nodes, int16* count)` (0x44E4F0, thiscall, `ret 0x14`;
/// not in the catalogue: its only caller is 0x156A596 below, with `this` from the VM).
///
/// With at least two nodes whose areas are both loaded (`m_pPathNodes` +0x804): A = node 1's position, B = node 0's,
/// `((n0.y - p.y) * f32(n1.y - p.y)) + ((n1.x - p.x) * (n0.x - p.x))` (node positions re-read through `GetPosition`
/// as the original does); below 0.0 (strict, `test ah, 5; jp` not taken) the first node is dropped (count - 1, the
/// rest moved down with a 16-bit index).
fn remove_bad_start_node(this: GPtr<u8>, px: f32, py: f32, nodes: GPtr<u32>, count: GPtr<i16>) {
    if count.read() < 2 {
        return;
    }
    let n0 = nodes.read();
    let a0 = rd::<GPtr<u8>>(this, 0x804 + 4 * (n0 & 0xFFFF) as isize);
    if a0.is_null() {
        return;
    }
    let n1 = nodes.wrapping_index(1).read();
    let a1 = rd::<GPtr<u8>>(this, 0x804 + 4 * (n1 & 0xFFFF) as isize);
    if a1.is_null() {
        return;
    }
    let node0 = a0.byte_add(((n0 >> 16) * 0x1C) as isize);
    let node1 = a1.byte_add(((n1 >> 16) * 0x1C) as isize);
    let (mut pa, mut pb, mut pc) = ([0f32; 3], [0f32; 3], [0f32; 3]);
    let ra = node_get_position(node1, GPtr::from_raw(pa.as_mut_ptr()));
    let rb = node_get_position(node0, GPtr::from_raw(pb.as_mut_ptr()));
    let rc = node_get_position(node1, GPtr::from_raw(pc.as_mut_ptr()));
    let t1 = x(rc.wrapping_index(1).read()).fsub_m(py).to_f32();
    let rc = node_get_position(node0, GPtr::from_raw(pc.as_mut_ptr()));
    let yprod = x(rc.wrapping_index(1).read()).fsub_m(py).fmul_m(t1);
    let dx1 = x(ra.read()).fsub_m(px);
    let dx0 = x(rb.read()).fsub_m(px);
    let sum = yprod.fadd(dx1.fmul(dx0));
    if sum.fcom(x(g::<f32>(0x858B50).read())).test_parity(0x05) {
        return;
    }
    let n = count.read().wrapping_sub(1);
    count.write(n);
    let mut i: i16 = 0;
    while i < count.read() {
        nodes.wrapping_index(i as i32).write(nodes.wrapping_index(i as i32 + 1).read());
        i = i.wrapping_add(1);
    }
}

/// `CCarCtrl::JoinCarWithRoadSystemGotoCoors(CVehicle*, const CVector& coors, bool, bool forBoats)` (stub 0x42F870 →
/// 0x156A4A0, cdecl).
///
/// The autopilot's destination (+0x3EC) = coors; `ThePaths.DoPathSearch(0, position, empty, coors, +0x3F8,
/// &+0x418, 8, null, 999999.9, null, 999999.9, (+0x3DB >> 6) & 1, empty, model == 0x21B (the low byte of a dword
/// whose upper bytes are the last argument's, as the original pushes EDX after `sete dl`), forBoats)`; then
/// `RemoveBadStartNode(position, +0x3F8, &+0x418)` on the VM's `ThePaths`. With two or more nodes: current node
/// (+0x390) = node 0, starting node word (+0x398) = 0xFFFF, `RemoveOnePathNode`, next node (+0x394) = node 0,
/// `RemoveOnePathNode`, `FindLinksToGoWithTheseNodes`, +0x3B7 = +0x3B8 = 0, false. Else `JoinCarWithRoadSystem`,
/// count 0, true.
#[hook(addr = 0x42F870, conv = cdecl, name = "CCarCtrl::JoinCarWithRoadSystemGotoCoors", expect = "E9 2B AC 13 01",
       unverified = "intro3-5", route = "L2", hazards = "heap-writes", recs = "intro",
       note = "RemoveBadStartNode (0x44E4F0, not catalogued) is ported inline; its this comes from the no-CD VM (evaluated: 0x96F050)")]
pub fn join_car_with_road_system_goto_coors(veh: GPtr<u8>, coors: GPtr<u32>, _unused: u32, for_boats: u32) -> u8 {
    for i in 0..3 {
        wr::<u32>(veh, 0x3EC + 4 * i, coors.wrapping_index(i as i32).read());
    }
    let p = entity_pos(veh);
    let water = (for_boats & 0xFFFF_FF00) | u32::from(rd::<u16>(veh, 0x22) == 0x21B);
    let one_side = u32::from((rd::<u8>(veh, 0x3DB) >> 6) & 1);
    let empty = g::<u32>(EMPTY_NODE_ADDRESS).read();
    let far = f32::from_bits(0x4974_23FE);
    do_path_search(
        g(0x96F050),
        0,
        rd::<f32>(p, 0),
        rd::<f32>(p, 4),
        rd::<f32>(p, 8),
        empty,
        f32::from_bits(coors.read()),
        f32::from_bits(coors.wrapping_index(1).read()),
        f32::from_bits(coors.wrapping_index(2).read()),
        veh.byte_add(0x3F8).cast(),
        veh.byte_add(0x418).cast(),
        8,
        GPtr::NULL,
        far,
        GPtr::NULL,
        far,
        one_side,
        g::<u32>(EMPTY_NODE_ADDRESS).read(),
        water,
        for_boats,
    );
    let p = entity_pos(veh);
    remove_bad_start_node(
        g(THE_PATHS_FROM_VM),
        rd::<f32>(p, 0),
        rd::<f32>(p, 4),
        veh.byte_add(0x3F8).cast(),
        veh.byte_add(0x418).cast(),
    );
    if rd::<i16>(veh, 0x418) >= 2 {
        let ap = veh.byte_add(0x390);
        wr::<u32>(ap, 0, rd::<u32>(veh, 0x3F8));
        wr::<u16>(veh, 0x398, 0xFFFF);
        auto_pilot_remove_one_path_node(ap);
        wr::<u32>(veh, 0x394, rd::<u32>(veh, 0x3F8));
        auto_pilot_remove_one_path_node(ap);
        find_links_to_go_with_these_nodes(veh);
        wr::<u8>(veh, 0x3B7, 0);
        wr::<u8>(veh, 0x3B8, 0);
        0
    } else {
        join_car_with_road_system(veh);
        wr::<i16>(veh, 0x418, 0);
        1
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_JOIN_CAR_WITH_ROAD_SYSTEM_GOTO_COORS];
