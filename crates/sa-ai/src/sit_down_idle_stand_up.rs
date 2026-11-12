//! Sitting at an attractor (batch `attr-1`): `CTaskComplexSitDownThenIdleThenStandUp` (vtable 0x86DF60; 0x14 bytes:
//! +0xC duration, +0x10 sit-on-step, +0x11 instant, +0x12 the "attractors" anim block reference taken) and its three
//! simple tasks `CTaskSimpleSitDown` (0x86DEF4), `CTaskSimpleSitIdle` (0x86DF18) and `CTaskSimpleStandUp` (0x86DF3C).
//! Ported from the 1.0 US bytes (gta-reversed as a reading aid; where it differs, the bytes are followed). Unverified
//! (docs/verify/HOWTO.md).
//!
//! Simple task layouts: sit down / stand up (0x10 bytes) +8 sit-on-step, +9 anim finished, +0xC the anim; sit idle
//! (0x20 bytes) +8 sit-after-step, +0xC duration, +0x10 the anim, +0x14 / +0x18 / +0x1C / +0x1D a `CTaskTimer`
//! (start, interval, started, changed).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskComplex::CTaskComplex()` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CTaskComplex::~CTaskComplex()` (0x61A3D0)
    fn task_complex_dtor(t: GPtr<u8>) = thiscall 0x61A3D0 => unported;
    /// `CTaskSimple::CTaskSimple()` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int priority, const CEvent*)` (vtable slot 0x18, `ret 0xC`)
    fn task_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskSimpleSitDown(bool sitOnStep)` (0x631030, `ret 4`)
    fn sit_down_ctor(t: GPtr<u8>, on_step: u32) -> GPtr<u8> = thiscall 0x631030 => unported;
    /// `CTaskSimpleSitIdle(int durationMs, bool sitAfterStep)` (0x631160, `ret 8`)
    fn sit_idle_ctor(t: GPtr<u8>, ms: u32, on_step: u32) -> GPtr<u8> = thiscall 0x631160 => unported;
    /// `CTaskSimpleSitIdle::StartAnim(CPed*)` (0x631260, `ret 4`)
    fn sit_idle_start_anim(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x631260 => unported;
    /// `CTaskSimpleStandStill(int ms, bool looped, bool useAnimIdle, float blend)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, ms: i32, looped: u32, idle: u32, blend: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990)
    fn anim_get_block_index(name: GPtr<u8>) -> i32 = cdecl 0x4D3990 => unported;
    /// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0)
    fn anim_add_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0)
    fn anim_remove_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CStreaming::RequestModel(int32 id, int32 flags)` (0x4087E0)
    fn streaming_request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610)
    fn anim_blend_animation(clump: u32, group: u32, id: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(cb, data)` (0x4CEBE0, `ret 8`)
    fn anim_set_finish_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBE0 => unported;
    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0, `ret 4`)
    fn ik_is_looking(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::AbortLookAt(CPed*, int32 blend)` (0x618280, `ret 8`)
    fn ik_abort_look_at(m: GPtr<u8>, ped: GPtr<u8>, blend: i32) = thiscall 0x618280 => unported;
    /// `IKChainManager_c::LookAt(name, ped, target, time, bone, pos, bool, speed, blend, priority, bool)` (0x618970,
    /// `ret 0x2C`)
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, pos: GPtr<u8>, b: u32, speed: u32, blend: i32, prio: i32, b2: u32) = thiscall 0x618970 => unported;
    /// `CPedScanner::GetClosestPedInRange()` (0x5FFF20)
    fn scanner_closest_ped(s: GPtr<u8>) -> GPtr<u8> = thiscall 0x5FFF20 => unported;
    /// `CPlaceable::GetForward(CVector* out)` (0x41CCB0, `ret 4`)
    fn placeable_get_forward(p: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x41CCB0 => unported;
    /// `CPlaceable::GetRight(CVector* out)` (0x41CC70, `ret 4`)
    fn placeable_get_right(p: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x41CC70 => unported;
    /// `CGeneral::GetRandomNumberInRange(int, int)` (0x407180; draws its own rand)
    fn random_int(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    fn create_sub_task_o(t: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x631550 => sit_create_sub_task;
    fn sit_down_start_anim_o(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x6379F0 => sit_down_start_anim;
    fn stand_up_start_anim_o(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x631410 => stand_up_start_anim;
}

/// `"attractors"` (the anim block name)
const ATTRACTORS: usize = 0x86DF8C;
/// `"TaskSitIdle"` (the look-at owner name)
const TASK_SIT_IDLE: u32 = 0x86E624;
/// `g_ikChainMan`
const IK_CHAIN_MAN: usize = 0xC15448;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `CAnimManager::ms_aAnimBlocks[0].bLoaded` (0x20-byte blocks)
const ANIM_BLOCK_LOADED: usize = 0xB5D4B0;
/// `CDefaultAnimCallback::DefaultAnimCB`
const DEFAULT_ANIM_CB: u32 = 0x4D1390;

fn ik() -> GPtr<u8> {
    GPtr::from_addr(IK_CHAIN_MAN)
}

/// The placeable's position: the matrix's (+0x14) +0x30, else the simple transform's +4.
fn pos_of(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

// ---------------------------------------------------------------------------------------------------------------
// CTaskComplexSitDownThenIdleThenStandUp

/// `CTaskComplexSitDownThenIdleThenStandUp::~CTaskComplexSitDownThenIdleThenStandUp()` (0x6314A0, thiscall, SEH
/// frame): vtable 0x86DF60; when +0x12 is set, `RemoveAnimBlockRef(GetAnimationBlockIndex("attractors"))` and +0x12 =
/// 0; then `CTaskComplex::~CTaskComplex`.
#[hook(addr = 0x6314A0, conv = thiscall, name = "CTaskComplexSitDownThenIdleThenStandUp::Destructor",
       expect = "6A FF 68 C8 04 84 00", unverified = "attr-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn sit_complex_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86DF60);
    if rd::<u8>(this, 0x12) != 0 {
        anim_remove_block_ref(anim_get_block_index(GPtr::from_addr(ATTRACTORS)));
        wr::<u8>(this, 0x12, 0);
    }
    task_complex_dtor(this);
}

/// `CTaskComplexSitDownThenIdleThenStandUp::MakeAbortable(CPed*, int, const CEvent*)` (0x631510, thiscall, `ret 0xC`):
/// priority 2 (immediate) tail-jumps to the sub-task's `MakeAbortable(ped, 2, event)`; otherwise the duration (+0xC) =
/// 0 and the sub-task's `MakeAbortable(ped, 0, event)` (priority 0, not the caller's), result false.
#[hook(addr = 0x631510, conv = thiscall, name = "CTaskComplexSitDownThenIdleThenStandUp::MakeAbortable",
       expect = "83 7C 24 08 02 75 10", unverified = "attr-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn sit_complex_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 {
    if prio == 2 {
        return task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 2, ev);
    }
    wr::<u32>(this, 0xC, 0);
    task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 0, ev);
    0
}

/// `CTaskComplexSitDownThenIdleThenStandUp::CreateSubTask(eTaskType)` (0x631550, thiscall, `ret 4`): 0xDC a
/// `CTaskSimpleSitDown(+0x10)` (0x10 bytes), 0xDD a `CTaskSimpleSitIdle(+0xC, +0x10)` (0x20), 0xDE a
/// `CTaskSimpleStandUp(+0x10)` (0x10, constructor inlined: vtable 0x86DF3C, +9 = 0, +0xC = 0); anything else null.
#[hook(addr = 0x631550, conv = thiscall, name = "CTaskComplexSitDownThenIdleThenStandUp::CreateSubTask",
       expect = "64 A1 00 00 00 00 6A FF", unverified = "attr-1", route = "L2", hazards = "heap-writes",
       recs = "grove-walk")]
pub fn sit_create_sub_task(this: GPtr<u8>, ty: u32) -> GPtr<u8> {
    match ty {
        0xDC => {
            let t = task_new(0x10);
            if t.is_null() { GPtr::NULL } else { sit_down_ctor(t, rd::<u8>(this, 0x10) as u32) }
        }
        0xDD => {
            let t = task_new(0x20);
            if t.is_null() {
                GPtr::NULL
            } else {
                sit_idle_ctor(t, rd::<u32>(this, 0xC), rd::<u8>(this, 0x10) as u32)
            }
        }
        0xDE => {
            let t = task_new(0x10);
            if t.is_null() {
                return GPtr::NULL;
            }
            let on_step = rd::<u8>(this, 0x10);
            task_simple_ctor(t);
            wr::<u32>(t, 0, 0x86DF3C);
            wr::<u8>(t, 8, on_step);
            wr::<u8>(t, 9, 0);
            wr::<u32>(t, 0xC, 0);
            t
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexSitDownThenIdleThenStandUp::Clone()` (0x636390, thiscall): a new 0x14-byte task with the same
/// duration and flags, +0x12 = 0.
#[hook(addr = 0x636390, conv = thiscall, name = "CTaskComplexSitDownThenIdleThenStandUp::Clone",
       expect = "6A FF 68 5B 0C 84 00", unverified = "attr-1", route = "L2", hazards = "heap-writes",
       recs = "grove-walk")]
