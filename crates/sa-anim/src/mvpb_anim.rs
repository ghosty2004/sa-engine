//! WRITER-MVP-B (helperB) ANIM ports: skinned/unskinned clump initialisation helpers, the frame-search
//! callbacks, `ConvertBoneTag2BoneName`, `RpAnimBlendNodeUpdateKeyFrames` and part of `CCutsceneMgr`. Ported from
//! the 1.0 US bytes (the exe wins over gta-reversed).
//!
//! Register arguments: 0x4D6510, 0x4D66A0, 0x4D6450 and 0x4D1570 take a pointer in EAX (whole-program-optimised
//! static functions). They are hooked with `entry_eax` and read it through `sa_core::abi::entry_eax()` as their
//! first action; their stack arguments follow cdecl.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::*;

/// `RpAnimBlendClumpDataOffset` (the clump plugin offset of `CAnimBlendClumpData*`)
const CLUMP_DATA_OFFSET: usize = 0xB5F878;
/// the frame `ForAllFramesF` callbacks store the match in
const FOUND_FRAME: usize = 0xB5F87C;

orig! {
    /// `RpAnimBlendAllocateData(RpClump*)` (0x4D5F50, cdecl)
    fn rp_anim_blend_allocate_data(c: GPtr<u8>) = cdecl 0x4D5F50 => unported;
    /// `GetFirstAtomic(RpClump*)` (0x734820, cdecl)
    fn get_first_atomic(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734820 => unported;
    /// `RpSkinGeometryGetSkin(RpGeometry*)` (0x7C7550, cdecl)
    fn rp_skin_geometry_get_skin(g: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7550 => unported;
    /// `RpSkinGetNumBones(RpSkin*)` (0x7C77E0, cdecl)
    fn rp_skin_get_num_bones(s: GPtr<u8>) -> i32 = cdecl 0x7C77E0 => unported;
    /// `CAnimBlendClumpData::SetNumberOfBones(int32)` (0x4CF140, `ret 4`)
    fn clump_data_set_number_of_bones(d: GPtr<u8>, n: i32) = thiscall 0x4CF140 => unported;
    /// `CAnimBlendClumpData::ForAllFrames(cb, data)` (0x4CF190, `ret 8`)
    fn clump_data_for_all_frames(d: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CF190 => unported;
    /// `GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40, cdecl)
    fn get_anim_hierarchy_from_skin_clump(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `SkinGetBonePositionsToTable(RpClump*, RwV3d*)` (0x735360, cdecl)
    fn skin_get_bone_positions_to_table(c: GPtr<u8>, t: GPtr<u8>) = cdecl 0x735360 => unported;
    /// `RwFrameForAllChildren(RwFrame*, cb, void*)` (0x7F0DC0, cdecl)
    fn rw_frame_for_all_children(f: GPtr<u8>, cb: u32, data: GPtr<u32>) -> GPtr<u8> = cdecl 0x7F0DC0 => unported;
    /// `ConvertPedNode2BoneTag(int32)` (0x4D58A0, cdecl)
    fn convert_ped_node_2_bone_tag(n: i32) -> i32 = cdecl 0x4D58A0 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32)` (0x7C51A0, cdecl)
    fn rp_hanim_id_get_index(h: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `ConvertBoneTag2BoneName(int32)` (0x4D56F0, cdecl)
    fn convert_bone_tag_2_bone_name_o(t: i32) -> u32 = cdecl 0x4D56F0 => crate::mvpb_anim::convert_bone_tag_2_bone_name;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30, cdecl)
    fn get_uppercase_key(s: u32) -> u32 = cdecl 0x53CF30 => unported;
    /// `GetFrameNodeName(RwFrame*)` (0x72FB30, cdecl)
    fn get_frame_node_name(f: GPtr<u8>) -> u32 = cdecl 0x72FB30 => unported;
    /// `CAnimBlendNode::FindKeyFrame(float)` (0x4D0240, `ret 4`)
    fn anim_blend_node_find_key_frame(n: GPtr<u8>, t: f32) -> u8 = thiscall 0x4D0240 => unported;
    /// `CCamera::GetCutSceneFinishTime` (0x50AD90)
    fn camera_get_cut_scene_finish_time(c: GPtr<u8>) -> u32 = thiscall 0x50AD90 => unported;
    /// `CCamera::FinishCutscene` (0x514950)
    fn camera_finish_cutscene(c: GPtr<u8>) = thiscall 0x514950 => unported;
    /// `FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPlayerInfo::MakePlayerSafe(bool, float)` (0x56E870, `ret 8`)
    fn player_info_make_player_safe(p: GPtr<u8>, safe: u32, r: f32) = thiscall 0x56E870 => unported;
    /// `CModelInfo::GetModelInfo(const char*, int32*)` (0x4C5940, cdecl)
    fn model_info_get_model_info(name: GPtr<u8>, idx: GPtr<i32>) -> GPtr<u8> = cdecl 0x4C5940 => unported;
    /// `CWorld::FindObjectsOfTypeInRange(...)` (0x564C70, cdecl, 12 arguments)
    fn world_find_objects_of_type_in_range(mi: i32, pos: GPtr<f32>, r: f32, b2d: u32, n: GPtr<i16>, max: i16, out: GPtr<u32>, b: u32, v: u32, p: u32, o: u32, d: u32) = cdecl 0x564C70 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CStreaming::ImGonnaUseStreamingMemory` (0x407BE0)
    fn im_gonna_use_streaming_memory() = cdecl 0x407BE0 => unported;
    /// `CStreaming::IHaveUsedStreamingMemory` (0x407BF0)
    fn i_have_used_streaming_memory() = cdecl 0x407BF0 => unported;
    /// `CBaseModelInfo::SetColModel(CColModel*, bool)` (0x4C4BC0, `ret 8`)
    fn base_model_info_set_col_model(mi: GPtr<u8>, c: GPtr<u8>, owns: u32) = thiscall 0x4C4BC0 => unported;
    /// `CCutsceneMgr::UpdateCutsceneObjectBoundingBox(RpClump*, int32)` (0x5B01E0, cdecl)
    fn update_cutscene_object_bounding_box(c: GPtr<u8>, id: i32) = cdecl 0x5B01E0 => unported;
    /// `CObject::operator new(uint32)` (0x5A1EE0, cdecl)
    fn object_new(size: u32) -> GPtr<u8> = cdecl 0x5A1EE0 => unported;
    /// `CCutsceneObject::CCutsceneObject` (0x5B19D0)
    fn cutscene_object_ctor_o(o: GPtr<u8>) -> GPtr<u8> = thiscall 0x5B19D0 => crate::mvpb_anim::cutscene_object_ctor;
    /// `CObject::CObject` (0x5A1D10)
    fn object_ctor(o: GPtr<u8>) -> GPtr<u8> = thiscall 0x5A1D10 => unported;
    /// `CObject::SetIsStatic(bool)` (0x5A0760, `ret 4`)
    fn object_set_is_static(o: GPtr<u8>, b: u32) = thiscall 0x5A0760 => unported;
    /// `CEntity::SetModelIndex(uint32)` (vtable +0x14)
    fn v_set_model_index(e: GPtr<u8>, id: i32) = thiscall vslot 0x14 => unported;
}

fn clump_data(clump: GPtr<u8>) -> GPtr<u8> {
    let off = g::<i32>(CLUMP_DATA_OFFSET).read();
    rd::<GPtr<u8>>(clump, off as isize)
}

/// `RpAnimBlendClumpInitSkinned(RpClump*)` (0x4D6510, clump in EAX): allocates the blend data, sizes it for the
/// skin's bone count, reads the bone positions (`SkinGetBonePositionsToTable` into a zeroed 64-entry table) and fills
/// each frame entry (0x18 bytes): +0x10 = the hierarchy's interpolated key frame `i` (`+0x20` base + 0x4C +
/// i * stride `+0x24`), +0x14 = node id (`pNodeInfo[i]`), +4..+0xC = bone position; then `ForAllFrames(0x4D6500)`
/// and flag 8 on the first frame.
#[hook(addr = 0x4D6510, conv = cdecl, name = "RpAnimBlendClumpInitSkinned", expect = "55 8B EC 83 E4 F8",
       entry_eax, unverified = "mvpb-anim-1", route = "L2", hazards = "heap-writes", note = "clump pointer in EAX")]
pub fn rp_anim_blend_clump_init_skinned() {
    let clump: GPtr<u8> = GPtr::from_addr(sa_core::abi::entry_eax() as usize);
    rp_anim_blend_allocate_data(clump);
    let data = clump_data(clump);
    let atomic = get_first_atomic(clump);
    let skin = rp_skin_geometry_get_skin(rd::<GPtr<u8>>(atomic, 0x18));
    let n = rp_skin_get_num_bones(skin);
    clump_data_set_number_of_bones(data, n);
    let hier = get_anim_hierarchy_from_skin_clump(clump);
    let mut table = [0u32; 0xC0];
    let tp: GPtr<u8> = GPtr::from_raw(table.as_mut_ptr().cast());
    skin_get_bone_positions_to_table(clump, tp);
    let frames = rd::<GPtr<u8>>(data, 0x10);
    for i in 0..n.max(0) {
        let f = frames.byte_add(0x18 * i as isize);
        let id = rd::<u32>(rd::<GPtr<u8>>(hier, 0x10), 0x10 * i as isize);
        let kf = rd::<u32>(hier, 0x20);
        let stride = rd::<u32>(GPtr::from_addr(kf as usize), 0x24);
        wr::<u32>(f, 0x10, stride.wrapping_mul(i as u32).wrapping_add(kf).wrapping_add(0x4C));
        wr::<u32>(f, 0x14, id);
        for k in 0..3isize {
            wr::<u32>(f, 4 + 4 * k, rd::<u32>(tp, 0xC * i as isize + 4 * k));
        }
    }
    clump_data_for_all_frames(data, 0x4D6500, 0);
    let f0 = rd::<GPtr<u8>>(data, 0x10);
    wr::<u8>(f0, 0, rd::<u8>(f0, 0) | 8);
}

/// `FUN_004d66a0(RpClump*)` (0x4D66A0, clump in EAX; the unskinned `RpAnimBlendClumpInit` path): allocates the
/// blend data, counts the frames (`RwFrameForAllChildren(0x4D6610)`), sets the bone count, fills the frame entries
/// (`RwFrameForAllChildren(0x4D6670)` with a cursor starting at the frame array), `ForAllFrames(0x4D6640)`, flag 8 on
/// the first frame.
#[hook(addr = 0x4D66A0, conv = cdecl, name = "FUN_004d66a0", expect = "83 EC 08 56 57 8B F8",
       entry_eax, unverified = "mvpb-anim-1", route = "L2", hazards = "heap-writes", note = "clump pointer in EAX")]
