//! `CWorld` leaves that do not touch `CPhysical` (PORT-WORLD part 1): the sector/list accessors, the
//! scan-code reset, the line-test options and the `FindPlayer*` family. Ported from the 1.0 US bytes
//! (gta-reversed `World.cpp` as reading aid; several `FindPlayer*` bodies differ from it, see the docs).
//!
//! `CPlayerInfo` (0x190) array `CWorld::Players` at 0xB7CD98: +0 `m_pPed`, +4 `m_pWanted`... (gr:
//! `m_PlayerData.m_Wanted`), +0xB0 `m_pRemoteVehicle`. `CPed`: +0x46C flags (bit 8 `bInVehicle`), +0x58C
//! `m_pVehicle`; `CVehicle` +0x590 vehicle type (6 = train); `CPlaceable` +4 placement position, +0x14
//! matrix (position at +0x30); `CPhysical` +0x44 move speed; `CEntity` +0x2C scan code.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use super::common::{g, rd, wr};

sa_core::globals! {
    list GLOBALS;
    /// `CWorld::fWeaponSpreadRate`
    pub static WEAPON_SPREAD_RATE @ 0xB7CD64: f32 = 0.0;
    /// `CWorld::pIgnoreEntity`
    pub static IGNORE_ENTITY @ 0xB7CD68: GPtr<u8> = GPtr::NULL;
    /// `CWorld::bIncludeBikers`
    pub static INCLUDE_BIKERS @ 0xB7CD6F: u8 = 0;
    /// `CWorld::bIncludeCarTyres`
    pub static INCLUDE_CAR_TYRES @ 0xB7CD70: u8 = 0;
    /// `CWorld::bIncludeDeadPeds`
    pub static INCLUDE_DEAD_PEDS @ 0xB7CD71: u8 = 0;
    /// `CWorld::PlayerInFocus`
    pub static PLAYER_IN_FOCUS @ 0xB7CD74: u8 = 0;
    /// `CWorld::ms_nCurrentScanCode`
    pub static CURRENT_SCAN_CODE @ 0xB7CD78: u16 = 0;
}

/// `CWorld::Players` (`CPlayerInfo[2]`, 0x190 each); +0 is sa-core's `PLAYER0_PED`, so the array is
/// addressed directly.
pub const PLAYERS: usize = 0xB7CD98;
pub const PLAYER_INFO_SIZE: usize = 0x190;
/// `CWorld::ms_aSectors` (`CSector[120][120]`, 8 bytes: buildings, dummies lists)
pub const SECTORS: usize = 0xB7D0B8;
/// `CWorld::ms_aRepeatSectors` (`CRepeatSector[16][16]`, 12 bytes: vehicles, peds, objects lists)
pub const REPEAT_SECTORS: usize = 0xB992B8;
/// `CWorld::ms_aLodPtrLists` (`CPtrListSingleLink[30][30]`)
pub const LOD_PTR_LISTS: usize = 0xB99EB8;
/// `TheCamera.m_bCameraJustRestored`... the byte at 0x9690C1 that `FindPlayerCentreOfWorld` tests
/// (gr: `CCamera::m_bUseMouse3rdPerson`... `TheCamera.m_bUseScriptZoomValue`... unnamed here)
const CAM_FLAG_9690C1: usize = 0x9690C1;
/// the static zero vector `FindPlayerCentreOfWorld_NoSniperShift` returns without a ped
const ZERO_VECTOR_B9B970: usize = 0xB9B970;

fn player(id: i32) -> GPtr<u8> {
    g::<u8>(PLAYERS.wrapping_add((id as isize * PLAYER_INFO_SIZE as isize) as usize))
}

/// `id < 0` → `PlayerInFocus` (movzx).
fn resolve(id: i32) -> i32 {
    if id < 0 { i32::from(PLAYER_IN_FOCUS.get()) } else { id }
}

fn ped_of(id: i32) -> GPtr<u8> {
    rd::<GPtr<u8>>(player(id), 0)
}

/// `CPlaceable::GetPosition()`: matrix + 0x30 when there is a matrix, else +4.
fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

