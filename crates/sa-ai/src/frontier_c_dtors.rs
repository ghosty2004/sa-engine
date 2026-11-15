//! Complete-object destructors of AI tasks that the catalogue had unnamed (FRONTIER-2 lane C, batch `frontier2-c1`).
//! Each class comes from its vtable (slot 4 `GetTaskType` against gta-reversed's `eTaskType`), the vtable the body
//! installs first and the scalar deleting destructor in slot 0 that calls it. Ported from the 1.0 US bytes; the SEH
//! frames of the originals are not modelled (nothing in between throws). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::*;

use crate::bindings::{dtor_669d20, simple_anim_dtor, stand_still_dtor, task_complex_dtor, task_simple_dtor};
use crate::dtors::dtor_460d80;
use crate::fight::duck_dtor_orig;
use crate::geh::wait_area_dtor;
use sa_anim::bindings::anim_set_delete_callback;
use sa_anim::bindings::assoc_set_finish_callback as anim_set_finish_callback;
use sa_core::bindings::eh_vector_dtor;
use sa_stream::bindings::anim_remove_block_ref;
use sa_world::bindings::{enter_car_dtor, entity_clean_up_old_reference};

orig! {
    /// `CTaskSimpleDie::~CTaskSimpleDie` (0x62FB40, the complete-object destructor)
    fn die_dtor(t: GPtr<u8>) = thiscall 0x62FB40 => unported;
    /// `CPatrolRoute::operator delete` (0x41B820)
    fn patrol_route_delete(p: GPtr<u8>) = cdecl 0x41B820 => unported;
}

/// `CAnimBlendAssociation`'s default callback (`CDefaultAnimCallback::DefaultAnimCB`).
const DEFAULT_ANIM_CB: u32 = 0x4D1390;
/// `CAnimManager::ms_aAnimBlocks`
const ANIM_BLOCKS: u32 = 0xB5D4A0;

/// The anim block reference a task holds for an anim group: `RemoveAnimBlockRef((ms_aAnimAssocGroups[group].m_pAnimBlock
/// - ms_aAnimBlocks) >> 5)` (groups at `[0xB4EA34]`, 0x14 bytes each; arithmetic shift).
fn release_group_block(group: u32) {
    let groups = g::<GPtr<u8>>(0xB4EA34).read();
    let block = rd::<u32>(groups, group.wrapping_mul(5).wrapping_mul(4) as i32 as isize);
    anim_remove_block_ref(((block.wrapping_sub(ANIM_BLOCKS) as i32) >> 5) as u32);
}

/// `CEntity::CleanUpOldReference` on the entity pointer at `this + off`, when it is set.
fn clean_ref(this: GPtr<u8>, off: isize) {
    let e = rd::<GPtr<u8>>(this, off);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(off));
    }
}

/// The hit / kick anim at `this + off`, when set: blend delta -4 (`0xC0800000`, written before the call), then its finish
/// callback reset to the default one.
fn blend_out_hit_anim(this: GPtr<u8>, off: isize) {
    let a = rd::<GPtr<u8>>(this, off);
    if !a.is_null() {
        wr::<u32>(a, 0x1C, 0xC080_0000);
        anim_set_finish_callback(rd::<GPtr<u8>>(this, off), DEFAULT_ANIM_CB, 0);
    }
}

/// `CTaskSimpleTogglePedThreatScanner::Destructor` (0x492D40): vtable 0x85A334, then `~CTaskSimple`.
#[hook(addr = 0x492D40, conv = thiscall, name = "CTaskSimpleTogglePedThreatScanner::Destructor", expect = "C7 01 34 A3 85 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn toggle_ped_threat_scanner_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85A334);
    task_simple_dtor(this);
}

/// `CTaskComplexBeCop::Destructor` (0x4994E0): vtable 0x85A428, then the `CTaskComplexWanderCop` destructor 0x460D80.
#[hook(addr = 0x4994E0, conv = thiscall, name = "CTaskComplexBeCop::Destructor", expect = "C7 01 28 A4 85 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn be_cop_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85A428);
    dtor_460d80(this);
}

/// `CTaskSimpleOnEscalator::Destructor` (0x4B8750): vtable 0x85B8DC, then the `CTaskSimpleStandStill` destructor
/// 0x62F360.
#[hook(addr = 0x4B8750, conv = thiscall, name = "CTaskSimpleOnEscalator::Destructor", expect = "C7 01 DC B8 85 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn on_escalator_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85B8DC);
    stand_still_dtor(this);
}

