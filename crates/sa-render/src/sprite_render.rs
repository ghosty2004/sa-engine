//! WRITER-RENDER-BIG-2: `CSprite::RenderOneXLUSprite` (0x70D000) and `RenderOneXLUSprite_Triangle` (0x70D320). Ported from the 1.0 US bytes;
//! gta-reversed `Sprite.cpp` as reading aid. x87 arithmetic follows the original's instruction order on
//! the `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, jne, jp, k, rd, x, xi};

orig! {
    /// `RwIm2DRenderPrimitive(type, verts, n)` (0x734e90)
    fn im2d_render_primitive(prim: u32, verts: GPtr<u8>, n: i32) -> i32 = cdecl 0x734e90 => unported;
}

/// `CSprite::maVertices` (4 x `RwIm2DVertex`, 0x1C bytes: x, y, z, rhw, colour, u, v).
const SPRITE_VERTICES: usize = 0xc4_b8e0;
/// `CDraw::ms_fNearClipZ`, `CDraw::ms_fFarClipZ`; `RwEngineInstance`.
const NEAR_CLIP_Z: usize = 0xc3_efa0;
const FAR_CLIP_Z: usize = 0xc3_ef9c;
const RW_ENGINE_INSTANCE: usize = 0xc9_7b24;

/// `CSprite::RenderOneXLUSprite(float x, float y, float z, float w, float h, uint8 r, uint8 g, uint8 b,
/// int16 intensity, float recipZ, uint8 alpha, uint8 flipU, uint8 flipV)` (0x70D000). Corners x `(x-w,
/// x-w, x+w, x+w)`, y `(y-h, y+h, y+h, y-h)`; u `(0, 0, 1, 1)` (flipped: `(1, 1, 0, 0)`), v `(0, 1, 1, 0)`
/// (flipped: `(1, 0, 0, 1)`). Per corner, against the screen (`RsGlobal` width / height as ints): x < 0
/// → `u = x / w * -0.5`, x = 0; then x > width → `u = 1 - (x - width) * 0.5 / w`, x = width; the same for
/// y / v with h and height (the height test is `height < y`, unordered keeps y). z `= ((z - near) *
/// (rwFar - rwNear) * far) / ((far - near) * z) + rwNear` for all four, rhw `recipZ`, colour `alpha <<
/// 24 | ((r * intensity) >> 8) << 16 | ((g * intensity) >> 8) << 8 | (b * intensity) >> 8` (signed
/// shifts, low bytes). `RwIm2DRenderPrimitive(TRIFAN, maVertices, 4)`.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x70d000, conv = cdecl, name = "CSprite::RenderOneXLUSprite", expect = "83 EC 40 D9 44 24 44",
       unverified = "renderbig2-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_one_xlu_sprite(px: f32, py: f32, pz: f32, w: f32, h: f32, r: u8, gg: u8, b: u8, intensity: i16, recip_z: u32, alpha: u8, flip_u: u8, flip_v: u8) {
    let xl = x(px).fsub_m(w).to_f32();
    let xr = x(px).fadd_m(w).to_f32();
    let yt = x(py).fsub_m(h).to_f32();
    let yb = x(py).fadd_m(h).to_f32();
    let mut xs = [xl, xl, xr, xr];
    let mut ys = [yt, yb, yb, yt];
    let mut us: [f32; 4] = if flip_u == 0 { [0.0, 0.0, 1.0, 1.0] } else { [1.0, 1.0, 0.0, 0.0] };
    let mut vs: [f32; 4] = if flip_v == 0 { [0.0, 1.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0, 1.0] };
    let wv = xi(g::<i32>(0xc1_7044).read());
    let hv = xi(g::<i32>(0xc1_7048).read());
    // -0.5 (0x858F40)
    let mhalf = k(0xbf00_0000);
    for i in 0..4 {
        if !jp(x(xs[i]), x(0.0), 0x5) {
            let old = xs[i];
            xs[i] = 0.0;
            us[i] = x(old).fdiv_m(w).fmul_m(mhalf).to_f32();
        }
        if !jne(x(xs[i]), wv, 0x41) {
            us[i] = x(xs[i]).fsub(wv).fmul_m(0.5).fdiv_m(w).fsubr_m(1.0).to_f32();
            xs[i] = wv.to_f32();
        }
        if !jp(x(ys[i]), x(0.0), 0x5) {
            let old = ys[i];
            ys[i] = 0.0;
            vs[i] = x(old).fdiv_m(h).fmul_m(mhalf).to_f32();
        }
        if !jp(hv, x(ys[i]), 0x5) {
            vs[i] = x(ys[i]).fsub(hv).fmul_m(0.5).fdiv_m(h).fsubr_m(1.0).to_f32();
            ys[i] = hv.to_f32();
        }
    }
    let rw = g::<GPtr<u8>>(RW_ENGINE_INSTANCE).read();
    let near = g::<f32>(NEAR_CLIP_Z).read();
    let far = g::<f32>(FAR_CLIP_Z).read();
    let num = x(rd::<f32>(rw, 0x1c)).fsub_m(rd::<f32>(rw, 0x18)).fmul(x(pz).fsub_m(near)).fmul_m(far);
    let den = x(far).fsub_m(near).fmul_m(pz);
    let zz = num.fdiv(den).fadd_m(rd::<f32>(rw, 0x18)).to_f32();
    let i = i32::from(intensity);
    let sc = |c: u8| ((i32::from(c) * i) >> 8) as u8 as u32;
    let col = (u32::from(alpha) << 24) | (sc(r) << 16) | (sc(gg) << 8) | sc(b);
    for j in 0..4 {
        let v = GPtr::<u8>::from_addr(SPRITE_VERTICES + 0x1c * j);
        crate::common::wr::<f32>(v, 0, xs[j]);
        crate::common::wr::<f32>(v, 4, ys[j]);
        crate::common::wr::<f32>(v, 8, zz);
        crate::common::wr::<u32>(v, 0xc, recip_z);
        crate::common::wr::<u32>(v, 0x10, col);
        crate::common::wr::<f32>(v, 0x14, us[j]);
        crate::common::wr::<f32>(v, 0x18, vs[j]);
    }
    im2d_render_primitive(5, GPtr::from_addr(SPRITE_VERTICES), 4);
}

