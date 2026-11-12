//! MVP-A AI ports, batch `mvpa-ai-2`: functions whose entries pass through no-CD protection fragments
//! (`CPedGeometryAnalyser::ComputeEntityHitSide` (normals), `CTaskComplexWalkRoundBuildingAttempt::CreateSubTask`, the
//! `CTaskComplexWander` destructor), the anim task constructors and `StartAnim`s, the car-drive bopping helpers, the
//! event-handler history queries, `CTaskTimer::IsOutOfTime` and the entity seek-position calculators. Ported from
//! the 1.0 US bytes (protection fragments inlined; the exe wins over gta-reversed). Every call to another catalogued
//! function goes through `orig!` at the original address.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskComplex::~CTaskComplex` (0x61A3D0)
    fn task_complex_dtor(t: GPtr<u8>) = thiscall 0x61A3D0 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new_o(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(int32, bool, bool, float)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, time: u32, looped: u32, idle: u32, delta: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskSimpleHitWall::CTaskSimpleHitWall` (0x652F90)
    fn hit_wall_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x652F90 => unported;
    /// task constructor for type 421 (0x48DF30, a `.HOODLUM` stub; no catalogue name)
    fn ctor_48df30(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x48DF30 => unported;
    /// `CTaskSimpleLookAbout::CTaskSimpleLookAbout(int32)` (0x48E0A0, `ret 4`)
    fn look_about_ctor(t: GPtr<u8>, time: u32) -> GPtr<u8> = thiscall 0x48E0A0 => unported;
    /// `CTaskSimpleGoToPoint::CTaskSimpleGoToPoint(int32 moveState, const CVector&, float radius, bool, bool)`
    /// (0x667CD0, `ret 0x14`)
    fn go_to_point_ctor(t: GPtr<u8>, ms: i32, target: GPtr<u8>, radius: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x667CD0 => unported;
    /// `CTaskComplexFollowPointRoute::CTaskComplexFollowPointRoute(int32 moveState, const CPointRoute&, int32 mode,
    /// float targetRadius, float slowDownDistance, bool, bool, bool)` (0x671510, `ret 0x20`)
    #[allow(clippy::too_many_arguments)]
    fn follow_point_route_ctor(t: GPtr<u8>, ms: i32, route: u32, mode: u32, r: u32, s: u32, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x671510 => unported;
    /// `CTaskSimpleAchieveHeading::CTaskSimpleAchieveHeading(float heading, float changeRate, float tolerance)`
    /// (0x667E20, `ret 0xC`)
    fn achieve_heading_ctor(t: GPtr<u8>, h: f32, rate: u32, tol: u32) -> GPtr<u8> = thiscall 0x667E20 => unported;
    /// `CVector::Normalise` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CBE0, ST0)
    fn radian_angle_between_points(x1: f32, y1: f32, x2: f32, y2: f32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn limit_radian_angle(a: f32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CEvent::GetEventType` (vtable +4)
    fn v_get_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CPedGeometryAnalyser::ComputeClearTarget(const CPed&, const CVector&, CVector&)` (0x5F5D80, cdecl)
    fn compute_clear_target(ped: GPtr<u8>, target: GPtr<u8>, out: GPtr<u8>) -> u32 = cdecl 0x5F5D80 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740, `ret 4`)
    fn task_manager_find_active_task_by_type(m: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CTaskSimpleCarDrive::UpdateBopping` (0x63C900)
    fn update_bopping_o(t: GPtr<u8>) = thiscall 0x63C900 => crate::mvpa_ai2::car_drive_update_bopping;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): out = a - b
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CVector::Magnitude` (0x406DA0, ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CTaskSimpleCarDrive::ProcessHeadBopping(CPed*, bool, float)` (0x6428C0, `ret 0xC`)
    fn process_head_bopping(t: GPtr<u8>, ped: GPtr<u8>, b: u32, dist: u32) = thiscall 0x6428C0 => unported;
    /// `CTaskSimpleCarDrive::ProcessArmBopping(CPed*, bool, float)` (0x642AE0, `ret 0xC`)
    fn process_arm_bopping(t: GPtr<u8>, ped: GPtr<u8>, b: u32, dist: u32) = thiscall 0x642AE0 => unported;
    /// `CTaskSimple::CTaskSimple` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610, cdecl)
    fn blend_animation(clump: u32, group: u32, anim: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(void (*)(CAnimBlendAssociation*, void*), void*)` (0x4CEBE0, `ret 8`)
    fn assoc_set_finish_callback(a: GPtr<u8>, cb: u32, data: GPtr<u8>) = thiscall 0x4CEBE0 => unported;
    /// `CAnimBlendAssociation::SetDeleteCallback(void (*)(CAnimBlendAssociation*, void*), void*)` (0x4CEBC0, `ret 8`)
    fn assoc_set_delete_callback(a: GPtr<u8>, cb: u32, data: GPtr<u8>) = thiscall 0x4CEBC0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCornersUncached(float z, CEntity&, CVector*)` (0x5F1FA0, cdecl)
    fn bbox_corners_uncached(z: u32, e: GPtr<u8>, corners: GPtr<u8>) = cdecl 0x5F1FA0 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxPlanesUncached(float z, const CVector*, CVector*, float*)`
    /// (0x5F1670, cdecl)
    fn bbox_planes_uncached(z: u32, corners: GPtr<u8>, normals: GPtr<u8>, d: GPtr<u8>) = cdecl 0x5F1670 => crate::mvpa_ai1::compute_entity_bounding_box_planes_uncached;
    /// `CTaskUtilityLineUpPedWithCar::~CTaskUtilityLineUpPedWithCar` (0x64FC00)
    fn line_up_dtor(u: GPtr<u8>) = thiscall 0x64FC00 => unported;
    /// CRT `free` (0x8214BD, cdecl)
    fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `CVehicle::ClearGettingOutFlags(uint8)` (0x6D3060, `ret 4`)
    fn vehicle_clear_getting_out_flags(v: GPtr<u8>, door: u32) = thiscall 0x6D3060 => unported;
    /// `CTaskSimple::~CTaskSimple` (0x61A3A0)
    fn task_simple_dtor(t: GPtr<u8>) = thiscall 0x61A3A0 => unported;
    /// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990, cdecl)
    fn anim_block_index_by_name(name: GPtr<u8>) -> u32 = cdecl 0x4D3990 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// CRT `operator new` (0x82119A, cdecl)
    fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `CTaskUtilityLineUpPedWithCar::CTaskUtilityLineUpPedWithCar(const CVector&, int32, int32, int32)` (0x64FBB0,
    /// `ret 0x10`)
    fn line_up_ctor(u: GPtr<u8>, offset: GPtr<u8>, time: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x64FBB0 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: u32, e: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0, cdecl)
    fn clump_get_association(clump: u32, anim: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CTaskSimpleRunTimedAnim::StartAnim(CPed*)` (0x61AC50, `ret 4`)
    fn start_timed_anim(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x61AC50 => crate::mvpa_ai2::run_timed_anim_start_anim;
}

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `CTaskSimpleAnim::FinishRunAnimCB` (0x61A8A0)
const FINISH_RUN_ANIM_CB: u32 = 0x61A8A0;

fn cf(addr: usize) -> X87 {
    x(g::<f32>(addr).read())
}

fn local<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// The entity's position: `m_matrix->pos` (+0x14 -> +0x30) when it has a matrix, else the placement (+4).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// The f32 an `fstp dword` stores of an ST0 result.
fn st0_f32(v: f64) -> f32 {
    X87::from_f64_exact(v).to_f32()
}

/// `CPedGeometryAnalyser::ComputeEntityHitSide(const CVector& pos, const CVector* normals, const float* distances)`
/// (0x5F1450, cdecl; the counter start 4 is set in the protection fragment 0x4014D8): for k = 4..7, a = (k-1)&3,
/// c = k&3: returns c (EAX) when `((n[a].y*p.y + n[a].z*p.z) + p.x*n[a].x) + d[a]` is not below [0x858B50] (C0
/// clear) and the same sum for c is below it (`test ah, 5` / `jnp`). Else 0.
#[hook(addr = 0x5F1450, conv = cdecl, name = "CPedGeometryAnalyser::ComputeEntityHitSide-2", expect = "8B 54 24 04 53 55",
       unverified = "mvpa-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn compute_entity_hit_side_2(p: GPtr<u8>, normals: GPtr<u8>, d: GPtr<u8>) -> u32 {
    let side = |i: u32| -> X87 {
        let n = normals.byte_add(0xC * i as isize);
        x(rd::<f32>(n, 4))
            .fmul_m(rd::<f32>(p, 4))
            .fadd(x(rd::<f32>(n, 8)).fmul_m(rd::<f32>(p, 8)))
            .fadd(x(rd::<f32>(p, 0)).fmul_m(rd::<f32>(n, 0)))
            .fadd_m(rd::<f32>(d, 4 * i as isize))
    };
    let zero = cf(0x858B50);
    for k in 4u32..8 {
        let a = (k - 1) & 3;
        let c = k & 3;
        if side(a).fcom(zero).c0() {
            continue;
        }
        if !side(c).fcom(zero).test_parity(5) {
            return c;
        }
    }
    0
}

/// `CTaskComplexWander` destructor (0x669D20, entered through the protection fragment 0x40387C that stores the
/// vtable): vtable 0x86FE84, then `CTaskComplex::~CTaskComplex` (0x61A3D0, tail jump).
#[hook(addr = 0x669D20, conv = thiscall, name = "thunk_FUN_0040387c", expect = "90 E9 56 9B D9 FF",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn task_complex_wander_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86FE84);
    task_complex_dtor(this);
}

/// `CTaskComplexWalkRoundBuildingAttempt::CreateSubTask(int32 taskType, CPed*)` (0x654C80, `ret 8`; prologue in the
/// protection fragment 0x403788): a new task from the task pool (null when the pool is full):
/// 203 `CTaskSimpleStandStill(1000, false, false, 8.0)`; 411 `CTaskSimpleHitWall`; 421 the 0x48DF30 task; 422
/// `CTaskSimpleLookAbout(1000)`; 900 `CTaskSimpleGoToPoint(moveState (i8 +0x48), &target (+0x14), [0x86FC78], 0,
/// 0)`; 902 `CTaskSimpleAchieveHeading(LimitRadianAngle(GetRadianAngleBetweenPoints(d.x, d.y, 0, 0)), 1.0, 0.1)`
/// with d = normalised (first route point (+0xC -> +4) - ped position); 905 `CTaskComplexFollowPointRoute(moveState,
/// route (+0xC), 0, [0x86FC98], [0x86FC98], false, false, true)`. Every other type: null. The two angles are stored
/// as f32 (into the original's argument slots).
#[hook(addr = 0x654C80, conv = thiscall, name = "CTaskComplexWalkRoundBuildingAttempt::CreateSubTask", expect = "90 E9 02 EB DA FF",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn walk_round_building_attempt_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    let ms = rd::<i8>(this, 0x48) as i32;
    match ty {
        203 => {
            let t = task_new_o(0x20);
            if t.is_null() {
                return GPtr::NULL;
            }
            stand_still_ctor(t, 1000, 0, 0, 0x4100_0000)
        }
        411 => {
            let t = task_new_o(0x20);
            if t.is_null() {
                return GPtr::NULL;
            }
            hit_wall_ctor(t)
        }
        421 => {
            let t = task_new_o(0x20);
            if t.is_null() {
                return GPtr::NULL;
            }
            ctor_48df30(t)
        }
        422 => {
            let t = task_new_o(0x34);
            if t.is_null() {
                return GPtr::NULL;
            }
            look_about_ctor(t, 1000)
        }
        900 => {
            let t = task_new_o(0x30);
            if t.is_null() {
                return GPtr::NULL;
            }
            go_to_point_ctor(t, ms, this.byte_add(0x14), g::<u32>(0x86FC78).read(), 0, 0)
        }
        902 => {
            let p = pos(ped);
            let r = rd::<GPtr<u8>>(this, 0xC).byte_add(4);
            let mut v = [0u32; 3];
            let vp = local(&mut v);
            wr::<f32>(vp, 0, f32_of(x(rd::<f32>(r, 0)).fsub_m(rd::<f32>(p, 0))));
            wr::<f32>(vp, 4, f32_of(x(rd::<f32>(r, 4)).fsub_m(rd::<f32>(p, 4))));
            wr::<f32>(vp, 8, f32_of(x(rd::<f32>(r, 8)).fsub_m(rd::<f32>(p, 8))));
            vector_normalise(vp);
            let a = st0_f32(radian_angle_between_points(rd::<f32>(vp, 0), rd::<f32>(vp, 4), 0.0, 0.0));
            let h = st0_f32(limit_radian_angle(a));
            let t = task_new_o(0x18);
            if t.is_null() {
                return GPtr::NULL;
            }
            achieve_heading_ctor(t, h, 0x3F80_0000, 0x3DCC_CCCD)
        }
        905 => {
            let t = task_new_o(0x3C);
            if t.is_null() {
                return GPtr::NULL;
            }
            let r = g::<u32>(0x86FC98).read();
            follow_point_route_ctor(t, ms, rd::<u32>(this, 0xC), 0, r, r, 0, 0, 1)
        }
        _ => GPtr::NULL,
    }
}

/// `CEventHandler::GetCurrentEventType` (0x4B8CC0): the type (vtable +4, tail call) of the current event (+0xC),
/// else of the stored event (+8); -1 when neither is set.
#[hook(addr = 0x4B8CC0, conv = thiscall, name = "CEventHandler::GetCurrentEventType", expect = "8B 51 0C 83 C8 FF",
       unverified = "mvpa-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn event_handler_get_current_event_type(this: GPtr<u8>) -> i32 {
    let mut e = rd::<GPtr<u8>>(this, 0xC);
    if e.is_null() {
        e = rd::<GPtr<u8>>(this, 8);
        if e.is_null() {
            return -1;
        }
    }
    v_get_event_type(e)
}

/// `CEventHandlerHistory::IsRespondingToEvent(int32 type)` (0x4B8B90, `ret 4`, AL): for -1, whether the current
/// event (+4) or the stored event (+0xC) is set; else whether either one's type (vtable +4, current first, both
/// queried) equals `type`. The upper EAX bits are left from the pointer / type the original last loaded.
#[hook(addr = 0x4B8B90, conv = thiscall, name = "CEventHandlerHistory::IsRespondingToEvent", expect = "56 57 8B 7C 24 0C",
       unverified = "mvpa-ai-2", route = "L2", hazards = "ax-al-return", recs = "grove-walk")]
pub fn event_handler_history_is_responding_to_event(this: GPtr<u8>, ty: i32) -> u8 {
    if ty == -1 {
        return (rd::<u32>(this, 4) != 0 || rd::<u32>(this, 0xC) != 0) as u8;
    }
    let mut r = 0u8;
    let cur = rd::<GPtr<u8>>(this, 4);
    if !cur.is_null() && v_get_event_type(cur) == ty {
        r = 1;
    }
    let st = rd::<GPtr<u8>>(this, 0xC);
    if !st.is_null() && v_get_event_type(st) == ty {
        return 1;
    }
    r
}

/// `CTaskTimer::IsOutOfTime` (0x420E30, a `.HOODLUM` stub; body 0x156D5D0, the time load obfuscated): false (AL 0,
/// upper EAX bits untouched) when not started (+8); when the reset flag (+9) is set, start (+0) = now and the flag is
/// cleared; then 1 when `now >= start + interval (+4)` (unsigned), else 0 (full EAX).
#[hook(addr = 0x420E30, conv = thiscall, name = "CTaskTimer::IsOutOfTime", expect = "E9 9B C7 14 01",
       route = "L2", hazards = "heap-writes,ax-al-return", recs = "grove-walk")]
pub fn task_timer_is_out_of_time(this: GPtr<u8>) -> u32 {
    if rd::<u8>(this, 8) == 0 {
        return 0;
    }
    if rd::<u8>(this, 9) != 0 {
        wr::<u32>(this, 0, g::<u32>(TIME_MS).read());
        wr::<u8>(this, 9, 0);
    }
    let end = rd::<u32>(this, 4).wrapping_add(rd::<u32>(this, 0));
    (g::<u32>(TIME_MS).read() >= end) as u32
}

/// `CEntitySeekPosCalculatorStandard::ComputeEntitySeekPos(const CPed&, const CEntity&, CVector& out)` (0x46AF20,
/// `ret 0xC`): out = the entity's position (matrix, else placement), dword copies.
#[hook(addr = 0x46AF20, conv = thiscall, name = "CEntitySeekPosCalculatorStandard::ComputeEntitySeekPos", expect = "8B 4C 24 08 8B 41 14",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn seek_pos_standard(_this: GPtr<u8>, _ped: GPtr<u8>, e: GPtr<u8>, out: GPtr<u8>) {
    let p = pos(e);
    wr::<u32>(out, 0, rd::<u32>(p, 0));
    wr::<u32>(out, 4, rd::<u32>(p, 4));
    wr::<u32>(out, 8, rd::<u32>(p, 8));
}

/// `CEntitySeekPosCalculatorXYOffset::ComputeEntitySeekPos(const CPed&, const CEntity&, CVector& out)` (0x6946A0,
/// `ret 0xC`): target = (e.x + off.x, off.y + e.y, off.z + e.z) with the offset at +4 (f32 each), then
/// `ComputeClearTarget(ped, target, out)` (EAX passed on).
#[hook(addr = 0x6946A0, conv = thiscall, name = "CEntitySeekPosCalculatorXYOffset::ComputeEntitySeekPos", expect = "8B 54 24 08 8B 42 14",
       unverified = "mvpa-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn seek_pos_xy_offset(this: GPtr<u8>, ped: GPtr<u8>, e: GPtr<u8>, out: GPtr<u8>) -> u32 {
    let p = pos(e);
    let mut t = [0u32; 3];
    let tp = local(&mut t);
    wr::<f32>(tp, 0, f32_of(x(rd::<f32>(p, 0)).fadd_m(rd::<f32>(this, 4))));
    wr::<f32>(tp, 4, f32_of(x(rd::<f32>(this, 8)).fadd_m(rd::<f32>(p, 4))));
    wr::<f32>(tp, 8, f32_of(x(rd::<f32>(this, 0xC)).fadd_m(rd::<f32>(p, 8))));
    compute_clear_target(ped, tp, out)
}

/// `CTaskSimpleCarDrive::GetTaskType` (0x63C450): 709.
#[hook(addr = 0x63C450, conv = thiscall, name = "CTaskSimpleCarDrive::GetTaskType", expect = "B8 C5 02 00 00",
       route = "L2", hazards = "none", recs = "driving")]
