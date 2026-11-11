//! WRITER-MVP-E batch `mvpe-coll-1`: the sphere/triangle and line/box primitives CJ's world collision
//! runs every frame (`TestSphereTriangle`, `ProcessSphereTriangle`, `ProcessLineBox`). Ported from the
//! 1.0 US bytes (SecuROM fragments inlined), x87 sequences through the [`Fpu`] model (PC = 24) in the
//! original's operand order; gta-reversed `Collision.cpp` was a reading aid only. Unverified
//! (docs/verify/HOWTO.md).
//!
//! Layouts: `CColSphere` centre +0, radius +0xC, surface +0x10, piece +0x11, light +0x12. `CColBox`
//! min +0, max +0xC, surface +0x18, piece +0x19, light +0x1A. `CColTriangle` u16 a, b, c, surface +6.
//! `CColTrianglePlane` i16 normal x, y, z (1/4096), i16 distance (1/128). `CompressedVector` 3 x i16
//! (1/128). `CColLine` start +0, end +0x10. `CColPoint` point +0, normal +0x10, surface A +0x20,
//! piece A +0x21, light A +0x22, surface B +0x23, piece B +0x24, light B +0x25, depth +0x28.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use super::fpu::Fpu;
use super::mvpc_coll1::local;
use crate::world::common::{g, rd, wr};

/// `CCollision::ms_iProcessLineNumCrossings`
const NUM_CROSSINGS: usize = 0x9655D0;
/// `1.0f / 4096` (0x858BFC)
const K_1_4096: u32 = 0x858BFC;
/// `1.0f / 128` (0x858B88)
const K_1_128: u32 = 0x858B88;
/// `0.0f` (0x858B50)
const K_0: u32 = 0x858B50;
/// `1.0f` (0x858624)
const K_1: u32 = 0x858624;

orig! {
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): `out = a - b`
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => crate::world::mvpa_math::vector_sub;
    /// `operator+(const CVector&, const CVector&)` (0x40FE30, out first)
    fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => crate::world::mvpa_math::vector_add;
    /// `operator*(const CVector&, float)` (0x40FEC0, out first)
    fn vector_scale(out: GPtr<u8>, v: GPtr<u8>, s: f32) -> GPtr<u8> = cdecl 0x40FEC0 => crate::world::mvpa_math::vector_mul_scalar;
    /// `CVector::Magnitude` (0x4082C0, thiscall, ST0)
    fn magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CVector::CrossProduct(CVector* out, const CVector* a, const CVector* b)` (0x59C730)
    fn cross_product(out: GPtr<CVector>, a: GPtr<CVector>, b: GPtr<CVector>) -> GPtr<CVector>
        = cdecl 0x59C730 => super::data::cross_product_port;
    /// `CVector::Normalise` (0x59C910, thiscall)
    fn normalise(v: GPtr<CVector>) = thiscall 0x59C910 => super::data::normalise_port;
    /// `CompressedVector` -> `CVector` (0x411590, cdecl `(CVector* out, const CompressedVector*)`)
    fn uncompress_vector(out: GPtr<u8>, v: GPtr<u8>) -> i32 = cdecl 0x411590 => crate::world::mvpa_math::uncompress_vector;
    /// `CCollision::CalculateColPointInsideBox(const CBox&, const CVector&, CColPoint&)` (0x411EC0)
    fn calculate_col_point_inside_box(bx: GPtr<f32>, p: GPtr<f32>, cp: GPtr<f32>)
        = cdecl 0x411EC0 => super::prim::calculate_col_point_inside_box;
    /// `CPed::IsPlayer` (0x5DF8F0, thiscall, AL; ported in sa-ped)
    fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CEntity::GetBoundCentre(CVector& out)` (0x534250, thiscall, returns `&out`)
    fn get_bound_centre(e: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x534250 => crate::world::entity::get_bound_centre_vec;
    /// `FUN_0050D590(CPed*)` (cdecl, ST0; animates the skinned ped col model and returns a height)
    fn ped_col_fn_50d590(ped: GPtr<u8>) -> f64 = cdecl 0x50D590 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, thiscall)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => crate::world::entity::register_reference;
    /// `CPedIntelligence::GetTaskHold(bool)` (0x600FF0, thiscall; ported in sa-ai)
    fn get_task_hold(intel: GPtr<u8>, ignore: u8) -> GPtr<u8> = thiscall 0x600FF0 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => crate::world::cworld::find_player_vehicle;
    /// `CEntity::DoesNotCollideWithFlyers` (0x532D40, thiscall, AL)
    fn does_not_collide_with_flyers(e: GPtr<u8>) -> u8 = thiscall 0x532D40 => crate::world::entity::does_not_collide_with_flyers;
    /// `CColSphere::Set(float radius, const CVector& centre, uint8 material, uint8 piece, uint8 light)`
    /// (0x40FD10, thiscall, `ret 0x14`)
    fn col_sphere_set(sp: GPtr<u8>, radius: f32, centre: GPtr<u8>, material: u8, piece: u8, light: u8) = thiscall 0x40FD10 => unported;
    /// `CCollision::SphereCastVsEntity(const CColSphere&, const CColSphere&, CEntity*)` (0x419F00, AL)
    fn sphere_cast_vs_entity(a: GPtr<u8>, b: GPtr<u8>, e: GPtr<u8>) -> u8 = cdecl 0x419F00 => unported;
    /// `CCollision::IsThisVehicleSittingOnMe(CVehicle*, CVehicle*)` (0x4152C0, AL)
    fn is_this_vehicle_sitting_on_me(veh: GPtr<u8>, on_me: GPtr<u8>) -> u8 = cdecl 0x4152C0 => super::prim::is_this_vehicle_sitting_on_me;
    /// `CCollision::TestSphereSphere(const CColSphere&, const CColSphere&)` (0x411E70, AL)
    fn test_sphere_sphere(a: GPtr<f32>, b: GPtr<f32>) -> u32 = cdecl 0x411E70 => super::prim::test_sphere_sphere;
    /// `CWorld::CameraToIgnoreThisObject(CEntity*)` (0x563F40, AL)
    fn camera_to_ignore_this_object(e: GPtr<u8>) -> u8 = cdecl 0x563F40 => crate::world::cworld2::camera_to_ignore_this_object;
    /// `CCollision::GetBoundingBoxFromTwoSpheres(CColBox*, const CColSphere&, const CColSphere&)` (0x415230)
    fn get_bounding_box_from_two_spheres(out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>)
        = cdecl 0x415230 => super::prim::get_bounding_box_from_two_spheres;
    /// CRT `floor(double)` (0x8219F0, ST0)
    fn crt_floor(v: f64) -> f64 = cdecl 0x8219F0 => unported;
    /// `CWorld::ClearScanCodes` (0x563470)
    fn clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => crate::world::cworld::find_player_ped;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740, thiscall; ported in sa-ai)
    fn find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CTaskComplexEnterCar::GetCameraAvoidVehicle` (0x63A690, thiscall)
    fn get_camera_avoid_vehicle(task: GPtr<u8>) -> GPtr<u8> = thiscall 0x63A690 => unported;
    /// `CWorld::FindPlayerSpeed(int32)` (0x56E090, returns `CVector*`)
    fn find_player_speed(id: i32) -> GPtr<u8> = cdecl 0x56E090 => crate::world::cworld::find_player_speed;
    /// `CCollision::SphereCastVsCaches(CColSphere*, CVector*, int32, CColCacheEntry*, int32*,
    /// CColCacheEntry*)` (0x4181B0, AL)
    fn sphere_cast_vs_caches_call(sp: GPtr<u8>, v: GPtr<u8>, n: i32, cache: GPtr<u8>, n_out: GPtr<u8>, out: GPtr<u8>) -> u8
        = cdecl 0x4181B0 => sphere_cast_vs_caches;
    /// `CCamera::IsExtraEntityToIgnore(CEntity*)` (0x50CE80, thiscall, AL; ported in sa-cam)
    fn is_extra_entity_to_ignore(cam: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x50CE80 => unported;
    /// `CPlaceable::AllocateMatrix` (0x54F560, thiscall)
    fn allocate_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x54F560 => crate::world::entity::allocate_matrix;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CMatrix::Invert(const CMatrix& src, CMatrix& dst)` (0x59B920, cdecl, returns `dst`)
    fn matrix_invert(src: GPtr<u8>, dst: GPtr<u8>) -> GPtr<u8> = cdecl 0x59B920 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CEntity::GetColModel` (0x535300, thiscall)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => crate::world::entity::get_col_model;
    /// `CCollision::TestSphereBox(const CColSphere&, const CBox&)` (0x4120C0, AL)
    fn test_sphere_box(s: GPtr<f32>, bx: GPtr<f32>) -> u8 = cdecl 0x4120C0 => super::prim::test_sphere_box;
    /// `CCollision::CalculateTrianglePlanes(CCollisionData*)` (0x416330)
    fn calculate_triangle_planes(cd: super::data::CdPtr) = cdecl 0x416330 => super::data::calculate_triangle_planes_col_data;
    /// `CCollision::TestSphereTriangle` (0x4165B0, AL)
    fn test_sphere_triangle_call(sp: GPtr<u8>, verts: GPtr<u8>, tri: GPtr<u8>, plane: GPtr<u8>) -> u8
        = cdecl 0x4165B0 => test_sphere_triangle;
    /// `CColLine::CColLine(const CVector& start, const CVector& end)` (0x40EF50, `ret 8`)
    fn col_line_ctor(l: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>) -> GPtr<u8> = thiscall 0x40EF50 => crate::world::mvpa_math::col_line_ctor;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, out first)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CCollision::SphereCastVsBBox(CColSphere*, CColSphere*, CColBox*)` (0x415590, AL)
    fn sphere_cast_vs_bbox(a: GPtr<f32>, b: GPtr<f32>, bx: GPtr<f32>) -> u8 = cdecl 0x415590 => super::prim::sphere_cast_vs_bbox;
    /// `CCollision::SphereCastVsSphere(CColSphere*, CColSphere*, CColSphere*)` (0x417F20, AL)
    fn sphere_cast_vs_sphere(a: GPtr<f32>, b: GPtr<f32>, s: GPtr<f32>) -> u8 = cdecl 0x417F20 => super::prim::sphere_cast_vs_sphere;
    /// `CCollision::SphereCastVersusVsPoly(CColSphere*, CColSphere*, CColTriangle*, CColTrianglePlane*,
    /// CompressedVector*)` (0x415CF0, AL)
    fn sphere_cast_versus_vs_poly(a: GPtr<u8>, b: GPtr<u8>, tri: GPtr<u8>, plane: GPtr<u8>, verts: GPtr<u8>) -> u8
        = cdecl 0x415CF0 => unported;
}

/// f32 constant at `addr`.
fn kf(addr: u32) -> f32 {
    g::<f32>(addr as usize).read()
}

/// `fild word-as-dword; fmul [k]; fstp` of the i16 at `p + o`.
fn scaled(f: &mut Fpu, p: GPtr<u8>, o: isize, k: f32) -> f32 {
    f.fild(rd::<i16>(p, o) as i32);
    f.fmul_m(k);
    f.fstp()
}

/// Vertex `k` (0..3) of `tri`, uncompressed inline (`fild * 1/128`, stored f32 per component).
fn vertex_inline(f: &mut Fpu, verts: GPtr<u8>, tri: GPtr<u8>, k: isize, q128: f32) -> [f32; 3] {
    let v = verts.byte_add(rd::<u16>(tri, 2 * k) as isize * 6);
    [scaled(f, v, 0, q128), scaled(f, v, 2, q128), scaled(f, v, 4, q128)]
}

/// Writes three f32s into a fresh local (a stack vector of the original).
fn vec_local(b: &mut [u32; 3], v: [f32; 3]) -> GPtr<u8> {
    for (d, x) in b.iter_mut().zip(v) {
        *d = x.to_bits();
    }
    local(b)
}

fn v3(p: GPtr<u8>) -> [f32; 3] {
    [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
}

/// `Magnitude(VectorSub(tmp, sphere, v))`: the distance from the sphere centre to a vertex, as the
/// original computes it in its switch arms (the stack temporary it subtracts into is not reproduced).
fn dist_to(sphere: GPtr<u8>, v: [f32; 3]) -> X87 {
    let mut vb = [0u32; 3];
    let vp = vec_local(&mut vb, v);
    let mut ob = [0u32; 3];
    let r = vector_sub(local(&mut ob), sphere, vp);
    X87::from_f64_exact(magnitude(r))
}

/// The common front of `TestSphereTriangle` / `ProcessSphereTriangle` after the plane test: edge
/// direction `d = (v1 - v0) / |v1 - v0|`, `perp = CrossProduct(d, n)`, the third vertex and the sphere
/// centre projected onto (d, perp) and the three edge-side tests.
struct TriFrame {
    /// `|v1 - v0|`
    len: f32,
    /// `(v2 - v0) . d`, `(v2 - v0) . perp`
    p2u: f32,
    p2v: f32,
    /// `(c - v0) . d`, `(c - v0) . perp`
    cu: f32,
    cv: f32,
    /// `cv*len - cu*0` (stored f32)
    t1v: f32,
    /// `cu*p2v`, `cv*p2u` (stored f32)
    cu_p2v: f32,
    cv_p2u: f32,
    /// `p2u - len`, `cu - len` (stored f32)
    p2u_len: f32,
    cu_len: f32,
    t1: bool,
    t2: bool,
    t3: bool,
}

/// `TestSphereTriangle`'s edge frame (0x416741..0x41690B): `|e1| = sqrt((z*z + y*y) + x*x)` (z through
/// its stored f32), `d = e1 * (1/|e1|)`, `perp = CrossProduct(d, n)`, v2 projected as
/// `(e2y*dy + e2x*dx) + e2z*dz` and `(e2x*px + pz*e2z) + py*e2y`, the centre as
/// `(cx*dx + cz*dz) + cy*dy` and `(cx*px + cz*pz) + cy*py`, then the three edge-side tests.
fn tri_frame(f: &mut Fpu, e1: [X87; 3], v0: [f32; 3], v2: [f32; 3], c: [f32; 3], n: [f32; 3]) -> TriFrame {
    let one = kf(K_1);
    let zero = kf(K_0);
    // on entry: nothing on the stack; e1 = (x, y, z) unrounded register values, z already stored
    f.push(e1[0]);
    f.push(e1[1]);
    f.push(e1[2]);
    let e1z = f.fst();
    f.fmul_m(e1z);
    f.fld_st(1);
    f.fmul_st(2);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fsqrt();
    let len = f.fstp();
    f.fld(one);
    f.fdiv_m(len);
    let inv = f.fstp();
    f.fxch(1);
    f.fmul_m(inv);
    let dx = f.fstp();
    f.fmul_m(inv);
    let dy = f.fstp();
    f.fld(e1z);
    f.fmul_m(inv);
    let dz = f.fstp();
    let mut db = [0u32; 3];
    let mut nb = [0u32; 3];
    let mut pb = [0u32; 3];
    let dp = vec_local(&mut db, [dx, dy, dz]);
    let np = vec_local(&mut nb, n);
    cross_product(local(&mut pb).cast(), dp.cast(), np.cast());
    let [px, py, pz] = [f32::from_bits(pb[0]), f32::from_bits(pb[1]), f32::from_bits(pb[2])];
    // v2 - v0
    f.fld(v2[0]);
    f.fsub_m(v0[0]);
    f.fld(v2[1]);
    f.fsub_m(v0[1]);
    f.fld(v2[2]);
    f.fsub_m(v0[2]); // st0 = e2z, st1 = e2y, st2 = e2x
    f.fld_st(1);
    f.fmul_m(dy);
    f.fld_st(3);
    f.fmul_m(dx);
    f.faddp(1);
    f.fld_st(1);
    f.fmul_m(dz);
    f.faddp(1);
    let p2u = f.fstp();
    f.fxch(2);
    f.fmul_m(px);
    f.fld(pz);
    f.fmul_st(3);
    f.faddp(1);
    f.fld(py);
    f.fmul_st(2);
    f.faddp(1);
    let p2v = f.fstp();
    f.fstp_st(1);
    f.fstp_st(0);
    // c - v0
    f.fld(c[0]);
    f.fsub_m(v0[0]);
    f.fld(c[1]);
    f.fsub_m(v0[1]);
    f.fld(c[2]);
    f.fsub_m(v0[2]); // st0 = cz, st1 = cy, st2 = cx
    f.fld_st(2);
    f.fmul_m(dx);
    f.fld_st(1);
    f.fmul_m(dz);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_m(dy);
    f.faddp(1);
    let cu = f.fstp();
    f.fxch(2);
    f.fmul_m(px);
    f.fxch(2);
    f.fmul_m(pz);
    f.faddp(2);
    f.fmul_m(py);
    f.faddp(1);
    let cv = f.fstp();
    // edge tests (`test ah, 1; jne`: counted unless below zero; NaN counts as below)
    f.fld(cv);
    f.fmul_m(len);
    f.fld(cu);
    f.fmul_m(zero);
    f.fsubp(1);
    let t1v = f.fst();
    f.fcomp_m(zero);
    let t1 = !f.jne(1);
    f.fld(cu);
    f.fmul_m(p2v);
    let cu_p2v = f.fstp();
    f.fld(cv);
    f.fmul_m(p2u);
    let cv_p2u = f.fstp();
    f.fld(cu_p2v);
    f.fsub_m(cv_p2u);
    f.fcomp_m(zero);
    let t2 = !f.jne(1);
    f.fld(p2u);
    f.fsub_m(len);
    let p2u_len = f.fstp();
    f.fld(cu);
    f.fsub_m(len);
    let cu_len = f.fstp();
    f.fld(p2u_len);
    f.fmul_m(cv);
    f.fld(cu_len);
    f.fmul_m(p2v);
    f.fsubp(1);
    f.fcomp_m(zero);
    let t3 = !f.jne(1);
    TriFrame { len, p2u, p2v, cu, cv, t1v, cu_p2v, cv_p2u, p2u_len, cu_len, t1, t2, t3 }
}

/// `CCollision::TestSphereTriangle(const CColSphere& sphere, const CompressedVector* verts,
/// const CColTriangle& tri, const CColTrianglePlane& plane)` (0x4165B0, AL; `nop; jmp` into a
/// fragment holding `sub esp, 0xC8`). Plane normal `n = i16 * (1/4096)` (stored f32), signed distance
/// `dist = (n.x*c.x + n.z*c.z) + n.y*c.y - d*(1/128)`; `|dist| <= radius` (NaN passes) or false. The
/// vertices are uncompressed inline; edge 0 direction `d = (v1 - v0) * (1 / |v1 - v0|)`,
/// `perp = CrossProduct(d, n)`; v2 and the centre are projected onto `(d, perp)` and three edge-side
/// tests count how many edges the centre is inside of (`< 0` or NaN → outside). Count 0 → false;
/// 3 → `|dist|` (stored f32) `< radius`; 1 → distance to the vertex shared by the two outside edges
/// (`Magnitude(VectorSub(c, v))`); 2 → the distance to the outside edge (the parameter `t` along it
/// clamped to its ends through the vertex distance, else `sqrt(perp_dist^2 + dist^2)`). The final
/// compare is `distance < radius` (`test ah, 5; jp`; NaN false).
#[hook(addr = 0x4165B0, conv = cdecl, name = "CCollision::TestSphereTriangle", expect = "90 E9 D1 AE FE FF",
       unverified = "mvpe-coll-1", route = "L2", hazards = "x87-transcendental", recs = "grove-walk,driving",
       note = "FSQRT results compared bit-exactly against the radius")]
