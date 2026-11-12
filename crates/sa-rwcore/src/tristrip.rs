//! RpMesh tri-strip generation (STD-5): `RpBuildMeshGenerateDefaultTriStrip` (0x7591B0) and the
//! generator behind it (0x7591D0) with its helpers, ported from the 1.0 US bytes (no gta-reversed
//! counterpart: RenderWare). Reached whenever a geometry built at run time is unlocked
//! (`_rpMeshOptimise`): plant models, road signs, the clothes builder's player model.
//!
//! Layouts (all from the bytes):
//! - build-mesh triangle (0x14, `RpBuildMesh` +8): +0/+2/+4 u16 vertex indices, +8 material, +0xC
//!   matIndex, +0xE textureIndex, +0x10 rasterIndex, +0x12 pipelineIndex (see `mesh.rs`);
//! - strip (0x10, from the temporary freelist at MESHMODULE +4): +0 u16 buffer (allocated as
//!   `capacity * 4` bytes), +4 count, +8 capacity, +0xC next;
//! - triangle node (0x24, freelist): +0 triangle index, +4/+8/+0xC edges (v0-v1, v1-v2, v2-v0), +0x10
//!   next / +0x14 previous in its bucket, +0x18 used, +0x1C used in the trial pass, u8 +0x20 number of
//!   unused neighbours (the bucket index, 0..3);
//! - edge (0x10, freelist): +0 u16 first vertex, +2 u16 second vertex, +4 first triangle, +8 second
//!   triangle (the one that found it reversed), +0xC next in the edge list.
//!
//! Every call between these functions goes through `orig!` (each keeps its own hook); the CRT `qsort`
//! (0x8247E0) sorts the triangles with the comparator 0x759640.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{module, p8, rd, rw_free, rw_malloc, rw_memory_alloc, rw_memory_free, wr};

/// MESHMODULE offset global (+0 u16 next serial number, +4 the temporary strip freelist).
const MESH_MODULE: usize = 0xC9B8C0;
/// Memory hints.
const HINT_MESH: u32 = 0x10502;
const HINT_MESH_ARRAY: u32 = 0x0101_0502;
const HINT_STRIP: u32 = 0x30502;
/// The triangle comparator (`qsort` callback).
const MATERIAL_COMPARE: u32 = 0x759640;

orig! {
    /// The generator `(buildMesh, winding, joinParity)` (0x7591D0)
    fn generate_tristrip_call(bm: GPtr<u8>, winding: u32, join_parity: u32) -> GPtr<u8> = cdecl 0x7591D0 => generate_tristrip;
    /// One material's strips `(tris, n, &list, winding)` (0x759790)
    fn strip_material_call(tris: GPtr<u8>, n: u32, list: GPtr<u8>, winding: u32) -> u32 = cdecl 0x759790 => strip_material;
    /// The adjacency build `(n, freelists[2], &edgeList, tris)` (0x75A090)
    fn build_adjacency_call(n: u32, fls: GPtr<u8>, edges: GPtr<u8>, tris: GPtr<u8>) -> GPtr<u8> = cdecl 0x75A090 => build_adjacency;
    /// Edge lookup / creation `(edgeFreeList, &edgeList, va, vb, triIndex, nodes)` (0x75A1B0)
    fn add_edge_call(fl: GPtr<u8>, edges: GPtr<u8>, va: u32, vb: u32, index: u32, nodes: GPtr<u8>) -> GPtr<u8> = cdecl 0x75A1B0 => add_edge;
    /// Marks a triangle used `(node, buckets, pass)` (0x75A250)
    fn mark_used_call(t: GPtr<u8>, buckets: GPtr<u8>, pass: u32) = cdecl 0x75A250 => mark_used;
    /// Extends a strip across an edge `(strip, edge, buckets, tris, pass)` (0x75A370)
    fn extend_strip_call(s: GPtr<u8>, edge: GPtr<u8>, buckets: GPtr<u8>, tris: GPtr<u8>, pass: u32) -> u32 = cdecl 0x75A370 => extend_strip;
    /// Joins a strip list into one strip `(&list, joinParity)` (0x75A660)
    fn join_strips_call(list: GPtr<u8>, join_parity: u32) -> u32 = cdecl 0x75A660 => join_strips;
    /// CRT `qsort(base, n, size, cmp)` (0x8247E0)
    fn crt_qsort(base: GPtr<u8>, n: u32, size: u32, cmp: u32) = cdecl 0x8247E0 => unported;
    /// `_rwFreeListCreate(entrySize, entriesPerBlock, alignment, hint)` (0x801980)
    fn free_list_create(size: u32, per_block: u32, align: u32, hint: u32) -> GPtr<u8> = cdecl 0x801980 => crate::memory::rw_freelist_create;
    /// `RwFreeListDestroy(fl)` (0x801B80)
    fn free_list_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => crate::memory::rw_freelist_destroy;
    /// The mesh header allocation `RwMalloc(size, 0x30502)` (0x758920)
    fn mesh_header_alloc(size: u32) -> GPtr<u8> = cdecl 0x758920 => unported;
}

/// A pointer field.
fn ptr(p: GPtr<u8>, off: isize) -> GPtr<u8> {
    p8(rd::<u32>(p, off))
}

