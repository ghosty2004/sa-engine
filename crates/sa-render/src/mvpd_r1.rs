//! WRITER-MVP-D (batch `mvpd-render-1`): the real-time shadow pipeline of the in-world frame —
//! `CShadowCamera` (centre, raster invert / resample / blur, outline border, quad helper),
//! `CRealTimeShadow` (update, light, texture), `CRealTimeShadowManager::Update` and
//! `CShadows::StoreRealTimeShadow` / `StoreShadowToBeRendered` (texture overload).
//!
//! Ported from the 1.0 US bytes; gta-reversed `ShadowCamera.cpp`, `RealTimeShadow.cpp`,
//! `RealTimeShadowManager.cpp` and `Shadows.cpp` as reading aids. x87 arithmetic follows the
//! original's instruction order on the `sa_math::x87::X87` model. Vertex buffers the original builds
//! in its own frame are built in Rust locals; the fields it leaves unwritten (texture coordinates of
//! untextured quads) are zero here (hazard `uninit-locals`).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, jp, k, rd, wr, x};

orig! {
    /// `RwRenderStateSet` (`RwEngineInstance` slot 0x20)
    fn rs(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwMatrixUpdate(RwMatrix*)` (0x7F18A0)
    fn rw_matrix_update(m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18A0 => unported;
    /// `RwFrameUpdateObjects(RwFrame*)` (0x7F0910)
    fn rw_frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => unported;
    /// `RwFrameOrthoNormalize(RwFrame*)` (0x7F1170)
    fn rw_frame_ortho_normalize(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F1170 => unported;
    /// `RwFrameRotate(RwFrame*, const RwV3d* axis, RwReal angle, RwOpCombineType)` (0x7F1010)
    fn rw_frame_rotate(f: GPtr<u8>, axis: GPtr<u8>, angle: u32, combine: u32) -> GPtr<u8> = cdecl 0x7F1010 => unported;
    /// `RwCameraClear(RwCamera*, RwRGBA*, RwInt32 mode)` (0x7EE340)
    fn rw_camera_clear(cam: GPtr<u8>, colour: GPtr<u8>, mode: u32) -> GPtr<u8> = cdecl 0x7EE340 => unported;
    /// `RwCameraBeginUpdate(RwCamera*)` (0x7EE190)
    fn rw_camera_begin_update(cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EE190 => unported;
    /// `RwCameraEndUpdate(RwCamera*)` (0x7EE180)
    fn rw_camera_end_update(cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EE180 => unported;
    /// `GetFirstAtomic(RpClump*)` (0x734820)
    fn get_first_atomic(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734820 => unported;
    /// `RpClumpForAllAtomics(RpClump*, callback, data)` (0x749B70)
    fn rp_clump_for_all_atomics(clump: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `RwIm2DRenderPrimitive(type, verts, n)` (0x734E90)
    fn im2d_render_primitive(prim: u32, verts: GPtr<u8>, n: i32) -> i32 = cdecl 0x734E90 => unported;
    /// `RwIm2DRenderIndexedPrimitive(type, verts, nv, indices, ni)` (0x734EA0)
    fn im2d_render_indexed_primitive(prim: u32, verts: GPtr<u8>, nv: i32, idx: GPtr<u8>, ni: i32) -> i32 = cdecl 0x734EA0 => unported;
    /// `RwV3dTransformPoints(RwV3d* out, const RwV3d* in, RwInt32 n, const RwMatrix*)` (0x7EDD90)
    fn rw_v3d_transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
    /// `CShadowCamera::SetLight(RpLight*)` (0x705520)
    fn shadow_camera_set_light(this: GPtr<u8>, light: GPtr<u8>) -> GPtr<u8> = thiscall 0x705520 => unported;
    /// `CShadowCamera::Update(RpAtomic*)` (0x705C80)
    fn shadow_camera_update_atomic(this: GPtr<u8>, atomic: GPtr<u8>) -> GPtr<u8> = thiscall 0x705C80 => unported;
    /// `CRealTimeShadowManager::ReInit()` (0x706870)
    fn rts_manager_reinit(this: GPtr<u8>) = thiscall 0x706870 => unported;
    /// `CPed::GetBonePosition(RwV3d& out, uint32 bone, bool updateSkin)` (0x5E4280)
    fn ped_get_bone_position(this: GPtr<u8>, out: GPtr<u8>, bone: u32, update: u32) = thiscall 0x5E4280 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CCamera::IsSphereVisible(const CVector&, float)` (0x420D40, bool in AL)
    fn camera_is_sphere_visible(this: GPtr<u8>, c: GPtr<u8>, r: u32) -> u8 = thiscall 0x420D40 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(this: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CShadows::CalcPedShadowValues(CVector sun, float& dx, float& dy, float& fx, float& fy, float& sx,
    /// float& sy)` (0x7076C0; the vector by value, three dwords)
    #[allow(clippy::too_many_arguments)]
    fn calc_ped_shadow_values(sx: u32, sy: u32, sz: u32, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, d: GPtr<u8>, e: GPtr<u8>, f: GPtr<u8>) = cdecl 0x7076C0 => unported;
}

orig! {
    /// `CShadowCamera::Im2DRenderQuad` (0x705A20)
    #[allow(clippy::too_many_arguments)]
    fn im2d_render_quad_o(x1: u32, y1: u32, x2: u32, y2: u32, z: u32, recip_z: u32, uv: u32) -> u32 = cdecl 0x705A20 => crate::mvpd_r1::im2d_render_quad;
    /// `CShadowCamera::DrawOutlineBorder(const CRGBA&)` (0x705790)
    fn draw_outline_border_o(this: GPtr<u8>, colour: GPtr<u8>) -> GPtr<u8> = thiscall 0x705790 => crate::mvpd_r1::draw_outline_border;
    /// `CShadowCamera::SetCenter(RwV3d*)` (0x705590)
    fn set_center_o(this: GPtr<u8>, c: GPtr<u8>) -> GPtr<u8> = thiscall 0x705590 => crate::mvpd_r1::set_center;
    /// `CShadowCamera::InvertRaster()` (0x705660)
    fn invert_raster_o(this: GPtr<u8>) = thiscall 0x705660 => crate::mvpd_r1::invert_raster;
    /// `CShadowCamera::Update(RpClump*)` (0x705BF0)
    fn update_clump_o(this: GPtr<u8>, clump: GPtr<u8>) -> GPtr<u8> = thiscall 0x705BF0 => crate::mvpd_r1::update_clump;
    /// `CShadowCamera::RasterResample(RwRaster*)` (0x706070)
    fn raster_resample_o(this: GPtr<u8>, r: GPtr<u8>) -> GPtr<u8> = thiscall 0x706070 => crate::mvpd_r1::raster_resample;
    /// `CShadowCamera::RasterBlur(RwRaster*, int32)` (0x706170)
    fn raster_blur_o(this: GPtr<u8>, r: GPtr<u8>, passes: i32) -> GPtr<u8> = thiscall 0x706170 => crate::mvpd_r1::raster_blur;
    /// `CShadowCamera::DrawBlurryRaster2(RwRaster*)` (0x706330)
    fn draw_blurry_raster2_o(this: GPtr<u8>, r: GPtr<u8>) -> GPtr<u8> = thiscall 0x706330 => crate::mvpd_r1::draw_blurry_raster2;
    /// `CRealTimeShadow::Update()` (0x706600)
    fn rts_update_o(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x706600 => crate::mvpd_r1::rts_update;
    /// `CRealTimeShadow::GetShadowRwTexture()` (0x7059F0)
    fn get_shadow_rw_texture_o(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x7059F0 => crate::mvpd_r1::get_shadow_rw_texture;
    /// `CRealTimeShadow::DrawBorderAroundTexture(const CRGBA&)` (0x705A00)
    fn draw_border_around_texture_o(this: GPtr<u8>, c: GPtr<u8>) -> GPtr<u8> = thiscall 0x705A00 => crate::mvpd_r1::draw_border_around_texture;
    /// `CRealTimeShadow::SetLightProperties(float, float, bool)` (0x705900)
    fn set_light_properties_o(this: GPtr<u8>, angle: u32, unused: u32, set_light: u32) -> GPtr<u8> = thiscall 0x705900 => crate::mvpd_r1::set_light_properties;
    /// `CShadows::StoreRealTimeShadow(CPhysical*, 6 floats)` (0x707CA0)
    #[allow(clippy::too_many_arguments)]
    fn store_real_time_shadow_o(e: GPtr<u8>, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32) = cdecl 0x707CA0 => crate::mvpd_r1::store_real_time_shadow;
    /// `CShadows::StoreShadowToBeRendered(type, RwTexture*, CVector*, 4 floats, intensity, r, g, b,
    /// zDistance, drawOnWater, scale, CRealTimeShadow*, drawOnBuildings)` (0x707390)
    #[allow(clippy::too_many_arguments)]
    fn store_shadow_to_be_rendered_tex_o(ty: u32, tex: GPtr<u8>, pos: GPtr<u8>, fx: u32, fy: u32, sx: u32, sy: u32, intensity: u32, r: u32, gg: u32, b: u32, zd: u32, water: u32, scale: u32, rts: GPtr<u8>, bld: u32) = cdecl 0x707390 => crate::mvpd_r1::store_shadow_to_be_rendered_tex;
}

/// `RwEngineInstance` (0xC97B24): its +0x18 is the Im2D near screen z.
fn near_screen_z() -> u32 {
    g::<GPtr<u8>>(0xC97B24).read().byte_add(0x18).cast::<u32>().read()
}

fn lp<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

/// 1.0 (0x858624)
const K_ONE: u32 = 0x3F80_0000;
/// 0.5 (0x872670)
const K_HALF_A: u32 = 0x3F00_0000;
/// 0.5 (0x858B8C)
const K_HALF_B: u32 = 0x3F00_0000;

// ------------------------------------------------------------------ CShadowCamera -----------------

/// `CShadowCamera::SetCenter(RwV3d* center)` (0x705590, thiscall, `ret 4`): the camera frame's
/// modelling matrix (`[[this]+4]+0x10`) gets `pos = center`, then each component
/// `pos.c += (camera farPlane (+0x84) * -0.5) * at.c` (f32 stores, x, y, z in turn);
/// `RwMatrixUpdate`, `RwFrameUpdateObjects`, `RwFrameOrthoNormalize`. Returns the RwCamera.
#[hook(addr = 0x705590, conv = thiscall, name = "CShadowCamera::SetCenter", this = "CShadowCamera", expect = "8B 54 24 04 53",
       unverified = "mvpd-render-1", route = "L2", hazards = "heap-writes",
       recs = "grove-walk", note = "writes the shadow camera's RwFrame matrix (RW heap)")]
pub fn set_center(this: GPtr<u8>, center: GPtr<u8>) -> GPtr<u8> {
    let cam = rd::<GPtr<u8>>(this, 0);
    let frame = rd::<GPtr<u8>>(cam, 4);
    let m = frame.byte_add(0x10);
    let pos = m.byte_add(0x30);
    wr::<u32>(pos, 0, rd::<u32>(center, 0));
    wr::<u32>(pos, 4, rd::<u32>(center, 4));
    wr::<u32>(pos, 8, rd::<u32>(center, 8));
    for c in 0..3isize {
        let far = rd::<f32>(rd::<GPtr<u8>>(this, 0), 0x84);
        let v = x(far).fmul_m(k(0xBF00_0000)).fmul_m(rd::<f32>(m, 0x20 + c * 4)).fadd_m(rd::<f32>(pos, c * 4));
        wr::<f32>(pos, c * 4, v.to_f32());
    }
    rw_matrix_update(m);
    rw_frame_update_objects(frame);
    rw_frame_ortho_normalize(frame);
    rd::<GPtr<u8>>(this, 0)
}

/// One `RwIm2DVertex` (0x1C bytes): x, y, z, rhw, emissive colour, u, v.
type Vtx = [u32; 7];

/// `CShadowCamera::InvertRaster()` (0x705660, thiscall): an untextured 4-vertex strip over the camera's
/// frame buffer (`[cam+0x60]`; x 0 / width, y 0 / height from `fild`, z = Im2D near z, rhw =
/// 1 / nearPlane (+0x80), colour 0xFFFFFFFF; u/v left unwritten by the original) drawn with
/// z-test off (6,0), no raster (1,0), vertex alpha on (12,1), blend INVDESTCOLOR/ZERO (10,10)(11,1);
/// then (6,1), (10,5), (11,6).
#[hook(addr = 0x705660, conv = thiscall, name = "CShadowCamera::InvertRaster", this = "CShadowCamera", expect = "83 EC 74 8B 01",
       unverified = "mvpd-render-1", route = "L2", hazards = "uninit-locals",
       note = "the original leaves the vertices' u/v unwritten (stack garbage, untextured)")]
pub fn invert_raster(this: GPtr<u8>) {
    let cam = rd::<GPtr<u8>>(this, 0);
    let fb = rd::<GPtr<u8>>(cam, 0x60);
    let w = X87::from_i32(rd::<i32>(fb, 0xC)).to_f32().to_bits();
    let h = X87::from_i32(rd::<i32>(fb, 0x10)).to_f32().to_bits();
    let r = x(rd::<f32>(cam, 0x80)).fdivr_m(k(K_ONE)).to_f32().to_bits();
    let z = near_screen_z();
    let mut v: [Vtx; 4] = [
        [0, 0, z, r, 0xFFFF_FFFF, 0, 0],
        [0, h, z, r, 0xFFFF_FFFF, 0, 0],
        [w, 0, z, r, 0xFFFF_FFFF, 0, 0],
        [w, h, z, r, 0xFFFF_FFFF, 0, 0],
    ];
    rs(6, 0);
    rs(1, 0);
    rs(0xC, 1);
    rs(0xA, 0xA);
    rs(0xB, 1);
    im2d_render_primitive(4, lp(&mut v), 4);
    rs(6, 1);
    rs(0xA, 5);
    rs(0xB, 6);
}

/// `CShadowCamera::DrawOutlineBorder(const CRGBA& colour)` (0x705790, thiscall, `ret 4`): a closed
/// polyline (indices 0,1,2,3,0) around the frame buffer raster: corners (0,0), (w-1,0), (w-1,w-1),
/// (0,w-1) — the original uses the width minus one for both axes —, z = Im2D near z, rhw =
/// 1 / nearPlane, colour packed 0xAARRGGBB, u/v unwritten. Inside `RwCameraBeginUpdate`: (6,0),
/// (12,0), (1,0), `RwIm2DRenderIndexedPrimitive(2, …, 4, …, 5)`, (6,1), (12,1), `RwCameraEndUpdate`.
/// Returns the frame buffer raster.
#[hook(addr = 0x705790, conv = thiscall, name = "CShadowCamera::DrawOutlineBorder", this = "CShadowCamera", expect = "81 EC 80 00 00 00",
       unverified = "mvpd-render-1", route = "L2", hazards = "uninit-locals",
       note = "the original leaves the vertices' u/v unwritten (stack garbage, untextured)")]
