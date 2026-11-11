//! `CVehicle` (0x5A0 bytes), the leaf part: the base-class virtual defaults, the getting-in/out flags,
//! driver checks, lighting bytes, the winch wrappers. Ported from the 1.0 US bytes; gta-reversed
//! `Vehicle.cpp` as a reading aid.
//!
//! Not hooked (3 bytes, the installer needs 5: `TooShort(3)`): `GetRideAnimData`, `IsOpenTopCar`,
//! `BreakTowLink`; their Rust functions stay for phase B.
//!
//! Return registers: the base-class defaults `xor al, al` / `mov al, 1` set only AL (EAX's upper bytes
//! are the caller's); the ports return `u8` and callers test AL.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{b, g, model_info, rd, wr, x};

/// `CVehicle*` (thiscall `this`).
type Veh = GPtr<u8>;

orig! {
    /// `CRopes::FindRope(uint32 id)` (0x556000)
    pub fn ropes_find_rope(id: u32) -> i32 = cdecl 0x556000 => unported;
    /// `CRope::ReleasePickedUpObject()` (0x556030)
    pub fn rope_release_picked_up_object(rope: GPtr<u8>) = thiscall 0x556030 => unported;
    /// `CRope::PickUpObject(CEntity*)` (0x5569C0)
    pub fn rope_pick_up_object(rope: GPtr<u8>, e: GPtr<u8>) = thiscall 0x5569C0 => unported;
    /// `RwTextureDestroy(RwTexture*)` (0x7F3820)
    pub fn rw_texture_destroy(t: GPtr<u8>) -> i32 = cdecl 0x7F3820 => unported;
    /// `CColModel::RemoveCollisionVolumes()` (0x40F9E0)
    pub fn col_model_remove_collision_volumes(cm: GPtr<u8>) = thiscall 0x40F9E0 => unported;
    /// `RpClumpForAllAtomics(RpClump*, RpAtomicCallBack, void*)` (0x749B70)
    pub fn rp_clump_for_all_atomics(clump: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `RpGeometryForAllMaterials(RpGeometry*, RpMaterialCallBack, void*)` (0x74C790)
    pub fn rp_geometry_for_all_materials(geo: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x74C790 => unported;
}

// --- base-class virtual defaults --------------------------------------------------------------

/// `CVehicle::GetRideAnimData()` (0x6A6280): null.
pub fn get_ride_anim_data(_v: Veh) -> GPtr<u8> {
    GPtr::NULL
}

/// `CVehicle::GetDooorAngleOpenRatioU32(uint32)` (0x6B5830): 0.0 in ST0.
#[hook(addr = 0x6B5830, conv = thiscall, name = "CVehicle::GetDooorAngleOpenRatioU32", expect = "D9 05 50 8B 85 00", this = "CVehicle")]
pub fn get_door_angle_open_ratio_u32(_v: Veh, _door: u32) -> f32 {
    g::<f32>(0x858B50).read()
}

/// `CVehicle::GetDooorAngleOpenRatio(eDoors)` (0x6D56B0): 0.0 in ST0.
#[hook(addr = 0x6D56B0, conv = thiscall, name = "CVehicle::GetDooorAngleOpenRatio", expect = "D9 05 50 8B 85 00", this = "CVehicle")]
pub fn get_door_angle_open_ratio(_v: Veh, _door: u32) -> f32 {
    g::<f32>(0x858B50).read()
}

/// `CVehicle::FindWheelWidth(bool)` (0x6D3D50): 0.25 (0x858C84) in ST0.
#[hook(addr = 0x6D3D50, conv = thiscall, name = "CVehicle::FindWheelWidth", expect = "D9 05 84 8C 85 00", this = "CVehicle")]
pub fn find_wheel_width(_v: Veh, _rear: u32) -> f32 {
    g::<f32>(0x858C84).read()
}

/// `CVehicle::IsOpenTopCar()` (0x6B5840): false.
pub fn is_open_top_car(_v: Veh) -> u8 {
    0
}

/// `CVehicle::SetTowLink(CVehicle*, bool)` (0x6B5880): false.
#[hook(addr = 0x6B5880, conv = thiscall, name = "CVehicle::SetTowLink", expect = "32 C0 C2 08 00", this = "CVehicle")]
pub fn set_tow_link(_v: Veh, _other: u32, _put: u32) -> u8 {
    0
}

/// `CVehicle::BreakTowLink()` (0x6B5890): false.
pub fn break_tow_link(_v: Veh) -> u8 {
    0
}

/// `CVehicle::IsComponentPresent` (0x6D62A0): false (`xor al, al; ret 4`).
#[hook(addr = 0x6D62A0, conv = thiscall, name = "CVehicle::IsComponentPresent", expect = "32 C0 C2 04 00", this = "CVehicle")]
pub fn is_component_present(_v: Veh, _a: u32) -> u8 {
    0
}

/// `CVehicle::IsDoorReady` (0x6D62B0): false (`xor al, al; ret 4`).
#[hook(addr = 0x6D62B0, conv = thiscall, name = "CVehicle::IsDoorReady", expect = "32 C0 C2 04 00", this = "CVehicle")]
pub fn is_door_ready(_v: Veh, _a: u32) -> u8 {
    0
}

/// `CVehicle::IsDoorFullyOpen` (0x6D62C0): false (`xor al, al; ret 4`).
#[hook(addr = 0x6D62C0, conv = thiscall, name = "CVehicle::IsDoorFullyOpen", expect = "32 C0 C2 04 00", this = "CVehicle")]
pub fn is_door_fully_open(_v: Veh, _a: u32) -> u8 {
    0
}

/// `CVehicle::IsDoorClosed` (0x6D62D0): false (`xor al, al; ret 4`).
#[hook(addr = 0x6D62D0, conv = thiscall, name = "CVehicle::IsDoorClosed", expect = "32 C0 C2 04 00", this = "CVehicle")]
pub fn is_door_closed(_v: Veh, _a: u32) -> u8 {
    0
}

/// `CVehicle::IsDoorMissing` (0x6D62E0): false (`xor al, al; ret 4`).
#[hook(addr = 0x6D62E0, conv = thiscall, name = "CVehicle::IsDoorMissing", expect = "32 C0 C2 04 00", this = "CVehicle")]
pub fn is_door_missing(_v: Veh, _a: u32) -> u8 {
    0
}

/// `CVehicle::IsDoorReadyU32` (0x6D62F0): false (`xor al, al; ret 4`).
#[hook(addr = 0x6D62F0, conv = thiscall, name = "CVehicle::IsDoorReadyU32", expect = "32 C0 C2 04 00", this = "CVehicle")]
pub fn is_door_ready_u32(_v: Veh, _a: u32) -> u8 {
    0
}

/// `CVehicle::IsDoorFullyOpenU32` (0x6D6300): false (`xor al, al; ret 4`).
#[hook(addr = 0x6D6300, conv = thiscall, name = "CVehicle::IsDoorFullyOpenU32", expect = "32 C0 C2 04 00", this = "CVehicle")]
pub fn is_door_fully_open_u32(_v: Veh, _a: u32) -> u8 {
    0
}

/// `CVehicle::IsDoorClosedU32` (0x6D6310): false (`xor al, al; ret 4`).
#[hook(addr = 0x6D6310, conv = thiscall, name = "CVehicle::IsDoorClosedU32", expect = "32 C0 C2 04 00", this = "CVehicle")]
pub fn is_door_closed_u32(_v: Veh, _a: u32) -> u8 {
    0
}

/// `CVehicle::IsDoorMissingU32` (0x6D6320): false (`xor al, al; ret 4`).
#[hook(addr = 0x6D6320, conv = thiscall, name = "CVehicle::IsDoorMissingU32", expect = "32 C0 C2 04 00", this = "CVehicle")]
pub fn is_door_missing_u32(_v: Veh, _a: u32) -> u8 {
    0
}

/// `CVehicle::SetUpWheelColModel` (0x6D6350): false (`xor al, al; ret 4`).
#[hook(addr = 0x6D6350, conv = thiscall, name = "CVehicle::SetUpWheelColModel", expect = "32 C0 C2 04 00", this = "CVehicle")]
pub fn set_up_wheel_col_model(_v: Veh, _a: u32) -> u8 {
    0
}

/// `CVehicle::BurstTyre(uint8, bool)` (0x6D6360): false.
#[hook(addr = 0x6D6360, conv = thiscall, name = "CVehicle::BurstTyre", expect = "32 C0 C2 08 00", this = "CVehicle")]
pub fn burst_tyre(_v: Veh, _tyre: u32, _effects: u32) -> u8 {
    0
}

/// `CVehicle::IsRoomForPedToLeaveCar(uint32, CVector*)` (0x6D6370): false.
#[hook(addr = 0x6D6370, conv = thiscall, name = "CVehicle::IsRoomForPedToLeaveCar", expect = "32 C0 C2 08 00", this = "CVehicle")]
pub fn is_room_for_ped_to_leave_car(_v: Veh, _door: u32, _offset: u32) -> u8 {
    0
}

/// `CVehicle::GetNumContactWheels()` (0x6D63D0): 4.
#[hook(addr = 0x6D63D0, conv = thiscall, name = "CVehicle::GetNumContactWheels", expect = "B8 04 00 00 00", this = "CVehicle")]
pub fn get_num_contact_wheels(_v: Veh) -> i32 {
    4
}

/// `CVehicle::GetHeightAboveRoad()` (0x6D63F0): `-colModel->bbox.min.z` of the model (fld, fmul -1.0;
/// ST0 is not rounded to f32 by a store).
#[hook(addr = 0x6D63F0, conv = thiscall, name = "CVehicle::GetHeightAboveRoad", expect = "0F BF 41 22 8B 0C 85 C8 B0 A9 00", this = "CVehicle")]
pub fn get_height_above_road(v: Veh) -> X87 {
    let mi = model_info(i32::from(rd::<i16>(v, 0x22)));
    let col = rd::<GPtr<u8>>(mi, 0x14);
    x(rd::<f32>(col, 8)).fmul_m(g::<f32>(0x858C1C).read())
}

/// `CVehicle::AddMovingCollisionSpeed(CVector&)` (0x6B5860): zero vector into the hidden return
/// slot (arg 0); EAX = that pointer.
#[hook(addr = 0x6B5860, conv = thiscall, name = "CVehicle::AddMovingCollisionSpeed", expect = "8B 44 24 04 33 C9", this = "CVehicle")]
pub fn add_moving_collision_speed(_v: Veh, out: GPtr<[f32; 3]>, _point: u32) -> GPtr<[f32; 3]> {
    out.write([0.0; 3]);
    out
}

// --- flags ------------------------------------------------------------------------------------

/// `CVehicle::SetGettingInFlags(uint8)` (0x6D3000): `m_nGettingInFlags |= flags`.
#[hook(addr = 0x6D3000, conv = thiscall, name = "CVehicle::SetGettingInFlags", expect = "8A 91 86 04 00 00", this = "CVehicle")]
pub fn set_getting_in_flags(v: Veh, flags: u32) {
    wr::<u8>(v, 0x486, rd::<u8>(v, 0x486) | flags as u8);
}

/// `CVehicle::SetGettingOutFlags(uint8)` (0x6D3020).
#[hook(addr = 0x6D3020, conv = thiscall, name = "CVehicle::SetGettingOutFlags", expect = "8A 91 87 04 00 00", this = "CVehicle")]
pub fn set_getting_out_flags(v: Veh, flags: u32) {
    wr::<u8>(v, 0x487, rd::<u8>(v, 0x487) | flags as u8);
}

/// `CVehicle::ClearGettingInFlags(uint8)` (0x6D3040, middle part at 0x403844).
#[hook(addr = 0x6D3040, conv = thiscall, name = "CVehicle::ClearGettingInFlags", expect = "8A 44 24 04 90", this = "CVehicle")]
pub fn clear_getting_in_flags(v: Veh, flags: u32) {
    wr::<u8>(v, 0x486, rd::<u8>(v, 0x486) & !(flags as u8));
}

/// `CVehicle::ClearGettingOutFlags(uint8)` (0x6D3060).
#[hook(addr = 0x6D3060, conv = thiscall, name = "CVehicle::ClearGettingOutFlags", expect = "8A 44 24 04 8A 91 87 04 00 00", this = "CVehicle")]
pub fn clear_getting_out_flags(v: Veh, flags: u32) {
    wr::<u8>(v, 0x487, rd::<u8>(v, 0x487) & !(flags as u8));
}

/// `CVehicle::InitWinch(int32)` (0x6D3B60): bits 5..6 of +0x4A8 = `type << 5`.
#[hook(addr = 0x6D3B60, conv = thiscall, name = "CVehicle::InitWinch", expect = "8A 44 24 04 8A 91 A8 04 00 00", this = "CVehicle")]
pub fn init_winch(v: Veh, kind: u32) {
    let dl = rd::<u8>(v, 0x4A8);
    let al = ((kind as u8).wrapping_shl(5) ^ dl) & 0x60;
    wr::<u8>(v, 0x4A8, dl ^ al);
}

/// `CVehicle::SetHasslePosId(int32, bool)` (0x6D3B30): set (or toggle, `xor`) bit `id` of +0x51B
/// (`1 << cl` in 8 bits).
#[hook(addr = 0x6D3B30, conv = thiscall, name = "CVehicle::SetHasslePosId", expect = "8B C1 8A 4C 24 08", this = "CVehicle")]
pub fn set_hassle_pos_id(v: Veh, id: u32, set: u32) {
    let bit = 1u8.wrapping_shl(u32::from(id as u8) & 0x1F);
    // `shl dl, cl` masks the count to 5 bits; counts 8..31 give 0
    let bit = if (id as u8 & 0x1F) >= 8 { 0 } else { bit };
    let cur = rd::<u8>(v, 0x51B);
    wr::<u8>(v, 0x51B, if set as u8 != 0 { cur | bit } else { cur ^ bit });
}

/// `CVehicle::SetCompAlphaCB(RpMaterial*, void* alpha)` (0x6D2950, cdecl material callback): the
/// material colour's alpha (+7) = the alpha byte; EAX = the material (continue).
#[hook(addr = 0x6D2950, conv = cdecl, name = "CVehicle::SetCompAlphaCB", expect = "8B 44 24 04 8A 4C 24 08")]
pub fn set_comp_alpha_cb(atomic: GPtr<u8>, alpha: u32) -> GPtr<u8> {
    wr::<u8>(atomic, 7, alpha as u8);
    atomic
}

/// `CVehicle::SetComponentAtomicAlpha(RpAtomic*, int32)` (0x6D2960, cdecl): the atomic's geometry
/// flags |= 0x40 (modulate material colour), then `RpGeometryForAllMaterials(geometry,
/// SetCompAlphaCB, alpha)`.
#[hook(addr = 0x6D2960, conv = cdecl, name = "CVehicle::SetComponentAtomicAlpha", expect = "8B 44 24 04 8B 40 18")]
pub fn set_component_atomic_alpha(atomic: GPtr<u8>, alpha: u32) {
    let geo = rd::<GPtr<u8>>(atomic, 0x18);
    wr::<u32>(geo, 8, rd::<u32>(geo, 8) | 0x40);
    rp_geometry_for_all_materials(geo, 0x6D2950, alpha);
}

// --- driver / model checks --------------------------------------------------------------------

/// `CVehicle::IsDriver(CPed*)` (0x6D1C40).
#[hook(addr = 0x6D1C40, conv = thiscall, name = "CVehicle::IsDriver-Ped", expect = "8B 44 24 04 85 C0", this = "CVehicle")]
pub fn is_driver_ped(v: Veh, ped: GPtr<u8>) -> u8 {
    if ped.is_null() {
        return 0;
    }
    b(ped.addr() as u32 == rd::<u32>(v, 0x460))
}

/// `CVehicle::IsDriver(int32 modelIndex)` (0x6D1C60).
#[hook(addr = 0x6D1C60, conv = thiscall, name = "CVehicle::IsDriver-ModelID", expect = "8B 81 60 04 00 00", this = "CVehicle")]
pub fn is_driver_model(v: Veh, model: i32) -> u8 {
    let d = rd::<GPtr<u8>>(v, 0x460);
    b(!d.is_null() && i32::from(rd::<i16>(d, 0x22)) == model)
}

/// `CVehicle::IsLawEnforcementVehicle()` (0x6D2370): models 427, 430, 432, 433, 490, 523, 528,
/// 596..599, 601 (the byte table at 0x6D239C).
#[hook(addr = 0x6D2370, conv = thiscall, name = "CVehicle::IsLawEnforcementVehicle", expect = "0F BF 41 22 05 55 FE FF FF", this = "CVehicle")]
pub fn is_law_enforcement_vehicle(v: Veh) -> u8 {
    let m = i32::from(rd::<i16>(v, 0x22));
    b(matches!(
        m,
        427 | 430 | 432 | 433 | 490 | 523 | 528 | 596..=599 | 601
    ))
}

/// `CVehicle::CanDoorsBeDamaged()` (0x6D1E60): door lock status 0, 1 or 7.
#[hook(addr = 0x6D1E60, conv = thiscall, name = "CVehicle::CanDoorsBeDamaged", expect = "8B 81 F8 04 00 00", this = "CVehicle")]
pub fn can_doors_be_damaged(v: Veh) -> u8 {
    b(matches!(rd::<u32>(v, 0x4F8), 0 | 1 | 7))
}

// --- lighting ---------------------------------------------------------------------------------

/// `CVehicle::SetCollisionLighting(tColLighting)` (0x6D0CA0): the byte into all four of +0x574..0x577.
#[hook(addr = 0x6D0CA0, conv = thiscall, name = "CVehicle::SetCollisionLighting", expect = "8D 81 74 05 00 00", this = "CVehicle")]
pub fn set_collision_lighting(v: Veh, lighting: u32) {
    let l = u32::from(lighting as u8);
    wr::<u32>(v, 0x574, l * 0x0101_0101);
}

/// `CVehicle::UpdateLightingFromStoredPolys()` (0x6D0CC0): front pair from +0x544, rear pair from +0x570.
#[hook(addr = 0x6D0CC0, conv = thiscall, name = "CVehicle::UpdateLightingFromStoredPolys", expect = "8A 81 44 05 00 00", this = "CVehicle")]
pub fn update_lighting_from_stored_polys(v: Veh) {
    let f = rd::<u8>(v, 0x544);
    wr::<u8>(v, 0x574, f);
    wr::<u8>(v, 0x575, f);
    let r = rd::<u8>(v, 0x570);
    wr::<u8>(v, 0x576, r);
    wr::<u8>(v, 0x577, r);
}

/// `CVehicle::CustomCarPlate_TextureDestroy()` (0x6D1150).
#[hook(addr = 0x6D1150, conv = thiscall, name = "CVehicle::CustomCarPlate_TextureDestroy", expect = "56 8B F1 8B 86 88 05 00 00", this = "CVehicle")]
pub fn custom_car_plate_texture_destroy(v: Veh) {
    let t = rd::<GPtr<u8>>(v, 0x588);
    if !t.is_null() {
        rw_texture_destroy(t);
        wr::<u32>(v, 0x588, 0);
    }
}

// --- winch (CRopes) ---------------------------------------------------------------------------

fn rope_of(v: Veh) -> i32 {
    ropes_find_rope((v.addr() as u32).wrapping_add(0x1D))
}

/// `CVehicle::QueryPickedUpEntityWithWinch()` (0x6D3CF0): `CRopes::aRopes[FindRope(this + 0x1D)]`
/// +0x318 (no check of a -1 index, as the original).
#[hook(addr = 0x6D3CF0, conv = thiscall, name = "CVehicle::QueryPickedUpEntityWithWinch", expect = "83 C1 1D 51 E8 07 23 E8 FF", this = "CVehicle")]
pub fn query_picked_up_entity_with_winch(v: Veh) -> u32 {
    let k = rope_of(v);
    g::<u32>((k.wrapping_mul(0x328) as u32).wrapping_add(0xB76BD0) as usize).read()
}

/// `CVehicle::GetRopeHeightForHeli()` (0x6D3D10).
#[hook(addr = 0x6D3D10, conv = thiscall, name = "CVehicle::GetRopeHeightForHeli", expect = "83 C1 1D 51 E8 E7 22 E8 FF", this = "CVehicle")]
pub fn get_rope_height_for_heli(v: Veh) -> f32 {
    let k = rope_of(v);
    g::<f32>((k.wrapping_mul(0x328) as u32).wrapping_add(0xB76BD4) as usize).read()
}

/// `CVehicle::SetRopeHeightForHeli(float)` (0x6D3D30).
#[hook(addr = 0x6D3D30, conv = thiscall, name = "CVehicle::SetRopeHeightForHeli", expect = "83 C1 1D 51 E8 C7 22 E8 FF", this = "CVehicle")]
pub fn set_rope_height_for_heli(v: Veh, height: u32) {
    let k = rope_of(v);
    g::<u32>((k.wrapping_mul(0x328) as u32).wrapping_add(0xB76BD4) as usize).write(height);
}

/// `CVehicle::ReleasePickedUpEntityWithWinch()` (0x6D3CB0): tail call
/// `CRope::ReleasePickedUpObject` on `CRopes::aRopes[FindRope(this + 0x1D)]` (0xB768B8, 0x328 each).
#[hook(addr = 0x6D3CB0, conv = thiscall, name = "CVehicle::ReleasePickedUpEntityWithWinch", expect = "83 C1 1D 51 E8 47 23 E8 FF", this = "CVehicle")]
pub fn release_picked_up_entity_with_winch(v: Veh) {
    let k = rope_of(v);
    rope_release_picked_up_object(g::<u8>(
        (k.wrapping_mul(0x328) as u32).wrapping_add(0xB768B8) as usize
    ));
}

/// `CVehicle::PickUpEntityWithWinch(CEntity*)` (0x6D3CD0): tail call `CRope::PickUpObject(entity)`.
#[hook(addr = 0x6D3CD0, conv = thiscall, name = "CVehicle::PickUpEntityWithWinch", expect = "83 C1 1D 51 E8 27 23 E8 FF", this = "CVehicle")]
pub fn pick_up_entity_with_winch(v: Veh, e: GPtr<u8>) {
    let k = rope_of(v);
    rope_pick_up_object(
        g::<u8>((k.wrapping_mul(0x328) as u32).wrapping_add(0xB768B8) as usize),
        e,
    );
}

/// `CVehicle::RemoveUpgrade(int32)` (0x6D3630): `RpClumpForAllAtomics(m_pRwClump, RemoveUpgradeCB
/// (0x6D3300), upgrade)`.
#[hook(addr = 0x6D3630, conv = thiscall, name = "CVehicle::RemoveUpgrade", expect = "8B 44 24 04 8B 49 18", this = "CVehicle")]
pub fn remove_upgrade(v: Veh, upgrade: u32) {
    rp_clump_for_all_atomics(rd::<GPtr<u8>>(v, 0x18), 0x6D3300, upgrade);
}

/// `CVehicle::Shutdown()` (0x6D0B40, static): `m_aSpecialColModel[4]` (0xC1CC78, CColModel 0x30):
/// those with collision data (+0x2C) get `CColModel::RemoveCollisionVolumes`.
#[hook(addr = 0x6D0B40, conv = cdecl, name = "CVehicle::Shutdown", expect = "56 BE 78 CC C1 00")]
pub fn shutdown() {
    let mut s = 0xC1CC78usize;
    while s < 0xC1CD38 {
        let slot = g::<u8>(s);
        if rd::<u32>(slot, 0x2C) != 0 {
            col_model_remove_collision_volumes(slot);
        }
        s += 0x30;
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GET_DOOR_ANGLE_OPEN_RATIO_U32,
    HOOK_GET_DOOR_ANGLE_OPEN_RATIO,
    HOOK_FIND_WHEEL_WIDTH,
    HOOK_SET_TOW_LINK,
    HOOK_IS_COMPONENT_PRESENT,
    HOOK_IS_DOOR_READY,
    HOOK_IS_DOOR_FULLY_OPEN,
    HOOK_IS_DOOR_CLOSED,
    HOOK_IS_DOOR_MISSING,
    HOOK_IS_DOOR_READY_U32,
    HOOK_IS_DOOR_FULLY_OPEN_U32,
    HOOK_IS_DOOR_CLOSED_U32,
    HOOK_IS_DOOR_MISSING_U32,
    HOOK_SET_UP_WHEEL_COL_MODEL,
    HOOK_BURST_TYRE,
    HOOK_IS_ROOM_FOR_PED_TO_LEAVE_CAR,
    HOOK_GET_NUM_CONTACT_WHEELS,
    HOOK_GET_HEIGHT_ABOVE_ROAD,
    HOOK_ADD_MOVING_COLLISION_SPEED,
    HOOK_SET_GETTING_IN_FLAGS,
    HOOK_SET_GETTING_OUT_FLAGS,
    HOOK_CLEAR_GETTING_IN_FLAGS,
    HOOK_CLEAR_GETTING_OUT_FLAGS,
    HOOK_INIT_WINCH,
    HOOK_SET_HASSLE_POS_ID,
    HOOK_SET_COMP_ALPHA_CB,
    HOOK_SET_COMPONENT_ATOMIC_ALPHA,
    HOOK_IS_DRIVER_PED,
    HOOK_IS_DRIVER_MODEL,
    HOOK_IS_LAW_ENFORCEMENT_VEHICLE,
    HOOK_CAN_DOORS_BE_DAMAGED,
    HOOK_SET_COLLISION_LIGHTING,
    HOOK_UPDATE_LIGHTING_FROM_STORED_POLYS,
    HOOK_CUSTOM_CAR_PLATE_TEXTURE_DESTROY,
    HOOK_QUERY_PICKED_UP_ENTITY_WITH_WINCH,
    HOOK_GET_ROPE_HEIGHT_FOR_HELI,
    HOOK_SET_ROPE_HEIGHT_FOR_HELI,
    HOOK_RELEASE_PICKED_UP_ENTITY_WITH_WINCH,
    HOOK_PICK_UP_ENTITY_WITH_WINCH,
    HOOK_REMOVE_UPGRADE,
    HOOK_SHUTDOWN,
];

/// This module's globals (replace with a `sa_core::globals! { list GLOBALS; ... }` block when it gets some).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
