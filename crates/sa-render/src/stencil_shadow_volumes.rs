//! Stencil shadow volume builders (batch `mvpm-render-5`): `CStencilShadows::RenderForVehicle` and
//! `CStencilShadows::RenderForObject`. Ported from the 1.0 US bytes (`docs/re/specs/RENDER/CStencilShadows.md`; the two
//! bodies run the same x87 sequence and differ only in the extrusion length and the bike lean matrix). Unverified
//! (docs/verify/HOWTO.md).
//!
//! `CStencilShadowObject`: +0 owner, +4 i16 face count, +8 vertex capacity, +0xC vertices written, +0x10 vertex array
//! (CVector). The edge list (u16 pairs, `s_ShadowTrianglePointsUnk` 0xC6A170) is toggled through the register helper
//! `ToggleEdge` 0x70FA70.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, wr, x};

orig! {
    /// `CEntity::GetColModel()` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*) const` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CBike::CalculateLeanMatrix()` (0x6B7150)
    fn bike_calculate_lean_matrix(b: GPtr<u8>) = thiscall 0x6B7150 => unported;
    /// `Invert(const CMatrix& src, CMatrix& dst)` (0x59B920, cdecl)
    fn matrix_invert(src: GPtr<u8>, dst: GPtr<u8>) -> GPtr<u8> = cdecl 0x59B920 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, hidden result pointer first)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CCollisionData::GetShadTrianglePoint(CVector& out, int32 i)` (0x40F640, `ret 8`)
    fn col_data_get_shad_triangle_point(cd: GPtr<u8>, out: GPtr<u8>, i: i32) = thiscall 0x40F640 => unported;
    /// `TransformPoints(RwV3d* out, int32 n, const CMatrix&, const RwV3d* in)` (0x54EEF0, cdecl)
    fn transform_points(out: GPtr<u8>, n: i32, m: GPtr<u8>, inp: GPtr<u8>) = cdecl 0x54EEF0 => unported;
    /// `ToggleEdge` (0x70FA70): edges in ECX, the u16 count pointer in EDX, vertex a in DI, vertex b on the stack
    fn toggle_edge(edges: GPtr<u8>, count: GPtr<u16>, a: u16, b: u16) = reg("args = ecx edx di stack; keep = ecx edx") 0x70FA70 => crate::harness5::toggle_edge;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
}

/// `s_ShadowTrianglePointsUnk` (edge pairs, 0xC6A170), `s_ShadowTrianglePoints` (0xC6A174),
/// `s_TransformedShadowTrianglePoints` (0xC6A178), `s_SunPosNrm` (CVector, 0x8D5244)
const EDGES: usize = 0xC6A170;
const POINTS: usize = 0xC6A174;
const TPOINTS: usize = 0xC6A178;
const SUN: usize = 0x8D5244;
/// 60.0f (0x858B34): the object extrusion length
const K_OBJECT_LEN: usize = 0x858B34;

fn v3(p: GPtr<u8>) -> [f32; 3] {
    [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
}
fn put(dst: GPtr<u8>, v: [f32; 3]) {
    for (k, c) in v.iter().enumerate() {
        wr::<u32>(dst, 4 * k as isize, c.to_bits());
    }
}
fn pt(base: u32, i: u16) -> GPtr<u8> {
    GPtr::from_addr(base.wrapping_add(u32::from(i) * 12) as usize)
}

/// The shared body (spec steps 2-8): `len` is the extrusion length (a stack local for vehicles, the global 60.0f for
/// objects), `bike` enables the bike lean matrix.
fn build_volume(obj: GPtr<u8>, len: f32, bike: bool) -> u8 {
    let e = rd::<GPtr<u8>>(obj, 0);
    let cd = rd::<GPtr<u8>>(entity_get_col_model(e), 0x2C);
    if cd.is_null() || rd::<i32>(cd, 0x20) != i32::from(rd::<i16>(obj, 4)) {
        return 0;
    }
    if rd::<u32>(e, 0x14) == 0 {
        placeable_allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    let mut m = rd::<GPtr<u8>>(e, 0x14);
    if bike && rd::<u8>(e, 0x36) & 7 == 2 && rd::<u32>(e, 0x590) == 9 {
        bike_calculate_lean_matrix(e);
        m = e.byte_add(0x5CC);
    }
    let mut inv = [0u32; 0x12];
    let invp: GPtr<u8> = GPtr::from_raw(inv.as_mut_ptr()).cast();
    matrix_invert(m, invp);
    let sun = v3(GPtr::from_addr(SUN));
    let mut zero = [0f32; 3];
    let mut sunv = sun;
    let mut o = [0f32; 3];
    let mut s = [0f32; 3];
    let op = matrix_mul_vec(GPtr::from_raw(o.as_mut_ptr()).cast(), invp, GPtr::from_raw(zero.as_mut_ptr()).cast());
    let o = v3(op);
    let sp = matrix_mul_vec(GPtr::from_raw(s.as_mut_ptr()).cast(), invp, GPtr::from_raw(sunv.as_mut_ptr()).cast());
    let s = v3(sp);
    wr::<u32>(obj, 0xC, 0);
    let dz = x(s[2]).fsub_m(o[2]).to_f32();
    let dy = x(s[1]).fsub_m(o[1]).to_f32();
    let dx = x(s[0]).fsub_m(o[0]).to_f32();
    let mut edge_count: u16 = 0;
    let ecp: GPtr<u16> = GPtr::from_raw(&mut edge_count);
    let edges = g::<GPtr<u8>>(EDGES).read();
    let pts = g::<u32>(POINTS).read();
    let tpts = g::<u32>(TPOINTS).read();
    let nv = rd::<i32>(cd, 0x24);
    for i in 0..nv {
        col_data_get_shad_triangle_point(cd, GPtr::from_addr(pts.wrapping_add((i as u32).wrapping_mul(12)) as usize), i);
    }
    transform_points(GPtr::from_addr(tpts as usize), nv, m, GPtr::from_addr(pts as usize));
    let faces = i32::from(rd::<i16>(obj, 4));
    let ext = |l: f32| [x(sun[0]).fmul_m(l), x(sun[1]).fmul_m(l), x(sun[2]).fmul_m(l)];
    let append = |vs: &[[f32; 3]]| -> bool {
        let n = rd::<i32>(obj, 0xC);
        if n + vs.len() as i32 >= rd::<i32>(obj, 8) {
            return false;
        }
        let base = rd::<u32>(obj, 0x10).wrapping_add((n as u32).wrapping_mul(12));
        for (k, v) in vs.iter().enumerate() {
            put(GPtr::from_addr(base.wrapping_add(12 * k as u32) as usize), *v);
        }
        wr::<i32>(obj, 0xC, n + vs.len() as i32);
        true
    };
    let mut j = 0;
    while j < faces {
        let tri = GPtr::<u8>::from_addr(rd::<u32>(cd, 0x2C).wrapping_add((j as u32) * 8) as usize);
        let (a, b, c) = (rd::<u16>(tri, 0), rd::<u16>(tri, 2), rd::<u16>(tri, 4));
        let (pa, pb, pc) = (v3(pt(pts, a)), v3(pt(pts, b)), v3(pt(pts, c)));
        let e1z = x(pb[2]).fsub_m(pa[2]);
        let e1y = x(pb[1]).fsub_m(pa[1]);
        let e1x = x(pb[0]).fsub_m(pa[0]);
        let e2z = x(pc[2]).fsub_m(pa[2]);
        let e2y = x(pc[1]).fsub_m(pa[1]).to_f32();
        let e2x = x(pc[0]).fsub_m(pa[0]).to_f32();
        let nx = e2z.fmul(e1y).fsub(x(e2y).fmul(e1z)).to_f32();
        let ny = x(e2x).fmul(e1z).fsub(e2z.fmul(e1x)).to_f32();
        let nz = x(e2y).fmul(e1x).fsub(x(e2x).fmul(e1y));
        let dot = x(dz).fmul(nz).fadd(x(ny).fmul_m(dy)).fadd(x(nx).fmul_m(dx));
        let (ta, tb, tc) = (v3(pt(tpts, a)), v3(pt(tpts, b)), v3(pt(tpts, c)));
        if !dot.fcom(X87::zero()).test_any(1) {
            toggle_edge(edges, ecp, a, b);
            toggle_edge(edges, ecp, b, c);
            toggle_edge(edges, ecp, c, a);
            if !append(&[ta, tc, tb]) {
                break;
            }
        } else {
            let [ex, ey, ez] = ext(len);
            let (fx, fy, fz) = (ex.to_f32(), ey.to_f32(), ez.to_f32());
            let va = [x(fx).fadd_m(ta[0]).to_f32(), x(ta[1]).fadd_m(fy).to_f32(), x(ta[2]).fadd_m(fz).to_f32()];
            let vb = [x(tb[0]).fadd_m(fx).to_f32(), x(tb[1]).fadd_m(fy).to_f32(), ez.fadd_m(tb[2]).to_f32()];
            let vc = [x(tc[0]).fadd_m(fx).to_f32(), x(tc[1]).fadd_m(fy).to_f32(), x(tc[2]).fadd_m(fz).to_f32()];
            if !append(&[va, vc, vb]) {
                break;
            }
        }
        j += 1;
    }
    // Silhouette edges.
    let [ex, ey, ez] = ext(len);
    let (fx, fy, fz) = (ex.to_f32(), ey.to_f32(), ez.to_f32());
    for k in 0..u32::from(edge_count) {
        let ea = rd::<u16>(edges, (k * 4) as isize);
        let eb = rd::<u16>(edges, (k * 4 + 2) as isize);
        let (va, vb) = (v3(pt(tpts, ea)), v3(pt(tpts, eb)));
        let pa = [x(fx).fadd_m(va[0]).to_f32(), x(va[1]).fadd_m(fy).to_f32(), x(fz).fadd_m(va[2]).to_f32()];
        let pb = [x(vb[0]).fadd_m(fx).to_f32(), x(vb[1]).fadd_m(fy).to_f32(), x(vb[2]).fadd_m(fz).to_f32()];
        if !append(&[va, vb, pa, vb, pb, pa]) {
            break;
        }
    }
    matrix_dtor(invp);
    1
}

/// `CStencilShadows::RenderForVehicle(CStencilShadowObject*)` (0x70FAE0, cdecl), AL.
///
/// Extrusion length 40 for an owner that is a vehicle of type 3 or 4 (+0x594), else 5 (stack local). Fails (AL = 0)
/// without collision data or when its shadow triangle count (+0x20) differs from the object's face count. The owner's
/// matrix (allocated when missing; a bike (+0x590 == 9) uses its lean matrix +0x5CC after `CalculateLeanMatrix`) is
/// inverted into a local CMatrix; the sun direction in object space is `inv * s_SunPosNrm - inv * 0` (dz, dy, dx). The
/// shadow vertices are fetched (`GetShadTrianglePoint`) and transformed to world space; each face with `dot(n, sun) >= 0`
/// (n from the f32 edge differences; `test ah, 1`) toggles its three edges and adds its world triangle `a, c, b`; a back
/// face adds the triangle extruded by `s_SunPosNrm * L` (world space). Then every silhouette edge adds the quad `Va, Vb,
/// A, Vb, B, A`. A full buffer (`count + n >= capacity`) ends the face loop or the edge loop. The local matrix is
/// destroyed; AL = 1.
#[hook(addr = 0x70fae0, conv = cdecl, name = "CStencilShadows::RenderForVehicle", expect = "64 A1 00 00 00 00",
       unverified = "mvpm-render-5", route = "L2", hazards = "ax-al-return,heap-writes", recs = "driving,grove-walk",
       note = "ToggleEdge is a register-convention callee (ECX, EDX, DI, stack); the inverse CMatrix lives on the Rust stack")]
