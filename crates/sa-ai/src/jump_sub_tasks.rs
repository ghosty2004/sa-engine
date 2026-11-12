//! WRITER-NATIVE (batch `native-23`): `CTaskComplexJump::CreateSubTask` (0x67D980), the `CTaskSimpleClimb`
//! constructor (0x67A110) and the destructor body 0x67A060 (vtable 0x870570), reached when the player jumps
//! (LShift) on the native build. Ported from the 1.0 US bytes; gta-reversed (`TaskComplexJump.cpp`) as a reading
//! aid.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new_c(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => crate::mvpa_ai1::task_operator_new;
    /// `CTaskSimple::CTaskSimple()` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => crate::mvpa_ai1::task_simple_ctor;
    /// `CTaskComplex::CTaskComplex()` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CTaskComplex::~CTaskComplex()` body (0x61A3D0)
    fn task_complex_dtor(t: GPtr<u8>) = thiscall 0x61A3D0 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskComplexInAirAndLand(bool usePlayerLaunchForce, bool invalidClimb)` (0x678C80)
    fn in_air_and_land_ctor(t: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x678C80 => unported;
    /// `CTaskSimpleJump(bool canClimb)` (0x679AA0)
    fn simple_jump_ctor(t: GPtr<u8>, can_climb: u32) -> GPtr<u8> = thiscall 0x679AA0 => unported;
    /// `CPedGroups::IsInPlayersGroup(CPed*)` (0x5F7F10)
    fn groups_is_in_players_group(ped: GPtr<u8>) -> u8 = cdecl 0x5F7F10 => unported;
    /// `CTaskSimpleClimb(CEntity*, const CVector&, float heading, uint8 surface, int8 stage, bool force)` (0x67A110)
    #[allow(clippy::too_many_arguments)]
    fn simple_climb_ctor_c(t: GPtr<u8>, e: GPtr<u8>, pos: GPtr<u8>, heading: u32, surface: u32, stage: u32, force: u32) -> GPtr<u8> = thiscall 0x67A110 => simple_climb_ctor;
    /// `CTaskSimpleHitHead()` (0x653060)
    fn hit_head_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x653060 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
}

/// The entity's position (`m_matrix` +0x30, else the placement position +4).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CTaskComplexJump::CreateSubTask(int32 taskType, CPed* ped)` (0x67D980, `ret 8`, SEH frame).
///
/// 0xD2 (jump): `CTaskSimpleJump(+0xC == 1)`, its +0x25 set when +0x10 is set or the ped is in the player's group.
/// 0xF0: `CTaskComplexInAirAndLand(true, false)`, its +0xE set when the current sub-task is 0xFE with +0xC set (the
/// original writes +0xE also when the pool returned null). 0xFE: after a jump sub-task (0xD2) a
/// `CTaskSimpleClimb(sub +0x1C, &sub +8, sub +0x14, sub +0x18, stage, +0xC == 1)` with stage 2 when `sub +0x10 -
/// ped z < [0x8D2F34]` (ordered), else 1; otherwise an inline `CTaskComplexInAirAndLand` (vtable 0x8704AC, +0xC =
/// 1). 0x1F4: `CTaskSimpleHitHead`. 0x516: clears ped +0x46C bit 9, null. Others null. The original passes the
/// climb's stage and force flag through its argument slots (the force byte over the ped pointer's upper bytes).
#[hook(addr = 0x67D980, conv = thiscall, name = "CTaskComplexJump::CreateSubTask",
       expect = "6A FF 68 37 64 84 00",
       unverified = "native-23", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "the climb constructor's stage / force arguments are the type / ped argument slots with a byte replaced")]
