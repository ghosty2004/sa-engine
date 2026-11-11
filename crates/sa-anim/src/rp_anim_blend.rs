//! `RpAnimBlend` clump functions, the key-frame apply callback and `RtAnimAnimationCreate` (STD-6, batch
//! `std6-anim-1`). Ported from the 1.0 US bytes (0x4D15E0, 0x4D5EF0..0x4D6C68, 0x72FB00, 0x735360,
//! 0x7CCE40); gta-reversed `RpAnimBlend.cpp` as reading aid.
//!
//! Clump plugin data (`clump + [0xB5F878]`): a `CAnimBlendClumpData*` (0x14): +0 association list head
//! (a link), +8 frame count, +0x10 frames (`AnimBlendFrameData`, 0x18: +0 flags byte, +4 bone position,
//! +0x10 RwFrame* or key frame, +0x14 node id). `CAnimBlendAssociation`: +0 vtable, +4 link {next, prev},
//! +0x14 hierarchy, +0x18 blend amount, +0x1C blend delta, +0x2C animation id (i16), +0x2E flags (u16;
//! bit 0 playing, bit 4 partial). The list walks below use the link (association + 4).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GameData};
use sa_core::{hook, orig};
use sa_math::x87::X87;

orig! {
    /// `CAnimBlendClumpData::CAnimBlendClumpData()` (0x4CF0E0)
    fn clump_data_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4CF0E0 => crate::clumpdata::constructor;
    /// `CAnimBlendClumpData::~CAnimBlendClumpData()` (0x4CF100)
    fn clump_data_dtor(this: GPtr<u8>) = thiscall 0x4CF100 => crate::clumpdata::destructor;
    /// `CAnimBlendClumpData::SetNumberOfBones(n)` (0x4CF140)
    fn clump_data_set_number_of_bones(this: GPtr<u8>, n: i32) = thiscall 0x4CF140 => crate::clumpdata::set_number_of_bones;
    /// `CAnimBlendClumpData::ForAllFrames(cb, data)` (0x4CF190)
    fn clump_data_for_all_frames(this: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CF190 => crate::clumpdata::for_all_frames;
    /// `CAnimBlendAssociation::Start(t)` (0x4CEB70)
    fn assoc_start(this: GPtr<u8>, t: f32) = thiscall 0x4CEB70 => crate::assoc::start;
    /// The association's virtual destructor (vtable slot 0, `deleting` flag)
    fn assoc_vdtor(this: GPtr<u8>, flags: u32) = thiscall vslot 0 => unported;
    /// `operator new(size)` (0x82119A), `operator delete(p)` (0x8214BD)
    fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    fn operator_delete(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// CRT `_stricmp` (0x8229B6), `strncpy` (0x821F40)
    fn crt_stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    fn crt_strncpy(d: GPtr<u8>, s: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// `CKeyGen::GetUppercaseKey(str)` (0x53CF30)
    fn get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => unported;
    /// `GetFrameNodeName(frame)` (0x72FB30)
    fn get_frame_node_name(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x72FB30 => unported;
    /// `NodeNamePlugin::SetFrameNodeName(frame, name)` (0x72FB00)
    fn set_frame_node_name_call(f: GPtr<u8>, name: GPtr<u8>) = cdecl 0x72FB00 => set_frame_node_name;
    /// `ConvertBoneTag2BoneName(tag)` (0x4D56F0)
    fn convert_bone_tag_2_bone_name(tag: i32) -> GPtr<u8> = cdecl 0x4D56F0 => unported;
    /// `ConvertPedNode2BoneTag(node)` (0x4D58A0)
    fn convert_ped_node_2_bone_tag(node: i32) -> i32 = cdecl 0x4D58A0 => unported;
    /// `RwHelper::GetFirstAtomic(clump)` (0x734820)
    fn get_first_atomic(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734820 => unported;
    /// `RwHelper::GetAnimHierarchyFromSkinClump(clump)` (0x734A40)
    fn get_anim_hierarchy_from_skin_clump(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `RpSkinGeometryGetSkin(geometry)` (0x7C7550), `RpSkinGetNumBones(skin)` (0x7C77E0),
    /// `RpSkinGetSkinToBoneMatrices(skin)` (0x7C7810)
    fn skin_geometry_get_skin(g: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7550 => unported;
    fn skin_get_num_bones(s: GPtr<u8>) -> i32 = cdecl 0x7C77E0 => unported;
    fn skin_get_skin_to_bone_matrices(s: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7810 => unported;
    /// `RpHAnimIDGetIndex(hierarchy, id)` (0x7C51A0)
    fn hanim_id_get_index(h: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `RpClumpForAllAtomics(clump, cb, data)` (0x749B70)
    fn clump_for_all_atomics(c: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `RwFrameForAllChildren(frame, cb, data)` (0x7F0DC0)
    fn frame_for_all_children(f: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x7F0DC0 => unported;
    /// `RwMatrixInvert(out, in)` (0x7F2070)
    fn matrix_invert(out: GPtr<u8>, inp: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F2070 => unported;
    /// `RwV3dTransformPoints(out, in, n, matrix)` (0x7EDD90)
    fn v3d_transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
    /// `CVisibilityPlugins::GetFrameHierarchyId(frame)` (0x732A20)
    fn get_frame_hierarchy_id(f: GPtr<u8>) -> i32 = cdecl 0x732A20 => unported;
    /// `RpAnimBlendAllocateData(clump)` (0x4D5F50)
    fn allocate_data_call(c: GPtr<u8>) = cdecl 0x4D5F50 => rp_anim_blend_allocate_data;
    /// `RpAnimBlendClumpIsInitialized(clump)` (0x4D6760)
    fn is_initialized_call(c: GPtr<u8>) -> u32 = cdecl 0x4D6760 => rp_anim_blend_clump_is_initialized;
    /// `RpAnimBlendClumpRemoveAllAssociations(clump)` (0x4D6C00)
    fn remove_all_associations_call(c: GPtr<u8>) = cdecl 0x4D6C00 => rp_anim_blend_clump_remove_all_associations;
    /// `SkinGetBonePositionsToTable(clump, table)` (0x735360)
    fn skin_get_bone_positions_to_table_call(c: GPtr<u8>, t: GPtr<u8>) = cdecl 0x735360 => skin_get_bone_positions_to_table;
    /// `RtAnimAnimationCreate(typeID, numFrames, flags, duration)` (0x7CCE40)
    fn animation_create_call(ty: u32, n: i32, flags: u32, duration: f32) -> GPtr<u8> = cdecl 0x7CCE40 => rt_anim_animation_create;
    /// `_rwerror(code)` (0x8088D0), `RwErrorSet(err)` (0x808820)
    fn rw_error1(code: u32) -> u32 = cdecl 0x8088D0 => unported;
    fn rw_error_set(e: GPtr<u32>) -> GPtr<u32> = cdecl 0x808820 => unported;
    /// `G->rwmalloc(size, hint)` (`RwGlobals` +0x134)
    fn rw_malloc(size: u32, hint: u32) -> u32 = cdecl table 0xC97B24 0x134 => unported;
}

/// `RpAnimBlend` clump plugin offset, the frame found by the `Find*` callbacks.
const CLUMP_OFFSET: usize = 0xB5F878;
const FOUND_FRAME: usize = 0xB5F87C;
/// The node-name plugin offset (`[0xC87C5C]`).
const NODE_NAME_OFFSET: usize = 0xC87C5C;
/// RtAnim interpolation schemes (0x30 each, +0 type id, +8 key frame size, +0x2C custom data size) and
/// their count; `G`.
const SCHEMES: u32 = 0xC9B920;
const SCHEME_COUNT: usize = 0xC97900;

fn g<T: GameData>(a: usize) -> GPtr<T> {
    GPtr::from_addr(a)
}

fn rd<T: GameData>(p: GPtr<u8>, off: isize) -> T {
    p.byte_add(off).cast::<T>().read()
}

fn wr<T: GameData>(p: GPtr<u8>, off: isize, v: T) {
    p.byte_add(off).cast::<T>().write(v)
}

fn p8(a: u32) -> GPtr<u8> {
    GPtr::from_addr(a as usize)
}

fn a32(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

fn clump_data(clump: GPtr<u8>) -> GPtr<u8> {
    p8(rd::<u32>(clump, g::<i32>(CLUMP_OFFSET).read() as isize))
}

/// The first link of a clump's association list.
fn first_link(clump: GPtr<u8>) -> GPtr<u8> {
    p8(rd::<u32>(clump_data(clump), 0))
}

fn next(link: GPtr<u8>) -> GPtr<u8> {
    p8(rd::<u32>(link, 0))
}

fn assoc_of(link: GPtr<u8>) -> GPtr<u8> {
    link.byte_add(-4)
}

fn is_partial(link: GPtr<u8>) -> bool {
    (rd::<u8>(link, 0x2A) >> 4) & 1 != 0
}

fn has_skin(clump: GPtr<u8>) -> bool {
    let a = get_first_atomic(clump);
    !a.is_null() && !skin_geometry_get_skin(p8(rd::<u32>(a, 0x18))).is_null()
}

/// `RpAnimBlend::RpAnimBlendClumpGetFirstAssociation(clump)` (0x4D15E0, a protected entry: the chunk
/// 0x401829 loads the plugin offset): NULL unless initialised, else the first association.
#[sa_core::native(addr = 0x4D15E0, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpGetFirstAssociation-")]
pub fn rp_anim_blend_clump_get_first_association(clump: GPtr<u8>) -> u32 {
    let d = clump_data(clump);
    if is_initialized_call(clump) & 0xFF == 0 {
        return 0;
    }
    let l = rd::<u32>(d, 0);
    if l == 0 { 0 } else { l - 4 }
}

/// `RpAnimBlend::RpAnimBlendFrameGetName(frame)` (0x4D5EF0): `jmp GetFrameNodeName`. A `jmp` into game
/// `.text` (not a `.HOODLUM` stub), so not hookable as is: a phase B native.
#[sa_core::native(addr = 0x4D5EF0, conv = cdecl, name = "RpAnimBlend::RpAnimBlendFrameGetName")]
pub fn rp_anim_blend_frame_get_name(frame: GPtr<u8>) -> u32 {
    a32(get_frame_node_name(frame))
}

/// `RpAnimBlend::RpAnimBlendFrameSetName(frame, name)` (0x4D5F00). Returns the frame.
#[hook(addr = 0x4D5F00, conv = cdecl, name = "RpAnimBlend::RpAnimBlendFrameSetName", expect = "8B 44 24 08 56 8B 74 24 08",
       unverified = "std6-anim-1", route = "L2", hazards = "none")]
pub fn rp_anim_blend_frame_set_name(frame: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> {
    set_frame_node_name_call(frame, name);
    frame
}

/// `NodeNamePlugin::SetFrameNodeName(frame, name)` (0x72FB00): with a positive plugin offset, 23 bytes
/// `strncpy`d and the 24th cleared.
#[hook(addr = 0x72FB00, conv = cdecl, name = "NodeNamePlugin::SetFrameNodeName", expect = "A1 5C 7C C8 00 85 C0",
       unverified = "std6-anim-1", route = "L1-pure", hazards = "none")]
pub fn set_frame_node_name(frame: GPtr<u8>, name: GPtr<u8>) {
    let off = g::<i32>(NODE_NAME_OFFSET).read();
    if off > 0 {
        let p = frame.byte_add(off as isize);
        crt_strncpy(p, name, 0x17);
        wr::<u8>(p, 0x17, 0);
    }
}

/// `CAnimBlendClumpData` scalar deleting destructor (0x4D5F20, thiscall): destructor, `operator
/// delete` with flag 1. Returns `this`.
#[hook(addr = 0x4D5F20, conv = thiscall, name = "CAnimBlendClumpData", expect = "56 8B F1 E8 D8 91 FF FF",
       unverified = "std6-anim-1", route = "L2", hazards = "heap-writes")]
pub fn clump_data_deleting_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    clump_data_dtor(this);
    if flags & 1 != 0 {
        operator_delete(this);
    }
    this
}

/// `RpAnimBlend::RpAnimBlendAllocateData(clump)` (0x4D5F50): a new `CAnimBlendClumpData` (0x14) as
/// the clump's plugin data (NULL when the allocation fails).
#[hook(addr = 0x4D5F50, conv = cdecl, name = "RpAnimBlend::RpAnimBlendAllocateData", expect = "6A 14 E8 43 B2 34 00",
       unverified = "std6-anim-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn rp_anim_blend_allocate_data(clump: GPtr<u8>) {
    let p = operator_new(0x14);
    let d = if p.is_null() { GPtr::NULL } else { clump_data_ctor(p) };
    wr::<u32>(clump, g::<i32>(CLUMP_OFFSET).read() as isize, a32(d));
}

/// `RpAnimBlend::RtAnimBlendKeyFrameApply(matrix, frame)` (0x4D5FA0): the rotation matrix of the
/// frame's quaternion (x, y, z, w) and its translation (+0x10), flags 3. The x87 order is kept: the
/// squares and cross products are stored as floats, `w*y` and `w*z` stay on the stack.
#[hook(addr = 0x4D5FA0, conv = cdecl, name = "RpAnimBlend::RtAnimBlendKeyFrameApply", expect = "83 EC 28 8B 4C 24 30",
       unverified = "std6-anim-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn rt_anim_blend_key_frame_apply(m: GPtr<u8>, frame: GPtr<u8>) {
    let q = |o: isize| X87::from_f32(rd::<f32>(frame, o));
    let (x, y, z, w) = (q(0), q(4), q(8), q(0xC));
    let st = |v: X87| X87::from_f32(v.to_f32());
    let xx = st(x.fmul(x));
    let yy = st(y.fmul(y));
    let zz = st(z.fmul(z));
    let yz = st(z.fmul(y));
    let xz = st(z.fmul(x));
    let xy = st(y.fmul(x));
    let wx = st(w.fmul(x));
    let wy = w.fmul(y);
    let wz = w.fmul(z);
    let one = X87::from_f32(1.0);
    let two = |v: X87| v.fadd(v);
    wr::<f32>(m, 0, one.fsub(two(zz.fadd(yy))).to_f32());
    wr::<f32>(m, 4, two(xy.fadd(wz)).to_f32());
    wr::<f32>(m, 8, two(xz.fsub(wy)).to_f32());
    wr::<f32>(m, 0x10, two(xy.fsub(wz)).to_f32());
    wr::<f32>(m, 0x14, one.fsub(two(zz.fadd(xx))).to_f32());
    wr::<f32>(m, 0x18, two(wx.fadd(yz)).to_f32());
    wr::<f32>(m, 0x20, two(wy.fadd(xz)).to_f32());
    wr::<f32>(m, 0x24, two(yz.fsub(wx)).to_f32());
    let atz = one.fsub(two(yy.fadd(xx))).to_f32();
    wr::<u32>(m, 0x30, 0);
    wr::<u32>(m, 0x34, 0);
    wr::<u32>(m, 0x38, 0);
    wr::<u32>(m, 0xC, 3);
    wr::<f32>(m, 0x28, atz);
    wr::<u32>(m, 0x30, rd::<u32>(frame, 0x10));
    wr::<u32>(m, 0x34, rd::<u32>(frame, 0x14));
    wr::<u32>(m, 0x38, rd::<u32>(frame, 0x18));
}

/// `RtAnimAnimationCreate(typeID, numFrames, flags, duration)` (0x7CCE40): the registered scheme of the
/// type (else error {0x1B7, `_rwerror(2)`} and NULL); `RwMalloc(numFrames * keyFrameSize +
/// customDataSize + 0x18, 0x3001B)` (unchecked): +0 scheme, +4 numFrames, +8 flags, +0xC duration,
/// +0x10 frames (+0x18), +0x14 custom data after the frames (NULL without).
#[hook(addr = 0x7CCE40, conv = cdecl, name = "RtAnimAnimationCreate", expect = "8B 15 00 79 C9 00 83 EC 08",
       unverified = "std6-anim-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn rt_anim_animation_create(ty: u32, n: i32, flags: u32, duration: f32) -> GPtr<u8> {
    let count = g::<i32>(SCHEME_COUNT).read();
    let mut info = GPtr::<u8>::NULL;
    for i in 0..count.max(0) {
        let s = p8(SCHEMES + i as u32 * 0x30);
        if rd::<u32>(s, 0) == ty {
            info = s;
            break;
        }
    }
    if info.is_null() {
        let mut e = [0x1B7u32, 0];
        e[1] = rw_error1(2);
        rw_error_set(GPtr::from_raw(e.as_mut_ptr()));
        return GPtr::NULL;
    }
    let kfs = rd::<i32>(info, 8);
    let size = kfs.wrapping_mul(n).wrapping_add(rd::<i32>(info, 0x2C)).wrapping_add(0x18) as u32;
    let a = p8(rw_malloc(size, 0x3001B));
    wr::<u32>(a, 8, flags);
    wr::<i32>(a, 4, n);
    wr::<f32>(a, 0xC, duration);
    wr::<u32>(a, 0, a32(info));
    let frames = a32(a) + 0x18;
    wr::<u32>(a, 0x10, frames);
    if rd::<i32>(info, 0x2C) > 0 {
        wr::<u32>(a, 0x14, frames.wrapping_add(kfs.wrapping_mul(n) as u32));
    } else {
        wr::<u32>(a, 0x14, 0);
    }
    a
}

/// `RpAnimBlend::RpAnimBlendCreateAnimationForHierarchy(hierarchy)` (0x4D60E0): NULL for NULL; an
/// `RtAnimAnimationCreate(0x253F2FB, 0, 0, 0.0)` whose frame count becomes `2 * numNodes`.
#[hook(addr = 0x4D60E0, conv = cdecl, name = "RpAnimBlend::RpAnimBlendCreateAnimationForHierarchy", expect = "8B 44 24 04 85 C0 75 01 C3",
       unverified = "std6-anim-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn rp_anim_blend_create_animation_for_hierarchy(h: GPtr<u8>) -> GPtr<u8> {
    if h.is_null() {
        return h;
    }
    let n = rd::<i32>(h, 4);
    let a = animation_create_call(0x253_F2FB, 0, 0, 0.0);
    if a.is_null() {
        return a;
    }
    wr::<i32>(a, 4, n + n);
    a
}

/// Bone-name frame search (0x4D6240, `ForAllFrames` callback): the frame whose bone tag's name
/// `_stricmp`-matches `name` is stored in the found-frame global.
#[hook(addr = 0x4D6240, conv = cdecl, name = "ForAllFramesF", expect = "56 8B 74 24 08 8B 46 14 50",
       unverified = "std6-anim-1", route = "L2", hazards = "none")]
pub fn find_frame_by_bone_name_cb(f: GPtr<u8>, name: GPtr<u8>) {
    let bn = convert_bone_tag_2_bone_name(rd::<i32>(f, 0x14));
    if !bn.is_null() && crt_stricmp(name, bn) == 0 {
        g::<u32>(FOUND_FRAME).write(a32(f));
    }
}

/// Node-name frame search (0x4D6270, `ForAllFrames` callback).
#[hook(addr = 0x4D6270, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "56 8B 74 24 08 8B 46 10 50",
       unverified = "std6-anim-1", route = "L2", hazards = "none")]
