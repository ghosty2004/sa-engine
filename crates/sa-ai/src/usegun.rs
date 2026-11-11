//! Hand-ported `CTaskSimpleUseGun` (PORT-AI-2, helper A). Ported from the 1.0 US bytes (the exe wins over gta-reversed and the specs).
//! Every call to another catalogued function goes through `orig!` at the original address; task objects
//! keep their original vtables and come from the original task pool (`common::task_new`).
//!
//! Field map (this): +8 finished, +9 in control, +0xA has move control, +0xB has fired, +0xC LOS blocked,
//! +0xD fire right (bit 0) / left (bit 1) this frame, +0xE skip aim, +0xF next command (i8), +0x10 last
//! command (i8), +0x14/+0x18 move command, +0x1C target entity, +0x20 target position, +0x2C anim,
//! +0x30 weapon info, +0x34 burst length (u16), +0x36 burst shots (i16), +0x38 countdown frames,
//! +0x39 arm IK in use, +0x3A look IK in use, +0x3B "aim immediately" flag.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `g_ikChainMan` (IKChainManager_c)
const IK_CHAIN_MAN: usize = 0xC15448;
/// `TheCamera` (CCamera)
const THE_CAMERA: usize = 0xB6F028;
/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7CB5C;
/// `CDefaultAnimCallback::DefaultAnimCB`
const DEFAULT_ANIM_CB: u32 = 0x4D1390;
/// `CTaskSimpleUseGun::FinishGunAnimCB` (passed as the original address)
const FINISH_GUN_ANIM_CB: u32 = 0x61F3A0;
/// -4.0f
const MINUS_FOUR: u32 = 0xC080_0000;
/// 8.0f
const EIGHT: u32 = 0x4100_0000;

