//! `CTaskGoToVehicleAndLean::CreateNextSubTask` (0x664590) and `CTaskLeanOnVehicle::ProcessPed` (0x6648C0): an
//! ambient ped walks to a parked car and leans on it (Grove Street; reached on the native walk after the
//! frame-limiter toggle shifted the timing). Ported from the 1.0 US bytes; gta-reversed has no bodies for them
//! (`gr-unreversed`).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, eAbortPriority, const CEvent*)` (vtable slot 0x18)
    fn task_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CBE0, ST0)
    fn general_get_radian_angle_between_points(x1: u32, y1: u32, x2: u32, y2: u32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn general_limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => crate::mvpa_ai1::task_operator_new;
    /// `CTaskSimpleAchieveHeading(float heading, float changeRate, float tolerance)` (0x667E20)
    fn achieve_heading_ctor(t: GPtr<u8>, heading: u32, rate: u32, tolerance: u32) -> GPtr<u8> = thiscall 0x667E20 => unported;
    /// `CTaskGoToVehicleAndLean::DoTidyUp(CPed*)` (0x660F60)
    fn go_to_vehicle_and_lean_do_tidy_up(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x660F60 => unported;
    /// `CTaskLeanOnVehicle(CEntity* vehicle, int32 leanAnimDuration, uint8)` (0x660F90)
    fn lean_on_vehicle_ctor(t: GPtr<u8>, veh: GPtr<u8>, duration: u32, a: u32) -> GPtr<u8> = thiscall 0x660F90 => unported;
    /// `CTaskSimpleStandStill(int32 time, bool looped, bool keep, float delta)` (0x62F310)
    fn stand_still_ctor(t: GPtr<u8>, time: u32, looped: u32, keep: u32, delta: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CPed::SetMoveState(eMoveState)` (0x5DEC00)
    fn ped_set_move_state(ped: GPtr<u8>, state: u32) = thiscall 0x5DEC00 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    fn clump_get_association(clump: u32, anim: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610)
    fn anim_blend_animation(clump: u32, group: u32, anim: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimBlendAssociation::SetDeleteCallback(callback, data)` (0x4CEBC0)
    fn assoc_set_delete_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBC0 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(callback, data)` (0x4CEBE0)
    fn assoc_set_finish_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBE0 => unported;
    /// `CTaskTimer::IsOutOfTime()` (0x420E30)
    fn task_timer_is_out_of_time(t: GPtr<u8>) -> u8 = thiscall 0x420E30 => unported;
}

/// `CDefaultAnimCallback::DefaultAnimCB` (0x4D1390)
const DEFAULT_ANIM_CB: u32 = 0x4D_1390;
/// `CTaskLeanOnVehicle::FinishAnimCB` (0x661160)
const LEAN_FINISH_ANIM_CB: u32 = 0x66_1160;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7_CB84;
/// lean animations (group 0x34): start, loop, finish
const ANIM_LEAN_IN: i32 = 0x12F;
const ANIM_LEAN_LOOP: i32 = 0x130;
const ANIM_LEAN_OUT: i32 = 0x131;
/// 1000.0, 4.0, 3.0, -8.0
const F_1000: u32 = 0x447A_0000;
const F_4: u32 = 0x4080_0000;
const F_3: u32 = 0x4040_0000;
const F_MINUS_8: u32 = 0xC100_0000;

/// `CTaskGoToVehicleAndLean::CreateNextSubTask(CPed* ped)` (0x664590, thiscall, `ret 4`, SEH frame). Fields: +8 the
/// sub-task, +0xC the vehicle, +0x10 the lean duration, +0x14 "lean" flag, +0x15 "on the left side" flag.
/// - no vehicle: the sub-task (if any) `MakeAbortable(ped, ABORT_PRIORITY_IMMEDIATE, NULL)`; NULL.
/// - sub-task 0x387 (the go-to-car-door step): the vehicle matrix's right vector (+0x14 → +0, +4, +8), negated (x87
///   `fchs`, stored as floats) unless +0x15; `heading = GetRadianAngleBetweenPoints(x, y, 0, 0)` (stored as a float),
///   `LimitRadianAngle(heading)` with its result discarded, `new CTaskSimpleAchieveHeading(heading, [0x86FC7C],
///   [0x86FC80])` (0x18 bytes).
/// - sub-task 0x386 or 0xCB: with +0x14 `new CTaskLeanOnVehicle(vehicle, +0x10, 0)` (0x2C), else
///   `new CTaskSimpleStandStill(500, false, false, 8.0)` (0x20).
/// - anything else: `DoTidyUp(ped)`, NULL.
///
/// A failed allocation returns NULL.
#[hook(addr = 0x664590, conv = thiscall, name = "CTaskGoToVehicleAndLean::CreateNextSubTask", this = "CTaskGoToVehicleAndLean",
       expect = "64 A1 00 00 00 00 6A FF",
       unverified = "framelimit-ai", route = "L2", hazards = "heap-writes")]
pub fn go_to_vehicle_and_lean_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    if rd::<u32>(this, 0xC) == 0 {
        if !sub.is_null() {
            task_make_abortable(sub, ped, 2, 0);
        }
        return GPtr::NULL;
    }
    if task_get_type(sub) == 0x387 {
        let m = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 0x14);
        let (mut vx, mut vy) = (rd::<f32>(m, 0), rd::<f32>(m, 4));
        if rd::<u8>(this, 0x15) == 0 {
            vx = x(vx).fchs().to_f32();
            vy = x(vy).fchs().to_f32();
        }
        let heading = general_get_radian_angle_between_points(vx.to_bits(), vy.to_bits(), 0, 0) as f32;
        general_limit_radian_angle(heading.to_bits());
        let t = task_new(0x18);
        if t.is_null() {
            return GPtr::NULL;
        }
        return achieve_heading_ctor(t, heading.to_bits(), g::<u32>(0x86FC7C).read(), g::<u32>(0x86FC80).read());
    }
    let ty = task_get_type(rd::<GPtr<u8>>(this, 8));
    if ty != 0x386 && task_get_type(rd::<GPtr<u8>>(this, 8)) != 0xCB {
        go_to_vehicle_and_lean_do_tidy_up(this, ped);
        return GPtr::NULL;
    }
    if rd::<u8>(this, 0x14) != 0 {
        let t = task_new(0x2C);
        if t.is_null() {
            return GPtr::NULL;
        }
        return lean_on_vehicle_ctor(t, rd::<GPtr<u8>>(this, 0xC), rd::<u32>(this, 0x10), 0);
    }
    let t = task_new(0x20);
    if t.is_null() {
        return GPtr::NULL;
    }
    stand_still_ctor(t, 500, 0, 0, 0x4100_0000)
}

