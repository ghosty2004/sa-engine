//! ANIM: `CAnimBlendAssociation` and `CAnimBlendStaticAssociation` (PORT-ANIM-INPUT). Ported from the
//! 1.0 US bytes (gta-reversed `AnimBlendAssociation.cpp`, `AnimBlendStaticAssociation.cpp` as reading
//! aids, `docs/re/specs/ANIM/CAnimBlend*Association*.md`). See `docs/arch/anim-input-port.md`.
//!
//! `CAnimBlendAssociation` (0x3C): +0x0 vtable (0x85C6D0), +0x4/+0x8 `m_Link` (next/prev), +0xC
//! `m_NumBlendNodes` (int16), +0xE `m_AnimGroupId`, +0x10 `m_BlendNodes` (`CAnimBlendNode[]`, 0x18
//! each, 64-byte aligned block), +0x14 `m_BlendHier`, +0x18 `m_BlendAmount`, +0x1C `m_BlendDelta`,
//! +0x20 `m_CurrentTime`, +0x24 `m_Speed`, +0x28 `m_TimeStep`, +0x2C `m_AnimId`, +0x2E `m_Flags`
//! (u16), +0x30 `m_nCallbackType`, +0x34 `m_pCallbackFunc`, +0x38 `m_pCallbackData`.
//! `CAnimBlendStaticAssociation` (0x14): +0x0 vtable (0x85C6CC), +0x4 `m_NumBlendNodes` (int16), +0x6
//! `m_AnimId`, +0x8 `m_AnimGroupId`, +0xA `m_Flags`, +0xC `m_BlendSeqs` (`CAnimBlendSequence*[]`),
//! +0x10 `m_BlendHier`.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;

use crate::common::{g, rd, wr, x};
use crate::sequence::{self, FLAG_BONE_TAG, crt_free, mem_free, mem_malloc};

sa_core::globals! {
    list GLOBALS;
}

/// `CAnimBlendAssociation` vtable.
pub const VTBL_ASSOC: u32 = 0x85C6D0;
/// `CAnimBlendStaticAssociation` vtable.
pub const VTBL_STATIC: u32 = 0x85C6CC;
/// `ClumpOffset` (RpAnimBlend plugin offset of `CAnimBlendClumpData*` in an `RpClump`).
pub const CLUMP_OFFSET: usize = 0xB5F878;

orig! {
    /// `CMemoryMgr::MallocAlign(size, align, hint)` (0x72F4C0)
    pub fn mem_malloc_align(size: u32, align: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F4C0 => unported;
    /// `CMemoryMgr::FreeAlign(p)` (0x72F4F0)
    pub fn mem_free_align(p: GPtr<u8>) = cdecl 0x72F4F0 => unported;
    /// `CAnimBlendNode::Init()` (0x4CFB70)
    pub fn node_init(this: GPtr<u8>) = thiscall 0x4CFB70 => crate::node::init;
    /// `CAnimBlendNode::SetupKeyFrameCompressed()` (0x4D0650)
    pub fn node_setup_key_frame_compressed(this: GPtr<u8>) -> u8 = thiscall 0x4D0650 => crate::node::setup_key_frame_compressed;
    /// `CAnimBlendNode::FindKeyFrame(float)` (0x4D0240)
    pub fn node_find_key_frame(this: GPtr<u8>, t: f32) -> u8 = thiscall 0x4D0240 => crate::node::find_key_frame;
    /// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0)
    pub fn add_anim_block_ref(i: i32) = cdecl 0x4D3FB0 => crate::manager::add_anim_block_ref;
    /// `CAnimManager::UncompressAnimation(CAnimBlendHierarchy*)` (0x4D41C0)
    pub fn uncompress_animation(h: GPtr<u8>) = cdecl 0x4D41C0 => crate::manager::uncompress_animation;
    /// `RpAnimBlendClumpFindBone(RpClump*, uint32 boneTag)` (0x4D6400)
    pub fn clump_find_bone(clump: GPtr<u8>, tag: i32) -> GPtr<u8> = cdecl 0x4D6400 => unported;
    /// `RpAnimBlendClumpFindFrameFromHashKey(RpClump*, uint32 key)` (0x4D6370)
    pub fn clump_find_frame_from_hash_key(clump: GPtr<u8>, key: u32) -> GPtr<u8> = cdecl 0x4D6370 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80)
    pub fn call_set_current_time(this: GPtr<u8>, t: f32) = thiscall 0x4CEA80 => set_current_time;
    /// `CAnimBlendAssociation::AllocateAnimBlendNodeArray(int32)` (0x4CE9F0)
    pub fn call_allocate_anim_blend_node_array(this: GPtr<u8>, n: i32) = thiscall 0x4CE9F0 => allocate_anim_blend_node_array;
    /// `CAnimBlendAssociation::Init(RpClump*, CAnimBlendHierarchy*)` (0x4CED50)
    pub fn call_init_clump(this: GPtr<u8>, clump: GPtr<u8>, h: GPtr<u8>) = thiscall 0x4CED50 => init_clump;
    /// `CAnimBlendAssociation::Init(CAnimBlendAssociation&)` (0x4CEE40)
    pub fn call_init_assoc(this: GPtr<u8>, src: GPtr<u8>) = thiscall 0x4CEE40 => init_assoc;
    /// `CAnimBlendAssociation::Init(CAnimBlendStaticAssociation&)` (0x4CEEC0)
    pub fn call_init_static(this: GPtr<u8>, src: GPtr<u8>) = thiscall 0x4CEEC0 => init_static;
    /// `CAnimBlendStaticAssociation::AllocateSequenceArray(int32)` (0x4CE960)
    pub fn call_allocate_sequence_array(this: GPtr<u8>, n: i32) = thiscall 0x4CE960 => allocate_sequence_array;
    /// `CAnimBlendStaticAssociation::Init(RpClump*, CAnimBlendHierarchy*)` (0x4CEC20)
    pub fn call_static_init(this: GPtr<u8>, clump: GPtr<u8>, h: GPtr<u8>) = thiscall 0x4CEC20 => static_init;
    /// `CAnimBlendStaticAssociation::~CAnimBlendStaticAssociation()` (0x4CEC00)
    pub fn call_static_dtor(this: GPtr<u8>) = thiscall 0x4CEC00 => static_dtor;
}

