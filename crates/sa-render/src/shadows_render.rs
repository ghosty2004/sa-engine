//! WRITER-RENDER-BIG-2: the `CShadows` render / store workers (`RenderStoredShadows`, and the
//! `RenderBuffer::StopStoring` leaf). Ported from the 1.0 US bytes; gta-reversed `Shadows.cpp` as
//! reading aid. x87 arithmetic follows the original's instruction order on the `sa_math::x87::X87` model.
//!
//! `CShadows::asShadowsStored` (0xC40430, 0x34 bytes each, count `ShadowsStoredToBeRendered` u16 at
//! 0xC403DC): +0 position, +0xC front (x, y), +0x14 side (x, y), +0x1C z distance, +0x20 scale, +0x24
//! texture, +0x28 real-time shadow, +0x2C intensity (u16), +0x2E type, +0x2F..+0x31 r, g, b, +0x32 flags
//! (bit 0 draw on water, bit 1 already rendered in batch, bit 2 draw on buildings).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, jp, k, rd, wr, x, xi};

orig! {
    /// `RwRenderStateSet(state, value)` (`RwGlobals` +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xc97b24 0x20 => unported;
    /// `CShadows::SetRenderModeForShadowType(uint8)` (0x707460; the dword the original pushes)
    fn set_render_mode_for_shadow_type(ty: u32) = cdecl 0x707460 => crate::shadows::set_render_mode_for_shadow_type;
    /// `CWorld::ClearScanCodes()` (0x563470)
    fn clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CShadows::CastRealTimeShadowSectorList(list, ...)` (0x70a7e0, 19 dwords)
    fn cast_real_time_shadow_sector_list(list: GPtr<u8>, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32, a12: u32, a13: u32, a14: u32, a15: u32, a16: u32, a17: u32, a18: u32, a19: u32) = cdecl 0x70a7e0 => crate::mvpd_r1::cast_real_time_shadow_sector_list;
    /// `CShadows::CastShadowSectorList(list, ...)` (0x70a630, 19 dwords)
    fn cast_shadow_sector_list(list: GPtr<u8>, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32, a12: u32, a13: u32, a14: u32, a15: u32, a16: u32, a17: u32, a18: u32, a19: u32) = cdecl 0x70a630 => unported;
    /// `CShadows::CastPlayerShadowSectorList(list, ...)` (0x70a470, 19 dwords)
    fn cast_player_shadow_sector_list(list: GPtr<u8>, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32, a12: u32, a13: u32, a14: u32, a15: u32, a16: u32, a17: u32, a18: u32, a19: u32) = cdecl 0x70a470 => crate::mvpd_r1::cast_player_shadow_sector_list;
    /// `CWaterLevel::GetWaterLevelNoWaves(x, y, z, float* level, float* big, float* small)` (0x6e8580)
    fn get_water_level_no_waves(px: u32, py: u32, pz: u32, level: GPtr<f32>, big: GPtr<f32>, small: GPtr<f32>) -> u8 = cdecl 0x6e8580 => crate::waterlevel::get_water_level_no_waves;
    /// `FUN_006e8550(int x, int y, float* inout, float big, float small)` (0x6e8550): the waves at a point
    fn waves_for_coordinate(px: i32, py: i32, out: GPtr<f32>, big: u32, small: u32) = cdecl 0x6e8550 => crate::waterlevel::waves_for_coordinate;
    /// `RwIm3DTransform(verts, count, ltm, flags)` (0x7ef450)
    fn rw_im3d_transform(verts: u32, count: u32, ltm: u32, flags: u32) -> GPtr<u8> = cdecl 0x7ef450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(primType, indices, numIndices)` (0x7ef550)
    fn rw_im3d_render_indexed_primitive(prim: u32, indices: u32, n: u32) -> u32 = cdecl 0x7ef550 => unported;
    /// `RwIm3DEnd()` (0x7ef520)
    fn rw_im3d_end() -> u32 = cdecl 0x7ef520 => unported;
    /// `RenderBuffer::StopStoring()` (0x7077a0)
    fn stop_storing_call() = cdecl 0x7077a0 => self::render_buffer_stop_storing;
}

