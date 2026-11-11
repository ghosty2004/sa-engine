//! PORT-RW-5 module `d3d9native`: the D3D9 native-data plugin (pre-instanced geometry / world-sector
//! resource entries in a stream chunk): geometry write / read / size, the world-sector size and the
//! plugin's stream-read callbacks (see docs/arch/rw-port.md, PORT-RW-5). Ported from the 1.0 US bytes.
//!
//! Resource entry header (entry +0x18, the part written to the stream): +4 numMeshes, +8 index
//! buffer, +0xC primitive type, +0x10 two vertex streams of 0x10 (+0 vertex buffer, +4 offset, +8
//! stride, +0xE managed byte), +0x30 useOffsets, +0x34 vertex declaration, +0x38 totalNumIndex,
//! +0x3C totalNumVertex, meshes of 0x24 from +0x40 (+4 minVertex, +8 material, +0x14 baseIndex).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, rw_error_set, rw_malloc, wr, RwError};

/// `"D3D9"` platform id (rwID_PCD3D9 = 9) written by the stream writer.
const PLATFORM_ID: u32 = 0x8D7384;
/// D3D9 caps word whose bit 0 enables `useOffsets` for multi-stream entries.
const CAPS_OFFSETS: usize = 0xC9BFD4;
/// The entry destroy notify (`_rpD3D9...ResEntryDestroy`, 0x4C9990).
const ENTRY_DESTROY: u32 = 0x4C_9990;

