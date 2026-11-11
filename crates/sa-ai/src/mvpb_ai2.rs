//! WRITER-MVP-B (helperB) AI ports, part 2: `CTaskComplexFacial` / `CTaskSimpleFacial`, `CTaskSimpleHoldEntity`
//! (constructor, `DropEntity`, `StartAnim`, the finish callback), `CPedGroupIntelligence` helpers, the inform
//! event queues and small destructors. Ported from the 1.0 US bytes (the exe wins over gta-reversed). SEH frame
//! set-up/tear-down (including the no-CD chunks some prologues jump through) is not modelled.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;

orig! {
    /// `CPed::IsAlive` (0x5E0170)
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CTaskSimple::CTaskSimple` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CTaskSimplePause::CTaskSimplePause(int32)` (0x48E750, `ret 4`)
    fn task_simple_pause_ctor(t: GPtr<u8>, ms: i32) -> GPtr<u8> = thiscall 0x48E750 => unported;
    /// `CTaskSimpleFacial::CTaskSimpleFacial(int32 type, int32 duration)` (0x690C70, `ret 8`)
    fn task_simple_facial_ctor(t: GPtr<u8>, ty: i32, ms: i32) -> GPtr<u8> = thiscall 0x690C70 => unported;
    /// `CTaskSimpleFacial::GetAnimId(int32)` (0x690CC0, cdecl)
    fn facial_get_anim_id(ty: i32) -> i32 = cdecl 0x690CC0 => crate::mvpb_ai2::task_simple_facial_get_anim_id;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, uint32 animId)` (0x4D68B0, cdecl)
    fn clump_get_association(c: GPtr<u8>, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, CAnimBlendHierarchy*, int32 flags, float)` (0x4D4410, cdecl)
    fn blend_animation_hier(c: GPtr<u8>, h: GPtr<u8>, flags: u32, d: f32) -> GPtr<u8> = cdecl 0x4D4410 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610, cdecl)
    fn blend_animation(c: GPtr<u8>, g: u32, a: u32, d: f32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimManager::GetAnimBlockName(AssocGroupId)` (0x4D3A30, cdecl)
    fn get_anim_block_name(g: u32) -> GPtr<u8> = cdecl 0x4D3A30 => unported;
    /// `CAnimManager::GetAnimationBlock(const char*)` (0x4D3940, cdecl)
    fn get_animation_block(n: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D3940 => unported;
    /// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0, cdecl)
    fn add_anim_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    /// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990, cdecl)
    fn get_animation_block_index(n: u32) -> i32 = cdecl 0x4D3990 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0, cdecl)
    fn remove_anim_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0, cdecl)
    fn request_model(id: i32, flags: u32) = cdecl 0x4087E0 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(cb, data)` (0x4CEBE0, `ret 8`)
    fn assoc_set_finish_callback(a: GPtr<u8>, cb: u32, d: GPtr<u8>) = thiscall 0x4CEBE0 => unported;
    /// `CAnimBlendAssociation::SetDeleteCallback(cb, data)` (0x4CEBC0, `ret 8`)
    fn assoc_set_delete_callback(a: GPtr<u8>, cb: u32, d: GPtr<u8>) = thiscall 0x4CEBC0 => unported;
    /// `CPhysical::AddToMovingList` (0x542800)
    fn add_to_moving_list(e: GPtr<u8>) = thiscall 0x542800 => unported;
    /// `CPhysical::RemoveFromMovingList` (0x542860)
    fn remove_from_moving_list(e: GPtr<u8>) = thiscall 0x542860 => unported;
    /// `CEventSoundQuiet::CEventSoundQuiet(CEntity*, float, uint32, const CVector&)` (0x5E05B0, `ret 0x10`)
    fn event_sound_quiet_ctor(e: GPtr<u8>, ent: GPtr<u8>, vol: f32, t: u32, pos: GPtr<f32>) -> GPtr<u8> = thiscall 0x5E05B0 => unported;
    /// `GetEventGlobalGroup()` (0x4ABA50, cdecl)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `FUN_005dea00` (0x5DEA00): `CEventSoundQuiet` destructor body
    fn event_sound_quiet_dtor(e: GPtr<u8>) = thiscall 0x5DEA00 => crate::mvpb_ai::event_dtor_5dea00;
    /// `CEntity::GetColModel` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0, `ret 4`)
    fn placeable_set_posn(e: GPtr<u8>, v: GPtr<f32>) = thiscall 0x4241C0 => unported;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70, `ret 4`)
    fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40, `ret 4`)
    fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// `CEntity::UpdateRwFrame` (0x532B00)
    fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CPedGroupMembership::IsMember(const CPed*)` (0x5F6A10, `ret 4`)
    fn group_membership_is_member(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5F6A10 => unported;
    /// `CGroupEventHandler::ComputeEventResponseTasks(const CPedGroupEvent*... )` (0x5FC200, cdecl, 2 args)
    fn group_event_handler_compute(a: GPtr<u8>, b: GPtr<u8>) = cdecl 0x5FC200 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80, cdecl)
    fn get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroups::GetGroupId(CPedGroup*)` (0x5F7EE0, cdecl)
    fn get_group_id(g: GPtr<u8>) -> i32 = cdecl 0x5F7EE0 => unported;
    /// `CEventGroupEvent::CEventGroupEvent(CPed*, CEvent*)` (0x4ADFD0, `ret 8`)
    fn event_group_event_ctor(e: GPtr<u8>, ped: GPtr<u8>, ev: GPtr<u8>) -> GPtr<u8> = thiscall 0x4ADFD0 => unported;
    /// `CEventGroupEvent::~CEventGroupEvent` (0x4AE070)
    fn event_group_event_dtor(e: GPtr<u8>) = thiscall 0x4AE070 => unported;
    /// `CPedGroupIntelligence::AddEvent(CEvent*)` (0x5F7470, `ret 4`)
    fn pgi_add_event(g: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x5F7470 => crate::mvpb_ai::pgi_add_event;
    /// `FUN_0061a3d0` (0x61A3D0): `CTaskComplex` base destructor
    fn task_complex_dtor(t: GPtr<u8>) = thiscall 0x61A3D0 => crate::mvpb_ai::task_complex_dtor_61a3d0;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: u32, e: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CEvent::GetEventType` (vtable +0x4)
    fn v_get_event_type(e: GPtr<u8>) -> u32 = thiscall vslot 0x4 => unported;
    /// `CEvent::Clone` (vtable +0x10)
    fn v_event_clone(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x10 => unported;
    /// `CEvent::GetSourceEntity` (vtable +0x28)
    fn v_get_source_entity(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x28 => unported;
    /// `CEntity::SetIsStatic(bool)` (vtable +0x10)
    fn v_set_is_static(e: GPtr<u8>, b: u32) = thiscall vslot 0x10 => unported;
    /// `CPhysical::...` (vtable +0x34, one argument, returns bool)
    fn v_entity_34(e: GPtr<u8>, a: u32) -> u8 = thiscall vslot 0x34 => unported;
    /// scalar deleting destructor (vtable +0)
    fn v_delete(t: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
}

fn now() -> u32 {
    rd::<u32>(GPtr::from_addr(TIME_MS), 0)
}

/// The inline 0x18-byte simple task both facial tasks create on death (vtable 0x85A278, +0x14 = 5000).
fn new_inline_pause() -> GPtr<u8> {
    let t = task_new(0x18);
    if t.is_null() {
        return GPtr::NULL;
    }
    task_simple_ctor(t);
    wr::<u32>(t, 0, 0x85A278);
    wr::<u32>(t, 8, 0);
    wr::<u32>(t, 0xC, 0);
    wr::<u8>(t, 0x10, 0);
    wr::<u8>(t, 0x11, 0);
    wr::<u32>(t, 0x14, 5000);
    t
}

fn new_pause_5000() -> GPtr<u8> {
    let t = task_new(0x18);
    if t.is_null() { GPtr::NULL } else { task_simple_pause_ctor(t, 5000) }
}

/// `CTaskComplexFacial::MakeAbortable(CPed*, int32 prio, const CEvent*)` (0x690DA0, `ret 0xC`): only for priority 2
/// (urgent), the sub-task's `MakeAbortable(ped, 2, event)` (tail jump); otherwise AL = 0.
#[hook(addr = 0x690DA0, conv = thiscall, name = "CTaskComplexFacial::MakeAbortable", this = "CTaskComplexFacial",
       expect = "83 7C 24 08 02", unverified = "mvpb-ai-2", route = "L2", hazards = "none")]
pub fn task_complex_facial_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, prio: u32, e: u32) -> u8 {
    if prio != 2 {
        return 0;
    }
    v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 2, e)
}

/// `CTaskSimpleFacial::MakeAbortable(CPed*, int32, const CEvent*)` (0x692E50, `ret 0xC`): the running facial
/// animation (`GetAnimId(+0x14)` in the ped's clump) gets blend delta -4; AL = 1 always.
#[hook(addr = 0x692E50, conv = thiscall, name = "CTaskSimpleFacial::MakeAbortable", this = "CTaskSimpleFacial",
       expect = "8B 41 14 50 E8 67 DE FF FF", unverified = "mvpb-ai-2", route = "L2", hazards = "heap-writes")]
pub fn task_simple_facial_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, _prio: u32, _e: u32) -> u8 {
    let id = facial_get_anim_id(rd::<i32>(this, 0x14));
    let a = clump_get_association(rd::<GPtr<u8>>(ped, 0x18), id as u32);
    if !a.is_null() {
        wr::<u32>(a, 0x1C, 0xC080_0000);
    }
    1
}

/// `CTaskSimpleFacial::GetAnimId(int32 type)` (0x690CC0, cdecl): 0..8 -> 0x9A, 0x9B, 0x9C, 0x9D, 0x9C, 0x9D, -1,
/// 0xA0, 0xA1 (jump table 0x690CF8); anything else (unsigned > 8) -1.
#[hook(addr = 0x690CC0, conv = cdecl, name = "CTaskSimpleFacial::GetAnimId", expect = "8B 44 24 04 83 F8 08",
       unverified = "mvpb-ai-2", route = "L2", hazards = "none")]
pub fn task_simple_facial_get_anim_id(ty: i32) -> i32 {
    match ty {
        0 => 0x9A,
        1 => 0x9B,
        2 | 4 => 0x9C,
        3 | 5 => 0x9D,
        7 => 0xA0,
        8 => 0xA1,
        _ => -1,
    }
}

/// `CTaskComplexFacial::CreateNextSubTask(CPed*)` (0x690DC0, `ret 4`): a dead ped whose sub-task aborts gets the
/// inline 5 s simple task; else a pending requested facial (+0x18 != -1; cleared first) becomes
/// `CTaskSimpleFacial` (0x1C bytes inline: vtable 0x870AB8, +0x14 type, +0x18 = +0x1C duration); else after a
/// `CTaskSimpleFacial` sub-task a 5 s pause; else when talking (+0xC) a facial of type 8 for 5 s; else a 5 s pause.
#[hook(addr = 0x690DC0, conv = thiscall, name = "CTaskComplexFacial::CreateNextSubTask", this = "CTaskComplexFacial",
       expect = "64 A1 00 00 00 00", unverified = "mvpb-ai-2", route = "L2", hazards = "heap-writes")]
pub fn task_complex_facial_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if ped_is_alive(ped) == 0 && v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 2, 0) != 0 {
        return new_inline_pause();
    }
    let req = rd::<u32>(this, 0x18);
    if req != 0xFFFF_FFFF {
        wr::<u32>(this, 0x18, 0xFFFF_FFFF);
        let t = task_new(0x1C);
        if t.is_null() {
            return GPtr::NULL;
        }
        let dur = rd::<u32>(this, 0x1C);
        task_simple_ctor(t);
        wr::<u32>(t, 0, 0x870AB8);
        wr::<u32>(t, 8, 0);
        wr::<u32>(t, 0xC, 0);
        wr::<u8>(t, 0x10, 0);
        wr::<u8>(t, 0x11, 0);
        wr::<u32>(t, 0x18, dur);
        wr::<u32>(t, 0x14, req);
        return t;
    }
    if v_get_task_type(rd::<GPtr<u8>>(this, 8)) == 0x12F {
        return new_pause_5000();
    }
    if rd::<u8>(this, 0xC) != 0 {
        let t = task_new(0x1C);
        if t.is_null() {
            return GPtr::NULL;
        }
        return task_simple_facial_ctor(t, 8, 5000);
    }
    new_pause_5000()
}

/// `CTaskComplexFacial::CreateFirstSubTask(CPed*)` (0x690F30, `ret 4`): the player never talks (+0xC = 0); returns
/// the inline 5 s simple task (vtable 0x85A278).
#[hook(addr = 0x690F30, conv = thiscall, name = "CTaskComplexFacial::CreateFirstSubTask", this = "CTaskComplexFacial",
       expect = "64 A1 00 00 00 00", unverified = "mvpb-ai-2", route = "L2", hazards = "heap-writes")]
pub fn task_complex_facial_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if ped_is_player(ped) != 0 {
        wr::<u8>(this, 0xC, 0);
    }
    new_inline_pause()
}

/// `CTaskComplexFacial::ControlSubTask(CPed*)` (0x690FC0, `ret 4`): dead + abortable -> inline 5 s task; stop
/// request (+0xE) + abortable (prio 1) -> clear +0xE/+0xD, 5 s pause; while the chat animation 0xC plays: +0x18 =
/// -1, +0xD = 1, and unless anim 0xA0 also plays a new facial (7, 0); talking stopped (+0xD) + abortable -> 5 s
/// pause; a requested facial (+0x10): when the current sub-task is that facial it is restarted in place (+0x10 = 1,
/// +8 = now, +0xC = +0x14), else a new `CTaskSimpleFacial(+0x10, +0x14)`; +0x10 = -1 either way. Otherwise the
/// current sub-task.
#[hook(addr = 0x690FC0, conv = thiscall, name = "CTaskComplexFacial::ControlSubTask", this = "CTaskComplexFacial",
       expect = "64 A1 00 00 00 00", unverified = "mvpb-ai-2", route = "L2", hazards = "heap-writes")]
pub fn task_complex_facial_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if ped_is_alive(ped) == 0 && v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 2, 0) != 0 {
        return new_inline_pause();
    }
    if rd::<u8>(this, 0xE) != 0 && v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, 0) != 0 {
        wr::<u8>(this, 0xE, 0);
        wr::<u8>(this, 0xD, 0);
        return new_pause_5000();
    }
    if !clump_get_association(rd::<GPtr<u8>>(ped, 0x18), 0xC).is_null() {
        let a = clump_get_association(rd::<GPtr<u8>>(ped, 0x18), 0xA0);
        wr::<u32>(this, 0x18, 0xFFFF_FFFF);
        wr::<u8>(this, 0xD, 1);
        if !a.is_null() {
            return rd::<GPtr<u8>>(this, 8);
        }
        let t = task_new(0x1C);
        if t.is_null() {
            return GPtr::NULL;
        }
        return task_simple_facial_ctor(t, 7, 0);
    }
    if rd::<u8>(this, 0xD) != 0 && v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, 0) != 0 {
        wr::<u8>(this, 0xD, 0);
        return new_pause_5000();
    }
    if rd::<u32>(this, 0x10) == 0xFFFF_FFFF {
        return rd::<GPtr<u8>>(this, 8);
    }
    let mut create = true;
    if v_get_task_type(rd::<GPtr<u8>>(this, 8)) == 0x12F {
        let sub = rd::<GPtr<u8>>(this, 8);
        if rd::<u32>(sub, 0x14) == rd::<u32>(this, 0x10) {
            let d = rd::<u32>(this, 0x14);
            wr::<u8>(sub, 0x10, 1);
            wr::<u32>(sub, 8, now());
            wr::<u32>(sub, 0xC, d);
            create = false;
        }
    }
    let ty = rd::<i32>(this, 0x10);
    wr::<u32>(this, 0x10, 0xFFFF_FFFF);
    if !create {
        return rd::<GPtr<u8>>(this, 8);
    }
    let t = task_new(0x1C);
    if t.is_null() {
        return GPtr::NULL;
    }
    task_simple_facial_ctor(t, ty, rd::<i32>(this, 0x14))
}

/// `CTaskSimpleHoldEntity::CTaskSimpleHoldEntity(CEntity*, const CVector*, uint8 boneFrameId, uint8 boneFlags,
/// AnimationId, AssocGroupId, bool disallowDroppingOnAnimEnd)` (0x6913A0, `ret 0x1C`): `CTaskSimple` base, vtable
/// 0x870B2C, the fields from the arguments (+8 entity, +0xC..+0x14 position or 0, +0x18/+0x19 bone bytes, +0x20 anim,
/// +0x24 group, +0x36 flag), +0x2C = +0x30 = +0x38 = 0, +0x34 = 0, +0x35 = 1; a held entity gets flag 0x400 in +0x1C
/// and a registered reference. Returns this.
#[hook(addr = 0x6913A0, conv = thiscall, name = "CTaskSimpleHoldEntity::Constructor-1", this = "CTaskSimpleHoldEntity",
       expect = "6A FF E9 AC 28 D7 FF", unverified = "mvpb-ai-2", route = "L2", hazards = "heap-writes,entry-protection-read")]
