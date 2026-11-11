//! `CBike` (0x814 bytes) and `CBmx`: the leaf part (virtual overrides that return constants or one
//! field, horn counter, Fix, RemoveRefsToVehicle). Ported from the 1.0 US bytes; gta-reversed
//! `Bike.cpp` / `Bmx.cpp` as reading aids. `bool` results set only AL (callers test AL).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{b, g, rd, wr};

/// `CBike*` (thiscall `this`).
type Bike = GPtr<u8>;

orig! {
    /// virtual `CVehicle::PlayCarHorn()` (vtable +0xD8)
    pub fn v_play_car_horn(v: GPtr<u8>) = thiscall vslot 0xD8 => unported;
}

/// `CBike::GetNumContactWheels()` (0x6B58A0).
#[hook(addr = 0x6B58A0, conv = thiscall, name = "CBike::GetNumContactWheels", expect = "0F B6 81 04 08 00 00", this = "CBike")]
pub fn get_num_contact_wheels(bk: Bike) -> u32 {
    u32::from(rd::<u8>(bk, 0x804))
}

/// `CBike::GetAllWheelsOffGround()` (0x6B6790): +0x805 == 0.
#[hook(addr = 0x6B6790, conv = thiscall, name = "CBike::GetAllWheelsOffGround", expect = "8A 81 05 08 00 00", this = "CBike")]
pub fn get_all_wheels_off_ground(bk: Bike) -> u8 {
    b(rd::<u8>(bk, 0x805) == 0)
}

/// `CBike::GetHeightAboveRoad()` (0x6B58B0): +0x790.
#[hook(addr = 0x6B58B0, conv = thiscall, name = "CBike::GetHeightAboveRoad", expect = "D9 81 90 07 00 00", this = "CBike")]
pub fn get_height_above_road(bk: Bike) -> f32 {
    rd::<f32>(bk, 0x790)
}

/// `CBike::GetRideAnimData()` (0x6B58C0): `&m_rideAnimData` (+0x640).
#[hook(addr = 0x6B58C0, conv = thiscall, name = "CBike::GetRideAnimData", expect = "8D 81 40 06 00 00", this = "CBike")]
pub fn get_ride_anim_data(bk: Bike) -> GPtr<u8> {
    bk.byte_add(0x640)
}

/// `CBike::IsDoorReady` (0x6B58E0): true (AL).
#[hook(addr = 0x6B58E0, conv = thiscall, name = "CBike::IsDoorReady", expect = "B0 01 C2 04 00", this = "CBike")]
pub fn is_door_ready(_bk: Bike, _a: u32) -> u8 {
    1
}

/// `CBike::IsDoorFullyOpen` (0x6B58F0): false (AL).
#[hook(addr = 0x6B58F0, conv = thiscall, name = "CBike::IsDoorFullyOpen", expect = "32 C0 C2 04 00", this = "CBike")]
pub fn is_door_fully_open(_bk: Bike, _a: u32) -> u8 {
    0
}

/// `CBike::IsDoorClosed` (0x6B5900): false (AL).
#[hook(addr = 0x6B5900, conv = thiscall, name = "CBike::IsDoorClosed", expect = "32 C0 C2 04 00", this = "CBike")]
pub fn is_door_closed(_bk: Bike, _a: u32) -> u8 {
    0
}

/// `CBike::IsDoorMissing` (0x6B5910): true (AL).
#[hook(addr = 0x6B5910, conv = thiscall, name = "CBike::IsDoorMissing", expect = "B0 01 C2 04 00", this = "CBike")]
pub fn is_door_missing(_bk: Bike, _a: u32) -> u8 {
    1
}

/// `CBike::IsDoorReadyU32` (0x6B5920): true (AL).
#[hook(addr = 0x6B5920, conv = thiscall, name = "CBike::IsDoorReadyU32", expect = "B0 01 C2 04 00", this = "CBike")]
pub fn is_door_ready_u32(_bk: Bike, _a: u32) -> u8 {
    1
}