/// `CTaskSimpleWaitUntilLeaderAreaCodesMatch::Destructor` (0x5F67E0): vtable 0x86C7E8, then the
/// `CTaskSimpleWaitUntilAreaCodesMatch` destructor 0x6355E0.
#[hook(addr = 0x5F67E0, conv = thiscall, name = "CTaskSimpleWaitUntilLeaderAreaCodesMatch::Destructor", expect = "C7 01 E8 C7 86 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn wait_until_leader_area_codes_match_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86C7E8);
    wait_area_dtor(this);
}

/// The destructor of vtable 0x86D594 (0x61BCA0): a jump to `CTaskSimpleAnim::~CTaskSimpleAnim` 0x61A6F0 (no vtable of
/// its own is installed).
#[hook(addr = 0x61BCA0, conv = thiscall, name = "thunk_FUN_0061a6f0", expect = "E9 4B EA FF FF",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn anim_86d594_dtor(this: GPtr<u8>) {
    simple_anim_dtor(this);
}

/// `CTaskSimpleBeKickedOnGround::Destructor` (0x61FC50): vtable 0x86D7D8; the anim (+0xC) blends out (delta -4) with
/// the default finish callback; then `~CTaskSimple`.
#[hook(addr = 0x61FC50, conv = thiscall, name = "CTaskSimpleBeKickedOnGround::Destructor", expect = "6A FF 68 F8 F4 83 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn be_kicked_on_ground_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86D7D8);
    blend_out_hit_anim(this, 0xC);
    task_simple_dtor(this);
}

/// `CTaskSimpleBeHitWhileMoving::Destructor` (0x61FE70): vtable 0x86D7FC; the anim (+0x14) blends out (delta -4) with
/// the default finish callback; then `~CTaskSimple`.
#[hook(addr = 0x61FE70, conv = thiscall, name = "CTaskSimpleBeHitWhileMoving::Destructor", expect = "6A FF 68 18 F5 83 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn be_hit_while_moving_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86D7FC);
    blend_out_hit_anim(this, 0x14);
    task_simple_dtor(this);
}

/// `CTaskComplexReactToAttack::Destructor` (0x620B90): vtable 0x86D868; the attacker reference (+0x14) cleaned up;
/// then `~CTaskComplex`.
#[hook(addr = 0x620B90, conv = thiscall, name = "CTaskComplexReactToAttack::Destructor", expect = "6A FF 68 D8 F5 83 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn react_to_attack_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86D868);
    clean_ref(this, 0x14);
    task_complex_dtor(this);
}

/// `CTaskSimpleGangDriveBy::Destructor` (0x6218C0): vtable 0x86D944; with an anim block held (+9) the group's (+0x28)
/// block reference is released (the flag is not cleared); the drive-by anim (+0x2C) gets the default delete callback;
/// the target reference (+0x34) is cleaned up; then `~CTaskSimple`.
#[hook(addr = 0x6218C0, conv = thiscall, name = "CTaskSimpleGangDriveBy::Destructor", expect = "6A FF 68 28 F7 83 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn gang_drive_by_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86D944);
    if rd::<u8>(this, 9) != 0 {
        release_group_block(rd::<u32>(this, 0x28));
    }
    let a = rd::<GPtr<u8>>(this, 0x2C);
    if !a.is_null() {
        anim_set_delete_callback(a, DEFAULT_ANIM_CB, 0);
    }
    clean_ref(this, 0x34);
    task_simple_dtor(this);
}

/// `CTaskComplexKillAllThreats::Destructor` (0x6221C0): vtable 0x86DA1C, then `~CTaskComplex`.
#[hook(addr = 0x6221C0, conv = thiscall, name = "CTaskComplexKillAllThreats::Destructor", expect = "C7 01 1C DA 86 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn kill_all_threats_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86DA1C);
    task_complex_dtor(this);
}

/// `CTaskSimpleDieInCar::Destructor` (0x62FC70): vtable 0x86DDE0, then the `CTaskSimpleDie` destructor 0x62FB40.
#[hook(addr = 0x62FC70, conv = thiscall, name = "CTaskSimpleDieInCar::Destructor", expect = "C7 01 E0 DD 86 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn die_in_car_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86DDE0);
    die_dtor(this);
}

/// `CTaskSimpleWaitForPizza::Destructor` (0x632B70): vtable 0x86E1DC, then `~CTaskSimple`.
#[hook(addr = 0x632B70, conv = thiscall, name = "CTaskSimpleWaitForPizza::Destructor", expect = "C7 01 DC E1 86 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn wait_for_pizza_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86E1DC);
    task_simple_dtor(this);
}

