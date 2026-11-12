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
    /// `CPed::SetCharCreatedBy(uint8)` (0x5E47E0, `ret 4`)
    fn ped_set_char_created_by(p: GPtr<u8>, by: u32) = thiscall 0x5E47E0 => unported;
    /// `CCarCtrl::JoinCarWithRoadSystem(CVehicle*)` (0x42F5A0)
    fn car_ctrl_join_car_with_road_system(v: GPtr<u8>) = cdecl 0x42F5A0 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    fn ped_groups_get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupMembership::IsFollower(const CPed*)` (0x5F69E0, AL)
    fn membership_is_follower(m: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x5F69E0 => unported;
    /// `CPedGroupMembership::RemoveMember(CPed*)` (0x5FB210)
    fn membership_remove_member(m: GPtr<u8>, p: GPtr<u8>) = thiscall 0x5FB210 => unported;
    /// `CEventGroup::GetEventOfType(int32)` (0x4AB650)
    fn event_group_get_event_of_type(g: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x4AB650 => unported;
    /// Virtual `CTask::GetTaskType()` (vtable +0x10)
    fn vcall_task_type(f: u32, t: GPtr<u8>) -> u32 = thiscall dyn => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence()` (0x632BD0)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    /// `CTaskComplexLeaveAnyCar::CTaskComplexLeaveAnyCar(int32 delay, bool sensibleLeaveCar, bool forceGetOut)`
    /// (0x421150)
    fn leave_any_car_ctor(t: GPtr<u8>, delay: u32, sensible: u32, force: u32) -> GPtr<u8> = thiscall 0x421150 => unported;
    /// `CTaskComplexWander::GetWanderTaskByPedType(CPed*)` (0x673D00)
    fn get_wander_task_by_ped_type(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x673D00 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10)
    fn sequence_add_task(s: GPtr<u8>, t: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32, CTask*, bool)` (0x4B0A00, `ret 0xC`)
    fn event_script_command_ctor(e: GPtr<u8>, idx: u32, t: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CEventScriptCommand::~CEventScriptCommand()` (0x4B0A50)
    fn event_script_command_dtor(e: GPtr<u8>) = thiscall 0x4B0A50 => unported;
}

/// `CPopulation::ms_nTotalMissionPeds` (0xC0EC24, int32)
const TOTAL_MISSION_PEDS: usize = 0xC0EC24;

fn rd32(p: GPtr<u8>, off: isize) -> u32 {
    p.byte_add(off).cast::<u32>().read()
}
fn rdp(p: GPtr<u8>, off: isize) -> GPtr<u8> {
    p.byte_add(off).cast::<GPtr<u8>>().read()
}
/// The type of the task at `t` (vtable +0x10), or `None` for a null task.
fn task_type(t: GPtr<u8>) -> Option<u32> {
    if t.is_null() {
        return None;
    }
    Some(vcall_task_type(rdp(t, 0).byte_add(0x10).cast::<u32>().read(), t))
}
/// True when the ped's intelligence (+0x47C) already has a script-command event (type 0x20) or a primary task (+0x10)
/// of type `ty`: the event's task (+0x10) is checked first.
fn has_task_of_type(ped: GPtr<u8>, ty: u32) -> bool {
    let ev = event_group_get_event_of_type(rdp(ped, 0x47C).byte_add(0x68), 0x20);
    if !ev.is_null() && task_type(rdp(ev, 0x10)) == Some(ty) {
        return true;
    }
    task_type(rdp(rdp(ped, 0x47C), 0x10)) == Some(ty)
}
/// `CEventScriptCommand ev(3, task, false)` on the stack, added to the ped's event group (intelligence +0x68), then
/// destroyed.
fn give_script_command(ped: GPtr<u8>, task: GPtr<u8>) {
    let mut e = [0u32; 0x10];
    let ep: GPtr<u8> = GPtr::from_raw(e.as_mut_ptr()).cast();
    event_script_command_ctor(ep, 3, task, 0);
    event_group_add(rdp(ped, 0x47C).byte_add(0x68), ep, 0);
    event_script_command_dtor(ep);
}