pub fn rp_anim_blend_clump_init_unskinned() {
    let clump: GPtr<u8> = GPtr::from_addr(sa_core::abi::entry_eax() as usize);
    rp_anim_blend_allocate_data(clump);
    let data = clump_data(clump);
    let frame = rd::<GPtr<u8>>(clump, 4);
    let mut cnt = 0u32;
    rw_frame_for_all_children(frame, 0x4D6610, GPtr::from_raw(&mut cnt));
    clump_data_set_number_of_bones(data, cnt as i32);
    let mut cursor = rd::<u32>(data, 0x10);
    rw_frame_for_all_children(frame, 0x4D6670, GPtr::from_raw(&mut cursor));
    clump_data_for_all_frames(data, 0x4D6640, 0);
    let f0 = rd::<GPtr<u8>>(data, 0x10);
    wr::<u8>(f0, 0, rd::<u8>(f0, 0) | 8);
}

/// `FUN_004d6450(RpClump*, AnimBlendFrameData** out)` (0x4D6450, clump in EAX; `RpAnimBlendClumpFillFrameArray`'s
/// skinned path): for ped nodes 1..18, `out[i]` = the frame entry of the bone `ConvertPedNode2BoneTag(i)`
/// (`RpHAnimIDGetIndex` in the skin hierarchy; 0x18-byte entries).
#[hook(addr = 0x4D6450, conv = cdecl, name = "FUN_004d6450", expect = "8B 0D 78 F8 B5 00",
       entry_eax, unverified = "mvpb-anim-1", route = "L2", hazards = "none", note = "clump pointer in EAX")]
