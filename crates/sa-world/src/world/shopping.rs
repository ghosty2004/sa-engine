//! `CShopping` leaves (PORT-WORLD). Ported from the 1.0 US bytes (gta-reversed `Shopping.cpp`,
//! `docs/re/specs/WORLD/CShopping*.md` as reading aids).
//!
//! Price items (`ms_prices`, 0xA986F0, 0x18 each, `ms_numPrices` at 0xA9A7CC): +0 key, +4 price, +8/+0xC
//! extra info, +0x10 name tag. Bought flags `ms_bHasBought` (0xA972A0, one byte per key of
//! `ms_keys`, 0xA97D90, 560 keys).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{ftol32, g, rd, wr, x, xi};
use super::cworld::{PLAYERS, PLAYER_INFO_SIZE, PLAYER_IN_FOCUS};

const NUM_KEYS: i32 = 0x230;
const KEYS: usize = 0xA97D90;
const HAS_BOUGHT: usize = 0xA972A0;
const PRICES: usize = 0xA986F0;
const NUM_PRICES: usize = 0xA9A7CC;
const SECTION_NAMES: usize = 0x8A61D8;
const PRICE_MODIFIERS: usize = 0x8A6204;

sa_core::globals! {
    list GLOBALS;
    /// `CShopping::ms_currentProperty`... (0xA97294)
    pub static CURRENT_PROPERTY @ 0xA97294: i32 = 0;
    /// `CShopping::ms_bClothesStored`... (0xA97298)
    pub static CLOTHES_STORED @ 0xA97298: u8 = 0;
    /// `gStoredClothesState` (`CPedClothesDesc`, 0x78 bytes)
    pub static STORED_CLOTHES @ 0xA9A810: [u32; 30] = [0; 30];
    /// `CShopping::ms_numBuyableItems` (0xA9A310)
    pub static NUM_BUYABLE_ITEMS @ 0xA9A310: i32 = 0;
}

/// `CShopping::ms_statModifiers` (0xA974D0): 4 bytes per buyable item (stat, change, stat, change).
const STAT_MODIFIERS: usize = 0xA974D0;
/// `"section"` (0x85A514)
const STR_SECTION: usize = 0x85A514;
/// `" \t"` (0x85A510): the section-line token delimiters
const STR_SECTION_DELIMS: usize = 0x85A510;
/// `" \t,"` (0x85A51C): the stats-line token delimiters
const STR_STATS_DELIMS: usize = 0x85A51C;
/// `"prices"` (0x85A520)
const STR_PRICES: usize = 0x85A520;
/// `"data\shopping.dat"` (0x85A528)
const STR_SHOPPING_DAT: usize = 0x85A528;
/// `"r"` (0x85A53C)
const STR_MODE_R: usize = 0x85A53C;

pub use sa_core::bindings::stricmp;
use sa_core::bindings::atol;
use sa_core::bindings::file_close as close_file;
use sa_core::bindings::open_file;
use sa_core::bindings::strncmp;
use sa_core::bindings::strtok;
use crate::bindings::find_weapon_type;
use sa_core::bindings::get_model_info;
use sa_core::bindings::get_uppercase_key;
use sa_stream::bindings::load_line;

/// `line` starts with `"end"`.
fn is_end(line: GPtr<u8>) -> bool {
    line.read() == b'e' && line.byte_add(1).read() == b'n' && line.byte_add(2).read() == b'd'
}

fn key_index(key: u32) -> i32 {
    for i in 0..NUM_KEYS {
        if g::<u32>(KEYS).wrapping_index(i).read() == key {
            return i;
        }
    }
    -1
}

fn find_item_index(key: u32) -> i32 {
    let n = g::<i32>(NUM_PRICES).read();
    for i in 0..n.max(0) {
        if g::<u32>(PRICES + i as usize * 0x18).read() == key {
            return i;
        }
    }
    -1
}

fn item(i: i32) -> GPtr<u8> {
    g::<u8>(PRICES).byte_add(i as isize * 0x18)
}

/// `CShopping::GetPriceSectionFromName(const char*)` (0x49AAD0): `_stricmp` against the 11 section names.
#[hook(addr = 0x49AAD0, conv = cdecl, name = "CShopping::GetPriceSectionFromName", expect = "56 57 8B 7C 24 0C")]
pub fn get_price_section_from_name(name: GPtr<u8>) -> i32 {
    for i in 0..0xB {
        if stricmp(name, g::<GPtr<u8>>(SECTION_NAMES).wrapping_index(i).read()) == 0 {
            return i;
        }
    }
    -1
}

/// `CShopping::GetItemIndex(uint32 key)` (stub 0x49AB10 → 0x156E330): the key's index or -1.
#[hook(addr = 0x49AB10, conv = cdecl, name = "CShopping::GetItemIndex", expect = "E9 1B 38 0D 01")]
pub fn get_item_index(key: u32) -> i32 {
    key_index(key)
}

