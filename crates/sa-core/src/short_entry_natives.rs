//! STD-8: phase B natives of game functions whose 1.0 US entry is too short to hook (fewer than five bytes before
//! the `ret`), ported from the bytes.

/// `CCranes::IsThisCarBeingTargettedByAnyCrane(CVehicle*)` (0x6F4000, cdecl): `xor al, al; ret` (false in AL).
#[crate::native(addr = 0x6F4000, conv = cdecl, name = "CCranes::IsThisCarBeingTargettedByAnyCrane")]
pub fn cranes_is_this_car_being_targetted_by_any_crane(_veh: u32) -> u8 {
    0
}

/// `CCranes::IsThisCarBeingCarriedByAnyCrane(CVehicle*)` (0x6F3FF0, cdecl): `xor al, al; ret` (false in AL). STD-14.
#[crate::native(addr = 0x6F3FF0, conv = cdecl, name = "CCranes::IsThisCarBeingCarriedByAnyCrane")]
pub fn cranes_is_this_car_being_carried_by_any_crane(_veh: u32) -> u8 {
    0
}

/// `CCamera::Get_Just_Switched_Status()` (0x50AE10, thiscall): `mov al, [ecx + 0x2A]; ret`.
#[crate::native(addr = 0x50AE10, conv = thiscall, name = "CCamera::Get_Just_Switched_Status")]
pub fn camera_get_just_switched_status(cam: crate::mem::GPtr<u8>) -> u8 {
    cam.byte_add(0x2A).read()
}

/// `CBridge::ThisIsABridgeObjectMovingUp(int32 modelIndex)` (0x41BCB0, cdecl): `xor al, al; ret` (false in AL).
#[crate::native(addr = 0x41BCB0, conv = cdecl, name = "CBridge::ThisIsABridgeObjectMovingUp")]
pub fn bridge_this_is_a_bridge_object_moving_up(_model: i32) -> u8 {
    0
}

/// 0x4217B0 (catalogue: missed by Ghidra, data-code-immediate), proposed `identity cast getter` (thiscall,
/// `mov eax, ecx; ret`: returns `this`; only reference `CBike::ProcessEntityCollision`).
#[crate::native(addr = 0x4217B0, conv = thiscall, name = "(missed by ghidra: data-code-immediate)")]
pub fn identity_this_4217b0(this: crate::mem::GPtr<u8>) -> crate::mem::GPtr<u8> {
    this
}

/// `FUN_0047e050` (0x47E050), proposed `CColPoint::GetSurfaceTypeB()` (thiscall, `mov al, [ecx + 0x23]; ret`;
/// only caller `CAutomobile::ProcessCarWheelPair`).
#[crate::native(addr = 0x47E050, conv = thiscall, name = "FUN_0047e050")]
pub fn col_point_get_surface_type_b(cp: crate::mem::GPtr<u8>) -> u8 {
    cp.byte_add(0x23).read()
}

/// `CPedDamageResponseCalculator::~CPedDamageResponseCalculator()` (0x4AD420, thiscall): `ret` (empty).
#[crate::native(addr = 0x4AD420, conv = thiscall, name = "CPedDamageResponseCalculator::~CPedDamageResponseCalculator")]
pub fn ped_damage_response_calculator_dtor(_this: crate::mem::GPtr<u8>) {}

/// `CPed::UpdateStatLeavingVehicle()` (0x5E01B0, thiscall): `ret` (empty).
#[crate::native(addr = 0x5E01B0, conv = thiscall, name = "CPed::UpdateStatLeavingVehicle")]
pub fn ped_update_stat_leaving_vehicle(_this: crate::mem::GPtr<u8>) {}

/// This module's natives.
pub const NATIVES: &[crate::abi::NativeReg] = &[
    NATIVE_CRANES_IS_THIS_CAR_BEING_TARGETTED_BY_ANY_CRANE,
    NATIVE_CRANES_IS_THIS_CAR_BEING_CARRIED_BY_ANY_CRANE,
    NATIVE_CAMERA_GET_JUST_SWITCHED_STATUS,
    NATIVE_BRIDGE_THIS_IS_A_BRIDGE_OBJECT_MOVING_UP,
    NATIVE_IDENTITY_THIS_4217B0,
    NATIVE_COL_POINT_GET_SURFACE_TYPE_B,
    NATIVE_PED_DAMAGE_RESPONSE_CALCULATOR_DTOR,
    NATIVE_PED_UPDATE_STAT_LEAVING_VEHICLE,
];
