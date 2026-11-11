//! `CTaskSimplePlayerOnFoot`: `ProcessPed`, `MakeAbortable`, `PlayerControlZelda` and `PlayIdleAnimations`
//! (batch `mvpj-ai-3`). Ported from the 1.0 US bytes (gta-reversed `TaskSimplePlayerOnFoot.cpp` as a reading
//! aid; the bytes win). Unverified (docs/verify/HOWTO.md).
//!
//! Ped fields used: +0x18 clump, +0x46C/+0x478 flags, +0x47C intelligence, +0x480 player data, +0x4D4 anim
//! group, +0x528 flags, +0x534 move state, +0x55C rotation target, +0x568 contact entity, +0x598 player
//! number, +0x5A0 weapon slots (0x1C each, +0 type), +0x718 current slot (i8), +0x71C weapon lock target,
//! +0xBA contact surface, +0xFC attached-to entity. Player data: +0xC/+0x10 cleared each Zelda frame,
//! +0x14 move blend ratio, +0x34 flags, +0x84 "can't sprint" byte.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::m_FrameCounter` (0xB7CB4C)
const TIMER_FRAME_COUNTER: usize = 0xB7CB4C;
/// `CTimer::ms_fTimeStep` (0xB7CB5C)
const TIMER_TIME_STEP: usize = 0xB7CB5C;
/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const TIMER_TIME_MS: usize = 0xB7CB84;
/// `TheCamera` (0xB6F028)
const THE_CAMERA: usize = 0xB6F028;
/// `TheCamera.m_bWideScreenOn` (0xB6F065)
const CAMERA_WIDESCREEN: usize = 0xB6F065;
/// `TheCamera.m_nActiveCam` (0xB6F081)
const CAMERA_ACTIVE_CAM: usize = 0xB6F081;
/// `TheCamera` +0x150: the camera orientation the stick direction is relative to (0xB6F178)
const CAMERA_ORIENTATION: usize = 0xB6F178;
/// `TheCamera.m_aCams[0].m_nMode` (0xB6F1A8; cams are 0x238 bytes)
const CAMERA_CAM0_MODE: usize = 0xB6F1A8;
/// `CWorld::Players[0].m_pPed` (0xB7CD98)
const PLAYER1_PED: usize = 0xB7CD98;
/// `CWorld::Players[1].m_pPed` (0xB7CF28)
const PLAYER2_PED: usize = 0xB7CF28;
/// `CAnimManager::ms_aAnimAssocGroups` (0xB4EA34, a pointer; 0x14 bytes per group)
const ANIM_ASSOC_GROUPS: usize = 0xB4EA34;
/// `CAnimManager::ms_aAnimations` (0xB4EA40, 0x18 bytes each)
const ANIMATIONS: usize = 0xB4EA40;
/// `CAnimManager::ms_aAnimBlocks` (0xB5D4A0, 0x20 bytes each: +0x10 loaded, +0x14 first anim, +0x18 count)
const ANIM_BLOCKS: usize = 0xB5D4A0;
/// `g_surfaceInfos` (0xB79538)
const SURFACE_INFOS: usize = 0xB79538;
/// `gLastTouchTimeDelta` (0xC19664)
const LAST_TOUCH_TIME_DELTA: usize = 0xC19664;
/// `gLastRandomNumberForIdleAnimationID` (0x8D2FEC)
const LAST_IDLE_ANIM: usize = 0x8D2FEC;
/// 1/60 (0x859044): the stick magnitude scale
const K_STICK_SCALE: usize = 0x859044;
/// 0.07 (0x858CA8): the move blend ratio step per time step
const K_BLEND_STEP: usize = 0x858CA8;

