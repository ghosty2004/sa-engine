//! The bedroom, lounge and kitchen-edge furnishing passes of `Interior_c`: furniture along the walls
//! and in the corners, clutter on the free worktop / table slots.

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::mem::GPtr;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use super::common::{rd, wr, x, xi};
use super::interior_room_items::{place_centred, place_object};
use super::interior_shop_furnishing::rand_scaled;

use crate::bindings::{add_interior_info, get_furniture, get_random_id, place_furniture, set_tiles_status};

orig! {
    /// `Interior_c::FindEmptyTiles(int32 w, int32 d, int32* outX, int32* outY)` (0x591C50)
    fn find_empty_tiles(this: GPtr<u8>, w: i32, d: i32, out_x: GPtr<i32>, out_y: GPtr<i32>) -> u8 = thiscall 0x591C50 => super::interior_furnish_lookup::find_empty_tiles;
    /// `Interior_c::Bedroom_AddTableItem(group, subGroup, dir, x, y, rot)` (0x593F10)
    fn bedroom_add_table_item(this: GPtr<u8>, group: i32, sub: i32, dir: i32, tx: i32, ty: i32, rot: i32) -> GPtr<u8> = thiscall 0x593F10 => super::interior_room_items::bedroom_add_table_item;
    /// `Interior_c::PlaceFurnitureOnWall(group, subGroup, id, float z, a5, wall, offset, depth, int32* outWall,
    /// int32* outOffset, int32* outX, int32* outY, int32* outW, int32* outD)` (0x593120)
    pub(crate) fn place_furniture_on_wall(this: GPtr<u8>, group: i32, sub: i32, id: i32, z: u32, a5: i32, wall: i32, offset: i32, depth: i32, out_wall: GPtr<i32>, out_offset: GPtr<i32>, out_x: GPtr<i32>, out_y: GPtr<i32>, out_w: GPtr<i32>, out_d: GPtr<i32>) -> i32 = thiscall 0x593120 => super::interior_wall_fillers::place_furniture_on_wall;
}

/// `g_furnitureMan`.
const FURNITURE_MAN: usize = 0xBA_B380;
const F_HALF: f32 = 0.5;
const F_1_5: f32 = 1.5;
/// The worktop clutter height (immediate 0x3F866666).
const WORKTOP_Z: f32 = f32::from_bits(0x3F86_6666);
const ONE_BITS: u32 = 0x3F80_0000;
/// The floor clutter height (immediate 0x3D4CCCCD).
const CLUTTER_Z: f32 = f32::from_bits(0x3D4C_CCCD);

fn man() -> GPtr<u8> {
    GPtr::from_addr(FURNITURE_MAN)
}

fn info(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0x14)
}

fn wealth(this: GPtr<u8>) -> u32 {
    u32::from(rd::<u8>(info(this), 0x1F))
}

fn style(this: GPtr<u8>) -> i32 {
    i32::from(rd::<i8>(this, 0x791))
}

/// `(f32)v + 0.5`.
fn tile_mid(v: i32) -> f32 {
    xi(v).fadd_m(F_HALF).to_f32()
}

/// The larger of two signed info bytes, 0 when not positive.
fn run_end(a: i8, b: i8) -> i32 {
    i32::from(a.max(b)).max(0)
}

/// The kitchen's worktop slots: the original's four 32-entry arrays (Y, X, rotation, taken mark) in one
/// block, so a room with more slots spills from one array into the next as the bytes do.
struct Worktops([u32; 0xA0]);

const SLOT_Y: usize = 0;
const SLOT_X: usize = 0x20;
const SLOT_ROT: usize = 0x40;
const SLOT_MARK: usize = 0x60;

