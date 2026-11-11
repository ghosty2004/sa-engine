//! WRITER-MVP-B (helper A, batch `mvpb-veh-1`): vehicle functions on the MVP call path. Ported from
//! the 1.0 US bytes; gta-reversed and docs/re/specs/VEH as reading aids. Where they differ, the bytes
//! win.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;

use crate::common::{f32_of, g, model_info, rand_at, rd, wr, x};

orig! {
    /// `CTaskSimpleCarSetPedInAsDriver::CTaskSimpleCarSetPedInAsDriver(CVehicle*, CTaskUtilityLineUpPedWithCar*)` (0x6470E0)
    fn set_in_as_driver_ctor(t: GPtr<u8>, veh: GPtr<u8>, line_up: GPtr<u8>) -> GPtr<u8> = thiscall 0x6470E0 => unported;
    /// `CTaskSimpleCarSetPedInAsDriver::ProcessPed(CPed*)` (0x64B950)
    fn set_in_as_driver_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x64B950 => unported;
    /// `CTaskSimpleCarSetPedInAsDriver::~CTaskSimpleCarSetPedInAsDriver()` (0x647170)
    fn set_in_as_driver_dtor(t: GPtr<u8>) = thiscall 0x647170 => unported;
    /// `CTaskSimpleCarSetPedInAsPassenger::CTaskSimpleCarSetPedInAsPassenger(CVehicle*, int32, CTaskUtilityLineUpPedWithCar*)` (0x646FE0)
    fn set_in_as_passenger_ctor(t: GPtr<u8>, veh: GPtr<u8>, door: i32, line_up: GPtr<u8>) -> GPtr<u8> = thiscall 0x646FE0 => unported;
    /// `CTaskSimpleCarSetPedInAsPassenger::ProcessPed(CPed*)` (0x64B5D0)
    fn set_in_as_passenger_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x64B5D0 => unported;
    /// `CTaskSimpleCarSetPedInAsPassenger::~CTaskSimpleCarSetPedInAsPassenger()` (0x647080)
    fn set_in_as_passenger_dtor(t: GPtr<u8>) = thiscall 0x647080 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexCarDriveWander::CTaskComplexCarDriveWander(CVehicle*, int32, float)` (0x63CB10)
    fn car_drive_wander_ctor(t: GPtr<u8>, veh: GPtr<u8>, style: i32, speed: f32) -> GPtr<u8> = thiscall 0x63CB10 => unported;
    /// `CTaskComplexCarDrive::CTaskComplexCarDrive(CVehicle*)` (0x63C9D0)
    fn car_drive_ctor(t: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> = thiscall 0x63C9D0 => unported;
    /// `CTaskManager::SetTask(CTask*, int32, bool)` (0x681AF0)
    fn tm_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: i32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CCarEnterExit::ComputePassengerIndexFromCarDoor(const CVehicle*, int32)` (0x64F1E0)
    fn compute_passenger_index_from_car_door(veh: GPtr<u8>, door: i32) -> i32 = cdecl 0x64F1E0 => crate::enterexit::compute_passenger_index_from_car_door;
    /// `CPedType::GetPedFlag(ePedType)` (0x608830)
    fn ped_type_get_ped_flag(t: u32) -> u32 = cdecl 0x608830 => unported;
    /// `CAcquaintance::SetAsAcquaintance(int32, uint32)` (0x608DA0)
    fn acquaintance_set(a: GPtr<u8>, kind: i32, bits: u32) = thiscall 0x608DA0 => unported;
    /// `CPhysical::ApplyTurnForce(CVector force, CVector point)` (0x542A50)
    fn apply_turn_force(p: GPtr<u8>, force: CVector, point: CVector) = thiscall 0x542A50 => sa_world::physical::forces::apply_turn_force;
    /// `CVehicle::RemovePassenger(CPed*)` (0x6D1610, called directly)
    fn remove_passenger(v: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x6D1610 => crate::vehicle2::remove_passenger;
    /// `CPopulation::AddPedInCar(CVehicle*, bool driver, int32 gang, int32 seat, bool male, bool criminal)` (0x613A00)
    fn add_ped_in_car(veh: GPtr<u8>, driver: u8, gang: i32, seat: i32, male: u8, criminal: u8) -> GPtr<u8> = cdecl 0x613A00 => sa_pop::population2::add_ped_in_car;
    /// `CPopulation::RemovePed(CPed*)` (0x610F20)
    fn remove_ped(ped: GPtr<u8>) = cdecl 0x610F20 => sa_pop::population::remove_ped;
    /// `CLinkedUpgradeList::FindOtherUpgrade(int16)` (0x4C74D0) on `CVehicleModelInfo::ms_linkedUpgrades`
    fn find_other_upgrade(list: GPtr<u8>, upgrade: i16) -> u32 = thiscall 0x4C74D0 => sa_stream::modelinfo::vehicle::find_other_upgrade;
    /// `CVehicle::GetReplacementUpgrade(int32 node)` (0x6D3A50)
    fn get_replacement_upgrade(v: GPtr<u8>, node: u32) -> i32 = thiscall 0x6D3A50 => crate::vehicle2::get_replacement_upgrade;
    /// `CVehicle::AddReplacementUpgrade(int32 model, int32 node)` (0x6D3830)
    fn add_replacement_upgrade(v: GPtr<u8>, model: i32, node: u32) = thiscall 0x6D3830 => unported;
    /// `CVehicle::GetUpgrade(int32 node)` (0x6D3650)
    fn get_upgrade(v: GPtr<u8>, node: u32) -> i32 = thiscall 0x6D3650 => unported;
    /// `CVehicle::AddUpgrade(int32 model, int32 node)` (0x6DFA20)
    fn add_upgrade(v: GPtr<u8>, model: i32, node: u32) = thiscall 0x6DFA20 => unported;
    /// `CVehicle::SetVehicleUpgradeFlags(int32, int32, int32&)` (0x6D30E0)
    fn set_vehicle_upgrade_flags_call(v: GPtr<u8>, model: i32, kind: u32, out: GPtr<i32>) -> u8 = thiscall 0x6D30E0 => set_vehicle_upgrade_flags;
    /// `CAutomobile::NitrousControl(int8)` (0x6A3EA0)
    fn nitrous_control(a: GPtr<u8>, v: i32) = thiscall 0x6A3EA0 => unported;
    /// `CAudioEngine::SetRadioBassSetting(int8)` (0x506FA0)
    fn audio_set_radio_bass_setting(ae: GPtr<u8>, v: u32) = thiscall 0x506FA0 => unported;
    /// `CPed::Say(uint16, uint32, float, bool, bool, bool)` (0x5EFFE0), the float as its bits
    fn ped_say(p: GPtr<u8>, id: u32, delay: u32, prob: u32, a4: u32, a5: u32, a6: u32) = thiscall 0x5EFFE0 => unported;
    /// `CAutomobile::HasCarStoppedBecauseOfLight()` (0x44D520)
    fn has_car_stopped_because_of_light_call(a: GPtr<u8>) -> u8 = thiscall 0x44D520 => has_car_stopped_because_of_light;
    /// `CVehicle::PlayCarHorn()` (virtual, vtable +0xD8)
    fn play_car_horn_virtual(v: GPtr<u8>) = thiscall vslot 0xD8 => unported;
    /// `CVehicle::FlyingControl(eFlightModel, float, float, float, float)` (0x6D85F0)
    fn flying_control(v: GPtr<u8>, model: u32, a: f32, b: f32, c: f32, d: f32) = thiscall 0x6D85F0 => unported;
}

/// `ped->m_pIntelligence->m_TaskMgr` (`[ped + 0x47C] + 4`).
fn task_manager(ped: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(ped, 0x47C).byte_add(4)
}

/// `of` (unless created by a mission, +0x484 == 2) gets the respect acquaintance (0) with the ped
/// flag of `with`'s ped type (+0x598), on its acquaintance at +0x4E0.
fn set_respect(of: GPtr<u8>, with: GPtr<u8>) {
    if rd::<u8>(of, 0x484) != 2 {
        let bits = ped_type_get_ped_flag(rd::<u32>(with, 0x598));
        acquaintance_set(of.byte_add(0x4E0), 0, bits);
    }
}

/// `CCarEnterExit::SetPedInCarDirect(CPed*, CVehicle*, int32 door, bool asDriver)` (0x650280, entry
/// `nop; jmp` into the SEH-prologue protection stub 0x4067F2, body at 0x650286). As driver: a stack
/// `CTaskSimpleCarSetPedInAsDriver(veh, null)` with the warp flag (+0x18) set is run on the ped and
/// destroyed, then a new `CTaskComplexCarDriveWander(veh, (int8) veh+0x3B9, (float) (uint8) veh+0x3D0)`
/// gets byte +0x1C = 1 (written even when the allocation failed, as compiled) and becomes the primary
/// task (slot 3). As passenger: the passenger index is computed first; a stack
/// `CTaskSimpleCarSetPedInAsPassenger(veh, door, null)` with the warp flag (+0x1C) is run; for a bike
/// (+0x590 == 9) a new `CTaskComplexCarDrive(veh)` with +0x1C = 0 becomes the primary task; then the
/// mutual respect acquaintance with the driver (+0x460) and with the passengers +0x464[0..index]
/// (each direction skipped for mission peds); the stack task is destroyed last. Returns true.
#[hook(addr = 0x650280, conv = cdecl, name = "CCarEnterExit::SetPedInCarDirect", expect = "90 E9 6C 65 DB FF",
       unverified = "mvpb-veh-1", route = "L2", hazards = "heap-writes,entry-protection-read",
       note = "the original stores the cruise speed into its own bool argument slot (esp+0x10) before the fild; the port does not")]
