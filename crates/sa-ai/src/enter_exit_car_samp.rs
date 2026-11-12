//! Car enter / exit task pieces the original executes under SA:MP 0.3.7 (`catalog/samp-priority.csv`):
//! `CTaskComplexEnterCar` constructor / destructor, `CTaskComplexEnterCarAsDriverTimed` constructor,
//! `CTaskComplexLeaveBoat::CreateSubTask`, the open / close door from outside constructors and destructor, the
//! vehicle anim group critical blend time helper and `CTaskComplexGotoDoorAndOpen::MakeAbortable` (PORT-AI-7).
//! Ported from the 1.0 US bytes (the exe wins over gta-reversed). Every call to another catalogued function goes
//! through `orig!` at the original address. Entries starting `push -1; jmp` go through protection islands that
//! only push the (decoded) SEH handler and jump back; the SEH frame has no Rust counterpart.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CVehicleAnimGroupData::m_vehicleAnimGroups` (0x94 bytes each)
const VEH_ANIM_GROUPS: u32 = 0xC1CDC0;
/// `CDefaultAnimCallback::DefaultAnimCB`
const DEFAULT_ANIM_CB: u32 = 0x4D1390;

orig! {
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18, `ret 0xC`)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CTaskComplex::~CTaskComplex` (`FUN_0061a3d0`)
    fn task_complex_dtor(t: GPtr<u8>) = thiscall 0x61A3D0 => unported;
    /// `CTaskSimple::CTaskSimple` (`FUN_0061a390`)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CTaskSimple::~CTaskSimple` (`FUN_0061a3a0`)
    fn task_simple_dtor(t: GPtr<u8>) = thiscall 0x61A3A0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CTaskUtilityLineUpPedWithCar::~CTaskUtilityLineUpPedWithCar` (0x64FC00)
    fn line_up_dtor(u: GPtr<u8>) = thiscall 0x64FC00 => unported;
    /// CRT `free` (0x8214BD; `operator delete` of the line-up utility)
    fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `CVehicle::ClearGettingInFlags(uint8)` (0x6D3040, `ret 4`)
    fn vehicle_clear_getting_in_flags(v: GPtr<u8>, f: u32) = thiscall 0x6D3040 => unported;
    /// `CTaskSimpleCarSetPedOut::CTaskSimpleCarSetPedOut(CVehicle*, int32 door, bool)` (0x6478B0, `ret 0xC`)
    fn car_set_ped_out_ctor(t: GPtr<u8>, veh: GPtr<u8>, door: u32, b: u32) -> GPtr<u8> = thiscall 0x6478B0 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(cb, data)` (0x4CEBE0, `ret 8`)
    fn assoc_set_finish_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBE0 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(i: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CVehicleAnimGroup::GetGroup(int32 animId)` (0x6E3B00, `ret 4`)
    fn veh_anim_group_get_group(g: GPtr<u8>, anim: u32) -> u32 = thiscall 0x6E3B00 => unported;
    /// `CVehicleAnimGroup::ComputeCriticalBlendTime(int32 animId)` (0x6E3C80, `ret 4`, ST0)
    fn veh_anim_group_compute_critical_blend_time(g: GPtr<u8>, anim: u32) -> f64 = thiscall 0x6E3C80 => unported;
}

/// Registers `slot`'s entity (if any) as a reference to `slot`.
fn register_if_set(slot: GPtr<u8>) {
    let e = rd::<GPtr<u8>>(slot, 0);
    if !e.is_null() {
        entity_register_reference(e, slot);
    }
}

/// `CTaskComplexEnterCar::CTaskComplexEnterCar(CVehicle*, bool asDriver, bool quitAfterOpeningDoor, bool
/// quitAfterDraggingPedOut, bool carryOnAfterFallingOff)` (0x63A220, `ret 0x14`): `CTaskComplex` base,
/// `vtbl = 0x86E6F0`, the vehicle at +0xC (registered when set), flag byte +0x10 = the four flags in bits 0..3 over
/// the old upper nibble, +0x14..+0x20 = 0, +0x24 = 6, bytes +0x28 / +0x29 / +0x3C / +0x44 = 0, +0x38 / +0x40 = 0,
/// +0x48 = -1.0.
#[hook(addr = 0x63A220, conv = thiscall, name = "CTaskComplexEnterCar::Constructor", expect = "6A FF 68 68 13 84 00",
       route = "L2", hazards = "heap-writes", recs = "car-ai")]
