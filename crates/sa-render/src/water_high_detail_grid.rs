//! `FUN_006E91D0` (batch `samp2-water-1`): the vertex grid of `CWaterLevel::RenderHighDetailWaterRectangle`
//! (0x6EB810, its only caller, twice: layer 0 computes waves and colours, layer 1 reuses them). Unnamed in
//! gta-reversed; ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_world::physical::base::floor_f64;

use crate::common::{ftol32, g, k, x, xi};

orig! {
    /// `RenderBuffer::RenderIfDoesntFit()` (0x6E7680; ECX not set by this caller)
    fn render_if_doesnt_fit(this: u32) = thiscall 0x6E7680 => unported;
    /// `CWaterLevel::CalculateWavesOnlyForCoordinate(x, y, big, small, float* wave, float* colour, float*
    /// glare, CVector* normal)` (0x6E6EF0)
    #[allow(clippy::too_many_arguments)]
    fn calculate_waves_only_for_coordinate(x0: i32, y0: i32, big: f32, small: f32, out: GPtr<f32>, colour: GPtr<f32>, glare: GPtr<f32>, n: GPtr<f32>) = cdecl 0x6E6EF0 => crate::waterlevel::calculate_waves_only_for_coordinate;
}

/// Render buffer (see `water_wake_segment`): vertices 0xC4D958 (0x24 each), indices 0xC4B958, counts
/// 0xC4B950 / 0xC4B954. The per-vertex colour cache (r 0xC20968, g 0xC20168, b 0xC1F968, index 0xC1F960),
/// the water colour (0xC2116C..E), the alphas (0x8D3808 layer 0, 0x8D380C layer 1), the fade radius (int,
/// 0x8D37D0), the no-indices flag (0xC278E0), the normal out (0xC278D4).
const VERTS: usize = 0xC4_D958;
const INDICES: usize = 0xC4_B958;
const NVERTS: usize = 0xC4_B950;
const NINDICES: usize = 0xC4_B954;
const CACHE_R: usize = 0xC2_0968;
const CACHE_G: usize = 0xC2_0168;
const CACHE_B: usize = 0xC1_F968;
const CACHE_N: usize = 0xC1_F960;

fn floor_x(v: X87) -> X87 {
    X87::from_f64_exact(floor_f64(v.to_f64()))
}

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// `FUN_006E91D0(int32 minX, int32 maxX, int32 y1, int32 y2, CRenPar p1, p2, p3, p4, int32 layer, ., ., int32
/// nx, int32 ny)` (cdecl). Corner parameters: p1 at (minX, y1), p2 along x, p3 along y, p4 opposite.
///
/// Clears the colour-cache index and makes room in the buffer. `rx = 1 / nx`, `ry = 1 / ny` (f32); the
/// steps `(p_j - p_k) * r` per channel (f32). Layer 0 / 1: the texture origin `frac(minX * 0.08 + sx)`,
/// `frac(y1 * 0.08 + sy)` (0.04 and the second shift for layer 1; `floor` as a double). For j = 0..=ny
/// (rows) and i = 0..=nx: below the diagonal (`i*rx + (f32)(j*ry) < 1`) the parameters interpolate from p1
/// at `(minX + i*dx, y1 + j*dy)`, otherwise back from p4 with the remaining counts. The camera fade `d =
/// sqrt(dy² + dx²) / R` gives 0 beyond 1, 1 up to 0.75, else `(1 - d) * 4`. Layer 0: `z = p.z`, then
/// `CalculateWavesOnlyForCoordinate(x, y, fade * big, small * fade, &z, &c, &glare, 0xC278D4)`, the vertex
/// (x, y, z), uv `i*dx*0.08 + u0`, colour `alpha0 << 24 | ftol(r*c) << 16 | ftol(g*c) << 8 | ftol(b*c)`
/// cached; layer 1: only uv (0.04) and the cached colour with alpha1 (the position stays from the
/// previous use of that buffer slot). Other layers write nothing and do not advance. Unless 0xC278E0 is
/// set, rows / columns past the first add two triangles and every vertex advances the count.
#[hook(addr = 0x6E91D0, conv = cdecl, name = "FUN_006e91d0", expect = "81 EC C4 00 00 00",
       unverified = "samp2-water-1", route = "L3-only", hazards = "uninit-locals",
       recs = "boat", note = "fills the water render buffer; gta-reversed's RenderHighDetailWaterRectangle helper")]