pub fn set_ped_in_car_direct(ped: GPtr<u8>, veh: GPtr<u8>, door: i32, as_driver: u8) -> u8 {
    if as_driver != 0 {
        let mut task = [0u32; 8];
        let t = GPtr::from_raw(task.as_mut_ptr()).cast::<u8>();
        set_in_as_driver_ctor(t, veh, GPtr::NULL);
        wr::<u8>(t, 0x18, 1);
        set_in_as_driver_process_ped(t, ped);
        let mut wander = task_new(0x24);
        if !wander.is_null() {
            let speed = rd::<u8>(veh, 0x3D0) as f32;
            let style = rd::<i8>(veh, 0x3B9) as i32;
            wander = car_drive_wander_ctor(wander, veh, style, speed);
        }
        wr::<u8>(wander, 0x1C, 1);
        tm_set_task(task_manager(ped), wander, 3, 0);
        set_in_as_driver_dtor(t);
        return 1;
    }
    let index = compute_passenger_index_from_car_door(veh, door);
    let mut task = [0u32; 8];
    let t = GPtr::from_raw(task.as_mut_ptr()).cast::<u8>();
    set_in_as_passenger_ctor(t, veh, door, GPtr::NULL);
    wr::<u8>(t, 0x1C, 1);
    set_in_as_passenger_process_ped(t, ped);
    if rd::<u32>(veh, 0x590) == 9 {
        let mut drive = task_new(0x24);
        if !drive.is_null() {
            drive = car_drive_ctor(drive, veh);
        }
        wr::<u8>(drive, 0x1C, 0);
        tm_set_task(task_manager(ped), drive, 3, 0);
    }
    let driver = rd::<GPtr<u8>>(veh, 0x460);
    if !driver.is_null() {
        set_respect(ped, driver);
    }
    let driver = rd::<GPtr<u8>>(veh, 0x460);
    if !driver.is_null() {
        set_respect(driver, ped);
    }
    let mut i = 0;
    while i < index {
        let other = rd::<GPtr<u8>>(veh, 0x464 + 4 * i as isize);
        if !other.is_null() {
            set_respect(ped, other);
            set_respect(other, ped);
        }
        i += 1;
    }
    set_in_as_passenger_dtor(t);
    1
}

