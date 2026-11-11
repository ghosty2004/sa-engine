//! CCarEnterExit (the leaf helpers) (PORT-VEH). Ported from the 1.0 US bytes; gta-reversed as a reading aid.
//!
//! All functions are static (cdecl). Vehicle fields used: `+0x36` status (upper 5 bits), `+0x384`
//! handling data (`+0xCC` handling flags, bit 9 tested; `+0xDE` anim group), `+0x429` vehicle flags byte,
//! `+0x460` driver, `+0x464..+0x46C` passengers 0..2, `+0x486`/`+0x487` getting-in/getting-out door
//! flags, `+0x590` vehicle subclass (9 = bike). Door ids: 8 front left, 9 rear left, 10 front right,
//! 11 rear right, 18 (rear doors of the Rhino/van style cars).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr, x};

orig! {
    /// `CVehicle::IsDoorReady(uint32)` (vtable slot 0x7C)
    pub fn veh_is_door_ready(veh: GPtr<u8>, door: u32) -> u8 = thiscall vslot 0x7C => unported;
    /// `CVehicle::IsDoorFullyOpen(uint32)` (vtable slot 0x84)
    pub fn veh_is_door_fully_open(veh: GPtr<u8>, door: u32) -> u8 = thiscall vslot 0x84 => unported;
    /// `CVehicle::IsDoorClosed(uint32)` (vtable slot 0x8C)
    pub fn veh_is_door_closed(veh: GPtr<u8>, door: u32) -> u8 = thiscall vslot 0x8C => unported;
    /// `CVehicle::IsDoorMissing(uint32)` (vtable slot 0x94)
    pub fn veh_is_door_missing(veh: GPtr<u8>, door: u32) -> u8 = thiscall vslot 0x94 => unported;
    /// `CVehicle::CanPedOpenLocks(const CPed*)` (0x6D1E20)
    pub fn can_ped_open_locks(veh: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6D1E20 => unported;
    /// `RpAnimBlendClumpGetFirstAssociation(RpClump*, uint32 flags)` (0x4D6A70)
    pub fn clump_get_first_association(clump: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x4D6A70 => unported;
    /// `RpAnimBlendGetNextAssociation(CAnimBlendAssociation*, uint32 flags)` (0x4D6AD0)
    pub fn get_next_association(assoc: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x4D6AD0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610)
    pub fn blend_animation(clump: GPtr<u8>, group: u32, anim: u32, delta: f32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
}

/// `subclass == 9 (bike) || handling flags bit 9`: the test every door helper starts with.
fn is_bike_like(veh: GPtr<u8>) -> bool {
    rd::<u32>(veh, 0x590) == 9 || (rd::<u32>(rd::<GPtr<u8>>(veh, 0x384), 0xCC) >> 9) & 1 != 0
}

/// The door-id -> getting-in/out flag map of ComputeDoorFlag's car branch and IsCarDoorInUse
/// (8 -> 4, 9 -> 8, 10 -> 1, 11 -> 2, 18 -> 1, else 0).
fn car_door_flag(door: i32) -> u32 {
    match door {
        8 => 4,
        9 => 8,
        10 | 18 => 1,
        11 => 2,
        _ => 0,
    }
}

/// `CCarEnterExit::IsVehicleHealthy(const CVehicle*)` (0x64EEC0): status != wrecked (`(+0x36 & 0xF8) != 0x28`);
/// full EAX 0/1.
#[hook(addr = 0x64EEC0, conv = cdecl, name = "CCarEnterExit::IsVehicleHealthy", expect = "8B 44 24 04 8A 48 36")]
pub fn is_vehicle_healthy(veh: GPtr<u8>) -> u32 {
    u32::from(rd::<u8>(veh, 0x36) & 0xF8 != 0x28)
}

/// `CCarEnterExit::IsPedHealthy(CPed*)` (0x64EEE0): `health (+0x540) > 0.0` (`test ah, 0x41`; NaN is
/// false); full EAX 0/1.
#[hook(addr = 0x64EEE0, conv = cdecl, name = "CCarEnterExit::IsPedHealthy", expect = "8B 44 24 04 D9 80 40 05 00 00")]
pub fn is_ped_healthy(ped: GPtr<u8>) -> u32 {
    let zero = g::<f32>(0x858B50).read();
    u32::from(!x(rd::<f32>(ped, 0x540)).fcom(x(zero)).test_any(0x41))
}

/// `CCarEnterExit::IsCarDoorReady(const CVehicle*, int32)` (0x64ED90): `IsDoorReady || IsDoorFullyOpen`
/// (virtual); full EAX 0/1.
#[hook(addr = 0x64ED90, conv = cdecl, name = "CCarEnterExit::IsCarDoorReady", expect = "56 8B 74 24 08")]
pub fn is_car_door_ready(veh: GPtr<u8>, door: u32) -> u32 {
    u32::from(veh_is_door_ready(veh, door) != 0 || veh_is_door_fully_open(veh, door) != 0)
}

/// `CCarEnterExit::CarHasDoorToClose(const CVehicle*, int32)` (0x64EE10): `!IsDoorMissing && !IsDoorClosed`;
/// full EAX 0/1.
#[hook(addr = 0x64EE10, conv = cdecl, name = "CCarEnterExit::CarHasDoorToClose", expect = "56 8B 74 24 08")]
pub fn car_has_door_to_close(veh: GPtr<u8>, door: u32) -> u32 {
    u32::from(veh_is_door_missing(veh, door) == 0 && veh_is_door_closed(veh, door) == 0)
}

/// `CCarEnterExit::CarHasDoorToOpen(const CVehicle*, int32)` (0x64EDD0, `mov eax, 1` in a .HOODLUM
/// fragment): `!IsDoorMissing && !IsDoorFullyOpen`; full EAX 0/1.
#[hook(addr = 0x64EDD0, conv = cdecl, name = "CCarEnterExit::CarHasDoorToOpen", expect = "56 8B 74 24 08")]
pub fn car_has_door_to_open(veh: GPtr<u8>, door: u32) -> u32 {
    u32::from(veh_is_door_missing(veh, door) == 0 && veh_is_door_fully_open(veh, door) == 0)
}

/// `CCarEnterExit::CarHasPartiallyOpenDoor(const CVehicle*, int32)` (0x64EE70): not missing, not fully
/// open, not closed; full EAX 0/1.
#[hook(addr = 0x64EE70, conv = cdecl, name = "CCarEnterExit::CarHasPartiallyOpenDoor", expect = "56 8B 74 24 08")]
pub fn car_has_partially_open_door(veh: GPtr<u8>, door: u32) -> u32 {
    u32::from(
        veh_is_door_missing(veh, door) == 0
            && veh_is_door_fully_open(veh, door) == 0
            && veh_is_door_closed(veh, door) == 0,
    )
}

/// `CCarEnterExit::CarHasOpenableDoor(const CVehicle*, int32, const CPed*)` (0x64EE50): only
/// `CanPedOpenLocks(ped)`, the door is ignored. `setne al`: EAX's upper bytes are CanPedOpenLocks'.
#[hook(addr = 0x64EE50, conv = cdecl, name = "CCarEnterExit::CarHasOpenableDoor", expect = "8B 44 24 0C 8B 4C 24 04")]
pub fn car_has_openable_door(veh: GPtr<u8>, _door: u32, ped: GPtr<u8>) -> u8 {
    u8::from(can_ped_open_locks(veh, ped) != 0)
}

/// `CCarEnterExit::IsCarQuickJackPossible(CVehicle*, int32, const CPed*)` (0x64EF00): original bug kept,
/// always false (the final `mov al, [veh+0x429]` is followed by `xor al, al`). The checks still run:
/// door 10, automobile subclass, door 10 not missing and closed, CanPedOpenLocks, a driver, its
/// `+0x484` byte. Only AL is defined (EAX's upper bytes are leftovers).
#[hook(addr = 0x64EF00, conv = cdecl, name = "CCarEnterExit::IsCarQuickJackPossible", expect = "83 7C 24 08 0A")]
pub fn is_car_quick_jack_possible(veh: GPtr<u8>, door: i32, ped: GPtr<u8>) -> u8 {
    if door != 10 || rd::<u32>(veh, 0x590) != 0 {
        return 0;
    }
    if veh_is_door_missing(veh, 10) != 0 || veh_is_door_closed(veh, 10) == 0 {
        return 0;
    }
    if can_ped_open_locks(veh, ped) == 0 {
        return 0;
    }
    let driver = rd::<GPtr<u8>>(veh, 0x460);
    if driver.is_null() || rd::<u8>(driver, 0x484) == 2 {
        return 0;
    }
    let _ = rd::<u8>(veh, 0x429);
    0
}

/// `CCarEnterExit::ComputeTargetDoorToEnterAsPassenger(const CVehicle*, int32)` (0x64F190): flag
/// `+0x429 & 2` -> 8; passenger 0 -> 11 (bike-like) else 8; 1 -> 11; 2 -> 9; else -1.
#[hook(addr = 0x64F190, conv = cdecl, name = "CCarEnterExit::ComputeTargetDoorToEnterAsPassenger", expect = "8B 54 24 04 8A 8A 29 04 00 00")]
pub fn compute_target_door_to_enter_as_passenger(veh: GPtr<u8>, passenger: i32) -> i32 {
    if rd::<u8>(veh, 0x429) & 2 != 0 {
        return 8;
    }
    match passenger {
        0 => {
            if is_bike_like(veh) {
                11
            } else {
                8
            }
        }
        1 => 11,
        2 => 9,
        _ => -1,
    }
}

/// `CCarEnterExit::ComputePassengerIndexFromCarDoor(const CVehicle*, int32)` (0x64F1E0): bike-like: 9 and
/// 11 -> 0; else 8 -> 0, 9 -> 2, 11 -> 1; anything else -1. EDX is preserved (the original never writes
/// it and CCarEnterExit::SetPedInCarDirect pushes EDX after the call, at 0x650360).
#[hook(addr = 0x64F1E0, conv = cdecl, name = "CCarEnterExit::ComputePassengerIndexFromCarDoor", expect = "8B 44 24 04 83 B8 90 05 00 00 09", preserve = "edx")]
pub fn compute_passenger_index_from_car_door(veh: GPtr<u8>, door: i32) -> i32 {
    if is_bike_like(veh) {
        return if door == 9 || door == 11 { 0 } else { -1 };
    }
    match door {
        8 => 0,
        9 => 2,
        11 => 1,
        _ => -1,
    }
}

/// `CCarEnterExit::ComputeDoorFlag(const CVehicle*, int32, bool)` (0x64E550): with the bool and a
/// bike-like vehicle: 8/10/18 -> 5, 9/11 -> 10; otherwise 8 -> 4, 9 -> 8, 10/18 -> 1, 11 -> 2; else 0.
#[hook(addr = 0x64E550, conv = cdecl, name = "CCarEnterExit::ComputeDoorFlag", expect = "8A 4C 24 0C 33 C0")]
pub fn compute_door_flag(veh: GPtr<u8>, door: i32, check_type: u32) -> u32 {
    if check_type as u8 != 0 && is_bike_like(veh) {
        return match door {
            8 | 10 | 18 => 5,
            9 | 11 => 10,
            _ => 0,
        };
    }
    car_door_flag(door)
}

/// `CCarEnterExit::ComputeOppositeDoorFlag(const CVehicle*, int32, bool)` (0x64E610): bike-like branch as
/// ComputeDoorFlag; otherwise 8 -> 1, 9 -> 2, 10/18 -> 4, 11 -> 8; else 0.
#[hook(addr = 0x64E610, conv = cdecl, name = "CCarEnterExit::ComputeOppositeDoorFlag", expect = "8A 4C 24 0C 33 C0")]
pub fn compute_opposite_door_flag(veh: GPtr<u8>, door: i32, check_type: u32) -> u32 {
    if check_type as u8 != 0 && is_bike_like(veh) {
        return match door {
            8 | 10 | 18 => 5,
            9 | 11 => 10,
            _ => 0,
        };
    }
    match door {
        8 => 1,
        9 => 2,
        10 | 18 => 4,
        11 => 8,
        _ => 0,
    }
}

/// `CCarEnterExit::ComputeTargetDoorToExit(const CVehicle*, const CPed*)` (0x64F110): driver -> 10;
/// anim group 15/16 -> 8; passenger 0 -> 11 if bike-like else 8; passenger 1 -> 11; 2 -> 9; else -1.
#[hook(addr = 0x64F110, conv = cdecl, name = "CCarEnterExit::ComputeTargetDoorToExit", expect = "8B 4C 24 04 8B 91 60 04 00 00")]
pub fn compute_target_door_to_exit(veh: GPtr<u8>, ped: GPtr<u8>) -> i32 {
    if rd::<GPtr<u8>>(veh, 0x460) == ped {
        return 10;
    }
    let anim_group = u32::from(rd::<u8>(rd::<GPtr<u8>>(veh, 0x384), 0xDE));
    if anim_group == 0xF || anim_group == 0x10 {
        return 8;
    }
    if rd::<GPtr<u8>>(veh, 0x464) == ped {
        return if is_bike_like(veh) { 11 } else { 8 };
    }
    if rd::<GPtr<u8>>(veh, 0x468) == ped {
        return 11;
    }
    if rd::<GPtr<u8>>(veh, 0x46C) == ped {
        return 9;
    }
    -1
}

/// The occupant a slow jack through `door` would drag out (shared by IsCarSlowJackRequired /
/// ComputeSlowJackedPed): bike-like: 8/10/18 driver, 9/11 passenger 0; anim group 15/16 (only for
/// IsCarSlowJackRequired): 10 driver, else none; else 8 passenger 0, 9 passenger 2, 10 driver,
/// 11 passenger 1.
fn slow_jack_slot(veh: GPtr<u8>, door: i32, check_anim_group: bool) -> Option<isize> {
    if is_bike_like(veh) {
        return match door {
            8 | 10 | 18 => Some(0x460),
            9 | 11 => Some(0x464),
            _ => None,
        };
    }
    if check_anim_group {
        let ag = u32::from(rd::<u8>(rd::<GPtr<u8>>(veh, 0x384), 0xDE));
        if ag == 0xF || ag == 0x10 {
            return if door == 10 { Some(0x460) } else { None };
        }
    }
    match door {
        8 => Some(0x464),
        9 => Some(0x46C),
        10 => Some(0x460),
        11 => Some(0x468),
        _ => None,
    }
}

/// `CCarEnterExit::IsCarSlowJackRequired(const CVehicle*, int32)` (0x64EF70): the occupant behind the
/// door is not null. Only AL is defined (`xor al, al` / `mov al, 1`; EAX's upper bytes are the caller's).
#[hook(addr = 0x64EF70, conv = cdecl, name = "CCarEnterExit::IsCarSlowJackRequired", expect = "8B 4C 24 04 8B 91 90 05 00 00")]
pub fn is_car_slow_jack_required(veh: GPtr<u8>, door: i32) -> u8 {
    match slow_jack_slot(veh, door, true) {
        Some(o) => u8::from(!rd::<GPtr<u8>>(veh, o).is_null()),
        None => 0,
    }
}

/// `CCarEnterExit::ComputeSlowJackedPed(const CVehicle*, int32)` (0x64F070): the occupant behind the door
/// (no anim-group special case), else null.
#[hook(addr = 0x64F070, conv = cdecl, name = "CCarEnterExit::ComputeSlowJackedPed", expect = "8B 4C 24 04 8B 91 90 05 00 00")]
pub fn compute_slow_jacked_ped(veh: GPtr<u8>, door: i32) -> GPtr<u8> {
    match slow_jack_slot(veh, door, false) {
        Some(o) => rd::<GPtr<u8>>(veh, o),
        None => GPtr::from_addr(0),
    }
}

/// `CCarEnterExit::IsCarDoorInUse(const CVehicle*, int32, int32)` (0x64EC90): either door (0 = none)
/// has its flag in the getting-in (`+0x486`) or getting-out (`+0x487`) byte; full EAX 0/1.
#[hook(addr = 0x64EC90, conv = cdecl, name = "CCarEnterExit::IsCarDoorInUse", expect = "8B 4C 24 08 32 D2")]
pub fn is_car_door_in_use(veh: GPtr<u8>, door1: i32, door2: i32) -> u32 {
    let in_use = |door: i32| {
        if door == 0 {
            return false;
        }
        let f = car_door_flag(door) as u8;
        rd::<u8>(veh, 0x486) & f != 0 || rd::<u8>(veh, 0x487) & f != 0
    };
    let a = in_use(door1);
    let b = in_use(door2);
    u32::from(a || b)
}

/// Marks every association of `clump` with `first_flags` (first) / `flags` (next) for deletion (`+0x2E |= 4`) and a blend delta of
/// -1000.0 (`+0x1C`).
fn blend_out_associations(clump: GPtr<u8>, first_flags: u32, flags: u32) {
    let mut a = clump_get_first_association(clump, first_flags);
    while !a.is_null() {
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 4);
        wr::<u32>(a, 0x1C, 0xC47A_0000);
        a = get_next_association(a, flags);
    }
}

/// `CCarEnterExit::RemoveGetInAnims(const CPed*)` (0x64F6E0): blends out the clump's (`ped+0x18`)
/// associations with flag 0x10. Writes the associations' `+0x1C` (f32) and `+0x2E` (byte).
#[hook(addr = 0x64F6E0, conv = cdecl, name = "CCarEnterExit::RemoveGetInAnims", expect = "8B 44 24 04 8B 48 18")]
pub fn remove_get_in_anims(ped: GPtr<u8>) {
    blend_out_associations(rd::<GPtr<u8>>(ped, 0x18), 0x10, 0x10);
}

/// `CCarEnterExit::RemoveCarSitAnim(const CPed*)` (0x64F680): blends out the associations with flag
/// 0x1000, then `BlendAnimation(clump, ped->m_nAnimGroup (+0x4D4), 3 (car sit), 1000.0)`. The first
/// flags argument is computed by the .HOODLUM fragment at 0x40A1FD as `~[0x1560910] ^ [0x4D4314]`
/// (0x1000 with the image bytes); it is read the same way here. EAX is BlendAnimation's result.
#[hook(addr = 0x64F680, conv = cdecl, name = "CCarEnterExit::RemoveCarSitAnim", expect = "53 8B 5C 24 08")]
pub fn remove_car_sit_anim(ped: GPtr<u8>) -> GPtr<u8> {
    let first_flags = !g::<u32>(0x1560910).read() ^ g::<u32>(0x4D4314).read();
    blend_out_associations(rd::<GPtr<u8>>(ped, 0x18), first_flags, 0x1000);
    blend_animation(
        rd::<GPtr<u8>>(ped, 0x18),
        rd::<u32>(ped, 0x4D4),
        3,
        f32::from_bits(0x447A_0000),
    )
}

pub const HOOKS: &[HookReg] = &[
    HOOK_IS_VEHICLE_HEALTHY,
    HOOK_IS_PED_HEALTHY,
    HOOK_IS_CAR_DOOR_READY,
    HOOK_CAR_HAS_DOOR_TO_CLOSE,
    HOOK_CAR_HAS_DOOR_TO_OPEN,
    HOOK_CAR_HAS_PARTIALLY_OPEN_DOOR,
    HOOK_CAR_HAS_OPENABLE_DOOR,
    HOOK_IS_CAR_QUICK_JACK_POSSIBLE,
    HOOK_COMPUTE_TARGET_DOOR_TO_ENTER_AS_PASSENGER,
    HOOK_COMPUTE_PASSENGER_INDEX_FROM_CAR_DOOR,
    HOOK_COMPUTE_DOOR_FLAG,
    HOOK_COMPUTE_OPPOSITE_DOOR_FLAG,
    HOOK_COMPUTE_TARGET_DOOR_TO_EXIT,
    HOOK_IS_CAR_SLOW_JACK_REQUIRED,
    HOOK_COMPUTE_SLOW_JACKED_PED,
    HOOK_IS_CAR_DOOR_IN_USE,
    HOOK_REMOVE_GET_IN_ANIMS,
    HOOK_REMOVE_CAR_SIT_ANIM,
];

/// This module's globals (replace with a `sa_core::globals! { list GLOBALS; ... }` block when it gets some).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
