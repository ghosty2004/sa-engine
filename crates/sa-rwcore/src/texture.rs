//! TEXTUREMODULE: `RwTexture` / `RwTexDictionary` (PORT-RW-1). Ported from the 1.0 US bytes;
//! `docs/re/specs/RW/RwTexture.md` is the reading aid.
//!
//! Module `M = G + [0xC97B4C]` (0x34): +0 RwLLLink head of all dictionaries, +8 texture freelist,
//! +0xC dictionary freelist, +0x10 current dictionary, +0x14 read callback, +0x18 find callback,
//! +0x1C mipmapping, +0x20 auto mipmapping, +0x24 dword / +0x28 word cleared at open, +0x2C mipmap
//! generate callback, +0x30 mipmap name callback.
//!
//! `RwTexDictionary` (0x18 + plugins): +0 RwObject (type 6), +8 RwLinkList texturesInDict, +0x10
//! RwLLLink lInInstance. `RwTexture` (0x58 + plugins): +0 raster, +4 dict, +8 RwLLLink lInDictionary,
//! +0x10 name[32], +0x30 mask[32], +0x50 filterAddressing, +0x54 refCount.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{
    RwError, error1, g, llink, module, p8, rd, rw_error_set, rw_memory_alloc, rw_memory_free, wr,
};
use crate::stream::rw_free_list_create_and_preallocate;

/// TEXTUREMODULE offset global.
pub const TEXTURE_MODULE: usize = 0xC97B4C;
/// Module open count.
const TEXTURE_INSTANCES: usize = 0xC97B50;
/// The dictionary created at open.
const DEFAULT_DICTIONARY: usize = 0xC97B78;
/// Static freelist structs (textures, dictionaries).
const TEXTURE_FL_STATIC: u32 = 0xC97B54;
const DICT_FL_STATIC: u32 = 0xC97B28;
/// Plugin registries (`sizeOfStruct` at +0).
pub const TEXTURE_REGISTRY: u32 = 0x8E23CC;
pub const DICT_REGISTRY: u32 = 0x8E23E4;
/// Freelist parameters in `.data` (0x80, 1, 5, 1).
const TEXTURE_FL_PER_BLOCK: usize = 0x8E23FC;
const TEXTURE_FL_PREALLOC: usize = 0x8E2400;
const DICT_FL_PER_BLOCK: usize = 0x8E2404;
const DICT_FL_PREALLOC: usize = 0x8E2408;
/// "(null)"
const STR_NULL: u32 = 0x8E240C;
/// Default callbacks installed at open.
const DEFAULT_FIND_CB: u32 = 0x7F53D0;
const DEFAULT_READ_CB: u32 = 0x7F40F0;
const DEFAULT_MIPMAP_GENERATE_CB: u32 = 0x7F5140;
const DEFAULT_MIPMAP_NAME_CB: u32 = 0x7F4080;

const E_RW_BADVERSION: u32 = 0x8000_0004;
/// E_RW_TEXTURENOTFOUND-style core code raised by `RwTextureRead`.
const E_READ_FAILED: u32 = 0x16;
/// E_RW_STRING_TRUNCATION (name longer than 31 chars).
const E_RW_TRUNCATED: u32 = 0x8000_001E;

