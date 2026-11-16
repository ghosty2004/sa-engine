//! PORT-AI-4: fleeing — `CTaskComplexSmartFleePoint`, `CTaskComplexSmartFleeEntity`.
//! Ported from the 1.0 US bytes (gta-reversed and `docs/re/specs/AI` as reading aids only).
#![allow(clippy::unreadable_literal)]

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `g_ikChainMan` (`IKChainManager_c`)
const IK_CHAIN_MAN: usize = 0xC15448;

use sa_world::bindings::smart_flee_point_ctor as flee_point_ctor;
use crate::bindings::task_timer_is_out_of_time;
use sa_core::bindings::general_random_int_in_range as general_get_random_number_in_range;
use crate::bindings::event_group_add;
use sa_world::bindings::get_event_global_group;
use sa_world::bindings::ik_is_looking;
use crate::bindings::interesting_events_add;
use sa_world::bindings::ped_say;
use sa_world::bindings::run_anim_ctor;
use sa_world::bindings::sequence_add_task;
use sa_world::bindings::sequence_ctor;
use sa_world::bindings::stand_still_ctor;
use crate::bindings::task_complex_ctor;
use sa_world::bindings::tired_ctor;
use sa_world::bindings::wander_ctor;
use crate::bindings::flee_entity_create_sub_task;
use sa_core::bindings::general_get_node_heading_from_vector;
use sa_anim::bindings::ik_abort_look_at;
use sa_world::bindings::ik_look_at;
use crate::bindings::{go_to_point_shooting_ctor, steal_car_ctor};
use crate::bindings::{car_enter_exit_is_vehicle_stealable, event_vehicle_damage_dtor, scanner_closest_vehicle};
use crate::bindings::{tm_get_task_secondary, tm_set_task_secondary, use_gun_control_gun, use_gun_ctor};
use sa_core::bindings::get_weapon_info;
use sa_world::bindings::ped_get_weapon_skill;
use sa_core::bindings::vector_normalise;
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::{entity_clean_up_old_reference, entity_register_reference, event_vehicle_damage_weapon_ctor};
use sa_world::bindings::car_drive_mission_ctor;
orig! {
    /// `CTaskComplexFleeShooting::CTaskComplexFleeShooting(CEntity*, bool, float, int32, int32, int32, int32, float)`
    /// (0x65C900, `ret 0x20`)
    fn flee_shooting_ctor(t: GPtr<u8>, e: u32, scream: u32, safe: u32, time: u32, shoot: u32, recover: u32, period: u32,
        tolerance: u32) -> GPtr<u8> = thiscall 0x65C900 => unported;
    /// `CTaskComplexFleeAnyMeans::CreateSubTask(eTaskType, CPed*)` (0x65D2E0, `ret 8`)
    fn flee_any_create_sub_task_o(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x65D2E0 => flee_any_means_create_sub_task;
    /// `CVehicle` vtable +0xE4 (`CanPedStepOutCar(bool)`)
    fn v_vehicle_e4(v: GPtr<u8>, b: u32) -> u8 = thiscall vslot 0xE4 => unported;
    /// `CTaskComplexSmartFleeEntity::ControlSubTask(CPed*)` (0x65C780, `ret 4`)
    fn flee_entity_control_sub_task_o(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x65C780 => flee_entity_control_sub_task;
    /// `CTaskComplex::CreateFirstSubTask(CPed*)` (vtable +0x24)
    fn v_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: u32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplexSmartFleePoint::SetDefaultTaskWanderDir(CPed*)` (0x65BE00, `ret 4`)
    fn flee_point_set_default_task_wander_dir(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x65BE00 => unported;
    /// `CTaskComplexSmartFleePoint::CreateSubTask(int32, CPed*)` (0x65BE80, `ret 8`)
    fn flee_point_create_sub_task(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x65BE80 => unported;
    /// `CTaskComplexSmartFleePoint::ComputeFleeDir(CPed*)` (0x65BE40, `ret 4`, AL)
    fn flee_point_compute_flee_dir(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x65BE40 => unported;
    /// `FUN_00669d30`: the wander task's direction setter (thiscall, `ret 4`)
    fn wander_set_dir(t: GPtr<u8>, dir: u32) = thiscall 0x669D30 => unported;
    /// `CTaskComplexSmartFleePoint::SetFleePosition(const CVector&, float, bool)` (0x65C3C0, `ret 0xC`)
    fn flee_point_set_flee_position(t: GPtr<u8>, pos: GPtr<u8>, radius: u32, scream: u32) = thiscall 0x65C3C0 => unported;
    /// `CEventSeenPanickedPed::CEventSeenPanickedPed(CPed*)` (0x4B2080, `ret 4`)
    fn seen_panicked_ped_ctor(e: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B2080 => unported;
    /// `FUN_004b2130`: the `CEventSeenPanickedPed` destructor the original calls on its stack event
    fn seen_panicked_ped_dtor(e: GPtr<u8>) = thiscall 0x4B2130 => unported;
}

use sa_core::field::ptr_of as buf;

use sa_world::world::common::entity_pos as ent_pos;

fn ik() -> GPtr<u8> {
    GPtr::from_addr(IK_CHAIN_MAN)
}

use sa_world::world::garage_update::now as time_ms;
/// `(dz·dz + dy·dy) + dx·dx` of `a - p` as the original's x87 sequence computes it (unrounded differences).
pub(crate) fn dist2(a: GPtr<u8>, p: GPtr<u8>) -> X87 {
    let dx = x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(p, 0));
    let dy = x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(p, 4));
    let dz = x(rd::<f32>(a, 8)).fsub_m(rd::<f32>(p, 8));
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx))
}

// ---- CTaskComplexSmartFleePoint -------------------------------------------------------------------------

/// `CTaskComplexSmartFleePoint::CreateSubTask(int32 type, CPed*)` (0x65BE80, `ret 8`): 0xCB → `SetDefaultTaskWander
/// Dir`, `CTaskSimpleStandStill(0, false, false, 8)`; 0xF4 → for move state 6 (+0x30) a sequence of
/// `CTaskSimpleRunAnim(0, 0xA5, 4, false)` and `CTaskSimpleTired(2000)` (a failed allocation adds null, as the
/// original), else null; 0x2CA → the inlined 0x14-byte task with vtable 0x858EB0 (+0xC = 0, +0x10 = 0, +0x11 = 1);
/// 0x390 → `CTaskComplexWander(move state, dir +0x40, false, [0x86FCB0])` with vtable 0x86F690 (`WanderFlee`);
/// anything else (0x516 included) null.
#[hook(addr = 0x65BE80, conv = thiscall, name = "CTaskComplexSmartFleePoint::CreateSubTask", expect = "64 A1 00 00 00 00")]
pub fn flee_point_create_sub_task_port(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    match ty {
        0xCB => {
            flee_point_set_default_task_wander_dir(this, ped);
            let t = task_new(0x20);
            if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, 0, 0, 0, f32::from_bits(0x4100_0000)) }
        }
        0xF4 => {
            if rd::<u32>(this, 0x30) != 6 {
                return GPtr::NULL;
            }
            let s = task_new(0x40);
            let s = if s.is_null() { GPtr::NULL } else { sequence_ctor(s) };
            let a = task_new(0x20);
            let a = if a.is_null() { GPtr::NULL } else { run_anim_ctor(a, 0, 0xA5, f32::from_bits(0x4080_0000), 0) };
            sequence_add_task(s, a);
            let t = task_new(0x1C);
            let t = if t.is_null() { GPtr::NULL } else { tired_ctor(t, 0x7D0) };
            sequence_add_task(s, t);
            s
        }
        0x2CA => {
            let t = task_new(0x14);
            if t.is_null() {
                return GPtr::NULL;
            }
            task_complex_ctor(t.addr() as u32);
            wr::<u32>(t, 0, 0x858EB0);
            wr::<u32>(t, 0xC, 0);
            wr::<u8>(t, 0x10, 0);
            wr::<u8>(t, 0x11, 1);
            t
        }
        0x390 => {
            let t = task_new(0x28);
            if t.is_null() {
                return GPtr::NULL;
            }
            let k = g::<u32>(0x86FCB0).read();
            let ms = rd::<u32>(this, 0x30);
            let dir = rd::<u8>(this, 0x40) as u32;
            wander_ctor(t, ms, dir, 0, k);
            wr::<u32>(t, 0, 0x86F690);
            t
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexSmartFleePoint::CreateFirstSubTask(CPed*)` (0x65C140, `ret 4`): the start position +0xC = the ped's
/// position; a ped in a vehicle (+0x46C bit 0x100 and +0x58C) leaves it first (0x2CA); otherwise the wander
/// direction +0x40 = `GetNodeHeadingFromVector(f32(x - flee x), f32(y - flee y))` and a flee wander (0x390).
#[hook(addr = 0x65C140, conv = thiscall, name = "CTaskComplexSmartFleePoint::CreateFirstSubTask", expect = "56 8B 74 24 08")]
pub fn flee_point_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let p = ent_pos(ped);
    wr::<u32>(this, 0xC, rd::<u32>(p, 0));
    wr::<u32>(this, 0x10, rd::<u32>(p, 4));
    wr::<u32>(this, 0x14, rd::<u32>(p, 8));
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 && rd::<u32>(ped, 0x58C) != 0 {
        return flee_point_create_sub_task(this, 0x2CA, ped);
    }
    let p = ent_pos(ped);
    let dy = f32_of(x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(this, 0x1C)));
    let dx = f32_of(x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(this, 0x18)));
    let d = general_get_node_heading_from_vector(dx, dy);
    wr::<u8>(this, 0x40, d);
    flee_point_create_sub_task(this, 0x390, ped)
}

/// `CTaskComplexSmartFleePoint::ControlSubTask(CPed*)` (0x65C1E0, `ret 4`). Not a flee wander (0x390): quit any
/// look-at, keep the sub-task. Otherwise: scream (`Say(0x15B, 0, 0.1)`) while +0x24; the wander's move state = +0x30;
/// a new flee position (+0x41) restarts a running timer (+0x34 = now, +0x38 = +0x28) and re-aims the wander when
/// `ComputeFleeDir` changed (+0x40); else a timed-out timer (`CTaskTimer::IsOutOfTime`) or being farther than the
/// radius (+0x2C, squared as f32) from both the flee point (+0x18) and the start (+0xC) switches to stand still
/// (0xCB). Sprinting (state 6) peds not looking at anything look back at the flee point with a 6% chance
/// (`GetRandomNumberInRange(0, 100) <= 5`).
#[hook(addr = 0x65C1E0, conv = thiscall, name = "CTaskComplexSmartFleePoint::ControlSubTask", expect = "56 8B F1 57 8B 7E 08")]
pub fn flee_point_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let mut res = rd::<GPtr<u8>>(this, 8);
    if v_get_task_type(res) != 0x390 {
        if ik_is_looking(ik(), ped) != 0 {
            ik_abort_look_at(ik(), ped, 0xFA);
        }
        return rd::<GPtr<u8>>(this, 8);
    }
    if rd::<u8>(this, 0x24) != 0 {
        ped_say(ped, 0x15B, 0, f32::from_bits(0x3DCC_CCCD), 0, 0, 0);
    }
    let sub = rd::<GPtr<u8>>(this, 8);
    wr::<u32>(sub, 0xC, rd::<u32>(this, 0x30));
    if rd::<u8>(this, 0x41) != 0 {
        let started = rd::<u8>(this, 0x3C) != 0;
        wr::<u8>(this, 0x41, 0);
        if started {
            let t = rd::<u32>(this, 0x28);
            wr::<u32>(this, 0x34, time_ms());
            wr::<u32>(this, 0x38, t);
            wr::<u8>(this, 0x3C, 1);
        }
        let d = flee_point_compute_flee_dir(this, ped);
        if rd::<u8>(this, 0x40) != d {
            wr::<u8>(this, 0x40, d);
            wander_set_dir(sub, d as u32);
        }
        res = rd::<GPtr<u8>>(this, 8);
    } else if rd::<u8>(this, 0x3C) != 0 && task_timer_is_out_of_time(this.byte_add(0x34)) != 0 {
        res = flee_point_create_sub_task(this, 0xCB, ped);
    } else {
        let p = ent_pos(ped);
        let r = x(rd::<f32>(this, 0x2C));
        let r2 = f32_of(r.fmul(r));
        if dist2(this.byte_add(0x18), p).fcom(x(r2)).test_any(0x41) {
            res = rd::<GPtr<u8>>(this, 8);
        } else if !dist2(this.byte_add(0xC), p).fcom(x(r2)).test_any(0x41) {
            res = flee_point_create_sub_task(this, 0xCB, ped);
        }
    }
    if rd::<u32>(this, 0x30) == 6 && ik_is_looking(ik(), ped) == 0 && general_get_random_number_in_range(0, 0x64) <= 5 {
        ik_look_at(ik(), GPtr::from_addr(0x86F770), ped, GPtr::NULL, 0x7D0, -1, this.byte_add(0x18), 0, f32::from_bits(0x3E80_0000), 0x1F4, 3, 0);
    }
    res
}

// ---- CTaskComplexSmartFleeEntity ------------------------------------------------------------------------

/// `CTaskComplexSmartFleeEntity::CreateSubTask(int32 type)` (0x65C530, `ret 4`; the SEH handler push continues
/// through the island 0x401D31): 0xCB → `CTaskSimpleStandStill(_ftol((rand() & 0xFFFF)·[0x858B14]·[0x858B40]),
/// false, false, 8)` (rand at 0x65C637, only when the pool slot was allocated); 0x38E → timer (+0x34 = now, +0x38
/// = +0x28, started), the flee position +0x10 = the entity's position, `CTaskComplexSmartFleePoint(&+0x10, +0x24,
/// +0x20, +0x1C)`; anything else null.
#[hook(addr = 0x65C530, conv = thiscall, name = "CTaskComplexSmartFleeEntity::CreateSubTask", expect = "6A FF E9 FA 57 DA FF")]
pub fn flee_entity_create_sub_task_port(this: GPtr<u8>, ty: u32) -> GPtr<u8> {
    match ty {
        0xCB => {
            let t = task_new(0x20);
            if t.is_null() {
                return GPtr::NULL;
            }
            let r = sa_core::orig::at_site(0x65C637, rand) & 0xFFFF;
            let ms = ftol32(xi(r).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(0x858B40).read()));
            stand_still_ctor(t, (ms as u32) as i32, 0, 0, f32::from_bits(0x4100_0000))
        }
        0x38E => {
            let t = rd::<u32>(this, 0x28);
            wr::<u32>(this, 0x34, time_ms());
            wr::<u32>(this, 0x38, t);
            wr::<u8>(this, 0x3C, 1);
            let p = ent_pos(rd::<GPtr<u8>>(this, 0xC));
            wr::<u32>(this, 0x10, rd::<u32>(p, 0));
            wr::<u32>(this, 0x14, rd::<u32>(p, 4));
            wr::<u32>(this, 0x18, rd::<u32>(p, 8));
            let n = task_new(0x44);
            if n.is_null() {
                return GPtr::NULL;
            }
            let (a, b) = (rd::<u32>(this, 0x1C), rd::<u32>(this, 0x20));
            flee_point_ctor(n, this.byte_add(0x10), rd::<u8>(this, 0x24) as u32, b, a)
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexSmartFleeEntity::CreateFirstSubTask(CPed*)` (0x65C6F0, `ret 4`): null without an entity (+0xC);
/// else an interesting event 0xD for the ped, a 3 s look at the entity (bone 5 for a ped entity, type bits of
/// +0x36 == 3, else -1) unless in a vehicle or already looking, and the flee-point sub-task (0x38E).
#[hook(addr = 0x65C6F0, conv = thiscall, name = "CTaskComplexSmartFleeEntity::CreateFirstSubTask", expect = "57 8B F9 8B 47 0C")]
pub fn flee_entity_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(this, 0xC) == 0 {
        return GPtr::NULL;
    }
    interesting_events_add(GPtr::from_addr(0xC0B058), 0xD, ped);
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 && ik_is_looking(ik(), ped) == 0 {
        let e = rd::<GPtr<u8>>(this, 0xC);
        let bone = if rd::<u8>(e, 0x36) & 7 == 3 { 5 } else { -1 };
        ik_look_at(ik(), GPtr::from_addr(0x86F7B0), ped, GPtr::from_addr((e.addr() as u32) as usize), 0xBB8, bone as i32, GPtr::NULL, 1, f32::from_bits(0x3E80_0000), 0x1F4, 3, 0);
    }
    flee_entity_create_sub_task(this, 0x38E)
}

/// `CTaskComplexSmartFleeEntity::ControlSubTask(CPed*)` (0x65C780, `ret 4`): without an entity the sub-task is asked
/// to abort (`MakeAbortable(ped, 0, null)`); a flee-point sub-task (0x38E) gets the move state (+0x30 → sub +0x30)
/// and, once the started timer (+0x34/+0x38, restart flag +0x3D) expired and the entity moved farther than +0x2C
/// from the stored position ((dz² + dy²) + dx² > r², unrounded), the timer restarts, the position is refreshed and
/// passed on (`SetFleePosition(&+0x10, +0x20, +0x24)`), and a running ped (state > 4) raises a stack
/// `CEventSeenPanickedPed` in the global event group. Always the current sub-task.
#[hook(addr = 0x65C780, conv = thiscall, name = "CTaskComplexSmartFleeEntity::ControlSubTask", expect = "64 A1 00 00 00 00")]
pub fn flee_entity_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    if rd::<u32>(this, 0xC) == 0 {
        v_make_abortable(sub, ped, 0, 0);
        return rd::<GPtr<u8>>(this, 8);
    }
    if v_get_task_type(sub) != 0x38E {
        return rd::<GPtr<u8>>(this, 8);
    }
    wr::<u32>(rd::<GPtr<u8>>(this, 8), 0x30, rd::<u32>(this, 0x30));
    if rd::<u8>(this, 0x3C) == 0 {
        return rd::<GPtr<u8>>(this, 8);
    }
    if rd::<u8>(this, 0x3D) != 0 {
        wr::<u32>(this, 0x34, time_ms());
        wr::<u8>(this, 0x3D, 0);
    }
    let end = rd::<u32>(this, 0x38).wrapping_add(rd::<u32>(this, 0x34));
    let now = time_ms();
    if now < end {
        return rd::<GPtr<u8>>(this, 8);
    }
    let ep = ent_pos(rd::<GPtr<u8>>(this, 0xC));
    let r = x(rd::<f32>(this, 0x2C));
    if r.fmul(r).fcom(dist2(this.byte_add(0x10), ep)).test_parity(5) {
        return rd::<GPtr<u8>>(this, 8);
    }
    let t = rd::<u32>(this, 0x28);
    wr::<u32>(this, 0x34, now);
    wr::<u32>(this, 0x38, t);
    wr::<u8>(this, 0x3C, 1);
    let p = ent_pos(rd::<GPtr<u8>>(this, 0xC));
    wr::<u32>(this, 0x10, rd::<u32>(p, 0));
    wr::<u32>(this, 0x14, rd::<u32>(p, 4));
    wr::<u32>(this, 0x18, rd::<u32>(p, 8));
    let scream = rd::<u8>(this, 0x24) as u32;
    flee_point_set_flee_position(sub, this.byte_add(0x10), rd::<u32>(this, 0x20), scream);
    if rd::<i32>(this, 0x30) > 4 {
        broadcast_seen_panicked_ped(ped);
    }
    rd::<GPtr<u8>>(this, 8)
}

/// A stack `CEventSeenPanickedPed(ped)` added to the global event group (`Add(ev, false)`), then destroyed.
pub(crate) fn broadcast_seen_panicked_ped(ped: GPtr<u8>) {
    let mut ev = [0u32; 0x11];
    let ep = buf(&mut ev);
    seen_panicked_ped_ctor(ep, ped);
    event_group_add(get_event_global_group(), ep, 0);
    seen_panicked_ped_dtor(ep);
}

/// `CTaskComplexFleeAnyMeans::CreateSubTask(eTaskType, CPed*)` (0x65D2E0, `ret 8`; the entry `nop; jmp` runs its
/// first instruction in the no-CD chunk 0x402333; FRONTIER-2 part 4, crossing 4000):
/// - 0x3A0 `CTaskComplexFleeShooting(+0xC, byte +0x24, +0x20, +0x1C, +0x44, +0x48, +0x28, +0x2C)`;
/// - 0x390 a `CTaskComplexWander(7, dir, 0, [0x86FCB0])` turned into the wander-flee (vtable 0x86F690), `dir` = the
///   task-type argument's word with its low byte replaced by `GetRandomNumberInRange(0, 8)` (the original writes the
///   byte over that argument slot);
/// - 0x2BE `CTaskComplexStealCar(+0x50)`; 0x2D4 `CTaskComplexCarDriveMission(ped's vehicle, null, 1, 3, 40.0)`;
/// - 0x385 `CTaskComplexGoToPointShooting(6, &vehicle pos, vehicle (+0x50), (0, 0, 0), bound radius + 0.35,
///   [0x86FC94])`;
/// - 0x516 and anything else: null (also when the pool is full).
#[hook(addr = 0x65D2E0, conv = thiscall, name = "CTaskComplexFleeAnyMeans::CreateSubTask", expect = "90 E9 4D 50 DA FF",
       unverified = "frontier2-8", route = "L2", hazards = "heap-writes")]
pub fn flee_any_means_create_sub_task(this: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> {
    let new_task = |size: u32| task_new(size);
    match ty {
        0x3A0 => {
            let t = new_task(0x54);
            if t.is_null() {
                return GPtr::NULL;
            }
            flee_shooting_ctor(t, rd::<u32>(this, 0xC), u32::from(rd::<u8>(this, 0x24)), rd::<u32>(this, 0x20),
                rd::<u32>(this, 0x1C), rd::<u32>(this, 0x44), rd::<u32>(this, 0x48), rd::<u32>(this, 0x28), rd::<u32>(this, 0x2C))
        }
        0x390 => {
            let t = new_task(0x28);
            if t.is_null() {
                return GPtr::NULL;
            }
            let r = general_get_random_number_in_range(0, 8) as u32;
            let dir = (ty & 0xFFFF_FF00) | (r & 0xFF);
            wander_ctor(t, 7, dir, 0, g::<u32>(0x86_FCB0).read());
            wr::<u32>(t, 0, 0x86_F690);
            t
        }
        0x2BE => {
            let t = new_task(0x20);
            if t.is_null() { GPtr::NULL } else { steal_car_ctor(t, rd::<GPtr<u8>>(this, 0x50)) }
        }
        0x2D4 => {
            let t = new_task(0x2C);
            if t.is_null() { GPtr::NULL } else { car_drive_mission_ctor(t, rd::<GPtr<u8>>(ped, 0x58C), GPtr::NULL, 1, 3, 0x4220_0000) }
        }
        0x385 => {
            let t = new_task(0x38);
            if t.is_null() {
                return GPtr::NULL;
            }
            let veh = rd::<GPtr<u8>>(this, 0x50);
            let mi = g::<GPtr<u8>>(0xA9_B0C8usize.wrapping_add((4 * i32::from(rd::<i16>(veh, 0x22))) as isize as usize)).read();
            let col = rd::<GPtr<u8>>(mi, 0x14);
            let radius = x(rd::<f32>(col, 0x24)).fadd_m(sa_core::field::gf(0x85_8F9C)).to_f32();
            go_to_point_shooting_ctor(t, 6, ent_pos(veh).cast(), veh, 0, 0, 0, radius.to_bits(), g::<u32>(0x86_FC94).read())
        }
        _ => GPtr::NULL,
    }
}


/// A stack `CEventVehicleDamageWeapon(veh, ped, 0x16)` with its +0xE word = 0x2C2, added to `to`'s event group, then
/// destroyed as the original does (vtable 0x86D640 stored, `CEventVehicleDamage`'s destructor 0x4B1990).
fn flee_vehicle_damage_event(to: GPtr<u8>, veh: GPtr<u8>, ped: GPtr<u8>) {
    let mut ev = [0u32; 0x10];
    let ep: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    event_vehicle_damage_weapon_ctor(ep, veh, ped, 0x16);
    wr::<u16>(ep, 0xE, 0x2C2);
    event_group_add(rd::<GPtr<u8>>(to, 0x47C).byte_add(0x68), ep, 0);
    wr::<u32>(ep, 0, 0x86_D640);
    event_vehicle_damage_dtor(ep);
}

/// `CTaskComplexFleeAnyMeans::ControlSubTask(CPed*)` (0x65D780, `ret 4`; FRONTIER-2 part 4, crossing 4000). Without
/// a flee target (+0xC): null when the sub-task aborts (`MakeAbortable(ped, 0, null)`), else the sub-task. With one, a
/// running flee-shooting (0x3A0) or wander (0x390) sub-task (the type read twice when the first is not 0x3A0), the
/// vehicle scanner's closest vehicle, `(dz² + dy²) + dx²` of ped - target (floats) above `+0x4C²`, the normalised
/// direction to the vehicle (floats) in front of the ped (dot with the matrix forward, summed z, y, x, > 0) and
/// `CCarEnterExit::IsVehicleStealable(vehicle, ped)`: the vehicle becomes +0x50 (`CleanUpOldReference` of the old one,
/// `RegisterReference`). Shooting while fleeing (+0x40): the driver and each passenger (count +0x488 re-read) that is
/// not the player get a `CEventVehicleDamageWeapon(vehicle, ped, 0x16)` (its word +0xE = 0x2C2) and the next task is
/// go-to-point-shooting (0x385); otherwise the vehicle must pass its vtable +0xE4 (false) test and the task is steal
/// car (0x2BE). The new sub-task when the current one aborts, else the current one.
#[hook(addr = 0x65D780, conv = thiscall, name = "CTaskComplexFleeAnyMeans::ControlSubTask", expect = "6A FF 68 80 42 84 00",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes")]
pub fn flee_any_means_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = || rd::<GPtr<u8>>(this, 8);
    if rd::<u32>(this, 0xC) == 0 {
        return if v_make_abortable(sub(), ped, 0, 0) != 0 { GPtr::NULL } else { sub() };
    }
    let veh = scanner_closest_vehicle(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0xD4));
    if veh.is_null() {
        return sub();
    }
    if v_get_task_type(sub()) != 0x3A0 && v_get_task_type(sub()) != 0x390 {
        return sub();
    }
    let (pp, tp) = (ent_pos(ped), ent_pos(rd::<GPtr<u8>>(this, 0xC)));
    let d: [f32; 3] = core::array::from_fn(|k| x(rd::<f32>(pp, 4 * k as isize)).fsub_m(rd::<f32>(tp, 4 * k as isize)).to_f32());
    let dsq = x(d[2]).fmul_m(d[2]).fadd(x(d[1]).fmul_m(d[1])).fadd(x(d[0]).fmul_m(d[0]));
    let r = rd::<f32>(this, 0x4C);
    if !sa_math::fpu::lt(x(r).fmul_m(r), dsq) {
        return sub();
    }
    let (vp, pp) = (ent_pos(veh), ent_pos(ped));
    let mut v: [f32; 3] = core::array::from_fn(|k| x(rd::<f32>(vp, 4 * k as isize)).fsub_m(rd::<f32>(pp, 4 * k as isize)).to_f32());
    vector_normalise(GPtr::from_raw(v.as_mut_ptr()).cast());
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let dot = x(v[2]).fmul_m(rd::<f32>(m, 0x18)).fadd(x(v[1]).fmul_m(rd::<f32>(m, 0x14))).fadd(x(v[0]).fmul_m(rd::<f32>(m, 0x10)));
    if !sa_math::fpu::gt(dot, x(0.0)) || car_enter_exit_is_vehicle_stealable(veh, ped) == 0 {
        return sub();
    }
    let slot = this.byte_add(0x50);
    let old = rd::<GPtr<u8>>(slot, 0);
    if !old.is_null() {
        entity_clean_up_old_reference(old, slot);
    }
    wr::<GPtr<u8>>(slot, 0, veh);
    entity_register_reference(veh, slot);
    let ty = if rd::<u8>(this, 0x40) != 0 {
        let v = rd::<GPtr<u8>>(this, 0x50);
        let drv = rd::<GPtr<u8>>(v, 0x460);
        if !drv.is_null() && ped_is_player(drv) == 0 {
            flee_vehicle_damage_event(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x50), 0x460), rd::<GPtr<u8>>(this, 0x50), ped);
        }
        let mut i = 0isize;
        while i < isize::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x50), 0x488)) {
            let p = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x50), 0x464 + 4 * i);
            if !p.is_null() && ped_is_player(p) == 0 {
                flee_vehicle_damage_event(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x50), 0x464 + 4 * i), rd::<GPtr<u8>>(this, 0x50), ped);
            }
            i += 1;
        }
        0x385
    } else {
        if v_vehicle_e4(veh, 0) == 0 {
            return sub();
        }
        0x2BE
    };
    if v_make_abortable(sub(), ped, 0, 0) != 0 {
        flee_any_create_sub_task_o(this, ty, ped)
    } else {
        sub()
    }
}