/// `FUN_0065da70` (0x65DA70, `ret 0x18`; a by-value forwarding copy of `CPhysical::ApplyTurnForce`,
/// docs/re/specs/PHYS/_unk-B.md): copies the two stack vectors into a local and re-pushes them to
/// `CPhysical::ApplyTurnForce(force, point)` with the same `this`. No other effect.
#[hook(addr = 0x65DA70, conv = thiscall, name = "FUN_0065da70", this = "CPhysical", expect = "83 EC 18 8B 44 24 28",
       unverified = "mvpb-veh-1", route = "L2", hazards = "none", note = "UNK-game (a CPhysical helper) kept in sa-veh; callers include CVehicle::AddPassenger")]
pub fn physical_apply_turn_force_copy(this: GPtr<u8>, force: CVector, point: CVector) {
    apply_turn_force(this, force, point);
}

/// `CVehicle::SetupPassenger(int32 seat, int32 gangPedType, bool male, bool criminal)` (0x6D1AA0,
/// `ret 0x10`): an occupied seat returns its passenger. Models 0x1A4/0x1B6/0x199 (taxi, cabbie,
/// stretch) with seat 0: `RemovePassenger(passengers[0])` (always null here) and null. Else
/// `CPopulation::AddPedInCar(this, false, gang, seat, male, criminal)`; the new ped is kept as is when
/// its ped type (+0x598) is 6, 7..=16, 18 or 19, or 20 with gang in 14..=23. Otherwise, when the driver
/// (+0x460) has the same model (+0x22, sign-extended) it is removed (`RemovePassenger` + `CPopulation::
/// RemovePed`) and null returned; else the passengers before `seat` are scanned and the first one
/// (non-null, not the ped) with the same model removes the new ped the same way (the scan goes on
/// with a null ped). Returns the ped (or null).
#[hook(addr = 0x6D1AA0, conv = thiscall, name = "CVehicle::SetupPassenger", this = "CVehicle", expect = "55 8B 6C 24 08 56 57",
       unverified = "mvpb-veh-1", route = "L2", hazards = "heap-writes",
       note = "the original stores the new ped's model index into its own seat argument slot (entry esp+4); the port does not")]
pub fn setup_passenger(this: GPtr<u8>, seat: i32, gang: i32, male: u8, criminal: u8) -> GPtr<u8> {
    let existing = rd::<GPtr<u8>>(this, 0x464 + 4 * seat as isize);
    if !existing.is_null() {
        return existing;
    }
    let mi = rd::<u16>(this, 0x22);
    if (mi == 0x1A4 || mi == 0x1B6 || mi == 0x199) && seat == 0 {
        remove_passenger(this, rd::<GPtr<u8>>(this, 0x464));
        return GPtr::NULL;
    }
    let mut ped = add_ped_in_car(this, 0, gang, seat, male, criminal);
    let model = rd::<i16>(ped, 0x22) as i32;
    let pt = rd::<i32>(ped, 0x598);
    if pt == 0x12 || pt == 0x13 || pt == 6 || (7..=0x10).contains(&pt) || (pt == 0x14 && (0xE..=0x17).contains(&gang)) {
        return ped;
    }
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if !driver.is_null() && rd::<i16>(driver, 0x22) as i32 == model {
        remove_passenger(this, ped);
        remove_ped(ped);
        return GPtr::NULL;
    }
    let mut i = 0;
    while i < seat {
        let p = rd::<GPtr<u8>>(this, 0x464 + 4 * i as isize);
        if !p.is_null() && !ped.is_null() && p != ped && rd::<i16>(p, 0x22) as i32 == model {
            remove_passenger(this, ped);
            remove_ped(ped);
            ped = GPtr::NULL;
        }
        i += 1;
    }
    ped
}

