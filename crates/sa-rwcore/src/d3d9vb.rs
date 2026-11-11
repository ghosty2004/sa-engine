//! PORT-RW-4 module `d3d9vb`: the D3D9 driver's vertex-buffer managers (static shared buffers per stride
//! with free-block lists, the dynamic buffer descriptors and the four rotating Im buffers) and
//! `RwD3D9IndexBufferCreate` (see docs/arch/rw-port.md, PORT-RW-4). Ported from the 1.0 US bytes
//! (0x7F5500..0x7F5EE7, 0x4C9970); `docs/re/specs/RW/RwD3D9VertexBuffer.md` as a reading aid.
//!
//! Layouts (freelist entries): stride node (`[0xC97B80]`, 0x10): +0 stride, +4 free-block list, +8 VB
//! records, +0xC next; free block (`[0xC97B84]`, 0x14): +0 offset, +4 size, +8 VB, +0xC next, +0x10 prev;
//! VB record (`[0xC97B88]`, 8): +0 VB, +4 next; dynamic descriptor (`[0xC97B90]`, 0x14): +0 size,
//! +4 in use, +8 VB, +0xC owner (`IDirect3DVertexBuffer9**`), +0x10 next.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, rw_memory_alloc, rw_memory_free, wr};
use crate::d3d9state::device;

const STATIC_LIST: usize = 0xC97B7C;
const FL_NODES: usize = 0xC97B80;
const FL_BLOCKS: usize = 0xC97B84;
const FL_RECORDS: usize = 0xC97B88;
const DYN_LIST: usize = 0xC97B8C;
const FL_DYN: usize = 0xC97B90;
const IM_CURRENT: usize = 0xC97B94;
const IM_USED: u32 = 0xC97B98;
const IM_CAPACITY: u32 = 0xC97BA8;
const IM_VB: u32 = 0xC97BB8;
/// `D3DCAPS9.DevCaps` (bit 0x10000 = HWTRANSFORMANDLIGHT).
const DEV_CAPS: usize = 0xC9BF1C;
/// Default static VB size in bytes (`.data`, 0x20000).
const DEFAULT_SIZE: usize = 0x8E2414;

orig! {
    /// `IDirect3DDevice9::CreateVertexBuffer` (+0x68)
    pub fn dev_create_vertex_buffer(dev: GPtr<u8>, len: u32, usage: u32, fvf: u32, pool: u32, out: GPtr<u32>, shared: u32) -> i32 = stdcall vslot 0x68 => unported;
    /// `IDirect3DVertexBuffer9::Lock` (+0x2C)
    pub fn vb_lock(vb: GPtr<u8>, offset: u32, size: u32, data: u32, flags: u32) -> i32 = stdcall vslot 0x2C => unported;
    /// `IDirect3DVertexBuffer9::Unlock` (+0x30)
    pub fn vb_unlock(vb: GPtr<u8>) -> i32 = stdcall vslot 0x30 => unported;
    /// `IUnknown::Release` (+8)
    pub fn com_release(obj: GPtr<u8>) -> u32 = stdcall vslot 0x8 => unported;
    /// `_rwFreeListCreate(entrySize, entriesPerBlock, alignment, hint)` (0x801980)
    pub fn free_list_create(size: u32, per_block: u32, align: u32, hint: u32) -> GPtr<u8> = cdecl 0x801980 => unported;
    /// `RwFreeListDestroy(fl)` (0x801B80)
    pub fn free_list_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => unported;
    /// `D3DResourceSystem::CreateIndexBuffer(numIndices, format, out)` (0x7306A0)
    pub fn resource_create_index_buffer(num: u32, fmt: u32, out: u32) -> i32 = cdecl 0x7306A0 => unported;
    /// `RwD3D9DynamicVertexBufferCreate(size, out)` (0x7F5A00)
    pub fn dyn_create_call(size: u32, out: GPtr<u32>) -> u32 = cdecl 0x7F5A00 => dynamic_vertex_buffer_create;
    /// `_rwD3D9DynamicVertexBufferManagerCreate()` (0x7F5940)
    pub fn dyn_manager_create_call() -> u32 = cdecl 0x7F5940 => dynamic_vertex_buffer_manager_create;
    /// `_rwD3D9VertexBufferManagerClose()` (0x7F5D20)
    pub fn manager_close_call() = cdecl 0x7F5D20 => vertex_buffer_manager_close;
}

