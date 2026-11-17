//! RtAnim toolkit, RpHAnim and RpUVAnim plugin attaches with their engine-module and object
//! callbacks, and `RtDictSchemaInit` / `Destruct` (PORT-RW-1 follow-up, written for the native boot).
//! Ported from the 1.0 US bytes.
//!
//! Interpolation-scheme table: 16 `RtAnimInterpolatorInfo` (0x30 bytes, +0 typeID) at 0xC9B920, count
//! at 0xC97900. RpHAnim frame extension (`frame + [0xC9B8C8]`, 8 bytes): +0 id, +4 hierarchy.
//! RpUVAnim material extension (`material + [0xC9B8D0]`, 0x28 bytes): +0/+4 two matrices, +8 eight
//! interpolators.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{RwError, p8, rd, rw_error1, rw_error_set, rw_free, rw_memory_free, wr};

sa_core::globals! {
    list ANIM_GLOBALS;
    /// Number of registered interpolation schemes
    pub static ANIM_SCHEME_COUNT @ 0xC97900: i32 = 0;
    /// The 16 registered `RtAnimInterpolatorInfo` (0x30 bytes each)
    pub static ANIM_SCHEMES @ 0xC9B920: [u32; 16 * 12] = [0; 16 * 12];
    /// The animation toolkit's freelist (0x18-byte entries)
    pub static ANIM_FREELIST @ 0xC9BC20: u32 = 0;
    /// Static space of the animation freelist (0x24 bytes)
    pub static ANIM_FL_SPACE @ 0xC9B8E0: [u32; 9] = [0; 9];
    /// Animation freelist entries per block (0x80)
    pub static ANIM_FL_PER_BLOCK @ 0x8DEDEC: u32 = 0x80;
    /// Animation freelist preallocated blocks (1)
    pub static ANIM_FL_PREALLOC @ 0x8DEDF0: u32 = 1;
    /// RwFrame plugin offset of the RpHAnim extension
    pub static HANIM_FRAME_OFFSET @ 0xC9B8C8: i32 = 0;
    /// RpHAnim hierarchy freelist (0x24-byte entries)
    pub static HANIM_FREELIST @ 0xC9B8CC: u32 = 0;
    /// Static space of the hierarchy freelist (0x24 bytes)
    pub static HANIM_FL_SPACE @ 0xC97054: [u32; 9] = [0; 9];
    /// Hierarchy freelist entries per block (0x80)
    pub static HANIM_FL_PER_BLOCK @ 0x8DECD8: u32 = 0x80;
    /// Hierarchy freelist preallocated blocks (1)
    pub static HANIM_FL_PREALLOC @ 0x8DECDC: u32 = 1;
    /// RpMaterial plugin offset of the RpUVAnim extension
    pub static UVANIM_MATERIAL_OFFSET @ 0xC9B8D0: i32 = 0;
    /// RpUVAnim animation freelist (0x44-byte entries); its old value is also passed as the
    /// freelist's static space
    pub static UVANIM_FREELIST @ 0xC978F4: u32 = 0;
    /// RpUVAnim open-instance counter
    pub static UVANIM_INSTANCES @ 0xC978FC: i32 = 0;
    /// UV animation freelist entries per block (0x80)
    pub static UVANIM_FL_PER_BLOCK @ 0x8DED48: u32 = 0x80;
    /// UV animation freelist preallocated blocks (1)
    pub static UVANIM_FL_PREALLOC @ 0x8DED4C: u32 = 1;
}

/// The RpUVAnim dictionary schema (`RtDictSchema`, 0x8DED50) and its two interpolation schemes.
const UV_DICT_SCHEMA: u32 = 0x8DED50;
const UV_SCHEME_A: u32 = 0x8DED84;
const UV_SCHEME_B: u32 = 0x8DEDB4;