/// `CSpecialPlateHandler::Find(int32 carGenId, char* out)` (0x6F2D30, `ret 8`): `out[0] = 0`; with a
/// non-zero count (+0xF0) the 15 entries (0x10 bytes: id, then the text at +4) are searched for the id
/// and the first match's text is copied (byte loop, terminator included). EAX = the entry index, or
/// -1 when the count is 0 or nothing matches.
#[hook(addr = 0x6F2D30, conv = thiscall, name = "CSpecialPlateHandler::Find", this = "CSpecialPlateHandler", expect = "56 8B 74 24 0C C6 06 00",
       unverified = "mvpb-veh-1", route = "L2", hazards = "none")]
pub fn special_plate_find(this: GPtr<u8>, id: i32, out: GPtr<u8>) -> i32 {
    out.write(0);
    if rd::<u32>(this, 0xF0) == 0 {
        return -1;
    }
    for i in 0..0xF {
        let e = this.byte_add(i as isize * 0x10);
        if rd::<i32>(e, 0) == id {
            let mut k = 0;
            loop {
                let c = rd::<u8>(e, 4 + k);
                wr::<u8>(out, k, c);
                k += 1;
                if c == 0 {
                    break;
                }
            }
            return i;
        }
    }
    -1
}

/// The upgrade node of a model: `(modelInfo->m_nFlags (+0x12, u16) >> 10) & 0x1F`, and whether bit 8
/// (replacement upgrade) is set.
fn upgrade_node(mi: GPtr<u8>) -> (u32, bool) {
    let f = u32::from(rd::<u16>(mi, 0x12));
    ((f >> 10) & 0x1F, f & 0x100 != 0)
}

/// `CVehicle::AddVehicleUpgrade(int32 model)` (0x6E3290, `ret 4`; gta-reversed calls the original).
/// The entry's own argument slot is the "replaced upgrade" local (set to -1, then by
/// `SetVehicleUpgradeFlags`, or by `GetReplacementUpgrade`/`GetUpgrade`); its final value is returned
/// in EAX. When `SetVehicleUpgradeFlags(model, node, &replaced)` is false: the linked upgrade
/// `(int16) ms_linkedUpgrades.FindOtherUpgrade(model)` is looked up first; a replacement upgrade
/// (flags bit 8, flags re-read) saves `GetReplacementUpgrade(node)` and `AddReplacementUpgrade(model,
/// node)`; for node 2 also `+0x458 = own model info +0x40` and replacement nodes 5, 4, 7, and for an
/// automobile (+0x594 == 0) handling bit 0x100000 of +0x38C is set when `model == MI 0x8CD758`, or
/// cleared when the handling data (+0x384) +0xD0 lacks it. Otherwise `GetUpgrade(node)` +
/// `AddUpgrade(model, node)`. The linked upgrade (when not -1) is added the same way. Finally the 15
/// `m_anUpgrades` slots (+0x43A, i16): a slot equal to `replaced` gets the model (or -1 once placed),
/// the first empty slot (0xFFFF) gets the model once.
#[hook(addr = 0x6E3290, conv = thiscall, name = "CVehicle::AddVehicleUpgrade", this = "CVehicle", expect = "53 8B 5C 24 08 56",
       unverified = "mvpb-veh-1", route = "L2", hazards = "heap-writes",
       note = "the original keeps the replaced upgrade in its own argument slot (entry esp+4) and leaves it there; the port uses a local")]