pub fn draw_outline_border(this: GPtr<u8>, colour: GPtr<u8>) -> GPtr<u8> {
    let cam = rd::<GPtr<u8>>(this, 0);
    let fb = rd::<GPtr<u8>>(cam, 0x60);
    let wm1 = X87::from_i32(rd::<i32>(fb, 0xC)).fsub_m(k(K_ONE)).to_f32().to_bits();
    let z = near_screen_z();
    let r = x(rd::<f32>(cam, 0x80)).fdivr_m(k(K_ONE)).to_f32().to_bits();
    let c = colour.cast::<[u8; 4]>().read();
    let col = (u32::from(c[3]) << 24) | (u32::from(c[0]) << 16) | (u32::from(c[1]) << 8) | u32::from(c[2]);
    let mut idx: [u16; 5] = [0, 1, 2, 3, 0];
    let mut v: [Vtx; 4] = [
        [0, 0, z, r, col, 0, 0],
        [wm1, 0, z, r, col, 0, 0],
        [wm1, wm1, z, r, col, 0, 0],
        [0, wm1, z, r, col, 0, 0],
    ];
    if !rw_camera_begin_update(cam).is_null() {
        rs(6, 0);
        rs(0xC, 0);
        rs(1, 0);
        im2d_render_indexed_primitive(2, lp(&mut v), 4, lp(&mut idx), 5);
        rs(6, 1);
        rs(0xC, 1);
        rw_camera_end_update(rd::<GPtr<u8>>(this, 0));
    }
    fb
}

/// `CRealTimeShadow::SetLightProperties(float angle, float unused, bool setLight)` (0x705900, thiscall,
/// `ret 0xC`): with the light's frame (`[[this+0x28]+4]`; NULL returns NULL),
/// `RwFrameRotate(frame, (0,1,0), angle, REPLACE)` then `RwFrameRotate(frame, (1,0,0), -90, POSTCONCAT)`;
/// with `setLight`, `m_Camera.SetLight(m_pLight)`. Returns the frame. The second float is ignored.
#[hook(addr = 0x705900, conv = thiscall, name = "CRealTimeShadow::SetLightProperties", this = "CRealTimeShadow", expect = "83 EC 18 56 57",
       unverified = "mvpd-render-1", route = "L2", hazards = "heap-writes",
       note = "rotates the shadow light's RwFrame (RW heap)")]
pub fn set_light_properties(this: GPtr<u8>, angle: u32, _unused: u32, set_light: u32) -> GPtr<u8> {
    let mut x_axis: [u32; 3] = [K_ONE, 0, 0];
    let mut y_axis: [u32; 3] = [0, K_ONE, 0];
    let frame = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x28), 4);
    if frame.is_null() {
        return GPtr::NULL;
    }
    rw_frame_rotate(frame, lp(&mut y_axis), angle, 0);
    rw_frame_rotate(frame, lp(&mut x_axis), 0xC2B4_0000, 2);
    if set_light as u8 != 0 {
        shadow_camera_set_light(this.byte_add(8), rd::<GPtr<u8>>(this, 0x28));
    }
    frame
}

/// `CRealTimeShadow::GetShadowRwTexture()` (0x7059F0, thiscall): `m_bBlurred` (+0x10) ?
/// `m_pRwTexture` of the blurred camera (+0x18) : that of the main camera (+0xC).
#[hook(addr = 0x7059F0, conv = thiscall, name = "CRealTimeShadow::GetShadowRwTexture", this = "CRealTimeShadow", expect = "8A 41 10 84 C0",
       unverified = "mvpd-render-1", route = "L2", hazards = "none")]
pub fn get_shadow_rw_texture(this: GPtr<u8>) -> GPtr<u8> {
    if rd::<u8>(this, 0x10) != 0 {
        rd::<GPtr<u8>>(this, 0x18)
    } else {
        rd::<GPtr<u8>>(this, 0xC)
    }
}

/// `CRealTimeShadow::DrawBorderAroundTexture(const CRGBA&)` (0x705A00, thiscall, tail jumps):
/// `DrawOutlineBorder` of the blurred camera (+0x14) when `m_bBlurred`, else of `m_Camera` (+8).
#[hook(addr = 0x705A00, conv = thiscall, name = "CRealTimeShadow::DrawBorderAroundTexture", this = "CRealTimeShadow", expect = "8A 41 10 84 C0",
       unverified = "mvpd-render-1", route = "L2", hazards = "uninit-locals",
       note = "through DrawOutlineBorder (unwritten u/v)")]
pub fn draw_border_around_texture(this: GPtr<u8>, colour: GPtr<u8>) -> GPtr<u8> {
    if rd::<u8>(this, 0x10) != 0 {
        draw_outline_border_o(this.byte_add(0x14), colour)
    } else {
        draw_outline_border_o(this.byte_add(8), colour)
    }
}

/// `CShadowCamera::Im2DRenderQuad(x1, y1, x2, y2, z, recipZ, uv)` (0x705A20, cdecl): a white
/// (0xFFFFFFFF) 4-vertex strip (`RwIm2DRenderPrimitive(4, …, 4)`): (x1,y1 | u,u), (x1,y2 | u,u+1),
/// (x2,y1 | u+1,u), (x2,y2 | u+1,u+1), each with z and rhw = recipZ; `u + 1` rounded to f32. Returns 1.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x705A20, conv = cdecl, name = "CShadowCamera::Im2DRenderQuad", expect = "83 EC 70 8B 4C 24 78",
       unverified = "mvpd-render-1", route = "L2", hazards = "none")]
pub fn im2d_render_quad(x1: u32, y1: u32, x2: u32, y2: u32, z: u32, recip_z: u32, uv: u32) -> u32 {
    let uv1 = x(f32::from_bits(uv)).fadd_m(k(K_ONE)).to_f32().to_bits();
    let c = 0xFFFF_FFFF;
    let mut v: [Vtx; 4] = [
        [x1, y1, z, recip_z, c, uv, uv],
        [x1, y2, z, recip_z, c, uv, uv1],
        [x2, y1, z, recip_z, c, uv1, uv],
        [x2, y2, z, recip_z, c, uv1, uv1],
    ];
    im2d_render_primitive(4, lp(&mut v), 4);
    1
}

/// `CShadowCamera::Update(RpClump*)` (0x705BF0, thiscall, `ret 4`): `RwCameraClear(cam, {255,255,255,0},
/// IMAGE|Z)`; inside `RwCameraBeginUpdate`: the first atomic's geometry flags (+8) are masked with
/// 0xFFFFFF13 while `RpClumpForAllAtomics(clump, atomicQuickRender 0x705620, NULL)` runs, then restored;
/// `InvertRaster()`, `RwCameraEndUpdate`. Returns the RwCamera.
#[hook(addr = 0x705BF0, conv = thiscall, name = "CShadowCamera::Update-Clump", this = "CShadowCamera", expect = "51 B0 FF 56 88 44 24 04",
       unverified = "mvpd-render-1", route = "L2", hazards = "heap-writes",
       note = "masks and restores the first atomic's geometry flags (RW heap)")]
pub fn update_clump(this: GPtr<u8>, clump: GPtr<u8>) -> GPtr<u8> {
    let mut col: [u8; 4] = [0xFF, 0xFF, 0xFF, 0];
    rw_camera_clear(rd::<GPtr<u8>>(this, 0), lp(&mut col), 3);
    if !rw_camera_begin_update(rd::<GPtr<u8>>(this, 0)).is_null() {
        let atomic = get_first_atomic(clump);
        let geom = rd::<GPtr<u8>>(atomic, 0x18);
        let flags = rd::<u32>(geom, 8);
        wr::<u32>(geom, 8, flags & 0xFFFF_FF13);
        rp_clump_for_all_atomics(clump, 0x705620, 0);
        wr::<u32>(geom, 8, flags);
        invert_raster_o(this);
        rw_camera_end_update(rd::<GPtr<u8>>(this, 0));
    }
    rd::<GPtr<u8>>(this, 0)
}

