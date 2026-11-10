//! `CTheZones`, `CZoneDef` (PORT-WORLD). Ported from the 1.0 US bytes; gta-reversed `TheZones.cpp` and
//! `docs/re/specs/WORLD/CTheZones*.md`, `CZoneDef.md` as reading aids. Where gta-reversed differs (index
//! by `/600` instead of `* 0.0016666667` with a u8 truncation, `SetZoneRadarColours` writing alpha,
//! `FindZoneByLabel` searching every type, `Init` through `CreateZone`) the bytes win.
//!
//! `CZone` (0x20): +0 label A[8] (gr `m_InfoLabel`), +8 label B[8] (gr `m_TextLabel`), +0x10 X1, +0x12 Y1,
//! +0x14 Z1, +0x16 X2, +0x18 Y2, +0x1A Z2 (int16), +0x1C zone info index (u16), +0x1E type, +0x1F level.
//! `CZoneInfo` (0x11): +0..+9 gang strengths, +0xA dealer strength, +0xB..+0xE colour RGBA, +0xF flags
//! (bits 0..4 pop type, 5..6 radar mode, 7 no cops), +0x10 pop races.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{find_player_coors, ftol32, load_data_from_work_buffer, rd, save_data_to_work_buffer, wr, x, xi};

pub const ZONE_SIZE: usize = 0x20;
pub const ZONE_INFO_SIZE: usize = 0x11;
pub const NUM_NAV_ZONES: usize = 380;
pub const NUM_MAP_ZONES: usize = 39;
pub const NUM_ZONE_INFOS: usize = 380;

const _: () = assert!(ZONE_SIZE == sa_core::layouts::size::CZone);
const _: () = assert!(ZONE_INFO_SIZE == sa_core::layouts::size::CZoneInfo);

sa_core::globals! {
    list GLOBALS;
    /// `CTheZones::ZonesRevealed`
    pub static ZONES_REVEALED @ 0xBA372C: i32 = 0;
    /// `CTheZones::ZonesVisited` (`bool[10][10]`)
    pub static ZONES_VISITED @ 0xBA3730: [u8; 100] = [0; 100];
    /// `CTheZones::TotalNumberOfNavigationZones`
    pub static TOTAL_NAV_ZONES @ 0xBA3794: u16 = 0;
    /// `CTheZones::NavigationZoneArray` (`CZone[380]`)
    pub static NAV_ZONES @ 0xBA3798: [u8; NUM_NAV_ZONES * ZONE_SIZE] = [0; NUM_NAV_ZONES * ZONE_SIZE];
    /// `CTheZones::TotalNumberOfMapZones`
    pub static TOTAL_MAP_ZONES @ 0xBA1900: u16 = 0;
    /// `CTheZones::MapZoneArray` (`CZone[39]`)
    pub static MAP_ZONES @ 0xBA1908: [u8; NUM_MAP_ZONES * ZONE_SIZE] = [0; NUM_MAP_ZONES * ZONE_SIZE];
    /// `CTheZones::TotalNumberOfZoneInfos`
    pub static TOTAL_ZONE_INFOS @ 0xBA1DE8: u16 = 0;
    /// `CTheZones::ZoneInfoArray` (`CZoneInfo[380]`)
    pub static ZONE_INFOS @ 0xBA1DF0: [u8; NUM_ZONE_INFOS * ZONE_INFO_SIZE] = [0; NUM_ZONE_INFOS * ZONE_INFO_SIZE];
    /// `CTheZones::m_CurrLevel`
    pub static CURR_LEVEL @ 0xBA6718: i32 = 0;
    /// `CTheZones::Update`'s static `CountSeconds`
    pub static UPDATE_COUNT_SECONDS @ 0xBA6720: u32 = 0;
    /// guard bits of `CTheZones::Update`'s function-local statics (bit 0 = `CountSeconds`)
    pub static UPDATE_STATIC_GUARD @ 0xBA6724: u32 = 0;
    /// gang colour component tables read by `FillZonesWithGangColours` (gr `gaGangColoursR`)
    pub static GANG_COLOURS_A @ 0x8D1344: [u8; 12] = [200, 70, 255, 0, 255, 200, 240, 0, 255, 255, 0, 0];
    /// second table (gr `gaGangColoursG`)
    pub static GANG_COLOURS_B @ 0x8D1350: [u8; 12] = [0, 200, 200, 0, 220, 200, 140, 200, 255, 255, 0, 0];
    /// third table (gr `gaGangColoursB`)
    pub static GANG_COLOURS_C @ 0x8D135C: [u8; 12] = [200, 0, 0, 200, 190, 200, 240, 255, 255, 255, 0, 0];
}

