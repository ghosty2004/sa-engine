//! HARNESS-5: render helpers with compiler-generated register conventions, ported from the 1.0 US
//! bytes (specs: `docs/re/specs/RENDER/CShadows.md`, `CStencilShadows.md`, `CPlantMgr.md`,
//! `_unattributed-R1.md`) and hooked with `conv = reg(...)` (`sa_core::regconv`). All unverified.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, k, rd, wr, x};

orig! {
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CEntity::TransformFromObjectSpace(CVector& out, const CVector& offset)` (0x5334F0), returns `out`
    fn transform_from_object_space(e: GPtr<u8>, out: GPtr<u8>, offset: GPtr<u8>) -> GPtr<u8> = thiscall 0x5334F0 => unported;
    /// `RwV3dTransformPoints(RwV3d* out, const RwV3d* in, int n, const RwMatrix* m)` (0x7EDD90)
    fn rw_v3d_transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
    /// `RenderBuffer::StartStoring(int nIdx, int nVtx, RwImVertexIndex** idx, RwIm3DVertex** vtx)` (0x708290)
    fn render_buffer_start_storing(n_idx: i32, n_vtx: i32, idx: GPtr<u32>, vtx: GPtr<u32>) = cdecl 0x708290 => unported;
    /// `RenderBuffer::StopStoring()` (0x7077A0)
    fn render_buffer_stop_storing() = cdecl 0x7077A0 => unported;
}

/// 0.0f (0x858B50), 1.0f (0x858624), -1.0f (0x858C1C), 0.06f (0x859934).
const ZERO: f32 = 0.0;
const ONE: u32 = 0x3F80_0000;
const MINUS_ONE: u32 = 0xBF80_0000;
const SHADOW_Z_LIFT: u32 = 0x3D75_C28F;

/// A CVector of three f32 read from game memory.
fn v3(p: GPtr<u8>) -> [f32; 3] {
    [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
}

/// `sqrt((dz*dz + dy*dy) + dx*dx)` with `d = P - W` loaded `fld P; fsub W` per component (the x87
/// order of 0x5DBE5D.. / 0x71117D..).
fn dist(p: [f32; 3], w: [f32; 3]) -> X87 {
    let dx = x(p[0]).fsub_m(w[0]);
    let dy = x(p[1]).fsub_m(w[1]);
    let dz = x(p[2]).fsub_m(w[2]);
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt()
}

/// The entity's bounding-sphere centre in world space (`TransformFromObjectSpace` into a local) and
/// radius (`+0x24`), as both register helpers compute them.
fn bound_sphere(entity: GPtr<u8>) -> ([f32; 3], f32) {
    let cm = get_col_model(entity);
    let mut out = [0f32; 3];
    transform_from_object_space(entity, GPtr::from_raw(out.as_mut_ptr()).cast(), cm.byte_add(0x18));
    (out, rd::<f32>(cm, 0x24))
}

/// `CPlantMgr::_CalcDistanceSqrToEntity(CEntity* entity (EBX), const CVector* posn (ESI))`
/// (0x5DBE40): `d = |posn - centre|` (x87, not stored); if `d > r` (strict, `test ah, 0x41`) `d -= r`;
/// returns `d * d` in ST0. Preserves EBX/ESI/EDI.
#[hook(addr = 0x5DBE40, conv = reg("args = ebx esi"), name = "CPlantMgr::_CalcDistanceSqrToEntity",
       expect = "83 EC 10 57 8B CB",
       unverified = "harness5-1", route = "L2", hazards = "none", recs = "grove-walk,driving",
       note = "register convention EBX/ESI -> ST0 (HARNESS-5); callers CPlantMgr::_ColEntityCache_Update")]
pub fn calc_distance_sqr_to_entity(entity: GPtr<u8>, posn: GPtr<u8>) -> X87 {
    let (c, r) = bound_sphere(entity);
    let mut d = dist(v3(posn), c);
    if !d.fcom(x(r)).test_any(0x41) {
        d = d.fsub_m(r);
    }
    d.fmul(d)
}

/// The body of `CStencilShadows` 0x711160 (objects) and 0x7111F0 (vehicles), byte-identical copies:
/// `d = (f32)|point - centre|`; `s = (d < r) ? -1.0f : 1.0f` (`test ah, 5; jp`: unordered keeps 1.0);
/// returns `(s * (d - r)) * (d - r)` in ST0.
fn signed_sq_dist_to_bound_sphere(entity: GPtr<u8>, point: GPtr<u8>) -> X87 {
    let (c, r) = bound_sphere(entity);
    let d = dist(v3(point), c).to_f32();
    let s = if x(d).fcom(x(r)).test_parity(0x05) { x(k(ONE)) } else { x(k(MINUS_ONE)) };
    let t = x(d).fsub_m(r);
    s.fmul(t).fmul(t)
}

/// `CStencilShadows` helper 0x711160 (`GetSignedSqDistToBoundSphere_Object`; EBX = entity, ESI =
/// point, ST0 result): see [`signed_sq_dist_to_bound_sphere`].
#[hook(addr = 0x711160, conv = reg("args = ebx esi"), name = "FUN_00711160",
       expect = "83 EC 14 57 8B CB",
       unverified = "harness5-1", route = "L2", hazards = "none", recs = "grove-walk,driving",
       note = "register convention EBX/ESI -> ST0 (HARNESS-5); caller RegisterStencilShadows (objects)")]
pub fn signed_sq_dist_object(entity: GPtr<u8>, point: GPtr<u8>) -> X87 {
    signed_sq_dist_to_bound_sphere(entity, point)
}

/// `CStencilShadows` helper 0x7111F0 (`GetSignedSqDistToBoundSphere_Vehicle`), the same body.
#[hook(addr = 0x7111F0, conv = reg("args = ebx esi"), name = "FUN_007111f0",
       expect = "83 EC 14 57 8B CB",
       unverified = "harness5-1", route = "L2", hazards = "none", recs = "driving",
       note = "register convention EBX/ESI -> ST0 (HARNESS-5); caller RegisterStencilShadows (vehicles)")]
pub fn signed_sq_dist_vehicle(entity: GPtr<u8>, point: GPtr<u8>) -> X87 {
    signed_sq_dist_to_bound_sphere(entity, point)
}

/// `CStencilShadows` helper 0x70FA70 (`ToggleSilhouetteEdge`; ECX = `u16 (*edges)[2]`, EDX = `u16*
/// count`, DI = vertex a, `[esp+4]` = vertex b (word), caller pops): an edge (a,b) or (b,a) already
/// in the list is removed (the last pair moved into its slot when `count > 1`; `v0` from the entry
/// snapshot `n`, `v1` from the re-read count), else (a,b) is appended. ECX, EDX and EDI are returned
/// unchanged (the third call site 0x70FF22 reuses ECX).
#[hook(addr = 0x70FA70, conv = reg("args = ecx edx di stack; keep = ecx edx"), name = "FUN_0070fa70",
       expect = "53 0F B7 1A 55",
       unverified = "harness5-1", route = "L1-pure", hazards = "preserved-regs", recs = "grove-walk,driving",
       note = "register convention ECX/EDX/DI + stack word (HARNESS-5); callers RenderForVehicle/RenderForObject")]
