//! CCarCtrl car generation and script car helpers (PORT-POP-3, helper B). Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/POP/*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, jne_mask, jp5, model_info, rand_at, rand_frac, rd, wr, x, xi};

/// Globals owned by this module: none.
pub const GLOBALS: &[GlobalDesc] = &[];

orig! {
    /// `CVehicle::operator new(uint32)` (0x6E2D50)
    fn vehicle_new(size: u32) -> GPtr<u8> = cdecl 0x6E2D50 => unported;
    /// `CMonsterTruck::CMonsterTruck(int32, uint8)` (0x6C8D60, `ret 8`)
    fn monster_truck_ctor(this: GPtr<u8>, mi: i32, created_by: u32) -> GPtr<u8> = thiscall 0x6C8D60 => unported;
    /// `CQuadBike::CQuadBike(int32, uint8)` (0x6CE370, `ret 8`)
    fn quad_bike_ctor(this: GPtr<u8>, mi: i32, created_by: u32) -> GPtr<u8> = thiscall 0x6CE370 => unported;
    /// `CHeli::CHeli(int32, uint8)` (0x6C4190, `ret 8`)
    fn heli_ctor(this: GPtr<u8>, mi: i32, created_by: u32) -> GPtr<u8> = thiscall 0x6C4190 => unported;
    /// `CPlane::CPlane(int32, uint8)` (0x6C8E20, `ret 8`)
    fn plane_ctor(this: GPtr<u8>, mi: i32, created_by: u32) -> GPtr<u8> = thiscall 0x6C8E20 => unported;
    /// `CBoat::CBoat(int32, uint8)` (0x6F2940, `ret 8`)
    fn boat_ctor(this: GPtr<u8>, mi: i32, created_by: u32) -> GPtr<u8> = thiscall 0x6F2940 => unported;
    /// `CTrain::CTrain(int32, uint8)` (0x6F6030, `ret 8`)
    fn train_ctor(this: GPtr<u8>, mi: i32, created_by: u32) -> GPtr<u8> = thiscall 0x6F6030 => unported;
    /// `CBike::CBike(int32, uint8)` (0x6BF430, `ret 8`)
    fn bike_ctor(this: GPtr<u8>, mi: i32, created_by: u32) -> GPtr<u8> = thiscall 0x6BF430 => unported;
    /// `CBmx::CBmx(int32, uint8)` (0x6BF820, `ret 8`)
    fn bmx_ctor(this: GPtr<u8>, mi: i32, created_by: u32) -> GPtr<u8> = thiscall 0x6BF820 => unported;
    /// `CTrailer::CTrailer(int32, uint8)` (0x6D03A0, `ret 8`)
    fn trailer_ctor(this: GPtr<u8>, mi: i32, created_by: u32) -> GPtr<u8> = thiscall 0x6D03A0 => unported;
    /// `CAutomobile::CAutomobile(int32, uint8, bool)` (0x6B0A90, `ret 0xC`)
    fn automobile_ctor(this: GPtr<u8>, mi: i32, created_by: u32, setup: u32) -> GPtr<u8> = thiscall 0x6B0A90 => unported;
    /// `CGangWars::GangWarFightingGoingOn()` (0x443AC0)
    fn gang_war_fighting_going_on() -> u8 = cdecl 0x443AC0 => crate::gangs::gang_war_fighting_going_on;
    /// `CCarCtrl::ChoosePoliceCarModel(uint32)` (0x421980)
    fn choose_police_car_model(ignore: u32) -> i32 = cdecl 0x421980 => crate::carctrl2::choose_police_car_model;
    /// `CLoadedCarGroup::PickRandomCar(bool, bool)` (0x611C50, `ret 8`)
    fn pick_random_car(this: GPtr<u8>, a: u32, b: u32) -> i32 = thiscall 0x611C50 => unported;
    /// `CPopulation::PickGangCar(int32)` (0x614490)
    fn pick_gang_car(gang: i32) -> i32 = cdecl 0x614490 => crate::population::pick_gang_car;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn random_int_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CPathFind::Find2NodesForCarCreation(CVector, CNodeAddress*, CNodeAddress*, bool)` (0x452090, `ret 0x18`)
    fn find_2_nodes_for_car_creation(this: GPtr<u8>, px: f32, py: f32, pz: f32, a: GPtr<u32>, b: GPtr<u32>, flag: u32) = thiscall 0x452090 => unported;
    /// `CPathFind::FindNodeClosestToCoors(CNodeAddress*, CVector, ...)` (0x44F460, `ret 0x2C`)
    fn find_node_closest_to_coors(this: GPtr<u8>, out: GPtr<u32>, px: f32, py: f32, pz: f32, node_type: u32, max_dist: f32, low_traffic: u32, unused: u32, unkn: u32, boats: u32, ignore_interior: u32) -> GPtr<u32> = thiscall 0x44F460 => crate::pathfind::find_node_closest_to_coors;
    /// `CompressedVector2D` → `CVector` (0x420A60, `ret 4`)
    fn uncompress(v: GPtr<i16>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x420A60 => crate::pathfind::compressed_vector2d_uncompress;
    /// `CPathNode::GetPosition(CVector*)` (0x420A10, `ret 4`)
    fn get_position(node: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x420A10 => crate::pathfind::get_position;
    /// `CCamera::IsSphereVisible(const CVector&, float)` (0x420D40, `ret 8`)
    fn camera_is_sphere_visible(cam: GPtr<u8>, c: GPtr<f32>, r: f32) -> u8 = thiscall 0x420D40 => unported;
    /// `CPathFind::ThisNodeWillLeadIntoADeadEnd(CPathNode*, CPathNode*)` (0x44D310, `ret 8`)
    fn this_node_will_lead_into_a_dead_end(this: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> u8 = thiscall 0x44D310 => crate::pathfind::this_node_will_lead_into_a_dead_end;
    /// `eh vector constructor iterator(void*, uint32 size, int32 n, ctor)` (0x403D60, stdcall)
    fn eh_vector_ctor_iter(arr: GPtr<u8>, size: u32, n: u32, ctor: u32) = stdcall 0x403D60 => unported;
    /// CRT `sprintf` (0x821BB5) with four int arguments
    fn sprintf4(buf: GPtr<u8>, fmt: GPtr<u8>, a: u32, b: u32, c: u32, d: u32) -> i32 = cdecl 0x821BB5 => unported;
}

/// `ThePaths` (0x96F050).
fn the_paths() -> GPtr<u8> {
    g(0x96F050)
}

/// `ThePaths.m_pPathNodes[area]` (0x96F854).
fn path_nodes(area: u32) -> GPtr<u8> {
    g::<GPtr<u8>>((0x96F854 + area * 4) as usize).read()
}

/// `node(A)`: `m_pPathNodes[A.area] + A.node * 0x1C` for a node address dword (area low word, node high word).
fn node_of(a: u32) -> GPtr<u8> {
    path_nodes(a & 0xFFFF).byte_add(((a >> 16) * 0x1C) as isize)
}

/// `(float)(int16)node[off] * 0.125f` (`fild` + `fmul [0x858C48]`).
fn ncoord(n: GPtr<u8>, off: isize) -> X87 {
    xi(i32::from(rd::<i16>(n, off))).fmul_m(0.125)
}

/// `|v|` as `fcom [0.0]; test ah, 5; jp skip; fchs`.
fn abs_fchs(v: X87) -> X87 {
    if jp5(v, x(0.0)) { v } else { v.fchs() }
}

/// `CStreaming::ms_aInfoForModel[mi].m_nLoadState == LOADSTATE_LOADED` (0x8E4CC0 + 0x10, stride 0x14).
fn model_loaded(mi: u32) -> bool {
    g::<u8>((0x8E4CD0 + mi * 20) as usize).read() == 1
}

/// `CCarCtrl::GetNewVehicleDependingOnCarModel(int32 mi, uint8 createdBy)` (0x421440, entry `nop; jmp`
/// into an EH-frame prologue at 0x401438): `operator new` + the constructor chosen by the model info's
/// vehicle type (+0x3C; 1 monster truck, 2 quad, 3 heli, 4 plane, 5 boat, 6 train, 9 bike, 10 BMX, 11
/// trailer, anything else `CAutomobile(mi, createdBy, true)`). Bikes and BMXs get bit 4 of +0x614; when
/// their allocation fails the original writes it through a null pointer (`xor eax, eax; or [eax+0x614]`,
/// kept: the port writes to 0x614 too). Other failed allocations return null. The EH frame only unwinds
/// a throwing constructor (none does).
#[hook(addr = 0x421440, conv = cdecl, name = "CCarCtrl::GetNewVehicleDependingOnCarModel", expect = "90 E9 F2 FF FD FF")]
pub fn get_new_vehicle_depending_on_car_model(mi: i32, created_by: u32) -> GPtr<u8> {
    type Ctor = fn(GPtr<u8>, i32, u32) -> GPtr<u8>;
    let ty = model_info(mi).byte_add(0x3C).cast::<u32>().read();
    let (size, ctor, bike): (u32, Option<Ctor>, bool) = match ty {
        1 => (0x99C, Some(monster_truck_ctor), false),
        2 => (0x9BC, Some(quad_bike_ctor), false),
        3 => (0xA18, Some(heli_ctor), false),
        4 => (0xA04, Some(plane_ctor), false),
        5 => (0x7E8, Some(boat_ctor), false),
        6 => (0x6AC, Some(train_ctor), false),
        9 => (0x814, Some(bike_ctor), true),
        10 => (0x838, Some(bmx_ctor), true),
        11 => (0x9F4, Some(trailer_ctor), false),
        _ => (0x988, None, false),
    };
    let mem = vehicle_new(size);
    if mem.is_null() {
        if bike {
            let p = GPtr::<u8>::from_addr(0x614);
            p.write(p.read() | 0x10);
        }
        return GPtr::NULL;
    }
    match ctor {
        Some(c) => {
            let v = c(mem, mi, created_by);
            if bike {
                wr::<u8>(v, 0x614, v.byte_add(0x614).read() | 0x10);
            }
            v
        }
        None => automobile_ctor(mem, mi, created_by, 1),
    }
}

/// `CCarCtrl::ChooseModel(int32* zoneType)` (0x424CE0): picks the class of a random car from the four
/// `CPopCycle` car-type percentages (0xC0BC30..0xC0BC3C, summed and `_ftol`'d):
/// - with 0x96917A set, `GetRandomNumberInRange(0, 100) != 0` (inlined) gives up;
/// - `r = rand() * 3.0518509e-05` (stored as f32) against the cumulative fractions `p3/total`,
///   `(p2+p3)/total`, `(p1+p2+p3)/total` (`fcomp; test ah, 0x41`): above the first (and 0x96915B clear) →
///   *zoneType = 0x19 and the model at 0xC0F240 when loaded; above the second (or 0x96915B set) → a gang
///   car unless 0xC0FCB2: gang `i` by the cumulative densities at `*(uint8**)0xC0BC68` (or
///   `GetRandomNumberInRange(0, 9)` with 0x96915B), *zoneType = i + 14, `PickGangCar(i)` must be >= 0, then
///   up to 10 draws `rand() * 23 / 32768` from the gang's model table 0xC0F074 (stride 0x17 words; draws of
///   0x7D0 retried without counting) until a loaded model; above the third → unless a gang war is
///   fighting or 0xC0FCB4, *zoneType = 0xD and the tail call `ChoosePoliceCarModel(0)` (the original
///   writes 0 into its own argument slot first); else *zoneType = 0 and the model at 0xA4448C when it is
///   not -1, otherwise `m_LoadedAppropriateCars.PickRandomCar(true, false)`. -1 for nothing.
#[hook(addr = 0x424CE0, conv = cdecl, name = "CCarCtrl::ChooseModel", expect = "83 EC 08 D9 05 30 BC C0 00")]
pub fn choose_model(zone_type: GPtr<i32>) -> i32 {
    let p = |a: usize| g::<f32>(a).read();
    let total = ftol32(
        x(p(0xC0BC30))
            .fadd_m(p(0xC0BC34))
            .fadd_m(p(0xC0BC38))
            .fadd_m(p(0xC0BC3C)),
    );
    if total <= 0 {
        return -1;
    }
    if g::<u8>(0x96917A).read() != 0 && ftol32(rand_frac(rand_at(0x424D1A)).fmul_m(100.0)) != 0 {
        return -1;
    }
    let r = xi(rand_at(0x424D41)).fmul_m(f32::from_bits(0x3800_0100)).to_f32();
    let gang_flag = g::<u8>(0x96915B).read() != 0;
    let tot = xi(total).to_f32();
    let le = |v: X87| jne_mask(v, x(r), 0x41);
    let c3 = x(p(0xC0BC3C)).fdiv_m(tot);
    if !le(c3) && !gang_flag {
        zone_type.write(0x19);
        let m = u32::from(g::<u16>(0xC0F240).read());
        return if model_loaded(m) { m as i32 } else { -1 };
    }
    let c23 = x(p(0xC0BC38)).fadd_m(p(0xC0BC3C)).fdiv_m(tot);
    if !le(c23) || gang_flag {
        // 0x424E52: gang car
        if g::<u8>(0xC0FCB2).read() != 0 {
            return -1;
        }
        let dens = g::<GPtr<u8>>(0xC0BC68).read();
        let sum: i32 = (0..10).map(|i| i32::from(dens.byte_add(i).read())).sum();
        if sum <= 0 {
            return -1;
        }
        let mut v = ftol32(rand_frac(rand_at(0x424EB9)).fmul(xi(sum)));
        let mut gang: i32 = 0;
        loop {
            let d = i32::from(dens.byte_add(gang as isize).read());
            if d >= v {
                break;
            }
            v -= d;
            gang += 1;
        }
        if gang_flag {
            gang = random_int_in_range(0, 9);
        }
        zone_type.write(gang + 14);
        if pick_gang_car(gang) < 0 {
            return -1;
        }
        let base = gang * 0x17;
        let mut tries = 0;
        loop {
            let i = ftol32(rand_frac(rand_at(0x424F2D)).fmul_m(23.0));
            let m = g::<u16>((0xC0F074 + (base + i) * 2) as usize).read();
            if m == 0x7D0 {
                continue;
            }
            if model_loaded(u32::from(m)) {
                return i32::from(m);
            }
            tries += 1;
            if tries >= 10 {
                return -1;
            }
        }
    }
    let c123 = x(p(0xC0BC34)).fadd_m(p(0xC0BC38)).fadd_m(p(0xC0BC3C)).fdiv_m(tot);
    if !le(c123) {
        if gang_war_fighting_going_on() != 0 || g::<u8>(0xC0FCB4).read() != 0 {
            return -1;
        }
        zone_type.write(0xD);
        return choose_police_car_model(0);
    }
    zone_type.write(0);
    let m = g::<i32>(0xA4448C).read();
    if m != -1 {
        return m;
    }
    pick_random_car(g(0xC0E9F8), 1, 0)
}

/// `CCarCtrl::GenerateCarCreationCoors2(CVector pos, float dirX, float dirY, float dotLimit, bool wantAhead,
/// float range1, float range2, CVector* outCoors, CNodeAddress* outNode1, CNodeAddress* outNode2, float* outFrac,
/// bool restrict, bool noRandomStart)` (0x424210, AL; spec CCarCtrl.md, checked against the bytes): the four
/// function-local node statics (guard 0x969108) are refreshed by `Find2NodesForCarCreation` every 5 s or after
/// 10 m (also recomputing `m_bMoreCarsAndFewerPeds` 0xC0FCB5); a start node (1 in 16: `FindNodeClosestToCoors`
/// and `range1 *= 1.5`); a walk over at most 30 nodes / 230 m along random links (`SequenceElements` /
/// `SequenceRandomOffset` / `bSequenceOtherWay` globals) looking for a link crossing the `range1` circle
/// in view (2D node positions; `je` at 0x424855 goes to test B when NOT visible, the spec has this reversed),
/// else the `range2` circle (3D positions) out of view, not too steep
/// (`L * 0.5 < |dz|` rejects); the orientation is random (`rand() & 8`), dead ends are rejected when
/// `restrict`, and the direction test keeps `dot > dotLimit` (wantAhead) or `dot <= dotLimit`. Success
/// prints the "tell Obbe" debug string into gString (kept). x87 sequence and f32 stores as the bytes.
/// The original also writes `range1 * 1.5` into its own argument slot (callers do not read it back).
/// A node with no links divides by zero in `rand() % 0` as the original (Rust panics).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x424210, conv = cdecl, name = "CCarCtrl::GenerateCarCreationCoors2", expect = "81 EC 78 01 00 00")]
pub fn generate_car_creation_coors2(
    px: f32,
    py: f32,
    pz: f32,
    dir_x: f32,
    dir_y: f32,
    dot_limit: f32,
    want_ahead: u8,
    range1: f32,
    range2: f32,
    out: GPtr<f32>,
    out_n1: GPtr<u32>,
    out_n2: GPtr<u32>,
    out_frac: GPtr<f32>,
    restrict: u8,
    no_random_start: u8,
) -> u8 {
    // 0x424216: the function-local statics
    let guard = g::<u32>(0x969108);
    for (bit, a) in [
        (1u32, 0x969104usize),
        (2, 0x969100),
        (4, 0x9690FC),
        (8, 0x9690F8),
        (0x10, 0),
    ] {
        let v = guard.read();
        if v & bit == 0 {
            guard.write(v | bit);
            if a != 0 {
                g::<u16>(a).write(0xFFFF);
            }
        }
    }
    // 0x424284: refresh test
    let dx = x(px).fsub_m(g::<f32>(0x9690EC).read());
    let dy = x(py).fsub_m(g::<f32>(0x9690F0).read());
    let d = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt();
    let now = || g::<u32>(0xB7CB84).read();
    if !jne_mask(d, x(10.0), 0x41) || now() > g::<u32>(0x9690E8).read() {
        find_2_nodes_for_car_creation(the_paths(), px, py, pz, g(0x969104), g(0x969100), 1);
        find_2_nodes_for_car_creation(the_paths(), px, py, pz, g(0x9690FC), g(0x9690F8), 0);
        g::<u32>(0x9690E8).write(now().wrapping_add(5000));
        g::<f32>(0x9690EC).write(px);
        g::<f32>(0x9690F0).write(py);
        g::<f32>(0x9690F4).write(pz);
        let a = u32::from(g::<u16>(0x969104).read());
        if a != 0xFFFF && !path_nodes(a).is_null() {
            let n = path_nodes(a).byte_add((u32::from(g::<u16>(0x969106).read()) * 0x1C) as isize);
            let navi_links = g::<GPtr<u16>>((0x96FDF4 + u32::from(rd::<u16>(n, 0x12)) * 4) as usize).read();
            let nl = u32::from(navi_links.wrapping_index(i32::from(rd::<i16>(n, 0x10))).read());
            let ar = nl >> 10;
            if !path_nodes(ar).is_null() {
                let navi = g::<GPtr<u8>>((0x96F974 + ar * 4) as usize).read();
                let link = navi.byte_add(((nl & 0x3FF) * 0xE) as isize);
                let b = rd::<u8>(link, 0xB);
                let flag = g::<u8>(0xC0FCB5);
                if b & 7 < 2 && b & 0x38 < 0x10 {
                    flag.write(0);
                } else {
                    let mut tmp = [0f32; 3];
                    let c = uncompress(link.cast(), GPtr::from_raw(tmp.as_mut_ptr()));
                    let ex = x(c.read()).fsub_m(px);
                    let ey = x(c.wrapping_index(1).read()).fsub_m(py);
                    let e = ey.fmul(ey).fadd(ex.fmul(ex)).fsqrt();
                    if jp5(e, x(40.0)) {
                        flag.write(0);
                    } else {
                        flag.write(1);
                        if rd::<u8>(n, 0x19) & 0x30 < 0x20 {
                            flag.write(0);
                        }
                    }
                }
            }
        }
    }
    // 0x424416: start node
    let mut range1 = range1;
    let start = if rand_at(0x42441B) & 0xF == 4 && no_random_start == 0 {
        let mut tmp = [0u32; 1];
        let p = find_node_closest_to_coors(
            the_paths(),
            GPtr::from_raw(tmp.as_mut_ptr()),
            px,
            py,
            pz,
            0,
            50.0,
            0,
            0,
            0,
            1,
            1,
        );
        range1 = x(range1).fmul_m(1.5).to_f32();
        p.read()
    } else {
        let r = rand_at(0x424476);
        let a = match (r & 3 != 0, restrict != 0) {
            (true, true) => 0x969104,
            (true, false) => 0x9690FC,
            (false, true) => 0x969100,
            (false, false) => 0x9690F8,
        };
        g::<u32>(a).read()
    };
    let mut cur = start;
    if cur & 0xFFFF == 0xFFFF || path_nodes(cur & 0xFFFF).is_null() {
        return 0;
    }
    let mut visited = [0u32; 30];
    eh_vector_ctor_iter(GPtr::from_raw(visited.as_mut_ptr()).cast(), 4, 30, 0x4241A0);
    visited[0] = start;
    let mut count = 1usize;
    let mut acc = 0f32;
    let cam = g::<u8>(0xB6F028);
    let outp = |i: i32| out.wrapping_index(i);
    loop {
        // 0x4244F0: outer loop
        if jp5(x(acc), x(230.0)) {
            return 0;
        }
        let area = cur & 0xFFFF;
        let nodes = path_nodes(area);
        if nodes.is_null() {
            return 0;
        }
        let cn = nodes.byte_add(((cur >> 16) * 0x1C) as isize);
        let num = i32::from(rd::<u8>(cn, 0x18) & 0xF);
        g::<i32>(0x969078).write(num);
        let off = rand_at(0x424540) % g::<i32>(0x969078).read();
        g::<i32>(0x969074).write(off);
        let other_way = ((rand_at(0x424552) as u8) >> 4) & 1;
        g::<u8>(0x969070).write(other_way);
        let mut i = 0i32;
        let mut j = g::<i32>(0x969074).read() + g::<i32>(0x969078).read();
        let (next, nn) = loop {
            // 0x424578: inner loop over the links
            if i >= num {
                return 0;
            }
            let elems = g::<i32>(0x969078).read();
            let k = if g::<u8>(0x969070).read() != 0 {
                (i + g::<i32>(0x969074).read()) % elems
            } else {
                j % elems
            };
            let idx = i32::from(rd::<i16>(cn, 0x10)) + k;
            let links = g::<GPtr<u32>>((0x96FA94 + area * 4) as usize).read();
            let next = links.wrapping_index(idx).read();
            let ok = !path_nodes(next & 0xFFFF).is_null()
                && {
                    let navi_links = g::<GPtr<u16>>((0x96FDF4 + area * 4) as usize).read();
                    let w = u32::from(navi_links.wrapping_index(idx).read());
                    !path_nodes(w >> 10).is_null()
                }
                && !visited[..count].contains(&next);
            if ok {
                break (next, node_of(next));
            }
            i += 1;
            j -= 1;
        };
        // 0x42463E: candidate link cur -> next
        let cxr = ncoord(cn, 8);
        let cx = cxr.to_f32();
        let cyr = ncoord(cn, 0xA);
        let cy = cyr.to_f32();
        let dxc = x(px).fsub(cxr).to_f32();
        let dyc = x(py).fsub(cyr);
        let d_cur = dyc.fmul(dyc).fadd(x(dxc).fmul_m(dxc)).fsqrt().to_f32();
        let nxr = ncoord(nn, 8);
        let nx = nxr.to_f32();
        let nyr = ncoord(nn, 0xA);
        let ny = nyr.to_f32();
        let dxn = x(px).fsub(nxr).to_f32();
        let dyn_ = x(py).fsub(nyr);
        let d_next = dyn_.fmul(dyn_).fadd(x(dxn).fmul_m(dxn)).fsqrt().to_f32();
        let switched_off = rd::<u8>(nn, 0x18) & 0x20 != 0 || rd::<u8>(cn, 0x18) & 0x20 != 0;
        let mut found = false;
        if !(switched_off && restrict != 0) {
            // 0x424717: range test A (2D node positions)
            let a = x(d_next).fsub_m(range1);
            let b = x(d_cur).fsub_m(range1);
            let mut try_b = true;
            if !jp5(b.fmul(a), x(0.0)) {
                let bf = abs_fchs(b).to_f32();
                let av = abs_fchs(a);
                let cz = ncoord(cn, 0xC).to_f32();
                let cxa = x(cx).fmul(av).to_f32();
                let cya = x(cy).fmul(av);
                let cza = x(cz).fmul(av);
                let nz = ncoord(nn, 0xC).to_f32();
                let nxb = x(nx).fmul_m(bf).to_f32();
                let nyb = x(ny).fmul_m(bf);
                let nzb = x(nz).fmul_m(bf).to_f32();
                let sx = x(nxb).fadd_m(cxa).to_f32();
                let sy = nyb.fadd(cya).to_f32();
                let sz = x(nzb).fadd(cza);
                let w = av.fadd_m(bf).fdivr_m(1.0);
                let wf = w.to_f32();
                outp(0).write(x(sx).fmul(w).to_f32());
                outp(1).write(x(sy).fmul(w).to_f32());
                outp(2).write(sz.fmul(w).to_f32());
                if camera_is_sphere_visible(cam, out, 5.0) != 0 {
                    out_frac.write(x(wf).fmul_m(bf).to_f32());
                    try_b = false;
                    found = true;
                }
            }
            if try_b {
                // 0x424878: range test B (3D node positions)
                let a = x(d_next).fsub_m(range2);
                let b = x(d_cur).fsub_m(range2);
                if !jp5(b.fmul(a), x(0.0)) {
                    let bf = abs_fchs(b).to_f32();
                    let af = abs_fchs(a).to_f32();
                    let mut t1 = [0f32; 3];
                    let p = get_position(cn, GPtr::from_raw(t1.as_mut_ptr()));
                    let apx = x(af).fmul_m(p.read()).to_f32();
                    let apy = x(af).fmul_m(p.wrapping_index(1).read()).to_f32();
                    let apz = x(af).fmul_m(p.wrapping_index(2).read()).to_f32();
                    let mut t2 = [0f32; 3];
                    let q = get_position(nn, GPtr::from_raw(t2.as_mut_ptr()));
                    let bqx = x(bf).fmul_m(q.read());
                    let bqy = x(bf).fmul_m(q.wrapping_index(1).read());
                    let bqz = x(bf).fmul_m(q.wrapping_index(2).read()).to_f32();
                    let sx = bqx.fadd_m(apx).to_f32();
                    let sy = bqy.fadd_m(apy);
                    let sz = x(bqz).fadd_m(apz);
                    let w = x(af).fadd_m(bf).fdivr_m(1.0);
                    let wf = w.to_f32();
                    outp(0).write(x(sx).fmul(w).to_f32());
                    outp(1).write(sy.fmul(w).to_f32());
                    outp(2).write(sz.fmul(w).to_f32());
                    if camera_is_sphere_visible(cam, out, 5.0) == 0 {
                        out_frac.write(x(wf).fmul_m(bf).to_f32());
                        found = true;
                    }
                }
            }
        }
        if found {
            // 0x4249AC: slope check
            let nxr = ncoord(nn, 8);
            let nyr = ncoord(nn, 0xA);
            let cxr = ncoord(cn, 8);
            let cyf = ncoord(cn, 0xA).to_f32();
            let ddx = cxr.fsub(nxr).to_f32();
            let ddy = x(cyf).fsub(nyr);
            let dz = ncoord(cn, 0xC).fsub(ncoord(nn, 0xC)).fabs();
            let l = x(ddx).fmul_m(ddx).fadd(ddy.fmul(ddy)).fsqrt();
            if jp5(l.fmul_m(0.5), dz) {
                // 0x424A66: accept
                if rand_at(0x424A6B) & 8 != 0 {
                    out_n1.write(cur);
                    out_n2.write(next);
                } else {
                    out_n1.write(next);
                    out_n2.write(cur);
                    out_frac.write(x(1.0).fsub_m(out_frac.read()).to_f32());
                }
                let n2 = node_of(out_n2.read());
                let n1 = node_of(out_n1.read());
                if rd::<u8>(n2, 0x18) & 0x10 != 0
                    && this_node_will_lead_into_a_dead_end(the_paths(), n2, n1) != 0
                    && restrict != 0
                {
                    return 0;
                }
                let ex = x(outp(0).read()).fsub_m(px);
                let ey = x(outp(1).read()).fsub_m(py);
                let inv = ey.fmul(ey).fadd(ex.fmul(ex)).fsqrt().fdivr_m(1.0);
                let dot = inv.fmul(ey).fmul_m(dir_y).fadd(inv.fmul(ex).fmul_m(dir_x));
                let c = dot.fcom(x(dot_limit));
                let ok = if want_ahead == 0 {
                    !c.test_parity(0x41)
                } else {
                    !c.test_any(0x41)
                };
                if !ok {
                    return 0;
                }
                let n1 = out_n1.read();
                let n2 = out_n2.read();
                sprintf4(
                    g(0xB71670),
                    g(0x858F58),
                    n1 & 0xFFFF,
                    n1 >> 16,
                    n2 & 0xFFFF,
                    n2 >> 16,
                );
                return 1;
            }
        }
        // 0x424A92: advance to `next`
        let cxr = ncoord(cn, 8);
        let cyr = ncoord(cn, 0xA);
        cur = next;
        visited[count] = next;
        count += 1;
        let nxr = ncoord(nn, 8);
        let nyf = ncoord(nn, 0xA).to_f32();
        let ddx = nxr.fsub(cxr).to_f32();
        let ddy = x(nyf).fsub(cyr);
        let l = x(ddx).fmul_m(ddx).fadd(ddy.fmul(ddy)).fsqrt();
        acc = l.fadd_m(acc).to_f32();
        if count >= 30 {
            return 0;
        }
    }
}

orig! {
    /// `CVehicle::CanBeDeleted()` (0x6D1180)
    fn vehicle_can_be_deleted(v: GPtr<u8>) -> u8 = thiscall 0x6D1180 => unported;
    /// `CCranes::IsThisCarBeingTargettedByAnyCrane(CVehicle*)` (0x6F4000)
    fn is_this_car_being_targetted_by_any_crane(v: GPtr<u8>) -> u8 = cdecl 0x6F4000 => unported;
    /// `CVisibilityPlugins::GetClumpAlpha(RpClump*)` (0x732B20)
    fn get_clump_alpha(clump: GPtr<u8>) -> i32 = cdecl 0x732B20 => unported;
    /// `CWorld::FindPlayerCentreOfWorld(int32)` (0x56E250)
    fn find_player_centre_of_world(id: i32) -> GPtr<u8> = cdecl 0x56E250 => sa_world::world::cworld::find_player_centre_of_world;
    /// `CEntity::GetIsOnScreen()` (0x534540)
    fn entity_get_is_on_screen(e: GPtr<u8>) -> u8 = thiscall 0x534540 => unported;
    /// `CCamera::GetLookDirection()` (0x50AE90)
    fn camera_get_look_direction(cam: GPtr<u8>) -> i32 = thiscall 0x50AE90 => unported;
    /// `max(float, float)` helper (0x420800, ST0)
    fn float_max(a: f32, b: f32) -> f64 = cdecl 0x420800 => unported;
    /// `CGarages::IsPointWithinHideOutGarage(const CVector&)` (0x448900)
    fn is_point_within_hide_out_garage(p: GPtr<f32>) -> u8 = cdecl 0x448900 => unported;
    /// `CCarCtrl::IsThisVehicleInteresting(CVehicle*)` (0x423EA0)
    fn is_this_vehicle_interesting(v: GPtr<u8>) -> u8 = cdecl 0x423EA0 => crate::carctrl::is_this_vehicle_interesting;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60)
    fn vector_sub(out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>) -> GPtr<f32> = cdecl 0x40FE60 => unported;
    /// `CVector::SquaredMagnitude()` (0x406DA0, ST0)
    fn vector_squared_magnitude(v: GPtr<f32>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CTrafficLights::ShouldCarStopForLight(CVehicle*, bool)` (0x49D610)
    fn should_car_stop_for_light(v: GPtr<u8>, ignore: u8) -> u8 = cdecl 0x49D610 => crate::misc2::should_car_stop_for_light;
    /// `CTrafficLights::ShouldCarStopForBridge(CVehicle*)` (0x49D420)
    fn should_car_stop_for_bridge(v: GPtr<u8>) -> u8 = cdecl 0x49D420 => crate::misc2::should_car_stop_for_bridge;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CEntity` scalar deleting destructor (vtable slot 0, `delete`)
    fn entity_vdelete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
}

/// `m_matrix ? &m_matrix->pos : &placement.pos`.
fn entity_pos(e: GPtr<u8>) -> GPtr<f32> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4).cast()
    } else {
        m.byte_add(0x30).cast()
    }
}

