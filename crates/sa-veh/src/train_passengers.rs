//! Train passengers: a random ped near a stopped carriage is sent to board it.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::fpu::{ge_or_nan, gt, lt, x};

use crate::common::{g, rd, wr};
use sa_core::bindings::{matrix_invert, matrix_mul_vec, task_new, vector_dot_product};
use sa_world::bindings::{
    enter_car_as_driver_ctor, enter_car_as_passenger_ctor, entity_clean_up_old_reference, entity_register_reference,
    find_player_ped, get_nearest_car_door, intel_find_task_by_type, matrix_dtor, ped_groups_get_peds_group,
    population_remove_ped, seek_entity_xy_offset_ctor, sequence_add_task, sequence_ctor, tm_set_task,
    vector_add_assign, vector_magnitude_sqr, vector_sub, vector_sub_assign,
};
use sa_world::world::common::entity_pos;
use sa_world::world::cworld::in_vehicle;

orig! {
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
}

/// `CPools::ms_pPedPool`.
const PED_POOL: usize = 0xB7_4490;
/// The task types `ENTER_CAR_AS_DRIVER` (700), `ENTER_CAR_AS_PASSENGER` (701) and the one that keeps a passenger
/// seated (0x2C3).
const ENTER_CAR_AS_DRIVER: i32 = 0x2BC;
const ENTER_CAR_AS_PASSENGER: i32 = 0x2BD;
const STAY_IN_TRAIN: i32 = 0x2C3;

/// The ped flag +0x478 bit 18.
fn flag_478_bit18(ped: GPtr<u8>) -> bool {
    rd::<u32>(ped, 0x478) & 0x4_0000 != 0
}

/// A pool ped that may become a passenger: a random ped (created by 1) in no group, with no +0x480 owner, not a
/// cop type (6), on foot, without flag +0x478 bit 18, not in states 0x36..0x38 and not already entering a car as driver.
fn eligible(ped: GPtr<u8>) -> bool {
    rd::<u8>(ped, 0x484) == 1
        && ped_groups_get_peds_group(ped.addr() as u32) == 0
        && rd::<u32>(ped, 0x480) == 0
        && rd::<u32>(ped, 0x598) != 6
        && !in_vehicle(ped)
        && !flag_478_bit18(ped)
        && !matches!(rd::<u32>(ped, 0x530), 0x36..=0x38)
        && intel_find_task_by_type(rd::<GPtr<u8>>(ped, 0x47C), ENTER_CAR_AS_DRIVER).is_null()
}

