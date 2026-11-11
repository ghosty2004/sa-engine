//! `CPathFind`, `CPathNode`, `CNodeAddress` and the path helpers (PORT-POP-2). Ported from the 1.0 US bytes;
//! gta-reversed and `docs/re/specs/POP/*.md` as reading aids.
//!
//! `ThePaths` (0x96F050, 0x3C80 bytes) is reached through `this` with byte offsets (no global covers it):
//! +0x004 the 512 hash buckets, +0x804 `m_pPathNodes[72]`, +0x924 `m_pNaviNodes[72]`, +0xA44 `m_pNodeLinks[72]`,
//! +0xB64 `m_pLinkLengths[72]`, +0xC84 `m_pPathIntersections[72]`, +0xDA4 `m_pNaviLinks[64]`, +0xEA4
//! `m_aTempNodes[64]`, +0xFA4 `m_anNumNodes[72]`, +0x10C4 `m_anNumVehicleNodes[72]`, +0x11E4 `m_anNumPedNodes[72]`,
//! +0x3544 hash-table count, +0x3548 `m_interiorIDs[8]`, +0x3568 `m_nNumNodeSwitches`, +0x356C `m_aNodeSwitches[64]`
//! (0x1C each), +0x3C6C `m_loadAreaRequestPending`, +0x3C70 the requested area (4 f32).
//! A `CNodeAddress` is a dword: low word area (0xFFFF = none), high word node index. `CPathNode` is 0x1C bytes.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{at, ftol32, g, jp5, rd, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// `CPathFind::NumTempExternalNodes` (zeroed by `Init`)
    pub static NUM_TEMP_EXTERNAL_NODES @ 0x96F02C: i32 = 0;
    /// `CPathFind::bLoadSceneForPathNodes` (set by `SetPathsNeededAtPosition`)
    pub static LOAD_SCENE_FOR_PATH_NODES @ 0x96F030: u8 = 0;
    /// `CPathFind::bInteriorBeingBuilt` (set by `StartNewInterior`)
    pub static INTERIOR_BEING_BUILT @ 0x96F031: u8 = 0;
    /// `CPathFind::LoadSceneForPathNodesCoors` (`SetPathsNeededAtPosition`)
    pub static LOAD_SCENE_COORS @ 0x977B70: [f32; 3] = [0.0; 3];
    /// `ToBeStreamed[8][8]` (x * 8 + y): the areas `MarkRegionsForCoors` asks for
    pub static TO_BE_STREAMED @ 0x96EFD0: [u8; 64] = [0; 64];
    /// `ToBeStreamedForScript[8][8]` (requested by `MakeRequestForNodesToBeLoaded`)
    pub static TO_BE_STREAMED_FOR_SCRIPT @ 0x96EF90: [u8; 64] = [0; 64];
    /// `CPathFind::NumNodesGiven` (nodes of the interior being built)
    pub static NUM_NODES_GIVEN @ 0x96EF80: i32 = 0;
    /// `CPathFind::NewInteriorSlot`
    pub static NEW_INTERIOR_SLOT @ 0x96EF84: i32 = 0;
    /// `CPathFind::InteriorIDBeingBuilt`
    pub static INTERIOR_ID_BEING_BUILT @ 0x96EF88: i32 = 0;
    /// `CPathFind::NumLinksToExteriorNodes`
    pub static NUM_LINKS_TO_EXTERIOR_NODES @ 0x96EAB8: i32 = 0;
    /// `CPathFind::aInteriorNodeLinkedToExterior[8]`
    pub static INTERIOR_NODE_LINKED_TO_EXTERIOR @ 0x96EA98: [i32; 8] = [0; 8];
    /// `CPathFind::aExteriorNodeLinkedTo[8]` (`CNodeAddress`, filled by `__init_aExteriorNodeLinkedTo`)
    pub static EXTERIOR_NODE_LINKED_TO @ 0x977B7C: [u32; 8] = [0; 8];
    /// `ConnectsToGiven`: 64 rows of 6 int8 (the bytes use stride 6; gr declares 8)
    pub static CONNECTS_TO_GIVEN @ 0x96EAC0: [i8; 0x180] = [0; 0x180];
    /// `DontWanderGiven[64]` (`AddNodeToNewInterior`)
    pub static DONT_WANDER_GIVEN @ 0x96EC40: [u8; 64] = [0; 64];
    /// `ZCoorGiven[64]`, `YCoorGiven[64]`, `XCoorGiven[64]` (0x96EC80 / 0x96ED80 / 0x96EE80)
    pub static COOR_GIVEN @ 0x96EC80: [f32; 192] = [0.0; 192];
    /// `aNodesToBeCleared[5000]` (`DoPathSearch`; filled by `__init_aNodesToBeCleared`)
    pub static NODES_TO_BE_CLEARED @ 0x972CD0: [u32; 5000] = [0; 5000];
    /// the int16 path count `TestCoorsCloseness` passes to `DoPathSearch` (scratch)
    pub static PATH_SEARCH_SCRATCH_COUNT @ 0x96F010: i16 = 0;
    /// the "no node" `CNodeAddress` `ComputeRoute` / `TestCoorsCloseness` pass as the forbidden node
    pub static FORBIDDEN_NONE @ 0x8A5F44: u32 = 0xFFFF;
    /// the rotating index of `FindParkingNodeInArea`
    pub static PARKING_NODE_INDEX @ 0x977B9C: i32 = 0;
    /// the 32 `CNodeAddress` filled by `__init_g_977af0`
    pub static NODE_ADDRESSES_977AF0 @ 0x977AF0: [u32; 32] = [0; 32];
}

// ---------------------------------------------------------------------------------------------------------
// field helpers

const PATH_NODES: isize = 0x804;
const NAVI_NODES: isize = 0x924;
const NODE_LINKS: isize = 0xA44;
const LINK_LENGTHS: isize = 0xB64;
const INTERSECTIONS: isize = 0xC84;
const NAVI_LINKS: isize = 0xDA4;
const TEMP_NODES: isize = 0xEA4;
const NUM_NODES: isize = 0xFA4;
const NUM_VEHICLE_NODES: isize = 0x10C4;
const NUM_PED_NODES: isize = 0x11E4;
const HASH_COUNT: isize = 0x3544;
const INTERIOR_IDS: isize = 0x3548;
const NUM_NODE_SWITCHES: isize = 0x3568;
const NODE_SWITCHES: isize = 0x356C;
const LOAD_AREA_PENDING: isize = 0x3C6C;
const LOAD_AREA: isize = 0x3C70;

/// `0.125f` (0x858C48): the node coordinate scale.
const COORD_SCALE: f32 = 0.125;
/// `-3000.0f` (0x859A90)
const REGION_ORIGIN: f32 = -3000.0;
/// `0.0013333333f` (0x859A8C) = 1 / 750
fn region_scale() -> f32 {
    f32::from_bits(0x3AAE_C33E)
}

/// `ThePaths`.
pub fn the_paths() -> GPtr<u8> {
    g(0x96F050)
}

/// Element `i` of one of the `this`-relative per-area arrays (dwords).
fn area_field<T: sa_core::mem::GameData>(this: GPtr<u8>, base: isize, i: i32) -> GPtr<T> {
    at::<T>(this, base + 4 * i as isize)
}

/// `m_pPathNodes[area]`.
fn nodes(this: GPtr<u8>, area: i32) -> GPtr<u8> {
    area_field::<GPtr<u8>>(this, PATH_NODES, area).read()
}

fn area_of(a: u32) -> i32 {
    (a & 0xFFFF) as i32
}

fn index_of(a: u32) -> i32 {
    (a >> 16) as i32
}

/// `&m_pPathNodes[addr.area][addr.node]` (no load check).
fn node_of(this: GPtr<u8>, a: u32) -> GPtr<u8> {
    nodes(this, area_of(a)).byte_add(index_of(a) as isize * 0x1C)
}

fn num_links(n: GPtr<u8>) -> i32 {
    i32::from(rd::<u8>(n, 0x18) & 0xF)
}

/// `m_wBaseLinkId + l`.
fn link_slot(n: GPtr<u8>, l: i32) -> i32 {
    i32::from(rd::<i16>(n, 0x10)).wrapping_add(l)
}

/// `m_pNodeLinks[n.area][n.base + l]`.
fn node_link(this: GPtr<u8>, n: GPtr<u8>, l: i32) -> u32 {
    let area = i32::from(rd::<u16>(n, 0x12));
    area_field::<GPtr<u32>>(this, NODE_LINKS, area).read().wrapping_index(link_slot(n, l)).read()
}

/// `m_pNaviLinks[n.area][n.base + l]` (u16: area `v >> 10`, index `v & 0x3FF`).
fn navi_link(this: GPtr<u8>, n: GPtr<u8>, l: i32) -> u32 {
    let area = i32::from(rd::<u16>(n, 0x12));
    u32::from(area_field::<GPtr<u16>>(this, NAVI_LINKS, area).read().wrapping_index(link_slot(n, l)).read())
}

/// `m_pLinkLengths[n.area][n.base + l]`.
fn link_length(this: GPtr<u8>, n: GPtr<u8>, l: i32) -> u8 {
    let area = i32::from(rd::<u16>(n, 0x12));
    area_field::<GPtr<u8>>(this, LINK_LENGTHS, area).read().wrapping_index(link_slot(n, l)).read()
}

/// `&m_pNaviNodes[v >> 10][v & 0x3FF]`.
fn navi_node(this: GPtr<u8>, v: u32) -> GPtr<u8> {
    area_field::<GPtr<u8>>(this, NAVI_NODES, (v >> 10) as i32)
        .read()
        .byte_add((v & 0x3FF) as isize * 0xE)
}

/// `fild word; fmul 0.125f` of a node coordinate (+8 / +0xA / +0xC).
fn coord(n: GPtr<u8>, off: isize) -> X87 {
    xi(i32::from(rd::<i16>(n, off))).fmul_m(COORD_SCALE)
}

/// FABS.
fn fabs(v: X87) -> X87 {
    if v.to_f64().is_sign_negative() { v.fchs() } else { v }
}

/// `fcomp` + `test ah, 5` + `jnp` taken: `a < b` (C0 alone).
fn lt(a: X87, b: X87) -> bool {
    !jp5(a, b)
}

/// `max(ftol(v), 0)` (the `setle; dec; and` sequence) clamped to 7 (`cmp 7; jl`).
fn clamp_region(v: X87) -> i32 {
    let r = ftol32(v);
    let r = if r <= 0 { 0 } else { r };
    if r < 7 { r } else { 7 }
}

/// `(c - -3000.0f) * 0.0013333333f`, the region of one coordinate.
fn region_value(c: X87) -> i32 {
    clamp_region(c.fsub_m(REGION_ORIGIN).fmul_m(region_scale()))
}