/// `CTaskSimpleTriggerLookAt::Destructor` (0x634500): vtable 0x86E3CC; the look-at entity reference (+8) cleaned up;
/// then `~CTaskSimple`.
#[hook(addr = 0x634500, conv = thiscall, name = "CTaskSimpleTriggerLookAt::Destructor", expect = "6A FF 68 78 08 84 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn trigger_look_at_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86E3CC);
    clean_ref(this, 8);
    task_simple_dtor(this);
}

/// `CTaskComplexUseMobilePhone::Destructor` (0x6348E0): vtable 0x86E454, then `~CTaskComplex`.
#[hook(addr = 0x6348E0, conv = thiscall, name = "CTaskComplexUseMobilePhone::Destructor", expect = "C7 01 54 E4 86 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn use_mobile_phone_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86E454);
    task_complex_dtor(this);
}

/// `CTaskComplexOpenDriverDoor::Destructor` (0x6403D0): vtable 0x86EB0C, then `CTaskComplexEnterCar::Destructor`
/// 0x63DFA0.
#[hook(addr = 0x6403D0, conv = thiscall, name = "CTaskComplexOpenDriverDoor::Destructor", expect = "C7 01 0C EB 86 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn open_driver_door_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86EB0C);
    enter_car_dtor(this);
}

/// `CTaskComplexWanderMedic::Destructor` (0x658850): vtable 0x86F48C, then the `CTaskComplexWander` destructor 0x669D20.
#[hook(addr = 0x658850, conv = thiscall, name = "CTaskComplexWanderMedic::Destructor", expect = "C7 01 8C F4 86 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn wander_medic_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86F48C);
    dtor_669d20(this);
}

/// `CTaskComplexPresentIDToCop::Destructor` (0x659240): vtable 0x86F544; the cop reference (+0xC) cleaned up; then
/// `~CTaskComplex`.
#[hook(addr = 0x659240, conv = thiscall, name = "CTaskComplexPresentIDToCop::Destructor", expect = "6A FF 68 28 3C 84 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn present_id_to_cop_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86F544);
    clean_ref(this, 0xC);
    task_complex_dtor(this);
}

/// `CTaskComplexExtinguishFireOnFoot::Destructor` (0x6598B0): vtable 0x86F5FC, then `~CTaskComplex`.
#[hook(addr = 0x6598B0, conv = thiscall, name = "CTaskComplexExtinguishFireOnFoot::Destructor", expect = "C7 01 FC F5 86 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn extinguish_fire_on_foot_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86F5FC);
    task_complex_dtor(this);
}

/// The destructor of vtable 0x86F45C (0x659E00): a jump to `CTaskSimpleAnim::~CTaskSimpleAnim` 0x61A6F0.
#[hook(addr = 0x659E00, conv = thiscall, name = "thunk_FUN_0061a6f0", expect = "E9 EB 08 FC FF",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn anim_86f45c_dtor(this: GPtr<u8>) {
    simple_anim_dtor(this);
}

/// `CTaskComplexAvoidEntity::Destructor` (0x66AAE0): vtable 0x86FF00; the entity reference (+0xC) cleaned up; then
/// `~CTaskComplex`.
#[hook(addr = 0x66AAE0, conv = thiscall, name = "CTaskComplexAvoidEntity::Destructor", expect = "6A FF 68 98 50 84 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn avoid_entity_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86FF00);
    clean_ref(this, 0xC);
    task_complex_dtor(this);
}

/// The seek-entity task destructor of vtable 0x87004C (0x66CBB0, the `CTaskComplexSeekEntity` instantiation the
/// investigate-disturbance task creates): the target reference (+0xC) cleaned up, the position calculator member
/// (+0x40) gets vtable 0x859DC4 back, then `~CTaskComplex` (as 0x46AD70).
#[hook(addr = 0x66CBB0, conv = thiscall, name = "FUN_0066cbb0", expect = "6A FF 68 13 53 84 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn seek_entity_87004c_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x87004C);
    clean_ref(this, 0xC);
    wr::<u32>(this, 0x40, 0x859DC4);
    task_complex_dtor(this);
}