pub fn car_drive_get_task_type(_this: GPtr<u8>) -> u32 {
    0x2C5
}

/// `CTaskComplexCarDrive::Drive` (0x63CAD0, `ret 4`): returns +8 (the argument is ignored).
#[hook(addr = 0x63CAD0, conv = thiscall, name = "CTaskComplexCarDrive::Drive", expect = "8B 41 08 C2 04 00",
       route = "L2", hazards = "none", recs = "driving")]
pub fn complex_car_drive_drive(this: GPtr<u8>, _ped: u32) -> u32 {
    rd::<u32>(this, 8)
}

/// `CTaskSimpleCarDrive::UpdateBopping` (0x63C900): with e = now - start (+0x20) and the period +0x28: +0x2C =
/// `(float)(e % period) / period` (fild/fidiv, f32) and +0x30 = `(e / period) % 2` (C remainder, signed).
#[hook(addr = 0x63C900, conv = thiscall, name = "CTaskSimpleCarDrive::UpdateBopping", expect = "83 EC 08 A1 84 CB B7 00",
       route = "L2", hazards = "heap-writes", recs = "driving")]
pub fn car_drive_update_bopping(this: GPtr<u8>) {
    let e = g::<i32>(TIME_MS).read().wrapping_sub(rd::<i32>(this, 0x20));
    let period = rd::<i32>(this, 0x28);
    let q = e / period;
    let r = e % period;
    wr::<f32>(this, 0x2C, f32_of(xi(r).fdiv(xi(period))));
    wr::<i32>(this, 0x30, q % 2);
}

