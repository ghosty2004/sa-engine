//! WRITER-RENDER-BIG-2: `CPointLights::RenderFogEffect` (0x7002D0). Ported from the 1.0 US bytes;
//! gta-reversed `PointLights.cpp` as reading aid. x87 arithmetic follows the original's instruction order
//! on the `sa_math::x87::X87` model.
//!
//! `CPointLights::aLights` (0xC3F0E0, 0x30 bytes, count `NumLights` at 0xC3F0D0): +0 position, +0xC
//! direction, +0x1C..+0x24 r, g, b (floats), +0x2C type, +0x2D fog type.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, jne, jp, k, rd, x, xi};

orig! {
    /// `RwRenderStateSet(state, value)` (`RwGlobals` +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xc97b24 0x20 => unported;
    /// `CSprite::InitSpriteBuffer()` (0x70cfb0)
    fn init_sprite_buffer() = cdecl 0x70cfb0 => crate::sprite::init_sprite_buffer;
    /// `CSprite::FlushSpriteBuffer()` (0x70cf20; the original's tail jump)
    fn flush_sprite_buffer() = cdecl 0x70cf20 => crate::sprite::flush_sprite_buffer;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool x6, CStoredCollPoly*)` (0x5674e0)
    fn process_vertical_line(origin: GPtr<u8>, distance: f32, cp: GPtr<u8>, out: GPtr<u8>, b: u8, v: u8, p: u8, o: u8, d: u8, see: u8, poly: GPtr<u8>) -> u8 = cdecl 0x5674e0 => sa_world::world::mvpc_world1::process_vertical_line;
    /// `CPointLights::ProcessVerticalLineUsingCache(CVector, float*)` (0x6ffff0)
    fn process_vertical_line_using_cache(px: f32, py: f32, pz: f32, out: GPtr<u32>) -> u8 = cdecl 0x6ffff0 => crate::pointlights::process_vertical_line_using_cache;
    /// `CSprite::CalcScreenCoors(const RwV3d&, RwV3d* out, float* w, float* h, bool, bool)` (0x70ce30)
    fn calc_screen_coors(inp: GPtr<f32>, out: GPtr<f32>, w: GPtr<f32>, h: GPtr<f32>, a: u8, b: u8) -> u8 = cdecl 0x70ce30 => crate::sprite::calc_screen_coors;
    /// `CSprite::RenderBufferedOneXLUSprite_Rotate_Aspect(x, y, z, w, h, r, g, b, intensity, recipZ,
    /// angle, alpha)` (0x70e780; every argument as the dword pushed)
    fn render_buffered_one_xlu_sprite_rotate_aspect(px: u32, py: u32, pz: u32, w: u32, h: u32, r: u32, gg: u32, b: u32, intensity: u32, recip_z: u32, angle: u32, alpha: u32) = cdecl 0x70e780 => unported;
}

const NUM_LIGHTS: usize = 0xc3_f0d0;
const LIGHTS: usize = 0xc3_f0e0;
/// `CCutsceneMgr::ms_running`, `CWeather::Foggyness` (0xC81300), `gpPointLightFogTex` (texture pointer),
/// `CTimer::m_snTimeInMilliseconds`, the 8 fog sprite size factors.
const CUTSCENE_RUNNING: usize = 0xb5_f851;
const FOGGYNESS: usize = 0xc8_1300;
const FOG_TEX: usize = 0xc6_aa78;
const TIME_MS: usize = 0xb7_cb84;
const SIZE_FACTORS: usize = 0x8d_5068;
/// `TheCamera.m_matrix` (pointer) and `TheCamera.m_placement` position.
const CAMERA_MATRIX: usize = 0xb6_f03c;
const CAMERA_PLACEMENT_POS: usize = 0xb6_f02c;

/// `(int16)_ftol(v)`.
fn ftol16(v: X87) -> i32 {
    i32::from(ftol32(v) as i16)
}

