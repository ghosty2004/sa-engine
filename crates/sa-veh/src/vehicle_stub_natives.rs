//! PORT-VEH-4: phase B natives (`#[native]`) of the vehicle virtuals whose 1.0 US body is an empty
//! `ret N` or `xor al, al; ret` (shorter than the 5-byte hook patch). Ported from the bytes; the stack
//! arguments are only popped, as the original does.

use sa_core::abi::NativeReg;
use sa_core::native;

/// `CVehicle::OpenDoor(CPed*, int32, eDoors, float, bool)` (0x6B5820): `ret 0x14`.
#[native(addr = 0x6B5820, conv = thiscall, name = "CVehicle::OpenDoor")]
pub fn vehicle_open_door_stub(_this: u32, _a: u32, _b: u32, _c: u32, _d: u32, _e: u32) {}

/// `CVehicle::IsOpenTopCar()` (0x6B5840): `xor al, al; ret` (false in AL).
#[native(addr = 0x6B5840, conv = thiscall, name = "CVehicle::IsOpenTopCar")]
pub fn vehicle_is_open_top_car_stub(_this: u32) -> u8 {
    0
}

/// `CVehicle::BlowUpCarCutSceneNoExtras(bool, bool, bool, bool)` (0x6B5850): `ret 0x10`.
#[native(addr = 0x6B5850, conv = thiscall, name = "CVehicle::BlowUpCarCutSceneNoExtras")]
pub fn vehicle_blow_up_cutscene_stub(_this: u32, _a: u32, _b: u32, _c: u32, _d: u32) {}

/// `CVehicle::BreakTowLink()` (0x6B5890): `xor al, al; ret` (false in AL).
#[native(addr = 0x6B5890, conv = thiscall, name = "CVehicle::BreakTowLink")]
pub fn vehicle_break_tow_link_stub(_this: u32) -> u8 {
    0
}

/// `CBike::ProcessOpenDoor(CPed*, uint32, uint32, uint32, float)` (0x6B58D0): `ret 0x14`.
#[native(addr = 0x6B58D0, conv = thiscall, name = "CBike::ProcessOpenDoor")]
pub fn bike_process_open_door_stub(_this: u32, _a: u32, _b: u32, _c: u32, _d: u32, _e: u32) {}

/// `CBike::dmgDrawCarCollidingParticles(const CVector&, float, eWeaponType)` (0x6B5A00): `ret 0xC`.
#[native(addr = 0x6B5A00, conv = thiscall, name = "CBike::dmgDrawCarCollidingParticles")]
pub fn bike_dmg_draw_particles_stub(_this: u32, _a: u32, _b: u32, _c: u32) {}

/// `CBmx::BlowUpCar(CEntity*, bool)` (0x6C0560): `ret 8`.
#[native(addr = 0x6C0560, conv = thiscall, name = "CBmx::BlowUpCar")]
pub fn bmx_blow_up_car_stub(_this: u32, _a: u32, _b: u32) {}

/// `CVehicle::ProcessControlCollisionCheck(bool)` (0x6D6270): `ret 4`.
#[native(addr = 0x6D6270, conv = thiscall, name = "CVehicle::ProcessControlCollisionCheck")]
pub fn vehicle_process_control_collision_check_stub(_this: u32, _a: u32) {}

/// `CVehicle::ProcessControlInputs(uint8)` (0x6D6280): `ret 4`.
#[native(addr = 0x6D6280, conv = thiscall, name = "CVehicle::ProcessControlInputs")]
pub fn vehicle_process_control_inputs_stub(_this: u32, _a: u32) {}

/// `CVehicle::GetComponentWorldPosition(int32, CVector&)` (0x6D6290): `ret 8`.
#[native(addr = 0x6D6290, conv = thiscall, name = "CVehicle::GetComponentWorldPosition")]
pub fn vehicle_get_component_world_position_stub(_this: u32, _a: u32, _b: u32) {}