#[inline]
fn flags(a: GPtr<u8>) -> u16 {
    rd::<u16>(a, 0x2E)
}
#[inline]
fn hier(a: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(a, 0x14)
}
#[inline]
fn num_nodes(a: GPtr<u8>) -> i16 {
    rd::<i16>(a, 0xC)
}

/// `CAnimBlendAssociation::CAnimBlendAssociation()` (0x4CE9B0): every field but the node count and
/// the callback pointers (gta-reversed also nulls those); returns `this`.
#[hook(addr = 0x4CE9B0, conv = thiscall, name = "CAnimBlendAssociation::Constructor0-", expect = "8B C1 33 C9 BA 00 00 80 3F", this = "CAnimBlendAssociation")]
pub fn constructor0(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, VTBL_ASSOC);
    wr::<i16>(this, 0xE, -1);
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0x14, 0);
    wr::<f32>(this, 0x18, 1.0);
    wr::<u32>(this, 0x1C, 0);
    wr::<u32>(this, 0x20, 0);
    wr::<f32>(this, 0x24, 1.0);
    wr::<u32>(this, 0x28, 0);
    wr::<i16>(this, 0x2C, -1);
    wr::<u16>(this, 0x2E, 0);
    wr::<u32>(this, 0x30, 0);
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0);
    this
}

/// The common field setup of the three non-default constructors (everything but the anim id, group
/// id and flags, which `Init` sets).
fn construct_common(this: GPtr<u8>) {
    wr::<u32>(this, 0x10, 0);
    wr::<f32>(this, 0x18, 1.0);
    wr::<u32>(this, 0x1C, 0);
    wr::<u32>(this, 0x20, 0);
    wr::<f32>(this, 0x24, 1.0);
    wr::<u32>(this, 0x28, 0);
    wr::<u32>(this, 0x30, 0);
    wr::<u32>(this, 0, VTBL_ASSOC);
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0);
}

/// `ReferenceAnimBlock` as the constructors inline it.
fn reference_block(this: GPtr<u8>) {
    if flags(this) & 0x4000 == 0 {
        add_anim_block_ref(rd::<i32>(hier(this), 0xC));
        wr::<u8>(this, 0x2F, rd::<u8>(this, 0x2F) | 0x40);
    }
}

/// `CAnimBlendAssociation::AllocateAnimBlendNodeArray(int32 n)` (0x4CE9F0): a
/// `CMemoryMgr::MallocAlign(((n*0x18 - 1) >> 6) + 1) << 6, 0x40, 0)` block, every node `Init`.
#[hook(addr = 0x4CE9F0, conv = thiscall, name = "CAnimBlendAssociation::AllocateAnimBlendNodeArray", expect = "53 57 8B 7C 24 0C", this = "CAnimBlendAssociation")]
pub fn allocate_anim_blend_node_array(this: GPtr<u8>, n: i32) {
    let size = (((n.wrapping_mul(0x18) as u32).wrapping_sub(1) >> 6) + 1) << 6;
    let p = mem_malloc_align(size, 0x40, 0);
    wr::<GPtr<u8>>(this, 0x10, p);
    let mut i = 0;
    while i < n {
        node_init(rd::<GPtr<u8>>(this, 0x10).byte_add(i as isize * 0x18));
        i += 1;
    }
}