fn in_vehicle(ped: GPtr<u8>) -> bool {
    rd::<u32>(ped, 0x46C) & 0x100 != 0
}

/// `CWorld::GetCurrentScanCode` (0x407250).
#[hook(addr = 0x407250, conv = cdecl, name = "CWorld::GetCurrentScanCode", expect = "66 A1 78 CD B7 00")]
pub fn get_current_scan_code() -> u16 {
    CURRENT_SCAN_CODE.get()
}

/// `CWorld::GetSector(int32 x, int32 y)` (stub 0x407260 → 0x1566820): both clamped to 0..119 (negative
/// → 0, `>= 0x77` → 0x77).
#[hook(addr = 0x407260, conv = cdecl, name = "CWorld::GetSector", expect = "E9 BB F5 15 01")]
pub fn get_sector(x: i32, y: i32) -> GPtr<u8> {
    let c = |v: i32| {
        let v = if v <= 0 { 0 } else { v };
        if v >= 0x77 { 0x77 } else { v }
    };
    g::<u8>(SECTORS.wrapping_add(((c(y) * 0x78 + c(x)) * 8) as usize))
}

/// `CWorld::GetRepeatSector(int32 x, int32 y)` (stub 0x4072A0 → 0x156C6B0): `(y & 15) * 16 + (x & 15)`.
#[hook(addr = 0x4072A0, conv = cdecl, name = "CWorld::GetRepeatSector", expect = "E9 0B 54 16 01")]
pub fn get_repeat_sector(x: i32, y: i32) -> GPtr<u8> {
    g::<u8>(REPEAT_SECTORS.wrapping_add((((y & 0xF) * 16 + (x & 0xF)) * 12) as usize))
}

/// `CWorld::GetLodPtrList(int32 x, int32 y)` (0x4072C0): `y * 30 + x`, unchecked.
#[hook(addr = 0x4072C0, conv = cdecl, name = "CWorld::GetLodPtrList", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn get_lod_ptr_list(x: i32, y: i32) -> GPtr<u8> {
    g::<u8>(LOD_PTR_LISTS.wrapping_add((y.wrapping_mul(0x1E).wrapping_add(x) as u32).wrapping_mul(4) as usize))
}

/// `CWorld::ResetLineTestOptions` (0x5631C0).
#[hook(addr = 0x5631C0, conv = cdecl, name = "CWorld::ResetLineTestOptions", expect = "33 C0 A2 71 CD B7 00")]
pub fn reset_line_test_options() {
    INCLUDE_DEAD_PEDS.set(0);
    INCLUDE_CAR_TYRES.set(0);
    INCLUDE_BIKERS.set(0);
    IGNORE_ENTITY.set(GPtr::NULL);
    WEAPON_SPREAD_RATE.set(f32::from_bits(0));
}

/// Clears the scan code (+0x2C) of every entity in a `CPtrList` (node: +0 item, +4 next).
fn clear_list(list: GPtr<u8>) {
    let mut n = rd::<GPtr<u8>>(list, 0);
    while !n.is_null() {
        wr::<u16>(rd::<GPtr<u8>>(n, 0), 0x2C, 0);
        n = rd::<GPtr<u8>>(n, 4);
    }
}

/// `CWorld::ClearScanCodes` (stub 0x563470 → 0x408258): every sector's two lists, then every repeat
/// sector's three lists.
// Not hooked: the protection code at 0x4013A2 reads the entry dword 0x563470 as data (`add eax, [0x563470]`).
// Ported, unhookable.
pub fn clear_scan_codes() {
    let mut s = SECTORS;
    while s < REPEAT_SECTORS {
        clear_list(g(s));
        clear_list(g(s + 4));
        s += 8;
    }
    let mut r = REPEAT_SECTORS;
    while r < LOD_PTR_LISTS {
        clear_list(g(r));
        clear_list(g(r + 4));
        clear_list(g(r + 8));
        r += 12;
    }
}

