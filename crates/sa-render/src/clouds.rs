//! PORT-RENDER: `CClouds` (sky polys, volumetric clouds, moving fog): the leaves and initialisers.
//! Ported from the 1.0 US bytes; gta-reversed `Clouds.cpp` and `docs/re/specs/RENDER/CClouds*.md` as
//! reading aids.
//!
//! `ms_vc` (tVolumetricClouds, 0xC6AAB0): bUsed[180] +0, bJustCreated[180] +0xB4, pos[180] +0x168,
//! size[180] +0x9D8, alpha[180] +0x1248, texture +0x1518, quadNormal[3] +0x151C, modelX/Y/Z/U/V[18]
//! +0x1540 / +0x1588 / +0x15D0 / +0x1618 / +0x1660. `ms_mf` (tMovingFog, 0xC6C158): fogSlots[350] +0,
//! posn[350] +0x160, size +0x11C8, intensity +0x1740, maxIntensity +0x1CB8, wind +0x2230, speed +0x223C,
//! primIndices[6] +0x27E4.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::timer::TIME_STEP;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{Fpu, ftol32, g, k, rd, wr};
use crate::weather::{FOGGYNESS_SF, WIND};

sa_core::globals! {
    list GLOBALS;
    /// `CClouds::m_fVolumetricCloudMaxDistance`
    pub static VC_MAX_DISTANCE @ 0xC6AA58: f32 = 0.0;
    /// `CClouds::m_VolumetricCloudsUsedNum`
    pub static VC_USED_NUM @ 0xC6AA5C: u32 = 0;
    /// `CClouds::IndividualRotation`
    pub static INDIVIDUAL_ROTATION @ 0xC6AA6C: i32 = 0;
    /// `CClouds::CloudRotation`
    pub static CLOUD_ROTATION @ 0xC6AA70: f32 = 0.0;
    /// `gpMoonMask`
    pub static MOON_MASK @ 0xC6AA74: GPtr<u8> = GPtr::NULL;
    /// `gpCloudTex`
    pub static CLOUD_TEX @ 0xC6AA78: GPtr<u8> = GPtr::NULL;
    /// `gpCloudMaskTex`
    pub static CLOUD_MASK_TEX @ 0xC6AA7C: GPtr<u8> = GPtr::NULL;
    /// `CClouds::ms_vc` (tVolumetricClouds)
    pub static MS_VC @ 0xC6AAB0: [u8; 0x16A8] = [0; 0x16A8];
    /// `CClouds::ms_mf` (tMovingFog)
    pub static MS_MF @ 0xC6C158: [u8; 0x27FC] = [0; 0x27FC];
}

const VC: usize = 0xC6AAB0;
const VC_USED: isize = 0;
const VC_JUST_CREATED: isize = 0xB4;
const VC_TEXTURE: isize = 0x1518;
const MAX_VOLUMETRIC_CLOUDS: i32 = 180;
const MF: usize = 0xC6C158;
const MF_WIND: isize = 0x2230;
const MF_PRIM_INDICES: isize = 0x27E4;
const MAX_MOVING_FOG: i32 = 350;

/// `TheCamera.m_fOrientation` (TheCamera +0x150).
const CAM_ORIENTATION: usize = 0xB6F178;
/// `Scene.camera`
const SCENE_CAMERA: usize = 0xC1703C;
/// `TempBufferVerticesStored`, `TempBufferIndicesStored`, `TempBufferRenderVertices` (RwIm3DVertex,
/// 0x24 bytes) and `aTempBufferIndices` (u16).
const TEMP_VERTICES_STORED: usize = 0xC4B950;
const TEMP_INDICES_STORED: usize = 0xC4B954;
const TEMP_RENDER_VERTICES: usize = 0xC4D958;
const TEMP_BUFFER_INDICES: usize = 0xC4B958;

/// 0.85 (0x862E1C), 0.001 (0x858CDC), 0.5 (0x858B8C), 0.3 (0x858C24), 60.0 (0x858B34), 2^32 (0x858C54),
/// 600.0 (0x858EDC), 0.06 (the wind initial value)
const ORIENT_OFFSET: u32 = 0x3F59_999A;
const ROT_SCALE: u32 = 0x3A83_126F;
const HALF: u32 = 0x3F00_0000;
const P03: u32 = 0x3E99_999A;
const SIXTY: u32 = 0x4270_0000;
const TWO_POW_32: u32 = 0x4F80_0000;
const VC_MAX_DIST: u32 = 0x4416_0000;
const MF_WIND_INIT: u32 = 0x3D75_C28F;

