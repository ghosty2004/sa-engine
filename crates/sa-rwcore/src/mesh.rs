//! MESHMODULE, build meshes and the bin-mesh stream reader (RpWorld, PORT-RW-1). Ported from the
//! 1.0 US bytes; `docs/re/specs/RW/RpMesh.md` is the reading aid.
//!
//! `RpMeshHeader` (0x10, then `numMeshes` RpMesh (0xC: +0 indices, +4 numIndices, +8 material), then
//! the u16 indices; one malloc, hint 0x30502): +0 flags, +4 u16 numMeshes, +6 u16 serialNum, +8
//! totalIndicesInMesh, +0xC firstMeshOffset. `RpBuildMesh` (0xC, freelist `[0xC926F0]`): +0
//! triangleBufferSize, +4 numTriangles, +8 meshTriangles (0x14 each: +0/+2/+4 u16 vertIndex, +8
//! material, +0xC matIndex, +0xE textureIndex, +0x10 rasterIndex, +0x12 pipelineIndex).
//! MESHMODULE `G + [0xC9B8C0]` (0x30): +0 u16 next serial number, +8 flags -> primitive type table,
//! +0x28 primitive type -> flags table. RpWorld errors are `RwErrorSet({2, _rwerror(..)})`.
//!
//! `_rpMeshRead` keeps its loop counter in the `flags` slot of its stack header (not reproduced:
//! the header is a Rust local).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, module, p8, rd, rw_error2, rw_error_set, rw_free, rw_malloc, rw_memory_alloc, rw_memory_free, rw_realloc, wr, RwError};

/// MESHMODULE offset global.
pub const MESH_MODULE: usize = 0xC9B8C0;
/// MESHMODULE open count.
const MESH_INSTANCES: usize = 0xC9B8C4;
/// The build-mesh freelist (shared by all engine instances).
const BUILD_MESH_FREE_LIST: usize = 0xC926F0;

const E_RW_NOMEM: u32 = 0x8000_0013;
const FLAG_NATIVE: u32 = 0x0100_0000;

