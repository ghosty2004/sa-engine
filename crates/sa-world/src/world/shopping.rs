//! `CShopping` leaves (PORT-WORLD). Ported from the 1.0 US bytes (gta-reversed `Shopping.cpp`,
//! `docs/re/specs/WORLD/CShopping*.md` as reading aids).
//!
//! Price items (`ms_prices`, 0xA986F0, 0x18 each, `ms_numPrices` at 0xA9A7CC): +0 key, +4 price, +8/+0xC
//! extra info, +0x10 name tag. Bought flags `ms_bHasBought` (0xA972A0, one byte per key of
//! `ms_keys`, 0xA97D90, 560 keys).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{ftol32, g, rd, x, xi};
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

orig! {
    /// CRT `_stricmp` (0x8229B6)
    pub fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// CRT `strncmp` (0x8214D0)
    fn strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
    /// CRT `strtok` (0x82244B)
    fn strtok(s: GPtr<u8>, delim: GPtr<u8>) -> GPtr<u8> = cdecl 0x82244B => unported;
    /// CRT `atol` (0x82258E)
    fn atol(s: GPtr<u8>) -> i32 = cdecl 0x82258E => unported;
    /// `CFileLoader::LoadLine(FILESTREAM)` (0x536F80)
    fn load_line(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x536F80 => sa_stream::fileloader::load_line_file;
    /// `CFileMgr::OpenFile(const char*, const char*)` (0x538900)
    fn open_file(name: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => sa_core::filemgr::open_file;
    /// `CFileMgr::CloseFile(FILESTREAM)` (0x5389D0)
    fn close_file(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => sa_core::filemgr::close_file;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    fn get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => sa_core::keygen::get_uppercase_key;
    /// `CWeaponInfo::FindWeaponType(const char*)` (0x743D10)
    fn find_weapon_type(name: GPtr<u8>) -> u32 = cdecl 0x743D10 => unported;
    /// `CModelInfo::GetModelInfo(const char*, int* index)` (0x4C5940)
    fn get_model_info(name: GPtr<u8>, idx: GPtr<u32>) -> GPtr<u8> = cdecl 0x4C5940 => unported;
}

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
                    9 => key = find_weapon_type(tok),
                    2 => {}
                    _ => {
                        key = u32::MAX;
                        get_model_info(tok, GPtr::from_raw(&raw mut key));
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
];
