//! `CWaterLevel::SplitWaterTriangleAlongXLine` and `SplitWaterTriangleAlongYLine`: a right triangle cut by
//! a camera-range line into a rectangle and two triangles.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_math::x87::X87;

use crate::common::{f32_of, x, xi};
use crate::waterlevel::lerp_into;

use crate::bindings::o_render_water_rectangle as render_water_rectangle;
use crate::bindings::render_water_triangle;

/// A `CRenPar` by value: z, big waves, small waves (f32 bits), then the flow bytes dword.
#[derive(Clone, Copy)]
struct RenPar([u32; 4]);

impl RenPar {
    fn f(&self, i: usize) -> f32 {
        f32::from_bits(self.0[i])
    }
    /// An interpolated value: the flow bytes are zero; the original leaves the dword's upper half as
    /// whatever its stack slot held (uninitialised; the callees read the two bytes only), here 0.
    fn new(z: f32, b: f32, s: f32) -> RenPar {
        RenPar([z.to_bits(), b.to_bits(), s.to_bits(), 0])
    }
}

#[allow(clippy::too_many_arguments)]
fn tri(x1: i32, y1: i32, a: RenPar, x2: i32, y2: i32, b: RenPar, x3: i32, y3: i32, c: RenPar) {
    let (a, b, c) = (a.0, b.0, c.0);
    render_water_triangle(x1, y1, a[0], a[1], a[2], a[3], x2, y2, b[0], b[1], b[2], b[3], x3, y3, c[0], c[1], c[2], c[3]);
}

#[allow(clippy::too_many_arguments)]
fn rect(x1: i32, x2: i32, y1: i32, y2: i32, a: RenPar, b: RenPar, c: RenPar, d: RenPar) {
    let (a, b, c, d) = (a.0, b.0, c.0, d.0);
    render_water_rectangle(x1, x2, y1, y2, a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3],
        d[0], d[1], d[2], d[3]);
}

/// `CWaterLevel::SplitWaterTriangleAlongXLine(int32 xs, int32 x1, int32 y1, CRenPar p1, int32 x2, int32 y2,
/// CRenPar p2, int32 x3, int32 y3, CRenPar p3)` (0x6ECF00, cdecl; `y2` unused). The entry `sub esp, 0x54;
/// nop; nop; jmp 0x404CA0` loads `x3` in the chunk.
///
/// `sw = xs - x1`, `tw = x2 - x1` (ints). With `x1 == x3` (right angle at the left): `t = sw / tw` (x87,
/// 80 bits), `u = 1 - t` (80 bits for z, its f32 for the rest); `ys = (y1 - y3) * sw / tw + y3` (int
/// division); `Pb = (t*p2.z + u*p3.z, t*p2.b + u32*p3.b, t*p2.s + (f32)(u32*p3.s))`, `Pa = (t*p1.z +
/// u*p3.z, t*p1.b + u32*p3.b, t*p1.s + (f32)(u32*p3.s))`, `Pc = (u32*p1.c + (f32)(t*p2.c))`; then
/// `RenderWaterTriangle(x1, ys, Pa, xs, ys, Pb, x3, y3, p3)`, `RenderWaterRectangle(x1, xs, y1, ys, p1, Pc,
/// Pa, Pb)`, `RenderWaterTriangle(xs, y1, Pc, x2, y1, p2, xs, ys, Pb)`. Otherwise (right angle at the right):
/// `t32 = (f32)(sw / tw)`, `u = 1 - t32` (80 bits); `A = (u*p2.z + t32*p3.z, u*p2.b + t32*p3.b, u*p2.s +
/// (f32)(t32*p3.s))`, `B = (t32*p3.c + (f32)(u*p1.c))` (z, b; s as `(f32)(t32*p3.s) + (f32)(u*p1.s)`), `C =
/// (t32*p2.c + (f32)(u*p1.c))`; `ys = (y3 - y1) * sw / tw + y1`; `RenderWaterTriangle(xs, ys, B, x2, ys, A,
/// x2, y3, p3)`, `RenderWaterRectangle(xs, x2, y1, ys, C, p2, B, A)`, `RenderWaterTriangle(x1, y1, p1, xs, y1,
/// C, xs, ys, B)`.
#[hook(addr = 0x6ECF00, conv = cdecl, name = "CWaterLevel::SplitWaterTriangleAlongXLine",
       expect = "83 EC 54 90 90 E9 96 7D D1 FF",
       unverified = "samp2-water-1", route = "L3-only", hazards = "uninit-locals",
       recs = "boat",
       note = "entry jmps to the protection chunk 0x404CA0; the reader 0x402019 reads the jmp rel32 at 0x6ECF06, past the 5 patched bytes")]