/// `CTaskLeanOnVehicle::ProcessPed(CPed* ped)` (0x6648C0, thiscall, `ret 4`). Fields: +8 the vehicle, +0xC the lean
/// duration, +0x10 "skip the lean-in", +0x14 the animation, +0x18 the requested animation, +0x1C `CTaskTimer` (start,
/// interval, started +0x24), +0x28 "stop when the lean-out ends", +0x29 "leave now", +0x2A "finished".
/// No vehicle sets +0x2A. `ped->m_pEntityIgnoredCollision` (+0x128) = the vehicle (also when NULL). The current
/// animation id is its word +0x2C (-1 without one). Finished (+0x2A), or +0x28 with no lean-out (0x131) on the clump:
/// +0x128 cleared, returns 1. Otherwise `SetMoveState(PEDMOVE_STILL)`; with +0x29: lean-in gets blend delta -8.0, a
/// lean loop gets the default delete callback and the lean-out is blended (1000.0), a lean-out gets speed 3.0. Then:
/// no animation: requested -1 without +0x10 blends the lean-in (4.0); requested -1 with +0x10, or the lean-in
/// requested, starts the timer when the duration is >= 0 (now, duration) and blends the lean loop (1000.0); any other
/// request returns 0. With an animation: a started timer that ran out while it is not the lean-out gets the default
/// delete callback and the lean-out is blended (1000.0). Every blended animation is stored at +0x14 with
/// `FinishAnimCB(this)` as its finish callback. Returns 0.
#[hook(addr = 0x6648C0, conv = thiscall, name = "CTaskLeanOnVehicle::ProcessPed", this = "CTaskLeanOnVehicle",
       expect = "55 56 8B F1 8B 46 08",
       unverified = "framelimit-ai", route = "L2", hazards = "heap-writes")]
