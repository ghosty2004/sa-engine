//! `CCustomRoadsignMgr::CreateRoadsignAtomicA` (batch `mvpn-render-1`): builds a road-sign atomic of up to four text
//! lines. Ported from the 1.0 US bytes (gta-reversed `CustomRoadsignMgr.cpp` as a reading aid; the bytes win: the line
//! offsets are products of the stored line height, the used height multiplies the unrounded quotient). Unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, wr};

orig! {
    /// `RpMaterialCreate()` (0x74D990)
    fn rp_material_create() -> GPtr<u8> = cdecl 0x74D990 => unported;
    /// `RpMaterialDestroy(RpMaterial*)` (0x74DA20)
    fn rp_material_destroy(m: GPtr<u8>) -> u32 = cdecl 0x74DA20 => unported;
    /// `RpMaterialSetTexture(RpMaterial*, RwTexture*)` (0x74DBC0)
    fn rp_material_set_texture(m: GPtr<u8>, t: GPtr<u8>) -> GPtr<u8> = cdecl 0x74DBC0 => unported;
    /// `RwTextureDestroy(RwTexture*)` (0x7F3820)
    fn rw_texture_destroy(t: GPtr<u8>) -> u32 = cdecl 0x7F3820 => unported;
    /// `CCustomRoadsignMgr::CreateRoadsignTexture(const char*, int32)` (0x6FECA0)
    fn create_roadsign_texture(text: GPtr<u8>, n: i32) -> GPtr<u8> = cdecl 0x6FECA0 => crate::mvpc_render1::roadsign_create_texture;
    /// `RpGeometryCreate(numVerts, numTriangles, format)` (0x74CA90)
    fn rp_geometry_create(verts: i32, tris: i32, format: u32) -> GPtr<u8> = cdecl 0x74CA90 => unported;
    /// `RpGeometryDestroy(RpGeometry*)` (0x74CCC0)
    fn rp_geometry_destroy(geo: GPtr<u8>) -> u32 = cdecl 0x74CCC0 => unported;
    /// `RpGeometryTriangleSetVertexIndices(geo, tri, v0, v1, v2)` (0x74C690)
    fn rp_geometry_triangle_set_vertex_indices(geo: GPtr<u8>, tri: GPtr<u8>, a: i32, b: i32, c: i32) -> GPtr<u8> = cdecl 0x74C690 => unported;
    /// `RpGeometryTriangleSetMaterial(geo, tri, material)` (0x74C6C0)
    fn rp_geometry_triangle_set_material(geo: GPtr<u8>, tri: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x74C6C0 => unported;
    /// `RpMorphTargetCalcBoundingSphere(RpMorphTarget*, RwSphere*)` (0x74C200)
    fn rp_morph_target_calc_bounding_sphere(mt: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> = cdecl 0x74C200 => unported;
    /// `RpGeometryUnlock(RpGeometry*)` (0x74C800)
    fn rp_geometry_unlock(geo: GPtr<u8>) -> GPtr<u8> = cdecl 0x74C800 => unported;
    /// `RpAtomicCreate()` (0x749C50)
    fn rp_atomic_create() -> GPtr<u8> = cdecl 0x749C50 => unported;
    /// `RpAtomicSetGeometry(atomic, geo, flags)` (0x749D40)
    fn rp_atomic_set_geometry(a: GPtr<u8>, geo: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x749D40 => unported;
    /// `RpAtomicDestroy(RpAtomic*)` (0x749DC0)
    fn rp_atomic_destroy(a: GPtr<u8>) -> u32 = cdecl 0x749DC0 => unported;
    /// `RwFrameCreate()` (0x7F0410)
    fn rw_frame_create() -> GPtr<u8> = cdecl 0x7F0410 => unported;
    /// `RwFrameSetIdentity(RwFrame*)` (0x7F10B0)
    fn rw_frame_set_identity(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F10B0 => unported;
    /// `RpAtomicSetFrame(RpAtomic*, RwFrame*)` (0x74BF20)
    fn rp_atomic_set_frame(a: GPtr<u8>, f: GPtr<u8>) -> GPtr<u8> = cdecl 0x74BF20 => unported;
}

/// 0.95f (0x858EF0), 0.0f (0x858B50), -0.5f (0x858F40), -2.0f (0x858B18), 0.5f (0x858B8C), -1.0f (0x858C1C),
/// -1.5 (double, 0x8725A0)
const K_USED: usize = 0x858EF0;
const K_ZERO: usize = 0x858B50;
const K_NEG_HALF: usize = 0x858F40;
const K_NEG_TWO: usize = 0x858B18;
const K_HALF: usize = 0x858B8C;
const K_NEG_ONE: usize = 0x858C1C;
const K_NEG_ONE_HALF_D: usize = 0x8725A0;

fn kf(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// Destroys every non-null material of the four slots (the original's cleanup loops).
fn destroy_materials(mats: &[GPtr<u8>; 4]) {
    for m in mats {
        if !m.is_null() {
            rp_material_destroy(*m);
        }
    }
}

/// The y offset of line `i` of `n` lines: `lineH * K` in extended precision (`lineH` = the stored f32 quotient), 0.0
/// when the pair is not in the original's tables.
fn line_y(n: i32, i: i32, line_h: f32) -> X87 {
    let h = X87::from_f32(line_h);
    match (n, i) {
        (1, _) => h.fmul_m(kf(K_NEG_HALF)),
        (2, 0) | (4, 1) => h.fmul_m(kf(K_ZERO)),
        (2, 1) | (4, 2) => h.fmul_m(kf(K_NEG_ONE)),
        (3, 0) => h.fmul_m(kf(K_HALF)),
        (3, 1) => h.fmul_m(kf(K_NEG_HALF)),
        (3, 2) => h.fmul(X87::from_f64_exact(g::<f64>(K_NEG_ONE_HALF_D).read())),
        (4, 0) => h,
        (4, 3) => h.fmul_m(kf(K_NEG_TWO)),
        _ => X87::from_f32(kf(K_ZERO)),
    }
}

/// `CCustomRoadsignMgr::CreateRoadsignAtomicA(float width, float height, int32 numLines, char* line1..line4, int32
/// lettersPerLine, uint8 palette)` (0x6FEDA0, cdecl).
///
/// Per line: `RpMaterialCreate` (null → every stored material destroyed, 0), colour `{r, g, b, 255}` written to
/// material +4 (palette 1 black, 2 grey 0x80, 3 red, else white), `CreateRoadsignTexture(line, lettersPerLine)` (null →
/// the stored materials destroyed, the new one leaked, 0), `RpMaterialSetTexture` + `RwTextureDestroy`, surface
/// properties 0.3 / 0.3 / 0.7 (+0xC..+0x14). Then `RpGeometryCreate(4n, 2n, 0x4E)` (null → materials destroyed, 0).
/// `lineH = f32(height / n)`, `usedH = f32(height / n * 0.95)` (the unrounded quotient); with `negHalf = width * -0.5`,
/// `w0 = width * 0`, `u0 = usedH * 0` (all extended) and the line offset `y` (see `line_y`) each line's four vertices
/// are `(w0 + negHalf, u0 + y)`, `(negHalf + width, u0 + y)`, `(negHalf + width, usedH + y)`, `(w0 + negHalf, y +
/// usedH)`, z 0; texture coordinates (0,1) (1,1) (1,0) (0,0); prelit colours all 0xFF; triangles `(4i, 4i+1, 4i+2)`
/// and `(4i, 4i+2, 4i+3)` with the line's material, which is then destroyed. The bounding sphere is computed into
/// morph target +4, the geometry unlocked; `RpAtomicCreate` (null → geometry destroyed, 0), `RpAtomicSetGeometry(.., 0)`
/// (null → geometry and atomic destroyed, 0), geometry destroyed (the atomic keeps its reference), atomic flags byte
/// (+2) = 4 (render), a new identity frame set; returns the atomic.
///
/// Original behaviour kept: the failed-texture path leaks the line's material; the original's dead first vertex
/// writes (overwritten in the same iteration) are not repeated. More than four lines overrun the original's
/// four-slot arrays; the port panics instead (callers pass 1..=4).
#[hook(addr = 0x6feda0, conv = cdecl, name = "CCustomRoadsignMgr::CreateRoadsignAtomicA", expect = "83 EC 3C 8B 4C 24 50",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "creates materials, textures, a geometry, an atomic and a frame (RW heap); called when road-sign models are set up")]
#[allow(clippy::too_many_arguments)]
pub fn create_roadsign_atomic_a(width: f32, height: f32, n: i32, l1: GPtr<u8>, l2: GPtr<u8>, l3: GPtr<u8>,
                                l4: GPtr<u8>, letters: i32, palette: u32) -> GPtr<u8> {
    let lines = [l1, l2, l3, l4];
    let mut mats = [GPtr::<u8>::NULL; 4];
    for i in 0..n.max(0) as usize {
        let m = rp_material_create();
        if m.is_null() {
            destroy_materials(&mats);
            return GPtr::NULL;
        }
        let rgb: u32 = match palette as u8 {
            1 => 0,
            2 => 0x80_8080,
            3 => 0xFF,
            _ => 0xFF_FFFF,
        };
        wr::<u32>(m, 4, rgb | 0xFF00_0000);
        let tex = create_roadsign_texture(lines[i], letters);
        if tex.is_null() {
            destroy_materials(&mats);
            return GPtr::NULL;
        }
        rp_material_set_texture(m, tex);
        rw_texture_destroy(tex);
        wr::<f32>(m, 0xC, f32::from_bits(0x3E99999A));
        mats[i] = m;
        wr::<f32>(m, 0x10, f32::from_bits(0x3E99999A));
        wr::<f32>(m, 0x14, f32::from_bits(0x3F333333));
    }
    let geo = rp_geometry_create(n.wrapping_mul(4), n.wrapping_mul(2), 0x4E);
    if geo.is_null() {
        destroy_materials(&mats);
        return GPtr::NULL;
    }
    let mt = rd::<GPtr<u8>>(geo, 0x5C);
    let verts = rd::<GPtr<u8>>(mt, 0x14);
    let q = X87::from_i32(n).fdivr_m(height);
    let line_h = q.to_f32();
    let used_h = q.fmul_m(kf(K_USED)).to_f32();
    if n > 0 {
        let u0 = X87::from_f32(used_h).fmul_m(kf(K_ZERO));
        let w0 = X87::from_f32(width).fmul_m(kf(K_ZERO));
        let neg_half = X87::from_f32(width).fmul_m(kf(K_NEG_HALF));
        for i in 0..n {
            let v = verts.byte_add(i as isize * 0x30);
            let y = line_y(n, i, line_h);
            wr::<f32>(v, 0, w0.fadd(neg_half).to_f32());
            wr::<f32>(v, 4, u0.fadd(y).to_f32());
            wr::<f32>(v, 0xC, neg_half.fadd_m(width).to_f32());
            wr::<f32>(v, 0x10, u0.fadd(y).to_f32());
            wr::<f32>(v, 0x18, neg_half.fadd_m(width).to_f32());
            wr::<f32>(v, 0x1C, X87::from_f32(used_h).fadd(y).to_f32());
            wr::<f32>(v, 0x24, w0.fadd(neg_half).to_f32());
            wr::<f32>(v, 0x28, y.fadd_m(used_h).to_f32());
            for off in [8, 0x14, 0x20, 0x2C] {
                wr::<u32>(v, off, 0);
            }
        }
        let tc = rd::<GPtr<u8>>(geo, 0x34);
        for i in 0..n {
            let t = tc.byte_add(i as isize * 0x20);
            for (k, w) in [0u32, 0x3F80_0000, 0x3F80_0000, 0x3F80_0000, 0x3F80_0000, 0, 0, 0].iter().enumerate() {
                wr::<u32>(t, 4 * k as isize, *w);
            }
        }
        let pre = rd::<GPtr<u8>>(geo, 0x30);
        for i in 0..n {
            for k in 0..16 {
                wr::<u8>(pre, i as isize * 0x10 + k, 0xFF);
            }
        }
        let tris = rd::<GPtr<u8>>(geo, 0x2C);
        for i in 0..n {
            let t = tris.byte_add(i as isize * 0x10);
            let v = i * 4;
            rp_geometry_triangle_set_vertex_indices(geo, t, v, v + 1, v + 2);
            let m = mats[i as usize];
            rp_geometry_triangle_set_material(geo, t, m);
            rp_geometry_triangle_set_vertex_indices(geo, t.byte_add(8), v, v + 2, v + 3);
            rp_geometry_triangle_set_material(geo, t.byte_add(8), m);
            rp_material_destroy(m);
            mats[i as usize] = GPtr::NULL;
        }
    }
    let mut sphere = [0u32; 4];
    rp_morph_target_calc_bounding_sphere(mt, GPtr::from_raw(sphere.as_mut_ptr()).cast());
    for (k, w) in sphere.iter().enumerate() {
        wr::<u32>(mt, 4 + 4 * k as isize, *w);
    }
    rp_geometry_unlock(geo);
    let atomic = rp_atomic_create();
    if atomic.is_null() {
        destroy_materials(&mats);
        rp_geometry_destroy(geo);
        return GPtr::NULL;
    }
    if rp_atomic_set_geometry(atomic, geo, 0).is_null() {
        destroy_materials(&mats);
        rp_geometry_destroy(geo);
        rp_atomic_destroy(atomic);
        return GPtr::NULL;
    }
    rp_geometry_destroy(geo);
    wr::<u8>(atomic, 2, 4);
    let frame = rw_frame_create();
    rw_frame_set_identity(frame);
    rp_atomic_set_frame(atomic, frame);
    atomic
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[HOOK_CREATE_ROADSIGN_ATOMIC_A];
