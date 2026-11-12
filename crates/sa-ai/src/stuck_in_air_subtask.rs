//! `CTaskComplexStuckInAir::CreateSubTask` (0x67BA80) and the `CEventObjectCollision` destructor (0x4ACDD0), reached
//! when a ped is stuck in the air / bumps an object while walking (batch `std11-ai-1`). Ported from the 1.0 US bytes;
//! unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, thiscall, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CTaskSimpleFallAndGetUp(int32 anim, int32 group, int32 time)` (0x6786C0)
    fn fall_and_get_up_ctor(t: GPtr<u8>, anim: u32, group: u32, time: i32) -> GPtr<u8> = thiscall 0x6786C0 => unported;
    /// `CTaskSimpleStandStill(int32 time, bool looped, bool keep, float delta)` (0x62F310)
    fn stand_still_ctor(t: GPtr<u8>, time: u32, looped: u32, keep: u32, delta: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskComplex::CTaskComplex()` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CTaskComplexGoToPointAndStandStill(...)`-style constructor (0x65B390): (target, mode, radius, time)
    fn go_to_point_ctor(t: GPtr<u8>, target: GPtr<u8>, mode: u32, radius: u32, time: u32) -> GPtr<u8> = thiscall 0x65B390 => unported;
}

/// `CEventObjectCollision::~CEventObjectCollision` (0x4ACDD0, thiscall; entry `push -1` then the protection chunk
/// 0x406A96, which builds the SEH frame and returns to 0x4ACDD7): vtable 0x85ACA8; with the object (+0x14) set,
/// `object->CleanUpOldReference(&this->object)`; then the `CEvent` vtable 0x85AB38.
#[hook(addr = 0x4ACDD0, conv = thiscall, name = "FUN_004acdd0", expect = "6A FF E9 BF 9C F5 FF",
       unverified = "std11-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_object_collision_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85ACA8);
    let o = rd::<GPtr<u8>>(this, 0x14);
    if !o.is_null() {
        entity_clean_up_old_reference(o, this.byte_add(0x14));
    }
    wr::<u32>(this, 0, 0x85AB38);
}

/// `CTaskComplexStuckInAir::CreateSubTask(int32 type, CPed* ped)` (0x67BA80, thiscall, `ret 8`; `this` is not read).
/// 0xD0: the intelligence's +0x270 / +0x272 words cleared, `new CTaskSimpleFallAndGetUp(0x1A, 0, 1000)` (0x18 bytes).
/// 0xCB: standing flag set, `new CTaskSimpleStandStill(5000, false, false, 8.0)` (0x20). 0xD3: standing flag set, the
/// words cleared, a 0x14-byte `CTaskComplex` with vtable 0x870570, byte +0x10 = 0 and +0xC = -1. 0x38C: standing flag
/// set; T = position - forward * k (k = [0x858B8C]; x, y as f32, z with the unrounded product), then ± right * k by
/// `rand() & 1` (+ when odd), `new` (0x3C) task 0x65B390 (&T, 0, 5.0, 10000). Other types, or a failed allocation: 0.
///
/// Stack: the original keeps forward.y * k in its own `ped` argument slot (not reproduced: the slot is dead after it).
#[hook(addr = 0x67BA80, conv = thiscall, name = "CTaskComplexStuckInAir::CreateSubTask", expect = "64 A1 00 00 00 00 8B 4C 24 04",
       unverified = "std11-ai-1", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk")]
pub fn task_complex_stuck_in_air_create_sub_task(_this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    let clear_words = || {
        let intel = rd::<GPtr<u8>>(ped, 0x47C);
        wr::<u16>(intel, 0x270, 0);
        wr::<u16>(intel, 0x272, 0);
    };
    let standing = || wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 1);
    match ty {
        0xD0 => {
            clear_words();
            let t = task_new(0x18);
            if t.is_null() { t } else { fall_and_get_up_ctor(t, 0x1A, 0, 1000) }
        }
        0xCB => {
            standing();
            let t = task_new(0x20);
            if t.is_null() { t } else { stand_still_ctor(t, 5000, 0, 0, 0x4100_0000) }
        }
        0xD3 => {
            standing();
            clear_words();
            let t = task_new(0x14);
            if t.is_null() {
                return t;
            }
            task_complex_ctor(t);
            wr::<u8>(t, 0x10, 0);
            wr::<u32>(t, 0, 0x870570);
            wr::<i32>(t, 0xC, -1);
            t
        }
        0x38C => {
            standing();
            let m = rd::<GPtr<u8>>(ped, 0x14);
            let k = g::<f32>(0x858B8C).read();
            let az = x(rd::<f32>(m, 0x18)).fmul_m(k);
            let fy = f32_of(x(rd::<f32>(m, 0x14)).fmul_m(k));
            let fx = f32_of(x(rd::<f32>(m, 0x10)).fmul_m(k));
            let p = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
            let tz = az.fsubr_m(rd::<f32>(p, 8));
            let ty_ = x(rd::<f32>(p, 4)).fsub_m(fy);
            let mut t = [f32_of(x(rd::<f32>(p, 0)).fsub_m(fx)), f32_of(ty_), f32_of(tz)];
            let odd = at_site(0x67BC62, rand) & 1 != 0;
            let r = rd::<GPtr<u8>>(ped, 0x14);
            let (rz, ry, rx) = (x(rd::<f32>(r, 8)).fmul_m(k), x(rd::<f32>(r, 4)).fmul_m(k), x(rd::<f32>(r, 0)).fmul_m(k));
            for (i, v) in [(0, rx), (1, ry), (2, rz)] {
                t[i] = f32_of(if odd { v.fadd_m(t[i]) } else { x(t[i]).fsub(v) });
            }
            let task = task_new(0x3C);
            if task.is_null() {
                return task;
            }
            go_to_point_ctor(task, GPtr::from_raw(t.as_mut_ptr()).cast(), 0, 0x40A0_0000, 10000)
        }
        _ => GPtr::NULL,
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_EVENT_OBJECT_COLLISION_DTOR, HOOK_TASK_COMPLEX_STUCK_IN_AIR_CREATE_SUB_TASK];