#[inline]
fn gu(addr: usize) -> u32 {
    g::<u32>(addr).read()
}

#[inline]
fn su(addr: usize, v: u32) {
    g::<u32>(addr).write(v)
}

#[inline]
fn im(base: u32, i: u32) -> GPtr<u32> {
    g::<u32>(base.wrapping_add(i.wrapping_mul(4)) as usize)
}

fn a(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

fn hwtl() -> bool {
    gu(DEV_CAPS) & 0x1_0000 != 0
}

/// `RwD3D9IndexBufferCreate` (0x4C9970): `D3DResourceSystem::CreateIndexBuffer(n, INDEX16, out)`;
/// `hr >= 0`.
#[hook(addr = 0x4C9970, conv = cdecl, name = "RwD3D9IndexBufferCreate", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn index_buffer_create(num: u32, out: u32) -> u32 {
    (resource_create_index_buffer(num, 0x65, out) >= 0) as u32
}

/// `RwD3D9CreateVertexBuffer` (0x7F5500): carves `size` bytes out of a shared managed buffer of the stride
/// (the first free block that is large enough, else a new buffer of `max(size, default rounded to the
/// stride)` bytes, `CreateVertexBuffer(len, WRITEONLY, 0, MANAGED)`); returns the buffer and the byte
/// offset. A failed creation frees the block and returns 0 (the stride node stays).
#[hook(addr = 0x7F5500, conv = cdecl, name = "RwD3D9CreateVertexBuffer", expect = "53 8B 5C 24 08 55")]
pub fn create_vertex_buffer(stride: u32, size: u32, vb_out: GPtr<u32>, offset_out: GPtr<u32>) -> u32 {
    let mut node = p8(gu(STATIC_LIST));
    while !node.is_null() && rd::<u32>(node, 0) != stride {
        node = p8(rd::<u32>(node, 0xC));
    }
    if node.is_null() {
        node = rw_memory_alloc(p8(gu(FL_NODES)), 0x30411);
        wr::<u32>(node, 0, stride);
        wr::<u32>(node, 4, 0);
        wr::<u32>(node, 8, 0);
        wr::<u32>(node, 0xC, gu(STATIC_LIST));
        su(STATIC_LIST, a(node));
    }
    let mut blk = p8(rd::<u32>(node, 4));
    while !blk.is_null() && rd::<u32>(blk, 4) < size {
        blk = p8(rd::<u32>(blk, 0xC));
    }
    if blk.is_null() {
        blk = rw_memory_alloc(p8(gu(FL_BLOCKS)), 0x30411);
        wr::<u32>(blk, 0, 0);
        let rounded = (gu(DEFAULT_SIZE).wrapping_add(stride).wrapping_sub(1) / stride).wrapping_mul(stride);
        wr::<u32>(blk, 4, rounded);
        if size >= rounded {
            wr::<u32>(blk, 4, size);
        }
        let hr = dev_create_vertex_buffer(device(), rd::<u32>(blk, 4), 8, 0, 1, blk.byte_add(8).cast(), 0);
        if hr != 0 {
            rw_memory_free(p8(gu(FL_BLOCKS)), blk);
            return 0;
        }
        wr::<u32>(blk, 0x10, 0);
        wr::<u32>(blk, 0xC, rd::<u32>(node, 4));
        let head = rd::<u32>(node, 4);
        if head != 0 {
            wr::<u32>(p8(head), 0x10, a(blk));
        }
        wr::<u32>(node, 4, a(blk));
        let rec = rw_memory_alloc(p8(gu(FL_RECORDS)), 0x30411);
        wr::<u32>(rec, 0, rd::<u32>(blk, 8));
        wr::<u32>(rec, 4, rd::<u32>(node, 8));
        wr::<u32>(node, 8, a(rec));
    }
    vb_out.write(rd::<u32>(blk, 8));
    let off = rd::<u32>(blk, 0);
    offset_out.write(off);
    wr::<u32>(blk, 0, off.wrapping_add(size));
    let left = rd::<u32>(blk, 4).wrapping_sub(size);
    wr::<u32>(blk, 4, left);
    if left == 0 {
        let prev = rd::<u32>(blk, 0x10);
        if prev != 0 {
            wr::<u32>(p8(prev), 0xC, rd::<u32>(blk, 0xC));
        }
        let next = rd::<u32>(blk, 0xC);
        if next != 0 {
            wr::<u32>(p8(next), 0x10, rd::<u32>(blk, 0x10));
        }
        if rd::<u32>(node, 4) == a(blk) {
            wr::<u32>(node, 4, rd::<u32>(blk, 0xC));
        }
        rw_memory_free(p8(gu(FL_BLOCKS)), blk);
    }
    1
}

