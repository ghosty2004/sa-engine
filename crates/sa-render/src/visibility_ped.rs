//! `CVisibilityPlugins::RenderPedCB` and `RenderAtomicWithAlpha` (batch `std9-render-5`): ported from the 1.0 US bytes
//! (gta-reversed's versions as a reading aid). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, wr};

orig! {
    /// `RwFrameGetLTM(frame)` (0x7F0990)
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// `AtomicDefaultRenderCallBack(atomic)` (0x7491C0)
    fn atomic_default_render(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x7491C0 => unported;
    /// `CVisibilityPlugins::RenderAtomicWithAlpha(atomic, alpha)` (0x732480)
    fn render_atomic_with_alpha_call(a: GPtr<u8>, alpha: i32) = cdecl 0x732480 => render_atomic_with_alpha;
}

/// `CVisibilityPlugins::ms_pCameraPosn` (0xC88050), `ms_pedLodDist` (squared, 0xC88034), the clump plugin offset
/// (0x8D6094; the alpha at +4)
const CAMERA_POSN: usize = 0xC88050;
const PED_LOD_DIST: usize = 0xC88034;
const CLUMP_PLUGIN_OFFSET: usize = 0x8D6094;

/// `CVisibilityPlugins::RenderAtomicWithAlpha(RpAtomic*, int32 alpha)` (0x732480, cdecl): the geometry (+0x18) gets
/// flag 0x40 (modulate material colour); each material's alpha (colour +7) is saved and lowered to `alpha` (byte
/// compare, signed against the int); `AtomicDefaultRenderCallBack`; the alphas restored (last first) and the flags.
#[hook(addr = 0x732480, conv = cdecl, name = "CVisibilityPlugins::RenderAtomicWithAlpha", expect = "81 EC 98 00 00 00 53 55",
       route = "L2", hazards = "none", recs = "grove-walk",
       note = "the original's 0x98-byte alpha buffer overflows past 152 materials; the port's holds 256")]
pub fn render_atomic_with_alpha(atomic: GPtr<u8>, alpha: i32) {
    let geo = rd::<GPtr<u8>>(atomic, 0x18);
    let flags = rd::<u32>(geo, 8);
    wr::<u32>(geo, 8, flags | 0x40);
    let mut saved = [0u8; 256];
    let mut n = 0usize;
    while (n as i32) < rd::<i32>(geo, 0x24) {
        let m = rd::<GPtr<u8>>(rd::<GPtr<u8>>(geo, 0x20), 4 * n as isize);
        let a = rd::<u8>(m, 7);
        saved[n & 0xFF] = a;
        if i32::from(a) > alpha {
            wr::<u8>(m, 7, alpha as u8);
        }
        n += 1;
    }
    atomic_default_render(atomic);
    while n != 0 {
        let m = rd::<GPtr<u8>>(rd::<GPtr<u8>>(geo, 0x20), 4 * (n as isize - 1));
        wr::<u8>(m, 7, saved[(n - 1) & 0xFF]);
        n -= 1;
    }
    wr::<u32>(geo, 8, flags);
}

/// `CVisibilityPlugins::RenderPedCB(RpAtomic*)` (0x7335B0, cdecl): the clump (+0x3C) frame's LTM position against the
/// camera, `(dz*dz + dy*dy) + dx*dx` below `ms_pedLodDist` (else nothing): the clump's alpha 255 renders with
/// `AtomicDefaultRenderCallBack`, any other through `RenderAtomicWithAlpha`. Returns the atomic.
#[hook(addr = 0x7335B0, conv = cdecl, name = "CVisibilityPlugins::RenderPedCB", expect = "56 57 8B 7C 24 0C 8B 77 3C",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_ped_cb(atomic: GPtr<u8>) -> GPtr<u8> {
    let clump = rd::<GPtr<u8>>(atomic, 0x3C);
    let m = frame_get_ltm(rd::<GPtr<u8>>(clump, 4)).byte_add(0x30);
    let cam = g::<GPtr<u8>>(CAMERA_POSN).read();
    let d = |o: isize| X87::from_f32(rd::<f32>(m, o)).fsub_m(rd::<f32>(cam, o));
    let (dx, dy, dz) = (d(0), d(4), d(8));
    let s = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    if s.fcom(X87::from_f32(g::<f32>(PED_LOD_DIST).read())).test_parity(0x05) {
        return atomic;
    }
    let off = g::<u32>(CLUMP_PLUGIN_OFFSET).read();
    let alpha = rd::<i32>(clump, off as isize + 4);
    if alpha == 0xFF {
        atomic_default_render(atomic);
    } else {
        render_atomic_with_alpha_call(atomic, alpha);
    }
    atomic
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_RENDER_ATOMIC_WITH_ALPHA, HOOK_RENDER_PED_CB];