/// `CTaskComplexFollowPatrolRoute::Destructor` (0x6709F0): vtable 0x870114; an owned route (+0x1C) has its eight
/// 0x28-byte anim records (+4) destroyed (`eh_vector_destructor_iterator` with the empty destructor 0x46A380) and goes
/// back to the patrol route pool (0x41B820); then `~CTaskComplex`.
#[hook(addr = 0x6709F0, conv = thiscall, name = "CTaskComplexFollowPatrolRoute::Destructor", expect = "6A FF 68 78 56 84 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn follow_patrol_route_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x870114);
    let r = rd::<GPtr<u8>>(this, 0x1C);
    if !r.is_null() {
        eh_vector_dtor(r.byte_add(4), 0x28, 8, 0x46A380);
        patrol_route_delete(r);
    }
    task_complex_dtor(this);
}

/// `CTaskInteriorSitOnChair::Destructor` (0x675C90): vtable 0x870314; the anim (+0x14) gets the default delete
/// callback; then `~CTaskSimple`.
#[hook(addr = 0x675C90, conv = thiscall, name = "CTaskInteriorSitOnChair::Destructor", expect = "6A FF 68 D8 5C 84 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn interior_sit_on_chair_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x870314);
    let a = rd::<GPtr<u8>>(this, 0x14);
    if !a.is_null() {
        anim_set_delete_callback(a, DEFAULT_ANIM_CB, 0);
    }
    task_simple_dtor(this);
}

/// `CTaskComplexFallAndStayDown::Destructor` (0x678A90): vtable 0x870480, then `~CTaskComplex`.
#[hook(addr = 0x678A90, conv = thiscall, name = "CTaskComplexFallAndStayDown::Destructor", expect = "C7 01 80 04 87 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn fall_and_stay_down_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x870480);
    task_complex_dtor(this);
}

/// `CTaskComplexGoPickUpEntity::Destructor` (0x691A50): vtable 0x870B98; the entity reference (+0xC) cleaned up; with
/// an anim block held (+0x30) the group's (+0x2C) block reference is released and the flag cleared; then
/// `~CTaskComplex`.
#[hook(addr = 0x691A50, conv = thiscall, name = "CTaskComplexGoPickUpEntity::Destructor", expect = "6A FF 68 E8 72 84 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn go_pick_up_entity_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x870B98);
    clean_ref(this, 0xC);
    if rd::<u8>(this, 0x30) != 0 {
        release_group_block(rd::<u32>(this, 0x2C));
        wr::<u8>(this, 0x30, 0);
    }
    task_complex_dtor(this);
}

/// `CTaskSimpleDuckWhileShotsWhizzing::Destructor` (0x6926F0): vtable 0x870BEC, then `CTaskSimpleDuck::Destructor`
/// 0x692030.
#[hook(addr = 0x6926F0, conv = thiscall, name = "CTaskSimpleDuckWhileShotsWhizzing::Destructor", expect = "C7 01 EC 0B 87 00",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes")]
pub fn duck_while_shots_whizzing_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x870BEC);
    duck_dtor_orig(this);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_TOGGLE_PED_THREAT_SCANNER_DTOR,
    HOOK_BE_COP_DTOR,
    HOOK_ON_ESCALATOR_DTOR,
    HOOK_WAIT_UNTIL_LEADER_AREA_CODES_MATCH_DTOR,
    HOOK_ANIM_86D594_DTOR,
    HOOK_BE_KICKED_ON_GROUND_DTOR,
    HOOK_BE_HIT_WHILE_MOVING_DTOR,
    HOOK_REACT_TO_ATTACK_DTOR,
    HOOK_GANG_DRIVE_BY_DTOR,
    HOOK_KILL_ALL_THREATS_DTOR,
    HOOK_DIE_IN_CAR_DTOR,
    HOOK_WAIT_FOR_PIZZA_DTOR,
    HOOK_TRIGGER_LOOK_AT_DTOR,
    HOOK_USE_MOBILE_PHONE_DTOR,
    HOOK_OPEN_DRIVER_DOOR_DTOR,
    HOOK_WANDER_MEDIC_DTOR,
    HOOK_PRESENT_ID_TO_COP_DTOR,
    HOOK_EXTINGUISH_FIRE_ON_FOOT_DTOR,
    HOOK_ANIM_86F45C_DTOR,
    HOOK_AVOID_ENTITY_DTOR,
    HOOK_SEEK_ENTITY_87004C_DTOR,
    HOOK_FOLLOW_PATROL_ROUTE_DTOR,
    HOOK_INTERIOR_SIT_ON_CHAIR_DTOR,
    HOOK_FALL_AND_STAY_DOWN_DTOR,
    HOOK_GO_PICK_UP_ENTITY_DTOR,
    HOOK_DUCK_WHILE_SHOTS_WHIZZING_DTOR,
];
