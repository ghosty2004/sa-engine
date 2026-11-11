//! PORT-RENDER: `CSprite` (3D-positioned screen sprites and the shared indexed sprite buffer). Ported
//! from the 1.0 US bytes; gta-reversed `Sprite.cpp` and `docs/re/specs/RENDER/CSprite.deviations.md` as
//! reading aids. Vertex layout and colour packing as in [`crate::sprite2d`].

use sa_core::abi::HookReg;
use sa_core::common::DRAW_FOV;
use sa_core::mem::GPtr;
use sa_core::platform::{RS_MAXIMUM_HEIGHT, RS_MAXIMUM_WIDTH};
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_world::world::common::{CAM_ENTITY, CAM_VECTOR};

use crate::common::{Fpu, f32_of, g, k, rd, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// `CSprite::m_fRecipNearClipPlane`
    pub static RECIP_NEAR_CLIP_PLANE @ 0xC4B8D0: f32 = 0.0;
    /// `CSprite::m_f2DFarScreenZ`
    pub static FAR_SCREEN_Z_2D @ 0xC4B8D4: f32 = 0.0;
    /// `CSprite::m_f2DNearScreenZ`
    pub static NEAR_SCREEN_Z_2D @ 0xC4B8D8: f32 = 0.0;
    /// `nSpriteBufferIndex` (sprites in the shared buffer, 4 vertices / 6 indices each)
    pub static SPRITE_BUFFER_INDEX @ 0xC6A158: i32 = 0;
    /// `CSprite::m_bFlushSpriteBufferSwitchZTest` (tested as a dword)
    pub static FLUSH_SWITCH_Z_TEST @ 0xC6A15C: u32 = 0;
}

/// `CDraw::ms_fFarClipZ` / `ms_fNearClipZ` (owner: CDraw, not declared yet).
const FAR_CLIP_Z: usize = 0xC3EF9C;
const NEAR_CLIP_Z: usize = 0xC3EFA0;
/// `TheCamera.m_mViewMatrix` (CMatrix at TheCamera +0xA04).
const VIEW_MATRIX: usize = 0xB6FA2C;
/// `TheCamera.m_mCameraMatrix` forward x / y (TheCamera +0xDC / +0xE0).
const CAM_FORWARD_X: usize = 0xB6F104;
const CAM_FORWARD_Y: usize = 0xB6F108;
/// `RwEngineInstance`
const RW_ENGINE_INSTANCE: usize = 0xC97B24;
/// The shared buffers (`aTempBufferIndices`, `aRadiosityVertexBuffer`).
const TEMP_BUFFER_INDICES: usize = 0xC4B958;
const RADIOSITY_VERTEX_BUFFER: usize = 0xC5F958;

/// 1.0 (0x858624), 70.0 (0x858CE0), 3000.0 (0x859A94)
const ONE: u32 = 0x3F80_0000;
const SEVENTY: u32 = 0x428C_0000;
const HORIZON_DIST: u32 = 0x453B_8000;

const RS_ZTESTENABLE: u32 = 6;

orig! {
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, result by hidden pointer)
    pub fn matrix_mul_vec(out: GPtr<f32>, m: GPtr<u8>, v: GPtr<f32>) -> GPtr<f32> = cdecl 0x59C890 => unported;
    /// `RwIm2DRenderIndexedPrimitive(type, vertices, nv, indices, ni)` (0x734EA0)
    pub fn rw_im2d_render_indexed_primitive(ty: u32, verts: GPtr<u8>, nv: i32, idx: GPtr<u16>, ni: i32) -> i32 = cdecl 0x734EA0 => unported;
    /// `RwRenderStateSet(state, value)` through `RwEngineInstance` +0x20
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
}

/// The D3DCOLOR of a `CRGBA` (bytes r, g, b, a).
fn argb(c: GPtr<u8>) -> u32 {
    (u32::from(rd::<u8>(c, 3)) << 24)
        | (u32::from(rd::<u8>(c, 0)) << 16)
        | (u32::from(rd::<u8>(c, 1)) << 8)
        | u32::from(rd::<u8>(c, 2))
}