fn addr(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// The temporary strip freelist (MESHMODULE +4), re-read at every use as the original does.
fn strip_free_list() -> GPtr<u8> {
    ptr(module(MESH_MODULE), 4)
}

/// `t != NULL && t->flag == 0` as 0/1 (the original's per-neighbour test).
fn free_tri(t: GPtr<u8>, flag: isize) -> u32 {
    u32::from(!t.is_null() && rd::<u32>(t, flag) == 0)
}

/// Unused triangles on both sides of an edge (the triangle itself counts when unused).
fn free_on(e: GPtr<u8>, flag: isize) -> u32 {
    free_tri(ptr(e, 4), flag) + free_tri(ptr(e, 8), flag)
}

/// The used flag a pass tests: the trial passes (< 4) use +0x1C, the final one +0x18.
fn flag_of(pass: u32) -> isize {
    if (pass as i32) < 4 { 0x1C } else { 0x18 }
}

/// `RpBuildMeshGenerateDefaultTriStrip(buildMesh, data)` (0x7591B0): the generator with winding 0
/// (one pass with the start rotation chosen from the neighbours) and parity-preserving joins.
#[hook(addr = 0x7591B0, conv = cdecl, name = "RpBuildMeshGenerateDefaultTriStrip", expect = "8B 44 24 04 6A 01 6A 00",
       route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn generate_default_tristrip(bm: GPtr<u8>, _data: u32) -> GPtr<u8> {
    generate_tristrip_call(bm, 0, 1)
}

/// The tri-strip generator (0x7591D0, cdecl `(buildMesh, winding, joinParity)`): sorts pointers to
/// the build mesh's triangles with `qsort` (comparator 0x759640), splits them into runs of one
/// material (`groups`: 12-byte entries {0, count, material}), creates the strip freelist (0x10 bytes,
/// `n / 10 + 5` per block) at MESHMODULE +4, and per run copies its triangles, strips them
/// (0x759790), joins the strips (0x75A660) and keeps each resulting strip as a mesh piece (`count *
/// 2 + 0xC` bytes: indices pointer, count, material). Then the header (`0x10 + Σ(0xC + 2 count)`
/// bytes, flags 1 = tri-strip, the module's serial number, post-incremented) with the meshes and their
/// indices; the temporaries are freed. NULL when the pointer array cannot be allocated.
#[hook(addr = 0x7591D0, conv = cdecl, name = "FUN_007591d0", expect = "83 EC 1C 55 8B 0D 24 7B C9 00",
       route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn generate_tristrip(bm: GPtr<u8>, winding: u32, join_parity: u32) -> GPtr<u8> {
    let sorted = rw_malloc(rd::<u32>(bm, 4) << 2, HINT_MESH);
    if sorted.is_null() {
        return p8(0);
    }
    let mut i = 0u32;
    while i < rd::<u32>(bm, 4) {
        wr::<u32>(sorted, 4 * i as isize, rd::<u32>(bm, 8).wrapping_add(0x14 * i));
        i += 1;
    }
    crt_qsort(sorted, rd::<u32>(bm, 4), 4, MATERIAL_COMPARE);
    let sorted_at = |i: u32| ptr(sorted, 4 * i as isize);
    let n = rd::<u32>(bm, 4);
    let mut ngroups = 1u32;
    if n >= 2 {
        let mut m = rd::<u32>(sorted_at(0), 8);
        for i in 1..n {
            let mi = rd::<u32>(sorted_at(i), 8);
            if mi != m {
                ngroups += 1;
                m = mi;
            }
        }
    }
    let out = rw_malloc(ngroups * 4, HINT_MESH_ARRAY);
    let groups = rw_malloc(ngroups * 12, HINT_MESH);
    wr::<u32>(groups, 8, rd::<u32>(sorted_at(0), 8));
    wr::<u32>(groups, 0, 0);
    wr::<u32>(groups, 4, 0);
    let mut ng = 1u32;
    if rd::<u32>(bm, 4) >= 2 {
        // `cur` = &groups[k].count
        let mut cur = groups.byte_add(4);
        let mut i = 0u32;
        while i < rd::<u32>(bm, 4) - 1 {
            let next = rd::<u32>(sorted_at(i + 1), 8);
            if rd::<u32>(sorted_at(i), 8) != next {
                wr::<u32>(cur, 0x10, next);
                wr::<u32>(cur, 8, 0);
                wr::<u32>(cur, 0xC, i + 1);
                wr::<u32>(cur, 0, i.wrapping_sub(rd::<u32>(cur, 0)) + 1);
                ng += 1;
                cur = cur.byte_add(0xC);
            }
            i += 1;
        }
    }
    let last = groups.byte_add(ng as isize * 12 - 8);
    wr::<u32>(last, 0, rd::<u32>(bm, 4).wrapping_sub(rd::<u32>(last, 0)));
    let fl = free_list_create(0x10, rd::<u32>(bm, 4) / 10 + 5, 4, HINT_MESH);
    wr::<u32>(module(MESH_MODULE), 4, addr(fl));

    let mut list = 0u32;
    let list_p: GPtr<u8> = GPtr::from_raw(&mut list as *mut u32).cast();
    let mut cursor = 0u32;
    let mut nout = 0u32;
    for k in 0..ng {
        let entry = groups.byte_add(12 * k as isize + 4);
        let tris = rw_malloc(rd::<u32>(entry, 0) * 0x14, HINT_MESH);
        let mut j = 0u32;
        while j < rd::<u32>(entry, 0) {
            let src = sorted_at(cursor);
            for w in 0..5 {
                wr::<u32>(tris, (0x14 * j + 4 * w) as isize, rd::<u32>(src, 4 * w as isize));
            }
            cursor += 1;
            j += 1;
        }
        strip_material_call(tris, rd::<u32>(entry, 0), list_p, winding);
        join_strips_call(list_p, join_parity);
        let mut s = p8(list);
        while !s.is_null() {
            let m = rw_malloc(rd::<u32>(s, 4) * 2 + 0xC, HINT_MESH);
            wr::<u32>(m, 8, rd::<u32>(entry, 4));
            wr::<u32>(m, 4, rd::<u32>(s, 4));
            wr::<u32>(m, 0, addr(m) + 0xC);
            let mut j = 0u32;
            while j < rd::<u32>(m, 4) {
                wr::<u16>(ptr(m, 0), 2 * j as isize, rd::<u16>(ptr(s, 0), 2 * j as isize));
                j += 1;
            }
            wr::<u32>(out, 4 * (nout & 0xFFFF) as isize, addr(m));
            nout += 1;
            s = ptr(s, 0xC);
        }
        while list != 0 {
            let s = p8(list);
            list = rd::<u32>(s, 0xC);
            rw_free(ptr(s, 0));
            wr::<u32>(s, 0, 0);
            rw_memory_free(strip_free_list(), s);
        }
        rw_free(tris);
    }
    free_list_destroy(strip_free_list());
    let count = nout & 0xFFFF;
    wr::<u32>(module(MESH_MODULE), 4, 0);
    let mut total = 0u32;
    let mut size = 0x10u32;
    for i in 0..count {
        let c = rd::<u32>(ptr(out, 4 * i as isize), 4);
        total = total.wrapping_add(c);
        size = size.wrapping_add(0xC + 2 * c);
    }
    let h = mesh_header_alloc(size);
    wr::<u32>(h, 0, 1);
    wr::<u16>(h, 4, nout as u16);
    let serial = rd::<u16>(module(MESH_MODULE), 0);
    wr::<u32>(h, 0xC, 0);
    wr::<u16>(h, 6, serial);
    wr::<u32>(h, 8, total);
    let mm = module(MESH_MODULE);
    wr::<u16>(mm, 0, rd::<u16>(mm, 0).wrapping_add(1));
    let mut mesh = h.byte_add(0x10);
    let mut idx = addr(h) + 0x10 + 12 * count;
    for i in 0..count {
        let piece = ptr(out, 4 * i as isize);
        wr::<u32>(mesh, 0, idx);
        wr::<u32>(mesh, 4, rd::<u32>(piece, 4));
        wr::<u32>(mesh, 8, rd::<u32>(piece, 8));
        let bytes = rd::<u32>(piece, 4).wrapping_mul(2);
        let src = ptr(piece, 0);
        for b in 0..bytes {
            wr::<u8>(p8(idx), b as isize, rd::<u8>(src, b as isize));
        }
        idx = idx.wrapping_add(bytes);
        rw_free(piece);
        wr::<u32>(out, 4 * i as isize, 0);
        mesh = mesh.byte_add(0xC);
    }
    rw_free(sorted);
    rw_free(out);
    rw_free(groups);
    h
}

/// The material's transparency bits for the comparator: 0x10 when its texture's raster format
/// (`raster +0x23` low nibble << 8) is 0x100 / 0x300 / 0x500 (1555, 4444, 8888), or'ed with 0x10 when
/// the colour's alpha (+7) is not 0xFF; 0 for no material.
fn transparency(mat: GPtr<u8>) -> u32 {
    let mut f = 0u32;
    if mat.is_null() {
        return 0;
    }
    let tex = ptr(mat, 0);
    if !tex.is_null() {
        let fmt = (u32::from(rd::<u8>(ptr(tex, 0), 0x23)) & 0xF) << 8;
        if fmt == 0x100 || fmt == 0x300 || fmt == 0x500 {
            f = 0x10;
        }
    }
    if rd::<u8>(mat, 7) != 0xFF {
        f |= 0x10;
    }
    f
}

/// The `qsort` comparator of the generator (0x759640, cdecl, on pointers to build-mesh
/// triangles): 0 for the same material; else a key per side, `key(b) - key(a)` where `key(x)` =
/// transparency of the *other* side's material (0x10) | 8 (x.rasterIndex < y.rasterIndex) | 4
/// (pipelineIndex <) | 2 (textureIndex <) | 1 (matIndex <), all u16 comparisons, as the bytes compute
/// it (each side's transparency flag is accumulated in the other argument's slot).
#[hook(addr = 0x759640, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "83 EC 0C 8B 44 24 10 8B 4C 24 14",
       route = "L2", hazards = "none", recs = "new-game")]
