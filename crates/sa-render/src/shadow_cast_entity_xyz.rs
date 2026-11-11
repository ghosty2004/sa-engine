//! `CShadows::CastShadowEntityXYZ` (batch `mvpm-render-4`): projects a real-time shadow onto one entity's collision
//! triangles. Ported from the 1.0 US bytes (`docs/re/specs/RENDER/CShadows.md` §0x70A040; gta-reversed only calls the
//! original). Unverified (docs/verify/HOWTO.md).
//!
//! `_ProjectionParam` (0x98, the callback's EDI): +0 at (CVector), +0xC inverse projector matrix (RwMatrix), +0x4C
//! shadow value (u8), +0x50 fade (RwBool), +0x54 numIm3DBatch (left uninitialised by the original), +0x58 entity
//! matrix (RwMatrix).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, wr, x};

orig! {
    /// `CRealTimeShadow::GetShadowCamera()` (0x7059E0)
    fn rt_get_shadow_camera(rt: GPtr<u8>) -> GPtr<u8> = thiscall 0x7059E0 => unported;
    /// `CCollision::CalculateTrianglePlanes(CColModel*)` (0x418580)
    fn collision_calculate_triangle_planes(cm: GPtr<u8>) = cdecl 0x418580 => unported;
    /// `RwMatrixRotate(RwMatrix*, const RwV3d* axis, float angle, RwOpCombineType)` (0x7F1FD0)
    fn rw_matrix_rotate(m: GPtr<u8>, axis: GPtr<u8>, angle: u32, combine: u32) -> GPtr<u8> = cdecl 0x7F1FD0 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*) const` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CMatrix::CopyToRwMatrix(RwMatrix*) const` (0x59B8B0, `ret 4`)
    fn matrix_copy_to_rw_matrix(m: GPtr<u8>, out: GPtr<u8>) = thiscall 0x59B8B0 => unported;
    /// `RwMatrixInvert(RwMatrix* dst, const RwMatrix* src)` (0x7F2070)
    fn rw_matrix_invert(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F2070 => unported;
    /// `RwMatrixScale(RwMatrix*, const RwV3d*, RwOpCombineType)` (0x7F22C0)
    fn rw_matrix_scale(m: GPtr<u8>, v: GPtr<u8>, combine: u32) -> GPtr<u8> = cdecl 0x7F22C0 => unported;
    /// `RwMatrixTranslate(RwMatrix*, const RwV3d*, RwOpCombineType)` (0x7F2450)
    fn rw_matrix_translate(m: GPtr<u8>, v: GPtr<u8>, combine: u32) -> GPtr<u8> = cdecl 0x7F2450 => unported;
    /// `RwV3dTransformPoints(RwV3d* out, const RwV3d* in, int32 n, const RwMatrix*)` (0x7EDD90)
    fn rw_v3d_transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
    /// `CColSphere::Set(float radius, const CVector& centre, uint8 material, uint8 flags, uint8 lighting)` (0x40FD10,
    /// `ret 0x14`)
    fn col_sphere_set(s: GPtr<u8>, radius: u32, centre: GPtr<u8>, material: u32, flags: u32, lighting: u32) = thiscall 0x40FD10 => unported;
    /// `CCollisionData::GetTrianglePoint(CVector& out, int32 vertex)` (0x40F5E0, `ret 8`)
    fn col_data_get_triangle_point(cd: GPtr<u8>, out: GPtr<u8>, v: u32) = thiscall 0x40F5E0 => unported;
    /// `CCollision::TestSphereTriangle(const CColSphere&, const CompressedVector* verts, const CColTriangle&, const
    /// CColTrianglePlane&)` (0x4165B0), AL
    fn collision_test_sphere_triangle(s: GPtr<u8>, verts: GPtr<u8>, tri: GPtr<u8>, plane: GPtr<u8>) -> u8 = cdecl 0x4165B0 => unported;
    /// `ShadowRenderTriangleCB(CVector* normal (EAX), CVector* tri (EBX), _ProjectionParam* (EDI))` (0x709CF0): returns
    /// the triangle pointer in EAX
    fn shadow_render_triangle_cb(n: GPtr<u8>, t: GPtr<u8>, p: GPtr<u8>) -> GPtr<u8> = reg("args = eax ebx edi") 0x709CF0 => crate::harness5::shadow_render_triangle_cb;
}

/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8)
const MODEL_INFOS: u32 = 0xA9B0C8;
/// 0.9f (0x858C20), -0.5f (0x858F40), 0.8f (0x858C98), 1.0f (0x858624), -1.1f (0x871654), 0.5f (0x858B8C),
/// 1/4096 (0x858BFC), 0.028f (0x872720)
const K_09: usize = 0x858C20;
const K_M05: usize = 0x858F40;
const K_08: usize = 0x858C98;
const K_ONE: usize = 0x858624;
const K_M11: usize = 0x871654;
const K_HALF: usize = 0x858B8C;
const K_NORMAL: usize = 0x858BFC;
const K_LIFT: usize = 0x872720;