pub fn sit_complex_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0x14);
    if t.is_null() {
        return GPtr::NULL;
    }
    let on_step = rd::<u8>(this, 0x10);
    let instant = rd::<u8>(this, 0x11);
    let ms = rd::<u32>(this, 0xC);
    task_complex_ctor(t);
    wr::<u8>(t, 0x11, instant);
    wr::<u32>(t, 0, 0x86DF60);
    wr::<u32>(t, 0xC, ms);
    wr::<u8>(t, 0x10, on_step);
    wr::<u8>(t, 0x12, 0);
    t
}

/// `CTaskComplexSitDownThenIdleThenStandUp::CreateNextSubTask(CPed*)` (0x637DB0, thiscall, `ret 4`), on the
/// sub-task's type: stand still (0xCB) → when sitting on a step without the anim block yet, another
/// `CTaskSimpleStandStill(100, 0, 0, 8.0)`; else a sit idle (instant) or `CreateSubTask(0xDC)`; sit down (0xDC) → a
/// `CTaskSimpleSitIdle(+0xC, +0x10)`; sit idle (0xDD) → `CreateSubTask(0xDE)`; anything else null.
#[hook(addr = 0x637DB0, conv = thiscall, name = "CTaskComplexSitDownThenIdleThenStandUp::CreateNextSubTask",
       expect = "6A FF 68 9C 10 84 00", unverified = "attr-1", route = "L2", hazards = "heap-writes",
       recs = "grove-walk")]
