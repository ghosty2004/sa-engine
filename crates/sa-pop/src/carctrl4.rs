//! `CCarCtrl` / `CCarAI` node picking and the car AI update (PORT-POP-3). Ported from the 1.0 US bytes;
//! gta-reversed and `docs/re/specs/POP/*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, model_info, rand_at, rd, wr, x, xi};

/// Globals owned by this module: none.
pub const GLOBALS: &[GlobalDesc] = &[];

orig! {
    /// `CPathNode::GetPosition(CVector*)` (0x420A10, thiscall, returns `out`)
    fn get_position(node: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x420A10 => crate::pathfind::get_position;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): `out = a - b`
    fn vector_sub(out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>) -> GPtr<f32> = cdecl 0x40FE60 => unported;
    /// `CrossProduct(CVector* out, const CVector* a, const CVector* b)` (0x59C730)
    fn cross_product(out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>) -> GPtr<f32> = cdecl 0x59C730 => unported;
    /// `DotProduct(const CVector* a, const CVector* b)` (0x40FDB0, ST0)
    fn dot_product(a: GPtr<f32>, b: GPtr<f32>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `CPlaceable::GetRightDirection(CVector* out)` (0x41CC70, thiscall, returns `out`)
    fn get_right_direction(e: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x41CC70 => unported;
    /// `CVehicle::IsLawEnforcementVehicle()` (0x6D2370)
    fn is_law_enforcement_vehicle(v: GPtr<u8>) -> u8 = thiscall 0x6D2370 => unported;
    /// `CCarCtrl::IsAnyoneParking()` (0x42C250)
    fn is_anyone_parking() -> u8 = cdecl 0x42C250 => crate::carctrl2::is_anyone_parking;
    /// `CWorld::FindObjectsKindaColliding(const CVector&, float, bool, int16*, int16, CEntity**, bool, bool, bool,
    /// bool, bool)` (0x568B80; sets `*count = 0` first)
    /// `CCarCtrl::ThisVehicleShouldTryNotToTurn(CVehicle*)` (0x421FE0)
    fn this_vehicle_should_try_not_to_turn(v: GPtr<u8>) -> u8 = cdecl 0x421FE0 => crate::carctrl::this_vehicle_should_try_not_to_turn;
    /// `CCarCtrl::InitSequence(int32)` (0x421740)
    fn init_sequence(n: i32) = cdecl 0x421740 => crate::carctrl::init_sequence;
    /// `CCarCtrl::FindPathDirection(CNodeAddress, CNodeAddress, CNodeAddress, bool*)` (0x422090)
    fn find_path_direction_call(a: u32, b: u32, c: u32, sharp: GPtr<u8>) -> u8 = cdecl 0x422090 => find_path_direction;
    /// `CCarCtrl::IsThisAnAppropriateNode(CVehicle*, CNodeAddress, CNodeAddress, CNodeAddress, bool)` (0x42DAB0;
    /// the caller pushes a sixth dword the callee never reads)
    fn is_this_an_appropriate_node_call(v: GPtr<u8>, n1: u32, n2: u32, n3: u32, wrong: u32, other: u32) -> u8 = cdecl 0x42DAB0 => unported;
    /// `CAutoPilot::ModifySpeed(float)` (0x41B980, thiscall on `veh + 0x390`)
    fn modify_speed(ap: GPtr<u8>, s: f32) = thiscall 0x41B980 => unported;
    /// `CCarPathLink::OneWayLaneOffset()` (0x44DB00, thiscall, ST0)
    fn one_way_lane_offset(l: GPtr<u8>) -> f64 = thiscall 0x44DB00 => unported;
    /// `CCurves::CalcSpeedScaleFactor(CVector*, CVector*, float, float, float, float)` (0x43C710, ST0)
    fn calc_speed_scale_factor(p1: GPtr<f32>, p2: GPtr<f32>, d1x: f32, d1y: f32, d2x: f32, d2y: f32) -> f64 = cdecl 0x43C710 => unported;
    fn find_objects_kinda_colliding(p: GPtr<f32>, r: f32, b2d: u32, count: GPtr<i16>, max: u32, out: GPtr<u8>, buildings: u32, vehicles: u32, peds: u32, objects: u32, dummies: u32) = cdecl 0x568B80 => unported;
}

/// `ThePaths.m_pPathNodes[area]` (0x96F854).
fn path_nodes(area: u32) -> GPtr<u8> {
    g::<GPtr<u8>>((0x96F854 + area * 4) as usize).read()
}

/// `CNodeAddress` packed in a dword: area (low word), node index (high word).
fn area_of(a: u32) -> u32 {
    a & 0xFFFF
}

fn index_of(a: u32) -> u32 {
    a >> 16
}

/// `CPathNode::GetPosition` into a local vector, through the hook table.
fn pos(node: GPtr<u8>) -> [f32; 3] {
    let mut v = [0f32; 3];
    get_position(node, GPtr::from_raw(v.as_mut_ptr()));
    v
}

/// `fcom m; fnstsw ax; test ah, 0x44; jp` taken: not equal (unordered counts as equal).
fn ne(a: X87, b: X87) -> bool {
    a.fcom(b).test_parity(0x44)
}

/// `CCarCtrl::FindPathDirection(CNodeAddress a, CNodeAddress b, CNodeAddress c, bool* sharpTurn)` (0x422090):
/// 0 when an address is empty or its area is not loaded or a leg has zero length, 1 when the normalised
/// legs' dot product is above 0.4, else 4 (cross product > 0) or 2, with `*sharpTurn = 1` when the dot
/// product is below -0.3. `*sharpTurn` is cleared first. The x87 sequence keeps `dy1 / len1`, both normalised
/// second-leg components and the dot product unrounded; `dx1n` and the cross product go through f32 slots.
/// The original uses its own argument slots as scratch (nothing reads them after the call).
#[hook(addr = 0x422090, conv = cdecl, name = "CCarCtrl::FindPathDirection", expect = "8B 44 24 04 83 EC 44")]
pub fn find_path_direction(a: u32, b: u32, c: u32, sharp: GPtr<u8>) -> u8 {
    sharp.write(0);
    if area_of(a) == 0xFFFF || area_of(b) == 0xFFFF || area_of(c) == 0xFFFF {
        return 0;
    }
    let (pa, pb, pc) = (
        path_nodes(area_of(a)),
        path_nodes(area_of(b)),
        path_nodes(area_of(c)),
    );
    if pa.is_null() || pb.is_null() || pc.is_null() {
        return 0;
    }
    let na = pa.byte_add(index_of(a) as isize * 0x1C);
    let nb = pb.byte_add(index_of(b) as isize * 0x1C);
    let nc = pc.byte_add(index_of(c) as isize * 0x1C);
    // The eight GetPosition calls in the original order: B, A (x), B, A (y), C, B (x), C, B (y).
    let bx = pos(nb)[0];
    let dx1 = x(bx).fsub_m(pos(na)[0]).to_f32();
    let by = pos(nb)[1];
    let dy1 = x(by).fsub_m(pos(na)[1]).to_f32();
    let cx = pos(nc)[0];
    let dx2 = x(cx).fsub_m(pos(nb)[0]).to_f32();
    let cy = pos(nc)[1];
    let dy2 = x(cy).fsub_m(pos(nb)[1]).to_f32();
    let len1 = x(dy1).fmul_m(dy1).fadd(x(dx1).fmul_m(dx1)).fsqrt();
    if !ne(len1, x(0.0)) {
        return 0;
    }
    let i1 = len1.fdivr_m(1.0);
    let dx1n = x(dx1).fmul(i1).to_f32();
    let dy1n = i1.fmul_m(dy1);
    let len2 = x(dy2).fmul_m(dy2).fadd(x(dx2).fmul_m(dx2)).fsqrt();
    if !ne(len2, x(0.0)) {
        return 0;
    }
    let i2 = len2.fdivr_m(1.0);
    let dx2n = x(dx2).fmul(i2);
    let dy2n = i2.fmul_m(dy2);
    let cross = x(dx1n).fmul(dy2n).fsub(dx2n.fmul(dy1n)).to_f32();
    let dot = dy1n.fmul(dy2n).fadd(dx2n.fmul_m(dx1n));
    // fcom 0.4; test ah, 0x41; jne: dot <= 0.4 or unordered goes on
    if !dot.fcom(x(f32::from_bits(0x3ECC_CCCD))).test_any(0x41) {
        return 1;
    }
    // fcomp -0.3; test ah, 5; jp not taken: dot < -0.3
    if !dot.fcom(x(f32::from_bits(0xBE99_999A))).test_parity(0x05) {
        sharp.write(1);
    }
    if x(cross).fcom(x(0.0)).test_any(0x41) {
        2
    } else {
        4
    }
}

/// `CPlaceable::GetPosition`: the matrix translation (+0x14 → +0x30) or the simple placement (+4).
fn entity_pos(e: GPtr<u8>) -> GPtr<f32> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4).cast()
    } else {
        m.byte_add(0x30).cast()
    }
}

fn lp(v: &mut [f32; 3]) -> GPtr<f32> {
    GPtr::from_raw(v.as_mut_ptr())
}

/// The vehicle class (`CVehicleModelInfo` +0x4D) of `veh`'s model.
fn vehicle_class(veh: GPtr<u8>) -> u8 {
    rd::<u8>(model_info(i32::from(rd::<i16>(veh, 0x22))), 0x4D)
}

/// The colmodel bounding box maximum component `off` (+0xC x, +0x10 y, +0x14 z) of `veh`'s model.
fn bbox_max(veh: GPtr<u8>, off: isize) -> X87 {
    x(rd::<f32>(
        rd::<GPtr<u8>>(model_info(i32::from(rd::<i16>(veh, 0x22))), 0x14),
        off,
    ))
}

/// The shared tail of the parking cases: not for law enforcers, class 5 (big), criminal drivers, while
/// anyone is parking, or when a vehicle is within 5 m (2D) of the candidate node.
fn parking_tail(veh: GPtr<u8>, n3: GPtr<u8>) -> u8 {
    if is_law_enforcement_vehicle(veh) != 0 || vehicle_class(veh) == 5 {
        return 0;
    }
    let driver = rd::<GPtr<u8>>(veh, 0x460);
    if !driver.is_null() && rd::<i32>(driver, 0x598) == 0x14 {
        return 0;
    }
    if is_anyone_parking() != 0 {
        return 0;
    }
    let mut count = [0i16];
    let mut p = [0f32; 3];
    get_position(n3, lp(&mut p));
    find_objects_kinda_colliding(
        lp(&mut p),
        5.0,
        1,
        GPtr::from_raw(count.as_mut_ptr()),
        2,
        GPtr::NULL,
        0,
        1,
        0,
        0,
        0,
    );
    u8::from(count[0] == 0)
}

/// `CCarCtrl::IsThisAnAppropriateNode(CVehicle*, CNodeAddress prev, CNodeAddress cur, CNodeAddress next, bool
/// wrongWay)` (0x42DAB0): 0 when `next`'s area is not loaded or `next == prev`, when the water flag changes
/// between `cur` and `next` (except the Vortex, 539); then by `next`'s behaviour type (jump table 0x42DE54 /
/// byte table 0x42DE68): 1 / 2 parking nodes (not for mission vehicles or mission drivers; type 1 also not
/// for class 5; without `wrongWay` a side test: type 1 the z of `(next - cur) x (next - veh)` must not be
/// negative, type 2 `right · (next - veh)` must not be negative; then [`parking_tail`]); 5 / 10 need a bbox
/// max z >= 2; 8 / 9 need max z <= 1.5, x <= 2, y <= 4 (types 5, 8, 9, 10 reject `wrongWay`); every other
/// type and the passing size tests: mission cruisers keep off don't-wander nodes, nobody enters a dead end
/// or switched-off nodes from outside, result `!wrongWay`. `cur`'s area is not NULL-checked (as the bytes).
#[hook(addr = 0x42DAB0, conv = cdecl, name = "CCarCtrl::IsThisAnAppropriateNode", expect = "8B 4C 24 10 0F B7 C1")]
pub fn is_this_an_appropriate_node(veh: GPtr<u8>, n1: u32, n2: u32, n3: u32, wrong_way: u8) -> u8 {
    let p3 = path_nodes(area_of(n3));
    if p3.is_null() || n1 == n3 {
        return 0;
    }
    let node2 = path_nodes(area_of(n2)).byte_add(index_of(n2) as isize * 0x1C);
    let node3 = p3.byte_add(index_of(n3) as isize * 0x1C);
    let f2 = rd::<u8>(node2, 0x18);
    let f3 = rd::<u8>(node3, 0x18);
    if (f2 ^ f3) & 0x80 != 0 && rd::<i16>(veh, 0x22) != 0x21B {
        return 0;
    }
    let t = u32::from(rd::<u8>(node3, 0x1A) >> 4);
    let case = match t.wrapping_sub(1) {
        0 => 0,
        1 => 1,
        4 | 9 => 2,
        7 | 8 => 3,
        _ => 4,
    };
    let mission_owned = |veh: GPtr<u8>| {
        if rd::<u8>(veh, 0x4A4) == 2 {
            return true;
        }
        let d = rd::<GPtr<u8>>(veh, 0x460);
        !d.is_null() && rd::<u8>(d, 0x484) == 2
    };
    match case {
        0 => {
            if mission_owned(veh) || vehicle_class(veh) == 5 {
                return 0;
            }
            if wrong_way == 0 {
                let vp = entity_pos(veh);
                let (mut t3, mut b, mut t2, mut t3b, mut a, mut c) =
                    ([0f32; 3], [0f32; 3], [0f32; 3], [0f32; 3], [0f32; 3], [0f32; 3]);
                let p = get_position(node3, lp(&mut t3));
                vector_sub(lp(&mut b), p, vp);
                let q2 = get_position(node2, lp(&mut t2));
                let q3 = get_position(node3, lp(&mut t3b));
                vector_sub(lp(&mut a), q3, q2);
                cross_product(lp(&mut c), lp(&mut a), lp(&mut b));
                // fcomp 0; test ah, 5; jnp: z < 0
                if !x(c[2]).fcom(x(0.0)).test_parity(0x05) {
                    return 0;
                }
            }
            parking_tail(veh, node3)
        }
        1 => {
            if mission_owned(veh) {
                return 0;
            }
            if wrong_way == 0 {
                let vp = entity_pos(veh);
                let (mut t3, mut d, mut r) = ([0f32; 3], [0f32; 3], [0f32; 3]);
                let p = get_position(node3, lp(&mut t3));
                vector_sub(lp(&mut d), p, vp);
                let rp = get_right_direction(veh, lp(&mut r));
                if !X87::from_f64_exact(dot_product(rp, lp(&mut d)))
                    .fcom(x(0.0))
                    .test_parity(0x05)
                {
                    return 0;
                }
            }
            parking_tail(veh, node3)
        }
        2 | 3 => {
            if wrong_way != 0 {
                return 0;
            }
            if case == 2 {
                // fcomp 2.0; test ah, 5; jnp: max z < 2
                if !bbox_max(veh, 0x14).fcom(x(2.0)).test_parity(0x05) {
                    return 0;
                }
            } else {
                // each `test ah, 0x41; je`: greater fails
                if !bbox_max(veh, 0x14).fcom(x(1.5)).test_any(0x41)
                    || !bbox_max(veh, 0xC).fcom(x(2.0)).test_any(0x41)
                {
                    return 0;
                }
                if !bbox_max(veh, 0x10).fcom(x(4.0)).test_any(0x41) {
                    return 0;
                }
            }
            default_tail(veh, node2, node3, f2, f3, wrong_way)
        }
        _ => default_tail(veh, node2, node3, f2, f3, wrong_way),
    }
}

fn default_tail(veh: GPtr<u8>, node2: GPtr<u8>, node3: GPtr<u8>, f2: u8, f3: u8, wrong_way: u8) -> u8 {
    if rd::<u8>(veh, 0x4A4) == 2
        && rd::<u8>(veh, 0x3BA) == 1
        && rd::<u8>(node3, 0x19) & 4 != 0
        && rd::<u8>(node2, 0x19) & 4 == 0
    {
        return 0;
    }
    if f3 & 0x10 != 0 && f2 & 0x10 == 0 {
        return 0;
    }
    if f3 & 0x20 != 0 && f2 & 0x20 == 0 {
        return 0;
    }
    u8::from(wrong_way == 0)
}