orig! {
    /// `CTask::CTask` simple base constructor (0x61A390)
    fn ug_task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// simple task base destructor (0x61A3A0)
    fn ug_task_simple_dtor(t: GPtr<u8>) = thiscall 0x61A3A0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn ug_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn ug_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CAnimBlendAssociation::SetDeleteCallback(cb, data)` (0x4CEBC0)
    fn ug_set_delete_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBC0 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(cb, data)` (0x4CEBE0)
    fn ug_set_finish_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBE0 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80)
    fn ug_set_current_time(a: GPtr<u8>, t: u32) = thiscall 0x4CEA80 => unported;
    /// `CAnimBlendAssociation::Start(float)` (0x4CEB70)
    fn ug_assoc_start(a: GPtr<u8>, t: u32) = thiscall 0x4CEB70 => unported;
    /// `IKChainManager_c::IsArmPointing(arm, ped)` (0x6182B0)
    fn ug_ik_is_arm_pointing(m: GPtr<u8>, arm: i32, ped: GPtr<u8>) -> u8 = thiscall 0x6182B0 => unported;
    /// `IKChainManager_c::AbortPointArm(arm, ped, blend)` (0x6182F0)
    fn ug_ik_abort_point_arm(m: GPtr<u8>, arm: i32, ped: GPtr<u8>, blend: i32) = thiscall 0x6182F0 => unported;
    /// `IKChainManager_c::IsLooking(ped)` (0x6181A0)
    fn ug_ik_is_looking(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::AbortLookAt(ped, blend)` (0x618280)
    fn ug_ik_abort_look_at(m: GPtr<u8>, ped: GPtr<u8>, blend: i32) = thiscall 0x618280 => unported;
    /// `IKChainManager_c::IsFacingTarget(ped, slot)` (0x618330)
    fn ug_ik_is_facing_target(m: GPtr<u8>, ped: GPtr<u8>, slot: i32) -> u8 = thiscall 0x618330 => unported;
    /// `IKChainManager_c::LookAt(name, ped, target, time, bone, pos, useTorso, speed, blend, priority, b)` (0x618970)
    #[allow(clippy::too_many_arguments)]
    fn ug_ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, pos: GPtr<u8>, torso: u32, speed: u32, blend: i32, prio: i32, b: u32) = thiscall 0x618970 => unported;
    /// `IKChainManager_c::PointArm(name, arm, ped, target, bone, pos, speed, blendTime, cosLimit)` (0x618B60)
    #[allow(clippy::too_many_arguments)]
    fn ug_ik_point_arm(m: GPtr<u8>, name: u32, arm: i32, ped: GPtr<u8>, target: GPtr<u8>, bone: i32, pos: GPtr<u8>, speed: u32, blend: u32, limit: u32) = thiscall 0x618B60 => unported;
    /// `CPedIntelligence::GetTaskUseGun()` (0x600F70)
    fn ug_get_task_use_gun(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F70 => unported;
    /// `CPedIntelligence::GetTaskDuck(bool)` (0x6010A0)
    fn ug_get_task_duck(intel: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x6010A0 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810)
    fn ug_task_manager_get_task_secondary(tm: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    fn ug_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn ug_v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(ped, priority, event)` (vtable +0x18)
    fn ug_v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CPed::GetBonePosition(CVector&, bone, bool)` (0x5E4280)
    fn ug_ped_get_bone_position(p: GPtr<u8>, out: GPtr<u8>, bone: u32, b: u32) = thiscall 0x5E4280 => unported;
    /// `CPed::GetTransformedBonePosition(CVector&, bone, bool)` (0x5E01C0)
    fn ug_ped_get_transformed_bone_position(p: GPtr<u8>, io: GPtr<u8>, bone: u32, b: u32) = thiscall 0x5E01C0 => unported;
    /// `CPed::IsAlive()` (0x5E0170)
    fn ug_ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ug_ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::SetMoveState(eMoveState)` (0x5DEC00)
    fn ug_ped_set_move_state(p: GPtr<u8>, s: u32) = thiscall 0x5DEC00 => unported;
    /// `CPed::Say(id, ...)` (0x5EFFE0)
    fn ug_ped_say(p: GPtr<u8>, id: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32) = thiscall 0x5EFFE0 => unported;
    /// `CPed::DoGunFlash(lifetime, leftHand)` (0x5DF340)
    fn ug_ped_do_gun_flash(p: GPtr<u8>, life: u32, left: u32) -> u32 = thiscall 0x5DF340 => unported;
    /// `CPed::GetWeaponSkill()` (0x5E6580)
    fn ug_ped_get_weapon_skill(p: GPtr<u8>) -> u32 = thiscall 0x5E6580 => unported;
    /// `CPlacebale::GetAtDirection` style getter with a hidden out vector (0x50E420)
    fn ug_get_at_direction(p: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x50E420 => unported;
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn ug_player_get_pad(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CPad::GetPedWalkUpDown()` (0x53FD30)
    fn ug_pad_walk_up_down(pad: GPtr<u8>) -> i16 = thiscall 0x53FD30 => unported;
    /// `CPad::GetPedWalkLeftRight()` (0x53FC90)
    fn ug_pad_walk_left_right(pad: GPtr<u8>) -> i16 = thiscall 0x53FC90 => unported;
    /// `CWeaponInfo::GetWeaponInfo(type, skill)` (0x743C60)
    fn ug_get_weapon_info(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CWeapon::Fire(owner, src, src2, target, targetPos, altPos)` (0x742300)
    fn ug_weapon_fire(w: GPtr<u8>, owner: GPtr<u8>, src: GPtr<u8>, src2: GPtr<u8>, target: GPtr<u8>, tpos: GPtr<u8>, alt: GPtr<u8>) -> u32 = thiscall 0x742300 => unported;
    /// `CWeaponInfo::GetAnimLoopStart(bool duck)` (0x608F00, result in ST0)
    fn ug_wi_loop_start(wi: GPtr<u8>, duck: u32) -> f64 = thiscall 0x608F00 => unported;
    /// `CWeaponInfo::GetAnimLoopEnd(bool duck)` (0x608F20, result in ST0)
    fn ug_wi_loop_end(wi: GPtr<u8>, duck: u32) -> f64 = thiscall 0x608F20 => unported;
    /// `CWeaponInfo::GetAnimLoopFire(bool duck)` (0x61C150, result in ST0)
    fn ug_wi_loop_fire(wi: GPtr<u8>, duck: u32) -> f64 = thiscall 0x61C150 => unported;
    /// `CAnimManager::BlendAnimation(clump, group, id, delta)` (0x4D4610)
    fn ug_blend_animation(clump: u32, group: u32, id: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimManager::AddAnimation(clump, group, id)` (0x4D3AA0)
    fn ug_add_animation(clump: u32, group: u32, id: u32) -> GPtr<u8> = cdecl 0x4D3AA0 => unported;
    /// `RpAnimBlendClumpGetAssociation(clump, id)` (0x4D68B0)
    fn ug_clump_get_association(clump: u32, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CTaskSimpleDuck::ForceStopMove()` (0x6924B0)
    fn ug_duck_force_stop_move(d: GPtr<u8>) = thiscall 0x6924B0 => unported;
    /// `CTaskSimpleDuck::IsTaskInUseByOtherTasks()` (0x61C3D0)
    fn ug_duck_in_use_by_other_tasks(d: GPtr<u8>) -> u8 = thiscall 0x61C3D0 => unported;
    /// `CTaskSimpleDuck::StopFireGun()` (0x61C420)
    fn ug_duck_stop_fire_gun(d: GPtr<u8>) -> u8 = thiscall 0x61C420 => unported;
    /// `CPedIK::RotateTorsoForArm(const CVector&)` (0x5FDF90)
    fn ug_pedik_rotate_torso_for_arm(ik: GPtr<u8>, v: GPtr<u8>) = thiscall 0x5FDF90 => unported;
    /// `CPedIK::PointGunAtPosition(const CVector&, float)` (0x5FDE20)
    fn ug_pedik_point_gun_at_position(ik: GPtr<u8>, v: GPtr<u8>, amount: u32) -> u8 = thiscall 0x5FDE20 => unported;
    /// `CPedIK::PointGunInDirection(zAngle, angle2, bool, float)` (0x5FDC00)
    fn ug_pedik_point_gun_in_direction(ik: GPtr<u8>, a: u32, b: u32, c: u32, amount: u32) -> u8 = thiscall 0x5FDC00 => unported;
    /// `operator+(CVector* out, const CVector*, const CVector*)` (0x40FE30)
    fn ug_vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    /// `VectorSub(CVector* out, const CVector*, const CVector*)` (0x40FE60)
    fn ug_vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CVector::operator-=(const CVector&)` (0x406D70)
    fn ug_vector_sub_assign(v: GPtr<u8>, o: GPtr<u8>) = thiscall 0x406D70 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn ug_vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `operator*(CVector* out, const CMatrix&, const CVector&)` (0x59C890)
    fn ug_matrix_mul_vector(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CCamera::Find3rdPersonCamTargetVector(range, CVector source, CVector& camPos, CVector& pointAt)` (0x514970)
    #[allow(clippy::too_many_arguments)]
    fn ug_find_3rd_person_cam_target_vector(cam: GPtr<u8>, range: u32, sx: u32, sy: u32, sz: u32, cam_pos: GPtr<u8>, point: GPtr<u8>) = thiscall 0x514970 => unported;
    /// `CTaskSimpleFight::CTaskSimpleFight(target, command, time)` (0x61C470)
    fn ug_fight_ctor(t: GPtr<u8>, target: GPtr<u8>, cmd: u32, time: u32) -> GPtr<u8> = thiscall 0x61C470 => unported;
    /// `CTaskSimpleFight::~CTaskSimpleFight()` (0x61C530)
    fn ug_fight_dtor(t: GPtr<u8>) = thiscall 0x61C530 => unported;
    /// `CTaskSimpleFight` pistol-whip hit test `(ped, const CVector*)` (0x6240B0)
    fn ug_fight_6240b0(t: GPtr<u8>, ped: GPtr<u8>, v: GPtr<u8>) = thiscall 0x6240B0 => unported;
    // This class's own methods (called through their original addresses).
    /// `CTaskSimpleUseGun::Destructor` (0x61DF30)
    fn ug_destructor_orig(t: GPtr<u8>) = thiscall 0x61DF30 => unported;
    /// `CTaskSimpleUseGun::AbortIK(ped)` (0x61DFA0)
    fn ug_abort_ik_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x61DFA0 => unported;
    /// `CTaskSimpleUseGun::SetMoveAnim(ped)` (0x61E3F0)
    fn ug_set_move_anim_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x61E3F0 => unported;
    /// `CTaskSimpleUseGun::RemoveStanceAnims(ped, float)` (0x61E8E0)
    fn ug_remove_stance_anims_orig(t: GPtr<u8>, ped: GPtr<u8>, delta: u32) = thiscall 0x61E8E0 => unported;
    /// `CTaskSimpleUseGun::FireGun(ped, bool left)` (0x61EB10)
    fn ug_fire_gun_orig(t: GPtr<u8>, ped: GPtr<u8>, left: u32) -> u32 = thiscall 0x61EB10 => unported;
    /// `CTaskSimpleUseGun::AimGun(ped)` (0x61ED10)
    fn ug_aim_gun_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x61ED10 => unported;
    /// `CTaskSimpleUseGun::StartAnim(ped)` (0x624F30)
    fn ug_start_anim_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x624F30 => unported;
}

fn ik() -> GPtr<u8> {
    GPtr::<u8>::from_addr(IK_CHAIN_MAN)
}

fn cf(addr: usize) -> X87 {
    x(g::<f32>(addr).read())
}

/// The entity's position: `m_matrix->pos` (+0x30) when it has a matrix, else the placement (+4).
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// Bit 26 of `ped+0x46C` (`bIsDucking`) as 0/1.
fn ducking(ped: GPtr<u8>) -> u32 {
    (rd::<u32>(ped, 0x46C) >> 26) & 1
}

/// The active weapon (`ped+0x5A0 + 0x1C * (i8)ped+0x718`).
fn active_weapon(ped: GPtr<u8>) -> GPtr<u8> {
    ped.byte_add(0x5A0 + 0x1C * rd::<i8>(ped, 0x718) as isize)
}

/// `fld v; fcomp 0.0; test ah, 0x44; jp`: true unless `v == 0.0` (NaN counts as non-zero).
fn nonzero(v: f32) -> bool {
    x(v).fcom(cf(0x858B50)).test_parity(0x44)
}

fn wi_flags(this: GPtr<u8>) -> u32 {
    rd::<u32>(rd::<GPtr<u8>>(this, 0x30), 0x18)
}

/// `CTaskSimpleUseGun::CTaskSimpleUseGun(CEntity* target, CVector pos, uint8 cmd, uint16 burst, bool aimImmediate)`
/// (0x61DE60, `ret 0x1C`): simple-task base, vtable 0x86D724, flags cleared (+9 in control = 1), +0xF = cmd,
/// +0x10 = 0xFF, target registered at +0x1C, +0x20 = pos, +0x34 = burst, +0x38 = 0xFF, +0x3B = aimImmediate.
#[hook(addr = 0x61DE60, conv = thiscall, name = "CTaskSimpleUseGun::Constructor", expect = "6A FF 68 D8 F3 83 00")]
#[allow(clippy::too_many_arguments)]
pub fn use_gun_ctor(
    this: GPtr<u8>,
    target: GPtr<u8>,
    px: u32,
    py: u32,
    pz: u32,
    cmd: u32,
    burst: u32,
    aim_immediate: u32,
) -> GPtr<u8> {
    ug_task_simple_ctor(this);
    wr::<u8>(this, 0xF, cmd as u8);
    wr::<u32>(this, 0, 0x86D724);
    wr::<u8>(this, 8, 0);
    wr::<u8>(this, 9, 1);
    wr::<u8>(this, 0xA, 0);
    wr::<u8>(this, 0xB, 0);
    wr::<u8>(this, 0xC, 0);
    wr::<u8>(this, 0xD, 0);
    wr::<u8>(this, 0xE, 0);
    wr::<u8>(this, 0x10, 0xFF);
    wr::<u32>(this, 0x14, 0);
    wr::<u32>(this, 0x18, 0);
    wr::<GPtr<u8>>(this, 0x1C, target);
    if !target.is_null() {
        ug_register_reference(target, this.byte_add(0x1C));
    }
    wr::<u32>(this, 0x20, px);
    wr::<u32>(this, 0x24, py);
    wr::<u32>(this, 0x28, pz);
    wr::<u16>(this, 0x34, burst as u16);
    wr::<u32>(this, 0x2C, 0);
    wr::<u32>(this, 0x30, 0);
    wr::<u16>(this, 0x36, 0);
    wr::<u8>(this, 0x39, 0);
    wr::<u8>(this, 0x3A, 0);
    wr::<u8>(this, 0x3B, aim_immediate as u8);
    wr::<u8>(this, 0x38, 0xFF);
    this
}

/// `CTaskSimpleUseGun::GetTaskType` (0x61DF20): `mov eax, 0x3F9; ret` -> 1017.
#[hook(addr = 0x61DF20, conv = thiscall, name = "CTaskSimpleUseGun::GetTaskType", expect = "B8 F9 03 00 00")]
pub fn use_gun_get_task_type(_this: GPtr<u8>) -> u32 {
    0x3F9
}

/// `CTaskSimpleUseGun::~CTaskSimpleUseGun` complete destructor (0x61DF30): vtable 0x86D724, releases the
/// target reference, resets the anim's delete callback to the default one, then the simple-task base.
#[hook(addr = 0x61DF30, conv = thiscall, name = "CTaskSimpleUseGun::Destructor", expect = "6A FF 68 F8 F3 83 00")]
pub fn use_gun_destructor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86D724);
    let t = rd::<GPtr<u8>>(this, 0x1C);
    if !t.is_null() {
        ug_clean_up_old_reference(t, this.byte_add(0x1C));
    }
    let a = rd::<GPtr<u8>>(this, 0x2C);
    if !a.is_null() {
        ug_set_delete_callback(a, DEFAULT_ANIM_CB, 0);
    }
    ug_task_simple_dtor(this);
}

/// `CTaskSimpleUseGun::AbortIK(CPed*)` (0x61DFA0): with arm IK in use, aborts arm 0 and arm 1 pointing
/// (blend 250) where pointing and clears +0x39; with look IK, aborts the look-at and clears +0x3A; always
/// clears bit 4 of `ped+0x528`.
#[hook(addr = 0x61DFA0, conv = thiscall, name = "CTaskSimpleUseGun::AbortIK", expect = "56 8B 74 24 08")]
pub fn use_gun_abort_ik(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u8>(this, 0x39) != 0 {
        if ug_ik_is_arm_pointing(ik(), 0, ped) != 0 {
            ug_ik_abort_point_arm(ik(), 0, ped, 0xFA);
        }
        if ug_ik_is_arm_pointing(ik(), 1, ped) != 0 {
            ug_ik_abort_point_arm(ik(), 1, ped, 0xFA);
        }
        wr::<u8>(this, 0x39, 0);
    }
    if rd::<u8>(this, 0x3A) != 0 {
        if ug_ik_is_looking(ik(), ped) != 0 {
            ug_ik_abort_look_at(ik(), ped, 0xFA);
        }
        wr::<u8>(this, 0x3A, 0);
    }
    wr::<u32>(ped, 0x528, rd::<u32>(ped, 0x528) & 0xFFFF_FFEF);
}

/// `CTaskSimpleUseGun::ControlGun(CPed*, CEntity* target, int8 cmd)` (0x61E040, `ret 0xC`): +9 = 1; a new
/// target replaces the registered one and resets the IK (`AbortIK`, +0x39 = +0x3A = 0); a higher command
/// (signed) replaces +0xF. Returns true.
#[hook(addr = 0x61E040, conv = thiscall, name = "CTaskSimpleUseGun::ControlGun", expect = "55 8B 6C 24 0C")]
pub fn use_gun_control_gun(this: GPtr<u8>, ped: GPtr<u8>, target: GPtr<u8>, cmd: u32) -> u8 {
    let old = rd::<GPtr<u8>>(this, 0x1C);
    wr::<u8>(this, 9, 1);
    if target != old {
        if !old.is_null() {
            ug_clean_up_old_reference(old, this.byte_add(0x1C));
        }
        wr::<GPtr<u8>>(this, 0x1C, target);
        if !target.is_null() {
            ug_register_reference(target, this.byte_add(0x1C));
        }
        ug_abort_ik_orig(this, ped);
        wr::<u8>(this, 0x39, 0);
        wr::<u8>(this, 0x3A, 0);
    }
    if cmd as i8 > rd::<i8>(this, 0xF) {
        wr::<u8>(this, 0xF, cmd as u8);
    }
    1
}

/// `CTaskSimpleUseGun::PlayerPassiveControlGun()` (0x61E0A0): +9 = 1; when both commands (signed) are
/// below FIRE, +0xF = END_LEISURE (6) and true, else false.
#[hook(addr = 0x61E0A0, conv = thiscall, name = "CTaskSimpleUseGun::PlayerPassiveControlGun", expect = "8A 51 10 B0 02")]
pub fn use_gun_player_passive_control_gun(this: GPtr<u8>) -> u8 {
    let last = rd::<i8>(this, 0x10);
    wr::<u8>(this, 9, 1);
    if last < 2 && rd::<i8>(this, 0xF) < 2 {
        wr::<u8>(this, 0xF, 6);
        return 1;
    }
    0
}

/// `CTaskSimpleUseGun::ControlGunMove(const CVector2D*)` (0x61E0C0, `ret 4`): each move component (y first,
/// then x) moves towards the input by at most `ts * 0.07`: above `cur + t` -> `cur + t`, below `cur - t`
/// -> `cur - t`, else (NaN too) the input copied. +0xA = 1.
#[hook(addr = 0x61E0C0, conv = thiscall, name = "CTaskSimpleUseGun::ControlGunMove", expect = "D9 05 5C CB B7 00")]
pub fn use_gun_control_gun_move(this: GPtr<u8>, mv: GPtr<u8>) {
    let t = cf(TIME_STEP).fmul_m(g::<f32>(0x858CA8).read());
    for (src, dst) in [(4isize, 0x18isize), (0, 0x14)] {
        let d = x(rd::<f32>(mv, src)).fsub_m(rd::<f32>(this, dst));
        if !d.fcom(t).test_any(0x41) {
            wr::<f32>(this, dst, f32_of(t.fadd_m(rd::<f32>(this, dst))));
        } else if !d.fcom(t.fchs()).test_parity(5) {
            wr::<f32>(this, dst, f32_of(x(rd::<f32>(this, dst)).fsub(t)));
        } else {
            wr::<u32>(this, dst, rd::<u32>(mv, src));
        }
        if dst == 0x14 {
            wr::<u8>(this, 0xA, 1);
        }
    }
}

/// `CTaskSimpleUseGun::RequirePistolWhip(CPed*, CEntity* target)` (0x61E200, static cdecl): with the ped's
/// own use-gun task already pistol-whipping, false. Candidates: the target when it is a ped (only it), else
/// the ped intelligence's 16 scanned entities (+0x130). A candidate qualifies when alive, its bone 5 is
/// above the ped (z strictly, and below z + 1.5), the ped is not ducking (target flag 0x100000 of +0x474)
/// or its simplest active task is 0xCD, it is in front (forward dot > 0), within 2D distance < [0x8D2E68]
/// and `|right dot| / forward dot < [0x8D2E6C]`. The forward dot is stored into the caller's ped argument
/// slot (the original reuses it as a temporary).
#[hook(addr = 0x61E200, conv = cdecl, name = "CTaskSimpleUseGun::RequirePistolWhip", expect = "83 EC 10 8B 44 24 18")]
pub fn use_gun_require_pistol_whip(ped: GPtr<u8>, target: GPtr<u8>) -> u8 {
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    let mut cand = GPtr::<u8>::NULL;
    let mut i: i32 = 0;
    if !target.is_null() && rd::<u8>(target, 0x36) & 7 == 3 {
        cand = target;
        i = 0xF;
    }
    let task = ug_get_task_use_gun(intel);
    if !task.is_null() && rd::<u8>(task, 0x10) == 5 {
        return 0;
    }
    if i >= 0x10 {
        return 0;
    }
    loop {
        'cand: {
            if cand.is_null() {
                let e = rd::<GPtr<u8>>(intel, 0x130 + 4 * i as isize);
                if e.is_null() {
                    break 'cand;
                }
                cand = e;
            }
            let mut bone = [0f32; 3];
            let bp: GPtr<u8> = GPtr::from_raw(&mut bone as *mut [f32; 3]).cast();
            ug_ped_get_bone_position(cand, bp, 5, 0);
            if ug_ped_is_alive(cand) == 0 {
                break 'cand;
            }
            let pp = ent_pos(ped);
            if x(rd::<f32>(bp, 8)).fcom(x(rd::<f32>(pp, 8))).test_any(0x41) {
                break 'cand;
            }
            if x(rd::<f32>(pp, 8))
                .fadd_m(g::<f32>(0x858CE8).read())
                .fcom(x(rd::<f32>(bp, 8)))
                .test_any(0x41)
            {
                break 'cand;
            }
            if rd::<u32>(cand, 0x474) & 0x100000 != 0 {
                let t = ug_get_simplest_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
                if ug_v_get_task_type(t) != 0xCD {
                    break 'cand;
                }
            }
            let m = rd::<GPtr<u8>>(ped, 0x14);
            let (pp, cp) = (ent_pos(ped), ent_pos(cand));
            let dx = x(rd::<f32>(cp, 0)).fsub_m(rd::<f32>(pp, 0));
            let dy = x(rd::<f32>(cp, 4)).fsub_m(rd::<f32>(pp, 4));
            let dz = x(rd::<f32>(cp, 8)).fsub_m(rd::<f32>(pp, 8));
            let fwd = dz
                .fmul_m(rd::<f32>(m, 0x18))
                .fadd(dy.fmul_m(rd::<f32>(m, 0x14)))
                .fadd(dx.fmul_m(rd::<f32>(m, 0x10)));
            // fst dword [esp+0x24]: the caller's ped argument slot
            let fwd_stored = f32_of(fwd);
            if fwd.fcom(cf(0x858B50)).test_any(0x41) {
                break 'cand;
            }
            let (pp, cp) = (ent_pos(ped), ent_pos(cand));
            let dx = x(rd::<f32>(cp, 0)).fsub_m(rd::<f32>(pp, 0));
            let dy = x(rd::<f32>(cp, 4)).fsub_m(rd::<f32>(pp, 4));
            let d2 = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
            if d2.fcom(cf(0x8D2E68)).test_parity(5) {
                break 'cand;
            }
            let (pp, cp) = (ent_pos(ped), ent_pos(cand));
            let dx = x(rd::<f32>(cp, 0)).fsub_m(rd::<f32>(pp, 0));
            let dy = x(rd::<f32>(cp, 4)).fsub_m(rd::<f32>(pp, 4));
            let dz = x(rd::<f32>(cp, 8)).fsub_m(rd::<f32>(pp, 8));
            let r = dz
                .fmul_m(rd::<f32>(m, 8))
                .fadd(dy.fmul_m(rd::<f32>(m, 4)))
                .fadd(dx.fmul_m(rd::<f32>(m, 0)));
            let q = X87::from_f64_exact(r.to_f64().abs()).fdiv_m(fwd_stored);
            if !q.fcom(cf(0x8D2E6C)).test_parity(5) {
                return 1;
            }
        }
        cand = GPtr::NULL;
        i += 1;
        if i >= 0x10 {
            return 0;
        }
    }
}