/// A free block of `node` on `vb` adjacent to `[offset, offset + size)` (either side), from the head.
fn find_adjacent(node: GPtr<u8>, vb: u32, offset: u32, size: u32) -> GPtr<u8> {
    let mut b = p8(rd::<u32>(node, 4));
    while !b.is_null() {
        if rd::<u32>(b, 8) == vb {
            let (o, s) = (rd::<u32>(b, 0), rd::<u32>(b, 4));
            if o.wrapping_add(s) == offset || size.wrapping_add(offset) == o {
                return b;
            }
        }
        b = p8(rd::<u32>(b, 0xC));
    }
    GPtr::NULL
}

/// `RwD3D9DestroyVertexBuffer` (0x7F56A0): gives `[offset, offset + size)` of `vb` back to its stride
/// node: merged into an adjacent free block (and the merged block again into further adjacent ones,
/// each absorbed block unlinked and freed), else a new free block at the list head. Unknown strides are
/// ignored.
#[hook(addr = 0x7F56A0, conv = cdecl, name = "RwD3D9DestroyVertexBuffer", expect = "51 53 55 8B 2D 7C 7B C9 00")]
pub fn destroy_vertex_buffer(stride: u32, size: u32, vb: u32, offset: u32) {
    let mut node = p8(gu(STATIC_LIST));
    while !node.is_null() && rd::<u32>(node, 0) != stride {
        node = p8(rd::<u32>(node, 0xC));
    }
    if node.is_null() {
        return;
    }
    let mut blk = find_adjacent(node, vb, offset, size);
    if blk.is_null() {
        let b = rw_memory_alloc(p8(gu(FL_BLOCKS)), 0x30411);
        wr::<u32>(b, 0, offset);
        wr::<u32>(b, 4, size);
        wr::<u32>(b, 8, vb);
        wr::<u32>(b, 0x10, 0);
        wr::<u32>(b, 0xC, rd::<u32>(node, 4));
        let head = rd::<u32>(node, 4);
        if head != 0 {
            wr::<u32>(p8(head), 0x10, a(b));
        }
        wr::<u32>(node, 4, a(b));
        return;
    }
    let (mut off, mut sz) = (offset, size);
    loop {
        let (o, s) = (rd::<u32>(blk, 0), rd::<u32>(blk, 4));
        if o.wrapping_add(s) == off {
            wr::<u32>(blk, 4, s.wrapping_add(sz));
        } else if sz.wrapping_add(off) == o {
            wr::<u32>(blk, 0, off);
            wr::<u32>(blk, 4, s.wrapping_add(sz));
        }
        off = rd::<u32>(blk, 0);
        sz = rd::<u32>(blk, 4);
        let merged = blk;
        blk = find_adjacent(node, vb, off, sz);
        if blk.is_null() {
            return;
        }
        let prev = rd::<u32>(merged, 0x10);
        if prev != 0 {
            wr::<u32>(p8(prev), 0xC, rd::<u32>(merged, 0xC));
        }
        let next = rd::<u32>(merged, 0xC);
        if next != 0 {
            wr::<u32>(p8(next), 0x10, rd::<u32>(merged, 0x10));
        }
        if rd::<u32>(node, 4) == a(merged) {
            wr::<u32>(node, 4, rd::<u32>(merged, 0xC));
        }
        rw_memory_free(p8(gu(FL_BLOCKS)), merged);
    }
}