pub fn test_sphere_triangle(sphere: GPtr<u8>, verts: GPtr<u8>, tri: GPtr<u8>, plane: GPtr<u8>) -> u8 {
    let q4096 = kf(K_1_4096);
    let q128 = kf(K_1_128);
    let zero = kf(K_0);
    let one = kf(K_1);
    let mut f = Fpu::new();
    let nx = scaled(&mut f, plane, 0, q4096);
    let ny = scaled(&mut f, plane, 2, q4096);
    let nz = scaled(&mut f, plane, 4, q4096);
    let c = v3(sphere);
    let radius = rd::<f32>(sphere, 0xC);
    f.fld(nx);
    f.fmul_m(c[0]);
    f.fld(nz);
    f.fmul_m(c[2]);
    f.faddp(1);
    f.fld(ny);
    f.fmul_m(c[1]);
    f.faddp(1);
    f.fild(rd::<i16>(plane, 6) as i32);
    f.fmul_m(q128);
    f.fsubp(1);
    let dist = f.fst();
    f.fabs();
    let adist = f.fst();
    f.fcomp_m(radius);
    if !f.jne(0x41) {
        return 0;
    }
    let v0 = vertex_inline(&mut f, verts, tri, 0, q128);
    let v1 = vertex_inline(&mut f, verts, tri, 1, q128);
    let v2 = vertex_inline(&mut f, verts, tri, 2, q128);
    let e1 = [
        X87::from_f32(v1[0]).fsub_m(v0[0]),
        X87::from_f32(v1[1]).fsub_m(v0[1]),
        X87::from_f32(v1[2]).fsub_m(v0[2]),
    ];
    let t = tri_frame(&mut f, e1, v0, v2, c, [nx, ny, nz]);
    let n = t.t1 as u32 + t.t2 as u32 + t.t3 as u32;
    match n {
        0 => return 0,
        3 => f.fld(adist),
        1 => {
            let v = if t.t1 {
                v2
            } else if t.t2 {
                v1
            } else {
                v0
            };
            f.push(dist_to(sphere, v));
        }
        _ => {
            if !t.t1 {
                // outside edge 0 (v0 -> v1)
                f.fld(t.len);
                f.fmul_m(t.len);
                let len2 = f.fstp();
                f.fld(t.cu);
                f.fmul_m(t.len);
                f.fld(t.cv);
                f.fmul_m(zero);
                f.faddp(1);
                f.fdiv_m(len2);
                f.fcom_m(zero);
                if !f.jp(0x41) {
                    f.fstp_st(0);
                    f.push(dist_to(sphere, v0));
                } else {
                    f.fcomp_m(one);
                    if !f.jne(1) {
                        f.push(dist_to(sphere, v1));
                    } else {
                        f.fld(len2);
                        f.fsqrt();
                        f.fdivr_m(t.t1v);
                        edge_tail(&mut f, dist);
                    }
                }
            } else if !t.t2 {
                // outside edge 2 (v0 -> v2)
                f.fld(t.p2v);
                f.fmul_m(t.p2v);
                f.fld(t.p2u);
                f.fmul_m(t.p2u);
                f.faddp(1);
                let l2 = f.fstp();
                f.fld(t.cv);
                f.fmul_m(t.p2v);
                f.fld(t.cu);
                f.fmul_m(t.p2u);
                f.faddp(1);
                f.fdiv_m(l2);
                f.fcom_m(zero);
                if !f.jp(0x41) {
                    f.fstp_st(0);
                    f.push(dist_to(sphere, v0));
                } else {
                    f.fcomp_m(one);
                    if !f.jne(1) {
                        f.push(dist_to(sphere, v2));
                    } else {
                        f.fld(t.cv_p2u);
                        f.fsub_m(t.cu_p2v);
                        f.fld(l2);
                        div_sqrt_tail(&mut f, dist);
                    }
                }
            } else {
                // outside edge 1 (v1 -> v2)
                f.fld(t.p2v);
                f.fmul_m(t.p2v);
                f.fld(t.p2u_len);
                f.fmul_m(t.p2u_len);
                f.faddp(1);
                let l2 = f.fstp();
                f.fld(t.cu_len);
                f.fmul_m(t.p2u_len);
                f.fld(t.cv);
                f.fmul_m(t.p2v);
                f.faddp(1);
                f.fdiv_m(l2);
                f.fcom_m(zero);
                if !f.jp(0x41) {
                    f.fstp_st(0);
                    f.push(dist_to(sphere, v1));
                } else {
                    f.fcomp_m(one);
                    if !f.jne(1) {
                        f.push(dist_to(sphere, v2));
                    } else {
                        f.fld(t.cv);
                        f.fmul_m(t.p2u_len);
                        f.fld(t.cu_len);
                        f.fmul_m(t.p2v);
                        f.fsubp(1);
                        f.fld(l2);
                        div_sqrt_tail(&mut f, dist);
                    }
                }
            }
        }
    }
    f.fcomp_m(radius);
    if f.jp(5) {
        return 0;
    }
    1
}

