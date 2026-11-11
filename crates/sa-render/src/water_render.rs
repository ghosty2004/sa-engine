//! WRITER-MVP-G: `CWaterLevel` render passes (boat wakes). Ported from the 1.0 US bytes; gta-reversed
//! `WaterLevel.cpp` as reading aid. x87 arithmetic follows the original's instruction order on the
//! `sa_math::x87::X87` model; values the original keeps on the x87 stack across iterations stay `X87`.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, jne, jp, k, rd, x, xi};

orig! {
    /// `CBoat::FillBoatList()` (0x6F2710)
    fn fill_boat_list() = cdecl 0x6F2710 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CWaterLevel::RenderWakeSegment(CVector2D& a, b, c, d, float& sizeA, float& sizeB, float& alphaA,
    /// float& alphaB, float& wakeZ)` (0x6EA260)
    fn render_wake_segment(a: GPtr<f32>, b: GPtr<f32>, c: GPtr<f32>, d: GPtr<f32>, s0: GPtr<f32>, s1: GPtr<f32>, a0: GPtr<f32>, a1: GPtr<f32>, z: GPtr<f32>) = cdecl 0x6EA260 => unported;
    /// `Common::LittleTest()` (0x541330)
    fn little_test() = cdecl 0x541330 => sa_core::common::little_test;
    /// `RwRenderStateSet(state, value)` (`RwGlobals` +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwIm3DTransform(verts, count, ltm, flags)` (0x7EF450)
    fn rw_im3d_transform(verts: u32, count: u32, ltm: u32, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(primType, indices, numIndices)` (0x7EF550)
    fn rw_im3d_render_indexed_primitive(prim: u32, indices: u32, n: u32) -> u32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    fn rw_im3d_end() -> u32 = cdecl 0x7EF520 => unported;
}

/// `CBoat::apFrameWakeGeneratingBoats[4]`.
const WAKE_BOATS: usize = 0xC2_7994;
/// The wake lifetime (`.data` f32, 150.0 in the image).
const WAKE_LIFETIME: usize = 0x8D_3940;
/// `TempBufferVerticesStored`, `TempBufferIndicesStored`, `aTempBufferIndices`, `TempBufferRenderVertices`.
const TEMP_VERTICES_STORED: usize = 0xC4_B950;
const TEMP_INDICES_STORED: usize = 0xC4_B954;
const TEMP_BUFFER_INDICES: u32 = 0xC4_B958;
const TEMP_RENDER_VERTICES: u32 = 0xC4_D958;

/// The wake width at a point: `(lifetime - age) * (intensity * 0.04 + 0.5) / lifetime * half + half`
/// (x87, left on the stack by the original).
fn wake_width(age: f32, intensity: f32, half: f32) -> X87 {
    let t = x(g::<f32>(WAKE_LIFETIME).read()).fsub_m(age);
    let q = x(intensity).fmul_m(k(0x3D23_D70A)).fadd_m(0.5).fdiv_m(g::<f32>(WAKE_LIFETIME).read());
    t.fmul(q).fmul_m(half).fadd_m(half)
}

/// `CWaterLevel::RenderBoatWakes()` (0x6ED9A0). Sets the texture raster (state 1) to the dword at
/// 0xC228B8, `CBoat::FillBoatList`, then for up to four wake-generating boats (stops at the first
/// NULL): the half width `colModel.max.x * 0.65` (`* 0.4` for model 460), the boat's forward x/y as the
/// first side direction, the first width from point 0 (age +0x748, intensity +0x7C8), a first alpha of
/// `intensity * 0.01 * 0.0` (sic) and the camera distance (2D, from `TheCamera`'s matrix or placement)
/// fading alphas beyond 50 by `(80 - d) / 30`. For each wake segment i = 1..count (+0x644, u16, re-read;
/// points CVector2D at +0x648): the segment direction (normalised when its squared length exceeds 9,
/// a segment longer than 13 is not drawn), the new width and the quad corners (previous point ± its
/// side direction * previous width, this point ± direction-normal * new width), the alpha
/// `(1 - i / count) * 160` (`* i / 3` for i < 3) `* (intensity * 0.01 + 0.15)` (distance-faded), and
/// `RenderWakeSegment(a, b, c, d, &(-0.03), &newWidth, &prevAlpha, &newAlpha, &(-0.03))`. Finally the
/// buffered triangles are flushed (`LittleTest`, `RwIm3DTransform(.., NULL, 1)`, indexed triangle
/// list, end) and both buffer counts zeroed.
///
/// Kept from the bytes: the first width stays unrounded on the x87 stack, later ones are reloaded from
/// their f32 slot (after `RenderWakeSegment`, which takes it by reference); the first distance test
/// uses the unrounded distance, the per-segment ones its f32; the `i < 3` test uses the 16-bit
/// counter's flags from before the x87 block.
#[hook(addr = 0x6ED9A0, conv = cdecl, name = "CWaterLevel::RenderBoatWakes", expect = "55 8B EC 83 E4 F8 83 EC 58",
       route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "air-water",
       note = "fsqrt for the camera distance and segment length; RenderWakeSegment fills TempBufferRenderVertices / aTempBufferIndices; zeroes the two counts")]
pub fn render_boat_wakes() {
    rw_render_state_set(1, g::<u32>(0xC2_28B8).read());
    fill_boat_list();
    let mut minus003 = k(0xBCF5_C28F); // [esp+0x34]
    for b in 0..4usize {
        let boat = g::<GPtr<u8>>(WAKE_BOATS + 4 * b).read();
        if boat.is_null() {
            break;
        }
        let m = rd::<GPtr<u8>>(boat, 0x14);
        let mut dir_x = rd::<f32>(m, 0x10); // [esp+0x40]
        let mut dir_y = rd::<f32>(m, 0x14); // [esp+0x44]
        let col = get_col_model(boat);
        let mut half = x(rd::<f32>(col, 0xC)).fmul_m(k(0x3F26_6666)).to_f32(); // [esp+0x10]
        if rd::<u16>(boat, 0x22) == 0x1CC {
            half = x(half).fmul_m(k(0x3ECC_CCCD)).to_f32();
        }
        let i0 = xi(i32::from(rd::<u8>(boat, 0x7C8))).to_f32();
        let mut width = wake_width(rd::<f32>(boat, 0x748), i0, half); // st0
        let mut alpha = x(i0).fmul_m(k(0x3C23_D70A)).fmul_m(0.0).to_f32(); // [esp+0x20]
        let cm = g::<GPtr<u8>>(0xB6_F03C).read();
        let cp: GPtr<u8> = if cm.is_null() { GPtr::from_addr(0xB6_F02C) } else { cm.byte_add(0x30) };
        let bm = rd::<GPtr<u8>>(boat, 0x14);
        let bp: GPtr<u8> = if bm.is_null() { boat.byte_add(4) } else { bm.byte_add(0x30) };
        let dx = x(rd::<f32>(bp, 0)).fsub_m(rd::<f32>(cp, 0));
        let dy = x(rd::<f32>(bp, 4)).fsub_m(rd::<f32>(cp, 4));
        let dist_x = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
        let dist = dist_x.to_f32(); // [esp+0x14]
        let far = !dist_x.fcom(x(50.0)).test_any(0x41);
        if far {
            alpha = x(80.0).fsub_m(dist).fmul_m(k(0x3D08_8889)).fmul_m(alpha).to_f32();
        }
        let mut count = u32::from(rd::<u16>(boat, 0x644)); // [esp+0x28]
        if count as i32 > 1 {
            let mut i: u32 = 1; // ecx / edi (16-bit)
            loop {
                let pa = boat.byte_add(0x640 + 8 * i as isize); // previous point (x, y)
                let pb = boat.byte_add(0x648 + 8 * i as isize); // this point
                let mut sx = x(rd::<f32>(pa, 0)).fsub_m(rd::<f32>(pb, 0)).to_f32(); // [esp+0x38]
                let syx = x(rd::<f32>(pa, 4)).fsub_m(rd::<f32>(pb, 4));
                let mut sy = syx.to_f32(); // [esp+0x3c]
                let l2 = syx.fmul_m(sy).fadd(x(sx).fmul_m(sx));
                let mut draw = true;
                if !l2.fcom(x(9.0)).test_any(0x41) {
                    let l = l2.fsqrt();
                    let inv = X87::one().fdiv(l);
                    let invf = inv.to_f32(); // [esp+0x1c]
                    let nx = inv.fmul_m(sx);
                    let ny = x(invf).fmul_m(sy);
                    sx = nx.to_f32();
                    sy = ny.to_f32();
                    if !l.fcom(x(13.0)).test_any(0x41) {
                        draw = false;
                    }
                }
                let inten = xi(i32::from(rd::<u8>(boat, 0x7C8 + i as isize))).to_f32(); // [esp+0x24]
                let new_width = wake_width(rd::<f32>(boat, 0x748 + 4 * i as isize), inten, half).to_f32(); // [esp+0x18]
                let p0x = rd::<f32>(pa, 0);
                let p0y = rd::<f32>(pa, 4);
                let wy = x(dir_y).fmul(width).to_f32(); // [esp+0x1c]
                let mut a = [x(p0x).fsub_m(wy).to_f32(), 0f32]; // [esp+0x60]
                let wx = width.fmul_m(dir_x);
                a[1] = wx.fadd_m(p0y).to_f32();
                let mut bq = [x(wy).fadd_m(p0x).to_f32(), x(p0y).fsub(wx).to_f32()]; // [esp+0x58]
                let ny = x(new_width).fmul_m(sy);
                let mut c = [ny.fadd_m(rd::<f32>(pb, 0)).to_f32(), 0f32]; // [esp+0x50]
                let nx = x(new_width).fmul_m(sx);
                c[1] = x(rd::<f32>(pb, 4)).fsub(nx).to_f32();
                let mut d = [x(rd::<f32>(pb, 0)).fsub(ny).to_f32(), nx.fadd_m(rd::<f32>(pb, 4)).to_f32()]; // [esp+0x48]
                let fi = xi(i as i32).to_f32();
                let mut fade = xi(count as i32).fdivr_m(fi).fsubr_m(1.0).fmul_m(160.0);
                if (i as u16 as i16) < 3 {
                    fade = fade.fmul_m(fi).fmul_m(k(0x3EAA_AAAB));
                }
                let mut new_alpha = x(inten).fmul_m(k(0x3C23_D70A)).fadd_m(k(0x3E19_999A)).fmul(fade).to_f32(); // [esp+0x1c]
                if !x(dist).fcom(x(50.0)).test_any(0x41) {
                    new_alpha = x(80.0).fsub_m(dist).fmul_m(k(0x3D08_8889)).fmul_m(new_alpha).to_f32();
                }
                let mut nw = new_width;
                if draw {
                    render_wake_segment(
                        GPtr::from_raw(a.as_mut_ptr()),
                        GPtr::from_raw(bq.as_mut_ptr()),
                        GPtr::from_raw(c.as_mut_ptr()),
                        GPtr::from_raw(d.as_mut_ptr()),
                        GPtr::from_raw(&mut minus003 as *mut f32),
                        GPtr::from_raw(&mut nw as *mut f32),
                        GPtr::from_raw(&mut alpha as *mut f32),
                        GPtr::from_raw(&mut new_alpha as *mut f32),
                        GPtr::from_raw(&mut minus003 as *mut f32),
                    );
                }
                dir_x = sx;
                width = x(nw);
                alpha = new_alpha;
                dir_y = sy;
                count = u32::from(rd::<u16>(boat, 0x644));
                i = (i + 1) as u16 as i16 as i32 as u32;
                if (i as i32) >= count as i32 {
                    break;
                }
            }
        }
    }
    if g::<u32>(TEMP_VERTICES_STORED).read() != 0 {
        little_test();
        let n = g::<u32>(TEMP_VERTICES_STORED).read();
        if !rw_im3d_transform(TEMP_RENDER_VERTICES, n, 0, 1).is_null() {
            rw_im3d_render_indexed_primitive(3, TEMP_BUFFER_INDICES, g::<u32>(TEMP_INDICES_STORED).read());
            rw_im3d_end();
        }
    }
    g::<u32>(TEMP_VERTICES_STORED).write(0);
    g::<u32>(TEMP_INDICES_STORED).write(0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_RENDER_BOAT_WAKES, HOOK_RENDER_WATER];

// ---- WRITER-RENDER-BIG: CWaterLevel::RenderWater ----

orig! {
    /// `CGame::CanSeeWaterFromCurrArea()` (0x53C4B0, bool in AL)
    fn rw_can_see_water() -> u32 = cdecl 0x53C4B0 => sa_core::game::can_see_water_from_curr_area;
    /// `CWaterLevel::SetCameraRange()` (0x6E9C80)
    fn rw_set_camera_range() = cdecl 0x6E9C80 => crate::waterlevel::set_camera_range;
    /// `App::DefinedState()` (0x734650)
    fn rw_defined_state() = cdecl 0x734650 => sa_core::input::defined_state;
    /// The detailed sea-bed segment of a block (0x6E6A10)
    fn rw_detailed_segment(bx: i32, by: i32, fx0: f32, fx1: f32, fy0: f32, fy1: f32) = cdecl 0x6E6A10 => crate::waterlevel::render_detailed_sea_bed_segment;
    /// The sea-bed segment of a block (0x6E6870)
    fn rw_segment(bx: i32, by: i32, fx0: f32, fx1: f32, fy0: f32, fy1: f32) = cdecl 0x6E6870 => crate::waterlevel::render_sea_bed_segment;
    /// `CWaterLevel::RenderWaterTriangle(x1, y1, CRenPar, x2, y2, CRenPar, x3, y3, CRenPar)` (0x6EE240)
    fn rw_triangle(x1: i32, y1: i32, p1z: u32, p1b: u32, p1s: u32, p1f: u32, x2: i32, y2: i32, p2z: u32, p2b: u32, p2s: u32, p2f: u32, x3: i32, y3: i32, p3z: u32, p3b: u32, p3s: u32, p3f: u32) = cdecl 0x6EE240 => crate::waterlevel::render_water_triangle;
    /// `CWaterLevel::RenderWaterRectangle(x1, x2, y1, y2, CRenPar x4)` (0x6EC5D0)
    fn rw_rectangle(x1: i32, x2: i32, y1: i32, y2: i32, p1z: u32, p1b: u32, p1s: u32, p1f: u32, p2z: u32, p2b: u32, p2s: u32, p2f: u32, p3z: u32, p3b: u32, p3s: u32, p3f: u32, p4z: u32, p4b: u32, p4s: u32, p4f: u32) = cdecl 0x6EC5D0 => crate::waterlevel::render_water_rectangle;
}

/// The visible water blocks (count, x and y as int16).
const RW_BLOCK_COUNT: usize = 0xC2_15EC;
const RW_BLOCK_X: usize = 0xC2_1560;
const RW_BLOCK_Y: usize = 0xC2_14D0;
/// The detailed-segment distance (`.data`, 600).
const RW_DETAIL_DIST: usize = 0x8D_3934;
/// The water rasters: the one the protection chunk 0x407EE4 decodes (`[[0x15608B0] + [0x42952A]]` =
/// `[0xC228B0]`) and the second pass's.
const RW_RASTER: usize = 0xC2_28B0;
const RW_RASTER2: usize = 0xC2_28A8;
/// TEXTUREADDRESS value (`.data`), the scroll speeds and the four texture offsets (`.data`).
const RW_ADDRESS_MODE: usize = 0x8D_3930;
const RW_SCROLL_SCALE: usize = 0x8D_392C;
const RW_OFFSETS: usize = 0x8D_3824;
const RW_WIND_X: usize = 0xC2_2890;
const RW_WIND_Y: usize = 0xC2_2894;
/// `CWeather::Wavyness`.
const RW_WAVYNESS: usize = 0xC8_12E8;
/// Random amplitude and wave factor (`.data`).
const RW_RAND_AMP: usize = 0x8D_3928;
const RW_WAVE_AMP: usize = 0x8D_3834;
/// Outputs: the shader offsets 0xC21170..0xC21184, the colour 0xC2116C (bytes) / 0xC21168 (dword), the
/// alpha 0x8D380C and the scaled alpha 0x8D3808.
const RW_COLOUR: usize = 0xC2_116C;
const RW_COLOUR_DW: usize = 0xC2_1168;
const RW_ALPHA: usize = 0x8D_380C;
const RW_ALPHA_SCALED: usize = 0x8D_3808;
/// `CTimeCycle::m_CurrentColours` water red / green / blue / alpha (floats).
const RW_WATER_RGBA: usize = 0xB7_C508;
/// Triangles (count, 8-byte records: vertex indices at +0/+2/+4, flags at +6) and quads (count, 10-byte
/// records: indices +0..+6, flags +8); vertices: 20 bytes (x, y int16, CRenPar 16 bytes).
const RW_TRI_COUNT: usize = 0xC2_2884;
const RW_TRIS: usize = 0xC2_2854;
const RW_QUAD_COUNT: usize = 0xC2_2888;
const RW_QUADS: usize = 0xC2_1C90;
const RW_VERTS: usize = 0xC2_2910;
/// The debug "colour every quad" byte.
const RW_DEBUG_COLOURS: usize = 0xC2_28DC;

fn rw_rs(state: u32, value: u32) {
    rw_render_state_set(state, value);
}

fn rw_rand_at(site: u32) -> i32 {
    sa_core::orig::at_site(site, sa_core::crt::rand)
}

/// A water vertex: (x, y, the four CRenPar dwords).
fn rw_vertex(i: i16) -> (i32, i32, [u32; 4]) {
    let v = RW_VERTS + 20 * i as isize as usize;
    (
        i32::from(g::<i16>(v).read()),
        i32::from(g::<i16>(v + 2).read()),
        [g::<u32>(v + 4).read(), g::<u32>(v + 8).read(), g::<u32>(v + 0xC).read(), g::<u32>(v + 0x10).read()],
    )
}

/// `fld [m]; fcomp 1.0; test ah, 41h; jne` + `-= 1.0`: wrap a texture offset above 1.
fn rw_wrap(a: usize) {
    if !jne(x(g::<f32>(a).read()), x(1.0), 0x41) {
        g::<f32>(a).write(x(g::<f32>(a).read()).fsub_m(1.0).to_f32());
    }
}

fn rw_flush() {
    if g::<u32>(TEMP_VERTICES_STORED).read() != 0 {
        little_test_rw();
        if !rw_im3d_transform(TEMP_RENDER_VERTICES, g::<u32>(TEMP_VERTICES_STORED).read(), 0, 1).is_null() {
            rw_im3d_render_indexed_primitive(3, TEMP_BUFFER_INDICES, g::<u32>(TEMP_INDICES_STORED).read());
            rw_im3d_end();
        }
    }
}

orig! {
    /// `Common::LittleTest()` (0x541330)
    fn little_test_rw() = cdecl 0x541330 => sa_core::common::little_test;
}

/// `CWaterLevel::RenderWater()` (0x6EF650). Nothing unless `CGame::CanSeeWaterFromCurrArea`.
/// `SetCameraRange`, `App::DefinedState`; the protection chunk at 0x407EE4 loads the raster from
/// `[[0x15608B0] + [0x42952A]]` (= `[0xC228B0]`; 0x42952A is code read as data, `code-data-reads.csv`).
/// TEXTURERASTER, both temp-buffer counts zeroed, FOG 1, SRCALPHA / INVSRCALPHA, state 0x1E = 0, ZTEST 1.
///
/// **Sea bed:** for every visible block (x, y int16): the camera distance to its centre `((b + 0.5) *
/// 500 - 3000)` (x unrounded, y f32; `sqrt(dy² + dx²)`) below 600 (not NaN) selects the detailed
/// segment (0x6E6A10) over the plain one (0x6E6870). Out of the 12 x 12 grid one segment `(0, 1, 0, 1)`.
/// Inside it, the edge blocks get partial segments: x 0 → x range (0, 0.04), x 11 → (0.96, 1); y 0 →
/// y range (0, 0.04), y 11 → (0.96, 1); an x edge draws `(x0, x1, 0, 1)`, a y edge `(0, 1, y0, y1)`.
/// The buffer is flushed (`LittleTest`, `RwIm3DTransform(.., NULL, 1)`, indexed triangle list).
///
/// ZWRITE 0, TEXTUREADDRESS from 0x8D3930. Scrolling: `t = timeStep * wind * scale`; offsets 0/1 +=
/// `0.08 * t` and 2/3 += `t * 0.04` (each wrapped above 1 by `- 1`). Waves: `a = (ms & 0xFFF) *
/// 0.0015339808`, `b` = f32 `(ms & 0x1FFF) * 0.00076699`; 0xC21184 = `sin a * Wavyness * 0.08 + off0`,
/// 0xC21180 = `cos a * Wavyness * 0.08 + off1`, 0xC2117C = off2 (raw), 0xC21178 = `cos b * 0.024 +
/// off3`; two `rand()` (0x6EFA9C, 0x6EFAC3): 0xC21174 = `sin b * 0.1 + (r1 / 32767) * amp`, 0xC21170 =
/// `((r2 / 32767) * amp) + cos b * 0.1`. Colour bytes `ftol` of the water r, g, b; the dword 0xC21168 is
/// copied before the alpha byte is written (the old fourth byte goes with it); alpha `ftol(a * 0.5)` →
/// 0x8D380C, `min((alpha << 8) / (256 - alpha), 255)` → 0x8D3808 (signed division; 256 faults as the
/// original's IDIV).
///
/// **Surface:** raster 0xC228A8, counts zeroed, FOG 1, SRCALPHA / INVSRCALPHA; every triangle and
/// quad with its bit 0 set is drawn (`RenderWaterTriangle(x1, y1, P1, x2, y2, P2, x3, y3, P3)`,
/// `RenderWaterRectangle(x[v1], x[v2], y[v1], y[v3], P1..P4)`) and the bit cleared; the quad count is
/// re-read every iteration. With the debug byte set, a quad's colour is `ftol((i & 15) / 16 * 255)`
/// etc. for `i`, `i / 16`, `i / 256` and `ftol(water alpha)` (whole dword copied after). Blocks outside
/// the grid draw a flat rectangle `((b * 500 - 3000) .. ((b + 1) * 500 - 3000))` with four CRenPar
/// `(0, 1, 0, bytes 0, 0, residue, residue)` (the original builds the last dword from two zero bytes
/// and two never-written stack bytes; 0 here). Flush, counts zeroed, `RenderBoatWakes`,
/// `App::DefinedState` (tail call).
#[allow(clippy::cognitive_complexity)]
#[hook(addr = 0x6EF650, conv = cdecl, name = "CWaterLevel::RenderWater", expect = "83 EC 5C 53 55 56",
       route = "L2", hazards = "x87-transcendental,heap-writes,uninit-locals",
       recs = "grove-walk,air-water",
       note = "rand at 0x6EFA9C / 0x6EFAC3; FSIN/FCOS/FSQRT; the protection chunk 0x407EE4 (raster [0xC228B0]) is inlined; writes 0x8D3808..0x8D3834, 0xC21168..0xC21187, the triangle/quad flag bytes and the temp buffers; the outside-grid CRenPar upper bytes are stack residue")]
pub fn render_water() {
    if rw_can_see_water() as u8 == 0 {
        return;
    }
    rw_set_camera_range();
    rw_defined_state();
    rw_rs(1, g::<u32>(RW_RASTER).read());
    g::<u32>(TEMP_INDICES_STORED).write(0);
    g::<u32>(TEMP_VERTICES_STORED).write(0);
    rw_rs(14, 1);
    rw_rs(10, 5);
    rw_rs(11, 6);
    rw_rs(0x1E, 0);
    rw_rs(12, 1);
    let cpos = || sa_world::world::common::cam_pos().cast::<u8>();
    let mut n = 0i32;
    while n < g::<i32>(RW_BLOCK_COUNT).read() {
        let bx = i32::from(g::<i16>(RW_BLOCK_X + 2 * n as usize).read());
        let by = i32::from(g::<i16>(RW_BLOCK_Y + 2 * n as usize).read());
        n += 1;
        // 0.5, 500 (0x858B58), 3000 (0x859A94)
        let cx = xi(bx).fadd_m(0.5).fmul_m(500.0).fsub_m(3000.0);
        let cy = xi(by).fadd_m(0.5).fmul_m(500.0).fsub_m(3000.0).to_f32();
        let dx = cx.fsubr_m(rd::<f32>(cpos(), 0));
        let dy = x(rd::<f32>(cpos(), 4)).fsub_m(cy);
        let dist = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
        let near = !jp(dist, x(g::<f32>(RW_DETAIL_DIST).read()), 5);
        let seg = |fx0: f32, fx1: f32, fy0: f32, fy1: f32| {
            if near {
                rw_detailed_segment(bx, by, fx0, fx1, fy0, fy1);
            } else {
                rw_segment(bx, by, fx0, fx1, fy0, fy1);
            }
        };
        // 0.04 (0x3D23D70A), 0.96 (0x3F75C28F)
        let (lo, hi) = (k(0x3D23_D70A), k(0x3F75_C28F));
        if !(0..12).contains(&bx) || !(0..12).contains(&by) {
            seg(0.0, 1.0, 0.0, 1.0);
            continue;
        }
        let (mut x0, mut x1, mut y0, mut y1) = (0.0f32, 1.0f32, 0.0f32, 1.0f32);
        let mut x_edge = false;
        let mut y_edge = false;
        if bx == 0 {
            x1 = lo;
            x_edge = true;
        } else if bx == 11 {
            x0 = hi;
            x1 = 1.0;
            x_edge = true;
        }
        if by == 0 {
            y1 = lo;
            y_edge = true;
        } else if by == 11 {
            y0 = hi;
            y1 = 1.0;
            y_edge = true;
        }
        if x_edge {
            seg(x0, x1, 0.0, 1.0);
        }
        if y_edge {
            seg(0.0, 1.0, y0, y1);
        }
    }
    rw_flush();
    g::<u32>(TEMP_VERTICES_STORED).write(0);
    g::<u32>(TEMP_INDICES_STORED).write(0);
    rw_rs(8, 0);
    rw_rs(2, g::<u32>(RW_ADDRESS_MODE).read());

    // 0.08 (0x859018), 0.04 (0x858CEC)
    let ts = || g::<f32>(TIME_STEP).read();
    let t1 = x(ts()).fmul_m(g::<f32>(RW_WIND_X).read()).fmul_m(g::<f32>(RW_SCROLL_SCALE).read());
    g::<f32>(RW_OFFSETS).write(x(k(0x3DA3_D70A)).fmul(t1).fadd_m(g::<f32>(RW_OFFSETS).read()).to_f32());
    let t2 = x(ts()).fmul_m(g::<f32>(RW_WIND_Y).read()).fmul_m(g::<f32>(RW_SCROLL_SCALE).read());
    g::<f32>(RW_OFFSETS + 4).write(x(k(0x3DA3_D70A)).fmul(t2).fadd_m(g::<f32>(RW_OFFSETS + 4).read()).to_f32());
    rw_wrap(RW_OFFSETS);
    rw_wrap(RW_OFFSETS + 4);
    g::<f32>(RW_OFFSETS + 8).write(t1.fmul_m(k(0x3D23_D70A)).fadd_m(g::<f32>(RW_OFFSETS + 8).read()).to_f32());
    g::<f32>(RW_OFFSETS + 0xC).write(t2.fmul_m(k(0x3D23_D70A)).fadd_m(g::<f32>(RW_OFFSETS + 0xC).read()).to_f32());
    rw_wrap(RW_OFFSETS + 8);
    rw_wrap(RW_OFFSETS + 0xC);

    let ms = g::<u32>(TIME_MS_RW).read();
    // 0.0015339808 (0x872174), 0.00076699 (0x872170), 0.024 (0x87216C), 1/32767 (0x858C7C)
    let a = xi((ms & 0xFFF) as i32).fmul_m(k(0x3AC9_0FDB));
    let wav = || g::<f32>(RW_WAVYNESS).read();
    g::<f32>(0xC2_1184).write(a.fsin().fmul_m(wav()).fmul_m(k(0x3DA3_D70A)).fadd_m(g::<f32>(RW_OFFSETS).read()).to_f32());
    g::<f32>(0xC2_1180).write(a.fcos().fmul_m(wav()).fmul_m(k(0x3DA3_D70A)).fadd_m(g::<f32>(RW_OFFSETS + 4).read()).to_f32());
    let b = xi((ms & 0x1FFF) as i32).fmul_m(k(0x3A49_0FDB)).to_f32();
    g::<u32>(0xC2_117C).write(g::<u32>(RW_OFFSETS + 8).read());
    let amp = g::<f32>(RW_RAND_AMP).read();
    let cb = x(b).fcos().to_f32();
    g::<f32>(0xC2_1178).write(x(b).fcos().fmul_m(k(0x3CC4_9BA6)).fadd_m(g::<f32>(RW_OFFSETS + 0xC).read()).to_f32());
    let r1 = rw_rand_at(0x6E_FA9C);
    let amp2 = g::<f32>(RW_RAND_AMP).read();
    g::<f32>(0xC2_1174).write(xi(r1).fmul_m(k(0x3800_0100)).fmul_m(amp).to_f32());
    let r2 = rw_rand_at(0x6E_FAC3);
    let rr = xi(r2).fmul_m(k(0x3800_0100));
    let wa = || g::<f32>(RW_WAVE_AMP).read();
    g::<f32>(0xC2_1174).write(x(b).fsin().fmul_m(wa()).fadd_m(g::<f32>(0xC2_1174).read()).to_f32());
    g::<f32>(0xC2_1170).write(rr.fmul_m(amp2).fadd(x(cb).fmul_m(wa())).to_f32());

    let col = |i: usize| ftol32(x(g::<f32>(RW_WATER_RGBA + 4 * i).read())) as u8;
    g::<u8>(RW_COLOUR).write(col(0));
    g::<u8>(RW_COLOUR + 1).write(col(1));
    g::<u8>(RW_COLOUR + 2).write(col(2));
    g::<u32>(RW_COLOUR_DW).write(g::<u32>(RW_COLOUR).read());
    let alpha = ftol32(x(g::<f32>(RW_WATER_RGBA + 0xC).read()).fmul_m(0.5));
    g::<i32>(RW_ALPHA).write(alpha);
    let q = (alpha << 8) / (0x100 - alpha);
    g::<i32>(RW_ALPHA_SCALED).write(0xFF);
    if q <= 0xFF {
        g::<i32>(RW_ALPHA_SCALED).write(q);
    }

    rw_rs(1, g::<u32>(RW_RASTER2).read());
    g::<u32>(TEMP_INDICES_STORED).write(0);
    g::<u32>(TEMP_VERTICES_STORED).write(0);
    rw_rs(14, 1);
    rw_rs(10, 5);
    rw_rs(11, 6);
    let mut i = 0usize;
    while (i as i32) < g::<i32>(RW_TRI_COUNT).read() {
        let t = RW_TRIS + 8 * i;
        i += 1;
        if g::<u8>(t + 6).read() & 1 == 0 {
            continue;
        }
        let (x1, y1, p1) = rw_vertex(g::<i16>(t).read());
        let (x2, y2, p2) = rw_vertex(g::<i16>(t + 2).read());
        let (x3, y3, p3) = rw_vertex(g::<i16>(t + 4).read());
        rw_triangle(x1, y1, p1[0], p1[1], p1[2], p1[3], x2, y2, p2[0], p2[1], p2[2], p2[3], x3, y3, p3[0], p3[1], p3[2], p3[3]);
        g::<u8>(t + 6).write(g::<u8>(t + 6).read() & 0xFE);
    }
    let mut i = 0i32;
    while i < g::<i32>(RW_QUAD_COUNT).read() {
        let qd = RW_QUADS + 10 * i as usize;
        if g::<u8>(qd + 8).read() & 1 != 0 {
            if g::<u8>(RW_DEBUG_COLOURS).read() != 0 {
                // 1/16 (0x858620), 255 (0x859AAC)
                let c = |v: i32| ftol32(xi(v & 0xF).fmul_m(0.0625).fmul_m(255.0)) as u8;
                g::<u8>(RW_COLOUR).write(c(i));
                g::<u8>(RW_COLOUR + 1).write(c(i / 16));
                g::<u8>(RW_COLOUR + 2).write(c(i / 256));
                g::<u8>(RW_COLOUR + 3).write(ftol32(x(g::<f32>(RW_WATER_RGBA + 0xC).read())) as u8);
                g::<u32>(RW_COLOUR_DW).write(g::<u32>(RW_COLOUR).read());
            }
            let (x1, y1, p1) = rw_vertex(g::<i16>(qd).read());
            let (x2, _, p2) = rw_vertex(g::<i16>(qd + 2).read());
            let (_, y3, p3) = rw_vertex(g::<i16>(qd + 4).read());
            let (_, _, p4) = rw_vertex(g::<i16>(qd + 6).read());
            rw_rectangle(x1, x2, y1, y3, p1[0], p1[1], p1[2], p1[3], p2[0], p2[1], p2[2], p2[3], p3[0], p3[1], p3[2], p3[3], p4[0], p4[1], p4[2], p4[3]);
            g::<u8>(qd + 8).write(g::<u8>(qd + 8).read() & 0xFE);
        }
        i += 1;
    }
    let mut n = 0usize;
    while (n as i32) < g::<i32>(RW_BLOCK_COUNT).read() {
        let bx = g::<i16>(RW_BLOCK_X + 2 * n).read();
        let by = g::<i16>(RW_BLOCK_Y + 2 * n).read();
        n += 1;
        if (0..12).contains(&bx) && (0..12).contains(&by) {
            continue;
        }
        let ax = i32::from(bx) * 500;
        let ay = i32::from(by) * 500;
        let w = |v: i32| ftol32(xi(v).fsub_m(3000.0));
        let (y2, y1) = (w(ay + 500), w(ay));
        let (x2, x1) = (w(ax + 500), w(ax));
        // CRenPar (0, 1, 0, two zero bytes + two residue bytes)
        let (pz, pb, ps, pf) = (0u32, 0x3F80_0000u32, 0u32, 0u32);
        rw_rectangle(x1, x2, y1, y2, pz, pb, ps, pf, pz, pb, ps, pf, pz, pb, ps, pf, pz, pb, ps, pf);
    }
    rw_flush();
    g::<u32>(TEMP_VERTICES_STORED).write(0);
    g::<u32>(TEMP_INDICES_STORED).write(0);
    rw_render_boat_wakes();
    rw_defined_state();
}

orig! {
    /// `CWaterLevel::RenderBoatWakes()` (0x6ED9A0)
    fn rw_render_boat_wakes() = cdecl 0x6ED9A0 => self::render_boat_wakes;
}

/// `CTimer::m_snTimeInMilliseconds`.
const TIME_MS_RW: usize = 0xB7_CB84;
/// `CTimer::ms_fTimeStep`.
const TIME_STEP: usize = 0xB7_CB5C;