/// One RwIm2DVertex (dword fields in layout order).
#[allow(clippy::too_many_arguments)]
fn vertex(v: GPtr<u8>, px: u32, py: u32, pz: u32, rhw: u32, col: u32, u: u32, w: u32) {
    for (o, d) in [
        (0, px),
        (4, py),
        (8, pz),
        (0xC, rhw),
        (0x10, col),
        (0x14, u),
        (0x18, w),
    ] {
        wr::<u32>(v, o, d);
    }
}

/// The f32 bits a `fld dword` / `fst(p) dword` pair stores.
fn through_x87(bits: u32) -> u32 {
    f32_of(x(f32::from_bits(bits))).to_bits()
}

/// `z` / `rhw` of the 2D vertex setters: `m_f2DNearScreenZ` and `m_fRecipNearClipPlane` through `fld`.
fn near_z_and_rhw() -> (u32, u32) {
    (
        through_x87(NEAR_SCREEN_Z_2D.ptr().cast::<u32>().read()),
        through_x87(RECIP_NEAR_CLIP_PLANE.ptr().cast::<u32>().read()),
    )
}

/// `CSprite::CalcScreenCoors(const RwV3d& in, RwV3d* out, float* w, float* h, bool checkMaxVisible,
/// bool checkMinVisible)` (0x70CE30): `out = TheCamera.m_mViewMatrix * in`; behind the near plane
/// (`!(near + 1.0 < out.z)`) with `checkMinVisible`, or not before the far plane (`!(out.z < far)`) with
/// `checkMaxVisible`, returns false (out already written). Else `r = 1 / out.z`, `out.x *= W * r`,
/// `out.y *= H * r` (W / H = RsGlobal maximum width / height), `*w = W * r / FOV * 70`, `*h = H * r / FOV
/// * 70`, true.
#[hook(addr = 0x70CE30, conv = cdecl, name = "CSprite::CalcScreenCoors", expect = "83 EC 14 A1 A0 EF C3 00")]
pub fn calc_screen_coors(
    input: GPtr<f32>,
    out: GPtr<f32>,
    w: GPtr<f32>,
    h: GPtr<f32>,
    check_max_visible: u8,
    check_min_visible: u8,
) -> u8 {
    let near = g::<f32>(NEAR_CLIP_Z).read();
    let far = g::<f32>(FAR_CLIP_Z).read();
    let mut t = [0u32; 3];
    let r = matrix_mul_vec(
        GPtr::from_raw(t.as_mut_ptr().cast()),
        GPtr::from_addr(VIEW_MATRIX),
        input,
    );
    let mut f = Fpu::new();
    f.fld(near);
    f.fadd_m(k(ONE));
    let rr = r.cast::<u32>();
    let ou = out.cast::<u32>();
    ou.write(rr.read());
    ou.wrapping_index(1).write(rr.wrapping_index(1).read());
    ou.wrapping_index(2).write(rr.wrapping_index(2).read());
    f.fcomp_m(out.wrapping_index(2).read());
    // jnp: near + 1 < z (C0 alone) skips the min test
    if f.jp(0x05) && check_min_visible != 0 {
        return 0;
    }
    f.fld(out.wrapping_index(2).read());
    f.fcomp_m(far);
    if f.jp(0x05) && check_max_visible != 0 {
        return 0;
    }
    f.fld(k(ONE));
    f.fdiv_m(out.wrapping_index(2).read());
    f.fild(RS_MAXIMUM_WIDTH.get());
    f.fmul_st(1);
    f.fmul_m(out.read());
    out.write(f.fstp());
    f.fild(RS_MAXIMUM_HEIGHT.get());
    f.fmul_st(1);
    f.fmul_m(out.wrapping_index(1).read());
    out.wrapping_index(1).write(f.fstp());
    f.fild(RS_MAXIMUM_WIDTH.get());
    f.fmul_st(1);
    w.write(f.fstp());
    f.fild(RS_MAXIMUM_HEIGHT.get());
    f.fmul_st(1);
    h.write(f.fstp());
    f.fstp_st(0);
    let fov = DRAW_FOV.get();
    w.write(f32_of(x(w.read()).fdiv_m(fov).fmul_m(k(SEVENTY))));
    h.write(f32_of(x(h.read()).fdiv_m(fov).fmul_m(k(SEVENTY))));
    1
}