/// `CTheScripts::bPlayerIsOffTheMap`
const PLAYER_IS_OFF_THE_MAP: usize = 0xA444A4;
/// 0x859A94: 3000.0
const F3000: f32 = 3000.0;
/// 0x865060: 0.0016666667 (bits 0x3ADA740E)
fn f_inv600() -> f32 {
    f32::from_bits(0x3ADA_740E)
}

orig! {
    /// `CTheZones::PointLiesWithinZone(const CVector*, CZone*)` (0x572270)
    pub fn point_lies_within_zone_call(p: GPtr<f32>, z: GPtr<u8>) -> u8 = cdecl 0x572270 => point_lies_within_zone;
    /// `CTheZones::GetLevelFromPosition(const CVector&)` (0x572300)
    pub fn get_level_from_position_call(p: GPtr<f32>) -> i32 = cdecl 0x572300 => get_level_from_position;
    /// `CTheZones::GetCurrentZoneLockedOrUnlocked(CVector2D)` (0x572130)
    pub fn get_current_zone_locked_or_unlocked_call(x: f32, y: f32) -> u8 = cdecl 0x572130 => get_current_zone_locked_or_unlocked;
    /// `CTheZones::FindSmallestZoneForPosition(const CVector&, bool)` (0x572360)
    pub fn find_smallest_zone_for_position_call(p: GPtr<f32>, navi: u8) -> GPtr<u8> = cdecl 0x572360 => find_smallest_zone_for_position;
    /// `CTheZones::InitZonesPopulationSettings` (0x5720D0)
    pub fn init_zones_population_settings_call() = cdecl 0x5720D0 => init_zones_population_settings;
    /// `CTheZones::SetCurrentZoneAsUnlocked` (0x572800)
    pub fn set_current_zone_as_unlocked_call() = cdecl 0x572800 => set_current_zone_as_unlocked;
    /// `CTheZones::Init` (0x572670)
    pub fn init_call() = cdecl 0x572670 => init;
    /// `CGangWars::CanPlayerStartAGangWarHere(CZoneInfo*)` (0x443F80)
    pub fn can_player_start_a_gang_war_here(info: GPtr<u8>) -> u8 = cdecl 0x443F80 => unported;
}

fn nav_zone(i: u32) -> GPtr<u8> {
    NAV_ZONES.ptr().cast::<u8>().byte_add((i as usize * ZONE_SIZE) as isize)
}

fn map_zone(i: u32) -> GPtr<u8> {
    MAP_ZONES.ptr().cast::<u8>().byte_add((i as usize * ZONE_SIZE) as isize)
}

fn zone_info(i: u32) -> GPtr<u8> {
    ZONE_INFOS.ptr().cast::<u8>().byte_add((i as usize * ZONE_INFO_SIZE) as isize)
}

/// `ZonesVisited + 9 + idx` (0xBA3739 + idx; idx may leave the array, as in the original).
fn zones_visited_byte(idx: i32) -> GPtr<u8> {
    ZONES_VISITED.ptr().cast::<u8>().byte_add(9 + idx as isize)
}

/// The visited-zone index: `(u8)_ftol((x + 3000) * 0.0016666667) * 10 - (u8)_ftol((y + 3000) * ...)`.
fn visited_index(px: f32, py: f32) -> i32 {
    let xi8 = ftol32(x(px).fadd_m(F3000).fmul_m(f_inv600())) as u8;
    let yi8 = ftol32(x(py).fadd_m(F3000).fmul_m(f_inv600())) as u8;
    i32::from(xi8) * 10 - i32::from(yi8)
}

/// `CTheZones::InitZonesPopulationSettings` (0x5720D0): for every one of the 380 zone infos (fixed
/// count): +0xF = (+0xF & 0xE5) | 5, +0x10 |= 0xF, +0xA = 0, dwords +0/+4 and word +8 = 0, then +0xF &= 0x1F.
#[hook(addr = 0x5720D0, conv = cdecl, name = "CTheZones::InitZonesPopulationSettings", expect = "53 56 B8 FF 1D BA 00")]
pub fn init_zones_population_settings() {
    for i in 0..NUM_ZONE_INFOS as u32 {
        let z = zone_info(i);
        let f = (rd::<u8>(z, 0xF) & 0xE5) | 5;
        wr::<u8>(z, 0xF, f);
        wr::<u8>(z, 0x10, rd::<u8>(z, 0x10) | 0xF);
        wr::<u8>(z, 0xA, 0);
        wr::<u32>(z, 0, 0);
        wr::<u32>(z, 4, 0);
        wr::<u16>(z, 8, 0);
        wr::<u8>(z, 0xF, rd::<u8>(z, 0xF) & 0x1F);
    }
}