/// `GetChangingStatIndex(const char*)` (stub 0x49ABD0 → 0x1565360): "-" → -1, "fat" 0, "respect" 1,
/// "sexy" 2, "health" 3, "stamina" 4, "calories" 5, else -1 (`repe cmpsb` over the literal including
/// its NUL).
#[hook(addr = 0x49ABD0, conv = cdecl, name = "GetChangingStatIndex", expect = "E9 8B A7 0C 01")]
pub fn get_changing_stat_index(name: GPtr<u8>) -> i32 {
    let eq = |lit: &[u8]| {
        for (k, &c) in lit.iter().enumerate() {
            if name.byte_add(k as isize).read() != c {
                return false;
            }
        }
        true
    };
    if eq(b"-\0") {
        return -1;
    }
    for (i, lit) in [&b"fat\0"[..], b"respect\0", b"sexy\0", b"health\0", b"stamina\0"].iter().enumerate() {
        if eq(lit) {
            return i as i32;
        }
    }
    if eq(b"calories\0") { 5 } else { -1 }
}

/// `CShopping::FindItem(uint32 key)` (0x49AD20): index in the loaded prices or -1.
#[hook(addr = 0x49AD20, conv = cdecl, name = "CShopping::FindItem", expect = "8B 15 CC A7 A9 00")]
pub fn find_item(key: u32) -> i32 {
    find_item_index(key)
}

/// `CShopping::GetPrice(uint32 key)` (0x49AD50): `_ftol(price * modifier[CTheZones::m_CurrLevel])` (index
/// -1 reads the slot before the table, original).
#[hook(addr = 0x49AD50, conv = cdecl, name = "CShopping::GetPrice", expect = "8B 15 CC A7 A9 00")]
pub fn get_price(key: u32) -> i32 {
    let i = find_item_index(key);
    let lvl = super::zones::CURR_LEVEL.get();
    let m = g::<f32>(PRICE_MODIFIERS).wrapping_index(lvl).read();
    ftol32(xi(rd::<i32>(item(i), 4)).fmul(x(m)))
}

/// `CShopping::GetNameTag(uint32 key)` (stub 0x49ADA0 → 0x156F0D0): `&item.nameTag`.
#[hook(addr = 0x49ADA0, conv = cdecl, name = "CShopping::GetNameTag", expect = "E9 2B 43 0D 01")]
pub fn get_name_tag(key: u32) -> GPtr<u8> {
    item(find_item_index(key)).byte_add(0x10)
}

/// `CShopping::GetExtraInfo(uint32 key, int32 which)` (stub 0x49ADE0 → 0x156F080): extra 0 / 1, else 0.
#[hook(addr = 0x49ADE0, conv = cdecl, name = "CShopping::GetExtraInfo", expect = "E9 9B 42 0D 01")]
pub fn get_extra_info(key: u32, which: i32) -> i32 {
    let i = find_item_index(key);
    match which {
        0 => rd::<i32>(item(i), 8),
        1 => rd::<i32>(item(i), 0xC),
        _ => 0,
    }
}

/// `CShopping::SetCurrentProperty(int32)` (0x49B1F0).
#[hook(addr = 0x49B1F0, conv = cdecl, name = "CShopping::SetCurrentProperty", expect = "8B 44 24 04 A3 94 72 A9 00")]
pub fn set_current_property(p: i32) {
    CURRENT_PROPERTY.set(p);
}

fn player_clothes() -> GPtr<u32> {
    let ped = g::<GPtr<u8>>(PLAYERS + usize::from(PLAYER_IN_FOCUS.get()) * PLAYER_INFO_SIZE).read();
    rd::<GPtr<u32>>(rd::<GPtr<u8>>(ped, 0x480), 4)
}

/// `CShopping::StoreClothesState` (stub 0x49B200 → 0x156E2F0): copies the focus player's clothes
/// description (0x78 bytes) aside; flag = 1.
#[hook(addr = 0x49B200, conv = cdecl, name = "CShopping::StoreClothesState", expect = "E9 EB 30 0D 01")]
pub fn store_clothes_state() {
    let src = player_clothes();
    let dst = STORED_CLOTHES.ptr().cast::<u32>();
    for k in 0..30 {
        dst.wrapping_index(k).write(src.wrapping_index(k).read());
    }
    CLOTHES_STORED.set(1);
}

/// `CShopping::RestoreClothesState` (stub 0x49B240 → 0x1565320): copies it back; flag = 0.
#[hook(addr = 0x49B240, conv = cdecl, name = "CShopping::RestoreClothesState", expect = "E9 DB A0 0C 01")]
pub fn restore_clothes_state() {
    let dst = player_clothes();
    let src = STORED_CLOTHES.ptr().cast::<u32>();
    for k in 0..30 {
        dst.wrapping_index(k).write(src.wrapping_index(k).read());
    }
    CLOTHES_STORED.set(0);
}

/// `CShopping::HasPlayerBought(uint32 key)` (stub 0x49B5E0 → 0x156B770): the bought byte of the key
/// (an unknown key reads the byte before the table).
#[hook(addr = 0x49B5E0, conv = cdecl, name = "CShopping::HasPlayerBought", expect = "E9 8B 01 0D 01")]
pub fn has_player_bought(key: u32) -> u8 {
    g::<u8>(HAS_BOUGHT).byte_add(key_index(key) as isize).read()
}

/// `CShopping::SetPlayerHasBought(uint32 key)` (stub 0x49B610 → 0x156F3D0): bought byte = 1 (an unknown
/// key writes the byte before the table, original).
#[hook(addr = 0x49B610, conv = cdecl, name = "CShopping::SetPlayerHasBought", expect = "E9 BB 3D 0D 01")]
pub fn set_player_has_bought(key: u32) {
    g::<u8>(HAS_BOUGHT).byte_add(key_index(key) as isize).write(1);
}

