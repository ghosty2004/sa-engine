//! PORT-RW-4 module `d3d9vsmat`: the D3D9 vertex-shader matrix helpers (`_rwD3D9VS*`, 0x764650..0x765020)
//! and the vertex-shader constant uploads (`_rpD3D9VertexShaderUpdate*` / `Set*Matrix`, 0x761720..0x761C25).
//! Ported from the 1.0 US bytes; `docs/re/specs/RW/RwD3D9VertexShader*.md` as reading aids.
//!
//! State (raw cells, shared with other modules):
//! - 0xC94C70 active world `RwMatrix*` (NULL = identity), 0x8D7390 its D3DMATRIX copy (rows 0..3, the w
//!   column keeps its image values 0, 0, 0, 1);
//! - 0xC94C74 cached inverse world (0x8D73D0 or the identity 0x8753D0), 0xC94C78 cached normal matrix
//!   (0x8D7410, 0x8D73D0 or the identity), 0xC94C7C cached orthonormalised world (0x8D7450);
//! - 0xC9BC80 view D3DMATRIX, 0x8E2458 projection, 0xC94C30 view*projection, 0xC97C64 its pointer
//!   (NULL = recompute).
//!
//! D3DX helpers linked into the exe are called by address (stdcall) and the D3D9 constant uploads through
//! the device vtable (`SetVertexShaderConstantF`, +0x178), with the original arguments and order.
//! Local matrices are 16-byte aligned (the SSE multiply kernel may be reached through `RwMatrixMultiply`).
//! Pads that the multiply kernel leaves unwritten in the originals' stack temporaries are 0 here; every
//! such pad is overwritten or dropped before it reaches memory outside the function.
//!
//! Not reproduced (stack only): `_rpD3D9VertexShaderUpdateMaterialColor` keeps its integer temporaries in
//! its `color` argument slot; `_rpD3D9VertexShaderUpdateFogData` stores the fog density over its `desc`
//! argument slot, and with fog mode 0 uploads its 4 uninitialised stack floats (here: zeros).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, p8, rd, rw_globals, wr, x};

/// `_RwD3DDevice`.
const D3D_DEVICE: usize = 0xC97C28;
/// Active world matrix (`RwMatrix*`).
const VS_WORLD: usize = 0xC94C70;
/// Cached inverse world matrix.
const VS_INV_CACHE: usize = 0xC94C74;
/// Cached normal matrix.
const VS_NORMAL_CACHE: usize = 0xC94C78;
/// Cached orthonormalised world matrix.
const VS_ORTHO_CACHE: usize = 0xC94C7C;
/// World matrix copy (D3DMATRIX).
const WORLD_D3D: u32 = 0x8D7390;
/// Inverse world storage.
const INV_D3D: u32 = 0x8D73D0;
/// Normal matrix storage.
const NORMAL_D3D: u32 = 0x8D7410;
/// Orthonormalised world storage.
const ORTHO_D3D: u32 = 0x8D7450;
/// Identity D3DMATRIX (.rdata).
const IDENTITY_D3D: u32 = 0x8753D0;
/// View D3DMATRIX.
const VIEW_D3D: u32 = 0xC9BC80;
/// Projection D3DMATRIX.
const PROJ_D3D: u32 = 0x8E2458;
/// View * projection storage.
const VIEWPROJ_D3D: u32 = 0xC94C30;
/// `_RwD3D9ActiveViewProjTransform`.
const ACTIVE_VIEWPROJ: usize = 0xC97C64;
/// Environment-map texture matrix (.rdata).
const ENV_TEX_MATRIX: u32 = 0x875250;
/// `rwMATRIXINTERNALIDENTITY`.
const MAT_IDENTITY: u32 = 0x20000;
/// 1.0f (0x858624).
const ONE: f32 = 1.0;
/// 1/255 (0x859A3C).
const INV_255_ADDR: usize = 0x859A3C;
/// Fog density scale (0x875294).
const FOG_DENSITY_SCALE_ADDR: usize = 0x875294;
/// 1/sqrt(3) (`0x3F13CD3A`), the probe vector of `_rwD3D9VSGetRadiusInLocalSpace`.
const INV_SQRT3_BITS: u32 = 0x3F13_CD3A;
const ONE_BITS: u32 = 0x3F80_0000;
const HALF_BITS: u32 = 0x3F00_0000;

