//! SAVE-1: the world blocks of the save game (gta_sa.exe 1.0 US): `CGarages` (+
//! `CloseHideOutGaragesBeforeSave`), `CGameLogic`, `CPickups`, `CRestart`, `CStats`, `CSetPieces`,
//! `CShopping`, `CStuntJumpManager`, `CEntryExitManager` `Save` / `Load`. Ported from the bytes (objdump;
//! gta-reversed as a reading aid); docs/arch/save-port.md.
//!
//! Each writes its statics through `CGenericGameStorage::SaveDataToWorkBuffer` / reads them back through
//! `LoadDataFromWorkBuffer` in the bytes' order, with the bytes' loop bounds (counts re-read on every test).
//! `CGarages::Save` builds each `CSaveGarage` in an uninitialised 0x50-byte local that
//! `CopyGarageIntoSaveGarage` fills only partly: its other bytes are the stack residue at entry ESP -0x50..0,
//! taken from the footprint snapshot (ADR-001 §5b); the iterations reuse the same local.

use sa_core::abi::{HookReg, footprint_residue_u8};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

fn g<T>(a: usize) -> GPtr<T> {
    GPtr::from_addr(a)
}

fn save(a: usize, n: i32) {
    save_data(g(a), n);
}

fn load(a: usize, n: i32) {
    load_data(g(a), n);
}

orig! {
    /// `CGenericGameStorage::SaveDataToWorkBuffer(void*, int32)` (0x5D1270)
    fn save_data(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1270 => sa_core::save::save_data_to_work_buffer;
    /// `CGenericGameStorage::LoadDataFromWorkBuffer(void*, int32)` (0x5D1300)
    fn load_data(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1300 => sa_core::save::load_data_from_work_buffer;
    /// `CGarages::CloseHideOutGaragesBeforeSave()` (0x44A170)
    fn close_hide_out_garages_before_save_call() = cdecl 0x44A170 => close_hide_out_garages_before_save;
    /// `CGarages::FindSafeHouseIndexForGarageType(eGarageType)` (0x4489F0)
    fn find_safe_house_index_for_garage_type(ty: u32) -> i32 = cdecl 0x4489F0 => unported;
    /// `CGarage::StoreAndRemoveCarsForThisHideOut(CStoredCar*, int32)` (0x449900)
    fn store_and_remove_cars_for_this_hide_out(garage: GPtr<u8>, cars: GPtr<u8>, max: i32) = thiscall 0x449900 => unported;
    /// `CGarage::RemoveCarsBlockingDoorNotInside()` (0x449690)
    fn remove_cars_blocking_door_not_inside(garage: GPtr<u8>) = thiscall 0x449690 => unported;
    /// `CSaveGarage::CopyGarageIntoSaveGarage(const CGarage&)` (0x5D3020)
    fn copy_garage_into_save_garage(s: GPtr<u8>, garage: GPtr<u8>) = thiscall 0x5D3020 => super::misc2::copy_garage_into_save_garage;
    /// `CSaveGarage::CopyGarageOutOfSaveGarage(CGarage&)` (0x5D30C0)
    fn copy_garage_out_of_save_garage(s: GPtr<u8>, garage: GPtr<u8>) = thiscall 0x5D30C0 => super::misc2::copy_garage_out_of_save_garage;
    /// `CRestart::Initialise()` (0x460630)
    fn restart_initialise() = cdecl 0x460630 => unported;
    /// `CStats::GetStatValue(uint16)` (0x558E40, ST0)
    fn stats_get_stat_value(stat: u32) -> f32 = cdecl 0x558E40 => unported;
    /// `CStats::IncrementStat(uint16, float)` (0x55C180)
    fn stats_increment_stat(stat: u32, value: f32) = cdecl 0x55C180 => unported;
    /// `CStats::SetStatValue(uint16, float)` (0x55A070)
    fn stats_set_stat_value(stat: u32, value: f32) = cdecl 0x55A070 => unported;
    /// `CStats::PopulateFavoriteRadioStationList()` (0x558EC0)
    fn stats_populate_favorite_radio_station_list() = cdecl 0x558EC0 => unported;
    /// `CPool<CStuntJump>::New()` (0x49C9E0)
    fn stunt_jump_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x49C9E0 => unported;
}

// ------------------------------------------------------------------------------- CGarages -------

/// `CGarages::NumGarages`
const NUM_GARAGES: usize = 0x96C024;
/// `CGarages::aGarages` (0xD8 each)
const GARAGES: usize = 0x96C048;
/// `CGarages::aCarsInSafeHouse` (4 x 20 `CStoredCar`, 0x40 each; the save loops walk 4 columns of 0x40
/// with 20 rows of 0x100)
const CARS_IN_SAFE_HOUSE: usize = 0x96ABD8;
/// The footprint window of `CGarages::Save`.
const GARAGES_SAVE_WINDOW: u32 = 0x1000;

/// The garage statics both directions walk (address, size), in the bytes' order.
const GARAGE_STATICS: [(usize, i32); 11] = [
    (NUM_GARAGES, 4),
    (0x96C00A, 1),
    (0x96C009, 1),
    (0x96C008, 1),
    (0x96C004, 4),
    (0x96C000, 4),
    (0x96BFFC, 4),
    (0x96BFEC, 4),
    (0x96BFF0, 4),
    (0x96BFF4, 4),
    (0x96BFF8, 4),
];

/// `CGarages::Save()` (0x5D3160): `CloseHideOutGaragesBeforeSave()`; the statics (`NumGarages` 4, the bytes
/// 0x96C00A / 0x96C009 / 0x96C008, the dwords 0x96C004 / 0x96C000 / 0x96BFFC, the four dwords 0x96BFEC..,
/// 0x96BFE0); the stored cars (0x40 each, outer loop over 4 columns of 0x40, inner over 20 rows of 0x100);
/// then per garage (`i < NumGarages`, unsigned, re-read) a `CSaveGarage` built in the 0x50-byte local and
/// written whole (its unwritten bytes: residue, see the module doc). AL = 1.
#[hook(addr = 0x5D3160, conv = cdecl, name = "CGarages::Save", expect = "83 EC 50 56 57",
       unverified = "save-world-1", route = "split", hazards = "uninit-locals", recs = "save-load", footprint = 0x1000)]