#[allow(clippy::too_many_arguments)]
pub fn hold_entity_ctor(
    this: GPtr<u8>,
    ent: GPtr<u8>,
    pos: GPtr<u32>,
    bone: u8,
    bone_flags: u8,
    anim: u32,
    group: u32,
    no_drop: u8,
) -> GPtr<u8> {
    task_simple_ctor(this);
    wr(this, 8, ent);
    wr::<u32>(this, 0, 0x870B2C);
    wr::<u32>(this, 0xC, 0);
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0x14, 0);
    wr::<u8>(this, 0x18, bone);
    wr::<u8>(this, 0x19, bone_flags);
    wr::<u32>(this, 0x20, anim);
    wr::<u32>(this, 0x24, group);
    wr::<u8>(this, 0x36, no_drop);
    wr::<u32>(this, 0x2C, 0);
    wr::<u32>(this, 0x30, 0);
    wr::<u8>(this, 0x34, 0);
    wr::<u8>(this, 0x35, 1);
    wr::<u32>(this, 0x38, 0);
    if !pos.is_null() {
        wr::<u32>(this, 0xC, pos.read());
        wr::<u32>(this, 0x10, pos.wrapping_index(1).read());
        wr::<u32>(this, 0x14, pos.wrapping_index(2).read());
    }
    let e = rd::<GPtr<u8>>(this, 8);
    if !e.is_null() {
        wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) | 0x400);
        register_reference(rd::<GPtr<u8>>(this, 8), this.byte_add(8));
    }
    this
}