/// `CTheScripts::CleanUpThisPed(CPed*)` (0x486300, cdecl).
///
/// Nothing for a null ped or one whose created-by byte (+0x484) is not 2. `SetCharCreatedBy(1)`; with byte +0x478
/// bit 4 set only `CPopulation::ms_nTotalMissionPeds--` follows. As the driver of its vehicle (+0x46C bit 8, +0x58C,
/// vehicle +0x460 == ped) the ped is "kept" (no new task) and the vehicle is released: vehicle type +0x594 3 (heli)
/// → mission byte +0x3BA 0x14 unless it is 0x39/0x3A, destination +0x3EC = (10000, -10000, 1000), +0x9B0 = +0x9A8 =
/// 1000.0; type 4 (plane) → 0x16 and (10000, 10000, 1000); otherwise, for +0x590 0 or 9,
/// `CCarCtrl::JoinCarWithRoadSystem(v)` and (vehicle re-read) +0x3BA = 1 unless 0x39/0x3A. Then +0x46C loses bit 21,
/// the ped leaves its group when a follower. A ped that is not a driver gets a script-command event (primary slot 3):
/// in a vehicle, unless it already has a task of type 0xF4 (event first, then the primary task), a new
/// `CTaskComplexSequence` (null on failure) of `CTaskComplexLeaveAnyCar(0, true, false)` (null on failure) and its
/// wander task; on foot, unless it already has a type 0x390 task, its wander task. Finally
/// `ms_nTotalMissionPeds--` (also when an existing task skipped the event). The SEH frame guards only the event's
/// destructor (no unwind in the port).
#[hook(addr = 0x486300, conv = cdecl, name = "CTheScripts::CleanUpThisPed", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "script5-5", route = "L2", hazards = "none", recs = "grove-walk",
       note = "SEH frame not reproduced (unwind only destroys the stack event)")]
pub fn clean_up_this_ped(ped: GPtr<u8>) {
    if ped.is_null() || ped.byte_add(0x484).read() != 2 {
        return;
    }
    ped_set_char_created_by(ped, 1);
    let dec = || {
        let n = GPtr::<i32>::from_addr(TOTAL_MISSION_PEDS);
        n.write(n.read().wrapping_sub(1));
    };
    if ped.byte_add(0x478).read() & 0x10 != 0 {
        dec();
        return;
    }
    let mut give_task = true;
    let v = rdp(ped, 0x58C);
    if rd32(ped, 0x46C) & 0x100 != 0 && !v.is_null() && rd32(v, 0x460) == ped.addr() as u32 {
        give_task = false;
        let ty = rd32(v, 0x594);
        let release = |v: GPtr<u8>, byte: u8, y: u32| {
            let m = v.byte_add(0x3BA);
            let c = m.read();
            if c != 0x39 && c != 0x3A {
                m.write(byte);
            }
            let d = v.byte_add(0x3EC).cast::<u32>();
            d.write(0x461C_4000);
            d.byte_add(4).write(y);
            d.byte_add(8).write(0x447A_0000);
            v.byte_add(0x9B0).cast::<u32>().write(0x447A_0000);
            v.byte_add(0x9A8).cast::<u32>().write(0x447A_0000);
        };
        if ty == 3 {
            release(v, 0x14, 0xC61C_4000);
        } else if ty == 4 {
            release(v, 0x16, 0x461C_4000);
        } else {
            let u = rd32(v, 0x590);
            if u == 0 || u == 9 {
                car_ctrl_join_car_with_road_system(v);
                let m = rdp(ped, 0x58C).byte_add(0x3BA);
                let c = m.read();
                if c != 0x39 && c != 0x3A {
                    m.write(1);
                }
            }
        }
    }
    let f = ped.byte_add(0x46C).cast::<u32>();
    f.write(f.read() & 0xFFDF_FFFF);
    let grp = ped_groups_get_peds_group(ped);
    if !grp.is_null() {
        let m = grp.byte_add(8);
        if membership_is_follower(m, ped) != 0 {
            membership_remove_member(m, ped);
        }
    }
    if give_task {
        if rd32(ped, 0x46C) & 0x100 != 0 && !rdp(ped, 0x58C).is_null() {
            if !has_task_of_type(ped, 0xF4) {
                let s = task_new(0x40);
                let seq = if s.is_null() { s } else { sequence_ctor(s) };
                let l = task_new(0x14);
                let leave = if l.is_null() { l } else { leave_any_car_ctor(l, 0, 1, 0) };
                let wander = get_wander_task_by_ped_type(ped);
                sequence_add_task(seq, leave);
                sequence_add_task(seq, wander);
                give_script_command(ped, seq);
            }
        } else if !has_task_of_type(ped, 0x390) {
            let wander = get_wander_task_by_ped_type(ped);
            give_script_command(ped, wander);
        }
    }
    dec();
}

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
    &[HOOK_CLEAN_UP_THIS_VEHICLE, HOOK_ADD_TO_WAITING_FOR_SCRIPT_BRAIN_ARRAY, HOOK_REMOVE_THIS_PED, HOOK_CLEAN_UP_THIS_PED];
