//! ANIM: `CAnimBlendAssocGroup` (PORT-ANIM-INPUT). Ported from the 1.0 US bytes (gta-reversed
//! `AnimBlendAssocGroup.cpp` as a reading aid, `docs/re/specs/ANIM/CAnimBlendAssocGroup.md`). See
//! `docs/arch/anim-input-port.md`.
//!
//! Layout (0x14): +0x0 `m_AnimBlock` (`CAnimBlock*`), +0x4 `m_Anims` (`CAnimBlendStaticAssociation[]`
//! from `new[]`, count at [-4]), +0x8 `m_NumAnims`, +0xC `m_IdOffset`, +0x10 `m_GroupID`.
//! The SEH frames of the creators/copiers (around the array construction and `operator new`) are not
//! built: they only matter when a constructor throws, which these never do.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;

use crate::assoc::call_static_init;
use crate::common::{g, rd, wr};
use crate::manager::{
    ANIMATIONS, call_get_animation_block, destroy_temp_clump, model_create_instance, rp_anim_blend_clump_init,
    uncompress_animation_call,
};
use crate::sequence::{crt_free, eh_vector_ctor, get_uppercase_key, operator_new};

sa_core::globals! {
    list GLOBALS;
}

orig! {
    /// the static association's vector deleting destructor (vtable slot 0)
    pub fn static_vdelete(this: GPtr<u8>, fl: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    /// `CAnimBlendAssocGroup::GetAnimation(const char*)` (0x4CE040)
    pub fn call_get_animation_name(this: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = thiscall 0x4CE040 => get_animation_name;
    /// `CAnimBlendAssocGroup::CreateAssociations(const char*)` (0x4CE220)
    pub fn call_create_associations_1(this: GPtr<u8>, block: GPtr<u8>) = thiscall 0x4CE220 => create_associations_1;
    /// `CAnimBlendAssocGroup::DestroyAssociations()` (0x4CDFF0)
    pub fn call_destroy_associations(this: GPtr<u8>) = thiscall 0x4CDFF0 => destroy_associations;
    /// `CAnimBlendAssociation::CAnimBlendAssociation(CAnimBlendStaticAssociation&)` (0x4CF080)
    pub fn assoc_constructor3(this: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x4CF080 => crate::assoc::constructor3;
    /// `CAnimManager::GetAnimation(const char*, const CAnimBlock*)` (0x4D42F0)
    pub fn manager_get_animation_name(name: GPtr<u8>, block: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D42F0 => crate::manager::get_animation_name;
    /// `CModelInfo::GetModelInfoFromHashKey(uint32, int32*)` (0x4C59B0)
    pub fn get_model_info_from_hash_key(key: u32, idx: GPtr<i32>) -> GPtr<u8> = cdecl 0x4C59B0 => unported;
    /// `CModelInfo::GetModelInfo(const char*, int32*)` (0x4C5940)
    pub fn get_model_info_by_name(name: GPtr<u8>, idx: GPtr<i32>) -> GPtr<u8> = cdecl 0x4C5940 => unported;
}

#[inline]
fn anims(g: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(g, 4)
}
#[inline]
fn sa(g: GPtr<u8>, i: i32) -> GPtr<u8> {
    anims(g).byte_add(i as isize * 0x14)
}

/// `CAnimBlendAssocGroup::GetNumAnimations()` (0x45B050): `mov eax, [ecx+8]; ret` (not hookable).
pub fn get_num_animations(this: GPtr<u8>) -> i32 {
    rd::<i32>(this, 8)
}

/// `CAnimBlendAssocGroup::GetAnimBlock()` (0x45B060): `mov eax, [ecx]; ret` (not hookable).
pub fn get_anim_block(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0)
}

/// `CAnimBlendAssocGroup::CAnimBlendAssocGroup()` (0x4CDE70): block, anims, count, offset 0, group -1.
#[hook(addr = 0x4CDE70, conv = thiscall, name = "CAnimBlendAssocGroup::Constructor", expect = "8B C1 33 C9 89 08", this = "CAnimBlendAssocGroup")]
pub fn constructor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0);
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0xC, 0);
    wr::<i32>(this, 0x10, -1);
    this
}

