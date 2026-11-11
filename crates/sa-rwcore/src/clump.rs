//! PORT-RW-1 module `clump`: `RpClump` and CLUMPMODULE (see docs/arch/rw-port.md). Ported from the 1.0 US
//! bytes; `docs/re/specs/RW/RpClump.md` as a reading aid.
//!
//! `RpClump` (0x2C base): +0 type (2), +1..+3 subType/flags/privateFlags, +4 root frame, +8 atomicList
//! (links at atomic + 0x40), +0x10 lightList, +0x18 cameraList (links at ext + 4), +0x20 inWorldLink,
//! +0x28 callback. The CLUMP extension of lights / cameras (0xC bytes at `obj + [0xC924DC]` /
//! `obj + [0xC924D8]`): +0 clump, +4 inClumpLink. CLUMPMODULE data `G + [0xC924AC]`: +0 atomic freelist,
//! +4 clump freelist.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::atomic::{atomic_registry, destroy_body as atomic_destroy_body};
use crate::common::{
    RwError, p8, rd, rw_error_set, rw_error1, rw_globals, rw_memory_alloc, rw_memory_free, wr,
};

/// `_rpClumpDefaultCallBack`.
const DEFAULT_CALLBACK: u32 = 0x74A260;

sa_core::globals! {
    list CLUMP_GLOBALS;
    /// The atomic freelist's in-place `RwFreeList`
    pub static ATOMIC_FREELIST_SPACE @ 0xC92484: [u32; 9] = [0; 9];
    /// CLUMPMODULE offset in `RwGlobals`
    pub static CLUMP_MODULE_OFFSET @ 0xC924AC: u32 = 0;
    /// CLUMPMODULE open-instance counter
    pub static CLUMP_OPEN_COUNT @ 0xC924B0: i32 = 0;
    /// The clump freelist's in-place `RwFreeList`
    pub static CLUMP_FREELIST_SPACE @ 0xC924B4: [u32; 9] = [0; 9];
    /// Offset of the CLUMP extension in `RwCamera`
    pub static CAMERA_EXT_OFFSET @ 0xC924D8: i32 = 0;
    /// Offset of the CLUMP extension in `RpLight`
    pub static LIGHT_EXT_OFFSET @ 0xC924DC: i32 = 0;
    /// Clump `RwPluginRegistry`
    pub static CLUMP_REGISTRY @ 0x8D6264: [u32; 6] = [0x2C, 0x2C, 0, 0, 0, 0];
    /// Atomic freelist entries per block, blocks preallocated; clump freelist the same
    pub static FREELIST_PARAMS @ 0x8D627C: [i32; 4] = [0x80, 1, 0x80, 1];
}