/// `CTheZones::ResetZonesRevealed` (0x572110): `ZonesVisited` = 0 (100 bytes), `ZonesRevealed` = 0.
#[hook(addr = 0x572110, conv = cdecl, name = "CTheZones::ResetZonesRevealed", expect = "57 B9 19 00 00 00")]
pub fn reset_zones_revealed() {
    ZONES_VISITED.set([0; 100]);
    ZONES_REVEALED.set(0);
}

/// `CTheZones::GetCurrentZoneLockedOrUnlocked(CVector2D)` (0x572130): `ZonesVisited[9 + idx] != 0`
/// (out-of-map positions index outside the array, original bug).
#[hook(addr = 0x572130, conv = cdecl, name = "CTheZones::GetCurrentZoneLockedOrUnlocked", expect = "D9 44 24 04 56")]
pub fn get_current_zone_locked_or_unlocked(px: f32, py: f32) -> u8 {
    (zones_visited_byte(visited_index(px, py)).read() != 0) as u8
}

/// `CTheZones::AssignZoneInfoForThisZone(int16)` (0x572180): the first other navigation zone (u16 index
/// below the count) whose label A (8 bytes, two dword compares) equals this one's gives its info index;
/// else the index is `TotalNumberOfZoneInfos++`.
#[hook(addr = 0x572180, conv = cdecl, name = "CTheZones::AssignZoneInfoForThisZone", expect = "53 55 56 66 8B 74 24 10")]
pub fn assign_zone_info_for_this_zone(index: u16) {
    let count = TOTAL_NAV_ZONES.get();
    let me = nav_zone(u32::from(index));
    let mut i: u16 = 0;
    while i < count {
        if i == index {
            // 0x5721A7 `je 0x5721D6` skips `inc edx`: with index < count the original spins forever
            // (the game only calls it with index == count, before the count is incremented).
            loop {
                core::hint::spin_loop();
            }
        }
        let z = nav_zone(u32::from(i));
        if rd::<u32>(z, 0) == rd::<u32>(me, 0) && rd::<u32>(z, 4) == rd::<u32>(me, 4) {
            wr::<u16>(me, 0x1C, rd::<u16>(z, 0x1C));
            return;
        }
        i = i.wrapping_add(1);
    }
    let n = TOTAL_ZONE_INFOS.get();
    wr::<u16>(me, 0x1C, n);
    TOTAL_ZONE_INFOS.set(n.wrapping_add(1));
}

/// `CTheZones::ZoneIsEntirelyContainedWithinOtherZone(CZone*, CZone*)` (0x572220): signed int16
/// compares a.X1 >= b.X1, a.X2 <= b.X2, Y, Z.
#[hook(addr = 0x572220, conv = cdecl, name = "CTheZones::ZoneIsEntirelyContainedWithinOtherZone", expect = "8B 44 24 04 66 8B 50 10")]
pub fn zone_is_entirely_contained_within_other_zone(a: GPtr<u8>, b: GPtr<u8>) -> u8 {
    let r = |o: isize| (rd::<i16>(a, o), rd::<i16>(b, o));
    let (a1, b1) = r(0x10);
    let (a2, b2) = r(0x16);
    let (a3, b3) = r(0x12);
    let (a4, b4) = r(0x18);
    let (a5, b5) = r(0x14);
    let (a6, b6) = r(0x1A);
    (a1 >= b1 && a2 <= b2 && a3 >= b3 && a4 <= b4 && a5 >= b5 && a6 <= b6) as u8
}

/// `CTheZones::PointLiesWithinZone(const CVector*, CZone*)` (0x572270): `(float)X1 <= p.x <= (float)X2`
/// etc. (`fild` + `fcomp`, false on NaN).
#[hook(addr = 0x572270, conv = cdecl, name = "CTheZones::PointLiesWithinZone", expect = "8B 4C 24 08 0F BF 41 10")]
pub fn point_lies_within_zone(p: GPtr<f32>, z: GPtr<u8>) -> u8 {
    let pf = p.cast::<u8>();
    for (lo, hi, c) in [(0x10, 0x16, 0), (0x12, 0x18, 4), (0x14, 0x1A, 8)] {
        let v = x(rd::<f32>(pf, c));
        // fild lo; fcomp v; test ah,0x41; jp fail -> pass on Less/Equal
        if xi(i32::from(rd::<i16>(z, lo))).fcom(v).test_parity(0x41) {
            return 0;
        }
        // fild hi; fcomp v; test ah,1; jne fail -> pass on Greater/Equal
        if xi(i32::from(rd::<i16>(z, hi))).fcom(v).test_any(0x01) {
            return 0;
        }
    }
    1
}

