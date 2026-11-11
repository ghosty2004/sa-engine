//! The off-screen frame update callback (batch `mvpm-anim-1`): `FrameUpdateCallBackOffscreen`, the
//! `CAnimBlendClumpData::ForAllFrames` callback `RpAnimBlendClumpUpdateAnimations` uses for clumps that are not
//! visible. Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `FrameUpdateCallBackSkinnedWithVelocityExtraction(AnimBlendFrameData*, void* data)` (0x4D1680): register
    /// convention, the update data in EAX and the frame on the stack (caller pops).
    fn frame_update_skinned_with_velocity_extraction(data: GPtr<u8>, frame: GPtr<u8>) = reg("args = eax stack") 0x4D1680 => unported;
}

/// `gpAnimBlendClump` (the clump data being updated, 0xB4EA0C)
const ANIM_BLEND_CLUMP: usize = 0xB4EA0C;

/// `FrameUpdateCallBackOffscreen(AnimBlendFrameData* frame, void* data)` (0x4D2E10, cdecl). Only a frame flagged 8
/// (skinned with 3D velocity extraction) is updated, and only while the current clump data's ped position pointer
/// (+0xC) is set: `FrameUpdateCallBackSkinnedWithVelocityExtraction` with the data in EAX and the frame pushed.
#[hook(addr = 0x4d2e10, conv = cdecl, name = "FrameUpdateCallBackOffscreen", expect = "8B 44 24 04 F6 00 08",
       unverified = "mvpm-anim-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "the callee takes the data pointer in EAX (orig reg call)")]
pub fn frame_update_call_back_offscreen(frame: GPtr<u8>, data: GPtr<u8>) {
    if rd::<u8>(frame, 0) & 8 == 0 {
        return;
    }
    if rd::<u32>(g::<GPtr<u8>>(ANIM_BLEND_CLUMP).read(), 0xC) == 0 {
        return;
    }
    frame_update_skinned_with_velocity_extraction(data, frame);
}

pub const HOOKS: &[HookReg] = &[HOOK_FRAME_UPDATE_CALL_BACK_OFFSCREEN];
