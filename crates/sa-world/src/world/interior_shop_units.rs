//! PORT-VEH-3 (batch `world3-2`, SA:MP-executed rows): shop furnishing (`Interior_c::FurnishShop`,
//! `Shop_PlaceFixedUnits`) and `FurnitureSubGroup_c::GetFurniture`. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{ftol32, rd, wr, xi};

orig! {
    /// `Interior_c::SetTilesStatus(int32, int32, int32, int32, int32, bool)` (0x591700)
    fn interior_set_tiles_status(i: GPtr<u8>, tx: i32, ty: i32, w: i32, h: i32, status: i32, force: u32) = thiscall 0x591700 => unported;
    /// `Interior_c::Shop_PlaceCounter(bool)` (0x599EF0)
    fn shop_place_counter(i: GPtr<u8>, left: u32) -> i32 = thiscall 0x599EF0 => unported;
    /// `Interior_c::Shop_FurnishEdges()` (0x59A1B0)
    fn shop_furnish_edges(i: GPtr<u8>) = thiscall 0x59A1B0 => unported;
    /// `Interior_c::Shop_FurnishAisles()` (0x59A590)
    fn shop_furnish_aisles(i: GPtr<u8>) = thiscall 0x59A590 => unported;
    /// `Interior_c::AddInteriorInfo(int32, float, float, int32, CEntity*)` (0x591E40)
    fn interior_add_interior_info(i: GPtr<u8>, kind: u32, x: u32, y: u32, dir: u32, ent: u32) -> u8 = thiscall 0x591E40 => unported;
    /// `Interior_c::Shop_PlaceFixedUnits()` (0x59A030)
    fn call_shop_place_fixed_units(i: GPtr<u8>) = thiscall 0x59A030 => interior_shop_place_fixed_units;
}

/// `rand()` as called from `site`.
fn rand_at(site: u32) -> i32 {
    sa_core::orig::at_site(site, sa_core::crt::rand)
}

/// `Interior_c::Shop_PlaceFixedUnits()` (0x59A030): unless the info's door column (+5) is -1, the
/// door tiles are reserved (`SetTilesStatus(door - 1, 0, 2, 1, 7, 0)`), a counter is placed when
/// there is room on either side of the door (6 tiles: random side when both, else the free one) and
/// infos 9 (at (counter, 2)) and 10 (at (counter, 0), direction 2) are added. Without room the
/// counter position is the uninitialised local the original reads: the `push ecx` slot, i.e. the
/// `this` pointer as an int.
#[hook(addr = 0x59A030, conv = thiscall, name = "Interior_c::Shop_PlaceFixedUnits", this = "Interior_c",
       expect = "51 56 8B F1 8B 46 14",
       unverified = "world3-2", route = "L2", hazards = "heap-writes",
       note = "rand site 0x59A07A; the bool passed to Shop_PlaceCounter keeps the this pointer's upper bytes; the counter places objects")]
pub fn interior_shop_place_fixed_units(this: GPtr<u8>) {
    let info = rd::<GPtr<u8>>(this, 0x14);
    let door = rd::<i8>(info, 5);
    if door == -1 {
        return;
    }
    interior_set_tiles_status(this, i32::from(door) - 1, 0, 2, 1, 7, 0);
    let info = rd::<GPtr<u8>>(this, 0x14);
    let d = i32::from(rd::<i8>(info, 5));
    let right = i32::from(rd::<u8>(info, 2)) - d - 2;
    let left = d - 2;
    // the `push ecx` slot: this pointer, low byte replaced by the random side
    let mut slot = this.addr() as u32;
    let arg = if right >= 6 {
        Some(if left >= 6 {
            let side = u32::from(rand_at(0x59_A07A) < 0x3FFF);
            slot = (slot & 0xFFFF_FF00) | side;
            slot
        } else {
            0
        })
    } else if left >= 6 {
        Some(1)
    } else {
        None
    };
    if let Some(a) = arg {
        slot = shop_place_counter(this, a) as u32;
    }
    let f = xi(slot as i32).to_f32().to_bits();
    interior_add_interior_info(this, 9, f, 0x4000_0000, 0, 0);
    interior_add_interior_info(this, 10, f, 0, 2, 0);
}