impl Worktops {
    fn get(&self, array: usize, i: i32) -> u32 {
        self.0[(array as isize + i as isize) as usize]
    }
    fn f(&self, array: usize, i: i32) -> f32 {
        f32::from_bits(self.get(array, i))
    }
    fn set(&mut self, array: usize, i: i32, v: u32) {
        self.0[(array as isize + i as isize) as usize] = v;
    }
    fn push(&mut self, n: &mut i32, xv: f32, yv: f32, rot: f32) {
        self.set(SLOT_X, *n, xv.to_bits());
        self.set(SLOT_ROT, *n, rot.to_bits());
        self.set(SLOT_Y, *n, yv.to_bits());
        *n += 1;
    }
    /// `PlaceObject(stealable, f, X[i], Y[i], 1.05, Rot[i])`.
    fn place(&self, this: GPtr<u8>, stealable: u32, f: GPtr<u8>, i: i32) {
        place_object(this, stealable, f, self.f(SLOT_X, i), self.f(SLOT_Y, i), WORKTOP_Z, self.f(SLOT_ROT, i));
    }
}

/// A free slot index `ftol(rand * count / 32768)`, retried up to 30 times while it is marked; -1 when the
/// 30th draw is reached (even when that draw was free).
fn second_slot(slots: &Worktops, count_f: f32) -> i32 {
    let mut i = rand_scaled(0x596E60, count_f);
    if slots.get(SLOT_MARK, i) != ONE_BITS {
        return i;
    }
    let mut k = 0;
    while k < 30 {
        i = rand_scaled(0x596E9A, count_f);
        k += 1;
        if slots.get(SLOT_MARK, i) != ONE_BITS {
            break;
        }
    }
    if k == 30 { -1 } else { i }
}

/// The clutter chance in percent from the room's wealth: >= 75: 0..19, >= 50: 20..49, else 50..99.
fn clutter_chance(wl: u8) -> i32 {
    if wl >= 75 {
        rand_scaled(0x596EDB, 20.0)
    } else if wl >= 50 {
        20 - rand_scaled(0x596F06, -30.0)
    } else {
        50 - rand_scaled(0x596F30, -50.0)
    }
}

/// `Interior_c::Kitchen_FurnishEdges()` (0x596930, thiscall, no arguments; EAX is left from the last call and
/// unused). With W1 / D1 = width / depth - 1 and the style id +0x791: the back corner units (group 4 / 7) at
/// (0, D1) and (W1, D1); the side runs end at the larger of the door / window ends on each side (both zero:
/// random lengths `GetRandomNumberInRange(0, D1)`), blocked with status 2 and capped by the end units 4 / 0 and
/// 4 / 2; the sink (4 / 3) on the back wall at the window (its info at the next tile), then 4 / 5, 4 / 4 and
/// 4 / 6 anywhere on a wall. The worktops (4 / 1) fill the left side, the back and the right side, each placed
/// one recording a slot (centre, rotation 90 / 0 / 270). A corner item (4 / 10) goes on the left corner on a
/// coin flip, else the right one. Two slots get the stealable items 4 / 8 and 4 / 9 (the second redrawn up to
/// 30 times while taken), and every unmarked slot gets clutter (8 / 3 or 8 / 4) by the wealth's chance.
#[hook(addr = 0x596930, conv = thiscall, name = "Interior_c::Kitchen_FurnishEdges", this = "Interior_c",
       expect = "55 8B EC 83 E4 F8 81 EC 28 02 00 00",
       unverified = "frontier2-b35", route = "L2", hazards = "heap-writes",
       note = "rand at_site 0x596D90, 0x596E2F, 0x596E60, 0x596E9A, 0x596EDB / 0x596F06 / 0x596F30, 0x596F8D, 0x596FB4")]