pub fn material_compare(pa: GPtr<u8>, pb: GPtr<u8>) -> i32 {
    let a = ptr(pa, 0);
    let b = ptr(pb, 0);
    let (ma, mb) = (ptr(a, 8), ptr(b, 8));
    if addr(ma) == addr(mb) {
        return 0;
    }
    // slot of argument b holds a's transparency, slot of a holds b's
    let ta = transparency(ma);
    let tb = transparency(mb);
    let lt = |x: u16, y: u16, bit: u32| if x < y { bit } else { 0 };
    let f = |p: GPtr<u8>, o: isize| rd::<u16>(p, o);
    let key_a = lt(f(a, 0x10), f(b, 0x10), 8)
        | lt(f(a, 0x12), f(b, 0x12), 4)
        | lt(f(a, 0xE), f(b, 0xE), 2)
        | lt(f(a, 0xC), f(b, 0xC), 1)
        | tb;
    let key_b = lt(f(b, 0x10), f(a, 0x10), 8)
        | lt(f(b, 0x12), f(a, 0x12), 4)
        | lt(f(b, 0xE), f(a, 0xE), 2)
        | lt(f(b, 0xC), f(a, 0xC), 1)
        | ta;
    key_b.wrapping_sub(key_a) as i32
}

/// Moves a triangle node out of its bucket (`buckets[node.count]`).
fn bucket_unlink(t: GPtr<u8>, buckets: GPtr<u8>) {
    let slot = 4 * isize::from(rd::<u8>(t, 0x20));
    if rd::<u32>(buckets, slot) == addr(t) {
        wr::<u32>(buckets, slot, rd::<u32>(t, 0x10));
        let h = ptr(buckets, 4 * isize::from(rd::<u8>(t, 0x20)));
        if !h.is_null() {
            wr::<u32>(h, 0x14, 0);
        }
    } else {
        let next = ptr(t, 0x10);
        if !next.is_null() {
            wr::<u32>(next, 0x14, rd::<u32>(t, 0x14));
        }
        let prev = ptr(t, 0x14);
        if !prev.is_null() {
            wr::<u32>(prev, 0x10, rd::<u32>(t, 0x10));
        }
    }
}

