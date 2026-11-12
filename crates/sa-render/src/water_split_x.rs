//! `CWaterLevel::SplitWaterTriangleAlongXLine` (0x6ECF00, batch `samp2-water-1`). Ported from the 1.0 US
//! bytes (gta-reversed `WaterLevel.cpp` gives the structure; the rounding and the argument order of the
//! six calls follow the bytes). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{x, xi};

orig! {
    /// `CWaterLevel::RenderWaterTriangle(x1, y1, CRenPar p1, x2, y2, p2, x3, y3, p3)` (0x6EE240)
    #[allow(clippy::too_many_arguments)]
    fn render_water_triangle(x1: i32, y1: i32, a0: u32, a1: u32, a2: u32, a3: u32, x2: i32, y2: i32, b0: u32, b1: u32, b2: u32, b3: u32, x3: i32, y3: i32, c0: u32, c1: u32, c2: u32, c3: u32) = cdecl 0x6EE240 => crate::waterlevel::render_water_triangle;
    /// `CWaterLevel::RenderWaterRectangle(minX, maxX, y1, y2, p1, p2, p3, p4)` (0x6EC5D0)
    #[allow(clippy::too_many_arguments)]
    fn render_water_rectangle(x1: i32, x2: i32, y1: i32, y2: i32, a0: u32, a1: u32, a2: u32, a3: u32, b0: u32, b1: u32, b2: u32, b3: u32, c0: u32, c1: u32, c2: u32, c3: u32, d0: u32, d1: u32, d2: u32, d3: u32) = cdecl 0x6EC5D0 => crate::waterlevel::render_water_rectangle;
}

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

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_SPLIT_WATER_TRIANGLE_ALONG_X_LINE];
