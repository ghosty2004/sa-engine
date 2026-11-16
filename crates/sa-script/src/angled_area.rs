//! The angled-area script checks (`IS_CHAR_IN_ANGLED_AREA_*` and friends; FRONTIER-2 lane C turn 3, batch
//! `frontier2-c7`). Unreversed in gta-reversed; ported from the 1.0 US bytes, the x87 data flow (which values are
//! rounded to f32 by a store and which stay on the stack) followed instruction by instruction. Unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::field::rd;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use crate::bindings::{
    draw_debug_angled_square, highlight_important_angled_area, is_ped_stopped, shot_info_get_flame_thrower_shot_posn,
};
use crate::vm::{collect_parameters_call, update_compare_flag_call};
use sa_core::bindings::{general_get_radian_angle_between_points, vector2d_normalise};
use sa_world::bindings::world_process_vertical_line;
use sa_world::world::common::entity_pos;

/// `ScriptParams[k]` (0xA43C78) as a dword.
fn param(k: usize) -> u32 {
    GPtr::<u32>::from_addr(0xA43C78 + 4 * k).read()
}

fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// The rectangle an angled-area check tests against: the base edge (x1, y1) → (x2, y2), the far corners (x3, y3) /
/// (x4, y4) `width` away on the left side, the edge vector (`dx` as the x87 holds it when the edge is normalised,
/// `dy` f32), its squared length (as the x87 holds it), both side lengths (f32) and the second side vector (f32,
/// still to be normalised).
struct Area {
    q: Quad,
    dx: X87,
    dy: f32,
    sq: X87,
    len1: f32,
    len2: f32,
    side: [f32; 2],
}

/// The base edge (x1, y1) → (x2, y2) (the parameters' bits) and the far corners (x3, y3) / (x4, y4) of an area.
struct Quad {
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
    x3: f32,
    y3: f32,
    x4: f32,
    y4: f32,
}

/// `(x1, y1, x2, y2, width, highlight, z range)` of [`area_params`].
type AreaParams = (u32, u32, u32, u32, u32, u32, Option<(f32, f32)>);

/// The script parameters of an angled-area check whose first coordinate is `ScriptParams[b]`: `(x1, y1, x2, y2,
/// width, highlight, z range)`. 2D: x1, y1, x2, y2, width, highlight. 3D: x1, y1, z1, x2, y2, z2, width, highlight,
/// the z range sorted (swapped unless `z1 < z2` or equal, NaN included).
fn area_params(b: usize, is3d: bool) -> AreaParams {
    let (x1, y1) = (param(b), param(b + 1));
    if is3d {
        let (mut zmin, mut zmax) = (param(b + 2), param(b + 5));
        if !X87::from_f32(f(zmin)).fcom(X87::from_f32(f(zmax))).test_any(0x41) {
            (zmin, zmax) = (zmax, zmin);
        }
        (x1, y1, param(b + 3), param(b + 4), param(b + 6), param(b + 7), Some((f(zmin), f(zmax))))
    } else {
        (x1, y1, param(b + 2), param(b + 3), param(b + 4), param(b + 5), None)
    }
}

/// The far corners every angled-area check computes the same way (inlined in each): `angle =
/// GetRadianAngleBetweenPoints(x1, y1, x2, y2) + PI/2` brought into [0, 2PI] (`while < 0: += 2PI`, `while > 2PI: -=
/// 2PI`, both against the f32 constants); `x3 = x2 + w sin`, `y3 = y2 + (-w cos)` (the x87 product), `x4 = x1 + w
/// sin`, `y4 = y1 + f32(-w cos)`. Returns `(x3, y3, x4, y4)`.
fn corners(x1: u32, y1: u32, x2: u32, y2: u32, width: u32) -> (f32, f32, f32, f32) {
    let (fx1, fy1, fx2, fy2, w) = (f(x1), f(y1), f(x2), f(y2), f(width));
    let two_pi = f(0x40C90FDB);
    let mut a = general_get_radian_angle_between_points(fx1, fy1, fx2, fy2).fadd_m(f(0x3FC90FDB));
    while !a.fcom(X87::zero()).test_parity(5) {
        a = a.fadd_m(two_pi);
    }
    while !a.fcom(X87::from_f32(two_pi)).test_any(0x41) {
        a = a.fsub_m(two_pi);
    }
    let sinw = a.fsin().fmul_m(w);
    let x3 = X87::from_f32(fx2).fadd(sinw).to_f32();
    let ncw = a.fcos().fmul_m(w).fchs();
    let ncw32 = ncw.to_f32();
    let y3 = ncw.fadd_m(fy2).to_f32();
    let x4 = sinw.fadd_m(fx1).to_f32();
    let y4 = X87::from_f32(ncw32).fadd_m(fy1).to_f32();
    (x3, y3, x4, y4)
}

