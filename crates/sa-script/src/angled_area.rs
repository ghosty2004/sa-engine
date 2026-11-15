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

use crate::bindings::{draw_debug_angled_square, highlight_important_angled_area, is_ped_stopped};
use crate::vm::{collect_parameters_call, update_compare_flag_call};
use sa_core::bindings::{general_get_radian_angle_between_points, vector2d_normalise};
use sa_world::world::common::entity_pos;

/// `ScriptParams[k]` (0xA43C78) as a dword.
fn param(k: usize) -> u32 {
    GPtr::<u32>::from_addr(0xA43C78 + 4 * k).read()
}

fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// The rectangle every angled-area check builds from its parameters: the base edge (x1, y1) → (x2, y2) and the far
/// corners (x3, y3) / (x4, y4) `width` away on the left side, the edge vector (`dx`, `dy`, f32), its squared length
/// (f32), both side lengths (f32) and the second side vector (f32, still to be normalised).
struct Area {
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
    x3: f32,
    y3: f32,
    x4: f32,
    y4: f32,
    dx: f32,
    dy: f32,
    sq: f32,
    len1: f32,
    len2: f32,
    side: [f32; 2],
}

/// The corner computation shared by the angled-area checks (inlined in each): `angle =
/// GetRadianAngleBetweenPoints(x1, y1, x2, y2) + PI/2` brought into [0, 2PI] (`while < 0: += 2PI`, `while > 2PI: -=
/// 2PI`, both against the f32 constants); `x3 = x2 + w sin`, `y3 = y2 + (-w cos)` (the x87 product), `x4 = x1 + w
/// sin`, `y4 = y1 + f32(-w cos)`; `len1 = sqrt(dy*dy + dx*dx)` from the x87 sum, `len2` likewise from the x87
/// differences `x4 - x1`, `y4 - y1`.
fn build_area(x1: u32, y1: u32, x2: u32, y2: u32, width: u32) -> Area {
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
    let dx = X87::from_f32(fx2).fsub_m(fx1).to_f32();
    let dy = X87::from_f32(fy2).fsub_m(fy1).to_f32();
    let b = X87::from_f32(x4).fsub_m(fx1);
    let c = X87::from_f32(y4).fsub_m(fy1);
    let side = [b.to_f32(), c.to_f32()];
    let sq = X87::from_f32(dy).fmul_m(dy).fadd(X87::from_f32(dx).fmul_m(dx));
    let len1 = sq.fsqrt().to_f32();
    let len2 = c.fmul(c).fadd(b.fmul(b)).fsqrt().to_f32();
    Area { x1, y1, x2, y2, x3, y3, x4, y4, dx, dy, sq: sq.to_f32(), len1, len2, side }
}

/// The 2D test of a point against the area (inlined in each check): with `r = p - (x1, y1)` (f32), the projection on
/// the edge's unit vector (the inline `CVector2D::Normalise` of (dx, dy): x from the x87 `1/sqrt(sq) * dx`, y stored;
/// `sq <= 0` or NaN: x = 1.0, y unchanged) lies in [0, len1], and the projection on the normalised side vector
/// (`CVector2D::Normalise`, 0x44E480) in [0, len2].
fn inside_2d(ar: &mut Area, px: f32, py: f32) -> bool {
    let rx = X87::from_f32(px).fsub_m(f(ar.x1)).to_f32();
    let ry = X87::from_f32(py).fsub_m(f(ar.y1)).to_f32();
    let l = X87::from_f32(ar.sq);
    let (nx, ny) = if l.fcom(X87::zero()).test_parity(0x41) {
        let inv = l.fsqrt().fdivr_m(1.0);
        let inv32 = inv.to_f32();
        (inv.fmul_m(ar.dx), X87::from_f32(inv32).fmul_m(ar.dy).to_f32())
    } else {
        (X87::one(), ar.dy)
    };
    let d1 = X87::from_f32(ry).fmul_m(ny).fadd(X87::from_f32(rx).fmul(nx));
    if d1.fcom(X87::zero()).test_any(1) || d1.fcom(X87::from_f32(ar.len1)).test_parity(0x41) {
        return false;
    }
    vector2d_normalise(GPtr::from_raw(&raw mut ar.side).cast());
    let d2 = X87::from_f32(ry).fmul_m(ar.side[1]).fadd(X87::from_f32(rx).fmul_m(ar.side[0]));
    !(d2.fcom(X87::zero()).test_any(1) || d2.fcom(X87::from_f32(ar.len2)).test_parity(0x41))
}

/// The tails of every check: the area highlighted when the highlight parameter is set (`HighlightImportantAngledArea`
/// with id `this + this->m_pBaseIP` and z = `(zmax + zmin) * 0.5`, or -100.0 for the 2D commands), then the debug
/// square of a 2D command when the debug flag byte 0x859CF8 is set.
fn highlight(this: GPtr<u8>, ar: &Area, on: u32, z: Option<(f32, f32)>) {
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
    let (x1, y1) = (param(1), param(2));
    let (x2, y2, width, on, zr) = if is3d {
        let (mut zmin, mut zmax) = (param(3), param(6));
        if !X87::from_f32(f(zmin)).fcom(X87::from_f32(f(zmax))).test_any(0x41) {
            (zmin, zmax) = (zmax, zmin);
        }
        (param(4), param(5), param(7), param(8), Some((f(zmin), f(zmax))))
    } else {
        (param(3), param(4), param(5), param(6), None)
    };
    let mut ar = build_area(x1, y1, x2, y2, width);
    if !not_stopped {
        let in_car = (rd::<u32>(ped, 0x46C) >> 8) & 1;
        let car = rd::<GPtr<u8>>(ped, 0x58C);
        let p = if in_car != 0 && !car.is_null() { entity_pos(car) } else { entity_pos(ped) };
        let (px, py, pz) = (rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8));
        let z_ok = |zr: Option<(f32, f32)>| match zr {
            Some((zmin, zmax)) => {
                !X87::from_f32(pz).fcom(X87::from_f32(zmin)).test_any(1)
                    && !X87::from_f32(pz).fcom(X87::from_f32(zmax)).test_parity(0x41)
            }
            None => true,
        };
        if inside_2d(&mut ar, px, py) && z_ok(zr) {
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
    highlight(this, &ar, on, zr);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_CHAR_IN_ANGLED_AREA_CHECK_COMMAND];