/// `CWorld::Remove(veh); delete veh`.
fn kill(veh: GPtr<u8>) {
    world_remove(veh);
    entity_vdelete(veh, 1);
}

/// `IsPointWithinHideOutGarage` on a local copy of the position (as the original).
fn in_hideout(veh: GPtr<u8>) -> bool {
    let p = entity_pos(veh);
    let mut c = [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()];
    is_point_within_hide_out_garage(GPtr::from_raw(c.as_mut_ptr())) != 0
}

/// `CCarCtrl::PossiblyRemoveVehicle(CVehicle*)` (0x424F80; spec CCarCtrl.md, checked against the bytes): removal
/// by distance (2D distance to the player centre against `max(m_nExtendedRemovalRange, 170) * R / 170`, `R` = 45
/// for an off-screen ambient car with the camera looking ahead, else `TheCamera[+0xF4] * 170`, 70 when looking
/// steeply down; far cars are kept when on a plane police mission, in a hideout garage, interesting (fake
/// physics 10) or faded out when on screen), flipped simple cars (`|up.z| < 0.74`), stuck planes/helis, stuck
/// abandoned cars (> 5 s real physics, 2D > 22 m, ...), and burnt wrecks (60 s, 3D > 6.5 m). The original keeps
/// the range in its own argument slot (not visible to callers).
#[hook(addr = 0x424F80, conv = cdecl, name = "CCarCtrl::PossiblyRemoveVehicle", expect = "83 EC 2C 56 8B 74 24 34")]
pub fn possibly_remove_vehicle(veh: GPtr<u8>) {
    if rd::<u8>(veh, 0x485) != 0 {
        return;
    }
    if rd::<u8>(veh, 0x42B) & 8 != 0 && !rd::<GPtr<u8>>(veh, 0x41C).is_null() {
        return;
    }
    let now = || g::<u32>(0xB7CB84).read();
    let mut cen = [0f32; 3];
    let deletable = rd::<u8>(veh, 0x428) & 8 == 0
        && vehicle_can_be_deleted(veh) != 0
        && is_this_car_being_targetted_by_any_crane(veh) == 0;
    if deletable {
        if rd::<u8>(veh, 0x42A) & 4 != 0 && get_clump_alpha(rd::<GPtr<u8>>(veh, 0x18)) == 0 {
            kill(veh);
            return;
        }
        let c = find_player_centre_of_world(i32::from(g::<u8>(0xB7CD74).read())).cast::<f32>();
        cen = [c.read(), c.wrapping_index(1).read(), c.wrapping_index(2).read()];
        let p = entity_pos(veh);
        let dx = x(p.read()).fsub_m(cen[0]);
        let dy = x(p.wrapping_index(1).read()).fsub_m(cen[1]);
        let dist = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt().to_f32();
        let small = entity_get_is_on_screen(veh) == 0 && {
            let cam = (u32::from(g::<u8>(0xB6F081).read()) * 0x238) as usize;
            g::<u8>(cam + 0xB6F1A4).read() == 0
                && g::<u8>(cam + 0xB6F1A5).read() == 0
                && g::<u8>(cam + 0xB6F1A3).read() == 0
                && camera_get_look_direction(g(0xB6F028)) != 0
                && rd::<u8>(veh, 0x4A4) != 3
                && rd::<u16>(veh, 0x22) != 0x1A0
                && rd::<u16>(veh, 0x22) != 0x197
                && rd::<u8>(veh, 0x428) & 1 == 0
                && rd::<u8>(veh, 0x42B) & 2 == 0
                && now() >= rd::<u32>(veh, 0x4D0)
                && rd::<u8>(veh, 0x42C) & 0x40 == 0
        };
        let mut r = if small {
            45.0
        } else {
            x(g::<f32>(0xB6F11C).read()).fmul_m(170.0).to_f32()
        };
        if !jp5(x(g::<f32>(0xB6F9B4).read()), x(f32::from_bits(0xBF66_6666))) {
            r = 70.0;
        }
        let ext = xi(i32::from(rd::<i16>(veh, 0x4A6))).to_f32();
        let e = X87::from_f64_exact(float_max(ext, 170.0));
        let thr = e.fmul_m(r).fmul_m(f32::from_bits(0x3BC0_C0C1));
        if !jp5(thr, x(dist)) && rd::<u8>(veh, 0x3BA) != 0x3B && !in_hideout(veh) {
            if is_this_vehicle_interesting(veh) != 0 {
                wr::<u8>(veh, 0xB8, 10);
                return;
            }
            if entity_get_is_on_screen(veh) != 0 {
                wr::<u8>(veh, 0x42A, rd::<u8>(veh, 0x42A) | 4);
                return;
            }
            kill(veh);
            return;
        }
        // 0x4251B5
        let s = rd::<u8>(veh, 0x36) >> 3;
        if s == 2 {
            let uz = x(rd::<f32>(rd::<GPtr<u8>>(veh, 0x14), 0x28));
            let a = if jp5(uz, x(0.0)) { uz } else { uz.fchs() };
            if !jp5(a, x(f32::from_bits(0x3F3D_70A4))) {
                kill(veh);
                return;
            }
        }
        if (s == 3 || s == 5)
            && matches!(rd::<u32>(veh, 0x594), 3 | 4)
            && (rd::<u32>(veh, 0x1C) >> 4) & 1 != 0
        {
            kill(veh);
            return;
        }
    }
    // 0x425236: stuck / abandoned
    let sub = rd::<u32>(veh, 0x594);
    let cenp = GPtr::from_raw(cen.as_mut_ptr());
    if sub != 3 && sub != 4 && rd::<u8>(veh, 0x3BB) != 0xC {
        let s = rd::<u8>(veh, 0x36) >> 3;
        let style = rd::<u8>(veh, 0x3B9);
        if s == 2 || (s == 3 && (style == 0 || style == 4)) {
            let t = now();
            let ok = t.wrapping_sub(rd::<u32>(veh, 0x3B0)) > 5000
                && rd::<u32>(veh, 0x4D0) == 0
                && t != 0
                && entity_get_is_on_screen(veh) == 0
                && {
                    let mut d = [0f32; 3];
                    let dp = vector_sub(GPtr::from_raw(d.as_mut_ptr()), entity_pos(veh), cenp);
                    let dy = x(dp.wrapping_index(1).read());
                    let dx = x(dp.read());
                    let l = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt();
                    !jne_mask(l, x(22.0), 0x41)
                }
                && is_this_vehicle_interesting(veh) == 0
                && rd::<u8>(veh, 0x428) & 8 == 0
                && vehicle_can_be_deleted(veh) != 0
                && should_car_stop_for_light(veh, 1) == 0
                && should_car_stop_for_bridge(veh) == 0
                && !in_hideout(veh);
            if ok {
                kill(veh);
                return;
            }
        }
    }
    // 0x425379: burnt wrecks
    if rd::<u8>(veh, 0x36) & 0xF8 != 0x28 {
        return;
    }
    let blown = rd::<u32>(veh, 0x4D8);
    if blown == 0 {
        return;
    }
    let t = now();
    if t <= blown.wrapping_add(60000) || t <= rd::<u32>(veh, 0x4D0) || entity_get_is_on_screen(veh) != 0 {
        return;
    }
    let mut d = [0f32; 3];
    let dp = vector_sub(GPtr::from_raw(d.as_mut_ptr()), entity_pos(veh), cenp);
    let sq = X87::from_f64_exact(vector_squared_magnitude(dp));
    if jne_mask(sq, x(42.25), 0x41) || in_hideout(veh) {
        return;
    }
    kill(veh);
}