orig! {
    /// `CMemoryMgr::Free(void*)` (0x72F430)
    pub fn mem_free(p: GPtr<u8>) = cdecl 0x72F430 => unported;
    /// CRT `_atexit` (0x821D1E)
    pub fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// the constructor of the 0x977BE8 object (0x4541E0, thiscall, returns `this`)
    pub fn ctor_977be8(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4541E0 => unported;
    /// the destructor body `__dtor_g_977be8` jumps to (0x454200, thiscall)
    pub fn dtor_977be8_body(this: GPtr<u8>) = thiscall 0x454200 => unported;
    /// `CCollision::DistToLineSqr(const CVector* start, const CVector* end, const CVector* point)` (0x412850)
    pub fn dist_to_line_sqr_st0(a: GPtr<f32>, b: GPtr<f32>, p: GPtr<f32>) -> f64 = cdecl 0x412850 => unported;
    /// `CGenericGameStorage::SaveDataToWorkBuffer(void*, int32)` (0x5D1270)
    pub fn save_data_to_work_buffer(data: GPtr<u8>, size: i32) = cdecl 0x5D1270 => unported;
    /// `CGenericGameStorage::LoadDataFromWorkBuffer(void*, int32)` (0x5D1300)
    pub fn load_data_from_work_buffer(data: GPtr<u8>, size: i32) = cdecl 0x5D1300 => unported;

    /// `CPathFind::CalcDistToAnyConnectingLinks(CPathNode*, CVector)` (0x44F190)
    pub fn calc_dist_to_any_connecting_links_call_st0(this: GPtr<u8>, node: GPtr<u8>, px: f32, py: f32, pz: f32) -> f64 = thiscall 0x44F190 => calc_dist_to_any_connecting_links_f64;
    /// `CPathFind::FindNodeClosestInRegion(...)` (0x44F2C0)
    pub fn find_node_closest_in_region_call(this: GPtr<u8>, out: GPtr<u32>, region: u32, px: f32, py: f32, pz: f32, node_type: u32, dist: GPtr<f32>, low_traffic: u32, unkn: u32, boats: u32, unused: u32) = thiscall 0x44F2C0 => find_node_closest_in_region;
    /// `CPathFind::FindNodeClosestToCoors(CVector, ...)` (0x44F460)
    pub fn find_node_closest_to_coors_call(this: GPtr<u8>, out: GPtr<u32>, px: f32, py: f32, pz: f32, node_type: u32, max_dist: f32, low_traffic: u32, unused: u32, unkn: u32, boats: u32, ignore_interior: u32) -> GPtr<u32> = thiscall 0x44F460 => find_node_closest_to_coors;
    /// `CPathFind::CountNeighboursToBeSwitchedOff(const CPathNode&)` (0x4504F0)
    pub fn count_neighbours_to_be_switched_off_call(this: GPtr<u8>, node: GPtr<u8>) -> i32 = thiscall 0x4504F0 => count_neighbours_to_be_switched_off;
    /// `CPathFind::UpdateStreaming(bool)` (0x450A60)
    pub fn update_streaming_call(this: GPtr<u8>, force: u32) = thiscall 0x450A60 => update_streaming;
    /// `CPathFind::AreNodesLoadedForArea(float, float, float, float)` (0x44DD10)
    pub fn are_nodes_loaded_for_area_call(this: GPtr<u8>, x0: f32, x1: f32, y0: f32, y1: f32) -> u8 = thiscall 0x44DD10 => are_nodes_loaded_for_area;
    /// `CPathFind::SwitchRoadsOffInArea(...)` (0x452C80)
    pub fn switch_roads_off_in_area_call(this: GPtr<u8>, x0: f32, x1: f32, y0: f32, y1: f32, z0: f32, z1: f32, off: u32, cars: u32, back: u32) = thiscall 0x452C80 => switch_roads_off_in_area;
}

// ---------------------------------------------------------------------------------------------------------
// CPathNode / CNodeAddress / CCarPathLink leaves

/// `CPathNode::GetPosition(CVector* out)` (0x420A10, thiscall, `ret 4`): returns `out`.
#[hook(addr = 0x420A10, conv = thiscall, name = "CPathNode::GetPosition", expect = "51 0F BF 41 0C", this = "CPathNode")]
pub fn get_position(node: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> {
    let z = coord(node, 0xC);
    let y = coord(node, 0xA);
    let xv = coord(node, 0x8);
    out.write(xv.to_f32());
    out.wrapping_index(1).write(y.to_f32());
    out.wrapping_index(2).write(z.to_f32());
    out
}

/// `CNodeAddress::operator==` (stub 0x420920 → 0x1565640): returns EAX 1 / 0.
#[hook(addr = 0x420920, conv = thiscall, name = "thunk_FUN_01565640", expect = "E9 1B 4D 14 01", this = "CNodeAddress")]
pub fn node_address_eq(a: GPtr<u16>, b: GPtr<u16>) -> u32 {
    u32::from(a.read() == b.read() && a.wrapping_index(1).read() == b.wrapping_index(1).read())
}

/// `CNodeAddress::operator!=` (stub 0x420950 → 0x156AB40).
#[hook(addr = 0x420950, conv = thiscall, name = "thunk_FUN_0156ab40", expect = "E9 EB A1 14 01", this = "CNodeAddress")]
pub fn node_address_ne(a: GPtr<u16>, b: GPtr<u16>) -> u32 {
    u32::from(!(a.read() == b.read() && a.wrapping_index(1).read() == b.wrapping_index(1).read()))
}

/// `CNodeAddress::operator<` (stub 0x420980 → 0x156E7E0): area first, then node (unsigned).
#[hook(addr = 0x420980, conv = thiscall, name = "thunk_FUN_0156e7e0", expect = "E9 5B DE 14 01", this = "CNodeAddress")]
pub fn node_address_lt(a: GPtr<u16>, b: GPtr<u16>) -> u32 {
    let (aa, ba) = (a.read(), b.read());
    if aa < ba {
        return 1;
    }
    if aa != ba {
        return 0;
    }
    u32::from(a.wrapping_index(1).read() < b.wrapping_index(1).read())
}

/// `CompressedVector2D` → `CVector` (0x420A60, thiscall, `ret 4`; body continues through 0x401492):
/// `out = (x * 0.125, y * 0.125, 0)`, returns `out`.
#[hook(addr = 0x420A60, conv = thiscall, name = "FUN_00420a60", expect = "51 0F BF 41 02")]
pub fn compressed_vector2d_uncompress(v: GPtr<i16>, out: GPtr<f32>) -> GPtr<f32> {
    out.wrapping_index(2).write(0.0);
    let y = xi(i32::from(v.wrapping_index(1).read())).fmul_m(COORD_SCALE);
    let xv = xi(i32::from(v.read())).fmul_m(COORD_SCALE);
    out.write(xv.to_f32());
    out.wrapping_index(1).write(y.to_f32());
    out
}

/// `CNodeAddress::CNodeAddress()` (0x4241A0): area word = 0xFFFF, returns `this`. Ported, unhookable: protection
/// code reads its bytes as data (0x1465802 `call [0x4241A4]`, 0x147358B `mov [0x4241A0], 3`).
pub fn node_address_ctor(a: GPtr<u16>) -> GPtr<u16> {
    a.write(0xFFFF);
    a
}

/// `CNodeAddress::CNodeAddress()` (0x4241B0, a second copy of 0x4241A0). Ported, unhookable: protection code
/// writes its bytes (0x14735AE `mov [0x4241B0], 4`).
pub fn node_address_ctor2(a: GPtr<u16>) -> GPtr<u16> {
    a.write(0xFFFF);
    a
}

/// `CCarPathLink::OneWayLaneOffset()` (0x44DB00, thiscall, ST0): follows the bytes (spec CCarPathLink.md):
/// opposite == 0 → `0.5 - same*0.5`; same == 0 → `0.5 - opposite*0.5`; else `width_u8 * 0.011574074 + 0.5`.
#[hook(addr = 0x44DB00, conv = thiscall, name = "CCarPathLink::OneWayLaneOffset", expect = "51 8A 41 0B 8A D0", this = "CCarPathLink")]
pub fn one_way_lane_offset(link: GPtr<u8>) -> X87 {
    let b = rd::<u8>(link, 0xB);
    let opposite = i32::from(b & 7);
    let same = i32::from((b >> 3) & 7);
    if opposite == 0 {
        return xi(same).fmul_m(0.5).fsubr_m(0.5);
    }
    if b & 0x38 == 0 {
        return xi(opposite).fmul_m(0.5).fsubr_m(0.5);
    }
    xi(i32::from(rd::<u8>(link, 0xA)))
        .fmul_m(f32::from_bits(0x3C3D_A12F))
        .fadd_m(0.5)
}

// ---------------------------------------------------------------------------------------------------------
// CPathFind: small members

/// `CPathFind::IsAreaNodesAvailable(CNodeAddress)` (0x420AA0, thiscall, `ret 4`).
#[hook(addr = 0x420AA0, conv = thiscall, name = "CPathFind::IsAreaNodesAvailable", expect = "0F B7 44 24 04", this = "CPathFind")]
pub fn is_area_nodes_available(this: GPtr<u8>, a: u32) -> u8 {
    u8::from(!nodes(this, area_of(a)).is_null())
}

/// `CPathFind::GetPathNode(CNodeAddress)` (0x420AC0, thiscall, `ret 4`): no load check.
#[hook(addr = 0x420AC0, conv = thiscall, name = "CPathFind::GetPathNode", expect = "8B 44 24 04 0F B7 54 24 04", this = "CPathFind")]
pub fn get_path_node(this: GPtr<u8>, a: u32) -> GPtr<u8> {
    node_of(this, a)
}

/// `CPathFind::Init()` (stub 0x44D080 → 0x1560DC0). Keeps the original's out-of-bounds writes: the 64-entry
/// arrays at +0xDA4 / +0xEA4 are zeroed for 72 entries, so `m_aTempNodes[0..7]` and `m_anNumNodes[0..7]` are
/// zeroed too (CPathFind.deviations.md, keep-original).
#[hook(addr = 0x44D080, conv = thiscall, name = "CPathFind::Init", expect = "E9 3B 3D 11 01", this = "CPathFind")]
pub fn init(this: GPtr<u8>) {
    NUM_TEMP_EXTERNAL_NODES.set(0);
    wr::<i32>(this, NUM_NODE_SWITCHES, 0);
    wr::<u8>(this, LOAD_AREA_PENDING, 0);
    for i in 0..0x48 {
        for base in [PATH_NODES, NAVI_NODES, NODE_LINKS, LINK_LENGTHS, INTERSECTIONS, NAVI_LINKS, TEMP_NODES] {
            area_field::<u32>(this, base, i).write(0);
        }
    }
    for i in 0..8 {
        area_field::<i32>(this, INTERIOR_IDS, i).write(-1);
    }
}

/// `CPathFind::UnLoadPathFindData(int32 area)` (stub 0x44D0F0 → 0x1563FB0): frees nodes, navi nodes, links,
/// navi links, link lengths, intersections (in that order) and clears the six pointers.
#[hook(addr = 0x44D0F0, conv = thiscall, name = "CPathFind::UnLoadPathFindData", expect = "E9 BB 6E 11 01", this = "CPathFind")]
pub fn unload_path_find_data(this: GPtr<u8>, area: i32) {
    let order = [PATH_NODES, NAVI_NODES, NODE_LINKS, NAVI_LINKS, LINK_LENGTHS, INTERSECTIONS];
    for base in order {
        let p = area_field::<GPtr<u8>>(this, base, area).read();
        if !p.is_null() {
            mem_free(p);
        }
    }
    for base in order {
        area_field::<GPtr<u8>>(this, base, area).write(GPtr::NULL);
    }
}

/// `CPathFind::RemoveNodeFromList(CPathNode*)` (0x44D1B0): `*prev = next; if next: next.prev = prev; count--`.
#[hook(addr = 0x44D1B0, conv = thiscall, name = "CPathFind::RemoveNodeFromList", expect = "8B 44 24 04 8B 50 04", this = "CPathFind")]
pub fn remove_node_from_list(this: GPtr<u8>, node: GPtr<u8>) {
    unlink(this, node);
}

fn unlink(this: GPtr<u8>, node: GPtr<u8>) {
    let prev = rd::<GPtr<GPtr<u8>>>(node, 4);
    let next = rd::<GPtr<u8>>(node, 0);
    prev.write(next);
    if !next.is_null() {
        wr(next, 4, prev);
    }
    let c = at::<i32>(this, HASH_COUNT);
    c.write(c.read().wrapping_sub(1));
}

/// `CPathFind::AddNodeToList(CPathNode*, int32 dist)` (0x44D1E0): head insert into bucket `(int16)(dist & 0x1FF)`,
/// `+0xE = (int16)dist` (no check; gr's assert is harmless), count++.
#[hook(addr = 0x44D1E0, conv = thiscall, name = "CPathFind::AddNodeToList", expect = "8B 54 24 04 56", this = "CPathFind")]
pub fn add_node_to_list(this: GPtr<u8>, node: GPtr<u8>, dist: i32) {
    link_into(this, node, dist);
}

fn link_into(this: GPtr<u8>, node: GPtr<u8>, dist: i32) {
    let b = i32::from((dist & 0x1FF) as i16);
    let slot = at::<GPtr<u8>>(this, 4 + 4 * b as isize);
    wr(node, 0, slot.read());
    wr(node, 4, slot);
    let head = slot.read();
    if !head.is_null() {
        wr(head, 4, node);
    }
    slot.write(node);
    wr::<i16>(node, 0xE, dist as i16);
    let c = at::<i32>(this, HASH_COUNT);
    c.write(c.read().wrapping_add(1));
}

/// `CPathFind::TidyUpNodeSwitchesAfterMission()` (stub 0x44D3B0 → 0x156DE10): `m_nNumNodeSwitches = min(n, 0x36)`.
#[hook(addr = 0x44D3B0, conv = thiscall, name = "CPathFind::TidyUpNodeSwitchesAfterMission", expect = "E9 5B 0A 12 01", this = "CPathFind")]
pub fn tidy_up_node_switches_after_mission(this: GPtr<u8>) {
    let n = rd::<i32>(this, NUM_NODE_SWITCHES);
    wr(this, NUM_NODE_SWITCHES, if n < 0x36 { n } else { 0x36 });
}

/// `CPathFind::ThisNodeHasToBeSwitchedOff(CPathNode*)` (stub 0x44D3E0 → 0x1561710): behaviour type
/// (`+0x1A >> 4`) not 1 / 2. ECX is not read.
#[hook(addr = 0x44D3E0, conv = thiscall, name = "CPathFind::ThisNodeHasToBeSwitchedOff", expect = "E9 2B 43 11 01", this = "CPathFind")]
pub fn this_node_has_to_be_switched_off(_this: GPtr<u8>, node: GPtr<u8>) -> u8 {
    let t = rd::<u8>(node, 0x1A) >> 4;
    u8::from(!(t == 1 || t == 2))
}

/// `CPathFind::FindRegionForCoors(float x, float y)` (0x44D830): the bytes' multiply by 0.0013333333f and signed
/// clamp (gr divides by 750 and converts unsigned: deviations, keep-original). ECX is not read.
#[hook(addr = 0x44D830, conv = thiscall, name = "CPathFind::FindRegionForCoors", expect = "D9 44 24 04 56", this = "CPathFind")]
pub fn find_region_for_coors(_this: GPtr<u8>, px: f32, py: f32) -> i32 {
    let rx = region_value(x(px));
    let ry = region_value(x(py));
    rx + ry * 8
}

/// `CPathFind::FindXRegionForCoors(float)` (0x44D890).
#[hook(addr = 0x44D890, conv = thiscall, name = "CPathFind::FindXRegionForCoors", expect = "D9 44 24 04 D8 25 90 9A 85 00", this = "CPathFind")]
pub fn find_x_region_for_coors(_this: GPtr<u8>, c: f32) -> i32 {
    region_value(x(c))
}

/// `CPathFind::FindYRegionForCoors(float)` (0x44D8C0).
#[hook(addr = 0x44D8C0, conv = thiscall, name = "CPathFind::FindYRegionForCoors", expect = "D9 44 24 04 D8 25 90 9A 85 00", this = "CPathFind")]
pub fn find_y_region_for_coors(_this: GPtr<u8>, c: f32) -> i32 {
    region_value(x(c))
}

/// `r * 750.0f - 3000.0f`.
fn region_start(r: i32) -> X87 {
    xi(r).fmul_m(750.0).fsub_m(3000.0)
}

/// `CPathFind::FindXCoorsForRegion(int32)` (0x44D8F0, ST0).
#[hook(addr = 0x44D8F0, conv = thiscall, name = "CPathFind::FindXCoorsForRegion", expect = "DB 44 24 04 D8 0D 98 9A 85 00", this = "CPathFind")]
pub fn find_x_coors_for_region(_this: GPtr<u8>, r: i32) -> X87 {
    region_start(r)
}

/// `CPathFind::FindYCoorsForRegion(int32)` (0x44D910, ST0).
#[hook(addr = 0x44D910, conv = thiscall, name = "CPathFind::FindYCoorsForRegion", expect = "DB 44 24 04 D8 0D 98 9A 85 00", this = "CPathFind")]
pub fn find_y_coors_for_region(_this: GPtr<u8>, r: i32) -> X87 {
    region_start(r)
}

/// `CPathFind::FindStartPointOfRegion(int32 rx, int32 ry, float* outX, float* outY)` (0x44D930).
#[hook(addr = 0x44D930, conv = thiscall, name = "CPathFind::FindStartPointOfRegion-", expect = "DB 44 24 04 8B 44 24 0C", this = "CPathFind")]
pub fn find_start_point_of_region(_this: GPtr<u8>, rx: i32, ry: i32, out_x: GPtr<f32>, out_y: GPtr<f32>) {
    out_x.write(region_start(rx).to_f32());
    out_y.write(region_start(ry).to_f32());
}

/// `CPathFind::MarkRegionsForCoors(CVector pos, float radius)` (0x44DB60, `ret 0x10`, ECX not read): sets
/// `ToBeStreamed[x][y] = 1` for the regions of `pos.x ± r`, `pos.y ± r` (the extra range checks in the bytes are
/// dead after the clamp).
#[hook(addr = 0x44DB60, conv = thiscall, name = "CPathFind::MarkRegionsForCoors", expect = "D9 44 24 04 53", this = "CPathFind")]
pub fn mark_regions_for_coors(_this: GPtr<u8>, px: f32, py: f32, _pz: f32, r: f32) {
    let x0 = region_value(x(px).fsub_m(r));
    let x1 = region_value(x(px).fadd_m(r));
    let y0 = region_value(x(py).fsub_m(r));
    let y1 = region_value(x(py).fadd_m(r));
    let t = TO_BE_STREAMED.ptr().cast::<u8>();
    for xx in x0..=x1 {
        for yy in y0..=y1 {
            t.wrapping_index(xx * 8 + yy).write(1);
        }
    }
}

/// `CPathFind::SetPathsNeededAtPosition(const CVector&)` (stub 0x44DCD0 → 0x156DE80).
#[hook(addr = 0x44DCD0, conv = thiscall, name = "CPathFind::SetPathsNeededAtPosition", expect = "E9 AB 01 12 01", this = "CPathFind")]
pub fn set_paths_needed_at_position(_this: GPtr<u8>, pos: GPtr<f32>) {
    LOAD_SCENE_COORS.set([pos.read(), pos.wrapping_index(1).read(), pos.wrapping_index(2).read()]);
    LOAD_SCENE_FOR_PATH_NODES.set(1);
}

/// `CPathFind::ReleaseRequestedNodes()` (stub 0x44DD00 → 0x1561B10).
#[hook(addr = 0x44DD00, conv = thiscall, name = "CPathFind::ReleaseRequestedNodes", expect = "E9 0B 3E 11 01", this = "CPathFind")]
pub fn release_requested_nodes(this: GPtr<u8>) {
    wr::<u8>(this, LOAD_AREA_PENDING, 0);
}

/// `CPathFind::StartNewInterior(int32 id)` (stub 0x44DE80 → 0x156AD40): the free interior slot is the first
/// `m_interiorIDs[i] == -1` (no bound, as the bytes).
#[hook(addr = 0x44DE80, conv = thiscall, name = "CPathFind::StartNewInterior", expect = "E9 BB CE 11 01", this = "CPathFind")]
pub fn start_new_interior(this: GPtr<u8>, id: i32) {
    INTERIOR_ID_BEING_BUILT.set(id);
    INTERIOR_BEING_BUILT.set(1);
    NUM_NODES_GIVEN.set(0);
    NUM_LINKS_TO_EXTERIOR_NODES.set(0);
    NEW_INTERIOR_SLOT.set(0);
    let mut i = 0;
    while area_field::<i32>(this, INTERIOR_IDS, i).read() != -1 {
        i += 1;
        NEW_INTERIOR_SLOT.set(i);
    }
}

fn add_one_interior_link(a: i32, b: i32) {
    let row = CONNECTS_TO_GIVEN.ptr().cast::<i8>().wrapping_index(a.wrapping_mul(6));
    let mut i = 0;
    if row.read() >= 0 {
        loop {
            if i >= 6 {
                break;
            }
            let v = row.wrapping_index(i + 1).read();
            i += 1;
            if v < 0 {
                break;
            }
        }
    }
    row.wrapping_index(i).write(b as i8);
}

/// `CPathFind::AddInteriorLink(int32 a, int32 b)` (stub 0x44DED0 → 0x156EC90, `ret 8`, ECX not read): the scan
/// stops at ANY negative byte, capped at index 6 (gr stops only at -1: deviations, keep-original).
#[hook(addr = 0x44DED0, conv = thiscall, name = "CPathFind::AddInteriorLink", expect = "E9 BB 0D 12 01", this = "CPathFind")]
pub fn add_interior_link(_this: GPtr<u8>, a: i32, b: i32) {
    add_one_interior_link(a, b);
    add_one_interior_link(b, a);
}

/// `CPathFind::AddInteriorLinkToExternalNode(int32 node, CNodeAddress ext)` (0x44DF30, ECX not read).
#[hook(addr = 0x44DF30, conv = thiscall, name = "CPathFind::AddInteriorLinkToExternalNode", expect = "A1 B8 EA 96 00", this = "CPathFind")]
pub fn add_interior_link_to_external_node(_this: GPtr<u8>, node: i32, ext: u32) {
    let n = NUM_LINKS_TO_EXTERIOR_NODES.get();
    INTERIOR_NODE_LINKED_TO_EXTERIOR.ptr().cast::<i32>().wrapping_index(n).write(node);
    EXTERIOR_NODE_LINKED_TO.ptr().cast::<u32>().wrapping_index(n).write(ext);
    NUM_LINKS_TO_EXTERIOR_NODES.set(n.wrapping_add(1));
}

/// `CPathFind::ReInit()` (stub 0x44E4E0 → 0x156A600).
#[hook(addr = 0x44E4E0, conv = thiscall, name = "CPathFind::ReInit", expect = "E9 1B C1 11 01", this = "CPathFind")]
pub fn re_init(this: GPtr<u8>) {
    wr::<u8>(this, LOAD_AREA_PENDING, 0);
    wr::<i32>(this, NUM_NODE_SWITCHES, 0);
}

/// `CPathFind::MakeRequestForNodesToBeLoaded(float, float, float, float)` (stub 0x450D70 → 0x156A6B0).
#[hook(addr = 0x450D70, conv = thiscall, name = "CPathFind::MakeRequestForNodesToBeLoaded", expect = "E9 3B 99 11 01", this = "CPathFind")]
pub fn make_request_for_nodes_to_be_loaded(this: GPtr<u8>, x0: f32, x1: f32, y0: f32, y1: f32) {
    wr(this, LOAD_AREA, x0);
    wr(this, LOAD_AREA + 4, x1);
    wr::<u8>(this, LOAD_AREA_PENDING, 1);
    wr(this, LOAD_AREA + 8, y0);
    wr(this, LOAD_AREA + 0xC, y1);
    update_streaming_call(this, 1);
}

/// `CPathFind::HaveRequestedNodesBeenLoaded()` (0x450DB0).
#[hook(addr = 0x450DB0, conv = thiscall, name = "CPathFind::HaveRequestedNodesBeenLoaded", expect = "8B 81 7C 3C 00 00", this = "CPathFind")]
pub fn have_requested_nodes_been_loaded(this: GPtr<u8>) -> u8 {
    are_nodes_loaded_for_area_call(
        this,
        rd(this, LOAD_AREA),
        rd(this, LOAD_AREA + 4),
        rd(this, LOAD_AREA + 8),
        rd(this, LOAD_AREA + 0xC),
    )
}

/// `CPathFind::SwitchPedRoadsOffInArea(x0, x1, y0, y1, z0, z1, bool off, bool back)` (stub 0x452F00 → 0x1565860):
/// `SwitchRoadsOffInArea(..., off, cars = 0, back)`.
#[hook(addr = 0x452F00, conv = thiscall, name = "CPathFind::SwitchPedRoadsOffInArea", expect = "E9 5B 29 11 01", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn switch_ped_roads_off_in_area(this: GPtr<u8>, x0: f32, x1: f32, y0: f32, y1: f32, z0: f32, z1: f32, off: u32, back: u32) {
    switch_roads_off_in_area_call(this, x0, x1, y0, y1, z0, z1, off, 0, back);
}

/// `CPathFind::Save()` (0x5D34C0): the switch count, then each 0x1C-byte switch.
#[hook(addr = 0x5D34C0, conv = thiscall, name = "CPathFind::Save", expect = "53 56 57 8B D9", this = "CPathFind")]
pub fn save(this: GPtr<u8>) -> u8 {
    save_data_to_work_buffer(at::<u8>(this, NUM_NODE_SWITCHES), 4);
    let mut i = 0;
    while i < rd::<i32>(this, NUM_NODE_SWITCHES) {
        save_data_to_work_buffer(at::<u8>(this, NODE_SWITCHES + 0x1C * i as isize), 0x1C);
        i += 1;
    }
    1
}

/// `CPathFind::Load()` (0x5D3500).
#[hook(addr = 0x5D3500, conv = thiscall, name = "CPathFind::Load", expect = "53 56 57 8B D9", this = "CPathFind")]
pub fn load(this: GPtr<u8>) -> u8 {
    load_data_from_work_buffer(at::<u8>(this, NUM_NODE_SWITCHES), 4);
    let mut i = 0;
    while i < rd::<i32>(this, NUM_NODE_SWITCHES) {
        load_data_from_work_buffer(at::<u8>(this, NODE_SWITCHES + 0x1C * i as isize), 0x1C);
        i += 1;
    }
    1
}

/// `__init_g_977af0` (0x84A060): 32 `CNodeAddress` at 0x977AF0 get area 0xFFFF.
#[hook(addr = 0x84A060, conv = cdecl, name = "__init_g_977af0", expect = "B8 F0 7A 97 00")]
pub fn init_g_977af0() {
    let p = NODE_ADDRESSES_977AF0.ptr().cast::<u16>();
    for k in 0..32 {
        p.wrapping_index(2 * k).write(0xFFFF);
    }
}

/// `__init_g_977be8` (0x84A220): constructs the object at 0x977BE8 (0x4541E0) and registers its destructor.
#[hook(addr = 0x84A220, conv = cdecl, name = "__init_g_977be8", expect = "B9 E8 7B 97 00")]
pub fn init_g_977be8() {
    ctor_977be8(g(0x977BE8));
    atexit(0x856150);
}

/// `__dtor_g_977be8` (0x856150): `jmp 0x454200` with ECX = 0x977BE8.
#[hook(addr = 0x856150, conv = cdecl, name = "__dtor_g_977be8", expect = "B9 E8 7B 97 00")]
pub fn dtor_g_977be8() {
    dtor_977be8_body(g(0x977BE8));
}

// ---------------------------------------------------------------------------------------------------------
// CPathFind: distances and closest-node searches

/// `CPathFind::CalcRoadDensity(float x, float y)` (0x44EFC0, ST0): over the vehicle nodes of areas 0..63 within
/// 80 (|dx| and |dy| strictly below, NaN fails) of (x, y), the sum of `len * lanes` of every link whose navi area
/// (`v >> 10`) has nodes loaded (both lane counts), times 0.0004f.
#[hook(addr = 0x44EFC0, conv = thiscall, name = "CPathFind::CalcRoadDensity", expect = "83 EC 18 D9 05 50 8B 85 00", this = "CPathFind")]
pub fn calc_road_density(this: GPtr<u8>, px: f32, py: f32) -> X87 {
    let mut sum = x(0.0);
    for a in 0..0x40 {
        let base = nodes(this, a);
        if base.is_null() {
            continue;
        }
        let count = area_field::<i32>(this, NUM_VEHICLE_NODES, a).read();
        if count <= 0 {
            continue;
        }
        for k in 0..count {
            let n = base.byte_add(k as isize * 0x1C);
            let nx = coord(n, 8);
            let nxf = nx.to_f32();
            if !lt(fabs(nx.fsub_m(px)), x(80.0)) {
                continue;
            }
            let ny = coord(n, 0xA);
            let nyf = ny.to_f32();
            if !lt(fabs(ny.fsub_m(py)), x(80.0)) {
                continue;
            }
            let links = num_links(n);
            for l in 0..links {
                let a2 = node_link(this, n, l);
                if nodes(this, area_of(a2)).is_null() {
                    continue;
                }
                let t = node_of(this, a2);
                let dx = x(nxf).fsub(coord(t, 8));
                let dy = x(nyf).fsub(coord(t, 0xA));
                let len = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
                let v = navi_link(this, n, l);
                if nodes(this, (v >> 10) as i32).is_null() {
                    continue;
                }
                let b = rd::<u8>(navi_node(this, v), 0xB);
                sum = sum.fadd(xi(i32::from(b & 7)).fmul(len));
                sum = sum.fadd(xi(i32::from((b >> 3) & 7)).fmul(len));
            }
        }
    }
    sum.fmul_m(f32::from_bits(0x39D1_B717))
}

/// `CPathFind::CalcDistToAnyConnectingLinks(CPathNode*, CVector)` (0x44F190, ST0): `sqrt` of the smallest
/// `CCollision::DistToLineSqr(node, link, pos)` over the loaded links (start 999999.875f).
#[hook(addr = 0x44F190, conv = thiscall, name = "CPathFind::CalcDistToAnyConnectingLinks", expect = "83 EC 1C 53 56", this = "CPathFind")]
pub fn calc_dist_to_any_connecting_links(this: GPtr<u8>, node: GPtr<u8>, px: f32, py: f32, pz: f32) -> X87 {
    let mut best = f32::from_bits(0x4974_23FE);
    let mut pos = [px, py, pz];
    let mut i = 0;
    if num_links(node) != 0 {
        loop {
            let a = node_link(this, node, i);
            if !nodes(this, area_of(a)).is_null() {
                let l = node_of(this, a);
                let mut nv = [coord(node, 8).to_f32(), coord(node, 0xA).to_f32(), coord(node, 0xC).to_f32()];
                let mut lv = [coord(l, 8).to_f32(), coord(l, 0xA).to_f32(), coord(l, 0xC).to_f32()];
                let r = dist_to_line_sqr(
                    GPtr::from_raw(nv.as_mut_ptr()),
                    GPtr::from_raw(lv.as_mut_ptr()),
                    GPtr::from_raw(pos.as_mut_ptr()),
                );
                if jp5(x(best), r) {
                    best = r.to_f32();
                }
            }
            i += 1;
            if i >= num_links(node) {
                break;
            }
        }
    }
    x(best).fsqrt()
}

/// `CPathFind::FindNodeClosestInRegion(CNodeAddress* out, uint16 region, CVector pos, uint8 type, float* dist,
/// bool lowTraffic, bool unkn, bool boats, bool unused)` (0x44F2C0, entry `sub esp,0x10; push ebx; jmp 0x4074E5`).
/// Type 0 = vehicle nodes, 1 = ped nodes, other = the empty range `[type, type)`. Updates `out` / `*dist` only
/// for a strictly better node: `(|dz|*3 + |dy| + |dx|) * 0.3`, then plus `CalcDistToAnyConnectingLinks * 0.2`.
#[hook(addr = 0x44F2C0, conv = thiscall, name = "CPathFind::FindNodeClosestInRegion", expect = "83 EC 10 53 E9 1C 82 FB FF", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn find_node_closest_in_region(
    this: GPtr<u8>,
    out: GPtr<u32>,
    region: u32,
    px: f32,
    py: f32,
    pz: f32,
    node_type: u32,
    dist: GPtr<f32>,
    low_traffic: u32,
    unkn: u32,
    boats: u32,
    _unused: u32,
) {
    let area = (region & 0xFFFF) as i32;
    if nodes(this, area).is_null() {
        return;
    }
    let (start, end) = match node_type as u8 {
        0 => (0, area_field::<i32>(this, NUM_VEHICLE_NODES, area).read()),
        1 => (
            area_field::<i32>(this, NUM_VEHICLE_NODES, area).read(),
            area_field::<i32>(this, NUM_NODES, area).read(),
        ),
        _ => (node_type as i32, node_type as i32),
    };
    let mut i = start;
    while i < end {
        let n = nodes(this, area).byte_add(i as isize * 0x1C);
        i += 1;
        if low_traffic as u8 != 0 && rd::<u8>(n, 0x18) & 0x20 != 0 {
            continue;
        }
        if unkn as u8 != 0 && rd::<u8>(n, 0x19) & 2 != 0 {
            continue;
        }
        if boats as u8 != rd::<u8>(n, 0x18) >> 7 {
            continue;
        }
        let nx = coord(n, 8).to_f32();
        let ny = coord(n, 0xA);
        let nz = coord(n, 0xC);
        let dz3 = fabs(nz.fsub_m(pz)).fmul_m(3.0);
        let dy = fabs(ny.fsub_m(py));
        let dx = fabs(x(nx).fsub_m(px));
        let d = dz3.fadd(dy).fadd(dx).fmul_m(f32::from_bits(0x3E99_999A));
        let df = d.to_f32();
        if !lt(d, x(dist.read())) {
            continue;
        }
        let r = calc_dist_to_any_connecting_links_call(this, n, px, py, pz)
            .fmul_m(f32::from_bits(0x3E4C_CCCD))
            .fadd_m(df);
        if lt(r, x(dist.read())) {
            dist.write(r.to_f32());
            out.cast::<u16>().write(region as u16);
            out.cast::<u16>().wrapping_index(1).write((i - 1) as u16);
        }
    }
}

