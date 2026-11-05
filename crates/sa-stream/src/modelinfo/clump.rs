//! `CClumpModelInfo` (vtable 0x85BD30; `m_nAnimFileIndex`/anim file name at +0x20, size 0x24) and its
//! RenderWare callbacks.
//!
//! Ported from the 1.0 US bytes; gta-reversed `Models/ClumpModelInfo.cpp` is a reading aid. Kept from
//! the original:
//! - `SetClump` reads the 2d-effect plugin offset for the old clump through the protection chunk
//!   0x401307 (`[[0x1560618] ^ [0x5A6FDC]]`, = `[0xC3A1E0]`), computed the same way at run time; the
//!   new clump's count uses 0xC3A1E0 directly. Skinned clumps: per vertex the four weights are
//!   normalised with `r = 1 / (((w1 + w0) + w2) + w3)` (x87, PC=24), the vertex count re-read
//!   every iteration; then `hierarchy->flags = 0x3000` (gr-deviations ClumpModelInfo.cpp:196,
//!   harmless). The first atomic's morph-target bounding radius (+0x10) is scaled by 1.2.
//! - the frame-search callbacks return `found ? null : frame` with `setne; dec; and`;
//! - `SetAnimFile` skips "null" (case-insensitive), otherwise stores a `new char[]` copy at +0x20;
//! - `CreateInstance(RwMatrix*)` copies the matrix into the new clump's frame (+0x10) without
//!   `RwFrameUpdateObjects`.

use sa_core::abi::HookReg;
use sa_core::light::RwMatrix;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::atomic::{
    set_base_model_info_flags, add_anim_block_ref, add_ref, add_tex_dictionary_ref, custom_building_atomic_setup, is_cbpc_pipeline_attached,
    is_ccpc_pipeline_attached, remove_anim_block_ref, remove_ref, remove_tex_dictionary_ref,
};
use super::base::{
    add_2dfx_count, atomic_2dfx_count, f, geometry_2dfx_count, get_2d_effect_atomic,
    vcall_create_instance, vcall_get_anim_file_index, MiPtr,
};
use super::weapon::rp_clump_for_all_atomics;
use crate::shared::mi;

/// +0x20: `m_nAnimFileIndex` (or, before `ConvertAnimFileIndex`, the anim file name).
pub const ANIM_FILE: isize = 0x20;

sa_core::globals! {
    list GLOBALS;
    /// Protection constant of `SetClump`'s chunk 0x401307 (`.HOODLUM` dword); xor with the `.text`
    /// dword below = 0xC3A1E0 (`g2dEffectPluginOffset`'s address)
    pub static PROT_SETCLUMP_A @ 0x1560618: u32 = 0xFF26_0C44;
    /// ... `.text` dword
    pub static PROT_SETCLUMP_B @ 0x5A6FDC: u32 = 0xFFE5_ADA4;
}

/// "null" (0x85BBDC)
pub const STR_NULL: u32 = 0x85BBDC;
/// 1.2f (0x858F08)
pub const K_1_2: u32 = 0x3F99_999A;
/// `CClumpModelInfo::AtomicSetupLightingCB` (0x4C4F30)
pub const ATOMIC_SETUP_LIGHTING_CB: u32 = 0x4C4F30;
/// `CClumpModelInfo::SetHierarchyForSkinAtomic` (0x4C4EF0)
pub const SET_HIERARCHY_FOR_SKIN_ATOMIC: u32 = 0x4C4EF0;
/// `CClumpModelInfo::FindFrameFromNameCB` (0x4C52A0)
pub const FIND_FRAME_FROM_NAME_CB: u32 = 0x4C52A0;
/// `CClumpModelInfo::FindFrameFromNameWithoutIdCB` (0x4C52F0)
pub const FIND_FRAME_FROM_NAME_WITHOUT_ID_CB: u32 = 0x4C52F0;
/// `CClumpModelInfo::FindFrameFromIdCB` (0x4C5350)
pub const FIND_FRAME_FROM_ID_CB: u32 = 0x4C5350;
/// `CClumpModelInfo::FillFrameArrayCB` (0x4C5390)
pub const FILL_FRAME_ARRAY_CB: u32 = 0x4C5390;
/// `CAnimManager::ms_aAnimBlocks` (0xB5D4A0, 0x20 bytes each)
pub const ANIM_BLOCKS: u32 = 0xB5D4A0;

