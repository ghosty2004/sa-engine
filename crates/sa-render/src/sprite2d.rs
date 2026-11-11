//! PORT-RENDER: `CSprite2d` (a 4-byte object holding an `RwTexture*`; the static quad `maVertices` and the
//! radiosity vertex buffer). Ported from the 1.0 US bytes; gta-reversed `Sprite2d.cpp` and
//! `docs/re/specs/RENDER/CSprite2d.md` as reading aids.
//!
//! `RwIm2DVertex` (0x1C bytes): +0 x, +4 y, +8 z, +0xC rhw, +0x10 colour (D3DCOLOR, `a << 24 | r << 16 |
//! g << 8 | b`), +0x14 u, +0x18 v. `CRect`: +0 left, +4 top, +8 right, +0xC bottom. Coordinates the
//! original copies with `mov` are copied as dwords here; values it moves through `fld` / `fstp` go through
//! the x87 model (a signalling NaN is quieted there, as on the CPU).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::platform::SCENE_CAMERA;
use sa_core::{hook, orig};

use crate::common::{f32_of, g, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// `CSprite2d::nextBufferIndex`
    pub static NEXT_BUFFER_INDEX @ 0xC80458: i32 = 0;
    /// `CSprite2d::nextBufferVertex`
    pub static NEXT_BUFFER_VERTEX @ 0xC8045C: i32 = 0;
    /// `CSprite2d::NearScreenZ`
    pub static NEAR_SCREEN_Z @ 0xC80460: f32 = 0.0;
    /// `CSprite2d::RecipNearClip`
    pub static RECIP_NEAR_CLIP @ 0xC80464: f32 = 0.0;
    /// `CSprite2d::maVertices[4]` (RwIm2DVertex)
    pub static MA_VERTICES @ 0xC80468: [u8; 4 * 0x1C] = [0; 4 * 0x1C];
}

/// `&maVertices[0]`.
const VERTICES: usize = 0xC80468;
/// `aTempBufferIndices` (u16) and `aRadiosityVertexBuffer` (RwIm2DVertex), the `RenderVertexBuffer`
/// arguments.
const TEMP_BUFFER_INDICES: u32 = 0xC4B958;
const RADIOSITY_VERTEX_BUFFER: u32 = 0xC5F958;
/// `RwEngineInstance`
const RW_ENGINE_INSTANCE: usize = 0xC97B24;

/// u / v constants of the CRect overload: 1/1024 and 1 + 1/1024.
const UV_LO: u32 = 0x3A80_0000;
const UV_HI: u32 = 0x3F80_2000;
const ONE: u32 = 0x3F80_0000;
/// 1e-4 (0x858FC4), the z bias of `SetVertices(int, float*, float*, const CRGBA&)`.
const Z_BIAS: u32 = 0x38D1_B717;

/// RenderWare render states the module sets.
const RS_TEXTURERASTER: u32 = 1;
const RS_SHADEMODE: u32 = 9;
const RS_SRCBLEND: u32 = 0xA;
const RS_DESTBLEND: u32 = 0xB;
const RS_VERTEXALPHAENABLE: u32 = 0xC;