/// `CWorld::FindPlayerSlotWithPedPointer(void*)` (0x563FA0): slot 0..1 whose ped is `p`, else -1.
#[hook(addr = 0x563FA0, conv = cdecl, name = "CWorld::FindPlayerSlotWithPedPointer", expect = "8B 54 24 04 33 C0")]
pub fn find_player_slot_with_ped_pointer(p: GPtr<u8>) -> i32 {
    for i in 0..2 {
        if rd::<GPtr<u8>>(player(i), 0) == p {
            return i;
        }
    }
    -1
}

/// `CWorld::FindPlayerSlotWithRemoteVehiclePointer(void*)` (0x563FD0): slot whose remote vehicle
/// (+0xB0) is `p`, else -1.
#[hook(addr = 0x563FD0, conv = cdecl, name = "CWorld::FindPlayerSlotWithRemoteVehiclePointer", expect = "8B 54 24 04 33 C0")]
pub fn find_player_slot_with_remote_vehicle_pointer(p: GPtr<u8>) -> i32 {
    for i in 0..2 {
        if rd::<GPtr<u8>>(player(i), 0xB0) == p {
            return i;
        }
    }
    -1
}

/// `CWorld::FindPlayerSlotWithVehiclePointer(CEntity*)` (0x564000): slot whose ped's `m_pVehicle`
/// (+0x58C, no in-vehicle test) is `v`; **0** when none (original; gr returns -1).
#[hook(addr = 0x564000, conv = cdecl, name = "CWorld::FindPlayerSlotWithVehiclePointer", expect = "56 8B 74 24 08")]
pub fn find_player_slot_with_vehicle_pointer(v: GPtr<u8>) -> i32 {
    for i in 0..2 {
        let ped = rd::<GPtr<u8>>(player(i), 0);
        if !ped.is_null() && rd::<GPtr<u8>>(ped, 0x58C) == v {
            return i;
        }
    }
    0
}

/// `FindPlayerCoors(CVector* out, int32 id)` (0x56E010): the vehicle's position when the ped is in one,
/// else the ped's; zero without a ped. Returns `out`.
#[hook(addr = 0x56E010, conv = cdecl, name = "CWorld::FindPlayerCoors", expect = "8B 44 24 08 33 D2")]
pub fn find_player_coors(out: GPtr<f32>, id: i32) -> GPtr<f32> {
    let o = out.cast::<u8>();
    let ped = ped_of(resolve(id));
    if ped.is_null() {
        wr::<u32>(o, 0, 0);
        wr::<u32>(o, 4, 0);
        wr::<u32>(o, 8, 0);
        return out;
    }
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    let src = if in_vehicle(ped) && !veh.is_null() { pos_ptr(veh) } else { pos_ptr(ped) };
    wr::<u32>(o, 0, rd::<u32>(src, 0));
    wr::<u32>(o, 4, rd::<u32>(src, 4));
    wr::<u32>(o, 8, rd::<u32>(src, 8));
    out
}

pub fn find_player_coors_port(out: GPtr<f32>, id: i32) -> GPtr<f32> {
    find_player_coors(out, id)
}

/// `FindPlayerSpeed(int32 id)` (0x56E090): `&vehicle->m_vecMoveSpeed` (+0x44) in a vehicle, else the
/// ped's (no null check of the ped).
#[hook(addr = 0x56E090, conv = cdecl, name = "CWorld::FindPlayerSpeed", expect = "8B 44 24 04 85 C0")]
pub fn find_player_speed(id: i32) -> GPtr<u8> {
    let ped = ped_of(resolve(id));
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    if in_vehicle(ped) && !veh.is_null() { veh.byte_add(0x44) } else { ped.byte_add(0x44) }
}

/// `FindPlayerVehicle(int32 id, bool includeRemote)` (0x56E0D0): null without a ped or outside a
/// vehicle; with `includeRemote` the remote vehicle when set; else `m_pVehicle`.
#[hook(addr = 0x56E0D0, conv = cdecl, name = "CWorld::FindPlayerVehicle", expect = "8B 44 24 04 85 C0")]
pub fn find_player_vehicle(id: i32, include_remote: u8) -> GPtr<u8> {
    let p = player(resolve(id));
    let ped = rd::<GPtr<u8>>(p, 0);
    if ped.is_null() || !in_vehicle(ped) {
        return GPtr::NULL;
    }
    if include_remote != 0 {
        let r = rd::<GPtr<u8>>(p, 0xB0);
        if !r.is_null() {
            return r;
        }
    }
    rd::<GPtr<u8>>(ped, 0x58C)
}

