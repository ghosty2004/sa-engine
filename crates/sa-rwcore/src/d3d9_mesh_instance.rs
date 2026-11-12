//! `_rxD3D9Instance` (0x756960) with its index helpers (batch `std9-rw-2`): the D3D9 all-in-one nodes' mesh
//! instancing (resource entry, index buffer, per-mesh `RxD3D9InstanceData`, the instance callback). Ported from the
//! 1.0 US bytes (RW has no gta-reversed source). Unverified (docs/verify/HOWTO.md).
//!
//! Resource entry: 0x18-byte `RwResEntry` header, then `RxD3D9ResEntryHeader` (0x40): +0 serial number, +4 mesh
//! count, +8 index buffer, +0xC primitive type, +0x10 two vertex streams (0x10 each), +0x34 vertex declaration, +0x38
//! total index count; then one 0x24-byte `RxD3D9InstanceData` per mesh: +0 index count, +4 min vertex, +8 material,
//! +0xC vertex alpha, +0x10 vertex shader, +0x14 base index, +0x18 vertex count, +0x1C start index, +0x20 primitive
//! count. `RpMesh` (0xC): +0 indices, +4 index count, +8 material; they follow the 0x10-byte `RpMeshHeader` (+0
//! flags, +4 u16 mesh count, +6 u16 serial number).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, rw_globals, wr};

/// MESHMODULE offset global (`G + [0xC9B8C0]`, +8: the RW primitive type -> D3D type index bytes)
const MESH_MODULE: usize = 0xC9B8C0;
/// RW primitive type index -> `D3DPRIMITIVETYPE` (dwords at 0x874FEC)
const PRIM_TYPE_TABLE: usize = 0x874FEC;
/// The resource entry's destroy notification (`_rxD3D9ResEntryDestroyNotify`-like, 0x4C9990)
const DESTROY_NOTIFY: u32 = 0x4C9990;
/// The triangle comparator handed to `qsort`
const TRIANGLE_COMPARE: u32 = 0x7567A0;

orig! {
    /// `G->memoryFuncs.rwmalloc(size, hint)` (+0x134)
    fn rw_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl table 0xC97B24 0x134 => unported;
    /// `G->memoryFuncs.rwfree(p)` (+0x138)
    fn rw_free(p: GPtr<u8>) = cdecl table 0xC97B24 0x138 => unported;
    /// `RwResourcesAllocateResEntry(owner, ownerRef, size, destroyNotify)` (0x807ED0)
    fn allocate_res_entry(owner: u32, owner_ref: GPtr<u32>, size: u32, notify: u32) -> GPtr<u8> = cdecl 0x807ED0 => crate::memory::rw_resources_allocate_res_entry;
    /// `RwResourcesFreeResEntry(entry)` (0x807DE0)
    fn free_res_entry(e: GPtr<u8>) -> u32 = cdecl 0x807DE0 => crate::memory::rw_resources_free_res_entry;
    /// `RwD3D9IndexBufferCreate(numIndices, IDirect3DIndexBuffer9** out)` (0x4C9970)
    fn index_buffer_create(num: u32, out: u32) -> u32 = cdecl 0x4C9970 => crate::d3d9vb::index_buffer_create;
    /// `IDirect3DIndexBuffer9::Lock` / `Unlock`
    fn ib_lock(ib: GPtr<u8>, offset: u32, size: u32, data: GPtr<u32>, flags: u32) -> i32 = stdcall vslot 0x2C => unported;
    fn ib_unlock(ib: GPtr<u8>) -> i32 = stdcall vslot 0x30 => unported;
    /// The strip-to-list rebuild (0x756830)
    fn strip_to_list_call(dst: GPtr<u8>, src: GPtr<u8>, n: u32, min_vert: u32) -> u32 = cdecl 0x756830 => d3d9_strip_to_list;
    /// CRT `qsort(base, num, width, comp)` (0x8247E0)
    fn qsort(base: GPtr<u8>, num: u32, width: u32, comp: u32) = cdecl 0x8247E0 => unported;
    /// The instance callback (`RwBool (*)(void* object, RxD3D9ResEntryHeader*, RwBool reinstance)`)
    fn instance_cb(f: u32, object: GPtr<u8>, header: GPtr<u8>, reinstance: u32) -> u32 = cdecl dyn => unported;
}