pub fn kitchen_furnish_edges(this: GPtr<u8>) {
    use sa_core::bindings::general_random_int_in_range;
    use sa_core::field::ptr_of as p;
    let box_ = info(this);
    let w1 = i32::from(rd::<u8>(box_, 2)) - 1;
    let d1 = i32::from(rd::<u8>(box_, 3)) - 1;
    let id = style(this);
    let (mut ow, mut od) = (0i32, 0i32);
    let mut place = |f: GPtr<u8>, tx: i32, ty: i32, rot: i32| {
        !place_furniture(this, f, tx, ty, 0, 1, rot, p(&mut ow), p(&mut od), 0).is_null()
    };

    let corner = get_furniture(man(), 4, 7, id, wealth(this));
    let corner_l = place(corner, 0, d1, 1);
    let corner_r = place(corner, w1, d1, 0);

    let box_ = info(this);
    let left_end = run_end(rd::<i8>(box_, 7), rd::<i8>(box_, 0xD));
    let right_end = run_end(rd::<i8>(box_, 9), rd::<i8>(box_, 0xF));
    let (l, r) = if right_end > 0 {
        (left_end, right_end)
    } else if left_end > 0 {
        (left_end, 0)
    } else {
        let l = general_random_int_in_range(0, d1);
        (l, general_random_int_in_range(0, d1))
    };
    if l > 0 {
        set_tiles_status(this, 0, 0, 1, l, 2, 0);
    }
    if r > 0 {
        set_tiles_status(this, w1, 0, 1, r, 2, 0);
    }
    place(get_furniture(man(), 4, 0, style(this), wealth(this)), 0, l, 1);
    place(get_furniture(man(), 4, 2, style(this), wealth(this)), w1, r, 3);

    let window = rd::<u8>(info(this), 0x10);
    let window = if window == 0xFF { -1 } else { i32::from(window as i8) };
    let (mut out_wall, mut out_pos) = (0i32, 0i32);
    let on_wall = |sub: i32, wall: i32, pos: i32, out_wall: &mut i32, out_pos: &mut i32| {
        place_furniture_on_wall(this, 4, sub, style(this), 0, 1, wall, pos, 0, GPtr::from_raw(out_wall),
                                GPtr::from_raw(out_pos), GPtr::NULL, GPtr::NULL, GPtr::NULL, GPtr::NULL)
    };
    if on_wall(3, 0, window, &mut out_wall, &mut out_pos) != 0 {
        out_pos += 1;
        let d = i32::from(rd::<u8>(info(this), 3));
        let py = xi(d).fsub_m(F_1_5).to_f32();
        add_interior_info(this, 5, xi(out_pos).to_f32(), py, 2, 0);
    }
    for sub in [5, 4, 6] {
        on_wall(sub, -1, -1, &mut out_wall, &mut out_pos);
    }

    let worktop = get_furniture(man(), 4, 1, style(this), wealth(this));
    let mut slots = Worktops([0; 0xA0]);
    let mut count = 0i32;
    for ty in l + 1..d1 {
        if place(worktop, 0, ty, 1) {
            slots.push(&mut count, F_HALF, tile_mid(ty), 90.0);
        }
    }
    for tx in 1..w1 {
        if place(worktop, tx, d1, 0) {
            slots.push(&mut count, tile_mid(tx), tile_mid(d1), 0.0);
        }
    }
    for ty in r + 1..=d1 - 1 {
        if place(worktop, w1, ty, 3) {
            slots.push(&mut count, tile_mid(w1), tile_mid(ty), 270.0);
        }
    }

    let corner_item = get_furniture(man(), 4, 10, -1, wealth(this));
    let flip = at_site(0x596D90, rand);
    if flip < 0x3FFF && corner_l {
        let d = xi(i32::from(rd::<u8>(info(this), 3)));
        place_object(this, 1, corner_item, F_HALF, d.fsub_m(F_HALF).to_f32(), WORKTOP_Z, 45.0);
    } else if corner_r {
        let b = info(this);
        let d = xi(i32::from(rd::<u8>(b, 3))).fsub_m(F_HALF).to_f32();
        let w = xi(i32::from(rd::<u8>(b, 2))).fsub_m(F_HALF).to_f32();
        place_object(this, 1, corner_item, w, d, WORKTOP_Z, 315.0);
    }
    if count <= 0 {
        return;
    }

    let count_f = xi(count).to_f32();
    let first = rand_scaled(0x596E2F, count_f);
    slots.set(SLOT_MARK, first, ONE_BITS);
    let second = second_slot(&slots, count_f);
    let chance = clutter_chance(wealth(this) as u8);
    for i in 0..count {
        let free = !x(slots.f(SLOT_MARK, i)).fcom(x(0.0)).test_parity(0x44);
        if free && rand_scaled(0x596F8D, 100.0) < chance {
            let sub = if at_site(0x596FB4, rand) < 0x3FFF { 3 } else { 4 };
            let f = get_furniture(man(), 8, sub, -1, wealth(this));
            if !f.is_null() {
                slots.place(this, 0, f, i);
            }
        }
    }
    if first != -1 {
        slots.place(this, 1, get_furniture(man(), 4, 8, -1, wealth(this)), first);
    }
    if second != -1 {
        slots.place(this, 1, get_furniture(man(), 4, 9, -1, wealth(this)), second);
    }
}

