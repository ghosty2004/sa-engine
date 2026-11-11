//! MATERIALMODULE (RpWorld): `RpMaterial` and `RpMaterialList` (PORT-RW-1). Ported from the 1.0 US
//! bytes; `docs/re/specs/RW/RpMaterial.md` is the reading aid.
//!
//! `RpMaterial` (0x1C + plugins, registry 0x8D62A4): +0 texture, +4 RwRGBA color, +8 pipeline, +0xC
//! surfaceProps {ambient, specular, diffuse}, +0x18 i16 refCount. `RpMaterialList`: +0 materials, +4
//! numMaterials, +8 space. Module `G + [0xC92514]` +0 = material freelist. RpWorld errors are
//! `RwErrorSet({2, _rwerror(..)})`.
//!
//! `RpMaterialCreate` and its inlined copies build the white colour in a stack dword; the inlined copy
//! in `RpMaterialStreamRead` uses its own `stream` argument slot for it (not reproduced).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{
    RwError, g, module, p8, rd, rw_error_set, rw_error1, rw_error2, rw_free, rw_malloc, rw_memory_alloc,
    rw_memory_free, rw_realloc, wr,
};
use crate::stream::rw_free_list_create_and_preallocate;

/// MATERIALMODULE offset global.
pub const MATERIAL_MODULE: usize = 0xC92514;
/// Module open count.
const MATERIAL_INSTANCES: usize = 0xC92518;
/// Static freelist struct.
const MATERIAL_FL_STATIC: u32 = 0xC924EC;
/// Plugin registry (`sizeOfStruct` at +0).
pub const MATERIAL_REGISTRY: u32 = 0x8D62A4;
/// Default surface properties (`.data`, 1.0f each).
const DEFAULT_AMBIENT: usize = 0x8D62BC;
const DEFAULT_SPECULAR: usize = 0x8D62C0;
const DEFAULT_DIFFUSE: usize = 0x8D62C4;
/// Freelist entries per block (0x100) and blocks to preallocate (1).
const MATERIAL_FL_PER_BLOCK: usize = 0x8D62C8;
const MATERIAL_FL_PREALLOC: usize = 0x8D62CC;
/// Rights plugin id / extra data set by the RIGHTTORENDER reader during `ReadDataChunks`.
const RIGHTS_PLUGIN_ID: usize = 0xC92510;
const RIGHTS_EXTRA_DATA: usize = 0xC924E8;

const E_RW_BADVERSION: u32 = 0x8000_0004;
const E_RW_NOMEM: u32 = 0x8000_0013;