/// `CTaskComplexFleeAnyMeans::CreateNextSubTask(CPed*)` (0x65D540, `ret 4`; FRONTIER-2 part 4, crossing 4000). By the
/// finished sub-task (jump table 0x65D634 / 0x65D648 over 0x2BE..0x3A0):
/// - 0x2BE steal car: in a vehicle (+0x46C bit 8 and +0x58C) `CreateSubTask(0x2D4)`, else `CreateFirstSubTask`; the
///   stolen-vehicle slot +0x50 is cleared after the call (gta-reversed clears it first);
/// - 0x2D4 car drive mission: in a vehicle `CreateSubTask(0x2D4)`, else `CreateFirstSubTask`;
/// - 0x385 go to point shooting: with a vehicle to steal (+0x50) `CreateSubTask(0x2BE)`, else +0x50 = 0 and
///   `CreateFirstSubTask`;
/// - 0x390 wander, 0x3A0 flee shooting: `CreateSubTask(0x516)` (finished); anything else null.
#[hook(addr = 0x65D540, conv = thiscall, name = "CTaskComplexFleeAnyMeans::CreateNextSubTask", expect = "56 8B F1 8B 4E 08",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes")]
pub fn flee_any_means_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let in_vehicle = || rd::<u32>(ped, 0x46C) & 0x100 != 0 && rd::<u32>(ped, 0x58C) != 0;
    match v_get_task_type(rd::<GPtr<u8>>(this, 8)) {
        0x2BE => {
            let r = if in_vehicle() { flee_any_create_sub_task_o(this, 0x2D4, ped) } else { v_create_first_sub_task(this, ped) };
            wr::<u32>(this, 0x50, 0);
            r
        }
        0x2D4 => {
            if in_vehicle() { flee_any_create_sub_task_o(this, 0x2D4, ped) } else { v_create_first_sub_task(this, ped) }
        }
        0x385 => {
            if rd::<u32>(this, 0x50) != 0 {
                return flee_any_create_sub_task_o(this, 0x2BE, ped);
            }
            wr::<u32>(this, 0x50, 0);
            v_create_first_sub_task(this, ped)
        }
        0x390 | 0x3A0 => flee_any_create_sub_task_o(this, 0x516, ped),
        _ => GPtr::NULL,
    }
}