#[allow(clippy::too_many_arguments)]
pub fn high_detail_water_grid(min_x: i32, max_x: i32, y1: i32, y2: i32,
    p1z: f32, p1b: f32, p1s: f32, _p1f: u32, p2z: f32, p2b: f32, p2s: f32, _p2f: u32,
    p3z: f32, p3b: f32, p3s: f32, _p3f: u32, p4z: f32, p4b: f32, p4s: f32, _p4f: u32,
    layer: i32, _a: u32, _b: u32, nx: i32, ny: i32) {
    g::<u32>(CACHE_N).write(0);
    render_if_doesnt_fit(0);
    let rx = xi(nx).fdivr_m(1.0).to_f32();
    let ry = xi(ny).fdivr_m(1.0).to_f32();
    let step = |a: f32, b: f32, r: f32| x(a).fsub_m(b).fmul_m(r).to_f32();
    let dx = max_x.wrapping_sub(min_x).wrapping_div(nx);
    let dy = y2.wrapping_sub(y1).wrapping_div(ny);
    let ndx = min_x.wrapping_sub(max_x).wrapping_div(nx);
    let ndy = y1.wrapping_sub(y2).wrapping_div(ny);
    let (az, bz) = (step(p2z, p1z, rx), step(p3z, p1z, ry));
    let (ab, bb) = (step(p2b, p1b, rx), step(p3b, p1b, ry));
    let (as_, bs) = (step(p2s, p1s, rx), step(p3s, p1s, ry));
    let (cz, dz) = (step(p3z, p4z, rx), step(p2z, p4z, ry));
    let (cb, db) = (step(p3b, p4b, rx), step(p2b, p4b, ry));
    let (cs, ds) = (step(p3s, p4s, rx), step(p2s, p4s, ry));
    let (mut u0, mut v0) = (0f32, 0f32);
    let tex = match layer {
        0 => Some((k(0x3DA3_D70A), 0xC2_1184usize, 0xC2_1180usize)),
        1 => Some((k(0x3D23_D70A), 0xC2_117C, 0xC2_1178)),
        _ => None,
    };
    if let Some((s, ax, ay)) = tex {
        let a = xi(min_x).fmul_m(s).fadd_m(gf(ax)).to_f32();
        let b = xi(y1).fmul_m(s).fadd_m(gf(ay)).to_f32();
        u0 = x(a).fsub(floor_x(x(a))).to_f32();
        v0 = x(b).fsub(floor_x(x(b))).to_f32();
    }
    let m = g::<u32>(0xB6_F03C).read();
    let cam: [f32; 3] = g::<[f32; 3]>(if m != 0 { m as usize + 0x30 } else { 0xB6_F02C }).read();
    let mut j = 0i32;
    let mut rows_left = ny;
    let mut far_y = ndy.wrapping_mul(ny).wrapping_add(y2);
    let mut off_y = 0i32;
    while j <= ny {
        let fj = xi(j).to_f32();
        let fjr = x(fj).fmul_m(ry).to_f32();
        let mut i = 0i32;
        let mut off_x = 0i32;
        let mut far_x = ndx.wrapping_mul(nx).wrapping_add(max_x);
        let mut cols_left = nx;
        while i <= nx {
            let fi = xi(i).to_f32();
            let (vx, vy, mut z, b, s);
            if !x(fi).fmul_m(rx).fadd_m(fjr).fcom(x(1.0)).test_parity(5) {
                z = x(fj).fmul_m(bz).fadd(x(fi).fmul_m(az)).fadd_m(p1z).to_f32();
                vx = off_x.wrapping_add(min_x);
                vy = y1.wrapping_add(off_y);
                b = x(fj).fmul_m(bb).fadd(x(fi).fmul_m(ab)).fadd_m(p1b);
                s = x(fi).fmul_m(as_).fadd(x(fj).fmul_m(bs)).fadd_m(p1s).to_f32();
            } else {
                let fk = xi(cols_left).to_f32();
                let fl = xi(rows_left).to_f32();
                vy = far_y;
                vx = far_x;
                z = x(fl).fmul_m(dz).fadd(x(fk).fmul_m(cz)).fadd_m(p4z).to_f32();
                b = x(fl).fmul_m(db).fadd(x(fk).fmul_m(cb)).fadd_m(p4b);
                s = x(fl).fmul_m(ds).fadd(x(fk).fmul_m(cs)).fadd_m(p4s).to_f32();
            }
            let xf = xi(vx).to_f32();
            let yf = xi(vy).to_f32();
            let ddx = x(cam[0]).fsub_m(xf).to_f32();
            let ddy = x(cam[1]).fsub_m(yf).to_f32();
            let d = x(ddy).fmul_m(ddy).fadd(x(ddx).fmul_m(ddx)).fsqrt().fdiv(xi(g::<i32>(0x8D_37D0).read()));
            let d32 = d.to_f32();
            let fade = if d.fcom(x(1.0)).test_any(0x41) {
                if x(d32).fcom(x(k(0x3F40_0000))).test_any(0x41) {
                    x(1.0)
                } else {
                    x(1.0).fsub_m(d32).fmul_m(4.0)
                }
            } else {
                x(0.0)
            };
            let cnt = g::<u32>(CACHE_N).read() as usize;
            let mut advance = true;
            match layer {
                0 => {
                    let small = x(s).fmul(fade).to_f32();
                    let big = fade.fmul(b).to_f32();
                    let (mut c, mut glare) = (0f32, 0f32);
                    calculate_waves_only_for_coordinate(vx, vy, big, small, GPtr::from_raw(&mut z),
                        GPtr::from_raw(&mut c), GPtr::from_raw(&mut glare), g(0xC2_78D4));
                    let n = g::<u32>(NVERTS).read() as usize;
                    let v = VERTS + n * 0x24;
                    g::<f32>(v + 0x1C).write(xi(off_x).fmul_m(k(0x3DA3_D70A)).fadd_m(u0).to_f32());
                    g::<f32>(v + 0x20).write(xi(off_y).fmul_m(k(0x3DA3_D70A)).fadd_m(v0).to_f32());
                    g::<f32>(v).write(xf);
                    g::<f32>(v + 4).write(yf);
                    g::<f32>(v + 8).write(z);
                    let ch = |a: usize| ftol32(xi(i32::from(g::<u8>(a).read())).fmul_m(c)) as u8;
                    let r = ch(0xC2_116C);
                    let gg = ch(0xC2_116D);
                    let bb_ = ch(0xC2_116E);
                    let alpha = g::<u32>(0x8D_3808).read();
                    g::<u32>(v + 0x18).write((((alpha << 8) | u32::from(r)) << 8 | u32::from(gg)) << 8 | u32::from(bb_));
                    let cnt = g::<u32>(CACHE_N).read() as usize;
                    g::<u8>(CACHE_G + cnt).write(gg);
                    g::<u8>(CACHE_R + cnt).write(r);
                    g::<u8>(CACHE_B + cnt).write(bb_);
                }
                1 => {
                    let n = g::<u32>(NVERTS).read() as usize;
                    let v = VERTS + n * 0x24;
                    let alpha = g::<u32>(0x8D_380C).read() << 8;
                    g::<f32>(v + 0x1C).write(xi(off_x).fmul_m(k(0x3D23_D70A)).fadd_m(u0).to_f32());
                    g::<f32>(v + 0x20).write(xi(off_y).fmul_m(k(0x3D23_D70A)).fadd_m(v0).to_f32());
                    let col = (((u32::from(g::<u8>(CACHE_R + cnt).read()) | alpha) << 8 | u32::from(g::<u8>(CACHE_G + cnt).read()))
                        << 8)
                        | u32::from(g::<u8>(CACHE_B + cnt).read());
                    g::<u32>(v + 0x18).write(col);
                }
                _ => advance = false,
            }
            if advance {
                let c2 = g::<u32>(CACHE_N).read();
                g::<u32>(CACHE_N).write(c2.wrapping_add(1));
            }
            if g::<u8>(0xC2_78E0).read() == 0 {
                let n = g::<u32>(NVERTS).read() as i32;
                if j != 0 && i != 0 {
                    let ni = g::<u32>(NINDICES).read() as usize;
                    let a = n.wrapping_sub(nx);
                    let w = |o: usize, v: i32| g::<u16>(INDICES + (ni + o) * 2).write(v as u16);
                    w(0, a - 2);
                    w(1, a - 1);
                    w(2, n - 1);
                    w(3, n);
                    w(4, a - 1);
                    w(5, n - 1);
                    g::<u32>(NINDICES).write(ni as u32 + 6);
                }
                g::<u32>(NVERTS).write(n as u32 + 1);
            }
            far_x = far_x.wrapping_sub(ndx);
            off_x = off_x.wrapping_add(dx);
            i += 1;
            cols_left -= 1;
        }
        j += 1;
        rows_left -= 1;
        far_y = far_y.wrapping_sub(ndy);
        off_y = off_y.wrapping_add(dy);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_HIGH_DETAIL_WATER_GRID];