use crate::bindings::free_list_create_prealloc;
use crate::bindings::animation_destroy;
use crate::bindings::free_list_destroy;
use crate::bindings::hanim_hierarchy_create;
use crate::bindings::interpolator_destroy;
use crate::bindings::rw_engine_register_plugin as engine_register_plugin;
use crate::bindings::frame_register_plugin;
use crate::bindings::frame_register_plugin_stream;
use crate::bindings::register_scheme_call;
use crate::bindings::rp_material_register_plugin as material_register_plugin;
use crate::bindings::rp_material_register_plugin_stream as material_register_plugin_stream;
use crate::bindings::rw_matrix_destroy as matrix_destroy;
use sa_core::bindings::animation_create;
use crate::bindings::interpolator_set_current_anim;
orig! {
    /// `_rwSListCreate(entrySize, hint)` (0x809160)
    pub fn slist_create(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x809160 => crate::rxpipeline::rw_slist_create;
    /// `_rwSListDestroy(list)` (0x809440)
    pub fn slist_destroy(s: GPtr<u8>) -> i32 = cdecl 0x809440 => crate::rxpipeline::rw_slist_destroy;
    /// `RtDictSchemaInit(schema)` (0x7CED40)
    fn dict_schema_init_call(schema: GPtr<u8>) -> GPtr<u8> = cdecl 0x7CED40 => rt_dict_schema_init;
    /// `_rpUVAnimCustomDataStreamRead(stream)` (0x7CBF70)
    fn custom_data_stream_read_call(stream: GPtr<u8>) -> GPtr<u8> = cdecl 0x7CBF70 => uv_anim_custom_data_stream_read;
    /// `RtDictSchemaGetCurrentDict(schema)` (0x7CEEE0)
    fn schema_get_current_dict(schema: GPtr<u8>) -> GPtr<u8> = cdecl 0x7CEEE0 => unported;
    /// `RtDictFindNamedEntry(dict, name)` (0x7CEFE0)
    fn dict_find_named_entry(dict: GPtr<u8>, name: GPtr<u8>) -> u32 = cdecl 0x7CEFE0 => crate::frontier_b::rt_dict_find_named_entry;
    /// `RtAnimAnimationGetNumNodes(anim)` (0x7CD4F0)
    fn animation_get_num_nodes(anim: GPtr<u8>) -> i32 = cdecl 0x7CD4F0 => crate::hanim_stream::rt_anim_animation_get_num_nodes;
    /// The UV key frame set `(anim, frame, prev, time, matrix)` (0x7CC810)
    fn key_frame_set(anim: u32, kf: u32, prev: u32, time: u32, m: u32) -> u32 = cdecl 0x7CC810 => unported;
    /// The UV material stream read's dummy animation (0x7CBE00)
    fn uv_anim_create_dummy_call(name: GPtr<u8>) -> GPtr<u8> = cdecl 0x7CBE00 => uv_anim_create_dummy;
    /// The material extension's UV matrices update (0x7CBD80)
    fn uv_matrices_update_call(ext: GPtr<u8>) -> GPtr<u8> = cdecl 0x7CBD80 => uv_matrices_update;
    /// `RtDictSchemaDestruct(schema)` (0x7CED70)
    fn dict_schema_destruct_call(schema: GPtr<u8>) -> i32 = cdecl 0x7CED70 => rt_dict_schema_destruct;
}

use sa_core::field::addr32 as a32;

/// `RwErrorSet({0x1B7, _rwerror(code)})` (the RtAnim toolkit's plugin id).
pub(crate) fn anim_error(code: u32) {
    let mut e = RwError {
        plugin_id: 0x1B7,
        error_code: 0,
    };
    e.error_code = rw_error1(code) as i32;
    rw_error_set(GPtr::from_raw(&mut e).cast());
}

/// `RtAnimInitialize()` (0x7CCCA0): registers the ANIMTOOLKIT engine module (0x1B7, size 0); returns
/// `offset > 0` (quirk kept: an offset of 0 counts as failure).
#[hook(addr = 0x7CCCA0, conv = cdecl, name = "RtAnimInitialize", expect = "68 10 CD 7C 00")]
pub fn rt_anim_initialize() -> i32 {
    (engine_register_plugin(0, 0x1B7, 0x7CCCD0, 0x7CCD10) > 0) as i32
}

/// `cb_ANIMTOOLKIT_RwEngine_Open(instance, offset, size)` (0x7CCCD0): the animation freelist
/// (0x18-byte entries, hint 0x4001B, space 0xC9B8E0) into 0xC9BC20; NULL on failure, else `instance`.
#[hook(addr = 0x7CCCD0, conv = cdecl, name = "cb_ANIMTOOLKIT_RwEngine_Open", expect = "A1 F0 ED 8D 00")]
pub fn anim_toolkit_open(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    let fl = free_list_create_prealloc(
        0x18,
        ANIM_FL_PER_BLOCK.get(),
        4,
        ANIM_FL_PREALLOC.get(),
        ANIM_FL_SPACE.ptr().cast(),
        0x4001B,
    );
    ANIM_FREELIST.set(a32(fl));
    if fl.is_null() {
        return GPtr::NULL;
    }
    instance
}

/// `cb_ANIMTOOLKIT_RwEngine_Close(instance, offset, size)` (0x7CCD10): forgets every registered
/// interpolation scheme (count = 0), destroys the animation freelist when present (and clears it).
/// Returns `instance`.
#[hook(addr = 0x7CCD10, conv = cdecl, name = "cb_ANIMTOOLKIT_RwEngine_Close", expect = "A1 20 BC C9 00")]
pub fn anim_toolkit_close(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    let fl = ANIM_FREELIST.get();
    ANIM_SCHEME_COUNT.set(0);
    if fl != 0 {
        free_list_destroy(p8(fl));
        ANIM_FREELIST.set(0);
    }
    instance
}

/// `RtAnimRegisterInterpolationScheme(info)` (0x7CCD40): with 16 schemes registered reports
/// `{0x1B7, _rwerror(1)}`; a scheme whose typeID is already registered reports `{0x1B7,
/// _rwerror(0)}`; both return FALSE. Otherwise the 0x30-byte info is copied to the next slot and
/// counted. Returns TRUE.
#[hook(addr = 0x7CCD40, conv = cdecl, name = "RtAnimRegisterInterpolationScheme", expect = "A1 00 79 C9 00")]
pub fn rt_anim_register_interpolation_scheme(info: GPtr<u8>) -> i32 {
    let n = ANIM_SCHEME_COUNT.get();
    if n >= 0x10 {
        anim_error(1);
        return 0;
    }
    let table: GPtr<u8> = ANIM_SCHEMES.ptr().cast();
    let id = rd::<u32>(info, 0);
    let mut i = 0;
    while i < n {
        if rd::<u32>(table, (i as isize) * 0x30) == id {
            anim_error(0);
            return 0;
        }
        i += 1;
    }
    let dst = table.byte_add((n as isize) * 0x30);
    for k in 0..12isize {
        wr::<u32>(dst, k * 4, rd::<u32>(info, k * 4));
    }
    ANIM_SCHEME_COUNT.set(n + 1);
    1
}

/// `RpHAnimPluginAttach()` (0x7C4600): the HANIMPLUGIN engine module (0x11E, size 0; negative
/// returns FALSE), the 8-byte RwFrame extension (offset in 0xC9B8C8) and its stream callbacks.
/// Returns FALSE when the stream registration or the stored offset is negative, else TRUE.
#[hook(addr = 0x7C4600, conv = cdecl, name = "RpHAnimPluginAttach", expect = "68 20 47 7C 00")]
pub fn rp_hanim_plugin_attach() -> i32 {
    if engine_register_plugin(0, 0x11E, 0x7C4670, 0x7C4720) < 0 {
        return 0;
    }
    HANIM_FRAME_OFFSET.set(frame_register_plugin(8, 0x11E, 0x7C4750, 0x7C4770, 0x7C4830));
    if frame_register_plugin_stream(0x11E, 0x7C4A00, 0x7C48D0, 0x7C4BF0) < 0 {
        return 0;
    }
    if HANIM_FRAME_OFFSET.get() < 0 {
        return 0;
    }
    1
}

/// `cb_HANIMPLUGIN_RwEngine_Open(instance, offset, size)` (0x7C4670): the hierarchy freelist (0x24-
/// byte entries, hint 0x4011E, space 0xC97054) into 0xC9B8CC, then registers the standard keyframe
/// scheme (typeID 1, key size 0x24, callbacks 0x7C5B80, 0x7C60C0, 0x7C5CA0, 0x7C6720, 0x7C65C0,
/// 0x7C64C0, 0x7C6540, 0x7C65B0, custom size 0) built on the stack, whatever the freelist result.
/// Returns `instance`, or NULL when the freelist failed.
#[hook(addr = 0x7C4670, conv = cdecl, name = "cb_HANIMPLUGIN_RwEngine_Open", expect = "A1 DC EC 8D 00")]
pub fn hanim_open(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    let fl = free_list_create_prealloc(
        0x24,
        HANIM_FL_PER_BLOCK.get(),
        4,
        HANIM_FL_PREALLOC.get(),
        HANIM_FL_SPACE.ptr().cast(),
        0x4011E,
    );
    HANIM_FREELIST.set(a32(fl));
    let r = if fl.is_null() { GPtr::NULL } else { instance };
    let mut info: [u32; 12] = [
        1, 0x24, 0x24, 0x7C5B80, 0x7C60C0, 0x7C5CA0, 0x7C6720, 0x7C65C0, 0x7C64C0, 0x7C6540, 0x7C65B0, 0,
    ];
    register_scheme_call(GPtr::from_raw(&mut info).cast());
    r
}

/// `cb_HANIMPLUGIN_RwEngine_Close(instance, offset, size)` (0x7C4720): destroys the hierarchy freelist
/// when present (and clears it). Returns `instance`.
#[hook(addr = 0x7C4720, conv = cdecl, name = "cb_HANIMPLUGIN_RwEngine_Close", expect = "A1 CC B8 C9 00")]
pub fn hanim_close(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    let fl = HANIM_FREELIST.get();
    if fl != 0 {
        free_list_destroy(p8(fl));
        HANIM_FREELIST.set(0);
    }
    instance
}

/// `frame + [0xC9B8C8]`.
fn hanim_ext(frame: GPtr<u8>) -> GPtr<u8> {
    frame.byte_add(HANIM_FRAME_OFFSET.get() as isize)
}

/// `cb_HANIMPLUGIN_RwFrame_Ctor(frame, offset, size)` (0x7C4750): hierarchy NULL, id -1. Returns
/// `frame`.
#[hook(addr = 0x7C4750, conv = cdecl, name = "cb_HANIMPLUGIN_RwFrame_Ctor", expect = "8B 44 24 04 8B 0D C8 B8 C9 00")]
pub fn hanim_frame_ctor(frame: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let e = hanim_ext(frame);
    wr::<u32>(e, 4, 0);
    wr::<u32>(e, 0, 0xFFFF_FFFF);
    frame
}

/// `cb_HANIMPLUGIN_RwFrame_Dtor(frame, offset, size)` (0x7C4770): with a hierarchy, clears every
/// node's frame pointer (node +0xC, 0x10-byte nodes; count re-read each step, signed); when this frame
/// is the hierarchy's parent (+0x14): unless flag 1 (sub-hierarchy) frees the matrix array (+0xC,
/// skipped with flag 2) and the node array (+0x10), clears +0xC, +8, +0x10, destroys the current
/// interpolator (+0x20), returns the hierarchy to its freelist and clears the frame's hierarchy word.
/// Then hierarchy NULL and id -1. Returns `frame`.
#[hook(addr = 0x7C4770, conv = cdecl, name = "cb_HANIMPLUGIN_RwFrame_Dtor", expect = "A1 C8 B8 C9 00")]
pub fn hanim_frame_dtor(frame: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let e = hanim_ext(frame);
    let h = p8(rd::<u32>(e, 4));
    if !h.is_null() {
        if rd::<i32>(h, 4) > 0 {
            let mut i: i32 = 0;
            loop {
                let nodes = p8(rd::<u32>(h, 0x10));
                wr::<u32>(nodes, (i as isize) * 0x10 + 0xC, 0);
                i += 1;
                if i >= rd::<i32>(h, 4) {
                    break;
                }
            }
        }
        let parent = rd::<u32>(h, 0x14);
        if parent == a32(frame) {
            let flags = rd::<u32>(h, 0);
            if flags & 1 == 0 {
                if flags & 2 == 0 {
                    rw_free(p8(rd::<u32>(h, 0xC)));
                }
                rw_free(p8(rd::<u32>(h, 0x10)));
            }
            let interp = rd::<u32>(h, 0x20);
            wr::<u32>(h, 0xC, 0);
            wr::<u32>(h, 8, 0);
            wr::<u32>(h, 0x10, 0);
            interpolator_destroy(p8(interp));
            rw_memory_free(p8(HANIM_FREELIST.get()), h);
            if parent != 0 {
                wr::<u32>(hanim_ext(p8(parent)), 4, 0);
            }
        }
        wr::<u32>(e, 4, 0);
    }
    wr::<u32>(e, 0, 0xFFFF_FFFF);
    frame
}

/// `cb_HANIMPLUGIN_RwFrame_Copy(dst, src, offset, size)` (0x7C4830): copies the id; a source
/// hierarchy without flag 1 is duplicated with `RpHAnimHierarchyCreate(numNodes, NULL, NULL, flags,
/// keyFrameSize of its interpolator)` (no NULL check, kept), every node's frame cleared and its words
/// +8, +4, +0 copied, the copy attached to `dst` (its parent = dst). Returns `dst`.
#[hook(addr = 0x7C4830, conv = cdecl, name = "cb_HANIMPLUGIN_RwFrame_Copy", expect = "8B 0D C8 B8 C9 00")]
pub fn hanim_frame_copy(dst: GPtr<u8>, src: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let s = hanim_ext(src);
    let d = hanim_ext(dst);
    wr::<u32>(d, 0, rd::<u32>(s, 0));
    let sh = p8(rd::<u32>(s, 4));
    if sh.is_null() {
        return dst;
    }
    let flags = rd::<u32>(sh, 0);
    if flags & 1 != 0 {
        return dst;
    }
    let key = rd::<i32>(p8(rd::<u32>(sh, 0x20)), 0x20);
    let h = hanim_hierarchy_create(rd::<i32>(sh, 4), GPtr::NULL, GPtr::NULL, flags, key);
    let mut i: i32 = 0;
    if rd::<i32>(h, 4) > 0 {
        loop {
            let o = (i as isize) * 0x10;
            i += 1;
            wr::<u32>(p8(rd::<u32>(h, 0x10)), o + 0xC, 0);
            for f in [8, 4, 0] {
                let v = rd::<u32>(p8(rd::<u32>(sh, 0x10)), o + f);
                wr::<u32>(p8(rd::<u32>(h, 0x10)), o + f, v);
            }
            if i >= rd::<i32>(h, 4) {
                break;
            }
        }
    }
    wr::<u32>(d, 4, a32(h));
    wr::<u32>(h, 0x14, a32(dst));
    dst
}

/// `RpUVAnimPluginAttach()` (0x7CB940): the UVANIMPLUGIN engine module (0x135, size 0; negative
/// returns FALSE), the 0x28-byte RpMaterial extension (offset in 0xC9B8D0) and its stream callbacks.
/// FALSE when the stream registration or the stored offset is negative, else TRUE.
#[hook(addr = 0x7CB940, conv = cdecl, name = "RpUVAnimPluginAttach", expect = "68 20 BA 7C 00")]
pub fn rp_uvanim_plugin_attach() -> i32 {
    if engine_register_plugin(0, 0x135, 0x7CB9B0, 0x7CBA20) < 0 {
        return 0;
    }
    UVANIM_MATERIAL_OFFSET.set(material_register_plugin(0x28, 0x135, 0x7CBA80, 0x7CBAA0, 0x7CBB30));
    if material_register_plugin_stream(0x135, 0x7CBC20, 0x7CBB70, 0x7CBF20) < 0 {
        return 0;
    }
    if UVANIM_MATERIAL_OFFSET.get() < 0 {
        return 0;
    }
    1
}

/// `cb_UVANIMPLUGIN_RwEngine_Open(instance, offset, size)` (0x7CB9B0): the UV animation freelist
/// (0x44-byte entries, hint 0x40135); quirk kept: the static-space argument is the old value of the
/// freelist global 0xC978F4 itself (NULL on a first open). Then, whatever the result,
/// `RtDictSchemaInit(0x8DED50)` and the two UV interpolation schemes (0x8DED84, 0x8DEDB4) are
/// registered (results ignored) and the instance is counted. Returns `instance`, or NULL when the
/// freelist failed.
#[hook(addr = 0x7CB9B0, conv = cdecl, name = "cb_UVANIMPLUGIN_RwEngine_Open", expect = "A1 F4 78 C9 00")]
pub fn uvanim_open(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    let space = UVANIM_FREELIST.get();
    let n = UVANIM_FL_PREALLOC.get();
    let per_block = UVANIM_FL_PER_BLOCK.get();
    let fl = free_list_create_prealloc(0x44, per_block, 4, n, p8(space), 0x40135);
    UVANIM_FREELIST.set(a32(fl));
    let r = if fl.is_null() { GPtr::NULL } else { instance };
    dict_schema_init_call(p8(UV_DICT_SCHEMA));
    register_scheme_call(p8(UV_SCHEME_A));
    register_scheme_call(p8(UV_SCHEME_B));
    UVANIM_INSTANCES.set(UVANIM_INSTANCES.get().wrapping_add(1));
    r
}

/// `cb_UVANIMPLUGIN_RwEngine_Close(instance, offset, size)` (0x7CBA20): decrements the count, destructs
/// the UV dictionary schema, destroys the UV animation freelist when present (and clears it). Returns
/// `instance`.
#[hook(addr = 0x7CBA20, conv = cdecl, name = "cb_UVANIMPLUGIN_RwEngine_Close", expect = "8B 0D FC 78 C9 00")]
pub fn uvanim_close(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    UVANIM_INSTANCES.set(UVANIM_INSTANCES.get().wrapping_sub(1));
    dict_schema_destruct_call(p8(UV_DICT_SCHEMA));
    let fl = UVANIM_FREELIST.get();
    if fl != 0 {
        free_list_destroy(p8(fl));
        UVANIM_FREELIST.set(0);
    }
    instance
}

/// `material + [0xC9B8D0]`.
fn uv_ext(m: GPtr<u8>) -> GPtr<u8> {
    m.byte_add(UVANIM_MATERIAL_OFFSET.get() as isize)
}

/// `cb_UVANIMPLUGIN_RpMaterial_Ctor(material, offset, size)` (0x7CBA80): clears the 10 words of the
/// extension. Returns `material`.
#[hook(addr = 0x7CBA80, conv = cdecl, name = "cb_UVANIMPLUGIN_RpMaterial_Ctor", expect = "8B 54 24 04 56")]
pub fn uvanim_material_ctor(m: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let e = uv_ext(m);
    for k in 0..10isize {
        wr::<u32>(e, k * 4, 0);
    }
    m
}

/// `cb_UVANIMPLUGIN_RpMaterial_Dtor(material, offset, size)` (0x7CBAA0): destroys and clears the two
/// matrices; for each of the 8 interpolators: decrements the reference count (+0x40) of its current
/// animation's UV data (`anim+0x14`), and at zero returns that data to the UV freelist and destroys the
/// animation; then destroys the interpolator (the slot is not cleared). Returns `material`.
#[hook(addr = 0x7CBAA0, conv = cdecl, name = "cb_UVANIMPLUGIN_RpMaterial_Dtor", expect = "A1 D0 B8 C9 00")]
pub fn uvanim_material_dtor(m: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let e = uv_ext(m);
    for k in 0..2isize {
        let mx = rd::<u32>(e, k * 4);
        if mx != 0 {
            matrix_destroy(p8(mx));
        }
        wr::<u32>(e, k * 4, 0);
    }
    for k in 0..8isize {
        let interp = p8(rd::<u32>(e, 8 + k * 4));
        if interp.is_null() {
            continue;
        }
        uv_anim_destroy(p8(rd::<u32>(interp, 0)));
        interpolator_destroy(interp);
    }
    m
}

/// `cb_UVANIMPLUGIN_RpMaterial_Copy(dst, src, offset, size)` (0x7CBB30): copies the 10 extension words
/// and increments the reference count of each copied interpolator's current animation's UV data.
/// Returns `dst`.
#[hook(addr = 0x7CBB30, conv = cdecl, name = "cb_UVANIMPLUGIN_RpMaterial_Copy", expect = "8B 0D D0 B8 C9 00")]
pub fn uvanim_material_copy(dst: GPtr<u8>, src: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let off = UVANIM_MATERIAL_OFFSET.get() as isize;
    let d = dst.byte_add(off);
    let s = src.byte_add(off);
    for k in 0..10isize {
        wr::<u32>(d, k * 4, rd::<u32>(s, k * 4));
    }
    for k in 0..8isize {
        let interp = rd::<u32>(d, 8 + k * 4);
        if interp != 0 {
            let anim = p8(rd::<u32>(p8(interp), 0));
            let data = p8(rd::<u32>(anim, 0x14));
            wr::<i32>(data, 0x40, rd::<i32>(data, 0x40).wrapping_add(1));
        }
    }
    dst
}

/// `RtDictSchemaInit(schema)` (0x7CED40): the dictionary list `_rwSListCreate(4, schema+4 | 0x40000)`
/// into `schema+0x10` (NULL returns NULL), current dictionary (+0x14) NULL. Returns `schema`.
#[hook(addr = 0x7CED40, conv = cdecl, name = "RtDictSchemaInit", expect = "56 8B 74 24 08")]
pub fn rt_dict_schema_init(schema: GPtr<u8>) -> GPtr<u8> {
    let l = slist_create(4, rd::<u32>(schema, 4) | 0x40000);
    wr::<u32>(schema, 0x10, a32(l));
    if l.is_null() {
        return GPtr::NULL;
    }
    wr::<u32>(schema, 0x14, 0);
    schema
}

/// `RtDictSchemaDestruct(schema)` (0x7CED70): `_rwSListDestroy(schema+0x10)`; returns TRUE.
#[hook(addr = 0x7CED70, conv = cdecl, name = "RtDictSchemaDestruct", expect = "8B 44 24 04 8B 48 10")]
pub fn rt_dict_schema_destruct(schema: GPtr<u8>) -> i32 {
    slist_destroy(p8(rd::<u32>(schema, 0x10)));
    1
}

/// Every `#[hook]` of this module.
/// `RpUVAnimDestroy(RtAnimAnimation*)` (0x7CC0C0; inlined in the material destructor above): the custom data's
/// (+0x14) reference count (+0x40) drops; at 0 the custom data goes back to the freelist `[0xC978F4]` and the
/// animation is destroyed (`RtAnimAnimationDestroy`). Returns 1.
#[hook(addr = 0x7CC0C0, conv = cdecl, name = "RpUVAnimDestroy", expect = "56 8B 74 24 08 8B 46 14",
       unverified = "frontier2-c17", route = "L2", hazards = "heap-writes")]