orig! {
    /// `IDirect3DVertexDeclaration9::GetDeclaration(elements, count)` (+0x10)
    fn decl_get_declaration(decl: GPtr<u8>, elements: GPtr<u8>, count: GPtr<u32>) -> i32 = stdcall vslot 0x10 => unported;
    /// `IDirect3DIndexBuffer9 / IDirect3DVertexBuffer9::Lock(offset, size, data, flags)` (+0x2C)
    fn buffer_lock(b: GPtr<u8>, offset: u32, size: u32, data: GPtr<u32>, flags: u32) -> i32 = stdcall vslot 0x2C => unported;
    /// `IDirect3DIndexBuffer9 / IDirect3DVertexBuffer9::Unlock()` (+0x30)
    fn buffer_unlock(b: GPtr<u8>) -> i32 = stdcall vslot 0x30 => unported;

    /// `RwStreamFindChunk(stream, type, lengthOut, versionOut)` (0x7ED2D0)
    fn stream_find_chunk(s: GPtr<u8>, ty: u32, len: GPtr<u32>, ver: GPtr<u32>) -> u32 = cdecl 0x7ED2D0 => crate::stream::rw_stream_find_chunk;
    /// `RwStreamRead(stream, buffer, length)` (0x7EC9D0)
    fn stream_read(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => crate::stream::rw_stream_read;
    /// `RwStreamWrite(stream, buffer, length)` (0x7ECB30)
    fn stream_write(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> GPtr<u8> = cdecl 0x7ECB30 => crate::stream::rw_stream_write;
    /// `_rwStreamWriteVersionedChunkHeader(stream, type, size, version, build)` (0x7ED270)
    fn stream_write_chunk_header(s: GPtr<u8>, ty: u32, size: u32, version: u32, build: u32) -> GPtr<u8> = cdecl 0x7ED270 => unported;
    /// `_rpMaterialListFindMaterialIndex(list, material)` (0x74E420)
    fn material_list_find_index(l: GPtr<u8>, m: GPtr<u8>) -> i32 = cdecl 0x74E420 => crate::material::material_list_find_material_index;
    /// `_rpMaterialListGetMaterial(list, index)` (0x74E2B0)
    fn material_list_get(l: GPtr<u8>, i: i32) -> GPtr<u8> = cdecl 0x74E2B0 => crate::material::material_list_get_material;
    /// `RwD3D9CreateVertexDeclaration(elements, out)` (0x7FAA30)
    fn create_vertex_declaration(elements: GPtr<u8>, out: GPtr<u32>) -> u32 = cdecl 0x7FAA30 => crate::d3d9dev::create_vertex_declaration;
    /// `RwD3D9IndexBufferCreate(numIndices, out)` (0x4C9970)
    fn index_buffer_create(num: u32, out: u32) -> u32 = cdecl 0x4C9970 => crate::d3d9vb::index_buffer_create;
    /// `RwD3D9DynamicVertexBufferCreate(size, out)` (0x7F5A00)
    fn dynamic_vertex_buffer_create(size: u32, out: GPtr<u32>) -> u32 = cdecl 0x7F5A00 => crate::d3d9vb::dynamic_vertex_buffer_create;
    /// `RwD3D9CreateVertexBuffer(stride, size, vbOut, offsetOut)` (0x7F5500)
    fn create_vertex_buffer(stride: u32, size: u32, vb: GPtr<u32>, offset: GPtr<u32>) -> u32 = cdecl 0x7F5500 => crate::d3d9vb::create_vertex_buffer;
    /// The resource entry destroy notify `(entry)` (0x4C9990)
    fn entry_destroy(e: GPtr<u8>) = cdecl 0x4C9990 => unported;
    /// `RpWorldSectorGetWorld(sector)` (0x74F4E0)
    fn world_sector_get_world(s: GPtr<u8>) -> GPtr<u8> = cdecl 0x74F4E0 => unported;
    /// `_rwerror(code)` (0x8088D0)
    fn rw_error(code: u32) -> u32 = cdecl 0x8088D0 => unported;
    /// `_rpD3D9NativeDataWorldSectorStreamRead(stream, sector)` (0x7641C0)
    fn native_data_world_sector_stream_read(s: GPtr<u8>, sector: GPtr<u8>) -> GPtr<u8> = cdecl 0x7641C0 => unported;
}

/// The size `GetSize` reports for a native resource entry: the declaration element count from
/// `GetDeclaration` (into a 0x98-byte stack buffer in the original), then
/// `entry.size + 0x14 + 2 * (totalNumIndex + 4 * count) + 4 + 0x10` plus `totalNumVertex * stride`
/// for each stream with a vertex buffer.
fn native_entry_size(e: GPtr<u8>) -> u32 {
    let decl = p8(rd::<u32>(e, 0x4C));
    let mut count: u32 = 0;
    let mut elements = [0u8; 0x98];
    decl_get_declaration(decl, GPtr::from_raw(elements.as_mut_ptr()), GPtr::from_raw(&mut count));
    let base = rd::<u32>(e, 8).wrapping_add(0x14);
    let mut size = base
        .wrapping_add(rd::<u32>(e, 0x50).wrapping_add(count.wrapping_mul(4)).wrapping_mul(2))
        .wrapping_add(4);
    for k in 0..2 {
        size = size.wrapping_add(0x10);
        if rd::<u32>(e, 0x28 + k * 0x10) != 0 {
            size = size.wrapping_add(rd::<u32>(e, 0x54).wrapping_mul(rd::<u32>(e, 0x30 + k * 0x10)));
        }
    }
    size
}

/// `_rpD3D9NativeDataGeometryStreamGetSize(geometry)` (0x763EB0): 0 unless rpGEOMETRYNATIVE
/// (flags 0x1000000); otherwise the native size of the geometry's resource entry (+0x58, not checked
/// for NULL).
#[hook(addr = 0x763EB0, conv = cdecl, name = "_rpD3D9NativeDataGeometryStreamGetSize", expect = "8B 4C 24 04 81 EC A4 00 00 00",
       unverified = "rw5-native-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn native_data_geometry_stream_get_size(geometry: GPtr<u8>) -> u32 {
    if rd::<u32>(geometry, 8) & 0x100_0000 == 0 {
        return 0;
    }
    native_entry_size(p8(rd::<u32>(geometry, 0x58)))
}

/// `_rpD3D9NativeDataWorldSectorStreamGetSize(sector)` (0x7645C0): 0 without a resource entry (+0x34)
/// or when the sector's world (`RpWorldSectorGetWorld`) is not rpWORLDNATIVE (0x1000000); otherwise the
/// native size of the entry.
#[hook(addr = 0x7645C0, conv = cdecl, name = "_rpD3D9NativeDataWorldSectorStreamGetSize", expect = "81 EC A4 00 00 00",
       unverified = "rw5-native-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn native_data_world_sector_stream_get_size(sector: GPtr<u8>) -> u32 {
    if rd::<u32>(sector, 0x34) == 0 {
        return 0;
    }
    if rd::<u32>(world_sector_get_world(sector), 8) & 0x100_0000 == 0 {
        return 0;
    }
    native_entry_size(p8(rd::<u32>(sector, 0x34)))
}