#[allow(clippy::too_many_arguments)]
pub fn split_water_triangle_along_x_line(xs: i32, x1: i32, y1: i32, a0: u32, a1: u32, a2: u32, a3: u32, x2: i32,
    _y2: i32, b0: u32, b1: u32, b2: u32, b3: u32, x3: i32, y3: i32, c0: u32, c1: u32, c2: u32, c3: u32) {
    let p1 = RenPar([a0, a1, a2, a3]);
    let p2 = RenPar([b0, b1, b2, b3]);
    let p3 = RenPar([c0, c1, c2, c3]);
    let sw = xs.wrapping_sub(x1);
    let tw = x2.wrapping_sub(x1);
    let t = xi(sw).fdiv(xi(tw));
    if x1 == x3 {
        let u = x(1.0).fsub(t);
        let u32_ = u.to_f32();
        let ys = y1.wrapping_sub(y3).wrapping_mul(sw).wrapping_div(tw).wrapping_add(y3);
        let up3z = u.fmul_m(p3.f(0));
        let tp2z = t.fmul_m(p2.f(0));
        let up3b = x(u32_).fmul_m(p3.f(1));
        let tp2b = t.fmul_m(p2.f(1));
        let up3s = x(u32_).fmul_m(p3.f(2)).to_f32();
        let tp2s = t.fmul_m(p2.f(2));
        let pb = RenPar::new(tp2z.fadd(up3z).to_f32(), tp2b.fadd(up3b).to_f32(), tp2s.fadd_m(up3s).to_f32());
        let pa = RenPar::new(
            t.fmul_m(p1.f(0)).fadd(up3z).to_f32(),
            t.fmul_m(p1.f(1)).fadd(up3b).to_f32(),
            t.fmul_m(p1.f(2)).fadd_m(up3s).to_f32(),
        );
        tri(x1, ys, pa, xs, ys, pb, x3, y3, p3);
        let pc = RenPar::new(
            x(u32_).fmul_m(p1.f(0)).fadd_m(tp2z.to_f32()).to_f32(),
            x(u32_).fmul_m(p1.f(1)).fadd_m(tp2b.to_f32()).to_f32(),
            x(u32_).fmul_m(p1.f(2)).fadd_m(tp2s.to_f32()).to_f32(),
        );
        rect(x1, xs, y1, ys, p1, pc, pa, pb);
        tri(xs, y1, pc, x2, y1, p2, xs, ys, pb);
    } else {
        let t32 = t.to_f32();
        let u: X87 = x(1.0).fsub_m(t32);
        let tp3z = x(p3.f(0)).fmul_m(t32);
        let tp3b = x(p3.f(1)).fmul_m(t32);
        let tp3s = x(p3.f(2)).fmul_m(t32).to_f32();
        let a = RenPar::new(
            u.fmul_m(p2.f(0)).fadd(tp3z).to_f32(),
            u.fmul_m(p2.f(1)).fadd(tp3b).to_f32(),
            u.fmul_m(p2.f(2)).fadd_m(tp3s).to_f32(),
        );
        let up1z = u.fmul_m(p1.f(0)).to_f32();
        let up1b = u.fmul_m(p1.f(1)).to_f32();
        let up1s = u.fmul_m(p1.f(2)).to_f32();
        let b = RenPar::new(tp3z.fadd_m(up1z).to_f32(), tp3b.fadd_m(up1b).to_f32(), x(tp3s).fadd_m(up1s).to_f32());
        let ys = y3.wrapping_sub(y1).wrapping_mul(sw).wrapping_div(tw).wrapping_add(y1);
        tri(xs, ys, b, x2, ys, a, x2, y3, p3);
        let c = RenPar::new(
            x(p2.f(0)).fmul_m(t32).fadd_m(up1z).to_f32(),
            x(p2.f(1)).fmul_m(t32).fadd_m(up1b).to_f32(),
            x(p2.f(2)).fmul_m(t32).fadd_m(up1s).to_f32(),
        );
        rect(xs, x2, y1, ys, c, p2, b, a);
        tri(x1, y1, p1, xs, y1, c, xs, ys, b);
    }
}