/// `CTaskSimpleCarDrive::ProcessBopping(CPed*, bool)` (0x642E70, `ret 8`; one load in the protection fragment
/// 0x4012DB): nothing when the ped's vehicle (+0x58C) is driven (+0x460) by `FindPlayerPed(0)`, the ped type (+0x598)
/// is 6, the ped has an active task of type 824, the vehicle's +0x590 is non-zero or +0x594 is 2, or ped byte +0x484
/// is 2. Else `UpdateBopping` when +0x20 != -1, dist = |ped position - camera position| (f32), then
/// `ProcessHeadBopping` and `ProcessArmBopping` (ped, flag, dist); finally +0x20 = -1 when it is not -1 and +0x5C has
/// neither bit 0 nor bit 1. The original stores dist in its first argument slot.
#[hook(addr = 0x642E70, conv = thiscall, name = "CTaskSimpleCarDrive::ProcessBopping", expect = "83 EC 0C 56 8B 74 24 14",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes", recs = "driving")]
pub fn car_drive_process_bopping(this: GPtr<u8>, ped: GPtr<u8>, flag: u32) {
    let player = find_player_ped(0);
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    if rd::<GPtr<u8>>(veh, 0x460) == player || rd::<u32>(ped, 0x598) == 6 {
        return;
    }
    let tm = rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
    if !task_manager_find_active_task_by_type(tm, 0x338).is_null() {
        return;
    }
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    if rd::<u32>(veh, 0x590) != 0 || rd::<u32>(veh, 0x594) == 2 || rd::<u8>(ped, 0x484) == 2 {
        return;
    }
    if rd::<i32>(this, 0x20) != -1 {
        update_bopping_o(this);
    }
    let cm = g::<GPtr<u8>>(0xB6F03C).read();
    let cam = if cm.is_null() { g::<u8>(0xB6F02C) } else { cm.byte_add(0x30) };
    let mut v = [0u32; 3];
    let vp = local(&mut v);
    vector_sub(vp, pos(ped), cam);
    let dist = st0_f32(vector_magnitude(vp)).to_bits();
    process_head_bopping(this, ped, flag, dist);
    process_arm_bopping(this, ped, flag, dist);
    if rd::<i32>(this, 0x20) != -1 && rd::<u8>(this, 0x5C) & 3 == 0 {
        wr::<i32>(this, 0x20, -1);
    }
}

