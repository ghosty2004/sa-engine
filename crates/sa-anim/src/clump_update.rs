//! STD-7: `RpAnimBlendClumpUpdateAnimations` (batch `std7-anim-1`): the per-frame animation update of a clump.
//! Ported from the 1.0 US bytes; unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::*;

/// `RpAnimBlendClumpDataOffset`, `gpAnimBlendClump`
const CLUMP_DATA_OFFSET: usize = 0xB5F878;
const ANIM_BLEND_CLUMP: usize = 0xB4EA0C;
/// The frame update callbacks passed to `ForAllFrames`
const CB_COMPRESSED_SKINNED: u32 = 0x4D2E40;
const CB_COMPRESSED_NON_SKINNED: u32 = 0x4D32D0;
const CB_SKINNED: u32 = 0x4D2B90;
const CB_NON_SKINNED: u32 = 0x4D30A0;
const CB_OFFSCREEN: u32 = 0x4D2E10;

orig! {
    /// `CAnimBlendAssociation::UpdateBlend(float timeStep)` (0x4D1490), AL
    fn assoc_update_blend(a: GPtr<u8>, step: f32) -> u8 = thiscall 0x4D1490 => unported;
    /// `CAnimManager::UncompressAnimation(CAnimBlendHierarchy*)` (0x4D41C0)
    fn uncompress_animation(h: GPtr<u8>) = cdecl 0x4D41C0 => unported;
    /// `CAnimBlendAssociation::GetNode(int32)` (0x4CEB60)
    fn assoc_get_node(a: GPtr<u8>, i: i32) -> u32 = thiscall 0x4CEB60 => unported;
    /// `CAnimBlendAssociation::UpdateTime(float timeStep, float timeMult)` (0x4D13D0)
    fn assoc_update_time(a: GPtr<u8>, step: f32, mult: f32) = thiscall 0x4D13D0 => unported;
    /// `GetFirstAtomic(RpClump*)` (0x734820), `RpSkinGeometryGetSkin(RpGeometry*)` (0x7C7550)
    fn get_first_atomic(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734820 => unported;
    fn rp_skin_geometry_get_skin(g: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7550 => unported;
    /// `CAnimBlendClumpData::ForAllFrames(cb, data)` (0x4CF190)
    fn for_all_frames(d: GPtr<u8>, cb: u32, data: GPtr<u8>) = thiscall 0x4CF190 => unported;
    /// `RpAnimBlendNodeUpdateKeyFrames(frames, n)` (0x4D1570, the update data in EAX)
    fn update_key_frames(data: GPtr<u8>, frames: GPtr<u8>, n: i32) = reg("args = eax stack stack") 0x4D1570 => update_key_frames_b;
    /// `RwFrameUpdateObjects(RwFrame*)` (0x7F0910)
    fn rw_frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => unported;
}

/// Phase B side of [`update_key_frames`]: the port reads its EAX argument from `sa_core::abi::ENTRY_EAX`
/// (unused in phase A, where the `reg` call loads EAX).
#[allow(dead_code)]
fn update_key_frames_b(data: GPtr<u8>, frames: GPtr<u8>, n: i32) {
    sa_core::abi::set_entry_eax(data.addr() as u32);
    crate::mvpb_anim::rp_anim_blend_node_update_key_frames(frames, n);
}

/// The skinned test of the update: the clump's first atomic has a skinned geometry.
fn is_skinned(clump: GPtr<u8>) -> bool {
    let a = get_first_atomic(clump);
    !a.is_null() && !rp_skin_geometry_get_skin(rd::<GPtr<u8>>(a, 0x18)).is_null()
}

/// `RpAnimBlendClumpUpdateAnimations(RpClump*, float timeStep, bool onScreen)` (0x4D34F0, cdecl). The clump data
/// (`gpAnimBlendClump` = it); nothing without associations. Pass 1 over the associations (link at +4 of each):
/// `UpdateBlend(step)` false or no sequences (`hierarchy +4`) skip; else `UncompressAnimation`, the first 11 get
/// their node 0 into the update data's node list; with the movement flag (+0x2E bit 5) `length += f32(h.length
/// / speed * blend)` and `blend += blend`, otherwise the update data's "partial" word = 1. Pass 2 sets each running
/// (+0x2E bit 0) association's time step (+0x28): `speed` or, moving, `h.length` (length == 0) /
/// `h.length / length * blendSum`, times `step`. The node list ends with 0; frame 0's flags get bit 7. Frame
/// flags bit 6 (compressed): `ForAllFrames` with the compressed skinned / non-skinned callback; else on screen:
/// bit 5 → `RpAnimBlendNodeUpdateKeyFrames`, then the skinned / non-skinned callback and bit 5 cleared;
/// off screen: the off-screen callback and bit 5 set. Pass 3: `UpdateTime(step, length == 0 ? 1 :
/// f32(blendSum / length))`; then `RwFrameUpdateObjects(clump frame)`. Comparisons with 0 are x87 (`test ah,
/// 0x44; jp`: NaN counts as non-zero).
#[hook(addr = 0x4D34F0, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpUpdateAnimations", expect = "83 EC 3C A1 78 F8 B5 00",
       route = "L2", hazards = "uninit-locals,heap-writes", recs = "grove-walk")]
