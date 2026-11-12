//! `CClothesBuilder`: the tree under `CreateSkinnedClump` (0x5A69D0) that builds the player's skinned
//! clump from a `CPedClothesDesc` (WRITER-CLUMP). Ported from the 1.0 US bytes; gta-reversed's
//! `ClothesBuilder.cpp` was only a guide (several of its versions differ from the original: the
//! change flags, the clothes-rule interpreter, the bone index array, the texture blending loops).
//!
//! Static data (raw addresses, phase B reaches them through the legacy arena):
//! - `playerImg` `CDirectory` 0xBC12C0 with 0x226 entries at 0xBBCDC8 (`MODELS\PLAYER.IMG`);
//! - `ms_ratiosHaveChanged` 0x8D0AA4 (fat or muscle stat changed), `ms_geometryHasChanged` 0x8D0AA5 (the
//!   stats or a model key changed), `ms_textureHasChanged` 0x8D0AA6 (a texture key changed);
//! - the previous description passed to `CreateSkinnedClump`, 0xBC1288;
//! - `gBoneIndices` 0xBBC8C8: `int16[10][64]`, the bone IDs of each clothes model's hierarchy;
//! - the default-TXD rotor 0xBC12D0 (0..3) of `RequestTexture`;
//! - the name tables (`const char*` arrays in `.data`): 0x8D0A38 (5 default model names), 0x8D0A4C..0x8D0AA0
//!   (texture names `ConstructTextures` looks up and gives), 0x8D0A7C (the 10 body-part texture names).
//!
//! `CPedClothesDesc` (0x78): +0 model keys[10], +0x28 texture keys[18], +0x70 fat stat, +0x74 muscle stat.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `playerImg` (`CDirectory`)
const PLAYER_IMG: usize = 0xBC12C0;
/// `playerImgEntries` (0x226 `CDirectory::DirectoryInfo`)
const PLAYER_IMG_ENTRIES: usize = 0xBBCDC8;
/// `ms_ratiosHaveChanged`, `ms_geometryHasChanged`, `ms_textureHasChanged` (bytes)
const RATIOS_CHANGED: usize = 0x8D0AA4;
const GEOMETRY_CHANGED: usize = 0x8D0AA5;
const TEXTURE_CHANGED: usize = 0x8D0AA6;
/// The `oldClothes` argument of the last `CreateSkinnedClump` call
const LAST_OLD_DESC: usize = 0xBC1288;
/// `gBoneIndices` (`int16[10][64]`)
const BONE_INDICES: usize = 0xBBC8C8;
/// The default TXD rotor of `RequestTexture`
const DEFAULT_TXD_ROTOR: usize = 0xBC12D0;
/// `CClothes::ms_clothesImageId`
const CLOTHES_IMAGE_ID: usize = 0xBC12F8;
/// `CClothes::ms_numRuleTags`, `CClothes::ms_clothesRules`
const NUM_RULE_TAGS: usize = 0xBC12FC;
const CLOTHES_RULES: usize = 0xBC1300;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CTxdStore::ms_pTxdPool`
const TXD_POOL: usize = 0xC8800C;
/// The first clothes model (`MODEL_CLOTHES01`)
const CLOTHES_MODEL: i32 = 0x180;
/// The streaming index of TXD slot 0
const TXD_INDEX: i32 = 20000;

orig! {
    /// `CPedClothesDesc::CPedClothesDesc` (0x5A8020)
    fn desc_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x5A8020 => crate::clothes::desc_constructor;
    /// `CPedClothesDesc::SetModel(uint32, eClothesModelPart)` (0x5A7910)
    fn set_model_hash(this: GPtr<u8>, key: u32, part: i32) = thiscall 0x5A7910 => crate::clothes::set_model_hash;
    /// `CPedClothesDesc::SetModel(const char*, eClothesModelPart)` (0x5A7920)
    fn set_model_name(this: GPtr<u8>, name: u32, part: i32) = thiscall 0x5A7920 => crate::clothes::set_model_name;
    /// `CPedClothesDesc::SetTextureAndModel(uint32, uint32, eClothesTexturePart)` (0x5A8050)
    fn set_texture_and_model(this: GPtr<u8>, tex: u32, model: u32, part: i32) = thiscall 0x5A8050 => crate::clothes::set_texture_and_model_hash;
    /// `CClothes::GetDependentTexture(eClothesModelPart)` (0x5A7F30)
    fn get_dependent_texture(m: i32) -> i32 = cdecl 0x5A7F30 => crate::clothes::get_dependent_texture;
    /// `CDirectory::Init(int32 capacity, DirectoryInfo* entries)` (0x5322F0)
    fn directory_init(dir: GPtr<u8>, capacity: u32, entries: GPtr<u8>) = thiscall 0x5322F0 => unported;
    /// `CDirectory::ReadDirFile(const char*)` (0x532350)
    fn directory_read_dir_file(dir: GPtr<u8>, name: GPtr<u8>) = thiscall 0x532350 => unported;
    /// `CDirectory::FindItem(uint32 key, uint32& offset, uint32& size)` (0x5324D0)
    fn directory_find_item(dir: GPtr<u8>, key: u32, offset: GPtr<u32>, size: GPtr<u32>) -> u8 = thiscall 0x5324D0 => unported;
    /// `CKeyGen::AppendStringToKey(uint32, const char*)` (0x53CF70)
    fn append_string_to_key(key: u32, s: GPtr<u8>) -> u32 = cdecl 0x53CF70 => unported;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    fn get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => unported;
    /// `CStreaming::RequestFile(int32 index, int32 offset, int32 size, int32 imgId, int32 flags)` (0x40A080)
    fn request_file(index: i32, offset: u32, size: u32, img: i32, flags: i32) = cdecl 0x40A080 => unported;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    fn load_all_requested_models(only_priority: u8) = cdecl 0x40EA10 => unported;
    /// `CStreaming::LoadRequestedModels()` (0x40E3A0)
    fn load_requested_models() = cdecl 0x40E3A0 => unported;
    /// `CStreaming::RemoveModel(int32)` (0x4089A0)
    fn remove_model(index: i32) = cdecl 0x4089A0 => unported;
    /// `FUN_007316d0(int32 i)` (0x7316D0): the TXD slot of default TXD `i`
    fn default_txd_slot(i: i32) -> i32 = cdecl 0x7316D0 => crate::default_txd_slot::default_txd_slot;
    /// `CStats::GetStatValue(eStats)` (0x558E40, ST0)
    fn get_stat_value(stat: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `CTimer::Suspend()` (0x5619D0)
    fn timer_suspend() = cdecl 0x5619D0 => unported;
    /// `CTimer::Resume()` (0x561A00)
    fn timer_resume() = cdecl 0x561A00 => unported;
    /// CRT `operator new` (0x82119A)
    fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// CRT `operator delete` (0x8214BD)
    fn operator_delete(p: GPtr<u8>) = cdecl 0x8214BD => unported;

    /// `RwHelper::GetFirstAtomic(RpClump*)` (0x734820)
    fn get_first_atomic(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734820 => unported;
    /// `GetFirstTexture(RwTexDictionary*)` (0x734940)
    fn get_first_texture(d: GPtr<u8>) -> GPtr<u8> = cdecl 0x734940 => unported;
    /// `CClothesBuilder::CopyTexture(RwTexture*)` (0x5A5730)
    fn copy_texture(t: GPtr<u8>) -> GPtr<u8> = cdecl 0x5A5730 => unported;
    /// `RpClumpForAllAtomics(RpClump*, callback, void*)` (0x749B70)
    fn rp_clump_for_all_atomics(c: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `RwTexDictionaryForAllTextures(RwTexDictionary*, callback, void*)` (0x7F3730)
    fn rw_tex_dictionary_for_all_textures(d: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x7F3730 => unported;
    /// `RwTexDictionaryFindNamedTexture(RwTexDictionary*, const char*)` (0x7F39F0)
    fn rw_tex_dictionary_find_named_texture(d: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F39F0 => unported;
    /// `RwTexDictionaryAddTexture(RwTexDictionary*, RwTexture*)` (0x7F3980)
    fn rw_tex_dictionary_add_texture(d: GPtr<u8>, t: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3980 => unported;
    /// `RwTexDictionaryRemoveTexture(RwTexture*)` (0x7F39C0)
    fn rw_tex_dictionary_remove_texture(t: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F39C0 => unported;
    /// `RwTexDictionarySetCurrent(RwTexDictionary*)` (0x7F3A70)
    fn rw_tex_dictionary_set_current(d: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3A70 => unported;
    /// `RwTextureDestroy(RwTexture*)` (0x7F3820)
    fn rw_texture_destroy(t: GPtr<u8>) -> u32 = cdecl 0x7F3820 => unported;
    /// `RwTextureSetName(RwTexture*, const char*)` (0x7F38A0)
    fn rw_texture_set_name(t: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F38A0 => unported;
    /// `RwRasterLock(RwRaster*, uint8 level, int32 mode)` (0x7FB2D0)
    fn rw_raster_lock(r: GPtr<u8>, level: u32, mode: u32) -> GPtr<u8> = cdecl 0x7FB2D0 => unported;
    /// `RwRasterUnlock(RwRaster*)` (0x7FAEC0)
    fn rw_raster_unlock(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7FAEC0 => unported;
    /// `RwV3dNormalize(RwV3d* out, const RwV3d* in)` (0x7ED9B0, ST0 = the length)
    fn rw_v3d_normalize(out: GPtr<u8>, inp: GPtr<u8>) -> f64 = cdecl 0x7ED9B0 => unported;
    /// `RwFrameCreate()` (0x7F0410)
    fn rw_frame_create() -> GPtr<u8> = cdecl 0x7F0410 => unported;
    /// `RwFrameAddChild(RwFrame* parent, RwFrame* child)` (0x7F0B00)
    fn rw_frame_add_child(p: GPtr<u8>, c: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0B00 => unported;
    /// `RpMaterialCreate()` (0x74D990)
    fn rp_material_create() -> GPtr<u8> = cdecl 0x74D990 => unported;
    /// `RpMaterialSetTexture(RpMaterial*, RwTexture*)` (0x74DBC0)
    fn rp_material_set_texture(m: GPtr<u8>, t: GPtr<u8>) -> GPtr<u8> = cdecl 0x74DBC0 => unported;
    /// `RpMaterialDestroy(RpMaterial*)` (0x74DA20)
    fn rp_material_destroy(m: GPtr<u8>) -> u32 = cdecl 0x74DA20 => unported;
    /// `RpGeometryCreate(int32 numVerts, int32 numTriangles, uint32 format)` (0x74CA90)
    fn rp_geometry_create(nv: i32, nt: i32, format: u32) -> GPtr<u8> = cdecl 0x74CA90 => unported;
    /// `RpGeometryLock(RpGeometry*, int32 lockMode)` (0x74C7D0)
    fn rp_geometry_lock(gm: GPtr<u8>, mode: u32) -> GPtr<u8> = cdecl 0x74C7D0 => unported;
    /// `RpGeometryUnlock(RpGeometry*)` (0x74C800)
    fn rp_geometry_unlock(gm: GPtr<u8>) -> GPtr<u8> = cdecl 0x74C800 => unported;
    /// `_rpGeometryAddRef(RpGeometry*)` (0x74CCB0)
    fn rp_geometry_add_ref(gm: GPtr<u8>) -> GPtr<u8> = cdecl 0x74CCB0 => unported;
    /// `RpGeometryDestroy(RpGeometry*)` (0x74CCC0)
    fn rp_geometry_destroy(gm: GPtr<u8>) -> u32 = cdecl 0x74CCC0 => unported;
    /// `RpGeometryTriangleSetVertexIndices(const RpGeometry*, RpTriangle*, uint16, uint16, uint16)` (0x74C690)
    fn rp_geometry_triangle_set_vertex_indices(gm: GPtr<u8>, tri: GPtr<u8>, v0: u32, v1: u32, v2: u32) -> GPtr<u8> = cdecl 0x74C690 => unported;
    /// `RpGeometryTriangleSetMaterial(RpGeometry*, RpTriangle*, RpMaterial*)` (0x74C6C0)
    fn rp_geometry_triangle_set_material(gm: GPtr<u8>, tri: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x74C6C0 => unported;
    /// `RpMorphTargetCalcBoundingSphere(const RpMorphTarget*, RwSphere*)` (0x74C200)
    fn rp_morph_target_calc_bounding_sphere(mt: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = cdecl 0x74C200 => unported;
    /// `RpAtomicCreate()` (0x749C50)
    fn rp_atomic_create() -> GPtr<u8> = cdecl 0x749C50 => unported;
    /// `RpAtomicSetGeometry(RpAtomic*, RpGeometry*, uint32 flags)` (0x749D40)
    fn rp_atomic_set_geometry(a: GPtr<u8>, gm: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x749D40 => unported;
    /// `RpAtomicSetFrame(RpAtomic*, RwFrame*)` (0x74BF20)
    fn rp_atomic_set_frame(a: GPtr<u8>, f: GPtr<u8>) -> GPtr<u8> = cdecl 0x74BF20 => unported;
    /// `RpClumpCreate()` (0x74A290)
    fn rp_clump_create() -> GPtr<u8> = cdecl 0x74A290 => unported;
    /// `RpClumpAddAtomic(RpClump*, RpAtomic*)` (0x74A490)
    fn rp_clump_add_atomic(c: GPtr<u8>, a: GPtr<u8>) -> GPtr<u8> = cdecl 0x74A490 => unported;
    /// `RpSkinGeometryGetSkin(RpGeometry*)` (0x7C7550)
    fn rp_skin_geometry_get_skin(gm: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7550 => unported;
    /// `RpSkinGeometrySetSkin(RpGeometry*, RpSkin*)` (0x7C7560)
    fn rp_skin_geometry_set_skin(gm: GPtr<u8>, skin: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7560 => unported;
    /// `RpSkinCreate(numVertices, numBones, RwMatrixWeights*, uint32* indices, RwMatrix* inverseMatrices)` (0x7C75B0)
    fn rp_skin_create(nv: u32, nb: u32, weights: GPtr<u8>, indices: GPtr<u8>, inv: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C75B0 => unported;
    /// `RpSkinGetNumBones(RpSkin*)` (0x7C77E0)
    fn rp_skin_get_num_bones(skin: GPtr<u8>) -> u32 = cdecl 0x7C77E0 => unported;
    /// `RpSkinGetVertexBoneWeights(RpSkin*)` (0x7C77F0)
    fn rp_skin_get_vertex_bone_weights(skin: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C77F0 => unported;
    /// `RpSkinGetVertexBoneIndices(RpSkin*)` (0x7C7800)
    fn rp_skin_get_vertex_bone_indices(skin: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7800 => unported;
    /// `RpSkinGetSkinToBoneMatrices(RpSkin*)` (0x7C7810)
    fn rp_skin_get_skin_to_bone_matrices(skin: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7810 => unported;
    /// `RpSkinAtomicGetHAnimHierarchy(const RpAtomic*)` (0x7C7540)
    fn rp_skin_atomic_get_hanim_hierarchy(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7540 => unported;
    /// `RpSkinAtomicSetHAnimHierarchy(RpAtomic*, RpHAnimHierarchy*)` (0x7C7520)
    fn rp_skin_atomic_set_hanim_hierarchy(a: GPtr<u8>, h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7520 => unported;
    /// `RpSkinAtomicSetType(RpAtomic*, RpSkinType)` (0x7C7830)
    fn rp_skin_atomic_set_type(a: GPtr<u8>, ty: u32) -> GPtr<u8> = cdecl 0x7C7830 => unported;
    /// `RpHAnimHierarchyCreateFromHierarchy(RpHAnimHierarchy*, RpHAnimHierarchyFlag, int32 maxKeyFrameSize)` (0x7C4ED0)
    fn rp_hanim_hierarchy_create_from_hierarchy(h: GPtr<u8>, flags: u32, kfs: i32) -> GPtr<u8> = cdecl 0x7C4ED0 => unported;
    /// `RpHAnimFrameSetHierarchy(RwFrame*, RpHAnimHierarchy*)` (0x7C5130)
    fn rp_hanim_frame_set_hierarchy(f: GPtr<u8>, h: GPtr<u8>) -> u32 = cdecl 0x7C5130 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32 id)` (0x7C51A0)
    fn rp_hanim_id_get_index(h: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;

    /// `CClothesBuilder::RequestGeometry` (0x5A41C0)
    fn request_geometry_call(model: i32, key: u32) = cdecl 0x5A41C0 => request_geometry;
    /// `CClothesBuilder::RequestTexture` (0x5A4220)
    fn request_texture_call(key: u32) -> i32 = cdecl 0x5A4220 => request_texture;
    /// The blend-ratio helper 0x5A42B0 (register arguments)
    fn blend_ratios_call(desc: GPtr<u8>, fat: GPtr<f32>, muscle: GPtr<f32>, normal: GPtr<f32>) = reg("args = eax esi edi ebx") 0x5A42B0 => blend_ratios;
    /// `CClothesBuilder::PreprocessClothesDesc` (0x5A44C0)
    fn preprocess_clothes_desc_call(desc: GPtr<u8>, cutscene: u8) = cdecl 0x5A44C0 => preprocess_clothes_desc;
    /// `CClothesBuilder::AddWeightToBoneVertex` (0x5A4840)
    fn add_weight_to_bone_vertex_call(weights: GPtr<u8>, idx: GPtr<u8>, w: f32, target: u32) = cdecl 0x5A4840 => add_weight_to_bone_vertex;
    /// `CClothesBuilder::StoreBoneArray` (0x5A48B0)
    fn store_bone_array_call(clump: GPtr<u8>, i: i32) = cdecl 0x5A48B0 => store_bone_array;
    /// `CClothesBuilder::BlendGeometry` (3 frames, 0x5A4940)
    fn blend_geometry3_call(clump: GPtr<u8>, n0: GPtr<u8>, n1: GPtr<u8>, n2: GPtr<u8>, r0: f32, r1: f32, r2: f32) -> GPtr<u8> = cdecl 0x5A4940 => blend_geometry3;
    /// `CClothesBuilder::ConstructGeometryArray` (0x5A55A0)
    fn construct_geometry_array_call(out: GPtr<u8>, keys: GPtr<u8>, normal: f32, fat: f32, muscle: f32) = cdecl 0x5A55A0 => construct_geometry_array;
    /// `CClothesBuilder::BuildBoneIndexConversionTable` (0x5A56E0)
    fn build_bone_index_conversion_table_call(table: GPtr<u8>, h: GPtr<u8>, i: i32) = cdecl 0x5A56E0 => build_bone_index_conversion_table;
    /// `CClothesBuilder::PlaceTextureOnTopOfTexture` (0x5A57B0)
    fn place_texture_on_top_of_texture_call(dst: GPtr<u8>, src: GPtr<u8>) = cdecl 0x5A57B0 => place_texture_on_top_of_texture;
    /// `CClothesBuilder::BlendTextures(dst, src, r1, r2, n)` (0x5A5820)
    fn blend_textures2_call(dst: GPtr<u8>, src: GPtr<u8>, r1: f32, r2: f32, n: i32) = cdecl 0x5A5820 => blend_textures2;
    /// `CClothesBuilder::BlendTextures(dst, src1, src2, r1, r2, r3, n)` (0x5A59C0)
    fn blend_textures3_call(dst: GPtr<u8>, s1: GPtr<u8>, s2: GPtr<u8>, r1: f32, r2: f32, r3: f32, n: i32) = cdecl 0x5A59C0 => blend_textures3;
    /// `CClothesBuilder::BlendTextures(dst, src1, src2, r1, r2, r3, n, tattoos)` (0x5A5BC0)
    fn blend_textures_tattoo_call(dst: GPtr<u8>, s1: GPtr<u8>, s2: GPtr<u8>, r1: f32, r2: f32, r3: f32, n: i32, tat: GPtr<u8>) = cdecl 0x5A5BC0 => blend_textures_tattoo;
    /// `GetTextureFromTxdAndLoadNextTxd` (0x5A5F70)
    fn get_texture_from_txd_call(dst: GPtr<u8>, txd: i32, next_key: u32, next_txd: GPtr<i32>) -> GPtr<u8> = cdecl 0x5A5F70 => get_texture_from_txd_and_load_next_txd;
    /// `CClothesBuilder::ConstructTextures` (0x5A6040)
    fn construct_textures_call(dict: GPtr<u8>, keys: GPtr<u8>, normal: f32, fat: f32, muscle: f32) = cdecl 0x5A6040 => construct_textures;
    /// `CClothesBuilder::ConstructGeometryAndSkinArrays` (0x5A6530)
    fn construct_geometry_and_skin_arrays_call(h: GPtr<u8>, geo: GPtr<u8>, weights: GPtr<u8>, indices: GPtr<u8>, n: u32, geos: GPtr<u8>, mats: GPtr<u8>) = cdecl 0x5A6530 => construct_geometry_and_skin_arrays;
}