/// `ThePaths.m_pNaviNodes[l >> 10] + (l & 0x3FF) * 0xE`: the `CCarPathLink` of a link address.
fn navi_link(l: u16) -> GPtr<u8> {
    g::<GPtr<u8>>((0x96F974 + u32::from(l >> 10) * 4) as usize)
        .read()
        .byte_add((l & 0x3FF) as isize * 0xE)
}

/// `ThePaths.m_pPathNodes[l >> 10]` is loaded (the area test of a link address).
fn link_area_loaded(l: u16) -> bool {
    !path_nodes(u32::from(l >> 10)).is_null()
}

/// `ThePaths.m_pNodeLinks[area][i]` (0x96FA94, CNodeAddress dwords).
fn node_link_at(area: u32, i: i32) -> u32 {
    g::<GPtr<u32>>((0x96FA94 + area * 4) as usize)
        .read()
        .wrapping_index(i)
        .read()
}

/// `ThePaths.m_pNaviLinks[area][i]` (0x96FDF4, link address words).
fn navi_link_at(area: u32, i: i32) -> u16 {
    g::<GPtr<u16>>((0x96FDF4 + area * 4) as usize)
        .read()
        .wrapping_index(i)
        .read()
}

fn path_node(a: u32) -> GPtr<u8> {
    path_nodes(area_of(a)).byte_add(index_of(a) as isize * 0x1C)
}

/// `CCarPathLink` +4 / +6 (the node it is attached to) equal `a`.
fn attached_to(l: GPtr<u8>, a: u32) -> bool {
    rd::<u16>(l, 4) == (a & 0xFFFF) as u16 && rd::<u16>(l, 6) == (a >> 16) as u16
}

/// The sequence index of loop counter `i` (`CCarCtrl::FindSequenceElement` inlined, signed `idiv`).
fn seq_index(i: i32) -> i32 {
    let off = g::<i32>(0x969074).read();
    let n = g::<i32>(0x969078).read();
    if g::<u8>(0x969070).read() != 0 {
        off.wrapping_add(i).wrapping_rem(n)
    } else {
        n.wrapping_sub(i).wrapping_add(off).wrapping_rem(n)
    }
}

/// The vehicle status (+0x36 bits 3..7) is `STATUS_SIMPLE` (2).
fn status_simple(veh: GPtr<u8>) -> bool {
    rd::<u8>(veh, 0x36) & 0xF8 == 0x10
}

/// The `(wrongWay, otherNone)` lane flags of `link` seen from node `from`: attached → (B == 0, A == 0), else
/// (A == 0, B == 0), A = bits 0..2 and B = bits 3..5 of +0xB.
fn lane_flags(link: GPtr<u8>, from: u32) -> (bool, bool) {
    let b = rd::<u8>(link, 0xB);
    if attached_to(link, from) {
        (b & 0x38 == 0, b & 7 == 0)
    } else {
        (b & 7 == 0, b & 0x38 == 0)
    }
}

/// `CTimer::m_snTimeInMilliseconds`.
fn now() -> u32 {
    g::<u32>(0xB7CB84).read()
}

/// `fild i8; fmul 0.01f; fmul sign` of a link direction byte (+8 x, +9 y).
fn link_dir(l: GPtr<u8>, off: isize, sign: X87) -> X87 {
    xi(i32::from(rd::<i8>(l, off)))
        .fmul_m(f32::from_bits(0x3C23_D70A))
        .fmul(sign)
}

/// `fild i16; fmul 0.125f` of a node / link coordinate.
fn c8(p: GPtr<u8>, off: isize) -> X87 {
    xi(i32::from(rd::<i16>(p, off))).fmul_m(0.125)
}

/// Status → `STATUS_PHYSICS` (3) with the temp action cleared and the mission timer 2 s ahead (the inlined
/// `SwitchVehicleToRealPhysics` stores).
fn to_physics_status(veh: GPtr<u8>) {
    wr(veh, 0x36, (rd::<u8>(veh, 0x36) & 7) | 0x18);
    wr::<u8>(veh, 0x3BB, 0);
    wr(veh, 0x3AC, now().wrapping_add(2000));
}

/// `CCarCtrl::PickNextNodeRandomly(CVehicle*)` (0x42DE80): the vehicle's forced route seed (+0x45E) seeds
/// `srand`; the next node's links are tried in sequence order (`InitSequence(numLinks)`, called before the
/// `numLinks > 0` test as the bytes do): loop 1 takes an appropriate node whose turn direction is in the lane
/// mask (unless both the current link and the candidate's far side are one-way), loop 2 any node not
/// against one-way traffic that is not switched off (or we come from switched-off nodes), loop 3 any such
/// node; else a U-turn (status → physics). Every candidate is written to +0x394 as it is tried. Then the
/// parking / wait behaviour of the new nodes, the link shift, the lane change countdown (rand 0x42E969 /
/// 0x42E978 when the nodes are more than 16 m apart), the lane flags and, for `STATUS_SIMPLE`, the curve
/// duration from `CalcSpeedScaleFactor` (at least 10). The original reuses its argument slot as scratch
/// (turn mask, link word); nothing reads it after the call.
#[hook(addr = 0x42DE80, conv = cdecl, name = "CCarCtrl::PickNextNodeRandomly", expect = "83 EC 44 56 8B 74 24 4C")]
pub fn pick_next_node_randomly(veh: GPtr<u8>) {
    let seed = rd::<u16>(veh, 0x45E);
    if seed != 0 {
        sa_core::orig::at_site(0x42_DEA2, || sa_core::crt::srand(u32::from(seed)));
    }
    let old_cur = rd::<u32>(veh, 0x390);
    if path_nodes(area_of(old_cur)).is_null() {
        return;
    }
    let old_next = rd::<u32>(veh, 0x394);
    if path_nodes(area_of(old_next)).is_null() {
        return;
    }
    let lk = rd::<u16>(veh, 0x3A6);
    if !link_area_loaded(lk) {
        return;
    }
    let next_area = area_of(old_next);
    let num_links = i32::from(rd::<u8>(path_node(old_next), 0x18) & 0xF);
    let base = |i: i32| i32::from(rd::<i16>(path_node(old_next), 0x10)).wrapping_add(seq_index(i));

    let cl = navi_link(lk);
    let clb = rd::<u8>(cl, 0xB);
    let (n_lanes, cur_one_way) = if attached_to(cl, old_next) {
        (i32::from(clb & 7), clb & 0x38 == 0)
    } else {
        (i32::from((clb >> 3) & 7), clb & 7 == 0)
    };
    let lane = i32::from(rd::<i8>(veh, 0x3B8));
    let mut mask: u8 = if lane == 0 { 4 } else { 0 };
    if lane == n_lanes - 1 {
        mask |= 2;
    }
    if n_lanes < 3 || mask == 0 {
        mask |= 1;
    }
    wr(veh, 0x398, old_cur);
    wr(veh, 0x390, old_next);
    if this_vehicle_should_try_not_to_turn(veh) != 0 {
        mask = 1;
    }
    init_sequence(num_links);

    let mut new_link: Option<u16> = None;
    let mut sharp = [0u8];
    for i in 0..num_links {
        let k = base(i);
        let cand = node_link_at(next_area, k);
        wr(veh, 0x394, cand);
        if path_nodes(area_of(cand)).is_null() {
            continue;
        }
        let dir = find_path_direction_call(
            old_cur,
            rd::<u32>(veh, 0x390),
            cand,
            GPtr::from_raw(sharp.as_mut_ptr()),
        );
        let cand_link = navi_link_at(next_area, k);
        if !link_area_loaded(cand_link) {
            continue;
        }
        if status_simple(veh) && sharp[0] != 0 {
            continue;
        }
        let (wrong, other_none) = lane_flags(navi_link(cand_link), old_next);
        let ok = is_this_an_appropriate_node_call(
            veh,
            old_cur,
            old_next,
            cand,
            u32::from(wrong),
            u32::from(other_none),
        );
        if ok != 0 && mask & dir != 0 && (!cur_one_way || !other_none) {
            new_link = Some(cand_link);
            break;
        }
    }
    for strict in [true, false] {
        if new_link.is_some() {
            break;
        }
        for i in 0..num_links {
            let k = base(i);
            let cand = node_link_at(next_area, k);
            wr(veh, 0x394, cand);
            if path_nodes(area_of(cand)).is_null() {
                continue;
            }
            let cand_link = navi_link_at(next_area, k);
            if !link_area_loaded(cand_link) {
                continue;
            }
            let (wrong, _) = lane_flags(navi_link(cand_link), old_next);
            if cand == old_cur || wrong {
                continue;
            }
            if !strict
                || rd::<u8>(path_node(cand), 0x18) & 0x20 == 0
                || rd::<u8>(path_node(old_cur), 0x18) & 0x20 != 0
            {
                new_link = Some(cand_link);
                break;
            }
        }
    }
    let new_link = match new_link {
        Some(l) => l,
        None => {
            wr(veh, 0x394, old_cur);
            rd::<u16>(veh, 0x3A6)
        }
    };
    if rd::<u32>(veh, 0x394) == old_cur && rd::<u8>(veh, 0x36) & 0xF8 != 0x18 {
        to_physics_status(veh);
        wr(veh, 0x3B0, now());
        wr::<u8>(veh, 0xB8, 0);
    }
    let ap = veh.byte_add(0x390);
    match rd::<u8>(path_node(rd::<u32>(veh, 0x394)), 0x1A) >> 4 {
        t @ (1 | 2) => {
            to_physics_status(veh);
            wr::<u8>(veh, 0x3BA, if t == 1 { 0x2E } else { 0x2D });
            wr(veh, 0x3B0, now());
            wr::<u8>(veh, 0xB8, 0);
            wr::<u8>(veh, 0x3B9, 0);
        }
        10 => {
            wr::<u8>(veh, 0x3BB, 1);
            wr(veh, 0x3BC, now().wrapping_add(10000));
            if status_simple(veh) {
                modify_speed(ap, 0.0);
            }
        }
        _ => {}
    }
    if rd::<u8>(path_node(rd::<u32>(veh, 0x390)), 0x1A) & 0xF0 == 0x90 && rd::<u8>(veh, 0x4A4) != 2 {
        wr::<u8>(veh, 0x3BB, 1);
        wr(veh, 0x3BC, now().wrapping_add(4500));
        if status_simple(veh) {
            modify_speed(ap, 0.0);
        }
    }
    // Shift the links.
    wr(veh, 0x3A8, rd::<u16>(veh, 0x3A4));
    wr(veh, 0x3A4, rd::<u16>(veh, 0x3A6));
    wr(
        veh,
        0x39C,
        rd::<u32>(veh, 0x39C).wrapping_add(rd::<u32>(veh, 0x3A0)),
    );
    wr(veh, 0x3B4, rd::<u8>(veh, 0x3B5));
    wr(veh, 0x3B5, rd::<u8>(veh, 0x3B6));
    wr(veh, 0x3B7, rd::<u8>(veh, 0x3B8));
    wr(veh, 0x3A6, new_link);
    let nn = rd::<u32>(veh, 0x394);
    let (oa, na) = (old_next & 0xFFFF, nn & 0xFFFF);
    let backwards = oa < na || (oa == na && (old_next >> 16) < (nn >> 16));
    wr::<i8>(veh, 0x3B6, if backwards { -1 } else { 1 });
    let lanes = if !link_area_loaded(new_link) {
        1
    } else {
        let b = rd::<u8>(navi_link(new_link), 0xB);
        if backwards {
            i32::from((b >> 3) & 7)
        } else {
            i32::from(b & 7)
        }
    };
    // The direction floats of the current (+0x3A4) and the new link, stored f32 (m_pNaviNodes of the new
    // link's area is read without a NULL check, as the bytes).
    let cur = navi_link(rd::<u16>(veh, 0x3A4));
    let s5 = xi(i32::from(rd::<i8>(veh, 0x3B5)));
    let cdx = link_dir(cur, 8, s5).to_f32();
    let cdy = link_dir(cur, 9, s5).to_f32();
    let nl = navi_link(new_link);
    let s6 = xi(i32::from(rd::<i8>(veh, 0x3B6)));
    let ndx = link_dir(nl, 8, s6).to_f32();
    let ndy = link_dir(nl, 9, s6).to_f32();
    // The lane change (its `lanes < 0` guard never skips it).
    let a = path_node(rd::<u32>(veh, 0x394));
    let c = path_node(rd::<u32>(veh, 0x390));
    let dx = c8(a, 8).fsub(c8(c, 8));
    let dy = c8(a, 0xA).fsub(c8(c, 0xA));
    let d2 = dx.fmul(dx).fadd(dy.fmul(dy));
    // fcomp 256; test ah, 0x41; jne: only d2 > 256 goes on
    if !d2.fcom(x(256.0)).test_any(0x41) {
        let cd = rd::<u8>(veh, 0x3E0).wrapping_sub(1);
        wr(veh, 0x3E0, cd);
        if cd == 0 {
            wr(veh, 0x3E0, ((rand_at(0x42E969) as u8) & 3).wrapping_add(4));
            let r = rand_at(0x42E978);
            let l = rd::<u8>(veh, 0x3B8);
            wr(
                veh,
                0x3B8,
                if r < 0x3FFF {
                    l.wrapping_add(1)
                } else {
                    l.wrapping_sub(1)
                },
            );
        }
    }
    let mut l = i32::from(rd::<i8>(veh, 0x3B8));
    if l >= lanes - 1 {
        l = lanes - 1;
    }
    wr(veh, 0x3B8, l as u8);
    if (l as u8 as i8) <= 0 {
        wr::<u8>(veh, 0x3B8, 0);
    }
    let flags = rd::<u8>(veh, 0x3DB);
    if flags & 8 != 0 {
        wr::<u8>(veh, 0x3B8, 0);
    } else if flags & 0x10 != 0 {
        wr(veh, 0x3B8, if lanes - 1 < 0 { 0 } else { (lanes - 1) as u8 });
    }
    if !status_simple(veh) {
        return;
    }
    curve_timing(veh, cdx, cdy, ndx, ndy);
}

/// The curve duration (+0x3A0) of the node pickers: lateral offsets `(OneWayLaneOffset + lane) * 5.4` of the
/// current (+0x3A4, lane +0x3B7, stored f32) and next link (+0x3A6, lane +0x3B8, x87), the lane points
/// `link + offset * (dir.y, -dir.x)` with the stored f32 directions, `CalcSpeedScaleFactor` with the
/// directions recomputed from the link bytes, `_ftol(f * (1000 / speed))`, then at least 10 (the unclamped
/// value is stored first).
fn curve_timing(veh: GPtr<u8>, cdx: f32, cdy: f32, ndx: f32, ndy: f32) {
    let cur = navi_link(rd::<u16>(veh, 0x3A4));
    let o1 = X87::from_f64_exact(one_way_lane_offset(cur))
        .fadd(xi(i32::from(rd::<i8>(veh, 0x3B7))))
        .fmul_m(f32::from_bits(0x40AC_CCCD))
        .to_f32();
    let nl = navi_link(rd::<u16>(veh, 0x3A6));
    let o2 = X87::from_f64_exact(one_way_lane_offset(nl))
        .fadd(xi(i32::from(rd::<i8>(veh, 0x3B8))))
        .fmul_m(f32::from_bits(0x40AC_CCCD));
    let mut p2 = [
        c8(nl, 0).fadd(o2.fmul_m(ndy)).to_f32(),
        c8(nl, 2).fsub(o2.fmul_m(ndx)).to_f32(),
        0.0,
    ];
    let mut p1 = [
        c8(cur, 0).fadd(x(o1).fmul_m(cdy)).to_f32(),
        c8(cur, 2).fsub(x(o1).fmul_m(cdx)).to_f32(),
        0.0,
    ];
    let s6 = xi(i32::from(rd::<i8>(veh, 0x3B6)));
    let s5 = xi(i32::from(rd::<i8>(veh, 0x3B5)));
    let d2y = link_dir(nl, 9, s6).to_f32();
    let d2x = link_dir(nl, 8, s6).to_f32();
    let d1y = link_dir(cur, 9, s5).to_f32();
    let d1x = link_dir(cur, 8, s5).to_f32();
    let s = X87::from_f64_exact(calc_speed_scale_factor(
        lp(&mut p1),
        lp(&mut p2),
        d1x,
        d1y,
        d2x,
        d2y,
    ));
    let v = ftol32(s.fmul(x(1000.0).fdiv_m(rd::<f32>(veh, 0x3C8))));
    wr(veh, 0x3A0, v);
    wr(veh, 0x3A0, if v > 10 { v } else { 10 });
}

