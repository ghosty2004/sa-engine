//! STD-4 front-end path: `CDraw::CalculateAspectRatio`, `CVisibilityPlugins::SetRenderWareCamera`,
//! and `CSprite2d::SetRecipNearClip`. Ported from the 1.0 US bytes
//! (gta-reversed `Draw.cpp`, `VisibilityPlugins.cpp` as reading aid).

use sa_core::abi::{HookReg, NativeReg};
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

/// `FrontEndMenuManager.m_bWidescreenOn` (+0x4B).
const MENU_WIDESCREEN: usize = 0xBA6793;
/// `TheCamera.m_bWideScreenOn` (+0x3D).
const CAMERA_WIDESCREEN: usize = 0xB6F065;
/// `CDraw::ms_fAspectRatio`.
const ASPECT_RATIO: usize = 0xC3EFA4;

fn rdf(a: usize) -> f32 {
    GPtr::<f32>::from_addr(a).read()
}
fn wrf(a: usize, v: f32) {
    GPtr::<f32>::from_addr(a).write(v);
}

/// `CDraw::CalculateAspectRatio` (0x6FF420, cdecl, result in ST0): 16:9 ([0x862CE4]) when the menu's
/// widescreen option is on, else 5:4 ([0x8595F0]) when the camera's widescreen flag is set, else 4:3
/// ([0x85F09C]); stored into `ms_fAspectRatio` with `fst` (the value stays in ST0).
///
/// Not registered: 0x6FF420 is hooked by sa-render mvpb_r1.rs (duplicate port left unregistered when STD-5 merged both).
pub fn calculate_aspect_ratio() -> f32 {
    let r = if GPtr::<u8>::from_addr(MENU_WIDESCREEN).read() != 0 {
        rdf(0x862CE4)
    } else if GPtr::<u8>::from_addr(CAMERA_WIDESCREEN).read() != 0 {
        rdf(0x8595F0)
    } else {
        rdf(0x85F09C)
    };
    wrf(ASPECT_RATIO, r);
    r
}

/// `(base, k)` → `round24(round24(base * k)^2)`: `fld base; fmul k; fld st0; fmul st1`.
fn sq(base: f32, k: usize) -> X87 {
    let a = X87::from_f32(base).fmul_m(rdf(k));
    a.fmul(a)
}

/// `CVisibilityPlugins::SetRenderWareCamera(RwCamera*)` (0x7328C0, cdecl): `ms_pCamera = camera`,
/// `ms_pCameraPosn = &RwCameraGetFrame(camera)->ltm.pos` (frame +0x40), then the squared LOD
/// distances from `TheCamera.m_fLODDistMultiplier` (0xB6F118) and `m_fGenerationDistMultiplier`
/// (0xB6F11C), each `(m * k)^2` (doubled with `fadd st, st` where the original does) at PC=24:
/// 0xC8802C = (m1·20)², 0xC88028 = (m1·50)², 0xC88044 = 2(m2·45)², 0xC88040 = 2(m2·70)²,
/// 0xC8803C = 0xC88038 = 2(m2·150)², 0xC88034 = 2(m1·60)², 0xC88030 = 2(m1·70)².
#[hook(addr = 0x7328C0, conv = cdecl, name = "CVisibilityPlugins::SetRenderWareCamera", expect = "D9 05 18 F1 B6 00",
       route = "L2", hazards = "none", recs = "boot")]
pub fn set_render_ware_camera(camera: GPtr<u8>) {
    let m1 = rdf(0xB6F118);
    GPtr::<u32>::from_addr(0xC8804C).write(camera.addr() as u32);
    let frame = camera.byte_add(4).cast::<u32>().read();
    GPtr::<u32>::from_addr(0xC88050).write(frame.wrapping_add(0x40));
    wrf(0xC8802C, sq(m1, 0x858BA4).to_f32());
    wrf(0xC88028, sq(m1, 0x858B40).to_f32());
    let m2 = rdf(0xB6F11C);
    let v = sq(m2, 0x858CB4);
    wrf(0xC88044, v.fadd(v).to_f32());
    let v = sq(m2, 0x858CE0);
    wrf(0xC88040, v.fadd(v).to_f32());
    let v = sq(m2, 0x858A28);
    let v = v.fadd(v).to_f32();
    wrf(0xC8803C, v);
    wrf(0xC88038, v);
    let m1 = rdf(0xB6F118);
    let v = sq(m1, 0x858B34);
    wrf(0xC88034, v.fadd(v).to_f32());
    let v = sq(m1, 0x858CE0);
    wrf(0xC88030, v.fadd(v).to_f32());
}

/// `CSprite2d::SetRecipNearClip` (0x727260): a lone `ret` in 1.0 US.
#[sa_core::native(addr = 0x727260, conv = cdecl, name = "CSprite2d::SetRecipNearClip")]
pub fn set_recip_near_clip() {}

/// `Fx_c::GetFxQuality()` (0x49EA50, thiscall): `return this->m_fxQuality` (+0x54); 4 bytes, not
/// hookable.
#[sa_core::native(addr = 0x49EA50, conv = thiscall, name = "Fx_c::GetFxQuality")]
pub fn fx_get_fx_quality(this: GPtr<u8>) -> u32 {
    this.byte_add(0x54).cast::<u32>().read()
}

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[NATIVE_SET_RECIP_NEAR_CLIP, NATIVE_FX_GET_FX_QUALITY];

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_SET_RENDER_WARE_CAMERA];
