//! FRONTIER-2 turn 3, lane B (batch `frontier2-b10`): WORLD functions on the phase B frontier. Ported from the
//! 1.0 US bytes; gta-reversed as a reading aid only (the bytes win). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::fpu::{ge_or_nan, gt};

use super::mvpb_world::{advance_scan_code, cell, sector_index};
use super::common::{entity_pos, f32_of, find_player_ped, find_player_vehicle, g, rd, wr, x};
use crate::bindings::{
    entity_get_col_model, event_group_add, event_script_command_ctor, event_script_command_dtor, find_player_wanted,
    membership_get_leader, ped_groups_get_peds_group, stand_still_ctor, tm_find_active_task_by_type, wanted_remove_cop,
};
use sa_core::bindings::{matrix_mul_vec, ped_is_alive, task_new};

sa_core::orig! {
    /// `CReplay::FindSizeOfPacket(uint8)` (0x45C850)
    fn replay_find_size_of_packet(ty: u32) -> u32 = cdecl 0x45C850 => crate::generated::replay_generated::g_creplay_find_size_of_packet_45c850;
    /// `CWorld::FindObjectsIntersectingAngledCollisionBoxSectorList(list, box, transform, point, count, maxCount, out)`
    /// (0x565200)
    fn list_scan(list: GPtr<u8>, bx: GPtr<u8>, transform: GPtr<u8>, point: GPtr<u8>, count: GPtr<i16>, max: i16, out: GPtr<GPtr<u8>>) = cdecl 0x565200 => super::protected_world_queries::find_objects_intersecting_angled_collision_box_sector_list;
}

/// `CWorld::ms_nCurrentScanCode`
const SCAN_CODE: usize = 0xB7_CD78;

/// The entity's position strictly inside the open rectangle (`x > x0`, `x < x1`, `y > y0`, `y < y1`, as the x87
/// compares do; NaN is outside).
fn pos_inside(e: GPtr<u8>, x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
    let p = entity_pos(e);
    gt(x(rd(p, 0)), x(x0)) && !ge_or_nan(x(rd(p, 0)), x(x1)) && gt(x(rd(p, 4)), x(y0)) && !ge_or_nan(x(rd(p, 4)), x(y1))
}

/// Marks the entity with the current scan code; false when it was already marked.
fn take_scan_code(e: GPtr<u8>) -> bool {
    let code = g::<u16>(SCAN_CODE).read();
    if rd::<u16>(e, 0x2C) == code {
        return false;
    }
    wr::<u16>(e, 0x2C, code);
    true
}

/// `CWorld::CallOffChaseForAreaSectorListVehicles(CPtrList&, x1, y1, x2, y2, minX, minY, maxX, maxY)` (0x563A80,
/// cdecl). Every vehicle of the list not yet scanned and not the player's, inside the grown rectangle, that is a law
/// enforcer (+0x428 bit 0) on a chase mission (+0x3BA 2, 3, 4, 5, 0x3C, 0x3D) gets temp action 1 (+0x3BB) until
/// now + 2000 (+0x3BC). When any collision sphere (centre through the matrix, plus/minus its radius) overlaps the
/// original rectangle, the speed toward the rectangle's centre is cut: `ms.x` = 0 when the car is right of the
/// centre and moving left (or left and moving right), likewise `ms.y`.
#[hook(addr = 0x563A80, conv = cdecl, name = "CWorld::CallOffChaseForAreaSectorListVehicles", expect = "8B 44 24 04 8B 00 83 EC 1C",
       unverified = "frontier2-b10", route = "L2", hazards = "none",
       note = "the original keeps its sphere-hit bool in its own (caller-owned) list argument slot (not modelled)")]