/// `CTaskSimpleRunAnim::CTaskSimpleRunAnim(AssocGroupId, AnimationId, float blendDelta, bool holdLastFrame)`
/// (0x61A8B0, `ret 0x10`): `CTaskSimple` base; flag byte +0xC bit 2 = hold (bits 0..1 and 3..7 kept), +0x14 anim,
/// +8 = 0, vtable 0x86D528, +0x10 group, +0x18 blend delta, word +0x1C = 400 (task type). Returns this.
#[hook(addr = 0x61A8B0, conv = thiscall, name = "CTaskSimpleRunAnim::Constructor", expect = "56 8B F1 E8 D8 FA FF FF",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn run_anim_ctor(this: GPtr<u8>, group: u32, anim: u32, delta: u32, hold: u8) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u8>(this, 0xC, ((hold & 1) << 2) | (rd::<u8>(this, 0xC) & 0xF8));
    wr::<u32>(this, 0x14, anim);
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0, 0x86D528);
    wr::<u32>(this, 0x10, group);
    wr::<u32>(this, 0x18, delta);
    wr::<u16>(this, 0x1C, 0x190);
    this
}

/// `CTaskSimpleRunAnim::CTaskSimpleRunAnim(AssocGroupId, AnimationId, float blendDelta, eTaskType, const char*,
/// bool holdLastFrame)` (0x61A900, `ret 0x18`): as the 4-argument constructor with the word task type (+0x1C) from
/// the fourth argument; the name is unused.
#[hook(addr = 0x61A900, conv = thiscall, name = "CTaskSimpleRunAnim::Constructor2", expect = "56 8B F1 E8 88 FA FF FF",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn run_anim_ctor2(this: GPtr<u8>, group: u32, anim: u32, delta: u32, ty: u32, _name: u32, hold: u8) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u8>(this, 0xC, ((hold & 1) << 2) | (rd::<u8>(this, 0xC) & 0xF8));
    wr::<u32>(this, 0x10, group);
    wr::<u32>(this, 0x14, anim);
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0, 0x86D528);
    wr::<u32>(this, 0x18, delta);
    wr::<u16>(this, 0x1C, ty as u16);
    this
}