/// `fsqrt; fxch; fxch; fdivp st1` (st1 / sqrt(st0)) then [`edge_tail`].
fn div_sqrt_tail(f: &mut Fpu, dist: f32) {
    f.fsqrt();
    f.fdivp(1);
    edge_tail(f, dist);
}

/// `fld st0; fmulp st1; fld [dist]; fmul [dist]; faddp; fsqrt` (0x416A4C).
fn edge_tail(f: &mut Fpu, dist: f32) {
    f.fld_st(0);
    f.fmulp(1);
    f.fld(dist);
    f.fmul_m(dist);
    f.faddp(1);
    f.fsqrt();
}

/// `CCollision::ProcessSphereTriangle(const CColSphere& sphere, const CompressedVector* verts,
/// const CColTriangle& tri, const CColTrianglePlane& plane, CColPoint& colPoint, float& minDistSq)`
/// (0x416BA0, AL). Plane distance `dist = (n.z*c.z + n.y*c.y) + n.x*c.x - d*(1/128)` (note the order
/// differs from `TestSphereTriangle`); `|dist| <= radius` and `dist^2 <= minDistSq` (stored f32; NaN
/// passes both) or false. Vertices through `CompressedVector` -> `CVector` (0x411590), then the same
/// edge frame and tests as `TestSphereTriangle`. The closest point and the distance per case:
/// inside (3) → `c - n*dist` (x, y from the register product, z through a stored f32), distance `|dist|`;
/// one vertex (1) → that vertex, `Magnitude(c - v)`; one edge (2) → the clamped end vertex, or
/// `start + edge * t` through `operator*` / `operator+` with distance `sqrt((perp/|edge|)^2 + dist^2)`
/// (`dist^2` the stored f32). Count 0 (only with NaNs) keeps `distance = |v1 - v0|` and an unwritten
/// closest point (zero here). Then `distance < radius` and `distance^2 < minDistSq` (f32, NaN false) or
/// false; `normal = Normalise(c - closest)`, the colpoint gets point, normal, surface/piece/light A from
/// the sphere, surface B from the triangle, piece B 0, depth `radius - distance`; `minDistSq =
/// distance^2` → true.
#[hook(addr = 0x416BA0, conv = cdecl, name = "CCollision::ProcessSphereTriangle", expect = "81 EC 38 01 00 00",
       unverified = "mvpe-coll-1", route = "L2", hazards = "x87-transcendental,uninit-locals",
       recs = "grove-walk,driving",
       note = "edge count 0 (NaN inputs) reads the never-written closest-point locals")]
pub fn process_sphere_triangle(sphere: GPtr<u8>, verts: GPtr<u8>, tri: GPtr<u8>, plane: GPtr<u8>, cp: GPtr<u8>,
    min: GPtr<u8>) -> u8 {
    let q4096 = kf(K_1_4096);
    let q128 = kf(K_1_128);
    let zero = kf(K_0);
    let one = kf(K_1);
    let mut f = Fpu::new();
    let nx = scaled(&mut f, plane, 0, q4096);
    let ny = scaled(&mut f, plane, 2, q4096);
    let nz = scaled(&mut f, plane, 4, q4096);
    let c = v3(sphere);
    let radius = rd::<f32>(sphere, 0xC);
    f.fld(nz);
    f.fmul_m(c[2]);
    f.fld(ny);
    f.fmul_m(c[1]);
    f.faddp(1);
    f.fld(nx);
    f.fmul_m(c[0]);
    f.faddp(1);
    f.fild(rd::<i16>(plane, 6) as i32);
    f.fmul_m(q128);
    f.fsubp(1);
    let dist = f.fst();
    f.fabs();
    let adist = f.fst();
    f.fcomp_m(radius);
    if !f.jne(0x41) {
        return 0;
    }
    f.fld(dist);
    f.fmul_m(dist);
    let dist2 = f.fst();
    f.fcomp_m(rd::<f32>(min, 0));
    if !f.jne(0x41) {
        return 0;
    }
    let mut vb = [[0u32; 3]; 3];
    for (k, b) in vb.iter_mut().enumerate() {
        let src = verts.byte_add(rd::<u16>(tri, 2 * k as isize) as isize * 6);
        uncompress_vector(local(b), src);
    }
    let vf = |b: &[u32; 3]| [f32::from_bits(b[0]), f32::from_bits(b[1]), f32::from_bits(b[2])];
    let (v0, v1, v2) = (vf(&vb[0]), vf(&vb[1]), vf(&vb[2]));
    // e1 components: x and y stored then kept, z stored (`fst`) and kept
    let e1 = [
        X87::from_f32(v1[0]).fsub_m(v0[0]),
        X87::from_f32(v1[1]).fsub_m(v0[1]),
        X87::from_f32(v1[2]).fsub_m(v0[2]),
    ];
    let e1f = [e1[0].to_f32(), e1[1].to_f32(), e1[2].to_f32()];
    let (t, e2) = process_frame(&mut f, e1, v0, v2, c, [nx, ny, nz]);
    let n = t.t1 as u32 + t.t2 as u32 + t.t3 as u32;
    let mut closest = [0f32; 3];
    // the distance local (S+0x10) holds |v1 - v0| until a case overwrites it
    let mut distance = t.len;
    let at_vertex = |f: &mut Fpu, v: [f32; 3], closest: &mut [f32; 3]| {
        f.push(dist_to(sphere, v));
        *closest = v;
        f.fstp()
    };
    // `start + e * t` through `operator*` and `operator+`
    let along = |start: [f32; 3], e: [f32; 3], tt: f32| -> [f32; 3] {
        let mut eb = [0u32; 3];
        let mut sb = [0u32; 3];
        let mut ob1 = [0u32; 3];
        let mut ob2 = [0u32; 3];
        let ep = vec_local(&mut eb, e);
        let sp = vec_local(&mut sb, start);
        let s = vector_scale(local(&mut ob1), ep, tt);
        let r = vector_add(local(&mut ob2), sp, s);
        v3(r)
    };
    match n {
        3 => {
            // inside: closest = c - n*dist; x, y with the register products, z through a stored f32
            f.fld(nx);
            f.fmul_m(dist);
            f.fld(ny);
            f.fmul_m(dist);
            f.fld(nz);
            f.fmul_m(dist);
            let ndz = f.fstp();
            f.fld(c[0]);
            f.fsub_st(2);
            closest[0] = f.fstp();
            f.fld(c[1]);
            f.fsub_st(1);
            closest[1] = f.fstp();
            f.fstp_st(0);
            f.fstp_st(0);
            f.fld(c[2]);
            f.fsub_m(ndz);
            closest[2] = f.fstp();
            distance = adist;
        }
        1 => {
            let v = if t.t1 {
                v2
            } else if t.t2 {
                v1
            } else {
                v0
            };
            distance = at_vertex(&mut f, v, &mut closest);
        }
        2 => {
            if !t.t1 {
                // outside edge 0 (v0 -> v1)
                f.fld(t.len);
                f.fmul_m(t.len);
                let len2 = f.fstp();
                f.fld(t.cu);
                f.fmul_m(t.len);
                f.fld(t.cv);
                f.fmul_m(zero);
                f.faddp(1);
                f.fdiv_m(len2);
                let tt = f.fst();
                f.fcomp_m(zero);
                if !f.jp(0x41) {
                    distance = at_vertex(&mut f, v0, &mut closest);
                } else {
                    f.fld(tt);
                    f.fcomp_m(one);
                    if !f.jne(1) {
                        distance = at_vertex(&mut f, v1, &mut closest);
                    } else {
                        f.fld(len2);
                        f.fsqrt();
                        f.fdivr_m(t.t1v);
                        distance = proc_tail(&mut f, dist2);
                        closest = along(v0, e1f, tt);
                    }
                }
            } else if !t.t2 {
                // outside edge 2 (v0 -> v2)
                f.fld(t.p2v);
                f.fmul_m(t.p2v);
                f.fld(t.p2u);
                f.fmul_m(t.p2u);
                f.faddp(1);
                let l2 = f.fstp();
                f.fld(t.cv);
                f.fmul_m(t.p2v);
                f.fld(t.cu);
                f.fmul_m(t.p2u);
                f.faddp(1);
                f.fdiv_m(l2);
                let tt = f.fst();
                f.fcomp_m(zero);
                if !f.jp(0x41) {
                    distance = at_vertex(&mut f, v0, &mut closest);
                } else {
                    f.fld(tt);
                    f.fcomp_m(one);
                    if !f.jne(1) {
                        distance = at_vertex(&mut f, v2, &mut closest);
                    } else {
                        f.fld(t.cv_p2u);
                        f.fsub_m(t.cu_p2v);
                        f.fld(l2);
                        f.fsqrt();
                        f.fdivp(1);
                        distance = proc_tail(&mut f, dist2);
                        closest = along(v0, e2, tt);
                    }
                }
            } else {
                // outside edge 1 (v1 -> v2)
                f.fld(t.p2v);
                f.fmul_m(t.p2v);
                f.fld(t.p2u_len);
                f.fmul_m(t.p2u_len);
                f.faddp(1);
                let l2 = f.fstp();
                f.fld(t.cu_len);
                f.fmul_m(t.p2u_len);
                f.fld(t.cv);
                f.fmul_m(t.p2v);
                f.faddp(1);
                f.fdiv_m(l2);
                let tt = f.fst();
                f.fcomp_m(zero);
                if !f.jp(0x41) {
                    distance = at_vertex(&mut f, v1, &mut closest);
                } else {
                    f.fld(tt);
                    f.fcomp_m(one);
                    if !f.jne(1) {
                        distance = at_vertex(&mut f, v2, &mut closest);
                    } else {
                        f.fld(t.cv);
                        f.fmul_m(t.p2u_len);
                        f.fld(t.cu_len);
                        f.fmul_m(t.p2v);
                        f.fsubp(1);
                        f.fld(l2);
                        f.fsqrt();
                        f.fdivp(1);
                        distance = proc_tail(&mut f, dist2);
                        // e3 = v2 - v1 (stored f32 per component)
                        let e3 = [
                            X87::from_f32(v2[0]).fsub_m(v1[0]).to_f32(),
                            X87::from_f32(v2[1]).fsub_m(v1[1]).to_f32(),
                            X87::from_f32(v2[2]).fsub_m(v1[2]).to_f32(),
                        ];
                        closest = along(v1, e3, tt);
                    }
                }
            }
        }
        _ => {}
    }
    f.fld(distance);
    f.fmul_m(distance);
    let dsq = f.fstp();
    f.fld(distance);
    f.fcomp_m(radius);
    if f.jp(5) {
        return 0;
    }
    f.fld(dsq);
    f.fcomp_m(rd::<f32>(min, 0));
    if f.jp(5) {
        return 0;
    }
    let mut nb = [0u32; 3];
    for ((d, ck), pk) in nb.iter_mut().zip(c).zip(closest) {
        f.fld(ck);
        f.fsub_m(pk);
        *d = f.fstp().to_bits();
    }
    let np = local(&mut nb);
    normalise(np.cast());
    for k in 0..3isize {
        wr::<f32>(cp, 4 * k, closest[k as usize]);
    }
    for k in 0..3isize {
        wr::<u32>(cp, 0x10 + 4 * k, rd::<u32>(np, 4 * k));
    }
    wr::<u16>(cp, 0x20, rd::<u16>(sphere, 0x10));
    wr::<u8>(cp, 0x22, rd::<u8>(sphere, 0x12));
    wr::<u8>(cp, 0x23, rd::<u8>(tri, 6));
    wr::<u8>(cp, 0x24, 0);
    f.fld(radius);
    f.fsub_m(distance);
    wr::<f32>(cp, 0x28, f.fstp());
    wr::<f32>(min, 0, dsq);
    1
}

