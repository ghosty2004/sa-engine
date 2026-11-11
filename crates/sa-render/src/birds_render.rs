//! WRITER-RENDER-BIG: `CBirds::Render`. Ported from the 1.0 US bytes; gta-reversed `Birds.cpp` as reading
//! aid. x87 arithmetic follows the original's instruction order on the `sa_math::x87::X87` model.
//! Bird layout: see `mvpd_r2` (position +0, angle +0x24, wing period +0x2C, size / wing speed +0x30,
//! max distance +0x34, mode +0x38, colours +0x39..+0x3E, active +0x3F).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, jne, k, rd, wr, x, xi};

orig! {
    /// `CMatrix::SetTranslate(float x, float y, float z)` (0x59AF40); the floats as raw dwords
    fn matrix_set_translate(m: GPtr<u8>, px: u32, py: u32, pz: u32) = thiscall 0x59AF40 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, result through a hidden pointer)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CSprite::CalcScreenCoors(const RwV3d&, RwV3d* out, float* w, float* h, bool, bool)` (0x70CE30)
    fn calc_screen_coors(inp: GPtr<f32>, out: GPtr<f32>, w: GPtr<f32>, h: GPtr<f32>, max_vis: u8, min_vis: u8) -> u8 = cdecl 0x70CE30 => crate::sprite::calc_screen_coors;
    /// `RwRenderStateSet(state, value)` (`RwGlobals` +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CBrightLights::RenderOutGeometryBuffer()` (0x722150)
    fn bright_lights_render_out_geometry_buffer() = cdecl 0x722150 => unported;
}

/// `CBirds::NumberOfBirds`, `CBirds::aBirds` (6 x 0x44).
const NUMBER_OF_BIRDS: usize = 0xC6_A8A4;
const BIRDS: usize = 0xC6_A8B0;
/// `CTimer::m_snTimeInMilliseconds`.
const TIME_MS: usize = 0xB7_CB84;
/// The bird model (`.data`): 8 vertex y offsets, u and v, 30 triangle indices (int32, low word used).
const BIRD_Y: usize = 0x8D_52D8;
const BIRD_U: usize = 0x8D_52B8;
const BIRD_V: usize = 0x8D_5298;
const BIRD_INDICES: usize = 0x8D_52F8;
/// `TempBufferVerticesStored`, `TempBufferIndicesStored`, `aTempBufferIndices`, `TempBufferRenderVertices`.
const TEMP_VERTICES_STORED: usize = 0xC4_B950;
const TEMP_INDICES_STORED: usize = 0xC4_B954;
const TEMP_BUFFER_INDICES: usize = 0xC4_B958;
const TEMP_RENDER_VERTICES: usize = 0xC4_D958;
/// `gpCloudMaskTex` (its raster is the birds' texture).
const BIRD_TEX: usize = 0xC6_AA7C;

/// The vertex model's offset vector for vertex `k` (the original's jump table at 0x712E18): 0, 1, 7 →
/// (0, y, 0) in colour 1; 2, 3 → (0.5 cos a1, y, 0.5 sin a1) colour 1; 4, 5 → the sums with the second
/// wing angle, colour 2; 6 → (0.1, y, 0) colour 1.
fn wing_offset(k: usize, c1: (f32, f32), c2: (f32, f32)) -> (f32, f32, bool) {
    match k {
        2 | 3 => (c1.0, c1.1, false),
        4 | 5 => (x(c2.0).fadd_m(c1.0).to_f32(), x(c2.1).fadd_m(c1.1).to_f32(), true),
        6 => (k32(0x3DCC_CCCD), 0.0, false),
        _ => (0.0, 0.0, false),
    }
}

fn k32(bits: u32) -> f32 {
    k(bits)
}

/// `fild` of a dword as unsigned (`+ 2^32` when negative).
fn fild_u(v: u32) -> X87 {
    let r = xi(v as i32);
    if (v as i32) < 0 { r.fadd_m(k(0x4F80_0000)) } else { r }
}

/// `CBirds::Render()` (0x712810). Nothing without birds. Both temp-buffer counts are zeroed. For each
/// active bird (time offset `100 * index`): a local `CMatrix` (`SetTranslate(position)`, destroyed at the
/// end); modes 1 and 2 bob its z by `sin(((ms + offset) % period) * (6.28 / period)) * 0.1` (unsigned
/// arithmetic). `CalcScreenCoors(pos, .., false, true)` off screen skips the bird. The rotation is the
/// angle's `(c*s, -s*s, 0)`, `(s*s, c*s, 0)`, `(0, 0, s)` with `s` the size (+0x30); other modes stop
/// here. Indices: the 30 model indices + n, then again + n + 8. Wings: `a1 = -sin(((period/6 + offset +
/// ms) % period) * w)` (f32), `a2 = -sin(((offset + ms) % period) * w)`, `w = 6.28 / period`, each
/// divided by `max(1, size)` (1 when it is larger; NaN takes the size). Alpha 255, or beyond `0.7 *
/// max distance` (`sqrt((dx² + dy²) + dz²)` to the camera) `ftol(max(0, 1 - (d - 0.7m) / (0.3m)) *
/// 255)` (0 only when below 0). Eight vertices `M * (x, y[k], z)` (see `wing_offset`) with colour
/// `alpha << 24 | rgb`, model uv, and their mirrors `M * (-x, y[k], z)` eight slots later with `rgb *
/// 0.8`. Counts += 60 indices and 16 vertices. At the end, with indices stored: SRCALPHA /
/// INVSRCALPHA, TEXTUREADDRESS 3, FOG 0, ZTEST 1, the cloud-mask raster, and
/// `CBrightLights::RenderOutGeometryBuffer`.
#[allow(clippy::cognitive_complexity)]
#[hook(addr = 0x712810, conv = cdecl, name = "CBirds::Render", expect = "64 A1 00 00 00 00",
       unverified = "renderbig-3", route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "grove-walk",
       note = "FSIN/FCOS/FSQRT; SEH frame for the local CMatrix (not reproduced: nothing in it throws); writes TempBufferRenderVertices / aTempBufferIndices and both counts")]