/// `CTaskSimpleUseGun::SetMoveAnim(CPed*)` (0x61E3F0, `ret 4`). Looks up the move anims 0x31..0x35 first.
/// Ducking: forces the duck task to stop moving while FIRE/FIREBURST/RELOAD, clears the move command and
/// +0xA. Otherwise `sum = |y| + |x|` of the move command (0 without move control or for FIRE without
/// weapon flag 0x20), `len = sqrt(x² + y²)`. `sum < 0.1`: blends the idle (aim-with-arm: SetMoveState(1)
/// and anim 0xBE (ped types 6..15) or the group's 3; else 0x31 from group 0), stops the four strafe anims
/// (PLAYING off; restarted at 0 once the new anim is above 0.95), clears the move command and +0xA.
/// Moving with aim-with-arm: one of the strafe anims 0x35/0x33/0x34/0x32 by the command, SetMoveState(0).
/// Moving otherwise: fades anim 0x31, then blends the strafe anims by the normalised command (speed
/// `wi+0x3C * len`); a running AIM anim with flag 0x10 clear is blended out. The 0x32 association is kept
/// in the caller's ped argument slot (the original reuses it as a local).
#[hook(addr = 0x61E3F0, conv = thiscall, name = "CTaskSimpleUseGun::SetMoveAnim", expect = "83 EC 14 53 55")]
pub fn use_gun_set_move_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let a31 = ug_clump_get_association(rd::<u32>(ped, 0x18), 0x31);
    let mut a32 = ug_clump_get_association(rd::<u32>(ped, 0x18), 0x32);
    let mut a33 = ug_clump_get_association(rd::<u32>(ped, 0x18), 0x33);
    let mut a34 = ug_clump_get_association(rd::<u32>(ped, 0x18), 0x34);
    let mut a35 = ug_clump_get_association(rd::<u32>(ped, 0x18), 0x35);
    if rd::<u32>(ped, 0x46C) & 0x400_0000 != 0 {
        let duck = ug_get_task_duck(rd::<GPtr<u8>>(ped, 0x47C), 1);
        if !duck.is_null() {
            let c = rd::<u8>(this, 0x10);
            if c == 2 || c == 3 || c == 4 {
                ug_duck_force_stop_move(duck);
            }
        }
        wr::<u32>(this, 0x14, 0);
        wr::<u32>(this, 0x18, 0);
        wr::<u8>(this, 0xA, 0);
        return;
    }
    let compute = rd::<u8>(this, 0xA) != 0 && (rd::<u8>(this, 0x10) != 2 || wi_flags(this) & 0x20 != 0);
    let (sum, len) = if compute {
        let ay = X87::from_f64_exact(x(rd::<f32>(this, 0x18)).to_f64().abs());
        let ax = X87::from_f64_exact(x(rd::<f32>(this, 0x14)).to_f64().abs());
        let sum = ay.fadd(ax);
        let (vy, vx) = (x(rd::<f32>(this, 0x18)), x(rd::<f32>(this, 0x14)));
        let l = vx.fmul(vx).fadd(vy.fmul(vy)).fsqrt();
        (sum, f32_of(l))
    } else {
        (cf(0x858B50), 0.0f32)
    };
    let mut l20: u32 = 0x3F80_0000;
    if !sum.fcom(cf(0x858B1C)).test_parity(5) {
        // sum < 0.1: stop
        let anim = if wi_flags(this) & 2 != 0 {
            ug_ped_set_move_state(ped, 1);
            let t = rd::<i32>(ped, 0x598);
            if (7..=0xF).contains(&t) || t == 6 {
                ug_blend_animation(rd::<u32>(ped, 0x18), 0, 0xBE, 0x4080_0000)
            } else {
                ug_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), 3, EIGHT)
            }
        } else {
            ug_blend_animation(rd::<u32>(ped, 0x18), 0, 0x31, EIGHT)
        };
        for a in [a32, a33, a34, a35] {
            if !a.is_null() {
                wr::<u16>(a, 0x2E, rd::<u16>(a, 0x2E) & 0xFFFE);
            }
        }
        if !anim.is_null() && !x(rd::<f32>(anim, 0x18)).fcom(cf(0x858EF0)).test_any(0x41) {
            for a in [a32, a33, a34, a35] {
                if !a.is_null() {
                    ug_set_current_time(a, 0);
                }
            }
        }
        wr::<u32>(this, 0x14, 0);
        wr::<u32>(this, 0x18, 0);
        wr::<u8>(this, 0xA, 0);
        return;
    }
    if wi_flags(this) & 2 != 0 {
        let id = if !x(rd::<f32>(this, 0x14)).fcom(cf(0x858F34)).test_any(0x41) {
            0x35
        } else if !x(rd::<f32>(this, 0x14)).fcom(cf(0x858F30)).test_parity(5) {
            0x33
        } else if !x(rd::<f32>(this, 0x18)).fcom(cf(0x858B50)).test_any(0x41) {
            0x34
        } else {
            0x32
        };
        let a = ug_blend_animation(rd::<u32>(ped, 0x18), 0, id, EIGHT);
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
        ug_ped_set_move_state(ped, 0);
        return;
    }
    if !a31.is_null() {
        if !x(rd::<f32>(a31, 0x1C)).fcom(cf(0x858B50)).test_any(0x01) {
            wr::<u32>(a31, 0x1C, 0);
            let mut v = x(rd::<f32>(a31, 0x18)).fsub(cf(TIME_STEP).fmul_m(g::<f32>(0x86D748).read()));
            if !cf(0x858B50).fcom(v).test_any(0x41) {
                v = cf(0x858B50);
            }
            wr::<f32>(a31, 0x18, f32_of(v));
        }
        let mut v2 = cf(TIME_STEP)
            .fmul_m(rd::<f32>(a31, 0x1C))
            .fmul_m(g::<f32>(0x858B38).read())
            .fadd_m(rd::<f32>(a31, 0x18));
        let one = cf(0x858624);
        if !one.fcom(v2).test_parity(5) {
            // 1.0 < v2
            if !cf(0x858624).fcom(v2).test_parity(5) {
                v2 = cf(0x858624);
            }
        } else if !cf(0x858B50).fcom(v2).test_any(0x41) {
            v2 = cf(0x858B50);
        } else if !cf(0x858624).fcom(v2).test_parity(5) {
            v2 = cf(0x858624);
        }
        l20 = f32_of(cf(0x858624).fsub(v2)).to_bits();
    }
    let inv = sum.fdivr_m(g::<f32>(0x858624).read());
    let sx = f32_of(inv.fmul_m(rd::<f32>(this, 0x14)).fmul_m(f32::from_bits(l20)));
    let sy = f32_of(inv.fmul_m(rd::<f32>(this, 0x18)).fmul_m(f32::from_bits(l20)));
    let speed = |this: GPtr<u8>| f32_of(x(rd::<f32>(rd::<GPtr<u8>>(this, 0x30), 0x3C)).fmul_m(len));
    if !x(sx).fcom(cf(0x858B50)).test_any(0x41) {
        if !a33.is_null() {
            wr::<u32>(a33, 0x18, 0);
            wr::<u32>(a33, 0x1C, 0);
        }
        if a35.is_null() {
            a35 = ug_add_animation(rd::<u32>(ped, 0x18), 0, 0x35);
        }
        wr::<f32>(a35, 0x18, sx);
        wr::<u32>(a35, 0x1C, 0);
        wr::<u8>(a35, 0x2E, rd::<u8>(a35, 0x2E) | 1);
        wr::<f32>(a35, 0x24, speed(this));
    } else if !x(sx).fcom(cf(0x858B50)).test_parity(5) {
        if !a35.is_null() {
            wr::<u32>(a35, 0x18, 0);
            wr::<u32>(a35, 0x1C, 0);
        }
        if a33.is_null() {
            a33 = ug_add_animation(rd::<u32>(ped, 0x18), 0, 0x33);
        }
        wr::<u32>(a33, 0x1C, 0);
        wr::<f32>(a33, 0x18, f32_of(x(sx).fchs()));
        wr::<u8>(a33, 0x2E, rd::<u8>(a33, 0x2E) | 1);
        wr::<f32>(a33, 0x24, speed(this));
    }
    if !x(sy).fcom(cf(0x858B50)).test_parity(5) {
        if !a34.is_null() {
            wr::<u32>(a34, 0x18, 0);
            wr::<u32>(a34, 0x1C, 0);
        }
        if a32.is_null() {
            a32 = ug_add_animation(rd::<u32>(ped, 0x18), 0, 0x32);
        }
        wr::<f32>(a32, 0x18, f32_of(x(sy).fchs()));
        wr::<u32>(a32, 0x1C, 0);
        wr::<f32>(a32, 0x24, speed(this));
        wr::<u8>(a32, 0x2E, rd::<u8>(a32, 0x2E) | 1);
    } else if !x(sy).fcom(cf(0x858B50)).test_any(0x41) {
        if !a32.is_null() {
            wr::<u32>(a32, 0x18, 0);
            wr::<u32>(a32, 0x1C, 0);
        }
        if a34.is_null() {
            a34 = ug_add_animation(rd::<u32>(ped, 0x18), 0, 0x34);
        }
        wr::<f32>(a34, 0x18, sy);
        wr::<u32>(a34, 0x1C, 0);
        wr::<f32>(a34, 0x24, speed(this));
        wr::<u8>(a34, 0x2E, rd::<u8>(a34, 0x2E) | 1);
    }
    if rd::<u8>(this, 0x10) == 1 {
        let a = rd::<GPtr<u8>>(this, 0x2C);
        if !a.is_null()
            && !x(rd::<f32>(a, 0x18)).fcom(cf(0x858B50)).test_any(0x41)
            && !x(rd::<f32>(a, 0x1C)).fcom(cf(0x858B50)).test_any(0x01)
            && wi_flags(this) & 0x10 == 0
        {
            wr::<u32>(a, 0x1C, MINUS_FOUR);
        }
    }
    wr::<u8>(this, 0xA, 0);
}