orig! {
    /// `RwTextureDestroy(RwTexture*)` (0x7F3820)
    pub fn rw_texture_destroy(tex: GPtr<u8>) -> i32 = cdecl 0x7F3820 => unported;
    /// `RwTextureRead(const char* name, const char* mask)` (0x7F3AC0)
    pub fn rw_texture_read(name: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3AC0 => unported;
    /// `RwIm2DRenderPrimitive(type, vertices, n)` (0x734E90)
    pub fn rw_im2d_render_primitive(ty: u32, verts: GPtr<u8>, n: i32) -> i32 = cdecl 0x734E90 => unported;
    /// `RwIm2DRenderIndexedPrimitive(type, vertices, nv, indices, ni)` (0x734EA0)
    pub fn rw_im2d_render_indexed_primitive(ty: u32, verts: GPtr<u8>, nv: i32, idx: GPtr<u16>, ni: i32) -> i32 = cdecl 0x734EA0 => unported;
    /// `RwRenderStateSet(state, value)` through `RwEngineInstance` +0x20
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CSprite2d::SetVertices(const CRect&, const CRGBA&, x4)` (0x727420)
    pub fn set_vertices_rect_call(r: GPtr<u8>, c1: GPtr<u8>, c2: GPtr<u8>, c3: GPtr<u8>, c4: GPtr<u8>) = cdecl 0x727420 => set_vertices_rect;
    /// `CSprite2d::SetVertices(x1, y1, .. y4, const CRGBA&, x4)` (0x727590)
    pub fn set_vertices_quad_call(x1: u32, y1: u32, x2: u32, y2: u32, x3: u32, y3: u32, x4: u32, y4: u32, c1: GPtr<u8>, c2: GPtr<u8>, c3: GPtr<u8>, c4: GPtr<u8>) = cdecl 0x727590 => set_vertices_quad;
}

/// The D3DCOLOR of a `CRGBA` (bytes r, g, b, a): `a << 24 | r << 16 | g << 8 | b`.
fn argb(c: GPtr<u8>) -> u32 {
    (u32::from(rd::<u8>(c, 3)) << 24)
        | (u32::from(rd::<u8>(c, 0)) << 16)
        | (u32::from(rd::<u8>(c, 1)) << 8)
        | u32::from(rd::<u8>(c, 2))
}

/// One RwIm2DVertex at `v`: dword fields in layout order.
#[allow(clippy::too_many_arguments)]
fn vertex(v: GPtr<u8>, px: u32, py: u32, pz: u32, rhw: u32, col: u32, u: u32, w: u32) {
    wr::<u32>(v, 0, px);
    wr::<u32>(v, 4, py);
    wr::<u32>(v, 8, pz);
    wr::<u32>(v, 0xC, rhw);
    wr::<u32>(v, 0x10, col);
    wr::<u32>(v, 0x14, u);
    wr::<u32>(v, 0x18, w);
}

/// `RecipNearClip` / `NearScreenZ` as the dwords a `mov` copies.
fn recip_bits() -> u32 {
    RECIP_NEAR_CLIP.ptr().cast::<u32>().read()
}

fn near_z_bits() -> u32 {
    NEAR_SCREEN_Z.ptr().cast::<u32>().read()
}

/// The f32 bits a `fld dword` / `fstp dword` pair stores.
fn through_x87(bits: u32) -> u32 {
    f32_of(x(f32::from_bits(bits))).to_bits()
}

fn vtx(i: isize) -> GPtr<u8> {
    GPtr::from_addr(VERTICES).byte_add(i * 0x1C)
}

/// `CSprite2d::CSprite2d()` (0x727230): texture = null, returns `this`.
#[hook(addr = 0x727230, conv = thiscall, name = "CSprite2d::Constructor", expect = "8B C1 C7 00 00 00 00 00", this = "CSprite2d")]
pub fn constructor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0);
    this
}

/// `CSprite2d::Delete()` (0x727240): destroys the texture when set and clears it (the clear is in the
/// protection chunk 0x404875, which jumps back to the epilogue).
#[hook(addr = 0x727240, conv = thiscall, name = "CSprite2d::Delete", expect = "56 8B F1 8B 06", this = "CSprite2d")]
pub fn delete(this: GPtr<u8>) {
    let tex = rd::<GPtr<u8>>(this, 0);
    if !tex.is_null() {
        rw_texture_destroy(tex);
        wr::<u32>(this, 0, 0);
    }
}

/// `CSprite2d::SetTexture(const char* name)` (0x727270): `Delete()` inlined, then
/// `texture = RwTextureRead(name, NULL)` when `name` is set.
#[hook(addr = 0x727270, conv = thiscall, name = "CSprite2d::SetTexture-", expect = "56 8B F1 8B 06", this = "CSprite2d")]
pub fn set_texture(this: GPtr<u8>, name: GPtr<u8>) {
    let tex = rd::<GPtr<u8>>(this, 0);
    if !tex.is_null() {
        rw_texture_destroy(tex);
        wr::<u32>(this, 0, 0);
    }
    if !name.is_null() {
        let t = rw_texture_read(name, GPtr::NULL);
        wr::<GPtr<u8>>(this, 0, t);
    }
}

/// `CSprite2d::SetTexture(const char* name, const char* mask)` (0x7272B0): when both are set,
/// `texture = RwTextureRead(name, mask)` (the old texture is not destroyed).
#[hook(addr = 0x7272B0, conv = thiscall, name = "CSprite2d::SetTexture-mask", expect = "56 8B F1 8B 4C 24 08", this = "CSprite2d")]
pub fn set_texture_mask(this: GPtr<u8>, name: GPtr<u8>, mask: GPtr<u8>) {
    if !name.is_null() && !mask.is_null() {
        let t = rw_texture_read(name, mask);
        wr::<GPtr<u8>>(this, 0, t);
    }
}

/// `CSprite2d::SetAddressingUV(RwTextureAddressMode u, v)` (0x7272E0): the texture's filterAddressing
/// (+0x50) bits 8..11 = u, 12..15 = v; returns the texture (EAX), 0 without one.
#[hook(addr = 0x7272E0, conv = thiscall, name = "CSprite2d::SetAddressingUV", expect = "8B 01 85 C0 74 35", this = "CSprite2d")]
pub fn set_addressing_uv(this: GPtr<u8>, u: u32, v: u32) -> GPtr<u8> {
    let tex = rd::<GPtr<u8>>(this, 0);
    if tex.is_null() {
        return GPtr::NULL;
    }
    let f = rd::<u32>(tex, 0x50);
    wr::<u32>(tex, 0x50, f ^ ((u << 8 ^ f) & 0xF00));
    let tex = rd::<GPtr<u8>>(this, 0);
    let f = rd::<u32>(tex, 0x50);
    wr::<u32>(tex, 0x50, f ^ ((v << 12 ^ f) & 0xF000));
    tex
}

/// `CSprite2d::SetAddressing(RwTextureAddressMode m)` (0x727320): filterAddressing bits 8..15 =
/// `(m & 0xF) * 0x11`; returns the texture, 0 without one.
#[hook(addr = 0x727320, conv = thiscall, name = "CSprite2d::SetAddressing", expect = "8B 11 85 D2 74 25", this = "CSprite2d")]
pub fn set_addressing(this: GPtr<u8>, mode: u32) -> GPtr<u8> {
    let tex = rd::<GPtr<u8>>(this, 0);
    if tex.is_null() {
        return GPtr::NULL;
    }
    let m = mode & 0xF;
    let f = rd::<u32>(tex, 0x50);
    wr::<u32>(tex, 0x50, ((m << 4 | m) << 8) | (f & 0xFFFF_00FF));
    rd::<GPtr<u8>>(this, 0)
}

/// `CSprite2d::InitPerFrame()` (0x727350): both buffer counters 0, `RecipNearClip = 1.0f /
/// Scene.camera->nearPlane` (+0x80), `NearScreenZ = RwEngineInstance->+0x18` (through `fld` / `fstp`).
#[hook(addr = 0x727350, conv = cdecl, name = "CSprite2d::InitPerFrame", expect = "33 C0 A3 5C 04 C8 00")]
pub fn init_per_frame() {
    NEXT_BUFFER_VERTEX.set(0);
    NEXT_BUFFER_INDEX.set(0);
    let cam = SCENE_CAMERA.get();
    let eng = g::<GPtr<u8>>(RW_ENGINE_INSTANCE).read();
    RECIP_NEAR_CLIP.set(f32_of(x(rd::<f32>(cam, 0x80)).fdivr_m(f32::from_bits(ONE))));
    NEAR_SCREEN_Z.set(f32_of(x(rd::<f32>(eng, 0x18))));
}

/// `CSprite2d::IsVertexBufferEmpty()` (0x727390): `nextBufferVertex == 0` (EAX 0 / 1).
#[hook(addr = 0x727390, conv = cdecl, name = "CSprite2d::IsVertexBufferEmpty", expect = "8B 0D 5C 04 C8 00")]
pub fn is_vertex_buffer_empty() -> u32 {
    u32::from(NEXT_BUFFER_VERTEX.get() == 0)
}

/// `CSprite2d::IsVertexBufferFull()` (0x7273A0): `nextBufferVertex > 0x5FC || nextBufferIndex > 0xFFA`
/// (signed; EAX 0 / 1).
#[hook(addr = 0x7273A0, conv = cdecl, name = "CSprite2d::IsVertexBufferFull", expect = "81 3D 5C 04 C8 00 FC 05 00 00")]
pub fn is_vertex_buffer_full() -> u32 {
    u32::from(NEXT_BUFFER_VERTEX.get() > 0x5FC || NEXT_BUFFER_INDEX.get() > 0xFFA)
}

/// `CSprite2d::RenderVertexBuffer()` (0x7273D0): when vertices are buffered, shade mode Gouraud (2),
/// one indexed triangle-list draw (`RwIm2DRenderIndexedPrimitive(3, aRadiosityVertexBuffer,
/// nextBufferVertex, aTempBufferIndices, nextBufferIndex)`), then both counters 0.
#[hook(addr = 0x7273D0, conv = cdecl, name = "CSprite2d::RenderVertexBuffer", expect = "A1 5C 04 C8 00")]
pub fn render_vertex_buffer() {
    if NEXT_BUFFER_VERTEX.get() <= 0 {
        return;
    }
    rw_render_state_set(RS_SHADEMODE, 2);
    let (ni, nv) = (NEXT_BUFFER_INDEX.get(), NEXT_BUFFER_VERTEX.get());
    rw_im2d_render_indexed_primitive(
        3,
        GPtr::from_addr(RADIOSITY_VERTEX_BUFFER as usize),
        nv,
        GPtr::from_addr(TEMP_BUFFER_INDICES as usize),
        ni,
    );
    NEXT_BUFFER_VERTEX.set(0);
    NEXT_BUFFER_INDEX.set(0);
}

/// `CSprite2d::SetVertices(const CRect& r, const CRGBA& c1, c2, c3, c4)` (0x727420): z = NearScreenZ and
/// rhw = RecipNearClip (both through `fld` / `fst`); v0 = (left, bottom, c3, 1/1024, 1/1024), v1 = (right,
/// bottom, c4, 1+1/1024, 1/1024), v2 = (right, top, c2, 1+1/1024, 1+1/1024), v3 = (left, top, c1, 1/1024,
/// 1+1/1024).
#[hook(addr = 0x727420, conv = cdecl, name = "CSprite2d::SetVertices-CRectCRGBA4", expect = "8B 44 24 04 D9 05 60 04 C8 00")]
pub fn set_vertices_rect(r: GPtr<u8>, c1: GPtr<u8>, c2: GPtr<u8>, c3: GPtr<u8>, c4: GPtr<u8>) {
    let z = through_x87(near_z_bits());
    let w = through_x87(recip_bits());
    let (l, t, rt, b) = (
        rd::<u32>(r, 0),
        rd::<u32>(r, 4),
        rd::<u32>(r, 8),
        rd::<u32>(r, 0xC),
    );
    vertex(vtx(0), l, b, z, w, argb(c3), UV_LO, UV_LO);
    vertex(vtx(1), rt, rd::<u32>(r, 0xC), z, w, argb(c4), UV_HI, UV_LO);
    vertex(vtx(2), rd::<u32>(r, 8), t, z, w, argb(c2), UV_HI, UV_HI);
    vertex(
        vtx(3),
        rd::<u32>(r, 0),
        rd::<u32>(r, 4),
        z,
        w,
        argb(c1),
        UV_LO,
        UV_HI,
    );
}

/// `CSprite2d::SetVertices(x1, y1, x2, y2, x3, y3, x4, y4, const CRGBA& c1, c2, c3, c4)` (0x727590): z =
/// NearScreenZ (`fld` / `fst`), rhw = RecipNearClip (dword copy); v0 = (x3, y3, c3, 0, 0), v1 = (x4, y4, c4,
/// 1, 0), v2 = (x2, y2, c2, 1, 1), v3 = (x1, y1, c1, 0, 1).
#[hook(addr = 0x727590, conv = cdecl, name = "CSprite2d::SetVertices-ffffffffCRGBA", expect = "D9 05 60 04 C8 00")]
#[allow(clippy::too_many_arguments)]
pub fn set_vertices_quad(
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
    let z = through_x87(near_z_bits());
    let w = recip_bits();
    vertex(vtx(0), x3, y3, z, w, argb(c3), 0, 0);
    vertex(vtx(1), x4, y4, z, w, argb(c4), ONE, 0);
    vertex(vtx(2), x2, y2, z, w, argb(c2), ONE, ONE);
    vertex(vtx(3), x1, y1, z, w, argb(c1), 0, ONE);
}

/// `CSprite2d::SetVertices(const CRect& r, const CRGBA& c1..c4, u1, v1, u2, v2, u3, v3, u4, v4)` (0x727710):
/// as the CRect overload with explicit texture coordinates, rhw a dword copy; v0 gets (u1, v1), v1 (u2,
/// v2), v2 (u4, v4), v3 (u3, v3).
#[hook(addr = 0x727710, conv = cdecl, name = "CSprite2d::SetVertices-CRectCRGBA4ffffffff", expect = "8B 44 24 04 D9 05 60 04 C8 00")]
#[allow(clippy::too_many_arguments)]
pub fn set_vertices_rect_uv(
    r: GPtr<u8>,
    c1: GPtr<u8>,
    c2: GPtr<u8>,
    c3: GPtr<u8>,
    c4: GPtr<u8>,
    u1: u32,
    v1: u32,
    u2: u32,
    v2: u32,
    u3: u32,
    v3: u32,
    u4: u32,
    v4: u32,
) {
    set_vertices_uv_into(
        GPtr::from_addr(VERTICES),
        r,
        c1,
        c2,
        c3,
        c4,
        [u1, v1, u2, v2, u3, v3, u4, v4],
    );
}

/// The common body of 0x727710 and 0x727A00 (destination vertices `v`).
#[allow(clippy::too_many_arguments)]
fn set_vertices_uv_into(
    v: GPtr<u8>,
    r: GPtr<u8>,
    c1: GPtr<u8>,
    c2: GPtr<u8>,
    c3: GPtr<u8>,
    c4: GPtr<u8>,
    uv: [u32; 8],
) {
    let z = through_x87(near_z_bits());
    let w = recip_bits;
    let rr = |o: isize| rd::<u32>(r, o);
    vertex(v, rr(0), rr(0xC), z, w(), argb(c3), uv[0], uv[1]);
    vertex(v.byte_add(0x1C), rr(8), rr(0xC), z, w(), argb(c4), uv[2], uv[3]);
    vertex(v.byte_add(0x38), rr(8), rr(4), z, w(), argb(c2), uv[6], uv[7]);
    vertex(v.byte_add(0x54), rr(0), rr(4), z, w(), argb(c1), uv[4], uv[5]);
}

/// `CSprite2d::SetVertices(int n, float* posn, float* texCoors, const CRGBA& c)` (0x727890): for each of
/// the `n` vertices (no bound: n > 4 writes past maVertices) x / y / u through `fld` / `fstp`, v a dword
/// copy, z = `NearScreenZ + 1e-6f` (x87), rhw = RecipNearClip, colour c.
#[hook(addr = 0x727890, conv = cdecl, name = "CSprite2d::SetVertices-iffCRGBA", expect = "8B 44 24 04 D9 05 60 04 C8 00")]
pub fn set_vertices_n_uv(n: i32, posn: GPtr<u32>, tex: GPtr<u32>, c: GPtr<u8>) {
    if n <= 0 {
        return;
    }
    let z = f32_of(x(f32::from_bits(near_z_bits())).fadd_m(f32::from_bits(Z_BIAS)));
    let mut i = 0;
    while i < n {
        let v = vtx(i as isize);
        wr::<u32>(v, 0, through_x87(posn.wrapping_index(2 * i).read()));
        wr::<u32>(v, 0xC, recip_bits());
        wr::<u32>(v, 4, through_x87(posn.wrapping_index(2 * i + 1).read()));
        wr::<u32>(v, 8, z.to_bits());
        wr::<u32>(v, 0x14, through_x87(tex.wrapping_index(2 * i).read()));
        wr::<u32>(v, 0x18, tex.wrapping_index(2 * i + 1).read());
        wr::<u32>(v, 0x10, argb(c));
        i += 1;
    }
}

/// `CSprite2d::SetVertices(int n, float* posn, const CRGBA* colors)` (0x727920): for each vertex x / y
/// through `fld` / `fstp`, z = NearScreenZ and rhw = RecipNearClip as dword copies, u = v = 1.0f, colour
/// `colors[i]`.
#[hook(addr = 0x727920, conv = cdecl, name = "CSprite2d::SetVertices-ifCRGBA", expect = "51 A1 60 04 C8 00")]
pub fn set_vertices_n_colors(n: i32, posn: GPtr<u32>, colors: GPtr<u8>) {
    let z = near_z_bits();
    let mut i = 0;
    while i < n {
        let v = vtx(i as isize);
        wr::<u32>(v, 0, through_x87(posn.wrapping_index(2 * i).read()));
        wr::<u32>(v, 8, z);
        wr::<u32>(v, 0xC, recip_bits());
        wr::<u32>(v, 0x14, ONE);
        wr::<u32>(v, 0x18, ONE);
        wr::<u32>(v, 4, through_x87(posn.wrapping_index(2 * i + 1).read()));
        wr::<u32>(v, 0x10, argb(colors.byte_add(4 * i as isize)));
        i += 1;
    }
}

/// `CSprite2d::SetMaskVertices(int n, float* posn, float z)` (0x7279B0): x / y through `fld` / `fstp`, z
/// (dword), rhw = RecipNearClip, colour 0; u / v untouched.
#[hook(addr = 0x7279B0, conv = cdecl, name = "CSprite2d::SetMaskVertices", expect = "56 8B 74 24 08")]
pub fn set_mask_vertices(n: i32, posn: GPtr<u32>, z: u32) {
    let mut i = 0;
    while i < n {
        let v = vtx(i as isize);
        wr::<u32>(v, 0, through_x87(posn.wrapping_index(2 * i).read()));
        wr::<u32>(v, 8, z);
        wr::<u32>(v, 0xC, recip_bits());
        wr::<u32>(v, 4, through_x87(posn.wrapping_index(2 * i + 1).read()));
        wr::<u32>(v, 0x10, 0);
        i += 1;
    }
}

/// `CSprite2d::SetVertices(RwD3D9Vertex* v, const CRect& r, const CRGBA& c1..c4, u1, v1, .. u4, v4)`
/// (0x727A00): the explicit-UV CRect body written to `v` instead of maVertices.
#[hook(addr = 0x727A00, conv = cdecl, name = "CSprite2d::SetVertices-RwD3D9Vertex", expect = "8B 44 24 04 D9 05 60 04 C8 00")]
#[allow(clippy::too_many_arguments)]
pub fn set_vertices_d3d9(
    v: GPtr<u8>,
    r: GPtr<u8>,
    c1: GPtr<u8>,
    c2: GPtr<u8>,
    c3: GPtr<u8>,
    c4: GPtr<u8>,
    u1: u32,
    v1: u32,
    u2: u32,
    v2: u32,
    u3: u32,
    v3: u32,
    u4: u32,
    v4: u32,
) {
    set_vertices_uv_into(v, r, c1, c2, c3, c4, [u1, v1, u2, v2, u3, v3, u4, v4]);
}

/// `CSprite2d::SetRenderState()` (0x727B30): `RwRenderStateSet(TEXTURERASTER, texture ? texture->raster
/// : NULL)`.
#[hook(addr = 0x727B30, conv = thiscall, name = "CSprite2d::SetRenderState", expect = "8B 01 85 C0 75 0F", this = "CSprite2d")]
pub fn set_render_state(this: GPtr<u8>) {
    let tex = rd::<GPtr<u8>>(this, 0);
    let raster = if tex.is_null() { 0 } else { rd::<u32>(tex, 0) };
    rw_render_state_set(RS_TEXTURERASTER, raster);
}

/// `CSprite2d::DrawRect(const CRect& r, const CRGBA& c)` (0x727B60): no texture, the quad in colour c,
/// vertex alpha on unless `c.a == 255`, one triangle fan, then vertex alpha off. The entry jumps into
/// the protection chunk 0x409737, which loads `RwEngineInstance` into EAX and jumps back to 0x727B65.
/// Not hooked: the protection reads the entry's jump displacement (0x727B61, `mov ecx, [0x727B61]` at
/// 0x401462); callers reach the original through `orig!`.
pub fn draw_rect(r: GPtr<u8>, c: GPtr<u8>) {
    rw_render_state_set(RS_TEXTURERASTER, 0);
    set_vertices_rect_call(r, c, c, c, c);
    let alpha = u32::from(rd::<u8>(c, 3) != 0xFF);
    rw_render_state_set(RS_VERTEXALPHAENABLE, alpha);
    rw_im2d_render_primitive(5, GPtr::from_addr(VERTICES), 4);
    rw_render_state_set(RS_VERTEXALPHAENABLE, 0);
}

/// `CSprite2d::DrawTxRect(const CRect& r, const CRGBA& c)` (0x727BE0): the quad in colour c, one triangle
/// fan, render states untouched.
#[hook(addr = 0x727BE0, conv = cdecl, name = "CSprite2d::DrawTxRect", expect = "8B 44 24 08 50")]
pub fn draw_tx_rect(r: GPtr<u8>, c: GPtr<u8>) {
    set_vertices_rect_call(r, c, c, c, c);
    rw_im2d_render_primitive(5, GPtr::from_addr(VERTICES), 4);
}

/// `CSprite2d::DrawRect(const CRect& r, const CRGBA& c1, c2, c3, c4)` (0x727C10): no texture, the quad,
/// one triangle fan (vertex alpha untouched).
#[hook(addr = 0x727C10, conv = cdecl, name = "CSprite2d::DrawRect-CRGBA4", expect = "A1 24 7B C9 00")]
pub fn draw_rect4(r: GPtr<u8>, c1: GPtr<u8>, c2: GPtr<u8>, c3: GPtr<u8>, c4: GPtr<u8>) {
    rw_render_state_set(RS_TEXTURERASTER, 0);
    set_vertices_rect_call(r, c1, c2, c3, c4);
    rw_im2d_render_primitive(5, GPtr::from_addr(VERTICES), 4);
}

/// `CSprite2d::DrawRectXLU(const CRect& r, const CRGBA& c1..c4)` (0x727C50): the quad, no texture,
/// vertex alpha on, blend src alpha (5) / inv src alpha (6), one triangle fan.
#[hook(addr = 0x727C50, conv = cdecl, name = "CSprite2d::DrawRectXLU", expect = "8B 44 24 14 8B 4C 24 10")]
pub fn draw_rect_xlu(r: GPtr<u8>, c1: GPtr<u8>, c2: GPtr<u8>, c3: GPtr<u8>, c4: GPtr<u8>) {
    set_vertices_rect_call(r, c1, c2, c3, c4);
    rw_render_state_set(RS_TEXTURERASTER, 0);
    rw_render_state_set(RS_VERTEXALPHAENABLE, 1);
    rw_render_state_set(RS_SRCBLEND, 5);
    rw_render_state_set(RS_DESTBLEND, 6);
    rw_im2d_render_primitive(5, GPtr::from_addr(VERTICES), 4);
}

/// `CSprite2d::DrawAnyRect(x1, y1, .. y4, const CRGBA& c1..c4)` (0x727CC0): the free quad
/// (`SetVertices` 0x727590), vertex alpha off only when all four alphas are 255, one triangle fan.
#[hook(addr = 0x727CC0, conv = cdecl, name = "CSprite2d::DrawAnyRect", expect = "8B 44 24 20 8B 4C 24 1C")]
#[allow(clippy::too_many_arguments)]
pub fn draw_any_rect(
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
    set_vertices_quad_call(x1, y1, x2, y2, x3, y3, x4, y4, c1, c2, c3, c4);
    let opaque = [c1, c2, c3, c4].iter().all(|c| rd::<u8>(*c, 3) == 0xFF);
    rw_render_state_set(RS_VERTEXALPHAENABLE, u32::from(!opaque));
    rw_im2d_render_primitive(5, GPtr::from_addr(VERTICES), 4);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_CONSTRUCTOR,
    HOOK_DELETE,
    HOOK_SET_TEXTURE,
    HOOK_SET_TEXTURE_MASK,
    HOOK_SET_ADDRESSING_UV,
    HOOK_SET_ADDRESSING,
    HOOK_INIT_PER_FRAME,
    HOOK_IS_VERTEX_BUFFER_EMPTY,
    HOOK_IS_VERTEX_BUFFER_FULL,
    HOOK_RENDER_VERTEX_BUFFER,
    HOOK_SET_VERTICES_RECT,
    HOOK_SET_VERTICES_QUAD,
    HOOK_SET_VERTICES_RECT_UV,
    HOOK_SET_VERTICES_N_UV,
    HOOK_SET_VERTICES_N_COLORS,
    HOOK_SET_MASK_VERTICES,
    HOOK_SET_VERTICES_D3D9,
    HOOK_SET_RENDER_STATE,
    HOOK_DRAW_TX_RECT,
    HOOK_DRAW_RECT4,
    HOOK_DRAW_RECT_XLU,
    HOOK_DRAW_ANY_RECT,
];