/// `_rpD3D9NativeDataGeometryStreamWrite(stream, geometry)` (0x7637F0). Without rpGEOMETRYNATIVE returns
/// the stream. Otherwise (header = entry + 0x18): the declaration (`GetDeclaration`, a 0xA0-byte stack
/// buffer), the chunk size `entry.size + 8 + 8 * count + 4` (+ `2 * totalNumIndex` with an index
/// buffer, + 0x10 per stream, + `totalNumVertex * stride` per vertex buffer); each mesh's material
/// pointer is replaced by its material-list index; then a STRUCT chunk header (version 0x36003, build
/// 0xFFFF), the platform id (4 bytes at 0x8D7384), `entry.size`, the header bytes, the element count and
/// elements; the index buffer (locked whole, `2 * totalNumIndex` bytes; a failed lock or write returns
/// 0); each stream's 0x10 descriptor and, with a vertex buffer, its locked range (offset, `stride *
/// totalNumVertex`, flags NOSYSLOCK; a failed lock or write returns 0). Finally the material indices
/// are turned back into pointers and the stream is returned. Every failure returns 0 with the meshes
/// still holding indices (original bug, kept).
#[hook(addr = 0x7637F0, conv = cdecl, name = "_rpD3D9NativeDataGeometryStreamWrite", expect = "8B 44 24 08 81 EC AC 00 00 00",
       unverified = "rw5-native-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "stream writes (file effects); rewrites the mesh material pointers in place")]
pub fn native_data_geometry_stream_write(stream: GPtr<u8>, geometry: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(geometry, 8) & 0x100_0000 == 0 {
        return stream;
    }
    let entry = p8(rd::<u32>(geometry, 0x58));
    let h = entry.byte_add(0x18);
    let mut count: u32 = 0;
    let mut elements = [0u8; 0xA0];
    let ep = GPtr::from_raw(elements.as_mut_ptr());
    decl_get_declaration(p8(rd::<u32>(h, 0x34)), ep, GPtr::from_raw(&mut count));
    let mut size = rd::<u32>(entry, 8)
        .wrapping_add(8)
        .wrapping_add(count.wrapping_mul(8))
        .wrapping_add(4);
    if rd::<u32>(h, 8) != 0 {
        size = size.wrapping_add(rd::<u32>(h, 0x38).wrapping_mul(2));
    }
    for k in 0..2 {
        size = size.wrapping_add(0x10);
        if rd::<u32>(h, 0x10 + k * 0x10) != 0 {
            size = size.wrapping_add(rd::<u32>(h, 0x3C).wrapping_mul(rd::<u32>(h, 0x18 + k * 0x10)));
        }
    }
    let mat_list = geometry.byte_add(0x20);
    let n = rd::<u32>(h, 4);
    for i in 0..n {
        let m = h.byte_add(0x48 + (i as isize) * 0x24);
        wr::<u32>(m, 0, material_list_find_index(mat_list, p8(rd::<u32>(m, 0))) as u32);
    }
    if stream_write_chunk_header(stream, 1, size, 0x36003, 0xFFFF).is_null() {
        return GPtr::NULL;
    }
    if stream_write(stream, p8(PLATFORM_ID), 4).is_null() {
        return GPtr::NULL;
    }
    let entry = p8(rd::<u32>(geometry, 0x58));
    if stream_write(stream, entry.byte_add(8), 4).is_null() {
        return GPtr::NULL;
    }
    let entry = p8(rd::<u32>(geometry, 0x58));
    if stream_write(stream, entry.byte_add(0x18), rd::<u32>(entry, 8)).is_null() {
        return GPtr::NULL;
    }
    if stream_write(stream, GPtr::from_raw(&mut count).cast(), 4).is_null() {
        return GPtr::NULL;
    }
    if stream_write(stream, ep, count.wrapping_mul(8)).is_null() {
        return GPtr::NULL;
    }
    let ib = p8(rd::<u32>(h, 8));
    if !ib.is_null() {
        let mut data: u32 = 0;
        buffer_lock(ib, 0, 0, GPtr::from_raw(&mut data), 0);
        if data == 0 {
            return GPtr::NULL;
        }
        let w = stream_write(stream, p8(data), rd::<u32>(h, 0x38).wrapping_mul(2));
        buffer_unlock(p8(rd::<u32>(h, 8)));
        if w.is_null() {
            return GPtr::NULL;
        }
    }
    for k in 0..2isize {
        let s = h.byte_add(0x10 + k * 0x10);
        if stream_write(stream, s, 0x10).is_null() {
            return GPtr::NULL;
        }
        let vb = p8(rd::<u32>(s, 0));
        if !vb.is_null() {
            let mut data: u32 = 0;
            if buffer_lock(
                vb,
                rd::<u32>(s, 4),
                rd::<u32>(s, 8).wrapping_mul(rd::<u32>(h, 0x3C)),
                GPtr::from_raw(&mut data),
                0x800,
            ) != 0
            {
                return GPtr::NULL;
            }
            let w = stream_write(stream, p8(data), rd::<u32>(s, 8).wrapping_mul(rd::<u32>(h, 0x3C)));
            buffer_unlock(p8(rd::<u32>(s, 0)));
            if w.is_null() {
                return GPtr::NULL;
            }
        }
    }
    let n = rd::<u32>(h, 4);
    for i in 0..n {
        let m = h.byte_add(0x48 + (i as isize) * 0x24);
        wr::<u32>(m, 0, material_list_get(mat_list, rd::<u32>(m, 0) as i32).addr() as u32);
    }
    stream
}

