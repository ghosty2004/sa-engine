//! `CShadows::CastShadowEntityXYZ` (0x70A040), the real-time shadow projection of one entity's
//! collision triangles. Ported from the 1.0 US bytes (spec `docs/re/specs/RENDER/CShadows.md`); its
//! per-triangle callback `ShadowRenderTriangleCB` (0x709CF0) takes EAX / EBX / EDI and is called
//! through `orig! { ... = reg("args = eax ebx edi") }` (HARNESS-5).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_world::collision::data::CdPtr;

use crate::common::{g, k, rd, wr, x, xi};

use sa_core::bindings::matrix_copy_to_rw_matrix as copy_to_rw_matrix;
use sa_rwcore::bindings::rw_matrix_invert;
use sa_core::bindings::rw_v3d_transform_points;
use sa_world::bindings::calculate_triangle_planes;
use sa_core::bindings::rw_matrix_scale;
use sa_core::bindings::rw_matrix_translate;
use sa_world::bindings::test_sphere_triangle;
use sa_world::bindings::col_sphere_set;
use sa_core::bindings::rw_matrix_rotate;
use sa_world::bindings::placeable_allocate_matrix as allocate_matrix;
use crate::bindings::simple_transform_update_matrix;
use sa_world::bindings::get_triangle_point;
orig! {
    /// `CRealTimeShadow::GetShadowCamera()` (0x7059E0)
    fn get_shadow_camera(rt: GPtr<u8>) -> GPtr<u8> = thiscall 0x7059E0 => unported;
    /// `ShadowRenderTriangleCB` (0x709CF0): EAX = normal, EBX = triangle, EDI = projection param; returns EBX
    fn shadow_render_triangle_cb(normal: GPtr<u8>, tri: GPtr<u8>, param: GPtr<u8>) -> GPtr<u8> = reg("args = eax ebx edi") 0x709CF0 => crate::harness5::shadow_render_triangle_cb;
}

/// `CModelInfo::ms_modelInfoPtrs`.
const MODEL_INFO_PTRS: usize = 0xA9_B0C8;
/// -45.0f (pushed), 0.9f (0x858C20), -0.5f (0x858F40), 0.8f (0x858C98), 1.0f (0x858624), -1.1f
/// (0x871654), 0.5f (0x858B8C), 1/4096 (0x858BFC), 0.028f (0x872720), 2.0f (pushed).
const MINUS_45: u32 = 0xC234_0000;
const K09: u32 = 0x3F66_6666;
const MINUS_HALF: u32 = 0xBF00_0000;
const K08: u32 = 0x3F4C_CCCD;
const ONE: u32 = 0x3F80_0000;
const MINUS_1_1: u32 = 0xBF8C_CCCD;
const HALF: u32 = 0x3F00_0000;
const NORMAL_SCALE: u32 = 0x3980_0000;
const LIFT: u32 = 0x3CE5_6042;

