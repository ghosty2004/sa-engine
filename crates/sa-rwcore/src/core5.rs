//! PORT-RW-5 module `core5`: RenderWare core leftovers (stream sizes and small stream readers /
//! writers of atomics, clumps, geometry, material lists, texture dictionaries and frame lists, the
//! versioned chunk header writer, rights-to-render stream writers, string helpers; see
//! docs/arch/rw-port.md, PORT-RW-5). Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{p8, rd, rw_free, rw_malloc, wr, x};

/// Plugin registries (`RwPluginRegistry`) of atomics, geometry, texture dictionaries, textures and
/// frames.
const ATOMIC_TK_LIST: u32 = 0x8D624C;
const GEOMETRY_TK_LIST: u32 = 0x8D628C;
const TEXDICT_TK_LIST: u32 = 0x8E23E4;
const TEXTURE_TK_LIST: u32 = 0x8E23CC;
const FRAME_TK_LIST: u32 = 0x8E2270;

orig! {
    /// `_rwPluginRegistryGetSize(registry, object)` (0x808B00)
    fn plugin_get_size(reg: u32, obj: GPtr<u8>) -> i32 = cdecl 0x808B00 => unported;
    /// `RpGeometryStreamGetSize(geometry)` (0x74CE20)
    fn geometry_stream_get_size(g: GPtr<u8>) -> u32 = cdecl 0x74CE20 => geometry_stream_get_size_port;
    /// The geometry struct-chunk size (0x74CE60)
    fn geometry_struct_get_size(g: GPtr<u8>) -> u32 = cdecl 0x74CE60 => unported;
    /// `_rpMaterialListStreamGetSize(list)` (0x74E450)
    fn material_list_stream_get_size(l: GPtr<u8>) -> u32 = cdecl 0x74E450 => material_list_stream_get_size_port;
    /// `RpMaterialStreamGetSize(material)` (0x74E010)
    fn material_stream_get_size(m: GPtr<u8>) -> u32 = cdecl 0x74E010 => unported;
    /// `_rpAtomicStreamWrite(atomic, info)` (0x74A880)
    fn atomic_stream_write_internal(a: GPtr<u8>, info: GPtr<u8>) -> GPtr<u8> = cdecl 0x74A880 => unported;
    /// `_rpClumpAtomicStreamRead(stream, geometryList, frameList)` (0x74B060)
    fn clump_atomic_stream_read(s: GPtr<u8>, geos: GPtr<u8>, frames: GPtr<u8>) -> GPtr<u8> = cdecl 0x74B060 => unported;
    /// `RpGeometryDestroy(geometry)` (0x74CCC0)
    fn geometry_destroy(g: GPtr<u8>) -> u32 = cdecl 0x74CCC0 => unported;
    /// `RpLightStreamGetSize(light)` (0x751E10)
    fn light_stream_get_size(l: GPtr<u8>) -> u32 = cdecl 0x751E10 => unported;
    /// `RwCameraStreamGetSize(camera)` (0x808CE0)
    fn camera_stream_get_size(c: GPtr<u8>) -> u32 = cdecl 0x808CE0 => unported;
    /// `RwStreamFindChunk(stream, type, lengthOut, versionOut)` (0x7ED2D0)
    fn stream_find_chunk(s: GPtr<u8>, ty: u32, len: GPtr<u32>, ver: GPtr<u32>) -> u32 = cdecl 0x7ED2D0 => crate::stream::rw_stream_find_chunk;
    /// `RwStreamRead(stream, buffer, length)` (0x7EC9D0)
    fn stream_read(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => crate::stream::rw_stream_read;
    /// `RwStreamWrite(stream, buffer, length)` (0x7ECB30)
    fn stream_write(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> GPtr<u8> = cdecl 0x7ECB30 => crate::stream::rw_stream_write;
    /// `RwStreamWriteInt32(stream, ints, numBytes)` (0x7ED460)
    fn stream_write_int32(s: GPtr<u8>, ints: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x7ED460 => unported;
    /// `RwTexDictionaryForAllTextures(dict, callback, data)` (0x7F3730)
    fn tex_dictionary_for_all_textures(d: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3730 => unported;
    /// The native texture size query through `RwGlobals` +0xAC `(out, texture, 0)`
    fn native_texture_get_size(out: GPtr<u32>, t: GPtr<u8>, z: u32) -> u32 = cdecl table 0xC97B24 0xAC => unported;
    /// `_rwFrameListFlattenRecurse(cursor, frame)` (0x8076A0)
    fn frame_list_flatten_recurse_call(cursor: GPtr<u32>, f: GPtr<u8>) -> u32 = cdecl 0x8076A0 => frame_list_flatten_recurse;
}

/// `_rwStreamWriteVersionedChunkHeader(stream, type, size, version, build)` (0x7ED270): writes the
/// 12-byte header `{type, size, libraryID}` with `libraryID = (((version + 0x10000) << 14) &
/// 0xFFC00000) | ((version & 0x3F) << 16) | (build & 0xFFFF)`; returns `RwStreamWrite`'s result.
#[hook(addr = 0x7ED270, conv = cdecl, name = "_rwStreamWriteVersionedChunkHeader", expect = "83 EC 0C 8B 44 24 14",
       unverified = "rw5-core-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "stream write (file effects)")]
pub fn stream_write_versioned_chunk_header(s: GPtr<u8>, ty: u32, size: u32, version: u32, build: u32) -> GPtr<u8> {
    let lib = ((version.wrapping_add(0x10000) << 14) & 0xFFC0_0000) | ((version & 0x3F) << 16) | (build & 0xFFFF);
    let mut h = [ty, size, lib];
    stream_write(s, GPtr::from_raw(h.as_mut_ptr()).cast(), 0xC)
}

/// `RpAtomicStreamGetSize(atomic)` (0x74A5B0): `RpGeometryStreamGetSize(geometry) + 0x28 +
/// pluginSize + 0xC`.
#[hook(addr = 0x74A5B0, conv = cdecl, name = "RpAtomicStreamGetSize", expect = "56 57 8B 7C 24 0C",
       unverified = "rw5-core-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn atomic_stream_get_size(a: GPtr<u8>) -> u32 {
    let s = geometry_stream_get_size(p8(rd::<u32>(a, 0x18))).wrapping_add(0x28);
    (plugin_get_size(ATOMIC_TK_LIST, a) as u32).wrapping_add(s).wrapping_add(0xC)
}

/// `RpAtomicStreamWrite(atomic, stream)` (0x74A850): `_rpAtomicStreamWrite(atomic, &info)` with the
/// stack info `{stream, ?, 0, ?, 0, 1}`; the original leaves the words at +4 and +0xC uninitialised
/// (port: 0).
#[hook(addr = 0x74A850, conv = cdecl, name = "RpAtomicStreamWrite", expect = "83 EC 18 8B 44 24 20",
       unverified = "rw5-core-1", route = "split", hazards = "uninit-locals", recs = "grove-walk",
       note = "stream write (file effects); info +4 / +0xC uninitialised in the original")]
pub fn atomic_stream_write(a: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> {
    let mut info = [s.addr() as u32, 0, 0, 0, 0, 1];
    atomic_stream_write_internal(a, GPtr::from_raw(info.as_mut_ptr()).cast())
}

/// `_rpClumpGeometryListDestroy(list)` (0x74AE50): `RpGeometryDestroy` of each of `count` (+4, signed,
/// re-read) geometries, then frees the array (+0) when set and clears it. Returns the list.
#[hook(addr = 0x74AE50, conv = cdecl, name = "_rpClumpGeometryListDestroy", expect = "56 8B 74 24 08 57",
       unverified = "rw5-core-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn clump_geometry_list_destroy(l: GPtr<u8>) -> GPtr<u8> {
    let mut i: i32 = 0;
    while i < rd::<i32>(l, 4) {
        geometry_destroy(p8(rd::<u32>(p8(rd::<u32>(l, 0)), (i * 4) as isize)));
        i += 1;
    }
    let arr = rd::<u32>(l, 0);
    if arr != 0 {
        rw_free(p8(arr));
        wr::<u32>(l, 0, 0);
    }
    l
}

/// `_rpClumpLightStreamGetSizeCB(light, size)` (0x74AFA0): `*size += 0x10`, then `*size +=
/// RpLightStreamGetSize(light) + 0xC`; returns the light.
#[hook(addr = 0x74AFA0, conv = cdecl, name = "_rpClumpLightStreamGetSizeCB", expect = "56 8B 74 24 0C 57",
       unverified = "rw5-core-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "adds to the caller's size (stack)")]
pub fn clump_light_stream_get_size_cb(light: GPtr<u8>, size: GPtr<u32>) -> GPtr<u8> {
    size.write(size.read().wrapping_add(0x10));
    let n = light_stream_get_size(light).wrapping_add(0xC);
    size.write(size.read().wrapping_add(n));
    light
}

/// `_rpClumpCameraStreamGetSizeCB(camera, size)` (0x74AFD0): `*size += 0x10`, then `*size +=
/// RwCameraStreamGetSize(camera) + 0xC`; returns the camera.
#[hook(addr = 0x74AFD0, conv = cdecl, name = "_rpClumpCameraStreamGetSizeCB", expect = "56 8B 74 24 0C 57",
       unverified = "rw5-core-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "adds to the caller's size (stack)")]
pub fn clump_camera_stream_get_size_cb(camera: GPtr<u8>, size: GPtr<u32>) -> GPtr<u8> {
    size.write(size.read().wrapping_add(0x10));
    let n = camera_stream_get_size(camera).wrapping_add(0xC);
    size.write(size.read().wrapping_add(n));
    camera
}

/// `_rpClumpAtomicStreamGetSizeCB(atomic, size)` (0x74B000): `*size += 0x28`, then `*size +=
/// pluginSize(atomic) + 0xC`; returns the atomic.
#[hook(addr = 0x74B000, conv = cdecl, name = "_rpClumpAtomicStreamGetSizeCB", expect = "56 8B 74 24 0C 57",
       unverified = "rw5-core-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "adds to the caller's size (stack)")]
pub fn clump_atomic_stream_get_size_cb(a: GPtr<u8>, size: GPtr<u32>) -> GPtr<u8> {
    size.write(size.read().wrapping_add(0x28));
    let n = (plugin_get_size(ATOMIC_TK_LIST, a) as u32).wrapping_add(0xC);
    size.write(size.read().wrapping_add(n));
    a
}

/// `RpAtomicStreamRead(stream)` (0x74B030): `_rpClumpAtomicStreamRead(stream, &geometryList,
/// &frameList)` with two stack lists whose counts (+4) are 0 and whose array words the original leaves
/// uninitialised (port: 0).
#[hook(addr = 0x74B030, conv = cdecl, name = "RpAtomicStreamRead", expect = "83 EC 10 8B 54 24 14",
       unverified = "rw5-core-1", route = "split", hazards = "uninit-locals", recs = "grove-walk",
       note = "stream read (file effects)")]