/// The `RenderBufferedOneXLUSprite_Rotate_Aspect` tail shared by both fog kinds: `intensity` C, colour
/// `ftol(C * r / g / b)`, `recipZ = 1 / out.z`, size `w * s`, `h * s * hmul` (`s` the size factor).
#[allow(clippy::too_many_arguments)]
fn emit(light: GPtr<u8>, out: &[f32; 3], w: f32, h: f32, c: f32, s: f32, hmul: Option<f32>, rot: f32) {
    let recip = x(1.0).fdiv_m(out[2]).to_f32();
    let inten = ftol32(x(c)) as u32;
    let bb = ftol32(x(c).fmul_m(rd::<f32>(light, 0x24))) as u32;
    let gg = ftol32(x(c).fmul_m(rd::<f32>(light, 0x20))) as u32;
    let rr = ftol32(x(c).fmul_m(rd::<f32>(light, 0x1c))) as u32;
    let mut hh = x(h).fmul_m(s);
    if let Some(m) = hmul {
        hh = hh.fmul_m(m);
    }
    let ww = x(w).fmul_m(s).to_f32();
    render_buffered_one_xlu_sprite_rotate_aspect(
        out[0].to_bits(), out[1].to_bits(), out[2].to_bits(), ww.to_bits(), hh.to_f32().to_bits(),
        rr, gg, bb, inten, recip.to_bits(), rot.to_bits(), 0xff,
    );
}

/// `CPointLights::RenderFogEffect()` (0x7002D0). Nothing in cutscenes. States FOG 0, ZWRITE 0,
/// ZTEST 1, VERTEXALPHA 1, SRCBLEND / DESTBLEND 2, the fog texture's raster; `InitSpriteBuffer`.
///
/// Per light with fog type 1 (strength = `Foggyness`, radius 9; nothing when Foggyness is 0) or 2
/// (strength 0.4, radius 3):
/// - light type 1 (a beam): the box of `pos` and `pos + 12 * dir.xy` grown by 5, as `(int16)ftol`
///   rounded towards zero to multiples of 4 (the far ends + 4); per grid point whose hash `((y >> 2) ^
///   (x >> 2)) & 0xF` is odd: `t = r . dir` (xy) in (0, 12) and `|r|² - t² < 25`; the ground below
///   `(x, y, pos.z + 10)` (`ProcessVerticalLine`, 20 down, buildings and dummies) + 1.3, then with the
///   3D `r` again `t` in (0, 12) and `q = |r|² - t² < 25`; `C = (t / |r|) * strength * 50 * (1 - (t /
///   12)²) * (1 - (0.2 * sqrt(q))²)`; on screen: a sprite of size factor `hash >> 1`, rotation `(ms &
///   0x1FFF) * 6.28/8192`.
/// - light types 0, 3, 4 (with a cached ground z): the box `pos +/- radius` rounded towards zero to
///   multiples of 2 (far ends + 2); per point with an odd hash `((y / 2) ^ (x / 2)) & 0xF` closer than
///   the radius (2D) and within 15 of the camera (2D): fade 1 below 7.5, else `1 - (d - 7.5) / 7.5`;
///   `C = (1 - (d / R)²) * fade * strength * 37` at z = ground + 1.6; rotation `((hash >> 1) * 0x8FC +
///   ms) & 0x7FFF) * 6.28/32768`, height also `* 0.7`.
///
/// Then `FlushSpriteBuffer` (a tail jump in the original).
#[allow(clippy::cognitive_complexity)]
#[hook(addr = 0x7002d0, conv = cdecl, name = "CPointLights::RenderFogEffect", expect = "A0 51 F8 B5 00 81 EC C8 00 00 00",
       route = "L2", hazards = "x87-transcendental",
       recs = "grove-walk,driving",
       note = "FSQRT; ProcessVerticalLine per grid point of beam lights")]
