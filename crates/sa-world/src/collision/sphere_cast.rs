//! Collision casts and copies (batch `mvpf-coll-1`): the collision functions the ped/vehicle collision of the native
//! MVP still reached through the original (`CCollisionData::Copy`, `SphereCastVersusVsPoly`,
//! `SphereCastVsEntity`, `CCollision::ProcessColModels`, `CWorld::ProcessLineOfSightSectorList`).
//! Ported from the 1.0 US bytes (SecuROM fragments inlined, the `push r; call $+5; mov r,[a]; op r,[b];
//! xchg [esp],r; add [esp+4],n; ret` call obfuscation resolved to its target), x87 sequences through the
//! [`Fpu`] model (PC = 24) in the original's operand order; gta-reversed was a reading aid only.
//! Unverified (docs/verify/HOWTO.md).
//!
//! `CCollisionData`: spheres count i16 +0, boxes count i16 +2, triangles count i16 +4, lines/disks
//! count i8 +6, flags +7 (bit 0 uses disks, bit 2 has shadow), spheres +8 (0x14 bytes), boxes +0xC
//! (0x1C bytes), vertices +0x10 lines (0x20) / disks (0x24) ... see the field docs on each function.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::data::{mem_free, mem_malloc};
use super::fpu::Fpu;
use super::mvpc_coll1::local;
use crate::world::common::{g, rd, wr};

/// `1.0f / 4096` (0x858BFC)
const K_1_4096: usize = 0x858BFC;
/// `1.0f / 128` (0x858B88)
const K_1_128: usize = 0x858B88;
/// `0.0f` (0x858B50)
const K_0: usize = 0x858B50;