orig! {
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn player_get_pad(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CPed::SetMoveState(eMoveState)` (0x5DEC00, `ret 4`)
    fn ped_set_move_state(p: GPtr<u8>, s: u32) = thiscall 0x5DEC00 => unported;
    /// `CPedIntelligence::GetTaskFighting()` (0x600F30)
    fn intel_get_task_fighting(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F30 => unported;
    /// `CPedIntelligence::GetTaskUseGun()` (0x600F70)
    fn intel_get_task_use_gun(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F70 => unported;
    /// `CPedIntelligence::GetTaskHold(bool)` (0x600FF0, `ret 4`)
    fn intel_get_task_hold(i: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x600FF0 => unported;
    /// `CPedIntelligence::SetTaskDuckSecondary(uint16)` (0x601230, `ret 4`)
    fn intel_set_task_duck_secondary(i: GPtr<u8>, t: u32) = thiscall 0x601230 => unported;
    /// `CTaskSimplePlayerOnFoot::PlayerControlDucked(CPlayerPed*)` (0x687F30, `ret 4`)
    fn on_foot_player_control_ducked(t: GPtr<u8>, p: GPtr<u8>) = thiscall 0x687F30 => unported;
    /// `CTaskSimplePlayerOnFoot::PlayerControlFighter(CPlayerPed*)` (0x687530, `ret 4`)
    fn on_foot_player_control_fighter(t: GPtr<u8>, p: GPtr<u8>) = thiscall 0x687530 => unported;
    /// `CTaskSimplePlayerOnFoot::PlayerControlZeldaWeapon(CPlayerPed*)` (0x687C20, `ret 4`)
    fn on_foot_player_control_zelda_weapon(t: GPtr<u8>, p: GPtr<u8>) = thiscall 0x687C20 => unported;
    /// `CTaskSimplePlayerOnFoot::ProcessPlayerWeapon(CPlayerPed*)` (0x6859A0, `ret 4`)
    fn on_foot_process_player_weapon(t: GPtr<u8>, p: GPtr<u8>) = thiscall 0x6859A0 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810, `ret 4`)
    fn tm_get_task_secondary(tm: GPtr<u8>, i: u32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CTaskManager::GetActiveTask()` (0x681720)
    fn tm_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0, `ret 0xC`)
    fn tm_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: u32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CEvent::GetEventType` (vtable +4)
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CEvent::GetEventPriority` (vtable +8)
    fn v_event_priority(e: GPtr<u8>) -> i32 = thiscall vslot 0x8 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18, `ret 0xC`)
    fn v_task_make_abortable(t: GPtr<u8>, p: GPtr<u8>, prio: u32, e: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610)
    fn anim_blend_animation(clump: u32, group: u32, id: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CCamera::Using1stPersonWeaponMode()` (0x50BFF0)
    fn camera_using_1st_person_weapon_mode(c: GPtr<u8>) -> u8 = thiscall 0x50BFF0 => unported;
    /// `CCamera::ClearPlayerWeaponMode()` (0x50AB10)
    fn camera_clear_player_weapon_mode(c: GPtr<u8>) = thiscall 0x50AB10 => unported;
    /// `CWeaponEffects::ClearCrossHair(int32 player)` (0x742C60)
    fn weapon_effects_clear_cross_hair(n: u32) = cdecl 0x742C60 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CPad::GetPedWalkLeftRight()` (0x53FC90; int16 in AX)
    fn pad_get_ped_walk_left_right(pad: GPtr<u8>) -> u32 = thiscall 0x53FC90 => unported;
    /// `CPad::GetPedWalkUpDown()` (0x53FD30; int16 in AX)
    fn pad_get_ped_walk_up_down(pad: GPtr<u8>) -> u32 = thiscall 0x53FD30 => unported;
    /// `CPad::GetSprint()` (0x5407A0)
    fn pad_get_sprint(pad: GPtr<u8>) -> u8 = thiscall 0x5407A0 => unported;
    /// `CPad::DuckJustDown(CPed*)` (0x540720; the argument is not pushed here)
    fn pad_duck_just_down(pad: GPtr<u8>) -> u8 = thiscall 0x540720 => unported;
    /// `CPad::JumpJustDown()` (0x540770)
    fn pad_jump_just_down(pad: GPtr<u8>) -> u8 = thiscall 0x540770 => unported;
    /// `CPad::GetTarget()` (0x540670)
    fn pad_get_target(pad: GPtr<u8>) -> u8 = thiscall 0x540670 => unported;
    /// `CPad::SetTouched()` (0x53F200)
    fn pad_set_touched(pad: GPtr<u8>) = thiscall 0x53F200 => unported;
    /// `CPad::GetTouchedTimeDelta()` (0x53F210)
    fn pad_get_touched_time_delta(pad: GPtr<u8>) -> i32 = thiscall 0x53F210 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CBE0, ST0)
    fn general_get_radian_angle_between_points(x1: f32, y1: f32, x2: f32, y2: f32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn general_limit_radian_angle(a: f32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn general_random_int_in_range(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CGameLogic::IsPlayerAllowedToGoInThisDirection(CPed*, CVector dir, float limit)` (0x441E10)
    fn game_logic_is_player_allowed_to_go_in_this_direction(p: GPtr<u8>, dx: f32, dy: f32, dz: f32, limit: f32) -> u8 = cdecl 0x441E10 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn weapon_info_get(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `SurfaceInfos_c::CantSprintOn(uint32)` (0x55E870, `ret 4`)
    fn surface_cant_sprint_on(s: GPtr<u8>, id: u32) -> u8 = thiscall 0x55E870 => unported;
    /// `CAnimBlendAssocGroup::GetAnimation(AnimationId)` (0x4CE090, `ret 4`)
    fn assoc_group_get_animation(g: GPtr<u8>, id: u32) -> GPtr<u8> = thiscall 0x4CE090 => unported;
    /// `CPlayerPed::ControlButtonSprint(eSprintType)` (0x60A610, `ret 4`, ST0)
    fn player_control_button_sprint(p: GPtr<u8>, ty: u32) -> f64 = thiscall 0x60A610 => unported;
    /// `CPlayerPed::SetRealMoveAnim()` (0x60A9C0)
    fn player_set_real_move_anim(p: GPtr<u8>) = thiscall 0x60A9C0 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    fn clump_get_association(clump: u32, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `RpAnimBlendClumpGetFirstAssociation(RpClump*)` (0x4D15E0)
    fn clump_get_first_association(clump: u32) -> GPtr<u8> = cdecl 0x4D15E0 => unported;
    /// `RpAnimBlendGetNextAssociation(CAnimBlendAssociation*)` (0x4D6AB0)
    fn anim_get_next_association(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D6AB0 => unported;
    /// `CTaskSimpleDuck::CanPedDuck(CPed*)` (0x692610)
    fn duck_can_ped_duck(p: GPtr<u8>) -> u8 = cdecl 0x692610 => unported;
    /// `CPlayerPed::ClearWeaponTarget()` (0x609C80)
    fn player_clear_weapon_target(p: GPtr<u8>) = thiscall 0x609C80 => unported;
    /// `CTaskComplexJump::CTaskComplexJump(eForceClimb)` (0x67A030, `ret 4`)
    fn complex_jump_ctor(t: GPtr<u8>, climb: u32) -> GPtr<u8> = thiscall 0x67A030 => unported;
    /// `CStreaming::RequestModel(int32, int32 flags)` (0x4087E0)
    fn streaming_request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CStreaming::SetModelIsDeletable(int32)` (0x409C10)
    fn streaming_set_model_is_deletable(id: i32) = cdecl 0x409C10 => unported;
    /// `CStats::GetStatValue(eStats)` (0x558E40, ST0)
    fn stats_get_stat_value(id: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `CPed::Say(uint16, uint32, float, uint8, uint8, uint8)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, off: u32, prob: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
}

fn intel(ped: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(ped, 0x47C)
}

fn task_manager(ped: GPtr<u8>) -> GPtr<u8> {
    intel(ped).byte_add(4)
}

/// The current weapon's `CWeaponInfo` (skill 1).
fn current_weapon_info(ped: GPtr<u8>) -> GPtr<u8> {
    let slot = i32::from(rd::<i8>(ped, 0x718));
    weapon_info_get(rd::<u32>(ped, 0x5A0 + slot as isize * 0x1C), 1)
}

/// `CTaskSimplePlayerOnFoot::ProcessPed(CPed*)` (0x688810, thiscall, `ret 4`). No pad
/// (`GetPadFromPlayer`) → +0xC = `CTimer::m_FrameCounter`, AL = 0 (EAX = 0). Else: "fighting allowed" =
/// move state (+0x534) ≥ 4, or a chainsaw (type 9) in the current slot with a fight task whose +0x25 is 4
/// (the original keeps it in its own `ped` argument slot); `SetMoveState(PEDMOVE_STILL)`; then ducking
/// (+0x46C bit 26) → `PlayerControlDucked`, else a fight task and not that flag → `PlayerControlFighter`,
/// else a use-gun task whose weapon info (+0x30) has flag bit 1 clear → `PlayerControlZeldaWeapon`, else
/// `PlayerControlZelda(ped, false)`; `ProcessPlayerWeapon`; +0xC = the frame counter; AL = 0 (EAX's upper
/// bytes are the frame counter's, reproduced).
#[hook(addr = 0x688810, conv = thiscall, name = "CTaskSimplePlayerOnFoot::ProcessPed", expect = "56 8B 74 24 08 57 8B F9",
       unverified = "mvpj-ai-3", route = "L2", hazards = "ax-al-return,stack-residue", recs = "grove-walk",
       note = "writes a flag byte into its own ped argument slot (popped by ret 4)")]
pub fn on_foot_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    if player_get_pad(ped).is_null() {
        let fc = g::<u32>(TIMER_FRAME_COUNTER).read();
        wr::<u32>(this, 0xC, fc);
        return 0;
    }
    let mut fight_ok = rd::<i32>(ped, 0x534) >= 4;
    let slot = i32::from(rd::<i8>(ped, 0x718));
    if rd::<u32>(ped, 0x5A0 + slot as isize * 0x1C) == 9
        && !intel_get_task_fighting(intel(ped)).is_null()
        && rd::<u8>(intel_get_task_fighting(intel(ped)), 0x25) == 4
    {
        fight_ok = true;
    }
    ped_set_move_state(ped, 1);
    if rd::<u32>(ped, 0x46C) & 0x0400_0000 != 0 {
        on_foot_player_control_ducked(this, ped);
    } else if !intel_get_task_fighting(intel(ped)).is_null() && !fight_ok {
        on_foot_player_control_fighter(this, ped);
    } else if !intel_get_task_use_gun(intel(ped)).is_null()
        && !rd::<GPtr<u8>>(intel_get_task_use_gun(intel(ped)), 0x30).is_null()
        && (rd::<u32>(rd::<GPtr<u8>>(intel_get_task_use_gun(intel(ped)), 0x30), 0x18) >> 1) & 1 == 0
    {
        on_foot_player_control_zelda_weapon(this, ped);
    } else {
        on_foot_player_control_zelda(this, ped, 0);
    }
    on_foot_process_player_weapon(this, ped);
    let fc = g::<u32>(TIMER_FRAME_COUNTER).read();
    wr::<u32>(this, 0xC, fc);
    fc & 0xFFFF_FF00
}

/// `CTaskSimplePlayerOnFoot::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x6857E0, thiscall, `ret
/// 0xC`). Priority 2: player data +0x14 = 0, `BlendAnimation(clump, anim group, ANIM_ID_IDLE (3), 1000.0)`,
/// result 1. Priority 1: with a secondary task (slot 0): an event of priority below 0x3D → AL = 0 at once; a
/// damage event (type 9) → when the secondary task is of type 0x3FA or the event's +0x25 bit 1 is set, the
/// secondary task's `MakeAbortable(ped, 1, event)` (result ignored), result 1; any other event or none →
/// the secondary task's `MakeAbortable(ped, 1, event)` decides (not 1 → returned at once). Without a
/// secondary task: 1. Other priorities → AL = 0 at once. Result 1 then: with a weapon lock target (+0x71C),
/// player data +0x34 bit 3 or `TheCamera.Using1stPersonWeaponMode()`, and an event of type 9 or 0x42: type
/// 0x42, or a damage event with +0x40 and +0xC set, or one with no attached-to entity (+0xFC) and +0x25 bit 1
/// set, or weapon type (+0x1C) above 0x2F and not 0x34 → `ClearPlayerWeaponMode`,
/// `CWeaponEffects::ClearCrossHair(player number)`, the lock target's `CleanUpOldReference` and cleared.
/// EAX's upper bytes are left from earlier calls in the original (not reproduced).
#[hook(addr = 0x6857e0, conv = thiscall, name = "CTaskSimplePlayerOnFoot::MakeAbortable", expect = "8B 44 24 08 53 56 32 DB",
       unverified = "mvpj-ai-3", route = "L2", hazards = "ax-al-return", recs = "grove-walk")]
pub fn on_foot_make_abortable(_this: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 {
    let r: u8;
    if prio == 2 {
        wr::<u32>(rd::<GPtr<u8>>(ped, 0x480), 0x14, 0);
        anim_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), 3, 1000f32.to_bits());
        r = 1;
    } else if prio == 1 {
        let tm = task_manager(ped);
        if tm_get_task_secondary(tm, 0).is_null() {
            r = 1;
        } else if !ev.is_null() && v_event_priority(ev) < 0x3D {
            return 0;
        } else if !ev.is_null() && v_event_type(ev) == 9 {
            if v_task_type(tm_get_task_secondary(tm, 0)) == 0x3FA || rd::<u8>(ev, 0x25) & 2 != 0 {
                v_task_make_abortable(tm_get_task_secondary(tm, 0), ped, 1, ev);
            }
            r = 1;
        } else {
            let s = tm_get_task_secondary(tm, 0);
            if s.is_null() {
                r = 1;
            } else {
                let b = v_task_make_abortable(s, ped, 1, ev);
                if b != 1 {
                    return b;
                }
                r = 1;
            }
        }
    } else {
        return 0;
    }
    let target = ped.byte_add(0x71C);
    if rd::<u32>(ped, 0x71C) == 0
        && rd::<u8>(rd::<GPtr<u8>>(ped, 0x480), 0x34) & 8 == 0
        && camera_using_1st_person_weapon_mode(g::<u8>(THE_CAMERA)) == 0
    {
        return r;
    }
    if ev.is_null() {
        return r;
    }
    if v_event_type(ev) != 9 && v_event_type(ev) != 0x42 {
        return r;
    }
    if v_event_type(ev) == 9 && !(rd::<u8>(ev, 0x40) != 0 && rd::<u8>(ev, 0xC) != 0) {
        if rd::<u32>(ped, 0xFC) != 0 {
            return r;
        }
        if rd::<u8>(ev, 0x25) & 2 == 0 {
            let w = rd::<i32>(ev, 0x1C);
            if w <= 0x2F || w == 0x34 {
                return r;
            }
        }
    }
    camera_clear_player_weapon_mode(g::<u8>(THE_CAMERA));
    weapon_effects_clear_cross_hair(rd::<u32>(ped, 0x598));
    let t = rd::<GPtr<u8>>(target, 0);
    if !t.is_null() {
        entity_clean_up_old_reference(t, target);
    }
    wr::<u32>(target, 0, 0);
    r
}

