//! Named-anim task construction (batch `mvpi-ai-1`): `CTaskSimpleRunNamedAnim`. Ported from the 1.0
//! US bytes; unverified (docs/verify/HOWTO.md).
//!
//! `CTaskSimpleRunNamedAnim` (0x60): `CTaskSimpleAnim` (anim +8, flags byte +0xC: bit 1 don't
//! interrupt, bit 2 hold last frame, bit 4 run in sequence, bit 5 offset ped), anim name char[24] +0x10,
//! group name char[16] +0x28, blend delta +0x38, `CAnimBlendHierarchy*` +0x3C, end time +0x40,
//! +0x44..+0x58 cleared, flags +0x5C.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTaskSimpleRunNamedAnim` vtable (0x86D54C).
const VT_RUN_NAMED_ANIM: u32 = 0x86D54C;

orig! {
    /// `CTaskSimple::CTaskSimple` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => crate::mvpa_ai1::task_simple_ctor;
    /// `CAnimManager::GetAnimationBlock(const char*)` (0x4D3940)
    fn anim_get_animation_block(name: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D3940 => unported;
    /// `CAnimManager::GetAnimation(const char*, const CAnimBlock*)` (0x4D42F0)
    fn anim_get_animation(name: GPtr<u8>, block: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D42F0 => unported;
}

/// The byte loop the compiler emits for `strcpy(dst, src)` (the terminating NUL included).
fn copy_str(dst: GPtr<u8>, src: GPtr<u8>) {
    let mut i = 0isize;
    loop {
        let c = src.byte_add(i).read();
        dst.byte_add(i).write(c);
        if c == 0 {
            break;
        }
        i += 1;
    }
}

/// `CTaskSimpleRunNamedAnim::CTaskSimpleRunNamedAnim(const char* animName, const char* groupName, int32
/// flags, float blendDelta, int32 endTime, bool dontInterrupt, bool runInSequence, bool offsetPed, bool
/// holdLastFrame)` (0x61A990, thiscall, `ret 0x24`): `CTaskSimple` constructor; anim +8 = 0, flags byte
/// +0xC = (its previous value & 0xF8) | holdLastFrame << 2; blend delta +0x38, end time +0x40, vtable
/// 0x86D54C, +0x44 / +0x48 = 0, bytes +0x4C / +0x4D = 0, +0x50..+0x58 = 0, flags +0x5C; +0xC = (+0xC &
/// 0x8D) | offsetPed << 5 | runInSequence << 4 | dontInterrupt << 1 (each argument's bit 0); the two
/// names copied to +0x10 and +0x28; +0x3C = `GetAnimation(+0x10, block)` when
/// `GetAnimationBlock(+0x28)` is non-null, else 0. Returns `this`. The flags byte's other bits are
/// whatever the task pool slot held.
#[hook(addr = 0x61a990, conv = thiscall, name = "CTaskSimpleRunNamedAnim::Constructor-Anim", expect = "6A FF 68 28 F1 83 00",
       route = "L2", hazards = "none", recs = "grove-walk")]
#[allow(clippy::too_many_arguments)]
pub fn run_named_anim_ctor(
    this: GPtr<u8>,
    anim: GPtr<u8>,
    group: GPtr<u8>,
    flags: u32,
    blend: u32,
    end_time: u32,
    dont_interrupt: u32,
    run_in_sequence: u32,
    offset_ped: u32,
    hold_last_frame: u32,
) -> GPtr<u8> {
    task_simple_ctor(this);
    let fl = ((hold_last_frame as u8 & 1) << 2) | (rd::<u8>(this, 0xC) & 0xF8);
    wr::<u32>(this, 8, 0);
    wr::<u8>(this, 0xC, fl);
    wr::<u32>(this, 0x38, blend);
    wr::<u32>(this, 0x40, end_time);
    wr::<u32>(this, 0, VT_RUN_NAMED_ANIM);
    wr::<u32>(this, 0x44, 0);
    wr::<u32>(this, 0x48, 0);
    wr::<u8>(this, 0x4C, 0);
    wr::<u8>(this, 0x4D, 0);
    wr::<u32>(this, 0x50, 0);
    wr::<u32>(this, 0x54, 0);
    wr::<u32>(this, 0x58, 0);
    let old = rd::<u8>(this, 0xC);
    wr::<u32>(this, 0x5C, flags);
    let hi = (((offset_ped as u8 & 1) << 1) | (run_in_sequence as u8 & 1)) << 3;
    let fl = ((hi | (dont_interrupt as u8 & 1)) << 1) | (old & 0x8D);
    wr::<u8>(this, 0xC, fl);
    copy_str(this.byte_add(0x10), anim);
    copy_str(this.byte_add(0x28), group);
    let block = anim_get_animation_block(this.byte_add(0x28));
    if block.is_null() {
        wr::<u32>(this, 0x3C, 0);
    } else {
        let a = anim_get_animation(this.byte_add(0x10), block);
        wr::<GPtr<u8>>(this, 0x3C, a);
    }
    this
}

pub const HOOKS: &[HookReg] = &[HOOK_RUN_NAMED_ANIM_CTOR];