/// `CSprite::RenderOneXLUSprite_Triangle(float x1, y1, x2, y2, x3, y3, float z, uint8 r, uint8 g,
/// uint8 b, int16 intensity, float recipZ, uint8 alpha)` (0x70D320). Below z 2.3 the triangle fades:
/// nothing below z 1.3, else `f = ftol((z - 1.3) * 255)` scales r, g, b and the intensity (`>> 8`,
/// signed). The three corners go to `maVertices` with the XLU sprite's z mapping and `recipZ`; colour
/// `alpha << 24 | ((c * intensity) >> 8)` per channel (the intensity's low word sign-extended);
/// `RwIm2DRenderPrimitive(TRILIST, maVertices, 3)`.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x70d320, conv = cdecl, name = "CSprite::RenderOneXLUSprite_Triangle", expect = "D9 44 24 1C 53 D8 1D 54 8F 85 00",
       unverified = "renderbig2-5", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_one_xlu_sprite_triangle(x1: u32, y1: u32, x2: u32, y2: u32, x3: u32, y3: u32, pz: f32, r: u8, gg: u8, b: u8, intensity: u32, recip_z: u32, alpha: u8) {
    let (mut rr, mut gv, mut bv, mut iv) = (i32::from(r), i32::from(gg), i32::from(b), intensity as i32);
    // 2.3 (0x858F54), 1.3 (0x859918), 255 (0x859AAC)
    if !jp(x(pz), x(k(0x4013_3333)), 0x5) {
        if !jp(x(pz), x(k(0x3fa6_6666)), 0x5) {
            return;
        }
        let f = crate::common::ftol32(x(pz).fsub_m(k(0x3fa6_6666)).fmul_m(k(0x437f_0000)));
        rr = (rr * f) >> 8;
        gv = (gv * f) >> 8;
        bv = (bv * f) >> 8;
        iv = (i32::from(intensity as u16 as i16) * f) >> 8;
    }
    for (i, (vx, vy)) in [(x1, y1), (x2, y2), (x3, y3)].into_iter().enumerate() {
        g::<u32>(SPRITE_VERTICES + 0x1c * i).write(vx);
        g::<u32>(SPRITE_VERTICES + 0x1c * i + 4).write(vy);
    }
    let rw = g::<GPtr<u8>>(RW_ENGINE_INSTANCE).read();
    let near = g::<f32>(NEAR_CLIP_Z).read();
    let far = g::<f32>(FAR_CLIP_Z).read();
    let num = x(rd::<f32>(rw, 0x1c)).fsub_m(rd::<f32>(rw, 0x18)).fmul(x(pz).fsub_m(near)).fmul_m(far);
    let den = x(far).fsub_m(near).fmul_m(pz);
    let zz = num.fdiv(den).fadd_m(rd::<f32>(rw, 0x18)).to_f32();
    let si = i32::from(iv as u16 as i16);
    let sc = |c: i32| (((c & 0xff) * si) >> 8) as u8 as u32;
    let col = (u32::from(alpha) << 24) | (sc(rr) << 16) | (sc(gv) << 8) | sc(bv);
    for i in 0..3 {
        let v = GPtr::<u8>::from_addr(SPRITE_VERTICES + 0x1c * i);
        crate::common::wr::<f32>(v, 8, zz);
        crate::common::wr::<u32>(v, 0xc, recip_z);
        crate::common::wr::<u32>(v, 0x10, col);
    }
    im2d_render_primitive(3, GPtr::from_addr(SPRITE_VERTICES), 3);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_RENDER_ONE_XLU_SPRITE, HOOK_RENDER_ONE_XLU_SPRITE_TRIANGLE];
