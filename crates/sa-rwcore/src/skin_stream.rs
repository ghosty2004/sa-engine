//! RpSkin geometry stream read and its helpers (STD-6, batch `std6-rw-1`). Ported from the 1.0 US
//! bytes (0x7C6D20, 0x7C70C0, 0x7C7190, 0x7C8BE0). `RpSkin` layout as in `skin.rs`.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, rw_free, rw_malloc, rw_memory_alloc, rw_memory_free, wr};

orig! {
    /// `RwStreamReadInt32(stream, buf, size)` (0x7ED540)
    fn read_int32(s: GPtr<u8>, p: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x7ED540 => crate::stream::rw_stream_read_int32;
    /// `RwStreamReadReal(stream, buf, size)` (0x7ED4F0)
    fn read_real(s: GPtr<u8>, p: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x7ED4F0 => crate::stream::rw_stream_read_real;
    /// `RwStreamRead(stream, buf, len)` (0x7EC9D0)
    fn stream_read(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => crate::stream::rw_stream_read;
    /// `RwStreamSkip(stream, offset)` (0x7ECD00)
    fn stream_skip(s: GPtr<u8>, offset: u32) -> GPtr<u8> = cdecl 0x7ECD00 => crate::stream::rw_stream_skip;
    /// `_rpSkinGeometryNativeRead(stream, geometry)` (0x7C88B0)
    fn geometry_native_read(s: GPtr<u8>, geom: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C88B0 => unported;
    /// `_rpSkinDeinitialize(geometry)` (0x7C8820)
    fn skin_deinitialize(geom: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C8820 => crate::skin::rp_skin_deinitialize;
    /// `_rpSkinInitialize(geometry)` (0x7C8740)
    fn skin_initialize(geom: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C8740 => crate::skin::rp_skin_initialize;
    /// `_rpSkinGetBonesUsed(skin, indices, weights, list, count, numVertices)` (0x7C70C0)
    fn bones_used_call(skin: GPtr<u8>, idx: GPtr<u8>, w: GPtr<u8>, list: GPtr<u8>, count: GPtr<u32>, nv: u32) = cdecl 0x7C70C0 => rp_skin_get_bones_used;
    /// `_rpSkinCreateData(skin, numBones, numUsed, numVertices, list, weights, indices, matrices)` (0x7C7190)
    fn create_data_call(skin: GPtr<u8>, nb: u32, nu: u32, nv: u32, list: GPtr<u8>, w: GPtr<u8>, idx: GPtr<u8>, m: GPtr<u8>) -> u32 = cdecl 0x7C7190 => rp_skin_create_data;
    /// `_rpSkinSplitDataStreamRead(stream, skin)` (0x7C8BE0)
    fn split_read_call(s: GPtr<u8>, skin: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C8BE0 => rp_skin_split_data_stream_read;
    /// `RwEngineGetPluginOffset(id)` (0x7F2BE0)
    fn engine_get_plugin_offset(id: u32) -> i32 = cdecl 0x7F2BE0 => crate::plugin::engine_get_plugin_offset;
    /// `_rpSkinAtomicSetPipeline(atomic, type)` (0x7C89B0)
    fn atomic_set_pipeline(a: GPtr<u8>, ty: u32) -> GPtr<u8> = cdecl 0x7C89B0 => crate::skin::rp_skin_atomic_set_pipeline;
}

/// Geometry plugin offset (the `RpSkin*`) and the skin freelist.
const SKIN_GEOMETRY_OFFSET: usize = 0xC978A8;
const SKIN_FREELIST: usize = 0xC978B8;
const HINT: u32 = 0x30116;

fn a32(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// `_rpSkinGetBonesUsed(skin, indices, weights, list, count, numVertices)` (0x7C70C0): `*count = 0`;
/// for every vertex and each of its `skin->maxWeights` (+0x10) weights that is non-zero (as a dword),
/// the vertex's bone index byte (`indices[v] >> 8 * j`) is appended to `list` unless already there.
#[hook(addr = 0x7C70C0, conv = cdecl, name = "FUN_007c70c0", expect = "51 8B 44 24 1C 55",
       unverified = "std6-rw-1", route = "L2", hazards = "none")]
pub fn rp_skin_get_bones_used(skin: GPtr<u8>, indices: GPtr<u8>, weights: GPtr<u8>, list: GPtr<u8>, count: GPtr<u32>, nv: u32) {
    count.write(0);
    for v in 0..nv {
        let mut j = 0u32;
        while j < rd::<u32>(skin, 0x10) {
            if rd::<u32>(weights, (v * 16 + j * 4) as isize) != 0 {
                let idx = (rd::<u32>(indices, (v * 4) as isize) >> (j * 8)) as u8;
                let n = count.read();
                if !(0..n).any(|k| rd::<u8>(list, k as isize) == idx) {
                    wr::<u8>(list, n as isize, idx);
                    count.write(n + 1);
                }
            }
            j += 1;
        }
    }
}

/// `_rpSkinCreateData(skin, numBones, numUsed, numVertices, list, weights, indices, matrices)`
/// (0x7C7190): one zeroed `RwMalloc` block (hint 0x30116) of `numBones * 64 + numVertices * 20 +
/// numUsed + 15` bytes at +0x3C (NULL returns 0): +8 the used-bone list (numUsed bytes), +0xC the
/// inverse matrices (16-aligned after the list), +0x14 the vertex bone indices (after the matrices),
/// +0x18 the weights; +0 numBones, +4 numUsed; the non-NULL sources are copied in. Returns 1.
#[hook(addr = 0x7C7190, conv = cdecl, name = "FUN_007c7190", expect = "8B 44 24 10 53 8B 5C 24 10",
       unverified = "std6-rw-1", route = "L2", hazards = "heap-writes")]
#[allow(clippy::too_many_arguments)]
pub fn rp_skin_create_data(skin: GPtr<u8>, nb: u32, nu: u32, nv: u32, list: GPtr<u8>, w: GPtr<u8>, idx: GPtr<u8>, m: GPtr<u8>) -> u32 {
    let size = (nb << 6).wrapping_add(nv.wrapping_mul(20)).wrapping_add(nu).wrapping_add(0xF);
    let data = rw_malloc(size, HINT);
    wr::<u32>(skin, 0x3C, a32(data));
    if data.is_null() {
        return 0;
    }
    for k in 0..size {
        wr::<u8>(data, k as isize, 0);
    }
    wr::<u32>(skin, 0, nb);
    wr::<u32>(skin, 4, nu);
    let mats = a32(data).wrapping_add(nu).wrapping_add(0xF) & !0xF;
    wr::<u32>(skin, 0xC, mats);
    let inds = mats.wrapping_add(nb << 6);
    wr::<u32>(skin, 0x14, inds);
    wr::<u32>(skin, 0x18, inds.wrapping_add(nv * 4));
    wr::<u32>(skin, 8, a32(data));
    let copy = |dst: u32, src: GPtr<u8>, n: u32| {
        for k in 0..n {
            wr::<u8>(p8(dst), k as isize, rd::<u8>(src, k as isize));
        }
    };
    if !list.is_null() && nu != 0 {
        copy(a32(data), list, nu);
    }
    if !m.is_null() {
        for i in (0..nb).rev() {
            copy(mats + i * 64, m.byte_add((i * 64) as isize), 64);
        }
    }
    if !idx.is_null() {
        copy(inds, idx, nv * 4);
    }
    if !w.is_null() {
        copy(rd::<u32>(skin, 0x18), w, nv * 16);
    }
    1
}

/// `_rpSkinSplitDataStreamRead(stream, skin)` (0x7C8BE0): boneLimit, numMeshes, numRLE (int32 each, a
/// failure returns NULL); numMeshes <= 0 returns the stream. Otherwise the old split block is freed, the
/// split fields cleared, a zeroed block of `numBones + 2 * (numMeshes + numRLE)` bytes allocated (NULL
/// returns NULL) and laid out as in `_rpSkinSplitDataCreate`, then read in one `RwStreamRead`; a read of
/// 0 bytes frees `skin + 0x24` (the original's pointer, not the block) and returns NULL.
#[hook(addr = 0x7C8BE0, conv = cdecl, name = "FUN_007c8be0", expect = "8B 44 24 08 83 EC 18",
       unverified = "std6-rw-1", route = "L2", hazards = "heap-writes")]
pub fn rp_skin_split_data_stream_read(s: GPtr<u8>, skin: GPtr<u8>) -> GPtr<u8> {
    let sp = skin.byte_add(0x24);
    let mut v = [0i32; 3];
    for (i, slot) in v.iter_mut().enumerate() {
        let _ = i;
        if read_int32(s, GPtr::from_raw(slot as *mut i32).cast(), 4).is_null() {
            return GPtr::NULL;
        }
    }
    let [bone_limit, nmeshes, nrle] = v;
    if nmeshes <= 0 {
        return s;
    }
    let nbones = rd::<u32>(skin, 0);
    let old = rd::<u32>(sp, 0xC);
    if old != 0 {
        rw_free(p8(old));
    }
    for o in [0isize, 4, 8, 0xC, 0x10, 0x14] {
        wr::<u32>(sp, o, 0);
    }
    let size = nbones.wrapping_add((nmeshes as u32).wrapping_add(nrle as u32).wrapping_mul(2));
    let blk = rw_malloc(size, HINT);
    wr::<u32>(sp, 0xC, a32(blk));
    if blk.is_null() {
        return GPtr::NULL;
    }
    for k in 0..size {
        wr::<u8>(blk, k as isize, 0);
    }
    wr::<i32>(sp, 0, bone_limit);
    wr::<i32>(sp, 4, nmeshes);
    let rlec = a32(blk).wrapping_add(nbones);
    wr::<u32>(sp, 0x10, rlec);
    wr::<i32>(sp, 8, nrle);
    wr::<u32>(sp, 0x14, rlec.wrapping_add(nmeshes as u32 * 2));
    let n = nbones.wrapping_add((nrle as u32).wrapping_add(nmeshes as u32).wrapping_mul(2));
    if stream_read(s, blk, n) == 0 {
        rw_free(sp);
        return GPtr::NULL;
    }
    s
}

/// `cb_SKINPLUGIN_RpGeometry_StreamRead(stream, length, geometry, offset, size)` (0x7C6D20): a native
/// geometry (flag 0x1000000) goes to `_rpSkinGeometryNativeRead`. Otherwise the header dword: numBones
/// (byte 0), numUsed (byte 1), maxWeights (byte 2). A skin comes from the freelist (16 words zeroed);
/// maxWeights 0 (old format): the data block sized with numUsed = numBones; else the used-bone list is
/// read (numUsed bytes, a short read returns NULL). Then the vertex bone indices (int32) and weights
/// (real). New format: maxWeights stored, the inverse matrices read in one block, then the split data.
/// Old format: per bone a 4-byte skip and a 64-byte matrix, maxWeights derived from the weights (1 + the
/// leading non-zero weights after the current count, up to 4), the used-bone list recomputed. The skin
/// replaces the geometry's (`_rpSkinDeinitialize` on an old one, `_rpSkinInitialize` after). Failures
/// return NULL (a failed data allocation returns the skin to the freelist).
#[hook(addr = 0x7C6D20, conv = cdecl, name = "cb_SKINPLUGIN_RpGeometry_StreamRead", expect = "81 EC 14 02 00 00",
       unverified = "std6-rw-1", route = "L2", hazards = "heap-writes,uninit-locals", recs = "new-game",
       note = "the original copies an uninitialised stack bone list into the new data block before overwriting it; here zeros")]
