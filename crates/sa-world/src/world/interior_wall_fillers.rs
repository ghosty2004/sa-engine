//! WRITER-WORLD-2 (batch `world6-1`): interior functions whose bodies continue in a no-CD protection
//! chunk of `.text` 0x401000.., decoded by hand from the exe image:
//!
//! - `Interior_c::PlaceFurnitureOnWall`: chunk 0x4015B9 is `mov ecx, 0xBAB380; jmp 0x593146` (the
//!   furniture manager `this` of the `GetFurniture` call, moved out of the body);
//! - `Interior_c::Office_PlaceEdgeFillers`: chunk 0x405BEF is `and eax, 0xFFFF; jmp 0x599223` (the
//!   mask of the first `rand()`);
//! - `InteriorManager_c::AreAnimsLoaded`: chunk 0x4045B5 pushes `[0x15607B8] + [0x5CAFC6] + CF(0)` =
//!   0x1D183357 + 0xE36E39E9 = 0x866D40 ("int_house"), then jumps back to 0x5980FD;
//! - `InteriorGroup_c::FindInteriorInfo`: the entry is `nop; jmp 0x4075A5`, whose chunk does `sub esp,
//!   0x208` and returns into `[0x1560874] + [0x4556C6]` = 0x5E7F85 + 0xFFFAC9F1 = 0x594976 (the body
//!   right after the entry). The protection reads the entry's rel32 as a key (`sbb edx, [0x594972]` at
//!   0x401249): sa-hook's keycells `READERS` keep it reading the original dword.
//!
//! The decoded constants are those of the exe image (the key dwords at 0x15607B8, 0x1560874 and the
//! code dwords 0x5CAFC6, 0x4556C6 are not written by the game; 0x5CAFC6 and 0x4556C6 are not hooked
//! entries). Ported from the bytes; gta-reversed has none of them reversed.

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::mem::GPtr;
use sa_core::orig::at_site;
use sa_core::hook;

use super::common::{ftol32, g, rd, xi};

use crate::bindings::add_interior_info;
use crate::bindings::get_furniture;
use crate::bindings::get_num_empty_tiles;
use crate::bindings::place_furniture;
use sa_core::bindings::general_random_int_in_range as random_in_range;
use sa_stream::bindings::get_animation_block_index;

/// `g_furnitureMan` (0xBAB380).
const FURNITURE_MAN: usize = 0xBA_B380;
/// `1 / 32768` (0x858B14).
use super::interior_shop_furnishing::INV_32768;
/// `CAnimManager::ms_aAnimBlocks[i].bLoaded` (0xB5D4B0 + i * 0x20).
const ANIM_BLOCK_LOADED: usize = 0xB5_D4B0;
/// The "office edge filler placed" latch (0xBB3DC8).
const OFFICE_LATCH: usize = 0xBB_3DC8;

use sa_core::field::ptr_of as p;

use super::interior_shop_furnishing::rand_scaled;

/// `ftol((rand() & 0xFFFF) * (1/32768) * n)` with an integer `n` (`fimul m32`).
fn rand_times(site: u32, n: i32) -> i32 {
    let r = at_site(site, rand) & 0xFFFF;
    ftol32(xi(r).fmul_m(INV_32768).fmul(xi(n)))
}

/// `Interior_c::PlaceFurnitureOnWall(int32 group, int32 subGroup, int32 id, float height, int32 a5,
/// int32 wall, int32 offset, int32 depth, int32* outWall, int32* outOffset, int32* outX, int32* outY,
/// int32* outA, int32* outB)` (0x593120): fetches the furniture (`GetFurniture(group, subGroup, id,
/// interior info +0x1F wealth)`; none: 0) and tries `PlaceFurniture` up to 100 times (once when both
/// `wall` and `offset` are given, i.e. not -1). A -1 `wall` is drawn each try from `rand() * 4 / 32768`,
/// a -1 `offset` from `rand() * (span - furniture size) / 32768` (span: the interior's width +0x14->+2
/// for walls 0 / 2, its depth +3 for walls 1 / 3; size: furniture +0xC). The tile is wall 0: `(offset,
/// depth_total - furniture +0xD - depth)`, wall 2: `(offset, depth)`, wall 1: `(depth, offset)`, wall
/// 3: `(width - furniture +0xD - depth, offset)`; any other wall value is treated as wall 0. On success
/// the non-null out pointers get the wall, the offset, the tile and `PlaceFurniture`'s two outputs, and
/// the result is `PlaceFurniture`'s; after the last failure 0. The drawn wall / offset persist into the
/// next try only through the argument slots (each try redraws them).
#[hook(addr = 0x593120, conv = thiscall, name = "Interior_c::PlaceFurnitureOnWall", this = "Interior_c",
       expect = "83 EC 10 8B 54 24 1C",
       unverified = "world6-1", route = "L2", hazards = "uninit-locals,heap-writes",
       note = "PlaceFurniture's two outputs are uninitialised stack words in the original (0 here); body continues in protection chunk 0x4015B9 (mov ecx, 0xBAB380); rand at_site 0x5931AE, 0x5931FB, 0x59325B")]
