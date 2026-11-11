//! PORT-AI-4 (helper B, round 2): `CTaskComplexWalkRoundCar`, `CTaskComplexWalkRoundObject` and
//! `CTaskComplexWalkRoundBuildingAttempt`. Ported from the 1.0 US bytes (the EXE wins over gta-reversed and
//! the specs). Every call to another catalogued function goes through `orig!` at the original address;
//! task objects keep their original vtables and come from the original task pool (`common::task_new`).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_math::x87::Cmp;

use crate::common::*;

orig! {
    // ---- virtual slots ----
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplex::CreateFirstSubTask(CPed*)` (vtable +0x24)
    fn v_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;

    // ---- walk-round tasks ----
    /// `CTaskComplexWalkRoundObject::CreateRouteTask(CPed*)` (0x655140, `ret 4`)
    fn object_create_route_task_orig(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x655140 => unported;
    /// `CTaskComplexWalkRoundBuildingAttempt::CreateSubTask(eTaskType, CPed*)` (0x654C80, `ret 8`)
    fn building_create_sub_task(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x654C80 => unported;

    // ---- other tasks ----
    /// `CTaskComplexFollowPointRoute::Constructor(ms, route, mode, radius, slowDown, overshoot, blending,
    /// standStill)` (0x671510, `ret 0x20`)
    fn follow_point_route_ctor(t: GPtr<u8>, ms: u32, route: GPtr<u8>, mode: u32, radius: u32, slow: u32, a5: u32, a6: u32, a7: u32) -> GPtr<u8> = thiscall 0x671510 => unported;
    /// `CTaskComplexGoToPointAndStandStill::Constructor(ms, const CVector*, radius, slowDown, bool, bool)`
    /// (0x668120, `ret 0x18`)
    fn go_to_point_and_stand_still_ctor(t: GPtr<u8>, ms: u32, pt: GPtr<u8>, radius: u32, slow: u32, a4: u32, a5: u32) -> GPtr<u8> = thiscall 0x668120 => unported;
    /// `CTaskTimer::IsOutOfTime` (0x420E30)
    fn task_timer_is_out_of_time(t: GPtr<u8>) -> u8 = thiscall 0x420E30 => unported;

    // ---- other ----
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CCarEnterExit::IsPlayerToQuitCarEnter(CPed*, CVehicle*, int32 startTime, CTask*)` (0x64F240, cdecl)
    fn is_player_to_quit_car_enter(ped: GPtr<u8>, veh: GPtr<u8>, start: u32, task: GPtr<u8>) -> u8 = cdecl 0x64F240 => unported;
    /// `CPedGeometryAnalyser::ComputeClosestSurfacePoint-ped(CPed*, CEntity*, CVector*)` (0x5F3B70, cdecl)
    fn compute_closest_surface_point_ped(ped: GPtr<u8>, e: GPtr<u8>, out: GPtr<u8>) = cdecl 0x5F3B70 => unported;
    /// `CPedGeometryAnalyser::ComputeClosestSurfacePoint-pos(const CVector*, CEntity*, CVector*)` (0x5F36F0, cdecl)
    fn compute_closest_surface_point_pos(pos: GPtr<u8>, e: GPtr<u8>, out: GPtr<u8>) = cdecl 0x5F36F0 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityHitSide-3(const CVector*, CEntity*)` (0x5F3AC0, cdecl)
    fn compute_entity_hit_side(pos: GPtr<u8>, e: GPtr<u8>) -> i32 = cdecl 0x5F3AC0 => unported;
    /// 0x48E180 (`.HOODLUM` stub: `CPointRoute` add point, `ret 4`)
    fn point_route_add_48e180(r: GPtr<u8>, p: GPtr<u8>) = thiscall 0x48E180 => unported;
    /// `CTaskComplexWalkRoundBuildingAttempt::ComputeRoute(CPed*)` (0x657BA0, `ret 4`)
    fn building_compute_route(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x657BA0 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CVector::FromMultiply(const CMatrix&, const CVector&)` (0x59C670, `ret 8`)
    fn vector_from_multiply(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) = thiscall 0x59C670 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
}

/// A local buffer's address as a `GPtr` (the original's stack object).
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// Entity position: matrix +0x30 when the matrix pointer (+0x14) is set, else the placement at +4.
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// The shared body of the two `CreateRouteTask`s: null for an empty route (+0x20), else a
/// `CTaskComplexFollowPointRoute(ms, route, 0, [0x86FC98], 0, true, false, false)`.
fn create_route_task(this: GPtr<u8>, ms: u32) -> GPtr<u8> {
    if rd::<u32>(rd::<GPtr<u8>>(this, 0x20), 0) == 0 {
        return GPtr::NULL;
    }
    let t = task_new(0x3C);
    if t.is_null() {
        return t;
    }
    follow_point_route_ctor(t, ms, rd::<GPtr<u8>>(this, 0x20), 0, g::<u32>(0x86FC98).read(), 0, 1, 0, 0)
}

/// `CTaskComplexWalkRoundCar::CreateRouteTask(CPed*)` (0x6542E0, `ret 4`): a point-route task over the
/// route (+0x20) with the move state byte +0xC (sign-extended); null for an empty route.
#[hook(addr = 0x6542E0, conv = thiscall, name = "CTaskComplexWalkRoundCar::CreateRouteTask", expect = "6A FF 68 DB 35 84 00")]
pub fn car_create_route_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    create_route_task(this, i32::from(rd::<i8>(this, 0xC)) as u32)
}

/// `CTaskComplexWalkRoundObject::CreateRouteTask(CPed*)` (0x655140, `ret 4`): as the car version with the
/// move state dword +0xC.
#[hook(addr = 0x655140, conv = thiscall, name = "CTaskComplexWalkRoundObject::CreateRouteTask", expect = "6A FF 68 2B 37 84 00")]
pub fn object_create_route_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    create_route_task(this, rd::<u32>(this, 0xC))
}

/// `CTaskComplexWalkRoundCar::ControlSubTask(CPed*)` (0x654370, `ret 4`): a new route (flag 2 of +0xD)
/// restarts; a player quitting the car entry (`IsPlayerToQuitCarEnter`, start time +0x54, sub-task 0x389)
/// or a 0x389 sub-task whose timer (+0x24) ran out asks to finish; the sub-task is kept only while the car
/// (+0x1C) is within `[0x86F140]` (squared, 3D) of the stored position (+0x30) and both stored directions
/// (+0x3C forward vs matrix +0x10, +0x48 right vs matrix +0) still have a dot product >= `[0x86F144]`, and
/// nothing asked to finish; otherwise an abortable sub-task (`MakeAbortable(ped, 1, null)`) ends the task
/// (null), and a quitting player also aborts the primary 0x2BD task (intelligence +0x10). The car's matrix
/// is read without a null check when the car has none (kept).
#[hook(addr = 0x654370, conv = thiscall, name = "CTaskComplexWalkRoundCar::ControlSubTask", expect = "56 8B F1 8A 46 0D")]
pub fn car_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let f = rd::<u8>(this, 0xD);
    if f & 2 != 0 {
        wr::<u8>(this, 0xD, f & 0xFD);
        return v_create_first_sub_task(this, ped);
    }
    let sub = || rd::<GPtr<u8>>(this, 8);
    let mut finish = false;
    let mut quit = false;
    if ped_is_player(ped) != 0
        && !rd::<GPtr<u8>>(this, 0x1C).is_null()
        && rd::<i32>(this, 0x54) != -1
        && is_player_to_quit_car_enter(ped, rd::<GPtr<u8>>(this, 0x1C), rd::<u32>(this, 0x54), sub()) != 0
        && v_get_task_type(sub()) == 0x389
    {
        quit = true;
        finish = true;
    } else if v_get_task_type(sub()) == 0x389 && task_timer_is_out_of_time(this.byte_add(0x24)) != 0 {
        finish = true;
    }
    let veh = rd::<GPtr<u8>>(this, 0x1C);
    if !veh.is_null() {
        let m = rd::<GPtr<u8>>(veh, 0x14);
        let p = ent_pos(veh);
        let a = x(rd::<f32>(this, 0x30)).fsub_m(rd::<f32>(p, 0));
        let b = x(rd::<f32>(this, 0x34)).fsub_m(rd::<f32>(p, 4));
        let c = x(rd::<f32>(this, 0x38)).fsub_m(rd::<f32>(p, 8));
        let d2 = c.fmul(c).fadd(b.fmul(b)).fadd(a.fmul(a));
        if d2.fcom(x(g::<f32>(0x86F140).read())) != Cmp::Greater {
            let k = x(g::<f32>(0x86F144).read());
            let d1 = x(rd::<f32>(m, 0x18))
                .fmul_m(rd::<f32>(this, 0x44))
                .fadd(x(rd::<f32>(m, 0x14)).fmul_m(rd::<f32>(this, 0x40)))
                .fadd(x(rd::<f32>(m, 0x10)).fmul_m(rd::<f32>(this, 0x3C)));
            if d1.fcom(k) != Cmp::Less {
                let d2 = x(rd::<f32>(this, 0x50))
                    .fmul_m(rd::<f32>(m, 8))
                    .fadd(x(rd::<f32>(this, 0x4C)).fmul_m(rd::<f32>(m, 4)))
                    .fadd(x(rd::<f32>(m, 0)).fmul_m(rd::<f32>(this, 0x48)));
                if d2.fcom(k) != Cmp::Less && !finish {
                    return sub();
                }
            }
        }
    }
    if v_make_abortable(sub(), ped, 1, GPtr::NULL) == 0 {
        return sub();
    }
    if quit {
        let t = rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x47C), 0x10);
        if !t.is_null() && v_get_task_type(t) == 0x2BD {
            v_make_abortable(t, ped, 0, GPtr::NULL);
        }
    }
    GPtr::NULL
}

/// `CTaskComplexWalkRoundBuildingAttempt::ComputeMoveDir(CPed*, CVector* out)` (0x654B80, `ret 8`): `out`
/// = the stored direction (+0x2C) minus its projection on the 2D offset target (+0x14/+0x18) - ped (the
/// z term multiplies by `[0x858B50]`; `out.z` = 0); normalised, or the ped's matrix right vector (no null
/// check) when its squared 2D length is below `[0x86F360]`. The original keeps `out.x` in its own ped
/// argument slot for the length (a local here).
#[hook(addr = 0x654B80, conv = thiscall, name = "CTaskComplexWalkRoundBuildingAttempt::ComputeMoveDir", expect = "8B 54 24 04 8B 42 14")]
pub fn building_compute_move_dir(this: GPtr<u8>, ped: GPtr<u8>, out: GPtr<u8>) {
    let p = ent_pos(ped);
    let a = x(rd::<f32>(this, 0x14)).fsub_m(rd::<f32>(p, 0));
    let b = x(rd::<f32>(this, 0x18)).fsub_m(rd::<f32>(p, 4));
    for o in [0, 4, 8] {
        wr::<u32>(out, o, rd::<u32>(this, 0x2C + o));
    }
    let t0 = b.fmul_m(rd::<f32>(this, 0x30)).fadd(a.fmul_m(rd::<f32>(this, 0x2C)));
    let dz = rd::<f32>(this, 0x34);
    wr::<u32>(out, 8, 0);
    let t = t0.fadd(x(dz).fmul_m(g::<f32>(0x858B50).read()));
    let ex = f32_of(a.fsub(t.fmul_m(rd::<f32>(out, 0))));
    wr::<f32>(out, 0, ex);
    let ey = b.fsub(t.fmul_m(rd::<f32>(out, 4)));
    wr::<f32>(out, 4, f32_of(ey));
    let l2 = ey.fmul(ey).fadd(x(ex).fmul_m(ex));
    if l2.fcom(x(g::<f32>(0x86F360).read())) == Cmp::Less {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        for o in [0, 4, 8] {
            wr::<u32>(out, o, rd::<u32>(m, o));
        }
        return;
    }
    vector_normalise(out);
}

/// `CTaskComplexWalkRoundBuildingAttempt::CreateNextSubTask(CPed*)` (0x656E70, `ret 4`): a pending finish
/// (flag 0x10 of +0x49) → 0x516 and clears ped +0x478 bit 0x40; by sub-task type: 0x384/0x19B/0x1A5/0x1A6
/// → 0x386 with flag 2 (the ped passed), else 0x1A6; 0x386 → 0x389; 0xCB → 0x516 (clearing the ped bit);
/// 0x389 → 0x516; other types clear the ped bit and return null. `CreateSubTask` gets a null ped except
/// for 0x386.
#[hook(addr = 0x656E70, conv = thiscall, name = "CTaskComplexWalkRoundBuildingAttempt::CreateNextSubTask", expect = "56 8B F1 8A 46 49")]
pub fn building_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let clear = || wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) & 0xFFFF_FFBF);
    if rd::<u8>(this, 0x49) & 0x10 != 0 {
        let t = building_create_sub_task(this, 0x516, GPtr::NULL);
        clear();
        return t;
    }
    match v_get_task_type(rd::<GPtr<u8>>(this, 8)) {
        0x384 | 0x19B | 0x1A5 | 0x1A6 => {
            if rd::<u8>(this, 0x49) & 2 != 0 {
                building_create_sub_task(this, 0x386, ped)
            } else {
                building_create_sub_task(this, 0x1A6, GPtr::NULL)
            }
        }
        0x386 => building_create_sub_task(this, 0x389, GPtr::NULL),
        0xCB => {
            clear();
            building_create_sub_task(this, 0x516, GPtr::NULL)
        }
        0x389 => building_create_sub_task(this, 0x516, GPtr::NULL),
        _ => {
            clear();
            GPtr::NULL
        }
    }
}

/// `CTaskComplexWalkRoundObject::CreateNextSubTask(CPed*)` (0x657220, `ret 4`): null without an object
/// (+0x1C); 0x387 → the route task; 0x386 with a route → the route task; 0x386 without one computes the
/// closest surface point of the object to the ped into a local, and when it lies on the same side as the
/// target (+0x10, `ComputeEntityHitSide`) adds it and the target's closest surface point to the route
/// (0x48E180) and returns the route task; on another side goes to that point and stands still (move state
/// 4, radius `[0x86FC84]`, slow-down `[0x86FC88]`); other types (0xCB, 0x389, ...) null.
#[hook(addr = 0x657220, conv = thiscall, name = "CTaskComplexWalkRoundObject::CreateNextSubTask", expect = "64 A1 00 00 00 00")]
pub fn object_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<GPtr<u8>>(this, 0x1C).is_null() {
        return GPtr::NULL;
    }
    match v_get_task_type(rd::<GPtr<u8>>(this, 8)) {
        0x387 => object_create_route_task_orig(this, ped),
        0x386 => {
            if rd::<u32>(rd::<GPtr<u8>>(this, 0x20), 0) != 0 {
                return object_create_route_task_orig(this, ped);
            }
            let mut l = [0u32; 3];
            let lp = buf(&mut l);
            compute_closest_surface_point_ped(ped, rd::<GPtr<u8>>(this, 0x1C), lp);
            let s1 = compute_entity_hit_side(lp, rd::<GPtr<u8>>(this, 0x1C));
            let target = this.byte_add(0x10);
            let s2 = compute_entity_hit_side(target, rd::<GPtr<u8>>(this, 0x1C));
            if s1 == s2 {
                point_route_add_48e180(rd::<GPtr<u8>>(this, 0x20), lp);
                compute_closest_surface_point_pos(target, rd::<GPtr<u8>>(this, 0x1C), lp);
                point_route_add_48e180(rd::<GPtr<u8>>(this, 0x20), lp);
                return object_create_route_task_orig(this, ped);
            }
            let t = task_new(0x28);
            if t.is_null() {
                return t;
            }
            go_to_point_and_stand_still_ctor(t, 4, lp, g::<u32>(0x86FC84).read(), g::<u32>(0x86FC88).read(), 0, 0)
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexWalkRoundBuildingAttempt::ControlSubTask(CPed*)` (0x658020, `ret 4`): a finished task
/// (flag 0x10 of +0x49) keeps its sub-task; with flag 4, a vanished entity (+0x38) or one whose transformed
/// offset (+0x3C, through its matrix, allocated and updated when missing) moved more than `[0x86F150]`
/// (squared, 3D) from the stored point (+0x14) finishes with a stand-still (0xCB, null ped); without a
/// route (flag 2) computes it; with a route not yet started (flag 0x20) an abortable sub-task
/// (`MakeAbortable(ped, 1, null)`) starts it (0x389 when the route at +0xC has points, else finishes with
/// 0x19B; null ped); finally a 0x389 sub-task above the intelligence limit (`[0x86C938]`) that aborts is
/// replaced by 0x19B (ped passed) and the task finishes.
#[hook(addr = 0x658020, conv = thiscall, name = "CTaskComplexWalkRoundBuildingAttempt::ControlSubTask", expect = "83 EC 0C 56 8B F1")]
pub fn building_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let al = rd::<u8>(this, 0x49);
    if al & 0x10 != 0 {
        return rd::<GPtr<u8>>(this, 8);
    }
    if al & 4 != 0 {
        let e = rd::<GPtr<u8>>(this, 0x38);
        if e.is_null() {
            wr::<u8>(this, 0x49, al | 0x10);
            return building_create_sub_task(this, 0xCB, GPtr::NULL);
        }
        if rd::<GPtr<u8>>(e, 0x14).is_null() {
            placeable_allocate_matrix(e);
            simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        let mut l = [0u32; 3];
        let lp = buf(&mut l);
        vector_from_multiply(lp, rd::<GPtr<u8>>(e, 0x14), this.byte_add(0x3C));
        let a = x(rd::<f32>(lp, 0)).fsub_m(rd::<f32>(this, 0x14));
        let b = x(rd::<f32>(lp, 4)).fsub_m(rd::<f32>(this, 0x18));
        let c = x(rd::<f32>(lp, 8)).fsub_m(rd::<f32>(this, 0x1C));
        let d2 = c.fmul(c).fadd(b.fmul(b)).fadd(a.fmul(a));
        if d2.fcom(x(g::<f32>(0x86F150).read())) == Cmp::Greater {
            wr::<u8>(this, 0x49, rd::<u8>(this, 0x49) | 0x10);
            return building_create_sub_task(this, 0xCB, GPtr::NULL);
        }
    }
    let al = rd::<u8>(this, 0x49);
    if al & 2 == 0 {
        building_compute_route(this, ped);
    } else if al & 0x20 == 0 && v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, GPtr::NULL) != 0 {
        let al = rd::<u8>(this, 0x49) | 0x20;
        wr::<u8>(this, 0x49, al);
        if rd::<u32>(rd::<GPtr<u8>>(this, 0xC), 0) != 0 {
            return building_create_sub_task(this, 0x389, GPtr::NULL);
        }
        wr::<u8>(this, 0x49, al | 0x10);
        return building_create_sub_task(this, 0x19B, GPtr::NULL);
    }
    let mut ret = rd::<GPtr<u8>>(this, 8);
    if v_get_task_type(ret) == 0x389
        && rd::<i32>(rd::<GPtr<u8>>(ped, 0x47C), 0x274) > g::<i32>(0x86C938).read()
        && v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, GPtr::NULL) != 0
    {
        wr::<u8>(this, 0x49, rd::<u8>(this, 0x49) | 0x10);
        ret = building_create_sub_task(this, 0x19B, ped);
    }
    ret
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_CAR_CREATE_ROUTE_TASK,
    HOOK_OBJECT_CREATE_ROUTE_TASK,
    HOOK_CAR_CONTROL_SUB_TASK,
    HOOK_BUILDING_COMPUTE_MOVE_DIR,
    HOOK_BUILDING_CREATE_NEXT_SUB_TASK,
    HOOK_OBJECT_CREATE_NEXT_SUB_TASK,
    HOOK_BUILDING_CONTROL_SUB_TASK,
];