pub fn clump_fill_frame_array_skinned(out: GPtr<u32>) {
    let clump: GPtr<u8> = GPtr::from_addr(sa_core::abi::entry_eax() as usize);
    let data = clump_data(clump);
    let h = get_anim_hierarchy_from_skin_clump(clump);
    for i in 1..0x13 {
        let tag = convert_ped_node_2_bone_tag(i);
        let idx = rp_hanim_id_get_index(h, tag);
        let v = rd::<u32>(data, 0x10).wrapping_add((idx.wrapping_mul(0x18)) as u32);
        out.wrapping_index(i).write(v);
    }
}

/// `RpAnimBlendNodeUpdateKeyFrames(AnimBlendFrameData* frames, int32 n)` (0x4D1570, node list in EAX, cdecl
/// otherwise): for every association in the null-terminated list starting at EAX+4, every frame (0x18 stride) whose
/// node (`assoc nodes + off`) has a sequence (+0x10) calls `CAnimBlendNode::FindKeyFrame(hierarchy time +0x20)`;
/// frames flagged 8 are skipped while `[0xB4EA0C]+0xC` is set.
#[hook(addr = 0x4D1570, conv = cdecl, name = "RpAnimBlendNodeUpdateKeyFrames", expect = "57 8B F8 8B 47 04",
       entry_eax, unverified = "mvpb-anim-1", route = "L2", hazards = "heap-writes", note = "association list in EAX")]