/// The address of a Rust local the original keeps on its stack (passed to game code as a pointer).
fn local<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(core::ptr::from_mut(v)).cast()
}

/// The `const char*` stored at `addr` (the name tables in `.data`).
fn name_at(addr: usize) -> GPtr<u8> {
    g::<GPtr<u8>>(addr).read()
}

/// A `.rdata` string literal.
fn rdata(addr: usize) -> GPtr<u8> {
    GPtr::from_addr(addr)
}

/// The `RwTexDictionary*` of TXD slot `id` as the inlined `CPool::GetAt` reads it: a free slot gives a
/// null `TxdDef*`, whose first dword the original then reads (address 0, a crash).
fn txd_dict(id: i32) -> GPtr<u8> {
    let pool = g::<GPtr<u8>>(TXD_POOL).read();
    let flags = rd::<GPtr<u8>>(pool, 4);
    if (flags.byte_add(id as isize).read() as i8) < 0 {
        panic!("CClothesBuilder: TXD slot {id} is free; the original reads address 0 here");
    }
    rd::<GPtr<u8>>(rd::<GPtr<u8>>(pool, 0).byte_add((id as isize).wrapping_mul(12)), 0)
}

/// `CClothesBuilder::RequestGeometry(int32 modelId, uint32 key)` (0x5A41C0, cdecl): sets
/// `bHasComplexHierarchy` (+0x13 bit 1) of the model info, looks `key + ".DFF"` up in `playerImg` and
/// requests that file for the model (image `CClothes::ms_clothesImageId`, flags 0x12). `FindItem`'s
/// result is ignored: when the entry is missing the size is the argument slot's old value (the model id,
/// the original passes its own argument slot) and the offset an uninitialised local (0 here).
#[hook(addr = 0x5A41C0, conv = cdecl, name = "CClothesBuilder::RequestGeometry", expect = "51 56 8B 74 24 0C",
       unverified = "clump-1", route = "L2", hazards = "uninit-locals", recs = "boot")]
