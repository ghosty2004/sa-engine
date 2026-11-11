//! Phase B natives (`#[native]`) of short leaves on the in-world and menu paths that phase A cannot hook (bodies that
//! end before the 5-byte entry patch fits, or a bare `jmp` into game `.text`). Collected here by address whatever their subsystem; ported from
//! the 1.0 US bytes.

use sa_core::abi::NativeReg;
use sa_core::mem::GPtr;
use sa_core::native;

/// `List_c::GetNumItems()` (0x4A8EC0, thiscall): `mov eax, [ecx+8]; ret` (the count).
#[native(addr = 0x4A8EC0, conv = thiscall, name = "FUN_004a8ec0")]
pub fn list_get_num_items(this: GPtr<u8>) -> i32 {
    this.byte_add(8).cast::<i32>().read()
}

/// `CCamera::GetFading()` (0x50ADE0, thiscall): `mov al, [ecx+0x51]; ret` (AL only).
#[native(addr = 0x50ADE0, conv = thiscall, name = "CCamera::GetFading")]
pub fn camera_get_fading(this: GPtr<u8>) -> u8 {
    this.byte_add(0x51).read()
}

/// `FUN_00424100(float x, float y)` (0x424100, cdecl; a stubbed area check of the vehicle controls): `xor al, al;
/// ret` (AL only).
#[native(addr = 0x424100, conv = cdecl, name = "FUN_00424100")]
pub fn area_check_stub(_x: u32, _y: u32) -> u8 {
    0
}

/// `CTheScripts::CheckStreamedScriptVersion(RwStream*, const char*)` (0x464FF0, cdecl): `mov al, 1; ret` (AL only).
#[native(addr = 0x464FF0, conv = cdecl, name = "CTheScripts::CheckStreamedScriptVersion")]
pub fn check_streamed_script_version(_stream: GPtr<u8>, _name: GPtr<u8>) -> u8 {
    1
}

/// `FUN_0041bd10` (0x41BD10, thiscall): `lea eax, [ecx+0x10]; ret`.
#[native(addr = 0x41BD10, conv = thiscall, name = "FUN_0041bd10")]
pub fn this_plus_10_41bd10(this: GPtr<u8>) -> GPtr<u8> {
    this.byte_add(0x10)
}

/// `FUN_00411ba0` (0x411BA0, thiscall): `lea eax, [ecx+0x10]; ret`.
#[native(addr = 0x411BA0, conv = thiscall, name = "FUN_00411ba0")]
pub fn this_plus_10_411ba0(this: GPtr<u8>) -> GPtr<u8> {
    this.byte_add(0x10)
}

/// `CRealTimeShadow::GetShadowCamera()` (0x7059E0, thiscall): `lea eax, [ecx+8]; ret`.
#[native(addr = 0x7059E0, conv = thiscall, name = "CRealTimeShadow::GetShadowCamera")]
pub fn real_time_shadow_get_shadow_camera(this: GPtr<u8>) -> GPtr<u8> {
    this.byte_add(8)
}

/// `CVehicle::GetRideAnimData()` (0x6A6280, thiscall): `xor eax, eax; ret` (NULL).
#[native(addr = 0x6A6280, conv = thiscall, name = "CVehicle::GetRideAnimData")]
pub fn vehicle_get_ride_anim_data(_this: GPtr<u8>) -> GPtr<u8> {
    GPtr::NULL
}

/// `CMovingThings::Render_BeforeClouds()` (0x7178F0, cdecl): `jmp CPlaneTrails::Render` (0x7173A0). A 5-byte `jmp` into
/// game `.text`, which the phase A hook table only accepts for `.HOODLUM` stubs.
#[native(addr = 0x7178F0, conv = cdecl, name = "CMovingThings::Render_BeforeClouds")]
pub fn moving_things_render_before_clouds() {
    crate::mvpd_r3::plane_trails_render();
}

/// `FUN_0041bce0(float)` (0x41BCE0, cdecl): `fld` of the argument, FSIN, result in ST0 (53 of the CPU's 64 significand
/// bits, see `sa_math::x87`). Hookable by size, but code outside the game `.text` reads its entry bytes as data
/// (catalog/code-data-reads.csv), so phase A leaves it alone.
#[native(addr = 0x41BCE0, conv = cdecl, name = "FUN_0041bce0")]
pub fn x87_sin(a: u32) -> f64 {
    sa_math::x87::X87::from_f32(f32::from_bits(a)).fsin().to_f64()
}

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[
    NATIVE_LIST_GET_NUM_ITEMS,
    NATIVE_CAMERA_GET_FADING,
    NATIVE_AREA_CHECK_STUB,
    NATIVE_CHECK_STREAMED_SCRIPT_VERSION,
    NATIVE_THIS_PLUS_10_41BD10,
    NATIVE_THIS_PLUS_10_411BA0,
    NATIVE_REAL_TIME_SHADOW_GET_SHADOW_CAMERA,
    NATIVE_VEHICLE_GET_RIDE_ANIM_DATA,
    NATIVE_MOVING_THINGS_RENDER_BEFORE_CLOUDS,
    NATIVE_X87_SIN,
];