/// `CTaskSimpleHoldEntity::DropEntity(CPed*, bool addEvent)` (0x6930F0, `ret 8`): see the original for the full flow;
/// for an object (type 4): ignored-collision entity = ped; an object that is already static-capable (+0x40 & 4) and
/// dropped with `addEvent` becomes a temp object (unless flag 0x2000 in +0x140 or object type 2/6: then it is put
/// back to its place); otherwise it loses +0x40 bit 0x2000000 and, when `addEvent`, gets the ped's move speed plus
/// two random throws along the ped's forward (0.03..0.06) and right (-0.03..0.03) vectors and 0.01 x up, then vslot
/// 0x34 decides whether it is put back; without `addEvent` its speed is cleared. Objects with flag 0x2000 dropped by
/// a ped with +0x480 (with `addEvent`) raise a `CEventSoundQuiet(ped, 60, -1, 0)` in the global group. A put-back
/// object is made static, removed from the moving list, placed at the ped's position with z = z - 1 - colmodel min
/// z, and its RW matrix/frame updated. Finally the held-entity reference is released. Kept original detail: both
/// `rand()` results are stored into the caller's first argument slot (not modelled).
#[hook(addr = 0x6930F0, conv = thiscall, name = "CTaskSimpleHoldEntity::DropEntity", this = "CTaskSimpleHoldEntity",
       expect = "6A FF E9 EE 41 D7 FF", unverified = "mvpb-ai-2", route = "L2", hazards = "heap-writes",
       note = "rand() at 0x6931F2/0x693235; the original writes the rand values into its own (callee-popped) ped argument slot")]