/// `CAnimBlendAssociation::FreeAnimBlendNodeArray()` (0x4CEA40): `CMemoryMgr::FreeAlign(nodes)`.
#[hook(addr = 0x4CEA40, conv = thiscall, name = "CAnimBlendAssociation::FreeAnimBlendNodeArray", expect = "8B 41 10 85 C0", this = "CAnimBlendAssociation")]
pub fn free_anim_blend_node_array(this: GPtr<u8>) {
    let p = rd::<GPtr<u8>>(this, 0x10);
    if !p.is_null() {
        mem_free_align(p);
    }
}

/// `CAnimBlendAssociation::ReferenceAnimBlock()` (0x4CEA50): once (flag 0x4000), `AddAnimBlockRef` of
/// the hierarchy's block.
#[hook(addr = 0x4CEA50, conv = thiscall, name = "CAnimBlendAssociation::ReferenceAnimBlock", expect = "56 33 C0 8B F1", this = "CAnimBlendAssociation")]
pub fn reference_anim_block(this: GPtr<u8>) {
    reference_block(this);
}

/// `CAnimBlendAssociation::SetCurrentTime(float t)` (0x4CEA80): at or past the end a looped animation
/// wraps by repeated subtraction (never returns for a 0 total time with t >= 0, as the original),
/// else it is clamped to the total; then `UncompressAnimation` and every node with a sequence is
/// re-seeked (`SetupKeyFrameCompressed` when kept compressed, else `FindKeyFrame(time)`).
#[hook(addr = 0x4CEA80, conv = thiscall, name = "CAnimBlendAssociation::SetCurrentTime", expect = "8B 44 24 04 D9 44 24 04", this = "CAnimBlendAssociation")]
pub fn set_current_time(this: GPtr<u8>, t: f32) {
    let h = hier(this);
    wr::<f32>(this, 0x20, t);
    if !x(t).fcom(x(rd::<f32>(h, 0x10))).c0() {
        if flags(this) & 2 != 0 {
            loop {
                let c = x(rd::<f32>(this, 0x20)).fsub_m(rd::<f32>(h, 0x10)).to_f32();
                wr::<f32>(this, 0x20, c);
                let tot = x(rd::<f32>(hier(this), 0x10));
                if tot.fcom(x(c)).test_parity(0x41) {
                    break;
                }
            }
        } else {
            wr::<u32>(this, 0x20, rd::<u32>(h, 0x10));
        }
    }
    uncompress_animation(h);
    let keep = rd::<u8>(hier(this), 0xB) != 0;
    let mut i = 0;
    while i < i32::from(num_nodes(this)) {
        let node = rd::<GPtr<u8>>(this, 0x10).byte_add(i as isize * 0x18);
        if !rd::<GPtr<u8>>(node, 0x10).is_null() {
            if keep {
                node_setup_key_frame_compressed(node);
            } else {
                node_find_key_frame(node, rd::<f32>(this, 0x20));
            }
        }
        i += 1;
    }
}

/// `CAnimBlendAssociation::SyncAnimation(CAnimBlendAssociation* other)` (0x4CEB40):
/// `SetCurrentTime(other->time / other->total * total)`.
#[hook(addr = 0x4CEB40, conv = thiscall, name = "CAnimBlendAssociation::SyncAnimation", expect = "8B 44 24 04 D9 40 20", this = "CAnimBlendAssociation")]
pub fn sync_animation(this: GPtr<u8>, other: GPtr<u8>) {
    let t = x(rd::<f32>(other, 0x20))
        .fdiv_m(rd::<f32>(hier(other), 0x10))
        .fmul_m(rd::<f32>(hier(this), 0x10))
        .to_f32();
    call_set_current_time(this, t);
}

/// `CAnimBlendAssociation::GetNode(int32 i)` (0x4CEB60): `m_BlendNodes + i * 0x18`.
#[hook(addr = 0x4CEB60, conv = thiscall, name = "CAnimBlendAssociation::GetNode", expect = "8B 44 24 04 8B 49 10", this = "CAnimBlendAssociation")]
pub fn get_node(this: GPtr<u8>, i: i32) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0x10).byte_add((i as isize).wrapping_mul(0x18))
}

/// `CAnimBlendAssociation::Start(float t)` (0x4CEB70): PLAYING, then `SetCurrentTime(t)` (tail jump).
#[hook(addr = 0x4CEB70, conv = thiscall, name = "CAnimBlendAssociation::Start", expect = "80 49 2E 01 E9 07 FF FF FF", this = "CAnimBlendAssociation")]
pub fn start(this: GPtr<u8>, t: f32) {
    wr::<u8>(this, 0x2E, rd::<u8>(this, 0x2E) | 1);
    call_set_current_time(this, t);
}

