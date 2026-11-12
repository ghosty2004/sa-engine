//! `CHandObject` (vtable 0x866EE0; batch `attr-8`): the constructor 0x59EEB0, `PreRender` 0x59ECD0 and `Render`
//! 0x59EE80, the hand models gang peds hold up when signalling on Grove Street. Layout (0x18C bytes): `CObject`, +0x17C
//! the ped, +0x180 the forearm bone index, +0x184 the ped's skin texture, +0x188 "matrices updated" flag. Ported from
//! the 1.0 US bytes (gta-reversed `HandObject.cpp` as a reading aid). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, wr};

orig! {
    /// `CObject::CObject()` (0x5A1D10)
    fn object_ctor(o: GPtr<u8>) -> GPtr<u8> = thiscall 0x5A1D10 => unported;
    /// `GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40)
    fn anim_hierarchy_from_skin_clump(clump: u32) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `CEntity::UpdateRpHAnim()` (0x532B20)
    fn entity_update_rp_h_anim(e: GPtr<u8>) = thiscall 0x532B20 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32 id)` (0x7C51A0)
    fn rp_h_anim_id_get_index(h: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `CEntity::SetModelIndex(uint32)` (0x532AE0, `ret 4`)
    fn entity_set_model_index(e: GPtr<u8>, mi: u32) = thiscall 0x532AE0 => unported;
    /// `RpAnimBlendClumpInit(RpClump*)` (0x4D6720)
    fn rp_anim_blend_clump_init(clump: u32) = cdecl 0x4D6720 => unported;
    /// `RwTexDictionaryFindHashNamedTexture(RwTexDictionary*, uint32)` (0x734E50)
    fn tex_dict_find_hash_named_texture(dict: u32, hash: u32) -> u32 = cdecl 0x734E50 => unported;
    /// `GetFirstTexture(RwTexDictionary*)` (0x734940)
    fn get_first_texture(dict: u32) -> u32 = cdecl 0x734940 => unported;
    /// `CObject::SetIsStatic(bool)` (0x5A0760, `ret 4`)
    fn object_set_is_static(o: GPtr<u8>, s: u32) = thiscall 0x5A0760 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(RpHAnimHierarchy*)` (0x7C5120)
    fn rp_h_anim_hierarchy_get_matrix_array(h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C5120 => unported;
    /// `CMatrix::CMatrix(RwMatrix*, bool owner)` (0x59C050, `ret 8`)
    fn matrix_ctor_attach(m: GPtr<u8>, rw: GPtr<u8>, owner: u32) -> GPtr<u8> = thiscall 0x59C050 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0, `ret 4`)
    fn matrix_assign(dst: GPtr<u8>, src: GPtr<u8>) = thiscall 0x59BBC0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70, `ret 4`)
    fn matrix_update_rw_matrix(m: GPtr<u8>, rw: u32) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40, `ret 4`)
    fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: u32) = thiscall 0x54EF40 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `GetFirstAtomic(RpClump*)` (0x734820)
    fn get_first_atomic(clump: u32) -> GPtr<u8> = cdecl 0x734820 => unported;
    /// `RpMaterialSetTexture(RpMaterial*, RwTexture*)` (0x74DBC0)
    fn rp_material_set_texture(mat: u32, tex: u32) -> u32 = cdecl 0x74DBC0 => unported;
    /// `CObject::Render()` (0x59F180)
    fn object_render(o: GPtr<u8>) = thiscall 0x59F180 => unported;
}

/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CTxdStore::ms_pTxdPool`
const TXD_POOL: usize = 0xC8800C;

fn model_info(mi: i32) -> GPtr<u8> {
    g::<GPtr<u8>>(MODEL_INFO_PTRS.wrapping_add((mi as usize).wrapping_mul(4))).read()
}

/// `CHandObject::CHandObject(int32 model, CPed* ped, bool left)` (0x59EEB0, thiscall, `ret 0xC`; entry `push -1;
/// jmp 0x407615`, the protection fragment pushes the SEH handler). `CObject()`, vtable 0x866EE0; the ped's skin
/// hierarchy, +0x17C = ped, the ped's `UpdateRpHAnim`, +0x180 = the bone index of id 0x21 (left) / 0x17 (right);
/// `SetModelIndex(model)`, `RpAnimBlendClumpInit`; +0x184 = the texture of the ped model's key (model info +4) in the
/// ped model's txd (its pool slot read even when the slot is free: then the dictionary pointer is read from address
/// 0, as in the original), else the dictionary's first texture. Entity flags +0x1C `(f & ~1) | 0x10000400`, status
/// (+0x36 high bits) 2, object type (+0x13C) 5, `SetIsStatic(false)`, the hand model's alpha (+0xC) 0xFF, +0x188 = 1.
#[hook(addr = 0x59EEB0, conv = thiscall, name = "FUN_0059eeb0", expect = "6A FF E9 5E 87 E6 FF",
       unverified = "attr-8", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "entry `push -1; jmp` into the protection fragment 0x407615")]