pub fn add_vehicle_upgrade(this: GPtr<u8>, model: i32) -> i32 {
    let mut replaced = [-1i32; 1];
    let rp = GPtr::from_raw(replaced.as_mut_ptr());
    let mi = model_info(model);
    let (node, _) = upgrade_node(mi);
    let mut upgrade = model;
    if set_vehicle_upgrade_flags_call(this, model, node, rp) == 0 {
        let linked = find_other_upgrade(g::<u8>(0xB4E6D8), model as i16) as u16 as i16 as i32;
        let (node, repl) = upgrade_node(mi);
        if repl {
            rp.write(get_replacement_upgrade(this, node));
            add_replacement_upgrade(this, model, node);
            if node == 2 {
                let own = model_info(rd::<i16>(this, 0x22) as i32);
                wr::<u32>(this, 0x458, rd::<u32>(own, 0x40));
                add_replacement_upgrade(this, model, 5);
                add_replacement_upgrade(this, model, 4);
                add_replacement_upgrade(this, model, 7);
                if rd::<u32>(this, 0x594) == 0 {
                    if model == i32::from(g::<u16>(0x8CD758).read()) {
                        wr::<u32>(this, 0x38C, rd::<u32>(this, 0x38C) | 0x10_0000);
                    } else if rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xD0) & 0x10_0000 == 0 {
                        wr::<u32>(this, 0x38C, rd::<u32>(this, 0x38C) & 0xFFEF_FFFF);
                    }
                }
            }
        } else {
            rp.write(get_upgrade(this, node));
            add_upgrade(this, model, node);
        }
        if linked != -1 {
            let (n2, r2) = upgrade_node(model_info(linked));
            if r2 {
                add_replacement_upgrade(this, linked, n2);
            } else {
                add_upgrade(this, linked, n2);
            }
        }
    }
    for i in 0..15 {
        let off = 0x43A + 2 * i;
        let slot = rd::<u16>(this, off);
        let r = rp.read();
        if slot as i16 as i32 == r {
            wr::<u16>(this, off, 0xFFFF);
            wr::<u16>(this, off, upgrade as u16);
            upgrade = -1;
        } else if slot == 0xFFFF {
            wr::<u16>(this, off, upgrade as u16);
            upgrade = -1;
        }
    }
    rp.read()
}

/// `CVehicle::SetVehicleUpgradeFlags(int32 model, int32 node, int32& replaced)` (0x6D30E0, `ret 0xC`).
/// Node 16 (hydraulics): `replaced = model` when handling flag 0x20000 (+0x38C) was set; the flag is
/// set, `m_nFakePhysics` (+0xB8) = 0, move speed z (+0x4C) = 0; true. Node 17 (stereo): with a
/// non-civilian radio (+0x1D3) or the upgraded-stereo flag (+0x42E bit 4): `replaced = model`, true;
/// bass setting (+0x1BE) 1 -> true; 0 -> 1, 2 -> 0, then `AudioEngine.SetRadioBassSetting(bass)` and
/// the stereo flag set, true. Node 15 (nitro) on an automobile (+0x590 == 0): the amount (2, 5 for MI
/// 0x8CD764, 10 for MI 0x8CD768; the original builds it in the low byte of its node argument slot and
/// pushes that whole dword) goes to `NitrousControl`, `replaced = MI 0x8CD760` when nitro bit 0x80000
/// was set; returns whether the own vehicle struct's upgrade 15 parent (+0x2B0) is negative.
/// Otherwise false.
#[hook(addr = 0x6D30E0, conv = thiscall, name = "CVehicle::SetVehicleUpgradeFlags", this = "CVehicle", expect = "8B 44 24 08 83 F8 10 56",
       unverified = "mvpb-veh-1", route = "L2", hazards = "heap-writes",
       note = "node 15: the original writes the nitro amount byte into its own node argument slot (entry esp+8); the port does not")]
pub fn set_vehicle_upgrade_flags(this: GPtr<u8>, model: i32, node: u32, out: GPtr<i32>) -> u8 {
    match node {
        0x10 => {
            if rd::<u32>(this, 0x38C) & 0x2_0000 != 0 {
                out.write(model);
            }
            wr::<u32>(this, 0x38C, rd::<u32>(this, 0x38C) | 0x2_0000);
            wr::<u8>(this, 0xB8, 0);
            wr::<u32>(this, 0x4C, 0);
            1
        }
        0x11 => {
            if rd::<u8>(this, 0x1D3) != 0 || rd::<u8>(this, 0x42E) & 0x10 != 0 {
                out.write(model);
                return 1;
            }
            let bass = rd::<u8>(this, 0x1BE);
            if bass == 1 {
                return 1;
            }
            if bass == 0 {
                wr::<u8>(this, 0x1BE, 1);
            } else if bass == 2 {
                wr::<u8>(this, 0x1BE, 0);
            }
            audio_set_radio_bass_setting(g::<u8>(0xB6BC90), u32::from(rd::<u8>(this, 0x1BE)));
            wr::<u8>(this, 0x42E, rd::<u8>(this, 0x42E) | 0x10);
            1
        }
        0xF if rd::<u32>(this, 0x590) == 0 => {
            let amount = if model == i32::from(g::<u16>(0x8CD764).read()) {
                5
            } else if model == i32::from(g::<u16>(0x8CD768).read()) {
                10
            } else {
                2
            };
            if rd::<u32>(this, 0x38C) & 0x8_0000 != 0 {
                out.write(i32::from(g::<u16>(0x8CD760).read()));
            }
            nitrous_control(this, (node & 0xFFFF_FF00) as i32 | amount);
            let mi = model_info(rd::<i16>(this, 0x22) as i32);
            u8::from(rd::<i32>(rd::<GPtr<u8>>(mi, 0x5C), 0x2B0) < 0)
        }
        _ => 0,
    }
}