orig! {
    /// `CWorld::FindPlayerSpeed(int32)` (0x56E090)
    fn find_player_speed(id: i32) -> GPtr<f32> = cdecl 0x56E090 => unported;
    /// `CCullZones::FewerCars()` (0x72DD80)
    fn cull_zones_fewer_cars() -> u8 = cdecl 0x72DD80 => unported;
    /// `CPopulation::FindCarMultiplierMotorway()` (0x611B60, ST0)
    fn find_car_multiplier_motorway() -> f64 = cdecl 0x611B60 => unported;
    /// `CWorld::FindPlayerWanted(int32)` (0x56E230)
    fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CCarCtrl::ChooseModel(int32*)` (0x424CE0)
    fn choose_model_call(t: GPtr<i32>) -> i32 = cdecl 0x424CE0 => choose_model;
    /// `CGameLogic::LaRiotsActiveHere()` (0x441C10)
    fn la_riots_active_here() -> u8 = cdecl 0x441C10 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => sa_world::world::cworld::find_player_vehicle_port;
    /// `CCarCtrl::GenerateCarCreationCoors2(...)` (0x424210)
    fn generate_car_creation_coors2_call(px: f32, py: f32, pz: f32, dir_x: f32, dir_y: f32, dot_limit: f32, want_ahead: u32, range1: f32, range2: f32, out: GPtr<f32>, out_n1: GPtr<u32>, out_n2: GPtr<u32>, out_frac: GPtr<f32>, restrict: u32, no_random_start: u32) -> u8 = cdecl 0x424210 => unported;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0)
    fn request_model(m: i32, flags: i32) = cdecl 0x4087E0 => sa_stream::streaming::request_model;
    /// `CLoadedCarGroup::PickLeastUsedModel(int32)` (0x611E90, `ret 4`)
    fn pick_least_used_model(this: GPtr<u8>, n: i32) -> i32 = thiscall 0x611E90 => unported;
    /// `CWorld::FindObjectsKindaColliding(...)` (0x568B80)
    fn find_objects_kinda_colliding(pos: GPtr<f32>, r: f32, b2d: u32, cnt: GPtr<i16>, max: i32, out: u32, bld: u32, veh: u32, ped: u32, obj: u32, dum: u32) = cdecl 0x568B80 => unported;
    /// `CNodeAddress::operator!=` (0x420950)
    fn node_ne(a: GPtr<u16>, b: GPtr<u16>) -> u32 = thiscall 0x420950 => crate::pathfind::node_address_ne;
    /// `CNodeAddress::operator==` (0x420920)
    fn node_eq(a: GPtr<u16>, b: GPtr<u16>) -> u32 = thiscall 0x420920 => crate::pathfind::node_address_eq;
    /// `CNodeAddress::operator<` (0x420980)
    fn node_lt(a: GPtr<u16>, b: GPtr<u16>) -> u32 = thiscall 0x420980 => crate::pathfind::node_address_lt;
    /// `CTheZones::GetZoneInfo(const CVector&, CZone**)` (0x572400)
    fn get_zone_info(p: GPtr<f32>, out: GPtr<GPtr<u8>>) -> GPtr<u8> = cdecl 0x572400 => sa_world::world::zones::get_zone_info;
    /// `CCarCtrl::GetNewVehicleDependingOnCarModel(int32, uint8)` (0x421440)
    fn get_new_vehicle_call(mi: i32, created_by: u32) -> GPtr<u8> = cdecl 0x421440 => get_new_vehicle_depending_on_car_model;
    /// `CGeneral::GetRandomNumberInRange<float>(float, float)` (0x41BD90, ST0)
    fn random_float_in_range(min: f32, max: f32) -> f64 = cdecl 0x41BD90 => unported;
    /// `CCarAI::FindPoliceBoatMissionForWantedLevel()` (0x41CA50)
    fn find_police_boat_mission() -> u8 = cdecl 0x41CA50 => crate::carctrl2::find_police_boat_mission_for_wanted_level;
    /// `CCarAI::FindPoliceCarSpeedForWantedLevel(CVehicle*)` (0x41CAA0; ported unhookable in carctrl2)
    fn find_police_car_speed(veh: GPtr<u8>) -> i32 = cdecl 0x41CAA0 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn get_vehicle_appearance(veh: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `CCarAI::FindPoliceBikeMissionForWantedLevel()` (0x41CA40)
    fn find_police_bike_mission() -> u8 = cdecl 0x41CA40 => unported;
    /// `CCarAI::FindPoliceCarMissionForWantedLevel()` (0x41C9D0; ported unhookable in carctrl2)
    fn find_police_car_mission() -> u8 = cdecl 0x41C9D0 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<f32>) = thiscall 0x59C910 => unported;
    /// `CCarCtrl::PickNextNodeRandomly(CVehicle*)` (0x42DE80)
    fn pick_next_node_randomly(veh: GPtr<u8>) = cdecl 0x42DE80 => crate::carctrl4::pick_next_node_randomly;
    /// `CCarPathLink::OneWayLaneOffset()` (0x44DB00, ST0)
    fn one_way_lane_offset(l: GPtr<u8>) -> f64 = thiscall 0x44DB00 => unported;
    /// `CCarCtrl::FindSpeedMultiplierWithSpeedFromNodes(int8)` (0x424130, ST0)
    fn find_speed_multiplier_with_speed_from_nodes(s: u32) -> f64 = cdecl 0x424130 => unported;
    /// `CCurves::CalcSpeedScaleFactor(const CVector&, const CVector&, float x4)` (0x43C710, ST0)
    fn calc_speed_scale_factor(p1: GPtr<f32>, p2: GPtr<f32>, d1x: f32, d1y: f32, d2x: f32, d2y: f32) -> f64 = cdecl 0x43C710 => unported;
    /// `CCurves::CalcCurvePoint(const CVector&, const CVector&, const CVector&, const CVector&, float, int32, CVector&, CVector&)` (0x43C900)
    fn calc_curve_point(p1: GPtr<f32>, p2: GPtr<f32>, d1: GPtr<f32>, d2: GPtr<f32>, t: f32, ms: i32, out_pos: GPtr<f32>, out_speed: GPtr<f32>) = cdecl 0x43C900 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    fn vector_magnitude(v: GPtr<f32>) -> f64 = thiscall 0x4082C0 => unported;
    /// `out = v * s` (0x40FEC0)
    fn vector_scale(out: GPtr<f32>, v: GPtr<f32>, s: f32) -> GPtr<f32> = cdecl 0x40FEC0 => unported;
    /// `out = a + b` (0x40FE30)
    fn vector_add(out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>) -> GPtr<f32> = cdecl 0x40FE30 => unported;
    /// `out = v * (1 / d)` (0x4119D0)
    fn vector_div(out: GPtr<f32>, v: GPtr<f32>, d: f32) -> GPtr<f32> = cdecl 0x4119D0 => unported;
    /// `CWaterLevel::GetWaterLevel(float, float, float, float*, bool, CVector*)` (0x6EB690)
    fn get_water_level(x: f32, y: f32, z: f32, out: GPtr<f32>, waves: u32, normal: u32) -> u8 = cdecl 0x6EB690 => unported;
    /// `CVehicle` vtable slot 0xD4 `GetHeightAboveRoad()` (ST0)
    fn vehicle_get_height_above_road(v: GPtr<u8>) -> f64 = thiscall vslot 0xD4 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0, `ret 4`)
    fn placeable_set_posn_v(e: GPtr<u8>, p: GPtr<f32>) = thiscall 0x4241C0 => unported;
    /// `CVehicle::AddVehicleUpgrade(int32)` (0x6E3290, `ret 4`)
    fn vehicle_add_upgrade(v: GPtr<u8>, mi: i32) = thiscall 0x6E3290 => unported;
    /// `CVehicleModelInfo::ChooseVehicleColour(uint8&, uint8&, uint8&, uint8&, int32)` (0x4C8500, `ret 0x14`)
    fn choose_vehicle_colour(mi: GPtr<u8>, c1: GPtr<u8>, c2: GPtr<u8>, c3: GPtr<u8>, c4: GPtr<u8>, inc: i32) = thiscall 0x4C8500 => unported;
    /// `CAutomobile::SetRandomDamage(bool)` (0x6A2530, `ret 4`)
    fn automobile_set_random_damage(v: GPtr<u8>, b: u32) = thiscall 0x6A2530 => unported;
    /// `CPlayerPed::GetWantedLevel()` (0x41BE60)
    fn player_get_wanted_level(p: GPtr<u8>) -> i32 = thiscall 0x41BE60 => unported;
    /// `CCarCtrl::CreatePoliceChase(CVehicle*, int32, CNodeAddress)` (0x42C2B0)
    fn create_police_chase(v: GPtr<u8>, ty: i32, node: u32) -> u8 = cdecl 0x42C2B0 => unported;
    /// `CCarCtrl::CreateConvoy(CVehicle*, int32)` (0x42C740)
    fn create_convoy(v: GPtr<u8>, ty: i32) -> u8 = cdecl 0x42C740 => unported;
    /// `CPlaceable::GetTopDirection(CVector*)` (0x41CCB0, `ret 4`)
    fn placeable_get_top_direction(e: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x41CCB0 => unported;
    /// `CCarAI::AddPoliceCarOccupants(CVehicle*, bool)` (0x41C070)
    fn add_police_car_occupants(v: GPtr<u8>, always: u8) = cdecl 0x41C070 => crate::carctrl4::add_police_car_occupants;
    /// `CVehicle::ChangeLawEnforcerState(uint8)` (0x6D2330, `ret 4`)
    fn change_law_enforcer_state(v: GPtr<u8>, on: u32) = thiscall 0x6D2330 => unported;
    /// `CStreaming::PossiblyStreamCarOutAfterCreation(int32)` (0x40BA70)
    fn possibly_stream_car_out_after_creation(model: i32) = cdecl 0x40BA70 => sa_stream::streaming::possibly_stream_car_out_after_creation;
    /// model-info use counter (0x421120, thiscall)
    fn model_info_use_counter(mi: GPtr<u8>) = thiscall 0x421120 => crate::carctrl2::auto_pilot_speed_step;
    /// `CVisibilityPlugins::SetClumpAlpha(RpClump*, int32)` (0x732B00)
    fn set_clump_alpha(clump: GPtr<u8>, alpha: i32) = cdecl 0x732B00 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CCarCtrl::SetUpDriverAndPassengersForVehicle(CVehicle*, int32, int32, bool, bool, int32)` (0x4217C0)
    fn set_up_driver_and_passengers_for_vehicle(veh: GPtr<u8>, pt: i32, min: i32, a: u32, b: u32, max: i32) = cdecl 0x4217C0 => crate::carctrl2::set_up_driver_and_passengers_for_vehicle;
    /// `CModelInfo::IsBoatModel(int32)` (0x4C5A70)
    fn is_boat_model(id: i32) -> bool = cdecl 0x4C5A70 => sa_stream::modelinfo::model_info::is_boat_model;
    /// `CWorld::ProcessVerticalLine(...)` (0x5674E0)
    fn process_vertical_line_g(o: GPtr<f32>, z: f32, cp: GPtr<u8>, ent: GPtr<u32>, b: u32, v: u32, p: u32, ob: u32, d: u32, see: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
}

/// `CEntity` scalar deleting destructor (`delete veh`, vtable slot 0 with 1).
fn gorc_delete(veh: GPtr<u8>) {
    entity_vdelete(veh, 1);
}

/// The position pointer of an entity (matrix +0x30 or placement +4).
fn pos_of(e: GPtr<u8>) -> GPtr<f32> {
    entity_pos(e)
}

/// `CPathNode::GetPosition` into a fresh local; returns the 3 floats.
fn node_pos(n: GPtr<u8>) -> [f32; 3] {
    let mut b = [0f32; 3];
    let p = get_position(n, GPtr::from_raw(b.as_mut_ptr()));
    [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()]
}

/// `navi(L)`: `m_pNaviNodes[L >> 10] + (L & 0x3FF) * 0xE` for a navi link address word.
fn navi_link(l: u32) -> GPtr<u8> {
    g::<GPtr<u8>>((0x96F974 + (l >> 10) * 4) as usize)
        .read()
        .byte_add(((l & 0x3FF) * 0xE) as isize)
}

/// 2D length of `VectorSub(a, b)` as the sequences `fld v.y; fld v.x; fld st0; fmul st1; fld st2; fmul st3;
/// faddp; fsqrt` compute it: `x*x + y*y`.
fn len2d_sub(a: GPtr<f32>, b: GPtr<f32>) -> X87 {
    let mut o = [0f32; 3];
    let v = vector_sub(GPtr::from_raw(o.as_mut_ptr()), a, b);
    let vy = x(v.wrapping_index(1).read());
    let vx = x(v.read());
    vx.fmul(vx).fadd(vy.fmul(vy)).fsqrt()
}

/// `CCarCtrl::GenerateOneRandomCar()` (0x430050, entry `nop; jmp` to `sub esp, 0x12C` at 0x403AEE; spec
/// CCarCtrl.p3.md, every block re-read against the bytes). Budget (car counts against the motorway multiplier
/// times the car limit and the popcycle car percentages), model (police car on wanted level 2+ within the
/// pursuit limits and timers, else `ChooseModel`, LA riots police), search direction (camera looking down, the
/// player's vehicle speed direction with frame-dependent dot limits, the camera 2D direction),
/// `GenerateCarCreationCoors2`, spawn probability of the two nodes, boats on water nodes, clearance, lanes
/// (one past the node's links when N2 is not linked, kept), gang zones, the vehicle (no null check, kept) and
/// its autopilot (cruise speeds, missions, mad drivers), the position on the link (`frac` clamped by half the
/// model length), the next link, the orientation, the curve parameters (`CalcSpeedScaleFactor`,
/// `CalcCurvePoint`), the ground (water level or two vertical lines, at most 7.0 away), status, the visibility
/// tests (on screen: farther than the camera range or nearer than 150 / 120; off screen: farther than 45 m
/// scaled), the clearance and approach test, colours, `CWorld::Add`, damage, occupants (police chases,
/// convoys, mad drivers, police occupants) and the law enforcer state. Every stored f32 of the original is
/// stored (rounded) here at the same point.
#[hook(addr = 0x430050, conv = cdecl, name = "CCarCtrl::GenerateOneRandomCar", expect = "90 E9 98 3A FD FF")]
pub fn generate_one_random_car() {
    // Block A: budget
    let c = find_player_centre_of_world(i32::from(g::<u8>(0xB7CD74).read())).cast::<f32>();
    let mut cen = [c.read(), c.wrapping_index(1).read(), c.wrapping_index(2).read()];
    let s = find_player_speed(-1);
    let spd = [s.read(), s.wrapping_index(1).read(), s.wrapping_index(2).read()];
    let gi = |a: usize| g::<i32>(a).read();
    let total = gi(0x9690AC)
        .wrapping_add(gi(0x9690A8))
        .wrapping_add(gi(0x96909C))
        .wrapping_add(gi(0x969098))
        .wrapping_add(gi(0x969094));
    let mut dens = g::<f32>(0x8A5B20).read();
    if cull_zones_fewer_cars() != 0 {
        dens = x(dens).fmul_m(f32::from_bits(0x3F19_999A)).to_f32();
    }
    let tf = xi(total).to_f32();
    let m1 = X87::from_f64_exact(find_car_multiplier_motorway())
        .fmul(xi(g::<i32>(0x8A5B24).read()))
        .fmul_m(dens);
    if !m1.fcom(x(tf)).test_parity(0x41) {
        return;
    }
    let m2 = X87::from_f64_exact(find_car_multiplier_motorway());
    let p = |a: usize| g::<f32>(a).read();
    let sum = x(p(0xC0BC30))
        .fadd_m(p(0xC0BC34))
        .fadd_m(p(0xC0BC38))
        .fadd_m(p(0xC0BC3C));
    if !sum.fmul(m2).fmul_m(dens).fcom(x(tf)).test_parity(0x41) {
        return;
    }
    // Block B: model
    let wl = || rd::<i32>(find_player_wanted(-1), 0x2C);
    let now = || g::<u32>(0xB7CB84).read();
    let mut ty = [0i32; 1];
    let mut model: i32;
    let police = wl() > 1
        && g::<i32>(0x969098).read() < i32::from(rd::<u8>(find_player_wanted(-1), 0x1A))
        && {
            let w1 = find_player_wanted(-1);
            let w2 = find_player_wanted(-1);
            rd::<u8>(w2, 0x18) < rd::<u8>(w1, 0x19)
        }
        && g::<i32>(0xB72914).read() == 0
        && gang_war_fighting_going_on() == 0
        && (wl() > 3 || {
            let l = wl();
            let last = g::<u32>(0x9690B8).read();
            let t = now();
            (l > 2 && t > last.wrapping_add(5000)) || t > last.wrapping_add(8000)
        });
    if police {
        model = choose_police_car_model(0);
        ty[0] = 13;
    } else {
        model = choose_model_call(GPtr::from_raw(ty.as_mut_ptr()));
        if model == -1 {
            return;
        }
        if (ty[0] == 13 || ty[0] == 0x18) && wl() >= 1 {
            return;
        }
    }
    if la_riots_active_here() != 0
        && g::<u8>(0xB72959).read() == 0
        && ((rand_at(0x43027A) & 0x7F) as i8) < 0x37
    {
        model = choose_police_car_model(0);
        ty[0] = 13;
    }
    // Block C: search direction
    let mut top_down = false;
    let (dir_x, dir_y, dot_limit, want_ahead): (f32, f32, f32, u8);
    let frame = || g::<u32>(0xB7CB4C).read();
    let cam_dir = || (g::<f32>(0xB6F104).read(), g::<f32>(0xB6F108).read());
    let by_frame_parity = |d: (f32, f32)| {
        if frame() & 1 == 0 {
            (d.0, d.1, f32::from_bits(0x3F34_FDF4), 1u8)
        } else {
            (d.0, d.1, f32::from_bits(0x3F34_FDF4), 0u8)
        }
    };
    if !jp5(x(g::<f32>(0xB6F9B4).read()), x(f32::from_bits(0xBF66_6666))) {
        top_down = true;
        (dir_x, dir_y, dot_limit, want_ahead) =
            (f32::from_bits(0x3F34_FDF4), f32::from_bits(0x3F34_FDF4), -1.0, 1);
    } else {
        let pv = find_player_vehicle(-1, 0);
        if pv.is_null() {
            (dir_x, dir_y, dot_limit, want_ahead) = by_frame_parity(cam_dir());
        } else {
            let my = x(rd::<f32>(pv, 0x48));
            let mx = rd::<f32>(pv, 0x44);
            let sp = my.fmul(my).fadd(x(mx).fmul_m(mx)).fsqrt();
            let k85 = f32::from_bits(0x3F59_999A);
            let k707 = f32::from_bits(0x3F34_FDF4);
            let table = if !jne_mask(sp, x(f32::from_bits(0x3ECC_CCCD)), 0x41) {
                Some([(k85, 1u8), (k85, 1), (k707, 1), (k707, 0)])
            } else if !jne_mask(sp, x(f32::from_bits(0x3DCC_CCCD)), 0x41) {
                Some([(k85, 1u8), (k707, 1), (k707, 0), (k707, 0)])
            } else {
                None
            };
            match table {
                Some(t) => {
                    let inv = sp.fdivr_m(1.0);
                    let dx = x(mx).fmul(inv).to_f32();
                    let dy = inv.fmul(my).to_f32();
                    let (dl, wa) = t[(frame() & 3) as usize];
                    (dir_x, dir_y, dot_limit, want_ahead) = (dx, dy, dl, wa);
                }
                None => {
                    (dir_x, dir_y, dot_limit, want_ahead) = by_frame_parity(cam_dir());
                }
            }
        }
    }
    let _ = dir_y;
    // Block D: creation point (`frac` shares the dirY slot)
    let restrict = if ty[0] == 13 {
        let w = find_player_wanted(-1);
        if rd::<i32>(w, 0x2C) >= 1 {
            (w.addr() as u32) & !0xFF
        } else {
            1
        }
    } else {
        1
    };
    let range1 = x(g::<f32>(0xB6F11C).read()).fmul_m(160.0).to_f32();
    let mut coors = [0f32; 3];
    let mut n1a = [0u32; 1];
    let mut n2a = [0u32; 1];
    let mut frac = [dir_y; 1];
    let cp = GPtr::from_raw(coors.as_mut_ptr());
    let n1p = GPtr::<u32>::from_raw(n1a.as_mut_ptr());
    let n2p = GPtr::<u32>::from_raw(n2a.as_mut_ptr());
    let fp = GPtr::<f32>::from_raw(frac.as_mut_ptr());
    if generate_car_creation_coors2_call(
        cen[0],
        cen[1],
        cen[2],
        dir_x,
        dir_y,
        dot_limit,
        u32::from(want_ahead),
        range1,
        38.0,
        cp,
        n1p,
        n2p,
        fp,
        restrict,
        0,
    ) == 0
    {
        return;
    }
    // Block E: node checks, boats, clearance, lanes
    let n1 = n1a[0];
    let n2 = n2a[0];
    let nodes = |a: u32| g::<GPtr<u8>>((0x96F854 + (a & 0xFFFF) * 4) as usize).read();
    let n1node = (n1 >> 16) * 0x1C;
    let n2node = (n2 >> 16) * 0x1C;
    let pa = rd::<u8>(nodes(n2), (n2node + 0x1A) as isize) & 0xF;
    let pb = rd::<u8>(nodes(n1), (n1node + 0x1A) as isize) & 0xF;
    let prob = if pb < pa { pb } else { pa };
    let mut is_boat = false;
    if (rand_at(0x4304BE) & 0xF) as u32 > u32::from(prob) {
        return;
    }
    let radius = if rd::<u8>(nodes(n1), (n1node + 0x18) as isize) & 0x80 != 0 {
        is_boat = true;
        if ty[0] == 13 {
            model = 0x1AE;
            ty[0] = 0x18;
            if g::<u8>(0x8E6E68).read() != 1 {
                request_model(0x1AE, 8);
                return;
            }
        } else {
            model = pick_least_used_model(g(0xC0E998), 1);
            if model == -1 || g::<u8>((0x8E4CD0 + model * 20) as usize).read() != 1 {
                return;
            }
        }
        40.0
    } else {
        8.0
    };
    let mut cnt = [0i16; 2];
    find_objects_kinda_colliding(
        cp,
        radius,
        1,
        GPtr::from_raw(cnt.as_mut_ptr()),
        2,
        0,
        0,
        1,
        1,
        0,
        0,
    );
    if cnt[0] != 0 {
        return;
    }
    let n1n = nodes(n1).byte_add(n1node as isize);
    let num_links = i32::from(rd::<u8>(n1n, 0x18) & 0xF);
    let base = i32::from(rd::<i16>(n1n, 0x10));
    let node_links = g::<GPtr<u32>>((0x96FA94 + (n1 & 0xFFFF) * 4) as usize).read();
    let mut i: i16 = 0;
    if num_links != 0 {
        loop {
            let l = node_links.wrapping_index(i32::from(i) + base);
            if node_ne(l.cast(), n2p.cast()) & 0xFF == 0 {
                break;
            }
            i = i.wrapping_add(1);
            if i32::from(i) >= num_links {
                break;
            }
        }
    }
    let navi_links = g::<GPtr<u16>>((0x96FDF4 + (n1 & 0xFFFF) * 4) as usize).read();
    let nav = navi_links.wrapping_index(base + i32::from(i)).read();
    let lnk = navi_link(u32::from(nav));
    let lanes: i16 = if node_eq(lnk.byte_add(4).cast(), n2p.cast()) & 0xFF != 0 {
        i16::from(rd::<u8>(lnk, 0xB) & 7)
    } else {
        i16::from((rd::<u8>(lnk, 0xB) >> 3) & 7)
    };
    if lanes > 1 {
        if rd::<u32>(model_info(model), 0x3C) == 10 {
            return;
        }
    } else if model == 0x1B5 || model == 0x1AF {
        return;
    }
    if lanes == 0 {
        return;
    }
    let mut gang_zone = false;
    if !g::<GPtr<u8>>(0xC0BC64).read().is_null() {
        let zi = get_zone_info(cp, GPtr::NULL);
        let z = i32::from(rd::<u8>(zi, 0xF) & 0x1F);
        if (0x11..=0x13).contains(&z) {
            if z != g::<i32>(0xC0BC6C).read() {
                return;
            }
            gang_zone = true;
        }
    }
    // Block F: vehicle and driving parameters
    let veh = get_new_vehicle_call(model, 1);
    let t = ty[0];
    wr::<u16>(veh, 0x398, 0xFFFF);
    wr::<u32>(veh, 0x390, n1);
    wr::<u32>(veh, 0x394, n2);
    let rnd = |a: f32, b: f32| ftol32(X87::from_f64_exact(random_float_in_range(a, b))) as u8;
    if t == 13 {
        wr::<u8>(veh, 0x3BB, 0);
        if wl() == 0 {
            wr::<u8>(veh, 0x3D0, rnd(18.0, 24.0));
            wr::<u8>(veh, 0x3B9, 0);
            wr::<u8>(veh, 0x3BA, 1);
        } else {
            wr::<u8>(veh, 0x3D0, find_police_car_speed(veh) as u8);
            let m = if get_vehicle_appearance(veh) == 2 {
                find_police_bike_mission()
            } else {
                find_police_car_mission()
            };
            wr::<u8>(veh, 0x3BA, m);
            wr::<u8>(veh, 0x3B9, 2);
        }
        if model == 0x1EA {
            wr::<u8>(veh, 0x434, 0);
            wr::<u8>(veh, 0x435, 0);
        }
        wr::<u8>(veh, 0x42C, rd::<u8>(veh, 0x42C) | 1);
    } else if t == 0x18 {
        wr::<u8>(veh, 0x3BB, 0);
        wr::<u8>(veh, 0x3D0, rnd(14.0, 18.0));
        wr::<u8>(veh, 0x3B9, 2);
        wr::<u8>(veh, 0x3BA, find_police_boat_mission());
        wr::<u8>(veh, 0x42C, rd::<u8>(veh, 0x42C) | 1);
    } else {
        wr::<u8>(veh, 0x3D0, rnd(13.0, 21.0));
        if t == 3 {
            wr::<u8>(veh, 0x3D0, rnd(18.0, 27.0));
        } else if t == 1 {
            wr::<u8>(veh, 0x3D0, rnd(10.0, 15.0));
        }
        let vm = rd::<i16>(veh, 0x22);
        let col = rd::<GPtr<u8>>(model_info(i32::from(vm)), 0x14);
        let long = !jne_mask(x(rd::<f32>(col, 0x10)).fsub_m(rd::<f32>(col, 4)), x(10.0), 0x41);
        if long || t == 5 {
            let c3 = i32::from(rd::<u8>(veh, 0x3D0)) * 3;
            wr::<u8>(veh, 0x3D0, (c3 / 4) as u8);
        }
        if is_boat {
            let fast = vm == 0x1BE || vm == 0x1C4 || vm == 0x1ED;
            wr::<u8>(veh, 0x3D0, if fast { rnd(25.0, 35.0) } else { rnd(15.0, 24.0) });
        }
        wr::<u8>(veh, 0x3BA, 1);
        wr::<u8>(veh, 0x3BB, 0);
        wr::<u8>(veh, 0x3B9, 0);
    }
    if rd::<u16>(veh, 0x22) == 0x1A7 {
        wr::<u8>(veh, 0x42D, rd::<u8>(veh, 0x42D) | 0x80);
    }
    wr::<u16>(veh, 0x3A6, nav);
    let lane = (rand_at(0x4308C1) % i32::from(lanes)) as u8;
    wr::<u8>(veh, 0x3B7, lane);
    wr::<u8>(veh, 0x3B8, lane);
    let n = if la_riots_active_here() != 0 {
        0x50
    } else {
        match get_vehicle_appearance(veh) {
            2 => 0x32,
            4 => 0xA,
            _ => 0xC8,
        }
    };
    let mut mad = false;
    if !is_boat && t != 13 && !gang_zone && (random_int_in_range(0, n) == 0 || g::<u8>(0x96914F).read() != 0)
    {
        mad = true;
        frac[0] = 1.0;
    }
    // Block G: position on the link
    let vcol = rd::<GPtr<u8>>(model_info(i32::from(rd::<i16>(veh, 0x22))), 0x14);
    let node1 = nodes(n1).byte_add(n1node as isize);
    let half = x(rd::<f32>(vcol, 0x10))
        .fsub_m(rd::<f32>(vcol, 4))
        .fmul_m(0.5)
        .fadd_m(1.0)
        .to_f32();
    let p1y_a = node_pos(node1)[1];
    let node2 = nodes(n2).byte_add(n2node as isize);
    let p1y_b = node_pos(node1)[1];
    let p1x_a = node_pos(node1)[0];
    let p2_a = node_pos(node2);
    let p1x_b = node_pos(node1)[0];
    let p2_b = node_pos(node2);
    let p2_c = node_pos(node2);
    let dy_f = x(p1y_b).fsub_m(p2_c[1]).to_f32();
    let p2_d = node_pos(node2);
    let t1 = x(p1y_a).fsub_m(p2_d[1]).fmul_m(dy_f);
    let dx = x(p1x_b).fsub_m(p2_b[0]);
    let dx2 = x(p1x_a).fsub_m(p2_a[0]);
    let len = t1.fadd(dx.fmul(dx2)).fsqrt();
    if !jp5(x(0.5).fmul(len), x(half)) {
        frac[0] = 0.5;
    } else {
        let q = len.fdivr_m(half);
        if jne_mask(x(frac[0]), q, 0x41) {
            frac[0] = q.to_f32();
        }
        let r = q.fsubr_m(1.0);
        if jp5(x(frac[0]), r) {
            frac[0] = r.to_f32();
        }
    }
    wr::<u8>(
        veh,
        0x3B6,
        if node_lt(n1p.cast(), n2p.cast()) & 0xFF != 0 {
            0xFF
        } else {
            1
        },
    );
    let node1 = nodes(n1).byte_add(n1node as isize);
    let nl1 = i32::from(rd::<u8>(node1, 0x18) & 0xF);
    if nl1 == 1 {
        gorc_delete(veh);
        return;
    }
    let (a, k) = loop {
        let k = rand_at(0x430AD5) % i32::from(rd::<u8>(node1, 0x18) & 0xF);
        let idx = i32::from(rd::<i16>(nodes(n1).byte_add(n1node as isize), 0x10)) + i32::from(k as i16);
        let a = g::<GPtr<u16>>((0x96FDF4 + (n1 & 0xFFFF) * 4) as usize)
            .read()
            .wrapping_index(idx)
            .read();
        if a != rd::<u16>(veh, 0x3A6) {
            break (a, k);
        }
    };
    wr::<u16>(veh, 0x3A4, a);
    if nodes(u32::from(a >> 10)).is_null() {
        gorc_delete(veh);
        return;
    }
    let idx = i32::from(rd::<i16>(nodes(n1).byte_add(n1node as isize), 0x10)) + i32::from(k as i16);
    let lk = g::<GPtr<u32>>((0x96FA94 + (n1 & 0xFFFF) * 4) as usize)
        .read()
        .wrapping_index(idx);
    wr::<u8>(
        veh,
        0x3B5,
        if node_lt(lk.cast(), n1p.cast()) & 0xFF != 0 {
            0xFF
        } else {
            1
        },
    );
    // orientation
    let node2 = nodes(n2).byte_add(n2node as isize);
    let node1 = nodes(n1).byte_add(n1node as isize);
    let mut b1 = [0f32; 3];
    let pp1 = get_position(node1, GPtr::from_raw(b1.as_mut_ptr()));
    let mut b2 = [0f32; 3];
    let pp2 = get_position(node2, GPtr::from_raw(b2.as_mut_ptr()));
    let mut dd = [0f32; 3];
    let d = vector_sub(GPtr::from_raw(dd.as_mut_ptr()), pp2, pp1);
    let mut d3 = [d.read(), d.wrapping_index(1).read(), d.wrapping_index(2).read()];
    let (ddx, ddy) = (d.read(), d.wrapping_index(1).read());
    let l = x(ddy).fmul_m(ddy).fadd(x(ddx).fmul_m(ddx)).fsqrt();
    let (d2x, d2y) = if !l.fcom(x(0.0)).test_parity(0x44) {
        (1.0f32, ddy)
    } else {
        let inv = l.fdivr_m(1.0);
        (x(ddx).fmul(inv).to_f32(), inv.fmul_m(ddy).to_f32())
    };
    vector_normalise(GPtr::from_raw(d3.as_mut_ptr()));
    let mat = |o: isize, v: f32| rd::<GPtr<u8>>(veh, 0x14).byte_add(o).cast::<f32>().write(v);
    let neg = x(d2x).fchs().to_f32();
    mat(0x10, d3[0]);
    mat(0x14, d3[1]);
    mat(0x18, d3[2]);
    mat(0, d2y);
    mat(4, neg);
    mat(8, 0.0);
    mat(0x20, 0.0);
    mat(0x24, 0.0);
    mat(0x28, 1.0);
    // frac2
    let nc = rd::<u32>(veh, 0x390);
    let ln = u32::from(rd::<u16>(veh, 0x3A6));
    let lp = |l: u32, buf: &mut [f32; 3]| uncompress(navi_link(l).cast(), GPtr::from_raw(buf.as_mut_ptr()));
    let npos = |n: u32, buf: &mut [f32; 3]| get_position(node_of(n), GPtr::from_raw(buf.as_mut_ptr()));
    let (mut q1, mut q2) = ([0f32; 3], [0f32; 3]);
    let pnc = npos(nc, &mut q1);
    let lnc = lp(ln, &mut q2);
    let d_a = len2d_sub(lnc, pnc).to_f32();
    let nn = rd::<u32>(veh, 0x394);
    let (mut q3, mut q4) = ([0f32; 3], [0f32; 3]);
    let pnn = npos(nn, &mut q3);
    let lnn = lp(ln, &mut q4);
    let d_b = len2d_sub(lnn, pnn);
    let ratio = d_b.fadd_m(d_a).fdivr_m(d_a);
    let lc: u32;
    let ln2: u32;
    let frac2_x = if !jne_mask(ratio, x(frac[0]), 0x41) {
        lc = u32::from(rd::<u16>(veh, 0x3A4));
        ln2 = ln;
        let (mut q5, mut q6) = ([0f32; 3], [0f32; 3]);
        let pc = npos(nc, &mut q5);
        let lcc = lp(lc, &mut q6);
        let d_c = len2d_sub(lcc, pc).to_f32();
        let mut q7 = [0f32; 3];
        let pc2 = npos(nc, &mut q7);
        let e = len2d_sub(cp, pc2);
        e.fadd_m(d_c).fdiv(x(d_c).fadd_m(d_a))
    } else {
        pick_next_node_randomly(veh);
        let nc = rd::<u32>(veh, 0x390);
        ln2 = u32::from(rd::<u16>(veh, 0x3A6));
        let (mut q5, mut q6) = ([0f32; 3], [0f32; 3]);
        let pc = npos(nc, &mut q5);
        let lnc = lp(ln2, &mut q6);
        let mut vb = [0f32; 3];
        let v = vector_sub(GPtr::from_raw(vb.as_mut_ptr()), lnc, pc);
        lc = u32::from(rd::<u16>(veh, 0x3A4));
        let (mut q7, mut q8) = ([0f32; 3], [0f32; 3]);
        let pc2 = npos(nc, &mut q7);
        let lcc = lp(lc, &mut q8);
        let d_c = len2d_sub(lcc, pc2).to_f32();
        let mut q9 = [0f32; 3];
        let pc3 = npos(nc, &mut q9);
        let e = len2d_sub(cp, pc3);
        let (vx, vy) = (v.read(), v.wrapping_index(1).read());
        let num = e.fsubr_m(d_c);
        let vl = x(vx).fmul_m(vx).fadd(x(vy).fmul_m(vy)).fsqrt();
        num.fdiv(vl.fadd_m(d_c))
    };
    let mut frac2 = frac2_x.to_f32();
    if !jne_mask(x(0.0), x(frac2), 0x41) {
        frac2 = 0.0;
    } else if !jp5(x(1.0), x(frac2)) {
        frac2 = 1.0;
    }
    // Block H: curve setup
    let lcl = navi_link(lc);
    let lnl = navi_link(ln2);
    let s1 = xi(i32::from(rd::<i8>(veh, 0x3B5)));
    let k01 = f32::from_bits(0x3C23_D70A);
    let d1x = xi(i32::from(rd::<i8>(lcl, 8))).fmul_m(k01).fmul(s1).to_f32();
    let d1y = xi(i32::from(rd::<i8>(lcl, 9))).fmul_m(k01).fmul(s1).to_f32();
    let s2 = xi(i32::from(rd::<i8>(veh, 0x3B6)));
    let d2x = xi(i32::from(rd::<i8>(lnl, 8))).fmul_m(k01).fmul(s2).to_f32();
    let d2y = xi(i32::from(rd::<i8>(lnl, 9))).fmul_m(k01).fmul(s2).to_f32();
    let k54 = f32::from_bits(0x40AC_CCCD);
    let mut lo1 = X87::from_f64_exact(one_way_lane_offset(lcl))
        .fadd(xi(i32::from(rd::<i8>(veh, 0x3B7))))
        .fmul_m(k54)
        .to_f32();
    let mut lo2 = X87::from_f64_exact(one_way_lane_offset(navi_link(u32::from(rd::<u16>(veh, 0x3A6)))))
        .fadd(xi(i32::from(rd::<i8>(veh, 0x3B8))))
        .fmul_m(k54)
        .to_f32();
    if rd::<u32>(veh, 0x594) == 10 {
        let k = f32::from_bits(0x3FBA_9FBF);
        lo1 = x(lo1).fadd_m(k).to_f32();
        lo2 = x(lo2).fadd_m(k).to_f32();
    }
    let nnv = rd::<u32>(veh, 0x394);
    let sb = (rd::<u8>(node_of(nnv), 0x19) >> 4) & 3;
    wr::<u8>(veh, 0x3D1, sb);
    let m = X87::from_f64_exact(find_speed_multiplier_with_speed_from_nodes(u32::from(sb)));
    wr::<f32>(veh, 0x3D4, m.to_f32());
    let speed = xi(i32::from(rd::<u8>(veh, 0x3D0))).fmul(m).to_f32();
    wr::<f32>(veh, 0x3C8, speed);
    let lnl = navi_link(u32::from(rd::<u16>(veh, 0x3A6)));
    let lcl = navi_link(u32::from(rd::<u16>(veh, 0x3A4)));
    let lo2_d2x = x(lo2).fmul_m(d2x).to_f32();
    let end_y = xi(i32::from(rd::<i16>(lnl, 2))).fmul_m(0.125).fsub_m(lo2_d2x);
    let lo2_d2y = x(lo2).fmul_m(d2y).to_f32();
    let end_x = xi(i32::from(rd::<i16>(lnl, 0))).fmul_m(0.125).fadd_m(lo2_d2y);
    let mut end = [end_x.to_f32(), end_y.to_f32(), 0.0];
    let lo1_d1x = x(lo1).fmul_m(d1x).to_f32();
    let start_y = xi(i32::from(rd::<i16>(lcl, 2))).fmul_m(0.125).fsub_m(lo1_d1x);
    let lo1_d1y = x(lo1).fmul_m(d1y).to_f32();
    let start_x = xi(i32::from(rd::<i16>(lcl, 0))).fmul_m(0.125).fadd_m(lo1_d1y);
    let mut start = [start_x.to_f32(), start_y.to_f32(), 0.0];
    let scale = X87::from_f64_exact(calc_speed_scale_factor(
        GPtr::from_raw(start.as_mut_ptr()),
        GPtr::from_raw(end.as_mut_ptr()),
        d1x,
        d1y,
        d2x,
        d2y,
    ));
    let ss = ftol32(scale.fmul(x(1000.0).fdiv_m(rd::<f32>(veh, 0x3C8))));
    wr::<i32>(veh, 0x3A0, ss);
    let ss_f = xi(ss);
    let u32f = |v: u32| {
        let f = xi(v as i32);
        if (v as i32) < 0 {
            f.fadd_m(4_294_967_296.0)
        } else {
            f
        }
    };
    let t0 = ftol32(u32f(now()).fsub(x(frac2).fmul(ss_f)));
    wr::<i32>(veh, 0x39C, t0);
    let mut d1v = [d1x, d1y, 0.0];
    let mut d2v = [d2x, d2y, 0.0];
    let mut end2 = end;
    let mut start2 = start;
    let tt = u32f(now().wrapping_sub(t0 as u32)).fdiv(ss_f).to_f32();
    let mut speed_out = [0f32; 3];
    calc_curve_point(
        GPtr::from_raw(start2.as_mut_ptr()),
        GPtr::from_raw(end2.as_mut_ptr()),
        GPtr::from_raw(d1v.as_mut_ptr()),
        GPtr::from_raw(d2v.as_mut_ptr()),
        tt,
        ss,
        cp,
        GPtr::from_raw(speed_out.as_mut_ptr()),
    );
    // Block I: final position
    let node1 = nodes(n1).byte_add(n1node as isize);
    let node2 = nodes(n2).byte_add(n2node as isize);
    let (mut r1, mut r2, mut r3) = ([0f32; 3], [0f32; 3], [0f32; 3]);
    let pn2 = get_position(node2, GPtr::from_raw(r1.as_mut_ptr()));
    let pn1 = get_position(node1, GPtr::from_raw(r2.as_mut_ptr()));
    let dv = vector_sub(GPtr::from_raw(r3.as_mut_ptr()), pn1, pn2);
    let mut dir = [
        dv.read(),
        dv.wrapping_index(1).read(),
        dv.wrapping_index(2).read(),
    ];
    let dirp = GPtr::from_raw(dir.as_mut_ptr());
    let mag = X87::from_f64_exact(vector_magnitude(dirp));
    let sc = x(2.0).fdiv(mag).to_f32();
    let (mut r4, mut r5) = ([0f32; 3], [0f32; 3]);
    let scaled = vector_scale(GPtr::from_raw(r4.as_mut_ptr()), dirp, sc);
    let pv = vector_add(GPtr::from_raw(r5.as_mut_ptr()), cp, scaled);
    let mut pp = [
        pv.read(),
        pv.wrapping_index(1).read(),
        pv.wrapping_index(2).read(),
    ];
    let a_z = x(1.0).fsub_m(frac[0]).fmul_m(node_pos(node1)[2]).to_f32();
    let mut gz = f32::from_bits(0x4E6E_6B28);
    pp[2] = x(frac[0]).fmul_m(node_pos(node2)[2]).fadd_m(a_z).to_f32();
    let ppp = GPtr::from_raw(pp.as_mut_ptr());
    if is_boat {
        let mut wlv = [0f32; 1];
        if get_water_level(pp[0], pp[1], pp[2], GPtr::from_raw(wlv.as_mut_ptr()), 1, 0) == 0 {
            gorc_delete(veh);
            return;
        }
        gz = wlv[0];
    } else {
        let mut cpt = [0u32; 12];
        let mut ent = [0u32; 1];
        let cpp = GPtr::<u8>::from_raw(cpt.as_mut_ptr().cast());
        let entp = GPtr::from_raw(ent.as_mut_ptr());
        if process_vertical_line_g(ppp, 1000.0, cpp, entp, 1, 0, 0, 0, 1, 0, 0) != 0 {
            gz = rd::<f32>(cpp, 8);
        }
        if process_vertical_line_g(ppp, -1000.0, cpp, entp, 1, 0, 0, 0, 1, 0, 0) != 0 {
            let a = abs_fchs(x(rd::<f32>(cpp, 8)).fsub_m(pp[2]));
            let b = abs_fchs(x(gz).fsub_m(pp[2]));
            if !jp5(a, b) {
                gz = rd::<f32>(cpp, 8);
            }
        }
    }
    if !x(gz).fcom(x(f32::from_bits(0x4E6E_6B28))).test_parity(0x44) {
        gorc_delete(veh);
        return;
    }
    if !abs_fchs(x(gz).fsub_m(pp[2])).fcom(x(7.0)).test_any(0x41) {
        gorc_delete(veh);
        return;
    }
    if is_boat_model(i32::from(rd::<i16>(veh, 0x22))) {
        ppp.wrapping_index(2).write(gz);
        wr::<u16>(veh, 0x4A6, 0xFF);
    } else {
        ppp.wrapping_index(2).write(
            X87::from_f64_exact(vehicle_get_height_above_road(veh))
                .fadd_m(gz)
                .to_f32(),
        );
    }
    placeable_set_posn_v(veh, ppp);
    wr::<f32>(veh, 0x44, 0.0);
    wr::<f32>(veh, 0x48, 0.0);
    wr::<f32>(veh, 0x4C, 0.0);
    let mut spd_l = spd;
    let (mut r6, mut r7) = ([0f32; 3], [0f32; 3]);
    let sdiv = vector_div(
        GPtr::from_raw(r6.as_mut_ptr()),
        GPtr::from_raw(speed_out.as_mut_ptr()),
        60.0,
    );
    let rel = vector_sub(
        GPtr::from_raw(r7.as_mut_ptr()),
        sdiv,
        GPtr::from_raw(spd_l.as_mut_ptr()),
    );
    let cp2x = x(coors[0]).fsub_m(cen[0]).to_f32();
    let (relx, rely) = (rel.read(), rel.wrapping_index(1).read());
    let cp2y = x(coors[1]).fsub_m(cen[1]).to_f32();
    let t = ty[0];
    let st = rd::<u8>(veh, 0x36);
    let status = |v: u8| wr::<u8>(veh, 0x36, (rd::<u8>(veh, 0x36) & 7) | v);
    if t == 13 {
        status(if rd::<u8>(veh, 0x3BA) == 1 { 0x10 } else { 0x18 });
    } else if t == 0x18 || is_boat {
        status(0x18);
    } else if st & 0xF8 != 0x18 {
        status(0x10);
    }
    set_clump_alpha(rd::<GPtr<u8>>(veh, 0x18), 0);
    if g::<u8>(0x969176).read() != 0 && rd::<u32>(veh, 0x590) == 0 {
        vehicle_add_upgrade(veh, i32::from(g::<u16>(0x8CD76C).read()));
    }
    // Block J: visibility and acceptance
    let k = g::<f32>(0xB6F11C).read();
    let ext = xi(i32::from(rd::<i16>(veh, 0x4A6))).to_f32();
    if entity_get_is_on_screen(veh) != 0 {
        let dp = len2d_sub(GPtr::from_raw(cen.as_mut_ptr()), pos_of(veh)).to_f32();
        let vmax = X87::from_f64_exact(float_max(170.0, ext));
        if !vmax.fmul_m(k).fcom(x(dp)).test_parity(0x05) {
            gorc_delete(veh);
            return;
        }
        if !x(k).fmul_m(150.0).fcom(x(dp)).test_any(0x41) {
            gorc_delete(veh);
            return;
        }
        let cm = g::<GPtr<u8>>(0xB6F03C).read();
        let camp: GPtr<f32> = if cm.is_null() {
            g(0xB6F02C)
        } else {
            cm.byte_add(0x30).cast()
        };
        let dc = len2d_sub(camp, pos_of(veh));
        if !x(k).fmul_m(120.0).fcom(dc).test_any(0x41) || top_down || rd::<u16>(veh, 0x22) == 0x1E4 {
            gorc_delete(veh);
            return;
        }
    } else {
        let dp = len2d_sub(GPtr::from_raw(cen.as_mut_ptr()), pos_of(veh)).to_f32();
        let v = X87::from_f64_exact(float_max(ext, 170.0))
            .fmul_m(f32::from_bits(0x3BC0_C0C1))
            .fmul_m(45.0);
        if !jp5(v, x(dp)) && !top_down {
            gorc_delete(veh);
            return;
        }
    }
    let r = rd::<f32>(
        rd::<GPtr<u8>>(model_info(i32::from(rd::<i16>(veh, 0x22))), 0x14),
        0x24,
    );
    find_objects_kinda_colliding(
        pos_of(veh),
        r,
        1,
        GPtr::from_raw(cnt.as_mut_ptr()),
        2,
        0,
        0,
        1,
        1,
        0,
        0,
    );
    if cnt[0] != 0 {
        gorc_delete(veh);
        return;
    }
    if jp5(x(relx).fmul_m(cp2x).fadd(x(rely).fmul_m(cp2y)), x(0.0)) {
        gorc_delete(veh);
        return;
    }
    choose_vehicle_colour(
        model_info(i32::from(rd::<i16>(veh, 0x22))),
        veh.byte_add(0x434),
        veh.byte_add(0x435),
        veh.byte_add(0x436),
        veh.byte_add(0x437),
        1,
    );
    world_add(veh);
    // Block K: post-add
    let vm = rd::<u16>(veh, 0x22);
    if vm == 0x213 || vm == 0x214 || rd::<u32>(veh, 0x594) == 10 {
        wr::<u8>(veh, 0x3D0, (i32::from(rd::<u8>(veh, 0x3D0)) / 3) as u8);
    }
    if la_riots_active_here() != 0 {
        let h = rand_at(0x431C30) % 1000;
        wr::<f32>(veh, 0x4C0, xi(h).to_f32());
    }
    if t == 13 {
        g::<u32>(0x9690B8).write(now());
    }
    if rd::<u16>(veh, 0x22) == 0x1C9 {
        status(0x18);
        wr::<u8>(veh, 0x3B9, 2);
    }
    if rd::<u32>(veh, 0x590) == 0 && (0..=0x17).contains(&t) {
        const DAMAGE: [u8; 24] = [
            0, 1, 2, 2, 0, 0, 0, 2, 2, 2, 2, 2, 2, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        ];
        match DAMAGE[t as usize] {
            0 if random_int_in_range(0, 0x14) == 0 => automobile_set_random_damage(veh, 0),
            1 if random_int_in_range(0, 8) == 0 => automobile_set_random_damage(veh, 1),
            _ => {}
        }
    }
    if rd::<u32>(veh, 0x594) == 9 && rd::<u8>(veh, 0x3B9) == 0 {
        status(0x18);
        wr::<u8>(veh, 0x3B9, 6);
    }
    let chase = !is_boat
        && t != 13
        && player_get_wanted_level(find_player_ped(-1)) == 0
        && (g::<u8>(0x96914F).read() != 0 || !x(g::<f32>(0x9690BC).read()).fcom(x(0.0)).test_parity(0x41))
        && !gang_zone
        && create_police_chase(veh, t, n1) != 0;
    if chase {
        let v = if la_riots_active_here() != 0 {
            random_float_in_range(240.0, 480.0)
        } else {
            random_float_in_range(600.0, 1200.0)
        };
        g::<f32>(0x9690BC).write(X87::from_f64_exact(v).to_f32());
    } else if mad {
        let convoy_model = matches!(vm, 0x1CF | 0x1CD | 0x209 | 0x20A | 0x245 | 0x24A);
        if convoy_model
            && g::<u8>(0xB72958).read() == 0
            && random_int_in_range(0, 7) == 0
            && create_convoy(veh, t) != 0
        {
            set_up_driver_and_passengers_for_vehicle(veh, t, 1, 1, 0, 0x63);
        } else {
            set_up_driver_and_passengers_for_vehicle(veh, t, 1, 1, 0, 0x63);
            status(0x18);
            wr::<u8>(veh, 0x3B9, 2);
            let cr = ftol32(xi(i32::from(rd::<u8>(veh, 0x3D0))).fadd_m(10.0)) as u8;
            wr::<u8>(veh, 0x3D0, cr);
            let crf = xi(i32::from(cr)).to_f32();
            let (mut r8, mut r9, mut r10) = ([0f32; 3], [0f32; 3], [0f32; 3]);
            let top = placeable_get_top_direction(veh, GPtr::from_raw(r8.as_mut_ptr()));
            let a1 = vector_scale(GPtr::from_raw(r9.as_mut_ptr()), top, crf);
            let a2 = vector_scale(GPtr::from_raw(r10.as_mut_ptr()), a1, f32::from_bits(0x3CA3_D70A));
            wr::<f32>(veh, 0x44, a2.read());
            wr::<f32>(veh, 0x48, a2.wrapping_index(1).read());
            wr::<f32>(veh, 0x4C, a2.wrapping_index(2).read());
            if la_riots_active_here() != 0 || g::<u8>(0x96914F).read() != 0 {
                let drv = rd::<GPtr<u8>>(veh, 0x460);
                if !drv.is_null() {
                    wr::<u32>(drv, 0x478, rd::<u32>(drv, 0x478) | 0x800);
                }
            }
            wr::<u8>(veh, 0x42E, rd::<u8>(veh, 0x42E) | 8);
        }
    } else if t == 13 || t == 0x18 {
        add_police_car_occupants(veh, 0);
    } else {
        g::<u8>(0x9690CC).write(1);
        set_up_driver_and_passengers_for_vehicle(veh, t, 0, 0, 0, 0x63);
        g::<u8>(0x9690CC).write(0);
    }
    if t == 13 || t == 0x18 {
        change_law_enforcer_state(veh, 1);
    }
    possibly_stream_car_out_after_creation(i32::from(rd::<i16>(veh, 0x22)));
    model_info_use_counter(model_info(i32::from(rd::<i16>(veh, 0x22))));
}

orig! {
    /// `CCarCtrl::GenerateOneEmergencyServicesCar(uint32, CVector)` (0x42B7D0)
    fn generate_one_emergency_services_car(model: u32, x: f32, y: f32, z: f32) -> GPtr<u8> = cdecl 0x42B7D0 => unported;
    /// `CCarCtrl::JoinCarWithRoadSystemGotoCoors(CVehicle*, const CVector&, bool, bool)` (0x42F870)
    fn join_car_with_road_system_goto_coors(v: GPtr<u8>, c: GPtr<f32>, a: u32, b: u32) -> u8 = cdecl 0x42F870 => unported;
}

/// `CCarCtrl::ScriptGenerateOneEmergencyServicesCar(uint32 model, CVector target)` (stub 0x42FBC0 -> 0x1570100,
/// AL): false unless the model is loaded; `GenerateOneEmergencyServicesCar(model, target)` (false on null);
/// autopilot destination (+0x3EC) = target; mission (+0x3BA) = 9 when `JoinCarWithRoadSystemGotoCoors(veh,
/// &target, false, k)` succeeds, else 8. `k` is pushed through an obfuscated pair: `[0x15605D4] - [0x6A3589]`
/// with CF clear = 0xFFD66F0B - 0xFFD66F0B = 0 (decoded from the static image). The original passes the address
/// of its own argument slot as `&target`; the port passes a local copy.
#[hook(addr = 0x42FBC0, conv = cdecl, name = "CCarCtrl::ScriptGenerateOneEmergencyServicesCar", expect = "E9 3B 05 14 01")]
pub fn script_generate_one_emergency_services_car(model: u32, tx: f32, ty: f32, tz: f32) -> u8 {
    if !model_loaded(model) {
        return 0;
    }
    let veh = generate_one_emergency_services_car(model, tx, ty, tz);
    if veh.is_null() {
        return 0;
    }
    wr::<f32>(veh, 0x3EC, tx);
    wr::<f32>(veh, 0x3F0, ty);
    wr::<f32>(veh, 0x3F4, tz);
    let mut t = [tx, ty, tz];
    let r = join_car_with_road_system_goto_coors(veh, GPtr::from_raw(t.as_mut_ptr()), 0, 0);
    wr::<u8>(veh, 0x3BA, if r != 0 { 9 } else { 8 });
    1
}

orig! {
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660, ST0)
    fn find_ground_z_for_coord(x: f32, y: f32) -> f64 = cdecl 0x569660 => unported;
    /// `CEntity::GetDistanceFromCentreOfMassToBaseOfModel()` (0x536BE0, ST0)
    fn get_distance_from_centre_of_mass_to_base_of_model(e: GPtr<u8>) -> f64 = thiscall 0x536BE0 => unported;
    /// `CTheScripts::ClearSpaceForMissionEntity(const CVector&, CEntity*)` (0x486B00)
    fn clear_space_for_mission_entity(p: GPtr<f32>, e: GPtr<u8>) = cdecl 0x486B00 => unported;
    /// `CCarCtrl::JoinCarWithRoadSystem(CVehicle*)` (0x42F5A0)
    fn join_car_with_road_system(v: GPtr<u8>) = cdecl 0x42F5A0 => unported;
    /// `CMissionCleanup::AddEntityToList(int32 handle, uint8 type)` (0x4637E0, `ret 8`)
    fn mission_cleanup_add_entity_to_list(this: GPtr<u8>, handle: i32, ty: u32) = thiscall 0x4637E0 => unported;
    /// `CAutomobile::PlaceOnRoadProperly()` (0x6AF420)
    fn automobile_place_on_road_properly(v: GPtr<u8>) = thiscall 0x6AF420 => unported;
    /// `CBike::PlaceOnRoadProperly()` (0x6BEEB0)
    fn bike_place_on_road_properly(v: GPtr<u8>) = thiscall 0x6BEEB0 => unported;
}

