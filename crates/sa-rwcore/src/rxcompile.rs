//! The Rx pipeline compiler (STD-4 helper, written for the native boot): `RxPipelineLock`,
//! `RxLockedPipeAddFragment`, `RxLockedPipeUnlock` and every helper they call, plus
//! `RxPipelineFindNodeByName` and the world-sector node-definition getter. Ported from the 1.0 US
//! bytes (RW has no gta-reversed source); `docs/re/specs/RW/RxPipeline.md` is the reading aid, the
//! bytes win where they differ (noted per function).
//!
//! Layouts (RW 3.6): `RxPipeline` (0x34) +0 locked, +4 numNodes, +8 nodes, +0xC
//! packetNumClusterSlots, +0x10 embeddedPacketState, +0x14 embeddedPacket, +0x18
//! numInputRequirements, +0x1C inputRequirements, +0x20 superBlock, +0x24 superBlockSize, +0x28
//! entryPoint. `RxPipelineNode` (0x28) +0 nodeDef, +4 numOutputs, +8 outputs, +0xC slotClusterRefs,
//! +0x10 slotsContinue, +0x14 privateData, +0x18 inputToClusterSlot, +0x1C topSortData, +0x20
//! initializationData, +0x24 initializationDataSize. `RxNodeDefinition` +0 name, +8 nodeInit, +0xC
//! nodeTerm, +0x10 pipelineNodeInit, +0x14 pipelineNodeTerm, +0x18 pipelineNodeConfig, +0x20
//! numClustersOfInterest, +0x24 clustersOfInterest (0xC each: clusterDef, forcePresent, reserved),
//! +0x28 inputRequirements, +0x2C numOutputs, +0x30 outputs (0xC each: name, outputClusters,
//! allOtherClusters), +0x34 privateDataSize, +0x38 editable, +0x3C InputPipesCnt.
//! A locked pipeline's super block is `nodes[max] (0x28) | outputs[max][32] (0x80) |
//! topSortData[max] (0xC)` with `max = P+0x38` (`P = G + [0xC9BC60]`).
//!
//! Unlock-time structures (in the scratch window [0xC9A6B8, 0xC9A6B4) of the super block):
//! requirement set (0x14) +0 count, +4 capacity, +8 slot mask, +0xC entries, +0x10 node; entry
//! (0x24) +0 clusterDef, +4 requirement, +8 reference count, +0xC group, +0x10 next in group,
//! +0x14 set, +0x18 output mask, +0x1C slot, +0x20 requiring node; group (0x10) +0 entry list,
//! +4 next merged group, +8 next group of the list, +0xC parent.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{error1, error2, g, p8, rw_error_set, rw_free, rw_globals, rw_malloc, rw_realloc, RwError};

sa_core::globals! {
    list RXC_GLOBALS;
    /// Scratch window top while a pipeline is unlocked (allocations grow down from here)
    pub static SCRATCH_TOP @ 0xC9A6B4: u32 = 0;
    /// Scratch window bottom while a pipeline is unlocked (allocations grow up from here)
    pub static SCRATCH_BOTTOM @ 0xC9A6B8: u32 = 0;
}

orig! {
    /// `_rwerror(code, a, b)` (0x8088D0, varargs; three arguments)
    fn rw_error3(code: u32, a: u32, b: u32) -> u32 = cdecl 0x8088D0 => unported;
    /// `_rwerror(code, a, b, c, d, e)` (0x8088D0, varargs; six arguments)
    fn rw_error6(code: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 = cdecl 0x8088D0 => unported;
    /// `G->stringFuncs.vecStrcmp` (`RwGlobals` +0x114)
    fn rw_strcmp(a: u32, b: u32) -> i32 = cdecl table 0xC97B24 0x114 => unported;
    /// A node-definition callback `(def)` (nodeInit / nodeTerm) through its stored pointer
    fn call_def_cb(f: u32, def: u32) -> i32 = cdecl dyn => unported;
    /// A pipeline-node callback `(node)` (pipelineNodeInit / pipelineNodeTerm) through its stored pointer
    fn call_node_cb(f: u32, node: u32) -> i32 = cdecl dyn => unported;
    /// `pipelineNodeConfig(node, pipeline)` through its stored pointer
    fn call_config_cb(f: u32, node: u32, pipe: u32) -> i32 = cdecl dyn => unported;

    /// `_rxPipelineNodeDestroy(node, pipeline)` (0x8050C0)
    fn node_destroy_call(node: u32, pipe: u32) -> u32 = cdecl 0x8050C0 => pipeline_node_destroy;
    /// Scratch allocation from the top (0x805890)
    fn scratch_top_call(size: u32) -> u32 = cdecl 0x805890 => scratch_alloc_top;
    /// Scratch allocation from the bottom (0x8058F0)
    fn scratch_bottom_call(size: u32) -> u32 = cdecl 0x8058F0 => scratch_alloc_bottom;
    /// Distinct cluster count (0x805950)
    fn count_clusters_call(pipe: u32) -> u32 = cdecl 0x805950 => pipeline_count_clusters;
    /// `_rxPipelineResize(pipe, size)` (0x8064D0)
    fn resize_call(pipe: u32, size: u32) -> i32 = cdecl 0x8064D0 => pipeline_resize;
    /// `_rxPipelineCopyInto(dst, src)` (0x8065E0)
    fn copy_into_call(dst: u32, src: u32) -> i32 = cdecl 0x8065E0 => pipeline_copy_into;
    /// Topological sort step (0x8067B0)
    fn top_sort_call(ctx: u32, idx: u32) = cdecl 0x8067B0 => pipeline_top_sort;
    /// Node set-up from its definition (0x806DE0)
    fn node_init_call(pipe: u32, node: u32, def: u32) -> i32 = cdecl 0x806DE0 => pipeline_node_init;
    /// Unlock: cluster resolution (0x80F0D0)
    fn resolve_call(pipe: u32) -> u32 = cdecl 0x80F0D0 => unlock_resolve;
    /// Requirement set creation (0x80F460)
    fn req_create_call(node: u32, capacity: u32) -> u32 = cdecl 0x80F460 => req_create;
    /// Requirement add (0x80F4B0)
    fn req_add_call(req: u32, cd: u32, requirement: u32, refs: u32, node: u32) -> u32 = cdecl 0x80F4B0 => req_add;
    /// Cluster index in an io spec (0x80F540)
    fn io_find_call(io: u32, cd: u32) -> i32 = cdecl 0x80F540 => io_find_cluster;
    /// Requirement release (0x80F570)
    fn req_release_call(pipe: u32, node: u32, cd: u32) = cdecl 0x80F570 => req_release;
    /// Unlock: forward pass (0x80F690)
    fn forward_call(pipe: u32) -> u32 = cdecl 0x80F690 => unlock_forward;
    /// Unlock: cluster groups (0x80F970)
    fn groups_call(pipe: u32, list: u32) -> u32 = cdecl 0x80F970 => unlock_groups;
    /// Unlock: slot assignment (0x80FBA0)
    fn slots_call(pipe: u32, list: u32) -> u32 = cdecl 0x80FBA0 => unlock_slots;
    /// Unlock: packet / cluster build (0x80FC30)
    fn build_call(pipe: u32, list: u32) -> u32 = cdecl 0x80FC30 => unlock_build;
    /// Radix exchange sort (0x8103D0)
    fn sort_call(base: u32, count: u32, stride: u32, key: u32, min: u32, max: u32) = cdecl 0x8103D0 => radix_sort;
}

#[inline]
fn r(a: u32) -> u32 {
    g::<u32>(a as usize).read()
}

#[inline]
fn w(a: u32, v: u32) {
    g::<u32>(a as usize).write(v)
}

fn a32(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// `P+0x38` (max nodes per pipeline), both globals reloaded as the original does.
fn max_nodes() -> u32 {
    r(a32(rw_globals()).wrapping_add(r(0xC9BC60)).wrapping_add(0x38))
}

/// `RwErrorSet(&{1, code})` with an already computed `_rwerror` result.
fn error_set_code(code: u32) {
    let mut e = RwError {
        plugin_id: 1,
        error_code: code as i32,
    };
    rw_error_set(GPtr::from_raw(&mut e));
}

/// Index of `node` in `nodes` as the original computes it (signed division by 0x28).
fn node_index(node: u32, nodes: u32) -> u32 {
    ((node.wrapping_sub(nodes) as i32) / 0x28) as u32
}

/// Copies `n` dwords from `src` to `dst`, forwards (`rep movsd`).
fn copy_dwords(dst: u32, src: u32, n: u32) {
    for k in 0..n {
        w(dst.wrapping_add(4 * k), r(src.wrapping_add(4 * k)));
    }
}

/// The validity of `cd` on an output: `ci = index in io` (`-1`: allOtherClusters, else
/// outputClusters[ci]).
fn output_validity(out_spec: u32, ci: i32) -> u32 {
    if ci == -1 {
        r(out_spec + 8)
    } else {
        r(r(out_spec + 4).wrapping_add(4 * ci as u32))
    }
}

/// Linear search of a requirement set for `cd` (the inlined loop): the entry, or 0.
fn req_find(req: u32, cd: u32) -> u32 {
    let n = r(req);
    if n == 0 {
        return 0;
    }
    let mut e = r(req + 0xC);
    for _ in 0..n {
        if r(e) == cd {
            return e;
        }
        e = e.wrapping_add(0x24);
    }
    0
}

/// The inlined append of a not-yet-present entry `{cd, 0, 1, 0, 0, req, 0, -1, node}` (0x80F6F4,
/// 0x80F830): stores in the original order.
fn req_append_default(req: u32, cd: u32, node: u32) -> u32 {
    let n = r(req);
    let e = r(req + 0xC).wrapping_add(n * 0x24);
    w(req, n.wrapping_add(1));
    w(e + 0x14, req);
    w(e, cd);
    w(e + 4, 0);
    w(e + 8, 1);
    w(e + 0x10, 0);
    w(e + 0xC, 0);
    w(e + 0x18, 0);
    w(e + 0x1C, 0xFFFF_FFFF);
    w(e + 0x20, node);
    e
}

// ------------------------------------------------------------------ small helpers ---------------

/// `RxNodeDefinitionGetD3D9WorldSectorAllInOne()` (0x75E9F0): the static node definition 0x8D63B8.
#[hook(addr = 0x75E9F0, conv = cdecl, name = "RxNodeDefinitionGetD3D9WorldSectorAllInOne", expect = "B8 B8 63 8D 00 C3",
       route = "L2", hazards = "none")]
