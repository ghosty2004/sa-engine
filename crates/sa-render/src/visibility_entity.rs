//! `CVisibilityPlugins::RenderEntity` and the fading helpers (batch `std9-render-7`): `CalculateFadingAtomicAlpha`,
//! `RenderFadingAtomic`, `RenderFadingClump`. Ported from the 1.0 US bytes (gta-reversed's versions as a reading aid).
//! Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{ftol32, g, rd, wr, x, xi};

orig! {
    /// `G->dOpenDevice.fpRenderStateSet(state, value)` (`[G] + 0x20`)
    fn rw_render_state_set(state: u32, value: u32) -> u32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CVisibilityPlugins::CalculateFadingAtomicAlpha(CBaseModelInfo*, CEntity*, float dist)` (0x732500)
    fn calculate_fading_atomic_alpha_call(mi: GPtr<u8>, e: GPtr<u8>, dist: u32) -> i32 = cdecl 0x732500 => calculate_fading_atomic_alpha;
    /// `CVisibilityPlugins::RenderFadingAtomic(CBaseModelInfo*, RpAtomic*, int32 alpha)` (0x732610)
    fn render_fading_atomic_call(mi: GPtr<u8>, a: GPtr<u8>, alpha: i32) = cdecl 0x732610 => render_fading_atomic;
    /// `CVisibilityPlugins::RenderFadingClump(CBaseModelInfo*, RpClump*, int32 alpha)` (0x732680)
    fn render_fading_clump_call(mi: GPtr<u8>, c: GPtr<u8>, alpha: i32) = cdecl 0x732680 => render_fading_clump;
    /// `CVisibilityPlugins::RenderAtomicWithAlpha(RpAtomic*, int32)` (0x732480)
    fn render_atomic_with_alpha(a: GPtr<u8>, alpha: i32) = cdecl 0x732480 => crate::visibility_ped::render_atomic_with_alpha;
    /// `RpClumpForAllAtomics(clump, cb, data)` (0x749B70)
    fn clump_for_all_atomics(c: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `CRenderer::RenderOneNonRoad(CEntity*)` (0x553260)
    fn render_one_non_road(e: GPtr<u8>) = cdecl 0x553260 => unported;
    /// `CEntity::SetupLighting()` (vtable +0x4C, AL) / `RemoveLighting(bool)` (+0x50)
    fn entity_setup_lighting(e: GPtr<u8>) -> u8 = thiscall vslot 0x4C => unported;
    fn entity_remove_lighting(e: GPtr<u8>, b: u32) = thiscall vslot 0x50 => unported;
}

/// `CModelInfo::ms_modelInfoPtrs`, `CGame::currArea`, `TheCamera.m_fLODDistMultiplier`, the fade distance offset
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
const CURR_AREA: usize = 0xB72914;
const LOD_DIST_MULT: usize = 0xB6F118;
const FADE_OFFSET: usize = 0xB76848;
/// `CVisibilityPlugins::RenderAtomicWithAlphaCB` (0x732660)
const RENDER_ATOMIC_WITH_ALPHA_CB: u32 = 0x732660;

fn c(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

/// `CVisibilityPlugins::CalculateFadingAtomicAlpha(CBaseModelInfo* mi, CEntity* e, float dist)` (0x732500, cdecl):
/// `a = min'(colModel radius + [0xB76848], LOD multiplier * lodDistance)`; without a LOD entity (+0x30) and with
/// `b = min'(lodDistance, a)` above 150 the fade range is `b / 15 + 10` (else 20); flag bit 8 of +0x1C scales `a`
/// by [0x8CD804]; `min'((a + 20 - dist) / range, 1) * alpha byte (+0xC)` through `_ftol`.
#[hook(addr = 0x732500, conv = cdecl, name = "CVisibilityPlugins::CalculateFadingAtomicAlpha", expect = "51 8B 4C 24 08 8B 41 14",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn calculate_fading_atomic_alpha(mi: GPtr<u8>, e: GPtr<u8>, dist: u32) -> i32 {
    let mut range = 20.0f32;
    let r = x(rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x24)).fadd_m(c(FADE_OFFSET));
    let lodm = x(c(LOD_DIST_MULT)).fmul_m(rd::<f32>(mi, 0x18)).to_f32();
    let mut a = if x(lodm).fcom(r).test_parity(0x05) { r } else { x(lodm) };
    if rd::<u32>(e, 0x30) == 0 {
        let lod = x(rd::<f32>(mi, 0x18));
        let b = if !lod.fcom(a).test_parity(0x05) { lod } else { a };
        if !b.fcom(x(c(0x858A28))).test_any(0x41) {
            range = b.fmul_m(c(0x863E0C)).fadd_m(c(0x85862C)).to_f32();
        }
    }
    if rd::<u32>(e, 0x1C) & 0x100 != 0 {
        a = a.fmul_m(c(0x8CD804));
    }
    let mut t = a.fadd_m(c(0x858BA4)).fsub_m(f32::from_bits(dist)).fdiv_m(range);
    if !t.fcom(x(c(0x858624))).test_any(0x41) {
        t = x(c(0x858624));
    }
    ftol32(xi(i32::from(rd::<u8>(mi, 0xC))).fmul(t))
}