/// The inlined `CTaskTimer::IsOutOfTime` of the flee-shooting gun timer (+0x48 start, +0x4C length, +0x50 started,
/// +0x51 reset requested): false while not started; a pending reset restarts it at now; then `now >= start + length`.
fn flee_gun_timer_out(this: GPtr<u8>) -> bool {
    if rd::<u8>(this, 0x50) == 0 {
        return false;
    }
    if rd::<u8>(this, 0x51) != 0 {
        wr::<u32>(this, 0x48, g::<u32>(0xB7_CB84).read());
        wr::<u8>(this, 0x51, 0);
    }
    g::<u32>(0xB7_CB84).read() >= rd::<u32>(this, 0x4C).wrapping_add(rd::<u32>(this, 0x48))
}

/// `CTaskComplexFleeShooting::ControlSubTask(CPed*)` (0x65CA90, `ret 4`; FRONTIER-2 part 4, crossing 4000). Without a
/// secondary attack task: move state (+0x30) sprint (7); once the gun timer is out and the ped's weapon aims with the
/// arm (info +0x18 bit 1) the timer restarts with the shoot time (+0x40) and a `CTaskSimpleUseGun(target, (0, 0, 0),
/// fire burst 3, burst 3, false)` becomes the secondary attack task. With one: move state run (6); a use-gun task
/// (0x3F9) fires a burst at the target (`ControlGun(ped, target, 3)`) until the timer is out, then, when its last command
/// (+0x10) was a burst, restarts the timer with the recover time (+0x44) and ends the burst (command 6). Then
/// `CTaskComplexSmartFleeEntity::ControlSubTask`.
#[hook(addr = 0x65CA90, conv = thiscall, name = "CTaskComplexFleeShooting::ControlSubTask", expect = "64 A1 00 00 00 00 6A FF 68 EB 40 84 00",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes")]
pub fn flee_shooting_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let tm = rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
    let attack = tm_get_task_secondary(tm, 0);
    if attack.is_null() {
        wr::<u32>(this, 0x30, 7);
        if flee_gun_timer_out(this) {
            let slot = isize::from(rd::<i8>(ped, 0x718));
            let wt = rd::<i32>(ped, 0x5A0 + slot * 0x1C);
            let wi = get_weapon_info(wt, ped_get_weapon_skill(ped));
            if (rd::<u32>(wi, 0x18) >> 1) & 1 != 0 {
                wr::<u32>(this, 0x48, g::<u32>(0xB7_CB84).read());
                wr::<u32>(this, 0x4C, rd::<u32>(this, 0x40));
                wr::<u8>(this, 0x50, 1);
                let t = task_new(0x3C);
                let t = if t.is_null() { GPtr::NULL } else { use_gun_ctor(t, rd::<GPtr<u8>>(this, 0xC), 0, 0, 0, 3, 3, 0) };
                tm_set_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 0);
            }
        }
    } else {
        wr::<u32>(this, 0x30, 6);
        if v_get_task_type(attack) == 0x3F9 {
            if !flee_gun_timer_out(this) {
                use_gun_control_gun(attack, ped, rd::<GPtr<u8>>(this, 0xC), 3);
            } else if rd::<u8>(attack, 0x10) == 3 {
                wr::<u32>(this, 0x48, g::<u32>(0xB7_CB84).read());
                wr::<u32>(this, 0x4C, rd::<u32>(this, 0x44));
                wr::<u8>(this, 0x50, 1);
                use_gun_control_gun(attack, ped, rd::<GPtr<u8>>(this, 0xC), 6);
            }
        }
    }
    flee_entity_control_sub_task_o(this, ped)
}


/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_FLEE_ANY_MEANS_CONTROL_SUB_TASK,
    HOOK_FLEE_ANY_MEANS_CREATE_NEXT_SUB_TASK,
    HOOK_FLEE_SHOOTING_CONTROL_SUB_TASK,
    HOOK_FLEE_ANY_MEANS_CREATE_SUB_TASK,
    HOOK_FLEE_POINT_CREATE_SUB_TASK_PORT,
    HOOK_FLEE_POINT_CREATE_FIRST_SUB_TASK,
    HOOK_FLEE_POINT_CONTROL_SUB_TASK,
    HOOK_FLEE_ENTITY_CREATE_SUB_TASK_PORT,
    HOOK_FLEE_ENTITY_CREATE_FIRST_SUB_TASK,
    HOOK_FLEE_ENTITY_CONTROL_SUB_TASK,
];
