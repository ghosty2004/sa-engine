//! WRITER-RENDER-BIG-2: `CShadows::CastShadowEntityXY` (0x7086B0). Ported from the 1.0 US bytes
//! (gta-reversed has not reversed it). x87 arithmetic follows the original's instruction order on the
//! `sa_math::x87::X87` model.
//!
//! The shadow quad is moved into the entity's local frame (2D: right / forward vectors), clipped
//! against every collision triangle (three Sutherland-Hodgman passes over the global scratch arrays
//! 0xC4B6B8 / 0xC4B7A8 and 0xC4B730 / 0xC4B820), lifted onto the triangle's plane, moved back to world
//! space and either stored as a `CPolyBunch` or written to the temp render buffer.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use sa_world::collision::data::{CdPtr, CmPtr};

use crate::common::{ftol32, g, jne, jp, k, rd, wr, x, xi};

orig! {
    /// `CCollision::CalculateTrianglePlanes(CColModel*)` (0x418580)
    fn calculate_triangle_planes(cm: CmPtr) = cdecl 0x418580 => sa_world::collision::data::calculate_triangle_planes_col_model;
    /// `CPlaceable::AllocateMatrix()` (0x54f560)
    fn allocate_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x54f560 => sa_world::world::entity::allocate_matrix;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*) const` (0x54f1b0)
    fn simple_transform_update_matrix(this: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54f1b0 => crate::mvpd_unk::simple_transform_update_matrix;
    /// `CCollisionData::GetTrianglePoint(CVector&, int)` (0x40f5e0)
    fn get_triangle_point(cd: CdPtr, out: GPtr<CVector>, idx: i32) = thiscall 0x40f5e0 => sa_world::collision::data::get_triangle_point;
    /// `CShadows::AffectColourWithLighting(type, lighting, r, g, b, uint8* outR, outG, outB)` (0x707850)
    fn affect_colour_with_lighting(kind: u32, lighting: u32, r: u32, gg: u32, b: u32, out_r: GPtr<u8>, out_g: GPtr<u8>, out_b: GPtr<u8>) = cdecl 0x707850 => crate::mvpb_r1::affect_colour_with_lighting;
    /// `RwIm3DTransform(verts, count, ltm, flags)` (0x7ef450)
    fn rw_im3d_transform(verts: u32, count: u32, ltm: u32, flags: u32) -> GPtr<u8> = cdecl 0x7ef450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(primType, indices, numIndices)` (0x7ef550)
    fn rw_im3d_render_indexed_primitive(prim: u32, indices: u32, n: u32) -> u32 = cdecl 0x7ef550 => unported;
    /// `RwIm3DEnd()` (0x7ef520)
    fn rw_im3d_end() -> u32 = cdecl 0x7ef520 => unported;
    /// `RenderBuffer::StopStoring()` (0x7077a0)
    fn stop_storing() = cdecl 0x7077a0 => crate::shadows_render::render_buffer_stop_storing;
}