pub fn find_frame_by_node_name_cb(f: GPtr<u8>, name: GPtr<u8>) {
    let nn = get_frame_node_name(p8(rd::<u32>(f, 0x10)));
    if crt_stricmp(name, nn) == 0 {
        g::<u32>(FOUND_FRAME).write(a32(f));
    }
}

/// `RpAnimBlend::RpAnimBlendClumpFindFrame(clump, name)` (0x4D62A0): by bone name for a skinned clump,
/// else by node name; NULL when none.
#[hook(addr = 0x4D62A0, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpFindFrame", expect = "8B 44 24 04 8B 0D 78 F8 B5 00 56",
       unverified = "std6-anim-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn rp_anim_blend_clump_find_frame(clump: GPtr<u8>, name: GPtr<u8>) -> u32 {
    let d = clump_data(clump);
    g::<u32>(FOUND_FRAME).write(0);
    let cb = if has_skin(clump) { 0x4D6240 } else { 0x4D6270 };
    clump_data_for_all_frames(d, cb, a32(name));
    g::<u32>(FOUND_FRAME).read()
}

/// `RpAnimBlend::RpAnimBlendClumpFindFrameFromHashKey(clump, key)` (0x4D6370).
#[hook(addr = 0x4D6370, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpFindFrameFromHashKey", expect = "8B 44 24 04 8B 0D 78 F8 B5 00 56",
       unverified = "std6-anim-1", route = "L2", hazards = "none")]