pub fn hold_entity_drop_entity(this: GPtr<u8>, ped: GPtr<u8>, add_event: u8) {
    let slot = this.byte_add(8);
    let e0 = rd::<GPtr<u8>>(this, 8);
    if e0.is_null() {
        return;
    }
    wr::<u32>(e0, 0x1C, rd::<u32>(e0, 0x1C) | 1);
    let ent = rd::<GPtr<u8>>(this, 8);
    let mut put_back = false;
    if rd::<u8>(ent, 0x36) & 7 == 4 {
        let f40 = rd::<u32>(ent, 0x40);
        wr(ent, 0x128, ped);
        if f40 & 4 != 0 && add_event != 0 {
            let ot = rd::<u8>(ent, 0x13C);
            if rd::<u32>(ent, 0x140) & 0x2000 != 0 || ot == 2 || ot == 6 {
                put_back = true;
            } else {
                if ot != 3 {
                    let n: GPtr<u16> = GPtr::from_addr(0xBB4A70);
                    n.write(n.read().wrapping_add(1));
                }
                let f = rd::<u32>(ent, 0x1C) & 0xFFFF_FF7E;
                wr::<u8>(ent, 0x13C, 3);
                wr::<u32>(ent, 0x150, 0);
                wr::<u32>(ent, 0x1C, f);
            }
        } else {
            wr::<u32>(ent, 0x40, f40 & 0xFDFF_FFFF);
            if add_event != 0 {
                let f = rd::<u32>(ent, 0x1C);
                if f & 4 != 0 || f & 0x40000 != 0 {
                    v_set_is_static(ent, 0);
                    add_to_moving_list(ent);
                }
                for k in 0..3isize {
                    wr::<u32>(ent, 0x44 + 4 * k, rd::<u32>(ped, 0x44 + 4 * k));
                }
                let s = |k: isize| rd::<f32>(ent, 0x44 + 4 * k);
                let m = rd::<GPtr<u8>>(ped, 0x14);
                let r = sa_core::orig::at_site(0x6931F2, rand);
                let t = xi(r).fmul_m(3.051_851e-5).fmul_m(0.03).fadd_m(0.03);
                let a = t.fmul_m(rd::<f32>(m, 0x18));
                let b = t.fmul_m(rd::<f32>(m, 0x14));
                wr::<f32>(ent, 0x44, f32_of(t.fmul_m(rd::<f32>(m, 0x10)).fadd_m(s(0))));
                wr::<f32>(ent, 0x48, f32_of(b.fadd_m(s(1))));
                wr::<f32>(ent, 0x4C, f32_of(a.fadd_m(s(2))));
                let m = rd::<GPtr<u8>>(ped, 0x14);
                let r = sa_core::orig::at_site(0x693235, rand);
                let t = xi(r).fmul_m(3.051_851e-5).fmul_m(0.06).fsub_m(0.03);
                let a = t.fmul_m(rd::<f32>(m, 8));
                let b = t.fmul_m(rd::<f32>(m, 4));
                wr::<f32>(ent, 0x44, f32_of(t.fmul_m(rd::<f32>(m, 0)).fadd_m(s(0))));
                wr::<f32>(ent, 0x48, f32_of(b.fadd_m(s(1))));
                wr::<f32>(ent, 0x4C, f32_of(a.fadd_m(s(2))));
                let m = rd::<GPtr<u8>>(ped, 0x14);
                let z = x(rd::<f32>(m, 0x28)).fmul_m(0.01);
                let y = x(rd::<f32>(m, 0x24)).fmul_m(0.01);
                let xx = x(rd::<f32>(m, 0x20)).fmul_m(0.01);
                wr::<f32>(ent, 0x44, f32_of(xx.fadd_m(s(0))));
                wr::<f32>(ent, 0x48, f32_of(y.fadd_m(s(1))));
                wr::<f32>(ent, 0x4C, f32_of(z.fadd_m(s(2))));
                if v_entity_34(ent, 0) != 0 {
                    put_back = true;
                }
            } else {
                wr::<u32>(ent, 0x44, 0);
                wr::<u32>(ent, 0x48, 0);
                wr::<u32>(ent, 0x4C, 0);
            }
        }
        if rd::<u32>(ent, 0x140) & 0x2000 != 0 && rd::<u32>(ped, 0x480) != 0 && add_event != 0 {
            let mut v = [0.0f32; 3];
            let mut ev = [0u32; 16];
            let evp: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr().cast());
            event_sound_quiet_ctor(evp, ped, 60.0, 0xFFFF_FFFF, GPtr::from_raw(v.as_mut_ptr()));
            let g = get_event_global_group();
            event_group_add(g, evp, 0);
            event_sound_quiet_dtor(evp);
        }
        if put_back {
            wr::<u32>(ent, 0x44, 0);
            wr::<u32>(ent, 0x48, 0);
            wr::<u32>(ent, 0x4C, 0);
            v_set_is_static(ent, 1);
            remove_from_moving_list(ent);
            let m = rd::<GPtr<u8>>(ped, 0x14);
            let p = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
            let mut v = [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)];
            let cm = entity_get_col_model(ent);
            v[2] = f32_of(x(v[2]).fsub_m(1.0).fsub_m(rd::<f32>(cm, 8)));
            placeable_set_posn(ent, GPtr::from_raw(v.as_mut_ptr()));
            let rw = rd::<GPtr<u8>>(ent, 0x18);
            if !rw.is_null() {
                let fm = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
                let mm = rd::<GPtr<u8>>(ent, 0x14);
                if mm.is_null() {
                    simple_transform_update_rw_matrix(ent.byte_add(4), fm);
                } else {
                    matrix_update_rw_matrix(mm, fm);
                }
            }
            entity_update_rw_frame(ent);
        }
    }
    clean_up_old_reference(rd::<GPtr<u8>>(this, 8), slot);
    wr::<u32>(this, 8, 0);
}

