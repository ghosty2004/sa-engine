//! Static initialisers / `atexit` destructors of the PORT-WORLD part 1 globals (CRT `__xc` entries;
//! names from CAT-6 `__init_*` / `__dtor_*`). Same pattern as sa-stream's `CLoadingScreen` statics: the
//! array constructions go through the CRT's `eh vector constructor iterator` with the original
//! constructor/destructor addresses, and the destructor is registered with `_atexit`.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, x};

orig! {
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// CRT `eh vector destructor iterator` (0x821E02, stdcall)
    pub fn eh_vector_dtor(arr: GPtr<u8>, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// CRT `_atexit` (0x821D1E)
    pub fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// `CPedClothesDesc::CPedClothesDesc` (0x5A8020)
    pub fn ped_clothes_desc_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x5A8020 => unported;
    /// `CPtrListDoubleLink::Flush` (0x552470)
    pub fn ptr_list_double_link_flush(this: GPtr<u8>) = thiscall 0x552470 => unported;
}

/// `__init_g_96a718` (0x849B20): `[0x96A718] = [0x96A714] * 10.0` (f32).
#[hook(addr = 0x849B20, conv = cdecl, name = "__init_g_96a718", expect = "D9 05 14 A7 96 00")]
pub fn init_g_96a718() {
    g::<f32>(0x96A718).write(x(g::<f32>(0x96A714).read()).fmul_m(10.0).to_f32());
}

/// `__init_g_96a71c` (0x849B40): `[0x96A71C] = [0x858630] / 50.0` (f32).
#[hook(addr = 0x849B40, conv = cdecl, name = "__init_g_96a71c", expect = "D9 05 30 86 85 00")]
pub fn init_g_96a71c() {
    g::<f32>(0x96A71C).write(x(f32::from_bits(0x3E8E_38EB)).fdiv_m(50.0).to_f32());
}

/// `__init_g_97d64c` (0x84A260): `[0x97D64C] = 1.0 / 50.0` (f32).
#[hook(addr = 0x84A260, conv = cdecl, name = "__init_g_97d64c", expect = "D9 05 24 86 85 00")]
pub fn init_g_97d64c() {
    g::<f32>(0x97D64C).write(x(1.0).fdiv_m(50.0).to_f32());
}

/// `__init_g_97d650` (0x84A280): `[0x97D650] = 100.0 / 3.6` (f32; 3.6 = bits 0x40666666).
#[hook(addr = 0x84A280, conv = cdecl, name = "__init_g_97d650", expect = "D9 05 28 86 85 00")]
pub fn init_g_97d650() {
    g::<f32>(0x97D650).write(x(100.0).fdiv_m(f32::from_bits(0x4066_6666)).to_f32());
}

/// `__init_CGameLogic__SavedWeapons` (0x849CE0): 13 `CWeapon` (0x1C) at 0x96A9B8.
#[hook(addr = 0x849CE0, conv = cdecl, name = "__init_CGameLogic__SavedWeapons", expect = "68 A0 11 44 00")]
pub fn init_saved_weapons() {
    eh_vector_ctor(g(0x96A9B8), 0x1C, 0xD, 0x441E00, 0x4411A0);
    atexit(0x856110);
}

/// `__init_CGarages__aGarages` (0x849E90): 50 `CGarage` (0xD8) at 0x96C048.
#[hook(addr = 0x849E90, conv = cdecl, name = "__init_CGarages__aGarages", expect = "68 10 71 44 00")]
pub fn init_garages() {
    eh_vector_ctor(g(0x96C048), 0xD8, 0x32, 0x4470E0, 0x447110);
    atexit(0x856130);
}

/// `__init_CSetPieces__aSetPieces` (0x84B1C0): 210 set pieces (0x20): +0x1C = 0, +0 = 0.
#[hook(addr = 0x84B1C0, conv = cdecl, name = "__init_CSetPieces__aSetPieces", expect = "B8 18 58 A9 00")]
pub fn init_set_pieces() {
    for i in 0..0xD2usize {
        let p = g::<u8>(0xA95818 + i * 0x20);
        p.byte_add(0x1C).write(0);
        p.cast::<u32>().write(0);
    }
}

/// `__init_gStoredClothesState` (0x84B2A0): `CPedClothesDesc` constructor on 0xA9A810 (tail jump).
#[hook(addr = 0x84B2A0, conv = cdecl, name = "__init_gStoredClothesState", expect = "B9 10 A8 A9 00")]
pub fn init_stored_clothes_state() -> GPtr<u8> {
    ped_clothes_desc_ctor(g(0xA9A810))
}

/// `__init_CWorld__Players` (0x84E980): 2 `CPlayerInfo` (0x190) at 0xB7CD98.
#[hook(addr = 0x84E980, conv = cdecl, name = "__init_CWorld__Players", expect = "68 10 B1 45 00")]
pub fn init_players() {
    eh_vector_ctor(g(0xB7CD98), 0x190, 2, 0x571920, 0x45B110);
    atexit(0x856500);
}

/// `__init_CWorld__ms_aSectors` (0x84E9B0): 14400 `CSector` (8) at 0xB7D0B8.
#[hook(addr = 0x84E9B0, conv = cdecl, name = "__init_CWorld__ms_aSectors", expect = "68 30 31 56 00")]
pub fn init_sectors() {
    eh_vector_ctor(g(0xB7D0B8), 8, 0x3840, 0x564040, 0x563130);
    atexit(0x856520);
}

/// `__init_CWorld__ms_aRepeatSectors` (0x84E9E0): 256 `CRepeatSector` (0xC) at 0xB992B8.
#[hook(addr = 0x84E9E0, conv = cdecl, name = "__init_CWorld__ms_aRepeatSectors", expect = "68 B0 31 56 00")]
pub fn init_repeat_sectors() {
    eh_vector_ctor(g(0xB992B8), 0xC, 0x100, 0x563190, 0x5631B0);
    atexit(0x856540);
}

/// `__init_CWorld__ms_aLodPtrLists` (0x84EA10): 900 `CPtrListSingleLink` (4) at 0xB99EB8.
#[hook(addr = 0x84EA10, conv = cdecl, name = "__init_CWorld__ms_aLodPtrLists", expect = "68 A0 E1 43 00")]
pub fn init_lod_ptr_lists() {
    eh_vector_ctor(g(0xB99EB8), 4, 0x384, 0x43E3F0, 0x43E1A0);
    atexit(0x8564C0);
}

/// `__init_CWorld__ms_listMovingEntityPtrs` (0x84EA40): `_atexit` of its destructor only.
#[hook(addr = 0x84EA40, conv = cdecl, name = "__init_CWorld__ms_listMovingEntityPtrs", expect = "68 E0 64 85 00")]
pub fn init_list_moving_entity_ptrs() {
    atexit(0x8564E0);
}

/// `__init_CWorld__ms_listObjectsWithControlCode` (0x84EA50): `_atexit` of its destructor only.
#[hook(addr = 0x84EA50, conv = cdecl, name = "__init_CWorld__ms_listObjectsWithControlCode", expect = "68 F0 64 85 00")]
pub fn init_list_objects_with_control_code() {
    atexit(0x8564F0);
}

/// `__init_CEscalators__aEscalators` (0x854D50): 32 `CEscalator` (0x150) at 0xC6E9A8.
#[hook(addr = 0x854D50, conv = cdecl, name = "__init_CEscalators__aEscalators", expect = "68 30 71 71 00")]
pub fn init_escalators() {
    eh_vector_ctor(g(0xC6E9A8), 0x150, 0x20, 0x717110, 0x717130);
    atexit(0x856B20);
}

/// `__dtor_CGameLogic__SavedWeapons` (0x856110).
#[hook(addr = 0x856110, conv = cdecl, name = "__dtor_CGameLogic__SavedWeapons", expect = "68 A0 11 44 00")]
pub fn dtor_saved_weapons() {
    eh_vector_dtor(g(0x96A9B8), 0x1C, 0xD, 0x4411A0);
}

/// `__dtor_CGarages__aGarages` (0x856130).
#[hook(addr = 0x856130, conv = cdecl, name = "__dtor_CGarages__aGarages", expect = "68 10 71 44 00")]
pub fn dtor_garages() {
    eh_vector_dtor(g(0x96C048), 0xD8, 0x32, 0x447110);
}

/// `__dtor_CWorld__ms_aLodPtrLists` (0x8564C0).
#[hook(addr = 0x8564C0, conv = cdecl, name = "__dtor_CWorld__ms_aLodPtrLists", expect = "68 A0 E1 43 00")]
pub fn dtor_lod_ptr_lists() {
    eh_vector_dtor(g(0xB99EB8), 4, 0x384, 0x43E1A0);
}

/// `__dtor_CWorld__ms_listMovingEntityPtrs` (0x8564E0): `CPtrListDoubleLink::Flush` (tail jump).
#[hook(addr = 0x8564E0, conv = cdecl, name = "__dtor_CWorld__ms_listMovingEntityPtrs", expect = "B9 C8 AC B9 00")]
pub fn dtor_list_moving_entity_ptrs() {
    ptr_list_double_link_flush(g(0xB9ACC8));
}

/// `__dtor_CWorld__ms_listObjectsWithControlCode` (0x8564F0): `CPtrListDoubleLink::Flush` (tail jump).
#[hook(addr = 0x8564F0, conv = cdecl, name = "__dtor_CWorld__ms_listObjectsWithControlCode", expect = "B9 CC AC B9 00")]
pub fn dtor_list_objects_with_control_code() {
    ptr_list_double_link_flush(g(0xB9ACCC));
}

/// `__dtor_CWorld__Players` (0x856500).
#[hook(addr = 0x856500, conv = cdecl, name = "__dtor_CWorld__Players", expect = "68 10 B1 45 00")]
pub fn dtor_players() {
    eh_vector_dtor(g(0xB7CD98), 0x190, 2, 0x45B110);
}

/// `__dtor_CWorld__ms_aSectors` (0x856520).
#[hook(addr = 0x856520, conv = cdecl, name = "__dtor_CWorld__ms_aSectors", expect = "68 30 31 56 00")]
pub fn dtor_sectors() {
    eh_vector_dtor(g(0xB7D0B8), 8, 0x3840, 0x563130);
}

/// `__dtor_CWorld__ms_aRepeatSectors` (0x856540).
#[hook(addr = 0x856540, conv = cdecl, name = "__dtor_CWorld__ms_aRepeatSectors", expect = "68 B0 31 56 00")]
pub fn dtor_repeat_sectors() {
    eh_vector_dtor(g(0xB992B8), 0xC, 0x100, 0x5631B0);
}

/// `__dtor_CEscalators__aEscalators` (0x856B20).
#[hook(addr = 0x856B20, conv = cdecl, name = "__dtor_CEscalators__aEscalators", expect = "68 30 71 71 00")]
pub fn dtor_escalators() {
    eh_vector_dtor(g(0xC6E9A8), 0x150, 0x20, 0x717130);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_G_96A718,
    HOOK_INIT_G_96A71C,
    HOOK_INIT_G_97D64C,
    HOOK_INIT_G_97D650,
    HOOK_INIT_SAVED_WEAPONS,
    HOOK_INIT_GARAGES,
    HOOK_INIT_SET_PIECES,
    HOOK_INIT_STORED_CLOTHES_STATE,
    HOOK_INIT_PLAYERS,
    HOOK_INIT_SECTORS,
    HOOK_INIT_REPEAT_SECTORS,
    HOOK_INIT_LOD_PTR_LISTS,
    HOOK_INIT_LIST_MOVING_ENTITY_PTRS,
    HOOK_INIT_LIST_OBJECTS_WITH_CONTROL_CODE,
    HOOK_INIT_ESCALATORS,
    HOOK_DTOR_SAVED_WEAPONS,
    HOOK_DTOR_GARAGES,
    HOOK_DTOR_LOD_PTR_LISTS,
    HOOK_DTOR_LIST_MOVING_ENTITY_PTRS,
    HOOK_DTOR_LIST_OBJECTS_WITH_CONTROL_CODE,
    HOOK_DTOR_PLAYERS,
    HOOK_DTOR_SECTORS,
    HOOK_DTOR_REPEAT_SECTORS,
    HOOK_DTOR_ESCALATORS,
];