orig! {
    /// `RwTextureDestroy(RwTexture*)` (0x7F3820)
    pub fn rw_texture_destroy(tex: GPtr<u8>) -> i32 = cdecl 0x7F3820 => unported;
}

/// `CClouds::Shutdown()` (0x712FA0): destroys gpCloudTex, gpCloudMaskTex and `ms_vc.texture` (no null
/// checks) and clears the three pointers.
#[hook(addr = 0x712FA0, conv = cdecl, name = "CClouds::Shutdown", expect = "A1 78 AA C6 00")]
pub fn shutdown() {
    rw_texture_destroy(CLOUD_TEX.get());
    CLOUD_TEX.set(GPtr::NULL);
    rw_texture_destroy(CLOUD_MASK_TEX.get());
    CLOUD_MASK_TEX.set(GPtr::NULL);
    let vc = GPtr::<u8>::from_addr(VC);
    rw_texture_destroy(rd::<GPtr<u8>>(vc, VC_TEXTURE));
    wr::<u32>(vc, VC_TEXTURE, 0);
}

/// `CClouds::Update()` (0x712FF0): `CloudRotation += sin(TheCamera.m_fOrientation - 0.85f) * Wind *
/// 0.001f` (FSIN: sa-math's f64 model); `IndividualRotation = _ftol((TimeStep * Wind * 0.5f + 0.3f) *
/// 60.0f + (float)(uint32)IndividualRotation)`.
#[hook(addr = 0x712FF0, conv = cdecl, name = "CClouds::Update", expect = "51 D9 05 78 F1 B6 00")]
pub fn update() {
    let mut f = Fpu::new();
    f.fld(g::<f32>(CAM_ORIENTATION).read());
    let rot = INDIVIDUAL_ROTATION.get();
    f.fsub_m(k(ORIENT_OFFSET));
    let v = f.pop();
    f.push(v.fsin());
    f.fmul_m(WIND.get());
    f.fmul_m(k(ROT_SCALE));
    f.fadd_m(CLOUD_ROTATION.get());
    CLOUD_ROTATION.set(f.fstp());
    f.fld(TIME_STEP.get());
    f.fmul_m(WIND.get());
    f.fmul_m(k(HALF));
    f.fadd_m(k(P03));
    f.fmul_m(k(SIXTY));
    f.fild(INDIVIDUAL_ROTATION.get());
    if rot < 0 {
        f.fadd_m(k(TWO_POW_32));
    }
    f.faddp(1);
    INDIVIDUAL_ROTATION.set(ftol32(f.pop()));
}

/// `CClouds::SetUpOneSkyPoly(CVector v1, v2, v3, v4, u8 r1, g1, b1, u8 r2, g2, b2)` (0x713060): appends
/// a quad to the 3D temp buffer: vertices n..n+3 at the four corners (normals untouched, u = v = 0),
/// colour `0xFF r1 g1 b1` on the first two, `0xFF r2 g2 b2` on the last two; indices (n, n+2, n+1,
/// n+1, n+2, n+3).
#[hook(addr = 0x713060, conv = cdecl, name = "CClouds::SetUpOneSkyPoly", expect = "0F B6 4C 24 34")]
#[allow(clippy::too_many_arguments)]
pub fn set_up_one_sky_poly(
    v1x: u32,
    v1y: u32,
    v1z: u32,
    v2x: u32,
    v2y: u32,
    v2z: u32,
    v3x: u32,
    v3y: u32,
    v3z: u32,
    v4x: u32,
    v4y: u32,
    v4z: u32,
    r1: u8,
    g1: u8,
    b1: u8,
    r2: u8,
    g2: u8,
    b2: u8,
) {
    let col = |r: u8, g_: u8, b: u8| 0xFF00_0000 | u32::from(r) << 16 | u32::from(g_) << 8 | u32::from(b);
    let (c1, c2) = (col(r1, g1, b1), col(r2, g2, b2));
    let n = g::<i32>(TEMP_VERTICES_STORED).read();
    let base = GPtr::<u8>::from_addr(TEMP_RENDER_VERTICES).byte_add(n as isize * 0x24);
    for (i, (px, py, pz, c)) in [
        (v1x, v1y, v1z, c1),
        (v2x, v2y, v2z, c1),
        (v3x, v3y, v3z, c2),
        (v4x, v4y, v4z, c2),
    ]
    .into_iter()
    .enumerate()
    {
        let v = base.byte_add(i as isize * 0x24);
        wr::<u32>(v, 0x18, c);
        wr::<u32>(v, 0, px);
        wr::<u32>(v, 4, py);
        wr::<u32>(v, 8, pz);
        wr::<u32>(v, 0x1C, 0);
        wr::<u32>(v, 0x20, 0);
    }
    let ni = g::<i32>(TEMP_INDICES_STORED).read();
    let idx = GPtr::<u16>::from_addr(TEMP_BUFFER_INDICES);
    for (o, d) in [(0, 0), (1, 2), (2, 1), (3, 1), (4, 2), (5, 3)] {
        idx.wrapping_index(ni.wrapping_add(o))
            .write(n.wrapping_add(d) as u16);
    }
    g::<i32>(TEMP_INDICES_STORED).write(ni.wrapping_add(6));
    g::<i32>(TEMP_VERTICES_STORED).write(n.wrapping_add(4));
}

