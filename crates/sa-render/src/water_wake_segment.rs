//! `CWaterLevel::RenderWakeSegment` (0x6EA260, batch `samp2-water-1`): one boat-wake quad strip into the
//! water render buffer. Ported from the 1.0 US bytes (gta-reversed `WaterLevel.cpp` as a reading aid).
//! Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_world::physical::base::floor_f64;

use crate::common::{ftol32, g, k, x, xi};

orig! {
    /// `Common::LittleTest()` (0x541330)
    fn little_test() = cdecl 0x541330 => unported;
    /// `RwIm3DTransform(RwIm3DVertex*, uint32, RwMatrix*, uint32)` (0x7EF450)
    fn rw_im3d_transform(verts: GPtr<u8>, n: u32, ltm: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(RwPrimitiveType, RwImVertexIndex*, int32)` (0x7EF550)
    fn rw_im3d_render_indexed_primitive(ty: u32, idx: GPtr<u8>, n: i32) -> i32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    fn rw_im3d_end() -> i32 = cdecl 0x7EF520 => unported;
}

/// The water render buffer: `TempBufferVertices` 3D (0xC4D958, `RwIm3DVertex` 0x24: pos +0, colour +0x18, u
/// +0x1C, v +0x20), indices (0xC4B958, u16), counts (0xC4B950 vertices, 0xC4B954 indices).
const VERTS: usize = 0xC4_D958;
const INDICES: usize = 0xC4_B958;
const NVERTS: usize = 0xC4_B950;
const NINDICES: usize = 0xC4_B954;
/// `ALPHA_MULTS` (0.4, 1, 0.2, 1, 0.4 at 0x8D390C), the texture shift (0xC228D4 / 0xC228D8),
/// `CWeather::WindClipped` (0xC812EC).
const ALPHA_MULTS: usize = 0x8D_390C;
const UV_SHIFT_X: usize = 0xC2_28D4;
const UV_SHIFT_Y: usize = 0xC2_28D8;
const WIND_CLIPPED: usize = 0xC8_12EC;

use sa_core::field::gf;

use sa_core::field::rdf as rf;

/// `fstp qword; call floor` (CRT 0x8219F0, exact).
fn floor_x(v: X87) -> X87 {
    X87::from_f64_exact(floor_f64(v.to_f64()))
}

/// `a < b ? a : b` (`fcomp; test ah, 5; jp` keeps b).
fn min2(a: f32, b: f32) -> f32 {
    if !x(a).fcom(x(b)).test_parity(5) { a } else { b }
}

/// The wave height of a corner: `wakeZ + sin(((fy - floor fy) + (fx - floor fx)) * 2pi + angle) *
/// (wind * 0.4 + 0.2)` with `f = (c + 3072) * (1/32)`: `floor` of the 80-bit value (as a double), minus
/// from its f32 store.
fn corner_z(cx: f32, cy: f32, angle: f32, z: f32) -> f32 {
    let fx = x(cx).fadd_m(k(0x4540_0000)).fmul_m(k(0x3D00_0000));
    let fracx = x(fx.to_f32()).fsub(floor_x(fx)).to_f32();
    let fy = x(cy).fadd_m(k(0x4540_0000)).fmul_m(k(0x3D00_0000));
    let fracy = x(fy.to_f32()).fsub(floor_x(fy));
    let s = fracy.fadd_m(fracx).fmul_m(k(0x40C9_0FDB)).fadd_m(angle).fsin();
    let w = x(gf(WIND_CLIPPED)).fmul_m(k(0x3ECC_CCCD)).fadd_m(k(0x3E4C_CCCD));
    s.fmul(w).fadd_m(z).to_f32()
}

/// `CWaterLevel::RenderWakeSegment(CVector2D& a, CVector2D& b, CVector2D& c, CVector2D& d, float& widthA,
/// float& widthB, float& alphaA, float& alphaB, float& wakeZ)` (0x6EA260, cdecl; the widths unused).
///
/// `angle = (float)(time & 0xFFF) * 0.0015339355` (f32). Four parts i = 0..3, each: flush the buffer when it
/// has 4090+ indices or 2044+ vertices (`LittleTest`, `RwIm3DTransform(.., 0, 1)`, an indexed tri-list,
/// `RwIm3DEnd`; counts reset); colours `0xFFFFFF | ftol(alphaA * m[i]) << 24`, `alphaA * m[i+1]`, `m[i+1] *
/// alphaB`, `m[i] * alphaB`; with `t0 = i * 0.25`, `t1 = (i + 1) * 0.25` the corners `a*(1-t0) + b*t0`,
/// `a*(1-t1) + b*t1`, `c*t1 + d*(1-t1)`, `c*t0 + d*(1-t0)` (x87 order kept, `1-t0` stored f32 for the y and
/// c3 terms); uv = corner * 0.25 + shift, minus `floor` of the minimum u (`min(u0, min(u1, min(u2, u3)))`)
/// and v; z per corner from [`corner_z`]; indices `v, v+2, v+1, v, v+3, v+2`. Normals are not written.
#[hook(addr = 0x6EA260, conv = cdecl, name = "CWaterLevel::RenderWakeSegment", expect = "55 8B EC 83 E4 F8",
       unverified = "samp2-water-1", route = "L3-only", hazards = "x87-transcendental",
       recs = "boat", note = "fsin per corner; fills the water render buffer")]
