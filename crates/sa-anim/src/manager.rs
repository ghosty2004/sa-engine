//! ANIM: `CAnimManager` (PORT-ANIM-INPUT), and `CLinkList<CAnimBlendHierarchy*>::Insert` (0x4D40C0).
//! Ported from the 1.0 US bytes (gta-reversed `AnimManager.cpp` as a reading aid,
//! `docs/re/specs/ANIM/CAnimManager*.md`). See `docs/arch/anim-input-port.md`.
//!
//! Globals: `ms_aAnimations` 0xB4EA40 (`CAnimBlendHierarchy[2500]`, 0x18 each), `ms_aAnimBlocks`
//! 0xB5D4A0 (`CAnimBlock[180]`, 0x20: name[16] +0, `IsLoaded` +0x10, `RefCnt` +0x12 (u16),
//! `FirstAnimIdx` +0x14, `NumAnims` +0x18, `GroupId` +0x1C), `ms_numAnimAssocDefinitions` 0xB4EA28,
//! `ms_numAnimations` 0xB4EA2C, `ms_numAnimBlocks` 0xB4EA30, `ms_aAnimAssocGroups` 0xB4EA34,
//! `ms_aAnimAssocDefinitions` 0x8AA5A8 (0x30: group name[16], block name[16] +0x10, model +0x20,
//! anim count +0x24, names +0x28, descriptions +0x2C), the uncompressed cache `ms_AnimCache` 0xB5EB20
//! (`CLinkList`: used head +0, used tail +0xC, free head +0x18, free tail +0x24, links +0x30; links are
//! {data, prev, next}).
//!
//! The `CAnimBlendHierarchy`/`CAnimBlock` arrays are only addressed here (their static initialisers
//! live in `statics`); they are not declared with `globals!` because of their size.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;
use sa_math::x87::{Cmp, X87};

use crate::assoc::CLUMP_OFFSET;
use crate::common::{g, rd, wr, x};
use crate::sequence::{crt_free, get_uppercase_key, operator_new, operator_new_array, stricmp};

pub const ANIMATIONS: usize = 0xB4EA40;
pub const ANIM_BLOCKS: usize = 0xB5D4A0;
pub const ASSOC_DEFS: usize = 0x8AA5A8;
pub const ANIM_CACHE: usize = 0xB5EB20;
pub const USED_HEAD: usize = ANIM_CACHE;
pub const USED_TAIL: usize = ANIM_CACHE + 0xC;
pub const FREE_HEAD: usize = ANIM_CACHE + 0x18;
pub const FREE_TAIL: usize = ANIM_CACHE + 0x24;

sa_core::globals! {
    list GLOBALS;
    /// `CAnimManager::ms_numAnimAssocDefinitions`
    pub static NUM_ANIM_ASSOC_DEFINITIONS @ 0xB4EA28: i32 = 0;
    /// `CAnimManager::ms_numAnimations`
    pub static NUM_ANIMATIONS @ 0xB4EA2C: i32 = 0;
    /// `CAnimManager::ms_numAnimBlocks`
    pub static NUM_ANIM_BLOCKS @ 0xB4EA30: i32 = 0;
    /// `CAnimManager::ms_aAnimAssocGroups`
    pub static ANIM_ASSOC_GROUPS @ 0xB4EA34: GPtr<u8> = GPtr::NULL;
    /// `CAnimManager::ms_AnimCache` (the list heads and the link array pointer at +0x30)
    pub static ANIM_CACHE_LIST @ 0xB5EB20: [u32; 13] = [0; 13];
}

