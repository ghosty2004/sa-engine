//! `CTaskComplexGoToAttractor::CreateFirstSubTask(CPed*)` (0x670420, batch `std13-ai-1`; unreversed in gta-reversed).
//! Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
//!
//! The task (0x2C bytes): +0xC the attractor, +0x10 the target position, +0x1C the heading, +0x20 the time, +0x24 the
//! queue slot, +0x28 the move state.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPedAttractor::GetType()` (vtable slot 0)
    fn attractor_get_type(a: GPtr<u8>) -> u32 = thiscall vslot 0x0 => unported;
    /// `CPedPlacement::FindZCoorForPed(CVector*)` (0x616920)
    fn find_z_coor_for_ped(v: GPtr<u8>) -> u8 = cdecl 0x616920 => unported;
    /// `CTaskSimpleStandStill(int ms, bool looped, bool useAnimIdle, float blend)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, ms: i32, looped: u32, idle: u32, blend: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence()` (0x632BD0)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10, `ret 4`)
    fn sequence_add_task(t: GPtr<u8>, task: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CTaskComplexGoToPointAndStandStill(int moveState, const CVector&, float radius, float moveStateRadius, bool,
    /// bool)` (0x668120, `ret 0x18`)
    fn go_to_point_and_stand_still_ctor(t: GPtr<u8>, ms: u32, pos: GPtr<u8>, radius: u32, ms_radius: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x668120 => unported;
    /// `CTaskComplexTurnToFaceEntityOrCoord`-like heading task ctor (0x66C3E0: `(const CVector&, float heading, float)`,
    /// `ret 0xC`)
    fn heading_task_ctor(t: GPtr<u8>, pos: GPtr<u8>, heading: u32, f: u32) -> GPtr<u8> = thiscall 0x66C3E0 => unported;
}

/// `0.5f` and `2.0f` (0x86FC84 / 0x86FC88)
const RADIUS: usize = 0x86FC84;
const MS_RADIUS: usize = 0x86FC88;

/// `CTaskComplexGoToAttractor::CreateFirstSubTask(CPed*)` (0x670420, thiscall, `ret 4`, SEH frame). Move state = +0x28,
/// or 6 (sprint) when the attractor's type is 4. A ped with flag 0x200000 at +0x474 is placed at once: target position
/// through `FindZCoorForPed`, written to the matrix (+0x30..) or the placeable position, +0x55C / +0x558 = the heading,
/// and the sub-task is `CTaskSimpleStandStill(0, 0, 0, 8.0)` (0x20 bytes). Otherwise a `CTaskComplexSequence` (0x40)
/// of `CTaskComplexGoToPointAndStandStill(moveState, target, [0x86FC84], [0x86FC88], 0, 0)` (0x28) and the 0x66C3E0
/// task (0x80; target, heading, 0.5); `AddTask` runs on a null sequence too, as in the original.
#[hook(addr = 0x670420, conv = thiscall, name = "CTaskComplexGoToAttractor::CreateFirstSubTask",
       expect = "64 A1 00 00 00 00 6A FF", unverified = "std13-ai-1", route = "L2", hazards = "heap-writes",
       recs = "grove-walk", note = "FindZCoorForPed writes the task's own target z")]
pub fn create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let mut ms = rd::<u32>(this, 0x28);
    if attractor_get_type(rd::<GPtr<u8>>(this, 0xC)) == 4 {
        ms = 6;
    }
    if rd::<u32>(ped, 0x474) & 0x20_0000 != 0 {
        let pos = this.byte_add(0x10);
        find_z_coor_for_ped(pos);
        let v = [rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8)];
        let m = rd::<GPtr<u8>>(ped, 0x14);
        if m.is_null() {
            for (i, f) in v.iter().enumerate() {
                wr::<f32>(ped, 4 + 4 * i as isize, x(*f).to_f32());
            }
        } else {
            for (i, f) in v.iter().enumerate() {
                wr::<f32>(rd::<GPtr<u8>>(ped, 0x14), 0x30 + 4 * i as isize, x(*f).to_f32());
            }
        }
        wr::<u32>(ped, 0x55C, rd::<u32>(this, 0x1C));
        wr::<u32>(ped, 0x558, rd::<u32>(this, 0x1C));
        let t = task_new(0x20);
        return if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, 0, 0, 0, 0x4100_0000) };
    }
    let t = task_new(0x40);
    let seq = if t.is_null() { GPtr::NULL } else { sequence_ctor(t) };
    let t = task_new(0x28);
    let go = if t.is_null() {
        GPtr::NULL
    } else {
        go_to_point_and_stand_still_ctor(
            t,
            ms,
            this.byte_add(0x10),
            g::<u32>(RADIUS).read(),
            g::<u32>(MS_RADIUS).read(),
            0,
            0,
        )
    };
    sequence_add_task(seq, go);
    let t = task_new(0x80);
    let turn = if t.is_null() {
        GPtr::NULL
    } else {
        heading_task_ctor(t, this.byte_add(0x10), rd::<u32>(this, 0x1C), 0x3F00_0000)
    };
    sequence_add_task(seq, turn);
    seq
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_CREATE_FIRST_SUB_TASK];