/// `CTheZones::GetLevelFromPosition(const CVector&)` (0x572300): the level byte (+0x1F) of the first map
/// zone 1.. (count read once, u16) containing the point (`PointLiesWithinZone`), else map zone 0's.
#[hook(addr = 0x572300, conv = cdecl, name = "CTheZones::GetLevelFromPosition", expect = "53 55 56 57 66 8B 3D 00 19 BA 00")]
pub fn get_level_from_position(p: GPtr<f32>) -> i32 {
    let count = TOTAL_MAP_ZONES.get();
    let mut i: u16 = 1;
    while i < count {
        let z = map_zone(u32::from(i));
        if point_lies_within_zone_call(p, z) != 0 {
            return i32::from(rd::<u8>(z, 0x1F));
        }
        i += 1;
    }
    i32::from(rd::<u8>(map_zone(0), 0x1F))
}

/// The int16 size `X2 - X1 + Y2 - Y1` (int32) of a zone.
fn zone_size(z: GPtr<u8>) -> i32 {
    i32::from(rd::<i16>(z, 0x16)) - i32::from(rd::<i16>(z, 0x10)) - i32::from(rd::<i16>(z, 0x12))
        + i32::from(rd::<i16>(z, 0x18))
}

/// `CTheZones::FindSmallestZoneForPosition(const CVector&, bool navOnly)` (0x572360): starts with
/// navigation zone 0; for zones 1..count (count read once), skipping type != 0 when `navOnly`, takes a
/// zone containing the point (`PointLiesWithinZone`) whose size is below the best one's in an
/// **unsigned** compare (`jae`, original).
// Not hooked: the protection code at 0x406A9E reads the entry bytes 0x572363.. as data
// (`xor ecx, [0x572363]`). Ported, unhookable.
pub fn find_smallest_zone_for_position(p: GPtr<f32>, nav_only: u8) -> GPtr<u8> {
    let mut best = nav_zone(0);
    let mut best_size = zone_size(best);
    let count = TOTAL_NAV_ZONES.get();
    if count > 1 {
        for i in 1..u32::from(count) {
            let z = nav_zone(i);
            if nav_only != 0 && rd::<u8>(z, 0x1E) != 0 {
                continue;
            }
            if point_lies_within_zone_call(p, z) == 0 {
                continue;
            }
            let s = zone_size(z);
            if (s as u32) < (best_size as u32) {
                best = z;
                best_size = s;
            }
        }
    }
    best
}

/// `CTheZones::GetZoneInfo(const CVector&, CZone**)` (0x572400).
#[hook(addr = 0x572400, conv = cdecl, name = "CTheZones::GetZoneInfo-", expect = "8B 44 24 04 6A 00")]
pub fn get_zone_info(p: GPtr<f32>, out_zone: GPtr<GPtr<u8>>) -> GPtr<u8> {
    let z = find_smallest_zone_for_position_call(p, 0);
    if !z.is_null() {
        if !out_zone.is_null() {
            out_zone.write(z);
        }
        return zone_info(u32::from(rd::<u16>(z, 0x1C)));
    }
    if !out_zone.is_null() {
        out_zone.write(nav_zone(0));
    }
    zone_info(0)
}

