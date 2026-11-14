//! `CEventVehicleCollision::AffectsPed` (0x4B2EE0, batch `carjack-1`): whether a vehicle's collision concerns a ped
//! (a jacked driver standing by the road, traffic around the jack). Ported from the 1.0 US bytes (gta-reversed as a
//! reading aid). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal, clippy::needless_range_loop)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_math::x87::Cmp;

use crate::common::*;

use crate::bindings::bbox_planes as compute_entity_bbox_planes;
use crate::bindings::compute_entity_hit_side as compute_entity_hit_side_pos;
use crate::bindings::compute_entity_hit_side_ped;
use crate::bindings::task_is_go_to_task;
use crate::bindings::walk_round_car_set_new_vehicle;
use sa_core::bindings::ped_is_alive;
use sa_core::bindings::tm_get_active_task as get_active_task;
use sa_core::bindings::tm_get_simplest_active_task as get_simplest_active_task;
use sa_core::bindings::vector_dot_product;
use sa_world::bindings::vector_ctor_iterator;
orig! {
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
}

/// The facing threshold (0.35) and 0.0.
const FACING_MIN: usize = 0x858F9C;
const ZERO: usize = 0x858B50;

use sa_core::field::ptr_of as buf;

use sa_world::world::common::entity_pos as pos_ptr;

/// `DotProduct(a, b) + d < 0.0` (x87: the ST0 dot plus the float, `fcomp` C0 alone).
fn behind(a: GPtr<u8>, b: GPtr<u8>, d: u32) -> bool {
    X87::from_f64_exact(vector_dot_product(a, b).to_f64()).fadd_m(f32::from_bits(d)).fcom(x(g::<f32>(ZERO).read())) == Cmp::Less
}

/// `CEventVehicleCollision::AffectsPed(CPed*)` (0x4B2EE0, `ret 4`). A dead ped: false. Word +0xE set: true. No
/// vehicle (+0x14), a ped in a vehicle (+0x46C bit 8), a boat (+0x590 == 5) or `-DotProduct(normal +0x18, ped
/// forward) < 0.35` (`fcomp`, NaN passes; the ped's matrix read unchecked): false. A go-to simplest task (target +0xC)
/// whose target lies on the same hit side of the vehicle as the ped: false unless the vehicle tows or is towed
/// (+0x4C4 / +0x4C8); then the vehicle's four bounding-box planes at the ped's height (`eh vector constructor
/// iterator` over the CVector locals first) must hold the target or the ped inside all four (`dot + d < 0`), else
/// false. Then the active task: a walk-round-car (0x1FC) around this vehicle → false; around the vehicle it tows
/// (+0x4C8) or that tows it (+0x4C4) → `SetNewVehicle(vehicle, signed bits 2..5 of +0xD)`; a hit-ped-with-car
/// (0x1F9) with this vehicle (+0xC) → false. Otherwise true.
#[hook(addr = 0x4B2EE0, conv = thiscall, name = "CEventVehicleCollision::AffectsPed", expect = "83 EC 48 53 8B 5C 24 50",
       unverified = "carjack-1", route = "L2", hazards = "ax-al-return,uninit-locals", recs = "carjack")]
pub fn event_vehicle_collision_affects_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if ped_is_alive(ped) as u8 == 0 {
        return 0;
    }
    if rd::<u16>(this, 0xE) != 0 {
        return 1;
    }
    let veh = rd::<GPtr<u8>>(this, 0x14);
    if veh.is_null() || rd::<u32>(ped, 0x46C) & 0x100 != 0 || rd::<u32>(veh, 0x590) == 5 {
        return 0;
    }
    let fwd = rd::<GPtr<u8>>(ped, 0x14).byte_add(0x10);
    let d = X87::from_f64_exact(vector_dot_product(this.byte_add(0x18), fwd).to_f64()).fchs();
    if d.fcom(x(g::<f32>(FACING_MIN).read())) == Cmp::Less {
        return 0;
    }
    let simplest = get_simplest_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
    if !simplest.is_null() && task_is_go_to_task(simplest) != 0 {
        let target = simplest.byte_add(0xC);
        let side = compute_entity_hit_side_ped(ped, rd::<GPtr<u8>>(this, 0x14));
        if side == compute_entity_hit_side_pos(target, rd::<GPtr<u8>>(this, 0x14)) as i32 {
            let v = rd::<GPtr<u8>>(this, 0x14);
            if rd::<u32>(v, 0x4C4) == 0 && rd::<u32>(v, 0x4C8) == 0 {
                return 0;
            }
            let mut planes = [0u32; 12];
            let mut dots = [0u32; 4];
            vector_ctor_iterator(buf(&mut planes), 0xC, 4, 0x404350);
            let z = rd::<u32>(pos_ptr(ped), 8);
            compute_entity_bbox_planes(z, rd::<GPtr<u8>>(this, 0x14), buf(&mut planes), buf(&mut dots));
            let (mut n_target, mut n_ped) = (0, 0);
            for i in 0..4usize {
                let plane = buf(&mut planes).byte_add(i as isize * 0xC);
                if behind(target, plane, dots[i]) {
                    n_target += 1;
                }
                if behind(pos_ptr(ped), plane, dots[i]) {
                    n_ped += 1;
                }
            }
            if n_target != 4 && n_ped != 4 {
                return 0;
            }
        }
    }
    let active = get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
    if active.is_null() {
        return 1;
    }
    if v_get_task_type(active) == 0x1FC {
        let round = rd::<GPtr<u8>>(active, 0x1C);
        let v = rd::<GPtr<u8>>(this, 0x14);
        if round == v {
            return 0;
        }
        let towed = rd::<GPtr<u8>>(v, 0x4C8);
        let tower = rd::<GPtr<u8>>(v, 0x4C4);
        if !towed.is_null() && towed == round || !tower.is_null() && tower == round {
            let f = i32::from(((rd::<u8>(active, 0xD) << 2) as i8) >> 4);
            walk_round_car_set_new_vehicle(active, v, f);
        }
        return 1;
    }
    if v_get_task_type(active) == 0x1F9 && rd::<GPtr<u8>>(active, 0xC) == rd::<GPtr<u8>>(this, 0x14) {
        return 0;
    }
    1
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[HOOK_EVENT_VEHICLE_COLLISION_AFFECTS_PED];