pub fn uv_anim_destroy(anim: GPtr<u8>) -> u32 {
    let data = p8(rd::<u32>(anim, 0x14));
    let rc = rd::<i32>(data, 0x40).wrapping_sub(1);
    wr::<i32>(data, 0x40, rc);
    if rc == 0 {
        rw_memory_free(p8(UVANIM_FREELIST.get()), p8(rd::<u32>(anim, 0x14)));
        animation_destroy(anim);
    }
    1
}

/// `cb_UVANIMPLUGIN_RpMaterial_StreamRead(stream, len, material, offset, size)` (0x7CBC20): the struct chunk
/// (type 1), a mask of the eight interpolator slots (int32, read into the material argument's slot; the extension
/// pointer was taken before). For each set bit: the animation's 0x20-byte name (`RwStreamRead`), the animation
/// found by name in the UV dictionary schema's current dictionary (`RtDictFindNamedEntry`; its custom data's
/// reference count +1) or, without a dictionary or a match, a dummy one (0x7CBE00); a null animation or a null
/// stream gives 0. The slot's interpolator: `RtAnimInterpolatorCreate(RtAnimAnimationGetNumNodes(anim), scheme
/// +4)` with the animation set current; either failing releases the animation (`RpUVAnimDestroy`, inlined; the
/// interpolator is kept) and gives 0. Then the two UV matrices (0x7CBD80): the stream when they are made, else 0.
#[hook(addr = 0x7CBC20, conv = cdecl, name = "cb_UVANIMPLUGIN_RpMaterial_StreamRead", expect = "83 EC 24 8B 44 24 30",
       unverified = "frontier2-c17", route = "L2", hazards = "heap-writes")]
