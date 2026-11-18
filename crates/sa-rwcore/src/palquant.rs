//! RwPalQuant: the octree palette quantiser behind palettised textures read from image files.
//!
//! The quantiser (0x4008 bytes, a caller's local): +0 boxes (0x20 each: min r/g/b/a, max r/g/b/a as
//! ints), +0x2000 the boxes' variances (f32), +0x2400 the boxes' statistics (0x1C each), +0x4000 the
//! octree root, +0x4004 the node free list. A node (0x5C) starts with the statistics (+0 weight, +4..+0x10
//! the r / g / b / a sums, +0x14 the sum of squares, +0x18 the palette index) and has 16 children at
//! +0x1C. The tree is `[0x8E26E0]` levels deep; a pixel's path interleaves the top bits of its four
//! channels through the spread table at 0xC9A730. Arithmetic is x87 (PC=24) in the original's order.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;
use sa_math::x87::{Cmp, X87};

use crate::bindings::{free_list_create, free_list_destroy};
use crate::common::{g, p8, rd, rw_memory_alloc, rw_memory_free, wr};
use crate::image2::fistp_chop_f32;

const DEPTH: usize = 0x8E_26E0;
const SPREAD: usize = 0xC9_A730;
const NODE_HINT: u32 = 0x3_0411;
const ROOT: isize = 0x4000;
const FREE_LIST: isize = 0x4004;
const BOX_VARIANCE: isize = 0x2000;
const BOX_STATS: isize = 0x2400;
/// 1/255 (0x859A3C) and the channel weights of red (0x885004) and blue (0x885000), with their inverses
/// (0x885010, 0x88500C) and 255.99 (0x885008).
const INV_255: f32 = f32::from_bits(0x3B80_8081);
const RED_WEIGHT: f32 = f32::from_bits(0x3F02_660D);
const BLUE_WEIGHT: f32 = f32::from_bits(0x3E46_DE6B);
const RED_UNWEIGHT: f32 = f32::from_bits(0x3FFB_4A7D);
const BLUE_UNWEIGHT: f32 = f32::from_bits(0x40A4_C59D);
const COLOUR_SCALE: f32 = f32::from_bits(0x437F_FD71);

fn depth() -> i32 {
    g::<i32>(DEPTH).read()
}

fn spread(v: u8, shift: i32) -> u32 {
    rd::<u32>(p8(SPREAD as u32), (u32::from(v) >> (shift & 0x1F)) as isize * 4)
}

/// The octree path of an RGBA colour: its channels' top bits interleaved, red highest.
fn colour_path(c: GPtr<u8>) -> u32 {
    let s = 8 - depth();
    (((spread(rd(c, 0), s) << 1 | spread(rd(c, 1), s)) << 1 | spread(rd(c, 2), s)) << 1) | spread(rd(c, 3), s)
}

fn child_slot(node: GPtr<u8>, path: u32) -> GPtr<u8> {
    node.byte_add(0x1C + (path & 0xF) as isize * 4)
}

fn clear_stats(s: GPtr<u8>) {
    wr::<u8>(s, 0x18, 0);
    for k in 0..6 {
        wr::<u32>(s, k * 4, 0);
    }
}

/// A new node from the free list with no children (a leaf's statistics cleared too).
fn new_node(pq: GPtr<u8>, leaf: bool) -> GPtr<u8> {
    let n = rw_memory_alloc(p8(rd::<u32>(pq, FREE_LIST)), NODE_HINT);
    for k in 0..16 {
        wr::<u32>(n, 0x1C + k * 4, 0);
    }
    if leaf {
        clear_stats(n);
    }
    n
}

/// `RwPalQuantInit(pq)` (0x80C470): the spread table, the node free list and an empty root.
#[hook(addr = 0x80C470, conv = cdecl, name = "RwPalQuantInit", expect = "51 55 56 57 8B 3D E0 26 8E 00",
       unverified = "frontier2-c33", route = "L2", hazards = "heap-writes")]
pub fn rw_pal_quant_init(pq: GPtr<u8>) -> u32 {
    let bits = depth() as u32;
    let count = 1u32 << (bits & 0x1F);
    let mut v = 0u32;
    while v < count {
        let mut out = 0u32;
        for j in 0..bits {
            if v & (1 << (j & 0x1F)) != 0 {
                out |= 1 << ((bits * 4 - 4 - j * 4) & 0x1F);
            }
        }
        wr::<u32>(p8(SPREAD as u32), v as isize * 4, out);
        v += 1;
    }
    let fl = free_list_create(0x5C, 0x400, 4, NODE_HINT);
    wr::<u32>(pq, FREE_LIST, fl.addr() as u32);
    let root = rw_memory_alloc(fl, NODE_HINT);
    wr::<u32>(pq, ROOT, root.addr() as u32);
    for k in 0..16 {
        wr::<u32>(root, 0x1C + k * 4, 0);
    }
    1
}

