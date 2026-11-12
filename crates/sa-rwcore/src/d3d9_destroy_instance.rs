//! `_rxD3D9DefaultDestroyInstanceCallback` (0x4C9990, batch `boot-rw-1`): the destroy notification of a D3D9
//! instanced resource entry (registered by the all-in-one pipeline nodes, 0x756960). Ported from the 1.0 US bytes.
//! Unverified (docs/verify/HOWTO.md).
//!
//! Resource entry: owner +0xC (an `RwObject`: atomic type 1, geometry type 8); `RxD3D9ResEntryHeader` from +0x18:
//! index buffer +0x20, two 0x10-byte vertex streams from +0x28 (vertex buffer +0, offset +4, stride +8, geometry
//! flags word +0xC, managed byte +0xE, dynamic-lock byte +0xF), vertex declaration +0x4C, the vertex count +0x54.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{rd, wr};

orig! {
    /// `_rpD3D9RemoveDynamicGeometry(geometry)` (0x74E9E0)
    fn remove_dynamic_geometry(geo: u32) = cdecl 0x74E9E0 => crate::d3d9misc::remove_dynamic_geometry;
    /// `D3DResourceSystem::DestroyIndexBuffer(ib)` (0x730D30, sa-render)
    fn destroy_index_buffer(ib: u32) = cdecl 0x730D30 => unported;
    /// `RwD3D9DeleteVertexDeclaration(decl)` (0x7FAC10)
    fn delete_vertex_declaration(decl: u32) = cdecl 0x7FAC10 => crate::d3d9dev::delete_vertex_declaration;
    /// `RwD3D9DestroyVertexBuffer(stride, size, vb, offset)` (0x7F56A0)
    fn destroy_vertex_buffer(stride: u32, size: u32, vb: u32, offset: u32) = cdecl 0x7F56A0 => crate::d3d9vb::destroy_vertex_buffer;
    /// `RwD3D9DynamicVertexBufferDestroy(vb)` (0x7F5AE0)
    fn dynamic_vertex_buffer_destroy(vb: u32) = cdecl 0x7F5AE0 => crate::d3d9vb::dynamic_vertex_buffer_destroy;
}

/// `_rxD3D9DefaultDestroyInstanceCallback(RwResEntry* entry)` (0x4C9990, cdecl; catalogue `FUN_004c9990`, rw-names).
/// 1. An owner whose type byte is not 0xFF, when either stream's managed byte is 0 (first stream first), loses its
///    dynamic geometry: `_rpD3D9RemoveDynamicGeometry(owner)` for a geometry (type 8), of `owner->geometry` (+0x18)
///    for an atomic (type 1), nothing for other types.
/// 2. The index buffer is destroyed and cleared, then the vertex declaration deleted and cleared.
/// 3. Per stream with a vertex buffer: managed → `RwD3D9DestroyVertexBuffer(stride, count * stride, vb, offset)`;
///    otherwise, when offset and dynamic lock are 0, `RwD3D9DynamicVertexBufferDestroy(vb)`; then the buffer pointer
///    is cleared. Every stream then gets offset, stride, flags word, managed and dynamic lock zeroed.
#[hook(addr = 0x4C9990, conv = cdecl, name = "_rxD3D9DefaultDestroyInstanceCallback", expect = "53 55 56 57 8B 7C 24 14",
       unverified = "boot-rw-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "frees D3D9 vertex / index buffers and declarations of a resource-arena entry; the entry's header is zeroed")]
pub fn default_destroy_instance_callback(entry: GPtr<u8>) {
    let owner = rd::<GPtr<u8>>(entry, 0xC);
    if !owner.is_null() {
        let ty = rd::<u8>(owner, 0);
        if ty != 0xFF && (0..2isize).any(|i| rd::<u8>(entry, 0x36 + 0x10 * i) == 0) {
            if ty == 8 {
                remove_dynamic_geometry(owner.addr() as u32);
            } else if ty == 1 {
                remove_dynamic_geometry(rd::<u32>(owner, 0x18));
            }
        }
    }
    let ib = rd::<u32>(entry, 0x20);
    if ib != 0 {
        destroy_index_buffer(ib);
        wr::<u32>(entry, 0x20, 0);
    }
    let decl = rd::<u32>(entry, 0x4C);
    if decl != 0 {
        delete_vertex_declaration(decl);
        wr::<u32>(entry, 0x4C, 0);
    }
    for i in 0..2isize {
        let s = entry.byte_add(0x28 + 0x10 * i);
        let vb = rd::<u32>(s, 0);
        if vb != 0 {
            if rd::<u8>(s, 0xE) != 0 {
                let stride = rd::<u32>(s, 8);
                let size = rd::<u32>(entry, 0x54).wrapping_mul(stride);
                destroy_vertex_buffer(stride, size, vb, rd::<u32>(s, 4));
            } else if rd::<u32>(s, 4) == 0 && rd::<u8>(s, 0xF) == 0 {
                dynamic_vertex_buffer_destroy(vb);
            }
            wr::<u32>(s, 0, 0);
        }
        wr::<u32>(s, 4, 0);
        wr::<u32>(s, 8, 0);
        wr::<u16>(s, 0xC, 0);
        wr::<u8>(s, 0xE, 0);
        wr::<u8>(s, 0xF, 0);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_DEFAULT_DESTROY_INSTANCE_CALLBACK];