/// `RwD3D9VertexBufferManagerChangeDefaultSize` (0x7F57F0).
#[hook(addr = 0x7F57F0, conv = cdecl, name = "RwD3D9VertexBufferManagerChangeDefaultSize", expect = "8B 44 24 04 A3 14 24 8E 00")]
pub fn vertex_buffer_manager_change_default_size(size: u32) {
    su(DEFAULT_SIZE, size);
}

/// `FUN_007f5800` (`_rwD3D9DynamicVertexBufferManagerFrame`): the four Im buffers' used counts 0, the
/// current index advanced (mod 4).
#[hook(addr = 0x7F5800, conv = cdecl, name = "FUN_007f5800", expect = "33 C0 A3 98 7B C9 00")]
pub fn dynamic_vertex_buffer_manager_frame() {
    for i in 0..4 {
        im(IM_USED, i).write(0);
    }
    let c = gu(IM_CURRENT).wrapping_add(1);
    su(IM_CURRENT, c);
    if c >= 4 {
        su(IM_CURRENT, 0);
    }
}

/// The dynamic descriptor of `vb` marked unused (owner cleared), if any.
fn dyn_forget(vb: u32) {
    let mut d = p8(gu(DYN_LIST));
    while !d.is_null() {
        if rd::<u32>(d, 8) == vb {
            wr::<u32>(d, 4, 0);
            wr::<u32>(d, 0xC, 0);
            return;
        }
        d = p8(rd::<u32>(d, 0x10));
    }
}

/// The Im-buffer reset of release / close: index 0, every buffer's used / capacity 0, its descriptor
/// forgotten and the slot cleared.
fn im_reset() {
    su(IM_CURRENT, 0);
    for i in 0..4 {
        let vb = im(IM_VB, i).read();
        im(IM_USED, i).write(0);
        im(IM_CAPACITY, i).write(0);
        if vb != 0 {
            dyn_forget(vb);
            im(IM_VB, i).write(0);
        }
    }
}

/// `_rwD3D9DynamicVertexBufferRelease` (0x7F5840): Im reset; with hardware T&L every descriptor's buffer
/// is released and its owner variable cleared (D3DPOOL_DEFAULT buffers before a device reset).
#[hook(addr = 0x7F5840, conv = cdecl, name = "_rwD3D9DynamicVertexBufferRelease", expect = "53 33 DB 56 8B 35 8C 7B C9 00")]
pub fn dynamic_vertex_buffer_release() {
    im_reset();
    if !hwtl() {
        return;
    }
    let mut d = p8(gu(DYN_LIST));
    while !d.is_null() {
        let next = rd::<u32>(d, 0x10);
        let vb = rd::<u32>(d, 8);
        if vb != 0 {
            com_release(p8(vb));
            wr::<u32>(d, 8, 0);
            let owner = rd::<u32>(d, 0xC);
            if owner != 0 {
                wr::<u32>(p8(owner), 0, 0);
            }
        }
        d = p8(next);
    }
}

/// `_rwD3D9DynamicVertexBufferRestore` (0x7F58D0): with hardware T&L re-creates every released
/// descriptor buffer (`CreateVertexBuffer(size, DYNAMIC | WRITEONLY, 0, DEFAULT)`, owner updated; a failure
/// returns 0), then (tail call) `_rwD3D9DynamicVertexBufferManagerCreate`.
#[hook(addr = 0x7F58D0, conv = cdecl, name = "_rwD3D9DynamicVertexBufferRestore", expect = "8B 0D 1C BF C9 00")]
pub fn dynamic_vertex_buffer_restore() -> u32 {
    if hwtl() {
        let mut d = p8(gu(DYN_LIST));
        while !d.is_null() {
            let next = rd::<u32>(d, 0x10);
            if rd::<u32>(d, 8) == 0 {
                let hr = dev_create_vertex_buffer(device(), rd::<u32>(d, 0), 0x208, 0, 0, d.byte_add(8).cast(), 0);
                let owner = rd::<u32>(d, 0xC);
                if owner != 0 {
                    wr::<u32>(p8(owner), 0, rd::<u32>(d, 8));
                }
                if hr < 0 {
                    return 0;
                }
            }
            d = p8(next);
        }
    }
    dyn_manager_create_call()
}