/// `CPathFind::FindNodeClosestToCoors(CNodeAddress* out, CVector pos, uint8 type, float maxDist, bool lowTraffic,
/// int32 unused, bool unkn, bool boats, bool ignoreInterior)` (0x44F460, `ret 0x2C`; spec CPathFind.md): own region,
/// then rings 1..4 while the best distance exceeds the border distance, then the interior areas 64..71 unless
/// `ignoreInterior`. The best address local starts with only its area word set (0xFFFF); its node word is
/// uninitialised stack in the original and 0 here.
#[hook(addr = 0x44F460, conv = thiscall, name = "CPathFind::FindNodeClosestToCoors", expect = "83 EC 1C D9 44 24 24", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn find_node_closest_to_coors(
    this: GPtr<u8>,
    out: GPtr<u32>,
    px: f32,
    py: f32,
    pz: f32,
    node_type: u32,
    max_dist: f32,
    low_traffic: u32,
    _unused: u32,
    unkn: u32,
    boats: u32,
    ignore_interior: u32,
) -> GPtr<u32> {
    let mut best = [0x0000_FFFFu32];
    let mut dist = [max_dist];
    let bp = GPtr::from_raw(best.as_mut_ptr());
    let dp = GPtr::from_raw(dist.as_mut_ptr());
    let search = |r: i32, last: u32| {
        find_node_closest_in_region_call(this, bp, r as u32, px, py, pz, node_type, dp, low_traffic, unkn, boats, last);
    };
    let rx = region_value(x(px));
    let ry = region_value(x(py));
    let mut m = region_start(rx).fsubr_m(px).to_f32();
    let c = region_start(rx + 1).fsub_m(px);
    if !lt(x(m), c) {
        m = c.to_f32();
    }
    let c = region_start(ry).fsubr_m(py);
    if !lt(x(m), c) {
        m = c.to_f32();
    }
    let c = region_start(ry + 1).fsub_m(py);
    if !lt(x(m), c) {
        m = c.to_f32();
    }
    search(rx + ry * 8, ignore_interior);
    if !crate::common::jne_mask(x(dp.read()), x(m), 0x41) {
        let d = ry - rx;
        let (mut xx, mut ylo, mut xh) = (rx, ry - 1, rx + 1);
        let inr = |v: i32| (0..8).contains(&v);
        for _k in 1..5 {
            let xlo = xx - 1;
            if inr(xlo) {
                for y in ylo..=(d + xh) {
                    if inr(y) {
                        search(xlo + y * 8, ignore_interior);
                    }
                }
            }
            if inr(xh) {
                for y in ylo..=(d + xh) {
                    if inr(y) {
                        search(xh + y * 8, ignore_interior);
                    }
                }
            }
            if inr(ylo) {
                for xv in xx..xh {
                    if inr(xv) {
                        search(xv + ylo * 8, ignore_interior);
                    }
                }
            }
            let yh = d + xh;
            if inr(yh) {
                for xv in xx..xh {
                    if inr(xv) {
                        search(xv + yh * 8, ignore_interior);
                    }
                }
            }
            m = x(m).fadd_m(750.0).to_f32();
            if lt(x(dp.read()), x(m)) {
                break;
            }
            ylo -= 1;
            xx -= 1;
            xh += 1;
        }
    }
    if ignore_interior as u8 == 0 {
        for r in 0x40..0x48 {
            search(r, 0);
        }
    }
    out.write(bp.read());
    out
}

/// `CPathFind::FindNodeClosestToCoors(CVector pos, uint8 type, int32 count, CNodeAddress* out, float maxDist,
/// bool lowTraffic, bool unused, bool boats, bool ignoreInterior)` (0x44FA30; gr's `RecordNodesClosestToCoors`):
/// clears bit 1 of `+0x19` over the type's node range in all 72 areas (type other than 0/1: the range
/// `[type dword, pos.y bits)`, as the bytes' leftover registers), then up to `count` times takes the closest unmarked
/// node (`FindNodeClosestToCoors(..., unkn = 1, ...)`), marks it and stores it; stops at the first miss.
#[hook(addr = 0x44FA30, conv = thiscall, name = "CPathFind::FindNodeClosestToCoors", expect = "51 53 55 8B 6C 24 1C", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn record_nodes_closest_to_coors(
    this: GPtr<u8>,
    px: f32,
    py: f32,
    pz: f32,
    node_type: u32,
    count: i32,
    out: GPtr<u32>,
    max_dist: f32,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
) {
    let (mut start, mut end) = (node_type as i32, py.to_bits() as i32);
    for area in 0..0x48 {
        let base = nodes(this, area);
        if base.is_null() {
            continue;
        }
        match node_type as u8 {
            0 => {
                end = area_field::<i32>(this, NUM_VEHICLE_NODES, area).read();
                start = 0;
            }
            1 => {
                start = area_field::<i32>(this, NUM_VEHICLE_NODES, area).read();
                end = area_field::<i32>(this, NUM_NODES, area).read();
            }
            _ => {}
        }
        for i in start..end {
            let p = at::<u8>(nodes(this, area), i as isize * 0x1C + 0x19);
            p.write(p.read() & 0xFD);
        }
    }
    let mut n = count;
    let mut dst = out;
    while n > 0 {
        let mut r = [0u32];
        let rp = GPtr::from_raw(r.as_mut_ptr());
        let addr = find_node_closest_to_coors_call(this, rp, px, py, pz, node_type, max_dist, a, b, 1, c, d).read();
        if addr as u16 == 0xFFFF {
            break;
        }
        let f = at::<u8>(node_of(this, addr), 0x19);
        f.write(f.read() | 2);
        dst.write(addr);
        n -= 1;
        dst = dst.wrapping_index(1);
    }
}

/// `CPathFind::CountNeighboursToBeSwitchedOff(const CPathNode&)` (stub 0x4504F0 → 0x156A990): loaded links whose behaviour type is not 1 / 2. ECX is preserved (callers such
/// as `SwitchOffNodeAndNeighbours` keep `this` in it across the call).
#[hook(addr = 0x4504F0, conv = thiscall, name = "CPathFind::CountNeighboursToBeSwitchedOff", expect = "E9 9B A4 11 01", this = "CPathFind", preserve = "ecx")]
pub fn count_neighbours_to_be_switched_off(this: GPtr<u8>, node: GPtr<u8>) -> i32 {
    let links = num_links(node);
    let mut c = 0;
    for l in 0..links {
        let a = node_link(this, node, l);
        if nodes(this, area_of(a)).is_null() {
            continue;
        }
        let t = rd::<u8>(node_of(this, a), 0x1A) >> 4;
        if t != 1 && t != 2 {
            c += 1;
        }
    }
    c
}

/// `CPathFind::IsWaterNodeNearby(CVector pos, float radius)` (0x450DE0): a water vehicle node (`+0x18` bit 7) of
/// areas 0..63 with 2D distance strictly below `radius`.
#[hook(addr = 0x450DE0, conv = thiscall, name = "CPathFind::IsWaterNodeNearby", expect = "83 EC 10 53 55", this = "CPathFind")]
pub fn is_water_node_nearby(this: GPtr<u8>, px: f32, py: f32, _pz: f32, radius: f32) -> u8 {
    for a in 0..0x40 {
        let base = nodes(this, a);
        if base.is_null() {
            continue;
        }
        let count = area_field::<i32>(this, NUM_VEHICLE_NODES, a).read();
        for k in 0..count {
            let n = base.byte_add(k as isize * 0x1C);
            if rd::<u8>(n, 0x18) & 0x80 == 0 {
                continue;
            }
            let nx = coord(n, 8);
            let ny = coord(n, 0xA);
            let dx = x(px).fsub(nx).to_f32();
            let dy = x(py).fsub(ny);
            let d = dy.fmul(dy).fadd(x(dx).fmul_m(dx)).fsqrt();
            if lt(d, x(radius)) {
                return 1;
            }
        }
    }
    0
}

/// `CPathFind::SwitchOffNodeAndNeighbours(CPathNode*, CPathNode** out1, CPathNode** out2, bool switchTo,
/// bool backToOriginal)` (stub 0x452160 → 0x15646A0). Follows the bytes: `*out2` is written without a NULL check
/// (gr's FIX_BUGS adds one) and the neighbour test counts the neighbours of the LINKED node (gr counts the start
/// node again); deviations, keep-original.
#[hook(addr = 0x452160, conv = thiscall, name = "CPathFind::SwitchOffNodeAndNeighbours", expect = "E9 3B 25 11 01", this = "CPathFind")]
pub fn switch_off_node_and_neighbours(
    this: GPtr<u8>,
    node: GPtr<u8>,
    out1: GPtr<GPtr<u8>>,
    out2: GPtr<GPtr<u8>>,
    switch_to: u8,
    back: u8,
) {
    let b18 = rd::<u8>(node, 0x18);
    if back != 0 {
        let al = ((rd::<u8>(node, 0x19) << 5) ^ b18) & 0x20;
        wr(node, 0x18, b18 ^ al);
    } else {
        let dl = ((switch_to << 5) ^ b18) & 0x20;
        wr(node, 0x18, b18 ^ dl);
    }
    out1.write(GPtr::NULL);
    if !out2.is_null() {
        out2.write(GPtr::NULL);
    }
    if count_neighbours_to_be_switched_off_call(this, node) >= 3 {
        return;
    }
    let mut l = 0;
    if num_links(node) == 0 {
        return;
    }
    loop {
        let a = node_link(this, node, l);
        if !nodes(this, area_of(a)).is_null() {
            let n = node_of(this, a);
            let t = rd::<u8>(n, 0x1A) >> 4;
            if t != 1 && t != 2 {
                let same = if back != 0 {
                    ((rd::<u8>(n, 0x18) >> 5) ^ rd::<u8>(n, 0x19)) & 1 == 0
                } else {
                    (rd::<u8>(n, 0x18) >> 5) & 1 == switch_to
                };
                if !same && count_neighbours_to_be_switched_off_call(this, n) < 3 {
                    if out1.read().is_null() {
                        out1.write(n);
                    } else {
                        out2.write(n);
                    }
                }
            }
        }
        l += 1;
        if l >= num_links(node) {
            break;
        }
    }
}

/// `CPathFind::AreNodesLoadedForArea(float x0, float x1, float y0, float y1)` (0x44DD10).
#[hook(addr = 0x44DD10, conv = thiscall, name = "CPathFind::AreNodesLoadedForArea", expect = "D9 44 24 04 53", this = "CPathFind")]
pub fn are_nodes_loaded_for_area(this: GPtr<u8>, x0: f32, x1: f32, y0: f32, y1: f32) -> u8 {
    let rx0 = region_value(x(x0));
    let ry0 = region_value(x(y0));
    let rx1 = region_value(x(x1));
    let ry1 = region_value(x(y1));
    for xx in rx0..=rx1 {
        for yy in ry0..=ry1 {
            if nodes(this, xx + yy * 8).is_null() {
                return 0;
            }
        }
    }
    1
}

/// `CPathFind::UpdateStreaming(bool force)` (0x450A60): unless forced, a pending load-scene request or a new
/// 512 ms period (`(prev ^ now) & ~0x1FF`), returns. Clears `ToBeStreamed` / `ToBeStreamedForScript`, marks
/// 350 around the player, 300 around the load-scene coords (and clears that flag), 300 around every vehicle
/// created by MISSION (`+0x4A4 == 2`, the bytes `jne` skip all others; subtypes 3, 4, 5, 6, 8 skipped; descending
/// pool walk), and the requested area (both arrays). Then requests (flags 4 for script areas, else 8) every
/// marked unloaded area and removes every unmarked loaded one (`RESOURCE_ID_DAT` 0x63A7 + area).
#[hook(addr = 0x450A60, conv = thiscall, name = "CPathFind::UpdateStreaming", expect = "83 EC 14 8A 44 24 18", this = "CPathFind")]
pub fn update_streaming(this: GPtr<u8>, force: u32) {
    use sa_core::timer::{PREV_TIME_MS, TIME_MS};
    if force as u8 == 0
        && LOAD_SCENE_FOR_PATH_NODES.get() == 0
        && (PREV_TIME_MS.get() ^ TIME_MS.get()) & 0xFFFF_FE00 == 0
    {
        return;
    }
    TO_BE_STREAMED.set([0; 64]);
    TO_BE_STREAMED_FOR_SCRIPT.set([0; 64]);
    if !crate::common::find_player_ped(-1).is_null() {
        let mut v = [0f32; 3];
        let p = crate::common::find_player_coors(GPtr::from_raw(v.as_mut_ptr()), -1);
        mark_regions_for_coors_call(this, p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read(), 350.0);
    }
    if LOAD_SCENE_FOR_PATH_NODES.get() != 0 {
        let c = LOAD_SCENE_COORS.get();
        mark_regions_for_coors_call(this, c[0], c[1], c[2], 300.0);
        LOAD_SCENE_FOR_PATH_NODES.set(0);
    }
    let pool = sa_core::pool::VEHICLE_POOL.get().cast::<u8>();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        let flags = rd::<GPtr<i8>>(pool, 4);
        let free = flags.wrapping_index(i - 1).read() < 0;
        i -= 1;
        if free {
            continue;
        }
        let objs = rd::<GPtr<u8>>(pool, 0);
        let veh = objs.byte_add(i as isize * 0xA18);
        if veh.is_null() {
            continue;
        }
        let sub = rd::<i32>(veh, 0x594);
        if matches!(sub, 3 | 4 | 5 | 6 | 8) || rd::<u8>(veh, 0x4A4) != 2 {
            continue;
        }
        let m = rd::<GPtr<u8>>(veh, 0x14);
        let p = if m.is_null() { veh.byte_add(4) } else { m.byte_add(0x30) };
        mark_regions_for_coors_call(this, rd(p, 0), rd(p, 4), rd(p, 8), 300.0);
    }
    if rd::<u8>(this, LOAD_AREA_PENDING) != 0 {
        let x0 = region_value(x(rd(this, LOAD_AREA)));
        let x1 = region_value(x(rd(this, LOAD_AREA + 4)));
        let y0 = region_value(x(rd(this, LOAD_AREA + 8)));
        let y1 = region_value(x(rd(this, LOAD_AREA + 0xC)));
        let s = TO_BE_STREAMED_FOR_SCRIPT.ptr().cast::<u8>();
        let t = TO_BE_STREAMED.ptr().cast::<u8>();
        for xx in x0..=x1 {
            for yy in y0..=y1 {
                s.wrapping_index(xx * 8 + yy).write(1);
                t.wrapping_index(xx * 8 + yy).write(1);
            }
        }
    }
    let s = TO_BE_STREAMED_FOR_SCRIPT.ptr().cast::<u8>();
    let t = TO_BE_STREAMED.ptr().cast::<u8>();
    for xx in 0..8 {
        for yy in 0..8 {
            let id = 0x63A7 + xx + yy * 8;
            let loaded = !nodes(this, xx + yy * 8).is_null();
            if t.wrapping_index(xx * 8 + yy).read() != 0 {
                if !loaded {
                    let flags = if s.wrapping_index(xx * 8 + yy).read() != 0 { 4 } else { 8 };
                    request_model(id, flags);
                }
            } else if loaded {
                remove_model(id);
            }
        }
    }
}

/// `fcomp` + `test ah, 0x41` + `jp` taken: `a > b` or unordered.
fn gt_or_nan(a: X87, b: X87) -> bool {
    a.fcom(b).test_parity(0x41)
}

orig! {
    /// `CPathFind::MarkRegionsForCoors(CVector, float)` (0x44DB60)
    pub fn mark_regions_for_coors_call(this: GPtr<u8>, px: f32, py: f32, pz: f32, r: f32) = thiscall 0x44DB60 => mark_regions_for_coors;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0)
    pub fn request_model(id: i32, flags: i32) = cdecl 0x4087E0 => sa_stream::streaming::request_model;
    /// `CStreaming::RemoveModel(int32)` (0x4089A0)
    pub fn remove_model(id: i32) = cdecl 0x4089A0 => sa_stream::streaming::remove_model;
    /// `CPathNode::GetPosition(CVector*)` (0x420A10)
    pub fn get_position_call(node: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x420A10 => get_position;
    /// `CPathFind::ThisNodeHasToBeSwitchedOff(CPathNode*)` (0x44D3E0)
    pub fn this_node_has_to_be_switched_off_call(this: GPtr<u8>, node: GPtr<u8>) -> u8 = thiscall 0x44D3E0 => this_node_has_to_be_switched_off;
    /// `CPathFind::SwitchOffNodeAndNeighbours(...)` (0x452160)
    pub fn switch_off_node_and_neighbours_call(this: GPtr<u8>, node: GPtr<u8>, out1: GPtr<GPtr<u8>>, out2: GPtr<GPtr<u8>>, switch_to: u8, back: u8) = thiscall 0x452160 => switch_off_node_and_neighbours;
    /// `CPathFind::SwitchRoadsOffInAreaForOneRegion(...)` (0x452820)
    pub fn switch_roads_off_in_area_for_one_region_call(this: GPtr<u8>, x0: f32, x1: f32, y0: f32, y1: f32, z0: f32, z1: f32, off: u32, cars: u32, area: i32, back: u32) = thiscall 0x452820 => switch_roads_off_in_area_for_one_region;
    /// `RwStreamRead(RwStream*, void*, uint32)` (0x7EC9D0)
    pub fn rw_stream_read(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => unported;
    /// `CMemoryMgr::Malloc(uint32 size, uint32 hint)` (0x72F420)
    pub fn mem_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
}

/// `CPathFind::SwitchRoadsOffInAreaForOneRegion(x0, x1, y0, y1, z0, z1, bool off, bool cars, int32 area,
/// bool back)` (0x452820). Box test: `x < x0` (or NaN) rejects, `!(x <= x1)` rejects, same for y, then z through
/// two `GetPosition` calls. Nodes that must be switched off and are not yet in the target state go through
/// `SwitchOffNodeAndNeighbours`, then the two neighbour chains it returns.
#[hook(addr = 0x452820, conv = thiscall, name = "CPathFind::SwitchRoadsOffInAreaForOneRegion", expect = "8B 54 24 24 83 EC 20", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn switch_roads_off_in_area_for_one_region(
    this: GPtr<u8>,
    x0: f32,
    x1: f32,
    y0: f32,
    y1: f32,
    z0: f32,
    z1: f32,
    off: u32,
    cars: u32,
    area: i32,
    back: u32,
) {
    if nodes(this, area).is_null() {
        return;
    }
    let (start, end) = if cars as u8 != 0 {
        (0, area_field::<i32>(this, NUM_VEHICLE_NODES, area).read())
    } else {
        (
            area_field::<i32>(this, NUM_VEHICLE_NODES, area).read(),
            area_field::<i32>(this, NUM_NODES, area).read(),
        )
    };
    let sw = off as u8;
    for i in start..end {
        let n = nodes(this, area).byte_add(i as isize * 0x1C);
        let le = |v: X87, hi: f32| !gt_or_nan(v, x(hi));
        let nx = coord(n, 8);
        if nx.fcom(x(x0)).c0() || !le(nx, x1) {
            continue;
        }
        let ny = coord(n, 0xA);
        if ny.fcom(x(y0)).c0() || !le(ny, y1) {
            continue;
        }
        let mut t = [0f32; 3];
        let z = get_position_call(n, GPtr::from_raw(t.as_mut_ptr())).wrapping_index(2).read();
        if x(z).fcom(x(z0)).c0() {
            continue;
        }
        let mut t2 = [0f32; 3];
        let z = get_position_call(n, GPtr::from_raw(t2.as_mut_ptr())).wrapping_index(2).read();
        if !le(x(z), z1) {
            continue;
        }
        if this_node_has_to_be_switched_off_call(this, n) == 0 {
            continue;
        }
        let already = if back as u8 != 0 {
            ((rd::<u8>(n, 0x18) >> 5) ^ rd::<u8>(n, 0x19)) & 1 == 0
        } else {
            (rd::<u8>(n, 0x18) >> 5) & 1 == sw
        };
        if already {
            continue;
        }
        let mut n1 = [GPtr::<u8>::NULL];
        let mut n2 = [GPtr::<u8>::NULL];
        let p1 = GPtr::from_raw(n1.as_mut_ptr());
        let p2 = GPtr::from_raw(n2.as_mut_ptr());
        switch_off_node_and_neighbours_call(this, n, p1, p2, sw, back as u8);
        while !p1.read().is_null() {
            switch_off_node_and_neighbours_call(this, p1.read(), p1, GPtr::NULL, sw, back as u8);
        }
        while !p2.read().is_null() {
            switch_off_node_and_neighbours_call(this, p2.read(), p2, GPtr::NULL, sw, back as u8);
        }
    }
}