pub fn sit_create_next_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    let new_sit_idle = |this: GPtr<u8>| {
        let t = task_new(0x20);
        if t.is_null() {
            GPtr::NULL
        } else {
            sit_idle_ctor(t, rd::<u32>(this, 0xC), rd::<u8>(this, 0x10) as u32)
        }
    };
    match task_get_type(rd::<GPtr<u8>>(this, 8)) {
        0xCB => {
            if rd::<u8>(this, 0x10) != 0 && rd::<u8>(this, 0x12) == 0 {
                let t = task_new(0x20);
                return if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, 100, 0, 0, 0x4100_0000) };
            }
            if rd::<u8>(this, 0x11) != 0 { new_sit_idle(this) } else { create_sub_task_o(this, 0xDC) }
        }
        0xDC => new_sit_idle(this),
        0xDD => create_sub_task_o(this, 0xDE),
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexSitDownThenIdleThenStandUp::ControlSubTask(CPed*)` (0x631730, thiscall, `ret 4`): sitting on a step
/// without the "attractors" block reference: when the block is loaded, `AddAnimBlockRef` and +0x12 = 1, else
/// `CStreaming::RequestModel(index + 0x63E7, 8)`. Returns the sub-task.
#[hook(addr = 0x631730, conv = thiscall, name = "CTaskComplexSitDownThenIdleThenStandUp::ControlSubTask",
       expect = "56 8B F1 8A 46 12 84 C0", unverified = "attr-1", route = "L2", hazards = "none",
       recs = "grove-walk")]
pub fn sit_control_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u8>(this, 0x12) == 0 && rd::<u8>(this, 0x10) != 0 {
        let i = anim_get_block_index(GPtr::from_addr(ATTRACTORS));
        let loaded = g::<u8>(ANIM_BLOCK_LOADED.wrapping_add((i as usize).wrapping_shl(5))).read();
        if loaded == 1 {
            anim_add_block_ref(i);
            wr::<u8>(this, 0x12, 1);
        } else {
            streaming_request_model(i.wrapping_add(0x63E7), 8);
        }
    }
    rd::<GPtr<u8>>(this, 8)
}

// ---------------------------------------------------------------------------------------------------------------
// CTaskSimpleSitDown / CTaskSimpleStandUp

fn blend_with_callback(this: GPtr<u8>, ped: GPtr<u8>, step_anim: u32, seat_anim: u32, cb: u32) {
    let clump = rd::<u32>(ped, 0x18);
    let a = if rd::<u8>(this, 8) != 0 {
        anim_blend_animation(clump, 0x35, step_anim, 0x4080_0000)
    } else {
        anim_blend_animation(clump, 0, seat_anim, 0x4080_0000)
    };
    wr::<GPtr<u8>>(this, 0xC, a);
    anim_set_finish_callback(a, cb, this.addr() as u32);
}

/// `CTaskSimpleSitDown::StartAnim(CPed*)` (0x6379F0, thiscall, `ret 4`): blends (0x35, 0x134) on a step, else (0,
/// 0x94), delta 4.0, into +0xC; finish callback 0x631150 with the task (no null check).
#[hook(addr = 0x6379F0, conv = thiscall, name = "CTaskSimpleSitDown::StartAnim", expect = "56 8B F1 8A 46 08 84 C0",
       unverified = "attr-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn sit_down_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    blend_with_callback(this, ped, 0x134, 0x94, 0x631150);
}

/// `CTaskSimpleStandUp::StartAnim(CPed*)` (0x631410, thiscall, `ret 4`): as the sit-down one with (0x35, 0x135) /
/// (0, 0x95) and callback 0x631400.
#[hook(addr = 0x631410, conv = thiscall, name = "CTaskSimpleStandUp::StartAnim", expect = "56 8B F1 8A 46 08 84 C0",
       unverified = "attr-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn stand_up_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    blend_with_callback(this, ped, 0x135, 0x95, 0x631400);
}

/// `CTaskSimpleSitDown::ProcessPed(CPed*)` (0x639960, thiscall, `ret 4`): true once the anim finished (+9); starts
/// the anim when there is none.
#[hook(addr = 0x639960, conv = thiscall, name = "CTaskSimpleSitDown::ProcessPed", expect = "8A 41 09 84 C0 74 05 B0",
       unverified = "attr-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn sit_down_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 9) != 0 {
        return 1;
    }
    if rd::<u32>(this, 0xC) == 0 {
        sit_down_start_anim_o(this, ped);
    }
    0
}

/// `CTaskSimpleStandUp::ProcessPed(CPed*)` (0x637D60, thiscall, `ret 4`): as the sit-down one.
#[hook(addr = 0x637D60, conv = thiscall, name = "CTaskSimpleStandUp::ProcessPed", expect = "8A 41 09 84 C0 74 05 B0",
       unverified = "attr-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn stand_up_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 9) != 0 {
        return 1;
    }
    if rd::<u32>(this, 0xC) == 0 {
        stand_up_start_anim_o(this, ped);
    }
    0
}

fn clone_sit_or_stand(this: GPtr<u8>, vtable: u32) -> GPtr<u8> {
    let t = task_new(0x10);
    if t.is_null() {
        return GPtr::NULL;
    }
    let on_step = rd::<u8>(this, 8);
    task_simple_ctor(t);
    wr::<u8>(t, 8, on_step);
    wr::<u32>(t, 0, vtable);
    wr::<u8>(t, 9, 0);
    wr::<u32>(t, 0xC, 0);
    t
}

/// `CTaskSimpleSitDown::Clone()` (0x636200, thiscall): a new 0x10-byte sit-down task with the same flag.
#[hook(addr = 0x636200, conv = thiscall, name = "CTaskSimpleSitDown::Clone", expect = "6A FF 68 FB 0B 84 00",
       unverified = "attr-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn sit_down_clone(this: GPtr<u8>) -> GPtr<u8> {
    clone_sit_or_stand(this, 0x86DEF4)
}

/// `CTaskSimpleStandUp::Clone()` (0x636310, thiscall): a new 0x10-byte stand-up task with the same flag.
#[hook(addr = 0x636310, conv = thiscall, name = "CTaskSimpleStandUp::Clone", expect = "6A FF 68 3B 0C 84 00",
       unverified = "attr-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn stand_up_clone(this: GPtr<u8>) -> GPtr<u8> {
    clone_sit_or_stand(this, 0x86DF3C)
}

// ---------------------------------------------------------------------------------------------------------------
// CTaskSimpleSitIdle

/// `CTaskSimpleSitIdle::Clone()` (0x636280, thiscall): a new 0x20-byte task with the same flag (+8) and duration
/// (+0xC), no anim, a cleared timer.
#[hook(addr = 0x636280, conv = thiscall, name = "CTaskSimpleSitIdle::Clone", expect = "6A FF 68 1B 0C 84 00",
       unverified = "attr-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn sit_idle_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0x20);
    if t.is_null() {
        return GPtr::NULL;
    }
    let on_step = rd::<u8>(this, 8);
    let ms = rd::<u32>(this, 0xC);
    task_simple_ctor(t);
    wr::<u32>(t, 0xC, ms);
    wr::<u32>(t, 0x10, 0);
    wr::<u32>(t, 0, 0x86DF18);
    wr::<u8>(t, 8, on_step);
    wr::<u32>(t, 0x14, 0);
    wr::<u32>(t, 0x18, 0);
    wr::<u8>(t, 0x1C, 0);
    wr::<u8>(t, 0x1D, 0);
    t
}

/// `CTaskSimpleSitIdle::MakeAbortable(CPed*, int, const CEvent*)` (0x6311E0, thiscall, `ret 0xC`). Immediate (2): the
/// anim's blend delta (+0x1C) = -1000.0, default finish callback, anim = null; the look-at is aborted (750 ms) when
/// the ped is looking; true. Otherwise the duration (+0xC) and the timer interval (+0x18) become -1 (the original
/// overwrites the duration too), the timer starts now; false.
#[hook(addr = 0x6311E0, conv = thiscall, name = "CTaskSimpleSitIdle::MakeAbortable", expect = "83 7C 24 08 02 56 8B F1",
       unverified = "attr-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn sit_idle_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, prio: i32, _ev: GPtr<u8>) -> u8 {
    if prio != 2 {
        wr::<i32>(this, 0xC, -1);
        let now = g::<u32>(TIME_MS).read();
        wr::<i32>(this, 0x18, -1);
        wr::<u32>(this, 0x14, now);
        wr::<u8>(this, 0x1C, 1);
        return 0;
    }
    let a = rd::<GPtr<u8>>(this, 0x10);
    if !a.is_null() {
        wr::<u32>(a, 0x1C, 0xC47A_0000);
        anim_set_finish_callback(rd::<GPtr<u8>>(this, 0x10), DEFAULT_ANIM_CB, 0);
        wr::<u32>(this, 0x10, 0);
    }
    if ik_is_looking(ik(), ped) != 0 {
        ik_abort_look_at(ik(), ped, 750);
    }
    1
}

/// `CTaskSimpleSitIdle::ProcessPed(CPed*)` (0x637A60, thiscall, `ret 4`). Starts the anim when there is none. When the
/// ped is not looking and `ftol((rand() & 0xFFFF) * 2^-15 * 100.0) < 5`: look time = `(float)(3000 - ftol((rand() &
/// 0xFFFF) * 2^-15 * -5000.0))`; the scanner's closest ped, when in front (`dot(other - ped, forward) > 0.2`, products
/// summed z, y, x) and `GetRandomNumberInRange(0, 100) < 25`, is looked at (bone 5); otherwise a point `pos + forward *
/// (rand * [1/RAND_MAX] * 4 + 1) + right * (rand * [1/RAND_MAX] * 8 - 4)` (bone -1). Both look-ats: "TaskSitIdle",
/// speed 0.25, blend 750, priority 3. Then, when the timer runs (+0x1C) and is out of time, the look-at is aborted
/// (750) when looking and the task ends (true).
#[hook(addr = 0x637A60, conv = thiscall, name = "CTaskSimpleSitIdle::ProcessPed", expect = "83 EC 1C 53 8B D9 8B 43",
       unverified = "attr-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "rand() at 0x637A91, 0x637AD0, 0x637BCF, 0x637C38; GetRandomNumberInRange at 0x637B76")]
pub fn sit_idle_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u32>(this, 0x10) == 0 {
        sit_idle_start_anim(this, ped);
    }
    if ik_is_looking(ik(), ped) == 0 {
        let r = at_site(0x637A91, rand) & 0xFFFF;
        let v = ftol32(xi(r).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(0x858628).read()));
        if v < 5 {
            let scanner = rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x124);
            let other = scanner_closest_ped(scanner);
            let r = at_site(0x637AD0, rand) & 0xFFFF;
            let t = ftol32(xi(r).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(0x86E630).read()));
            let look_time = f32_of(xi(3000i32.wrapping_sub(t)));
            let mut buf = [0f32; 3];
            let out = GPtr::from_raw(buf.as_mut_ptr()).cast::<u8>();
            let mut looked = false;
            if !other.is_null() {
                let pp = pos_of(ped);
                let op = pos_of(other);
                let d = [
                    f32_of(x(rd::<f32>(op, 0)).fsub_m(rd::<f32>(pp, 0))),
                    f32_of(x(rd::<f32>(op, 4)).fsub_m(rd::<f32>(pp, 4))),
                    f32_of(x(rd::<f32>(op, 8)).fsub_m(rd::<f32>(pp, 8))),
                ];
                let f = placeable_get_forward(ped, out);
                let dot = x(d[2])
                    .fmul_m(rd::<f32>(f, 8))
                    .fadd(x(d[1]).fmul_m(rd::<f32>(f, 4)))
                    .fadd(x(d[0]).fmul_m(rd::<f32>(f, 0)));
                if !dot.fcom(x(g::<f32>(0x858CC4).read())).test_any(0x41) && random_int(0, 100) < 25 {
                    ik_look_at(ik(), TASK_SIT_IDLE, ped, other, ftol32(x(look_time)), 5, GPtr::NULL, 0, 0x3E80_0000, 750, 3, 0);
                    looked = true;
                }
            }
            if !looked {
                let pp = pos_of(ped);
                let mut p = [rd::<f32>(pp, 0), rd::<f32>(pp, 4), rd::<f32>(pp, 8)];
                let r = at_site(0x637BCF, rand);
                let s1 = f32_of(
                    xi(r).fmul_m(g::<f32>(0x858C7C).read()).fmul_m(g::<f32>(0x858B90).read()).fadd_m(g::<f32>(0x858624).read()),
                );
                let f = placeable_get_forward(ped, out);
                let fx = x(s1).fmul_m(rd::<f32>(f, 0));
                let fy = x(s1).fmul_m(rd::<f32>(f, 4));
                let fz = f32_of(x(s1).fmul_m(rd::<f32>(f, 8)));
                p[0] = f32_of(fx.fadd_m(p[0]));
                p[1] = f32_of(x(p[1]).fadd(fy));
                p[2] = f32_of(x(p[2]).fadd_m(fz));
                let r = at_site(0x637C38, rand);
                let s2 = f32_of(
                    xi(r).fmul_m(g::<f32>(0x858C7C).read()).fmul_m(g::<f32>(0x859000).read()).fsub_m(g::<f32>(0x858B90).read()),
                );
                let rv = placeable_get_right(ped, out);
                let rx = x(s2).fmul_m(rd::<f32>(rv, 0));
                let ry = x(s2).fmul_m(rd::<f32>(rv, 4));
                let rz = f32_of(x(s2).fmul_m(rd::<f32>(rv, 8)));
                p[0] = f32_of(rx.fadd_m(p[0]));
                p[1] = f32_of(ry.fadd_m(p[1]));
                p[2] = f32_of(x(rz).fadd_m(p[2]));
                let pg = GPtr::from_raw(p.as_mut_ptr()).cast::<u8>();
                ik_look_at(ik(), TASK_SIT_IDLE, ped, GPtr::NULL, ftol32(x(look_time)), -1, pg, 0, 0x3E80_0000, 750, 3, 0);
            }
        }
    }
    if rd::<u8>(this, 0x1C) == 0 {
        return 0;
    }
    if rd::<u8>(this, 0x1D) != 0 {
        wr::<u32>(this, 0x14, g::<u32>(TIME_MS).read());
        wr::<u8>(this, 0x1D, 0);
    }
    let end = rd::<u32>(this, 0x14).wrapping_add(rd::<u32>(this, 0x18));
    if g::<u32>(TIME_MS).read() < end {
        return 0;
    }
    if ik_is_looking(ik(), ped) != 0 {
        ik_abort_look_at(ik(), ped, 750);
    }
    1
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_SIT_COMPLEX_DTOR,
    HOOK_SIT_COMPLEX_MAKE_ABORTABLE,
    HOOK_SIT_CREATE_SUB_TASK,
    HOOK_SIT_COMPLEX_CLONE,
    HOOK_SIT_CREATE_NEXT_SUB_TASK,
    HOOK_SIT_CONTROL_SUB_TASK,
    HOOK_SIT_DOWN_START_ANIM,
    HOOK_STAND_UP_START_ANIM,
    HOOK_SIT_DOWN_PROCESS_PED,
    HOOK_STAND_UP_PROCESS_PED,
    HOOK_SIT_DOWN_CLONE,
    HOOK_STAND_UP_CLONE,
    HOOK_SIT_IDLE_CLONE,
    HOOK_SIT_IDLE_MAKE_ABORTABLE,
    HOOK_SIT_IDLE_PROCESS_PED,
];