/// `CTaskSimpleHoldEntity::StartAnim(CPed*)` (0x692FF0, `ret 4`): with a hierarchy (+0x30) blends it with flags
/// +0x28 | 0x414 (stored back); else, with an anim group (+0x24) whose block is not referenced yet (+0x2C), finds
/// the group's anim block (by name when the group has none), requests it (model 0x63E7 + index, flags 8) and returns
/// when not loaded, else adds a block reference and stores it; then blends (group, anim +0x20, 4.0), sets flag 4 in
/// assoc +0x2E (and +0x2F for task type 0x133). The association (+0x38) gets `FinishAnimHoldEntityCB` as finish
/// callback for task type 0x134, else as delete callback.
#[hook(addr = 0x692FF0, conv = thiscall, name = "CTaskSimpleHoldEntity::StartAnim", this = "CTaskSimpleHoldEntity",
       expect = "56 8B F1 8B 46 30", unverified = "mvpb-ai-2", route = "L2", hazards = "heap-writes")]
pub fn hold_entity_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let hier = rd::<GPtr<u8>>(this, 0x30);
    if !hier.is_null() {
        let f = rd::<u32>(this, 0x28) | 0x414;
        wr::<u32>(this, 0x28, f);
        let a = blend_animation_hier(rd::<GPtr<u8>>(ped, 0x18), hier, f, 4.0);
        wr(this, 0x38, a);
    } else {
        let g = rd::<u32>(this, 0x24);
        if g != 0 && rd::<u32>(this, 0x2C) == 0 {
            let groups = rd::<GPtr<u8>>(GPtr::from_addr(0xB4EA34), 0);
            let mut block = rd::<GPtr<u8>>(groups, (g.wrapping_mul(0x14)) as i32 as isize);
            if block.is_null() {
                block = get_animation_block(get_anim_block_name(g));
            }
            let idx = ((block.addr() as u32).wrapping_sub(0xB5D4A0) as i32) >> 5;
            if rd::<u8>(block, 0x10) == 0 {
                request_model(idx.wrapping_add(0x63E7), 8);
                return;
            }
            add_anim_block_ref(idx);
            wr(this, 0x2C, block);
        }
        let a = blend_animation(rd::<GPtr<u8>>(ped, 0x18), rd::<u32>(this, 0x24), rd::<u32>(this, 0x20), 4.0);
        wr(this, 0x38, a);
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 4);
        if v_get_task_type(this) == 0x133 {
            let a = rd::<GPtr<u8>>(this, 0x38);
            wr::<u8>(a, 0x2F, rd::<u8>(a, 0x2F) | 4);
        }
    }
    let t = v_get_task_type(this);
    let a = rd::<GPtr<u8>>(this, 0x38);
    if t == 0x134 {
        assoc_set_finish_callback(a, 0x691740, this);
    } else {
        assoc_set_delete_callback(a, 0x691740, this);
    }
}