orig! {
    /// `CCarCtrl::PickNextNodeToChaseCar(CVehicle*, float x, float y, float z, CEntity*)` (0x426EF0; five dwords)
    fn pick_next_node_to_chase_car_call(v: GPtr<u8>, px: f32, py: f32, pz: f32, target: GPtr<u8>) -> u8 = cdecl 0x426EF0 => unported;
    /// `CCarCtrl::PickNextNodeToFollowPath(CVehicle*)` (0x427740)
    fn pick_next_node_to_follow_path_call(v: GPtr<u8>) -> u8 = cdecl 0x427740 => unported;
    /// `CCarCtrl::PickNextNodeRandomly(CVehicle*)` (0x42DE80)
    fn pick_next_node_randomly_call(v: GPtr<u8>) = cdecl 0x42DE80 => pick_next_node_randomly;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
}

/// `CCarCtrl::PickNextNodeAccordingStrategy(CVehicle*)` (0x432B10): +0x3D1 = the next node's highway bits
/// (+0x19 bits 4..5), 0xFF on behaviour types 8 / 9; then by mission (byte table 0x432C64, jump table
/// 0x432C54): 2, 4, 0x2B chase the player (`FindPlayerCoors` x / z / y in three calls, the player vehicle as
/// fifth argument), 8, 0xC follow the path, 0xF, 0x11, 0x34, 0x36, 0x3D, 0x41..0x44 chase the target entity
/// (+0x41C, its position read three times), every other mission picks randomly and returns 0. The next node
/// is not validated (as the bytes).
#[hook(addr = 0x432B10, conv = cdecl, name = "CCarCtrl::PickNextNodeAccordingStrategy", expect = "83 EC 24 56 8B 74 24 2C")]
pub fn pick_next_node_according_strategy(veh: GPtr<u8>) -> u8 {
    let n = path_node(rd::<u32>(veh, 0x394));
    wr(veh, 0x3D1, (rd::<u8>(n, 0x19) >> 4) & 3);
    if matches!(rd::<u8>(n, 0x1A) >> 4, 8 | 9) {
        wr::<u8>(veh, 0x3D1, 0xFF);
    }
    let m = i32::from(rd::<i8>(veh, 0x3BA));
    match m {
        2 | 4 | 0x2B => {
            let mut a = [0f32; 3];
            let mut b = [0f32; 3];
            let mut c = [0f32; 3];
            let pa = crate::common::find_player_coors(lp(&mut a), -1);
            let pv = find_player_vehicle(-1, 0);
            let z = crate::common::find_player_coors(lp(&mut b), -1)
                .wrapping_index(2)
                .read();
            let y = crate::common::find_player_coors(lp(&mut c), -1)
                .wrapping_index(1)
                .read();
            pick_next_node_to_chase_car_call(veh, pa.read(), y, z, pv)
        }
        8 | 0xC => pick_next_node_to_follow_path_call(veh),
        0xF | 0x11 | 0x34 | 0x36 | 0x3D | 0x41..=0x44 => {
            let t = rd::<GPtr<u8>>(veh, 0x41C);
            let p = entity_pos(t);
            pick_next_node_to_chase_car_call(
                veh,
                p.read(),
                p.wrapping_index(1).read(),
                p.wrapping_index(2).read(),
                t,
            )
        }
        _ => {
            pick_next_node_randomly_call(veh);
            0
        }
    }
}

orig! {
    /// `CPathFind::DoPathSearch(...)` (0x4515D0, thiscall on ThePaths, `ret 0x4C`)
    fn do_path_search(this: GPtr<u8>, path_type: u32, ox: f32, oy: f32, oz: f32, origin_addr: u32, tx: f32, ty: f32, tz: f32, out_nodes: GPtr<u32>, out_count: GPtr<i16>, max_nodes: i32, out_distance: GPtr<f32>, max_search: f32, target_addr: GPtr<u32>, max_depth: f32, same_lane: u32, forbidden: u32, allow_water: u32, for_boats: u32) = thiscall 0x4515D0 => crate::pathfind::do_path_search;
    /// `CCarCtrl::StopCarIfNodesAreInvalid(CVehicle*)` (0x422590)
    fn stop_car_if_nodes_are_invalid(v: GPtr<u8>) -> u8 = cdecl 0x422590 => crate::carctrl2::stop_car_if_nodes_are_invalid;
}

/// `ThePaths` (0x96F050).
fn the_paths() -> GPtr<u8> {
    g::<u8>(0x96F050)
}