pub fn hand_object_ctor(this: GPtr<u8>, model: u32, ped: GPtr<u8>, left: u32) -> GPtr<u8> {
    object_ctor(this);
    wr::<u32>(this, 0, 0x866EE0);
    let h = anim_hierarchy_from_skin_clump(rd::<u32>(ped, 0x18));
    wr::<GPtr<u8>>(this, 0x17C, ped);
    entity_update_rp_h_anim(ped);
    let bone = rp_h_anim_id_get_index(h, if left as u8 != 0 { 0x21 } else { 0x17 });
    wr::<i32>(this, 0x180, bone);
    entity_set_model_index(this, model);
    rp_anim_blend_clump_init(rd::<u32>(this, 0x18));
    let mi = model_info(rd::<i16>(ped, 0x22) as i32);
    let txd = rd::<i16>(mi, 0xA) as i32;
    let pool = g::<GPtr<u8>>(TXD_POOL).read();
    let flags = rd::<GPtr<u8>>(pool, 4);
    let slot = if rd::<u8>(flags, txd as isize) & 0x80 != 0 {
        GPtr::<u8>::NULL
    } else {
        rd::<GPtr<u8>>(pool, 0).byte_add((txd as isize).wrapping_mul(12))
    };
    let dict = rd::<u32>(slot, 0);
    let mut tex = tex_dict_find_hash_named_texture(dict, rd::<u32>(mi, 4));
    wr::<u32>(this, 0x184, tex);
    if tex == 0 {
        tex = get_first_texture(dict);
        wr::<u32>(this, 0x184, tex);
    }
    wr::<u32>(this, 0x1C, (rd::<u32>(this, 0x1C) & !1) | 0x1000_0400);
    wr::<u8>(this, 0x36, (rd::<u8>(this, 0x36) & 7) | 0x10);
    wr::<u8>(this, 0x13C, 5);
    object_set_is_static(this, 0);
    wr::<u8>(model_info(model as i32), 0xC, 0xFF);
    wr::<u8>(this, 0x188, 1);
    this
}

/// `CHandObject::PreRender()` (0x59ECD0, thiscall, SEH frame). The ped's skin hierarchy, its `UpdateRpHAnim`, ped
/// flag 0x800000 (+0x1C); the object's matrix = a `CMatrix` attached (not owned) to the bone's (+0x180) RwMatrix.
/// When +0x188 is set it is cleared; otherwise the RwMatrices from the next bone on get their right / up / at
/// vectors zeroed (at, up, right in that order) while the push / pop depth (node flags +8 bit 1 push, bit 0 pop;
/// 0x10-byte node infos at hierarchy +0x10) stays non-negative. Then the contact-surface brightness (+0x12C) is the
/// ped's; with an RwObject (+0x18) its frame's matrix (frame +0x10) is updated from the matrix or the simple
/// transform; `UpdateRwFrame`, `UpdateRpHAnim`.
#[hook(addr = 0x59ECD0, conv = thiscall, name = "CHandObject::PreRender", expect = "6A FF 68 E8 D1 83 00",
       unverified = "attr-8", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn hand_object_pre_render(this: GPtr<u8>) {
    let h = anim_hierarchy_from_skin_clump(rd::<u32>(rd::<GPtr<u8>>(this, 0x17C), 0x18));
    entity_update_rp_h_anim(rd::<GPtr<u8>>(this, 0x17C));
    let ped = rd::<GPtr<u8>>(this, 0x17C);
    wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) | 0x80_0000);
    let mats = rp_h_anim_hierarchy_get_matrix_array(h);
    let mut m = mats.byte_add((rd::<u32>(this, 0x180) << 6) as i32 as isize);
    let mut tmp = [0u32; 0x12];
    let tp = GPtr::from_raw(tmp.as_mut_ptr()).cast::<u8>();
    matrix_ctor_attach(tp, m, 0);
    matrix_assign(rd::<GPtr<u8>>(this, 0x14), tp);
    matrix_dtor(tp);
    let mut node = (rd::<u32>(this, 0x180).wrapping_add(1) << 4) as usize;
    m = m.byte_add(0x40);
    if rd::<u8>(this, 0x188) != 0 {
        wr::<u8>(this, 0x188, 0);
    } else {
        let mut depth: i32 = 0;
        loop {
            for off in [0x20isize, 0x10, 0] {
                wr::<u32>(m, off, 0);
                wr::<u32>(m, off + 4, 0);
                wr::<u32>(m, off + 8, 0);
            }
            let f = rd::<u32>(rd::<GPtr<u8>>(h, 0x10).byte_add(node as isize), 8);
            if f & 2 != 0 {
                depth += 1;
            } else if f & 1 != 0 {
                depth -= 1;
            }
            node = node.wrapping_add(0x10);
            m = m.byte_add(0x40);
            if depth < 0 {
                break;
            }
        }
    }
    let ped = rd::<GPtr<u8>>(this, 0x17C);
    wr::<u32>(this, 0x12C, rd::<u32>(ped, 0x12C));
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if !rw.is_null() {
        let rwm = rd::<u32>(rw, 4).wrapping_add(0x10);
        let mx = rd::<GPtr<u8>>(this, 0x14);
        if mx.is_null() {
            simple_transform_update_rw_matrix(this.byte_add(4), rwm);
        } else {
            matrix_update_rw_matrix(mx, rwm);
        }
    }
    entity_update_rw_frame(this);
    entity_update_rp_h_anim(this);
}

/// `CHandObject::Render()` (0x59EE80, thiscall): the first atomic's geometry's (+0x18) first material (+0x20, [0])
/// gets the ped's texture (+0x184), then `CObject::Render` (tail jump).
#[hook(addr = 0x59EE80, conv = thiscall, name = "CHandObject::Render", expect = "56 8B F1 8B 46 18 50 E8",
       unverified = "attr-8", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn hand_object_render(this: GPtr<u8>) {
    let a = get_first_atomic(rd::<u32>(this, 0x18));
    let geo = rd::<GPtr<u8>>(a, 0x18);
    let mat = rd::<u32>(rd::<GPtr<u8>>(geo, 0x20), 0);
    rp_material_set_texture(mat, rd::<u32>(this, 0x184));
    object_render(this);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_HAND_OBJECT_CTOR, HOOK_HAND_OBJECT_PRE_RENDER, HOOK_HAND_OBJECT_RENDER];