/// `FUN_007f5940` (`_rwD3D9DynamicVertexBufferManagerCreate`): index 0; with hardware T&L four Im
/// buffers of 0x40000 bytes, else one (the other slots cleared after it succeeds). 0 on a failed
/// creation.
#[hook(addr = 0x7F5940, conv = cdecl, name = "FUN_007f5940", expect = "A1 1C BF C9 00 53")]
pub fn dynamic_vertex_buffer_manager_create() -> u32 {
    su(IM_CURRENT, 0);
    if hwtl() {
        for i in 0..4 {
            im(IM_USED, i).write(0);
            im(IM_CAPACITY, i).write(0x40000);
            if dyn_create_call(0x40000, im(IM_VB, i)) == 0 {
                return 0;
            }
        }
        return 1;
    }
    im(IM_USED, 0).write(0);
    im(IM_CAPACITY, 0).write(0x40000);
    if dyn_create_call(0x40000, im(IM_VB, 0)) == 0 {
        return 0;
    }
    for i in 1..4 {
        im(IM_USED, i).write(0);
        im(IM_VB, i).write(0);
        im(IM_CAPACITY, i).write(0);
    }
    1
}

/// `RwD3D9DynamicVertexBufferCreate` (0x7F5A00): reuses an unused descriptor of exactly `size` bytes;
/// else releases the buffer of the largest unused one (first on ties) and re-creates it, or allocates a
/// new descriptor at the list head. `CreateVertexBuffer(size, DYNAMIC | WRITEONLY, 0, DEFAULT with
/// hardware T&L / SYSTEMMEM)`; any non-zero HRESULT returns 0.
#[hook(addr = 0x7F5A00, conv = cdecl, name = "RwD3D9DynamicVertexBufferCreate", expect = "A1 8C 7B C9 00 53")]
pub fn dynamic_vertex_buffer_create(size: u32, out: GPtr<u32>) -> u32 {
    let mut best = GPtr::<u8>::NULL;
    let mut d = p8(gu(DYN_LIST));
    while !d.is_null() {
        let next = rd::<u32>(d, 0x10);
        if rd::<u32>(d, 4) == 0 {
            let s = rd::<u32>(d, 0);
            if s == size {
                wr::<u32>(d, 4, 1);
                out.write(rd::<u32>(d, 8));
                wr::<u32>(d, 0xC, out.addr() as u32);
                return 1;
            }
            if best.is_null() || rd::<u32>(best, 0) < s {
                best = d;
            }
        }
        d = p8(next);
    }
    let desc = if !best.is_null() {
        com_release(p8(rd::<u32>(best, 8)));
        wr::<u32>(best, 8, 0);
        best
    } else {
        let n = rw_memory_alloc(p8(gu(FL_DYN)), 0x40411);
        wr::<u32>(n, 0x10, gu(DYN_LIST));
        su(DYN_LIST, a(n));
        n
    };
    let pool = (!gu(DEV_CAPS) >> 15) & 2;
    if dev_create_vertex_buffer(device(), size, 0x208, 0, pool, out, 0) != 0 {
        return 0;
    }
    wr::<u32>(desc, 0, size);
    wr::<u32>(desc, 4, 1);
    wr::<u32>(desc, 0xC, out.addr() as u32);
    wr::<u32>(desc, 8, out.read());
    1
}

/// `RwD3D9DynamicVertexBufferDestroy` (0x7F5AE0): the descriptor of `vb` becomes unused (buffer kept).
#[hook(addr = 0x7F5AE0, conv = cdecl, name = "RwD3D9DynamicVertexBufferDestroy", expect = "A1 8C 7B C9 00 33 D2")]
pub fn dynamic_vertex_buffer_destroy(vb: u32) {
    dyn_forget(vb);
}