pub fn enter_car_ctor(this: GPtr<u8>, veh: GPtr<u8>, b1: u32, b2: u32, b3: u32, b4: u32) -> GPtr<u8> {
    task_complex_ctor(this);
    let low = ((((b4 as u8 & 1) << 1 | (b3 as u8 & 1)) << 1 | (b2 as u8 & 1)) << 1) | (b1 as u8 & 1);
    let flags = low | (rd::<u8>(this, 0x10) & 0xF0);
    wr::<u32>(this, 0, 0x86E6F0);
    wr::<GPtr<u8>>(this, 0xC, veh);
    wr::<u8>(this, 0x10, flags);
    for off in [0x14, 0x18, 0x1C, 0x20] {
        wr::<u32>(this, off, 0);
    }
    wr::<u32>(this, 0x24, 6);
    wr::<u8>(this, 0x28, 0);
    wr::<u8>(this, 0x29, 0);
    wr::<u32>(this, 0x38, 0);
    wr::<u8>(this, 0x3C, 0);
    wr::<u32>(this, 0x40, 0);
    wr::<u8>(this, 0x44, 0);
    wr::<f32>(this, 0x48, -1.0);
    if !veh.is_null() {
        entity_register_reference(veh, this.byte_add(0xC));
    }
    this
}

/// `CTaskComplexEnterCar::~CTaskComplexEnterCar` (0x63DFA0; entry through the island 0x401148): drops the vehicle
/// reference, destroys and frees the line-up utility (+0x38), takes the task's passenger count (+0x28) off the
/// vehicle's byte +0x485 and clears its getting-in flags (+0x44), then the `CTaskComplex` destructor.
#[hook(addr = 0x63DFA0, conv = thiscall, name = "CTaskComplexEnterCar::Destructor", expect = "6A FF E9 A1 31 DC FF",
       unverified = "ai7-11", route = "L2", hazards = "heap-writes", recs = "car-ai")]
pub fn enter_car_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86E6F0);
    let slot = this.byte_add(0xC);
    let v = rd::<GPtr<u8>>(slot, 0);
    if !v.is_null() {
        entity_clean_up_old_reference(v, slot);
    }
    let lu = rd::<GPtr<u8>>(this, 0x38);
    if !lu.is_null() {
        line_up_dtor(lu);
        crt_free(lu);
    }
    let v = rd::<GPtr<u8>>(slot, 0);
    if !v.is_null() {
        wr::<u8>(v, 0x485, rd::<u8>(v, 0x485).wrapping_sub(rd::<u8>(this, 0x28)));
        vehicle_clear_getting_in_flags(rd::<GPtr<u8>>(slot, 0), u32::from(rd::<u8>(this, 0x44)));
    }
    task_complex_dtor(this);
}

/// `CTaskComplexEnterCarAsDriverTimed::CTaskComplexEnterCarAsDriverTimed(CVehicle*, int32 time)` (0x63AD70,
/// `ret 8`; entry through the island 0x40A4B9): `CTaskComplex` base, vehicle +0xC (registered when set), time
/// +0x10, `vtbl = 0x86E720`, +0x14 = 6, +0x18 / +0x1C = 0, bytes +0x20 / +0x21 = 0.
#[hook(addr = 0x63AD70, conv = thiscall, name = "CTaskComplexEnterCarAsDriverTimed::Constructor", expect = "6A FF E9 42 F7 DC FF",
       unverified = "ai7-11", route = "L2", hazards = "heap-writes", recs = "car-ai")]