/// The area of `CharInAngledAreaCheckCommand`: [`corners`]; `dx`, `dy` stored (f32), their squared length stored
/// (f32), `len1 = sqrt` of the x87 sum; `len2` from the x87 differences `x4 - x1`, `y4 - y1`.
fn build_area(x1: u32, y1: u32, x2: u32, y2: u32, width: u32) -> Area {
    let (fx1, fy1, fx2, fy2) = (f(x1), f(y1), f(x2), f(y2));
    let (x3, y3, x4, y4) = corners(x1, y1, x2, y2, width);
    let dx = X87::from_f32(fx2).fsub_m(fx1).to_f32();
    let dy = X87::from_f32(fy2).fsub_m(fy1).to_f32();
    let b = X87::from_f32(x4).fsub_m(fx1);
    let c = X87::from_f32(y4).fsub_m(fy1);
    let side = [b.to_f32(), c.to_f32()];
    let sq = X87::from_f32(dy).fmul_m(dy).fadd(X87::from_f32(dx).fmul_m(dx));
    let len1 = sq.fsqrt().to_f32();
    let len2 = c.fmul(c).fadd(b.fmul(b)).fsqrt().to_f32();
    Area { q: Quad { x1, y1, x2, y2, x3, y3, x4, y4 }, dx: X87::from_f32(dx), dy, sq: X87::from_f32(sq.to_f32()), len1, len2, side }
}

/// The area of `ObjectInAngledAreaCheckCommand`: [`corners`]; `dx` stays on the x87 (80 bits) for the squared length
/// (`dy` stored, f32: `dy*dy + dx*dx`, kept on the x87) and for the edge's normalisation; `len2` from the stored
/// (f32) side vector.
fn build_area_x87(x1: u32, y1: u32, x2: u32, y2: u32, width: u32) -> Area {
    let (fx1, fy1, fx2, fy2) = (f(x1), f(y1), f(x2), f(y2));
    let (x3, y3, x4, y4) = corners(x1, y1, x2, y2, width);
    let dx = X87::from_f32(fx2).fsub_m(fx1);
    let dy = X87::from_f32(fy2).fsub_m(fy1).to_f32();
    let side = [X87::from_f32(x4).fsub_m(fx1).to_f32(), X87::from_f32(y4).fsub_m(fy1).to_f32()];
    let sq = X87::from_f32(dy).fmul_m(dy).fadd(dx.fmul(dx));
    let len1 = sq.fsqrt().to_f32();
    let len2 = X87::from_f32(side[1]).fmul_m(side[1]).fadd(X87::from_f32(side[0]).fmul_m(side[0])).fsqrt().to_f32();
    Area { q: Quad { x1, y1, x2, y2, x3, y3, x4, y4 }, dx, dy, sq, len1, len2, side }
}

/// The 2D test of a point against the area (inlined in each check): with `r = p - (x1, y1)` (f32), the projection on
/// the edge's unit vector (the inline `CVector2D::Normalise` of (dx, dy): x from the x87 `1/sqrt(sq) * dx`, y stored;
/// `sq <= 0` or NaN: x = 1.0, y unchanged) lies in [0, len1], and the projection on the normalised side vector
/// (`CVector2D::Normalise`, 0x44E480) in [0, len2].
fn inside_2d(ar: &mut Area, px: f32, py: f32) -> bool {
    let rx = X87::from_f32(px).fsub_m(f(ar.q.x1)).to_f32();
    let ry = X87::from_f32(py).fsub_m(f(ar.q.y1)).to_f32();
    let l = ar.sq;
    let (nx, ny) = if l.fcom(X87::zero()).test_parity(0x41) {
        let inv = l.fsqrt().fdivr_m(1.0);
        let inv32 = inv.to_f32();
        (inv.fmul(ar.dx), X87::from_f32(inv32).fmul_m(ar.dy).to_f32())
    } else {
        (X87::one(), ar.dy)
    };
    let d1 = X87::from_f32(ry).fmul_m(ny).fadd(X87::from_f32(rx).fmul(nx));
    in_range(d1, ar.len1) && side_ok(&mut ar.side, ar.len2, rx, ry)
}

