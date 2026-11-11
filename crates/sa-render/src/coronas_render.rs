//! WRITER-MVP-G: `CCoronas` render passes (wet-road reflections). Ported from the 1.0 US bytes;
//! gta-reversed `Coronas.cpp` as reading aid. x87 arithmetic follows the original's instruction order
//! on the `sa_math::x87::X87` model.
//!
//! `CCoronas::aCoronas` (0xC3E058, 64 x `CRegisteredCorona`, 0x3C bytes): +0 position, +0xC id, +0x14
//! size, +0x1C far clip, +0x24 height above ground, +0x2C..+0x2F colour r, g, b, a, +0x30 faded
//! intensity, +0x33 reflection flag, +0x36 flags (bit 2: ground found), +0x38 attached entity.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{at, f32_of, ftol32, g, jp, k, rd, wr, x, xi};

orig! {
    /// `CSprite::InitSpriteBuffer()` (0x70CFB0)
    fn init_sprite_buffer() = cdecl 0x70CFB0 => crate::sprite::init_sprite_buffer;
    /// `CSprite::FlushSpriteBuffer()` (0x70CF20)
    fn flush_sprite_buffer() = cdecl 0x70CF20 => crate::sprite::flush_sprite_buffer;
    /// `RwRenderStateSet(state, value)` (`RwGlobals` +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(this: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*) const` (0x54F1B0)
    fn simple_transform_update_matrix(this: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, result through a hidden pointer)
    fn matrix_mul_vec(out: GPtr<f32>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<f32> = cdecl 0x59C890 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool buildings, bool
    /// vehicles, bool peds, bool objects, bool dummies, bool seeThrough, CStoredCollPoly*)` (0x5674E0)
    fn process_vertical_line(p: GPtr<f32>, z: f32, cp: GPtr<u8>, e: GPtr<u32>, b: u32, v: u32, pe: u32, o: u32, d: u32, st: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `CSprite::CalcScreenCoors(const RwV3d&, RwV3d* out, float* w, float* h, bool, bool)` (0x70CE30)
    fn calc_screen_coors(inp: GPtr<f32>, out: GPtr<f32>, w: GPtr<f32>, h: GPtr<f32>, a: u32, b: u32) -> u8 = cdecl 0x70CE30 => unported;
    /// `CSprite::RenderBufferedOneXLUSprite(x, y, z, w, h, r, g, b, intensity, recipNearZ, alpha)` (0x70E4A0)
    fn render_buffered_one_xlu_sprite(px: f32, py: f32, pz: f32, w: f32, h: f32, r: u32, gg: u32, b: u32, intensity: u32, recip_z: u32, alpha: u32) = cdecl 0x70E4A0 => crate::mvpd_r3::render_buffered_one_xlu_sprite;
    /// The sun-reflection fade of a point `(x, y, z, r)` (0x6FBA20, ST0)
    fn sun_reflection_fade(px: f32, py: f32, pz: f32, r: f32) -> f64 = cdecl 0x6FBA20 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<f32>) = thiscall 0x59C910 => unported;
    /// `RwIm3DTransform(verts, count, ltm, flags)` (0x7EF450)
    fn rw_im3d_transform(verts: u32, count: u32, ltm: u32, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(primType, indices, numIndices)` (0x7EF550)
    fn rw_im3d_render_indexed_primitive(prim: u32, indices: u32, n: u32) -> u32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    fn rw_im3d_end() -> u32 = cdecl 0x7EF520 => unported;
}

/// `TempBufferVerticesStored`, `TempBufferIndicesStored`, `aTempBufferIndices`, `TempBufferRenderVertices`.
const TEMP_VERTICES_STORED: usize = 0xC4_B950;
const TEMP_INDICES_STORED: usize = 0xC4_B954;
const TEMP_BUFFER_INDICES: u32 = 0xC4_B958;
const TEMP_RENDER_VERTICES: u32 = 0xC4_D958;
/// `CTimeCycle::m_CurrentStoredValue` and `CTimeCycle::m_VectorToSun[16]`.
const SUN_INDEX: usize = 0xB7_9FD0;
const VECTOR_TO_SUN: usize = 0xB7_CA50;
/// Three `CWeather` factors (`1 - each` scales the reflection; the last also lifts the quad,
/// `* 0.5 + 0.1`).
const WEATHER_A: usize = 0xC8_1304;
const WEATHER_B: usize = 0xC8_1300;
const WEATHER_C: usize = 0xC8_12F0;
/// `CTimer::m_snTimeInMilliseconds`.
const TIME_MS: usize = 0xB7_CB84;

/// `CWeather::WetRoads`.
const WET_ROADS: usize = 0xC8_1308;
/// `CCoronas::aCoronas`.
const CORONAS: usize = 0xC3_E058;
/// `CTimer::m_FrameCounter`.
const FRAME_COUNTER: usize = 0xB7_CB4C;
/// `TheCamera.m_pMat` and the placement position.
const CAM_MATRIX: usize = 0xB6_F03C;
const CAM_PLACEMENT_POS: usize = 0xB6_F02C;
/// `Scene.camera`.
const SCENE_CAMERA: usize = 0xC1_703C;

/// `CCoronas::RenderReflections()` (0x6FB630). With `WetRoads <= 0` (ordered) only clears flag bit 2
/// of every corona. Otherwise: `InitSpriteBuffer`, states (14,0) (8,0) (6,0) (12,1) (10,2) (11,2) and
/// the first corona texture's raster; for each corona with an id, a faded intensity or alpha and the
/// reflection flag (+0x33): its position (through the attached entity's matrix, allocated and
/// updated if missing); the ground below (`ProcessVerticalLine` 1000 down, buildings and dummies)
/// once (sets bit 2) and then every 16th frame (`(frame + index) & 15 == 0`), storing
/// `z - ground.z` in +0x24. With the ground known, a height below 20 and the point `z - height` not
/// above the camera, the mirrored point `z - 2 * height` is projected (`CalcScreenCoors`, both clip
/// checks); the screen depth must be below `min(farClip * 0.75, 55)` (the comparisons as the bytes:
/// NaN keeps the far clip), the fade is 1 within half that depth, else `1 - (z - d/2) / (d/2)`
/// clamped to 0..1; intensity `(int16) ftol(WetRoads * fade * (20 - height) * 230 * 0.05)` scales the
/// colour (`c * intensity >> 8`, signed), drawn with `RenderBufferedOneXLUSprite(x, y, nearZ (RwGlobals
/// +0x18), w * size * 0.75, h * size * 2, r, g, b, 128, 1 / Scene.camera->farPlane, 255)`. Then
/// `FlushSpriteBuffer` and states (10,5) (11,6) (12,0) (8,1) (6,1).
#[hook(addr = 0x6FB630, conv = cdecl, name = "CCoronas::RenderReflections", expect = "D9 05 08 13 C8 00 83 EC 6C",
       unverified = "mvpg-render-1", route = "L2", hazards = "heap-writes",
       recs = "grove-walk",
       note = "writes the coronas' flags / heights (0xC3E058..) and the sprite buffer; allocates attached entities' matrices")]
pub fn render_reflections() {
    let wet = || x(g::<f32>(WET_ROADS).read());
    if !jp(wet(), x(0.0), 0x41) {
        for i in 0..64usize {
            let f = g::<u8>(CORONAS + 0x36 + 0x3C * i);
            f.write(f.read() & 0xFB);
        }
        return;
    }
    init_sprite_buffer();
    rw_render_state_set(0xE, 0);
    rw_render_state_set(8, 0);
    rw_render_state_set(6, 0);
    rw_render_state_set(0xC, 1);
    rw_render_state_set(0xA, 2);
    rw_render_state_set(0xB, 2);
    let tex = g::<GPtr<u8>>(0xC3_E00C).read();
    rw_render_state_set(1, rd::<u32>(tex, 0));
    let mut col_point = [0u8; 0x2C]; // [esp+0x50]
    let mut ent: u32 = 0; // [esp+0x14]
    let mut tmp = [0f32; 3]; // [esp+0x44]
    let mut screen = [0f32; 3]; // [esp+0x2c]
    let mut sw = 0f32; // [esp+0x1c]
    let mut sh = 0f32; // [esp+0x18]
    for i in 0..64u32 {
        let c = GPtr::<u8>::from_addr(CORONAS + 0x3C * i as usize);
        if rd::<u32>(c, 0xC) == 0 {
            continue;
        }
        if rd::<u8>(c, 0x30) == 0 && rd::<u8>(c, 0x2F) == 0 {
            continue;
        }
        if rd::<u8>(c, 0x33) == 0 {
            continue;
        }
        let e = rd::<GPtr<u8>>(c, 0x38);
        // [esp+0x20]
        let mut pos = if !e.is_null() {
            if rd::<u32>(e, 0x14) == 0 {
                placeable_allocate_matrix(e);
                simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
            }
            let r = matrix_mul_vec(GPtr::from_raw(tmp.as_mut_ptr()), rd::<GPtr<u8>>(e, 0x14), c);
            [r.read(), r.wrapping_index(1).read(), r.wrapping_index(2).read()]
        } else {
            [rd::<f32>(c, 0), rd::<f32>(c, 4), rd::<f32>(c, 8)]
        };
        let line = |pos: &mut [f32; 3], cp: &mut [u8; 0x2C], ent: &mut u32| {
            process_vertical_line(
                GPtr::from_raw(pos.as_mut_ptr()),
                -1000.0,
                GPtr::from_raw(cp.as_mut_ptr()),
                GPtr::from_raw(ent as *mut u32),
                1,
                0,
                0,
                0,
                1,
                0,
                0,
            ) != 0
        };
        let store = |pos: &[f32; 3], cp: &[u8; 0x2C]| {
            let gz = f32::from_le_bytes([cp[8], cp[9], cp[10], cp[11]]);
            wr::<f32>(c, 0x24, x(pos[2]).fsub_m(gz).to_f32());
        };
        if rd::<u8>(c, 0x36) & 4 != 0 {
            if g::<u32>(FRAME_COUNTER).read().wrapping_add(i) & 0xF == 0 && line(&mut pos, &mut col_point, &mut ent) {
                store(&pos, &col_point);
            }
        } else if line(&mut pos, &mut col_point, &mut ent) {
            wr::<u8>(c, 0x36, rd::<u8>(c, 0x36) | 4);
            store(&pos, &col_point);
        }
        if rd::<u8>(c, 0x36) & 4 == 0 {
            continue;
        }
        let height = rd::<f32>(c, 0x24); // [esp+0x10]
        if jp(x(height), x(20.0), 5) {
            continue;
        }
        let cm = g::<GPtr<u8>>(CAM_MATRIX).read();
        let cp: GPtr<u8> = if cm.is_null() { GPtr::from_addr(CAM_PLACEMENT_POS) } else { cm.byte_add(0x30) };
        if jp(x(pos[2]).fsub_m(height), x(rd::<f32>(cp, 8)), 0x41) {
            continue;
        }
        let mut refl = [pos[0], pos[1], x(height).fadd(x(height)).fsubr_m(pos[2]).to_f32()];
        if calc_screen_coors(
            GPtr::from_raw(refl.as_mut_ptr()),
            GPtr::from_raw(screen.as_mut_ptr()),
            GPtr::from_raw(&mut sw as *mut f32),
            GPtr::from_raw(&mut sh as *mut f32),
            1,
            1,
        ) == 0
        {
            continue;
        }
        let mut d = x(rd::<f32>(c, 0x1C)).fmul_m(0.75);
        if !jp(x(55.0), d, 5) {
            d = x(55.0);
        }
        let z = x(screen[2]);
        if jp(z, d, 5) {
            continue;
        }
        let hd = d.fmul_m(0.5);
        let fade = if !jp(z, hd, 5) {
            x(1.0)
        } else {
            let f = z.fsub(hd).fdiv(hd).fsubr_m(1.0);
            if !jp(x(1.0), f, 5) {
                x(1.0)
            } else if !x(0.0).fcom(f).test_any(0x41) {
                x(0.0)
            } else {
                f
            }
        };
        let v = wet().fmul(fade).fmul(x(20.0).fsub_m(height)).fmul_m(230.0).fmul_m(k(0x3D4C_CCCD));
        let intensity = ftol32(v) as i16 as i32;
        let recip = x(rd::<f32>(g::<GPtr<u8>>(SCENE_CAMERA).read(), 0x80)).fdivr_m(1.0);
        let sz = rd::<f32>(c, 0x14);
        let hh = f32_of(x(sh).fmul_m(sz).fadd(x(sh).fmul_m(sz)));
        let ww = f32_of(x(sw).fmul_m(sz).fmul_m(0.75));
        let near = rd::<f32>(GPtr::<u8>::from_addr(g::<u32>(0xC9_7B24).read() as usize), 0x18);
        let ch = |o: isize| ((i32::from(rd::<u8>(c, o)) * intensity) >> 8) as u32;
        render_buffered_one_xlu_sprite(
            screen[0],
            screen[1],
            near,
            ww,
            hh,
            ch(0x2C),
            ch(0x2D),
            ch(0x2E),
            0x80,
            recip.to_f32().to_bits(),
            0xFF,
        );
    }
    flush_sprite_buffer();
    rw_render_state_set(0xA, 5);
    rw_render_state_set(0xB, 6);
    rw_render_state_set(0xC, 0);
    rw_render_state_set(8, 1);
    rw_render_state_set(6, 1);
}

fn tv(n: u32, off: u32) -> GPtr<u32> {
    GPtr::from_addr((TEMP_RENDER_VERTICES + 0x24 * n + off) as usize)
}

fn ti(n: u32) -> GPtr<u16> {
    GPtr::from_addr((TEMP_BUFFER_INDICES + 2 * n) as usize)
}

fn fbits(v: X87) -> u32 {
    v.to_f32().to_bits()
}

/// `CCoronas::RenderSunReflection()` (0x6FBAA0): the sun's reflection on the sea. Only with the sun
/// vector's z above -0.05; the intensity `(0.3 - |z - 0.25|) * 3.333 * (1 - [0xC81304]) * (1 -
/// [0xC81300]) * (1 - [0xC812F0])` must be positive. It is faded by the five fixed points
/// (611, 875) r 250, (-929, 2364) 250, (-1034, 2640) 250, (2372, -1854) 350, (-1633, 106) 250 (0x6FBA20;
/// each product stored as f32 before the next factor), times 0.25; the colour is the time cycle's
/// sun core + corona colour words (0xB7C4D0.. + 0xB7C4D6..) times that (ftol each), alpha 255. The
/// quad sits 40 sun-vectors from the camera at height `[0xC812F0] * 0.5 + 0.1`, 30 to each side of
/// the normalised (x, y) direction and 60 out (UVs (0,1) (1,1) (0,0.5) (1,0.5), indices 2 1 0 2 3 1);
/// then 20 vertex pairs further out, at `i / 20 + 60` along the direction and `j / 20 + 30 + sin((time
/// + 900 k) & 0x7FF) * pi/1024) * 10` across (i, j stepping by 1440 and 970), each joined to the
/// previous pair by two triangles. Drawn with states (8,0) (6,1) (14,0) (16,1) (10,2) (11,2) (12,1)
/// and the second corona texture, then (8,1) (6,1) (10,5) (11,6) (14,0) (12,0) and both temp-buffer
/// counts zeroed.
#[hook(addr = 0x6FBAA0, conv = cdecl, name = "CCoronas::RenderSunReflection", expect = "A1 D0 9F B7 00 83 EC 60",
       unverified = "mvpg-render-1", route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "air-water",
       note = "fsin for the strip wobble; writes TempBufferRenderVertices / aTempBufferIndices and zeroes the counts")]
pub fn render_sun_reflection() {
    let si = g::<u32>(SUN_INDEX).read().wrapping_mul(0xC) as usize;
    let sun = |c: usize| g::<f32>(VECTOR_TO_SUN + si + 4 * c).read();
    if x(sun(2)).fcom(x(k(0xBD4C_CCCD))).test_any(0x41) {
        return;
    }
    let mut d = x(sun(2)).fsub_m(0.25);
    if !jp(d, x(0.0), 5) {
        d = d.fchs();
    }
    let one_minus = |a: usize| x(1.0).fsub_m(g::<f32>(a).read());
    let inten = d
        .fsubr_m(k(0x3E99_999A))
        .fmul_m(k(0x4055_4FDF))
        .fmul(one_minus(WEATHER_A))
        .fmul(one_minus(WEATHER_B))
        .fmul(one_minus(WEATHER_C));
    if inten.fcom(x(0.0)).test_any(0x41) {
        return;
    }
    let mut acc = inten.to_f32(); // [esp+0x24]
    let pts: [(f32, f32, f32); 5] =
        [(611.0, 875.0, 250.0), (-929.0, 2364.0, 250.0), (-1034.0, 2640.0, 250.0), (2372.0, -1854.0, 350.0), (-1633.0, 106.0, 250.0)];
    let mut f = x(0.0);
    for (n, (qx, qy, r)) in pts.iter().enumerate() {
        if n > 0 {
            acc = f.to_f32();
        }
        f = X87::from_f64_exact(sun_reflection_fade(*qx, *qy, 0.0, *r)).fmul_m(acc);
    }
    let a = f.fmul_m(0.25);
    let ch = |w0: usize, w1: usize| {
        let sum = i32::from(g::<u16>(w0).read()) + i32::from(g::<u16>(w1).read());
        ftol32(xi(sum).fmul(a)) as u8
    };
    let red = ch(0xB7_C4D0, 0xB7_C4D6);
    let green = ch(0xB7_C4D2, 0xB7_C4D8);
    let blue = ch(0xB7_C4D4, 0xB7_C4DA);
    let colour = u32::from_be_bytes([0xFF, red, green, blue]);

    let cm = g::<GPtr<u8>>(CAM_MATRIX).read();
    let cp: GPtr<u8> = if cm.is_null() { GPtr::from_addr(CAM_PLACEMENT_POS) } else { cm.byte_add(0x30) };
    let sx = x(sun(0)).fmul_m(40.0);
    let sy = x(sun(1)).fmul_m(40.0);
    let px = sx.fadd_m(rd::<f32>(cp, 0)).to_f32(); // [esp+0x28]
    let py = sy.fadd_m(rd::<f32>(cp, 4)).to_f32(); // [esp+0x2c]
    let mut dir = [sun(0), sun(1), 0.0]; // [esp+0x34]
    let zb = fbits(x(g::<f32>(WEATHER_C).read()).fmul_m(0.5).fadd_m(k(0x3DCC_CCCD))); // [esp+0x30]
    vector_normalise(GPtr::from_raw(dir.as_mut_ptr()));
    let (dx, dy) = (dir[0], dir[1]);
    let a30 = x(dy).fmul_m(30.0);
    let b30 = x(dx).fmul_m(30.0);
    let c60 = x(dx).fmul_m(60.0);
    let d60 = x(dy).fmul_m(60.0);
    let d60f = d60.to_f32(); // [esp+0x18]
    let quad = [
        (x(px).fadd(a30), x(py).fsub(b30)),
        (x(px).fsub(a30), x(py).fadd(b30)),
        (c60.fadd(a30).fadd_m(px), d60.fadd_m(py).fsub(b30)),
        (c60.fadd_m(px).fsub(a30), x(d60f).fadd(b30).fadd_m(py)),
    ];
    let uvs = [(0u32, 0x3F80_0000u32), (0x3F80_0000, 0x3F80_0000), (0, 0x3F00_0000), (0x3F80_0000, 0x3F00_0000)];
    for n in 0..4u32 {
        let (vx, vy) = quad[n as usize];
        tv(n, 0).write(fbits(vx));
        tv(n, 4).write(fbits(vy));
        tv(n, 8).write(zb);
        tv(n, 0x18).write(colour);
        tv(n, 0x1C).write(uvs[n as usize].0);
        tv(n, 0x20).write(uvs[n as usize].1);
    }
    for (n, v) in [2u16, 1, 0, 2, 3, 1].iter().enumerate() {
        ti(n as u32).write(*v);
    }
    let mut time = g::<u32>(TIME_MS).read(); // [esp+0x14]
    let mut j: i32 = 0; // [esp+0x18]
    let mut i: i32 = 0; // ebx
    let mut ni: u32 = 6; // esi
    let mut nv: u32 = 4; // edi
    while i < 0x7080 {
        let inner = xi(i / 20 + 60);
        let wob = xi((time & 0x7FF) as i32).fmul_m(k(0x3B49_0FDB)).fsin();
        let outer = wob.fmul_m(10.0).fadd(xi(j / 20 + 30));
        tv(nv, 0x18).write(colour);
        for (o, v) in [nv, nv - 1, nv - 2, nv, nv + 1, nv - 1].iter().enumerate() {
            ti(ni + o as u32).write(*v as u16);
        }
        let e = inner.fmul_m(dx).to_f32(); // [esp+0x1c]
        let ff = x(dy).fmul(outer);
        let ffs = ff.to_f32(); // [esp+0x20]
        tv(nv, 0).write(fbits(ff.fadd_m(e).fadd_m(px)));
        let gg = inner.fmul_m(dy).to_f32(); // [esp+0x24]
        let h = outer.fmul_m(dx);
        tv(nv, 4).write(fbits(x(gg).fadd_m(py).fsub(h)));
        tv(nv, 8).write(zb);
        tv(nv + 1, 0x18).write(colour);
        tv(nv + 1, 0).write(fbits(x(e).fadd_m(px).fsub_m(ffs)));
        tv(nv + 1, 4).write(fbits(h.fadd_m(gg).fadd_m(py)));
        tv(nv + 1, 8).write(zb);
        tv(nv, 0x1C).write(0);
        tv(nv, 0x20).write(0x3F00_0000);
        tv(nv + 1, 0x1C).write(0x3F80_0000);
        tv(nv + 1, 0x20).write(0x3F00_0000);
        i += 0x5A0;
        ni += 6;
        nv += 2;
        time = time.wrapping_add(0x384);
        j += 0x3CA;
    }
    g::<u32>(TEMP_INDICES_STORED).write(ni);
    g::<u32>(TEMP_VERTICES_STORED).write(nv);
    rw_render_state_set(8, 0);
    rw_render_state_set(6, 1);
    rw_render_state_set(0xE, 0);
    rw_render_state_set(0x10, 1);
    rw_render_state_set(0xA, 2);
    rw_render_state_set(0xB, 2);
    rw_render_state_set(0xC, 1);
    let tex = g::<GPtr<u8>>(0xC3_E010).read();
    rw_render_state_set(1, rd::<u32>(tex, 0));
    if !rw_im3d_transform(TEMP_RENDER_VERTICES, g::<u32>(TEMP_VERTICES_STORED).read(), 0, 1).is_null() {
        rw_im3d_render_indexed_primitive(3, TEMP_BUFFER_INDICES, g::<u32>(TEMP_INDICES_STORED).read());
        rw_im3d_end();
    }
    rw_render_state_set(8, 1);
    rw_render_state_set(6, 1);
    rw_render_state_set(0xA, 5);
    rw_render_state_set(0xB, 6);
    rw_render_state_set(0xE, 0);
    rw_render_state_set(0xC, 0);
    g::<u32>(TEMP_INDICES_STORED).write(0);
    g::<u32>(TEMP_VERTICES_STORED).write(0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_RENDER_REFLECTIONS, HOOK_RENDER_SUN_REFLECTION, HOOK_CORONAS_RENDER];

// ---- WRITER-RENDER-BIG: CCoronas::Render ----

orig! {
    /// `CSprite::CalcScreenCoors(const RwV3d&, RwV3d* out, float* w, float* h, bool, bool)` (0x70CE30)
    fn cr_calc_screen_coors(inp: GPtr<f32>, out: GPtr<f32>, w: GPtr<f32>, h: GPtr<f32>, max_vis: u8, min_vis: u8) -> u8 = cdecl 0x70CE30 => crate::sprite::calc_screen_coors;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn cr_allocate_matrix(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x54F560 => sa_world::world::entity::allocate_matrix;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*) const` (0x54F1B0)
    fn cr_update_matrix(this: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => crate::mvpd_unk::simple_transform_update_matrix;
    /// `CVector::Normalise()` (0x59C910)
    fn cr_normalise(v: GPtr<sa_math::vector::CVector>) = thiscall 0x59C910 => sa_world::collision::data::normalise_port;
    /// `CSprite::RenderOneXLUSprite_Rotate_Aspect(x, y, z, w, h, r, g, b, intensity, recipZ, angle, alpha)` (0x70D490)
    fn cr_render_one_xlu_sprite_rotate_aspect(px: f32, py: f32, pz: f32, w: f32, h: f32, r: u32, gg: u32, b: u32, intensity: u32, recip_z: u32, angle: f32, alpha: u32) = cdecl 0x70D490 => crate::mvpd_r5::render_one_xlu_sprite_rotate_aspect;
    /// `CWorld::ProcessLineOfSight(origin, target, colPoint, entity, buildings, vehicles, peds, objects,
    /// dummies, seeThrough, cameraIgnore, shootThrough)` (0x56BA00)
    fn cr_process_line_of_sight(o: GPtr<u8>, t: GPtr<u8>, cp: GPtr<u8>, out: GPtr<u8>, b: u8, v: u8, p: u8, ob: u8, d: u8, see: u8, cam: u8, shoot: u8) -> u8 = cdecl 0x56BA00 => sa_world::world::mvpc_world1::process_line_of_sight;
    /// `CSprite::RenderBufferedOneXLUSprite2D(x, y, w, h, const RwRGBA&, int16 intensity, uint8 alpha)` (0x70F440)
    fn cr_render_buffered_one_xlu_sprite_2d(px: f32, py: f32, w: f32, h: f32, colour: GPtr<u8>, intensity: u32, alpha: u32) = cdecl 0x70F440 => crate::mvpd_r3::render_buffered_one_xlu_sprite_2d;
}

/// `CWeather::Foggyness`, `CWeather::HeadLightsSpectrum`, `CGame::currArea`.
const CR_FOGGYNESS: usize = 0xC8_1300;
const HEADLIGHTS_SPECTRUM: usize = 0xC8_12D0;
const CR_CURR_AREA: usize = 0xB7_2914;
/// `SunFlareDef` / `HeadLightsFlareDef` (`.data`, 0x14-byte records: position, size, colour multipliers
/// r/g/b and intensity multiplier as int16, sprite int16 at +0x10; a zero sprite ends the list).
const SUN_FLARE_DEF: usize = 0x8D_4B68;
const HEADLIGHTS_FLARE_DEF: usize = 0x8D_4D88;
/// `gpCoronaTexture[0]` (RwTexture*).
const CR_STAR_TEX: usize = 0xC3_E000;

fn cr_rs(state: u32, value: u32) {
    rw_render_state_set(state, value);
}

fn cr_rand_at(site: u32) -> i32 {
    sa_core::orig::at_site(site, sa_core::crt::rand)
}

/// `CCoronas::Render()` (0x6FAEC0). Half the camera raster's size (signed `/ 2`) is kept; ZWRITE 0,
/// ZTEST 1, SRCBLEND/DESTBLEND ONE, VERTEXALPHA 1; a local z-test state starts at 1. For each of the 64
/// coronas with an id and a faded intensity or alpha:
///
/// - The position: an attached entity transforms the offset (`operator*(CMatrix, CVector)`) by the
///   matrix at +0x5CC for a vehicle (type 2) of subclass 9, else by its own matrix (allocated and
///   updated from the placement when missing); without one the position is copied raw.
/// - `CalcScreenCoors(.., true, true)` failing sets the offscreen flag (bit 1 of +0x34). On screen the
///   flag is cleared and set again when x < 0, y < 0, W < x or H < y (strict, NaN passes). A zero faded
///   intensity, or `z >= far clip` (or NaN), ends here. `rz = 1 / z` (f32); intensity `ftol(faded *
///   f)` with `f = 1` below `far / 2`, else `1 - (z - far/2) / (far/2)` (kept as the whole dword).
/// - The z-test state follows the check-obstacles bit (bit 0 off and state 0 → ZTEST 1; bit 0 on and
///   state 1 → ZTEST 0).
/// - With a texture: `scale = min(z, 40) * Foggyness * 0.025 + 1` (z when below 40, else 40; NaN takes
///   40) (f32); id 1 sets out.z to `Scene.camera->farPlane * 0.95`; ZTEST 1 (the local state is not
///   updated), its raster; the position pulled towards the camera by the near clip (`P - normalise(P -
///   cam) * near`, x with the unrounded product, y and z with the stored ones), projected again into
///   the same out/w/h slots and drawn by `RenderOneXLUSprite_Rotate_Aspect(out, w * size, (scale *
///   size) * h, ftol(r / scale), .., intensity, rz * 20, 0, 255)`.
/// - With a flare type (1 sun, 2 headlights; another value keeps the previous corona's table, the
///   first time the original's uninitialised ESI): ZTEST 0, the star raster, `rand()` (0x6FB31E)
///   gives `m = ((rand * (1/32767)) * 0.3 + 0.7) * faded * (1/65536)` (f32); for each flare sprite whose
///   line of sight from the corona to the camera is clear (`ProcessLineOfSight(.., vehicles, peds,
///   shootThrough)`), a buffered 2D sprite at `(out - half) * position + half` (x and y each against the
///   integer half sizes), size `def.size * 4`, colour `ftol((colour * def.mult) * m)` alpha 255. Then ZTEST
///   1; headlights with a non-zero `HeadLightsSpectrum` (NaN passes) in the outside area draw for each
///   headlight sprite a red one at position + 0.05 and a blue one at position - 0.05, colour
///   `ftol((c * mult) * spectrum * (1/256))`, intensity `(def.intensity * (int16)intensity) >> 8`.
///
/// Finally the sprite buffer is flushed (tail call); the render states are not restored.
#[allow(clippy::cognitive_complexity)]
#[hook(addr = 0x6FAEC0, conv = cdecl, name = "CCoronas::Render", expect = "81 EC B8 00 00 00",
       unverified = "renderbig-2", route = "L2", hazards = "heap-writes,uninit-locals",
       recs = "grove-walk",
       note = "rand at 0x6FB31E; writes the corona flag bytes (+0x34), the attached entities' matrices (AllocateMatrix) and the sprite buffer; the flare table pointer (ESI) starts as stack residue and survives across coronas")]
pub fn coronas_render() {
    let cam = g::<GPtr<u8>>(SCENE_CAMERA).read();
    let raster = rd::<GPtr<u8>>(cam, 0x60);
    let width = rd::<i32>(raster, 0xC);
    let height = rd::<i32>(raster, 0x10);
    let half_w = width / 2;
    let half_h = height / 2;
    cr_rs(8, 0);
    cr_rs(12, 1);
    cr_rs(10, 2);
    cr_rs(11, 2);
    cr_rs(6, 1);
    let mut ztest: u8 = 1;
    // ESI: the flare table; uninitialised in the original until a sun / headlight corona is reached.
    let mut table: usize = 0;
    let f = f32::from_bits;
    let mut out = [0u32; 3];
    let mut w = 0f32;
    let mut h = 0f32;
    for n in 0..64usize {
        let c = GPtr::<u8>::from_addr(CORONAS + 0x3C * n);
        if rd::<u32>(c, 0xC) == 0 {
            continue;
        }
        if rd::<u8>(c, 0x30) == 0 && rd::<u8>(c, 0x2F) == 0 {
            continue;
        }
        let mut p: [u32; 3];
        let ent = rd::<GPtr<u8>>(c, 0x38);
        if ent.is_null() {
            p = [rd::<u32>(c, 0), rd::<u32>(c, 4), rd::<u32>(c, 8)];
        } else {
            let m = if rd::<u8>(ent, 0x36) & 7 == 2 && rd::<u32>(ent, 0x594) == 9 {
                ent.byte_add(0x5CC)
            } else {
                if rd::<u32>(ent, 0x14) == 0 {
                    cr_allocate_matrix(ent);
                    cr_update_matrix(ent.byte_add(4), rd::<GPtr<u8>>(ent, 0x14));
                }
                rd::<GPtr<u8>>(ent, 0x14)
            };
            let mut tmp = [0u32; 3];
            let r = matrix_mul_vec(GPtr::<u32>::from_raw(tmp.as_mut_ptr()).cast(), m, c);
            p = [rd::<u32>(r.cast(), 0), rd::<u32>(r.cast(), 4), rd::<u32>(r.cast(), 8)];
        }
        let ok = cr_calc_screen_coors(
            GPtr::<u32>::from_raw(p.as_mut_ptr()).cast(),
            GPtr::<u32>::from_raw(out.as_mut_ptr()).cast(),
            GPtr::from_raw(&mut w),
            GPtr::from_raw(&mut h),
            1,
            1,
        );
        let flags = at::<u8>(c, 0x34);
        if ok == 0 {
            flags.write(flags.read() | 2);
            continue;
        }
        let fl = flags.read() & 0xFD;
        flags.write(fl);
        let zero = x(0.0);
        if !jp(x(f(out[0])), zero, 5)
            || !jp(x(f(out[1])), zero, 5)
            || !jp(xi(width), x(f(out[0])), 5)
            || !jp(xi(height), x(f(out[1])), 5)
        {
            flags.write(fl | 2);
        }
        let faded = rd::<u8>(c, 0x30);
        if faded == 0 {
            continue;
        }
        let far = rd::<f32>(c, 0x1C);
        if jp(x(f(out[2])), x(far), 5) {
            continue;
        }
        let rz = x(1.0).fdiv_m(f(out[2])).to_f32();
        let half = x(far).fmul_m(0.5);
        let fac = if jp(x(f(out[2])), half, 5) { x(f(out[2])).fsub(half).fdiv(half).fsubr_m(1.0) } else { x(1.0) };
        let intensity = ftol32(xi(i32::from(faded)).fmul(fac)) as u32;
        if rd::<u8>(c, 0x34) & 1 == 0 {
            if ztest == 0 {
                cr_rs(6, 1);
                ztest = 1;
            }
        } else if ztest != 0 {
            cr_rs(6, 0);
            ztest = 0;
        }

        let tex = rd::<GPtr<u8>>(c, 0x10);
        if !tex.is_null() {
            // 40 (0x858A10), 0.025 (0x859038)
            let zz = x(f(out[2]));
            let lim = if jp(zz, x(40.0), 5) { x(40.0) } else { zz };
            let scale = lim.fmul_m(g::<f32>(CR_FOGGYNESS).read()).fmul_m(k(0x3CCC_CCCD)).fadd_m(1.0).to_f32();
            if rd::<u32>(c, 0xC) == 1 {
                // 0.95 (0x858EF0)
                out[2] = fbits(x(rd::<f32>(cam, 0x84)).fmul_m(k(0x3F73_3333)));
            }
            cr_rs(6, 1);
            cr_rs(1, rd::<u32>(tex, 0));
            let q = p;
            let cp = sa_world::world::common::cam_pos().cast::<u8>();
            let mut d = [
                fbits(x(f(p[0])).fsub_m(rd::<f32>(cp, 0))),
                fbits(x(f(p[1])).fsub_m(rd::<f32>(cp, 4))),
                fbits(x(f(p[2])).fsub_m(rd::<f32>(cp, 8))),
            ];
            cr_normalise(GPtr::<u32>::from_raw(d.as_mut_ptr()).cast());
            let near = x(rd::<f32>(c, 0x20));
            let dxn = x(f(d[0])).fmul(near);
            let dyn_ = x(f(d[1])).fmul(near).to_f32();
            let dzn = x(f(d[2])).fmul(near).to_f32();
            let mut q2 = [
                fbits(x(f(q[0])).fsub(dxn)),
                fbits(x(f(q[1])).fsub_m(dyn_)),
                fbits(x(f(q[2])).fsub_m(dzn)),
            ];
            let ok = cr_calc_screen_coors(
                GPtr::<u32>::from_raw(q2.as_mut_ptr()).cast(),
                GPtr::<u32>::from_raw(out.as_mut_ptr()).cast(),
                GPtr::from_raw(&mut w),
                GPtr::from_raw(&mut h),
                1,
                1,
            );
            if ok != 0 {
                let inv = x(1.0).fdiv_m(scale);
                // 20 (0x858BA4)
                let rz20 = fbits(x(rz).fmul_m(20.0));
                let cb = ftol32(xi(i32::from(rd::<u8>(c, 0x2E))).fmul(inv)) as u32;
                let cg = ftol32(xi(i32::from(rd::<u8>(c, 0x2D))).fmul(inv)) as u32;
                let cr = ftol32(xi(i32::from(rd::<u8>(c, 0x2C))).fmul(inv)) as u32;
                let size = rd::<f32>(c, 0x14);
                let sh = x(scale).fmul_m(size).fmul_m(h).to_f32();
                let sw = x(w).fmul_m(size).to_f32();
                cr_render_one_xlu_sprite_rotate_aspect(f(out[0]), f(out[1]), f(out[2]), sw, sh, cr, cg, cb, intensity, rz20, 0.0, 0xFF);
            }
        }

        let flare = rd::<u8>(c, 0x32);
        if flare == 0 {
            continue;
        }
        if flare == 1 {
            table = SUN_FLARE_DEF;
        } else if flare == 2 {
            table = HEADLIGHTS_FLARE_DEF;
        }
        cr_rs(6, 0);
        let r = cr_rand_at(0x6F_B31E);
        // 1/32767 (0x858C7C), 0.3 (0x858C24), 0.7 (0x858CB0), 1/65536 (0x859D5C)
        let mult = xi(r)
            .fmul_m(k(0x3800_0100))
            .fmul_m(k(0x3E99_999A))
            .fadd_m(k(0x3F33_3333))
            .fmul(xi(i32::from(rd::<u8>(c, 0x30))))
            .fmul_m(k(0x3780_0000))
            .to_f32();
        cr_rs(1, rd::<u32>(g::<GPtr<u8>>(CR_STAR_TEX).read(), 0));
        let mut def = table;
        while g::<i16>(def + 0x10).read() != 0 {
            let ch = |o: usize, m: usize| {
                let v = i32::from(rd::<u8>(c, o as isize)).wrapping_mul(i32::from(g::<i16>(def + m).read()));
                ftol32(xi(v).fmul_m(mult)) as u8
            };
            let mut colour = [ch(0x2C, 8), ch(0x2D, 0xA), ch(0x2E, 0xC), 0xFF];
            let mut cp_out = [0u8; 0x2C];
            let mut hit_ent = 0u32;
            let campos = sa_world::world::common::cam_pos().cast::<u8>();
            let hit = cr_process_line_of_sight(
                GPtr::<u32>::from_raw(p.as_mut_ptr()).cast(),
                campos,
                GPtr::from_raw(cp_out.as_mut_ptr()),
                GPtr::<u32>::from_raw(&mut hit_ent).cast(),
                0, 1, 1, 0, 0, 0, 0, 1,
            );
            if hit == 0 {
                // 4 (0x858B90)
                let s4 = x(g::<f32>(def + 4).read()).fmul_m(4.0).to_f32();
                let pos = g::<f32>(def).read();
                let hh = xi(half_h);
                let hw = xi(half_w);
                let py = x(f(out[1])).fsub(hh).fmul_m(pos).fadd(hh).to_f32();
                let px = x(f(out[0])).fsub(hw).fmul_m(pos).fadd(hw).to_f32();
                cr_render_buffered_one_xlu_sprite_2d(px, py, s4, s4, GPtr::from_raw(colour.as_mut_ptr()), 0xFF, 0xFF);
            }
            def += 0x14;
        }
        cr_rs(6, 1);
        if flare != 2 {
            continue;
        }
        if !jp(x(g::<f32>(HEADLIGHTS_SPECTRUM).read()), x(0.0), 0x44) {
            continue;
        }
        if g::<u32>(CR_CURR_AREA).read() != 0 {
            continue;
        }
        let mut def = HEADLIGHTS_FLARE_DEF;
        if g::<i16>(def + 0x10).read() == 0 {
            continue;
        }
        let fh = xi(half_h).to_f32();
        let fw = xi(half_w).to_f32();
        let i16i = i32::from(intensity as u16 as i16);
        while g::<i16>(def + 0x10).read() != 0 {
            let spec = || g::<f32>(HEADLIGHTS_SPECTRUM).read();
            let pos = g::<f32>(def).read();
            let size = g::<f32>(def + 4).read();
            let si = (i32::from(g::<i16>(def + 0xE).read()).wrapping_mul(i16i) >> 8) as u32;
            let vr = i32::from(rd::<u8>(c, 0x2C)).wrapping_mul(i32::from(g::<i16>(def + 8).read()));
            // 1/256 (0x859AA0), 0.05 (0x858C28)
            let red = ftol32(xi(vr).fmul_m(spec()).fmul_m(k(0x3B80_0000))) as u8;
            let p1 = x(pos).fadd_m(k(0x3D4C_CCCD));
            let mut col = [red, 0, 0, 0xFF];
            let py = x(f(out[1])).fsub_m(fh).fmul(p1).fadd_m(fh).to_f32();
            let px = x(f(out[0])).fsub_m(fw).fmul(p1).fadd_m(fw).to_f32();
            cr_render_buffered_one_xlu_sprite_2d(px, py, size, size, GPtr::from_raw(col.as_mut_ptr()), si, 0xFF);
            let vb = i32::from(rd::<u8>(c, 0x2E)).wrapping_mul(i32::from(g::<i16>(def + 0xC).read()));
            let blue = ftol32(xi(vb).fmul_m(spec()).fmul_m(k(0x3B80_0000))) as u8;
            let p2 = x(pos).fsub_m(k(0x3D4C_CCCD));
            let mut col = [0, 0, blue, 0xFF];
            let py = x(f(out[1])).fsub_m(fh).fmul(p2).fadd_m(fh).to_f32();
            let px = x(f(out[0])).fsub_m(fw).fmul(p2).fadd_m(fw).to_f32();
            cr_render_buffered_one_xlu_sprite_2d(px, py, size, size, GPtr::from_raw(col.as_mut_ptr()), si, 0xFF);
            def += 0x14;
        }
    }
    flush_sprite_buffer();
}