pub fn atomic_stream_read(s: GPtr<u8>) -> GPtr<u8> {
    let mut lists = [0u32; 4];
    let frames: GPtr<u8> = GPtr::from_raw(lists.as_mut_ptr()).cast();
    clump_atomic_stream_read(s, frames.byte_add(8), frames)
}

/// `_rpClumpChunkInfoRead(stream, info, lengthOut)` (0x74BD40): `RwStreamFindChunk(STRUCT)` (the length
/// goes to the original's `stream` argument slot; a local here), then 12 bytes into `info`; on success
/// `*lengthOut = length + 0xC` and returns `info`; 0 when the chunk is not found or the read is short.
#[hook(addr = 0x74BD40, conv = cdecl, name = "_rpClumpChunkInfoRead", expect = "51 56 8D 44 24 04",
       unverified = "rw5-core-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "stream read (file effects)")]
pub fn clump_chunk_info_read(s: GPtr<u8>, info: GPtr<u8>, len_out: GPtr<u32>) -> GPtr<u8> {
    let mut len: u32 = s.addr() as u32;
    let mut ver: u32 = 0;
    if stream_find_chunk(s, 1, GPtr::from_raw(&mut len), GPtr::from_raw(&mut ver)) == 0 {
        return GPtr::NULL;
    }
    if stream_read(s, info, 0xC) != 0xC {
        return GPtr::NULL;
    }
    len_out.write(len.wrapping_add(0xC));
    info
}

/// `RpGeometryStreamGetSize(geometry)` (0x74CE20): struct size (0x74CE60) + 0xC, material list size +
/// 0xC, plugin size + 0xC.
#[hook(addr = 0x74CE20, conv = cdecl, name = "RpGeometryStreamGetSize", expect = "56 57 8B 7C 24 0C",
       unverified = "rw5-core-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn geometry_stream_get_size_port(g: GPtr<u8>) -> u32 {
    let s = geometry_struct_get_size(g).wrapping_add(0xC);
    let s = material_list_stream_get_size(g.byte_add(0x20)).wrapping_add(s).wrapping_add(0xC);
    (plugin_get_size(GEOMETRY_TK_LIST, g) as u32).wrapping_add(s).wrapping_add(0xC)
}

/// `cb_RIGHTTORENDER_RpAtomic_StreamWrite(stream, length, atomic, offset, size)` (0x749160): the two
/// dwords at `[atomic + 0x6C] + 0x2C` / `+ 0x30` (the pointer is re-read for the second).
#[hook(addr = 0x749160, conv = cdecl, name = "cb_RIGHTTORENDER_RpAtomic_StreamWrite", expect = "56 8B 74 24 08 57",
       unverified = "rw5-core-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "stream write (file effects)")]