/// `CShadowCamera::RasterResample(RwRaster* src)` (0x706070, thiscall, `ret 4`): NULL without a camera.
/// Inside `RwCameraBeginUpdate`: (10,2), (11,1), (6,0), (9,2), (1,src), `Im2DRenderQuad(0, 0, w, w,
/// nearZ, 1/nearPlane, 0.5/w)` with w the frame buffer width as f32 (used for both axes), (6,1),
/// (10,5), (11,6), `RwCameraEndUpdate`. Returns the frame buffer raster.
#[hook(addr = 0x706070, conv = thiscall, name = "CShadowCamera::RasterResample", this = "CShadowCamera", expect = "83 EC 0C 56 8B F1",
       unverified = "mvpd-render-1", route = "L2", hazards = "none")]
pub fn raster_resample(this: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    let cam = rd::<GPtr<u8>>(this, 0);
    if cam.is_null() {
        return GPtr::NULL;
    }
    let fb = rd::<GPtr<u8>>(cam, 0x60);
    let w = X87::from_i32(rd::<i32>(fb, 0xC)).to_f32();
    let uv = x(k(K_HALF_A)).fdiv_m(w).to_f32().to_bits();
    let r = x(rd::<f32>(cam, 0x80)).fdivr_m(k(K_ONE)).to_f32().to_bits();
    if !rw_camera_begin_update(cam).is_null() {
        rs(0xA, 2);
        rs(0xB, 1);
        rs(6, 0);
        rs(9, 2);
        rs(1, src.addr() as u32);
        let wb = w.to_bits();
        im2d_render_quad_o(0, 0, wb, wb, near_screen_z(), r, uv);
        rs(6, 1);
        rs(0xA, 5);
        rs(0xB, 6);
        rw_camera_end_update(rd::<GPtr<u8>>(this, 0));
    }
    fb
}

/// `CShadowCamera::RasterBlur(RwRaster* src, int32 passes)` (0x706170, thiscall, `ret 8`): NULL without
/// a camera. With w = src width (f32) and r = 1/nearPlane, each pass: the camera renders `src` into
/// its own frame buffer with uv offset `(0.5 + 0.5) * (1/w)` (blend state (10,2)(11,1)(6,0)(9,2)
/// set on the first pass only), then the frame buffer is switched to `src` and the original buffer
/// is drawn back with uv `(0.5 - 0.5) * (1/w)`; after the last pass's second quad (6,1)(10,5)(11,6)
/// are restored. Each half only inside a successful `RwCameraBeginUpdate`. Quads are
/// `(0, 0, w, w, nearZ, r, uv)`. The camera's frame buffer is restored; returns `src`.
#[hook(addr = 0x706170, conv = thiscall, name = "CShadowCamera::RasterBlur", this = "CShadowCamera", expect = "83 EC 10 57 8B F9",
       unverified = "mvpd-render-1", route = "L2", hazards = "heap-writes",
       note = "switches the RwCamera frame buffer (+0x60) back and forth (RW heap)")]
pub fn raster_blur(this: GPtr<u8>, src: GPtr<u8>, passes: i32) -> GPtr<u8> {
    let cam0 = rd::<GPtr<u8>>(this, 0);
    if cam0.is_null() {
        return GPtr::NULL;
    }
    let w = X87::from_i32(rd::<i32>(src, 0xC)).to_f32();
    let orig_fb = rd::<u32>(cam0, 0x60);
    let inv_w = x(k(K_ONE)).fdiv_m(w).to_f32();
    let r = x(rd::<f32>(cam0, 0x80)).fdivr_m(k(K_ONE)).to_f32().to_bits();
    let wb = w.to_bits();
    let mut i = 0i32;
    while i < passes {
        wr::<u32>(rd::<GPtr<u8>>(this, 0), 0x60, orig_fb);
        if !rw_camera_begin_update(rd::<GPtr<u8>>(this, 0)).is_null() {
            if i == 0 {
                rs(0xA, 2);
                rs(0xB, 1);
                rs(6, 0);
                rs(9, 2);
            }
            rs(1, src.addr() as u32);
            let uv = x(k(K_HALF_A)).fadd_m(k(K_HALF_B)).fmul_m(inv_w).to_f32().to_bits();
            im2d_render_quad_o(0, 0, wb, wb, near_screen_z(), r, uv);
            rw_camera_end_update(rd::<GPtr<u8>>(this, 0));
        }
        wr::<u32>(rd::<GPtr<u8>>(this, 0), 0x60, src.addr() as u32);
        if !rw_camera_begin_update(rd::<GPtr<u8>>(this, 0)).is_null() {
            rs(1, orig_fb);
            let uv = x(k(K_HALF_A)).fsub_m(k(K_HALF_B)).fmul_m(inv_w).to_f32().to_bits();
            im2d_render_quad_o(0, 0, wb, wb, near_screen_z(), r, uv);
            if i == passes - 1 {
                rs(6, 1);
                rs(0xA, 5);
                rs(0xB, 6);
            }
            rw_camera_end_update(rd::<GPtr<u8>>(this, 0));
        }
        i += 1;
    }
    wr::<u32>(rd::<GPtr<u8>>(this, 0), 0x60, orig_fb);
    src
}

/// `CShadowCamera::DrawBlurryRaster2(RwRaster* raster)` (0x706330, thiscall, `ret 4`): the camera's
/// frame buffer is set to `raster`; inside `RwCameraBeginUpdate`: (10,1), (11,3), (6,0), (9,2),
/// (1, original frame buffer), `Im2DRenderQuad(0, 0, w, w, nearZ, 1/nearPlane, 0)` (w = raster width,
/// both axes), (6,1), (10,5), (11,6), `RwCameraEndUpdate`; the frame buffer is restored. Returns
/// `raster`. The original keeps 1/nearPlane in its own argument slot (stack residue).
#[hook(addr = 0x706330, conv = thiscall, name = "CShadowCamera::DrawBlurryRaster2", this = "CShadowCamera", expect = "51 53 8B 5C 24 0C",
       unverified = "mvpd-render-1", route = "L2", hazards = "heap-writes,stack-residue",
       note = "overwrites its argument slot with 1/nearPlane (f32); switches the RwCamera frame buffer")]
pub fn draw_blurry_raster2(this: GPtr<u8>, raster: GPtr<u8>) -> GPtr<u8> {
    let w = X87::from_i32(rd::<i32>(raster, 0xC)).to_f32().to_bits();
    let cam = rd::<GPtr<u8>>(this, 0);
    let orig_fb = rd::<u32>(cam, 0x60);
    let r = x(rd::<f32>(cam, 0x80)).fdivr_m(k(K_ONE)).to_f32().to_bits();
    wr::<u32>(cam, 0x60, raster.addr() as u32);
    if !rw_camera_begin_update(rd::<GPtr<u8>>(this, 0)).is_null() {
        rs(0xA, 1);
        rs(0xB, 3);
        rs(6, 0);
        rs(9, 2);
        rs(1, orig_fb);
        im2d_render_quad_o(0, 0, w, w, near_screen_z(), r, 0);
        rs(6, 1);
        rs(0xA, 5);
        rs(0xB, 6);
        rw_camera_end_update(rd::<GPtr<u8>>(this, 0));
    }
    wr::<u32>(rd::<GPtr<u8>>(this, 0), 0x60, orig_fb);
    raster
}

// ------------------------------------------------------------------ CRealTimeShadow ---------------

/// `CRealTimeShadow::Update()` (0x706600, thiscall): for an atomic (`m_nRwObjectType` +0x24 == 1) or
/// clump (2) owner object (`[[this]+0x18]`), `RwV3dTransformPoints(&m_vecCenter +0x3C,
/// &m_BoundingSphere.center +0x2C, 1, &frame->modelling)`; `m_Camera.SetCenter(+0x3C)`;
/// `m_Camera.Update(atomic | clump)`. The frame buffer raster then goes through
/// `m_BlurCamera.RasterResample` (+0x14) when `m_bBlurred` (+0x10), the manager's `m_BlurCamera`
/// (0xC40394) `RasterBlur(…, m_nBlurPasses +0x1C)` when passes > 0, and its `m_GradientCamera`
/// (0xC4039C) `DrawBlurryRaster2` when `m_bDrawMoreBlur` (+0x20). Returns `GetShadowRwTexture()`.
#[hook(addr = 0x706600, conv = thiscall, name = "CRealTimeShadow::Update", this = "CRealTimeShadow", expect = "56 8B F1 8B 46 24",
       unverified = "mvpd-render-1", route = "L2", hazards = "heap-writes",
       recs = "grove-walk", note = "renders into the shadow cameras (RW heap state)")]
pub fn rts_update(this: GPtr<u8>) -> GPtr<u8> {
    let ty = rd::<i32>(this, 0x24);
    if ty == 1 || ty == 2 {
        let obj = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x18);
        let frame = rd::<GPtr<u8>>(obj, 4);
        rw_v3d_transform_points(this.byte_add(0x3C), this.byte_add(0x2C), 1, frame.byte_add(0x10));
    }
    let camera = this.byte_add(8);
    set_center_o(camera, this.byte_add(0x3C));
    let ty = rd::<i32>(this, 0x24);
    if ty == 1 {
        shadow_camera_update_atomic(camera, rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x18));
    } else if ty == 2 {
        update_clump_o(camera, rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x18));
    }
    let mut r = rd::<GPtr<u8>>(rd::<GPtr<u8>>(camera, 0), 0x60);
    if rd::<u8>(this, 0x10) != 0 {
        r = raster_resample_o(this.byte_add(0x14), r);
    }
    let passes = rd::<i32>(this, 0x1C);
    if passes != 0 {
        r = raster_blur_o(g::<u8>(0xC40394), r, passes);
    }
    if rd::<u8>(this, 0x20) != 0 {
        draw_blurry_raster2_o(g::<u8>(0xC4039C), r);
    }
    if rd::<u8>(this, 0x10) != 0 {
        rd::<GPtr<u8>>(this, 0x18)
    } else {
        rd::<GPtr<u8>>(this, 0xC)
    }
}

/// `CRealTimeShadowManager::Update()` (0x706AB0, thiscall): `ReInit()` when initialised (+0) and
/// `m_bNeedsReinit` (+1), which is then cleared. For each of the 16 shadows (`m_apShadows` +4): with an
/// owner, the intensity (+5, signed byte) goes up by 3 (max 100) when `m_bKeepAlive` (+4), else down
/// by 3 (min 0). At 0 and initialised the owner's `m_pShadowData` (+0x134) and the owner are cleared;
/// otherwise `shadow->Update()` and `CShadows::StoreRealTimeShadow(owner, ...)` with the current
/// time-cycle shadow values (index `[0xB79FD0]`: 0xB79E90, 0xB79E50, 0xB79F90, 0xB79F50, 0xB79F10,
/// 0xB79ED0). Finally every shadow's keep-alive flag is cleared.
#[hook(addr = 0x706AB0, conv = thiscall, name = "CRealTimeShadowManager::Update", this = "CRealTimeShadowManager", expect = "51 53 57 8B F9",
       unverified = "mvpd-render-1", route = "L2", hazards = "heap-writes",
       recs = "grove-walk", note = "writes the CRealTimeShadow objects (heap) and entities' +0x134")]