/// `_rwPalQuantOctreeInsert(pq, node, path, levels)` (0x80AED0): the node `levels` steps down `path`,
/// created as needed.
#[hook(addr = 0x80AED0, conv = cdecl, name = "FUN_0080aed0", expect = "55 8B 6C 24 14",
       unverified = "frontier2-c33", route = "L2", hazards = "heap-writes")]
pub fn pal_quant_octree_insert(pq: GPtr<u8>, mut node: GPtr<u8>, mut path: u32, mut levels: i32) -> GPtr<u8> {
    while levels != 0 {
        let slot = child_slot(node, path);
        if rd::<u32>(slot, 0) == 0 {
            wr::<u32>(slot, 0, new_node(pq, levels == 1).addr() as u32);
        }
        node = p8(rd::<u32>(slot, 0));
        levels -= 1;
        path >>= 4;
    }
    node
}

/// The channels of a colour (bytes masked by `mask`) scaled to [0, 1]: red and blue weighted, red and
/// green kept in registers, blue and alpha stored.
struct Channels {
    r: X87,
    g: X87,
    b: f32,
    a: f32,
}

fn channels(c: GPtr<u8>, mask: u8) -> Channels {
    let ch = |k: isize| X87::from_i32(i32::from(rd::<u8>(c, k) & mask)).fmul_m(INV_255);
    let (r, g, b, a) = (ch(0), ch(1), ch(2), ch(3));
    let a = a.to_f32();
    Channels { r: r.fmul_m(RED_WEIGHT), g, b: b.fmul_m(BLUE_WEIGHT).to_f32(), a }
}

/// Adds the weighted channels to a node's colour sums.
fn add_weighted(node: GPtr<u8>, c: &Channels, weight: impl Fn() -> f32) {
    let r = c.r.fmul_m(weight()).to_f32();
    let gw = c.g.fmul_m(weight());
    let bw = X87::from_f32(c.b).fmul_m(weight());
    let a = X87::from_f32(c.a).fmul_m(weight()).to_f32();
    wr::<f32>(node, 4, X87::from_f32(rd(node, 4)).fadd_m(r).to_f32());
    wr::<f32>(node, 8, X87::from_f32(rd(node, 8)).fadd(gw).to_f32());
    wr::<f32>(node, 0xC, X87::from_f32(rd(node, 0xC)).fadd(bw).to_f32());
    wr::<f32>(node, 0x10, X87::from_f32(rd(node, 0x10)).fadd_m(a).to_f32());
}

/// `_rwPalQuantAddPixel(node, colour, weight)` (0x80AD90): the colour's offset inside its leaf cell
/// added to the leaf's weight, sums and sum of squares.
#[hook(addr = 0x80AD90, conv = cdecl, name = "FUN_0080ad90", expect = "83 EC 18 A1 E0 26 8E 00",
       unverified = "frontier2-c33", route = "L2", hazards = "heap-writes")]
pub fn pal_quant_add_pixel(node: GPtr<u8>, colour: GPtr<u8>, weight: f32) {
    let shift = (8 - depth()) as u32 & 0x1F;
    let mask = if shift >= 8 { 0u8 } else { 1u8 << shift }.wrapping_sub(1);
    let c = channels(colour, mask);
    wr::<f32>(node, 0, X87::from_f32(weight).fadd_m(rd(node, 0)).to_f32());
    let sq = X87::from_f32(c.a)
        .fmul_m(c.a)
        .fadd(X87::from_f32(c.b).fmul_m(c.b))
        .fadd(c.g.fmul(c.g))
        .fadd(c.r.fmul(c.r))
        .fmul_m(weight)
        .fadd_m(rd(node, 0x14));
    wr::<f32>(node, 0x14, sq.to_f32());
    add_weighted(node, &c, || weight);
}

/// `RwPalQuantAddImage(pq, image, weight)` (0x80AA80): every pixel of a 4/8-bit (through its palette)
/// or 32-bit image into its leaf.
#[hook(addr = 0x80AA80, conv = cdecl, name = "RwPalQuantAddImage", expect = "83 EC 1C 8B 54 24 24 53",
       unverified = "frontier2-c33", route = "L2", hazards = "heap-writes")]
