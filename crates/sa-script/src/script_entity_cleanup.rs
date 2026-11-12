//! Script entity bookkeeping with return-address-obfuscated calls (batch `mvpm-script-1`):
//! `CTheScripts::CleanUpThisVehicle` and `CTheScripts::AddToWaitingForScriptBrainArray`. Both bodies live in
//! `.HOODLUM` behind 5-byte entry stubs and reach one callee through `call $+5; push r; mov r, <decode>; xchg [esp],
//! r; add [esp+4], N; ret` (the decoded target is called with the return address `$+5+N`). The targets were decoded
//! from the image (fixed values). Ported from the 1.0 US bytes (gta-reversed `TheScripts.cpp` as a reading aid; the
//! bytes win). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `CCarCtrl::RemoveFromInterestingVehicleList(CVehicle*)` (0x423ED0)
    fn car_ctrl_remove_from_interesting_vehicle_list(v: GPtr<u8>) = cdecl 0x423ED0 => unported;
    /// `CVehicleRecording::StopPlaybackRecordedCar(CVehicle*)` (0x45A280)
    fn vehicle_recording_stop_playback_recorded_car(v: GPtr<u8>) = cdecl 0x45A280 => unported;
    /// `CVehicle::SetVehicleCreatedBy(int32)` (0x6D5D70, `ret 4`)
    fn vehicle_set_vehicle_created_by(v: GPtr<u8>, by: i32) = thiscall 0x6D5D70 => unported;
    /// `CEntity::RegisterReference(CEntity** slot)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CVehicle::RemoveDriver(bool)` (0x6D1950, `ret 4`)
    fn vehicle_remove_driver(v: GPtr<u8>, b: u32) = thiscall 0x6D1950 => unported;
    /// `CVehicle::RemovePassenger(CPed*)` (0x6D1610, `ret 4`)
    fn vehicle_remove_passenger(v: GPtr<u8>, p: GPtr<u8>) = thiscall 0x6D1610 => unported;
    /// `CVehicle::IsLawEnforcementVehicle()` (0x6D2370)
    fn vehicle_is_law_enforcement_vehicle(v: GPtr<u8>) -> u8 = thiscall 0x6D2370 => unported;
    /// `CVehicle::ChangeLawEnforcerState(bool)` (0x6D2330, `ret 4`)
    fn vehicle_change_law_enforcer_state(v: GPtr<u8>, b: u32) = thiscall 0x6D2330 => unported;
    /// `CWorld::RemoveReferencesToDeletedObject(CEntity*)` (0x565510)
    fn world_remove_references_to_deleted_object(e: GPtr<u8>) = cdecl 0x565510 => unported;
    /// Virtual slot 0 (the scalar deleting destructor, `ret 4`)
    fn vcall_delete(f: u32, this: GPtr<u8>, flags: u32) = thiscall dyn => unported;
}

/// `CPopulation::ms_nTotalMissionPeds` (0xC0EC24, int32)
const TOTAL_MISSION_PEDS: usize = 0xC0EC24;

/// `CTheScripts::RemoveThisPed(CPed*)` (0x486240, cdecl).
///
/// Nothing for a null ped. `mission = created-by byte (+0x484) == 2`. In a vehicle (+0x46C bit 8, +0x58C non-null):
/// as its driver (+0x460) `RemoveDriver(false)`, then (the vehicle re-read from the ped each time) its +0x36 byte
/// becomes `(b & 7) | 0x20` (status 4: abandoned), +0x4F8 5 → 1, and for a cop (ped type +0x598 == 6) in a law
/// enforcement vehicle `ChangeLawEnforcerState(false)`; as a passenger `RemovePassenger(ped)`. Then
/// `CWorld::RemoveReferencesToDeletedObject(ped)`, the deleting destructor (vtable slot 0, flag 1) and, for a mission
/// ped, `CPopulation::ms_nTotalMissionPeds--`.
#[hook(addr = 0x486240, conv = cdecl, name = "CTheScripts::RemoveThisPed", expect = "56 8B 74 24 08",
       unverified = "script5-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn remove_this_ped(ped: GPtr<u8>) {
    if ped.is_null() {
        return;
    }
    let mission = ped.byte_add(0x484).read() == 2;
    let veh_of = || ped.byte_add(0x58C).cast::<GPtr<u8>>().read();
    let v = veh_of();
    if ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 && !v.is_null() {
        if v.byte_add(0x460).cast::<u32>().read() == ped.addr() as u32 {
            vehicle_remove_driver(v, 0);
            let b = veh_of().byte_add(0x36);
            b.write((b.read() & 7) | 0x20);
            let st = veh_of().byte_add(0x4F8).cast::<u32>();
            if st.read() == 5 {
                st.write(1);
            }
            if ped.byte_add(0x598).cast::<u32>().read() == 6 && vehicle_is_law_enforcement_vehicle(veh_of()) != 0 {
                vehicle_change_law_enforcer_state(veh_of(), 0);
            }
        } else {
            vehicle_remove_passenger(v, ped);
        }
    }
    world_remove_references_to_deleted_object(ped);
    let f = ped.cast::<GPtr<u32>>().read().read();
    vcall_delete(f, ped, 1);
    if mission {
        let n = GPtr::<i32>::from_addr(TOTAL_MISSION_PEDS);
        n.write(n.read().wrapping_sub(1));
    }
}

