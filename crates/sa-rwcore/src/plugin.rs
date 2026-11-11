//! PORT-RW-1 module `plugin`: `RwPluginRegistry` (see docs/arch/rw-port.md). Ported from the 1.0 US bytes;
//! `docs/re/specs/RW/RwPluginRegistry.md` as a reading aid.
//!
//! `RwPluginRegistry` (0x18): +0 sizeOfStruct, +4 origSizeOfStruct, +8 maxSizeOfStruct, +0xC staticAlloc,
//! +0x10 firstRegEntry, +0x14 lastRegEntry. `RwPluginRegEntry` (0x3C): +0 offset, +4 size, +8 pluginID,
//! +0xC readCB, +0x10 writeCB, +0x14 getSizeCB, +0x18 alwaysCB, +0x1C rightsCB, +0x20 constructCB,
//! +0x24 destructCB, +0x28 copyCB, +0x2C errStrCB, +0x30 next, +0x34 prev, +0x38 parentRegistry.
//! Every callback is called through its stored pointer.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{error1, p8, rd, rw_free, rw_globals, rw_malloc, rw_memory_alloc, wr};

/// `_rwFreeListAllocReal` (the default `G->memoryAlloc`).
const FREELIST_ALLOC_REAL: u32 = 0x801C30;
/// `_rwPluginRegistryDefaultConstructor` / `Destructor` / `CopyConstructor`.
const DEFAULT_CTOR: u32 = 0x8086B0;
const DEFAULT_DTOR: u32 = 0x8086C0;
const DEFAULT_COPY: u32 = 0x8086D0;

sa_core::globals! {
    list PLUGIN_GLOBALS;
    /// Registry-entry freelist (`RwFreeList*`, created by the memory module)
    pub static REG_ENTRY_FREELIST @ 0xC9A6E4: u32 = 0;
    /// `RwPluginRegistry**` array of registries (kept only without the default freelist allocator)
    pub static REGISTRY_ARRAY @ 0xC9A6E8: u32 = 0;
    /// Number of registries in [`REGISTRY_ARRAY`]
    pub static REGISTRY_COUNT @ 0xC9A6EC: u32 = 0;
    /// Engine `RwPluginRegistry`
    pub static ENGINE_REGISTRY @ 0x8E2298: [u32; 6] = [0x158, 0x158, 0, 0, 0, 0];
}