/// The clutter threshold of a bedroom: >= 75: 0..19, >= 50: 20..49, else `GetRandomNumberInRange(50, 100)`.
fn bedroom_clutter_chance(wl: u8) -> i32 {
    if wl >= 75 {
        rand_scaled(0x5943CE, 20.0)
    } else if wl >= 50 {
        20 - rand_scaled(0x5943F7, -30.0)
    } else {
        sa_core::bindings::general_random_int_in_range(50, 100)
    }
}

/// A bedside table's info and blocked tile by the wall it stands against:
/// `(info type, dir, info x, info y, tile x, tile y)`.
type Bedside = (i32, i32, i32, i32, i32, i32);

/// `Interior_c::FurnishBedroom()` (0x593FC0, thiscall, no arguments). The style id (+0x791) = a random id of
/// group 3 / 1; the door tiles 2 x 2 get status 7. The bed (3 / 0) goes anywhere on a wall; bedside tables
/// (3 / 1) are tried beside it at `pos - 1` (only when `pos > 0`) and `pos + 2`, each placed one adding an
/// info towards the bed and blocking its front tile. Two wall units (3 / 3, 3 / 2); on a percent roll below 75
/// one or two wall tables (2 / 6) with a table item (2 / 8 below 25 or from 50, 2 / 3 from 25); five clutter
/// items (8 / 2, 5, 4, 3, 6, the first and last needing 2 x 2 free tiles) each by the wealth's chance; then
/// the rug (8 / 1) centred. When the bed finds no wall, the wall / position (and the two coordinate locals
/// the info cases share) are the original's uninitialised stack words, 0 here.
#[hook(addr = 0x593FC0, conv = thiscall, name = "Interior_c::FurnishBedroom", this = "Interior_c",
       expect = "83 EC 20 53 55 56 8B F1",
       unverified = "frontier2-b35", route = "L2", hazards = "uninit-locals,heap-writes",
       note = "rand at_site 0x5942B0, 0x5943CE / 0x5943F7, 0x594431, 0x5944D2, 0x59456F, 0x59460C, 0x5946A9")]