/// `CBike::IsDoorFullyOpenU32` (0x6B5930): false (AL).
#[hook(addr = 0x6B5930, conv = thiscall, name = "CBike::IsDoorFullyOpenU32", expect = "32 C0 C2 04 00", this = "CBike")]
pub fn is_door_fully_open_u32(_bk: Bike, _a: u32) -> u8 {
    0
}

/// `CBike::IsDoorClosedU32` (0x6B5940): false (AL).
#[hook(addr = 0x6B5940, conv = thiscall, name = "CBike::IsDoorClosedU32", expect = "32 C0 C2 04 00", this = "CBike")]
pub fn is_door_closed_u32(_bk: Bike, _a: u32) -> u8 {
    0
}

/// `CBike::IsDoorMissingU32` (0x6B5950): true (AL).
#[hook(addr = 0x6B5950, conv = thiscall, name = "CBike::IsDoorMissingU32", expect = "B0 01 C2 04 00", this = "CBike")]
pub fn is_door_missing_u32(_bk: Bike, _a: u32) -> u8 {
    1
}

/// `CBike::IsRoomForPedToLeaveCar(uint32, CVector*)` (0x6B7270): true.
#[hook(addr = 0x6B7270, conv = thiscall, name = "CBike::IsRoomForPedToLeaveCar", expect = "B0 01 C2 08 00", this = "CBike")]
pub fn is_room_for_ped_to_leave_car(_bk: Bike, _door: u32, _offset: u32) -> u8 {
    1
}

/// `CBike::KnockOffRider(eWeaponType, uint8, CPed*, bool)` (0x6B5F40): returns the ped (EAX).
#[hook(addr = 0x6B5F40, conv = thiscall, name = "CBike::KnockOffRider", expect = "8B 44 24 0C C2 10 00", this = "CBike")]
pub fn knock_off_rider(_bk: Bike, _weapon: u32, _dir: u32, ped: GPtr<u8>, _gun: u32) -> GPtr<u8> {
    ped
}

/// `CBike::FindWheelWidth(bool)` (0x6B8940): 0.15 (0x858FCC) in ST0.
#[hook(addr = 0x6B8940, conv = thiscall, name = "CBike::FindWheelWidth", expect = "D9 05 CC 8F 85 00", this = "CBike")]
pub fn find_wheel_width(_bk: Bike, _rear: u32) -> f32 {
    g::<f32>(0x858FCC).read()
}

/// `CBike::IsComponentPresent(int32)` (0x6B59E0): `m_aBikeNodes[comp] != null` (+0x5A0, no range check).
#[hook(addr = 0x6B59E0, conv = thiscall, name = "CBike::IsComponentPresent", expect = "8B 44 24 04 8B 94 81 A0 05 00 00", this = "CBike")]
pub fn is_component_present(bk: Bike, comp: i32) -> u8 {
    b(rd::<u32>(bk, (comp as isize).wrapping_mul(4).wrapping_add(0x5A0)) != 0)
}

/// `CBike::PlayHornIfNecessary()` (0x6B7130): when +0x3DB has bit 0 or 1, tail call the virtual
/// PlayCarHorn.
#[hook(addr = 0x6B7130, conv = thiscall, name = "CBike::PlayHornIfNecessary", expect = "F6 81 DB 03 00 00 03", this = "CBike")]
pub fn play_horn_if_necessary(bk: Bike) {
    if rd::<u8>(bk, 0x3DB) & 3 != 0 {
        v_play_car_horn(bk);
    }
}

/// `CBike::ReduceHornCounter()` (0x6B5F90): `m_nHornCounter` (+0x514) decremented when > 0.
#[hook(addr = 0x6B5F90, conv = thiscall, name = "CBike::ReduceHornCounter", expect = "8B 81 14 05 00 00", this = "CBike")]
pub fn reduce_horn_counter(bk: Bike) {
    let c = rd::<u32>(bk, 0x514);
    if c != 0 {
        wr::<u32>(bk, 0x514, c - 1);
    }
}