/// `fdivp`/`fdivr` result on st0 → `fld st0; fmulp; fadd [dist^2]; fsqrt; fstp` (the distance).
fn proc_tail(f: &mut Fpu, dist2: f32) -> f32 {
    f.fld_st(0);
    f.fmulp(1);
    f.fadd_m(dist2);
    f.fsqrt();
    f.fstp()
}

/// `ProcessSphereTriangle`'s edge frame: the same sequence as [`tri_frame`] except the length sum
/// `(x*x + z*z) + y*y` (`fld st1; fmul st2; fld [z]; fmul [z]; faddp; fld st1; fmul st2; faddp`) and
/// the v2 projection `(e2x*dx + e2z*dz) + e2y*dy`. Returns the frame and `v2 - v0` (stored f32).
fn process_frame(f: &mut Fpu, e1: [X87; 3], v0: [f32; 3], v2: [f32; 3], c: [f32; 3], n: [f32; 3]) -> (TriFrame, [f32; 3]) {
    let one = kf(K_1);
    let zero = kf(K_0);
    f.push(e1[0]);
    f.push(e1[1]);
    let e1z = e1[2].to_f32();
    // st0 = e1y, st1 = e1x
    f.fld_st(1);
    f.fmul_st(2);
    f.fld(e1z);
    f.fmul_m(e1z);
    f.faddp(1);
    f.fld_st(1);
    f.fmul_st(2);
    f.faddp(1);
    f.fsqrt();
    let len = f.fstp();
    f.fld(one);
    f.fdiv_m(len);
    let inv = f.fstp();
    f.fxch(1);
    f.fmul_m(inv);
    let dx = f.fstp();
    f.fmul_m(inv);
    let dy = f.fstp();
    f.fld(e1z);
    f.fmul_m(inv);
    let dz = f.fstp();
    let mut db = [0u32; 3];
    let mut nb = [0u32; 3];
    let mut pb = [0u32; 3];
    let dp = vec_local(&mut db, [dx, dy, dz]);
    let np = vec_local(&mut nb, n);
    cross_product(local(&mut pb).cast(), dp.cast(), np.cast());
    let [px, py, pz] = [f32::from_bits(pb[0]), f32::from_bits(pb[1]), f32::from_bits(pb[2])];
    f.fld(v2[0]);
    f.fsub_m(v0[0]);
    let e2x = f.fst();
    f.fld(v2[1]);
    f.fsub_m(v0[1]);
    let e2y = f.fst();
    f.fld(v2[2]);
    f.fsub_m(v0[2]);
    let e2z = f.fst(); // st0 = e2z, st1 = e2y, st2 = e2x
    f.fld_st(2);
    f.fmul_m(dx);
    f.fld_st(1);
    f.fmul_m(dz);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_m(dy);
    f.faddp(1);
    let p2u = f.fstp();
    f.fxch(2);
    f.fmul_m(px);
    f.fld(pz);
    f.fmul_st(3);
    f.faddp(1);
    f.fld(py);
    f.fmul_st(2);
    f.faddp(1);
    let p2v = f.fstp();
    f.fstp_st(1);
    f.fstp_st(0);
    f.fld(c[0]);
    f.fsub_m(v0[0]);
    f.fld(c[1]);
    f.fsub_m(v0[1]);
    f.fld(c[2]);
    f.fsub_m(v0[2]);
    f.fld_st(2);
    f.fmul_m(dx);
    f.fld_st(1);
    f.fmul_m(dz);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_m(dy);
    f.faddp(1);
    let cu = f.fstp();
    f.fxch(2);
    f.fmul_m(px);
    f.fxch(2);
    f.fmul_m(pz);
    f.faddp(2);
    f.fmul_m(py);
    f.faddp(1);
    let cv = f.fstp();
    f.fld(cv);
    f.fmul_m(len);
    f.fld(cu);
    f.fmul_m(zero);
    f.fsubp(1);
    let t1v = f.fst();
    f.fcomp_m(zero);
    let t1 = !f.jne(1);
    f.fld(cu);
    f.fmul_m(p2v);
    let cu_p2v = f.fstp();
    f.fld(cv);
    f.fmul_m(p2u);
    let cv_p2u = f.fstp();
    f.fld(cu_p2v);
    f.fsub_m(cv_p2u);
    f.fcomp_m(zero);
    let t2 = !f.jne(1);
    f.fld(p2u);
    f.fsub_m(len);
    let p2u_len = f.fstp();
    f.fld(cu);
    f.fsub_m(len);
    let cu_len = f.fstp();
    f.fld(p2u_len);
    f.fmul_m(cv);
    f.fld(cu_len);
    f.fmul_m(p2v);
    f.fsubp(1);
    f.fcomp_m(zero);
    let t3 = !f.jne(1);
    (
        TriFrame { len, p2u, p2v, cu, cv, t1v, cu_p2v, cv_p2u, p2u_len, cu_len, t1, t2, t3 },
        [e2x, e2y, e2z],
    )
}

/// One face of `ProcessLineBox` (axis `k`, the box's min face when `!max_face`; the other two axes
/// `i1`, `i2` in the original's order). On the x87 stack st0 is the running minimum `t`. Min face:
/// `a = min.k - s.k`, `b = min.k - e.k`; max face: `a = s.k - max.k`, `b = e.k - max.k` (register
/// values); `a*b < 0` (else no hit), `t = a / (a - b)` stored f32 (into the `line` argument slot), the
/// other coordinates `(e.i - s.i) * t + s.i` (f32; the first into the `box` slot), each strictly inside
/// the box (`> min`, `< max`, NaN fails), then `t < bound` (the constant 1.0 for the first face,
/// `st(1)` = the running minimum for the others). A hit replaces the minimum with `t` and returns the
/// hit coordinates.
#[allow(clippy::too_many_arguments)]
fn line_box_face(f: &mut Fpu, s: [f32; 3], e: [f32; 3], bmin: [f32; 3], bmax: [f32; 3], k: usize, max_face: bool,
    i1: usize, i2: usize, first: bool) -> Option<(f32, f32, f32)> {
    if max_face {
        f.fld(s[k]);
        f.fsub_m(bmax[k]);
        f.fld(e[k]);
        f.fsub_m(bmax[k]);
    } else {
        f.fld(bmin[k]);
        f.fsub_m(s[k]);
        f.fld(bmin[k]);
        f.fsub_m(e[k]);
    }
    f.fld_st(0);
    f.fmul_st(2);
    f.fcomp_m(kf(K_0));
    if f.jp(5) {
        f.fstp_st(0);
        f.fstp_st(0);
        return None;
    }
    f.fld_st(1);
    f.fsub_st(1);
    f.fdivr_st(2);
    let t = f.fstp();
    f.fstp_st(0);
    f.fstp_st(0);
    let coord = |f: &mut Fpu, i: usize| {
        f.fld(e[i]);
        f.fsub_m(s[i]);
        f.fmul_m(t);
        f.fadd_m(s[i]);
        f.fstp()
    };
    let c1 = coord(f, i1);
    let c2 = coord(f, i2);
    for (c, i) in [(c1, i1), (c2, i2)] {
        f.fld(c);
        f.fcomp_m(bmin[i]);
        if f.jne(0x41) {
            return None;
        }
        f.fld(c);
        f.fcomp_m(bmax[i]);
        if f.jp(5) {
            return None;
        }
    }
    f.fld(t);
    if first {
        f.fcomp_m(kf(K_1));
    } else {
        f.fcomp_st(1);
    }
    if f.jp(5) {
        return None;
    }
    f.fstp_st(0);
    f.fld(t);
    Some((t, c1, c2))
}