pub fn rw_pal_quant_add_image(pq: GPtr<u8>, img: GPtr<u8>, weight: f32) {
    let stride = rd::<i32>(img, 0x10);
    let palette = rd::<u32>(img, 0x18);
    let mut row = rd::<u32>(img, 0x14);
    let d = rd::<i32>(img, 0xC);
    let step: u32 = match d {
        4 | 8 => 1,
        0x20 => 4,
        _ => return,
    };
    let mut rows = rd::<i32>(img, 8);
    while rows != 0 {
        let mut px = row;
        let mut cols = rd::<i32>(img, 4);
        while cols != 0 {
            let colour = if step == 1 { p8(palette.wrapping_add(u32::from(rd::<u8>(p8(px), 0)) * 4)) } else { p8(px) };
            let path = colour_path(colour);
            let levels = depth();
            let mut node = p8(rd::<u32>(pq, ROOT));
            if levels != 0 {
                let slot = child_slot(node, path);
                if rd::<u32>(slot, 0) == 0 {
                    wr::<u32>(slot, 0, new_node(pq, levels == 1).addr() as u32);
                }
                node = pal_quant_octree_insert(pq, p8(rd::<u32>(slot, 0)), path >> 4, levels - 1);
            }
            pal_quant_add_pixel(node, colour, weight);
            px = px.wrapping_add(step);
            cols -= 1;
        }
        row = row.wrapping_add(stride as u32);
        rows -= 1;
    }
}

/// `_rwPalQuantMergeStats(dst, a, b)` (0x80B910): the merged statistics, the sum of squares growing by
/// the between-means term when both weights are positive. `dst` may be `a`.
#[hook(addr = 0x80B910, conv = cdecl, name = "FUN_0080b910", expect = "83 EC 20 8B 4C 24 28",
       unverified = "frontier2-c33", route = "L2", hazards = "heap-writes")]
pub fn pal_quant_merge_stats(dst: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) {
    let var = X87::from_f32(rd(a, 0x14)).fadd_m(rd(b, 0x14)).to_f32();
    wr::<f32>(dst, 0x14, var);
    let positive = |p: GPtr<u8>| X87::from_f32(rd(p, 0)).fcom(X87::zero()) == Cmp::Greater;
    if positive(a) && positive(b) {
        let ia = X87::one().fdiv_m(rd(a, 0));
        let ib = X87::one().fdiv_m(rd(b, 0));
        let ar = X87::from_f32(rd(a, 4)).fmul(ia);
        let ag = X87::from_f32(rd(a, 8)).fmul(ia);
        let ab = X87::from_f32(rd(a, 0xC)).fmul(ia).to_f32();
        let aa = X87::from_f32(rd(a, 0x10)).fmul(ia).to_f32();
        let br = X87::from_f32(rd(b, 4)).fmul(ib);
        let bg = X87::from_f32(rd(b, 8)).fmul(ib).to_f32();
        let bb = X87::from_f32(rd(b, 0xC)).fmul(ib).to_f32();
        let ba = X87::from_f32(rd(b, 0x10)).fmul(ib).to_f32();
        let dr = ar.fsub(br).to_f32();
        let dg = ag.fsub_m(bg);
        let db = X87::from_f32(ab).fsub_m(bb);
        let da = X87::from_f32(aa).fsub_m(ba);
        let q = da.fmul(da).fadd(db.fmul(db)).fadd(dg.fmul(dg)).fadd(X87::from_f32(dr).fmul_m(dr));
        let t = ib.fadd(ia);
        wr::<f32>(dst, 0x14, q.fdiv(t).fadd_m(var).to_f32());
    }
    for k in 1..5 {
        wr::<f32>(dst, k * 4, X87::from_f32(rd(a, k * 4)).fadd_m(rd(b, k * 4)).to_f32());
    }
    wr::<f32>(dst, 0, X87::from_f32(rd(a, 0)).fadd_m(rd(b, 0)).to_f32());
}

/// `_rwPalQuantAssignIndices(node, cellColour, levels)` (0x80BCA0): turns the leaves' sums of squares
/// into variances about their means, moves their sums from the cell to absolute colours, and gathers
/// every inner node's statistics from its children. Returns the node (NULL for none).
#[hook(addr = 0x80BCA0, conv = cdecl, name = "FUN_0080bca0", expect = "83 EC 10 33 C0 53 56",
       unverified = "frontier2-c33", route = "L2", hazards = "heap-writes")]