/// `CVehicle::CanBeDeleted()` (0x6D1180): false when the keep flags +0x485 or +0x487 are set, or the
/// driver (+0x460) is a mission ped (+0x484 == 2) or not in ped state 0x32/0x37 (+0x530); any of the
/// 8 passengers (+0x464) that is a mission ped or not in state 0x32/0x37 gives false; then by
/// `m_nCreatedBy` (+0x4A4, jump table 0x6D1220): 2 and 4 false, everything else true.
#[hook(addr = 0x6D1180, conv = thiscall, name = "CVehicle::CanBeDeleted", this = "CVehicle", expect = "8A 81 85 04 00 00",
       unverified = "mvpb-veh-1", route = "L2", hazards = "none")]
pub fn can_be_deleted(this: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x485) != 0 || rd::<u8>(this, 0x487) != 0 {
        return 0;
    }
    let ok = |p: GPtr<u8>| {
        if rd::<u8>(p, 0x484) == 2 {
            return false;
        }
        let st = rd::<u32>(p, 0x530);
        st == 0x32 || st == 0x37
    };
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if !driver.is_null() && !ok(driver) {
        return 0;
    }
    for i in 0..8 {
        let p = rd::<GPtr<u8>>(this, 0x464 + 4 * i);
        if !p.is_null() && !ok(p) {
            return 0;
        }
    }
    match rd::<u8>(this, 0x4A4) {
        2 | 4 => 0,
        _ => 1,
    }
}

/// `CTrain::FindMaximumSpeedToStopAtStations(float* out)` (0x6F5BA0, `ret 4`; docs/re/specs/VEH/
/// CTrain.md): `*out = 50`, min = 10000; false unless track 0 (+0x5C0). With `hi = total * 0.5`,
/// `lo = total * -0.5` (x87 stack) and the direction bit (+0x5B8 bit 6), each of the 6 stations'
/// `StationDist - railDistance (+0x5A8) ± 40` is wrapped into (lo, hi] by the track length (re-read),
/// taken when ahead (> 0 forward; < 0 backward, negated) and below the minimum. Then `*out` = `(1 -
/// (500 - min) * 0.002) * 50` when min < 500, else 100000; returns min < 5.
#[hook(addr = 0x6F5BA0, conv = thiscall, name = "CTrain::FindMaximumSpeedToStopAtStations", this = "CTrain", expect = "51 57 8B 7C 24 0C",
       unverified = "mvpb-veh-1", route = "L2", hazards = "none")]
pub fn find_maximum_speed_to_stop_at_stations(this: GPtr<u8>, out: GPtr<f32>) -> u8 {
    out.write(50.0);
    let mut min = 10000.0f32;
    if rd::<u8>(this, 0x5C0) != 0 {
        return 0;
    }
    let total = || g::<f32>(0xC37FEC).read();
    let hi = x(total()).fmul_m(0.5);
    let lo = x(total()).fmul_m(-0.5);
    let fwd = (rd::<u8>(this, 0x5B8) >> 6) & 1 != 0;
    for s in 0..6usize {
        let mut d = x(g::<f32>(0xC38034 + 4 * s).read()).fsub_m(rd::<f32>(this, 0x5A8));
        d = if fwd { d.fadd_m(40.0) } else { d.fsub_m(40.0) };
        if !d.fcom(hi).test_any(0x41) {
            loop {
                d = d.fsub_m(total());
                if d.fcom(hi).test_any(0x41) {
                    break;
                }
            }
        }
        while !d.fcom(lo).test_parity(0x05) {
            d = d.fadd_m(total());
        }
        let zero = x(0.0);
        let cand = if fwd {
            !d.fcom(zero).test_any(0x41)
        } else if !d.fcom(zero).test_parity(0x05) {
            d = d.fchs();
            true
        } else {
            false
        };
        if cand && !d.fcom(x(min)).test_parity(0x05) {
            min = f32_of(d);
        }
    }
    if !x(min).fcom(x(500.0)).test_parity(0x05) {
        out.write(f32_of(x(500.0).fsub_m(min).fmul_m(f32::from_bits(0x3B03_126F)).fsubr_m(1.0).fmul_m(50.0)));
    } else {
        out.write(100000.0);
    }
    u8::from(!x(min).fcom(x(5.0)).test_parity(0x05))
}

/// `CPlane::AreWeInNoPlaneZone()` (0x6CCAA0, static): the camera position (`[0xB6F03C] + 0x30`, else
/// 0xB6F02C) copied; true when `sqrt(dz² + dy² + dx²)` (that order, x87) to (-1073, -675, 50) is < 200,
/// or it lies strictly inside x (-2743, -2626) y (1300, 2200) or x (-1668, -1122) y (541, 1118).
#[hook(addr = 0x6CCAA0, conv = cdecl, name = "CPlane::AreWeInNoPlaneZone", expect = "A1 3C F0 B6 00 83 EC 0C",
       unverified = "mvpb-veh-1", route = "L2", hazards = "x87-transcendental")]