pub fn garages_save() -> u8 {
    close_hide_out_garages_before_save_call();
    for (a, n) in GARAGE_STATICS {
        save(a, n);
    }
    save(0x96BFE0, 4);
    for col in 0..4usize {
        for row in 0..20usize {
            save(CARS_IN_SAFE_HOUSE + col * 0x40 + row * 0x100, 0x40);
        }
    }
    let mut local = [0u8; 0x50];
    for (k, b) in local.iter_mut().enumerate() {
        *b = footprint_residue_u8(k as i32 - 0x50, GARAGES_SAVE_WINDOW);
    }
    let lp = GPtr::<u8>::from_raw(local.as_mut_ptr());
    let mut i = 0u32;
    while i < g::<u32>(NUM_GARAGES).read() {
        copy_garage_into_save_garage(lp, g(GARAGES + i as usize * 0xD8));
        save_data(lp, 0x50);
        i += 1;
    }
    1
}

/// `CGarages::Load()` (0x5D3270): `CloseHideOutGaragesBeforeSave()`, the same statics and stored cars read
/// back, per garage the `CSaveGarage` read and copied out; then the dwords 0x96C01C / 0x96C020 and the byte
/// 0x96BFE4 cleared. AL = 1.
#[hook(addr = 0x5D3270, conv = cdecl, name = "CGarages::Load", expect = "83 EC 50 53 56 57",
       unverified = "save-world-1", route = "split", hazards = "none", recs = "save-load")]