pub fn skin_geometry_stream_read(s: GPtr<u8>, _len: i32, geom: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    if rd::<u32>(geom, 8) & 0x100_0000 != 0 {
        if geometry_native_read(s, geom).is_null() {
            return GPtr::NULL;
        }
        return s;
    }
    let mut header = 0u32;
    if read_int32(s, GPtr::from_raw(&mut header as *mut u32).cast(), 4).is_null() {
        return GPtr::NULL;
    }
    let nbones = header & 0xFF;
    let nused_hdr = (header >> 8) & 0xFF;
    let max_weights = (header >> 16) & 0xFF;
    let nv = rd::<u32>(geom, 0x14);
    let fl = p8(g::<u32>(SKIN_FREELIST).read());
    let skin = rw_memory_alloc(fl, HINT);
    for k in 0..16 {
        wr::<u32>(skin, k * 4, 0);
    }
    let mut list = [0u8; 0x100];
    let lp = GPtr::from_raw(list.as_mut_ptr());
    let mut nused: u32 = if max_weights == 0 { nbones } else { nused_hdr };
    if nused == 0 {
        bones_used_call(skin, GPtr::NULL, GPtr::NULL, lp, GPtr::from_raw(&mut nused as *mut u32), nv);
    }
    if create_data_call(skin, nbones, nused, nv, lp, GPtr::NULL, GPtr::NULL, GPtr::NULL) == 0 {
        rw_memory_free(fl, skin);
        return GPtr::NULL;
    }
    if skin.is_null() {
        return GPtr::NULL;
    }
    if max_weights != 0 && stream_read(s, p8(rd::<u32>(skin, 8)), nused_hdr) != nused_hdr {
        return GPtr::NULL;
    }
    if read_int32(s, p8(rd::<u32>(skin, 0x14)), nv * 4).is_null() {
        return GPtr::NULL;
    }
    let weights = p8(rd::<u32>(skin, 0x18));
    if read_real(s, weights, nv * 16).is_null() {
        return GPtr::NULL;
    }
    if max_weights != 0 {
        wr::<u32>(skin, 0x10, max_weights);
        if read_real(s, p8(rd::<u32>(skin, 0xC)), rd::<u32>(skin, 0) << 6).is_null() {
            return GPtr::NULL;
        }
        if split_read_call(s, skin).is_null() {
            return GPtr::NULL;
        }
    } else {
        let mut i = 0u32;
        while i < rd::<u32>(skin, 0) {
            if stream_skip(s, 4).is_null() {
                return GPtr::NULL;
            }
            if read_real(s, p8(rd::<u32>(skin, 0xC) + i * 64), 0x40).is_null() {
                return GPtr::NULL;
            }
            i += 1;
        }
        wr::<u32>(skin, 0x10, 1);
        'all: for v in 0..nv {
            let mut j = rd::<u32>(skin, 0x10);
            while j < 4 {
                if rd::<u32>(weights, ((j + v * 4) * 4) as isize) == 0 {
                    break;
                }
                let m = rd::<u32>(skin, 0x10) + 1;
                wr::<u32>(skin, 0x10, m);
                if m == 4 {
                    break 'all;
                }
                j += 1;
            }
        }
        bones_used_call(
            skin,
            p8(rd::<u32>(skin, 0x14)),
            weights,
            p8(rd::<u32>(skin, 8)),
            skin.byte_add(4).cast(),
            nv,
        );
    }
    let off = g::<i32>(SKIN_GEOMETRY_OFFSET).read() as isize;
    let cur = rd::<u32>(geom, off);
    if cur == a32(skin) {
        return s;
    }
    if cur != 0 {
        skin_deinitialize(geom);
    }
    wr::<u32>(geom, g::<i32>(SKIN_GEOMETRY_OFFSET).read() as isize, a32(skin));
    skin_initialize(geom);
    s
}