/// The three u16s of a triangle in ascending order (the original's three compare-and-swaps).
fn sorted3(t: GPtr<u8>) -> (u32, u32, u32) {
    let (mut a, mut b, mut c) = (u32::from(rd::<u16>(t, 0)), u32::from(rd::<u16>(t, 2)), u32::from(rd::<u16>(t, 4)));
    if a > b {
        core::mem::swap(&mut a, &mut b);
    }
    if b > c {
        core::mem::swap(&mut b, &mut c);
        if a > b {
            core::mem::swap(&mut a, &mut b);
        }
    }
    (a, b, c)
}

/// The `qsort` comparator of triangle lists (0x7567A0, cdecl): both triangles' indices sorted; the first differing
/// of (min, mid, max) decides (`a - b`, 32-bit).
#[hook(addr = 0x7567A0, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 4C 24 04 33 C0 33 D2 55",
       unverified = "std9-rw-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn d3d9_triangle_compare(a: GPtr<u8>, b: GPtr<u8>) -> i32 {
    let (a0, a1, a2) = sorted3(a);
    let (b0, b1, b2) = sorted3(b);
    let d = if a0 != b0 {
        a0.wrapping_sub(b0)
    } else if a1 != b1 {
        a1.wrapping_sub(b1)
    } else {
        a2.wrapping_sub(b2)
    };
    d as i32
}

/// The strip-to-list rebuild (0x756830, cdecl): for each of the `n - 2` strip triangles `(s[i], s[i+1], s[i+2])` that
/// is not degenerate, three indices minus `minVert` (u16 arithmetic; reversed on odd `i`); returns the number written.
#[hook(addr = 0x756830, conv = cdecl, name = "FUN_00756830", expect = "51 8B 44 24 10 53 56 57",
       unverified = "std9-rw-2", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "n < 2 wraps the unsigned triangle count as the original does")]
pub fn d3d9_strip_to_list(dst: GPtr<u8>, src: GPtr<u8>, n: u32, min_vert: u32) -> u32 {
    let tris = n.wrapping_sub(2);
    let mv = min_vert as u16;
    let mut out = 0u32;
    let mut d = dst;
    let mut i = 0u32;
    while i < tris {
        let s = src.byte_add(2 * i as isize);
        let (a, b, c) = (rd::<u16>(s, 0), rd::<u16>(s, 2), rd::<u16>(s, 4));
        if a != b && a != c && b != c {
            let (a, b, c) = (a.wrapping_sub(mv), b.wrapping_sub(mv), c.wrapping_sub(mv));
            if i & 1 != 0 {
                wr::<u16>(d, 4, a);
                wr::<u16>(d, 2, b);
                wr::<u16>(d, 0, c);
            } else {
                wr::<u16>(d, 0, a);
                wr::<u16>(d, 2, b);
                wr::<u16>(d, 4, c);
            }
            d = d.byte_add(6);
            out = out.wrapping_add(3);
        }
        i += 1;
    }
    out
}

/// The RW primitive type's D3D table index byte (`G + [0xC9B8C0] + (flags & 0xFF) + 8`).
fn prim_index(flags: u32) -> u8 {
    let off = g::<u32>(MESH_MODULE).read();
    rd::<u8>(rw_globals(), off.wrapping_add(flags & 0xFF).wrapping_add(8) as isize)
}