pub fn node_def_world_sector_all_in_one() -> u32 {
    0x8D_63B8
}

/// Scratch allocation from the top (0x805890, catalogue `FUN_00805890`): `n = (size + 3) & ~3`,
/// `top -= n`; below the bottom (unsigned) the top is restored, `{1, E_RW_NOMEM(n)}` reported and
/// NULL returned; else the new top.
#[hook(addr = 0x805890, conv = cdecl, name = "FUN_00805890", expect = "8B 44 24 04 8B 15 B8 A6 C9 00",
       unverified = "std4-rxpipe-1", route = "L2", hazards = "none")]
pub fn scratch_alloc_top(size: u32) -> u32 {
    let bottom = SCRATCH_BOTTOM.get();
    let n = size.wrapping_add(3) & !3;
    let t = SCRATCH_TOP.get().wrapping_sub(n);
    SCRATCH_TOP.set(t);
    if t >= bottom {
        return t;
    }
    SCRATCH_TOP.set(t.wrapping_add(n));
    error2(0x8000_0013, n);
    0
}

/// Scratch allocation from the bottom (0x8058F0, catalogue `FUN_008058f0`): `bottom += n`; above the
/// top (unsigned) the bottom is restored, `{1, E_RW_NOMEM(n)}` reported and NULL returned; else the
/// old bottom.
#[hook(addr = 0x8058F0, conv = cdecl, name = "FUN_008058f0", expect = "8B 44 24 04 8B 15 B4 A6 C9 00",
       unverified = "std4-rxpipe-1", route = "L2", hazards = "none")]
pub fn scratch_alloc_bottom(size: u32) -> u32 {
    let top = SCRATCH_TOP.get();
    let n = size.wrapping_add(3) & !3;
    let b = SCRATCH_BOTTOM.get().wrapping_add(n);
    SCRATCH_BOTTOM.set(b);
    if b <= top {
        return b.wrapping_sub(n);
    }
    SCRATCH_BOTTOM.set(b.wrapping_sub(n));
    error2(0x8000_0013, n);
    0
}

/// The number of distinct non-NULL cluster definitions over every node's clusters of interest
/// (selection by increasing pointer value).
fn distinct_clusters(pipe: u32) -> u32 {
    let n = r(pipe + 4);
    let mut count = 0u32;
    let mut prev = 0u32;
    loop {
        let mut best = 0xFFFF_FFFFu32;
        if n == 0 {
            return count;
        }
        let mut node = r(pipe + 8);
        for _ in 0..n {
            let def = r(node);
            let c = r(def + 0x20);
            let mut cr = r(def + 0x24);
            for _ in 0..c {
                let v = r(cr);
                if v > prev && v < best {
                    best = v;
                }
                cr = cr.wrapping_add(0xC);
            }
            node = node.wrapping_add(0x28);
        }
        if best == 0xFFFF_FFFF {
            return count;
        }
        count += 1;
        prev = best;
    }
}

/// Distinct cluster count (0x805950, catalogue `FUN_00805950`); see [`distinct_clusters`].
#[hook(addr = 0x805950, conv = cdecl, name = "FUN_00805950", expect = "83 EC 08 8B 44 24 0C 53",
       route = "L2", hazards = "none")]
pub fn pipeline_count_clusters(pipe: u32) -> u32 {
    distinct_clusters(pipe)
}

/// `_rxPipelineResize(pipe, size)` (0x8064D0, catalogue `FUN_008064d0`): `rwrealloc(superBlock,
/// size, 0x1040409)`; NULL reports `{1, E_RW_NOMEM(size)}` and returns FALSE. Else superBlock and
/// nodes = the new block, superBlockSize = size, and every non-NULL pointer into the block (the
/// embedded packet, the input requirements, each node's +8 .. +0x1C) moves by the delta. TRUE.
#[hook(addr = 0x8064D0, conv = cdecl, name = "FUN_008064d0", expect = "83 EC 08 A1 24 7B C9 00 53",
       route = "L2", hazards = "heap-writes")]
pub fn pipeline_resize(pipe: u32, size: u32) -> i32 {
    let old = r(pipe + 0x20);
    let nb = a32(rw_realloc(p8(old), size, 0x0104_0409));
    if nb == 0 {
        error2(0x8000_0013, size);
        return 0;
    }
    let n = r(pipe + 4);
    let d = nb.wrapping_sub(old);
    w(pipe + 0x20, nb);
    w(pipe + 8, nb);
    w(pipe + 0x24, size);
    for off in [0x14u32, 0x1C] {
        let v = r(pipe + off);
        if v != 0 {
            w(pipe + off, v.wrapping_add(d));
        }
    }
    let mut o = 0u32;
    for _ in 0..n {
        for f in [8u32, 0xC, 0x10, 0x14, 0x18, 0x1C] {
            let a = r(pipe + 8).wrapping_add(o).wrapping_add(f);
            let v = r(a);
            if v != 0 {
                w(a, v.wrapping_add(d));
            }
        }
        o = o.wrapping_add(0x28);
    }
    1
}

/// `_rxPipelineCopyInto(dst, src)` (0x8065E0, catalogue `FUN_008065e0`). For `dst != src`: nodes
/// `n-1 .. 0` are copied, their +0xC .. +0x18 cleared and a non-empty initialization data block
/// duplicated (`rwmalloc(size, 0x30409)`; failure reports `{1, E_RW_NOMEM(size)}` and returns
/// FALSE), then `dst->numNodes = src->numNodes`. Layout (also for `dst == src`): node `i`'s outputs
/// = slot buffer `i` (`nodes + max*0x28 + i*0x80`), filled with 32 dwords from `src`'s outputs
/// (read after the store, so a self copy copies nothing), and topSortData = slot `i` of the
/// topSortData area, cleared. TRUE.
#[hook(addr = 0x8065E0, conv = cdecl, name = "FUN_008065e0", expect = "8B 54 24 08 83 EC 08 53",
       unverified = "std4-rxpipe-1", route = "L2", hazards = "heap-writes")]
pub fn pipeline_copy_into(dst: u32, src: u32) -> i32 {
    if dst != src {
        let mut i = r(src + 4).wrapping_sub(1) as i32;
        while i >= 0 {
            let o = i as u32 * 0x28;
            copy_dwords(r(dst + 8) + o, r(src + 8) + o, 10);
            w(r(dst + 8) + o + 0xC, 0);
            w(r(dst + 8) + o + 0x10, 0);
            w(r(dst + 8) + o + 0x14, 0);
            w(r(dst + 8) + o + 0x18, 0);
            let size = r(r(dst + 8) + o + 0x24);
            if size > 0 {
                let p = a32(rw_malloc(size, 0x30409));
                w(r(dst + 8) + o + 0x20, p);
                let nd = r(dst + 8) + o;
                let d = r(nd + 0x20);
                if d == 0 {
                    error2(0x8000_0013, r(r(dst + 8) + i as u32 * 0x28 + 0x24));
                    return 0;
                }
                let s = r(r(src + 8) + o + 0x20);
                let len = r(nd + 0x24);
                for k in 0..len {
                    g::<u8>((d + k) as usize).write(g::<u8>((s + k) as usize).read());
                }
            }
            i -= 1;
        }
        w(dst + 4, r(src + 4));
    }
    let out_base = r(dst + 8).wrapping_add(max_nodes().wrapping_mul(0x28));
    let mut i = r(src + 4).wrapping_sub(1) as i32;
    while i >= 0 {
        let o = i as u32 * 0x28;
        let buf = out_base.wrapping_add(i as u32 * 0x80);
        w(r(dst + 8) + o + 8, buf);
        let s = r(r(src + 8) + o + 8);
        if s != 0 {
            copy_dwords(r(r(dst + 8) + o + 8), s, 0x20);
        }
        i -= 1;
    }
    let mut ts = out_base.wrapping_add(max_nodes() << 7);
    let mut o = 0u32;
    let mut k = 0u32;
    while k < r(src + 4) {
        w(ts, 0);
        w(ts + 4, 0);
        w(ts + 8, 0);
        w(r(dst + 8) + o + 0x1C, ts);
        ts = ts.wrapping_add(0xC);
        o += 0x28;
        k += 1;
    }
    1
}

