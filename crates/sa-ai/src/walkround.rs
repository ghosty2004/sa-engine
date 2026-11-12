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

orig! {
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCorners(float z, CEntity&, CVector* corners)` (0x5F3650)
    fn compute_entity_bounding_box_corners(z: u32, e: GPtr<u8>, corners: GPtr<u8>) = cdecl 0x5F3650 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxPlanes(float z, CEntity&, CVector* normals, float* ds)` (0x5F3660)
    fn compute_entity_bounding_box_planes(z: u32, e: GPtr<u8>, normals: GPtr<u8>, ds: GPtr<u8>) = cdecl 0x5F3660 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityHitSide(const CPed&, CEntity&)` (0x5F3BC0)
    fn compute_entity_hit_side_ped(ped: GPtr<u8>, e: GPtr<u8>) -> i32 = cdecl 0x5F3BC0 => unported;
    /// `CPedGeometryAnalyser::ComputeRouteRoundEntityBoundingBox(const CPed&, CEntity&, const CVector&, CPointRoute&,
    /// int32)` (0x5F6110)
    fn compute_route_round_entity_bounding_box(ped: GPtr<u8>, e: GPtr<u8>, target: GPtr<u8>, route: u32, dir: i32) -> u32 = cdecl 0x5F6110 => unported;
}

/// `CTaskComplexWalkRoundCar::ComputeRouteRoundSmallCar(CPed*)` (0x6544F0, thiscall, `ret 4`; float in ST0).
/// x87. Vehicle V = +0x1C, target T = +0x10. Box corners and planes of V at the ped's z (cached versions); the
/// side the ped hits (`ComputeEntityHitSide`, -1 none) gives the result distance f32(n.z*p.z + n.y*p.y + n.x*p.x
/// + d) of the ped position p, else 0. With flag +0xD bit 0: the plane distances of T (plane 0 summed x, z, y;
/// planes 1..3 y, x, z; each stored f32) up to the first not below 0; T inside all four → the side of T
/// (`ComputeEntityHitSide(T, V)`), s = 0.05 - that plane's distance (register), T.x = f32(T.x + s*n.x), T.y,
/// T.z plus the f32 products. Then `ComputeRouteRoundEntityBoundingBox(ped, V, T, route +0x20, the signed low
/// nibble of +0xE)` and its low 4 bits go to bits 2..5 of +0xD. Returns the distance.
#[hook(addr = 0x6544f0, conv = thiscall, name = "CTaskComplexWalkRoundCar::ComputeRouteRoundSmallCar",
       expect = "81 EC 84 00 00 00 53", unverified = "mvpj-ai-5", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn car_compute_route_round_small_car(this: GPtr<u8>, ped: GPtr<u8>) -> X87 {
    let veh = rd::<GPtr<u8>>(this, 0x1C);
    let pz = rd::<u32>(ent_pos(ped), 8);
    let mut corners = [0u32; 12];
    compute_entity_bounding_box_corners(pz, veh, buf(&mut corners));
    let pz = rd::<u32>(ent_pos(ped), 8);
    let mut normals = [0u32; 12];
    let mut ds = [0u32; 4];
    compute_entity_bounding_box_planes(pz, rd::<GPtr<u8>>(this, 0x1C), buf(&mut normals), buf(&mut ds));
    let side = compute_entity_hit_side_ped(ped, rd::<GPtr<u8>>(this, 0x1C));
    let nf = |i: usize| f32::from_bits(normals[i]);
    let dsf = |i: usize| f32::from_bits(ds[i]);
    let mut dist = 0f32;
    if side != -1 {
        let p = ent_pos(ped);
        let s = side as usize;
        dist = x(nf(s * 3 + 2))
            .fmul_m(rd::<f32>(p, 8))
            .fadd(x(nf(s * 3 + 1)).fmul_m(rd::<f32>(p, 4)))
            .fadd(x(nf(s * 3)).fmul_m(rd::<f32>(p, 0)))
            .fadd_m(dsf(s))
            .to_f32();
    }
    if rd::<u8>(this, 0xD) & 1 != 0 {
        let t = this.byte_add(0x10);
        let (tx, ty, tz) = (rd::<f32>(t, 0), rd::<f32>(t, 4), rd::<f32>(t, 8));
        let mut dd = [0f32; 4];
        let mut k = 0;
        while k < 4 {
            let n = |i: usize| nf(k * 3 + i);
            let v = if k == 0 {
                x(n(0)).fmul_m(tx).fadd(x(n(2)).fmul_m(tz)).fadd(x(ty).fmul_m(n(1)))
            } else {
                x(n(1)).fmul_m(ty).fadd(x(n(0)).fmul_m(tx)).fadd(x(n(2)).fmul_m(tz))
            }
            .fadd_m(dsf(k));
            dd[k] = v.to_f32();
            if !v.fcom(X87::zero()).c0() {
                break;
            }
            k += 1;
        }
        if k == 4 {
            let side2 = compute_entity_hit_side(t, rd::<GPtr<u8>>(this, 0x1C));
            if side2 != -1 {
                let s2 = side2 as usize;
                let s = x(dd[s2]).fchs().fadd_m(0.05);
                let sx = s.fmul_m(nf(s2 * 3));
                let sy = s.fmul_m(nf(s2 * 3 + 1)).to_f32();
                let sz = s.fmul_m(nf(s2 * 3 + 2)).to_f32();
                wr::<f32>(t, 0, sx.fadd_m(rd::<f32>(t, 0)).to_f32());
                wr::<f32>(t, 4, x(sy).fadd_m(rd::<f32>(t, 4)).to_f32());
                wr::<f32>(t, 8, x(sz).fadd_m(rd::<f32>(t, 8)).to_f32());
            }
        }
    }
    let nib = i32::from((rd::<u8>(this, 0xE) << 4) as i8) >> 4;
    let r = compute_route_round_entity_bounding_box(ped, rd::<GPtr<u8>>(this, 0x1C), this.byte_add(0x10), rd::<u32>(this, 0x20), nib);
    let b = rd::<u8>(this, 0xD);
    wr::<u8>(this, 0xD, b ^ (((r as u8) << 2 ^ b) & 0x3C));
    x(dist)
}

orig! {
    /// `CTaskComplexWalkRoundObject::CreateSubTask(int32 type, CPed*)` (`FUN_00655290`, `ret 8`)
    fn object_create_sub_task(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x655290 => unported;
    /// `CVector::DotProduct(const CVector*, const CVector*)` (0x40FDB0, ST0)
    fn vector_dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `CTaskSimpleScratchHead::CTaskSimpleScratchHead()` (thunk 0x48DF30)
    fn scratch_head_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x48DF30 => unported;
    /// `CTaskSimpleHitWall::CTaskSimpleHitWall()` (0x652F90)
    fn hit_wall_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x652F90 => unported;
}

/// `CTaskComplexWalkRoundObject::ControlSubTask(CPed*)` (0x6575F0, thiscall, `ret 4`). x87. Abort (→ when the
/// sub-task's `MakeAbortable(ped, 1, NULL)` succeeds, `CreateSubTask(0xCB, ped)`, else the sub-task) when: no
/// object (+0x1C); or the sub-task is of type 0x389 with the timer set (+0x2C: a pending start +0x2D takes
/// the current time into +0x24) and now ≥ +0x24 + +0x28 (unsigned); or the ped intelligence's +0x274 is above
/// 30 ([0x86C938]). Else a sub-task of type 0xCB is kept; otherwise (matrix allocated from the simple transform
/// when missing) the object is kept while |+0x30 - object position|² ≤ 0.0625, the matrix forward (+0x10)·
/// +0x3C (z, y, x terms) is not below 0.9 and `DotProduct(+0x48, matrix right)` is not below 0.9; else abort.
#[hook(addr = 0x6575f0, conv = thiscall, name = "CTaskComplexWalkRoundObject::ControlSubTask", expect = "53 8B 5C 24 08 56 8B F1",
       unverified = "mvpj-ai-6", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn object_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = || rd::<GPtr<u8>>(this, 8);
    let abort = |ped: GPtr<u8>| {
        if v_make_abortable(sub(), ped, 1, GPtr::NULL) != 0 {
            object_create_sub_task(this, 0xCB, ped)
        } else {
            sub()
        }
    };
    if rd::<u32>(this, 0x1C) == 0 {
        return abort(ped);
    }
    if v_get_task_type(sub()) == 0x389 && rd::<u8>(this, 0x2C) != 0 {
        if rd::<u8>(this, 0x2D) != 0 {
            wr::<u32>(this, 0x24, g::<u32>(0xB7CB84).read());
            wr::<u8>(this, 0x2D, 0);
        }
        let end = rd::<u32>(this, 0x28).wrapping_add(rd::<u32>(this, 0x24));
        if g::<u32>(0xB7CB84).read() >= end {
            return abort(ped);
        }
    }
    if rd::<i32>(rd::<GPtr<u8>>(ped, 0x47C), 0x274) > g::<i32>(0x86C938).read() {
        return abort(ped);
    }
    if v_get_task_type(sub()) == 0xCB {
        return sub();
    }
    let obj = rd::<GPtr<u8>>(this, 0x1C);
    if rd::<u32>(obj, 0x14) == 0 {
        placeable_allocate_matrix(obj);
        simple_transform_update_matrix(obj.byte_add(4), rd::<GPtr<u8>>(obj, 0x14));
    }
    let o = rd::<GPtr<u8>>(this, 0x1C);
    let m = rd::<GPtr<u8>>(obj, 0x14);
    let p = ent_pos(o);
    let dx = x(rd::<f32>(this, 0x30)).fsub_m(rd::<f32>(p, 0));
    let dy = x(rd::<f32>(this, 0x34)).fsub_m(rd::<f32>(p, 4));
    let dz = x(rd::<f32>(this, 0x38)).fsub_m(rd::<f32>(p, 8));
    let dsq = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    if !dsq.fcom(x(g::<f32>(0x86F15C).read())).test_any(0x41) {
        return abort(ped);
    }
    let k = x(g::<f32>(0x86F160).read());
    let fwd = x(rd::<f32>(m, 0x18))
        .fmul_m(rd::<f32>(this, 0x44))
        .fadd(x(rd::<f32>(m, 0x14)).fmul_m(rd::<f32>(this, 0x40)))
        .fadd(x(rd::<f32>(this, 0x3C)).fmul_m(rd::<f32>(m, 0x10)));
    if !fwd.fcom(k).test_parity(0x05) {
        return abort(ped);
    }
    let r = X87::from_f64_exact(vector_dot_product(this.byte_add(0x48), m));
    if r.fcom(k).test_parity(0x05) {
        return sub();
    }
    abort(ped)
}

/// `CTaskComplexWalkRoundBuildingAttempt::CreateFirstSubTask(CPed*)` (0x654A00, thiscall, `ret 4`). With flag
/// +0x49 bit 2: no entity (+0x38) → ped +0x478 bit 6 cleared, NULL; else (matrix allocated when missing) +0x14 =
/// entity matrix × +0x3C (`CVector::FromMultiply`). Ped +0x478 bit 6 set; by the i8 at +0x48: 1..4 → a new
/// `CTaskSimpleScratchHead`, 6/7 → a new `CTaskSimpleHitWall` (NULL when the pool is full, the flag stays set);
/// others → the flag cleared again, NULL.
#[hook(addr = 0x654a00, conv = thiscall, name = "CTaskComplexWalkRoundBuildingAttempt::CreateFirstSubTask",
       expect = "64 A1 00 00 00 00 6A FF", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn building_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u8>(this, 0x49) & 4 != 0 {
        let e = rd::<GPtr<u8>>(this, 0x38);
        if e.is_null() {
            wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) & 0xFFFF_FFBF);
            return GPtr::NULL;
        }
        if rd::<u32>(e, 0x14) == 0 {
            placeable_allocate_matrix(e);
            simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        vector_from_multiply(this.byte_add(0x14), rd::<GPtr<u8>>(e, 0x14), this.byte_add(0x3C));
    }
    let f = rd::<u32>(ped, 0x478) | 0x40;
    wr::<u32>(ped, 0x478, f);
    match rd::<i8>(this, 0x48) {
        1..=4 => {
            let t = task_new(0x20);
            if t.is_null() { GPtr::NULL } else { scratch_head_ctor(t) }
        }
        6 | 7 => {
            let t = task_new(0x20);
            if t.is_null() { GPtr::NULL } else { hit_wall_ctor(t) }
        }
        _ => {
            wr::<u32>(ped, 0x478, f & 0xFFFF_FFBF);
            GPtr::NULL
        }
    }
}