/// `CTaskSimpleRunTimedAnim::CTaskSimpleRunTimedAnim(AssocGroupId, AnimationId, float blendDelta, float blendOut,
/// int32 duration, eTaskType, const char*, bool holdLastFrame)` (0x61ABE0, `ret 0x20`; one load in the protection
/// fragment 0x40685A): `CTaskSimple` base; +0xC bit 2 = hold; +0x10..+0x20 = the first five arguments; +8, +0x24,
/// +0x28 = 0, bytes +0x2C/+0x2D = 0, vtable 0x86D570, word +0x30 = task type. Returns this.
#[hook(addr = 0x61ABE0, conv = thiscall, name = "CTaskSimpleRunTimedAnim::Constructor-2", expect = "56 8B F1 E8 A8 F7 FF FF",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
#[allow(clippy::too_many_arguments)]
pub fn run_timed_anim_ctor2(this: GPtr<u8>, group: u32, anim: u32, delta: u32, out: u32, dur: u32, ty: u32, _name: u32, hold: u8) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u8>(this, 0xC, ((hold & 1) << 2) | (rd::<u8>(this, 0xC) & 0xF8));
    wr::<u32>(this, 0x10, group);
    wr::<u32>(this, 0x14, anim);
    wr::<u32>(this, 0x18, delta);
    wr::<u32>(this, 0x1C, out);
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0, 0x86D570);
    wr::<u32>(this, 0x20, dur);
    wr::<u32>(this, 0x24, 0);
    wr::<u32>(this, 0x28, 0);
    wr::<u8>(this, 0x2C, 0);
    wr::<u8>(this, 0x2D, 0);
    wr::<u16>(this, 0x30, ty as u16);
    this
}

/// `CTaskSimpleRunAnim::StartAnim(CPed*)` (0x61A950, `ret 4`): +8 = `BlendAnimation(ped clump (+0x18), group
/// (+0x10), anim (+0x14), delta (+0x18))`, then its finish callback = `FinishRunAnimCB(this)` (no null test).
#[hook(addr = 0x61A950, conv = thiscall, name = "CTaskSimpleRunAnim::StartAnim", expect = "56 8B F1 8B 46 18",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn run_anim_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let a = blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(this, 0x10), rd::<u32>(this, 0x14), rd::<u32>(this, 0x18));
    wr(this, 8, a);
    assoc_set_finish_callback(a, FINISH_RUN_ANIM_CB, this);
}

/// `CTaskSimpleRunTimedAnim::StartAnim(CPed*)` (0x61AC50, `ret 4`): as `CTaskSimpleRunAnim::StartAnim` but with
/// `SetDeleteCallback` (0x4CEBC0).
#[hook(addr = 0x61AC50, conv = thiscall, name = "CTaskSimpleRunTimedAnim::StartAnim", expect = "56 8B F1 8B 46 18",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn run_timed_anim_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let a = blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(this, 0x10), rd::<u32>(this, 0x14), rd::<u32>(this, 0x18));
    wr(this, 8, a);
    assoc_set_delete_callback(a, FINISH_RUN_ANIM_CB, this);
}

/// The event destructors of shape `vtbl = V; if (ref) CleanUpOldReference(&ref); vtbl = CEvent` (the SEH frame has
/// no other effect).
fn event_dtor_with_ref(this: GPtr<u8>, vt: u32, off: isize) {
    wr::<u32>(this, 0, vt);
    let e = rd::<GPtr<u8>>(this, off);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(off));
    }
    wr::<u32>(this, 0, 0x85AB38);
}

/// `CEventPlayerCommandToGroup::~CEventPlayerCommandToGroup` (0x4B2470): vtable 0x85B800, releases the reference
/// at +0x18, then the `CEvent` vtable.
#[hook(addr = 0x4B2470, conv = thiscall, name = "FUN_004b2470", expect = "6A FF 68 98 A2 83 00",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_player_command_to_group_dtor(this: GPtr<u8>) {
    event_dtor_with_ref(this, 0x85B800, 0x18);
}

/// `CEventPedCollisionWithPed::~CEventPedCollisionWithPed` (0x4ACA70): vtable 0x85AC68, releases the reference at
/// +0x14, then the `CEvent` vtable.
#[hook(addr = 0x4ACA70, conv = thiscall, name = "FUN_004aca70", expect = "6A FF 68 E8 99 83 00",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_ped_collision_with_ped_dtor(this: GPtr<u8>) {
    event_dtor_with_ref(this, 0x85AC68, 0x14);
}