/// Topological sort step (0x8067B0, catalogue `FUN_008067b0`), `ctx = {pipe, next}`: moves node
/// `idx` to position `n = ctx->next` (swapping the 32 output dwords and the topSortData contents
/// so the buffers stay with their slot, then the node structs, then renumbering every output
/// `n <-> idx`), `ctx->next = n + 1`, and for each output `t != -1` of the placed node counts the
/// visit and recurses when every input of `t` has been visited.
#[hook(addr = 0x8067B0, conv = cdecl, name = "FUN_008067b0", expect = "83 EC 40 8B 44 24 48 53",
       unverified = "std4-rxpipe-1", route = "L2", hazards = "heap-writes")]
pub fn pipeline_top_sort(ctx: u32, idx: u32) {
    let pipe = r(ctx);
    let n = r(ctx + 4);
    if n != idx {
        let on = n.wrapping_mul(0x28);
        let oi = idx.wrapping_mul(0x28);
        let nodes = r(pipe + 8);
        let a = r(nodes + on + 8);
        let b = r(nodes + oi + 8);
        for k in 0..0x20u32 {
            let t = r(a + 4 * k);
            w(a + 4 * k, r(b + 4 * k));
            w(b + 4 * k, t);
        }
        w(r(r(ctx) + 8) + on + 8, b);
        w(r(r(ctx) + 8) + oi + 8, a);
        let ta = r(r(r(ctx) + 8) + on + 0x1C);
        let tb = r(r(r(ctx) + 8) + oi + 0x1C);
        let save = [r(ta), r(ta + 4), r(ta + 8)];
        w(ta, r(tb));
        w(ta + 4, r(tb + 4));
        w(ta + 8, r(tb + 8));
        w(tb, save[0]);
        w(tb + 4, save[1]);
        w(tb + 8, save[2]);
        w(r(r(ctx) + 8) + on + 0x1C, tb);
        w(r(r(ctx) + 8) + oi + 0x1C, ta);
        let nodes = r(r(ctx) + 8);
        let mut tmp = [0u32; 10];
        for (k, t) in tmp.iter_mut().enumerate() {
            *t = r(nodes + on + 4 * k as u32);
        }
        copy_dwords(nodes + on, nodes + oi, 10);
        let nodes = r(r(ctx) + 8);
        for (k, t) in tmp.iter().enumerate() {
            w(nodes + oi + 4 * k as u32, *t);
        }
        let mut j = 0u32;
        let mut o = 0u32;
        while j < r(r(ctx) + 4) {
            let node = r(r(ctx) + 8) + o;
            let mut k = 0u32;
            while k < r(node + 4) {
                let p = r(node + 8) + 4 * k;
                let v = r(p);
                if v == n {
                    w(p, idx);
                } else if v == idx {
                    w(p, n);
                }
                k += 1;
            }
            o += 0x28;
            j += 1;
        }
    }
    let n = r(ctx + 4);
    let node = r(r(ctx) + 8) + n * 0x28;
    w(ctx + 4, n + 1);
    if r(node + 4) == 0 {
        return;
    }
    let mut k = 0u32;
    while k < r(node + 4) {
        let t = r(r(node + 8) + 4 * k);
        if t != 0xFFFF_FFFF {
            let tn = r(r(ctx) + 8) + t.wrapping_mul(0x28);
            let ts = r(tn + 0x1C);
            w(ts + 4, r(ts + 4).wrapping_add(1));
            let ts = r(tn + 0x1C);
            if r(ts) == r(ts + 4) {
                top_sort_call(ctx, t);
            }
        }
        k += 1;
    }
}

/// `_rxPipelineNodeDestroy(node, pipe)` (0x8050C0, catalogue `FUN_008050c0`).
///
/// Unlocked pipeline: pipelineNodeTerm(node), `InputPipesCnt -= 1`; at 0 nodeTerm(def) and, for an
/// editable definition, `rwfree(def)` and nodeDef = NULL; the initialization data is freed; the
/// node is zeroed and `numNodes -= 1` (no compaction). Locked pipeline: the initialization data is
/// freed; an unused editable definition is freed; for a node below the last one the output slot
/// buffers, the topSortData slots (quirk: shifted from slot 0, not from the node's index), and the
/// nodes after it move down one place, and every output index is renumbered (`v < idx` kept, `v ==
/// idx` -> -1, else `v - 1`, so -1 becomes -2 as in the original). `numNodes -= 1`. Returns node.
#[hook(addr = 0x8050C0, conv = cdecl, name = "FUN_008050c0", expect = "53 8B 5C 24 0C 55 56",
       unverified = "std4-rxpipe-1", route = "L2", hazards = "heap-writes")]
pub fn pipeline_node_destroy(node: u32, pipe: u32) -> u32 {
    if r(pipe) == 0 {
        let f = r(r(node) + 0x14);
        if f != 0 {
            call_node_cb(f, node);
        }
        let def = r(node);
        w(def + 0x3C, r(def + 0x3C).wrapping_sub(1));
        let def = r(node);
        if r(def + 0x3C) == 0 {
            let f = r(def + 0xC);
            if f != 0 {
                call_def_cb(f, def);
            }
            let def = r(node);
            if r(def + 0x38) != 0 {
                rw_free(p8(def));
                w(node, 0);
            }
        }
        let d = r(node + 0x20);
        if d != 0 {
            rw_free(p8(d));
            w(node + 0x20, 0);
            w(node + 0x24, 0);
        }
        for k in 0..10 {
            w(node + 4 * k, 0);
        }
        w(pipe + 4, r(pipe + 4).wrapping_sub(1));
        return node;
    }
    let d = r(node + 0x20);
    if d != 0 {
        rw_free(p8(d));
        w(node + 0x20, 0);
        w(node + 0x24, 0);
    }
    let def = r(node);
    if r(def + 0x3C) == 0 && r(def + 0x38) != 0 {
        rw_free(p8(def));
        w(node, 0);
    }
    let nodes = r(pipe + 8);
    let idx = node_index(node, nodes);
    if idx < r(pipe + 4).wrapping_sub(1) {
        let m = max_nodes();
        let mut dst = nodes.wrapping_add(idx << 7).wrapping_add(m.wrapping_mul(0x28));
        let mut src = dst.wrapping_add(0x80);
        let mut j = idx;
        loop {
            copy_dwords(dst, src, 0x20);
            dst = src;
            src = src.wrapping_add(0x80);
            j += 1;
            if j >= r(pipe + 4).wrapping_sub(1) {
                break;
            }
        }
        let m = max_nodes();
        let mut tdst = r(pipe + 8).wrapping_add(m.wrapping_mul(0xA8));
        let mut tsrc = tdst.wrapping_add(0xC);
        let mut j = idx;
        if idx < r(pipe + 4).wrapping_sub(1) {
            loop {
                w(tdst, r(tsrc));
                w(tdst + 4, r(tsrc + 4));
                w(tdst + 8, r(tsrc + 8));
                tdst = tsrc;
                tsrc = tsrc.wrapping_add(0xC);
                j += 1;
                if j >= r(pipe + 4).wrapping_sub(1) {
                    break;
                }
            }
        }
        let mut j = idx;
        if idx < r(pipe + 4).wrapping_sub(1) {
            let mut o = idx.wrapping_mul(0x28);
            loop {
                let nodes = r(pipe + 8);
                copy_dwords(nodes + o, nodes + o + 0x28, 10);
                let a = r(pipe + 8) + o + 8;
                w(a, r(a).wrapping_sub(0x80));
                let a = r(pipe + 8) + o + 0x1C;
                w(a, r(a).wrapping_sub(0xC));
                j += 1;
                o += 0x28;
                if j >= r(pipe + 4).wrapping_sub(1) {
                    break;
                }
            }
        }
        if r(pipe + 4).wrapping_sub(1) != 0 {
            let mut j = 0u32;
            let mut o = 0u32;
            loop {
                let nd = r(pipe + 8) + o;
                if r(nd + 4) > 0 {
                    let mut k = 0u32;
                    loop {
                        let p = r(r(pipe + 8) + o + 8) + 4 * k;
                        let v = r(p);
                        if v >= idx {
                            if v == idx {
                                w(p, 0xFFFF_FFFF);
                            } else {
                                w(p, v.wrapping_sub(1));
                            }
                        }
                        k += 1;
                        if k >= r(r(pipe + 8) + o + 4) {
                            break;
                        }
                    }
                }
                j += 1;
                o += 0x28;
                if j >= r(pipe + 4).wrapping_sub(1) {
                    break;
                }
            }
        }
    }
    w(pipe + 4, r(pipe + 4).wrapping_sub(1));
    node
}

/// Node set-up from its definition (0x806DE0, catalogue `FUN_00806de0`), `node` being slot
/// `numNodes`: the node is zeroed; more than 32 outputs report `0x29`, more than 32 clusters of
/// interest `0x28` (both continue); `numOutputs >= max` reports `0x2A` and returns FALSE; after an
/// earlier error FALSE. Else outputs = slot buffer `numNodes` (all -1), topSortData = slot
/// `numNodes` (cleared), init data cleared, nodeDef = def, `numNodes += 1`. TRUE.
#[hook(addr = 0x806DE0, conv = cdecl, name = "FUN_00806de0", expect = "8B 44 24 0C 83 EC 08 B9 0A 00 00 00",
       route = "L2", hazards = "heap-writes")]