orig! {
    /// `CTaskComplexWalkRoundCar::ComputeRouteRoundBigCar(CPed*)` (0x656BB0, `ret 4`, ST0)
    fn car_compute_route_round_big_car(t: GPtr<u8>, ped: GPtr<u8>) -> f64 = thiscall 0x656BB0 => unported;
    /// `CTaskComplexWalkRoundCar::ComputeRouteRoundSmallCar(CPed*)` (0x6544F0, `ret 4`, ST0)
    fn car_compute_route_round_small_car_st0(t: GPtr<u8>, ped: GPtr<u8>) -> f64 = thiscall 0x6544F0 => unported;
    /// `CVector::NormaliseAndMag()` (0x59C970, ST0)
    fn vector_normalise_and_mag(v: GPtr<u8>) -> f64 = thiscall 0x59C970 => unported;
    /// `CTaskManager::FindTaskByType(int32 slot, int32 type)` (0x6817D0, `ret 8`)
    fn tm_find_task_by_type(tm: GPtr<u8>, slot: i32, ty: i32) -> GPtr<u8> = thiscall 0x6817D0 => unported;
    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(int32, bool, bool, float)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, time: i32, looped: u32, b: u32, delta: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskSimpleStandStill::ProcessPed(CPed*)` (0x62F370, `ret 4`)
    fn stand_still_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x62F370 => unported;
    /// `CTaskSimpleStandStill` complete destructor (`FUN_0062F360`)
    fn stand_still_dtor(t: GPtr<u8>) = thiscall 0x62F360 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float, float, float, float)` (0x53CBE0, ST0)
    fn general_get_radian_angle_between_points(x1: f32, y1: f32, x2: f32, y2: f32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn general_limit_radian_angle(a: f32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CTaskSimpleAchieveHeading::CTaskSimpleAchieveHeading(float heading, float changeRate, float tolerance)`
    /// (0x667E20, `ret 0xC`)
    fn achieve_heading_ctor(t: GPtr<u8>, heading: u32, rate: u32, tol: u32) -> GPtr<u8> = thiscall 0x667E20 => unported;
}

/// `CTaskComplexWalkRoundCar::CreateFirstSubTask(CPed*)` (0x658200, thiscall, `ret 4`). x87. Route (+0x20) count =
/// 0; dist = f32 `ComputeRouteRoundBigCar` (vehicle +0x429 bit 2) or `ComputeRouteRoundSmallCar`; an empty route →
/// NULL. Timer: +0x24 = now, +0x28 = 20000 (move state +0xC == 4) or 15000, scaled `_ftol(t * [0x858B90])` when the
/// vehicle has a driver (+0x4C8) or passenger (+0x4C4) slot set; +0x2C = 1; +0x30 = vehicle position, +0x3C its
/// matrix forward, +0x48 its matrix right. u = f32(route[0] - ped) (z, y, x registers), m = f32 `NormaliseAndMag`;
/// dot = f32(u·ped forward) (z, y, x). Threshold 2 (move state 4), 4 (6) or 6. A player with a type-0x2BD task in
/// slot 3 copies its +0x4C to +0x54. Not a big car, m, dist above the threshold and dot above [0x86F424] →
/// `CreateRouteTask(ped)`. Else a stack `CTaskSimpleStandStill(0, false, false, 8.0)` `ProcessPed`s the ped and a
/// new `CTaskSimpleAchieveHeading(f32 LimitRadianAngle(GetRadianAngleBetweenPoints(f32(route.x - ped.x),
/// f32(route.y - ped.y), 0, 0)), 1.0, 0.1)` (NULL when the pool is full) is returned.
///
/// The original keeps the timer, the threshold and the heading in its own `ped` argument slot (popped by `ret 4`).
#[hook(addr = 0x658200, conv = thiscall, name = "CTaskComplexWalkRoundCar::CreateFirstSubTask", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "mvpj-ai-6", route = "L2", hazards = "stack-residue", recs = "grove-walk")]
pub fn car_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let route = rd::<GPtr<u8>>(this, 0x20);
    wr::<u32>(route, 0, 0);
    let big = rd::<u8>(rd::<GPtr<u8>>(this, 0x1C), 0x429) & 4 != 0;
    let dist = X87::from_f64_exact(if big {
        car_compute_route_round_big_car(this, ped)
    } else {
        car_compute_route_round_small_car_st0(this, ped)
    })
    .to_f32();
    if rd::<u32>(rd::<GPtr<u8>>(this, 0x20), 0) == 0 {
        return GPtr::NULL;
    }
    let t: i32 = if rd::<u8>(this, 0xC) == 4 { 0x4E20 } else { 0x3A98 };
    let veh = rd::<GPtr<u8>>(this, 0x1C);
    let dur = if rd::<u32>(veh, 0x4C8) != 0 || rd::<u32>(veh, 0x4C4) != 0 {
        X87::from_i32(t).fmul_m(g::<f32>(0x858B90).read()).ftol() as u32 as i32
    } else {
        t
    };
    wr::<u32>(this, 0x24, g::<u32>(0xB7CB84).read());
    wr::<i32>(this, 0x28, dur);
    wr::<u8>(this, 0x2C, 1);
    let vp = ent_pos(veh);
    for i in 0..3isize {
        wr::<u32>(this, 0x30 + i * 4, rd::<u32>(vp, i * 4));
    }
    let m = rd::<GPtr<u8>>(veh, 0x14);
    for i in 0..3isize {
        wr::<u32>(this, 0x3C + i * 4, rd::<u32>(m, 0x10 + i * 4));
    }
    for i in 0..3isize {
        wr::<u32>(this, 0x48 + i * 4, rd::<u32>(m, i * 4));
    }
    let pp = ent_pos(ped);
    let r = rd::<GPtr<u8>>(this, 0x20).byte_add(4);
    let dz = x(rd::<f32>(r, 8)).fsub_m(rd::<f32>(pp, 8));
    let dy = x(rd::<f32>(r, 4)).fsub_m(rd::<f32>(pp, 4));
    let dx = x(rd::<f32>(r, 0)).fsub_m(rd::<f32>(pp, 0));
    let mut u = [dx.to_f32(), dy.to_f32(), dz.to_f32()];
    let mag = X87::from_f64_exact(vector_normalise_and_mag(GPtr::from_raw(u.as_mut_ptr()).cast())).to_f32();
    let pm = rd::<GPtr<u8>>(ped, 0x14);
    let dot = x(u[2])
        .fmul_m(rd::<f32>(pm, 0x18))
        .fadd(x(u[1]).fmul_m(rd::<f32>(pm, 0x14)))
        .fadd(x(u[0]).fmul_m(rd::<f32>(pm, 0x10)))
        .to_f32();
    let thr: f32 = match rd::<u8>(this, 0xC) {
        4 => 2.0,
        6 => 4.0,
        _ => 6.0,
    };
    if ped_is_player(ped) != 0 {
        let tm = rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
        let tk = tm_find_task_by_type(tm, 3, 0x2BD);
        if !tk.is_null() {
            wr::<u32>(this, 0x54, rd::<u32>(tk, 0x4C));
        }
    }
    let gt = |a: f32, b: f32| !x(a).fcom(x(b)).test_any(0x41);
    if rd::<u8>(rd::<GPtr<u8>>(this, 0x1C), 0x429) & 4 == 0
        && gt(mag, thr)
        && gt(dist, thr)
        && gt(dot, g::<f32>(0x86F424).read())
    {
        return car_create_route_task(this, ped);
    }
    let mut ss = [0u32; 8];
    let sp = buf(&mut ss);
    stand_still_ctor(sp, 0, 0, 0, 8f32.to_bits());
    stand_still_process_ped(sp, ped);
    let pp = ent_pos(ped);
    let r = rd::<GPtr<u8>>(this, 0x20).byte_add(4);
    let hx = x(rd::<f32>(r, 0)).fsub_m(rd::<f32>(pp, 0)).to_f32();
    let hy = x(rd::<f32>(r, 4)).fsub_m(rd::<f32>(pp, 4)).to_f32();
    let a = X87::from_f64_exact(general_get_radian_angle_between_points(hx, hy, 0.0, 0.0)).to_f32();
    let h = X87::from_f64_exact(general_limit_radian_angle(a)).to_f32();
    let tk = task_new(0x18);
    let res = if tk.is_null() {
        GPtr::NULL
    } else {
        achieve_heading_ctor(tk, h.to_bits(), 0x3F80_0000, 0x3DCC_CCCD)
    };
    stand_still_dtor(sp);
    res
}

orig! {
    /// `CTaskComplexWalkRoundObject::ComputeRoute(CPed*)` (0x6551D0, `ret 4`, ST0; not in the catalogue's names)
    fn object_compute_route(t: GPtr<u8>, ped: GPtr<u8>) -> f64 = thiscall 0x6551D0 => unported;
}

/// `CTaskComplexWalkRoundObject::CreateFirstSubTask(CPed*)` (0x657380, thiscall, `ret 4`). x87. No object (+0x1C)
/// → NULL. Route (+0x20) count = 0; dist = f32 of `FUN_006551D0` (the route computation); an empty route → NULL.
/// Timer +0x24 = now, +0x28 = 8000 (move state dword +0xC == 4) or 4000, +0x2C = 1; +0x30 = object position, +0x3C
/// its matrix forward, +0x48 its matrix right (matrix allocated from the simple transform when missing, twice). u =
/// f32(route[0] - ped) (z, y, x registers), m = f32 `NormaliseAndMag`, dot = u·ped forward (z, y, x; register).
/// Threshold 2 / 4 / 6 (move state 4 / 6 / other). m, dist above it and dot above [0x86F420] → `CreateRouteTask`;
/// else a new `CTaskSimpleAchieveHeading(f32 LimitRadianAngle(GetRadianAngleBetweenPoints(f32(route.x - ped.x),
/// f32(route.y - ped.y), 0, 0)), 1.0, 0.2)` (NULL when the pool is full).
///
/// The original keeps m and the heading in its own `ped` argument slot (popped by `ret 4`).
#[hook(addr = 0x657380, conv = thiscall, name = "CTaskComplexWalkRoundObject::CreateFirstSubTask", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "mvpj-ai-6", route = "L2", hazards = "stack-residue", recs = "grove-walk")]
pub fn object_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(this, 0x1C) == 0 {
        return GPtr::NULL;
    }
    wr::<u32>(rd::<GPtr<u8>>(this, 0x20), 0, 0);
    let dist = X87::from_f64_exact(object_compute_route(this, ped)).to_f32();
    if rd::<u32>(rd::<GPtr<u8>>(this, 0x20), 0) == 0 {
        return GPtr::NULL;
    }
    let t: u32 = if rd::<u32>(this, 0xC) == 4 { 0x1F40 } else { 0xFA0 };
    wr::<u32>(this, 0x24, g::<u32>(0xB7CB84).read());
    wr::<u32>(this, 0x28, t);
    wr::<u8>(this, 0x2C, 1);
    let ensure = |o: GPtr<u8>| {
        if rd::<u32>(o, 0x14) == 0 {
            placeable_allocate_matrix(o);
            simple_transform_update_matrix(o.byte_add(4), rd::<GPtr<u8>>(o, 0x14));
        }
    };
    let o = rd::<GPtr<u8>>(this, 0x1C);
    let op = ent_pos(o);
    for i in 0..3isize {
        wr::<u32>(this, 0x30 + i * 4, rd::<u32>(op, i * 4));
    }
    ensure(o);
    let m = rd::<GPtr<u8>>(o, 0x14);
    for i in 0..3isize {
        wr::<u32>(this, 0x3C + i * 4, rd::<u32>(m, 0x10 + i * 4));
    }
    let o = rd::<GPtr<u8>>(this, 0x1C);
    ensure(o);
    let m = rd::<GPtr<u8>>(o, 0x14);
    for i in 0..3isize {
        wr::<u32>(this, 0x48 + i * 4, rd::<u32>(m, i * 4));
    }
    let pp = ent_pos(ped);
    let r = rd::<GPtr<u8>>(this, 0x20).byte_add(4);
    let dz = x(rd::<f32>(r, 8)).fsub_m(rd::<f32>(pp, 8));
    let dy = x(rd::<f32>(r, 4)).fsub_m(rd::<f32>(pp, 4));
    let dx = x(rd::<f32>(r, 0)).fsub_m(rd::<f32>(pp, 0));
    let mut u = [dx.to_f32(), dy.to_f32(), dz.to_f32()];
    let mag = X87::from_f64_exact(vector_normalise_and_mag(GPtr::from_raw(u.as_mut_ptr()).cast())).to_f32();
    let pm = rd::<GPtr<u8>>(ped, 0x14);
    let dot = x(u[2])
        .fmul_m(rd::<f32>(pm, 0x18))
        .fadd(x(u[1]).fmul_m(rd::<f32>(pm, 0x14)))
        .fadd(x(u[0]).fmul_m(rd::<f32>(pm, 0x10)));
    let thr = x(match rd::<u32>(this, 0xC) {
        4 => g::<f32>(0x858CA0).read(),
        6 => g::<f32>(0x858B90).read(),
        _ => g::<f32>(0x858B44).read(),
    });
    if !x(mag).fcom(thr).test_any(0x41)
        && !x(dist).fcom(thr).test_any(0x41)
        && !dot.fcom(x(g::<f32>(0x86F420).read())).test_any(0x41)
    {
        return object_create_route_task_orig(this, ped);
    }
    let pp = ent_pos(ped);
    let r = rd::<GPtr<u8>>(this, 0x20).byte_add(4);
    let hx = x(rd::<f32>(r, 0)).fsub_m(rd::<f32>(pp, 0)).to_f32();
    let hy = x(rd::<f32>(r, 4)).fsub_m(rd::<f32>(pp, 4)).to_f32();
    let a = X87::from_f64_exact(general_get_radian_angle_between_points(hx, hy, 0.0, 0.0)).to_f32();
    let h = X87::from_f64_exact(general_limit_radian_angle(a)).to_f32();
    let tk = task_new(0x18);
    if tk.is_null() {
        return GPtr::NULL;
    }
    achieve_heading_ctor(tk, h.to_bits(), 0x3F80_0000, 0x3E4C_CCCD)
}

orig! {
    /// `CVector::CrossProduct(CVector* out, const CVector* a, const CVector* b)` (0x59C730): `out = a × b`
    fn vector_cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `CWorld::ProcessLineOfSight(start, end, CColPoint&, CEntity*&, buildings, vehicles, peds, objects, dummies,
    /// seeThrough, camera, shootThrough)` (0x56BA00)
    fn world_process_line_of_sight(s: GPtr<u8>, e: GPtr<u8>, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, st: u32, c: u32, sh: u32) -> u8 = cdecl 0x56BA00 => unported;
}

/// An entity's position (matrix +0x30, else the placement +4).
fn entity_pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// 1/32767 (0x858C7C): the `rand()` scale
const K_RAND_SCALE: usize = 0x858C7C;
/// 1.0 (0x858624)
const K_ONE_F: usize = 0x858624;
/// 0.5 (0x858B8C): the line start below the ped
const K_HALF_F: usize = 0x858B8C;
/// 0.35 (0x858F9C): the minimum free distance and the step back from the hit
const K_CRAP_ROUTE_MARGIN: usize = 0x858F9C;

/// `CTaskComplexWalkRoundBuildingAttempt::ComputeCrapRoute(const CPed&)` (0x656F80, thiscall, `ret 4`). x87; two direct
/// `rand()` calls (return addresses 0x656F8E, 0x65701C).
///
/// `f = rand() * (1/32767) + 1`; `P = (f32(f * N.x), f32(f * N.y), f32(f * N.z))` with N = this +0x2C;
/// `C = CrossProduct(N, (0, 0, 1))`; `h = 2 * rand() * (1/32767) - 1`; `Q = (f32(P.x + C.x*h), f32(P.y + f32(C.y*h)),
/// P.z + f32(C.z*h))` (the z as a register value). `start = (ped x, ped y, f32(ped z - 0.5))`, `end = (f32(start.x +
/// Q.x), f32(Q.y + ped y), f32(Q.z + (ped z - 0.5)))`; `ProcessLineOfSight(start, end, cp, ent, buildings only)`.
/// No hit → `end` is the point. A hit: `d = f32(hit - start)` per component, `NormaliseAndMag(d)` below 0.35 (or
/// NaN) → return without the flag; else the point is `(f32(hit.x - d.x*0.35), f32(hit.y - d.y*0.35), f32(hit.z -
/// f32(d.z*0.35)))`. The point route (+0x10: count +0, 12-byte points +4) takes it when below 8 points; then flag
/// +0x49 |= 8.
#[hook(addr = 0x656f80, conv = thiscall, name = "CTaskComplexWalkRoundBuildingAttempt::ComputeCrapRoute",
       expect = "81 EC 88 00 00 00", unverified = "mvpk-ai-2", route = "L2", hazards = "none", recs = "grove-walk",
       note = "rand() through at_site (0x656F8E, 0x65701C)")]
pub fn building_attempt_compute_crap_route(this: GPtr<u8>, ped: GPtr<u8>) {
    let k_rand = g::<f32>(K_RAND_SCALE).read();
    let k_one = g::<f32>(K_ONE_F).read();
    let r1 = sa_core::orig::at_site(0x656F8E, rand);
    let f = X87::from_i32(r1).fmul_m(k_rand).fadd_m(k_one);
    let n = this.byte_add(0x2C);
    let mut up = [0f32, 0.0, 1.0];
    let px = f.fmul_m(rd::<f32>(n, 0)).to_f32();
    let py = f.fmul_m(rd::<f32>(n, 4)).to_f32();
    let pz = f.fmul_m(rd::<f32>(n, 8)).to_f32();
    let mut cross = [0f32; 3];
    let out = vector_cross_product(GPtr::from_raw(cross.as_mut_ptr()).cast(), n, GPtr::from_raw(up.as_mut_ptr()).cast());
    let c = [rd::<f32>(out, 0), rd::<f32>(out, 4), rd::<f32>(out, 8)];
    let r2 = sa_core::orig::at_site(0x65701C, rand);
    let h = X87::from_i32(r2).fmul_m(k_rand).fdouble().fsub_m(k_one);
    let cxh = x(c[0]).fmul(h);
    let cyh = x(c[1]).fmul(h).to_f32();
    let czh = x(c[2]).fmul(h).to_f32();
    let qx = x(px).fadd(cxh).to_f32();
    let qy = x(py).fadd_m(cyh).to_f32();
    let qz = x(pz).fadd_m(czh);
    let pos = entity_pos_ptr(ped);
    let pz5 = x(rd::<f32>(pos, 8)).fsub_m(g::<f32>(K_HALF_F).read());
    let sy = rd::<f32>(pos, 4);
    let mut start = [rd::<f32>(pos, 0), sy, pz5.to_f32()];
    let mut end = [x(start[0]).fadd_m(qx).to_f32(), x(qy).fadd(x(sy)).to_f32(), qz.fadd(pz5).to_f32()];
    let mut cp = [0u32; 11];
    let mut ent: u32 = 0;
    let hit = world_process_line_of_sight(
        GPtr::from_raw(start.as_mut_ptr()).cast(),
        GPtr::from_raw(end.as_mut_ptr()).cast(),
        GPtr::from_raw(cp.as_mut_ptr()).cast(),
        GPtr::from_raw(&raw mut ent).cast(),
        1, 0, 0, 0, 0, 0, 0, 0,
    );
    let point = if hit == 0 {
        end
    } else {
        let hp = [f32::from_bits(cp[0]), f32::from_bits(cp[1]), f32::from_bits(cp[2])];
        let mut d = [
            x(hp[0]).fsub_m(start[0]).to_f32(),
            x(hp[1]).fsub_m(start[1]).to_f32(),
            x(hp[2]).fsub_m(start[2]).to_f32(),
        ];
        let k = g::<f32>(K_CRAP_ROUTE_MARGIN).read();
        let mag = vector_normalise_and_mag(GPtr::from_raw(d.as_mut_ptr()).cast());
        if X87::from_f64_exact(mag).fcom(x(k)).c0() {
            return;
        }
        let a = x(d[0]).fmul_m(k);
        let b = x(d[1]).fmul_m(k);
        let cz = x(d[2]).fmul_m(k).to_f32();
        [x(hp[0]).fsub(a).to_f32(), x(hp[1]).fsub(b).to_f32(), x(hp[2]).fsub_m(cz).to_f32()]
    };
    let route = rd::<GPtr<u8>>(this, 0x10);
    let count = rd::<i32>(route, 0);
    if count < 8 {
        let slot = route.byte_add(4 + count as isize * 12);
        for (i, v) in point.iter().enumerate() {
            wr::<f32>(slot, 4 * i as isize, *v);
        }
        wr::<i32>(route, 0, count + 1);
    }
    wr::<u8>(this, 0x49, rd::<u8>(this, 0x49) | 8);
}

orig! {
    /// `CTaskComplexWalkRoundBuildingAttempt::ComputeMoveDir(CPed*, CVector* out)` (0x654B80, `ret 8`)
    fn building_compute_move_dir_orig(t: GPtr<u8>, ped: GPtr<u8>, out: GPtr<u8>) = thiscall 0x654B80 => unported;
    /// `CWorld::GetIsLineOfSightClear(start, end, buildings, vehicles, peds, objects, dummies, seeThrough,
    /// ignoreSomeObjects)` (0x56A490)
    fn world_get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bld: u32, veh: u32, ped: u32, obj: u32, dum: u32, see: u32, ign: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CTaskComplexWalkRoundBuildingAttempt::ComputeCrapRoute(const CPed&)` (0x656F80, `ret 4`)
    fn building_compute_crap_route_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x656F80 => unported;
    /// `CPointRoute::operator=(const CPointRoute&)` (0x63A040, `ret 4`)
    fn point_route_assign(r: GPtr<u8>, src: GPtr<u8>) = thiscall 0x63A040 => unported;
}

/// 0.4 (0x858EE8) and 0.2 (0x858CC4): the random spread of the step around the building
const K_ROUTE_SPREAD: usize = 0x858EE8;
const K_ROUTE_SPREAD_HALF: usize = 0x858CC4;
/// 0.7 (0x858CB0): the probe length along the normalised offset
const K_ROUTE_PROBE: usize = 0x858CB0;

/// Appends `p` (three dwords) to a point route (count +0, 12-byte points +4) when it holds fewer than 8.
fn route_push(route: GPtr<u8>, p: &[f32; 3]) {
    let n = rd::<i32>(route, 0);
    if n < 8 {
        let slot = route.byte_add(4 + n as isize * 12);
        for (i, v) in p.iter().enumerate() {
            wr::<f32>(slot, 4 * i as isize, *v);
        }
        wr::<i32>(route, 0, n + 1);
    }
}

/// `CTaskComplexWalkRoundBuildingAttempt::ComputeRoute(const CPed&)` (0x657BA0, thiscall, `ret 4`). x87; one direct
/// `rand()` per attempt (return address 0x657C47).
///
/// Route A (+0xC) is emptied; the attempt counter (+0x4A, i8) + 1 reaching 20 → unless flag +0x49 bit 3,
/// `ComputeCrapRoute(ped)`; route A = route B (+0x10); flag bit 1; done. Else, `N2 = f32(2 * normal +0x2C)`; for i = 1
/// and, with flag bit 0, -1 (i decreases by 2, else 3, while not below -1):
/// `ComputeMoveDir(ped, &d)`; `f = rand() * (1/32767) * 0.4 - 0.2 + attempts`; `a = f32(i * (d.x * f) + normal.x *
/// 0.35)`, `b = f32(f32(d.y * f) * i + f32(normal.y * 0.35))`, `c = f32(f32(normal.z * 0.35) + f32(f32(d.z * f) * i))`;
/// `o = Normalise((a, b, c)) * 0.7` (f32 per component); `P = (f32(a + +0x20), f32(b + +0x24), f32(c + +0x28 - 0.5))`,
/// `Q = P - o`, the ped's `V = (x, y, f32(z - 0.5))`. Buildings-only lines of sight: V→P blocked → blocked count + 1;
/// P→target (+0x14) clear → route A += P, flag bit 1, done. Else `R = P - N2`, `W = Q - N2`; P→R and Q→W clear →
/// (flag bit 3 clear: route B = (P, R), flag bit 3) and R→target clear → route A += P, R and flag bit 1 (the loop
/// goes on). After the loop: first attempt and every probe blocked at V→P → attempts = 20.
#[hook(addr = 0x657ba0, conv = thiscall, name = "CTaskComplexWalkRoundBuildingAttempt::ComputeRoute",
       expect = "81 EC 88 00 00 00", unverified = "mvpk-ai-3", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "rand() through at_site (0x657C47); writes the task's point routes (task pool)")]
pub fn building_attempt_compute_route(this: GPtr<u8>, ped: GPtr<u8>) {
    let expected_blocked: i32 = if rd::<u8>(this, 0x49) & 1 != 0 { 2 } else { 1 };
    wr::<u32>(rd::<GPtr<u8>>(this, 0xC), 0, 0);
    let mut blocked = 0;
    let attempts = rd::<i8>(this, 0x4A).wrapping_add(1);
    wr::<i8>(this, 0x4A, attempts);
    if attempts >= 0x14 {
        if rd::<u8>(this, 0x49) & 8 == 0 {
            building_compute_crap_route_orig(this, ped);
        }
        point_route_assign(rd::<GPtr<u8>>(this, 0xC), rd::<GPtr<u8>>(this, 0x10));
        wr::<u8>(this, 0x49, rd::<u8>(this, 0x49) | 2);
        return;
    }
    let n2 = [
        x(rd::<f32>(this, 0x2C)).fdouble().to_f32(),
        x(rd::<f32>(this, 0x30)).fdouble().to_f32(),
        x(rd::<f32>(this, 0x34)).fdouble().to_f32(),
    ];
    let k_rand = g::<f32>(K_RAND_SCALE).read();
    let k_margin = g::<f32>(K_CRAP_ROUTE_MARGIN).read();
    let k_probe = g::<f32>(K_ROUTE_PROBE).read();
    let half = g::<f32>(K_HALF_F).read();
    let target = this.byte_add(0x14);
    let mut i: i32 = 1;
    loop {
        let mut d = [0f32; 3];
        building_compute_move_dir_orig(this, ped, GPtr::from_raw(d.as_mut_ptr()).cast());
        let r = sa_core::orig::at_site(0x657C47, rand);
        let f = X87::from_i32(r)
            .fmul_m(k_rand)
            .fmul_m(g::<f32>(K_ROUTE_SPREAD).read())
            .fsub_m(g::<f32>(K_ROUTE_SPREAD_HALF).read())
            .fadd(X87::from_i32(i32::from(rd::<i8>(this, 0x4A))));
        let dxf = x(d[0]).fmul(f);
        let dyf = x(d[1]).fmul(f).to_f32();
        let dzf = x(d[2]).fmul(f).to_f32();
        let fi = i as f32;
        let ax = X87::from_i32(i).fmul(dxf);
        let by = x(dyf).fmul_m(fi);
        let cz = x(dzf).fmul_m(fi).to_f32();
        let ny = x(rd::<f32>(this, 0x30)).fmul_m(k_margin).to_f32();
        let nz = x(rd::<f32>(this, 0x34)).fmul_m(k_margin).to_f32();
        let a = x(rd::<f32>(this, 0x2C)).fmul_m(k_margin).fadd(ax).to_f32();
        let b = x(ny).fadd(by).to_f32();
        let c = x(nz).fadd_m(cz).to_f32();
        let mut o = [a, b, c];
        vector_normalise(GPtr::from_raw(o.as_mut_ptr()).cast());
        for v in o.iter_mut() {
            *v = x(*v).fmul_m(k_probe).to_f32();
        }
        i -= if rd::<u8>(this, 0x49) & 1 != 0 { 2 } else { 3 };
        let mut p = [
            x(a).fadd_m(rd::<f32>(this, 0x20)).to_f32(),
            x(b).fadd_m(rd::<f32>(this, 0x24)).to_f32(),
            x(c).fadd_m(rd::<f32>(this, 0x28)).fsub_m(half).to_f32(),
        ];
        let mut q = [x(p[0]).fsub_m(o[0]).to_f32(), x(p[1]).fsub_m(o[1]).to_f32(), x(p[2]).fsub_m(o[2]).to_f32()];
        let pp = entity_pos_ptr(ped);
        let mut v = [rd::<f32>(pp, 0), rd::<f32>(pp, 4), x(rd::<f32>(pp, 8)).fsub_m(half).to_f32()];
        let pv = |a: &mut [f32; 3]| -> GPtr<u8> { GPtr::from_raw(a.as_mut_ptr()).cast() };
        if world_get_is_line_of_sight_clear(pv(&mut v), pv(&mut p), 1, 0, 0, 0, 0, 0, 0) == 0 {
            blocked += 1;
        } else if world_get_is_line_of_sight_clear(pv(&mut p), target, 1, 0, 0, 0, 0, 0, 0) != 0 {
            route_push(rd::<GPtr<u8>>(this, 0xC), &p);
            wr::<u8>(this, 0x49, rd::<u8>(this, 0x49) | 2);
            return;
        } else {
            let mut rr = [x(p[0]).fsub_m(n2[0]).to_f32(), x(p[1]).fsub_m(n2[1]).to_f32(), x(p[2]).fsub_m(n2[2]).to_f32()];
            let mut w = [x(q[0]).fsub_m(n2[0]).to_f32(), x(q[1]).fsub_m(n2[1]).to_f32(), x(q[2]).fsub_m(n2[2]).to_f32()];
            if world_get_is_line_of_sight_clear(pv(&mut p), pv(&mut rr), 1, 0, 0, 0, 0, 0, 0) != 0
                && world_get_is_line_of_sight_clear(pv(&mut q), pv(&mut w), 1, 0, 0, 0, 0, 0, 0) != 0
            {
                if rd::<u8>(this, 0x49) & 8 == 0 {
                    let rb = rd::<GPtr<u8>>(this, 0x10);
                    wr::<u32>(rb, 0, 0);
                    route_push(rd::<GPtr<u8>>(this, 0x10), &p);
                    route_push(rd::<GPtr<u8>>(this, 0x10), &rr);
                    wr::<u8>(this, 0x49, rd::<u8>(this, 0x49) | 8);
                }
                if world_get_is_line_of_sight_clear(pv(&mut rr), target, 1, 0, 0, 0, 0, 0, 0) != 0 {
                    route_push(rd::<GPtr<u8>>(this, 0xC), &p);
                    route_push(rd::<GPtr<u8>>(this, 0xC), &rr);
                    wr::<u8>(this, 0x49, rd::<u8>(this, 0x49) | 2);
                }
            }
        }
        if i < -1 {
            break;
        }
    }
    if rd::<i8>(this, 0x4A) == 1 && blocked == expected_blocked {
        wr::<i8>(this, 0x4A, 0x14);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_BUILDING_ATTEMPT_COMPUTE_ROUTE,
    HOOK_BUILDING_ATTEMPT_COMPUTE_CRAP_ROUTE,
    HOOK_OBJECT_CONTROL_SUB_TASK,
    HOOK_BUILDING_CREATE_FIRST_SUB_TASK,
    HOOK_CAR_CREATE_FIRST_SUB_TASK,
    HOOK_OBJECT_CREATE_FIRST_SUB_TASK,
    HOOK_CAR_COMPUTE_ROUTE_ROUND_SMALL_CAR,
    HOOK_CAR_CREATE_ROUTE_TASK,
    HOOK_OBJECT_CREATE_ROUTE_TASK,
    HOOK_CAR_CONTROL_SUB_TASK,
    HOOK_BUILDING_COMPUTE_MOVE_DIR,
    HOOK_BUILDING_CREATE_NEXT_SUB_TASK,
    HOOK_OBJECT_CREATE_NEXT_SUB_TASK,
    HOOK_BUILDING_CONTROL_SUB_TASK,
];