/// `TempBufferVerticesStored`, `TempBufferIndicesStored`, `aTempBufferIndices`, `TempBufferRenderVertices`.
const TEMP_VERTICES_STORED: usize = 0xc4_b950;
const TEMP_INDICES_STORED: usize = 0xc4_b954;
const TEMP_BUFFER_INDICES: usize = 0xc4_b958;
const TEMP_RENDER_VERTICES: usize = 0xc4_d958;
/// `RenderBuffer`'s pending index / vertex counts of the current `StartStoring`.
const PENDING_INDICES: usize = 0xc4_0410;
const PENDING_VERTICES: usize = 0xc4_0414;
/// `CShadows::asShadowsStored`, `ShadowsStoredToBeRendered`.
const SHADOWS_STORED: usize = 0xc4_0430;
const SHADOWS_STORED_COUNT: usize = 0xc4_03dc;
/// `CWorld::ms_nCurrentScanCode` (u16), `CWorld::ms_aSectors` (120 x 120, 8 bytes, buildings list first).
const SCAN_CODE: usize = 0xb7_cd78;
const SECTORS: usize = 0xb7_d0b8;

/// `RenderBuffer::StopStoring()` (0x7077A0): the pending indices get the stored vertex count added
/// (u16), then both counts advance by the pending counts.
#[hook(addr = 0x7077a0, conv = cdecl, name = "RenderBuffer::StopStoring", expect = "A1 54 B9 C4 00 8B 0D 10 04 C4 00",
       unverified = "renderbig2-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_buffer_stop_storing() {
    let first = g::<i32>(TEMP_INDICES_STORED).read();
    let end = g::<i32>(PENDING_INDICES).read().wrapping_add(first);
    if first < end {
        let nv = g::<u32>(TEMP_VERTICES_STORED).read();
        let mut i = first;
        while i < end {
            let p = g::<u16>(TEMP_BUFFER_INDICES.wrapping_add((i as usize).wrapping_mul(2)));
            p.write(p.read().wrapping_add(nv as u16));
            i += 1;
        }
        g::<u32>(TEMP_VERTICES_STORED).write(nv.wrapping_add(g::<u32>(PENDING_VERTICES).read()));
    } else {
        let nv = g::<u32>(TEMP_VERTICES_STORED).read();
        g::<u32>(TEMP_VERTICES_STORED).write(nv.wrapping_add(g::<u32>(PENDING_VERTICES).read()));
    }
    g::<i32>(TEMP_INDICES_STORED).write(end);
}

/// The inlined `RenderBuffer::RenderStuffInBuffer`: with vertices stored, `RwIm3DTransform(verts, n, 0,
/// 1)` and, when it succeeds, `RenderIndexedPrimitive(TRILIST, indices, ni)` + `RwIm3DEnd`; both counts
/// zeroed.
fn render_stuff_in_buffer() {
    let nv = g::<u32>(TEMP_VERTICES_STORED).read();
    if nv != 0 && !rw_im3d_transform(TEMP_RENDER_VERTICES as u32, nv, 0, 1).is_null() {
        rw_im3d_render_indexed_primitive(3, TEMP_BUFFER_INDICES as u32, g::<u32>(TEMP_INDICES_STORED).read());
        rw_im3d_end();
    }
    g::<u32>(TEMP_INDICES_STORED).write(0);
    g::<u32>(TEMP_VERTICES_STORED).write(0);
}

/// CRT `floor` / `ceil` (0x8219F0 / 0x823820) of the double the original passes (exact in f64).
fn floor_x(v: X87) -> X87 {
    X87::from_f64_exact(sa_world::physical::base::floor_f64(v.to_f64()))
}

fn ceil_x(v: X87) -> X87 {
    X87::from_f64_exact(-sa_world::physical::base::floor_f64(-v.to_f64()))
}