/// `CTaskSimpleUseGun::RemoveStanceAnims(CPed*, float)` (0x61E8E0, `ret 8`; the float is never read). A
/// stopped gun anim still before its end is restarted at the loop end when exactly at the loop start, and
/// set PLAYING. The five move anims 0x31..0x35 get FREEZE (flag 4); SetMoveState(1), ped+0x538 = 1. When
/// any existed: a player whose pad walks (|stick| > 50, each axis read twice) blends walk (group, 0, 8.0),
/// SetMoveState(4), +0x538 = 4 and player data +0x14 = 1.0 (no pad: like a non-player); others blend
/// idle (group, 3, 8.0) and, for END_LEISURE outside groups 0x39..0x3B, 0x36 from group 0. The stick
/// values are converted through the caller's ped argument slot (the original reuses it).
#[hook(addr = 0x61E8E0, conv = thiscall, name = "CTaskSimpleUseGun::RemoveStanceAnims", expect = "51 53 55 56 57")]
pub fn use_gun_remove_stance_anims(this: GPtr<u8>, ped: GPtr<u8>, _delta: u32) {
    let a = rd::<GPtr<u8>>(this, 0x2C);
    if !a.is_null()
        && rd::<u8>(a, 0x2E) & 1 == 0
        && !x(rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10))
            .fcom(x(rd::<f32>(a, 0x20)))
            .test_any(0x41)
    {
        let wi = rd::<GPtr<u8>>(this, 0x30);
        if !wi.is_null()
            && !x(rd::<f32>(wi, 0x40))
                .fcom(x(rd::<f32>(a, 0x20)))
                .test_parity(0x44)
        {
            ug_set_current_time(a, rd::<u32>(wi, 0x44));
        }
        let a = rd::<GPtr<u8>>(this, 0x2C);
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
    }
    let mut any = false;
    for id in 0x31..=0x35 {
        let s = ug_clump_get_association(rd::<u32>(ped, 0x18), id);
        if !s.is_null() {
            wr::<u16>(s, 0x2E, rd::<u16>(s, 0x2E) | 4);
            any = true;
        }
    }
    ug_ped_set_move_state(ped, 1);
    wr::<u32>(ped, 0x538, 1);
    if !any {
        return;
    }
    if ug_ped_is_player(ped) != 0 {
        let pad = ug_player_get_pad(ped);
        if !pad.is_null() {
            let walk = !X87::from_i32(i32::from(ug_pad_walk_up_down(pad)))
                .fcom(cf(0x858B40))
                .test_any(0x41)
                || !X87::from_i32(i32::from(ug_pad_walk_up_down(pad)))
                    .fcom(cf(0x859B60))
                    .test_parity(5)
                || !X87::from_i32(i32::from(ug_pad_walk_left_right(pad)))
                    .fcom(cf(0x858B40))
                    .test_any(0x41)
                || !X87::from_i32(i32::from(ug_pad_walk_left_right(pad)))
                    .fcom(cf(0x859B60))
                    .test_parity(5);
            if walk {
                let w = ug_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), 0, EIGHT);
                wr::<u8>(w, 0x2E, rd::<u8>(w, 0x2E) | 1);
                ug_ped_set_move_state(ped, 4);
                wr::<u32>(ped, 0x538, 4);
                let pd = rd::<GPtr<u8>>(ped, 0x480);
                if !pd.is_null() {
                    wr::<u32>(pd, 0x14, 0x3F80_0000);
                }
                return;
            }
        }
    }
    let i = ug_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), 3, EIGHT);
    wr::<u8>(i, 0x2E, rd::<u8>(i, 0x2E) | 1);
    if rd::<u8>(this, 0xF) == 6 {
        let grp = rd::<u32>(ped, 0x4D4);
        if grp != 0x39 && grp != 0x3A && grp != 0x3B {
            ug_blend_animation(rd::<u32>(ped, 0x18), 0, 0x36, EIGHT);
        }
    }
}

/// `CTaskSimpleUseGun::FireGun(CPed*, bool left)` (0x61EB10, `ret 8`). Source = the ped position raised by
/// 0.7 (both vectors; ped+0x528 bit 0 set when bit 2 is set), or with ped flag 0x400 of +0x474 the hand
/// bone (0x22 left / 0x18 right) raised by `0.15 + offset.z` and the weapon's fire offset (+0x24)
/// transformed by that bone (ped+0x528 bit 0 = IsFacingTarget(slot 2 / 1) when bit 2 is set).
/// `CWeapon::Fire(ped, &src, &src2, target, targetPos or null, null)` with the target position only when
/// both its x and y are non-zero; `DoGunFlash([0x8D2E88], left)` when the weapon state is 1. Returns the
/// last callee's EAX (callers ignore it).
#[hook(addr = 0x61EB10, conv = thiscall, name = "CTaskSimpleUseGun::FireGun", expect = "83 EC 18 56 8B 74 24 20")]
pub fn use_gun_fire_gun(this: GPtr<u8>, ped: GPtr<u8>, left: u32) -> u32 {
    let off = rd::<GPtr<u8>>(this, 0x30).byte_add(0x24);
    let mut v1 = [0u32; 3];
    let mut v2 = [rd::<u32>(off, 0), rd::<u32>(off, 4), rd::<u32>(off, 8)];
    let p1: GPtr<u8> = GPtr::from_raw(&mut v1 as *mut [u32; 3]).cast();
    let p2: GPtr<u8> = GPtr::from_raw(&mut v2 as *mut [u32; 3]).cast();
    if rd::<u32>(ped, 0x474) & 0x400 == 0 {
        let pos = ent_pos(ped);
        let (px, py, pz) = (rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8));
        wr::<u32>(p1, 0, px);
        wr::<u32>(p1, 4, py);
        wr::<u32>(p1, 8, pz);
        let z = f32_of(x(f32::from_bits(pz)).fadd_m(g::<f32>(0x858CB0).read()));
        wr::<u32>(p2, 4, py);
        wr::<u32>(p2, 0, px);
        wr::<f32>(p1, 8, z);
        wr::<f32>(p2, 8, z);
        if rd::<u32>(ped, 0x528) & 4 != 0 {
            wr::<u32>(ped, 0x528, rd::<u32>(ped, 0x528) | 1);
        }
    } else {
        let (bone, slot) = if left as u8 != 0 { (0x22, 2) } else { (0x18, 1) };
        ug_ped_get_bone_position(ped, p1, bone, 0);
        let z = cf(0x86D628).fadd_m(rd::<f32>(p2, 8)).fadd_m(rd::<f32>(p1, 8));
        wr::<f32>(p1, 8, f32_of(z));
        ug_ped_get_transformed_bone_position(ped, p2, bone, 0);
        if rd::<u32>(ped, 0x528) & 4 != 0 {
            let r = ug_ik_is_facing_target(ik(), ped, slot);
            let f = rd::<u32>(ped, 0x528);
            wr::<u32>(ped, 0x528, if r != 0 { f | 1 } else { f & 0xFFFF_FFFE });
        }
    }
    // `test ah, 0x44; jnp` skips on equality: the position is used only when x != 0 and y != 0.
    let use_pos = nonzero(rd::<f32>(this, 0x20)) && nonzero(rd::<f32>(this, 0x24));
    let tpos = if use_pos { this.byte_add(0x20) } else { GPtr::NULL };
    let r = ug_weapon_fire(
        active_weapon(ped),
        ped,
        p1,
        p2,
        rd::<GPtr<u8>>(this, 0x1C),
        tpos,
        GPtr::NULL,
    );
    if rd::<u32>(active_weapon(ped), 4) == 1 {
        return ug_ped_do_gun_flash(ped, u32::from(g::<u16>(0x8D2E88).read()), left);
    }
    r
}