fn kf(a: usize) -> f32 {
    g::<f32>(a).read()
}
fn p<T>(a: &mut [T]) -> GPtr<u8> {
    GPtr::from_raw(a.as_mut_ptr()).cast()
}
/// `if (!e->m_matrix) { AllocateMatrix(); m_placement.UpdateMatrix(m_matrix); }` then `CopyToRwMatrix(out)`.
fn entity_rw_matrix(e: GPtr<u8>, out: GPtr<u8>) {
    if rd::<u32>(e, 0x14) == 0 {
        placeable_allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    matrix_copy_to_rw_matrix(rd::<GPtr<u8>>(e, 0x14), out);
}

/// `CShadows::CastShadowEntityXYZ(CEntity*, CVector* posn, float frontX, float frontY, float sideX, float sideY, int16
/// intensity, uint8 r, uint8 g, uint8 b, float zDistance, float scale, CPolyBunch**, CRealTimeShadow* rt)` (0x70A040,
/// cdecl). Only the entity, frontX/frontY, the intensity's low byte and `rt` are read.
///
/// Nothing without `rt` or without collision data (model info +0x14 col model, +0x2C data). `CalculateTrianglePlanes`;
/// the projector `M` = the shadow camera frame's modelling matrix rotated by -45 degrees about (1, 0, 0) (pre-concat);
/// `P.at = M.at`; `P.entity = e` as an RwMatrix; `P.inv = M^-1` scaled (post) by `(-0.5 / (0.9 vw), same, 1 / (vw 0.8))`
/// (`vw` = camera +0x68) and translated (post) by (0.5, 0, 0); fade 0, shadow value = intensity. The test sphere: radius
/// 2 around `E^-1 * (c.x + frontX * -1.1, c.y + frontY * -1.1, c.z - 0.5)` with `c` = `rt` +0x3C. Every collision
/// triangle (count re-read) that `TestSphereTriangle` accepts: `N` = the i16 plane normal / 4096 (stored), the three
/// points lifted by `N * 0.028` (x/y offsets unrounded, z stored), then `ShadowRenderTriangleCB(&N, T, &P)` (a null
/// result would stop).
#[hook(addr = 0x70a040, conv = cdecl, name = "CShadows::CastShadowEntityXYZ", expect = "8B 4C 24 38 81 EC DC 01 00 00",
       unverified = "mvpm-render-4", route = "L2", hazards = "uninit-locals,heap-writes", recs = "grove-walk,driving",
       note = "_ProjectionParam +0x54 is uninitialised in the original (zero here; the callback does not read it); ShadowRenderTriangleCB is a register-convention callee (EAX, EBX, EDI)")]