/// f32 constant at `addr`.
fn kf(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

fn v3(p: GPtr<u8>) -> [f32; 3] {
    [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
}

fn bits3(v: [f32; 3]) -> [u32; 3] {
    [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()]
}

/// `CCollision::ClosestSquaredDistanceBetweenFiniteLines` through its port, as the f64 of its ST0.
pub fn closest_sq_dist_st0(p0: GPtr<f32>, q0: GPtr<f32>, q1: GPtr<f32>, u: GPtr<f32>, uu: f32) -> f64 {
    super::prim::closest_squared_distance_between_finite_lines(p0, q0, q1, u, uu).to_f64()
}

orig! {
    /// `CColBox::operator=(const CColBox&)` (0x40EEC0, thiscall, `ret 4`): copies min, max (6 dwords),
    /// the surface/piece word and the light byte (0x1B bytes).
    fn col_box_assign(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x40eec0 => unported;
    /// `CCollision::PointInPoly(const CVector&, const CColTriangle&, const CVector& normal, const
    /// CVector* verts)` (0x415730, AL tested)
    fn point_in_poly(p: GPtr<f32>, tri: GPtr<u8>, normal: GPtr<f32>, verts: GPtr<f32>) -> u32
        = cdecl 0x415730 => super::prim::point_in_poly;
    /// `CCollision::ClosestSquaredDistanceBetweenFiniteLines(p0, q0, q1, u, uu)` (0x415A40, ST0)
    fn closest_sq_dist(p0: GPtr<f32>, q0: GPtr<f32>, q1: GPtr<f32>, u: GPtr<f32>, uu: f32) -> f64
        = cdecl 0x415a40 => closest_sq_dist_st0;
    /// `CCamera::IsExtraEntityToIgnore(CEntity*)` (0x50CE80, thiscall, AL; ported in sa-cam)
    fn is_extra_entity_to_ignore(cam: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x50ce80 => unported;
    /// `CPlaceable::AllocateMatrix` (0x54F560, thiscall)
    fn allocate_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x54f560 => crate::world::entity::allocate_matrix;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54f1b0 => unported;
    /// `CMatrix::Invert(const CMatrix& src, CMatrix& dst)` (0x59B920, cdecl, returns `dst`)
    fn matrix_invert(src: GPtr<u8>, dst: GPtr<u8>) -> GPtr<u8> = cdecl 0x59b920 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, out first)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59c890 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59acd0 => crate::world::mvpa_math::matrix_dtor;
    /// `CColLine::CColLine(const CVector& start, const CVector& end)` (0x40EF50, `ret 8`)
    fn col_line_ctor(l: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>) -> GPtr<u8> = thiscall 0x40ef50 => crate::world::mvpa_math::col_line_ctor;
    /// `CColSphere::Set(float radius, const CVector& centre, uint8 material, uint8 piece, uint8 light)`
    /// (0x40FD10, thiscall, `ret 0x14`)
    fn col_sphere_set(sp: GPtr<u8>, radius: f32, centre: GPtr<u8>, material: u8, piece: u8, light: u8) = thiscall 0x40fd10 => unported;
    /// `CColBox::Set(const CVector& min, const CVector& max, uint8 material, uint8 piece, uint8 light)`
    /// (0x40EE70, thiscall, `ret 0x14`)
    fn col_box_set(bx: GPtr<u8>, min: GPtr<u8>, max: GPtr<u8>, material: u8, piece: u8, light: u8) = thiscall 0x40ee70 => unported;
    /// `CEntity::GetColModel` (0x535300, thiscall)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => crate::world::entity::get_col_model;
    /// `CCollision::SphereCastVsBBox(CColSphere*, CColSphere*, CColBox*)` (0x415590, AL)
    fn sphere_cast_vs_bbox(a: GPtr<f32>, b: GPtr<f32>, bx: GPtr<f32>) -> u8 = cdecl 0x415590 => super::prim::sphere_cast_vs_bbox;
    /// `CCollision::SphereCastVsSphere(CColSphere*, CColSphere*, CColSphere*)` (0x417F20, AL)
    fn sphere_cast_vs_sphere(a: GPtr<f32>, b: GPtr<f32>, s: GPtr<f32>) -> u8 = cdecl 0x417f20 => super::prim::sphere_cast_vs_sphere;
    /// `CCollision::CalculateTrianglePlanes(CCollisionData*)` (0x416330)
    fn calculate_triangle_planes(cd: super::data::CdPtr) = cdecl 0x416330 => super::data::calculate_triangle_planes_col_data;
    /// `CCollision::SphereCastVersusVsPoly` (0x415CF0, AL)
    fn sphere_cast_versus_vs_poly_call(a: GPtr<u8>, b: GPtr<u8>, tri: GPtr<u8>, plane: GPtr<u8>, verts: GPtr<u8>) -> u8
        = cdecl 0x415cf0 => sphere_cast_versus_vs_poly;
}

/// Copies `n` dwords from `src + off` to `dst + off` (`rep movsd`).
fn copy_dwords(dst: GPtr<u8>, src: GPtr<u8>, off: isize, n: isize) {
    for k in 0..n {
        wr::<u32>(dst, off + 4 * k, rd::<u32>(src, off + 4 * k));
    }
}

/// Frees `this + off` if non-null (the original's direct or obfuscated `CMemoryMgr::Free` call).
fn free_field(this: GPtr<u8>, off: isize) {
    let p = rd::<GPtr<u8>>(this, off);
    if !p.is_null() {
        mem_free(p);
    }
}

/// The highest vertex index of `n` triangles (8 bytes: u16 a, b, c, ...) at `tris`, scanning with
/// an unsigned 16-bit compare from `start` (the original's running `esi`), plus one, as the u16 the
/// original then tests (`inc esi; test si, si`).
fn vertex_count(tris: GPtr<u8>, n: u32, start: u32) -> u32 {
    let mut m = start;
    let mut p = tris;
    let mut k = n;
    while k != 0 {
        for o in [0isize, 2, 4] {
            let c = rd::<u16>(p, o) as u32;
            if c as u16 > m as u16 {
                m = c;
            }
        }
        p = p.byte_add(8);
        k = k.wrapping_sub(1);
    }
    m.wrapping_add(1)
}

/// Copies `n` (u16 from the vertex count) 6-byte `CompressedVector`s (dword + word).
fn copy_verts(dst: GPtr<u8>, src: GPtr<u8>, n: u32) {
    let mut off = 0isize;
    for _ in 0..n {
        wr::<u32>(dst, off, rd::<u32>(src, off));
        wr::<u16>(dst, off + 4, rd::<u16>(src, off + 4));
        off += 6;
    }
}

/// `CCollisionData::Copy(const CCollisionData& src)` (0x40F120, thiscall `ret 4`, returns `this`; a
/// `.HOODLUM` stub).
///
/// Per volume list, in the original's order:
/// - spheres (i16 +0, +8, 0x14 bytes): src count 0 → count 0, the array freed and nulled; otherwise
///   when the counts differ the count is set, the old array freed and `Malloc(count*0x14, 0)`
///   (sign-extended count) taken; then `count` spheres are copied (the loop compares unsigned
///   against the sign-extended count).
/// - lines/disks (i8 +6, +0x10): src count 0 → count 0, freed and nulled. A src with disks (flag +7
///   bit 0) reallocates (`0x24` per disk) when the counts differ or this has no disks; a src with lines
///   (`0x20` per line) when the counts differ or this has disks. Reallocating sets the count first.
///   The elements are copied by dwords (9 / 8).
/// - flags +7: bits 0 and 2 are taken from the src, the others kept.
/// - boxes (i16 +2, +0xC, 0x1C bytes): like spheres, copied with `CColBox::operator=` (0x1B bytes).
/// - triangles (i16 +4, +0x18, 8 bytes) and vertices (+0x14, 6 bytes): src count 0 → count 0, both
///   arrays freed and nulled. Otherwise the vertex count is the highest index the src triangles use,
///   plus one (u16); this' vertex array is always freed (when non-null) and, only when that count is
///   not 0, reallocated and copied; with a count of 0 the freed pointer stays (original bug). The
///   triangle array is reallocated when the counts differ and copied by 8 bytes.
/// - shadow (u32 triangles +0x20, u32 vertices +0x24, vertices +0x28, triangles +0x2C): a src with
///   shadow triangles and flag bit 2 gets the same treatment (vertex count from the highest index;
///   the vertex array freed without nulling), then `+0x24` copied; otherwise both counts are zeroed and
///   both arrays freed and nulled.
#[hook(addr = 0x40f120, conv = thiscall, name = "CCollisionData::Copy", expect = "E9 6B 3B 15 01",
       unverified = "mvpf-coll-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "copies a CCollisionData into a heap block (CColModel::operator=, MakeMultipleAlloc)")]
pub fn col_data_copy(this: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    // spheres
    let ns = rd::<i16>(src, 0);
    if ns == 0 {
        wr::<i16>(this, 0, 0);
        free_field(this, 8);
        wr::<u32>(this, 8, 0);
    } else {
        if rd::<i16>(this, 0) != ns {
            wr::<i16>(this, 0, ns);
            free_field(this, 8);
            let p = mem_malloc((rd::<i16>(this, 0) as i32 * 0x14) as u32, 0);
            wr(this, 8, p);
        }
        let mut k = 0u32;
        while k < rd::<i16>(this, 0) as i32 as u32 {
            copy_dwords(rd(this, 8), rd(src, 8), (k * 0x14) as isize, 5);
            k += 1;
        }
    }
    // lines / disks
    let nl = rd::<u8>(src, 6);
    if nl == 0 {
        wr::<u8>(this, 6, 0);
        free_field(this, 0x10);
        wr::<u32>(this, 0x10, 0);
    } else {
        let cur = rd::<u8>(this, 6);
        let this_disks = rd::<u8>(this, 7) & 1 != 0;
        let (realloc, size, dwords) = if rd::<u8>(src, 7) & 1 != 0 {
            (cur != nl || !this_disks, 0x24, 9)
        } else {
            (cur != nl || this_disks, 0x20, 8)
        };
        if realloc {
            wr::<u8>(this, 6, nl);
            free_field(this, 0x10);
            let p = mem_malloc((rd::<i8>(this, 6) as i32 * size) as u32, 0);
            wr(this, 0x10, p);
        }
        let mut k = 0u32;
        while rd::<u8>(this, 6) != 0 && k < rd::<i8>(this, 6) as i32 as u32 {
            copy_dwords(rd(this, 0x10), rd(src, 0x10), (k as i32 * size) as isize, dwords);
            k += 1;
        }
    }
    // flags: bits 0 and 2 from the src
    let f = rd::<u8>(this, 7);
    let f = f ^ ((rd::<u8>(src, 7) ^ f) & 1);
    let f = f ^ ((rd::<u8>(src, 7) ^ f) & 4);
    wr::<u8>(this, 7, f);
    // boxes
    let nb = rd::<i16>(src, 2);
    if nb == 0 {
        wr::<i16>(this, 2, 0);
        free_field(this, 0xC);
        wr::<u32>(this, 0xC, 0);
    } else {
        if rd::<i16>(this, 2) != nb {
            wr::<i16>(this, 2, nb);
            free_field(this, 0xC);
            let p = mem_malloc((rd::<i16>(this, 2) as i32 * 0x1C) as u32, 0);
            wr(this, 0xC, p);
        }
        let mut k = 0u32;
        while k < rd::<i16>(this, 2) as i32 as u32 {
            let off = (k * 0x1C) as isize;
            col_box_assign(rd::<GPtr<u8>>(this, 0xC).byte_add(off), rd::<GPtr<u8>>(src, 0xC).byte_add(off));
            k += 1;
        }
    }
    // triangles and vertices
    let nt = rd::<i16>(src, 4);
    if nt == 0 {
        wr::<i16>(this, 4, 0);
        free_field(this, 0x18);
        wr::<u32>(this, 0x18, 0);
        free_field(this, 0x14);
        wr::<u32>(this, 0x14, 0);
    } else {
        let nv = vertex_count(rd(src, 0x18), nt as i32 as u32, 0);
        free_field(this, 0x14);
        let nv = nv as u16 as u32;
        if nv != 0 {
            let p = mem_malloc(nv * 6, 0);
            wr(this, 0x14, p);
            copy_verts(p, rd(src, 0x14), nv);
        }
        let nt = rd::<i16>(src, 4);
        if rd::<i16>(this, 4) != nt {
            wr::<i16>(this, 4, nt);
            free_field(this, 0x18);
            let p = mem_malloc((rd::<i16>(this, 4) as i32 * 8) as u32, 0);
            wr(this, 0x18, p);
        }
        let mut k = 0u32;
        while k < rd::<i16>(this, 4) as i32 as u32 {
            copy_dwords(rd(this, 0x18), rd(src, 0x18), (k * 8) as isize, 2);
            k += 1;
        }
    }
    // shadow
    let nst = rd::<u32>(src, 0x20);
    if nst == 0 || rd::<u8>(src, 7) & 4 == 0 {
        wr::<u32>(this, 0x20, 0);
        wr::<u32>(this, 0x24, 0);
        free_field(this, 0x2C);
        wr::<u32>(this, 0x2C, 0);
        free_field(this, 0x28);
        wr::<u32>(this, 0x28, 0);
        return this;
    }
    let nv = vertex_count(rd(src, 0x2C), nst, 0);
    free_field(this, 0x28);
    let nv = nv as u16 as u32;
    if nv != 0 {
        let p = mem_malloc(nv * 6, 0);
        wr(this, 0x28, p);
        copy_verts(p, rd(src, 0x28), nv);
    }
    let nst = rd::<u32>(src, 0x20);
    if rd::<u32>(this, 0x20) != nst {
        wr::<u32>(this, 0x20, nst);
        free_field(this, 0x2C);
        let p = mem_malloc(rd::<u32>(this, 0x20).wrapping_mul(8), 0);
        wr(this, 0x2C, p);
    }
    let mut k = 0u32;
    while k < rd::<u32>(this, 0x20) {
        copy_dwords(rd(this, 0x2C), rd(src, 0x2C), (k * 8) as isize, 2);
        k += 1;
    }
    wr::<u32>(this, 0x24, rd::<u32>(src, 0x24));
    this
}

/// Vertex `k` of `tri` (u16 index, 6-byte `CompressedVector`s at `verts`), each component
/// `fild(i16) * (1/128)` stored as f32.
fn vertex(f: &mut Fpu, verts: GPtr<u8>, tri: GPtr<u8>, k: isize, q128: f32) -> [f32; 3] {
    let v = verts.byte_add(rd::<u16>(tri, 2 * k) as isize * 6);
    let mut out = [0f32; 3];
    for (c, o) in out.iter_mut().zip([0isize, 2, 4]) {
        f.fild(rd::<i16>(v, o) as i32);
        f.fmul_m(q128);
        *c = f.fstp();
    }
    out
}

/// `CCollision::SphereCastVersusVsPoly(const CColSphere& a, const CColSphere& b, const CColTriangle&
/// tri, const CColTrianglePlane& plane, const CompressedVector* verts)` (0x415CF0, AL; `sub esp, 0x78;
/// nop; nop` then a fragment).
///
/// `dir = b.centre - a.centre` (f32 stores), `len2 = (dz*dz + dy*dy) + dx*dx` (dz from the register,
/// times its stored f32), plane normal `n = i16 * (1/4096)`, `dist = ((a.z*n.z + a.y*n.y) + a.x*n.x) -
/// d*(1/128)` kept on the x87 stack. `|dist| <= a.radius` (NaN too): `Q = a.centre - n*dist` (x, y from
/// the register products, z through the stored f32 product). Otherwise `P = a.centre - n*radius`;
/// `num = ((A - P).z*n.z + (A - P).y*n.y) + (A - P).x*n.x` with A the first vertex; `num > 0` → false;
/// `den = (n.z*dz + n.y*dy) + n.x*dx`; unless `num > den` (or NaN) → false; `t = num/den`,
/// `Q = P + dir*t` (x from the register, y and z through f32 products). Then the three vertices go to
/// `PointInPoly(Q, tri, n, verts)`: inside → true (`mov al, 1`). Otherwise `r2 = radius*radius` and
/// `ClosestSquaredDistanceBetweenFiniteLines(a.centre, X, Y, dir, len2) < r2` for the edges (A, B),
/// (C, B), (A, C) in that order: the first that holds → true; none → false (NaN never holds).
#[hook(addr = 0x415cf0, conv = cdecl, name = "CCollision::SphereCastVersusVsPoly", expect = "83 EC 78 90 90",
       route = "L2", hazards = "none", recs = "grove-walk",
       note = "AL result; the early false returns leave AH = the fnstsw status")]
pub fn sphere_cast_versus_vs_poly(a: GPtr<u8>, b: GPtr<u8>, tri: GPtr<u8>, plane: GPtr<u8>, verts: GPtr<u8>) -> u8 {
    let q4096 = kf(K_1_4096);
    let q128 = kf(K_1_128);
    let zero = kf(K_0);
    let mut f = Fpu::new();
    let ac = v3(a);
    let bc = v3(b);
    f.fld(bc[0]);
    f.fsub_m(ac[0]);
    let dx = f.fstp();
    f.fld(bc[1]);
    f.fsub_m(ac[1]);
    let dy = f.fstp();
    f.fld(bc[2]);
    f.fsub_m(ac[2]);
    let dz = f.fst();
    f.fmul_m(dz);
    f.fld(dy);
    f.fmul_m(dy);
    f.faddp(1);
    f.fld(dx);
    f.fmul_m(dx);
    f.faddp(1);
    let len2 = f.fstp();
    let radius = rd::<f32>(a, 0xC);
    let mut n = [0f32; 3];
    for (c, o) in n.iter_mut().zip([0isize, 2, 4]) {
        f.fild(rd::<i16>(plane, o) as i32);
        f.fmul_m(q4096);
        *c = f.fstp();
    }
    f.fld(ac[2]);
    f.fmul_m(n[2]);
    f.fld(ac[1]);
    f.fmul_m(n[1]);
    f.faddp(1);
    f.fld(ac[0]);
    f.fmul_m(n[0]);
    f.faddp(1);
    f.fild(rd::<i16>(plane, 6) as i32);
    f.fmul_m(q128);
    f.fsubp(1);
    let va = vertex(&mut f, verts, tri, 0, q128);
    f.fld_st(0);
    f.fabs();
    f.fcomp_m(radius);
    let q: [f32; 3] = if f.jne(0x41) {
        // touching the plane: project the centre
        f.fld(n[0]);
        f.fmul_st(1);
        f.fld(n[1]);
        f.fmul_st(2);
        f.fld(n[2]);
        f.fmul_st(3);
        let nzd = f.fstp();
        f.fld(ac[0]);
        f.fsub_st(2);
        let px = f.fstp();
        f.fld(ac[1]);
        f.fsub_st(1);
        let py = f.fstp();
        f.fstp_st(0);
        f.fstp_st(0);
        f.fstp_st(0);
        f.fld(ac[2]);
        f.fsub_m(nzd);
        [px, py, f.fstp()]
    } else {
        f.fstp_st(0);
        f.fld(n[0]);
        f.fmul_m(radius);
        f.fld(n[1]);
        f.fmul_m(radius);
        f.fld(n[2]);
        f.fmul_m(radius);
        let nzr = f.fstp();
        f.fld(ac[0]);
        f.fsub_st(2);
        let px = f.fstp();
        f.fld(ac[1]);
        f.fsub_st(1);
        let py = f.fstp();
        f.fstp_st(0);
        f.fstp_st(0);
        f.fld(ac[2]);
        f.fsub_m(nzr);
        let pz = f.fstp();
        f.fld(va[0]);
        f.fsub_m(px);
        f.fld(va[1]);
        f.fsub_m(py);
        f.fld(va[2]);
        f.fsub_m(pz);
        f.fmul_m(n[2]);
        f.fxch(1);
        f.fmul_m(n[1]);
        f.faddp(1);
        f.fxch(1);
        f.fmul_m(n[0]);
        f.faddp(1);
        let num = f.fst();
        f.fcomp_m(zero);
        if !f.jne(0x41) {
            return 0;
        }
        f.fld(n[2]);
        f.fmul_m(dz);
        f.fld(n[1]);
        f.fmul_m(dy);
        f.faddp(1);
        f.fld(n[0]);
        f.fmul_m(dx);
        f.faddp(1);
        f.fld(num);
        f.fcomp_st(1);
        if !f.jp(0x41) {
            return 0;
        }
        f.fdivr_m(num);
        f.fld(dx);
        f.fmul_st(1);
        f.fld(dy);
        f.fmul_st(2);
        let ty = f.fstp();
        f.fld(dz);
        f.fmul_st(2);
        let tz = f.fstp();
        f.fadd_m(px);
        let qx = f.fstp();
        f.fstp_st(0);
        f.fld(ty);
        f.fadd_m(py);
        let qy = f.fstp();
        f.fld(tz);
        f.fadd_m(pz);
        [qx, qy, f.fstp()]
    };
    let vb = vertex(&mut f, verts, tri, 1, q128);
    let vc = vertex(&mut f, verts, tri, 2, q128);
    let mut tri_v = [0u32; 9];
    tri_v[0..3].copy_from_slice(&bits3(va));
    tri_v[3..6].copy_from_slice(&bits3(vb));
    tri_v[6..9].copy_from_slice(&bits3(vc));
    let mut qb = bits3(q);
    let mut nb = bits3(n);
    let mut cb = bits3(ac);
    let mut db = bits3([dx, dy, dz]);
    let vp = local(&mut tri_v);
    if point_in_poly(local(&mut qb).cast(), tri, local(&mut nb).cast(), vp.cast()) as u8 != 0 {
        return 1;
    }
    f.fld(radius);
    f.fmul_m(radius);
    let r2 = f.fstp();
    let cp = local(&mut cb).cast::<f32>();
    let dp = local(&mut db).cast::<f32>();
    let pa = vp.cast::<f32>();
    let pb = vp.byte_add(12).cast::<f32>();
    let pc = vp.byte_add(24).cast::<f32>();
    for (x, y) in [(pa, pb), (pc, pb), (pa, pc)] {
        let d = closest_sq_dist(cp, x, y, dp, len2);
        let mut c = Fpu::new();
        c.push(sa_math::x87::X87::from_f64_exact(d));
        c.fcomp_m(r2);
        if !c.jp(5) {
            return 1;
        }
    }
    0
}

/// `TheCamera` (0xB6F028)
const THE_CAMERA: usize = 0xB6F028;
/// `CCollision::ms_colCache` entries pointer (0x9655CC; 16-byte entries)
const COL_CACHE: usize = 0x9655CC;
/// `CCollision::ms_iColCacheCount` (0x9655D8)
const COL_CACHE_COUNT: usize = 0x9655D8;
/// Byte flag at 0x9655E4: also cast backwards against near-vertical triangles.
const CAST_BACKFACES: usize = 0x9655E4;
/// f32 threshold at 0x858C28 (`|n.z|` below it selects the backwards cast).
const K_BACKFACE_NZ: usize = 0x858C28;

/// One cache record (the block at 0x41A110 and its copies): `count >= 0x63` → full (the caller
/// returns true). Otherwise the entry `cache + count*16` gets type 0, then the entity at +4 for the
/// first record of the call (`hit` set) or 0; the count is re-read, incremented, and that entry gets
/// the index word at `field` and the type.
fn record(cache: GPtr<u8>, hit: &mut u8, entity: GPtr<u8>, ty: u8, field: isize, idx: u16) -> bool {
    let cnt = g::<i32>(COL_CACHE_COUNT);
    let n = cnt.read();
    if n >= 0x63 {
        return true;
    }
    let e = cache.byte_add((n << 4) as isize);
    wr::<u8>(e, 0, 0);
    if *hit == 0 {
        wr(e, 4, entity);
        *hit = 1;
    } else {
        wr::<u32>(e, 4, 0);
    }
    let n = cnt.read();
    let e = cache.byte_add((n << 4) as isize);
    cnt.write(n.wrapping_add(1));
    wr::<u16>(e, field, idx);
    wr::<u8>(e, 0, ty);
    false
}

/// `|fild(plane.nz) * (1/4096)| < [0x858C28]` (`test ah, 5; jp` not taken).
fn near_vertical(plane: GPtr<u8>) -> bool {
    let mut f = Fpu::new();
    f.fild(rd::<i16>(plane, 4) as i32);
    f.fmul_m(kf(K_1_4096));
    f.fabs();
    f.fcomp_m(kf(K_BACKFACE_NZ));
    !f.jp(5)
}

/// The per-call state of [`sphere_cast_vs_entity`]'s triangle casts.
struct TriCast {
    sa: GPtr<u8>,
    sb: GPtr<u8>,
    verts: GPtr<u8>,
    cache: GPtr<u8>,
    entity: GPtr<u8>,
    backfaces: bool,
}

impl TriCast {
    /// One triangle: the forward cast (index `i`), then with the backface flag the backwards cast of a
    /// near-vertical one (index `0xFFFF - i`). True when the cache is full.
    fn cast(&self, i: i32, tri: GPtr<u8>, plane: GPtr<u8>, hit: &mut u8) -> bool {
        if sphere_cast_versus_vs_poly_call(self.sa, self.sb, tri, plane, self.verts) != 0
            && record(self.cache, hit, self.entity, 1, 8, i as u16)
        {
            return true;
        }
        self.backfaces
            && near_vertical(plane)
            && sphere_cast_versus_vs_poly_call(self.sb, self.sa, tri, plane, self.verts) != 0
            && record(self.cache, hit, self.entity, 1, 8, (0xFFFF - i) as u16)
    }
}

/// `CCollision::SphereCastVsEntity(CColSphere* a, CColSphere* b, CEntity* e)` (0x419F00, AL; SEH frame
/// for the local `CMatrix`).
///
/// False unless the entity uses collision (+0x1C bit 0) and `TheCamera.IsExtraEntityToIgnore(e)` is
/// false. Its matrix is allocated and updated from the placement when missing; the inverse goes into a
/// local `CMatrix` (destroyed afterwards). The segment `a.centre .. b.centre` is built as a `CColLine`,
/// both ends are transformed into model space (`inverse * end`, then `inverse * start`) into a second
/// `CColLine`, and two local spheres `SA`, `SB` are set at its ends, both with `a`'s radius (surface 0,
/// piece 0, light 0xFF). No `CCollisionData` → false. A `CColBox` of the col model's bounds must pass
/// `SphereCastVsBBox(SA, SB, box)` (else false).
///
/// Then every primitive the swept sphere touches is appended to the collision cache (`*0x9655CC`,
/// 16-byte entries, count 0x9655D8; at 0x63 entries the call returns true at once): spheres from the
/// last down (`SphereCastVsSphere`, type 2, index +0xA); after `CalculateTrianglePlanes(colData)` the
/// triangles (`SphereCastVersusVsPoly(SA, SB, tri, plane, verts)`, type 1, index +8), through the face
/// groups (+7 bit 1: the dword count before the triangle array, groups of 0x1C bytes before it, each a
/// bbox tested with `SphereCastVsBBox` and a first..last range walked upwards) or else all triangles
/// from the last down; with byte 0x9655E4 set, a triangle whose `|n.z|` is below `[0x858C28]` is also
/// cast backwards (`SB, SA`), recorded as index `0xFFFF - i`; finally the boxes from the last down
/// (`SphereCastVsBBox`, type 3, index +0xC). The first record of the call stores the entity at +4,
/// later ones 0. Returns whether anything was recorded.
#[hook(addr = 0x419f00, conv = cdecl, name = "CCollision::SphereCastVsEntity", expect = "6A FF E9 4E 02 FF FF",
       unverified = "mvpf-coll-1", route = "L2", hazards = "entry-protection-read,heap-writes", recs = "grove-walk,driving",
       note = "camera collision; the no-CD protection reads the entry (0x40102D); writes col cache entries")]