pub fn pipeline_node_init(pipe: u32, node: u32, def: u32) -> i32 {
    let num_out = r(def + 0x2C);
    for k in 0..10 {
        w(node + 4 * k, 0);
    }
    let mut ok = 1;
    if num_out > 0x20 {
        error1(0x29);
        ok = 0;
    }
    if r(def + 0x20) > 0x20 {
        error1(0x28);
        ok = 0;
    }
    let m = max_nodes();
    if num_out >= m {
        error1(0x2A);
        return 0;
    }
    if ok == 0 {
        return 0;
    }
    let mut out = (r(pipe + 4) << 7).wrapping_add(m.wrapping_mul(0x28)).wrapping_add(r(pipe + 8));
    w(node + 8, out);
    w(node + 4, num_out);
    let mut k = 0u32;
    while k < r(node + 4) {
        w(out, 0xFFFF_FFFF);
        out = out.wrapping_add(4);
        k += 1;
    }
    let m = max_nodes();
    let ts = r(pipe + 8).wrapping_add(r(pipe + 4).wrapping_add(m.wrapping_mul(14)).wrapping_mul(12));
    w(ts, 0);
    w(ts + 4, 0);
    w(ts + 8, 0);
    w(node + 0x1C, ts);
    w(node + 0x20, 0);
    w(node + 0x24, 0);
    w(node, def);
    w(pipe + 4, r(pipe + 4).wrapping_add(1));
    1
}

// ------------------------------------------------------------------ public API -------------------

/// `RxPipelineLock(pipe)` (0x806990). An already locked pipeline is returned as it is. A new one
/// gets its super block (`rwmalloc(max * 0xB4, 0x1030409)`; failure: `{1, E_RW_NOMEM(size)}`, NULL);
/// an unlocked compiled one is grown when smaller than `max * 0xB4` (0x8064D0) and re-laid out
/// (0x8065E0). Then locked = 1 and each node is torn down: pipelineNodeTerm(node), `InputPipesCnt
/// -= 1` (nodeTerm(def) at 0), slotClusterRefs = NULL. Returns the pipeline.
#[hook(addr = 0x806990, conv = cdecl, name = "RxPipelineLock", expect = "83 EC 08 55 56 8B 74 24 14",
       route = "L2", hazards = "heap-writes")]
pub fn rx_pipeline_lock(pipe: u32) -> u32 {
    if r(pipe) != 0 {
        return pipe;
    }
    let size = max_nodes().wrapping_mul(0xB4);
    if r(pipe + 8) != 0 {
        if size > r(pipe + 0x24) && resize_call(pipe, size) == 0 {
            return 0;
        }
        if copy_into_call(pipe, pipe) == 0 {
            return 0;
        }
    } else {
        let sb = a32(rw_malloc(size, 0x0103_0409));
        w(pipe + 0x20, sb);
        if sb == 0 {
            error2(0x8000_0013, size);
            return 0;
        }
        w(pipe + 0x24, size);
        w(pipe + 8, sb);
    }
    w(pipe, 1);
    if r(pipe + 8) == 0 || r(pipe + 4) == 0 {
        return pipe;
    }
    let mut i = 0u32;
    let mut o = 0u32;
    loop {
        let node = r(pipe + 8) + o;
        let def = r(node);
        let f = r(def + 0x14);
        if f != 0 {
            call_node_cb(f, node);
        }
        let def = r(r(pipe + 8) + o);
        w(def + 0x3C, r(def + 0x3C).wrapping_sub(1));
        let def = r(r(pipe + 8) + o);
        if r(def + 0x3C) == 0 {
            let f = r(r(node) + 0xC);
            if f != 0 {
                call_def_cb(f, def);
            }
        }
        w(r(pipe + 8) + o + 0xC, 0);
        i += 1;
        o += 0x28;
        if i >= r(pipe + 4) {
            break;
        }
    }
    pipe
}

/// `RxLockedPipeAddFragment(pipe, firstIndex, def0, ..., NULL)` (0x806BE0, varargs cdecl).
///
/// Every call in 1.0 US passes exactly one definition and the terminating NULL (0x5D6782,
/// 0x5D7ACA, 0x5D9FAA, 0x74EAFE, 0x74EBAE, 0x7CB363, 0x807399, 0x80A0CE, 0x80A1D2, 0x815E8F,
/// 0x815F16), so the port takes the list as `def0, def1`; a non-NULL `def1` would need the stack
/// words after it, which this signature cannot reach (panics instead of guessing).
///
/// NULL pipeline: `{1, E_RW_NULLP}`, NULL. Not locked: `{1, _rwerror(0x34)}`, NULL. No definition:
/// NULL. `numNodes + n > max`: `0x2A`, NULL. Each definition is set up in the next slot (0x806DE0;
/// failure stops); from the second on, the previous node's output 0 is linked to it (the inlined
/// `RxLockedPipeAddPath` checks; a failed check destroys the new node and stops). All added:
/// `*firstIndex = start` (when given), pipe. Else the added nodes are destroyed in reverse, NULL.
#[hook(addr = 0x806BE0, conv = cdecl, name = "RxLockedPipeAddFragment", expect = "83 EC 18 53 55 8B 6C 24 24",
       route = "L2", hazards = "heap-writes",
       note = "varargs: the port reads def0 and def1 only (every 1.0 US call passes one definition)")]
pub fn rx_locked_pipe_add_fragment(pipe: u32, first_index: u32, def0: u32, def1: u32) -> u32 {
    if pipe == 0 {
        error1(0x8000_0016);
        return 0;
    }
    if r(pipe) == 0 {
        error1(0x34);
        return 0;
    }
    if def0 == 0 {
        return 0;
    }
    assert!(
        def1 == 0,
        "RxLockedPipeAddFragment: more than one node definition (the varargs beyond def1 are not reachable)"
    );
    let defs = [def0];
    let n = defs.len() as u32;
    let start = r(pipe + 4);
    if start.wrapping_add(n) > max_nodes() {
        error1(0x2A);
        return 0;
    }
    let mut added = 0u32;
    let mut off = start.wrapping_mul(0x28);
    let mut prev = 0u32;
    for &def in defs.iter() {
        let node = r(pipe + 8).wrapping_add(off);
        if node_init_call(pipe, node, def) == 0 {
            break;
        }
        if prev != 0 {
            let cur = if node != 0 && r(node) != 0 { node } else { 0 };
            let out = if r(prev) != 0 && r(prev + 4) > 0 { r(prev + 8) } else { 0 };
            let mut linked = false;
            if r(pipe) != 0 && out != 0 && r(out) == 0xFFFF_FFFF && cur != 0 && r(cur) != 0 {
                let nodes = r(pipe + 8);
                let k = node_index(cur, nodes);
                if nodes.wrapping_add(k.wrapping_mul(0x28)) == cur && k < r(pipe + 4) && k != 0xFFFF_FFFF {
                    w(out, k);
                    linked = true;
                }
            }
            if !linked {
                node_destroy_call(node, pipe);
                break;
            }
        }
        added += 1;
        off = off.wrapping_add(0x28);
        prev = node;
    }
    if added == n {
        if first_index != 0 {
            w(first_index, start);
        }
        return pipe;
    }
    let mut k = added;
    while k != 0 {
        let node = r(pipe + 8).wrapping_add(start.wrapping_add(k).wrapping_mul(0x28)).wrapping_sub(0x28);
        node_destroy_call(node, pipe);
        k -= 1;
    }
    0
}

/// `RxPipelineFindNodeByName(pipe, name, start, &index)` (0x806B30): the first node after `start`
/// (from the first when NULL) whose definition's name compares equal (`G->vecStrcmp`); `*index =
/// numNodes - left` when found. Not found / NULL pipe / NULL name / no nodes: `*index = -1`, NULL.
#[hook(addr = 0x806B30, conv = cdecl, name = "RxPipelineFindNodeByName", expect = "53 8B 5C 24 08 55 56",
       route = "L2", hazards = "none")]
pub fn rx_pipeline_find_node_by_name(pipe: u32, name: u32, start: u32, index: u32) -> u32 {
    let not_found = || {
        if index != 0 {
            w(index, 0xFFFF_FFFF);
        }
        0
    };
    if pipe == 0 || name == 0 {
        return not_found();
    }
    let mut left = r(pipe + 4) as i32;
    if left == 0 {
        return not_found();
    }
    let mut cur = r(pipe + 8);
    if start != 0 {
        while cur != start && left > 0 {
            cur = cur.wrapping_add(0x28);
            left -= 1;
        }
        cur = cur.wrapping_add(0x28);
        left -= 1;
    }
    while left > 0 {
        let def = r(cur);
        if def != 0 && rw_strcmp(r(def), name) == 0 {
            if index != 0 {
                w(index, r(pipe + 4).wrapping_sub(left as u32));
            }
            return cur;
        }
        cur = cur.wrapping_add(0x28);
        left -= 1;
    }
    not_found()
}

