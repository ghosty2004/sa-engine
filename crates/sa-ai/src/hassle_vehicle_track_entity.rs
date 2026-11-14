//! `CTaskGangHassleVehicle`'s destructor and `CTaskComplexTrackEntity::CreateFirstSubTask` (INTRO-2: the gang
//! peds that hassle the intro's police car). Ported from the 1.0 US bytes; gta-reversed as a reading aid.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, native, orig};

use crate::common::*;

orig! {
    /// `CVehicle::SetHasslePosId(int32, bool)` (0x6D3B30, `ret 8`)
    fn vehicle_set_hassle_pos_id(v: GPtr<u8>, id: i32, b: u32) = thiscall 0x6D3B30 => unported;
    /// `CEntity::CleanUpOldReference(CEntity** ref)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990)
    fn anim_get_block_index(name: u32) -> i32 = cdecl 0x4D3990 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0)
    fn anim_remove_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CTaskComplex::~CTaskComplex()` (0x61A3D0)
    fn task_complex_dtor(t: GPtr<u8>) = thiscall 0x61A3D0 => unported;
    /// `CTaskComplex::CreateNextSubTask`-slot +0x20 of the task's vtable (the tail jump of 0x65F700)
    fn v_slot_20(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x20 => unported;
}

/// `CTaskGangHassleVehicle::~CTaskGangHassleVehicle()` (0x65FB60, thiscall; entry `push -1; jmp 0x4039EE`, the
/// protection chunk that pushes the SEH handler).
///
/// Vtable 0x86F9D4. With a vehicle (+0x10): `SetHasslePosId(+0x14, false)` when +0x14 > -1, then (re-read) its
/// `CleanUpOldReference(&+0x10)`. With an entity at +0x34: `CleanUpOldReference(&+0x34)`. With the anim-block flag
/// (+0x30): `RemoveAnimBlockRef(GetAnimationBlockIndex(0x86F92C))`, flag 0. Then `CTaskComplex::~CTaskComplex`.
///
/// Registered for phase B only (`#[native]`): protection code reads the entry's jmp as a key (`adc ecx, [0x65FB62]`
/// at 0x401A26). The SEH frame is not reproduced.
#[native(addr = 0x65FB60, conv = thiscall, name = "CTaskGangHassleVehicle::Destructor")]
pub fn gang_hassle_vehicle_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x0086_F9D4);
    let veh = rd::<GPtr<u8>>(this, 0x10);
    if !veh.is_null() {
        let id = rd::<i32>(this, 0x14);
        if id > -1 {
            vehicle_set_hassle_pos_id(veh, id, 0);
        }
        let veh = rd::<GPtr<u8>>(this, 0x10);
        if !veh.is_null() {
            entity_clean_up_old_reference(veh, this.byte_add(0x10));
        }
    }
    let e = rd::<GPtr<u8>>(this, 0x34);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(0x34));
    }
    if rd::<u8>(this, 0x30) != 0 {
        anim_remove_block_ref(anim_get_block_index(0x86F92C));
        wr::<u8>(this, 0x30, 0);
    }
    task_complex_dtor(this);
}

/// `CTaskComplexTrackEntity::CreateFirstSubTask(CPed*)` (0x65F700, thiscall, `ret 4`).
///
/// When the stored move state factor (+0x3C) is not below 0.0 (`test ah, 5; jp` taken; NaN too) nothing changes;
/// below 0.0 it becomes 0 / 1 / 2 / 3 for the ped's move state (+0x534) 1 / 4 / 6 / other. Then the tail call of
/// vtable slot +0x20 (`CreateNextSubTask`) with the ped.
#[hook(addr = 0x65F700, conv = thiscall, name = "CTaskComplexTrackEntity::CreateFirstSubTask", expect = "D9 41 3C 8B 54 24 04",
       unverified = "intro3-13", route = "L2", hazards = "heap-writes", recs = "intro")]
pub fn track_entity_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if !x(rd::<f32>(this, 0x3C)).fcom(x(0.0)).test_parity(0x05) {
        let v: u32 = match rd::<i32>(ped, 0x534) {
            1 => 0,
            4 => 0x3F80_0000,
            6 => 0x4000_0000,
            _ => 0x4040_0000,
        };
        wr::<u32>(this, 0x3C, v);
    }
    v_slot_20(this, ped)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_TRACK_ENTITY_CREATE_FIRST_SUB_TASK];

/// This module's `#[native]` registrations.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_GANG_HASSLE_VEHICLE_DTOR];