pub fn pal_quant_assign_indices(node: GPtr<u8>, cell: GPtr<u8>, levels: i32) -> GPtr<u8> {
    if node.is_null() {
        return GPtr::NULL;
    }
    if levels > 0 {
        clear_stats(node);
        let shift = (levels - depth() + 7) as u32 & 0x1F;
        for i in 0..16u32 {
            let bit = |k: u32| (((i >> k) & 1) << shift) as u8;
            let mut child_cell = [
                bit(3).wrapping_add(rd(cell, 0)),
                bit(2).wrapping_add(rd(cell, 1)),
                bit(1).wrapping_add(rd(cell, 2)),
                bit(0).wrapping_add(rd(cell, 3)),
            ];
            let child = p8(rd::<u32>(node, 0x1C + i as isize * 4));
            let r = pal_quant_assign_indices(child, GPtr::from_raw(child_cell.as_mut_ptr()), levels - 1);
            if !r.is_null() {
                pal_quant_merge_stats(node, node, r);
            }
        }
        return node;
    }
    let (r, gr, b, a) = (X87::from_f32(rd(node, 4)), X87::from_f32(rd(node, 8)), X87::from_f32(rd(node, 0xC)), X87::from_f32(rd(node, 0x10)));
    let s = r.fmul(r).fadd(gr.fmul(gr)).fadd(b.fmul(b)).fadd(a.fmul(a)).fdiv_m(rd(node, 0));
    let v = X87::from_f32(rd(node, 0x14)).fsub(s);
    wr::<f32>(node, 0x14, v.to_f32());
    if v.fcom(X87::zero()) == Cmp::Less {
        wr::<u32>(node, 0x14, 0);
    }
    let c = channels(cell, 0xFF);
    add_weighted(node, &c, || rd::<f32>(node, 0));
    node
}

/// `_rwPalQuantCountLeaves(node, levels)` (0x80BE60).
#[hook(addr = 0x80BE60, conv = cdecl, name = "FUN_0080be60", expect = "8B 4C 24 04 55",
       unverified = "frontier2-c33", route = "L2", hazards = "none")]
pub fn pal_quant_count_leaves(node: GPtr<u8>, levels: i32) -> i32 {
    if node.is_null() {
        return 0;
    }
    if levels < 1 {
        return 1;
    }
    (0..16).fold(0i32, |n, k| n.wrapping_add(pal_quant_count_leaves(p8(rd::<u32>(node, 0x1C + k * 4)), levels - 1)))
}

/// `_rwPalQuantFillPalette(node, palette, next, levels)` (0x80BEB0): one palette entry per leaf, in tree
/// order, each leaf remembering its index. Returns the next free index.
#[hook(addr = 0x80BEB0, conv = cdecl, name = "FUN_0080beb0", expect = "53 56 8B 74 24 0C 85 F6",
       unverified = "frontier2-c33", route = "L2", hazards = "heap-writes")]
pub fn pal_quant_fill_palette(node: GPtr<u8>, palette: GPtr<u8>, mut next: i32, levels: i32) -> i32 {
    if node.is_null() {
        return next;
    }
    if levels > 0 {
        for k in 0..16 {
            next = pal_quant_fill_palette(p8(rd::<u32>(node, 0x1C + k * 4)), palette, next, levels - 1);
        }
        return next;
    }
    pal_quant_node_to_colour(palette.byte_add(next.wrapping_mul(4) as isize), node);
    wr::<u8>(node, 0x18, next as u8);
    next.wrapping_add(1)
}

/// `_rwPalQuantNodeToColour(rgba, stats)` (0x80B490): the statistics' mean colour, unweighted, scaled
/// by 255.99 and truncated; 255 for anything not below it.
#[hook(addr = 0x80B490, conv = cdecl, name = "FUN_0080b490", expect = "83 EC 28 8B 44 24 30",
       unverified = "frontier2-c33", route = "L2", hazards = "heap-writes")]
pub fn pal_quant_node_to_colour(out: GPtr<u8>, s: GPtr<u8>) {
    let inv = X87::one().fdiv_m(rd(s, 0));
    let r = X87::from_f32(rd(s, 4)).fmul(inv);
    let gr = X87::from_f32(rd(s, 8)).fmul(inv).to_f32();
    let b = X87::from_f32(rd(s, 0xC)).fmul(inv).to_f32();
    let a = X87::from_f32(rd(s, 0x10)).fmul(inv).to_f32();
    let r = r.fmul_m(RED_UNWEIGHT);
    let b = X87::from_f32(b).fmul_m(BLUE_UNWEIGHT).to_f32();
    let ir = fistp_chop_f32(r.fmul_m(COLOUR_SCALE).to_f32());
    let ig = fistp_chop_f32(X87::from_f32(gr).fmul_m(COLOUR_SCALE).to_f32());
    let ib = fistp_chop_f32(X87::from_f32(b).fmul_m(COLOUR_SCALE).to_f32());
    let ia = fistp_chop_f32(X87::from_f32(a).fmul_m(COLOUR_SCALE).to_f32());
    for (k, v) in [ir, ig, ib, ia].into_iter().enumerate() {
        wr::<u8>(out, k as isize, if v < 0xFF { v as u8 } else { 0xFF });
    }
}

/// The child cell origins: bit 3 of the child index moves red, bit 0 alpha, by `1 << shift`.
fn child_origin(origin: GPtr<i32>, i: u32, shift: i32) -> [i32; 4] {
    let s = (shift as u32) & 0x1F;
    core::array::from_fn(|c| ((((i >> (3 - c)) & 1) << s) as i32).wrapping_add(rd::<i32>(origin.cast(), c as isize * 4)))
}