pub fn sphere_cast_vs_entity(a: GPtr<u8>, b: GPtr<u8>, e: GPtr<u8>) -> u8 {
    let mut hit = 0u8;
    if rd::<u8>(e, 0x1C) & 1 == 0 {
        return 0;
    }
    if is_extra_entity_to_ignore(GPtr::from_addr(THE_CAMERA), e) != 0 {
        return 0;
    }
    if rd::<GPtr<u8>>(e, 0x14).is_null() {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    let mut mb = [0u32; 0x48 / 4];
    let m = local(&mut mb);
    matrix_invert(rd::<GPtr<u8>>(e, 0x14), m);
    let mut l0b = [0u32; 8];
    let l0 = local(&mut l0b);
    col_line_ctor(l0, a, b);
    let mut o1b = [0u32; 3];
    let o1 = local(&mut o1b);
    matrix_mul_vec(o1, m, l0.byte_add(0x10));
    let mut o2b = [0u32; 3];
    let o2 = local(&mut o2b);
    matrix_mul_vec(o2, m, l0);
    let mut l1b = [0u32; 8];
    let l1 = local(&mut l1b);
    col_line_ctor(l1, o2, o1);
    let mut sab = [0u32; 5];
    let sa = local(&mut sab);
    let mut sbb = [0u32; 5];
    let sb = local(&mut sbb);
    let mut cb = [0u32; 3];
    let c = local(&mut cb);
    copy_dwords(c, l1, 0, 3);
    col_sphere_set(sa, rd::<f32>(a, 0xC), c, 0, 0, 0xFF);
    for k in 0..3 {
        wr::<u32>(c, 4 * k, rd::<u32>(l1, 0x10 + 4 * k));
    }
    col_sphere_set(sb, rd::<f32>(a, 0xC), c, 0, 0, 0xFF);
    matrix_dtor(m);
    let cm = get_col_model(e);
    let cd = rd::<GPtr<u8>>(cm, 0x2C);
    if cd.is_null() {
        return 0;
    }
    let mut boxb = [0u32; 7];
    let bx = local(&mut boxb);
    col_box_set(bx, cm, cm.byte_add(0xC), 0, 0, 0xFF);
    if sphere_cast_vs_bbox(sa.cast(), sb.cast(), bx.cast()) == 0 {
        return 0;
    }
    let cache = g::<GPtr<u8>>(COL_CACHE).read();
    // spheres, last first
    let mut i = rd::<i16>(cd, 0) as i32 - 1;
    while i >= 0 {
        let sp = rd::<GPtr<u8>>(cd, 8).byte_add(i as isize * 0x14);
        if sphere_cast_vs_sphere(sa.cast(), sb.cast(), sp.cast()) != 0 && record(cache, &mut hit, e, 2, 0xA, i as u16) {
            return 1;
        }
        i -= 1;
    }
    let backfaces = g::<u8>(CAST_BACKFACES).read() != 0;
    let cd2 = rd::<GPtr<u8>>(cm, 0x2C);
    if !cd2.is_null() {
        calculate_triangle_planes(cd2.cast());
    }
    let verts = rd::<GPtr<u8>>(cd, 0x14);
    let tris = rd::<GPtr<u8>>(cd, 0x18);
    let planes = rd::<GPtr<u8>>(cd, 0x1C);
    let tc = TriCast { sa, sb, verts, cache, entity: e, backfaces };
    if rd::<i16>(cd, 4) != 0 {
        if rd::<u8>(cd, 7) & 2 != 0 {
            let mut gi = 0i32;
            let mut off = 0isize;
            loop {
                let n = if rd::<u8>(cd, 7) & 2 != 0 { rd::<i32>(rd::<GPtr<u8>>(cd, 0x18), -4) } else { 0 };
                if gi >= n {
                    break;
                }
                let grp = rd::<GPtr<u8>>(cd, 0x18).byte_add(-off - 0x20);
                let mut gbb = [0u32; 7];
                let gb = local(&mut gbb);
                col_box_set(gb, grp, grp.byte_add(0xC), 0, 0, 0xFF);
                if sphere_cast_vs_bbox(sa.cast(), sb.cast(), gb.cast()) != 0 {
                    let mut i = rd::<i16>(grp, 0x18) as i32;
                    if i <= rd::<i16>(grp, 0x1A) as i32 {
                        let mut plane = planes.byte_add(i as isize * 10);
                        let mut tri = tris.byte_add(i as isize * 8);
                        loop {
                            if tc.cast(i, tri, plane, &mut hit) {
                                return 1;
                            }
                            i += 1;
                            tri = tri.byte_add(8);
                            plane = plane.byte_add(10);
                            if i > rd::<i16>(grp, 0x1A) as i32 {
                                break;
                            }
                        }
                    }
                }
                gi += 1;
                off += 0x1C;
            }
        } else {
            let mut i = rd::<i16>(cd, 4) as i32 - 1;
            let mut tri = tris.byte_add(i as isize * 8);
            let mut plane = planes.byte_add(i as isize * 10);
            while i >= 0 {
                if tc.cast(i, tri, plane, &mut hit) {
                    return 1;
                }
                i -= 1;
                tri = tri.byte_add(-8);
                plane = plane.byte_add(-10);
            }
        }
    }
    // boxes, last first
    let mut i = rd::<i16>(cd, 2) as i32 - 1;
    while i >= 0 {
        let b = rd::<GPtr<u8>>(cd, 0xC).byte_add(i as isize * 0x1C);
        if sphere_cast_vs_bbox(sa.cast(), sb.cast(), b.cast()) != 0 && record(cache, &mut hit, e, 3, 0xC, i as u16) {
            return 1;
        }
        i -= 1;
    }
    hit
}

orig! {
    /// `CColModel::CColModel()` (0x40FB60, thiscall)
    fn col_model_ctor(cm: GPtr<u8>) -> GPtr<u8> = thiscall 0x40fb60 => unported;
    /// `CCollisionData::CCollisionData()` (0x40F030, thiscall)
    fn collision_data_ctor(cd: GPtr<u8>) -> GPtr<u8> = thiscall 0x40f030 => crate::world::mvpa_math::collision_data_ctor;
    /// `CColModel::~CColModel()` (0x40F700, thiscall)
    fn col_model_dtor(cm: GPtr<u8>) = thiscall 0x40f700 => crate::world::mvpa_math::col_model_dtor;
    /// `CColPoint::operator=(const CColPoint&)` (stub 0x40FC80, thiscall `ret 4`)
    fn col_point_assign(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x40fc80 => unported;
    /// `CWorld::CameraToIgnoreThisObject(CEntity*)` (0x563F40, AL)
    fn camera_to_ignore_this_object(e: GPtr<u8>) -> u8 = cdecl 0x563f40 => crate::world::cworld2::camera_to_ignore_this_object;
    /// `CPed::IsAlive()` (0x5E0170, thiscall, AL; ported in sa-ped)
    fn ped_is_alive(ped: GPtr<u8>) -> u8 = thiscall 0x5e0170 => unported;
    /// `CPedModelInfo::AnimatePedColModelSkinned(RpClump*)` (0x4C6F70, thiscall, returns the col model)
    fn animate_ped_col_model_skinned(mi: sa_stream::modelinfo::base::MiPtr, clump: GPtr<u8>) -> GPtr<u8>
        = thiscall 0x4c6f70 => sa_stream::modelinfo::ped::animate_ped_col_model_skinned;
    /// `CCollision::ProcessLineOfSight(const CColLine&, const CMatrix&, CColModel&, CColPoint&, float&,
    /// bool seeThrough, bool shootThrough)` (0x417950, AL)
    fn process_line_of_sight(line: GPtr<u8>, m: GPtr<u8>, cm: GPtr<u8>, cp: GPtr<u8>, max: GPtr<u8>, see: u8, shoot: u8) -> u8
        = cdecl 0x417950 => super::mvpc_coll1::process_line_of_sight;
    /// virtual `CVehicle::SetUpWheelColModel(CColModel*)` (vtable +0xAC, AL)
    fn set_up_wheel_col_model_v(veh: GPtr<u8>, cm: GPtr<u8>) -> u8 = thiscall vslot 0xac => unported;
}

/// `CWorld::ms_nCurrentScanCode` (0xB7CD78, u16)
const SCAN_CODE: usize = 0xB7CD78;
/// `CWorld::pIgnoreEntity` (0xB7CD68)
const IGNORE_ENTITY: usize = 0xB7CD68;
/// `CWorld::bIncludeCarTyres` (0xB7CD70)
const INCLUDE_CAR_TYRES: usize = 0xB7CD70;
/// `CWorld::bIncludeDeadPeds` (0xB7CD71)
const INCLUDE_DEAD_PEDS: usize = 0xB7CD71;
/// `CWorld::bIncludeBikers` (0xB7CD6F)
const INCLUDE_BIKERS: usize = 0xB7CD6F;
/// `CWorld::fWeaponSpreadRate` (0xB7CD64)
const WEAPON_SPREAD_RATE: usize = 0xB7CD64;
/// `CWorld::ms_iProcessLineNumCrossings` (0xB7CD60)
const WORLD_NUM_CROSSINGS: usize = 0xB7CD60;
/// `CCollision::ms_iProcessLineNumCrossings` (0x9655D0)
const COLL_NUM_CROSSINGS: usize = 0x9655D0;
/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8)
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// f32 at 0x858B8C: the minimum `|dot(line, right)| / |line|` of a tyre hit taken from the side.
const K_TYRE_SIDE: usize = 0x858B8C;

/// `entity->GetMatrix()` with the allocate-and-update the original inlines (`AllocateMatrix`, then
/// `CSimpleTransform::UpdateMatrix` from the placement).
fn ensure_matrix(e: GPtr<u8>) -> GPtr<u8> {
    if rd::<GPtr<u8>>(e, 0x14).is_null() {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    rd::<GPtr<u8>>(e, 0x14)
}

/// Adds `d` (`fld d; fadd [p]`) or subtracts it (`fld [p]; fsub d`) to the f32 at `p + off`.
fn adjust(p: GPtr<u8>, off: isize, d: f32, add: bool) {
    let mut f = Fpu::new();
    if add {
        f.fld(d);
        f.fadd_m(rd::<f32>(p, off));
    } else {
        f.fld(rd::<f32>(p, off));
        f.fsub_m(d);
    }
    wr::<f32>(p, off, f.fstp());
}

/// The weapon-spread widening (or its undo, `grow = false`) of a col model: bbox min `-= d` (`fld min;
/// fsub d`), max `+= d` (`fld d; fadd max`), bounding radius +0x24 `+= d`, every sphere radius `+= d`;
/// the undo mirrors the operand forms (`fld d; fadd min`, `fld max; fsub d`, ...).
fn widen(cm: GPtr<u8>, cd: GPtr<u8>, d: f32, grow: bool) {
    for o in [0isize, 4, 8] {
        adjust(cm, o, d, !grow);
    }
    for o in [0xCisize, 0x10, 0x14, 0x24] {
        adjust(cm, o, d, grow);
    }
    let mut i = 0i32;
    while i < rd::<i16>(cd, 0) as i32 {
        let sp = rd::<GPtr<u8>>(cd, 8).byte_add(i as isize * 0x14);
        adjust(sp, 0xC, d, grow);
        i += 1;
    }
}

/// `CWorld::ProcessLineOfSightSectorList(CPtrList& list, const CColLine& line, CColPoint& out, float&
/// maxTouch, CEntity*& outEntity, bool seeThrough, bool ignoreCamera, bool shootThrough)` (0x566EE0, AL;
/// SEH frame for the local `CColModel`).
///
/// Locals: a `CColModel` (ctor 0x40FB60) followed by a `CCollisionData` (ctor 0x40F030), a `CColPoint`
/// and room for 6 spheres; `min = *maxTouch`. With `bIncludeCarTyres` and a vehicle as the list's
/// first entity, the local col model gets the local data (6 spheres at the local array, no
/// boxes/triangles/lines) and the tyre pass is on for the whole list. `bIncludeDeadPeds` /
/// `bIncludeBikers` count only when the first entity is a ped (and then for the whole list).
///
/// Per entity: skipped at the current scan code, when it is `pIgnoreEntity`, without collision (+0x1C
/// bit 0) unless one of the two ped flags is on, or (with `ignoreCamera`) when
/// `CameraToIgnoreThisObject`. It gets the scan code; `min2 = min`. A ped is tested on its skinned col
/// model (`AnimatePedColModelSkinned`) when it uses collision, has +0xFC set, is dead with
/// `bIncludeDeadPeds`, or has +0x478 bit 0x100000 with `bIncludeBikers`; other entities need collision
/// and their `GetColModel`. With a col model and data: when `fWeaponSpreadRate > 0` the model is widened
/// by `|pos - line.start| * rate` (sum order `(x*x + y*y) + z*z`), `ProcessLineOfSight(line, matrix,
/// model, out, min, seeThrough, shootThrough)` sets `outEntity` on a hit and adds
/// `CCollision::ms_iProcessLineNumCrossings` to `CWorld`'s, then the widening is undone in place (the
/// original's float round trip kept). Tyre pass: the vehicle's virtual `SetUpWheelColModel(local)`;
/// then `ProcessLineOfSight(line, matrix, local, localPoint, min2, false, shootThrough)`; on a hit the
/// crossings are added and `min2 < min` takes the point (`min = min2`, `out = localPoint`,
/// `outEntity`); otherwise a hit on the side facing the line (`dot(line dir, right)` and `dot(point -
/// pos, right)` of opposite signs) with `|dot1| / |line| > [0x858B8C]` takes it the same way.
///
/// At the end the local model's data pointer is cleared and it is destroyed; `min < *maxTouch` →
/// `*maxTouch = min`, true.
#[hook(addr = 0x566ee0, conv = cdecl, name = "CWorld::ProcessLineOfSightSectorList<CPtrListDoubleLink<CPhysical*>>", expect = "6A FF 68 0B CD 83 00",
       unverified = "mvpf-coll-1", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "grove-walk,driving",
       note = "FSQRT spread distance; widens and restores col models in place (not bit-exact round trip)")]