/// `RwD3D9DynamicVertexBufferLock` (0x7F5B10): `bytes = vertexSize * numVertices` from the current Im
/// buffer at the used count rounded up to the vertex size; when it does not fit (hardware T&L) another
/// buffer with room becomes current, else the next one is started from 0. A buffer too small for
/// `bytes` is re-created at that size. Offset 0 locks with DISCARD, else NOOVERWRITE (both NOSYSLOCK);
/// returns the buffer, the data pointer, the base vertex index and `hr >= 0`.
#[hook(addr = 0x7F5B10, conv = cdecl, name = "RwD3D9DynamicVertexBufferLock", expect = "8B 4C 24 04 A1 1C BF C9 00")]
pub fn dynamic_vertex_buffer_lock(vsize: u32, num: u32, vb_out: GPtr<u32>, data: u32, base_index: GPtr<u32>) -> u32 {
    let bytes = vsize.wrapping_mul(num);
    let mut cur = gu(IM_CURRENT);
    let fits = |i: u32| {
        let off = (im(IM_USED, i).read().wrapping_add(vsize).wrapping_sub(1) / vsize).wrapping_mul(vsize);
        let end = off.wrapping_add(bytes);
        (off, end, end <= im(IM_CAPACITY, i).read())
    };
    let (mut off, mut end) = (0u32, bytes);
    if hwtl() {
        let (o, e, ok) = fits(cur);
        if ok {
            (off, end) = (o, e);
        } else {
            let mut found = None;
            for i in 0..4u32 {
                if i == cur {
                    continue;
                }
                let (o, e, ok) = fits(i);
                if ok {
                    found = Some((i, o, e));
                    break;
                }
            }
            if let Some((i, o, e)) = found {
                cur = i;
                su(IM_CURRENT, cur);
                (off, end) = (o, e);
            } else {
                cur = cur.wrapping_add(1);
                su(IM_CURRENT, cur);
                if cur >= 4 {
                    cur = 0;
                    su(IM_CURRENT, 0);
                }
            }
        }
    }
    if bytes > im(IM_CAPACITY, cur).read() {
        dyn_forget(im(IM_VB, cur).read());
        cur = gu(IM_CURRENT);
        im(IM_CAPACITY, cur).write(bytes);
        dyn_create_call(bytes, im(IM_VB, cur));
        cur = gu(IM_CURRENT);
    }
    let vb = p8(im(IM_VB, cur).read());
    let hr = if off == 0 {
        let hr = vb_lock(vb, 0, bytes, data, 0x2800);
        im(IM_USED, gu(IM_CURRENT)).write(bytes);
        base_index.write(0);
        hr
    } else {
        let hr = vb_lock(vb, off, bytes, data, 0x1800);
        im(IM_USED, gu(IM_CURRENT)).write(end);
        base_index.write(off / vsize);
        hr
    };
    vb_out.write(im(IM_VB, gu(IM_CURRENT)).read());
    (hr >= 0) as u32
}

/// `RwD3D9DynamicVertexBufferUnlock` (0x7F5C90): `Unlock`; `hr >= 0`.
#[hook(addr = 0x7F5C90, conv = cdecl, name = "RwD3D9DynamicVertexBufferUnlock", expect = "8B 44 24 04 50 8B 08")]
pub fn dynamic_vertex_buffer_unlock(vb: GPtr<u8>) -> u32 {
    (vb_unlock(vb) >= 0) as u32
}

/// `_rwD3D9VertexBufferManagerOpen` (0x7F5CB0): close, the four freelists (nodes 0x10 x 16, blocks
/// 0x14 x 100, records 8 x 100, dynamic descriptors 0x14 x 42; alignment 4, hint 0x40411), the dynamic
/// manager (result ignored); returns 1.
#[hook(addr = 0x7F5CB0, conv = cdecl, name = "_rwD3D9VertexBufferManagerOpen", expect = "E8 6B 00 00 00")]
pub fn vertex_buffer_manager_open() -> u32 {
    manager_close_call();
    su(FL_NODES, a(free_list_create(0x10, 0x10, 4, 0x4_0411)));
    su(FL_BLOCKS, a(free_list_create(0x14, 0x64, 4, 0x4_0411)));
    su(FL_RECORDS, a(free_list_create(8, 0x64, 4, 0x4_0411)));
    su(FL_DYN, a(free_list_create(0x14, 0x2A, 4, 0x4_0411)));
    dyn_manager_create_call();
    1
}