/// `CTaskSimpleHoldEntity::FinishAnimHoldEntityCB(CAnimBlendAssociation*, void* task)` (0x691740, cdecl): with
/// "disallow dropping" (+0x36) the anim becomes 0xBF / group 0 and +0x38 = 0; otherwise a put-down task (type 0x134)
/// whose association still has blend amount > 0 releases the held entity; then +0x34 = 1 (finished), +0x38 = 0.
#[hook(addr = 0x691740, conv = cdecl, name = "CTaskSimpleHoldEntity::FinishAnimHoldEntityCB",
       expect = "56 8B 74 24 0C", unverified = "mvpb-ai-2", route = "L2", hazards = "heap-writes")]
pub fn hold_entity_finish_anim_cb(assoc: GPtr<u8>, t: GPtr<u8>) {
    if rd::<u8>(t, 0x36) != 0 {
        wr::<u32>(t, 0x20, 0xBF);
        wr::<u32>(t, 0x24, 0);
        wr::<u32>(t, 0x38, 0);
        return;
    }
    if v_get_task_type(t) == 0x134 && x(rd::<f32>(assoc, 0x18)).fcom(X87::zero()) == sa_math::x87::Cmp::Greater {
        let e = rd::<GPtr<u8>>(t, 8);
        if !e.is_null() {
            clean_up_old_reference(e, t.byte_add(8));
            wr::<u32>(t, 8, 0);
        }
    }
    wr::<u8>(t, 0x34, 1);
    wr::<u32>(t, 0x38, 0);
}

/// `CPedGroupIntelligence::IsGroupResponding` (0x5F7760): AL = 1 when any of the 8 ped task pairs has both ped and
/// task set (+0xC/+0x10 at 0x14 stride) or any of the 8 secondary pairs (+0xAC/+0xB0).
#[hook(addr = 0x5F7760, conv = thiscall, name = "CPedGroupIntelligence::IsGroupResponding", this = "CPedGroupIntelligence",
       expect = "32 C0 33 D2 83 C1 10", unverified = "mvpb-ai-2", route = "L2", hazards = "none")]