/// `CAnimBlendAssociation::SetBlendTo(float target, float delta)` (0x4CEB80):
/// `m_BlendDelta = (target - m_BlendAmount) * delta`.
#[hook(addr = 0x4CEB80, conv = thiscall, name = "CAnimBlendAssociation::SetBlendTo", expect = "D9 44 24 04 D8 61 18", this = "CAnimBlendAssociation")]
pub fn set_blend_to(this: GPtr<u8>, target: f32, delta: f32) {
    wr::<f32>(this, 0x1C, x(target).fsub_m(rd::<f32>(this, 0x18)).fmul_m(delta).to_f32());
}

/// `CAnimBlendAssociation::SetBlend(float amount, float delta)` (0x4CEBA0): raw stores.
#[hook(addr = 0x4CEBA0, conv = thiscall, name = "CAnimBlendAssociation::SetBlend", expect = "8B 44 24 04 8B 54 24 08", this = "CAnimBlendAssociation")]
pub fn set_blend(this: GPtr<u8>, amount: u32, delta: u32) {
    wr::<u32>(this, 0x18, amount);
    wr::<u32>(this, 0x1C, delta);
}

/// `CAnimBlendAssociation::SetDeleteCallback(fn, data)` (0x4CEBC0): type 2.
#[hook(addr = 0x4CEBC0, conv = thiscall, name = "CAnimBlendAssociation::SetDeleteCallback", expect = "8B 44 24 04 8B 54 24 08", this = "CAnimBlendAssociation")]
pub fn set_delete_callback(this: GPtr<u8>, f: u32, data: u32) {
    wr::<u32>(this, 0x30, 2);
    wr::<u32>(this, 0x34, f);
    wr::<u32>(this, 0x38, data);
}

/// `CAnimBlendAssociation::SetFinishCallback(fn, data)` (0x4CEBE0): type 1.
#[hook(addr = 0x4CEBE0, conv = thiscall, name = "CAnimBlendAssociation::SetFinishCallback", expect = "8B 44 24 04 8B 54 24 08", this = "CAnimBlendAssociation")]
pub fn set_finish_callback(this: GPtr<u8>, f: u32, data: u32) {
    wr::<u32>(this, 0x30, 1);
    wr::<u32>(this, 0x34, f);
    wr::<u32>(this, 0x38, data);
}

/// The frame of a sequence in a clump (by bone tag when the sequence has one, else by name hash).
fn find_frame(clump: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> {
    if sequence::flags(s) & FLAG_BONE_TAG != 0 {
        clump_find_bone(clump, i32::from(rd::<i16>(s, 0)))
    } else {
        clump_find_frame_from_hash_key(clump, rd::<u32>(s, 0))
    }
}

/// `(frame - clumpData->frames) / sizeof(AnimBlendFrameData)` (MSVC signed division by 0x18).
fn frame_index(frame: GPtr<u8>, cd: GPtr<u8>) -> i32 {
    (frame.addr() as u32).wrapping_sub(rd::<u32>(cd, 0x10)) as i32 / 0x18
}

/// `CAnimBlendAssociation::Init(RpClump*, CAnimBlendHierarchy*)` (0x4CED50): one node per clump frame
/// (each pointing back here), every sequence with frames attached to the node of its clump frame.
#[hook(addr = 0x4CED50, conv = thiscall, name = "CAnimBlendAssociation::Init-0", expect = "51 A1 78 F8 B5 00", this = "CAnimBlendAssociation")]
pub fn init_clump(this: GPtr<u8>, clump: GPtr<u8>, h: GPtr<u8>) {
    let cd = rd::<GPtr<u8>>(clump, g::<i32>(CLUMP_OFFSET).read() as isize);
    let nf = rd::<u32>(cd, 8);
    wr::<u16>(this, 0xC, nf as u16);
    call_allocate_anim_blend_node_array(this, i32::from(nf as u16 as i16));
    let mut i = 0;
    while i < i32::from(num_nodes(this)) {
        wr::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), i as isize * 0x18 + 0x14, this);
        i += 1;
    }
    wr::<GPtr<u8>>(this, 0x14, h);
    let mut j = 0;
    while j < i32::from(rd::<i16>(h, 8)) {
        let s = rd::<GPtr<u8>>(h, 4).byte_add(j as isize * 0xC);
        let f = find_frame(clump, s);
        if !f.is_null() && sequence::num_frames(s) > 0 {
            let k = frame_index(f, cd);
            wr::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), k as isize * 0x18 + 0x10, s);
        }
        j += 1;
    }
}