/// `_rpD3D9NativeDataGeometryStreamRead(stream, geometry)` (0x763A80). `RwStreamFindChunk(STRUCT)`, then
/// a version outside [0x34000, 0x36003] raises E_RW_BADVERSION (plugin id 2) and returns NULL; the
/// platform id must be 9; the entry size is read and the entry allocated (`rwmalloc(size + 0x18,
/// 0x3050D)`, not checked) into `geometry->repEntry` (+0x58): links cleared, owner = geometry, size,
/// ownerRef = &repEntry, destroy notify 0x4C9990; the header bytes are read over it and the index
/// buffer, the two vertex-buffer pointers and the declaration cleared. The declaration elements (count,
/// then `8 * count` bytes into a 0xA0-byte stack buffer) give `RwD3D9CreateVertexDeclaration`. With
/// indices an index buffer is created (failure destroys the entry, NULL) and filled from the stream
/// (fewer bytes than `2 * totalNumIndex`: destroy, NULL). Each stream descriptor is read (short: destroy,
/// NULL); with a vertex buffer a dynamic buffer (more than one morph target, managed byte 0) or a
/// managed one from `RwD3D9CreateVertexBuffer` (byte 1) is created; when creation fails and no earlier
/// stream succeeded the stream is skipped, but after a success the lock and read go ahead on the failed
/// buffer (original bug, kept); the buffer is locked (NOSYSLOCK, plus DISCARD for dynamic ones) and
/// filled. Unless the last buffer read succeeded the entry is destroyed (NULL). Two vertex buffers: with
/// caps bit 0 (0xC9BFD4) `useOffsets` = 1, and every mesh's baseIndex = minVertex; otherwise
/// `useOffsets` = 0 and baseIndex = `offset0 / stride0 + minVertex` (both loops run `numMeshes` times as
/// do-whiles: 0 wraps). Material indices become pointers; returns the geometry.
#[hook(addr = 0x763A80, conv = cdecl, name = "_rpD3D9NativeDataGeometryStreamRead", expect = "81 EC CC 00 00 00",
       unverified = "rw5-native-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "stream reads (file effects); creates D3D buffers and a resource entry")]