pub fn are_we_in_no_plane_zone() -> u8 {
    let cam = sa_world::world::common::cam_pos();
    let (px, py, pz) = (cam.read(), cam.wrapping_index(1).read(), cam.wrapping_index(2).read());
    let dx = x(px).fsub_m(-1073.0);
    let dy = x(py).fsub_m(-675.0);
    let dz = x(pz).fsub_m(50.0);
    let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
    if !d.fcom(x(200.0)).test_parity(0x05) {
        return 1;
    }
    let gt = |v: f32, c: f32| !x(v).fcom(x(c)).test_any(0x41);
    let lt = |v: f32, c: f32| !x(v).fcom(x(c)).test_parity(0x05);
    if gt(px, -2743.0) && lt(px, -2626.0) && gt(py, 1300.0) && lt(py, 2200.0) {
        return 1;
    }
    u8::from(gt(px, -1668.0) && lt(px, -1122.0) && gt(py, 541.0) && lt(py, 1118.0))
}

/// `CAutomobile::PlayCarHorn()` (0x6A3770): nothing while an alarm (+0x45C, u16, not 0/0xFFFF) runs on
/// a non-wrecked car (status bits +0x36 & 0xF8 != 0x28) or the horn counter (+0x514) runs. A non-zero
/// horn timer (+0x519) is decremented. Else timer = `(rand() & 0x7F) - 0x6A` (byte); `r = timer & 7`:
/// r < 2 -> counter 45; r 2..3 -> the driver (+0x460) with honk-at-car (+0x3DB bit 0) says 0x16, counter
/// 45; r >= 4 -> the driver says 0x16 (`CPed::Say(0x16, 0, 1.0, 0, 0, 0)`).
#[hook(addr = 0x6A3770, conv = thiscall, name = "CAutomobile::PlayCarHorn", this = "CAutomobile", expect = "56 8B F1 66 8B 86 5C 04 00 00",
       unverified = "mvpb-veh-1", route = "L2", hazards = "heap-writes,entry-protection-read")]
pub fn play_car_horn(this: GPtr<u8>) {
    let alarm = rd::<u16>(this, 0x45C);
    if alarm != 0 && alarm != 0xFFFF && rd::<u8>(this, 0x36) & 0xF8 != 0x28 {
        return;
    }
    if rd::<u32>(this, 0x514) != 0 {
        return;
    }
    let t = rd::<u8>(this, 0x519);
    if t != 0 {
        wr::<u8>(this, 0x519, t - 1);
        return;
    }
    let timer = ((rand_at(0x6A37B5) as u8) & 0x7F).wrapping_sub(0x6A);
    wr::<u8>(this, 0x519, timer);
    let r = timer & 7;
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if r < 2 {
        wr::<u32>(this, 0x514, 0x2D);
    } else if r < 4 {
        if !driver.is_null() && rd::<u8>(this, 0x3DB) & 1 != 0 {
            ped_say(driver, 0x16, 0, 0x3F80_0000, 0, 0, 0);
        }
        wr::<u32>(this, 0x514, 0x2D);
    } else if !driver.is_null() {
        ped_say(driver, 0x16, 0, 0x3F80_0000, 0, 0, 0);
    }
}

/// `CAutomobile::PlayHornIfNecessary()` (0x6A3820): with honk-at-car/ped (+0x3DB & 3) and not
/// `HasCarStoppedBecauseOfLight()`: the virtual `PlayCarHorn` (vtable +0xD8, tail jump).
#[hook(addr = 0x6A3820, conv = thiscall, name = "CAutomobile::PlayHornIfNecessary", this = "CAutomobile", expect = "56 8B F1 F6 86 DB 03 00 00 03",
       unverified = "mvpb-veh-1", route = "L2", hazards = "none")]
pub fn play_horn_if_necessary(this: GPtr<u8>) {
    if rd::<u8>(this, 0x3DB) & 3 != 0 && has_car_stopped_because_of_light_call(this) == 0 {
        play_car_horn_virtual(this);
    }
}

/// `ThePaths.m_pPathNodes[area]` (0x96F854; the area is not range-checked, as compiled).
fn path_nodes(area: u32) -> u32 {
    g::<u32>(0x96F854 + 4 * area as usize).read()
}

/// One link scan of `CAutomobile::HasCarStoppedBecauseOfLight`: over the links of node `addr`
/// (`{area, node}` dword), a link equal to `(want_area, want_node)` whose navi link's node has a
/// traffic light state (+0xC & 3) gives true.
fn link_has_light(addr: u32, want_area: impl Fn() -> u16, want_node: impl Fn() -> u16) -> bool {
    let area = addr & 0xFFFF;
    let node = |a: u32| path_nodes(a & 0xFFFF).wrapping_add((a >> 16).wrapping_mul(0x1C));
    if g::<u8>(node(addr) as usize + 0x18).read() & 0xF == 0 {
        return false;
    }
    let links = g::<u32>(0x96FA94 + 4 * area as usize).read();
    let mut i: i32 = 0;
    loop {
        let base = g::<i16>(node(addr) as usize + 0x10).read() as i32;
        let k = base.wrapping_add(i) as u32;
        let link = links.wrapping_add(k.wrapping_mul(4));
        if g::<u16>(link as usize).read() == want_area() && g::<u16>(link as usize + 2).read() == want_node() {
            let base = g::<i16>(node(addr) as usize + 0x10).read() as i32;
            let navi_links = g::<u32>(0x96FDF4 + 4 * area as usize).read();
            let k = base.wrapping_add(i) as u32;
            let nav = u32::from(g::<u16>(navi_links.wrapping_add(k.wrapping_mul(2)) as usize).read());
            let na = nav >> 10;
            if path_nodes(na) != 0 {
                let navi = g::<u32>(0x96F974 + 4 * na as usize).read();
                if g::<u8>(navi.wrapping_add((nav & 0x3FF) * 0xE) as usize + 0xC).read() & 3 != 0 {
                    return true;
                }
            }
        }
        i += 1;
        if i >= i32::from(g::<u8>(node(addr) as usize + 0x18).read() & 0xF) {
            return false;
        }
    }
}