pub fn rp_anim_blend_clump_find_frame_from_hash_key(clump: GPtr<u8>, key: u32) -> u32 {
    let d = clump_data(clump);
    g::<u32>(FOUND_FRAME).write(0);
    let cb = if has_skin(clump) { 0x4D6310 } else { 0x4D6340 };
    clump_data_for_all_frames(d, cb, key);
    g::<u32>(FOUND_FRAME).read()
}

/// Bone-id frame search (0x4D63E0, `ForAllFrames` callback).
#[hook(addr = 0x4D63E0, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 44 24 04 8B 4C 24 08 3B 48 14",
       unverified = "std6-anim-1", route = "L1-pure", hazards = "none")]
pub fn find_bone_cb(f: GPtr<u8>, id: i32) {
    if id == rd::<i32>(f, 0x14) {
        g::<u32>(FOUND_FRAME).write(a32(f));
    }
}

/// `RpAnimBlend::RpAnimBlendClumpFindBone(clump, boneId)` (0x4D6400).
#[hook(addr = 0x4D6400, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpFindBone", expect = "8B 44 24 08 8B 15 78 F8 B5 00",
       unverified = "std6-anim-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn rp_anim_blend_clump_find_bone(clump: GPtr<u8>, id: i32) -> u32 {
    let d = clump_data(clump);
    g::<u32>(FOUND_FRAME).write(0);
    clump_data_for_all_frames(d, 0x4D63E0, id as u32);
    g::<u32>(FOUND_FRAME).read()
}