/// `CTaskSimpleUseGun::AimGun(CPed*)` (0x61ED10, `ret 4`; its tail continues through the protection island
/// 0x40164F, a plain `test`/`jmp` back). Point-arm speed/blend come from [0x8D2E70]/[0x8D2E74] (1.0/0 once
/// when +0x3B is set, which is cleared). Arm IK mode (ped+0x528 bit 2) is set for aim-with-arm weapons
/// when not ducking (bit 4 cleared), else bit 2 cleared and bit 4 set (AbortIK first when bit 4 was clear).
/// With a target: its aim point (a ped: player data's offset +0x74 transformed by bone +0x70, without
/// player data bone 3; other entities: their position); non-players `Say(0xDC)`; arm mode: look-at (look
/// IK unused and the anim's blend > 0.98) and point arm 0 (arm 1 too for twin pistols unless ped flag
/// 0x20000) at the target, then the torso rotation towards the target position; otherwise
/// `PointGunAtPosition`. Without a target: arm mode with a player aiming through the camera (player data
/// +0x34 bit 3, zero target position, camera mode 0x35 or 0x41) points at
/// `Find3rdPersonCamTargetVector(20, pos + (0, 0, 0.7))`; else at the target position (only when both x
/// and y are non-zero; that look-at does not set +0x3A) or at bone 0x16 offset (0, 2, 0). Non-arm mode:
/// `PointGunAtPosition` or `PointGunInDirection`. Twin pistols with ped flag 0x20000 finally abort arm 1.
#[hook(addr = 0x61ED10, conv = thiscall, name = "CTaskSimpleUseGun::AimGun", expect = "83 EC 2C A1 70 2E 8D 00")]
pub fn use_gun_aim_gun(this: GPtr<u8>, ped: GPtr<u8>) {
    let mut speed = g::<u32>(0x8D2E70).read();
    let mut blend = g::<u32>(0x8D2E74).read();
    if rd::<u8>(this, 0x3B) != 0 {
        blend = 0;
        speed = 0x3F80_0000;
        wr::<u8>(this, 0x3B, 0);
    }
    if rd::<u32>(ped, 0x46C) & 0x400_0000 == 0 && wi_flags(this) & 2 != 0 {
        let f = rd::<u32>(ped, 0x528) | 4;
        wr::<u32>(ped, 0x528, f);
        wr::<u32>(ped, 0x528, f & 0xFFFF_FFEF);
    } else {
        if rd::<u32>(ped, 0x528) & 0x10 == 0 {
            ug_abort_ik_orig(this, ped);
        }
        let f = rd::<u32>(ped, 0x528) & 0xFFFF_FFFB;
        wr::<u32>(ped, 0x528, f);
        wr::<u32>(ped, 0x528, f | 0x10);
    }
    let name_look: u32 = 0x86D760;
    let name_arm: u32 = 0x86D74C;
    let amount = |this: GPtr<u8>| x(rd::<f32>(rd::<GPtr<u8>>(this, 0x2C), 0x18));
    let look_ok = |this: GPtr<u8>| !amount(this).fcom(cf(0x863E18)).test_any(0x41);
    let twin =
        |this: GPtr<u8>, ped: GPtr<u8>| wi_flags(this) & 0x800 != 0 && rd::<u32>(ped, 0x474) & 0x20000 == 0;
    let pedik = ped.byte_add(0x50C);
    let mut p = [0u32; 3];
    let pp: GPtr<u8> = GPtr::from_raw(&mut p as *mut [u32; 3]).cast();
    let target = rd::<GPtr<u8>>(this, 0x1C);
    if !target.is_null() {
        if rd::<u8>(target, 0x36) & 7 == 3 {
            let pd = rd::<GPtr<u8>>(ped, 0x480);
            if !pd.is_null() {
                wr::<u32>(pp, 0, rd::<u32>(pd, 0x74));
                wr::<u32>(pp, 4, rd::<u32>(pd, 0x78));
                wr::<u32>(pp, 8, rd::<u32>(pd, 0x7C));
                ug_ped_get_transformed_bone_position(target, pp, rd::<u32>(pd, 0x70), 0);
            } else {
                ug_ped_get_bone_position(target, pp, 3, 0);
            }
        } else {
            let pos = ent_pos(target);
            wr::<u32>(pp, 0, rd::<u32>(pos, 0));
            wr::<u32>(pp, 4, rd::<u32>(pos, 4));
            wr::<u32>(pp, 8, rd::<u32>(pos, 8));
        }
        if ug_ped_is_player(ped) == 0 {
            ug_ped_say(ped, 0xDC, 0, 0x3F80_0000, 0, 0, 0);
        }
        if rd::<u32>(ped, 0x528) & 4 != 0 {
            if !(rd::<u8>(this, 0x39) != 0 && rd::<u8>(this, 0x3A) != 0) {
                let mut bone: i32 = -1;
                if rd::<u8>(rd::<GPtr<u8>>(this, 0x1C), 0x36) & 7 == 3 {
                    bone = if ug_ped_is_player(ped) != 0 {
                        rd::<i32>(rd::<GPtr<u8>>(ped, 0x480), 0x70)
                    } else {
                        3
                    };
                }
                if rd::<u8>(this, 0x3A) == 0 && look_ok(this) {
                    ug_ik_look_at(
                        ik(),
                        name_look,
                        ped,
                        rd::<GPtr<u8>>(this, 0x1C),
                        9_999_999,
                        bone,
                        GPtr::NULL,
                        1,
                        0x3E80_0000,
                        0xFA,
                        3,
                        0,
                    );
                    wr::<u8>(this, 0x3A, 1);
                }
                if rd::<u8>(this, 0x39) == 0 {
                    ug_ik_point_arm(
                        ik(),
                        name_arm,
                        0,
                        ped,
                        rd::<GPtr<u8>>(this, 0x1C),
                        bone,
                        GPtr::NULL,
                        speed,
                        blend,
                        0x42C8_0000,
                    );
                    if twin(this, ped) {
                        ug_ik_point_arm(
                            ik(),
                            name_arm,
                            1,
                            ped,
                            rd::<GPtr<u8>>(this, 0x1C),
                            bone,
                            GPtr::NULL,
                            speed,
                            blend,
                            0x42C8_0000,
                        );
                    }
                    wr::<u8>(this, 0x39, 1);
                }
            }
            ug_pedik_rotate_torso_for_arm(pedik, ent_pos(rd::<GPtr<u8>>(this, 0x1C)));
        } else {
            ug_pedik_point_gun_at_position(pedik, pp, f32_of(amount(this)).to_bits());
        }
    } else if rd::<u32>(ped, 0x528) & 4 != 0 {
        let pd = rd::<GPtr<u8>>(ped, 0x480);
        if !pd.is_null()
            && rd::<u8>(pd, 0x34) & 8 != 0
            && !nonzero(rd::<f32>(this, 0x20))
            && !nonzero(rd::<f32>(this, 0x24))
            && matches!(g::<u16>(0xB6F1A8).read(), 0x35 | 0x41)
        {
            wr::<u32>(pp, 0, 0);
            wr::<u32>(pp, 4, 0);
            wr::<u32>(pp, 8, 0x3F33_3333);
            let mut sum = [0u32; 3];
            let sp: GPtr<u8> = GPtr::from_raw(&mut sum as *mut [u32; 3]).cast();
            let r = ug_vector_add(sp, ent_pos(ped), pp);
            let mut cam = [0u32; 3];
            let cp: GPtr<u8> = GPtr::from_raw(&mut cam as *mut [u32; 3]).cast();
            let (sx, sy, sz) = (rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8));
            ug_find_3rd_person_cam_target_vector(
                GPtr::from_addr(THE_CAMERA),
                0x41A0_0000,
                sx,
                sy,
                sz,
                cp,
                pp,
            );
            if look_ok(this) {
                ug_ik_look_at(
                    ik(),
                    name_look,
                    ped,
                    GPtr::NULL,
                    9_999_999,
                    -1,
                    pp,
                    1,
                    0x3E80_0000,
                    0xFA,
                    3,
                    0,
                );
                wr::<u8>(this, 0x3A, 1);
            }
            ug_ik_point_arm(
                ik(),
                name_arm,
                0,
                ped,
                GPtr::NULL,
                -1,
                pp,
                speed,
                blend,
                0x42C8_0000,
            );
            if twin(this, ped) {
                ug_ik_point_arm(
                    ik(),
                    name_arm,
                    1,
                    ped,
                    GPtr::NULL,
                    -1,
                    pp,
                    speed,
                    blend,
                    0x42C8_0000,
                );
            }
            wr::<u8>(this, 0x39, 1);
            ug_pedik_rotate_torso_for_arm(pedik, pp);
        } else if !(rd::<u8>(this, 0x39) != 0 && rd::<u8>(this, 0x3A) != 0) {
            let tp = this.byte_add(0x20);
            if nonzero(rd::<f32>(this, 0x20)) && nonzero(rd::<f32>(this, 0x24)) {
                if look_ok(this) {
                    ug_ik_look_at(
                        ik(),
                        name_look,
                        ped,
                        GPtr::NULL,
                        9_999_999,
                        -1,
                        tp,
                        1,
                        0x3E80_0000,
                        0xFA,
                        3,
                        0,
                    );
                }
                ug_ik_point_arm(
                    ik(),
                    name_arm,
                    0,
                    ped,
                    GPtr::NULL,
                    -1,
                    tp,
                    speed,
                    blend,
                    0x42C8_0000,
                );
                if twin(this, ped) {
                    ug_ik_point_arm(
                        ik(),
                        name_arm,
                        1,
                        ped,
                        GPtr::NULL,
                        -1,
                        tp,
                        speed,
                        blend,
                        0x42C8_0000,
                    );
                }
            } else {
                wr::<u32>(pp, 0, 0);
                wr::<u32>(pp, 4, 0x4000_0000);
                wr::<u32>(pp, 8, 0);
                if look_ok(this) && rd::<u8>(this, 0x3A) == 0 {
                    ug_ik_look_at(
                        ik(),
                        name_look,
                        ped,
                        ped,
                        9_999_999,
                        0x16,
                        pp,
                        1,
                        0x3E80_0000,
                        0xFA,
                        3,
                        0,
                    );
                    wr::<u8>(this, 0x3A, 1);
                }
                ug_ik_point_arm(ik(), name_arm, 0, ped, ped, 0x16, pp, speed, blend, 0x42C8_0000);
                if twin(this, ped) {
                    ug_ik_point_arm(ik(), name_arm, 1, ped, ped, 0x16, pp, speed, blend, 0x42C8_0000);
                }
            }
            wr::<u8>(this, 0x39, 1);
        }
    } else if nonzero(rd::<f32>(this, 0x20)) && nonzero(rd::<f32>(this, 0x24)) {
        ug_pedik_point_gun_at_position(pedik, this.byte_add(0x20), f32_of(amount(this)).to_bits());
    } else if ug_ped_is_player(ped) != 0 {
        let a = f32_of(amount(this)).to_bits();
        let d = rd::<u32>(rd::<GPtr<u8>>(ped, 0x480), 0x54);
        ug_pedik_point_gun_in_direction(pedik, rd::<u32>(ped, 0x558), d, 0, a);
    } else {
        let a = f32_of(amount(this)).to_bits();
        ug_pedik_point_gun_in_direction(pedik, rd::<u32>(ped, 0x73C), 0, 0, a);
    }
    if wi_flags(this) & 0x800 != 0
        && rd::<u32>(ped, 0x474) & 0x20000 != 0
        && ug_ik_is_arm_pointing(ik(), 1, ped) != 0
    {
        ug_ik_abort_point_arm(ik(), 1, ped, 0xFA);
    }
}

/// `CTaskSimpleUseGun::FinishGunAnimCB(CAnimBlendAssociation*, void* task)` (0x61F3A0, cdecl): with burst
/// shots left (> 0, signed) and a reload anim (0xE2/0xE3) finishing during RELOAD, a pending command below
/// END_LEISURE becomes RELOAD; the task's anim pointer is cleared when it is this anim.
#[hook(addr = 0x61F3A0, conv = cdecl, name = "CTaskSimpleUseGun::FinishGunAnimCB", expect = "8B 44 24 08 66 83 78 36 00")]
pub fn use_gun_finish_gun_anim_cb(anim: GPtr<u8>, task: GPtr<u8>) {
    if rd::<i16>(task, 0x36) > 0 {
        let id = rd::<u16>(anim, 0x2C);
        if (id == 0xE2 || id == 0xE3) && rd::<u8>(task, 0x10) == 4 && rd::<i8>(task, 0xF) < 6 {
            wr::<u8>(task, 0xF, 4);
        }
    }
    if rd::<GPtr<u8>>(task, 0x2C) == anim {
        wr::<u32>(task, 0x2C, 0);
    }
}

/// `CTaskSimpleUseGun` scalar deleting destructor (0x624DA0): the complete destructor, then
/// `CTask::operator delete` when `flags & 1`; returns this.
#[hook(addr = 0x624DA0, conv = thiscall, name = "CTaskSimpleUseGun::~CTaskSimpleUseGun", expect = "56 8B F1 E8 88 91 FF FF")]
pub fn use_gun_scalar_deleting_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    ug_destructor_orig(this);
    if flags as u8 & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleUseGun::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x624E30). Priorities other