/// Undo of the init loop (0x8063D8 / 0x8063E6): nodes `from .. n-1` get pipelineNodeTerm and their
/// `InputPipesCnt -= 1` (nodeTerm at 0), then the pipeline is re-laid out and NULL returned.
fn unlock_undo(pipe: u32, from: u32) -> u32 {
    let mut k = from;
    while k < r(pipe + 4) {
        let node = r(pipe + 8).wrapping_add(k.wrapping_mul(0x28));
        let def = r(node);
        let f = r(def + 0x14);
        if f != 0 {
            call_node_cb(f, node);
        }
        w(def + 0x3C, r(def + 0x3C).wrapping_sub(1));
        if r(def + 0x3C) == 0 {
            let f = r(def + 0xC);
            if f != 0 {
                call_def_cb(f, def);
            }
        }
        k += 1;
    }
    copy_into_call(pipe, pipe);
    0
}

/// `RxLockedPipeUnlock(pipe)` (0x805F40): compiles a locked pipeline.
///
/// Checks: NULL `{1, E_RW_NULLP}`; not locked `0x34`; no nodes: unlocked as it is; a bad entry
/// point `0x24`. The super block is grown to the size the compiler needs (formula of the bytes,
/// at least `max * 0xB4`), the scratch window set, the nodes topologically sorted from the entry
/// point (`0x24` entry has inputs, `0x22` another node has none, `0x1C` a node not reached), the
/// entry point reset to 0, the topSortData moved to the top of the needed area and the outputs
/// packed after the node array. 0x80F0D0 resolves the clusters (failure: NULL), the block shrinks
/// to the scratch bottom, topSortData is cleared, then nodes `n-1 .. 0` are initialised (nodeInit
/// on the first user, pipelineNodeInit) and configured (pipelineNodeConfig); a failure undoes the
/// initialised nodes (quirk kept: a failed nodeInit leaves its `InputPipesCnt` incremented),
/// re-lays the pipeline out and returns NULL. Success: unlocked, pipe.
#[hook(addr = 0x805F40, conv = cdecl, name = "RxLockedPipeUnlock", expect = "83 EC 14 53 8B 5C 24 1C",
       route = "L2", hazards = "heap-writes")]
pub fn rx_locked_pipe_unlock(pipe: u32) -> u32 {
    if pipe == 0 {
        error1(0x8000_0016);
        return 0;
    }
    if r(pipe) == 0 {
        error1(0x34);
        return 0;
    }
    let n = r(pipe + 4);
    if n == 0 {
        w(pipe, 0);
        return pipe;
    }
    let ep = r(pipe + 0x28);
    if ep >= n || r(r(pipe + 8).wrapping_add(ep.wrapping_mul(0x28))) == 0 {
        error1(0x24);
        return 0;
    }
    let k = distinct_clusters(pipe);
    let m = max_nodes();
    let mut a = n.wrapping_mul(40);
    if n > 0 {
        a = a.wrapping_add(r(r(pipe + 8) + 4).wrapping_mul(n).wrapping_mul(4));
    }
    let b = m
        .wrapping_mul(12)
        .wrapping_add(k.wrapping_mul(52).wrapping_add(0x14).wrapping_mul(n))
        .wrapping_add(a);
    let mut c = n.wrapping_add(5).wrapping_mul(k).wrapping_add(k.wrapping_add(1).wrapping_mul(n)) << 2;
    let mut o = 0u32;
    for _ in 0..n {
        let def = r(r(pipe + 8) + o);
        c = c.wrapping_add(r(def + 0x34));
        c = c.wrapping_add(r(def + 0x20).wrapping_mul(4));
        o += 0x28;
    }
    let mut need = m.wrapping_mul(180);
    let total = c.wrapping_add(k.wrapping_mul(28)).wrapping_add(b).wrapping_add(0x14);
    if total > need {
        need = total;
    }
    if need > r(pipe + 0x24) && resize_call(pipe, need) == 0 {
        return 0;
    }
    SCRATCH_BOTTOM.set(0);
    SCRATCH_TOP.set(r(pipe + 0x20).wrapping_add(need));
    // in-degree count
    let mut i = 0u32;
    while i < r(pipe + 4) {
        let node = r(pipe + 8) + i * 0x28;
        if r(node) != 0 {
            w(r(node + 0x1C) + 4, 0);
            w(r(node + 0x1C), 0);
        }
        i += 1;
    }
    let mut i = 0u32;
    while i < r(pipe + 4) {
        let node = r(pipe + 8) + i * 0x28;
        if r(node) != 0 && r(node + 4) != 0 {
            let cnt = r(node + 4);
            let outs = r(node + 8);
            for q in 0..cnt {
                let t = r(outs + 4 * q);
                if t != 0xFFFF_FFFF {
                    let ts = r(r(pipe + 8) + t.wrapping_mul(0x28) + 0x1C);
                    w(ts, r(ts).wrapping_add(1));
                }
            }
        }
        i += 1;
    }
    let ep = r(pipe + 0x28);
    if r(r(r(pipe + 8) + ep * 0x28 + 0x1C)) != 0 {
        error1(0x24);
        return 0;
    }
    let n = r(pipe + 4);
    for i in 0..n {
        if i != ep && r(r(r(pipe + 8) + i * 0x28 + 0x1C)) == 0 {
            error1(0x22);
            return 0;
        }
    }
    let mut ctx = [pipe, 0u32];
    top_sort_call(a32(GPtr::from_raw(ctx.as_mut_ptr()).cast()), ep);
    let n = r(pipe + 4);
    for i in 0..n {
        let ts = r(r(pipe + 8) + i * 0x28 + 0x1C);
        if r(ts) != r(ts + 4) {
            error1(0x1C);
            return 0;
        }
    }
    w(pipe + 0x28, 0);
    // topSortData to the top of the needed area
    let m = max_nodes();
    let out_base = r(pipe + 8).wrapping_add(m.wrapping_mul(0x28));
    let n = r(pipe + 4);
    let mut src = out_base.wrapping_add(m << 7).wrapping_add(n.wrapping_mul(12)).wrapping_sub(12);
    let mut dst = r(pipe + 0x20).wrapping_add(need).wrapping_sub(12);
    let mut i = n as i32 - 1;
    while i >= 0 {
        w(dst, r(src));
        w(dst + 4, r(src + 4));
        w(dst + 8, r(src + 8));
        w(r(pipe + 8) + i as u32 * 0x28 + 0x1C, dst);
        dst = dst.wrapping_sub(12);
        src = src.wrapping_sub(12);
        i -= 1;
    }
    let top = src;
    // outputs packed after the node array
    let mut slot = out_base;
    let mut sum = 0u32;
    let mut pack = r(pipe + 8).wrapping_add(r(pipe + 4).wrapping_mul(0x28));
    let mut i = 0u32;
    while i < r(pipe + 4) {
        let node = r(pipe + 8) + i * 0x28;
        let c = r(node + 4);
        if c == 0 {
            w(node + 8, 0);
        } else {
            copy_dwords(pack, slot, c);
            w(r(pipe + 8) + i * 0x28 + 8, pack);
        }
        slot = slot.wrapping_add(0x80);
        let c = r(r(pipe + 8) + i * 0x28 + 4);
        sum = sum.wrapping_add(c);
        pack = pack.wrapping_add(c * 4);
        i += 1;
    }
    SCRATCH_TOP.set(top);
    SCRATCH_BOTTOM.set(slot.wrapping_add(sum.wrapping_mul(4)));
    if resolve_call(pipe) != 0 {
        return 0;
    }
    if resize_call(pipe, SCRATCH_BOTTOM.get().wrapping_sub(r(pipe + 0x20))) == 0 {
        copy_into_call(pipe, pipe);
        return 0;
    }
    let mut i = 0u32;
    while i < r(pipe + 4) {
        w(r(pipe + 8) + i * 0x28 + 0x1C, 0);
        i += 1;
    }
    let mut i = r(pipe + 4) as i32 - 1;
    while i >= 0 {
        let node = r(pipe + 8) + i as u32 * 0x28;
        let def = r(node);
        let cnt = r(def + 0x3C);
        w(def + 0x3C, cnt.wrapping_add(1));
        if cnt == 0 {
            let f = r(def + 8);
            if f != 0 && call_def_cb(f, def) == 0 {
                return unlock_undo(pipe, i as u32 + 1);
            }
        }
        let f = r(def + 0x10);
        if f != 0 && call_node_cb(f, node) == 0 {
            w(def + 0x3C, r(def + 0x3C).wrapping_sub(1));
            if r(def + 0x3C) == 0 {
                let t = r(def + 0xC);
                if t != 0 {
                    call_def_cb(t, def);
                }
            }
            return unlock_undo(pipe, i as u32 + 1);
        }
        i -= 1;
    }
    let mut i = r(pipe + 4) as i32 - 1;
    while i >= 0 {
        let node = r(pipe + 8) + i as u32 * 0x28;
        let f = r(r(node) + 0x18);
        if f != 0 && call_config_cb(f, node, pipe) == 0 {
            return unlock_undo(pipe, 0);
        }
        i -= 1;
    }
    w(pipe, 0);
    pipe
}

// ------------------------------------------------------------------ unlock: cluster resolution ---

/// Cluster resolution (0x80F0D0, catalogue `FUN_0080f0d0`), returns 0 or the reported error.
///
/// Backward over the sorted nodes: the clusters of interest are checked (NULL: `_rwerror(0x1F,
/// name, i)`; a duplicate: `_rwerror(0x1E, name, cluster name)`); the node's requirement set is
/// created (capacity = the distinct cluster count; NULL: `0x20`); for every output `k` to `t` and
/// every requirement `e` of `t`: validity 0 inherits it (0x80F4B0 with the node's input count);
/// a REQUIRED cluster must be VALID on the output (else `_rwerror(0x1D, cluster, requiring node,
/// node, k, output name)`); an INVALID optional one is released downstream (0x80F570); then the
/// node's own input requirements are added. Then the forward pass, the groups, the slots and the
/// build (first non-zero result returned).
#[hook(addr = 0x80F0D0, conv = cdecl, name = "FUN_0080f0d0", expect = "83 EC 24 8B 4C 24 28 53",
       route = "L2", hazards = "heap-writes")]
