//! `CPedGeometryAnalyser::ComputeEntityBoundingBoxCornersUncached` (batch `intro-ai-1`): the 2D bounding rectangle
//! (four corners at a given height) peds walk round. Ported from the 1.0 US bytes; every x87 value that stays on
//! the register stack between instructions is an `X87`, every value the original stores to its frame is an `f32`, in
//! the original's order (gta-reversed rewrote the corner arithmetic; the bytes win). Unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal, clippy::many_single_char_names, clippy::similar_names, clippy::needless_range_loop)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `InteriorManager_c::GetBoundingBox(CEntity*, CVector* corners)` (0x598090, `ret 8`)
    fn interior_manager_get_bounding_box(m: GPtr<u8>, e: GPtr<u8>, corners: GPtr<u8>) -> u8 = thiscall 0x598090 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(p: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CVector::FromMultiply(const CMatrix&, const CVector&)` (0x59C670, `ret 8`): `this = m * v` (with position)
    fn vector_from_multiply(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x59C670 => unported;
    /// `CVector::Normalise` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
}

/// `g_interiorMan` (`InteriorManager_c`)
const INTERIOR_MANAGER: usize = 0xBAF670;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// 6.0: the bounding-box height above which an object's boxes and spheres are used
const K_TALL: usize = 0x86C6A8;
/// 1.0: the height band around `zPos`
const K_BAND: usize = 0x858624;
/// `CPedGeometryAnalyser::ms_fPedNominalRadius` (0.35)
const PED_NOMINAL_RADIUS: usize = 0x8D22B0;
/// 0.5
const K_HALF: usize = 0x858B8C;
/// 0.0
const K_ZERO: usize = 0x858B50;

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn v3(p: GPtr<u8>) -> [f32; 3] {
    [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
}

fn ptr(v: &mut [f32; 3]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

/// x87 `min` as the loops do it: `fld cand; fld cur; fcomp st(1)`, `test ah, 5` / `jp` keeps the candidate unless
/// the current value is below it; the kept register value is stored as f32.
fn keep_min(cur: f32, cand: X87) -> f32 {
    if x(cur).fcom(cand).test_parity(0x05) { f32_of(cand) } else { cur }
}

/// x87 `max`: `test ah, 0x41` / `jne` keeps the candidate unless the current value is above it.
fn keep_max(cur: f32, cand: X87) -> f32 {
    if x(cur).fcom(cand).test_any(0x41) { f32_of(cand) } else { cur }
}

/// The height test of one transformed box: skipped when both world z values are below `zPos - 1` or both above
/// `zPos + 1` (x87 compares as the bytes: `test ah, 5` / `jp`, then `test ah, 0x41` / `jne`).
fn outside_band(z1: f32, z2: f32, zmin: f32, zmax: f32) -> bool {
    let below = |z: f32| !x(z).fcom(x(zmin)).test_parity(0x05);
    let above = |z: f32| !x(z).fcom(x(zmax)).test_any(0x41);
    (below(z1) && below(z2)) || (above(z1) && above(z2))
}

/// x87 `|v|` as `fcom 0.0; test ah, 5; jp; fchs`: negated when below zero.
fn x87_abs(v: X87) -> X87 {
    if v.fcom(x(gf(K_ZERO))).test_parity(0x05) { v } else { v.fchs() }
}

/// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCornersUncached(float zPos, CEntity&, CVector corners[4])`
/// (0x5F1FA0, cdecl, returns 1 in AL). Read from the bytes:
///
/// - A temporary building (type 1, flag 0x400000 at +0x1C) whose `g_interiorMan.GetBoundingBox(entity, corners)`
///   succeeds: the four corner z values = `zPos`, return 1.
/// - The matrix (allocated and updated from the placement when missing). The model's collision model (+0x14 of the
///   model info) and its data (+0x2C). An object (type 4) with data, a collision bounding box taller than 6 and at
///   least one box or sphere: `min` / `max` start at ±`FLT_MAX`; every box (data +0xC, 0x1C bytes, count +2) and
///   every sphere's box (data +8, 0x14 bytes, count +0; `centre ∓ radius`) whose transformed min and max z are not
///   both below `zPos - 1` or both above `zPos + 1` widens them in model space. Else `min` / `max` = the collision
///   bounding box.
/// - `min -= 0.35`, `max += 0.35` (`ms_fPedNominalRadius`); `h` = the half extents, the 2D squared lengths of the
///   matrix's right / forward / up vectors times `2 h` pick the axis `V` (right when its value is above both others,
///   else forward when above up, else up); the model-space centre goes to world space (`FromMultiply`). The two end
///   points `centre ± V·h` along that axis, `V = (axis.x, axis.y, 0)` normalised, and the other two axes' extents
///   projected on `V` and its perpendicular give `extU` / `extV`; the corners are `end ∓ ...` in the original's
///   order and precision (see the code).
/// - Every corner's z = `zPos`; return 1. No NULL checks on the collision model or data (as the original).
#[hook(addr = 0x5f1fa0, conv = cdecl, name = "CPedGeometryAnalyser::ComputeEntityBoundingBoxCornersUncached",
       expect = "90 E9 FA 1A E1 FF", unverified = "intro-ai-1", route = "L2", hazards = "none",
       recs = "grove-walk", note = "long x87 chains: values kept unrounded on the register stack as the original")]