/// Frame-array fill for a non-skinned clump (0x4D6430, `ForAllFrames` callback): `array[hierarchy id
/// of the frame's RwFrame] = frame`.
#[hook(addr = 0x4D6430, conv = cdecl, name = "ForAllFramesF", expect = "56 8B 74 24 08 8B 46 10 50 E8",
       unverified = "std6-anim-1", route = "L2", hazards = "none")]
pub fn fill_frame_array_cb(f: GPtr<u8>, array: GPtr<u32>) {
    let i = get_frame_hierarchy_id(p8(rd::<u32>(f, 0x10)));
    array.byte_add(i as isize * 4).write(a32(f));
}

/// `RpAnimBlend::RpAnimBlendClumpFillFrameArray(clump, array)` (0x4D64A0): skinned: entries 1..18 are
/// the frames of the ped nodes' bone tags (`RpHAnimIDGetIndex`, the helper 0x4D6450 inlined); else by
/// hierarchy id.
#[hook(addr = 0x4D64A0, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpFillFrameArray", expect = "A1 78 F8 B5 00 56 8B 74 24 08",
       unverified = "std6-anim-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn rp_anim_blend_clump_fill_frame_array(clump: GPtr<u8>, array: GPtr<u32>) {
    let d = clump_data(clump);
    if has_skin(clump) {
        let h = get_anim_hierarchy_from_skin_clump(clump);
        for i in 1..0x13 {
            let idx = hanim_id_get_index(h, convert_ped_node_2_bone_tag(i));
            array.byte_add(i as isize * 4).write(rd::<u32>(d, 0x10).wrapping_add((idx as u32).wrapping_mul(0x18)));
        }
    } else {
        clump_data_for_all_frames(d, 0x4D6430, a32(array.cast()));
    }
}

/// Frame-flags reset (0x4D6500, `ForAllFrames` callback): flags = 0.
#[hook(addr = 0x4D6500, conv = cdecl, name = "ForAllFramesF", expect = "8B 44 24 04 C6 00 00 C3",
       unverified = "std6-anim-1", route = "L1-pure", hazards = "none")]
pub fn clear_frame_flags_cb(f: GPtr<u8>, _data: u32) {
    wr::<u8>(f, 0, 0);
}

/// `SkinGetBonePositionsToTable(clump, table)` (0x735360): table[0] = 0; for bone i >= 1: the position
/// of the inverted skin-to-bone matrix i transformed by the parent bone's skin-to-bone matrix, the
/// parent tracked through the hierarchy's push (2) / pop (1) node flags.
#[hook(addr = 0x735360, conv = cdecl, name = "SkinGetBonePositionsToTable", expect = "81 EC 5C 01 00 00 55 56",
       unverified = "std6-anim-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn skin_get_bone_positions_to_table(clump: GPtr<u8>, table: GPtr<u8>) {
    if table.is_null() {
        return;
    }
    let mut atomic = 0u32;
    clump_for_all_atomics(clump, 0x734810, GPtr::from_raw(&mut atomic as *mut u32).addr() as u32);
    let skin = skin_geometry_get_skin(p8(rd::<u32>(p8(atomic), 0x18)));
    let mut hier = 0u32;
    clump_for_all_atomics(clump, 0x734A20, GPtr::from_raw(&mut hier as *mut u32).addr() as u32);
    for o in [0, 4, 8] {
        wr::<u32>(table, o, 0);
    }
    let n = skin_get_num_bones(skin);
    let mut parent = 0i32;
    let mut stack = [0i32; 64];
    let mut sp = 0usize;
    let mut a = [0u8; 0x40];
    let mut b = [0u8; 0x40];
    let mut c = [0u8; 0x40];
    let (ap, bp, cp) = (GPtr::from_raw(a.as_mut_ptr()), GPtr::from_raw(b.as_mut_ptr()), GPtr::from_raw(c.as_mut_ptr()));
    for i in 1..n.max(1) {
        let mats = skin_get_skin_to_bone_matrices(skin);
        for (k, v) in a.iter_mut().enumerate() {
            *v = rd::<u8>(mats, i as isize * 0x40 + k as isize);
        }
        matrix_invert(bp, ap);
        let mats = skin_get_skin_to_bone_matrices(skin);
        for (k, v) in c.iter_mut().enumerate() {
            *v = rd::<u8>(mats, parent as isize * 0x40 + k as isize);
        }
        v3d_transform_points(table.byte_add(i as isize * 0xC), bp.byte_add(0x30), 1, cp);
        let flags = rd::<u32>(p8(rd::<u32>(p8(hier), 0x10)), i as isize * 0x10 + 8);
        if flags & 2 != 0 {
            sp = (sp + 1).min(stack.len() - 1);
            stack[sp] = parent;
        }
        if flags & 1 != 0 {
            parent = stack[sp];
            sp = sp.saturating_sub(1);
        } else {
            parent = i;
        }
    }
}

/// `RpAnimBlendClumpInitSkinned` (0x4D6510, clump in EAX): plugin data allocated, one frame per bone;
/// each frame's key frame (+0x10) is the hierarchy interpolator's node i, its id (+0x14) node i's id and
/// its position (+4) the bone position table entry; flags cleared, the first frame gets bit 3.
fn clump_init_skinned(clump: GPtr<u8>) {
    allocate_data_call(clump);
    let d = clump_data(clump);
    let atomic = get_first_atomic(clump);
    let skin = skin_geometry_get_skin(p8(rd::<u32>(atomic, 0x18)));
    let n = skin_get_num_bones(skin);
    clump_data_set_number_of_bones(d, n);
    let h = get_anim_hierarchy_from_skin_clump(clump);
    let mut table = [0u8; 0x300];
    skin_get_bone_positions_to_table_call(clump, GPtr::from_raw(table.as_mut_ptr()));
    let frames = p8(rd::<u32>(d, 0x10));
    for i in 0..n.max(0) {
        let f = frames.byte_add(i as isize * 0x18);
        let interp = p8(rd::<u32>(h, 0x20));
        let kf = rd::<i32>(interp, 0x24).wrapping_mul(i) as u32;
        wr::<u32>(f, 0x10, kf.wrapping_add(a32(interp)).wrapping_add(0x4C));
        wr::<u32>(f, 0x14, rd::<u32>(p8(rd::<u32>(h, 0x10)), i as isize * 0x10));
        let t = i as usize * 0xC;
        for k in 0..3 {
            let v = u32::from_le_bytes([table[t + k * 4], table[t + k * 4 + 1], table[t + k * 4 + 2], table[t + k * 4 + 3]]);
            wr::<u32>(f, 4 + k as isize * 4, v);
        }
    }
    clump_data_for_all_frames(d, 0x4D6500, 0);
    let f0 = p8(rd::<u32>(d, 0x10));
    wr::<u8>(f0, 0, rd::<u8>(f0, 0) | 8);
}