pub fn rts_manager_update(this: GPtr<u8>) {
    if rd::<u8>(this, 0) != 0 && rd::<u8>(this, 1) != 0 {
        rts_manager_reinit(this);
        wr::<u8>(this, 1, 0);
    }
    for i in 0..16isize {
        let slot = this.byte_add(4 + i * 4);
        let s = slot.cast::<GPtr<u8>>().read();
        if rd::<u32>(s, 0) == 0 {
            continue;
        }
        let cur = i32::from(rd::<i8>(s, 5));
        let nv = if rd::<u8>(s, 4) != 0 {
            let v = cur + 3;
            if v < 100 { v } else { 100 }
        } else {
            let v = cur - 3;
            if v <= 0 { 0 } else { v }
        };
        wr::<u8>(s, 5, nv as u8);
        let s = slot.cast::<GPtr<u8>>().read();
        if rd::<u8>(s, 5) == 0 {
            if rd::<u8>(this, 0) != 0 {
                let owner = rd::<GPtr<u8>>(s, 0);
                wr::<u32>(owner, 0x134, 0);
                wr::<u32>(s, 0, 0);
            }
            continue;
        }
        rts_update_o(s);
        let t = g::<i32>(0xB79FD0).read() as isize;
        let tc = |base: usize| g::<u32>(base).byte_add(t * 4).read();
        let (a7, a6, a5, a4, a3, a2) = (tc(0xB79ED0), tc(0xB79F10), tc(0xB79F50), tc(0xB79F90), tc(0xB79E50), tc(0xB79E90));
        let owner = rd::<GPtr<u8>>(slot.cast::<GPtr<u8>>().read(), 0);
        store_real_time_shadow_o(owner, a2, a3, a4, a5, a6, a7);
    }
    for i in 0..16isize {
        let s = this.byte_add(4 + i * 4).cast::<GPtr<u8>>().read();
        wr::<u8>(s, 4, 0);
    }
}

// ------------------------------------------------------------------ CShadows ----------------------

/// `CShadows::StoreShadowToBeRendered(uint8 type, RwTexture*, const CVector* pos, float frontX,
/// frontY, sideX, sideY, int16 intensity, uint8 r, g, b, float zDistance, bool drawOnWater, float
/// scale, CRealTimeShadow*, bool drawOnBuildings)` (0x707390, cdecl): while fewer than 48 are stored
/// (`ShadowsStoredToBeRendered`, u16 0xC403DC), fills the next `asShadowsStored` entry (0x34 bytes
/// at 0xC40430): pos +0, front/side +0xC..+0x18, zDistance +0x1C, scale +0x20, texture +0x24,
/// real-time shadow +0x28, intensity +0x2C, type +0x2E, rgb +0x2F..+0x31, flags +0x32 (bit 0
/// drawOnWater, bit 2 drawOnBuildings, the other bits kept except bit 1, cleared); count + 1.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x707390, conv = cdecl, name = "CShadows::StoreShadowToBeRendered-Texture", expect = "56 90 90 E9 EC 9F CF FF",
       unverified = "mvpd-render-1", route = "L2", hazards = "entry-protection-read",
       recs = "grove-walk", note = "entry is a SecuROM fragment jump (`nop nop jmp 0x401384`)")]
pub fn store_shadow_to_be_rendered_tex(ty: u32, tex: GPtr<u8>, pos: GPtr<u8>, fx: u32, fy: u32, sx: u32, sy: u32, intensity: u32, r: u32, gg: u32, b: u32, zd: u32, water: u32, scale: u32, rts: GPtr<u8>, bld: u32) {
    let n = g::<u16>(0xC403DC).read();
    if n >= 0x30 {
        return;
    }
    let e = g::<u8>(0xC40430).byte_add(n as isize * 0x34);
    wr::<u8>(e, 0x2E, ty as u8);
    wr::<u32>(e, 0x24, tex.addr() as u32);
    wr::<u32>(e, 0, rd::<u32>(pos, 0));
    wr::<u32>(e, 4, rd::<u32>(pos, 4));
    wr::<u32>(e, 8, rd::<u32>(pos, 8));
    wr::<u32>(e, 0xC, fx);
    wr::<u32>(e, 0x10, fy);
    wr::<u32>(e, 0x14, sx);
    wr::<u32>(e, 0x18, sy);
    wr::<u16>(e, 0x2C, intensity as u16);
    wr::<u8>(e, 0x2F, r as u8);
    wr::<u8>(e, 0x30, gg as u8);
    wr::<u8>(e, 0x31, b as u8);
    wr::<u32>(e, 0x1C, zd);
    let fl = (((bld as u8) & 1) << 2) | (rd::<u8>(e, 0x32) & 0xFA) | ((water as u8) & 1);
    wr::<u8>(e, 0x32, fl);
    wr::<u32>(e, 0x20, scale);
    wr::<u32>(e, 0x28, rts.addr() as u32);
    g::<u16>(0xC403DC).write(n + 1);
}

/// `CShadows::StoreRealTimeShadow(CPhysical*, float a2..a7)` (0x707CA0, cdecl): with shadow data
/// (+0x134): the position (matrix +0x30 or placement +4; a ped's (type 3) bone 0 position instead)
/// must be strictly closer (squared 2D distance to the camera, `fcomp` of the unrounded sum against
/// `[0xC4B6B0]`) and, unless it is the player ped, `TheCamera.IsSphereVisible(pos, 2.0)`. Strength:
/// the time-cycle shadow strength (i16 0xB7C4E8), faded by `1 - (d - h) / h` beyond h = 15.0 (0x8D5240)
/// * 0.5; then `ftol(intensity (+5) * 0.01 * strength)`. Border colour (0,0,0,255) around the shadow
/// texture; `SetLightProperties(deg(atan2(-sun.x, -sun.y)), -deg(atan2(-sun.y, -sun.z)), true)` with
/// the current sun vector (0xB7CA50 + 12 * index); the light frame's `at` normalised goes to
/// `CalcPedShadowValues`, which writes its results into this function's own argument slots (a4, a5,
/// a6, a7, a2, a3). The shadow is stored as type 3 at `pos - (a2, a3) * 2.5` (x, y), with
/// front/side `a4..a7 * 1.5`, intensity and rgb = strength, z distance 4.0, scale 1.0.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x707CA0, conv = cdecl, name = "CShadows::StoreRealTimeShadow", expect = "83 EC 18 55 8B 6C 24 20",
       unverified = "mvpd-render-1", route = "L2", hazards = "x87-transcendental,stack-residue",
       recs = "grove-walk",
       note = "FSQRT and FPATAN; the original writes temporaries (distance, colour, CalcPedShadowValues outputs) into its own argument slots")]
pub fn store_real_time_shadow(e: GPtr<u8>, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32) {
    let rts = rd::<GPtr<u8>>(e, 0x134);
    if rts.is_null() {
        return;
    }
    let m = rd::<GPtr<u8>>(e, 0x14);
    let src = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) };
    let mut pos: [f32; 3] = src.cast::<[f32; 3]>().read();
    if rd::<u8>(e, 0x36) & 7 == 3 {
        ped_get_bone_position(e, lp(&mut pos), 0, 0);
    }
    let cm = g::<GPtr<u8>>(0xB6F03C).read();
    let cam = if cm.is_null() { g::<u8>(0xB6F02C) } else { cm.byte_add(0x30) };
    let dy = x(pos[1]).fsub_m(rd::<f32>(cam, 4));
    let dy2 = dy.fmul(x(pos[1]).fsub_m(rd::<f32>(cam, 4)));
    let dx = x(pos[0]).fsub_m(rd::<f32>(cam, 0));
    let dx2 = dx.fmul(x(pos[0]).fsub_m(rd::<f32>(cam, 0)));
    let d2 = dy2.fadd(dx2);
    let d2f = d2.to_f32();
    if jp(d2, x(g::<f32>(0xC4B6B0).read()), 5) {
        return;
    }
    if e != find_player_ped(-1) && camera_is_sphere_visible(g::<u8>(0xB6F028), lp(&mut pos), 0x4000_0000) == 0 {
        return;
    }
    let d = x(d2f).fsqrt();
    let h = x(g::<f32>(0x8D5240).read()).fmul_m(k(K_HALF_B));
    let strength: i16 = if !jp(d, h, 5) {
        g::<u32>(0xB7C4E8).read() as i16
    } else {
        let s = i32::from(g::<i16>(0xB7C4E8).read());
        let v = d.fsub(h).fdiv(h).fsubr_m(k(K_ONE)).fmul(X87::from_i32(s));
        v.ftol() as u32 as i16
    };
    let inten = i32::from(rd::<i8>(rts, 5));
    let st = X87::from_i32(inten).fmul_m(k(0x3C23_D70A)).fmul(X87::from_i32(i32::from(strength))).ftol() as u32;
    let tex = get_shadow_rw_texture_o(rts);
    let mut col: [u8; 4] = [0, 0, 0, 0xFF];
    draw_border_around_texture_o(rts, lp(&mut col));
    let t = g::<i32>(0xB79FD0).read() as isize;
    let sun = g::<f32>(0xB7CA50).byte_add(t * 12);
    let nx = x(sun.read()).fchs();
    let ny = x(sun.byte_add(4).read()).fchs();
    let nz = x(sun.byte_add(8).read()).fchs();
    let a_unused = ny.fpatan(nz).fmul_m(k(0x4265_2EE0)).fchs().to_f32().to_bits();
    let a_angle = nx.fpatan(ny).fmul_m(k(0x4265_2EE0)).to_f32().to_bits();
    let frame = set_light_properties_o(rts, a_angle, a_unused, 1);
    let mut dir: [u32; 3] = frame.byte_add(0x30).cast::<[u32; 3]>().read();
    vector_normalise(lp(&mut dir));
    let (mut s2, mut s3, mut s4, mut s5, mut s6, mut s7) = (a2, a3, a4, a5, a6, a7);
    calc_ped_shadow_values(dir[0], dir[1], dir[2], lp(&mut s4), lp(&mut s5), lp(&mut s6), lp(&mut s7), lp(&mut s2), lp(&mut s3));
    pos[0] = x(f32::from_bits(s2)).fmul_m(k(0x4020_0000)).fsubr_m(pos[0]).to_f32();
    pos[1] = x(f32::from_bits(s3)).fmul_m(k(0x4020_0000)).fsubr_m(pos[1]).to_f32();
    let sc = |v: u32| x(f32::from_bits(v)).fmul_m(k(0x3FC0_0000)).to_f32().to_bits();
    let (p4, p5, p6, p7) = (sc(s4), sc(s5), sc(s6), sc(s7));
    store_shadow_to_be_rendered_tex_o(3, tex, lp(&mut pos), p4, p5, p6, p7, st, st, st, st, 0x4080_0000, 0, K_ONE, rts, 0);
}