pub fn garages_load() -> u8 {
    close_hide_out_garages_before_save_call();
    for (a, n) in GARAGE_STATICS {
        load(a, n);
    }
    load(0x96BFE0, 4);
    for col in 0..4usize {
        for row in 0..20usize {
            load(CARS_IN_SAFE_HOUSE + col * 0x40 + row * 0x100, 0x40);
        }
    }
    let mut local = [0u8; 0x50];
    let lp = GPtr::<u8>::from_raw(local.as_mut_ptr());
    let mut i = 0u32;
    while i < g::<u32>(NUM_GARAGES).read() {
        load_data(lp, 0x50);
        copy_garage_out_of_save_garage(lp, g(GARAGES + i as usize * 0xD8));
        i += 1;
    }
    g::<u32>(0x96C01C).write(0);
    g::<u32>(0x96C020).write(0);
    g::<u8>(0x96BFE4).write(0);
    1
}

/// `CGarages::CloseHideOutGaragesBeforeSave()` (0x44A170): every garage (50, 0xD8 each) of a hide-out type
/// (0x10..0x12, 0x18..0x20, 0x27..0x2A, 0x2D; type byte +0x4C) whose door state byte +0x4D is set: +0x4D = 0,
/// `StoreAndRemoveCarsForThisHideOut(&aCarsInSafeHouse[FindSafeHouseIndexForGarageType(type)], 4)`,
/// `RemoveCarsBlockingDoorNotInside()`, +0x38 = 0.
#[hook(addr = 0x44A170, conv = cdecl, name = "CGarages::CloseHideOutGaragesBeforeSave", expect = "53 56 57 BF 94 C0 96 00",
       unverified = "save-world-1", route = "split", hazards = "heap-writes", recs = "save-load")]
pub fn close_hide_out_garages_before_save() {
    for k in 0..50usize {
        let garage = g::<u8>(GARAGES + k * 0xD8);
        let ty = garage.byte_add(0x4C).read();
        let hide_out = matches!(ty, 0x10..=0x12 | 0x18..=0x20 | 0x27..=0x2A | 0x2D);
        if !hide_out || garage.byte_add(0x4D).read() == 0 {
            continue;
        }
        garage.byte_add(0x4D).write(0);
        let idx = find_safe_house_index_for_garage_type(u32::from(ty));
        let cars = g::<u8>((idx as u32).wrapping_shl(8).wrapping_add(CARS_IN_SAFE_HOUSE as u32) as usize);
        store_and_remove_cars_for_this_hide_out(garage, cars, 4);
        remove_cars_blocking_door_not_inside(garage);
        garage.byte_add(0x38).cast::<u32>().write(0);
    }
}

// ----------------------------------------------------------------------------- CGameLogic ------

/// `CGameLogic::NumAfterDeathStartPoints`
const NUM_AFTER_DEATH_START_POINTS: usize = 0x96A890;

/// `CGameLogic::Save()` (0x5D33C0): the count (4), the bytes 0x8A5E48 / 0x8A5E49 / 0x96A8B0, the dword
/// 0x96A8AC; per start point (`i < count`, signed, re-read) its position (0xC, 0x96A8E0) and heading (4,
/// 0x96A850). AL = 1.
#[hook(addr = 0x5D33C0, conv = cdecl, name = "CGameLogic::Save", expect = "56 6A 04 68 90 A8 96 00",
       unverified = "save-world-1", route = "split", hazards = "none", recs = "save-load")]
pub fn game_logic_save() -> u8 {
    game_logic_blocks(save);
    1
}

/// `CGameLogic::Load()` (0x5D3440): the same layout read back. AL = 1.
#[hook(addr = 0x5D3440, conv = cdecl, name = "CGameLogic::Load", expect = "56 6A 04 68 90 A8 96 00",
       unverified = "save-world-1", route = "split", hazards = "none", recs = "save-load")]
pub fn game_logic_load() -> u8 {
    game_logic_blocks(load);
    1
}