/// Frame counter (0x4D6610, `RwFrameForAllChildren` callback): `(*count)++`, then the children.
#[hook(addr = 0x4D6610, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 44 24 08 8B 08 56",
       unverified = "std6-anim-1", route = "L2", hazards = "none")]
pub fn count_frames_cb(frame: GPtr<u8>, count: GPtr<i32>) -> GPtr<u8> {
    count.write(count.read() + 1);
    frame_for_all_children(frame, 0x4D6610, count.addr() as u32);
    frame
}

/// Non-skinned frame init (0x4D6640, `ForAllFrames` callback): flags 0, position = the RwFrame's
/// modelling position (+0x40), id -1.
#[hook(addr = 0x4D6640, conv = cdecl, name = "ForAllFramesF", expect = "8B 44 24 04 8B 48 10 83 C1 40",
       unverified = "std6-anim-1", route = "L1-pure", hazards = "none")]
pub fn init_frame_not_skinned_cb(f: GPtr<u8>, _data: u32) {
    let rf = p8(rd::<u32>(f, 0x10)).byte_add(0x40);
    wr::<u8>(f, 0, 0);
    for k in [0isize, 4, 8] {
        wr::<u32>(f, 4 + k, rd::<u32>(rf, k));
    }
    wr::<i32>(f, 0x14, -1);
}

/// Frame binder (0x4D6670, `RwFrameForAllChildren` callback): `(*cursor)->frame = frame`, the cursor
/// advances one frame data, then the children.
#[hook(addr = 0x4D6670, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 44 24 08 8B 08 56 8B 74 24 08",
       unverified = "std6-anim-1", route = "L2", hazards = "none")]
pub fn bind_frames_cb(frame: GPtr<u8>, cursor: GPtr<u32>) -> GPtr<u8> {
    wr::<u32>(p8(cursor.read()), 0x10, a32(frame));
    cursor.write(cursor.read() + 0x18);
    frame_for_all_children(frame, 0x4D6670, cursor.addr() as u32);
    frame
}

/// `RpAnimBlendClumpInitNotSkinned` (0x4D66A0, clump in EAX): one frame data per RwFrame below the
/// clump's frame (counted, then bound in the same order), initialised from the frames; the first gets
/// bit 3.
fn clump_init_not_skinned(clump: GPtr<u8>) {
    let mut cell = 0u32;
    allocate_data_call(clump);
    let d = clump_data(clump);
    let frame = p8(rd::<u32>(clump, 4));
    let cp = GPtr::from_raw(&mut cell as *mut u32);
    frame_for_all_children(frame, 0x4D6610, cp.addr() as u32);
    clump_data_set_number_of_bones(d, cp.read() as i32);
    cp.write(rd::<u32>(d, 0x10));
    frame_for_all_children(frame, 0x4D6670, cp.addr() as u32);
    clump_data_for_all_frames(d, 0x4D6640, 0);
    let f0 = p8(rd::<u32>(d, 0x10));
    wr::<u8>(f0, 0, rd::<u8>(f0, 0) | 8);
}

/// `RpAnimBlend::RpAnimBlendClumpInit(clump)` (0x4D6720): skinned or non-skinned init.
#[hook(addr = 0x4D6720, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpInit", expect = "56 8B 74 24 08 56 E8",
       unverified = "std6-anim-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn rp_anim_blend_clump_init(clump: GPtr<u8>) {
    if has_skin(clump) {
        clump_init_skinned(clump);
    } else {
        clump_init_not_skinned(clump);
    }
}

/// `RpAnimBlend::RpAnimBlendClumpIsInitialized(clump)` (0x4D6760, a protected entry: the chunk
/// 0x401929 loads the plugin offset): 1 with plugin data and a non-zero frame count.
#[sa_core::native(addr = 0x4D6760, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpIsInitialized")]
pub fn rp_anim_blend_clump_is_initialized(clump: GPtr<u8>) -> u32 {
    let d = clump_data(clump);
    (!d.is_null() && rd::<u32>(d, 8) != 0) as u32
}

/// `RpAnimBlend::RpAnimBlendClumpAddAssociation(clump, assoc, flags, startTime, blendAmount)`
/// (0x4D6790): linked at the head, started at `startTime`, flags or-ed in, blend amount set. Returns the
/// association.
#[hook(addr = 0x4D6790, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpAddAssociation", expect = "8B 0D 78 F8 B5 00 8B 44 24 04",
       unverified = "std6-anim-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn rp_anim_blend_clump_add_association(clump: GPtr<u8>, assoc: GPtr<u8>, flags: u32, start: f32, blend: f32) -> GPtr<u8> {
    let d = clump_data(clump);
    let link = assoc.byte_add(4);
    let head = rd::<u32>(d, 0);
    if head != 0 {
        wr::<u32>(p8(head), 4, a32(link));
    }
    wr::<u32>(link, 4, a32(d));
    wr::<u32>(link, 0, rd::<u32>(d, 0));
    wr::<u32>(d, 0, a32(link));
    assoc_start(assoc, start);
    wr::<u16>(assoc, 0x2E, rd::<u16>(assoc, 0x2E) | flags as u16);
    wr::<f32>(assoc, 0x18, blend);
    assoc
}

/// `RpAnimBlend::RpAnimBlendClumpSetBlendDeltas(clump, mask, delta)` (0x4D67E0): every association
/// (with any `mask` flag, or all for 0) gets the blend delta.
#[hook(addr = 0x4D67E0, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpSetBlendDeltas", expect = "8B 44 24 04 8B 0D 78 F8 B5 00 8B 04 01 8B 00",
       unverified = "std6-anim-1", route = "L2", hazards = "none")]
pub fn rp_anim_blend_clump_set_blend_deltas(clump: GPtr<u8>, mask: u32, delta: f32) {
    let mut l = first_link(clump);
    while !l.is_null() {
        if mask == 0 || mask & rd::<u16>(l, 0x2A) as u32 != 0 {
            wr::<f32>(l, 0x18, delta);
        }
        l = next(l);
    }
}