/// `e->GetMatrix()` made current (`AllocateMatrix` + `m_placement.UpdateMatrix` when it has none),
/// copied into the RwMatrix `out`.
fn entity_rw_matrix(e: GPtr<u8>, out: GPtr<u8>) {
    if rd::<u32>(e, 0x14) == 0 {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    copy_to_rw_matrix(rd::<GPtr<u8>>(e, 0x14), out);
}

/// `CShadows::CastShadowEntityXYZ(CEntity* e, CVector* posn, float frontX, float frontY, float sideX,
/// float sideY, int16 intensity, uint8 r, g, b, float zDistance, float scale, CPolyBunch** pp,
/// CRealTimeShadow* rt)` (0x70A040, cdecl; the only caller pushes a 15th argument). Only `e`,
/// `frontX`, `frontY`, `intensity` and `rt` are read. Nothing without `rt` or without collision data.
/// The projection param (0x98 bytes): the shadow camera frame's modelling matrix rotated by -45°
/// about x (pre-concat) gives `at` (+0); +0x58 the entity matrix; +0xC its inverse, scaled by
/// `(-0.5 / (0.9 * vw), same, 1 / (vw * 0.8))` (vw = the RwCamera's view window x, +0x68) and
/// translated by (0.5, 0, 0) (post-concat); `shadowValue` (+0x4C, a byte) = intensity, `fade` (+0x50)
/// = 0. The test sphere (radius 2, material 0, piece 0, lighting 0xFF) is centred on
/// `rt->m_baseSphere.center` (+0x3C) moved by `front * -1.1` in x/y and -0.5 in z, in entity space
/// (inverse entity matrix). Every collision triangle the sphere touches is lifted along its plane
/// normal (`n = plane * 1/4096`, stored f32; offset `n * 0.028`, x / y unrounded, z stored) and passed
/// to `ShadowRenderTriangleCB`; the triangle count (+4, i16) is re-read per iteration.
#[hook(addr = 0x70A040, conv = cdecl, name = "CShadows::CastShadowEntityXYZ", expect = "8B 4C 24 38 81 EC DC 01 00 00",
       unverified = "regconv-render-1", route = "L2", hazards = "heap-writes", recs = "grove-walk,driving",
       note = "calls ShadowRenderTriangleCB (EAX/EBX/EDI) through orig! reg; the projection param's numIm3DBatch dword (+0x54) and bytes +0x4D..+0x4F stay uninitialised in the original (not read by the callback)")]
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
    let cam = get_shadow_camera(rt);
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * rd::<i16>(e, 0x22) as isize as usize).read();
    let cm = rd::<GPtr<u8>>(mi, 0x14);
    let cd = rd::<GPtr<u8>>(cm, 0x2C);
    if cd.is_null() {
        return;
    }
    calculate_triangle_planes(cm.cast::<sa_world::collision::shared::CColModel>());
    let rw_cam = rd::<GPtr<u8>>(cam, 0);
    let frame = rd::<GPtr<u8>>(rw_cam, 4);
    let mut m = [0u32; 16];
    for (i, w) in m.iter_mut().enumerate() {
        *w = rd::<u32>(frame, 0x10 + 4 * i as isize);
    }
    let mp: GPtr<u8> = GPtr::from_raw(m.as_mut_ptr()).cast();
    let mut axis = [ONE, 0, 0];
    rw_matrix_rotate(mp, GPtr::from_raw(axis.as_mut_ptr()).cast(), k(MINUS_45), 1);
    let mut param = [0u32; 0x98 / 4];
    let pp: GPtr<u8> = GPtr::from_raw(param.as_mut_ptr()).cast();
    wr::<u32>(pp, 0, m[8]);
    wr::<u32>(pp, 4, m[9]);
    wr::<u32>(pp, 8, m[10]);
    entity_rw_matrix(e, pp.byte_add(0x58));
    rw_matrix_invert(pp.byte_add(0xC), mp);
    let vw = x(rd::<f32>(rw_cam, 0x68));
    let sxy = x(k(K09)).fmul(vw).fdivr_m(k(MINUS_HALF)).to_f32();
    let sz = vw.fmul_m(k(K08)).fdivr_m(k(ONE)).to_f32();
    let mut s = [sxy, sxy, sz];
    rw_matrix_scale(pp.byte_add(0xC), GPtr::from_raw(s.as_mut_ptr()).cast(), 2);
    let mut t = [HALF, 0, 0];
    rw_matrix_translate(pp.byte_add(0xC), GPtr::from_raw(t.as_mut_ptr()).cast(), 2);
    wr::<u32>(pp, 0x50, 0);
    wr::<u8>(pp, 0x4C, intensity as u8);

    let mut em = [0u32; 16];
    let emp: GPtr<u8> = GPtr::from_raw(em.as_mut_ptr()).cast();
    entity_rw_matrix(e, emp);
    let mut ei = [0u32; 16];
    let eip: GPtr<u8> = GPtr::from_raw(ei.as_mut_ptr()).cast();
    rw_matrix_invert(eip, emp);
    let ox = x(front_x).fmul_m(k(MINUS_1_1));
    let oy = x(front_y).fmul_m(k(MINUS_1_1));
    let mut c = [
        ox.fadd_m(rd::<f32>(rt, 0x3C)).to_f32(),
        x(rd::<f32>(rt, 0x40)).fadd(oy).to_f32(),
        x(rd::<f32>(rt, 0x44)).fsub_m(k(HALF)).to_f32(),
    ];
    let mut l = [0f32; 3];
    rw_v3d_transform_points(GPtr::<u8>::cast::<f32>(GPtr::from_raw(l.as_mut_ptr()).cast()), GPtr::<u8>::cast::<f32>(GPtr::from_raw(c.as_mut_ptr()).cast()), 1, eip);
    let mut centre = [l[0].to_bits(), l[1].to_bits(), l[2].to_bits()];
    let mut sphere = [0u32; 0x14 / 4];
    let sp: GPtr<u8> = GPtr::from_raw(sphere.as_mut_ptr()).cast();
    col_sphere_set(sp, f32::from_bits(0x4000_0000).to_bits(), GPtr::from_raw(centre.as_mut_ptr()), 0, 0, 0xFF);

    let cdp = CdPtr::from_addr(cd.addr());
    let mut i: i32 = 0;
    while i < i32::from(rd::<i16>(cd, 4)) {
        let tri = rd::<GPtr<u8>>(cd, 0x18).byte_add(8 * i as isize);
        let (va, vb, vc) = (rd::<u16>(tri, 0), rd::<u16>(tri, 2), rd::<u16>(tri, 4));
        let mut tp = [0f32; 9];
        let tpp: GPtr<u8> = GPtr::from_raw(tp.as_mut_ptr()).cast();
        get_triangle_point(cdp.cast::<sa_world::collision::shared::CCollisionData>(), tpp.cast(), i32::from(va));
        get_triangle_point(cdp.cast::<sa_world::collision::shared::CCollisionData>(), tpp.byte_add(0xC).cast(), i32::from(vb));
        get_triangle_point(cdp.cast::<sa_world::collision::shared::CCollisionData>(), tpp.byte_add(0x18).cast(), i32::from(vc));
        let plane = rd::<GPtr<u8>>(cd, 0x1C).byte_add(10 * i as isize);
        if test_sphere_triangle(sp, rd::<GPtr<u8>>(cd, 0x14), tri, plane) != 0 {
            let n_of = |o: isize| xi(i32::from(rd::<i16>(plane, o))).fmul_m(k(NORMAL_SCALE)).to_f32();
            let mut n = [n_of(0), n_of(2), n_of(4)];
            let lx = x(n[0]).fmul_m(k(LIFT));
            let ly = x(n[1]).fmul_m(k(LIFT));
            let lz = x(n[2]).fmul_m(k(LIFT)).to_f32();
            for v in 0..3 {
                tp[3 * v] = lx.fadd_m(tp[3 * v]).to_f32();
                tp[3 * v + 1] = x(tp[3 * v + 1]).fadd(ly).to_f32();
                tp[3 * v + 2] = x(tp[3 * v + 2]).fadd_m(lz).to_f32();
            }
            if shadow_render_triangle_cb(GPtr::from_raw(n.as_mut_ptr()).cast(), tpp, pp).is_null() {
                return;
            }
        }
        i += 1;
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_CAST_SHADOW_ENTITY_XYZ];