/// The f32 constant at `a` in the image.
fn kf(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// `CWeather::WeatherRegion` is countryside (0) or desert (4).
fn countryside() -> bool {
    matches!(g::<i16>(0xC81314).read(), 0 | 4)
}

/// The lane clamp shared by the node pickers: `min((i8)nextLane, lanes - 1)`, 0 when not positive.
fn clamp_next_lane(veh: GPtr<u8>, lanes: i32) {
    let mut l = i32::from(rd::<i8>(veh, 0x3B8));
    if l >= lanes - 1 {
        l = lanes - 1;
    }
    wr(veh, 0x3B8, l as u8);
    if (l as u8 as i8) <= 0 {
        wr::<u8>(veh, 0x3B8, 0);
    }
}

/// `(X.x/8 - C.x/8)^2 + (X.y/8 - C.y/8)^2` of two link positions (x87, x term first).
fn link_dist2(c: GPtr<u8>, xl: GPtr<u8>) -> X87 {
    let dx = c8(xl, 0).fsub(c8(c, 0));
    let dy = c8(xl, 2).fsub(c8(c, 2));
    dx.fmul(dx).fadd(dy.fmul(dy))
}

/// The commit step shared by the chase / follow pickers' link bookkeeping: previous ← current, current ←
/// next link, the curve timer, the direction and lane bytes.
fn shift_links(veh: GPtr<u8>) {
    wr(
        veh,
        0x39C,
        rd::<u32>(veh, 0x39C).wrapping_add(rd::<u32>(veh, 0x3A0)),
    );
    wr(veh, 0x3A8, rd::<u16>(veh, 0x3A4));
    wr(veh, 0x3A4, rd::<u16>(veh, 0x3A6));
}

/// `CCarCtrl::PickNextNodeToChaseCar(CVehicle*, float x, float y, float z, CEntity*)` (0x426EF0): a two-node
/// `DoPathSearch` from the next node towards the target (maximum distance 50 in the countryside, where a
/// failed search or a path longer than three times the straight distance returns 1); a found neighbour is
/// taken with its link index (the unbounded link search of the bytes), otherwise the link whose heading is
/// closest to the target's (ties take the last, the previous node only when it is the only link). Then the
/// node / link shift, `StopCarIfNodesAreInvalid` (→ 1), the direction and lane count, the lane-change
/// countdown (rand 0x42765C / 0x42766B, links more than 16 m apart, not for the chase missions), the lane
/// clamp (fast lane, slow lane or BMX), two discarded `OneWayLaneOffset` calls; returns 0. The fifth
/// argument is not read. With no acceptable link the chosen node keeps area 0xFFFF and the node word of
/// the uninitialised local `[entry ESP - 0x3A]`: read from the footprint snapshot.
#[hook(addr = 0x426EF0, conv = cdecl, name = "CCarCtrl::PickNextNodeToChaseCar", expect = "83 EC 40 53 55", footprint = 0x100)]
pub fn pick_next_node_to_chase_car(veh: GPtr<u8>, dx: f32, dy: f32, dz: f32, _target: GPtr<u8>) -> u8 {
    let mut res = [0xFFFFu32; 2];
    let seed = rd::<u16>(veh, 0x45E);
    if seed != 0 {
        sa_core::orig::at_site(0x42_6F33, || sa_core::crt::srand(u32::from(seed)));
    }
    let prev = rd::<u32>(veh, 0x390);
    let s = rd::<u32>(veh, 0x394);
    let max_dist = if countryside() {
        50.0
    } else {
        f32::from_bits(0x4974_23FE)
    };
    let ns = path_node(s);
    let o = [c8(ns, 8).to_f32(), c8(ns, 0xA).to_f32(), c8(ns, 0xC).to_f32()];
    let mut count = [0i16];
    let mut path_dist = [0f32];
    do_path_search(
        the_paths(),
        0,
        o[0],
        o[1],
        o[2],
        s,
        dx,
        dy,
        dz,
        GPtr::from_raw(res.as_mut_ptr()),
        GPtr::from_raw(count.as_mut_ptr()),
        2,
        GPtr::from_raw(path_dist.as_mut_ptr()),
        max_dist,
        GPtr::NULL,
        f32::from_bits(0x4974_23FE),
        0,
        g::<u32>(0x8A5F44).read(),
        u32::from(rd::<i16>(veh, 0x22) == 0x21B),
        0,
    );
    let count = count[0];
    if countryside() {
        if count == 0 {
            return 1;
        }
        let v = entity_pos(veh);
        let a = x(dx).fsub_m(v.read());
        let b = x(dy).fsub_m(v.wrapping_index(1).read());
        let d = b.fmul(b).fadd(a.fmul(a)).fsqrt().fmul_m(3.0);
        // fcomp pathDist; test ah, 5; jnp: d < pathDist
        if !d.fcom(x(path_dist[0])).test_parity(0x05) {
            return 1;
        }
    }
    let base = i32::from(rd::<i16>(ns, 0x10));
    let s_area = area_of(s);
    // The unbounded search of S's links for `r` (the bytes rely on DoPathSearch returning a neighbour).
    let link_index = |r: u32| -> i32 {
        let mut k: i32 = 0;
        loop {
            if node_link_at(s_area, base.wrapping_add(i32::from(k as i16))) == r {
                return k;
            }
            k = k.wrapping_add(1);
        }
    };
    let mut chosen: Option<(u32, i32)> = None;
    if count == 1 || count == 2 {
        if res[0] != s {
            chosen = Some((res[0], link_index(res[0])));
        } else if count == 2 && res[1] != s {
            chosen = Some((res[1], link_index(res[1])));
        }
    }
    let (chosen, k) = match chosen {
        Some(c) => c,
        None => {
            let v = entity_pos(veh);
            let target = X87::from_f64_exact(crate::common::get_atan_of_xy(
                x(dx).fsub_m(v.read()).to_f32(),
                x(dy).fsub_m(v.wrapping_index(1).read()).to_f32(),
            ))
            .to_f32();
            let n = i32::from(rd::<u8>(ns, 0x18) & 0xF);
            let mut lo = sa_core::abi::footprint_residue_u8(-0x3A, 0x100) as u32;
            lo |= (sa_core::abi::footprint_residue_u8(-0x39, 0x100) as u32) << 8;
            let mut chosen = 0xFFFF | (lo << 16);
            let mut best = 10.0f32;
            let mut best_k = 0;
            let (pi, two_pi, neg_pi) = (kf(0x858CB8), kf(0x858CBC), kf(0x858CC0));
            let mut k: i32 = 0;
            while (k as i16 as i32) < n {
                let l = node_link_at(s_area, base.wrapping_add(k));
                if !(l == prev && n > 1) && !path_nodes(area_of(l)).is_null() {
                    let ln = path_node(l);
                    let lx = c8(ln, 8).to_f32();
                    let sx = c8(ns, 8).to_f32();
                    let yd = c8(ln, 0xA).fsub(c8(ns, 0xA)).to_f32();
                    let a = X87::from_f64_exact(crate::common::get_atan_of_xy(x(lx).fsub_m(sx).to_f32(), yd));
                    let mut t = a.fsub_m(target);
                    while !t.fcom(x(pi)).test_any(0x41) {
                        t = t.fsub_m(two_pi);
                    }
                    while !t.fcom(x(neg_pi)).test_parity(0x05) {
                        t = t.fadd_m(two_pi);
                    }
                    if !t.fcom(x(0.0)).test_parity(0x05) {
                        t = t.fchs();
                    }
                    // fcom best; test ah, 0x41; jp: greater or unordered skips
                    if !t.fcom(x(best)).test_parity(0x41) {
                        best_k = k;
                        best = t.to_f32();
                        chosen = l;
                    }
                }
                k = k.wrapping_add(1);
            }
            (chosen, best_k)
        }
    };
    let old_cur = rd::<u32>(veh, 0x390);
    wr(veh, 0x390, rd::<u32>(veh, 0x394));
    wr(veh, 0x398, old_cur);
    wr(veh, 0x394, chosen);
    let b5 = rd::<u8>(veh, 0x3B5);
    shift_links(veh);
    wr(veh, 0x3B5, rd::<u8>(veh, 0x3B6));
    wr(veh, 0x3B7, rd::<u8>(veh, 0x3B8));
    wr(veh, 0x3B4, b5);
    wr(
        veh,
        0x3A6,
        navi_link_at(s_area, base.wrapping_add(i32::from(k as i16))),
    );
    if stop_car_if_nodes_are_invalid(veh) != 0 {
        return 1;
    }
    let nn = rd::<u32>(veh, 0x394);
    let (sa, na) = (s & 0xFFFF, nn & 0xFFFF);
    let less = sa < na || (sa == na && (s >> 16) < (nn >> 16));
    let xb = rd::<u8>(navi_link(rd::<u16>(veh, 0x3A6)), 0xB);
    wr::<i8>(veh, 0x3B6, if less { -1 } else { 1 });
    let lanes = i32::from(if less { xb & 7 } else { (xb >> 3) & 7 });
    let c = navi_link(rd::<u16>(veh, 0x3A4));
    let xl = navi_link(rd::<u16>(veh, 0x3A6));
    if !link_dist2(c, xl).fcom(x(256.0)).test_any(0x41)
        && !matches!(
            rd::<u8>(veh, 0x3BA),
            2 | 4 | 0xF | 0x34 | 0x11 | 0x36 | 0x2B | 0x3D
        )
    {
        let cd = rd::<u8>(veh, 0x3E0).wrapping_sub(1);
        wr(veh, 0x3E0, cd);
        if cd == 0 {
            wr(veh, 0x3E0, ((rand_at(0x42765C) as u8) & 3).wrapping_add(4));
            let r = rand_at(0x42766B);
            let l = rd::<u8>(veh, 0x3B8);
            wr(
                veh,
                0x3B8,
                if r < 0x3FFF {
                    l.wrapping_add(1)
                } else {
                    l.wrapping_sub(1)
                },
            );
        }
    }
    clamp_next_lane(veh, lanes);
    let flags = rd::<u8>(veh, 0x3DB);
    if flags & 8 != 0 {
        wr::<u8>(veh, 0x3B8, 0);
    } else if flags & 0x10 != 0 || rd::<i32>(veh, 0x594) == 10 {
        wr(veh, 0x3B8, if lanes - 1 < 0 { 0 } else { (lanes - 1) as u8 });
    }
    one_way_lane_offset(navi_link(rd::<u16>(veh, 0x3A4)));
    one_way_lane_offset(navi_link(rd::<u16>(veh, 0x3A6)));
    0
}

orig! {
    /// `CAutoPilot::RemoveOnePathNode()` (0x41B950, thiscall on `veh + 0x390`)
    fn remove_one_path_node(ap: GPtr<u8>) = thiscall 0x41B950 => crate::carctrl2::auto_pilot_remove_one_path_node;
    /// `CCarCtrl::SwitchVehicleToRealPhysics(CVehicle*)` (0x423FC0)
    fn switch_vehicle_to_real_physics(v: GPtr<u8>) = cdecl 0x423FC0 => crate::carctrl::switch_vehicle_to_real_physics;
}

/// `CCarCtrl::PickNextNodeToFollowPath(CVehicle*)` (0x427740): with no stored path nodes (+0x418 <= 0) an
/// eight-node `DoPathSearch` from the vehicle to its destination (+0x3EC; same-lane-only from flag 0x40 of
/// +0x3DB) into +0x3F8; fewer than two nodes → 1; a first node equal to the next node is dropped (twice at
/// most); none left → 1. Then the next node is the first path node (removed from the list), the links
/// shift, the link index is searched without bound (as the bytes), the direction and lane count (the
/// reverse mapping of `PickNextNodeToChaseCar`), a random lane change (rand 0x427C79 with `& 0x600 == 0`,
/// rand 0x427C83) when the links are more than 16 m apart, the lane clamp, the curve duration, and a parking
/// node (behaviour 2) switches to real physics with mission 0x2D. Returns 0.
// Unhookable: protection code reads and calls its entry dwords as data (0x146C35E, 0x146C570, 0x146C585 `call [0x427744]`, ...; sa-hook every_hook_installs_on_the_exe).
pub fn pick_next_node_to_follow_path(veh: GPtr<u8>) -> u8 {
    let seed = rd::<u16>(veh, 0x45E);
    if seed != 0 {
        sa_core::orig::at_site(0x42_7760, || sa_core::crt::srand(u32::from(seed)));
    }
    let s0 = rd::<u32>(veh, 0x394);
    let count = veh.byte_add(0x418).cast::<i16>();
    let nodes = veh.byte_add(0x3F8).cast::<u32>();
    let ap = veh.byte_add(0x390);
    if count.read() <= 0 {
        let v = entity_pos(veh);
        let d = veh.byte_add(0x3EC).cast::<f32>();
        do_path_search(
            the_paths(),
            0,
            v.read(),
            v.wrapping_index(1).read(),
            v.wrapping_index(2).read(),
            rd::<u32>(veh, 0x394),
            d.read(),
            d.wrapping_index(1).read(),
            d.wrapping_index(2).read(),
            nodes,
            count,
            8,
            GPtr::NULL,
            f32::from_bits(0x4974_23FE),
            GPtr::NULL,
            f32::from_bits(0x4974_23FE),
            u32::from((rd::<u8>(veh, 0x3DB) >> 6) & 1),
            g::<u32>(0x8A5F44).read(),
            u32::from(rd::<i16>(veh, 0x22) == 0x21B),
            0,
        );
        if count.read() < 2 {
            return 1;
        }
        for _ in 0..2 {
            if nodes.read() == rd::<u32>(veh, 0x394) {
                remove_one_path_node(ap);
            }
        }
        if count.read() < 1 {
            return 1;
        }
    }
    let first = nodes.read();
    let old_next = rd::<u32>(veh, 0x394);
    let old_cur = rd::<u32>(veh, 0x390);
    wr(veh, 0x394, first);
    wr(veh, 0x398, old_cur);
    wr(veh, 0x390, old_next);
    remove_one_path_node(ap);
    let b5 = rd::<u8>(veh, 0x3B5);
    shift_links(veh);
    wr(veh, 0x3B5, rd::<u8>(veh, 0x3B6));
    wr(veh, 0x3B4, b5);
    wr(veh, 0x3B7, rd::<u8>(veh, 0x3B8));
    let s = rd::<u32>(veh, 0x390);
    let base = i32::from(rd::<i16>(path_node(s), 0x10));
    let target = rd::<u32>(veh, 0x394);
    let mut k: i32 = 0;
    while node_link_at(area_of(s), base.wrapping_add(i32::from(k as i16))) != target {
        k = k.wrapping_add(1);
    }
    wr(
        veh,
        0x3A6,
        navi_link_at(area_of(s), base.wrapping_add(i32::from(k as i16))),
    );
    let (sa, na) = (s0 & 0xFFFF, target & 0xFFFF);
    let less = sa < na || (sa == na && (s0 >> 16) < (target >> 16));
    let xb = rd::<u8>(navi_link(rd::<u16>(veh, 0x3A6)), 0xB);
    wr::<i8>(veh, 0x3B6, if less { -1 } else { 1 });
    let lanes = i32::from(if less { (xb >> 3) & 7 } else { xb & 7 });
    let cur = navi_link(rd::<u16>(veh, 0x3A4));
    let s5 = xi(i32::from(rd::<i8>(veh, 0x3B5)));
    let cdx = link_dir(cur, 8, s5).to_f32();
    let cdy = link_dir(cur, 9, s5).to_f32();
    let nl = navi_link(rd::<u16>(veh, 0x3A6));
    let s6 = xi(i32::from(rd::<i8>(veh, 0x3B6)));
    let ndx = link_dir(nl, 8, s6).to_f32();
    let ndy = link_dir(nl, 9, s6).to_f32();
    if !link_dist2(cur, nl).fcom(x(256.0)).test_any(0x41) && rand_at(0x427C79) & 0x600 == 0 {
        let r = rand_at(0x427C83);
        let l = rd::<u8>(veh, 0x3B8);
        wr(
            veh,
            0x3B8,
            if r < 0x3FFF {
                l.wrapping_add(1)
            } else {
                l.wrapping_sub(1)
            },
        );
    }
    clamp_next_lane(veh, lanes);
    let flags = rd::<u8>(veh, 0x3DB);
    if flags & 8 != 0 {
        wr::<u8>(veh, 0x3B8, 0);
    } else if flags & 0x10 != 0 {
        wr(veh, 0x3B8, if lanes - 1 < 0 { 0 } else { (lanes - 1) as u8 });
    }
    curve_timing(veh, cdx, cdy, ndx, ndy);
    let n = rd::<u32>(veh, 0x394);
    if !path_nodes(area_of(n)).is_null() && rd::<u8>(path_node(n), 0x1A) & 0xF0 == 0x20 {
        switch_vehicle_to_real_physics(veh);
        wr::<u8>(veh, 0x3BA, 0x2D);
    }
    0
}

orig! {
    /// `CCarCtrl::JoinCarWithRoadSystemGotoCoors(CVehicle*, const CVector&, bool, bool)` (0x42F870)
    fn join_car_with_road_system_goto_coors(v: GPtr<u8>, c: GPtr<f32>, a: u32, b: u32) -> u8 = cdecl 0x42F870 => unported;
    /// `CCarAI::GetCarToGoToCoors(CVehicle*, const CVector&, eCarDrivingStyle, bool)` (0x41CE30, ST0)
    fn get_car_to_go_to_coors_call(v: GPtr<u8>, c: GPtr<f32>, style: u32, cruise: u32) -> f64 = cdecl 0x41CE30 => get_car_to_go_to_coors_abi;
}

/// `sqrt((v.y - c.y)^2 + (v.x - c.x)^2)` of the vehicle position and `c` (the tail of the GetCarToGoTo*
/// functions, y term first).
fn dist2d_to(veh: GPtr<u8>, c: GPtr<f32>) -> X87 {
    let v = entity_pos(veh);
    let dy = x(v.wrapping_index(1).read()).fsub_m(c.wrapping_index(1).read());
    let dx = x(v.read()).fsub_m(c.read());
    dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt()
}

/// `|c - dest|` (x or y, `fabs` by `fchs` below zero) is greater than `k` (+0x3EC / +0x3F0, `test ah, 0x41`
/// greater only).
fn dest_moved(veh: GPtr<u8>, c: GPtr<f32>, k: f32) -> bool {
    let gt = |i: i32, off: isize| {
        let mut d = x(c.wrapping_index(i).read()).fsub_m(rd::<f32>(veh, off));
        if !d.fcom(x(0.0)).test_parity(0x05) {
            d = d.fchs();
        }
        !d.fcom(x(k)).test_any(0x41)
    };
    gt(0, 0x3EC) || gt(1, 0x3F0)
}

/// The shared "new mission" prologue: driving style, temp action cleared, cruise speed 20 when asked,
/// mission start time now, status physics unless ghost (6).
fn start_goto(veh: GPtr<u8>, style: u8, cruise: u8) {
    wr(veh, 0x3B9, style);
    wr::<u8>(veh, 0x3BB, 0);
    if cruise != 0 {
        wr::<u8>(veh, 0x3D0, 0x14);
    }
    wr(veh, 0x3AC, now());
    let s = rd::<u8>(veh, 0x36);
    if s & 0xF8 != 0x60 {
        wr(veh, 0x36, (s & 7) | 0x18);
    }
}

fn set_dest(veh: GPtr<u8>, c: GPtr<f32>) {
    for i in 0..3 {
        wr(veh, 0x3EC + 4 * i as isize, c.wrapping_index(i).read());
    }
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn get_car_to_go_to_coors_abi(v: GPtr<u8>, c: GPtr<f32>, style: u32, cruise: u32) -> f64 {
    get_car_to_go_to_coors(v, c, style as u8, cruise as u8).to_f64()
}

/// `CCarAI::GetCarToGoToCoors(CVehicle*, const CVector&, eCarDrivingStyle, bool setCruiseSpeed)` (0x41CE30,
/// ST0): the parking missions 0x2D, 0x2E, 0x31, 0x32 (byte table 0x41CFA0) keep their mission; missions 8 / 9
/// keep it while the target is within 5 (x and y) of the stored destination (gta-reversed tests the vehicle
/// position instead); otherwise a new go-to mission (9 when `JoinCarWithRoadSystemGotoCoors` says straight
/// line, else 8). Returns the 2D distance from the vehicle to the target.
#[hook(addr = 0x41CE30, conv = cdecl, name = "CCarAI::GetCarToGoToCoors", expect = "53 56 8B 74 24 0C")]
pub fn get_car_to_go_to_coors(veh: GPtr<u8>, c: GPtr<f32>, style: u8, cruise: u8) -> X87 {
    let m = rd::<u8>(veh, 0x3BA);
    let keep = match m {
        0x2D | 0x2E | 0x31 | 0x32 => true,
        8 | 9 => !dest_moved(veh, c, 5.0),
        _ => false,
    };
    if !keep {
        start_goto(veh, style, cruise);
        let j = join_car_with_road_system_goto_coors(veh, c, 0, 0);
        wr(veh, 0x3BA, u8::from(j != 0) + 8);
    }
    dist2d_to(veh, c)
}

/// `CCarAI::GetCarToGoToCoorsStraightLine(CVehicle*, const CVector&, eCarDrivingStyle, bool)` (0x41CFB0, ST0):
/// mission 9 only moves the destination when it is more than 2 away (x or y); otherwise a new straight-line
/// mission 9 to the target (no road-system join).
#[hook(addr = 0x41CFB0, conv = cdecl, name = "CCarAI::GetCarToGoToCoorsStraightLine", expect = "8B 4C 24 04 8A 81 BA 03 00 00")]
pub fn get_car_to_go_to_coors_straight_line(veh: GPtr<u8>, c: GPtr<f32>, style: u8, cruise: u8) -> X87 {
    if rd::<u8>(veh, 0x3BA) == 9 {
        if dest_moved(veh, c, 2.0) {
            set_dest(veh, c);
        }
    } else {
        start_goto(veh, style, cruise);
        set_dest(veh, c);
        wr::<u8>(veh, 0x3BA, 9);
    }
    dist2d_to(veh, c)
}

/// `CCarAI::GetCarToGoToCoorsAccurate(CVehicle*, const CVector&, eCarDrivingStyle, bool)` (0x41D0E0, ST0):
/// missions 0xC / 0xD move the destination when it is more than 2 away; otherwise a new accurate go-to
/// (0xD straight line from `JoinCarWithRoadSystemGotoCoors`, else 0xC).
// Unhookable: protection code calls through its entry dwords as data (`call [0x41D0E0]` at 0x1463AE5, 0x1463B0F, 0x146955E; sa-hook every_hook_installs_on_the_exe).
pub fn get_car_to_go_to_coors_accurate(veh: GPtr<u8>, c: GPtr<f32>, style: u8, cruise: u8) -> X87 {
    if matches!(rd::<u8>(veh, 0x3BA), 0xC | 0xD) {
        if dest_moved(veh, c, 2.0) {
            set_dest(veh, c);
        }
    } else {
        start_goto(veh, style, cruise);
        let j = join_car_with_road_system_goto_coors(veh, c, 0, 0);
        wr(veh, 0x3BA, u8::from(j != 0) + 0xC);
    }
    dist2d_to(veh, c)
}

/// `CCarAI::GetCarToGoToCoorsRacing(CVehicle*, const CVector&, eCarDrivingStyle, bool)` (0x41D210, ST0):
/// missions 0x21 / 9 move the destination when it is more than 2 away; otherwise destination, mission 0x21
/// and a `JoinCarWithRoadSystemGotoCoors` whose result is ignored.
#[hook(addr = 0x41D210, conv = cdecl, name = "CCarAI::GetCarToGoToCoorsRacing", expect = "53 56 8B 74 24 0C")]
pub fn get_car_to_go_to_coors_racing(veh: GPtr<u8>, c: GPtr<f32>, style: u8, cruise: u8) -> X87 {
    if matches!(rd::<u8>(veh, 0x3BA), 0x21 | 9) {
        if dest_moved(veh, c, 2.0) {
            set_dest(veh, c);
        }
    } else {
        start_goto(veh, style, cruise);
        set_dest(veh, c);
        wr::<u8>(veh, 0x3BA, 0x21);
        join_car_with_road_system_goto_coors(veh, c, 0, 0);
    }
    dist2d_to(veh, c)
}

/// `CCarAI::GetCarToParkAtCoors(CVehicle*, const CVector&)` (0x41D350, ST0): `GetCarToGoToCoors(veh, c, 0,
/// true)` (result dropped), +0x42C bit 2, cruise speed 10, the 2D distance.
#[hook(addr = 0x41D350, conv = cdecl, name = "CCarAI::GetCarToParkAtCoors", expect = "53 56 8B 74 24 0C")]
pub fn get_car_to_park_at_coors(veh: GPtr<u8>, c: GPtr<f32>) -> X87 {
    get_car_to_go_to_coors_call(veh, c, 0, 1);
    wr(veh, 0x42C, rd::<u8>(veh, 0x42C) | 4);
    wr::<u8>(veh, 0x3D0, 0xA);
    dist2d_to(veh, c)
}

orig! {
    /// `CVector::Normalise()` (0x59C910, thiscall)
    fn vector_normalise(v: GPtr<f32>) = thiscall 0x59C910 => unported;
    /// `CWorld::FindPlayerWanted(int32)` (0x56E230)
    fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CCullZones::NoPolice()` (0x72DD50)
    fn cull_zones_no_police() -> u8 = cdecl 0x72DD50 => unported;
    /// `CCarCtrl::JoinCarWithRoadSystem(CVehicle*)` (0x42F5A0)
    fn join_car_with_road_system_call(v: GPtr<u8>) = cdecl 0x42F5A0 => join_car_with_road_system;
}

/// The forward vector of an entity as the x87 sees it: the matrix's (+0x14 → +0x10..+0x18), or
/// `(-sin h, cos h, 0)` of the simple placement heading (+0x10).
fn forward(e: GPtr<u8>) -> [X87; 3] {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        let h = x(rd::<f32>(e, 0x10));
        [h.fsin().fchs(), h.fcos(), x(0.0)]
    } else {
        [
            x(rd::<f32>(m, 0x10)),
            x(rd::<f32>(m, 0x14)),
            x(rd::<f32>(m, 0x18)),
        ]
    }
}

/// `CCarAI::EntitiesGoHeadOn(CEntity* a, CEntity* b)` (0x41CD00): `d = normalise(a.pos - b.pos)`; true when
/// `a.forward · d <= -0.8` (or unordered) and `b.forward · d >= 0.8` (or unordered); each dot product is
/// `(d.z f.z + f.y d.y) + f.x d.x` in x87.
#[hook(addr = 0x41CD00, conv = cdecl, name = "CCarAI::EntitiesGoHeadOn", expect = "83 EC 18 57 8B 7C 24 24")]
pub fn entities_go_head_on(a: GPtr<u8>, b: GPtr<u8>) -> u8 {
    let pb = entity_pos(b);
    let pa = entity_pos(a);
    let dz = x(pa.wrapping_index(2).read()).fsub_m(pb.wrapping_index(2).read());
    let dy = x(pa.wrapping_index(1).read()).fsub_m(pb.wrapping_index(1).read());
    let dx = x(pa.read()).fsub_m(pb.read());
    let mut d = [dx.to_f32(), dy.to_f32(), dz.to_f32()];
    vector_normalise(lp(&mut d));
    let dot = |e: GPtr<u8>| {
        let [fx, fy, fz] = forward(e);
        x(d[2]).fmul(fz).fadd(fy.fmul_m(d[1])).fadd(fx.fmul_m(d[0]))
    };
    // fcomp -0.8; test ah, 0x41; je: greater fails
    if !dot(a).fcom(x(f32::from_bits(0xBF4C_CCCD))).test_any(0x41) {
        return 0;
    }
    // fcomp 0.8; test ah, 5; jp: not less passes
    u8::from(dot(b).fcom(x(f32::from_bits(0x3F4C_CCCD))).test_parity(0x05))
}

/// `CCarAI::BackToCruisingIfNoWantedLevel(CVehicle*)` (stub 0x41BFA0 → 0x15612E0): law enforcers only; when
/// the wanted flags +0x1E have bit 2 set, or the player's wanted level (`FindPlayerPed(-1)` through an
/// obfuscated call `[0x1560474] - [0x638D32]` = 0x56E210, +0x480 → +0x2C) is 0, or any of bits 0..2 are set,
/// or `CCullZones::NoPolice()`: `JoinCarWithRoadSystem`, +0x42D bit 7 cleared, mission 1 (cruise), driving
/// style 0, mission 0 in a no-police zone.
#[hook(addr = 0x41BFA0, conv = cdecl, name = "CCarAI::BackToCruisingIfNoWantedLevel", expect = "E9 3B 53 14 01")]
pub fn back_to_cruising_if_no_wanted_level(veh: GPtr<u8>) {
    if rd::<u8>(veh, 0x428) & 1 == 0 {
        return;
    }
    let join = rd::<u8>(find_player_wanted(-1), 0x1E) & 4 != 0 || {
        let pd = rd::<GPtr<u8>>(crate::common::find_player_ped(-1), 0x480);
        let w = if pd.is_null() {
            GPtr::NULL
        } else {
            rd::<GPtr<u8>>(pd, 0)
        };
        rd::<i32>(w, 0x2C) == 0
            || rd::<u8>(find_player_wanted(-1), 0x1E) & 7 != 0
            || cull_zones_no_police() != 0
    };
    if !join {
        return;
    }
    join_car_with_road_system_call(veh);
    wr::<u8>(veh, 0x3BA, 1);
    wr::<u8>(veh, 0x3B9, 0);
    wr(veh, 0x42D, rd::<u8>(veh, 0x42D) & 0x7F);
    if cull_zones_no_police() != 0 {
        wr::<u8>(veh, 0x3BA, 0);
    }
}

/// `sqrt((dz^2 + dy^2) + dx^2)` of `a - b` (x87).
fn dist3(a: GPtr<f32>, b: GPtr<f32>) -> X87 {
    let dx = x(a.read()).fsub_m(b.read());
    let dy = x(a.wrapping_index(1).read()).fsub_m(b.wrapping_index(1).read());
    let dz = x(a.wrapping_index(2).read()).fsub_m(b.wrapping_index(2).read());
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt()
}

/// `fcomp k; test ah, 5; jp` not taken: `v < k`.
fn lt_k(v: X87, k: f32) -> bool {
    !v.fcom(x(k)).test_parity(0x05)
}

/// `CCarAI::MellowOutChaseSpeed(CVehicle*)` (0x41D3D0): cruise speed (+0x3D0) of a chasing car by wanted
/// level and the vehicle–player distance (stored f32): level 1 in a vehicle < 10 → 15, < 20 → 22, else 25;
/// on foot < 20 → 5, < 40 → 13, else 25; level 2 in a vehicle < 10 → 27, < 20 → 30, else 34; on foot < 20 →
/// 5, < 40 → 18, else 34. Then a player on foot moving slower than 0.07 within 30 caps it at 10.
#[hook(addr = 0x41D3D0, conv = cdecl, name = "CCarAI::MellowOutChaseSpeed", expect = "83 EC 10 56 57")]
pub fn mellow_out_chase_speed(veh: GPtr<u8>) {
    // the original reads the wanted level a second time for the level 2 test
    let table: Option<[u8; 6]> = if rd::<i32>(find_player_wanted(-1), 0x2C) == 1 {
        Some([0xF, 0x16, 0x19, 5, 0xD, 0x19])
    } else if rd::<i32>(find_player_wanted(-1), 0x2C) == 2 {
        Some([0x1B, 0x1E, 0x22, 5, 0x12, 0x22])
    } else {
        None
    };
    if let Some(t) = table {
        let mut p = [0f32; 3];
        let pp = crate::common::find_player_coors(lp(&mut p), -1);
        let d = dist3(entity_pos(veh), pp).to_f32();
        let s = if !find_player_vehicle(-1, 0).is_null() {
            if lt_k(x(d), kf(0x85862C)) {
                t[0]
            } else if lt_k(x(d), kf(0x858BA4)) {
                t[1]
            } else {
                t[2]
            }
        } else if lt_k(x(d), kf(0x858BA4)) {
            t[3]
        } else if lt_k(x(d), kf(0x858A10)) {
            t[4]
        } else {
            t[5]
        };
        wr(veh, 0x3D0, s);
    }
    if !find_player_vehicle(-1, 0).is_null() {
        return;
    }
    let ped = crate::common::find_player_ped(-1);
    let (sx, sy, sz) = (
        x(rd::<f32>(ped, 0x44)),
        x(rd::<f32>(ped, 0x48)),
        x(rd::<f32>(ped, 0x4C)),
    );
    let speed = sx.fmul(sx).fadd(sy.fmul(sy)).fadd(sz.fmul(sz)).fsqrt();
    if !lt_k(speed, kf(0x858CA8)) {
        return;
    }
    let mut p = [0f32; 3];
    let pp = crate::common::find_player_coors(lp(&mut p), -1);
    if !lt_k(dist3(pp, entity_pos(veh)), kf(0x858CA4)) {
        return;
    }
    let c = rd::<u8>(veh, 0x3D0);
    wr(veh, 0x3D0, if c < 0xA { c } else { 0xA });
}

/// `CCarAI::MakeWayForCarWithSiren(CVehicle* siren)` (0x41D660): when the siren car moves faster (2D) than
/// [0x858B1C], every random, non-law, ambient-driven car (type 0 / 9, status 2 / 3, flags +0x428 bits 0..2
/// clear, +0x42E bit 3 clear, not class 5, a random driver or none) within 5 m of height, closer than
/// `speed * [0x858CB4] + 40` and moving faster than [0x858C28] reacts (vehicle pool from the top): ahead of
/// the siren (`dir · d / |d| > 0.8`) and roughly crossing (`[0x858CAC] <= fwd · dir <= [0x858CB0]`) it waits 4 s
/// when coming towards; otherwise it swerves (temp action 0x14 / 0x15 by the side, swapped when facing
/// against, 0x15 then becomes 1) for 2.5 s and switches to physics; behind it waits 2 s when coming
/// towards. The original stores the candidate's status in its argument slot.
#[hook(addr = 0x41D660, conv = cdecl, name = "CCarAI::MakeWayForCarWithSiren", expect = "83 EC 24 55 8B 6C 24 2C")]
pub fn make_way_for_car_with_siren(siren: GPtr<u8>) {
    let svx = rd::<f32>(siren, 0x44);
    let svy = rd::<f32>(siren, 0x48);
    let speed = x(svx).fmul_m(svx).fadd(x(svy).fmul_m(svy)).fsqrt().to_f32();
    if lt_k(x(speed), kf(0x858B1C)) {
        return;
    }
    let pool = g::<GPtr<u8>>(0xB74494).read();
    let size = rd::<i32>(pool, 8);
    if size == 0 {
        return;
    }
    let inv = x(1.0).fdiv_m(speed);
    let dirx = inv.fmul_m(svx).to_f32();
    let diry = inv.fmul_m(svy).to_f32();
    let objects = rd::<GPtr<u8>>(pool, 0);
    let flags = rd::<GPtr<u8>>(pool, 4);
    let sp = entity_pos(siren);
    let mut i = size;
    while i != 0 {
        i -= 1;
        if (flags.byte_add(i as isize).read() as i8) < 0 {
            continue;
        }
        let v = objects.byte_add(i as isize * 0xA18);
        if v.is_null() {
            continue;
        }
        let ty = rd::<u32>(v, 0x590);
        if ty != 0 && ty != 9 {
            continue;
        }
        let b36 = rd::<u8>(v, 0x36);
        let status = b36 >> 3;
        if status != 2 && status != 3 {
            continue;
        }
        if rd::<u8>(v, 0x4A4) != 1 {
            continue;
        }
        let f428 = rd::<u8>(v, 0x428);
        if f428 & 1 != 0 {
            continue;
        }
        let drv = rd::<GPtr<u8>>(v, 0x460);
        if !drv.is_null() && rd::<u8>(drv, 0x484) != 1 {
            continue;
        }
        if v == siren || f428 & 6 != 0 || rd::<u8>(v, 0x42E) & 8 != 0 || vehicle_class(v) == 5 {
            continue;
        }
        let vp = entity_pos(v);
        let mut dz = x(sp.wrapping_index(2).read()).fsub_m(vp.wrapping_index(2).read());
        if !dz.fcom(x(0.0)).test_parity(0x05) {
            dz = dz.fchs();
        }
        if !lt_k(dz, kf(0x858C80)) {
            continue;
        }
        let dx = x(vp.read()).fsub_m(sp.read()).to_f32();
        let dy = x(vp.wrapping_index(1).read())
            .fsub_m(sp.wrapping_index(1).read())
            .to_f32();
        let dist = x(dy).fmul_m(dy).fadd(x(dx).fmul_m(dx)).fsqrt().to_f32();
        // fcomp dist; test ah, 0x41; jne: only `speed * k + 40 > dist` goes on
        if x(speed)
            .fmul_m(kf(0x858CB4))
            .fadd_m(kf(0x858A10))
            .fcom(x(dist))
            .test_any(0x41)
        {
            continue;
        }
        let vvx = rd::<f32>(v, 0x44);
        let vvy = rd::<f32>(v, 0x48);
        if x(vvy)
            .fmul_m(vvy)
            .fadd(x(vvx).fmul_m(vvx))
            .fsqrt()
            .fcom(x(kf(0x858C28)))
            .test_any(0x41)
        {
            continue;
        }
        let towards = || {
            !x(dx)
                .fmul_m(vvx)
                .fadd(x(dy).fmul_m(vvy))
                .fcom(x(0.0))
                .test_parity(0x05)
        };
        let wait = |ms: u32| {
            let t = rd::<u8>(v, 0x3BB);
            if t != 1 && t != 0x18 {
                wr::<u8>(v, 0x3BB, 1);
                wr(v, 0x3BC, now().wrapping_add(ms));
            }
        };
        let ahead = x(dy).fmul_m(diry).fadd(x(dx).fmul_m(dirx)).fdiv_m(dist);
        if ahead.fcom(x(kf(0x858C98))).test_any(0x41) {
            if towards() {
                wait(2000);
            }
            continue;
        }
        let m = rd::<GPtr<u8>>(v, 0x14);
        let f = x(diry)
            .fmul_m(rd::<f32>(m, 0x14))
            .fadd(x(dirx).fmul_m(rd::<f32>(m, 0x10)))
            .to_f32();
        let crossing = x(f).fcom(x(kf(0x858CB0))).test_any(0x41) && !lt_k(x(f), kf(0x858CAC));
        if crossing {
            if towards() {
                wait(4000);
            }
            continue;
        }
        let t = rd::<u8>(v, 0x3BB);
        if t != 0x14 && t != 0x15 {
            let side = x(dx).fmul_m(diry).fsub(x(dy).fmul_m(dirx));
            wr::<u8>(
                v,
                0x3BB,
                if side.fcom(x(0.0)).test_any(0x41) {
                    0x15
                } else {
                    0x14
                },
            );
            if lt_k(x(f), 0.0) {
                let t = rd::<u8>(v, 0x3BB);
                wr(v, 0x3BB, u8::from(t == 0x14) + 0x14);
            }
            if rd::<u8>(v, 0x3BB) == 0x15 {
                wr::<u8>(v, 0x3BB, 1);
            }
            wr(v, 0x3BC, now().wrapping_add(2500));
        }
        if status != 0xC {
            wr(v, 0x36, (b36 & 7) | 0x18);
        }
    }
}

orig! {
    /// `CVehicle::SetUpDriver(int32 pedType, bool, bool)` (0x6D1A50, thiscall)
    fn set_up_driver(v: GPtr<u8>, ty: i32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x6D1A50 => unported;
    /// `CVehicle::SetupPassenger(int32 seat, int32 pedType, bool, bool)` (0x6D1AA0, thiscall)
    fn setup_passenger(v: GPtr<u8>, seat: i32, ty: i32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x6D1AA0 => unported;
    /// `CPed::GetWantedLevel()` (0x41BE60, thiscall)
    fn ped_get_wanted_level(p: GPtr<u8>) -> i32 = thiscall 0x41BE60 => unported;
    /// `CGeneral::GetRandomNumberInRange<float>(float, float)` (0x41BD90, ST0)
    fn random_float_in_range(a: f32, b: f32) -> f64 = cdecl 0x41BD90 => unported;
    /// `CGeneral::GetRandomNumberInRange<int32>(int32, int32)` (0x407180)
    fn random_int_in_range(a: i32, b: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CPed::GiveDelayedWeapon(eWeaponType, uint32 ammo)` (0x5E89B0, thiscall)
    fn give_delayed_weapon(p: GPtr<u8>, w: u32, ammo: u32) = thiscall 0x5E89B0 => unported;
    /// `CPedIntelligence::ClearTasks(bool, bool)` (0x601420, thiscall)
    fn intel_clear_tasks(i: GPtr<u8>, a: u32, b: u32) = thiscall 0x601420 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexCopInCar::CTaskComplexCopInCar(CVehicle*, CPed* partner, CPed* target, bool driver)` (0x68C7F0)
    fn task_cop_in_car_ctor(t: GPtr<u8>, v: GPtr<u8>, partner: GPtr<u8>, target: GPtr<u8>, driver: u32) -> GPtr<u8> = thiscall 0x68C7F0 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0, thiscall)
    fn task_manager_set_task(m: GPtr<u8>, t: GPtr<u8>, slot: i32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CTaskSimpleCarSetPedOut::CTaskSimpleCarSetPedOut(CVehicle*, int32 door, bool)` (0x6478B0)
    fn task_car_set_ped_out_ctor(t: GPtr<u8>, v: GPtr<u8>, door: i32, b: u32) -> GPtr<u8> = thiscall 0x6478B0 => unported;
    /// `CTaskSimpleCarSetPedOut::ProcessPed(CPed*)` (0x647D10)
    fn task_car_set_ped_out_process_ped(t: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x647D10 => unported;
    /// `CTaskSimpleCarSetPedOut::~CTaskSimpleCarSetPedOut()` (0x647950)
    fn task_car_set_ped_out_dtor(t: GPtr<u8>) = thiscall 0x647950 => unported;
    /// `CPed::AttachPedToEntity(CEntity*, CVector offset, uint16 dir, float limit, eWeaponType)` (0x5E7CB0)
    fn attach_ped_to_entity(p: GPtr<u8>, e: GPtr<u8>, ox: f32, oy: f32, oz: f32, dir: u32, lim: f32, w: u32) = thiscall 0x5E7CB0 => unported;
    /// `CTaskComplexKillPedFromBoat::CTaskComplexKillPedFromBoat(CPed*)` (0x6227C0)
    fn task_kill_ped_from_boat_ctor(t: GPtr<u8>, target: GPtr<u8>) -> GPtr<u8> = thiscall 0x6227C0 => unported;
}

/// The cop-in-car task for `ped` (intelligence +0x47C: ClearTasks, new 0x34-byte task, primary slot 3).
fn give_cop_in_car(veh: GPtr<u8>, ped: GPtr<u8>, partner: GPtr<u8>, driver: u32) {
    intel_clear_tasks(rd::<GPtr<u8>>(ped, 0x47C), 1, 1);
    let t = task_new(0x34);
    let t = if t.is_null() {
        t
    } else {
        task_cop_in_car_ctor(t, veh, partner, crate::common::find_player_ped(-1), driver)
    };
    task_manager_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 3, 1);
}

/// `CCarAI::AddPoliceCarOccupants(CVehicle*, bool alwaysCreatePassenger)` (0x41C070; entry `nop; jmp` into the
/// SEH prologue): once per vehicle (+0x42A bit 6). By model (table 0x41C3F0 / 0x41C3DC): 427, 490 a driver and
/// passengers 0..2; 430 (Predator) with wanted level > 1 a driver taken out and attached to the boat at (0,
/// 3, 2) with a pistol, staying in place and killing the player from the boat, then a driver; 432, 523 a
/// driver; 433, 596..599 a driver, and with wanted level > 1 a passenger, a shotgun (25, 1000 rounds) for the
/// driver (random < 0.25) or else the passenger (random < 0.25) when the level is above 2, and cop-in-car
/// tasks for both; at level <= 1 a passenger when asked or for `GetRandomNumberInRange(0, 100) < 50`.
#[hook(addr = 0x41C070, conv = cdecl, name = "CCarAI::AddPoliceCarOccupants", expect = "90 E9 79 51 FE FF")]
pub fn add_police_car_occupants(veh: GPtr<u8>, always: u8) {
    let f = rd::<u8>(veh, 0x42A);
    if f & 0x40 != 0 {
        return;
    }
    wr(veh, 0x42A, f | 0x40);
    match rd::<i16>(veh, 0x22) {
        427 | 490 => {
            set_up_driver(veh, -1, 0, 0);
            for seat in 0..3 {
                setup_passenger(veh, seat, -1, 0, 0);
            }
        }
        430 => {
            if ped_get_wanted_level(crate::common::find_player_ped(-1)) > 1 {
                let drv = set_up_driver(veh, -1, 0, 0);
                let mut task = [0u32; 16];
                let tp = GPtr::from_raw(task.as_mut_ptr()).cast::<u8>();
                task_car_set_ped_out_ctor(tp, veh, 0xA, 1);
                task_car_set_ped_out_process_ped(tp, drv);
                attach_ped_to_entity(drv, veh, 0.0, 3.0, 2.0, 0, f32::from_bits(0x40C9_0FDB), 0x16);
                wr(drv, 0x46C, rd::<u32>(drv, 0x46C) | 0x20_0000);
                let t = task_new(0x10);
                let t = if t.is_null() {
                    t
                } else {
                    task_kill_ped_from_boat_ctor(t, crate::common::find_player_ped(-1))
                };
                task_manager_set_task(rd::<GPtr<u8>>(drv, 0x47C).byte_add(4), t, 3, 0);
                task_car_set_ped_out_dtor(tp);
            }
            set_up_driver(veh, -1, 0, 0);
        }
        432 | 523 => {
            set_up_driver(veh, -1, 0, 0);
        }
        433 | 596..=599 => {
            let drv = set_up_driver(veh, -1, 0, 0);
            let pd = rd::<GPtr<u8>>(crate::common::find_player_ped(-1), 0x480);
            let w = if pd.is_null() {
                GPtr::NULL
            } else {
                rd::<GPtr<u8>>(pd, 0)
            };
            if rd::<i32>(w, 0x2C) <= 1 {
                if always == 0 && random_int_in_range(0, 100) >= 0x32 {
                    return;
                }
                setup_passenger(veh, 0, -1, 0, 0);
                return;
            }
            let psg = setup_passenger(veh, 0, -1, 0, 0);
            if ped_get_wanted_level(crate::common::find_player_ped(-1)) > 2 {
                let k = kf(0x858C84);
                if lt_k(X87::from_f64_exact(random_float_in_range(0.0, 1.0)), k) {
                    give_delayed_weapon(drv, 0x19, 1000);
                } else if lt_k(X87::from_f64_exact(random_float_in_range(0.0, 1.0)), k) {
                    give_delayed_weapon(psg, 0x19, 1000);
                }
            }
            give_cop_in_car(veh, drv, psg, 1);
            give_cop_in_car(veh, psg, drv, 0);
        }
        _ => {}
    }
}

orig! {
    /// `CCarCtrl::FindSpeedMultiplierWithSpeedFromNodes(int8)` (0x424130, ST0)
    fn find_speed_multiplier_with_speed_from_nodes(s: u32) -> f64 = cdecl 0x424130 => unported;
    /// `CTrafficLights::ShouldCarStopForBridge(CVehicle*)` (0x49D420)
    fn should_car_stop_for_bridge(v: GPtr<u8>) -> u8 = cdecl 0x49D420 => crate::misc2::should_car_stop_for_bridge;
    /// virtual `CVehicle::PlayCarHorn()` (vtable +0xD8)
    fn play_car_horn(v: GPtr<u8>) = thiscall vslot 0xD8 => unported;
}

/// `CCarAI::UpdateCarAI(CVehicle*)` (0x41DA30; entry `nop; jmp` into its SEH prologue). PARTIAL (ADR-002):
/// ported are the early returns (a vehicle recording whose `bUseCarAI` (0x97D6C0) is off, vehicle type 3) and
/// the whole body for mission 0 (none) or 1 (cruise) when the vehicle is not a law enforcer (+0x428 bit 0) or the
/// player's wanted level is 0 (STD-12: for law enforcers the original then only reads the wanted level and adds
/// [`police_gang_area_cruise_speed`]); every other call runs the original body. That path: status 4 / 5 → cruise speed 0 and mission
/// 0; the 2D speed squared (f32) above 0.0025 restarts both timers (+0x3B0, +0x3AC); with no temp action,
/// cruise speed set and mission 1 the stuck test (last collision more than 0.5 s ago restarts +0x3AC; below
/// 0.000144 for `((seed & 0xF) + 40) * 500` ms with styles 0 / 4, else 1000 ms: the reverse mode cycles
/// (reset after 10 s), a cruising ambient car reverses for 0.7 s, others by mode 3 / 1.5 s, 3 / 4 s, 0xD / 2.5 s,
/// 0xE / 2.5 s; random cars with styles 0, 1, 4, 6 switch to style 2; simple status → physics; the horn);
/// one seed in eight is switched to real physics once 30 s after its last switch when cruising and not
/// stopped for a bridge; up.z below -0.7 → wait 1 s; the siren horn (`rand` 0x4203AB, low byte of
/// `rand ^ seed` == 0xAD → +0x514 = 45); the speed multiplier (+0x3D4) moves towards the node speed (cruise)
/// or 1.0 by `timestep * 0.01`, snapping to the target when closer (gta-reversed keeps the old value).
/// gta-reversed's stuck test runs for the listed missions or zero cruise speed; the bytes skip exactly those.
#[hook(addr = 0x41DA30, conv = cdecl, name = "CCarAI::UpdateCarAI", expect = "90 E9 12 62 FE FF", fallback,
       unverified = "std12-pop-1", route = "L2", hazards = "heap-writes", recs = "grove-walk,driving",
       note = "STD-12 extended the partial port to law enforcers while the wanted level is 0")]