pub fn render_fog_effect() {
    if g::<u8>(CUTSCENE_RUNNING).read() != 0 {
        return;
    }
    for (s, v) in [(14, 0), (8, 0), (6, 1), (12, 1), (10, 2), (11, 2)] {
        rw_render_state_set(s, v);
    }
    rw_render_state_set(1, rd::<u32>(g::<GPtr<u8>>(FOG_TEX).read(), 0));
    init_sprite_buffer();
    let mut n = 0;
    while n < g::<i32>(NUM_LIGHTS).read() {
        let light = GPtr::<u8>::from_addr(LIGHTS + 0x30 * n as usize);
        n += 1;
        let fog = rd::<u8>(light, 0x2d);
        let (strength, radius) = match fog {
            1 => {
                let f = g::<f32>(FOGGYNESS).read();
                if !jp(x(f), x(0.0), 0x44) {
                    continue;
                }
                (f, k(0x4110_0000))
            }
            2 => (k(0x3ecc_cccd), k(0x4040_0000)),
            _ => continue,
        };
        match rd::<u8>(light, 0x2c) {
            1 => beam(light, strength),
            0 | 3 | 4 => ground(light, strength, radius),
            _ => {}
        }
    }
    flush_sprite_buffer();
}

/// The beam-light fog (0x7003AB..0x700851).
fn beam(light: GPtr<u8>, strength: f32) {
    let (px, py, pz) = (rd::<f32>(light, 0), rd::<f32>(light, 4), rd::<f32>(light, 8));
    let (dx, dy, dz) = (rd::<f32>(light, 0xc), rd::<f32>(light, 0x10), rd::<f32>(light, 0x14));
    // 12 (0x858CCC), 5 (0x858C80)
    let twelve = k(0x4140_0000);
    let five = k(0x40a0_0000);
    let dy12 = x(dy).fmul_m(twelve).to_f32();
    let ex = x(dx).fmul_m(twelve).fadd_m(px);
    let ey = x(dy12).fadd_m(py).to_f32();
    let (minx, maxx) = if jp(ex, x(px), 0x5) { (px, ex.to_f32()) } else { (ex.to_f32(), px) };
    let (miny, maxy) = if jp(x(ey), x(py), 0x5) { (x(py), x(ey)) } else { (x(ey), x(py)) };
    let minx = x(minx).fsub_m(five).to_f32();
    let maxx = x(maxx).fadd_m(five).to_f32();
    let miny = miny.fsub_m(five).to_f32();
    let maxy = maxy.fadd_m(five);
    let v = ftol16(x(minx));
    let x0 = v - v % 4;
    let x1 = ftol16(x(maxx)) + 4;
    if x0 > x1 {
        return;
    }
    let v = ftol16(x(miny));
    let y0 = v - v % 4;
    let y1 = ftol16(maxy) + 4;
    let mut gx = x0;
    while gx <= x1 {
        let mut gy = y0;
        while gy <= y1 {
            let hash = ((gy >> 2) ^ (gx >> 2)) & 0xf;
            if hash & 1 != 0 {
                beam_point(light, strength, gx, gy, (hash as i16 >> 1) as usize, (px, py, pz), (dx, dy, dz));
            }
            gy += 4;
        }
        gx += 4;
    }
}