/// `CAnimBlendAssociation::Init(CAnimBlendAssociation& src)` (0x4CEE40): copies the hierarchy, node
/// count, flags, anim and group ids, then a new node array with the source's nodes copied (0x18 bytes
/// each) and re-pointed here.
#[hook(addr = 0x4CEE40, conv = thiscall, name = "CAnimBlendAssociation::Init-1", expect = "53 8B D9 55 8B 6C 24 0C", this = "CAnimBlendAssociation")]
pub fn init_assoc(this: GPtr<u8>, src: GPtr<u8>) {
    wr::<u32>(this, 0x14, rd::<u32>(src, 0x14));
    wr::<u16>(this, 0xC, rd::<u16>(src, 0xC));
    wr::<u16>(this, 0x2E, rd::<u16>(src, 0x2E));
    wr::<u16>(this, 0x2C, rd::<u16>(src, 0x2C));
    wr::<u16>(this, 0xE, rd::<u16>(src, 0xE));
    call_allocate_anim_blend_node_array(this, i32::from(num_nodes(this)));
    let mut i = 0;
    while i < i32::from(num_nodes(this)) {
        let off = i as isize * 0x18;
        let s = rd::<GPtr<u8>>(src, 0x10).byte_add(off);
        let d = rd::<GPtr<u8>>(this, 0x10).byte_add(off);
        for k in 0..6 {
            wr::<u32>(d, 4 * k, rd::<u32>(s, 4 * k));
        }
        wr::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), off + 0x14, this);
        i += 1;
    }
}

/// `CAnimBlendAssociation::Init(CAnimBlendStaticAssociation& src)` (0x4CEEC0): hierarchy, counts,
/// ids and flags from the static association; one node per sequence slot, pointing back here.
#[hook(addr = 0x4CEEC0, conv = thiscall, name = "CAnimBlendAssociation::Init-2", expect = "56 8B F1 57 8B 7C 24 0C", this = "CAnimBlendAssociation")]
pub fn init_static(this: GPtr<u8>, src: GPtr<u8>) {
    wr::<u32>(this, 0x14, rd::<u32>(src, 0x10));
    wr::<u16>(this, 0xC, rd::<u16>(src, 4));
    wr::<u16>(this, 0x2E, rd::<u16>(src, 0xA));
    wr::<u16>(this, 0x2C, rd::<u16>(src, 6));
    wr::<u16>(this, 0xE, rd::<u16>(src, 8));
    call_allocate_anim_blend_node_array(this, i32::from(num_nodes(this)));
    let mut i = 0;
    while i < i32::from(num_nodes(this)) {
        let s = rd::<u32>(rd::<GPtr<u8>>(src, 0xC), i as isize * 4);
        let off = i as isize * 0x18;
        wr::<u32>(rd::<GPtr<u8>>(this, 0x10), off + 0x10, s);
        wr::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), off + 0x14, this);
        i += 1;
    }
}

/// `CAnimBlendAssociation::CAnimBlendAssociation(RpClump*, CAnimBlendHierarchy*)` (0x4CEFC0): field
/// defaults, `Init(clump, hier)`; returns `this` (no block reference here).
#[hook(addr = 0x4CEFC0, conv = thiscall, name = "CAnimBlendAssociation::Constructor1-clump_hier", expect = "33 C0 56 8B F1", this = "CAnimBlendAssociation")]
pub fn constructor1(this: GPtr<u8>, clump: GPtr<u8>, h: GPtr<u8>) -> GPtr<u8> {
    wr::<f32>(this, 0x18, 1.0);
    wr::<f32>(this, 0x24, 1.0);
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0x1C, 0);
    wr::<u32>(this, 0x20, 0);
    wr::<u32>(this, 0x28, 0);
    wr::<u16>(this, 0x2E, 0);
    wr::<u32>(this, 0x30, 0);
    wr::<u32>(this, 0, VTBL_ASSOC);
    wr::<i16>(this, 0xE, -1);
    wr::<i16>(this, 0x2C, -1);
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0);
    call_init_clump(this, clump, h);
    this
}

/// `CAnimBlendAssociation::CAnimBlendAssociation(CAnimBlendAssociation&)` (0x4CF020): field defaults,
/// `Init(src)`, `ReferenceAnimBlock` (inlined); returns `this`.
#[hook(addr = 0x4CF020, conv = thiscall, name = "CAnimBlendAssociation::Constructor2-blend", expect = "33 C0 56 8B F1", this = "CAnimBlendAssociation")]
pub fn constructor2(this: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    construct_common(this);
    call_init_assoc(this, src);
    reference_block(this);
    this
}

/// `CAnimBlendAssociation::CAnimBlendAssociation(CAnimBlendStaticAssociation&)` (0x4CF080): as the
/// copy constructor from a static association.
#[hook(addr = 0x4CF080, conv = thiscall, name = "CAnimBlendAssociation::Constructor3-static_blend", expect = "33 C0 56 8B F1", this = "CAnimBlendAssociation")]
pub fn constructor3(this: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    construct_common(this);
    call_init_static(this, src);
    reference_block(this);
    this
}