/// The three function-local statics' guard bits (trivially initialised).
const STATIC_GUARD: usize = 0xc4_b8c8;
/// The shadow quad in the entity's frame (4 x (x, y, -), 12 bytes apart).
const QUAD: usize = 0xc4_b898;
/// The clip scratch arrays: uv / position A (input of passes 1 and 3), B (input of pass 2, the result).
const UV_A: usize = 0xc4_b6b8;
const POS_A: usize = 0xc4_b7a8;
const UV_B: usize = 0xc4_b730;
const POS_B: usize = 0xc4_b820;
/// `CModelInfo::ms_modelInfoPtrs`.
const MODEL_INFOS: usize = 0xa9_b0c8;
/// `TempBufferVerticesStored`, `TempBufferIndicesStored`, `aTempBufferIndices`, `TempBufferRenderVertices`;
/// `RenderBuffer`'s pending counts; the shadow fan index list.
const TEMP_VERTICES_STORED: usize = 0xc4_b950;
const TEMP_INDICES_STORED: usize = 0xc4_b954;
const TEMP_BUFFER_INDICES: usize = 0xc4_b958;
const TEMP_RENDER_VERTICES: usize = 0xc4_d958;
const PENDING_INDICES: usize = 0xc4_0410;
const PENDING_VERTICES: usize = 0xc4_0414;
const FAN_INDICES: usize = 0xc4_03a8;
/// `CShadows::pEmptyBunchList`.
const EMPTY_BUNCH_LIST: usize = 0xc4_03d8;

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// The inlined `CPlaceable::GetMatrix()`: allocate and fill the matrix when the entity has none.
fn get_matrix(e: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(e, 0x14) == 0 {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    rd::<GPtr<u8>>(e, 0x14)
}

/// The entity position (`m_matrix ? &m_matrix->pos : &m_placement.pos`), no allocation.
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `max(a, b)` as the original's `fcomp; test ah, 0x41; jne` picks it (b when `a <= b` or unordered).
fn pick_max(a: f32, b: f32) -> f32 {
    if jne(x(a), x(b), 0x41) { b } else { a }
}

/// `min(a, b)` as `fcomp; test ah, 5; jp` picks it (b when `a >= b` or unordered).
fn pick_min(a: f32, b: f32) -> f32 {
    if jp(x(a), x(b), 0x5) { b } else { a }
}

/// One clip pass: the `n` points of (`uv_in`, `pos_in`) against the edge through `o` along `e`; points
/// with `(p - o) x e > 0` stay. The intersection with the previous point uses `t = |cross(prev)| /
/// (|cross(cur)| + |cross(prev)|)` (the current cross as the stored f32), `uv = (1-t) prev + t cur`,
/// `xy = t cur + (1-t) prev` (z is not written). The closing edge (last to first) is checked after the
/// loop. Returns the output count.
fn clip_pass(uv_in: usize, pos_in: usize, n: i32, uv_out: usize, pos_out: usize, o: (f32, f32), e: (X87, X87)) -> i32 {
    let cross = |p: usize| x(gf(p)).fsub_m(o.0).fmul(e.1).fsub(x(gf(p + 4)).fsub_m(o.1).fmul(e.0));
    let mut out = 0usize;
    let emit = |prev: usize, cur: usize, cs: f32, out: &mut usize| {
        let pp = pos_in.wrapping_add(12 * prev);
        let pc = pos_in.wrapping_add(12 * cur);
        let up = uv_in.wrapping_add(12 * prev);
        let uc = uv_in.wrapping_add(12 * cur);
        let cp = cross(pp).fabs();
        let t = cp.fdiv(x(cs).fabs().fadd(cp));
        let s = x(1.0).fsub(t);
        let o_uv = uv_out + 12 * *out;
        let o_pos = pos_out + 12 * *out;
        g::<f32>(o_uv).write(s.fmul_m(gf(up)).fadd(t.fmul_m(gf(uc))).to_f32());
        g::<f32>(o_uv + 4).write(s.fmul_m(gf(up + 4)).fadd(t.fmul_m(gf(uc + 4))).to_f32());
        g::<f32>(o_pos).write(t.fmul_m(gf(pc)).fadd(s.fmul_m(gf(pp))).to_f32());
        g::<f32>(o_pos + 4).write(t.fmul_m(gf(pc + 4)).fadd(s.fmul_m(gf(pp + 4))).to_f32());
        *out += 1;
    };
    let mut state = 0u8;
    for i in 0..n.max(0) as usize {
        let c = cross(pos_in + 12 * i);
        let cs = c.to_f32();
        if !jne(c, x(0.0), 0x41) {
            if state == 2 {
                emit(i.wrapping_sub(1), i, cs, &mut out);
            }
            g::<u32>(uv_out + 12 * out).write(g::<u32>(uv_in + 12 * i).read());
            g::<u32>(uv_out + 12 * out + 4).write(g::<u32>(uv_in + 12 * i + 4).read());
            g::<u32>(pos_out + 12 * out).write(g::<u32>(pos_in + 12 * i).read());
            g::<u32>(pos_out + 12 * out + 4).write(g::<u32>(pos_in + 12 * i + 4).read());
            out += 1;
            state = 1;
        } else {
            if state == 1 {
                emit(i.wrapping_sub(1), i, cs, &mut out);
            }
            state = 2;
        }
    }
    let c0 = cross(pos_in);
    let cs0 = c0.to_f32();
    let close = (!jne(c0, x(0.0), 0x41) && state == 2) || (!jp(x(cs0), x(0.0), 0x41) && state == 1);
    if close {
        emit((n as usize).wrapping_sub(1), 0, cs0, &mut out);
    }
    out as i32
}

/// `CShadows::CastShadowEntityXY(CEntity*, float cornerAX, float cornerAY, float cornerBX, float
/// cornerBY, CVector* pos, float frontX, float frontY, float sideX, float sideY, int16 intensity, uint8
/// r, uint8 g, uint8 b, float zDistance, float scale, CPolyBunch** ppPolyBunch, uint8* pDayNight, int32
/// shadowType)` (0x7086B0); the four corner arguments are not read.
///
/// Nothing without collision triangles. Front, side and `pos - entity pos` go into the entity's frame
/// with the right / forward vectors (`(a.y * r.y) + (a.x * r.x)`, `(a.x * f.x) + (a.y * f.y)`); the quad
/// `p + f - s, p + s + f, p - f + s, p - f - s` (0xC4B898) and its bounds. Per triangle (of the
/// bounding sections that overlap, or all): skipped when `|normal.z| <= 0.1`, when all three points are
/// beyond one side of the bounds in x or y, at or above the local z, or at or below `z - zDistance`.
/// The quad (uv (0,0), (1,0), (1,1), (0,1)) is clipped by the three triangle edges; with at least 3
/// points left their z is put on the triangle plane (`((ny y + nx x) - d) * (-1 / nz)`) and they go back
/// to world space; `*pDayNight` = the triangle's lighting byte. With `ppPolyBunch`: the first empty
/// bunch (none: skip) is linked in, with the points and `ftol(uv * 200)` bytes. Otherwise the temp
/// buffer (flushed at 0x1000 indices / 0x800 vertices) gets the points (z + 0.06), `scale * uv`, colour
/// `intensity << 24 | AffectColourWithLighting(...)`, the fan indices of 0xC403A8 and `StopStoring`.
#[allow(clippy::too_many_arguments, clippy::cognitive_complexity)]
#[hook(addr = 0x7086b0, conv = cdecl, name = "CShadows::CastShadowEntityXY", expect = "A1 C8 B8 C4 00 83 EC 74",
       unverified = "renderbig2-3", route = "L2", hazards = "heap-writes",
       recs = "grove-walk,driving",
       note = "writes the clip scratch arrays, TempBufferRenderVertices / aTempBufferIndices and counts, CPolyBunch nodes (heap pool), the entity matrix when it has none")]