pub fn request_geometry(model: i32, key: u32) {
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * model as usize).read();
    wr::<u8>(mi, 0x13, rd::<u8>(mi, 0x13) | 2);
    let k = append_string_to_key(key, rdata(0x867150)); // ".DFF"
    let mut offset = 0u32;
    let mut size = model as u32;
    directory_find_item(GPtr::from_addr(PLAYER_IMG), k, local(&mut offset).cast(), local(&mut size).cast());
    request_file(model, offset, size, g::<i32>(CLOTHES_IMAGE_ID).read(), 0x12);
}

/// `CClothesBuilder::RequestTexture(uint32 key)` (0x5A4220, cdecl): takes default TXD slot
/// `FUN_007316d0(rotor)` (before the key check), returns -1 for key 0; else advances the rotor 0xBC12D0
/// (wrapping 3 → 0), requests `key + ".TXD"` from `playerImg` into that slot and returns the slot.
/// Offset and size are uninitialised locals when `FindItem` fails (0 here).
#[hook(addr = 0x5A4220, conv = cdecl, name = "CClothesBuilder::RequestTexture", expect = "A1 D0 12 BC 00",
       unverified = "clump-1", route = "L2", hazards = "uninit-locals", recs = "boot")]
pub fn request_texture(key: u32) -> i32 {
    let slot = default_txd_slot(g::<i32>(DEFAULT_TXD_ROTOR).read());
    if key == 0 {
        return -1;
    }
    let n = g::<i32>(DEFAULT_TXD_ROTOR).read().wrapping_add(1);
    g::<i32>(DEFAULT_TXD_ROTOR).write(n);
    if n > 3 {
        g::<i32>(DEFAULT_TXD_ROTOR).write(0);
    }
    let k = append_string_to_key(key, rdata(0x867158)); // ".TXD"
    let (mut size, mut offset) = (0u32, 0u32);
    directory_find_item(GPtr::from_addr(PLAYER_IMG), k, local(&mut offset).cast(), local(&mut size).cast());
    request_file(slot.wrapping_add(TXD_INDEX), offset, size, g::<i32>(CLOTHES_IMAGE_ID).read(), 0x12);
    slot
}

/// The blend-ratio helper 0x5A42B0 (`CClothesBuilder::nullptr` in the catalogue; register arguments:
/// EAX = the `CPedClothesDesc`, ESI = out fat ratio, EDI = out muscle ratio, EBX = out normal ratio).
/// x87 as the original: `fat = max(desc.fatStat - 200, 0) * 0.00125`, `muscle = max(GetStatValue(23
/// MUSCLE), 0) * 0.001` (each clamped to at most 1 after the store), `normal = 1 - (fat + muscle)`; when
/// `fat + muscle > 1` both are scaled by `1 / (fat + muscle)` and `normal = 0`. NaN stats pass the
/// clamps unchanged.
#[hook(addr = 0x5A42B0, conv = reg("args = eax esi edi ebx"), name = "CClothesBuilder::nullptr", expect = "83 EC 08 8B 48 70",
       unverified = "clump-1", route = "L2", hazards = "none", recs = "boot")]
pub fn blend_ratios(desc: GPtr<u8>, fat: GPtr<f32>, muscle: GPtr<f32>, normal: GPtr<f32>) {
    let fat_stat = rd::<f32>(desc, 0x70);
    let mut mus = get_stat_value(23) as f32; // fstp dword
    let mut f = x(fat_stat).fsub_m(200.0);
    if x(mus).fcom(X87::zero()) == sa_math::x87::Cmp::Less {
        mus = 0.0;
    }
    if f.fcom(X87::zero()) == sa_math::x87::Cmp::Less {
        f = X87::zero();
    }
    fat.write(f32_of(f.fmul_m(0.00125)));
    muscle.write(f32_of(x(mus).fmul_m(0.001)));
    if x(fat.read()).fcom(X87::one()) == sa_math::x87::Cmp::Greater {
        fat.write(1.0);
    }
    if x(muscle.read()).fcom(X87::one()) == sa_math::x87::Cmp::Greater {
        muscle.write(1.0);
    }
    normal.write(f32_of(x(fat.read()).fadd_m(muscle.read()).fsubr_m(1.0)));
    let sum = x(fat.read()).fadd_m(muscle.read());
    if sum.fcom(X87::one()) == sa_math::x87::Cmp::Greater {
        let t = sum.fdivr_m(1.0);
        fat.write(f32_of(t.fmul_m(fat.read())));
        muscle.write(f32_of(t.fmul_m(muscle.read())));
        normal.write(0.0);
    }
}

/// The `RwTexDictionaryForAllTextures` callback 0x5A44A0 (`CClothesBuilder::nullptr` in the catalogue):
/// removes the texture from its dictionary, destroys it, returns it.
#[hook(addr = 0x5A44A0, conv = cdecl, name = "CClothesBuilder::nullptr", expect = "56 8B 74 24 08",
       unverified = "clump-1", route = "L2", hazards = "heap-writes", recs = "boot")]
pub fn destroy_texture_cb(tex: GPtr<u8>, _data: GPtr<u8>) -> GPtr<u8> {
    rw_tex_dictionary_remove_texture(tex);
    rw_texture_destroy(tex);
    tex
}

/// `CClothesBuilder::PreprocessClothesDesc(CPedClothesDesc&, bool cutscene)` (0x5A44C0, cdecl): runs the
/// clothes rules of `clothes.dat` (`CClothes::ms_clothesRules`, `ms_numRuleTags` dwords) over the
/// description. Each rule is a type dword and its parameters:
/// - 0 `A C`: for every model part whose key is A, set its model to C (cutscene builds only);
/// - 1 `A B C D`: for every part with key A: B == 2 → model part 2 = C; else (D = the texture key of
///   `GetDependentTexture(B)` when 0, C = model B when 0, both kept for the next parts)
///   `SetTextureAndModel(D, C, GetDependentTexture(B))`;
/// - 2 `A D`: for every part with key A, `SetTextureAndModel(D, A, GetDependentTexture(part))`;
/// - 3 `A B`: for every part with key A, clear model part B;
/// - 4/6 `K`: drop condition key K (4 marks it 1, 6 marks it 0); 5/7 `K`: add condition K (5 = must
///   be worn, 7 = must not; with all 8 slots used the parameter is not consumed and is read as the next
///   rule type, as in the original).
/// A rule acts on a part only if no active condition matches (`(model[sel] == key) == flag` for none of
/// them), `sel` = B for types 1 and 3, else the part. The conditions persist across rules; the flag bytes
/// are uninitialised in the original until a condition is added (zero here, never read before written).
#[hook(addr = 0x5A44C0, conv = cdecl, name = "CClothesBuilder::PreprocessClothesDesc", expect = "83 EC 3C 55 57",
       unverified = "clump-1", route = "L2", hazards = "none", recs = "boot")]