orig! {
    /// `_rwerror(code, a, b)` (0x8088D0, varargs; three arguments)
    fn rw_error3(code: u32, a: u32, b: u32) -> u32 = cdecl 0x8088D0 => unported;
    /// `_rwerror(code, a, b, c, d)` (0x8088D0, varargs; five arguments)
    fn rw_error5(code: u32, a: u32, b: u32, c: u32, d: i32) -> u32 = cdecl 0x8088D0 => unported;
    /// `RwFreeListDestroy(fl)` (0x801B80)
    fn rw_free_list_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => unported;
    /// `_rwPluginRegistryInitObject(reg, obj)` (0x8086E0)
    fn plugin_init_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x8086E0 => crate::plugin::registry_init_object;
    /// `_rwPluginRegistryDeInitObject(reg, obj)` (0x808740)
    fn plugin_deinit_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808740 => crate::plugin::registry_deinit_object;
    /// `_rwPluginRegistryAddPlugin(reg, size, id, ctor, dtor, copy)` (0x8084A0)
    fn plugin_add(reg: GPtr<u8>, size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x8084A0 => crate::plugin::registry_add_plugin;
    /// `_rwPluginRegistryGetPluginOffset(reg, id)` (0x808470)
    fn plugin_get_offset(reg: GPtr<u8>, id: u32) -> i32 = cdecl 0x808470 => crate::plugin::registry_get_plugin_offset;
    /// `_rwPluginRegistryAddPluginStream(reg, id, read, write, getSize)` (0x8088E0)
    fn plugin_add_stream(reg: GPtr<u8>, id: u32, read: u32, write: u32, get_size: u32) -> i32 = cdecl 0x8088E0 => crate::plugin::registry_add_plugin_stream;
    /// `_rwPluginRegistryReadDataChunks(reg, stream, obj)` (0x808980)
    fn plugin_read_data_chunks(reg: GPtr<u8>, s: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808980 => crate::plugin::registry_read_data_chunks;
    /// `_rwPluginRegistrySkipDataChunks(reg, stream)` (0x808C10)
    fn plugin_skip_data_chunks(reg: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> = cdecl 0x808C10 => crate::plugin::registry_skip_data_chunks;
    /// `RwRasterDestroy(raster)` (0x7FB020)
    fn rw_raster_destroy(r: GPtr<u8>) -> u32 = cdecl 0x7FB020 => unported;
    /// `G->stringFuncs.rwstrncpy(dst, src, n)` (G+0xFC)
    fn rw_strncpy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl table 0xC97B24 0xFC => unported;
    /// `G->stringFuncs.rwstrlen(s)` (G+0x120)
    fn rw_strlen(s: GPtr<u8>) -> u32 = cdecl table 0xC97B24 0x120 => unported;
    /// `G->stdFunc[8]` (+0x68, rwSTANDARDTEXTURESETRASTER)`(texture, raster, 0)`
    fn std_texture_set_raster(t: GPtr<u8>, r: GPtr<u8>, n: u32) -> u32 = cdecl table 0xC97B24 0x68 => unported;
    /// The find callback `M+0x18 (name)` (stored pointer)
    fn call_find_cb(f: u32, name: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
    /// The read callback `M+0x14 (name, mask)` (stored pointer)
    fn call_read_cb(f: u32, name: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
    /// A texture callback `cb(texture, data)` (`RwTexDictionaryForAllTextures`)
    fn call_texture_cb(f: u32, t: GPtr<u8>, data: u32) -> GPtr<u8> = cdecl dyn => unported;
    /// A dictionary callback `cb(dict, data)` (`RwTexDictionaryForAllTexDictionaries`)
    fn call_dict_cb(f: u32, d: GPtr<u8>, data: u32) -> GPtr<u8> = cdecl dyn => unported;

    /// `RwTextureDestroy(texture)` as `RwTexDictionaryDestroy` calls it (an extra 0 argument pushed)
    fn rw_texture_destroy_call(t: GPtr<u8>, extra: u32) -> u32 = cdecl 0x7F3820 => texture_destroy_extra;
    /// `RwTexDictionaryCreate()` (0x7F3600)
    fn rw_tex_dictionary_create_call() -> GPtr<u8> = cdecl 0x7F3600 => rw_tex_dictionary_create;
    /// `RwTextureGetMipmapping()` (0x7F3550)
    fn get_mipmapping_call() -> u32 = cdecl 0x7F3550 => rw_texture_get_mipmapping;
    /// `RwTextureSetMipmapping(enable)` (0x7F3530)
    fn set_mipmapping_call(v: u32) -> u32 = cdecl 0x7F3530 => rw_texture_set_mipmapping;
    /// `RwTextureGetAutoMipmapping()` (0x7F3580)
    fn get_auto_mipmapping_call() -> u32 = cdecl 0x7F3580 => rw_texture_get_auto_mipmapping;
    /// `RwTextureSetAutoMipmapping(enable)` (0x7F3560)
    fn set_auto_mipmapping_call(v: u32) -> u32 = cdecl 0x7F3560 => rw_texture_set_auto_mipmapping;
    /// `RwStreamFindChunk(stream, type, length, version)` (0x7ED2D0)
    fn rw_stream_find_chunk_call(s: GPtr<u8>, ty: u32, len: GPtr<u32>, ver: GPtr<u32>) -> u32 = cdecl 0x7ED2D0 => crate::stream::rw_stream_find_chunk;
    /// `RwStreamRead(stream, buf, len)` (0x7EC9D0)
    fn rw_stream_read_call(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => crate::stream::rw_stream_read;
    /// `_rwStringStreamFindAndRead(string, stream)` (0x80A510)
    fn string_find_and_read_call(string: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> = cdecl 0x80A510 => crate::stream::string_stream_find_and_read;
    /// `RwTextureRead(name, mask)` (0x7F3AC0)
    fn rw_texture_read_call(name: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3AC0 => rw_texture_read;
}

/// The phase B/C path of `rw_texture_destroy_call` (the extra argument is ignored).
#[cfg_attr(sa_phase_a, allow(dead_code))]
fn texture_destroy_extra(t: GPtr<u8>, _extra: u32) -> u32 {
    rw_texture_destroy(t)
}

/// `M = G + [0xC97B4C]`.
fn tmod() -> GPtr<u8> {
    module(TEXTURE_MODULE)
}

fn rw_error_with(code: u32) {
    let mut e = RwError {
        plugin_id: 1,
        error_code: 0,
    };
    e.error_code = code as i32;
    rw_error_set(GPtr::from_raw(&mut e));
}

/// `RwTextureSetFindCallBack(cb)` (0x7F34D0).
#[hook(addr = 0x7F34D0, conv = cdecl, name = "RwTextureSetFindCallBack", expect = "8B 0D 24 7B C9 00")]
pub fn rw_texture_set_find_callback(cb: u32) -> u32 {
    wr::<u32>(tmod(), 0x18, cb);
    1
}

/// `RwTextureGetFindCallBack()` (0x7F34F0).
#[hook(addr = 0x7F34F0, conv = cdecl, name = "RwTextureGetFindCallBack", expect = "A1 24 7B C9 00")]
pub fn rw_texture_get_find_callback() -> u32 {
    rd::<u32>(tmod(), 0x18)
}

/// `RwTextureSetReadCallBack(cb)` (0x7F3500).
#[hook(addr = 0x7F3500, conv = cdecl, name = "RwTextureSetReadCallBack", expect = "8B 0D 24 7B C9 00")]
pub fn rw_texture_set_read_callback(cb: u32) -> u32 {
    wr::<u32>(tmod(), 0x14, cb);
    1
}

/// `RwTextureGetReadCallBack()` (0x7F3520).
#[hook(addr = 0x7F3520, conv = cdecl, name = "RwTextureGetReadCallBack", expect = "A1 24 7B C9 00")]
pub fn rw_texture_get_read_callback() -> u32 {
    rd::<u32>(tmod(), 0x14)
}

/// `RwTextureSetMipmapping(enable)` (0x7F3530): stores the whole dword.
#[hook(addr = 0x7F3530, conv = cdecl, name = "RwTextureSetMipmapping", expect = "8B 0D 24 7B C9 00")]
pub fn rw_texture_set_mipmapping(v: u32) -> u32 {
    wr::<u32>(tmod(), 0x1C, v);
    1
}

/// `RwTextureGetMipmapping()` (0x7F3550).
#[hook(addr = 0x7F3550, conv = cdecl, name = "RwTextureGetMipmapping", expect = "A1 24 7B C9 00")]
pub fn rw_texture_get_mipmapping() -> u32 {
    rd::<u32>(tmod(), 0x1C)
}

/// `RwTextureSetAutoMipmapping(enable)` (0x7F3560).
#[hook(addr = 0x7F3560, conv = cdecl, name = "RwTextureSetAutoMipmapping", expect = "8B 0D 24 7B C9 00")]
pub fn rw_texture_set_auto_mipmapping(v: u32) -> u32 {
    wr::<u32>(tmod(), 0x20, v);
    1
}

/// `RwTextureGetAutoMipmapping()` (0x7F3580).
#[hook(addr = 0x7F3580, conv = cdecl, name = "RwTextureGetAutoMipmapping", expect = "A1 24 7B C9 00")]
pub fn rw_texture_get_auto_mipmapping() -> u32 {
    rd::<u32>(tmod(), 0x20)
}

/// `_rwTextureSetAutoMipMapState(enable)` (0x7F3590): mipmapping and auto mipmapping both = enable.
#[hook(addr = 0x7F3590, conv = cdecl, name = "_rwTextureSetAutoMipMapState", expect = "8B 0D 4C 7B C9 00")]
pub fn rw_texture_set_auto_mipmap_state(v: u32) -> u32 {
    wr::<u32>(tmod(), 0x1C, v);
    wr::<u32>(tmod(), 0x20, v);
    1
}

/// `_rwTextureGetAutoMipMapState()` (0x7F35C0): the auto mipmapping flag.
#[hook(addr = 0x7F35C0, conv = cdecl, name = "_rwTextureGetAutoMipMapState", expect = "A1 24 7B C9 00")]
pub fn rw_texture_get_auto_mipmap_state() -> u32 {
    rd::<u32>(tmod(), 0x20)
}

/// `RwTextureSetRaster(texture, raster)` (0x7F35D0): a NULL raster clears `t->raster` and returns t;
/// otherwise the driver's `stdFunc[rwSTANDARDTEXTURESETRASTER](t, raster, 0)` decides (t or NULL).
#[hook(addr = 0x7F35D0, conv = cdecl, name = "RwTextureSetRaster", expect = "8B 44 24 08 85 C0")]
pub fn rw_texture_set_raster(t: GPtr<u8>, raster: GPtr<u8>) -> GPtr<u8> {
    if raster.is_null() {
        wr::<u32>(t, 0, 0);
        return t;
    }
    if std_texture_set_raster(t, raster, 0) != 0 {
        t
    } else {
        GPtr::NULL
    }
}

/// `RwTexDictionaryCreate()` (0x7F3600): from the dictionary freelist (hint 0x30016; NULL -> NULL, no
/// error); RwObject type 6, linked at the head of the module list, empty texture list, plugin init.
#[hook(addr = 0x7F3600, conv = cdecl, name = "RwTexDictionaryCreate", expect = "A1 24 7B C9 00")]
pub fn rw_tex_dictionary_create() -> GPtr<u8> {
    let d = rw_memory_alloc(p8(rd::<u32>(tmod(), 0xC)), 0x30016);
    if d.is_null() {
        return GPtr::NULL;
    }
    wr::<u8>(d, 0, 6);
    wr::<u8>(d, 1, 0);
    wr::<u8>(d, 2, 0);
    wr::<u8>(d, 3, 0);
    wr::<u32>(d, 4, 0);
    llink::add_head(tmod(), d.byte_add(0x10));
    let list = d.byte_add(8);
    wr::<u32>(d, 0xC, list.addr() as u32);
    wr::<u32>(d, 8, list.addr() as u32);
    plugin_init_object(p8(DICT_REGISTRY), d);
    d
}

/// `RwTexDictionaryDestroy(dict)` (0x7F36A0): clears the current dictionary if it is `dict`, destroys
/// every texture (next read first; the walk stops if `RwTextureDestroy` returns 0), plugin deinit,
/// unlinks and frees. Textures still referenced elsewhere stay linked into the freed dictionary
/// (original behaviour). Returns 1.
#[hook(addr = 0x7F36A0, conv = cdecl, name = "RwTexDictionaryDestroy", expect = "A1 4C 7B C9 00")]
pub fn rw_tex_dictionary_destroy(d: GPtr<u8>) -> u32 {
    if rd::<u32>(tmod(), 0x10) == d.addr() as u32 {
        wr::<u32>(tmod(), 0x10, 0);
    }
    let head = d.byte_add(8);
    let mut l = llink::next(head);
    if l != head {
        loop {
            let next = llink::next(l);
            if rw_texture_destroy_call(l.byte_add(-8), 0) == 0 {
                break;
            }
            l = next;
            if next == head {
                break;
            }
        }
    }
    plugin_deinit_object(p8(DICT_REGISTRY), d);
    llink::remove(d.byte_add(0x10));
    rw_memory_free(p8(rd::<u32>(tmod(), 0xC)), d);
    1
}

/// `RwTexDictionaryForAllTextures(dict, cb, data)` (0x7F3730): `cb(texture, data)` from the head (next
/// read before the call) until it returns NULL. Returns dict.
#[hook(addr = 0x7F3730, conv = cdecl, name = "RwTexDictionaryForAllTextures", expect = "8B 4C 24 04 53")]
pub fn rw_tex_dictionary_for_all_textures(d: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> {
    let head = d.byte_add(8);
    let mut l = llink::next(head);
    while l != head {
        let next = llink::next(l);
        if call_texture_cb(cb, l.byte_add(-8), data).is_null() {
            break;
        }
        l = next;
    }
    d
}

/// `RwTexDictionaryForAllTexDictionaries(cb, data)` (0x7F3770): `cb(dict, data)` over the module list
/// (link at dict+0x10, next read first) until it returns NULL. Returns 1.
#[hook(addr = 0x7F3770, conv = cdecl, name = "RwTexDictionaryForAllTexDictionaries", expect = "A1 24 7B C9 00")]
pub fn rw_tex_dictionary_for_all_tex_dictionaries(cb: u32, data: u32) -> u32 {
    let head = tmod();
    let mut l = llink::next(head);
    while l != head {
        let next = llink::next(l);
        if call_dict_cb(cb, l.byte_add(-0x10), data).is_null() {
            break;
        }
        l = next;
    }
    1
}

/// `RwTextureCreate(raster)` (0x7F37C0): from the texture freelist (hint 0x30006; NULL -> NULL);
/// filterAddressing 0x1101, no dict, empty name and mask, refCount 1 (lInDictionary not initialised),
/// plugin init.
#[hook(addr = 0x7F37C0, conv = cdecl, name = "RwTextureCreate", expect = "A1 24 7B C9 00")]
pub fn rw_texture_create(raster: GPtr<u8>) -> GPtr<u8> {
    let t = rw_memory_alloc(p8(rd::<u32>(tmod(), 8)), 0x30006);
    if t.is_null() {
        return GPtr::NULL;
    }
    wr::<u32>(t, 0x50, 0);
    wr::<u32>(t, 0, raster.addr() as u32);
    wr::<u8>(t, 0x51, 0x11);
    wr::<u32>(t, 4, 0);
    wr::<u8>(t, 0x10, 0);
    wr::<u8>(t, 0x30, 0);
    wr::<u32>(t, 0x54, 1);
    wr::<u8>(t, 0x50, 1);
    plugin_init_object(p8(TEXTURE_REGISTRY), t);
    t
}

/// `RwTextureDestroy(texture)` (0x7F3820): decrements refCount; still positive -> 1. Otherwise refCount
/// is put back to its entry value around the plugin deinit, the texture is unlinked from its
/// dictionary (`dict` kept), its raster destroyed and cleared, refCount decremented again and the
/// texture freed. Returns 1.
#[hook(addr = 0x7F3820, conv = cdecl, name = "RwTextureDestroy", expect = "56 8B 74 24 08")]
pub fn rw_texture_destroy(t: GPtr<u8>) -> u32 {
    let rc = rd::<i32>(t, 0x54).wrapping_sub(1);
    wr::<i32>(t, 0x54, rc);
    if rc > 0 {
        return 1;
    }
    wr::<i32>(t, 0x54, rc.wrapping_add(1));
    plugin_deinit_object(p8(TEXTURE_REGISTRY), t);
    if rd::<u32>(t, 4) != 0 {
        llink::remove(t.byte_add(8));
    }
    let r = rd::<u32>(t, 0);
    if r != 0 {
        rw_raster_destroy(p8(r));
        wr::<u32>(t, 0, 0);
    }
    wr::<i32>(t, 0x54, rd::<i32>(t, 0x54).wrapping_sub(1));
    rw_memory_free(p8(rd::<u32>(tmod(), 8)), t);
    1
}

/// The body of `RwTextureSetName` / `RwTextureSetMaskName` on the 32-byte field at `off`.
fn set_name_at(t: GPtr<u8>, name: GPtr<u8>, off: isize) -> GPtr<u8> {
    rw_strncpy(t.byte_add(off), name, 0x20);
    if rw_strlen(name) >= 0x20 {
        let c = rd::<i8>(name, 0x1F) as i32;
        rw_error_with(rw_error5(E_RW_TRUNCATED, name.addr() as u32, 0x20, 0x1F, c));
        wr::<u8>(t, off + 0x1F, 0);
    }
    t
}

/// `RwTextureSetName(texture, name)` (0x7F38A0): `rwstrncpy(name, 32)`; a name of 32+ chars raises
/// E_RW_TRUNCATED (name, 0x20, 0x1F, `(signed char)name[0x1F]`) and terminates at [0x1F].
#[hook(addr = 0x7F38A0, conv = cdecl, name = "RwTextureSetName", expect = "83 EC 08 8B 0D 24 7B C9 00")]
pub fn rw_texture_set_name(t: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> {
    set_name_at(t, name, 0x10)
}

/// `RwTextureSetMaskName(texture, mask)` (0x7F3910): as `RwTextureSetName` on +0x30.
#[hook(addr = 0x7F3910, conv = cdecl, name = "RwTextureSetMaskName", expect = "83 EC 08 8B 0D 24 7B C9 00")]
pub fn rw_texture_set_mask_name(t: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> {
    set_name_at(t, mask, 0x30)
}

/// Unlinks `t` from its dictionary if it has one, sets `t->dict = d` and inserts at the head.
fn add_texture(d: GPtr<u8>, t: GPtr<u8>) {
    if rd::<u32>(t, 4) != 0 {
        llink::remove(t.byte_add(8));
    }
    wr::<u32>(t, 4, d.addr() as u32);
    llink::add_head(d.byte_add(8), t.byte_add(8));
}

/// `RwTexDictionaryAddTexture(dict, texture)` (0x7F3980): no duplicate check. Returns texture.
#[hook(addr = 0x7F3980, conv = cdecl, name = "RwTexDictionaryAddTexture", expect = "8B 44 24 08 8B 48 04")]
pub fn rw_tex_dictionary_add_texture(d: GPtr<u8>, t: GPtr<u8>) -> GPtr<u8> {
    add_texture(d, t);
    t
}

/// `RwTexDictionaryRemoveTexture(texture)` (0x7F39C0): clears dict and unlinks when set. Returns texture.
#[hook(addr = 0x7F39C0, conv = cdecl, name = "RwTexDictionaryRemoveTexture", expect = "8B 44 24 04 8B 48 04")]
pub fn rw_tex_dictionary_remove_texture(t: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(t, 4) != 0 {
        wr::<u32>(t, 4, 0);
        llink::remove(t.byte_add(8));
    }
    t
}

/// The case-insensitive compare of `RwTexDictionaryFindNamedTexture`: signed chars, only 'a'..'z'
/// folded (+0xE0), up to the first NUL of either; equal when the final raw bytes are equal.
fn names_match(a: GPtr<u8>, b: GPtr<u8>) -> bool {
    let (mut i, mut j) = (a, b);
    let mut cl = i.read() as i8;
    if cl != 0 {
        loop {
            let mut dl = j.read() as i8;
            if dl == 0 {
                break;
            }
            if (0x61..=0x7A).contains(&cl) {
                cl = cl.wrapping_add(-0x20);
            }
            if (0x61..=0x7A).contains(&dl) {
                dl = dl.wrapping_add(-0x20);
            }
            if cl != dl {
                return false;
            }
            cl = i.byte_add(1).read() as i8;
            i = i.byte_add(1);
            j = j.byte_add(1);
            if cl == 0 {
                break;
            }
        }
    }
    i.read() == j.read()
}

/// `RwTexDictionaryFindNamedTexture(dict, name)` (0x7F39F0): the first texture from the head whose
/// name matches case-insensitively (no length limit), else NULL.
#[hook(addr = 0x7F39F0, conv = cdecl, name = "RwTexDictionaryFindNamedTexture", expect = "8B 44 24 04 53")]
pub fn rw_tex_dictionary_find_named_texture(d: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> {
    let head = d.byte_add(8);
    let mut l = llink::next(head);
    while l != head {
        let t = l.byte_add(-8);
        // `t + 0x10 == NULL` is tested by the original (dead for any real texture)
        if t.byte_add(0x10).addr() as u32 != 0 && names_match(t.byte_add(0x10), name) {
            return t;
        }
        l = llink::next(l);
    }
    GPtr::NULL
}

/// `RwTexDictionarySetCurrent(dict)` (0x7F3A70). Returns dict.
#[hook(addr = 0x7F3A70, conv = cdecl, name = "RwTexDictionarySetCurrent", expect = "8B 0D 24 7B C9 00")]
pub fn rw_tex_dictionary_set_current(d: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(tmod(), 0x10, d.addr() as u32);
    d
}

/// `RwTexDictionaryGetCurrent()` (0x7F3A90).
#[hook(addr = 0x7F3A90, conv = cdecl, name = "RwTexDictionaryGetCurrent", expect = "A1 24 7B C9 00")]
pub fn rw_tex_dictionary_get_current() -> GPtr<u8> {
    p8(rd::<u32>(tmod(), 0x10))
}

/// `RwTextureRead(name, mask)` (0x7F3AC0): `findCB(name)` hit -> refCount++ and return it (not moved
/// into the current dictionary). Else `readCB(name, mask)`; NULL -> error 0x16 (name, mask or
/// "(null)"), NULL. A read texture goes into the current dictionary when there is one.
#[hook(addr = 0x7F3AC0, conv = cdecl, name = "RwTextureRead", expect = "A1 24 7B C9 00")]
pub fn rw_texture_read(name: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> {
    let t = call_find_cb(rd::<u32>(tmod(), 0x18), name);
    if !t.is_null() {
        wr::<i32>(t, 0x54, rd::<i32>(t, 0x54).wrapping_add(1));
        return t;
    }
    let t = call_read_cb(rd::<u32>(tmod(), 0x14), name, mask);
    if t.is_null() {
        let m = if mask.is_null() {
            STR_NULL
        } else {
            mask.addr() as u32
        };
        rw_error_with(rw_error3(E_READ_FAILED, name.addr() as u32, m));
        return GPtr::NULL;
    }
    let cur = p8(rd::<u32>(tmod(), 0x10));
    if !cur.is_null() {
        add_texture(cur, t);
    }
    t
}

/// `RwTextureRegisterPlugin(size, id, ctor, dtor, copy)` (0x7F3BB0).
#[hook(addr = 0x7F3BB0, conv = cdecl, name = "RwTextureRegisterPlugin", expect = "8B 44 24 14 8B 4C 24 10")]
pub fn rw_texture_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 {
    plugin_add(p8(TEXTURE_REGISTRY), size, id, ctor, dtor, copy)
}

/// `RwTextureGetPluginOffset(id)` (0x7F3BE0).
#[hook(addr = 0x7F3BE0, conv = cdecl, name = "RwTextureGetPluginOffset", expect = "8B 44 24 04 50")]
pub fn rw_texture_get_plugin_offset(id: u32) -> i32 {
    plugin_get_offset(p8(TEXTURE_REGISTRY), id)
}

/// `RwTexDictionaryRegisterPlugin(size, id, ctor, dtor, copy)` (0x7F3C10).
#[hook(addr = 0x7F3C10, conv = cdecl, name = "RwTexDictionaryRegisterPlugin", expect = "8B 44 24 14 8B 4C 24 10")]
pub fn rw_tex_dictionary_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 {
    plugin_add(p8(DICT_REGISTRY), size, id, ctor, dtor, copy)
}

/// `cb_TEXTUREMODULE_RwEngine_Open(instance, offset, size)` (0x7F3EA0): the texture freelist (entry
/// size from the registry, hint 0x40006), the dictionary freelist (hint 0x40408; failure destroys the
/// texture freelist), the empty dictionary list, the instance count, the default dictionary
/// (`[0xC97B78]` and current; failure destroys both freelists, dictionaries first), then the default
/// state and callbacks. Returns instance, or NULL on failure.
#[hook(addr = 0x7F3EA0, conv = cdecl, name = "cb_TEXTUREMODULE_RwEngine_Open", expect = "8B 44 24 08 8B 0D 00 24 8E 00")]
pub fn texture_module_open(instance: GPtr<u8>, offset: u32, _size: u32) -> GPtr<u8> {
    let prealloc = g::<u32>(TEXTURE_FL_PREALLOC).read();
    let per_block = g::<u32>(TEXTURE_FL_PER_BLOCK).read();
    g::<u32>(TEXTURE_MODULE).write(offset);
    let size = g::<u32>(TEXTURE_REGISTRY as usize).read();
    let fl =
        rw_free_list_create_and_preallocate(size, per_block, 4, prealloc, p8(TEXTURE_FL_STATIC), 0x40006);
    wr::<u32>(tmod(), 8, fl.addr() as u32);
    if rd::<u32>(tmod(), 8) == 0 {
        return GPtr::NULL;
    }
    let prealloc = g::<u32>(DICT_FL_PREALLOC).read();
    let per_block = g::<u32>(DICT_FL_PER_BLOCK).read();
    let size = g::<u32>(DICT_REGISTRY as usize).read();
    let fl = rw_free_list_create_and_preallocate(size, per_block, 4, prealloc, p8(DICT_FL_STATIC), 0x40408);
    wr::<u32>(tmod(), 0xC, fl.addr() as u32);
    let m = tmod();
    if rd::<u32>(m, 0xC) == 0 {
        rw_free_list_destroy(p8(rd::<u32>(m, 8)));
        wr::<u32>(tmod(), 8, 0);
        return GPtr::NULL;
    }
    wr::<u32>(m, 0, m.addr() as u32);
    let m = tmod();
    wr::<u32>(m, 4, m.addr() as u32);
    let c = g::<u32>(TEXTURE_INSTANCES);
    c.write(c.read().wrapping_add(1));
    let d = rw_tex_dictionary_create_call();
    g::<u32>(DEFAULT_DICTIONARY).write(d.addr() as u32);
    wr::<u32>(tmod(), 0x10, d.addr() as u32);
    if rd::<u32>(tmod(), 0x10) == 0 {
        rw_free_list_destroy(p8(rd::<u32>(tmod(), 0xC)));
        wr::<u32>(tmod(), 0xC, 0);
        rw_free_list_destroy(p8(rd::<u32>(tmod(), 8)));
        wr::<u32>(tmod(), 8, 0);
        return GPtr::NULL;
    }
    let m = tmod();
    wr::<u32>(m, 0x1C, 0);
    wr::<u32>(m, 0x20, 0);
    wr::<u32>(m, 0x18, DEFAULT_FIND_CB);
    wr::<u32>(m, 0x14, DEFAULT_READ_CB);
    wr::<u32>(m, 0x2C, DEFAULT_MIPMAP_GENERATE_CB);
    wr::<u32>(m, 0x30, DEFAULT_MIPMAP_NAME_CB);
    wr::<u32>(m, 0x24, 0);
    wr::<u16>(m, 0x28, 0);
    instance
}

/// `RwTextureRegisterPluginStream(id, read, write, getSize)` (0x804550).
#[hook(addr = 0x804550, conv = cdecl, name = "RwTextureRegisterPluginStream", expect = "8B 44 24 10 8B 4C 24 0C")]
pub fn rw_texture_register_plugin_stream(id: u32, read: u32, write: u32, get_size: u32) -> i32 {
    plugin_add_stream(p8(TEXTURE_REGISTRY), id, read, write, get_size)
}

/// The 0x110-byte local frame of `RwTextureStreamRead` in the original's order, so that the unchecked
/// struct-chunk read (`length` bytes into `flags`) overwrites the same neighbours.
#[repr(C)]
struct StreamReadFrame {
    flags: u32,
    /// the `RwError.errorCode` slot of the version error
    err_code: u32,
    length: u32,
    version: u32,
    mask: [u8; 0x80],
    name: [u8; 0x80],
}

/// `RwTextureStreamRead(stream)` (0x8046E0): the struct chunk (version in [0x34000, 0x36003], else
/// E_RW_BADVERSION) gives the flags (addressV := addressU when 0), the mip state for the read (filter
/// modes 3..6 -> mipmapping on, auto unless bit 16), the name and mask strings, `RwTextureRead`
/// (NULL -> plugin chunks skipped), filterAddressing = `flags & 0xFFFF` for a fresh texture (refCount
/// 1), the mip state restored, then the plugin data chunks (failure -> NULL, texture leaked).
/// Original bug kept: `length` is not checked against 4 (the read overwrites the following locals; past
/// the 0x110-byte frame the original corrupts its saved registers, the port its own stack).
#[hook(addr = 0x8046E0, conv = cdecl, name = "RwTextureStreamRead", expect = "81 EC 10 01 00 00")]
pub fn rw_texture_stream_read(s: GPtr<u8>) -> GPtr<u8> {
    let mut f = StreamReadFrame {
        flags: 0,
        err_code: 0,
        length: 0,
        version: 0,
        mask: [0; 0x80],
        name: [0; 0x80],
    };
    let fp: GPtr<u8> = GPtr::from_raw(&mut f).cast();
    let flags_p = fp;
    let length_p = fp.byte_add(8).cast::<u32>();
    let version_p = fp.byte_add(0xC).cast::<u32>();
    let mask_p = fp.byte_add(0x10);
    let name_p = fp.byte_add(0x90);
    if rw_stream_find_chunk_call(s, 1, length_p, version_p) == 0 {
        return GPtr::NULL;
    }
    let v = version_p.read();
    if !(0x34000..=0x36003).contains(&v) {
        error1(E_RW_BADVERSION);
        return GPtr::NULL;
    }
    flags_p.cast::<u32>().write(0);
    let n = rw_stream_read_call(s, flags_p, length_p.read());
    if n != length_p.read() {
        return GPtr::NULL;
    }
    let mut flags = flags_p.cast::<u32>().read();
    let filter = flags & 0xFF;
    if flags & 0xF000 == 0 {
        flags |= (flags & 0xF00) << 4;
        flags_p.cast::<u32>().write(flags);
    }
    let b2 = flags_p.byte_add(2).read();
    let old_mip = get_mipmapping_call();
    let old_auto = get_auto_mipmapping_call();
    if matches!(filter, 3..=6) {
        set_mipmapping_call(1);
        set_auto_mipmapping_call(if b2 & 1 != 0 { 0 } else { 1 });
    } else {
        set_mipmapping_call(0);
        set_auto_mipmapping_call(0);
    }
    if string_find_and_read_call(name_p, s).is_null() || string_find_and_read_call(mask_p, s).is_null() {
        set_mipmapping_call(old_mip);
        set_auto_mipmapping_call(old_auto);
        return GPtr::NULL;
    }
    let t = rw_texture_read_call(name_p, mask_p);
    if t.is_null() {
        plugin_skip_data_chunks(p8(TEXTURE_REGISTRY), s);
        set_mipmapping_call(old_mip);
        set_auto_mipmapping_call(old_auto);
        return GPtr::NULL;
    }
    if rd::<u32>(t, 0x54) == 1 {
        wr::<u32>(t, 0x50, flags_p.cast::<u32>().read() & 0xFFFF);
    }
    set_mipmapping_call(old_mip);
    set_auto_mipmapping_call(old_auto);
    if plugin_read_data_chunks(p8(TEXTURE_REGISTRY), s, t).is_null() {
        GPtr::NULL
    } else {
        t
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RW_TEXTURE_SET_FIND_CALLBACK,
    HOOK_RW_TEXTURE_GET_FIND_CALLBACK,
    HOOK_RW_TEXTURE_SET_READ_CALLBACK,
    HOOK_RW_TEXTURE_GET_READ_CALLBACK,
    HOOK_RW_TEXTURE_SET_MIPMAPPING,
    HOOK_RW_TEXTURE_GET_MIPMAPPING,
    HOOK_RW_TEXTURE_SET_AUTO_MIPMAPPING,
    HOOK_RW_TEXTURE_GET_AUTO_MIPMAPPING,
    HOOK_RW_TEXTURE_SET_AUTO_MIPMAP_STATE,
    HOOK_RW_TEXTURE_GET_AUTO_MIPMAP_STATE,
    HOOK_RW_TEXTURE_SET_RASTER,
    HOOK_RW_TEX_DICTIONARY_CREATE,
    HOOK_RW_TEX_DICTIONARY_DESTROY,
    HOOK_RW_TEX_DICTIONARY_FOR_ALL_TEXTURES,
    HOOK_RW_TEX_DICTIONARY_FOR_ALL_TEX_DICTIONARIES,
    HOOK_RW_TEXTURE_CREATE,
    HOOK_RW_TEXTURE_DESTROY,
    HOOK_RW_TEXTURE_SET_NAME,
    HOOK_RW_TEXTURE_SET_MASK_NAME,
    HOOK_RW_TEX_DICTIONARY_ADD_TEXTURE,
    HOOK_RW_TEX_DICTIONARY_REMOVE_TEXTURE,
    HOOK_RW_TEX_DICTIONARY_FIND_NAMED_TEXTURE,
    HOOK_RW_TEX_DICTIONARY_SET_CURRENT,
    HOOK_RW_TEX_DICTIONARY_GET_CURRENT,
    HOOK_RW_TEXTURE_READ,
    HOOK_RW_TEXTURE_REGISTER_PLUGIN,
    HOOK_RW_TEXTURE_GET_PLUGIN_OFFSET,
    HOOK_RW_TEX_DICTIONARY_REGISTER_PLUGIN,
    HOOK_TEXTURE_MODULE_OPEN,
    HOOK_RW_TEXTURE_REGISTER_PLUGIN_STREAM,
    HOOK_RW_TEXTURE_STREAM_READ,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
