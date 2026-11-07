//! `CCollision` primitive tests and distances (spheres, boxes, lines, triangles), ported instruction
//! by instruction from the 1.0 US bytes with the [`Fpu`] stack machine (x87, PC = 24). Each port
//! follows the original's operand forms (memory vs register), sum orders, f32 stores and compare
//! branches (`test ah, mask` + `jp`/`jne`), so NaN and rounding behaviour are those of the original;
//! gta-reversed `Collision.cpp` was a reading aid only. Differences to gta-reversed that the bytes
//! decide are listed per function and in `docs/arch/coll-port.md`.
//!
//! Pointer arguments are structs of f32 words read through [`w`] (`CColSphere`: centre +0..+8, radius
//! +0xC; `CBox`/`CColBox`: min +0..+8, max +0xC..+0x14; `CColLine`: start +0..+8, end +0x10..+0x18;
//! `CVector`: x, y, z).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use super::fpu::Fpu;
use super::shared::{cf, k};

sa_core::globals! {
    list GLOBALS;
    /// `CCollision::ms_iProcessLineNumCrossings`
    pub static PROCESS_LINE_NUM_CROSSINGS @ 0x9655D0: u32 = 0;
}

/// f32 word `i` of the struct `p` points at.
pub fn w<T>(p: GPtr<T>, i: isize) -> f32 {
    p.cast::<f32>().wrapping_index(i as i32).read()
}

/// `0.0f` (0x858B50)
fn zero() -> f32 {
    cf(k::F0)
}

