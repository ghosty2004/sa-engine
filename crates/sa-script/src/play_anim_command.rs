//! `CRunningScript::PlayAnimScriptCommand` (batch `mvpn-script-1`): the shared body of `TASK_PLAY_ANIM` (0x605),
//! `TASK_PLAY_ANIM_NON_INTERRUPTABLE` (0x812), `TASK_PLAY_ANIM_WITH_FLAGS` (0x88A) and `TASK_PLAY_ANIM_SECONDARY`
//! (0xA1A). Ported from the 1.0 US bytes (the body lives in `.HOODLUM` at 0x1569460 behind the 5-byte entry stub;
//! gta-reversed does not reverse it). Unverified (docs/verify/HOWTO.md).
//!
//! Protection decoding (by hand): the `CollectParameters` count of 0x88A is an obfuscated push (`[0x1560540] -
//! [0x659622]` = 8); the last constructor argument of the untimed task is an obfuscated push (`~[0x1560548] ^
//! [0x6DBA32]` = 0). The timed task's last argument goes through the no-CD VM (0xCB73E0, descriptor 0x155FFBC,
//! return-address obfuscated: the VM's `ret 8` lands at 0x15695FD, which turns the result R into the pushed word
//! `R + 0x5C31F220 - 0x471803E0`). R was evaluated under Unicorn on the exe image (0x1166B30 with the descriptor):
//! 0xA7AE0DE0, independent of the stack words it is given, so the word is 0xBCC7FC20 (a `bool`: low byte 0x20,
//! true). Assumption: the VM's data is not changed at run time by the no-CD loader.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::cmdlib::{collect, read_text_label, sp};
use crate::vm::Scr;

/// `CTaskSequences::ms_iActiveSequence` (0x8D2E98)
const ACTIVE_SEQUENCE: usize = 0x8D2E98;
/// The timed task's VM-produced "hold last frame" word (see the module docs)
const TIMED_HOLD_WORD: u32 = 0xBCC7FC20;

orig! {
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskSimpleRunNamedAnim::CTaskSimpleRunNamedAnim(const char* anim, const char* group, int32 flags, float blend,
    /// int32 endTime, bool dontInterrupt, bool runInSequence, bool offsetPed, bool holdLastFrame)` (0x61A990, `ret 0x24`)
    fn run_named_anim_ctor(t: GPtr<u8>, anim: GPtr<u8>, group: GPtr<u8>, flags: u32, blend: u32, end_time: i32,
        dont_interrupt: u32, run_in_sequence: u32, offset_ped: u32, hold_last_frame: u32) -> GPtr<u8> = thiscall 0x61A990 => unported;
    /// `CTaskSimpleAffectSecondaryBehaviour::CTaskSimpleAffectSecondaryBehaviour(bool add, int32 type, CTask*)`
    /// (0x691270, `ret 0xC`)
    fn affect_secondary_behaviour_ctor(t: GPtr<u8>, add: u32, ty: u32, task: GPtr<u8>) -> GPtr<u8> = thiscall 0x691270 => unported;
    /// `CRunningScript::GivePedScriptedTask(int32 pedHandle, CTask*, int32 opcode)` (0x465C20)
    fn give_ped_scripted_task(s: Scr, ped: i32, task: GPtr<u8>, command: u32) = thiscall 0x465C20 => crate::script_tasks::give_ped_scripted_task;
}

/// `CRunningScript::PlayAnimScriptCommand(int32 command)` (0x470150, thiscall, `ret 4`; body at 0x1569460).
///
/// Reads the ped handle (1 parameter), the animation name (label, 24 bytes) and the group (16 bytes). 0x605 / 0xA1A
/// read 6 parameters, 0x812 too with "interruptable" cleared, 0x88A reads 8 and takes "interruptable" from
/// parameter 6 and "offset ped" from parameter 7 (other commands read nothing more). With `blend, loop, lockX,
/// lockY, lockF, time` = parameters 0..5 the flags are 0x10, 0x12 when looping or (time > 0 and not lockF), |0x40
/// lockX, |0x80 lockY, |8 without lockF, |0x400 for 0xA1A; "run in sequence" = a sequence is open
/// (`[0x8D2E98] >= 0`). A 0x64-byte task gets `CTaskSimpleRunNamedAnim(anim, group, flags, blend, time > 0 ? time :
/// -1, !interruptable, runInSequence, offsetPed, hold)` with hold = the VM word (true) when timed, else 0 (a failed
/// allocation gives no task). 0xA1A wraps it in `CTaskSimpleAffectSecondaryBehaviour(true, 4, task)` (a failed
/// allocation drops the anim task, as the original). `GivePedScriptedTask(ped, task, command)`.
///
/// Not reproduced: the original's SEH state bookkeeping, the uninitialised upper bytes of the `bool` argument words
/// (the constructor reads bytes; the port pushes 0/1).
#[hook(addr = 0x470150, conv = thiscall, name = "CRunningScript::PlayAnimScriptCommand", expect = "E9 0B 93 0F 01",
       unverified = "mvpn-script-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "the timed task's hold-last-frame word comes from the no-CD VM (evaluated statically as 0xBCC7FC20; see the module docs)")]
pub fn play_anim_script_command(s: Scr, command: u32) {
    let mut interruptable = true;
    let mut offset_ped = false;
    collect(s, 1);
    let ped = sp(0) as i32;
    let mut anim = [0u8; 24];
    let mut group = [0u8; 16];
    read_text_label(s, GPtr::from_raw(anim.as_mut_ptr()), 0x18);
    read_text_label(s, GPtr::from_raw(group.as_mut_ptr()), 0x10);
    match command {
        0x88A => {
            collect(s, 8);
            interruptable = sp(6) != 0;
            offset_ped = sp(7) != 0;
        }
        0x812 => {
            interruptable = false;
            collect(s, 6);
        }
        0x605 | 0xA1A => collect(s, 6),
        _ => {}
    }
    let blend = sp(0);
    let time = sp(5) as i32;
    let lock_f = sp(4);
    let mut flags: u32 = if sp(1) != 0 || (time > 0 && lock_f == 0) { 0x12 } else { 0x10 };
    if sp(2) != 0 {
        flags |= 0x40;
    }
    if sp(3) != 0 {
        flags |= 0x80;
    }
    if lock_f == 0 {
        flags |= 8;
    }
    if command == 0xA1A {
        flags |= 0x400;
    }
    let in_sequence = (GPtr::<i32>::from_addr(ACTIVE_SEQUENCE).read() >= 0) as u32;
    let t = task_new(0x64);
    let anim_p = GPtr::from_raw(anim.as_mut_ptr());
    let group_p = GPtr::from_raw(group.as_mut_ptr());
    let mut task = if t.is_null() {
        GPtr::NULL
    } else if time > 0 {
        run_named_anim_ctor(t, anim_p, group_p, flags, blend, time, (!interruptable) as u32, in_sequence,
                            offset_ped as u32, TIMED_HOLD_WORD)
    } else {
        run_named_anim_ctor(t, anim_p, group_p, flags, blend, -1, (!interruptable) as u32, in_sequence,
                            offset_ped as u32, 0)
    };
    if command == 0xA1A {
        let w = task_new(0x14);
        task = if w.is_null() { GPtr::NULL } else { affect_secondary_behaviour_ctor(w, 1, 4, task) };
    }
    give_ped_scripted_task(s, ped, task, command);
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PLAY_ANIM_SCRIPT_COMMAND];