/// The vehicle pool handle `(index << 8) | flags[index]` (index = (veh - objects) / 0xA18).
fn vehicle_handle(veh: GPtr<u8>) -> i32 {
    let pool = g::<GPtr<u8>>(0xB74494).read();
    let index = (veh.addr() as i32).wrapping_sub(rd::<i32>(pool, 0)) / 0xA18;
    i32::from(rd::<GPtr<u8>>(pool, 4).wrapping_index(index).read()) + (index << 8)
}

/// `CCarCtrl::CreateCarForScript(int32 model, CVector pos, bool missionCleanup)` (0x431F80, SEH frame): boats are
/// `new CBoat(model, 2)` (null not checked), others `GetNewVehicleDependingOnCarModel(model, 2)`; z <= -100
/// (`test ah, 0x41; jp`) takes `FindGroundZForCoord(x, y)`; z += `GetDistanceFromCentreOfMassToBaseOfModel()`;
/// position set (matrix or placement); non-boats without mission cleanup are placed on the road (automobiles
/// / bikes), trains (+0x590 == 6) get +0x5B9 bit 0; `ClearSpaceForMissionEntity(&pos, veh)`; status 4 (abandoned),
/// +0x428 bit 3 (non-boats clear bit 4 after `JoinCarWithRoadSystem`, boats before), autopilot defaults (boats:
/// speed 20, cruise 20; others: speed 13, cruise 13, style 0, lanes 0, +0x42A bit 1), +0x1C bit 18 with mission
/// cleanup, `CWorld::Add`, `CMissionCleanup::AddEntityToList(handle, 1)` with mission cleanup; +0x3DC bit 0 for
/// boats and for non-boats that are not helis / planes / trains (+0x594 3, 4, 6). The original passes the
/// address of its own argument slots (updated z) to ClearSpaceForMissionEntity; the port passes a local copy.
#[hook(addr = 0x431F80, conv = cdecl, name = "CCarCtrl::CreateCarForScript", expect = "6A FF 68 BB 74 83 00")]
pub fn create_car_for_script(model: i32, px: f32, py: f32, pz: f32, mission_cleanup: u8) -> GPtr<u8> {
    let boat = is_boat_model(model);
    let veh = if boat {
        let m = vehicle_new(0x7E8);
        if m.is_null() { m } else { boat_ctor(m, model, 2) }
    } else {
        get_new_vehicle_call(model, 2)
    };
    let mut z = pz;
    if !x(z).fcom(x(-100.0)).test_parity(0x41) {
        z = X87::from_f64_exact(find_ground_z_for_coord(px, py)).to_f32();
    }
    z = X87::from_f64_exact(get_distance_from_centre_of_mass_to_base_of_model(veh))
        .fadd_m(z)
        .to_f32();
    let m = rd::<GPtr<u8>>(veh, 0x14);
    if m.is_null() {
        wr::<f32>(veh, 8, py);
        wr::<f32>(veh, 4, px);
        wr::<f32>(veh, 0xC, z);
    } else {
        wr::<f32>(m, 0x30, px);
        wr::<f32>(rd::<GPtr<u8>>(veh, 0x14), 0x34, py);
        wr::<f32>(rd::<GPtr<u8>>(veh, 0x14), 0x38, z);
    }
    if !boat && mission_cleanup == 0 {
        match rd::<u32>(veh, 0x590) {
            0 => automobile_place_on_road_properly(veh),
            9 => bike_place_on_road_properly(veh),
            _ => {}
        }
    }
    if !boat && rd::<u32>(veh, 0x590) == 6 {
        wr::<u8>(veh, 0x5B9, rd::<u8>(veh, 0x5B9) | 1);
    }
    let mut pos = [px, py, z];
    clear_space_for_mission_entity(GPtr::from_raw(pos.as_mut_ptr()), veh);
    wr::<u8>(veh, 0x36, (rd::<u8>(veh, 0x36) & 7) | 0x20);
    if boat {
        wr::<u8>(veh, 0x428, (rd::<u8>(veh, 0x428) & 0xEF) | 8);
        join_car_with_road_system(veh);
        wr::<u8>(veh, 0x3BA, 0);
        wr::<u8>(veh, 0x3BB, 0);
        wr::<f32>(veh, 0x3C8, 20.0);
        wr::<u8>(veh, 0x3D0, 0x14);
        if mission_cleanup != 0 {
            wr::<u32>(veh, 0x1C, rd::<u32>(veh, 0x1C) | 0x40000);
        }
        wr::<u8>(veh, 0x3DC, rd::<u8>(veh, 0x3DC) | 1);
        world_add(veh);
        if mission_cleanup != 0 {
            mission_cleanup_add_entity_to_list(g(0xA90850), vehicle_handle(veh), 1);
        }
        return veh;
    }
    wr::<u8>(veh, 0x428, rd::<u8>(veh, 0x428) | 8);
    join_car_with_road_system(veh);
    wr::<u8>(veh, 0x3BA, 0);
    wr::<u8>(veh, 0x3BB, 0);
    wr::<u8>(veh, 0x3B9, 0);
    wr::<f32>(veh, 0x3C8, 13.0);
    wr::<u8>(veh, 0x3D0, 0xD);
    wr::<u8>(veh, 0x3B7, 0);
    wr::<u8>(veh, 0x3B8, 0);
    wr::<u8>(veh, 0x428, rd::<u8>(veh, 0x428) & 0xEF);
    wr::<u8>(veh, 0x42A, rd::<u8>(veh, 0x42A) | 2);
    if mission_cleanup != 0 {
        wr::<u32>(veh, 0x1C, rd::<u32>(veh, 0x1C) | 0x40000);
    }
    world_add(veh);
    if mission_cleanup != 0 {
        mission_cleanup_add_entity_to_list(g(0xA90850), vehicle_handle(veh), 1);
    }
    if !matches!(rd::<u32>(veh, 0x594), 3 | 4 | 6) {
        wr::<u8>(veh, 0x3DC, rd::<u8>(veh, 0x3DC) | 1);
    }
    veh
}