/// `d` in [0, `len`] (`fcom 0; test ah, 1; jne` fails below 0 or NaN, `fcomp len; test ah, 41h; jp` above `len`).
fn in_range(d: X87, len: f32) -> bool {
    !(d.fcom(X87::zero()).test_any(1) || d.fcom(X87::from_f32(len)).test_parity(0x41))
}

/// The second half of every 2D test: the side vector normalised in place (`CVector2D::Normalise`, 0x44E480), the
/// projection of `r` on it in [0, `len2`].
fn side_ok(side: &mut [f32; 2], len2: f32, rx: f32, ry: f32) -> bool {
    vector2d_normalise(GPtr::from_raw(side as *mut [f32; 2]).cast());
    let d2 = X87::from_f32(ry).fmul_m(side[1]).fadd(X87::from_f32(rx).fmul_m(side[0]));
    in_range(d2, len2)
}

/// The angled-area test of a position (0x477DA0, cdecl `(const CVector* pos, float x1, float y1, float x2, float y2,
/// float width)`, AL; FRONTIER-2 lane C turn 7; reached from `GET_RANDOM_CAR_OF_TYPE_IN_ANGLED_AREA_NO_SAVE` 09C0).
/// The [`corners`], `dx` stored (f32) and `dy` kept on the x87, the squared length `dy*dy + dx*dx` on the x87 with
/// `len1 = f32(sqrt)`, the stored side vector and `len2`; `r = pos - (x1, y1)` (f32). The edge's unit vector: `sq > 0`
/// or NaN: x = `1/sqrt(sq) * dx`, y = `f32(1/sqrt(sq)) * dy` (both on the x87), else (1.0, dy); then the two range
/// tests (`in_range`, `side_ok`). The debug square when the flag byte 0x859CF8 is set; no highlight.
#[hook(addr = 0x477DA0, conv = cdecl, name = "(missed by ghidra: cat7-call)", expect = "8B 44 24 08 83 EC 24",
       unverified = "frontier2-c13", route = "L2", hazards = "x87-transcendental")]
pub fn position_in_angled_area(pos: GPtr<u8>, x1: u32, y1: u32, x2: u32, y2: u32, width: u32) -> u8 {
    let (fx1, fy1, fx2, fy2) = (f(x1), f(y1), f(x2), f(y2));
    let (x3, y3, x4, y4) = corners(x1, y1, x2, y2, width);
    let dx = X87::from_f32(fx2).fsub_m(fx1).to_f32();
    let dy = X87::from_f32(fy2).fsub_m(fy1);
    let mut side = [X87::from_f32(x4).fsub_m(fx1).to_f32(), X87::from_f32(y4).fsub_m(fy1).to_f32()];
    let sq = dy.fmul(dy).fadd(X87::from_f32(dx).fmul_m(dx));
    let len1 = sq.fsqrt().to_f32();
    let len2 = X87::from_f32(side[1]).fmul_m(side[1]).fadd(X87::from_f32(side[0]).fmul_m(side[0])).fsqrt().to_f32();
    let rx = X87::from_f32(rd::<f32>(pos, 0)).fsub_m(fx1).to_f32();
    let ry = X87::from_f32(rd::<f32>(pos, 4)).fsub_m(fy1).to_f32();
    let (nx, ny) = if sq.fcom(X87::zero()).test_parity(0x41) {
        let inv = sq.fsqrt().fdivr_m(1.0);
        (inv.fmul_m(dx), X87::from_f32(inv.to_f32()).fmul(dy))
    } else {
        (X87::one(), dy)
    };
    let d1 = X87::from_f32(ry).fmul(ny).fadd(X87::from_f32(rx).fmul(nx));
    let inside = in_range(d1, len1) && side_ok(&mut side, len2, rx, ry);
    if GPtr::<u8>::from_addr(0x859CF8).read() != 0 {
        draw_debug_angled_square(x1, y1, x2, y2, x3.to_bits(), y3.to_bits(), x4.to_bits(), y4.to_bits());
    }
    u8::from(inside)
}