// ---- CAnimBlendStaticAssociation ---------------------------------------------------------------------

/// `CAnimBlendStaticAssociation::CAnimBlendStaticAssociation()` (0x4CE940): ids -1, flags/sequences/
/// hierarchy 0 (the node count is not written); returns `this`.
#[hook(addr = 0x4CE940, conv = thiscall, name = "CAnimBlendStaticAssociation::Constructor1", expect = "8B C1 83 C9 FF", this = "CAnimBlendStaticAssociation")]
pub fn static_constructor1(this: GPtr<u8>) -> GPtr<u8> {
    wr::<i16>(this, 6, -1);
    wr::<i16>(this, 8, -1);
    wr::<u32>(this, 0, VTBL_STATIC);
    wr::<u16>(this, 0xA, 0);
    wr::<u32>(this, 0xC, 0);
    wr::<u32>(this, 0x10, 0);
    this
}

/// `CAnimBlendStaticAssociation::AllocateSequenceArray(int32 n)` (0x4CE960): `CMemoryMgr::Malloc(n*4)`,
/// zeroed.
#[hook(addr = 0x4CE960, conv = thiscall, name = "CAnimBlendStaticAssociation::AllocateSequenceArray", expect = "56 57 8B 7C 24 0C", this = "CAnimBlendStaticAssociation")]
pub fn allocate_sequence_array(this: GPtr<u8>, n: i32) {
    let p = mem_malloc((n as u32).wrapping_mul(4), 0);
    wr::<GPtr<u8>>(this, 0xC, p);
    let mut i = 0;
    while i < n {
        wr::<u32>(rd::<GPtr<u8>>(this, 0xC), i as isize * 4, 0);
        i += 1;
    }
}

/// `CAnimBlendStaticAssociation::FreeSequenceArray()` (0x4CE9A0): `CMemoryMgr::Free` (field kept).
#[hook(addr = 0x4CE9A0, conv = thiscall, name = "CAnimBlendStaticAssociation::FreeSequenceArray", expect = "8B 41 0C 85 C0", this = "CAnimBlendStaticAssociation")]
pub fn free_sequence_array(this: GPtr<u8>) {
    let p = rd::<GPtr<u8>>(this, 0xC);
    if !p.is_null() {
        mem_free(p);
    }
}

/// `CAnimBlendStaticAssociation::~CAnimBlendStaticAssociation()` (0x4CEC00): vtable, sequence array
/// freed (field kept).
#[hook(addr = 0x4CEC00, conv = thiscall, name = "CAnimBlendStaticAssociation::~CAnimBlendStaticAssociation", expect = "C7 01 CC C6 85 00", this = "CAnimBlendStaticAssociation")]
pub fn static_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, VTBL_STATIC);
    let p = rd::<GPtr<u8>>(this, 0xC);
    if !p.is_null() {
        mem_free(p);
    }
}

/// `CAnimBlendStaticAssociation::Init(RpClump*, CAnimBlendHierarchy*)` (0x4CEC20): without a clump
/// only the hierarchy; else one sequence slot per clump frame, each sequence with frames stored at
/// the index of its clump frame.
#[hook(addr = 0x4CEC20, conv = thiscall, name = "CAnimBlendStaticAssociation::Init", expect = "51 8B 44 24 08", this = "CAnimBlendStaticAssociation")]
pub fn static_init(this: GPtr<u8>, clump: GPtr<u8>, h: GPtr<u8>) {
    if clump.is_null() {
        wr::<GPtr<u8>>(this, 0x10, h);
        return;
    }
    let cd = rd::<GPtr<u8>>(clump, g::<i32>(CLUMP_OFFSET).read() as isize);
    let nf = rd::<u32>(cd, 8);
    wr::<u16>(this, 4, nf as u16);
    call_allocate_sequence_array(this, i32::from(nf as u16 as i16));
    wr::<GPtr<u8>>(this, 0x10, h);
    let mut j = 0;
    while j < i32::from(rd::<i16>(h, 8)) {
        let s = rd::<GPtr<u8>>(h, 4).byte_add(j as isize * 0xC);
        let f = find_frame(clump, s);
        if !f.is_null() && sequence::num_frames(s) > 0 {
            let k = frame_index(f, cd);
            wr::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), k as isize * 4, s);
        }
        j += 1;
    }
}