/// `RpAnimBlend::RpAnimBlendClumpRemoveAssociations(clump, mask)` (0x4D6820): every association (with
/// any `mask` flag, or all for 0) is deleted (virtual destructor, flag 1).
#[hook(addr = 0x4D6820, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpRemoveAssociations", expect = "8B 44 24 04 8B 0D 78 F8 B5 00 8B 04 01 8B 00 85 C0 74 30",
       unverified = "std6-anim-1", route = "L2", hazards = "heap-writes")]
pub fn rp_anim_blend_clump_remove_associations(clump: GPtr<u8>, mask: u32) {
    let mut l = first_link(clump);
    while !l.is_null() {
        let nx = next(l);
        let a = assoc_of(l);
        if (mask == 0 || mask & rd::<u16>(a, 0x2E) as u32 != 0) && !a.is_null() {
            assoc_vdtor(a, 1);
        }
        l = nx;
    }
}

/// `RpAnimBlend::RpAnimBlendClumpGetAssociation(clump, name)` (0x4D6870): by the hierarchy name hash.
#[hook(addr = 0x4D6870, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpGetAssociation-AnimName", expect = "8B 54 24 08 8B 44 24 04",
       unverified = "std6-anim-1", route = "L2", hazards = "none")]
pub fn rp_anim_blend_clump_get_association_name(clump: GPtr<u8>, name: GPtr<u8>) -> u32 {
    let d = clump_data(clump);
    let key = get_uppercase_key(name);
    let mut l = p8(rd::<u32>(d, 0));
    while !l.is_null() {
        if rd::<u32>(p8(rd::<u32>(l, 0x10)), 0) == key {
            return a32(l) - 4;
        }
        l = next(l);
    }
    0
}

/// `RpAnimBlend::RpAnimBlendClumpGetAssociation(clump, animId)` (0x4D68B0).
#[hook(addr = 0x4D68B0, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpGetAssociation-AnimId", expect = "8B 44 24 04 8B 0D 78 F8 B5 00 8B 04 01 8B 00 85 C0 74 12",
       unverified = "std6-anim-1", route = "L1-pure", hazards = "none")]
pub fn rp_anim_blend_clump_get_association_id(clump: GPtr<u8>, id: i32) -> u32 {
    let mut l = first_link(clump);
    while !l.is_null() {
        if rd::<i16>(l, 0x28) as i32 == id {
            return a32(l) - 4;
        }
        l = next(l);
    }
    0
}

/// `RpAnimBlend::RpAnimBlendClumpGetAssociation(clump, unused, hierarchy)` (0x4D68E0).
#[hook(addr = 0x4D68E0, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpGetAssociation-Hier", expect = "8B 44 24 04 8B 0D 78 F8 B5 00 8B 04 01 8B 00 85 C0 74 0F",
       unverified = "std6-anim-1", route = "L1-pure", hazards = "none")]
pub fn rp_anim_blend_clump_get_association_hier(clump: GPtr<u8>, _b: u32, h: u32) -> u32 {
    let mut l = first_link(clump);
    while !l.is_null() {
        if rd::<u32>(l, 0x10) == h {
            return a32(l) - 4;
        }
        l = next(l);
    }
    0
}

/// `RpAnimBlend::RpAnimBlendClumpGetMainAssociation(clump, outSecond, outSecondBlend)` (0x4D6910): the
/// non-partial association with the highest blend amount (strictly above the best so far, starting at
/// 0) and the runner-up (the previous best, or one whose blend is above the runner-up's); the
/// runner-up and its blend are stored when asked for.
#[hook(addr = 0x4D6910, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpGetMainAssociation", expect = "51 8B 0D 78 F8 B5 00",
       unverified = "std6-anim-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn rp_anim_blend_clump_get_main_association(clump: GPtr<u8>, out_second: GPtr<u32>, out_blend: GPtr<f32>) -> u32 {
    let mut l = first_link(clump);
    let (mut main, mut second) = (0u32, 0u32);
    let (mut best, mut second_blend) = (0.0f32, 0.0f32);
    while !l.is_null() {
        if !is_partial(l) {
            let b = rd::<f32>(l, 0x14);
            if b > best {
                second = main;
                second_blend = best;
                best = b;
                main = a32(l) - 4;
            } else if second_blend < b {
                second = a32(l) - 4;
                second_blend = b;
            }
        }
        l = next(l);
    }
    if !out_second.is_null() {
        out_second.write(second);
    }
    if !out_blend.is_null() {
        out_blend.write(second_blend);
    }
    main
}

/// `RpAnimBlend::RpAnimBlendClumpGetMainPartialAssociation(clump)` (0x4D69A0): the partial
/// association with the highest blend (strictly above the best so far, from 0).
#[hook(addr = 0x4D69A0, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpGetMainPartialAssociation", expect = "8B 0D 78 F8 B5 00 D9 05 50 8B 85 00",
       unverified = "std6-anim-1", route = "L2", hazards = "none")]
pub fn rp_anim_blend_clump_get_main_partial_association(clump: GPtr<u8>) -> u32 {
    let mut l = first_link(clump);
    let mut best = 0.0f32;
    let mut r = 0u32;
    while !l.is_null() {
        if is_partial(l) {
            let b = rd::<f32>(l, 0x14);
            if best < b {
                r = a32(l) - 4;
                best = b;
            }
        }
        l = next(l);
    }
    r
}

/// The n-th association whose partial flag equals `partial` (0x4D69F0: non-partial; 0x4D6A30:
/// partial).
fn nth(clump: GPtr<u8>, n: i32, partial: bool) -> u32 {
    let mut l = first_link(clump);
    let mut i = 0;
    while !l.is_null() {
        if is_partial(l) == partial {
            if i == n {
                return a32(l) - 4;
            }
            i += 1;
        }
        l = next(l);
    }
    0
}

/// `RpAnimBlend::RpAnimBlendClumpGetMainPartialAssociation_N(clump, n)` (0x4D69F0): the n-th
/// non-partial association (the bytes test the partial bit and skip set ones).
#[hook(addr = 0x4D69F0, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpGetMainPartialAssociation_N", expect = "8B 44 24 04 8B 15 78 F8 B5 00 8B 04 02 8B 00 33 C9 85 C0 56 74 20 8B 74 24 0C 8D 9B 00 00 00 00 8A 50 2A C0 EA 04 F6 C2 01 75",
       unverified = "std6-anim-1", route = "L1-pure", hazards = "none")]
pub fn rp_anim_blend_clump_get_main_partial_association_n(clump: GPtr<u8>, n: i32) -> u32 {
    nth(clump, n, false)
}

/// `RpAnimBlend::RpAnimBlendClumpGetMainAssociation_N(clump, n)` (0x4D6A30): the n-th partial
/// association.
#[hook(addr = 0x4D6A30, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpGetMainAssociation_N", expect = "8B 44 24 04 8B 15 78 F8 B5 00 8B 04 02 8B 00 33 C9 85 C0 56 74 20 8B 74 24 0C 8D 9B 00 00 00 00 8A 50 2A C0 EA 04 F6 C2 01 74",
       unverified = "std6-anim-1", route = "L1-pure", hazards = "none")]
pub fn rp_anim_blend_clump_get_main_association_n(clump: GPtr<u8>, n: i32) -> u32 {
    nth(clump, n, true)
}