orig! {
    /// `_rwFreeListCreate(entrySize, entriesPerBlock, alignment, hint)` (0x801980)
    fn rw_free_list_create(size: u32, per_block: u32, align: u32, hint: u32) -> GPtr<u8> = cdecl 0x801980 => crate::memory::rw_freelist_create;
    /// `RwFreeListDestroy(fl)` (0x801B80)
    fn rw_free_list_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => crate::memory::rw_freelist_destroy;
    /// `RwStreamReadInt32(stream, p, numBytes)` (0x7ED540)
    fn rw_stream_read_int32_call(s: GPtr<u8>, p: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x7ED540 => crate::stream::rw_stream_read_int32;
    /// `_rpMaterialListGetMaterial(list, index)` (0x74E2B0)
    fn material_list_get_material_call(l: GPtr<u8>, i: i32) -> GPtr<u8> = cdecl 0x74E2B0 => crate::material::material_list_get_material;
    /// `_rpMeshRead(stream, object, matList)` (0x758EC0)
    fn rp_mesh_read_call(s: GPtr<u8>, obj: GPtr<u8>, ml: GPtr<u8>) -> GPtr<u8> = cdecl 0x758EC0 => rp_mesh_read;
    /// A mesh callback `cb(mesh, header, data)` (`_rpMeshHeaderForAllMeshes`)
    fn call_mesh_cb(f: u32, m: GPtr<u8>, h: GPtr<u8>, data: u32) -> GPtr<u8> = cdecl dyn => unported;
}

/// `RwErrorSet({2, _rwerror(code, a)})`.
fn error2_p2(code: u32, a: u32) {
    let mut e = RwError { plugin_id: 2, error_code: 0 };
    e.error_code = rw_error2(code, a) as i32;
    rw_error_set(GPtr::from_raw(&mut e));
}

fn build_mesh_free_list() -> GPtr<u8> {
    p8(g::<u32>(BUILD_MESH_FREE_LIST).read())
}

/// `cb_BINMESHPLUGIN_RpGeometry_StreamRead(stream, len, geometry, offset, size)` (0x750DB0):
/// `geometry->mesh = _rpMeshRead(stream, geometry, &geometry->matList)` (an old mesh is overwritten,
/// not freed); stream when a mesh was read, else NULL.
#[hook(addr = 0x750DB0, conv = cdecl, name = "cb_BINMESHPLUGIN_RpGeometry_StreamRead", expect = "56 8B 74 24 10")]
pub fn bin_mesh_stream_read(s: GPtr<u8>, _len: i32, gm: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let mesh = rp_mesh_read_call(s, gm, gm.byte_add(0x20));
    wr::<u32>(gm, 0x54, mesh.addr() as u32);
    if mesh.is_null() {
        GPtr::NULL
    } else {
        s
    }
}

/// `_rpMeshHeaderCreate(size)` (0x758920): `malloc(size, 0x30502)`, uninitialised.
#[hook(addr = 0x758920, conv = cdecl, name = "_rpMeshHeaderCreate", expect = "8B 44 24 04 8B 0D 24 7B C9 00")]
pub fn rp_mesh_header_create(size: u32) -> GPtr<u8> {
    rw_malloc(size, 0x30502)
}

/// `_rpMeshClose(instance, offset, size)` (0x758940): the last close destroys and clears the build-mesh
/// freelist. Returns instance.
#[hook(addr = 0x758940, conv = cdecl, name = "_rpMeshClose", expect = "A1 C4 B8 C9 00")]
pub fn rp_mesh_close(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    let c = g::<u32>(MESH_INSTANCES);
    let n = c.read().wrapping_sub(1);
    c.write(n);
    if n == 0 {
        let fl = build_mesh_free_list();
        if !fl.is_null() {
            rw_free_list_destroy(fl);
            g::<u32>(BUILD_MESH_FREE_LIST).write(0);
        }
    }
    instance
}

/// `_rpMeshOpen(instance, offset, size)` (0x758970): the first open creates the build-mesh freelist
/// (0xC bytes, 0x32 per block, alignment 4, hint 0x40502; NULL -> NULL). Then serial number 1, the
/// count, and the flag/primitive tables (only the listed bytes are written).
#[hook(addr = 0x758970, conv = cdecl, name = "_rpMeshOpen", expect = "8B 44 24 08 A3 C0 B8 C9 00")]
pub fn rp_mesh_open(instance: GPtr<u8>, offset: u32, _size: u32) -> GPtr<u8> {
    g::<u32>(MESH_MODULE).write(offset);
    if g::<u32>(MESH_INSTANCES).read() == 0 {
        let fl = rw_free_list_create(0xC, 0x32, 4, 0x40502);
        g::<u32>(BUILD_MESH_FREE_LIST).write(fl.addr() as u32);
        if fl.is_null() {
            return GPtr::NULL;
        }
    }
    wr::<u16>(module(MESH_MODULE), 0, 1);
    let c = g::<u32>(MESH_INSTANCES);
    c.write(c.read().wrapping_add(1));
    for (off, v) in [
        (0x8, 3u8),
        (0x9, 4),
        (0xA, 5),
        (0xC, 1),
        (0x10, 2),
        (0x18, 6),
        (0x28, 4),
        (0x29, 8),
        (0x2A, 0),
        (0x2B, 1),
        (0x2C, 2),
        (0x2D, 0x10),
    ] {
        wr::<u8>(module(MESH_MODULE), off, v);
    }
    instance
}

/// `_rpBuildMeshCreate(bufferSize)` (0x758A90): from the build-mesh freelist (hint 0x30502; failure ->
/// error {2, E_RW_NOMEM(0xC)}, NULL); a triangle buffer of `bufferSize * 20` bytes (hint 0x1030502;
/// failure -> build mesh freed, error {2, E_RW_NOMEM(size)}, NULL) or none for 0.
#[hook(addr = 0x758A90, conv = cdecl, name = "_rpBuildMeshCreate", expect = "A1 F0 26 C9 00")]
pub fn rp_build_mesh_create(n: u32) -> GPtr<u8> {
    let bm = rw_memory_alloc(build_mesh_free_list(), 0x30502);
    if bm.is_null() {
        error2_p2(E_RW_NOMEM, 0xC);
        return GPtr::NULL;
    }
    wr::<u32>(bm, 4, 0);
    if n == 0 {
        wr::<u32>(bm, 8, 0);
        wr::<u32>(bm, 0, 0);
        return bm;
    }
    let size = n.wrapping_mul(20);
    let p = rw_malloc(size, 0x0103_0502);
    wr::<u32>(bm, 8, p.addr() as u32);
    if p.is_null() {
        rw_memory_free(build_mesh_free_list(), bm);
        error2_p2(E_RW_NOMEM, size);
        return GPtr::NULL;
    }
    wr::<u32>(bm, 0, n);
    bm
}

/// `_rpBuildMeshDestroy(bm)` (0x758B80): frees the triangle buffer (cleared) and the build mesh.
/// Returns 1.
#[hook(addr = 0x758B80, conv = cdecl, name = "_rpBuildMeshDestroy", expect = "56 8B 74 24 08")]
pub fn rp_build_mesh_destroy(bm: GPtr<u8>) -> u32 {
    let t = rd::<u32>(bm, 8);
    if t != 0 {
        rw_free(p8(t));
        wr::<u32>(bm, 8, 0);
    }
    rw_memory_free(build_mesh_free_list(), bm);
    1
}

/// `_rpMeshDestroy(header)` (0x758BC0): frees the header unless its 16 bytes are all zero. Returns 1.
#[hook(addr = 0x758BC0, conv = cdecl, name = "_rpMeshDestroy", expect = "8B 44 24 04 83 38 00")]
pub fn rp_mesh_destroy(h: GPtr<u8>) -> u32 {
    let empty = rd::<u32>(h, 0) == 0
        && rd::<u16>(h, 4) == 0
        && rd::<u16>(h, 6) == 0
        && rd::<u32>(h, 8) == 0
        && rd::<u32>(h, 0xC) == 0;
    if !empty {
        rw_free(h);
    }
    1
}

/// `_rpBuildMeshAddTriangle(bm, mat, v0, v1, v2, matIndex, texIndex, rasIndex, pipeIndex)` (0x758C00):
/// grows the buffer one entry at a time when full (realloc when the count is non-zero, else malloc;
/// hint 0x1030502; failure -> error {2, E_RW_NOMEM(size)}, NULL), stores the low 16 bits of the
/// indices. Returns bm.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x758C00, conv = cdecl, name = "_rpBuildMeshAddTriangle", expect = "83 EC 08 56 8B 74 24 10")]
pub fn rp_build_mesh_add_triangle(bm: GPtr<u8>, mat: GPtr<u8>, v0: u32, v1: u32, v2: u32, mi: u32, ti: u32, ri: u32, pi: u32) -> GPtr<u8> {
    let n = rd::<u32>(bm, 4);
    if n >= rd::<u32>(bm, 0) {
        let size = n.wrapping_add(1).wrapping_mul(20);
        let p = if n != 0 { rw_realloc(p8(rd::<u32>(bm, 8)), size, 0x0103_0502) } else { rw_malloc(size, 0x0103_0502) };
        if p.is_null() {
            error2_p2(E_RW_NOMEM, size);
            return GPtr::NULL;
        }
        wr::<u32>(bm, 8, p.addr() as u32);
        wr::<u32>(bm, 0, rd::<u32>(bm, 4).wrapping_add(1));
    }
    let t = || p8(rd::<u32>(bm, 8).wrapping_add(rd::<u32>(bm, 4).wrapping_mul(20)));
    wr::<u32>(t(), 8, mat.addr() as u32);
    wr::<u16>(t(), 0, v0 as u16);
    wr::<u16>(t(), 2, v1 as u16);
    wr::<u16>(t(), 4, v2 as u16);
    wr::<u16>(t(), 0xC, mi as u16);
    wr::<u16>(t(), 0xE, ti as u16);
    wr::<u16>(t(), 0x10, ri as u16);
    wr::<u16>(t(), 0x12, pi as u16);
    wr::<u32>(bm, 4, rd::<u32>(bm, 4).wrapping_add(1));
    bm
}

/// `_rpMeshHeaderForAllMeshes(header, cb, data)` (0x758D30): `cb(mesh, header, data)` over the
/// `numMeshes` meshes from `header + 0x10 + firstMeshOffset` until it returns NULL. Returns header.
#[hook(addr = 0x758D30, conv = cdecl, name = "_rpMeshHeaderForAllMeshes", expect = "53 8B 5C 24 08")]
pub fn rp_mesh_header_for_all_meshes(h: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> {
    let mut n = rd::<u16>(h, 4) as u32;
    let mut m = h.byte_add(rd::<u32>(h, 0xC) as i32 as isize + 0x10);
    while n != 0 {
        n -= 1;
        if call_mesh_cb(cb, m, h, data).is_null() {
            break;
        }
        m = m.byte_add(0xC);
    }
    h
}

/// `_rpMeshGetNextSerialNumber()` (0x7590E0): returns the MESHMODULE's u16 serial counter (`G +
/// [0xC9B8C0]` +0) and stores it + 1. The result is `mov ax, [..]`: EAX's upper half stays the
/// caller's (`ret = ax:entry`); ECX/EDX hold the module offset / global base, ESI is saved.
#[hook(addr = 0x7590E0, conv = reg("ret = ax:entry"), name = "_rpMeshGetNextSerialNumber",
       expect = "8B 0D C0 B8 C9 00",
       unverified = "regconv-rw-1", route = "L2", hazards = "ax-al-return", recs = "boot",
       note = "16-bit result with the caller's upper EAX (HARNESS-5 ax:entry)")]
