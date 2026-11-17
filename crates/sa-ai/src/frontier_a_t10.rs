//! FRONTIER-2 turn 10, lane A (batch `frontier2-a15`): the last AI frontier virtual, ported from the 1.0 US bytes
//! (gta-reversed read as an aid; where it differs the bytes win).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::task_timer_is_out_of_time;
use crate::common::{g, rd, wr, x};
use crate::frontier_a_t9::{are_anims_loaded, interior_blend_group, interior_start_timer, interior_step_to_info};
use sa_core::bindings::{clump_get_association, general_limit_radian_angle, matrix_mul_vec, matrix_set_rotate_z_only};
use sa_core::crt::rand_at;
use sa_world::bindings::{placeable_set_posn, ped_set_move_state};
use sa_world::world::common::entity_pos;

orig! {
    /// `CTaskInteriorSitAtDesk::StartRandomLoopAnim(CPed*, float blendDelta)` (0x677780, `ret 8`)
    fn desk_start_random_loop_anim(t: GPtr<u8>, ped: GPtr<u8>, delta: f32) = thiscall 0x677780 => unported;
    /// `CTaskInteriorSitAtDesk::StartRandomOneOffAnim(CPed*)` (0x677880, `ret 4`)
    fn desk_start_random_one_off_anim(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x677880 => unported;
}

/// `CTaskInteriorSitAtDesk::FinishAnimCB` (0x676190).
const SIT_AT_DESK_FINISH_CB: u32 = 0x676190;
/// The desk offset vector (0xC18C6C).
const DESK_OFFSET: usize = 0xC18C6C;
/// 0x858CB8: pi
const PI: f32 = f32::from_bits(0x4049_0FDB);

/// `CTaskInteriorSitAtDesk::ProcessPed(CPed*)` (0x677920, `ret 4`, AL). int_office anims (group 0x55): 0x148 sit
/// down, 0x149 stand up, 0x14A..0x150 the loops / one-offs; anim +0x14, previous anim +0x18, the stand-up timer
/// +0x1C, the one-off timer +0x28 (active +0x30), +0x34 "place the ped", +0x35 finished, +0x36 aborting, +0x10 start
/// sitting. `SetMoveState(1)`. Finished with 0x149 gone: the ped turned by pi (`LimitRadianAngle`, +0x55C / +0x558
/// and the matrix's Z rotation or +0x10), 1. Aborting: without the anims 1; sit-down → blend delta -8; a loop (not
/// being placed) → stand-up blended (1000), 0; stand-up → its +0x24 = 2. No anim (with the anims): the first run
/// sits down (blend 4) unless +0x10, which (like a finished sit-down) starts the timer and a random loop (1000,
/// placed); after a one-off (0x14D..0x150) a random loop (4); 0. With an anim: placing moves the ped to its matrix
/// times the desk offset (loops 0x14A..0x14C) or minus it (stand-up), else its own position, z kept; the timer out →
/// stand-up; else the one-off timer out → `rand() < 0x3FFF` a one-off, else a loop (4); sitting down steps towards
/// the interior info. 0.
#[hook(addr = 0x677920, conv = thiscall, name = "CTaskInteriorSitAtDesk::ProcessPed", expect = "83 EC 24 53 55 56 8B F1",
       unverified = "frontier2-a15", route = "L2", hazards = "heap-writes,ax-al-return,stack-residue",
       note = "the original stores floats over its own ped argument slot; AL result")]