/// `CTrain::AddNearbyPedAsRandomPassenger()` (0x6F8170, thiscall). Nothing while the player is in this carriage or
/// its primary task (+0x10 of its intelligence) enters it, or when the passenger count (+0x5CB low nibble) has reached
/// the maximum (high nibble). A passenger still boarding (+0x5CC) stays while its +0x478 bit 18 equals its in-vehicle
/// bit or it has the 0x2C3 task; otherwise the reference is dropped (a seated random ped of this
/// carriage removed from the population). Then the nearest eligible ped (by squared horizontal distance below 625;
/// the pool walked from the last slot) on the carriage's boarding side (+0x5B8 bit 6: the side where `pos . right +
/// d` is not positive, else not negative, `d = -(carriage pos . right)`) gets a sequence: seek the carriage to the
/// nearest door's offset in carriage space moved by `-right` (`+right` on the other side), then enter it as driver
/// (as passenger on the other side); it becomes the boarding passenger and the count goes up.
#[hook(addr = 0x6F8170, conv = thiscall, name = "CTrain::AddNearbyPedAsRandomPassenger", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier2-b33", route = "L2", hazards = "heap-writes")]
pub fn add_nearby_ped_as_random_passenger(this: GPtr<u8>) {
    let player = find_player_ped(-1);
    if rd::<GPtr<u8>>(player, 0x58C) == this {
        return;
    }
    let task = rd::<GPtr<u8>>(rd::<GPtr<u8>>(player, 0x47C), 0x10);
    if !task.is_null()
        && (v_task_get_type(task) == ENTER_CAR_AS_PASSENGER || v_task_get_type(task) == ENTER_CAR_AS_DRIVER)
        && rd::<GPtr<u8>>(task, 0xC) == this
    {
        return;
    }
    let other_side = (rd::<u8>(this, 0x5B8) >> 6) & 1;
    let count = rd::<u8>(this, 0x5CB);
    if count & 0xF == count >> 4 {
        return;
    }
    let slot = this.byte_add(0x5CC);
    let boarding = rd::<GPtr<u8>>(slot, 0);
    if !boarding.is_null() {
        if flag_478_bit18(boarding) == in_vehicle(boarding)
            || !intel_find_task_by_type(rd::<GPtr<u8>>(boarding, 0x47C), STAY_IN_TRAIN).is_null()
        {
            return;
        }
        if !rd::<GPtr<u8>>(slot, 0).is_null() {
            entity_clean_up_old_reference(rd::<GPtr<u8>>(slot, 0), slot);
        }
        let p = rd::<GPtr<u8>>(slot, 0);
        if !flag_478_bit18(p) && in_vehicle(p) && rd::<GPtr<u8>>(p, 0x58C) == this {
            population_remove_ped(p);
        }
        wr::<u32>(slot, 0, 0);
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let pos = entity_pos(this);
    let dot = x(rd::<f32>(pos, 8))
        .fmul_m(rd::<f32>(m, 8))
        .fadd(x(rd::<f32>(pos, 4)).fmul_m(rd::<f32>(m, 4)))
        .fadd(x(rd::<f32>(pos, 0)).fmul_m(rd::<f32>(m, 0)));
    let plane = dot.fchs().to_f32();
    let pool = g::<GPtr<u8>>(PED_POOL).read();
    let mut best = GPtr::<u8>::NULL;
    let mut best_dist = f32::from_bits(0x4974_23F0);
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        if rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize) < 0 {
            continue;
        }
        let ped = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0x7C4);
        if ped.is_null() || !eligible(ped) {
            continue;
        }
        let mut d = [0u32; 3];
        let dv = GPtr::from_raw(d.as_mut_ptr().cast::<u8>());
        vector_sub(dv, entity_pos(ped), entity_pos(this));
        wr::<u32>(dv, 8, 0);
        let dist = vector_magnitude_sqr(dv);
        let dist_f32 = dist.to_f32();
        if gt(dist, x(f32::from_bits(g::<u32>(0x87_2368).read()))) {
            continue;
        }
        let side = vector_dot_product(entity_pos(ped), rd::<GPtr<u8>>(this, 0x14)).fadd_m(plane);
        let zero = x(g::<f32>(0x85_8B50).read());
        let wrong_side = match other_side {
            0 => gt(side, zero),
            _ => lt(side, zero),
        };
        if wrong_side || ge_or_nan(x(dist_f32), x(best_dist)) {
            continue;
        }
        best_dist = dist_f32;
        best = ped;
    }
    if best.is_null() {
        return;
    }
    let mut door = [0u32; 3];
    let mut door_id = 0u32;
    let door_pos = GPtr::from_raw(door.as_mut_ptr().cast::<u8>());
    get_nearest_car_door(best, this, door_pos, GPtr::from_raw((&mut door_id as *mut u32).cast::<u8>()));
    let mut inv = [0u32; 0x48 / 4];
    let inv_m = GPtr::from_raw(inv.as_mut_ptr().cast::<u8>());
    wr::<u32>(inv_m, 0x40, 0);
    wr::<u32>(inv_m, 0x44, 0);
    matrix_invert(rd::<GPtr<u8>>(this, 0x14), inv_m);
    let mut out = [0u32; 3];
    let local = matrix_mul_vec(GPtr::from_raw(out.as_mut_ptr().cast::<u8>()), inv_m, door_pos);
    let mut offset = [rd::<u32>(local, 0), rd::<u32>(local, 4), rd::<u32>(local, 8)];
    let offset_p = GPtr::from_raw(offset.as_mut_ptr().cast::<u8>());
    if other_side != 0 {
        vector_add_assign(offset_p, rd::<GPtr<u8>>(this, 0x14));
    } else {
        vector_sub_assign(offset_p, rd::<GPtr<u8>>(this, 0x14));
    }
    let mem = task_new(0x58);
    let seek = if mem.is_null() {
        GPtr::NULL
    } else {
        seek_entity_xy_offset_ctor(
            mem,
            this,
            50_000,
            1000,
            g::<u32>(0x86_FC2C).read(),
            g::<u32>(0x86_FC28).read(),
            g::<u32>(0x86_FC30).read(),
            1,
            1,
        )
    };
    for (k, v) in offset.iter().enumerate() {
        wr::<u32>(seek, 0x44 + 4 * k as isize, *v);
    }
    wr::<u32>(seek, 0x50, 7);
    let mem = task_new(0x40);
    let sequence = if mem.is_null() { GPtr::NULL } else { sequence_ctor(mem) };
    sequence_add_task(sequence, seek);
    let mem = task_new(0x50);
    let enter = if mem.is_null() {
        GPtr::NULL
    } else if other_side == 0 {
        enter_car_as_driver_ctor(mem, this)
    } else {
        enter_car_as_passenger_ctor(mem, this, 0, 0)
    };
    sequence_add_task(sequence, enter);
    tm_set_task(rd::<GPtr<u8>>(best, 0x47C).byte_add(4), sequence, 3, 0);
    let count = rd::<u8>(this, 0x5CB);
    wr::<u8>(this, 0x5CB, ((count.wrapping_add(1) ^ count) & 0xF) ^ count);
    wr::<GPtr<u8>>(slot, 0, best);
    entity_register_reference(best, slot);
    matrix_dtor(inv_m);
}

pub const HOOKS: &[HookReg] = &[HOOK_ADD_NEARBY_PED_AS_RANDOM_PASSENGER];