/// Strips one material's triangles (0x759790, cdecl `(tris, n, &list, winding)`): builds the
/// adjacency (0x75A090), puts every node at the front of the bucket of its neighbour count, then
/// until every triangle is in a strip: an isolated triangle (bucket 0) becomes a 3-index strip; else
/// the first node of the lowest non-empty bucket starts a strip. Its rotation comes from the unused
/// neighbours across its edges; with `winding` the passes 0..2 try the three rotations from it (trial
/// flags +0x1C) and pass 3 keeps the best, without it a single pass 3 uses it. A pass marks the
/// triangle (0x75A250), extends the strip forwards across one edge (0x75A370), then backwards across
/// the other when that side has an unused triangle (an odd backward strip gets a repeated index);
/// the final pass emits the backward part reversed followed by the forward strip (or the forward
/// strip alone) at the front of `list`. Returns 1. The buffers copy `count * 4` bytes of the forward
/// strip (twice its indices, as the original's `rep movsd`).
#[hook(addr = 0x759790, conv = cdecl, name = "FUN_00759790", expect = "83 EC 4C 8B 44 24 50 53",
       unverified = "std5-tristrip-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn strip_material(tris: GPtr<u8>, n: u32, list: GPtr<u8>, winding: u32) -> u32 {
    let mut buckets = [0u32; 4];
    let mut edges = 0u32;
    let mut fls = [0u32; 2];
    let bp: GPtr<u8> = GPtr::from_raw(buckets.as_mut_ptr()).cast();
    let arr = build_adjacency_call(
        n,
        GPtr::from_raw(fls.as_mut_ptr()).cast(),
        GPtr::from_raw(&mut edges as *mut u32).cast(),
        tris,
    );
    let node = |i: u32| ptr(arr, 4 * i as isize);
    for i in 0..n {
        let t = node(i);
        let c = 4 * isize::from(rd::<u8>(t, 0x20));
        wr::<u32>(t, 0x10, rd::<u32>(bp, c));
        let nx = ptr(t, 0x10);
        if !nx.is_null() {
            wr::<u32>(nx, 0x14, addr(t));
        }
        wr::<u32>(bp, 4 * isize::from(rd::<u8>(t, 0x20)), addr(t));
        wr::<u32>(node(i), 0x14, 0);
    }
    let cap = 2 * n + 2;
    let s1 = rw_memory_alloc(strip_free_list(), HINT_MESH);
    wr::<u32>(s1, 4, 0);
    wr::<u32>(s1, 8, cap);
    wr::<u32>(s1, 0, addr(rw_malloc(cap * 4, HINT_MESH)));
    let s2 = rw_memory_alloc(strip_free_list(), HINT_MESH);
    wr::<u32>(s2, 8, cap);
    wr::<u32>(s2, 4, 0);
    wr::<u32>(s2, 0, addr(rw_malloc(cap * 4, HINT_MESH)));

    let new_strip = |cap: u32, count: u32| -> GPtr<u8> {
        let st = rw_memory_alloc(strip_free_list(), HINT_STRIP);
        wr::<u32>(st, 0xC, rd::<u32>(list, 0));
        wr::<u32>(list, 0, addr(st));
        wr::<u32>(st, 8, cap);
        wr::<u32>(st, 4, count);
        st
    };
    let copy_bytes = |dst: u32, src: GPtr<u8>, bytes: u32| {
        for b in 0..bytes {
            wr::<u8>(p8(dst), b as isize, rd::<u8>(src, b as isize));
        }
    };

    let mut stripped = 0u32; // [esp+0x14]
    let (mut first_edge, mut second_edge) = (0u32, 0u32); // [esp+0x20], [esp+0x10]
    if n != 0 {
        loop {
            let start_pass: u32 = if winding != 0 { 0 } else { 3 };
            let mut best = 0u32;
            if buckets[0] != 0 {
                // an isolated triangle
                let t = p8(buckets[0]);
                let idx = rd::<u32>(t, 0);
                let st = rw_memory_alloc(strip_free_list(), HINT_STRIP);
                wr::<u32>(st, 0xC, rd::<u32>(list, 0));
                wr::<u32>(list, 0, addr(st));
                wr::<u32>(st, 8, 3);
                wr::<u32>(st, 4, 3);
                let buf = rw_malloc(0xC, HINT_STRIP);
                wr::<u32>(st, 0, addr(buf));
                let tri = tris.byte_add(0x14 * idx as isize);
                wr::<u16>(ptr(st, 0), 0, rd::<u16>(tri, 0));
                wr::<u16>(ptr(st, 0), 2, rd::<u16>(tri, 2));
                wr::<u16>(ptr(st, 0), 4, rd::<u16>(tri, 4));
                let t = p8(buckets[0]);
                wr::<u32>(t, 0x18, 1);
                wr::<u32>(t, 0x1C, 1);
                buckets[0] = rd::<u32>(t, 0x10);
                if buckets[0] != 0 {
                    wr::<u32>(p8(buckets[0]), 0x14, 0);
                }
                stripped += 1;
            } else {
                let mut bi = 1usize;
                while buckets[bi] == 0 {
                    bi += 1;
                }
                let head = |b: &[u32; 4]| p8(b[bi]);
                let t = head(&buckets);
                let (e0, e1, e2) = (ptr(t, 4), ptr(t, 8), ptr(t, 0xC));
                let used = 0x18;
                let a = free_on(e2, used);
                let c = free_on(e0, used);
                let g = free_on(e1, used);
                let rot = if a >= 2 && g >= 2 {
                    1
                } else if c >= 2 && a >= 2 {
                    2
                } else if g >= 2 && c >= 2 {
                    0
                } else if c > g {
                    1 + u32::from(c > a)
                } else {
                    u32::from(g <= a)
                };
                let base = stripped;
                let mut best_rot = rot;
                let sel = rot;
                let mut pass = start_pass;
                loop {
                    for j in 1..=n {
                        let tj = node(j - 1);
                        wr::<u32>(tj, 0x1C, rd::<u32>(tj, 0x18));
                    }
                    let p = pass;
                    pass += 1;
                    let r = match p {
                        0 => sel % 3,
                        1 => (sel + 1) % 3,
                        2 => (sel + 2) % 3,
                        _ => best_rot,
                    };
                    let t = head(&buckets);
                    match r {
                        0 => (first_edge, second_edge) = (rd::<u32>(t, 8), rd::<u32>(t, 4)),
                        1 => (first_edge, second_edge) = (rd::<u32>(t, 0xC), rd::<u32>(t, 8)),
                        2 => (first_edge, second_edge) = (rd::<u32>(t, 4), rd::<u32>(t, 0xC)),
                        _ => {}
                    }
                    let tri = tris.byte_add(0x14 * rd::<u32>(t, 0) as isize);
                    let b1 = ptr(s1, 0);
                    wr::<u16>(b1, 0, rd::<u16>(tri, 2 * (r % 3) as isize));
                    wr::<u16>(b1, 2, rd::<u16>(tri, 2 * ((r + 1) % 3) as isize));
                    wr::<u16>(b1, 4, rd::<u16>(tri, 2 * ((r + 2) % 3) as isize));
                    wr::<u32>(s1, 4, 3);
                    mark_used_call(head(&buckets), bp, pass);
                    let added = extend_strip_call(s1, p8(first_edge), bp, tris, pass);
                    let mut score = added.wrapping_add(base).wrapping_add(1);
                    stripped = score;
                    let back = free_on(p8(second_edge), flag_of(pass)) > 0;
                    if back {
                        let b2 = ptr(s2, 0);
                        wr::<u16>(b2, 0, rd::<u16>(ptr(s1, 0), 2));
                        wr::<u16>(ptr(s2, 0), 2, rd::<u16>(ptr(s1, 0), 0));
                        wr::<u32>(s2, 4, 2);
                        score = score.wrapping_add(extend_strip_call(s2, p8(second_edge), bp, tris, pass));
                        stripped = score;
                        let c2 = rd::<u32>(s2, 4);
                        if c2 & 1 != 0 {
                            let b2 = ptr(s2, 0);
                            wr::<u16>(b2, 2 * c2 as isize, rd::<u16>(b2, 2 * c2 as isize - 4));
                            wr::<u32>(s2, 4, rd::<u32>(s2, 4) + 1);
                        }
                    }
                    if score > best {
                        best = score;
                        best_rot = r;
                    }
                    if (pass as i32) < 4 {
                        continue;
                    }
                    if back {
                        let cap = rd::<u32>(s1, 4).wrapping_add(rd::<u32>(s2, 4)).wrapping_sub(2);
                        let st = new_strip(cap, 0);
                        wr::<u32>(st, 0, addr(rw_malloc(cap << 2, HINT_STRIP)));
                        while rd::<u32>(s2, 4) > 2 {
                            let c2 = rd::<u32>(s2, 4);
                            let v = rd::<u16>(ptr(s2, 0), 2 * c2 as isize - 2);
                            wr::<u16>(ptr(st, 0), 2 * rd::<u32>(st, 4) as isize, v);
                            wr::<u32>(st, 4, rd::<u32>(st, 4) + 1);
                            wr::<u32>(s2, 4, c2 - 1);
                        }
                        let dst = rd::<u32>(st, 0).wrapping_add(2 * rd::<u32>(st, 4));
                        copy_bytes(dst, ptr(s1, 0), rd::<u32>(s1, 4) << 2);
                        wr::<u32>(st, 4, rd::<u32>(st, 8));
                    } else {
                        let c1 = rd::<u32>(s1, 4);
                        let st = new_strip(c1, c1);
                        let buf = rw_malloc(rd::<u32>(s1, 4) << 2, HINT_STRIP);
                        wr::<u32>(st, 0, addr(buf));
                        copy_bytes(addr(buf), ptr(s1, 0), rd::<u32>(s1, 4) << 2);
                    }
                    break;
                }
            }
            let _ = best;
            if stripped >= n {
                break;
            }
        }
    }
    let _ = (first_edge, second_edge);
    rw_free(ptr(s2, 0));
    wr::<u32>(s2, 0, 0);
    rw_memory_free(strip_free_list(), s2);
    rw_free(ptr(s1, 0));
    wr::<u32>(s1, 0, 0);
    rw_memory_free(strip_free_list(), s1);
    let mut e = edges;
    while e != 0 {
        let nx = rd::<u32>(p8(e), 0xC);
        rw_memory_free(p8(fls[1]), p8(e));
        e = nx;
    }
    free_list_destroy(p8(fls[1]));
    for i in 0..n {
        rw_memory_free(p8(fls[0]), node(i));
        wr::<u32>(arr, 4 * i as isize, 0);
    }
    free_list_destroy(p8(fls[0]));
    rw_free(arr);
    1
}

