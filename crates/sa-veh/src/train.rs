//! CTrain (PORT-VEH). Ported from the 1.0 US bytes; gta-reversed as a reading aid.
//!
//! Fields used: `+0x3D0` cruise speed byte (CAutoPilot), `+0x5A4` train speed, `+0x5B8` train flags
//! (bit 5 mission train, bit 6 direction / side of the station), `+0x5D0` previous carriage,
//! `+0x5D4` next carriage.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{ftol32, rd, wr, x};

/// `CTrain*`.
type Train = GPtr<u8>;

sa_core::globals! {
    list GLOBALS;
    /// `CTrain::bDisableRandomTrains`
    pub static DISABLE_RANDOM_TRAINS @ 0xC38010: u8 = 0;
}

orig! {
    /// `CWorld::Remove(CEntity*)` (0x563280)
    pub fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// Virtual slot 0 (`scalar deleting destructor`)
    pub fn entity_vdelete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
}

const NEXT: isize = 0x5D4;
const PREV: isize = 0x5D0;

/// `CTrain::DisableRandomTrains(bool)` (0x6F5DB0).
#[hook(addr = 0x6F5DB0, conv = cdecl, name = "CTrain::DisableRandomTrains", expect = "8A 44 24 04 A2 10 80 C3 00")]
pub fn disable_random_trains(disable: u32) {
    DISABLE_RANDOM_TRAINS.set(disable as u8);
}

/// `CTrain::FindNumCarriagesPulled()` (0x6F5CD0): carriages after `this` along `+0x5D4`.
#[hook(addr = 0x6F5CD0, conv = thiscall, name = "CTrain::FindNumCarriagesPulled", expect = "8B 89 D4 05 00 00", this = "CTrain")]
pub fn find_num_carriages_pulled(t: Train) -> u32 {
    let mut n = 0u32;
    let mut c = rd::<GPtr<u8>>(t, NEXT);
    while !c.is_null() {
        c = rd::<GPtr<u8>>(c, NEXT);
        n = n.wrapping_add(1);
    }
    n
}

/// `CTrain::RemoveOneMissionTrain(CTrain*)` (0x6F5DC0): for every carriage from `train` on (the next
/// pointer read first), `CWorld::Remove` then the deleting destructor (`vtable[0](1)`).
#[hook(addr = 0x6F5DC0, conv = cdecl, name = "CTrain::RemoveOneMissionTrain", expect = "56 8B 74 24 08")]
pub fn remove_one_mission_train(train: Train) {
    let mut c = train;
    while !c.is_null() {
        let next = rd::<GPtr<u8>>(c, NEXT);
        world_remove(c);
        entity_vdelete(c, 1);
        c = next;
    }
}

/// `CTrain::ReleaseOneMissionTrain(CTrain*)` (0x6F5DF0): clears flag bit 5 (`& 0xDF` of `+0x5B8`) of
/// every carriage from `train` on.
#[hook(addr = 0x6F5DF0, conv = cdecl, name = "CTrain::ReleaseOneMissionTrain", expect = "8B 44 24 04 85 C0")]
pub fn release_one_mission_train(train: Train) {
    let mut c = train;
    while !c.is_null() {
        wr::<u8>(c, 0x5B8, rd::<u8>(c, 0x5B8) & 0xDF);
        c = rd::<GPtr<u8>>(c, NEXT);
    }
}

/// `CTrain::SetTrainSpeed(CTrain*, float)` (0x6F5E20): `+0x5A4 = speed * 0.02`, negated (a second
/// store) when flag bit 6 is clear.
#[hook(addr = 0x6F5E20, conv = cdecl, name = "CTrain::SetTrainSpeed", expect = "D9 44 24 08 8B 44 24 04")]
pub fn set_train_speed(train: Train, speed: f32) {
    let v = x(speed).fmul_m(f32::from_bits(0x3CA3_D70A));
    wr::<f32>(train, 0x5A4, v.to_f32());
    if rd::<u8>(train, 0x5B8) & 0x40 == 0 {
        wr::<f32>(train, 0x5A4, v.fchs().to_f32());
    }
}