pub fn furnish_bedroom(this: GPtr<u8>) {
    use sa_core::field::ptr_of as p;
    let id = get_random_id(man(), 3, 1, wealth(this));
    wr::<u8>(this, 0x791, id as u8);
    set_tiles_status(this, i32::from(rd::<i8>(info(this), 5)) - 1, 0, 2, 2, 7, 0);
    let (mut wall, mut pos, mut a, mut b, mut tx, mut ty) = (0i32, 0i32, 0i32, 0i32, 0i32, 0i32);
    let null = GPtr::<i32>::NULL;
    let bed = place_furniture_on_wall(this, 3, 0, -1, 0, 1, -1, -1, 0, p(&mut wall).cast(), p(&mut pos).cast(),
                                      null, null, null, null);
    let bedside = |wall: i32, offset: i32| {
        place_furniture_on_wall(this, 3, 1, style(this), 0, 1, wall, offset, 0, null, null, null, null, null, null) != 0
    };
    let size = |o: isize| i32::from(rd::<u8>(info(this), o));
    let add_bedside = |(ty_, dir, ix, iy, tile_x, tile_y): Bedside| {
        add_interior_info(this, ty_, xi(ix).to_f32(), xi(iy).to_f32(), dir as u32, bed as u32);
        set_tiles_status(this, tile_x, tile_y, 1, 1, 2, 0);
    };
    if pos > 0 && bedside(wall, pos - 1) {
        let q = pos - 1;
        match wall {
            1 => (b, a) = (1, q),
            3 => (b, a) = (size(2) - 2, q),
            0 => (a, b) = (size(3) - 2, q),
            2 => (a, b) = (1, q),
            _ => {}
        }
        let (ty_, dir, tile_x, tile_y) = match wall {
            1 => (3, 2, 2, q),
            3 => (4, 2, size(2) - 3, q),
            0 => (3, 1, q, size(3) - 3),
            2 => (4, 1, q, 2),
            _ => (b, b, b, b),
        };
        add_bedside((ty_, dir, b, a, tile_x, tile_y));
    }
    if bedside(wall, pos + 2) {
        let q = pos + 2;
        match wall {
            1 => (a, b) = (1, q),
            3 => (a, b) = (size(2) - 2, q),
            0 => (b, a) = (size(3) - 2, q),
            2 => (b, a) = (1, q),
            _ => {}
        }
        let (ty_, dir, tile_x, tile_y) = match wall {
            1 => (4, 0, 2, q),
            3 => (3, 0, size(2) - 3, q),
            0 => (4, 3, q, size(3) - 3),
            2 => (3, 3, q, 2),
            _ => (b, b, b, b),
        };
        add_bedside((ty_, dir, a, b, tile_x, tile_y));
    }
    for sub in [3, 2] {
        place_furniture_on_wall(this, 3, sub, style(this), 0, 1, -1, -1, 0, null, null, null, null, null, null);
    }

    let roll = rand_scaled(0x5942B0, 100.0);
    let mut table = |item: i32| {
        if place_furniture_on_wall(this, 2, 6, -1, 0, 1, -1, -1, 0, p(&mut wall).cast(), null, p(&mut tx).cast(),
                                   p(&mut ty).cast(), null, null) != 0
        {
            bedroom_add_table_item(this, 2, item, wall, tx, ty, wall);
        }
    };
    if roll < 25 {
        table(8);
    } else if roll < 50 {
        table(3);
    } else if roll < 75 {
        table(8);
        table(3);
    }

    let chance = bedroom_clutter_chance(wealth(this) as u8);
    for (site, k, size, sub, mark) in [
        (0x594431, 60.0, 2, 2, 2),
        (0x5944D2, 100.0, 1, 5, 1),
        (0x59456F, 100.0, 1, 4, 1),
        (0x59460C, 100.0, 1, 3, 1),
        (0x5946A9, 100.0, 2, 6, 1),
    ] {
        if rand_scaled(site, k) < chance && find_empty_tiles(this, size, size, p(&mut a).cast(), p(&mut b).cast()) != 0 {
            let f = get_furniture(man(), 8, sub, -1, wealth(this));
            let py = tile_mid(b);
            place_object(this, 0, f, tile_mid(a), py, CLUTTER_Z, 0.0);
            set_tiles_status(this, a, b, mark, mark, 2, 0);
        }
    }
    place_centred(this, get_furniture(man(), 8, 1, -1, wealth(this)));
}

pub const HOOKS: &[HookReg] = &[HOOK_KITCHEN_FURNISH_EDGES, HOOK_FURNISH_BEDROOM];