pub fn uvanim_material_stream_read(stream: GPtr<u8>, _len: i32, m: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let ext = uv_ext(m);
    if sa_core::bindings::rw_stream_find_chunk_call(stream, 1, GPtr::NULL, GPtr::NULL) == 0 {
        return GPtr::NULL;
    }
    let mut mask = 0u32;
    if crate::bindings::read_int32(stream, GPtr::from_raw(&mut mask).cast(), 4).is_null() {
        return GPtr::NULL;
    }
    for i in 0..8u32 {
        if mask & (1 << i) == 0 {
            continue;
        }
        let mut name = [0u8; 0x20];
        let name_p = GPtr::from_raw(name.as_mut_ptr());
        if crate::bindings::stream_read(stream, name_p, 0x20) == 0 {
            return GPtr::NULL;
        }
        let dict = schema_get_current_dict(p8(UV_DICT_SCHEMA));
        let mut anim = GPtr::NULL;
        if !dict.is_null() {
            anim = p8(dict_find_named_entry(dict, name_p));
            if !anim.is_null() {
                let data = p8(rd::<u32>(anim, 0x14));
                wr::<i32>(data, 0x40, rd::<i32>(data, 0x40).wrapping_add(1));
            }
        }
        if anim.is_null() {
            anim = uv_anim_create_dummy_call(name_p);
        }
        if anim.is_null() || stream.is_null() {
            return GPtr::NULL;
        }
        let kfs: i32 = rd(rd::<GPtr<u8>>(anim, 0), 4);
        let interp = crate::bindings::interpolator_create(animation_get_num_nodes(anim), kfs);
        wr::<u32>(ext, 8 + i as isize * 4, interp.addr() as u32);
        if interp.is_null() || interpolator_set_current_anim(interp, anim) == 0 {
            uv_anim_destroy(anim);
            return GPtr::NULL;
        }
    }
    if uv_matrices_update_call(ext).is_null() {
        GPtr::NULL
    } else {
        stream
    }
}