/// Whether the cell `[origin, origin + size)` overlaps the box (`min` at +0, `max` at +0x10, exclusive).
fn cell_overlaps(origin: GPtr<i32>, size: i32, bx: GPtr<u8>) -> bool {
    (0..4).all(|c| {
        let lo = rd::<i32>(origin.cast(), c * 4);
        let hi = size.wrapping_add(lo);
        hi.wrapping_sub(rd::<i32>(bx, c * 4)) > 0 && lo.wrapping_sub(rd::<i32>(bx, 0x10 + c * 4)) < 0
    })
}

/// `_rwPalQuantBoxStats(node, origin, levels, box, stats)` (0x80B730): adds the statistics of every
/// subtree whose cell lies inside the box.
#[hook(addr = 0x80B730, conv = cdecl, name = "FUN_0080b730", expect = "8B 44 24 04 83 EC 28",
       unverified = "frontier2-c33", route = "L2", hazards = "heap-writes")]
pub fn pal_quant_box_stats(node: GPtr<u8>, origin: GPtr<i32>, levels: i32, bx: GPtr<u8>, stats: GPtr<u8>) {
    if node.is_null() {
        return;
    }
    let size = 1i32 << (levels as u32 & 0x1F);
    if !cell_overlaps(origin, size, bx) {
        return;
    }
    let inside = (0..4).all(|c| {
        let lo = rd::<i32>(origin.cast(), c * 4);
        lo.wrapping_sub(rd::<i32>(bx, c * 4)) >= 0 && rd::<i32>(bx, 0x10 + c * 4).wrapping_sub(size.wrapping_add(lo)) >= 0
    });
    if inside {
        pal_quant_merge_stats(stats, stats, node);
        return;
    }
    if levels > 0 {
        for i in 0..16u32 {
            let mut o = child_origin(origin, i, levels - 1);
            let child = p8(rd::<u32>(node, 0x1C + i as isize * 4));
            pal_quant_box_stats(child, GPtr::from_raw(o.as_mut_ptr()), levels - 1, bx, stats);
        }
    }
}

/// `_rwPalQuantBuildBoxes(node, origin, levels, box, index)` (0x80B5D0): every leaf inside the box gets
/// the box's palette index.
#[hook(addr = 0x80B5D0, conv = cdecl, name = "FUN_0080b5d0", expect = "8B 44 24 04 83 EC 24",
       unverified = "frontier2-c33", route = "L2", hazards = "heap-writes")]
pub fn pal_quant_build_boxes(node: GPtr<u8>, origin: GPtr<i32>, levels: i32, bx: GPtr<u8>, index: u32) {
    if node.is_null() {
        return;
    }
    if !cell_overlaps(origin, 1i32 << (levels as u32 & 0x1F), bx) {
        return;
    }
    if levels == 0 {
        wr::<u8>(node, 0x18, index as u8);
        return;
    }
    for i in 0..16u32 {
        let mut o = child_origin(origin, i, levels - 1);
        let child = p8(rd::<u32>(node, 0x1C + i as isize * 4));
        pal_quant_build_boxes(child, GPtr::from_raw(o.as_mut_ptr()), levels - 1, bx, index);
    }
}

/// `_rwPalQuantSplitCost(root, box, axis, cuts, stats)` (0x80BA30): bisects the box along `axis` for the
/// cut that minimises the two halves' summed variance; `cuts[axis]` gets the best cut. Returns the best
/// summed variance (the box's own when no cut improves it).
#[hook(addr = 0x80BA30, conv = cdecl, name = "FUN_0080ba30", expect = "81 EC 8C 00 00 00 53 55",
       unverified = "frontier2-c33", route = "L2", hazards = "heap-writes")]