pub fn rp_anim_blend_node_update_key_frames(frames: GPtr<u8>, n: i32) {
    let mut list: GPtr<u8> = GPtr::from_addr(sa_core::abi::entry_eax() as usize).byte_add(4);
    let mut a = rd::<GPtr<u8>>(list, 0);
    while !a.is_null() {
        let hier = rd::<GPtr<u8>>(a, 0x14);
        for k in 0..n.max(0) {
            let off = 0x18 * k as isize;
            if rd::<u8>(frames, off) & 8 != 0 && rd::<u32>(g::<GPtr<u8>>(0xB4EA0C).read(), 0xC) != 0 {
                continue;
            }
            let node = rd::<GPtr<u8>>(list, 0).byte_add(off);
            if rd::<u32>(node, 0x10) != 0 {
                anim_blend_node_find_key_frame(node, x(rd::<f32>(hier, 0x20)).to_f32());
            }
        }
        list = list.byte_add(4);
        a = rd::<GPtr<u8>>(list, 0);
    }
}

/// `ForAllFramesF` (0x4D6310, cdecl callback `(AnimBlendFrameData*, void* key)`): stores the frame in 0xB5F87C when
/// the upper-case key of its bone name (`ConvertBoneTag2BoneName(+0x14)`, skipped when null) equals the key.
#[hook(addr = 0x4D6310, conv = cdecl, name = "ForAllFramesF", expect = "56 8B 74 24 08",
       unverified = "mvpb-anim-1", route = "L2", hazards = "none")]