#[allow(clippy::too_many_arguments)]
pub fn place_furniture_on_wall(this: GPtr<u8>, group: i32, sub: i32, id: i32, height: u32, a5: i32, wall: i32, offset: i32,
                               depth: i32, out_wall: GPtr<i32>, out_offset: GPtr<i32>, out_x: GPtr<i32>, out_y: GPtr<i32>,
                               out_a: GPtr<i32>, out_b: GPtr<i32>) -> i32 {
    let info = || rd::<GPtr<u8>>(this, 0x14);
    let wealth = u32::from(rd::<u8>(info(), 0x1F));
    let f = get_furniture(g(FURNITURE_MAN), group, sub, id, wealth);
    let tries = if wall != -1 && offset != -1 { 1 } else { 100 };
    if f.is_null() {
        return 0;
    }
    let (mut wall_slot, mut off_slot) = (wall, offset);
    let mut i = 0;
    while i < tries {
        let w = if wall == -1 {
            wall_slot = rand_scaled(0x59_31AE, 4.0);
            wall_slot
        } else {
            wall_slot
        };
        let (tx, ty);
        if w == 1 || w == 3 {
            if offset == -1 {
                let span = i32::from(rd::<u8>(info(), 3)) - i32::from(rd::<u8>(f, 0xC));
                off_slot = rand_times(0x59_325B, span);
            }
            ty = off_slot;
            tx = if w == 1 {
                depth
            } else {
                i32::from(rd::<u8>(info(), 2)) - i32::from(rd::<u8>(f, 0xD)) - depth
            };
        } else {
            if offset == -1 {
                let span = i32::from(rd::<u8>(info(), 2)) - i32::from(rd::<u8>(f, 0xC));
                off_slot = rand_times(0x59_31FB, span);
            }
            tx = off_slot;
            ty = if w == 2 {
                depth
            } else {
                i32::from(rd::<u8>(info(), 3)) - i32::from(rd::<u8>(f, 0xD)) - depth
            };
        }
        let (mut oa, mut ob) = (0i32, 0i32);
        let r = GPtr::<u8>::addr(place_furniture(this, f, tx, ty, height, a5, wall_slot, p(&mut oa), p(&mut ob), 0)) as i32;
        if r != 0 {
            if !out_wall.is_null() {
                out_wall.write(wall_slot);
            }
            if !out_offset.is_null() {
                out_offset.write(off_slot);
            }
            if !out_x.is_null() {
                out_x.write(tx);
            }
            if !out_y.is_null() {
                out_y.write(ty);
            }
            if !out_a.is_null() {
                out_a.write(oa);
            }
            if !out_b.is_null() {
                out_b.write(ob);
            }
            return r;
        }
        i += 1;
    }
    0
}

/// `Interior_c::Office_PlaceEdgeFillers(int32 furnitureId, int32 x, int32 y, int32 dir, int32)`
/// (0x599210): draws `r = rand() * 100 / 32768` first; with fewer than one empty tile at `(x, y)`
/// towards `dir == 0 || dir == 2 ? 1 : 2` returns 1. The neighbour tile is `y + 1` (dir 2), `y - 1`
/// (dir 0), `x - 1` (dir 3), `x + 1` (dir 1). A given `furnitureId` places that furniture of group
/// 1; otherwise `r > 90` (once per game: latch 0xBB3DC8) adds interior info 7 at the neighbour (when
/// furniture 1/2 exists) and places furniture (1, 2), `r > 75` places furniture (8, 0) with a random
/// rotation 0..5, `r > 25` adds info 7 and places furniture (1, 3), else returns 1. The placement is
/// `PlaceFurniture(f, x, y, 0, 1, rot, ...)` at the original tile, even with a null furniture
/// (original bug kept), and its first output is the result.
#[hook(addr = 0x599210, conv = thiscall, name = "Interior_c::Office_PlaceEdgeFillers", this = "Interior_c",
       expect = "83 EC 0C 53 55 56 57 8B F9",
       unverified = "world6-1", route = "L2", hazards = "uninit-locals,heap-writes",
       note = "the result is PlaceFurniture's first output, an uninitialised stack word when it does not write it (0 here); body continues in protection chunk 0x405BEF (and eax, 0xFFFF); rand at_site 0x59921E; the protection reads 0x59921E (the jmp after the rand call) as a key: the expect stops before it")]