pub fn right_to_render_atomic_stream_write(s: GPtr<u8>, _len: u32, a: GPtr<u8>, _off: u32, _size: u32) -> GPtr<u8> {
    if stream_write_int32(s, p8(rd::<u32>(a, 0x6C).wrapping_add(0x2C)), 4).is_null() {
        return GPtr::NULL;
    }
    rights_to_render_tail(s, rd::<u32>(a, 0x6C))
}

/// The second rights-to-render write: the dword at `data + 0x30`; the stream, or 0 when it fails.
fn rights_to_render_tail(s: GPtr<u8>, data: u32) -> GPtr<u8> {
    if stream_write_int32(s, p8(data.wrapping_add(0x30)), 4).is_null() { GPtr::NULL } else { s }
}

/// `cb_RIGHTTORENDER_RpMaterial_StreamWrite(stream, length, material, offset, size)` (0x74D810): the
/// two dwords at `[material + 8] + 0x2C` / `+ 0x30` (the pointer is re-read for the second).
#[hook(addr = 0x74D810, conv = cdecl, name = "cb_RIGHTTORENDER_RpMaterial_StreamWrite", expect = "56 8B 74 24 08 57",
       unverified = "rw5-core-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "stream write (file effects)")]
pub fn right_to_render_material_stream_write(s: GPtr<u8>, _len: u32, m: GPtr<u8>, _off: u32, _size: u32) -> GPtr<u8> {
    if stream_write_int32(s, p8(rd::<u32>(m, 8).wrapping_add(0x2C)), 4).is_null() {
        return GPtr::NULL;
    }
    rights_to_render_tail(s, rd::<u32>(m, 8))
}

/// `_rpMaterialListAlloc(count)` (0x74E1C0): `rwmalloc(4 * count, 0x1030008)`.
#[hook(addr = 0x74E1C0, conv = cdecl, name = "_rpMaterialListAlloc", expect = "8B 44 24 04 8B 15 24 7B C9 00",
       unverified = "rw5-core-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn material_list_alloc(count: u32) -> GPtr<u8> {
    rw_malloc(count.wrapping_mul(4), 0x103_0008)
}

/// `_rpMaterialListStreamGetSize(list)` (0x74E450): `4 * count + 0x10`, plus `RpMaterialStreamGetSize +
/// 0xC` for every material that does not appear earlier in the list (count +4 re-read).
#[hook(addr = 0x74E450, conv = cdecl, name = "_rpMaterialListStreamGetSize", expect = "53 56 57 8B 7C 24 10",
       unverified = "rw5-core-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn material_list_stream_get_size_port(l: GPtr<u8>) -> u32 {
    let mut size = (rd::<u32>(l, 4)).wrapping_mul(4).wrapping_add(0x10);
    let mut i: i32 = 0;
    while i < rd::<i32>(l, 4) {
        let arr = p8(rd::<u32>(l, 0));
        let m = rd::<u32>(arr, (i * 4) as isize);
        let seen = (0..i).rev().any(|j| rd::<u32>(arr, (j * 4) as isize) == m);
        if !seen {
            size = size.wrapping_add(material_stream_get_size(p8(m))).wrapping_add(0xC);
        }
        i += 1;
    }
    size
}

/// `RwTexDictionaryStreamGetSize(dict)` (0x804930): `{size = 0x10, ok = 1}` filled by
/// `RwTexDictionaryForAllTextures(dict, _rwTexDictionaryTextureGetSizeCB)`; 0 when a texture failed,
/// else `size + pluginSize + 0xC`.
#[hook(addr = 0x804930, conv = cdecl, name = "RwTexDictionaryStreamGetSize", expect = "83 EC 08 8D 44 24 00",
       unverified = "rw5-core-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn tex_dictionary_stream_get_size(d: GPtr<u8>) -> u32 {
    let mut acc = [0x10u32, 1];
    tex_dictionary_for_all_textures(d, 0x80_4990, GPtr::from_raw(acc.as_mut_ptr()).cast());
    if acc[1] == 0 {
        return 0;
    }
    acc[0].wrapping_add(plugin_get_size(TEXDICT_TK_LIST, d) as u32).wrapping_add(0xC)
}

/// `_rwTexDictionaryTextureGetSizeCB(texture, acc)` (0x804990): the native texture size (`RwGlobals`
/// +0xAC `(&size, texture, 0)`; the size goes to the original's `texture` argument slot, a local here);
/// failure clears `acc.ok` (+4) and returns 0; otherwise `acc.size += size + 0xC`, then `+= pluginSize
/// + 0xC`, and returns the texture.
#[hook(addr = 0x804990, conv = cdecl, name = "_rwTexDictionaryTextureGetSizeCB", expect = "8B 0D 24 7B C9 00",
       unverified = "rw5-core-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the caller's accumulator (stack)")]
pub fn tex_dictionary_texture_get_size_cb(t: GPtr<u8>, acc: GPtr<u32>) -> GPtr<u8> {
    let mut size: u32 = t.addr() as u32;
    let r = native_texture_get_size(GPtr::from_raw(&mut size), t, 0);
    if r == 0 {
        wr::<u32>(p8(acc.addr() as u32), 4, r);
        return GPtr::NULL;
    }
    acc.write(acc.read().wrapping_add(size.wrapping_add(0xC)));
    let n = (plugin_get_size(TEXTURE_TK_LIST, t) as u32).wrapping_add(0xC);
    acc.write(acc.read().wrapping_add(n));
    t
}

/// `_rwFrameListFlattenRecurse(cursor, frame)` (0x8076A0): for a non-NULL frame stores it at `*cursor`,
/// advances the cursor by 4, then recurses into the child (+0x9C) and the next sibling (+0x98). Returns 1.
#[hook(addr = 0x8076A0, conv = cdecl, name = "_rwFrameListFlattenRecurse", expect = "57 8B 7C 24 0C",
       unverified = "rw5-core-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn frame_list_flatten_recurse(cursor: GPtr<u32>, f: GPtr<u8>) -> u32 {
    if !f.is_null() {
        wr::<u32>(p8(cursor.read()), 0, f.addr() as u32);
        cursor.write(cursor.read().wrapping_add(4));
        frame_list_flatten_recurse_call(cursor, p8(rd::<u32>(f, 0x9C)));
        frame_list_flatten_recurse_call(cursor, p8(rd::<u32>(f, 0x98)));
    }
    1
}

