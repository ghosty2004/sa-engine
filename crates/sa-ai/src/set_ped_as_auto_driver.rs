//! `CTaskSimpleSetPedAsAutoDriver::ProcessPed` (0x649020): the step that hands a car to its AI driver (a ped
//! that took a car at Grove Street ran it on the native walk after the frame-limiter toggle shifted the timing).
//! Ported from the 1.0 US bytes; gta-reversed `TaskSimpleSetPedAsAutoDriver.cpp` as a reading aid.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CCarCtrl::JoinCarWithRoadSystem(CVehicle*)` (0x42F5A0)
    fn join_car_with_road_system(veh: GPtr<u8>) = cdecl 0x42F5A0 => unported;
}

/// `CTaskSimpleSetPedAsAutoDriver::ProcessPed(CPed*)` (0x649020, thiscall, `ret 4`; the ped is unused). With the
/// vehicle at +8 (read again before every store, no NULL check): `CCarCtrl::JoinCarWithRoadSystem(vehicle)`; the
/// status bits of +0x36 become `STATUS_PHYSICS` (`(b & 7) | 0x18`); the autopilot's driving style +0x3BA = 1 and
/// cruise speed +0x3D0 = 10; vehicle flag +0x428 bit 0x10 (engine on) = !(+0x42D bit 0x2 (engine broken)).
/// Returns 1 (AL; the rest of EAX holds the vehicle pointer's upper bytes in the original).
#[hook(addr = 0x649020, conv = thiscall, name = "CTaskSimpleSetPedAsAutoDriver::ProcessPed", expect = "56 8B F1 8B 46 08",
       unverified = "framelimit-ai", route = "L2", hazards = "heap-writes")]
pub fn set_ped_as_auto_driver_process_ped(this: GPtr<u8>, _ped: GPtr<u8>) -> u8 {
    join_car_with_road_system(rd::<GPtr<u8>>(this, 8));
    let veh = rd::<GPtr<u8>>(this, 8);
    wr::<u8>(veh, 0x36, (rd::<u8>(veh, 0x36) & 7) | 0x18);
    wr::<u8>(rd::<GPtr<u8>>(this, 8), 0x3BA, 1);
    wr::<u8>(rd::<GPtr<u8>>(this, 8), 0x3D0, 10);
    let veh = rd::<GPtr<u8>>(this, 8);
    let flags = rd::<u8>(veh, 0x428);
    if rd::<u8>(veh, 0x42D) & 2 != 0 {
        wr::<u8>(veh, 0x428, flags & 0xEF);
    } else {
        wr::<u8>(veh, 0x428, flags | 0x10);
    }
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_SET_PED_AS_AUTO_DRIVER_PROCESS_PED];