/// The UV matrices of a material extension (0x7CBD80): for each of the two (+0, +4), whether a node of the eight
/// interpolators (+8; the first node is always tested, then up to the node count +0x2C) maps to it (the current
/// animation's custom data, +0x20 + node * 4, equal to the matrix index): a used matrix is made when missing
/// (`RwMatrixCreate`; null → 0), an unused one destroyed (`RwMatrixDestroy`) and cleared. Returns the extension.
#[hook(addr = 0x7CBD80, conv = cdecl, name = "FUN_007cbd80", expect = "53 55 56 57 8B 7C 24 14",
       unverified = "frontier2-c17", route = "L2", hazards = "heap-writes")]
pub fn uv_matrices_update(ext: GPtr<u8>) -> GPtr<u8> {
    for j in 0..2u32 {
        let used = (0..8isize).any(|k| {
            let interp = p8(rd::<u32>(ext, 8 + k * 4));
            if interp.is_null() {
                return false;
            }
            let data = p8(rd::<u32>(p8(rd::<u32>(interp, 0)), 0x14));
            let n = rd::<u32>(interp, 0x2C).max(1);
            (0..n).any(|c| rd::<u32>(data, 0x20 + c as isize * 4) == j)
        });
        let slot = j as isize * 4;
        if used {
            if rd::<u32>(ext, slot) == 0 {
                let mx = sa_core::bindings::rw_matrix_create();
                wr::<u32>(ext, slot, mx.addr() as u32);
                if mx.is_null() {
                    return GPtr::NULL;
                }
            }
        } else if rd::<u32>(ext, slot) != 0 {
            matrix_destroy(p8(rd::<u32>(ext, slot)));
            wr::<u32>(ext, slot, 0);
        }
    }
    ext
}