/// `_rwFrameListFindFrame(list, frame, indexOut)` (0x8076E0): the first index (of `count` +4, signed)
/// whose entry equals `frame`: stores it and returns 1; else 0.
#[hook(addr = 0x8076E0, conv = cdecl, name = "_rwFrameListFindFrame", expect = "8B 4C 24 04 33 C0",
       unverified = "rw5-core-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn frame_list_find_frame(l: GPtr<u8>, f: GPtr<u8>, out: GPtr<i32>) -> u32 {
    let n = rd::<i32>(l, 4);
    let arr = p8(rd::<u32>(l, 0));
    let mut i: i32 = 0;
    while i < n {
        if rd::<u32>(arr, (i * 4) as isize) == f.addr() as u32 {
            out.write(i);
            return 1;
        }
        i += 1;
    }
    0
}

/// `_rwFrameListStreamGetSize(list)` (0x807750): `0x38 * count + 0x10` plus `pluginSize + 0xC` per frame
/// (count +4 re-read).
#[hook(addr = 0x807750, conv = cdecl, name = "_rwFrameListStreamGetSize", expect = "53 8B 5C 24 08",
       unverified = "rw5-core-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn frame_list_stream_get_size(l: GPtr<u8>) -> u32 {
    let mut size = rd::<u32>(l, 4).wrapping_mul(0x38).wrapping_add(0x10);
    let mut i: i32 = 0;
    while i < rd::<i32>(l, 4) {
        let f = p8(rd::<u32>(p8(rd::<u32>(l, 0)), (i * 4) as isize));
        size = size.wrapping_add(plugin_get_size(FRAME_TK_LIST, f) as u32).wrapping_add(0xC);
        i += 1;
    }
    size
}

/// ASCII lower-casing of the string helpers (signed byte compares: only 'A'..='Z').
fn lower(c: u8) -> u8 {
    if (0x41..=0x5A).contains(&c) { c + 0x20 } else { c }
}

/// `_rwStringCompareNoCase(a, b)` (0x80A350): 0 when either is NULL; otherwise compares lower-cased
/// bytes up to the first difference (returns `(i8)a - (i8)b` of the lower-cased bytes) or the end (0).
#[hook(addr = 0x80A350, conv = cdecl, name = "_rwStringCompareNoCase", expect = "56 8B 74 24 08",
       unverified = "rw5-core-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn string_compare_no_case(a: GPtr<u8>, b: GPtr<u8>) -> i32 {
    if a.is_null() || b.is_null() {
        return 0;
    }
    let mut k: isize = 0;
    loop {
        let x = lower(rd::<u8>(a, k));
        let y = lower(rd::<u8>(b, k));
        if x != y {
            return (x as i8 as i32) - (y as i8 as i32);
        }
        if x == 0 {
            return 0;
        }
        k += 1;
    }
}

/// `_rwStringToUpper(s)` (0x80A3A0): 'a'..='z' minus 0x20 in place (NULL / empty untouched); returns `s`.
#[hook(addr = 0x80A3A0, conv = cdecl, name = "_rwStringToUpper", expect = "8B 44 24 04 85 C0",
       unverified = "rw5-core-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn string_to_upper(s: GPtr<u8>) -> GPtr<u8> {
    if s.is_null() {
        return s;
    }
    let mut k: isize = 0;
    while rd::<u8>(s, k) != 0 {
        let c = rd::<u8>(s, k);
        if (0x61..=0x7A).contains(&c) {
            wr::<u8>(s, k, c - 0x20);
        }
        k += 1;
    }
    s
}

/// `_rwStringToLower(s)` (0x80A3D0): 'A'..='Z' plus 0x20 in place (NULL / empty untouched); returns `s`.
#[hook(addr = 0x80A3D0, conv = cdecl, name = "_rwStringToLower", expect = "8B 44 24 04 85 C0",
       unverified = "rw5-core-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn string_to_lower(s: GPtr<u8>) -> GPtr<u8> {
    if s.is_null() {
        return s;
    }
    let mut k: isize = 0;
    while rd::<u8>(s, k) != 0 {
        let c = rd::<u8>(s, k);
        if (0x41..=0x5A).contains(&c) {
            wr::<u8>(s, k, c + 0x20);
        }
        k += 1;
    }
    s
}

/// `_rwStringFindChar(s, c)` (0x80A400): the first byte equal to `c` (the terminator matches `c` = 0),
/// else NULL.
#[hook(addr = 0x80A400, conv = cdecl, name = "_rwStringFindChar", expect = "8A 54 24 08 8B 44 24 04",
       unverified = "rw5-core-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn string_find_char(s: GPtr<u8>, c: u32) -> GPtr<u8> {
    let c = c as u8;
    let mut k: isize = 0;
    loop {
        let b = rd::<u8>(s, k);
        if b == c {
            return s.byte_add(k);
        }
        if b == 0 {
            return GPtr::NULL;
        }
        k += 1;
    }
}

/// `_rwStringFindCharReverse(s, c)` (0x80A420): the last byte equal to `c` up to and including the
/// terminator, else NULL.
#[hook(addr = 0x80A420, conv = cdecl, name = "_rwStringFindCharReverse", expect = "8B 4C 24 04 53",
       unverified = "rw5-core-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn string_find_char_reverse(s: GPtr<u8>, c: u32) -> GPtr<u8> {
    let c = c as u8;
    let mut found = GPtr::NULL;
    let mut k: isize = 0;
    loop {
        let b = rd::<u8>(s, k);
        if b == c {
            found = s.byte_add(k);
        }
        if b == 0 {
            return found;
        }
        k += 1;
    }
}

/// `_rwStringDestroy(s)` (0x80A450): `rwfree(s)`; returns 1.
#[hook(addr = 0x80A450, conv = cdecl, name = "_rwStringDestroy", expect = "8B 44 24 04 8B 0D 24 7B C9 00",
       unverified = "rw5-core-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn string_destroy(s: GPtr<u8>) -> u32 {
    rw_free(s);
    1
}

/// MatFX plugin state: material / atomic / world-sector extension offsets, the open count and the
/// effects freelist.
const MATFX_MATERIAL_OFFSET: usize = 0xC9AB74;
const MATFX_OPEN_COUNT: usize = 0xC9AB7C;
const MATFX_FREE_LIST: usize = 0xC9AB80;
const MATFX_ATOMIC_OFFSET: usize = 0xC9AB84;
const MATFX_SECTOR_OFFSET: usize = 0xC9AB88;