/// The z test of the 3D commands: `zmin <= z` and `z <= zmax` (a NaN fails); always true for the 2D ones.
fn z_ok(zr: Option<(f32, f32)>, pz: f32) -> bool {
    match zr {
        Some((zmin, zmax)) => {
            !X87::from_f32(pz).fcom(X87::from_f32(zmin)).test_any(1)
                && !X87::from_f32(pz).fcom(X87::from_f32(zmax)).test_parity(0x41)
        }
        None => true,
    }
}

/// The tails of every check: the area highlighted when the highlight parameter is set (`HighlightImportantAngledArea`
/// with id `this + this->m_pBaseIP` and z = `(zmax + zmin) * 0.5`, or -100.0 for the 2D commands), then the debug
/// square of a 2D command when the debug flag byte 0x859CF8 is set.
fn highlight(this: GPtr<u8>, ar: &Quad, on: u32, z: Option<(f32, f32)>) {
    let (x3, y3, x4, y4) = (ar.x3.to_bits(), ar.y3.to_bits(), ar.x4.to_bits(), ar.y4.to_bits());
    if on != 0 {
        let z = match z {
            Some((zmin, zmax)) => X87::from_f32(zmax).fadd_m(zmin).fmul_m(0.5).to_f32().to_bits(),
            None => 0xC2C80000,
        };
        let id = rd::<u32>(this, 0x14).wrapping_add(this.addr() as u32);
        highlight_important_angled_area(id, ar.x1, ar.y1, ar.x2, ar.y2, x3, y3, x4, y4, z);
    }
    if GPtr::<u8>::from_addr(0x859CF8).read() != 0 && z.is_none() {
        draw_debug_angled_square(ar.x1, ar.y1, ar.x2, ar.y2, x3, y3, x4, y4);
    }
}

/// `CRunningScript::CharInAngledAreaCheckCommand(int32 command)` (0x487F60, thiscall, `ret 4`): commands 0x5F6..0x601
/// (`IS_CHAR_IN_ANGLED_AREA_[ON_FOOT_|IN_CAR_][STOPPED_]2D/3D`; 0x5FC..0x601 are 3D with 9 parameters, else 7). The
/// ped from its handle (`CPedPool::GetAtRef` inlined); the stopped commands (0x5F9..0x5FB, 0x5FF..0x601) whose ped
/// `IsPedStopped` says no skip the test (result 0). The 3D z range is sorted (swapped when `z1 > z2`). The position is
/// the ped's car's when it is in one (+0x46C bit 8, car +0x58C non-null), else its own. Inside the rectangle (and the
/// z range) the result is `(command - 0x5F6) % 3`: 0 any, 1 on foot only, 2 in a car only. `UpdateCompareFlag`, then
/// the highlight and debug tails.
#[hook(addr = 0x487F60, conv = thiscall, name = "CRunningScript::CharInAngledAreaCheckCommand", expect = "83 EC 6C 53 55",
       unverified = "frontier2-c7", route = "L2", hazards = "x87-transcendental")]
pub fn char_in_angled_area_check_command(this: GPtr<u8>, command: i32) {
    let is3d = (0x5FC..=0x601).contains(&command);
    collect_parameters_call(this, if is3d { 9 } else { 7 });
    let ped = sa_core::pool::pool_at_ref(sa_core::pool::PED_POOL.get(), param(0) as i32).cast::<u8>();
    let mut result = 0u8;
    let t = command.wrapping_sub(0x5F9) as u32;
    let not_stopped = t <= 8 && !(3..=5).contains(&t) && is_ped_stopped(ped) == 0;
    let (x1, y1, x2, y2, width, on, zr) = area_params(1, is3d);
    let mut ar = build_area(x1, y1, x2, y2, width);
    if !not_stopped {
        let in_car = (rd::<u32>(ped, 0x46C) >> 8) & 1;
        let car = rd::<GPtr<u8>>(ped, 0x58C);
        let p = if in_car != 0 && !car.is_null() { entity_pos(car) } else { entity_pos(ped) };
        let (px, py, pz) = (rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8));
        if inside_2d(&mut ar, px, py) && z_ok(zr, pz) {
            let k = command.wrapping_sub(0x5F6) as u32;
            if k <= 0xB {
                match k % 3 {
                    0 => result = 1,
                    1 if in_car == 0 => result = 1,
                    2 if in_car != 0 => result = 1,
                    _ => {}
                }
            }
        }
    }
    update_compare_flag_call(this, u32::from(result));
    highlight(this, &ar.q, on, zr);
}