pub fn for_all_frames_find_bone_name(fd: GPtr<u8>, key: u32) {
    let name = convert_bone_tag_2_bone_name_o(rd::<i32>(fd, 0x14));
    if name != 0 && get_uppercase_key(name) == key {
        g::<GPtr<u8>>(FOUND_FRAME).write(fd);
    }
}

/// `ForAllFramesF` (0x4D6340, cdecl callback `(AnimBlendFrameData*, void* key)`): stores the frame in 0xB5F87C when
/// the upper-case key of its RW frame's node name (`GetFrameNodeName(+0x10)`) equals the key.
#[hook(addr = 0x4D6340, conv = cdecl, name = "ForAllFramesF", expect = "56 8B 74 24 08",
       unverified = "mvpb-anim-1", route = "L2", hazards = "none")]
pub fn for_all_frames_find_node_name(fd: GPtr<u8>, key: u32) {
    let name = get_frame_node_name(rd::<GPtr<u8>>(fd, 0x10));
    if get_uppercase_key(name) == key {
        g::<GPtr<u8>>(FOUND_FRAME).write(fd);
    }
}

/// `ConvertBoneTag2BoneName(int32 tag)` (0x4D56F0, cdecl): the bone name string for a ped bone tag (jump tables
/// 0x4D5864 / 0x4D57EC for 0..0x36, then 0xC9, 0x12D, 0x12E); 0 for anything else.
#[hook(addr = 0x4D56F0, conv = cdecl, name = "ConvertBoneTag2BoneName", expect = "8B 44 24 04 3D C9 00 00 00",
       unverified = "mvpb-anim-1", route = "L2", hazards = "none")]
pub fn convert_bone_tag_2_bone_name(tag: i32) -> u32 {
    match tag {
        0 => 0x85EE74,
        1 => 0x85EE7C,
        2 => 0x85EE84,
        3 => 0x85EE8C,
        4 => 0x85EE94,
        5 => 0x85EE9C,
        6 => 0x85EE6C,
        7 => 0x85EE64,
        8 => 0x85EE60,
        21 => 0x85EE4C,
        22 => 0x85EE40,
        23 => 0x85EE34,
        24 => 0x85EE2C,
        25 => 0x85EE20,
        26 => 0x85EE14,
        31 => 0x85EE00,
        32 => 0x85EDF4,
        33 => 0x85EDE8,
        34 => 0x85EDE0,
        35 => 0x85EDD4,
        36 => 0x85EDC8,
        41 => 0x85EDC0,
        42 => 0x85EDB8,
        43 => 0x85EDB0,
        44 => 0x85EDA8,
        51 => 0x85EDA0,
        52 => 0x85ED98,
        53 => 0x85ED90,
        54 => 0x85ED88,
        0xC9 => 0x85ED80,
        0x12D => 0x85ED68,
        0x12E => 0x85ED74,
        _ => 0,
    }
}

/// `CCutsceneMgr::FinishCutscene` (0x5B04D0, static): when the cutscene was running (0xBC4006) the time offset
/// (0xBC3F08) becomes the camera's cutscene finish time (unsigned, x87) * 0.001 and the camera finishes it; the player
/// ped gets flag 0x80 in +0x1C and `MakePlayerSafe(false, 10000)`.
#[hook(addr = 0x5B04D0, conv = cdecl, name = "CCutsceneMgr::FinishCutscene", expect = "51 A0 06 40 BC 00",
       unverified = "mvpb-anim-1", route = "L2", hazards = "heap-writes")]