orig! {
    /// `CBaseModelInfo::Init` (0x4C4B10)
    pub fn base_init(this: MiPtr) = thiscall 0x4C4B10 => super::base::init;
    /// `CBaseModelInfo::Shutdown` (0x4C4D50)
    pub fn base_shutdown(this: MiPtr) = thiscall 0x4C4D50 => super::base::shutdown;
    /// `CBaseModelInfo::DeleteCollisionModel` (0x4C4C40)
    pub fn delete_collision_model(this: MiPtr) = thiscall 0x4C4C40 => super::base::delete_collision_model;
    /// CRT `_stricmp` (0x8229B6)
    pub fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// CRT `operator new` (0x82119A)
    pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// CRT `free` (0x8214BD)
    pub fn free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990)
    pub fn get_animation_block_index(name: GPtr<u8>) -> i32 = cdecl 0x4D3990 => unported;
    /// `CAnimManager::GetAnimation(uint32 key, CAnimBlock*)` (0x4D39F0)
    pub fn get_animation(key: u32, block: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D39F0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, CAnimBlendHierarchy*, int, float)` (0x4D4410)
    pub fn blend_animation(clump: GPtr<u8>, hier: GPtr<u8>, flags: i32, delta: f32) -> GPtr<u8> = cdecl 0x4D4410 => unported;
    /// `CCarFXRenderer::CustomCarPipeAtomicSetup` (0x5D5B20)
    pub fn custom_car_pipe_atomic_setup(atomic: GPtr<u8>) -> i32 = cdecl 0x5D5B20 => unported;
    /// `CVisibilityPlugins::SetAtomicRenderCallback` (0x7328A0)
    pub fn set_atomic_render_callback(atomic: GPtr<u8>, cb: u32) = cdecl 0x7328A0 => unported;
    /// `CVisibilityPlugins::SetClumpModelInfo` (0x733750)
    pub fn set_clump_model_info(clump: GPtr<u8>, mi: MiPtr) = cdecl 0x733750 => unported;
    /// `CVisibilityPlugins::GetFrameHierarchyId` (0x732A20)
    pub fn get_frame_hierarchy_id(frame: GPtr<u8>) -> i32 = cdecl 0x732A20 => unported;
    /// `CVisibilityPlugins::SetFrameHierarchyId` (0x732A00)
    pub fn set_frame_hierarchy_id(frame: GPtr<u8>, id: i32) = cdecl 0x732A00 => unported;
    /// `NodeNamePlugin::GetFrameNodeName` (0x72FB30)
    pub fn get_frame_node_name(frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x72FB30 => unported;
    /// `RwFrameForAllChildren` (0x7F0DC0)
    pub fn rw_frame_for_all_children(frame: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0DC0 => unported;
    /// `RpSkinAtomicSetHAnimHierarchy` (0x7C7520)
    pub fn rp_skin_atomic_set_hanim_hierarchy(atomic: GPtr<u8>, hier: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7520 => unported;
    /// `RwHelper::GetAnimHierarchyFromFrame` (0x734AB0)
    pub fn get_anim_hierarchy_from_frame(frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x734AB0 => unported;
    /// `RwHelper::GetAnimHierarchyFromClump` (0x734B10)
    pub fn get_anim_hierarchy_from_clump(clump: GPtr<u8>) -> GPtr<u8> = cdecl 0x734B10 => unported;
    /// `RwHelper::GetFirstAtomic` (0x734820)
    pub fn get_first_atomic(clump: GPtr<u8>) -> GPtr<u8> = cdecl 0x734820 => unported;
    /// `RpSkinGeometryGetSkin` (0x7C7550)
    pub fn rp_skin_geometry_get_skin(geometry: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7550 => unported;
    /// `RpSkinGetVertexBoneWeights` (0x7C77F0)
    pub fn rp_skin_get_vertex_bone_weights(skin: GPtr<u8>) -> GPtr<f32> = cdecl 0x7C77F0 => unported;
    /// `RpClumpDestroy` (0x74A310)
    pub fn rp_clump_destroy(clump: GPtr<u8>) -> i32 = cdecl 0x74A310 => unported;
    /// `RpClumpClone` (0x749F70)
    pub fn rp_clump_clone(clump: GPtr<u8>) -> GPtr<u8> = cdecl 0x749F70 => unported;
    /// `RpAnimBlend::RpAnimBlendCreateAnimationForHierarchy` (0x4D60E0)
    pub fn rp_anim_blend_create_animation_for_hierarchy(hier: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D60E0 => unported;
    /// `RtAnimInterpolatorSetCurrentAnim` (0x7CD5A0)
    pub fn rt_anim_interpolator_set_current_anim(interp: GPtr<u8>, anim: GPtr<u8>) -> i32 = cdecl 0x7CD5A0 => unported;
    /// `RpAnimBlend::RpAnimBlendClumpInit` (0x4D6720)
    pub fn rp_anim_blend_clump_init(clump: GPtr<u8>) = cdecl 0x4D6720 => unported;
    /// `CClumpModelInfo::~CClumpModelInfo` (0x4C5750)
    pub fn clump_destructor(this: MiPtr) = thiscall 0x4C5750 => unported;
}

fn rw(this: MiPtr) -> GPtr<u8> {
    f::<GPtr<u8>>(this, mi::RW_OBJECT).read()
}

fn frame_of(object: GPtr<u8>) -> GPtr<u8> {
    object.byte_add(4).cast::<GPtr<u8>>().read()
}

/// `CClumpModelInfo::SetAnimFile(const char*)` (0x4C5200, `ret 4`).
#[hook(addr = 0x4C5200, conv = thiscall, name = "CClumpModelInfo::SetAnimFile", expect = "56 8B 74 24 08", this = "CClumpModelInfo")]
pub fn set_anim_file(this: MiPtr, name: GPtr<u8>) {
    if stricmp(name, GPtr::from_addr(STR_NULL as usize)) == 0 {
        return;
    }
    let mut len = 0u32;
    while name.byte_add(len as isize).read() != 0 {
        len = len.wrapping_add(1);
    }
    let p = operator_new(len.wrapping_add(1));
    f::<GPtr<u8>>(this, ANIM_FILE).write(p);
    let mut k = 0isize;
    loop {
        let c = name.byte_add(k).read();
        p.byte_add(k).write(c);
        if c == 0 {
            break;
        }
        k += 1;
    }
}

/// `CClumpModelInfo::ConvertAnimFileIndex` (0x4C5250): name → anim block index, name freed.
#[hook(addr = 0x4C5250, conv = thiscall, name = "CClumpModelInfo::ConvertAnimFileIndex", expect = "56 8B F1 8B 46 20", this = "CClumpModelInfo")]
pub fn convert_anim_file_index(this: MiPtr) {
    let p = f::<GPtr<u8>>(this, ANIM_FILE).read();
    if p.addr() as u32 == 0xFFFF_FFFF {
        return;
    }
    let idx = get_animation_block_index(p);
    free(f::<GPtr<u8>>(this, ANIM_FILE).read());
    f::<i32>(this, ANIM_FILE).write(idx);
}

/// `CClumpModelInfo::GetBoundingBox` (0x4C5710): `m_pColModel` (+0x14) (unhookable: 4 bytes).
pub fn get_bounding_box(this: MiPtr) -> GPtr<u8> {
    f::<GPtr<u8>>(this, mi::COL_MODEL).read()
}

/// `CClumpModelInfo::GetModelType` (0x4C5720): AL = 5 (unhookable).
pub fn get_model_type(_this: MiPtr) -> u8 {
    5
}

/// `CClumpModelInfo::GetRwModelType` (0x4C5730): 2 (`rpCLUMP`).
#[hook(addr = 0x4C5730, conv = thiscall, name = "CClumpModelInfo::GetRwModelType", expect = "B8 02 00 00 00", this = "CClumpModelInfo")]
pub fn get_rw_model_type(_this: MiPtr) -> u32 {
    2
}

/// `CClumpModelInfo::GetAnimFileIndex` (0x4C5740): +0x20 (unhookable).
pub fn get_anim_file_index(this: MiPtr) -> i32 {
    f::<i32>(this, ANIM_FILE).read()
}

/// `CClumpModelInfo::Init` (0x4C4E40): base init, anim file index -1.
#[hook(addr = 0x4C4E40, conv = thiscall, name = "CClumpModelInfo::Init", expect = "56 8B F1 E8 C8 FC FF FF", this = "CClumpModelInfo")]
pub fn init(this: MiPtr) {
    base_init(this);
    f::<i32>(this, ANIM_FILE).write(-1);
}

/// `CClumpModelInfo::Shutdown` (0x4C4E60): `jmp CBaseModelInfo::Shutdown`.
#[hook(addr = 0x4C4E60, conv = thiscall, name = "CClumpModelInfo::Shutdown", expect = "E9 EB FE FF FF 90", this = "CClumpModelInfo")]
pub fn shutdown(this: MiPtr) {
    base_shutdown(this)
}

/// `CClumpModelInfo::DeleteRwObject` (0x4C4E70).
#[hook(addr = 0x4C4E70, conv = thiscall, name = "CClumpModelInfo::DeleteRwObject", expect = "56 8B F1 8B 46 1C", this = "CClumpModelInfo")]
pub fn delete_rw_object(this: MiPtr) {
    let clump = rw(this);
    if clump.is_null() {
        return;
    }
    let a = get_2d_effect_atomic(clump);
    if !a.is_null() {
        add_2dfx_count(this, atomic_2dfx_count(a), true);
    }
    rp_clump_destroy(rw(this));
    f::<u32>(this, mi::RW_OBJECT).write(0);
    remove_tex_dictionary_ref(this);
    if vcall_get_anim_file_index(this) != -1 {
        remove_anim_block_ref(vcall_get_anim_file_index(this));
    }
    if f::<u8>(this, 0x13).read() & 8 != 0 {
        delete_collision_model(this);
    }
}

/// `CClumpModelInfo::SetHierarchyForSkinAtomic(RpAtomic*, void*)` (0x4C4EF0, cdecl callback).
#[hook(addr = 0x4C4EF0, conv = cdecl, name = "CClumpModelInfo::SetHierarchyForSkinAtomic", expect = "8B 44 24 08 85 C0")]
pub fn set_hierarchy_for_skin_atomic(atomic: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> {
    if !data.is_null() {
        rp_skin_atomic_set_hanim_hierarchy(atomic, data);
        return GPtr::NULL;
    }
    let h = get_anim_hierarchy_from_frame(frame_of(atomic));
    rp_skin_atomic_set_hanim_hierarchy(atomic, h);
    atomic
}

/// `CClumpModelInfo::AtomicSetupLightingCB(RpAtomic*, void*)` (0x4C4F30, cdecl).
#[hook(addr = 0x4C4F30, conv = cdecl, name = "CClumpModelInfo::AtomicSetupLightingCB", expect = "56 8B 74 24 08")]
pub fn atomic_setup_lighting_cb(atomic: GPtr<u8>, _data: GPtr<u8>) -> GPtr<u8> {
    if is_cbpc_pipeline_attached(atomic) != 0 {
        custom_building_atomic_setup(atomic);
    } else if is_ccpc_pipeline_attached(atomic) != 0 {
        custom_car_pipe_atomic_setup(atomic);
    }
    atomic
}

/// The 2d-effect plugin offset as `SetClump`'s protection chunk computes it.
fn plugin_offset_via_chunk() -> u32 {
    GPtr::<u32>::from_addr((PROT_SETCLUMP_A.get() ^ PROT_SETCLUMP_B.get()) as usize).read()
}

/// Normalises one vertex's four bone weights as the original's x87 sequence (PC=24).
pub fn normalise_weights(w: [f32; 4]) -> [f32; 4] {
    let s = X87::from_f32(w[1]).fadd_m(w[0]).fadd_m(w[2]).fadd_m(w[3]);
    let r = s.fdivr_m(1.0);
    [r.fmul_m(w[0]).to_f32(), r.fmul_m(w[1]).to_f32(), r.fmul_m(w[2]).to_f32(), r.fmul_m(w[3]).to_f32()]
}

/// `CClumpModelInfo::SetClump(RpClump*)` (0x4C4F70, `ret 4`).
#[hook(addr = 0x4C4F70, conv = thiscall, name = "CClumpModelInfo::SetClump", expect = "56 57 8B F9 8B 47 1C", this = "CClumpModelInfo")]
pub fn set_clump(this: MiPtr, clump: GPtr<u8>) {
    let old = rw(this);
    if !old.is_null() {
        let a = get_2d_effect_atomic(old);
        if !a.is_null() {
            let geometry = a.byte_add(0x18).cast::<GPtr<u8>>().read();
            add_2dfx_count(this, geometry_2dfx_count(geometry, plugin_offset_via_chunk()), true);
        }
    }
    f::<GPtr<u8>>(this, mi::RW_OBJECT).write(clump);
    if !clump.is_null() {
        let a = get_2d_effect_atomic(clump);
        if !a.is_null() {
            add_2dfx_count(this, atomic_2dfx_count(a), false);
        }
    }
    set_clump_model_info(rw(this), this);
    add_tex_dictionary_ref(this);
    if vcall_get_anim_file_index(this) != -1 {
        add_anim_block_ref(vcall_get_anim_file_index(this));
    }
    rp_clump_for_all_atomics(clump, ATOMIC_SETUP_LIGHTING_CB, this.addr() as u32);
    let first = get_first_atomic(clump);
    if first.is_null() {
        return;
    }
    if rp_skin_geometry_get_skin(first.byte_add(0x18).cast::<GPtr<u8>>().read()).is_null() {
        return;
    }
    if f::<u8>(this, 0x13).read() & 2 != 0 {
        rp_clump_for_all_atomics(clump, SET_HIERARCHY_FOR_SKIN_ATOMIC, 0);
        return;
    }
    let a = get_first_atomic(clump);
    let geometry = a.byte_add(0x18).cast::<GPtr<u8>>().read();
    let morph = geometry.byte_add(0x5C).cast::<GPtr<f32>>().read();
    let radius = morph.byte_add(0x10);
    radius.write(X87::from_f32(radius.read()).fmul_m(f32::from_bits(K_1_2)).to_f32());
    let hier = get_anim_hierarchy_from_clump(clump);
    rp_clump_for_all_atomics(clump, SET_HIERARCHY_FOR_SKIN_ATOMIC, hier.addr() as u32);
    let a = get_first_atomic(clump);
    let skin = rp_skin_geometry_get_skin(a.byte_add(0x18).cast::<GPtr<u8>>().read());
    let geometry = a.byte_add(0x18).cast::<GPtr<u8>>().read();
    let mut i = 0i32;
    while i < geometry.byte_add(0x14).cast::<i32>().read() {
        let w = rp_skin_get_vertex_bone_weights(skin).byte_add(16 * i as isize).cast::<[f32; 4]>();
        w.write(normalise_weights(w.read()));
        i += 1;
    }
    hier.cast::<u32>().write(0x3000);
}

/// `CClumpModelInfo::CreateInstance(RwMatrix*)` (0x4C5110, `ret 4`).
#[hook(addr = 0x4C5110, conv = thiscall, name = "CClumpModelInfo::CreateInstance-mat", expect = "8B 41 1C 85 C0", this = "CClumpModelInfo")]
pub fn create_instance_mat(this: MiPtr, m: GPtr<RwMatrix>) -> GPtr<u8> {
    if rw(this).is_null() {
        return GPtr::NULL;
    }
    let clump = vcall_create_instance(this);
    frame_of(clump).byte_add(0x10).cast::<RwMatrix>().write(m.read());
    clump
}

/// `CClumpModelInfo::CreateInstance()` (0x4C5140).
#[hook(addr = 0x4C5140, conv = thiscall, name = "CClumpModelInfo::CreateInstance-void", expect = "56 8B F1 8B 46 1C", this = "CClumpModelInfo")]
pub fn create_instance_void(this: MiPtr) -> GPtr<u8> {
    if rw(this).is_null() {
        return GPtr::NULL;
    }
    add_ref(this);
    let clone = rp_clump_clone(rw(this));
    let a = get_first_atomic(clone);
    if !a.is_null()
        && !rp_skin_geometry_get_skin(a.byte_add(0x18).cast::<GPtr<u8>>().read()).is_null()
        && f::<u8>(this, 0x13).read() & 2 == 0
    {
        let hier = get_anim_hierarchy_from_clump(clone);
        rp_clump_for_all_atomics(clone, SET_HIERARCHY_FOR_SKIN_ATOMIC, hier.addr() as u32);
        let anim = rp_anim_blend_create_animation_for_hierarchy(hier);
        rt_anim_interpolator_set_current_anim(hier.byte_add(0x20).cast::<GPtr<u8>>().read(), anim);
        hier.cast::<u32>().write(0x3000);
    }
    if f::<u8>(this, 0x13).read() & 1 != 0 {
        rp_anim_blend_clump_init(clone);
        let idx = f::<u32>(this, ANIM_FILE).read();
        let block = GPtr::<u8>::from_addr(((idx << 5).wrapping_add(ANIM_BLOCKS)) as usize);
        let anim = get_animation(f::<u32>(this, mi::KEY).read(), block);
        if !anim.is_null() {
            blend_animation(clone, anim, 2, 1.0);
        }
    }
    remove_ref(this);
    clone
}

/// `CClumpModelInfo::SetAtomicRendererCB(RpAtomic*, void*)` (0x4C5280, cdecl).
#[hook(addr = 0x4C5280, conv = cdecl, name = "CClumpModelInfo::SetAtomicRendererCB", expect = "8B 44 24 08 56")]
pub fn set_atomic_renderer_cb(atomic: GPtr<u8>, data: u32) -> GPtr<u8> {
    set_atomic_render_callback(atomic, data);
    atomic
}

/// `{ name or id, frame }` the frame-search callbacks receive (`tFrameSearchData`).
#[repr(C)]
#[derive(Clone, Copy)]
struct SearchData {
    key: u32,
    frame: GPtr<u8>,
}
// SAFETY: an integer and a game pointer.
#[allow(unsafe_code)]
unsafe impl sa_core::mem::GameData for SearchData {}

/// `found ? null : frame` (the original's `setne al; dec eax; and eax, frame`).
fn recurse(frame: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> {
    rw_frame_for_all_children(frame, cb, data);
    let found = data.byte_add(4).cast::<GPtr<u8>>().read();
    if found.is_null() { frame } else { GPtr::NULL }
}

fn name_matches(frame: GPtr<u8>, data: GPtr<u8>) -> bool {
    let node = get_frame_node_name(frame);
    stricmp(data.cast::<GPtr<u8>>().read(), node) == 0
}

/// `CClumpModelInfo::FindFrameFromNameCB(RwFrame*, void*)` (0x4C52A0, cdecl).
#[hook(addr = 0x4C52A0, conv = cdecl, name = "CClumpModelInfo::FindFrameFromNameCB", expect = "56 8B 74 24 08")]
pub fn find_frame_from_name_cb(frame: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> {
    if name_matches(frame, data) {
        data.byte_add(4).cast::<GPtr<u8>>().write(frame);
        return GPtr::NULL;
    }
    recurse(frame, FIND_FRAME_FROM_NAME_CB, data)
}

/// `CClumpModelInfo::FindFrameFromNameWithoutIdCB(RwFrame*, void*)` (0x4C52F0, cdecl): frames
/// with a hierarchy id are skipped (searched below only).
#[hook(addr = 0x4C52F0, conv = cdecl, name = "CClumpModelInfo::FindFrameFromNameWithoutIdCB", expect = "56 8B 74 24 08")]
pub fn find_frame_from_name_without_id_cb(frame: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> {
    if get_frame_hierarchy_id(frame) == 0 && name_matches(frame, data) {
        data.byte_add(4).cast::<GPtr<u8>>().write(frame);
        return GPtr::NULL;
    }
    recurse(frame, FIND_FRAME_FROM_NAME_WITHOUT_ID_CB, data)
}

/// `CClumpModelInfo::FindFrameFromIdCB(RwFrame*, void*)` (0x4C5350, cdecl).
#[hook(addr = 0x4C5350, conv = cdecl, name = "CClumpModelInfo::FindFrameFromIdCB", expect = "56 8B 74 24 08")]
pub fn find_frame_from_id_cb(frame: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> {
    let id = get_frame_hierarchy_id(frame);
    if data.cast::<i32>().read() == id {
        data.byte_add(4).cast::<GPtr<u8>>().write(frame);
        return GPtr::NULL;
    }
    recurse(frame, FIND_FRAME_FROM_ID_CB, data)
}

/// `CClumpModelInfo::FillFrameArrayCB(RwFrame*, void*)` (0x4C5390, cdecl): `array[id] = frame` for
/// id > 0 (signed), recursion, returns `frame`.
#[hook(addr = 0x4C5390, conv = cdecl, name = "CClumpModelInfo::FillFrameArrayCB", expect = "56 8B 74 24 08")]
pub fn fill_frame_array_cb(frame: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> {
    let id = get_frame_hierarchy_id(frame);
    if id > 0 {
        data.cast::<GPtr<u8>>().wrapping_index(id).write(frame);
    }
    rw_frame_for_all_children(frame, FILL_FRAME_ARRAY_CB, data);
    frame
}

fn search(clump: GPtr<u8>, key: u32, cb: u32) -> GPtr<u8> {
    let mut d = SearchData { key, frame: GPtr::NULL };
    let p = GPtr::from_raw((&raw mut d).cast::<u8>());
    rw_frame_for_all_children(frame_of(clump), cb, p);
    p.byte_add(4).cast::<GPtr<u8>>().read()
}

/// `CClumpModelInfo::GetFrameFromId(RpClump*, int32)` (0x4C53C0, cdecl).
#[hook(addr = 0x4C53C0, conv = cdecl, name = "CClumpModelInfo::GetFrameFromId", expect = "83 EC 08 8B 44 24 10")]
pub fn get_frame_from_id(clump: GPtr<u8>, id: i32) -> GPtr<u8> {
    search(clump, id as u32, FIND_FRAME_FROM_ID_CB)
}

/// `CClumpModelInfo::GetFrameFromName(RpClump*, const char*)` (0x4C5400, cdecl).
#[hook(addr = 0x4C5400, conv = cdecl, name = "CClumpModelInfo::GetFrameFromName", expect = "83 EC 08 8B 44 24 10")]
pub fn get_frame_from_name(clump: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> {
    search(clump, name.addr() as u32, FIND_FRAME_FROM_NAME_CB)
}

/// `CClumpModelInfo::FillFrameArray(RpClump*, RwFrame**)` (0x4C5440, cdecl).
#[hook(addr = 0x4C5440, conv = cdecl, name = "CClumpModelInfo::FillFrameArray", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn fill_frame_array(clump: GPtr<u8>, array: GPtr<u8>) {
    rw_frame_for_all_children(frame_of(clump), FILL_FRAME_ARRAY_CB, array);
}

/// `CClumpModelInfo::SetFrameIds(RwObjectNameIdAssocation*)` (0x4C5460, `ret 4`): entries of 0x0C
/// bytes `{ name, id, flags }` up to a null name; entries with `flags & 1` are skipped.
#[hook(addr = 0x4C5460, conv = thiscall, name = "CClumpModelInfo::SetFrameIds", expect = "83 EC 08 55 56", this = "CClumpModelInfo")]
pub fn set_frame_ids(this: MiPtr, ids: GPtr<u8>) {
    let mut e = ids;
    while e.cast::<u32>().read() != 0 {
        if e.byte_add(8).read() & 1 == 0 {
            let name = e.cast::<u32>().read();
            let frame = search(rw(this), name, FIND_FRAME_FROM_NAME_WITHOUT_ID_CB);
            if !frame.is_null() {
                set_frame_hierarchy_id(frame, e.byte_add(4).cast::<i32>().read());
            }
        }
        e = e.byte_add(0x0C);
    }
}

/// `CClumpModelInfo`'s scalar deleting destructor (0x4C62B0).
#[hook(addr = 0x4C62B0, conv = thiscall, name = "CClumpModelInfo::vfunc_0", expect = "56 8B F1 E8 98 F4 FF FF", this = "CClumpModelInfo")]
pub fn scalar_deleting_destructor(this: MiPtr, flags: u32) -> MiPtr {
    super::atomic::scalar_delete(this, flags, clump_destructor)
}

/// `CClumpModelInfo::SetClumpModelInfoFlags(CClumpModelInfo*, uint32)` (0x5B3C30, cdecl):
/// `SetBaseModelInfoFlags`, then bit 2 of flag byte +0x13 = IDE bit 0x20.
#[hook(addr = 0x5B3C30, conv = cdecl, name = "CClumpModelInfo::SetClumpModelInfoFlags", expect = "8B 54 24 04 53")]
pub fn set_clump_model_info_flags(m: MiPtr, flags: u32) {
    set_base_model_info_flags(m, flags);
    let p = f::<u8>(m, 0x13);
    p.write(if flags & 0x20 != 0 { p.read() | 4 } else { p.read() & 0xFB });
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[HookReg] = &[
    HOOK_SET_ANIM_FILE,
    HOOK_CONVERT_ANIM_FILE_INDEX,
    HOOK_GET_RW_MODEL_TYPE,
    HOOK_INIT,
    HOOK_SHUTDOWN,
    HOOK_DELETE_RW_OBJECT,
    HOOK_SET_HIERARCHY_FOR_SKIN_ATOMIC,
    HOOK_ATOMIC_SETUP_LIGHTING_CB,
    HOOK_SET_CLUMP,
    HOOK_CREATE_INSTANCE_MAT,
    HOOK_CREATE_INSTANCE_VOID,
    HOOK_SET_ATOMIC_RENDERER_CB,
    HOOK_FIND_FRAME_FROM_NAME_CB,
    HOOK_FIND_FRAME_FROM_NAME_WITHOUT_ID_CB,
    HOOK_FIND_FRAME_FROM_ID_CB,
    HOOK_FILL_FRAME_ARRAY_CB,
    HOOK_GET_FRAME_FROM_ID,
    HOOK_GET_FRAME_FROM_NAME,
    HOOK_FILL_FRAME_ARRAY,
    HOOK_SET_FRAME_IDS,
    HOOK_SCALAR_DELETING_DESTRUCTOR,
    HOOK_SET_CLUMP_MODEL_INFO_FLAGS,
];
