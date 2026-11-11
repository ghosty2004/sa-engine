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

pub const HOOKS: &[HookReg] = &[HOOK_CLEAN_UP_THIS_VEHICLE, HOOK_ADD_TO_WAITING_FOR_SCRIPT_BRAIN_ARRAY];