/// `CShopping::FindSection(FILESTREAM file, const char* name)` (stub 0x49AE70 → 0x15659E0, cdecl). Reads
/// lines (`LoadLine`), skipping empty ones and `#` comments. A line starting with `"section"` (strncmp 7)
/// raises the nesting depth and is split with `strtok(line, " \t")`, `strtok(NULL, " \t")`; at depth 1
/// the second token compared `_stricmp` with `name` equal → AL = 1 (EAX = 1). Another line starting with
/// "end" lowers the depth; below 0 → AL = 0. End of file → EAX = 0. The "end" exit leaves the
/// `strncmp` result in EAX's upper bytes (reproduced).
#[hook(addr = 0x49ae70, conv = cdecl, name = "CShopping::FindSection", expect = "E9 6B AB 0C 01",
       unverified = "mvpj-world-1", route = "split", hazards = "ax-al-return", recs = "grove-walk",
       note = "reads the shop data file (file position effects)")]
pub fn find_section(file: GPtr<u8>, name: GPtr<u8>) -> u32 {
    let mut depth: i32 = 0;
    let mut line = load_line(file);
    while !line.is_null() {
        let c = line.read();
        if c != b'#' && c != 0 {
            let r = strncmp(line, g::<u8>(STR_SECTION), 7);
            if r == 0 {
                depth += 1;
                strtok(line, g::<u8>(STR_SECTION_DELIMS));
                let tok = strtok(GPtr::NULL, g::<u8>(STR_SECTION_DELIMS));
                if depth == 1 && stricmp(name, tok) == 0 {
                    return 1;
                }
            } else if is_end(line) {
                depth -= 1;
                if depth < 0 {
                    return r as u32 & 0xFFFF_FF00;
                }
            }
        }
        line = load_line(file);
    }
    0
}

/// `CShopping::GetNextSection(FILESTREAM file)` (stub 0x49AF10 → 0x156B920, cdecl). Reads lines,
/// skipping empty ones and `#` comments, until one starting with `"section"` (strncmp 7): returns its
/// second token (`strtok(line, D)`, `strtok(NULL, " \t")`); a line starting with "end" or the end of
/// the file → NULL. D is pushed by a no-CD VM call (0xCB73E0, descriptor 0x1560034, not evaluated); it
/// is taken to be `" \t"` (0x85A510), the delimiter of the second call and of `FindSection`.
#[hook(addr = 0x49af10, conv = cdecl, name = "CShopping::GetNextSection", expect = "E9 0B 0A 0D 01",
       unverified = "mvpj-world-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "the first strtok delimiter comes from the no-CD VM 0xCB73E0 (assumed 0x85A510)")]
pub fn get_next_section(file: GPtr<u8>) -> GPtr<u8> {
    let mut line = load_line(file);
    while !line.is_null() {
        let c = line.read();
        if c != b'#' && c != 0 {
            if strncmp(line, g::<u8>(STR_SECTION), 7) == 0 {
                strtok(line, g::<u8>(STR_SECTION_DELIMS));
                return strtok(GPtr::NULL, g::<u8>(STR_SECTION_DELIMS));
            }
            if is_end(line) {
                break;
            }
        }
        line = load_line(file);
    }
    GPtr::NULL
}

/// `CShopping::LoadStats()` (0x49B6A0, cdecl). Opens `data\shopping.dat` ("r"), `FindSection(f,
/// "prices")` (result ignored), `ms_numBuyableItems` = 0, then for every `GetNextSection` name: the
/// section index = the first of the 11 price-section names equal (`_stricmp`), else -1. Each line of
/// the section (empty and `#` skipped, "end" ends it): first token (`strtok`, `" \t,"`) → key:
/// sections 4, 5, 6 `GetUppercaseKey`, 9 `CWeaponInfo::FindWeaponType`, 2 the previous line's key
/// unchanged (the stack slot is not written for that section: a first line there reads an
/// uninitialised local; zero here), else the model index of `GetModelInfo` (preset -1).
/// `ms_keys[n]` = key, `ms_bHasBought[n]` = 0; then one token skipped (two more for 4..6, one more
/// for 9); tokens a, b, c, d: `ms_statModifiers[n]` = { `GetChangingStatIndex(a)`, `atol(b)`,
/// `GetChangingStatIndex(c)`, `atol(d)` } as bytes; n += 1. Closes the file.
#[hook(addr = 0x49b6a0, conv = cdecl, name = "CShopping::LoadStats", expect = "83 EC 10 53 57",
       unverified = "mvpj-world-1", route = "split", hazards = "uninit-locals", recs = "grove-walk",
       note = "file I/O; one-time loader")]
