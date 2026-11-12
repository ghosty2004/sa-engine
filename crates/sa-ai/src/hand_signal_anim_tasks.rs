//! The hand-signal tasks (batch `attr-7`): `CTaskComplexPlayHandSignalAnim` (vtable 0x86D5DC) `CreateSubTask`
//! 0x61B2F0 and `ControlSubTask` 0x61B580 (both unreversed in gta-reversed), and
//! `CTaskSimplePlayHandSignalAnim::StartAnim` 0x61AF60; reached by the gang peds greeting each other on Grove Street.
//! Complex task layout (0x18 bytes): +0xC anim id, +0x10 blend delta, +0x14 flags (bit 0 left hand model referenced,
//! bit 1 right hand, bit 2 the "ghands" anim block referenced, bit 3 fat hands). Ported from the 1.0 US bytes.
//! Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskComplexSequence::CTaskComplexSequence()` (0x632BD0)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10, `ret 4`)
    fn sequence_add_task(t: GPtr<u8>, task: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CTaskSimpleStandStill(int ms, bool looped, bool useAnimIdle, float blend)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, ms: i32, looped: u32, idle: u32, blend: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskSimplePlayHandSignalAnim(AnimationId, float blend, bool fatHands, bool)` (0x61AE50, `ret 0x10`)
    fn play_hand_signal_anim_ctor(t: GPtr<u8>, anim: u32, blend: u32, fat: u32, b: u32) -> GPtr<u8> = thiscall 0x61AE50 => unported;
    /// `CGeneral::GetRandomNumberInRange(int, int)` (0x407180; draws its own rand)
    fn random_int(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CBaseModelInfo::AddRef()` (0x4C4BA0)
    fn model_info_add_ref(mi: GPtr<u8>) = thiscall 0x4C4BA0 => unported;
    /// `CStreaming::RequestModel(int32 id, int32 flags)` (0x4087E0)
    fn streaming_request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990)
    fn anim_get_block_index(name: GPtr<u8>) -> i32 = cdecl 0x4D3990 => unported;
    /// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0)
    fn anim_add_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    fn create_sub_task_o(t: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x61B2F0 => hand_signal_create_sub_task;
    /// `CPed::GetEntityThatThisPedIsHolding()` (0x5E02E0)
    fn ped_get_held_entity(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x5E02E0 => unported;
    /// `IKChainManager_c::IsArmPointing(int arm, CPed*)` (0x6182B0, `ret 8`)
    fn ik_is_arm_pointing(m: GPtr<u8>, arm: i32, ped: GPtr<u8>) -> u8 = thiscall 0x6182B0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610)
    fn anim_blend_animation(clump: u32, group: u32, id: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(cb, data)` (0x4CEBE0, `ret 8`)
    fn anim_set_finish_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBE0 => unported;
    /// `CObject::operator new(uint32)` (0x5A1EE0)
    fn object_new(size: u32) -> GPtr<u8> = cdecl 0x5A1EE0 => unported;
    /// `CHandObject::CHandObject(int32 model, CPed*, bool left)` (0x59EEB0, `ret 0xC`)
    fn hand_object_ctor(o: GPtr<u8>, model: u32, ped: GPtr<u8>, left: u32) -> GPtr<u8> = thiscall 0x59EEB0 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CAnimManager::GetAnimAssociation(AssocGroupId, AnimationId)` (0x4D3A60)
    fn anim_get_anim_association(group: u32, id: u32) -> GPtr<u8> = cdecl 0x4D3A60 => unported;
    /// `CAnimManager::AddAnimation(RpClump*, CAnimBlendHierarchy*, int flags)` (0x4D4330)
    fn anim_add_animation(clump: u32, hier: u32, flags: u32) -> GPtr<u8> = cdecl 0x4D4330 => unported;
    /// `CBaseModelInfo::RemoveRef()` (0x4C4BB0)
    fn model_info_remove_ref(mi: GPtr<u8>) = thiscall 0x4C4BB0 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0)
    fn anim_remove_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CStreaming::RemoveModel(int32)` (0x4089A0)
    fn streaming_remove_model(id: i32) = cdecl 0x4089A0 => unported;
    /// `CTaskComplex::~CTaskComplex()` (0x61A3D0)
    fn task_complex_dtor(t: GPtr<u8>) = thiscall 0x61A3D0 => unported;
}

/// `g_ikChainMan`
const IK_CHAIN_MAN: usize = 0xC15448;
/// `CObject::nNoTempObjects` (u16)
const NO_TEMP_OBJECTS: usize = 0xBB4A70;
/// `CTaskSimpleAnim::FinishRunAnimCB`
const FINISH_RUN_ANIM_CB: u32 = 0x61A8A0;

/// `CStreaming::ms_aInfoForModel[id].m_nLoadState` (0x14-byte entries)
fn load_state(id: usize) -> u8 {
    g::<u8>(0x8E4CD0 + 0x14 * id).read()
}

/// `CTaskComplexPlayHandSignalAnim::ms_animBlock` (-1 until looked up)
const ANIM_BLOCK: usize = 0x8D2E38;
/// `"ghands"`
const GHANDS: usize = 0x86D608;

/// `CTaskComplexPlayHandSignalAnim::CreateSubTask(eTaskType)` (0x61B2F0, thiscall, `ret 4`, SEH frame). 0xCB: a
/// `CTaskSimpleStandStill(0, 1, 0, 8.0)`. 0x1A9: a `CTaskComplexSequence` of `CTaskSimpleStandStill(
/// GetRandomNumberInRange(0, 1500), 0, 0, 8.0)` (the random number is drawn only when the pool gave a slot) and a
/// `CTaskSimplePlayHandSignalAnim(+0xC, +0x10, fat, 0)` with fat = bit 3 of +0x14 sign-extended into a byte (0xFF or
/// 0); `AddTask` runs on a null sequence too, as in the original. Anything else null.
#[hook(addr = 0x61B2F0, conv = thiscall, name = "CTaskComplexPlayHandSignalAnim::CreateSubTask",
       expect = "64 A1 00 00 00 00 6A FF", unverified = "attr-7", route = "L2", hazards = "heap-writes",
       recs = "grove-walk")]