/// One beam grid point (0x70050A..0x70081F).
fn beam_point(light: GPtr<u8>, strength: f32, gx: i32, gy: i32, size: usize, p: (f32, f32, f32), d: (f32, f32, f32)) {
    let twelve = k(0x4140_0000);
    let max_q = k(0x41c8_0000);
    let fx = xi(gx).to_f32();
    let fy = xi(gy).to_f32();
    let rx = x(fx).fsub_m(p.0).to_f32();
    let ry = x(fy).fsub_m(p.1).to_f32();
    let tx = x(rx).fmul_m(d.0).fadd(x(ry).fmul_m(d.1));
    let t = tx.to_f32();
    if jne(tx, x(0.0), 0x41) || jp(x(t), x(twelve), 0x5) {
        return;
    }
    if jp(x(ry).fmul_m(ry).fadd(x(rx).fmul_m(rx)).fsub(x(t).fmul_m(t)), x(max_q), 0x5) {
        return;
    }
    // 10 (0x85862C), 20 (0x858BA4)
    let top = x(p.2).fadd_m(k(0x4120_0000)).to_f32();
    let mut origin = [fx, fy, top];
    let mut cp = [0u32; 11];
    let mut ent = 0u32;
    let depth = x(top).fsub_m(k(0x41a0_0000)).to_f32();
    if process_vertical_line(
        GPtr::<f32>::from_raw(origin.as_mut_ptr()).cast(),
        depth,
        GPtr::<u32>::from_raw(cp.as_mut_ptr()).cast(),
        GPtr::<u32>::from_raw(&mut ent).cast(),
        1, 0, 0, 0, 1, 0,
        GPtr::from_addr(0),
    ) == 0
    {
        return;
    }
    // 1.3 (0x859918)
    let z = x(f32::from_bits(cp[2])).fadd_m(k(0x3fa6_6666)).to_f32();
    let rx = x(fx).fsub_m(p.0).to_f32();
    let ry = x(fy).fsub_m(p.1).to_f32();
    let rzx = x(z).fsub_m(p.2);
    let rz = rzx.to_f32();
    let tx = rzx.fmul_m(d.2).fadd(x(rx).fmul_m(d.0)).fadd(x(ry).fmul_m(d.1));
    let t = tx.to_f32();
    if jne(tx, x(0.0), 0x41) || jp(x(t), x(twelve), 0x5) {
        return;
    }
    let d2 = x(rz).fmul_m(rz).fadd(x(ry).fmul_m(ry)).fadd(x(rx).fmul_m(rx)).to_f32();
    let q = x(t).fmul_m(t).fsubr_m(d2);
    if jp(q, x(max_q), 0x5) {
        return;
    }
    let sq = q.fsqrt();
    // 1/12 (0x85F0FC), 50 (0x858B40), 0.2 (0x858CC4)
    let tk = x(t).fmul_m(k(0x3daa_aaab));
    let a = x(d2).fsqrt().fdivr_m(t).fmul_m(strength).fmul_m(k(0x4248_0000));
    let b = a.fmul(tk.fmul(tk).fsubr_m(1.0));
    let s5 = sq.fmul_m(k(0x3e4c_cccd));
    let s5b = sq.fmul_m(k(0x3e4c_cccd));
    let c = b.fmul(s5b.fmul(s5).fsubr_m(1.0)).to_f32();
    let mut inp = [fx, fy, z];
    let mut out = [0f32; 3];
    let (mut w, mut h) = (0f32, 0f32);
    if calc_screen_coors(GPtr::from_raw(inp.as_mut_ptr()), GPtr::from_raw(out.as_mut_ptr()), GPtr::from_raw(&mut w), GPtr::from_raw(&mut h), 1, 1) == 0 {
        return;
    }
    let ms = g::<u32>(TIME_MS).read() & 0x1fff;
    // 6.28 / 8192 (0x872610, double)
    let rot = xi(ms as i32).fmul(X87::from_f64_exact(f64::from_bits(0x3f49_1eb8_51eb_851f))).to_f32();
    let s = g::<f32>(SIZE_FACTORS + 4 * size).read();
    emit(light, &out, w, h, c, s, None, rot);
}