pub fn native_data_geometry_stream_read(stream: GPtr<u8>, geometry: GPtr<u8>) -> GPtr<u8> {
    let mut len: u32 = 0;
    let mut version: u32 = 0;
    if stream_find_chunk(stream, 1, GPtr::from_raw(&mut len), GPtr::from_raw(&mut version)) == 0 {
        return GPtr::NULL;
    }
    if !(0x34000..=0x36003).contains(&version) {
        let mut e = RwError { plugin_id: 2, error_code: 0 };
        e.error_code = rw_error(0x8000_0004) as i32;
        rw_error_set(GPtr::from_raw(&mut e));
        return GPtr::NULL;
    }
    let mut platform: u32 = 0;
    if stream_read(stream, GPtr::from_raw(&mut platform).cast(), 4) != 4 || platform != 9 {
        return GPtr::NULL;
    }
    let mut size: u32 = 0;
    if stream_read(stream, GPtr::from_raw(&mut size).cast(), 4) != 4 {
        return GPtr::NULL;
    }
    let slot = geometry.byte_add(0x58);
    let entry = rw_malloc(size.wrapping_add(0x18), 0x3050D);
    wr::<u32>(slot, 0, entry.addr() as u32);
    wr::<u32>(entry, 0, 0);
    wr::<u32>(p8(rd::<u32>(slot, 0)), 4, 0);
    wr::<u32>(p8(rd::<u32>(slot, 0)), 0xC, geometry.addr() as u32);
    wr::<u32>(p8(rd::<u32>(slot, 0)), 8, size);
    wr::<u32>(p8(rd::<u32>(slot, 0)), 0x10, slot.addr() as u32);
    wr::<u32>(p8(rd::<u32>(slot, 0)), 0x14, ENTRY_DESTROY);
    if stream_read(stream, p8(rd::<u32>(slot, 0)).byte_add(0x18), size) != size {
        return GPtr::NULL;
    }
    let destroy = || entry_destroy(p8(rd::<u32>(slot, 0)));
    let h = p8(rd::<u32>(slot, 0)).byte_add(0x18);
    wr::<u32>(h, 8, 0);
    wr::<u32>(h, 0x10, 0);
    wr::<u32>(h, 0x20, 0);
    wr::<u32>(h, 0x34, 0);
    let mut count: u32 = 0;
    if stream_read(stream, GPtr::from_raw(&mut count).cast(), 4) != 4 {
        return GPtr::NULL;
    }
    let mut elements = [0u8; 0xA0];
    let ep = GPtr::from_raw(elements.as_mut_ptr());
    if stream_read(stream, ep, count.wrapping_mul(8)) != count.wrapping_mul(8) {
        return GPtr::NULL;
    }
    create_vertex_declaration(ep, h.byte_add(0x34).cast());
    let num_index = rd::<u32>(h, 0x38);
    if num_index != 0 && index_buffer_create(num_index, h.byte_add(8).addr() as u32) == 0 {
        destroy();
        return GPtr::NULL;
    }
    let ib = p8(rd::<u32>(h, 8));
    if !ib.is_null() {
        let mut data: u32 = 0;
        let mut got: u32 = 0;
        buffer_lock(ib, 0, 0, GPtr::from_raw(&mut data), 0);
        let want = rd::<u32>(h, 0x38).wrapping_mul(2);
        if data != 0 {
            got = stream_read(stream, p8(data), want);
            buffer_unlock(p8(rd::<u32>(h, 8)));
        }
        if got != want {
            destroy();
            return GPtr::NULL;
        }
    }
    let mut num_vb: u32 = 0;
    let mut failed = true;
    for k in 0..2isize {
        let s = h.byte_add(0x10 + k * 0x10);
        if stream_read(stream, s, 0x10) != 0x10 {
            destroy();
            return GPtr::NULL;
        }
        if rd::<u32>(s, 0) == 0 {
            continue;
        }
        num_vb += 1;
        let stride = rd::<u32>(s, 8);
        let bytes = rd::<u32>(h, 0x3C).wrapping_mul(stride);
        let ok = if rd::<i32>(geometry, 0x18) > 1 {
            wr::<u8>(s, 0xE, 0);
            dynamic_vertex_buffer_create(bytes, s.cast())
        } else {
            wr::<u8>(s, 0xE, 1);
            create_vertex_buffer(stride, bytes, s.cast(), s.byte_add(4).cast())
        };
        if ok == 0 && failed {
            continue;
        }
        failed = true;
        let flags = if rd::<u8>(s, 0xE) != 0 { 0x800 } else { 0x2800 };
        let mut data: u32 = 0;
        buffer_lock(p8(rd::<u32>(s, 0)), rd::<u32>(s, 4), bytes, GPtr::from_raw(&mut data), flags);
        let got = stream_read(stream, p8(data), bytes);
        buffer_unlock(p8(rd::<u32>(s, 0)));
        if got == bytes {
            failed = false;
        }
    }
    if failed {
        destroy();
        return GPtr::NULL;
    }
    let mut left = rd::<u32>(h, 4);
    if num_vb > 1 {
        if g::<u32>(CAPS_OFFSETS).read() & 1 != 0 {
            wr::<u32>(h, 0x30, 1);
        }
        let mut m = h.byte_add(0x40);
        loop {
            wr::<u32>(m, 0x14, rd::<u32>(m, 4));
            m = m.byte_add(0x24);
            left = left.wrapping_sub(1);
            if left == 0 {
                break;
            }
        }
    } else {
        wr::<u32>(h, 0x30, 0);
        let mut m = h.byte_add(0x40);
        loop {
            let base = rd::<u32>(h, 0x14) / rd::<u32>(h, 0x18);
            wr::<u32>(m, 0x14, base.wrapping_add(rd::<u32>(m, 4)));
            m = m.byte_add(0x24);
            left = left.wrapping_sub(1);
            if left == 0 {
                break;
            }
        }
    }
    let mat_list = geometry.byte_add(0x20);
    let n = rd::<u32>(h, 4);
    for i in 0..n {
        let m = h.byte_add(0x48 + (i as isize) * 0x24);
        wr::<u32>(m, 0, material_list_get(mat_list, rd::<u32>(m, 0) as i32).addr() as u32);
    }
    geometry
}