pub fn hand_signal_create_sub_task(this: GPtr<u8>, ty: u32) -> GPtr<u8> {
    match ty {
        0xCB => {
            let t = task_new(0x20);
            if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, 0, 1, 0, 0x4100_0000) }
        }
        0x1A9 => {
            let t = task_new(0x40);
            let seq = if t.is_null() { GPtr::NULL } else { sequence_ctor(t) };
            let t = task_new(0x20);
            let ss = if t.is_null() {
                GPtr::NULL
            } else {
                let ms = random_int(0, 1500);
                stand_still_ctor(t, ms, 0, 0, 0x4100_0000)
            };
            sequence_add_task(seq, ss);
            let t = task_new(0x24);
            let anim = if t.is_null() {
                GPtr::NULL
            } else {
                let fat = (((rd::<u8>(this, 0x14) << 4) as i8) >> 7) as u8 as u32;
                play_hand_signal_anim_ctor(t, rd::<u32>(this, 0xC), rd::<u32>(this, 0x10), fat, 0)
            };
            sequence_add_task(seq, anim);
            seq
        }
        _ => GPtr::NULL,
    }
}

/// One hand model: referenced once it is loaded (flag `bit`), requested (flags 8) until then.
fn hand_model(this: GPtr<u8>, id: usize, mi_slot: usize, bit: u8, flags_at_entry: u8) {
    if load_state(id) == 1 {
        if flags_at_entry & bit == 0 {
            model_info_add_ref(g::<GPtr<u8>>(mi_slot).read());
            wr::<u8>(this, 0x14, rd::<u8>(this, 0x14) | bit);
        }
    } else {
        streaming_request_model(id as i32, 8);
    }
}

/// `CTaskComplexPlayHandSignalAnim::ControlSubTask(CPed*)` (0x61B580, thiscall, `ret 4`). The hand models (fat: 0x18C
/// / 0x18D, else 0x18A / 0x18B; model infos at 0xA9B6F0.. by id) are referenced once loaded or requested; with both
/// hands the "ghands" anim block (its index cached at 0x8D2E38) is referenced once its IFP is loaded, else
/// requested. With the block referenced and a stand-still sub-task (0xCB) the result is `CreateSubTask(0x1A9)`;
/// otherwise the sub-task read on entry. The left hand's check tests the flags read on entry, the right hand's the
/// current ones, as in the original.
#[hook(addr = 0x61B580, conv = thiscall, name = "CTaskComplexPlayHandSignalAnim::ControlSubTask",
       expect = "56 8B F1 8A 46 14 A8 08", unverified = "attr-7", route = "L2", hazards = "none",
       recs = "grove-walk")]
