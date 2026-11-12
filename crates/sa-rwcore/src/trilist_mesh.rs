//! RenderWare's triangle-list mesh generator (`_rpTriListMeshGenerate`) and its per-mesh index reordering
//! callback (batch `std8-rw-1`), ported from the 1.0 US bytes (RenderWare is not in gta-reversed). Unverified
//! (docs/verify/HOWTO.md).
//!
//! `RpBuildMesh`: +4 triangle count, +8 the `RpBuildMeshTriangle` array (0x14 bytes each: vertex indices u16 +0,
//! +2, +4, material +8). `RpMeshHeader`: +0 flags, +4 mesh count (u16), +6 serial number (u16), +8 total indices,
//! +0xC first-mesh offset, the `RpMesh`es from +0x10 (12 bytes: indices, index count, material), then the indices.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{RwError, module, p8, rd, rw_error_set, rw_error2, rw_free, rw_malloc, wr};

orig! {
    /// CRT `qsort(base, n, size, cmp)` (0x8247E0)
    fn crt_qsort(base: GPtr<u8>, n: u32, size: u32, cmp: u32) = cdecl 0x8247E0 => unported;
    /// `_rpMeshHeaderCreate(size)` (0x758920)
    fn mesh_header_create(size: u32) -> GPtr<u8> = cdecl 0x758920 => crate::mesh::rp_mesh_header_create;
    /// `_rpMeshHeaderForAllMeshes(header, callback, data)` (0x758D30)
    fn mesh_header_for_all_meshes(h: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x758D30 => crate::mesh::rp_mesh_header_for_all_meshes;
}

/// MESHMODULE offset global (+0 u16: the next mesh header serial number).
const MESH_MODULE: usize = 0xC9B8C0;
/// Memory hint of both arrays.
const HINT: u32 = 0x10502;
/// The triangle comparator (`qsort` callback: by material).
const MATERIAL_COMPARE: u32 = 0x759640;
/// [`trilist_reorder_mesh`]
const REORDER_CB: u32 = 0x75D730;

/// `RwErrorSet({2, _rwerror(0x80000013, size)})`: the out-of-memory error.
fn out_of_memory(size: u32) {
    let mut e = RwError {
        plugin_id: 2,
        error_code: 0,
    };
    e.error_code = rw_error2(0x8000_0013, size) as i32;
    rw_error_set(GPtr::from_raw(&mut e));
}

/// `_rpTriListMeshGenerate(RpBuildMesh* bm, void* data)` (0x75D530, cdecl). An array of pointers to the `n`
/// triangles (`RwMalloc(n * 4)`), `qsort`ed by material (0x759640); the material runs counted (at least 1, also for
/// `n < 2`); a mesh header of `6 * (n + 2 * runs) + 0x10` bytes (`_rpMeshHeaderCreate`): flags 0, one mesh, the
/// module's serial number (then incremented), `3n` indices, first mesh at 0. The meshes follow in run order, their
/// indices packed after the last mesh; a new mesh starts (count + 1) wherever the material changes. The loop body
/// runs once even for `n == 0`, as the original's (it reads the first pointer of the empty array). Then
/// `_rpMeshHeaderForAllMeshes(header, 0x75D730, NULL)`, the array freed, the header returned. Either allocation
/// failing: `RwErrorSet({2, _rwerror(0x80000013, size)})` and null (the array freed first in the second case).
#[hook(addr = 0x75D530, conv = cdecl, name = "_rpTriListMeshGenerate", expect = "83 EC 08 8B 0D 24 7B C9 00",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn trilist_mesh_generate(bm: GPtr<u8>, _data: u32) -> GPtr<u8> {
    let n = rd::<u32>(bm, 4);
    let list = rw_malloc(n << 2, HINT);
    if list.is_null() {
        out_of_memory(n << 2);
        return GPtr::NULL;
    }
    let tri = |i: u32| p8(rd::<u32>(list, i as isize * 4));
    for i in 0..n {
        wr::<u32>(list, i as isize * 4, rd::<u32>(bm, 8).wrapping_add(i * 0x14));
    }
    crt_qsort(list, rd::<u32>(bm, 4), 4, MATERIAL_COMPARE);
    let n = rd::<u32>(bm, 4);
    let mut runs = 1u32;
    if n >= 2 {
        let mut mat = rd::<u32>(tri(0), 8);
        for i in 1..n {
            let m = rd::<u32>(tri(i), 8);
            if m != mat {
                mat = m;
                runs += 1;
            }
        }
    }
    let size = (n.wrapping_add(runs * 2))
        .wrapping_mul(3)
        .wrapping_mul(2)
        .wrapping_add(0x10);
    let h = mesh_header_create(size);
    if h.is_null() {
        rw_free(list);
        out_of_memory(size);
        return GPtr::NULL;
    }
    wr::<u32>(h, 0, 0);
    wr::<u16>(h, 4, 1);
    let serial = module(MESH_MODULE);
    wr::<u32>(h, 0xC, 0);
    wr::<u16>(h, 6, rd::<u16>(serial, 0));
    wr::<u32>(h, 8, rd::<u32>(bm, 4).wrapping_mul(3));
    let serial = module(MESH_MODULE);
    wr::<u16>(serial, 0, rd::<u16>(serial, 0).wrapping_add(1));
    let mut mesh = h.byte_add(0x10);
    let mut idx = mesh.byte_add(runs as isize * 12);
    wr::<u32>(mesh, 4, 0);
    wr::<u32>(mesh, 0, idx.addr() as u32);
    wr::<u32>(mesh, 8, rd::<u32>(tri(0), 8));
    let mut i = 0u32;
    loop {
        let t = tri(i);
        if rd::<u32>(t, 8) != rd::<u32>(mesh, 8) {
            mesh = mesh.byte_add(12);
            wr::<u32>(mesh, 4, 0);
            wr::<u32>(mesh, 0, idx.addr() as u32);
            wr::<u32>(mesh, 8, rd::<u32>(tri(i), 8));
            wr::<u16>(h, 4, rd::<u16>(h, 4).wrapping_add(1));
        }
        for k in 0..3 {
            wr::<u16>(idx, k * 2, rd::<u16>(tri(i), k * 2));
        }
        idx = idx.byte_add(6);
        wr::<u32>(mesh, 4, rd::<u32>(mesh, 4).wrapping_add(3));
        i += 1;
        if i >= rd::<u32>(bm, 4) {
            break;
        }
    }
    mesh_header_for_all_meshes(h, REORDER_CB, 0);
    rw_free(list);
    h
}