pub fn load_stats() {
    let f = open_file(g::<u8>(STR_SHOPPING_DAT), g::<u8>(STR_MODE_R));
    find_section(f, g::<u8>(STR_PRICES));
    NUM_BUYABLE_ITEMS.set(0);
    let d = || g::<u8>(STR_STATS_DELIMS);
    let mut key: u32 = 0;
    let mut name = get_next_section(f);
    while !name.is_null() {
        let mut sec: i32 = -1;
        for i in 0..11 {
            if stricmp(name, g::<GPtr<u8>>(SECTION_NAMES).wrapping_index(i).read()) == 0 {
                sec = i;
                break;
            }
        }
        let mut line = load_line(f);
        while !line.is_null() {
            let c = line.read();
            if c != b'#' && c != 0 {
                if is_end(line) {
                    break;
                }
                let tok = strtok(line, d());
                match sec {
                    4..=6 => key = get_uppercase_key(tok),
                    9 => key = find_weapon_type(tok) as u32,
                    2 => {}
                    _ => {
                        key = u32::MAX;
                        get_model_info(tok, GPtr::<u32>::cast::<i32>(GPtr::from_raw(&raw mut key)));
                    }
                }
                let n = NUM_BUYABLE_ITEMS.get();
                g::<u32>(KEYS).wrapping_index(n).write(key);
                g::<u8>(HAS_BOUGHT).byte_add(n as isize).write(0);
                strtok(GPtr::NULL, d());
                if (4..=6).contains(&sec) {
                    strtok(GPtr::NULL, d());
                    strtok(GPtr::NULL, d());
                } else if sec == 9 {
                    strtok(GPtr::NULL, d());
                }
                let a = strtok(GPtr::NULL, d());
                let b = strtok(GPtr::NULL, d());
                let cc = strtok(GPtr::NULL, d());
                let dd = strtok(GPtr::NULL, d());
                let m = g::<u8>(STAT_MODIFIERS).byte_add(n as isize * 4);
                m.write(get_changing_stat_index(a) as u8);
                m.byte_add(2).write(get_changing_stat_index(cc) as u8);
                let bv = atol(b);
                let dv = atol(dd);
                m.byte_add(1).write(bv as u8);
                m.byte_add(3).write(dv as u8);
                NUM_BUYABLE_ITEMS.set(n + 1);
            }
            line = load_line(f);
        }
        name = get_next_section(f);
    }
    close_file(f);
}

/// `ms_priceSectionLoaded` (0xA9A7C8), `ms_numPriceModifiers` (0xA9A7D0), `ms_shopLoaded` (0xA9A7D8, a string),
/// `ms_numItemsInShop` (0xA9A7F0), `ms_shopContents` (0xA9A318), `ms_priceModifiers` (0xA98650, key / price).
const SECTION_LOADED: usize = 0xA9_A7C8;
const NUM_PRICE_MODIFIERS: usize = 0xA9_A7D0;
const SHOP_LOADED_NAME: usize = 0xA9_A7D8;
const NUM_ITEMS_IN_SHOP: usize = 0xA9_A7F0;
const SHOP_CONTENTS: usize = 0xA9_A318;
const PRICE_MODIFIER_TABLE: usize = 0xA9_8650;
/// `"shops"` (0x85A550), `"bought"` (0x85A558), `"type"` (0x85A548), `"item"` (0x85A540).
const STR_SHOPS: usize = 0x85_A550;
const STR_BOUGHT: usize = 0x85_A558;
const STR_TYPE: usize = 0x85_A548;
const STR_ITEM: usize = 0x85_A540;
/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8).
const MODEL_INFOS: usize = 0xA9_B0C8;

orig! {
    /// `CVehicleModelInfo::GetNumWheelUpgrades(int32 set)` (0x4C8740)
    fn get_num_wheel_upgrades(set: i32) -> i32 = cdecl 0x4C8740 => sa_stream::modelinfo::vehicle::get_num_wheel_upgrades;
    /// `CVehicleModelInfo::GetWheelUpgrade(int32 set, int32 i)` (0x4C8750)
    fn get_wheel_upgrade(set: i32, i: i32) -> i32 = cdecl 0x4C8750 => sa_stream::modelinfo::vehicle::get_wheel_upgrade;
    /// `CShopping::GetKey(const char*, int32 section)` (0x49AB30)
    fn get_key_call(name: GPtr<u8>, section: i32) -> u32 = cdecl 0x49AB30 => get_key;
    /// `IsValidModForVehicle(uint32 key, CVehicle*)` (0x49B010)
    fn is_valid_mod_for_vehicle_call(key: u32, veh: GPtr<u8>) -> u8 = cdecl 0x49B010 => is_valid_mod_for_vehicle;
}
use crate::bindings::find_player_vehicle;
use sa_stream::bindings::{get_animation_block_index, load_all_requested_models};
use crate::bindings::load_prices as load_prices_call;
use sa_core::bindings::{crt_strncpy, streaming_request_model, timer_resume, timer_suspend};

/// `CShopping::GetKey(const char* name, int32 section)` (stub 0x49AB30 -> 0x1561A10, cdecl): sections 4, 5, 6
/// `CKeyGen::GetUppercaseKey(name)`; 9 `CWeaponInfo::FindWeaponType(name)` (called through the protection's computed
/// jump, `[0x1560480] - [0x4D9E12] - 1` = 0x743D10); 2 the section itself; else the model index of
/// `CModelInfo::GetModelInfo(name, &slot)` with the section slot preset to -1.
#[hook(addr = 0x49AB30, conv = cdecl, name = "CShopping::GetKey", expect = "E9 DB 6E 0C 01",
       unverified = "frontier2-a27", route = "L2", hazards = "none")]
pub fn get_key(name: GPtr<u8>, section: i32) -> u32 {
    match section {
        4..=6 => get_uppercase_key(name),
        9 => find_weapon_type(name) as u32,
        2 => 2,
        _ => {
            let mut idx: i32 = -1;
            get_model_info(name, GPtr::from_raw(&raw mut idx));
            idx as u32
        }
    }
}