orig! {
    /// `CEntity::GetBoundRect()` (vtable slot 0x24, thiscall, `CRect` through a hidden pointer)
    fn v_get_bound_rect(e: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(this: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*) const` (0x54F1B0)
    fn simple_transform_update_matrix(this: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CShadows::CastShadowEntityXY(CEntity*, 18 dwords)` (0x7086B0)
    #[allow(clippy::too_many_arguments)]
    fn cast_shadow_entity_xy(e: GPtr<u8>, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32, a12: u32, a13: u32, a14: u32, a15: u32, a16: u32, a17: u32, a18: u32, a19: u32) = cdecl 0x7086B0 => unported;
    /// `CShadows::CastShadowEntityXYZ(CEntity*, CVector*, 13 dwords)` (0x70A040)
    #[allow(clippy::too_many_arguments)]
    fn cast_shadow_entity_xyz(e: GPtr<u8>, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32, a12: u32, a13: u32, a14: u32, a15: u32, a16: u32, a17: u32, a18: u32, a19: u32) = cdecl 0x70A040 => unported;
}

/// The entity filter shared by the two `Cast*ShadowSectorList` walks: not yet scanned this pass
/// (`m_nScanCode` +0x2C against `CWorld::ms_nCurrentScanCode` 0xB7CD78, then set), `bUsesCollision`
/// (+0x1C bit 0) set, +0x1E bit 0 clear, area (+0x2F) the current one (0xB72914) or 13.
fn sector_entity_ok(e: GPtr<u8>) -> bool {
    let sc = g::<u16>(0xB7CD78).read();
    if rd::<u16>(e, 0x2C) == sc {
        return false;
    }
    wr::<u16>(e, 0x2C, sc);
    if rd::<u8>(e, 0x1C) & 1 == 0 || rd::<u8>(e, 0x1E) & 1 != 0 {
        return false;
    }
    let area = rd::<u8>(e, 0x2F);
    u32::from(area) == g::<u32>(0xB72914).read() || area == 0xD
}

/// The bound-rect and height filter of the two walks: rect from `GetBoundRect`; `minX < right`,
/// `maxX > left`, `minY < bottom'` (+4), `maxY > top'` (+0xC) as the original compares them; then
/// `colModel.max.z + z > pos.z - zDistance` and `colModel.min.z + z < pos.z` (model info
/// `ms_modelInfoPtrs` 0xA9B0C8, col model +0x14, box min z +8, max z +0x14).
fn sector_bounds_ok(e: GPtr<u8>, a: &[u32; 20]) -> bool {
    let mut rect = [0f32; 4];
    v_get_bound_rect(e, lp(&mut rect));
    let f = |i: usize| f32::from_bits(a[i]);
    if jp(x(f(2)), x(rect[2]), 5) || jne_(x(f(4)), x(rect[0])) || jp(x(f(3)), x(rect[1]), 5) || jne_(x(f(5)), x(rect[3])) {
        return false;
    }
    let m = rd::<GPtr<u8>>(e, 0x14);
    let epos = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) };
    let mi = g::<GPtr<u8>>(0xA9B0C8).wrapping_index(i32::from(rd::<i16>(e, 0x22))).read();
    let col = rd::<GPtr<u8>>(mi, 0x14);
    let pos = GPtr::<u8>::from_addr(a[6] as usize);
    let lo = x(rd::<f32>(pos, 8)).fsub_m(f(15));
    let hi = x(rd::<f32>(col, 0x14)).fadd_m(rd::<f32>(epos, 8));
    if jne_(hi, lo) {
        return false;
    }
    let m = rd::<GPtr<u8>>(e, 0x14);
    let epos = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) };
    !jp(x(rd::<f32>(col, 8)).fadd_m(rd::<f32>(epos, 8)), x(rd::<f32>(pos, 8)), 5)
}

/// `fcom` + `test ah, 0x41` + `jne` taken (below or equal, or unordered).
fn jne_(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(0x41)
}

/// `CShadows::CastPlayerShadowSectorList(CPtrListSingleLink&, float minX, minY, maxX, maxY, CVector*
/// pos, 13 more dwords)` (0x70A470, cdecl): for each entity of the list passing the sector filter
/// whose matrix (allocated and updated when missing) has `at.z > 0.97` (0x872724) and whose bounds
/// pass, `CastShadowEntityXY(entity, a2..a19)`. The walk keeps the next node in its own first
/// argument slot.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x70A470, conv = cdecl, name = "CShadows::CastPlayerShadowSectorList<CPtrListSingleLink<CPhysical*>>", expect = "8B 44 24 04 8B 00",
       unverified = "mvpd-render-1", route = "L2", hazards = "stack-residue,heap-writes",
       recs = "grove-walk", note = "next-node pointer kept in its own argument slot; entity scan codes; may allocate entity matrices")]
pub fn cast_player_shadow_sector_list(list: GPtr<u8>, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32, a12: u32, a13: u32, a14: u32, a15: u32, a16: u32, a17: u32, a18: u32, a19: u32) {
    let a = [0, 0, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19];
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        if !sector_entity_ok(e) {
            continue;
        }
        if rd::<u32>(e, 0x14) == 0 {
            placeable_allocate_matrix(e);
            simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        if jne_(x(rd::<f32>(rd::<GPtr<u8>>(e, 0x14), 0x28)), x(k(0x3F78_51EC))) {
            continue;
        }
        if !sector_bounds_ok(e, &a) {
            continue;
        }
        cast_shadow_entity_xy(e, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19);
    }
}

/// `CShadows::CastRealTimeShadowSectorList(CPtrListSingleLink&, float minX, minY, maxX, maxY, CVector*
/// pos, 13 more dwords)` (0x70A7E0, cdecl): as `CastPlayerShadowSectorList` without the matrix and
/// upright tests, calling `CastShadowEntityXYZ(entity, pos, a7..a19)`.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x70A7E0, conv = cdecl, name = "CShadows::CastRealTimeShadowSectorList<CPtrListSingleLink<CPhysical*>>", expect = "8B 44 24 04 8B 00",
       unverified = "mvpd-render-1", route = "L2", hazards = "stack-residue,heap-writes",
       recs = "grove-walk", note = "next-node pointer kept in its own argument slot; entity scan codes")]
pub fn cast_real_time_shadow_sector_list(list: GPtr<u8>, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32, a12: u32, a13: u32, a14: u32, a15: u32, a16: u32, a17: u32, a18: u32, a19: u32) {
    let a = [0, 0, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19];
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        if !sector_entity_ok(e) || !sector_bounds_ok(e, &a) {
            continue;
        }
        cast_shadow_entity_xyz(e, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19);
    }
}

orig! {
    /// `CStaticShadow::Free()` (0x707670)
    fn static_shadow_free(this: GPtr<u8>) = thiscall 0x707670 => unported;
    /// `CShadows::GeneratePolysForStaticShadow(int16 index)` (0x70B730; the index pushed as a dword)
    fn generate_polys_for_static_shadow(i: u32) = cdecl 0x70B730 => unported;
    /// `CFireManager::StartFire(CVector pos, float size, uint8, CEntity*, uint32 time, int8 gens, uint8)`
    /// (0x539F00; the vector by value)
    #[allow(clippy::too_many_arguments)]
    fn fire_manager_start_fire(this: GPtr<u8>, px: u32, py: u32, pz: u32, size: u32, a: u32, creator: u32, time: u32, gens: u32, b: u32) -> GPtr<u8> = thiscall 0x539F00 => unported;
    /// `CShadows::StoreStaticShadow` (0x70BA00)
    #[allow(clippy::too_many_arguments)]
    fn store_static_shadow_o(id: u32, ty: u32, tex: u32, pos: GPtr<u8>, fx: u32, fy: u32, sx: u32, sy: u32, intensity: u32, r: u32, gg: u32, b: u32, zd: u32, scale: u32, dd: u32, temp: u32, up: u32) -> u8 = cdecl 0x70BA00 => crate::mvpd_r1::store_static_shadow;
}

/// `CShadows::aStaticShadows` (48 × 0x40 at 0xC4A030).
const STATIC_SHADOWS: usize = 0xC4A030;
/// `CShadows::aPermanentShadows` (48 × 0x38 at 0xC4AC30).
const PERM_SHADOWS: usize = 0xC4AC30;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;

/// `CShadows::StoreStaticShadow(uint32 id, uint8 type, RwTexture*, const CVector* pos, float frontX,
/// frontY, sideX, sideY, int16 intensity, uint8 r, g, b, float zDistance, float scale, float
/// drawDistance, bool temporary, float upDistance)` (0x70BA00, cdecl, bool in AL). With d² the squared
/// 2D distance to the camera (stored as f32 into the `pos` argument slot): beyond `drawDistance²`
/// (`dd² <= d²` or unordered) returns true unless `drawDistance == 0`; with `dd² > d²` and `dd != 0`
/// the intensity and colour fade by `1 - (sqrt(d²) - 0.75 dd) * (4 / dd)` once past `0.75 dd`
/// (results written back into the argument slots). An existing entry with this id and polys is
/// refreshed (texture, type, colour, intensity, z distance, scale, temporary, time, +0x3A = 1;
/// returns true) when |dx|,|dy| < upDistance — or, failing that, |dx|,|dy| < 0.05 and |dz| < 2.0 — and
/// front/side are equal; otherwise it is freed and rebuilt in place. A new entry takes the first slot
/// without polys (none: true), is filled and `GeneratePolysForStaticShadow(index)`; returns whether it
/// got polys.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x70BA00, conv = cdecl, name = "CShadows::StoreStaticShadow", expect = "E9 F5 64 CF FF 85 C0",
       unverified = "mvpd-render-1", route = "L2", hazards = "x87-transcendental,stack-residue,entry-protection-read,ax-al-return",
       recs = "grove-walk",
       note = "entry is a SecuROM fragment jump (reads TheCamera.m_pMat); FSQRT; writes d2 and the faded intensity/colours into its own argument slots; bool in AL")]
pub fn store_static_shadow(id: u32, ty: u32, tex: u32, pos: GPtr<u8>, fx: u32, fy: u32, sx: u32, sy: u32, intensity: u32, r: u32, gg: u32, b: u32, zd: u32, scale: u32, dd: u32, temp: u32, up: u32) -> u8 {
    let (mut intensity, mut r, mut gg, mut b) = (intensity, r, gg, b);
    let cm = g::<GPtr<u8>>(0xB6F03C).read();
    let cam = if cm.is_null() { g::<u8>(0xB6F02C) } else { cm.byte_add(0x30) };
    let dy = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(cam, 4));
    let dy2 = dy.fmul(x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(cam, 4)));
    let dx = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(cam, 0));
    let d2 = dy2.fadd(dx.fmul(x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(cam, 0)))).to_f32();
    let ddf = f32::from_bits(dd);
    let near = !x(ddf).fmul_m(ddf).fcom(x(d2)).test_any(0x41);
    let dd_zero = !x(ddf).fcom(X87::zero()).test_parity(0x44);
    if !near {
        if !dd_zero {
            return 1;
        }
    } else if !dd_zero {
        let d = x(d2).fsqrt();
        let h = x(ddf).fmul_m(k(0x3F40_0000));
        if jp(d, h, 5) {
            let f = d.fsub(h).fmul(x(k(0x4080_0000)).fdiv_m(ddf)).fsubr_m(k(K_ONE));
            intensity = X87::from_i32(i32::from(intensity as u16 as i16)).fmul(f).ftol() as u32;
            let nr = X87::from_i32(i32::from(r as u8)).fmul(f).ftol() as u8;
            r = (r & !0xFF) | u32::from(nr);
            let ng = X87::from_i32(i32::from(gg as u8)).fmul(f).ftol() as u8;
            gg = (gg & !0xFF) | u32::from(ng);
            let nb = X87::from_i32(i32::from(b as u8)).fmul(f).ftol() as u8;
            b = (b & !0xFF) | u32::from(nb);
        }
    }
    let ent = |i: i16| g::<u8>(STATIC_SHADOWS).byte_add(isize::from(i) * 0x40);
    let mut idx: i16 = 0;
    let mut found = false;
    while idx < 0x30 {
        let e = ent(idx);
        if rd::<u32>(e, 0) == id && rd::<u32>(e, 4) != 0 {
            found = true;
            break;
        }
        idx += 1;
    }
    if found {
        let e = ent(idx);
        let upf = f32::from_bits(up);
        let adx = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(e, 0xC)).fabs();
        let mut same_spot = false;
        if !jp(adx, x(upf), 5) {
            let ady = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(e, 0x10)).fabs();
            same_spot = !ady.fcom(x(upf)).test_parity(5);
        }
        if !same_spot {
            let k05 = x(k(0x3D4C_CCCD));
            let ok = !jp(adx, k05, 5)
                && !jp(x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(e, 0x10)).fabs(), k05, 5)
                && !jp(x(rd::<f32>(pos, 8)).fsub_m(rd::<f32>(e, 0x14)).fabs(), x(k(0x4000_0000)), 5)
                && !jp(x(f32::from_bits(fx)), x(rd::<f32>(e, 0x18)), 0x44)
                && !jp(x(f32::from_bits(fy)), x(rd::<f32>(e, 0x1C)), 0x44)
                && !jp(x(f32::from_bits(sx)), x(rd::<f32>(e, 0x20)), 0x44)
                && !jp(x(f32::from_bits(sy)), x(rd::<f32>(e, 0x24)), 0x44);
            if !ok {
                static_shadow_free(e);
                return store_new_static(idx, id, ty, tex, pos, fx, fy, sx, sy, intensity, r, gg, b, zd, scale, temp);
            }
        }
        wr::<u32>(e, 0x30, tex);
        wr::<u8>(e, 0x36, ty as u8);
        wr::<u8>(e, 0x37, r as u8);
        wr::<u16>(e, 0x34, intensity as u16);
        wr::<u8>(e, 0x39, b as u8);
        wr::<u8>(e, 0x38, gg as u8);
        wr::<u32>(e, 0x2C, scale);
        wr::<u32>(e, 0x28, zd);
        wr::<u32>(e, 8, g::<u32>(TIME_MS).read());
        wr::<u8>(e, 0x3A, 1);
        wr::<u8>(e, 0x3C, temp as u8);
        return 1;
    }
    idx = 0;
    while idx < 0x30 && rd::<u32>(ent(idx), 4) != 0 {
        idx += 1;
    }
    if idx == 0x30 {
        return 1;
    }
    store_new_static(idx, id, ty, tex, pos, fx, fy, sx, sy, intensity, r, gg, b, zd, scale, temp)
}