orig! {
    /// `CEntity` vtable slot 0x10 with one argument (0)
    fn entity_vslot_10(e: GPtr<u8>, a: u32) = thiscall vslot 0x10 => unported;
    /// `CStuckCarCheck::ClearStuckFlagForCar(int32 handle)` (0x463C40, `ret 4`)
    fn stuck_car_check_clear_stuck_flag_for_car(this: GPtr<u8>, handle: i32) = thiscall 0x463C40 => unported;
    /// `CEntity` vtable slot 0x38 `Teleport(CVector, bool)` (`ret 0x10`)
    fn entity_teleport(e: GPtr<u8>, x: f32, y: f32, z: f32, reset_rot: u32) = thiscall vslot 0x38 => unported;
    /// `CCarCtrl::JoinCarWithRoadAccordingToMission(CVehicle*)` (0x432CB0)
    fn join_car_with_road_according_to_mission(v: GPtr<u8>) = cdecl 0x432CB0 => unported;
}

/// `CCarCtrl::SetCoordsOfScriptCar(CVehicle* veh, float x, float y, float z, bool resetRotation, bool
/// addBaseDistance)` (0x4342A0): z <= -100 (`test ah, 0x41; jp`) takes `FindGroundZForCoord(x, y)`; with
/// `addBaseDistance` z += `GetDistanceFromCentreOfMassToBaseOfModel()`; vtable slot 0x10 (0);
/// `CStuckCarCheck::ClearStuckFlagForCar(handle)`; `Teleport((x, y, z), resetRotation)` (vtable 0x38); trains
/// (+0x590 == 5) then only `ClearSpaceForMissionEntity`; others are placed on the road (automobiles 0 / 11,
/// bikes 9), `ClearSpaceForMissionEntity`, `JoinCarWithRoadAccordingToMission`; temp action (+0x3BB) = 0.
#[hook(addr = 0x4342A0, conv = cdecl, name = "CCarCtrl::SetCoordsOfScriptCar", expect = "D9 44 24 10 83 EC 0C")]
pub fn set_coords_of_script_car(veh: GPtr<u8>, px: f32, py: f32, pz: f32, reset_rotation: u32, add_base: u8) {
    let handle = vehicle_handle(veh);
    let mut z = pz;
    if !x(z).fcom(x(-100.0)).test_parity(0x41) {
        z = X87::from_f64_exact(find_ground_z_for_coord(px, py)).to_f32();
    }
    if add_base != 0 {
        z = X87::from_f64_exact(get_distance_from_centre_of_mass_to_base_of_model(veh))
            .fadd_m(z)
            .to_f32();
    }
    entity_vslot_10(veh, 0);
    stuck_car_check_clear_stuck_flag_for_car(g(0xA90AB0), handle);
    let train = rd::<u32>(veh, 0x590) == 5;
    entity_teleport(veh, px, py, z, reset_rotation);
    let mut pos = [px, py, z];
    if train {
        clear_space_for_mission_entity(GPtr::from_raw(pos.as_mut_ptr()), veh);
    } else {
        match rd::<u32>(veh, 0x590) {
            0 | 11 => automobile_place_on_road_properly(veh),
            9 => bike_place_on_road_properly(veh),
            _ => {}
        }
        clear_space_for_mission_entity(GPtr::from_raw(pos.as_mut_ptr()), veh);
        join_car_with_road_according_to_mission(veh);
    }
    wr::<u8>(veh, 0x3BB, 0);
}