pub fn cutscene_mgr_finish_cutscene() {
    let cam: GPtr<u8> = GPtr::from_addr(0xB6F028);
    if g::<u8>(0xBC4006).read() != 0 {
        let t = camera_get_cut_scene_finish_time(cam);
        let mut v = xi(t as i32);
        if (t as i32) < 0 {
            v = v.fadd_m(4294967296.0);
        }
        g::<f32>(0xBC3F08).write(v.fmul_m(f32::from_bits(0x3A83_126F)).to_f32());
        camera_finish_cutscene(cam);
    }
    let p = find_player_ped(-1);
    let id = g::<u8>(0xB7CD74).read() as usize;
    wr::<u32>(p, 0x1C, rd::<u32>(p, 0x1C) | 0x80);
    player_info_make_player_safe(GPtr::from_addr(0xB7CD98 + 0x190 * id), 0, 10000.0);
}

/// `CCutsceneMgr::HideRequestedObjects` (0x5AFAD0, static): for each of the `ms_numObjectsToHide` (0xBC3FF8, reset
/// to 0 first) requests (0xBC20D0, 0x2C stride: position, model name at +0xC) whose model exists, finds up to 32
/// objects of that model within 1.5 (2D) and hides each visible one (+0x1C bit 0x80): it is appended to the hidden
/// list (0xBC2968, reference registered) and loses the flag.
#[hook(addr = 0x5AFAD0, conv = cdecl, name = "CCutsceneMgr::HideRequestedObjects", expect = "A1 F8 3F BC 00",
       unverified = "mvpb-anim-1", route = "L2", hazards = "heap-writes")]
pub fn cutscene_mgr_hide_requested_objects() {
    let num = g::<i32>(0xBC3FF8);
    let n = num.read();
    num.write(0);
    for k in 0..n.max(0) {
        let e: GPtr<u8> = GPtr::from_addr(0xBC20D0 + 0x2C * k as usize);
        let mut mi = 0i32;
        if model_info_get_model_info(e.byte_add(0xC), GPtr::from_raw(&mut mi)).is_null() {
            continue;
        }
        let mut pos = [
            f32::from_bits(rd::<u32>(e, 0)),
            x(rd::<f32>(e, 4)).to_f32(),
            x(rd::<f32>(e, 8)).to_f32(),
        ];
        let mut cnt = 0i16;
        let mut objs = [0u32; 32];
        world_find_objects_of_type_in_range(
            mi,
            GPtr::from_raw(pos.as_mut_ptr()),
            1.5,
            1,
            GPtr::from_raw(&mut cnt),
            32,
            GPtr::from_raw(objs.as_mut_ptr()),
            1,
            0,
            0,
            1,
            1,
        );
        let mut i = 0i32;
        while i < cnt as i32 {
            let o: GPtr<u8> = GPtr::from_addr(objs[i as usize] as usize);
            if (rd::<u32>(o, 0x1C) >> 7) & 1 != 0 {
                let slot: GPtr<u8> = GPtr::from_addr(0xBC2968 + 4 * num.read() as usize);
                wr(slot, 0, o);
                register_reference(o, slot);
                let h = rd::<GPtr<u8>>(GPtr::from_addr(0xBC2968 + 4 * num.read() as usize), 0);
                wr::<u32>(h, 0x1C, rd::<u32>(h, 0x1C) & 0xFFFF_FF7F);
                num.write(num.read().wrapping_add(1));
            }
            i += 1;
        }
    }
}

/// `CCutsceneMgr::CreateCutsceneObject(int32 modelId)` (0x5B02A0, static): for the special cutscene models
/// 300..319 the model gets its cutscene col model (0x968A30 + (id-300)*0x30, not owned; its +0x24 cleared) and its
/// bounding box updated; then a new `CCutsceneObject` (null not checked before `SetModelIndex`, as the original)
/// is appended to `ms_pCutsceneObjects` (0xBC3F18, count 0xBC3FE4) and returned, inside the streaming-memory
/// bracket.
#[hook(addr = 0x5B02A0, conv = cdecl, name = "CCutsceneMgr::CreateCutsceneObject", expect = "64 A1 00 00 00 00",
       unverified = "mvpb-anim-1", route = "L2", hazards = "heap-writes")]