orig! {
    /// `RwFreeListCreateAndPreallocateSpace` (0x801B70)
    pub fn free_list_create_and_preallocate_space(size: i32, per_block: i32, align: i32, blocks: i32, space: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl 0x801B70 => unported;
    /// `RwFreeListDestroy` (0x801B80)
    pub fn free_list_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => unported;
    /// `RwCameraRegisterPlugin(size, id, ctor, dtor, copy)` (0x7EE450)
    pub fn camera_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x7EE450 => unported;
    /// `RpLightRegisterPlugin(size, id, ctor, dtor, copy)` (0x751D60)
    pub fn light_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x751D60 => unported;
    /// `RpLightDestroy(light)` (0x7520D0)
    pub fn light_destroy(l: GPtr<u8>) -> i32 = cdecl 0x7520D0 => unported;
    /// `RwCameraDestroy(camera)` (0x7EE4B0)
    pub fn camera_destroy(c: GPtr<u8>) -> i32 = cdecl 0x7EE4B0 => unported;
    /// `atomic->renderCallBack(atomic)` (the frame pushed for `RwFrameGetLTM` is still on the stack as a
    /// second argument)
    pub fn call_render_cb(f: u32, a: GPtr<u8>, frame: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
    /// `cb(object, data)` of the ForAll* iterators
    pub fn call_obj_cb(f: u32, obj: GPtr<u8>, data: u32) -> GPtr<u8> = cdecl dyn => unported;

    /// `RwFrameGetLTM` (0x7F0990)
    pub fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => crate::frame::get_ltm;
    /// `RwFrameDestroyHierarchy` (0x7F08A0)
    pub fn frame_destroy_hierarchy(f: GPtr<u8>) -> u32 = cdecl 0x7F08A0 => crate::frame::destroy_hierarchy;
    /// `_rwPluginRegistryInitObject` (0x8086E0)
    pub fn registry_init_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x8086E0 => crate::plugin::registry_init_object;
    /// `_rwPluginRegistryDeInitObject` (0x808740)
    pub fn registry_deinit_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808740 => crate::plugin::registry_deinit_object;
    /// `_rwPluginRegistryCopyObject` (0x808770)
    pub fn registry_copy_object(reg: GPtr<u8>, dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl 0x808770 => crate::plugin::registry_copy_object;
    /// `_rwFrameCloneAndLinkClones` (0x7EFFB0)
    pub fn frame_clone_and_link_clones(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EFFB0 => crate::frame::clone_and_link_clones;
    /// `_rwFramePurgeClone` (0x7F01A0)
    pub fn frame_purge_clone(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F01A0 => crate::frame::purge_clone;
    /// `_rwObjectHasFrameSetFrame` (0x804EF0)
    pub fn object_set_frame(obj: GPtr<u8>, f: GPtr<u8>) = cdecl 0x804EF0 => crate::frame::object_has_frame_set_frame;
    /// `_rwObjectHasFrameReleaseFrame` (0x804F40)
    pub fn object_release_frame(obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x804F40 => crate::frame::object_has_frame_release_frame;
    /// `RwResourcesFreeResEntry(entry)` (0x807DE0)
    pub fn resources_free_res_entry(e: GPtr<u8>) -> i32 = cdecl 0x807DE0 => unported;
    /// `RpAtomicClone` (0x749E60)
    pub fn atomic_clone(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x749E60 => crate::atomic::clone;
    /// `RpAtomicDestroy` (0x749DC0)
    pub fn atomic_destroy(a: GPtr<u8>) -> u32 = cdecl 0x749DC0 => crate::atomic::destroy;
    /// `RpAtomicSetGeometry` (0x749D40)
    pub fn atomic_set_geometry(a: GPtr<u8>, geom: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x749D40 => crate::atomic::set_geometry;
    /// `RpClumpCreate` (0x74A290)
    pub fn create_call() -> GPtr<u8> = cdecl 0x74A290 => create;
    /// `RpClumpAddAtomic` (0x74A490)
    pub fn add_atomic_call(c: GPtr<u8>, a: GPtr<u8>) -> GPtr<u8> = cdecl 0x74A490 => add_atomic;
    /// `RpClumpRemoveLight` (0x74A520)
    pub fn remove_light_call(c: GPtr<u8>, l: GPtr<u8>) -> GPtr<u8> = cdecl 0x74A520 => remove_light;
    /// `_rpClumpRemoveAndDestroyLight` (0x74A1C0)
    pub fn remove_and_destroy_light_call(l: GPtr<u8>, data: u32) -> GPtr<u8> = cdecl 0x74A1C0 => remove_and_destroy_light;
    /// `RpClumpForAllCameras` (0x749BB0)
    pub fn for_all_cameras_call(c: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749BB0 => for_all_cameras;
    /// `_rwPluginRegistryAddPlugin` (0x8084A0)
    pub fn registry_add_plugin(reg: GPtr<u8>, size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x8084A0 => crate::plugin::registry_add_plugin;
    /// `_rwPluginRegistryAddPluginStream` (0x8088E0)
    pub fn registry_add_plugin_stream(reg: GPtr<u8>, id: u32, read: u32, write: u32, get_size: u32) -> i32 = cdecl 0x8088E0 => crate::plugin::registry_add_plugin_stream;
    /// `_rwPluginRegistryAddPlgnStrmlwysCB` (0x808920)
    pub fn registry_add_always_cb(reg: GPtr<u8>, id: u32, cb: u32) -> i32 = cdecl 0x808920 => crate::plugin::registry_add_plugin_stream_always_cb;
    /// `_rwPluginRegistryGetPluginOffset` (0x808470)
    pub fn registry_get_plugin_offset(reg: GPtr<u8>, id: u32) -> i32 = cdecl 0x808470 => crate::plugin::registry_get_plugin_offset;
}

fn a32(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// The clump registry (`0x8D6264`).
pub fn clump_registry() -> GPtr<u8> {
    CLUMP_REGISTRY.ptr().cast()
}

/// CLUMPMODULE data `G + [0xC924AC]`.
fn module_data() -> GPtr<u8> {
    rw_globals().byte_add(CLUMP_MODULE_OFFSET.get() as i32 as isize)
}

/// Unlink an extension's `inClumpLink` (ext + 4) and zero the extension (+8, +4, +0).
fn unlink_ext(ext: GPtr<u8>) {
    wr::<u32>(p8(rd::<u32>(ext, 8)), 0, rd::<u32>(ext, 4));
    wr::<u32>(p8(rd::<u32>(ext, 4)), 4, rd::<u32>(ext, 8));
    wr::<u32>(ext, 8, 0);
    wr::<u32>(ext, 4, 0);
    wr::<u32>(ext, 0, 0);
}

/// `cb_CLUMPMODULE_RwEngine_Open` (0x749760): the atomic and clump freelists in their static space; a
/// clump freelist failure destroys the atomic one.
#[hook(addr = 0x749760, conv = cdecl, name = "cb_CLUMPMODULE_RwEngine_Open", expect = "8B 44 24 08 8B 0D 80 62 8D 00")]
pub fn module_open(instance: GPtr<u8>, offset: i32, _size: i32) -> GPtr<u8> {
    let p = FREELIST_PARAMS.get();
    CLUMP_MODULE_OFFSET.set(offset as u32);
    let afl = free_list_create_and_preallocate_space(
        rd::<i32>(atomic_registry(), 0),
        p[0],
        4,
        p[1],
        ATOMIC_FREELIST_SPACE.ptr().cast(),
        0x40010,
    );
    wr::<u32>(module_data(), 0, a32(afl));
    if rd::<u32>(module_data(), 0) == 0 {
        return GPtr::NULL;
    }
    let p = FREELIST_PARAMS.get();
    let cfl = free_list_create_and_preallocate_space(
        rd::<i32>(clump_registry(), 0),
        p[2],
        4,
        p[3],
        CLUMP_FREELIST_SPACE.ptr().cast(),
        0x40014,
    );
    wr::<u32>(module_data(), 4, a32(cfl));
    if rd::<u32>(module_data(), 4) != 0 {
        CLUMP_OPEN_COUNT.set(CLUMP_OPEN_COUNT.get().wrapping_add(1));
        return instance;
    }
    free_list_destroy(p8(rd::<u32>(module_data(), 0)));
    wr::<u32>(module_data(), 0, 0);
    GPtr::NULL
}

/// `_rpClumpRegisterExtensions` (0x749830): the CLUMP extensions of cameras and lights.
#[hook(addr = 0x749830, conv = cdecl, name = "_rpClumpRegisterExtensions", expect = "6A 00 68 A0 98 74 00")]
pub fn register_extensions() -> u32 {
    let c = camera_register_plugin(0xC, 0x10, 0x749880, 0x7498A0, 0);
    CAMERA_EXT_OFFSET.set(c);
    if c < 0 {
        return 0;
    }
    let l = light_register_plugin(0xC, 0x10, 0x7498B0, 0x7498D0, 0);
    LIGHT_EXT_OFFSET.set(l);
    (l >= 0) as u32
}

fn zero_ext(ext: GPtr<u8>) {
    wr::<u32>(ext, 8, 0);
    wr::<u32>(ext, 4, 0);
    wr::<u32>(ext, 0, 0);
}

/// `cb_CLUMP_RwCamera_Ctor` (0x749880): zeroes the extension at the global offset; returns camera.
#[hook(addr = 0x749880, conv = cdecl, name = "cb_CLUMP_RwCamera_Ctor", expect = "8B 44 24 04 8B 0D D8 24 C9 00")]
pub fn camera_ctor(cam: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    zero_ext(cam.byte_add(CAMERA_EXT_OFFSET.get() as isize));
    cam
}

/// `cb_CLUMP_RwCamera_Dtor` (0x7498A0): returns camera.
#[hook(addr = 0x7498A0, conv = cdecl, name = "cb_CLUMP_RwCamera_Dtor", expect = "8B 44 24 04 C3")]
pub fn camera_dtor(cam: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    cam
}

/// `cb_CLUMP_RpLight_Ctor` (0x7498B0).
#[hook(addr = 0x7498B0, conv = cdecl, name = "cb_CLUMP_RpLight_Ctor", expect = "8B 44 24 04 8B 0D DC 24 C9 00")]
pub fn light_ctor(light: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    zero_ext(light.byte_add(LIGHT_EXT_OFFSET.get() as isize));
    light
}

/// `cb_CLUMP_RpLight_Dtor` (0x7498D0): returns light.
#[hook(addr = 0x7498D0, conv = cdecl, name = "cb_CLUMP_RpLight_Dtor", expect = "8B 44 24 04 C3")]
pub fn light_dtor(light: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    light
}

/// `RpClumpGetNumAtomics` (0x7498E0): counts links; stops after counting a link whose atomic is NULL.
#[hook(addr = 0x7498E0, conv = cdecl, name = "RpClumpGetNumAtomics", expect = "8B 4C 24 04 33 C0")]
pub fn get_num_atomics(c: GPtr<u8>) -> i32 {
    let head = c.byte_add(8);
    let mut n: i32 = 0;
    let mut l = p8(rd::<u32>(c, 8));
    while l != head {
        n += 1;
        if l.byte_add(-0x40).is_null() {
            break;
        }
        l = p8(rd::<u32>(l, 0));
    }
    n
}

/// Shared body of `RpClumpGetNumLights` / `GetNumCameras` (the offset read once).
fn count_ext_list(c: GPtr<u8>, list: isize, off: i32) -> i32 {
    let head = c.byte_add(list);
    let mut n: i32 = 0;
    let mut l = p8(rd::<u32>(c, list));
    while l != head {
        n += 1;
        if l.byte_add(-(off as isize) - 4).is_null() {
            break;
        }
        l = p8(rd::<u32>(l, 0));
    }
    n
}

/// `RpClumpGetNumLights` (0x749910).
#[hook(addr = 0x749910, conv = cdecl, name = "RpClumpGetNumLights", expect = "8B 4C 24 04 33 C0")]
pub fn get_num_lights(c: GPtr<u8>) -> i32 {
    let head = c.byte_add(0x10);
    if p8(rd::<u32>(c, 0x10)) == head {
        return 0;
    }
    count_ext_list(c, 0x10, LIGHT_EXT_OFFSET.get())
}

/// `RpClumpGetNumCameras` (0x749940).
#[hook(addr = 0x749940, conv = cdecl, name = "RpClumpGetNumCameras", expect = "8B 4C 24 04 33 C0")]
pub fn get_num_cameras(c: GPtr<u8>) -> i32 {
    let head = c.byte_add(0x18);
    if p8(rd::<u32>(c, 0x18)) == head {
        return 0;
    }
    count_ext_list(c, 0x18, CAMERA_EXT_OFFSET.get())
}

/// `RpClumpRender` (0x749B20): renders every atomic with rpATOMICRENDER (LTM synced first); NULL if any
/// callback returned NULL, all atomics rendered anyway; next link read after the callback.
#[hook(addr = 0x749B20, conv = cdecl, name = "RpClumpRender", expect = "8B 44 24 04 53")]
pub fn render(c: GPtr<u8>) -> GPtr<u8> {
    let head = c.byte_add(8);
    let mut result = c;
    let mut l = p8(rd::<u32>(c, 8));
    while l != head {
        let a = l.byte_add(-0x40);
        if rd::<u8>(a, 2) & 4 != 0 {
            let frame = p8(rd::<u32>(a, 4));
            frame_get_ltm(frame);
            if call_render_cb(rd::<u32>(a, 0x48), a, frame).is_null() {
                result = GPtr::NULL;
            }
        }
        l = p8(rd::<u32>(l, 0));
    }
    result
}

/// `RpClumpForAllAtomics` (0x749B70): next read before the callback; returns clump.
#[hook(addr = 0x749B70, conv = cdecl, name = "RpClumpForAllAtomics", expect = "8B 44 24 04 53")]
pub fn for_all_atomics(c: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> {
    let head = c.byte_add(8);
    let mut l = p8(rd::<u32>(c, 8));
    while l != head {
        let n = p8(rd::<u32>(l, 0));
        if call_obj_cb(cb, l.byte_add(-0x40), data).is_null() {
            break;
        }
        l = n;
    }
    c
}

/// Shared body of `RpClumpForAllCameras` / `ForAllLights` (offset re-read per link).
fn for_all_ext(c: GPtr<u8>, list: isize, off: &sa_core::mem::Global<i32>, cb: u32, data: u32) -> GPtr<u8> {
    let head = c.byte_add(list);
    let mut l = p8(rd::<u32>(c, list));
    while l != head {
        let o = off.get();
        let n = p8(rd::<u32>(l, 0));
        if call_obj_cb(cb, l.byte_add(-(o as isize) - 4), data).is_null() {
            break;
        }
        l = n;
    }
    c
}

/// `RpClumpForAllCameras` (0x749BB0).
#[hook(addr = 0x749BB0, conv = cdecl, name = "RpClumpForAllCameras", expect = "8B 44 24 04 53")]
pub fn for_all_cameras(c: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> {
    for_all_ext(c, 0x18, &CAMERA_EXT_OFFSET, cb, data)
}

/// `RpClumpForAllLights` (0x749C00).
#[hook(addr = 0x749C00, conv = cdecl, name = "RpClumpForAllLights", expect = "8B 44 24 04 53")]
pub fn for_all_lights(c: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> {
    for_all_ext(c, 0x10, &LIGHT_EXT_OFFSET, cb, data)
}

/// `RpLightGetClump` (0x749E40).
#[hook(addr = 0x749E40, conv = cdecl, name = "RpLightGetClump", expect = "8B 44 24 04 8B 0D DC 24 C9 00")]
pub fn light_get_clump(light: GPtr<u8>) -> GPtr<u8> {
    p8(rd::<u32>(light, LIGHT_EXT_OFFSET.get() as isize))
}

/// `RwCameraGetClump` (0x749E50).
#[hook(addr = 0x749E50, conv = cdecl, name = "RwCameraGetClump", expect = "8B 44 24 04 8B 0D D8 24 C9 00")]
pub fn camera_get_clump(cam: GPtr<u8>) -> GPtr<u8> {
    p8(rd::<u32>(cam, CAMERA_EXT_OFFSET.get() as isize))
}

/// `_rpClumpRemoveAndDestroyLight` (0x74A1C0): unlink, zero the extension, `RpLightDestroy`; returns light.
#[hook(addr = 0x74A1C0, conv = cdecl, name = "_rpClumpRemoveAndDestroyLight", expect = "A1 DC 24 C9 00")]
pub fn remove_and_destroy_light(light: GPtr<u8>, _data: u32) -> GPtr<u8> {
    unlink_ext(light.byte_add(LIGHT_EXT_OFFSET.get() as isize));
    light_destroy(light);
    light
}

/// `_rpClumpRemoveAndDestroyCamera` (0x74A200).
#[hook(addr = 0x74A200, conv = cdecl, name = "_rpClumpRemoveAndDestroyCamera", expect = "A1 D8 24 C9 00")]
pub fn remove_and_destroy_camera(cam: GPtr<u8>, _data: u32) -> GPtr<u8> {
    unlink_ext(cam.byte_add(CAMERA_EXT_OFFSET.get() as isize));
    camera_destroy(cam);
    cam
}

/// `RpClumpSetCallBack` (0x74A240): NULL installs the default; returns EAX = clump.
#[hook(addr = 0x74A240, conv = cdecl, name = "RpClumpSetCallBack", expect = "8B 4C 24 08 8B 44 24 04")]
pub fn set_callback(c: GPtr<u8>, cb: u32) -> GPtr<u8> {
    wr::<u32>(c, 0x28, if cb != 0 { cb } else { DEFAULT_CALLBACK });
    c
}

/// `_rpClumpDefaultCallBack` (0x74A260): returns its argument. Not hooked: sa-hook's keycells test
/// reports 0x74A278 (`RpClumpGetCallBack`, which compares the callback with this address) as a code-key
/// reader of the patched range.
pub fn default_callback(c: GPtr<u8>, _data: u32) -> GPtr<u8> {
    c
}

/// `RpClumpGetCallBack` (0x74A270): NULL for the default callback.
#[hook(addr = 0x74A270, conv = cdecl, name = "RpClumpGetCallBack", expect = "8B 44 24 04 8B 48 28")]
pub fn get_callback(c: GPtr<u8>) -> u32 {
    let cb = rd::<u32>(c, 0x28);
    if cb == DEFAULT_CALLBACK { 0 } else { cb }
}

/// `RpClumpCreate` (0x74A290): no RwError on allocation failure.
#[hook(addr = 0x74A290, conv = cdecl, name = "RpClumpCreate", expect = "A1 24 7B C9 00")]
pub fn create() -> GPtr<u8> {
    let c = rw_memory_alloc(p8(rd::<u32>(module_data(), 4)), 0x30010);
    if c.is_null() {
        return c;
    }
    for list in [8isize, 0x10] {
        let h = c.byte_add(list);
        wr::<u32>(c, list + 4, a32(h));
        wr::<u32>(h, 0, a32(h));
    }
    wr::<u8>(c, 0, 2);
    wr::<u8>(c, 1, 0);
    wr::<u8>(c, 2, 0);
    wr::<u8>(c, 3, 0);
    wr::<u32>(c, 4, 0);
    let h = c.byte_add(0x18);
    wr::<u32>(h, 0, a32(h));
    wr::<u32>(c, 0x1C, a32(h));
    wr::<u32>(c, 0x24, 0);
    wr::<u32>(c, 0x20, 0);
    wr::<u32>(c, 0x28, DEFAULT_CALLBACK);
    registry_init_object(clump_registry(), c);
    c
}

/// Lights / cameras of `RpClumpDestroy`: unlink + zero the extension, destroy; stops after a NULL
/// object pointer (offset re-read per link).
fn destroy_ext_list(c: GPtr<u8>, list: isize, off: &sa_core::mem::Global<i32>, destroy: fn(GPtr<u8>) -> i32) {
    let head = c.byte_add(list);
    let mut l = p8(rd::<u32>(c, list));
    while l != head {
        let o = off.get();
        let n = p8(rd::<u32>(l, 0));
        let obj = l.byte_add(-(o as isize) - 4);
        unlink_ext(obj.byte_add(o as isize));
        destroy(obj);
        if obj.is_null() {
            break;
        }
        l = n;
    }
}

/// `RpClumpDestroy` (0x74A310): plugin destructors, the atomics (inline `RpAtomicDestroy`, not
/// unlinked), lights, cameras, the frame hierarchy, the clump. Returns 1.
#[hook(addr = 0x74A310, conv = cdecl, name = "RpClumpDestroy", expect = "53 55 56 8B 74 24 10")]
pub fn destroy(c: GPtr<u8>) -> u32 {
    registry_deinit_object(clump_registry(), c);
    let head = c.byte_add(8);
    let mut l = p8(rd::<u32>(c, 8));
    while l != head {
        let n = p8(rd::<u32>(l, 0));
        atomic_destroy_body(l.byte_add(-0x40));
        l = n;
    }
    destroy_ext_list(c, 0x10, &LIGHT_EXT_OFFSET, light_destroy);
    destroy_ext_list(c, 0x18, &CAMERA_EXT_OFFSET, camera_destroy);
    let root = p8(rd::<u32>(c, 4));
    if !root.is_null() {
        frame_destroy_hierarchy(root);
    }
    rw_memory_free(p8(rd::<u32>(module_data(), 4)), c);
    1
}

/// `RpAtomicDestroy` on every atomic of `c` (next read before; stops after a NULL atomic pointer).
fn destroy_atomics(c: GPtr<u8>) {
    let head = c.byte_add(8);
    let mut l = p8(rd::<u32>(c, 8));
    while l != head {
        let n = p8(rd::<u32>(l, 0));
        let a = l.byte_add(-0x40);
        atomic_destroy(a);
        if a.is_null() {
            break;
        }
        l = n;
    }
}

/// `RpClumpClone` (0x749F70). Kept from the original: an atomic without a frame is destroyed, reports
/// `{2, 5}` and ends the atomic loop with success (the clone lacks the remaining atomics); lights and
/// cameras are not cloned; the atomic order is reversed (head insertion).
#[hook(addr = 0x749F70, conv = cdecl, name = "RpClumpClone", expect = "83 EC 14 53 55")]
pub fn clone(src: GPtr<u8>) -> GPtr<u8> {
    let c = create_call();
    if c.is_null() {
        return GPtr::NULL;
    }
    let root = frame_clone_and_link_clones(p8(rd::<u32>(src, 4)));
    wr::<u32>(c, 4, a32(root));
    if root.is_null() {
        // failure A
        registry_deinit_object(clump_registry(), c);
        destroy_atomics(c);
        let head = c.byte_add(0x10);
        let mut l = p8(rd::<u32>(c, 0x10));
        while l != head {
            let o = LIGHT_EXT_OFFSET.get() as isize;
            let n = p8(rd::<u32>(l, 0));
            let owner = p8(rd::<u32>(l, -4));
            let light = l.byte_add(-o - 4);
            remove_light_call(owner, light);
            light_destroy(light);
            if light.is_null() {
                break;
            }
            l = n;
        }
        return clone_fail_tail(c);
    }
    let mut ok = true;
    let head = src.byte_add(8);
    let mut l = p8(rd::<u32>(src, 8));
    while l != head {
        let n = p8(rd::<u32>(l, 0));
        let a = l.byte_add(-0x40);
        let na = atomic_clone(a);
        if na.is_null() {
            ok = false;
            break;
        }
        let frame = p8(rd::<u32>(a, 4));
        if frame.is_null() {
            registry_deinit_object(atomic_registry(), na);
            let rep = p8(rd::<u32>(na, 0x14));
            if !rep.is_null() {
                resources_free_res_entry(rep);
            }
            atomic_set_geometry(na, GPtr::NULL, 0);
            object_release_frame(na);
            rw_memory_free(p8(rd::<u32>(module_data(), 0)), na);
            let mut e = RwError {
                plugin_id: 2,
                error_code: 0,
            };
            e.error_code = rw_error1(5) as i32;
            rw_error_set(GPtr::from_raw(&mut e));
            break;
        }
        object_set_frame(na, p8(rd::<u32>(frame, 0xA0)));
        wr::<u8>(na, 3, rd::<u8>(na, 3) | 1);
        add_atomic_call(c, na);
        l = n;
    }
    frame_purge_clone(p8(rd::<u32>(src, 4)));
    if ok {
        wr::<u32>(c, 0x28, rd::<u32>(src, 0x28));
        registry_copy_object(clump_registry(), c, src);
        return c;
    }
    // failure B
    registry_deinit_object(clump_registry(), c);
    destroy_atomics(c);
    let head = c.byte_add(0x10);
    let mut l = p8(rd::<u32>(c, 0x10));
    while l != head {
        let o = LIGHT_EXT_OFFSET.get() as isize;
        let n = p8(rd::<u32>(l, 0));
        if remove_and_destroy_light_call(l.byte_add(-o - 4), 0).is_null() {
            break;
        }
        l = n;
    }
    clone_fail_tail(c)
}

/// The common failure tail of `RpClumpClone` (0x74A15D): cameras, frame hierarchy, the clump; NULL.
fn clone_fail_tail(c: GPtr<u8>) -> GPtr<u8> {
    for_all_cameras_call(c, 0x74A200, 0);
    let root = p8(rd::<u32>(c, 4));
    if !root.is_null() {
        frame_destroy_hierarchy(root);
    }
    rw_memory_free(p8(rd::<u32>(module_data(), 4)), c);
    GPtr::NULL
}

/// `RpClumpAddAtomic` (0x74A490): head insertion; returns clump.
#[hook(addr = 0x74A490, conv = cdecl, name = "RpClumpAddAtomic", expect = "8B 44 24 04 56")]
pub fn add_atomic(c: GPtr<u8>, a: GPtr<u8>) -> GPtr<u8> {
    let list = c.byte_add(8);
    let link = a.byte_add(0x40);
    let old = rd::<u32>(list, 0);
    wr::<u32>(a, 0x44, a32(list));
    wr::<u32>(link, 0, old);
    wr::<u32>(p8(rd::<u32>(list, 0)), 4, a32(link));
    wr::<u32>(list, 0, a32(link));
    wr::<u32>(a, 0x3C, a32(c));
    c
}

/// `RpClumpRemoveAtomic` (0x74A4C0): unlink (fields kept), `clump = NULL`; returns clump.
#[hook(addr = 0x74A4C0, conv = cdecl, name = "RpClumpRemoveAtomic", expect = "8B 44 24 08 8B 48 44")]
pub fn remove_atomic(c: GPtr<u8>, a: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(p8(rd::<u32>(a, 0x44)), 0, rd::<u32>(a, 0x40));
    wr::<u32>(p8(rd::<u32>(a, 0x40)), 4, rd::<u32>(a, 0x44));
    wr::<u32>(a, 0x3C, 0);
    c
}

/// `RpClumpAddLight` (0x74A4F0): head insertion of the extension link; returns clump.
#[hook(addr = 0x74A4F0, conv = cdecl, name = "RpClumpAddLight", expect = "8B 44 24 08 8B 0D DC 24 C9 00")]
pub fn add_light(c: GPtr<u8>, light: GPtr<u8>) -> GPtr<u8> {
    let ext = light.byte_add(LIGHT_EXT_OFFSET.get() as isize);
    let list = c.byte_add(0x10);
    let link = ext.byte_add(4);
    let old = rd::<u32>(list, 0);
    wr::<u32>(ext, 8, a32(list));
    wr::<u32>(link, 0, old);
    wr::<u32>(p8(rd::<u32>(list, 0)), 4, a32(link));
    wr::<u32>(list, 0, a32(link));
    wr::<u32>(ext, 0, a32(c));
    c
}

/// `RpClumpRemoveLight` (0x74A520): unlink and zero the extension; returns clump.
#[hook(addr = 0x74A520, conv = cdecl, name = "RpClumpRemoveLight", expect = "8B 44 24 08 8B 0D DC 24 C9 00")]
pub fn remove_light(c: GPtr<u8>, light: GPtr<u8>) -> GPtr<u8> {
    unlink_ext(light.byte_add(LIGHT_EXT_OFFSET.get() as isize));
    c
}

/// `RpClumpRegisterPlugin` (0x74BDD0).
#[hook(addr = 0x74BDD0, conv = cdecl, name = "RpClumpRegisterPlugin", expect = "8B 44 24 14 8B 4C 24 10")]
pub fn register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 {
    registry_add_plugin(clump_registry(), size, id, ctor, dtor, copy)
}

/// `RpClumpRegisterPluginStream` (0x74BE70).
#[hook(addr = 0x74BE70, conv = cdecl, name = "RpClumpRegisterPluginStream", expect = "8B 44 24 10 8B 4C 24 0C")]
pub fn register_plugin_stream(id: u32, read: u32, write: u32, get_size: u32) -> i32 {
    registry_add_plugin_stream(clump_registry(), id, read, write, get_size)
}

/// `RpClumpSetStreamAlwaysCallBack` (0x74BEA0).
#[hook(addr = 0x74BEA0, conv = cdecl, name = "RpClumpSetStreamAlwaysCallBack", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn set_stream_always_callback(id: u32, cb: u32) -> i32 {
    registry_add_always_cb(clump_registry(), id, cb)
}

/// `RpClumpGetPluginOffset` (0x74BEE0).
#[hook(addr = 0x74BEE0, conv = cdecl, name = "RpClumpGetPluginOffset", expect = "8B 44 24 04 50")]
pub fn get_plugin_offset(id: u32) -> i32 {
    registry_get_plugin_offset(clump_registry(), id)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_MODULE_OPEN,
    HOOK_REGISTER_EXTENSIONS,
    HOOK_CAMERA_CTOR,
    HOOK_CAMERA_DTOR,
    HOOK_LIGHT_CTOR,
    HOOK_LIGHT_DTOR,
    HOOK_GET_NUM_ATOMICS,
    HOOK_GET_NUM_LIGHTS,
    HOOK_GET_NUM_CAMERAS,
    HOOK_RENDER,
    HOOK_FOR_ALL_ATOMICS,
    HOOK_FOR_ALL_CAMERAS,
    HOOK_FOR_ALL_LIGHTS,
    HOOK_LIGHT_GET_CLUMP,
    HOOK_CAMERA_GET_CLUMP,
    HOOK_REMOVE_AND_DESTROY_LIGHT,
    HOOK_REMOVE_AND_DESTROY_CAMERA,
    HOOK_SET_CALLBACK,
    HOOK_GET_CALLBACK,
    HOOK_CLONE,
    HOOK_CREATE,
    HOOK_DESTROY,
    HOOK_ADD_ATOMIC,
    HOOK_REMOVE_ATOMIC,
    HOOK_ADD_LIGHT,
    HOOK_REMOVE_LIGHT,
    HOOK_REGISTER_PLUGIN,
    HOOK_REGISTER_PLUGIN_STREAM,
    HOOK_SET_STREAM_ALWAYS_CALLBACK,
    HOOK_GET_PLUGIN_OFFSET,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = CLUMP_GLOBALS;