pub fn render_for_vehicle(obj: GPtr<u8>) -> u8 {
    let e = rd::<GPtr<u8>>(obj, 0);
    let big = rd::<u8>(e, 0x36) & 7 == 2 && matches!(rd::<u32>(e, 0x594), 3 | 4);
    let len = if big { 40.0f32 } else { 5.0 };
    build_volume(obj, len, true)
}

/// `CStencilShadows::RenderForObject(CStencilShadowObject*)` (0x710310, cdecl), AL: [`render_for_vehicle`] without the
/// bike branch and with the extrusion length 60.0f read from 0x858B34.
#[hook(addr = 0x710310, conv = cdecl, name = "CStencilShadows::RenderForObject", expect = "6A FF 68 D8 8B 84 00",
       unverified = "mvpm-render-5", route = "L2", hazards = "ax-al-return,heap-writes", recs = "grove-walk",
       note = "ToggleEdge is a register-convention callee (ECX, EDX, DI, stack); the inverse CMatrix lives on the Rust stack")]
pub fn render_for_object(obj: GPtr<u8>) -> u8 {
    build_volume(obj, g::<f32>(K_OBJECT_LEN).read(), false)
}

pub const HOOKS: &[HookReg] = &[HOOK_RENDER_FOR_VEHICLE, HOOK_RENDER_FOR_OBJECT];