orig! {
    /// `RwFreeListDestroy(freelist)` (0x801B80)
    fn free_list_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => unported;
    /// `RwStreamReadInt32(stream, ints, numBytes)` (0x7ED540)
    fn stream_read_int32(s: GPtr<u8>, ints: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x7ED540 => unported;
    /// `_rpMatFXPipelinesDestroy()` (0x816190)
    fn matfx_pipelines_destroy() -> u32 = cdecl 0x816190 => unported;
    /// `MaterialDataDestroyEffects(data)` (0x810CA0)
    fn matfx_material_data_destroy_effects(d: GPtr<u8>) -> GPtr<u8> = cdecl 0x810CA0 => unported;
    /// `_rpMatFXAtomicSetPipeline(atomic)` (0x816260)
    fn matfx_atomic_set_pipeline(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x816260 => unported;
    /// `_rpMatFXWorldSectorSetPipeline(sector)` (0x816270)
    fn matfx_world_sector_set_pipeline(s: GPtr<u8>) -> GPtr<u8> = cdecl 0x816270 => unported;
}

fn gu(addr: usize) -> u32 {
    crate::common::g::<u32>(addr).read()
}

/// `cb_MATERIALEFFECTSPLUGIN_RwEngine_Close(instance, offset, size)` (0x810BB0): decrements the open
/// count and destroys the MatFX pipelines when it reaches 0; destroys the effects freelist when set and
/// clears it. Returns the instance.
#[hook(addr = 0x810BB0, conv = cdecl, name = "cb_MATERIALEFFECTSPLUGIN_RwEngine_Close", expect = "A1 7C AB C9 00",
       unverified = "rw5-core-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "engine shutdown only (one-time)")]
pub fn matfx_engine_close(instance: GPtr<u8>, _off: u32, _size: u32) -> GPtr<u8> {
    let n = gu(MATFX_OPEN_COUNT).wrapping_sub(1);
    crate::common::g::<u32>(MATFX_OPEN_COUNT).write(n);
    if n == 0 {
        matfx_pipelines_destroy();
    }
    let fl = gu(MATFX_FREE_LIST);
    if fl != 0 {
        free_list_destroy(p8(fl));
        crate::common::g::<u32>(MATFX_FREE_LIST).write(0);
    }
    instance
}

/// `cb_MATERIALEFFECTSPLUGIN_RpMaterial_Dtor(material, offset, size)` (0x810C60): with effect data in
/// the material's extension: `MaterialDataDestroyEffects(data)`, the data freed to the effects
/// freelist (`RwGlobals` +0x148) and the extension cleared (offset re-read). Returns the material.
#[hook(addr = 0x810C60, conv = cdecl, name = "cb_MATERIALEFFECTSPLUGIN_RpMaterial_Dtor", expect = "A1 74 AB C9 00",
       unverified = "rw5-core-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn matfx_material_dtor(m: GPtr<u8>, _off: u32, _size: u32) -> GPtr<u8> {
    let d = rd::<u32>(m, gu(MATFX_MATERIAL_OFFSET) as i32 as isize);
    if d != 0 {
        matfx_material_data_destroy_effects(p8(d));
        crate::common::rw_memory_free(p8(gu(MATFX_FREE_LIST)), p8(d));
        wr::<u32>(m, gu(MATFX_MATERIAL_OFFSET) as i32 as isize, 0);
    }
    m
}

/// The MatFX "effects enabled" flag stream writer: the flag at `object + [offset]` as one int32 (the
/// original stores it in its own argument slot).
fn matfx_flag_write(s: GPtr<u8>, obj: GPtr<u8>, offset_global: usize) -> GPtr<u8> {
    let mut v = rd::<u32>(obj, gu(offset_global) as i32 as isize);
    stream_write_int32(s, GPtr::from_raw(&mut v).cast(), 4)
}

/// The MatFX flag stream reader: one int32 (into the original's `length` argument slot); NULL when
/// the read fails; a non-zero value on an object whose flag is clear sets the MatFX pipeline and, when
/// that succeeds, the flag. Returns the stream.
fn matfx_flag_read(s: GPtr<u8>, len: u32, obj: GPtr<u8>, offset_global: usize, set: fn(GPtr<u8>) -> GPtr<u8>) -> GPtr<u8> {
    let mut v = len;
    if stream_read_int32(s, GPtr::from_raw(&mut v).cast(), 4).is_null() {
        return GPtr::NULL;
    }
    if v != 0 {
        let off = gu(offset_global) as i32 as isize;
        if rd::<u32>(obj, off) == 0 && !set(obj).is_null() {
            wr::<u32>(obj, off, 1);
        }
    }
    s
}

/// `cb_MATERIALEFFECTSPLUGIN_RpAtomic_StreamWrite(stream, length, atomic, offset, size)` (0x811A60): the
/// atomic's MatFX flag as an int32; returns `RwStreamWriteInt32`'s result.
#[hook(addr = 0x811A60, conv = cdecl, name = "cb_MATERIALEFFECTSPLUGIN_RpAtomic_StreamWrite", expect = "8B 44 24 0C 8B 0D 84 AB C9 00",
       unverified = "rw5-core-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "stream write (file effects)")]
pub fn matfx_atomic_stream_write(s: GPtr<u8>, _len: u32, a: GPtr<u8>, _off: u32, _size: u32) -> GPtr<u8> {
    matfx_flag_write(s, a, MATFX_ATOMIC_OFFSET)
}

/// `cb_MATERIALEFFECTSPLUGIN_RpAtomic_StreamRead(stream, length, atomic, offset, size)` (0x811A90): the
/// flag; non-zero enables the MatFX atomic pipeline (`_rpMatFXAtomicSetPipeline`, 0x816260).
#[hook(addr = 0x811A90, conv = cdecl, name = "cb_MATERIALEFFECTSPLUGIN_RpAtomic_StreamRead", expect = "57 8B 7C 24 08",
       unverified = "rw5-core-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "stream read (file effects)")]
pub fn matfx_atomic_stream_read(s: GPtr<u8>, len: u32, a: GPtr<u8>, _off: u32, _size: u32) -> GPtr<u8> {
    matfx_flag_read(s, len, a, MATFX_ATOMIC_OFFSET, matfx_atomic_set_pipeline)
}

/// `cb_MATERIALEFFECTSPLUGIN_RpAtomic_StreamGetSize(atomic, offset, size)` (0x811AE0): 4 when the
/// atomic's MatFX flag is set, else 0.
#[hook(addr = 0x811AE0, conv = cdecl, name = "cb_MATERIALEFFECTSPLUGIN_RpAtomic_StreamGetSize", expect = "8B 44 24 04 8B 0D 84 AB C9 00",
       unverified = "rw5-core-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn matfx_atomic_stream_get_size(a: GPtr<u8>, _off: u32, _size: u32) -> u32 {
    if rd::<u32>(a, gu(MATFX_ATOMIC_OFFSET) as i32 as isize) != 0 { 4 } else { 0 }
}

/// `cb_MATERIALEFFECTSPLUGIN_RpWorldSector_StreamWrite(stream, length, sector, offset, size)` (0x811B60):
/// the sector's MatFX flag as an int32.
#[hook(addr = 0x811B60, conv = cdecl, name = "cb_MATERIALEFFECTSPLUGIN_RpWorldSector_StreamWrite", expect = "8B 44 24 0C 8B 0D 88 AB C9 00",
       unverified = "rw5-core-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "stream write (file effects)")]