/// `CPathFind::SwitchRoadsOffInArea(x0, x1, y0, y1, z0, z1, bool off, bool cars, bool back)` (0x452C80; the count is
/// read through the stub 0x401956). Every region, then the node-switch list as the bytes (deviations,
/// keep-original): an entry fully inside the box is "removed" by copying entry `i + 1` over entry `i`
/// `count - 1 - i` times (later entries are not shifted), `count--`, and index `i` is tested again; finally, unless
/// `back`, the box is appended when `count < 64` (no `cars` comparison, no toggled-back detection).
#[hook(addr = 0x452C80, conv = thiscall, name = "CPathFind::SwitchRoadsOffInArea", expect = "83 EC 18 53 8B 5C 24 30", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn switch_roads_off_in_area(this: GPtr<u8>, x0: f32, x1: f32, y0: f32, y1: f32, z0: f32, z1: f32, off: u32, cars: u32, back: u32) {
    for r in 0..0x40 {
        switch_roads_off_in_area_for_one_region_call(this, x0, x1, y0, y1, z0, z1, off, cars, r, back);
    }
    let count = at::<i32>(this, NUM_NODE_SWITCHES);
    let entry = |i: i32| at::<u8>(this, NODE_SWITCHES + 0x1C * i as isize);
    let mut i = 0;
    while i < count.read() {
        let e = entry(i);
        let f = |o: isize| x(rd::<f32>(e, o));
        let keep = f(0).fcom(x(x0)).c0()
            || f(8).fcom(x(y0)).c0()
            || f(0x10).fcom(x(z0)).c0()
            || gt_or_nan(f(4), x(x1))
            || gt_or_nan(f(0xC), x(y1))
            || gt_or_nan(f(0x14), x(z1));
        if !keep {
            let mut j = i;
            while j < count.read() - 1 {
                for w in 0..7 {
                    let v = rd::<u32>(entry(i + 1), 4 * w);
                    wr(entry(i), 4 * w, v);
                }
                j += 1;
            }
            count.write(count.read() - 1);
            i -= 1;
        }
        i += 1;
    }
    if back as u8 == 0 && count.read() < 0x40 {
        let e = entry(count.read());
        for (k, v) in [x0, x1, y0, y1, z0, z1].into_iter().enumerate() {
            wr(e, 4 * k as isize, x(v).to_f32());
        }
        wr::<u8>(e, 0x18, off as u8);
        wr::<u8>(e, 0x19, cars as u8);
        count.write(count.read() + 1);
    }
}

/// `CPathFind::LoadPathFindData(RwStream*, int32 area)` (stub 0x4529F0 → 0x156F750). The links part runs through
/// protection code (0x156F85C → 0xCB73E0 → 0x156F890) that only computes the second `Malloc` argument of the navi
/// links allocation (0, as every other call here). Reads the five counts (nodes, vehicle nodes, ped nodes, car
/// path links +0x1304, links +0x1424), allocates and reads nodes (`n * 0x1C`; 0x1C unread bytes when n == 0), navi
/// nodes (`n * 0xE`, NULL when 0), and with links L != 0: links `L*4 + 0x300`, navi links `L*2`, lengths and
/// intersections `L + 0xC0` each (all four read with those sizes; NULL when L == 0). Then copies bit 5 of `+0x18`
/// into bit 0 of `+0x19` for every node, re-applies the node switches to this area, and fills the two 16-dword
/// rows at `+0x1544 + area*0x40` and `+0x2544 + area*0x40` with -1.
#[hook(addr = 0x4529F0, conv = thiscall, name = "CPathFind::LoadPathFindData-FromStream", expect = "E9 5B CD 11 01", this = "CPathFind")]
pub fn load_path_find_data_from_stream(this: GPtr<u8>, stream: GPtr<u8>, area: i32) {
    let cnt = |base: isize| area_field::<u8>(this, base, area);
    for base in [NUM_NODES, NUM_VEHICLE_NODES, NUM_PED_NODES, 0x1304, 0x1424] {
        rw_stream_read(stream, cnt(base), 4);
    }
    let rdc = |base: isize| area_field::<i32>(this, base, area).read();
    let set = |base: isize, p: GPtr<u8>| area_field::<GPtr<u8>>(this, base, area).write(p);
    if rdc(NUM_NODES) != 0 {
        let p = mem_malloc(rdc(NUM_NODES).wrapping_mul(0x1C) as u32, 0);
        set(PATH_NODES, p);
        rw_stream_read(stream, p, rdc(NUM_NODES).wrapping_mul(0x1C) as u32);
    } else {
        let p = mem_malloc(0x1C, 0);
        set(PATH_NODES, p);
    }
    if rdc(0x1304) != 0 {
        let p = mem_malloc(rdc(0x1304).wrapping_mul(0xE) as u32, 0);
        set(NAVI_NODES, p);
        rw_stream_read(stream, p, rdc(0x1304).wrapping_mul(0xE) as u32);
    } else {
        set(NAVI_NODES, GPtr::NULL);
    }
    if rdc(0x1424) != 0 {
        let l = rdc(0x1424) as u32;
        let p = mem_malloc(l.wrapping_mul(4).wrapping_add(0x300), 0);
        set(NODE_LINKS, p);
        let p = mem_malloc(rdc(0x1424).wrapping_mul(2) as u32, 0);
        set(NAVI_LINKS, p);
        let p = mem_malloc((rdc(0x1424) as u32).wrapping_add(0xC0), 0);
        set(LINK_LENGTHS, p);
        let p = mem_malloc((rdc(0x1424) as u32).wrapping_add(0xC0), 0);
        set(INTERSECTIONS, p);
        let get = |base: isize| area_field::<GPtr<u8>>(this, base, area).read();
        rw_stream_read(stream, get(NODE_LINKS), (rdc(0x1424) as u32).wrapping_mul(4).wrapping_add(0x300));
        rw_stream_read(stream, get(NAVI_LINKS), (rdc(0x1424) as u32).wrapping_mul(2));
        rw_stream_read(stream, get(LINK_LENGTHS), (rdc(0x1424) as u32).wrapping_add(0xC0));
        rw_stream_read(stream, get(INTERSECTIONS), (rdc(0x1424) as u32).wrapping_add(0xC0));
    } else {
        for base in [NODE_LINKS, NAVI_LINKS, LINK_LENGTHS, INTERSECTIONS] {
            set(base, GPtr::NULL);
        }
    }
    let mut i = 0;
    while i < rdc(NUM_NODES) {
        let n = nodes(this, area).byte_add(i as isize * 0x1C);
        let b18 = rd::<u8>(n, 0x18);
        let b19 = rd::<u8>(n, 0x19);
        wr(n, 0x19, b19 ^ (((b18 >> 5) ^ b19) & 1));
        i += 1;
    }
    let mut k = 0;
    while k < rd::<i32>(this, NUM_NODE_SWITCHES) {
        let e = at::<u8>(this, NODE_SWITCHES + 0x1C * k as isize);
        switch_roads_off_in_area_for_one_region_call(
            this,
            rd(e, 0),
            rd(e, 4),
            rd(e, 8),
            rd(e, 0xC),
            rd(e, 0x10),
            rd(e, 0x14),
            u32::from(rd::<u8>(e, 0x18)),
            u32::from(rd::<u8>(e, 0x19)),
            area,
            0,
        );
        k += 1;
    }
    let row = at::<i32>(this, 0x1544 + (area << 6) as isize);
    for w in 0..16 {
        row.wrapping_index(w + 0x400).write(-1);
        row.wrapping_index(w).write(-1);
    }
}

// ---------------------------------------------------------------------------------------------------------
// CPathFind: path search and link helpers

orig! {
    /// `CPathFind::DoPathSearch(...)` (0x4515D0)
    pub fn do_path_search_call(this: GPtr<u8>, path_type: u32, ox: f32, oy: f32, oz: f32, origin_addr: u32, tx: f32, ty: f32, tz: f32, out_nodes: GPtr<u32>, out_count: GPtr<i16>, max_nodes: i32, out_distance: GPtr<f32>, max_search: f32, target_addr: GPtr<u32>, max_depth: f32, same_lane: u32, forbidden: u32, allow_water: u32, for_boats: u32) = thiscall 0x4515D0 => do_path_search;
    /// `CPathFind::AddDynamicLinkBetween2Nodes_For1Node(CNodeAddress, CNodeAddress)` (0x44E000)
    pub fn add_dynamic_link_between_2_nodes_for_1_node_call(this: GPtr<u8>, a: u32, b: u32) = thiscall 0x44E000 => add_dynamic_link_between_2_nodes_for_1_node;
}

/// The search's "fail": `*outCount = 0`, `*outDistance = 100000.0f` when given.
fn path_fail(out_count: GPtr<i16>, out_distance: GPtr<f32>) {
    out_count.write(0);
    if !out_distance.is_null() {
        out_distance.write(f32::from_bits(0x47C3_5000));
    }
}

/// `CPathFind::DoPathSearch(uint8 type, CVector originPos, CNodeAddress originAddr, CVector targetPos,
/// CNodeAddress* outNodes, int16* outCount, int32 maxNodes, float* outDistance, float maxSearchDistance,
/// CNodeAddress* targetAddr, float maxDepth, bool sameLaneOnly, CNodeAddress forbidden, bool allowWater,
/// bool forBoats)` (0x4515D0, `ret 0x4C`; spec CPathFind.md, ported from the bytes: target resolved before the
/// origin, the back-trace uses the base link id and increments the count once per step, its loop condition is
/// `count < max && node != target`). Keeps the original hang: when no link of the current node leads one step
/// back along the distance field, the back-trace loop never ends.
#[hook(addr = 0x4515D0, conv = thiscall, name = "CPathFind::DoPathSearch", expect = "51 8B 44 24 3C", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn do_path_search(
    this: GPtr<u8>,
    path_type: u32,
    ox: f32,
    oy: f32,
    oz: f32,
    origin_addr: u32,
    tx: f32,
    ty: f32,
    tz: f32,
    out_nodes: GPtr<u32>,
    out_count: GPtr<i16>,
    max_nodes: i32,
    out_distance: GPtr<f32>,
    max_search: f32,
    target_addr: GPtr<u32>,
    max_depth: f32,
    same_lane: u32,
    forbidden: u32,
    allow_water: u32,
    for_boats: u32,
) {
    wr::<i32>(this, HASH_COUNT, 0);
    let t = if !target_addr.is_null()
        && target_addr.cast::<u16>().read() != 0xFFFF
        && !nodes(this, area_of(target_addr.read())).is_null()
    {
        target_addr.read()
    } else {
        let mut r = [0u32];
        let rp = GPtr::from_raw(r.as_mut_ptr());
        find_node_closest_to_coors_call(this, rp, tx, ty, tz, path_type, max_search, 0, 0, 0, for_boats, 0).read()
    };
    if t as u16 == 0xFFFF {
        path_fail(out_count, out_distance);
        return;
    }
    let o = if origin_addr as u16 != 0xFFFF && !nodes(this, area_of(origin_addr)).is_null() {
        origin_addr
    } else {
        let mut r = [0u32];
        let rp = GPtr::from_raw(r.as_mut_ptr());
        find_node_closest_to_coors_call(this, rp, ox, oy, oz, path_type, max_search, 0, 0, 0, for_boats, 0).read()
    };
    if o as u16 == 0xFFFF {
        path_fail(out_count, out_distance);
        return;
    }
    if o == t {
        out_count.write(0);
        if !out_distance.is_null() {
            out_distance.write(0.0);
        }
        return;
    }
    let tn = node_of(this, t);
    let on = node_of(this, o);
    if rd::<u8>(on, 0x17) != rd::<u8>(tn, 0x17) {
        path_fail(out_count, out_distance);
        return;
    }
    for b in 0..0x200 {
        wr::<u32>(this, 4 + 4 * b, 0);
    }
    link_into(this, tn, 0);
    let cleared_list = NODES_TO_BE_CLEARED.ptr().cast::<u32>();
    cleared_list.write(rd::<u32>(tn, 0x12));
    let mut cleared: i32 = 1;
    let mut depth: i32 = 0;
    let mut finished = false;
    let mut bucket = 0;
    let found = loop {
        let mut xn = at::<GPtr<u8>>(this, 4 + 4 * bucket as isize).read();
        while !xn.is_null() {
            if xn == on {
                finished = true;
            }
            let links = num_links(xn);
            let mut l = 0;
            while l < links {
                let a = node_link(this, xn, l);
                l += 1;
                let lp = nodes(this, area_of(a));
                if lp.is_null() {
                    continue;
                }
                let ln = lp.byte_add(index_of(a) as isize * 0x1C);
                let mut blocked = false;
                if same_lane as u8 != 0 {
                    let v = navi_link(this, xn, l - 1);
                    if !nodes(this, (v >> 10) as i32).is_null() {
                        let c = navi_node(this, v);
                        let b = rd::<u8>(c, 0xB);
                        let lanes = if rd::<u16>(c, 4) == a as u16 && rd::<u16>(c, 6) == (a >> 16) as u16 {
                            (b >> 3) & 7
                        } else {
                            b & 7
                        };
                        if lanes == 0 {
                            blocked = true;
                        }
                    }
                }
                if forbidden as u16 == a as u16 && (forbidden >> 16) as u16 == (a >> 16) as u16 {
                    blocked = true;
                }
                if (rd::<u8>(ln, 0x18) ^ rd::<u8>(xn, 0x18)) & 0x80 != 0 && allow_water as u8 == 0 {
                    continue;
                }
                if blocked {
                    continue;
                }
                let d = i32::from(rd::<i16>(xn, 0xE)) + i32::from(link_length(this, xn, l - 1));
                let le = rd::<i16>(ln, 0xE);
                if d >= i32::from(le) {
                    continue;
                }
                if le != 0x7FFE {
                    unlink(this, ln);
                }
                if rd::<i16>(ln, 0xE) == 0x7FFE && cleared < 5000 {
                    cleared_list.wrapping_index(cleared).write(rd::<u32>(ln, 0x12));
                    cleared += 1;
                }
                link_into(this, ln, d);
            }
            unlink(this, xn);
            xn = rd::<GPtr<u8>>(xn, 0);
        }
        depth += 1;
        bucket = depth & 0x1FF;
        if rd::<i32>(this, HASH_COUNT) == 0 {
            break false;
        }
        if xi(depth).fcom(x(max_depth)) == sa_math::x87::Cmp::Greater {
            break false;
        }
        if cleared >= 0x1356 {
            break false;
        }
        if finished {
            break true;
        }
    };
    out_count.write(0);
    if found {
        if !out_distance.is_null() {
            out_distance.write(xi(i32::from(rd::<i16>(on, 0xE))).to_f32());
        }
        if !out_nodes.is_null() {
            out_nodes.wrapping_index(i32::from(out_count.read())).write(rd::<u32>(on, 0x12));
            out_count.write(out_count.read().wrapping_add(1));
        }
        let mut xn = on;
        while i32::from(out_count.read()) < max_nodes {
            if xn == tn {
                break;
            }
            // no link matching: `xn` and the count stay unchanged and this loop spins forever (original hang)
            let links = num_links(xn);
            let mut l = 0;
            while l < links {
                let a = node_link(this, xn, l);
                let lp = nodes(this, area_of(a));
                if !lp.is_null() {
                    let ln = lp.byte_add(index_of(a) as isize * 0x1C);
                    let back = i32::from(rd::<i16>(xn, 0xE)) - i32::from(link_length(this, xn, l));
                    if back == i32::from(rd::<i16>(ln, 0xE)) {
                        out_nodes.wrapping_index(i32::from(out_count.read())).write(rd::<u32>(ln, 0x12));
                        out_count.write(out_count.read().wrapping_add(1));
                        xn = ln;
                        break;
                    }
                }
                l += 1;
            }
        }
    }
    for i in 0..cleared {
        let a = cleared_list.wrapping_index(i).read();
        wr::<i16>(node_of(this, a), 0xE, 0x7FFE);
    }
}

/// `CPathFind::ComputeRoute(uint8 type, const CVector& start, const CVector& end, const CNodeAddress& startAddr,
/// CNodeRoute* route)` (stub 0x452760 → 0x156B7D0): `DoPathSearch` into 8 local addresses (area words 0xFFFF; the
/// node words are uninitialised stack in the original, 0 here), forbidden = `[0x8A5F44]`, then copies up to 8 into
/// the route (`+0` count, `+4` nodes).
#[hook(addr = 0x452760, conv = thiscall, name = "CPathFind::ComputeRoute", expect = "E9 6B 90 11 01", this = "CPathFind")]
pub fn compute_route(this: GPtr<u8>, node_type: u32, start: GPtr<f32>, end: GPtr<f32>, start_addr: GPtr<u32>, route: GPtr<u8>) {
    let mut count = [0i32];
    let mut nodes8 = [0x0000_FFFFu32; 8];
    let np = GPtr::from_raw(nodes8.as_mut_ptr());
    let cp = GPtr::from_raw(count.as_mut_ptr());
    let v = |p: GPtr<f32>, i: i32| p.wrapping_index(i).read();
    do_path_search_call(
        this,
        node_type,
        v(start, 0),
        v(start, 1),
        v(start, 2),
        start_addr.read(),
        v(end, 0),
        v(end, 1),
        v(end, 2),
        np,
        cp.cast(),
        8,
        GPtr::NULL,
        f32::from_bits(0x4974_23FE),
        GPtr::NULL,
        f32::from_bits(0x4974_23FE),
        0,
        FORBIDDEN_NONE.get(),
        0,
        0,
    );
    let n = i32::from(cp.cast::<i16>().read());
    let rc = at::<i32>(route, 0);
    rc.write(0);
    for i in 0..n {
        let c = rc.read();
        if c < 8 {
            at::<u32>(route, 4 + 4 * c as isize).write(np.wrapping_index(i).read());
            rc.write(c + 1);
        }
    }
}

/// `CPathFind::TestCoorsCloseness(CVector target, uint8 type, CVector start)` (0x452000): `DoPathSearch` from
/// `start` (origin address area 0xFFFF; its node word is the high word of the `this` ECX the original pushed) to
/// `target` with no output nodes, count into the scratch int16 at 0x96F010, max search 50, max depth 100; true when
/// the distance (written into the `type` argument slot) is below 100.
#[hook(addr = 0x452000, conv = thiscall, name = "CPathFind::TestCoorsCloseness", expect = "51 A1 44 5F 8A 00", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn test_coors_closeness(this: GPtr<u8>, tx: f32, ty: f32, tz: f32, node_type: u32, sx: f32, sy: f32, sz: f32) -> u8 {
    let mut dist = [f32::from_bits(node_type)];
    let dp = GPtr::from_raw(dist.as_mut_ptr());
    let origin = ((this.addr() as u32) & 0xFFFF_0000) | 0xFFFF;
    do_path_search_call(
        this,
        node_type,
        sx,
        sy,
        sz,
        origin,
        tx,
        ty,
        tz,
        GPtr::NULL,
        PATH_SEARCH_SCRATCH_COUNT.ptr(),
        0,
        dp,
        50.0,
        GPtr::NULL,
        100.0,
        0,
        FORBIDDEN_NONE.get(),
        0,
        0,
    );
    u8::from(lt(x(dp.read()), x(100.0)))
}

/// `CPathFind::FindLinkBetweenNodes(CNodeAddress a, CNodeAddress b)` (stub 0x451350 → 0x156A180, hidden return
/// `CCarPathLinkAddress*`): writes only the word. The load checks use `this`, the lookups the absolute `ThePaths`;
/// the link scan is unbounded (as the bytes).
#[hook(addr = 0x451350, conv = thiscall, name = "CPathFind::FindLinkBetweenNodes", expect = "E9 2B 8E 11 01", this = "CPathFind")]
pub fn find_link_between_nodes(this: GPtr<u8>, out: GPtr<u16>, a: u32, b: u32) -> GPtr<u16> {
    if nodes(this, area_of(a)).is_null() || nodes(this, area_of(b)).is_null() {
        out.write(0xFFFF);
        return out;
    }
    let tp = the_paths();
    let n = node_of(tp, a);
    let base = i32::from(rd::<i16>(n, 0x10));
    let links = area_field::<GPtr<u32>>(tp, NODE_LINKS, area_of(a)).read().wrapping_index(base);
    let mut i = 0;
    loop {
        let c = links.wrapping_index(i).read();
        if c as u16 == b as u16 && (c >> 16) as u16 == (b >> 16) as u16 {
            break;
        }
        i += 1;
    }
    let v = area_field::<GPtr<u16>>(tp, NAVI_LINKS, area_of(a)).read().wrapping_index(base + i).read();
    out.write(v);
    out
}

/// `CPathFind::ReturnInteriorNodeIndex(uint32 interiorId, int16 node)` (0x451300, hidden return): area
/// `0x40 + slot` of the interior id; not found: area 0xFFFF with the HIGH word of the id argument (as the bytes).
#[hook(addr = 0x451300, conv = thiscall, name = "CPathFind::ReturnInteriorNodeIndex", expect = "8B 54 24 08 33 C0", this = "CPathFind")]
pub fn return_interior_node_index(this: GPtr<u8>, out: GPtr<u32>, id: u32, node: u32) -> GPtr<u32> {
    for i in 0..8 {
        if area_field::<u32>(this, INTERIOR_IDS, i).read() == id {
            out.write((0x40 + i as u32) & 0xFFFF | (node << 16));
            return out;
        }
    }
    out.write((id & 0xFFFF_0000) | 0xFFFF);
    out
}

/// `CPathFind::AddDynamicLinkBetween2Nodes(CNodeAddress a, CNodeAddress b)` (0x4512D0): the one-node version for
/// (a, b) then (b, a).
#[hook(addr = 0x4512D0, conv = thiscall, name = "CPathFind::AddDynamicLinkBetween2Nodes", expect = "53 8B 5C 24 08", this = "CPathFind")]
pub fn add_dynamic_link_between_2_nodes(this: GPtr<u8>, a: u32, b: u32) {
    add_dynamic_link_between_2_nodes_for_1_node_call(this, a, b);
    add_dynamic_link_between_2_nodes_for_1_node_call(this, b, a);
}