orig! {
    /// `CLinkList<CAnimBlendHierarchy*>::Insert` (0x4D40C0)
    pub fn call_link_insert(this: GPtr<u8>, data: GPtr<u32>) -> GPtr<u8> = thiscall 0x4D40C0 => link_insert;
    /// `CAnimManager::GetAnimationBlock(const char*)` (0x4D3940)
    pub fn call_get_animation_block(name: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D3940 => get_animation_block;
    /// `CAnimManager::GetFirstAssocGroup(const char*)` (0x4D39B0)
    pub fn call_get_first_assoc_group(name: GPtr<u8>) -> i32 = cdecl 0x4D39B0 => get_first_assoc_group;
    /// `CAnimManager::AddAnimationAndSync` (0x4D3B30)
    pub fn call_add_animation_and_sync(clump: GPtr<u8>, sync: GPtr<u8>, group: i32, anim: i32) -> GPtr<u8> = cdecl 0x4D3B30 => add_animation_and_sync;
    /// `CAnimManager::CreateAnimAssocGroups()` (0x4D3CC0)
    pub fn call_create_anim_assoc_groups() = cdecl 0x4D3CC0 => create_anim_assoc_groups;
    /// `CAnimManager::ReadAnimAssociationDefinitions()` (0x5BC910)
    pub fn call_read_anim_association_definitions() = cdecl 0x5BC910 => read_anim_association_definitions;
    /// `CAnimManager::RegisterAnimBlock(const char*)` (0x4D3E50)
    pub fn call_register_anim_block(name: GPtr<u8>) -> i32 = cdecl 0x4D3E50 => register_anim_block;
    /// `CAnimManager::AddAnimAssocDefinition` (0x4D3BA0)
    pub fn call_add_anim_assoc_definition(g: GPtr<u8>, b: GPtr<u8>, model: u32, n: u32, descr: u32) -> GPtr<u8> = cdecl 0x4D3BA0 => add_anim_assoc_definition;
    /// `CAnimManager::AddAnimToAssocDefinition` (0x4D3C80)
    pub fn call_add_anim_to_assoc_definition(def: GPtr<u8>, name: GPtr<u8>) = cdecl 0x4D3C80 => add_anim_to_assoc_definition;
    /// `CAnimManager::LoadAnimFile(RwStream*, bool, const char (*)[32])` (0x4D47F0)
    pub fn load_anim_file(s: GPtr<u8>, compressed: u32, names: GPtr<u8>) = cdecl 0x4D47F0 => crate::anim_file_load::load_anim_file;
    /// `CAnimBlendHierarchy::Shutdown()` (0x4CF980)
    pub fn hier_shutdown(this: GPtr<u8>) = thiscall 0x4CF980 => crate::hierarchy::shutdown;
    /// `CAnimBlendHierarchy::RemoveUncompressedData()` (0x4CF760)
    pub fn hier_remove_uncompressed_data(this: GPtr<u8>) = thiscall 0x4CF760 => crate::hierarchy::remove_uncompressed_data;
    /// `CAnimBlendHierarchy::Uncompress()` (0x4CF5F0)
    pub fn hier_uncompress(this: GPtr<u8>) = thiscall 0x4CF5F0 => crate::hierarchy::uncompress;
    /// `CAnimBlendHierarchy::CalcTotalTimeCompressed()` (0x4CF3E0)
    pub fn hier_calc_total_time_compressed(this: GPtr<u8>) = thiscall 0x4CF3E0 => crate::hierarchy::calc_total_time_compressed;
    /// `CAnimBlendAssocGroup::CopyAnimation(int32)` (0x4CE130)
    pub fn group_copy_animation_id(this: GPtr<u8>, id: i32) -> GPtr<u8> = thiscall 0x4CE130 => crate::group::copy_animation_id;
    /// `CAnimBlendAssocGroup::GetAnimation(int32)` (0x4CE090)
    pub fn group_get_animation_id(this: GPtr<u8>, id: i32) -> GPtr<u8> = thiscall 0x4CE090 => crate::group::get_animation_id;
    /// `CAnimBlendAssocGroup::GetAnimation(const char*)` (0x4CE040)
    pub fn group_get_animation_name(this: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = thiscall 0x4CE040 => crate::group::get_animation_name;
    /// `CAnimBlendAssocGroup::DestroyAssociations()` (0x4CDFF0)
    pub fn group_destroy_associations(this: GPtr<u8>) = thiscall 0x4CDFF0 => crate::group::destroy_associations;
    /// `CAnimBlendAssocGroup::CreateAssociations(const char*, RpClump*, char**, int32)` (0x4CE6E0)
    pub fn group_create_associations_2(this: GPtr<u8>, block: GPtr<u8>, clump: GPtr<u8>, names: GPtr<u8>, n: i32) = thiscall 0x4CE6E0 => crate::group::create_associations_2;
    /// `CAnimBlendAssociation::Start(float)` (0x4CEB70)
    pub fn assoc_start(this: GPtr<u8>, t: f32) = thiscall 0x4CEB70 => crate::assoc::start;
    /// `CAnimBlendAssociation::SyncAnimation(CAnimBlendAssociation*)` (0x4CEB40)
    pub fn assoc_sync_animation(this: GPtr<u8>, other: GPtr<u8>) = thiscall 0x4CEB40 => crate::assoc::sync_animation;
    /// `CAnimBlendAssociation::ReferenceAnimBlock()` (0x4CEA50)
    pub fn assoc_reference_anim_block(this: GPtr<u8>) = thiscall 0x4CEA50 => crate::assoc::reference_anim_block;
    /// `CAnimBlendAssociation::CAnimBlendAssociation(RpClump*, CAnimBlendHierarchy*)` (0x4CEFC0)
    pub fn assoc_constructor1(this: GPtr<u8>, clump: GPtr<u8>, h: GPtr<u8>) -> GPtr<u8> = thiscall 0x4CEFC0 => crate::assoc::constructor1;
    /// `CStreaming::RemoveModel(int32)` (0x4089A0)
    pub fn streaming_remove_model(id: i32) = cdecl 0x4089A0 => unported;
    /// `RpAnimBlendClumpInit(RpClump*)` (0x4D6720)
    pub fn rp_anim_blend_clump_init(clump: GPtr<u8>) = cdecl 0x4D6720 => unported;
    /// `GetFirstAtomic(RpClump*)` (0x734820)
    pub fn get_first_atomic(clump: GPtr<u8>) -> GPtr<u8> = cdecl 0x734820 => unported;
    /// `RpSkinGeometryGetSkin(RpGeometry*)` (0x7C7550)
    pub fn rp_skin_geometry_get_skin(geo: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7550 => unported;
    /// `RpClumpForAllAtomics(RpClump*, cb, data)` (0x749B70)
    pub fn rp_clump_for_all_atomics(clump: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `RpClumpDestroy(RpClump*)` (0x74A310)
    pub fn rp_clump_destroy(clump: GPtr<u8>) -> i32 = cdecl 0x74A310 => unported;
    /// `CBaseModelInfo::CreateInstance()` (vtable +0x2C)
    pub fn model_create_instance(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x2C => unported;
    /// `RwStreamOpen(type, access, data)` (0x7ECEF0)
    pub fn rw_stream_open(ty: i32, access: i32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7ECEF0 => unported;
    /// `RwStreamClose(stream, data)` (0x7ECE20)
    pub fn rw_stream_close(s: GPtr<u8>, data: u32) -> i32 = cdecl 0x7ECE20 => unported;
    /// CRT `sprintf` (0x821BB5) with one argument
    pub fn sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
    /// CRT `sscanf` (0x8220AD) with four targets
    pub fn sscanf4(s: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, d: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// CRT `sscanf` (0x8220AD) with one target
    pub fn sscanf1(s: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// CRT `strncpy` (0x821F40)
    pub fn strncpy(d: GPtr<u8>, s: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// `CFileMgr::SetDir(const char*)` (0x5387D0)
    pub fn file_set_dir(dir: GPtr<u8>) = cdecl 0x5387D0 => unported;
    /// `CFileMgr::OpenFile(const char*, const char*)` (0x538900)
    pub fn file_open(name: GPtr<u8>, mode: GPtr<u8>) -> i32 = cdecl 0x538900 => unported;
    /// `CFileMgr::CloseFile(int32)` (0x5389D0)
    pub fn file_close(f: i32) -> i32 = cdecl 0x5389D0 => unported;
    /// `CFileLoader::LoadLine(FILESTREAM)` (0x536F80)
    pub fn load_line(f: i32) -> GPtr<u8> = cdecl 0x536F80 => unported;
}

#[inline]
fn block(i: i32) -> GPtr<u8> {
    g::<u8>(ANIM_BLOCKS).byte_add(i as isize * 0x20)
}
#[inline]
fn anim(i: i32) -> GPtr<u8> {
    g::<u8>(ANIMATIONS).byte_add(i as isize * 0x18)
}
#[inline]
fn group(i: i32) -> GPtr<u8> {
    ANIM_ASSOC_GROUPS.get().byte_add(i as isize * 0x14)
}
#[inline]
fn def(i: i32) -> GPtr<u8> {
    g::<u8>(ASSOC_DEFS).byte_add(i as isize * 0x30)
}
#[inline]
fn clump_data(clump: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(clump, g::<i32>(CLUMP_OFFSET).read() as isize)
}

/// Unlinks link `l` and inserts it after the list head `head` (the cache's inlined `CLink` moves).
fn move_after(l: GPtr<u8>, head: usize) {
    let next = rd::<GPtr<u8>>(l, 8);
    let prev = rd::<GPtr<u8>>(l, 4);
    wr::<GPtr<u8>>(next, 4, prev);
    wr::<GPtr<u8>>(prev, 8, next);
    let h = g::<u8>(head);
    wr::<GPtr<u8>>(l, 8, rd::<GPtr<u8>>(h, 8));
    wr::<GPtr<u8>>(rd::<GPtr<u8>>(h, 8), 4, l);
    wr::<GPtr<u8>>(l, 4, h);
    wr::<GPtr<u8>>(h, 8, l);
}

/// `FUN_004D40C0` = `CLinkList<CAnimBlendHierarchy*>::Insert(T const&)`: the first free link gets the
/// data and moves after the used head; null when the free list is empty.
#[hook(addr = 0x4D40C0, conv = thiscall, name = "FUN_004d40c0", expect = "8B 41 20 8D 51 24")]
pub fn link_insert(this: GPtr<u8>, data: GPtr<u32>) -> GPtr<u8> {
    let l = rd::<GPtr<u8>>(this, 0x20);
    if l.addr() == this.byte_add(0x24).addr() {
        return GPtr::NULL;
    }
    wr::<u32>(l, 0, data.read());
    let next = rd::<GPtr<u8>>(l, 8);
    wr::<u32>(next, 4, rd::<u32>(l, 4));
    let prev = rd::<GPtr<u8>>(l, 4);
    wr::<u32>(prev, 8, rd::<u32>(l, 8));
    wr::<u32>(l, 8, rd::<u32>(this, 8));
    wr::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), 4, l);
    wr::<GPtr<u8>>(l, 4, this);
    wr::<GPtr<u8>>(this, 8, l);
    l
}

/// `CAnimManager::GetAnimationBlock(const char*)` (0x4D3940): the first block whose name
/// `_stricmp`-matches, else null. Not hookable: the protection reads its entry bytes (0x156F054); the
/// entry's chunk reads `ms_numAnimBlocks` through a rotated pointer (0xEA3000B4 rol 208 = 0xB4EA30).
/// Phase B entry (STD-6).
#[sa_core::native(addr = 0x4D3940, conv = cdecl, name = "CAnimManager::GetAnimationBlock-")]
pub fn get_animation_block(name: GPtr<u8>) -> GPtr<u8> {
    let mut i = 0;
    while i < NUM_ANIM_BLOCKS.get() {
        if stricmp(block(i), name) == 0 {
            return block(i);
        }
        i += 1;
    }
    GPtr::NULL
}

/// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990): index or -1.
#[hook(addr = 0x4D3990, conv = cdecl, name = "CAnimManager::GetAnimationBlockIndex-by-name", expect = "8B 44 24 04 50")]
pub fn get_animation_block_index(name: GPtr<u8>) -> i32 {
    let b = call_get_animation_block(name);
    if b.is_null() {
        return -1;
    }
    ((b.addr() as u32).wrapping_sub(ANIM_BLOCKS as u32) as i32) >> 5
}

/// `CAnimManager::GetFirstAssocGroup(const char* blockName)` (0x4D39B0): the first of the 118 fixed
/// definitions whose block name matches, else 118.
#[hook(addr = 0x4D39B0, conv = cdecl, name = "CAnimManager::GetFirstAssocGroup", expect = "53 8B 5C 24 08")]
pub fn get_first_assoc_group(name: GPtr<u8>) -> i32 {
    let mut i = 0;
    while i < 0x76 {
        if stricmp(def(i).byte_add(0x10), name) == 0 {
            return i;
        }
        i += 1;
    }
    0x76
}

/// `CAnimManager::GetAnimation(uint32 hash, const CAnimBlock*)` (0x4D39F0).
#[hook(addr = 0x4D39F0, conv = cdecl, name = "CAnimManager::GetAnimation-0", expect = "8B 54 24 08 8B 42 14")]
pub fn get_animation_hash(hash: u32, blk: GPtr<u8>) -> GPtr<u8> {
    let first = rd::<i32>(blk, 0x14);
    let n = rd::<i32>(blk, 0x18);
    let mut i = 0;
    while i < n {
        let h = anim(first.wrapping_add(i));
        if rd::<u32>(h, 0) == hash {
            return h;
        }
        i += 1;
    }
    GPtr::NULL
}

/// `CAnimManager::GetAnimGroupName(int32)` (0x4D3A20): `&defs[i].groupName`.
#[hook(addr = 0x4D3A20, conv = cdecl, name = "CAnimManager::GetAnimGroupName", expect = "8B 44 24 04 8D 04 40")]
pub fn get_anim_group_name(i: i32) -> u32 {
    (i.wrapping_mul(0x30) as u32).wrapping_add(ASSOC_DEFS as u32)
}

/// `CAnimManager::GetAnimBlockName(int32)` (0x4D3A30): `&defs[i].blockName`.
#[hook(addr = 0x4D3A30, conv = cdecl, name = "CAnimManager::GetAnimBlockName", expect = "8B 44 24 04 8D 04 40")]
pub fn get_anim_block_name(i: i32) -> u32 {
    (i.wrapping_mul(0x30) as u32).wrapping_add(ASSOC_DEFS as u32 + 0x10)
}

/// `CAnimManager::CreateAnimAssociation(group, anim)` (0x4D3A40): `groups[group].CopyAnimation(anim)`.
#[hook(addr = 0x4D3A40, conv = cdecl, name = "CAnimManager::CreateAnimAssociation", expect = "8B 44 24 08 8B 15 34 EA B4 00")]
pub fn create_anim_association(grp: i32, id: i32) -> GPtr<u8> {
    group_copy_animation_id(group(grp), id)
}

/// `CAnimManager::GetAnimAssociation(group, int32 anim)` (0x4D3A60).
#[hook(addr = 0x4D3A60, conv = cdecl, name = "CAnimManager::GetAnimAssociation-by-id", expect = "8B 44 24 08 8B 15 34 EA B4 00")]
pub fn get_anim_association_id(grp: i32, id: i32) -> GPtr<u8> {
    group_get_animation_id(group(grp), id)
}

/// `CAnimManager::GetAnimAssociation(group, const char*)` (0x4D3A80).
#[hook(addr = 0x4D3A80, conv = cdecl, name = "CAnimManager::GetAnimAssociation-by-name", expect = "8B 44 24 08 8B 15 34 EA B4 00")]
pub fn get_anim_association_name(grp: i32, name: GPtr<u8>) -> GPtr<u8> {
    group_get_animation_name(group(grp), name)
}

/// Links association `a` at the head of the clump data's association list.
fn link_assoc(cd: GPtr<u8>, a: GPtr<u8>) {
    let l = a.byte_add(4);
    let head = rd::<GPtr<u8>>(cd, 0);
    if !head.is_null() {
        wr::<GPtr<u8>>(head, 4, l);
    }
    wr::<GPtr<u8>>(l, 4, cd);
    wr::<GPtr<u8>>(l, 0, rd::<GPtr<u8>>(cd, 0));
    wr::<GPtr<u8>>(cd, 0, l);
}

/// The first association of the clump's list with the MOVEMENT flag (0x20), or null.
fn first_moving(cd: GPtr<u8>) -> GPtr<u8> {
    let mut l = rd::<GPtr<u8>>(cd, 0);
    while !l.is_null() {
        if rd::<u8>(l, 0x2A) & 0x20 != 0 {
            return l.byte_add(-4);
        }
        l = rd::<GPtr<u8>>(l, 0);
    }
    GPtr::NULL
}

/// `CAnimManager::AddAnimation(RpClump*, group, anim)` (0x4D3AA0): a copy of the group's animation,
/// synced to the first moving association (and played) when it moves, else started at 0; linked
/// first into the clump's list.
#[hook(addr = 0x4D3AA0, conv = cdecl, name = "CAnimManager::AddAnimation-id", expect = "8B 44 24 0C 8B 15 34 EA B4 00")]
pub fn add_animation_id(clump: GPtr<u8>, grp: i32, id: i32) -> GPtr<u8> {
    let a = group_copy_animation_id(group(grp), id);
    let cd = clump_data(clump);
    let m = if rd::<u8>(a, 0x2E) & 0x20 != 0 { first_moving(cd) } else { GPtr::NULL };
    if m.is_null() {
        assoc_start(a, 0.0);
    } else {
        assoc_sync_animation(a, m);
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
    }
    link_assoc(cd, a);
    a
}

/// `CAnimManager::AddAnimationAndSync(RpClump*, CAnimBlendAssociation* sync, group, anim)` (0x4D3B30).
#[hook(addr = 0x4D3B30, conv = cdecl, name = "CAnimManager::AddAnimationAndSync", expect = "8B 44 24 10 8B 15 34 EA B4 00")]
pub fn add_animation_and_sync(clump: GPtr<u8>, sync: GPtr<u8>, grp: i32, id: i32) -> GPtr<u8> {
    let a = group_copy_animation_id(group(grp), id);
    let cd = clump_data(clump);
    if rd::<u8>(a, 0x2E) & 0x20 != 0 && !sync.is_null() {
        assoc_sync_animation(a, sync);
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
    } else {
        assoc_start(a, 0.0);
    }
    link_assoc(cd, a);
    a
}

/// Inlined `strcpy`.
fn strcpy(d: GPtr<u8>, s: GPtr<u8>) {
    let mut i = 0;
    loop {
        let c = rd::<u8>(s, i);
        wr::<u8>(d, i, c);
        if c == 0 {
            break;
        }
        i += 1;
    }
}

/// `CAnimManager::AddAnimAssocDefinition(group, block, model, numAnims, descriptions)` (0x4D3BA0): the
/// next free definition (skipping used ones), names copied, `numAnims` name slots (24 bytes each,
/// emptied) allocated; the following definition's name is emptied while below 145.
#[hook(addr = 0x4D3BA0, conv = cdecl, name = "CAnimManager::AddAnimAssocDefinition", expect = "A1 28 EA B4 00")]
pub fn add_anim_assoc_definition(gname: GPtr<u8>, bname: GPtr<u8>, model: u32, n: u32, descr: u32) -> GPtr<u8> {
    let mut k = NUM_ANIM_ASSOC_DEFINITIONS.get();
    if rd::<u8>(def(k), 0) != 0 {
        while rd::<u8>(def(k + 1), 0) != 0 {
            k += 1;
        }
        k += 1;
        NUM_ANIM_ASSOC_DEFINITIONS.set(k);
    }
    let d = def(k);
    strcpy(d, gname);
    strcpy(d.byte_add(0x10), bname);
    wr::<u32>(d, 0x20, model);
    wr::<u32>(d, 0x24, n);
    wr::<u32>(d, 0x2C, descr);
    let names = operator_new(n.wrapping_mul(4));
    wr::<GPtr<u8>>(d, 0x28, names);
    let mut p = operator_new(n.wrapping_mul(0x18));
    let mut i = 0u32;
    while i < n {
        wr::<GPtr<u8>>(rd::<GPtr<u8>>(d, 0x28), i as isize * 4, p);
        let s = rd::<GPtr<u8>>(rd::<GPtr<u8>>(d, 0x28), i as isize * 4);
        p = p.byte_add(0x18);
        wr::<u8>(s, 0, 0);
        i += 1;
    }
    let k = NUM_ANIM_ASSOC_DEFINITIONS.get() + 1;
    NUM_ANIM_ASSOC_DEFINITIONS.set(k);
    if k < 0x91 {
        wr::<u8>(def(k), 0, 0);
    }
    d
}

/// `CAnimManager::AddAnimToAssocDefinition(def, name)` (0x4D3C80): copied into the first empty name.
#[hook(addr = 0x4D3C80, conv = cdecl, name = "CAnimManager::AddAnimToAssocDefinition", expect = "8B 44 24 04 56")]
pub fn add_anim_to_assoc_definition(d: GPtr<u8>, name: GPtr<u8>) {
    let names = rd::<GPtr<u8>>(d, 0x28);
    let mut i = 0isize;
    while rd::<u8>(rd::<GPtr<u8>>(names, i * 4), 0) != 0 {
        i += 1;
    }
    strcpy(rd::<GPtr<u8>>(names, i * 4), name);
}

/// The skinned-clump clean-up and destruction the group creators share.
pub fn destroy_temp_clump(clump: GPtr<u8>) {
    let a = get_first_atomic(clump);
    if !a.is_null() && !rp_skin_geometry_get_skin(rd::<GPtr<u8>>(a, 0x18)).is_null() {
        rp_clump_for_all_atomics(clump, 0x734B90, 0);
    }
    rp_clump_destroy(clump);
}

/// `CAnimManager::CreateAnimAssocGroups()` (0x4D3CC0): every definition whose block is loaded and
/// whose group has no associations yet gets them (from a temporary instance of the definition's
/// model when it has one), with the description flags OR-ed into the static associations.
#[hook(addr = 0x4D3CC0, conv = cdecl, name = "CAnimManager::CreateAnimAssocGroups", expect = "83 EC 08 A1 28 EA B4 00")]
pub fn create_anim_assoc_groups() {
    let mut i = 0;
    while i < NUM_ANIM_ASSOC_DEFINITIONS.get() {
        let d = def(i);
        let mut b = GPtr::NULL;
        let mut j = 0;
        while j < NUM_ANIM_BLOCKS.get() {
            if stricmp(block(j), d.byte_add(0x10)) == 0 {
                b = block(j);
                break;
            }
            j += 1;
        }
        if !b.is_null() && rd::<u8>(b, 0x10) != 0 && rd::<u32>(group(i), 4) == 0 {
            let mut clump = GPtr::NULL;
            let model = rd::<i32>(d, 0x20);
            if model != -1 {
                let mi = g::<GPtr<u8>>(0xA9B0C8 + model as usize * 4).read();
                clump = model_create_instance(mi);
                rp_anim_blend_clump_init(clump);
            }
            wr::<i32>(group(i), 0x10, i);
            let descr = rd::<GPtr<u8>>(d, 0x2C);
            wr::<u32>(group(i), 0xC, rd::<u32>(descr, 0));
            group_create_associations_2(group(i), d.byte_add(0x10), clump, rd::<GPtr<u8>>(d, 0x28), rd::<i32>(d, 0x24));
            let mut k = 0u32;
            while k < rd::<u32>(group(i), 8) {
                let ds = descr.byte_add(k as isize * 8);
                let s = group_get_animation_id(group(i), rd::<i32>(ds, 0));
                wr::<u16>(s, 0xA, rd::<u16>(s, 0xA) | rd::<u16>(ds, 4));
                k += 1;
            }
            if !clump.is_null() {
                destroy_temp_clump(clump);
            }
        }
        i += 1;
    }
}

/// `CAnimManager::RegisterAnimBlock(const char*)` (0x4D3E50): the block's index, a new block (name
/// `strncpy`-ed, 16 bytes; no animations; its first association group) when unknown.
#[hook(addr = 0x4D3E50, conv = cdecl, name = "CAnimManager::RegisterAnimBlock", expect = "56 57 8B 7C 24 0C")]
pub fn register_anim_block(name: GPtr<u8>) -> i32 {
    let mut b = call_get_animation_block(name);
    if b.is_null() {
        let n = NUM_ANIM_BLOCKS.get();
        b = block(n);
        NUM_ANIM_BLOCKS.set(n + 1);
        strncpy(b, name, 0x10);
        wr::<u32>(b, 0x18, 0);
        let gi = call_get_first_assoc_group(b);
        wr::<i32>(b, 0x1C, gi);
    }
    ((b.addr() as u32).wrapping_sub(ANIM_BLOCKS as u32) as i32) >> 5
}

/// `CAnimManager::RemoveLastAnimFile()` (0x4D3ED0): drops the last block and its animations.
#[hook(addr = 0x4D3ED0, conv = cdecl, name = "CAnimManager::RemoveLastAnimFile", expect = "8B 0D 30 EA B4 00")]
pub fn remove_last_anim_file() {
    let n = NUM_ANIM_BLOCKS.get() - 1;
    NUM_ANIMATIONS.set(rd::<i32>(block(n), 0x14));
    NUM_ANIM_BLOCKS.set(n);
    let mut i = 0;
    while i < rd::<i32>(block(NUM_ANIM_BLOCKS.get()), 0x18) {
        hier_shutdown(anim(rd::<i32>(block(NUM_ANIM_BLOCKS.get()), 0x14).wrapping_add(i)));
        i += 1;
    }
    wr::<u8>(block(NUM_ANIM_BLOCKS.get()), 0x10, 0);
}

/// `CAnimManager::RemoveAnimBlock(int32)` (0x4D3F40): the groups using it lose their associations,
/// its animations are shut down, not loaded, 0 references.
#[hook(addr = 0x4D3F40, conv = cdecl, name = "CAnimManager::RemoveAnimBlock", expect = "A1 28 EA B4 00")]
pub fn remove_anim_block(idx: i32) {
    let b = block(idx);
    let mut i = 0;
    while i < NUM_ANIM_ASSOC_DEFINITIONS.get() {
        if rd::<u32>(group(i), 0) == b.addr() as u32 {
            group_destroy_associations(group(i));
        }
        i += 1;
    }
    let mut i = 0;
    while i < rd::<i32>(b, 0x18) {
        hier_shutdown(anim(rd::<i32>(b, 0x14).wrapping_add(i)));
        i += 1;
    }
    wr::<u8>(b, 0x10, 0);
    wr::<u16>(b, 0x12, 0);
}

/// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0): `++RefCnt` (16-bit).
#[hook(addr = 0x4D3FB0, conv = cdecl, name = "CAnimManager::AddAnimBlockRef", expect = "8B 44 24 04 C1 E0 05")]
pub fn add_anim_block_ref(i: i32) {
    let b = block(i);
    wr::<u16>(b, 0x12, rd::<u16>(b, 0x12).wrapping_add(1));
}

/// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0): `--RefCnt` (16-bit, wraps).
#[hook(addr = 0x4D3FD0, conv = cdecl, name = "CAnimManager::RemoveAnimBlockRef", expect = "8B 44 24 04 C1 E0 05")]
pub fn remove_anim_block_ref(i: i32) {
    let b = block(i);
    wr::<u16>(b, 0x12, rd::<u16>(b, 0x12).wrapping_sub(1));
}

/// `CAnimManager::RemoveAnimBlockRefWithoutDelete(int32)` (0x4D3FF0): `--RefCnt`.
#[hook(addr = 0x4D3FF0, conv = cdecl, name = "CAnimManager::RemoveAnimBlockRefWithoutDelete", expect = "8B 44 24 04 C1 E0 05")]
pub fn remove_anim_block_ref_without_delete(i: i32) {
    let b = block(i);
    wr::<u16>(b, 0x12, rd::<u16>(b, 0x12).wrapping_sub(1));
}

/// `CAnimManager::GetNumRefsToAnimBlock(int32)` (0x4D4010): `RefCnt` sign-extended.
#[hook(addr = 0x4D4010, conv = cdecl, name = "CAnimManager::GetNumRefsToAnimBlock", expect = "8B 44 24 04 C1 E0 05")]
pub fn get_num_refs_to_anim_block(i: i32) -> i32 {
    i32::from(rd::<i16>(block(i), 0x12))
}

/// `CAnimManager::Shutdown()` (0x4D4130): the 180 IFP streaming models removed, every animation shut
/// down, the cache link array freed (pointer cleared), the group array destroyed (`delete[]`; the
/// pointer is left dangling, as in the original). Not hookable: the model loop branches back to
/// 0x4D4133, inside the 9 bytes an entry patch would replace.
pub fn shutdown() {
    for i in 0..0xB4 {
        streaming_remove_model(0x63E7 + i);
    }
    let mut i = 0;
    while i < NUM_ANIMATIONS.get() {
        hier_shutdown(anim(i));
        i += 1;
    }
    let links = g::<GPtr<u8>>(ANIM_CACHE + 0x30);
    crt_free(links.read());
    links.write(GPtr::NULL);
    let grp = ANIM_ASSOC_GROUPS.get();
    if !grp.is_null() {
        let n = rd::<u32>(grp, -4);
        crate::hierarchy::eh_vector_dtor(grp, 0x14, n, 0x4CE1D0);
        crt_free(grp.byte_add(-4));
    }
}

/// `CAnimManager::UncompressAnimation(CAnimBlendHierarchy*)` (0x4D41C0): kept-compressed hierarchies
/// only get their total time (when exactly 0); uncompressed ones move to the front of the cache;
/// compressed ones get a cache link (evicting and recompressing the least recently used one when the
/// cache is full) and are uncompressed.
#[hook(addr = 0x4D41C0, conv = cdecl, name = "CAnimManager::UncompressAnimation", expect = "57 8B 7C 24 08")]
pub fn uncompress_animation(h: GPtr<u8>) {
    if rd::<u8>(h, 0xB) != 0 {
        if x(rd::<f32>(h, 0x10)).fcom(X87::zero()) == Cmp::Equal {
            hier_calc_total_time_compressed(h);
        }
        return;
    }
    if rd::<u8>(h, 0xA) == 0 {
        let l = rd::<GPtr<u8>>(h, 0x14);
        if !l.is_null() {
            move_after(l, USED_HEAD);
        }
        return;
    }
    let mut slot = [h.addr() as u32];
    let cache = g::<u8>(ANIM_CACHE);
    let mut l = call_link_insert(cache, GPtr::from_raw(slot.as_mut_ptr()));
    if l.is_null() {
        let t = g::<GPtr<u8>>(USED_TAIL + 4).read();
        hier_remove_uncompressed_data(rd::<GPtr<u8>>(t, 0));
        move_after(t, FREE_HEAD);
        wr::<u32>(rd::<GPtr<u8>>(t, 0), 0x14, 0);
        l = call_link_insert(cache, GPtr::from_raw(slot.as_mut_ptr()));
    }
    wr::<GPtr<u8>>(h, 0x14, l);
    hier_uncompress(h);
}

/// `CAnimManager::RemoveFromUncompressedCache(CAnimBlendHierarchy*)` (0x4D42A0): its link moves to the
/// free list, `m_Link = 0`.
#[hook(addr = 0x4D42A0, conv = cdecl, name = "CAnimManager::RemoveFromUncompressedCache", expect = "8B 4C 24 04 8B 41 14")]
pub fn remove_from_uncompressed_cache(h: GPtr<u8>) {
    let l = rd::<GPtr<u8>>(h, 0x14);
    if l.is_null() {
        return;
    }
    move_after(l, FREE_HEAD);
    wr::<u32>(h, 0x14, 0);
}

/// `CAnimManager::GetAnimation(const char*, const CAnimBlock*)` (0x4D42F0): by
/// `CKeyGen::GetUppercaseKey(name)`.
#[hook(addr = 0x4D42F0, conv = cdecl, name = "CAnimManager::GetAnimation-1", expect = "8B 44 24 04 56")]
pub fn get_animation_name(name: GPtr<u8>, blk: GPtr<u8>) -> GPtr<u8> {
    let key = get_uppercase_key(name);
    get_animation_hash(key, blk)
}

/// The `BlendAnimation` fade-out delta of a non-target association: `min(-(blend * d), -0.05)`.
fn fade_delta(blend: f32, d: f32) -> f32 {
    let v = x(blend).fmul_m(d).fchs();
    let c = x(f32::from_bits(0xBD4C_CCCD));
    if c.fcom(v) == Cmp::Less { c.to_f32() } else { v.to_f32() }
}

/// `CAnimManager::AddAnimation(RpClump*, CAnimBlendHierarchy*, int32 flags)` (0x4D4330): a new
/// association (`operator new` + constructor) with these flags, block referenced, hierarchy
/// uncompressed, synced to the first moving association or started; linked first.
#[hook(addr = 0x4D4330, conv = cdecl, name = "CAnimManager::AddAnimation-hier", expect = "6A FF 68 4B BD 83 00")]
pub fn add_animation_hier(clump: GPtr<u8>, h: GPtr<u8>, fl: i32) -> GPtr<u8> {
    let p = operator_new(0x3C);
    let a = if p.is_null() { p } else { assoc_constructor1(p, clump, h) };
    wr::<u16>(a, 0x2E, fl as u16);
    assoc_reference_anim_block(a);
    uncompress_animation_call(h);
    let cd = clump_data(clump);
    let m = if rd::<u8>(a, 0x2E) & 0x20 != 0 { first_moving(cd) } else { GPtr::NULL };
    if m.is_null() {
        assoc_start(a, 0.0);
    } else {
        assoc_sync_animation(a, m);
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
    }
    link_assoc(cd, a);
    a
}

orig! {
    /// `CAnimManager::UncompressAnimation` (0x4D41C0)
    pub fn uncompress_animation_call(h: GPtr<u8>) = cdecl 0x4D41C0 => uncompress_animation;
}

/// The found-association tail of both `BlendAnimation`s.
fn blend_found(a: GPtr<u8>, d: f32) -> GPtr<u8> {
    let v = X87::one().fsub_m(rd::<f32>(a, 0x18)).fmul_m(d).to_f32();
    wr::<f32>(a, 0x1C, v);
    let h = rd::<GPtr<u8>>(a, 0x14);
    if x(rd::<f32>(h, 0x10)).fcom(x(rd::<f32>(a, 0x20))) == Cmp::Equal {
        assoc_start(a, 0.0);
    }
    uncompress_animation_call(rd::<GPtr<u8>>(a, 0x14));
    a
}

/// `CAnimManager::BlendAnimation(RpClump*, CAnimBlendHierarchy*, int32 flags, float delta)` (0x4D4410):
/// the clump's association with this hierarchy blends in; every other one of the same partial-ness
/// fades out (flag 4; delta `min(-(blend*d), -0.05)` unless it is already fading faster and partial
/// with another anim block, -1 when not blended in); without one a new association is added (blend 1
/// when nothing fades and not partial, else blend 0 rising by `delta`).
#[hook(addr = 0x4D4410, conv = cdecl, name = "CAnimManager::BlendAnimation-hier", expect = "6A FF 68 6B BD 83 00")]
pub fn blend_animation_hier(clump: GPtr<u8>, h: GPtr<u8>, fl: i32, d: f32) -> GPtr<u8> {
    let cd = clump_data(clump);
    let partial = (fl >> 4) as u8 & 1;
    let mut fading = false;
    let mut found = GPtr::NULL;
    let mut l = rd::<GPtr<u8>>(cd, 0);
    while !l.is_null() {
        let a = l.byte_add(-4);
        let ah = rd::<GPtr<u8>>(a, 0x14);
        if !ah.is_null() && ah.addr() == h.addr() {
            found = a;
        } else {
            let af = rd::<u16>(a, 0x2E);
            if (af >> 4) as u8 & 1 == partial {
                let mut set = true;
                if x(rd::<f32>(a, 0x18)).fcom(X87::zero()) != Cmp::Greater {
                    wr::<u32>(a, 0x1C, 0xBF80_0000);
                    set = false;
                } else {
                    let v = x(rd::<f32>(a, 0x18)).fmul_m(d).fchs();
                    if v.fcom(x(rd::<f32>(a, 0x1C))) != Cmp::Less && partial != 0 {
                        let ab = rd::<u32>(ah, 0xC);
                        if !(ab == rd::<u32>(h, 0xC) && ab != 0) {
                            set = false;
                        }
                    }
                    if set {
                        wr::<f32>(a, 0x1C, fade_delta(rd::<f32>(a, 0x18), d));
                    }
                }
                let _ = set;
                wr::<u16>(a, 0x2E, af | 4);
                fading = true;
            }
        }
        l = rd::<GPtr<u8>>(l, 0);
    }
    if !found.is_null() {
        return blend_found(found, d);
    }
    let p = operator_new(0x3C);
    let a = if p.is_null() { p } else { assoc_constructor1(p, clump, h) };
    wr::<u16>(a, 0x2E, fl as u16);
    assoc_reference_anim_block(a);
    uncompress_animation_call(h);
    link_assoc(cd, a);
    assoc_start(a, 0.0);
    if !fading && partial == 0 {
        wr::<f32>(a, 0x18, 1.0);
        return a;
    }
    wr::<u32>(a, 0x18, 0);
    wr::<f32>(a, 0x1C, d);
    uncompress_animation_call(rd::<GPtr<u8>>(a, 0x14));
    a
}

/// `CAnimManager::BlendAnimation(RpClump*, group, anim, float delta)` (0x4D4610): as the hierarchy
/// version with the group animation's partial (0x10) and 0x8000 flags as the match, the last moving
/// association as the sync partner of a new one (`AddAnimationAndSync`), and no anim-block test.
#[hook(addr = 0x4D4610, conv = cdecl, name = "CAnimManager::BlendAnimation-id", expect = "83 EC 14 8B 4C 24 18")]
pub fn blend_animation_id(clump: GPtr<u8>, grp: i32, id: i32, d: f32) -> GPtr<u8> {
    let cd = clump_data(clump);
    let moving = rd::<u8>(group_get_animation_id(group(grp), id), 0xA) >> 5 & 1;
    let partial = rd::<u8>(group_get_animation_id(group(grp), id), 0xA) >> 4 & 1;
    let hi = (rd::<u16>(group_get_animation_id(group(grp), id), 0xA) >> 15) as u8 & 1;
    let mut sync = GPtr::NULL;
    let mut found = GPtr::NULL;
    let mut fading = false;
    let mut l = rd::<GPtr<u8>>(cd, 0);
    while !l.is_null() {
        let a = l.byte_add(-4);
        if moving != 0 && rd::<u8>(a, 0x2E) & 0x20 != 0 {
            sync = a;
        }
        if i32::from(rd::<i16>(a, 0x2C)) == id && i32::from(rd::<i16>(a, 0xE)) == grp {
            found = a;
        } else {
            let af = rd::<u16>(a, 0x2E);
            if (af >> 4) as u8 & 1 == partial && (af >> 15) as u8 & 1 == hi {
                if x(rd::<f32>(a, 0x18)).fcom(X87::zero()) != Cmp::Greater {
                    wr::<u32>(a, 0x1C, 0xBF80_0000);
                } else {
                    let v = x(d).fmul_m(rd::<f32>(a, 0x18)).fchs();
                    if v.fcom(x(rd::<f32>(a, 0x1C))) == Cmp::Less || partial == 0 {
                        let c = x(f32::from_bits(0xBD4C_CCCD));
                        let r = if c.fcom(v) == Cmp::Less { c } else { v };
                        wr::<f32>(a, 0x1C, r.to_f32());
                    }
                }
                wr::<u16>(a, 0x2E, af | 4);
                fading = true;
            }
        }
        l = rd::<GPtr<u8>>(l, 0);
    }
    if !found.is_null() {
        return blend_found(found, d);
    }
    let a = call_add_animation_and_sync(clump, sync, grp, id);
    if !fading && partial == 0 {
        wr::<f32>(a, 0x18, 1.0);
        return a;
    }
    wr::<u32>(a, 0x18, 0);
    wr::<f32>(a, 0x1C, d);
    uncompress_animation_call(rd::<GPtr<u8>>(a, 0x14));
    a
}

/// `CAnimManager::LoadAnimFiles()` (0x4D5620): `ANIM\PED.IFP` loaded (compressed), the group array
/// (`new CAnimBlendAssocGroup[numDefs]`), `CreateAnimAssocGroups`.
#[hook(addr = 0x4D5620, conv = cdecl, name = "CAnimManager::LoadAnimFiles", expect = "6A FF 68 BE BD 83 00")]
pub fn load_anim_files() {
    let mut buf = [0u8; 0x100];
    let b = GPtr::from_raw(buf.as_mut_ptr());
    sprintf1(b, g(0x85C714), g(0x85C718));
    let s = rw_stream_open(2, 1, b);
    load_anim_file(s, 1, GPtr::NULL);
    rw_stream_close(s, 0);
    let n = NUM_ANIM_ASSOC_DEFINITIONS.get() as u32;
    let p = operator_new(n.wrapping_mul(0x14).wrapping_add(4));
    let arr = if p.is_null() {
        GPtr::NULL
    } else {
        wr::<u32>(p, 0, n);
        crate::sequence::eh_vector_ctor(p.byte_add(4), 0x14, n, 0x4CDE70, 0x4CE1D0);
        p.byte_add(4)
    };
    ANIM_ASSOC_GROUPS.set(arr);
    call_create_anim_assoc_groups();
}

/// `CAnimManager::ReadAnimAssociationDefinitions()` (0x5BC910): `DATA\ANIMGRP.DAT`: a header line
/// `"%s %s %s %d"` (group, block, model name unused, count) starts a definition (model 7, the shared
/// description table 0x8A7788); then one anim name per line up to `end`. `#` and empty lines skipped.
/// The parse buffers share one stack frame as in the original (the name buffer is the group buffer).
#[hook(addr = 0x5BC910, conv = cdecl, name = "CAnimManager::ReadAnimAssociationDefinitions", expect = "83 EC 68 53 56")]
pub fn read_anim_association_definitions() {
    let mut fr = [0u8; 0x80];
    let f = GPtr::from_raw(fr.as_mut_ptr());
    file_set_dir(g(0x858B54));
    let fd = file_open(g(0x86A7F4), g(0x859C6C));
    let mut in_def = false;
    let mut d = GPtr::NULL;
    let mut line = load_line(fd);
    while !line.is_null() {
        let c = rd::<u8>(line, 0);
        if c != b'#' && c != 0 {
            if !in_def {
                sscanf4(line, g(0x86A7E8), f.byte_add(0x18), f.byte_add(0x38), f.byte_add(0x58), f.byte_add(0x10));
                d = call_add_anim_assoc_definition(f.byte_add(0x18), f.byte_add(0x38), 7, rd::<u32>(f, 0x10), 0x8A7788);
                in_def = true;
            } else if sscanf1(line, g(0x85C714), f.byte_add(0x18)) == 1 {
                if (0..4).all(|k| rd::<u8>(f, 0x18 + k) == rd::<u8>(g::<u8>(0x8671EC), k)) {
                    in_def = false;
                } else {
                    call_add_anim_to_assoc_definition(d, f.byte_add(0x18));
                }
            }
        }
        line = load_line(fd);
    }
    file_close(fd);
}

/// `CAnimManager::Initialise()` (0x5BF6B0): counters reset (118 fixed definitions), the cache's 50
/// links (`new[]`, 0x258 bytes) put on the free list (last first), the definitions read, block "ped"
/// registered.
#[hook(addr = 0x5BF6B0, conv = cdecl, name = "CAnimManager::Initialise", expect = "33 C0 68 58 02 00 00")]
pub fn initialise() {
    NUM_ANIMATIONS.set(0);
    NUM_ANIM_BLOCKS.set(0);
    NUM_ANIM_ASSOC_DEFINITIONS.set(0x76);
    let links = operator_new_array(0x258);
    let c = g::<u32>(ANIM_CACHE);
    c.byte_add(0x30).write(links.addr() as u32);
    c.byte_add(8).write(USED_TAIL as u32);
    c.byte_add(0x10).write(USED_HEAD as u32);
    c.byte_add(0x20).write(FREE_TAIL as u32);
    c.byte_add(0x28).write(FREE_HEAD as u32);
    let mut off = 0x258isize;
    while off != 0 {
        off -= 0xC;
        let l = g::<GPtr<u8>>(ANIM_CACHE + 0x30).read().byte_add(off);
        let head_next = g::<GPtr<u8>>(FREE_HEAD + 8).read();
        wr::<GPtr<u8>>(l, 8, head_next);
        wr::<GPtr<u8>>(head_next, 4, l);
        wr::<u32>(l, 4, FREE_HEAD as u32);
        g::<GPtr<u8>>(FREE_HEAD + 8).write(l);
    }
    call_read_anim_association_definitions();
    call_register_anim_block(g(0x86AB68));
}

pub const HOOKS: &[HookReg] = &[
    HOOK_LINK_INSERT,
    HOOK_GET_ANIMATION_BLOCK_INDEX,
    HOOK_GET_FIRST_ASSOC_GROUP,
    HOOK_GET_ANIMATION_HASH,
    HOOK_GET_ANIM_GROUP_NAME,
    HOOK_GET_ANIM_BLOCK_NAME,
    HOOK_CREATE_ANIM_ASSOCIATION,
    HOOK_GET_ANIM_ASSOCIATION_ID,
    HOOK_GET_ANIM_ASSOCIATION_NAME,
    HOOK_ADD_ANIMATION_ID,
    HOOK_ADD_ANIMATION_AND_SYNC,
    HOOK_ADD_ANIM_ASSOC_DEFINITION,
    HOOK_ADD_ANIM_TO_ASSOC_DEFINITION,
    HOOK_CREATE_ANIM_ASSOC_GROUPS,
    HOOK_REGISTER_ANIM_BLOCK,
    HOOK_REMOVE_LAST_ANIM_FILE,
    HOOK_REMOVE_ANIM_BLOCK,
    HOOK_ADD_ANIM_BLOCK_REF,
    HOOK_REMOVE_ANIM_BLOCK_REF,
    HOOK_REMOVE_ANIM_BLOCK_REF_WITHOUT_DELETE,
    HOOK_GET_NUM_REFS_TO_ANIM_BLOCK,
    HOOK_UNCOMPRESS_ANIMATION,
    HOOK_REMOVE_FROM_UNCOMPRESSED_CACHE,
    HOOK_GET_ANIMATION_NAME,
    HOOK_ADD_ANIMATION_HIER,
    HOOK_BLEND_ANIMATION_HIER,
    HOOK_BLEND_ANIMATION_ID,
    HOOK_LOAD_ANIM_FILES,
    HOOK_READ_ANIM_ASSOCIATION_DEFINITIONS,
    HOOK_INITIALISE,
];