/// The adjacency build (0x75A090, cdecl `(n, freelists[2], &edgeList, tris)`): the node pointer
/// array (`n * 4`, hint 0x30502), the node freelist (0x24, `n` per block) and the edge freelist
/// (0x10, `n / 2 + 1` per block) into `freelists`; per triangle a node {index, count 0, links 0,
/// used 0} and its three edges (v0-v1, v1-v2, v2-v0) through 0x75A1B0. Returns the array.
#[hook(addr = 0x75A090, conv = cdecl, name = "FUN_0075a090", expect = "8B 0D 24 7B C9 00 53 55",
       unverified = "std5-tristrip-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn build_adjacency(n: u32, fls: GPtr<u8>, edges: GPtr<u8>, tris: GPtr<u8>) -> GPtr<u8> {
    let arr = rw_malloc(n * 4, HINT_STRIP);
    wr::<u32>(fls, 0, addr(free_list_create(0x24, n, 4, HINT_STRIP)));
    wr::<u32>(fls, 4, addr(free_list_create(0x10, (n >> 1) + 1, 4, HINT_STRIP)));
    for i in 0..n {
        let t = rw_memory_alloc(ptr(fls, 0), HINT_STRIP);
        wr::<u32>(arr, 4 * i as isize, addr(t));
        wr::<u8>(t, 0x20, 0);
        let t = ptr(arr, 4 * i as isize);
        wr::<u32>(t, 0, i);
        wr::<u32>(t, 0x14, 0);
        wr::<u32>(t, 0x10, 0);
        wr::<u32>(t, 0x18, 0);
        let tri = tris.byte_add(0x14 * i as isize);
        let v = |k: isize| u32::from(rd::<u16>(tri, 2 * k));
        let e = add_edge_call(ptr(fls, 4), edges, v(0), v(1), i, arr);
        wr::<u32>(ptr(arr, 4 * i as isize), 4, addr(e));
        let e = add_edge_call(ptr(fls, 4), edges, v(1), v(2), i, arr);
        wr::<u32>(ptr(arr, 4 * i as isize), 8, addr(e));
        let e = add_edge_call(ptr(fls, 4), edges, v(2), v(0), i, arr);
        wr::<u32>(ptr(arr, 4 * i as isize), 0xC, addr(e));
    }
    arr
}