/// `CEventBuildingCollision::~CEventBuildingCollision` (0x4ACFF0): vtable 0x85ACE8, releases the reference at
/// +0x14, then the `CEvent` vtable.
#[hook(addr = 0x4ACFF0, conv = thiscall, name = "FUN_004acff0", expect = "6A FF 68 68 9A 83 00",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_building_collision_dtor(this: GPtr<u8>) {
    event_dtor_with_ref(this, 0x85ACE8, 0x14);
}

/// `CEventPotentialWalkIntoVehicle::~CEventPotentialWalkIntoVehicle` (0x4AE3C0): vtable 0x85AED0, releases the
/// reference at +0x14, then the `CEvent` vtable.
#[hook(addr = 0x4AE3C0, conv = thiscall, name = "FUN_004ae3c0", expect = "6A FF 68 E8 9B 83 00",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_potential_walk_into_vehicle_dtor(this: GPtr<u8>) {
    event_dtor_with_ref(this, 0x85AED0, 0x14);
}

/// `CEventPotentialWalkIntoPed::~CEventPotentialWalkIntoPed` (0x4AE7A0): vtable 0x85AF60, releases the reference at
/// +0x20, then the `CEvent` vtable.
#[hook(addr = 0x4AE7A0, conv = thiscall, name = "FUN_004ae7a0", expect = "6A FF 68 68 9C 83 00",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_potential_walk_into_ped_dtor(this: GPtr<u8>) {
    event_dtor_with_ref(this, 0x85AF60, 0x20);
}

/// `CPointRoute::operator=(const CPointRoute&)` (0x63A040, `ret 4`): count (+0) copied, then while i < this->count
/// (re-read each step) point i (+4, 0xC each) copied as three dwords. EAX: the count when it is <= 0, else the
/// address after the last copied point (what the original leaves).
#[hook(addr = 0x63A040, conv = thiscall, name = "FUN_0063a040", expect = "56 8B 74 24 08",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn point_route_assign(this: GPtr<u8>, src: GPtr<u8>) -> u32 {
    let n = rd::<i32>(src, 0);
    wr::<i32>(this, 0, n);
    if n <= 0 {
        return n as u32;
    }
    let mut i = 0isize;
    loop {
        let off = 4 + 0xC * i;
        wr::<u32>(this, off, rd::<u32>(src, off));
        wr::<u32>(this, off + 4, rd::<u32>(src, off + 4));
        wr::<u32>(this, off + 8, rd::<u32>(src, off + 8));
        i += 1;
        if i as i32 >= rd::<i32>(this, 0) {
            return this.byte_add(4 + 0xC * i).addr() as u32;
        }
    }
}

/// `CPedGeometryAnalyser::ComputeEntityBoundingBoxPlanes(float z, CEntity&, CVector* normals, float* distances)`
/// (0x5F3660, cdecl): the box corners at `z` into a local array, then `ComputeEntityBoundingBoxPlanesUncached(z,
/// corners, normals, distances)`.
#[hook(addr = 0x5F3660, conv = cdecl, name = "CPedGeometryAnalyser::ComputeEntityBoundingBoxPlanes", expect = "8B 4C 24 08 83 EC 30",
       unverified = "mvpa-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn compute_entity_bounding_box_planes(z: u32, e: GPtr<u8>, normals: GPtr<u8>, d: GPtr<u8>) {
    let mut c = [0u32; 12];
    let cp = local(&mut c);
    bbox_corners_uncached(z, e, cp);
    bbox_planes_uncached(z, cp, normals, d);
}

/// `CTaskComplexCarDrive::~CTaskComplexCarDrive` (0x63CA40): vtable 0x86E934; with a vehicle (+0xC): when byte
/// +0x20 is set, restores the vehicle's bytes +0x3B9/+0x3BA/+0x3D0 from +0x1D/+0x1E/+0x1F, then releases the
/// reference; then `CTaskComplex::~CTaskComplex`.
#[hook(addr = 0x63CA40, conv = thiscall, name = "CTaskComplexCarDrive::Destructor", expect = "6A FF 68 B8 18 84 00",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes", recs = "driving")]
pub fn complex_car_drive_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86E934);
    if !rd::<GPtr<u8>>(this, 0xC).is_null() {
        if rd::<u8>(this, 0x20) != 0 {
            wr::<u8>(rd::<GPtr<u8>>(this, 0xC), 0x3B9, rd::<u8>(this, 0x1D));
            wr::<u8>(rd::<GPtr<u8>>(this, 0xC), 0x3BA, rd::<u8>(this, 0x1E));
            wr::<u8>(rd::<GPtr<u8>>(this, 0xC), 0x3D0, rd::<u8>(this, 0x1F));
        }
        let v = rd::<GPtr<u8>>(this, 0xC);
        if !v.is_null() {
            entity_clean_up_old_reference(v, this.byte_add(0xC));
        }
    }
    task_complex_dtor(this);
}

/// `CTaskSimpleCarDrive::~CTaskSimpleCarDrive` (0x63C460): vtable 0x86E904; releases the vehicle reference (+8);
/// destroys and frees (`_free`) the line-up utility (+0x10) and clears it; when +0x5C has bit 0x20, the anim (+0xC,
/// no null test) gets finish callback 0x4D1390 with null data and the vehicle (when set) `ClearGettingOutFlags(1)`;
/// then `CTaskSimple::~CTaskSimple`.
#[hook(addr = 0x63C460, conv = thiscall, name = "CTaskSimpleCarDrive::Destructor", expect = "6A FF 68 38 18 84 00",
       route = "L2", hazards = "heap-writes", recs = "driving")]