/// The fill-and-generate tail of `StoreStaticShadow` (0x70BB71).
#[allow(clippy::too_many_arguments)]
fn store_new_static(idx: i16, id: u32, ty: u32, tex: u32, pos: GPtr<u8>, fx: u32, fy: u32, sx: u32, sy: u32, intensity: u32, r: u32, gg: u32, b: u32, zd: u32, scale: u32, temp: u32) -> u8 {
    let e = g::<u8>(STATIC_SHADOWS).byte_add(isize::from(idx) * 0x40);
    wr::<u8>(e, 0x36, ty as u8);
    wr::<u32>(e, 0x30, tex);
    wr::<u16>(e, 0x34, intensity as u16);
    wr::<u8>(e, 0x37, r as u8);
    wr::<u8>(e, 0x38, gg as u8);
    wr::<u8>(e, 0x39, b as u8);
    wr::<u32>(e, 0x28, zd);
    wr::<u32>(e, 0x2C, scale);
    wr::<u32>(e, 0, id);
    wr::<u32>(e, 0xC, rd::<u32>(pos, 0));
    wr::<u32>(e, 0x10, rd::<u32>(pos, 4));
    wr::<u32>(e, 0x14, rd::<u32>(pos, 8));
    wr::<u32>(e, 0x18, fx);
    wr::<u32>(e, 0x1C, fy);
    wr::<u32>(e, 0x20, sx);
    wr::<u32>(e, 0x24, sy);
    wr::<u8>(e, 0x3A, 1);
    wr::<u8>(e, 0x3C, temp as u8);
    wr::<u32>(e, 8, g::<u32>(TIME_MS).read());
    generate_polys_for_static_shadow(idx as u16 as u32);
    u8::from(rd::<u32>(e, 4) != 0)
}

/// `fild dword` of an unsigned value as MSVC does it: `fild`, plus 2^32 (0x858C54) when negative.
fn fild_u32(v: u32) -> X87 {
    let r = X87::from_i32(v as i32);
    if (v as i32) < 0 { r.fadd_m(k(0x4F80_0000)) } else { r }
}

/// `CShadows::UpdatePermanentShadows()` (0x70C950): every permanent shadow with a type (+0x32): expired
/// (`now - created (+0x24) >= lifetime (+0x28)`, unsigned) → type 0; else re-stored with
/// `StoreStaticShadow(id = its address, type, texture +0x2C, pos +0, front/side +0xC..+0x18,
/// intensity +0x30, rgb +0x33..+0x35, zDistance +0x1C, 1.0, 40.0, false, 0)` — in the last quarter of
/// its life the intensity (signed) and colours scaled by `1 - (elapsed - 3/4 life) / (life / 4)`
/// (unsigned conversions) — and dropped (type 0) when that fails, unless type 8. Every fourth frame
/// (`CTimer::m_FrameCounter & 3 == 0`) each type-5 shadow becomes type 6 (now, 2000 ms) and sets fire
/// (`gFireManager.StartFire(pos, 1.8, 0, NULL, 2000, 0, 1)`, the target becomes type 7) to the nearest
/// type 4/8 shadow closer than 3.0, then to the nearest one on the opposite side (dot of the 2D
/// directions < 0); finally type 7 → 5.
#[hook(addr = 0x70C950, conv = cdecl, name = "CShadows::UpdatePermanentShadows", expect = "83 EC 08 53 55",
       route = "L2", hazards = "x87-transcendental",
       recs = "grove-walk", note = "FSQRT distances (gasoline trail fire spread)")]
pub fn update_permanent_shadows() {
    for i in 0..48isize {
        let b = g::<u8>(PERM_SHADOWS).byte_add(i * 0x38);
        let ty = rd::<u8>(b, 0x32);
        if ty == 0 {
            continue;
        }
        let elapsed = g::<u32>(TIME_MS).read().wrapping_sub(rd::<u32>(b, 0x24));
        let life = rd::<u32>(b, 0x28);
        if elapsed >= life {
            wr::<u8>(b, 0x32, 0);
            continue;
        }
        let q3 = life.wrapping_mul(3) >> 2;
        let (inten, r, gg, bb) = if elapsed < q3 {
            (u32::from(rd::<u16>(b, 0x30)), u32::from(rd::<u8>(b, 0x33)), u32::from(rd::<u8>(b, 0x34)), u32::from(rd::<u8>(b, 0x35)))
        } else {
            let f = fild_u32(elapsed - q3).fdiv(fild_u32(life >> 2)).fsubr_m(k(K_ONE));
            let nb = X87::from_i32(i32::from(rd::<u8>(b, 0x35))).fmul(f).ftol() as u32;
            let ng = X87::from_i32(i32::from(rd::<u8>(b, 0x34))).fmul(f).ftol() as u32;
            let nr = X87::from_i32(i32::from(rd::<u8>(b, 0x33))).fmul(f).ftol() as u32;
            let ni = X87::from_i32(i32::from(rd::<i16>(b, 0x30))).fmul(f).ftol() as u32;
            (ni, nr, ng, nb)
        };
        let ok = store_static_shadow_o(
            b.addr() as u32, u32::from(ty), rd::<u32>(b, 0x2C), b, rd::<u32>(b, 0xC), rd::<u32>(b, 0x10), rd::<u32>(b, 0x14), rd::<u32>(b, 0x18),
            inten, r, gg, bb, rd::<u32>(b, 0x1C), K_ONE, 0x4220_0000, 0, 0,
        );
        if ok == 0 && rd::<u8>(b, 0x32) != 8 {
            wr::<u8>(b, 0x32, 0);
        }
    }
    if g::<u32>(0xB7CB4C).read() & 3 != 0 {
        return;
    }
    let ent = |i: isize| g::<u8>(PERM_SHADOWS).byte_add(i * 0x38);
    let dist = |a: GPtr<u8>, c: GPtr<u8>| {
        let dx = x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(c, 0));
        let dy = x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(c, 4));
        let dz = x(rd::<f32>(a, 8)).fsub_m(rd::<f32>(c, 8));
        dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt()
    };
    let fire = |j: isize| {
        let p = ent(j);
        wr::<u8>(p, 0x32, 7);
        fire_manager_start_fire(g::<u8>(0xB71F80), rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8), 0x3FE6_6666, 0, 0, 2000, 0, 1);
    };
    for j in 0..48isize {
        let bj = ent(j);
        if rd::<u8>(bj, 0x32) != 5 {
            continue;
        }
        wr::<u8>(bj, 0x32, 6);
        wr::<u32>(bj, 0x24, g::<u32>(TIME_MS).read());
        wr::<u32>(bj, 0x28, 2000);
        let mut best = k(0x4040_0000);
        let mut sel: isize = -1;
        for kk in 0..48isize {
            let bk = ent(kk);
            let t = rd::<u8>(bk, 0x32);
            if t != 4 && t != 8 {
                continue;
            }
            let d = dist(bj, bk);
            if !jp(d, x(best), 5) {
                best = d.to_f32();
                sel = kk;
            }
        }
        if sel < 0 {
            continue;
        }
        let bs = ent(sel);
        fire(sel);
        let ddx = x(rd::<f32>(bs, 0)).fsub_m(rd::<f32>(bj, 0));
        let ddy = x(rd::<f32>(bs, 4)).fsub_m(rd::<f32>(bj, 4));
        let mut best = x(k(0x4040_0000));
        let mut sel: isize = -1;
        for kk in 0..48isize {
            let bk = ent(kk);
            let t = rd::<u8>(bk, 0x32);
            if t != 4 && t != 8 {
                continue;
            }
            let d = dist(bj, bk);
            let df = d.to_f32();
            if jp(d, best, 5) {
                continue;
            }
            let dot = x(rd::<f32>(bk, 4)).fsub_m(rd::<f32>(bj, 4)).fmul(ddy).fadd(x(rd::<f32>(bk, 0)).fsub_m(rd::<f32>(bj, 0)).fmul(ddx));
            if jp(dot, X87::zero(), 5) {
                continue;
            }
            sel = kk;
            best = x(df);
        }
        if sel >= 0 {
            fire(sel);
        }
    }
    for j in 0..48isize {
        if rd::<u8>(ent(j), 0x32) == 7 {
            wr::<u8>(ent(j), 0x32, 5);
        }
    }
}

orig! {
    /// CRT `floor(double)` (0x8219F0, result in ST0)
    fn crt_floor(v: f64) -> f64 = cdecl 0x8219F0 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470)
    fn world_clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CShadows::CastPlayerShadowSectorList` (0x70A470)
    #[allow(clippy::too_many_arguments)]
    fn cast_player_shadow_sector_list_o(list: GPtr<u8>, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32, a12: u32, a13: u32, a14: u32, a15: u32, a16: u32, a17: u32, a18: u32, a19: u32) = cdecl 0x70A470 => crate::mvpd_r1::cast_player_shadow_sector_list;
    /// `FindPlayerVehicle(int32 id, bool includeRemote)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
}

/// One sector bound of `GeneratePolysForStaticShadow`: `v * 0.02 + 60` (f32 kept, the original stores
/// it into its own argument slot), `ftol(floor(v as double))`; when that passes `test` (> 0 for the
/// minimum, < 119 for the maximum) the bound is recomputed from the stored f32.
fn sector_bound(v: f32, is_max: bool) -> i32 {
    let s = x(v).fmul_m(k(0x3CA3_D70A)).fadd_m(k(0x4270_0000));
    let first = X87::from_f64_exact(crt_floor(s.to_f64())).ftol() as u32 as i32;
    let pass = if is_max { first < 0x77 } else { first > 0 };
    if pass {
        X87::from_f64_exact(crt_floor(f64::from(s.to_f32()))).ftol() as u32 as i32
    } else if is_max {
        0x77
    } else {
        0
    }
}

/// `CShadows::GeneratePolysForStaticShadow(int16 index)` (0x70B730, cdecl): the static shadow's
/// rectangle is `pos ± (|frontX| + |sideX|, |frontY| + |sideY|)` (rounded to f32); its sector range
/// (`floor(v / 50 + 60)` through `sector_bound`, clamped to 0..119) gets a new scan code
/// (`CWorld::ClearScanCodes` and 1 on wrap), and every sector's building list
/// (`CWorld::ms_aSectors` 0xB7D0B8 + 8 * (y * 120 + x)) goes to `CastPlayerShadowSectorList(list,
/// minX, minY, maxX, maxY, &pos, front/side, 0, 0, 0, 0, zDistance, scale, &polys (+4), +0x3D, 0)`.
#[hook(addr = 0x70B730, conv = cdecl, name = "CShadows::GeneratePolysForStaticShadow", expect = "83 EC 2C 53 55",
       unverified = "mvpd-render-1", route = "L2", hazards = "stack-residue",
       recs = "grove-walk", note = "keeps sector temporaries in its own argument slot")]