fn game_logic_blocks(io: fn(usize, i32)) {
    io(NUM_AFTER_DEATH_START_POINTS, 4);
    io(0x8A5E48, 1);
    io(0x8A5E49, 1);
    io(0x96A8B0, 1);
    io(0x96A8AC, 4);
    let mut i = 0i32;
    while i < g::<i32>(NUM_AFTER_DEATH_START_POINTS).read() {
        io(0x96A8E0 + 0xC * i as usize, 0xC);
        io(0x96A850 + 4 * i as usize, 4);
        i += 1;
    }
}

// ------------------------------------------------------------------------------- CPickups ------

/// `CPickups::aPickUps` (620 x 0x20)
const PICKUPS: usize = 0x9788C0;
const NUM_PICKUPS: usize = 620;

/// `CPickups::Save()` (0x5D3540): the 620 pickups (0x20 each), `CollectedPickUpIndex` (2, 0x978624), the
/// byte 0x8A5F48, the 20 collected-pickup dwords (0x978628). AL = 1.
#[hook(addr = 0x5D3540, conv = cdecl, name = "CPickups::Save", expect = "56 57 BE C0 88 97 00",
       unverified = "save-world-1", route = "split", hazards = "none", recs = "save-load")]
pub fn pickups_save() -> u8 {
    for k in 0..NUM_PICKUPS {
        save(PICKUPS + k * 0x20, 0x20);
    }
    save(0x978624, 2);
    save(0x8A5F48, 1);
    for k in 0..20usize {
        save(0x978628 + 4 * k, 4);
    }
    1
}

/// `CPickups::Load()` (0x5D35A0): each pickup read, then one with a type (+0x1C) and an object (+4) loses
/// the object (+4 = 0) and bit 3 of its flags (+0x1D); the word 0x978678 = 0; the rest as `Save`. AL = 1.
#[hook(addr = 0x5D35A0, conv = cdecl, name = "CPickups::Load", expect = "56 57 BE C4 88 97 00",
       unverified = "save-world-1", route = "split", hazards = "none", recs = "save-load")]
pub fn pickups_load() -> u8 {
    for k in 0..NUM_PICKUPS {
        let p = g::<u8>(PICKUPS + k * 0x20);
        load_data(p, 0x20);
        if p.byte_add(0x1C).read() != 0 && p.byte_add(4).cast::<u32>().read() != 0 {
            let flags = p.byte_add(0x1D).read() & 0xF7;
            p.byte_add(4).cast::<u32>().write(0);
            p.byte_add(0x1D).write(flags);
        }
    }
    g::<u16>(0x978678).write(0);
    load(0x978624, 2);
    load(0x8A5F48, 1);
    for k in 0..20usize {
        load(0x978628 + 4 * k, 4);
    }
    1
}

// ------------------------------------------------------------------------------- CRestart ------

/// `CRestart::Save()` (0x5D3620): the hospital count (2, 0xA4326C) and per hospital (`i < count`, u16,
/// re-read) position (0xC), heading (4), area (4); the police station count (2, 0xA43268) and its points
/// alike; then `bOverrideRestart` (1), the override position (0xC), the two fade-in bytes, the extra
/// hospital position, radius and heading, the extra police station position, radius and heading. AL = 1.
#[hook(addr = 0x5D3620, conv = cdecl, name = "CRestart::Save", expect = "56 57 6A 02 68 6C 32 A4 00",
       unverified = "save-world-1", route = "split", hazards = "none", recs = "save-load")]
pub fn restart_save() -> u8 {
    restart_blocks(save);
    1
}

/// `CRestart::Load()` (0x5D3770): `CRestart::Initialise()`, then the layout of `Save` read back. AL = 1.
#[hook(addr = 0x5D3770, conv = cdecl, name = "CRestart::Load", expect = "56 57 E8 B9 CE E8 FF",
       unverified = "save-world-1", route = "split", hazards = "none", recs = "save-load")]