/// `CTheZones::FillZonesWithGangColours(bool disableRadarGangColors)` (0x572440). Component k of the
/// colour is `sum_g table_k[g] * strength[g]` over gangs 0..2 (tables at 0x8D1344/0x8D1350/0x8D135C,
/// indexed by gang), divided (signed) by `max(sum, 1)`; alpha `min(sum*3, 120)` raised to 55 when sum != 0.
#[hook(addr = 0x572440, conv = cdecl, name = "CTheZones::FillZonesWithGangColours", expect = "83 EC 08 66 83 3D E8 1D BA 00 00")]
pub fn fill_zones_with_gang_colours(disable: u8) {
    if TOTAL_ZONE_INFOS.get() == 0 {
        return;
    }
    let (ta, tb, tc) = (GANG_COLOURS_A.get(), GANG_COLOURS_B.get(), GANG_COLOURS_C.get());
    let mut i: i32 = 0;
    loop {
        let z = zone_info(i as u32);
        let s = [rd::<u8>(z, 0) as i32, rd::<u8>(z, 1) as i32, rd::<u8>(z, 2) as i32];
        let comp = |t: &[u8; 12]| (0..3).fold(0i32, |a, k| a.wrapping_add((t[k] as i32).wrapping_mul(s[k])));
        let (ca, cb, cc) = (comp(&ta), comp(&tb), comp(&tc));
        let sum = s[0] + s[1] + s[2];
        let on = sum != 0 && disable == 0 && can_player_start_a_gang_war_here(z) != 0;
        let f = rd::<u8>(z, 0xF);
        wr::<u8>(z, 0xF, if on { (f & 0xBF) | 0x20 } else { f & 0x9F });
        let mut a = sum * 3;
        if a > 0x78 {
            a = 0x78;
        }
        wr::<u8>(z, 0xE, a as u8);
        if sum != 0 {
            let al = a as u8;
            wr::<u8>(z, 0xE, if al > 0x37 { al } else { 0x37 });
        }
        let d = if sum > 1 { sum } else { 1 };
        wr::<u8>(z, 0xB, (ca / d) as u8);
        wr::<u8>(z, 0xC, (cb / d) as u8);
        wr::<u8>(z, 0xD, (cc / d) as u8);
        i += 1;
        if i >= i32::from(TOTAL_ZONE_INFOS.get()) {
            break;
        }
    }
}

/// `CTheZones::GetNavigationZone(uint16)` (0x572590).
#[hook(addr = 0x572590, conv = cdecl, name = "CTheZones::GetNavigationZone", expect = "0F B7 44 24 04")]
pub fn get_navigation_zone(index: u16) -> GPtr<u8> {
    nav_zone(u32::from(index))
}

/// `CTheZones::GetMapZone(uint16)` (0x5725A0).
#[hook(addr = 0x5725A0, conv = cdecl, name = "CTheZones::GetMapZone", expect = "0F B7 44 24 04")]
pub fn get_map_zone(index: u16) -> GPtr<u8> {
    map_zone(u32::from(index))
}

/// `CTheZones::Calc2DDistanceBetween2Zones(CZone*, CZone*)` (0x5725B0): int gaps per axis (0 when the
/// ranges overlap), `sqrt(dy*dy + dx*dx)` on the x87 (dx stored as f32), result in ST0.
#[hook(addr = 0x5725B0, conv = cdecl, name = "CTheZones::Calc2DDistanceBetween2Zones", expect = "51 8B 44 24 08")]
pub fn calc_2d_distance_between_2_zones(a: GPtr<u8>, b: GPtr<u8>) -> X87 {
    let w = |p: GPtr<u8>, o: isize| i32::from(rd::<i16>(p, o));
    let mut dx = 0.0f32;
    if w(a, 0x10) > w(b, 0x16) {
        dx = xi(w(a, 0x10) - w(b, 0x16)).to_f32();
    } else if w(a, 0x16) < w(b, 0x10) {
        dx = xi(w(b, 0x10) - w(a, 0x16)).to_f32();
    }
    let dy = if w(a, 0x12) > w(b, 0x18) {
        xi(w(a, 0x12) - w(b, 0x18))
    } else if w(a, 0x18) < w(b, 0x12) {
        xi(w(b, 0x12) - w(a, 0x18))
    } else {
        X87::zero()
    };
    dy.fmul(dy).fadd(x(dx).fmul_m(dx)).fsqrt()
}

