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

/// This module's natives.
pub const NATIVES: &[crate::abi::NativeReg] = &[
    NATIVE_CRANES_IS_THIS_CAR_BEING_TARGETTED_BY_ANY_CRANE,
    NATIVE_CRANES_IS_THIS_CAR_BEING_CARRIED_BY_ANY_CRANE,
    NATIVE_CAMERA_GET_JUST_SWITCHED_STATUS,
];