pub fn preprocess_clothes_desc(desc: GPtr<u8>, cutscene: u8) {
    let mut keys = [u32::MAX; 8];
    let mut flags = [0u8; 8];
    let (mut a, mut c, mut d) = (0u32, 0u32, 0u32);
    let rule = |i: u32| g::<u32>(CLOTHES_RULES + 4 * i as usize).read();
    let model = |p: u32| rd::<u32>(desc, (p as isize).wrapping_mul(4));
    if g::<u32>(NUM_RULE_TAGS).read() == 0 {
        return;
    }
    let mut p: u32 = 0;
    loop {
        let ty = rule(p);
        p = p.wrapping_add(1);
        let mut b = u32::MAX;
        match ty {
            0 => {
                a = rule(p);
                c = rule(p + 1);
                p += 2;
            }
            1 => {
                a = rule(p);
                b = rule(p + 1);
                c = rule(p + 2);
                d = rule(p + 3);
                p += 4;
            }
            2 => {
                a = rule(p);
                d = rule(p + 1);
                p += 2;
            }
            3 => {
                a = rule(p);
                b = rule(p + 1);
                p += 2;
            }
            4 | 6 => {
                let k = rule(p);
                a = k;
                p += 1;
                if let Some(i) = keys.iter().position(|&v| v == k) {
                    keys[i] = u32::MAX;
                    flags[i] = (ty == 4) as u8;
                }
            }
            5 | 7 => {
                if let Some(i) = keys.iter().position(|&v| v == u32::MAX) {
                    keys[i] = rule(p);
                    flags[i] = (ty == 5) as u8;
                    p += 1;
                }
            }
            _ => {}
        }
        for part in 0..10u32 {
            let sel = if b == u32::MAX { part } else { b };
            let blocked =
                (0..8).any(|k| keys[k] != u32::MAX && ((model(sel) == keys[k]) as u8) == flags[k]);
            if blocked {
                continue;
            }
            match ty {
                0 if cutscene != 0 && model(part) == a => set_model_hash(desc, c, part as i32),
                1 if model(part) == a => {
                    if b == 2 {
                        set_model_hash(desc, c, 2);
                    } else {
                        if d == 0 {
                            let t = get_dependent_texture(b as i32);
                            d = rd::<u32>(desc, 0x28 + (t as isize).wrapping_mul(4));
                        }
                        if c == 0 {
                            c = model(b);
                        }
                        let t = get_dependent_texture(b as i32);
                        set_texture_and_model(desc, d, c, t);
                    }
                }
                2 if model(part) == a => {
                    let t = get_dependent_texture(part as i32);
                    set_texture_and_model(desc, d, a, t);
                }
                3 if model(part) == a => set_model_hash(desc, 0, b as i32),
                _ => {}
            }
        }
        if p >= g::<u32>(NUM_RULE_TAGS).read() {
            break;
        }
    }
}

/// `CClothesBuilder::AddWeightToBoneVertex(float weights[8], uint8 bones[8], float w, uint32 bone)`
/// (0x5A4840, cdecl): nothing for `w == 0`; else the first slot whose weight is 0 takes the bone (low
/// byte) and `w + 0` and zeroes the next weight, or the first slot already holding `bone` (compared as a
/// zero-extended byte with the whole dword) gets `w + weight`. With all 8 slots taken the original adds
/// `w` to `weights[8]` (one past the array) — kept.
#[hook(addr = 0x5A4840, conv = cdecl, name = "CClothesBuilder::AddWeightToBoneVertex", expect = "D9 44 24 0C D8 1D 50 8B 85 00",
       unverified = "clump-1", route = "L2", hazards = "none", recs = "boot")]
pub fn add_weight_to_bone_vertex(weights: GPtr<u8>, bones: GPtr<u8>, w: f32, bone: u32) {
    if x(w).fcom(X87::zero()) == sa_math::x87::Cmp::Equal {
        return;
    }
    let mut i: isize = 0;
    while i < 8 {
        if x(rd::<f32>(weights, 4 * i)).fcom(X87::zero()) == sa_math::x87::Cmp::Equal {
            wr::<u8>(bones, i, bone as u8);
            wr::<f32>(weights, 4 * i + 4, 0.0);
            break;
        }
        if rd::<u8>(bones, i) as u32 == bone {
            break;
        }
        i += 1;
    }
    let v = x(w).fadd_m(rd::<f32>(weights, 4 * i));
    wr::<f32>(weights, 4 * i, f32_of(v));
}

/// `CClothesBuilder::StoreBoneArray(RpClump*, int32 i)` (0x5A48B0, cdecl): finds the atomic named
/// "normal" (`RpClumpForAllAtomics` with the callback 0x5A47E0), takes its skin's HAnim hierarchy and
/// copies the low 16 bits of every node's ID (`pNodeInfo[k].nodeID`, 16-byte entries) into
/// `gBoneIndices[i][k]`, then writes one -1 terminator when fewer than 64 nodes were copied (the rest of
/// the row keeps its old values).
#[hook(addr = 0x5A48B0, conv = cdecl, name = "CClothesBuilder::StoreBoneArray", expect = "83 EC 08 8B 4C 24 0C",
       unverified = "clump-1", route = "L2", hazards = "none", recs = "boot")]
pub fn store_bone_array(clump: GPtr<u8>, i: i32) {
    let mut ctx = [0u32; 2];
    let cp = local(&mut ctx);
    wr::<GPtr<u8>>(cp, 4, GPtr::NULL);
    wr::<GPtr<u8>>(cp, 0, rdata(0x867160)); // "normal"
    rp_clump_for_all_atomics(clump, 0x5A47E0, cp);
    let h = rp_skin_atomic_get_hanim_hierarchy(rd::<GPtr<u8>>(cp, 4));
    let row = g::<u16>(BONE_INDICES + (i as isize as usize).wrapping_mul(128));
    let mut k: i32 = 0;
    if rd::<i32>(h, 4) > 0 {
        loop {
            let id = rd::<u16>(rd::<GPtr<u8>>(h, 0x10), (k as isize) * 16);
            row.wrapping_index(k).write(id);
            k += 1;
            if k >= rd::<i32>(h, 4) {
                break;
            }
        }
        if k >= 64 {
            return;
        }
    }
    row.wrapping_index(k).write(0xFFFF);
}

/// `CClothesBuilder::BlendGeometry(RpClump*, const char* f0, const char* f1, const char* f2, float r0,
/// float r1, float r2)` (0x5A4940, cdecl): finds the three atomics by frame name (callback 0x5A47E0),
/// locks geometry 0 (`rpGEOMETRYLOCKALL`) and for each of its vertices blends position, normal
/// (re-normalised with `RwV3dNormalize`) and texture coordinates as `r0*v0 + r1*v1 + r2*v2` (x87, the
/// original's addition order per component), then rebuilds the skin weights: each geometry's 4 weights
/// times its ratio go through `AddWeightToBoneVertex` (geometry 0 first), the first 4 slots are stored
/// (bone indices and weights), and when a fifth slot is used the 4 are scaled by `1 / (w3+w2+w1+w0)`.
/// Unlocks, adds a reference and returns geometry 0. The weight / bone locals: the original zeroes 5
/// weights and 4 bone bytes (the others are written before they are read); all are zero here.
#[hook(addr = 0x5A4940, conv = cdecl, name = "CClothesBuilder::BlendGeometry-3", expect = "81 EC 80 00 00 00",
       unverified = "clump-1", route = "L2", hazards = "heap-writes", recs = "boot")]