pub fn birds_render() {
    if g::<u32>(NUMBER_OF_BIRDS).read() == 0 {
        return;
    }
    g::<u32>(TEMP_INDICES_STORED).write(0);
    g::<u32>(TEMP_VERTICES_STORED).write(0);
    let mut offset: u32 = 0;
    for b in 0..6usize {
        let bird = GPtr::<u8>::from_addr(BIRDS + 0x44 * b);
        let cur = offset;
        offset = offset.wrapping_add(0x64);
        if rd::<u8>(bird, 0x3F) == 0 {
            continue;
        }
        let mut m = [0u32; 18];
        matrix_set_translate(GPtr::<u32>::from_raw(m.as_mut_ptr()).cast(), rd::<u32>(bird, 0), rd::<u32>(bird, 4), rd::<u32>(bird, 8));
        let mode = rd::<u8>(bird, 0x38);
        let period = rd::<u32>(bird, 0x2C);
        let ms = g::<u32>(TIME_MS).read();
        // 6.28 (0x872828), 0.1 (0x858B1C)
        let two_pi = k(0x40C8_F5C3);
        if mode == 1 || mode == 2 {
            let t = ms.wrapping_add(cur) % period;
            let w = fild_u(period).fdivr_m(two_pi);
            m[14] = fild_u(t).fmul(w).fsin().fmul_m(k(0x3DCC_CCCD)).fadd_m(f32::from_bits(m[14])).to_f32().to_bits();
        }
        let mut out = [0u32; 3];
        let mut sw = 0f32;
        let mut sh = 0f32;
        let ok = calc_screen_coors(
            GPtr::<u32>::from_raw(m[12..15].as_mut_ptr()).cast(),
            GPtr::<u32>::from_raw(out.as_mut_ptr()).cast(),
            GPtr::from_raw(&mut sw),
            GPtr::from_raw(&mut sh),
            0,
            1,
        );
        if ok != 0 {
            let angle = rd::<f32>(bird, 0x24);
            let size = rd::<f32>(bird, 0x30);
            let c = x(angle).fcos();
            let s = x(angle).fsin();
            let cs = c.fmul_m(size);
            m[0] = cs.to_f32().to_bits();
            m[1] = s.fmul_m(size).fchs().to_f32().to_bits();
            m[2] = 0;
            m[4] = s.fmul_m(size).to_f32().to_bits();
            m[5] = cs.to_f32().to_bits();
            m[6] = 0;
            m[8] = 0;
            m[9] = 0;
            m[10] = size.to_bits();
            if mode == 1 || mode == 2 {
                render_bird(bird, &mut m, cur, period, size);
            }
        }
        matrix_dtor(GPtr::<u32>::from_raw(m.as_mut_ptr()).cast());
    }
    if g::<u32>(TEMP_INDICES_STORED).read() == 0 {
        return;
    }
    rw_render_state_set(10, 5);
    rw_render_state_set(11, 6);
    rw_render_state_set(2, 3);
    rw_render_state_set(14, 0);
    rw_render_state_set(12, 1);
    rw_render_state_set(1, rd::<u32>(g::<GPtr<u8>>(BIRD_TEX).read(), 0));
    bright_lights_render_out_geometry_buffer();
}