/// `IsValidModForVehicle(uint32 key, CVehicle* veh)` (stub 0x49B010 -> 0x15691D0, cdecl, AL). With the upgrade
/// model's flag 0x100 (+0x12): a wheel (`flags & 0x7C00 == 0x800`) is valid when it is one of the vehicle model's
/// wheel set (+0x4F) upgrades (`GetNumWheelUpgrades` through the protection's computed jump, `~[0x156053C] ^
/// [0x665DA4]` = 0x4C8740, re-read per step); another upgrade when it is among the model's 18 upgrades (+0x2D6, i16).
/// Without the flag a 0x4400 kind is valid when +0x1D3 is clear and (+0x1BE != 1 or +0x42E bit 0x10); any other kind
/// by the upgrade list.
#[hook(addr = 0x49B010, conv = cdecl, name = "IsValidModForVehicle", expect = "E9 BB E1 0C 01",
       unverified = "frontier2-a27", route = "L2", hazards = "none")]
pub fn is_valid_mod_for_vehicle(key: u32, veh: GPtr<u8>) -> u8 {
    let mi = |i: i32| g::<GPtr<u8>>(MODEL_INFOS).wrapping_index(i).read();
    let vmi = mi(i32::from(rd::<i16>(veh, 0x22)));
    let flags = u32::from(rd::<u16>(mi(key as i32), 0x12));
    let in_list = || (0..0x12isize).any(|i| i32::from(rd::<i16>(vmi, 0x2D6 + 2 * i)) == key as i32);
    if flags & 0x100 != 0 {
        if flags & 0x7C00 != 0x800 {
            return u8::from(in_list());
        }
        let set = i32::from(rd::<i8>(vmi, 0x4F));
        let mut i = 0;
        if get_num_wheel_upgrades(set) > 0 {
            loop {
                if get_wheel_upgrade(set, i) == key as i32 {
                    return 1;
                }
                i += 1;
                if i >= get_num_wheel_upgrades(set) {
                    break;
                }
            }
        }
        return 0;
    }
    if flags & 0x7C00 != 0x4400 {
        return u8::from(in_list());
    }
    if rd::<u8>(veh, 0x1D3) != 0 {
        return 0;
    }
    if rd::<u8>(veh, 0x1BE) != 1 || rd::<u8>(veh, 0x42E) & 0x10 != 0 {
        return 1;
    }
    0
}

/// `CShopping::LoadPrices(const char* section)` (0x49B8D0, cdecl). Nothing when the section
/// (`GetPriceSectionFromName`) is already loaded; a loaded one first gets its animation block deletable (the inlined
/// RemoveLoadedPrices). Then the section index, `ms_numPrices` and `ms_numItemsInShop` = 0, `CTimer::Suspend`,
/// `data\shopping.dat` read from `section prices` / `section <name>`: every line not empty, `#` or "end" (which
/// stops) adds a price: key (`GetKey(first token)`), name tag (`strncpy` 8), sections 4 / 5 the clothes model key
/// (`GetUppercaseKey`) and type (`atol`), 6 the tattoo type (-1 for `-`) and texture key, 9 the ammo; four tokens
/// skipped, the price (`atol`), replaced by every matching price modifier (the last one wins; gta-reversed stops
/// at the first); comment lines do not count (gta-reversed counts them). Then the section's animation block
/// requested (flags 0x1A) and loaded, and `CTimer::Resume` (tail call).
#[hook(addr = 0x49B8D0, conv = cdecl, name = "CShopping::LoadPrices", expect = "56 57 8B 7C 24 0C 57",
       unverified = "frontier2-a27", route = "L2", hazards = "heap-writes",
       note = "file I/O (shopping.dat), streams the section's animation block")]