/// `CTheZones::Init` (0x572670): clears every navigation zone (bytes 0..0x1E; the level byte +0x1F is
/// kept) and every map zone (type +0x1E = 3), `InitZonesPopulationSettings`, clears `ZonesVisited`, then
/// writes zone 0 of both arrays inline ("SAN_AND"/"THEMAP" in both labels, -3000..3000 x/y,
/// -2000..2000 z, level 0), counts 1/1/1, `ZonesRevealed` = 0, `m_CurrLevel` = 0. No `CreateZone` call.
#[hook(addr = 0x572670, conv = cdecl, name = "CTheZones::Init", expect = "53 55 56 57 B8 A0 37 BA 00")]
pub fn init() {
    let clear = |z: GPtr<u8>, ty: u8| {
        wr::<u32>(z, 0, 0);
        wr::<u32>(z, 4, 0);
        wr::<u32>(z, 8, 0);
        wr::<u32>(z, 0xC, 0);
        for o in [0x10, 0x12, 0x14, 0x16, 0x18, 0x1A] {
            wr::<u16>(z, o, 0);
        }
        wr::<u8>(z, 0x1E, ty);
        wr::<u16>(z, 0x1C, 0);
    };
    for i in 0..NUM_NAV_ZONES as u32 {
        clear(nav_zone(i), 0);
    }
    init_zones_population_settings_call();
    ZONES_VISITED.set([0; 100]);
    // 0x86506C / 0x865070: "SAN_AND\0"
    let (s0, s1) = (u32::from_le_bytes(*b"SAN_"), u32::from_le_bytes(*b"AND\0"));
    let z = nav_zone(0);
    wr::<u32>(z, 0, s0);
    wr::<u32>(z, 4, s1);
    wr::<u32>(z, 8, s0);
    wr::<u32>(z, 0xC, s1);
    TOTAL_NAV_ZONES.set(1);
    TOTAL_ZONE_INFOS.set(1);
    ZONES_REVEALED.set(0);
    let corners = |z: GPtr<u8>| {
        wr::<i16>(z, 0x10, -3000);
        wr::<i16>(z, 0x12, -3000);
        wr::<i16>(z, 0x14, -2000);
        wr::<i16>(z, 0x16, 3000);
        wr::<i16>(z, 0x18, 3000);
        wr::<i16>(z, 0x1A, 2000);
    };
    corners(z);
    wr::<u8>(z, 0x1F, 0);
    wr::<u8>(z, 0x1E, 0);
    CURR_LEVEL.set(0);
    for i in 0..NUM_MAP_ZONES as u32 {
        clear(map_zone(i), 3);
    }
    // 0x865064 "THEM", 0x865068 "AP", 0x86506A 0
    let m = map_zone(0);
    TOTAL_MAP_ZONES.set(1);
    corners(m);
    wr::<u8>(m, 0x1F, 0);
    wr::<u32>(m, 0, u32::from_le_bytes(*b"THEM"));
    wr::<u16>(m, 4, u16::from_le_bytes(*b"AP"));
    wr::<u8>(m, 6, 0);
    wr::<u32>(m, 8, u32::from_le_bytes(*b"THEM"));
    wr::<u16>(m, 0xC, u16::from_le_bytes(*b"AP"));
    wr::<u8>(m, 0xE, 0);
}

/// `CTheZones::SetCurrentZoneAsUnlocked` (0x572800), see the spec: level from the player position,
/// then (on the map, area 0, zone not yet visited) `ZonesRevealed++` and the visited byte = 1.
#[hook(addr = 0x572800, conv = cdecl, name = "CTheZones::SetCurrentZoneAsUnlocked", expect = "83 EC 0C 8D 44 24 00")]
pub fn set_current_zone_as_unlocked() {
    let mut pos = [0f32; 3];
    let pp = GPtr::from_raw(pos.as_mut_ptr());
    find_player_coors(pp, -1);
    CURR_LEVEL.set(get_level_from_position_call(pp));
    if super::common::g::<u8>(PLAYER_IS_OFF_THE_MAP).read() != 0 {
        return;
    }
    if sa_core::game::CURR_AREA.get() != 0 {
        return;
    }
    let (px, py) = (pp.read(), pp.wrapping_index(1).read());
    if get_current_zone_locked_or_unlocked_call(px, py) != 0 {
        return;
    }
    let idx = visited_index(px, py);
    let n = ZONES_REVEALED.get().wrapping_add(1);
    zones_visited_byte(idx).write(1);
    ZONES_REVEALED.set(n);
}

/// `CTheZones::FindZone(CVector*, uint64 label, eZoneType)` (0x572B80): type 0 compares label B (+8),
/// type 2 label A (+0) (8 bytes, two dwords); any other type: false. True when a matching navigation
/// zone (count re-read each step for type 2... read once per loop, u16) contains the point.
#[hook(addr = 0x572B80, conv = cdecl, name = "CTheZones::FindZone-", expect = "8B 44 24 10 83 E8 00")]
pub fn find_zone(p: GPtr<f32>, lo: u32, hi: u32, ty: i32) -> u8 {
    let off = match ty {
        0 => 8,
        2 => 0,
        _ => return 0,
    };
    let count = TOTAL_NAV_ZONES.get();
    let mut i: u16 = 0;
    while i < count {
        let z = nav_zone(u32::from(i));
        if rd::<u32>(z, off) == lo && rd::<u32>(z, off + 4) == hi && point_lies_within_zone_call(p, z) != 0 {
            return 1;
        }
        i += 1;
    }
    0
}

/// `CTheZones::PostZoneCreation` (0x572B70): a lone `ret` (not hookable).
pub fn post_zone_creation() {}