pub fn update_car_ai(veh: GPtr<u8>) {
    let rec = rd::<i8>(veh, 0x424);
    if rec >= 0 && g::<u8>(0x97D6C0 + rec as usize).read() == 0 {
        return;
    }
    if rd::<u32>(veh, 0x594) == 3 {
        return;
    }
    let mission = rd::<u8>(veh, 0x3BA);
    let law = rd::<u8>(veh, 0x428) & 1 != 0;
    if mission > 1 || (law && player_wanted_level() > 0) {
        return update_car_ai_original(veh);
    }
    if matches!(rd::<u8>(veh, 0x36) >> 3, 4 | 5) {
        wr::<u8>(veh, 0x3D0, 0);
        wr::<u8>(veh, 0x3BA, 0);
    }
    let (vx, vy) = (rd::<f32>(veh, 0x44), rd::<f32>(veh, 0x48));
    let ms2 = x(vx).fmul_m(vx).fadd(x(vy).fmul_m(vy)).to_f32();
    // fcomp 0.0025; test ah, 0x41; jne: only greater restarts the timers
    if !x(ms2).fcom(x(kf(0x858CD8))).test_any(0x41) {
        wr(veh, 0x3B0, now());
        wr(veh, 0x3AC, now());
    }
    let m = rd::<u8>(veh, 0x3BA);
    if rd::<u8>(veh, 0x3BB) == 0
        && !matches!(m, 0 | 0xB | 6)
        && rd::<u8>(veh, 0x3D0) != 0
        && !matches!(m, 0x1B..=0x20 | 0x35)
    {
        if now().wrapping_sub(rd::<u32>(veh, 0x3C)) > 500 {
            wr(veh, 0x3AC, now());
        }
        if lt_k(x(ms2), kf(0x858CD4)) {
            let style = rd::<u8>(veh, 0x3B9);
            let interval = if style == 0 || style == 4 {
                ((rd::<u8>(veh, 0x20) & 0xF) as i32 + 40) * 500
            } else {
                1000
            };
            let t = now();
            if (t.wrapping_sub(rd::<u32>(veh, 0x3AC)) as i32) > interval {
                if t < rd::<u32>(veh, 0x3C0).wrapping_add(10000) {
                    wr(veh, 0x3C4, ((i32::from(rd::<u8>(veh, 0x3C4)) + 1) % 4) as u8);
                } else {
                    wr::<u8>(veh, 0x3C4, 0);
                }
                wr(veh, 0x3C0, now());
                let set = |a: u8, ms: u32| {
                    wr(veh, 0x3BC, now().wrapping_add(ms));
                    wr::<u8>(veh, 0x3BB, a);
                };
                if m == 1 && rd::<u8>(veh, 0x4A4) != 2 && rd::<u8>(veh, 0x42E) & 8 == 0 {
                    set(3, 700);
                } else {
                    match rd::<u8>(veh, 0x3C4) {
                        0 => set(3, 1500),
                        1 => set(3, 4000),
                        2 => set(0xD, 2500),
                        3 => set(0xE, 2500),
                        _ => {}
                    }
                }
                wr(veh, 0x3AC, now());
                if rd::<u8>(veh, 0x4A4) == 1 && matches!(style as i8, 0 | 1 | 4 | 6) {
                    wr::<u8>(veh, 0x3B9, 2);
                }
                if status_simple(veh) {
                    wr(veh, 0x36, (rd::<u8>(veh, 0x36) & 7) | 0x18);
                }
                play_car_horn(veh);
            }
        }
    }
    if rd::<u8>(veh, 0x20) & 7 == 0 {
        let sw = rd::<u32>(veh, 0x3B0);
        if now().wrapping_sub(sw) > 30000
            && g::<u32>(0xB7CB78).read().wrapping_sub(sw) <= 30000
            && rd::<u8>(veh, 0x3BA) == 1
            && should_car_stop_for_bridge(veh) == 0
        {
            switch_vehicle_to_real_physics(veh);
            wr::<u8>(veh, 0x3B9, 2);
            wr::<u8>(veh, 0x3BB, 3);
            wr(veh, 0x3BC, now().wrapping_add(400));
        }
    }
    if lt_k(x(rd::<f32>(rd::<GPtr<u8>>(veh, 0x14), 0x28)), kf(0x858CD0)) {
        wr::<u8>(veh, 0x3BB, 1);
        wr(veh, 0x3BC, now().wrapping_add(1000));
    }
    if rd::<u8>(veh, 0x42D) & 0x80 != 0 {
        let r = rand_at(0x4203AB) as u32 ^ u32::from(rd::<u16>(veh, 0x20));
        if r as u8 == 0xAD {
            wr::<u32>(veh, 0x514, 0x2D);
        }
    }
    let target = if rd::<u8>(veh, 0x3BA) == 1 {
        X87::from_f64_exact(find_speed_multiplier_with_speed_from_nodes(u32::from(rd::<u8>(
            veh, 0x3D1,
        ))))
    } else {
        x(1.0)
    };
    let step = x(g::<f32>(0xB7CB5C).read())
        .fmul_m(f32::from_bits(0x3C23_D70A))
        .to_f32();
    let mult = rd::<f32>(veh, 0x3D4);
    let mut delta = x(mult).fsub(target);
    if !delta.fcom(x(0.0)).test_parity(0x05) {
        delta = delta.fchs();
    }
    let new = if lt_k(delta, step) {
        target
    } else if lt_k(target, mult) {
        x(mult).fsub_m(step)
    } else {
        x(step).fadd_m(mult)
    };
    wr(veh, 0x3D4, new.to_f32());
    if law {
        police_gang_area_cruise_speed(veh);
    }
}

