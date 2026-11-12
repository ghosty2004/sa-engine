//! `CCover::Update` (batch `std7-ai-1`): cover point upkeep and the per-frame scans that add vehicle,
//! object and building cover points. Ported from the 1.0 US bytes; unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CReplay::Mode`
const REPLAY_MODE: usize = 0xA43088;
/// `CCover::m_aPoints` (100 x 0x1C; +0 type (int8), +0x10 entity) and `CCover::m_NumPoints`
const COVER_POINTS: usize = 0xC197C8;
const COVER_POINTS_END: usize = 0xC1A2B8;
const COVER_NUM_POINTS: usize = 0xC197A4;
/// `CCover::m_ListOfProcessedBuildings` (head of a `CPtrListDoubleLink`: node +0 item, +4 next, +8 prev)
const PROCESSED_BUILDINGS: usize = 0xC1A2B8;
/// `CTimer::m_FrameCounter`, `CWorld::ms_nCurrentScanCode`, the sectors (120 x 120, 8 bytes: buildings
/// list first)
const FRAME_COUNTER: usize = 0xB7CB4C;
const SCAN_CODE: usize = 0xB7CD78;
const SECTORS: usize = 0xB7D0B8;
/// The vehicle and object pools
const VEHICLE_POOL: usize = 0xB74494;
const OBJECT_POOL: usize = 0xB7449C;