pub fn lean_on_vehicle_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let veh = rd::<u32>(this, 8);
    if veh == 0 {
        wr::<u8>(this, 0x2A, 1);
    }
    wr::<u32>(ped, 0x128, veh);
    let anim = rd::<GPtr<u8>>(this, 0x14);
    let id = if anim.is_null() { -1 } else { i32::from(rd::<i16>(anim, 0x2C)) };
    let finish = |ped: GPtr<u8>| {
        wr::<u32>(ped, 0x128, 0);
        1
    };
    if rd::<u8>(this, 0x2A) != 0 {
        return finish(ped);
    }
    ped_set_move_state(ped, 1);
    if rd::<u8>(this, 0x28) != 0 && clump_get_association(rd::<u32>(ped, 0x18), ANIM_LEAN_OUT as u32).is_null() {
        return finish(ped);
    }
    let blend = |this: GPtr<u8>, ped: GPtr<u8>, anim: i32, delta: u32| {
        let a = anim_blend_animation(rd::<u32>(ped, 0x18), 0x34, anim as u32, delta);
        wr::<GPtr<u8>>(this, 0x14, a);
        assoc_set_finish_callback(a, LEAN_FINISH_ANIM_CB, this.addr() as u32);
        0
    };
    if rd::<u8>(this, 0x29) != 0 {
        if id == ANIM_LEAN_IN {
            wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x1C, F_MINUS_8);
        } else if id == ANIM_LEAN_LOOP {
            assoc_set_delete_callback(rd::<GPtr<u8>>(this, 0x14), DEFAULT_ANIM_CB, 0);
            return blend(this, ped, ANIM_LEAN_OUT, F_1000);
        } else if id == ANIM_LEAN_OUT {
            wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x24, F_3);
        }
    }
    let anim = rd::<GPtr<u8>>(this, 0x14);
    if anim.is_null() {
        let req = rd::<i32>(this, 0x18);
        if req == -1 && rd::<u8>(this, 0x10) == 0 {
            return blend(this, ped, ANIM_LEAN_IN, F_4);
        }
        if req != -1 && req != ANIM_LEAN_IN {
            return 0;
        }
        let d = rd::<i32>(this, 0xC);
        if d >= 0 {
            wr::<u32>(this, 0x1C, g::<u32>(TIME_MS).read());
            wr::<i32>(this, 0x20, d);
            wr::<u8>(this, 0x24, 1);
        }
        return blend(this, ped, ANIM_LEAN_LOOP, F_1000);
    }
    if rd::<u8>(this, 0x24) != 0
        && task_timer_is_out_of_time(this.byte_add(0x1C)) != 0
        && rd::<i16>(rd::<GPtr<u8>>(this, 0x14), 0x2C) != ANIM_LEAN_OUT as i16
    {
        assoc_set_delete_callback(rd::<GPtr<u8>>(this, 0x14), DEFAULT_ANIM_CB, 0);
        return blend(this, ped, ANIM_LEAN_OUT, F_1000);
    }
    0
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_GO_TO_VEHICLE_AND_LEAN_CREATE_NEXT_SUB_TASK, HOOK_LEAN_ON_VEHICLE_PROCESS_PED];