orig! {
    /// `RwFreeListDestroy(fl)` (0x801B80)
    fn rw_free_list_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => unported;
    /// `_rwPluginRegistryInitObject(reg, obj)` (0x8086E0)
    fn plugin_init_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x8086E0 => crate::plugin::registry_init_object;
    /// `_rwPluginRegistryDeInitObject(reg, obj)` (0x808740)
    fn plugin_deinit_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808740 => crate::plugin::registry_deinit_object;
    /// `_rwPluginRegistryCopyObject(reg, dst, src)` (0x808770)
    fn plugin_copy_object(reg: GPtr<u8>, dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl 0x808770 => crate::plugin::registry_copy_object;
    /// `_rwPluginRegistryAddPlugin(reg, size, id, ctor, dtor, copy)` (0x8084A0)
    fn plugin_add(reg: GPtr<u8>, size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x8084A0 => crate::plugin::registry_add_plugin;
    /// `_rwPluginRegistryAddPluginStream(reg, id, read, write, getSize)` (0x8088E0)
    fn plugin_add_stream(reg: GPtr<u8>, id: u32, read: u32, write: u32, get_size: u32) -> i32 = cdecl 0x8088E0 => crate::plugin::registry_add_plugin_stream;
    /// `_rwPluginRegistryReadDataChunks(reg, stream, obj)` (0x808980)
    fn plugin_read_data_chunks(reg: GPtr<u8>, s: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808980 => crate::plugin::registry_read_data_chunks;
    /// `_rwPluginRegistryInvokeRights(reg, id, obj, extra)` (0x808AB0)
    fn plugin_invoke_rights(reg: GPtr<u8>, id: u32, obj: GPtr<u8>, extra: u32) -> GPtr<u8> = cdecl 0x808AB0 => crate::plugin::registry_invoke_rights;

    /// `RwTextureDestroy(texture)` (0x7F3820)
    fn rw_texture_destroy_call(t: GPtr<u8>) -> u32 = cdecl 0x7F3820 => crate::texture::rw_texture_destroy;
    /// `RwTextureStreamRead(stream)` (0x8046E0)
    fn rw_texture_stream_read_call(s: GPtr<u8>) -> GPtr<u8> = cdecl 0x8046E0 => crate::texture::rw_texture_stream_read;
    /// `RwStreamFindChunk(stream, type, length, version)` (0x7ED2D0)
    fn rw_stream_find_chunk_call(s: GPtr<u8>, ty: u32, len: GPtr<u32>, ver: GPtr<u32>) -> u32 = cdecl 0x7ED2D0 => crate::stream::rw_stream_find_chunk;
    /// `RwStreamRead(stream, buf, len)` (0x7EC9D0)
    fn rw_stream_read_call(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => crate::stream::rw_stream_read;
    /// `RwStreamReadInt32(stream, p, numBytes)` (0x7ED540)
    fn rw_stream_read_int32_call(s: GPtr<u8>, p: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x7ED540 => crate::stream::rw_stream_read_int32;
    /// `RpMaterialDestroy(material)` (0x74DA20)
    fn rp_material_destroy_call(m: GPtr<u8>) -> u32 = cdecl 0x74DA20 => rp_material_destroy;
    /// `RpMaterialStreamRead(stream)` (0x74DD30)
    fn rp_material_stream_read_call(s: GPtr<u8>) -> GPtr<u8> = cdecl 0x74DD30 => rp_material_stream_read;
    /// `_rpMaterialListAppendMaterial(list, material)` (0x74E350)
    fn material_list_append_call(l: GPtr<u8>, m: GPtr<u8>) -> i32 = cdecl 0x74E350 => material_list_append_material;
}

/// `RwErrorSet({2, _rwerror(code)})`.
fn error1_p2(code: u32) {
    let mut e = RwError {
        plugin_id: 2,
        error_code: 0,
    };
    e.error_code = rw_error1(code) as i32;
    rw_error_set(GPtr::from_raw(&mut e));
}

/// `RwErrorSet({2, _rwerror(code, a)})`.
fn error2_p2(code: u32, a: u32) {
    let mut e = RwError {
        plugin_id: 2,
        error_code: 0,
    };
    e.error_code = rw_error2(code, a) as i32;
    rw_error_set(GPtr::from_raw(&mut e));
}

/// The material freelist `G + [0xC92514]` +0.
fn material_free_list() -> GPtr<u8> {
    p8(rd::<u32>(module(MATERIAL_MODULE), 0))
}

/// `cb_MATERIALMODULE_RwEngine_Open(instance, offset, size)` (0x74D8E0): the material freelist (entry
/// size from the registry, hint 0x40007); NULL -> NULL, else counts the instance and returns it.
#[hook(addr = 0x74D8E0, conv = cdecl, name = "cb_MATERIALMODULE_RwEngine_Open", expect = "8B 44 24 08 8B 0D CC 62 8D 00")]
pub fn material_module_open(instance: GPtr<u8>, offset: u32, _size: u32) -> GPtr<u8> {
    let prealloc = g::<u32>(MATERIAL_FL_PREALLOC).read();
    let per_block = g::<u32>(MATERIAL_FL_PER_BLOCK).read();
    g::<u32>(MATERIAL_MODULE).write(offset);
    let size = g::<u32>(MATERIAL_REGISTRY as usize).read();
    let fl =
        rw_free_list_create_and_preallocate(size, per_block, 4, prealloc, p8(MATERIAL_FL_STATIC), 0x40007);
    wr::<u32>(module(MATERIAL_MODULE), 0, fl.addr() as u32);
    if rd::<u32>(module(MATERIAL_MODULE), 0) == 0 {
        return GPtr::NULL;
    }
    let c = g::<u32>(MATERIAL_INSTANCES);
    c.write(c.read().wrapping_add(1));
    instance
}

/// `cb_MATERIALMODULE_RwEngine_Close(instance, offset, size)` (0x74D950): destroys and clears the
/// freelist when set, decrements the open count, returns instance.
#[hook(addr = 0x74D950, conv = cdecl, name = "cb_MATERIALMODULE_RwEngine_Close", expect = "A1 24 7B C9 00")]
pub fn material_module_close(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    let fl = material_free_list();
    if !fl.is_null() {
        rw_free_list_destroy(fl);
        wr::<u32>(module(MATERIAL_MODULE), 0, 0);
    }
    let c = g::<u32>(MATERIAL_INSTANCES);
    c.write(c.read().wrapping_sub(1));
    instance
}

/// The inlined `RpMaterialCreate` body: refCount 1, white, no texture / pipeline, the default surface
/// properties (raw dwords), plugin init. NULL when the freelist is exhausted.
fn material_create() -> GPtr<u8> {
    let m = rw_memory_alloc(material_free_list(), 0x30007);
    if m.is_null() {
        return GPtr::NULL;
    }
    wr::<i16>(m, 0x18, 1);
    wr::<u32>(m, 0, 0);
    wr::<u32>(m, 4, 0xFFFF_FFFF);
    wr::<u32>(m, 8, 0);
    wr::<u32>(m, 0xC, g::<u32>(DEFAULT_AMBIENT).read());
    wr::<u32>(m, 0x10, g::<u32>(DEFAULT_SPECULAR).read());
    wr::<u32>(m, 0x14, g::<u32>(DEFAULT_DIFFUSE).read());
    plugin_init_object(p8(MATERIAL_REGISTRY), m);
    m
}

/// The inlined `RpMaterialDestroy` body.
fn material_destroy(m: GPtr<u8>) {
    let rc = rd::<i16>(m, 0x18);
    if rc == 1 {
        plugin_deinit_object(p8(MATERIAL_REGISTRY), m);
        let t = rd::<u32>(m, 0);
        if t != 0 {
            rw_texture_destroy_call(p8(t));
        }
        wr::<u32>(m, 0, 0);
        rw_memory_free(material_free_list(), m);
    } else {
        wr::<i16>(m, 0x18, rc.wrapping_sub(1));
    }
}

/// `RpMaterialCreate()` (0x74D990).
#[hook(addr = 0x74D990, conv = cdecl, name = "RpMaterialCreate", expect = "51 A1 24 7B C9 00")]
pub fn rp_material_create() -> GPtr<u8> {
    material_create()
}

/// `RpMaterialDestroy(material)` (0x74DA20): refCount == 1 (16-bit) -> plugin deinit, texture destroyed
/// and cleared, freed; otherwise refCount-- (also at 0 or below). Returns 1.
#[hook(addr = 0x74DA20, conv = cdecl, name = "RpMaterialDestroy", expect = "56 8B 74 24 08")]
pub fn rp_material_destroy(m: GPtr<u8>) -> u32 {
    material_destroy(m);
    1
}

/// `RpMaterialClone(material)` (0x74DA80): a new material taking a texture reference, color, pipeline,
/// surface properties and the plugin data of `src`. The original's `&src->surfaceProps == NULL` branch
/// (destroy the clone, NULL) is kept.
#[hook(addr = 0x74DA80, conv = cdecl, name = "RpMaterialClone", expect = "51 A1 24 7B C9 00")]
pub fn rp_material_clone(src: GPtr<u8>) -> GPtr<u8> {
    let m = material_create();
    if m.is_null() {
        return GPtr::NULL;
    }
    let t = rd::<u32>(src, 0);
    if t != 0 {
        wr::<i32>(p8(t), 0x54, rd::<i32>(p8(t), 0x54).wrapping_add(1));
        let old = rd::<u32>(m, 0);
        if old != 0 {
            rw_texture_destroy_call(p8(old));
        }
        wr::<u32>(m, 0, t);
    }
    wr::<u32>(m, 4, rd::<u32>(src, 4));
    wr::<u32>(m, 8, rd::<u32>(src, 8));
    if (src.addr() as u32).wrapping_add(0xC) == 0 {
        material_destroy(m);
        return GPtr::NULL;
    }
    wr::<u32>(m, 0xC, rd::<u32>(src, 0xC));
    wr::<u32>(m, 0x10, rd::<u32>(src, 0x10));
    wr::<u32>(m, 0x14, rd::<u32>(src, 0x14));
    plugin_copy_object(p8(MATERIAL_REGISTRY), m, src);
    m
}

/// `RpMaterialSetTexture(material, texture)` (0x74DBC0): the new texture's refCount++ first, then the
/// old one destroyed. Returns material.
#[hook(addr = 0x74DBC0, conv = cdecl, name = "RpMaterialSetTexture", expect = "56 57 8B 7C 24 10")]
pub fn rp_material_set_texture(m: GPtr<u8>, t: GPtr<u8>) -> GPtr<u8> {
    if !t.is_null() {
        wr::<i32>(t, 0x54, rd::<i32>(t, 0x54).wrapping_add(1));
    }
    let old = rd::<u32>(m, 0);
    if old != 0 {
        rw_texture_destroy_call(p8(old));
    }
    wr::<u32>(m, 0, t.addr() as u32);
    m
}

/// `RpMaterialRegisterPlugin(size, id, ctor, dtor, copy)` (0x74DBF0).
#[hook(addr = 0x74DBF0, conv = cdecl, name = "RpMaterialRegisterPlugin", expect = "8B 44 24 14 8B 4C 24 10")]
pub fn rp_material_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 {
    plugin_add(p8(MATERIAL_REGISTRY), size, id, ctor, dtor, copy)
}

/// `RpMaterialRegisterPluginStream(id, read, write, getSize)` (0x74DC20).
#[hook(addr = 0x74DC20, conv = cdecl, name = "RpMaterialRegisterPluginStream", expect = "8B 44 24 10 8B 4C 24 0C")]
pub fn rp_material_register_plugin_stream(id: u32, read: u32, write: u32, get_size: u32) -> i32 {
    plugin_add_stream(p8(MATERIAL_REGISTRY), id, read, write, get_size)
}

/// The 0x2C-byte local frame of `RpMaterialStreamRead`, in the original's order, so the unclamped
/// struct-chunk read (`len` bytes into `s`) lands as in the original within the frame.
#[repr(C)]
struct MaterialReadFrame {
    version: u32,
    len: u32,
    /// the `RwError` of the version error
    err: [u32; 2],
    /// {flags, color, unused, textured, ambient, specular, diffuse}
    s: [u32; 7],
}

/// `RpMaterialStreamRead(stream)` (0x74DD30): the struct chunk (version in [0x34000, 0x36003], else
/// error {2, E_RW_BADVERSION}) read unclamped into a zeroed 0x1C-byte struct; a new material takes its
/// colour and surface properties. A textured material needs a texture chunk (missing -> destroyed,
/// NULL; bad version -> destroyed, error, NULL); a NULL `RwTextureStreamRead` is accepted. Then the
/// plugin data chunks (failure -> destroyed, NULL) and the rights callback when one was read.
/// Original bug kept: `len` > 0x1C overruns the struct (past the frame the original overwrites its
/// return address, the port its own stack).
#[hook(addr = 0x74DD30, conv = cdecl, name = "RpMaterialStreamRead", expect = "83 EC 2C 8D 44 24 00")]
pub fn rp_material_stream_read(stream: GPtr<u8>) -> GPtr<u8> {
    let mut f = MaterialReadFrame {
        version: 0,
        len: 0,
        err: [0; 2],
        s: [0; 7],
    };
    let fp: GPtr<u8> = GPtr::from_raw(&mut f).cast();
    let ver_p = fp.cast::<u32>();
    let len_p = fp.byte_add(4).cast::<u32>();
    let sp = fp.byte_add(0x10);
    if rw_stream_find_chunk_call(stream, 1, len_p, ver_p) == 0 {
        return GPtr::NULL;
    }
    let v = ver_p.read();
    if !(0x34000..=0x36003).contains(&v) {
        error1_p2(E_RW_BADVERSION);
        return GPtr::NULL;
    }
    for i in 0..7 {
        wr::<u32>(sp, i * 4, 0);
    }
    if rw_stream_read_call(stream, sp, len_p.read()) != len_p.read() {
        return GPtr::NULL;
    }
    let m = material_create();
    if m.is_null() {
        return GPtr::NULL;
    }
    wr::<u32>(m, 4, rd::<u32>(sp, 4));
    wr::<u32>(m, 0xC, rd::<u32>(sp, 0x10));
    wr::<u32>(m, 0x10, rd::<u32>(sp, 0x14));
    wr::<u32>(m, 0x14, rd::<u32>(sp, 0x18));
    wr::<u32>(m, 0, 0);
    if rd::<u32>(sp, 0xC) != 0 {
        if rw_stream_find_chunk_call(stream, 6, GPtr::NULL, ver_p) == 0 {
            material_destroy(m);
            return GPtr::NULL;
        }
        let v = ver_p.read();
        if !(0x34000..=0x36003).contains(&v) {
            material_destroy(m);
            error1_p2(E_RW_BADVERSION);
            return GPtr::NULL;
        }
        let t = rw_texture_stream_read_call(stream);
        wr::<u32>(m, 0, t.addr() as u32);
    }
    g::<u32>(RIGHTS_PLUGIN_ID).write(0);
    g::<u32>(RIGHTS_EXTRA_DATA).write(0);
    if plugin_read_data_chunks(p8(MATERIAL_REGISTRY), stream, m).is_null() {
        material_destroy(m);
        return GPtr::NULL;
    }
    let id = g::<u32>(RIGHTS_PLUGIN_ID).read();
    if id != 0 {
        plugin_invoke_rights(p8(MATERIAL_REGISTRY), id, m, g::<u32>(RIGHTS_EXTRA_DATA).read());
    }
    m
}

/// `_rpMaterialListDeinitialize(list)` (0x74E150): destroys and clears every material (count read
/// once), frees the array; count and space 0. Returns list.
#[hook(addr = 0x74E150, conv = cdecl, name = "_rpMaterialListDeinitialize", expect = "55 8B 6C 24 08")]
pub fn material_list_deinitialize(l: GPtr<u8>) -> GPtr<u8> {
    let mats = p8(rd::<u32>(l, 0));
    if !mats.is_null() {
        let n = rd::<i32>(l, 4);
        let mut i = 0;
        while i < n {
            rp_material_destroy_call(p8(rd::<u32>(mats, i as isize * 4)));
            wr::<u32>(mats, i as isize * 4, 0);
            i += 1;
        }
        rw_free(mats);
        wr::<u32>(l, 0, 0);
    }
    wr::<u32>(l, 4, 0);
    wr::<u32>(l, 8, 0);
    l
}

/// `_rpMaterialListInitialize(list)` (0x74E1B0).
#[hook(addr = 0x74E1B0, conv = cdecl, name = "_rpMaterialListInitialize", expect = "8B 44 24 04 33 C9")]
pub fn material_list_initialize(l: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(l, 8, 0);
    wr::<u32>(l, 0, 0);
    wr::<u32>(l, 4, 0);
    l
}

/// `_rpMaterialListGetMaterial(list, index)` (0x74E2B0): no bounds check.
#[hook(addr = 0x74E2B0, conv = cdecl, name = "_rpMaterialListGetMaterial", expect = "8B 44 24 04 8B 54 24 08")]
pub fn material_list_get_material(l: GPtr<u8>, index: i32) -> GPtr<u8> {
    p8(p8(rd::<u32>(l, 0)).cast::<u32>().wrapping_index(index).read())
}

/// `_rpMaterialListAppendMaterial(list, material)` (0x74E350): grows by 20 entries when full
/// (`space*4 + 0x50` bytes, hint 0x1030008; failure -> error {2, E_RW_NOMEM(size)}, -1), stores the
/// material, refCount++ (16-bit), returns its index.
#[hook(addr = 0x74E350, conv = cdecl, name = "_rpMaterialListAppendMaterial", expect = "83 EC 08 56 8B 74 24 10")]
pub fn material_list_append_material(l: GPtr<u8>, m: GPtr<u8>) -> i32 {
    let space = rd::<i32>(l, 8);
    let num = rd::<i32>(l, 4);
    if space <= num {
        let size = (space as u32).wrapping_mul(4).wrapping_add(0x50);
        let old = p8(rd::<u32>(l, 0));
        let p = if old.is_null() {
            rw_malloc(size, 0x0103_0008)
        } else {
            rw_realloc(old, size, 0x0103_0008)
        };
        if p.is_null() {
            error2_p2(E_RW_NOMEM, size);
            return -1;
        }
        let space = rd::<i32>(l, 8);
        wr::<u32>(l, 0, p.addr() as u32);
        wr::<i32>(l, 8, space.wrapping_add(0x14));
    }
    let num = rd::<i32>(l, 4);
    p8(rd::<u32>(l, 0))
        .cast::<u32>()
        .wrapping_index(num)
        .write(m.addr() as u32);
    wr::<i16>(m, 0x18, rd::<i16>(m, 0x18).wrapping_add(1));
    let num = rd::<i32>(l, 4).wrapping_add(1);
    wr::<i32>(l, 4, num);
    num.wrapping_sub(1)
}

/// `_rpMaterialListFindMaterialIndex(list, material)` (0x74E420): the highest index holding `material`,
/// else -1 (`numMaterials - 1` when the count is negative).
#[hook(addr = 0x74E420, conv = cdecl, name = "_rpMaterialListFindMaterialIndex", expect = "8B 4C 24 04 56")]
pub fn material_list_find_material_index(l: GPtr<u8>, m: GPtr<u8>) -> i32 {
    let mut i = rd::<i32>(l, 4);
    if i > 0 {
        let mats = p8(rd::<u32>(l, 0)).cast::<u32>();
        loop {
            i -= 1;
            if mats.wrapping_index(i).read() == m.addr() as u32 {
                return i;
            }
            if i <= 0 {
                break;
            }
        }
    }
    i.wrapping_sub(1)
}

/// Cleanup "C" of `_rpMaterialListStreamRead`: destroys every listed material (count read once) and
/// frees the array when there is one; count and space 0.
fn list_cleanup(l: GPtr<u8>) {
    let mats = p8(rd::<u32>(l, 0));
    if !mats.is_null() {
        let n = rd::<i32>(l, 4);
        let mut i = 0;
        while i < n {
            rp_material_destroy_call(p8(rd::<u32>(mats, i as isize * 4)));
            wr::<u32>(mats, i as isize * 4, 0);
            i += 1;
        }
        rw_free(mats);
        wr::<u32>(l, 0, 0);
    }
    wr::<u32>(l, 4, 0);
    wr::<u32>(l, 8, 0);
}

/// `_rpMaterialListStreamRead(stream, list)` (0x74E600): the struct chunk (version checked, error
/// {2, E_RW_BADVERSION}) gives the count `n` and `n` indices (array allocated with hint 0x10501, NOT
/// checked). Index >= 0 shares an earlier material (refCount++, unchecked index); otherwise a material
/// chunk is read. Each material is appended and its extra reference dropped. Any failure frees the
/// index array, destroys the list and returns NULL.
#[hook(addr = 0x74E600, conv = cdecl, name = "_rpMaterialListStreamRead", expect = "83 EC 14 8D 44 24 04")]
pub fn material_list_stream_read(stream: GPtr<u8>, l: GPtr<u8>) -> GPtr<u8> {
    let (mut n, mut ver, mut len) = (0i32, 0u32, 0u32);
    let (np, vp, lp) = (
        GPtr::from_raw(&mut n),
        GPtr::from_raw(&mut ver),
        GPtr::from_raw(&mut len),
    );
    if rw_stream_find_chunk_call(stream, 1, lp, vp) == 0 {
        return GPtr::NULL;
    }
    let v = vp.read();
    if !(0x34000..=0x36003).contains(&v) {
        error1_p2(E_RW_BADVERSION);
        return GPtr::NULL;
    }
    if rw_stream_read_int32_call(stream, np.cast(), 4).is_null() {
        return GPtr::NULL;
    }
    wr::<u32>(l, 8, 0);
    wr::<u32>(l, 0, 0);
    wr::<u32>(l, 4, 0);
    let count = np.read();
    if count == 0 {
        return l;
    }
    if count > 0 {
        let size = (count as u32).wrapping_mul(4);
        let p = rw_malloc(size, 0x0103_0008);
        if p.is_null() {
            error2_p2(E_RW_NOMEM, size);
            list_cleanup(l);
            return GPtr::NULL;
        }
        wr::<u32>(l, 0, p.addr() as u32);
        wr::<i32>(l, 8, count);
    }
    let idx = rw_malloc((np.read() as u32).wrapping_mul(4), 0x10501);
    if rw_stream_read_int32_call(stream, idx, (np.read() as u32).wrapping_mul(4)).is_null() {
        rw_free(idx);
        list_cleanup(l);
        return GPtr::NULL;
    }
    let mut i: i32 = 0;
    if np.read() > 0 {
        loop {
            let k = idx.cast::<i32>().wrapping_index(i).read();
            let m = if k >= 0 {
                let m = p8(p8(rd::<u32>(l, 0)).cast::<u32>().wrapping_index(k).read());
                wr::<i16>(m, 0x18, rd::<i16>(m, 0x18).wrapping_add(1));
                m
            } else {
                if rw_stream_find_chunk_call(stream, 7, GPtr::NULL, vp) == 0 {
                    rw_free(idx);
                    list_cleanup(l);
                    return GPtr::NULL;
                }
                let v = vp.read();
                if !(0x34000..=0x36003).contains(&v) {
                    error1_p2(E_RW_BADVERSION);
                    rw_free(idx);
                    list_cleanup(l);
                    return GPtr::NULL;
                }
                let m = rp_material_stream_read_call(stream);
                if m.is_null() {
                    rw_free(idx);
                    list_cleanup(l);
                    return GPtr::NULL;
                }
                m
            };
            material_list_append_call(l, m);
            rp_material_destroy_call(m);
            i += 1;
            if i >= np.read() {
                break;
            }
        }
    }
    rw_free(idx);
    l
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_MATERIAL_MODULE_OPEN,
    HOOK_MATERIAL_MODULE_CLOSE,
    HOOK_RP_MATERIAL_CREATE,
    HOOK_RP_MATERIAL_DESTROY,
    HOOK_RP_MATERIAL_CLONE,
    HOOK_RP_MATERIAL_SET_TEXTURE,
    HOOK_RP_MATERIAL_REGISTER_PLUGIN,
    HOOK_RP_MATERIAL_REGISTER_PLUGIN_STREAM,
    HOOK_RP_MATERIAL_STREAM_READ,
    HOOK_MATERIAL_LIST_DEINITIALIZE,
    HOOK_MATERIAL_LIST_INITIALIZE,
    HOOK_MATERIAL_LIST_GET_MATERIAL,
    HOOK_MATERIAL_LIST_APPEND_MATERIAL,
    HOOK_MATERIAL_LIST_FIND_MATERIAL_INDEX,
    HOOK_MATERIAL_LIST_STREAM_READ,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
