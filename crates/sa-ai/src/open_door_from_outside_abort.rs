//! `CTaskSimpleCarOpenDoorFromOutside::MakeAbortable` (0x64AD00, batch `carjack-1`): reached when a ped's door-opening
//! task is flushed (e.g. CJ teleported or knocked away while opening a door to jack). Ported from the 1.0 US bytes
//! (gta-reversed as a reading aid). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskSimpleCarOpenDoorFromOutside::ComputeAnimID(int32* group, int32* anim)` (0x645FA0, `ret 8`)
    fn compute_anim_id(t: GPtr<u8>, group: GPtr<u8>, anim: GPtr<u8>) = thiscall 0x645FA0 => unported;
    /// `CVehicle::ProcessOpenDoor(CPed*, uint32 door, uint32 group, uint32 anim, float time)` (vtable +0x70)
    fn v_vehicle_process_open_door(v: GPtr<u8>, ped: GPtr<u8>, door: u32, group: u32, anim: u32, time: u32) = thiscall vslot 0x70 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
}

fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// `CTaskSimpleCarOpenDoorFromOutside::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x64AD00, `ret 0xC`).
/// Priority 2 only (else false): the assoc (+0xC), if any, gets blend delta -1000.0; with a vehicle (+0x10)
/// `ComputeAnimID(&group, &anim)` (group in the pushed-ECX slot = `this`, anim in the priority slot = 2, kept for a
/// door outside the table) and `veh->ProcessOpenDoor(ped, door +0x14, group, anim, 1.0)`; when +0x19 is set it is
/// cleared and `FindPlayerPed(-1)` +0x474 gains 0x4000000; true.
#[hook(addr = 0x64AD00, conv = thiscall, name = "CTaskSimpleCarOpenDoorFromOutside::MakeAbortable", expect = "51 83 7C 24 0C 02",
       unverified = "carjack-1", route = "L2", hazards = "ax-al-return,heap-writes", recs = "carjack")]
pub fn open_door_from_outside_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, _event: GPtr<u8>) -> u8 {
    if priority != 2 {
        return 0;
    }
    let a = rd::<GPtr<u8>>(this, 0xC);
    if !a.is_null() {
        wr::<u32>(a, 0x1C, 0xC47A_0000);
    }
    if !rd::<GPtr<u8>>(this, 0x10).is_null() {
        let mut l = [this.addr() as u32, 2];
        let p = buf(&mut l);
        compute_anim_id(this, p, p.byte_add(4));
        let v = rd::<GPtr<u8>>(this, 0x10);
        v_vehicle_process_open_door(v, ped, rd::<u32>(this, 0x14), l[0], l[1], 0x3F80_0000);
    }
    if rd::<u8>(this, 0x19) != 0 {
        wr::<u8>(this, 0x19, 0);
        let p = find_player_ped(-1);
        wr::<u32>(p, 0x474, rd::<u32>(p, 0x474) | 0x400_0000);
    }
    1
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[HOOK_OPEN_DOOR_FROM_OUTSIDE_MAKE_ABORTABLE];