pub fn blend_geometry3(clump: GPtr<u8>, n0: GPtr<u8>, n1: GPtr<u8>, n2: GPtr<u8>, r0: f32, r1: f32, r2: f32) -> GPtr<u8> {
    let mut ctx = [0u32; 2];
    let cp = local(&mut ctx);
    let mut atoms = [GPtr::<u8>::NULL; 3];
    for (k, n) in [n0, n1, n2].into_iter().enumerate() {
        wr::<GPtr<u8>>(cp, 4, GPtr::NULL);
        wr::<GPtr<u8>>(cp, 0, n);
        rp_clump_for_all_atomics(clump, 0x5A47E0, cp);
        atoms[k] = rd::<GPtr<u8>>(cp, 4);
    }
    let mut geo = [GPtr::<u8>::NULL; 3];
    let mut vtx = [GPtr::<u8>::NULL; 3];
    let mut nrm = [GPtr::<u8>::NULL; 3];
    let mut uv = [GPtr::<u8>::NULL; 3];
    let mut bi = [GPtr::<u8>::NULL; 3];
    let mut bw = [GPtr::<u8>::NULL; 3];
    for k in 0..3 {
        let gm = rd::<GPtr<u8>>(atoms[k], 0x18);
        let mt = rd::<GPtr<u8>>(gm, 0x5C);
        geo[k] = gm;
        let skin = rp_skin_geometry_get_skin(gm);
        vtx[k] = rd::<GPtr<u8>>(mt, 0x14);
        nrm[k] = rd::<GPtr<u8>>(mt, 0x18);
        uv[k] = rd::<GPtr<u8>>(gm, 0x34);
        bi[k] = rp_skin_get_vertex_bone_indices(skin);
        bw[k] = rp_skin_get_vertex_bone_weights(skin);
    }
    let g0 = geo[0];
    rp_geometry_lock(g0, 0xFFF);
    let m = |p: GPtr<u8>, o: isize| rd::<f32>(p, o);
    let mut i: i32 = 0;
    while i < rd::<i32>(g0, 0x14) {
        let (v0, v1, v2) = (vtx[0], vtx[1], vtx[2]);
        let px = x(r1).fmul_m(m(v1, 0)).fadd(x(r0).fmul_m(m(v0, 0))).fadd(x(r2).fmul_m(m(v2, 0)));
        wr::<f32>(v0, 0, f32_of(px));
        for o in [4isize, 8] {
            let p = x(r1).fmul_m(m(v1, o)).fadd(x(r2).fmul_m(m(v2, o))).fadd(x(r0).fmul_m(m(v0, o)));
            wr::<f32>(v0, o, f32_of(p));
        }
        let (q0, q1, q2) = (nrm[0], nrm[1], nrm[2]);
        let nx = x(r2).fmul_m(m(q2, 0)).fadd(x(r1).fmul_m(m(q1, 0))).fadd(x(r0).fmul_m(m(q0, 0)));
        wr::<f32>(q0, 0, f32_of(nx));
        for o in [4isize, 8] {
            let p = x(r1).fmul_m(m(q1, o)).fadd(x(r2).fmul_m(m(q2, o))).fadd(x(r0).fmul_m(m(q0, o)));
            wr::<f32>(q0, o, f32_of(p));
        }
        rw_v3d_normalize(q0, q0);
        let (t0, t1, t2) = (uv[0], uv[1], uv[2]);
        let u = x(r2).fmul_m(m(t2, 0)).fadd(x(r1).fmul_m(m(t1, 0))).fadd(x(r0).fmul_m(m(t0, 0)));
        wr::<f32>(t0, 0, f32_of(u));
        let v = x(r1).fmul_m(m(t1, 4)).fadd(x(r2).fmul_m(m(t2, 4))).fadd(x(r0).fmul_m(m(t0, 4)));
        wr::<f32>(t0, 4, f32_of(v));
        // weights[8] plus the slot AddWeightToBoneVertex may write past it; bone bytes[8]
        let mut weights = [0f32; 9];
        let mut bones = [0u8; 8];
        let (wp, bp) = (local(&mut weights), local(&mut bones));
        for (k, r) in [r0, r1, r2].into_iter().enumerate() {
            for j in 0..4isize {
                let w = f32_of(x(r).fmul_m(m(bw[k], 4 * j)));
                add_weight_to_bone_vertex_call(wp, bp, w, rd::<u8>(bi[k], j) as u32);
            }
        }
        let w4 = rd::<f32>(wp, 0x10);
        for j in 0..4isize {
            wr::<u8>(bi[0], j, rd::<u8>(bp, j));
            wr::<f32>(bw[0], 4 * j, rd::<f32>(wp, 4 * j));
        }
        if x(w4).fcom(X87::zero()) != sa_math::x87::Cmp::Equal {
            let s = x(rd::<f32>(wp, 12)).fadd_m(rd::<f32>(wp, 8)).fadd_m(rd::<f32>(wp, 4)).fadd_m(rd::<f32>(wp, 0));
            let t = s.fdivr_m(1.0);
            for j in 0..4isize {
                wr::<f32>(bw[0], 4 * j, f32_of(t.fmul_m(rd::<f32>(bw[0], 4 * j))));
            }
        }
        for k in 0..3 {
            vtx[k] = vtx[k].byte_add(12);
            nrm[k] = nrm[k].byte_add(12);
            uv[k] = uv[k].byte_add(8);
            bi[k] = bi[k].byte_add(4);
            bw[k] = bw[k].byte_add(16);
        }
        i += 1;
    }
    rp_geometry_unlock(g0);
    rp_geometry_add_ref(g0);
    g0
}

/// `CClothesBuilder::ConstructGeometryArray(RpGeometry** out, uint32* modelKeys, float normal, float fat,
/// float muscle)` (0x5A55A0, cdecl): for each of the 10 model parts, `out[i] = null` for key 0; else model
/// 0x180 + i gets `bHasComplexHierarchy`, its DFF is requested from `playerImg` (inlined
/// `RequestGeometry`, offset / size locals uninitialised on a failed `FindItem` and kept across parts)
/// and loaded (`LoadAllRequestedModels(true)`), the next part's DFF is requested ahead
/// (`RequestGeometry` + `LoadRequestedModels`), `out[i] = BlendGeometry(clump, "normal", "fat",
/// "ripped", normal, fat, muscle)`, `StoreBoneArray(clump, i)`, and the model is removed.
#[hook(addr = 0x5A55A0, conv = cdecl, name = "CClothesBuilder::ConstructGeometryArray", expect = "83 EC 0C 8B 44 24 10",
       unverified = "clump-1", route = "split", hazards = "heap-writes,uninit-locals", recs = "boot")]
pub fn construct_geometry_array(out: GPtr<u8>, keys: GPtr<u8>, normal: f32, fat: f32, muscle: f32) {
    let (mut size, mut offset) = (0u32, 0u32);
    for i in 0..10i32 {
        let model = CLOTHES_MODEL + i;
        let mi_cell = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * model as usize);
        let key = rd::<u32>(keys, 4 * i as isize);
        let cursor = out.byte_add(4 * i as isize);
        if key == 0 {
            wr::<u32>(cursor, 0, 0);
            continue;
        }
        let mi = mi_cell.read();
        wr::<u8>(mi, 0x13, rd::<u8>(mi, 0x13) | 2);
        let k = append_string_to_key(key, rdata(0x867150)); // ".DFF"
        directory_find_item(GPtr::from_addr(PLAYER_IMG), k, local(&mut offset).cast(), local(&mut size).cast());
        request_file(model, offset, size, g::<i32>(CLOTHES_IMAGE_ID).read(), 0x12);
        load_all_requested_models(1);
        if i + 1 < 10 {
            let next = rd::<u32>(keys, 4 * (i + 1) as isize);
            if next != 0 {
                request_geometry_call(model + 1, next);
                load_requested_models();
            }
        }
        let mi = mi_cell.read();
        let gm = blend_geometry3_call(
            rd::<GPtr<u8>>(mi, 0x1C),
            rdata(0x867160), // "normal"
            rdata(0x85A508), // "fat"
            rdata(0x867168), // "ripped"
            normal,
            fat,
            muscle,
        );
        wr::<GPtr<u8>>(cursor, 0, gm);
        store_bone_array_call(rd::<GPtr<u8>>(mi, 0x1C), i);
        remove_model(model);
    }
}

/// `CClothesBuilder::BuildBoneIndexConversionTable(uint8* table, RpHAnimHierarchy*, int32 i)` (0x5A56E0,
/// cdecl): for the bone IDs of `gBoneIndices[i]` up to the first -1 (at most 64), `table[k] =
/// RpHAnimIDGetIndex(h, id)` (low byte; 0xFF becomes 0).
#[hook(addr = 0x5A56E0, conv = cdecl, name = "CClothesBuilder::BuildBoneIndexConversionTable", expect = "53 8B 5C 24 08",
       unverified = "clump-1", route = "L2", hazards = "none", recs = "boot")]
pub fn build_bone_index_conversion_table(table: GPtr<u8>, h: GPtr<u8>, i: i32) {
    let row = g::<i16>(BONE_INDICES + (i as isize as usize).wrapping_mul(128));
    for k in 0..64i32 {
        let id = row.wrapping_index(k).read() as i32;
        if id == -1 {
            break;
        }
        let r = rp_hanim_id_get_index(h, id) as u8;
        table.byte_add(k as isize).write(if r == 0xFF { 0 } else { r });
    }
}

/// `CClothesBuilder::PlaceTextureOnTopOfTexture(RwTexture* dst, RwTexture* src)` (0x5A57B0, cdecl):
/// both rasters locked read/write (src first); for `src.width * src.height` pixels, a source pixel with
/// a non-zero alpha byte replaces the destination pixel; unlocks src, then dst.
#[hook(addr = 0x5A57B0, conv = cdecl, name = "CClothesBuilder::PlaceTextureOnTopOfTexture", expect = "8B 44 24 08 8B 4C 24 04",
       unverified = "clump-1", route = "L2", hazards = "heap-writes", recs = "boot")]
pub fn place_texture_on_top_of_texture(dst: GPtr<u8>, src: GPtr<u8>) {
    let dr = rd::<GPtr<u8>>(dst, 0);
    let sr = rd::<GPtr<u8>>(src, 0);
    let sp = rw_raster_lock(sr, 0, 3);
    let dp = rw_raster_lock(dr, 0, 3);
    let mut i: i32 = 0;
    while i < rd::<i32>(sr, 0xC).wrapping_mul(rd::<i32>(sr, 0x10)) {
        let px = rd::<u32>(sp, 4 * i as isize);
        if px & 0xFF00_0000 != 0 {
            wr::<u32>(dp, 4 * i as isize, px);
        }
        i += 1;
    }
    rw_raster_unlock(sr);
    rw_raster_unlock(dr);
}

/// `_ftol` of an x87 sum, low byte (the original's 0..255 clamp is a byte compare that never fires).
fn byte_of(v: X87) -> u8 {
    ftol32(v) as u8
}

/// `CClothesBuilder::BlendTextures(RwTexture* dst, RwTexture* src, float r1, float r2, int32)` (0x5A5820,
/// cdecl): under `CTimer::Suspend`/`Resume`, src locked for reading, dst for read/write; for
/// `src.width * src.height` pixels each RGB byte becomes `ftol(s*r2 + d*r1)` (low byte). Alpha: every
/// destination pixel gets the alpha byte of destination pixel 0 (the original reloads the base pointer,
/// not the cursor) — kept.
#[hook(addr = 0x5A5820, conv = cdecl, name = "CClothesBuilder::BlendTextures-Dst-Src", expect = "83 EC 14 8B 4C 24 18",
       unverified = "clump-1", route = "L2", hazards = "heap-writes", recs = "boot")]
pub fn blend_textures2(dst: GPtr<u8>, src: GPtr<u8>, r1: f32, r2: f32, _n: i32) {
    let sr = rd::<GPtr<u8>>(src, 0);
    let dr = rd::<GPtr<u8>>(dst, 0);
    timer_suspend();
    let sp = rw_raster_lock(sr, 0, 2);
    let dp = rw_raster_lock(dr, 0, 3);
    let mut i: i32 = 0;
    while i < rd::<i32>(sr, 0xC).wrapping_mul(rd::<i32>(sr, 0x10)) {
        let (s, d) = (sp.byte_add(4 * i as isize), dp.byte_add(4 * i as isize));
        let alpha = dp.byte_add(3).read();
        let mut c = [0u8; 3];
        for (k, ck) in c.iter_mut().enumerate() {
            let k = k as isize;
            let (sv, dv) = (s.byte_add(k).read() as i32, d.byte_add(k).read() as i32);
            *ck = byte_of(xi(sv).fmul_m(r2).fadd(xi(dv).fmul_m(r1)));
        }
        d.write(c[0]);
        d.byte_add(2).write(c[2]);
        d.byte_add(1).write(c[1]);
        d.byte_add(3).write(alpha);
        i += 1;
    }
    rw_raster_unlock(sr);
    rw_raster_unlock(dr);
    timer_resume();
}