/// `FindPlayerPed(-1)->GetWantedLevel()` (0x41BE60 → 0x156D300: the player data's wanted +0x2C).
fn player_wanted_level() -> i32 {
    let pd = rd::<GPtr<u8>>(crate::common::find_player_ped(-1), 0x480);
    rd::<i32>(rd::<GPtr<u8>>(pd, 0), 0x2C)
}

/// The law-enforcer tail of `UpdateCarAI` (0x420553..0x42065A): on frames with `(frameCounter & 0xF) == 0xE`, a
/// cruising (mission 1) automobile (+0x594 == 0) with status 2 or 3 in a gang area (`CPopCycle::
/// m_bCurrentZoneIsGangArea` 0xC0BC5C) counts the peds of types 7..=17 (gangs, dealers) closer than 10 (the pool
/// walked from the last slot down) and cruises at `max(10 - count, 1)`.
fn police_gang_area_cruise_speed(veh: GPtr<u8>) {
    if g::<u32>(0xB7CB4C).read() & 0xF != 0xE
        || !matches!(rd::<u8>(veh, 0x36) >> 3, 2 | 3)
        || rd::<u8>(veh, 0x3BA) != 1
        || rd::<u32>(veh, 0x594) != 0
        || g::<u8>(0xC0BC5C).read() == 0
    {
        return;
    }
    let pool = g::<GPtr<u8>>(0xB74490).read();
    let mut i = rd::<i32>(pool, 8);
    let mut count = 0i32;
    let mut tmp = [0f32; 3];
    while i != 0 {
        let flag = rd::<i8>(rd::<GPtr<u8>>(pool, 4), (i - 1) as isize);
        i -= 1;
        if flag < 0 {
            continue;
        }
        let ped = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0x7C4);
        if ped.is_null() || !(7..=0x11).contains(&rd::<i32>(ped, 0x598)) {
            continue;
        }
        let d = police_vector_sub(GPtr::from_raw(tmp.as_mut_ptr()), entity_pos(veh), entity_pos(ped));
        if !X87::from_f64_exact(police_vector_magnitude(d)).fcom(x(kf(0x85862C))).test_parity(0x05) {
            count += 1;
        }
    }
    wr::<u8>(veh, 0x3D0, (10 - count).max(1) as u8);
}

orig! {
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, hidden result pointer first)
    fn police_vector_sub(out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>) -> GPtr<f32> = cdecl 0x40FE60 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    fn police_vector_magnitude(v: GPtr<f32>) -> f64 = thiscall 0x4082C0 => unported;
}

orig! {
    /// `CCarCtrl::SlowCarOnRailsDownForTrafficAndLights(CVehicle*)` (0x434790)
    fn slow_car_on_rails_down_for_traffic_and_lights(v: GPtr<u8>) = cdecl 0x434790 => crate::carctrl2::slow_car_on_rails_down_for_traffic_and_lights;
    /// `CCarCtrl::PickNextNodeAccordingStrategy(CVehicle*)` (0x432B10)
    fn pick_next_node_according_strategy_call(v: GPtr<u8>) -> u8 = cdecl 0x432B10 => pick_next_node_according_strategy;
    /// `CCurves::CalcCurvePoint(const CVector*, const CVector*, const CVector*, const CVector*, float, int32,
    /// CVector*, CVector*)` (0x43C900)
    fn calc_curve_point(s: GPtr<f32>, e: GPtr<f32>, sd: GPtr<f32>, ed: GPtr<f32>, t: f32, ms: i32, out: GPtr<f32>, speed: GPtr<f32>) = cdecl 0x43C900 => unported;
    /// `CCarCtrl::DragCarToPoint(CVehicle*, CVector*)` (0x42EC90)
    fn drag_car_to_point(v: GPtr<u8>, p: GPtr<f32>) = cdecl 0x42EC90 => unported;
}

/// The jitter `((v & 7) - 3) * 0.009` / `(((v >> 3) & 7) - 3) * 0.009` of a seed + link sum.
fn jitter(v: i32, shift: u32) -> X87 {
    xi(((v >> shift) & 7) - 3).fmul_m(kf(0x859048))
}