/// `CCollision::TestSphereSphere(const CColSphere&, const CColSphere&)` (0x411E70, EAX 0/1):
/// `d = a.c - b.c`, `R = a.r + b.r`; true iff `R*R > (d.z*d.z + d.x*d.x) + d.y*d.y` (strict, NaN
/// false; gr: `<=` test with x, y, z order). EDX is preserved: `SphereCastVsSphere` (0x417F83) pushes
/// the EDX it loaded before the call as an argument afterwards (VER-2 caller scan).
#[hook(addr = 0x411E70, conv = cdecl, name = "CCollision::TestSphereSphere", expect = "8B 44 24 04 8B 4C 24 08", preserve = "edx")]
pub fn test_sphere_sphere(a: GPtr<f32>, b: GPtr<f32>) -> u32 {
    let mut f = Fpu::new();
    f.fld(w(a, 0));
    f.fsub_m(w(b, 0));
    f.fld(w(a, 1));
    f.fsub_m(w(b, 1));
    f.fld(w(a, 2));
    f.fsub_m(w(b, 2));
    f.fld(w(a, 3));
    f.fadd_m(w(b, 3));
    f.fld_st(1);
    f.fmul_st(2);
    f.fld_st(4);
    f.fmul_st(5);
    f.faddp(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fld_st(1);
    f.fmul_st(2);
    f.fcompp();
    let r = f.jne(0x41);
    (!r) as u32
}

/// `CCollision::TestSphereBox(const CSphere&, const CBox&)` (0x4120C0, AL): per axis rejects
/// `c + r < min` and `c - r > max` (NaN passes both).
#[hook(addr = 0x4120C0, conv = cdecl, name = "CCollision::TestSphereBox", expect = "8B 4C 24 04 D9 01")]
pub fn test_sphere_box(s: GPtr<f32>, bx: GPtr<f32>) -> u8 {
    let mut f = Fpu::new();
    for a in 0..3 {
        f.fld(w(s, a));
        f.fadd_m(w(s, 3));
        f.fcomp_m(w(bx, a));
        if !f.jp(5) {
            return 0;
        }
        f.fld(w(s, a));
        f.fsub_m(w(s, 3));
        f.fcomp_m(w(bx, a + 3));
        if !f.jne(0x41) {
            return 0;
        }
    }
    1
}

/// `CCollision::PointInTriangle(const CVector& p, const CVector* tri)` (0x412700, stdcall `ret 8`,
/// AL): barycentric test with f32-stored dot products (`u >= 0`, `v >= 0`, `u + v <= d`; NaN passes).
#[hook(addr = 0x412700, conv = stdcall, name = "CCollision::PointInTriangle", expect = "83 EC 20 8B 44 24 28")]
pub fn point_in_triangle(p: GPtr<f32>, tri: GPtr<f32>) -> u8 {
    let t = |i| w(tri, i);
    let mut f = Fpu::new();
    f.fld(t(3));
    f.fsub_m(t(0));
    f.fld(t(4));
    f.fsub_m(t(1));
    f.fld(t(5));
    f.fsub_m(t(2));
    f.fld(t(6));
    f.fsub_m(t(0));
    let l8 = f.fstp();
    f.fld(t(7));
    f.fsub_m(t(1));
    let lc = f.fstp();
    f.fld(t(8));
    f.fsub_m(t(2));
    let l10 = f.fstp();
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    let l4 = f.fstp();
    f.fld(l10);
    f.fmul_m(l10);
    f.fld(lc);
    f.fmul_m(lc);
    f.faddp(1);
    f.fld(l8);
    f.fmul_m(l8);
    f.faddp(1);
    let l0 = f.fstp();
    f.fld(l10);
    f.fmul_st(1);
    f.fld(lc);
    f.fmul_st(3);
    f.faddp(1);
    f.fld(l8);
    f.fmul_st(4);
    f.faddp(1);
    let a28 = f.fstp();
    f.fld(w(p, 0));
    f.fsub_m(t(0));
    let l14 = f.fstp();
    f.fld(w(p, 1));
    f.fsub_m(t(1));
    let l18 = f.fstp();
    f.fld(w(p, 2));
    f.fsub_m(t(2));
    let l1c = f.fst();
    f.fmul_st(1);
    f.fld(l14);
    f.fmul_st(4);
    f.faddp(1);
    f.fld(l18);
    f.fmul_st(3);
    f.faddp(1);
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
    f.fld(l1c);
    f.fmul_m(l10);
    f.fld(l18);
    f.fmul_m(lc);
    f.faddp(1);
    f.fld(l14);
    f.fmul_m(l8);
    f.faddp(1);
    f.fld_st(1);
    f.fmul_m(l0);
    f.fld_st(1);
    f.fmul_m(a28);
    f.fsubp(1);
    let a24 = f.fst();
    f.fcomp_m(zero());
    if !f.jp(5) {
        return 0;
    }
    f.fmul_m(l4);
    f.fxch(1);
    f.fmul_m(a28);
    f.fsubp(1);
    f.fcom_m(zero());
    if !f.jp(5) {
        return 0;
    }
    f.fld(a24);
    f.fadd_st(1);
    f.fld(l0);
    f.fmul_m(l4);
    f.fld(a28);
    f.fmul_m(a28);
    f.fsubp(1);
    f.fcompp();
    if !f.jp(5) {
        return 0;
    }
    1
}

/// `CCollision::DistToLineSqr(const CVector& a, const CVector& b, const CVector& p)` (0x412850, ST0):
/// squared distance from `p` to segment `ab`; the projection `t` is stored as f32; the middle case is
/// `|p-a|^2 - t*t / |ab|^2` clamped at 0 (`<= 0` or NaN keeps the subtraction... see the branches).
#[hook(addr = 0x412850, conv = cdecl, name = "CCollision::DistToLineSqr", expect = "83 EC 0C 8B 44 24 10")]
pub fn dist_to_line_sqr(a: GPtr<f32>, b: GPtr<f32>, p: GPtr<f32>) -> X87 {
    let mut f = Fpu::new();
    f.fld(w(b, 0));
    f.fsub_m(w(a, 0));
    f.fld(w(b, 1));
    f.fsub_m(w(a, 1));
    f.fld(w(b, 2));
    f.fsub_m(w(a, 2));
    f.fld(w(p, 0));
    f.fsub_m(w(a, 0));
    let l0 = f.fstp();
    f.fld(w(p, 1));
    f.fsub_m(w(a, 1));
    let l4 = f.fstp();
    f.fld(w(p, 2));
    f.fsub_m(w(a, 2));
    let l8 = f.fstp();
    f.fld(l0);
    f.fmul_st(3);
    f.fld(l8);
    f.fmul_st(2);
    f.faddp(1);
    f.fld(l4);
    f.fmul_st(3);
    f.faddp(1);
    let t = f.fst();
    f.fcomp_m(zero());
    if !f.jp(0x41) {
        f.fstp_st(0);
        f.fstp_st(0);
        f.fstp_st(0);
        f.fld(l8);
        f.fmul_m(l8);
        f.fld(l4);
        f.fmul_m(l4);
        f.faddp(1);
        f.fld(l0);
        f.fmul_m(l0);
        f.faddp(1);
        return f.pop();
    }
    f.fld_st(2);
    f.fmulp(3);
    f.fld_st(0);
    f.fmul_st(1);
    f.faddp(3);
    f.fld_st(1);
    f.fmul_st(2);
    f.faddp(3);
    f.fstp_st(0);
    f.fstp_st(0);
    f.fld(t);
    f.fcomp_st(1);
    if f.jne(1) {
        f.fld(l8);
        f.fmul_m(l8);
        f.fld(l4);
        f.fmul_m(l4);
        f.faddp(1);
        f.fld(l0);
        f.fmul_m(l0);
        f.faddp(1);
        f.fld(t);
        f.fmul_m(t);
        f.fdiv_st(2);
        f.fsubp(1);
        f.fstp_st(1);
        f.fcom_m(zero());
        if !f.jp(0x41) {
            f.fstp_st(0);
            f.fld(zero());
        }
        return f.pop();
    }
    f.fstp_st(0);
    f.fld(w(p, 0));
    f.fsub_m(w(b, 0));
    f.fld(w(p, 1));
    f.fsub_m(w(b, 1));
    f.fld(w(p, 2));
    f.fsub_m(w(b, 2));
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
    f.pop()
}

/// `CCollision::DistToMathematicalLine(const CVector* a, const CVector* b, const CVector* p)`
/// (0x412970, ST0): `sqrt(|p-a|^2 - t*t/|b-a|^2)` with the radicand stored as f32 first; 0 when it is
/// `<= 0` (NaN takes the root).
#[hook(addr = 0x412970, conv = cdecl, name = "CCollision::DistToMathematicalLine", expect = "83 EC 0C 8B 4C 24 14")]
pub fn dist_to_mathematical_line(a: GPtr<f32>, b: GPtr<f32>, p: GPtr<f32>) -> X87 {
    let mut f = Fpu::new();
    f.fld(w(b, 0));
    f.fsub_m(w(a, 0));
    f.fld(w(b, 1));
    f.fsub_m(w(a, 1));
    f.fld(w(b, 2));
    f.fsub_m(w(a, 2));
    f.fld(w(p, 0));
    f.fsub_m(w(a, 0));
    f.fld(w(p, 1));
    f.fsub_m(w(a, 1));
    let l4 = f.fstp();
    f.fld(w(p, 2));
    f.fsub_m(w(a, 2));
    let l8 = f.fstp();
    f.fld_st(0);
    f.fmul_st(4);
    f.fld(l8);
    f.fmul_st(3);
    f.faddp(1);
    f.fld(l4);
    f.fmul_st(4);
    f.faddp(1);
    let t = f.fstp();
    f.fld(l8);
    f.fmul_m(l8);
    f.fld(l4);
    f.fmul_m(l4);
    f.faddp(1);
    f.fld_st(1);
    f.fmul_st(2);
    f.faddp(1);
    f.fld(t);
    f.fmul_m(t);
    f.fld_st(5);
    f.fmul_st(6);
    f.fld_st(4);
    f.fmul_st(5);
    f.faddp(1);
    f.fld_st(5);
    f.fmul_st(6);
    f.faddp(1);
    f.fdivp(1);
    f.fsubp(1);
    let r = f.fstp();
    for _ in 0..4 {
        f.fstp_st(0);
    }
    f.fld(r);
    f.fcomp_m(zero());
    if !f.jp(0x41) {
        f.fld(zero());
        return f.pop();
    }
    f.fld(r);
    f.fsqrt();
    f.pop()
}

/// `CCollision::DistToMathematicalLine2D(x, y, dx, dy, px, py)` (0x412A30, ST0): `a = px - x`,
/// `b = py - y`, `t = a*dx + b*dy`, `D = (b*b + a*a) - t*t` (registers, not stored); `sqrt(D)` when
/// `D > 0` or NaN, else 0.
#[hook(addr = 0x412A30, conv = cdecl, name = "CCollision::DistToMathematicalLine2D", expect = "D9 44 24 14 D8 64 24 04")]
pub fn dist_to_mathematical_line_2d(x: f32, y: f32, dx: f32, dy: f32, px: f32, py: f32) -> X87 {
    let mut f = Fpu::new();
    f.fld(px);
    f.fsub_m(x);
    f.fld(py);
    f.fsub_m(y);
    f.fld_st(1);
    f.fmul_m(dx);
    f.fld_st(1);
    f.fmul_m(dy);
    f.faddp(1);
    f.fld_st(1);
    f.fmul_st(2);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fld_st(1);
    f.fmul_st(2);
    f.fsubp(1);
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
    f.fcom_m(zero());
    if f.jp(0x41) {
        f.fsqrt();
    } else {
        f.fstp_st(0);
        f.fld(zero());
    }
    f.pop()
}

/// `CCollision::DistAlongLine2D(x, y, dx, dy, px, py)` (0x412A80, ST0): `(px - x)*dx + (py - y)*dy`.
#[hook(addr = 0x412A80, conv = cdecl, name = "CCollision::DistAlongLine2D", expect = "D9 44 24 14 D8 64 24 04")]
pub fn dist_along_line_2d(x: f32, y: f32, dx: f32, dy: f32, px: f32, py: f32) -> X87 {
    let mut f = Fpu::new();
    f.fld(px);
    f.fsub_m(x);
    f.fmul_m(dx);
    f.fld(py);
    f.fsub_m(y);
    f.fmul_m(dy);
    f.faddp(1);
    f.pop()
}

/// `CCollision::TestVerticalLineBox(const CColLine&, const CBox&)` (0x413080, AL): the start's x/y
/// strictly inside the box (NaN fails), the z range `[min(s.z, e.z), max]` overlapping `[min.z,
/// max.z]` (`lo > max.z` or `hi < min.z` rejects; the end's x/y are not used).
#[hook(addr = 0x413080, conv = cdecl, name = "CCollision::TestVerticalLineBox", expect = "8B 4C 24 04 D9 01")]
pub fn test_vertical_line_box(line: GPtr<f32>, bx: GPtr<f32>) -> u8 {
    let mut f = Fpu::new();
    f.fld(w(line, 0));
    f.fcomp_m(w(bx, 0));
    if f.jne(0x41) {
        return 0;
    }
    f.fld(w(line, 1));
    f.fcomp_m(w(bx, 1));
    if f.jne(0x41) {
        return 0;
    }
    f.fld(w(line, 0));
    f.fcomp_m(w(bx, 3));
    if f.jp(5) {
        return 0;
    }
    f.fld(w(line, 1));
    f.fcomp_m(w(bx, 4));
    if f.jp(5) {
        return 0;
    }
    f.fld(w(line, 2));
    f.fcomp_m(w(line, 6));
    let hi = if f.jp(5) {
        f.fld(w(line, 6));
        w(line, 2)
    } else {
        f.fld(w(line, 2));
        w(line, 6)
    };
    f.fcomp_m(w(bx, 5));
    if !f.jne(0x41) {
        return 0;
    }
    f.fld(hi);
    f.fcomp_m(w(bx, 2));
    if !f.jp(5) {
        return 0;
    }
    1
}

/// `CCollision::Test2DLineAgainst2DLine(x1, y1, dx1, dy1, x2, y2, dx2, dy2)` (0x4138D0, AL): the two
/// cross-product sign tests; the first rejects a product `> 0`, the second accepts `<= 0` (NaN:
/// first passes, second accepts).
#[hook(addr = 0x4138D0, conv = cdecl, name = "CCollision::Test2DLineAgainst2DLine", expect = "D9 44 24 14 D8 64 24 04")]
#[allow(clippy::too_many_arguments)]
pub fn test_2d_line_against_2d_line(
    x1: f32,
    y1: f32,
    dx1: f32,
    dy1: f32,
    x2: f32,
    y2: f32,
    dx2: f32,
    dy2: f32,
) -> u8 {
    let mut f = Fpu::new();
    f.fld(x2);
    f.fsub_m(x1);
    f.fld(y2);
    f.fsub_m(y1);
    f.fld_st(1);
    f.fadd_m(dx2);
    f.fmul_m(dy1);
    f.fld_st(1);
    f.fadd_m(dy2);
    f.fmul_m(dx1);
    f.fsubp(1);
    f.fxch(2);
    f.fmul_m(dy1);
    f.fxch(1);
    f.fmul_m(dx1);
    f.fsubp(1);
    f.fmulp(1);
    f.fcomp_m(zero());
    if !f.jne(0x41) {
        return 0;
    }
    f.fld(x1);
    f.fsub_m(x2);
    f.fld(y1);
    f.fsub_m(y2);
    f.fld_st(1);
    f.fadd_m(dx1);
    f.fmul_m(dy2);
    f.fld_st(1);
    f.fadd_m(dy1);
    f.fmul_m(dx2);
    f.fsubp(1);
    f.fxch(2);
    f.fmul_m(dy2);
    f.fxch(1);
    f.fmul_m(dx2);
    f.fsubp(1);
    f.fmulp(1);
    f.fcomp_m(zero());
    f.jne(0x41) as u8
}

/// `CCollision::GetBoundingBoxFromTwoSpheres(const CColSphere& a, const CColSphere& b)` (0x415230,
/// result through the hidden pointer `out`; EAX is not set). Per axis `min = lo - a.r`, `max = a.r +
/// hi` with `lo`/`hi` the smaller/larger centre coordinate (`a >= b` or NaN takes `b` as `lo`); both
/// spheres use **a's radius** (gr: each its own). The box's surface word is not written.
#[hook(addr = 0x415230, conv = cdecl, name = "CCollision::GetBoundingBoxFromTwoSpheres", expect = "8B 54 24 08 D9 42 0C")]
pub fn get_bounding_box_from_two_spheres(out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>) {
    let mut f = Fpu::new();
    f.fld(w(a, 3));
    for i in 0..3 {
        f.fld(w(a, i));
        f.fcomp_m(w(b, i));
        let (lo, hi) = if f.jp(5) {
            (w(b, i), w(a, i))
        } else {
            (w(a, i), w(b, i))
        };
        f.fld(lo);
        f.fsub_st(1);
        out.wrapping_index(i as i32).write(f.fstp());
        if i < 2 {
            f.fld_st(0);
        }
        f.fadd_m(hi);
        out.wrapping_index(i as i32 + 3).write(f.fstp());
    }
}

/// `CCollision::GetPrincipleAxis(const CVector&)` (0x4156D0, EAX): 0 when `|x| > |y|` and `|x| > |z|`,
/// else 2 when `|y| > |z|`, else 4 (strict; NaN goes to the later choice). `|x|` stays a register,
/// `|y|`/`|z|` are stored as f32.
#[hook(addr = 0x4156D0, conv = cdecl, name = "CCollision::GetPrincipleAxis", expect = "83 EC 0C 8B 44 24 10")]
pub fn get_principle_axis(n: GPtr<f32>) -> u32 {
    let mut f = Fpu::new();
    f.fld(w(n, 0));
    f.fabs();
    f.fld(w(n, 1));
    f.fabs();
    let l4 = f.fstp();
    f.fld(w(n, 2));
    f.fabs();
    let l8 = f.fstp();
    f.fcom_m(l4);
    if !f.jne(0x41) {
        f.fcomp_m(l8);
        if !f.jne(0x41) {
            return 0;
        }
    } else {
        f.fstp_st(0);
    }
    f.fld(l4);
    f.fcomp_m(l8);
    if f.jne(0x41) { 4 } else { 2 }
}

/// `CCollision::IsThisVehicleSittingOnMe(CVehicle* veh, CVehicle* onMe)` (stub 0x4152C0 → 0x1566DA0,
/// AL): false for a null pointer; `onMe` of type 0 (automobile) compares its 4 wheel collision
/// entities at +0x8E8, type 9 (bike) its 4 at +0x7C0, others false.
#[hook(addr = 0x4152C0, conv = cdecl, name = "CCollision::IsThisVehicleSittingOnMe", expect = "E9 DB 1A 15 01")]
pub fn is_this_vehicle_sitting_on_me(veh: GPtr<u8>, on_me: GPtr<u8>) -> u8 {
    if veh.is_null() || on_me.is_null() {
        return 0;
    }
    let kind = on_me.byte_add(0x590).cast::<u32>().read();
    let base = match kind {
        0 => 0x8E8,
        9 => 0x7C0,
        _ => return 0,
    };
    let ents = on_me.byte_add(base).cast::<GPtr<u8>>();
    (0..4).any(|i| ents.wrapping_index(i).read() == veh) as u8
}

/// `TestLineBox_DW` outcode word: the sign bits of six f32 differences packed as the original does
/// (`and`/`shr`/`or` sequence, then `>> 26`).
fn outcode(d: [f32; 6]) -> u32 {
    let b = d.map(f32::to_bits);
    let mut x = b[0] & 0xBFFF_FFFF;
    x |= b[3] >> 1;
    x >>= 1;
    x |= b[4] & 0x9FFF_FFFF;
    x >>= 1;
    x |= b[1] & 0x8FFF_FFFF;
    x >>= 1;
    x |= b[5] & 0x87FF_FFFF;
    x >>= 1;
    x |= b[2] & 0x83FF_FFFF;
    x >> 26
}

/// One slab test of `TestLineBox_DW`: `t = (neg ? -(num * inv) : num * inv)` (register), the other two
/// coordinates `d.c * t + s.c` stored as f32 and tested strictly inside the box's other two ranges.
fn slab_hit(
    s: GPtr<f32>,
    bx: GPtr<f32>,
    d: [f32; 3],
    num: f32,
    inv: f32,
    neg: bool,
    axes: [usize; 2],
) -> bool {
    let mut f = Fpu::new();
    f.fld(num);
    f.fmul_m(inv);
    if neg {
        f.fchs();
    }
    let mut c = [0f32; 2];
    for (k, a) in axes.iter().enumerate() {
        f.fld(d[*a]);
        f.fmul_st(1);
        f.fadd_m(w(s, *a as isize));
        c[k] = f.fstp();
    }
    f.fstp_st(0);
    for (k, a) in axes.iter().enumerate() {
        f.fld(c[k]);
        f.fcomp_m(w(bx, *a as isize));
        if f.jne(0x41) {
            return false;
        }
        f.fld(c[k]);
        f.fcomp_m(w(bx, *a as isize + 3));
        if f.jp(5) {
            return false;
        }
    }
    true
}

/// `CCollision::TestLineBox_DW(const CColLine&, const CBox&)` (0x412C70, AL): Cohen-Sutherland style
/// outcodes from the sign bits of the f32 differences; trivially out when both codes share a bit, in
/// when their 32-bit product is 0; otherwise up to six slab tests (x, y, z pairs; `1/d` stored as f32,
/// the hit coordinates stored as f32 and compared strictly).
#[hook(addr = 0x412C70, conv = cdecl, name = "CCollision::TestLineBox_DW", expect = "83 EC 44 8B 4C 24 48")]
pub fn test_line_box_dw(line: GPtr<f32>, bx: GPtr<f32>) -> u8 {
    let sub = |a: f32, b: f32| X87::from_f32(a).fsub_m(b).to_f32();
    let (s, e) = (
        [w(line, 0), w(line, 1), w(line, 2)],
        [w(line, 4), w(line, 5), w(line, 6)],
    );
    let (mn, mx) = ([w(bx, 0), w(bx, 1), w(bx, 2)], [w(bx, 3), w(bx, 4), w(bx, 5)]);
    let a = [
        sub(mx[0], s[0]),
        sub(mx[1], s[1]),
        sub(mx[2], s[2]),
        sub(s[0], mn[0]),
        sub(s[1], mn[1]),
        sub(s[2], mn[2]),
    ];
    let b = [
        sub(mx[0], e[0]),
        sub(mx[1], e[1]),
        sub(mx[2], e[2]),
        sub(e[0], mn[0]),
        sub(e[1], mn[1]),
        sub(e[2], mn[2]),
    ];
    let (c0, c1) = (outcode(a), outcode(b));
    if c0 & c1 != 0 {
        return 0;
    }
    if c1.wrapping_mul(c0) == 0 {
        return 1;
    }
    let dl = (c1 | c0) as u8;
    let d = [sub(e[0], s[0]), sub(e[1], s[1]), sub(e[2], s[2])];
    let s_ptr = line;
    let inv = |c: f32| X87::one().fdiv_m(c).to_f32();
    if dl & 3 != 0 {
        let i = inv(d[0]);
        if dl & 1 != 0 && slab_hit(s_ptr, bx, d, a[3], i, true, [1, 2]) {
            return 1;
        }
        if dl & 2 != 0 && slab_hit(s_ptr, bx, d, a[0], i, false, [1, 2]) {
            return 1;
        }
    }
    if dl & 0xC != 0 {
        let i = inv(d[1]);
        if dl & 4 != 0 && slab_hit(s_ptr, bx, d, a[4], i, true, [0, 2]) {
            return 1;
        }
        if dl & 8 != 0 && slab_hit(s_ptr, bx, d, a[1], i, false, [0, 2]) {
            return 1;
        }
    }
    if dl & 0x30 == 0 {
        return 0;
    }
    let i = inv(d[2]);
    if dl & 0x10 != 0 && slab_hit(s_ptr, bx, d, a[5], i, true, [0, 1]) {
        return 1;
    }
    (dl & 0x20 != 0 && slab_hit(s_ptr, bx, d, a[2], i, false, [0, 1])) as u8
}

/// `CCollision::TestLineBox` (0x413070): `jmp TestLineBox_DW`.
#[hook(addr = 0x413070, conv = cdecl, name = "CCollision::TestLineBox", expect = "E9 FB FB FF FF 90")]
pub fn test_line_box(line: GPtr<f32>, bx: GPtr<f32>) -> u8 {
    test_line_box_dw_call(line, bx)
}

sa_core::orig! {
    /// `CCollision::TestLineBox_DW` (0x412C70)
    pub fn test_line_box_dw_call(line: GPtr<f32>, bx: GPtr<f32>) -> u8 = cdecl 0x412C70 => test_line_box_dw;
}

/// `CCollision::TestLineSphere(const CColLine&, const CColSphere&)` (0x417470, AL). `len = |e - s|`
/// (stored). Lines shorter than 1e-6 (`len < K`, NaN not) test the start point: true unless `r*r <
/// |c - s|^2` (NaN false). Otherwise the quadratic with `B = -2 * ((m.z*d.z + m.x*d.x) + m.y*d.y)`
/// (stored), `A = len*len` (stored), `C = ((|c|^2 + |s|^2) - 2*(s.c)) - r*r` and `disc = B*B - C*A*4`
/// (stored): false when `disc < 0`; `t = (-B - sqrt(disc)) / (A + A)`: true when `0 <= t <= 1` (NaN
/// passes both).
#[hook(addr = 0x417470, conv = cdecl, name = "CCollision::TestLineSphere", expect = "83 EC 1C 8B 4C 24 20")]
pub fn test_line_sphere(line: GPtr<f32>, sp: GPtr<f32>) -> u8 {
    let l = |i| w(line, i);
    let c = |i| w(sp, i);
    let mut f = Fpu::new();
    f.fld(l(4));
    f.fsub_m(l(0));
    let l10 = f.fstp();
    f.fld(l(5));
    f.fsub_m(l(1));
    let l14 = f.fstp();
    f.fld(l(6));
    f.fsub_m(l(2));
    let l18 = f.fst();
    f.fmul_m(l18);
    f.fld(l10);
    f.fmul_m(l10);
    f.faddp(1);
    f.fld(l14);
    f.fmul_m(l14);
    f.faddp(1);
    f.fsqrt();
    let len = f.fstp();
    f.fld(c(0));
    f.fsub_m(l(0));
    f.fld(c(1));
    f.fsub_m(l(1));
    f.fld(c(2));
    f.fsub_m(l(2));
    f.fld(len);
    f.fcomp_m(f32::from_bits(0x3586_37BD));
    if !f.jp(5) {
        f.fld(c(3));
        f.fld_st(1);
        f.fmul_st(2);
        f.fld_st(3);
        f.fmul_st(4);
        f.faddp(1);
        f.fld_st(4);
        f.fmul_st(5);
        f.faddp(1);
        f.fld_st(1);
        f.fmul_st(2);
        f.fcompp();
        return (!f.jne(1)) as u8;
    }
    f.fmul_m(l18);
    f.fxch(2);
    f.fmul_m(l10);
    f.faddp(2);
    f.fmul_m(l14);
    f.faddp(1);
    f.fmul_m(f32::from_bits(0xC000_0000));
    let bb = f.fstp();
    let (s0, s1, s2) = (l(2), l(1), l(0));
    f.fld(c(3));
    f.fld(c(2));
    f.fld(c(1));
    f.fld(c(0));
    f.fld(len);
    f.fmul_m(len);
    let lc = f.fstp();
    f.fld(bb);
    f.fmul_m(bb);
    f.fld_st(3);
    f.fmul_st(4);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fld(s0);
    f.fmul_m(s0);
    f.fld(s1);
    f.fmul_m(s1);
    f.faddp(1);
    f.fld(s2);
    f.fmul_m(s2);
    f.faddp(1);
    f.faddp(1);
    f.fld(l(2));
    f.fmul_m(c(2));
    f.fld(l(0));
    f.fmul_m(c(0));
    f.faddp(1);
    f.fld(l(1));
    f.fmul_m(c(1));
    f.faddp(1);
    f.fadd_st(0);
    f.fsubp(1);
    f.fld_st(5);
    f.fmul_st(6);
    f.fsubp(1);
    f.fmul_m(lc);
    f.fmul_m(f32::from_bits(0x4080_0000));
    f.fsubp(1);
    let disc = f.fstp();
    f.fld(disc);
    f.fcomp_m(zero());
    if !f.jp(5) {
        return 0;
    }
    let mut g = Fpu::new();
    g.fld(bb);
    g.fchs();
    g.fld(disc);
    g.fsqrt();
    g.fsubp(1);
    g.fld(lc);
    g.fadd_st(0);
    g.fdivp(1);
    g.fcom_m(zero());
    if !g.jp(5) {
        return 0;
    }
    g.fcomp_m(cf(k::F1));
    g.jne(0x41) as u8
}

/// `CCollision::DistToLine(const CVector& a, const CVector& b, const CVector& p)` (0x417610, ST0): as
/// `DistToLineSqr` with square roots (`sqrt(|p-a|^2)` before `a`, `sqrt(|p-b|^2)` past `b`, `sqrt(D)`
/// of the f32-free middle case, clamped to 0 when `D <= 0`).
#[hook(addr = 0x417610, conv = cdecl, name = "CCollision::DistToLine", expect = "83 EC 0C 8B 44 24 10")]
pub fn dist_to_line(a: GPtr<f32>, b: GPtr<f32>, p: GPtr<f32>) -> X87 {
    let mut f = Fpu::new();
    f.fld(w(b, 0));
    f.fsub_m(w(a, 0));
    f.fld(w(b, 1));
    f.fsub_m(w(a, 1));
    f.fld(w(b, 2));
    f.fsub_m(w(a, 2));
    f.fld(w(p, 0));
    f.fsub_m(w(a, 0));
    let l0 = f.fstp();
    f.fld(w(p, 1));
    f.fsub_m(w(a, 1));
    let l4 = f.fstp();
    f.fld(w(p, 2));
    f.fsub_m(w(a, 2));
    let l8 = f.fst();
    f.fmul_st(1);
    f.fld(l0);
    f.fmul_st(4);
    f.faddp(1);
    f.fld(l4);
    f.fmul_st(3);
    f.faddp(1);
    let t = f.fst();
    f.fcomp_m(zero());
    if !f.jp(0x41) {
        f.fstp_st(0);
        f.fstp_st(0);
        f.fstp_st(0);
        f.fld(l8);
        f.fmul_m(l8);
        f.fld(l4);
        f.fmul_m(l4);
        f.faddp(1);
        f.fld(l0);
        f.fmul_m(l0);
        f.faddp(1);
        f.fsqrt();
        return f.pop();
    }
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
    f.fld(t);
    f.fcomp_st(1);
    if f.jne(1) {
        f.fld(l8);
        f.fmul_m(l8);
        f.fld(l4);
        f.fmul_m(l4);
        f.faddp(1);
        f.fld(l0);
        f.fmul_m(l0);
        f.faddp(1);
        f.fld(t);
        f.fmul_m(t);
        f.fdiv_st(2);
        f.fsubp(1);
        f.fstp_st(1);
        f.fcom_m(zero());
        if f.jp(0x41) {
            f.fsqrt();
        } else {
            f.fstp_st(0);
            f.fld(zero());
        }
        return f.pop();
    }
    f.fstp_st(0);
    f.fld(w(p, 0));
    f.fsub_m(w(b, 0));
    f.fld(w(p, 1));
    f.fsub_m(w(b, 1));
    f.fld(w(p, 2));
    f.fsub_m(w(b, 2));
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fsqrt();
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
    f.pop()
}

/// `CCollision::ClosestPointOnLine(const CVector& a, const CVector& b, const CVector& p, CVector& out)`
/// (0x417FD0). As the bytes compute it: `len = |p - b|`, `q = (p - b) * (1/len)` (all stored),
/// `t = (q.z*d.z + q.y*d.y) + q.x*d.x` with `d = a - b` (stored); `out = b` when `t < 0`, `out = p` when
/// `t > len`, else `out = b + q*t` (each stored). The copies are raw dword moves.
#[hook(addr = 0x417FD0, conv = cdecl, name = "CCollision::ClosestPointOnLine", expect = "83 EC 18 8B 44 24 1C")]
pub fn closest_point_on_line(a: GPtr<f32>, b: GPtr<f32>, p: GPtr<f32>, out: GPtr<f32>) {
    let mut f = Fpu::new();
    f.fld(w(a, 0));
    f.fsub_m(w(b, 0));
    f.fld(w(a, 1));
    f.fsub_m(w(b, 1));
    f.fld(w(a, 2));
    f.fsub_m(w(b, 2));
    f.fld(w(p, 0));
    f.fsub_m(w(b, 0));
    f.fld(w(p, 1));
    f.fsub_m(w(b, 1));
    f.fld(w(p, 2));
    f.fsub_m(w(b, 2));
    let mut l8 = f.fst();
    f.fmul_m(l8);
    f.fld_st(1);
    f.fmul_st(2);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fsqrt();
    let len = f.fstp();
    f.fld(cf(k::F1));
    f.fdiv_m(len);
    let inv = f.fstp();
    f.fxch(1);
    f.fmul_m(inv);
    let l0 = f.fstp();
    f.fmul_m(inv);
    let l4 = f.fstp();
    f.fld(l8);
    f.fmul_m(inv);
    l8 = f.fst();
    f.fmul_st(1);
    f.fld(l4);
    f.fmul_st(3);
    f.faddp(1);
    f.fld(l0);
    f.fmul_st(4);
    f.faddp(1);
    let t = f.fstp();
    let copy = |src: GPtr<f32>| {
        let (s, d) = (src.cast::<u32>(), out.cast::<u32>());
        for i in 0..3 {
            d.wrapping_index(i).write(s.wrapping_index(i).read());
        }
    };
    let mut g = Fpu::new();
    g.fld(t);
    g.fcomp_m(zero());
    if !g.jp(5) {
        copy(b);
        return;
    }
    g.fld(t);
    g.fcomp_m(len);
    if !g.jne(0x41) {
        copy(p);
        return;
    }
    g.fld(l0);
    g.fmul_m(t);
    g.fld(l4);
    g.fmul_m(t);
    g.fld(l8);
    g.fmul_m(t);
    let z = g.fstp();
    g.fxch(1);
    g.fadd_m(w(b, 0));
    let x = g.fstp();
    g.fadd_m(w(b, 1));
    let y = g.fstp();
    let mut h = Fpu::new();
    h.fld(z);
    h.fadd_m(w(b, 2));
    let zz = h.fstp();
    out.write(x);
    out.wrapping_index(1).write(y);
    out.wrapping_index(2).write(zz);
}

/// Writes f32 word `i` of `p`.
fn ww<T>(p: GPtr<T>, i: isize, v: f32) {
    p.cast::<f32>().wrapping_index(i as i32).write(v)
}

/// Copies byte `n` bytes from `src + so` to `dst + d0` (the original's word/byte moves).
fn copy_bytes<T, U>(dst: GPtr<T>, d0: isize, src: GPtr<U>, so: isize, n: isize) {
    for i in 0..n {
        let b = src.cast::<u8>().byte_add(so + i).read();
        dst.cast::<u8>().byte_add(d0 + i).write(b);
    }
}

/// `CCollision::CalculateColPointInsideBox(const CBox&, const CVector& p, CColPoint& cp)` (0x411EC0):
/// `cp.point = p` (dword copies); per axis `d = p - centre` (centre `(max + min) * 0.5`, x stored,
/// y/z registers) and `dist = (d <= 0 or NaN) ? p - min : max - p` (x, z registers, y stored); x
/// wins when `distX < distY` and `distX < distZ(stored)`, else y when `distY < distX` and `distY <
/// distZ`, else z; `cp.normal = ±1` on the winner (`+1` when `d > 0`), `cp.depth = dist`. Nothing else
/// of `cp` is written (gr: `colPoint = {}` first).
#[hook(addr = 0x411EC0, conv = cdecl, name = "CCollision::CalculateColPointInsideBox", expect = "83 EC 18 8B 4C 24 1C")]
pub fn calculate_col_point_inside_box(bx: GPtr<f32>, p: GPtr<f32>, cp: GPtr<f32>) {
    let mut f = Fpu::new();
    f.fld(w(bx, 3));
    f.fadd_m(w(bx, 0));
    f.fld(w(bx, 4));
    f.fadd_m(w(bx, 1));
    f.fld(w(bx, 5));
    f.fadd_m(w(bx, 2));
    let sz = f.fstp();
    f.fxch(1);
    f.fmul_m(cf(k::F0_5));
    let cx = f.fstp();
    f.fmul_m(cf(k::F0_5));
    f.fld(sz);
    f.fmul_m(cf(k::F0_5));
    f.fld(w(p, 0));
    f.fsub_m(cx);
    let dx = f.fstp();
    f.fld(w(p, 1));
    f.fsub_st(2);
    let dy = f.fstp();
    f.fld(w(p, 2));
    f.fsub_st(1);
    let dz = f.fstp();
    f.fstp_st(0);
    f.fstp_st(0);
    let dist = |f: &mut Fpu, d: f32, a: isize| {
        f.fld(d);
        f.fcomp_m(zero());
        if f.jne(0x41) {
            f.fld(w(p, a));
            f.fsub_m(w(bx, a));
        } else {
            f.fld(w(bx, a + 3));
            f.fsub_m(w(p, a));
        }
    };
    dist(&mut f, dx, 0);
    dist(&mut f, dy, 1);
    let dist_y = f.fstp();
    dist(&mut f, dz, 2);
    let dist_z = f.fstp();
    let (src, dst) = (p.cast::<u32>(), cp.cast::<u32>());
    let px = src.read();
    f.fcom_m(dist_y);
    dst.write(px);
    dst.wrapping_index(1).write(src.wrapping_index(1).read());
    dst.wrapping_index(2).write(src.wrapping_index(2).read());
    let sign = |d: f32| {
        let mut g = Fpu::new();
        g.fld(d);
        g.fcomp_m(zero());
        if g.jne(0x41) { -1.0f32 } else { 1.0f32 }
    };
    if !f.jp(5) {
        f.fcom_m(dist_z);
        if !f.jp(5) {
            let s = sign(dx);
            ww(cp, 10, f.fstp());
            ww(cp, 4, s);
            ww(cp, 5, 0.0);
            ww(cp, 6, 0.0);
            return;
        }
    }
    f.fld(dist_y);
    f.fcomp_st(1);
    f.fstp_st(0);
    if !f.jp(5) {
        f.fld(dist_y);
        f.fcomp_m(dist_z);
        if !f.jp(5) {
            ww(cp, 4, 0.0);
            ww(cp, 5, sign(dy));
            ww(cp, 6, 0.0);
            ww(cp, 10, dist_y);
            return;
        }
    }
    ww(cp, 4, 0.0);
    ww(cp, 5, 0.0);
    ww(cp, 6, sign(dz));
    ww(cp, 10, dist_z);
}

sa_core::orig! {
    /// `CVector::Normalise` (0x59C910)
    pub fn vec_normalise(v: GPtr<sa_math::vector::CVector>) = thiscall 0x59C910 => super::data::normalise_port;
}

/// `CCollision::ProcessSphereSphere(const CColSphere& a, const CColSphere& b, CColPoint& cp, float&
/// maxTouch)` (0x416450, AL). `d = a.c - b.c` (stored), `touch = sqrt((d.z*d.z + d.x*d.x) + d.y*d.y) -
/// b.r` (stored), `depth = a.r - touch` (stored, before the clamp), `touch < 0 → 0`, `touchSq =
/// touch*touch` (stored); rejects unless `touchSq < maxTouch` and `touch < a.r` (strict, NaN rejects;
/// gr adds a `(ra + rb)^2` pre-test). `Normalise(d)`, `point = a.c - d*touch`, surfaces A = a+0x10..0x12,
/// B = b+0x10..0x12, `maxTouch = touchSq`.
#[hook(addr = 0x416450, conv = cdecl, name = "CCollision::ProcessSphereSphere", expect = "83 EC 28 53 56")]
pub fn process_sphere_sphere(a: GPtr<f32>, b: GPtr<f32>, cp: GPtr<f32>, max_touch: GPtr<f32>) -> u8 {
    let mut f = Fpu::new();
    let mut d = [0f32; 3];
    f.fld(w(a, 0));
    f.fsub_m(w(b, 0));
    d[0] = f.fstp();
    f.fld(w(a, 1));
    f.fsub_m(w(b, 1));
    d[1] = f.fstp();
    f.fld(w(a, 2));
    f.fsub_m(w(b, 2));
    d[2] = f.fst();
    f.fmul_m(d[2]);
    f.fld(d[0]);
    f.fmul_m(d[0]);
    f.faddp(1);
    f.fld(d[1]);
    f.fmul_m(d[1]);
    f.faddp(1);
    f.fsqrt();
    f.fsub_m(w(b, 3));
    let mut touch = f.fstp();
    f.fld(w(a, 3));
    f.fsub_m(touch);
    let depth = f.fstp();
    f.fld(touch);
    f.fcomp_m(zero());
    if !f.jp(5) {
        touch = f32::from_bits(0);
    }
    f.fld(touch);
    f.fmul_m(touch);
    let touch_sq = f.fst();
    f.fcomp_m(max_touch.read());
    if f.jp(5) {
        return 0;
    }
    f.fld(touch);
    f.fcomp_m(w(a, 3));
    if f.jp(5) {
        return 0;
    }
    let dp = GPtr::from_raw(d.as_mut_ptr()).cast::<sa_math::vector::CVector>();
    vec_normalise(dp);
    f.fld(d[0]);
    f.fmul_m(touch);
    f.fld(d[1]);
    f.fmul_m(touch);
    f.fld(d[2]);
    f.fmul_m(touch);
    let tz = f.fstp();
    f.fld(w(a, 0));
    f.fsub_st(2);
    let px = f.fstp();
    f.fld(w(a, 1));
    f.fsub_st(1);
    let py = f.fstp();
    f.fstp_st(0);
    f.fstp_st(0);
    f.fld(w(a, 2));
    f.fsub_m(tz);
    let pz = f.fstp();
    ww(cp, 0, px);
    ww(cp, 1, py);
    ww(cp, 2, pz);
    ww(cp, 4, d[0]);
    ww(cp, 5, d[1]);
    ww(cp, 6, d[2]);
    copy_bytes(cp, 0x20, a, 0x10, 2);
    copy_bytes(cp, 0x22, a, 0x12, 1);
    copy_bytes(cp, 0x23, b, 0x10, 2);
    copy_bytes(cp, 0x25, b, 0x12, 1);
    ww(cp, 10, depth);
    max_touch.write(touch_sq);
    1
}

/// `CCollision::ProcessLineSphere(const CColLine&, const CColSphere&, CColPoint& cp, float& depth)`
/// (0x412AA0, AL). `A = (d.z*d.z + d.x*d.x) + d.y*d.y` (stored), `B = -((m.z*d.z + m.y*d.y) +
/// m.x*d.x)`, `disc = B*B - (((m.z*m.z + m.y*m.y) + m.x*m.x) - r*r) * A`; false when `disc < 0`;
/// `t = (-B - sqrt(disc)) / A` (stored); false when `t < 0`, `t > 1` or `!(t < depth)`. Then the point
/// `d*t + s` (stored), the normal `Normalise(point - c)`, surfaces B = sphere+0x10..0x12, A = 0 and
/// piece A = 0 (lighting A untouched), `depth = t`, `ms_iProcessLineNumCrossings += 2`. (gr: x,y,z sum
/// order and early rejections `c <= 0`, `b > 0`, `a <= 0`.)
#[hook(addr = 0x412AA0, conv = cdecl, name = "CCollision::ProcessLineSphere", expect = "83 EC 18 8B 4C 24 1C")]
pub fn process_line_sphere(line: GPtr<f32>, sp: GPtr<f32>, cp: GPtr<f32>, depth: GPtr<f32>) -> u8 {
    let l = |i| w(line, i);
    let c = |i| w(sp, i);
    let mut f = Fpu::new();
    f.fld(l(4));
    f.fsub_m(l(0));
    f.fld(l(5));
    f.fsub_m(l(1));
    f.fld(l(6));
    f.fsub_m(l(2));
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    let a = f.fstp();
    f.fld(c(0));
    f.fsub_m(l(0));
    let mx = f.fstp();
    f.fld(c(1));
    f.fsub_m(l(1));
    let my = f.fstp();
    f.fld(c(2));
    f.fsub_m(l(2));
    let mz = f.fst();
    f.fmul_st(1);
    f.fld(my);
    f.fmul_st(3);
    f.faddp(1);
    f.fld(mx);
    f.fmul_st(4);
    f.faddp(1);
    f.fchs();
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
    f.fld(c(3));
    f.fld_st(1);
    f.fmul_st(2);
    f.fld(mz);
    f.fmul_m(mz);
    f.fld(my);
    f.fmul_m(my);
    f.faddp(1);
    f.fld(mx);
    f.fmul_m(mx);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.fsubp(1);
    f.fmul_m(a);
    f.fsubp(1);
    f.fstp_st(1);
    f.fcom_m(zero());
    if !f.jp(5) {
        return 0;
    }
    f.fxch(1);
    f.fchs();
    f.fxch(1);
    f.fsqrt();
    f.fsubp(1);
    f.fdiv_m(a);
    let t = f.fst();
    f.fcomp_m(zero());
    if !f.jp(5) {
        return 0;
    }
    f.fld(t);
    f.fcomp_m(cf(k::F1));
    if !f.jne(0x41) {
        return 0;
    }
    f.fld(t);
    f.fcomp_m(depth.read());
    if f.jp(5) {
        return 0;
    }
    f.fld(l(4));
    f.fsub_m(l(0));
    f.fld(l(5));
    f.fsub_m(l(1));
    f.fld(l(6));
    f.fsub_m(l(2));
    let dz = f.fstp();
    f.fxch(1);
    f.fmul_m(t);
    let dxt = f.fstp();
    f.fmul_m(t);
    f.fld(dz);
    f.fmul_m(t);
    f.fld(dxt);
    f.fadd_m(l(0));
    let px = f.fstp();
    f.fxch(1);
    f.fadd_m(l(1));
    let py = f.fstp();
    f.fadd_m(l(2));
    let pz = f.fstp();
    let mut n = [0f32; 3];
    for (i, v) in [px, py, pz].iter().enumerate() {
        let mut g = Fpu::new();
        g.fld(*v);
        g.fsub_m(c(i as isize));
        n[i] = g.fstp();
    }
    vec_normalise(GPtr::from_raw(n.as_mut_ptr()).cast());
    ww(cp, 0, px);
    ww(cp, 1, py);
    ww(cp, 2, pz);
    ww(cp, 4, n[0]);
    ww(cp, 5, n[1]);
    ww(cp, 6, n[2]);
    copy_bytes(cp, 0x23, sp, 0x10, 2);
    copy_bytes(cp, 0x25, sp, 0x12, 1);
    cp.cast::<u8>().byte_add(0x20).write(0);
    cp.cast::<u8>().byte_add(0x21).write(0);
    depth.write(t);
    PROCESS_LINE_NUM_CROSSINGS.set(PROCESS_LINE_NUM_CROSSINGS.get().wrapping_add(2));
    1
}

/// The small vector helpers the COLL code calls (`.text`, not hooked by any crate; their bodies are
/// modelled here instruction by instruction and used inline).
pub mod vh {
    use super::Fpu;
    use crate::collision::shared::{cf, k};

    /// `VectorSub(out, a, b)` 0x40FE60: `a - b`, z, y, x computed, then x, y, z stored.
    pub fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        let mut f = Fpu::new();
        f.fld(a[2]);
        f.fsub_m(b[2]);
        f.fld(a[1]);
        f.fsub_m(b[1]);
        f.fld(a[0]);
        f.fsub_m(b[0]);
        [f.fstp(), f.fstp(), f.fstp()]
    }

    /// 0x40FE30 `(out, a, b)`: `a + b`, same order as [`sub`].
    pub fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        let mut f = Fpu::new();
        f.fld(a[2]);
        f.fadd_m(b[2]);
        f.fld(a[1]);
        f.fadd_m(b[1]);
        f.fld(a[0]);
        f.fadd_m(b[0]);
        [f.fstp(), f.fstp(), f.fstp()]
    }

    /// 0x40FEC0 `(out, v, s)`: `s * v` per component (`fld s; fmul v.c`).
    pub fn scale(v: [f32; 3], s: f32) -> [f32; 3] {
        let mut f = Fpu::new();
        for c in [2, 1, 0] {
            f.fld(s);
            f.fmul_m(v[c]);
        }
        [f.fstp(), f.fstp(), f.fstp()]
    }

    /// 0x4119D0 `(out, v, d)`: `k = 1.0f / d` (z uses the register value, x and y the f32-stored one).
    pub fn div(v: [f32; 3], d: f32) -> [f32; 3] {
        let mut f = Fpu::new();
        f.fld(cf(k::F1));
        f.fdiv_m(d);
        let kk = f.fst();
        f.fmul_m(v[2]);
        f.fld(kk);
        f.fmul_m(v[1]);
        f.fld(kk);
        f.fmul_m(v[0]);
        [f.fstp(), f.fstp(), f.fstp()]
    }

    /// `operator*(const CMatrix&, const CVector&)` 0x59C890 on a matrix in memory (16 f32 words):
    /// `((up.c*v.z + r1.c*v.a) + r2.c*v.b) + pos.c` as `sa_math::vector::matrix_mul_vector`.
    pub fn mat_mul_point(m: sa_core::mem::GPtr<f32>, v: [f32; 3]) -> [f32; 3] {
        let w = |i: i32| m.wrapping_index(i).read();
        let mut out = [0f32; 3];
        // x: up.x*v.z + fwd.x*v.y + right.x*v.x; y, z: up*v.z + right*v.x + fwd*v.y
        let terms = [
            [(8, 2), (4, 1), (0, 0)],
            [(9, 2), (1, 0), (5, 1)],
            [(10, 2), (2, 0), (6, 1)],
        ];
        for (c, t) in terms.iter().enumerate() {
            let mut f = Fpu::new();
            f.fld(w(t[0].0));
            f.fmul_m(v[t[0].1]);
            f.fld(w(t[1].0));
            f.fmul_m(v[t[1].1]);
            f.faddp(1);
            f.fld(w(t[2].0));
            f.fmul_m(v[t[2].1]);
            f.faddp(1);
            f.fadd_m(w(12 + c as i32));
            out[c] = f.fstp();
        }
        out
    }

    /// `Multiply3x3(out, const CMatrix&, const CVector&)` 0x59C790: x = `(up.x*v.z + fwd.x*v.y) +
    /// right.x*v.x`, y = `(up.y*v.z + right.y*v.x) + fwd.y*v.y`, z likewise (no translation).
    pub fn mul3x3(m: sa_core::mem::GPtr<f32>, v: [f32; 3]) -> [f32; 3] {
        let w = |i: i32| m.wrapping_index(i).read();
        let terms = [
            [(8, 2), (4, 1), (0, 0)],
            [(9, 2), (1, 0), (5, 1)],
            [(10, 2), (2, 0), (6, 1)],
        ];
        core::array::from_fn(|c| {
            let t = terms[c];
            let mut f = Fpu::new();
            f.fld(w(t[0].0));
            f.fmul_m(v[t[0].1]);
            f.fld(w(t[1].0));
            f.fmul_m(v[t[1].1]);
            f.faddp(1);
            f.fld(w(t[2].0));
            f.fmul_m(v[t[2].1]);
            f.faddp(1);
            f.fstp()
        })
    }

    /// 0x406DA0 (thiscall, ST0): `(x*x + y*y) + z*z`, registers.
    pub fn mag_sqr(f: &mut Fpu, v: [f32; 3]) {
        f.fld(v[2]);
        f.fld(v[1]);
        f.fld(v[0]);
        f.fld_st(0);
        f.fmul_st(1);
        f.fld_st(2);
        f.fmul_st(3);
        f.faddp(1);
        f.fld_st(3);
        f.fmul_st(4);
        f.faddp(1);
        f.fstp_st(3);
        f.fstp_st(0);
        f.fstp_st(0);
    }
}