#[allow(clippy::too_many_arguments)]
pub fn process_line_of_sight_sector_list(list: GPtr<u8>, line: GPtr<u8>, out: GPtr<u8>, max_touch: GPtr<u8>,
    out_entity: GPtr<u8>, see_through: u8, ignore_camera: u8, shoot_through: u8) -> u8 {
    let zero = kf(K_0);
    // S+0x14 min, S+0x18 min2
    let mut minb = [rd::<u32>(max_touch, 0), 0u32];
    let min_p = local(&mut minb);
    let min2_p = min_p.byte_add(4);
    // S+0x24: CColModel (0x30), CCollisionData (0x30), CColPoint (0x2C), 6 spheres (0x78)
    let mut frame = [0u32; 0x104 / 4];
    let cm_l = local(&mut frame);
    let cd_l = cm_l.byte_add(0x30);
    let cp_l = cm_l.byte_add(0x60);
    let sp_l = cm_l.byte_add(0x8C);
    let mut tyres = false;
    let mut dead_peds = false;
    let mut bikers = false;
    col_model_ctor(cm_l);
    collision_data_ctor(cd_l);
    let mut node = rd::<GPtr<u8>>(list, 0);
    if g::<u8>(INCLUDE_CAR_TYRES).read() != 0 && !node.is_null() && rd::<u8>(rd::<GPtr<u8>>(node, 0), 0x36) & 7 == 2 {
        tyres = true;
        wr(cm_l, 0x2C, cd_l);
        wr(cd_l, 8, sp_l);
        wr::<u16>(cd_l, 0, 6);
        wr::<u8>(cd_l, 6, 0);
        wr::<u16>(cd_l, 2, 0);
        wr::<u16>(cd_l, 4, 0);
    }
    if !node.is_null() {
        let first_ped = rd::<u8>(rd::<GPtr<u8>>(node, 0), 0x36) & 7 == 3;
        dead_peds = g::<u8>(INCLUDE_DEAD_PEDS).read() != 0 && first_ped;
        bikers = g::<u8>(INCLUDE_BIKERS).read() != 0 && first_ped;
    }
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        let scan = g::<u16>(SCAN_CODE).read();
        if rd::<u16>(e, 0x2C) == scan || e == g::<GPtr<u8>>(IGNORE_ENTITY).read() {
            continue;
        }
        let flags = rd::<u8>(e, 0x1C);
        if flags & 1 == 0 && !dead_peds && !bikers {
            continue;
        }
        if ignore_camera != 0 && camera_to_ignore_this_object(e) != 0 {
            continue;
        }
        wr::<u16>(e, 0x2C, g::<u16>(SCAN_CODE).read());
        wr::<u32>(min2_p, 0, rd::<u32>(min_p, 0));
        let cm = if rd::<u8>(e, 0x36) & 7 == 3 {
            let skinned = flags & 1 != 0
                || !rd::<GPtr<u8>>(e, 0xFC).is_null()
                || (dead_peds && ped_is_alive(e) == 0)
                || (bikers && rd::<u32>(e, 0x478) & 0x100000 != 0);
            if skinned {
                let mi = g::<sa_stream::modelinfo::base::MiPtr>(MODEL_INFO_PTRS + rd::<i16>(e, 0x22) as isize as usize * 4).read();
                animate_ped_col_model_skinned(mi, rd::<GPtr<u8>>(e, 0x18))
            } else {
                GPtr::NULL
            }
        } else if flags & 1 != 0 {
            get_col_model(e)
        } else {
            GPtr::NULL
        };
        if !cm.is_null() && !rd::<GPtr<u8>>(cm, 0x2C).is_null() {
            let cd = rd::<GPtr<u8>>(cm, 0x2C);
            let mut spread = 0f32;
            let mut f = Fpu::new();
            f.fld(g::<f32>(WEAPON_SPREAD_RATE).read());
            f.fcomp_m(zero);
            if !f.jne(0x41) {
                let m = rd::<GPtr<u8>>(e, 0x14);
                let pos = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) };
                f.fld(rd::<f32>(pos, 8));
                f.fsub_m(rd::<f32>(line, 8));
                f.fld(rd::<f32>(pos, 4));
                f.fsub_m(rd::<f32>(line, 4));
                f.fld(rd::<f32>(pos, 0));
                f.fsub_m(rd::<f32>(line, 0));
                f.fld_st(0);
                f.fmul_st(1);
                f.fld_st(2);
                f.fmul_st(3);
                f.faddp(1);
                f.fld_st(3);
                f.fmul_st(4);
                f.faddp(1);
                f.fstp_st(3);
                f.fstp_st(0);
                f.fstp_st(0);
                f.fsqrt();
                f.fmul_m(g::<f32>(WEAPON_SPREAD_RATE).read());
                spread = f.fstp();
                widen(cm, cd, spread, true);
            }
            let m = ensure_matrix(e);
            if process_line_of_sight(line, m, cm, out, min_p, see_through, shoot_through) != 0 {
                wr(out_entity, 0, e);
                let w = g::<i32>(WORLD_NUM_CROSSINGS);
                w.write(w.read().wrapping_add(g::<i32>(COLL_NUM_CROSSINGS).read()));
            }
            let mut f = Fpu::new();
            f.fld(spread);
            f.fcomp_m(zero);
            if !f.jne(0x41) {
                widen(cm, cd, spread, false);
            }
        }
        if !tyres || set_up_wheel_col_model_v(e, cm_l) == 0 {
            continue;
        }
        let m = ensure_matrix(e);
        if process_line_of_sight(line, m, cm_l, cp_l, min2_p, 0, shoot_through) == 0 {
            continue;
        }
        let w = g::<i32>(WORLD_NUM_CROSSINGS);
        w.write(w.read().wrapping_add(g::<i32>(COLL_NUM_CROSSINGS).read()));
        let mut f = Fpu::new();
        f.fld(rd::<f32>(min2_p, 0));
        f.fcomp_m(rd::<f32>(min_p, 0));
        if !f.jp(5) {
            wr::<u32>(min_p, 0, rd::<u32>(min2_p, 0));
            col_point_assign(out, cp_l);
            wr(out_entity, 0, e);
            continue;
        }
        // a tyre hit from the side
        let m = ensure_matrix(e);
        f.fld(rd::<f32>(line, 0x18));
        f.fsub_m(rd::<f32>(line, 8));
        f.fld(rd::<f32>(line, 0x14));
        f.fsub_m(rd::<f32>(line, 4));
        f.fld(rd::<f32>(line, 0x10));
        f.fsub_m(rd::<f32>(line, 0));
        f.fxch(1);
        f.fmul_m(rd::<f32>(m, 4));
        f.fxch(2);
        f.fmul_m(rd::<f32>(m, 8));
        f.faddp(2);
        f.fmul_m(rd::<f32>(m, 0));
        f.faddp(1);
        let dot1 = f.fstp();
        let m = ensure_matrix(e);
        let pos = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) };
        f.fld(rd::<f32>(cp_l, 8));
        f.fsub_m(rd::<f32>(pos, 8));
        f.fld(rd::<f32>(cp_l, 4));
        f.fsub_m(rd::<f32>(pos, 4));
        f.fld(rd::<f32>(cp_l, 0));
        f.fsub_m(rd::<f32>(pos, 0));
        f.fxch(1);
        f.fmul_m(rd::<f32>(m, 4));
        f.fxch(2);
        f.fmul_m(rd::<f32>(m, 8));
        f.faddp(2);
        f.fld_st(0);
        f.fmul_m(rd::<f32>(m, 0));
        f.faddp(2);
        f.fstp_st(0);
        // st0 = dot2
        f.fld(dot1);
        f.fcomp_m(zero);
        let mut take = false;
        let mut decided = false;
        if !f.jp(5) {
            // dot1 < 0
            f.fcom_m(zero);
            if !f.jne(0x41) {
                // dot2 > 0
                f.fstp_st(0);
                take = true;
                decided = true;
            }
        }
        if !decided {
            f.fld(dot1);
            f.fcomp_m(zero);
            if f.jne(0x41) {
                f.fstp_st(0);
                continue;
            }
            f.fcomp_m(zero);
            if f.jp(5) {
                continue;
            }
            take = true;
        }
        if take {
            f.fld(rd::<f32>(line, 0x18));
            f.fsub_m(rd::<f32>(line, 8));
            f.fld(rd::<f32>(line, 0x14));
            f.fsub_m(rd::<f32>(line, 4));
            f.fld(rd::<f32>(line, 0x10));
            f.fsub_m(rd::<f32>(line, 0));
            f.fld_st(0);
            f.fmul_st(1);
            f.fld_st(2);
            f.fmul_st(3);
            f.faddp(1);
            f.fld_st(3);
            f.fmul_st(4);
            f.faddp(1);
            f.fstp_st(3);
            f.fstp_st(0);
            f.fstp_st(0);
            f.fsqrt();
            f.fld(dot1);
            f.fabs();
            f.fdiv_st(1);
            f.fcomp_m(kf(K_TYRE_SIDE));
            f.fstp_st(0);
            if f.jne(0x41) {
                continue;
            }
            wr::<u32>(min_p, 0, rd::<u32>(min2_p, 0));
            col_point_assign(out, cp_l);
            wr(out_entity, 0, e);
        }
    }
    wr::<u32>(cm_l, 0x2C, 0);
    let mut f = Fpu::new();
    f.fld(rd::<f32>(min_p, 0));
    f.fcomp_m(rd::<f32>(max_touch, 0));
    if !f.jp(5) {
        wr::<u32>(max_touch, 0, rd::<u32>(min_p, 0));
        col_model_dtor(cm_l);
        return 1;
    }
    col_model_dtor(cm_l);
    0
}

pub const HOOKS: &[HookReg] = &[
    HOOK_COL_DATA_COPY,
    HOOK_SPHERE_CAST_VERSUS_VS_POLY,
    HOOK_SPHERE_CAST_VS_ENTITY,
    HOOK_PROCESS_LINE_OF_SIGHT_SECTOR_LIST,
];
