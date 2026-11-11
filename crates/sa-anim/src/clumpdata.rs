//! ANIM: `CAnimBlendClumpData` (PORT-ANIM-INPUT). Ported from the 1.0 US bytes (gta-reversed
//! `AnimBlendClumpData.cpp` as a reading aid). See `docs/arch/anim-input-port.md`.
//!
//! Layout (0x14): +0x0 `m_Anims` link (next), +0x4 link prev (pointer to the previous link's next
//! field), +0x8 `m_NumFrames`, +0xC `m_PedPosition`, +0x10 `m_Frames` (`AnimBlendFrameData[]`, 0x18
//! each, 64-byte aligned block).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;

use crate::assoc::{mem_free_align, mem_malloc_align};
use crate::common::{rd, wr};

sa_core::globals! {
    list GLOBALS;
}

orig! {
    /// a `void (*)(AnimBlendFrameData*, void*)` callback (cdecl)
    pub fn call_frame_callback(f: u32, frame: GPtr<u8>, data: u32) = cdecl dyn => unported;
}

/// `CAnimBlendClumpData::CAnimBlendClumpData()` (0x4CF0E0): every field 0; returns `this`.
#[hook(addr = 0x4CF0E0, conv = thiscall, name = "CAnimBlendClumpData::Constructor", expect = "8B C1 33 C9 89 48 08", this = "CAnimBlendClumpData")]
pub fn constructor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0xC, 0);
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0, 0);
    wr::<u32>(this, 4, 0);
    this
}

/// `CAnimBlendClumpData::~CAnimBlendClumpData()` (0x4CF100): unlinks the association list head,
/// clears the link, `CMemoryMgr::FreeAlign(frames)`.
#[hook(addr = 0x4CF100, conv = thiscall, name = "CAnimBlendClumpData::Destructor", expect = "8B 41 04 85 C0", this = "CAnimBlendClumpData")]
pub fn destructor(this: GPtr<u8>) {
    let prev = rd::<GPtr<u8>>(this, 4);
    if !prev.is_null() {
        wr::<u32>(prev, 0, rd::<u32>(this, 0));
    }
    let next = rd::<GPtr<u8>>(this, 0);
    if !next.is_null() {
        wr::<u32>(next, 4, rd::<u32>(this, 4));
    }
    wr::<u32>(this, 0, 0);
    wr::<u32>(this, 4, 0);
    let f = rd::<GPtr<u8>>(this, 0x10);
    if !f.is_null() {
        mem_free_align(f);
    }
}

/// `CAnimBlendClumpData::SetNumberOfBones(int32 n)` (0x4CF140): the old frame block freed, a new
/// `MallocAlign(((n*0x18 - 1) >> 6) + 1) << 6, 0x40, 0)` one; `m_NumFrames = n`.
#[hook(addr = 0x4CF140, conv = thiscall, name = "CAnimBlendClumpData::SetNumberOfBones", expect = "56 8B F1 8B 46 10", this = "CAnimBlendClumpData")]
pub fn set_number_of_bones(this: GPtr<u8>, n: i32) {
    let f = rd::<GPtr<u8>>(this, 0x10);
    if !f.is_null() {
        mem_free_align(f);
    }
    let size = (((n.wrapping_mul(0x18) as u32).wrapping_sub(1) >> 6) + 1) << 6;
    let p = mem_malloc_align(size, 0x40, 0);
    wr::<i32>(this, 8, n);
    wr::<GPtr<u8>>(this, 0x10, p);
}

/// `CAnimBlendClumpData::ForAllFrames(void (*cb)(AnimBlendFrameData*, void*), void* data)` (0x4CF190).
#[hook(addr = 0x4CF190, conv = thiscall, name = "CAnimBlendClumpData::ForAllFrames", expect = "56 8B F1 8B 46 08", this = "CAnimBlendClumpData")]
pub fn for_all_frames(this: GPtr<u8>, cb: u32, data: u32) {
    let mut i = 0;
    while i < rd::<i32>(this, 8) {
        call_frame_callback(cb, rd::<GPtr<u8>>(this, 0x10).byte_add(i as isize * 0x18), data);
        i += 1;
    }
}

/// `CAnimBlendClumpData::LoadFramesIntoSPR()` (0x4CF1D0): `ret` (not hookable).
pub fn load_frames_into_spr(_this: GPtr<u8>) {}

/// `CAnimBlendClumpData::ForAllFramesInSPR(cb, data, n)` (0x4CF1E0): `ret 0xC` (not hookable).
pub fn for_all_frames_in_spr(_this: GPtr<u8>, _cb: u32, _data: u32, _n: u32) {}

pub const HOOKS: &[HookReg] = &[HOOK_CONSTRUCTOR, HOOK_DESTRUCTOR, HOOK_SET_NUMBER_OF_BONES, HOOK_FOR_ALL_FRAMES];