/// `CCollision::ProcessLineBox(const CColLine& line, const CColBox& box, CColPoint& colPoint,
/// float& minDist)` (0x413100, AL). Start strictly inside the box (`> min`, `< max` per axis, NaN
/// counts as outside): `ms_iProcessLineNumCrossings` += 1 when the end is strictly outside on some
/// axis (`< min` or `> max`, NaN inside), `CalculateColPointInsideBox(box, start, colPoint)`, surface
/// A / piece A 0, surface B / piece B / light B from the box (+0x18 word, +0x1A byte), `*minDist = 0`
/// → true. Otherwise the six faces in the order -x, +x, -y, +y, -z, +z through [`line_box_face`] with
/// the running minimum `t` (starting at 1.0) on the x87 stack; each hit sets the point (the face's
/// coordinate and the two interpolated ones) and the axis normal (+-1). Unless `t < *minDist` (NaN
/// false) → false; else the colpoint gets point, normal, surface B / piece B / light B from the box,
/// surface A / piece A 0, `*minDist = t`, and the crossings counter += 1 when the end is inside the
/// box (`>= min`, `<= max`, NaN inside), else += 2 → true.
/// Quirks kept: when no face hits but `1.0 < *minDist`, the colpoint gets the point / normal locals the
/// original never wrote (when the -x face misses, the point registers are even reloaded from three
/// unwritten locals; zero here). Stack: `t` and an interpolated coordinate are stored into the `line`
/// (S+4) and `box` (S+8) argument slots (not reproduced).
#[hook(addr = 0x413100, conv = cdecl, name = "CCollision::ProcessLineBox", expect = "8B 4C 24 04 D9 05 24 86 85 00",
       unverified = "mvpe-coll-1", route = "L2", hazards = "stack-residue,uninit-locals",
       recs = "grove-walk,driving",
       note = "stores t / a coordinate into its line/box argument slots; no-hit with *minDist > 1 writes unwritten locals")]
pub fn process_line_box(line: GPtr<u8>, bx: GPtr<u8>, cp: GPtr<u8>, min: GPtr<u8>) -> u8 {
    let s = v3(line);
    let e = v3(line.byte_add(0x10));
    let bmin = v3(bx);
    let bmax = v3(bx.byte_add(0xC));
    let mut f = Fpu::new();
    let mut start_inside = true;
    for k in 0..3 {
        f.fld(s[k]);
        f.fcomp_m(bmin[k]);
        if f.jne(0x41) {
            start_inside = false;
            break;
        }
    }
    if start_inside {
        for k in 0..3 {
            f.fld(s[k]);
            f.fcomp_m(bmax[k]);
            if f.jp(5) {
                start_inside = false;
                break;
            }
        }
    }
    // end strictly outside: `< min` on some axis, or `> max` on some axis (NaN neither)
    let end_outside = |f: &mut Fpu| -> bool {
        for k in 0..3 {
            f.fld(e[k]);
            f.fcomp_m(bmin[k]);
            if !f.jp(5) {
                return true;
            }
        }
        for k in 0..3 {
            f.fld(e[k]);
            f.fcomp_m(bmax[k]);
            if !f.jne(0x41) {
                return true;
            }
        }
        false
    };
    if start_inside {
        if end_outside(&mut f) {
            g::<u32>(NUM_CROSSINGS).write(g::<u32>(NUM_CROSSINGS).read().wrapping_add(1));
        }
        calculate_col_point_inside_box(bx.cast(), line.cast(), cp.cast());
        wr::<u8>(cp, 0x20, 0);
        wr::<u8>(cp, 0x21, 0);
        wr::<u16>(cp, 0x23, rd::<u16>(bx, 0x18));
        wr::<u8>(cp, 0x25, rd::<u8>(bx, 0x1A));
        wr::<u32>(min, 0, 0);
        return 1;
    }
    f.fld(kf(K_1));
    // point (edx, edi, ebx) and normal locals; never written unless a face hits
    let mut p = [0f32; 3];
    let mut n = [0f32; 3];
    let faces: [(usize, bool, usize, usize, f32); 6] = [
        (0, false, 1, 2, -1.0),
        (0, true, 1, 2, 1.0),
        (1, false, 0, 2, -1.0),
        (1, true, 0, 2, 1.0),
        (2, false, 0, 1, -1.0),
        (2, true, 0, 1, 1.0),
    ];
    for (idx, &(k, mx, i1, i2, sign)) in faces.iter().enumerate() {
        if let Some((_, c1, c2)) = line_box_face(&mut f, s, e, bmin, bmax, k, mx, i1, i2, idx == 0) {
            p[k] = if mx { bmax[k] } else { bmin[k] };
            p[i1] = c1;
            p[i2] = c2;
            n = [0.0; 3];
            n[k] = sign;
        }
    }
    f.fld_st(0);
    f.fcomp_m(rd::<f32>(min, 0));
    if f.jp(5) {
        return 0;
    }
    for k in 0..3 {
        wr::<f32>(cp, 4 * k as isize, p[k]);
        wr::<f32>(cp, 0x10 + 4 * k as isize, n[k]);
    }
    wr::<u16>(cp, 0x23, rd::<u16>(bx, 0x18));
    wr::<u8>(cp, 0x25, rd::<u8>(bx, 0x1A));
    wr::<u8>(cp, 0x20, 0);
    wr::<u8>(cp, 0x21, 0);
    wr::<f32>(min, 0, f.fstp());
    let add = if end_outside(&mut f) { 2 } else { 1 };
    g::<u32>(NUM_CROSSINGS).write(g::<u32>(NUM_CROSSINGS).read().wrapping_add(add));
    1
}

/// `CWorld::ms_nCurrentScanCode`
const SCAN_CODE: usize = 0xB7CD78;
/// `CWorld::pIgnoreEntity`
const IGNORE_ENTITY: usize = 0xB7CD68;
/// `gpMadeInvisibleEntities[10]`
const MADE_INVISIBLE: usize = 0x9655A0;
/// `gNumEntitiesSetInvisible`
const NUM_INVISIBLE: usize = 0x9655DC;

/// Appends `e` to `gpMadeInvisibleEntities[gNumEntitiesSetInvisible]` (no bound check, as the
/// original), registers the slot as a reference of `e` and increments the count.
fn add_made_invisible(e: GPtr<u8>) {
    let slot = MADE_INVISIBLE + 4 * g::<i32>(NUM_INVISIBLE).read() as usize;
    g::<GPtr<u8>>(slot).write(e);
    register_reference(e, GPtr::from_addr(slot));
    g::<i32>(NUM_INVISIBLE).write(g::<i32>(NUM_INVISIBLE).read().wrapping_add(1));
}

/// `CCollision::CheckCameraCollisionPeds(int32 sectorX, int32 sectorY, const CVector& pos,
/// const CVector& dir, float& t)` (0x415320, AL). `r2 = f(0x8CCB8C)^2` (stored f32 into the `sectorY`
/// argument slot); the repeat sector `((y & 15) << 4) + (x & 15)` ped list (0xB992BC + 12*i).
/// `Normalise((dir.x, dir.y, 0))` into a local that is never read. Per ped not yet scanned (+0x2C u16
/// against `ms_nCurrentScanCode`, then set): visible (+0x1C bit 7), not `pIgnoreEntity`, not a player
/// (`IsPlayer`), `(dy*dy + dx*dx) < r2` with `d = pos - GetBoundCentre()` (register values), and
/// `FUN_0050D590(ped) <= 0` (NaN fails) → the ped is made invisible: appended to
/// `gpMadeInvisibleEntities` (reference registered), its held entity (`GetTaskHold(false)` +8) too
/// when visible (bit 7 cleared first), then the ped's bit 7 cleared; result true. `t` is unused.
#[hook(addr = 0x415320, conv = cdecl, name = "CCollision::CheckCameraCollisionPeds", expect = "83 EC 18 8B 44 24 20",
       unverified = "mvpe-coll-1", route = "L2", hazards = "stack-residue,heap-writes", recs = "grove-walk,driving",
       note = "stores the squared radius into its sectorY argument slot; clears ped/held-entity flags (+0x1C)")]
pub fn check_camera_collision_peds(sx: i32, sy: i32, pos: GPtr<u8>, dir: GPtr<u8>, _t: GPtr<u8>) -> u8 {
    let mut f = Fpu::new();
    let k = kf(0x8CCB8C);
    f.fld(k);
    f.fmul_m(k);
    let r2 = f.fstp();
    let idx = (((sy & 0xF) << 4) + (sx & 0xF)) as usize;
    let mut node = g::<GPtr<u8>>(0xB992BC + 12 * idx).read();
    let mut db = [rd::<u32>(dir, 0), rd::<u32>(dir, 4), 0];
    normalise(local(&mut db).cast());
    let mut hit = 0u8;
    while !node.is_null() {
        let ped = rd::<GPtr<u8>>(node, 0);
        let sc = g::<u16>(SCAN_CODE).read();
        if rd::<u16>(ped, 0x2C) != sc {
            wr::<u16>(ped, 0x2C, sc);
            if (rd::<u32>(ped, 0x1C) >> 7) & 1 != 0
                && g::<GPtr<u8>>(IGNORE_ENTITY).read() != ped
                && ped_is_player(ped) == 0
            {
                let mut cb = [0u32; 3];
                let bc = get_bound_centre(ped, local(&mut cb));
                f.fld(rd::<f32>(pos, 0));
                f.fsub_m(rd::<f32>(bc, 0));
                f.fld(rd::<f32>(pos, 4));
                f.fsub_m(rd::<f32>(bc, 4));
                f.fld_st(0);
                f.fmul_st(1);
                f.fld_st(2);
                f.fmul_st(3);
                f.faddp(1);
                f.fcomp_m(r2);
                f.fstp_st(0);
                f.fstp_st(0);
                if !f.jp(5) {
                    f.push(X87::from_f64_exact(ped_col_fn_50d590(ped)));
                    f.fcomp_m(kf(K_0));
                    if !f.jp(0x41) {
                        add_made_invisible(ped);
                        let task = get_task_hold(rd::<GPtr<u8>>(ped, 0x47C), 0);
                        if !task.is_null() {
                            let held = rd::<GPtr<u8>>(task, 8);
                            if !held.is_null() {
                                let fl = rd::<u32>(held, 0x1C);
                                if (fl >> 7) & 1 != 0 {
                                    wr::<u32>(held, 0x1C, fl & 0xFFFF_FF7F);
                                    add_made_invisible(held);
                                }
                            }
                        }
                        wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) & 0xFFFF_FF7F);
                        hit = 1;
                    }
                }
            }
        }
        node = rd::<GPtr<u8>>(node, 4);
    }
    hit
}

/// `mi->m_pColModel->m_boundSphere.m_fRadius` of entity `e` (`CModelInfo::ms_modelInfoPtrs[e+0x22]`
/// +0x14, then +0x24).
fn bound_radius(e: GPtr<u8>) -> f32 {
    let mi = g::<GPtr<u8>>((0xA9B0C8 + 4 * i32::from(rd::<i16>(e, 0x22))) as u32 as usize).read();
    rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x24)
}

/// `CColSphere::Set(radius, GetBoundCentre(), 0, 0, 0xFF)` into `sp` (the original passes the radius
/// through one of its own argument slots, not reproduced).
fn entity_bound_sphere(e: GPtr<u8>, sp: GPtr<u8>) {
    let r = bound_radius(e);
    let mut cb = [0u32; 3];
    let c = get_bound_centre(e, local(&mut cb));
    col_sphere_set(sp, r, c, 0, 0, 0xFF);
}

