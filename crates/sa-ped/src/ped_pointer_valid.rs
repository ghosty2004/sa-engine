//! WRITER-NATIVE (batch `native-16`): `IsPedPointerValid` (0x5E8A30), reached by the attractor / road-rage
//! tasks on the native walk. Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::IsEntityPointerValid(CEntity*)` (0x533310)
    fn entity_is_entity_pointer_valid(e: GPtr<u8>) -> u8 = cdecl 0x533310 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
}

/// `CPools::ms_pPedPool` (0xB74490)
const PED_POOL: usize = 0xB7_4490;

/// `IsPedPointerValid(CPed*)` (0x5E8A30, cdecl, bool in AL).
///
/// False unless the pointer is a used slot of the ped pool (`(ped - objects) / 0x7C4` truncated, in range, flag
/// byte not negative). A ped in a vehicle (+0x46C bit 8, +0x58C) answers `IsEntityPointerValid(vehicle)` (a tail
/// jump). Otherwise true with a collision list (+0xB0) or for the player ped, else false.
#[hook(addr = 0x5E8A30, conv = cdecl, name = "IsPedPointerValid",
       expect = "56 8B 74 24 08 57 8B 3D 90 44 B7 00",
       unverified = "native-16", route = "L2", hazards = "ax-al-return", recs = "grove-walk")]
pub fn is_ped_pointer_valid(ped: GPtr<u8>) -> u8 {
    let pool = g::<GPtr<u8>>(PED_POOL).read();
    let diff = (ped.addr() as u32).wrapping_sub(rd::<u32>(pool, 0)) as i32;
    let idx = diff / 0x7C4;
    if idx < 0 || idx >= rd::<i32>(pool, 8) {
        return 0;
    }
    if rd::<GPtr<i8>>(pool, 4).byte_add(idx as isize).read() < 0 {
        return 0;
    }
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        if !v.is_null() {
            return entity_is_entity_pointer_valid(v);
        }
    }
    if rd::<u32>(ped, 0xB0) == 0 && ped != find_player_ped(-1) {
        return 0;
    }
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_IS_PED_POINTER_VALID];