/// `CPathFind::FindNearestExteriorNodeToInteriorNode(int32 id)` (0x450F30, hidden return; ECX not read: the
/// search runs on `ThePaths`): ped node within 3.0 of `(XCoorGiven, YCoorGiven, ZCoorGiven)[id]`, interiors
/// ignored.
#[hook(addr = 0x450F30, conv = thiscall, name = "CPathFind::FindNearestExteriorNodeToInteriorNode", expect = "83 EC 0C 8B 44 24 14", this = "CPathFind")]
pub fn find_nearest_exterior_node_to_interior_node(_this: GPtr<u8>, out: GPtr<u32>, id: i32) -> GPtr<u32> {
    let c = COOR_GIVEN.ptr().cast::<f32>();
    let px = x(c.wrapping_index(128 + id).read()).to_f32();
    let py = x(c.wrapping_index(64 + id).read()).to_f32();
    let pz = x(c.wrapping_index(id).read()).to_f32();
    let mut r = [0u32];
    let rp = GPtr::from_raw(r.as_mut_ptr());
    let a = find_node_closest_to_coors_call(the_paths(), rp, px, py, pz, 1, 3.0, 0, 0, 0, 0, 1).read();
    out.write(a);
    out
}

/// `CPathFind::Shutdown()` (stub 0x450950 → 0x1562750): `CStreaming::RemoveModel(0x63A7 + area)` for every loaded
/// area 0..63 (x outer, y inner).
#[hook(addr = 0x450950, conv = thiscall, name = "CPathFind::Shutdown", expect = "E9 FB 1D 11 01", this = "CPathFind")]
pub fn shutdown(this: GPtr<u8>) {
    for xx in 0..8 {
        for yy in 0..8 {
            if !nodes(this, xx + yy * 8).is_null() {
                remove_model(0x63A7 + xx + yy * 8);
            }
        }
    }
}

/// `CPathFind::MarkRoadNodeAsDontWander(float x, float y, float z)` (stub 0x450560 → 0x156ECF0): the closest car
/// node gets `+0x19 |= 4`.
#[hook(addr = 0x450560, conv = thiscall, name = "CPathFind::MarkRoadNodeAsDontWander", expect = "E9 8B E7 11 01", this = "CPathFind")]
pub fn mark_road_node_as_dont_wander(this: GPtr<u8>, px: f32, py: f32, pz: f32) {
    let mut r = [0u32];
    let rp = GPtr::from_raw(r.as_mut_ptr());
    let a = find_node_closest_to_coors_call(this, rp, px, py, pz, 0, f32::from_bits(0x4974_23FE), 0, 0, 0, 0, 0).read();
    if a as u16 != 0xFFFF {
        let f = at::<u8>(node_of(this, a), 0x19);
        f.write(f.read() | 4);
    }
}

/// `CPathFind::These2NodesAreAdjacent(CNodeAddress a, CNodeAddress b)` (stub 0x44D230 → 0x1569710; reads the
/// absolute `ThePaths`, ECX not read).
#[hook(addr = 0x44D230, conv = thiscall, name = "CPathFind::These2NodesAreAdjacent", expect = "E9 DB C4 11 01", this = "CPathFind")]
pub fn these_2_nodes_are_adjacent(_this: GPtr<u8>, a: u32, b: u32) -> u8 {
    let tp = the_paths();
    let n = node_of(tp, a);
    for l in 0..num_links(n) {
        let c = node_link(tp, n, l);
        if c as u16 == b as u16 && (c >> 16) as u16 == (b >> 16) as u16 {
            return 1;
        }
    }
    0
}

// ---------------------------------------------------------------------------------------------------------
// CPathFind: more closest-node searches

orig! {
    /// `CVector::Normalise()` (0x59C910)
    pub fn vector_normalise(v: GPtr<f32>) = thiscall 0x59C910 => unported;
}

/// `CPathFind::FindNodeClosestToCoorsFavourDirection(CVector pos, uint8 type, float dirX, float dirY)` (0x44FCE0,
/// hidden return, `ret 0x1C`). The direction is normalised (`1.0f / len`; a zero length gives `(1.0, dirY)`); over
/// the type's nodes of all 72 areas, `d = |dz|*3 + |dy| + |dx|` must be below the best (start 10000), then the
/// score `d - (dot(normalised node - pos, dir) - 1) * 20` (a zero offset gives the x factor 1.0 and an
/// unnormalised y) must be below the best. The result local starts with only its area word set (0xFFFF); its
/// node word is uninitialised stack in the original and 0 here.
#[hook(addr = 0x44FCE0, conv = thiscall, name = "CPathFind::FindNodeClosestToCoorsFavourDirection", expect = "D9 44 24 18 83 EC 1C", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn find_node_closest_to_coors_favour_direction(
    this: GPtr<u8>,
    out: GPtr<u32>,
    px: f32,
    py: f32,
    pz: f32,
    node_type: u32,
    dir_x: f32,
    dir_y: f32,
) -> GPtr<u32> {
    let len = x(dir_x).fmul_m(dir_x).fadd(x(dir_y).fmul_m(dir_y)).fsqrt();
    let (ndx, ndy) = if len.fcom(x(0.0)) == sa_math::x87::Cmp::Equal {
        (x(1.0), dir_y)
    } else {
        let inv = x(1.0).fdiv(len).to_f32();
        (x(inv).fmul_m(dir_x), x(inv).fmul_m(dir_y).to_f32())
    };
    let mut best_addr: u32 = 0x0000_FFFF;
    let mut best = f32::from_bits(0x461C_4000);
    let (mut start, mut end) = (node_type as i32, node_type as i32);
    for area in 0..0x48 {
        let base = nodes(this, area);
        if base.is_null() {
            continue;
        }
        match node_type as u8 {
            0 => {
                end = area_field::<i32>(this, NUM_VEHICLE_NODES, area).read();
                start = 0;
            }
            1 => {
                start = area_field::<i32>(this, NUM_VEHICLE_NODES, area).read();
                end = area_field::<i32>(this, NUM_NODES, area).read();
            }
            _ => {}
        }
        let mut i = start;
        while i < end {
            let n = base.byte_add(i as isize * 0x1C);
            let nx = coord(n, 8);
            let nxf = nx.to_f32();
            let nyf = coord(n, 0xA).to_f32();
            let dz3 = fabs(coord(n, 0xC).fsub_m(pz)).fmul_m(3.0);
            let dy = fabs(x(nyf).fsub_m(py));
            let d = dz3.fadd(dy).fadd(fabs(x(nxf).fsub_m(px)));
            let df = d.to_f32();
            if lt(d, x(best)) {
                let vx = nx.fsub_m(px).to_f32();
                let vy = x(nyf).fsub_m(py);
                let l = vy.fmul(vy).fadd(x(vx).fmul_m(vx)).fsqrt();
                let dot = if l.fcom(x(0.0)) == sa_math::x87::Cmp::Equal {
                    ndx.fadd(vy.fmul_m(ndy))
                } else {
                    let inv = x(1.0).fdiv(l).to_f32();
                    let a = x(inv).fmul_m(vx).fmul(ndx);
                    let b = vy.fmul_m(inv).fmul_m(ndy);
                    a.fadd(b)
                };
                let score = dot.fsub_m(1.0).fmul_m(20.0).fsubr_m(df);
                if lt(score, x(best)) {
                    best = score.to_f32();
                    best_addr = (area as u32 & 0xFFFF) | ((i as u32) << 16);
                }
            }
            i += 1;
        }
    }
    out.write(best_addr);
    out
}

/// `CPathFind::FindNthNodeClosestToCoors(CVector pos, uint8 type, float maxDist, bool lowTraffic, bool unkn,
/// int32 nth, bool boats, bool ignoreInterior, CNodeAddress* outNode)` (0x44F8C0, hidden return): clears bit 1 of
/// `+0x19` over the type's node range of every area (other types: the empty range `[nth, nth)`), marks the
/// `nth` closest nodes one by one (stopping at a miss, which is returned), then returns the next closest
/// (interiors always searched in that final call; `outNode` not written by it; the marks stay set).
#[hook(addr = 0x44F8C0, conv = thiscall, name = "CPathFind::FindNthNodeClosestToCoors", expect = "83 EC 08 53 55", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn find_nth_node_closest_to_coors(
    this: GPtr<u8>,
    out: GPtr<u32>,
    px: f32,
    py: f32,
    pz: f32,
    node_type: u32,
    max_dist: f32,
    low_traffic: u32,
    unkn: u32,
    nth: i32,
    boats: u32,
    ignore_interior: u32,
    out_node: GPtr<u32>,
) -> GPtr<u32> {
    let (mut start, mut end) = (nth, nth);
    for area in 0..0x48 {
        if nodes(this, area).is_null() {
            continue;
        }
        match node_type as u8 {
            0 => {
                end = area_field::<i32>(this, NUM_VEHICLE_NODES, area).read();
                start = 0;
            }
            1 => {
                start = area_field::<i32>(this, NUM_VEHICLE_NODES, area).read();
                end = area_field::<i32>(this, NUM_NODES, area).read();
            }
            _ => {}
        }
        for i in start..end {
            let p = at::<u8>(nodes(this, area), i as isize * 0x1C + 0x19);
            p.write(p.read() & 0xFD);
        }
    }
    let mut n = nth;
    while n > 0 {
        let mut r = [0u32];
        let rp = GPtr::from_raw(r.as_mut_ptr());
        let a = find_node_closest_to_coors_call(this, rp, px, py, pz, node_type, max_dist, low_traffic, unkn, 1, boats, ignore_interior).read();
        if !out_node.is_null() {
            out_node.write(a);
        }
        if a as u16 == 0xFFFF {
            out.write(a);
            return out;
        }
        let f = at::<u8>(node_of(this, a), 0x19);
        f.write(f.read() | 2);
        n -= 1;
    }
    find_node_closest_to_coors_call(this, out, px, py, pz, node_type, max_dist, low_traffic, unkn, 1, boats, 0);
    out
}

/// `CPathFind::FindNodeOrientationForCarPlacement(CNodeAddress)` (0x450320, entry `sub esp,0x28; jmp 0x40A432`,
/// ST0): the first link (up to numLinks - 1) whose navi link has lanes leaving this node, else the last link;
/// `atan2(-d.x, d.y) * 57.295776f` of the normalised 2D direction to it; 0 when not loaded or no links.
#[hook(addr = 0x450320, conv = thiscall, name = "CPathFind::FindNodeOrientationForCarPlacement", expect = "83 EC 28 E9 0A A1 FB FF", this = "CPathFind")]
pub fn find_node_orientation_for_car_placement(this: GPtr<u8>, addr: u32) -> X87 {
    let area = area_of(addr);
    if nodes(this, area).is_null() {
        return x(0.0);
    }
    let n = node_of(this, addr);
    let links = num_links(n);
    if links == 0 {
        return x(0.0);
    }
    let last = links - 1;
    let mut l = 0;
    while l < last {
        let v = navi_link(this, n, l);
        if !nodes(this, (v >> 10) as i32).is_null() {
            let c = navi_node(this, v);
            let b = rd::<u8>(c, 0xB);
            let lanes = if rd::<u16>(c, 4) == addr as u16 && rd::<u16>(c, 6) == (addr >> 16) as u16 {
                b & 0x38
            } else {
                b & 7
            };
            if lanes != 0 {
                break;
            }
        }
        l += 1;
    }
    let t = node_link(this, n, l);
    if nodes(this, area_of(t)).is_null() {
        return x(0.0);
    }
    let tn = node_of(this, t);
    let dx = coord(tn, 8).fsub(coord(n, 8)).to_f32();
    let dy = coord(tn, 0xA).to_f32();
    let dy = x(dy).fsub(coord(n, 0xA)).to_f32();
    let mut v = [dx, dy, 0.0];
    let vp = GPtr::from_raw(v.as_mut_ptr());
    vector_normalise(vp);
    x(vp.read())
        .fchs()
        .fpatan(x(vp.wrapping_index(1).read()))
        .fmul_m(f32::from_bits(0x4265_2EE0))
}

/// `CPathFind::FindParkingNodeInArea(float minX, float maxX, float minY, float maxY, float minZ, float maxZ)`
/// (0x4513F0, hidden `CVector*` return): vehicle nodes of areas 0..63 strictly inside the box with behaviour type 2;
/// returns the one at the rotating index `[0x977B9C]` (advanced, wrapping at the count), else the first, else 0.
#[hook(addr = 0x4513F0, conv = thiscall, name = "CPathFind::FindParkingNodeInArea", expect = "83 EC 48 53 55", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn find_parking_node_in_area(
    this: GPtr<u8>,
    out: GPtr<f32>,
    x0: f32,
    x1: f32,
    y0: f32,
    y1: f32,
    z0: f32,
    z1: f32,
) -> GPtr<f32> {
    let mut count: i32 = 0;
    let (mut first, mut nth) = ([0f32; 3], [0f32; 3]);
    let (mut have_first, mut have_nth) = (false, false);
    let inside = |v: X87, lo: f32, hi: f32| !crate::common::jne_mask(v, x(lo), 0x41) && lt(v, x(hi));
    for a in 0..0x40 {
        let base = nodes(this, a);
        if base.is_null() {
            continue;
        }
        let num = area_field::<i32>(this, NUM_VEHICLE_NODES, a).read();
        for k in 0..num.max(0) {
            let n = base.byte_add(k as isize * 0x1C);
            if !inside(coord(n, 8), x0, x1) || !inside(coord(n, 0xA), y0, y1) || !inside(coord(n, 0xC), z0, z1) {
                continue;
            }
            if rd::<u8>(n, 0x1A) & 0xF0 != 0x20 {
                continue;
            }
            if count == 0 {
                get_position_call(n, GPtr::from_raw(first.as_mut_ptr()));
                have_first = true;
            }
            if count == PARKING_NODE_INDEX.get() {
                get_position_call(n, GPtr::from_raw(nth.as_mut_ptr()));
                have_nth = true;
            }
            count += 1;
        }
    }
    let s = PARKING_NODE_INDEX.get().wrapping_add(1);
    PARKING_NODE_INDEX.set(if s < count { s } else { 0 });
    let v = if !have_first {
        [0.0; 3]
    } else if have_nth {
        nth
    } else {
        first
    };
    for (i, c) in v.into_iter().enumerate() {
        out.wrapping_index(i as i32).write(c);
    }
    out
}

// ---------------------------------------------------------------------------------------------------------
// CPathFind: flags, wandering offsets, interiors

/// `CPathFind::UnMarkAllRoadNodesAsDontWander()` (stub 0x44D400 → 0x1565420): clears bit 2 of `+0x19` of every
/// vehicle node of areas 0..63.
#[hook(addr = 0x44D400, conv = thiscall, name = "CPathFind::UnMarkAllRoadNodesAsDontWander", expect = "E9 1B 80 11 01", this = "CPathFind")]
pub fn un_mark_all_road_nodes_as_dont_wander(this: GPtr<u8>) {
    for a in 0..0x40 {
        if nodes(this, a).is_null() {
            continue;
        }
        let mut i = 0;
        while i < area_field::<i32>(this, NUM_VEHICLE_NODES, a).read() {
            let p = at::<u8>(nodes(this, a), i as isize * 0x1C + 0x19);
            p.write(p.read() & 0xFB);
            i += 1;
        }
    }
}

/// `CPathFind::LoadSceneForPathNodes(CVector)` (stub 0x44DE00 → 0x1565690): clears `ToBeStreamed`, marks 350
/// around the point and requests (flags 0) every marked area, loaded or not.
#[hook(addr = 0x44DE00, conv = thiscall, name = "CPathFind::LoadSceneForPathNodes", expect = "E9 8B 78 11 01", this = "CPathFind")]
pub fn load_scene_for_path_nodes(this: GPtr<u8>, px: f32, py: f32, pz: f32) {
    TO_BE_STREAMED.set([0; 64]);
    mark_regions_for_coors_call(this, px, py, pz, 350.0);
    let t = TO_BE_STREAMED.ptr().cast::<u8>();
    for xx in 0..8 {
        for yy in 0..8 {
            if t.wrapping_index(xx * 8 + yy).read() != 0 {
                request_model(0x63A7 + xx + yy * 8, 0);
            }
        }
    }
}

/// `0.00775f` (0x859A9C): the wander offset per width unit.
fn wander_scale() -> f32 {
    f32::from_bits(0x3BFD_F3B6)
}

/// `CPathFind::TakeWidthIntoAccountForWandering(CNodeAddress, int16 seed)` (0x4509A0, hidden `CVector*` return):
/// the node position plus `((seed & 15) - 7) * width * 0.00775` in x and `(((seed >> 4) & 15) - 7) * width * 0.00775`
/// in y; (0, 0, 0) when the area is 0xFFFF or not loaded.
#[hook(addr = 0x4509A0, conv = thiscall, name = "CPathFind::TakeWidthIntoAccountForWandering", expect = "8B 44 24 08 83 EC 0C", this = "CPathFind")]
pub fn take_width_into_account_for_wandering(this: GPtr<u8>, out: GPtr<f32>, addr: u32, seed: u32) -> GPtr<f32> {
    if addr as u16 == 0xFFFF || nodes(this, area_of(addr)).is_null() {
        out.write(0.0);
        out.wrapping_index(1).write(0.0);
        out.wrapping_index(2).write(0.0);
        return out;
    }
    let n = node_of(this, addr);
    let mut p = [0f32; 3];
    get_position_call(n, GPtr::from_raw(p.as_mut_ptr()));
    let s = i32::from(seed as u16 as i16);
    let w = i32::from(rd::<u8>(n, 0x16));
    let dx = ((s & 0xF) - 7).wrapping_mul(w);
    let dy = (((s >> 4) & 0xF) - 7).wrapping_mul(w);
    out.write(xi(dx).fmul_m(wander_scale()).fadd_m(p[0]).to_f32());
    out.wrapping_index(1).write(xi(dy).fmul_m(wander_scale()).fadd_m(p[1]).to_f32());
    out.wrapping_index(2).write(p[2]);
    out
}

/// `FUN_0044DA30(CNodeAddress a, CNodeAddress b, int16 seed, float* x, float* y)` (0x44DA30, thiscall, `ret 0x14`):
/// the two-node wander offset: `min(width a, width b)` (unsigned) times `(seed & 15) - 7` / `((seed >> 4) & 15) - 7`
/// times 0.00775 added to `*x` / `*y`; nothing when either area is 0xFFFF or not loaded.
#[hook(addr = 0x44DA30, conv = thiscall, name = "FUN_0044da30", expect = "8B 44 24 04 66 3D FF FF", this = "CPathFind")]
pub fn add_wander_offset_between_nodes(this: GPtr<u8>, a: u32, b: u32, seed: u32, out_x: GPtr<f32>, out_y: GPtr<f32>) {
    if a as u16 == 0xFFFF || nodes(this, area_of(a)).is_null() {
        return;
    }
    if b as u16 == 0xFFFF || nodes(this, area_of(b)).is_null() {
        return;
    }
    let na = node_of(this, a);
    let nb = node_of(this, b);
    let s = i32::from(seed as u16 as i16);
    let w = i32::from(rd::<u8>(na, 0x16).min(rd::<u8>(nb, 0x16)));
    out_x.write(xi(((s & 0xF) - 7).wrapping_mul(w)).fmul_m(wander_scale()).fadd_m(out_x.read()).to_f32());
    let w = i32::from(rd::<u8>(na, 0x16).min(rd::<u8>(nb, 0x16)));
    out_y.write(xi((((s >> 4) & 0xF) - 7).wrapping_mul(w)).fmul_m(wander_scale()).fadd_m(out_y.read()).to_f32());
}

/// The intersection flags byte of the link from `a` to `b`, if both areas are loaded and the link exists.
fn intersection_flags(this: GPtr<u8>, a: u32, b: u32) -> Option<u8> {
    if nodes(this, area_of(a)).is_null() || nodes(this, area_of(b)).is_null() {
        return None;
    }
    let n = node_of(this, a);
    let base = i32::from(rd::<i16>(n, 0x10));
    let links = area_field::<GPtr<u32>>(this, NODE_LINKS, area_of(a)).read();
    let count = num_links(n);
    let mut i: i16 = 0;
    while i32::from(i) < count {
        let c = links.wrapping_index(i32::from(i) + base).read();
        if c as u16 == b as u16 && (c >> 16) as u16 == (b >> 16) as u16 {
            let p = area_field::<GPtr<u8>>(this, INTERSECTIONS, area_of(a)).read();
            return Some(p.wrapping_index(i32::from(i) + base).read());
        }
        i = i.wrapping_add(1);
    }
    None
}

/// `CPathFind::TestForPedTrafficLight(CNodeAddress a, CNodeAddress b)` (stub 0x44D480 → 0x156A400): bit 1 of the
/// link's intersection byte.
#[hook(addr = 0x44D480, conv = thiscall, name = "CPathFind::TestForPedTrafficLight", expect = "E9 7B CF 11 01", this = "CPathFind")]
pub fn test_for_ped_traffic_light(this: GPtr<u8>, a: u32, b: u32) -> u8 {
    intersection_flags(this, a, b).map_or(0, |f| (f >> 1) & 1)
}

/// `CPathFind::TestCrossesRoad(CNodeAddress a, CNodeAddress b)` (stub 0x44D790 → 0x1565AF0): bit 0 of the link's
/// intersection byte.
#[hook(addr = 0x44D790, conv = thiscall, name = "CPathFind::TestCrossesRoad", expect = "E9 5B 83 11 01", this = "CPathFind")]
pub fn test_crosses_road(this: GPtr<u8>, a: u32, b: u32) -> u8 {
    intersection_flags(this, a, b).map_or(0, |f| f & 1)
}

/// `CPathFind::SetLinksBridgeLights(float xMin, float xMax, float yMin, float yMax, bool on)` (0x44D960): bit 2 of
/// `+0xC` of every car path link of areas 0..63 strictly inside the rectangle (`x > xMin`, `x < xMax`; NaN fails).
#[hook(addr = 0x44D960, conv = thiscall, name = "CPathFind::SetLinksBridgeLights", expect = "53 8A 5C 24 18", this = "CPathFind")]
pub fn set_links_bridge_lights(this: GPtr<u8>, x0: f32, x1: f32, y0: f32, y1: f32, on: u8) {
    let inside = |v: X87, lo: f32, hi: f32| !crate::common::jne_mask(v, x(lo), 0x41) && lt(v, x(hi));
    for a in 0..0x40 {
        if nodes(this, a).is_null() {
            continue;
        }
        let mut i = 0;
        while i < area_field::<i32>(this, 0x1304, a).read() {
            let c = area_field::<GPtr<u8>>(this, NAVI_NODES, a).read().byte_add(i as isize * 0xE);
            let cx = xi(i32::from(rd::<i16>(c, 0))).fmul_m(COORD_SCALE);
            let cy = xi(i32::from(rd::<i16>(c, 2))).fmul_m(COORD_SCALE);
            if inside(cx, x0, x1) && inside(cy, y0, y1) {
                let b = rd::<u8>(c, 0xC);
                wr(c, 0xC, b ^ (((on << 2) ^ b) & 4));
            }
            i += 1;
        }
    }
}