/// `CClouds::VolumetricCloudsInit()` (0x7131C0): the quad normals and the 18-vertex model tables
/// (constant dword stores, in the original order), then bUsed / bJustCreated cleared for all 180.
#[hook(addr = 0x7131C0, conv = cdecl, name = "CClouds::VolumetricCloudsInit", expect = "C7 05 CC BF C6 00 00 00 00 00")]
pub fn volumetric_clouds_init() {
    for (a, v) in VC_INIT {
        g::<u32>(a).write(v);
    }
    let vc = GPtr::<u8>::from_addr(VC);
    let mut i = 0;
    while i < MAX_VOLUMETRIC_CLOUDS as isize {
        wr::<u8>(vc, VC_USED + i, 0);
        wr::<u8>(vc, VC_JUST_CREATED + i, 0);
        i += 1;
    }
}

/// `CClouds::VolumetricClouds_GetFirstFreeSlot()` (0x7135C0): the first `i < m_VolumetricCloudsUsedNum`
/// (signed) with `!bUsed[i]`, else -1.
#[hook(addr = 0x7135C0, conv = cdecl, name = "CClouds::VolumetricClouds_GetFirstFreeSlot", expect = "8B 0D 5C AA C6 00")]
pub fn volumetric_clouds_get_first_free_slot() -> i32 {
    let n = VC_USED_NUM.get() as i32;
    let vc = GPtr::<u8>::from_addr(VC);
    let mut i = 0;
    while i < n {
        if rd::<u8>(vc, VC_USED + i as isize) == 0 {
            return i;
        }
        i += 1;
    }
    -1
}

/// `CClouds::VolumetricClouds_Delete(int i)` (0x7135F0): i clamped to 0..179; clears bUsed[i] and
/// bJustCreated[i].
#[hook(addr = 0x7135F0, conv = cdecl, name = "CClouds::VolumetricClouds_Delete", expect = "8B 44 24 04 33 C9")]
pub fn volumetric_clouds_delete(i: i32) {
    let i = i.clamp(0, MAX_VOLUMETRIC_CLOUDS - 1) as isize;
    let vc = GPtr::<u8>::from_addr(VC);
    wr::<u8>(vc, VC_USED + i, 0);
    wr::<u8>(vc, VC_JUST_CREATED + i, 0);
}

/// `CClouds::VolumetricCloudsGetMaxDistance()` (0x713630, ST0): `Scene.camera->farPlane` (+0x84) when it
/// is below 600.0 (ordered), else 600.0.
#[hook(addr = 0x713630, conv = cdecl, name = "CClouds::VolumetricCloudsGetMaxDistance", expect = "51 A1 3C 70 C1 00")]
pub fn volumetric_clouds_get_max_distance() -> X87 {
    let cam = g::<GPtr<u8>>(SCENE_CAMERA).read();
    let mut f = Fpu::new();
    f.fld(k(VC_MAX_DIST));
    f.fld(rd::<f32>(cam, 0x84));
    let local = f.fst();
    f.fcomp_m(k(VC_MAX_DIST));
    if !f.jp(0x05) {
        f.fstp_st(0);
        f.fld(local);
    }
    f.pop()
}