/// `CTheZones::FindZoneByLabel(const char*, eZoneType)` (0x572C40): copies the name (unbounded, into a
/// zeroed 8-byte buffer: longer names overflow the original's stack, here they are cut at 8); type != 2
/// gives -1; else the first navigation zone whose label A equals the 8 bytes.
#[hook(addr = 0x572C40, conv = cdecl, name = "CTheZones::FindZoneByLabel", expect = "83 EC 08 8B 44 24 0C")]
pub fn find_zone_by_label(name: GPtr<u8>, ty: i32) -> i16 {
    let mut buf = [0u8; 8];
    let mut k = 0usize;
    loop {
        let c = name.byte_add(k as isize).read();
        if k < 8 {
            buf[k] = c;
        }
        if c == 0 {
            break;
        }
        k += 1;
    }
    if ty != 2 {
        return -1;
    }
    let (a, b) = (
        u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
        u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
    );
    let count = TOTAL_NAV_ZONES.get();
    let mut i: u16 = 0;
    while i < count {
        let z = nav_zone(u32::from(i));
        if rd::<u32>(z, 0) == a && rd::<u32>(z, 4) == b {
            return i as i16;
        }
        i += 1;
    }
    -1
}

/// `CTheZones::SetZoneRadarColours(int16, char radarMode, uint8 r, uint8 g, uint8 b)` (0x572CC0): radar
/// mode bits 5..6 of +0xF from `mode << 5`, colour bytes +0xB..+0xD. Alpha is not written (gr writes 255).
#[hook(addr = 0x572CC0, conv = cdecl, name = "CTheZones::SetZoneRadarColours", expect = "0F B7 44 24 04")]
pub fn set_zone_radar_colours(index: u16, mode: u8, r: u8, gr: u8, b: u8) {
    let z = zone_info(u32::from(rd::<u16>(nav_zone(u32::from(index)), 0x1C)));
    let f = rd::<u8>(z, 0xF);
    wr::<u8>(z, 0xF, f ^ (((mode << 5) ^ f) & 0x60));
    wr::<u8>(z, 0xB, r);
    wr::<u8>(z, 0xC, gr);
    wr::<u8>(z, 0xD, b);
}

/// `CTheZones::Update` (0x572D10): the function-local `CountSeconds` starts at `time - 5000` (guard bit
/// 0 of 0xBA6724); outside area 0 it follows the time; else after more than 5000 ms (unsigned)
/// `SetCurrentZoneAsUnlocked` and `CountSeconds` = the time re-read.
#[hook(addr = 0x572D10, conv = cdecl, name = "CTheZones::Update", expect = "A0 24 67 BA 00")]
pub fn update() {
    let t = sa_core::timer::TIME_MS.get();
    if UPDATE_STATIC_GUARD.get() & 1 == 0 {
        UPDATE_STATIC_GUARD.set(UPDATE_STATIC_GUARD.get() | 1);
        UPDATE_COUNT_SECONDS.set(t.wrapping_sub(5000));
    }
    if sa_core::game::CURR_AREA.get() != 0 {
        UPDATE_COUNT_SECONDS.set(t);
        return;
    }
    if t.wrapping_sub(UPDATE_COUNT_SECONDS.get()) <= 5000 {
        return;
    }
    set_current_zone_as_unlocked_call();
    UPDATE_COUNT_SECONDS.set(sa_core::timer::TIME_MS.get());
}

/// `CTheZones::Save` (0x5D2E60): level (4), the three counts (2 each), the zones (0x20), infos (0x11),
/// map zones, `ZonesVisited` (100), `ZonesRevealed` (4); counts re-read in each loop test. Returns true.
#[hook(addr = 0x5D2E60, conv = cdecl, name = "CTheZones::Save", expect = "56 6A 04 68 18 67 BA 00")]
pub fn save() -> u8 {
    save_data_to_work_buffer(CURR_LEVEL.ptr().cast(), 4);
    save_data_to_work_buffer(TOTAL_NAV_ZONES.ptr().cast(), 2);
    save_data_to_work_buffer(TOTAL_ZONE_INFOS.ptr().cast(), 2);
    save_data_to_work_buffer(TOTAL_MAP_ZONES.ptr().cast(), 2);
    let mut i: u16 = 0;
    while i < TOTAL_NAV_ZONES.get() {
        save_data_to_work_buffer(nav_zone(u32::from(i)), 0x20);
        i += 1;
    }
    i = 0;
    while i < TOTAL_ZONE_INFOS.get() {
        save_data_to_work_buffer(zone_info(u32::from(i)), 0x11);
        i += 1;
    }
    i = 0;
    while i < TOTAL_MAP_ZONES.get() {
        save_data_to_work_buffer(map_zone(u32::from(i)), 0x20);
        i += 1;
    }
    save_data_to_work_buffer(ZONES_VISITED.ptr().cast(), 100);
    save_data_to_work_buffer(ZONES_REVEALED.ptr().cast(), 4);
    1
}