pub fn simple_car_drive_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86E904);
    let v = rd::<GPtr<u8>>(this, 8);
    if !v.is_null() {
        entity_clean_up_old_reference(v, this.byte_add(8));
    }
    let u = rd::<GPtr<u8>>(this, 0x10);
    if !u.is_null() {
        line_up_dtor(u);
        crt_free(u);
        wr::<u32>(this, 0x10, 0);
    }
    if rd::<u8>(this, 0x5C) & 0x20 != 0 {
        assoc_set_finish_callback(rd::<GPtr<u8>>(this, 0xC), 0x4D1390, GPtr::NULL);
        let v = rd::<GPtr<u8>>(this, 8);
        if !v.is_null() {
            vehicle_clear_getting_out_flags(v, 1);
        }
    }
    task_simple_dtor(this);
}

/// `CTaskSimplePlayerOnFoot::CTaskSimplePlayerOnFoot` (0x685750): `CTaskSimple` base, vtable 0x8708EC, +8 =
/// `CAnimManager::GetAnimationBlockIndex("ped" name at 0x86D1EC)`, +0x14 = 0, +0xC = `CTimer::m_FrameCounter`
/// (0xB7CB4C), +0x10 = 0, +0x18 = 0. Returns this.
#[hook(addr = 0x685750, conv = thiscall, name = "CTaskSimplePlayerOnFoot::Constructor", expect = "6A FF 68 18 6B 84 00",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn player_on_foot_ctor(this: GPtr<u8>) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0, 0x8708EC);
    let b = anim_block_index_by_name(g::<u8>(0x86D1EC));
    wr::<u32>(this, 8, b);
    wr::<u32>(this, 0x14, 0);
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0x18, 0);
    wr::<u32>(this, 0xC, g::<u32>(0xB7CB4C).read());
    this
}

/// `CTaskSimpleCarDrive::CTaskSimpleCarDrive(CVehicle*, CTaskUtilityLineUpPedWithCar*, bool updateCurrentVehicle)`
/// (0x63C340, `ret 0xC`): `CTaskSimple` base; +8 vehicle, vtable 0x86E904; +0xC..+0x18, +0x4C..+0x54 = 0, bytes
/// +0x1C/+0x1D/+0x58/+0x59 = 0; flag byte +0x5C bits 2/3 = (update & 1) | 2 (bits 0..1, 4, 5 kept... mask 0xCB);
/// registers the vehicle reference; with a line-up template: +0x10 = a new (CRT heap, 0x1C) `CTaskUtilityLineUpPed
/// WithCar((0,0,0), 0, template+0x14, template+0x18)` (null when the allocation fails); then +0x5C bits 0..1
/// cleared, +0x44/+0x3C/+0x40 = 0, +0x20 = -1. Returns this.
#[hook(addr = 0x63C340, conv = thiscall, name = "CTaskSimpleCarDrive::Constructor", expect = "6A FF 68 23 18 84 00",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes", recs = "driving")]
pub fn simple_car_drive_ctor(this: GPtr<u8>, veh: GPtr<u8>, util: GPtr<u8>, update: u8) -> GPtr<u8> {
    task_simple_ctor(this);
    wr(this, 8, veh);
    wr::<u32>(this, 0, 0x86E904);
    for off in [0xC, 0x10, 0x14, 0x18] {
        wr::<u32>(this, off, 0);
    }
    wr::<u8>(this, 0x1C, 0);
    wr::<u8>(this, 0x1D, 0);
    wr::<u32>(this, 0x4C, 0);
    wr::<u32>(this, 0x50, 0);
    wr::<u32>(this, 0x54, 0);
    wr::<u8>(this, 0x58, 0);
    wr::<u8>(this, 0x59, 0);
    wr::<u8>(this, 0x5C, (((update & 1) | 2) << 2) | (rd::<u8>(this, 0x5C) & 0xCB));
    let v = rd::<GPtr<u8>>(this, 8);
    if !v.is_null() {
        entity_register_reference(v, this.byte_add(8));
    }
    if !util.is_null() {
        let p = operator_new(0x1C);
        let r = if p.is_null() {
            GPtr::NULL
        } else {
            let mut z = [0u32; 3];
            line_up_ctor(p, local(&mut z), 0, rd::<u32>(util, 0x14), rd::<u32>(util, 0x18))
        };
        wr(this, 0x10, r);
    }
    wr::<u8>(this, 0x5C, rd::<u8>(this, 0x5C) & 0xFC);
    wr::<u32>(this, 0x44, 0);
    wr::<u32>(this, 0x3C, 0);
    wr::<u32>(this, 0x40, 0);
    wr::<i32>(this, 0x20, -1);
    this
}

/// `CTaskSimpleAnim::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x61A790, `ret 0xC`, AL): delta = -4.0;
/// priority 2 (immediate) clears hold bit 2 of +0xC and uses -1000.0. Unless immediate, a task with bit 1 of +0xC
/// returns false for an event that is not of type 32. For a type-32 event whose task (+0x10) has type 401, the anim
/// (+8, when set) only gets flag 4 (word +0x2E). Otherwise the anim (when set) gets flag 4 and, unless +0xC bit 2,
/// either its blend delta (+0x1C) = delta (anim flag 0x10) or `BlendAnimation(ped clump, ped anim group (+0x4D4), 3,
/// -delta)`. Then priorities 1 and 2 and held tasks (bit 2) succeed: the anim's finish callback becomes 0x4D1390
/// (null data), +0xC bit 0 set, +8 = 0, true; others return false. Upper EAX bits are not reproduced.
#[hook(addr = 0x61A790, conv = thiscall, name = "CTaskSimpleAnim::MakeAbortable", expect = "51 53 55 8B 6C 24 14",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes,ax-al-return", recs = "grove-walk")]
pub fn simple_anim_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 {
    let mut delta: u32 = 0xC080_0000;
    let flags = rd::<u8>(this, 0xC);
    if prio == 2 {
        delta = 0xC47A_0000;
        wr::<u8>(this, 0xC, flags & 0xFB);
    } else if flags & 2 != 0 {
        if ev.is_null() {
            return finish_abortable(this, ped, prio, delta, false);
        }
        if v_get_event_type(ev) != 0x20 {
            return 0;
        }
    }
    let special = !ev.is_null() && v_get_event_type(ev) == 0x20 && {
        let t = rd::<GPtr<u8>>(ev, 0x10);
        !t.is_null() && v_get_task_type(t) == 0x191
    };
    finish_abortable(this, ped, prio, delta, special)
}

