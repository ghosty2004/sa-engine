//! `CVisibilityPlugins::RenderWeaponCB` (0x733670, batch `std10-render-2`): ported from the 1.0 US bytes
//! (gta-reversed's version as a reading aid). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd};

orig! {
    /// `RwFrameGetLTM(frame)` (0x7F0990)
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// `AtomicDefaultRenderCallBack(atomic)` (0x7491C0)
    fn atomic_default_render(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x7491C0 => unported;
}

/// `ms_pCameraPosn`, `TheCamera.m_fLODDistMultiplier`, the frame plugin offset.
const CAMERA_POSN: usize = 0xC88050;
const LOD_DIST_MULT: usize = 0xB6F118;
const FRAME_PLUGIN_OFFSET: usize = 0x8D6090;

/// `CVisibilityPlugins::RenderWeaponCB(RpAtomic*)` (0x733670, cdecl): the clump frame's plugin data (the model info,
/// its draw distance at +0x18) times the camera's LOD multiplier is the range; rendered (`AtomicDefaultRenderCallBack`)
/// when `r * r > (dz*dz + dx*dx) + dy*dy` for the frame's LTM position relative to the camera (all on the x87
/// stack). Returns the atomic.
#[hook(addr = 0x733670, conv = cdecl, name = "CVisibilityPlugins::RenderWeaponCB", expect = "8B 0D 90 60 8D 00 56 57 8B 7C",
       unverified = "std10-render-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_weapon_cb(atomic: GPtr<u8>) -> GPtr<u8> {
    let frame = rd::<GPtr<u8>>(rd::<GPtr<u8>>(atomic, 0x3C), 4);
    let mi = rd::<GPtr<u8>>(frame, g::<i32>(FRAME_PLUGIN_OFFSET).read() as isize);
    let pos = frame_get_ltm(frame).byte_add(0x30);
    let cam = g::<GPtr<u8>>(CAMERA_POSN).read();
    let dx = X87::from_f32(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(cam, 0));
    let dy = X87::from_f32(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(cam, 4));
    let dz = X87::from_f32(rd::<f32>(pos, 8)).fsub_m(rd::<f32>(cam, 8));
    let r = X87::from_f32(g::<f32>(LOD_DIST_MULT).read()).fmul_m(rd::<f32>(mi, 0x18));
    let sum = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
    if !r.fmul(r).fcom(sum).test_any(0x41) {
        atomic_default_render(atomic);
    }
    atomic
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_RENDER_WEAPON_CB];
