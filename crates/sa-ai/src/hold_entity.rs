//! `CTaskSimpleHoldEntity::SetPedPosition` (batch `mvpj-ai-4`). Ported from the 1.0 US bytes; unverified
//! (docs/verify/HOWTO.md).
//!
//! `CTaskSimpleHoldEntity` (from the bytes): +8 held entity, +0xC offset (CVector), +0x18 u8 bone frame index,
//! +0x19 flags (bit 4: offset in bone space), +0x1C heading, +0x35 u8 "entity required", +0x38 animation
//! association, +0x3C animation time.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type_he(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18, `ret 0xC`)
    fn v_task_make_abortable_he(t: GPtr<u8>, p: GPtr<u8>, prio: u32, e: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskSimpleHoldEntity::DropEntity(CPed*, bool)` (0x6930F0, `ret 8`)
    fn hold_entity_drop_entity(t: GPtr<u8>, p: GPtr<u8>, b: u32) = thiscall 0x6930F0 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0, `ret 4`)
    fn placeable_set_posn(e: GPtr<u8>, v: GPtr<u8>) = thiscall 0x4241C0 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990)
    fn placeable_get_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `CMatrix::UpdateMatrix(RwMatrix*)` (0x59AD20, `ret 4`)
    fn matrix_update_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD20 => unported;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70, `ret 4`)
    fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40, `ret 4`)
    fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `Multiply3x3(const CMatrix&, const CVector&)` (0x59C790; hidden result pointer first)
    fn multiply_3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    /// `CMatrix::CMatrix(const CMatrix&)` (0x59BCF0, `ret 4`)
    fn matrix_copy_ctor(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BCF0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CMatrix::RotateZ(float)` (0x59B390, `ret 4`)
    fn matrix_rotate_z(m: GPtr<u8>, a: u32) = thiscall 0x59B390 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0, `ret 4`)
    fn matrix_assign(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BBC0 => unported;
    /// `GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40)
    fn get_anim_hierarchy_from_skin_clump(c: u32) -> u32 = cdecl 0x734A40 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32)` (0x7C51A0)
    fn hanim_id_get_index(h: u32, id: u32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(RpHAnimHierarchy*)` (0x7C5120)
    fn hanim_hierarchy_get_matrix_array(h: u32) -> GPtr<u8> = cdecl 0x7C5120 => unported;
    /// `RwV3dTransformPoints(RwV3d* out, const RwV3d* in, int32 n, const RwMatrix*)` (0x7EDD90)
    fn rw_v3d_transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
}

fn vp(v: &mut [f32; 3]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

/// The matrix of the ped's bone frame `this+0x18` (`RpHAnimHierarchyGetMatrixArray` + index * 0x40).
fn bone_matrix(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let h = get_anim_hierarchy_from_skin_clump(rd::<u32>(ped, 0x18));
    let frame = rd::<GPtr<u8>>(ped, 0x488 + isize::from(rd::<u8>(this, 0x18)) * 4);
    let idx = hanim_id_get_index(h, rd::<u32>(frame, 0x14));
    hanim_hierarchy_get_matrix_array(h).byte_add(idx as isize * 0x40)
}

/// `CTaskSimpleHoldEntity::SetPedPosition(CPed*)` (0x6940A0, thiscall, `ret 4`). Pick-up task (type 0x134):
/// without an association (+0x38), or with the time +0x3C above its current time (+0x20) → AL = 0. Put-down
/// task (0x135) with +0x35 clear and no association or +0x3C below its time: `DropEntity(ped, false)` when an
/// entity is held, AL = 0. No held entity → `MakeAbortable(ped, 1, NULL)`, AL = 0. Else the entity's flag
/// +0x1C bit 7 = the ped's; ped +0x474 bit 10 clear → the entity at the ped's position. Else bone-space offset
/// (+0x19 bit 4): `RwV3dTransformPoints` of the offset by the bone matrix, the entity matrix
/// `UpdateMatrix(bone)`, position = that point. Else: R = `Multiply3x3(ped matrix, offset)`, P = the bone
/// matrix position, R = f32(P + R) per axis; a copy of the ped matrix `RotateZ(+0x1C)` becomes the entity
/// matrix, position R. Then the entity's RW matrix is updated (`CMatrix::UpdateRwMatrix`, or
/// `CSimpleTransform::UpdateRwMatrix` without a matrix) when it has an RW object, `UpdateRwFrame`, AL = 1.
/// EAX's upper bytes are left from the FPU status or earlier calls in the original (not reproduced).
#[hook(addr = 0x6940a0, conv = thiscall, name = "CTaskSimpleHoldEntity::SetPedPosition", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "mvpj-ai-4", route = "L2", hazards = "ax-al-return,heap-writes", recs = "grove-walk",
       note = "writes the held entity's matrix and RW frame (heap)")]
