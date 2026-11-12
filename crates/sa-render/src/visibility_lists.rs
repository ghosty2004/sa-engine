//! The `CVisibilityPlugins` sorted-list renderers (batch `std9-render-6`): `RenderFadingUnderwaterEntities`,
//! `RenderReallyDrawLastObjects`, `RenderAlphaAtomics`, `RenderBoatAlphaAtomics`, `RenderFadingEntities`. Ported from
//! the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
//!
//! A list (sorted by `sorted_list_insert` in `render_lists.rs`): the used nodes run from `[list + 0x20]` to the
//! sentinel at `list`; a node is `{data, render fn, distance, next}` as these loops read it (next at +0xC).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd};

orig! {
    /// A node's render callback (`void (*)(void* data, float distance)`, the distance as its dword)
    fn node_render(f: u32, data: u32, dist: u32) = cdecl dyn => unported;
    /// `G->dOpenDevice.fpRenderStateSet(state, value)` (`[G] + 0x20`)
    fn rw_render_state_set(state: u32, value: u32) -> u32 = cdecl table 0xC97B24 0x20 => unported;
    /// `Light::SetAmbientColours()` (0x735D30), `Light::DeActivateDirectional()` (0x735C70)
    fn set_ambient_colours() = cdecl 0x735D30 => unported;
    fn deactivate_directional() = cdecl 0x735C70 => unported;
    /// `CVisibilityPlugins::RenderBoatAlphaAtomics()` (0x733EC0)
    fn render_boat_alpha_atomics_call() = cdecl 0x733EC0 => render_boat_alpha_atomics;
}

/// `m_alphaUnderwaterEntityList` (0xC88178), `m_alphaReallyDrawLastList` (0xC881D0), `m_alphaList` (0xC88070),
/// `m_alphaBoatAtomicList` (0xC880C8), `m_alphaEntityList` (0xC88120)
const UNDERWATER: usize = 0xC88178;
const REALLY_LAST: usize = 0xC881D0;
const ALPHA: usize = 0xC88070;
const BOAT: usize = 0xC880C8;
const ENTITIES: usize = 0xC88120;

/// Every node from `[list + 0x20]` up to the sentinel `list`: `node.fn(node.data, node.distance)`.
fn render_ordered(list: usize) {
    let mut n = g::<GPtr<u8>>(list + 0x20).read();
    while n.addr() != list {
        node_render(rd::<u32>(n, 4), rd::<u32>(n, 0), rd::<u32>(n, 8));
        n = rd::<GPtr<u8>>(n, 0xC);
    }
}

/// `CVisibilityPlugins::RenderFadingUnderwaterEntities()` (0x7337D0, cdecl).
#[hook(addr = 0x7337D0, conv = cdecl, name = "CVisibilityPlugins::RenderFadingUnderwaterEntities", expect = "56 8B 35 98 81 C8 00",
       unverified = "std9-render-6", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_fading_underwater_entities() {
    render_ordered(UNDERWATER);
}

/// `CVisibilityPlugins::RenderReallyDrawLastObjects()` (0x733800, cdecl): render states (texture raster 0, z-test,
/// z-write, vertex alpha on, src / dest blend 5 / 6, fog on, cull mode 1), `SetAmbientColours`,
/// `DeActivateDirectional`, the list, fog off.
#[hook(addr = 0x733800, conv = cdecl, name = "CVisibilityPlugins::RenderReallyDrawLastObjects", expect = "A1 24 7B C9 00 56 6A 00",
       unverified = "std9-render-6", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_really_draw_last_objects() {
    for (s, v) in [(1, 0), (6, 1), (8, 1), (0xC, 1), (0xA, 5), (0xB, 6), (0xE, 1), (0x14, 1)] {
        rw_render_state_set(s, v);
    }
    set_ambient_colours();
    deactivate_directional();
    render_ordered(REALLY_LAST);
    rw_render_state_set(0xE, 0);
}

/// `CVisibilityPlugins::RenderAlphaAtomics()` (0x733E90, cdecl).
#[hook(addr = 0x733E90, conv = cdecl, name = "CVisibilityPlugins::RenderAlphaAtomics", expect = "56 8B 35 90 80 C8 00",
       unverified = "std9-render-6", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_alpha_atomics() {
    render_ordered(ALPHA);
}

/// `CVisibilityPlugins::RenderBoatAlphaAtomics()` (0x733EC0, cdecl): cull mode 1, the boat list, cull mode 2.
#[hook(addr = 0x733EC0, conv = cdecl, name = "CVisibilityPlugins::RenderBoatAlphaAtomics", expect = "A1 24 7B C9 00 56 6A 01",
       unverified = "std9-render-6", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_boat_alpha_atomics() {
    rw_render_state_set(0x14, 1);
    render_ordered(BOAT);
    rw_render_state_set(0x14, 2);
}

/// `CVisibilityPlugins::RenderFadingEntities()` (0x733F10, cdecl): the entity list, then (tail jump)
/// `RenderBoatAlphaAtomics`.
#[hook(addr = 0x733F10, conv = cdecl, name = "CVisibilityPlugins::RenderFadingEntities", expect = "56 8B 35 40 81 C8 00",
       unverified = "std9-render-6", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_fading_entities() {
    render_ordered(ENTITIES);
    render_boat_alpha_atomics_call();
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RENDER_FADING_UNDERWATER_ENTITIES,
    HOOK_RENDER_REALLY_DRAW_LAST_OBJECTS,
    HOOK_RENDER_ALPHA_ATOMICS,
    HOOK_RENDER_BOAT_ALPHA_ATOMICS,
    HOOK_RENDER_FADING_ENTITIES,
];