/// The dummy UV animation of a name the dictionary does not hold (0x7CBE00): `RtAnimAnimationCreate(scheme A's
/// type ([0x8DED84]), 2 frames, flags 0, 1.0)` (null → null); custom data from the freelist `[0xC978F4]` (hint
/// 0x30135; null → null): the name (`strncpy` 0x20, byte +0x1F cleared), channel map +0x20 = 0, reference count
/// +0x40 = 1. Both key frames are the identity (0x7CC810): the first at time 0 with no previous frame, the second
/// at 1.0 after the first. The original's identity matrix leaves its flags word (`garbage | 0x20003`) and pads
/// unset; the key frame set reads neither (only right / up / pos x, y), so the port writes 0x20003 and zeros.
#[hook(addr = 0x7CBE00, conv = cdecl, name = "FUN_007cbe00", expect = "A1 84 ED 8D 00",
       unverified = "frontier2-c17", route = "L2", hazards = "heap-writes")]
pub fn uv_anim_create_dummy(name: GPtr<u8>) -> GPtr<u8> {
    let anim = animation_create(GPtr::<u32>::from_addr(UV_SCHEME_A as usize).read(), 2, 0, 1.0);
    if anim.is_null() {
        return GPtr::NULL;
    }
    let data = crate::common::rw_memory_alloc(p8(UVANIM_FREELIST.get()), 0x30135);
    if data.is_null() {
        return GPtr::NULL;
    }
    sa_core::bindings::crt_strncpy(data, name, 0x20);
    wr::<u8>(data, 0x1F, 0);
    wr::<u32>(data, 0x20, 0);
    wr::<u32>(data, 0x40, 1);
    wr::<u32>(anim, 0x14, data.addr() as u32);
    let frames: u32 = rd(anim, 0x10);
    let second = rd::<u32>(rd::<GPtr<u8>>(anim, 0), 8).wrapping_add(frames);
    let mut m = [0u32; 16];
    m[0] = 0x3F80_0000;
    m[3] = 0x20003;
    m[5] = 0x3F80_0000;
    m[10] = 0x3F80_0000;
    let mp = GPtr::from_raw(m.as_mut_ptr()).addr() as u32;
    key_frame_set(anim.addr() as u32, frames, 0, 0, mp);
    key_frame_set(anim.addr() as u32, second, frames, 0x3F80_0000, mp);
    anim
}

