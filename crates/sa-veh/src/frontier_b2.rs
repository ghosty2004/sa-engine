//! FRONTIER-2 turn 2, lane B (batch `frontier2-b5`): vehicle functions on the phase B frontier (`CStuckCarCheck`, ...).
//! Ported from the 1.0 US bytes (the `.HOODLUM` bodies followed through the no-CD protection's obfuscated cells,
//! evaluated on the exe image); gta-reversed as a reading aid only (the bytes win). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::checks::stuck_reset;
use crate::common::{g, rd, wr, x};
use sa_core::bindings::matrix_set_rotate_z_only;
use sa_pop::bindings::set_coords_of_script_car;
use sa_world::bindings::{camera_is_sphere_visible_m, find_mission_entities_intersecting_cube};
use sa_world::world::common::entity_pos;

/// `CStuckCarCheck::AddCarToCheck(int32 handle, float radius, uint32 time, bool warp, bool stuck, bool flipped, bool
/// inWater, int8 nodes)` (stub 0x465970 → .HOODLUM 0x15627A0, thiscall, `ret 0x20`): only for a live vehicle handle
/// (the pool's `GetAtRef` inlined); the first of the 16 entries (0x24 bytes) whose handle is negative takes the handle,
/// the vehicle's position (+4), the time (+0x10, `m_snTimeInMilliseconds` through the cell decoded from 0x1560494 +
/// 0x7172D2 + 1 = 0xB7CB84), the radius (+0x14), the time (+0x18), the flags (+0x1D warp, +0x1E stuck, +0x1F flipped,
/// +0x20 in water), the node count (+0x21) and stuck = 0 (+0x1C). A full table is left as it is.
#[hook(addr = 0x465970, conv = thiscall, name = "CStuckCarCheck::AddCarToCheck", expect = "E9 2B CE 0F 01",
       this = "CStuckCarCheck", unverified = "frontier2-b5", route = "L2", hazards = "none")]
#[allow(clippy::too_many_arguments)]
pub fn stuck_add_car_to_check(this: GPtr<u8>, handle: i32, radius: u32, time: u32, warp: u32, stuck: u32, flipped: u32, water: u32, nodes: u32) {
    let v: GPtr<u8> = sa_core::pool::get_vehicle(handle).cast();
    if v.is_null() {
        return;
    }
    for k in 0..16isize {
        let e = this.byte_add(0x24 * k);
        if rd::<i32>(e, 0) >= 0 {
            continue;
        }
        wr::<i32>(e, 0, handle);
        let p = entity_pos(v);
        for o in [0isize, 4, 8] {
            wr::<u32>(e, 4 + o, rd(p, o));
        }
        wr::<u32>(e, 0x10, g::<u32>(0xB7_CB84).read());
        wr::<u32>(e, 0x14, radius);
        wr::<u32>(e, 0x18, time);
        wr::<u8>(e, 0x1D, warp as u8);
        wr::<u8>(e, 0x1E, stuck as u8);
        wr::<u8>(e, 0x1F, flipped as u8);
        wr::<u8>(e, 0x1C, 0);
        wr::<u8>(e, 0x20, water as u8);
        wr::<u8>(e, 0x21, nodes as u8);
        return;
    }
}

/// `CStuckCarCheck::RemoveCarFromCheck(int32 handle)` (stub 0x463B80 → .HOODLUM 0x156DC70, thiscall, `ret 4`): every
/// entry with the handle is reset as `ResetArrayElement` does (inlined: handle and time -1, position -5000, the rest 0).
#[hook(addr = 0x463B80, conv = thiscall, name = "CStuckCarCheck::RemoveCarFromCheck", expect = "E9 EB A0 10 01",
       this = "CStuckCarCheck", unverified = "frontier2-b5", route = "L2", hazards = "none")]
pub fn stuck_remove_car_from_check(this: GPtr<u8>, handle: i32) {
    for k in 0..16isize {
        let e = this.byte_add(0x24 * k);
        if rd::<i32>(e, 0) == handle {
            stuck_reset(e);
        }
    }
}

/// `CStuckCarCheck::AttemptToWarpVehicle(CVehicle*, CVector* origin, float headingDeg)` (0x463A60, thiscall (`this`
/// unused), `ret 0xC`, AL): false when a sphere of radius 4 at `origin` is visible to the camera (`IsSphereVisible`
/// with the view matrix 0xB6FA74, and with the mirror matrix 0xB6FABC while the mirror flag 0xB6F998 is set), or when
/// `FindMissionEntitiesIntersectingCube(origin - 4, origin + 4, &count, 2, null, vehicles, peds, objects)` finds any;
/// else `CCarCtrl::SetCoordsOfScriptCar(v, origin, 1, 1)`, the heading `headingDeg * [0x8595EC]` set
/// (`CMatrix::SetRotateZOnly` with a matrix, else the placement heading +0x10) and true. The original keeps the count in
/// its own `origin` argument slot (stack residue; the word written over the pointer's low half).
#[hook(addr = 0x463A60, conv = thiscall, name = "CStuckCarCheck::AttemptToWarpVehicle", expect = "83 EC 18 56 8B 74 24 24",
       this = "CStuckCarCheck", unverified = "frontier2-b5", route = "L2", hazards = "stack-residue",
       note = "the entity count lives in the origin argument slot")]
pub fn stuck_attempt_to_warp_vehicle(_this: GPtr<u8>, v: GPtr<u8>, origin: GPtr<u8>, heading: f32) -> u8 {
    let cam = GPtr::<u8>::from_addr(0xB6_F028);
    if camera_is_sphere_visible_m(cam, origin, 4.0, GPtr::from_addr(0xB6_FA74)) != 0 {
        return 0;
    }
    if g::<u8>(0xB6_F998).read() != 0 && camera_is_sphere_visible_m(cam, origin, 4.0, GPtr::from_addr(0xB6_FABC)) != 0 {
        return 0;
    }
    let four = g::<f32>(0x85_8B90).read();
    let o = |k: isize| rd::<f32>(origin, k);
    let mut max = [x(o(0)).fadd_m(four).to_f32(), x(o(4)).fadd_m(four).to_f32(), x(o(8)).fadd_m(four).to_f32()];
    let mut min = [x(o(0)).fsub_m(four).to_f32(), x(o(4)).fsub_m(four).to_f32(), x(o(8)).fsub_m(four).to_f32()];
    let mut count = [0u16; 2];
    find_mission_entities_intersecting_cube(
        GPtr::from_raw(min.as_mut_ptr()).cast(),
        GPtr::from_raw(max.as_mut_ptr()).cast(),
        GPtr::from_raw(count.as_mut_ptr()),
        2,
        0,
        1,
        1,
        1,
    );
    if count[0] != 0 {
        return 0;
    }
    set_coords_of_script_car(v, rd(origin, 0), rd(origin, 4), rd(origin, 8), 1, 1);
    let h = x(heading).fmul_m(g::<f32>(0x85_95EC).read()).to_f32();
    let m = rd::<GPtr<u8>>(v, 0x14);
    if m.is_null() {
        wr::<f32>(v, 0x10, h);
    } else {
        matrix_set_rotate_z_only(m, h);
    }
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_STUCK_ADD_CAR_TO_CHECK,
    HOOK_STUCK_REMOVE_CAR_FROM_CHECK,
    HOOK_STUCK_ATTEMPT_TO_WARP_VEHICLE,
];