/// `CVisibilityPlugins::RenderFadingAtomic(mi, atomic, alpha)` (0x732610, cdecl): additive models (+0x12 bit 2) with
/// dest blend ONE; `RenderAtomicWithAlpha`; dest blend back to INVSRCALPHA.
#[hook(addr = 0x732610, conv = cdecl, name = "CVisibilityPlugins::RenderFadingAtomic", expect = "56 8B 74 24 08 F6 46 12",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_fading_atomic(mi: GPtr<u8>, atomic: GPtr<u8>, alpha: i32) {
    if rd::<u8>(mi, 0x12) & 4 != 0 {
        rw_render_state_set(0xB, 2);
    }
    render_atomic_with_alpha(atomic, alpha);
    if rd::<u8>(mi, 0x12) & 4 != 0 {
        rw_render_state_set(0xB, 6);
    }
}

/// `CVisibilityPlugins::RenderFadingClump(mi, clump, alpha)` (0x732680, cdecl): as `RenderFadingAtomic` over
/// `RpClumpForAllAtomics(clump, RenderAtomicWithAlphaCB, &alpha)`.
#[hook(addr = 0x732680, conv = cdecl, name = "CVisibilityPlugins::RenderFadingClump", expect = "56 8B 74 24 08 F6 46 12",
       unverified = "std9-render-7", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_fading_clump(mi: GPtr<u8>, clump: GPtr<u8>, alpha: i32) {
    if rd::<u8>(mi, 0x12) & 4 != 0 {
        rw_render_state_set(0xB, 2);
    }
    let mut a = alpha;
    clump_for_all_atomics(clump, RENDER_ATOMIC_WITH_ALPHA_CB, GPtr::from_raw(&raw mut a).cast());
    if rd::<u8>(mi, 0x12) & 4 != 0 {
        rw_render_state_set(0xB, 6);
    }
}

/// `CVisibilityPlugins::RenderEntity(void* entity, float dist)` (0x732B40, cdecl): nothing without an RW object.
/// Models without z-write (+0x12 bit 3) turn it off around the draw. A fading entity (+0x1C bit 15): alpha
/// reference 0, the fading alpha, "being rendered" (bit 13) set, culling off unless back-face culled (bit 27),
/// `SetupLighting`, `RenderFadingAtomic` / `RenderFadingClump` by the RW object type, `RemoveLighting`, the flags and
/// culling restored. Otherwise alpha reference 0 (in an interior or without z-write) or 100 and
/// `CRenderer::RenderOneNonRoad`.
#[hook(addr = 0x732B40, conv = cdecl, name = "CVisibilityPlugins::RenderEntity", expect = "56 8B 74 24 08 8B 46 18",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_entity(e: GPtr<u8>, dist: u32) {
    if rd::<u32>(e, 0x18) == 0 {
        return;
    }
    let model = rd::<i16>(e, 0x22) as isize;
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * model as usize).read();
    if rd::<u8>(mi, 0x12) & 8 != 0 {
        rw_render_state_set(8, 0);
    }
    if rd::<u32>(e, 0x1C) & 0x8000 != 0 {
        rw_render_state_set(0x1E, 0);
        let alpha = calculate_fading_atomic_alpha_call(mi, e, dist);
        let f = rd::<u32>(e, 0x1C) | 0x2000;
        wr::<u32>(e, 0x1C, f);
        if f & 0x800_0000 == 0 {
            rw_render_state_set(0x14, 1);
        }
        let lit = entity_setup_lighting(e);
        let obj = rd::<GPtr<u8>>(e, 0x18);
        if rd::<u8>(obj, 0) == 1 {
            render_fading_atomic_call(mi, obj, alpha);
        } else {
            render_fading_clump_call(mi, obj, alpha);
        }
        entity_remove_lighting(e, u32::from(lit));
        let f = rd::<u32>(e, 0x1C) & 0xFFFF_DFFF;
        wr::<u32>(e, 0x1C, f);
        if f & 0x800_0000 == 0 {
            rw_render_state_set(0x14, 2);
        }
    } else {
        let r = if g::<u32>(CURR_AREA).read() != 0 || rd::<u8>(mi, 0x12) & 8 != 0 { 0 } else { 100 };
        rw_render_state_set(0x1E, r);
        render_one_non_road(e);
    }
    if rd::<u8>(mi, 0x12) & 8 != 0 {
        rw_render_state_set(8, 1);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_CALCULATE_FADING_ATOMIC_ALPHA,
    HOOK_RENDER_FADING_ATOMIC,
    HOOK_RENDER_FADING_CLUMP,
    HOOK_RENDER_ENTITY,
];