#[allow(clippy::too_many_arguments)]
pub fn render_wake_segment(a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, d: GPtr<u8>, _wa: GPtr<u8>, _wb: GPtr<u8>,
    alpha_a: GPtr<u8>, alpha_b: GPtr<u8>, wake_z: GPtr<u8>) {
    let ms = sa_core::timer::TIME_MS.get() & 0xFFF;
    let angle = xi(ms as i32).fmul_m(k(0x3AC9_0E56)).to_f32();
    for i in 0..4i32 {
        let mut nv = g::<u32>(NVERTS).read();
        if g::<i32>(NINDICES).read() >= 0xFFA || nv as i32 >= 0x7FC {
            if nv != 0 {
                little_test();
                if !rw_im3d_transform(g(VERTS), g::<u32>(NVERTS).read(), GPtr::NULL, 1).is_null() {
                    rw_im3d_render_indexed_primitive(3, g(INDICES), g::<i32>(NINDICES).read());
                    rw_im3d_end();
                }
            }
            g::<u32>(NVERTS).write(0);
            g::<u32>(NINDICES).write(0);
            nv = 0;
        }
        let v = VERTS + nv as usize * 0x24;
        let m = |j: i32| gf(ALPHA_MULTS + j as usize * 4);
        let col = |f: X87| 0x00FF_FFFF | (u32::from(ftol32(f) as u8) << 24);
        g::<u32>(v + 0x18).write(col(x(rf(alpha_a, 0)).fmul_m(m(i))));
        g::<u32>(v + 0x24 + 0x18).write(col(x(rf(alpha_a, 0)).fmul_m(m(i + 1))));
        g::<u32>(v + 0x48 + 0x18).write(col(x(m(i + 1)).fmul_m(rf(alpha_b, 0))));
        g::<u32>(v + 0x6C + 0x18).write(col(x(m(i)).fmul_m(rf(alpha_b, 0))));
        let t0 = xi(i).fmul_m(k(0x3E80_0000));
        let t1 = xi(i + 1).fmul_m(k(0x3E80_0000));
        let u0x = x(1.0).fsub(t0);
        let u0 = u0x.to_f32();
        let c0 = [
            u0x.fmul_m(rf(a, 0)).fadd(t0.fmul_m(rf(b, 0))).to_f32(),
            x(u0).fmul_m(rf(a, 4)).fadd(t0.fmul_m(rf(b, 4))).to_f32(),
        ];
        let u1 = x(1.0).fsub(t1);
        let c1 = [
            u1.fmul_m(rf(a, 0)).fadd(t1.fmul_m(rf(b, 0))).to_f32(),
            u1.fmul_m(rf(a, 4)).fadd(t1.fmul_m(rf(b, 4))).to_f32(),
        ];
        let c2 = [
            t1.fmul_m(rf(c, 0)).fadd(u1.fmul_m(rf(d, 0))).to_f32(),
            u1.fmul_m(rf(d, 4)).fadd(t1.fmul_m(rf(c, 4))).to_f32(),
        ];
        let c3 = [
            t0.fmul_m(rf(c, 0)).fadd(x(u0).fmul_m(rf(d, 0))).to_f32(),
            x(u0).fmul_m(rf(d, 4)).fadd(t0.fmul_m(rf(c, 4))).to_f32(),
        ];
        let corners = [c0, c1, c2, c3];
        let uv = corners.map(|p| {
            [
                x(p[0]).fmul_m(k(0x3E80_0000)).fadd_m(gf(UV_SHIFT_X)).to_f32(),
                x(p[1]).fmul_m(k(0x3E80_0000)).fadd_m(gf(UV_SHIFT_Y)).to_f32(),
            ]
        });
        let min_u = floor_x(x(min2(uv[0][0], min2(uv[1][0], min2(uv[2][0], uv[3][0]))))).to_f32();
        let min_v = floor_x(x(min2(uv[0][1], min2(uv[1][1], min2(uv[2][1], uv[3][1])))));
        for (j, t) in uv.iter().enumerate() {
            let o = v + j * 0x24;
            g::<f32>(o + 0x1C).write(x(t[0]).fsub_m(min_u).to_f32());
            g::<f32>(o + 0x20).write(x(t[1]).fsub(min_v).to_f32());
        }
        for (j, p) in corners.iter().enumerate() {
            let o = v + j * 0x24;
            let z = corner_z(p[0], p[1], angle, rf(wake_z, 0));
            g::<f32>(o).write(p[0]);
            g::<f32>(o + 4).write(p[1]);
            g::<f32>(o + 8).write(z);
        }
        let ni = g::<u32>(NINDICES).read() as usize;
        let base = nv as u16;
        for (j, add) in [0u16, 2, 1, 0, 3, 2].iter().enumerate() {
            g::<u16>(INDICES + (ni + j) * 2).write(base.wrapping_add(*add));
        }
        g::<u32>(NVERTS).write(nv + 4);
        g::<u32>(NINDICES).write(ni as u32 + 6);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_RENDER_WAKE_SEGMENT];