/// The fog around ground lights (0x700856..0x700BA5).
fn ground(light: GPtr<u8>, strength: f32, radius: f32) {
    let (px, py, pz) = (rd::<f32>(light, 0), rd::<f32>(light, 4), rd::<f32>(light, 8));
    let mut gz = 0u32;
    if process_vertical_line_using_cache(px, py, pz, GPtr::from_raw(&mut gz)) == 0 {
        return;
    }
    let ground_z = f32::from_bits(gz);
    let minx = x(px).fsub_m(radius);
    let maxx = x(radius).fadd_m(px).to_f32();
    let miny = x(py).fsub_m(radius).to_f32();
    let maxy = x(radius).fadd_m(py).to_f32();
    let v = ftol16(minx);
    let x0 = v - v % 2;
    let x1 = ftol16(x(maxx)) + 2;
    if x0 > x1 {
        return;
    }
    let v = ftol16(x(miny));
    let y0 = v - v % 2;
    let y1 = ftol16(x(maxy)) + 2;
    let mut gx = x0;
    while gx <= x1 {
        let mut gy = y0;
        while gy <= y1 {
            let hash = ((gy / 2) ^ (gx / 2)) & 0xf;
            if hash & 1 != 0 {
                ground_point(light, strength, radius, ground_z, gx, gy, (hash as i16 >> 1) as i32);
            }
            gy += 2;
        }
        gx += 2;
    }
}

/// One ground-light grid point (0x70096B..0x700B73).
fn ground_point(light: GPtr<u8>, strength: f32, radius: f32, ground_z: f32, gx: i32, gy: i32, idx: i32) {
    let fyv = xi(gy);
    let ry = fyv.fsub_m(rd::<f32>(light, 4));
    let fx = xi(gx).to_f32();
    let rx = x(fx).fsub_m(rd::<f32>(light, 0));
    let d = rx.fmul(rx).fadd(ry.fmul(ry)).fsqrt();
    let df = d.to_f32();
    if jp(d, x(radius), 0x5) {
        return;
    }
    let m = g::<GPtr<u8>>(CAMERA_MATRIX).read();
    let cp = if m.is_null() { GPtr::<u8>::from_addr(CAMERA_PLACEMENT_POS) } else { m.byte_add(0x30) };
    let cy = rd::<f32>(cp, 4);
    let cx = rd::<f32>(cp, 0);
    let cd = fyv.fsub_m(cy).fmul(fyv.fsub_m(cy)).fadd(x(fx).fsub_m(cx).fmul(x(fx).fsub_m(cx))).fsqrt();
    let cdf = cd.to_f32();
    // 15 (0x858B48), 7.5 (0x87260C), 2/15 (0x872608)
    if jp(cd, x(k(0x4170_0000)), 0x5) {
        return;
    }
    let fade = if !jp(x(cdf), x(k(0x40f0_0000)), 0x5) {
        x(1.0)
    } else {
        x(cdf).fsub_m(k(0x40f0_0000)).fmul_m(k(0x3e08_8889)).fsubr_m(1.0)
    };
    let dr = x(1.0).fdiv_m(radius).fmul_m(df);
    // 37 (0x8652F8), 1.6 (0x858FA8)
    let c = dr.fmul(dr).fsubr_m(1.0).fmul(fade).fmul_m(strength).fmul_m(k(0x4214_0000)).to_f32();
    let z = x(ground_z).fadd_m(k(0x3fcc_cccd)).to_f32();
    let mut inp = [fx, fyv.to_f32(), z];
    let mut out = [0f32; 3];
    let (mut w, mut h) = (0f32, 0f32);
    if calc_screen_coors(GPtr::from_raw(inp.as_mut_ptr()), GPtr::from_raw(out.as_mut_ptr()), GPtr::from_raw(&mut w), GPtr::from_raw(&mut h), 1, 1) == 0 {
        return;
    }
    let t = (idx.wrapping_mul(0x8fc) as u32).wrapping_add(g::<u32>(TIME_MS).read()) & 0x7fff;
    // 6.28 / 32768 (0x872600, double)
    let rot = xi(t as i32).fmul(X87::from_f64_exact(f64::from_bits(0x3f29_1eb8_51eb_851f))).to_f32();
    let s = g::<f32>(SIZE_FACTORS + 4 * idx as usize).read();
    // 0.7 (0x858CB0)
    emit(light, &out, w, h, c, s, Some(k(0x3f33_3333)), rot);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_RENDER_FOG_EFFECT];