pub fn hand_signal_control_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    let entry_flags = rd::<u8>(this, 0x14);
    let sub = rd::<GPtr<u8>>(this, 8);
    if entry_flags & 8 != 0 {
        hand_model(this, 0x18C, 0xA9B6F8, 1, entry_flags);
        hand_model(this, 0x18D, 0xA9B6FC, 2, rd::<u8>(this, 0x14));
    } else {
        hand_model(this, 0x18A, 0xA9B6F0, 1, entry_flags);
        hand_model(this, 0x18B, 0xA9B6F4, 2, rd::<u8>(this, 0x14));
    }
    let f = rd::<u8>(this, 0x14);
    if f & 1 != 0 && f & 2 != 0 {
        let mut idx = g::<i32>(ANIM_BLOCK).read();
        if idx == -1 {
            idx = anim_get_block_index(GPtr::from_addr(GHANDS));
            g::<i32>(ANIM_BLOCK).write(idx);
        }
        if load_state((idx as usize).wrapping_add(0x63E7)) == 1 {
            if rd::<u8>(this, 0x14) & 4 == 0 {
                anim_add_block_ref(idx);
                wr::<u8>(this, 0x14, rd::<u8>(this, 0x14) | 4);
            }
        } else {
            streaming_request_model(idx.wrapping_add(0x63E7), 8);
        }
    }
    if rd::<u8>(this, 0x14) & 4 != 0 && task_get_type(rd::<GPtr<u8>>(this, 8)) == 0xCB {
        return create_sub_task_o(this, 0x1A9);
    }
    sub
}

/// `CTaskComplexPlayHandSignalAnim::~CTaskComplexPlayHandSignalAnim()` (0x61BDF0, thiscall, SEH frame): vtable
/// 0x86D5DC; each hand model referenced (flags bits 0 / 1) is released (fat hands: 0x18C / 0x18D, else 0x18A /
/// 0x18B). With the "ghands" block referenced (bit 2) the reference is removed; otherwise, when either hand model
/// has no references left (model info +8) and the block index is known with no references (`ms_aAnimBlocks[i]`
/// +0x12), its IFP (`index + 0x63E7`) is removed from the streaming. Then `CTaskComplex::~CTaskComplex`.
#[hook(addr = 0x61BDF0, conv = thiscall, name = "CTaskComplexPlayHandSignalAnim::Destructor",
       expect = "6A FF 68 D8 F2 83 00", unverified = "attr-7", route = "L2", hazards = "heap-writes",
       recs = "grove-walk")]
pub fn hand_signal_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86D5DC);
    let mi = |slot: usize| g::<GPtr<u8>>(slot).read();
    let f = rd::<u8>(this, 0x14);
    if f & 1 != 0 {
        model_info_remove_ref(mi(if f & 8 != 0 { 0xA9B6F8 } else { 0xA9B6F0 }));
    }
    let f = rd::<u8>(this, 0x14);
    if f & 2 != 0 {
        model_info_remove_ref(mi(if f & 8 != 0 { 0xA9B6FC } else { 0xA9B6F4 }));
    }
    let f = rd::<u8>(this, 0x14);
    if f & 4 != 0 {
        anim_remove_block_ref(g::<i32>(ANIM_BLOCK).read());
    } else {
        let (l, r) = if f & 8 != 0 { (0xA9B6F8, 0xA9B6FC) } else { (0xA9B6F0, 0xA9B6F4) };
        let both_used = rd::<u16>(mi(l), 8) != 0 && rd::<u16>(mi(r), 8) != 0;
        if !both_used {
            let idx = g::<i32>(ANIM_BLOCK).read();
            if idx != -1 && g::<u16>(0xB5D4B2usize.wrapping_add((idx as usize).wrapping_shl(5))).read() == 0 {
                streaming_remove_model(idx.wrapping_add(0x63E7));
            }
        }
    }
    task_complex_dtor(this);
}