/// Edge lookup / creation (0x75A1B0, cdecl `(edgeFreeList, &edgeList, va, vb, index, nodes)`; the
/// vertices are compared and stored as u16): an edge (vb, va) of the list without a second
/// triangle gets `nodes[index]` as its second triangle and both triangles' neighbour counts go up;
/// else a new edge {va, vb, nodes[index], NULL} goes to the front of the list. Returns the edge.
#[hook(addr = 0x75A1B0, conv = cdecl, name = "FUN_0075a1b0", expect = "53 66 8B 5C 24 10 56",
       unverified = "std5-tristrip-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn add_edge(fl: GPtr<u8>, edges: GPtr<u8>, va: u32, vb: u32, index: u32, nodes: GPtr<u8>) -> GPtr<u8> {
    let (va, vb) = (va as u16, vb as u16);
    let mut e = ptr(edges, 0);
    while !e.is_null() {
        if rd::<u16>(e, 2) == va && rd::<u16>(e, 0) == vb && rd::<u32>(e, 8) == 0 {
            let t1 = ptr(e, 4);
            wr::<u8>(t1, 0x20, rd::<u8>(t1, 0x20).wrapping_add(1));
            let t = ptr(nodes, 4 * index as isize);
            wr::<u8>(t, 0x20, rd::<u8>(t, 0x20).wrapping_add(1));
            wr::<u32>(e, 8, rd::<u32>(nodes, 4 * index as isize));
            return e;
        }
        e = ptr(e, 0xC);
    }
    let e = rw_memory_alloc(fl, HINT_STRIP);
    wr::<u16>(e, 0, va);
    wr::<u16>(e, 2, vb);
    wr::<u32>(e, 8, 0);
    wr::<u32>(e, 4, rd::<u32>(nodes, 4 * index as isize));
    wr::<u32>(e, 0xC, rd::<u32>(edges, 0));
    wr::<u32>(edges, 0, addr(e));
    e
}