/// `CVehicle::RemoveRefsToVehicle(CEntity*)` (0x6D6330): `ret 4`.
#[native(addr = 0x6D6330, conv = thiscall, name = "CVehicle::RemoveRefsToVehicle")]
pub fn vehicle_remove_refs_to_vehicle_stub(_this: u32, _a: u32) {}

/// `CVehicle::BlowUpCar(CEntity*, bool)` (0x6D6340): `ret 8`.
#[native(addr = 0x6D6340, conv = thiscall, name = "CVehicle::BlowUpCar")]
pub fn vehicle_blow_up_car_stub(_this: u32, _a: u32, _b: u32) {}

/// `CVehicle::VehicleDamage(float, eVehicleCollisionComponent, CEntity*, CVector*, CVector*,
/// eWeaponType)` (0x6D63E0): `ret 0x18`.
#[native(addr = 0x6D63E0, conv = thiscall, name = "CVehicle::VehicleDamage")]
pub fn vehicle_vehicle_damage_stub(_this: u32, _a: u32, _b: u32, _c: u32, _d: u32, _e: u32, _f: u32) {}

/// `CBoat::ProcessOpenDoor(CPed*, uint32, uint32, uint32, float)` (0x6F0190): `ret 0x14`.
#[native(addr = 0x6F0190, conv = thiscall, name = "CBoat::ProcessOpenDoor")]
pub fn boat_process_open_door_stub(_this: u32, _a: u32, _b: u32, _c: u32, _d: u32, _e: u32) {}

/// `CTrain::OpenTrainDoor(float)` (0x6F5D80): `ret 4`.
#[native(addr = 0x6F5D80, conv = thiscall, name = "CTrain::OpenTrainDoor")]
pub fn train_open_train_door_stub(_this: u32, _a: u32) {}

/// `CTrain::AddPassenger(CPed*)` (0x6F5D90): `ret 4`.
#[native(addr = 0x6F5D90, conv = thiscall, name = "CTrain::AddPassenger")]
pub fn train_add_passenger_stub(_this: u32, _a: u32) {}

/// `CTrain::RemovePassenger(CPed*)` (0x6F5DA0): `ret 4`.
#[native(addr = 0x6F5DA0, conv = thiscall, name = "CTrain::RemovePassenger")]
pub fn train_remove_passenger_stub(_this: u32, _a: u32) {}

/// This module's natives.
pub const NATIVES: &[NativeReg] = &[
    NATIVE_VEHICLE_OPEN_DOOR_STUB,
    NATIVE_VEHICLE_IS_OPEN_TOP_CAR_STUB,
    NATIVE_VEHICLE_BLOW_UP_CUTSCENE_STUB,
    NATIVE_VEHICLE_BREAK_TOW_LINK_STUB,
    NATIVE_BIKE_PROCESS_OPEN_DOOR_STUB,
    NATIVE_BIKE_DMG_DRAW_PARTICLES_STUB,
    NATIVE_BMX_BLOW_UP_CAR_STUB,
    NATIVE_VEHICLE_PROCESS_CONTROL_COLLISION_CHECK_STUB,
    NATIVE_VEHICLE_PROCESS_CONTROL_INPUTS_STUB,
    NATIVE_VEHICLE_GET_COMPONENT_WORLD_POSITION_STUB,
    NATIVE_VEHICLE_REMOVE_REFS_TO_VEHICLE_STUB,
    NATIVE_VEHICLE_BLOW_UP_CAR_STUB,
    NATIVE_VEHICLE_VEHICLE_DAMAGE_STUB,
    NATIVE_BOAT_PROCESS_OPEN_DOOR_STUB,
    NATIVE_TRAIN_OPEN_TRAIN_DOOR_STUB,
    NATIVE_TRAIN_ADD_PASSENGER_STUB,
    NATIVE_TRAIN_REMOVE_PASSENGER_STUB,
];