/// One hand object of the signal: `CHandObject(model, ped, left)` (0x18C bytes) into `slot`; when created, added to
/// the world with the `group` animation of the task's anim id (+0x10) and counted as a temporary object. Returns
/// whether it was created.
fn hand_object(this: GPtr<u8>, ped: GPtr<u8>, model: u32, left: u32, group: u32, slot: isize) -> bool {
    let o = object_new(0x18C);
    let o = if o.is_null() { GPtr::NULL } else { hand_object_ctor(o, model, ped, left) };
    wr::<GPtr<u8>>(this, slot, o);
    if o.is_null() {
        return false;
    }
    world_add(o);
    let a = anim_get_anim_association(group, rd::<u32>(this, 0x10));
    anim_add_animation(rd::<u32>(rd::<GPtr<u8>>(this, slot), 0x18), rd::<u32>(a, 0x10), 0);
    let n = g::<u16>(NO_TEMP_OBJECTS);
    n.write(n.read().wrapping_add(1));
    true
}

/// `CTaskSimplePlayHandSignalAnim::StartAnim(CPed*)` (0x61AF60, thiscall, `ret 4`, SEH frame). Layout (0x24 bytes):
/// +8 the anim, +0x10 the hand anim id, +0x14 blend delta, +0x18 fat hands, +0x1C / +0x20 the hand objects. Anim
/// `0x140 - ftol((rand() & 0xFFFF) * 2^-15 * -5.0)`. A ped holding something, pointing its right arm or with a weapon
/// in the current slot (+0x718, i8; type at +0x5A0 + 0x1C * slot) blends it from group 0x4E and gets only a left hand
/// (group 0x4F); otherwise group 0x4D, a left hand (0x4F; on failure the right one is still tried) and a right hand
/// (0x50). Finish callback `CTaskSimpleAnim::FinishRunAnimCB`. No hands while the hand anim id is -1.
#[hook(addr = 0x61AF60, conv = thiscall, name = "CTaskSimplePlayHandSignalAnim::StartAnim",
       expect = "6A FF 68 C2 F1 83 00", unverified = "attr-7", route = "L2", hazards = "heap-writes",
       recs = "grove-walk", note = "rand() at 0x61AF80")]
pub fn play_hand_signal_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let r = sa_core::orig::at_site(0x61AF80, rand) & 0xFFFF;
    let anim = 0x140i32.wrapping_sub(ftol32(xi(r).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(0x858FC8).read())));
    let one_hand = !ped_get_held_entity(ped).is_null()
        || ik_is_arm_pointing(GPtr::from_addr(IK_CHAIN_MAN), 0, ped) != 0
        || {
            let slot = rd::<i8>(ped, 0x718) as i32;
            rd::<u32>(ped, (slot.wrapping_mul(0x1C) + 0x5A0) as isize) != 0
        };
    let group = if one_hand { 0x4E } else { 0x4D };
    let a = anim_blend_animation(rd::<u32>(ped, 0x18), group, anim as u32, rd::<u32>(this, 0x14));
    wr::<GPtr<u8>>(this, 8, a);
    anim_set_finish_callback(a, FINISH_RUN_ANIM_CB, this.addr() as u32);
    if rd::<i32>(this, 0x10) == -1 {
        return;
    }
    let fat = rd::<u8>(this, 0x18) != 0;
    if one_hand {
        hand_object(this, ped, if fat { 0x18C } else { 0x18A }, 1, 0x4F, 0x1C);
        return;
    }
    hand_object(this, ped, if fat { 0x18C } else { 0x18A }, 1, 0x4F, 0x1C);
    if rd::<i32>(this, 0x10) == -1 {
        return;
    }
    hand_object(this, ped, if fat { 0x18D } else { 0x18B }, 0, 0x50, 0x20);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_HAND_SIGNAL_CREATE_SUB_TASK,
    HOOK_HAND_SIGNAL_CONTROL_SUB_TASK,
    HOOK_PLAY_HAND_SIGNAL_START_ANIM,
    HOOK_HAND_SIGNAL_DTOR,
];