fn rv(p: GPtr<f32>) -> [f32; 3] {
    [w(p, 0), w(p, 1), w(p, 2)]
}

sa_core::orig! {
    /// `CCollision::CalculateColPointInsideBox` (0x411EC0)
    pub fn calculate_col_point_inside_box_call(bx: GPtr<f32>, p: GPtr<f32>, cp: GPtr<f32>)
        = cdecl 0x411EC0 => calculate_col_point_inside_box;
}

/// `CCollision::ProcessSphereBox(const CColSphere&, const CColBox&, CColPoint& cp, float& minDistSq)`
/// (0x412130, AL). Per axis: reject when `c + r < min` or `c - r > max` (NaN passes); the point is
/// `min` when `c < min`, else the centre when `c <= max` or NaN, else `max` (raw dword copies).
/// Outside (some axis clamped): `dir = c - p` (0x40FE60), `distSq = (x*x + y*y) + z*z` (stored); reject
/// unless `distSq < minDistSq`; `dist = sqrt` (stored); reject unless `dist <= r` (equal accepted, NaN
/// rejected; gr rejects equal); point, normal `dir * (1/dist)` (0x4119D0), surfaces A = sphere
/// +0x10..0x12 and B = box +0x18..0x1A (pieces included), depth `r - dist`, `minDistSq = distSq`.
/// Inside: `CalculateColPointInsideBox(box, c, cp)`, `depth += r`, `point -= normal * r`, the surfaces,
/// `minDistSq = 0`.
#[hook(addr = 0x412130, conv = cdecl, name = "CCollision::ProcessSphereBox", expect = "83 EC 18 53 55")]
pub fn process_sphere_box(sp: GPtr<f32>, bx: GPtr<f32>, cp: GPtr<f32>, min_dist_sq: GPtr<f32>) -> u8 {
    #[derive(PartialEq)]
    enum Side {
        Min,
        Centre,
        Max,
    }
    let r = w(sp, 3);
    let mut side = [Side::Centre, Side::Centre, Side::Centre];
    let mut f = Fpu::new();
    for a in 0..3isize {
        let c = w(sp, a);
        f.fld(c);
        f.fadd_m(r);
        f.fcomp_m(w(bx, a));
        if !f.jp(5) {
            return 0;
        }
        f.fld(c);
        f.fsub_m(r);
        f.fcomp_m(w(bx, a + 3));
        if !f.jne(0x41) {
            return 0;
        }
        f.fld(c);
        f.fcomp_m(w(bx, a));
        side[a as usize] = if !f.jp(5) {
            Side::Min
        } else {
            f.fld(c);
            f.fcomp_m(w(bx, a + 3));
            if f.jne(0x41) { Side::Centre } else { Side::Max }
        };
    }
    let surfaces = || {
        copy_bytes(cp, 0x20, sp, 0x10, 2);
        copy_bytes(cp, 0x22, sp, 0x12, 1);
        copy_bytes(cp, 0x23, bx, 0x18, 2);
        copy_bytes(cp, 0x25, bx, 0x1A, 1);
    };
    if side.iter().all(|s| *s == Side::Centre) {
        calculate_col_point_inside_box_call(bx, sp, cp);
        let mut g = Fpu::new();
        g.fld(w(cp, 10));
        g.fadd_m(r);
        ww(cp, 10, g.fstp());
        let t = vh::scale([w(cp, 4), w(cp, 5), w(cp, 6)], r);
        let q = vh::sub(rv(cp), t);
        for (i, v) in q.iter().enumerate() {
            ww(cp, i as isize, *v);
        }
        surfaces();
        min_dist_sq.write(f32::from_bits(0));
        return 1;
    }
    let (spu, bxu) = (sp.cast::<u32>(), bx.cast::<u32>());
    let pw: [u32; 3] = core::array::from_fn(|a| match side[a] {
        Side::Min => bxu.wrapping_index(a as i32).read(),
        Side::Centre => spu.wrapping_index(a as i32).read(),
        Side::Max => bxu.wrapping_index(a as i32 + 3).read(),
    });
    let p = pw.map(f32::from_bits);
    let dir = vh::sub(rv(sp), p);
    let mut g = Fpu::new();
    vh::mag_sqr(&mut g, dir);
    let dist_sq = g.fst();
    g.fcomp_m(min_dist_sq.read());
    if g.jp(5) {
        return 0;
    }
    g.fld(dist_sq);
    g.fsqrt();
    let dist = g.fst();
    g.fcomp_m(r);
    if g.jp(0x41) {
        return 0;
    }
    let cpu = cp.cast::<u32>();
    for (i, v) in pw.iter().enumerate() {
        cpu.wrapping_index(i as i32).write(*v);
    }
    let n = vh::div(dir, dist);
    for (i, v) in n.iter().enumerate() {
        ww(cp, 4 + i as isize, *v);
    }
    surfaces();
    let mut h = Fpu::new();
    h.fld(r);
    h.fsub_m(dist);
    min_dist_sq.write(dist_sq);
    ww(cp, 10, h.fstp());
    1
}