/// than 1 and 2: +0xF = END_LEISURE, false. Otherwise `RemoveStanceAnims(ped, -4)`; immediate (2) blends
/// the idle (group, 3, 1000.0) and gives the gun anim blend delta -1000; the anim's delete callback is
/// reset and +0x2C cleared; finished; `AbortIK`; the attack-button counter (player data +0x2C) is zeroed.
#[hook(addr = 0x624E30, conv = thiscall, name = "CTaskSimpleUseGun::MakeAbortable", expect = "53 8B 5C 24 0C")]
pub fn use_gun_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, _event: GPtr<u8>) -> u8 {
    if priority != 1 && priority != 2 {
        wr::<u8>(this, 0xF, 6);
        return 0;
    }
    ug_remove_stance_anims_orig(this, ped, MINUS_FOUR);
    if priority == 2 {
        ug_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), 3, 0x447A_0000);
    }
    let a = rd::<GPtr<u8>>(this, 0x2C);
    if !a.is_null() {
        if priority == 2 {
            wr::<u32>(a, 0x1C, 0xC47A_0000);
        }
        ug_set_delete_callback(rd::<GPtr<u8>>(this, 0x2C), DEFAULT_ANIM_CB, 0);
        wr::<u32>(this, 0x2C, 0);
    }
    wr::<u8>(this, 8, 1);
    ug_abort_ik_orig(this, ped);
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    if !pd.is_null() {
        wr::<u32>(pd, 0x2C, 0);
    }
    1
}

/// `CTaskSimpleUseGun::SetPedPosition(CPed*)` (0x624ED0): while the active weapon's info is still +0x30,
/// fires the right (+0xD bit 0) and left (bit 1) guns and clears +0xD. Returns false.
#[hook(addr = 0x624ED0, conv = thiscall, name = "CTaskSimpleUseGun::SetPedPosition", expect = "53 56 57 8B 7C 24 10")]
pub fn use_gun_set_ped_position(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let ty = rd::<u32>(active_weapon(ped), 0);
    let skill = ug_ped_get_weapon_skill(ped);
    let wi = ug_get_weapon_info(ty, skill);
    if rd::<GPtr<u8>>(this, 0x30) == wi {
        if rd::<u8>(this, 0xD) & 1 != 0 {
            ug_fire_gun_orig(this, ped, 0);
        }
        if rd::<u8>(this, 0xD) & 2 != 0 {
            ug_fire_gun_orig(this, ped, 1);
        }
        wr::<u8>(this, 0xD, 0);
    }
    0
}

/// `CTaskSimpleUseGun::StartAnim(CPed*)` (0x624F30, `ret 4`). A previous anim is released (END_NOW with
/// blend delta > -8, blend > 0 and last command below RELOAD: delta = -8 first). By the next command
/// (+0xF, jump table 0x625230; values outside 0..7 just move it to +0x10): NONE / END_LEISURE and END_NOW
/// remove the stance anims (4.0 / 8.0) and finish; AIM/FIRE/FIREBURST blend the fire anim (0xE0, crouch
/// 0xE1 for ducking crouch-fire weapons) of the weapon's group, unless AIM without the need to fire (no
/// movement or weapon flag 0x20, and the duck task not used by others) only turns a running FIRE/FIREBURST
/// into AIM, or the duck task refuses (`StopFireGun`); during RELOAD with flag 0x4000 the new anim starts
/// at the loop start; RELOAD blends 0xE2 (crouch 0xE3) and starts it while burst shots (set to 1, 2 for
/// twin pistols, on a new reload) remain; PISTOLWHIP blends 0xD6 (+1 ducking) of group [0xC17730]. The
/// finish callback is `FinishGunAnimCB` (0x61F3A0); +0x10 = +0xF, +0xF = 0.
#[hook(addr = 0x624F30, conv = thiscall, name = "CTaskSimpleUseGun::StartAnim", expect = "53 55 56 57 8B 7C 24 14")]
pub fn use_gun_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let duck = if rd::<u32>(ped, 0x46C) & 0x400_0000 != 0 {
        ug_get_task_duck(rd::<GPtr<u8>>(ped, 0x47C), 1)
    } else {
        GPtr::NULL
    };
    let a = rd::<GPtr<u8>>(this, 0x2C);
    if !a.is_null() {
        if rd::<u8>(this, 0xF) == 7
            && !x(rd::<f32>(a, 0x1C)).fcom(cf(0x85B320)).test_any(0x41)
            && !x(rd::<f32>(a, 0x18)).fcom(cf(0x858B50)).test_any(0x41)
            && rd::<i8>(this, 0x10) < 4
        {
            wr::<u32>(a, 0x1C, 0xC100_0000);
        }
        ug_set_delete_callback(rd::<GPtr<u8>>(this, 0x2C), DEFAULT_ANIM_CB, 0);
        wr::<u32>(this, 0x2C, 0);
    }
    let finish = |this: GPtr<u8>| {
        wr::<u8>(this, 0x10, rd::<u8>(this, 0xF));
        wr::<u8>(this, 0xF, 0);
    };
    let cmd = rd::<i8>(this, 0xF);
    let data = this.addr() as u32;
    match cmd {
        0 | 6 | 7 => {
            ug_remove_stance_anims_orig(this, ped, if cmd == 7 { EIGHT } else { 0x4080_0000 });
            wr::<u8>(this, 8, 1);
            finish(this);
        }
        1..=3 => {
            if cmd == 1 {
                let moving = nonzero(rd::<f32>(this, 0x14)) || nonzero(rd::<f32>(this, 0x18));
                let to_aim = if moving && wi_flags(this) & 0x20 == 0 {
                    true
                } else {
                    !duck.is_null() && ug_duck_in_use_by_other_tasks(duck) != 0
                };
                if to_aim {
                    let l = rd::<u8>(this, 0x10);
                    if l == 2 || l == 3 {
                        wr::<u8>(this, 0x10, 1);
                    }
                    return;
                }
            } else if !duck.is_null() && ug_duck_stop_fire_gun(duck) != 0 {
                return;
            }
            if cmd == 3 {
                wr::<u16>(this, 0x36, rd::<u16>(this, 0x34));
            }
            let wi = rd::<GPtr<u8>>(this, 0x30);
            let anim_id = if rd::<u32>(ped, 0x46C) & 0x400_0000 != 0 && rd::<u32>(wi, 0x18) & 0x2000 != 0 {
                0xE1
            } else {
                0xE0
            };
            let a = ug_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(wi, 0x1C), anim_id, EIGHT);
            wr::<GPtr<u8>>(this, 0x2C, a);
            if rd::<u8>(this, 0x10) == 4 && wi_flags(this) & 0x4000 != 0 {
                let t = ug_wi_loop_start(rd::<GPtr<u8>>(this, 0x30), ducking(ped));
                ug_set_current_time(
                    rd::<GPtr<u8>>(this, 0x2C),
                    f32_of(X87::from_f64_exact(t)).to_bits(),
                );
                let a = rd::<GPtr<u8>>(this, 0x2C);
                wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xFE);
            }
            ug_set_finish_callback(rd::<GPtr<u8>>(this, 0x2C), FINISH_GUN_ANIM_CB, data);
            finish(this);
        }
        4 => {
            if !duck.is_null() && ug_duck_stop_fire_gun(duck) != 0 {
                return;
            }
            if rd::<u8>(this, 0x10) != 4 {
                wr::<u16>(this, 0x36, if wi_flags(this) & 0x800 != 0 { 2 } else { 1 });
            }
            if rd::<i16>(this, 0x36) > 0 {
                let wi = rd::<GPtr<u8>>(this, 0x30);
                let anim_id = if rd::<u32>(ped, 0x46C) & 0x400_0000 != 0 && rd::<u32>(wi, 0x18) & 0x2000 != 0
                {
                    0xE3
                } else {
                    0xE2
                };
                let a = ug_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(wi, 0x1C), anim_id, EIGHT);
                wr::<GPtr<u8>>(this, 0x2C, a);
                ug_assoc_start(a, 0);
                ug_set_finish_callback(rd::<GPtr<u8>>(this, 0x2C), FINISH_GUN_ANIM_CB, data);
                let n = rd::<u8>(this, 0xF);
                wr::<u16>(this, 0x36, rd::<u16>(this, 0x36).wrapping_sub(1));
                wr::<u8>(this, 0x10, n);
                wr::<u8>(this, 0xF, 0);
            } else {
                wr::<u8>(this, 0xF, 0);
                finish(this);
            }
        }
        5 => {
            if !duck.is_null() && ug_duck_in_use_by_other_tasks(duck) != 0 {
                return;
            }
            let a = ug_blend_animation(
                rd::<u32>(ped, 0x18),
                g::<u32>(0xC17730).read(),
                0xD6 | ducking(ped),
                EIGHT,
            );
            wr::<GPtr<u8>>(this, 0x2C, a);
            ug_set_finish_callback(a, FINISH_GUN_ANIM_CB, data);
            finish(this);
        }
        _ => finish(this),
    }
}

/// `CPedIntelligence::GetTaskUseGun()` (0x600F70): the secondary attack task (slot 0) when its type is
/// 0x3F9 (`GetTaskSecondary(0)` is called up to three times), else null.
#[hook(addr = 0x600F70, conv = thiscall, name = "CPedIntelligence::GetTaskUseGun", expect = "56 8D 71 04 6A 00")]
pub fn ped_intelligence_get_task_use_gun(this: GPtr<u8>) -> GPtr<u8> {
    let tm = this.byte_add(4);
    if ug_task_manager_get_task_secondary(tm, 0).is_null() {
        return GPtr::NULL;
    }
    let t = ug_task_manager_get_task_secondary(tm, 0);
    if ug_v_get_task_type(t) != 0x3F9 {
        return GPtr::NULL;
    }
    ug_task_manager_get_task_secondary(tm, 0)
}

/// ProcessPed tail 12a: `RemoveStanceAnims(ped, -4)` then 12b.
fn pp_finish_a(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    ug_remove_stance_anims_orig(this, ped, MINUS_FOUR);
    pp_finish_b(this, ped)
}

/// ProcessPed tail 12b (0x62AF27): `AbortIK`, the attack-button counter zeroed, true.
fn pp_finish_b(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    ug_abort_ik_orig(this, ped);
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    if !pd.is_null() {
        wr::<u32>(pd, 0x2C, 0);
    }
    1
}

fn anim_of(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0x2C)
}

fn cur(a: GPtr<u8>) -> f32 {
    rd::<f32>(a, 0x20)
}

fn step(a: GPtr<u8>) -> f32 {
    rd::<f32>(a, 0x28)
}

fn is_fire_cmd(c: i8) -> bool {
    c == 2 || c == 3
}