pub fn pal_quant_split_cost(root: GPtr<u8>, bx: GPtr<u8>, axis: i32, cuts: GPtr<i32>, stats: GPtr<u8>) -> X87 {
    let mut best = rd::<f32>(stats, 0x14);
    let mut half: [i32; 8] = core::array::from_fn(|k| rd::<i32>(bx, k as isize * 4));
    let mut hi = rd::<i32>(bx, 0x10 + axis as isize * 4);
    let mut lo = rd::<i32>(bx, axis as isize * 4);
    let mut origin = [0i32; 4];
    while hi.wrapping_sub(lo) > 1 {
        let mid = (hi.wrapping_sub(lo) >> 1).wrapping_add(lo);
        half[4 + axis as usize] = mid;
        let mut s = [0u32; 7];
        let sp = GPtr::<u32>::from_raw(s.as_mut_ptr()).cast::<u8>();
        pal_quant_box_stats(root, GPtr::from_raw(origin.as_mut_ptr()), depth(), GPtr::<i32>::from_raw(half.as_mut_ptr()).cast(), sp);
        let d: [f32; 5] = core::array::from_fn(|k| X87::from_f32(rd(stats, k as isize * 4)).fsub_m(rd(sp, k as isize * 4)).to_f32());
        let mut dv = X87::from_f32(rd(stats, 0x14)).fsub_m(rd(sp, 0x14));
        let positive = |v: f32| X87::from_f32(v).fcom(X87::zero()) == Cmp::Greater;
        if positive(d[0]) && positive(rd(sp, 0)) {
            let is = X87::one().fdiv_m(rd(sp, 0));
            let id = X87::one().fdiv_m(d[0]);
            let sr = X87::from_f32(rd(sp, 4)).fmul(is);
            let sg = X87::from_f32(rd(sp, 8)).fmul(is);
            let sb = X87::from_f32(rd(sp, 0xC)).fmul(is).to_f32();
            let sa = X87::from_f32(rd(sp, 0x10)).fmul(is).to_f32();
            let dr = X87::from_f32(d[1]).fmul(id);
            let dg = X87::from_f32(d[2]).fmul(id).to_f32();
            let db = X87::from_f32(d[3]).fmul(id).to_f32();
            let da = X87::from_f32(d[4]).fmul(id).to_f32();
            let x0 = sr.fsub(dr).to_f32();
            let x1 = sg.fsub_m(dg);
            let x2 = X87::from_f32(sb).fsub_m(db);
            let x3 = X87::from_f32(sa).fsub_m(da);
            let q = x3.fmul(x3).fadd(x2.fmul(x2)).fadd(x1.fmul(x1)).fadd(X87::from_f32(x0).fmul_m(x0));
            dv = dv.fsub(q.fdiv(id.fadd(is)));
        }
        let zero = |v: f32| X87::from_f32(v).fcom(X87::zero()) == Cmp::Equal;
        if zero(rd(sp, 0)) {
            lo = mid;
        } else if zero(d[0]) {
            hi = mid;
        } else {
            let total = X87::from_f32(rd(sp, 0x14)).fadd(dv);
            if total.fcom(X87::from_f32(best)) == Cmp::Less {
                best = total.to_f32();
                wr::<i32>(cuts.cast(), axis as isize * 4, half[4 + axis as usize]);
            }
            if X87::from_f32(rd(sp, 0x14)).fcom(dv) == Cmp::Less {
                lo = mid;
            } else {
                hi = mid;
            }
        }
    }
    X87::from_f32(best)
}

fn boxp(pq: GPtr<u8>, i: i32) -> GPtr<u8> {
    pq.byte_add(i as isize * 0x20)
}

fn statp(pq: GPtr<u8>, i: i32) -> GPtr<u8> {
    pq.byte_add(BOX_STATS + i as isize * 0x1C)
}

fn varp(pq: GPtr<u8>, i: i32) -> GPtr<u8> {
    pq.byte_add(BOX_VARIANCE + i as isize * 4)
}

fn box_stats_of(pq: GPtr<u8>, bx: GPtr<u8>, stats: GPtr<u8>) {
    clear_stats(stats);
    let mut origin = [0i32; 4];
    pal_quant_box_stats(p8(rd::<u32>(pq, ROOT)), GPtr::from_raw(origin.as_mut_ptr()), depth(), bx, stats);
}

/// `RwPalQuantResolvePalette(palette, maxColours, pq)` (0x80AF60): with no more leaves than colours one
/// entry per leaf; otherwise the colour space is split into `maxColours` boxes, always the box with the
/// largest variance along its best axis, and each box's mean becomes an entry. Returns the number of
/// colours (`maxColours` in the split case).
#[hook(addr = 0x80AF60, conv = cdecl, name = "RwPalQuantResolvePalette", expect = "83 EC 68 A1 E0 26 8E 00",
       unverified = "frontier2-c33", route = "L2", hazards = "heap-writes",
       note = "boxes past the last split (only when every variance reached 0) are read from the quantiser as the caller left it")]