pub fn unlock_resolve(pipe: u32) -> u32 {
    let mut count = r(pipe + 4);
    let mut groups: u32 = 0;
    let mut node = r(pipe + 8).wrapping_add(count.wrapping_mul(0x28)).wrapping_sub(0x28);
    loop {
        let def = r(node);
        let io = def + 0x20;
        let ncoi = r(io);
        if ncoi > 0 {
            let mut e = r(def + 0x24);
            let mut i = 0u32;
            while i < ncoi {
                let cd = r(e);
                if cd == 0 {
                    error_set_code(rw_error3(0x1F, r(r(node)), i));
                    return 0x1F;
                }
                let mut j = i + 1;
                let mut f = e.wrapping_add(0xC);
                while j < ncoi {
                    let cj = r(f);
                    if cd == cj {
                        error_set_code(rw_error3(0x1E, r(r(node)), r(cj)));
                        return 0x1E;
                    }
                    j += 1;
                    f = f.wrapping_add(0xC);
                }
                i += 1;
                e = e.wrapping_add(0xC);
            }
        }
        let cap = count_clusters_call(pipe);
        let req = req_create_call(node, cap);
        let ts = node + 0x1C;
        w(r(ts) + 8, req);
        if r(r(ts) + 8) == 0 {
            error1(0x20);
            return 0x20;
        }
        let mut k = 0u32;
        while k < r(node + 4) {
            let t = r(r(node + 8) + 4 * k);
            if t != 0xFFFF_FFFF {
                let tn = r(pipe + 8).wrapping_add(t.wrapping_mul(0x28));
                let out_spec = r(r(node) + 0x30).wrapping_add(k * 0xC);
                let treq = r(r(tn + 0x1C) + 8);
                if r(treq) > 0 {
                    let mut ei = 0u32;
                    let mut eo = 0u32;
                    loop {
                        let treq = r(r(tn + 0x1C) + 8);
                        let e = r(treq + 0xC).wrapping_add(eo);
                        let ci = io_find_call(io, r(e));
                        let v = output_validity(out_spec, ci);
                        if v == 0 {
                            let own = r(ts);
                            if req_add_call(r(own + 8), r(e), r(e + 4), r(own), r(e + 0x20)) == 0 {
                                error1(0x20);
                                return 0x20;
                            }
                        } else if r(e + 4) == 1 {
                            if v != 1 {
                                let c = rw_error6(
                                    0x1D,
                                    r(r(e)),
                                    r(r(r(e + 0x20))),
                                    r(r(node)),
                                    k,
                                    r(out_spec),
                                );
                                error_set_code(c);
                                return 0x1D;
                            }
                        } else if v == 2 {
                            req_release_call(pipe, tn, r(e));
                        }
                        ei += 1;
                        eo += 0x24;
                        if ei >= r(r(r(tn + 0x1C) + 8)) {
                            break;
                        }
                    }
                }
            }
            k += 1;
        }
        let mut i = 0u32;
        while i < r(io) {
            let req_i = r(r(io + 8) + 4 * i);
            if req_i != 0 {
                let own = r(ts);
                let cd = r(r(io + 4) + i * 0xC);
                if req_add_call(r(own + 8), cd, req_i, r(own), node) == 0 {
                    error1(0x20);
                    return 0x20;
                }
            }
            i += 1;
        }
        node = node.wrapping_sub(0x28);
        count = count.wrapping_sub(1);
        if count == 0 {
            break;
        }
    }
    let rr = forward_call(pipe);
    if rr != 0 {
        return rr;
    }
    let gl = a32(GPtr::from_raw(&mut groups).cast());
    let rr = groups_call(pipe, gl);
    if rr != 0 {
        return rr;
    }
    let rr = slots_call(pipe, gl);
    if rr != 0 {
        return rr;
    }
    build_call(pipe, g::<u32>(gl as usize).read())
}

/// Requirement set creation (0x80F460, catalogue `FUN_0080f460`): the set (0x14) and its entries
/// (`capacity * 0x24`) from the scratch top; NULL when either fails. count 0, mask 0, node.
#[hook(addr = 0x80F460, conv = cdecl, name = "FUN_0080f460", expect = "56 57 6A 14 E8 27 64 FF FF",
       route = "L2", hazards = "heap-writes")]
pub fn req_create(node: u32, capacity: u32) -> u32 {
    let s = scratch_top_call(0x14);
    if s == 0 {
        return 0;
    }
    let e = scratch_top_call(capacity.wrapping_mul(0x24));
    w(s + 0xC, e);
    if e == 0 {
        return 0;
    }
    w(s + 4, capacity);
    w(s, 0);
    w(s + 0x10, node);
    w(s + 8, 0);
    s
}

/// Requirement add (0x80F4B0, catalogue `FUN_0080f4b0`): an existing entry for `cd` becomes
/// REQUIRED with `node` when `requirement == 1` (else unchanged); a new entry is appended
/// `{cd, requirement, refs, 0, 0, set, 0, -1, node}` (no capacity check). Returns the entry.
#[hook(addr = 0x80F4B0, conv = cdecl, name = "FUN_0080f4b0", expect = "56 8B 74 24 08 57 8B 7C 24 10",
       unverified = "std4-rxpipe-1", route = "L2", hazards = "heap-writes")]
pub fn req_add(req: u32, cd: u32, requirement: u32, refs: u32, node: u32) -> u32 {
    let e = req_find(req, cd);
    if e != 0 {
        if requirement == 1 {
            w(e + 4, 1);
            w(e + 0x20, node);
        }
        return e;
    }
    let n = r(req);
    let e = r(req + 0xC).wrapping_add(n.wrapping_mul(0x24));
    w(req, n.wrapping_add(1));
    w(e, cd);
    w(e + 8, refs);
    w(e + 0x10, 0);
    w(e + 0x14, req);
    w(e + 0xC, 0);
    w(e + 0x18, 0);
    w(e + 0x1C, 0xFFFF_FFFF);
    w(e + 4, requirement);
    w(e + 0x20, node);
    e
}

/// Cluster index in an io spec (0x80F540, catalogue `FUN_0080f540`): `i` with
/// `clustersOfInterest[i].clusterDef == cd`, else -1.
#[hook(addr = 0x80F540, conv = cdecl, name = "FUN_0080f540", expect = "8B 4C 24 04 33 C0 56",
       unverified = "std4-rxpipe-1", route = "L2", hazards = "none")]
pub fn io_find_cluster(io: u32, cd: u32) -> i32 {
    let n = r(io);
    let mut e = r(io + 4);
    for i in 0..n {
        if r(e) == cd {
            return i as i32;
        }
        e = e.wrapping_add(0xC);
    }
    -1
}

/// Requirement release (0x80F570, catalogue `FUN_0080f570`, recursive): the entry for `cd` in the
/// node's set loses a reference; at 0 it is replaced by the last entry and the count drops, then
/// every output on which `cd` is unchanged (validity 0) releases it downstream too.
#[hook(addr = 0x80F570, conv = cdecl, name = "FUN_0080f570", expect = "51 53 8B 5C 24 10 55",
       unverified = "std4-rxpipe-1", route = "L2", hazards = "heap-writes")]
pub fn req_release(pipe: u32, node: u32, cd: u32) {
    let io = r(node) + 0x20;
    let req = r(r(node + 0x1C) + 8);
    let e = req_find(req, cd);
    if e == 0 {
        return;
    }
    w(e + 8, r(e + 8).wrapping_sub(1));
    if r(e + 8) != 0 {
        return;
    }
    let req = r(r(node + 0x1C) + 8);
    let last = r(req + 0xC).wrapping_add(r(req).wrapping_mul(0x24)).wrapping_sub(0x24);
    if e != last {
        copy_dwords(e, last, 9);
    }
    w(req, r(req).wrapping_sub(1));
    let mut k = 0u32;
    while k < r(node + 4) {
        let t = r(r(node + 8) + 4 * k);
        if t != 0xFFFF_FFFF {
            let out_spec = r(r(node) + 0x30).wrapping_add(k * 0xC);
            let v = output_validity(out_spec, io_find_inline(io, cd));
            if v == 0 {
                req_release_call(pipe, r(pipe + 8).wrapping_add(t.wrapping_mul(0x28)), cd);
            }
        }
        k += 1;
    }
}

/// The inlined cluster lookup of 0x80F61A / 0x80F7C6 (same result as 0x80F540).
fn io_find_inline(io: u32, cd: u32) -> i32 {
    let n = r(io);
    let mut e = r(io + 4);
    for i in 0..n {
        if r(e) == cd {
            return i as i32;
        }
        e = e.wrapping_add(0xC);
    }
    -1
}

