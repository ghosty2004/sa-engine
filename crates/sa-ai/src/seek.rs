//! `CTaskComplexSeekEntity` family (PORT-AI-4, helper A): the `CEntitySeekPosCalculatorStandard`
//! instance (`CTaskComplexSeekEntityStandard`, flags byte at +0x48) and the
//! `CEntitySeekPosCalculatorXYOffset` instance (flags at +0x54). Ported from the 1.0 US bytes
//! (`docs/re/specs/AI/CTaskComplexSeekEntity*.md` as reading aid); the exe wins over gta-reversed.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(int, bool, bool, float)` (0x62F310)
    fn stand_still_ctor(t: GPtr<u8>, time: u32, a: u32, b: u32, f: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskSimpleStandStill::ProcessPed(CPed*)` (0x62F370)
    fn stand_still_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x62F370 => unported;
    /// `FUN_0062f360` (0x62F360): `CTaskSimpleStandStill` destructor
    fn stand_still_dtor(t: GPtr<u8>) = thiscall 0x62F360 => unported;
    /// `CTaskSimpleTired::Constructor(int)` (0x630F20)
    fn tired_ctor(t: GPtr<u8>, time: u32) -> GPtr<u8> = thiscall 0x630F20 => unported;
    /// `CTaskComplexSequence::Constructor` (0x632BD0)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10)
    fn sequence_add_task(t: GPtr<u8>, task: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CTaskComplexTurnToFaceEntityOrCoord::Constructor(CEntity*, float, float)` (0x66B890)
    fn turn_to_face_ctor(t: GPtr<u8>, e: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x66B890 => unported;
    /// `CTaskComplexLeaveCar::CTaskComplexLeaveCar(CVehicle*, int, int, bool, bool)` (0x63B8C0)
    fn leave_car_ctor(t: GPtr<u8>, v: u32, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    /// `CTaskSimpleCarDrive::Constructor(CVehicle*, CTaskUtilityLineUpPedWithCar*, bool)` (0x63C340)
    fn car_drive_simple_ctor(t: GPtr<u8>, v: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x63C340 => unported;
    /// `CTaskComplexGoToPointAndStandStill::Constructor(int, const CVector&, float, float, bool, bool)` (0x668120)
    fn go_to_point_and_stand_still_ctor(t: GPtr<u8>, ms: u32, pos: GPtr<u8>, r: u32, r2: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x668120 => unported;
    /// `CTaskComplexFollowNodeRoute::Constructor(int, const CVector&, float, float, float, bool, int, bool)` (0x66EA30)
    fn follow_node_route_ctor(t: GPtr<u8>, ms: u32, pos: GPtr<u8>, r: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> GPtr<u8> = thiscall 0x66EA30 => unported;
    /// `CPed::SetMoveState(eMoveState)` (0x5DEC00)
    fn ped_set_move_state(ped: GPtr<u8>, s: u32) = thiscall 0x5DEC00 => unported;
    /// `CPedGroups::AreInSameGroup(const CPed*, const CPed*)` (0x5F7F40, cdecl, AL)
    fn ped_groups_are_in_same_group(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x5F7F40 => unported;
    /// `CPed::DettachPedFromEntity` (0x5E7EC0)
    fn ped_dettach_from_entity(ped: GPtr<u8>) = thiscall 0x5E7EC0 => unported;
    /// `CVector::Normalise` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CVector::Difference` (0x40FE00, `ecx = out`): `out = a - b`
    fn vector_difference(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = thiscall 0x40FE00 => unported;
    /// `CPedPlacement::FindZCoorForPed(CVector*)` (0x616920, cdecl, AL)
    fn ped_placement_find_z_coor_for_ped(v: GPtr<u8>) -> u8 = cdecl 0x616920 => unported;
    /// `CMatrix::Invert(const CMatrix&, CMatrix&)` (0x59B920, cdecl)
    fn matrix_invert(src: GPtr<u8>, dst: GPtr<u8>) -> GPtr<u8> = cdecl 0x59B920 => unported;
    /// `CVector::FromMultiply3x3(const CMatrix&, const CVector&)` (0x59C6D0, `ecx = out`)
    fn vector_from_multiply3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) = thiscall 0x59C6D0 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float, float, float, float)` (0x53CBE0, cdecl, ST0)
    fn general_get_radian_angle_between_points_st0(x1: f32, y1: f32, x2: f32, y2: f32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, cdecl, ST0)
    fn general_limit_radian_angle_st0(a: f32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CPed::AttachPedToEntity(CEntity*, CVector, uint16, float, eWeaponType)` (0x5E7CB0)
    fn ped_attach_to_entity(ped: GPtr<u8>, e: GPtr<u8>, ox: u32, oy: u32, oz: u32, dir: u32, lim: u32, w: u32) = thiscall 0x5E7CB0 => unported;
    /// `CMatrix::~CMatrix` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CTaskTimer::IsOutOfTime` (0x420E30, AL)
    fn task_timer_is_out_of_time(t: GPtr<u8>) -> u8 = thiscall 0x420E30 => unported;
    /// `FUN_0046fee0` (0x46FEE0, `ret 8`): the seek position into `out`; returns `out`
    fn std_seek_pos(this: GPtr<u8>, out: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x46FEE0 => unported;
    /// `FUN_00665dc0` (0x665DC0, `ret 8`): the XYOffset instance's seek position into `out`
    fn xy_seek_pos(this: GPtr<u8>, out: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x665DC0 => unported;
    /// `CTaskComplexFollowNodeRoute::SetTarget(CPed*, const CVector&, float, float, float, bool)` (0x671750)
    fn follow_node_route_set_target(t: GPtr<u8>, ped: GPtr<u8>, pos: GPtr<u8>, a: u32, b: u32, c: u32, d: u32) = thiscall 0x671750 => unported;
    /// `CTaskComplexGoToPointAndStandStill::GoToPoint(const CVector&, float, float, bool)` (0x46FE60)
    fn go_to_point_and_stand_still_go_to_point(t: GPtr<u8>, pos: GPtr<u8>, a: u32, b: u32, c: u32) = thiscall 0x46FE60 => unported;
    /// `CWorld::FindPlayerPed(int)` (0x56E210, cdecl)
    fn find_player_ped(i: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPedGroupMembership::IsMember(const CPed*)` (0x5F6A10, AL)
    fn membership_is_member(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5F6A10 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0, AL)
    fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::Say(uint16, uint32, float, bool, bool, bool)` (0x5EFFE0)
    fn ped_say(ped: GPtr<u8>, id: u32, off: u32, vol: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CTask::MakeAbortable(CPed*, int, const CEvent*)` (vtable +0x18, AL)
    fn task_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, e: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplexSeekEntityStandard::CreateSubTask(int, CPed*)` (0x46F1E0)
    fn std_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x46F1E0 => unported;
    /// `CTaskComplexSeekEntity<XYOffset>::CreateSubTask(int, CPed*)` (0x665270)
    fn xy_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x665270 => seek_entity_xy_create_sub_task_665270;

    /// `CTask::GetTaskType` (vtable +0x10)
    fn task_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskComplex::CreateFirstSubTask(CPed*)` (vtable +0x24)
    fn task_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CEntitySeekPosCalculator::ComputeEntitySeekPos(const CPed&, const CEntity&, CVector&)` (vtable +0)
    fn calc_compute_pos(c: GPtr<u8>, ped: GPtr<u8>, e: u32, out: GPtr<u8>) = thiscall vslot 0x0 => unported;
}

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;

/// Pointer to a Rust stack buffer (stands in for an object the original builds on its own stack).
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// Entity position: matrix +0x30, else the placement at +4.
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `fld dword [addr]`.
fn gf(addr: usize) -> X87 {
    x(g::<f32>(addr).read())
}

/// The shared constructor body (both instances; `ret 0x20`): `CTaskComplex` ctor, `+0xC = entity`,
/// `+0x10..+0x1C`, `vtbl`, `+0x20 = 0`, `+0x24`, both task timers cleared (`+0x28..+0x3D`), the
/// calculator vtable at +0x40 (`extra` zeroed dwords after it), flags byte `(old & 0xF0) | b8 << 1 |
/// b7`, move state 6 just before it, and the entity reference registered when set.
#[allow(clippy::too_many_arguments)]
fn seek_ctor(
    this: GPtr<u8>,
    vtbl: u32,
    calc_vtbl: u32,
    extra: isize,
    args: [u32; 6],
    b7: u32,
    b8: u32,
) -> GPtr<u8> {
    task_complex_ctor(this);
    for (off, a) in [0xC, 0x10, 0x14, 0x18].into_iter().zip(&args[..4]) {
        wr::<u32>(this, off, *a);
    }
    wr::<u32>(this, 0, vtbl);
    wr::<u32>(this, 0x1C, args[4]);
    wr::<u32>(this, 0x20, 0);
    wr::<u32>(this, 0x24, args[5]);
    for o in [0x28, 0x2C] {
        wr::<u32>(this, o, 0);
    }
    wr::<u8>(this, 0x30, 0);
    wr::<u8>(this, 0x31, 0);
    wr::<u32>(this, 0x34, 0);
    wr::<u32>(this, 0x38, 0);
    wr::<u8>(this, 0x3C, 0);
    wr::<u8>(this, 0x3D, 0);
    wr::<u32>(this, 0x40, calc_vtbl);
    let mut o = 0x44;
    while o < 0x44 + extra {
        wr::<u32>(this, o, 0);
        o += 4;
    }
    let fo = 0x48 + extra;
    let f = ((b8 as u8 & 1) << 1) | (rd::<u8>(this, fo) & 0xF0) | (b7 as u8 & 1);
    wr::<u8>(this, fo, f);
    wr::<u32>(this, fo - 4, 6);
    let e = rd::<GPtr<u8>>(this, 0xC);
    if !e.is_null() {
        entity_register_reference(e, this.byte_add(0xC));
    }
    this
}

/// `CTaskComplexSeekEntityStandard::Constructor(CEntity*, int, int, float, float, float, bool, bool)`
/// (0x46AC10: `.HOODLUM` stub to 0x156E100, `ret 0x20`): vtbl 0x859DF8, calculator vtbl 0x859DC8,
/// move state (+0x44) = 6, flags at +0x48.
#[hook(addr = 0x46AC10, conv = thiscall, name = "CTaskComplexSeekEntityStandard::Constructor", expect = "E9 EB 34 10 01")]
#[allow(clippy::too_many_arguments)]
pub fn seek_entity_standard_ctor_46ac10(this: GPtr<u8>, e: u32, a: u32, b: u32, c: u32, d: u32, f: u32, b7: u32, b8: u32) -> GPtr<u8> {
    seek_ctor(this, 0x859DF8, 0x859DC8, 0, [e, a, b, c, d, f], b7, b8)
}

/// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::Constructor` (0x661DC0, `ret 0x20`):
/// vtbl 0x86FBB4, calculator vtbl 0x86F8F8 with its offset (+0x44..+0x4C) zeroed, move state (+0x50)
/// = 6, flags at +0x54.
#[hook(addr = 0x661DC0, conv = thiscall, name = "CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::Constructor", expect = "6A FF 68 53 49 84 00")]
#[allow(clippy::too_many_arguments)]
pub fn seek_entity_xy_ctor_661dc0(this: GPtr<u8>, e: u32, a: u32, b: u32, c: u32, d: u32, f: u32, b7: u32, b8: u32) -> GPtr<u8> {
    seek_ctor(this, 0x86FBB4, 0x86F8F8, 0xC, [e, a, b, c, d, f], b7, b8)
}

/// `d² = dx² + dy²` (x87 order) from the ped to the calculator's seek position (computed into a
/// stack vector through the calculator at +0x40).
fn seek_dist2(this: GPtr<u8>, ped: GPtr<u8>) -> X87 {
    let mut l = [0u32; 3];
    let lp = buf(&mut l);
    calc_compute_pos(this.byte_add(0x40), ped, rd::<u32>(this, 0xC), lp);
    let pp = ent_pos(ped);
    let dx = x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(lp, 0));
    let dy = x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(lp, 4));
    dx.fmul(dx).fadd(dy.fmul(dy))
}

/// Not arrived: with no "slow down" radius (+0x20 == 0, NaN not equal) → 0x38A (follow node route);
/// else `d² > s²` → 0x38A, otherwise 0x387 (go to point).
fn far_task_type(this: GPtr<u8>, d2: X87) -> i32 {
    if !x(rd::<f32>(this, 0x20)).fcom(gf(0x858B50)).test_parity(0x44) {
        return 0x38A;
    }
    let s = x(rd::<f32>(this, 0x20));
    if d2.fcom(s.fmul(s)).test_any(0x41) {
        0x387
    } else {
        0x38A
    }
}

/// The shared `CreateNextSubTask(CPed*)` body (`ret 4`); `fo` = flags offset, `create` = the
/// instance's `CreateSubTask`.
fn seek_create_next_sub_task(
    this: GPtr<u8>,
    ped: GPtr<u8>,
    fo: isize,
    create: fn(GPtr<u8>, i32, GPtr<u8>) -> GPtr<u8>,
) -> GPtr<u8> {
    if rd::<u32>(this, 0xC) == 0 {
        return create(this, 0x516, ped);
    }
    let t = task_get_task_type(rd::<GPtr<u8>>(this, 8));
    match t {
        0x387 | 0x38A => {
            let d2 = seek_dist2(this, ped);
            let r = x(rd::<f32>(this, 0x18));
            if !d2.fcom(r.fmul(r)).test_any(0x41) {
                let ty = far_task_type(this, d2);
                return create(this, ty, ped);
            }
            let f = rd::<u8>(this, fo);
            wr::<u8>(this, fo, f | 4);
            create(this, if f & 2 != 0 { 0x398 } else { 0x516 }, ped)
        }
        0x398 | 0xF4 => create(this, 0x516, ped),
        0x2CF => task_create_first_sub_task(this, ped),
        0x2C0 => {
            let ty = if rd::<u32>(ped, 0x46C) & 0x100 != 0 { 0x516 } else { 0x2CF };
            create(this, ty, ped)
        }
        0xCB => {
            if rd::<u32>(ped, 0xFC) != 0 {
                create(this, 0xCB, ped)
            } else {
                task_create_first_sub_task(this, ped)
            }
        }
        0xDB => {
            let mut l = [0u32; 8];
            let st = buf(&mut l);
            stand_still_ctor(st, 0xFFFF_FFFF, 0, 0, 0x4100_0000);
            stand_still_process_ped(st, ped);
            stand_still_dtor(st);
            create(this, 0x516, ped)
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexSeekEntityStandard::CreateNextSubTask(CPed*)` (0x46EFF0, `ret 4`): by the finished
/// sub task: no entity → 0x516; 0xDB → a stack `CTaskSimpleStandStill(-1, 0, 0, 8.0)` processed once
/// (0x20 bytes), then 0x516; 0xCB → attached ? 0xCB : `CreateFirstSubTask`; 0xF4 / 0x398 → 0x516;
/// 0x2C0 → in a vehicle ? 0x516 : 0x2CF; 0x2CF → `CreateFirstSubTask`; 0x387 / 0x38A → arrived
/// (`d² <= r²`, NaN arrives) sets flag 4 and gives 0x398 (flag 2) or 0x516, else 0x387 / 0x38A by
/// the slow-down radius; others null.
#[hook(addr = 0x46EFF0, conv = thiscall, name = "CTaskComplexSeekEntityStandard::CreateNextSubTask", expect = "64 A1 00 00 00 00")]
pub fn seek_entity_standard_create_next_sub_task_46eff0(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    seek_create_next_sub_task(this, ped, 0x48, std_create_sub_task)
}

/// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::CreateNextSubTask(CPed*)` (0x665080,
/// `ret 4`): as 0x46EFF0 with flags at +0x54 and `CreateSubTask` 0x665270.
#[hook(addr = 0x665080, conv = thiscall, name = "CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::CreateNextSubTask", expect = "64 A1 00 00 00 00")]
pub fn seek_entity_xy_create_next_sub_task_665080(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    seek_create_next_sub_task(this, ped, 0x54, xy_create_sub_task)
}

/// The shared `CreateFirstSubTask(CPed*)` body (`ret 4`).
fn seek_create_first_sub_task(
    this: GPtr<u8>,
    ped: GPtr<u8>,
    fo: isize,
    create: fn(GPtr<u8>, i32, GPtr<u8>) -> GPtr<u8>,
) -> GPtr<u8> {
    let iv = rd::<u32>(this, 0x14);
    wr::<u32>(this, 0x28, g::<u32>(TIME_MS).read());
    wr::<u32>(this, 0x2C, iv);
    wr::<u8>(this, 0x30, 1);
    let iv = rd::<i32>(this, 0x10);
    if iv != -1 {
        wr::<u32>(this, 0x34, g::<u32>(TIME_MS).read());
        wr::<i32>(this, 0x38, iv);
        wr::<u8>(this, 0x3C, 1);
    }
    if rd::<u32>(this, 0xC) == 0 {
        return create(this, 0xDB, ped);
    }
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 && rd::<u32>(ped, 0x58C) != 0 {
        return create(this, 0x2C0, ped);
    }
    let d2 = seek_dist2(this, ped);
    let r = x(rd::<f32>(this, 0x18));
    if !d2.fcom(r.fmul(r)).test_parity(5) {
        wr::<u8>(this, fo, rd::<u8>(this, fo) | 4);
        ped_set_move_state(ped, 1);
        return create(this, 0xF4, ped);
    }
    let ty = far_task_type(this, d2);
    create(this, ty, ped)
}

/// `CTaskComplexSeekEntityStandard::CreateFirstSubTask(CPed*)` (0x46F6A0, `ret 4`): starts the
/// seek timer (`+0x28 = now, +0x2C = +0x14`) and, when `+0x10 != -1`, the give-up timer; no entity →
/// 0xDB; in a vehicle → 0x2C0; within the radius (`d² < r²`, NaN not) → flag 4, move state 1, 0xF4;
/// else 0x387 / 0x38A as in `CreateNextSubTask`.
#[hook(addr = 0x46F6A0, conv = thiscall, name = "CTaskComplexSeekEntityStandard::CreateFirstSubTask", expect = "83 EC 0C 56 8B F1")]
pub fn seek_entity_standard_create_first_sub_task_46f6a0(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    seek_create_first_sub_task(this, ped, 0x48, std_create_sub_task)
}

/// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::CreateFirstSubTask(CPed*)` (0x665600,
/// `ret 4`): as 0x46F6A0 with flags at +0x54 and `CreateSubTask` 0x665270.
#[hook(addr = 0x665600, conv = thiscall, name = "CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::CreateFirstSubTask", expect = "83 EC 0C 56 8B F1")]
pub fn seek_entity_xy_create_first_sub_task_665600(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    seek_create_first_sub_task(this, ped, 0x54, xy_create_sub_task)
}

/// `CTaskComplexSeekEntityStandard::CreateSubTask(int, CPed*)` (0x46F1E0, `ret 8`): 0xDB →
/// `CTaskSimpleTired(1000)`; 0xCB → `CTaskSimpleStandStill(2000, 0, 0, 8.0)`; 0xF4 → a sequence of
/// (flag 2: turn to face the entity with `[0x86FC7C]`, `[0x86FC80]`) and a stand still (100 ms) —
/// `AddTask` is called even on a null sequence; 0x2C0 → `CTaskComplexLeaveCar(vehicle, 0, 0, 1, 0)`;
/// 0x2CF → `CTaskSimpleCarDrive(vehicle, 0, 0)` re-tabled as vtbl 0x859E50 with `+0x60 = 2000`,
/// `+0x64..+0x6D = 0`; 0x387 → `CTaskComplexGoToPointAndStandStill(moveState, seekPos, +0x18, +0x1C,
/// 0, 0)` with its `+0x24` bit 3 copied from the flags (on a null task too, as the original); 0x38A
/// → `CTaskComplexFollowNodeRoute(moveState, seekPos, +0x18, +0x1C, +0x24, 0, -1, 1)`; 0x398 → turn
/// to face; others (0x516 included) null. The seek position is computed into a stack vector and
/// copied into a second one for the constructor.
#[hook(addr = 0x46F1E0, conv = thiscall, name = "CTaskComplexSeekEntityStandard::CreateSubTask", expect = "64 A1 00 00 00 00")]
pub fn seek_entity_standard_create_sub_task_46f1e0(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    seek_create_sub_task(this, ty, ped, 0x48)
}

/// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::CreateSubTask(int, CPed*)` (0x665270: `nop; jmp`
/// into the chunk 0x402328, back at 0x665276; `ret 8`): the bytes of 0x46F1E0 with the flags at +0x54 and the move
/// state at +0x50 (STD-8).
// Not hookable: its entry bytes (0x665271..) are a code key protection code reads (0x407DF5 / 0x15650F8 `sub reg,
// [0x665271]`); a phase B native.
#[sa_core::native(addr = 0x665270, conv = thiscall, name = "CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::CreateSubTask")]
pub fn seek_entity_xy_create_sub_task_665270(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    seek_create_sub_task(this, ty, ped, 0x54)
}

/// The shared body of the two `CreateSubTask` instances: the flags at `flags` (+0x48 / +0x54) and the move state at
/// `flags - 4` (+0x44 / +0x50); otherwise the bytes are the same.
fn seek_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>, flags: isize) -> GPtr<u8> {
    match ty {
        0xDB => {
            let e = task_new(0x1C);
            if e.is_null() { GPtr::NULL } else { tired_ctor(e, 0x3E8) }
        }
        0xCB => {
            let e = task_new(0x20);
            if e.is_null() { GPtr::NULL } else { stand_still_ctor(e, 0x7D0, 0, 0, 0x4100_0000) }
        }
        0xF4 => {
            let s = task_new(0x40);
            let s = if s.is_null() { GPtr::NULL } else { sequence_ctor(s) };
            if rd::<u8>(this, flags) & 2 != 0 {
                let t = task_new(0x28);
                let t = if t.is_null() {
                    GPtr::NULL
                } else {
                    let (a, b) = (g::<u32>(0x86FC80).read(), g::<u32>(0x86FC7C).read());
                    turn_to_face_ctor(t, rd::<u32>(this, 0xC), b, a)
                };
                sequence_add_task(s, t);
            }
            let t = task_new(0x20);
            let t = if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, 0x64, 0, 0, 0x4100_0000) };
            sequence_add_task(s, t);
            s
        }
        0x2C0 => {
            let e = task_new(0x34);
            if e.is_null() {
                GPtr::NULL
            } else {
                leave_car_ctor(e, rd::<u32>(ped, 0x58C), 0, 0, 1, 0)
            }
        }
        0x2CF => {
            let e = task_new(0x70);
            if e.is_null() {
                return GPtr::NULL;
            }
            car_drive_simple_ctor(e, rd::<u32>(ped, 0x58C), 0, 0);
            wr::<u32>(e, 0, 0x859E50);
            wr::<u32>(e, 0x60, 0x7D0);
            wr::<u32>(e, 0x64, 0);
            wr::<u32>(e, 0x68, 0);
            wr::<u8>(e, 0x6C, 0);
            wr::<u8>(e, 0x6D, 0);
            e
        }
        0x387 => {
            let e = task_new(0x28);
            let e = if e.is_null() {
                GPtr::NULL
            } else {
                let mut l = [0u32; 6];
                let lp = buf(&mut l);
                calc_compute_pos(this.byte_add(0x40), ped, rd::<u32>(this, 0xC), lp);
                copy3(lp.byte_add(0xC), lp);
                go_to_point_and_stand_still_ctor(
                    e,
                    rd::<u32>(this, flags - 4),
                    lp.byte_add(0xC),
                    rd::<u32>(this, 0x18),
                    rd::<u32>(this, 0x1C),
                    0,
                    0,
                )
            };
            let b = rd::<u8>(e, 0x24);
            wr::<u8>(e, 0x24, b ^ ((rd::<u8>(this, flags) ^ b) & 8));
            e
        }
        0x38A => {
            let e = task_new(0x60);
            if e.is_null() {
                return GPtr::NULL;
            }
            let mut l = [0u32; 6];
            let lp = buf(&mut l);
            calc_compute_pos(this.byte_add(0x40), ped, rd::<u32>(this, 0xC), lp);
            copy3(lp.byte_add(0xC), lp);
            follow_node_route_ctor(
                e,
                rd::<u32>(this, flags - 4),
                lp.byte_add(0xC),
                rd::<u32>(this, 0x18),
                rd::<u32>(this, 0x1C),
                rd::<u32>(this, 0x24),
                0,
                0xFFFF_FFFF,
                1,
            )
        }
        0x398 => {
            let e = task_new(0x28);
            if e.is_null() {
                GPtr::NULL
            } else {
                let (a, b) = (g::<u32>(0x86FC80).read(), g::<u32>(0x86FC7C).read());
                turn_to_face_ctor(e, rd::<u32>(this, 0xC), b, a)
            }
        }
        _ => GPtr::NULL,
    }
}

/// Copies a 12-byte vector dword by dword.
fn copy3(dst: GPtr<u8>, src: GPtr<u8>) {
    for o in [0, 4, 8] {
        wr::<u32>(dst, o, rd::<u32>(src, o));
    }
}

/// A boat (`+0x590 == 5`) entity `v` the ped stands on / touches (`ped +0x568` or `+0x584`).
fn boat_under(ped: GPtr<u8>, v: GPtr<u8>) -> Option<bool> {
    if v.is_null() || rd::<u8>(v, 0x36) & 7 != 2 {
        return None;
    }
    if rd::<GPtr<u8>>(ped, 0x568) != v && rd::<GPtr<u8>>(ped, 0x584) != v {
        return None;
    }
    Some(rd::<u32>(v, 0x590) == 5)
}

/// The boat branch of `ControlSubTask` (0x46F94A..0x46FB61): attach the ped to the boat a step
/// (`[0x858C84]`) short of the boat along the ped→boat direction (z from `FindZCoorForPed` when
/// found), offset in the boat's frame through a stack `CMatrix` (inverse of the boat matrix,
/// attach fields zeroed, destroyed at the end), heading `_ftol(LimitRadianAngle(angle(dir.x,
/// dir.y, 0, 0)))`, 0.2 and the current weapon; returns a new `CTaskSimpleStandStill(2000)`.
fn attach_to_boat(ped: GPtr<u8>, boat: GPtr<u8>, xy: bool) -> GPtr<u8> {
    let mut l = [0u32; 40];
    let lp = buf(&mut l);
    let v1 = lp.byte_add(4);
    let n = lp.byte_add(0x10);
    let p = lp.byte_add(0x1C);
    let o = lp.byte_add(0x40);
    let m = lp.byte_add(0x58);
    let pp = ent_pos(ped);
    let bp = ent_pos(boat);
    if xy {
        vector_difference(v1, bp, pp);
    } else {
        for k in [0, 4, 8] {
            wr::<f32>(v1, k, f32_of(x(rd::<f32>(bp, k)).fsub_m(rd::<f32>(pp, k))));
        }
    }
    copy3(n, v1);
    vector_normalise(n);
    let c = g::<f32>(0x858C84).read();
    let a = x(rd::<f32>(n, 8)).fmul_m(c);
    let b = x(rd::<f32>(n, 4)).fmul_m(c);
    let cc = x(rd::<f32>(n, 0)).fmul_m(c);
    for (k, s) in [(0, cc), (4, b), (8, a)] {
        wr::<f32>(v1, k, f32_of(x(rd::<f32>(v1, k)).fsub(s)));
    }
    let bp = ent_pos(boat);
    for k in [0, 4, 8] {
        wr::<f32>(v1, k, f32_of(x(rd::<f32>(bp, k)).fsub_m(rd::<f32>(v1, k))));
    }
    copy3(p, v1);
    if ped_placement_find_z_coor_for_ped(p) != 0 {
        copy3(v1, p);
    }
    let bm = rd::<GPtr<u8>>(boat, 0x14);
    let bp = ent_pos(boat);
    for k in [0, 4, 8] {
        wr::<f32>(v1, k, f32_of(x(rd::<f32>(v1, k)).fsub_m(rd::<f32>(bp, k))));
    }
    wr::<u32>(m, 0x40, 0);
    wr::<u32>(m, 0x44, 0);
    matrix_invert(bm, m);
    vector_from_multiply3x3(o, m, v1);
    let ang = general_get_radian_angle_between_points(rd::<f32>(n, 0), rd::<f32>(n, 4), 0.0, 0.0);
    let lim = general_limit_radian_angle(f32_of(ang));
    let slot = i32::from(rd::<i8>(ped, 0x718));
    let w = rd::<u32>(ped, 0x5A0 + slot as isize * 0x1C);
    let dir = ftol32(lim) as u32;
    ped_attach_to_entity(ped, boat, rd::<u32>(o, 0), rd::<u32>(o, 4), rd::<u32>(o, 8), dir, 0x3E4C_CCCD, w);
    let t = task_new(0x20);
    let t = if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, 0x7D0, 0, 0, 0x4100_0000) };
    matrix_dtor(m);
    t
}

/// `CTaskComplexSeekEntityStandard::ControlSubTask(CPed*)` (0x46F800, `ret 4`; the epilogue goes
/// through the straight protection stub 0x403979). In order:
/// - a ped target of the same group standing on a boat the ped also touches (`+0x568`, then
///   `+0x584`; target `+0x40` bit 8 skips): attached peds stay; else, unless the sub task is 0xCB,
///   an abortable sub task is replaced by the attach-to-boat stand still (returned directly);
///   attached without a boat → `DettachPedFromEntity`;
/// - for a 0x38A / 0x387 sub task: no target → abort and return null; the give-up timer (+0x34,
///   +0x38, started by +0x3C, restarted by +0x3D) expired → abort, then `CTaskSimpleTired(1000)`
///   with flag 1 or null; the seek timer (+0x28) expired → restart it, recompute the seek position
///   (stack) and `d² = dx² + dy²` (stored as f32): a route (0x38A) within `(slow - [0x858624])²`
///   (`d² < q²`) aborts into a go-to-point (0x387, returned directly), else `SetTarget`; a go-to
///   (flag `+0x24` bit 4 clear) beyond `(slow + [0x858624])²` aborts into a new follow-node-route
///   (returned directly), else `GoToPoint`;
/// - a player target whose group holds the ped, with ped `+0x534` in 5..7, says 0x5B every 0x4000
///   frames (`(u16 +0x20 + CTimer::m_FrameCounter) & 0x3FFF == 0`).
#[hook(addr = 0x46F800, conv = thiscall, name = "CTaskComplexSeekEntityStandard::ControlSubTask", expect = "6A FF 68 F2 80 83 00")]
pub fn seek_entity_standard_control_sub_task_46f800(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    seek_control_sub_task(this, ped, false)
}

/// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::ControlSubTask(CPed*)` (0x665760,
/// `ret 4`; epilogue through the straight stub 0x40149E): as 0x46F800 with flags at +0x54, move
/// state at +0x50, `CreateSubTask` 0x665270 and seek position 0x665DC0; differences in the bytes:
/// the boat direction comes from a `CVector::Difference` call, the seek timer test is inlined
/// (`+0x30` started, `+0x31` restart, `now >= +0x28 + +0x2C`) instead of `CTaskTimer::IsOutOfTime`,
/// and `d² = dx² + dy²` is computed inline from the ped position (no `Difference` call, no stored
/// difference vector).
#[hook(addr = 0x665760, conv = thiscall, name = "CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::ControlSubTask", expect = "6A FF 68 42 4D 84 00")]
pub fn seek_entity_xy_control_sub_task_665760(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    seek_control_sub_task(this, ped, true)
}

/// The shared `ControlSubTask` body; `xy` selects the XYOffset instance.
fn seek_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>, xy: bool) -> GPtr<u8> {
    let (fo, mso): (isize, isize) = if xy { (0x54, 0x50) } else { (0x48, 0x44) };
    let create: fn(GPtr<u8>, i32, GPtr<u8>) -> GPtr<u8> = if xy { xy_create_sub_task } else { std_create_sub_task };
    let seek_pos: fn(GPtr<u8>, GPtr<u8>, GPtr<u8>) -> GPtr<u8> = if xy { xy_seek_pos } else { std_seek_pos };
    let mut r = rd::<GPtr<u8>>(this, 8);
    let e = rd::<GPtr<u8>>(this, 0xC);
    if !e.is_null() && rd::<u8>(e, 0x36) & 7 == 3 && ped_groups_are_in_same_group(ped, e) != 0 {
        let mut boat = GPtr::NULL;
        if rd::<u32>(e, 0x40) & 0x100 == 0 {
            match boat_under(ped, rd::<GPtr<u8>>(e, 0x568)) {
                Some(true) => boat = rd::<GPtr<u8>>(e, 0x568),
                Some(false) => {}
                None => {
                    if boat_under(ped, rd::<GPtr<u8>>(e, 0x584)) == Some(true) {
                        boat = rd::<GPtr<u8>>(e, 0x584);
                    }
                }
            }
        }
        if rd::<u32>(ped, 0xFC) != 0 {
            if boat.is_null() {
                ped_dettach_from_entity(ped);
            }
        } else if !boat.is_null()
            && task_get_task_type(rd::<GPtr<u8>>(this, 8)) != 0xCB
            && task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, 0) != 0
        {
            return attach_to_boat(ped, boat, xy);
        }
    }
    'f: {
        if task_get_task_type(rd::<GPtr<u8>>(this, 8)) != 0x38A && task_get_task_type(rd::<GPtr<u8>>(this, 8)) != 0x387 {
            break 'f;
        }
        if rd::<u32>(this, 0xC) == 0 {
            if task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, 0) != 0 {
                r = GPtr::NULL;
            }
            break 'f;
        }
        if rd::<u8>(this, 0x3C) != 0 {
            if rd::<u8>(this, 0x3D) != 0 {
                wr::<u32>(this, 0x34, g::<u32>(TIME_MS).read());
                wr::<u8>(this, 0x3D, 0);
            }
            let end = rd::<u32>(this, 0x38).wrapping_add(rd::<u32>(this, 0x34));
            if g::<u32>(TIME_MS).read() >= end {
                if task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, 0) != 0 {
                    r = if rd::<u8>(this, fo) & 1 != 0 {
                        let t = task_new(0x1C);
                        if t.is_null() { GPtr::NULL } else { tired_ctor(t, 0x3E8) }
                    } else {
                        GPtr::NULL
                    };
                }
                break 'f;
            }
        }
        if xy {
            if rd::<u8>(this, 0x30) == 0 {
                break 'f;
            }
            if rd::<u8>(this, 0x31) != 0 {
                wr::<u32>(this, 0x28, g::<u32>(TIME_MS).read());
                wr::<u8>(this, 0x31, 0);
            }
            let end = rd::<u32>(this, 0x2C).wrapping_add(rd::<u32>(this, 0x28));
            if g::<u32>(TIME_MS).read() < end {
                break 'f;
            }
        } else if task_timer_is_out_of_time(this.byte_add(0x28)) == 0 {
            break 'f;
        }
        let now = g::<u32>(TIME_MS).read();
        let iv = rd::<u32>(this, 0x14);
        let mut l = [0u32; 40];
        let lp = buf(&mut l);
        let out = lp.byte_add(0x34);
        wr::<u32>(this, 0x28, now);
        wr::<u32>(this, 0x2C, iv);
        wr::<u8>(this, 0x30, 1);
        seek_pos(this, out, ped);
        let d2 = if xy {
            let pp = ent_pos(ped);
            let dx = x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(out, 0));
            let dy = x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(out, 4));
            f32_of(dx.fmul(dx).fadd(dy.fmul(dy)))
        } else {
            let d = lp.byte_add(0x28);
            vector_difference(d, ent_pos(ped), out);
            let dx = x(rd::<f32>(d, 0)).fmul_m(rd::<f32>(d, 0));
            let dy = x(rd::<f32>(d, 4)).fmul_m(rd::<f32>(d, 4));
            wr::<u32>(d, 8, 0);
            f32_of(dx.fadd(dy))
        };
        let near = g::<f32>(0x858624).read();
        if task_get_task_type(rd::<GPtr<u8>>(this, 8)) == 0x38A {
            let s = rd::<f32>(this, 0x20);
            if x(s).fcom(gf(0x858B50)).test_parity(0x44) {
                let q = x(s).fsub_m(near);
                if !x(d2).fcom(q.fmul(q)).test_parity(5) {
                    if task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, 0) != 0 {
                        return create(this, 0x387, ped);
                    }
                    r = rd::<GPtr<u8>>(this, 8);
                    break 'f;
                }
            }
            follow_node_route_set_target(
                rd::<GPtr<u8>>(this, 8),
                ped,
                out,
                rd::<u32>(this, 0x18),
                rd::<u32>(this, 0x1C),
                rd::<u32>(this, 0x24),
                0,
            );
        } else {
            let sub = rd::<GPtr<u8>>(this, 8);
            if rd::<u8>(sub, 0x24) & 0x10 == 0 {
                let s = rd::<f32>(this, 0x20);
                let mut route = false;
                if x(s).fcom(gf(0x858B50)).test_parity(0x44) {
                    let q = x(s).fadd_m(near);
                    route = !x(d2).fcom(q.fmul(q)).test_any(0x41);
                }
                if route {
                    if task_make_abortable(sub, ped, 1, 0) != 0 {
                        let t = task_new(0x60);
                        if t.is_null() {
                            return GPtr::NULL;
                        }
                        let (a, b, c) = (rd::<u32>(this, 0x24), rd::<u32>(this, 0x1C), rd::<u32>(this, 0x18));
                        let pos = seek_pos(this, lp.byte_add(0x4C), ped);
                        return follow_node_route_ctor(t, rd::<u32>(this, mso), pos, c, b, a, 0, 0xFFFF_FFFF, 1);
                    }
                } else {
                    go_to_point_and_stand_still_go_to_point(sub, out, rd::<u32>(this, 0x18), rd::<u32>(this, 0x1C), 0);
                }
            }
        }
        r = rd::<GPtr<u8>>(this, 8);
    }
    let e = rd::<GPtr<u8>>(this, 0xC);
    if !e.is_null() && rd::<u8>(e, 0x36) & 7 == 3 && ped_is_player(e) != 0 {
        let pd = rd::<GPtr<u8>>(find_player_ped(-1), 0x480);
        let id = rd::<i32>(pd, 0x38);
        let gm = GPtr::<u8>::from_addr(id.wrapping_mul(0x2D4).wrapping_add(0xC09928) as u32 as usize);
        if membership_is_member(gm, ped) != 0 {
            let ms = rd::<u32>(ped, 0x534);
            if (5..=7).contains(&ms)
                && u32::from(rd::<u16>(ped, 0x20)).wrapping_add(g::<u32>(0xB7CB4C).read()) & 0x3FFF == 0
            {
                ped_say(ped, 0x5B, 0, 0x3F80_0000, 0, 0, 0);
            }
        }
    }
    r
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_SEEK_ENTITY_STANDARD_CTOR_46AC10,
    HOOK_SEEK_ENTITY_XY_CTOR_661DC0,
    HOOK_SEEK_ENTITY_STANDARD_CREATE_NEXT_SUB_TASK_46EFF0,
    HOOK_SEEK_ENTITY_XY_CREATE_NEXT_SUB_TASK_665080,
    HOOK_SEEK_ENTITY_STANDARD_CREATE_FIRST_SUB_TASK_46F6A0,
    HOOK_SEEK_ENTITY_XY_CREATE_FIRST_SUB_TASK_665600,
    HOOK_SEEK_ENTITY_STANDARD_CREATE_SUB_TASK_46F1E0,
    HOOK_SEEK_ENTITY_STANDARD_CONTROL_SUB_TASK_46F800,
    HOOK_SEEK_ENTITY_XY_CONTROL_SUB_TASK_665760,
];

/// `general_get_radian_angle_between_points_st0` with its ST0 result as `X87` (orig! returns stay register-sized).
fn general_get_radian_angle_between_points(x1: f32, y1: f32, x2: f32, y2: f32) -> X87 {
    X87::from_f64_exact(general_get_radian_angle_between_points_st0(x1, y1, x2, y2))
}

/// `general_limit_radian_angle_st0` with its ST0 result as `X87` (orig! returns stay register-sized).
fn general_limit_radian_angle(a: f32) -> X87 {
    X87::from_f64_exact(general_limit_radian_angle_st0(a))
}

/// This module's phase B natives (STD-8).
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_SEEK_ENTITY_XY_CREATE_SUB_TASK_665270];