pub fn cutscene_mgr_create_cutscene_object(id: i32) -> GPtr<u8> {
    im_gonna_use_streaming_memory();
    let mis: GPtr<u8> = GPtr::from_addr(0xA9B0C8);
    if (300..=319).contains(&id) {
        let mi = rd::<GPtr<u8>>(mis, 4 * id as isize);
        let off = ((id * 3 - 0x384) << 4) as usize;
        base_model_info_set_col_model(mi, GPtr::from_addr(0x968A30 + off), 0);
        let c = rd::<GPtr<u8>>(rd::<GPtr<u8>>(mis, 4 * id as isize), 0x1C);
        wr::<u32>(GPtr::from_addr(0x968A54 + off), 0, 0);
        update_cutscene_object_bounding_box(c, id);
    }
    let o = object_new(0x19C);
    let o = if o.is_null() { GPtr::NULL } else { cutscene_object_ctor_o(o) };
    v_set_model_index(o, id);
    let n = g::<i32>(0xBC3FE4);
    let k = n.read();
    wr(GPtr::from_addr(0xBC3F18 + 4 * k as usize), 0, o);
    n.write(k.wrapping_add(1));
    i_have_used_streaming_memory();
    o
}

/// `CCutsceneObject::CCutsceneObject` (0x5B19D0): `CObject` base, vtable 0x868A60, +0x184..+0x198 = 0, entity type
/// bits of +0x36 kept with status 2 (`| 0x10`), +0x13C = 4 (object type cutscene), +0x17C = +0x180 = 0, +0x1C flags:
/// clear bit 0, set 0x10000400; `SetIsStatic(false)`; clear 0x08000000; +0x12C = 0.5. Returns this.
#[hook(addr = 0x5B19D0, conv = thiscall, name = "CCutsceneObject::CCutsceneObject", expect = "6A FF 68 78 D4 83 00",
       unverified = "mvpb-anim-1", route = "L2", hazards = "heap-writes")]
pub fn cutscene_object_ctor(this: GPtr<u8>) -> GPtr<u8> {
    object_ctor(this);
    wr::<u32>(this, 0, 0x868A60);
    for k in 0..6isize {
        wr::<u32>(this, 0x184 + 4 * k, 0);
    }
    let cl = (rd::<u8>(this, 0x36) & 7) | 0x10;
    let f = (rd::<u32>(this, 0x1C) & 0xFFFF_FFFE) | 0x1000_0400;
    wr::<u8>(this, 0x36, cl);
    wr::<u8>(this, 0x13C, 4);
    wr::<u32>(this, 0x17C, 0);
    wr::<u32>(this, 0x180, 0);
    wr::<u32>(this, 0x1C, f);
    object_set_is_static(this, 0);
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) & 0xF7FF_FFFF);
    wr::<u32>(this, 0x12C, 0x3F00_0000);
    this
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RP_ANIM_BLEND_CLUMP_INIT_SKINNED,
    HOOK_RP_ANIM_BLEND_CLUMP_INIT_UNSKINNED,
    HOOK_CLUMP_FILL_FRAME_ARRAY_SKINNED,
    HOOK_RP_ANIM_BLEND_NODE_UPDATE_KEY_FRAMES,
    HOOK_FOR_ALL_FRAMES_FIND_BONE_NAME,
    HOOK_FOR_ALL_FRAMES_FIND_NODE_NAME,
    HOOK_CONVERT_BONE_TAG_2_BONE_NAME,
    HOOK_CUTSCENE_MGR_FINISH_CUTSCENE,
    HOOK_CUTSCENE_MGR_HIDE_REQUESTED_OBJECTS,
    HOOK_CUTSCENE_MGR_CREATE_CUTSCENE_OBJECT,
    HOOK_CUTSCENE_OBJECT_CTOR,
];