pub fn restart_load() -> u8 {
    restart_initialise();
    restart_blocks(load);
    1
}

fn restart_blocks(io: fn(usize, i32)) {
    for (count, pos, heading, area) in [(0xA4326Cusize, 0xA43318usize, 0xA432E8usize, 0xA432C0usize), (0xA43268, 0xA43390, 0xA43298, 0xA43270)] {
        io(count, 2);
        let mut i = 0u16;
        while i < g::<u16>(count).read() {
            let k = usize::from(i);
            io(pos + 0xC * k, 0xC);
            io(heading + 4 * k, 4);
            io(area + 4 * k, 4);
            i = i.wrapping_add(1);
        }
    }
    for (a, n) in [
        (0xA43264usize, 1),
        (0xA43408, 0xC),
        (0xA4325D, 1),
        (0xA4325C, 1),
        (0xA43414, 0xC),
        (0xA43258, 4),
        (0xA43254, 4),
        (0xA43420, 0xC),
        (0xA43250, 4),
        (0xA4324C, 4),
    ] {
        io(a, n);
    }
}

// --------------------------------------------------------------------------------- CStats ------

/// The stat arrays both directions walk (address, size).
const STATS_ARRAYS: [(usize, i32); 6] =
    [(0xB79380, 0x148), (0xB79000, 0x37C), (0xB78E90, 0x80), (0xB78A00, 8), (0xB78E58, 0x38), (0xB78CC8, 0x190)];

/// `CStats::Save()` (0x5D3B40): `IncrementStat(0xB1, GetStatValue(0xB0))`, `SetStatValue(0xB0, 0)`; the
/// float stats (0x148), the int stats (0x37C), 0xB78E90 (0x80), the last mission's key (8, 0xB78A00),
/// `PopulateFavoriteRadioStationList()` between it and 0xB78E58 (0x38), 0xB78CC8 (0x190), then one byte of
/// each of the 128 records at 0xB78202 (stride 0x10). AL = 1.
#[hook(addr = 0x5D3B40, conv = cdecl, name = "CStats::Save", expect = "56 57 68 B0 00 00 00",
       unverified = "save-world-1", route = "split", hazards = "none", recs = "save-load")]
pub fn stats_save() -> u8 {
    let v = stats_get_stat_value(0xB0);
    stats_increment_stat(0xB1, v);
    stats_set_stat_value(0xB0, 0.0);
    for (k, (a, n)) in STATS_ARRAYS.into_iter().enumerate() {
        if k == 4 {
            stats_populate_favorite_radio_station_list();
        }
        save(a, n);
    }
    for k in 0..0x80usize {
        save(0xB78202 + 0x10 * k, 1);
    }
    1
}

/// `CStats::Load()` (0x5D3BF0): the arrays and the 128 record bytes of `Save` read back. AL = 1.
#[hook(addr = 0x5D3BF0, conv = cdecl, name = "CStats::Load", expect = "56 57 68 48 01 00 00",
       unverified = "save-world-1", route = "split", hazards = "none", recs = "save-load")]
pub fn stats_load() -> u8 {
    for (a, n) in STATS_ARRAYS {
        load(a, n);
    }
    for k in 0..0x80usize {
        load(0xB78202 + 0x10 * k, 1);
    }
    1
}

// ----------------------------------------------------------------------------- CSetPieces ------

/// `CSetPieces::Save()` (0x5D3C70): `NumSetPieces` (4, 0xA957FC) and the whole array (0x1A40, 0xA95818).
#[hook(addr = 0x5D3C70, conv = cdecl, name = "CSetPieces::Save", expect = "6A 04 68 FC 57 A9 00",
       unverified = "save-world-1", route = "split", hazards = "none", recs = "save-load")]
pub fn set_pieces_save() -> u8 {
    save(0xA957FC, 4);
    save(0xA95818, 0x1A40);
    1
}