/// `_rpUVAnimCustomDataStreamRead(RwStream*)` (0x7CBF70): a UV animation's custom data from the freelist
/// `[0xC978F4]` (`G->memoryAlloc`, hint 0x30135; null → null): its 0x20-byte name (`RwStreamRead`), then eight
/// int32 node-to-UV-channel entries at +0x20 (`RwStreamReadInt32`, 0x20 bytes); either read failing returns null
/// (the entry is not freed); +0x40 reference count 1. Returns the custom data.
#[hook(addr = 0x7CBF70, conv = cdecl, name = "_rpUVAnimCustomDataStreamRead", expect = "A1 F4 78 C9 00",
       unverified = "frontier2-c17", route = "L2", hazards = "heap-writes")]
pub fn uv_anim_custom_data_stream_read(stream: GPtr<u8>) -> GPtr<u8> {
    let data = crate::common::rw_memory_alloc(p8(UVANIM_FREELIST.get()), 0x30135);
    if data.is_null() {
        return GPtr::NULL;
    }
    if crate::bindings::stream_read(stream, data, 0x20) == 0 {
        return GPtr::NULL;
    }
    if crate::bindings::read_int32(stream, data.byte_add(0x20), 0x20).is_null() {
        return GPtr::NULL;
    }
    wr::<u32>(data, 0x40, 1);
    data
}