/// `CTheScripts::EntitiesWaitingForScriptBrain` (0xA476B0, 150 x 8: +0 entity, +4 i16 brain index)
const WAITING_FOR_BRAIN: u32 = 0xA476B0;

/// `CTheScripts::CleanUpThisVehicle(CVehicle*)` (stub 0x486670 → 0x1565CA0, cdecl).
///
/// Nothing for a null vehicle or one whose created-by byte (+0x4A4) is not 2 (mission). Else the byte at +0x428 loses
/// bit 3 (locked) and the physical flags dword +0x40 loses bits 2 and 13 (read both, then write the byte first),
/// `CCarCtrl::RemoveFromInterestingVehicleList(v)`, `CVehicleRecording::StopPlaybackRecordedCar(v)` and
/// `SetVehicleCreatedBy(1)` (random).
///
/// The first call is return-address obfuscated: the target `rol([0x156050C], [0x1569FFF] & 31)` = 0x423ED0 returns
/// to 0x1565CEF, which pushes the vehicle again; both arguments are popped together after `StopPlaybackRecordedCar`.
#[hook(addr = 0x486670, conv = cdecl, name = "CTheScripts::CleanUpThisVehicle", expect = "E9 2B F6 0D 01",
       unverified = "mvpm-script-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "RemoveFromInterestingVehicleList reached through a decoded return-address-obfuscated call (0x423ED0)")]
pub fn clean_up_this_vehicle(v: GPtr<u8>) {
    if v.is_null() || v.byte_add(0x4A4).cast::<u8>().read() != 2 {
        return;
    }
    let b = v.byte_add(0x428).cast::<u8>();
    let f = v.byte_add(0x40).cast::<u32>();
    let nb = b.read() & 0xF7;
    let nf = f.read() & 0xFFFF_DFFB;
    b.write(nb);
    f.write(nf);
    car_ctrl_remove_from_interesting_vehicle_list(v);
    vehicle_recording_stop_playback_recorded_car(v);
    vehicle_set_vehicle_created_by(v, 1);
}

/// `CTheScripts::AddToWaitingForScriptBrainArray(CEntity*, int16 brainIndex)` (stub 0x46AB60 → 0x1565010, cdecl).
///
/// One pass over the 150 slots (16-bit index): an empty slot (entity 0) becomes the candidate (the *last* empty slot
/// wins, unlike gta-reversed's first); a slot holding the entity returns at once. Without a candidate (index -1)
/// nothing happens. Else the slot's entity is set, `entity->RegisterReference(&slot.entity)` and the slot's brain
/// index word is written.
///
/// The `RegisterReference` call is return-address obfuscated: target `[0x15634D4] ^ [0x15604F8]` = 0x571B70 (ECX is
/// still the entity), returning to 0x156506A.
#[hook(addr = 0x46ab60, conv = cdecl, name = "CTheScripts::AddToWaitingForScriptBrainArray", expect = "E9 AB A4 0F 01",
       unverified = "mvpm-script-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "RegisterReference reached through a decoded return-address-obfuscated call (0x571B70); the last free slot is taken")]
pub fn add_to_waiting_for_script_brain_array(entity: GPtr<u8>, brain: i16) {
    let mut free: i16 = -1;
    let mut i: i16 = 0;
    while i < 0x96 {
        let e = GPtr::<u32>::from_addr((WAITING_FOR_BRAIN + (i as u32) * 8) as usize).read();
        if e == 0 {
            free = i;
        } else if e == entity.addr() as u32 {
            return;
        }
        i += 1;
    }
    if free < 0 {
        return;
    }
    let slot = WAITING_FOR_BRAIN + (free as u32) * 8;
    GPtr::<u32>::from_addr(slot as usize).write(entity.addr() as u32);
    entity_register_reference(entity, GPtr::from_addr(slot as usize));
    GPtr::<i16>::from_addr((slot + 4) as usize).write(brain);
}

pub const HOOKS: &[HookReg] =
    &[HOOK_CLEAN_UP_THIS_VEHICLE, HOOK_ADD_TO_WAITING_FOR_SCRIPT_BRAIN_ARRAY, HOOK_REMOVE_THIS_PED];