/// The sector index of a rect edge: `ftol(floor(v * 0.02 + 60))`; `low` clamps below at 0 (else above
/// at 119). The original computes `floor` + `_ftol` again from the stored f32 when in range.
fn sector_coord(v: f32, low: bool) -> i32 {
    // 0.02 (0x858B38), 60 (0x858B34)
    let t = x(v).fmul_m(k(0x3ca3_d70a)).fadd_m(k(0x4270_0000));
    let n = ftol32(floor_x(t));
    let again = || ftol32(floor_x(x(t.to_f32())));
    if low {
        if n > 0 { again() } else { 0 }
    } else if n < 0x77 {
        again()
    } else {
        0x77
    }
}

/// One water vertex of the shadow quad (the four inlined copies at 0x70B0E9..0x70B639): position
/// `(vx, vy, z + 0.06)` with `z` the waves' height, colour `(intensity (i16) * 0.6) << 24 | rgb`,
/// `u = (((x - px) * sx + (y - py) * sy) + (-pz * 0)) * invSide + 1) * 0.5` (the x term stored as f32,
/// the y term kept on the stack) and `v = 0.5 - (((x - px) * fx + (y - py) * fy) + (-pz * 0)) *
/// invFront * 0.5` (every term stored as f32).
fn water_vertex(sh: GPtr<u8>, vtx: GPtr<u8>, vx: f32, vy: f32, z: f32, inv_side: f32, inv_front: f32) {
    let (px, py, pz) = (rd::<f32>(sh, 0), rd::<f32>(sh, 4), rd::<f32>(sh, 8));
    let (fx, fy, sx, sy) = (rd::<f32>(sh, 0xc), rd::<f32>(sh, 0x10), rd::<f32>(sh, 0x14), rd::<f32>(sh, 0x18));
    let dx = x(vx).fsub_m(px).to_f32();
    let dy = x(vy).fsub_m(py);
    let npz = x(pz).fchs();
    // 0.6 (0x858CC8)
    let alpha = ftol32(xi(i32::from(rd::<u16>(sh, 0x2c) as i16)).fmul_m(k(0x3f19_999a))) as u32 & 0xff;
    let col = (((((alpha << 8) | u32::from(rd::<u8>(sh, 0x2f))) << 8) | u32::from(rd::<u8>(sh, 0x30))) << 8)
        | u32::from(rd::<u8>(sh, 0x31));
    let u = dy.fmul_m(sy).fadd(x(dx).fmul_m(sx)).fadd(npz.fmul_m(0.0)).fmul_m(inv_side).fadd_m(1.0).fmul_m(0.5);
    let dx2 = x(vx).fsub_m(px).to_f32();
    let dy2 = x(vy).fsub_m(py).to_f32();
    let npz2 = x(pz).fchs().to_f32();
    let v = x(dx2)
        .fmul_m(fx)
        .fadd(x(dy2).fmul_m(fy))
        .fadd(x(npz2).fmul_m(0.0))
        .fmul_m(inv_front)
        .fmul_m(0.5)
        .fsubr_m(0.5);
    wr::<f32>(vtx, 0, vx);
    wr::<f32>(vtx, 4, vy);
    // 0.06 (0x859934)
    wr::<f32>(vtx, 8, x(z).fadd_m(k(0x3d75_c28f)).to_f32());
    wr::<u32>(vtx, 0x18, col);
    wr::<f32>(vtx, 0x1c, u.to_f32());
    wr::<f32>(vtx, 0x20, v.to_f32());
}