/// `RpAnimBlend::RpAnimBlendClumpGetFirstAssociation(clump, flags)` (0x4D6A70): the first association
/// with any of `flags`.
#[hook(addr = 0x4D6A70, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpGetFirstAssociation-Flags", expect = "8B 0D 78 F8 B5 00 8B 44 24 04 8B 04 01 8B 08",
       unverified = "std6-anim-1", route = "L1-pure", hazards = "none")]
pub fn rp_anim_blend_clump_get_first_association_flags(clump: GPtr<u8>, flags: u32) -> u32 {
    let mut l = first_link(clump);
    while !l.is_null() {
        if flags & rd::<u16>(l, 0x2A) as u32 != 0 {
            return a32(l) - 4;
        }
        l = next(l);
    }
    0
}

/// `RpAnimBlend::RpAnimBlendGetNextAssociation(assoc)` (0x4D6AB0).
#[hook(addr = 0x4D6AB0, conv = cdecl, name = "RpAnimBlend::RpAnimBlendGetNextAssociation-Any", expect = "8B 44 24 04 8B 40 04 85 C0",
       unverified = "std6-anim-1", route = "L1-pure", hazards = "none")]
pub fn rp_anim_blend_get_next_association(assoc: GPtr<u8>) -> u32 {
    let n = rd::<u32>(assoc, 4);
    if n == 0 { 0 } else { n - 4 }
}

/// `RpAnimBlend::RpAnimBlendGetNextAssociation(assoc, flags)` (0x4D6AD0).
#[hook(addr = 0x4D6AD0, conv = cdecl, name = "RpAnimBlend::RpAnimBlendGetNextAssociation-Flags", expect = "8B 44 24 04 8B 48 04 85 C9",
       unverified = "std6-anim-1", route = "L1-pure", hazards = "none")]
pub fn rp_anim_blend_get_next_association_flags(assoc: GPtr<u8>, flags: u32) -> u32 {
    let mut l = p8(rd::<u32>(assoc, 4));
    while !l.is_null() {
        if flags & rd::<u16>(l, 0x2A) as u32 != 0 {
            return a32(l) - 4;
        }
        l = next(l);
    }
    0
}

/// `RpAnimBlend::RpAnimBlendClumpPauseAllAnimations(clump)` (0x4D6B00): playing bit cleared.
#[hook(addr = 0x4D6B00, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpPauseAllAnimations", expect = "8B 44 24 04 8B 0D 78 F8 B5 00 8B 04 01 8B 00 85 C0 74 0F B9",
       unverified = "std6-anim-1", route = "L1-pure", hazards = "none")]
pub fn rp_anim_blend_clump_pause_all_animations(clump: GPtr<u8>) {
    let mut l = first_link(clump);
    while !l.is_null() {
        wr::<u16>(l, 0x2A, rd::<u16>(l, 0x2A) & 0xFFFE);
        l = next(l);
    }
}

/// `RpAnimBlend::RpAnimBlendClumpUnPauseAllAnimations(clump)` (0x4D6B30): playing bit set.
#[hook(addr = 0x4D6B30, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpUnPauseAllAnimations", expect = "8B 44 24 04 8B 0D 78 F8 B5 00 8B 04 01 8B 00 85 C0 74 0F B9 01",
       unverified = "std6-anim-1", route = "L1-pure", hazards = "none")]
pub fn rp_anim_blend_clump_unpause_all_animations(clump: GPtr<u8>) {
    let mut l = first_link(clump);
    while !l.is_null() {
        wr::<u16>(l, 0x2A, rd::<u16>(l, 0x2A) | 1);
        l = next(l);
    }
}

fn count_links(clump: GPtr<u8>, pred: impl Fn(GPtr<u8>) -> bool) -> u32 {
    let mut l = first_link(clump);
    let mut n = 0;
    while !l.is_null() {
        if pred(l) {
            n += 1;
        }
        l = next(l);
    }
    n
}

/// `RpAnimBlend::RpAnimBlendClumpGetNumAssociations(clump)` (0x4D6B60).
#[hook(addr = 0x4D6B60, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpGetNumAssociations", expect = "8B 44 24 04 8B 0D 78 F8 B5 00 8B 0C 01 8B 09 33 C0 85 C9 74 07",
       unverified = "std6-anim-1", route = "L1-pure", hazards = "none")]
pub fn rp_anim_blend_clump_get_num_associations(clump: GPtr<u8>) -> u32 {
    count_links(clump, |_| true)
}

/// `RpAnimBlend::RpAnimBlendClumpGetNumPartialAssociations(clump)` (0x4D6B80).
#[hook(addr = 0x4D6B80, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpGetNumPartialAssociations", expect = "8B 44 24 04 8B 0D 78 F8 B5 00 8B 0C 01 8B 09 33 C0 85 C9 74 12 8A 51 2A C0 EA 04 F6 C2 01 74",
       unverified = "std6-anim-1", route = "L1-pure", hazards = "none")]
pub fn rp_anim_blend_clump_get_num_partial_associations(clump: GPtr<u8>) -> u32 {
    count_links(clump, is_partial)
}

/// `RpAnimBlend::RpAnimBlendClumpGetNumNonPartialAssociations(clump)` (0x4D6BB0).
#[hook(addr = 0x4D6BB0, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpGetNumNonPartialAssociations", expect = "8B 44 24 04 8B 0D 78 F8 B5 00 8B 0C 01 8B 09 33 C0 85 C9 74 12 8A 51 2A C0 EA 04 F6 C2 01 75",
       unverified = "std6-anim-1", route = "L1-pure", hazards = "none")]
pub fn rp_anim_blend_clump_get_num_non_partial_associations(clump: GPtr<u8>) -> u32 {
    count_links(clump, |l| !is_partial(l))
}

/// `RpAnimBlend::RpAnimBlendClumpExtractAssociations(clump)` (0x4D6BE0): the list is detached (head
/// and the first link's prev cleared, no NULL check) and its first association returned.
#[hook(addr = 0x4D6BE0, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpExtractAssociations", expect = "8B 0D 78 F8 B5 00 8B 44 24 04 8B 04 01 8B 08 C7 00",
       unverified = "std6-anim-1", route = "L2", hazards = "none")]
pub fn rp_anim_blend_clump_extract_associations(clump: GPtr<u8>) -> u32 {
    let d = clump_data(clump);
    let first = p8(rd::<u32>(d, 0));
    wr::<u32>(d, 0, 0);
    wr::<u32>(first, 4, 0);
    a32(first).wrapping_sub(4)
}

/// `RpAnimBlend::RpAnimBlendClumpRemoveAllAssociations(clump)` (0x4D6C00): every association deleted.
#[hook(addr = 0x4D6C00, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpRemoveAllAssociations", expect = "8B 44 24 04 8B 0D 78 F8 B5 00 8B 04 01 8B 00 85 C0 74 17",
       unverified = "std6-anim-1", route = "L2", hazards = "heap-writes")]