pub fn generate_polys_for_static_shadow_port(index: u32) {
    let e = g::<u8>(STATIC_SHADOWS).byte_add(isize::from(index as u16 as i16) * 0x40);
    let mut pos: [u32; 3] = e.byte_add(0xC).cast::<[u32; 3]>().read();
    let ax = x(rd::<f32>(e, 0x20)).fabs().fadd(x(rd::<f32>(e, 0x18)).fabs());
    let ay = x(rd::<f32>(e, 0x24)).fabs().fadd(x(rd::<f32>(e, 0x1C)).fabs());
    let px = f32::from_bits(pos[0]);
    let py = f32::from_bits(pos[1]);
    let min_x = x(px).fsub(ax).to_f32();
    let max_x = x(px).fadd(ax).to_f32();
    let min_y = x(py).fsub(ay).to_f32();
    let max_y = x(py).fadd(ay).to_f32();
    let x0 = sector_bound(min_x, false);
    let y0 = sector_bound(min_y, false);
    let x1 = sector_bound(max_x, true);
    let y1 = sector_bound(max_y, true);
    let sc = g::<u16>(0xB7CD78);
    if sc.read() == 0xFFFF {
        world_clear_scan_codes();
        sc.write(1);
    } else {
        sc.write(sc.read().wrapping_add(1));
    }
    let mut y = y0;
    while y <= y1 {
        let cy = if y <= 0 { 0 } else { y.min(0x77) };
        let mut xx = x0;
        while xx <= x1 {
            let cx = if xx <= 0 { 0 } else { xx.min(0x77) };
            let list = g::<u8>(0xB7D0B8).byte_add(((cy * 0x78 + cx) * 8) as isize);
            cast_player_shadow_sector_list_o(
                list, min_x.to_bits(), min_y.to_bits(), max_x.to_bits(), max_y.to_bits(), lp(&mut pos).addr() as u32,
                rd::<u32>(e, 0x18), rd::<u32>(e, 0x1C), rd::<u32>(e, 0x20), rd::<u32>(e, 0x24), 0, 0, 0, 0,
                rd::<u32>(e, 0x28), rd::<u32>(e, 0x2C), e.byte_add(4).addr() as u32, e.byte_add(0x3D).addr() as u32, 0,
            );
            xx += 1;
        }
        y += 1;
    }
}

/// `CShadows::StoreCarLightShadow(CVehicle*, int32 id, RwTexture*, CVector* pos, float frontX, frontY,
/// sideX, sideY, uint8 r, g, b, float maxViewAngle)` (0x70C500, cdecl). `forced` = the active camera's
/// mode is 1 or 0x25, or [0xB5F851]. Squared 2D camera distance (kept in the `pos` argument slot):
/// beyond 729 only when forced; unforced, `(pos - cam) · camera front (0xB6F104/0xB6F108)` must
/// exceed `-maxViewAngle`. Unforced and at least 20.25 away the colours fade by
/// `1 - (dist - 18) / 9` (written back into the argument slots). A vehicle slower than 0.4 per frame
/// (`|moveSpeed| * timeStep`) that is not the player's gets a static shadow (`StoreStaticShadow(veh +
/// id, 2, tex, pos, …, 0x80, r, g, b, 6.0, 1.0, 0, false, 0.4)`), otherwise a per-frame one
/// (`StoreShadowToBeRendered(2, tex, pos, …, 0x80, r, g, b, 6.0, false, 1.0, NULL, veh == player's)`).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x70C500, conv = cdecl, name = "CShadows::StoreCarLightShadow", expect = "8B 0D 3C F0 B6 00",
       unverified = "mvpd-render-1", route = "L2", hazards = "x87-transcendental,stack-residue",
       recs = "driving", note = "FSQRT; distance and faded colours written into its own argument slots")]
pub fn store_car_light_shadow(veh: GPtr<u8>, id: u32, tex: u32, pos: GPtr<u8>, fx: u32, fy: u32, sx: u32, sy: u32, r: u32, gg: u32, b: u32, max_angle: u32) {
    let (mut r, mut gg, mut b) = (r, gg, b);
    let cm = g::<GPtr<u8>>(0xB6F03C).read();
    let cam = if cm.is_null() { g::<u8>(0xB6F02C) } else { cm.byte_add(0x30) };
    let (cx, cy) = (rd::<f32>(cam, 0), rd::<f32>(cam, 4));
    let dy = x(rd::<f32>(pos, 4)).fsub_m(cy);
    let dx = x(rd::<f32>(pos, 0)).fsub_m(cx);
    let d2 = dx.fmul(dx).fadd(dy.fmul(dy)).to_f32();
    let active = u32::from(g::<u8>(0xB6F081).read());
    let mode = g::<u16>(0xB6F1A8).byte_add((active * 0x238) as isize).read();
    let forced = mode == 1 || mode == 0x25 || g::<u8>(0xB5F851).read() != 0;
    let near = !x(d2).fcom(x(k(0x4436_4000))).test_parity(5);
    if !near && !forced {
        return;
    }
    if near && !forced {
        let cm = g::<GPtr<u8>>(0xB6F03C).read();
        let cam = if cm.is_null() { g::<u8>(0xB6F02C) } else { cm.byte_add(0x30) };
        let dot = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(cam, 4)).fmul_m(g::<f32>(0xB6F108).read())
            .fadd(x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(cam, 0)).fmul_m(g::<f32>(0xB6F104).read()));
        if jp(x(f32::from_bits(max_angle)).fchs(), dot, 5) {
            return;
        }
    }
    let d = x(d2).fsqrt().to_f32();
    let close = !x(d).fcom(x(k(0x41A2_0000))).test_parity(5);
    if !close && !forced {
        let f = x(d).fsub_m(k(0x4190_0000)).fmul_m(k(0x3DE3_8E39)).fsubr_m(k(K_ONE));
        let sc = |v: u32| (v & !0xFF) | u32::from(X87::from_i32(i32::from(v as u8)).fmul(f).ftol() as u8);
        r = sc(r);
        gg = sc(gg);
        b = sc(b);
    }
    let sp = x(rd::<f32>(veh, 0x44));
    let sp = sp.fmul(sp).fadd(x(rd::<f32>(veh, 0x48)).fmul(x(rd::<f32>(veh, 0x48)))).fadd(x(rd::<f32>(veh, 0x4C)).fmul(x(rd::<f32>(veh, 0x4C)))).fsqrt();
    let slow = sp.fmul_m(g::<f32>(0xB7CB5C).read()).fcom(x(k(0x3ECC_CCCD))).test_any(0x41);
    if slow && veh != find_player_vehicle(-1, 0) {
        store_static_shadow_o(veh.addr() as u32 + id, 2, tex, pos, fx, fy, sx, sy, 0x80, r, gg, b, 0x40C0_0000, K_ONE, 0, 0, 0x3ECC_CCCD);
        return;
    }
    let mine = u32::from(veh == find_player_vehicle(-1, 0));
    store_shadow_to_be_rendered_tex_o(2, GPtr::from_addr(tex as usize), pos, fx, fy, sx, sy, 0x80, r, gg, b, 0x40C0_0000, 0, K_ONE, GPtr::NULL, mine);
}