/// `CTaskSimplePlayerOnFoot::PlayerControlZelda(CPlayerPed*, bool noJumpDuck)` (stub 0x6883D0, thiscall, `ret
/// 8`). x87. Player data +0xC/+0x10 = 0; lr, ud = the pad's ped walk axes (int16); r = f32(sqrt(ud*ud +
/// lr*lr) * (1/60)), 0 when attached (+0xFC). Pad +0x2A set and r above 1 → r = 1 (skipping the next test);
/// else r not above 0 → blend ratio (+0x14) = 0. Otherwise a = `LimitRadianAngle(f32(
/// GetRadianAngleBetweenPoints(0, 0, -lr, ud) - camera orientation))`, stored f32 at +0x55C; with FSIN/FCOS
/// of the register value, `IsPlayerAllowedToGoInThisDirection(ped, (-f32 sin, f32 cos, 0), 0)` false →
/// ratio 0; true → the ratio moves toward r by at most `ms_fTimeStep * 0.07` (register compares; equal to r
/// when within). Sprint: unless the weapon has flag bit 9, a contact entity (+0x568) with flags +0x40 bit 2
/// set and bit 3 clear, or a hold task with +0x38: player data +0x84, `CantSprintOn(surface +0xBA)` or
/// anims 1 and 2 of the ped's anim group sharing +0x10 → `GetSprint` sets move state 6; else
/// `ControlButtonSprint(0)` not below 1.0 sets 7. `SetRealMoveAnim`; two `RpAnimBlendClumpGetAssociation`
/// (0xB, 0xC; unused). Duck just pressed and `CanPedDuck` and not `noJumpDuck` → `SetTaskDuckSecondary(0)`.
/// Unless `noJumpDuck`, ped flag +0x46C bit 9, the weapon flag bit 9, no jump press, `GetTarget`, attached or
/// the active camera mode 7/8/0x33/0x22/0x2D/0x2E: `ClearWeaponTarget`, and with an active task not of type
/// 0xD3 a new `CTaskComplexJump(0)` set as primary task 3. `PlayIdleAnimations`; ped +0x528 |= 8.
///
/// Stack: the original keeps r in its own `ped` argument slot and the new task in its `noJumpDuck` slot (popped
/// by `ret 8`); not reproduced.
#[hook(addr = 0x6883d0, conv = thiscall, name = "CTaskSimplePlayerOnFoot::PlayerControlZelda", expect = "6A FF E9 2E F8 D7 FF",
       unverified = "mvpj-ai-3", route = "L2", hazards = "x87-transcendental,stack-residue", recs = "grove-walk",
       note = "FSIN/FCOS of the move direction; argument slots overwritten (see doc)")]