/// `CShadows::RenderStoredShadows()` (0x70A960). Both temp-buffer counts zeroed; states ZWRITE 0,
/// ZTEST 1, VERTEXALPHA 1, FOG 0, TEXTUREADDRESS clamp, CULL none, ALPHATESTREF 0, FILTER linear (also
/// with no shadows: no early out); every shadow's "rendered in batch" bit cleared.
///
/// Per shadow `o` (the count re-read each time): unless already rendered, `SetRenderModeForShadowType`,
/// its texture's raster, and the batch: every later shadow `i >= o` with the same type and texture
/// (`o` itself included) gets the next scan code (`ClearScanCodes` at 0xFFFF) and its rect `pos -/+
/// (|side| + |front|)` per axis; over the sectors `ftol(floor(edge * 0.02 + 60))` clamped to 0..119 the
/// buildings list is cast with `(list, left, bottom, right, top, &posCopy, front, side, intensity,
/// r, g, b, zDistance, scale, 0, ...)`: real-time shadows `(.., rtShadow, &unusedByte)`, draw-on-
/// buildings `CastShadowSectorList(.., &unusedByte, type)`, others `CastPlayerShadowSectorList` (same
/// arguments); the shadow is marked rendered. Then the buffer is rendered out.
///
/// On water (flag bit 0): `GetWaterLevelNoWaves` of the stack copy of the position of the **last
/// shadow the most recent batch processed** (not necessarily `o`; an original bug kept); nothing when
/// it fails or the level is `>= o`'s z (or unordered). The grid steps by 2 over `2 * floor((pos -
/// ext) / 2) ..= 2 * ceil((pos + ext) / 2)` per axis; per square (StartStoring 6 / 4: flush when the
/// buffer would reach 0x1000 indices / 0x800 vertices) four `water_vertex`es at (x, y), (x+2, y),
/// (x, y+2), (x+2, y+2), each with the waves of `FUN_006e8550(x, y, &h, big, small)` (h set to the
/// level before each of the four calls), indices 0 1 2 1 3 2, `StopStoring`. Then the
/// buffer is rendered out.
///
/// End states: VERTEXALPHA 0, ZWRITE 1, ZTEST 1, TEXTUREADDRESS wrap, CULL back; the count zeroed.
#[allow(clippy::cognitive_complexity)]
#[hook(addr = 0x70a960, conv = cdecl, name = "CShadows::RenderStoredShadows", expect = "55 8B EC 83 E4 F8",
       unverified = "renderbig2-1", route = "L2", hazards = "heap-writes,uninit-locals",
       recs = "grove-walk,driving",
       note = "writes TempBufferRenderVertices / aTempBufferIndices and both counts; the water pass uses the stack copy of the last batch's shadow position (stale across shadows); the unused byte passed to the Cast*SectorList calls is uninitialised stack")]
pub fn render_stored_shadows() {
    g::<u32>(TEMP_INDICES_STORED).write(0);
    g::<u32>(TEMP_VERTICES_STORED).write(0);
    for (s, v) in [(8, 0), (6, 1), (12, 1), (14, 0), (2, 3), (20, 1), (30, 0), (9, 2)] {
        rw_render_state_set(s, v);
    }
    let count = || i32::from(g::<u16>(SHADOWS_STORED_COUNT).read());
    let shadow = |i: i32| GPtr::<u8>::from_addr(SHADOWS_STORED + 0x34 * i as usize);
    for i in 0..count() {
        wr::<u8>(shadow(i), 0x32, rd::<u8>(shadow(i), 0x32) & 0xfd);
    }
    // the stack copy of the batch shadow's position (S+0x58)
    let mut pos_copy = [0u32; 3];
    let mut unused: u8 = 0;
    let mut o = 0;
    while o < count() {
        let os = shadow(o);
        if rd::<u8>(os, 0x32) & 2 == 0 {
            set_render_mode_for_shadow_type(u32::from(rd::<u8>(os, 0x2e)));
            rw_render_state_set(1, rd::<u32>(rd::<GPtr<u8>>(os, 0x24), 0));
            let mut i = o;
            while i < count() {
                let is = shadow(i);
                if rd::<u8>(os, 0x2e) == rd::<u8>(is, 0x2e) && rd::<u32>(os, 0x24) == rd::<u32>(is, 0x24) {
                    cast_one(os, is, &mut pos_copy, &mut unused);
                    wr::<u8>(is, 0x32, rd::<u8>(is, 0x32) | 2);
                }
                i += 1;
            }
            render_stuff_in_buffer();
        }
        if rd::<u8>(os, 0x32) & 1 != 0 {
            water_pass(os, &pos_copy);
        }
        o += 1;
    }
    for (s, v) in [(12, 0), (8, 1), (6, 1), (2, 1), (20, 2)] {
        rw_render_state_set(s, v);
    }
    g::<u16>(SHADOWS_STORED_COUNT).write(0);
}