/// `Interior_c::FurnishShop(int32 kind)` (0x59A790): stores the kind byte (+0x790); unless both sides
/// of the door (+5) are narrower than 6 tiles, the four corner tiles are reserved (status 2) and the
/// fixed units, edges and aisles are placed.
#[hook(addr = 0x59A790, conv = thiscall, name = "Interior_c::FurnishShop", this = "Interior_c",
       expect = "8A 44 24 04 56",
       unverified = "world3-2", route = "L2", hazards = "heap-writes",
       note = "places furniture objects (pools, world)")]
pub fn interior_furnish_shop(this: GPtr<u8>, kind: u8) {
    wr::<u8>(this, 0x790, kind);
    let info = rd::<GPtr<u8>>(this, 0x14);
    let d = i32::from(rd::<i8>(info, 5));
    if d - 1 < 6 && i32::from(rd::<u8>(info, 2)) - d < 6 {
        return;
    }
    interior_set_tiles_status(this, 0, 0, 1, 1, 2, 0);
    let h = i32::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 3)) - 1;
    interior_set_tiles_status(this, 0, h, 1, 1, 2, 0);
    let w = i32::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 2)) - 1;
    interior_set_tiles_status(this, w, 0, 1, 1, 2, 0);
    let info = rd::<GPtr<u8>>(this, 0x14);
    interior_set_tiles_status(this, i32::from(rd::<u8>(info, 2)) - 1, i32::from(rd::<u8>(info, 3)) - 1, 1, 1, 2, 0);
    call_shop_place_fixed_units(this);
    shop_furnish_edges(this);
    shop_furnish_aisles(this);
}

/// `FurnitureSubGroup_c::GetFurniture(int16 id, uint8 rating)` (0x590EE0): with an id >= 0 the item
/// (list +0xC, next +4) with that id (+0xA), else null; otherwise a random item among those whose
/// rating range (+0xE..+0xF) contains `rating` (`ftol(rand frac * count)`), null when none. (The
/// original's `rating == -1` branch compares the zero-extended byte with -1 and is dead.)
#[hook(addr = 0x590EE0, conv = thiscall, name = "FurnitureSubGroup_c::GetFurniture", this = "FurnitureSubGroup_c",
       expect = "53 56 8B F1 66 8B 4C 24 0C",
       unverified = "world3-2", route = "L2", hazards = "none",
       note = "rand site 0x590F8C; the count is kept in the argument slot")]
pub fn furniture_sub_group_get_furniture(this: GPtr<u8>, id: i16, rating: u8) -> GPtr<u8> {
    if id > -1 {
        let mut it = rd::<GPtr<u8>>(this, 0xC);
        while !it.is_null() {
            if rd::<i16>(it, 0xA) == id {
                return it;
            }
            it = rd::<GPtr<u8>>(it, 4);
        }
        return it;
    }
    let fits = |it: GPtr<u8>| rating >= rd::<u8>(it, 0xE) && rating <= rd::<u8>(it, 0xF);
    let mut n = 0i32;
    let mut it = rd::<GPtr<u8>>(this, 0xC);
    while !it.is_null() {
        if fits(it) {
            n += 1;
        }
        it = rd::<GPtr<u8>>(it, 4);
    }
    let r = rand_at(0x59_0F8C) & 0xFFFF;
    let pick = ftol32(xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul(xi(n)));
    let mut k = 0i32;
    let mut it = rd::<GPtr<u8>>(this, 0xC);
    while !it.is_null() {
        if fits(it) {
            if k == pick {
                return it;
            }
            k += 1;
        }
        it = rd::<GPtr<u8>>(it, 4);
    }
    GPtr::from_addr(0)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_INTERIOR_SHOP_PLACE_FIXED_UNITS,
    HOOK_INTERIOR_FURNISH_SHOP,
    HOOK_FURNITURE_SUB_GROUP_GET_FURNITURE,
];