pub fn load_prices(name: GPtr<u8>) {
    let sec = get_price_section_from_name(name);
    let loaded = g::<i32>(SECTION_LOADED).read();
    if sec == loaded {
        return;
    }
    if loaded != 0 {
        super::misc2::remove_loaded_prices();
    }
    g::<i32>(SECTION_LOADED).write(sec);
    g::<i32>(NUM_PRICES).write(0);
    g::<i32>(NUM_ITEMS_IN_SHOP).write(0);
    timer_suspend();
    let f = open_file(g::<u8>(STR_SHOPPING_DAT), g::<u8>(STR_MODE_R));
    if find_section(f, g::<u8>(STR_PRICES)) & 0xFF != 0 {
        find_section(f, name);
    }
    let d = || g::<u8>(STR_STATS_DELIMS);
    let mut line = load_line(f);
    while !line.is_null() {
        let c = line.read();
        if c != b'#' && c != 0 {
            if is_end(line) {
                break;
            }
            let tok = strtok(line, d());
            let key = get_key_call(tok, g::<i32>(SECTION_LOADED).read());
            let it = || item(g::<i32>(NUM_PRICES).read());
            wr::<u32>(it(), 0, key);
            let tag = strtok(GPtr::NULL, d());
            crt_strncpy(it().byte_add(0x10), tag, 8);
            match g::<i32>(SECTION_LOADED).read() {
                4 | 5 => {
                    let a = strtok(GPtr::NULL, d());
                    let b = strtok(GPtr::NULL, d());
                    wr::<u32>(it(), 8, get_uppercase_key(a));
                    let v = atol(b);
                    wr::<i32>(it(), 0xC, v);
                }
                6 => {
                    let a = strtok(GPtr::NULL, d());
                    let b = strtok(GPtr::NULL, d());
                    let v = if a.read() == b'-' { -1 } else { atol(a) };
                    wr::<i32>(it(), 8, v);
                    wr::<u32>(it(), 0xC, get_uppercase_key(b));
                }
                9 => {
                    let v = atol(strtok(GPtr::NULL, d()));
                    wr::<i32>(it(), 8, v);
                }
                _ => {}
            }
            for _ in 0..4 {
                strtok(GPtr::NULL, d());
            }
            let p = it();
            wr::<i32>(p, 4, atol(strtok(GPtr::NULL, d())));
            let n = g::<i32>(NUM_PRICE_MODIFIERS).read();
            for k in 0..n.max(0) as isize {
                let m = g::<u8>(PRICE_MODIFIER_TABLE).byte_add(8 * k);
                if rd::<u32>(p, 0) == rd::<u32>(m, 0) {
                    wr::<u32>(p, 4, rd::<u32>(m, 4));
                }
            }
            g::<i32>(NUM_PRICES).write(g::<i32>(NUM_PRICES).read().wrapping_add(1));
        }
        line = load_line(f);
    }
    close_file(f);
    let sname = g::<GPtr<u8>>(SECTION_NAMES).wrapping_index(g::<i32>(SECTION_LOADED).read()).read();
    let b = get_animation_block_index(sname);
    if b != -1 {
        streaming_request_model(b.wrapping_add(0x63E7), 0x1A);
        load_all_requested_models(1);
    }
    timer_resume();
}

/// `repe cmpsb` over 5 bytes (a 4-letter keyword with its terminator).
fn bytes_eq5(a: GPtr<u8>, b: GPtr<u8>) -> bool {
    (0..5isize).all(|i| a.byte_add(i).read() == b.byte_add(i).read())
}

/// `CShopping::LoadShop(const char* name)` (0x49BBE0, cdecl). Nothing when `_stricmp(name, ms_shopLoaded)` is 0;
/// else the name copied into `ms_shopLoaded`, `CTimer::Suspend`, `ms_numItemsInShop` = 0. "bought": the shop holds
/// the keys of the loaded prices the player has bought (the CTimer stays suspended: no Resume on this path, as the
/// original). Else `data\shopping.dat` from `section shops` / `section <name>`: a `type` line loads its prices
/// (`LoadPrices` of the token copied to a local), an `item` line adds `GetKey(token, section)` unless the section is
/// car mods (1) and `IsValidModForVehicle(key, FindPlayerVehicle(-1, false))` fails; then `CTimer::Resume`.
#[hook(addr = 0x49BBE0, conv = cdecl, name = "CShopping::LoadShop", expect = "83 EC 20 56 8B 74 24 28",
       unverified = "frontier2-a27", route = "L2", hazards = "heap-writes",
       note = "file I/O (shopping.dat); the bought path leaves CTimer suspended (original)")]
pub fn load_shop(name: GPtr<u8>) {
    let shop = g::<u8>(SHOP_LOADED_NAME);
    if stricmp(name, shop) == 0 {
        return;
    }
    let mut i = 0isize;
    loop {
        let c = name.byte_add(i).read();
        shop.byte_add(i).write(c);
        if c == 0 {
            break;
        }
        i += 1;
    }
    timer_suspend();
    g::<i32>(NUM_ITEMS_IN_SHOP).write(0);
    if stricmp(name, g::<u8>(STR_BOUGHT)) == 0 {
        let n = g::<i32>(NUM_PRICES).read();
        if n > 0 {
            let mut count = g::<i32>(NUM_ITEMS_IN_SHOP).read();
            for k in 0..n {
                let key = rd::<u32>(item(k), 0);
                if g::<u8>(HAS_BOUGHT).byte_add(key_index(key) as isize).read() != 0 {
                    g::<u32>(SHOP_CONTENTS).wrapping_index(count).write(key);
                    count += 1;
                }
            }
            g::<i32>(NUM_ITEMS_IN_SHOP).write(count);
        }
        return;
    }
    let f = open_file(g::<u8>(STR_SHOPPING_DAT), g::<u8>(STR_MODE_R));
    if find_section(f, g::<u8>(STR_SHOPS)) & 0xFF != 0 {
        find_section(f, name);
    }
    let d = || g::<u8>(STR_SECTION_DELIMS);
    let mut line = load_line(f);
    while !line.is_null() {
        let c = line.read();
        if c != b'#' && c != 0 {
            if is_end(line) {
                break;
            }
            let tok = strtok(line, d());
            if bytes_eq5(g::<u8>(STR_TYPE), tok) {
                let t = strtok(GPtr::NULL, d());
                let mut buf = [0u8; 0x100];
                let mut k = 0usize;
                loop {
                    let ch = t.byte_add(k as isize).read();
                    buf[k] = ch;
                    if ch == 0 {
                        break;
                    }
                    k += 1;
                }
                load_prices_call(GPtr::from_raw(buf.as_mut_ptr()));
            } else if bytes_eq5(g::<u8>(STR_ITEM), tok) {
                let t = strtok(GPtr::NULL, d());
                let key = get_key_call(t, g::<i32>(SECTION_LOADED).read());
                if g::<i32>(SECTION_LOADED).read() != 1
                    || is_valid_mod_for_vehicle_call(key, find_player_vehicle(-1, 0)) != 0
                {
                    let n = g::<i32>(NUM_ITEMS_IN_SHOP).read();
                    g::<u32>(SHOP_CONTENTS).wrapping_index(n).write(key);
                    g::<i32>(NUM_ITEMS_IN_SHOP).write(n.wrapping_add(1));
                }
            }
        }
        line = load_line(f);
    }
    close_file(f);
    timer_resume();
}