orig! {
    /// `_rwGetNumEngineInstances()` (0x7F2B90, uncatalogued name)
    pub fn get_num_engine_instances() -> u32 = cdecl 0x7F2B90 => unported;
    /// `RwStreamFindChunk(stream, type, lengthOut, versionOut)` (0x7ED2D0)
    pub fn stream_find_chunk(s: GPtr<u8>, ty: u32, len: GPtr<u32>, ver: GPtr<u32>) -> i32 = cdecl 0x7ED2D0 => unported;
    /// `_rwStreamReadChunkHeader(stream, type, length, version, buildNum)` (0x7ED0F0)
    pub fn stream_read_chunk_header(s: GPtr<u8>, ty: GPtr<u32>, len: GPtr<u32>, ver: GPtr<u32>, build: GPtr<u32>) -> i32 = cdecl 0x7ED0F0 => unported;
    /// `RwStreamSkip(stream, length)` (0x7ECD00)
    pub fn stream_skip(s: GPtr<u8>, len: u32) -> GPtr<u8> = cdecl 0x7ECD00 => unported;
    /// `_rwStreamWriteVersionedChunkHeader(stream, type, size, version, buildNum)` (0x7ED270)
    pub fn stream_write_versioned_chunk_header(s: GPtr<u8>, ty: u32, size: i32, ver: u32, build: u32) -> GPtr<u8> = cdecl 0x7ED270 => unported;

    /// `constructCB` / `destructCB` / `alwaysCB` / `getSizeCB`: `cb(obj, offset, size)`
    pub fn call_obj_cb(f: u32, obj: GPtr<u8>, offset: i32, size: i32) -> u32 = cdecl dyn => unported;
    /// `copyCB(dst, src, offset, size)`
    pub fn call_copy_cb(f: u32, dst: GPtr<u8>, src: GPtr<u8>, offset: i32, size: i32) -> u32 = cdecl dyn => unported;
    /// `readCB` / `writeCB`: `cb(stream, length, obj, offset, size)`
    pub fn call_stream_cb(f: u32, s: GPtr<u8>, len: i32, obj: GPtr<u8>, offset: i32, size: i32) -> u32 = cdecl dyn => unported;
    /// `rightsCB(obj, offset, size, extraData)`
    pub fn call_rights_cb(f: u32, obj: GPtr<u8>, offset: i32, size: i32, extra: u32) -> u32 = cdecl dyn => unported;

    /// `_rwPluginRegistryAddPlugin` (0x8084A0)
    pub fn add_plugin_call(reg: GPtr<u8>, size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x8084A0 => registry_add_plugin;
    /// `_rwPluginRegistryGetPluginOffset` (0x808470)
    pub fn get_plugin_offset_call(reg: GPtr<u8>, id: u32) -> i32 = cdecl 0x808470 => registry_get_plugin_offset;
}

fn a32(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// The engine registry (`0x8E2298`).
pub fn engine_registry() -> GPtr<u8> {
    ENGINE_REGISTRY.ptr().cast()
}

/// Entry lookup: first entry from `firstRegEntry` with `pluginID == id`, or NULL.
fn find(reg: GPtr<u8>, id: u32) -> GPtr<u8> {
    let mut e = p8(rd::<u32>(reg, 0x10));
    while !e.is_null() {
        if rd::<u32>(e, 8) == id {
            return e;
        }
        e = p8(rd::<u32>(e, 0x30));
    }
    e
}

fn e_offset(e: GPtr<u8>) -> i32 {
    rd::<i32>(e, 0)
}
fn e_size(e: GPtr<u8>) -> i32 {
    rd::<i32>(e, 4)
}

/// `RwEngineRegisterPlugin` (0x7F2BB0): `_rwPluginRegistryAddPlugin(engine registry, ..., copy = NULL)`.
#[hook(addr = 0x7F2BB0, conv = cdecl, name = "RwEngineRegisterPlugin", expect = "8B 44 24 10 8B 4C 24 0C")]
pub fn engine_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32) -> i32 {
    add_plugin_call(engine_registry(), size, id, ctor, dtor, 0)
}

/// `RwEngineGetPluginOffset` (0x7F2BE0).
#[hook(addr = 0x7F2BE0, conv = cdecl, name = "RwEngineGetPluginOffset", expect = "8B 44 24 04 50")]
pub fn engine_get_plugin_offset(id: u32) -> i32 {
    get_plugin_offset_call(engine_registry(), id)
}

/// `_rwPluginRegistryGetPluginOffset` (0x808470): the entry's offset, or -1.
#[hook(addr = 0x808470, conv = cdecl, name = "_rwPluginRegistryGetPluginOffset", expect = "8B 44 24 04 8B 40 10")]
pub fn registry_get_plugin_offset(reg: GPtr<u8>, id: u32) -> i32 {
    let e = find(reg, id);
    if e.is_null() { -1 } else { e_offset(e) }
}

/// `_rwPluginRegistryAddPlugin` (0x8084A0). Without the default freelist allocator the registry is
/// recorded in the registry array (the new array is not NULL-checked; when the old array is NULL the
/// registry goes to index 0 whatever the count). A duplicate id reports E_RW_PLUGININIT and returns the
/// existing offset.
#[hook(addr = 0x8084A0, conv = cdecl, name = "_rwPluginRegistryAddPlugin", expect = "A1 E4 A6 C9 00")]
pub fn registry_add_plugin(reg: GPtr<u8>, size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 {
    if REG_ENTRY_FREELIST.get() == 0 {
        return -1;
    }
    if get_num_engine_instances() != 0 {
        error1(0x8000_0017);
        return -1;
    }
    if rd::<u32>(rw_globals(), 0x144) != FREELIST_ALLOC_REAL {
        let count = REGISTRY_COUNT.get();
        let mut i: u32 = 0;
        while i < count {
            if rd::<u32>(p8(REGISTRY_ARRAY.get()), (i as isize) * 4) == a32(reg) {
                break;
            }
            i += 1;
        }
        if count == i {
            let n = rw_malloc(count.wrapping_mul(4).wrapping_add(4), 0x40000);
            let mut k: u32 = 0;
            if REGISTRY_ARRAY.get() != 0 {
                while k < REGISTRY_COUNT.get() {
                    let old = p8(REGISTRY_ARRAY.get());
                    k += 1;
                    wr::<u32>(n, ((k - 1) as isize) * 4, rd::<u32>(old, ((k - 1) as isize) * 4));
                }
                rw_free(p8(REGISTRY_ARRAY.get()));
            }
            wr::<u32>(n, (k as isize) * 4, a32(reg));
            let c = REGISTRY_COUNT.get().wrapping_add(1);
            REGISTRY_ARRAY.set(a32(n));
            REGISTRY_COUNT.set(c);
        }
    }
    let dup = find(reg, id);
    if !dup.is_null() {
        error1(0x8000_0017);
        return e_offset(dup);
    }
    let new_size = (size.wrapping_add(3) & !3).wrapping_add(rd::<i32>(reg, 0));
    let max = rd::<i32>(reg, 8);
    if max != 0 && new_size > max {
        return -1;
    }
    let e = rw_memory_alloc(p8(REG_ENTRY_FREELIST.get()), 0x40000);
    if e.is_null() {
        return -1;
    }
    wr::<u32>(e, 0, rd::<u32>(reg, 0));
    wr::<i32>(reg, 0, new_size);
    wr::<i32>(e, 4, size);
    wr::<u32>(e, 8, id);
    for off in [0xC, 0x10, 0x14, 0x18, 0x1C] {
        wr::<u32>(e, off, 0);
    }
    wr::<u32>(e, 0x20, if ctor == 0 { DEFAULT_CTOR } else { ctor });
    wr::<u32>(e, 0x24, if dtor == 0 { DEFAULT_DTOR } else { dtor });
    wr::<u32>(e, 0x28, if copy == 0 { DEFAULT_COPY } else { copy });
    wr::<u32>(e, 0x2C, 0);
    wr::<u32>(e, 0x30, 0);
    wr::<u32>(e, 0x34, 0);
    wr::<u32>(e, 0x38, a32(reg));
    if rd::<u32>(reg, 0x10) == 0 {
        wr::<u32>(reg, 0x10, a32(e));
        wr::<u32>(reg, 0x14, a32(e));
    } else {
        wr::<u32>(p8(rd::<u32>(reg, 0x14)), 0x30, a32(e));
        wr::<u32>(e, 0x34, rd::<u32>(reg, 0x14));
        wr::<u32>(reg, 0x14, a32(e));
    }
    e_offset(e)
}

/// `_rwPluginRegistryDefaultConstructor` (0x8086B0): returns obj.
#[hook(addr = 0x8086B0, conv = cdecl, name = "_rwPluginRegistryDefaultConstructor", expect = "8B 44 24 04 C3")]
pub fn default_constructor(obj: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    obj
}

/// `_rwPluginRegistryDefaultDestructor` (0x8086C0): returns obj.
#[hook(addr = 0x8086C0, conv = cdecl, name = "_rwPluginRegistryDefaultDestructor", expect = "8B 44 24 04 C3")]
pub fn default_destructor(obj: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    obj
}

/// `_rwPluginRegistryDefaultCopyConstructor` (0x8086D0): returns dst.
#[hook(addr = 0x8086D0, conv = cdecl, name = "_rwPluginRegistryDefaultCopyConstructor", expect = "8B 44 24 04 C3")]
pub fn default_copy_constructor(dst: GPtr<u8>, _src: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    dst
}

/// `_rwPluginRegistryInitObject` (0x8086E0): constructors first to last; on a NULL result the
/// already-run entries' destructors (backwards) and NULL. Returns reg.
#[hook(addr = 0x8086E0, conv = cdecl, name = "_rwPluginRegistryInitObject", expect = "53 8B 5C 24 08")]
pub fn registry_init_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> {
    let mut e = p8(rd::<u32>(reg, 0x10));
    while !e.is_null() {
        if call_obj_cb(rd::<u32>(e, 0x20), obj, e_offset(e), e_size(e)) == 0 {
            let mut p = p8(rd::<u32>(e, 0x34));
            while !p.is_null() {
                call_obj_cb(rd::<u32>(p, 0x24), obj, e_offset(p), e_size(p));
                p = p8(rd::<u32>(p, 0x34));
            }
            return GPtr::NULL;
        }
        e = p8(rd::<u32>(e, 0x30));
    }
    reg
}

/// `_rwPluginRegistryDeInitObject` (0x808740): destructors last to first. Returns reg.
#[hook(addr = 0x808740, conv = cdecl, name = "_rwPluginRegistryDeInitObject", expect = "53 8B 5C 24 08")]
pub fn registry_deinit_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> {
    let mut e = p8(rd::<u32>(reg, 0x14));
    while !e.is_null() {
        call_obj_cb(rd::<u32>(e, 0x24), obj, e_offset(e), e_size(e));
        e = p8(rd::<u32>(e, 0x34));
    }
    reg
}

/// `_rwPluginRegistryCopyObject` (0x808770): copy constructors first to last. Returns reg.
#[hook(addr = 0x808770, conv = cdecl, name = "_rwPluginRegistryCopyObject", expect = "55 8B 6C 24 08")]
pub fn registry_copy_object(reg: GPtr<u8>, dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    let mut e = p8(rd::<u32>(reg, 0x10));
    while !e.is_null() {
        call_copy_cb(rd::<u32>(e, 0x28), dst, src, e_offset(e), e_size(e));
        e = p8(rd::<u32>(e, 0x30));
    }
    reg
}

/// `_rwPluginRegistryAddPluginStream` (0x8088E0): sets read/write/getSize; offset or -1.
#[hook(addr = 0x8088E0, conv = cdecl, name = "_rwPluginRegistryAddPluginStream", expect = "8B 44 24 04 8B 40 10")]
pub fn registry_add_plugin_stream(reg: GPtr<u8>, id: u32, read: u32, write: u32, get_size: u32) -> i32 {
    let e = find(reg, id);
    if e.is_null() {
        return -1;
    }
    wr::<u32>(e, 0xC, read);
    wr::<u32>(e, 0x10, write);
    wr::<u32>(e, 0x14, get_size);
    e_offset(e)
}

/// `_rwPluginRegistryAddPlgnStrmlwysCB` (0x808920): sets alwaysCB; offset or -1.
#[hook(addr = 0x808920, conv = cdecl, name = "_rwPluginRegistryAddPlgnStrmlwysCB", expect = "8B 44 24 04 8B 40 10")]
pub fn registry_add_plugin_stream_always_cb(reg: GPtr<u8>, id: u32, cb: u32) -> i32 {
    let e = find(reg, id);
    if e.is_null() {
        return -1;
    }
    wr::<u32>(e, 0x18, cb);
    e_offset(e)
}

/// `_rwPluginRegistryAddPlgnStrmRightsCB` (0x808950): sets rightsCB; offset or -1.
#[hook(addr = 0x808950, conv = cdecl, name = "_rwPluginRegistryAddPlgnStrmRightsCB", expect = "8B 44 24 04 8B 40 10")]
pub fn registry_add_plugin_stream_rights_cb(reg: GPtr<u8>, id: u32, cb: u32) -> i32 {
    let e = find(reg, id);
    if e.is_null() {
        return -1;
    }
    wr::<u32>(e, 0x1C, cb);
    e_offset(e)
}

/// `_rwPluginRegistryReadDataChunks` (0x808980). The remaining length is unsigned and only tested for
/// zero after the subtraction, so an inconsistent chunk length keeps reading. Without an extension
/// chunk the always-callbacks are not run.
#[hook(addr = 0x808980, conv = cdecl, name = "_rwPluginRegistryReadDataChunks", expect = "83 EC 10 8D 44 24 04")]
pub fn registry_read_data_chunks(reg: GPtr<u8>, s: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> {
    let mut length = [0u32; 1];
    let mut version = [0u32; 1];
    if stream_find_chunk(
        s,
        3,
        GPtr::from_raw(length.as_mut_ptr()),
        GPtr::from_raw(version.as_mut_ptr()),
    ) == 0
    {
        return GPtr::NULL;
    }
    if !(0x34000..=0x36003).contains(&version[0]) {
        error1(0x8000_0004);
        return GPtr::NULL;
    }
    let mut ty = [0u32; 1];
    let mut clen = [0u32; 1];
    let mut len = length[0];
    while len != 0 {
        if stream_read_chunk_header(
            s,
            GPtr::from_raw(ty.as_mut_ptr()),
            GPtr::from_raw(clen.as_mut_ptr()),
            GPtr::NULL,
            GPtr::NULL,
        ) == 0
        {
            return GPtr::NULL;
        }
        let e = find(reg, ty[0]);
        let ok = if !e.is_null() && rd::<u32>(e, 0xC) != 0 {
            call_stream_cb(rd::<u32>(e, 0xC), s, clen[0] as i32, obj, e_offset(e), e_size(e)) != 0
        } else {
            !stream_skip(s, clen[0]).is_null()
        };
        if !ok {
            return GPtr::NULL;
        }
        len = len.wrapping_add(0xFFFF_FFF4u32.wrapping_sub(clen[0]));
    }
    let mut e = p8(rd::<u32>(reg, 0x10));
    while !e.is_null() {
        let cb = rd::<u32>(e, 0x18);
        if cb != 0 && call_obj_cb(cb, obj, e_offset(e), e_size(e)) == 0 {
            return GPtr::NULL;
        }
        e = p8(rd::<u32>(e, 0x30));
    }
    reg
}

/// `_rwPluginRegistryInvokeRights` (0x808AB0): reg if the rights callback returned non-zero, else NULL
/// (also when the id or the callback is absent).
#[hook(addr = 0x808AB0, conv = cdecl, name = "_rwPluginRegistryInvokeRights", expect = "56 8B 74 24 08")]
pub fn registry_invoke_rights(reg: GPtr<u8>, id: u32, obj: GPtr<u8>, extra: u32) -> GPtr<u8> {
    let e = find(reg, id);
    if e.is_null() {
        return GPtr::NULL;
    }
    let cb = rd::<u32>(e, 0x1C);
    if cb == 0 {
        return GPtr::NULL;
    }
    if call_rights_cb(cb, obj, e_offset(e), e_size(e), extra) != 0 {
        reg
    } else {
        GPtr::NULL
    }
}

/// `_rwPluginRegistryGetSize` (0x808B00): sum of `getSize + 12` over the entries whose getSize > 0.
#[hook(addr = 0x808B00, conv = cdecl, name = "_rwPluginRegistryGetSize", expect = "8B 44 24 04 53")]
pub fn registry_get_size(reg: GPtr<u8>, obj: GPtr<u8>) -> i32 {
    let mut n: i32 = 0;
    let mut e = p8(rd::<u32>(reg, 0x10));
    while !e.is_null() {
        let cb = rd::<u32>(e, 0x14);
        if cb != 0 {
            let r = call_obj_cb(cb, obj, e_offset(e), e_size(e)) as i32;
            if r > 0 {
                n = n.wrapping_add(r).wrapping_add(0xC);
            }
        }
        e = p8(rd::<u32>(e, 0x30));
    }
    n
}

/// `_rwPluginRegistryWriteDataChunks` (0x808B40): the extension chunk header (size from the getSize
/// callbacks), then a chunk per entry with getSize > 0 and a writeCB.
#[hook(addr = 0x808B40, conv = cdecl, name = "_rwPluginRegistryWriteDataChunks", expect = "8B 44 24 04 53")]
pub fn registry_write_data_chunks(reg: GPtr<u8>, s: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> {
    let mut n: i32 = 0;
    let mut e = p8(rd::<u32>(reg, 0x10));
    while !e.is_null() {
        let cb = rd::<u32>(e, 0x14);
        if cb != 0 {
            let r = call_obj_cb(cb, obj, e_offset(e), e_size(e)) as i32;
            if r > 0 {
                n = n.wrapping_add(r).wrapping_add(0xC);
            }
        }
        e = p8(rd::<u32>(e, 0x30));
    }
    if stream_write_versioned_chunk_header(s, 3, n, 0x36003, 0xFFFF).is_null() {
        return GPtr::NULL;
    }
    let mut e = p8(rd::<u32>(reg, 0x10));
    while !e.is_null() {
        let cb = rd::<u32>(e, 0x14);
        if cb != 0 && rd::<u32>(e, 0x10) != 0 {
            let r = call_obj_cb(cb, obj, e_offset(e), e_size(e)) as i32;
            if r > 0 {
                if stream_write_versioned_chunk_header(s, rd::<u32>(e, 8), r, 0x36003, 0xFFFF).is_null() {
                    return GPtr::NULL;
                }
                if call_stream_cb(rd::<u32>(e, 0x10), s, r, obj, e_offset(e), e_size(e)) == 0 {
                    return GPtr::NULL;
                }
            }
        }
        e = p8(rd::<u32>(e, 0x30));
    }
    reg
}

/// `_rwPluginRegistrySkipDataChunks` (0x808C10).
#[hook(addr = 0x808C10, conv = cdecl, name = "_rwPluginRegistrySkipDataChunks", expect = "51 56 8B 74 24 10")]
pub fn registry_skip_data_chunks(reg: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> {
    let mut length = [0u32; 1];
    if stream_find_chunk(s, 3, GPtr::from_raw(length.as_mut_ptr()), GPtr::NULL) == 0 {
        return GPtr::NULL;
    }
    let mut clen = [0u32; 1];
    let mut len = length[0];
    while len != 0 {
        if stream_read_chunk_header(
            s,
            GPtr::NULL,
            GPtr::from_raw(clen.as_mut_ptr()),
            GPtr::NULL,
            GPtr::NULL,
        ) == 0
        {
            return GPtr::NULL;
        }
        if stream_skip(s, clen[0]).is_null() {
            return GPtr::NULL;
        }
        len = len.wrapping_add(0xFFFF_FFF4u32.wrapping_sub(clen[0]));
    }
    reg
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_ENGINE_REGISTER_PLUGIN,
    HOOK_ENGINE_GET_PLUGIN_OFFSET,
    HOOK_REGISTRY_GET_PLUGIN_OFFSET,
    HOOK_REGISTRY_ADD_PLUGIN,
    HOOK_DEFAULT_CONSTRUCTOR,
    HOOK_DEFAULT_DESTRUCTOR,
    HOOK_DEFAULT_COPY_CONSTRUCTOR,
    HOOK_REGISTRY_INIT_OBJECT,
    HOOK_REGISTRY_DEINIT_OBJECT,
    HOOK_REGISTRY_COPY_OBJECT,
    HOOK_REGISTRY_ADD_PLUGIN_STREAM,
    HOOK_REGISTRY_ADD_PLUGIN_STREAM_ALWAYS_CB,
    HOOK_REGISTRY_ADD_PLUGIN_STREAM_RIGHTS_CB,
    HOOK_REGISTRY_READ_DATA_CHUNKS,
    HOOK_REGISTRY_INVOKE_RIGHTS,
    HOOK_REGISTRY_GET_SIZE,
    HOOK_REGISTRY_WRITE_DATA_CHUNKS,
    HOOK_REGISTRY_SKIP_DATA_CHUNKS,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = PLUGIN_GLOBALS;