/// `CSprite::FlushSpriteBuffer()` (0x70CF20): when sprites are buffered, one indexed triangle-list draw
/// of `n * 4` vertices / `n * 6` indices from the shared buffers (z test off around it when
/// `m_bFlushSpriteBufferSwitchZTest`), then `n = 0`.
#[hook(addr = 0x70CF20, conv = cdecl, name = "CSprite::FlushSpriteBuffer", expect = "A1 58 A1 C6 00")]
pub fn flush_sprite_buffer() {
    let n = SPRITE_BUFFER_INDEX.get();
    if n <= 0 {
        return;
    }
    let draw = |n: i32| {
        rw_im2d_render_indexed_primitive(
            3,
            GPtr::from_addr(RADIOSITY_VERTEX_BUFFER),
            n.wrapping_mul(4),
            GPtr::from_addr(TEMP_BUFFER_INDICES),
            n.wrapping_mul(6),
        );
    };
    if FLUSH_SWITCH_Z_TEST.get() != 0 {
        rw_render_state_set(RS_ZTESTENABLE, 0);
        draw(SPRITE_BUFFER_INDEX.get());
        rw_render_state_set(RS_ZTESTENABLE, 1);
    } else {
        draw(n);
    }
    SPRITE_BUFFER_INDEX.set(0);
}

/// `CSprite::InitSpriteBuffer()` (0x70CFB0): `m_f2DNearScreenZ = RwEngineInstance->+0x18`,
/// `m_f2DFarScreenZ = +0x1C` (through `fld` / `fstp`). The entry jumps into the protection chunk
/// 0x40A196, which loads `RwEngineInstance` into EAX and jumps back to 0x70CFB5.
#[hook(addr = 0x70CFB0, conv = cdecl, name = "CSprite::InitSpriteBuffer", expect = "E9 E1 D1 CF FF D9 40 18")]
pub fn init_sprite_buffer() {
    let eng = g::<GPtr<u8>>(RW_ENGINE_INSTANCE).read();
    NEAR_SCREEN_Z_2D
        .ptr()
        .cast::<u32>()
        .write(through_x87(rd::<u32>(eng, 0x18)));
    FAR_SCREEN_Z_2D
        .ptr()
        .cast::<u32>()
        .write(through_x87(rd::<u32>(eng, 0x1C)));
}

/// `CSprite::Set4Vertices2D(RwIm2DVertex* v, const CRect& r, const CRGBA& c1, c2, c3, c4)` (0x70E1C0):
/// z = `m_f2DNearScreenZ`, rhw = `m_fRecipNearClipPlane`; v0 = (left, bottom, c3, 0, 0), v1 = (right,
/// bottom, c4, 1, 0), v2 = (right, top, c2, 1, 1), v3 = (left, top, c1, 0, 1).
#[hook(addr = 0x70E1C0, conv = cdecl, name = "CSprite::Set4Vertices2D-CRect", expect = "D9 05 D8 B8 C4 00")]
pub fn set_4_vertices_2d_rect(
    v: GPtr<u8>,
    r: GPtr<u8>,
    c1: GPtr<u8>,
    c2: GPtr<u8>,
    c3: GPtr<u8>,
    c4: GPtr<u8>,
) {
    let (z, w) = near_z_and_rhw();
    let rr = |o: isize| rd::<u32>(r, o);
    vertex(v, rr(0), rr(0xC), z, w, argb(c3), 0, 0);
    vertex(v.byte_add(0x1C), rr(8), rr(0xC), z, w, argb(c4), ONE, 0);
    vertex(v.byte_add(0x38), rr(8), rr(4), z, w, argb(c2), ONE, ONE);
    vertex(v.byte_add(0x54), rr(0), rr(4), z, w, argb(c1), 0, ONE);
}