pub fn jump_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    match ty {
        0xF0 => {
            let t = task_new_c(0x10);
            let t = if t.is_null() { t } else { in_air_and_land_ctor(t, 1, 0) };
            let sub = rd::<GPtr<u8>>(this, 8);
            if !sub.is_null() && task_get_type(sub) == 0xFE && rd::<u8>(rd::<GPtr<u8>>(this, 8), 0xC) != 0 {
                wr::<u8>(t, 0xE, 1);
            }
            t
        }
        0xD2 => {
            let t = task_new_c(0x2C);
            let t = if t.is_null() { t } else { simple_jump_ctor(t, u32::from(rd::<u32>(this, 0xC) == 1)) };
            if rd::<u8>(this, 0x10) != 0 || groups_is_in_players_group(ped) != 0 {
                wr::<u8>(t, 0x25, 1);
            }
            t
        }
        0xFE => {
            let sub = rd::<GPtr<u8>>(this, 8);
            if !sub.is_null() && task_get_type(sub) == 0xD2 {
                let sub = rd::<GPtr<u8>>(this, 8);
                let mut stage: u32 = 1;
                if !x(rd::<f32>(sub, 0x10))
                    .fsub_m(rd::<f32>(pos(ped), 8))
                    .fcom(x(g::<f32>(0x8D_2F34).read()))
                    .test_parity(0x05)
                {
                    stage = 2;
                }
                let force = (ped.addr() as u32 & 0xFFFF_FF00) | u32::from(rd::<u32>(this, 0xC) == 1);
                let t = task_new_c(0x30);
                if t.is_null() {
                    return t;
                }
                return simple_climb_ctor_c(
                    t,
                    rd::<GPtr<u8>>(sub, 0x1C),
                    sub.byte_add(8),
                    rd::<u32>(sub, 0x14),
                    u32::from(rd::<u8>(sub, 0x18)),
                    stage,
                    force,
                );
            }
            let t = task_new_c(0x10);
            if t.is_null() {
                return t;
            }
            task_complex_ctor(t);
            wr::<u32>(t, 0, 0x87_04AC);
            wr::<u8>(t, 0xC, 1);
            wr::<u8>(t, 0xD, 0);
            wr::<u8>(t, 0xE, 0);
            t
        }
        0x1F4 => {
            let t = task_new_c(0x10);
            if t.is_null() { t } else { hit_head_ctor(t) }
        }
        0x516 => {
            wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFFFF_FDFF);
            GPtr::NULL
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskSimpleClimb::CTaskSimpleClimb(CEntity* climbEntity, const CVector& pos, float heading, uint8 surface,
/// int8 stage, bool forceClimb)` (0x67A110, `ret 0x18`; the entry `push -1; jmp 0x4039B2` pushes the SEH handler
/// from a protection chunk): `CTaskSimple` base, vtable 0x87059C, +8 / +9 / +0xA / +0xC / +0xE = 0, +0x24 = entity
/// (registered reference), +0x2C = 0, +0x18 = pos, +0x14 = heading, +0xF = surface, +0xD = stage, word +0x28 = 0,
/// +0xB = force, +0x10 = 0xFF. Returns `this`.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x67A110, conv = thiscall, name = "CTaskSimpleClimb::Constructor",
       expect = "6A FF E9 9B 98 D8 FF",
       unverified = "native-23", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "entry jmp to the protection chunk 0x4039B2 (SEH handler push)")]
pub fn simple_climb_ctor(this: GPtr<u8>, e: GPtr<u8>, p: GPtr<u8>, heading: u32, surface: u32, stage: u32, force: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0, 0x87_059C);
    for o in [8, 9, 0xA, 0xC] {
        wr::<u8>(this, o, 0);
    }
    wr::<GPtr<u8>>(this, 0x24, e);
    wr::<u32>(this, 0x2C, 0);
    if !e.is_null() {
        entity_register_reference(e, this.byte_add(0x24));
    }
    for o in [0, 4, 8] {
        wr::<u32>(this, 0x18 + o, rd::<u32>(p, o));
    }
    wr::<u32>(this, 0x14, heading);
    wr::<u8>(this, 0xF, surface as u8);
    wr::<u8>(this, 0xD, stage as u8);
    wr::<u8>(this, 0xE, 0);
    wr::<u16>(this, 0x28, 0);
    wr::<u8>(this, 0xB, force as u8);
    wr::<u8>(this, 0x10, 0xFF);
    this
}

/// The destructor body 0x67A060 (vtable 0x870570): sets its vtable and tail-jumps to the `CTaskComplex`
/// destructor body 0x61A3D0.
#[hook(addr = 0x67A060, conv = thiscall, name = "FUN_0067a060",
       expect = "C7 01 70 05 87 00 E9",
       unverified = "native-23", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn jump_task_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x87_0570);
    task_complex_dtor(this);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_JUMP_CREATE_SUB_TASK, HOOK_SIMPLE_CLIMB_CTOR, HOOK_JUMP_TASK_DTOR];