/// `CTrain::SetTrainCruiseSpeed(CTrain*, float)` (0x6F5E50): cruise speed byte `+0x3D0 = _ftol(speed)`.
#[hook(addr = 0x6F5E50, conv = cdecl, name = "CTrain::SetTrainCruiseSpeed", expect = "D9 44 24 08 E8 E7 BC 12 00")]
pub fn set_train_cruise_speed(train: Train, speed: f32) {
    wr::<u8>(train, 0x3D0, ftol32(x(speed)) as u8);
}

/// `CTrain::FindCaboose(CTrain*)` (0x6F5E70): the last carriage along `+0x5D4`.
#[hook(addr = 0x6F5E70, conv = cdecl, name = "CTrain::FindCaboose", expect = "8B 44 24 04 8B 88 D4 05 00 00")]
pub fn find_caboose(train: Train) -> GPtr<u8> {
    let mut t = train;
    loop {
        let n = rd::<GPtr<u8>>(t, NEXT);
        if n.is_null() {
            return t;
        }
        t = n;
    }
}

/// `CTrain::FindEngine(CTrain*)` (0x6F5E90): the first carriage along `+0x5D0`.
#[hook(addr = 0x6F5E90, conv = cdecl, name = "CTrain::FindEngine", expect = "8B 44 24 04 8B 88 D0 05 00 00")]
pub fn find_engine(train: Train) -> GPtr<u8> {
    let mut t = train;
    loop {
        let n = rd::<GPtr<u8>>(t, PREV);
        if n.is_null() {
            return t;
        }
        t = n;
    }
}

/// `CTrain::FindCarriage(CTrain*, uint8)` (0x6F5EB0): `n` steps along `+0x5D4` (8-bit counter),
/// null if the chain ends first.
#[hook(addr = 0x6F5EB0, conv = cdecl, name = "CTrain::FindCarriage", expect = "8A 54 24 08 8B 44 24 04")]
pub fn find_carriage(train: Train, n: u32) -> GPtr<u8> {
    let n = n as u8;
    let mut t = train;
    let mut k = 0u8;
    while k < n {
        t = rd::<GPtr<u8>>(t, NEXT);
        if t.is_null() {
            return GPtr::NULL;
        }
        k = k.wrapping_add(1);
    }
    t
}

/// `CTrain::FindSideStationIsOn()` (0x6F5EF0): flag bit 6. Only AL is written (EAX's upper bytes are
/// the caller's).
#[hook(addr = 0x6F5EF0, conv = thiscall, name = "CTrain::FindSideStationIsOn", expect = "8A 81 B8 05 00 00", this = "CTrain")]
pub fn find_side_station_is_on(t: Train) -> u8 {
    (rd::<u8>(t, 0x5B8) >> 6) & 1
}

/// `CTrain::IsRoomForPedToLeaveCar(uint32, CVector*)` (0x6F5EE0): always true.
#[hook(addr = 0x6F5EE0, conv = thiscall, name = "CTrain::IsRoomForPedToLeaveCar", expect = "B0 01 C2 08 00", this = "CTrain")]
pub fn is_room_for_ped_to_leave_car(_t: Train, _door: u32, _offset: GPtr<u8>) -> u8 {
    1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_DISABLE_RANDOM_TRAINS,
    HOOK_FIND_NUM_CARRIAGES_PULLED,
    HOOK_REMOVE_ONE_MISSION_TRAIN,
    HOOK_RELEASE_ONE_MISSION_TRAIN,
    HOOK_SET_TRAIN_SPEED,
    HOOK_SET_TRAIN_CRUISE_SPEED,
    HOOK_FIND_CABOOSE,
    HOOK_FIND_ENGINE,
    HOOK_FIND_CARRIAGE,
    HOOK_FIND_SIDE_STATION_IS_ON,
    HOOK_IS_ROOM_FOR_PED_TO_LEAVE_CAR,
];
