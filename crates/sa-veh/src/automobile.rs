//! `CAutomobile` (0x988 bytes), `CQuadBike`, `CMonsterTruck`: the leaf part (getters, door queries,
//! taxi lights, horn counter, suspension reset, wheel width, Teleport). Ported from the 1.0 US bytes;
//! gta-reversed `Automobile.cpp` as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{b, g, model_info, rd, wr, x};

/// `CAutomobile*` (thiscall `this`).
type Car = GPtr<u8>;

/// `CAutomobile::m_doors[door]` (+0x5B8, `CDoor` 0x18 each): `this + (door * 3 + 0xB7) * 8`.
fn door(car: Car, d: u32) -> GPtr<u8> {
    car.byte_add((d.wrapping_mul(3).wrapping_add(0xB7)).wrapping_mul(8) as i32 as isize)
}

orig! {
    /// `CDoor::GetAngleOpenRatio()` (0x6F47E0), float in ST0
    pub fn door_get_angle_open_ratio(d: GPtr<u8>) -> f64 = thiscall 0x6F47E0 => unported;
    /// `CDoor::IsClosed()` (0x6F4800)
    pub fn door_is_closed(d: GPtr<u8>) -> u8 = thiscall 0x6F4800 => unported;
    /// `CDoor::IsFullyOpen()` (0x6F4820)
    pub fn door_is_fully_open(d: GPtr<u8>) -> u8 = thiscall 0x6F4820 => unported;
    /// `CDamageManager::GetDoorStatus(eDoors)` (0x6C2230)
    pub fn dm_get_door_status(dm: GPtr<u8>, d: u32) -> u32 = thiscall 0x6C2230 => unported;
    /// `CDamageManager::SetWheelStatus(eCarWheel, eCarWheelStatus)` (0x6C21A0)
    pub fn dm_set_wheel_status(dm: GPtr<u8>, wheel: u32, status: u32) = thiscall 0x6C21A0 => unported;
    /// virtual `CVehicle::IsDoorMissing(eDoors)` (vtable +0x98)
    pub fn v_is_door_missing(v: GPtr<u8>, d: u32) -> u8 = thiscall vslot 0x98 => unported;
    /// virtual `CVehicle::GetDooorAngleOpenRatio(eDoors)` (vtable +0x78), float in ST0
    pub fn v_get_door_angle_open_ratio(v: GPtr<u8>, d: u32) -> f64 = thiscall vslot 0x78 => unported;
    /// virtual `CVehicle::IsDoorReady(eDoors)` (vtable +0x80)
    pub fn v_is_door_ready(v: GPtr<u8>, d: u32) -> u8 = thiscall vslot 0x80 => unported;
    /// virtual `CVehicle::IsDoorFullyOpen(eDoors)` (vtable +0x88)
    pub fn v_is_door_fully_open(v: GPtr<u8>, d: u32) -> u8 = thiscall vslot 0x88 => unported;
    /// virtual `CVehicle::IsDoorClosed(eDoors)` (vtable +0x90)
    pub fn v_is_door_closed(v: GPtr<u8>, d: u32) -> u8 = thiscall vslot 0x90 => unported;
    /// virtual `CAutomobile::ResetSuspension()` (vtable +0x10C)
    pub fn v_reset_suspension(v: GPtr<u8>) = thiscall vslot 0x10C => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    pub fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    pub fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CPlaceable::SetOrientation(float, float, float)` (0x439A80)
    pub fn set_orientation(e: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x439A80 => unported;
}

/// `CAutomobile::ClearHeliOrientation()` (0x6A2460): `m_fForcedOrientation` = -1.0.
#[hook(addr = 0x6A2460, conv = thiscall, name = "CAutomobile::ClearHeliOrientation", expect = "C7 81 54 09 00 00 00 00 80 BF", this = "CAutomobile")]
pub fn clear_heli_orientation(car: Car) {
    wr::<u32>(car, 0x954, 0xBF80_0000);
}

/// `CAutomobile::SetHeliOrientation(float)` (0x6A2450, middle part at 0x40765C).
#[hook(addr = 0x6A2450, conv = thiscall, name = "CAutomobile::SetHeliOrientation", expect = "8B 44 24 04 90", this = "CAutomobile")]
pub fn set_heli_orientation(car: Car, angle: u32) {
    wr::<u32>(car, 0x954, angle);
}

/// `CAutomobile::IsRoomForPedToLeaveCar(uint32, CVector*)` (0x6A3850): true.
#[hook(addr = 0x6A3850, conv = thiscall, name = "CAutomobile::IsRoomForPedToLeaveCar", expect = "B0 01 C2 08 00", this = "CAutomobile")]
pub fn is_room_for_ped_to_leave_car(_car: Car, _door: u32, _offset: u32) -> u8 {
    1
}

/// `CAutomobile::KnockPedOutCar(eWeaponType, uint16, CPed*)` (0x6A44C0): returns the ped (EAX).
#[hook(addr = 0x6A44C0, conv = thiscall, name = "CAutomobile::KnockPedOutCar", expect = "8B 44 24 0C C2 0C 00", this = "CAutomobile")]
pub fn knock_ped_out_car(_car: Car, _weapon: u32, _door: u32, ped: GPtr<u8>) -> GPtr<u8> {
    ped
}

/// `CAutomobile::GetNumContactWheels()` (0x6A62A0).
#[hook(addr = 0x6A62A0, conv = thiscall, name = "CAutomobile::GetNumContactWheels", expect = "0F B6 81 60 09 00 00", this = "CAutomobile")]
pub fn get_num_contact_wheels(car: Car) -> u32 {
    u32::from(rd::<u8>(car, 0x960))
}

/// `CAutomobile::GetAllWheelsOffGround()` (0x6A2F70): `m_nWheelsOnGround == 0` (AL).
#[hook(addr = 0x6A2F70, conv = thiscall, name = "CAutomobile::GetAllWheelsOffGround", expect = "8A 81 61 09 00 00", this = "CAutomobile")]
pub fn get_all_wheels_off_ground(car: Car) -> u8 {
    b(rd::<u8>(car, 0x961) == 0)
}

/// `CAutomobile::GetHeightAboveRoad()` (0x6A62B0): `m_fFrontHeightAboveRoad` (+0x898).
#[hook(addr = 0x6A62B0, conv = thiscall, name = "CAutomobile::GetHeightAboveRoad", expect = "D9 81 98 08 00 00", this = "CAutomobile")]
pub fn get_height_above_road(car: Car) -> f32 {
    rd::<f32>(car, 0x898)
}

/// `CAutomobile::SetAllTaxiLights(bool)` (0x6A3760, static): `m_sAllTaxiLights` (0xC1BFD0).
#[hook(addr = 0x6A3760, conv = cdecl, name = "CAutomobile::SetAllTaxiLights", expect = "8A 44 24 04 A2 D0 BF C1 00")]
pub fn set_all_taxi_lights(on: u32) {
    g::<u8>(0xC1BFD0).write(on as u8);
}

/// `CAutomobile::SetTaxiLight(bool)` (0x6A3740): bit 0 of +0x868.
#[hook(addr = 0x6A3740, conv = thiscall, name = "CAutomobile::SetTaxiLight", expect = "8A 81 68 08 00 00", this = "CAutomobile")]
pub fn set_taxi_light(car: Car, on: u32) {
    let f = rd::<u8>(car, 0x868);
    wr::<u8>(car, 0x868, f ^ ((f ^ on as u8) & 1));
}

/// `CAutomobile::IsComponentPresent(int32)` (0x6A2250): `m_aCarNodes[comp] != null` (no range check).
#[hook(addr = 0x6A2250, conv = thiscall, name = "CAutomobile::IsComponentPresent", expect = "8B 44 24 04 8B 94 81 48 06 00 00", this = "CAutomobile")]
pub fn is_component_present(car: Car, comp: i32) -> u8 {
    b(rd::<u32>(car, (comp as isize).wrapping_mul(4).wrapping_add(0x648)) != 0)
}

/// `CAutomobile::ReduceHornCounter()` (0x6A29A0): `m_nHornCounter` (+0x514) decremented when > 0.
#[hook(addr = 0x6A29A0, conv = thiscall, name = "CAutomobile::ReduceHornCounter", expect = "8B 81 14 05 00 00", this = "CAutomobile")]
pub fn reduce_horn_counter(car: Car) {
    let c = rd::<u32>(car, 0x514);
    if c != 0 {
        wr::<u32>(car, 0x514, c - 1);
    }
}

/// `CAutomobile::RemoveRefsToVehicle(CEntity*)` (0x6A3030): clears the 4 wheel collision entities
/// (+0x8E8) equal to the entity.
#[hook(addr = 0x6A3030, conv = thiscall, name = "CAutomobile::RemoveRefsToVehicle", expect = "8B 54 24 04 8D 81 E8 08 00 00", this = "CAutomobile")]
pub fn remove_refs_to_vehicle(car: Car, e: u32) {
    for k in 0..4 {
        if rd::<u32>(car, 0x8E8 + 4 * k) == e {
            wr::<u32>(car, 0x8E8 + 4 * k, 0);
        }
    }
}

/// `CAutomobile::ResetSuspension()` (0x6A2AE0): per wheel k: +0x7D4 = 1.0, +0x7F4 = 0, +0x828 = 0,
/// +0x968 = 0 (stores in that order).
#[hook(addr = 0x6A2AE0, conv = thiscall, name = "CAutomobile::ResetSuspension", expect = "8D 81 F4 07 00 00", this = "CAutomobile")]
pub fn reset_suspension(car: Car) {
    for k in 0..4 {
        let o = 0x7F4 + 4 * k;
        wr::<u32>(car, o - 0x20, 0x3F80_0000);
        wr::<u32>(car, o, 0);
        wr::<u32>(car, o + 0x34, 0);
        wr::<u32>(car, o + 0x174, 0);
    }
}

/// `CAutomobile::GetDooorAngleOpenRatio(eDoors)` (0x6A2270): `CDoor::GetAngleOpenRatio` of the door
/// (ST0 passed through).
#[hook(addr = 0x6A2270, conv = thiscall, name = "CAutomobile::GetDooorAngleOpenRatio", expect = "8B 44 24 04 8D 84 40 B7 00 00 00", this = "CAutomobile")]
pub fn get_door_angle_open_ratio(car: Car, d: u32) -> X87 {
    X87::from_f64_exact(door_get_angle_open_ratio(door(car, d)))
}

/// `CAutomobile::IsDoorClosed(eDoors)` (0x6A2310): `CDoor::IsClosed` (`setne al`).
#[hook(addr = 0x6A2310, conv = thiscall, name = "CAutomobile::IsDoorClosed", expect = "8B 44 24 04 8D 84 40 B7 00 00 00", this = "CAutomobile")]
pub fn is_door_closed(car: Car, d: u32) -> u8 {
    b(door_is_closed(door(car, d)) != 0)
}

/// `CAutomobile::IsDoorMissing(eDoors)` (0x6A2330): damage manager door status == 4 (AL compare).
#[hook(addr = 0x6A2330, conv = thiscall, name = "CAutomobile::IsDoorMissing", expect = "8B 44 24 04 50", this = "CAutomobile")]
pub fn is_door_missing(car: Car, d: u32) -> u8 {
    b(dm_get_door_status(car.byte_add(0x5A0), d) as u8 == 4)
}

/// `CAutomobile::IsDoorReady(eDoors)` (0x6A2290): closed, or (virtual) missing.
#[hook(addr = 0x6A2290, conv = thiscall, name = "CAutomobile::IsDoorReady", expect = "56 57 8B 7C 24 0C", this = "CAutomobile")]
pub fn is_door_ready(car: Car, d: u32) -> u8 {
    if door_is_closed(door(car, d)) != 0 {
        return 1;
    }
    if v_is_door_missing(car, d) != 0 { 1 } else { 0 }
}

/// `CAutomobile::IsDoorFullyOpen(eDoors)` (0x6A22D0): fully open, or (virtual) missing.
#[hook(addr = 0x6A22D0, conv = thiscall, name = "CAutomobile::IsDoorFullyOpen", expect = "56 57 8B 7C 24 0C", this = "CAutomobile")]
pub fn is_door_fully_open(car: Car, d: u32) -> u8 {
    if door_is_fully_open(door(car, d)) != 0 {
        return 1;
    }
    if v_is_door_missing(car, d) != 0 { 1 } else { 0 }
}

/// The `...U32` door-flag variants: component 8 -> door 3, 9 -> 5, 10 -> 2, 11 -> 4 (byte table
/// 0x6A63CC and its copies), 18 -> the constant case, other -> the default.
fn door_of_flag(flag: u32) -> Result<u32, bool> {
    match flag.wrapping_sub(8) {
        0 => Ok(3),
        1 => Ok(5),
        2 => Ok(2),
        3 => Ok(4),
        10 => Err(true),
        _ => Err(false),
    }
}

/// `CAutomobile::IsDoorReadyU32(uint32)` (0x6A6350): tail call of the virtual IsDoorReady; flag 18
/// true, others false.
#[hook(addr = 0x6A6350, conv = thiscall, name = "CAutomobile::IsDoorReadyU32", expect = "8B 54 24 04 83 C2 F8", this = "CAutomobile")]
pub fn is_door_ready_u32(car: Car, flag: u32) -> u8 {
    match door_of_flag(flag) {
        Ok(d) => v_is_door_ready(car, d),
        Err(c) => b(c),
    }
}

/// `CAutomobile::IsDoorFullyOpenU32(uint32)` (0x6A63E0): flag 18 and others false.
#[hook(addr = 0x6A63E0, conv = thiscall, name = "CAutomobile::IsDoorFullyOpenU32", expect = "8B 54 24 04 83 C2 F8", this = "CAutomobile")]
pub fn is_door_fully_open_u32(car: Car, flag: u32) -> u8 {
    match door_of_flag(flag) {
        Ok(d) => v_is_door_fully_open(car, d),
        Err(_) => 0,
    }
}

/// `CAutomobile::IsDoorClosedU32(uint32)` (0x6A6470): flag 18 and others false.
#[hook(addr = 0x6A6470, conv = thiscall, name = "CAutomobile::IsDoorClosedU32", expect = "8B 54 24 04 83 C2 F8", this = "CAutomobile")]
pub fn is_door_closed_u32(car: Car, flag: u32) -> u8 {
    match door_of_flag(flag) {
        Ok(d) => v_is_door_closed(car, d),
        Err(_) => 0,
    }
}

/// `CAutomobile::IsDoorMissingU32(uint32)` (0x6A6500): flag 18 true, others false.
#[hook(addr = 0x6A6500, conv = thiscall, name = "CAutomobile::IsDoorMissingU32", expect = "8B 54 24 04 83 C2 F8", this = "CAutomobile")]
pub fn is_door_missing_u32(car: Car, flag: u32) -> u8 {
    match door_of_flag(flag) {
        Ok(d) => v_is_door_missing(car, d),
        Err(c) => b(c),
    }
}

/// `CAutomobile::GetDooorAngleOpenRatioU32(uint32)` (0x6A62C0): the virtual ratio of the mapped door,
/// else 0.0 (0x858B50).
#[hook(addr = 0x6A62C0, conv = thiscall, name = "CAutomobile::GetDooorAngleOpenRatioU32", expect = "8B 44 24 04 D9 05 50 8B 85 00", this = "CAutomobile")]
pub fn get_door_angle_open_ratio_u32(car: Car, flag: u32) -> X87 {
    match door_of_flag(flag) {
        Ok(d) => X87::from_f64_exact(v_get_door_angle_open_ratio(car, d)),
        Err(_) => x(g::<f32>(0x858B50).read()),
    }
}

/// `CAutomobile::IsOpenTopCar()` (0x6A2350): model 480 (Comet) or 439 (Stallion) with both
/// +0x438 and +0x439 (extras) non-zero.
#[hook(addr = 0x6A2350, conv = thiscall, name = "CAutomobile::IsOpenTopCar", expect = "66 8B 41 22 66 3D E0 01", this = "CAutomobile")]
pub fn is_open_top_car(car: Car) -> u8 {
    let m = rd::<u16>(car, 0x22);
    let extras = rd::<u8>(car, 0x438) != 0 && rd::<u8>(car, 0x439) != 0;
    b((m == 0x1E0 || m == 0x1B7) && extras)
}

/// `CAutomobile::FindWheelWidth(bool rear)` (0x6A6090): the model's front (+0x40) or rear (+0x44)
/// wheel scale times 0.35714287 (0x87111C), times the handling's wheel-width flags (front: model
/// flags bits 8..11 of +0x38C's second byte = 0.65 / 0.8 / 1.1 / 1.25; rear: bits 12..15, bit 15
/// = 1.25), times 1.5 (0x8710CC) for model 571 (Kart). ST0, not stored.
#[hook(addr = 0x6A6090, conv = thiscall, name = "CAutomobile::FindWheelWidth", expect = "8A 54 24 04 84 D2", this = "CAutomobile")]
pub fn find_wheel_width(car: Car, rear: u32) -> X87 {
    let rear = rear as u8 != 0;
    let model = rd::<i16>(car, 0x22);
    let mi = model_info(i32::from(model));
    let mut v = x(rd::<f32>(mi, if rear { 0x44 } else { 0x40 })).fmul_m(g::<f32>(0x87111C).read());
    let ch = (rd::<u32>(car, 0x38C) >> 8) as u8;
    let f = |a: usize| g::<f32>(a).read();
    if rear {
        if ch & 0xF0 != 0 {
            if ch & 0x10 != 0 {
                v = v.fmul_m(f(0x858F50));
            } else if ch & 0x20 != 0 {
                v = v.fmul_m(f(0x858C98));
            } else if ch & 0x40 != 0 {
                v = v.fmul_m(f(0x858F14));
            } else if ch & 0x80 != 0 {
                v = v.fmul_m(f(0x8595F0));
            }
        }
    } else if ch & 0x0F != 0 {
        if ch & 0x01 != 0 {
            v = v.fmul_m(f(0x858F50));
        } else if ch & 0x02 != 0 {
            v = v.fmul_m(f(0x858C98));
        } else if ch & 0x04 != 0 {
            v = v.fmul_m(f(0x858F14));
        } else if ch & 0x08 != 0 {
            v = v.fmul_m(f(0x8595F0));
        }
    }
    if model == 0x23B {
        v = v.fmul_m(f(0x8710CC));
    }
    v
}

/// `CAutomobile::FixTyre(eWheels)` (0x6A3580, middle part at 0x40A498): wheel status 0.
#[hook(addr = 0x6A3580, conv = thiscall, name = "CAutomobile::FixTyre", expect = "8B 44 24 04 6A 00", this = "CAutomobile")]
pub fn fix_tyre(car: Car, wheel: u32) {
    dm_set_wheel_status(car.byte_add(0x5A0), wheel, 0);
}

/// `CAutomobile::Teleport(CVector, bool resetRotation)` (0x6A9CA0): remove from the world, set the
/// position (matrix +0x30 or the simple transform +4), optionally `SetOrientation(0, 0, 0)`, zero move
/// and turn speed, the virtual ResetSuspension, add back. (The original zeroes its own argument slots
/// on the way; callers do not reuse them.)
#[hook(addr = 0x6A9CA0, conv = thiscall, name = "CAutomobile::Teleport", expect = "56 8B F1 56 E8 D7 95 EB FF", this = "CAutomobile")]
pub fn teleport(car: Car, px: u32, py: u32, pz: u32, reset_rotation: u32) {
    world_remove(car);
    let m = rd::<GPtr<u8>>(car, 0x14);
    if !m.is_null() {
        wr::<u32>(m, 0x30, px);
        wr::<u32>(rd::<GPtr<u8>>(car, 0x14), 0x34, py);
        wr::<u32>(rd::<GPtr<u8>>(car, 0x14), 0x38, pz);
    } else {
        wr::<u32>(car, 4, px);
        wr::<u32>(car, 8, py);
        wr::<u32>(car, 0xC, pz);
    }
    if reset_rotation as u8 != 0 {
        set_orientation(car, 0.0, 0.0, 0.0);
    }
    for o in [0x44, 0x48, 0x4C, 0x50, 0x54, 0x58] {
        wr::<u32>(car, o, 0);
    }
    v_reset_suspension(car);
    world_add(car);
}

/// `CQuadBike::GetRideAnimData()` (0x6CDC90): `&m_rideAnimData` (+0x98C).
#[hook(addr = 0x6CDC90, conv = thiscall, name = "CQuadBike::GetRideAnimData", expect = "8D 81 8C 09 00 00", this = "CQuadBike")]
pub fn quad_get_ride_anim_data(q: GPtr<u8>) -> GPtr<u8> {
    q.byte_add(0x98C)
}

/// `CMonsterTruck::SetUpWheelColModel(CColModel*)` (0x6C7D20): false.
#[hook(addr = 0x6C7D20, conv = thiscall, name = "CMonsterTruck::SetUpWheelColModel", expect = "32 C0 C2 04 00", this = "CMonsterTruck")]
pub fn monster_set_up_wheel_col_model(_m: GPtr<u8>, _col: u32) -> u8 {
    0
}

/// `CMonsterTruck::BurstTyre(uint8, bool)` (0x6C7D30): false.
#[hook(addr = 0x6C7D30, conv = thiscall, name = "CMonsterTruck::BurstTyre", expect = "32 C0 C2 08 00", this = "CMonsterTruck")]
pub fn monster_burst_tyre(_m: GPtr<u8>, _tyre: u32, _effects: u32) -> u8 {
    0
}

pub const HOOKS: &[HookReg] = &[
    HOOK_CLEAR_HELI_ORIENTATION,
    HOOK_SET_HELI_ORIENTATION,
    HOOK_IS_ROOM_FOR_PED_TO_LEAVE_CAR,
    HOOK_KNOCK_PED_OUT_CAR,
    HOOK_GET_NUM_CONTACT_WHEELS,
    HOOK_GET_ALL_WHEELS_OFF_GROUND,
    HOOK_GET_HEIGHT_ABOVE_ROAD,
    HOOK_SET_ALL_TAXI_LIGHTS,
    HOOK_SET_TAXI_LIGHT,
    HOOK_IS_COMPONENT_PRESENT,
    HOOK_REDUCE_HORN_COUNTER,
    HOOK_REMOVE_REFS_TO_VEHICLE,
    HOOK_RESET_SUSPENSION,
    HOOK_GET_DOOR_ANGLE_OPEN_RATIO,
    HOOK_IS_DOOR_CLOSED,
    HOOK_IS_DOOR_MISSING,
    HOOK_IS_DOOR_READY,
    HOOK_IS_DOOR_FULLY_OPEN,
    HOOK_IS_DOOR_READY_U32,
    HOOK_IS_DOOR_FULLY_OPEN_U32,
    HOOK_IS_DOOR_CLOSED_U32,
    HOOK_IS_DOOR_MISSING_U32,
    HOOK_GET_DOOR_ANGLE_OPEN_RATIO_U32,
    HOOK_IS_OPEN_TOP_CAR,
    HOOK_FIND_WHEEL_WIDTH,
    HOOK_FIX_TYRE,
    HOOK_TELEPORT,
    HOOK_QUAD_GET_RIDE_ANIM_DATA,
    HOOK_MONSTER_SET_UP_WHEEL_COL_MODEL,
    HOOK_MONSTER_BURST_TYRE,
];

/// This module's globals (replace with a `sa_core::globals! { list GLOBALS; ... }` block when it gets some).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