/// `CColLine::CColLine(const CVector& start, const CVector& end)` (stub 0x40EF50 → 0x15611B0) on a
/// local: raw dword copies (the pads are left as they are; nothing reads them).
fn col_line_local(start: GPtr<f32>, end: GPtr<f32>) -> [u32; 8] {
    let (a, b) = (start.cast::<u32>(), end.cast::<u32>());
    let mut l = [0u32; 8];
    for i in 0..3 {
        l[i] = a.wrapping_index(i as i32).read();
        l[4 + i] = b.wrapping_index(i as i32).read();
    }
    l
}

sa_core::orig! {
    /// `CCollision::TestSphereSphere` (0x411E70)
    pub fn test_sphere_sphere_call(a: GPtr<f32>, b: GPtr<f32>) -> u32 = cdecl 0x411E70 => test_sphere_sphere;
    /// `CCollision::TestLineSphere` (0x417470)
    pub fn test_line_sphere_call(line: GPtr<f32>, sp: GPtr<f32>) -> u8 = cdecl 0x417470 => test_line_sphere;
}

/// `CCollision::SphereCastVsBBox(const CColSphere& a, const CColSphere& b, const CColBox& box)`
/// (0x415590, AL 0/1): the box grown by **a's** radius (min - r, max + r stored on a local copy) tested
/// against the line from a's to b's centre with `TestLineBox_DW`.
#[hook(addr = 0x415590, conv = cdecl, name = "CCollision::SphereCastVsBBox", expect = "83 EC 3C 8B 44 24 40")]
pub fn sphere_cast_vs_bbox(a: GPtr<f32>, b: GPtr<f32>, bx: GPtr<f32>) -> u8 {
    let mut lb = [0u32; 7];
    for (i, v) in lb.iter_mut().enumerate() {
        *v = bx.cast::<u32>().wrapping_index(i as i32).read();
    }
    let mut f = Fpu::new();
    f.fld(w(a, 3));
    for x in lb.iter_mut().take(3) {
        f.fld(f32::from_bits(*x));
        f.fsub_st(1);
        *x = f.fstp().to_bits();
    }
    for x in lb.iter_mut().skip(3).take(3) {
        f.fld(f32::from_bits(*x));
        f.fadd_st(1);
        *x = f.fstp().to_bits();
    }
    let mut line = col_line_local(a, b);
    let r = test_line_box_dw_call(
        GPtr::from_raw(line.as_mut_ptr()).cast(),
        GPtr::from_raw(lb.as_mut_ptr()).cast(),
    );
    (r != 0) as u8
}