/// `_rwD3D9VertexBufferManagerClose` (0x7F5D20): Im reset; every dynamic descriptor's buffer released
/// (owner cleared) and the descriptor freed; every stride node's free blocks freed, its buffers released
/// and records freed, the node freed; the four freelists destroyed.
#[hook(addr = 0x7F5D20, conv = cdecl, name = "_rwD3D9VertexBufferManagerClose", expect = "8B 0D 8C 7B C9 00 57")]
pub fn vertex_buffer_manager_close() {
    im_reset();
    while gu(DYN_LIST) != 0 {
        let d = p8(gu(DYN_LIST));
        let next = rd::<u32>(d, 0x10);
        let vb = rd::<u32>(d, 8);
        if vb != 0 {
            com_release(p8(vb));
            wr::<u32>(p8(gu(DYN_LIST)), 8, 0);
            let owner = rd::<u32>(p8(gu(DYN_LIST)), 0xC);
            if owner != 0 {
                wr::<u32>(p8(owner), 0, 0);
            }
        }
        rw_memory_free(p8(gu(FL_DYN)), p8(gu(DYN_LIST)));
        su(DYN_LIST, next);
    }
    if gu(FL_DYN) != 0 {
        free_list_destroy(p8(gu(FL_DYN)));
        su(FL_DYN, 0);
    }
    while gu(STATIC_LIST) != 0 {
        let node = p8(gu(STATIC_LIST));
        let next = rd::<u32>(node, 0xC);
        while rd::<u32>(p8(gu(STATIC_LIST)), 4) != 0 {
            let b = p8(rd::<u32>(p8(gu(STATIC_LIST)), 4));
            let nb = rd::<u32>(b, 0xC);
            rw_memory_free(p8(gu(FL_BLOCKS)), b);
            wr::<u32>(p8(gu(STATIC_LIST)), 4, nb);
        }
        while rd::<u32>(p8(gu(STATIC_LIST)), 8) != 0 {
            let r = p8(rd::<u32>(p8(gu(STATIC_LIST)), 8));
            let nr = rd::<u32>(r, 4);
            let vb = rd::<u32>(r, 0);
            if vb != 0 {
                com_release(p8(vb));
            }
            rw_memory_free(p8(gu(FL_RECORDS)), p8(rd::<u32>(p8(gu(STATIC_LIST)), 8)));
            wr::<u32>(p8(gu(STATIC_LIST)), 8, nr);
        }
        rw_memory_free(p8(gu(FL_NODES)), p8(gu(STATIC_LIST)));
        su(STATIC_LIST, next);
    }
    for fl in [FL_RECORDS, FL_BLOCKS, FL_NODES] {
        if gu(fl) != 0 {
            free_list_destroy(p8(gu(fl)));
            su(fl, 0);
        }
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_INDEX_BUFFER_CREATE,
    HOOK_CREATE_VERTEX_BUFFER,
    HOOK_DESTROY_VERTEX_BUFFER,
    HOOK_VERTEX_BUFFER_MANAGER_CHANGE_DEFAULT_SIZE,
    HOOK_DYNAMIC_VERTEX_BUFFER_MANAGER_FRAME,
    HOOK_DYNAMIC_VERTEX_BUFFER_RELEASE,
    HOOK_DYNAMIC_VERTEX_BUFFER_RESTORE,
    HOOK_DYNAMIC_VERTEX_BUFFER_MANAGER_CREATE,
    HOOK_DYNAMIC_VERTEX_BUFFER_CREATE,
    HOOK_DYNAMIC_VERTEX_BUFFER_DESTROY,
    HOOK_DYNAMIC_VERTEX_BUFFER_LOCK,
    HOOK_DYNAMIC_VERTEX_BUFFER_UNLOCK,
    HOOK_VERTEX_BUFFER_MANAGER_OPEN,
    HOOK_VERTEX_BUFFER_MANAGER_CLOSE,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