/// `CTheZones::Load` (0x5D2F40): `Init`, then the same layout as `Save` read back. Returns true.
#[hook(addr = 0x5D2F40, conv = cdecl, name = "CTheZones::Load", expect = "56 E8 2A F7 F9 FF")]
pub fn load() -> u8 {
    init_call();
    load_data_from_work_buffer(CURR_LEVEL.ptr().cast(), 4);
    load_data_from_work_buffer(TOTAL_NAV_ZONES.ptr().cast(), 2);
    load_data_from_work_buffer(TOTAL_ZONE_INFOS.ptr().cast(), 2);
    load_data_from_work_buffer(TOTAL_MAP_ZONES.ptr().cast(), 2);
    let mut i: u16 = 0;
    while i < TOTAL_NAV_ZONES.get() {
        load_data_from_work_buffer(nav_zone(u32::from(i)), 0x20);
        i += 1;
    }
    i = 0;
    while i < TOTAL_ZONE_INFOS.get() {
        load_data_from_work_buffer(zone_info(u32::from(i)), 0x11);
        i += 1;
    }
    i = 0;
    while i < TOTAL_MAP_ZONES.get() {
        load_data_from_work_buffer(map_zone(u32::from(i)), 0x20);
        i += 1;
    }
    load_data_from_work_buffer(ZONES_VISITED.ptr().cast(), 100);
    load_data_from_work_buffer(ZONES_REVEALED.ptr().cast(), 4);
    1
}

/// `CZoneDef::IsPointWithin(CVector)` (0x72D850, thiscall, vector by value, `ret 0xC`): see
/// `docs/re/specs/WORLD/CZoneDef.md`.
#[hook(addr = 0x72D850, conv = thiscall, name = "CZoneDef::IsPointWithin", expect = "83 EC 0C 0F BF 41 0C", this = "CZoneDef")]
pub fn zone_def_is_point_within(this: GPtr<u8>, px: f32, py: f32, pz: f32) -> u8 {
    let w = |o: isize| i32::from(rd::<i16>(this, o));
    // minZ < z strictly (test ah,5; jp)
    if xi(w(0xC)).fcom(x(pz)).test_parity(0x05) {
        return 0;
    }
    // maxZ > z strictly (test ah,0x41; jne)
    if xi(w(0xE)).fcom(x(pz)).test_any(0x41) {
        return 0;
    }
    let dx = x(px).fsub(xi(w(0))).to_f32();
    let dy = x(py).fsub(xi(w(2))).to_f32();
    for (vx, vy) in [(w(4), w(6)), (w(8), w(0xA))] {
        let t = xi(vx).fmul_m(dx).fadd(xi(vy).fmul_m(dy));
        if t.fcom(X87::zero()).test_any(0x01) {
            return 0;
        }
        let s = vx.wrapping_mul(vx).wrapping_add(vy.wrapping_mul(vy));
        if t.fcom(xi(s)).test_parity(0x41) {
            return 0;
        }
    }
    1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_ZONES_POPULATION_SETTINGS,
    HOOK_RESET_ZONES_REVEALED,
    HOOK_GET_CURRENT_ZONE_LOCKED_OR_UNLOCKED,
    HOOK_ASSIGN_ZONE_INFO_FOR_THIS_ZONE,
    HOOK_ZONE_IS_ENTIRELY_CONTAINED_WITHIN_OTHER_ZONE,
    HOOK_POINT_LIES_WITHIN_ZONE,
    HOOK_GET_LEVEL_FROM_POSITION,
    HOOK_GET_ZONE_INFO,
    HOOK_FILL_ZONES_WITH_GANG_COLOURS,
    HOOK_GET_NAVIGATION_ZONE,
    HOOK_GET_MAP_ZONE,
    HOOK_CALC_2D_DISTANCE_BETWEEN_2_ZONES,
    HOOK_INIT,
    HOOK_SET_CURRENT_ZONE_AS_UNLOCKED,
    HOOK_FIND_ZONE,
    HOOK_FIND_ZONE_BY_LABEL,
    HOOK_SET_ZONE_RADAR_COLOURS,
    HOOK_UPDATE,
    HOOK_SAVE,
    HOOK_LOAD,
    HOOK_ZONE_DEF_IS_POINT_WITHIN,
];