/// The inline overlap test of the camera collision checks: `d = spS.c - sp.c` (register values),
/// `rs = sp.r + spS.r`; `rs*rs > (dz*dz + dy*dy) + dx*dx` (`fcompp; test ah, 0x41; jne`, NaN fails).
fn cam_spheres_overlap(sps: GPtr<u8>, sp: GPtr<u8>) -> bool {
    let mut f = Fpu::new();
    for k in 0..3isize {
        f.fld(rd::<f32>(sps, 4 * k));
        f.fsub_m(rd::<f32>(sp, 4 * k));
    }
    f.fld(rd::<f32>(sp, 0xC));
    f.fadd_m(rd::<f32>(sps, 0xC));
    f.fld_st(1);
    f.fmul_st(2);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fld_st(4);
    f.fmul_st(5);
    f.faddp(1);
    f.fld_st(1);
    f.fmul_st(2);
    f.fcompp();
    !f.jne(0x41)
}

/// `CCollision::CheckCameraCollisionBuildings(int32 x, int32 y, const CColBox& bbSpAB,
/// const CColSphere& spS, const CColSphere& spA, const CColSphere& spB)` (0x41A820, AL). Sector
/// `clamp(y, 0, 119) * 120 + clamp(x, 0, 119)` building list (0xB7D0B8 + 8*i). `flyer = player
/// vehicle && (veh+0x43 & 1)` (kept in the `x` argument slot). Per building not yet scanned (scan
/// code set): skipped when `flyer && DoesNotCollideWithFlyers()` or it is `pIgnoreEntity`; its bound
/// sphere (`CColSphere::Set(model bound radius, GetBoundCentre(), 0, 0, 0xFF)`; the radius goes through
/// the `y` argument slot) must overlap `spS` ([`cam_spheres_overlap`]); then
/// `SphereCastVsEntity(spA, spB, building)` → true. `bbSpAB` is unused.
#[hook(addr = 0x41A820, conv = cdecl, name = "CCollision::CheckCameraCollisionBuildings", expect = "8B 44 24 04 33 C9 83 EC 20",
       unverified = "mvpe-coll-1", route = "L2", hazards = "stack-residue", recs = "grove-walk,driving",
       note = "writes the flyer flag into its x argument slot and the bound radius into its y slot")]
pub fn check_camera_collision_buildings(x: i32, y: i32, _bb: GPtr<u8>, sps: GPtr<u8>, spa: GPtr<u8>, spb: GPtr<u8>) -> u8 {
    let cx = x.clamp(0, 0x77);
    let cy = y.clamp(0, 0x77);
    let mut node = g::<GPtr<u8>>(0xB7D0B8 + 8 * (cy * 0x78 + cx) as usize).read();
    let veh = find_player_vehicle(-1, 0);
    let flyer = !veh.is_null() && rd::<u8>(veh, 0x43) & 1 != 0;
    let mut hit = 0u8;
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        let sc = g::<u16>(SCAN_CODE).read();
        if rd::<u16>(e, 0x2C) != sc {
            wr::<u16>(e, 0x2C, sc);
            if !(flyer && does_not_collide_with_flyers(e) != 0) && g::<GPtr<u8>>(IGNORE_ENTITY).read() != e {
                let mut sb = [0u32; 5];
                let sp = local(&mut sb);
                entity_bound_sphere(e, sp);
                if cam_spheres_overlap(sps, sp) && sphere_cast_vs_entity(spa, spb, e) != 0 {
                    hit = 1;
                }
            }
        }
        node = rd::<GPtr<u8>>(node, 4);
    }
    hit
}

/// The vehicle the camera's ignore entity sits on (0x9689D8) and its countdown (0x9689D4).
const SIT_VEH: usize = 0x9689D8;
const SIT_COUNT: usize = 0x9689D4;

/// `CCollision::CheckCameraCollisionVehicles(int32 x, int32 y, const CColBox& bbSpAB,
/// const CColSphere& spS, const CColSphere& spA, const CColSphere& spB, const CVector* plyrVehVel)`
/// (0x41A990, AL). Repeat sector `((y & 15) << 4) + (x & 15)` vehicle list (0xB992B8 + 12*i). Per
/// vehicle not yet scanned (scan code set), not `pIgnoreEntity`: when
/// `IsThisVehicleSittingOnMe(pIgnoreEntity, veh)` the pair (0x9689D8, 0x9689D4) becomes (veh,
/// *0x8A5B1C) and it is skipped; else when it is the stored vehicle the counter is decremented (old
/// value 0 → the vehicle is cleared) and it is skipped. With `plyrVehVel`: skipped unless
/// `|plyrVehVel - veh.moveSpeed (+0x44)|^2 < *0x8A5B18` (`(dz*dz + dy*dy) + dx*dx`, NaN skips). Then
/// `TestSphereSphere(spS, bound sphere)` and `SphereCastVsEntity(spA, spB, veh)` → true. The bound
/// radius goes through the `y` (with velocity) or `x` argument slot.
/// Not registered as a hook: the no-CD code in `.data` (0x14A28ED, 0x14A29FA, ... 0x14A4C33) reads this entry's
/// bytes through `or r32, [reg*4 + 0x41A990]` (an indexed table read, which keycells.rs cannot redirect), so
/// patching the entry would change what it decodes. Kept as the port body for phases B/C; the `orig!` call
/// site in `camera_cone_cast_vs_world_collision` reaches it explicitly.
#[allow(clippy::too_many_arguments)]
pub fn check_camera_collision_vehicles(x: i32, y: i32, _bb: GPtr<u8>, sps: GPtr<u8>, spa: GPtr<u8>, spb: GPtr<u8>,
    vel: GPtr<u8>) -> u8 {
    let idx = (((y & 0xF) << 4) + (x & 0xF)) as usize;
    let mut node = g::<GPtr<u8>>(0xB992B8 + 12 * idx).read();
    let mut hit = 0u8;
    while !node.is_null() {
        let v = rd::<GPtr<u8>>(node, 0);
        let sc = g::<u16>(SCAN_CODE).read();
        if rd::<u16>(v, 0x2C) != sc {
            wr::<u16>(v, 0x2C, sc);
            let ign = g::<GPtr<u8>>(IGNORE_ENTITY).read();
            if ign != v {
                camera_vehicle(v, ign, sps, spa, spb, vel, &mut hit);
            }
        }
        node = rd::<GPtr<u8>>(node, 4);
    }
    hit
}

/// The body of [`check_camera_collision_vehicles`] for one vehicle.
fn camera_vehicle(v: GPtr<u8>, ign: GPtr<u8>, sps: GPtr<u8>, spa: GPtr<u8>, spb: GPtr<u8>, vel: GPtr<u8>, hit: &mut u8) {
    if is_this_vehicle_sitting_on_me(ign, v) != 0 {
        g::<GPtr<u8>>(SIT_VEH).write(v);
        g::<u32>(SIT_COUNT).write(g::<u32>(0x8A5B1C).read());
        return;
    }
    if g::<GPtr<u8>>(SIT_VEH).read() == v {
        let c = g::<i32>(SIT_COUNT).read();
        g::<i32>(SIT_COUNT).write(c.wrapping_sub(1));
        if c == 0 {
            g::<u32>(SIT_VEH).write(0);
        }
        return;
    }
    if !vel.is_null() {
        let mut f = Fpu::new();
        for k in 0..3isize {
            f.fld(rd::<f32>(vel, 4 * k));
            f.fsub_m(rd::<f32>(v, 0x44 + 4 * k));
        }
        f.fld_st(0);
        f.fmul_st(1);
        f.fld_st(2);
        f.fmul_st(3);
        f.faddp(1);
        f.fld_st(3);
        f.fmul_st(4);
        f.faddp(1);
        f.fcomp_m(kf(0x8A5B18));
        if f.jp(5) {
            return;
        }
    }
    let mut sb = [0u32; 5];
    let sp = local(&mut sb);
    entity_bound_sphere(v, sp);
    if test_sphere_sphere(sps.cast(), sp.cast()) & 0xFF != 0 && sphere_cast_vs_entity(spa, spb, v) != 0 {
        *hit = 1;
    }
}

/// `CCollision::CheckCameraCollisionObjects(int32 x, int32 y, const CColBox& bbSpAB,
/// const CColSphere& spS, const CColSphere& spA, const CColSphere& spB)` (0x41AB20, AL). Repeat sector
/// `((y & 15) << 4) + (x & 15)` object list (0xB992C0 + 12*i). Per object not yet scanned (scan code
/// set): skipped when `CameraToIgnoreThisObject` or it is `pIgnoreEntity`; then as the buildings: bound
/// sphere (radius through the `y` argument slot) overlapping `spS` ([`cam_spheres_overlap`]) and
/// `SphereCastVsEntity(spA, spB, object)` → true.
#[hook(addr = 0x41AB20, conv = cdecl, name = "CCollision::CheckCameraCollisionObjects", expect = "8B 44 24 08 8B 4C 24 04",
       unverified = "mvpe-coll-1", route = "L2", hazards = "stack-residue", recs = "grove-walk,driving",
       note = "stores the bound radius into its y argument slot")]
pub fn check_camera_collision_objects(x: i32, y: i32, _bb: GPtr<u8>, sps: GPtr<u8>, spa: GPtr<u8>, spb: GPtr<u8>) -> u8 {
    let idx = (((y & 0xF) << 4) + (x & 0xF)) as usize;
    let mut node = g::<GPtr<u8>>(0xB992C0 + 12 * idx).read();
    let mut hit = 0u8;
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        let sc = g::<u16>(SCAN_CODE).read();
        if rd::<u16>(e, 0x2C) != sc {
            wr::<u16>(e, 0x2C, sc);
            if camera_to_ignore_this_object(e) == 0 && g::<GPtr<u8>>(IGNORE_ENTITY).read() != e {
                let mut sb = [0u32; 5];
                let sp = local(&mut sb);
                entity_bound_sphere(e, sp);
                if cam_spheres_overlap(sps, sp) && sphere_cast_vs_entity(spa, spb, e) != 0 {
                    hit = 1;
                }
            }
        }
        node = rd::<GPtr<u8>>(node, 4);
    }
    hit
}

/// `_ftol(floor((double)v))` of the sector coordinate `v` (a register value, CRT calls kept).
fn sector_floor(v: X87) -> i32 {
    X87::from_f64_exact(crt_floor(v.to_f64())).ftol() as u32 as i32
}

/// `x * 0.02 + 60` (0x858B38, 0x858B34) as the x87 computes it; the f32 the original `fst`s too.
fn sector_coord(x: f32) -> (X87, f32) {
    let s = X87::from_f32(x).fmul_m(kf(0x858B38)).fadd_m(kf(0x858B34));
    (s, s.to_f32())
}

