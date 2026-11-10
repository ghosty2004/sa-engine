//! `CBridge`, `CCranes` (empty in the PC build) and `CGridRef` (PORT-WORLD). Ported from the 1.0 US
//! bytes; gta-reversed `Bridge.cpp`, `Cranes.cpp`, `GridRef.cpp` as reading aids.
//!
//! The bridge and crane functions are a lone `ret` or `xor al, al; ret`: too short for the 5-byte
//! entry patch, so they are ported but not hookable (`ported-unhookable`).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{find_player_coors, ftol32, x};

sa_core::globals! {
    list GLOBALS;
    /// `CGridRef::GridRefList` (`char[10][10][32]`)
    pub static GRID_REF_LIST @ 0xC72FB0: [u8; 3200] = [0; 3200];
}

/// 0x858B54: the empty string `GetArtistBugstarID` returns.
const EMPTY_STRING: usize = 0x858B54;

orig! {
    /// CRT `atol` (0x82258E)
    pub fn atol(s: GPtr<u8>) -> i32 = cdecl 0x82258E => unported;
}

/// `CBridge::Init` (0x41BC70): `ret`.
pub fn bridge_init() {}
/// `CBridge::Update` (0x41BC80): `ret`.
pub fn bridge_update() {}
/// `CBridge::ShouldLightsBeFlashing` (0x41BC90): false.
pub fn bridge_should_lights_be_flashing() -> u8 {
    0
}
/// `CBridge::FindBridgeEntities` (0x41BCA0): `ret`.
pub fn bridge_find_bridge_entities() {}
/// `CCranes::InitCranes` (0x6F3FC0): `ret`.
pub fn cranes_init_cranes() {}
/// `CCranes::IsThisCarPickedUp` (0x6F3FD0): false.
pub fn cranes_is_this_car_picked_up(_x: f32, _y: f32, _v: GPtr<u8>) -> u8 {
    0
}
/// `CCranes::UpdateCranes` (0x6F3FE0): `ret`.
pub fn cranes_update_cranes() {}
/// `CCranes::IsThisCarBeingCarriedByAnyCrane` (0x6F3FF0): false.
pub fn cranes_is_this_car_being_carried_by_any_crane(_v: GPtr<u8>) -> u8 {
    0
}
/// `CCranes::IsThisCarBeingTargettedByAnyCrane` (0x6F4000): false.
pub fn cranes_is_this_car_being_targetted_by_any_crane(_v: GPtr<u8>) -> u8 {
    0
}

/// The grid cell of a position: `x = (u8)_ftol((px + 3000) * 0.0016666667)`, `y = 9 - (u8)_ftol(...)`.
fn grid_pos(px: f32, py: f32, out_x: GPtr<u8>, out_y: GPtr<u8>) {
    let inv600 = f32::from_bits(0x3ADA_740E);
    let gx = ftol32(x(px).fadd_m(3000.0).fmul_m(inv600)) as u8;
    out_x.write(gx);
    let gy = ftol32(x(py).fadd_m(3000.0).fmul_m(inv600)) as u8;
    out_y.write(9u8.wrapping_sub(gy));
}

/// `CGridRef::GetGridRefPositions(CVector pos, uint8* x, uint8* y)` (0x71D5A0, vector by value).
#[hook(addr = 0x71D5A0, conv = cdecl, name = "CGridRef::GetGridRefPositions-vec", expect = "D9 44 24 04 D8 05 94 9A 85 00")]
pub fn get_grid_ref_positions_vec(px: f32, py: f32, _pz: f32, out_x: GPtr<u8>, out_y: GPtr<u8>) {
    grid_pos(px, py, out_x, out_y);
}

/// `CGridRef::GetGridRefPositions(uint8* x, uint8* y)` (0x71D5E0): `FindPlayerCoors(-1)` twice (the first
/// result unused, as in the original), the cell of the second.
#[hook(addr = 0x71D5E0, conv = cdecl, name = "CGridRef::GetGridRefPositions-", expect = "83 EC 24 8D 44 24 0C")]
pub fn get_grid_ref_positions(out_x: GPtr<u8>, out_y: GPtr<u8>) {
    let mut a = [0f32; 3];
    let mut b = [0f32; 3];
    find_player_coors(GPtr::from_raw(a.as_mut_ptr()), -1);
    let r = find_player_coors(GPtr::from_raw(b.as_mut_ptr()), -1);
    let (px, py) = (r.read(), r.wrapping_index(1).read());
    grid_pos(px, py, out_x, out_y);
}

/// `CGridRef::GetArtistBugstarID(uint8 x, uint8 y)` (0x71D650): "" for x, y < 10 (unsigned), else null.
#[hook(addr = 0x71D650, conv = cdecl, name = "CGridRef::GetArtistBugstarID", expect = "8A 4C 24 04 B0 0A")]
pub fn get_artist_bugstar_id(gx: u8, gy: u8) -> GPtr<u8> {
    if gx < 10 && gy < 10 { GPtr::from_addr(EMPTY_STRING) } else { GPtr::NULL }
}

/// `CGridRef::GetAreaName(uint8 x, uint8 y)` (0x71D670): `atol(GridRefList[x][y])` for x, y < 10, else 0.
#[hook(addr = 0x71D670, conv = cdecl, name = "CGridRef::GetAreaName", expect = "8A 44 24 04 3C 0A")]
pub fn get_area_name(gx: u8, gy: u8) -> i32 {
    if gx >= 10 || gy >= 10 {
        return 0;
    }
    let off = (u32::from(gx) * 10 + u32::from(gy)) * 32;
    atol(GRID_REF_LIST.ptr().cast::<u8>().byte_add(off as isize))
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GET_GRID_REF_POSITIONS_VEC,
    HOOK_GET_GRID_REF_POSITIONS,
    HOOK_GET_ARTIST_BUGSTAR_ID,
    HOOK_GET_AREA_NAME,
];