/// `CSetPieces::Load()` (0x5D3CA0): the same read back.
#[hook(addr = 0x5D3CA0, conv = cdecl, name = "CSetPieces::Load", expect = "6A 04 68 FC 57 A9 00",
       unverified = "save-world-1", route = "split", hazards = "none", recs = "save-load")]
pub fn set_pieces_load() -> u8 {
    load(0xA957FC, 4);
    load(0xA95818, 0x1A40);
    1
}

// ------------------------------------------------------------------------------ CShopping ------

/// `CShopping::Save()` (0x5D3DE0): the bought-items count (4, 0xA9A7D0) and its entries (8 each, 0xA98650;
/// `i < count`, unsigned, re-read), the price modifier count (4, 0xA9A310) and that many bytes of 0xA972A0.
#[hook(addr = 0x5D3DE0, conv = cdecl, name = "CShopping::Save", expect = "56 6A 04 68 D0 A7 A9 00",
       unverified = "save-world-1", route = "split", hazards = "none", recs = "save-load")]
pub fn shopping_save() -> u8 {
    shopping_blocks(save);
    1
}

/// `CShopping::Load()` (0x5D3E40): the same read back.
#[hook(addr = 0x5D3E40, conv = cdecl, name = "CShopping::Load", expect = "56 6A 04 68 D0 A7 A9 00",
       unverified = "save-world-1", route = "split", hazards = "none", recs = "save-load")]
pub fn shopping_load() -> u8 {
    shopping_blocks(load);
    1
}

fn shopping_blocks(io: fn(usize, i32)) {
    io(0xA9A7D0, 4);
    let mut i = 0u32;
    while i < g::<u32>(0xA9A7D0).read() {
        io(0xA98650 + 8 * i as usize, 8);
        i += 1;
    }
    io(0xA9A310, 4);
    io(0xA972A0, g::<i32>(0xA9A310).read());
}

// ---------------------------------------------------------------------- CStuntJumpManager ------

/// `CStuntJumpManager::mp_poolStuntJumps` (`CPool<CStuntJump>*`, 0x100 x 0x44)
const STUNT_JUMP_POOL: usize = 0xA9A888;
/// `CStuntJumpManager::m_iNumJumps`
const NUM_JUMPS: usize = 0xA9A89C;

/// `CStuntJumpManager::Save()` (0x5D5570): `m_iNumJumps` (4), then every used pool slot (flag byte not
/// negative; 256 slots) as 0x44 bytes (the pool pointer re-read after each write). AL = 1.
#[hook(addr = 0x5D5570, conv = cdecl, name = "CStuntJumpManager::Save", expect = "56 57 6A 04 68 9C A8 A9 00",
       unverified = "save-world-1", route = "split", hazards = "heap-writes", recs = "save-load")]
pub fn stunt_jumps_save() -> u8 {
    save(NUM_JUMPS, 4);
    for k in 0..0x100usize {
        let pool = g::<GPtr<u8>>(STUNT_JUMP_POOL).read();
        let flags = pool.byte_add(4).cast::<GPtr<u8>>().read();
        if (flags.byte_add(k as isize).read() as i8) < 0 {
            continue;
        }
        let objs = pool.cast::<GPtr<u8>>().read();
        save_data(objs.byte_add((k * 0x44) as isize), 0x44);
    }
    1
}

/// `CStuntJumpManager::Load()` (0x5D5920): the count into a local, then that many jumps, each read into a
/// new pool slot (`CPool::New`); `m_iNumJumps` = the count. AL = 1.
#[hook(addr = 0x5D5920, conv = cdecl, name = "CStuntJumpManager::Load", expect = "51 56 8D 44 24 04",
       unverified = "save-world-1", route = "split", hazards = "heap-writes", recs = "save-load")]
