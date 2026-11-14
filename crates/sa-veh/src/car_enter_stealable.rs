//! WRITER-NATIVE (batch `native-8`): `CCarEnterExit::IsVehicleStealable` (0x6510D0) with
//! `CCarEnterExit::IsClearToDriveAway` (0x6509B0) and `CPedGroup::IsAnyoneUsingCar` (0x5F7DB0), reached when
//! ambient peds look for a car to steal on the native walk. Ported from the 1.0 US bytes; gta-reversed
//! (`CarEnterExit.cpp`, `PedGroup.cpp`) as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{model_info, rd, x};

use sa_stream::bindings::ped_is_player;
use sa_world::bindings::intel_find_task_by_type;
use sa_world::bindings::intel_is_friendly_with;
use crate::bindings::is_law_enforcement_vehicle as vehicle_is_law_enforcement;
use sa_world::bindings::ped_groups_are_in_same_group as groups_are_in_same_group;
use sa_world::bindings::ped_groups_get_peds_group as groups_get_peds_group;
use crate::bindings::upside_down_car_check_is_car_upside_down as upside_down_check_is_car_upside_down;
use sa_world::bindings::vehicle_can_be_driven;
use sa_world::bindings::vehicle_is_on_its_side;
use crate::bindings::vehicle_is_upside_down;
use sa_world::bindings::process_line_of_sight as world_process_line_of_sight;
orig! {
    /// `CPedGroup::IsAnyoneUsingCar(const CVehicle*)` (0x5F7DB0)
    fn group_is_anyone_using_car_c(g: GPtr<u8>, v: GPtr<u8>) -> u8 = thiscall 0x5F7DB0 => group_is_anyone_using_car;
    /// `CCarEnterExit::IsClearToDriveAway(const CVehicle*)` (0x6509B0)
    fn is_clear_to_drive_away_c(v: GPtr<u8>) -> u8 = cdecl 0x6509B0 => is_clear_to_drive_away;
}

/// `CCarEnterExit::IsVehicleStealable(const CVehicle* vehicle, const CPed* ped)` (0x6510D0, cdecl, bool in AL).
///
/// False for vehicle classes +0x594 = 3 / 4, types other than automobile / bike, a vehicle in creation state
/// +0x4A4 other than 1 / 3 that is not the ped's own, an upside-down car (`CUpsideDownCarCheck`, called on an
/// uninitialised stack object), one that cannot be driven or a law vehicle; with a driver: a mission driver
/// (+0x484 = 2), the player, a friend of the ped or a member of its group; when someone of the ped's group uses
/// it; with +0x490 set; health +0x4C0 not above 600; upside down or on its side; not clear to drive away.
#[hook(addr = 0x6510D0, conv = cdecl, name = "CCarEnterExit::IsVehicleStealable",
       expect = "83 EC 30 56 8B 74 24 38",
       unverified = "native-8", route = "L2", hazards = "ax-al-return,uninit-locals", recs = "grove-walk,car-ai",
       note = "IsCarUpsideDown runs on an uninitialised local object (it does not read it)")]
pub fn is_vehicle_stealable(veh: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let class = rd::<u32>(veh, 0x594);
    if class == 4 || class == 3 {
        return 0;
    }
    let ty = rd::<u32>(veh, 0x590);
    if ty != 0 && ty != 9 {
        return 0;
    }
    let state = rd::<u8>(veh, 0x4A4);
    if state != 1 && state != 3 && rd::<GPtr<u8>>(ped, 0x58C) != veh {
        return 0;
    }
    let mut check = [0u32; 12];
    if upside_down_check_is_car_upside_down(GPtr::from_raw(&mut check as *mut [u32; 12]).cast(), veh) != 0 {
        return 0;
    }
    if vehicle_can_be_driven(veh) == 0 || vehicle_is_law_enforcement(veh) != 0 {
        return 0;
    }
    let driver = rd::<GPtr<u8>>(veh, 0x460);
    if !driver.is_null() {
        if rd::<u8>(driver, 0x484) == 2 || ped_is_player(driver) != 0 {
            return 0;
        }
        if intel_is_friendly_with(rd::<GPtr<u8>>(ped, 0x47C), rd::<GPtr<u8>>(veh, 0x460)) != 0 {
            return 0;
        }
        if groups_are_in_same_group(ped, rd::<GPtr<u8>>(veh, 0x460)) != 0 {
            return 0;
        }
    }
    let group = GPtr::from_addr(groups_get_peds_group(ped.addr() as u32) as usize);
    if !group.is_null() && group_is_anyone_using_car_c(group, veh) != 0 {
        return 0;
    }
    if rd::<u32>(veh, 0x490) != 0 {
        return 0;
    }
    if x(rd::<f32>(veh, 0x4C0)).fcom(x(600.0)).test_any(0x41) {
        return 0;
    }
    if vehicle_is_upside_down(veh) != 0 || vehicle_is_on_its_side(veh) != 0 {
        return 0;
    }
    u8::from(is_clear_to_drive_away_c(veh) != 0)
}

