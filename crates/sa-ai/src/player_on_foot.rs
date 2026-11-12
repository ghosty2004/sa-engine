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
       route = "L2", hazards = "ax-al-return,stack-residue", recs = "grove-walk",
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
       route = "L2", hazards = "ax-al-return", recs = "grove-walk")]
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
       route = "L2", hazards = "x87-transcendental,stack-residue", recs = "grove-walk",
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

orig! {
    /// `CWeapon::IsTypeMelee()` (0x73B1C0)
    fn weapon_is_type_melee(w: GPtr<u8>) -> u8 = thiscall 0x73B1C0 => unported;
    /// `CGameLogic::IsPlayerUse2PlayerControls(CPed*)` (0x442020)
    fn game_logic_is_player_use_2player_controls(p: GPtr<u8>) -> u8 = cdecl 0x442020 => unported;
    /// `CPedIntelligence::ClearTaskDuckSecondary()` (0x601390)
    fn intel_clear_task_duck_secondary(i: GPtr<u8>) = thiscall 0x601390 => unported;
    /// `CTaskSimpleDuck::ControlDuckMove(float x, float y)` (0x6923F0, `ret 8`)
    fn duck_control_duck_move(d: GPtr<u8>, mx: f32, my: f32) = thiscall 0x6923F0 => unported;
    /// `CTaskSimpleUseGun::ControlGunMove(const CVector2D&)` (0x61E0C0, `ret 4`)
    fn use_gun_control_gun_move(t: GPtr<u8>, v: GPtr<u8>) -> u8 = thiscall 0x61E0C0 => unported;
    /// `CPad::ExitVehicleJustDown()` (0x540120)
    fn pad_exit_vehicle_just_down(pad: GPtr<u8>) -> u8 = thiscall 0x540120 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): out = a - b
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
}

/// `CAnimManager` blend delta used by the duck/gun/stand-up blends (0x8D2FF0, 4.0)
const K_DUCK_BLEND_DELTA: usize = 0x8D2FF0;
/// 1/128 (0x858B88): the duck stick scale
const K_DUCK_STICK_SCALE: usize = 0x858B88;
/// 0.0 (0x858B50): the duck direction z weight, and the move ratio when the direction is blocked
const K_ZERO_F32: usize = 0x858B50;
/// 0.5 (0x858B8C): the stand-up move threshold
const K_HALF: usize = 0x858B8C;

/// The blend delta at 0x8D2FF0 (as raw float bits, read at the call).
fn blend_delta() -> u32 {
    g::<u32>(K_DUCK_BLEND_DELTA).read()
}

/// An entity's position (matrix +0x30, else the placement +4).
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CTaskSimplePlayerOnFoot::PlayerControlDucked(CPlayerPed*)` (0x687F30, thiscall, `ret 4`; `this` unused). x87.
///
/// The secondary task 1 (duck) NULL → nothing. `pad = GetPadFromPlayer()`; `sx = f32(lr * 1/128)`, `sy = f32(ud *
/// 1/128)` (the pad's ped walk axes, int16); `r = f32(sqrt(sx*sx + sy*sy))`, 0 when attached (+0xFC), else 1 when
/// above 1. Duck task +0x18 or +0x19 set → nothing more.
///
/// - Duck pressed, sprint, jump pressed, exit-vehicle pressed or `!CanPedDuck` → `ClearTaskDuckSecondary`; with a
///   use-gun task whose weapon info (+0x30) has flag bit 1 clear: r above 0.5 → `BlendAnimation(clump, 0, 0x32,
///   4.0)` (+0x2E |= 1), player data +0x14 = 1.0, `ControlGunMove(&(1.0, 0))`. Otherwise r above 0.5:
///   `BlendAnimation(clump, anim group, sprint ? 1 : 0, 4.0)` (+0x2E |= 1), +0x14 = 1.5, move state +0x534 and
///   +0x538 = 6 (sprint) or 4.
/// - Targeting with a non-melee weapon: two-player controls → `a = LimitRadianAngle(f32(GetRadianAngleBetweenPoints(0,
///   0, -sx, sy) - camera orientation))`, `s = -f32(fsin a)` (register value), `c = f32(fcos f32(a))` (the stored
///   value), `m = IsPlayerAllowedToGoInThisDirection(ped, (s, c, 0), 0) ? r : 0.0`; with the ped matrix rows `R`
///   (+0) and `F` (+0x10): `sx = f32((c*R.y + s*R.x + R.z*0.0) * m)`, `sy = f32(-((c*F.y + F.z*0.0 + s*F.x) * m))`;
///   +0x55C = with a weapon lock target (+0x71C) `fpatan(-d.x, d.y)` of `d = target - ped` (`VectorSub`), else
///   f32(a). Then (any targeting case) `ControlDuckMove(sx, sy)` and player data +0x14 = 0.
/// - Otherwise: r above 0.0 → +0x55C = f32(a) as above, FSIN/FCOS of the register value, not allowed → r = 0;
///   player data +0x14 = r, `ControlDuckMove(0.0, -r)`.
///
/// Stack: the original keeps the axes and r in its own `ped` argument slot (popped by `ret 4`); not reproduced.
#[hook(addr = 0x687f30, conv = thiscall, name = "CTaskSimplePlayerOnFoot::PlayerControlDucked", expect = "83 EC 18 55 56",
       unverified = "mvpk-ai-1", route = "L2", hazards = "x87-transcendental,stack-residue", recs = "grove-walk",
       note = "FSIN/FCOS/FSQRT/FPATAN of the duck move direction; argument slot overwritten (see doc)")]
pub fn on_foot_player_control_ducked_port(_this: GPtr<u8>, ped: GPtr<u8>) {
    let duck = tm_get_task_secondary(task_manager(ped), 1);
    if duck.is_null() {
        return;
    }
    let pad = player_get_pad(ped);
    let ud = f32::from(pad_get_ped_walk_up_down(pad) as i16);
    let lr = f32::from(pad_get_ped_walk_left_right(pad) as i16);
    let k = g::<f32>(K_DUCK_STICK_SCALE).read();
    let mut sx = x(lr).fmul_m(k).to_f32();
    let mut sy = x(ud).fmul_m(k).to_f32();
    let mut r = x(sx).fmul_m(sx).fadd(x(sy).fmul_m(sy)).fsqrt().to_f32();
    if rd::<u32>(ped, 0xFC) != 0 {
        r = 0.0;
    } else if !x(r).fcom(X87::one()).test_any(0x41) {
        r = 1.0;
    }
    if rd::<u8>(duck, 0x18) != 0 || rd::<u8>(duck, 0x19) != 0 {
        return;
    }
    let stop = pad_duck_just_down(pad) != 0
        || pad_get_sprint(pad) != 0
        || pad_jump_just_down(pad) != 0
        || pad_exit_vehicle_just_down(pad) != 0
        || duck_can_ped_duck(ped) == 0;
    if stop {
        let half = g::<f32>(K_HALF).read();
        intel_clear_task_duck_secondary(intel(ped));
        if !intel_get_task_use_gun(intel(ped)).is_null()
            && (rd::<u32>(rd::<GPtr<u8>>(intel_get_task_use_gun(intel(ped)), 0x30), 0x18) >> 1) & 1 == 0
        {
            if !x(r).fcom(x(half)).test_any(0x41) {
                let a = anim_blend_animation(rd::<u32>(ped, 0x18), 0, 0x32, blend_delta());
                wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
                wr::<u32>(rd::<GPtr<u8>>(ped, 0x480), 0x14, 0x3F80_0000);
                let mut mv = [1.0f32, 0.0];
                use_gun_control_gun_move(intel_get_task_use_gun(intel(ped)), GPtr::from_raw(mv.as_mut_ptr()).cast());
            }
            return;
        }
        let sprint = pad_get_sprint(pad) != 0;
        if x(r).fcom(x(half)).test_any(0x41) {
            return;
        }
        let a = anim_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), u32::from(sprint), blend_delta());
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
        wr::<u32>(rd::<GPtr<u8>>(ped, 0x480), 0x14, 0x3FC0_0000);
        let st = if sprint { 6 } else { 4 };
        wr::<u32>(ped, 0x534, st);
        wr::<u32>(ped, 0x538, st);
        return;
    }
    let cam = g::<f32>(CAMERA_ORIENTATION).read();
    let slot = i32::from(rd::<i8>(ped, 0x718));
    let targeting = pad_get_target(pad) != 0 && weapon_is_type_melee(ped.byte_add(0x5A0 + slot as isize * 0x1C)) == 0;
    if targeting {
        if game_logic_is_player_use_2player_controls(ped) != 0 {
            let a0 = X87::from_f64_exact(general_get_radian_angle_between_points(0.0, 0.0, x(sx).fchs().to_f32(), sy));
            let a1 = a0.fsub_m(cam).to_f32();
            let a = X87::from_f64_exact(general_limit_radian_angle(a1));
            let af = a.to_f32();
            let s = a.fsin().fchs().to_f32();
            let c = x(af).fcos().to_f32();
            let m = if game_logic_is_player_allowed_to_go_in_this_direction(ped, s, c, 0.0, 0.0) == 0 {
                x(g::<f32>(K_ZERO_F32).read())
            } else {
                x(r)
            };
            let z = g::<f32>(K_ZERO_F32).read();
            let mat = rd::<GPtr<u8>>(ped, 0x14);
            let row = |o: isize| rd::<f32>(mat, o);
            sx = x(c).fmul_m(row(4)).fadd(x(s).fmul_m(row(0))).fadd(x(row(8)).fmul_m(z)).fmul(m).to_f32();
            sy = x(c).fmul_m(row(0x14)).fadd(x(row(0x18)).fmul_m(z)).fadd(x(s).fmul_m(row(0x10))).fmul(m).fchs().to_f32();
            let target = rd::<GPtr<u8>>(ped, 0x71C);
            if target.is_null() {
                wr::<f32>(ped, 0x55C, af);
            } else {
                let mut d = [0.0f32; 3];
                vector_sub(GPtr::from_raw(d.as_mut_ptr()).cast(), entity_pos(target), entity_pos(ped));
                wr::<f32>(ped, 0x55C, x(d[0]).fchs().fpatan(x(d[1])).to_f32());
            }
        }
        duck_control_duck_move(duck, sx, sy);
        wr::<u32>(rd::<GPtr<u8>>(ped, 0x480), 0x14, 0);
        return;
    }
    if !x(r).fcom(X87::from_f64_exact(0.0)).test_any(0x41) {
        let a0 = X87::from_f64_exact(general_get_radian_angle_between_points(0.0, 0.0, x(sx).fchs().to_f32(), sy));
        let a1 = a0.fsub_m(cam).to_f32();
        let a = X87::from_f64_exact(general_limit_radian_angle(a1));
        wr::<f32>(ped, 0x55C, a.to_f32());
        let s = a.fsin().fchs().to_f32();
        let c = a.fcos().to_f32();
        if game_logic_is_player_allowed_to_go_in_this_direction(ped, s, c, 0.0, 0.0) == 0 {
            r = 0.0;
        }
    }
    wr::<f32>(rd::<GPtr<u8>>(ped, 0x480), 0x14, r);
    duck_control_duck_move(duck, 0.0, x(r).fchs().to_f32());
}