pub fn cast_shadow_entity_xy(
    ent: GPtr<u8>, _ax: u32, _ay: u32, _bx: u32, _by: u32, pos: GPtr<u8>, front_x: f32, front_y: f32, side_x: f32,
    side_y: f32, intensity: i16, r: u8, gg: u8, b: u8, z_distance: f32, scale: f32, pp_bunch: GPtr<u32>,
    p_day_night: GPtr<u8>, shadow_type: u32,
) {
    let mut guard = g::<u32>(STATIC_GUARD).read();
    for bit in [1u32, 2, 4] {
        if guard & bit == 0 {
            guard |= bit;
            g::<u32>(STATIC_GUARD).write(guard);
        }
    }
    let mi = g::<GPtr<u8>>(MODEL_INFOS + 4 * i32::from(rd::<u16>(ent, 0x22) as i16) as usize).read();
    let cm = rd::<GPtr<u8>>(mi, 0x14);
    let cd = rd::<GPtr<u8>>(cm, 0x2c);
    if cd.is_null() || rd::<u16>(cd, 4) == 0 {
        return;
    }
    calculate_triangle_planes(cm.cast());
    let m1 = get_matrix(ent);
    let m2 = get_matrix(ent);
    let lfx = x(front_y).fmul_m(rd::<f32>(m2, 4)).fadd(x(front_x).fmul_m(rd::<f32>(m1, 0))).to_f32();
    let m1 = get_matrix(ent);
    let m2 = get_matrix(ent);
    let lfy = x(front_x).fmul_m(rd::<f32>(m1, 0x10)).fadd(x(front_y).fmul_m(rd::<f32>(m2, 0x14))).to_f32();
    let m1 = get_matrix(ent);
    let m2 = get_matrix(ent);
    let lsx = x(side_y).fmul_m(rd::<f32>(m2, 4)).fadd(x(side_x).fmul_m(rd::<f32>(m1, 0))).to_f32();
    let m1 = get_matrix(ent);
    let m2 = get_matrix(ent);
    let lsy = x(side_x).fmul_m(rd::<f32>(m1, 0x10)).fadd(x(side_y).fmul_m(rd::<f32>(m2, 0x14))).to_f32();
    let ep = ent_pos(ent);
    let dx = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(ep, 0)).to_f32();
    let ep = ent_pos(ent);
    let dy = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(ep, 4)).to_f32();
    let m1 = get_matrix(ent);
    let m2 = get_matrix(ent);
    let lpx = x(dy).fmul_m(rd::<f32>(m2, 4)).fadd(x(dx).fmul_m(rd::<f32>(m1, 0))).to_f32();
    let m1 = get_matrix(ent);
    let m2 = get_matrix(ent);
    let lpy = x(dy).fmul_m(rd::<f32>(m2, 0x14)).fadd(x(dx).fmul_m(rd::<f32>(m1, 0x10)));
    let q = |i: usize, c: usize| QUAD + 12 * i + 4 * c;
    g::<f32>(q(0, 0)).write(x(lpx).fadd_m(lfx).fsub_m(lsx).to_f32());
    g::<f32>(q(0, 1)).write(x(lfy).fadd(lpy).fsub_m(lsy).to_f32());
    g::<f32>(q(1, 0)).write(x(lpx).fadd_m(lsx).fadd_m(lfx).to_f32());
    g::<f32>(q(1, 1)).write(x(lsy).fadd(lpy).fadd_m(lfy).to_f32());
    let a = x(lpx).fsub_m(lfx);
    g::<f32>(q(2, 0)).write(x(lsx).fadd(a).to_f32());
    let by = lpy.fsub_m(lfy);
    let bys = by.to_f32();
    g::<f32>(q(2, 1)).write(by.fadd_m(lsy).to_f32());
    g::<f32>(q(3, 0)).write(a.fsub_m(lsx).to_f32());
    g::<f32>(q(3, 1)).write(x(bys).fsub_m(lsy).to_f32());
    let ep = ent_pos(ent);
    let lz = x(rd::<f32>(pos, 8)).fsub_m(rd::<f32>(ep, 8)).to_f32();
    let qf = |i: usize, c: usize| gf(q(i, c));
    let min_x = pick_min(pick_min(qf(0, 0), qf(1, 0)), pick_min(qf(2, 0), qf(3, 0)));
    let max_x = pick_max(pick_max(qf(0, 0), qf(1, 0)), pick_max(qf(2, 0), qf(3, 0)));
    let min_y = pick_min(pick_min(qf(0, 1), qf(1, 1)), pick_min(qf(2, 1), qf(3, 1)));
    let max_y = pick_max(pick_max(qf(0, 1), qf(1, 1)), pick_max(qf(2, 1), qf(3, 1)));

    let has_sections = rd::<u8>(cd, 7) & 2 != 0;
    let nsec = if has_sections { rd::<i32>(rd::<GPtr<u8>>(cd, 0x18), -4) } else { 1 };
    let mut sec = 0;
    while sec < nsec {
        let (first, end) = if has_sections {
            let s = rd::<GPtr<u8>>(cd, 0x18).byte_add(-(0x1c * sec as isize) - 0x20);
            sec += 1;
            if jne(x(max_x), x(rd::<f32>(s, 0)), 0x41)
                || jp(x(min_x), x(rd::<f32>(s, 0xc)), 0x5)
                || jne(x(max_y), x(rd::<f32>(s, 4)), 0x41)
                || jp(x(min_y), x(rd::<f32>(s, 0x10)), 0x5)
            {
                continue;
            }
            (i32::from(rd::<u16>(s, 0x18) as i16), i32::from(rd::<u16>(s, 0x1a) as i16) + 1)
        } else {
            sec += 1;
            (0, i32::from(rd::<u16>(cd, 4) as i16))
        };
        let mut tri = first;
        while tri < end {
            one_triangle(ent, cd, tri, (min_x, max_x, min_y, max_y, lz), intensity, (r, gg, b), z_distance, scale, pp_bunch, p_day_night, shadow_type);
            tri += 1;
        }
    }
}