/// `CClothesBuilder::BlendTextures(dst, src1, src2, float r1, float r2, float r3, int32)` (0x5A59C0,
/// cdecl): as the two-texture version for `src1.width * src1.height` pixels with each RGB byte
/// `ftol((s2*r3 + s1*r2) + d*r1)`; alpha unchanged. Locks src1, src2 (read), dst (read/write); unlocks
/// src1, src2, dst.
#[hook(addr = 0x5A59C0, conv = cdecl, name = "CClothesBuilder::BlendTextures-Dst-Src1-Src2", expect = "83 EC 14 8B 4C 24 20",
       unverified = "clump-1", route = "L2", hazards = "heap-writes", recs = "boot")]
pub fn blend_textures3(dst: GPtr<u8>, s1: GPtr<u8>, s2: GPtr<u8>, r1: f32, r2: f32, r3: f32, _n: i32) {
    let r1r = rd::<GPtr<u8>>(s1, 0);
    let dr = rd::<GPtr<u8>>(dst, 0);
    let r2r = rd::<GPtr<u8>>(s2, 0);
    timer_suspend();
    let p1 = rw_raster_lock(r1r, 0, 2);
    let p2 = rw_raster_lock(r2r, 0, 2);
    let dp = rw_raster_lock(dr, 0, 3);
    let mut i: i32 = 0;
    while i < rd::<i32>(r1r, 0xC).wrapping_mul(rd::<i32>(r1r, 0x10)) {
        let o = 4 * i as isize;
        let (a, b, d) = (p1.byte_add(o), p2.byte_add(o), dp.byte_add(o));
        let mut c = [0u8; 3];
        for (k, ck) in c.iter_mut().enumerate() {
            let k = k as isize;
            let (v2, v1, vd) = (b.byte_add(k).read() as i32, a.byte_add(k).read() as i32, d.byte_add(k).read() as i32);
            *ck = byte_of(xi(v2).fmul_m(r3).fadd(xi(v1).fmul_m(r2)).fadd(xi(vd).fmul_m(r1)));
        }
        d.write(c[0]);
        d.byte_add(2).write(c[2]);
        d.byte_add(1).write(c[1]);
        i += 1;
    }
    rw_raster_unlock(r1r);
    rw_raster_unlock(r2r);
    rw_raster_unlock(dr);
    timer_resume();
}

/// `CClothesBuilder::BlendTextures(dst, src1, src2, r1, r2, r3, int32, RwTexture* tattoos)` (0x5A5BC0,
/// cdecl): as the three-texture blend (`c = ftol((s2*r3 + s1*r2) + d*r1)`, low byte), then each channel
/// is laid under the tattoo pixel: `ftol((t - c) * tatAlpha * (1/255 as f32) + c)` (the product an int,
/// `fiadd` of c), low byte; alpha unchanged. Locks src1, src2 (read), dst (read/write), tattoos (read);
/// unlocks in the same order.
#[hook(addr = 0x5A5BC0, conv = cdecl, name = "CClothesBuilder::BlendTextures-Dst-Src1-Src2-Tat", expect = "83 EC 28 8B 44 24 30",
       unverified = "clump-1", route = "L2", hazards = "heap-writes", recs = "boot")]
#[allow(clippy::too_many_arguments)]
pub fn blend_textures_tattoo(dst: GPtr<u8>, s1: GPtr<u8>, s2: GPtr<u8>, r1: f32, r2: f32, r3: f32, _n: i32, tat: GPtr<u8>) {
    let dr = rd::<GPtr<u8>>(dst, 0);
    let r2r = rd::<GPtr<u8>>(s2, 0);
    let r1r = rd::<GPtr<u8>>(s1, 0);
    let tr = rd::<GPtr<u8>>(tat, 0);
    timer_suspend();
    let p1 = rw_raster_lock(r1r, 0, 2);
    let p2 = rw_raster_lock(r2r, 0, 2);
    let dp = rw_raster_lock(dr, 0, 3);
    let tp = rw_raster_lock(tr, 0, 2);
    let k255 = f32::from_bits(0x3B80_8081); // 0x859A3C, 1/255
    let mut i: i32 = 0;
    while i < rd::<i32>(r1r, 0xC).wrapping_mul(rd::<i32>(r1r, 0x10)) {
        let o = 4 * i as isize;
        let (a, b, d, t) = (p1.byte_add(o), p2.byte_add(o), dp.byte_add(o), tp.byte_add(o));
        let ta = t.byte_add(3).read() as i32;
        let mut c = [0u8; 3];
        for (k, ck) in c.iter_mut().enumerate() {
            let k = k as isize;
            let (v2, v1, vd) = (b.byte_add(k).read() as i32, a.byte_add(k).read() as i32, d.byte_add(k).read() as i32);
            *ck = byte_of(xi(v2).fmul_m(r3).fadd(xi(v1).fmul_m(r2)).fadd(xi(vd).fmul_m(r1)));
        }
        let mut out = [0u8; 3];
        for k in 0..3 {
            let tv = t.byte_add(k as isize).read() as i32;
            let cv = c[k] as i32;
            out[k] = byte_of(xi(tv.wrapping_sub(cv).wrapping_mul(ta)).fmul_m(k255).fadd(xi(cv)));
        }
        d.write(out[0]);
        d.byte_add(1).write(out[1]);
        d.byte_add(2).write(out[2]);
        i += 1;
    }
    rw_raster_unlock(r1r);
    rw_raster_unlock(r2r);
    rw_raster_unlock(dr);
    rw_raster_unlock(tr);
    timer_resume();
}

/// `GetTextureFromTxdAndLoadNextTxd(RwTexture* dst, int32 txd, int32 nextKey, int32* nextTxd)` (0x5A5F70,
/// cdecl): txd -1 → only requests the next TXD (`*nextTxd = RequestTexture(nextKey)` +
/// `LoadRequestedModels`, or -1 for key 0) and returns dst. Else `LoadAllRequestedModels(true)`, the same
/// request, then the first texture of TXD `txd` is placed on top of dst (or copied when dst is null),
/// TXD `txd` is removed and dst / the copy returned. A free slot crashes the original (null `TxdDef`).
#[hook(addr = 0x5A5F70, conv = cdecl, name = "CClothesBuilder::GetTextureFromTxdAndLoadNextTxd", expect = "57 8B 7C 24 0C",
       unverified = "clump-1", route = "split", hazards = "heap-writes", recs = "boot")]
pub fn get_texture_from_txd_and_load_next_txd(dst: GPtr<u8>, txd: i32, next_key: u32, next_txd: GPtr<i32>) -> GPtr<u8> {
    let request_next = || {
        if next_key != 0 {
            next_txd.write(request_texture_call(next_key));
            load_requested_models();
        } else {
            next_txd.write(-1);
        }
    };
    if txd == -1 {
        request_next();
        return dst;
    }
    load_all_requested_models(1);
    request_next();
    let tex = get_first_texture(txd_dict(txd));
    let res = if !dst.is_null() {
        place_texture_on_top_of_texture_call(dst, tex);
        dst
    } else {
        copy_texture(tex)
    };
    remove_model(txd.wrapping_add(TXD_INDEX));
    res
}

/// `CClothesBuilder::ConstructTextures(RwTexDictionary*, uint32* texKeys, float normal, float fat, float
/// muscle)` (0x5A6040, cdecl): builds the player's textures from the 18 texture keys:
/// 1. the tattoo layer: TXDs of keys 4..12 placed on top of each other (`GetTextureFromTxdAndLoadNextTxd`
///    chain, the last step requesting the body TXD by the key of name 0x8D0A4C);
/// 2. a negative muscle ratio is folded into fat (`fat += muscle`, `muscle = 0`);
/// 3. torso: copy of texture 0x8D0A5C of the body TXD blended with 0x8D0A60 / 0x8D0A64 (with the tattoo
///    layer if any, which is then destroyed), the torso key 0 texture placed on top (TXD requested with
///    the key of name 0x8D0A50 next), named 0x8D0A7C and added to the dictionary;
/// 4. head: textures 0x8D0A68/6C/70 of the key-2 TXD blended, key 1 (or name 0x8D0A54) placed on top,
///    named 0x8D0A88;
/// 5. legs: texture 0x8D0A74 blended with 0x8D0A78 by `(normal + muscle) / sum` and `fat / sum` (sum =
///    normal + fat + muscle), or a copy of the TXD's first texture without 0x8D0A74; named 0x8D0A80; then
///    the key-3 (or name 0x8D0A58) TXD;
/// 6. keys 13..17: their first textures (named 0x8D0A8C (no null check), 0x8D0A90, 0x8D0A94, 0x8D0A98,
///    0x8D0A9C) and a final copy named 0x8D0AA0 for the TXD the last step requested.
#[hook(addr = 0x5A6040, conv = cdecl, name = "CClothesBuilder::ConstructTextures", expect = "83 EC 10 53 55",
       unverified = "clump-1", route = "split", hazards = "heap-writes", recs = "boot")]