/// The tail of [`simple_anim_make_abortable`] from 0x61A7FE / 0x61A80B.
fn finish_abortable(this: GPtr<u8>, ped: GPtr<u8>, prio: i32, delta: u32, special: bool) -> u8 {
    let a = rd::<GPtr<u8>>(this, 8);
    if !a.is_null() {
        wr::<u16>(a, 0x2E, rd::<u16>(a, 0x2E) | 4);
        if !special && rd::<u8>(this, 0xC) & 4 == 0 {
            let a = rd::<GPtr<u8>>(this, 8);
            if (rd::<u8>(a, 0x2E) >> 4) & 1 != 0 {
                wr::<u32>(a, 0x1C, delta);
            } else {
                let neg = f32_of(x(f32::from_bits(delta)).fchs());
                blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), 3, neg.to_bits());
            }
        }
    }
    if prio != 1 && prio != 2 && rd::<u8>(this, 0xC) & 4 == 0 {
        return 0;
    }
    let a = rd::<GPtr<u8>>(this, 8);
    if !a.is_null() {
        assoc_set_finish_callback(a, 0x4D1390, GPtr::NULL);
        wr::<u8>(this, 0xC, rd::<u8>(this, 0xC) | 1);
        wr::<u32>(this, 8, 0);
    }
    1
}

/// `CTaskSimpleRunTimedAnim::ProcessPed(CPed*)` (0x61BBE0, `ret 4`, AL): finished (+0xC bit 0) returns true (upper
/// EAX untouched). With the timer started (+0x2C): a pending reset (+0x2D) sets start (+0x24) = now; when `now >=
/// start + interval (+0x28)` (unsigned) the task's `MakeAbortable(ped, 0, null)` (vtable +0x18). Without an anim (+8)
/// and no association of anim +0x14 on the ped's clump: start = now, interval = duration (+0x20), started, then
/// `StartAnim(ped)`. Returns +0xC bit 0.
#[hook(addr = 0x61BBE0, conv = thiscall, name = "CTaskSimpleRunTimedAnim::ProcessPed", expect = "56 8B F1 F6 46 0C 01",
       unverified = "mvpa-ai-2", route = "L2", hazards = "heap-writes,ax-al-return", recs = "grove-walk")]
pub fn run_timed_anim_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0xC) & 1 != 0 {
        return 1;
    }
    if rd::<u8>(this, 0x2C) != 0 {
        if rd::<u8>(this, 0x2D) != 0 {
            wr::<u32>(this, 0x24, g::<u32>(TIME_MS).read());
            wr::<u8>(this, 0x2D, 0);
        }
        let end = rd::<u32>(this, 0x28).wrapping_add(rd::<u32>(this, 0x24));
        if g::<u32>(TIME_MS).read() >= end {
            v_make_abortable(this, ped, 0, 0);
        }
    }
    if rd::<u32>(this, 8) == 0 && clump_get_association(rd::<u32>(ped, 0x18), rd::<u32>(this, 0x14)).is_null() {
        let dur = rd::<u32>(this, 0x20);
        wr::<u32>(this, 0x24, g::<u32>(TIME_MS).read());
        wr::<u32>(this, 0x28, dur);
        wr::<u8>(this, 0x2C, 1);
        start_timed_anim(this, ped);
    }
    rd::<u8>(this, 0xC) & 1
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_PLAYER_ON_FOOT_CTOR,
    HOOK_SIMPLE_CAR_DRIVE_CTOR,
    HOOK_SIMPLE_ANIM_MAKE_ABORTABLE,
    HOOK_RUN_TIMED_ANIM_PROCESS_PED,
    HOOK_EVENT_PLAYER_COMMAND_TO_GROUP_DTOR,
    HOOK_EVENT_PED_COLLISION_WITH_PED_DTOR,
    HOOK_EVENT_BUILDING_COLLISION_DTOR,
    HOOK_EVENT_POTENTIAL_WALK_INTO_VEHICLE_DTOR,
    HOOK_EVENT_POTENTIAL_WALK_INTO_PED_DTOR,
    HOOK_POINT_ROUTE_ASSIGN,
    HOOK_COMPUTE_ENTITY_BOUNDING_BOX_PLANES,
    HOOK_COMPLEX_CAR_DRIVE_DTOR,
    HOOK_SIMPLE_CAR_DRIVE_DTOR,
    HOOK_COMPUTE_ENTITY_HIT_SIDE_2,
    HOOK_TASK_COMPLEX_WANDER_DTOR,
    HOOK_WALK_ROUND_BUILDING_ATTEMPT_CREATE_SUB_TASK,
    HOOK_EVENT_HANDLER_GET_CURRENT_EVENT_TYPE,
    HOOK_EVENT_HANDLER_HISTORY_IS_RESPONDING_TO_EVENT,
    HOOK_TASK_TIMER_IS_OUT_OF_TIME,
    HOOK_SEEK_POS_STANDARD,
    HOOK_SEEK_POS_XY_OFFSET,
    HOOK_CAR_DRIVE_GET_TASK_TYPE,
    HOOK_COMPLEX_CAR_DRIVE_DRIVE,
    HOOK_CAR_DRIVE_UPDATE_BOPPING,
    HOOK_CAR_DRIVE_PROCESS_BOPPING,
    HOOK_RUN_ANIM_CTOR,
    HOOK_RUN_ANIM_CTOR2,
    HOOK_RUN_TIMED_ANIM_CTOR2,
    HOOK_RUN_ANIM_START_ANIM,
    HOOK_RUN_TIMED_ANIM_START_ANIM,
];