orig! {
    fn find_player_coors(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E010 => unported;
    fn find_player_vehicle(id: i32, include_remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CObject::CanBeUsedToTakeCoverBehind()` (0x5A1B60)
    fn object_can_be_used_to_take_cover_behind(o: GPtr<u8>) -> u8 = thiscall 0x5A1B60 => unported;
    /// `CCover::ShouldThisBuildingHaveItsCoverPointsCreated(CBuilding*)` (0x699230)
    fn should_building_have_cover_points(b: GPtr<u8>) -> u8 = cdecl 0x699230 => unported;
    /// `CCover::RemoveCoverPointsForThisEntity(CEntity*)` (0x698740)
    fn remove_cover_points_for_entity(e: GPtr<u8>) = cdecl 0x698740 => unported;
    /// `CCover::FindCoverPointsForThisBuilding(CBuilding*)` (0x699120)
    fn find_cover_points_for_building(b: GPtr<u8>) = cdecl 0x699120 => unported;
    /// `CPtrList::IsMemberOfList(void*)` (0x5521D0)
    fn ptr_list_is_member(list: GPtr<u8>, item: GPtr<u8>) -> u8 = thiscall 0x5521D0 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470)
    fn world_clear_scan_codes() = cdecl 0x563470 => unported;
    /// `operator new(size)` (0x5523C0) / `operator delete(p)` (0x5523D0) of the pool-backed lists
    fn list_node_new(size: u32) -> GPtr<u8> = cdecl 0x5523C0 => unported;
    fn list_node_delete(p: GPtr<u8>) = cdecl 0x5523D0 => unported;
    /// CRT `floor(double)` (0x8219F0)
    fn floor(v: f64) -> f64 = cdecl 0x8219F0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
}

fn ldf(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// `|v|` of the vector at `p` (x87: `x*x + y*y + z*z`, `fsqrt`).
fn speed(p: GPtr<u8>) -> X87 {
    let (vx, vy, vz) = (x(rd::<f32>(p, 0)), x(rd::<f32>(p, 4)), x(rd::<f32>(p, 8)));
    vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt()
}

/// The 2D distance from the player to the entity's position (`(dy*dy + dx*dx)` then `fsqrt`).
fn player_distance_2d(e: GPtr<u8>) -> X87 {
    let m = rd::<GPtr<u8>>(e, 0x14);
    let pos = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) };
    let mut out = [0f32; 3];
    let pc = find_player_coors(GPtr::from_raw(out.as_mut_ptr()), -1).cast::<u8>();
    let dx = x(rd::<f32>(pc, 0)).fsub_m(rd::<f32>(pos, 0));
    let dy = x(rd::<f32>(pc, 4)).fsub_m(rd::<f32>(pos, 4));
    dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt()
}

/// One sector bound: `v = c * k + b` (x87; k `[0x858B38]`, b `[0x858B34]`) stored as f32; `ftol(floor(v))`
/// (the double of the register value) decides; the result is `ftol(floor(f32 v))`, clamped to 0 below
/// (`low`) or to 0x77 above.
fn sector_bound(c: f32, low: bool) -> i32 {
    let v = x(c).fmul_m(ldf(0x858B38)).fadd_m(ldf(0x858B34));
    let s = v.to_f32();
    let first = ftol32(X87::from_f64_exact(floor(v.to_f64())));
    let exact = || ftol32(X87::from_f64_exact(floor(f64::from(s))));
    if low {
        if first > 0 { exact() } else { 0 }
    } else if first < 0x77 {
        exact()
    } else {
        0x77
    }
}

/// `CCover::Update()` (0x6997E0, cdecl). Nothing in replay playback (`CReplay::Mode` 1). Every used
/// cover point (type 1: object, 2: vehicle, 3: other) is freed (type 0, count - 1) when its entity is
/// gone, an object's up.z (matrix allocated first when missing) is below `[0x858C20]`, or a vehicle's
/// speed is above `[0x858C58]`. Then by frame counter: `% 32 == 26`: each automobile of the vehicle pool
/// (descending, +0x590 type 0) slower than `[0x858B4C]` that provides cover (+0x42E bit 2) and within
/// `[0x858FE8]` (2D) of the player gets `AddCoverPoint(2, vehicle, 0, 0, 0xFF)`; `% 32 == 28`: each object
/// with up.z above `[0x858EF0]` that `CanBeUsedToTakeCoverBehind` and is near the player gets
/// `AddCoverPoint(1, ...)`; else `% 8 == 5`: processed buildings that should no longer have cover points
/// lose them and leave the list; then, on foot, a new scan code (`ClearScanCodes` at 0xFFFF) and the
/// buildings of the sectors around the player (`[0x858CA4]` margin) not yet scanned are marked; the first
/// that should have cover points and is not in the list gets them (`FindCoverPointsForThisBuilding`), is
/// pushed at the list head, and the function returns.
#[hook(addr = 0x6997E0, conv = cdecl, name = "CCover::Update", expect = "55 8B EC 83 E4 F8 83 EC 28",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn cover_update() {
    if g::<u8>(REPLAY_MODE).read() == 1 {
        return;
    }
    let mut p = COVER_POINTS;
    while p < COVER_POINTS_END {
        let pt = GPtr::<u8>::from_addr(p);
        let e = rd::<GPtr<u8>>(pt, 0x10);
        let free = match rd::<i8>(pt, 0) {
            1 => {
                if e.is_null() {
                    true
                } else {
                    if rd::<u32>(e, 0x14) == 0 {
                        allocate_matrix(e);
                        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
                    }
                    let upz = x(rd::<f32>(rd::<GPtr<u8>>(e, 0x14), 0x28));
                    !upz.fcom(x(ldf(0x858C20))).test_parity(5)
                }
            }
            2 => e.is_null() || !speed(e.byte_add(0x44)).fcom(x(ldf(0x858C58))).test_any(0x41),
            3 => e.is_null(),
            _ => false,
        };
        if free {
            let n = g::<u32>(COVER_NUM_POINTS);
            wr::<u8>(pt, 0, 0);
            n.write(n.read().wrapping_sub(1));
        }
        p += 0x1C;
    }
    let fc = g::<u32>(FRAME_COUNTER).read();
    match fc & 0x1F {
        0x1A => {
            let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
            let mut i = rd::<i32>(pool, 8);
            while i != 0 {
                i -= 1;
                if rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize) < 0 {
                    continue;
                }
                let v = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0xA18);
                if v.is_null() || rd::<u32>(v, 0x590) != 0 {
                    continue;
                }
                if speed(v.byte_add(0x44)).fcom(x(ldf(0x858B4C))).test_parity(5) {
                    continue;
                }
                if rd::<u8>(v, 0x42E) & 4 == 0 {
                    continue;
                }
                if player_distance_2d(v).fcom(x(ldf(0x858FE8))).test_parity(5) {
                    continue;
                }
                add_cover_point(2, v, GPtr::NULL, 0, 0xFF);
            }
            return;
        }
        0x1C => {
            let pool = g::<GPtr<u8>>(OBJECT_POOL).read();
            let mut i = rd::<i32>(pool, 8);
            while i != 0 {
                i -= 1;
                if rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize) < 0 {
                    continue;
                }
                let o = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0x19C);
                if o.is_null() {
                    continue;
                }
                let upz = x(rd::<f32>(rd::<GPtr<u8>>(o, 0x14), 0x28));
                if upz.fcom(x(ldf(0x858EF0))).test_any(0x41) {
                    continue;
                }
                if object_can_be_used_to_take_cover_behind(o) == 0 {
                    continue;
                }
                if player_distance_2d(o).fcom(x(ldf(0x858FE8))).test_parity(5) {
                    continue;
                }
                add_cover_point(1, o, GPtr::NULL, 0, 0xFF);
            }
            return;
        }
        _ => {}
    }
    if fc & 7 != 5 {
        return;
    }
    let head = g::<u32>(PROCESSED_BUILDINGS);
    let mut node = head.read();
    while node != 0 {
        let n = GPtr::<u8>::from_addr(node as usize);
        let item = rd::<GPtr<u8>>(n, 0);
        let next = rd::<u32>(n, 4);
        if should_building_have_cover_points(item) == 0 {
            remove_cover_points_for_entity(item);
            if head.read() == node {
                head.write(rd::<u32>(n, 4));
            }
            let prev = rd::<u32>(n, 8);
            if prev != 0 {
                g::<u32>(prev as usize + 4).write(rd::<u32>(n, 4));
            }
            let nx = rd::<u32>(n, 4);
            if nx != 0 {
                g::<u32>(nx as usize + 8).write(rd::<u32>(n, 8));
            }
            list_node_delete(n);
        }
        node = next;
    }
    if !find_player_vehicle(-1, 0).is_null() {
        return;
    }
    let margin = ldf(0x858CA4);
    let coor = |i: isize| {
        let mut out = [0f32; 3];
        rd::<f32>(find_player_coors(GPtr::from_raw(out.as_mut_ptr()), -1).cast::<u8>(), i * 4)
    };
    let left = x(coor(0)).fsub_m(margin).to_f32();
    let bottom = x(coor(1)).fsub_m(margin).to_f32();
    let right = x(coor(0)).fadd_m(margin).to_f32();
    let top = x(coor(1)).fadd_m(margin).to_f32();
    let x0 = sector_bound(left, true);
    let y0 = sector_bound(bottom, true);
    let x1 = sector_bound(right, false);
    let y1 = sector_bound(top, false);
    let sc = g::<u16>(SCAN_CODE);
    let mut code = sc.read();
    if code == 0xFFFF {
        world_clear_scan_codes();
        code = 1;
    } else {
        code += 1;
    }
    sc.write(code);
    let mut y = y0;
    while y <= y1 {
        let cy = y.clamp(0, 0x77);
        let mut xx = x0;
        while xx <= x1 {
            let cx = xx.clamp(0, 0x77);
            let mut link = g::<u32>(SECTORS + ((cy * 0x78 + cx) as usize) * 8).read();
            while link != 0 {
                let l = GPtr::<u8>::from_addr(link as usize);
                let b = rd::<GPtr<u8>>(l, 0);
                if rd::<u16>(b, 0x2C) != code {
                    wr::<u16>(b, 0x2C, code);
                    if should_building_have_cover_points(b) != 0
                        && ptr_list_is_member(GPtr::from_addr(PROCESSED_BUILDINGS), b) == 0
                    {
                        find_cover_points_for_building(b);
                        let node = list_node_new(0xC);
                        if !node.is_null() {
                            wr::<u32>(node, 0, b.addr() as u32);
                        }
                        wr::<u32>(node, 8, 0);
                        wr::<u32>(node, 4, head.read());
                        let h = head.read();
                        if h != 0 {
                            g::<u32>(h as usize + 8).write(node.addr() as u32);
                        }
                        head.write(node.addr() as u32);
                        return;
                    }
                    code = sc.read();
                }
                link = rd::<u32>(l, 4);
            }
            xx += 1;
        }
        y += 1;
    }
}

/// `CCover::AddCoverPoint(int32 type, CEntity* entity, const CVector* pos, uint8 usage, uint8 direction)` (0x698F30,
/// cdecl). Nothing with 100 points. Each used point of the same type that is the same entity (types 1, 2) or the
/// same position (type 3, `==` per component, NaN differs) cancels the add. Then for every used point: the
/// reference position is `*pos`, else the entity's position, else the previous one (the original's stack
/// locals, uninitialised on the first point; zero here); `d = point - ref` (x87, dy / dz also stored as f32);
/// `|d|` (`(dz² + dy²) + dx²`, `fsqrt`) below `[0x858C98]` cancels; else `|(dx, f32 dy, f32 dz)|` below
/// `[0x858CA0]` with the direction difference `(u8)(point.dir - dir)` below 0x20 or above 0xDF cancels. Else
/// the first free point (searched without a bound) gets type, usage (+1), direction (+2), +0x14 = +0x18 = 0, the
/// entity (+0x10; `RegisterReference` unless it is a building (+0x36 & 7 == 1) with +0x2E clear) and the
/// position (+4, when `pos`); the count goes up.
#[hook(addr = 0x698F30, conv = cdecl, name = "CCover::AddCoverPoint", expect = "A1 A4 97 C1 00 83 EC 14",
       route = "L2", hazards = "uninit-locals,heap-writes", recs = "grove-walk")]
pub fn add_cover_point(kind: i32, entity: GPtr<u8>, pos: GPtr<u8>, usage: u32, dir: u32) {
    if g::<i32>(COVER_NUM_POINTS).read() >= 100 {
        return;
    }
    let mut v = [0f32; 3];
    let mut p = COVER_POINTS;
    while p < COVER_POINTS_END {
        let pt = GPtr::<u8>::from_addr(p);
        let t = rd::<i8>(pt, 0);
        if i32::from(t) == kind {
            match t {
                1 | 2 => {
                    if rd::<u32>(pt, 0x10) == entity.addr() as u32 {
                        return;
                    }
                }
                3 => {
                    let same = |o: isize| !x(rd::<f32>(pos, o)).fcom(x(rd::<f32>(pt, 4 + o))).test_parity(0x44);
                    if same(0) && same(4) && same(8) {
                        return;
                    }
                }
                _ => {}
            }
        }
        if t != 0 {
            if !pos.is_null() {
                v = [rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8)];
            } else if !entity.is_null() {
                let m = rd::<GPtr<u8>>(entity, 0x14);
                let e = if m.is_null() { entity.byte_add(4) } else { m.byte_add(0x30) };
                v = [rd::<f32>(e, 0), rd::<f32>(e, 4), rd::<f32>(e, 8)];
            }
            let dx = x(rd::<f32>(pt, 4)).fsub_m(v[0]);
            let dy = x(rd::<f32>(pt, 8)).fsub_m(v[1]);
            let dz = x(rd::<f32>(pt, 0xC)).fsub_m(v[2]);
            let r = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
            if !r.fcom(x(ldf(0x858C98))).test_parity(5) {
                return;
            }
            let (dy, dz) = (x(dy.to_f32()), x(dz.to_f32()));
            let r2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
            if !r2.fcom(x(ldf(0x858CA0))).test_parity(5) {
                let d = rd::<u8>(pt, 2).wrapping_sub(dir as u8);
                if !(0x20..=0xDF).contains(&d) {
                    return;
                }
            }
        }
        p += 0x1C;
    }
    let mut i = 0usize;
    while g::<u8>(COVER_POINTS + i * 0x1C).read() != 0 {
        i += 1;
    }
    let pt = GPtr::<u8>::from_addr(COVER_POINTS + i * 0x1C);
    wr::<u8>(pt, 0, kind as u8);
    wr::<u8>(pt, 2, dir as u8);
    wr::<u8>(pt, 1, usage as u8);
    wr::<u32>(pt, 0x14, 0);
    wr::<u32>(pt, 0x18, 0);
    let n = g::<u32>(COVER_NUM_POINTS);
    n.write(n.read().wrapping_add(1));
    wr::<u32>(pt, 0x10, entity.addr() as u32);
    if !entity.is_null() && !(rd::<u8>(entity, 0x36) & 7 == 1 && rd::<u8>(entity, 0x2E) == 0) {
        register_reference(entity, pt.byte_add(0x10));
    }
    if !pos.is_null() {
        for o in [0, 4, 8] {
            wr::<u32>(pt, 4 + o, rd::<u32>(pos, o));
        }
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_COVER_UPDATE, HOOK_ADD_COVER_POINT];