/// ProcessPed step 9, last command 1..3 with a non-negative blend delta (0x62A7F7..0x62AC99): loop
/// times by the ducking state, pause/resume at the loop start, firing (continuous / single / twin), the
/// not-playing restart logic, refiring past the loop end and the END_NOW breakout.
fn pp_firing(this: GPtr<u8>, ped: GPtr<u8>, anim: GPtr<u8>) {
    let wi = rd::<GPtr<u8>>(this, 0x30);
    let dk = ducking(ped);
    let s = f32_of(X87::from_f64_exact(ug_wi_loop_start(wi, dk)));
    let e = f32_of(X87::from_f64_exact(ug_wi_loop_end(wi, dk)));
    let f = f32_of(X87::from_f64_exact(ug_wi_loop_fire(wi, dk)));
    let f2 = f32_of(x(e).fsub_m(s).fmul_m(g::<f32>(0x858B8C).read()).fadd_m(f));
    let reloading = |ped: GPtr<u8>| rd::<u32>(active_weapon(ped), 4) == 2;
    // 9b (0x62A855)
    if is_fire_cmd(rd::<i8>(this, 0x10)) {
        let less = !x(rd::<f32>(anim, 0x18)).fcom(cf(0x862CD0)).test_parity(5);
        if !less && !reloading(ped) {
            if rd::<u8>(anim, 0x2E) & 1 == 0 && !x(cur(anim)).fcom(x(s)).test_parity(0x44) {
                wr::<u8>(anim, 0x2E, rd::<u8>(anim, 0x2E) | 1);
            }
        } else if rd::<u8>(anim, 0x2E) & 1 != 0
            && !x(cur(anim)).fcom(x(s)).test_any(0x01)
            && !x(step(anim)).fsubr_m(cur(anim)).fcom(x(s)).test_parity(5)
        {
            wr::<u8>(anim, 0x2E, rd::<u8>(anim, 0x2E) & 0xFE);
            ug_set_current_time(anim_of(this), s.to_bits());
        }
    }
    // 9c (0x62A8D9)
    let dec_burst = |this: GPtr<u8>| {
        let c = rd::<i16>(this, 0x36);
        wr::<u8>(this, 0xB, 1);
        if c > 0 {
            wr::<i16>(this, 0x36, c - 1);
        }
    };
    if wi_flags(this) & 0x400 != 0 {
        let dl = rd::<i8>(this, 0x10);
        if is_fire_cmd(dl) {
            let a = anim_of(this);
            if !x(cur(a)).fcom(x(s)).test_any(0x41)
                && !x(cur(a)).fcom(x(e)).test_parity(5)
                && rd::<u8>(a, 0x2E) & 1 != 0
            {
                if rd::<u8>(this, 0xB) != 0
                    && !(dl == 3 && rd::<i16>(this, 0x36) > 0)
                    && !is_fire_cmd(rd::<i8>(this, 0xF))
                {
                    wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xFE);
                    wr::<u32>(anim_of(this), 0x1C, MINUS_FOUR);
                } else {
                    wr::<u8>(this, 0xD, rd::<u8>(this, 0xD) | 1);
                    wr::<u8>(this, 0xB, 1);
                    let n = rd::<i8>(this, 0xF);
                    if n > dl {
                        wr::<i8>(this, 0x10, n);
                    }
                    wr::<u8>(this, 0xF, 0);
                    let c = rd::<i16>(this, 0x36);
                    if rd::<u8>(this, 0x10) == 3 && c > 0 {
                        wr::<i16>(this, 0x36, c - 1);
                    } else {
                        wr::<u16>(this, 0x36, 0);
                    }
                }
            }
        }
    } else {
        let a = anim_of(this);
        if rd::<u8>(a, 0x2E) & 1 != 0
            && !x(cur(a)).fcom(x(f)).test_any(0x41)
            && !x(step(a)).fsubr_m(cur(a)).fcom(x(f)).test_parity(0x41)
            && is_fire_cmd(rd::<i8>(this, 0x10))
        {
            wr::<u8>(this, 0xD, rd::<u8>(this, 0xD) | 1);
            dec_burst(this);
        }
        if wi_flags(this) & 0x800 != 0
            && rd::<u32>(ped, 0x474) & 0x20000 == 0
            && rd::<u8>(a, 0x2E) & 1 != 0
            && !x(cur(a)).fcom(x(f2)).test_any(0x41)
            && !x(step(a)).fsubr_m(cur(a)).fcom(x(f2)).test_parity(0x41)
            && is_fire_cmd(rd::<i8>(this, 0x10))
        {
            wr::<u8>(this, 0xD, rd::<u8>(this, 0xD) | 2);
            dec_burst(this);
        }
    }
    // 9d (0x62AA48)
    let a = anim_of(this);
    let playing = rd::<u8>(a, 0x2E) & 1;
    if playing == 0 {
        // AIM with no higher command pending, or ended (total <= cur) while blending out: nothing here
        // (0x62AB35 then does nothing either, the anim is not playing).
        let dl = rd::<i8>(this, 0x10);
        let skip = (dl == 1 && rd::<i8>(this, 0xF) <= 1)
            || (x(rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10))
                .fcom(x(cur(a)))
                .test_any(0x41)
                && x(rd::<f32>(a, 0x1C)).fcom(cf(0x858B50)).test_any(0x01));
        if !skip {
            if dl > 4 || rd::<i8>(this, 0xF) > 4 {
                if wi_flags(this) & 0x40000 == 0 {
                    wr::<u32>(a, 0x1C, MINUS_FOUR);
                } else {
                    wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
                    let a = anim_of(this);
                    if !x(cur(a)).fcom(x(s)).test_parity(0x41) {
                        ug_set_current_time(a, e.to_bits());
                    }
                }
            } else if !x(rd::<f32>(a, 0x18)).fcom(cf(0x858B50)).test_any(0x41)
                && !x(rd::<f32>(a, 0x1C)).fcom(cf(0x858B50)).test_any(0x01)
                && !reloading(ped)
            {
                wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
            }
            // 0x62AB05
            let n = rd::<i8>(this, 0xF);
            if is_fire_cmd(n) {
                wr::<i8>(this, 0x10, n);
                wr::<u8>(this, 0xF, 0);
                if n == 3 {
                    wr::<u16>(this, 0x36, rd::<u16>(this, 0x34));
                }
            } else if rd::<u8>(this, 0x10) == 1 && n != 1 {
                wr::<u8>(this, 0x10, 0);
            }
        }
    }
    if playing != 0 && rd::<u8>(this, 0x10) == 1 {
        // 0x62AB35
        let stop =
            if x(cur(a)).fcom(x(s)).test_any(0x01) || x(step(a)).fsubr_m(cur(a)).fcom(x(s)).test_parity(5) {
                !x(step(a)).fadd_m(cur(a)).fcom(x(s)).test_any(0x01)
            } else {
                true
            };
        if stop {
            wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xFE);
            ug_set_current_time(anim_of(this), s.to_bits());
        }
    }
    // 9e (0x62AB80)
    let a = anim_of(this);
    if !x(cur(a)).fcom(x(e)).test_any(0x41) && !x(step(a)).fsubr_m(cur(a)).fcom(x(e)).test_parity(0x41) {
        let n = rd::<i8>(this, 0xF);
        let refire = is_fire_cmd(n) || (rd::<u8>(this, 0x10) == 3 && rd::<i16>(this, 0x36) > 0 && n != 4);
        if refire {
            ug_set_current_time(a, s.to_bits());
            let rl = reloading(ped);
            let a = anim_of(this);
            if rl {
                wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xFE);
            } else {
                wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
            }
            let n = rd::<i8>(this, 0xF);
            if is_fire_cmd(n) {
                if n > rd::<i8>(this, 0x10) {
                    wr::<i8>(this, 0x10, n);
                }
                if n == 3 && rd::<u16>(this, 0x36) == 0 {
                    wr::<u16>(this, 0x36, rd::<u16>(this, 0x34));
                }
                wr::<u8>(this, 0xF, 0);
            }
        } else if n == 1 {
            ug_set_current_time(a, s.to_bits());
            let a = anim_of(this);
            wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xFE);
            wr::<u8>(this, 0x10, 1);
            wr::<u8>(this, 0xF, 0);
        }
    }
    // 9f (0x62AC36)
    let a = anim_of(this);
    if !x(cur(a))
        .fcom(x(rd::<f32>(rd::<GPtr<u8>>(this, 0x30), 0x58)))
        .test_any(0x41)
        && rd::<u8>(this, 0xF) == 7
    {
        wr::<u8>(this, 8, 1);
        let d = if rd::<u16>(a, 0x2E) & 0x400 != 0 {
            0xBF80_0000
        } else {
            MINUS_FOUR
        };
        wr::<u32>(a, 0x1C, d);
    }
}