pub fn on_foot_player_control_zelda(this: GPtr<u8>, ped: GPtr<u8>, no_jump_duck: u8) {
    let cam = g::<f32>(CAMERA_ORIENTATION).read();
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    wr::<u32>(pd, 0xC, 0);
    wr::<u32>(pd, 0x10, 0);
    let pad = player_get_pad(ped);
    let lr = f32::from(pad_get_ped_walk_left_right(pad) as i16);
    let ud = f32::from(pad_get_ped_walk_up_down(pad) as i16);
    let mut r = x(ud).fmul_m(ud).fadd(x(lr).fmul_m(lr)).fsqrt().fmul_m(g::<f32>(K_STICK_SCALE).read()).to_f32();
    if rd::<u32>(ped, 0xFC) != 0 {
        r = 0.0;
    }
    let capped = rd::<u16>(pad, 0x2A) != 0 && !x(r).fcom(X87::one()).test_any(0x41);
    if capped {
        r = 1.0;
    }
    let mut moving = capped || !x(r).fcom(X87::zero()).test_any(0x41);
    if moving {
        let a0 = X87::from_f64_exact(general_get_radian_angle_between_points(0.0, 0.0, x(lr).fchs().to_f32(), ud));
        let a1 = a0.fsub_m(cam).to_f32();
        let a = X87::from_f64_exact(general_limit_radian_angle(a1));
        wr::<f32>(ped, 0x55C, a.to_f32());
        let s = a.fsin().to_f32();
        let c = a.fcos().to_f32();
        let ns = x(s).fchs().to_f32();
        if game_logic_is_player_allowed_to_go_in_this_direction(ped, ns, c, 0.0, 0.0) == 0 {
            moving = false;
        } else {
            let step = x(g::<f32>(TIMER_TIME_STEP).read()).fmul_m(g::<f32>(K_BLEND_STEP).read());
            let cur = rd::<f32>(pd, 0x14);
            if !x(r).fsub_m(cur).fcom(step).test_any(0x41) {
                wr::<f32>(pd, 0x14, step.fadd_m(cur).to_f32());
            } else if !step.fchs().fcom(x(r).fsub_m(cur)).test_any(0x41) {
                wr::<f32>(pd, 0x14, x(cur).fsub(step).to_f32());
            } else {
                wr::<f32>(pd, 0x14, r);
            }
        }
    }
    if !moving {
        wr::<u32>(pd, 0x14, 0);
    }
    if (rd::<u32>(current_weapon_info(ped), 0x18) >> 9) & 1 == 0 {
        let contact = rd::<GPtr<u8>>(ped, 0x568);
        let blocked_by_contact = !contact.is_null() && {
            let f = rd::<u32>(contact, 0x40);
            f & 4 != 0 && f & 8 == 0
        };
        let holding = !blocked_by_contact
            && !intel_get_task_hold(intel(ped), 0).is_null()
            && rd::<u32>(intel_get_task_hold(intel(ped), 0), 0x38) != 0;
        if !blocked_by_contact && !holding {
            let group = g::<GPtr<u8>>(ANIM_ASSOC_GROUPS).read().byte_add(rd::<i32>(ped, 0x4D4) as isize * 0x14);
            let mut use_pad_sprint = rd::<u8>(pd, 0x84) != 0
                || surface_cant_sprint_on(g::<u8>(SURFACE_INFOS), u32::from(rd::<u8>(ped, 0xBA))) != 0;
            if !use_pad_sprint {
                let a1 = rd::<u32>(assoc_group_get_animation(group, 1), 0x10);
                if a1 == rd::<u32>(assoc_group_get_animation(group, 2), 0x10) {
                    use_pad_sprint = true;
                } else if !X87::from_f64_exact(player_control_button_sprint(ped, 0)).fcom(X87::one()).c0() {
                    wr::<u32>(ped, 0x534, 7);
                }
            }
            if use_pad_sprint && pad_get_sprint(pad) != 0 {
                wr::<u32>(ped, 0x534, 6);
            }
        }
    }
    player_set_real_move_anim(ped);
    clump_get_association(rd::<u32>(ped, 0x18), 0xB);
    clump_get_association(rd::<u32>(ped, 0x18), 0xC);
    let ducked = pad_duck_just_down(pad) != 0 && duck_can_ped_duck(ped) != 0;
    if ducked && no_jump_duck == 0 {
        intel_set_task_duck_secondary(intel(ped), 0);
    }
    if no_jump_duck == 0 && try_jump(ped, pad) {
        player_clear_weapon_target(ped);
        let tm = task_manager(ped);
        if !tm_get_active_task(tm).is_null() && v_task_type(tm_get_active_task(tm)) != 0xD3 {
            let mut t = task_new(0x14);
            if !t.is_null() {
                t = complex_jump_ctor(t, 0);
            }
            tm_set_task(tm, t, 3, 0);
        }
    }
    on_foot_play_idle_animations(this, ped);
    wr::<u32>(ped, 0x528, rd::<u32>(ped, 0x528) | 8);
}