#[allow(clippy::too_many_arguments)]
pub fn call_off_chase_sector_list_vehicles(list: GPtr<u8>, x1: f32, y1: f32, x2: f32, y2: f32, a: f32, b: f32, c: f32, d: f32) {
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let v = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        if !take_scan_code(v) || v == find_player_vehicle(-1, 0) || !pos_inside(v, a, b, c, d) {
            continue;
        }
        if rd::<u8>(v, 0x428) & 1 == 0 || !matches!(rd::<u8>(v, 0x3BA), 2..=5 | 0x3C | 0x3D) {
            continue;
        }
        wr::<u8>(v, 0x3BB, 1);
        wr::<u32>(v, 0x3BC, g::<u32>(0xB7_CB84).read().wrapping_add(2000));
        let mut hit = false;
        let data = rd::<GPtr<u8>>(entity_get_col_model(v), 0x2C);
        let n = i32::from(rd::<i16>(data, 0));
        for i in 0..n {
            let s = rd::<GPtr<u8>>(data, 8).byte_add(0x14 * i as isize);
            let mut out = [0f32; 3];
            let op = GPtr::from_raw(out.as_mut_ptr()).cast::<u8>();
            matrix_mul_vec(op, rd::<GPtr<u8>>(v, 0x14), s);
            let r = rd::<f32>(rd::<GPtr<u8>>(data, 8).byte_add(0x14 * i as isize), 0xC);
            if gt(x(out[0]).fadd_m(r), x(x1))
                && !ge_or_nan(x(out[0]).fsub_m(r), x(x2))
                && gt(x(out[1]).fadd_m(r), x(y1))
                && !ge_or_nan(x(out[1]).fsub_m(r), x(y2))
            {
                hit = true;
            }
        }
        if !hit {
            continue;
        }
        for (k, (lo, hi)) in [(x1, x2), (y1, y2)].into_iter().enumerate() {
            let p = entity_pos(v);
            let centre = x(lo).fadd_m(hi).fmul_m(0.5);
            let o = 0x44 + 4 * k as isize;
            let ms = x(rd::<f32>(v, o));
            let zero = x(0.0);
            let cut = if ge_or_nan(centre, x(rd(p, 4 * k as isize))) { !ge_or_nan(zero, ms) } else { gt(zero, ms) };
            wr::<f32>(v, o, if cut { 0.0 } else { f32_of(ms) });
        }
    }
}

/// `CWorld::CallOffChaseForAreaSectorListPeds(CPtrList&, x1, y1, x2, y2, minX, minY, maxX, maxY)` (0x563D00, cdecl).
/// Every ped of the list not yet scanned, not the player and not in a group led by the player, inside the original
/// rectangle (the grown one is not used), that is killing / arresting the player (task 0x3E8's target is the player
/// ped) or chasing the player's vehicle (task 0x3EB's target), and alive: a cop (type 6) leaves the player's pursuit,
/// and it gets a script-command event (priority 3) with a stand-still task (0 ms, looped, blend 8.0).
#[hook(addr = 0x563D00, conv = cdecl, name = "CWorld::CallOffChaseForAreaSectorListPeds", expect = "64 A1 00 00 00 00 6A FF 68 D3 CC 83 00",
       unverified = "frontier2-b10", route = "L2", hazards = "heap-writes",
       note = "the original keeps the next node in its own (caller-owned) list argument slot (not modelled); adds an event to the ped's event group")]
#[allow(clippy::too_many_arguments)]
pub fn call_off_chase_sector_list_peds(list: GPtr<u8>, x1: f32, y1: f32, x2: f32, y2: f32, _a: f32, _b: f32, _c: f32, _d: f32) {
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let ped = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        if !take_scan_code(ped) {
            continue;
        }
        let group = ped_groups_get_peds_group(ped.addr() as u32);
        let leader = if group == 0 { GPtr::NULL } else { membership_get_leader(GPtr::from_addr(group as usize + 8)) };
        if ped == find_player_ped(-1) || leader == find_player_ped(-1) || !pos_inside(ped, x1, y1, x2, y2) {
            continue;
        }
        let target = |ty: i32| {
            let t = tm_find_active_task_by_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), ty);
            if t.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(t, 0x10) }
        };
        let kill = target(0x3E8);
        let chase = target(0x3EB);
        if kill != find_player_ped(-1) && (chase.is_null() || chase != find_player_vehicle(-1, 0)) {
            continue;
        }
        if ped_is_alive(ped) as u8 == 0 {
            continue;
        }
        if rd::<u32>(ped, 0x598) == 6 {
            wanted_remove_cop(find_player_wanted(-1), ped);
        }
        let mem = task_new(0x20);
        let task = if mem.is_null() { mem } else { stand_still_ctor(mem, 0, 1, 0, 8.0) };
        let mut ev = [0u32; 10];
        let e = GPtr::<u8>::from_raw(ev.as_mut_ptr().cast());
        event_script_command_ctor(e, 3, task, 0);
        event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e, 0);
        event_script_command_dtor(e.cast::<u32>());
    }
}