orig! {
    /// `CCarAI::AddAmbulanceOccupants(CVehicle*)` (0x41C4A0)
    fn add_ambulance_occupants(v: GPtr<u8>) = cdecl 0x41C4A0 => unported;
    /// `CCarAI::AddFiretruckOccupants(CVehicle*)` (0x41C600)
    fn add_firetruck_occupants(v: GPtr<u8>) = cdecl 0x41C600 => unported;
}

/// `CCarCtrl::GenerateOneEmergencyServicesCar(uint32 model, CVector target)` (0x42B7D0, entry `push -1; jmp
/// 0x4018AB`, whose stub pushes the SEH handler computed from an obfuscated pair and jumps back to 0x42B7D7; spec
/// CCarCtrl.p2.md, checked against the bytes): up to five `GenerateCarCreationCoors2(player centre, 0.707, 0.707,
/// -1.0, true, 160, 160, ..., false, false)` attempts whose first node is not a water node and whose point has
/// no vehicle / ped within 10.0 (a hit on the fifth attempt is discarded, original quirk); `new CAutomobile(model,
/// 1, true)` (null not checked); position; orientation facing `target` (2D, `1/len` stored f32; a zero length
/// gives (1, dy)); z interpolated between the node heights by `frac`, then the ground of two vertical lines
/// (deleted when none); base distance added, speed cleared, `PlaceOnRoadProperly`, `CWorld::Add`; fire trucks
/// (407) / ambulances (416) set their on-duty flag, count and occupants, police models (427, 523, 596..599) get
/// police occupants, and those three kinds set +0x42D bit 7.
// Unhookable: protection code reads its entry bytes as data (`mov ecx, [0x42B7D3]` at 0x403794; sa-hook
// every_hook_installs_on_the_exe / keycells / patch_refs).
pub fn generate_one_emergency_services_car_port(model: u32, tx: f32, ty: f32, _tz: f32) -> GPtr<u8> {
    // (the original also stores `target.z - coors.z` at [E+0x40]; nothing reads it)
    let c = find_player_centre_of_world(i32::from(g::<u8>(0xB7CD74).read())).cast::<f32>();
    let cen = [c.read(), c.wrapping_index(1).read(), c.wrapping_index(2).read()];
    let mut coors = [0f32; 3];
    let mut n1 = [0x0000_FFFFu32; 1];
    let mut n2 = [0x0000_FFFFu32; 1];
    let mut frac = [0f32; 1];
    let cp = GPtr::<f32>::from_raw(coors.as_mut_ptr());
    let k = f32::from_bits(0x3F34_FDF4);
    let mut i: i16 = 0;
    loop {
        if i >= 5 {
            return GPtr::NULL;
        }
        let mut found = false;
        if generate_car_creation_coors2_call(
            cen[0],
            cen[1],
            cen[2],
            k,
            k,
            -1.0,
            1,
            160.0,
            160.0,
            cp,
            GPtr::from_raw(n1.as_mut_ptr()),
            GPtr::from_raw(n2.as_mut_ptr()),
            GPtr::from_raw(frac.as_mut_ptr()),
            0,
            0,
        ) != 0
            && rd::<u8>(node_of(n1[0]), 0x18) & 0x80 == 0
        {
            let mut cnt = [0i16; 2];
            find_objects_kinda_colliding(cp, 10.0, 1, GPtr::from_raw(cnt.as_mut_ptr()), 2, 0, 0, 1, 1, 0, 0);
            found = cnt[0] == 0;
        }
        i += 1;
        if found {
            break;
        }
    }
    if i >= 5 {
        return GPtr::NULL;
    }
    let m = vehicle_new(0x988);
    let veh = if m.is_null() {
        m
    } else {
        automobile_ctor(m, model as i32, 1, 1)
    };
    let mt = rd::<GPtr<u8>>(veh, 0x14);
    if mt.is_null() {
        wr::<f32>(veh, 8, coors[1]);
        wr::<f32>(veh, 4, coors[0]);
        wr::<f32>(veh, 0xC, coors[2]);
    } else {
        wr::<f32>(mt, 0x30, coors[0]);
        wr::<f32>(rd::<GPtr<u8>>(veh, 0x14), 0x34, coors[1]);
        wr::<f32>(rd::<GPtr<u8>>(veh, 0x14), 0x38, coors[2]);
    }
    let dx = x(tx).fsub_m(coors[0]).to_f32();
    let dy = x(ty).fsub_m(coors[1]).to_f32();
    let len = x(dy).fmul_m(dy).fadd(x(dx).fmul_m(dx)).fsqrt();
    let (fx, fy) = if !len.fcom(x(0.0)).test_parity(0x44) {
        (x(1.0), x(dy))
    } else {
        let inv = x(1.0).fdiv(len).to_f32();
        (x(inv).fmul_m(dx), x(inv).fmul_m(dy))
    };
    let mat = |o: isize, v: f32| rd::<GPtr<u8>>(veh, 0x14).byte_add(o).cast::<f32>().write(v);
    mat(0x10, fx.to_f32());
    mat(0x14, fy.to_f32());
    mat(0x18, 0.0);
    mat(0, fy.to_f32());
    mat(4, fx.fchs().to_f32());
    mat(8, 0.0);
    mat(0x20, 0.0);
    mat(0x24, 0.0);
    mat(0x28, 1.0);
    let z1 = xi(i32::from(rd::<i16>(node_of(n1[0]), 0xC))).fmul_m(0.125);
    let z2 = xi(i32::from(rd::<i16>(node_of(n2[0]), 0xC))).fmul_m(0.125);
    coors[2] = x(1.0).fsub_m(frac[0]).fmul(z1).fadd(z2.fmul_m(frac[0])).to_f32();
    let mut gz = f32::from_bits(0x4E6E_6B28);
    let mut cpt = [0u32; 12];
    let mut ent = [0u32; 1];
    let cpp = GPtr::<u8>::from_raw(cpt.as_mut_ptr().cast());
    let entp = GPtr::from_raw(ent.as_mut_ptr());
    let cp = GPtr::<f32>::from_raw(coors.as_mut_ptr());
    if process_vertical_line_g(cp, 1000.0, cpp, entp, 1, 0, 0, 0, 1, 0, 0) != 0 {
        gz = rd::<f32>(cpp, 8);
    }
    if process_vertical_line_g(cp, -1000.0, cpp, entp, 1, 0, 0, 0, 1, 0, 0) != 0 {
        let cz = cp.wrapping_index(2).read();
        let a = abs_fchs(x(rd::<f32>(cpp, 8)).fsub_m(cz));
        let b = abs_fchs(x(gz).fsub_m(cz));
        if !jp5(a, b) {
            gz = rd::<f32>(cpp, 8);
        }
    }
    if !x(gz).fcom(x(f32::from_bits(0x4E6E_6B28))).test_parity(0x44) {
        entity_vdelete(veh, 1);
        return GPtr::NULL;
    }
    let z = X87::from_f64_exact(get_distance_from_centre_of_mass_to_base_of_model(veh))
        .fadd_m(gz)
        .to_f32();
    let (px, py) = (cp.read(), cp.wrapping_index(1).read());
    cp.wrapping_index(2).write(z);
    let mt = rd::<GPtr<u8>>(veh, 0x14);
    if mt.is_null() {
        wr::<f32>(veh, 8, py);
        wr::<f32>(veh, 4, px);
        wr::<f32>(veh, 0xC, z);
    } else {
        wr::<f32>(mt, 0x30, px);
        wr::<f32>(rd::<GPtr<u8>>(veh, 0x14), 0x34, py);
        wr::<f32>(rd::<GPtr<u8>>(veh, 0x14), 0x38, z);
    }
    wr::<f32>(veh, 0x44, 0.0);
    wr::<f32>(veh, 0x48, 0.0);
    wr::<f32>(veh, 0x4C, 0.0);
    automobile_place_on_road_properly(veh);
    world_add(veh);
    match model {
        407 => {
            wr::<u8>(veh, 0x428, rd::<u8>(veh, 0x428) | 4);
            let n = g::<i32>(0x9690AC);
            n.write(n.read().wrapping_add(1));
            add_firetruck_occupants(veh);
        }
        416 => {
            wr::<u8>(veh, 0x428, rd::<u8>(veh, 0x428) | 2);
            let n = g::<i32>(0x9690A8);
            n.write(n.read().wrapping_add(1));
            add_ambulance_occupants(veh);
        }
        427 | 523 | 596..=599 => add_police_car_occupants(veh, 0),
        _ => return veh,
    }
    wr::<u8>(veh, 0x42D, rd::<u8>(veh, 0x42D) | 0x80);
    veh
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GET_NEW_VEHICLE_DEPENDING_ON_CAR_MODEL,
    HOOK_CHOOSE_MODEL,
    HOOK_GENERATE_CAR_CREATION_COORS2,
    HOOK_POSSIBLY_REMOVE_VEHICLE,
    HOOK_GENERATE_ONE_RANDOM_CAR,
    HOOK_SCRIPT_GENERATE_ONE_EMERGENCY_SERVICES_CAR,
    HOOK_CREATE_CAR_FOR_SCRIPT,
    HOOK_SET_COORDS_OF_SCRIPT_CAR,
];