/// `CAutomobile::HasCarStoppedBecauseOfLight()` (0x44D520, a `jmp` stub; body .HOODLUM 0x156E9C0;
/// docs/re/specs/VEH/CAutomobile.md): only for status 2/3 (+0x36 >> 3); the current node (+0x390)
/// and the starting route node (+0x394) areas must be loaded; true when a link of the current node to
/// the starting node (+0x394/+0x396, re-read per link) leads to a navi node with a traffic light state;
/// else, with an ending route node (+0x398 area != 0xFFFF, loaded), the same over the ending node's
/// links to the current node (+0x390/+0x392). Areas are not range-checked (as compiled).
#[hook(addr = 0x44D520, conv = thiscall, name = "CAutomobile::HasCarStoppedBecauseOfLight", this = "CAutomobile", expect = "E9 9B 14 12 01",
       unverified = "mvpb-veh-1", route = "L2", hazards = "none")]
pub fn has_car_stopped_because_of_light(this: GPtr<u8>) -> u8 {
    let st = rd::<u8>(this, 0x36) >> 3;
    if st != 2 && st != 3 {
        return 0;
    }
    let cur = rd::<u32>(this, 0x390);
    if path_nodes(cur & 0xFFFF) == 0 || path_nodes(u32::from(rd::<u16>(this, 0x394))) == 0 {
        return 0;
    }
    if link_has_light(cur, || rd::<u16>(this, 0x394), || rd::<u16>(this, 0x396)) {
        return 1;
    }
    if rd::<u16>(this, 0x398) == 0xFFFF {
        return 0;
    }
    let end = rd::<u32>(this, 0x398);
    if path_nodes(end & 0xFFFF) == 0 {
        return 0;
    }
    let cur_area = rd::<u16>(this, 0x390);
    u8::from(link_has_light(end, || cur_area, || rd::<u16>(this, 0x392)))
}

/// `CAutomobile::ProcessFlyingCarStuff()` (0x6A8500): for status 0, 8 or 3 (+0x36 >> 3) with the
/// flying-cars cheat (0x969160), a moving car (`|moveSpeed|` > 0, x87 sqrt of x² + y² + z²) and
/// `CTimer::ms_fTimeStep` > 0: `FlyingControl(3, -9999.99, -9999.99, -9999.99, -9999.99)`.
#[hook(addr = 0x6A8500, conv = thiscall, name = "CAutomobile::ProcessFlyingCarStuff", this = "CAutomobile", expect = "8A 41 36 C0 E8 03 84 C0",
       unverified = "mvpb-veh-1", route = "L2", hazards = "x87-transcendental")]
pub fn process_flying_car_stuff(this: GPtr<u8>) {
    let st = rd::<u8>(this, 0x36) >> 3;
    if st != 0 && st != 8 && st != 3 {
        return;
    }
    if g::<u8>(0x969160).read() == 0 {
        return;
    }
    let z = x(rd::<f32>(this, 0x4C));
    let y = x(rd::<f32>(this, 0x48));
    let xx = x(rd::<f32>(this, 0x44));
    let m = xx.fmul(xx).fadd(y.fmul(y)).fadd(z.fmul(z)).fsqrt();
    if m.fcom(x(0.0)).test_any(0x41) {
        return;
    }
    if x(g::<f32>(0xB7CB5C).read()).fcom(x(0.0)).test_any(0x41) {
        return;
    }
    let v = f32::from_bits(0xC61C_3FF6);
    flying_control(this, 3, v, v, v, v);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_SET_PED_IN_CAR_DIRECT,
    HOOK_PHYSICAL_APPLY_TURN_FORCE_COPY,
    HOOK_SETUP_PASSENGER,
    HOOK_SPECIAL_PLATE_FIND,
    HOOK_ADD_VEHICLE_UPGRADE,
    HOOK_SET_VEHICLE_UPGRADE_FLAGS,
    HOOK_CAN_BE_DELETED,
    HOOK_FIND_MAXIMUM_SPEED_TO_STOP_AT_STATIONS,
    HOOK_ARE_WE_IN_NO_PLANE_ZONE,
    HOOK_PLAY_CAR_HORN,
    HOOK_PLAY_HORN_IF_NECESSARY,
    HOOK_HAS_CAR_STOPPED_BECAUSE_OF_LIGHT,
    HOOK_PROCESS_FLYING_CAR_STUFF,
];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