pub fn stunt_jumps_load() -> u8 {
    let mut count = 0i32;
    load_data(GPtr::from_raw(&mut count).cast(), 4);
    let mut i = 0i32;
    while i < count {
        let p = stunt_jump_pool_new(g::<GPtr<u8>>(STUNT_JUMP_POOL).read());
        load_data(p, 0x44);
        i += 1;
    }
    g::<i32>(NUM_JUMPS).write(count);
    1
}

// ---------------------------------------------------------------------- CEntryExitManager ------

/// `CEntryExitManager::ms_numVisibleEntries`
const NUM_VISIBLE_ENTRIES: usize = 0x96A7C4;
/// `CEntryExitManager::ms_visibleEntryExits` (`CEntryExit*[]`)
const VISIBLE_ENTRY_EXITS: usize = 0x96A720;
/// `CEntryExitManager::mp_poolEntryExits` (`CPool<CEntryExit>*`, 0x190 x 0x3C)
const ENTRY_EXIT_POOL: usize = 0x96A7D8;

/// `(p - objects) / 0x3C` as the bytes compute it (`imul 0x88888889`, `sar 5`, rounded toward zero).
fn entry_exit_index(p: u32, objects: u32) -> i32 {
    let d = p.wrapping_sub(objects) as i32;
    let hi = ((i64::from(d) * i64::from(0x8888_8889u32 as i32)) >> 32) as i32;
    let q = hi.wrapping_add(d) >> 5;
    q.wrapping_add((q as u32 >> 31) as i32)
}

/// The pool slot for a saved index: null when the slot's flag byte is negative (free), else `objects +
/// 0x3C * idx` (no bounds check).
fn entry_exit_at(idx: i32) -> u32 {
    let pool = g::<GPtr<u8>>(ENTRY_EXIT_POOL).read();
    let flags = pool.byte_add(4).cast::<GPtr<u8>>().read();
    if (flags.byte_add(idx as isize).read() as i8) < 0 {
        return 0;
    }
    pool.cast::<u32>().read().wrapping_add((idx as u32).wrapping_mul(0x3C))
}

/// `CEntryExitManager::Save()` (0x5D5970): `ms_numVisibleEntries` (4) and each visible entry's pool index (2;
/// `i < count`, signed, re-read); then for every used pool slot (0x190, flag byte not negative, non-null):
/// its index (2), its flags (2, +0x30) and its link's index (2; -1 for none, a link outside the pool or a
/// free slot); the list ends with an index of -1 (2). The indices are dword locals of which 2 bytes are
/// written. AL = 1.
#[hook(addr = 0x5D5970, conv = cdecl, name = "CEntryExitManager::Save", expect = "83 EC 08 53 55 56 57",
       unverified = "save-world-1", route = "split", hazards = "none", recs = "save-load")]
pub fn entry_exits_save() -> u8 {
    let mut idx = [0i32; 2];
    let ip = GPtr::<u8>::from_raw(idx.as_mut_ptr().cast());
    save(NUM_VISIBLE_ENTRIES, 4);
    let mut i = 0i32;
    while i < g::<i32>(NUM_VISIBLE_ENTRIES).read() {
        let objects = g::<GPtr<u32>>(ENTRY_EXIT_POOL).read().read();
        let p = g::<u32>(VISIBLE_ENTRY_EXITS + 4 * i as usize).read();
        ip.cast::<i32>().write(entry_exit_index(p, objects));
        save_data(ip, 2);
        i += 1;
    }
    let mut pool = g::<GPtr<u8>>(ENTRY_EXIT_POOL).read();
    for k in 0..0x190i32 {
        let flags = pool.byte_add(4).cast::<GPtr<u8>>().read();
        if (flags.byte_add(k as isize).read() as i8) < 0 {
            continue;
        }
        let ee = pool.cast::<u32>().read().wrapping_add((k as u32) * 0x3C);
        if ee == 0 {
            continue;
        }
        ip.cast::<i32>().write(k);
        let link = g::<u32>(ee as usize + 0x38).read();
        let mut li = -1i32;
        if link != 0 {
            let l = entry_exit_index(link, pool.cast::<u32>().read());
            let size = pool.byte_add(8).cast::<i32>().read();
            if l >= 0 && l < size && (flags.byte_add(l as isize).read() as i8) >= 0 {
                li = l;
            }
        }
        ip.byte_add(4).cast::<i32>().write(li);
        save_data(ip, 2);
        save_data(g(ee as usize + 0x30), 2);
        save_data(ip.byte_add(4), 2);
        pool = g::<GPtr<u8>>(ENTRY_EXIT_POOL).read();
    }
    ip.cast::<i32>().write(-1);
    save_data(ip, 2);
    1
}

