//! WRITER-CLIMB (batch `climb-2`): the climb task's animation side, `CTaskSimpleClimb::StartAnim` (0x67DBE0).
//! Ported from the 1.0 US bytes; gta-reversed (`TaskSimpleClimb.cpp`) as a reading aid.
//!
//! Task fields: +9 change-anim flag, +0xA change-position flag, +0xD height for the anim, +0xE height for the
//! position, +0x2C the current anim.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CAnimBlendAssociation::SetDeleteCallback(void (*)(CAnimBlendAssociation*, void*), void*)` (0x4CEBC0)
    fn assoc_set_delete_callback(a: GPtr<u8>, cb: u32, data: GPtr<u8>) = thiscall 0x4CEBC0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610)
    fn anim_blend_animation(clump: u32, group: u32, id: u32, delta: f32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CPed::SetMoveState(eMoveState)` (0x5DEC00)
    fn ped_set_move_state(p: GPtr<u8>, s: u32) = thiscall 0x5DEC00 => unported;
    /// `CPed::SetMoveAnim()` (vtable slot 0x5C)
    fn ped_set_move_anim(p: GPtr<u8>) = thiscall vslot 0x5C => unported;
    /// `CStats::GetFatAndMuscleModifier(int32)` (0x559AF0, ST0)
    fn stats_get_fat_and_muscle_modifier(i: i32) -> f64 = cdecl 0x559AF0 => unported;
}

/// `CDefaultAnimCallback::DefaultAnimCB` (0x4D1390) and `CTaskSimpleClimb::DeleteAnimCB` (0x67A380), passed as
/// the original addresses.
const DEFAULT_ANIM_CB: u32 = 0x4D_1390;
const DELETE_ANIM_CB: u32 = 0x67_A380;

/// Detaches the current anim (+0x2C) with the default delete callback.
fn release(this: GPtr<u8>) {
    let a = rd::<GPtr<u8>>(this, 0x2C);
    if !a.is_null() {
        assoc_set_delete_callback(a, DEFAULT_ANIM_CB, GPtr::NULL);
    }
}

/// `void CTaskSimpleClimb::StartAnim(CPed* ped)` (0x67DBE0, `ret 4`).
///
/// By the anim height +0xD: 1 (grab) `CLIMB_IDLE` 0x81 at 4 after an anim, else `CLIMB_JUMP` 0x80 at 8, position
/// height 1; 2 (pull-up) from position height 0: heights 3 and `CLIMB_STAND` 0x83 at 4, paused (+0x2E bit 0
/// cleared), else `CLIMB_PULL` 0x82 at 1000 and position height 2; 3 (stand-up) 0x83 at 1000; 4 / 6 (finished)
/// the idle anim 3 of the ped's group (+0x4D4), move state still, `SetMoveAnim`, the player's move blend ratio
/// (+0x14) zeroed, then 0x86 (6) or `CLIMB_STAND_FINISH` 0x84 at 1000; 5 (vault) `CLIMB_JUMP_B` 0x85 at 16 after
/// 0x83, else at 1000. Grab and pull-up clear +0xA, the rest set it; +9 is cleared. Then the anim gets
/// `DeleteAnimCB(this)` and, for the player while it plays, 0x82 / 0x83 / 0x85 run at
/// `GetFatAndMuscleModifier(1)`.
#[hook(addr = 0x67DBE0, conv = thiscall, name = "CTaskSimpleClimb::StartAnim",
       expect = "53 56 8B F1 0F BE 46 0D",
       unverified = "climb-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn climb_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let clump = rd::<u32>(ped, 0x18);
    let h = rd::<i8>(this, 0xD);
    let set = |a: GPtr<u8>, change_pos: u8| {
        wr::<GPtr<u8>>(this, 0x2C, a);
        wr::<u8>(this, 0xA, change_pos);
        wr::<u8>(this, 9, 0);
    };
    match h {
        1 => {
            let a = if rd::<u32>(this, 0x2C) != 0 {
                release(this);
                anim_blend_animation(clump, 0, 0x81, 4.0)
            } else {
                anim_blend_animation(clump, 0, 0x80, 8.0)
            };
            wr::<u8>(this, 0xE, 1);
            set(a, 0);
        }
        2 => {
            if rd::<u8>(this, 0xE) == 0 {
                wr::<u8>(this, 0xD, 3);
                wr::<u8>(this, 0xE, 3);
                let a = anim_blend_animation(rd::<u32>(ped, 0x18), 0, 0x83, 4.0);
                wr::<GPtr<u8>>(this, 0x2C, a);
                wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xFE);
                wr::<u8>(this, 0xA, 0);
                wr::<u8>(this, 9, 0);
            } else {
                release(this);
                let a = anim_blend_animation(rd::<u32>(ped, 0x18), 0, 0x82, 1000.0);
                wr::<GPtr<u8>>(this, 0x2C, a);
                wr::<u8>(this, 0xE, 2);
                wr::<u8>(this, 0xA, 0);
                wr::<u8>(this, 9, 0);
            }
        }
        3 => {
            release(this);
            set(anim_blend_animation(rd::<u32>(ped, 0x18), 0, 0x83, 1000.0), 1);
        }
        5 => {
            release(this);
            let a = rd::<GPtr<u8>>(this, 0x2C);
            let delta = if !a.is_null() && rd::<u16>(a, 0x2C) == 0x83 { 16.0 } else { 1000.0 };
            set(anim_blend_animation(rd::<u32>(ped, 0x18), 0, 0x85, delta), 1);
        }
        4 | 6 => {
            anim_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), 3, 1000.0);
            ped_set_move_state(ped, 1);
            ped_set_move_anim(ped);
            let pd = rd::<GPtr<u8>>(ped, 0x480);
            if !pd.is_null() {
                wr::<u32>(pd, 0x14, 0);
            }
            release(this);
            let id = if rd::<u8>(this, 0xD) == 6 { 0x86 } else { 0x84 };
            set(anim_blend_animation(rd::<u32>(ped, 0x18), 0, id, 1000.0), 1);
        }
        _ => {}
    }
    let a = rd::<GPtr<u8>>(this, 0x2C);
    if a.is_null() {
        return;
    }
    assoc_set_delete_callback(a, DELETE_ANIM_CB, this);
    if rd::<u32>(ped, 0x480) == 0 {
        return;
    }
    let a = rd::<GPtr<u8>>(this, 0x2C);
    if rd::<u8>(a, 0x2E) & 1 == 0 {
        return;
    }
    if matches!(rd::<u16>(a, 0x2C), 0x82 | 0x83 | 0x85) {
        let s = X87::from_f64_exact(stats_get_fat_and_muscle_modifier(1)).to_f32();
        wr::<f32>(rd::<GPtr<u8>>(this, 0x2C), 0x24, s);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_CLIMB_START_ANIM];