/// One collision triangle (0x708BF0..0x709CB4).
#[allow(clippy::too_many_arguments, clippy::cognitive_complexity)]
fn one_triangle(
    ent: GPtr<u8>, cd: GPtr<u8>, tri: i32, bounds: (f32, f32, f32, f32, f32), intensity: i16, rgb: (u8, u8, u8),
    z_distance: f32, scale: f32, pp_bunch: GPtr<u32>, p_day_night: GPtr<u8>, shadow_type: u32,
) {
    let (min_x, max_x, min_y, max_y, lz) = bounds;
    let plane = rd::<GPtr<u8>>(cd, 0x1c).byte_add(10 * tri as isize);
    // 1/4096 (0x858BFC), 0.1 (0x858B1C)
    let qn = k(0x3980_0000);
    if jne(xi(i32::from(rd::<u16>(plane, 4) as i16)).fmul_m(qn).fabs(), x(k(0x3dcc_cccd)), 0x41) {
        return;
    }
    let t = rd::<GPtr<u8>>(cd, 0x18).byte_add(8 * tri as isize);
    let mut p = [[0f32; 3]; 3];
    for (j, pt) in p.iter_mut().enumerate() {
        get_triangle_point(cd.cast(), GPtr::<f32>::from_raw(pt.as_mut_ptr()).cast(), i32::from(rd::<u16>(t, 2 * j as isize)));
    }
    let all = |f: &dyn Fn(f32) -> bool, c: usize| p.iter().all(|pt| f(pt[c]));
    if all(&|v| jne(x(v), x(min_x), 0x41), 0)
        || all(&|v| jp(x(v), x(max_x), 0x5), 0)
        || all(&|v| jne(x(v), x(min_y), 0x41), 1)
        || all(&|v| jp(x(v), x(max_y), 0x5), 1)
        || all(&|v| jp(x(v), x(lz), 0x5), 2)
    {
        return;
    }
    let low = x(lz).fsub_m(z_distance).to_f32();
    if all(&|v| jne(x(v), x(low), 0x41), 2) {
        return;
    }
    for (i, (u, v)) in [(0.0f32, 0.0f32), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].into_iter().enumerate() {
        g::<u32>(POS_A + 12 * i).write(g::<u32>(QUAD + 12 * i).read());
        g::<u32>(POS_A + 12 * i + 4).write(g::<u32>(QUAD + 12 * i + 4).read());
        g::<f32>(UV_A + 12 * i).write(u);
        g::<f32>(UV_A + 12 * i + 4).write(v);
    }
    let edge = |a: usize, bb: usize| (x(p[bb][0]).fsub_m(p[a][0]), x(p[bb][1]).fsub_m(p[a][1]));
    let n = clip_pass(UV_A, POS_A, 4, UV_B, POS_B, (p[0][0], p[0][1]), edge(0, 1));
    let n = clip_pass(UV_B, POS_B, n, UV_A, POS_A, (p[1][0], p[1][1]), edge(1, 2));
    let n = clip_pass(UV_A, POS_A, n, UV_B, POS_B, (p[2][0], p[2][1]), edge(2, 0));
    if n < 3 {
        return;
    }
    let n = n as usize;
    let nx = xi(i32::from(rd::<u16>(plane, 0) as i16)).fmul_m(qn).to_f32();
    let ny = xi(i32::from(rd::<u16>(plane, 2) as i16)).fmul_m(qn).to_f32();
    let nz = xi(i32::from(rd::<u16>(plane, 4) as i16)).fmul_m(qn);
    let d = x(p[0][2]).fmul(nz).fadd(x(ny).fmul_m(p[0][1])).fadd(x(nx).fmul_m(p[0][0])).to_f32();
    // -1 (0x858C1C)
    let rz = x(k(0xbf80_0000)).fdiv(nz);
    for i in 0..n {
        let pp = POS_B + 12 * i;
        let z = x(ny).fmul_m(gf(pp + 4)).fadd(x(nx).fmul_m(gf(pp))).fsub_m(d).fmul(rz);
        g::<f32>(pp + 8).write(z.to_f32());
    }
    for i in 0..n {
        let pp = POS_B + 12 * i;
        let lx = gf(pp);
        let m = get_matrix(ent);
        let m2 = get_matrix(ent);
        let ep = ent_pos(ent);
        g::<f32>(pp).write(x(rd::<f32>(m, 0x10)).fmul_m(gf(pp + 4)).fadd(x(rd::<f32>(m2, 0)).fmul_m(lx)).fadd_m(rd::<f32>(ep, 0)).to_f32());
        let m = get_matrix(ent);
        let m2 = get_matrix(ent);
        let ep = ent_pos(ent);
        g::<f32>(pp + 4).write(x(rd::<f32>(m, 0x14)).fmul_m(gf(pp + 4)).fadd(x(lx).fmul_m(rd::<f32>(m2, 4))).fadd_m(rd::<f32>(ep, 4)).to_f32());
        let ep = ent_pos(ent);
        g::<f32>(pp + 8).write(x(rd::<f32>(ep, 8)).fadd_m(gf(pp + 8)).to_f32());
    }
    p_day_night.write(rd::<u8>(t, 7));
    if !pp_bunch.is_null() {
        let bunch = g::<GPtr<u8>>(EMPTY_BUNCH_LIST).read();
        if bunch.is_null() {
            return;
        }
        g::<u32>(EMPTY_BUNCH_LIST).write(rd::<u32>(bunch, 0x54));
        wr::<u32>(bunch, 0x54, pp_bunch.read());
        pp_bunch.write(bunch.addr() as u32);
        wr::<u16>(bunch, 0x58, n as u16);
        for i in 0..n {
            for c in 0..3 {
                wr::<u32>(bunch, (12 * i + 4 * c) as isize, g::<u32>(POS_B + 12 * i + 4 * c).read());
            }
            // 200 (0x858A48)
            wr::<u8>(bunch, 0x5a + i as isize, ftol32(x(gf(UV_B + 12 * i)).fmul_m(k(0x4348_0000))) as u8);
            wr::<u8>(bunch, 0x61 + i as isize, ftol32(x(gf(UV_B + 12 * i + 4)).fmul_m(k(0x4348_0000))) as u8);
        }
        return;
    }
    let nidx = 3 * n as i32 - 6;
    let flush = || {
        let nv = g::<u32>(TEMP_VERTICES_STORED).read();
        if nv != 0 && !rw_im3d_transform(TEMP_RENDER_VERTICES as u32, nv, 0, 1).is_null() {
            rw_im3d_render_indexed_primitive(3, TEMP_BUFFER_INDICES as u32, g::<u32>(TEMP_INDICES_STORED).read());
            rw_im3d_end();
        }
        g::<u32>(TEMP_INDICES_STORED).write(0);
        g::<u32>(TEMP_VERTICES_STORED).write(0);
    };
    if g::<i32>(TEMP_INDICES_STORED).read().wrapping_add(nidx) >= 0x1000 {
        flush();
    }
    if g::<i32>(TEMP_VERTICES_STORED).read().wrapping_add(n as i32) >= 0x800 {
        flush();
    }
    let ni = g::<u32>(TEMP_INDICES_STORED).read() as usize;
    let nv = g::<u32>(TEMP_VERTICES_STORED).read() as usize;
    g::<i32>(PENDING_INDICES).write(nidx);
    g::<u32>(PENDING_VERTICES).write(n as u32);
    let mut col = [0u8; 3];
    let cp = GPtr::<u8>::from_raw(col.as_mut_ptr());
    affect_colour_with_lighting(
        shadow_type,
        u32::from(p_day_night.read()),
        u32::from(rgb.0),
        u32::from(rgb.1),
        u32::from(rgb.2),
        cp,
        cp.byte_add(1),
        cp.byte_add(2),
    );
    let colour = ((i32::from(intensity) as u32) << 24) | (u32::from(col[0]) << 16) | (u32::from(col[1]) << 8) | u32::from(col[2]);
    for i in 0..n {
        let v = GPtr::<u8>::from_addr(TEMP_RENDER_VERTICES + 0x24 * (nv + i));
        wr::<u32>(v, 0, g::<u32>(POS_B + 12 * i).read());
        wr::<u32>(v, 4, g::<u32>(POS_B + 12 * i + 4).read());
        // 0.06 (0x859934)
        wr::<f32>(v, 8, x(gf(POS_B + 12 * i + 8)).fadd_m(k(0x3d75_c28f)).to_f32());
        wr::<u32>(v, 0x18, colour);
        wr::<f32>(v, 0x1c, x(scale).fmul_m(gf(UV_B + 12 * i)).to_f32());
        wr::<f32>(v, 0x20, x(scale).fmul_m(gf(UV_B + 12 * i + 4)).to_f32());
    }
    for j in 0..nidx.max(0) as usize {
        g::<u16>(TEMP_BUFFER_INDICES + 2 * (ni + j)).write(g::<u16>(FAN_INDICES + 2 * j).read());
    }
    stop_storing();
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_CAST_SHADOW_ENTITY_XY];
