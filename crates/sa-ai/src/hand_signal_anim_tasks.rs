//! `CTaskComplexPlayHandSignalAnim` (vtable 0x86D5DC; batch `attr-7`): `CreateSubTask` 0x61B2F0 and `ControlSubTask`
//! 0x61B580 (both unreversed in gta-reversed), reached by the gang peds greeting each other on Grove Street. Layout
//! (0x18 bytes): +0xC anim id, +0x10 blend delta, +0x14 flags (bit 0 left hand model referenced, bit 1 right hand,
//! bit 2 the "ghands" anim block referenced, bit 3 fat hands). Ported from the 1.0 US bytes. Unverified
//! (docs/verify/HOWTO.md).
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
}

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

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_HAND_SIGNAL_CREATE_SUB_TASK, HOOK_HAND_SIGNAL_CONTROL_SUB_TASK];