/// `CAnimBlendStaticAssociation::CAnimBlendStaticAssociation(RpClump*, CAnimBlendHierarchy*)`
/// (0x4CEF60): ids -1, flags/sequences 0, `Init(clump, hier)`; returns `this`.
#[hook(addr = 0x4CEF60, conv = thiscall, name = "CAnimBlendStaticAssociation::Constructor2", expect = "56 8B F1 8B 4C 24 08", this = "CAnimBlendStaticAssociation")]
pub fn static_constructor2(this: GPtr<u8>, clump: GPtr<u8>, h: GPtr<u8>) -> GPtr<u8> {
    wr::<i16>(this, 6, -1);
    wr::<i16>(this, 8, -1);
    wr::<u16>(this, 0xA, 0);
    wr::<u32>(this, 0xC, 0);
    wr::<u32>(this, 0, VTBL_STATIC);
    call_static_init(this, clump, h);
    this
}

/// `CAnimBlendStaticAssociation` scalar/vector deleting destructor (0x4CDF50, vtable slot 0):
/// flags bit 1 = vector (`eh vector destructor iterator` over the count at [-4], then `free(this-4)`
/// when bit 0), else the destructor and `free(this)` when bit 0. Returns the freed address.
#[hook(addr = 0x4CDF50, conv = thiscall, name = "CAnimBlendStaticAssociation::Destructor", expect = "53 8A 5C 24 08", this = "CAnimBlendStaticAssociation")]
pub fn static_deleting_dtor(this: GPtr<u8>, fl: u8) -> GPtr<u8> {
    if fl & 2 != 0 {
        let n = rd::<u32>(this, -4);
        crate::hierarchy::eh_vector_dtor(this, 0x14, n, 0x4CEC00);
        let base = this.byte_add(-4);
        if fl & 1 != 0 {
            crt_free(base);
        }
        return base;
    }
    call_static_dtor(this);
    if fl & 1 != 0 {
        crt_free(this);
    }
    this
}

/// The default callback `CDefaultAnimCallback::DefaultAnimCB` (0x4D1390) the callbacks are reset to.
pub const DEFAULT_ANIM_CB: u32 = 0x4D1390;