pub fn find_player_vehicle_port(id: i32, remote: u8) -> GPtr<u8> {
    find_player_vehicle(id, remote)
}

/// `FindPlayerEntity(int32 id)` (0x56E120): the vehicle in a vehicle, else the ped (no null check).
#[hook(addr = 0x56E120, conv = cdecl, name = "CWorld::FindPlayerEntity", expect = "8B 44 24 04 85 C0")]
pub fn find_player_entity(id: i32) -> GPtr<u8> {
    let ped = ped_of(resolve(id));
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    if in_vehicle(ped) && !veh.is_null() { veh } else { ped }
}

/// `FindPlayerVehicle(id, false)` as inlined in the original (null-safe).
fn vehicle_inline(id: i32) -> GPtr<u8> {
    let ped = ped_of(resolve(id));
    if !ped.is_null() && in_vehicle(ped) { rd::<GPtr<u8>>(ped, 0x58C) } else { GPtr::NULL }
}

/// `FindPlayerTrain(int32 id)` (0x56E160): the vehicle when the player is in one whose type (+0x590)
/// is 6, else null.
#[hook(addr = 0x56E160, conv = cdecl, name = "CWorld::FindPlayerTrain", expect = "8B 44 24 04 85 C0")]
pub fn find_player_train(id: i32) -> GPtr<u8> {
    let v = vehicle_inline(id);
    if v.is_null() {
        return GPtr::NULL;
    }
    // re-evaluated (the second inlined copy is the same value)
    if rd::<i32>(vehicle_inline(id), 0x590) != 6 {
        return GPtr::NULL;
    }
    vehicle_inline(id)
}

/// `FindPlayerPed(int32 id)` (0x56E210).
#[hook(addr = 0x56E210, conv = cdecl, name = "CWorld::FindPlayerPed", expect = "8B 44 24 04 85 C0")]
pub fn find_player_ped(id: i32) -> GPtr<u8> {
    ped_of(resolve(id))
}

pub fn find_player_ped_port(id: i32) -> GPtr<u8> {
    find_player_ped(id)
}

/// `FindPlayerWanted(int32 id)` (0x56E230): `Players[id]` +4.
#[hook(addr = 0x56E230, conv = cdecl, name = "CWorld::FindPlayerWanted", preserve = "ecx", expect = "8B 44 24 04 85 C0")]
pub fn find_player_wanted(id: i32) -> GPtr<u8> {
    match resolve(id) {
        0 => super::common::PLAYER0_WANTED.get(),
        1 => super::common::PLAYER1_WANTED.get(),
        r => rd::<GPtr<u8>>(player(r), 4),
    }
}

/// `FindPlayerCentreOfWorld(int32 id)` (0x56E250). With the camera byte 0x9690C1 set: the camera
/// entity's matrix position (`[0xB6F03C] + 0x30`) or the vector at 0xB6F02C. Else the remote vehicle's
/// position (of `id`, resolved); else, when the **focus** player (`PlayerInFocus`, not `id`) is in a
/// vehicle, that vehicle's position; else the position of `Players[id].m_pPed` (no null check).
#[hook(addr = 0x56E250, conv = cdecl, name = "CWorld::FindPlayerCentreOfWorld", expect = "8B 44 24 04 85 C0")]
pub fn find_player_centre_of_world(id: i32) -> GPtr<u8> {
    let id = resolve(id);
    if g::<u8>(CAM_FLAG_9690C1).read() != 0 {
        return super::common::cam_pos().cast();
    }
    let remote = rd::<GPtr<u8>>(player(id), 0xB0);
    if !remote.is_null() {
        return pos_ptr(remote);
    }
    let focus = i32::from(PLAYER_IN_FOCUS.get());
    let fped = ped_of(focus);
    if !fped.is_null() && in_vehicle(fped) && !rd::<GPtr<u8>>(fped, 0x58C).is_null() {
        return pos_ptr(vehicle_inline(focus));
    }
    pos_ptr(ped_of(id))
}