/// `CCollision::SphereCastVsSphere(const CColSphere& a, const CColSphere& b, const CColSphere& s)`
/// (0x417F20, AL 0/1): true when `(s.r + a.r)^2 > (d.x*d.x + d.z*d.z) + d.y*d.y` for `d = a.c - s.c`
/// (strict), else when `TestSphereSphere(b, s)`, else `TestLineSphere(line a→b, s grown by a.r)`.
#[hook(addr = 0x417F20, conv = cdecl, name = "CCollision::SphereCastVsSphere", expect = "83 EC 34 53 8B 5C 24 3C")]
pub fn sphere_cast_vs_sphere(a: GPtr<f32>, b: GPtr<f32>, sph: GPtr<f32>) -> u8 {
    let mut f = Fpu::new();
    f.fld(w(a, 0));
    f.fsub_m(w(sph, 0));
    f.fld(w(a, 1));
    f.fsub_m(w(sph, 1));
    f.fld(w(a, 2));
    f.fsub_m(w(sph, 2));
    f.fld(w(sph, 3));
    f.fadd_m(w(a, 3));
    f.fld_st(3);
    f.fmul_st(4);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fld_st(1);
    f.fmul_st(2);
    f.fcompp();
    if !f.jne(0x41) {
        return 1;
    }
    if test_sphere_sphere_call(b, sph) as u8 != 0 {
        return 1;
    }
    let mut line = col_line_local(a, b);
    let mut s = [0u32; 5];
    for (i, v) in s.iter_mut().enumerate() {
        *v = sph.cast::<u32>().wrapping_index(i as i32).read();
    }
    let mut g = Fpu::new();
    g.fld(f32::from_bits(s[3]));
    g.fadd_m(w(a, 3));
    s[3] = g.fstp().to_bits();
    let r = test_line_sphere_call(
        GPtr::from_raw(line.as_mut_ptr()).cast(),
        GPtr::from_raw(s.as_mut_ptr()).cast(),
    );
    (r != 0) as u8
}