pub fn hold_entity_set_ped_position(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let ty = v_task_type_he(this);
    if ty == 0x134 {
        let a = rd::<GPtr<u8>>(this, 0x38);
        if a.is_null() || !x(rd::<f32>(this, 0x3C)).fcom(x(rd::<f32>(a, 0x20))).test_any(0x41) {
            return 0;
        }
    } else if v_task_type_he(this) == 0x135 && rd::<u8>(this, 0x35) == 0 {
        let a = rd::<GPtr<u8>>(this, 0x38);
        if a.is_null() || !x(rd::<f32>(this, 0x3C)).fcom(x(rd::<f32>(a, 0x20))).test_parity(0x05) {
            if !rd::<GPtr<u8>>(this, 8).is_null() {
                hold_entity_drop_entity(this, ped, 0);
            }
            return 0;
        }
    }
    let e = rd::<GPtr<u8>>(this, 8);
    if e.is_null() {
        v_task_make_abortable_he(this, ped, 1, 0);
        return 0;
    }
    let ef = rd::<u32>(e, 0x1C);
    wr::<u32>(e, 0x1C, ef ^ ((rd::<u32>(ped, 0x1C) ^ ef) & 0x80));
    if rd::<u32>(ped, 0x474) & 0x400 == 0 {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let p = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
        placeable_set_posn(rd::<GPtr<u8>>(this, 8), p);
    } else if rd::<u8>(this, 0x19) & 0x10 != 0 {
        let mut v = [rd::<f32>(this, 0xC), rd::<f32>(this, 0x10), rd::<f32>(this, 0x14)];
        let bm = bone_matrix(this, ped);
        rw_v3d_transform_points(vp(&mut v), vp(&mut v), 1, bm);
        matrix_update_matrix(placeable_get_matrix(rd::<GPtr<u8>>(this, 8)), bm);
        placeable_set_posn(rd::<GPtr<u8>>(this, 8), vp(&mut v));
    } else {
        let mut off = [rd::<f32>(this, 0xC), rd::<f32>(this, 0x10), rd::<f32>(this, 0x14)];
        let mut tmp = [0f32; 3];
        let r = multiply_3x3(vp(&mut tmp), rd::<GPtr<u8>>(ped, 0x14), vp(&mut off));
        let mut rv = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
        let bp = bone_matrix(this, ped).byte_add(0x30);
        let pm = rd::<GPtr<u8>>(ped, 0x14);
        for (i, v) in rv.iter_mut().enumerate() {
            *v = x(rd::<f32>(bp, i as isize * 4)).fadd_m(*v).to_f32();
        }
        let mut mat = [0u32; 0x48 / 4];
        let mp: GPtr<u8> = GPtr::from_raw(mat.as_mut_ptr()).cast();
        matrix_copy_ctor(mp, pm);
        matrix_rotate_z(mp, rd::<u32>(this, 0x1C));
        matrix_assign(placeable_get_matrix(rd::<GPtr<u8>>(this, 8)), mp);
        placeable_set_posn(rd::<GPtr<u8>>(this, 8), vp(&mut rv));
        matrix_dtor(mp);
    }
    let e = rd::<GPtr<u8>>(this, 8);
    let rw = rd::<GPtr<u8>>(e, 0x18);
    if !rw.is_null() {
        let frame_m = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let m = rd::<GPtr<u8>>(e, 0x14);
        if m.is_null() {
            simple_transform_update_rw_matrix(e.byte_add(4), frame_m);
        } else {
            matrix_update_rw_matrix(m, frame_m);
        }
    }
    entity_update_rw_frame(rd::<GPtr<u8>>(this, 8));
    1
}

pub const HOOKS: &[HookReg] = &[HOOK_HOLD_ENTITY_SET_PED_POSITION];