/// The batch body for shadow `is` (0x70AA76..0x70ADF1).
fn cast_one(os: GPtr<u8>, is: GPtr<u8>, pos_copy: &mut [u32; 3], unused: &mut u8) {
    let a = x(rd::<f32>(is, 0x14)).fabs().fadd(x(rd::<f32>(is, 0xc)).fabs());
    let b = x(rd::<f32>(is, 0x18)).fabs().fadd(x(rd::<f32>(is, 0x10)).fabs());
    *pos_copy = [rd::<u32>(is, 0), rd::<u32>(is, 4), rd::<u32>(is, 8)];
    let px = f32::from_bits(pos_copy[0]);
    let py = f32::from_bits(pos_copy[1]);
    let left = x(px).fsub(a).to_f32();
    let right = x(px).fadd(a).to_f32();
    let bottom = x(py).fsub(b).to_f32();
    let top = x(py).fadd(b).to_f32();
    let x0 = sector_coord(left, true);
    let y0 = sector_coord(bottom, true);
    let x1 = sector_coord(right, false);
    let y1 = sector_coord(top, false);
    let sc = g::<u16>(SCAN_CODE);
    if sc.read() < 0xffff {
        sc.write(sc.read() + 1);
    } else {
        clear_scan_codes();
        sc.write(1);
    }
    let pos = GPtr::<u32>::from_raw(pos_copy.as_mut_ptr()).cast::<u8>().addr() as u32;
    let unused_p = GPtr::<u8>::from_raw(unused as *mut u8).addr() as u32;
    let mut yy = y0;
    while yy <= y1 {
        let cy = if yy <= 0 { 0 } else { yy };
        let mut xx = x0;
        while xx <= x1 {
            let cx = if xx <= 0 { 0 } else { xx };
            let cx = if cx < 0x77 { cx } else { 0x77 };
            let cyy = if cy < 0x77 { cy } else { 0x77 };
            let list = GPtr::<u8>::from_addr(SECTORS + 8 * (cyy * 0x78 + cx) as usize);
            let rt = rd::<u32>(is, 0x28);
            let (fx, fy, sx, sy) = (rd::<u32>(is, 0xc), rd::<u32>(is, 0x10), rd::<u32>(is, 0x14), rd::<u32>(is, 0x18));
            let inten = u32::from(rd::<u16>(is, 0x2c));
            let (r, gg, bb) = (u32::from(rd::<u8>(is, 0x2f)), u32::from(rd::<u8>(is, 0x30)), u32::from(rd::<u8>(is, 0x31)));
            let (zd, scale) = (rd::<u32>(is, 0x1c), rd::<u32>(is, 0x20));
            let (l, bo, ri, t) = (left.to_bits(), bottom.to_bits(), right.to_bits(), top.to_bits());
            if rt != 0 {
                cast_real_time_shadow_sector_list(list, l, bo, ri, t, pos, fx, fy, sx, sy, inten, r, gg, bb, zd, scale, 0, rt, unused_p);
            } else {
                let ty = u32::from(rd::<u8>(os, 0x2e));
                if rd::<u8>(is, 0x32) & 4 != 0 {
                    cast_shadow_sector_list(list, l, bo, ri, t, pos, fx, fy, sx, sy, inten, r, gg, bb, zd, scale, 0, unused_p, ty);
                } else {
                    cast_player_shadow_sector_list(list, l, bo, ri, t, pos, fx, fy, sx, sy, inten, r, gg, bb, zd, scale, 0, unused_p, ty);
                }
            }
            xx += 1;
        }
        yy += 1;
    }
}