pub fn toggle_edge(edges: GPtr<u8>, count: GPtr<u16>, a: u16, b: u16) {
    let n = u32::from(count.read());
    let e = |i: u32, half: u32| edges.byte_add((4 * i + 2 * half) as i32 as isize).cast::<u16>();
    let mut found = None;
    if n != 0 {
        let mut i = 0u32;
        loop {
            let v0 = e(i, 0).read();
            if (v0 == a && e(i, 1).read() == b) || (v0 == b && e(i, 1).read() == a) {
                found = Some(i);
                break;
            }
            i += 1;
            if (i as i32) >= i32::from(count.read()) {
                break;
            }
        }
    }
    match found {
        None => {
            e(n, 0).write(a);
            e(u32::from(count.read()), 1).write(b);
            count.write(count.read().wrapping_add(1));
        }
        Some(i) => {
            if count.read() > 1 {
                e(i, 0).write(e(n.wrapping_sub(1), 0).read());
                let c = u32::from(count.read());
                e(i, 1).write(e(c.wrapping_sub(1), 1).read());
            }
            count.write(count.read().wrapping_sub(1));
        }
    }
}

/// `ShadowRenderTriangleCB(CVector* normal (EAX), CVector* tri (EBX), _ProjectionParam* param (EDI))`
/// (0x709CF0), the collision-triangle callback of `CShadows::CastShadowEntityXYZ`: the triangle in
/// world space (`param->entityMatrix`, +0x58) unless it faces the projection (`at . n > 0`, `at` at
/// +0), in projector space (`param->invMatrix`, +0xC); rejected when all three vertices are behind
/// (z < 0), left (x < 0), right (x > 1), below (y < 0) or above (y > 1); else stored into the render
/// buffer: world position lifted by 0.06 in z, uv = projector x/y, colour = `shadowValue` (+0x4C),
/// faded by `1 - z*z` when `param->fade` (+0x50). Returns EBX.
#[hook(addr = 0x709CF0, conv = reg("args = eax ebx edi"), name = "CShadows::ShadowRenderTriangleCB",
       expect = "83 EC 5C 56 8B F0",
       unverified = "harness5-1", route = "L2", hazards = "heap-writes", recs = "grove-walk,driving",
       note = "register convention EAX/EBX/EDI -> EAX = EBX (HARNESS-5); writes the RenderBuffer vertex/index arrays")]