pub fn enter_car_as_driver_timed_ctor(this: GPtr<u8>, veh: GPtr<u8>, time: u32) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<GPtr<u8>>(this, 0xC, veh);
    wr::<u32>(this, 0x10, time);
    wr::<u32>(this, 0, 0x86E720);
    wr::<u32>(this, 0x14, 6);
    wr::<u32>(this, 0x18, 0);
    wr::<u32>(this, 0x1C, 0);
    wr::<u8>(this, 0x20, 0);
    wr::<u8>(this, 0x21, 0);
    register_if_set(this.byte_add(0xC));
    this
}

/// `CTaskComplexLeaveBoat::CreateSubTask(int32)` (0x63BC50, `ret 4`): 0x330 → `CTaskSimpleCarSetPedOut(vehicle
/// +0xC, door 0, true)`; anything else or a full pool → null.
#[hook(addr = 0x63BC50, conv = thiscall, name = "CTaskComplexLeaveBoat::CreateSubTask", expect = "64 A1 00 00 00 00",
       unverified = "ai7-11", route = "L2", hazards = "heap-writes", recs = "car-ai")]
pub fn leave_boat_create_sub_task(this: GPtr<u8>, ty: i32) -> GPtr<u8> {
    if ty != 0x330 {
        return GPtr::NULL;
    }
    let t = task_new(0x18);
    if t.is_null() {
        return t;
    }
    car_set_ped_out_ctor(t, rd::<GPtr<u8>>(this, 0xC), 0, 1)
}

/// `CTaskSimpleCarOpenDoorFromOutside::CTaskSimpleCarOpenDoorFromOutside(CVehicle*, int32 door,
/// CTaskUtilityLineUpPedWithCar*, bool)` (0x645E50, `ret 0x10`): `CTaskSimple` base, door +0x14, `vtbl =
/// 0x86EC9C`, byte +8 = 0, +0xC = 0, vehicle +0x10 (registered when set), the flag byte +0x18, byte +0x19 = 0, the
/// line-up +0x1C, +0x20 = 0.
#[hook(addr = 0x645E50, conv = thiscall, name = "CTaskSimpleCarOpenDoorFromOutside::Constructor", expect = "6A FF 68 F8 23 84 00",
       route = "L2", hazards = "heap-writes", recs = "car-ai")]
pub fn car_open_door_from_outside_ctor(this: GPtr<u8>, veh: GPtr<u8>, door: u32, lineup: u32, b: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0x14, door);
    wr::<u32>(this, 0, 0x86EC9C);
    wr::<u8>(this, 8, 0);
    wr::<u32>(this, 0xC, 0);
    wr::<GPtr<u8>>(this, 0x10, veh);
    wr::<u8>(this, 0x18, b as u8);
    wr::<u8>(this, 0x19, 0);
    wr::<u32>(this, 0x1C, lineup);
    wr::<u32>(this, 0x20, 0);
    if !veh.is_null() {
        entity_register_reference(veh, this.byte_add(0x10));
    }
    this
}

/// `CTaskSimpleCarOpenDoorFromOutside::~CTaskSimpleCarOpenDoorFromOutside` (0x645EE0): drops the vehicle reference,
/// gives a playing association (+0xC) the default finish callback, and when +0x19 is set clears it and sets the
/// player ped's flag 0x4000000 (+0x474); then the `CTaskSimple` destructor.
#[hook(addr = 0x645EE0, conv = thiscall, name = "CTaskSimpleCarOpenDoorFromOutside::Destructor", expect = "6A FF 68 18 24 84 00",
       unverified = "ai7-11", route = "L2", hazards = "heap-writes", recs = "car-ai")]