/// Forward pass (0x80F690, catalogue `FUN_0080f690`), returns 0: for each node, every forced
/// cluster of interest gets an entry when missing (`{cd, 0, 1, ..., node}`); for every output `k`
/// to `t` and every entry `e` of `t`'s set: INVALID nothing; unchanged sets bit `k` of the node's
/// own entry for `e.cd` when there is one; otherwise the own entry is found or appended and bit
/// `k` set.
#[hook(addr = 0x80F690, conv = cdecl, name = "FUN_0080f690", expect = "83 EC 28 8B 44 24 2C 53",
       route = "L2", hazards = "heap-writes")]
pub fn unlock_forward(pipe: u32) -> u32 {
    let mut count = r(pipe + 4);
    let mut node = r(pipe + 8);
    loop {
        let io = r(node) + 0x20;
        let mut i = 0u32;
        while i < r(io) {
            let cr = r(io + 4).wrapping_add(i * 0xC);
            if r(cr + 4) != 0 {
                let cd = r(cr);
                let req = r(r(node + 0x1C) + 8);
                if req_find(req, cd) == 0 {
                    req_append_default(req, cd, node);
                }
            }
            i += 1;
        }
        let mut k = 0u32;
        while k < r(node + 4) {
            let t = r(r(node + 8) + 4 * k);
            if t != 0xFFFF_FFFF {
                let tn = r(pipe + 8).wrapping_add(t.wrapping_mul(0x28));
                let out_spec = r(r(node) + 0x30).wrapping_add(k * 0xC);
                if r(r(r(tn + 0x1C) + 8)) > 0 {
                    let mut ei = 0u32;
                    let mut eo = 0u32;
                    loop {
                        let treq = r(r(tn + 0x1C) + 8);
                        let e = r(treq + 0xC).wrapping_add(eo);
                        let v = output_validity(out_spec, io_find_inline(io, r(e)));
                        if v != 2 {
                            let own = r(r(node + 0x1C) + 8);
                            let oe = if v == 0 {
                                req_find(own, r(e))
                            } else {
                                let f = req_find(own, r(e));
                                if f != 0 {
                                    f
                                } else {
                                    req_append_default(own, r(e), node)
                                }
                            };
                            if oe != 0 {
                                w(oe + 0x18, r(oe + 0x18) | (1u32 << (k & 31)));
                            }
                        }
                        ei += 1;
                        eo += 0x24;
                        if ei >= r(r(r(tn + 0x1C) + 8)) {
                            break;
                        }
                    }
                }
            }
            k += 1;
        }
        node = node.wrapping_add(0x28);
        count = count.wrapping_sub(1);
        if count == 0 {
            break;
        }
    }
    0
}

/// Root group (follow +0xC).
fn group_root(mut gr: u32) -> u32 {
    while r(gr + 0xC) != 0 {
        gr = r(gr + 0xC);
    }
    gr
}

/// Cluster groups (0x80F970, catalogue `FUN_0080f970`), returns 0 or `0x20` (group allocation
/// failed): every entry without a group gets a new one (scratch top, prepended to `*list`); then
/// for every output `k` to `t`, every requirement `te` of `t` (requirement != 0) whose cluster the
/// node's own entry `oe` carries on output `k` joins `oe`'s group, or, already grouped, the two
/// root groups merge (the second appended to the first's +4 chain and unlinked from the list).
#[hook(addr = 0x80F970, conv = cdecl, name = "FUN_0080f970", expect = "83 EC 1C 8B 44 24 20 53",
       unverified = "std4-rxpipe-1", route = "L2", hazards = "heap-writes")]
pub fn unlock_groups(pipe: u32, list: u32) -> u32 {
    let mut count = r(pipe + 4);
    let mut node = r(pipe + 8);
    loop {
        let mut i = 0u32;
        while i < r(r(r(node + 0x1C) + 8)) {
            let req = r(r(node + 0x1C) + 8);
            let e = r(req + 0xC).wrapping_add(i * 0x24);
            if r(e + 0xC) == 0 {
                let gr = scratch_top_call(0x10);
                if gr != 0 {
                    w(gr + 4, 0);
                    w(gr, 0);
                    w(gr + 0xC, 0);
                    w(gr + 8, r(list));
                    w(list, gr);
                }
                w(e + 0xC, gr);
                if gr == 0 {
                    error1(0x20);
                    return 0x20;
                }
                w(e + 0x10, r(gr));
                w(gr, e);
            }
            i += 1;
        }
        let mut k = 0u32;
        while k < r(node + 4) {
            let t = r(r(node + 8) + 4 * k);
            if t != 0xFFFF_FFFF {
                let tn = r(pipe + 8).wrapping_add(t.wrapping_mul(0x28));
                if r(r(r(tn + 0x1C) + 8)) > 0 {
                    let mut ti = 0u32;
                    loop {
                        let te = r(r(r(tn + 0x1C) + 8) + 0xC).wrapping_add(ti * 0x24);
                        if r(te + 4) != 0 {
                            let own = r(r(node + 0x1C) + 8);
                            let oe = req_find(own, r(te));
                            if oe != 0 && r(oe + 0x18) & (1u32 << (k & 31)) != 0 {
                                if r(te + 0xC) == 0 {
                                    let gr = r(oe + 0xC);
                                    w(te + 0x10, r(gr));
                                    w(gr, te);
                                    w(te + 0xC, r(oe + 0xC));
                                } else {
                                    let ra = group_root(r(te + 0xC));
                                    let rb = group_root(r(oe + 0xC));
                                    if ra != rb {
                                        let mut last = ra;
                                        while r(last + 4) != 0 {
                                            last = r(last + 4);
                                        }
                                        w(last + 4, rb);
                                        w(rb + 0xC, ra);
                                        let mut link = list;
                                        let mut cur = r(list);
                                        while cur != rb {
                                            link = cur + 8;
                                            cur = r(cur + 8);
                                        }
                                        w(link, r(r(link) + 8));
                                    }
                                }
                            }
                        }
                        ti += 1;
                        if ti >= r(r(r(tn + 0x1C) + 8)) {
                            break;
                        }
                    }
                }
            }
            k += 1;
        }
        node = node.wrapping_add(0x28);
        count = count.wrapping_sub(1);
        if count == 0 {
            break;
        }
    }
    0
}

/// Slot assignment (0x80FBA0, catalogue `FUN_0080fba0`), returns 0: for each group of the list,
/// the lowest slot free in every requirement set of its merged chain (trailing ones of the OR of
/// their masks) is given to each entry (`+0x1C`) and marked in its set; `packetNumClusterSlots` =
/// highest slot + 1 (0 for an empty list).
#[hook(addr = 0x80FBA0, conv = cdecl, name = "FUN_0080fba0", expect = "8B 44 24 08 53 55 33 ED",
       unverified = "std4-rxpipe-1", route = "L2", hazards = "heap-writes")]
pub fn unlock_slots(pipe: u32, list: u32) -> u32 {
    let mut slots = 0u32;
    let mut gr = r(list);
    while gr != 0 {
        let mut mask = 0u32;
        let mut c = gr;
        while c != 0 {
            let mut e = r(c);
            while e != 0 {
                mask |= r(r(e + 0x14) + 8);
                e = r(e + 0x10);
            }
            c = r(c + 4);
        }
        let mut slot = 0u32;
        while mask & 1 != 0 {
            mask >>= 1;
            slot += 1;
        }
        if slot >= slots {
            slots = slot + 1;
        }
        let mut c = gr;
        while c != 0 {
            let mut e = r(c);
            while e != 0 {
                w(e + 0x1C, slot);
                let s = r(e + 0x14);
                w(s + 8, r(s + 8) | (1u32 << (slot & 31)));
                e = r(e + 0x10);
            }
            c = r(c + 4);
        }
        gr = r(gr + 8);
    }
    w(pipe + 0xC, slots);
    0
}

/// Whether group `gr` of the list starting at `head` is the first with its head entry's cluster.
fn first_occurrence(head: u32, gr: u32) -> bool {
    let cd = r(r(gr));
    let mut e = head;
    while e != gr {
        if r(r(e)) == cd {
            return false;
        }
        e = r(e + 8);
    }
    true
}

/// Packet / cluster build (0x80FC30, catalogue `FUN_0080fc30`), returns 0; every allocation from the
/// scratch bottom, unchecked. The free window is zeroed; the embedded packet (`28 * slots + 0x14`:
/// numClusters, pipeline) at +0x14, state 0; one `RxPipelineCluster {cd, defaultAttributes}` per
/// first-occurrence group; per node slotClusterRefs, slotsContinue (`[0]` = AND of the masks),
/// inputToClusterSlot and the private data; the entry node's REQUIRED / OPTIONAL clusters as
/// `{cd, requirement, slot}` at +0x1C (+0x18 count), sorted by cluster pointer (0x8103D0).
#[hook(addr = 0x80FC30, conv = cdecl, name = "FUN_0080fc30", expect = "83 EC 18 53 8B 5C 24 20",
       route = "L2", hazards = "heap-writes")]