orig! {
    /// `CPad::GetPedWalkUpDown(CPed*)` (0x540E20, `ret 4`; int16 in AX)
    fn pad_get_ped_walk_up_down_ped(pad: GPtr<u8>, p: GPtr<u8>) -> u32 = thiscall 0x540E20 => unported;
    /// `CPad::GetPedWalkLeftRight(CPed*)` (0x540DC0, `ret 4`; int16 in AX)
    fn pad_get_ped_walk_left_right_ped(pad: GPtr<u8>, p: GPtr<u8>) -> u32 = thiscall 0x540DC0 => unported;
}

/// `CTaskSimplePlayerOnFoot::PlayerControlZeldaWeapon(CPlayerPed*)` (0x687C20, thiscall, `ret 4`; `this` unused).
/// x87.
///
/// No use-gun task, or attached (+0xFC) → nothing. `pad = GetPadFromPlayer()`; the move `(mx, my)` starts at 0. The
/// stick: the gun's weapon info (+0x30) flag bit 2 set and not two-player controls → in 1st-person weapon mode
/// the ped variants `GetPedWalkUpDown(ped)` / `GetPedWalkLeftRight(ped)`, otherwise (0, 0); else the plain
/// `GetPedWalkUpDown()` / `GetPedWalkLeftRight()`. `mx = f32(lr * 1/128)`, `my = f32(ud * 1/128)`.
///
/// Two-player controls: `r = sqrt(mx² + my²)` (stored f32); r above 0 → `a = LimitRadianAngle(f32(
/// GetRadianAngleBetweenPoints(0, 0, -mx, my) - camera orientation))`, `s = f32(-fsin a)`, `c = f32(fcos a)`
/// (register a), `m = IsPlayerAllowedToGoInThisDirection(ped, (s, c, 0), 0) ? f32 r : 0.0`; with a weapon lock
/// target (+0x71C): `mx = f32((c*R.y + s*R.x + R.z*0.0) * m)`, `my = f32(-((c*F.y + s*F.x + F.z*0.0) * m))` (ped
/// matrix rows R +0, F +0x10); without: +0x55C = f32(a), `mx = 0`, `my = f32(-m)`. Then (r above 0 or not) a lock
/// target sets +0x55C = `fpatan(-(t.x - p.x), t.y - p.y)` (x87 differences of the positions).
///
/// Then `len = sqrt(mx² + my²)` above 1 → `inv = 1 / len`, `mx = f32(mx * inv)`, `my = f32(inv * my)`;
/// `ControlGunMove(&(mx, my))`; duck pressed and `CanPedDuck` → `SetTaskDuckSecondary(0)`.
///
/// Stack: the original keeps the axes and r in its own `ped` argument slot (popped by `ret 4`); not reproduced.
#[hook(addr = 0x687c20, conv = thiscall, name = "CTaskSimplePlayerOnFoot::PlayerControlZeldaWeapon", expect = "83 EC 1C 53 56",
       unverified = "mvpk-ai-1", route = "L2", hazards = "x87-transcendental,stack-residue", recs = "grove-walk,weapon-combat",
       note = "FSIN/FCOS/FSQRT/FPATAN of the aiming move direction; argument slot overwritten (see doc)")]
pub fn on_foot_player_control_zelda_weapon_port(_this: GPtr<u8>, ped: GPtr<u8>) {
    let gun = intel_get_task_use_gun(intel(ped));
    if gun.is_null() || rd::<u32>(ped, 0xFC) != 0 {
        return;
    }
    let pad = player_get_pad(ped);
    let (lr, ud);
    if (rd::<u32>(rd::<GPtr<u8>>(gun, 0x30), 0x18) >> 2) & 1 != 0 && game_logic_is_player_use_2player_controls(ped) == 0 {
        if camera_using_1st_person_weapon_mode(GPtr::from_addr(THE_CAMERA)) != 0 {
            ud = f32::from(pad_get_ped_walk_up_down_ped(pad, ped) as i16);
            lr = f32::from(pad_get_ped_walk_left_right_ped(pad, ped) as i16);
        } else {
            ud = 0.0;
            lr = 0.0;
        }
    } else {
        ud = f32::from(pad_get_ped_walk_up_down(pad) as i16);
        lr = f32::from(pad_get_ped_walk_left_right(pad) as i16);
    }
    let k = g::<f32>(K_DUCK_STICK_SCALE).read();
    let mut mx = x(lr).fmul_m(k).to_f32();
    let mut my = x(ud).fmul_m(k).to_f32();
    if game_logic_is_player_use_2player_controls(ped) != 0 {
        let z = g::<f32>(K_ZERO_F32).read();
        let r = x(mx).fmul_m(mx).fadd(x(my).fmul_m(my)).fsqrt();
        let rf = r.to_f32();
        if !r.fcom(x(z)).test_any(0x41) {
            let cam = g::<f32>(CAMERA_ORIENTATION).read();
            let a0 = X87::from_f64_exact(general_get_radian_angle_between_points(0.0, 0.0, x(mx).fchs().to_f32(), my));
            let a1 = a0.fsub_m(cam).to_f32();
            let a = X87::from_f64_exact(general_limit_radian_angle(a1));
            let s = a.fsin().fchs().to_f32();
            let c = a.fcos().to_f32();
            if rd::<u32>(ped, 0x71C) != 0 {
                let m = if game_logic_is_player_allowed_to_go_in_this_direction(ped, s, c, 0.0, 0.0) == 0 { x(z) } else { x(rf) };
                let mat = rd::<GPtr<u8>>(ped, 0x14);
                let row = |o: isize| rd::<f32>(mat, o);
                mx = x(c).fmul_m(row(4)).fadd(x(s).fmul_m(row(0))).fadd(x(row(8)).fmul_m(z)).fmul(m).to_f32();
                my = x(c).fmul_m(row(0x14)).fadd(x(s).fmul_m(row(0x10))).fadd(x(row(0x18)).fmul_m(z)).fmul(m).fchs().to_f32();
            } else {
                wr::<f32>(ped, 0x55C, a.to_f32());
                let m = if game_logic_is_player_allowed_to_go_in_this_direction(ped, s, c, 0.0, 0.0) == 0 { z } else { rf };
                mx = 0.0;
                my = x(m).fchs().to_f32();
            }
        }
        let target = rd::<GPtr<u8>>(ped, 0x71C);
        if !target.is_null() {
            let (t, p) = (entity_pos(target), entity_pos(ped));
            let dx = x(rd::<f32>(t, 0)).fsub_m(rd::<f32>(p, 0));
            let dy = x(rd::<f32>(t, 4)).fsub_m(rd::<f32>(p, 4));
            wr::<f32>(ped, 0x55C, dx.fchs().fpatan(dy).to_f32());
        }
    }
    let one = g::<f32>(K_ONE).read();
    let len = x(mx).fmul_m(mx).fadd(x(my).fmul_m(my)).fsqrt();
    if !len.fcom(x(one)).test_any(0x41) {
        let inv = len.fdivr_m(one);
        mx = x(mx).fmul(inv).to_f32();
        my = inv.fmul_m(my).to_f32();
    }
    let mut mv = [mx, my];
    use_gun_control_gun_move(gun, GPtr::from_raw(mv.as_mut_ptr()).cast());
    if pad_duck_just_down(pad) != 0 && duck_can_ped_duck(ped) != 0 {
        intel_set_task_duck_secondary(intel(ped), 0);
    }
}

/// 1.0 (0x858624)
const K_ONE: usize = 0x858624;
/// 0.02 (0x858B38) and 1000.0 (0x858C4C): the fight idle timer's milliseconds per time step
const K_FIGHT_TIMER_A: usize = 0x858B38;
const K_FIGHT_TIMER_B: usize = 0x858C4C;
/// 2^32 (0x858C54): `fild` of an unsigned dword
const K_TWO_POW_32: usize = 0x858C54;
/// 2000.0 (0x870870): the fight idle time before the idle moves
const K_FIGHT_IDLE_MS: usize = 0x870870;
/// 16.0 (0x87086C): the squared lock-on distance beyond which the fighter may run
const K_FIGHT_LOCK_DIST_SQ: usize = 0x87086C;
/// -0.5 (0x858F40): the forward stick threshold of the lock-on move
const K_FIGHT_FORWARD: usize = 0x858F40;

orig! {
    /// `CTaskSimpleFight::ControlFight(CEntity*, uint8 command)` (0x61C5E0, `ret 8`)
    fn fight_control_fight(f: GPtr<u8>, target: GPtr<u8>, cmd: u32) -> u8 = thiscall 0x61C5E0 => unported;
    /// `CPad::GetDuck()` (0x540700)
    fn pad_get_duck(pad: GPtr<u8>) -> u8 = thiscall 0x540700 => unported;
}

/// The player data's move blend (+0xC / +0x10) one step of `step` toward `want` (the original's register
/// compares: `|want - cur| > step` → `cur ± step` by the sign of the difference, else `want`). `sign_f32`: the
/// sign test reads the difference back as an f32 (the free-move path), else the register (the lock-on path).
fn fighter_approach(pd: GPtr<u8>, off: isize, want: f32, step: f32, sign_f32: bool) {
    let cur = rd::<f32>(pd, off);
    let d = x(want).fsub_m(cur);
    if !d.fabs().fcom(x(step)).test_any(0x41) {
        let ds = if sign_f32 { x(d.to_f32()) } else { d };
        if !ds.fcom(x(g::<f32>(K_ZERO_F32).read())).test_any(0x41) {
            wr::<f32>(pd, off, x(step).fadd_m(rd::<f32>(pd, off)).to_f32());
        } else {
            wr::<f32>(pd, off, x(rd::<f32>(pd, off)).fsub_m(step).to_f32());
        }
    } else {
        wr::<f32>(pd, off, want);
    }
}