#[allow(clippy::too_many_lines)]
pub fn ped_geometry_compute_entity_bbox_corners_uncached(z_pos: f32, entity: GPtr<u8>, corners: GPtr<u8>) -> u8 {
    if rd::<u8>(entity, 0x36) & 7 == 1
        && rd::<u32>(entity, 0x1C) & 0x40_0000 != 0
        && interior_manager_get_bounding_box(GPtr::from_addr(INTERIOR_MANAGER), entity, corners) != 0
    {
        for i in 0..4 {
            wr::<f32>(corners, 8 + 0xC * i, z_pos);
        }
        return 1;
    }
    if rd::<GPtr<u8>>(entity, 0x14).is_null() {
        placeable_allocate_matrix(entity);
        simple_transform_update_matrix(entity.byte_add(4), rd::<GPtr<u8>>(entity, 0x14));
    }
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * (i32::from(rd::<i16>(entity, 0x22)) as usize)).read();
    let col = rd::<GPtr<u8>>(mi, 0x14);
    let data = rd::<GPtr<u8>>(col, 0x2C);
    let tall = !x(rd::<f32>(col, 0x14)).fsub_m(rd::<f32>(col, 8)).fcom(x(gf(K_TALL))).test_any(0x41);
    let m = rd::<GPtr<u8>>(entity, 0x14);
    let mut min: [f32; 3];
    let mut max: [f32; 3];
    let n_boxes = || i32::from(rd::<i16>(data, 2));
    let n_spheres = || i32::from(rd::<i16>(data, 0));
    if rd::<u8>(entity, 0x36) & 7 == 4 && !data.is_null() && tall && (n_boxes() != 0 || n_spheres() != 0) {
        let zmin = f32_of(x(z_pos).fsub_m(gf(K_BAND)));
        let zmax = f32_of(x(z_pos).fadd_m(gf(K_BAND)));
        min = [f32::MAX; 3];
        max = [-f32::MAX; 3];
        let mut o1 = [0f32; 3];
        let mut o2 = [0f32; 3];
        let mut i = 0;
        while i < n_boxes() {
            let b = rd::<GPtr<u8>>(data, 0xC).byte_add(0x1C * i as isize);
            vector_from_multiply(ptr(&mut o1), m, b);
            vector_from_multiply(ptr(&mut o2), m, b.byte_add(0xC));
            if !outside_band(o1[2], o2[2], zmin, zmax) {
                for k in 0..3 {
                    min[k] = keep_min(min[k], x(rd::<f32>(b, 4 * k as isize)));
                }
                for k in 0..3 {
                    max[k] = keep_max(max[k], x(rd::<f32>(b, 0xC + 4 * k as isize)));
                }
            }
            i += 1;
        }
        let mut i = 0;
        while i < n_spheres() {
            let s = rd::<GPtr<u8>>(data, 8).byte_add(0x14 * i as isize);
            let c = v3(s);
            let r = rd::<f32>(s, 0xC);
            let mut bmin = [0f32; 3];
            let mut bmax = [0f32; 3];
            for k in 0..3 {
                bmin[k] = f32_of(x(c[k]).fsub_m(r));
            }
            for k in 0..3 {
                bmax[k] = f32_of(x(c[k]).fadd_m(r));
            }
            vector_from_multiply(ptr(&mut o1), m, ptr(&mut bmin));
            vector_from_multiply(ptr(&mut o2), m, ptr(&mut bmax));
            if !outside_band(o1[2], o2[2], zmin, zmax) {
                for k in 0..3 {
                    let r = rd::<f32>(s, 0xC);
                    min[k] = keep_min(min[k], x(rd::<f32>(s, 4 * k as isize)).fsub_m(r));
                }
                for k in 0..3 {
                    let r = rd::<f32>(s, 0xC);
                    max[k] = keep_max(max[k], x(r).fadd_m(rd::<f32>(s, 4 * k as isize)));
                }
            }
            i += 1;
        }
    } else {
        min = v3(col);
        max = v3(col.byte_add(0xC));
    }

    // 0x5F23D3: the nominal radius; min.x / min.y and max.x stay on the x87 stack.
    let pr = gf(PED_NOMINAL_RADIUS);
    let a = x(min[0]).fsub_m(pr);
    let b = x(min[1]).fsub_m(pr);
    min[2] = f32_of(x(min[2]).fsub_m(pr));
    let d = x(pr).fadd_m(max[0]);
    max[1] = f32_of(x(pr).fadd_m(max[1]));
    max[2] = f32_of(x(pr).fadd_m(max[2]));
    let len2 = |o: isize| {
        let vx = x(rd::<f32>(m, o));
        let vy = x(rd::<f32>(m, o + 4));
        f32_of(vx.fmul(vx).fadd(vy.fmul(vy)))
    };
    let len_right = len2(0);
    let len_fwd = len2(0x10);
    let len_up = len2(0x20);
    let half = gf(K_HALF);
    let hx = f32_of(d.fsub(a).fmul_m(half));
    let hy = f32_of(x(max[1]).fsub(b).fmul_m(half));
    let hz = f32_of(x(max[2]).fsub_m(min[2]).fmul_m(half));
    let sx = f32_of(d.fadd(a));
    let sy = x(max[1]).fadd(b);
    let sz = x(max[2]).fadd_m(min[2]);
    let mut centre = [f32_of(x(sx).fmul_m(half)), f32_of(sy.fmul_m(half)), f32_of(sz.fmul_m(half))];
    let mut wc = [0f32; 3];
    vector_from_multiply(ptr(&mut wc), m, ptr(&mut centre));

    let ext_r = x(hx).fmul_m(len_right).fadd(x(hx).fmul_m(len_right));
    let ext_f = {
        let t = x(hy).fmul_m(len_fwd);
        f32_of(t.fadd(t))
    };
    let ext_u = {
        let t = x(hz).fmul_m(len_up);
        f32_of(t.fadd(t))
    };
    // Axis choice (0x5F2503): right when above both, else forward when above up, else up.
    let axis = if !ext_r.fcom(x(ext_f)).test_any(0x41) && !ext_r.fcom(x(ext_u)).test_any(0x41) {
        0
    } else if !x(ext_f).fcom(x(ext_u)).test_any(0x41) {
        1
    } else {
        2
    };
    let (ax_off, h_axis) = match axis {
        0 => (0, hx),
        1 => (0x10, hy),
        _ => (0x20, hz),
    };
    // The end points and the axis vector V (0x5F2521 / 0x5F26A5 / 0x5F27CB).
    let vx = rd::<f32>(m, ax_off);
    let vy = rd::<f32>(m, ax_off + 4);
    let p1 = x(vx).fmul_m(h_axis);
    let p2 = x(vy).fmul_m(h_axis);
    let zr = x(h_axis).fmul_m(gf(K_ZERO));
    let zf = f32_of(zr);
    let p1f = f32_of(p1);
    let e1 = [f32_of(x(p1f).fadd_m(wc[0])), f32_of(x(wc[1]).fadd(p2)), f32_of(x(wc[2]).fadd(zr))];
    let e2 = [f32_of(x(wc[0]).fsub_m(p1f)), f32_of(x(wc[1]).fsub(p2)), f32_of(x(wc[2]).fsub_m(zf))];
    let mut v = [vx, vy, 0.0];
    vector_normalise(ptr(&mut v));
    let (nx, ny, nz) = (v[0], v[1], v[2]);
    // Projections of the other two axes on V and its perpendicular: dot = (Vx·a.x + Vy·a.y) · h, cross = (Vx·a.y −
    // Vy·a.x) · h, in the operand order of each branch.
    let dot = |o: isize, h: f32, y_first: bool| {
        let s = if y_first {
            x(ny).fmul_m(rd::<f32>(m, o + 4)).fadd(x(nx).fmul_m(rd::<f32>(m, o)))
        } else {
            x(nx).fmul_m(rd::<f32>(m, o)).fadd(x(ny).fmul_m(rd::<f32>(m, o + 4)))
        };
        s.fmul_m(h)
    };
    let cross = |o: isize, h: f32| x(nx).fmul_m(rd::<f32>(m, o + 4)).fsub(x(ny).fmul_m(rd::<f32>(m, o))).fmul_m(h);
    let (q1, q2, q3, q4) = match axis {
        0 => (dot(0x10, hy, false), f32_of(cross(0x10, hy)), f32_of(dot(0x20, hz, false)), f32_of(cross(0x20, hz))),
        1 => (dot(0, hx, true), f32_of(cross(0, hx)), f32_of(dot(0x20, hz, false)), f32_of(cross(0x20, hz))),
        _ => (dot(0, hx, true), f32_of(cross(0, hx)), f32_of(dot(0x10, hy, false)), f32_of(cross(0x10, hy))),
    };
    // 0x5F28EC: the extents along V (u) and its perpendicular (w).
    let neg_vx = f32_of(x(nx).fchs());
    let ext_v_axis = f32_of(x87_abs(x(q3)).fadd(x87_abs(q1)));
    let ext_w = f32_of(x87_abs(x(q4)).fadd(x87_abs(x(q2))));
    let w1 = x(ext_w).fmul_m(ny);
    let w1f = f32_of(w1);
    let w2 = x(neg_vx).fmul_m(ext_w);
    let w3 = x(ext_w).fmul_m(gf(K_ZERO));
    let w3f = f32_of(w3);
    let w4 = x(ext_v_axis).fmul_m(nx);
    let w4f = f32_of(w4);
    let w5 = x(ext_v_axis).fmul_m(ny);
    let w5f = f32_of(w5);
    let w6f = f32_of(x(nz).fmul_m(ext_v_axis));

    let put = |i: isize, c: [f32; 3]| {
        for (k, v) in c.iter().enumerate() {
            wr::<f32>(corners, 0xC * i + 4 * k as isize, *v);
        }
    };
    let t1x = f32_of(w4.fadd_m(e1[0]));
    let t1y = x(e1[1]).fadd(w5);
    let t1z = f32_of(x(e1[2]).fadd_m(w6f));
    put(0, [f32_of(x(t1x).fsub_m(w1f)), f32_of(t1y.fsub(w2)), f32_of(x(t1z).fsub(w3))]);
    let u_x = f32_of(x(e2[0]).fsub_m(w4f));
    let u_y = x(e2[1]).fsub_m(w5f);
    let u_z = f32_of(x(e2[2]).fsub_m(w6f));
    put(1, [f32_of(x(u_x).fsub(w1)), f32_of(u_y.fsub(w2)), f32_of(x(u_z).fsub_m(w3f))]);
    put(2, [f32_of(x(u_x).fadd(w1)), f32_of(u_y.fadd(w2)), f32_of(x(u_z).fadd_m(w3f))]);
    let t3x = f32_of(x(w4f).fadd_m(e1[0]));
    let t3y = x(w5f).fadd_m(e1[1]);
    let t3z = f32_of(x(w6f).fadd_m(e1[2]));
    put(3, [f32_of(x(t3x).fadd_m(w1f)), f32_of(t3y.fadd(w2)), f32_of(x(t3z).fadd_m(w3f))]);
    for i in 0..4 {
        wr::<f32>(corners, 8 + 0xC * i, z_pos);
    }
    1
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PED_GEOMETRY_COMPUTE_ENTITY_BBOX_CORNERS_UNCACHED];