/// The triangle-list mesh callback (0x75D730, cdecl `(RpMesh* mesh, RpMeshHeader*, void*)`, returns `mesh`):
/// reorders the mesh's triangles in passes so that no vertex repeats within a pass. A bit set over the vertex
/// indices (`((OR of all indices) + 31) >> 3` bytes) and a copy of the indices are allocated (either failing:
/// the mesh is left as it is). Each pass clears the bits and walks the copy: a triangle not yet taken (taken ones
/// are 0xFFFF ×3) whose three vertices are all clear is appended to the mesh's index array, its vertices set and its
/// copy marked taken. Passes repeat until every triangle is taken; both arrays are freed.
#[hook(addr = 0x75D730, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "83 EC 20 53 55 56 57 8B 7C 24 34",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "unnamed in the catalogue; the callback _rpTriListMeshGenerate passes to _rpMeshHeaderForAllMeshes")]
pub fn trilist_reorder_mesh(mesh: GPtr<u8>, _header: GPtr<u8>, _data: u32) -> GPtr<u8> {
    let out = p8(rd::<u32>(mesh, 0));
    let count = rd::<u32>(mesh, 4);
    let tris = count / 3;
    let mut all = 0u32;
    for i in 0..count {
        all |= rd::<u16>(out, i as isize * 2) as u32;
    }
    let bytes = (all + 0x1F) >> 3;
    let bits = rw_malloc(bytes, HINT);
    if bits.is_null() {
        return mesh;
    }
    let copy = rw_malloc(rd::<u32>(mesh, 4) * 2, HINT);
    if copy.is_null() {
        rw_free(bits);
        return mesh;
    }
    let len = rd::<u32>(mesh, 4) * 2;
    for k in 0..len as isize {
        copy.byte_add(k).write(out.byte_add(k).read());
    }
    let mut placed = 0u32;
    let mut dst = out;
    let bit = |v: u16| -> (GPtr<u8>, u32) { (bits.byte_add((v as isize >> 5) * 4), 1u32 << (v & 0x1F)) };
    if tris != 0 {
        loop {
            for k in 0..bytes as isize {
                bits.byte_add(k).write(0);
            }
            for t in 0..tris {
                let c = copy.byte_add(t as isize * 6);
                let (v0, v1, v2) = (rd::<u16>(c, 0), rd::<u16>(c, 2), rd::<u16>(c, 4));
                if v0 & v2 & v1 == 0xFFFF {
                    continue;
                }
                let taken = |v: u16| {
                    let (w, b) = bit(v);
                    rd::<u32>(w, 0) & b != 0
                };
                if taken(v0) || taken(v1) || taken(v2) {
                    continue;
                }
                for v in [v0, v1, v2] {
                    let (w, b) = bit(v);
                    wr::<u32>(w, 0, rd::<u32>(w, 0) | b);
                }
                wr::<u16>(dst, 0, v0);
                wr::<u16>(dst, 2, v1);
                wr::<u16>(dst, 4, v2);
                dst = dst.byte_add(6);
                wr::<u16>(c, 4, 0xFFFF);
                wr::<u16>(c, 2, 0xFFFF);
                wr::<u16>(c, 0, 0xFFFF);
                placed += 1;
            }
            if placed >= tris {
                break;
            }
        }
    }
    rw_free(copy);
    rw_free(bits);
    mesh
}

pub const HOOKS: &[HookReg] = &[HOOK_TRILIST_MESH_GENERATE, HOOK_TRILIST_REORDER_MESH];
