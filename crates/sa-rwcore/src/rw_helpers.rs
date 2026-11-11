//! `RwHelper` "first X" searches (STD-6, batch `std6-rw-1`). Ported from the 1.0 US bytes
//! (0x734850..0x734900).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, p8, rd};

orig! {
    /// `RpClumpForAllAtomics(clump, cb, data)` (0x749B70)
    fn clump_for_all_atomics(c: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => crate::clump::for_all_atomics;
    /// `RwFrameForAllObjects(frame, cb, data)` (0x7F1200)
    fn frame_for_all_objects(f: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x7F1200 => crate::frame::for_all_objects;
    /// `RwFrameForAllChildren(frame, cb, data)` (0x7F0DC0)
    fn frame_for_all_children(f: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x7F0DC0 => crate::frame::for_all_children;
    /// `RpSkinAtomicGetHAnimHierarchy(atomic)` (0x7C7540)
    fn skin_atomic_get_hierarchy(a: GPtr<u8>) -> u32 = cdecl 0x7C7540 => crate::skin::rp_skin_atomic_get_hanim_hierarchy;
    /// `RpSkinGeometryGetSkin(geometry)` (0x7C7550)
    fn skin_geometry_get_skin(g: GPtr<u8>) -> u32 = cdecl 0x7C7550 => crate::skin::rp_skin_geometry_get_skin;
    /// `RpHAnimFrameGetHierarchy(frame)` (0x7C5160)
    fn frame_get_hierarchy(f: GPtr<u8>) -> u32 = cdecl 0x7C5160 => crate::hanim_stream::rp_hanim_frame_get_hierarchy;
    /// `RtAnimAnimationDestroy(anim)` (0x7CCF10)
    fn animation_destroy(a: GPtr<u8>) -> u32 = cdecl 0x7CCF10 => crate::hanim_stream::rt_anim_animation_destroy;
}

/// The 2D-effect geometry plugin offset (`[0xC3A1E0]`).
const EFFECT_2D_OFFSET: usize = 0xC3A1E0;

/// `RwHelper::Get2DEffectAtomicCallback(atomic, data)` (0x734850): an atomic whose geometry carries a
/// 2D-effect block with a non-zero count is stored in `*data` and stops the iteration (NULL); else
/// the atomic.
#[hook(addr = 0x734850, conv = cdecl, name = "RwHelper::Get2DEffectAtomicCallback", expect = "8B 44 24 04 8B 48 18",
       unverified = "std6-rw-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn get_2d_effect_atomic_callback(atomic: GPtr<u8>, data: GPtr<u32>) -> GPtr<u8> {
    let geom = rd::<u32>(atomic, 0x18);
    let fx = g::<u32>((g::<u32>(EFFECT_2D_OFFSET).read().wrapping_add(geom)) as usize).read();
    if fx != 0 && g::<u32>(fx as usize).read() != 0 {
        data.write(atomic.addr() as u32);
        return GPtr::NULL;
    }
    atomic
}

/// `RwHelper::Get2DEffectAtomic(clump)` (0x734880): the first atomic with 2D effects, or NULL.
#[hook(addr = 0x734880, conv = cdecl, name = "RwHelper::Get2DEffectAtomic", expect = "51 8B 4C 24 08 8D 44 24 00",
       unverified = "std6-rw-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn get_2d_effect_atomic(clump: GPtr<u8>) -> GPtr<u8> {
    let mut out = 0u32;
    clump_for_all_atomics(clump, 0x734850, GPtr::from_raw(&mut out as *mut u32).addr() as u32);
    p8(out)
}

/// `RwHelper::GetFirstObjectCallback(object, data)` (0x7348B0): `*data = object`, stops (NULL).
#[hook(addr = 0x7348B0, conv = cdecl, name = "RwHelper::GetFirstObjectCallback", expect = "8B 44 24 08 8B 4C 24 04 89 08",
       unverified = "std6-rw-1", route = "L1-pure", hazards = "none")]
pub fn get_first_object_callback(object: GPtr<u8>, data: GPtr<u32>) -> GPtr<u8> {
    data.write(object.addr() as u32);
    GPtr::NULL
}

/// `RwHelper::GetFirstObject(frame)` (0x7348C0): the frame's first object, or NULL.
#[hook(addr = 0x7348C0, conv = cdecl, name = "RwHelper::GetFirstObject", expect = "51 8B 4C 24 08 8D 44 24 00",
       unverified = "std6-rw-1", route = "L2", hazards = "none")]
pub fn get_first_object(frame: GPtr<u8>) -> GPtr<u8> {
    let mut out = 0u32;
    frame_for_all_objects(frame, 0x7348B0, GPtr::from_raw(&mut out as *mut u32).addr() as u32);
    p8(out)
}

/// `RwHelper::GetFirstFrameCallback(frame, data)` (0x7348F0): `*data = frame`, stops (NULL).
#[hook(addr = 0x7348F0, conv = cdecl, name = "RwHelper::GetFirstFrameCallback", expect = "8B 44 24 08 8B 4C 24 04 89 08",
       unverified = "std6-rw-1", route = "L1-pure", hazards = "none")]
pub fn get_first_frame_callback(frame: GPtr<u8>, data: GPtr<u32>) -> GPtr<u8> {
    data.write(frame.addr() as u32);
    GPtr::NULL
}

/// `RwHelper::GetFirstChild(frame)` (0x734900): the frame's first child, or NULL.
#[hook(addr = 0x734900, conv = cdecl, name = "RwHelper::GetFirstChild", expect = "51 8B 4C 24 08 8D 44 24 00",
       unverified = "std6-rw-1", route = "L2", hazards = "none")]