pub fn interior_sit_at_desk_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let anim = rd::<GPtr<u8>>(this, 0x14);
    let cur = if anim.is_null() { -1 } else { i32::from(rd::<i16>(anim, 0x2C)) };
    ped_set_move_state(ped, 1);
    if rd::<u8>(this, 0x35) != 0 && clump_get_association(rd::<GPtr<u8>>(ped, 0x18), 0x149).is_null() {
        let h = x(rd::<f32>(ped, 0x558)).fadd_m(PI).to_f32();
        let h = general_limit_radian_angle(h) as f32;
        wr::<f32>(ped, 0x55C, h);
        wr::<f32>(ped, 0x558, h);
        let m = rd::<GPtr<u8>>(ped, 0x14);
        if m.is_null() {
            wr::<f32>(ped, 0x10, h);
        } else {
            matrix_set_rotate_z_only(m, h);
        }
        return 1;
    }
    if rd::<u8>(this, 0x36) != 0 {
        if are_anims_loaded(2) == 0 {
            return 1;
        }
        if cur == 0x148 {
            wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x1C, 0xC100_0000);
        } else if (0x14A..=0x150).contains(&cur) {
            if rd::<u8>(this, 0x34) == 0 {
                interior_blend_group(this, ped, 0x55, 0x149, 1000.0, SIT_AT_DESK_FINISH_CB, true);
                wr::<u8>(this, 0x34, 1);
                return 0;
            }
        } else if cur == 0x149 {
            wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x24, 0x4000_0000);
        }
    }
    if rd::<GPtr<u8>>(this, 0x14).is_null() {
        if are_anims_loaded(2) == 0 {
            return 0;
        }
        let prev = rd::<i32>(this, 0x18);
        if prev == -1 {
            if rd::<u8>(this, 0x10) == 0 {
                interior_blend_group(this, ped, 0x55, 0x148, 4.0, SIT_AT_DESK_FINISH_CB, false);
                return 0;
            }
        } else if prev != 0x148 {
            if (0x14D..=0x150).contains(&prev) {
                desk_start_random_loop_anim(this, ped, 4.0);
            }
            return 0;
        }
        interior_start_timer(this);
        desk_start_random_loop_anim(this, ped, 1000.0);
        wr::<u8>(this, 0x34, 1);
        return 0;
    }
    if rd::<u8>(this, 0x34) != 0 {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let mut p: [f32; 3] = core::array::from_fn(|i| rd::<f32>(entity_pos(ped), 4 * i as isize));
        let z = p[2];
        if (0x14A..=0x14C).contains(&cur) {
            let mut o = [0f32; 3];
            matrix_mul_vec(GPtr::from_raw(o.as_mut_ptr()).cast(), m, GPtr::from_addr(DESK_OFFSET));
            p = o;
        } else if cur == 0x149 {
            let mut neg: [f32; 3] = core::array::from_fn(|i| x(g::<f32>(DESK_OFFSET + 4 * i).read()).fchs().to_f32());
            let mut o = [0f32; 3];
            matrix_mul_vec(GPtr::from_raw(o.as_mut_ptr()).cast(), m, GPtr::from_raw(neg.as_mut_ptr()).cast());
            p = o;
        }
        p[2] = z;
        placeable_set_posn(ped, GPtr::from_raw(p.as_mut_ptr()));
        wr::<u8>(this, 0x34, 0);
    }
    if rd::<u8>(this, 0x24) != 0 && task_timer_is_out_of_time(this.byte_add(0x1C)) != 0 {
        if rd::<i16>(rd::<GPtr<u8>>(this, 0x14), 0x2C) != 0x149 {
            interior_blend_group(this, ped, 0x55, 0x149, 1000.0, SIT_AT_DESK_FINISH_CB, true);
            wr::<u8>(this, 0x34, 1);
        }
    } else if rd::<u8>(this, 0x30) != 0 && task_timer_is_out_of_time(this.byte_add(0x28)) != 0 {
        wr::<u8>(this, 0x30, 0);
        if rand_at(0x677C65) < 0x3FFF {
            desk_start_random_one_off_anim(this, ped);
        } else {
            desk_start_random_loop_anim(this, ped, 4.0);
        }
    }
    if rd::<i16>(rd::<GPtr<u8>>(this, 0x14), 0x2C) == 0x148 {
        interior_step_to_info(this, ped);
    }
    0
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_INTERIOR_SIT_AT_DESK_PROCESS_PED];