/// `CEntryExitManager::Load()` (0x5D55C0): `ms_numVisibleEntries` (4), each visible entry from its index
/// (2, sign-extended; [`entry_exit_at`]); then entries until an index of -1: its flags (2) and link index (2)
/// are read; a used slot gets the flags (+0x30) and its link (+0x38: the link's slot, or null for -1). AL = 1.
#[hook(addr = 0x5D55C0, conv = cdecl, name = "CEntryExitManager::Load", expect = "83 EC 0C 53 56 6A 04",
       unverified = "save-world-1", route = "split", hazards = "heap-writes", recs = "save-load")]
pub fn entry_exits_load() -> u8 {
    let mut idx = [0u16; 2];
    let mut flags = [0u16; 2];
    let mut link = [0u16; 2];
    let ip = GPtr::<u8>::from_raw(idx.as_mut_ptr().cast());
    load(NUM_VISIBLE_ENTRIES, 4);
    let mut i = 0i32;
    while i < g::<i32>(NUM_VISIBLE_ENTRIES).read() {
        load_data(ip, 2);
        let p = entry_exit_at(i32::from(ip.cast::<i16>().read()));
        g::<u32>(VISIBLE_ENTRY_EXITS + 4 * i as usize).write(p);
        i += 1;
    }
    load_data(ip, 2);
    while ip.cast::<u16>().read() != 0xFFFF {
        let ee = entry_exit_at(i32::from(ip.cast::<i16>().read()));
        load_data(GPtr::from_raw(flags.as_mut_ptr().cast()), 2);
        load_data(GPtr::from_raw(link.as_mut_ptr().cast()), 2);
        if ee != 0 {
            g::<u16>(ee as usize + 0x30).write(flags[0]);
            let l = if link[0] == 0xFFFF { 0 } else { entry_exit_at(i32::from(link[0] as i16)) };
            g::<u32>(ee as usize + 0x38).write(l);
        }
        load_data(ip, 2);
    }
    1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GARAGES_SAVE,
    HOOK_GARAGES_LOAD,
    HOOK_CLOSE_HIDE_OUT_GARAGES_BEFORE_SAVE,
    HOOK_GAME_LOGIC_SAVE,
    HOOK_GAME_LOGIC_LOAD,
    HOOK_PICKUPS_SAVE,
    HOOK_PICKUPS_LOAD,
    HOOK_RESTART_SAVE,
    HOOK_RESTART_LOAD,
    HOOK_STATS_SAVE,
    HOOK_STATS_LOAD,
    HOOK_SET_PIECES_SAVE,
    HOOK_SET_PIECES_LOAD,
    HOOK_SHOPPING_SAVE,
    HOOK_SHOPPING_LOAD,
    HOOK_STUNT_JUMPS_SAVE,
    HOOK_STUNT_JUMPS_LOAD,
    HOOK_ENTRY_EXITS_SAVE,
    HOOK_ENTRY_EXITS_LOAD,
];

#[cfg(test)]
mod tests {
    use super::entry_exit_index;

    #[test]
    fn entry_exit_index_matches_division() {
        for k in [0i32, 1, 7, 0x18F, -1, -5] {
            assert_eq!(entry_exit_index((0x1000_0000 + k * 0x3C) as u32, 0x1000_0000), k);
        }
    }
}