/// `CWaterLevel::SplitWaterTriangleAlongYLine(int32 ys, int32 x1, int32 y1, CRenPar p1, int32 x2, int32 y2,
/// CRenPar p2, int32 x3, int32 y3, CRenPar p3)` (0x6EE5A0, cdecl). `t = f32((ys - y1) / (y3 - y1))` (`fild` /
/// `fidiv`); every mid CRenPar is a call of the lerp 0x6E5280 at `t`, recomputed before each use. Right angle
/// at the left (`x1 == x3`): the cut meets the hypotenuse at `xs = (x1 - x2) * (ys - y1) / (y3 - y1) + x2`;
/// otherwise at `xs = (x2 - x1) * (ys - y1) / (y3 - y1) + x1`. Then the triangle above the cut, the
/// rectangle between (its y order by `y1 < y3`), the triangle below.
///
/// The bytes differ from gta-reversed (which re-derives the cut and calls the X split): with `y1 >= y3` and
/// `x1 != x3` the rectangle's `(x2, ys)` corner is `lerp(p1, p3)`, not `lerp(p2, p3)`. The lerp leaves the
/// flow dword's upper half of its output alone; the three output locals (entry ESP - 0x10, - 0x20, - 0x30)
/// are never written otherwise, so that half is the caller's stack residue (`peek`).
#[hook(addr = 0x6EE5A0, conv = cdecl, name = "CWaterLevel::SplitWaterTriangleAlongYLine", expect = "83 EC 30 8B 4C 24 34",
       peek = 0x30, unverified = "frontier2-c24", route = "L2", hazards = "stack-residue")]
#[allow(clippy::too_many_arguments)]
pub fn split_water_triangle_along_y_line(ys: i32, x1: i32, y1: i32, a0: u32, a1: u32, a2: u32, a3: u32, x2: i32,
    y2: i32, b0: u32, b1: u32, b2: u32, b3: u32, x3: i32, y3: i32, c0: u32, c1: u32, c2: u32, c3: u32) {
    let (p1, p2, p3) = ([a0, a1, a2, a3], [b0, b1, b2, b3], [c0, c1, c2, c3]);
    let dy = ys.wrapping_sub(y1);
    let h = y3.wrapping_sub(y1);
    let t = f32_of(xi(dy).fdiv(xi(h))).to_bits();
    let residue = |off: i32| sa_core::abi::peek_stack(0x6EE5A0, off).unwrap_or(0);
    let lerp = |buf: i32, a: [u32; 4], b: [u32; 4]| RenPar(lerp_into([0, 0, 0, residue(buf + 0xC)], a, b, t));
    let (l10, l20, l30) = (-0x10, -0x20, -0x30);
    let (q1, q2, q3) = (RenPar(p1), RenPar(p2), RenPar(p3));
    if x1 == x3 {
        let xs = x1.wrapping_sub(x2).wrapping_mul(dy).wrapping_div(h).wrapping_add(x2);
        let (r1, r2) = if y1 < y3 { (lerp(l30, p2, p3), lerp(l20, p2, p1)) } else { (lerp(l10, p2, p3), lerp(l20, p2, p1)) };
        tri(xs, y1, r2, x2, y2, q2, xs, ys, r1);
        if y1 < y3 {
            let (r4, r5, r6) = (lerp(l20, p2, p3), lerp(l30, p1, p3), lerp(l10, p2, p1));
            rect(x1, xs, y1, ys, q1, r6, r5, r4);
        } else {
            let (r4, r5, r6) = (lerp(l10, p2, p1), lerp(l20, p2, p3), lerp(l30, p1, p3));
            rect(x1, xs, ys, y1, r6, r5, q1, r4);
        }
        let (r8, r9) = (lerp(l10, p2, p3), lerp(l20, p1, p3));
        tri(x1, ys, r9, xs, ys, r8, x3, y3, q3);
    } else {
        let xs = x2.wrapping_sub(x1).wrapping_mul(dy).wrapping_div(h).wrapping_add(x1);
        let (r1, r2) = (lerp(l10, p1, p3), lerp(l20, p1, p2));
        tri(x1, y1, q1, xs, y1, r2, xs, ys, r1);
        if y1 < y3 {
            let (r4, r5, r6) = (lerp(l10, p2, p3), lerp(l20, p1, p3), lerp(l30, p1, p2));
            rect(xs, x2, y1, ys, r6, q2, r5, r4);
        } else {
            let (r4, r5, r6) = (lerp(l10, p1, p2), lerp(l20, p1, p3), lerp(l30, p1, p3));
            rect(xs, x2, ys, y1, r6, r5, r4, q2);
        }
        let (r8, r9) = (lerp(l10, p2, p3), lerp(l20, p1, p3));
        tri(xs, ys, r9, x2, ys, r8, x3, y3, q3);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_SPLIT_WATER_TRIANGLE_ALONG_X_LINE, HOOK_SPLIT_WATER_TRIANGLE_ALONG_Y_LINE];
