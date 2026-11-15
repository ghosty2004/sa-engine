//! FRONTIER-2 turn 2, lane B (batch `frontier2-b7`): more world functions on the phase B frontier (interiors).
//! Ported from the 1.0 US bytes; gta-reversed as a reading aid only (the bytes win). Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::crt::rand_at;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::fpu::ftol32;
use sa_math::x87::X87;

use super::common::{g, rd, wr, x, xi};
use super::interior_bounding_box::find_bounding_box;
use crate::bindings::{get_tile_centre, pickups_generate_new_one_weapon_type};
use sa_stream::bindings::generate_new_one;

/// `(rand() & 0xFFFF) * [0x858B14]` at `site` (`fild; fmul`), the interiors' random fraction.
fn rand_frac(site: u32) -> X87 {
    xi(rand_at(site) & 0xFFFF).fmul_m(g::<f32>(0x85_8B14).read())
}

/// `Interior_c::AddPickups()` (0x591F90, thiscall): nothing within 180000 ms of 0xBB3DC4. Otherwise up to 100 tries to
/// place one pickup: a random tile `(ftol(r * (width - 1)), ftol(r * (height - 1)))` of the interior info (+0x14: +2
/// width, +3 height; `fimul`) inside the grid whose tile type (+0x68 + 30 * x + y) is 0, 3 or 4; at its centre
/// (`GetTileCentre`) with chance 75 / 100 a pickup of model 0x8CD59C (type 8, ammo `10 - ftol(r * [0x85A708])`), else
/// 0.5 higher a weapon pickup (type 3, ammo `3 - ftol(r * [0x866D14])`) of weapon 5 (`r * 100` below 40), 22 (below
/// 80), 4 (below 90) or 25. Rand sites 0x591FCF, 0x592002, 0x59208C, 0x5920B4, 0x592123, 0x59216F.
#[hook(addr = 0x591F90, conv = thiscall, name = "Interior_c::AddPickups", expect = "A1 84 CB B7 00 8B 15 C4 3D BB 00",
       this = "Interior_c", unverified = "frontier2-b7", route = "L2", hazards = "heap-writes",
       note = "rand sites 0x591FCF 0x592002 0x59208C 0x5920B4 0x592123 0x59216F")]
pub fn interior_add_pickups(this: GPtr<u8>) {
    if g::<u32>(0xB7_CB84).read().wrapping_sub(g::<u32>(0xBB_3DC4).read()) < 0x2_BF20 {
        return;
    }
    let mut placed = 0;
    let mut tries = 0;
    while placed < 1 && tries < 100 {
        tries += 1;
        let info = || rd::<GPtr<u8>>(this, 0x14);
        let w = i32::from(rd::<u8>(info(), 2)) - 1;
        let tx = ftol32(rand_frac(0x59_1FCF).fmul(xi(w)));
        let h = i32::from(rd::<u8>(info(), 3)) - 1;
        let ty = ftol32(rand_frac(0x59_2002).fmul(xi(h)));
        if tx >= i32::from(rd::<u8>(info(), 2)) || ty >= i32::from(rd::<u8>(info(), 3)) || tx < 0 || ty < 0 {
            continue;
        }
        let tile = rd::<u8>(this, 0x68 + (tx * 30 + ty) as isize);
        if !matches!(tile, 0 | 3 | 4) {
            continue;
        }
        let mut pos = [0f32; 3];
        let pp = GPtr::from_raw(pos.as_mut_ptr()).cast::<u8>();
        get_tile_centre(this, xi(tx).to_f32(), xi(ty).to_f32(), pp);
        let hundred = g::<f32>(0x85_8628).read();
        if ftol32(rand_frac(0x59_208C).fmul_m(hundred)) < 0x4B {
            let ammo = 10 - ftol32(rand_frac(0x59_20B4).fmul_m(g::<f32>(0x85_A708).read()));
            let model = u32::from(g::<u16>(0x8C_D59C).read());
            generate_new_one(rd(pp, 0), rd(pp, 4), rd(pp, 8), model, 8, ammo as u32, 0, 0, GPtr::NULL);
        } else {
            wr::<f32>(pp, 8, x(rd(pp, 8)).fadd_m(g::<f32>(0x85_8B8C).read()).to_f32());
            let r = ftol32(rand_frac(0x59_2123).fmul_m(hundred));
            let weapon = if r < 0x28 {
                5
            } else if r < 0x50 {
                0x16
            } else if r >= 0x5A {
                0x19
            } else {
                4
            };
            let ammo = 3 - ftol32(rand_frac(0x59_216F).fmul_m(g::<f32>(0x86_6D14).read()));
            pickups_generate_new_one_weapon_type(rd::<u32>(pp, 0), rd::<u32>(pp, 4), rd::<u32>(pp, 8), weapon, 3, ammo, 0, 0);
        }
        placed += 1;
    }
}