pub fn construct_textures(dict: GPtr<u8>, keys: GPtr<u8>, normal: f32, fat: f32, muscle: f32) {
    let (fa, mut fb, mut fc) = (normal, fat, muscle);
    let h = |i: isize| rd::<u32>(keys, 4 * i);
    // the original's stack slots F+0, F+4, F+8, F+0xC and the reused argument slot F+0x18
    let (mut l4, mut l8, mut lc, mut l18) = (0i32, 0i32, 0i32, 0i32);
    let mut tattoo = GPtr::<u8>::NULL;
    let mut txd = request_texture_call(h(4));
    load_requested_models();
    for n in 4..=12isize {
        if n < 12 {
            tattoo = get_texture_from_txd_call(tattoo, txd, h(n + 1), local(&mut l4).cast());
        } else {
            let k = get_uppercase_key(name_at(0x8D0A4C));
            tattoo = get_texture_from_txd_call(tattoo, txd, k, local(&mut l8).cast());
        }
        txd = l4;
    }
    if x(fc).fcom(X87::zero()) == sa_math::x87::Cmp::Less {
        let s = x(fb).fadd_m(fc);
        fc = 0.0;
        fb = f32_of(s);
    }
    // torso
    load_all_requested_models(1);
    let mut l0 = request_texture_call(h(0));
    load_requested_models();
    let d = txd_dict(l8);
    let t1 = rw_tex_dictionary_find_named_texture(d, name_at(0x8D0A5C));
    let t2 = rw_tex_dictionary_find_named_texture(d, name_at(0x8D0A60));
    let t3 = rw_tex_dictionary_find_named_texture(d, name_at(0x8D0A64));
    let body = copy_texture(t1);
    if !tattoo.is_null() {
        blend_textures_tattoo_call(body, t2, t3, fa, fb, fc, 0x6C, tattoo);
        rw_texture_destroy(tattoo);
    } else {
        blend_textures3_call(body, t2, t3, fa, fb, fc, 0x6C);
    }
    remove_model(l8.wrapping_add(TXD_INDEX));
    let k = get_uppercase_key(name_at(0x8D0A50));
    get_texture_from_txd_call(body, l0, k, local(&mut lc).cast());
    rw_texture_set_name(body, name_at(0x8D0A7C));
    rw_tex_dictionary_add_texture(dict, body);
    // head
    load_all_requested_models(1);
    l0 = request_texture_call(h(2));
    load_requested_models();
    let head_txd = lc;
    let d = txd_dict(head_txd);
    let ha = rw_tex_dictionary_find_named_texture(d, name_at(0x8D0A68));
    let hb = rw_tex_dictionary_find_named_texture(d, name_at(0x8D0A6C));
    let hc = rw_tex_dictionary_find_named_texture(d, name_at(0x8D0A70));
    let head = copy_texture(ha);
    blend_textures3_call(head, hb, hc, fa, fb, fc, 0x6C);
    remove_model(head_txd.wrapping_add(TXD_INDEX));
    let k = if h(1) != 0 { h(1) } else { get_uppercase_key(name_at(0x8D0A54)) };
    get_texture_from_txd_call(head, l0, k, local(&mut l18).cast());
    rw_texture_set_name(head, name_at(0x8D0A88));
    rw_tex_dictionary_add_texture(dict, head);
    // legs
    load_all_requested_models(1);
    l8 = if h(3) != 0 {
        request_texture_call(h(3))
    } else {
        request_texture_call(get_uppercase_key(name_at(0x8D0A58)))
    };
    load_requested_models();
    let d = txd_dict(l18);
    let la = rw_tex_dictionary_find_named_texture(d, name_at(0x8D0A74));
    let lb = rw_tex_dictionary_find_named_texture(d, name_at(0x8D0A78));
    let legs = if !la.is_null() {
        let t = copy_texture(la);
        if !lb.is_null() {
            let sum = x(fa).fadd_m(fb).fadd_m(fc);
            let q2 = f32_of(x(fb).fdiv(sum));
            let q1 = f32_of(x(fa).fadd_m(fc).fdiv(sum));
            blend_textures2_call(t, lb, q1, q2, 0x6C);
        }
        t
    } else {
        copy_texture(get_first_texture(d))
    };
    remove_model(l18.wrapping_add(TXD_INDEX));
    rw_texture_set_name(legs, name_at(0x8D0A80));
    rw_tex_dictionary_add_texture(dict, legs);
    // keys 13..17
    let t = get_texture_from_txd_call(GPtr::NULL, l8, h(13), local(&mut l0).cast());
    rw_texture_set_name(t, name_at(0x8D0A8C));
    rw_tex_dictionary_add_texture(dict, t);
    let steps: [(isize, bool, usize); 4] = [(14, true, 0x8D0A90), (15, false, 0x8D0A94), (16, true, 0x8D0A98), (17, false, 0x8D0A9C)];
    for (key, from_l0, name) in steps {
        let t = if from_l0 {
            get_texture_from_txd_call(GPtr::NULL, l0, h(key), local(&mut l4).cast())
        } else {
            get_texture_from_txd_call(GPtr::NULL, l4, h(key), local(&mut l0).cast())
        };
        if !t.is_null() {
            rw_texture_set_name(t, name_at(name));
            rw_tex_dictionary_add_texture(dict, t);
        }
    }
    if l0 != -1 {
        load_all_requested_models(1);
        let d = txd_dict(l0);
        let t = copy_texture(get_first_texture(d));
        remove_model(l0.wrapping_add(TXD_INDEX));
        if !t.is_null() {
            rw_texture_set_name(t, name_at(0x8D0AA0));
            rw_tex_dictionary_add_texture(dict, t);
        }
    }
}

/// `CClothesBuilder::ConstructGeometryAndSkinArrays(RpHAnimHierarchy*, RpGeometry** outGeo,
/// RwMatrixWeights** outWeights, uint32** outIndices, uint32 n, RpGeometry** geos, RpMaterial** mats)`
/// (0x5A6530, cdecl): creates one geometry (format 0x35) with the summed vertex / triangle counts of the
/// non-null `geos`, copies their positions, normals and texture coordinates, rebuilds their triangles
/// (indices offset by the vertices before them, 16-bit adds) with material `mats[k]`, computes the
/// bounding sphere into morph target 0, unlocks, then allocates `numVertices * 16` bytes of weights and
/// `numVertices * 4` of bone indices (`operator new`) and fills them from each geometry's skin, the bone
/// indices mapped through `BuildBoneIndexConversionTable` (a 64-byte stack table the original never
/// clears, so entries past a model's bone list keep earlier values; zero-initialised here, 256 bytes so
/// any index byte stays inside it).
#[hook(addr = 0x5A6530, conv = cdecl, name = "CClothesBuilder::ConstructGeometryAndSkinArrays", expect = "81 EC 84 00 00 00",
       unverified = "clump-1", route = "L2", hazards = "heap-writes,uninit-locals", recs = "boot")]
pub fn construct_geometry_and_skin_arrays(
    h: GPtr<u8>,
    out_geo: GPtr<u8>,
    out_weights: GPtr<u8>,
    out_indices: GPtr<u8>,
    n: u32,
    geos: GPtr<u8>,
    mats: GPtr<u8>,
) {
    let src = |k: u32| rd::<GPtr<u8>>(geos, 4 * k as isize);
    let (mut nv, mut nt) = (0i32, 0i32);
    for k in 0..n {
        let gm = src(k);
        if !gm.is_null() {
            nv = nv.wrapping_add(rd::<i32>(gm, 0x14));
            nt = nt.wrapping_add(rd::<i32>(gm, 0x10));
        }
    }
    let geo = rp_geometry_create(nv, nt, 0x35);
    wr::<GPtr<u8>>(out_geo, 0, geo);
    let mt = rd::<GPtr<u8>>(geo, 0x5C);
    let mut nrm = rd::<GPtr<u8>>(mt, 0x18);
    let mut vtx = rd::<GPtr<u8>>(mt, 0x14);
    let mut uv = rd::<GPtr<u8>>(geo, 0x34);
    let mut tri = rd::<GPtr<u8>>(geo, 0x2C);
    let mut base: u32 = 0;
    for k in 0..n {
        let gm = src(k);
        if gm.is_null() {
            continue;
        }
        let ntri = rd::<i32>(gm, 0x10);
        let nvert = rd::<i32>(gm, 0x14);
        let gmt = rd::<GPtr<u8>>(gm, 0x5C);
        for i in 0..nvert.max(0) as isize {
            let sv = rd::<GPtr<u8>>(gmt, 0x14).byte_add(12 * i);
            for o in [0isize, 4, 8] {
                wr::<u32>(vtx, o, rd::<u32>(sv, o));
            }
            let sn = rd::<GPtr<u8>>(gmt, 0x18).byte_add(12 * i);
            for o in [0isize, 4, 8] {
                wr::<u32>(nrm, o, rd::<u32>(sn, o));
            }
            let su = rd::<GPtr<u8>>(gm, 0x34).byte_add(8 * i);
            wr::<u32>(uv, 0, rd::<u32>(su, 0));
            wr::<u32>(uv, 4, rd::<u32>(su, 4));
            vtx = vtx.byte_add(12);
            nrm = nrm.byte_add(12);
            uv = uv.byte_add(8);
        }
        let mut st = rd::<GPtr<u8>>(gm, 0x2C);
        for _ in 0..ntri.max(0) {
            let v = |o: isize| (rd::<u16>(st, o).wrapping_add(base as u16)) as u32;
            let (v2, v1, v0) = (v(4), v(2), v(0));
            rp_geometry_triangle_set_vertex_indices(geo, tri, v0, v1, v2);
            rp_geometry_triangle_set_material(geo, tri, rd::<GPtr<u8>>(mats, 4 * k as isize));
            tri = tri.byte_add(8);
            st = st.byte_add(8);
        }
        base = base.wrapping_add(nvert as u32);
    }
    let mut sphere = [0u32; 4];
    let sp = local(&mut sphere);
    rp_morph_target_calc_bounding_sphere(mt, sp);
    for o in [0isize, 4, 8, 12] {
        wr::<u32>(mt, 4 + o, rd::<u32>(sp, o));
    }
    rp_geometry_unlock(geo);
    let weights = operator_new((nv as u32) << 4);
    wr::<GPtr<u8>>(out_weights, 0, weights);
    let indices = operator_new((nv as u32).wrapping_mul(4));
    wr::<GPtr<u8>>(out_indices, 0, indices);
    let mut icur = indices;
    let mut wcur = rd::<GPtr<u8>>(out_weights, 0);
    let mut table = [0u8; 256];
    let tp = local(&mut table);
    for k in 0..n {
        let gm = src(k);
        if gm.is_null() {
            continue;
        }
        let skin = rp_skin_geometry_get_skin(gm);
        let nvert = rd::<i32>(gm, 0x14);
        build_bone_index_conversion_table_call(tp, h, k as i32);
        for i in 0..nvert.max(0) as isize {
            let bi = rp_skin_get_vertex_bone_indices(skin).byte_add(4 * i);
            for j in 0..4isize {
                let b = bi.byte_add(j).read() as isize;
                icur.byte_add(j).write(tp.byte_add(b).read());
            }
            let bw = rp_skin_get_vertex_bone_weights(skin).byte_add(16 * i);
            for o in [0isize, 4, 8, 12] {
                wr::<u32>(wcur, o, rd::<u32>(bw, o));
            }
            wcur = wcur.byte_add(16);
            icur = icur.byte_add(4);
        }
    }
}