/// `CClouds::MovingFogInit()` (0x713660): `m_nPrimIndices = {0, 1, 2, 0, 2, 3}`, all fog slots free,
/// wind (0.06, 0.06, 0).
#[hook(addr = 0x713660, conv = cdecl, name = "CClouds::MovingFogInit", expect = "33 C9 B8 02 00 00 00")]
pub fn moving_fog_init() {
    let mf = GPtr::<u8>::from_addr(MF);
    for (i, v) in [(0, 0u32), (2, 2), (3, 0), (4, 2), (1, 1), (5, 3)] {
        wr::<u32>(mf, MF_PRIM_INDICES + i * 4, v);
    }
    let mut o = 0;
    while o < MAX_MOVING_FOG as isize {
        wr::<u8>(mf, o, 0);
        o += 1;
    }
    wr::<u32>(mf, MF_WIND, MF_WIND_INIT);
    wr::<u32>(mf, MF_WIND + 4, MF_WIND_INIT);
    wr::<u32>(mf, MF_WIND + 8, 0);
}

/// `CClouds::MovingFog_GetFXIntensity()` (0x7136D0, ST0): `CWeather::Foggyness_SF`.
#[hook(addr = 0x7136D0, conv = cdecl, name = "CClouds::MovingFog_GetFXIntensity", expect = "D9 05 FC 12 C8 00")]
pub fn moving_fog_get_fx_intensity() -> X87 {
    X87::from_f32(FOGGYNESS_SF.get())
}

/// `CClouds::MovingFog_GetWind()` (0x7136E0, CVector by hidden pointer): copies `ms_mf.m_vecWind` (as
/// dwords), returns the pointer.
#[hook(addr = 0x7136E0, conv = cdecl, name = "CClouds::MovingFog_GetWind", expect = "8B 15 88 E3 C6 00")]
pub fn moving_fog_get_wind(out: GPtr<u32>) -> GPtr<u32> {
    let mf = GPtr::<u8>::from_addr(MF);
    for i in 0..3 {
        out.wrapping_index(i)
            .write(rd::<u32>(mf, MF_WIND + i as isize * 4));
    }
    out
}

/// `CClouds::MovingFog_GetFirstFreeSlot()` (0x713710): the first free of the 350 fog slots, else -1.
/// Not hooked: the loop branches back to 0x713712, inside the patched entry.
pub fn moving_fog_get_first_free_slot() -> i32 {
    let mf = GPtr::<u8>::from_addr(MF);
    let mut i = 0;
    while i < MAX_MOVING_FOG {
        if rd::<u8>(mf, i as isize) == 0 {
            return i;
        }
        i += 1;
    }
    -1
}