/// `CBike::Fix()` (0x6B7050): clears bit 0 of +0x42A and bit 6 of +0x614, the two wheel statuses
/// +0x65C / +0x65D.
#[hook(addr = 0x6B7050, conv = thiscall, name = "CBike::Fix", expect = "80 A1 2A 04 00 00 FE", this = "CBike")]
pub fn fix(bk: Bike) {
    wr::<u8>(bk, 0x42A, rd::<u8>(bk, 0x42A) & 0xFE);
    wr::<u8>(bk, 0x614, rd::<u8>(bk, 0x614) & 0xBF);
    wr::<u8>(bk, 0x65C, 0);
    wr::<u8>(bk, 0x65D, 0);
}

/// `CBike::RemoveRefsToVehicle(CEntity*)` (0x6B67B0): clears the 4 wheel collision entities (+0x7C0)
/// equal to the entity.
#[hook(addr = 0x6B67B0, conv = thiscall, name = "CBike::RemoveRefsToVehicle", expect = "8B 54 24 04 8D 81 C0 07 00 00", this = "CBike")]
pub fn remove_refs_to_vehicle(bk: Bike, e: u32) {
    for k in 0..4 {
        if rd::<u32>(bk, 0x7C0 + 4 * k) == e {
            wr::<u32>(bk, 0x7C0 + 4 * k, 0);
        }
    }
}

/// `CBmx::SetUpWheelColModel(CColModel*)` (0x6BF9B0): false.
#[hook(addr = 0x6BF9B0, conv = thiscall, name = "CBmx::SetUpWheelColModel", expect = "32 C0 C2 04 00", this = "CBmx")]
pub fn bmx_set_up_wheel_col_model(_bk: Bike, _col: u32) -> u8 {
    0
}

/// `CBmx::BurstTyre(uint8, bool)` (0x6BF9C0): false.
#[hook(addr = 0x6BF9C0, conv = thiscall, name = "CBmx::BurstTyre", expect = "32 C0 C2 08 00", this = "CBmx")]
pub fn bmx_burst_tyre(_bk: Bike, _tyre: u32, _effects: u32) -> u8 {
    0
}

/// `CBmx::FindWheelWidth(bool)` (0x6C0550): 0.07 (0x858CA8) in ST0.
#[hook(addr = 0x6C0550, conv = thiscall, name = "CBmx::FindWheelWidth", expect = "D9 05 A8 8C 85 00", this = "CBmx")]
pub fn bmx_find_wheel_width(_bk: Bike, _rear: u32) -> f32 {
    g::<f32>(0x858CA8).read()
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GET_NUM_CONTACT_WHEELS,
    HOOK_GET_ALL_WHEELS_OFF_GROUND,
    HOOK_GET_HEIGHT_ABOVE_ROAD,
    HOOK_GET_RIDE_ANIM_DATA,
    HOOK_IS_DOOR_READY,
    HOOK_IS_DOOR_FULLY_OPEN,
    HOOK_IS_DOOR_CLOSED,
    HOOK_IS_DOOR_MISSING,
    HOOK_IS_DOOR_READY_U32,
    HOOK_IS_DOOR_FULLY_OPEN_U32,
    HOOK_IS_DOOR_CLOSED_U32,
    HOOK_IS_DOOR_MISSING_U32,
    HOOK_IS_ROOM_FOR_PED_TO_LEAVE_CAR,
    HOOK_KNOCK_OFF_RIDER,
    HOOK_FIND_WHEEL_WIDTH,
    HOOK_IS_COMPONENT_PRESENT,
    HOOK_PLAY_HORN_IF_NECESSARY,
    HOOK_REDUCE_HORN_COUNTER,
    HOOK_FIX,
    HOOK_REMOVE_REFS_TO_VEHICLE,
    HOOK_BMX_SET_UP_WHEEL_COL_MODEL,
    HOOK_BMX_BURST_TYRE,
    HOOK_BMX_FIND_WHEEL_WIDTH,
];

/// This module's globals (replace with a `sa_core::globals! { list GLOBALS; ... }` block when it gets some).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