/// The jump conditions of `PlayerControlZelda` after `noJumpDuck` (in the original's order).
fn try_jump(ped: GPtr<u8>, pad: GPtr<u8>) -> bool {
    if rd::<u32>(ped, 0x46C) & 0x200 != 0 {
        return false;
    }
    if (rd::<u32>(current_weapon_info(ped), 0x18) >> 9) & 1 != 0 {
        return false;
    }
    if pad_jump_just_down(pad) == 0 || pad_get_target(pad) != 0 || rd::<u32>(ped, 0xFC) != 0 {
        return false;
    }
    let cam = u32::from(g::<u8>(CAMERA_ACTIVE_CAM).read());
    let mode = g::<u16>(CAMERA_CAM0_MODE + cam as usize * 0x238).read();
    !matches!(mode, 7 | 8 | 0x33 | 0x22 | 0x2D | 0x2E)
}

/// `CTaskSimplePlayerOnFoot::PlayIdleAnimations(CPlayerPed*)` (0x6872C0, thiscall, `ret 4`). Nothing in co-op
/// (both player peds set). The pad is `SetTouched` unless the player is idle (no widescreen, not ducking,
/// +0x478 bit 1 clear, no hold task, pad +0x10E word 0, move state ≤ 1) with anim group 0x36..0x38. Block B
/// = `ms_aAnimBlocks[this+8]`. Touch delta above 10000: `RequestModel(this+8 + 0x63E7, 2)`; with B loaded,
/// no association of the clump playing an animation of B (index of +0x14 in `ms_aAnimations`, 0x18 each),
/// ped +0x46C bits 2/3 clear and delta - `gLastTouchTimeDelta` above 20000: r = a random 0..3 other than the
/// last (`GetRandomNumberInRange(0, 4)` until different), `BlendAnimation(clump, 0x31, 0x105 + r, 8.0)` with
/// its flags +0x2F bit 1 set; last delta = delta, last idle = r; then with stat 0x12F not 0 and the time
/// above 1200000 ms, `Say(0x150, 0, 0.2, 0, 0, 0)`. Delta not above 10000: with B loaded,
/// `SetModelIsDeletable(this+8 + 0x63E7)` and every association with flag bit 9 (+0x2E) gets blend delta
/// (+0x1C) -8.0; the last delta = 0 (also when B is not loaded).
#[hook(addr = 0x6872c0, conv = thiscall, name = "CTaskSimplePlayerOnFoot::PlayIdleAnimations", expect = "83 EC 20 53 55 8B 6C 24 2C",
       unverified = "mvpj-ai-3", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes animation associations (RW heap)")]