orig! {
    /// `CShadows::SetRenderModeForShadowType(uint8)` (0x707460; pushed as a dword)
    fn set_render_mode_for_shadow_type(ty: u32) = cdecl 0x707460 => unported;
    /// `CShadows::AffectColourWithLighting(type, lightByte, r, g, b, uint8* rOut, gOut, bOut)` (0x707850)
    #[allow(clippy::too_many_arguments)]
    fn affect_colour_with_lighting(ty: u32, light: u32, r: u32, gg: u32, b: u32, ro: GPtr<u8>, go: GPtr<u8>, bo: GPtr<u8>) = cdecl 0x707850 => unported;
    /// `RenderBuffer::StopStoring()` (0x7077A0)
    fn render_buffer_stop_storing() = cdecl 0x7077A0 => unported;
    /// `RwIm3DTransform(verts, n, ltm, flags)` (0x7EF450)
    fn rw_im3d_transform(verts: GPtr<u8>, n: u32, ltm: u32, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(type, indices, n)` (0x7EF550)
    fn rw_im3d_render_indexed_primitive(ty: u32, idx: GPtr<u8>, n: u32) -> i32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    fn rw_im3d_end() -> i32 = cdecl 0x7EF520 => unported;
}

/// `TempBufferVerticesStored` / `TempBufferIndicesStored` / index list / `TempVertexBuffer`.
const TB_VERTS: usize = 0xC4B950;
const TB_INDICES: usize = 0xC4B954;
const TB_INDEX_LIST: usize = 0xC4B958;
const TB_VERTEX_BUF: usize = 0xC4D958;

/// The inlined `RenderBuffer::RenderStuffInBuffer`: with stored vertices, `RwIm3DTransform(buf, n, NULL,
/// 1)` and on success `RwIm3DRenderIndexedPrimitive(TRILIST, indices, ni)` + `RwIm3DEnd`; both counts 0.
fn flush_temp_buffer() {
    let nv = g::<u32>(TB_VERTS).read();
    if nv != 0 && !rw_im3d_transform(g(TB_VERTEX_BUF), nv, 0, 1).is_null() {
        rw_im3d_render_indexed_primitive(3, g(TB_INDEX_LIST), g::<u32>(TB_INDICES).read());
        rw_im3d_end();
    }
    g::<u32>(TB_INDICES).write(0);
    g::<u32>(TB_VERTS).write(0);
}

/// `CShadows::RenderStaticShadows()` (0x708300): render states (8,0), (6,1), (12,1), (14,0), (30,0), (9,2)
/// with the temp buffer emptied; every static shadow's "rendered" byte (+0x3B) cleared. For each shadow
/// with polys not yet rendered: `SetRenderModeForShadowType(type)`, its texture's raster, then every later
/// (or same) shadow of that type and texture: per poly bunch, `AffectColourWithLighting(type, +0x3D, rgb,
/// &out)`, the inlined `RenderBuffer::StartStoring(3(n-2), n)` (flushing at 4096 indices / 2048 vertices),
/// the n vertices (bunch position + 0.06 in z, colour `alpha << 24 | out rgb` with alpha =
/// `ftol((1 - [0xC81300] * 0.5) * intensity)`, u/v bytes * 0.005), the indices from 0xC403A8,
/// `RenderBuffer::StopStoring()`; that shadow's +0x3B = 1. A flush closes each group. Finally (12,0),
/// (8,1).
#[hook(addr = 0x708300, conv = cdecl, name = "CShadows::RenderStaticShadows", expect = "A1 24 7B C9 00",
       route = "L2", hazards = "uninit-locals",
       recs = "grove-walk", note = "AffectColourWithLighting may leave the colour locals unwritten")]
pub fn render_static_shadows() {
    g::<u32>(TB_INDICES).write(0);
    g::<u32>(TB_VERTS).write(0);
    rs(8, 0);
    rs(6, 1);
    rs(0xC, 1);
    rs(0xE, 0);
    rs(0x1E, 0);
    rs(9, 2);
    for i in 0..48isize {
        wr::<u8>(g::<u8>(STATIC_SHADOWS + 0x3B), i * 0x40, 0);
    }
    let mut out: [u8; 4] = [0; 4];
    for i in 0..48isize {
        let ei = g::<u8>(STATIC_SHADOWS).byte_add(i * 0x40);
        if rd::<u32>(ei, 4) == 0 || rd::<u8>(ei, 0x3B) != 0 {
            continue;
        }
        set_render_mode_for_shadow_type(u32::from(rd::<u8>(ei, 0x36)));
        rs(1, rd::<u32>(rd::<GPtr<u8>>(ei, 0x30), 0));
        for j in i..48isize {
            let ej = g::<u8>(STATIC_SHADOWS).byte_add(j * 0x40);
            let mut pb = rd::<GPtr<u8>>(ej, 4);
            if pb.is_null() || rd::<u8>(ej, 0x36) != rd::<u8>(ei, 0x36) || rd::<u32>(ej, 0x30) != rd::<u32>(ei, 0x30) {
                continue;
            }
            while !pb.is_null() {
                let o = lp(&mut out);
                affect_colour_with_lighting(
                    u32::from(rd::<u8>(ej, 0x36)), u32::from(rd::<u8>(ej, 0x3D)), u32::from(rd::<u8>(ej, 0x37)),
                    u32::from(rd::<u8>(ej, 0x38)), u32::from(rd::<u8>(ej, 0x39)), o.byte_add(1), o.byte_add(2), o.byte_add(3),
                );
                let n = i32::from(rd::<i16>(pb, 0x58));
                let ni = (n * 3 - 6) as u32;
                if g::<u32>(TB_INDICES).read().wrapping_add(ni) as i32 >= 0x1000 {
                    flush_temp_buffer();
                }
                if (g::<u32>(TB_VERTS).read().wrapping_add(n as u32) as i32) >= 0x800 {
                    flush_temp_buffer();
                }
                let alpha = x(g::<f32>(0xC81300).read()).fmul_m(k(K_HALF_B)).fsubr_m(k(K_ONE))
                    .fmul(X87::from_i32(i32::from(rd::<i16>(ej, 0x34)))).ftol() as u32;
                let vbase = g::<u8>(TB_VERTEX_BUF).byte_add(g::<u32>(TB_VERTS).read() as isize * 0x24);
                let ibase = g::<u16>(TB_INDEX_LIST).byte_add(g::<u32>(TB_INDICES).read() as isize * 2);
                g::<u32>(0xC40410).write(ni);
                g::<u32>(0xC40414).write(n as u32);
                if rd::<i16>(pb, 0x58) > 0 {
                    let col = (((((alpha << 8) | u32::from(out[1])) << 8) | u32::from(out[2])) << 8) | u32::from(out[3]);
                    let mut kk = 0i32;
                    loop {
                        let v = vbase.byte_add(kk as isize * 0x24);
                        wr::<u32>(v, 0x18, col);
                        let u = X87::from_i32(i32::from(rd::<u8>(pb, 0x5A + kk as isize))).fmul_m(k(0x3BA3_D70A));
                        wr::<f32>(v, 0x1C, u.to_f32());
                        let vv = X87::from_i32(i32::from(rd::<u8>(pb, 0x61 + kk as isize))).fmul_m(k(0x3BA3_D70A));
                        wr::<f32>(v, 0x20, vv.to_f32());
                        let p = pb.byte_add(kk as isize * 12);
                        wr::<u32>(v, 0, rd::<u32>(p, 0));
                        wr::<u32>(v, 4, rd::<u32>(p, 4));
                        wr::<f32>(v, 8, x(rd::<f32>(p, 8)).fadd_m(k(0x3D75_C28F)).to_f32());
                        kk += 1;
                        if kk >= i32::from(rd::<i16>(pb, 0x58)) {
                            break;
                        }
                    }
                }
                let mut m = 0i32;
                while m < (i32::from(rd::<i16>(pb, 0x58)) - 2) * 3 {
                    ibase.wrapping_index(m).write(g::<u16>(0xC403A8).wrapping_index(m).read());
                    m += 1;
                }
                render_buffer_stop_storing();
                pb = rd::<GPtr<u8>>(pb, 0x54);
            }
            wr::<u8>(ej, 0x3B, 1);
        }
        flush_temp_buffer();
    }
    rs(0xC, 0);
    rs(8, 1);
}

orig! {
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn get_vehicle_appearance(v: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `FindPlayerCoors(CVector* out, int32)` (0x56E010)
    fn find_player_coors(out: GPtr<u8>, id: i32) -> GPtr<u8> = cdecl 0x56E010 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): out = a - b
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = cdecl 0x40FE60 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CVector::operator*=(float)` (0x40FEF0)
    fn vector_mul_assign(v: GPtr<u8>, s: u32) = thiscall 0x40FEF0 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
}

/// `CShadows::RenderExtraPlayerShadows()` (0x707FA0): with a non-zero time-cycle light shadow
/// strength (i16 0xB7C4EA) and a player vehicle that is not the RC bandit (0x1B9), not a bike
/// (appearance 2) and not of subclass (+0x590) 5, 8 or 9: for each point light (`CPointLights::
/// NumLights` 0xC3F0D0, 0x30 bytes at 0xC3F0E0) of type 0 with extra shadows (+0x2E) and a non-zero
/// colour, closer to the player than its radius (+0x18): intensity = `strength * 5 / 8` (signed
/// division), faded by `1 - (2 dist - r) / r` beyond r/2; the direction light → player is scaled by
/// 1/dist and the vehicle position pushed 1.2 along it (x, y), then by `-(lenY/2 - colModel max.y)` along
/// the forward vector; a type-1 shadow with texture [0xC403E0], front = lenY/2 * forward.xy, side =
/// ±lenX/2 * right.xy (negated when the up vector's z <= 0), rgb 0, z distance 4.5, scale 1.0.
#[hook(addr = 0x707FA0, conv = cdecl, name = "CShadows::RenderExtraPlayerShadows", expect = "83 EC 30 66 83 3D EA C4 B7 00 00",
       route = "L2", hazards = "none", recs = "driving")]
pub fn render_extra_player_shadows() {
    if g::<i16>(0xB7C4EA).read() == 0 {
        return;
    }
    let veh = find_player_vehicle(-1, 0);
    if veh.is_null() || rd::<u16>(veh, 0x22) == 0x1B9 || get_vehicle_appearance(veh) == 2 {
        return;
    }
    let sub = rd::<u32>(veh, 0x590);
    if sub == 9 || sub == 8 || sub == 5 {
        return;
    }
    if g::<i32>(0xC3F0D0).read() <= 0 {
        return;
    }
    let mut i = 0i32;
    while i < g::<i32>(0xC3F0D0).read() {
        let l = g::<u8>(0xC3F0E0).byte_add(i as isize * 0x30);
        i += 1;
        if rd::<u8>(l, 0x2C) != 0 || rd::<u8>(l, 0x2E) == 0 {
            continue;
        }
        let nz = |o: isize| jp(x(rd::<f32>(l, o)), X87::zero(), 0x44);
        if !(nz(0x1C) || nz(0x20) || nz(0x24)) {
            continue;
        }
        let mut tmp = [0u32; 3];
        let mut dir = [0f32; 3];
        let pc = find_player_coors(lp(&mut tmp), -1);
        vector_sub(lp(&mut dir), l, pc);
        let mag = X87::from_f64_exact(vector_magnitude(lp(&mut dir)));
        let radius = rd::<f32>(l, 0x18);
        if jp(mag, x(radius), 5) {
            continue;
        }
        let s5 = i32::from(g::<i16>(0xB7C4EA).read()) * 5;
        let s58 = (s5 + ((s5 >> 31) & 7)) >> 3;
        let inten = if !jp(mag, x(radius).fmul_m(k(K_HALF_B)), 5) {
            s58
        } else {
            mag.fadd(mag).fsub_m(radius).fdiv_m(radius).fsubr_m(k(K_ONE)).fmul(X87::from_i32(s58)).ftol() as u32 as i32
        };
        vector_mul_assign(lp(&mut dir), x(k(K_ONE)).fdiv(mag).to_f32().to_bits());
        let pv = find_player_vehicle(-1, 0);
        let m = rd::<GPtr<u8>>(pv, 0x14);
        let src = if m.is_null() { pv.byte_add(4) } else { m.byte_add(0x30) };
        let mut v: [f32; 3] = src.cast::<[f32; 3]>().read();
        v[0] = x(dir[0]).fmul_m(k(0x3F99_999A)).fsubr_m(v[0]).to_f32();
        v[1] = x(dir[1]).fmul_m(k(0x3F99_999A)).fsubr_m(v[1]).to_f32();
        let c1 = get_col_model(veh);
        let c2 = get_col_model(veh);
        let len_y = x(rd::<f32>(c1, 0x10)).fsub_m(rd::<f32>(c2, 4)).to_f32();
        let c3 = get_col_model(veh);
        let c4 = get_col_model(veh);
        let len_x = x(rd::<f32>(c3, 0xC)).fsub_m(rd::<f32>(c4, 0)).to_f32();
        let c5 = get_col_model(veh);
        let mat = rd::<GPtr<u8>>(veh, 0x14);
        let hy = x(len_y).fmul_m(k(K_HALF_B));
        let t = hy.fsub_m(rd::<f32>(c5, 0x10));
        v[0] = t.fmul_m(rd::<f32>(mat, 0x10)).fsubr_m(v[0]).to_f32();
        v[1] = t.fmul_m(rd::<f32>(mat, 0x14)).fsubr_m(v[1]).to_f32();
        let upright = !x(rd::<f32>(mat, 0x28)).fcom(X87::zero()).test_any(0x41);
        let hx = x(len_x).fmul_m(k(K_HALF_B));
        let (mut sy, mut sx) = (hx.fmul_m(rd::<f32>(mat, 4)), hx.fmul_m(rd::<f32>(mat, 0)));
        if !upright {
            sy = sy.fchs();
            sx = sx.fchs();
        }
        let fy = hy.fmul_m(rd::<f32>(mat, 0x14)).to_f32().to_bits();
        let fx = hy.fmul_m(rd::<f32>(mat, 0x10)).to_f32().to_bits();
        let tex = g::<GPtr<u8>>(0xC403E0).read();
        store_shadow_to_be_rendered_tex_o(1, tex, lp(&mut v), fx, fy, sx.to_f32().to_bits(), sy.to_f32().to_bits(), inten as u32, 0, 0, 0, 0x4090_0000, 0, K_ONE, GPtr::NULL, 0);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_RENDER_EXTRA_PLAYER_SHADOWS,
    HOOK_RENDER_STATIC_SHADOWS,
    HOOK_GENERATE_POLYS_FOR_STATIC_SHADOW_PORT,
    HOOK_STORE_CAR_LIGHT_SHADOW,
    HOOK_STORE_STATIC_SHADOW,
    HOOK_UPDATE_PERMANENT_SHADOWS,
    HOOK_CAST_PLAYER_SHADOW_SECTOR_LIST,
    HOOK_CAST_REAL_TIME_SHADOW_SECTOR_LIST,
    HOOK_SET_CENTER,
    HOOK_INVERT_RASTER,
    HOOK_DRAW_OUTLINE_BORDER,
    HOOK_SET_LIGHT_PROPERTIES,
    HOOK_GET_SHADOW_RW_TEXTURE,
    HOOK_DRAW_BORDER_AROUND_TEXTURE,
    HOOK_IM2D_RENDER_QUAD,
    HOOK_UPDATE_CLUMP,
    HOOK_RASTER_RESAMPLE,
    HOOK_RASTER_BLUR,
    HOOK_DRAW_BLURRY_RASTER2,
    HOOK_RTS_UPDATE,
    HOOK_RTS_MANAGER_UPDATE,
    HOOK_STORE_SHADOW_TO_BE_RENDERED_TEX,
    HOOK_STORE_REAL_TIME_SHADOW,
];