/// `CSprite::Set4Vertices2D(RwIm2DVertex* v, x1, y1, x2, y2, x3, y3, x4, y4, const CRGBA& c1, c2, c3, c4)`
/// (0x70E2D0): v0 = (x3 through `fld` / `fstp`, y3, c3, 0, 0), v1 = (x4, y4, c4, 1, 0), v2 = (x2, y2, c2,
/// 1, 1), v3 = (x1, y1, c1, 0, 1); z / rhw as the CRect overload.
#[hook(addr = 0x70E2D0, conv = cdecl, name = "CSprite::Set4Vertices2D-1", expect = "D9 05 D8 B8 C4 00")]
#[allow(clippy::too_many_arguments)]
pub fn set_4_vertices_2d(
    v: GPtr<u8>,
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
    x3: u32,
    y3: u32,
    x4: u32,
    y4: u32,
    c1: GPtr<u8>,
    c2: GPtr<u8>,
    c3: GPtr<u8>,
    c4: GPtr<u8>,
) {
    let (z, w) = near_z_and_rhw();
    vertex(v, through_x87(x3), y3, z, w, argb(c3), 0, 0);
    vertex(v.byte_add(0x1C), x4, y4, z, w, argb(c4), ONE, 0);
    vertex(v.byte_add(0x38), x2, y2, z, w, argb(c2), ONE, ONE);
    vertex(v.byte_add(0x54), x1, y1, z, w, argb(c1), 0, ONE);
}

/// `CSprite::CalcHorizonCoors()` (0x70E3E0, result in ST0): the camera position `p` (attached entity's
/// matrix position or TheCamera +0x4), the point `(fwd.x * 3000 + p.x, fwd.y * 3000 + p.y, 0)` through the
/// view matrix, result `1 / v.z * H * v.y` (H = RsGlobal.maximumHeight, `fimul`).
#[hook(addr = 0x70E3E0, conv = cdecl, name = "CSprite::CalcHorizonCoors", expect = "A1 3C F0 B6 00")]
pub fn calc_horizon_coors() -> X87 {
    let ent = CAM_ENTITY.get();
    let p: GPtr<f32> = if ent.is_null() {
        CAM_VECTOR.ptr().cast()
    } else {
        ent.byte_add(0x30).cast()
    };
    let mut local = [0f32; 3];
    let px = p.cast::<u32>().read();
    let mut f = Fpu::new();
    f.fld(p.wrapping_index(1).read());
    f.fld(g::<f32>(CAM_FORWARD_X).read());
    f.fmul_m(k(HORIZON_DIST));
    f.fadd_m(f32::from_bits(px));
    local[0] = f.fstp();
    f.fld(g::<f32>(CAM_FORWARD_Y).read());
    f.fmul_m(k(HORIZON_DIST));
    f.fadd_st(1);
    local[1] = f.fstp();
    f.fstp_st(0);
    let mut t = [0u32; 3];
    let r = matrix_mul_vec(
        GPtr::from_raw(t.as_mut_ptr().cast()),
        GPtr::from_addr(VIEW_MATRIX),
        GPtr::from_raw(local.as_mut_ptr()),
    );
    let (vy, vz) = (r.wrapping_index(1).read(), r.wrapping_index(2).read());
    let mut f = Fpu::new();
    f.fld(k(ONE));
    f.fdiv_m(vz);
    let v = f.pop().fmul(xi(RS_MAXIMUM_HEIGHT.get()));
    v.fmul_m(vy)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_CALC_SCREEN_COORS,
    HOOK_FLUSH_SPRITE_BUFFER,
    HOOK_INIT_SPRITE_BUFFER,
    HOOK_SET_4_VERTICES_2D_RECT,
    HOOK_SET_4_VERTICES_2D,
    HOOK_CALC_HORIZON_COORS,
];