/// `cb_NATIVEDATAPLUGIN_RpGeometry_StreamRead(stream, length, geometry, offset, size)` (0x750E20):
/// `_rpD3D9NativeDataGeometryStreamRead(stream, geometry)`; the stream when it returned non-NULL,
/// else NULL.
#[hook(addr = 0x750E20, conv = cdecl, name = "cb_NATIVEDATAPLUGIN_RpGeometry_StreamRead", expect = "8B 44 24 0C 56",
       unverified = "rw5-native-1", route = "split", hazards = "heap-writes", recs = "grove-walk")]
pub fn native_data_plugin_geometry_stream_read(stream: GPtr<u8>, _len: u32, geometry: GPtr<u8>, _off: u32, _size: u32) -> GPtr<u8> {
    if orig_geometry_read(stream, geometry).is_null() { GPtr::NULL } else { stream }
}

orig! {
    /// `_rpD3D9NativeDataGeometryStreamRead(stream, geometry)` (0x763A80)
    fn orig_geometry_read(s: GPtr<u8>, geometry: GPtr<u8>) -> GPtr<u8> = cdecl 0x763A80 => native_data_geometry_stream_read;
}

/// `cb_NATIVEDATAPLUGIN_RpWorldSector_StreamRead(stream, length, sector, offset, size)` (0x750E70):
/// `_rpD3D9NativeDataWorldSectorStreamRead(stream, sector)`; the stream when it returned non-NULL,
/// else NULL.
#[hook(addr = 0x750E70, conv = cdecl, name = "cb_NATIVEDATAPLUGIN_RpWorldSector_StreamRead", expect = "8B 44 24 0C 56",
       unverified = "rw5-native-1", route = "split", hazards = "heap-writes", recs = "grove-walk")]
pub fn native_data_plugin_world_sector_stream_read(stream: GPtr<u8>, _len: u32, sector: GPtr<u8>, _off: u32, _size: u32) -> GPtr<u8> {
    if native_data_world_sector_stream_read(stream, sector).is_null() { GPtr::NULL } else { stream }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_NATIVE_DATA_GEOMETRY_STREAM_GET_SIZE,
    HOOK_NATIVE_DATA_WORLD_SECTOR_STREAM_GET_SIZE,
    HOOK_NATIVE_DATA_GEOMETRY_STREAM_WRITE,
    HOOK_NATIVE_DATA_GEOMETRY_STREAM_READ,
    HOOK_NATIVE_DATA_PLUGIN_GEOMETRY_STREAM_READ,
    HOOK_NATIVE_DATA_PLUGIN_WORLD_SECTOR_STREAM_READ,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