pub fn unlock_build(pipe: u32, list: u32) -> u32 {
    let req0 = r(r(r(pipe + 8) + 0x1C) + 8);
    let mut nreq = 0u32;
    for i in 0..r(req0) {
        let q = r(r(req0 + 0xC) + i * 0x24 + 4);
        if q == 1 || q == 2 {
            nreq += 1;
        }
    }
    let mut ngroups = 0u32;
    let mut c = list;
    while c != 0 {
        if first_occurrence(list, c) {
            ngroups += 1;
        }
        c = r(c + 8);
    }
    let top = scratch_top_call(0);
    let bot = scratch_bottom_call(0);
    let len = top.wrapping_sub(bot);
    for k in 0..len {
        g::<u8>(bot.wrapping_add(k) as usize).write(0);
    }
    let slots = r(pipe + 0xC);
    let pkt = scratch_bottom_call(slots.wrapping_mul(28).wrapping_add(0x14));
    w(pipe + 0x14, pkt);
    g::<u16>(pkt as usize + 2).write(r(pipe + 0xC) as u16);
    w(r(pipe + 0x14) + 4, pipe);
    w(pipe + 0x10, 0);
    let mut first = 0u32;
    let mut c = list;
    while c != 0 {
        if first_occurrence(list, c) {
            let cd = r(r(c));
            let pc = scratch_bottom_call(8);
            if first == 0 {
                first = pc;
            }
            w(pc, cd);
            w(pc + 4, r(cd + 8));
        }
        c = r(c + 8);
    }
    let mut i = 0u32;
    while i < r(pipe + 4) {
        let node = r(pipe + 8) + i * 0x28;
        if r(pipe + 0xC) > 0 {
            w(node + 0xC, scratch_bottom_call(r(pipe + 0xC) << 2));
        }
        w(node + 0x10, scratch_bottom_call((r(pipe + 0xC) << 2) + 4));
        let ncoi = r(r(node) + 0x20);
        if ncoi > 0 {
            w(node + 0x18, scratch_bottom_call(ncoi * 4));
        }
        let pds = r(r(node) + 0x34);
        if pds != 0 {
            w(node + 0x14, scratch_bottom_call(pds.wrapping_add(3) & !3));
        }
        let mut mask = 0xFFFF_FFFFu32;
        let mut j = 0u32;
        while j < r(r(r(node + 0x1C) + 8)) {
            let e = r(r(r(node + 0x1C) + 8) + 0xC).wrapping_add(j * 0x24);
            let mut pc = 0u32;
            for q in 0..ngroups {
                if r(first + q * 8) == r(e) {
                    pc = first + q * 8;
                    break;
                }
            }
            w(r(node + 0xC).wrapping_add(r(e + 0x1C).wrapping_mul(4)), pc);
            w(r(node + 0x10).wrapping_add(r(e + 0x1C).wrapping_mul(4)).wrapping_add(4), r(e + 0x18));
            mask &= r(e + 0x18);
            j += 1;
        }
        w(r(node + 0x10), mask);
        let mut k = 0u32;
        while k < r(r(node) + 0x20) {
            w(r(node + 0x18) + 4 * k, 0xFFFF_FFFF);
            let ns = r(pipe + 0xC);
            let mut s = 0u32;
            while s < ns {
                let rf = r(r(node + 0xC) + 4 * s);
                if rf != 0 && r(rf) == r(r(r(node) + 0x24) + k * 0xC) {
                    w(r(node + 0x18) + 4 * k, s);
                    break;
                }
                s += 1;
            }
            k += 1;
        }
        i += 1;
    }
    if nreq > 0 {
        w(pipe + 0x1C, scratch_bottom_call(nreq * 12));
    }
    let mut cnt = 0u32;
    let mut j = 0u32;
    while j < r(r(r(r(pipe + 8) + 0x1C) + 8)) {
        let e = r(r(r(r(pipe + 8) + 0x1C) + 8) + 0xC).wrapping_add(j * 0x24);
        let q = r(e + 4);
        if q == 1 || q == 2 {
            let d = r(pipe + 0x1C).wrapping_add(cnt * 12);
            cnt += 1;
            w(d, r(e));
            w(d + 4, r(e + 4));
            w(d + 8, r(e + 0x1C));
        }
        j += 1;
    }
    w(pipe + 0x18, cnt);
    sort_call(r(pipe + 0x1C), cnt, 0xC, 0, 0, 0xFFFF_FFFF);
    0
}

/// Swaps two elements of `stride / 4` dwords (`rep`-free loop of the original; nothing for
/// `stride < 4`).
fn swap_elems(a: u32, b: u32, stride: u32) {
    for k in 0..(stride >> 2) {
        let t = r(a + 4 * k);
        w(a + 4 * k, r(b + 4 * k));
        w(b + 4 * k, t);
    }
}

/// Radix exchange sort (0x8103D0, catalogue `FUN_008103d0`): `(base, count, stride, keyOffset,
/// keyMin, keyMax)`, unsigned u32 keys at `elem + keyOffset`, ascending, not stable. Nothing for a
/// NULL base, `keyOffset + 4 > stride` or `keyMin >= keyMax`. More than 5 elements: binary radix
/// exchange from the top bit of keyMax with an explicit stack (quirk kept: the lower part reuses
/// the parent's upper bound for its push test). More than 1: the minimum of elements 0..min(4,
/// count-1) (ties keep the higher index) is swapped to the front, then an insertion sort.
#[hook(addr = 0x8103D0, conv = cdecl, name = "FUN_008103d0", expect = "81 EC 94 01 00 00 53",
       route = "L2", hazards = "heap-writes")]
pub fn radix_sort(base: u32, count: u32, stride: u32, key: u32, key_min: u32, key_max: u32) {
    if base == 0 || key.wrapping_add(4) > stride || key_min >= key_max {
        return;
    }
    let kv = |e: u32| r(e.wrapping_add(key));
    if count > 5 {
        let shift = if key_max == 0 { 31 } else { 31 - key_max.leading_zeros() };
        let mut stack: [(u32, u32, u32); 32] = [(0, 0, 0); 32];
        stack[0] = (base, base.wrapping_add(count.wrapping_sub(1).wrapping_mul(stride)), 1u32 << shift);
        let mut sp = 1usize;
        while sp != 0 {
            sp -= 1;
            let (lo0, hi0, bit0) = stack[sp];
            let mut bit = bit0;
            let orig_hi = hi0;
            let orig_lo = lo0;
            let mut lo = lo0;
            let mut hi = hi0;
            loop {
                // partition by `bit`
                'part: loop {
                    if kv(lo) & bit == 0 {
                        loop {
                            lo = lo.wrapping_add(stride);
                            if lo > hi {
                                break 'part;
                            }
                            if kv(lo) & bit != 0 {
                                break;
                            }
                        }
                    }
                    if kv(hi) & bit != 0 {
                        loop {
                            hi = hi.wrapping_sub(stride);
                            if lo > hi {
                                break 'part;
                            }
                            if kv(hi) & bit == 0 {
                                break;
                            }
                        }
                    }
                    swap_elems(lo, hi, stride);
                    lo = lo.wrapping_add(stride);
                    hi = hi.wrapping_sub(stride);
                    if lo > hi {
                        break;
                    }
                }
                bit >>= 1;
                if bit == 0 {
                    break;
                }
                let up = hi.wrapping_add(stride);
                if orig_hi >= stride.wrapping_mul(5).wrapping_add(up) {
                    stack[sp] = (up, orig_hi, bit);
                    sp += 1;
                }
                hi = lo.wrapping_sub(stride);
                lo = orig_lo;
                if hi < stride.wrapping_mul(5).wrapping_add(orig_lo) {
                    break;
                }
            }
        }
    }
    if count <= 1 {
        return;
    }
    let last = count - 1;
    let m = if last < 4 { last } else { 4 };
    let mut best = m;
    let mut bk = kv(base.wrapping_add(m.wrapping_mul(stride)));
    let mut i = m;
    while i != 0 {
        i -= 1;
        let k = kv(base.wrapping_add(i.wrapping_mul(stride)));
        if k < bk {
            bk = k;
            best = i;
        }
    }
    if best != 0 {
        swap_elems(base, base.wrapping_add(best.wrapping_mul(stride)), stride);
    }
    let mut cur = base.wrapping_add(stride);
    for _ in 0..last {
        let ki = kv(cur);
        let mut j = cur.wrapping_sub(stride);
        while kv(j) > ki {
            swap_elems(j, j.wrapping_add(stride), stride);
            j = j.wrapping_sub(stride);
        }
        cur = cur.wrapping_add(stride);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_NODE_DEF_WORLD_SECTOR_ALL_IN_ONE,
    HOOK_SCRATCH_ALLOC_TOP,
    HOOK_SCRATCH_ALLOC_BOTTOM,
    HOOK_PIPELINE_COUNT_CLUSTERS,
    HOOK_PIPELINE_RESIZE,
    HOOK_PIPELINE_COPY_INTO,
    HOOK_PIPELINE_TOP_SORT,
    HOOK_PIPELINE_NODE_DESTROY,
    HOOK_PIPELINE_NODE_INIT,
    HOOK_RX_PIPELINE_LOCK,
    HOOK_RX_LOCKED_PIPE_ADD_FRAGMENT,
    HOOK_RX_PIPELINE_FIND_NODE_BY_NAME,
    HOOK_RX_LOCKED_PIPE_UNLOCK,
    HOOK_UNLOCK_RESOLVE,
    HOOK_REQ_CREATE,
    HOOK_REQ_ADD,
    HOOK_IO_FIND_CLUSTER,
    HOOK_REQ_RELEASE,
    HOOK_UNLOCK_FORWARD,
    HOOK_UNLOCK_GROUPS,
    HOOK_UNLOCK_SLOTS,
    HOOK_UNLOCK_BUILD,
    HOOK_RADIX_SORT,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = RXC_GLOBALS;