pub fn matfx_sector_stream_write(s: GPtr<u8>, _len: u32, sector: GPtr<u8>, _off: u32, _size: u32) -> GPtr<u8> {
    matfx_flag_write(s, sector, MATFX_SECTOR_OFFSET)
}

/// `cb_MATERIALEFFECTSPLUGIN_RpWorldSector_StreamRead(stream, length, sector, offset, size)` (0x811B90):
/// the flag; non-zero enables the MatFX sector pipeline (`_rpMatFXWorldSectorSetPipeline`, 0x816270).
#[hook(addr = 0x811B90, conv = cdecl, name = "cb_MATERIALEFFECTSPLUGIN_RpWorldSector_StreamRead", expect = "57 8B 7C 24 08",
       unverified = "rw5-core-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "stream read (file effects)")]
pub fn matfx_sector_stream_read(s: GPtr<u8>, len: u32, sector: GPtr<u8>, _off: u32, _size: u32) -> GPtr<u8> {
    matfx_flag_read(s, len, sector, MATFX_SECTOR_OFFSET, matfx_world_sector_set_pipeline)
}

/// `cb_MATERIALEFFECTSPLUGIN_RpWorldSector_StreamGetSize(sector, offset, size)` (0x811BE0): 4 when the
/// sector's MatFX flag is set, else 0.
#[hook(addr = 0x811BE0, conv = cdecl, name = "cb_MATERIALEFFECTSPLUGIN_RpWorldSector_StreamGetSize", expect = "8B 44 24 04 8B 0D 88 AB C9 00",
       unverified = "rw5-core-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn matfx_sector_stream_get_size(sector: GPtr<u8>, _off: u32, _size: u32) -> u32 {
    if rd::<u32>(sector, gu(MATFX_SECTOR_OFFSET) as i32 as isize) != 0 { 4 } else { 0 }
}

orig! {
    /// An iterator callback `(object, data)` through its pointer
    fn call_iter_cb(f: u32, obj: u32, data: u32) -> u32 = cdecl dyn => unported;
}

/// `RpWorldForAllClumps(world, callback, data)` (0x74FB80): the world's clump list (+0x2C, links at
/// clump +0x20); the next link is read before each call; stops when the callback returns NULL.
/// Returns the world.
#[hook(addr = 0x74FB80, conv = cdecl, name = "RpWorldForAllClumps", expect = "8B 44 24 04 53 55",
       unverified = "rw5-core-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn world_for_all_clumps(w: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> {
    let head = w.addr() as u32 + 0x2C;
    let mut l = rd::<u32>(w, 0x2C);
    while l != head {
        let next = rd::<u32>(p8(l), 0);
        if call_iter_cb(cb, l.wrapping_sub(0x20), data) == 0 {
            break;
        }
        l = next;
    }
    w
}

/// `RpWorldForAllMaterials(world, callback, data)` (0x74FBC0): the world's material list (array +0x10,
/// count +0x14 read once); stops when the callback returns NULL. Returns the world.
#[hook(addr = 0x74FBC0, conv = cdecl, name = "RpWorldForAllMaterials", expect = "53 55 56 57 8B 7C 24 14",
       unverified = "rw5-core-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn world_for_all_materials(w: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> {
    let n = rd::<i32>(w, 0x14);
    let mut i: i32 = 0;
    while i < n {
        let m = rd::<u32>(p8(rd::<u32>(w, 0x10)), (i * 4) as isize);
        if call_iter_cb(cb, m, data) == 0 {
            break;
        }
        i += 1;
    }
    w
}

/// `RpWorldForAllWorldSectors(world, callback, data)` (0x74FC70): depth-first over the BSP from the root
/// sector (+0x1C) with a 64-entry stack: a plane sector (type +0 >= 0) pushes its right subtree (+0xC)
/// and descends left (+8); an atomic sector is passed to the callback (NULL stops) and the next one is
/// popped. The original reads the never-written stack slot 0 after the last sector (unused). Returns the
/// world.
#[hook(addr = 0x74FC70, conv = cdecl, name = "RpWorldForAllWorldSectors", expect = "81 EC 00 01 00 00",
       unverified = "rw5-core-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn world_for_all_world_sectors(w: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> {
    let mut stack = [0u32; 64];
    let mut sp: i32 = 0;
    let mut s = rd::<u32>(w, 0x1C);
    loop {
        if rd::<i32>(p8(s), 0) < 0 {
            if call_iter_cb(cb, s, data) == 0 {
                break;
            }
            s = stack[sp as usize];
            sp -= 1;
        } else {
            let left = rd::<u32>(p8(s), 8);
            sp += 1;
            stack[sp as usize] = rd::<u32>(p8(s), 0xC);
            s = left;
        }
        if sp < 0 {
            break;
        }
    }
    w
}

/// `RpAtomicForAllWorldSectors(atomic, callback, data)` (0x751060): the atomic's sector ties (+0x64,
/// sector at tie +8); the next link is read before each call; stops when the callback returns NULL.
/// Returns the atomic.
#[hook(addr = 0x751060, conv = cdecl, name = "RpAtomicForAllWorldSectors", expect = "8B 44 24 04 53 55",
       unverified = "rw5-core-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn atomic_for_all_world_sectors(a: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> {
    let head = a.addr() as u32 + 0x64;
    let mut l = rd::<u32>(a, 0x64);
    while l != head {
        let next = rd::<u32>(p8(l), 0);
        if call_iter_cb(cb, rd::<u32>(p8(l), 8), data) == 0 {
            break;
        }
        l = next;
    }
    a
}

/// `RpLightSetConeAngle(light, angle)` (0x751D20): only for `angle >= 0` (or unordered) and `angle <=`
/// the f32 at 0x858FE4 (or unordered): `light.minusCosAngle (+0x28) = -fcos(angle)` and returns the
/// light; otherwise 0.
#[hook(addr = 0x751D20, conv = cdecl, name = "RpLightSetConeAngle", expect = "D9 44 24 08 D8 1D 50 8B 85 00",
       unverified = "rw5-core-2", route = "L2", hazards = "x87-transcendental", recs = "grove-walk")]
pub fn light_set_cone_angle(l: GPtr<u8>, angle: f32) -> GPtr<u8> {
    let a = x(angle);
    if !a.fcom(x(crate::common::g::<f32>(0x858B50).read())).test_parity(0x05) {
        return GPtr::NULL;
    }
    let c = a.fcom(x(crate::common::g::<f32>(0x858FE4).read()));
    if c.ah() & 0x41 == 0 {
        return GPtr::NULL;
    }
    wr::<f32>(l, 0x28, a.fcos().fchs().to_f32());
    l
}