pub fn office_place_edge_fillers(this: GPtr<u8>, furniture_id: i32, tx: i32, ty: i32, dir: i32, _a4: i32) -> i32 {
    let r = rand_scaled(0x59_921E, 100.0);
    let n = if dir == 2 || dir == 0 { 1 } else { 2 };
    if get_num_empty_tiles(this, tx, ty, n, 1) < 1 {
        return 1;
    }
    let (mut nx, mut ny) = (tx, ty);
    match dir {
        2 => ny = ty + 1,
        0 => ny = ty - 1,
        3 => nx = tx - 1,
        1 => nx = tx + 1,
        _ => {}
    }
    let wealth = || u32::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 0x1F));
    let mut rot = dir;
    let f;
    if furniture_id != -1 {
        f = get_furniture(g(FURNITURE_MAN), 1, furniture_id, -1, wealth());
    } else if r > 90 && g::<u8>(OFFICE_LATCH).read() == 0 {
        f = get_furniture(g(FURNITURE_MAN), 1, 2, -1, wealth());
        if !f.is_null() {
            add_interior_info(this, 7, xi(nx).to_f32(), xi(ny).to_f32(), ((dir - 2) & 3) as u32, 0);
        }
        g::<u8>(OFFICE_LATCH).write(1);
    } else if r > 75 {
        rot = random_in_range(0, 5);
        f = get_furniture(g(FURNITURE_MAN), 8, 0, -1, wealth());
    } else if r > 25 {
        f = get_furniture(g(FURNITURE_MAN), 1, 3, -1, wealth());
        if !f.is_null() {
            add_interior_info(this, 7, xi(nx).to_f32(), xi(ny).to_f32(), ((dir - 2) & 3) as u32, 0);
        }
    } else {
        return 1;
    }
    let (mut o1, mut o2) = (0i32, 0i32);
    place_furniture(this, f, tx, ty, 0, 1, rot, p(&mut o1), p(&mut o2), 0);
    o1
}

/// `InteriorManager_c::AreAnimsLoaded(int32 animBlock)` (0x5980F0, static: `ret 4`, ECX unused): 0, 1 and 2 test
/// the anim blocks "int_house", "int_shop" and "int_office" by name, any other value is used as the
/// block index itself; true when that block's loaded byte is 1.
#[hook(addr = 0x5980F0, conv = stdcall, name = "InteriorManager_c::AreAnimsLoaded",
       expect = "8B 44 24 04 85 C0 75 1F",
       unverified = "world6-1", route = "L2", hazards = "none",
       note = "the case-0 name is pushed by protection chunk 0x4045B5 (decoded 0x866D40); AL result over (index << 5) in EAX")]
pub fn are_anims_loaded(block: i32) -> u8 {
    let idx = match block {
        0 => get_animation_block_index(g(0x86_6D40)),
        1 => get_animation_block_index(g(0x86_6D34)),
        2 => get_animation_block_index(g(0x86_6D28)),
        b => b,
    };
    u8::from(g::<u8>(ANIM_BLOCK_LOADED.wrapping_add((idx as usize) << 5)).read() == 1)
}

/// `InteriorGroup_c::FindInteriorInfo(eInteriorInfoType type, InteriorInfo_t** outInfo, Interior_c**
/// outInterior)` (0x594970): collects every unused (+1 == 0) info of `type` (+0) of the group's 8
/// interiors (+0x10; each has +0x40D infos at +0x590, 0x20 apart), then picks one with `rand() *
/// count / 32768` and returns true; none: both outputs null, false. The original's two 64-entry
/// stack arrays overflow above 64 matches (not modelled).
#[hook(addr = 0x594970, conv = thiscall, name = "InteriorGroup_c::FindInteriorInfo", this = "InteriorGroup_c",
       expect = "90 E9 2F 2C E7 FF",
       unverified = "world6-1", route = "L2", hazards = "entry-protection-read",
       note = "entry nop; jmp 0x4075A5 (sub esp, 0x208 and a return into 0x594976); keycells reader 0x401249 reads the entry rel32; rand at_site 0x5949DB; false leaves EAX's upper bytes = outInterior")]
pub fn find_interior_info(this: GPtr<u8>, ty: i32, out_info: GPtr<GPtr<u8>>, out_interior: GPtr<GPtr<u8>>) -> u8 {
    // 8 interiors of at most 127 infos each; the original's arrays hold 64
    let mut infos = [GPtr::<u8>::NULL; 8 * 127];
    let mut owners = [GPtr::<u8>::NULL; 8 * 127];
    let mut count = 0usize;
    for k in 0..8 {
        let it = rd::<GPtr<u8>>(this, 0x10 + 4 * k);
        if it.is_null() {
            continue;
        }
        let n = i32::from(rd::<i8>(it, 0x40D));
        let mut info = it.byte_add(0x590);
        for _ in 0..n.max(0) {
            if i32::from(rd::<i8>(info, 0)) == ty && rd::<u8>(info, 1) == 0 {
                infos[count] = info;
                owners[count] = it;
                count += 1;
            }
            info = info.byte_add(0x20);
        }
    }
    if count == 0 {
        out_info.write(GPtr::NULL);
        out_interior.write(GPtr::NULL);
        return 0;
    }
    let i = rand_times(0x59_49DB, count as i32) as usize;
    out_info.write(infos[i]);
    out_interior.write(owners[i]);
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_PLACE_FURNITURE_ON_WALL,
    HOOK_OFFICE_PLACE_EDGE_FILLERS,
    HOOK_ARE_ANIMS_LOADED,
    HOOK_FIND_INTERIOR_INFO,
];