/// `FindPlayerCentreOfWorld_NoSniperShift(int32 id)` (0x56E320). As above, but the remote-vehicle test
/// and the final ped use the raw `id` (-1 indexes before the array, original); the in-vehicle test uses
/// `id` resolved; without a ped it returns the vector at 0xB9B970.
#[hook(addr = 0x56E320, conv = cdecl, name = "CWorld::FindPlayerCentreOfWorld_NoSniperShift", expect = "A0 C1 90 96 00")]
pub fn find_player_centre_of_world_no_sniper_shift(id: i32) -> GPtr<u8> {
    if g::<u8>(CAM_FLAG_9690C1).read() != 0 {
        return super::common::cam_pos().cast();
    }
    let remote = rd::<GPtr<u8>>(player(id), 0xB0);
    if !remote.is_null() {
        return pos_ptr(remote);
    }
    let ped = ped_of(resolve(id));
    if !ped.is_null() && in_vehicle(ped) && !rd::<GPtr<u8>>(ped, 0x58C).is_null() {
        return pos_ptr(vehicle_inline(id));
    }
    let raw = rd::<GPtr<u8>>(player(id), 0);
    if raw.is_null() {
        return g::<u8>(ZERO_VECTOR_B9B970);
    }
    pos_ptr(raw)
}

sa_core::orig! {
    /// `CWorld::FindPlayerCentreOfWorld(int32)` (0x56E250)
    pub fn find_player_centre_of_world_call(id: i32) -> GPtr<u8> = cdecl 0x56E250 => find_player_centre_of_world;
    /// `CEntryExitManager::GetPositionRelativeToOutsideWorld(CVector&)` (0x43F150)
    pub fn eem_get_position_relative_to_outside_world(v: GPtr<f32>) = cdecl 0x43F150 => unported;
}

/// `FindPlayerCentreOfWorldForMap(CVector* out, int32 id)` (0x56E400): copies
/// `FindPlayerCentreOfWorld(id)` to a local, `CEntryExitManager::GetPositionRelativeToOutsideWorld` on
/// it, copies it to `out`; returns `out`.
#[hook(addr = 0x56E400, conv = cdecl, name = "CWorld::FindPlayerCentreOfWorldForMap", expect = "83 EC 0C 8B 44 24 14")]
pub fn find_player_centre_of_world_for_map(out: GPtr<f32>, id: i32) -> GPtr<f32> {
    let c = find_player_centre_of_world_call(id);
    let mut v = [rd::<u32>(c, 0), rd::<u32>(c, 4), rd::<u32>(c, 8)];
    let vp = GPtr::from_raw(v.as_mut_ptr());
    eem_get_position_relative_to_outside_world(vp.cast());
    let o = out.cast::<u8>();
    wr::<u32>(o, 0, v[0]);
    wr::<u32>(o, 4, v[1]);
    wr::<u32>(o, 8, v[2]);
    out
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GET_CURRENT_SCAN_CODE,
    HOOK_GET_SECTOR,
    HOOK_GET_REPEAT_SECTOR,
    HOOK_GET_LOD_PTR_LIST,
    HOOK_RESET_LINE_TEST_OPTIONS,
    HOOK_FIND_PLAYER_SLOT_WITH_PED_POINTER,
    HOOK_FIND_PLAYER_SLOT_WITH_REMOTE_VEHICLE_POINTER,
    HOOK_FIND_PLAYER_SLOT_WITH_VEHICLE_POINTER,
    HOOK_FIND_PLAYER_COORS,
    HOOK_FIND_PLAYER_SPEED,
    HOOK_FIND_PLAYER_VEHICLE,
    HOOK_FIND_PLAYER_ENTITY,
    HOOK_FIND_PLAYER_TRAIN,
    HOOK_FIND_PLAYER_PED,
    HOOK_FIND_PLAYER_WANTED,
    HOOK_FIND_PLAYER_CENTRE_OF_WORLD,
    HOOK_FIND_PLAYER_CENTRE_OF_WORLD_NO_SNIPER_SHIFT,
    HOOK_FIND_PLAYER_CENTRE_OF_WORLD_FOR_MAP,
];
