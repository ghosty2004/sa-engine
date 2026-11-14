//! `CTaskSimpleDie::StartAnim` / `ProcessPed` and `CTaskComplexEnterCar::GetCameraAvoidVehicle`, executed by the
//! original under SA:MP 0.3.7 (`catalog/samp-priority.csv`) (PORT-AI-7). Ported from the 1.0 US bytes (the exe wins
//! over gta-reversed). Every call to another catalogued function goes through `orig!` at the original address.
//!
//! `CTaskSimpleDie`: +8 anim group, +0xC anim id, +0x10 anim hierarchy (or null), +0x14 anim flags, +0x18 blend
//! delta, +0x1C anim speed, +0x20 flags (bit 0 finished, bit 1 death event sent), +0x24 the association.
#![allow(clippy::unreadable_literal)]

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `CTaskSimpleDie::FinishAnimDieCB`
const DIE_CB: u32 = 0x62FC10;

use sa_world::bindings::anim_blend_animation;
use sa_stream::bindings::anim_blend_animation_hier;
use sa_anim::bindings::assoc_set_finish_callback;
use crate::bindings::death_dtor as event_death_dtor;
use crate::bindings::simple_die_ctor;
use sa_stream::bindings::ped_is_player;
use crate::bindings::event_group_add;
use sa_world::bindings::intel_clear_tasks;
use sa_world::bindings::leave_car_ctor;
use sa_world::bindings::ped_set_ped_state;
use sa_world::bindings::ped_stop_non_partial_anims;
use crate::bindings::task_complex_ctor;
use sa_world::bindings::vector_mul_scalar as vector_mul;
orig! {
    /// `CTask::GetSubTask` (vtable +8)
    fn v_sub_task(t: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x8 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CPed::ClearAll` (0x5E5320)
    fn ped_clear_all(p: GPtr<u8>) = thiscall 0x5E5320 => unported;
    /// `CEventDeath::CEventDeath(bool drowning)` (0x4ADDF0, `ret 4`)
    fn event_death_ctor(e: GPtr<u8>, drowning: u32) -> GPtr<u8> = thiscall 0x4ADDF0 => unported;
    /// `CTaskSimpleDie::StartAnim(CPed*)` (0x637520, `ret 4`)
    fn o_die_start_anim(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x637520 => unported;
    /// `CTaskComplexDie::SayDeathSample(CPed*)` (0x630100, `ret 4`)
    fn o_say_death_sample(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x630100 => unported;
    /// `CTaskSimpleDrown::CTaskSimpleDrown` (0x62FEC0)
    fn drown_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x62FEC0 => unported;
    /// `CTaskComplexFallToDeath::CTaskComplexFallToDeath(int32 dir, const CVector&, bool, bool)` (0x679040,
    /// `ret 0x10`)
    fn fall_to_death_ctor(t: GPtr<u8>, dir: u32, v: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x679040 => unported;
}

/// `CTaskSimpleDie::StartAnim(CPed*)` (0x637520, `ret 4`): blends the death anim (from the hierarchy +0x10 with
/// flags +0x14 when set, else group +8 / anim +0xC) at delta +0x18, keeps it (+0x24) with `FinishAnimDieCB`, clears
/// association flag bit 3 (+0x2E), applies a positive speed (+0x1C, `test ah, 0x41; jne`), then `ClearAll`, health
/// (+0x540) = 0, `StopNonPartialAnims` and the death time (+0x750) = now.
#[hook(addr = 0x637520, conv = thiscall, name = "CTaskSimpleDie::StartAnim", expect = "56 8B F1 8B 46 10",
       unverified = "ai7-12", route = "L2", hazards = "heap-writes", recs = "weapon-combat")]
pub fn die_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let hier = rd::<u32>(this, 0x10);
    let delta = rd::<u32>(this, 0x18);
    let clump = rd::<u32>(ped, 0x18);
    let a = if hier != 0 {
        anim_blend_animation_hier(GPtr::from_addr(clump as usize), GPtr::from_addr(hier as usize), rd::<u32>(this, 0x14), f32::from_bits(delta))
    } else {
        anim_blend_animation(GPtr::from_addr(clump as usize), rd::<u32>(this, 8), rd::<u32>(this, 0xC), f32::from_bits(delta))
    };
    wr::<GPtr<u8>>(this, 0x24, a);
    assoc_set_finish_callback(a, DIE_CB, GPtr::<u8>::addr(GPtr::from_addr((this.addr() as u32) as usize)) as u32);
    let a = rd::<GPtr<u8>>(this, 0x24);
    wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xF7);
    if !x(rd::<f32>(this, 0x1C)).fcom(x(g::<f32>(0x858B50).read())).test_any(0x41) {
        wr::<u32>(rd::<GPtr<u8>>(this, 0x24), 0x24, rd::<u32>(this, 0x1C));
    }
    ped_clear_all(ped);
    wr::<u32>(ped, 0x540, 0);
    ped_stop_non_partial_anims(ped);
    wr::<u32>(ped, 0x750, g::<u32>(TIME_MS).read());
}

/// `CTaskSimpleDie::ProcessPed(CPed*)` (0x6397E0, `ret 4`): always AL = 0. Sets ped flag 8 (+0x528). While the anim
/// is not finished (and the anim id is not 0xBF) it starts the anim once and sets ped state 0x36. Afterwards it
/// sets +0x478 bit 0x20 and, once, when the ped is settled (physical flags +0x40 bit 0x100 or 0x2000, not on the
/// ground (+0x1C bit 0), or standing on a non-player entity +0x568 without its flag 4), adds a stack `CEventDeath`
/// (drowning = anim 0x8C; Rust local, the bool passed zero-extended where the original pushes ECX with `this`'s
/// upper bytes) to the ped's event group, marks it sent (+0x20 bit 1) and clears +0x478 bit 0x20.
#[hook(addr = 0x6397E0, conv = thiscall, name = "CTaskSimpleDie::ProcessPed", expect = "64 A1 00 00 00 00",
       unverified = "ai7-12", route = "L2", hazards = "ax-al-return,heap-writes", recs = "weapon-combat")]
pub fn die_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    wr::<u32>(ped, 0x528, rd::<u32>(ped, 0x528) | 8);
    if rd::<u8>(this, 0x20) & 1 == 0 && rd::<u32>(this, 0xC) != 0xBF {
        if rd::<u32>(this, 0x24) == 0 {
            o_die_start_anim(this, ped);
            ped_set_ped_state(ped, 0x36);
        }
        return 0;
    }
    wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) | 0x20);
    if rd::<u8>(this, 0x20) & 2 != 0 {
        return 0;
    }
    let pf = rd::<u32>(ped, 0x40);
    let fire = if pf & 0x100 != 0 || rd::<u8>(ped, 0x1C) & 1 == 0 || pf & 0x2000 != 0 {
        true
    } else if rd::<u8>(ped, 0x46C) & 1 == 0 {
        false
    } else {
        let on = rd::<GPtr<u8>>(ped, 0x568);
        on.is_null() || ped_is_player(ped) != 0 || rd::<u8>(rd::<GPtr<u8>>(ped, 0x568), 0x40) & 4 != 0
    };
    if fire {
        let mut ev = [0u32; 8];
        let e: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
        event_death_ctor(e, u32::from(rd::<u32>(this, 0xC) == 0x8C));
        event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e, 0);
        wr::<u8>(this, 0x20, rd::<u8>(this, 0x20) | 2);
        wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) & 0xFFFF_FFDF);
        event_death_dtor(e);
    }
    0
}

/// `CTaskComplexEnterCar::GetCameraAvoidVehicle` (0x63A690): the vehicle (+0xC) while the sub-task (fetched twice)
/// is one of the door / seat / get-in / set-out types (0x2C0, 0x322..0x324, 0xCB, 0x331, 0x334, 0x330, 0x2CB,
/// 0x2CF, 0x327, 0x325, 0x328, 0x321, 0x32C, 0x32B); else null.
#[hook(addr = 0x63A690, conv = thiscall, name = "CTaskComplexEnterCar::GetCameraAvoidVehicle", expect = "56 8B F1 8B 46 0C",
       route = "L2", hazards = "none", recs = "car-ai")]
pub fn enter_car_get_camera_avoid_vehicle(this: GPtr<u8>) -> u32 {
    if rd::<u32>(this, 0xC) == 0 || v_sub_task(this).is_null() {
        return 0;
    }
    let ty = v_task_type(v_sub_task(this));
    const TYPES: [i32; 16] = [0x2C0, 0x322, 0x323, 0x324, 0xCB, 0x331, 0x334, 0x330, 0x2CB, 0x2CF, 0x327, 0x325,
                              0x328, 0x321, 0x32C, 0x32B];
    if TYPES.contains(&ty) { rd::<u32>(this, 0xC) } else { 0 }
}

/// `CTaskComplexDie::CreateFirstSubTask(CPed*)` (0x6302D0, `ret 4`): says the death sample, then: a drowning death
/// (weapon 0x35) of a ped in a vehicle (+0x46C bit 0x100, +0x478 bit 7 clear) that is not a boat type 3 / 4
/// (+0x594) leaves the car (`CTaskComplexLeaveCar(veh, 0, 0, false, true)`); otherwise ped state 0x36, the primary
/// tasks are cleared (`ClearTasks(false, true)`), and the sub-task is: in a vehicle the 0x1C-byte die-in-car task
/// (`vtbl = 0x86DE04`, weapon at +0xC), for 0x35 `CTaskSimpleDrown`, when falling (+0x20 bit 1)
/// `CTaskComplexFallToDeath(+0x24, dir, (+0x20 >> 2) & 0xFFFFFF01, false)` with dir by +0x24: 0 the ped's forward,
/// 1 -right, 2 -forward, 3 right (any other value passes the original's uninitialised stack vector; zeros here), else
/// `CTaskSimpleDie(+0x10, +0x14, +0x18, +0x1C)`. Null when the pool is full.
#[hook(addr = 0x6302D0, conv = thiscall, name = "CTaskComplexDie::CreateFirstSubTask", expect = "64 A1 00 00 00 00",
       route = "L2", hazards = "heap-writes,uninit-locals", recs = "weapon-combat")]
pub fn die_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    o_say_death_sample(this, ped);
    let weapon = rd::<u32>(this, 0xC);
    if weapon == 0x35 && rd::<u32>(ped, 0x46C) & 0x100 != 0 && rd::<u8>(ped, 0x478) & 0x80 == 0 {
        let veh = rd::<GPtr<u8>>(ped, 0x58C);
        let boat = !veh.is_null() && matches!(rd::<u32>(veh, 0x594), 3 | 4);
        if !boat {
            let t = task_new(0x34);
            return if t.is_null() { t } else { leave_car_ctor(t, rd::<GPtr<u8>>(ped, 0x58C), 0, 0, 0, 1) };
        }
    }
    ped_set_ped_state(ped, 0x36);
    intel_clear_tasks(rd::<GPtr<u8>>(ped, 0x47C), 0, 1);
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let t = task_new(0x1C);
        if t.is_null() {
            return t;
        }
        let w = rd::<u32>(this, 0xC);
        task_complex_ctor(t.addr() as u32);
        wr::<u32>(t, 0xC, w);
        wr::<u32>(t, 0, 0x86DE04);
        wr::<u32>(t, 0x10, 0);
        wr::<u32>(t, 0x14, 0);
        wr::<u8>(t, 0x18, 0);
        wr::<u8>(t, 0x19, 0);
        return t;
    }
    if rd::<u32>(this, 0xC) == 0x35 {
        let t = task_new(0x2C);
        return if t.is_null() { t } else { drown_ctor(t) };
    }
    if rd::<u8>(this, 0x20) & 2 != 0 {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let mut v = [0u32; 3];
        let copy = |src: GPtr<u8>, v: &mut [u32; 3]| {
            for (k, d) in v.iter_mut().enumerate() {
                *d = rd::<u32>(src, k as isize * 4);
            }
        };
        match rd::<u32>(this, 0x24) {
            0 => copy(m.byte_add(0x10), &mut v),
            1 | 2 => {
                let mut tmp = [0u32; 3];
                let tp: GPtr<u8> = GPtr::from_raw(tmp.as_mut_ptr()).cast();
                let src = if rd::<u32>(this, 0x24) == 1 { m } else { m.byte_add(0x10) };
                let r = vector_mul(tp, src, f32::from_bits(0xBF80_0000));
                copy(r, &mut v);
            }
            3 => copy(m, &mut v),
            _ => {}
        }
        let t = task_new(0x24);
        if t.is_null() {
            return t;
        }
        let flag = (rd::<u32>(this, 0x20) >> 2) & 0xFFFF_FF01;
        let vp: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr()).cast();
        return fall_to_death_ctor(t, rd::<u32>(this, 0x24), vp, flag, 0);
    }
    let t = task_new(0x28);
    if t.is_null() {
        return t;
    }
    simple_die_ctor(t, rd::<u32>(this, 0x10), rd::<u32>(this, 0x14), rd::<u32>(this, 0x18), rd::<u32>(this, 0x1C))
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_DIE_START_ANIM,
    HOOK_DIE_PROCESS_PED,
    HOOK_ENTER_CAR_GET_CAMERA_AVOID_VEHICLE,
    HOOK_DIE_CREATE_FIRST_SUB_TASK,
];