/// Marks a triangle (0x75A250, cdecl `(node, buckets, pass)`): a trial pass (< 4, signed) only sets
/// +0x1C. The final pass sets +0x18, takes the node out of its bucket, and for each edge moves the
/// unused triangle on the other side (the first triangle when it is set, not this node and unused,
/// else the second when set and unused) to the front of the next lower bucket. (The original keeps
/// its edge counter in the `pass` argument slot.)
#[hook(addr = 0x75A250, conv = cdecl, name = "FUN_0075a250", expect = "83 7C 24 0C 04 7D 0C",
       unverified = "std5-tristrip-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn mark_used(t: GPtr<u8>, buckets: GPtr<u8>, pass: u32) {
    if (pass as i32) < 4 {
        wr::<u32>(t, 0x1C, 1);
        return;
    }
    wr::<u32>(t, 0x18, 1);
    bucket_unlink(t, buckets);
    for k in 0..3isize {
        let e = ptr(t, 4 + 4 * k);
        let t1 = ptr(e, 4);
        let other = if !t1.is_null() && addr(t1) != addr(t) && rd::<u32>(t1, 0x18) == 0 {
            t1
        } else {
            let t2 = ptr(e, 8);
            if t2.is_null() || rd::<u32>(t2, 0x18) != 0 {
                continue;
            }
            t2
        };
        bucket_unlink(other, buckets);
        let c = rd::<u8>(other, 0x20).wrapping_sub(1);
        wr::<u8>(other, 0x20, c);
        let h = rd::<u32>(buckets, 4 * isize::from(c));
        wr::<u32>(other, 0x10, h);
        if h != 0 {
            wr::<u32>(p8(h), 0x14, addr(other));
        }
        wr::<u32>(buckets, 4 * isize::from(rd::<u8>(other, 0x20)), addr(other));
        wr::<u32>(other, 0x14, 0);
    }
}

/// Extends a strip (0x75A370, cdecl `(strip, edge, buckets, tris, pass)`): while the edge has an
/// unused triangle (the first triangle when unused, else the second; the pass's flag), that
/// triangle is marked (0x75A250) and counted; its vertex opposite the edge (from the edge after the
/// entry edge, `(i + 1) % 3`, i = -1 when the entry edge is not one of its own) is the new index.
/// The triangle's edge joining the strip's last index and the new one continues the strip when it
/// has an unused triangle (index appended). Else another of its edges (not the entry edge, not
/// that one; remembered across steps) can continue it after a swap (the index two back repeated)
/// when the strip has an even length and that edge has an unused triangle; otherwise the new index
/// ends the strip. Returns the number of triangles added. (The original keeps the new index in the
/// `edge` argument slot.)
#[hook(addr = 0x75A370, conv = cdecl, name = "FUN_0075a370", expect = "83 EC 14 53 55 56",
       unverified = "std5-tristrip-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn extend_strip(s: GPtr<u8>, edge: GPtr<u8>, buckets: GPtr<u8>, _tris: GPtr<u8>, pass: u32) -> u32 {
    let mut count = 0u32;
    let mut alt = 0u32;
    let mut edge = edge;
    if edge.is_null() {
        return 0;
    }
    let flag = flag_of(pass);
    let append = |v: u16| {
        let c = rd::<u32>(s, 4);
        wr::<u16>(ptr(s, 0), 2 * c as isize, v);
        wr::<u32>(s, 4, c + 1);
    };
    loop {
        let t1 = ptr(edge, 4);
        let t = if !t1.is_null() && rd::<u32>(t1, flag) == 0 {
            t1
        } else {
            let t2 = ptr(edge, 8);
            if t2.is_null() || rd::<u32>(t2, flag) != 0 {
                return count;
            }
            t2
        };
        count += 1;
        mark_used_call(t, buckets, pass);
        let e0 = rd::<u32>(t, 4);
        let ea = addr(edge);
        let i: i32 = if ea == e0 {
            0
        } else if ea == rd::<u32>(t, 8) {
            1
        } else if ea == rd::<u32>(t, 0xC) {
            2
        } else {
            -1
        };
        let opp = ptr(t, 4 + 4 * ((i + 1) % 3) as isize);
        let nv = if rd::<u32>(opp, 4) == addr(t) { rd::<u16>(opp, 2) } else { rd::<u16>(opp, 0) };
        let buf = ptr(s, 0);
        let c = rd::<u32>(s, 4);
        let last = rd::<u16>(buf, 2 * c as isize - 2);
        let joins = |e: GPtr<u8>| {
            let (a, b) = (rd::<u16>(e, 0), rd::<u16>(e, 2));
            (a == last && b == nv) || (a == nv && b == last)
        };
        let m = [e0, rd::<u32>(t, 8), rd::<u32>(t, 0xC)]
            .into_iter()
            .find(|&e| joins(p8(e)))
            .unwrap_or(0);
        if m != 0 && free_on(p8(m), flag) != 0 {
            wr::<u16>(buf, 2 * c as isize, nv);
            wr::<u32>(s, 4, rd::<u32>(s, 4) + 1);
            edge = p8(m);
            continue;
        }
        for e in [e0, rd::<u32>(t, 8), rd::<u32>(t, 0xC)] {
            if e != ea && e != m {
                alt = e;
                break;
            }
        }
        if alt == 0 || free_on(p8(alt), flag) == 0 || c & 1 != 0 {
            append(nv);
            return count;
        }
        wr::<u16>(buf, 2 * c as isize, rd::<u16>(buf, 2 * c as isize - 4));
        wr::<u32>(s, 4, c + 1);
        wr::<u16>(ptr(s, 0), 2 * (c + 1) as isize, nv);
        wr::<u32>(s, 4, rd::<u32>(s, 4) + 1);
        edge = p8(alt);
    }
}