pub fn pgi_is_group_responding(this: GPtr<u8>) -> u8 {
    for i in 0..8isize {
        let e = this.byte_add(0x10 + 0x14 * i);
        if rd::<u32>(e, -4) != 0 && rd::<u32>(e, 0) != 0 {
            return 1;
        }
        if rd::<u32>(e, 0x9C) != 0 && rd::<u32>(e, 0xA0) != 0 {
            return 1;
        }
    }
    0
}

/// `CPedGroupIntelligence::IsCurrentEventValid` (0x5F77A0): AL = 0 only when the current event (+4) wraps an event
/// of type 0x47 whose source entity is a ped that is a member of the group; else 1.
#[hook(addr = 0x5F77A0, conv = thiscall, name = "CPedGroupIntelligence::IsCurrentEventValid", this = "CPedGroupIntelligence",
       expect = "56 57 8B F9 8B 47 04", unverified = "mvpb-ai-2", route = "L2", hazards = "none")]
pub fn pgi_is_current_event_valid(this: GPtr<u8>) -> u8 {
    let ev = rd::<GPtr<u8>>(this, 4);
    if ev.is_null() {
        return 1;
    }
    if v_get_event_type(rd::<GPtr<u8>>(ev, 0x10)) != 0x47 {
        return 1;
    }
    let inner = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 4), 0x10);
    if v_get_source_entity(inner).is_null() {
        return 1;
    }
    if rd::<u8>(v_get_source_entity(inner), 0x36) & 7 != 3 {
        return 1;
    }
    let src = v_get_source_entity(inner);
    if group_membership_is_member(rd::<GPtr<u8>>(this, 0).byte_add(8), src) != 0 {
        return 0;
    }
    1
}

/// `CPedGroupIntelligence::ComputeEventResponseTasks` (0x5FC440): deletes and clears the 8 task slots at +0x10 and
/// the 8 at +0xB0 (0x14 stride), then `CGroupEventHandler::ComputeEventResponseTasks(+4, +0)`.
#[hook(addr = 0x5FC440, conv = thiscall, name = "CPedGroupIntelligence::ComputeEventResponseTasks", this = "CPedGroupIntelligence",
       expect = "53 56 8B D9 57", unverified = "mvpb-ai-2", route = "L2", hazards = "heap-writes")]
pub fn pgi_compute_event_response_tasks(this: GPtr<u8>) {
    for base in [0x10isize, 0xB0] {
        for i in 0..8isize {
            let off = base + 0x14 * i;
            let t = rd::<GPtr<u8>>(this, off);
            if !t.is_null() {
                v_delete(t, 1);
            }
            wr::<u32>(this, off, 0);
        }
    }
    group_event_handler_compute(rd::<GPtr<u8>>(this, 4), rd::<GPtr<u8>>(this, 0));
}

/// `CTaskGoToVehicleAndLean::Destructor` (0x660EE0): vtable 0x86FAC8; with the anim block referenced (+0x14) it
/// releases the block named at 0x86F92C and clears +0x14; releases the vehicle (+0xC); `CTaskComplex` base dtor.
#[hook(addr = 0x660EE0, conv = thiscall, name = "CTaskGoToVehicleAndLean::Destructor", this = "CTaskGoToVehicleAndLean",
       expect = "6A FF 68 78 47 84 00", unverified = "mvpb-ai-2", route = "L2", hazards = "heap-writes")]
pub fn go_to_vehicle_and_lean_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86FAC8);
    if rd::<u8>(this, 0x14) != 0 {
        let i = get_animation_block_index(0x86F92C);
        remove_anim_block_ref(i);
        wr::<u8>(this, 0x14, 0);
    }
    let e = rd::<GPtr<u8>>(this, 0xC);
    if !e.is_null() {
        clean_up_old_reference(e, this.byte_add(0xC));
    }
    task_complex_dtor(this);
}

/// `CPedAttractorManager::FindAssociatedAttractor<CPedAttractor>(const C2dEffect*, const CEntity*,
/// const SArray<CPedAttractor*>&)` (0x5EB6F0, `ret 0xC`): the first attractor of the array whose effect (+4) and
/// entity (+8) match; 0 when none (count = (end - begin) >> 2, 0 for a null begin).
#[hook(addr = 0x5EB6F0, conv = thiscall, name = "CPedAttractorManager::FindAssociatedAttractor<CPedAttractor>",
       this = "CPedAttractorManager", expect = "8B 44 24 0C 8B 50 04", unverified = "mvpb-ai-2", route = "L2", hazards = "none")]
pub fn attractor_find_associated(_this: GPtr<u8>, fx: u32, ent: u32, arr: GPtr<u8>) -> u32 {
    let begin = rd::<u32>(arr, 4);
    let n = if begin == 0 { 0 } else { (rd::<u32>(arr, 8).wrapping_sub(begin) as i32) >> 2 };
    for i in 0..n.max(0) {
        let a = rd::<GPtr<u8>>(GPtr::from_addr(begin.wrapping_add(4 * i as u32) as usize), 0);
        if rd::<u32>(a, 4) == fx && rd::<u32>(a, 8) == ent {
            return a.addr() as u32;
        }
    }
    0
}