#[allow(clippy::needless_range_loop)] // vertex k of three parallel arrays, as the original
pub fn shadow_render_triangle_cb(normal: GPtr<u8>, tri: GPtr<u8>, param: GPtr<u8>) -> GPtr<u8> {
    let mut w = [[0f32; 3]; 3];
    rw_v3d_transform_points(GPtr::from_raw(w.as_mut_ptr()).cast(), tri, 3, param.byte_add(0x58));
    let at = v3(param);
    let n = v3(normal);
    let d = x(at[1]).fmul_m(n[1]).fadd(x(at[2]).fmul_m(n[2])).fadd(x(at[0]).fmul_m(n[0]));
    if !d.fcom(x(ZERO)).test_any(0x41) {
        return tri;
    }
    let mut s = [[0f32; 3]; 3];
    rw_v3d_transform_points(
        GPtr::from_raw(s.as_mut_ptr()).cast(),
        GPtr::from_raw(w.as_mut_ptr()).cast(),
        3,
        param.byte_add(0xC),
    );
    // "all three below c" (`test ah, 5; jp` keeps going when one is not) / "all three above c"
    // (`test ah, 0x41; jne`), in the original's order
    let below = |c: usize| (0..3).all(|v| !x(s[v][c]).fcom(x(ZERO)).test_parity(0x05));
    let above = |c: usize| (0..3).all(|v| !x(s[v][c]).fcom(x(k(ONE))).test_any(0x41));
    if below(2) || below(0) || above(0) || below(1) || above(1) {
        return tri;
    }
    let (mut idx, mut vtx) = (0u32, 0u32);
    render_buffer_start_storing(3, 3, GPtr::from_raw(&mut idx), GPtr::from_raw(&mut vtx));
    let vb = GPtr::<u8>::from_addr(vtx as usize);
    for v in 0..3 {
        let o = 0x24 * v as isize;
        // x of vertex 0 through fld/fstp (a signalling NaN is quieted), the other x and y copied as
        // dwords, z + 0.06
        if v == 0 {
            wr::<f32>(vb, o, x(w[v][0]).to_f32());
        } else {
            wr::<u32>(vb, o, w[v][0].to_bits());
        }
        wr::<u32>(vb, o + 4, w[v][1].to_bits());
        wr::<f32>(vb, o + 8, x(w[v][2]).fadd_m(k(SHADOW_Z_LIFT)).to_f32());
    }
    for v in 0..3 {
        let o = 0x24 * v as isize;
        wr::<u32>(vb, o + 0x1C, s[v][0].to_bits());
        wr::<u32>(vb, o + 0x20, s[v][1].to_bits());
    }
    let shadow = rd::<u8>(param, 0x4C);
    let grey = |c: u8| u32::from_le_bytes([c, c, c, c]);
    if rd::<u32>(param, 0x50) != 0 {
        for v in 0..3 {
            let z = s[v][2];
            let t = x(z).fmul_m(z).fsubr_m(k(ONE));
            let c = if t.fcom(x(ZERO)).test_parity(0x05) {
                ftol32(X87::from_i32(i32::from(shadow)).fmul(t)) as u8
            } else {
                0
            };
            wr::<u32>(vb, 0x24 * v as isize + 0x18, grey(c));
        }
    } else {
        let c = grey(shadow);
        for v in 0..3 {
            wr::<u32>(vb, 0x24 * v as isize + 0x18, c);
        }
    }
    let ib = GPtr::<u16>::from_addr(idx as usize);
    ib.write(0);
    ib.wrapping_index(1).write(1);
    ib.wrapping_index(2).write(2);
    render_buffer_stop_storing();
    tri
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_CALC_DISTANCE_SQR_TO_ENTITY,
    HOOK_SIGNED_SQ_DIST_OBJECT,
    HOOK_SIGNED_SQ_DIST_VEHICLE,
    HOOK_TOGGLE_EDGE,
    HOOK_SHADOW_RENDER_TRIANGLE_CB,
];