/// `CClouds::MovingFog_Delete(int i)` (0x713730): i clamped to 0..349; frees the slot.
#[hook(addr = 0x713730, conv = cdecl, name = "CClouds::MovingFog_Delete", expect = "8B 44 24 04 33 C9")]
pub fn moving_fog_delete(i: i32) {
    let i = i.clamp(0, MAX_MOVING_FOG - 1) as isize;
    wr::<u8>(GPtr::from_addr(MF), i, 0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_SHUTDOWN,
    HOOK_UPDATE,
    HOOK_SET_UP_ONE_SKY_POLY,
    HOOK_VOLUMETRIC_CLOUDS_INIT,
    HOOK_VOLUMETRIC_CLOUDS_GET_FIRST_FREE_SLOT,
    HOOK_VOLUMETRIC_CLOUDS_DELETE,
    HOOK_VOLUMETRIC_CLOUDS_GET_MAX_DISTANCE,
    HOOK_MOVING_FOG_INIT,
    HOOK_MOVING_FOG_GET_FX_INTENSITY,
    HOOK_MOVING_FOG_GET_WIND,
    HOOK_MOVING_FOG_DELETE,
];

/// The constant stores of `VolumetricCloudsInit` (address, dword), in the original order (0x7131C0 ..
/// 0x713596).
#[rustfmt::skip]
const VC_INIT: [(usize, u32); 99] = [
    (0xC6BFCC, 0x0),
    (0xC6BFD0, 0x3F800000),
    (0xC6BFD4, 0x0),
    (0xC6BFF0, 0xBF000000),
    (0xC6C038, 0x0),
    (0xC6C080, 0x3F000000),
    (0xC6C0C8, 0x0),
    (0xC6C110, 0x0),
    (0xC6BFF4, 0x3F000000),
    (0xC6C03C, 0x0),
    (0xC6C084, 0x3F000000),
    (0xC6C0CC, 0x3F800000),
    (0xC6C114, 0x0),
    (0xC6BFF8, 0xBF000000),
    (0xC6C040, 0x0),
    (0xC6C088, 0xBF000000),
    (0xC6C0D0, 0x0),
    (0xC6C118, 0x3F800000),
    (0xC6BFFC, 0x3F000000),
    (0xC6C044, 0x0),
    (0xC6C08C, 0x3F000000),
    (0xC6C0D4, 0x3F800000),
    (0xC6C11C, 0x0),
    (0xC6C000, 0x3F000000),
    (0xC6C048, 0x0),
    (0xC6C090, 0xBF000000),
    (0xC6C0D8, 0x3F800000),
    (0xC6C120, 0x3F800000),
    (0xC6C004, 0xBF000000),
    (0xC6C04C, 0x0),
    (0xC6C094, 0xBF000000),
    (0xC6C0DC, 0x0),
    (0xC6C124, 0x3F800000),
    (0xC6BFD8, 0x0),
    (0xC6BFDC, 0x0),
    (0xC6BFE0, 0x3F800000),
    (0xC6C008, 0xBF000000),
    (0xC6C050, 0x3F000000),
    (0xC6C098, 0x0),
    (0xC6C0E0, 0x0),
    (0xC6C128, 0x0),
    (0xC6C00C, 0x3F000000),
    (0xC6C054, 0x3F000000),
    (0xC6C09C, 0x0),
    (0xC6C0E4, 0x3F800000),
    (0xC6C12C, 0x0),
    (0xC6C010, 0xBF000000),
    (0xC6C058, 0xBF000000),
    (0xC6C0A0, 0x0),
    (0xC6C0E8, 0x0),
    (0xC6C130, 0x3F800000),
    (0xC6C014, 0x3F000000),
    (0xC6C05C, 0x3F000000),
    (0xC6C0A4, 0x0),
    (0xC6C0EC, 0x3F800000),
    (0xC6C134, 0x0),
    (0xC6C018, 0x3F000000),
    (0xC6C060, 0xBF000000),
    (0xC6C0A8, 0x0),
    (0xC6C0F0, 0x3F800000),
    (0xC6C138, 0x3F800000),
    (0xC6C01C, 0xBF000000),
    (0xC6C064, 0xBF000000),
    (0xC6C0AC, 0x0),
    (0xC6C0F4, 0x0),
    (0xC6C13C, 0x3F800000),
    (0xC6BFE4, 0x3F800000),
    (0xC6BFE8, 0x0),
    (0xC6BFEC, 0x0),
    (0xC6C020, 0x0),
    (0xC6C068, 0xBF000000),
    (0xC6C0B0, 0x3F000000),
    (0xC6C0F8, 0x0),
    (0xC6C140, 0x0),
    (0xC6C024, 0x0),
    (0xC6C06C, 0x3F000000),
    (0xC6C0B4, 0x3F000000),
    (0xC6C0FC, 0x3F800000),
    (0xC6C144, 0x0),
    (0xC6C028, 0x0),
    (0xC6C070, 0xBF000000),
    (0xC6C0B8, 0xBF000000),
    (0xC6C100, 0x0),
    (0xC6C148, 0x3F800000),
    (0xC6C02C, 0x0),
    (0xC6C074, 0x3F000000),
    (0xC6C0BC, 0x3F000000),
    (0xC6C104, 0x3F800000),
    (0xC6C14C, 0x0),
    (0xC6C030, 0x0),
    (0xC6C078, 0x3F000000),
    (0xC6C0C0, 0xBF000000),
    (0xC6C108, 0x3F800000),
    (0xC6C150, 0x3F800000),
    (0xC6C034, 0x0),
    (0xC6C07C, 0xBF000000),
    (0xC6C0C4, 0xBF000000),
    (0xC6C10C, 0x0),
    (0xC6C154, 0x3F800000),
];