/// `CPathFind::ThisNodeWillLeadIntoADeadEnd(CPathNode* start, CPathNode* from)` (stub 0x44D310 → 0x1569F30): walks
/// while exactly one neighbour (other than the one it came from) has behaviour type 0 or > 10; two such neighbours
/// → false, none → true. Links come from the absolute `ThePaths`, load checks from `this`.
#[hook(addr = 0x44D310, conv = thiscall, name = "CPathFind::ThisNodeWillLeadIntoADeadEnd", expect = "E9 1B CC 11 01", this = "CPathFind")]
pub fn this_node_will_lead_into_a_dead_end(this: GPtr<u8>, start: GPtr<u8>, from: GPtr<u8>) -> u8 {
    let tp = the_paths();
    let (mut cur, mut prev) = (start, from);
    loop {
        let links = num_links(cur);
        if links <= 0 {
            return 1;
        }
        let mut cand = GPtr::<u8>::NULL;
        let base = i32::from(rd::<i16>(cur, 0x10));
        let arr = area_field::<GPtr<u32>>(tp, NODE_LINKS, i32::from(rd::<u16>(cur, 0x12))).read();
        for i in 0..links {
            let a = arr.wrapping_index(base + i).read();
            let np = nodes(this, area_of(a));
            if np.is_null() {
                continue;
            }
            let l = np.byte_add(index_of(a) as isize * 0x1C);
            if l.is_null() || l == prev {
                continue;
            }
            let t = rd::<u8>(l, 0x1A) >> 4;
            if t != 0 && t <= 10 {
                continue;
            }
            if !cand.is_null() {
                return 0;
            }
            cand = l;
        }
        if cand.is_null() {
            return 1;
        }
        prev = cur;
        cur = cand;
    }
}

/// `CPathFind::AddNodeToNewInterior(float x, float y, float z, bool dontWander, int8 c0..c5)` (0x450E90, hidden
/// return; entry `nop; jmp 0x401FD8`, where protection code loads ECX = `[[0x1560704] ^ [0x52FE18]]` =
/// `[0x96EF80]` = `NumNodesGiven`, from the static image): stores the node's coordinates, don't-wander byte and
/// six connections, returns `(0x40 + NewInteriorSlot, n)` and increments `NumNodesGiven`.
#[hook(addr = 0x450E90, conv = thiscall, name = "CPathFind::AddNodeToNewInterior", expect = "90 E9 42 11 FB FF", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn add_node_to_new_interior(
    _this: GPtr<u8>,
    out: GPtr<u32>,
    px: f32,
    py: f32,
    pz: f32,
    dont_wander: u8,
    c0: u8,
    c1: u8,
    c2: u8,
    c3: u8,
    c4: u8,
    c5: u8,
) -> GPtr<u32> {
    let n = NUM_NODES_GIVEN.get();
    let c = COOR_GIVEN.ptr().cast::<f32>();
    c.wrapping_index(128 + n).write(x(px).to_f32());
    DONT_WANDER_GIVEN.ptr().cast::<u8>().wrapping_index(n).write(dont_wander);
    c.wrapping_index(64 + n).write(x(py).to_f32());
    c.wrapping_index(n).write(x(pz).to_f32());
    let row = CONNECTS_TO_GIVEN.ptr().cast::<u8>().wrapping_index(n.wrapping_mul(6));
    for (k, v) in [c0, c1, c2, c3, c4, c5].into_iter().enumerate() {
        row.wrapping_index(k as i32).write(v);
    }
    let r = (NEW_INTERIOR_SLOT.get().wrapping_add(0x40) as u32 & 0xFFFF) | ((n as u32) << 16);
    NUM_NODES_GIVEN.set(n.wrapping_add(1));
    out.write(r);
    out
}

/// `CPathFind::FindNextNodeWandering(uint8 type, CVector pos, CNodeAddress* origin, CNodeAddress* target, uint8 dir,
/// uint8* outDir)` (0x451B70, `ret 0x20`; spec CPathFind.md). The start is `*origin` when loaded and within
/// `max(width*0.125, 7)` of `pos` (3D), else the closest node of `type`; the best link by `dot(normalised link,
/// (sin, cos)(dir * 45 deg))` becomes the target with its 8-way direction; `*origin = start` (no NULL check); finally
/// a target equal to the original origin is replaced by its first loaded link. The original's origin local has
/// only its area word set when `origin` is NULL (the node word is never compared then).
#[hook(addr = 0x451B70, conv = thiscall, name = "CPathFind::FindNextNodeWandering", expect = "83 EC 1C 53 55", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn find_next_node_wandering(
    this: GPtr<u8>,
    node_type: u32,
    px: f32,
    py: f32,
    pz: f32,
    origin: GPtr<u32>,
    target: GPtr<u32>,
    dir: u32,
    out_dir: GPtr<u8>,
) {
    let tw = target.cast::<u16>();
    let mut orig: u32 = 0x0000_FFFF;
    let mut start: Option<u32> = None;
    if !origin.is_null() {
        orig = origin.read();
        if orig as u16 != 0xFFFF {
            if nodes(this, area_of(orig)).is_null() {
                tw.write(0xFFFF);
                out_dir.write(0);
                return;
            }
            let n = node_of(this, orig);
            let mut t = [0f32; 3];
            let p = get_position_call(n, GPtr::from_raw(t.as_mut_ptr()));
            let dx = x(px).fsub_m(p.read());
            let dy = x(py).fsub_m(p.wrapping_index(1).read());
            let dz = x(pz).fsub_m(p.wrapping_index(2).read());
            let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).to_f32();
            let mut r = xi(i32::from(rd::<u8>(n, 0x16))).fmul_m(COORD_SCALE);
            if !crate::common::jne_mask(x(7.0), r, 0x41) {
                r = x(7.0);
            }
            if x(d2).fsqrt().fcom(r) != sa_math::x87::Cmp::Greater {
                start = Some(orig);
            }
        }
    }
    let mut start = match start {
        Some(s) => s,
        None => {
            let mut r = [0u32];
            let rp = GPtr::from_raw(r.as_mut_ptr());
            find_node_closest_to_coors_call(this, rp, px, py, pz, node_type, f32::from_bits(0x4974_23FE), 0, 0, 0, 0, 0).read()
        }
    };
    let invalid = start as u16 == 0xFFFF || nodes(this, area_of(start)).is_null();
    if invalid {
        tw.write(0xFFFF);
        start = (start & 0xFFFF_0000) | 0xFFFF;
        out_dir.write(0);
        target.write(start);
    } else {
        let n = node_of(this, start);
        let a = xi(i32::from(dir as u8)).fmul_m(f32::from_bits(0x3F49_0FDB));
        tw.write(0xFFFF);
        let mut best = f32::from_bits(0xC974_23F0);
        let s = a.fsin().to_f32();
        let c = a.fcos().to_f32();
        let mut l: i16 = 0;
        if num_links(n) != 0 {
            loop {
                let a2 = node_link(this, n, i32::from(l));
                let lp = nodes(this, area_of(a2));
                if !lp.is_null() {
                    let ln = lp.byte_add(index_of(a2) as isize * 0x1C);
                    let off = rd::<u8>(n, 0x18) & 0x20 == 0 && rd::<u8>(ln, 0x18) & 0x20 != 0;
                    let dw = rd::<u8>(n, 0x19) & 4 == 0 && rd::<u8>(ln, 0x19) & 4 != 0;
                    if !off && !dw {
                        let vx = coord(ln, 8).fsub(coord(n, 8));
                        let vy = coord(ln, 0xA).fsub(coord(n, 0xA));
                        let inv = vy.fmul(vy).fadd(vx.fmul(vx)).fsqrt().fdivr_m(1.0);
                        let nx = inv.fmul(vx).to_f32();
                        let ny = inv.fmul(vy).to_f32();
                        let score = x(ny).fmul_m(c).fadd(x(nx).fmul_m(s));
                        if !score.fcom(x(best)).c0() {
                            best = score.to_f32();
                            target.write(a2);
                            out_dir.write(wander_direction(nx, ny));
                        }
                    }
                }
                l = l.wrapping_add(1);
                if i32::from(l) >= num_links(n) {
                    break;
                }
            }
        }
        origin.write(start);
        if tw.read() == 0xFFFF {
            out_dir.write(0);
            target.write(start);
        }
    }
    if tw.read() == orig as u16 && tw.wrapping_index(1).read() == (orig >> 16) as u16 && orig as u16 != 0xFFFF {
        let base = nodes(this, area_of(orig));
        if !base.is_null() {
            let n = base.byte_add(index_of(orig) as isize * 0x1C);
            for k in 0..num_links(n) {
                let c = node_link(this, n, k);
                if !nodes(this, area_of(c)).is_null() {
                    target.write(c);
                    return;
                }
            }
        }
    }
}

/// The 8-way direction `FindNextNodeWandering` stores for the normalised link `(nx, ny)` (strict compares, NaN
/// false).
fn wander_direction(nx: f32, ny: f32) -> u8 {
    let (vx, vy) = (x(nx), x(ny));
    let less = |a: X87, b: X87| lt(a, b);
    let greater = |a: X87, b: X87| a.fcom(b) == sa_math::x87::Cmp::Greater;
    if !vx.fcom(x(0.0)).c0() {
        if less(fabs(vy).fdouble(), vx) {
            2
        } else if less(vx.fdouble(), vy) {
            0
        } else if greater(vx.fmul_m(-2.0), vy) {
            4
        } else if greater(vy, x(0.0)) {
            1
        } else {
            3
        }
    } else if less(fabs(vy).fdouble(), vx.fchs()) {
        6
    } else if less(vx.fmul_m(-2.0), vy) {
        0
    } else if greater(vx.fdouble(), vy) {
        4
    } else if greater(vy, x(0.0)) {
        7
    } else {
        5
    }
}

orig! {
    /// `CPathFind::FindRegionForCoors(float, float)` (0x44D830)
    pub fn find_region_for_coors_call(this: GPtr<u8>, px: f32, py: f32) -> i32 = thiscall 0x44D830 => find_region_for_coors;
    /// `CWorld::FindGroundZFor3DCoord(float x, float y, float z, bool* found, CEntity** outEntity)` (0x5696C0)
    pub fn find_ground_z_for_3d_coord_st0(px: f32, py: f32, pz: f32, found: GPtr<u8>, ent: GPtr<GPtr<u8>>) -> f64 = cdecl 0x5696C0 => unported;
}

/// `CPathFind::GeneratePedCreationCoors_Interior(float x, float y, CVector* outCoords, CNodeAddress*,
/// CNodeAddress*, float* outOrientation)` (0x44ECA0; spec CPathFind.md). 200 attempts in the region of (x, y): a
/// random ped node (`rand()` at 0x44ECF3) within 50 (2D) and above z 900, then for each link into a loaded
/// non-interior area up to 5 random interpolation factors (`rand()` at 0x44EE09; the factor is what
/// `*outOrientation` receives) whose point has ground within 3.0 below `z + 2`. The `found` flag lives in the
/// `outCoords` argument slot in the original; it starts here with that pointer's low byte, as there.
#[hook(addr = 0x44ECA0, conv = thiscall, name = "CPathFind::GeneratePedCreationCoors_Interior", expect = "83 EC 60 8B 44 24 68", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn generate_ped_creation_coors_interior(
    this: GPtr<u8>,
    px: f32,
    py: f32,
    out: GPtr<f32>,
    _unused1: GPtr<u32>,
    _unused2: GPtr<u32>,
    out_orientation: GPtr<f32>,
) -> u8 {
    let r = find_region_for_coors_call(this, px, py);
    for _attempt in 0..200 {
        let base = nodes(this, r);
        let num_ped = area_field::<i32>(this, NUM_PED_NODES, r).read();
        if base.is_null() || num_ped == 0 {
            continue;
        }
        let idx = (crate::common::rand_at(0x44ECF3) >> 6)
            .wrapping_rem(num_ped)
            .wrapping_add(area_field::<i32>(this, NUM_VEHICLE_NODES, r).read());
        let n = nodes(this, r).byte_add(idx as isize * 0x1C);
        let nx = coord(n, 8);
        let nxf = nx.to_f32();
        let ny = coord(n, 0xA);
        let y2 = ny.fsub_m(py).fmul(ny.fsub_m(py));
        let x2 = x(nxf).fsub_m(px).fmul(nx.fsub_m(px));
        if !lt(y2.fadd(x2), x(2500.0)) {
            continue;
        }
        if coord(n, 0xC).fcom(x(900.0)) != sa_math::x87::Cmp::Greater {
            continue;
        }
        let links = num_links(n);
        if links == 0 {
            continue;
        }
        for l in 0..links {
            let a = area_field::<GPtr<u32>>(this, NODE_LINKS, r)
                .read()
                .wrapping_index(i32::from(rd::<i16>(n, 0x10)) + l)
                .read();
            if a as u16 >= 0x40 {
                continue;
            }
            let lp = nodes(this, area_of(a));
            if lp.is_null() {
                continue;
            }
            let ln = lp.byte_add(index_of(a) as isize * 0x1C);
            for _try in 0..5 {
                let t = xi(crate::common::rand_at(0x44EE09) & 0xFF).fmul_m(f32::from_bits(0x3B80_0000));
                out_orientation.write(t.to_f32());
                let tf = t.to_f32();
                let lx = coord(ln, 8).fmul_m(tf).to_f32();
                let ly = coord(ln, 0xA).fmul_m(tf);
                let lz = x(coord(ln, 0xC).to_f32()).fmul_m(tf).to_f32();
                let s = x(1.0).fsub(t).to_f32();
                let nxs = coord(n, 8).fmul_m(s).to_f32();
                let nys = coord(n, 0xA).fmul_m(s);
                let nzs = x(coord(n, 0xC).to_f32()).fmul_m(s).to_f32();
                let p = [
                    x(nxs).fadd_m(lx).to_f32(),
                    nys.fadd(ly).to_f32(),
                    x(nzs).fadd_m(lz).to_f32(),
                ];
                for (k, v) in p.into_iter().enumerate() {
                    out.wrapping_index(k as i32).write(v);
                }
                let mut found = [out.addr() as u8];
                let zq = x(out.wrapping_index(2).read()).fadd_m(2.0).to_f32();
                let z = find_ground_z_for_3d_coord(
                    out.read(),
                    out.wrapping_index(1).read(),
                    zq,
                    GPtr::from_raw(found.as_mut_ptr()),
                    GPtr::NULL,
                );
                if found[0] != 0 && lt(fabs(z.fsub_m(out.wrapping_index(2).read())), x(3.0)) {
                    out.wrapping_index(2).write(z.to_f32());
                    return 1;
                }
            }
        }
    }
    0
}

orig! {
    /// `CVector2D::Normalise()` (0x44E480)
    pub fn vector2d_normalise(v: GPtr<f32>) = thiscall 0x44E480 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, hidden return)
    pub fn vector_mul(out: GPtr<f32>, v: GPtr<f32>, f: f32) -> GPtr<f32> = cdecl 0x40FEC0 => unported;
    /// `operator+(const CVector&, const CVector&)` (0x40FE30, hidden return)
    pub fn vector_add(out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>) -> GPtr<f32> = cdecl 0x40FE30 => unported;
}

/// `CPathFind::FindNodeCoorsForScript(CNodeAddress, bool* found)` (0x4505E0, hidden `CVector*` return): (0, 0, 0)
/// and `*found = 0` when the area is 0xFFFF or not loaded; else `*found = 1` and the node position, moved for a
/// node with a width and a loaded first link sideways (perpendicular to the normalised 2D direction to that link:
/// `(v.y, -v.x)` when `v.x < 0`, else `(-v.y, v.x)`) by `width * 0.0625 + 2.7`. The first link's base id is read
/// from the absolute `ThePaths`, the links array from `this`.
#[hook(addr = 0x4505E0, conv = thiscall, name = "CPathFind::FindNodeCoorsForScript-LinkedNode", expect = "8B 44 24 08 83 EC 24", this = "CPathFind")]
pub fn find_node_coors_for_script(this: GPtr<u8>, out: GPtr<f32>, addr: u32, found: GPtr<u8>) -> GPtr<f32> {
    if addr as u16 == 0xFFFF || nodes(this, area_of(addr)).is_null() {
        if !found.is_null() {
            found.write(0);
        }
        out.write(0.0);
        out.wrapping_index(1).write(0.0);
        out.wrapping_index(2).write(0.0);
        return out;
    }
    if !found.is_null() {
        found.write(1);
    }
    let n = node_of(this, addr);
    let width = rd::<u8>(n, 0x16);
    if width == 0 || num_links(n) == 0 {
        get_position_call(n, out);
        return out;
    }
    let base = i32::from(rd::<i16>(node_of(the_paths(), addr), 0x10));
    let a = area_field::<GPtr<u32>>(this, NODE_LINKS, area_of(addr)).read().wrapping_index(base).read();
    let lp = nodes(this, area_of(a));
    if lp.is_null() {
        get_position_call(n, out);
        return out;
    }
    let l = lp.byte_add(index_of(a) as isize * 0x1C);
    let (mut t1, mut t2, mut t3) = ([0f32; 3], [0f32; 3], [0f32; 3]);
    let p1 = get_position_call(l, GPtr::from_raw(t1.as_mut_ptr()));
    let p2 = get_position_call(n, GPtr::from_raw(t2.as_mut_ptr()));
    let mut v = [0f32; 3];
    v[0] = x(p1.read()).fsub_m(p2.read()).to_f32();
    let p3 = get_position_call(l, GPtr::from_raw(t2.as_mut_ptr()));
    let p4 = get_position_call(n, GPtr::from_raw(t3.as_mut_ptr()));
    v[1] = x(p3.wrapping_index(1).read()).fsub_m(p4.wrapping_index(1).read()).to_f32();
    let vp = GPtr::from_raw(v.as_mut_ptr());
    vector2d_normalise(vp);
    let (vx, vy) = (x(vp.read()), x(vp.wrapping_index(1).read()));
    let (nx, ny) = if lt(vx, x(0.0)) { (vy, vx.fchs()) } else { (vy.fchs(), vx) };
    v = [nx.to_f32(), ny.to_f32(), 0.0];
    let f = xi(i32::from(width)).fmul_m(0.0625).fadd_m(f32::from_bits(0x402C_CCCD)).to_f32();
    let mut scaled = [0f32; 3];
    let s = vector_mul(GPtr::from_raw(scaled.as_mut_ptr()), GPtr::from_raw(v.as_mut_ptr()), f);
    let mut t4 = [0f32; 3];
    let pos = get_position_call(n, GPtr::from_raw(t4.as_mut_ptr()));
    vector_add(out, pos, s);
    out
}

/// `CPathFind::AddDynamicLinkBetween2Nodes_For1Node(CNodeAddress a, CNodeAddress b)` (stub 0x44E000 → 0x1560C20):
/// a node whose links still sit in the static part (`base < m_anNumAddresses[area]`, +0x1424) first moves them to
/// the first free 12-slot block after it (area word 0xFFFF; links, lengths and intersections copied) and, for
/// areas below 64, records (old base, new base) in the first negative slot of the 16-entry rows at
/// `+0x1544 / +0x2544 + area*0x40`; then appends `b` with length 5 and intersection 0 and bumps the link count.
#[hook(addr = 0x44E000, conv = thiscall, name = "CPathFind::AddDynamicLinkBetween2Nodes_For1Node", expect = "E9 1B 2C 11 01", this = "CPathFind")]
pub fn add_dynamic_link_between_2_nodes_for_1_node(this: GPtr<u8>, a: u32, b: u32) {
    let area = area_of(a);
    let num_addr = area_field::<i32>(this, 0x1424, area).read();
    let n = node_of(this, a);
    let base = i32::from(rd::<i16>(n, 0x10));
    let links = || area_field::<GPtr<u32>>(this, NODE_LINKS, area).read();
    let lengths = || area_field::<GPtr<u8>>(this, LINK_LENGTHS, area).read();
    let inter = || area_field::<GPtr<u8>>(this, INTERSECTIONS, area).read();
    let new_base = if base >= num_addr {
        base
    } else {
        let mut k = 0;
        while links().wrapping_index(num_addr + 12 * k).cast::<u16>().read() != 0xFFFF {
            k += 1;
        }
        let nb = num_addr + 12 * k;
        let mut l = 0;
        while l < num_links(n) {
            links().wrapping_index(nb + l).write(links().wrapping_index(base + l).read());
            lengths().wrapping_index(nb + l).write(lengths().wrapping_index(base + l).read());
            inter().wrapping_index(nb + l).write(inter().wrapping_index(base + l).read());
            l += 1;
        }
        if area < 0x40 {
            let row = at::<i32>(this, 0x1544 + (area << 6) as isize);
            for s in 0..16 {
                if row.wrapping_index(s).read() < 0 {
                    row.wrapping_index(s).write(i32::from(rd::<i16>(n, 0x10)));
                    row.wrapping_index(s + 0x400).write(nb);
                    break;
                }
            }
        }
        nb
    };
    let cnt = num_links(n);
    links().wrapping_index(new_base + cnt).write(b);
    lengths().wrapping_index(new_base + cnt).write(5);
    inter().wrapping_index(new_base + cnt).write(0);
    let f = rd::<u8>(n, 0x18);
    wr::<i16>(n, 0x10, new_base as i16);
    wr(n, 0x18, (f & 0xF0) | (f.wrapping_add(1) & 0xF));
}