/// `CColPoint::operator=` (stub 0x40FC80 → 0x15637E0): point, normal, the surface bytes +0x20..+0x25
/// (word/byte/word/byte) and the depth, as raw moves (+0xC, +0x1C, +0x26 not copied).
pub fn col_point_copy(dst: GPtr<f32>, src: GPtr<f32>) {
    let (d, s) = (dst.cast::<u32>(), src.cast::<u32>());
    for i in [0, 1, 2, 4, 5, 6] {
        d.wrapping_index(i).write(s.wrapping_index(i).read());
    }
    copy_bytes(dst, 0x20, src, 0x20, 2);
    copy_bytes(dst, 0x22, src, 0x22, 1);
    copy_bytes(dst, 0x23, src, 0x23, 2);
    copy_bytes(dst, 0x25, src, 0x25, 1);
    d.wrapping_index(10).write(s.wrapping_index(10).read());
}

/// `CCollision::ProcessDiscCollision(CColPoint& tri, const CMatrix& matBA, const CColDisk& disk,
/// CColPoint& diskCP, bool& lineCollision, float& lineRatio, CColPoint& lineCP)` (0x413960, AL).
/// `p = matBA * tri.point`, `n = Multiply3x3(matBA, tri.normal)`. When `|(n.y*t.y + n.z*t.z) + n.x*t.x|
/// < 0.77` and `|((p.z - c.z)*t.z + dy*t.y) + dx*t.x| < thickness` (`dx`, `dy` stored): the line case,
/// `hitK = sqrt((r*r - dy*dy) - dx*dx) + p.z` with no clamp; when `!(hitK < lineRatio)` (NaN → no),
/// `lineCollision = 1`, `lineRatio = hitK`, `lineCP = tri` (+ depth through the FPU); returns 0.
/// Otherwise (the disk case): when `disk.piece < 0x11` and `tri.depth > diskCP.depth`, `diskCP = tri`,
/// `diskCP.surfaceB = 0x3C`, returns 1; else 0. (gr: `max(0, ...)` clamp and x, y, z sum order.)
#[hook(addr = 0x413960, conv = cdecl, name = "CCollision::ProcessDiscCollision", expect = "83 EC 24 53 56")]
pub fn process_disc_collision(
    tri: GPtr<f32>,
    mat: GPtr<f32>,
    disk: GPtr<f32>,
    disk_cp: GPtr<f32>,
    line_collision: GPtr<u8>,
    line_ratio: GPtr<f32>,
    line_cp: GPtr<f32>,
) -> u8 {
    let p = vh::mat_mul_point(mat, rv(tri));
    let n = vh::mul3x3(mat, [w(tri, 4), w(tri, 5), w(tri, 6)]);
    let dk = |i| w(disk, i);
    let mut f = Fpu::new();
    f.fld(n[1]);
    f.fmul_m(dk(6));
    f.fld(n[2]);
    f.fmul_m(dk(7));
    f.faddp(1);
    f.fld(n[0]);
    f.fmul_m(dk(5));
    f.faddp(1);
    f.fabs();
    f.fcomp_m(f32::from_bits(0x3F45_1EB8));
    let mut line_case = !f.jp(5);
    let (mut dx, mut dy) = (0f32, 0f32);
    if line_case {
        f.fld(p[0]);
        f.fsub_m(dk(0));
        dx = f.fst();
        f.fld(p[1]);
        f.fsub_m(dk(1));
        dy = f.fst();
        f.fld(p[2]);
        f.fsub_m(dk(2));
        f.fmul_m(dk(7));
        f.fxch(1);
        f.fmul_m(dk(6));
        f.faddp(1);
        f.fxch(1);
        f.fmul_m(dk(5));
        f.faddp(1);
        f.fabs();
        f.fcomp_m(dk(8));
        line_case = !f.jp(5);
    }
    if line_case {
        let mut g = Fpu::new();
        g.fld(dy);
        g.fld(dx);
        g.fld(dk(3));
        g.fld_st(0);
        g.fmul_st(1);
        g.fld_st(3);
        g.fmul_st(4);
        g.fsubp(1);
        g.fld_st(2);
        g.fmul_st(3);
        g.fsubp(1);
        g.fsqrt();
        g.fstp_st(3);
        g.fstp_st(0);
        g.fstp_st(0);
        g.fadd_m(p[2]);
        g.fld_st(0);
        g.fcomp_m(line_ratio.read());
        if g.jne(1) {
            return 0;
        }
        line_collision.write(1);
        line_ratio.write(g.fstp());
        col_point_copy(line_cp, tri);
        ww(line_cp, 10, X87::from_f32(w(tri, 10)).to_f32());
        return 0;
    }
    if disk.cast::<u8>().byte_add(0x11).read() >= 0x11 {
        return 0;
    }
    let mut g = Fpu::new();
    g.fld(w(tri, 10));
    g.fcomp_m(w(disk_cp, 10));
    if g.jne(0x41) {
        return 0;
    }
    col_point_copy(disk_cp, tri);
    disk_cp
        .cast::<u32>()
        .wrapping_index(10)
        .write(tri.cast::<u32>().wrapping_index(10).read());
    disk_cp.cast::<u8>().byte_add(0x23).write(0x3C);
    1
}