orig! {
    /// an association callback `void (*)(CAnimBlendAssociation*, void*)` (cdecl)
    pub fn call_assoc_callback(f: u32, a: GPtr<u8>, data: u32) = cdecl dyn => unported;
    /// `CAnimBlendAssociation::SetFinishCallback` (0x4CEBE0)
    pub fn call_set_finish_callback(this: GPtr<u8>, f: u32, data: u32) = thiscall 0x4CEBE0 => set_finish_callback;
    /// `CAnimBlendAssociation::SetDeleteCallback` (0x4CEBC0)
    pub fn call_set_delete_callback(this: GPtr<u8>, f: u32, data: u32) = thiscall 0x4CEBC0 => set_delete_callback;
    /// the association's virtual deleting destructor (vtable slot 0)
    pub fn assoc_vdelete(this: GPtr<u8>, fl: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
}

/// `CAnimBlendAssociation::UpdateTimeStep(float speedMult, float timeMult)` (0x4D13A0): while playing,
/// `m_TimeStep = m_Speed * speedMult`, or (flag 0x20) `total * timeMult * speedMult`.
#[hook(addr = 0x4D13A0, conv = thiscall, name = "CAnimBlendAssociation::UpdateTimeStep", expect = "8A 41 2E A8 01", this = "CAnimBlendAssociation")]
pub fn update_time_step(this: GPtr<u8>, speed_mult: f32, time_mult: f32) {
    let f = rd::<u8>(this, 0x2E);
    if f & 1 == 0 {
        return;
    }
    let v = if f & 0x20 == 0 {
        x(rd::<f32>(this, 0x24)).fmul_m(speed_mult)
    } else {
        x(rd::<f32>(hier(this), 0x10)).fmul_m(time_mult).fmul_m(speed_mult)
    };
    wr::<f32>(this, 0x28, v.to_f32());
}

/// `CAnimBlendAssociation::UpdateTime(float, float)` (0x4D13D0): advances the current time by the time
/// step while playing; at the end: looped = wrap once, else clamp, fade out (flag 8: flag 4, delta
/// -4), and a finish callback is called once (then reset to the default callback). AL = 1 always.
#[hook(addr = 0x4D13D0, conv = thiscall, name = "CAnimBlendAssociation::UpdateTime", expect = "51 56 8B F1 F6 46 2E 01", this = "CAnimBlendAssociation")]
pub fn update_time(this: GPtr<u8>, _a: f32, _b: f32) -> u8 {
    if rd::<u8>(this, 0x2E) & 1 == 0 {
        return 1;
    }
    let h = hier(this);
    let total = x(rd::<f32>(h, 0x10));
    if !x(rd::<f32>(this, 0x20)).fcom(total).c0() {
        wr::<u8>(this, 0x2E, rd::<u8>(this, 0x2E) & 0xFE);
        return 1;
    }
    let t = x(rd::<f32>(this, 0x28)).fadd_m(rd::<f32>(this, 0x20)).to_f32();
    wr::<f32>(this, 0x20, t);
    if x(t).fcom(x(rd::<f32>(h, 0x10))).c0() {
        return 1;
    }
    let fl = flags(this);
    if fl & 2 != 0 {
        wr::<f32>(this, 0x20, x(t).fsub_m(rd::<f32>(h, 0x10)).to_f32());
        return 1;
    }
    wr::<u32>(this, 0x20, rd::<u32>(h, 0x10));
    if fl & 8 != 0 {
        wr::<u16>(this, 0x2E, fl | 4);
        wr::<u32>(this, 0x1C, 0xC080_0000);
    }
    if rd::<u32>(this, 0x30) == 1 {
        wr::<u32>(this, 0x30, 0);
        call_assoc_callback(rd::<u32>(this, 0x34), this, rd::<u32>(this, 0x38));
        call_set_finish_callback(this, DEFAULT_ANIM_CB, 0);
    }
    1
}

/// `CAnimBlendAssociation::UpdateBlend(float mult)` (0x4D1490): `blend += delta * mult`; blended out
/// while fading (delta < 0): blend and delta 0, and with flag 4 the callback (finish or delete) is
/// called, reset to the default callback and the association deletes itself (AL = 0); above 1 the
/// blend is clamped (and a positive delta cleared). AL = 1 otherwise.
#[hook(addr = 0x4D1490, conv = thiscall, name = "CAnimBlendAssociation::UpdateBlend", expect = "D9 44 24 04 56", this = "CAnimBlendAssociation")]
pub fn update_blend(this: GPtr<u8>, mult: f32) -> u8 {
    let b = x(mult).fmul_m(rd::<f32>(this, 0x1C)).fadd_m(rd::<f32>(this, 0x18));
    wr::<f32>(this, 0x18, b.to_f32());
    let zero = sa_math::x87::X87::zero();
    if !b.fcom(zero).test_parity(0x41) && !x(rd::<f32>(this, 0x1C)).fcom(zero).test_parity(5) {
        // blended out
        wr::<u32>(this, 0x18, 0);
        if !x(rd::<f32>(this, 0x1C)).fcom(zero).test_parity(5) {
            wr::<u32>(this, 0x1C, 0);
        }
        if rd::<u8>(this, 0x2E) & 4 != 0 {
            let ty = rd::<u32>(this, 0x30);
            if ty == 2 || ty == 1 {
                call_assoc_callback(rd::<u32>(this, 0x34), this, rd::<u32>(this, 0x38));
                if rd::<u32>(this, 0x30) == 2 {
                    call_set_delete_callback(this, DEFAULT_ANIM_CB, 0);
                } else {
                    call_set_finish_callback(this, DEFAULT_ANIM_CB, 0);
                }
            }
            assoc_vdelete(this, 1);
            return 0;
        }
    }
    if x(rd::<f32>(this, 0x18)).fcom(sa_math::x87::X87::one()) == sa_math::x87::Cmp::Greater {
        wr::<f32>(this, 0x18, 1.0);
        if x(rd::<f32>(this, 0x1C)).fcom(zero) == sa_math::x87::Cmp::Greater {
            wr::<u32>(this, 0x1C, 0);
        }
    }
    1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_UPDATE_TIME_STEP,
    HOOK_UPDATE_TIME,
    HOOK_UPDATE_BLEND,
    HOOK_CONSTRUCTOR0,
    HOOK_ALLOCATE_ANIM_BLEND_NODE_ARRAY,
    HOOK_FREE_ANIM_BLEND_NODE_ARRAY,
    HOOK_REFERENCE_ANIM_BLOCK,
    HOOK_SET_CURRENT_TIME,
    HOOK_SYNC_ANIMATION,
    HOOK_GET_NODE,
    HOOK_START,
    HOOK_SET_BLEND_TO,
    HOOK_SET_BLEND,
    HOOK_SET_DELETE_CALLBACK,
    HOOK_SET_FINISH_CALLBACK,
    HOOK_INIT_CLUMP,
    HOOK_INIT_ASSOC,
    HOOK_INIT_STATIC,
    HOOK_CONSTRUCTOR1,
    HOOK_CONSTRUCTOR2,
    HOOK_CONSTRUCTOR3,
    HOOK_STATIC_CONSTRUCTOR1,
    HOOK_ALLOCATE_SEQUENCE_ARRAY,
    HOOK_FREE_SEQUENCE_ARRAY,
    HOOK_STATIC_DTOR,
    HOOK_STATIC_INIT,
    HOOK_STATIC_CONSTRUCTOR2,
    HOOK_STATIC_DELETING_DTOR,
];