/// `CWorld::FindObjectsIntersectingAngledCollisionBox(const CColBox& box, const CMatrix& transform, const CVector&
/// point, float x1, float y1, float x2, float y2, int16* count, int16 maxCount, CEntity** out, bool buildings, bool
/// vehicles, bool peds, bool objects, bool dummies)` (0x568FF0, cdecl): the sector rectangle of (x1, y1)-(x2, y2), the
/// scan code advanced, the count zeroed; per cell (y outer, x inner) the sector's (clamped) building / dummy lists and
/// the repeat sector's vehicle / ped / object lists through `FindObjectsIntersectingAngledCollisionBoxSectorList`.
/// Kept original detail: the last x index and the loop index live in the caller's y1 / x2 argument slots (not modelled).
#[hook(addr = 0x568FF0, conv = cdecl, name = "CWorld::FindObjectsIntersectingAngledCollisionBox", expect = "83 EC 10 D9 44 24 20 53",
       unverified = "frontier2-b10", route = "L2", hazards = "none",
       note = "overwrites its own (caller-owned) argument slots with loop indices")]
#[allow(clippy::too_many_arguments)]
pub fn world_find_objects_intersecting_angled_collision_box(bx: GPtr<u8>, transform: GPtr<u8>, point: GPtr<u8>, x1: f32, y1: f32,
                                                            x2: f32, y2: f32, count: GPtr<i16>, max: i16, out: GPtr<GPtr<u8>>,
                                                            b: u32, v: u32, p: u32, o: u32, d: u32) {
    let sx0 = sector_index(x(x1));
    let sy0 = sector_index(x(y1));
    let sx1 = sector_index(x(x2));
    let sy1 = sector_index(x(y2));
    advance_scan_code();
    count.write(0);
    for ys in sy0..=sy1 {
        for xs in sx0..=sx1 {
            let (sector, rep) = cell(xs, ys);
            if b as u8 != 0 {
                list_scan(sector, bx, transform, point, count, max, out);
            }
            if v as u8 != 0 {
                list_scan(rep, bx, transform, point, count, max, out);
            }
            if p as u8 != 0 {
                list_scan(rep.byte_add(4), bx, transform, point, count, max, out);
            }
            if o as u8 != 0 {
                list_scan(rep.byte_add(8), bx, transform, point, count, max, out);
            }
            if d as u8 != 0 {
                list_scan(sector.byte_add(4), bx, transform, point, count, max, out);
            }
        }
    }
}

/// `CReplay::StreamAllNecessaryCarsAndPeds()` (stub 0x45D4B0 → 0x156A8C0, cdecl): for each of the 8 replay buffers in
/// use (status 0x97FB7C + i), every packet up to the end marker (type 0; sizes from `FindSizeOfPacket`) requests its
/// model (flags 0): type 1 (vehicle) and 2 (ped) the word at +0x1A, type 3 the word at +2; then
/// `LoadAllRequestedModels(false)`.
#[hook(addr = 0x45D4B0, conv = cdecl, name = "CReplay::StreamAllNecessaryCarsAndPeds", expect = "E9 0B D4 10 01",
       unverified = "frontier2-b10", route = "L2", hazards = "none",
       note = "requests and loads models (streaming state)")]
pub fn replay_stream_all_necessary_cars_and_peds() {
    use sa_core::bindings::{load_all_requested_models, streaming_request_model};
    for i in 0..8usize {
        let buf = GPtr::<u8>::from_addr(0x97_FB88 + 100_000 * i);
        if g::<u8>(0x97_FB7C + i).read() == 0 || rd::<u8>(buf, 0) == 0 {
            continue;
        }
        let mut off = 0isize;
        loop {
            let ty = rd::<u8>(buf, off);
            match ty {
                1 | 2 => {
                    streaming_request_model(i32::from(rd::<i16>(buf, off + 0x1A)), 0);
                }
                3 => {
                    streaming_request_model(i32::from(rd::<i16>(buf, off + 2)), 0);
                }
                _ => {}
            }
            off += replay_find_size_of_packet(u32::from(ty)) as isize;
            if rd::<u8>(buf, off) == 0 {
                break;
            }
        }
    }
    load_all_requested_models(0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_CALL_OFF_CHASE_SECTOR_LIST_VEHICLES, HOOK_CALL_OFF_CHASE_SECTOR_LIST_PEDS,
    HOOK_WORLD_FIND_OBJECTS_INTERSECTING_ANGLED_COLLISION_BOX,
    HOOK_REPLAY_STREAM_ALL_NECESSARY_CARS_AND_PEDS,
];