pub fn on_foot_play_idle_animations(this: GPtr<u8>, ped: GPtr<u8>) {
    let pad = player_get_pad(ped);
    if g::<u32>(PLAYER1_PED).read() != 0 && g::<u32>(PLAYER2_PED).read() != 0 {
        return;
    }
    let busy = g::<u8>(CAMERA_WIDESCREEN).read() != 0
        || rd::<u32>(ped, 0x46C) & 0x0400_0000 != 0
        || rd::<u8>(ped, 0x478) & 2 != 0
        || !intel_get_task_hold(intel(ped), 0).is_null()
        || rd::<u16>(pad, 0x10E) != 0
        || rd::<i32>(ped, 0x534) > 1;
    if busy || !matches!(rd::<u32>(ped, 0x4D4), 0x36..=0x38) {
        pad_set_touched(pad);
    }
    let table: [(u32, u32); 4] = [(0x105, 0x31), (0x106, 0x31), (0x107, 0x31), (0x108, 0x31)];
    let block_idx = rd::<i32>(this, 8);
    let block = g::<u8>(ANIM_BLOCKS).byte_add(block_idx as isize * 0x20);
    let dt = pad_get_touched_time_delta(pad);
    let clump = rd::<u32>(ped, 0x18);
    if dt > 10000 {
        streaming_request_model(block_idx + 0x63E7, 2);
        if rd::<u8>(block, 0x10) == 0 {
            return;
        }
        let mut a = clump_get_first_association(clump);
        while !a.is_null() {
            let idx = (rd::<GPtr<u8>>(a, 0x14).addr().wrapping_sub(g::<u8>(ANIMATIONS).addr()) as i32) / 0x18;
            let first = rd::<i32>(block, 0x14);
            if idx >= first && idx < rd::<i32>(block, 0x18).wrapping_add(first) {
                return;
            }
            a = anim_get_next_association(a);
        }
        if rd::<u8>(ped, 0x46C) & 0xC != 0 {
            return;
        }
        if dt.wrapping_sub(g::<i32>(LAST_TOUCH_TIME_DELTA).read()) <= 20000 {
            return;
        }
        let mut r;
        loop {
            r = general_random_int_in_range(0, 4);
            if g::<i32>(LAST_IDLE_ANIM).read() != r {
                break;
            }
        }
        let (anim, group) = table[r as usize];
        let assoc = anim_blend_animation(clump, group, anim, 8f32.to_bits());
        wr::<u8>(assoc, 0x2F, rd::<u8>(assoc, 0x2F) | 2);
        g::<i32>(LAST_TOUCH_TIME_DELTA).write(dt);
        g::<i32>(LAST_IDLE_ANIM).write(r);
        if X87::from_f64_exact(stats_get_stat_value(0x12F)).fcom(X87::zero()).test_parity(0x44)
            && g::<u32>(TIMER_TIME_MS).read() > 0x124F80
        {
            ped_say(ped, 0x150, 0, 0.2f32.to_bits(), 0, 0, 0);
        }
        return;
    }
    if rd::<u8>(block, 0x10) != 0 {
        streaming_set_model_is_deletable(block_idx + 0x63E7);
        let mut a = clump_get_first_association(clump);
        while !a.is_null() {
            if (u32::from(rd::<u16>(a, 0x2E)) >> 9) & 1 != 0 {
                wr::<u32>(a, 0x1C, 0xC100_0000);
            }
            a = anim_get_next_association(a);
        }
    }
    g::<i32>(LAST_TOUCH_TIME_DELTA).write(0);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_ON_FOOT_PROCESS_PED,
    HOOK_ON_FOOT_MAKE_ABORTABLE,
    HOOK_ON_FOOT_PLAYER_CONTROL_ZELDA,
    HOOK_ON_FOOT_PLAY_IDLE_ANIMATIONS,
];