pub fn rw_pal_quant_resolve_palette(palette: GPtr<u8>, max: i32, pq: GPtr<u8>) -> i32 {
    let mut zero_cell = [0u8; 4];
    pal_quant_assign_indices(p8(rd::<u32>(pq, ROOT)), GPtr::from_raw(zero_cell.as_mut_ptr()), depth());
    for i in 0..max.max(0) as isize {
        wr::<u32>(palette, i * 4, 0);
    }
    let root = p8(rd::<u32>(pq, ROOT));
    let mut leaves = 0i32;
    if !root.is_null() {
        if depth() < 1 {
            leaves = 1;
        } else {
            let levels = depth() - 1;
            for k in 0..16 {
                leaves = leaves.wrapping_add(pal_quant_count_leaves(p8(rd::<u32>(root, 0x1C + k * 4)), levels));
            }
        }
    }
    if leaves <= max {
        let root = p8(rd::<u32>(pq, ROOT));
        if root.is_null() {
            return leaves;
        }
        if depth() <= 0 {
            pal_quant_node_to_colour(palette, root);
            wr::<u8>(root, 0x18, 0);
            return leaves;
        }
        let levels = depth() - 1;
        let mut next = 0i32;
        for k in 0..16 {
            next = pal_quant_fill_palette(p8(rd::<u32>(root, 0x1C + k * 4)), palette, next, levels);
        }
        return leaves;
    }
    for c in 0..4 {
        wr::<i32>(pq, c * 4, 0);
    }
    for c in 0..4 {
        wr::<i32>(pq, 0x10 + c * 4, 1 << (depth() as u32 & 0x1F));
    }
    box_stats_of(pq, boxp(pq, 0), statp(pq, 0));
    wr::<u32>(varp(pq, 0), 0, rd::<u32>(statp(pq, 0), 0x14));
    let mut n = 1i32;
    while n < max {
        if n <= 0 {
            break;
        }
        let mut top = X87::zero();
        let mut idx = -1i32;
        for i in 0..n {
            let v = X87::from_f32(rd(varp(pq, i), 0));
            if top.fcom(v) == Cmp::Less {
                top = v;
                idx = i;
            }
        }
        if idx == -1 {
            break;
        }
        let root = p8(rd::<u32>(pq, ROOT));
        let (bx, st) = (boxp(pq, idx), statp(pq, idx));
        let mut cuts = [0i32; 4];
        let cp = GPtr::<i32>::from_raw(cuts.as_mut_ptr());
        let costs: [f32; 4] = core::array::from_fn(|a| pal_quant_split_cost(root, bx, a as i32, cp, st).to_f32());
        let mut j = 0usize;
        for c in 0..4 {
            if X87::from_f32(costs[c]).fcom(X87::from_f32(costs[j])) == Cmp::Less {
                j = c;
            }
        }
        if X87::from_f32(costs[j]).fcom(X87::from_f32(rd(st, 0x14))) == Cmp::Less {
            let nb = boxp(pq, n);
            for k in 0..8 {
                wr::<u32>(nb, k * 4, rd::<u32>(bx, k * 4));
            }
            let cut = cuts[j] as u8 as i32;
            wr::<i32>(nb, j as isize * 4, cut);
            wr::<i32>(bx, 0x10 + j as isize * 4, cut);
            box_stats_of(pq, bx, st);
            box_stats_of(pq, nb, statp(pq, n));
            wr::<u32>(varp(pq, idx), 0, rd::<u32>(st, 0x14));
            wr::<u32>(varp(pq, n), 0, rd::<u32>(statp(pq, n), 0x14));
        } else {
            wr::<u32>(varp(pq, idx), 0, 0);
            n -= 1;
        }
        n += 1;
    }
    for i in 0..max.max(0) {
        let mut origin = [0i32; 4];
        pal_quant_build_boxes(p8(rd::<u32>(pq, ROOT)), GPtr::from_raw(origin.as_mut_ptr()), depth(), boxp(pq, i), i as u32);
        pal_quant_node_to_colour(palette.byte_add(i as isize * 4), statp(pq, i));
    }
    max
}

/// `_rwPalQuantLookup(node, path, levels)` (0x80C430): the palette index of the leaf on `path`.
#[hook(addr = 0x80C430, conv = cdecl, name = "FUN_0080c430", expect = "8B 44 24 0C 85 C0 75 08",
       unverified = "frontier2-c33", route = "L2", hazards = "none")]
pub fn pal_quant_lookup(mut node: GPtr<u8>, mut path: u32, mut levels: i32) -> u8 {
    while levels != 0 {
        node = p8(rd::<u32>(child_slot(node, path), 0));
        path >>= 4;
        levels -= 1;
    }
    rd::<u8>(node, 0x18)
}

fn palette_index(pq: GPtr<u8>, colour: GPtr<u8>) -> u8 {
    let path = colour_path(colour);
    let root = p8(rd::<u32>(pq, ROOT));
    if depth() == 0 {
        return rd::<u8>(root, 0x18);
    }
    pal_quant_lookup(p8(rd::<u32>(child_slot(root, path), 0)), path >> 4, depth() - 1)
}

/// `RwPalQuantMatchImage(dst, dstStride, dstDepth, packed, pq, image)` (0x80BF20): every pixel of a
/// 4/8-bit (through its palette) or 32-bit image as its palette index: one byte per pixel, or two per
/// byte (low nibble first) for a 4-bit destination with `packed == 1`.
#[hook(addr = 0x80BF20, conv = cdecl, name = "RwPalQuantMatchImage", expect = "83 EC 14 8B 44 24 2C",
       unverified = "frontier2-c33", route = "L2", hazards = "heap-writes")]