/// `CCarCtrl::UpdateCarOnRails(CVehicle*)` (0x436540): `StopCarIfNodesAreInvalid` (its result unused: the
/// detoured `mov al, [esi+0x3DC]` at 0x401284 tests the stopped flag, which ends the update); temp actions 1 /
/// 24 stop the car (and end the action after its time), 12 stops it; otherwise the rails update: slow down
/// for traffic and lights, the next node once the curve time has passed (`(int64)now >= (int64)end`), and
/// unless the car switched to physics the curve point at `t = (now - start) / duration` between the two lane
/// points (lane offsets `(OneWayLaneOffset + lane) * 5.4`, +1.458 for BMX) with seed-jittered normalised
/// directions, z 15, `DragCarToPoint`, and the curve speed / 60 as move speed when the links differ.
#[hook(addr = 0x436540, conv = cdecl, name = "CCarCtrl::UpdateCarOnRails", expect = "83 EC 64 56 8B 74 24 6C")]
pub fn update_car_on_rails(veh: GPtr<u8>) {
    stop_car_if_nodes_are_invalid(veh);
    if rd::<u8>(veh, 0x3DC) & 1 != 0 {
        return;
    }
    let ap = veh.byte_add(0x390);
    let stop = || {
        for i in 0..3 {
            wr(veh, 0x44 + 4 * i, 0f32);
        }
        modify_speed(ap, 0.0);
    };
    match rd::<i8>(veh, 0x3BB) {
        1 | 24 => {
            stop();
            if now() > rd::<u32>(veh, 0x3BC) {
                wr::<u8>(veh, 0x3BB, 0);
                wr(veh, 0x3AC, now());
                wr(veh, 0x3B0, now());
            }
            return;
        }
        12 => {
            stop();
            return;
        }
        _ => {}
    }
    slow_car_on_rails_down_for_traffic_and_lights(veh);
    let end = rd::<i32>(veh, 0x39C).wrapping_add(rd::<i32>(veh, 0x3A0));
    let mut t_now = now();
    if i64::from(t_now) >= i64::from(end) {
        pick_next_node_according_strategy_call(veh);
        t_now = now();
    }
    if rd::<u8>(veh, 0x36) & 0xF8 == 0x18 {
        return;
    }
    let diff = t_now.wrapping_sub(rd::<u32>(veh, 0x39C)) as i32;
    let mut tv = xi(diff);
    if diff < 0 {
        tv = tv.fadd_m(kf(0x858C54));
    }
    let t = tv.fdiv(xi(rd::<i32>(veh, 0x3A0))).to_f32();
    let cur = rd::<u16>(veh, 0x3A4);
    let next = rd::<u16>(veh, 0x3A6);
    let (l1, l2) = (navi_link(cur), navi_link(next));
    let s1 = xi(i32::from(rd::<i8>(veh, 0x3B5)));
    let d1x = link_dir(l1, 8, s1).to_f32();
    let d1y = link_dir(l1, 9, s1).to_f32();
    let s2 = xi(i32::from(rd::<i8>(veh, 0x3B6)));
    let d2x = link_dir(l2, 8, s2).to_f32();
    let d2y = link_dir(l2, 9, s2).to_f32();
    let k54 = f32::from_bits(0x40AC_CCCD);
    let mut lo1 = X87::from_f64_exact(one_way_lane_offset(l1))
        .fadd(xi(i32::from(rd::<i8>(veh, 0x3B7))))
        .fmul_m(k54)
        .to_f32();
    let mut lo2 = X87::from_f64_exact(one_way_lane_offset(l2))
        .fadd(xi(i32::from(rd::<i8>(veh, 0x3B8))))
        .fmul_m(k54)
        .to_f32();
    if rd::<i32>(veh, 0x594) == 10 {
        lo1 = x(lo1).fadd_m(kf(0x859010)).to_f32();
        lo2 = x(lo2).fadd_m(kf(0x859010)).to_f32();
    }
    let seed = i32::from(rd::<u16>(veh, 0x20));
    let a = seed + i32::from(cur);
    let b = seed + i32::from(next);
    let mut start_dir = [
        jitter(a, 0).fadd_m(d1x).to_f32(),
        jitter(a, 3).fadd_m(d1y).to_f32(),
        0.0,
    ];
    let mut end_dir = [
        jitter(b, 0).fadd_m(d2x).to_f32(),
        jitter(b, 3).fadd_m(d2y).to_f32(),
        0.0,
    ];
    vector_normalise(lp(&mut start_dir));
    vector_normalise(lp(&mut end_dir));
    let mut end_p = [
        c8(l2, 0).fadd(x(lo2).fmul_m(d2y)).to_f32(),
        c8(l2, 2).fsub(x(lo2).fmul_m(d2x)).to_f32(),
        0.0,
    ];
    let mut start_p = [
        c8(l1, 0).fadd(x(lo1).fmul_m(d1y)).to_f32(),
        c8(l1, 2).fsub(x(lo1).fmul_m(d1x)).to_f32(),
        0.0,
    ];
    let mut coor = [0f32; 3];
    let mut speed = [0f32; 3];
    calc_curve_point(
        lp(&mut start_p),
        lp(&mut end_p),
        lp(&mut start_dir),
        lp(&mut end_dir),
        t,
        rd::<i32>(veh, 0x3A0),
        lp(&mut coor),
        lp(&mut speed),
    );
    coor[2] = 15.0;
    drag_car_to_point(veh, lp(&mut coor));
    for s in speed.iter_mut() {
        *s = x(*s).fmul_m(kf(0x859044)).to_f32();
    }
    if rd::<u16>(veh, 0x3A4) != rd::<u16>(veh, 0x3A6) {
        for (i, s) in speed.iter().enumerate() {
            wr(veh, 0x44 + 4 * i as isize, *s);
        }
    }
}

orig! {
    /// CRT `floor(double)` (0x8219F0)
    fn crt_floor(v: f64) -> f64 = cdecl 0x8219F0 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470)
    fn clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CCarCtrl::SlowCarDownForPedsSectorList(CPtrListDoubleLink*, CVehicle*, float, float, float, float, float*,
    /// float)` (0x425440)
    fn slow_car_down_for_peds_sector_list(list: GPtr<u8>, veh: GPtr<u8>, min_x: f32, min_y: f32, max_x: f32, max_y: f32, cur: GPtr<f32>, speed: f32) = cdecl 0x425440 => unported;
}

/// `_ftol(floor(v))` of the sector coordinate `v * 0.02 + 60` (the `GetSector` shape); the clamp test and the
/// value use the same number (the stored f32 equals the x87 value).
fn sector_of(v: f32) -> i32 {
    let t = x(v).fmul_m(kf(0x858B38)).fadd_m(kf(0x858B34));
    ftol32(X87::from_f64_exact(crt_floor(t.to_f64())))
}

/// `CCarCtrl::ScanForPedDanger(CVehicle*)` (0x42CE40): radius 44 for the player's vehicle, else 11; the
/// rectangle around the vehicle in sector coordinates (clamped to 0..119), a new scan code (or
/// `ClearScanCodes` and 1 at 0xFFFF), `SlowCarDownForPedsSectorList` for every repeat sector of the
/// rectangle (rows then columns, `& 15`) with the cruise speed; then +0x429 bit 5 is set and bit 1 of +0x3DB
/// is restored to its value on entry (kept by the original in its argument slot).
#[hook(addr = 0x42CE40, conv = cdecl, name = "CCarCtrl::ScanForPedDanger", expect = "83 EC 24 53 55 56 57")]
pub fn scan_for_ped_danger(veh: GPtr<u8>) {
    let r = if veh == find_player_vehicle(-1, 0) {
        kf(0x858FF8)
    } else {
        kf(0x858FBC)
    };
    let saved = (rd::<u8>(veh, 0x3DB) >> 1) & 1;
    let p = entity_pos(veh);
    let min_x = x(p.read()).fsub_m(r).to_f32();
    let max_x = x(r).fadd_m(p.read()).to_f32();
    let min_y = x(p.wrapping_index(1).read()).fsub_m(r).to_f32();
    let max_y = x(r).fadd_m(p.wrapping_index(1).read()).to_f32();
    let k = sector_of(min_x);
    let start_x = if k > 0 { sector_of(min_x) } else { 0 };
    let k = sector_of(min_y);
    let start_y = if k > 0 { sector_of(min_y) } else { 0 };
    let k = sector_of(max_x);
    let end_x = if k < 0x77 { sector_of(max_x) } else { 0x77 };
    let k = sector_of(max_y);
    let end_y = if k < 0x77 { sector_of(max_y) } else { 0x77 };
    let code = g::<u16>(0xB7CD78);
    if code.read() == 0xFFFF {
        clear_scan_codes();
        code.write(1);
    } else {
        code.write(code.read() + 1);
    }
    let mut cur = [xi(i32::from(rd::<u8>(veh, 0x3D0))).to_f32()];
    let mut y = start_y;
    while y <= end_y {
        let mut xs = start_x;
        while xs <= end_x {
            let speed = xi(i32::from(rd::<u8>(veh, 0x3D0))).to_f32();
            let idx = ((y & 0xF) << 4) + (xs & 0xF);
            let list = g::<u8>(0xB992BC + idx as usize * 12);
            slow_car_down_for_peds_sector_list(
                list,
                veh,
                min_x,
                min_y,
                max_x,
                max_y,
                GPtr::from_raw(cur.as_mut_ptr()),
                speed,
            );
            xs += 1;
        }
        y += 1;
    }
    wr(veh, 0x429, rd::<u8>(veh, 0x429) | 0x20);
    let bl = rd::<u8>(veh, 0x3DB);
    wr(veh, 0x3DB, bl ^ (((saved << 1) ^ bl) & 2));
}

orig! {
    /// `CCollision::DistToLine(const CVector*, const CVector*, const CVector*)` (0x417610, ST0)
    fn dist_to_line(a: GPtr<f32>, b: GPtr<f32>, p: GPtr<f32>) -> f64 = cdecl 0x417610 => unported;
}

/// `(a < b)` of two node addresses as unsigned (area, node) words.
fn addr_less(a: u32, b: u32) -> bool {
    let (aa, ba) = (a & 0xFFFF, b & 0xFFFF);
    aa < ba || (aa == ba && (a >> 16) < (b >> 16))
}

/// A node position as three stored f32 (`fild i16; fmul 0.125`).
fn node_pos(n: GPtr<u8>) -> [f32; 3] {
    [c8(n, 8).to_f32(), c8(n, 0xA).to_f32(), c8(n, 0xC).to_f32()]
}

/// `CCarCtrl::FindLinksToGoWithTheseNodes(CVehicle*)` (0x42B470): the forced route seed reseeds `srand`; the
/// next link (+0x3A6) is the current node's link to the next node, searched over a constant 12 entries (the
/// 13th entry is used when none matches, as the bytes); +0x3B6 = -1 when current < next; the current link
/// (+0x3A4) is the only link of a single-link node, else the link (other than the next node, in a loaded
/// area) whose segment is closest to the vehicle (`DistToLine`, strict less, first of equals), link 0 when
/// none; +0x3B5 = -1 when that link's node is below the current node.
#[hook(addr = 0x42B470, conv = cdecl, name = "CCarCtrl::FindLinksToGoWithTheseNodes", expect = "83 EC 1C 56 8B 74 24 24")]
pub fn find_links_to_go_with_these_nodes(veh: GPtr<u8>) {
    let seed = rd::<u16>(veh, 0x45E);
    if seed != 0 {
        sa_core::orig::at_site(0x42_B48D, || sa_core::crt::srand(u32::from(seed)));
    }
    let cur = rd::<u32>(veh, 0x390);
    let area = area_of(cur);
    let base = i32::from(rd::<i16>(path_node(cur), 0x10));
    let next = rd::<u32>(veh, 0x394);
    let mut i: i32 = 0;
    loop {
        if node_link_at(area, base.wrapping_add(i32::from(i as i16))) == next {
            break;
        }
        i += 1;
        if (i as i16) >= 0xC {
            break;
        }
    }
    wr(
        veh,
        0x3A6,
        navi_link_at(area, base.wrapping_add(i32::from(i as i16))),
    );
    wr::<i8>(
        veh,
        0x3B6,
        if addr_less(rd::<u32>(veh, 0x390), rd::<u32>(veh, 0x394)) {
            -1
        } else {
            1
        },
    );
    let num = |c: u32| i32::from(rd::<u8>(path_node(c), 0x18) & 0xF);
    let idx = if num(cur) == 1 {
        0
    } else {
        let mut best: i32 = -1;
        let mut best_dist = f32::from_bits(0x4974_23FE);
        let mut i = 0;
        if num(cur) != 0 {
            loop {
                let c = rd::<u32>(veh, 0x390);
                let l = node_link_at(
                    area_of(c),
                    i32::from(rd::<i16>(path_node(c), 0x10)).wrapping_add(i),
                );
                if l != rd::<u32>(veh, 0x394) && !path_nodes(area_of(l)).is_null() {
                    let mut a = node_pos(path_node(c));
                    let mut b = node_pos(path_node(l));
                    let d = X87::from_f64_exact(dist_to_line(lp(&mut a), lp(&mut b), entity_pos(veh)));
                    if lt_k(d, best_dist) {
                        best_dist = d.to_f32();
                        best = i;
                    }
                }
                i += 1;
                if i >= num(rd::<u32>(veh, 0x390)) {
                    break;
                }
            }
        }
        if best < 0 { 0 } else { best }
    };
    let c = rd::<u32>(veh, 0x390);
    let base = i32::from(rd::<i16>(path_node(c), 0x10));
    wr(veh, 0x3A4, navi_link_at(area_of(c), base.wrapping_add(idx)));
    let l = node_link_at(area_of(c), base.wrapping_add(idx));
    wr::<i8>(
        veh,
        0x3B5,
        if addr_less(l, rd::<u32>(veh, 0x390)) {
            -1
        } else {
            1
        },
    );
}

orig! {
    /// `CPathFind::FindNodeClosestToCoorsFavourDirection(CVector, uint8, float, float)` (0x44FCE0, thiscall,
    /// returns the out pointer)
    fn find_node_closest_to_coors_favour_direction(this: GPtr<u8>, out: GPtr<u32>, px: f32, py: f32, pz: f32, ty: u32, dx: f32, dy: f32) -> GPtr<u32> = thiscall 0x44FCE0 => crate::pathfind::find_node_closest_to_coors_favour_direction;
    /// `CCarCtrl::FindLinksToGoWithTheseNodes(CVehicle*)` (0x42B470)
    fn find_links_to_go_with_these_nodes_call(v: GPtr<u8>) = cdecl 0x42B470 => find_links_to_go_with_these_nodes;
}

/// `CCarCtrl::JoinCarWithRoadSystem(CVehicle*)` (0x42F5A0; entry detour `mov ebx, 0xFFFF` at 0x405881): clears
/// the node areas (+0x398, +0x390, +0x394), the links (+0x3A4, +0x3A8, +0x3A6) and the path node count; the
/// car node closest to the vehicle favouring its forward direction (the matrix is read without a NULL check);
/// its loaded neighbour at the smallest 2D distance (strict); the pair is ordered by the sign of
/// `(N - B) · dir` (dir = forward, x replaced by 1 when both are zero): negative → current N, next B; then
/// `FindLinksToGoWithTheseNodes` and lanes 0.
#[hook(addr = 0x42F5A0, conv = cdecl, name = "CCarCtrl::JoinCarWithRoadSystem", expect = "83 EC 54 53 55")]
pub fn join_car_with_road_system(veh: GPtr<u8>) {
    for off in [0x398, 0x390, 0x394, 0x3A4, 0x3A8, 0x3A6] {
        wr::<u16>(veh, off, 0xFFFF);
    }
    wr::<u16>(veh, 0x418, 0);
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let p = entity_pos(veh);
    let mut out = [0u32];
    let node = find_node_closest_to_coors_favour_direction(
        the_paths(),
        GPtr::from_raw(out.as_mut_ptr()),
        p.read(),
        p.wrapping_index(1).read(),
        p.wrapping_index(2).read(),
        0,
        rd::<f32>(m, 0x10),
        rd::<f32>(m, 0x14),
    )
    .read();
    if area_of(node) == 0xFFFF || path_nodes(area_of(node)).is_null() {
        return;
    }
    let n = path_node(node);
    let num = rd::<u8>(n, 0x18) & 0xF;
    let mut best_dist = f32::from_bits(0x4974_23FE);
    let mut best: u32 = 0xFFFF;
    let base = i32::from(rd::<i16>(n, 0x10));
    for k in 0..i32::from(num) {
        let l = node_link_at(area_of(node), base + k);
        if path_nodes(area_of(l)).is_null() {
            continue;
        }
        let ln = path_node(l);
        let dx = c8(ln, 8).fsub(c8(n, 8));
        let dy = c8(ln, 0xA).fsub(c8(n, 0xA));
        let d = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt();
        if lt_k(d, best_dist) {
            best_dist = d.to_f32();
            best = l;
        }
    }
    if best & 0xFFFF == 0xFFFF {
        return;
    }
    let fx = rd::<f32>(m, 0x10);
    let fy = rd::<f32>(m, 0x14);
    let dir_x = if ne(x(fx), x(0.0)) || ne(x(fy), x(0.0)) {
        x(fx)
    } else {
        x(1.0)
    };
    let b = path_node(best);
    let dot = c8(n, 8)
        .fsub(c8(b, 8))
        .fmul(dir_x)
        .fadd(c8(n, 0xA).fsub(c8(b, 0xA)).fmul_m(fy));
    let (cur, next) = if lt_k(dot, 0.0) {
        (node, best)
    } else {
        (best, node)
    };
    wr::<u16>(veh, 0x398, 0xFFFF);
    wr(veh, 0x390, cur);
    wr(veh, 0x394, next);
    find_links_to_go_with_these_nodes_call(veh);
    wr::<u8>(veh, 0x3B7, 0);
    wr::<u8>(veh, 0x3B8, 0);
}