/// `CCollision::BuildCacheOfCameraCollision(const CColSphere& spA, const CColSphere& spB)` (0x41AC40,
/// AL). `bb = GetBoundingBoxFromTwoSpheres(spA, spB)`; `spS = CColSphere::Set(sqrt((wz*wz + wy*wy) +
/// wx*wx) * 0.5, min + w*0.5, 0, 0, 0xFF)` with `w = max - min` (register values; the centre
/// components stored f32). Sector rect: `left = max(0, ftol(floor(min.x*0.02 + 60)))`, `bottom` the
/// same of min.y, `right = min(119, ...)` of max.x, `top` of max.y (the second `floor` of each runs on
/// the stored f32) → gnLeft/gnBottom/gnRight/gnTop. Scan code incremented (at 0xFFFF and above
/// `ClearScanCodes` and 1). `gNumCacheEntries` (0x9655D8) = 0. Without `pIgnoreEntity` the player's
/// active task 0x2BD, else 0x2BC (`FindActiveTaskByType` on intelligence+4) gives
/// `GetCameraAvoidVehicle()` as the ignore entity. With a player vehicle `FindPlayerSpeed(0)` is copied
/// into a local whose address is the vehicles check's velocity. Every sector in the rect (rows outer,
/// bounds inclusive): buildings (with 0x9655E4 = 1 around the call), vehicles, objects, as enabled by
/// 0x8A5B16 / 0x8A5B14 / 0x8A5B15; any true → true. `pIgnoreEntity` is restored on exit.
#[hook(addr = 0x41AC40, conv = cdecl, name = "CCollision::BuildCacheOfCameraCollision", expect = "83 EC 60 53 8B 5C 24 68",
       unverified = "mvpe-coll-1", route = "L2", hazards = "x87-transcendental", recs = "grove-walk,driving",
       note = "FSQRT radius; CRT floor/_ftol for the sector rect")]
pub fn build_cache_of_camera_collision(spa: GPtr<u8>, spb: GPtr<u8>) -> u8 {
    let mut bbb = [0u32; 7];
    let bb = local(&mut bbb);
    get_bounding_box_from_two_spheres(bb.cast(), spa.cast(), spb.cast());
    let b = |o: isize| rd::<f32>(bb, o);
    let mut f = Fpu::new();
    f.fld(b(0xC));
    f.fsub_m(b(0));
    f.fld(b(0x10));
    f.fsub_m(b(4));
    f.fld(b(0x14));
    f.fsub_m(b(8)); // st0 = wz, st1 = wy, st2 = wx
    let half = kf(0x858B8C);
    f.fld_st(2);
    f.fmul_m(half);
    f.fld_st(2);
    f.fmul_m(half);
    f.fld_st(2);
    f.fmul_m(half);
    let hz = f.fstp();
    f.fxch(1);
    f.fadd_m(b(0));
    let cx = f.fstp();
    f.fadd_m(b(4));
    let cy = f.fstp();
    f.fld(hz);
    f.fadd_m(b(8));
    let cz = f.fstp();
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fsqrt();
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
    f.fmul_m(half);
    let r = f.fstp();
    let mut cb = [cx.to_bits(), cy.to_bits(), cz.to_bits()];
    let mut sb = [0u32; 5];
    let sps = local(&mut sb);
    col_sphere_set(sps, r, local(&mut cb), 0, 0, 0xFF);
    let lo = |x: f32| {
        let (s, sf) = sector_coord(x);
        if sector_floor(s) > 0 { sector_floor(X87::from_f32(sf)) } else { 0 }
    };
    let hi = |x: f32| {
        let (s, sf) = sector_coord(x);
        if sector_floor(s) < 0x77 { sector_floor(X87::from_f32(sf)) } else { 0x77 }
    };
    let left = lo(b(0));
    let bottom = lo(b(4));
    let right = hi(b(0xC));
    let top = hi(b(0x10));
    g::<i32>(0x96559C).write(left);
    g::<i32>(0x965598).write(bottom);
    g::<i32>(0x965594).write(right);
    g::<i32>(0x965590).write(top);
    if g::<u16>(SCAN_CODE).read() == 0xFFFF {
        clear_scan_codes();
        g::<u16>(SCAN_CODE).write(1);
    } else {
        g::<u16>(SCAN_CODE).write(g::<u16>(SCAN_CODE).read().wrapping_add(1));
    }
    let do_veh = g::<u8>(0x8A5B14).read();
    let do_bld = g::<u8>(0x8A5B16).read();
    let do_obj = g::<u8>(0x8A5B15).read();
    let saved = g::<GPtr<u8>>(IGNORE_ENTITY).read();
    g::<u32>(0x9655D8).write(0);
    let mut result = 0u8;
    if saved.is_null() {
        let ped = find_player_ped(0);
        let mut task = find_active_task_by_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 0x2BD);
        if task.is_null() {
            task = find_active_task_by_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 0x2BC);
        }
        if !task.is_null() {
            g::<GPtr<u8>>(IGNORE_ENTITY).write(get_camera_avoid_vehicle(task));
        }
    }
    let mut velb = [0u32; 3];
    let mut vel = GPtr::<u8>::NULL;
    if !find_player_vehicle(0, 0).is_null() {
        let s = find_player_speed(0);
        for (k, d) in velb.iter_mut().enumerate() {
            *d = rd::<u32>(s, 4 * k as isize);
        }
        vel = local(&mut velb);
    }
    let mut y = bottom;
    while y <= top {
        let mut x = left;
        while x <= right {
            if do_bld != 0 {
                g::<u8>(0x9655E4).write(1);
                if check_camera_collision_buildings_call(x, y, bb, sps, spa, spb) != 0 {
                    result = 1;
                }
                g::<u8>(0x9655E4).write(0);
            }
            if do_veh != 0 && check_camera_collision_vehicles_call(x, y, bb, sps, spa, spb, vel) != 0 {
                result = 1;
            }
            if do_obj != 0 && check_camera_collision_objects_call(x, y, bb, sps, spa, spb) != 0 {
                result = 1;
            }
            x += 1;
        }
        y += 1;
    }
    g::<GPtr<u8>>(IGNORE_ENTITY).write(saved);
    result
}

orig! {
    fn check_camera_collision_buildings_call(x: i32, y: i32, bb: GPtr<u8>, sps: GPtr<u8>, spa: GPtr<u8>, spb: GPtr<u8>) -> u8
        = cdecl 0x41A820 => check_camera_collision_buildings;
    fn check_camera_collision_vehicles_call(x: i32, y: i32, bb: GPtr<u8>, sps: GPtr<u8>, spa: GPtr<u8>, spb: GPtr<u8>,
        vel: GPtr<u8>) -> u8 = cdecl 0x41A990 => check_camera_collision_vehicles;
    fn check_camera_collision_objects_call(x: i32, y: i32, bb: GPtr<u8>, sps: GPtr<u8>, spa: GPtr<u8>, spb: GPtr<u8>) -> u8
        = cdecl 0x41AB20 => check_camera_collision_objects;
    fn build_cache_of_camera_collision_call(spa: GPtr<u8>, spb: GPtr<u8>) -> u8
        = cdecl 0x41AC40 => build_cache_of_camera_collision;
}

/// `CCollision::CameraConeCastVsWorldCollision(const CColSphere& spA, const CColSphere& spB,
/// float& dist, float minDist)` (0x41B000, AL). Two 0x640-byte collision caches live on its stack;
/// their addresses go to 0x9655CC (current) and 0x9655C8 (scratch) before
/// `BuildCacheOfCameraCollision(spA, spB)`; false → false. Else a bisection on `t` in `[minDist, 1]`:
/// `mid = lo + (hi - lo) * 0.5` (f32 stored, the register value scaled), a sphere at `spA.c` with radius
/// `mid * spA.r` cast along `(spB.c - spA.c) * mid` (each difference and product stored f32) through
/// `SphereCastVsCaches(sphere, delta, count, current, &newCount (0), scratch)`; a hit → `hi = mid`,
/// the caches swap and `count = newCount`; a miss → `lo = mid`; repeat while `hi - lo > *0x8A5B10`
/// (`fcom; test ah, 0x41; je`, NaN stops). `*dist = mid`, 0xB6EC6C = the last radius → true. The
/// globals keep pointing at the dead stack caches, as in the original.
#[hook(addr = 0x41B000, conv = cdecl, name = "CCollision::CameraConeCastVsWorldCollision", expect = "81 EC C8 0C 00 00",
       unverified = "mvpe-coll-1", route = "L2", hazards = "uninit-locals", recs = "grove-walk,driving",
       note = "0x9655CC/0x9655C8 hold addresses of its stack caches; the cast sphere's surface bytes are never written")]
pub fn camera_cone_cast_vs_world_collision(spa: GPtr<u8>, spb: GPtr<u8>, dist: GPtr<u8>, min_dist: f32) -> u8 {
    let mut cache_a = [0u32; 0x190];
    let mut cache_b = [0u32; 0x190];
    let mut cur = local(&mut cache_a);
    let mut scratch = local(&mut cache_b);
    g::<GPtr<u8>>(0x9655CC).write(cur);
    g::<GPtr<u8>>(0x9655C8).write(scratch);
    let ok = build_cache_of_camera_collision_call(spa, spb);
    if ok == 0 {
        return 0;
    }
    let a = v3(spa);
    let bv = v3(spb);
    let d: [f32; 3] = core::array::from_fn(|k| X87::from_f32(bv[k]).fsub_m(a[k]).to_f32());
    let radius = rd::<f32>(spa, 0xC);
    let eps = kf(0x8A5B10);
    let mut count = g::<i32>(0x9655D8).read();
    let mut lo = min_dist;
    let mut hi = kf(K_1);
    let mut mid;
    let mut sph_r;
    let mut f = Fpu::new();
    f.fld(kf(K_1));
    f.fsub_m(min_dist);
    loop {
        f.fmul_m(kf(0x858B8C));
        f.fadd_m(lo);
        mid = f.fst();
        f.fmul_m(radius);
        sph_r = f.fstp();
        let mut sb = [a[0].to_bits(), a[1].to_bits(), a[2].to_bits(), sph_r.to_bits(), 0];
        let mut db = [0u32; 3];
        for (k, o) in db.iter_mut().enumerate() {
            f.fld(d[k]);
            f.fmul_m(mid);
            *o = f.fstp().to_bits();
        }
        let mut nb = [0u32; 1];
        if sphere_cast_vs_caches_call(local(&mut sb), local(&mut db), count, cur, local(&mut nb), scratch) != 0 {
            count = nb[0] as i32;
            core::mem::swap(&mut cur, &mut scratch);
            hi = mid;
        } else {
            lo = mid;
        }
        f.fld(hi);
        f.fsub_m(lo);
        f.fcom_m(eps);
        if f.jne(0x41) {
            break;
        }
    }
    wr::<f32>(dist, 0, mid);
    g::<f32>(0xB6EC6C).write(sph_r);
    ok
}

/// `CCollision::SphereVsEntity(CColSphere* sphere, CEntity* entity)` (0x41A5A0, AL; SEH frame).
/// False unless the entity uses collision (+0x1C bit 0) and is not `TheCamera.IsExtraEntityToIgnore`.
/// The matrix is allocated / updated when missing; `Invert(entity matrix)` into a local `CMatrix` that is
/// destroyed unused. Original bug kept: the `sphere` argument is never read; every test uses a local
/// `CColSphere` the function never writes (stack garbage in the original, zero here). Against the
/// entity's col data (null → false): spheres from last to first ([`cam_spheres_overlap`]-style,
/// `(r + r')^2 > (dz*dz + dy*dy) + dx*dx`), boxes from last to first (`TestSphereBox`),
/// `CalculateTrianglePlanes`, then triangles (none → false): with face groups (flags +7 bit 1; count at
/// `triangles - 4`, group `i` at `triangles - 0x20 - 0x1C*i`: box, i16 first +0x18, i16 last +0x1A)
/// each group whose box passes `TestSphereBox` tests its triangles `first..=last`; without, triangles
/// from last to first (`TestSphereTriangle`). Any hit → true.
#[hook(addr = 0x41A5A0, conv = cdecl, name = "CCollision::SphereVsEntity", expect = "64 A1 00 00 00 00",
       unverified = "mvpe-coll-1", route = "split", hazards = "uninit-locals", recs = "grove-walk,driving",
       note = "1.0 bug: tests an uninitialised local CColSphere instead of its sphere argument (needs SPLIT_STACKFILL)")]