/// The geometry part of one bird (0x712985..0x712D72).
fn render_bird(bird: GPtr<u8>, m: &mut [u32; 18], offset: u32, period: u32, size: f32) {
    let ni = g::<u32>(TEMP_INDICES_STORED).read();
    let nv = g::<u32>(TEMP_VERTICES_STORED).read();
    for j in 0..30usize {
        let base = g::<u16>(BIRD_INDICES + 4 * j).read().wrapping_add(nv as u16);
        g::<u16>(TEMP_BUFFER_INDICES + 2 * (ni as usize + j)).write(base);
        g::<u16>(TEMP_BUFFER_INDICES + 2 * (ni as usize + j) + 0x3C).write(base.wrapping_add(8));
    }
    let two_pi = k(0x40C8_F5C3);
    let w = fild_u(period).fdivr_m(two_pi);
    let ms = g::<u32>(TIME_MS).read();
    let t1 = (period / 6).wrapping_add(offset).wrapping_add(ms) % period;
    let a1 = fild_u(t1).fmul(w).fsin().fchs().to_f32();
    let t2 = offset.wrapping_add(ms) % period;
    let a2 = fild_u(t2).fmul(w).fsin().fchs();
    let lim = || if jne(x(1.0), x(size), 0x41) { x(size) } else { x(1.0) };
    let a1 = x(a1).fdiv(lim()).to_f32();
    let a2 = a2.fdiv(lim());
    // 0.5
    let c1 = (x(a1).fcos().fmul_m(0.5).to_f32(), x(a1).fsin().fmul_m(0.5).to_f32());
    let c2 = (a2.fcos().fmul_m(0.5).to_f32(), a2.fsin().fmul_m(0.5).to_f32());

    let cp = sa_world::world::common::cam_pos().cast::<u8>();
    let dz = x(rd::<f32>(bird, 8)).fsub_m(rd::<f32>(cp, 8));
    let dy = x(rd::<f32>(bird, 4)).fsub_m(rd::<f32>(cp, 4));
    let dx = x(rd::<f32>(bird, 0)).fsub_m(rd::<f32>(cp, 0));
    let d = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz)).fsqrt();
    let maxd = rd::<f32>(bird, 0x34);
    // 0.7 (0x858CB0), 0.3 (0x858C24), 255 (0x859AAC)
    let near = x(maxd).fmul_m(k(0x3F33_3333));
    let mut alpha: u32 = 0xFF;
    if !jne(d, near, 0x41) {
        let f = d.fsub(near).fdiv(x(maxd).fmul_m(k(0x3E99_999A))).fsubr_m(1.0);
        let f = if jne(x(0.0), f, 0x41) { f } else { x(0.0) };
        alpha = ftol32(f.fmul_m(255.0)) as u32;
    }
    let a8 = alpha << 8;
    for kk in 0..8usize {
        let (xo, zo, second) = wing_offset(kk, c1, c2);
        let co = if second { 0x3C } else { 0x39 };
        let (r, gg, bb) = (rd::<u8>(bird, co), rd::<u8>(bird, co + 1), rd::<u8>(bird, co + 2));
        let y = x(g::<f32>(BIRD_Y + 4 * kk).read()).to_f32();
        let mut v = [xo.to_bits(), y.to_bits(), zo.to_bits()];
        let mut res = [0u32; 3];
        let r1 = matrix_mul_vec(GPtr::<u32>::from_raw(res.as_mut_ptr()).cast(), GPtr::<u32>::from_raw(m.as_mut_ptr()).cast(), GPtr::<u32>::from_raw(v.as_mut_ptr()).cast());
        let p1 = [rd::<u32>(r1, 0), rd::<u32>(r1, 4), rd::<u32>(r1, 8)];
        v[0] = x(xo).fchs().to_f32().to_bits();
        let vtx = GPtr::<u8>::from_addr(TEMP_RENDER_VERTICES + 0x24 * (g::<u32>(TEMP_VERTICES_STORED).read() as usize + kk));
        let col = |r: u8, gg: u8, bb: u8| ((((a8 | u32::from(r)) << 8) | u32::from(gg)) << 8) | u32::from(bb);
        wr::<u32>(vtx, 0x18, col(r, gg, bb));
        wr::<u32>(vtx, 0, p1[0]);
        wr::<u32>(vtx, 4, p1[1]);
        wr::<u32>(vtx, 8, p1[2]);
        let u = g::<u32>(BIRD_U + 4 * kk).read();
        let vv = g::<u32>(BIRD_V + 4 * kk).read();
        wr::<u32>(vtx, 0x1C, u);
        wr::<u32>(vtx, 0x20, vv);
        let mut res2 = [0u32; 3];
        let r2 = matrix_mul_vec(GPtr::<u32>::from_raw(res2.as_mut_ptr()).cast(), GPtr::<u32>::from_raw(m.as_mut_ptr()).cast(), GPtr::<u32>::from_raw(v.as_mut_ptr()).cast());
        let p2 = [rd::<u32>(r2, 0), rd::<u32>(r2, 4), rd::<u32>(r2, 8)];
        // 0.8 (0x858C98)
        let s = |c: u8| ftol32(xi(i32::from(c)).fmul_m(k(0x3F4C_CCCD))) as u8;
        let (r8, g8, b8) = (s(r), s(gg), s(bb));
        let mv = vtx.byte_add(8 * 0x24);
        wr::<u32>(mv, 0x18, col(r8, g8, b8));
        wr::<u32>(mv, 0, p2[0]);
        wr::<u32>(mv, 4, p2[1]);
        wr::<u32>(mv, 8, p2[2]);
        wr::<u32>(mv, 0x1C, u);
        wr::<u32>(mv, 0x20, vv);
    }
    g::<u32>(TEMP_INDICES_STORED).write(g::<u32>(TEMP_INDICES_STORED).read().wrapping_add(0x3C));
    g::<u32>(TEMP_VERTICES_STORED).write(g::<u32>(TEMP_VERTICES_STORED).read().wrapping_add(0x10));
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_BIRDS_RENDER];