orig! {
    /// `D3DXMatrixMultiply(out, a, b)` (0x7672E1)
    pub fn d3dx_matrix_multiply(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = stdcall 0x7672E1 => unported;
    /// `D3DXMatrixMultiplyTranspose(out, a, b)` (0x7676C7)
    pub fn d3dx_matrix_multiply_transpose(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = stdcall 0x7676C7 => unported;
    /// `D3DXMatrixTranspose(out, m)` (0x7675EE)
    pub fn d3dx_matrix_transpose(out: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = stdcall 0x7675EE => unported;
    /// `D3DXVec3TransformNormal(out, v, m)` (0x766AB7)
    pub fn d3dx_vec3_transform_normal(out: GPtr<u8>, v: GPtr<u8>, m: GPtr<u8>) -> u32 = stdcall 0x766AB7 => unported;
    /// `D3DXVec3TransformCoord(out, v, m)` (0x766A32)
    pub fn d3dx_vec3_transform_coord(out: GPtr<u8>, v: GPtr<u8>, m: GPtr<u8>) -> u32 = stdcall 0x766A32 => unported;
    /// `RwMatrixMultiply(dst, a, b)` (0x7F18B0)
    fn rw_matrix_multiply(dst: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18B0 => crate::matrix::rw_matrix_multiply;
    /// `RwMatrixInvert(dst, src)` (0x7F2070)
    fn rw_matrix_invert(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F2070 => crate::matrix::rw_matrix_invert;
    /// `RwMatrixOrthoNormalize(dst, src)` (0x7F1920)
    fn rw_matrix_ortho_normalize(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F1920 => unported;
    /// `RwFrameGetLTM(frame)` (0x7F0990)
    fn rw_frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// `_rwInvSqrt(x)` (0x7EDB90), result in ST0
    fn rw_inv_sqrt(v: f32) -> f64 = cdecl 0x7EDB90 => unported;
    /// `RwD3D9GetRenderState(state, &value)` (0x7FC320)
    fn rw_d3d9_get_render_state(state: u32, out: GPtr<u32>) = cdecl 0x7FC320 => unported;
    /// `IDirect3DDevice9::SetVertexShaderConstantF(reg, data, count)` (vtable +0x178)
    pub fn device_set_vs_constant_f(dev: GPtr<u8>, reg: u32, data: GPtr<u8>, count: u32) -> i32 = stdcall vslot 0x178 => unported;

    /// `_rwD3D9MatrixInvertOrthonormal` (0x764890)
    fn invert_orthonormal_call(dst: GPtr<u8>, src: GPtr<u8>) -> u32 = cdecl 0x764890 => rw_d3d9_matrix_invert_orthonormal;
    /// `_rwD3D9VSGetPointInLocalSpace` (0x764E70)
    fn get_point_in_local_space_call(v: GPtr<u8>, out: GPtr<u8>) -> u32 = cdecl 0x764E70 => vs_get_point_in_local_space;
    /// `_rwD3D9VSGetWorldNormalizedTransposeMatrix` (0x764C60)
    fn get_world_normalized_transpose_call(out: GPtr<u8>) -> u32 = cdecl 0x764C60 => vs_get_world_normalized_transpose_matrix;
    /// `_rwD3D9VSGetWorldNormalizedMultiplyTransposeMatrix` (0x764A70)
    fn get_world_normalized_multiply_transpose_call(out: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x764A70 => vs_get_world_normalized_multiply_transpose_matrix;
    /// `_rwD3D9VSGetWorldNormalizedViewMultiplyTransposeMatrix` (0x764B50)
    fn get_world_normalized_view_multiply_transpose_call(out: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x764B50 => vs_get_world_normalized_view_multiply_transpose_matrix;
    /// `_rwD3D9VSGetWorldMultiplyTransposeMatrix` (0x764960)
    fn get_world_multiply_transpose_call(out: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x764960 => vs_get_world_multiply_transpose_matrix;
}

/// A 16-byte aligned local D3DMATRIX / RwMatrix.
#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct Mat([u32; 16]);

impl Mat {
    fn new() -> Mat {
        Mat([0; 16])
    }
    fn p(&mut self) -> GPtr<u8> {
        GPtr::from_raw(self.0.as_mut_ptr()).cast()
    }
}

fn device() -> GPtr<u8> {
    p8(g::<u32>(D3D_DEVICE).read())
}

fn world() -> GPtr<u8> {
    p8(g::<u32>(VS_WORLD).read())
}

/// `world != NULL && !(world.flags & rwMATRIXINTERNALIDENTITY)`.
fn world_not_identity(w: GPtr<u8>) -> bool {
    !w.is_null() && rd::<u32>(w, 0xC) & MAT_IDENTITY != MAT_IDENTITY
}

/// `(flags & 3) == 3` (orthonormal), as `and reg, 3; cmp reg8, 3`.
fn is_orthonormal(w: GPtr<u8>) -> bool {
    rd::<u32>(w, 0xC) & 3 == 3
}

/// Dwords +0xC/+0x1C/+0x2C = 0 and +0x3C = 1.0f of a matrix at `m`.
fn fix_w_column(m: GPtr<u8>) {
    wr::<u32>(m, 0xC, 0);
    wr::<u32>(m, 0x1C, 0);
    wr::<u32>(m, 0x2C, 0);
    wr::<u32>(m, 0x3C, ONE_BITS);
}

/// Row 3 of a D3DMATRIX at `m` = (0, 0, 0, 1.0f).
fn fix_row3(m: GPtr<u8>) {
    wr::<u32>(m, 0x30, 0);
    wr::<u32>(m, 0x34, 0);
    wr::<u32>(m, 0x38, 0);
    wr::<u32>(m, 0x3C, ONE_BITS);
}

/// `RwMatrixInvert(0x8D73D0, world)`, w column fixed, [0xC94C74] = 0x8D73D0; returns 0x8D73D0.
fn inverse_general(w: GPtr<u8>) -> GPtr<u8> {
    rw_matrix_invert(p8(INV_D3D), w);
    fix_w_column(p8(INV_D3D));
    g::<u32>(VS_INV_CACHE).write(INV_D3D);
    p8(INV_D3D)
}

/// The orthonormalised world cache: computed into 0x8D7450 when [0xC94C7C] is NULL; returns the cache.
fn ortho_cache(w: GPtr<u8>) -> GPtr<u8> {
    let c = g::<u32>(VS_ORTHO_CACHE).read();
    if c != 0 {
        return p8(c);
    }
    rw_matrix_ortho_normalize(p8(ORTHO_D3D), w);
    fix_w_column(p8(ORTHO_D3D));
    g::<u32>(VS_ORTHO_CACHE).write(ORTHO_D3D);
    p8(ORTHO_D3D)
}

fn copy16(dst: GPtr<u8>, src: GPtr<u8>) {
    for k in 0..16 {
        wr::<u32>(dst, k * 4, rd::<u32>(src, k * 4));
    }
}

/// `_rwD3D9VSSetActiveWorldMatrix(m)` (0x764650): [0xC94C70] = m; m's right/up/at/pos copied into the rows of
/// 0x8D7390; the three caches cleared. EAX: m's dword +0x38, or 0.
#[hook(addr = 0x764650, conv = cdecl, name = "_rwD3D9VSSetActiveWorldMatrix", expect = "55 8B EC 8B 45 08")]
pub fn vs_set_active_world_matrix(m: GPtr<u8>) -> u32 {
    g::<u32>(VS_WORLD).write(m.addr() as u32);
    let mut eax = 0;
    if !m.is_null() {
        let d = p8(WORLD_D3D);
        for row in 0..4isize {
            for k in 0..3isize {
                let v = rd::<u32>(m, row * 0x10 + k * 4);
                wr::<u32>(d, row * 0x10 + k * 4, v);
                eax = v;
            }
        }
    }
    g::<u32>(VS_INV_CACHE).write(0);
    g::<u32>(VS_NORMAL_CACHE).write(0);
    g::<u32>(VS_ORTHO_CACHE).write(0);
    eax
}

/// `_rwD3D9VSGetComposedTransformMatrix(out)` (0x7646E0): view*projection recomputed into 0xC94C30 when
/// [0xC97C64] is NULL; out = transpose(world * viewProj) or transpose(viewProj).
#[hook(addr = 0x7646E0, conv = cdecl, name = "_rwD3D9VSGetComposedTransformMatrix", expect = "55 8B EC A1 64 7C C9 00")]
pub fn vs_get_composed_transform_matrix(out: GPtr<u8>) -> GPtr<u8> {
    let mut vp = g::<u32>(ACTIVE_VIEWPROJ).read();
    if vp == 0 {
        d3dx_matrix_multiply(p8(VIEWPROJ_D3D), p8(VIEW_D3D), p8(PROJ_D3D));
        vp = VIEWPROJ_D3D;
        g::<u32>(ACTIVE_VIEWPROJ).write(vp);
    }
    if g::<u32>(VS_WORLD).read() != 0 {
        d3dx_matrix_multiply_transpose(out, p8(WORLD_D3D), p8(vp))
    } else {
        d3dx_matrix_transpose(out, p8(vp))
    }
}

/// `_rwD3D9VSGetWorldViewTransposedMatrix(out)` (0x764730): transpose(world * view) or transpose(view).
#[hook(addr = 0x764730, conv = cdecl, name = "_rwD3D9VSGetWorldViewTransposedMatrix", expect = "55 8B EC A1 70 4C C9 00")]
pub fn vs_get_world_view_transposed_matrix(out: GPtr<u8>) -> GPtr<u8> {
    if g::<u32>(VS_WORLD).read() != 0 {
        d3dx_matrix_multiply_transpose(out, p8(WORLD_D3D), p8(VIEW_D3D))
    } else {
        d3dx_matrix_transpose(out, p8(VIEW_D3D))
    }
}

/// `_rwD3D9VSGetWorldViewMatrix(out)` (0x764760): `RwMatrixMultiply(out, world, view)` with the w column
/// fixed, or a copy of the view matrix (EAX 0).
#[hook(addr = 0x764760, conv = cdecl, name = "_rwD3D9VSGetWorldViewMatrix", expect = "A1 70 4C C9 00")]
pub fn vs_get_world_view_matrix(out: GPtr<u8>) -> u32 {
    let w = world();
    if !w.is_null() {
        let r = rw_matrix_multiply(out, w, p8(VIEW_D3D));
        fix_w_column(out);
        r.addr() as u32
    } else {
        copy16(out, p8(VIEW_D3D));
        0
    }
}

/// `_rwD3D9VSGetInverseWorldMatrix(out)` (0x7647B0): the cached inverse, or identity (both caches), or the
/// orthonormal inverse (both caches), or the general inverse with its w column fixed (inverse cache only);
/// 16 dwords copied to out. EAX is left undefined on the cached path (void).
#[hook(addr = 0x7647B0, conv = cdecl, name = "_rwD3D9VSGetInverseWorldMatrix", expect = "55 8B EC 56 8B 35 74 4C C9 00")]
pub fn vs_get_inverse_world_matrix(out: GPtr<u8>) {
    let c = g::<u32>(VS_INV_CACHE).read();
    let src = if c != 0 {
        p8(c)
    } else {
        let w = world();
        if !world_not_identity(w) {
            g::<u32>(VS_NORMAL_CACHE).write(IDENTITY_D3D);
            g::<u32>(VS_INV_CACHE).write(IDENTITY_D3D);
            p8(IDENTITY_D3D)
        } else if is_orthonormal(w) {
            invert_orthonormal_call(p8(INV_D3D), w);
            g::<u32>(VS_NORMAL_CACHE).write(INV_D3D);
            g::<u32>(VS_INV_CACHE).write(INV_D3D);
            p8(INV_D3D)
        } else {
            inverse_general(w)
        }
    };
    copy16(out, src);
}

/// `_rwD3D9MatrixInvertOrthonormal(dst, src)` (0x764890): transposed rotation, pos = -(R^T pos) per row
/// (x87, `(p.y*r.y + p.x*r.x) + p.z*r.z` grouping); dst pads untouched. EAX = src's dword +0x28.
/// Reads and writes interleave as in the original (dst == src behaves the same).
#[hook(addr = 0x764890, conv = cdecl, name = "_rwD3D9MatrixInvertOrthonormal", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn rw_d3d9_matrix_invert_orthonormal(dst: GPtr<u8>, src: GPtr<u8>) -> u32 {
    let f = |o: isize| rd::<f32>(src, o);
    // row 0
    let a = x(f(0x34)).fmul_m(f(0x4));
    let b = x(f(0x30)).fmul_m(f(0x0));
    let s = a.fadd(b);
    let c = x(f(0x38)).fmul_m(f(0x8));
    wr::<f32>(dst, 0x30, f32_of(s.fadd(c).fchs()));
    wr::<u32>(dst, 0x0, rd::<u32>(src, 0x0));
    wr::<u32>(dst, 0x10, rd::<u32>(src, 0x4));
    wr::<u32>(dst, 0x20, rd::<u32>(src, 0x8));
    // row 1
    let a = x(f(0x30)).fmul_m(f(0x10));
    let b = x(f(0x14)).fmul_m(f(0x34));
    let s = a.fadd(b);
    let c = x(f(0x38)).fmul_m(f(0x18));
    wr::<f32>(dst, 0x34, f32_of(s.fadd(c).fchs()));
    wr::<u32>(dst, 0x4, rd::<u32>(src, 0x10));
    wr::<u32>(dst, 0x14, rd::<u32>(src, 0x14));
    wr::<u32>(dst, 0x24, rd::<u32>(src, 0x18));
    // row 2
    let a = x(f(0x30)).fmul_m(f(0x20));
    let b = x(f(0x24)).fmul_m(f(0x34));
    let s = a.fadd(b);
    let c = x(f(0x38)).fmul_m(f(0x28));
    wr::<f32>(dst, 0x38, f32_of(s.fadd(c).fchs()));
    wr::<u32>(dst, 0x8, rd::<u32>(src, 0x20));
    wr::<u32>(dst, 0x18, rd::<u32>(src, 0x24));
    let e = rd::<u32>(src, 0x28);
    wr::<u32>(dst, 0x28, e);
    e
}

/// `_rwD3D9VSGetWorldMultiplyMatrix(out, m)` (0x764920): `RwMatrixMultiply(out, world, m)` (no identity
/// test), or a copy of m (EAX 0).
#[hook(addr = 0x764920, conv = cdecl, name = "_rwD3D9VSGetWorldMultiplyMatrix", expect = "A1 70 4C C9 00")]
pub fn vs_get_world_multiply_matrix(out: GPtr<u8>, m: GPtr<u8>) -> u32 {
    let w = world();
    if !w.is_null() {
        rw_matrix_multiply(out, w, m).addr() as u32
    } else {
        copy16(out, m);
        0
    }
}

/// `_rwD3D9VSGetWorldMultiplyTransposeMatrix(out, m)` (0x764960): transpose(world * m) (non-identity world)
/// or transpose(m); row 3 of out = (0, 0, 0, 1).
#[hook(addr = 0x764960, conv = cdecl, name = "_rwD3D9VSGetWorldMultiplyTransposeMatrix", expect = "A1 70 4C C9 00")]
pub fn vs_get_world_multiply_transpose_matrix(out: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> {
    let w = world();
    let mut t = Mat::new();
    let src = if world_not_identity(w) {
        rw_matrix_multiply(t.p(), w, m);
        t.p()
    } else {
        m
    };
    let r = d3dx_matrix_transpose(out, src);
    fix_row3(out);
    r
}

/// `_rwD3D9VSGetWorldViewMultiplyTransposeMatrix(out, m)` (0x7649D0): transpose((world * view) * m) or
/// transpose(view * m), with the product's w column set to (0, 0, 0, 1) before transposing.
#[hook(addr = 0x7649D0, conv = cdecl, name = "_rwD3D9VSGetWorldViewMultiplyTransposeMatrix", expect = "55 8B EC 83 E4 F0")]
pub fn vs_get_world_view_multiply_transpose_matrix(out: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> {
    let w = world();
    let (mut l0, mut l1) = (Mat::new(), Mat::new());
    if world_not_identity(w) {
        rw_matrix_multiply(l1.p(), w, p8(VIEW_D3D));
        rw_matrix_multiply(l0.p(), l1.p(), m);
    } else {
        rw_matrix_multiply(l0.p(), p8(VIEW_D3D), m);
    }
    let p = l0.p();
    wr::<u32>(p, 0x3C, ONE_BITS);
    wr::<u32>(p, 0x2C, 0);
    wr::<u32>(p, 0x1C, 0);
    wr::<u32>(p, 0xC, 0);
    d3dx_matrix_transpose(out, p)
}

/// `_rwD3D9VSGetWorldNormalizedMultiplyTransposeMatrix(out, m)` (0x764A70): transpose(W * m) with W the
/// orthonormal world or the cached orthonormalised world, or transpose(m); row 3 = (0, 0, 0, 1).
#[hook(addr = 0x764A70, conv = cdecl, name = "_rwD3D9VSGetWorldNormalizedMultiplyTransposeMatrix", expect = "55 8B EC 83 EC 40")]
pub fn vs_get_world_normalized_multiply_transpose_matrix(out: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> {
    let w = world();
    let mut t = Mat::new();
    let src = if world_not_identity(w) {
        let wm = if is_orthonormal(w) { w } else { ortho_cache(w) };
        rw_matrix_multiply(t.p(), wm, m);
        t.p()
    } else {
        m
    };
    let r = d3dx_matrix_transpose(out, src);
    fix_row3(out);
    r
}

/// `_rwD3D9VSGetWorldNormalizedViewMultiplyTransposeMatrix(out, m)` (0x764B50): transpose((W * view) * m)
/// (W as in 0x764A70) or transpose(view * m); product w column (0, 0, 0, 1) before transposing.
#[hook(addr = 0x764B50, conv = cdecl, name = "_rwD3D9VSGetWorldNormalizedViewMultiplyTransposeMatrix", expect = "55 8B EC 83 E4 F0")]
pub fn vs_get_world_normalized_view_multiply_transpose_matrix(out: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> {
    let w = world();
    let (mut l0, mut l1) = (Mat::new(), Mat::new());
    if world_not_identity(w) {
        let wm = if is_orthonormal(w) { w } else { ortho_cache(w) };
        rw_matrix_multiply(l1.p(), wm, p8(VIEW_D3D));
        rw_matrix_multiply(l0.p(), l1.p(), m);
    } else {
        rw_matrix_multiply(l0.p(), p8(VIEW_D3D), m);
    }
    let p = l0.p();
    wr::<u32>(p, 0x3C, ONE_BITS);
    wr::<u32>(p, 0x2C, 0);
    wr::<u32>(p, 0x1C, 0);
    wr::<u32>(p, 0xC, 0);
    d3dx_matrix_transpose(out, p)
}

/// `_rwD3D9VSGetWorldNormalizedTransposeMatrix(out)` (0x764C60): transpose(world copy 0x8D7390) for an
/// orthonormal world, transpose(orthonormalised cache) otherwise (no row-3 fix on either), or the identity
/// with its w column rewritten (EAX = out).
#[hook(addr = 0x764C60, conv = cdecl, name = "_rwD3D9VSGetWorldNormalizedTransposeMatrix", expect = "55 8B EC 8B 0D 70 4C C9 00")]
pub fn vs_get_world_normalized_transpose_matrix(out: GPtr<u8>) -> u32 {
    let w = world();
    if world_not_identity(w) {
        let src = if is_orthonormal(w) { p8(WORLD_D3D) } else { ortho_cache(w) };
        d3dx_matrix_transpose(out, src).addr() as u32
    } else {
        copy16(out, p8(IDENTITY_D3D));
        fix_w_column(out);
        out.addr() as u32
    }
}

/// `_rwD3D9VSGetProjectionTransposedMatrix(out)` (0x764D20): transpose(projection).
#[hook(addr = 0x764D20, conv = cdecl, name = "_rwD3D9VSGetProjectionTransposedMatrix", expect = "8B 44 24 04 68 58 24 8E 00")]
pub fn vs_get_projection_transposed_matrix(out: GPtr<u8>) -> GPtr<u8> {
    d3dx_matrix_transpose(out, p8(PROJ_D3D))
}

/// `_rwD3D9VSGetNormalInLocalSpace(v, out)` (0x764D30): the normal matrix (cached; identity for no or an
/// identity world; the orthonormal inverse for an orthonormal world; else orthonormalise(inverse) into
/// 0x8D7410 with its w column fixed); identity copies v (EAX = v.z dword), else
/// `D3DXVec3TransformNormal(out, v, M)`.
#[hook(addr = 0x764D30, conv = cdecl, name = "_rwD3D9VSGetNormalInLocalSpace", expect = "55 8B EC A1 78 4C C9 00")]
pub fn vs_get_normal_in_local_space(v: GPtr<u8>, out: GPtr<u8>) -> u32 {
    let mut n = g::<u32>(VS_NORMAL_CACHE).read();
    if n == 0 {
        let w = world();
        if !world_not_identity(w) {
            g::<u32>(VS_INV_CACHE).write(IDENTITY_D3D);
            g::<u32>(VS_NORMAL_CACHE).write(IDENTITY_D3D);
            return copy_v3(v, out);
        }
        let mut inv = g::<u32>(VS_INV_CACHE).read();
        if is_orthonormal(w) {
            if inv == 0 {
                invert_orthonormal_call(p8(INV_D3D), w);
                inv = INV_D3D;
                g::<u32>(VS_INV_CACHE).write(inv);
            }
            g::<u32>(VS_NORMAL_CACHE).write(inv);
            n = inv;
        } else {
            if inv == 0 {
                inv = inverse_general(w).addr() as u32;
            }
            rw_matrix_ortho_normalize(p8(NORMAL_D3D), p8(inv));
            let nm = p8(NORMAL_D3D);
            wr::<u32>(nm, 0xC, 0);
            g::<u32>(VS_NORMAL_CACHE).write(NORMAL_D3D);
            wr::<u32>(nm, 0x1C, 0);
            wr::<u32>(nm, 0x2C, 0);
            wr::<u32>(nm, 0x3C, ONE_BITS);
            return d3dx_vec3_transform_normal(out, v, nm);
        }
    }
    if n == IDENTITY_D3D {
        return copy_v3(v, out);
    }
    d3dx_vec3_transform_normal(out, v, p8(n))
}

/// `*out = *v` (3 dwords); EAX = v.z.
fn copy_v3(v: GPtr<u8>, out: GPtr<u8>) -> u32 {
    wr::<u32>(out, 0, rd::<u32>(v, 0));
    wr::<u32>(out, 4, rd::<u32>(v, 4));
    let z = rd::<u32>(v, 8);
    wr::<u32>(out, 8, z);
    z
}

/// `_rwD3D9VSGetPointInLocalSpace(v, out)` (0x764E70): the inverse world (cached; identity for no or an
/// identity world, only [0xC94C74] set; orthonormal or general inverse into 0x8D73D0); identity copies v,
/// else `D3DXVec3TransformCoord(out, v, M)`.
#[hook(addr = 0x764E70, conv = cdecl, name = "_rwD3D9VSGetPointInLocalSpace", expect = "55 8B EC A1 74 4C C9 00")]
pub fn vs_get_point_in_local_space(v: GPtr<u8>, out: GPtr<u8>) -> u32 {
    let inv = g::<u32>(VS_INV_CACHE).read();
    if inv != 0 {
        if inv == IDENTITY_D3D {
            return copy_v3(v, out);
        }
        return d3dx_vec3_transform_coord(out, v, p8(inv));
    }
    let w = world();
    if !world_not_identity(w) {
        g::<u32>(VS_INV_CACHE).write(IDENTITY_D3D);
        return copy_v3(v, out);
    }
    let m = if is_orthonormal(w) {
        invert_orthonormal_call(p8(INV_D3D), w);
        g::<u32>(VS_INV_CACHE).write(INV_D3D);
        p8(INV_D3D)
    } else {
        inverse_general(w)
    };
    d3dx_vec3_transform_coord(out, v, m)
}

/// `_rwD3D9VSGetRadiusInLocalSpace(radius, out)` (0x764F60): for a non-orthonormal world (no identity
/// test): v = (1/sqrt 3) x3 through the inverse world (cached or computed), `*out = _rwInvSqrt(|v|^2) *
/// radius` (x87, `(y*y + x*x) + z*z` stored as f32); else `*out = radius` (fld/fstp). EAX undefined (void).
#[hook(addr = 0x764F60, conv = cdecl, name = "_rwD3D9VSGetRadiusInLocalSpace", expect = "55 8B EC 83 EC 10")]
pub fn vs_get_radius_in_local_space(radius: f32, out: GPtr<u8>) {
    let w = world();
    if w.is_null() || is_orthonormal(w) {
        wr::<f32>(out, 0, f32_of(x(radius)));
        return;
    }
    let mut inv = g::<u32>(VS_INV_CACHE).read();
    if inv == 0 {
        inv = inverse_general(w).addr() as u32;
    }
    let mut v = [INV_SQRT3_BITS; 3];
    let vp: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr()).cast();
    d3dx_vec3_transform_normal(vp, vp, p8(inv));
    let (vx, vy, vz) = (rd::<f32>(vp, 0), rd::<f32>(vp, 4), rd::<f32>(vp, 8));
    let s = x(vy).fmul_m(vy).fadd(x(vx).fmul_m(vx));
    let len2 = f32_of(s.fadd(x(vz).fmul_m(vz)));
    let r = X87::from_f64_exact(rw_inv_sqrt(len2)).fmul_m(radius);
    wr::<f32>(out, 0, f32_of(r));
}

/// `_rpD3D9VertexShaderUpdateLightsColors(p, desc, ambientCoef, diffuseCoef)` (0x761720): scales the
/// ambient rgb by `ambientCoef` and every light colour rgb by `diffuseCoef` in the constant block at `p`
/// (directional: direction + colour; point: a radius slot before lights 0, 4, 8, ..., then position +
/// colour; spot: radius slot grouping, position, direction, cone, colour), then uploads it to register 4.
#[hook(addr = 0x761720, conv = cdecl, name = "_rpD3D9VertexShaderUpdateLightsColors", expect = "8B 44 24 04 56")]
pub fn vs_update_lights_colors(p: GPtr<u8>, desc: GPtr<u8>, ambient: f32, diffuse: f32) -> i32 {
    let scale3 = |at: GPtr<u8>, k: f32| {
        for o in [0isize, 4, 8] {
            wr::<f32>(at, o, f32_of(x(k).fmul_m(rd::<f32>(at, o))));
        }
    };
    scale3(p, ambient);
    let mut e = p.byte_add(0x10);
    let dirs = rd::<u8>(desc, 0) & 0xF;
    for _ in 0..dirs {
        scale3(e.byte_add(0x10), diffuse);
        e = e.byte_add(0x20);
    }
    let points = (rd::<u8>(desc, 0) >> 4) as u32;
    let mut i = 0u32;
    while i < points {
        if i & 3 == 0 {
            e = e.byte_add(0x10);
        }
        scale3(e.byte_add(0x10), diffuse);
        e = e.byte_add(0x20);
        i += 1;
    }
    let spots = (rd::<u8>(desc, 1) & 0xF) as u32;
    let mut i = 0u32;
    while i < spots {
        if i & 3 == 0 {
            e = e.byte_add(0x10);
        }
        scale3(e.byte_add(0x30), diffuse);
        e = e.byte_add(0x40);
        i += 1;
    }
    let n = ((e.addr() as u32).wrapping_sub(p.addr() as u32)) >> 4;
    device_set_vs_constant_f(device(), 4, p, n)
}

/// `_rpD3D9VertexShaderUpdateMaterialColor(color, dispatch)` (0x761820): the RwRGBA as 4 floats
/// (`fild` * 1/255) uploaded to register `dispatch[0]`.
#[hook(addr = 0x761820, conv = cdecl, name = "_rpD3D9VertexShaderUpdateMaterialColor", expect = "83 EC 10 8B 44 24 14")]
pub fn vs_update_material_color(color: GPtr<u8>, dispatch: GPtr<u8>) -> i32 {
    let k = g::<f32>(INV_255_ADDR).read();
    let mut d = [0f32; 4];
    for (i, v) in d.iter_mut().enumerate() {
        *v = f32_of(X87::from_i32(rd::<u8>(color, i as isize) as i32).fmul_m(k));
    }
    let reg = rd::<u8>(dispatch, 0) as u32;
    device_set_vs_constant_f(device(), reg, GPtr::from_raw(d.as_mut_ptr()).cast(), 1)
}

/// `_rpD3D9VertexShaderUpdateFogData(desc, dispatch)` (0x7618B0): fog mode 1 (linear): `k = 1 / (fogPlane -
/// farPlane)` of the current camera (+0x84 / +0x88), data (k, -(k * far), 0, 1); modes 2/3: (density *
/// [0x875294], 0, 0, 1) with the density from `RwD3D9GetRenderState(D3DRS_FOGDENSITY)`; mode 0 uploads the
/// uninitialised stack data (zeros here). Register `dispatch[1]`.
#[hook(addr = 0x7618B0, conv = cdecl, name = "_rpD3D9VertexShaderUpdateFogData", expect = "8B 44 24 04 83 EC 10")]
pub fn vs_update_fog_data(desc: GPtr<u8>, dispatch: GPtr<u8>) -> i32 {
    let mut d = [0u32; 4];
    let mode = rd::<u8>(desc, 3) & 3;
    if mode == 1 {
        let cam = p8(rd::<u32>(rw_globals(), 0));
        let far = x(rd::<f32>(cam, 0x88));
        let k = x(rd::<f32>(cam, 0x84)).fsub(far).fdivr_m(ONE);
        d[0] = f32_of(k).to_bits();
        d[1] = f32_of(k.fmul(far).fchs()).to_bits();
        d[2] = 0;
        d[3] = ONE_BITS;
    } else if mode != 0 {
        let mut density = 0u32;
        rw_d3d9_get_render_state(0x26, GPtr::from_raw(&mut density));
        let scale = g::<f32>(FOG_DENSITY_SCALE_ADDR).read();
        d[0] = f32_of(x(f32::from_bits(density)).fmul_m(scale)).to_bits();
        d[1] = 0;
        d[2] = 0;
        d[3] = ONE_BITS;
    }
    let reg = rd::<u8>(dispatch, 1) as u32;
    device_set_vs_constant_f(device(), reg, GPtr::from_raw(d.as_mut_ptr()).cast(), 1)
}

/// `_rpD3D9VertexShaderUpdateMorphingCoef(coef, dispatch)` (0x761950): (coef, 1 - coef, 1, 0) to register
/// `dispatch[3]`.
#[hook(addr = 0x761950, conv = cdecl, name = "_rpD3D9VertexShaderUpdateMorphingCoef", expect = "83 EC 10 D9 05 24 86 85 00")]
pub fn vs_update_morphing_coef(coef: u32, dispatch: GPtr<u8>) -> i32 {
    let mut d = [coef, 0, ONE_BITS, 0];
    d[1] = f32_of(x(ONE).fsub_m(f32::from_bits(coef))).to_bits();
    let reg = rd::<u8>(dispatch, 3) as u32;
    device_set_vs_constant_f(device(), reg, GPtr::from_raw(d.as_mut_ptr()).cast(), 1)
}

/// Locals of `_rpD3D9VertexShaderSetEnvMatrix` in the original layout: the point at +4 and the matrix at
/// +0x10, so the 1-vec4 upload of the point carries `m[0]` as its w.
#[repr(C, align(16))]
struct EnvLocals {
    _pad: u32,
    v: [u32; 3],
    m: [u32; 16],
}

/// `_rpD3D9VertexShaderSetEnvMatrix(frame, desc, dispatch)` (0x761A10). Effect (`desc[3] >> 2`) other than
/// 2/3: camera position in local space + the normalised world (transposed) to `dispatch[2]` and
/// `dispatch[2]+1` (effect 10) or `+2` / `+3`; effects 2/3: transpose(W * inverse(frame LTM) * envTex)
/// (or the view variant without a frame), rows 0..1 with w = 0.5, to `dispatch[2]` (effect 2) or `+2`.
#[hook(addr = 0x761A10, conv = cdecl, name = "_rpD3D9VertexShaderSetEnvMatrix", expect = "55 8B EC 83 E4 F0")]
pub fn vs_set_env_matrix(frame: GPtr<u8>, desc: GPtr<u8>, dispatch: GPtr<u8>) -> i32 {
    let effect = (rd::<u8>(desc, 3) >> 2) as u32;
    let mut l = EnvLocals {
        _pad: 0,
        v: [0; 3],
        m: [0; 16],
    };
    let lp: GPtr<u8> = GPtr::from_raw(&mut l).cast();
    let (vp, mp) = (lp.byte_add(4), lp.byte_add(0x10));
    if effect != 2 && effect != 3 {
        let cam = p8(rd::<u32>(rw_globals(), 0));
        let ltm = rw_frame_get_ltm(p8(rd::<u32>(cam, 4)));
        get_point_in_local_space_call(ltm.byte_add(0x30), vp);
        get_world_normalized_transpose_call(mp);
        let base = rd::<u8>(dispatch, 2) as u32;
        let (r0, r1) = if effect == 10 { (base, base + 1) } else { (base + 2, base + 3) };
        device_set_vs_constant_f(device(), r0, vp, 1);
        return device_set_vs_constant_f(device(), r1, mp, 3);
    }
    if !frame.is_null() {
        let ltm = rw_frame_get_ltm(frame);
        let (mut inv, mut prod) = (Mat::new(), Mat::new());
        rw_matrix_invert(inv.p(), ltm);
        rw_matrix_multiply(prod.p(), inv.p(), p8(ENV_TEX_MATRIX));
        get_world_normalized_multiply_transpose_call(mp, prod.p());
    } else {
        get_world_normalized_view_multiply_transpose_call(mp, p8(ENV_TEX_MATRIX));
    }
    wr::<u32>(mp, 0xC, HALF_BITS);
    wr::<u32>(mp, 0x1C, HALF_BITS);
    let base = rd::<u8>(dispatch, 2) as u32;
    let reg = if effect == 2 { base } else { base + 2 };
    device_set_vs_constant_f(device(), reg, mp, 2)
}

/// `_rpD3D9VertexShaderSetBumpMatrix(frame, coef, dispatch)` (0x761B70): transpose(W * inverse(LTM of the
/// frame, or of the camera's frame)), rows 0..1 scaled by coef (x87 `m * coef`) with w = 0, to
/// `dispatch[2]`.
#[hook(addr = 0x761B70, conv = cdecl, name = "_rpD3D9VertexShaderSetBumpMatrix", expect = "55 8B EC 83 E4 F0")]
pub fn vs_set_bump_matrix(frame: GPtr<u8>, coef: f32, dispatch: GPtr<u8>) -> i32 {
    let f = if frame.is_null() {
        p8(rd::<u32>(p8(rd::<u32>(rw_globals(), 0)), 4))
    } else {
        frame
    };
    let ltm = rw_frame_get_ltm(f);
    let (mut inv, mut m) = (Mat::new(), Mat::new());
    rw_matrix_invert(inv.p(), ltm);
    get_world_multiply_transpose_call(m.p(), inv.p());
    let mp = m.p();
    wr::<u32>(mp, 0xC, 0);
    wr::<u32>(mp, 0x1C, 0);
    for o in [0x0isize, 0x4, 0x8, 0x10, 0x14, 0x18] {
        wr::<f32>(mp, o, f32_of(x(rd::<f32>(mp, o)).fmul_m(coef)));
    }
    let reg = rd::<u8>(dispatch, 2) as u32;
    device_set_vs_constant_f(device(), reg, mp, 2)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_VS_SET_ACTIVE_WORLD_MATRIX,
    HOOK_VS_GET_COMPOSED_TRANSFORM_MATRIX,
    HOOK_VS_GET_WORLD_VIEW_TRANSPOSED_MATRIX,
    HOOK_VS_GET_WORLD_VIEW_MATRIX,
    HOOK_VS_GET_INVERSE_WORLD_MATRIX,
    HOOK_RW_D3D9_MATRIX_INVERT_ORTHONORMAL,
    HOOK_VS_GET_WORLD_MULTIPLY_MATRIX,
    HOOK_VS_GET_WORLD_MULTIPLY_TRANSPOSE_MATRIX,
    HOOK_VS_GET_WORLD_VIEW_MULTIPLY_TRANSPOSE_MATRIX,
    HOOK_VS_GET_WORLD_NORMALIZED_MULTIPLY_TRANSPOSE_MATRIX,
    HOOK_VS_GET_WORLD_NORMALIZED_VIEW_MULTIPLY_TRANSPOSE_MATRIX,
    HOOK_VS_GET_WORLD_NORMALIZED_TRANSPOSE_MATRIX,
    HOOK_VS_GET_PROJECTION_TRANSPOSED_MATRIX,
    HOOK_VS_GET_NORMAL_IN_LOCAL_SPACE,
    HOOK_VS_GET_POINT_IN_LOCAL_SPACE,
    HOOK_VS_GET_RADIUS_IN_LOCAL_SPACE,
    HOOK_VS_UPDATE_LIGHTS_COLORS,
    HOOK_VS_UPDATE_MATERIAL_COLOR,
    HOOK_VS_UPDATE_FOG_DATA,
    HOOK_VS_UPDATE_MORPHING_COEF,
    HOOK_VS_SET_ENV_MATRIX,
    HOOK_VS_SET_BUMP_MATRIX,
];

/// Globals owned by this module (the VS caches are shared cells, accessed raw).
pub const GLOBALS: &[GlobalDesc] = &[];