/// Byte `k` of buyable item `index`'s stat modifiers (stat, change, stat, change), sign-extended.
fn stat_modifier(index: i32, k: isize) -> i32 {
    i32::from(g::<i8>(STAT_MODIFIERS).byte_add(index as isize * 4 + k).read())
}

/// `CShopping::UpdateStats(int32 index, bool increment)` (stub 0x49BEF0 -> 0x156B6F0, cdecl): both stat modifiers of
/// the item through `IncrementStat`, their changes negated when `increment` is false.
#[hook(addr = 0x49BEF0, conv = cdecl, name = "CShopping::UpdateStats", expect = "E9 FB F7 0C 01",
       unverified = "frontier2-a28", route = "L2", hazards = "none")]
pub fn update_stats(index: i32, increment: u8) {
    let change = |k| if increment != 0 { stat_modifier(index, k) } else { stat_modifier(index, k).wrapping_neg() };
    super::misc2::shopping_increment_stat(stat_modifier(index, 0), change(1));
    super::misc2::shopping_increment_stat(stat_modifier(index, 2), change(3));
}

/// The car-mod part of `CShopping::Buy` (0x49B0E0, EDI = the upgrade's model index): `AddVehicleUpgrade` on the
/// player's vehicle; for an automobile (+0x590 == 0) the part the upgrade replaces is repaired. The part is the
/// upgrade model's slot (`(flags +0x12 >> 10) & 0x1F`) when it uses a vehicle dummy (flag 0x100), else the parent
/// component of that slot in the vehicle model's upgrade table (+0x5C, 0x20 bytes per slot, +0xD0): 2 (the wheels:
/// tyres 0, 2, 1, 3 in that order; gta-reversed fixes 0, 1, 2, 3), 12 / 13 the front / rear bumper panel, 16 / 17
/// the bonnet / boot door. Both model infos are read before the upgrade is added.
#[hook(addr = 0x49B0E0, conv = reg("args = edi"), name = "FUN_0049b0e0", expect = "53 55 56 6A 00 6A FF",
       unverified = "frontier2-a28", route = "L2", hazards = "heap-writes",
       note = "adds the upgrade's RW clump to the vehicle")]
pub fn buy_car_mod(key: u32) {
    let veh = find_player_vehicle(-1, 0);
    let upgrade = g::<GPtr<u8>>(MODEL_INFOS).wrapping_index(key as i32).read();
    let vmi = g::<GPtr<u8>>(MODEL_INFOS).wrapping_index(i32::from(rd::<i16>(veh, 0x22))).read();
    add_vehicle_upgrade(veh, key as i32);
    if rd::<u32>(veh, 0x590) != 0 {
        return;
    }
    let flags = u32::from(rd::<u16>(upgrade, 0x12));
    let slot = (flags >> 10) & 0x1F;
    let part = if flags & 0x100 != 0 {
        slot
    } else {
        rd::<u32>(rd::<GPtr<u8>>(vmi, 0x5C), (slot * 0x20 + 0xD0) as isize)
    };
    match part {
        2 => {
            for w in [0, 2, 1, 3] {
                fix_tyre(veh, w);
            }
        }
        12 => fix_panel(veh, 0xC, 5),
        13 => fix_panel(veh, 0xD, 6),
        16 => fix_door(veh, 0x10, 0),
        17 => fix_door(veh, 0x11, 1),
        _ => {}
    }
}

/// `CShopping::Buy(uint32 key, int32 extraInfo)` (0x49BF70, cdecl). The price item of the key (the inlined FindItem;
/// -1 when absent, which reads the slot before the table) and the buyable index of that item's key (the inlined
/// GetItemIndex, -1 when absent); the clothes description is the player's or, while stored, `gStoredClothesState`.
/// The focus player pays `GetPrice(key)`, both stat modifiers are applied, `STAT_TOTAL_SHOPPING_BUDGET` (0x3E) grows
/// by the price and the item is marked bought. Then by the loaded price section: car mods the car modification budget
/// (0x37) and then the upgrade (0x49B0E0; gta-reversed adds the upgrade first); paint jobs that budget; clothes the
/// fashion budget (0xE), the stats of the texture worn in the item's part taken off, the player's own description set
/// to the key / model, and parole for a part whose texture depends on model 0, 1, 3, 7 or 8; haircuts the same with
/// 0x1F and parole always; tattoos 0x1E and the texture set with model 0; gifts the flowers (weapon 0xE, no ammo);
/// food the meals eaten (0xC8, +1.0) before the food budget (0x14); weapons 0xD, then body armour (0x30) sets the
/// ped's armour to the player's maximum, any other weapon is given with the item's ammo and made current through its
/// slot. `extraInfo` is not read.
#[hook(addr = 0x49BF70, conv = cdecl, name = "CShopping::Buy", expect = "8B 15 CC A7 A9 00 83 EC 08",
       unverified = "frontier2-a28", route = "L2", hazards = "heap-writes",
       note = "clothes / weapons / vehicle upgrades of the player")]