/// `CInformFriendsEventQueue::Process` (0x4AC2E0, static): for each of the 8 entries (0xA9AFB8, 0xC stride: ped,
/// event, time) with a ped whose time has passed (`time < now`, unsigned): the event is added to the ped's event
/// group (`intelligence + 0x68`, `Add(event, false)`), the ped reference released; entries without a ped or just
/// processed have their event deleted and time = -1. Entries still waiting are left alone.
#[hook(addr = 0x4AC2E0, conv = cdecl, name = "CInformFriendsEventQueue::Process", expect = "53 56 57 BE BC AF A9 00",
       unverified = "mvpb-ai-2", route = "L2", hazards = "heap-writes")]
pub fn inform_friends_event_queue_process() {
    for i in 0..8usize {
        let e: GPtr<u8> = GPtr::from_addr(0xA9AFB8 + 0xC * i);
        let ped = rd::<GPtr<u8>>(e, 0);
        if !ped.is_null() {
            if rd::<u32>(e, 8) >= now() {
                continue;
            }
            let g = rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68);
            event_group_add(g, rd::<GPtr<u8>>(e, 4), 0);
            let p = rd::<GPtr<u8>>(e, 0);
            if !p.is_null() {
                clean_up_old_reference(p, e);
                wr::<u32>(e, 0, 0);
            }
        }
        let ev = rd::<GPtr<u8>>(e, 4);
        if !ev.is_null() {
            v_delete(ev, 1);
            wr::<u32>(e, 4, 0);
        }
        wr::<u32>(e, 8, 0xFFFF_FFFF);
    }
}

/// `CInformGroupEvent::Process` (0x4B29E0): without a ped the event (+8) is deleted and the time (+0xC) reset;
/// with a ped whose time has passed: when the ped's group exists and is active (`0xC098E0[id]`), a stack
/// `CEventGroupEvent(ped, event->Clone())` is added to the group intelligence (+0x30) and destroyed; then the ped
/// reference is released, the event deleted and the time reset to -1.
#[hook(addr = 0x4B29E0, conv = thiscall, name = "CInformGroupEvent::Process", this = "CInformGroupEvent",
       expect = "64 A1 00 00 00 00", unverified = "mvpb-ai-2", route = "L2", hazards = "heap-writes")]
pub fn inform_group_event_process(this: GPtr<u8>) {
    let ped = rd::<GPtr<u8>>(this, 0);
    if ped.is_null() {
        let ev = rd::<GPtr<u8>>(this, 8);
        if !ev.is_null() {
            v_delete(ev, 1);
            wr::<u32>(this, 8, 0);
        }
        wr::<u32>(this, 0xC, 0xFFFF_FFFF);
        return;
    }
    if rd::<u32>(this, 0xC) >= now() {
        return;
    }
    let g = get_peds_group(ped);
    if !g.is_null() {
        let id = get_group_id(g);
        if id >= 0 && rd::<u8>(GPtr::from_addr(0xC098E0), id as isize) != 0 {
            let c = v_event_clone(rd::<GPtr<u8>>(this, 8));
            let mut buf = [0u32; 8];
            let ge: GPtr<u8> = GPtr::from_raw(buf.as_mut_ptr().cast());
            event_group_event_ctor(ge, rd::<GPtr<u8>>(this, 0), c);
            pgi_add_event(g.byte_add(0x30), ge);
            event_group_event_dtor(ge);
        }
    }
    let p = rd::<GPtr<u8>>(this, 0);
    if !p.is_null() {
        clean_up_old_reference(p, this);
        wr::<u32>(this, 0, 0);
    }
    let ev = rd::<GPtr<u8>>(this, 8);
    if !ev.is_null() {
        v_delete(ev, 1);
        wr::<u32>(this, 8, 0);
    }
    wr::<u32>(this, 0xC, 0xFFFF_FFFF);
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_TASK_COMPLEX_FACIAL_MAKE_ABORTABLE,
    HOOK_TASK_SIMPLE_FACIAL_MAKE_ABORTABLE,
    HOOK_TASK_SIMPLE_FACIAL_GET_ANIM_ID,
    HOOK_TASK_COMPLEX_FACIAL_CREATE_NEXT_SUB_TASK,
    HOOK_TASK_COMPLEX_FACIAL_CREATE_FIRST_SUB_TASK,
    HOOK_TASK_COMPLEX_FACIAL_CONTROL_SUB_TASK,
    HOOK_HOLD_ENTITY_CTOR,
    HOOK_HOLD_ENTITY_DROP_ENTITY,
    HOOK_HOLD_ENTITY_START_ANIM,
    HOOK_HOLD_ENTITY_FINISH_ANIM_CB,
    HOOK_PGI_IS_GROUP_RESPONDING,
    HOOK_PGI_IS_CURRENT_EVENT_VALID,
    HOOK_PGI_COMPUTE_EVENT_RESPONSE_TASKS,
    HOOK_GO_TO_VEHICLE_AND_LEAN_DTOR,
    HOOK_ATTRACTOR_FIND_ASSOCIATED,
    HOOK_INFORM_FRIENDS_EVENT_QUEUE_PROCESS,
    HOOK_INFORM_GROUP_EVENT_PROCESS,
];