/// `_rpMeshHeaderDestroy(header)` (0x758910): a tail jump to `rwfree(header)`.
#[hook(addr = 0x758910, conv = cdecl, name = "_rpMeshHeaderDestroy", expect = "8B 0D 24 7B C9 00",
       unverified = "rw5-core-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn mesh_header_destroy(h: GPtr<u8>) {
    rw_free(h);
}

/// `_rpMeshSize(header, object)` (0x759090): for a native geometry (type byte 8 with flags +8 &
/// 0x1000000) or a native world (type 7, same flag) `8 * numMeshes + 0xC`, otherwise `4 * (2 *
/// numMeshes + totalIndices) + 0xC` (numMeshes u16 +4, totalIndices +8 of the header).
#[hook(addr = 0x759090, conv = cdecl, name = "_rpMeshSize", expect = "8B 44 24 08 BA 00 00 00 01",
       unverified = "rw5-core-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn mesh_size(h: GPtr<u8>, obj: GPtr<u8>) -> u32 {
    let ty = rd::<u8>(obj, 0);
    let native = (ty == 8 || ty == 7) && rd::<u32>(obj, 8) & 0x100_0000 != 0;
    let n = rd::<u16>(h, 4) as u32;
    if native {
        n * 8 + 0xC
    } else {
        rd::<u32>(h, 8).wrapping_add(n * 2).wrapping_mul(4).wrapping_add(0xC)
    }
}

orig! {
    /// `_rwDeviceSystemRequest(device, request, out, in, number)` (0x7F2AB0)
    fn device_system_request(dev: GPtr<u8>, req: u32, out: GPtr<u32>, inp: u32, num: u32) -> u32 = cdecl 0x7F2AB0 => unported;
    /// The device's camera clear through `RwGlobals` +0x9C `(camera, color, mode)`
    fn device_camera_clear(c: GPtr<u8>, color: u32, mode: u32) -> u32 = cdecl table 0xC97B24 0x9C => unported;
    /// Immediate-module teardown helper (0x80A140)
    fn immediate_close_a(p: GPtr<u8>) = cdecl 0x80A140 => unported;
    /// Immediate-module teardown helper (0x80A110)
    fn immediate_close_b(p: GPtr<u8>) = cdecl 0x80A110 => unported;
    /// CRT `malloc(size)` (0x824257)
    fn crt_malloc(size: u32) -> u32 = cdecl 0x824257 => unported;
    /// CRT `realloc(p, size)` (0x824269)
    fn crt_realloc(p: u32, size: u32) -> u32 = cdecl 0x824269 => unported;
    /// CRT `calloc(n, size)` (0x824416)
    fn crt_calloc(n: u32, size: u32) -> u32 = cdecl 0x824416 => unported;
}

/// A module close callback's freelist step: the freelist at `RwGlobals + [offset_global]` destroyed and
/// cleared when set (both re-read), then the open count at `count` decremented.
fn module_close_free_list(offset_global: usize, count: usize) {
    let slot = || crate::common::rw_globals().byte_add(gu(offset_global) as i32 as isize);
    let fl = rd::<u32>(slot(), 0);
    if fl != 0 {
        free_list_destroy(p8(fl));
        wr::<u32>(slot(), 0, 0);
    }
    crate::common::g::<u32>(count).write(gu(count).wrapping_sub(1));
}

/// `cb_CAMERAMODULE_RwEngine_Close(instance, offset, size)` (0x7EE0B0): destroys the camera freelist
/// (`RwGlobals + [0xC97960]`) and decrements the open count 0xC97964. Returns the instance.
#[hook(addr = 0x7EE0B0, conv = cdecl, name = "cb_CAMERAMODULE_RwEngine_Close", expect = "A1 24 7B C9 00",
       unverified = "rw5-core-2", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "engine shutdown only (one-time)")]
pub fn camera_module_close(instance: GPtr<u8>, _off: u32, _size: u32) -> GPtr<u8> {
    module_close_free_list(0xC97960, 0xC97964);
    instance
}

/// `cb_MATRIXMODULE_RwEngine_Close(instance, offset, size)` (0x7F1660): destroys the matrix freelist
/// (`RwGlobals + [0xC979BC]`) and decrements the open count 0xC979C0. Returns the instance.
#[hook(addr = 0x7F1660, conv = cdecl, name = "cb_MATRIXMODULE_RwEngine_Close", expect = "A1 24 7B C9 00",
       unverified = "rw5-core-2", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "engine shutdown only (one-time)")]
pub fn matrix_module_close(instance: GPtr<u8>, _off: u32, _size: u32) -> GPtr<u8> {
    module_close_free_list(0xC979BC, 0xC979C0);
    instance
}

/// `cb_IMMEDIATEMODULE_RwEngine_Close(instance, offset, size)` (0x7EFDE0): the teardown helpers
/// 0x80A140 (module +0x20) and 0x80A110 (module +0x1C) on the module data at `RwGlobals +
/// [0xC9C078]`, then the open count 0xC9C07C decremented. Returns the instance.
#[hook(addr = 0x7EFDE0, conv = cdecl, name = "cb_IMMEDIATEMODULE_RwEngine_Close", expect = "A1 24 7B C9 00",
       unverified = "rw5-core-2", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "engine shutdown only (one-time)")]
pub fn immediate_module_close(instance: GPtr<u8>, _off: u32, _size: u32) -> GPtr<u8> {
    let m = || crate::common::rw_globals().byte_add(gu(0xC9C078) as i32 as isize);
    immediate_close_a(m().byte_add(0x20));
    immediate_close_b(m().byte_add(0x1C));
    crate::common::g::<u32>(0xC9C07C).write(gu(0xC9C07C).wrapping_sub(1));
    instance
}

/// `RwCameraClear(camera, color, mode)` (0x7EE340): the device's clear (`RwGlobals` +0x9C); the camera
/// when it succeeded, else NULL.
#[hook(addr = 0x7EE340, conv = cdecl, name = "RwCore::RwCameraClear", expect = "8B 4C 24 0C A1 24 7B C9 00",
       unverified = "rw5-core-2", route = "L3-only", hazards = "none", recs = "grove-walk",
       note = "D3D clear (device effects)")]
pub fn camera_clear(c: GPtr<u8>, color: u32, mode: u32) -> GPtr<u8> {
    if device_camera_clear(c, color, mode) != 0 { c } else { GPtr::NULL }
}

/// The engine query shared by the two getters below: `_rwDeviceSystemRequest(&RwGlobals.device
/// (+0x10), request, &value, 0, 0)`; the value, or -1 when the request fails. The original's value
/// slot is uninitialised before the request (port: 0).
fn engine_query(req: u32) -> i32 {
    let mut v: u32 = 0;
    if device_system_request(crate::common::rw_globals().byte_add(0x10), req, GPtr::from_raw(&mut v), 0, 0) != 0 {
        v as i32
    } else {
        -1
    }
}