pub fn get_first_child(frame: GPtr<u8>) -> GPtr<u8> {
    let mut out = 0u32;
    frame_for_all_children(frame, 0x7348F0, GPtr::from_raw(&mut out as *mut u32).addr() as u32);
    p8(out)
}

/// `RwHelper::SkinAtomicGetHAnimHierarchCB(atomic, data)` (0x734A20): `*data =
/// RpSkinAtomicGetHAnimHierarchy(atomic)`, stops (NULL).
#[hook(addr = 0x734A20, conv = cdecl, name = "RwHelper::SkinAtomicGetHAnimHierarchCB", expect = "8B 44 24 04 50 E8 16 2B 09 00",
       unverified = "std6-rw-1", route = "L2", hazards = "none")]
pub fn skin_atomic_get_hanim_hierarchy_cb(atomic: GPtr<u8>, data: GPtr<u32>) -> GPtr<u8> {
    data.write(skin_atomic_get_hierarchy(atomic));
    GPtr::NULL
}

/// `RwHelper::GetAnimHierarchyFromSkinClump(clump)` (0x734A40).
#[hook(addr = 0x734A40, conv = cdecl, name = "RwHelper::GetAnimHierarchyFromSkinClump", expect = "51 8B 4C 24 08 8D 44 24 00",
       unverified = "std6-rw-1", route = "L2", hazards = "none")]
pub fn get_anim_hierarchy_from_skin_clump(clump: GPtr<u8>) -> u32 {
    let mut out = 0u32;
    clump_for_all_atomics(clump, 0x734A20, GPtr::from_raw(&mut out as *mut u32).addr() as u32);
    out
}

/// The `GetAnimHierarchyFromFrame` child callback (0x734A70): a frame with a hierarchy stores it and
/// stops (NULL); otherwise its children are searched (the same callback) and the frame returned.
#[hook(addr = 0x734A70, conv = cdecl, name = "FUN_00734a70", expect = "56 8B 74 24 08 56 E8",
       unverified = "std6-rw-1", route = "L2", hazards = "none")]
pub fn anim_hierarchy_from_frame_cb(frame: GPtr<u8>, data: GPtr<u32>) -> GPtr<u8> {
    let h = frame_get_hierarchy(frame);
    if h == 0 {
        frame_for_all_children(frame, 0x734A70, data.addr() as u32);
        return frame;
    }
    data.write(h);
    GPtr::NULL
}

/// `RwHelper::GetAnimHierarchyFromFrame(frame)` (0x734AB0): the frame's hierarchy, else the first in
/// its descendants (searched twice when the first search finds none, as the original does).
#[hook(addr = 0x734AB0, conv = cdecl, name = "RwHelper::GetAnimHierarchyFromFrame", expect = "51 56 8B 74 24 0C 56",
       unverified = "std6-rw-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn get_anim_hierarchy_from_frame(frame: GPtr<u8>) -> u32 {
    let mut out = frame_get_hierarchy(frame);
    if out == 0 {
        frame_for_all_children(frame, 0x734A70, GPtr::from_raw(&mut out as *mut u32).addr() as u32);
    }
    if out == 0 {
        frame_for_all_children(frame, 0x734A70, GPtr::from_raw(&mut out as *mut u32).addr() as u32);
    }
    out
}

/// `RwHelper::GetAnimHierarchyFromClump(clump)` (0x734B10): from the clump's frame.
#[hook(addr = 0x734B10, conv = cdecl, name = "RwHelper::GetAnimHierarchyFromClump", expect = "8B 44 24 04 8B 48 04 89 4C 24 04",
       unverified = "std6-rw-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn get_anim_hierarchy_from_clump(clump: GPtr<u8>) -> u32 {
    get_anim_hierarchy_from_frame(p8(rd::<u32>(clump, 4)))
}

/// `RwHelper::AtomicRemoveAnimFromSkinCB(atomic, data)` (0x734B90): for a skinned atomic with a
/// hierarchy, the current interpolator's animation (+0x20 -> +0) is destroyed and cleared. Returns
/// the atomic.
#[hook(addr = 0x734B90, conv = cdecl, name = "RwHelper::AtomicRemoveAnimFromSkinCB", expect = "56 57 8B 7C 24 0C 8B 47 18",
       unverified = "std6-rw-1", route = "L2", hazards = "heap-writes")]
pub fn atomic_remove_anim_from_skin_cb(atomic: GPtr<u8>, _data: u32) -> GPtr<u8> {
    if skin_geometry_get_skin(p8(rd::<u32>(atomic, 0x18))) != 0 {
        let h = skin_atomic_get_hierarchy(atomic);
        if h != 0 {
            let interp = p8(rd::<u32>(p8(h), 0x20));
            let anim = rd::<u32>(interp, 0);
            if anim != 0 {
                animation_destroy(p8(anim));
            }
            crate::common::wr::<u32>(p8(rd::<u32>(p8(h), 0x20)), 0, 0);
        }
    }
    atomic
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_GET_2D_EFFECT_ATOMIC_CALLBACK,
    HOOK_GET_2D_EFFECT_ATOMIC,
    HOOK_GET_FIRST_OBJECT_CALLBACK,
    HOOK_GET_FIRST_OBJECT,
    HOOK_GET_FIRST_FRAME_CALLBACK,
    HOOK_GET_FIRST_CHILD,
    HOOK_SKIN_ATOMIC_GET_HANIM_HIERARCHY_CB,
    HOOK_GET_ANIM_HIERARCHY_FROM_SKIN_CLUMP,
    HOOK_ANIM_HIERARCHY_FROM_FRAME_CB,
    HOOK_GET_ANIM_HIERARCHY_FROM_FRAME,
    HOOK_GET_ANIM_HIERARCHY_FROM_CLUMP,
    HOOK_ATOMIC_REMOVE_ANIM_FROM_SKIN_CB,
];