/// `CClothesBuilder::CreateSkinnedClump(RpClump* bones, RwTexDictionary*, CPedClothesDesc& new, const
/// CPedClothesDesc* old, bool cutscene)` (0x5A69D0, cdecl): the player's clump.
/// 1. `playerImg.Init(0x226, entries)` + `ReadDirFile("MODELS\PLAYER.IMG")`; empty torso / head / hands /
///    legs / feet model keys get their default names (table 0x8D0A38);
/// 2. 0xBC1288 = old. Without old all three change flags are set; else fat or muscle stat differing
///    (x87 equality, NaN = different) sets ratios and geometry, a differing model key geometry, a
///    differing texture key texture; nothing changed → null;
/// 3. a copy of new goes through `PreprocessClothesDesc`, the ratios through the register helper 0x5A42B0;
/// 4. (texture or ratios changed) and not a cutscene build: every texture of the dictionary is destroyed
///    (callback 0x5A44A0) and `ConstructTextures` rebuilds them;
/// 5. a material (white) for each body-part texture of table 0x8D0A7C found in the dictionary,
///    `ConstructGeometryArray`, `ConstructGeometryAndSkinArrays` against the bones clump's first atomic,
///    a new skin (the bones' skin-to-bone matrices), a copied HAnim hierarchy on a new frame, a generic
///    skinned atomic under a root frame in a new clump;
/// 6. the merged geometry is destroyed (the atomic keeps it) and then flagged `|= 2` at +8 (after the
///    destroy, as the original), the dictionary made current, the part geometries and materials destroyed.
#[hook(addr = 0x5A69D0, conv = cdecl, name = "CClothesBuilder::CreateSkinnedClump", expect = "81 EC D4 00 00 00",
       unverified = "clump-1", route = "split", hazards = "heap-writes", recs = "boot")]
pub fn create_skinned_clump(bones: GPtr<u8>, dict: GPtr<u8>, new_desc: GPtr<u8>, old_desc: GPtr<u8>, cutscene: u8) -> GPtr<u8> {
    let mut desc = [0u32; 30];
    let dp = local(&mut desc);
    desc_ctor(dp);
    directory_init(GPtr::from_addr(PLAYER_IMG), 0x226, GPtr::from_addr(PLAYER_IMG_ENTRIES));
    directory_read_dir_file(GPtr::from_addr(PLAYER_IMG), rdata(0x86713C)); // "MODELS\PLAYER.IMG"
    for part in 0..5i32 {
        if rd::<u32>(new_desc, 4 * part as isize) == 0 {
            set_model_name(new_desc, g::<u32>(0x8D0A38 + 4 * part as usize).read(), part);
        }
    }
    g::<GPtr<u8>>(LAST_OLD_DESC).write(old_desc);
    let (ratios, geometry, texture) = (g::<u8>(RATIOS_CHANGED), g::<u8>(GEOMETRY_CHANGED), g::<u8>(TEXTURE_CHANGED));
    if old_desc.is_null() {
        ratios.write(1);
        geometry.write(1);
        texture.write(1);
    } else {
        geometry.write(0);
        texture.write(0);
        let eq = |o: isize| x(rd::<f32>(old_desc, o)).fcom(x(rd::<f32>(new_desc, o))) == sa_math::x87::Cmp::Equal;
        let mut geo_changed = false;
        if eq(0x70) && eq(0x74) {
            ratios.write(0);
        } else {
            geo_changed = true;
            ratios.write(1);
            geometry.write(1);
        }
        if (0..10isize).any(|i| rd::<u32>(new_desc, 4 * i) != rd::<u32>(old_desc, 4 * i)) {
            geo_changed = true;
            geometry.write(1);
        }
        if (0..18isize).any(|i| rd::<u32>(new_desc, 0x28 + 4 * i) != rd::<u32>(old_desc, 0x28 + 4 * i)) {
            texture.write(1);
        }
        if ratios.read() == 0 && !geo_changed && texture.read() == 0 {
            return GPtr::NULL;
        }
    }
    for i in 0..30isize {
        wr::<u32>(dp, 4 * i, rd::<u32>(new_desc, 4 * i));
    }
    preprocess_clothes_desc_call(dp, cutscene);
    let (mut muscle, mut fat, mut normal) = (0f32, 0f32, 0f32);
    let (mp, fp, np) = (local(&mut muscle).cast::<f32>(), local(&mut fat).cast::<f32>(), local(&mut normal).cast::<f32>());
    blend_ratios_call(dp, fp, mp, np);
    let (rm, rf) = (mp.read(), fp.read());
    if (texture.read() != 0 || ratios.read() != 0) && cutscene == 0 {
        rw_tex_dictionary_for_all_textures(dict, 0x5A44A0, 0);
        construct_textures_call(dict, dp.byte_add(0x28), np.read(), rf, rm);
    }
    let mut ms = [0u32; 10];
    let msp = local(&mut ms);
    for i in 0..10isize {
        let tex = rw_tex_dictionary_find_named_texture(dict, name_at(0x8D0A7C + 4 * i as usize));
        if !tex.is_null() {
            let m = rp_material_create();
            wr::<GPtr<u8>>(msp, 4 * i, m);
            wr::<u32>(m, 4, 0xFFFF_FFFF);
            rp_material_set_texture(m, tex);
        } else {
            wr::<u32>(msp, 4 * i, 0);
        }
    }
    let mut gs = [0u32; 10];
    let gsp = local(&mut gs);
    construct_geometry_array_call(gsp, dp, np.read(), rf, rm);
    let atomic = get_first_atomic(bones);
    let skin = rp_skin_geometry_get_skin(rd::<GPtr<u8>>(atomic, 0x18));
    let hier = rp_skin_atomic_get_hanim_hierarchy(atomic);
    let (mut geo, mut weights, mut indices) = (0u32, 0u32, 0u32);
    let (geo_p, w_p, i_p) = (local(&mut geo), local(&mut weights), local(&mut indices));
    construct_geometry_and_skin_arrays_call(hier, geo_p, w_p, i_p, 10, gsp, msp);
    let gm = rd::<GPtr<u8>>(geo_p, 0);
    let nverts = rd::<u32>(gm, 0x14);
    let nbones = rp_skin_get_num_bones(skin);
    let inv = rp_skin_get_skin_to_bone_matrices(skin);
    let (wts, idx) = (rd::<GPtr<u8>>(w_p, 0), rd::<GPtr<u8>>(i_p, 0));
    let new_skin = rp_skin_create(nverts, nbones, wts, idx, inv);
    rp_skin_geometry_set_skin(gm, new_skin);
    operator_delete(wts);
    operator_delete(idx);
    let kfs = rd::<i32>(rd::<GPtr<u8>>(hier, 0x20), 0x20);
    let new_hier = rp_hanim_hierarchy_create_from_hierarchy(hier, rd::<u32>(hier, 0), kfs);
    let frame = rw_frame_create();
    rp_hanim_frame_set_hierarchy(frame, new_hier);
    let a = rp_atomic_create();
    rp_atomic_set_geometry(a, gm, 0);
    rp_skin_atomic_set_hanim_hierarchy(a, new_hier);
    rp_atomic_set_frame(a, frame);
    rp_skin_atomic_set_type(a, 1);
    let root = rw_frame_create();
    rw_frame_add_child(root, frame);
    let clump = rp_clump_create();
    wr::<GPtr<u8>>(clump, 4, root);
    rp_clump_add_atomic(clump, a);
    rp_geometry_destroy(gm);
    wr::<u32>(gm, 8, rd::<u32>(gm, 8) | 2);
    rw_tex_dictionary_set_current(dict);
    for i in 0..10isize {
        let g = rd::<GPtr<u8>>(gsp, 4 * i);
        if !g.is_null() {
            rp_geometry_destroy(g);
        }
        let m = rd::<GPtr<u8>>(msp, 4 * i);
        if !m.is_null() {
            rp_material_destroy(m);
        }
    }
    clump
}

pub const HOOKS: &[HookReg] = &[
    HOOK_REQUEST_GEOMETRY,
    HOOK_REQUEST_TEXTURE,
    HOOK_BLEND_RATIOS,
    HOOK_DESTROY_TEXTURE_CB,
    HOOK_PREPROCESS_CLOTHES_DESC,
    HOOK_ADD_WEIGHT_TO_BONE_VERTEX,
    HOOK_STORE_BONE_ARRAY,
    HOOK_BLEND_GEOMETRY3,
    HOOK_CONSTRUCT_GEOMETRY_ARRAY,
    HOOK_BUILD_BONE_INDEX_CONVERSION_TABLE,
    HOOK_PLACE_TEXTURE_ON_TOP_OF_TEXTURE,
    HOOK_BLEND_TEXTURES2,
    HOOK_BLEND_TEXTURES3,
    HOOK_BLEND_TEXTURES_TATTOO,
    HOOK_GET_TEXTURE_FROM_TXD_AND_LOAD_NEXT_TXD,
    HOOK_CONSTRUCT_TEXTURES,
    HOOK_CONSTRUCT_GEOMETRY_AND_SKIN_ARRAYS,
    HOOK_CREATE_SKINNED_CLUMP,
];