/// `CTaskSimplePlayerOnFoot::PlayerControlFighter(CPlayerPed*)` (0x687530, thiscall, `ret 4`). x87.
///
/// No fight task → nothing. `fire` = the current weapon's info (skill 1) dword +0 non-zero. `lr`, `ud` = the pad's
/// ped walk axes (f32 of int16); `mx = f32(lr/128)`, `my = f32(ud/128)`; `step = f32(ms_fTimeStep * 0.07)`.
///
/// 1. Move: two-player controls → `r = f32 sqrt(mx² + my²)`; above 0: `a = LimitRadianAngle(f32(
///    GetRadianAngleBetweenPoints(0, 0, -mx, my) - camera orientation))`, `s = f32(-fsin a)`, `c = f32(fcos a)`;
///    `IsPlayerAllowedToGoInThisDirection(ped, (s, c, 0), 0)` → `mx = f32((c*R.y + s*R.x + R.z*0) * r)`, `my =
///    f32(-((c*F.y + s*F.x + F.z*0) * r))` (ped matrix rows R +0, F +0x10), else mx = my = 0. Otherwise, with a
///    weapon lock target (+0x71C) or (byte 0xB6EC2E and `GetTarget`): r as above, above 0 → r capped at 1.0 and the
///    same rotation with the unrounded register sine/cosine (`fchs`/`fcos` on the stack, no allowed test).
/// 2. Locked on (lock target, or byte 0xB6EC2E and `GetTarget`): player data +0x10 approaches my, +0xC approaches mx
///    by `step`; the target farther than 4 (squared x87 distance above 16) and fight byte +0x27 below 11 (signed) →
///    sprint test; else `GetSprint` or `fire` → sprint test; else the idle direction. Sprint test: `GetSprint` →
///    `ControlFight(NULL, 0x11)`; my below -0.5 → `(NULL, 0x10)`; else `(NULL, 0xF)`.
/// 3. Free: `a0 = f32 GetRadianAngleBetweenPoints(0, 0, -lr, ud)`, `r = f32 sqrt(mx² + my²)`; above 1 → r = 1;
///    r zero (C3 only) → player data +0xC = +0x10 = 0; else my above 0 or unordered → nothing; else +0x10
///    approaches -r, +0xC = 0, +0x55C = f32 `LimitRadianAngle(f32(a0 - camera orientation))`. Then no lock target,
///    my not above 0 and fight byte +0x27 below 11 → the task's +0x10 += `_ftol(ms_fTimeStep * 0.02 * 1000)`, else
///    +0x10 = 0. Then no `GetSprint`, no `GetDuck`, no `fire`, my not above 0 and the task's +0x10 (unsigned, x87)
///    below 2000 → the idle direction; else `GetSprint` (result unused), duck pressed and `CanPedDuck` →
///    `SetTaskDuckSecondary(0)` and `ControlFight(NULL, 0x12)`, else `ControlFight(NULL, r > 0.5 ? 0x10 : 0xF)`.
/// 4. Idle direction (player data +0xC, +0x10): |+0x10| above 0 and above |+0xC| → `ControlFight(NULL, +0x10
///    below 0 ? 3 : 5)`; else |+0xC| above 0 → `(NULL, +0xC above 0 ? 6 : 4)`; else nothing.
///
/// Stack: the original keeps the axes and r in its own `ped` argument slot (popped by `ret 4`); not reproduced.
#[hook(addr = 0x687530, conv = thiscall, name = "CTaskSimplePlayerOnFoot::PlayerControlFighter", expect = "83 EC 28 53 55",
       unverified = "mvpk-ai-1", route = "L2", hazards = "x87-transcendental,stack-residue", recs = "grove-walk,weapon-combat",
       note = "FSIN/FCOS/FSQRT of the fight move direction, _ftol of the idle timer; argument slot overwritten (see doc)")]
pub fn on_foot_player_control_fighter_port(this: GPtr<u8>, ped: GPtr<u8>) {
    let fight = intel_get_task_fighting(intel(ped));
    if fight.is_null() {
        return;
    }
    let fire = rd::<u32>(current_weapon_info(ped), 0) != 0;
    let pad = player_get_pad(ped);
    let lr = f32::from(pad_get_ped_walk_left_right(pad) as i16);
    let ud = f32::from(pad_get_ped_walk_up_down(pad) as i16);
    let k = g::<f32>(K_DUCK_STICK_SCALE).read();
    let z = g::<f32>(K_ZERO_F32).read();
    let mut mx = x(lr).fmul_m(k).to_f32();
    let mut my = x(ud).fmul_m(k).to_f32();
    let step = x(g::<f32>(TIMER_TIME_STEP).read()).fmul_m(g::<f32>(K_BLEND_STEP).read()).to_f32();
    let cam = || g::<f32>(CAMERA_ORIENTATION).read();
    let angle = |mx: f32, my: f32| {
        let a0 = X87::from_f64_exact(general_get_radian_angle_between_points(0.0, 0.0, x(mx).fchs().to_f32(), my));
        X87::from_f64_exact(general_limit_radian_angle(a0.fsub_m(cam()).to_f32()))
    };
    let locked = |pad: GPtr<u8>| rd::<u32>(ped, 0x71C) != 0 || (g::<u8>(0xB6EC2E).read() != 0 && pad_get_target(pad) != 0);
    if game_logic_is_player_use_2player_controls(ped) != 0 {
        let rr = x(mx).fmul_m(mx).fadd(x(my).fmul_m(my)).fsqrt();
        let r = rr.to_f32();
        if !rr.fcom(x(z)).test_any(0x41) {
            let a = angle(mx, my);
            let s = a.fsin().fchs().to_f32();
            let c = a.fcos().to_f32();
            if game_logic_is_player_allowed_to_go_in_this_direction(ped, s, c, 0.0, 0.0) != 0 {
                let mat = rd::<GPtr<u8>>(ped, 0x14);
                let row = |o: isize| rd::<f32>(mat, o);
                mx = x(c).fmul_m(row(4)).fadd(x(s).fmul_m(row(0))).fadd(x(row(8)).fmul_m(z)).fmul_m(r).to_f32();
                my = x(c).fmul_m(row(0x14)).fadd(x(s).fmul_m(row(0x10))).fadd(x(row(0x18)).fmul_m(z)).fmul_m(r).fchs().to_f32();
            } else {
                mx = 0.0;
                my = 0.0;
            }
        }
    } else if locked(pad) {
        let rr = x(mx).fmul_m(mx).fadd(x(my).fmul_m(my)).fsqrt();
        let mut r = rr.to_f32();
        if !rr.fcom(x(z)).test_any(0x41) {
            if !x(r).fcom(x(g::<f32>(K_ONE).read())).test_any(0x41) {
                r = 1.0;
            }
            let a = angle(mx, my);
            let s = a.fsin().fchs();
            let c = a.fcos();
            let mat = rd::<GPtr<u8>>(ped, 0x14);
            let row = |o: isize| rd::<f32>(mat, o);
            mx = c.fmul_m(row(4)).fadd(s.fmul_m(row(0))).fadd(x(row(8)).fmul_m(z)).fmul_m(r).to_f32();
            my = c.fmul_m(row(0x14)).fadd(s.fmul_m(row(0x10))).fadd(x(row(0x18)).fmul_m(z)).fmul_m(r).fchs().to_f32();
        }
    }
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    let my_not_above_zero = |my: f32| !x(my).fcom(x(z)).test_parity(0x41);
    let sprint_test = |fight: GPtr<u8>| {
        if pad_get_sprint(pad) != 0 {
            fight_control_fight(fight, GPtr::NULL, 0x11);
        } else if !x(my).fcom(x(g::<f32>(K_FIGHT_FORWARD).read())).test_parity(5) {
            fight_control_fight(fight, GPtr::NULL, 0x10);
        } else {
            fight_control_fight(fight, GPtr::NULL, 0xF);
        }
    };
    if locked(pad) {
        fighter_approach(pd, 0x10, my, step, false);
        fighter_approach(pd, 0xC, mx, step, false);
        let target = rd::<GPtr<u8>>(ped, 0x71C);
        if !target.is_null() {
            let (t, p) = (entity_pos(target), entity_pos(ped));
            let dx = x(rd::<f32>(t, 0)).fsub_m(rd::<f32>(p, 0));
            let dy = x(rd::<f32>(t, 4)).fsub_m(rd::<f32>(p, 4));
            let dz = x(rd::<f32>(t, 8)).fsub_m(rd::<f32>(p, 8));
            let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
            if !d2.fcom(x(g::<f32>(K_FIGHT_LOCK_DIST_SQ).read())).test_any(0x41) && rd::<i8>(fight, 0x27) < 11 {
                sprint_test(fight);
                return;
            }
        }
        if pad_get_sprint(pad) != 0 || fire {
            sprint_test(fight);
            return;
        }
        fighter_idle_direction(fight, pd);
        return;
    }
    let a0 = X87::from_f64_exact(general_get_radian_angle_between_points(0.0, 0.0, x(lr).fchs().to_f32(), ud)).to_f32();
    let rr = x(mx).fmul_m(mx).fadd(x(my).fmul_m(my)).fsqrt();
    let mut r = rr.to_f32();
    let mut turn = true;
    if !rr.fcom(x(g::<f32>(K_ONE).read())).test_any(0x41) {
        r = 1.0;
    } else if !x(r).fcom(x(z)).test_parity(0x44) {
        wr::<u32>(pd, 0xC, 0);
        wr::<u32>(pd, 0x10, 0);
        turn = false;
    }
    if turn && my_not_above_zero(my) {
        fighter_approach(pd, 0x10, x(r).fchs().to_f32(), step, true);
        wr::<u32>(pd, 0xC, 0);
        let a = X87::from_f64_exact(general_limit_radian_angle(x(a0).fsub_m(cam()).to_f32()));
        wr::<f32>(ped, 0x55C, a.to_f32());
    }
    let t = this.byte_add(0x10).cast::<u32>();
    if rd::<u32>(ped, 0x71C) == 0 && my_not_above_zero(my) && rd::<i8>(fight, 0x27) < 11 {
        let add = x(g::<f32>(TIMER_TIME_STEP).read())
            .fmul_m(g::<f32>(K_FIGHT_TIMER_A).read())
            .fmul_m(g::<f32>(K_FIGHT_TIMER_B).read())
            .ftol() as u32;
        t.write(t.read().wrapping_add(add));
    } else {
        t.write(0);
    }
    if pad_get_sprint(pad) == 0 && pad_get_duck(pad) == 0 && !fire && my_not_above_zero(my) {
        let v = t.read() as i32;
        let mut fv = X87::from_i32(v);
        if v < 0 {
            fv = fv.fadd_m(g::<f32>(K_TWO_POW_32).read());
        }
        if fv.fcom(x(g::<f32>(K_FIGHT_IDLE_MS).read())).c0() {
            fighter_idle_direction(fight, pd);
            return;
        }
    }
    pad_get_sprint(pad);
    let cmd = if !x(r).fcom(x(g::<f32>(K_HALF).read())).test_any(0x41) { 0x10 } else { 0xF };
    if pad_duck_just_down(pad) != 0 && duck_can_ped_duck(ped) != 0 {
        intel_set_task_duck_secondary(intel(ped), 0);
        fight_control_fight(fight, GPtr::NULL, 0x12);
        return;
    }
    fight_control_fight(fight, GPtr::NULL, cmd);
}