pub fn rp_anim_blend_clump_update_animations(clump: GPtr<u8>, step: f32, on_screen: u8) {
    let data = rd::<GPtr<u8>>(clump, g::<i32>(CLUMP_DATA_OFFSET).read() as isize);
    g::<GPtr<u8>>(ANIM_BLEND_CLUMP).write(data);
    let mut link = rd::<GPtr<u8>>(data, 0);
    if link.is_null() {
        return;
    }
    // the update data: +0 the "partial" word, +4.. the node list (11 nodes and a terminator)
    let mut update = [0u32; 13];
    let up = GPtr::from_raw(update.as_mut_ptr()).cast::<u8>();
    let mut length = 0f32;
    let mut blend = 0f32;
    let mut n = 0usize;
    while !link.is_null() {
        let a = link.byte_add(-4);
        link = rd::<GPtr<u8>>(link, 0);
        if assoc_update_blend(a, step) == 0 {
            continue;
        }
        let h = rd::<GPtr<u8>>(a, 0x14);
        if rd::<u32>(h, 4) == 0 {
            continue;
        }
        uncompress_animation(h);
        if n < 11 {
            wr::<u32>(up, 4 + 4 * n as isize, assoc_get_node(a, 0));
            n += 1;
        }
        if (rd::<u8>(a, 0x2E) >> 5) & 1 != 0 {
            length = x(rd::<f32>(h, 0x10)).fdiv_m(rd::<f32>(a, 0x24)).fmul_m(rd::<f32>(a, 0x18)).fadd_m(length).to_f32();
            blend = x(blend).fadd_m(rd::<f32>(a, 0x18)).to_f32();
        } else {
            wr::<u32>(up, 0, 1);
        }
    }
    let zero = x(g::<f32>(0x858B50).read());
    let length_is_zero = || !x(length).fcom(zero).test_parity(0x44);
    let mut l = rd::<GPtr<u8>>(data, 0);
    while !l.is_null() {
        let fl = rd::<u8>(l, 0x2A);
        if fl & 1 != 0 {
            let moving = (fl >> 5) & 1 != 0;
            let h = rd::<GPtr<u8>>(l, 0x10);
            let v = if !moving {
                x(rd::<f32>(l, 0x20))
            } else if length_is_zero() {
                x(rd::<f32>(h, 0x10))
            } else {
                x(rd::<f32>(h, 0x10)).fdiv_m(length).fmul_m(blend)
            };
            wr::<f32>(l, 0x24, v.fmul_m(step).to_f32());
        }
        l = rd::<GPtr<u8>>(l, 0);
    }
    wr::<u32>(up, 4 + 4 * n as isize, 0);
    let frames = rd::<GPtr<u8>>(data, 0x10);
    wr::<u8>(frames, 0, rd::<u8>(frames, 0) | 0x80);
    let frames = rd::<GPtr<u8>>(data, 0x10);
    let fl = rd::<u8>(frames, 0);
    let count = rd::<i32>(data, 8);
    let keyframes = (fl >> 5) & 1 != 0;
    if fl & 0x40 != 0 {
        let cb = if is_skinned(clump) { CB_COMPRESSED_SKINNED } else { CB_COMPRESSED_NON_SKINNED };
        for_all_frames(data, cb, up);
    } else if on_screen != 0 {
        if keyframes {
            update_key_frames(up, frames, count);
        }
        let cb = if is_skinned(clump) { CB_SKINNED } else { CB_NON_SKINNED };
        for_all_frames(data, cb, up);
        let f = rd::<GPtr<u8>>(data, 0x10);
        wr::<u8>(f, 0, rd::<u8>(f, 0) & 0xDF);
    } else {
        for_all_frames(data, CB_OFFSCREEN, up);
        let f = rd::<GPtr<u8>>(data, 0x10);
        wr::<u8>(f, 0, rd::<u8>(f, 0) | 0x20);
    }
    let mut l = rd::<GPtr<u8>>(data, 0);
    while !l.is_null() {
        let mult = if length_is_zero() { 1.0 } else { x(blend).fdiv_m(length).to_f32() };
        assoc_update_time(l.byte_add(-4), step, mult);
        l = rd::<GPtr<u8>>(l, 0);
    }
    rw_frame_update_objects(rd::<GPtr<u8>>(clump, 4));
}

pub const HOOKS: &[HookReg] = &[HOOK_RP_ANIM_BLEND_CLUMP_UPDATE_ANIMATIONS];