pub fn rw_pal_quant_match_image(mut dst: GPtr<u8>, dst_stride: i32, dst_depth: u32, packed: u32, pq: GPtr<u8>, img: GPtr<u8>) {
    let stride = rd::<i32>(img, 0x10);
    let mut row = rd::<u32>(img, 0x14);
    let d = rd::<i32>(img, 0xC);
    let palette = rd::<u32>(img, 0x18);
    let step: u32 = match d {
        4 | 8 => 1,
        0x20 => 4,
        _ => return,
    };
    let colour = |px: u32| if step == 1 { p8(palette.wrapping_add(u32::from(rd::<u8>(p8(px), 0)) * 4)) } else { p8(px) };
    let nibbles = dst_depth == 4 && packed == 1;
    let mut rows = rd::<i32>(img, 8);
    if nibbles {
        while rows != 0 {
            let mut out = dst;
            let mut px = row;
            let width = rd::<u32>(img, 4);
            let mut x = 0u32;
            while x < width {
                let v = palette_index(pq, colour(px));
                let o = rd::<u8>(out, 0);
                if x & 1 == 0 {
                    wr::<u8>(out, 0, o ^ ((o ^ v) & 0xF));
                } else {
                    wr::<u8>(out, 0, (o & 0xF) | v << 4);
                    out = out.byte_add(1);
                }
                x += 1;
                px = px.wrapping_add(step);
            }
            dst = dst.byte_add(dst_stride as isize);
            row = row.wrapping_add(stride as u32);
            rows -= 1;
        }
        return;
    }
    while rows != 0 {
        let mut out = dst;
        let mut px = row;
        let mut cols = rd::<i32>(img, 4);
        while cols != 0 {
            wr::<u8>(out, 0, palette_index(pq, colour(px)));
            out = out.byte_add(1);
            px = px.wrapping_add(step);
            cols -= 1;
        }
        dst = dst.byte_add(dst_stride as isize);
        row = row.wrapping_add(stride as u32);
        rows -= 1;
    }
}

/// `_rwPalQuantFreeTree(pq, node, levels)` (0x80C5A0): the subtree back to the free list.
#[hook(addr = 0x80C5A0, conv = cdecl, name = "FUN_0080c5a0", expect = "56 8B 74 24 0C",
       unverified = "frontier2-c33", route = "L2", hazards = "heap-writes")]
pub fn pal_quant_free_tree(pq: GPtr<u8>, node: GPtr<u8>, levels: i32) {
    if node.is_null() {
        return;
    }
    if levels > 0 {
        for k in 0..16 {
            pal_quant_free_tree(pq, p8(rd::<u32>(node, 0x1C + k * 4)), levels - 1);
        }
    }
    rw_memory_free(p8(rd::<u32>(pq, FREE_LIST)), node);
}

/// `RwPalQuantTerm(pq)` (0x80C520): the tree and its free list released.
#[hook(addr = 0x80C520, conv = cdecl, name = "RwPalQuantTerm", expect = "56 57 8B 7C 24 0C",
       unverified = "frontier2-c33", route = "L2", hazards = "heap-writes")]
pub fn rw_pal_quant_term(pq: GPtr<u8>) -> u32 {
    let root = p8(rd::<u32>(pq, ROOT));
    if !root.is_null() {
        let levels = depth();
        if levels > 0 {
            for k in 0..16 {
                pal_quant_free_tree(pq, p8(rd::<u32>(root, 0x1C + k * 4)), levels - 1);
            }
        }
        rw_memory_free(p8(rd::<u32>(pq, FREE_LIST)), root);
    }
    let fl = p8(rd::<u32>(pq, FREE_LIST));
    wr::<u32>(pq, ROOT, 0);
    free_list_destroy(fl)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RW_PAL_QUANT_INIT,
    HOOK_PAL_QUANT_OCTREE_INSERT,
    HOOK_PAL_QUANT_ADD_PIXEL,
    HOOK_RW_PAL_QUANT_ADD_IMAGE,
    HOOK_PAL_QUANT_MERGE_STATS,
    HOOK_PAL_QUANT_ASSIGN_INDICES,
    HOOK_PAL_QUANT_COUNT_LEAVES,
    HOOK_PAL_QUANT_FILL_PALETTE,
    HOOK_PAL_QUANT_NODE_TO_COLOUR,
    HOOK_PAL_QUANT_BOX_STATS,
    HOOK_PAL_QUANT_BUILD_BOXES,
    HOOK_PAL_QUANT_SPLIT_COST,
    HOOK_RW_PAL_QUANT_RESOLVE_PALETTE,
    HOOK_PAL_QUANT_LOOKUP,
    HOOK_RW_PAL_QUANT_MATCH_IMAGE,
    HOOK_PAL_QUANT_FREE_TREE,
    HOOK_RW_PAL_QUANT_TERM,
];