/// `CCarCtrl::JoinCarWithRoadAccordingToMission(CVehicle*)` (0x432CB0): by mission (byte table 0x432D7C, jump
/// table 0x432D68): 0, 1, 7, 0xA, 0xB, 0x22, 0x2D, 0x2E, 0x31, 0x32 join the road system; 2..6, 0x15, 0x18..0x1A,
/// 0x2B, 0x2C, 0x40 go to the player's coordinates, 8, 9, 0xC..0xE, 0x21 to the destination (+0x3EC), 0xF..0x13,
/// 0x1B..0x20, 0x34..0x37, 0x3C, 0x3D, 0x41..0x44 to the target entity (+0x41C, when set), each through
/// `JoinCarWithRoadSystemGotoCoors(veh, pos, true, type == 5)`; other missions (and values above 0x44) nothing.
#[hook(addr = 0x432CB0, conv = cdecl, name = "CCarCtrl::JoinCarWithRoadAccordingToMission", expect = "83 EC 0C 56 8B 74 24 14")]
pub fn join_car_with_road_according_to_mission(veh: GPtr<u8>) {
    const T: [u8; 0x45] = [
        0, 0, 1, 1, 1, 1, 1, 0, 2, 2, 0, 0, 2, 2, 2, 3, 3, 3, 3, 3, 4, 1, 4, 4, 1, 1, 1, 3, 3, 3, 3, 3, 3, 2,
        0, 4, 4, 4, 4, 4, 4, 4, 4, 1, 1, 0, 0, 4, 4, 0, 0, 4, 3, 3, 3, 3, 4, 4, 4, 4, 3, 3, 4, 4, 1, 3, 3, 3,
        3,
    ];
    let m = rd::<i8>(veh, 0x3BA) as i32 as u32;
    if m > 0x44 {
        return;
    }
    let boat = u32::from(rd::<i32>(veh, 0x594) == 5);
    match T[m as usize] {
        0 => join_car_with_road_system_call(veh),
        1 => {
            let mut p = [0f32; 3];
            let pp = crate::common::find_player_coors(lp(&mut p), -1);
            join_car_with_road_system_goto_coors(veh, pp, 1, boat);
        }
        2 => {
            join_car_with_road_system_goto_coors(veh, veh.byte_add(0x3EC).cast(), 1, boat);
        }
        3 => {
            let t = rd::<GPtr<u8>>(veh, 0x41C);
            if !t.is_null() {
                join_car_with_road_system_goto_coors(veh, entity_pos(t), 1, boat);
            }
        }
        _ => {}
    }
}

orig! {
    /// `CPathFind::FindXRegionForCoors(float)` (0x44D890, thiscall)
    fn find_x_region_for_coors(this: GPtr<u8>, v: f32) -> i32 = thiscall 0x44D890 => unported;
    /// `CPathFind::FindYRegionForCoors(float)` (0x44D8C0, thiscall)
    fn find_y_region_for_coors(this: GPtr<u8>, v: f32) -> i32 = thiscall 0x44D8C0 => unported;
    /// `CPathFind::FindXCoorsForRegion(int32)` (0x44D8F0, thiscall, ST0)
    fn find_x_coors_for_region(this: GPtr<u8>, r: i32) -> f64 = thiscall 0x44D8F0 => unported;
    /// `CPathFind::FindYCoorsForRegion(int32)` (0x44D910, thiscall, ST0)
    fn find_y_coors_for_region(this: GPtr<u8>, r: i32) -> f64 = thiscall 0x44D910 => unported;
}

/// `CCarCtrl::FindNodesThisCarIsNearestTo(CVehicle*, CNodeAddress* outNode, CNodeAddress* outLinkNode)`
/// (0x42BD20): over the path areas around the vehicle (its region, plus the neighbour when within 200 of the
/// low edge or beyond 550, clamped to 0..7; x outer, y inner), every vehicle node within 150 (3D) and each of
/// its loaded links scores `(1 - dir · forward) * 5 + DistToLine(node, link, pos)` (z scaled by 3 for the
/// distance, `dir` the normalised node → link vector); the lowest (strict) wins: `outNode = (area, index)`,
/// `outLinkNode = link`. Without a candidate both carry area 0xFFFF and the node words of the original's
/// uninitialised locals (`[entry ESP - 0x8A]`, `[entry ESP - 0x86]`), read from the footprint snapshot.
#[hook(addr = 0x42BD20, conv = cdecl, name = "CCarCtrl::FindNodesThisCarIsNearestTo", expect = "81 EC 9C 00 00 00", footprint = 0x100)]
pub fn find_nodes_this_car_is_nearest_to(veh: GPtr<u8>, out_node: GPtr<u32>, out_link: GPtr<u32>) {
    let res16 = |off: i32| {
        u32::from(sa_core::abi::footprint_residue_u8(off, 0x100))
            | (u32::from(sa_core::abi::footprint_residue_u8(off + 1, 0x100)) << 8)
    };
    let mut best = 0xFFFF | (res16(-0x8A) << 16);
    let mut best_link = 0xFFFF | (res16(-0x86) << 16);
    let mut best_score = f32::from_bits(0x47C3_4FF3);
    let paths = the_paths();
    let xr = find_x_region_for_coors(paths, entity_pos(veh).read());
    let yr = find_y_region_for_coors(paths, entity_pos(veh).wrapping_index(1).read());
    let fx = x(entity_pos(veh).read())
        .fsub(X87::from_f64_exact(find_x_coors_for_region(paths, xr)))
        .to_f32();
    let fy = x(entity_pos(veh).wrapping_index(1).read())
        .fsub(X87::from_f64_exact(find_y_coors_for_region(paths, yr)));
    let k200 = kf(0x858A48);
    let k550 = kf(0x858FF0);
    let x0 = if lt_k(x(fx), k200) { xr - 1 } else { xr }.max(0);
    let y0 = if lt_k(fy, k200) { yr - 1 } else { yr }.max(0);
    let x1 = if !x(fx).fcom(x(k550)).test_any(0x41) {
        xr + 1
    } else {
        xr
    }
    .min(7);
    let y1 = if !fy.fcom(x(k550)).test_any(0x41) {
        yr + 1
    } else {
        yr
    }
    .min(7);
    for xa in x0..=x1 {
        for ya in y0..=y1 {
            let area = (xa + 8 * ya) as u32;
            let nodes = path_nodes(area);
            if nodes.is_null() {
                continue;
            }
            let count = g::<i32>((0x970114 + area * 4) as usize).read();
            let mut i = 0;
            while i < count {
                let n = path_nodes(area).byte_add(i as isize * 0x1C);
                let p = entity_pos(veh);
                let nz = c8(n, 0xC).to_f32();
                let dx = c8(n, 8).fsub_m(p.read()).to_f32();
                let dy = c8(n, 0xA).fsub_m(p.wrapping_index(1).read());
                let dz = x(nz).fsub_m(p.wrapping_index(2).read());
                let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(x(dx).fmul_m(dx)).fsqrt();
                if lt_k(d, kf(0x858A28)) {
                    let links = i32::from(rd::<u8>(n, 0x18) & 0xF);
                    for k in 0..links {
                        let l = node_link_at(area, i32::from(rd::<i16>(n, 0x10)) + k);
                        if path_nodes(area_of(l)).is_null() {
                            continue;
                        }
                        let ln = path_node(l);
                        let three = kf(0x858B3C);
                        let mut a = node_pos(n);
                        let mut b = node_pos(ln);
                        let p = entity_pos(veh);
                        let mut vp = [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()];
                        a[2] = x(a[2]).fmul_m(three).to_f32();
                        b[2] = x(b[2]).fmul_m(three).to_f32();
                        vp[2] = x(vp[2]).fmul_m(three).to_f32();
                        let dl =
                            X87::from_f64_exact(dist_to_line(lp(&mut a), lp(&mut b), lp(&mut vp))).to_f32();
                        let mut dir = [
                            c8(ln, 8).fsub(c8(n, 8)).to_f32(),
                            c8(ln, 0xA).fsub(c8(n, 0xA)).to_f32(),
                            x(c8(ln, 0xC).to_f32()).fsub_m(c8(n, 0xC).to_f32()).to_f32(),
                        ];
                        vector_normalise(lp(&mut dir));
                        let m = rd::<GPtr<u8>>(veh, 0x14);
                        let dot = x(dir[2])
                            .fmul_m(rd::<f32>(m, 0x18))
                            .fadd(x(dir[1]).fmul_m(rd::<f32>(m, 0x14)))
                            .fadd(x(dir[0]).fmul_m(rd::<f32>(m, 0x10)));
                        let score = dot.fsubr_m(1.0).fmul_m(kf(0x858C80)).fadd_m(dl);
                        if lt_k(score, best_score) {
                            best_score = score.to_f32();
                            best = (area & 0xFFFF) | ((i as u32 & 0xFFFF) << 16);
                            best_link = l;
                        }
                    }
                }
                i += 1;
            }
        }
    }
    out_node.write(best);
    out_link.write(best_link);
}

orig! {
    /// `CCarCtrl::FindNodesThisCarIsNearestTo(CVehicle*, CNodeAddress*, CNodeAddress*)` (0x42BD20)
    fn find_nodes_this_car_is_nearest_to_call(v: GPtr<u8>, a: GPtr<u32>, b: GPtr<u32>) = cdecl 0x42BD20 => unported;
}

/// `CCarCtrl::ReconsiderRoute(CVehicle*)` (0x42FC40): once per 2 s per vehicle (`(time + seed) / 2000`
/// changed); for the far-away chase / go-to missions (2, 4, 8, 0xF, 0x11, 0x2B, 0x34, 0x36, 0x3D; any other
/// mission clears the counter +0x3C5) the nodes the car is nearest to; when found and not already its current
/// / next / previous pair, and after more than four such tries, a path search from the second node to the
/// target (player, destination or target entity) under 90000 with at least two nodes makes the pair the
/// current / next nodes, refills the path list (+0x3F8, eight nodes, forbidding the new current node) and
/// drops its first node; the counter is cleared on every exit but the throttle, the no-node and the
/// fourth-try returns.
#[hook(addr = 0x42FC40, conv = cdecl, name = "CCarCtrl::ReconsiderRoute", expect = "A1 78 CB B7 00")]
pub fn reconsider_route(veh: GPtr<u8>) {
    let s = u32::from(rd::<u16>(veh, 0x20));
    if g::<u32>(0xB7CB78).read().wrapping_add(s) / 2000 == now().wrapping_add(s) / 2000 {
        return;
    }
    let reset = || wr::<u8>(veh, 0x3C5, 0);
    let m = rd::<i8>(veh, 0x3BA);
    if !matches!(m, 2 | 4 | 8 | 0xF | 0x11 | 0x2B | 0x34 | 0x36 | 0x3D) {
        return reset();
    }
    let mut n1 = [0xFFFFu32];
    let mut n2 = [0xFFFFu32];
    find_nodes_this_car_is_nearest_to_call(
        veh,
        GPtr::from_raw(n1.as_mut_ptr()),
        GPtr::from_raw(n2.as_mut_ptr()),
    );
    let (n1, n2) = (n1[0], n2[0]);
    if n1 & 0xFFFF == 0xFFFF {
        return;
    }
    let (cur, next, prev) = (
        rd::<u32>(veh, 0x390),
        rd::<u32>(veh, 0x394),
        rd::<u32>(veh, 0x398),
    );
    if (cur == n1 && next == n2)
        || (cur == n2 && next == n1)
        || (prev == n1 && cur == n2)
        || (prev == n2 && cur == n1)
        || cur == n2
        || prev == n2
    {
        return reset();
    }
    let c = rd::<u8>(veh, 0x3C5).wrapping_add(1);
    wr(veh, 0x3C5, c);
    if c <= 4 {
        return;
    }
    let target: [f32; 3] = match m {
        2 | 4 | 0x2B => {
            let mut p = [0f32; 3];
            let pp = crate::common::find_player_coors(lp(&mut p), -1);
            [
                pp.read(),
                pp.wrapping_index(1).read(),
                pp.wrapping_index(2).read(),
            ]
        }
        8 => [
            rd::<f32>(veh, 0x3EC),
            rd::<f32>(veh, 0x3F0),
            rd::<f32>(veh, 0x3F4),
        ],
        _ => {
            let p = entity_pos(rd::<GPtr<u8>>(veh, 0x41C));
            [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()]
        }
    };
    let same_lane = u32::from((rd::<u8>(veh, 0x3DB) >> 6) & 1);
    let water = u32::from(rd::<i16>(veh, 0x22) == 0x21B);
    let mut count = [0i16];
    let mut dist = [0f32];
    let v = entity_pos(veh);
    do_path_search(
        the_paths(),
        0,
        v.read(),
        v.wrapping_index(1).read(),
        v.wrapping_index(2).read(),
        n2,
        target[0],
        target[1],
        target[2],
        GPtr::NULL,
        GPtr::from_raw(count.as_mut_ptr()),
        0,
        GPtr::from_raw(dist.as_mut_ptr()),
        f32::from_bits(0x4974_23FE),
        GPtr::NULL,
        f32::from_bits(0x4974_23FE),
        same_lane,
        rd::<u32>(veh, 0x390),
        water,
        0,
    );
    if !lt_k(x(dist[0]), kf(0x85900C)) || count[0] < 2 {
        return reset();
    }
    wr(veh, 0x390, n1);
    wr(veh, 0x394, n2);
    find_links_to_go_with_these_nodes_call(veh);
    let v = entity_pos(veh);
    do_path_search(
        the_paths(),
        0,
        v.read(),
        v.wrapping_index(1).read(),
        v.wrapping_index(2).read(),
        n2,
        target[0],
        target[1],
        target[2],
        veh.byte_add(0x3F8).cast(),
        veh.byte_add(0x418).cast(),
        8,
        GPtr::NULL,
        f32::from_bits(0x4974_23FE),
        GPtr::NULL,
        f32::from_bits(0x4974_23FE),
        u32::from((rd::<u8>(veh, 0x3DB) >> 6) & 1),
        rd::<u32>(veh, 0x390),
        u32::from(rd::<i16>(veh, 0x22) == 0x21B),
        0,
    );
    remove_one_path_node(veh.byte_add(0x390));
    reset();
}

pub const HOOKS: &[HookReg] = &[
    HOOK_RECONSIDER_ROUTE,
    HOOK_FIND_NODES_THIS_CAR_IS_NEAREST_TO,
    HOOK_JOIN_CAR_WITH_ROAD_ACCORDING_TO_MISSION,
    HOOK_JOIN_CAR_WITH_ROAD_SYSTEM,
    HOOK_FIND_LINKS_TO_GO_WITH_THESE_NODES,
    HOOK_SCAN_FOR_PED_DANGER,
    HOOK_UPDATE_CAR_ON_RAILS,
    HOOK_UPDATE_CAR_AI,
    HOOK_ADD_POLICE_CAR_OCCUPANTS,
    HOOK_MAKE_WAY_FOR_CAR_WITH_SIREN,
    HOOK_MELLOW_OUT_CHASE_SPEED,
    HOOK_ENTITIES_GO_HEAD_ON,
    HOOK_BACK_TO_CRUISING_IF_NO_WANTED_LEVEL,
    HOOK_GET_CAR_TO_GO_TO_COORS,
    HOOK_GET_CAR_TO_GO_TO_COORS_STRAIGHT_LINE,
    HOOK_GET_CAR_TO_GO_TO_COORS_RACING,
    HOOK_GET_CAR_TO_PARK_AT_COORS,
    HOOK_PICK_NEXT_NODE_TO_CHASE_CAR,
    HOOK_FIND_PATH_DIRECTION,
    HOOK_IS_THIS_AN_APPROPRIATE_NODE,
    HOOK_PICK_NEXT_NODE_RANDOMLY,
    HOOK_PICK_NEXT_NODE_ACCORDING_STRATEGY,
];