/// `_rxD3D9Instance(object, owner, type, RwResEntry** entry, RpMeshHeader* meshes, instanceCallback,
/// allocateNative)` (0x756960, cdecl). The entry (`0x40 + 0x24 * meshCount` bytes after the 0x18-byte header) comes
/// from `rwmalloc(+0x18, 0x3050D)` (native: owner fields cleared, notify 0x4C9990) or `RwResourcesAllocateResEntry`,
/// and is zeroed. Indexed meshes: the total index count; a triangle list with more indices than three per triangle
/// of the object (`type` 1: the atomic's geometry +0x10; 0xFF: the sector's u16 +0x84; else 0x55555555) is rebuilt
/// from strips (count capped to 3 * triangles, primitive type 4). The index buffer is created and locked. Per mesh:
/// min vertex and vertex count (from the index range, or consecutive for unindexed meshes), the material, the start
/// index; the indices copied (rebased by the min vertex, or rebuilt) and, for triangle lists, sorted (`qsort`, 6-byte
/// elements); the primitive count by type (2: n/2, 3: n-1, 4: n/3, 5/6: n-2, else 0). The buffer is unlocked; a
/// failing instance callback frees the entry (NULL). Returns the entry.
#[hook(addr = 0x756960, conv = cdecl, name = "(missed by ghidra: cat7-call)", expect = "83 EC 1C 53 55 8B 6C 24 38",
       unverified = "std9-rw-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