pub fn car_open_door_from_outside_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86EC9C);
    let slot = this.byte_add(0x10);
    let v = rd::<GPtr<u8>>(slot, 0);
    if !v.is_null() {
        entity_clean_up_old_reference(v, slot);
    }
    let a = rd::<GPtr<u8>>(this, 0xC);
    if !a.is_null() {
        assoc_set_finish_callback(a, DEFAULT_ANIM_CB, 0);
    }
    if rd::<u8>(this, 0x19) != 0 {
        wr::<u8>(this, 0x19, 0);
        let p = find_player_ped(-1);
        wr::<u32>(p, 0x474, rd::<u32>(p, 0x474) | 0x400_0000);
    }
    task_simple_dtor(this);
}

/// `CTaskSimpleCarCloseDoorFromOutside::CTaskSimpleCarCloseDoorFromOutside(CVehicle*, int32 door,
/// CTaskUtilityLineUpPedWithCar*)` (0x6464F0, `ret 0xC`; entry through the island 0x406F6C): `CTaskSimple` base,
/// door +0x14, `vtbl = 0x86ED2C`, byte +8 = 0, +0xC = 0, vehicle +0x10 (registered when set), line-up +0x18.
#[hook(addr = 0x6464F0, conv = thiscall, name = "CTaskSimpleCarCloseDoorFromOutside::Constructor", expect = "6A FF E9 75 0A DC FF",
       route = "L2", hazards = "heap-writes", recs = "car-ai")]
pub fn car_close_door_from_outside_ctor(this: GPtr<u8>, veh: GPtr<u8>, door: u32, lineup: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0x14, door);
    wr::<u32>(this, 0, 0x86ED2C);
    wr::<u8>(this, 8, 0);
    wr::<u32>(this, 0xC, 0);
    wr::<GPtr<u8>>(this, 0x10, veh);
    wr::<u32>(this, 0x18, lineup);
    if !veh.is_null() {
        entity_register_reference(veh, this.byte_add(0x10));
    }
    this
}

/// `FUN_0064e4b0(int32 animGroup, int32 animId)` (0x64E4B0, cdecl; continues through the island 0x40107D, which
/// only scales the group index by 0x94): `GetGroup(animId)` on the vehicle anim group (result unused), then ST0 =
/// its `ComputeCriticalBlendTime(animId)`.
#[hook(addr = 0x64E4B0, conv = cdecl, name = "FUN_0064e4b0", expect = "56 8B 74 24 08",
       route = "L2", hazards = "none", recs = "car-ai")]
pub fn veh_anim_critical_blend_time(group: u32, anim: u32) -> X87 {
    let g: GPtr<u8> = GPtr::from_addr(group.wrapping_mul(0x94).wrapping_add(VEH_ANIM_GROUPS) as usize);
    veh_anim_group_get_group(g, anim);
    X87::from_f64_exact(veh_anim_group_compute_critical_blend_time(g, anim))
}

/// `CTaskComplexGotoDoorAndOpen::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x66BC80, `ret 0xC`): only
/// urgent aborts (priority 2) pass on to the sub-task (tail jump; the original rewrites its priority slot with the
/// same 2); otherwise AL = 0.
#[hook(addr = 0x66BC80, conv = thiscall, name = "CTaskComplexGotoDoorAndOpen::MakeAbortable", expect = "83 7C 24 08 02",
       unverified = "ai7-11", route = "L2", hazards = "ax-al-return", recs = "car-ai")]
pub fn goto_door_and_open_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: u32) -> u8 {
    if priority != 2 {
        return 0;
    }
    v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 2, event)
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_ENTER_CAR_CTOR,
    HOOK_ENTER_CAR_DTOR,
    HOOK_ENTER_CAR_AS_DRIVER_TIMED_CTOR,
    HOOK_LEAVE_BOAT_CREATE_SUB_TASK,
    HOOK_CAR_OPEN_DOOR_FROM_OUTSIDE_CTOR,
    HOOK_CAR_OPEN_DOOR_FROM_OUTSIDE_DTOR,
    HOOK_CAR_CLOSE_DOOR_FROM_OUTSIDE_CTOR,
    HOOK_VEH_ANIM_CRITICAL_BLEND_TIME,
    HOOK_GOTO_DOOR_AND_OPEN_MAKE_ABORTABLE,
];