/// `Interior_c::FindBoundingBox(int32 x, int32 y, int32* minX, int32* maxX, int32* minY, int32* maxY, int32* visited)`
/// (0x5922C0, thiscall, `ret 0x1C`): the flood fill over the interior's type-5 tiles (+0x68 + 30 * x + y; the first
/// tile read sits in protection chunk 0x40A4A3) in the grid (info +0x14: width +2, height +3): the neighbours (x - 1,
/// y), (x, y + 1) (only for y < 29), (x + 1, y) (only for x < 29) and (x, y - 1), each when inside, of type 5 and not
/// yet visited (`visited[30 * x + y]`), are marked and widen the box (`minX`, `maxY`, `maxX`, `minY`); the first three
/// recurse, the last continues in place (the compiler's tail loop). A negative y after the third neighbour's bounds
/// ends the call. Kept original detail: `y + 1` lives in the caller's `y` argument slot (not modelled).
#[hook(addr = 0x5922C0, conv = thiscall, name = "Interior_c::FindBoundingBox", expect = "53 8B 5C 24 08 55 8B 6C 24 24",
       this = "Interior_c", unverified = "frontier2-b7", route = "L2", hazards = "none",
       note = "y + 1 is kept in the caller's y argument slot")]
#[allow(clippy::too_many_arguments, clippy::int_plus_one)] // the bytes test both bounds
pub fn interior_find_bounding_box(this: GPtr<u8>, tx: i32, ty: i32, min_x: GPtr<i32>, max_x: GPtr<i32>, min_y: GPtr<i32>, max_y: GPtr<i32>, visited: GPtr<i32>) {
    let mut y = ty;
    let w = || i32::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 2));
    let h = || i32::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 3));
    let tile = |a: i32, b: i32| rd::<u8>(this, 0x68 + (a * 30 + b) as isize);
    let cell = |a: i32, b: i32| visited.byte_add(4 * (a * 30 + b) as isize);
    let recurse = |a: i32, b: i32| {
        find_bounding_box(this, a, b, min_x.cast(), max_x.cast(), min_y.cast(), max_y.cast(), visited.cast())
    };
    let x = tx;
    loop {
        if x > 0 && x - 1 < w() && y < h() && x - 1 >= 0 && y >= 0 && tile(x - 1, y) == 5 && cell(x - 1, y).read() == 0 {
            cell(x - 1, y).write(1);
            if x - 1 < min_x.read() {
                min_x.write(x - 1);
            }
            recurse(x - 1, y);
        }
        if y < 0x1D && x < w() && y + 1 < h() && x >= 0 && y + 1 >= 0 && tile(x, y + 1) == 5 && cell(x, y + 1).read() == 0 {
            cell(x, y + 1).write(1);
            if y + 1 > max_y.read() {
                max_y.write(y + 1);
            }
            recurse(x, y + 1);
        }
        if x < 0x1D && x + 1 < w() && y < h() && x + 1 >= 0 {
            if y < 0 {
                return;
            }
            if tile(x + 1, y) == 5 && cell(x + 1, y).read() == 0 {
                cell(x + 1, y).write(1);
                if x + 1 > max_x.read() {
                    max_x.write(x + 1);
                }
                recurse(x + 1, y);
            }
        }
        if y > 0 && x < w() && y - 1 < h() && x >= 0 && y - 1 >= 0 && tile(x, y - 1) == 5 && cell(x, y - 1).read() == 0 {
            cell(x, y - 1).write(1);
            if y - 1 < min_y.read() {
                min_y.write(y - 1);
            }
            y -= 1;
            continue;
        }
        return;
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_INTERIOR_ADD_PICKUPS, HOOK_INTERIOR_FIND_BOUNDING_BOX];