/// `CPathFind::RemoveInterior(uint32 id)` (stub 0x44E1A0 → 0x1564320). For each interior slot holding `id` (set to
/// -1): every ped node of areas 0..63 with dynamic links (`base >= m_anNumAddresses`) drops its links into area
/// `0x40 + slot`. Follows the bytes: the "shift the remaining links down" loop indexes `this + 0xA44` directly
/// (`ThePaths.m_pNodeLinks[base + m] = m_pNodeLinks[base + m + 1]` as dwords of the object, not the area's link
/// array). When a link was removed and no other interior link remains, the node's 12 link area words become 0xFFFF
/// and its base is restored from the (old, new) rows at `+0x1544 / +0x2544` (matched on the new base; both
/// cleared to -1). Then the slot's nodes, links, lengths and intersections are freed and its five counts zeroed.
#[hook(addr = 0x44E1A0, conv = thiscall, name = "CPathFind::RemoveInterior", expect = "E9 7B 61 11 01", this = "CPathFind")]
pub fn remove_interior(this: GPtr<u8>, id: i32) {
    for slot in 0..8 {
        if area_field::<i32>(this, INTERIOR_IDS, slot).read() != id {
            continue;
        }
        area_field::<i32>(this, INTERIOR_IDS, slot).write(-1);
        for a in 0..0x40 {
            if nodes(this, a).is_null() {
                continue;
            }
            let mut k = area_field::<i32>(this, NUM_VEHICLE_NODES, a).read();
            while k < area_field::<i32>(this, NUM_NODES, a).read() {
                let n = nodes(this, a).byte_add(k as isize * 0x1C);
                k += 1;
                if i32::from(rd::<i16>(n, 0x10)) < area_field::<i32>(this, 0x1424, a).read() {
                    continue;
                }
                let mut removed = false;
                let mut other = false;
                let mut l = 0;
                while l < num_links(n) {
                    let c = area_field::<GPtr<u32>>(this, NODE_LINKS, a)
                        .read()
                        .wrapping_index(i32::from(rd::<i16>(n, 0x10)) + l)
                        .read();
                    if (c & 0xFFFF) as i32 == slot + 0x40 {
                        let mut m = l;
                        while m < num_links(n) - 1 {
                            let idx = i32::from(rd::<i16>(n, 0x10)) + m;
                            let v = at::<u32>(this, 0xA48 + 4 * idx as isize).read();
                            at::<u32>(this, 0xA44 + 4 * idx as isize).write(v);
                            m += 1;
                        }
                        let f = rd::<u8>(n, 0x18);
                        wr(n, 0x18, (f & 0xF0) | (f.wrapping_sub(1) & 0xF));
                        l -= 1;
                        removed = true;
                    } else if c as u16 >= 0x40 {
                        other = true;
                    }
                    l += 1;
                }
                if removed && !other {
                    for e in 0..12 {
                        let p = area_field::<GPtr<u32>>(this, NODE_LINKS, a)
                            .read()
                            .wrapping_index(i32::from(rd::<i16>(n, 0x10)) + e);
                        p.cast::<u16>().write(0xFFFF);
                    }
                    let base = i32::from(rd::<i16>(n, 0x10));
                    let row1 = at::<i32>(this, 0x1544 + (a << 6) as isize);
                    for s in 0..16 {
                        if row1.wrapping_index(s + 0x400).read() == base {
                            wr::<u16>(n, 0x10, row1.wrapping_index(s).cast::<u16>().read());
                            row1.wrapping_index(s).write(-1);
                            row1.wrapping_index(s + 0x400).write(-1);
                            break;
                        }
                    }
                }
            }
        }
        let a = 0x40 + slot;
        for base in [PATH_NODES, NODE_LINKS, LINK_LENGTHS, INTERSECTIONS] {
            mem_free(area_field::<GPtr<u8>>(this, base, a).read());
            area_field::<GPtr<u8>>(this, base, a).write(GPtr::NULL);
        }
        for base in [NUM_NODES, NUM_VEHICLE_NODES, NUM_PED_NODES, 0x1304, 0x1424] {
            area_field::<i32>(this, base, a).write(0);
        }
    }
}

orig! {
    /// `CCamera::IsSphereVisible(const CVector&, float, RwMatrix*)` (0x420C40)
    pub fn camera_is_sphere_visible_m(cam: GPtr<u8>, c: GPtr<f32>, r: f32, m: GPtr<u8>) -> u8 = thiscall 0x420C40 => unported;
    /// `CCamera::IsSphereVisible(const CVector&, float)` (0x420D40)
    pub fn camera_is_sphere_visible(cam: GPtr<u8>, c: GPtr<f32>, r: f32) -> u8 = thiscall 0x420D40 => unported;
}

/// `FUN_0044E790`: gr's `CPathFind::GeneratePedCreationCoors(float x, float y, float minDist1, float maxDist1,
/// float minDist2, float maxDist2, CVector* outCoords, CNodeAddress* outA, CNodeAddress* outB, float* outOrientation,
/// bool lowTraffic, CMatrix* matrix)` (0x44E790, thiscall, `ret 0x30`). A density threshold
/// `ftol((rand() & 0xFFFF) / 32768 * 15)` (rand at 0x44E7A5); 300 attempts: a random ped node of the region of
/// (x, y) (rand at 0x44E833) within `maxDist1 + 30` (2D, squared compare) whose `+0x1A` low nibble exceeds the
/// threshold; for each link not crossing a road (intersection bit 0) into a loaded non-interior area (switched-off
/// ends only with `lowTraffic`), with the same density test, and with the node or the link end within `maxDist1`:
/// up to 5 random points between them (rand at 0x44EA3D; the factor goes to `*outOrientation`). A point visible
/// to the camera (radius 2, through `matrix` when given) must lie strictly between minDist1 and maxDist1, an
/// invisible one between minDist2 and maxDist2 and pass `rand() & 1` (rand at 0x44EBB6). The first accepted point
/// is written with both addresses; with ground found it returns whether `|groundZ - z| <= 3` (writing the ground
/// z), without ground the next try follows.
#[hook(addr = 0x44E790, conv = thiscall, name = "FUN_0044e790", expect = "81 EC C0 00 00 00", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn generate_ped_creation_coors(
    this: GPtr<u8>,
    px: f32,
    py: f32,
    min1: f32,
    max1: f32,
    min2: f32,
    max2: f32,
    out: GPtr<f32>,
    out_a: GPtr<u32>,
    out_b: GPtr<u32>,
    out_orientation: GPtr<f32>,
    low_traffic: u8,
    matrix: GPtr<u8>,
) -> u8 {
    use crate::common::rand_at;
    let r0 = rand_at(0x44E7A5) & 0xFFFF;
    let density = ftol32(xi(r0).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(15.0));
    let lim = x(max1).fadd_m(30.0);
    let lim2 = lim.fmul(lim).to_f32();
    let r = find_region_for_coors_call(this, px, py);
    let gt = |a: X87, b: f32| a.fcom(x(b)) == sa_math::x87::Cmp::Greater;
    for _attempt in 0..300 {
        if nodes(this, r).is_null() {
            continue;
        }
        let num_ped = area_field::<i32>(this, NUM_PED_NODES, r).read();
        if num_ped == 0 {
            continue;
        }
        let idx = (rand_at(0x44E833) >> 6)
            .wrapping_rem(num_ped)
            .wrapping_add(area_field::<i32>(this, NUM_VEHICLE_NODES, r).read());
        let n = nodes(this, r).byte_add(idx as isize * 0x1C);
        let nx = coord(n, 8);
        let nxf = nx.to_f32();
        let ny = coord(n, 0xA);
        let nyf = ny.to_f32();
        let y2 = ny.fsub_m(py).fmul(x(nyf).fsub_m(py));
        let x2 = nx.fsub_m(px).fmul(x(nxf).fsub_m(px));
        let d2 = y2.fadd(x2).to_f32();
        if !lt(x(d2), x(lim2)) {
            continue;
        }
        if i32::from(rd::<u8>(n, 0x1A) & 0xF) <= density {
            continue;
        }
        let links = num_links(n);
        if links == 0 {
            continue;
        }
        // the squared distance slot [esp+0x20] is overwritten by its square root for every link that gets
        // that far (0x44E98E), so the second such link takes the root of the root (original bug, kept)
        let mut d_slot = d2;
        for l in 0..links {
            let slot = i32::from(rd::<i16>(n, 0x10)) + l;
            if area_field::<GPtr<u8>>(this, INTERSECTIONS, r).read().wrapping_index(slot).read() & 1 != 0 {
                continue;
            }
            let a = area_field::<GPtr<u32>>(this, NODE_LINKS, r).read().wrapping_index(slot).read();
            if a as u16 >= 0x40 {
                continue;
            }
            let lp = nodes(this, area_of(a));
            if lp.is_null() {
                continue;
            }
            let ln = lp.byte_add(index_of(a) as isize * 0x1C);
            if (rd::<u8>(n, 0x18) & 0x20 != 0 || rd::<u8>(ln, 0x18) & 0x20 != 0) && low_traffic == 0 {
                continue;
            }
            if i32::from(rd::<u8>(ln, 0x1A) & 0xF) <= density {
                continue;
            }
            let dn = x(d_slot).fsqrt().to_f32();
            d_slot = dn;
            let (mut t1, mut t2, mut t3, mut t4) = ([0f32; 3], [0f32; 3], [0f32; 3], [0f32; 3]);
            let p1 = get_position_call(ln, GPtr::from_raw(t1.as_mut_ptr()));
            let p2 = get_position_call(ln, GPtr::from_raw(t2.as_mut_ptr()));
            let p3 = get_position_call(ln, GPtr::from_raw(t3.as_mut_ptr()));
            let ly = x(p3.wrapping_index(1).read()).fsub_m(py).to_f32();
            let p4 = get_position_call(ln, GPtr::from_raw(t4.as_mut_ptr()));
            let dy2 = x(p4.wrapping_index(1).read()).fsub_m(py).fmul_m(ly);
            let dx2 = x(p2.read()).fsub_m(px).fmul(x(p1.read()).fsub_m(px));
            let dl2 = dy2.fadd(dx2);
            if !lt(x(dn), x(max1)) && !lt(dl2.fsqrt(), x(max1)) {
                continue;
            }
            for _try in 0..5 {
                let f = xi(rand_at(0x44EA3D) & 0xFF).fmul_m(f32::from_bits(0x3B80_0000));
                let ff = f.to_f32();
                out_orientation.write(ff);
                let mut t5 = [0f32; 3];
                let lpos = get_position_call(ln, GPtr::from_raw(t5.as_mut_ptr()));
                let flx = x(ff).fmul_m(lpos.read()).to_f32();
                let fly = x(ff).fmul_m(lpos.wrapping_index(1).read()).to_f32();
                let flz = x(ff).fmul_m(lpos.wrapping_index(2).read()).to_f32();
                let s = x(1.0).fsub_m(ff).to_f32();
                let mut t6 = [0f32; 3];
                let npos = get_position_call(n, GPtr::from_raw(t6.as_mut_ptr()));
                let snx = x(s).fmul_m(npos.read());
                let sny = x(s).fmul_m(npos.wrapping_index(1).read());
                let snz = x(s).fmul_m(npos.wrapping_index(2).read()).to_f32();
                let mut p = [
                    snx.fadd_m(flx).to_f32(),
                    sny.fadd_m(fly).to_f32(),
                    x(snz).fadd_m(flz).to_f32(),
                ];
                let dx = x(p[0]).fsub_m(px);
                let dy = x(p[1]).fsub_m(py);
                let dist = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt().to_f32();
                let pp = GPtr::from_raw(p.as_mut_ptr());
                let cam = g::<u8>(0xB6F028);
                let ok = if !matrix.is_null() {
                    if camera_is_sphere_visible_m(cam, pp, 2.0, matrix) != 0 {
                        gt(x(dist), min1) && lt(x(dist), x(max1))
                    } else {
                        gt(x(dist), min2) && lt(x(dist), x(max2)) && rand_at(0x44EBB6) & 1 != 0
                    }
                } else if camera_is_sphere_visible(cam, pp, 2.0) != 0 {
                    gt(x(dist), min1) && lt(x(dist), x(max1))
                } else {
                    gt(x(dist), min2) && lt(x(dist), x(max2)) && rand_at(0x44EBB6) & 1 != 0
                };
                if !ok {
                    continue;
                }
                out_a.write(rd::<u32>(n, 0x12));
                out_b.write(rd::<u32>(ln, 0x12));
                out.write(p[0]);
                out.wrapping_index(1).write(p[1]);
                out.wrapping_index(2).write(p[2]);
                let mut found = [0u8];
                let zq = x(p[2]).fadd_m(2.0).to_f32();
                let z = find_ground_z_for_3d_coord(p[0], p[1], zq, GPtr::from_raw(found.as_mut_ptr()), GPtr::NULL);
                if found[0] == 0 {
                    continue;
                }
                if gt(fabs(z.fsub_m(p[2])), 3.0) {
                    return 0;
                }
                out.wrapping_index(2).write(z.to_f32());
                return 1;
            }
        }
    }
    0
}

/// `FUN_0044FB60` (0x44FB60, thiscall, `ret 0x28`): `(const CVector* pos, float radius, uint8 type, int32 max,
/// CNodeAddress* out, bool lowTraffic, int32 unused, bool unkn, bool boats, int32 unused)`, returns the count. Over
/// all 72 areas, the type's nodes (0 vehicle, 1 ped; another type uses an uninitialised local as the start in the
/// original, the empty range here) passing the same filters as `FindNodeClosestInRegion` and within `radius` (3D,
/// squared compare, strict) are appended to `out`; returns as soon as the count equals `max` (checked after every
/// node that passes the boat filter).
#[hook(addr = 0x44FB60, conv = thiscall, name = "FUN_0044fb60", expect = "83 EC 28 53 55", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn find_nodes_within_radius(
    this: GPtr<u8>,
    pos: GPtr<f32>,
    radius: f32,
    node_type: u32,
    max: i32,
    out: GPtr<u32>,
    low_traffic: u8,
    _unused: u32,
    unkn: u8,
    boats: u32,
    _unused2: u32,
) -> i32 {
    let mut count = 0;
    let (mut start, mut end) = (boats as i32, boats as i32);
    for area in 0..0x48 {
        if nodes(this, area).is_null() {
            continue;
        }
        match node_type as u8 {
            0 => {
                start = 0;
                end = area_field::<i32>(this, NUM_VEHICLE_NODES, area).read();
            }
            1 => {
                start = area_field::<i32>(this, NUM_VEHICLE_NODES, area).read();
                end = area_field::<i32>(this, NUM_NODES, area).read();
            }
            _ => {}
        }
        let r2 = x(radius).fmul_m(radius).to_f32();
        let mut i = start;
        while i < end {
            let n = nodes(this, area).byte_add(i as isize * 0x1C);
            let skip = (low_traffic != 0 && rd::<u8>(n, 0x18) & 0x20 != 0)
                || (unkn != 0 && rd::<u8>(n, 0x19) & 2 != 0)
                || boats as u8 != rd::<u8>(n, 0x18) >> 7;
            if !skip {
                let nx = coord(n, 8);
                let ny = coord(n, 0xA);
                let nz = coord(n, 0xC).to_f32();
                let dx = x(pos.read()).fsub(nx).to_f32();
                let dy = x(pos.wrapping_index(1).read()).fsub(ny);
                let dz = x(pos.wrapping_index(2).read()).fsub_m(nz);
                let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(x(dx).fmul_m(dx));
                if lt(d, x(r2)) {
                    out.wrapping_index(count).write((area as u32 & 0xFFFF) | ((i as u32) << 16));
                    count += 1;
                }
                if count == max {
                    return count;
                }
            }
            i += 1;
        }
    }
    count
}

/// The 3D distance `FUN_00450FB0` computes from the raw node coordinates (dx stored f32, dy / dz in registers).
fn interior_link_distance(a: GPtr<u8>, b: GPtr<u8>) -> X87 {
    let bz = coord(b, 0xC).to_f32();
    let ay = coord(a, 0xA).to_f32();
    let az = coord(a, 0xC).to_f32();
    let dx = coord(a, 8).fsub(coord(b, 8)).to_f32();
    let dy = x(ay).fsub(coord(b, 0xA));
    let dz = x(az).fsub_m(bz);
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(x(dx).fmul_m(dx)).fsqrt()
}

/// `FUN_00450FB0(int32 areaA, int8 nodeA, int32 areaB, int32 nodeB, int32 slot, int32* count)` (0x450FB0, cdecl):
/// the interior-building link helper on the absolute `ThePaths`: link `count` of interior area `0x40 + slot`
/// becomes (areaB, nodeB); its length is `ftol(min(dist(A, B), 255))` as a byte (1 when that byte is 0; the
/// second evaluation uses the `GetPosition` distance), its intersection byte 0; node `nodeA` of area
/// `0x40 + slot` gets one more link; `*count += 1`.
#[hook(addr = 0x450FB0, conv = cdecl, name = "FUN_00450fb0", expect = "83 EC 24 8B 4C 24 38")]
pub fn add_interior_link_record(area_a: i32, node_a: u32, area_b: i32, node_b: i32, slot: i32, count: GPtr<i32>) {
    let tp = the_paths();
    let ia = 0x40 + slot;
    let link = area_field::<GPtr<u32>>(tp, NODE_LINKS, ia).read().wrapping_index(count.read()).cast::<u16>();
    link.wrapping_index(1).write(node_b as u16);
    link.write(area_b as u16);
    let na = i32::from(node_a as u8 as i8);
    let b = nodes(tp, area_b).byte_add(node_b as isize * 0x1C);
    let a = nodes(tp, area_a).byte_add(na as isize * 0x1C);
    let c255 = x(255.0);
    let first = {
        let d = interior_link_distance(a, b);
        if crate::common::jne_mask(d, c255, 0x41) { interior_link_distance(a, b) } else { c255 }
    };
    let len: u8 = if (ftol32(first) as u8) < 1 {
        1
    } else {
        let d = interior_link_distance(a, b);
        let v = if crate::common::jne_mask(d, c255, 0x41) {
            let (mut t1, mut t2) = ([0f32; 3], [0f32; 3]);
            let pb = get_position_call(b, GPtr::from_raw(t1.as_mut_ptr()));
            let pa = get_position_call(a, GPtr::from_raw(t2.as_mut_ptr()));
            let dx = x(pa.read()).fsub_m(pb.read());
            let dy = x(pa.wrapping_index(1).read()).fsub_m(pb.wrapping_index(1).read());
            let dz = x(pa.wrapping_index(2).read()).fsub_m(pb.wrapping_index(2).read());
            dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt()
        } else {
            c255
        };
        ftol32(v) as u8
    };
    let i = count.read();
    area_field::<GPtr<u8>>(tp, LINK_LENGTHS, ia).read().wrapping_index(i).write(len);
    area_field::<GPtr<u8>>(tp, INTERSECTIONS, ia).read().wrapping_index(count.read()).write(0);
    let f = at::<u8>(nodes(tp, ia), na as isize * 0x1C + 0x18);
    let v = f.read();
    f.write((v & 0xF0) | (v.wrapping_add(1) & 0xF));
    count.write(count.read().wrapping_add(1));
}

/// `CPathFind::FindNodePairClosestToCoors(CVector pos, uint8 type, CNodeAddress* out1, CNodeAddress* out2,
/// float* outHeading, float minDist, float maxDist, bool lowTraffic, bool unused, bool boats)` (0x44FEE0,
/// `ret 0x30`). Over the type's nodes of all 72 areas (another type: an uninitialised start local in the
/// original, the empty range here) with the lowTraffic / boat filters, a node whose `|dz|*3 + |dy| + |dx|` is
/// below the best (start 10000, kept in ST0) is paired with each filtered loaded link longer than `minDist`
/// (3D; every such link overwrites the pair). With `best < maxDist`: both addresses (the first one's node word
/// written as the node index) and `atan2(-v.x, v.y) * 57.295776` of the normalised `first - second` (2D);
/// otherwise both area words 0xFFFF and heading 0.
#[hook(addr = 0x44FEE0, conv = thiscall, name = "CPathFind::FindNodePairClosestToCoors", expect = "83 EC 60 D9 05 A4 9A 85 00", this = "CPathFind")]
#[allow(clippy::too_many_arguments)]
pub fn find_node_pair_closest_to_coors(
    this: GPtr<u8>,
    px: f32,
    py: f32,
    pz: f32,
    node_type: u32,
    out1: GPtr<u32>,
    out2: GPtr<u32>,
    out_heading: GPtr<f32>,
    min_dist: f32,
    max_dist: f32,
    low_traffic: u8,
    _unused: u8,
    boats: u32,
) {
    let mut best = x(10000.0);
    let (mut a1, mut a2): (u32, u32) = (0xFFFF, 0xFFFF);
    let (mut start, mut end) = (boats as i32, boats as i32);
    let bt = boats as u8;
    for area in 0..0x48 {
        if nodes(this, area).is_null() {
            continue;
        }
        match node_type as u8 {
            0 => {
                start = 0;
                end = area_field::<i32>(this, NUM_VEHICLE_NODES, area).read();
            }
            1 => {
                start = area_field::<i32>(this, NUM_VEHICLE_NODES, area).read();
                end = area_field::<i32>(this, NUM_NODES, area).read();
            }
            _ => {}
        }
        let mut i = start;
        while i < end {
            let n = nodes(this, area).byte_add(i as isize * 0x1C);
            i += 1;
            let f = rd::<u8>(n, 0x18);
            if (low_traffic != 0 && f & 0x20 != 0) || bt != f >> 7 {
                continue;
            }
            let nxf = coord(n, 8).to_f32();
            let ny = coord(n, 0xA);
            let nyf = ny.to_f32();
            let nz = coord(n, 0xC);
            let nzf = nz.to_f32();
            let d = fabs(nz.fsub_m(pz))
                .fmul_m(3.0)
                .fadd(fabs(ny.fsub_m(py)))
                .fadd(fabs(x(nxf).fsub_m(px)));
            let df = d.to_f32();
            if !lt(d, best) {
                continue;
            }
            let links = i32::from(f & 0xF);
            if links == 0 {
                continue;
            }
            for l in 0..links {
                let a = node_link(this, n, l);
                let lp = nodes(this, area_of(a));
                if lp.is_null() {
                    continue;
                }
                let ln = lp.byte_add(index_of(a) as isize * 0x1C);
                let lf = rd::<u8>(ln, 0x18);
                if (low_traffic != 0 && lf & 0x20 != 0) || bt != lf >> 7 {
                    continue;
                }
                let lzf = coord(ln, 0xC).to_f32();
                let dx = x(nxf).fsub(coord(ln, 8)).to_f32();
                let dy = x(nyf).fsub(coord(ln, 0xA));
                let dz = x(nzf).fsub_m(lzf);
                let len = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(x(dx).fmul_m(dx)).fsqrt();
                if crate::common::jne_mask(len, x(min_dist), 0x41) {
                    continue;
                }
                best = x(df);
                a1 = (area as u32 & 0xFFFF) | (((i - 1) as u32) << 16);
                a2 = a;
            }
        }
    }
    if !lt(best, x(max_dist)) {
        out1.cast::<u16>().write(0xFFFF);
        out2.cast::<u16>().write(0xFFFF);
        out_heading.write(0.0);
        return;
    }
    out1.write(a1);
    out2.write(a2);
    let na = node_of(this, a1);
    let nb = node_of(this, a2);
    let bx = coord(nb, 8);
    let by = coord(nb, 0xA);
    let bz = coord(nb, 0xC).to_f32();
    let ax = coord(na, 8);
    let ay = coord(na, 0xA).to_f32();
    let az = coord(na, 0xC).to_f32();
    let mut v = [ax.fsub(bx).to_f32(), x(ay).fsub(by).to_f32(), x(az).fsub_m(bz).to_f32()];
    v[2] = 0.0;
    let vp = GPtr::from_raw(v.as_mut_ptr());
    vector_normalise(vp);
    let h = x(vp.read()).fchs().fpatan(x(vp.wrapping_index(1).read())).fmul_m(f32::from_bits(0x4265_2EE0));
    out_heading.write(h.to_f32());
}

