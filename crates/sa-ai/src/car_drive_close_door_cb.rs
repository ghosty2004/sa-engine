//! `CTaskSimpleCarDrive`'s close-door anim finish callback (0x63C840, batch `carjack-1`; catalogue `FUN_0063c840`,
//! installed by 0x642700 for anim 0x182). Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CVehicle::ClearGettingOutFlags(uint8 doors)` (0x6D3060, `ret 4`)
    fn vehicle_clear_getting_out_flags(v: GPtr<u8>, doors: u32) = thiscall 0x6D3060 => unported;
    /// `CVehicleAnimGroup::GetGroup(int32 animId)` (0x6E3B00, `ret 4`)
    fn vehicle_anim_group_get_group(g: GPtr<u8>, anim: u32) -> u32 = thiscall 0x6E3B00 => unported;
    /// `CVehicle::ProcessOpenDoor(CPed*, uint32 door, uint32 group, uint32 anim, float time)` (vtable +0x70)
    fn v_vehicle_process_open_door(v: GPtr<u8>, ped: GPtr<u8>, door: u32, group: u32, anim: u32, time: u32) = thiscall vslot 0x70 => unported;
}

/// `FUN_0063c840(CAnimBlendAssociation*, void* task)` (0x63C840, cdecl; a `CTaskSimpleCarDrive`): task +0x5C loses
/// bit 0x20, +0xC (the assoc) = NULL; with a vehicle (+8): `ClearGettingOutFlags(1)`, and with its driver (+0x460)
/// `veh->ProcessOpenDoor(driver, 10, GetGroup(0x182) of the vehicle's anim group, 0x182, 1.0)`.
#[hook(addr = 0x63C840, conv = cdecl, name = "FUN_0063c840", expect = "56 8B 74 24 0C 80 66 5C DF",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
pub fn car_drive_close_door_finish_cb(_assoc: GPtr<u8>, task: GPtr<u8>) {
    wr::<u8>(task, 0x5C, rd::<u8>(task, 0x5C) & 0xDF);
    let v = rd::<GPtr<u8>>(task, 8);
    wr::<u32>(task, 0xC, 0);
    if v.is_null() {
        return;
    }
    vehicle_clear_getting_out_flags(v, 1);
    let v = rd::<GPtr<u8>>(task, 8);
    if rd::<u32>(v, 0x460) == 0 {
        return;
    }
    let grp = GPtr::from_addr(0xC1CDC0 + usize::from(rd::<u8>(rd::<GPtr<u8>>(v, 0x384), 0xDE)) * 0x94);
    let group = vehicle_anim_group_get_group(grp, 0x182);
    let v = rd::<GPtr<u8>>(task, 8);
    v_vehicle_process_open_door(v, rd::<GPtr<u8>>(v, 0x460), 10, group, 0x182, 0x3F80_0000);
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[HOOK_CAR_DRIVE_CLOSE_DOOR_FINISH_CB];