/// Part 4 of `PlayerControlFighter` (the idle direction from the player data's +0xC / +0x10).
fn fighter_idle_direction(fight: GPtr<u8>, pd: GPtr<u8>) {
    let z = x(g::<f32>(K_ZERO_F32).read());
    let a10 = x(rd::<f32>(pd, 0x10)).fabs();
    if !a10.fcom(z).test_any(0x41) && !a10.fcom(x(rd::<f32>(pd, 0xC)).fabs()).test_any(0x41) {
        let cmd = if x(rd::<f32>(pd, 0x10)).fcom(z).test_parity(5) { 5 } else { 3 };
        fight_control_fight(fight, GPtr::NULL, cmd);
        return;
    }
    if !x(rd::<f32>(pd, 0xC)).fabs().fcom(z).test_any(0x41) {
        let cmd = if !x(rd::<f32>(pd, 0xC)).fcom(z).test_any(0x41) { 6 } else { 4 };
        fight_control_fight(fight, GPtr::NULL, cmd);
    }
}

orig! {
    /// `CPed::GetWeaponSkill()` (0x5E6580; the current weapon's skill)
    fn ped_get_weapon_skill(p: GPtr<u8>) -> u32 = thiscall 0x5E6580 => unported;
    /// `CPad::WeaponJustDown(CPed*)` (0x540250, `ret 4`; AL, EAX passed on whole once)
    fn pad_weapon_just_down(pad: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x540250 => unported;
    /// `CPlayerPed::PlayerHasJustAttackedSomeone()` (0x60D5A0)
    fn player_has_just_attacked_someone(p: GPtr<u8>) = thiscall 0x60D5A0 => unported;
    /// `CPad::GetEnterTargeting()` (0x5406B0)
    fn pad_get_enter_targeting(pad: GPtr<u8>) -> u8 = thiscall 0x5406B0 => unported;
    /// `CTaskSimpleUseGun::PlayerPassiveControlGun()` (0x61E0A0)
    fn use_gun_player_passive_control_gun(t: GPtr<u8>) -> u8 = thiscall 0x61E0A0 => unported;
    /// `CCamera::SetNewPlayerWeaponMode(int16 mode, int16 maxZoom, int16 minZoom)` (0x50BFB0, `ret 0xC`)
    fn camera_set_new_player_weapon_mode(cam: GPtr<u8>, mode: u32, max: u32, min: u32) = thiscall 0x50BFB0 => unported;
    /// `CPed::SetPedState(ePedState)` (0x5E4500, `ret 4`)
    fn ped_set_ped_state(p: GPtr<u8>, s: u32) = thiscall 0x5E4500 => unported;
    /// `CPad::MeleeAttackJustDown(bool)` (0x540390, `ret 4`)
    fn pad_melee_attack_just_down(pad: GPtr<u8>, b: u32) -> u8 = thiscall 0x540390 => unported;
    /// `CPad::GetMeleeAttack(bool)` (0x540340, `ret 4`)
    fn pad_get_melee_attack(pad: GPtr<u8>, b: u32) -> u8 = thiscall 0x540340 => unported;
    /// `CTaskSimpleStealthKill::CTaskSimpleStealthKill(bool, CPed*, AssocGroupId)` (0x6225F0, `ret 0xC`)
    fn stealth_kill_ctor(t: GPtr<u8>, b: u32, target: GPtr<u8>, group: u32) -> GPtr<u8> = thiscall 0x6225F0 => unported;
    /// `CPedIntelligence::TestForStealthKill(CPed*, bool)` (0x601E00, `ret 8`)
    fn intel_test_for_stealth_kill(i: GPtr<u8>, target: GPtr<u8>, full: u32) -> u8 = thiscall 0x601E00 => unported;
    /// `CPedIntelligence::GetTaskDuck(bool)` (0x6010A0, `ret 4`)
    fn intel_get_task_duck(i: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x6010A0 => unported;
    /// `CTaskSimpleDuck::IsTaskInUseByOtherTasks()` (0x61C3D0)
    fn duck_is_task_in_use_by_other_tasks(d: GPtr<u8>) -> u8 = thiscall 0x61C3D0 => unported;
    /// `CTaskSimpleFight::CTaskSimpleFight(CEntity*, int32 command, uint32 idlePeriod)` (0x61C470, `ret 0xC`)
    fn fight_ctor(t: GPtr<u8>, target: GPtr<u8>, cmd: u32, idle: u32) -> GPtr<u8> = thiscall 0x61C470 => unported;
    /// `CTaskManager::SetTaskSecondary(CTask*, int32)` (0x681B60, `ret 8`)
    fn tm_set_task_secondary(tm: GPtr<u8>, t: GPtr<u8>, slot: u32) = thiscall 0x681B60 => unported;
    /// `CPedDamageResponseCalculator::CPedDamageResponseCalculator(CEntity*, float, eWeaponType, ePedPieceTypes,
    /// bool)` (0x4AD3F0, `ret 0x14`)
    fn damage_calc_ctor(c: GPtr<u8>, src: GPtr<u8>, dmg: u32, wt: u32, piece: u32, b: u32) -> GPtr<u8> = thiscall 0x4AD3F0 => unported;
    /// `CPedDamageResponseCalculator::~CPedDamageResponseCalculator()` (0x4AD420)
    fn damage_calc_dtor(c: GPtr<u8>) = thiscall 0x4AD420 => unported;
    /// `CPedDamageResponseCalculator::ComputeDamageResponse(CPed*, CPedDamageResponse&, bool)` (0x4B5AC0, `ret 0xC`)
    fn damage_calc_compute_damage_response(c: GPtr<u8>, ped: GPtr<u8>, r: GPtr<u8>, b: u32) = thiscall 0x4B5AC0 => unported;
    /// `CEventDamage::CEventDamage(CEntity*, uint32 time, eWeaponType, ePedPieceTypes, uint8 direction, bool, bool)`
    /// (0x4AD830, `ret 0x1C`; the last bool goes as a whole dword)
    fn event_damage_ctor(e: GPtr<u8>, src: GPtr<u8>, time: u32, wt: u32, piece: u32, dir: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x4AD830 => unported;
    /// `CEventDamage::~CEventDamage()` (0x4AD960)
    fn event_damage_dtor(e: GPtr<u8>) = thiscall 0x4AD960 => unported;
    /// `CEventDamage::AffectsPed(CPed*)` (0x4B35A0, `ret 4`)
    fn event_damage_affects_ped(e: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x4B35A0 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CCrime::ReportCrime(eCrimeType, CEntity*, CPed*)` (0x532010)
    fn crime_report_crime(ty: i32, victim: GPtr<u8>, by: GPtr<u8>) = cdecl 0x532010 => unported;
    /// `CAEPedWeaponAudioEntity::AddAudioEvent(eAudioEvents)` (0x4E69F0, `ret 4`)
    fn ped_weapon_audio_add_audio_event(e: GPtr<u8>, ev: u32) = thiscall 0x4E69F0 => unported;
    /// `CTaskSimpleUseGun::ControlGun(CPed*, CEntity*, int8 command)` (0x61E040, `ret 0xC`)
    fn use_gun_control_gun(t: GPtr<u8>, ped: GPtr<u8>, target: GPtr<u8>, cmd: u32) -> u8 = thiscall 0x61E040 => unported;
    /// `CPad::GetWeapon(CPed*)` (0x540180, `ret 4`; the whole EAX is tested)
    fn pad_get_weapon(pad: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x540180 => unported;
    /// `CTaskSimpleUseGun::RequirePistolWhip(CPed*, CEntity*)` (0x61E200)
    fn use_gun_require_pistol_whip(ped: GPtr<u8>, target: GPtr<u8>) -> u8 = cdecl 0x61E200 => unported;
    /// `CTaskSimpleUseGun::CTaskSimpleUseGun(CEntity*, CVector, uint8 command, uint16 burst, bool aimImmediate)`
    /// (0x61DE60, `ret 0x1C`; the command goes as a whole dword)
    fn use_gun_ctor(t: GPtr<u8>, target: GPtr<u8>, x: u32, y: u32, z: u32, cmd: u32, burst: u32, aim: u32) -> GPtr<u8> = thiscall 0x61DE60 => unported;
    /// `CTaskComplexUseGoggles::CTaskComplexUseGoggles()` (0x634EF0)
    fn use_goggles_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x634EF0 => unported;
    /// `CWeapon::Fire(CEntity*, CVector* origin, CVector* muzzle, CEntity* target, CVector* targetPos, CVector*)`
    /// (0x742300, `ret 0x18`)
    fn weapon_fire(w: GPtr<u8>, owner: GPtr<u8>, origin: GPtr<u8>, muzzle: GPtr<u8>, target: GPtr<u8>, tpos: GPtr<u8>, alt: GPtr<u8>) -> u8 = thiscall 0x742300 => unported;
    /// `CPedIntelligence::GetTaskThrow()` (0x600FB0)
    fn intel_get_task_throw(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x600FB0 => unported;
    /// `CTaskSimpleThrowProjectile::ControlThrow(bool, CEntity*, CVector*)` (0x61F810, `ret 0xC`)
    fn throw_control_throw(t: GPtr<u8>, b: u32, target: GPtr<u8>, pos: GPtr<u8>) -> u8 = thiscall 0x61F810 => unported;
    /// `CTaskSimpleThrowProjectile::CTaskSimpleThrowProjectile(CEntity*, CVector)` (0x61F660, `ret 0x10`)
    fn throw_projectile_ctor(t: GPtr<u8>, target: GPtr<u8>, x: u32, y: u32, z: u32) -> GPtr<u8> = thiscall 0x61F660 => unported;
    /// `CMatrix::operator*(CVector* out, const CMatrix&, const CVector&)` (0x59C890)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CWeaponInfo::GetCrouchReloadAnimationID()` (0x685700)
    fn weapon_info_get_crouch_reload_anim_id(wi: GPtr<u8>) -> u32 = thiscall 0x685700 => unported;
    /// `CPad::AimWeaponLeftRight(CPed*)` (0x541040, `ret 4`; int16 in AX)
    fn pad_aim_weapon_left_right(pad: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x541040 => unported;
    /// `CPad::AimWeaponUpDown(CPed*)` (0x5410C0, `ret 4`; int16 in AX)
    fn pad_aim_weapon_up_down(pad: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x5410C0 => unported;
    /// `CGameLogic::IsCoopGameGoingOn()` (0x441390)
    fn game_logic_is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CPlayerPed::PedCanBeTargettedVehicleWise(CPed*)` (0x609D90)
    fn player_ped_can_be_targetted_vehicle_wise(p: GPtr<u8>) -> u8 = cdecl 0x609D90 => unported;
    /// `CLocalisation::KickingWhenDown()` (0x56D270)
    fn localisation_kicking_when_down() -> u8 = cdecl 0x56D270 => unported;
    /// `CPlayerPed::DoesTargetHaveToBeBroken(CEntity*, CWeapon*)` (0x60C0C0, `ret 8`)
    fn player_does_target_have_to_be_broken(p: GPtr<u8>, t: GPtr<u8>, w: GPtr<u8>) -> u8 = thiscall 0x60C0C0 => unported;
    /// `CPad::ShiftTargetLeftJustDown()` (0x540850; AL, EAX passed on whole once)
    fn pad_shift_target_left_just_down(pad: GPtr<u8>) -> u32 = thiscall 0x540850 => unported;
    /// `CPad::ShiftTargetRightJustDown()` (0x540880)
    fn pad_shift_target_right_just_down(pad: GPtr<u8>) -> u8 = thiscall 0x540880 => unported;
    /// `CPlayerPed::FindNextWeaponLockOnTarget(CEntity*, bool)` (0x60E530, `ret 8`)
    fn player_find_next_weapon_lock_on_target(p: GPtr<u8>, t: GPtr<u8>, left: u32) -> u8 = thiscall 0x60E530 => unported;
    /// `CPlayerPed::FindWeaponLockOnTarget()` (0x60DC50)
    fn player_find_weapon_lock_on_target(p: GPtr<u8>) -> u8 = thiscall 0x60DC50 => unported;
    /// `CPedIntelligence::IsInSeeingRange(const CVector&)` (0x600C60, `ret 4`)
    fn intel_is_in_seeing_range(i: GPtr<u8>, pos: GPtr<u8>) -> u8 = thiscall 0x600C60 => unported;
    /// `CPedIntelligence::GetActivePrimaryTask()` (0x4B85B0)
    fn intel_get_active_primary_task(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B85B0 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    fn ped_groups_get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroups::AreInSameGroup(const CPed*, const CPed*)` (0x5F7F40)
    fn ped_groups_are_in_same_group(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x5F7F40 => unported;
    /// `CEventGunAimedAt::CEventGunAimedAt(CPed*)` (0x4B0700, `ret 4`)
    fn event_gun_aimed_at_ctor(e: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B0700 => unported;
    /// `CEventGunAimedAt::~CEventGunAimedAt()` (0x4B07B0)
    fn event_gun_aimed_at_dtor(e: GPtr<u8>) = thiscall 0x4B07B0 => unported;
    /// `CEventGroupEvent::CEventGroupEvent(CPed*, CEvent*)` (0x4ADFD0, `ret 8`)
    fn event_group_event_ctor(g: GPtr<u8>, ped: GPtr<u8>, e: GPtr<u8>) -> GPtr<u8> = thiscall 0x4ADFD0 => unported;
    /// `CEventGroupEvent::~CEventGroupEvent()` (0x4AE070)
    fn event_group_event_dtor(g: GPtr<u8>) = thiscall 0x4AE070 => unported;
    /// `CPedGroupIntelligence::AddEvent(CEvent*)` (0x5F7470, `ret 4`)
    fn group_intel_add_event(gi: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x5F7470 => unported;
    /// `CPlayerPed::Compute3rdPersonMouseTarget(bool)` (0x60B650, `ret 4`; the bool goes as a whole dword)
    fn player_compute_3rd_person_mouse_target(p: GPtr<u8>, melee: u32) = thiscall 0x60B650 => unported;
    /// `CCamera::UpdateAimingCoors(const CVector&)` (0x50CB10, `ret 4`)
    fn camera_update_aiming_coors(cam: GPtr<u8>, v: GPtr<u8>) = thiscall 0x50CB10 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00, `ret 4`)
    fn vector_add_assign(v: GPtr<u8>, o: GPtr<u8>) -> GPtr<u8> = thiscall 0x411A00 => unported;
    /// `CTaskSimpleUseGun::AbortIK(CPed*)` (0x61DFA0, `ret 4`)
    fn use_gun_abort_ik(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x61DFA0 => unported;
    /// `CPlayerPed::Clear3rdPersonMouseTarget()` (0x609ED0)
    fn player_clear_3rd_person_mouse_target(p: GPtr<u8>) = thiscall 0x609ED0 => unported;
    /// `CPlaceable::GetTopDirection()` (0x41CCB0, `ret 4`; returns the out vector)
    fn placeable_get_top_direction(p: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x41CCB0 => unported;
    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0, `ret 4`)
    fn ik_is_looking(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::GetLookAtEntity(CPed*)` (0x6181D0, `ret 4`)
    fn ik_get_look_at_entity(m: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x6181D0 => unported;
    /// `IKChainManager_c::LookAt(...)` (0x618970, `ret 0x2C`)
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, pos: u32, b: u32, speed: u32, blend: i32, prio: i32, b2: u32) = thiscall 0x618970 => unported;
    /// `IKChainManager_c::AbortLookAt(CPed*, int32 blendOut)` (0x618280, `ret 8`)
    fn ik_abort_look_at(m: GPtr<u8>, ped: GPtr<u8>, blend: i32) = thiscall 0x618280 => unported;
}

/// `CCamera::m_bUseMouse3rdPerson` (0xB6EC2E)
const CAMERA_USE_MOUSE_3RD_PERSON: usize = 0xB6EC2E;
/// `TheCamera.m_bJustJumpedOutOf1stPersonBecauseOfTarget` (0xB6F048)
const CAMERA_JUMPED_OUT_OF_1ST_PERSON: usize = 0xB6F048;
/// 1.9 (0x866C0C): the stealth-kill move blend limit
const K_STEALTH_MOVE_BLEND: usize = 0x866C0C;
/// -20° in radians (0x8D610C): the fire extinguisher's aim pitch
const K_EXTINGUISHER_PITCH: usize = 0x8D610C;
/// 5.0 (0x858C80): the free-aim distance
const K_FREE_AIM_DISTANCE: usize = 0x858C80;
/// 100.0 (0x858628): the aim stick threshold that breaks a lock
const K_AIM_STICK_BREAK: usize = 0x858628;
/// 50.0 (0x858B40) / -50.0 (0x859B60): the walk stick thresholds of the gun's strafe control
const K_WALK_STICK_POS: usize = 0x858B40;
const K_WALK_STICK_NEG: usize = 0x859B60;
/// The crouch reload switch (0x8D2FE8, byte)
const CROUCH_RELOAD_ENABLED: usize = 0x8D2FE8;
/// "ProcPlyrWeapon" (0x870910)
const S_PROC_PLYR_WEAPON: u32 = 0x870910;
/// `g_ikChainMan` (0xC15448)
const IK_CHAIN_MAN: usize = 0xC15448;

fn tm_secondary(ped: GPtr<u8>, i: u32) -> GPtr<u8> {
    tm_get_task_secondary(task_manager(ped), i)
}
fn is_ped(e: GPtr<u8>) -> bool {
    rd::<u8>(e, 0x36) & 7 == 3
}
fn weapon_slot(ped: GPtr<u8>) -> GPtr<u8> {
    ped.byte_add(0x5A0 + isize::from(rd::<i8>(ped, 0x718)) * 0x1C)
}
fn player_data(ped: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(ped, 0x480)
}
fn or_u32(p: GPtr<u8>, off: isize, v: u32) {
    wr::<u32>(p, off, rd::<u32>(p, off) | v);
}
fn and_u32(p: GPtr<u8>, off: isize, v: u32) {
    wr::<u32>(p, off, rd::<u32>(p, off) & v);
}
fn cam() -> GPtr<u8> {
    GPtr::from_addr(THE_CAMERA)
}

/// The secondary task 0 (use gun) for `cmd`: none → a new `CTaskSimpleUseGun(target, (0, 0, 0), cmd, 1, false)` as
/// secondary 0 (returns true); one not of type 0x3F9 → `MakeAbortable(ped, 1, NULL)`; else, with a use-gun task,
/// `ControlGun(ped, target, cmd)`.
fn ppw_use_gun(ped: GPtr<u8>, cmd: u32) -> bool {
    if tm_secondary(ped, 0).is_null() {
        let t = task_new(0x3C);
        let t = if t.is_null() { t } else { use_gun_ctor(t, rd::<GPtr<u8>>(ped, 0x71C), 0, 0, 0, cmd, 1, 0) };
        tm_set_task_secondary(task_manager(ped), t, 0);
        return true;
    }
    if v_task_type(tm_secondary(ped, 0)) != 0x3F9 {
        v_task_make_abortable(tm_secondary(ped, 0), ped, 1, GPtr::NULL);
    } else if !intel_get_task_use_gun(intel(ped)).is_null() {
        use_gun_control_gun(tm_secondary(ped, 0), ped, rd::<GPtr<u8>>(ped, 0x71C), cmd);
    }
    false
}

/// The melee part of `ProcessPlayerWeapon` (fire type 0).
fn ppw_melee(this: GPtr<u8>, ped: GPtr<u8>, pad: GPtr<u8>, wi: GPtr<u8>) {
    let _ = this;
    let mut s20: u8 = 0;
    let mut bl: u8;
    let target = rd::<GPtr<u8>>(ped, 0x71C);
    let clump = || rd::<u32>(ped, 0x18);
    if target.is_null() && pad_get_target(pad) == 0 && tm_secondary(ped, 0).is_null() {
        bl = 0;
        if pad_melee_attack_just_down(pad, 0) != 0 {
            bl = 0xB;
            s20 = 0xB;
        }
        let a = clump_get_association(clump(), 0x15B);
        if !a.is_null() {
            wr::<u32>(a, 0x18, 0xC000_0000);
        }
        if bl == 0 {
            ppw_fight_task(ped, pad);
            return;
        }
        ppw_start_fight(ped, bl, s20);
        return;
    }
    let mut victim = GPtr::NULL;
    if !target.is_null() && is_ped(target) {
        victim = target;
    }
    if target.is_null() && g::<u8>(CAMERA_USE_MOUSE_3RD_PERSON).read() != 0 && rd::<u32>(ped, 0x79C) != 0 {
        victim = rd::<GPtr<u8>>(ped, 0x79C);
    }
    let stealth_ready = !victim.is_null()
        && pad_get_target(pad) != 0
        && {
            let c = x(rd::<f32>(player_data(ped), 0x14)).fcom(x(g::<f32>(K_STEALTH_MOVE_BLEND).read()));
            !c.test_parity(5)
        }
        && rd::<u32>(ped, 0x534) != 7
        && tm_secondary(ped, 0).is_null()
        && rd::<u32>(wi, 0x1C) != 0
        && {
            let groups = g::<GPtr<u8>>(ANIM_ASSOC_GROUPS).read();
            let block = rd::<GPtr<u8>>(groups, rd::<i32>(wi, 0x1C) as isize * 0x14);
            !block.is_null() && rd::<u8>(block, 0x10) != 0
        }
        && intel_test_for_stealth_kill(intel(ped), victim, 0) != 0;
    let assoc = if stealth_ready {
        let blend = || anim_blend_animation(clump(), rd::<u32>(wi, 0x1C), 0x15B, 0x4100_0000);
        if rd::<u32>(ped, 0x46C) & 0x0400_0000 != 0 {
            if !intel_get_task_duck(intel(ped), 1).is_null() && duck_is_task_in_use_by_other_tasks(intel_get_task_duck(intel(ped), 1)) != 0 {
                clump_get_first_association(clump())
            } else {
                blend()
            }
        } else {
            blend()
        }
    } else {
        let a = clump_get_association(clump(), 0x15B);
        if !a.is_null() {
            wr::<u32>(a, 0x18, 0xC000_0000);
        }
        a
    };
    let mut s10: u8 = 0;
    if rd::<u32>(ped, 0x71C) != 0 || pad_get_target(pad) != 0 {
        s10 = 1;
    }
    bl = pad_melee_attack_just_down(pad, u32::from(s10));
    if bl != 0
        && !assoc.is_null()
        && !x(rd::<f32>(assoc, 0x18)).fcom(x(g::<f32>(K_HALF).read())).test_any(0x41)
        && !victim.is_null()
        && intel_test_for_stealth_kill(intel(ped), victim, 1) != 0
    {
        ppw_stealth_kill(ped, wi, victim);
        return;
    }
    match bl {
        1 => {
            bl = 0xB;
            s20 = 0xB;
        }
        4 => {
            let ty = rd::<u32>(weapon_slot(ped), 0);
            bl = 0xC;
            s20 = 0xC;
            if (rd::<u32>(weapon_info_get(ty, 1), 0x18) >> 9) & 1 != 0 {
                bl = 0xB;
                s20 = 0xB;
            }
        }
        3 => {
            bl = 2;
            s20 = 2;
        }
        _ => {
            if pad_get_melee_attack(pad, 0) != 0 && rd::<u32>(weapon_slot(ped), 0) == 9 && !tm_secondary(ped, 0).is_null() {
                bl = 0xB;
                s20 = 0xB;
            } else {
                ppw_fight_task(ped, pad);
                return;
            }
        }
    }
    ppw_start_fight(ped, bl, s20);
}

/// No secondary task → a new `CTaskSimpleFight(target, (int8) cmd, 2000)` as secondary 0; else, with a fight task,
/// the secondary's `ControlFight(target, cmd)`.
fn ppw_start_fight(ped: GPtr<u8>, bl: u8, s20: u8) {
    if tm_secondary(ped, 0).is_null() {
        let t = task_new(0x28);
        let t = if t.is_null() { t } else { fight_ctor(t, rd::<GPtr<u8>>(ped, 0x71C), bl as i8 as i32 as u32, 2000) };
        tm_set_task_secondary(task_manager(ped), t, 0);
    } else if !intel_get_task_fighting(intel(ped)).is_null() {
        fight_control_fight(tm_secondary(ped, 0), rd::<GPtr<u8>>(ped, 0x71C), u32::from(s20));
    }
}

/// An existing fight task: move state 1 and sprint → `ControlFight(target, 0xF)`; else `ControlFight(target, player
/// data +0x20 == the current slot ? 0 : 1)`.
fn ppw_fight_task(ped: GPtr<u8>, pad: GPtr<u8>) {
    if intel_get_task_fighting(intel(ped)).is_null() {
        return;
    }
    let f = tm_secondary(ped, 0);
    if rd::<u32>(ped, 0x534) == 1 && pad_get_sprint(pad) != 0 {
        fight_control_fight(f, rd::<GPtr<u8>>(ped, 0x71C), 0xF);
    } else {
        let cmd = u32::from(rd::<u8>(player_data(ped), 0x20) != rd::<u8>(ped, 0x718));
        fight_control_fight(f, rd::<GPtr<u8>>(ped, 0x71C), cmd);
    }
}

/// The stealth kill: a new `CTaskSimpleStealthKill(true, victim, anim group)` as primary task 3; a stack
/// `CPedDamageResponseCalculator(ped, 0, weapon, 3, false)` and `CEventDamage(ped, now, weapon, 3, 0, false, victim
/// +0x46C >> 8 & 0xFFFFFF01)`; the event affecting the victim → `ComputeDamageResponse(victim, event +0x38, false)`,
/// the victim's event group (+0x68) `Add(event)`, `ReportCrime(0x12, victim, ped)`, the weapon audio event 0x9C;
/// `ClearWeaponTarget`; both destructors.
fn ppw_stealth_kill(ped: GPtr<u8>, wi: GPtr<u8>, victim: GPtr<u8>) {
    let t = task_new(0x20);
    let t = if t.is_null() { t } else { stealth_kill_ctor(t, 1, victim, rd::<u32>(wi, 0x1C)) };
    tm_set_task(task_manager(ped), t, 3, 0);
    let mut calc = [0u32; 9];
    let mut ev = [0u32; 0x11];
    let calc_p: GPtr<u8> = GPtr::from_raw(calc.as_mut_ptr()).cast();
    let ev_p: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    damage_calc_ctor(calc_p, ped, 0, rd::<u32>(weapon_slot(ped), 0), 3, 0);
    let flags = (rd::<u32>(victim, 0x46C) >> 8) & 0xFFFF_FF01;
    event_damage_ctor(ev_p, ped, g::<u32>(TIMER_TIME_MS).read(), rd::<u32>(weapon_slot(ped), 0), 3, 0, 0, flags);
    if event_damage_affects_ped(ev_p, victim) != 0 {
        damage_calc_compute_damage_response(calc_p, victim, ev_p.byte_add(0x38), 0);
        event_group_add(intel(victim).byte_add(0x68), ev_p, 0);
        crime_report_crime(0x12, victim, ped);
        ped_weapon_audio_add_audio_event(ped.byte_add(0x394), 0x9C);
    }
    player_clear_weapon_target(ped);
    event_damage_dtor(ev_p);
    damage_calc_dtor(calc_p);
}

/// The ranged part of `ProcessPlayerWeapon` (fire types 1..4).
fn ppw_gun(ped: GPtr<u8>, pad: GPtr<u8>, wi: GPtr<u8>) {
    if pad_get_weapon(pad, ped) == 0 {
        if !intel_get_task_throw(intel(ped)).is_null() {
            throw_control_throw(tm_secondary(ped, 0), 1, GPtr::NULL, GPtr::NULL);
        }
        return;
    }
    if rd::<u32>(ped, 0x534) == 7 || rd::<u8>(player_data(ped), 0x20) != rd::<u8>(ped, 0x718) {
        return;
    }
    match rd::<u32>(wi, 0).wrapping_sub(1) {
        0 | 2 => {
            let mut s20: u8 = 2;
            if use_gun_require_pistol_whip(ped, rd::<GPtr<u8>>(ped, 0x71C)) != 0 {
                s20 = 5;
            } else if rd::<u32>(weapon_slot(ped), 4) == 2 {
                if pad_get_target(pad) != 0 || rd::<u32>(ped, 0x71C) != 0 || rd::<u8>(player_data(ped), 0x34) & 8 != 0 {
                    s20 = 1;
                } else {
                    return;
                }
            }
            if ppw_use_gun(ped, u32::from(s20)) {
                wr::<u32>(player_data(ped), 0x2C, 0);
            }
            if pad_get_target(pad) == 0 {
                if rd::<u32>(weapon_slot(ped), 0) == 0x2A {
                    wr::<f32>(player_data(ped), 0x54, x(g::<f32>(K_EXTINGUISHER_PITCH).read()).fchs().to_f32());
                } else {
                    wr::<u32>(player_data(ped), 0x54, 0);
                }
            }
        }
        1 => {
            let ty = rd::<u32>(weapon_slot(ped), 0);
            if ty == 0x23 || ty == 0x24 {
                let s20: u8 = if rd::<u32>(weapon_slot(ped), 4) == 2 { 1 } else { 2 };
                ppw_use_gun(ped, u32::from(s20));
                return;
            }
            if tm_secondary(ped, 0).is_null() && pad_weapon_just_down(pad, ped) as u8 != 0 {
                let t = task_new(0x24);
                let t = if t.is_null() { t } else { throw_projectile_ctor(t, GPtr::NULL, 0, 0, 0) };
                tm_set_task_secondary(task_manager(ped), t, 0);
                return;
            }
            if !tm_secondary(ped, 0).is_null() && v_task_type(tm_secondary(ped, 0)) != 0x3FA {
                v_task_make_abortable(tm_secondary(ped, 0), ped, 1, GPtr::NULL);
                return;
            }
            if !intel_get_task_throw(intel(ped)).is_null() {
                let t = tm_secondary(ped, 0);
                let jd = pad_weapon_just_down(pad, ped);
                throw_control_throw(t, jd, GPtr::NULL, GPtr::NULL);
            }
        }
        3 => {
            let active = usize::from(g::<u8>(CAMERA_ACTIVE_CAM).read());
            if g::<u16>(CAMERA_CAM0_MODE + active * 0x238).read() != 0x2E {
                return;
            }
            if g::<u32>(TIMER_TIME_MS).read() <= rd::<u32>(weapon_slot(ped), 0x10) {
                return;
            }
            let mut v = [0u32, 0, 0x3F19_999A];
            let mut out = [0u32; 3];
            let r = matrix_mul_vec(GPtr::from_raw(out.as_mut_ptr()).cast(), rd::<GPtr<u8>>(ped, 0x14), GPtr::from_raw(v.as_mut_ptr()).cast());
            let mut fp = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
            weapon_fire(weapon_slot(ped), ped, GPtr::from_raw(fp.as_mut_ptr()).cast(), GPtr::NULL, GPtr::NULL, GPtr::NULL, GPtr::NULL);
        }
        _ => {}
    }
}

/// The "use" part of `ProcessPlayerWeapon` (fire type 5): `WeaponJustDown(NULL)`; the detonator (0x28) fires at the
/// ped's position; night/thermal goggles (0x2C, 0x2D) with no primary task in intelligence +0x10 → a new
/// `CTaskComplexUseGoggles` as primary task 3 and player data +0x85 = 1.
fn ppw_use(ped: GPtr<u8>, pad: GPtr<u8>) {
    if pad_weapon_just_down(pad, GPtr::NULL) as u8 == 0 {
        return;
    }
    let ty = rd::<i32>(weapon_slot(ped), 0);
    if ty == 0x28 {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
        weapon_fire(weapon_slot(ped), ped, pos, pos, GPtr::NULL, GPtr::NULL, GPtr::NULL);
    } else if ty > 0x2B && ty <= 0x2D && rd::<u32>(intel(ped), 0x10) == 0 {
        let t = task_new(0xC);
        let t = if t.is_null() { t } else { use_goggles_ctor(t) };
        tm_set_task(task_manager(ped), t, 3, 0);
        wr::<u8>(player_data(ped), 0x85, 1);
    }
}

/// Reloading (slot state 2) with weapon info flag bit 12: a use-gun task → `ControlGun(ped, target, 4)`; else the
/// crouch reload: ducking with `GetCrouchReloadAnimationID()`, the switch 0x8D2FE8, a duck task not in use by other
/// tasks and that anim not playing → `BlendAnimation(clump, group, id, 4.0)`; standing: anim 0xE2 (flag bit 12) or 0
/// not playing → blend it.
fn ppw_reload(ped: GPtr<u8>, wi: GPtr<u8>) {
    if rd::<u32>(weapon_slot(ped), 4) != 2 || (rd::<u32>(wi, 0x18) >> 0xC) & 1 == 0 {
        return;
    }
    if !intel_get_task_use_gun(intel(ped)).is_null() {
        use_gun_control_gun(tm_secondary(ped, 0), ped, rd::<GPtr<u8>>(ped, 0x71C), 4);
        return;
    }
    if rd::<u32>(ped, 0x46C) & 0x0400_0000 != 0 {
        if weapon_info_get_crouch_reload_anim_id(wi) != 0
            && g::<u8>(CROUCH_RELOAD_ENABLED).read() != 0
            && !intel_get_task_duck(intel(ped), 1).is_null()
            && duck_is_task_in_use_by_other_tasks(intel_get_task_duck(intel(ped), 1)) == 0
            && clump_get_association(rd::<u32>(ped, 0x18), weapon_info_get_crouch_reload_anim_id(wi)).is_null()
        {
            let group = rd::<u32>(wi, 0x1C);
            let id = weapon_info_get_crouch_reload_anim_id(wi);
            anim_blend_animation(rd::<u32>(ped, 0x18), group, id, 0x4080_0000);
        }
    } else {
        let id = |wi: GPtr<u8>| if (rd::<u32>(wi, 0x18) >> 0xC) & 1 != 0 { 0xE2 } else { 0 };
        if clump_get_association(rd::<u32>(ped, 0x18), id(wi)).is_null() {
            anim_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(wi, 0x1C), id(wi), 0x4080_0000);
        }
    }
}

/// The `test ah, 0x41` / `je` "greater" test of `|(int16) v| > limit`.
fn stick_above(v: u32, limit: f32) -> bool {
    !X87::from_i32(i32::from(v as i16)).fabs().fcom(x(limit)).test_any(0x41)
}

/// The aiming part of `ProcessPlayerWeapon` (from 0x686897).
fn ppw_aim(this: GPtr<u8>, ped: GPtr<u8>, pad: GPtr<u8>, wi: GPtr<u8>) {
    let target = || rd::<GPtr<u8>>(ped, 0x71C);
    let pd = || player_data(ped);
    let mouse = || g::<u8>(CAMERA_USE_MOUSE_3RD_PERSON).read() != 0;
    let aiming = pad_get_target(pad) != 0
        && rd::<u8>(pd(), 0x20) == rd::<u8>(ped, 0x718)
        && !(rd::<u32>(ped, 0x534) == 7 && rd::<u32>(wi, 0) != 0)
        && camera_using_1st_person_weapon_mode(cam()) == 0;
    if !aiming {
        ppw_not_aiming(ped, pad, wi);
        return;
    }
    let can_aim = rd::<u8>(wi, 0x18) & 1 != 0;
    if can_aim && rd::<u8>(pd(), 0x34) & 8 == 0 {
        if target().is_null() {
            if mouse() {
                player_clear_weapon_target(ped);
            } else if pad_get_enter_targeting(pad) != 0
                || g::<u8>(CAMERA_JUMPED_OUT_OF_1ST_PERSON).read() != 0
                || rd::<u32>(this, 0xC) < g::<u32>(TIMER_FRAME_COUNTER).read().wrapping_sub(1)
            {
                player_find_weapon_lock_on_target(ped);
            }
        } else {
            let lim = g::<f32>(K_AIM_STICK_BREAK).read();
            let moved = stick_above(pad_aim_weapon_left_right(pad, ped), lim) || stick_above(pad_aim_weapon_up_down(pad, ped), lim);
            let keep = (!moved || game_logic_is_coop_game_going_on() != 0)
                && !target().is_null()
                && g::<u8>(CAMERA_USE_MOUSE_3RD_PERSON).read() != 1
                && !(is_ped(target())
                    && (player_ped_can_be_targetted_vehicle_wise(target()) == 0
                        || (localisation_kicking_when_down() == 0 && matches!(rd::<u32>(target(), 0x530), 0x36 | 0x37))))
                && player_does_target_have_to_be_broken(ped, target(), weapon_slot(ped)) == 0
                && (rd::<u8>(ped, 0x46C) & 0x40 != 0 || {
                    let ty = rd::<u32>(weapon_slot(ped), 0);
                    rd::<u8>(weapon_info_get(ty, ped_get_weapon_skill(ped)), 0x18) & 1 != 0
                });
            if !keep {
                player_clear_weapon_target(ped);
                or_u32(pd(), 0x34, 8);
            }
            if !target().is_null() {
                if pad_shift_target_left_just_down(pad) as u8 != 0 {
                    player_find_next_weapon_lock_on_target(ped, target(), 1);
                }
                if pad_shift_target_right_just_down(pad) != 0 {
                    player_find_next_weapon_lock_on_target(ped, target(), 0);
                }
            }
            let ty = rd::<u32>(weapon_slot(ped), 0);
            let victim = target();
            if rd::<u32>(weapon_info_get(ty, 1), 0) == 1
                && !victim.is_null()
                && is_ped(victim)
                && intel_is_in_seeing_range(intel(victim), entity_pos(ped)) != 0
                && {
                    let t = intel_get_active_primary_task(intel(victim));
                    t.is_null() || v_task_type(t) != 0x259
                }
            {
                if rd::<u32>(weapon_slot(ped), 0) != 0x17 {
                    ped_say(ped, 0xB0, 0, 0x3F80_0000, 0, 0, 0);
                }
                let grp = ped_groups_get_peds_group(victim);
                if !grp.is_null() {
                    if ped_groups_are_in_same_group(victim, ped) == 0 {
                        let e = event_new(0x18);
                        let e = if e.is_null() { e } else { event_gun_aimed_at_ctor(e, ped) };
                        let mut ge = [0u32; 0x10];
                        let ge_p: GPtr<u8> = GPtr::from_raw(ge.as_mut_ptr()).cast();
                        event_group_event_ctor(ge_p, victim, e);
                        group_intel_add_event(grp.byte_add(0x30), ge_p);
                        event_group_event_dtor(ge_p);
                    }
                } else {
                    let mut ev = [0u32; 0x10];
                    let ev_p: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
                    event_gun_aimed_at_ctor(ev_p, ped);
                    event_group_add(intel(victim).byte_add(0x68), ev_p, 0);
                    event_gun_aimed_at_dtor(ev_p);
                }
            }
        }
        if target().is_null() {
            or_u32(pd(), 0x34, 8);
        }
    } else if rd::<u8>(pd(), 0x34) & 8 != 0
        && can_aim
        && (pad_shift_target_left_just_down(pad) as u8 != 0 || pad_shift_target_right_just_down(pad) != 0)
        && !mouse()
    {
        if !target().is_null() {
            if pad_shift_target_left_just_down(pad) as u8 != 0 {
                player_find_next_weapon_lock_on_target(ped, target(), 1);
            }
            if pad_shift_target_right_just_down(pad) != 0 {
                player_find_next_weapon_lock_on_target(ped, target(), 0);
            }
        } else {
            let left = pad_shift_target_left_just_down(pad);
            player_find_next_weapon_lock_on_target(ped, GPtr::NULL, left);
        }
    } else if (rd::<u32>(wi, 0x18) >> 3) & 1 != 0 && target().is_null() && rd::<u8>(pd(), 0x34) & 8 == 0 {
        or_u32(pd(), 0x34, 8);
    } else if mouse() && rd::<u8>(pd(), 0x34) & 8 != 0 {
        player_compute_3rd_person_mouse_target(ped, u32::from(rd::<u32>(wi, 0) != 0));
    }
    if target().is_null() {
        if matches!(rd::<u32>(wi, 0), 0 | 2 | 5) {
            return;
        }
    } else if rd::<u32>(wi, 0) != 0 || rd::<u32>(ped, 0x46C) & 0x0400_0000 != 0 {
        and_u32(pd(), 0x34, !8);
    }
    camera_set_new_player_weapon_mode(cam(), 0x35, 0, 0);
    if !target().is_null() {
        camera_update_aiming_coors(cam(), entity_pos(target()));
    } else {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let fwd = [rd::<f32>(m, 0x10), rd::<f32>(m, 0x14), rd::<f32>(m, 0x18)];
        let z = x(rd::<f32>(pd(), 0x54)).fsin().fadd_m(fwd[2]);
        let k = g::<f32>(K_FREE_AIM_DISTANCE).read();
        let mut v = [x(fwd[0]).fmul_m(k).to_f32(), x(fwd[1]).fmul_m(k).to_f32(), z.fmul_m(k).to_f32()];
        let vp: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr()).cast();
        vector_add_assign(vp, entity_pos(ped));
        camera_update_aiming_coors(cam(), vp);
    }
    ppw_use_gun_aim(ped);
}

/// No secondary task → a new `CTaskSimpleUseGun(target, (0, 0, 0), 1, 1, false)` as secondary 0; else, with a use-gun
/// task, the secondary's `ControlGun(ped, target, 1)`.
fn ppw_use_gun_aim(ped: GPtr<u8>) {
    if tm_secondary(ped, 0).is_null() {
        let t = task_new(0x3C);
        let t = if t.is_null() { t } else { use_gun_ctor(t, rd::<GPtr<u8>>(ped, 0x71C), 0, 0, 0, 1, 1, 0) };
        tm_set_task_secondary(task_manager(ped), t, 0);
    } else if !intel_get_task_use_gun(intel(ped)).is_null() {
        use_gun_control_gun(tm_secondary(ped, 0), ped, rd::<GPtr<u8>>(ped, 0x71C), 1);
    }
}

/// Not aiming (from 0x686EBC).
fn ppw_not_aiming(ped: GPtr<u8>, pad: GPtr<u8>, wi: GPtr<u8>) {
    let pd = || player_data(ped);
    let target = || rd::<GPtr<u8>>(ped, 0x71C);
    if (pad_get_target(pad) != 0 || rd::<u32>(ped, 0xFC) != 0)
        && rd::<u8>(pd(), 0x20) == rd::<u8>(ped, 0x718)
        && rd::<u32>(ped, 0x534) != 7
        && camera_using_1st_person_weapon_mode(cam()) != 0
    {
        ppw_use_gun_aim(ped);
        return;
    }
    if (!target().is_null() || rd::<u8>(pd(), 0x34) & 8 != 0) && !intel_get_task_fighting(intel(ped)).is_null() {
        let cmd = if x(rd::<f32>(pd(), 0x10)).fcom(x(g::<f32>(K_FIGHT_FORWARD).read())).test_parity(5) { 0xF } else { 0x10 };
        fight_control_fight(intel_get_task_fighting(intel(ped)), GPtr::NULL, cmd);
    }
    if !intel_get_task_use_gun(intel(ped)).is_null() {
        let strafe = pad_get_weapon(pad, GPtr::NULL) == 0 && {
            let pos = g::<f32>(K_WALK_STICK_POS).read();
            let neg = g::<f32>(K_WALK_STICK_NEG).read();
            let above = |v: u32| !X87::from_i32(i32::from(v as i16)).fcom(x(pos)).test_any(0x41);
            let below = |v: u32| !X87::from_i32(i32::from(v as i16)).fcom(x(neg)).test_parity(5);
            above(pad_get_ped_walk_up_down(pad))
                || below(pad_get_ped_walk_up_down(pad))
                || above(pad_get_ped_walk_left_right(pad))
                || below(pad_get_ped_walk_left_right(pad))
        };
        if strafe {
            use_gun_control_gun(intel_get_task_use_gun(intel(ped)), ped, target(), 7);
        } else {
            use_gun_player_passive_control_gun(intel_get_task_use_gun(intel(ped)));
        }
        if (rd::<u32>(wi, 0x18) >> 1) & 1 == 0 {
            let c = CAMERA_CAM0_MODE + usize::from(g::<u8>(CAMERA_ACTIVE_CAM).read()) * 0x238;
            if g::<u16>(c).read() == 4 {
                let a = x(g::<f32>(c + 0x184).read()).fchs().fpatan(x(g::<f32>(c + 0x188).read()));
                wr::<f32>(ped, 0x55C, a.to_f32());
            }
        }
        if !target().is_null() || rd::<u8>(pd(), 0x34) & 8 != 0 {
            use_gun_abort_ik(intel_get_task_use_gun(intel(ped)), ped);
        }
    }
    if !target().is_null() {
        player_clear_weapon_target(ped);
    }
    player_clear_3rd_person_mouse_target(ped);
    and_u32(pd(), 0x34, !8);
}

/// The look-at part of `ProcessPlayerWeapon` (from 0x6870FC).
fn ppw_look(this: GPtr<u8>, ped: GPtr<u8>, wi: GPtr<u8>) {
    let pd = player_data(ped);
    let has = u32::from(rd::<u32>(ped, 0x71C) != 0) << 2;
    wr::<u32>(pd, 0x34, (rd::<u32>(pd, 0x34) & !4) | has);
    let t = rd::<GPtr<u8>>(ped, 0x71C);
    let mut abort_check = t.is_null();
    if !t.is_null() {
        let special = is_ped(t) && (rd::<u32>(t, 0x474) & 0x10_0000 != 0 || rd::<u32>(t, 0x530) == 0x37);
        let look = if rd::<u32>(wi, 0) == 0 && (intel_get_task_fighting(intel(ped)).is_null() || special) {
            true
        } else {
            rd::<u32>(ped, 0x46C) & 0x0400_0000 == 0 && (rd::<u32>(wi, 0x18) >> 1) & 1 != 0
        };
        abort_check = true;
        if look {
            let (pp, tp) = (entity_pos(ped), entity_pos(t));
            let d = [
                x(rd::<f32>(tp, 0)).fsub_m(rd::<f32>(pp, 0)).to_f32(),
                x(rd::<f32>(tp, 4)).fsub_m(rd::<f32>(pp, 4)).to_f32(),
                x(rd::<f32>(tp, 8)).fsub_m(rd::<f32>(pp, 8)).to_f32(),
            ];
            let mut top = [0u32; 3];
            let up = placeable_get_top_direction(ped, GPtr::from_raw(top.as_mut_ptr()).cast());
            let dot = x(d[2]).fmul_m(rd::<f32>(up, 8)).fadd(x(d[1]).fmul_m(rd::<f32>(up, 4))).fadd(x(d[0]).fmul_m(rd::<f32>(up, 0)));
            if !dot.fcom(x(g::<f32>(K_ZERO_F32).read())).test_any(0x41) {
                abort_check = false;
                let ik = GPtr::from_addr(IK_CHAIN_MAN);
                if ik_is_looking(ik, ped) == 0 || ik_get_look_at_entity(ik, ped) != t {
                    let bone = if is_ped(t) { 5 } else { -1 };
                    ik_look_at(ik, S_PROC_PLYR_WEAPON, ped, t, 9_999_999, bone, 0, 0, 0x3E80_0000, 500, 3, 0);
                }
            }
        }
    }
    if abort_check {
        let ik = GPtr::from_addr(IK_CHAIN_MAN);
        if !rd::<GPtr<u8>>(this, 0x18).is_null()
            && ik_is_looking(ik, ped) != 0
            && ik_get_look_at_entity(ik, ped) == rd::<GPtr<u8>>(this, 0x18)
        {
            ik_abort_look_at(ik, ped, 250);
        }
    }
    wr::<GPtr<u8>>(this, 0x18, t);
}

/// `CTaskSimplePlayerOnFoot::ProcessPlayerWeapon(CPlayerPed*)` (0x6859A0, thiscall, `ret 4`). x87 (FSIN, FPATAN).
///
/// `wi` = `GetWeaponInfo(current type, GetWeaponSkill())`. Player data +0x34 bit 2 with no lock target (+0x71C) →
/// `ClearPlayerWeaponMode`, `ClearCrossHair(+0x598)`. Accuracy +0x71A = 95 with `wi` flag bit 0, else 100.
/// `WeaponJustDown(ped)` with a lock target or (mouse 3rd person 0xB6EC2E and +0x79C) → `PlayerHasJustAttackedSomeone`.
///
/// 1st-person weapons (`wi` flag bit 2) without a fire (+0x730): entering targeting (`GetEnterTargeting`, 0xB6F048, or
/// `GetTarget` with this +0xC below the frame counter - 1) → `SetNewPlayerWeaponMode` 8 (0x23), 0x33 (0x24; player data
/// +0x94 = now, +0x98 = 0), 7 (0x22), 0x2E (0x2B) or 0x22, `SetPedState(0xC)`, done; else, not in 1st-person mode and one
/// of those four weapons → `PlayerPassiveControlGun`, player data +0x34 bit 2 cleared, done. Others: no `GetTarget` with
/// weapon 0x24 → player data +0x94 = +0x98 = 0.
///
/// Then by fire type (`wi` +0): 0 melee (stealth kill with a damage event on the stack, `CTaskSimpleFight` creation or
/// `ControlFight`), 5 use (detonator `Fire`, goggles task), else the ranged control (`CTaskSimpleUseGun` /
/// `CTaskSimpleThrowProjectile` creation, `MakeAbortable`, `ControlGun`, `ControlThrow`, camera `Fire`); then the
/// reload animations; the aiming (lock-on, breaking the lock, shifting targets, gun-aimed-at events, aiming
/// coordinates, the use-gun task) or not-aiming control; then player data +0x34 bit 2 = has a lock target, the IK look
/// at the target ("ProcPlyrWeapon", bone 5 for peds) or the abort of the previous look (this +0x18), this +0x18 = the
/// target. See the helper functions for each part; byte commands go to their callees zero-extended (the original
/// passes them over uninitialised stack dwords).
#[hook(addr = 0x6859a0, conv = thiscall, name = "CTaskSimplePlayerOnFoot::ProcessPlayerWeapon", expect = "6A FF 68 CE 6B 84 00",
       unverified = "mvpk-ai-3", route = "L2", hazards = "x87-transcendental,uninit-locals,heap-writes", recs = "grove-walk,weapon-combat",
       note = "byte commands over uninitialised stack dwords (upper bytes) to MeleeAttackJustDown, ControlFight and the UseGun constructor; the stealth-kill CEventDamage (0x44) and damage calculator and the gun-aimed-at events live on the Rust stack; the chunk at 0x6860AD is a no-CD fragment (movsx of the slot)")]
pub fn on_foot_process_player_weapon_port(this: GPtr<u8>, ped: GPtr<u8>) {
    let pad = player_get_pad(ped);
    let ty = rd::<u32>(weapon_slot(ped), 0);
    let wi = weapon_info_get(ty, ped_get_weapon_skill(ped));
    let pd = || player_data(ped);
    if rd::<u8>(pd(), 0x34) & 4 != 0 && rd::<u32>(ped, 0x71C) == 0 {
        camera_clear_player_weapon_mode(cam());
        weapon_effects_clear_cross_hair(rd::<u32>(ped, 0x598));
    }
    wr::<u8>(ped, 0x71A, if rd::<u8>(wi, 0x18) & 1 != 0 { 0x5F } else { 0x64 });
    if pad_weapon_just_down(pad, ped) as u8 != 0
        && (rd::<u32>(ped, 0x71C) != 0 || (g::<u8>(CAMERA_USE_MOUSE_3RD_PERSON).read() != 0 && rd::<u32>(ped, 0x79C) != 0))
    {
        player_has_just_attacked_someone(ped);
    }
    if rd::<u32>(ped, 0x730) == 0 && (rd::<u32>(wi, 0x18) >> 2) & 1 != 0 {
        let enter = pad_get_enter_targeting(pad) != 0
            || g::<u8>(CAMERA_JUMPED_OUT_OF_1ST_PERSON).read() != 0
            || (pad_get_target(pad) != 0 && rd::<u32>(this, 0xC) < g::<u32>(TIMER_FRAME_COUNTER).read().wrapping_sub(1));
        if enter {
            let mode = match rd::<u32>(weapon_slot(ped), 0) {
                0x23 => 8,
                0x24 => {
                    wr::<u32>(pd(), 0x94, g::<u32>(TIMER_TIME_MS).read());
                    wr::<u32>(pd(), 0x98, 0);
                    0x33
                }
                0x22 => 7,
                0x2B => 0x2E,
                _ => 0x22,
            };
            camera_set_new_player_weapon_mode(cam(), mode, 0, 0);
            ped_set_ped_state(ped, 0xC);
            return;
        }
        if camera_using_1st_person_weapon_mode(cam()) == 0 && matches!(rd::<u32>(weapon_slot(ped), 0), 0x23 | 0x24 | 0x22 | 0x2B) {
            if !intel_get_task_use_gun(intel(ped)).is_null() {
                use_gun_player_passive_control_gun(intel_get_task_use_gun(intel(ped)));
            }
            and_u32(pd(), 0x34, !4);
            return;
        }
    } else if pad_get_target(pad) == 0 && rd::<u32>(weapon_slot(ped), 0) == 0x24 {
        wr::<u32>(pd(), 0x94, 0);
        wr::<u32>(pd(), 0x98, 0);
    }
    match rd::<u32>(wi, 0) {
        0 => ppw_melee(this, ped, pad, wi),
        5 => ppw_use(ped, pad),
        _ => ppw_gun(ped, pad, wi),
    }
    ppw_reload(ped, wi);
    ppw_aim(this, ped, pad, wi);
    ppw_look(this, ped, wi);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_ON_FOOT_PROCESS_PLAYER_WEAPON_PORT,
    HOOK_ON_FOOT_PLAYER_CONTROL_FIGHTER_PORT,
    HOOK_ON_FOOT_PLAYER_CONTROL_ZELDA_WEAPON_PORT,
    HOOK_ON_FOOT_PLAYER_CONTROL_DUCKED_PORT,
    HOOK_ON_FOOT_PROCESS_PED,
    HOOK_ON_FOOT_MAKE_ABORTABLE,
    HOOK_ON_FOOT_PLAYER_CONTROL_ZELDA,
    HOOK_ON_FOOT_PLAY_IDLE_ANIMATIONS,
];