/// The water pass of shadow `os` (0x70AE5B..0x70B6BB).
fn water_pass(os: GPtr<u8>, pos_copy: &[u32; 3]) {
    let mut level = 0f32;
    let mut big = 0f32;
    let mut small = 0f32;
    if get_water_level_no_waves(pos_copy[0], pos_copy[1], pos_copy[2], GPtr::from_raw(&mut level), GPtr::from_raw(&mut big), GPtr::from_raw(&mut small)) == 0 {
        return;
    }
    if jp(x(level), x(rd::<f32>(os, 8)), 0x5) {
        return;
    }
    let a = x(rd::<f32>(os, 0x14)).fabs().fadd(x(rd::<f32>(os, 0xc)).fabs()).to_f32();
    let b = x(rd::<f32>(os, 0x18)).fabs().fadd(x(rd::<f32>(os, 0x10)).fabs()).to_f32();
    let (px, py) = (rd::<f32>(os, 0), rd::<f32>(os, 4));
    let x0 = ftol32(floor_x(x(px).fsub_m(a).fmul_m(0.5))).wrapping_shl(1);
    let y0 = ftol32(floor_x(x(py).fsub_m(b).fmul_m(0.5))).wrapping_shl(1);
    let x1 = ftol32(ceil_x(x(a).fadd_m(px).fmul_m(0.5))).wrapping_shl(1);
    let y1 = ftol32(ceil_x(x(b).fadd_m(py).fmul_m(0.5))).wrapping_shl(1);
    let (fx, fy) = (x(rd::<f32>(os, 0xc)), x(rd::<f32>(os, 0x10)));
    let front_sq = fy.fmul(fy).fadd(fx.fmul(fx)).to_f32();
    let (sx, sy) = (x(rd::<f32>(os, 0x14)), x(rd::<f32>(os, 0x18)));
    let side_sq = sy.fmul(sy).fadd(sx.fmul(sx)).to_f32();
    g::<u32>(TEMP_INDICES_STORED).write(0);
    g::<u32>(TEMP_VERTICES_STORED).write(0);
    let mut gx = x0;
    while gx <= x1 {
        let mut gy = y0;
        if gy <= y1 {
            let fxv = xi(gx).to_f32();
            let inv_side = x(1.0).fdiv_m(side_sq).to_f32();
            let inv_front = x(1.0).fdiv_m(front_sq).to_f32();
            let fx2v = xi(gx.wrapping_add(2)).to_f32();
            loop {
                if g::<u32>(TEMP_INDICES_STORED).read().wrapping_add(6) as i32 >= 0x1000 {
                    render_stuff_in_buffer();
                }
                if g::<u32>(TEMP_VERTICES_STORED).read().wrapping_add(4) as i32 >= 0x800 {
                    render_stuff_in_buffer();
                }
                let ni = g::<u32>(TEMP_INDICES_STORED).read() as usize;
                let nv = g::<u32>(TEMP_VERTICES_STORED).read() as usize;
                let idx = TEMP_BUFFER_INDICES + 2 * ni;
                let vtx = GPtr::<u8>::from_addr(TEMP_RENDER_VERTICES + 0x24 * nv);
                g::<u32>(PENDING_INDICES).write(6);
                g::<u32>(PENDING_VERTICES).write(4);
                let mut h = level;
                let hp = GPtr::<f32>::from_raw(&mut h);
                waves_for_coordinate(gx, gy, hp, big.to_bits(), small.to_bits());
                let fyv = xi(gy).to_f32();
                water_vertex(os, vtx, fxv, fyv, h, inv_side, inv_front);
                h = level;
                waves_for_coordinate(gx.wrapping_add(2), gy, hp, big.to_bits(), small.to_bits());
                // the loop variable advances here (the original's y + 2 store at 0x70B37F)
                gy = gy.wrapping_add(2);
                water_vertex(os, vtx.byte_add(0x24), fx2v, fyv, h, inv_side, inv_front);
                h = level;
                waves_for_coordinate(gx, gy, hp, big.to_bits(), small.to_bits());
                let fy2v = xi(gy).to_f32();
                water_vertex(os, vtx.byte_add(0x48), fxv, fy2v, h, inv_side, inv_front);
                h = level;
                waves_for_coordinate(gx.wrapping_add(2), gy, hp, big.to_bits(), small.to_bits());
                water_vertex(os, vtx.byte_add(0x6c), fx2v, fy2v, h, inv_side, inv_front);
                for (j, v) in [0u16, 1, 2, 1, 3, 2].into_iter().enumerate() {
                    g::<u16>(idx + 2 * j).write(v);
                }
                stop_storing_call();
                if gy > y1 {
                    break;
                }
            }
        }
        gx = gx.wrapping_add(2);
    }
    render_stuff_in_buffer();
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_RENDER_BUFFER_STOP_STORING, HOOK_RENDER_STORED_SHADOWS];