/// `CRunningScript::ObjectInAngledAreaCheckCommand(int32 command)` (0x4883F0, thiscall, `ret 4`): command 0x8E4
/// (`IS_OBJECT_IN_ANGLED_AREA_3D`, 9 parameters) or any other (the 2D form, 7). The object from its handle
/// (`CPool::GetAtRef` inlined; a stale handle gives null, whose matrix the original then reads). The area as
/// [`build_area_x87`]; inside it (and the z range) the result is 1. `UpdateCompareFlag`, then the highlight and debug
/// tails.
#[hook(addr = 0x4883F0, conv = thiscall, name = "CRunningScript::ObjectInAngledAreaCheckCommand", expect = "8B 44 24 04 83 EC 68",
       unverified = "frontier2-c9", route = "L2", hazards = "x87-transcendental")]
pub fn object_in_angled_area_check_command(this: GPtr<u8>, command: i32) {
    let is3d = command == 0x8E4;
    collect_parameters_call(this, if is3d { 9 } else { 7 });
    let obj = sa_core::pool::pool_at_ref(sa_core::pool::OBJECT_POOL.get(), param(0) as i32).cast::<u8>();
    let (x1, y1, x2, y2, width, on, zr) = area_params(1, is3d);
    let mut ar = build_area_x87(x1, y1, x2, y2, width);
    let p = entity_pos(obj);
    let (px, py, pz) = (rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8));
    let result = inside_2d(&mut ar, px, py) && z_ok(zr, pz);
    update_compare_flag_call(this, u32::from(result));
    highlight(this, &ar.q, on, zr);
}

/// The inline `CVector2D::Normalise` of `FlameInAngledAreaCheckCommand` (in place): `sq = v.y*v.y + v.x*v.x`; above
/// zero, each component times the x87 `1/sqrt(sq)`, stored; else (zero, negative, NaN) `v.x = 1.0`.
fn normalise_inline(v: &mut [f32; 2]) {
    let sq = X87::from_f32(v[1]).fmul_m(v[1]).fadd(X87::from_f32(v[0]).fmul_m(v[0]));
    if sq.fcom(X87::zero()).test_parity(0x41) {
        let inv = sq.fsqrt().fdivr_m(1.0);
        v[0] = X87::from_f32(v[0]).fmul(inv).to_f32();
        v[1] = X87::from_f32(v[1]).fmul(inv).to_f32();
    } else {
        v[0] = 1.0;
    }
}

/// `CRunningScript::FlameInAngledAreaCheckCommand(int32 command)` (0x488780, thiscall, `ret 4`): command 0x72E
/// (`IS_FLAME_IN_ANGLED_AREA_3D`, 8 parameters) or any other (the 2D form, 6; no handle, the area starts at
/// `ScriptParams[0]`). [`corners`]; `dx`, `dy` from the x87 differences (stored f32), `len1 = sqrt(dy*dy + dx*dx)`
/// of the 80-bit differences, `len2` from the stored side vector. Then the flame-thrower shots 0..99
/// (`CShotInfo::GetFlameThrowerShotPosn`) until one lies inside: the edge vector and the side vector are normalised
/// in place ([`normalise_inline`], again for every shot tested, so each pass re-normalises the stored result), the
/// projections tested against [0, len1] / [0, len2], then the z range. `UpdateCompareFlag(found)`, then the
/// highlight and debug tails.
#[hook(addr = 0x488780, conv = thiscall, name = "CRunningScript::FlameInAngledAreaCheckCommand", expect = "83 EC 5C 8B 44 24 60",
       unverified = "frontier2-c9", route = "L2", hazards = "x87-transcendental")]