/// `CTaskSimpleUseGun::ProcessPed(CPed*)` (0x62A380, SEH frame). Port of the bytes (see
/// docs/re/specs/AI/CTaskSimpleUseGun.md steps 1-12, corrected here where the bytes differ):
/// IK bookkeeping; weapon info (first frame: bail out while the player's chosen weapon differs, finish for
/// non-firing/thrown weapons; later: `MakeAbortable(ped, 1)` on a weapon change); ped flag 0x8000 of +0x474
/// unless a first-person weapon of a player; finished -> 12a; attack counter decay `* _CIpow(0.96, ts)`;
/// not in control: countdown, finish at 0; in control: blocked arm handling, `StartAnim` / pistol-whip
/// restart, the per-command anim logic, the skip-aim test and `AimGun` (any last command != 0 and below
/// RELOAD, signed, so also -1) or `AbortIK`, then `SetMoveAnim` (unless an arm-aimed weapon, standing and
/// without move control) and the next command reset (kept from END_LEISURE up, and RELOAD right after
/// FIRE/FIREBURST). Stack objects: the CTaskSimpleFight (0x28 bytes) and the vectors are Rust locals.
#[hook(addr = 0x62A380, conv = thiscall, name = "CTaskSimpleUseGun::ProcessPed", expect = "6A FF 64 A1 00 00 00 00")]
pub fn use_gun_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    // the shared stack vector [esp+0x20] (aim point, also the pistol-whip VectorSub copy)
    let mut p = [0u32; 3];
    let pp: GPtr<u8> = GPtr::from_raw(&mut p as *mut [u32; 3]).cast();
    if rd::<u8>(this, 0x3A) != 0 {
        let r = ug_ik_is_looking(ik(), ped);
        if r == 0 {
            wr::<u8>(this, 0x3A, r);
        }
    }
    if rd::<u8>(this, 0x39) != 0 {
        let r = ug_ik_is_arm_pointing(ik(), 0, ped);
        if r == 0 {
            wr::<u8>(this, 0x39, r);
        }
    }
    let had_wi = rd::<u32>(this, 0x30) != 0;
    wr::<u8>(this, 0xD, 0);
    if !had_wi {
        let pd = rd::<GPtr<u8>>(ped, 0x480);
        if !pd.is_null() && rd::<u8>(pd, 0x20) != rd::<u8>(ped, 0x718) {
            return 0;
        }
        let ty = rd::<u32>(active_weapon(ped), 0);
        let skill = ug_ped_get_weapon_skill(ped);
        let wi = ug_get_weapon_info(ty, skill);
        wr::<GPtr<u8>>(this, 0x30, wi);
        if rd::<u32>(wi, 0) != 0 && rd::<u32>(wi, 0x18) & 0x100 == 0 {
            wr::<u32>(this, 0x14, 0);
            wr::<u32>(this, 0x18, 0);
        } else {
            wr::<u8>(this, 8, 1);
            wr::<u32>(this, 0x30, 0);
            return pp_finish_b(this, ped);
        }
    } else {
        let ty = rd::<u32>(active_weapon(ped), 0);
        let skill = ug_ped_get_weapon_skill(ped);
        let wi = ug_get_weapon_info(ty, skill);
        if rd::<GPtr<u8>>(this, 0x30) != wi {
            ug_v_make_abortable(this, ped, 1, GPtr::NULL);
        }
    }
    if wi_flags(this) & 4 == 0 || rd::<u32>(ped, 0x480) == 0 {
        wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 0x8000);
    }
    if rd::<u8>(this, 8) != 0 {
        return pp_finish_a(this, ped);
    }
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    if !pd.is_null()
        && !x(rd::<f32>(pd, 0x2C))
            .fcom(X87::from_f64_exact(g::<f64>(0x859EF8).read()))
            .test_any(0x41)
    {
        // fld [0x8D2E64]; fld ts; call _CIpow (st1 ^ st0)
        let pw = sa_core::orig::x87_binary(0x822130, cf(0x8D2E64).to_f64(), cf(TIME_STEP).to_f64());
        wr::<f32>(
            pd,
            0x2C,
            f32_of(X87::from_f64_exact(pw).fmul_m(rd::<f32>(pd, 0x2C))),
        );
    }
    if rd::<u8>(this, 9) == 0 {
        // 0x62AEFA
        wr::<u8>(this, 0xB, 0);
        wr::<u32>(this, 0x14, 0);
        wr::<u32>(this, 0x18, 0);
        let c = rd::<u8>(this, 0x38);
        wr::<u8>(this, 0x38, c.wrapping_sub(1));
        if c != 0 {
            return 0;
        }
        wr::<u8>(this, 8, 1);
        return pp_finish_a(this, ped);
    }
    wr::<u8>(this, 0x38, 0xFF);
    let duck = ducking(ped);
    let blocked = if duck != 0 {
        rd::<u32>(ped, 0x474) & 0x40000 != 0
    } else {
        rd::<u8>(ped, 0x476) & 1 != 0 && wi_flags(this) & 2 == 0
    };
    // true: continue at 0x62A54D (anim start / per-command logic); false: straight to 0x62A56D
    let go_anim = if blocked {
        if rd::<u8>(this, 0x10) == 5 {
            true
        } else {
            let cl = rd::<i8>(this, 0xF);
            if cl == 5 && anim_of(this).is_null() {
                true
            } else {
                let a = anim_of(this);
                if a.is_null() && !(1..=3).contains(&cl) {
                    true
                } else {
                    if !a.is_null() {
                        wr::<u32>(a, 0x1C, MINUS_FOUR);
                        let a = anim_of(this);
                        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xFE);
                        let a = anim_of(this);
                        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 4);
                    }
                    ug_abort_ik_orig(this, ped);
                    let l = rd::<u8>(this, 0x10);
                    wr::<u8>(this, 0xC, 1);
                    if l == 3 {
                        wr::<u8>(this, 0x10, 1);
                    }
                    false
                }
            }
        }
    } else {
        if rd::<u8>(this, 0xF) == 5 && rd::<i8>(this, 0x10) <= 3 {
            let a = anim_of(this);
            if !a.is_null() {
                ug_set_delete_callback(a, DEFAULT_ANIM_CB, 0);
            }
            wr::<u32>(this, 0x2C, 0);
            ug_abort_ik_orig(this, ped);
        }
        true
    };
    if go_anim {
        let anim = anim_of(this);
        if anim.is_null() {
            ug_start_anim_orig(this, ped);
            if anim_of(this).is_null() {
                ug_abort_ik_orig(this, ped);
            }
        } else {
            // step 9 (0x62A657)
            let cl = rd::<i8>(this, 0x10);
            if cl == 4 {
                let id = rd::<u16>(anim, 0x2C);
                if id != 0xE2 && id != 0xE3 && !x(rd::<f32>(anim, 0x1C)).fcom(cf(0x858B50)).test_any(0x01) {
                    wr::<u32>(anim, 0x1C, MINUS_FOUR);
                }
            } else if cl == 5 {
                let dk = ducking(ped) as usize;
                let t = g::<f32>(0xC17738 + 4 * dk).read();
                if !x(rd::<f32>(anim, 0x18)).fcom(cf(0x858C20)).test_any(0x41)
                    && !x(rd::<f32>(anim, 0x1C)).fcom(cf(0x858B50)).test_any(0x01)
                    && !x(cur(anim)).fcom(x(t)).test_any(0x41)
                    && !x(cur(anim)).fsub(x(step(anim))).fcom(x(t)).test_parity(5)
                {
                    let idx = g::<u8>(0xC17780 + dk).read() as usize;
                    let mut v = [0u32; 3];
                    let vp: GPtr<u8> = GPtr::from_raw(&mut v as *mut [u32; 3]).cast();
                    ug_matrix_mul_vector(
                        vp,
                        rd::<GPtr<u8>>(ped, 0x14),
                        GPtr::from_addr(0xC177D0 + 12 * idx),
                    );
                    let mut fight = [0u32; 10];
                    let fp: GPtr<u8> = GPtr::from_raw(&mut fight as *mut [u32; 10]).cast();
                    ug_fight_ctor(fp, rd::<GPtr<u8>>(this, 0x1C), 0xB, 0x4E20);
                    let a = anim_of(this);
                    wr::<u8>(fp, 0x24, 0x10);
                    wr::<u8>(fp, 0x25, dk as u8);
                    wr::<u8>(fp, 0x27, 0xB);
                    wr::<GPtr<u8>>(fp, 0x1C, a);
                    ug_fight_6240b0(fp, ped, vp);
                    wr::<u32>(fp, 0x1C, 0);
                    ug_fight_dtor(fp);
                }
                let t = rd::<GPtr<u8>>(this, 0x1C);
                if !t.is_null() {
                    let (pp_ped, pp_t) = (ent_pos(ped), ent_pos(t));
                    let mut d = [0u32; 3];
                    let dp: GPtr<u8> = GPtr::from_raw(&mut d as *mut [u32; 3]).cast();
                    let r = ug_vector_sub(dp, pp_t, pp_ped);
                    wr::<u32>(pp, 0, rd::<u32>(r, 0));
                    wr::<u32>(pp, 4, rd::<u32>(r, 4));
                    wr::<u32>(pp, 8, rd::<u32>(r, 8));
                    let h = x(rd::<f32>(pp, 0)).fchs().fpatan(x(rd::<f32>(pp, 4)));
                    wr::<f32>(ped, 0x55C, f32_of(h));
                }
            } else if cl <= 0 {
                if cl == 0
                    && !x(rd::<f32>(anim, 0x18)).fcom(cf(0x858B50)).test_any(0x41)
                    && !x(rd::<f32>(anim, 0x1C)).fcom(cf(0x858B50)).test_any(0x01)
                {
                    wr::<u32>(anim, 0x1C, MINUS_FOUR);
                }
            } else if cl < 4 && !x(rd::<f32>(anim, 0x1C)).fcom(cf(0x858B50)).test_any(0x01) {
                pp_firing(this, ped, anim);
            }
            // step 10 (0x62AC99)
            let mut dead = false;
            wr::<u8>(this, 0xE, 0);
            if rd::<u32>(ped, 0x528) & 4 != 0 {
                let t = rd::<GPtr<u8>>(this, 0x1C);
                if !t.is_null() {
                    if rd::<u8>(t, 0x36) & 7 == 3 {
                        ug_ped_get_bone_position(t, pp, 3, 0);
                        let t = rd::<GPtr<u8>>(this, 0x1C);
                        if !x(rd::<f32>(t, 0x540)).fcom(cf(0x858B50)).test_parity(0x41) {
                            dead = true;
                        }
                    } else {
                        let pos = ent_pos(t);
                        wr::<u32>(pp, 0, rd::<u32>(pos, 0));
                        wr::<u32>(pp, 4, rd::<u32>(pos, 4));
                        wr::<u32>(pp, 8, rd::<u32>(pos, 8));
                    }
                } else if ug_ped_is_player(ped) != 0 && g::<u16>(0xB6F1A8).read() == 0x35 {
                    let mut tmp = [0u32, 0, 0x3F33_3333];
                    let tp: GPtr<u8> = GPtr::from_raw(&mut tmp as *mut [u32; 3]).cast();
                    let mut sum = [0u32; 3];
                    let sp: GPtr<u8> = GPtr::from_raw(&mut sum as *mut [u32; 3]).cast();
                    let r = ug_vector_add(sp, ent_pos(ped), tp);
                    let mut cam = [0u32; 3];
                    let cp: GPtr<u8> = GPtr::from_raw(&mut cam as *mut [u32; 3]).cast();
                    let (sx, sy, sz) = (rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8));
                    ug_find_3rd_person_cam_target_vector(
                        GPtr::from_addr(THE_CAMERA),
                        0x41A0_0000,
                        sx,
                        sy,
                        sz,
                        cp,
                        pp,
                    );
                } else {
                    wr::<u32>(pp, 0, rd::<u32>(this, 0x20));
                    wr::<u32>(pp, 4, rd::<u32>(this, 0x24));
                    wr::<u32>(pp, 8, rd::<u32>(this, 0x28));
                }
                if nonzero(rd::<f32>(pp, 0)) || nonzero(rd::<f32>(pp, 4)) {
                    ug_vector_sub_assign(pp, ent_pos(ped));
                    ug_vector_normalise(pp);
                    let mut h = x(rd::<f32>(pp, 0))
                        .fchs()
                        .fpatan(x(rd::<f32>(pp, 4)))
                        .fsub_m(rd::<f32>(ped, 0x558));
                    if !h.fcom(cf(0x858CB8)).test_any(0x41) {
                        h = h.fsub_m(g::<f32>(0x858CBC).read());
                    } else if !h.fcom(cf(0x858CC0)).test_parity(5) {
                        h = h.fadd_m(g::<f32>(0x858CBC).read());
                    }
                    let k = X87::from_i32(i32::from(dead)).fmul_m(g::<f32>(0x86D624).read());
                    let skip = if !h.fcom(cf(0x86D61C).fsub(k)).test_any(0x41) {
                        true
                    } else {
                        !h.fcom(cf(0x86D620).fadd(k)).test_parity(5)
                    };
                    if skip {
                        wr::<u8>(this, 0xE, 1);
                    }
                    let mut at = [0u32; 3];
                    let atp: GPtr<u8> = GPtr::from_raw(&mut at as *mut [u32; 3]).cast();
                    let r = ug_get_at_direction(ped, atp);
                    if dead {
                        let d = x(rd::<f32>(pp, 4))
                            .fmul_m(rd::<f32>(r, 4))
                            .fadd(x(rd::<f32>(pp, 8)).fmul_m(rd::<f32>(r, 8)))
                            .fadd(x(rd::<f32>(pp, 0)).fmul_m(rd::<f32>(r, 0)));
                        if !d.fcom(cf(0x858C9C)).test_parity(5) {
                            wr::<u8>(this, 0xE, 1);
                        }
                    }
                }
                // 0x62AEB6: the bytes test NOT ducking (gta-reversed's comment has `bIsDucking &&`)
                if rd::<u32>(ped, 0x46C) & 0x400_0000 == 0 && rd::<u8>(ped, 0x476) & 1 != 0 {
                    wr::<u8>(this, 0xE, 1);
                }
            }
            let l = rd::<i8>(this, 0x10);
            if l != 0 && l < 4 && rd::<u8>(this, 0xE) == 0 {
                ug_aim_gun_orig(this, ped);
            } else {
                ug_abort_ik_orig(this, ped);
            }
        }
    }
    // 0x62A56D: the bytes call SetMoveAnim unless aim-with-arm, standing and no move control
    if wi_flags(this) & 2 == 0 || rd::<u32>(ped, 0x46C) & 0x400_0000 != 0 || rd::<u8>(this, 0xA) != 0 {
        ug_set_move_anim_orig(this, ped);
    }
    let n = rd::<i8>(this, 0xF);
    if n < 6 && !(n == 4 && is_fire_cmd(rd::<i8>(this, 0x10))) {
        wr::<u8>(this, 0xF, 0);
    }
    wr::<u8>(this, 9, 0);
    0
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_USE_GUN_CTOR,
    HOOK_USE_GUN_GET_TASK_TYPE,
    HOOK_USE_GUN_DESTRUCTOR,
    HOOK_USE_GUN_ABORT_IK,
    HOOK_USE_GUN_CONTROL_GUN,
    HOOK_USE_GUN_PLAYER_PASSIVE_CONTROL_GUN,
    HOOK_USE_GUN_CONTROL_GUN_MOVE,
    HOOK_USE_GUN_REQUIRE_PISTOL_WHIP,
    HOOK_USE_GUN_SET_MOVE_ANIM,
    HOOK_USE_GUN_REMOVE_STANCE_ANIMS,
    HOOK_USE_GUN_FIRE_GUN,
    HOOK_USE_GUN_AIM_GUN,
    HOOK_USE_GUN_FINISH_GUN_ANIM_CB,
    HOOK_USE_GUN_SCALAR_DELETING_DTOR,
    HOOK_USE_GUN_MAKE_ABORTABLE,
    HOOK_USE_GUN_SET_PED_POSITION,
    HOOK_USE_GUN_START_ANIM,
    HOOK_PED_INTELLIGENCE_GET_TASK_USE_GUN,
    HOOK_USE_GUN_PROCESS_PED,
];