/// `CAnimBlendAssocGroup::InitEmptyAssociations(RpClump*)` (0x4CDFB0): every static association without
/// a sequence array is `Init(clump, its hierarchy)`.
#[hook(addr = 0x4CDFB0, conv = thiscall, name = "CAnimBlendAssocGroup::InitEmptyAssociations", expect = "53 56 8B F1 8B 46 08", this = "CAnimBlendAssocGroup")]
pub fn init_empty_associations(this: GPtr<u8>, clump: GPtr<u8>) {
    let mut i = 0;
    while i < rd::<i32>(this, 8) {
        let s = sa(this, i);
        if rd::<u32>(s, 0xC) == 0 {
            call_static_init(s, clump, rd::<GPtr<u8>>(s, 0x10));
        }
        i += 1;
    }
}

/// The inlined `delete[] m_Anims` (vector deleting destructor through the first element's vtable when
/// the count is non-zero, else `free(m_Anims - 4)`).
fn delete_anims(this: GPtr<u8>) -> bool {
    let a = anims(this);
    if a.is_null() {
        return false;
    }
    if rd::<u32>(a, -4) != 0 {
        static_vdelete(a, 3);
    } else {
        crt_free(a.byte_add(-4));
    }
    true
}

/// `CAnimBlendAssocGroup::DestroyAssociations()` (0x4CDFF0): `delete[] m_Anims`, anims and count 0.
#[hook(addr = 0x4CDFF0, conv = thiscall, name = "CAnimBlendAssocGroup::DestroyAssociations", expect = "56 8B F1 8B 4E 04", this = "CAnimBlendAssocGroup")]
pub fn destroy_associations(this: GPtr<u8>) {
    if delete_anims(this) {
        wr::<u32>(this, 4, 0);
        wr::<u32>(this, 8, 0);
    }
}

/// `CAnimBlendAssocGroup::~CAnimBlendAssocGroup()` (0x4CE1D0): as `DestroyAssociations`.
#[hook(addr = 0x4CE1D0, conv = thiscall, name = "CAnimBlendAssocGroup::Destructor", expect = "56 8B F1 8B 4E 04", this = "CAnimBlendAssocGroup")]
pub fn destructor(this: GPtr<u8>) {
    destroy_associations(this);
}