pub fn rp_anim_blend_clump_remove_all_associations(clump: GPtr<u8>) {
    let mut l = first_link(clump);
    while !l.is_null() {
        let nx = next(l);
        let a = assoc_of(l);
        if !a.is_null() {
            assoc_vdtor(a, 1);
        }
        l = nx;
    }
}

/// `RpAnimBlend::RpAnimBlendClumpGiveAssociations(clump, assoc)` (0x4D6C30): the clump's associations
/// deleted, then `assoc`'s list attached.
#[hook(addr = 0x4D6C30, conv = cdecl, name = "RpAnimBlend::RpAnimBlendClumpGiveAssociations", expect = "8B 44 24 04 8B 0D 78 F8 B5 00 57",
       unverified = "std6-anim-1", route = "L2", hazards = "heap-writes")]
pub fn rp_anim_blend_clump_give_associations(clump: GPtr<u8>, assoc: GPtr<u8>) {
    let d = clump_data(clump);
    let mut l = p8(rd::<u32>(d, 0));
    while !l.is_null() {
        let nx = next(l);
        let a = assoc_of(l);
        if !a.is_null() {
            assoc_vdtor(a, 1);
        }
        l = nx;
    }
    wr::<u32>(d, 0, a32(assoc) + 4);
    wr::<u32>(assoc, 8, a32(d));
}

orig! {
    /// `FrameUpdateCallBackSkinnedWithVelocityExtraction(AnimBlendFrameData* fd)` (0x4D1680): EAX = the
    /// update data, `fd` on the stack (caller pops)
    fn frame_update_skinned_with_velocity_extraction(data: u32, fd: GPtr<u8>) = reg("args = eax stack") 0x4D1680 => unported;
}

/// `gpAnimBlendClump` (`CAnimBlendClumpData*`, set by `RpAnimBlendClumpUpdateAnimations`).
const ANIM_BLEND_CLUMP: usize = 0xB4EA0C;

/// `FrameUpdateCallBackOffscreen(AnimBlendFrameData* fd, void* data)` (0x4D2E10, the off-screen
/// `ForAllFrames` callback of `RpAnimBlendClumpUpdateAnimations`): only for a frame with flag 8
/// (velocity extraction) while `gpAnimBlendClump->m_PedPosition` (+0xC) is set, it calls
/// 0x4D1680 with EAX = `data` and `fd` pushed; otherwise nothing.
#[hook(addr = 0x4D2E10, conv = cdecl, name = "FrameUpdateCallBackOffscreen", expect = "8B 44 24 04 F6 00 08",
       unverified = "regconv-anim-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "calls the EAX + stack register-convention 0x4D1680 through orig! reg (HARNESS-5)")]
pub fn frame_update_callback_offscreen(fd: GPtr<u8>, data: u32) {
    if fd.read() & 8 == 0 {
        return;
    }
    let clump = g::<GPtr<u8>>(ANIM_BLEND_CLUMP).read();
    if rd::<u32>(clump, 0xC) == 0 {
        return;
    }
    frame_update_skinned_with_velocity_extraction(data, fd);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_FRAME_UPDATE_CALLBACK_OFFSCREEN,
    HOOK_RP_ANIM_BLEND_FRAME_SET_NAME,
    HOOK_SET_FRAME_NODE_NAME,
    HOOK_CLUMP_DATA_DELETING_DTOR,
    HOOK_RP_ANIM_BLEND_ALLOCATE_DATA,
    HOOK_RT_ANIM_BLEND_KEY_FRAME_APPLY,
    HOOK_RT_ANIM_ANIMATION_CREATE,
    HOOK_RP_ANIM_BLEND_CREATE_ANIMATION_FOR_HIERARCHY,
    HOOK_FIND_FRAME_BY_BONE_NAME_CB,
    HOOK_FIND_FRAME_BY_NODE_NAME_CB,
    HOOK_RP_ANIM_BLEND_CLUMP_FIND_FRAME,
    HOOK_RP_ANIM_BLEND_CLUMP_FIND_FRAME_FROM_HASH_KEY,
    HOOK_FIND_BONE_CB,
    HOOK_RP_ANIM_BLEND_CLUMP_FIND_BONE,
    HOOK_FILL_FRAME_ARRAY_CB,
    HOOK_RP_ANIM_BLEND_CLUMP_FILL_FRAME_ARRAY,
    HOOK_CLEAR_FRAME_FLAGS_CB,
    HOOK_SKIN_GET_BONE_POSITIONS_TO_TABLE,
    HOOK_COUNT_FRAMES_CB,
    HOOK_INIT_FRAME_NOT_SKINNED_CB,
    HOOK_BIND_FRAMES_CB,
    HOOK_RP_ANIM_BLEND_CLUMP_INIT,
    HOOK_RP_ANIM_BLEND_CLUMP_ADD_ASSOCIATION,
    HOOK_RP_ANIM_BLEND_CLUMP_SET_BLEND_DELTAS,
    HOOK_RP_ANIM_BLEND_CLUMP_REMOVE_ASSOCIATIONS,
    HOOK_RP_ANIM_BLEND_CLUMP_GET_ASSOCIATION_NAME,
    HOOK_RP_ANIM_BLEND_CLUMP_GET_ASSOCIATION_ID,
    HOOK_RP_ANIM_BLEND_CLUMP_GET_ASSOCIATION_HIER,
    HOOK_RP_ANIM_BLEND_CLUMP_GET_MAIN_ASSOCIATION,
    HOOK_RP_ANIM_BLEND_CLUMP_GET_MAIN_PARTIAL_ASSOCIATION,
    HOOK_RP_ANIM_BLEND_CLUMP_GET_MAIN_PARTIAL_ASSOCIATION_N,
    HOOK_RP_ANIM_BLEND_CLUMP_GET_MAIN_ASSOCIATION_N,
    HOOK_RP_ANIM_BLEND_CLUMP_GET_FIRST_ASSOCIATION_FLAGS,
    HOOK_RP_ANIM_BLEND_GET_NEXT_ASSOCIATION,
    HOOK_RP_ANIM_BLEND_GET_NEXT_ASSOCIATION_FLAGS,
    HOOK_RP_ANIM_BLEND_CLUMP_PAUSE_ALL_ANIMATIONS,
    HOOK_RP_ANIM_BLEND_CLUMP_UNPAUSE_ALL_ANIMATIONS,
    HOOK_RP_ANIM_BLEND_CLUMP_GET_NUM_ASSOCIATIONS,
    HOOK_RP_ANIM_BLEND_CLUMP_GET_NUM_PARTIAL_ASSOCIATIONS,
    HOOK_RP_ANIM_BLEND_CLUMP_GET_NUM_NON_PARTIAL_ASSOCIATIONS,
    HOOK_RP_ANIM_BLEND_CLUMP_EXTRACT_ASSOCIATIONS,
    HOOK_RP_ANIM_BLEND_CLUMP_REMOVE_ALL_ASSOCIATIONS,
    HOOK_RP_ANIM_BLEND_CLUMP_GIVE_ASSOCIATIONS,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] =
    &[NATIVE_RP_ANIM_BLEND_CLUMP_GET_FIRST_ASSOCIATION, NATIVE_RP_ANIM_BLEND_CLUMP_IS_INITIALIZED];