/// `RwEngineGetTextureMemorySize()` (0x7F2D80): device request 0xC.
#[hook(addr = 0x7F2D80, conv = cdecl, name = "RwEngineGetTextureMemorySize", expect = "51 8B 0D 24 7B C9 00",
       unverified = "rw5-core-2", route = "L2", hazards = "uninit-locals", recs = "grove-walk")]
pub fn engine_get_texture_memory_size() -> i32 {
    engine_query(0xC)
}

/// `RwEngineGetMaxTextureSize()` (0x7F2DB0): device request 0x13.
#[hook(addr = 0x7F2DB0, conv = cdecl, name = "RwEngineGetMaxTextureSize", expect = "51 8B 0D 24 7B C9 00",
       unverified = "rw5-core-2", route = "L2", hazards = "uninit-locals", recs = "grove-walk")]
pub fn engine_get_max_texture_size() -> i32 {
    engine_query(0x13)
}

/// `RwEngineSetFocus(enable)` (0x7F2DE0): device request 8 `(NULL, NULL, enable)`; 1 when it succeeded.
#[hook(addr = 0x7F2DE0, conv = cdecl, name = "RwEngineSetFocus", expect = "8B 44 24 04 8B 0D 24 7B C9 00",
       unverified = "rw5-core-2", route = "L3-only", hazards = "none", recs = "grove-walk",
       note = "device focus change (OS effects)")]
pub fn engine_set_focus(enable: u32) -> u32 {
    (device_system_request(crate::common::rw_globals().byte_add(0x10), 8, GPtr::NULL, 0, enable) != 0) as u32
}

/// `_rwMemoryDefaultMalloc(size)` (0x8020A0): CRT `malloc`.
#[hook(addr = 0x8020A0, conv = cdecl, name = "_rwMemoryDefaultMalloc", expect = "8B 44 24 04 50",
       unverified = "rw5-core-2", route = "L3-only", hazards = "none", recs = "grove-walk",
       note = "CRT heap allocation (addresses differ per run)")]
pub fn memory_default_malloc(size: u32) -> u32 {
    crt_malloc(size)
}

/// `_rwMemoryDefaultRealloc(p, size)` (0x8020B0): CRT `realloc`.
#[hook(addr = 0x8020B0, conv = cdecl, name = "_rwMemoryDefaultRealloc", expect = "8B 44 24 08 8B 4C 24 04",
       unverified = "rw5-core-2", route = "L3-only", hazards = "none", recs = "grove-walk",
       note = "CRT heap allocation (addresses differ per run)")]
pub fn memory_default_realloc(p: u32, size: u32) -> u32 {
    crt_realloc(p, size)
}

/// `_rwMemoryDefaultCalloc(n, size)` (0x8020D0): CRT `calloc`.
#[hook(addr = 0x8020D0, conv = cdecl, name = "_rwMemoryDefaultCalloc", expect = "8B 44 24 08 8B 4C 24 04",
       unverified = "rw5-core-2", route = "L3-only", hazards = "none", recs = "grove-walk",
       note = "CRT heap allocation (addresses differ per run)")]
pub fn memory_default_calloc(n: u32, size: u32) -> u32 {
    crt_calloc(n, size)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_STREAM_WRITE_VERSIONED_CHUNK_HEADER,
    HOOK_ATOMIC_STREAM_GET_SIZE,
    HOOK_ATOMIC_STREAM_WRITE,
    HOOK_CLUMP_GEOMETRY_LIST_DESTROY,
    HOOK_CLUMP_LIGHT_STREAM_GET_SIZE_CB,
    HOOK_CLUMP_CAMERA_STREAM_GET_SIZE_CB,
    HOOK_CLUMP_ATOMIC_STREAM_GET_SIZE_CB,
    HOOK_ATOMIC_STREAM_READ,
    HOOK_CLUMP_CHUNK_INFO_READ,
    HOOK_GEOMETRY_STREAM_GET_SIZE_PORT,
    HOOK_RIGHT_TO_RENDER_ATOMIC_STREAM_WRITE,
    HOOK_RIGHT_TO_RENDER_MATERIAL_STREAM_WRITE,
    HOOK_MATERIAL_LIST_ALLOC,
    HOOK_MATERIAL_LIST_STREAM_GET_SIZE_PORT,
    HOOK_TEX_DICTIONARY_STREAM_GET_SIZE,
    HOOK_TEX_DICTIONARY_TEXTURE_GET_SIZE_CB,
    HOOK_FRAME_LIST_FLATTEN_RECURSE,
    HOOK_FRAME_LIST_FIND_FRAME,
    HOOK_FRAME_LIST_STREAM_GET_SIZE,
    HOOK_STRING_COMPARE_NO_CASE,
    HOOK_STRING_TO_UPPER,
    HOOK_STRING_TO_LOWER,
    HOOK_STRING_FIND_CHAR,
    HOOK_STRING_FIND_CHAR_REVERSE,
    HOOK_STRING_DESTROY,
    HOOK_MATFX_ENGINE_CLOSE,
    HOOK_MATFX_MATERIAL_DTOR,
    HOOK_MATFX_ATOMIC_STREAM_WRITE,
    HOOK_MATFX_ATOMIC_STREAM_READ,
    HOOK_MATFX_ATOMIC_STREAM_GET_SIZE,
    HOOK_MATFX_SECTOR_STREAM_WRITE,
    HOOK_MATFX_SECTOR_STREAM_READ,
    HOOK_MATFX_SECTOR_STREAM_GET_SIZE,
    HOOK_WORLD_FOR_ALL_CLUMPS,
    HOOK_WORLD_FOR_ALL_MATERIALS,
    HOOK_WORLD_FOR_ALL_WORLD_SECTORS,
    HOOK_ATOMIC_FOR_ALL_WORLD_SECTORS,
    HOOK_LIGHT_SET_CONE_ANGLE,
    HOOK_MESH_HEADER_DESTROY,
    HOOK_MESH_SIZE,
    HOOK_CAMERA_MODULE_CLOSE,
    HOOK_MATRIX_MODULE_CLOSE,
    HOOK_IMMEDIATE_MODULE_CLOSE,
    HOOK_CAMERA_CLEAR,
    HOOK_ENGINE_GET_TEXTURE_MEMORY_SIZE,
    HOOK_ENGINE_GET_MAX_TEXTURE_SIZE,
    HOOK_ENGINE_SET_FOCUS,
    HOOK_MEMORY_DEFAULT_MALLOC,
    HOOK_MEMORY_DEFAULT_REALLOC,
    HOOK_MEMORY_DEFAULT_CALLOC,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