/// `CCarEnterExit::IsClearToDriveAway(const CVehicle*)` (0x6509B0, cdecl, bool in AL).
///
/// `len = (bbox.max.y - bbox.min.y) * 0.5 + 1.5` (model collision box); a line of sight (buildings and vehicles,
/// see-through and camera-ignore) from `pos + forward * len` back to the position: true unless it hits
/// something other than the vehicle. The original keeps the hit entity in its argument slot.
#[hook(addr = 0x6509B0, conv = cdecl, name = "CCarEnterExit::IsClearToDriveAway",
       expect = "83 EC 44 56 8B 74 24 4C",
       unverified = "native-8", route = "L2", hazards = "ax-al-return,stack-residue", recs = "grove-walk,car-ai")]
pub fn is_clear_to_drive_away(veh: GPtr<u8>) -> u8 {
    let info = model_info(i32::from(rd::<i16>(veh, 0x22)));
    let col = rd::<GPtr<u8>>(info, 0x14);
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let pos = if m.is_null() { veh.byte_add(4) } else { m.byte_add(0x30) };
    let len = x(rd::<f32>(col, 0x10))
        .fsub_m(rd::<f32>(col, 4))
        .fmul_m(0.5)
        .fadd_m(1.5);
    // the original reads the forward row through `m + 0x10` (null matrix: address 0x10)
    let fwd = m.byte_add(0x10);
    let fy = x(rd::<f32>(fwd, 4)).fmul(len).to_f32();
    let fz = x(rd::<f32>(fwd, 8)).fmul(len).to_f32();
    let mut start = [
        x(rd::<f32>(fwd, 0)).fmul(len).fadd_m(rd::<f32>(pos, 0)).to_f32(),
        x(fy).fadd_m(rd::<f32>(pos, 4)).to_f32(),
        x(fz).fadd_m(rd::<f32>(pos, 8)).to_f32(),
    ];
    let mut hit: GPtr<u8> = GPtr::NULL;
    let mut cp = [0u32; 11];
    let r = world_process_line_of_sight(
        GPtr::from_raw(&mut start as *mut [f32; 3]).cast(),
        pos,
        GPtr::from_raw(&mut cp as *mut [u32; 11]).cast(),
        GPtr::from_raw(&mut hit as *mut GPtr<u8>).cast(),
        1,
        1,
        0,
        0,
        0,
        1,
        1,
        0,
    );
    u8::from(r == 0 || hit == veh)
}

/// `CPedGroup::IsAnyoneUsingCar(const CVehicle*)` (0x5F7DB0, `ret 4`, bool in AL): a member (+0xC, 8 slots)
/// in the vehicle (+0x58C with in-vehicle flag +0x46C bit 8), or with a `0x2BD` / `0x2BC` task whose +0xC is it.
#[hook(addr = 0x5F7DB0, conv = thiscall, name = "CPedGroup::IsAnyoneUsingCar",
       expect = "53 55 8B 6C 24 0C",
       unverified = "native-8", route = "L2", hazards = "ax-al-return", recs = "grove-walk,car-ai")]
pub fn group_is_anyone_using_car(this: GPtr<u8>, veh: GPtr<u8>) -> u8 {
    for k in 0..8 {
        let m = rd::<GPtr<u8>>(this, 0xC + k * 4);
        if m.is_null() {
            continue;
        }
        let v = rd::<GPtr<u8>>(m, 0x58C);
        if !v.is_null() && rd::<u32>(m, 0x46C) & 0x100 != 0 && v == veh {
            return 1;
        }
        for ty in [0x2BD, 0x2BC] {
            let t = intel_find_task_by_type(rd::<GPtr<u8>>(m, 0x47C), ty);
            if !t.is_null() && rd::<GPtr<u8>>(t, 0xC) == veh {
                return 1;
            }
        }
    }
    0
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_IS_VEHICLE_STEALABLE, HOOK_IS_CLEAR_TO_DRIVE_AWAY, HOOK_GROUP_IS_ANYONE_USING_CAR];