sa_core::orig! {
    /// `CCollision::GetPrincipleAxis` (0x4156D0)
    pub fn get_principle_axis_call(n: GPtr<f32>) -> u32 = cdecl 0x4156D0 => get_principle_axis;
}

/// One edge test of `PointInPoly`: `(a0 - a1)*(b0 - b1) - (c0 - c1)*(d0 - d1)` (`fmulp` forms) compared
/// with 0; returns the condition codes holder.
fn poly_edge(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2]) -> Fpu {
    let mut f = Fpu::new();
    f.fld(a[0]);
    f.fsub_m(a[1]);
    f.fld(b[0]);
    f.fsub_m(b[1]);
    f.fmulp(1);
    f.fld(c[0]);
    f.fsub_m(c[1]);
    f.fld(d[0]);
    f.fsub_m(d[1]);
    f.fmulp(1);
    f.fsubp(1);
    f.fcomp_m(zero());
    f
}

/// `CCollision::PointInPoly(const CVector& p, const CColTriangle&, const CVector& normal, const CVector*
/// verts)` (0x415730, EAX 0/1): the 2D edge sign tests in the plane that drops `GetPrincipleAxis(normal)`
/// (axis 0: y/z, `>= 0`; axis 2: x/z, `<= 0`; axis 4: x/y, `>= 0`; NaN counts as "not"); true when the
/// three results are equal. Other axis values (never returned) compare three uninitialised stack bytes
/// in the original; the port compares zeros (true).
#[hook(addr = 0x415730, conv = cdecl, name = "CCollision::PointInPoly", expect = "51 8B 44 24 10 53")]
pub fn point_in_poly(p: GPtr<f32>, _tri: GPtr<u8>, normal: GPtr<f32>, verts: GPtr<f32>) -> u32 {
    let v = |i: usize, c: usize| w(verts, (3 * i + c) as isize);
    let q = |c: usize| w(p, c as isize);
    let (b0, b1, b2) = match get_principle_axis_call(normal) {
        4 => {
            let ge = |f: Fpu| !f.jne(1);
            (
                ge(poly_edge(
                    [q(0), v(0, 0)],
                    [v(1, 1), v(0, 1)],
                    [q(1), v(0, 1)],
                    [v(1, 0), v(0, 0)],
                )),
                ge(poly_edge(
                    [q(0), v(1, 0)],
                    [v(2, 1), v(1, 1)],
                    [q(1), v(1, 1)],
                    [v(2, 0), v(1, 0)],
                )),
                ge(poly_edge(
                    [q(0), v(2, 0)],
                    [v(0, 1), v(2, 1)],
                    [q(1), v(2, 1)],
                    [v(0, 0), v(2, 0)],
                )),
            )
        }
        2 => {
            let le = |f: Fpu| !f.jp(0x41);
            (
                le(poly_edge(
                    [q(0), v(0, 0)],
                    [v(1, 2), v(0, 2)],
                    [q(2), v(0, 2)],
                    [v(1, 0), v(0, 0)],
                )),
                le(poly_edge(
                    [v(2, 2), v(1, 2)],
                    [q(0), v(1, 0)],
                    [q(2), v(1, 2)],
                    [v(2, 0), v(1, 0)],
                )),
                le(poly_edge(
                    [q(0), v(2, 0)],
                    [v(0, 2), v(2, 2)],
                    [q(2), v(2, 2)],
                    [v(0, 0), v(2, 0)],
                )),
            )
        }
        0 => {
            let ge = |f: Fpu| !f.jne(1);
            (
                ge(poly_edge(
                    [q(1), v(0, 1)],
                    [v(1, 2), v(0, 2)],
                    [v(1, 1), v(0, 1)],
                    [q(2), v(0, 2)],
                )),
                ge(poly_edge(
                    [q(1), v(1, 1)],
                    [v(2, 2), v(1, 2)],
                    [q(2), v(1, 2)],
                    [v(2, 1), v(1, 1)],
                )),
                ge(poly_edge(
                    [q(1), v(2, 1)],
                    [v(0, 2), v(2, 2)],
                    [q(2), v(2, 2)],
                    [v(0, 1), v(2, 1)],
                )),
            )
        }
        _ => (false, false, false),
    };
    (b0 == b1 && b0 == b2) as u32
}

/// `CCollision::ClosestSquaredDistanceBetweenFiniteLines(const CVector* p0, const CVector* q0, const
/// CVector* q1, const CVector* u, float uu)` (0x415A40, ST0; spec `docs/re/specs/COLL/CCollision.md`):
/// Sunday's segment/segment distance (`p0 .. p0+u` against `q0 .. q1`) with the original's sum orders,
/// f32 stores, clamps and `1e-5` thresholds. gta-reversed's live code is Ericson's
/// `ClosestPtSegmentSegment` (a different algorithm); the bytes are followed.
#[hook(addr = 0x415A40, conv = cdecl, name = "CCollision::ClosestSquaredDistanceBetweenFiniteLines", expect = "83 EC 40 8B 44 24 48")]
pub fn closest_squared_distance_between_finite_lines(
    p0: GPtr<f32>,
    q0: GPtr<f32>,
    q1: GPtr<f32>,
    u: GPtr<f32>,
    uu: f32,
) -> X87 {
    let small = cf(k::SMALL);
    let sub = |a: f32, b: f32| X87::from_f32(a).fsub_m(b).to_f32();
    let v = [
        sub(w(q1, 0), w(q0, 0)),
        sub(w(q1, 1), w(q0, 1)),
        sub(w(q1, 2), w(q0, 2)),
    ];
    let ww_ = [
        sub(w(p0, 0), w(q0, 0)),
        sub(w(p0, 1), w(q0, 1)),
        sub(w(p0, 2), w(q0, 2)),
    ];
    let dot = |a: [f32; 3], b: [f32; 3], o: [usize; 3]| {
        let mut f = Fpu::new();
        f.fld(a[o[0]]);
        f.fmul_m(b[o[0]]);
        f.fld(a[o[1]]);
        f.fmul_m(b[o[1]]);
        f.faddp(1);
        f.fld(a[o[2]]);
        f.fmul_m(b[o[2]]);
        f.faddp(1);
        f.fstp()
    };
    let uv = rv(u);
    let b = dot(v, uv, [2, 1, 0]);
    let c = dot(v, v, [0, 2, 1]);
    let d = dot(ww_, uv, [2, 1, 0]);
    let e = dot(ww_, v, [0, 2, 1]);
    let mut f = Fpu::new();
    f.fld(c);
    f.fmul_m(uu);
    f.fld(b);
    f.fmul_m(b);
    f.fsubp(1);
    let dd = f.fstp();
    let mut s_d = dd;
    let mut t_d = dd;
    let mut t_n: f32;
    f.fld(dd);
    f.fcomp_m(small);
    let parallel = !f.jp(5);
    let mut clamp_path = true;
    if parallel {
        s_d = cf(k::F1);
        f.fld(zero());
        t_n = e;
        t_d = c;
    } else {
        f.fld(e);
        f.fmul_m(b);
        f.fld(d);
        f.fmul_m(c);
        f.fsubp(1);
        let mut g = Fpu::new();
        g.fld(e);
        g.fmul_m(uu);
        g.fld(d);
        g.fmul_m(b);
        g.fsubp(1);
        t_n = g.fstp();
        f.fcom_m(zero());
        if !f.jp(5) {
            f.fstp_st(0);
            f.fld(zero());
            t_n = e;
            t_d = c;
        } else {
            f.fcom_m(dd);
            if !f.jne(0x41) {
                f.fstp_st(0);
                f.fld(dd);
                t_d = c;
                let mut g = Fpu::new();
                g.fld(e);
                g.fadd_m(b);
                t_n = g.fstp();
            }
        }
    }
    let mut g = Fpu::new();
    g.fld(t_n);
    g.fcomp_m(zero());
    if !g.jp(5) {
        f.fstp_st(0);
        t_n = f32::from_bits(0);
        f.fld(d);
        f.fchs();
    } else {
        g.fld(t_n);
        g.fcomp_m(t_d);
        if g.jne(0x41) {
            clamp_path = false;
        } else {
            f.fstp_st(0);
            f.fld(b);
            f.fsub_m(d);
            t_n = t_d;
        }
    }
    if clamp_path {
        f.fcom_m(zero());
        if !f.jp(5) {
            f.fstp_st(0);
            f.fld(zero());
        } else {
            f.fcom_m(uu);
            if f.jne(0x41) {
                s_d = uu;
            } else {
                f.fstp_st(0);
                f.fld(s_d);
            }
        }
    }
    f.fld_st(0);
    f.fabs();
    f.fcomp_m(small);
    if f.jp(5) {
        f.fdiv_m(s_d);
    } else {
        f.fstp_st(0);
        f.fld(zero());
    }
    let mut h = Fpu::new();
    h.fld(t_n);
    h.fabs();
    h.fcomp_m(small);
    if h.jp(5) {
        f.fld(t_n);
        f.fdiv_m(t_d);
    } else {
        f.fld(zero());
    }
    // stack: tc, sc
    f.fld(v[0]);
    f.fmul_st(1);
    let tvx = f.fstp();
    f.fld(v[1]);
    f.fmul_st(1);
    let tvy = f.fstp();
    f.fmul_m(v[2]);
    f.fld_st(1);
    f.fmul_m(uv[0]);
    f.fld_st(2);
    f.fmul_m(uv[1]);
    f.fxch(3);
    f.fmul_m(uv[2]);
    let scuz = f.fstp();
    f.fadd_m(ww_[0]);
    let px = f.fstp();
    f.fxch(1);
    f.fadd_m(ww_[1]);
    f.fxch(1);
    f.fld(scuz);
    f.fadd_m(ww_[2]);
    let pz = f.fstp();
    f.fld(px);
    f.fsub_m(tvx);
    let dx = f.fstp();
    f.fxch(1);
    f.fsub_m(tvy);
    let dy = f.fstp();
    f.fld(pz);
    f.fsub_st(1);
    f.fxch(1);
    f.fstp_st(0);
    f.fld_st(0);
    f.fmulp(1);
    f.fld(dy);
    f.fmul_m(dy);
    f.faddp(1);
    f.fld(dx);
    f.fmul_m(dx);
    f.faddp(1);
    f.pop()
}