/// Joins a strip list (0x75A660, cdecl `(&list, joinParity)`): NULL list → 0. A new strip of
/// capacity Σ(count + 6) (`* 4` bytes) takes the first strip's indices; then repeatedly the first
/// strip in the list starting with the joined strip's last index (appended as is, or after
/// repeating that index when the joined length is odd and `joinParity`), else the first one starting
/// with the index before the last (its first index once, or twice when the length is even and
/// `joinParity`), else the list's first strip after the degenerate pair (last index, its first
/// index; the first index twice when the length is odd and `joinParity`). Each consumed strip is
/// freed. `list` becomes the joined strip; 1.
#[hook(addr = 0x75A660, conv = cdecl, name = "FUN_0075a660", expect = "53 55 8B 6C 24 0C",
       unverified = "std5-tristrip-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn join_strips(list: GPtr<u8>, join_parity: u32) -> u32 {
    if rd::<u32>(list, 0) == 0 {
        return 0;
    }
    let j = rw_memory_alloc(strip_free_list(), HINT_STRIP);
    wr::<u32>(j, 4, 0);
    wr::<u32>(j, 8, 0);
    let mut s = rd::<u32>(list, 0);
    while s != 0 {
        wr::<u32>(j, 8, rd::<u32>(j, 8).wrapping_add(rd::<u32>(p8(s), 4) + 6));
        s = rd::<u32>(p8(s), 0xC);
    }
    wr::<u32>(j, 0, addr(rw_malloc(rd::<u32>(j, 8) << 2, HINT_STRIP)));
    let push = |v: u16| {
        let c = rd::<u32>(j, 4);
        wr::<u16>(ptr(j, 0), 2 * c as isize, v);
        wr::<u32>(j, 4, c + 1);
    };
    let first = ptr(list, 0);
    let mut i = 0u32;
    while i < rd::<u32>(first, 4) {
        push(rd::<u16>(ptr(first, 0), 2 * i as isize));
        i += 1;
    }
    rw_free(ptr(first, 0));
    let mut rest = rd::<u32>(first, 0xC);
    wr::<u32>(first, 0, 0);
    rw_memory_free(strip_free_list(), first);
    while rest != 0 {
        let c = rd::<u32>(j, 4);
        let jb = ptr(j, 0);
        let first_of = |s: u32| rd::<u16>(ptr(p8(s), 0), 0);
        let find = |v: u16| {
            let mut s = rest;
            while s != 0 {
                if first_of(s) == v {
                    return s;
                }
                s = rd::<u32>(p8(s), 0xC);
            }
            0
        };
        let last = rd::<u16>(jb, 2 * c as isize - 2);
        let mut chosen = find(last);
        if chosen != 0 {
            if c & 1 != 0 && join_parity != 0 {
                wr::<u16>(jb, 2 * c as isize, first_of(chosen));
                wr::<u32>(j, 4, rd::<u32>(j, 4) + 1);
            }
        } else {
            chosen = find(rd::<u16>(jb, 2 * c as isize - 4));
            if chosen != 0 {
                if c & 1 == 0 && join_parity != 0 {
                    wr::<u16>(jb, 2 * c as isize, first_of(chosen));
                    wr::<u16>(ptr(j, 0), 2 * rd::<u32>(j, 4) as isize + 2, first_of(chosen));
                    wr::<u32>(j, 4, rd::<u32>(j, 4) + 2);
                } else {
                    wr::<u16>(jb, 2 * c as isize, first_of(chosen));
                    wr::<u32>(j, 4, rd::<u32>(j, 4) + 1);
                }
            } else {
                chosen = rest;
                let jb = ptr(j, 0);
                wr::<u16>(jb, 2 * c as isize, rd::<u16>(jb, 2 * c as isize - 2));
                wr::<u16>(ptr(j, 0), 2 * rd::<u32>(j, 4) as isize + 2, first_of(rest));
                if c & 1 != 0 && join_parity != 0 {
                    wr::<u16>(ptr(j, 0), 2 * rd::<u32>(j, 4) as isize + 4, first_of(rest));
                    wr::<u32>(j, 4, rd::<u32>(j, 4) + 3);
                } else {
                    wr::<u32>(j, 4, rd::<u32>(j, 4) + 2);
                }
            }
        }
        let cs = p8(chosen);
        let mut i = 0u32;
        while i < rd::<u32>(cs, 4) {
            push(rd::<u16>(ptr(cs, 0), 2 * i as isize));
            i += 1;
        }
        rw_free(ptr(cs, 0));
        wr::<u32>(cs, 0, 0);
        if rest == chosen {
            rest = rd::<u32>(cs, 0xC);
        } else {
            let mut prev = rest;
            let mut cur = rd::<u32>(p8(rest), 0xC);
            while cur != chosen {
                prev = cur;
                cur = rd::<u32>(p8(cur), 0xC);
            }
            wr::<u32>(p8(prev), 0xC, rd::<u32>(cs, 0xC));
        }
        rw_memory_free(strip_free_list(), cs);
    }
    wr::<u32>(list, 0, addr(j));
    wr::<u32>(j, 0xC, 0);
    1
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_GENERATE_DEFAULT_TRISTRIP,
    HOOK_GENERATE_TRISTRIP,
    HOOK_MATERIAL_COMPARE,
    HOOK_STRIP_MATERIAL,
    HOOK_BUILD_ADJACENCY,
    HOOK_ADD_EDGE,
    HOOK_MARK_USED,
    HOOK_EXTEND_STRIP,
    HOOK_JOIN_STRIPS,
];