#[allow(clippy::too_many_arguments)]
pub fn d3d9_instance(
    object: GPtr<u8>,
    owner: u32,
    ty: u32,
    entry_ref: GPtr<u32>,
    meshes: GPtr<u8>,
    callback: u32,
    allocate_native: u32,
) -> GPtr<u8> {
    let num_meshes = u32::from(rd::<u16>(meshes, 4));
    let size = num_meshes * 0x24 + 0x40;
    let entry = if allocate_native != 0 {
        let e = rw_malloc(size + 0x18, 0x3050D);
        entry_ref.write(e.addr() as u32);
        wr::<u32>(e, 0, 0);
        wr::<u32>(e, 4, 0);
        wr::<u32>(e, 8, size);
        wr::<u32>(e, 0xC, 0);
        wr::<u32>(e, 0x10, 0);
        wr::<u32>(e, 0x14, DESTROY_NOTIFY);
        e
    } else {
        allocate_res_entry(owner, entry_ref, size, DESTROY_NOTIFY)
    };
    let h = entry.byte_add(0x18);
    for i in 0..size as isize {
        h.byte_add(i).write(0u8);
    }
    wr::<u32>(h, 0, u32::from(rd::<u16>(meshes, 6)));
    wr::<u32>(h, 4, num_meshes);
    let mut locked = 0u32;
    wr::<u32>(h, 8, 0);
    wr::<u32>(h, 0x38, 0);
    let mut rebuild = false;
    let flags = rd::<u32>(meshes, 0);
    let mut prim_set = false;
    if flags & 0x100 == 0 {
        for m in 0..num_meshes as isize {
            let n = rd::<u32>(meshes, 0x14 + 0xC * m);
            wr::<u32>(h, 0x38, rd::<u32>(h, 0x38).wrapping_add(n));
        }
        let total = rd::<u32>(h, 0x38);
        if total != 0 {
            if prim_index(rd::<u32>(meshes, 0)) == 4 {
                let tris = match ty as u8 {
                    0xFF => u32::from(rd::<u16>(object, 0x84)),
                    1 => rd::<u32>(rd::<GPtr<u8>>(object, 0x18), 0x10),
                    _ => 0x5555_5555,
                };
                let cap = tris.wrapping_mul(3);
                if total > cap {
                    rebuild = true;
                    wr::<u32>(h, 0x38, cap);
                }
            }
            if index_buffer_create(rd::<u32>(h, 0x38), h.addr() as u32 + 8) != 0 {
                ib_lock(rd::<GPtr<u8>>(h, 8), 0, 0, GPtr::from_raw(&raw mut locked), 0);
            }
            if rebuild {
                wr::<u32>(h, 0xC, 4);
                prim_set = true;
            }
        }
    }
    if !prim_set {
        let i = prim_index(rd::<u32>(meshes, 0));
        wr::<u32>(h, 0xC, g::<u32>(PRIM_TYPE_TABLE + 4 * usize::from(i)).read());
    }
    for k in 0..2isize {
        let s = h.byte_add(0x10 + 0x10 * k);
        wr::<u32>(s, 0, 0);
        wr::<u32>(s, 4, 0);
        wr::<u32>(s, 8, 0);
        wr::<u16>(s, 0xC, 0);
        wr::<u8>(s, 0xE, 0);
        wr::<u8>(s, 0xF, 0);
    }
    let mut vert_base = 0u32;
    let mut index_off = 0u32;
    wr::<u32>(h, 0x34, 0);
    for m in 0..num_meshes as isize {
        let mesh = meshes.byte_add(0x10 + 0xC * m);
        let inst = h.byte_add(0x40 + 0x24 * m);
        let mut n = rd::<u32>(mesh, 4);
        if rd::<u32>(meshes, 0) & 0x100 != 0 {
            wr::<u32>(inst, 0x18, n);
            wr::<u32>(inst, 4, vert_base);
            vert_base = vert_base.wrapping_add(n);
        } else if n != 0 {
            let idx = rd::<GPtr<u8>>(mesh, 0);
            let (mut lo, mut hi) = (u32::MAX, 0u32);
            for i in 0..n as isize {
                let v = u32::from(rd::<u16>(idx, 2 * i));
                if v < lo {
                    lo = v;
                }
                if v > hi {
                    hi = v;
                }
            }
            wr::<u32>(inst, 0x18, hi.wrapping_sub(lo).wrapping_add(1));
            wr::<u32>(inst, 4, lo);
        } else {
            wr::<u32>(inst, 0x18, 0);
            wr::<u32>(inst, 4, 0);
        }
        wr::<u32>(inst, 0x10, 0);
        wr::<u32>(inst, 8, rd::<u32>(mesh, 8));
        wr::<u32>(inst, 0xC, 0);
        if locked != 0 {
            wr::<u32>(inst, 0x1C, index_off);
            let dst = p8(locked);
            let src = rd::<GPtr<u8>>(mesh, 0);
            let mv = rd::<u32>(inst, 4);
            if rebuild {
                n = strip_to_list_call(dst, src, n, mv);
            } else if mv != 0 {
                for i in 0..n as isize {
                    wr::<u16>(dst, 2 * i, rd::<u16>(src, 2 * i).wrapping_sub(mv as u16));
                }
            } else {
                for i in 0..(2 * n) as isize {
                    dst.byte_add(i).write(src.byte_add(i).read());
                }
            }
            wr::<u32>(inst, 0, n);
            if rd::<u32>(entry, 0x24) == 4 {
                qsort(dst, n / 3, 6, TRIANGLE_COMPARE);
            }
            locked = locked.wrapping_add(2 * n);
            index_off = index_off.wrapping_add(n);
        } else {
            wr::<u32>(inst, 0x1C, 0);
            wr::<u32>(inst, 0, 0);
        }
        let prims = match rd::<u32>(entry, 0x24) {
            2 => n >> 1,
            3 => n.wrapping_sub(1),
            4 => n / 3,
            5 | 6 => n.wrapping_sub(2),
            _ => 0,
        };
        wr::<u32>(inst, 0x20, prims);
        wr::<u32>(inst, 0x14, 0);
    }
    if locked != 0 {
        ib_unlock(rd::<GPtr<u8>>(h, 8));
    }
    if callback != 0 && instance_cb(callback, object, h, 0) == 0 {
        if allocate_native != 0 {
            rw_free(entry);
        } else {
            free_res_entry(entry);
        }
        return GPtr::NULL;
    }
    entry
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_D3D9_TRIANGLE_COMPARE, HOOK_D3D9_STRIP_TO_LIST, HOOK_D3D9_INSTANCE];