/// `CCollision::RayPolyPOP(CVector* p, CVector* dir, CColTriangle*, CVector* n, CVector* v)` (0x415620,
/// EAX 0/1; no callers in 1.0 US, gr `NOTSA_UNREACHABLE`). `num = ((v.x-p.x)*n.x + (v.z-p.z)*n.z) +
/// (v.y-p.y)*n.y` (stored): false when `num > 0`; `den = (n.z*dir.z + n.y*dir.y) + dir.x*n.x`: false
/// when `num <= den` (NaN passes); else `p += dir * (num / den)` and true.
#[hook(addr = 0x415620, conv = cdecl, name = "CCollision::RayPolyPOP", expect = "83 EC 0C 8B 44 24 20")]
pub fn ray_poly_pop(p: GPtr<f32>, dir: GPtr<f32>, _tri: GPtr<u8>, n: GPtr<f32>, v: GPtr<f32>) -> u32 {
    let mut f = Fpu::new();
    f.fld(w(v, 0));
    f.fsub_m(w(p, 0));
    f.fld(w(v, 1));
    f.fsub_m(w(p, 1));
    f.fld(w(v, 2));
    f.fsub_m(w(p, 2));
    f.fmul_m(w(n, 2));
    f.fxch(2);
    f.fmul_m(w(n, 0));
    f.faddp(2);
    f.fmul_m(w(n, 1));
    f.faddp(1);
    let num = f.fst();
    f.fcomp_m(zero());
    if !f.jne(0x41) {
        return 0;
    }
    f.fld(w(n, 2));
    f.fmul_m(w(dir, 2));
    f.fld(w(n, 1));
    f.fmul_m(w(dir, 1));
    f.faddp(1);
    f.fld(w(dir, 0));
    f.fmul_m(w(n, 0));
    f.faddp(1);
    f.fld(num);
    f.fcomp_st(1);
    if !f.jp(0x41) {
        return 0;
    }
    f.fdivr_m(num);
    f.fld_st(0);
    f.fmul_m(w(dir, 0));
    f.fld_st(1);
    f.fmul_m(w(dir, 1));
    let ty = f.fstp();
    f.fxch(1);
    f.fmul_m(w(dir, 2));
    let tz = f.fstp();
    f.fadd_m(w(p, 0));
    ww(p, 0, f.fstp());
    f.fld(ty);
    f.fadd_m(w(p, 1));
    ww(p, 1, f.fstp());
    f.fld(tz);
    f.fadd_m(w(p, 2));
    ww(p, 2, f.fstp());
    1
}

/// `|q - p|^2` as Closest3 computes it: the first distance `(dz*dz + dx*dx) + dy*dy`, the other two
/// `(dz*dz + dy*dy) + dx*dx` (`yx_order`), registers.
fn dist_sq_c3(f: &mut Fpu, q: GPtr<f32>, p: GPtr<f32>, yx_order: bool) {
    f.fld(w(q, 0));
    f.fsub_m(w(p, 0));
    f.fld(w(q, 1));
    f.fsub_m(w(p, 1));
    f.fld(w(q, 2));
    f.fsub_m(w(p, 2));
    f.fld_st(0);
    f.fmul_st(1);
    let (a, b) = if yx_order { (2, 3) } else { (3, 2) };
    f.fld_st(a);
    f.fmul_st(a + 1);
    f.faddp(1);
    f.fld_st(b);
    f.fmul_st(b + 1);
    f.faddp(1);
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
}

/// `CCollision::Closest3(CVector* pts, CVector* p)` (0x415950; only caller ClosestPointOnPoly, which has
/// none; gr `NOTSA_UNREACHABLE`): `p` = the nearest of the three points (raw dword copy); `d0` stays a
/// register, `d1`/`d2` are stored; ties and NaN go to the later point.
#[hook(addr = 0x415950, conv = cdecl, name = "CCollision::Closest3", expect = "8B 4C 24 08 D9 01")]
pub fn closest3(pts: GPtr<f32>, p: GPtr<f32>) {
    let pt = |i: i32| pts.wrapping_index(3 * i);
    let mut f = Fpu::new();
    dist_sq_c3(&mut f, p, pt(0), false);
    let mut g = Fpu::new();
    dist_sq_c3(&mut g, p, pt(1), true);
    let d1 = g.fstp();
    dist_sq_c3(&mut g, p, pt(2), true);
    let d2 = g.fstp();
    f.fcom_m(d1);
    let pick = if !f.jp(5) {
        f.fcomp_m(d2);
        if f.jp(5) { 2 } else { 0 }
    } else {
        f.fstp_st(0);
        f.fld(d1);
        f.fcomp_m(d2);
        if f.jp(5) { 2 } else { 1 }
    };
    let (s, d) = (pt(pick).cast::<u32>(), p.cast::<u32>());
    for i in 0..3 {
        d.wrapping_index(i).write(s.wrapping_index(i).read());
    }
}

sa_core::orig! {
    /// `CCollision::ClosestPointOnLine` (0x417FD0)
    pub fn closest_point_on_line_call(a: GPtr<f32>, b: GPtr<f32>, p: GPtr<f32>, out: GPtr<f32>)
        = cdecl 0x417FD0 => closest_point_on_line;
    /// `CCollision::Closest3` (0x415950)
    pub fn closest3_call(pts: GPtr<f32>, p: GPtr<f32>) = cdecl 0x415950 => closest3;
}

/// `CCollision::ClosestPointsOnPoly(CColTriangle*, CVector* verts, CVector* p, CVector* out)` (0x418100;
/// no callers, gr `NOTSA_UNREACHABLE`): `ClosestPointOnLine(p, v1, v0, out[0])`, `(p, v2, v1, out[1])`,
/// `(p, v0, v2, out[2])`.
#[hook(addr = 0x418100, conv = cdecl, name = "CCollision::ClosestPointsOnPoly", expect = "53 8B 5C 24 14")]
pub fn closest_points_on_poly(_tri: GPtr<u8>, verts: GPtr<f32>, p: GPtr<f32>, out: GPtr<f32>) {
    let v = |i: i32| verts.wrapping_index(3 * i);
    let o = |i: i32| out.wrapping_index(3 * i);
    closest_point_on_line_call(p, v(1), v(0), o(0));
    closest_point_on_line_call(p, v(2), v(1), o(1));
    closest_point_on_line_call(p, v(0), v(2), o(2));
}

/// `CCollision::ClosestPointOnPoly(CColTriangle*, CVector* verts, CVector* p)` (0x418150; no callers):
/// the three `ClosestPointOnLine` results on a local array, then `Closest3(local, p)`.
#[hook(addr = 0x418150, conv = cdecl, name = "CCollision::ClosestPointOnPoly", expect = "83 EC 24 53 55")]
pub fn closest_point_on_poly(_tri: GPtr<u8>, verts: GPtr<f32>, p: GPtr<f32>) {
    let v = |i: i32| verts.wrapping_index(3 * i);
    let mut l = [0f32; 9];
    let lp = GPtr::from_raw(l.as_mut_ptr());
    closest_point_on_line_call(p, v(1), v(0), lp);
    closest_point_on_line_call(p, v(2), v(1), lp.wrapping_index(3));
    closest_point_on_line_call(p, v(0), v(2), lp.wrapping_index(6));
    closest3_call(lp, p);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_TEST_SPHERE_SPHERE,
    HOOK_TEST_SPHERE_BOX,
    HOOK_POINT_IN_TRIANGLE,
    HOOK_DIST_TO_LINE_SQR,
    HOOK_DIST_TO_MATHEMATICAL_LINE,
    HOOK_DIST_TO_MATHEMATICAL_LINE_2D,
    HOOK_DIST_ALONG_LINE_2D,
    HOOK_TEST_VERTICAL_LINE_BOX,
    HOOK_TEST_2D_LINE_AGAINST_2D_LINE,
    HOOK_GET_BOUNDING_BOX_FROM_TWO_SPHERES,
    HOOK_GET_PRINCIPLE_AXIS,
    HOOK_IS_THIS_VEHICLE_SITTING_ON_ME,
    HOOK_TEST_LINE_BOX_DW,
    HOOK_TEST_LINE_BOX,
    HOOK_TEST_LINE_SPHERE,
    HOOK_DIST_TO_LINE,
    HOOK_CLOSEST_POINT_ON_LINE,
    HOOK_CALCULATE_COL_POINT_INSIDE_BOX,
    HOOK_PROCESS_SPHERE_SPHERE,
    HOOK_PROCESS_LINE_SPHERE,
    HOOK_PROCESS_SPHERE_BOX,
    HOOK_SPHERE_CAST_VS_BBOX,
    HOOK_SPHERE_CAST_VS_SPHERE,
    HOOK_PROCESS_DISC_COLLISION,
    HOOK_POINT_IN_POLY,
    HOOK_CLOSEST_SQUARED_DISTANCE_BETWEEN_FINITE_LINES,
    HOOK_RAY_POLY_POP,
    HOOK_CLOSEST3,
    HOOK_CLOSEST_POINTS_ON_POLY,
    HOOK_CLOSEST_POINT_ON_POLY,
];