pub fn rp_mesh_get_next_serial_number() -> u16 {
    let m = module(MESH_MODULE);
    let n = rd::<u16>(m, 0);
    wr::<u16>(m, 0, n.wrapping_add(1));
    n
}

/// `_rpMeshRead(stream, object, matList)` (0x758EC0): the 12-byte header (`flags, numMeshes,
/// totalIndices`), one block of `(numMeshes + 1) * 16` bytes plus `totalIndices * 2` for a non-native
/// geometry (type 8) or world (type 7) (hint 0x30502; NULL -> NULL), the module's next serial number,
/// then per mesh `{numIndices, matIndex}` (material without a new reference) and, for non-native
/// objects, the indices in chunks of up to 0x100 dwords stored as u16. Any short read returns NULL
/// (the header leaks, the serial number stays consumed).
#[hook(addr = 0x758EC0, conv = cdecl, name = "_rpMeshRead", expect = "8B 4C 24 04 81 EC 20 04 00 00")]
pub fn rp_mesh_read(s: GPtr<u8>, obj: GPtr<u8>, ml: GPtr<u8>) -> GPtr<u8> {
    let mut hdr = [0u32; 3];
    if rw_stream_read_int32_call(s, GPtr::from_raw(hdr.as_mut_ptr()).cast(), 0xC).is_null() {
        return GPtr::NULL;
    }
    let non_native = |o: GPtr<u8>| {
        let t = rd::<u8>(o, 0);
        (t == 8 || t == 7) && rd::<u32>(o, 8) & FLAG_NATIVE == 0
    };
    let mut size = hdr[1].wrapping_add(1).wrapping_mul(16);
    if non_native(obj) {
        size = size.wrapping_add(hdr[2].wrapping_mul(2));
    }
    let h = rw_malloc(size, 0x30502);
    if h.is_null() {
        return GPtr::NULL;
    }
    wr::<u32>(h, 0, hdr[0]);
    wr::<u16>(h, 4, hdr[1] as u16);
    let mut mesh = h.byte_add(0x10);
    let mut idx = mesh.byte_add(hdr[1].wrapping_mul(12) as i32 as isize);
    wr::<u16>(h, 6, rd::<u16>(module(MESH_MODULE), 0));
    wr::<u32>(h, 8, hdr[2]);
    wr::<u32>(h, 0xC, 0);
    let m = module(MESH_MODULE);
    wr::<u16>(m, 0, rd::<u16>(m, 0).wrapping_add(1));
    let mut count = rd::<u16>(h, 4) as u32;
    let mut mh = [0u32; 2];
    let mut buf = [0u32; 0x100];
    let bp: GPtr<u8> = GPtr::from_raw(buf.as_mut_ptr()).cast();
    while count != 0 {
        count -= 1;
        if rw_stream_read_int32_call(s, GPtr::from_raw(mh.as_mut_ptr()).cast(), 8).is_null() {
            return GPtr::NULL;
        }
        wr::<u32>(mesh, 4, mh[0]);
        let mat = material_list_get_material_call(ml, mh[1] as i32);
        wr::<u32>(mesh, 8, mat.addr() as u32);
        wr::<u32>(mesh, 0, idx.addr() as u32);
        if non_native(obj) {
            let mut remaining = rd::<u32>(mesh, 4);
            while remaining > 0 {
                let chunk = remaining.min(0x100);
                if rw_stream_read_int32_call(s, bp, chunk * 4).is_null() {
                    return GPtr::NULL;
                }
                remaining -= chunk;
                for k in 0..chunk as isize {
                    idx.cast::<u16>().write(bp.byte_add(k * 4).cast::<u16>().read());
                    idx = idx.byte_add(2);
                }
            }
        }
        mesh = mesh.byte_add(0xC);
    }
    h
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_BIN_MESH_STREAM_READ,
    HOOK_RP_MESH_HEADER_CREATE,
    HOOK_RP_MESH_CLOSE,
    HOOK_RP_MESH_OPEN,
    HOOK_RP_BUILD_MESH_CREATE,
    HOOK_RP_BUILD_MESH_DESTROY,
    HOOK_RP_MESH_DESTROY,
    HOOK_RP_BUILD_MESH_ADD_TRIANGLE,
    HOOK_RP_MESH_HEADER_FOR_ALL_MESHES,
    HOOK_RP_MESH_READ,
    HOOK_RP_MESH_GET_NEXT_SERIAL_NUMBER,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