pub fn sphere_vs_entity(_sphere: GPtr<u8>, e: GPtr<u8>) -> u8 {
    if rd::<u8>(e, 0x1C) & 1 == 0 || is_extra_entity_to_ignore(GPtr::from_addr(0xB6F028), e) != 0 {
        return 0;
    }
    if rd::<u32>(e, 0x14) == 0 {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    let mut mb = [0u32; 0x12];
    let m = local(&mut mb);
    matrix_invert(rd::<GPtr<u8>>(e, 0x14), m);
    matrix_dtor(m);
    // the never-written local sphere (S+0x28)
    let mut sb = [0u32; 5];
    let sp = local(&mut sb);
    let cd = rd::<GPtr<u8>>(get_col_model(e), 0x2C);
    if cd.is_null() {
        return 0;
    }
    let mut i = rd::<i16>(cd, 0) as i32 - 1;
    while i >= 0 {
        let s = rd::<GPtr<u8>>(cd, 8).byte_add(i as isize * 0x14);
        if cam_spheres_overlap(sp, s) {
            return 1;
        }
        i -= 1;
    }
    let mut i = rd::<i16>(cd, 2) as i32 - 1;
    while i >= 0 {
        let bx = rd::<GPtr<u8>>(cd, 0xC).byte_add(i as isize * 0x1C);
        if test_sphere_box(sp.cast(), bx.cast()) != 0 {
            return 1;
        }
        i -= 1;
    }
    calculate_triangle_planes(cd.cast());
    let ntri = rd::<i16>(cd, 4);
    let verts = rd::<GPtr<u8>>(cd, 0x14);
    let planes = rd::<GPtr<u8>>(cd, 0x1C);
    let tris = rd::<GPtr<u8>>(cd, 0x18);
    if ntri == 0 {
        return 0;
    }
    if rd::<u8>(cd, 7) & 2 != 0 {
        let mut gi = 0i32;
        loop {
            let count = if rd::<u8>(cd, 7) & 2 != 0 { rd::<i32>(rd::<GPtr<u8>>(cd, 0x18), -4) } else { 0 };
            if gi >= count {
                return 0;
            }
            let grp = rd::<GPtr<u8>>(cd, 0x18).byte_add(-0x20 - 0x1C * gi as isize);
            if test_sphere_box(sp.cast(), grp.cast()) != 0 {
                let mut t = rd::<i16>(grp, 0x18) as i32;
                while t <= rd::<i16>(grp, 0x1A) as i32 {
                    if test_sphere_triangle_call(sp, verts, tris.byte_add(8 * t as isize), planes.byte_add(0xA * t as isize)) != 0 {
                        return 1;
                    }
                    t += 1;
                }
            }
            gi += 1;
        }
    }
    let mut t = ntri as i32 - 1;
    while t >= 0 {
        if test_sphere_triangle_call(sp, verts, tris.byte_add(8 * t as isize), planes.byte_add(0xA * t as isize)) != 0 {
            return 1;
        }
        t -= 1;
    }
    0
}

/// `CCollision::SphereCastVsCaches(CColSphere* sphere, CVector* dir, int32 count,
/// CColCacheEntry* cache, int32* nOut, CColCacheEntry* outCache)` (0x4181B0, AL; SEH frame). The
/// second sphere is `sphere.c + dir` (each sum stored f32) with `sphere.r` and surface bytes the
/// original never writes (zero here). For each of the `count` 16-byte entries (type byte +0, entity
/// +4, u16 triangle +8 / sphere +0xA / box +0xC): with an entity, both spheres are reset to their
/// world copies, its matrix allocated/updated if missing, its col data (null → next entry) becomes
/// the current model, a local `Invert(matrix)` moves `CColLine(s1.c, s2.c)` into model space (end
/// transformed first, then a new `CColLine(start', end')`) and the spheres' centres are replaced;
/// `outCache[*nOut]` gets the entity and type 0 (not counted); the local matrix is destroyed. An entry
/// without entity keeps the previous model and spheres. Then by type: 3 → `SphereCastVsBBox(s1, s2,
/// box)`, 2 → `SphereCastVsSphere(s1, s2, sphere)`, 1 → `SphereCastVersusVsPoly(s1, s2, tri, plane,
/// verts)` for index < 0x7FFF, else with index `0xFFFF - idx` and the spheres swapped; a hit appends
/// the index (+0xC / +0xA / +8) and type to `outCache[(*nOut)++]` and pre-arms the next entry
/// (entity 0, type 0). Returns `*nOut > 0` (signed).
#[hook(addr = 0x4181B0, conv = cdecl, name = "CCollision::SphereCastVsCaches", expect = "6A FF 68 D8 71 83 00",
       unverified = "mvpe-coll-1", route = "L2", hazards = "uninit-locals,heap-writes", recs = "grove-walk,driving",
       note = "the cast end sphere's surface bytes and the model pointer before the first entity are never written; writes outCache (camera stack caches)")]
pub fn sphere_cast_vs_caches(sp: GPtr<u8>, dir: GPtr<u8>, count: i32, cache: GPtr<u8>, n_out: GPtr<u8>, out: GPtr<u8>) -> u8 {
    let s1w: [u32; 5] = core::array::from_fn(|k| rd::<u32>(sp, 4 * k as isize));
    let end: [f32; 3] =
        core::array::from_fn(|k| X87::from_f32(rd::<f32>(sp, 4 * k as isize)).fadd_m(rd::<f32>(dir, 4 * k as isize)).to_f32());
    let s2w: [u32; 5] = [end[0].to_bits(), end[1].to_bits(), end[2].to_bits(), s1w[3], 0];
    let mut s1m = s1w;
    let mut s2m = s2w;
    let mut cd = GPtr::<u8>::NULL;
    let mut p = cache.byte_add(4);
    let mut left = count;
    if left > 0 {
        loop {
            let e = rd::<GPtr<u8>>(p, 0);
            let mut go = true;
            if !e.is_null() {
                s1m = s1w;
                s2m = s2w;
                if rd::<u32>(e, 0x14) == 0 {
                    allocate_matrix(e);
                    simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
                }
                let m = rd::<GPtr<u8>>(e, 0x14);
                cd = rd::<GPtr<u8>>(get_col_model(e), 0x2C);
                if cd.is_null() {
                    go = false;
                } else {
                    let mut mb = [0u32; 0x12];
                    let inv = local(&mut mb);
                    matrix_invert(m, inv);
                    let mut lb = [0u32; 8];
                    let l = local(&mut lb);
                    col_line_ctor(l, local(&mut s1m), local(&mut s2m));
                    let mut eb = [0u32; 3];
                    let pe = matrix_mul_vec(local(&mut eb), inv, l.byte_add(0x10));
                    let mut sb = [0u32; 3];
                    let ps = matrix_mul_vec(local(&mut sb), inv, l);
                    let mut l2b = [0u32; 8];
                    let l2 = local(&mut l2b);
                    col_line_ctor(l2, ps, pe);
                    s1m[..3].copy_from_slice(&l2b[..3]);
                    s2m[..3].copy_from_slice(&l2b[4..7]);
                    let n = rd::<i32>(n_out, 0);
                    let o = out.byte_add(16 * n as isize);
                    wr::<GPtr<u8>>(o, 4, e);
                    wr::<u8>(o, 0, 0);
                    matrix_dtor(inv);
                }
            }
            if go {
                let ty = rd::<u8>(p, -4);
                let hit = match ty {
                    3 => {
                        let idx = rd::<u16>(p, 8);
                        let bx = rd::<GPtr<u8>>(cd, 0xC).byte_add(idx as isize * 0x1C);
                        (sphere_cast_vs_bbox(local(&mut s1m).cast(), local(&mut s2m).cast(), bx.cast()) != 0)
                            .then_some((0xC, idx, 3u8))
                    }
                    2 => {
                        let idx = rd::<u16>(p, 6);
                        let s = rd::<GPtr<u8>>(cd, 8).byte_add(idx as isize * 0x14);
                        (sphere_cast_vs_sphere(local(&mut s1m).cast(), local(&mut s2m).cast(), s.cast()) != 0)
                            .then_some((0xA, idx, 2u8))
                    }
                    1 => {
                        let idx = rd::<u16>(p, 4);
                        let verts = rd::<GPtr<u8>>(cd, 0x14);
                        let tris = rd::<GPtr<u8>>(cd, 0x18);
                        let planes = rd::<GPtr<u8>>(cd, 0x1C);
                        let r = if idx < 0x7FFF {
                            sphere_cast_versus_vs_poly(local(&mut s1m), local(&mut s2m), tris.byte_add(8 * idx as isize),
                                planes.byte_add(0xA * idx as isize), verts)
                        } else {
                            let j = 0xFFFF - idx;
                            sphere_cast_versus_vs_poly(local(&mut s2m), local(&mut s1m), tris.byte_add(8 * j as isize),
                                planes.byte_add(0xA * j as isize), verts)
                        };
                        (r != 0).then_some((8, idx, 1u8))
                    }
                    _ => None,
                };
                if let Some((off, idx, t)) = hit {
                    let n = rd::<i32>(n_out, 0);
                    let o = out.byte_add(16 * n as isize);
                    wr::<i32>(n_out, 0, n.wrapping_add(1));
                    wr::<u16>(o, off, idx);
                    wr::<u8>(o, 0, t);
                    let o2 = out.byte_add(16 * rd::<i32>(n_out, 0) as isize);
                    wr::<u32>(o2, 4, 0);
                    wr::<u8>(o2, 0, 0);
                }
            }
            p = p.byte_add(0x10);
            left -= 1;
            if left == 0 {
                break;
            }
        }
    }
    (rd::<i32>(n_out, 0) > 0) as u8
}

pub const HOOKS: &[HookReg] = &[
    HOOK_TEST_SPHERE_TRIANGLE,
    HOOK_PROCESS_SPHERE_TRIANGLE,
    HOOK_PROCESS_LINE_BOX,
    HOOK_CHECK_CAMERA_COLLISION_PEDS,
    HOOK_CHECK_CAMERA_COLLISION_BUILDINGS,
    HOOK_CHECK_CAMERA_COLLISION_OBJECTS,
    HOOK_BUILD_CACHE_OF_CAMERA_COLLISION,
    HOOK_CAMERA_CONE_CAST_VS_WORLD_COLLISION,
    HOOK_SPHERE_VS_ENTITY,
    HOOK_SPHERE_CAST_VS_CACHES,
];