/// The `qsort` comparator of `_rpSkinInitialize`'s used-bone list (0x7C8720): `*a - *b` on bytes.
#[hook(addr = 0x7C8720, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 4C 24 04 33 C0 33 D2",
       unverified = "std6-rw-1", route = "L1-pure", hazards = "none")]
pub fn skin_bone_compare(a: GPtr<u8>, b: GPtr<u8>) -> i32 {
    a.read() as i32 - b.read() as i32
}

/// `_rpSkinAtomicAlwaysCB(atomic, offset, size, extraData)` (0x7C6B70, the atomic's rights callback):
/// for an atomic whose geometry has a skin, the pipeline type `extraData` falls back to 1 when it asks
/// for MatFX (2) without the MatFX plugin (0x120) or for toon (3) without that plugin (0x12E), then
/// `_rpSkinAtomicSetPipeline(atomic, type)`. Returns 1.
#[hook(addr = 0x7C6B70, conv = cdecl, name = "FUN_007c6b70", expect = "57 8B 7C 24 08 8B 47 18",
       unverified = "std6-rw-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn skin_atomic_always_cb(atomic: GPtr<u8>, _offset: i32, _size: i32, extra: u32) -> u32 {
    let geom = rd::<u32>(atomic, 0x18);
    if geom != 0 && rd::<u32>(p8(geom), g::<i32>(SKIN_GEOMETRY_OFFSET).read() as isize) != 0 {
        let mut ty = extra;
        if (engine_get_plugin_offset(0x120) == -1 && ty == 2) || (engine_get_plugin_offset(0x12E) == -1 && ty == 3) {
            ty = 1;
        }
        atomic_set_pipeline(atomic, ty);
    }
    1
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RP_SKIN_GET_BONES_USED,
    HOOK_RP_SKIN_CREATE_DATA,
    HOOK_RP_SKIN_SPLIT_DATA_STREAM_READ,
    HOOK_SKIN_GEOMETRY_STREAM_READ,
    HOOK_SKIN_BONE_COMPARE,
    HOOK_SKIN_ATOMIC_ALWAYS_CB,
];