pub fn flame_in_angled_area_check_command(this: GPtr<u8>, command: i32) {
    let is3d = command == 0x72E;
    collect_parameters_call(this, if is3d { 8 } else { 6 });
    let (x1, y1, x2, y2, width, on, zr) = area_params(0, is3d);
    let (fx1, fy1, fx2, fy2) = (f(x1), f(y1), f(x2), f(y2));
    let (x3, y3, x4, y4) = corners(x1, y1, x2, y2, width);
    let dx = X87::from_f32(fx2).fsub_m(fx1);
    let dy = X87::from_f32(fy2).fsub_m(fy1);
    let mut edge = [dx.to_f32(), dy.to_f32()];
    let mut side = [X87::from_f32(x4).fsub_m(fx1).to_f32(), X87::from_f32(y4).fsub_m(fy1).to_f32()];
    let len1 = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt().to_f32();
    let len2 = X87::from_f32(side[1]).fmul_m(side[1]).fadd(X87::from_f32(side[0]).fmul_m(side[0])).fsqrt().to_f32();
    let mut found = false;
    let mut pos = [0f32; 3];
    let mut i: u16 = 0;
    while !found {
        if shot_info_get_flame_thrower_shot_posn(i as u8, GPtr::from_raw(&raw mut pos).cast()) != 0 {
            let rx = X87::from_f32(pos[0]).fsub_m(fx1).to_f32();
            let ry = X87::from_f32(pos[1]).fsub_m(fy1).to_f32();
            normalise_inline(&mut edge);
            let d1 = X87::from_f32(ry).fmul_m(edge[1]).fadd(X87::from_f32(rx).fmul_m(edge[0]));
            if !(d1.fcom(X87::zero()).test_any(1) || d1.fcom(X87::from_f32(len1)).test_parity(0x41)) {
                normalise_inline(&mut side);
                let d2 = X87::from_f32(ry).fmul_m(side[1]).fadd(X87::from_f32(rx).fmul_m(side[0]));
                if !(d2.fcom(X87::zero()).test_any(1) || d2.fcom(X87::from_f32(len2)).test_parity(0x41))
                    && z_ok(zr, pos[2])
                {
                    found = true;
                }
            }
        }
        i = i.wrapping_add(1);
        if i >= 100 {
            break;
        }
    }
    update_compare_flag_call(this, u32::from(found));
    highlight(this, &Quad { x1, y1, x2, y2, x3, y3, x4, y4 }, on, zr);
}

/// `CTheScripts::DrawDebugAngledSquare(float x1, float y1, float x2, float y2, float x3, float y3, float x4, float
/// y4)` (0x486990, cdecl; reached only with the debug flag 0x859CF8 set): for each corner in order a
/// `CWorld::ProcessVerticalLine((x, y, -1000), 1000, colPoint, &entity, buildings, dummies)`. The first call's line
/// length is pushed by the SecuROM fragment 0x401B61 (`[0x15606B8] - [0x6D3045]`, 0x447A0000 = 1000.0 from the exe
/// image), the others as the immediate. The `colPoint.z + 2.0f` the original stores after each probe and the debug
/// lines the FINAL build compiled out are never read; the entity pointer goes into the original's first argument
/// slot, which no caller reads after the call (here a local).
#[hook(addr = 0x486990, conv = cdecl, name = "CTheScripts::DrawDebugAngledSquare", expect = "83 EC 5C 8B 44 24 60",
       unverified = "frontier2-c9", route = "L2", hazards = "none")]
#[allow(clippy::too_many_arguments)]
pub fn the_scripts_draw_debug_angled_square(x1: u32, y1: u32, x2: u32, y2: u32, x3: u32, y3: u32, x4: u32, y4: u32) {
    let mut col_point = [0u8; 0x2C];
    let mut entity = 0u32;
    for (x, y) in [(x1, y1), (x2, y2), (x3, y3), (x4, y4)] {
        let mut p = [x, y, 0xC47A_0000u32];
        world_process_vertical_line(GPtr::from_raw(&raw mut p).cast(), 1000.0, GPtr::from_raw(&raw mut col_point).cast(),
            GPtr::from_raw(&raw mut entity).cast(), 1, 0, 0, 0, 1, 0, GPtr::NULL);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_CHAR_IN_ANGLED_AREA_CHECK_COMMAND,
    HOOK_OBJECT_IN_ANGLED_AREA_CHECK_COMMAND,
    HOOK_FLAME_IN_ANGLED_AREA_CHECK_COMMAND,
    HOOK_THE_SCRIPTS_DRAW_DEBUG_ANGLED_SQUARE,
    HOOK_POSITION_IN_ANGLED_AREA,
];