/// `RpUVAnimKeyFrameStreamRead(RwStream*, RtAnimAnimation*)` (0x7CC870): an int32 (discarded), the custom data
/// (`_rpUVAnimCustomDataStreamRead`, stored at +0x14), then per key frame (+4 frames, 0x20 bytes each from +0x10)
/// its time and data (`RwStreamReadReal`, 0x1C bytes at +4) and the previous frame's index (int32; +0 = frames +
/// index * 0x20). Any failure returns null; else the animation.
#[hook(addr = 0x7CC870, conv = cdecl, name = "RpUVAnimKeyFrameStreamRead", expect = "83 EC 08 8D 44 24 04",
       unverified = "frontier2-c17", route = "L2", hazards = "heap-writes")]
pub fn uv_anim_key_frame_stream_read(stream: GPtr<u8>, anim: GPtr<u8>) -> GPtr<u8> {
    let mut unused = 0u32;
    if crate::bindings::read_int32(stream, GPtr::from_raw(&mut unused).cast(), 4).is_null() {
        return GPtr::NULL;
    }
    let custom = custom_data_stream_read_call(stream);
    if custom.is_null() {
        return GPtr::NULL;
    }
    let frames: u32 = rd(anim, 0x10);
    wr::<u32>(anim, 0x14, custom.addr() as u32);
    let mut i = 0i32;
    while i < rd::<i32>(anim, 4) {
        let kf = p8(frames.wrapping_add(i as u32 * 0x20));
        if crate::bindings::read_real(stream, kf.byte_add(4), 0x1C).is_null() {
            return GPtr::NULL;
        }
        let mut prev = 0u32;
        if crate::bindings::read_int32(stream, GPtr::from_raw(&mut prev).cast(), 4).is_null() {
            return GPtr::NULL;
        }
        wr::<u32>(kf, 0, (prev << 5).wrapping_add(frames));
        i += 1;
    }
    anim
}

pub const HOOKS: &[HookReg] = &[
    HOOK_UVANIM_MATERIAL_STREAM_READ,
    HOOK_UV_MATRICES_UPDATE,
    HOOK_UV_ANIM_CREATE_DUMMY,
    HOOK_UV_ANIM_DESTROY,
    HOOK_UV_ANIM_CUSTOM_DATA_STREAM_READ,
    HOOK_UV_ANIM_KEY_FRAME_STREAM_READ,
    HOOK_RT_ANIM_INITIALIZE,
    HOOK_ANIM_TOOLKIT_OPEN,
    HOOK_ANIM_TOOLKIT_CLOSE,
    HOOK_RT_ANIM_REGISTER_INTERPOLATION_SCHEME,
    HOOK_RP_HANIM_PLUGIN_ATTACH,
    HOOK_HANIM_OPEN,
    HOOK_HANIM_CLOSE,
    HOOK_HANIM_FRAME_CTOR,
    HOOK_HANIM_FRAME_DTOR,
    HOOK_HANIM_FRAME_COPY,
    HOOK_RP_UVANIM_PLUGIN_ATTACH,
    HOOK_UVANIM_OPEN,
    HOOK_UVANIM_CLOSE,
    HOOK_UVANIM_MATERIAL_CTOR,
    HOOK_UVANIM_MATERIAL_DTOR,
    HOOK_UVANIM_MATERIAL_COPY,
    HOOK_RT_DICT_SCHEMA_INIT,
    HOOK_RT_DICT_SCHEMA_DESTRUCT,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = ANIM_GLOBALS;