#[allow(clippy::too_many_arguments)]
pub fn cast_shadow_entity_xyz(
    e: GPtr<u8>,
    _posn: GPtr<u8>,
    front_x: f32,
    front_y: f32,
    _side_x: f32,
    _side_y: f32,
    intensity: u32,
    _r: u32,
    _g: u32,
    _b: u32,
    _z_distance: f32,
    _scale: f32,
    _pp: GPtr<u8>,
    rt: GPtr<u8>,
) {
    if rt.is_null() {
        return;
    }
    let cam = rt_get_shadow_camera(rt);
    let mi_idx = rd::<i16>(e, 0x22) as i32;
    let mi = g::<GPtr<u8>>((MODEL_INFOS as i32).wrapping_add(mi_idx.wrapping_mul(4)) as u32 as usize).read();
    let cm = rd::<GPtr<u8>>(mi, 0x14);
    let cd = rd::<GPtr<u8>>(cm, 0x2C);
    if cd.is_null() {
        return;
    }
    collision_calculate_triangle_planes(cm);
    // The projector matrix (a copy of the camera frame's modelling matrix, rotated).
    let mut m = [0u32; 16];
    let frame = rd::<GPtr<u8>>(rd::<GPtr<u8>>(cam, 0), 4);
    for (i, w) in m.iter_mut().enumerate() {
        *w = rd::<u32>(frame, 0x10 + 4 * i as isize);
    }
    let mut axis = [1f32, 0.0, 0.0];
    rw_matrix_rotate(p(&mut m), p(&mut axis), 0xC234_0000, 1);
    let mut pp = [0u8; 0x98];
    let pr = p(&mut pp);
    for i in 0..3 {
        wr::<u32>(pr, 4 * i, m[8 + i as usize]);
    }
    entity_rw_matrix(e, pr.byte_add(0x58));
    rw_matrix_invert(pr.byte_add(0xC), p(&mut m));
    let vw = rd::<f32>(rd::<GPtr<u8>>(cam, 0), 0x68);
    let sxy = x(kf(K_09)).fmul(x(vw)).fdivr_m(kf(K_M05)).to_f32();
    let sz = x(vw).fmul_m(kf(K_08)).fdivr_m(kf(K_ONE)).to_f32();
    let mut s = [sxy, sxy, sz];
    rw_matrix_scale(pr.byte_add(0xC), p(&mut s), 2);
    let mut t = [0.5f32, 0.0, 0.0];
    rw_matrix_translate(pr.byte_add(0xC), p(&mut t), 2);
    wr::<u32>(pr, 0x50, 0);
    wr::<u8>(pr, 0x4C, intensity as u8);
    // The test sphere in object space.
    let mut em = [0u32; 16];
    let mut emi = [0u32; 16];
    entity_rw_matrix(e, p(&mut em));
    rw_matrix_invert(p(&mut emi), p(&mut em));
    let ox = x(front_x).fmul_m(kf(K_M11));
    let oy = x(front_y).fmul_m(kf(K_M11));
    let mut c = [
        ox.fadd_m(rd::<f32>(rt, 0x3C)).to_f32(),
        x(rd::<f32>(rt, 0x40)).fadd(oy).to_f32(),
        x(rd::<f32>(rt, 0x44)).fsub_m(kf(K_HALF)).to_f32(),
    ];
    let mut l = [0f32; 3];
    rw_v3d_transform_points(p(&mut l), p(&mut c), 1, p(&mut emi));
    let mut sphere = [0u32; 6];
    col_sphere_set(p(&mut sphere), 2f32.to_bits(), p(&mut l), 0, 0, 0xFF);
    // The triangles.
    let mut i: i32 = 0;
    let mut plane_off: u32 = 0;
    while i < i32::from(rd::<i16>(cd, 4)) {
        let tris = rd::<u32>(cd, 0x18);
        let tri = GPtr::<u8>::from_addr(tris.wrapping_add((i as u32).wrapping_mul(8)) as usize);
        let (va, vb, vc) = (rd::<u16>(tri, 0), rd::<u16>(tri, 2), rd::<u16>(tri, 4));
        let mut tv = [0f32; 9];
        let tp = p(&mut tv);
        col_data_get_triangle_point(cd, tp, u32::from(va));
        col_data_get_triangle_point(cd, tp.byte_add(0xC), u32::from(vb));
        col_data_get_triangle_point(cd, tp.byte_add(0x18), u32::from(vc));
        let planes = rd::<u32>(cd, 0x1C);
        let tri = GPtr::<u8>::from_addr(rd::<u32>(cd, 0x18).wrapping_add((i as u32).wrapping_mul(8)) as usize);
        let hit = collision_test_sphere_triangle(
            p(&mut sphere),
            rd::<GPtr<u8>>(cd, 0x14),
            tri,
            GPtr::from_addr(planes.wrapping_add(plane_off) as usize),
        );
        if hit != 0 {
            let pl = GPtr::<u8>::from_addr(rd::<u32>(cd, 0x1C).wrapping_add(plane_off) as usize);
            let kn = kf(K_NORMAL);
            let mut n = [
                X87::from_i32(i32::from(rd::<i16>(pl, 0))).fmul_m(kn).to_f32(),
                X87::from_i32(i32::from(rd::<i16>(pl, 2))).fmul_m(kn).to_f32(),
                X87::from_i32(i32::from(rd::<i16>(pl, 4))).fmul_m(kn).to_f32(),
            ];
            let kl = kf(K_LIFT);
            let lx = x(n[0]).fmul_m(kl);
            let ly = x(n[1]).fmul_m(kl);
            let lz = x(n[2]).fmul_m(kl).to_f32();
            for v in 0..2 {
                tv[3 * v] = lx.fadd_m(tv[3 * v]).to_f32();
                tv[3 * v + 1] = x(tv[3 * v + 1]).fadd(ly).to_f32();
                tv[3 * v + 2] = x(tv[3 * v + 2]).fadd_m(lz).to_f32();
            }
            tv[6] = lx.fadd_m(tv[6]).to_f32();
            tv[7] = x(tv[7]).fadd(ly).to_f32();
            tv[8] = x(tv[8]).fadd_m(lz).to_f32();
            if shadow_render_triangle_cb(p(&mut n), p(&mut tv), pr).is_null() {
                return;
            }
        }
        i += 1;
        plane_off = plane_off.wrapping_add(0xA);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_CAST_SHADOW_ENTITY_XYZ];