/// `CAnimBlendAssocGroup::GetAnimation(const char*)` (0x4CE040): the static association whose
/// hierarchy's hash equals `CKeyGen::GetUppercaseKey(name)`, else null.
#[hook(addr = 0x4CE040, conv = thiscall, name = "CAnimBlendAssocGroup::GetAnimation-Name", expect = "8B 44 24 04 53", this = "CAnimBlendAssocGroup")]
pub fn get_animation_name(this: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> {
    let key = get_uppercase_key(name);
    let n = rd::<i32>(this, 8);
    let mut i = 0;
    while i < n {
        if rd::<u32>(rd::<GPtr<u8>>(sa(this, i), 0x10), 0) == key {
            return sa(this, i);
        }
        i += 1;
    }
    GPtr::NULL
}

/// `CAnimBlendAssocGroup::GetAnimation(int32 id)` (0x4CE090): `&m_Anims[id - m_IdOffset]`, unchecked.
#[hook(addr = 0x4CE090, conv = thiscall, name = "CAnimBlendAssocGroup::GetAnimation-AnimationId", expect = "8B 51 0C 8B 44 24 04", this = "CAnimBlendAssocGroup")]
pub fn get_animation_id(this: GPtr<u8>, id: i32) -> GPtr<u8> {
    anims(this).byte_add((id.wrapping_sub(rd::<i32>(this, 0xC)) as isize).wrapping_mul(0x14))
}

/// `new CAnimBlendAssociation(*s)` after `UncompressAnimation(s->hier)`; null for a null `s`.
fn copy(s: GPtr<u8>) -> GPtr<u8> {
    if s.is_null() {
        return GPtr::NULL;
    }
    uncompress_animation_call(rd::<GPtr<u8>>(s, 0x10));
    let p = operator_new(0x3C);
    if p.is_null() {
        return GPtr::NULL;
    }
    assoc_constructor3(p, s)
}

/// `CAnimBlendAssocGroup::CopyAnimation(const char*)` (0x4CE0B0).
#[hook(addr = 0x4CE0B0, conv = thiscall, name = "CAnimBlendAssocGroup::CopyAnimation-Name", expect = "64 A1 00 00 00 00", this = "CAnimBlendAssocGroup")]
pub fn copy_animation_name(this: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> {
    copy(call_get_animation_name(this, name))
}

/// `CAnimBlendAssocGroup::CopyAnimation(int32 id)` (0x4CE130).
#[hook(addr = 0x4CE130, conv = thiscall, name = "CAnimBlendAssocGroup::CopyAnimation-AnimationId", expect = "64 A1 00 00 00 00", this = "CAnimBlendAssocGroup")]
pub fn copy_animation_id(this: GPtr<u8>, id: i32) -> GPtr<u8> {
    copy(get_animation_id(this, id))
}

/// `CAnimBlendAssocGroup::GetAnimationId(const char*)` (0x4CE1B0): the anim id (sign-extended) or -1.
#[hook(addr = 0x4CE1B0, conv = thiscall, name = "CAnimBlendAssocGroup::GetAnimationId", expect = "8B 44 24 04 50", this = "CAnimBlendAssocGroup")]
pub fn get_animation_id_by_name(this: GPtr<u8>, name: GPtr<u8>) -> i32 {
    let s = call_get_animation_name(this, name);
    if s.is_null() { -1 } else { i32::from(rd::<i16>(s, 6)) }
}

/// `new CAnimBlendStaticAssociation[n]` (count prefix; `eh vector constructor iterator`), m_Anims set,
/// m_NumAnims 0.
fn new_anims(this: GPtr<u8>, n: u32) {
    let p = operator_new(n.wrapping_mul(0x14).wrapping_add(4));
    let a = if p.is_null() {
        GPtr::NULL
    } else {
        wr::<u32>(p, 0, n);
        eh_vector_ctor(p.byte_add(4), 0x14, n, 0x4CE940, 0x4CEC00);
        p.byte_add(4)
    };
    wr::<GPtr<u8>>(this, 4, a);
    wr::<u32>(this, 8, 0);
}

#[inline]
fn hier_of(blk: GPtr<u8>, i: i32) -> GPtr<u8> {
    g::<u8>(ANIMATIONS).byte_add((rd::<i32>(blk, 0x14).wrapping_add(i)) as isize * 0x18)
}

/// Sets the anim id (`m_IdOffset + k`) and group id of static association `s`.
fn set_ids(this: GPtr<u8>, s: GPtr<u8>, k: i32) {
    wr::<u16>(s, 6, rd::<u16>(this, 0xC).wrapping_add(k as u16));
    wr::<u16>(s, 8, rd::<u16>(this, 0x10));
}

/// `CAnimBlendAssocGroup::CreateAssociations(const char* blockName)` (0x4CE220): one static association
/// per animation of the block whose model (by the hierarchy's name hash) exists, built on a temporary
/// instance of that model; the count ends as the block's animation count.
#[hook(addr = 0x4CE220, conv = thiscall, name = "CAnimBlendAssocGroup::CreateAssociations-1", expect = "64 A1 00 00 00 00", this = "CAnimBlendAssocGroup")]
pub fn create_associations_1(this: GPtr<u8>, block_name: GPtr<u8>) {
    if delete_anims(this) {
        wr::<u32>(this, 4, 0);
        wr::<u32>(this, 8, 0);
    }
    let blk = call_get_animation_block(block_name);
    wr::<GPtr<u8>>(this, 0, blk);
    new_anims(this, rd::<u32>(blk, 0x18));
    let mut i = 0;
    while i < rd::<i32>(rd::<GPtr<u8>>(this, 0), 0x18) {
        let h = hier_of(rd::<GPtr<u8>>(this, 0), i);
        let mi = get_model_info_from_hash_key(rd::<u32>(h, 0), GPtr::NULL);
        if !mi.is_null() {
            let clump = model_create_instance(mi);
            rp_anim_blend_clump_init(clump);
            call_static_init(sa(this, i), clump, h);
            destroy_temp_clump(clump);
            set_ids(this, sa(this, i), i);
        }
        wr::<u32>(this, 8, rd::<u32>(this, 8).wrapping_add(1));
        i += 1;
    }
    wr::<u32>(this, 8, rd::<u32>(rd::<GPtr<u8>>(this, 0), 0x18));
}

/// `CAnimBlendAssocGroup::CreateAssociations(const char* block, const char* animNames,
/// const char* objectNames, uint32 stride)` (0x4CE3B0): without object names `CreateAssociations(block)`;
/// else each animation of the block is matched (last match wins) by name hash against the anim names
/// and built on a temporary instance of the object with the same index (appended; ids `offset + j`).
#[hook(addr = 0x4CE3B0, conv = thiscall, name = "CAnimBlendAssocGroup::CreateAssociations-3", expect = "64 A1 00 00 00 00", this = "CAnimBlendAssocGroup")]
pub fn create_associations_3(this: GPtr<u8>, block_name: GPtr<u8>, anim_names: GPtr<u8>, obj_names: GPtr<u8>, stride: u32) {
    if obj_names.is_null() {
        call_create_associations_1(this, block_name);
        return;
    }
    if !anims(this).is_null() {
        call_destroy_associations(this);
    }
    let blk = call_get_animation_block(block_name);
    wr::<GPtr<u8>>(this, 0, blk);
    new_anims(this, rd::<u32>(blk, 0x18));
    let mut j = 0;
    while j < rd::<i32>(rd::<GPtr<u8>>(this, 0), 0x18) {
        let h = hier_of(rd::<GPtr<u8>>(this, 0), j);
        let mut found = -1i32;
        let mut k = 0;
        let mut name = anim_names;
        while k < rd::<i32>(rd::<GPtr<u8>>(this, 0), 0x18) {
            let hash = rd::<u32>(h, 0);
            if get_uppercase_key(name) == hash {
                found = k;
            }
            name = name.byte_add(stride as isize);
            k += 1;
        }
        if found != -1 {
            g::<u32>(0xAAE948).write(0);
            let obj = obj_names.byte_add((found as u32).wrapping_mul(stride) as i32 as isize);
            let mi = get_model_info_by_name(obj, GPtr::NULL);
            if !mi.is_null() {
                let clump = model_create_instance(mi);
                rp_anim_blend_clump_init(clump);
                let n = rd::<i32>(this, 8);
                call_static_init(sa(this, n), clump, h);
                destroy_temp_clump(clump);
                let n = rd::<i32>(this, 8);
                set_ids(this, sa(this, n), j);
                wr::<i32>(this, 8, rd::<i32>(this, 8) + 1);
            }
        }
        j += 1;
    }
}

/// `CAnimBlendAssocGroup::CreateAssociations(const char* blockName, RpClump* clump)` (0x4CE5C0, REA):
/// every animation of the block on `clump`; ids `offset + i`.
#[hook(addr = 0x4CE5C0, conv = thiscall, name = "CAnimBlendAssocGroup::CreateAssociations-", expect = "64 A1 00 00 00 00", this = "CAnimBlendAssocGroup")]
pub fn create_associations_clump(this: GPtr<u8>, block_name: GPtr<u8>, clump: GPtr<u8>) {
    if delete_anims(this) {
        wr::<u32>(this, 4, 0);
        wr::<u32>(this, 8, 0);
    }
    let blk = call_get_animation_block(block_name);
    wr::<GPtr<u8>>(this, 0, blk);
    new_anims(this, rd::<u32>(blk, 0x18));
    let mut i = 0;
    while i < rd::<i32>(rd::<GPtr<u8>>(this, 0), 0x18) {
        let h = hier_of(rd::<GPtr<u8>>(this, 0), i);
        call_static_init(sa(this, i), clump, h);
        set_ids(this, sa(this, i), i);
        wr::<u32>(this, 8, rd::<u32>(this, 8).wrapping_add(1));
        i += 1;
    }
    wr::<u32>(this, 8, rd::<u32>(rd::<GPtr<u8>>(this, 0), 0x18));
}

/// `CAnimBlendAssocGroup::CreateAssociations(const char* block, RpClump*, char** names, int32 n)`
/// (0x4CE6E0): one static association per name (by `CAnimManager::GetAnimation(name, block)`; an
/// empty name gets anim id -1 and no init); the count ends as `n`.
#[hook(addr = 0x4CE6E0, conv = thiscall, name = "CAnimBlendAssocGroup::CreateAssociations-2", expect = "64 A1 00 00 00 00", this = "CAnimBlendAssocGroup")]
pub fn create_associations_2(this: GPtr<u8>, block_name: GPtr<u8>, clump: GPtr<u8>, names: GPtr<u8>, n: i32) {
    if delete_anims(this) {
        wr::<u32>(this, 4, 0);
        wr::<u32>(this, 8, 0);
    }
    let blk = call_get_animation_block(block_name);
    wr::<GPtr<u8>>(this, 0, blk);
    new_anims(this, n as u32);
    let mut i = 0;
    while i < n {
        let nm = rd::<GPtr<u8>>(names, i as isize * 4);
        let s = sa(this, i);
        if rd::<u8>(nm, 0) != 0 {
            let h = manager_get_animation_name(nm, rd::<GPtr<u8>>(this, 0));
            call_static_init(s, clump, h);
            wr::<u16>(s, 6, rd::<u16>(this, 0xC).wrapping_add(i as u16));
        } else {
            wr::<i16>(s, 6, -1);
        }
        wr::<u16>(s, 8, rd::<u16>(this, 0x10));
        wr::<u32>(this, 8, rd::<u32>(this, 8).wrapping_add(1));
        i += 1;
    }
    wr::<i32>(this, 8, n);
}

/// `CAnimBlendAssocGroup::IsCreated()` (0x4D37A0): `m_Anims != 0` (EAX 0/1).
#[hook(addr = 0x4D37A0, conv = thiscall, name = "CAnimBlendAssocGroup::IsCreated", expect = "8B 51 04 33 C0", this = "CAnimBlendAssocGroup")]
pub fn is_created(this: GPtr<u8>) -> u32 {
    u32::from(!anims(this).is_null())
}

pub const HOOKS: &[HookReg] = &[
    HOOK_CONSTRUCTOR,
    HOOK_INIT_EMPTY_ASSOCIATIONS,
    HOOK_DESTROY_ASSOCIATIONS,
    HOOK_DESTRUCTOR,
    HOOK_GET_ANIMATION_NAME,
    HOOK_GET_ANIMATION_ID,
    HOOK_COPY_ANIMATION_NAME,
    HOOK_COPY_ANIMATION_ID,
    HOOK_GET_ANIMATION_ID_BY_NAME,
    HOOK_CREATE_ASSOCIATIONS_1,
    HOOK_CREATE_ASSOCIATIONS_3,
    HOOK_CREATE_ASSOCIATIONS_CLUMP,
    HOOK_CREATE_ASSOCIATIONS_2,
    HOOK_IS_CREATED,
];