pub fn buy(key: u32, _extra_info: i32) {
    let it = find_item_index(key);
    let price_item = item(it);
    let index = key_index(rd::<u32>(price_item, 0));
    let player = PLAYERS + usize::from(PLAYER_IN_FOCUS.get()) * PLAYER_INFO_SIZE;
    let ped = g::<GPtr<u8>>(player).read();
    let worn = if CLOTHES_STORED.get() != 0 {
        STORED_CLOTHES.ptr().cast::<u8>()
    } else {
        rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x480), 4)
    };
    let price = get_price(key);
    let money = g::<i32>(player + 0xB8);
    money.write(money.read().wrapping_sub(price));
    super::misc2::shopping_increment_stat(stat_modifier(index, 0), stat_modifier(index, 1));
    super::misc2::shopping_increment_stat(stat_modifier(index, 2), stat_modifier(index, 3));
    let budget = xi(price).to_f32().to_bits();
    let modify = |stat: u32, v: u32| super::stats::modify_stat(stat, v);
    modify(0x3E, budget);
    g::<u8>(HAS_BOUGHT).byte_add(index as isize).write(1);
    let own_clothes = || rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x480), 4);
    let ty = || rd::<i32>(price_item, 0xC);
    let take_off_worn = || {
        let worn_key = rd::<u32>(worn, 0x28 + ty() as isize * 4);
        if worn_key != 0 {
            update_stats_call(get_item_index(worn_key), 0);
        }
    };
    match g::<i32>(SECTION_LOADED).read() {
        1 => {
            modify(0x37, budget);
            buy_car_mod_call(key);
        }
        2 => modify(0x37, budget),
        4 => {
            modify(0xE, budget);
            take_off_worn();
            clothes_desc_set_texture_and_model(own_clothes(), key, rd::<u32>(price_item, 8), ty());
            if matches!(get_texture_dependency(ty()), 0 | 1 | 3 | 7 | 8) {
                clear_wanted_level_and_go_on_parole(find_player_wanted(-1));
            }
        }
        5 => {
            modify(0x1F, budget);
            take_off_worn();
            clothes_desc_set_texture_and_model(own_clothes(), key, rd::<u32>(price_item, 8), ty());
            clear_wanted_level_and_go_on_parole(find_player_wanted(-1));
        }
        6 => {
            modify(0x1E, budget);
            clothes_desc_set_texture_and_model(own_clothes(), key, 0, rd::<i32>(price_item, 8));
        }
        7 => {
            ped_give_weapon(ped, 0xE, 0, 0);
        }
        8 => {
            modify(0xC8, 1.0f32.to_bits());
            modify(0x14, budget);
        }
        9 => {
            modify(0xD, budget);
            if key == 0x30 {
                let max_armour = g::<u8>(PLAYERS + usize::from(PLAYER_IN_FOCUS.get()) * PLAYER_INFO_SIZE + 0x150).read();
                wr::<f32>(ped, 0x548, xi(i32::from(max_armour)).to_f32());
            } else {
                ped_give_weapon(ped, key, rd::<u32>(price_item, 8), 0);
                let slot = ped_get_weapon_slot(ped, key);
                ped_set_current_weapon_slot(ped, slot);
            }
        }
        _ => {}
    }
}

orig! {
    /// `CShopping::UpdateStats(int32, bool)` (0x49BEF0)
    fn update_stats_call(index: i32, increment: u8) = cdecl 0x49BEF0 => update_stats;
    /// The car-mod part of `CShopping::Buy` (0x49B0E0, EDI = the upgrade's model index)
    fn buy_car_mod_call(key: u32) = reg("args = edi") 0x49B0E0 => buy_car_mod;
}
use crate::bindings::{
    add_vehicle_upgrade, clear_wanted_level_and_go_on_parole, clothes_desc_set_texture_and_model, find_player_wanted,
    fix_door, fix_panel, fix_tyre, get_texture_dependency, ped_get_weapon_slot, ped_set_current_weapon_slot,
};
use sa_core::bindings::ped_give_weapon;


pub const HOOKS: &[HookReg] = &[
    HOOK_FIND_SECTION,
    HOOK_GET_NEXT_SECTION,
    HOOK_LOAD_STATS,
    HOOK_GET_PRICE_SECTION_FROM_NAME,
    HOOK_GET_ITEM_INDEX,
    HOOK_GET_CHANGING_STAT_INDEX,
    HOOK_FIND_ITEM,
    HOOK_GET_PRICE,
    HOOK_GET_NAME_TAG,
    HOOK_GET_EXTRA_INFO,
    HOOK_SET_CURRENT_PROPERTY,
    HOOK_STORE_CLOTHES_STATE,
    HOOK_RESTORE_CLOTHES_STATE,
    HOOK_HAS_PLAYER_BOUGHT,
    HOOK_SET_PLAYER_HAS_BOUGHT,
    HOOK_GET_KEY,
    HOOK_IS_VALID_MOD_FOR_VEHICLE,
    HOOK_LOAD_PRICES,
    HOOK_LOAD_SHOP,
    HOOK_UPDATE_STATS,
    HOOK_BUY_CAR_MOD,
    HOOK_BUY,
];