orig! {
    /// `FUN_00450FB0(...)` (0x450FB0, cdecl)
    pub fn add_interior_link_record_call(area_a: i32, node_a: u32, area_b: i32, node_b: i32, slot: i32, count: GPtr<i32>) = cdecl 0x450FB0 => add_interior_link_record;
}

/// `CPathFind::CompleteNewInterior(CNodeAddress* out)` (0x452270): builds interior area `0x40 + NewInteriorSlot` from
/// the `*Given` arrays: nodes (coordinates `ftol(c * 8)`, flood fill from the first exterior link's node or
/// `slot + 0x64`, fixed flag bits, don't-wander from `DontWanderGiven`), makes `ConnectsToGiven` symmetric (append
/// into the first negative entry when missing), allocates links / lengths / intersections for the connection count
/// plus 0xC0 spare, adds every connection through `FUN_00450FB0`, marks the 0xC0 spare link area words 0xFFFF, sets
/// the five counts and links the exterior nodes both ways (`AddDynamicLinkBetween2Nodes_For1Node`). `out` only
/// gets its area word set to 0xFFFF. Always clears `bInteriorBeingBuilt`.
#[hook(addr = 0x452270, conv = thiscall, name = "CPathFind::CompleteNewInterior", expect = "8B 44 24 04 83 EC 0C", this = "CPathFind")]
pub fn complete_new_interior(this: GPtr<u8>, out: GPtr<u16>) {
    if !out.is_null() {
        out.write(0xFFFF);
    }
    if NUM_NODES_GIVEN.get() != 0 {
        build_new_interior(this);
    }
    INTERIOR_BEING_BUILT.set(0);
}

fn build_new_interior(this: GPtr<u8>) {
    let slot = NEW_INTERIOR_SLOT.get();
    let mut flood = (slot as u8).wrapping_add(0x64);
    if NUM_LINKS_TO_EXTERIOR_NODES.get() > 0 {
        let e = EXTERIOR_NODE_LINKED_TO.ptr().cast::<u32>().read();
        flood = rd::<u8>(node_of(this, e), 0x17);
    }
    area_field::<i32>(this, INTERIOR_IDS, slot).write(INTERIOR_ID_BEING_BUILT.get());
    let ia = || 0x40 + NEW_INTERIOR_SLOT.get();
    let p = mem_malloc((NUM_NODES_GIVEN.get() as u32).wrapping_mul(0x1C), 0);
    area_field::<GPtr<u8>>(this, PATH_NODES, ia()).write(p);
    let coor = COOR_GIVEN.ptr().cast::<f32>();
    let conn = CONNECTS_TO_GIVEN.ptr().cast::<i8>();
    let mut i = 0;
    while i < NUM_NODES_GIVEN.get() {
        let node = || nodes(this, ia()).byte_add(i as isize * 0x1C);
        let n = node();
        wr::<u16>(n, 8, ftol32(x(coor.wrapping_index(128 + i).read()).fmul_m(8.0)) as u16);
        wr::<u16>(n, 0xA, ftol32(x(coor.wrapping_index(64 + i).read()).fmul_m(8.0)) as u16);
        wr::<u16>(n, 0xC, ftol32(x(coor.wrapping_index(i).read()).fmul_m(8.0)) as u16);
        let n = node();
        wr::<u16>(n, 0x14, i as u16);
        wr::<u16>(n, 0x12, (NEW_INTERIOR_SLOT.get() + 0x40) as u16);
        wr::<u8>(node(), 0x16, 0);
        wr::<u8>(node(), 0x17, flood);
        let upd = |off: isize, f: &dyn Fn(u8) -> u8| {
            let q = at::<u8>(node(), off);
            q.write(f(q.read()));
        };
        upd(0x18, &|b| b & 0xEF);
        upd(0x18, &|b| b | 0x20);
        upd(0x19, &|b| b | 1);
        upd(0x18, &|b| b & 0xBF);
        upd(0x18, &|b| b & 0x7F);
        upd(0x19, &|b| b & 0xFD);
        let dw = DONT_WANDER_GIVEN.ptr().cast::<u8>().wrapping_index(i).read();
        upd(0x19, &|b| b ^ (((dw << 2) ^ b) & 4));
        upd(0x19, &|b| b | 8);
        upd(0x19, &|b| b & 0xCF);
        upd(0x1A, &|b| b | 0xF);
        upd(0x1A, &|b| b & 0xF);
        wr::<i16>(node(), 0xE, 0x7FFE);
        i += 1;
    }
    let n_given = NUM_NODES_GIVEN.get();
    for i in 0..n_given {
        for j in 0..6 {
            let c = conn.wrapping_index(i * 6 + j).read();
            if c < 0 {
                continue;
            }
            let row = conn.wrapping_index(i32::from(c) * 6);
            if (0..6).any(|k| row.wrapping_index(k).read() as u8 == i as u8) {
                continue;
            }
            if let Some(k) = (0..6).find(|&k| row.wrapping_index(k).read() < 0) {
                row.wrapping_index(k).write(i as i8);
            }
        }
    }
    let mut total = 0;
    for i in 0..n_given {
        for j in 0..6 {
            if conn.wrapping_index(i * 6 + j).read() >= 0 {
                total += 1;
            }
        }
    }
    let size = (total as u32).wrapping_add(0xC0);
    let p = mem_malloc(size.wrapping_mul(4), 0);
    area_field::<GPtr<u8>>(this, NODE_LINKS, ia()).write(p);
    let p = mem_malloc(size, 0);
    area_field::<GPtr<u8>>(this, LINK_LENGTHS, ia()).write(p);
    let p = mem_malloc(size, 0);
    area_field::<GPtr<u8>>(this, INTERSECTIONS, ia()).write(p);
    let mut counter = [0i32];
    let cp = GPtr::from_raw(counter.as_mut_ptr());
    let mut i = 0;
    while i < NUM_NODES_GIVEN.get() {
        let n = nodes(this, ia()).byte_add(i as isize * 0x1C);
        wr::<u16>(n, 0x10, cp.read() as u16);
        let n = nodes(this, ia()).byte_add(i as isize * 0x1C);
        wr(n, 0x18, rd::<u8>(n, 0x18) & 0xF0);
        for j in 0..6 {
            let c = conn.wrapping_index(i * 6 + j).read();
            if c >= 0 {
                let s = NEW_INTERIOR_SLOT.get();
                add_interior_link_record_call(s + 0x40, i as u32, s + 0x40, i32::from(c), s, cp);
            }
        }
        i += 1;
    }
    for k in total..total + 0xC0 {
        area_field::<GPtr<u32>>(this, NODE_LINKS, ia()).read().wrapping_index(k).cast::<u16>().write(0xFFFF);
    }
    area_field::<i32>(this, NUM_NODES, ia()).write(NUM_NODES_GIVEN.get());
    area_field::<i32>(this, NUM_VEHICLE_NODES, ia()).write(0);
    area_field::<i32>(this, NUM_PED_NODES, ia()).write(NUM_NODES_GIVEN.get());
    area_field::<i32>(this, 0x1304, ia()).write(0);
    area_field::<i32>(this, 0x1424, ia()).write(total);
    let mut s = 0;
    while s < NUM_LINKS_TO_EXTERIOR_NODES.get() {
        let node = INTERIOR_NODE_LINKED_TO_EXTERIOR.ptr().cast::<u16>().wrapping_index(2 * s).read();
        let b = EXTERIOR_NODE_LINKED_TO.ptr().cast::<u32>().wrapping_index(s).read();
        let a = ((NEW_INTERIOR_SLOT.get() + 0x40) as u32 & 0xFFFF) | (u32::from(node) << 16);
        add_dynamic_link_between_2_nodes_for_1_node_call(this, a, b);
        add_dynamic_link_between_2_nodes_for_1_node_call(this, b, a);
        s += 1;
    }
}

/// `CPathFind::FindNodeCoorsForScript(CNodeAddress a, CNodeAddress b, float* heading, bool* found)` (0x450780,
/// hidden `CVector*` return, `ret 0x14`): (0, 0, 0) and `*found = 0` when either address is invalid or unloaded;
/// else `*found = 1`, `*heading = atan2(-(B - A).x, (B - A).y) * 57.295776`, and node A's position, moved for a node
/// with a width by `width * 0.0625 + 2.7` along `(v.y, -v.x, 0)` where `v` is the `CVector::Normalise`d
/// `(B.x - A.x, B.y - A.y, A.z)` (the z is the stale stack slot holding A's z from the heading computation, as the
/// bytes).
#[hook(addr = 0x450780, conv = thiscall, name = "CPathFind::FindNodeCoorsForScript-TwoNodes", expect = "8B 44 24 08 83 EC 30", this = "CPathFind")]
pub fn find_node_coors_for_script_two_nodes(
    this: GPtr<u8>,
    out: GPtr<f32>,
    a: u32,
    b: u32,
    heading: GPtr<f32>,
    found: GPtr<u8>,
) -> GPtr<f32> {
    if a as u16 == 0xFFFF
        || b as u16 == 0xFFFF
        || nodes(this, area_of(a)).is_null()
        || nodes(this, area_of(b)).is_null()
    {
        if !found.is_null() {
            found.write(0);
        }
        out.write(0.0);
        out.wrapping_index(1).write(0.0);
        out.wrapping_index(2).write(0.0);
        return out;
    }
    if !found.is_null() {
        found.write(1);
    }
    let na = node_of(this, a);
    let nb = node_of(this, b);
    let pos = |n: GPtr<u8>| {
        let mut t = [0f32; 3];
        get_position_call(n, GPtr::from_raw(t.as_mut_ptr()));
        t
    };
    let pb = pos(nb);
    let pa = pos(na);
    let stale_z = pa[2];
    let dy = x(pb[1]).fsub_m(pa[1]).to_f32();
    let pbx = pos(nb)[0];
    let pax = pos(na)[0];
    let h = x(pbx).fsub_m(pax).fchs().fpatan(x(dy)).fmul_m(f32::from_bits(0x4265_2EE0));
    heading.write(h.to_f32());
    let width = rd::<u8>(na, 0x16);
    if width == 0 {
        get_position_call(na, out);
        return out;
    }
    let vx = x(pos(nb)[0]).fsub_m(pos(na)[0]).to_f32();
    let vy = x(pos(nb)[1]).fsub_m(pos(na)[1]).to_f32();
    let mut v = [vx, vy, stale_z];
    let vp = GPtr::from_raw(v.as_mut_ptr());
    vector_normalise(vp);
    let mut side = [vp.wrapping_index(1).read(), x(vp.read()).fchs().to_f32(), 0.0];
    let f = xi(i32::from(width)).fmul_m(0.0625).fadd_m(f32::from_bits(0x402C_CCCD)).to_f32();
    let mut scaled = [0f32; 3];
    let s = vector_mul(GPtr::from_raw(scaled.as_mut_ptr()), GPtr::from_raw(side.as_mut_ptr()), f);
    let mut t = [0f32; 3];
    let p = get_position_call(na, GPtr::from_raw(t.as_mut_ptr()));
    vector_add(out, p, s);
    out
}

orig! {
    /// `CTimer::Suspend()` (0x5619D0)
    pub fn timer_suspend() = cdecl 0x5619D0 => sa_core::timer::suspend;
    /// `CTimer::Resume()` (0x561A00)
    pub fn timer_resume() = cdecl 0x561A00 => sa_core::timer::resume;
    /// CRT `sprintf(buf, fmt, int)` (0x821BB5)
    pub fn sprintf_i(buf: GPtr<u8>, fmt: GPtr<u8>, n: i32) -> i32 = cdecl 0x821BB5 => unported;
    /// `RwStreamOpen(RwStreamType, RwStreamAccessType, const void*)` (0x7ECEF0)
    pub fn rw_stream_open(ty: i32, access: i32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7ECEF0 => unported;
    /// `RwStreamClose(RwStream*, void*)` (0x7ECE20)
    pub fn rw_stream_close(s: GPtr<u8>, data: u32) -> i32 = cdecl 0x7ECE20 => unported;
    /// `CPathFind::LoadPathFindData(RwStream*, int32)` (0x4529F0)
    pub fn load_path_find_data_from_stream_call(this: GPtr<u8>, s: GPtr<u8>, area: i32) = thiscall 0x4529F0 => load_path_find_data_from_stream;
}

/// `CPathFind::LoadPathFindData(int32 area)` (0x452F40): with the timer suspended, formats the file name
/// (`[0x859AB4]` into the buffer 0xB71670), opens it as a file stream for reading, loads it and closes it.
#[hook(addr = 0x452F40, conv = thiscall, name = "CPathFind::LoadPathFindData-Area", expect = "53 56 57 8B F9", this = "CPathFind")]
pub fn load_path_find_data_area(this: GPtr<u8>, area: i32) {
    timer_suspend();
    let buf = g::<u8>(0xB71670);
    sprintf_i(buf, g(0x859AB4), area);
    let s = rw_stream_open(2, 1, buf);
    load_path_find_data_from_stream_call(this, s, area);
    rw_stream_close(s, 0);
    timer_resume();
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GET_POSITION,
    HOOK_NODE_ADDRESS_EQ,
    HOOK_NODE_ADDRESS_NE,
    HOOK_NODE_ADDRESS_LT,
    HOOK_COMPRESSED_VECTOR2D_UNCOMPRESS,
    HOOK_ONE_WAY_LANE_OFFSET,
    HOOK_IS_AREA_NODES_AVAILABLE,
    HOOK_GET_PATH_NODE,
    HOOK_INIT,
    HOOK_UNLOAD_PATH_FIND_DATA,
    HOOK_REMOVE_NODE_FROM_LIST,
    HOOK_ADD_NODE_TO_LIST,
    HOOK_TIDY_UP_NODE_SWITCHES_AFTER_MISSION,
    HOOK_THIS_NODE_HAS_TO_BE_SWITCHED_OFF,
    HOOK_FIND_REGION_FOR_COORS,
    HOOK_FIND_X_REGION_FOR_COORS,
    HOOK_FIND_Y_REGION_FOR_COORS,
    HOOK_FIND_X_COORS_FOR_REGION,
    HOOK_FIND_Y_COORS_FOR_REGION,
    HOOK_FIND_START_POINT_OF_REGION,
    HOOK_MARK_REGIONS_FOR_COORS,
    HOOK_SET_PATHS_NEEDED_AT_POSITION,
    HOOK_RELEASE_REQUESTED_NODES,
    HOOK_START_NEW_INTERIOR,
    HOOK_ADD_INTERIOR_LINK,
    HOOK_ADD_INTERIOR_LINK_TO_EXTERNAL_NODE,
    HOOK_RE_INIT,
    HOOK_MAKE_REQUEST_FOR_NODES_TO_BE_LOADED,
    HOOK_HAVE_REQUESTED_NODES_BEEN_LOADED,
    HOOK_SWITCH_PED_ROADS_OFF_IN_AREA,
    HOOK_SAVE,
    HOOK_LOAD,
    HOOK_INIT_G_977AF0,
    HOOK_INIT_G_977BE8,
    HOOK_DTOR_G_977BE8,
    HOOK_CALC_ROAD_DENSITY,
    HOOK_CALC_DIST_TO_ANY_CONNECTING_LINKS,
    HOOK_FIND_NODE_CLOSEST_IN_REGION,
    HOOK_FIND_NODE_CLOSEST_TO_COORS,
    HOOK_RECORD_NODES_CLOSEST_TO_COORS,
    HOOK_COUNT_NEIGHBOURS_TO_BE_SWITCHED_OFF,
    HOOK_IS_WATER_NODE_NEARBY,
    HOOK_SWITCH_OFF_NODE_AND_NEIGHBOURS,
    HOOK_ARE_NODES_LOADED_FOR_AREA,
    HOOK_UPDATE_STREAMING,
    HOOK_SWITCH_ROADS_OFF_IN_AREA_FOR_ONE_REGION,
    HOOK_SWITCH_ROADS_OFF_IN_AREA,
    HOOK_LOAD_PATH_FIND_DATA_FROM_STREAM,
    HOOK_DO_PATH_SEARCH,
    HOOK_COMPUTE_ROUTE,
    HOOK_TEST_COORS_CLOSENESS,
    HOOK_FIND_LINK_BETWEEN_NODES,
    HOOK_RETURN_INTERIOR_NODE_INDEX,
    HOOK_ADD_DYNAMIC_LINK_BETWEEN_2_NODES,
    HOOK_FIND_NEAREST_EXTERIOR_NODE_TO_INTERIOR_NODE,
    HOOK_SHUTDOWN,
    HOOK_MARK_ROAD_NODE_AS_DONT_WANDER,
    HOOK_THESE_2_NODES_ARE_ADJACENT,
    HOOK_FIND_NODE_CLOSEST_TO_COORS_FAVOUR_DIRECTION,
    HOOK_FIND_NTH_NODE_CLOSEST_TO_COORS,
    HOOK_FIND_NODE_ORIENTATION_FOR_CAR_PLACEMENT,
    HOOK_FIND_PARKING_NODE_IN_AREA,
    HOOK_UN_MARK_ALL_ROAD_NODES_AS_DONT_WANDER,
    HOOK_LOAD_SCENE_FOR_PATH_NODES,
    HOOK_TAKE_WIDTH_INTO_ACCOUNT_FOR_WANDERING,
    HOOK_ADD_WANDER_OFFSET_BETWEEN_NODES,
    HOOK_TEST_FOR_PED_TRAFFIC_LIGHT,
    HOOK_TEST_CROSSES_ROAD,
    HOOK_SET_LINKS_BRIDGE_LIGHTS,
    HOOK_THIS_NODE_WILL_LEAD_INTO_A_DEAD_END,
    HOOK_ADD_NODE_TO_NEW_INTERIOR,
    HOOK_FIND_NEXT_NODE_WANDERING,
    HOOK_GENERATE_PED_CREATION_COORS,
    HOOK_GENERATE_PED_CREATION_COORS_INTERIOR,
    HOOK_FIND_NODE_COORS_FOR_SCRIPT,
    HOOK_ADD_DYNAMIC_LINK_BETWEEN_2_NODES_FOR_1_NODE,
    HOOK_REMOVE_INTERIOR,
    HOOK_FIND_NODES_WITHIN_RADIUS,
    HOOK_ADD_INTERIOR_LINK_RECORD,
    HOOK_FIND_NODE_PAIR_CLOSEST_TO_COORS,
    HOOK_COMPLETE_NEW_INTERIOR,
    HOOK_FIND_NODE_COORS_FOR_SCRIPT_TWO_NODES,
    HOOK_LOAD_PATH_FIND_DATA_AREA,
];

/// `dist_to_line_sqr_st0` with its ST0 result as `X87` (orig! returns stay register-sized).
pub fn dist_to_line_sqr(a: GPtr<f32>, b: GPtr<f32>, p: GPtr<f32>) -> X87 {
    X87::from_f64_exact(dist_to_line_sqr_st0(a, b, p))
}

/// `calc_dist_to_any_connecting_links_call_st0` with its ST0 result as `X87` (orig! returns stay register-sized).
pub fn calc_dist_to_any_connecting_links_call(this: GPtr<u8>, node: GPtr<u8>, px: f32, py: f32, pz: f32) -> X87 {
    X87::from_f64_exact(calc_dist_to_any_connecting_links_call_st0(this, node, px, py, pz))
}

/// `find_ground_z_for_3d_coord_st0` with its ST0 result as `X87` (orig! returns stay register-sized).
pub fn find_ground_z_for_3d_coord(px: f32, py: f32, pz: f32, found: GPtr<u8>, ent: GPtr<GPtr<u8>>) -> X87 {
    X87::from_f64_exact(find_ground_z_for_3d_coord_st0(px, py, pz, found, ent))
}
/// Phase B target of `calc_dist_to_any_connecting_links_call_st0`: the port's ST0 result as f64.
pub fn calc_dist_to_any_connecting_links_f64(this: GPtr<u8>, node: GPtr<u8>, px: f32, py: f32, pz: f32) -> f64 {
    calc_dist_to_any_connecting_links(this, node, px, py, pz).to_f64()
}
