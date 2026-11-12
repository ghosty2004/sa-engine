//! MVP-A writer batch `mvpa-render-3`: FX frustum test and immediate-mode flush, real-time shadow set-up,
//! the plant manager's per-camera update and the env-map material unpacker. Ported from the 1.0 US
//! bytes; gta-reversed as a reading aid. Every hook is unverified.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{Fpu, ftol32, g, k, rd, wr};

orig! {
    /// `CGrassRenderer::SetCurrentScanCode(uint16)` (0x5DABB0)
    pub fn grass_set_current_scan_code(code: u32) = cdecl 0x5DABB0 => unported;
    /// `CGrassRenderer::SetGlobalCameraPos(const CVector&)` (0x5DABC0)
    pub fn grass_set_global_camera_pos(pos: GPtr<u8>) = cdecl 0x5DABC0 => unported;
    /// `CPlantMgr::UpdateAmbientColor()` (0x5DB310)
    pub fn plant_update_ambient_color() = cdecl 0x5DB310 => unported;
    /// `CPlantMgr::CalculateWindBending()` (0x5DB3D0): result in ST0
    pub fn plant_calculate_wind_bending() -> f64 = cdecl 0x5DB3D0 => unported;
    /// `CGrassRenderer::SetGlobalWindBending(float)` (0x5DAC00)
    pub fn grass_set_global_wind_bending(v: u32) = cdecl 0x5DAC00 => unported;
    /// `CPlantMgr::_ColEntityCache_Update(const CVector&, bool)` (0x5DC510)
    pub fn plant_col_entity_cache_update(pos: GPtr<u8>, all: u32) = cdecl 0x5DC510 => unported;
    /// `CPlantMgr::_ProcessEntryCollisionDataSections(CPlantColEntEntry*, const CVector&, int)` (0x5DCD80)
    pub fn plant_process_entry_col_data_sections(e: GPtr<u8>, pos: GPtr<u8>, i: u32) = cdecl 0x5DCD80 => unported;
    /// `RwIm3DTransform(RwIm3DVertex*, RwUInt32 n, RwMatrix*, RwUInt32 flags)` (0x7EF450)
    pub fn rw_im3d_transform(v: GPtr<u8>, n: u32, m: u32, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderPrimitive(RwPrimitiveType)` (0x7EF6B0)
    pub fn rw_im3d_render_primitive(ty: u32) -> i32 = cdecl 0x7EF6B0 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    pub fn rw_im3d_end() -> i32 = cdecl 0x7EF520 => unported;
    /// `RpClumpGetBoundingSphere(RpClump*, RwSphere*, bool useLTM)` (0x734FC0)
    pub fn rp_clump_get_bounding_sphere(c: GPtr<u8>, s: GPtr<u8>, ltm: u32) -> GPtr<u8> = cdecl 0x734FC0 => unported;
    /// `_rpAtomicResyncInterpolatedSphere(RpAtomic*)` (0x7491F0)
    pub fn rp_atomic_resync_interpolated_sphere(a: GPtr<u8>) = cdecl 0x7491F0 => unported;
    /// `RwV3dTransformPoints(RwV3d* out, const RwV3d* in, RwInt32 n, const RwMatrix*)` (0x7EDD90)
    pub fn rw_v3d_transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
    /// `CShadowCamera::SetFrustum(float)` (0x7054C0)
    pub fn shadow_camera_set_frustum(this: GPtr<u8>, size: u32) -> GPtr<u8> = thiscall 0x7054C0 => crate::mvpa_render2::shadow_camera_set_frustum;
    /// `CShadowCamera::SetCenter(RwV3d*)` (0x705590)
    pub fn shadow_camera_set_center(this: GPtr<u8>, c: GPtr<u8>) -> GPtr<u8> = thiscall 0x705590 => unported;
    /// `CRealTimeShadow::Destroy()` (0x705990)
    pub fn real_time_shadow_destroy(this: GPtr<u8>) = thiscall 0x705990 => crate::mvpa_render2::real_time_shadow_destroy;
}

/// `FxFrustumInfo_c::IsCollision(FxSphere_c* s)` (0x4AA030, thiscall, `ret 4`): this = the frustum
/// sphere (+0 centre, +0xC radius) and four planes (16 bytes from +0x14: normal, distance). 0 when
/// `(r + s.r)^2 <= dz^2 + dy^2 + dx^2` (x87, that order; unordered → 0). Otherwise the planes are tried
/// starting at the sphere's cached index (s+0x10, `& 3`): a plane with `n . s.centre - d > s.r` stores
/// its index at s+0x10 and gives 0; none → 1.
#[hook(addr = 0x4AA030, conv = thiscall, name = "FxFrustumInfo_c::IsCollision", expect = "8B 54 24 04 D9 02",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the sphere's cached plane index")]
pub fn fx_frustum_is_collision(this: GPtr<u8>, s: GPtr<u8>) -> u32 {
    let sf = |o: isize| rd::<f32>(s, o);
    let tf = |o: isize| rd::<f32>(this, o);
    let mut f = Fpu::new();
    f.fld(sf(0));
    f.fsub_m(tf(0));
    f.fld(tf(4));
    f.fsubr_m(sf(4));
    f.fld(tf(8));
    f.fsubr_m(sf(8));
    f.fld(tf(0xC));
    f.fadd_m(sf(0xC));
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
    if f.jne(0x41) {
        return 0;
    }
    let mut idx = rd::<u32>(s, 0x10);
    for _ in 0..4 {
        let p = idx & 3;
        let pl = this.byte_add(0x14 + p as isize * 16);
        let mut f = Fpu::new();
        f.fld(rd::<f32>(pl, 4));
        f.fmul_m(sf(4));
        f.fld(rd::<f32>(pl, 0));
        f.fmul_m(sf(0));
        f.faddp(1);
        f.fld(rd::<f32>(pl, 8));
        f.fmul_m(sf(8));
        f.faddp(1);
        f.fsub_m(rd::<f32>(pl, 0xC));
        f.fcomp_m(sf(0xC));
        if !f.jne(0x41) {
            wr::<u32>(s, 0x10, p);
            return 0;
        }
        idx = idx.wrapping_add(1);
    }
    1
}

/// `CPlantMgr::PreUpdateOnceForNewCameraPos(const CVector& cam)` (0x5DCF30, cdecl): the u16 scan code
/// (0xC03980) is incremented and passed on, the grass renderer gets the camera position, the ambient
/// colour is updated, the wind bending (ST0, stored as f32) is passed on, the col-entity cache updated
/// (`_ColEntityCache_Update(cam, false)`), and every entry of the close list (0xC0399C, next at +0xC)
/// gets `_ProcessEntryCollisionDataSections(entry, cam, 0xFAFAFAFA)`. AL = 1; the upper EAX bits are the
/// last callee's.
#[hook(addr = 0x5DCF30, conv = cdecl, name = "CPlantMgr::PreUpdateOnceForNewCameraPos", expect = "51 33 C0 66 A1 80 39 C0 00",
       route = "L2", hazards = "ax-al-return,heap-writes", recs = "grove-walk",
       note = "callers (0x5B14B5, 0x5B14C2) return EAX to their own callers; upper bits differ from the original's")]
pub fn plant_mgr_pre_update_once_for_new_camera_pos(cam: GPtr<u8>) -> u8 {
    let code = g::<u16>(0xC03980).read().wrapping_add(1);
    g::<u16>(0xC03980).write(code);
    grass_set_current_scan_code(u32::from(code));
    grass_set_global_camera_pos(cam);
    plant_update_ambient_color();
    let wind = X87::from_f64_exact(plant_calculate_wind_bending()).to_f32();
    grass_set_global_wind_bending(wind.to_bits());
    plant_col_entity_cache_update(cam, 0);
    let mut e = g::<GPtr<u8>>(0xC0399C).read();
    while !e.is_null() {
        plant_process_entry_col_data_sections(e, cam, 0xFAFA_FAFA);
        e = rd::<GPtr<u8>>(e, 0xC);
    }
    1
}

/// `CustomEnvMapPipeMaterialData::FromBuffer(const float* src, CustomEnvMapPipeMaterialData* dst)`
/// (0x5D8BE0, cdecl): bytes 0..3 = `_ftol(src[i] * 8.0f)`, byte 4 = `_ftol(src[4] * 255.0f)`, +8 =
/// src[5] (dword, also returned in EAX), u16 +6 = 0.
#[hook(addr = 0x5D8BE0, conv = cdecl, name = "CustomEnvMapPipeMaterialData::FromBuffer", expect = "56 57 8B 7C 24 0C",
       unverified = "mvpa-render-3", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the material plugin data")]
pub fn env_map_material_data_from_buffer(src: GPtr<u8>, dst: GPtr<u8>) -> u32 {
    for i in 0..4isize {
        let v = X87::from_f32(rd::<f32>(src, i * 4)).fmul_m(k(0x4100_0000));
        wr::<u8>(dst, i, ftol32(v) as u8);
    }
    let v = X87::from_f32(rd::<f32>(src, 0x10)).fmul_m(k(0x437F_0000));
    wr::<u8>(dst, 4, ftol32(v) as u8);
    let r = rd::<u32>(src, 0x14);
    wr::<u32>(dst, 8, r);
    wr::<u16>(dst, 6, 0);
    r
}

/// `Fx_c::RenderEnd()` (0x4A1600, cdecl): with `n = 0xA9AE5C` triangles buffered (read through the
/// protection chunk 0x4059D7): `RwIm3DTransform(TempBufferRenderVertices 0xC4D958, 3n, [0xA9AE68],
/// [0xA9AE60])`, and on success `RwIm3DRenderPrimitive(TRILIST)` + `RwIm3DEnd()`; then the counters
/// 0xA9AE5C / 0xA9AE58 = 0 and the write pointer 0xA9AE6C = 0xC4D958.
#[hook(addr = 0x4A1600, conv = cdecl, name = "Fx_c::RenderEnd", expect = "E9 D2 43 F6 FF 85 C0",
       unverified = "mvpa-render-3", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn fx_render_end() {
    let n = g::<u32>(0xA9AE5C).read();
    if n == 0 {
        return;
    }
    let flags = g::<u32>(0xA9AE60).read();
    let m = g::<u32>(0xA9AE68).read();
    if !rw_im3d_transform(GPtr::from_addr(0xC4D958), n.wrapping_mul(3), m, flags).is_null() {
        rw_im3d_render_primitive(3);
        rw_im3d_end();
    }
    g::<u32>(0xA9AE5C).write(0);
    g::<u32>(0xA9AE58).write(0);
    g::<u32>(0xA9AE6C).write(0xC4D958);
}

/// `CRealTimeShadow::SetupForThisEntity(CPhysical* e)` (0x706520, thiscall, `ret 4`): +0 = e; no RW
/// object (e+0x18) → AL = 0 (EAX = e with AL cleared). +0x24 = RW object type byte; not 1 or 2 →
/// `Destroy()`, AL = 0. Atomic: resync its interpolated sphere when flagged (+0x4C bit 1), copy the
/// bounding sphere (+0x1C) to +0x2C; clump: `RpClumpGetBoundingSphere(clump, this+0x2C, true)`. Both:
/// +0x48 = radius (+0x38), centre transformed by the frame's matrix (frame +0x10) into +0x3C;
/// `SetFrustum(radius * 1.1f)`, `SetCenter(this+0x3C)` on the camera (+8); AL = 1 (upper EAX from
/// SetCenter). The only caller ignores the result.
#[hook(addr = 0x706520, conv = thiscall, name = "CRealTimeShadow::SetupForThisEntity", expect = "8B 44 24 04 56 8B F1",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the shadow object (heap)")]
pub fn real_time_shadow_setup_for_this_entity(this: GPtr<u8>, e: GPtr<u8>) -> u8 {
    wr::<GPtr<u8>>(this, 0, e);
    let rw = rd::<GPtr<u8>>(e, 0x18);
    if rw.is_null() {
        return 0;
    }
    let ty = u32::from(rd::<u8>(rw, 0));
    wr::<u32>(this, 0x24, ty);
    if ty == 0 || ty > 2 {
        real_time_shadow_destroy(this);
        return 0;
    }
    let sphere = this.byte_add(0x2C);
    if ty == 1 {
        if rd::<u8>(rw, 0x4C) & 2 != 0 {
            rp_atomic_resync_interpolated_sphere(rw);
        }
        for o in [0, 4, 8, 0xC] {
            wr::<u32>(sphere, o, rd::<u32>(rw, 0x1C + o));
        }
    } else {
        rp_clump_get_bounding_sphere(rw, sphere, 1);
    }
    wr::<u32>(this, 0x48, rd::<u32>(this, 0x38));
    let m = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
    rw_v3d_transform_points(this.byte_add(0x3C), sphere, 1, m);
    let size = X87::from_f32(rd::<f32>(this, 0x38)).fmul_m(k(0x3F8C_CCCD)).to_f32();
    shadow_camera_set_frustum(this.byte_add(8), size.to_bits());
    shadow_camera_set_center(this.byte_add(8), this.byte_add(0x3C));
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_FX_FRUSTUM_IS_COLLISION,
    HOOK_PLANT_MGR_PRE_UPDATE_ONCE_FOR_NEW_CAMERA_POS,
    HOOK_ENV_MAP_MATERIAL_DATA_FROM_BUFFER,
    HOOK_FX_RENDER_END,
    HOOK_REAL_TIME_SHADOW_SETUP_FOR_THIS_ENTITY,
];
