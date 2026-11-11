//! `RpWorldPluginAttach` and what it registers (PORT-RW-1 follow-up, written for the native boot):
//! the world / sector / bin-world / world-object / world-device engine modules, the world-object
//! plugin callbacks of cameras, atomics, clumps and lights, the world and sector plugin-registry
//! wrappers, and `_rpWorldPipeOpen` / `Close`. Ported from the 1.0 US bytes.
//!
//! WORLDMODULE `W = G + [0xC9254C]`: +0 world freelist (0x10-byte entries), +4 RwLinkList.
//! WORLDOBJMODULE `O = G + [0xC9259C]`: +0 atomic-tie freelist, +4 light-tie freelist (0x18-byte
//! entries), +8 the world new clumps are added to. Plugin offsets: camera `[0xC925A4]` (0x1C: +0 sector
//! array, +4 capacity, +8 count, +0xC world, +0x10/+0x14/+0x18 the camera's saved callbacks), atomic
//! `[0xC925A8]` (+0 world, +4 saved sync callback), clump `[0xC925AC]` (+0 world, +4 link), light
//! `[0xC925B0]` (+0 world, +4 saved sync callback). A tie (0x18): +0/+4 link in the sector, +8 object,
//! +0xC/+0x10 link in the object's tie list, +0x14 sector.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, rw_free, rw_globals, rw_memory_free, wr};

/// WORLDMODULE offset global.
const WORLD_MODULE: usize = 0xC9254C;
/// WORLDOBJMODULE offset global.
const WORLDOBJ_MODULE: usize = 0xC9259C;
/// Pipeline-module offset global (`P = G + [0xC9BC60]`).
const PIPE_MODULE: usize = 0xC9BC60;
/// World-object plugin offsets (written by `_rpWorldObjRegisterPlugins`).
const CAMERA_WORLD_OFFSET: usize = 0xC925A4;
const ATOMIC_WORLD_OFFSET: usize = 0xC925A8;
const CLUMP_WORLD_OFFSET: usize = 0xC925AC;
const LIGHT_WORLD_OFFSET: usize = 0xC925B0;
/// CLUMPMODULE offset global and instance count (clump.rs).
const CLUMP_MODULE: usize = 0xC924AC;
const CLUMP_INSTANCES: usize = 0xC924B0;
/// `RpWorld` / `RpWorldSector` plugin registries.
const WORLD_REGISTRY: u32 = 0x8D62D0;
const SECTOR_REGISTRY: u32 = 0x8D736C;

sa_core::globals! {
    list WORLDPLUGIN_GLOBALS;
    /// WORLDMODULE open-instance counter
    pub static WORLD_INSTANCES @ 0xC92550: i32 = 0;
    /// Static space of the world freelist (0x24 bytes)
    pub static WORLD_FL_SPACE @ 0xC92528: [u32; 9] = [0; 9];
    /// SECTORMODULE open-instance counter
    pub static SECTOR_INSTANCES @ 0xC94C0C: i32 = 0;
    /// BINWORLDMODULE open-instance counter
    pub static BINWORLD_INSTANCES @ 0xC94C24: i32 = 0;
    /// WORLDOBJMODULE open-instance counter
    pub static WORLDOBJ_INSTANCES @ 0xC925A0: i32 = 0;
    /// Static space of the atomic-tie freelist (0x24 bytes)
    pub static ATOMIC_TIE_FL_SPACE @ 0xC92578: [u32; 9] = [0; 9];
    /// Static space of the light-tie freelist (0x24 bytes)
    pub static LIGHT_TIE_FL_SPACE @ 0xC92554: [u32; 9] = [0; 9];
    /// Atomic-tie freelist entries per block (0x100)
    pub static ATOMIC_TIE_FL_PER_BLOCK @ 0x8D62E8: u32 = 0x100;
    /// Atomic-tie freelist preallocated blocks (1)
    pub static ATOMIC_TIE_FL_PREALLOC @ 0x8D62EC: u32 = 1;
    /// Light-tie freelist entries per block (0x20)
    pub static LIGHT_TIE_FL_PER_BLOCK @ 0x8D62F0: u32 = 0x20;
    /// Light-tie freelist preallocated blocks (1)
    pub static LIGHT_TIE_FL_PREALLOC @ 0x8D62F4: u32 = 1;
    /// The restore callback that was installed before `_rpWorldDeviceRegisterPlugins` replaced it
    pub static PREV_RESTORE_CALLBACK @ 0xC9251C: u32 = 0;
    /// RpLight plugin offset of the D3D9 light data (`_rpD3D9LightRegisterPlugin`)
    pub static D3D9_LIGHT_OFFSET @ 0xC926B0: i32 = 0;
    /// RXWORLDDEVICEMODULE open-instance counter
    pub static WORLD_DEVICE_INSTANCES @ 0xC926E4: i32 = 0;
    /// RpGeometry plugin offset of the world-device data
    pub static WORLD_DEVICE_GEOMETRY_OFFSET @ 0xC926E8: i32 = 0;
    /// RpWorldSector plugin offset of the world-device data
    pub static WORLD_DEVICE_SECTOR_OFFSET @ 0xC926EC: i32 = 0;
}

orig! {
    /// `RwEngineRegisterPlugin(size, id, open, close)` (0x7F2BB0)
    fn engine_register_plugin(size: i32, id: u32, open: u32, close: u32) -> i32 = cdecl 0x7F2BB0 => crate::plugin::engine_register_plugin;
    /// `RwCameraRegisterPlugin` (0x7EE450)
    fn camera_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x7EE450 => crate::camera::register_plugin;
    /// `RpAtomicRegisterPlugin` (0x74BDA0)
    fn atomic_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x74BDA0 => crate::atomic::register_plugin;
    /// `RpAtomicRegisterPluginStream` (0x74BE00)
    fn atomic_register_plugin_stream(id: u32, read: u32, write: u32, get_size: u32) -> i32 = cdecl 0x74BE00 => crate::atomic::register_plugin_stream;
    /// `RpClumpRegisterPlugin` (0x74BDD0)
    fn clump_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x74BDD0 => crate::clump::register_plugin;
    /// `RpLightRegisterPlugin` (0x751D60)
    fn light_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x751D60 => crate::light::rp_light_register_plugin;
    /// `RpGeometryRegisterPlugin` (0x74CD70)
    fn geometry_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x74CD70 => crate::geometry::rp_geometry_register_plugin;
    /// `RpGeometryRegisterPluginStream` (0x74CDA0)
    fn geometry_register_plugin_stream(id: u32, read: u32, write: u32, get_size: u32) -> i32 = cdecl 0x74CDA0 => crate::geometry::rp_geometry_register_plugin_stream;
    /// `RpMaterialRegisterPlugin` (0x74DBF0)
    fn material_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x74DBF0 => crate::material::rp_material_register_plugin;
    /// `RpMaterialRegisterPluginStream` (0x74DC20)
    fn material_register_plugin_stream(id: u32, read: u32, write: u32, get_size: u32) -> i32 = cdecl 0x74DC20 => crate::material::rp_material_register_plugin_stream;
    /// `RpWorldRegisterPlugin` (0x74FCD0)
    fn world_register_plugin_call(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x74FCD0 => rp_world_register_plugin;
    /// `RpWorldRegisterPluginStream` (0x74FD00)
    fn world_register_plugin_stream_call(id: u32, read: u32, write: u32, get_size: u32) -> i32 = cdecl 0x74FD00 => rp_world_register_plugin_stream;
    /// `RpWorldSectorRegisterPlugin` (0x761C90)
    fn sector_register_plugin_call(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x761C90 => rp_world_sector_register_plugin;
    /// `RpWorldSectorRegisterPluginStream` (0x761CC0)
    fn sector_register_plugin_stream_call(id: u32, read: u32, write: u32, get_size: u32) -> i32 = cdecl 0x761CC0 => rp_world_sector_register_plugin_stream;
    /// `_rwPluginRegistryAddPlugin` (0x8084A0)
    fn plugin_add(reg: GPtr<u8>, size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x8084A0 => crate::plugin::registry_add_plugin;
    /// `_rwPluginRegistryAddPluginStream` (0x8088E0)
    fn plugin_add_stream(reg: GPtr<u8>, id: u32, read: u32, write: u32, get_size: u32) -> i32 = cdecl 0x8088E0 => crate::plugin::registry_add_plugin_stream;

    /// `_rpWorldObjRegisterPlugins()` (0x750050)
    fn world_obj_register_plugins_call() -> i32 = cdecl 0x750050 => rp_world_obj_register_plugins;
    /// `_rpClumpRegisterExtensions()` (0x749830)
    fn clump_register_extensions() -> u32 = cdecl 0x749830 => crate::clump::register_extensions;
    /// `_rpWorldDeviceRegisterPlugins()` (0x74EA40)
    fn world_device_register_plugins_call() -> i32 = cdecl 0x74EA40 => rp_world_device_register_plugins;
    /// `_rpWorldObjDummyTrue()` (0x75E3E0)
    fn world_obj_dummy_true_call() -> i32 = cdecl 0x75E3E0 => rp_world_obj_dummy_true;
    /// `_rpD3D9LightRegisterPlugin()` (0x755B90)
    fn d3d9_light_register_plugin_call() -> i32 = cdecl 0x755B90 => unported;
    /// `_rpWorldDeviceRegisterPlugins2()` (0x7587D0)
    fn world_device_register_plugins2_call() -> i32 = cdecl 0x7587D0 => rp_world_device_register_plugins2;
    /// `_rwD3D9DeviceGetRestoreCallback()` (0x7FAE30, D3D9 driver)
    fn d3d9_device_get_restore_callback() -> u32 = cdecl 0x7FAE30 => unported;
    /// `_rwD3D9DeviceSetRestoreCallback(cb)` (0x7FAE20, D3D9 driver)
    fn d3d9_device_set_restore_callback(cb: u32) = cdecl 0x7FAE20 => unported;

    /// `_rpWorldPipeOpen()` (0x75E310)
    fn world_pipe_open_call() -> i32 = cdecl 0x75E310 => rp_world_pipe_open;
    /// `_rpWorldPipeClose()` (0x75E300)
    fn world_pipe_close_call() -> u32 = cdecl 0x75E300 => rp_world_pipe_close;
    /// `_rpWorldPipeDummyTrue()` (0x74EAB0)
    fn pipe_dummy_true_call() -> i32 = cdecl 0x74EAB0 => rp_world_pipe_dummy_true;
    /// `_rpWorldPipeDummyVoid()` (0x74EAC0)
    fn pipe_dummy_void_call() = cdecl 0x74EAC0 => rp_world_pipe_dummy_void;
    /// `_rpCreateD3D9AtomicAllInOnePipe()` (0x74EB80, D3D9 pipeline)
    fn create_d3d9_atomic_pipe() -> i32 = cdecl 0x74EB80 => unported;
    /// `_rpCreateD3D9WorldSectorAllInOnePipe()` (0x74EAD0, D3D9 pipeline)
    fn create_d3d9_sector_pipe() -> i32 = cdecl 0x74EAD0 => unported;
    /// `_rpDestroyD3D9WorldSectorPipe()` (0x74EB40, D3D9 pipeline; EAX not used)
    fn destroy_d3d9_sector_pipe() = cdecl 0x74EB40 => unported;
    /// `_rpD3D9GeometryListDestroy()` (0x74EC00, D3D9; its EAX is `_rpWorldPipeClose`'s)
    fn d3d9_geometry_list_destroy() -> u32 = cdecl 0x74EC00 => unported;

    /// `RwFreeListCreateAndPreallocateSpace` (0x801B70)
    fn free_list_create_prealloc(size: u32, per_block: u32, align: u32, n: u32, space: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl 0x801B70 => crate::memory::rw_freelist_create_and_preallocate_space;
    /// `RwFreeListDestroy(fl)` (0x801B80)
    fn free_list_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => crate::memory::rw_freelist_destroy;
    /// `RwFreeListForAllUsed(fl, cb, data)` (0x801E90)
    fn free_list_for_all_used(fl: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x801E90 => unported;
    /// `RwFrameUpdateObjects(frame)` (0x7F0910)
    fn frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => crate::frame::update_objects;
    /// `RpWorldAddClump(world, clump)` (0x751300)
    fn world_add_clump(w: GPtr<u8>, c: GPtr<u8>) -> GPtr<u8> = cdecl 0x751300 => unported;
}

fn a32(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// `G + [offset global]`, recomputed on every access as the originals reload both globals.
fn module(offset_global: usize) -> GPtr<u8> {
    let off = g::<u32>(offset_global).read();
    rw_globals().byte_add(off as isize)
}

/// `obj + [plugin offset global]`.
fn ext(obj: GPtr<u8>, offset_global: usize) -> GPtr<u8> {
    obj.byte_add(g::<i32>(offset_global).read() as isize)
}

/// `RpWorldPluginAttach()` (0x74FDA0): registers the material (0x501), mesh (0x502), geometry
/// (0x503), clump (0x504), light (0x505), sector (0x50A), world (0x507) and bin-world (0x50B) engine
/// modules; a negative OR of the eight offsets returns FALSE. Then `_rpWorldObjRegisterPlugins`,
/// `_rpClumpRegisterExtensions` (each failure returns its 0) and `_rpWorldDeviceRegisterPlugins`,
/// whose result is normalised to 0/1.
#[hook(addr = 0x74FDA0, conv = cdecl, name = "RpWorldPluginAttach", expect = "56 68 50 D9 74 00")]
pub fn rp_world_plugin_attach() -> i32 {
    let mut r = engine_register_plugin(4, 0x501, 0x74D8E0, 0x74D950);
    r |= engine_register_plugin(0x30, 0x502, 0x758970, 0x758940);
    r |= engine_register_plugin(4, 0x503, 0x74BFA0, 0x74BFC0);
    r |= engine_register_plugin(8, 0x504, 0x749760, 0x749480);
    r |= engine_register_plugin(4, 0x505, 0x752270, 0x7521B0);
    r |= engine_register_plugin(0, 0x50A, 0x761C30, 0x761C40);
    r |= engine_register_plugin(0xC, 0x507, 0x74FEE0, 0x74FE90);
    r |= engine_register_plugin(0, 0x50B, 0x7637E0, 0x7637D0);
    if r < 0 {
        return 0;
    }
    let e = world_obj_register_plugins_call();
    if e == 0 {
        return e;
    }
    let e = clump_register_extensions();
    if e == 0 {
        return 0;
    }
    (world_device_register_plugins_call() != 0) as i32
}

/// `cb_WORLDMODULE_RwEngine_Open(instance, offset, size)` (0x74FEE0): stores the offset, opens the
/// world pipelines (`_rpWorldPipeOpen`; failure returns its 0), creates the world freelist (0x10-byte
/// entries, 8 per block, 1 block, static space 0xC92528, hint 0x4000B; failure returns NULL without
/// closing the pipelines), empties the world list, counts the instance. Returns `instance`.
#[hook(addr = 0x74FEE0, conv = cdecl, name = "cb_WORLDMODULE_RwEngine_Open", expect = "8B 44 24 08 A3 4C 25 C9 00")]
pub fn world_module_open(instance: GPtr<u8>, offset: u32, _size: u32) -> GPtr<u8> {
    g::<u32>(WORLD_MODULE).write(offset);
    if world_pipe_open_call() == 0 {
        return GPtr::NULL;
    }
    let fl = free_list_create_prealloc(0x10, 8, 4, 1, WORLD_FL_SPACE.ptr().cast(), 0x4000B);
    wr::<u32>(module(WORLD_MODULE), 0, a32(fl));
    if rd::<u32>(module(WORLD_MODULE), 0) == 0 {
        return GPtr::NULL;
    }
    let head = module(WORLD_MODULE).byte_add(4);
    wr::<u32>(head, 0, a32(head));
    let head = module(WORLD_MODULE).byte_add(4);
    wr::<u32>(module(WORLD_MODULE), 8, a32(head));
    WORLD_INSTANCES.set(WORLD_INSTANCES.get().wrapping_add(1));
    instance
}

/// `cb_WORLDMODULE_RwEngine_Close(instance, offset, size)` (0x74FE90): destroys the world freelist when
/// present (and clears it), closes the world pipelines, decrements the count. Returns `instance`.
#[hook(addr = 0x74FE90, conv = cdecl, name = "cb_WORLDMODULE_RwEngine_Close", expect = "A1 24 7B C9 00")]
pub fn world_module_close(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    let fl = rd::<u32>(module(WORLD_MODULE), 0);
    if fl != 0 {
        free_list_destroy(p8(fl));
        wr::<u32>(module(WORLD_MODULE), 0, 0);
    }
    world_pipe_close_call();
    WORLD_INSTANCES.set(WORLD_INSTANCES.get().wrapping_sub(1));
    instance
}

/// `cb_SECTORMODULE_RwEngine_Open` (0x761C30): counts the instance, returns `instance`.
#[hook(addr = 0x761C30, conv = cdecl, name = "cb_SECTORMODULE_RwEngine_Open", expect = "A1 0C 4C C9 00")]
pub fn sector_module_open(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    SECTOR_INSTANCES.set(SECTOR_INSTANCES.get().wrapping_add(1));
    instance
}

/// `cb_SECTORMODULE_RwEngine_Close` (0x761C40): decrements the count, returns `instance`.
#[hook(addr = 0x761C40, conv = cdecl, name = "cb_SECTORMODULE_RwEngine_Close", expect = "A1 0C 4C C9 00")]
pub fn sector_module_close(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    SECTOR_INSTANCES.set(SECTOR_INSTANCES.get().wrapping_sub(1));
    instance
}

/// `cb_BINWORLDMODULE_RwEngine_Open` (0x7637E0): counts the instance, returns `instance`.
#[hook(addr = 0x7637E0, conv = cdecl, name = "cb_BINWORLDMODULE_RwEngine_Open", expect = "A1 24 4C C9 00")]
pub fn binworld_module_open(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    BINWORLD_INSTANCES.set(BINWORLD_INSTANCES.get().wrapping_add(1));
    instance
}

/// `cb_BINWORLDMODULE_RwEngine_Close` (0x7637D0): decrements the count, returns `instance`.
#[hook(addr = 0x7637D0, conv = cdecl, name = "cb_BINWORLDMODULE_RwEngine_Close", expect = "A1 24 4C C9 00")]
pub fn binworld_module_close(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    BINWORLD_INSTANCES.set(BINWORLD_INSTANCES.get().wrapping_sub(1));
    instance
}

/// `_rpWorldObjRegisterPlugins()` (0x750050): the WORLDOBJMODULE engine module (0x509, 0xC bytes), the
/// world-object extensions of cameras (0x1C), atomics, clumps and lights (8 each; offsets stored in
/// 0xC925A4..0xC925B0), the geometry / sector / material / atomic / world registrations of the
/// plugin ids 0x50E, 0x510 and 0x1F (sizes 0, stream callbacks as in the bytes). Any negative result
/// in the OR returns FALSE; else `_rpWorldObjDummyTrue()` normalised to 0/1.
#[hook(addr = 0x750050, conv = cdecl, name = "_rpWorldObjRegisterPlugins", expect = "56 68 70 03 75 00")]
pub fn rp_world_obj_register_plugins() -> i32 {
    let mut r = engine_register_plugin(0xC, 0x509, 0x750280, 0x750370);
    let o = camera_register_plugin(0x1C, 0x509, 0x7503E0, 0x750760, 0x750710);
    g::<i32>(CAMERA_WORLD_OFFSET).write(o);
    r |= o;
    let o = atomic_register_plugin(8, 0x509, 0x7507B0, 0x7509B0, 0x7509A0);
    g::<i32>(ATOMIC_WORLD_OFFSET).write(o);
    r |= o;
    let o = clump_register_plugin(8, 0x509, 0x750A30, 0x750A90, 0x750A60);
    g::<i32>(CLUMP_WORLD_OFFSET).write(o);
    r |= o;
    let o = light_register_plugin(8, 0x509, 0x750AA0, 0x750D30, 0x750CD0);
    g::<i32>(LIGHT_WORLD_OFFSET).write(o);
    r |= o;
    r |= geometry_register_plugin(0, 0x50E, 0, 0, 0);
    r |= sector_register_plugin_call(0, 0x50E, 0, 0, 0);
    r |= geometry_register_plugin_stream(0x50E, 0x750DB0, 0x750D90, 0x750DE0);
    r |= sector_register_plugin_stream_call(0x50E, 0x750ED0, 0x750EA0, 0x750F00);
    r |= geometry_register_plugin(0, 0x510, 0, 0, 0);
    r |= sector_register_plugin_call(0, 0x510, 0, 0, 0);
    r |= geometry_register_plugin_stream(0x510, 0x750E20, 0x750E00, 0x750E40);
    r |= sector_register_plugin_stream_call(0x510, 0x750E70, 0x750E50, 0x750E90);
    r |= atomic_register_plugin(0, 0x1F, 0, 0, 0);
    r |= atomic_register_plugin_stream(0x1F, 0x749120, 0x749160, 0x7491A0);
    r |= world_register_plugin_call(0, 0x1F, 0, 0, 0);
    r |= world_register_plugin_stream_call(0x1F, 0x761D60, 0x761DA0, 0x761DE0);
    r |= sector_register_plugin_call(0, 0x1F, 0, 0, 0);
    r |= sector_register_plugin_stream_call(0x1F, 0x761E00, 0x761E40, 0x761E80);
    r |= material_register_plugin(0, 0x1F, 0, 0, 0);
    r |= material_register_plugin_stream(0x1F, 0x74D7D0, 0x74D810, 0x74D850);
    if r < 0 {
        return 0;
    }
    (world_obj_dummy_true_call() != 0) as i32
}

/// `_rpWorldObjDummyTrue()` (0x75E3E0, catalogue `FUN_0075e3e0`): returns TRUE.
#[hook(addr = 0x75E3E0, conv = cdecl, name = "FUN_0075e3e0", expect = "B8 01 00 00 00")]
pub fn rp_world_obj_dummy_true() -> i32 {
    1
}

/// `cb_WORLDOBJMODULE_RwEngine_Open(instance, offset, size)` (0x750280): the atomic-tie freelist
/// (0x18-byte entries, hint 0x40507, space 0xC92578) and the light-tie freelist (space 0xC92554;
/// failure destroys the first). Quirk kept: it then writes the u16 at `G+8` (not in the module) = 1;
/// `O+8` (world for new clumps) = NULL; counts the instance. Returns `instance` or NULL.
#[hook(addr = 0x750280, conv = cdecl, name = "cb_WORLDOBJMODULE_RwEngine_Open", expect = "8B 0D EC 62 8D 00")]
pub fn world_obj_module_open(instance: GPtr<u8>, offset: u32, _size: u32) -> GPtr<u8> {
    let prealloc = ATOMIC_TIE_FL_PREALLOC.get();
    let per_block = ATOMIC_TIE_FL_PER_BLOCK.get();
    g::<u32>(WORLDOBJ_MODULE).write(offset);
    let fl = free_list_create_prealloc(0x18, per_block, 4, prealloc, ATOMIC_TIE_FL_SPACE.ptr().cast(), 0x40507);
    wr::<u32>(module(WORLDOBJ_MODULE), 0, a32(fl));
    if rd::<u32>(module(WORLDOBJ_MODULE), 0) == 0 {
        return GPtr::NULL;
    }
    let prealloc = LIGHT_TIE_FL_PREALLOC.get();
    let per_block = LIGHT_TIE_FL_PER_BLOCK.get();
    let fl = free_list_create_prealloc(0x18, per_block, 4, prealloc, LIGHT_TIE_FL_SPACE.ptr().cast(), 0x40507);
    wr::<u32>(module(WORLDOBJ_MODULE), 4, a32(fl));
    let gl = rw_globals();
    let m = module(WORLDOBJ_MODULE);
    if rd::<u32>(m, 4) == 0 {
        free_list_destroy(p8(rd::<u32>(m, 0)));
        wr::<u32>(module(WORLDOBJ_MODULE), 0, 0);
        return GPtr::NULL;
    }
    wr::<u16>(gl, 8, 1);
    wr::<u32>(module(WORLDOBJ_MODULE), 8, 0);
    WORLDOBJ_INSTANCES.set(WORLDOBJ_INSTANCES.get().wrapping_add(1));
    instance
}

/// `cb_WORLDOBJMODULE_RwEngine_Close(instance, offset, size)` (0x750370): destroys the light-tie, then
/// the atomic-tie freelist when present (each cleared), decrements the count. Returns `instance`.
#[hook(addr = 0x750370, conv = cdecl, name = "cb_WORLDOBJMODULE_RwEngine_Close", expect = "A1 9C 25 C9 00")]
pub fn world_obj_module_close(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    let fl = rd::<u32>(module(WORLDOBJ_MODULE), 4);
    if fl != 0 {
        free_list_destroy(p8(fl));
        wr::<u32>(module(WORLDOBJ_MODULE), 4, 0);
    }
    let fl = rd::<u32>(module(WORLDOBJ_MODULE), 0);
    if fl != 0 {
        free_list_destroy(p8(fl));
        wr::<u32>(module(WORLDOBJ_MODULE), 0, 0);
    }
    WORLDOBJ_INSTANCES.set(WORLDOBJ_INSTANCES.get().wrapping_sub(1));
    instance
}

/// `RpWorldSectorRegisterPlugin(size, id, ctor, dtor, copy)` (0x761C90).
#[hook(addr = 0x761C90, conv = cdecl, name = "RpWorldSectorRegisterPlugin", expect = "8B 44 24 14 8B 4C 24 10")]
pub fn rp_world_sector_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 {
    plugin_add(p8(SECTOR_REGISTRY), size, id, ctor, dtor, copy)
}

/// `RpWorldSectorRegisterPluginStream(id, read, write, getSize)` (0x761CC0).
#[hook(addr = 0x761CC0, conv = cdecl, name = "RpWorldSectorRegisterPluginStream", expect = "8B 44 24 10 8B 4C 24 0C")]
pub fn rp_world_sector_register_plugin_stream(id: u32, read: u32, write: u32, get_size: u32) -> i32 {
    plugin_add_stream(p8(SECTOR_REGISTRY), id, read, write, get_size)
}

/// `RpWorldRegisterPlugin(size, id, ctor, dtor, copy)` (0x74FCD0).
#[hook(addr = 0x74FCD0, conv = cdecl, name = "RpWorldRegisterPlugin", expect = "8B 44 24 14 8B 4C 24 10")]
pub fn rp_world_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 {
    plugin_add(p8(WORLD_REGISTRY), size, id, ctor, dtor, copy)
}

/// `RpWorldRegisterPluginStream(id, read, write, getSize)` (0x74FD00).
#[hook(addr = 0x74FD00, conv = cdecl, name = "RpWorldRegisterPluginStream", expect = "8B 44 24 10 8B 4C 24 0C")]
pub fn rp_world_register_plugin_stream(id: u32, read: u32, write: u32, get_size: u32) -> i32 {
    plugin_add_stream(p8(WORLD_REGISTRY), id, read, write, get_size)
}

/// `_rpWorldPipeOpen()` (0x75E310): clears the nine words `P+0x3C..P+0x5C` of the pipeline module,
/// then `_rpWorldPipeDummyTrue`, `_rpCreateD3D9AtomicAllInOnePipe`,
/// `_rpCreateD3D9WorldSectorAllInOnePipe`; the first failure runs the close sequence (sector-pipe
/// destroy, geometry-list destroy, dummy) and returns FALSE.
#[hook(addr = 0x75E310, conv = cdecl, name = "_rpWorldPipeOpen", expect = "A1 60 BC C9 00")]
pub fn rp_world_pipe_open() -> i32 {
    for off in (0x3C..=0x5C).step_by(4) {
        wr::<u32>(module(PIPE_MODULE), off, 0);
    }
    if pipe_dummy_true_call() != 0 && create_d3d9_atomic_pipe() != 0 && create_d3d9_sector_pipe() != 0 {
        return 1;
    }
    destroy_d3d9_sector_pipe();
    d3d9_geometry_list_destroy();
    pipe_dummy_void_call();
    0
}

/// `_rpWorldPipeClose()` (0x75E300): sector-pipe destroy, geometry-list destroy, then a tail jump to
/// `_rpWorldPipeDummyVoid`; EAX is the geometry-list destroy's (returned here).
#[hook(addr = 0x75E300, conv = cdecl, name = "_rpWorldPipeClose", expect = "E8 3B 08 FF FF")]
pub fn rp_world_pipe_close() -> u32 {
    destroy_d3d9_sector_pipe();
    let r = d3d9_geometry_list_destroy();
    pipe_dummy_void_call();
    r
}

/// `_rpWorldPipeDummyTrue()` (0x74EAB0, catalogue `FUN_0074eab0`): returns TRUE.
#[hook(addr = 0x74EAB0, conv = cdecl, name = "FUN_0074eab0", expect = "B8 01 00 00 00")]
pub fn rp_world_pipe_dummy_true() -> i32 {
    1
}

/// `_rpWorldPipeDummyVoid()` (0x74EAC0, catalogue `FUN_0074eac0`): a lone `ret` (not hookable).
#[sa_core::native(addr = 0x74EAC0, conv = cdecl, name = "FUN_0074eac0")]
pub fn rp_world_pipe_dummy_void() {}

/// `_rpWorldDeviceRegisterPlugins()` (0x74EA40, catalogue `FUN_0074ea40`): result = TRUE when
/// `_rpD3D9LightRegisterPlugin` and `_rpWorldDeviceRegisterPlugins2` both succeed (the second is
/// skipped after a failure); then saves the device's restore callback in 0xC9251C and installs
/// `_rpD3D9WorldRestoreCallback` (0x74EA80) in its place, whatever the result.
#[hook(addr = 0x74EA40, conv = cdecl, name = "FUN_0074ea40", expect = "56 E8 4A 71 00 00")]
pub fn rp_world_device_register_plugins() -> i32 {
    let ok = (d3d9_light_register_plugin_call() != 0 && world_device_register_plugins2_call() != 0) as i32;
    PREV_RESTORE_CALLBACK.set(d3d9_device_get_restore_callback());
    d3d9_device_set_restore_callback(0x74EA80);
    ok
}

/// `_rpWorldDeviceRegisterPlugins2()` (0x7587D0): the RXWORLDDEVICEMODULE engine module (0x50F,
/// size 0; negative returns FALSE), a 4-byte RpGeometry plugin (offset in 0xC926E8; negative returns
/// FALSE) and a 4-byte RpWorldSector plugin (offset in 0xC926EC) with the same three callbacks.
/// Returns `sector offset >= 0`.
#[hook(addr = 0x7587D0, conv = cdecl, name = "_rpWorldDeviceRegisterPlugins2", expect = "68 A0 88 75 00")]
pub fn rp_world_device_register_plugins2() -> i32 {
    if engine_register_plugin(0, 0x50F, 0x758890, 0x7588A0) < 0 {
        return 0;
    }
    let o = geometry_register_plugin(4, 0x50F, 0x758850, 0x758860, 0x758870);
    WORLD_DEVICE_GEOMETRY_OFFSET.set(o);
    if o < 0 {
        return 0;
    }
    let o = sector_register_plugin_call(4, 0x50F, 0x758850, 0x758860, 0x758870);
    WORLD_DEVICE_SECTOR_OFFSET.set(o);
    (o >= 0) as i32
}

/// `cb_RXWORLDDEVICEMODULE_RpGeometry_RpWorldSector_Ctor(obj, offset, size)` (0x758850): clears the
/// plugin word; returns `obj`.
#[hook(addr = 0x758850, conv = cdecl, name = "cb_RXWORLDDEVICEMODULE_RpGeometry_RpWorldSector_Ctor", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn world_device_obj_ctor(obj: GPtr<u8>, offset: i32, _size: i32) -> GPtr<u8> {
    wr::<u32>(obj, offset as isize, 0);
    obj
}

/// `cb_RXWORLDDEVICEMODULE_RpGeometry_RpWorldSector_Dtor(obj, offset, size)` (0x758860): clears the
/// plugin word; returns `obj`.
#[hook(addr = 0x758860, conv = cdecl, name = "cb_RXWORLDDEVICEMODULE_RpGeometry_RpWorldSector_Dtor", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn world_device_obj_dtor(obj: GPtr<u8>, offset: i32, _size: i32) -> GPtr<u8> {
    wr::<u32>(obj, offset as isize, 0);
    obj
}

/// `cb_RXWORLDDEVICEMODULE_RpGeometry_RpWorldSector_Copy(dst, src, offset, size)` (0x758870): copies
/// the plugin word; returns `dst`.
#[hook(addr = 0x758870, conv = cdecl, name = "cb_RXWORLDDEVICEMODULE_RpGeometry_RpWorldSector_Copy", expect = "8B 4C 24 0C 8B 44 24 08")]
pub fn world_device_obj_copy(dst: GPtr<u8>, src: GPtr<u8>, offset: i32, _size: i32) -> GPtr<u8> {
    wr::<u32>(dst, offset as isize, rd::<u32>(src, offset as isize));
    dst
}

/// `cb_RXWORLDDEVICEMODULE_RwEngine_Open` (0x758890): counts the instance, returns `instance`.
#[hook(addr = 0x758890, conv = cdecl, name = "cb_RXWORLDDEVICEMODULE_RwEngine_Open", expect = "A1 E4 26 C9 00")]
pub fn world_device_module_open(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    WORLD_DEVICE_INSTANCES.set(WORLD_DEVICE_INSTANCES.get().wrapping_add(1));
    instance
}

/// `cb_RXWORLDDEVICEMODULE_RwEngine_Close` (0x7588A0): decrements the count, returns `instance`.
#[hook(addr = 0x7588A0, conv = cdecl, name = "cb_RXWORLDDEVICEMODULE_RwEngine_Close", expect = "A1 E4 26 C9 00")]
pub fn world_device_module_close(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    WORLD_DEVICE_INSTANCES.set(WORLD_DEVICE_INSTANCES.get().wrapping_sub(1));
    instance
}

/// `cb_WORLDOBJMODULE_RwCamera_Ctor(camera, offset, size)` (0x7503E0): sector array, capacity, count
/// cleared; saves the camera's callbacks +0x18, +0x1C, +0x10 in ext +0x10, +0x14, +0x18 and installs
/// 0x750480 (+0x10), 0x750430 (+0x18), 0x750460 (+0x1C); world (+0xC) NULL. Returns `camera`.
#[hook(addr = 0x7503E0, conv = cdecl, name = "cb_WORLDOBJMODULE_RwCamera_Ctor", expect = "8B 44 24 04 8B 0D A4 25 C9 00")]
pub fn world_obj_camera_ctor(camera: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let c = ext(camera, CAMERA_WORLD_OFFSET);
    wr::<u32>(c, 0, 0);
    wr::<u32>(c, 4, 0);
    wr::<u32>(c, 8, 0);
    wr::<u32>(c, 0x10, rd::<u32>(camera, 0x18));
    wr::<u32>(c, 0x14, rd::<u32>(camera, 0x1C));
    wr::<u32>(c, 0x18, rd::<u32>(camera, 0x10));
    wr::<u32>(camera, 0x10, 0x750480);
    wr::<u32>(camera, 0x18, 0x750430);
    wr::<u32>(camera, 0x1C, 0x750460);
    wr::<u32>(c, 0xC, 0);
    camera
}

/// `cb_WORLDOBJMODULE_RwCamera_Dtor(camera, offset, size)` (0x750760): frees the sector array when
/// present, clears array / capacity / count, restores the three saved camera callbacks. Returns
/// `camera`.
#[hook(addr = 0x750760, conv = cdecl, name = "cb_WORLDOBJMODULE_RwCamera_Dtor", expect = "A1 A4 25 C9 00")]
pub fn world_obj_camera_dtor(camera: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let c = ext(camera, CAMERA_WORLD_OFFSET);
    let arr = rd::<u32>(c, 0);
    if arr != 0 {
        rw_free(p8(arr));
    }
    let s10 = rd::<u32>(c, 0x10);
    wr::<u32>(c, 0, 0);
    wr::<u32>(c, 4, 0);
    wr::<u32>(c, 8, 0);
    wr::<u32>(camera, 0x18, s10);
    wr::<u32>(camera, 0x1C, rd::<u32>(c, 0x14));
    wr::<u32>(camera, 0x10, rd::<u32>(c, 0x18));
    camera
}

/// `cb_WORLDOBJMODULE_RwCamera_Copy(dst, src, offset, size)` (0x750710): clears the copy's sector
/// array / capacity / count; when the source is in a world, updates the copy's frame objects (if it
/// has a frame) and stores the world. Returns `dst`.
#[hook(addr = 0x750710, conv = cdecl, name = "cb_WORLDOBJMODULE_RwCamera_Copy", expect = "8B 0D A4 25 C9 00")]
pub fn world_obj_camera_copy(dst: GPtr<u8>, src: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let d = ext(dst, CAMERA_WORLD_OFFSET);
    let s = ext(src, CAMERA_WORLD_OFFSET);
    wr::<u32>(d, 0, 0);
    wr::<u32>(d, 4, 0);
    wr::<u32>(d, 8, 0);
    let world = rd::<u32>(s, 0xC);
    if world != 0 {
        let d = ext(dst, CAMERA_WORLD_OFFSET);
        let f = rd::<u32>(dst, 4);
        if f != 0 {
            frame_update_objects(p8(f));
        }
        wr::<u32>(d, 0xC, world);
    }
    dst
}

/// `cb_WORLDOBJMODULE_RpAtomic_Ctor(atomic, offset, size)` (0x7507B0): world NULL, the atomic's
/// render frame (u16 +0x60) = `u16 [G+8] - 1`, saves the frame-sync callback (+0x10) and installs
/// 0x7507E0. Returns `atomic`.
#[hook(addr = 0x7507B0, conv = cdecl, name = "cb_WORLDOBJMODULE_RpAtomic_Ctor", expect = "8B 44 24 04 8B 0D A8 25 C9 00")]
pub fn world_obj_atomic_ctor(atomic: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let a = ext(atomic, ATOMIC_WORLD_OFFSET);
    wr::<u32>(a, 0, 0);
    let rf = rd::<u16>(rw_globals(), 8).wrapping_sub(1);
    wr::<u16>(atomic, 0x60, rf);
    wr::<u32>(a, 4, rd::<u32>(atomic, 0x10));
    wr::<u32>(atomic, 0x10, 0x7507E0);
    atomic
}

/// Unlinks every tie of the tie list at `obj + list` whose object and sector words are set (both
/// links), returning it to the freelist at `O + fl_off`. Ties with either word NULL stay linked.
pub(crate) fn free_ties(obj: GPtr<u8>, list: isize, fl_off: isize, check: bool) {
    let head = a32(obj.byte_add(list));
    let mut l = rd::<u32>(obj, list);
    while l != head {
        let tie = p8(l.wrapping_sub(0xC));
        let next = rd::<u32>(p8(l), 0);
        if !check || (rd::<u32>(tie, 8) != 0 && rd::<u32>(tie, 0x14) != 0) {
            // link at +0xC (next) / +0x10 (prev), then the link at +0 / +4
            let (n, p) = (rd::<u32>(tie, 0xC), rd::<u32>(tie, 0x10));
            wr::<u32>(p8(p), 0, n);
            let (n, p) = (rd::<u32>(tie, 0xC), rd::<u32>(tie, 0x10));
            wr::<u32>(p8(n), 4, p);
            let (n, p) = (rd::<u32>(tie, 0), rd::<u32>(tie, 4));
            wr::<u32>(p8(p), 0, n);
            let (n, p) = (rd::<u32>(tie, 0), rd::<u32>(tie, 4));
            wr::<u32>(p8(n), 4, p);
            rw_memory_free(p8(rd::<u32>(module(WORLDOBJ_MODULE), fl_off)), tie);
        }
        l = next;
    }
}

/// `cb_WORLDOBJMODULE_RpAtomic_Dtor(atomic, offset, size)` (0x7509B0): frees the atomic's ties (list
/// at +0x64; only ties whose object and sector are set) to the atomic-tie freelist, restores the saved
/// frame-sync callback. Returns `atomic`.
#[hook(addr = 0x7509B0, conv = cdecl, name = "cb_WORLDOBJMODULE_RpAtomic_Dtor", expect = "A1 A8 25 C9 00")]
pub fn world_obj_atomic_dtor(atomic: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let a = ext(atomic, ATOMIC_WORLD_OFFSET);
    free_ties(atomic, 0x64, 0, true);
    wr::<u32>(atomic, 0x10, rd::<u32>(a, 4));
    atomic
}

/// `cb_WORLDOBJMODULE_RpAtomic_Copy(dst, src, offset, size)` (0x7509A0): returns `dst`.
#[hook(addr = 0x7509A0, conv = cdecl, name = "cb_WORLDOBJMODULE_RpAtomic_Copy", expect = "8B 44 24 04 C3")]
pub fn world_obj_atomic_copy(dst: GPtr<u8>, _src: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    dst
}

/// `cb_WORLDOBJMODULE_RpClump_Ctor(clump, offset, size)` (0x750A30): world NULL, +4 = `O+8`. Returns
/// `clump`.
#[hook(addr = 0x750A30, conv = cdecl, name = "cb_WORLDOBJMODULE_RpClump_Ctor", expect = "8B 44 24 04 8B 0D AC 25 C9 00")]
pub fn world_obj_clump_ctor(clump: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let c = ext(clump, CLUMP_WORLD_OFFSET);
    wr::<u32>(c, 0, 0);
    wr::<u32>(c, 4, rd::<u32>(module(WORLDOBJ_MODULE), 8));
    clump
}

/// `cb_WORLDOBJMODULE_RpClump_Copy(dst, src, offset, size)` (0x750A60): when the source clump is in a
/// world, `RpWorldAddClump(world, dst)`. Returns `dst`.
#[hook(addr = 0x750A60, conv = cdecl, name = "cb_WORLDOBJMODULE_RpClump_Copy", expect = "8B 44 24 08 8B 0D AC 25 C9 00")]
pub fn world_obj_clump_copy(dst: GPtr<u8>, src: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let w = rd::<u32>(ext(src, CLUMP_WORLD_OFFSET), 0);
    if w != 0 {
        world_add_clump(p8(w), dst);
    }
    dst
}

/// `cb_WORLDOBJMODULE_RpClump_Dtor(clump, offset, size)` (0x750A90): returns `clump`.
#[hook(addr = 0x750A90, conv = cdecl, name = "cb_WORLDOBJMODULE_RpClump_Dtor", expect = "8B 44 24 04 C3")]
pub fn world_obj_clump_dtor(clump: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    clump
}

/// `cb_WORLDOBJMODULE_RpLight_Ctor(light, offset, size)` (0x750AA0): world NULL, saves the light's
/// frame-sync callback (+0x10) and installs 0x750AC0. Returns `light`.
#[hook(addr = 0x750AA0, conv = cdecl, name = "cb_WORLDOBJMODULE_RpLight_Ctor", expect = "8B 44 24 04 8B 0D B0 25 C9 00")]
pub fn world_obj_light_ctor(light: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let l = ext(light, LIGHT_WORLD_OFFSET);
    wr::<u32>(l, 0, 0);
    wr::<u32>(l, 4, rd::<u32>(light, 0x10));
    wr::<u32>(light, 0x10, 0x750AC0);
    light
}

/// `cb_WORLDOBJMODULE_RpLight_Dtor(light, offset, size)` (0x750D30): unlinks every tie of the light's
/// tie list (+0x2C; no object/sector check) and returns it to the light-tie freelist (`O+4`). The
/// saved frame-sync callback is not restored. Returns `light`.
#[hook(addr = 0x750D30, conv = cdecl, name = "cb_WORLDOBJMODULE_RpLight_Dtor", expect = "53 8B 5C 24 08")]
pub fn world_obj_light_dtor(light: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    free_ties(light, 0x2C, 4, false);
    light
}

/// `cb_WORLDOBJMODULE_RpLight_Copy(dst, src, offset, size)` (0x750CD0): when the source light is in a
/// world, the copy gets the world and is linked (link at +0x34) at the head of the world's directional
/// list (`world+0x3C`, subtype < 0x80) or, after `RwFrameUpdateObjects` of its frame when it has one,
/// of the local list (`world+0x34`). Returns `dst`.
#[hook(addr = 0x750CD0, conv = cdecl, name = "cb_WORLDOBJMODULE_RpLight_Copy", expect = "A1 B0 25 C9 00")]
pub fn world_obj_light_copy(dst: GPtr<u8>, src: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let off = g::<i32>(LIGHT_WORLD_OFFSET).read() as isize;
    let world = rd::<u32>(src, off);
    if world == 0 {
        return dst;
    }
    wr::<u32>(dst, off, world);
    let list = if rd::<u8>(dst, 1) < 0x80 {
        world.wrapping_add(0x3C)
    } else {
        let f = rd::<u32>(dst, 4);
        if f != 0 {
            frame_update_objects(p8(f));
        }
        world.wrapping_add(0x34)
    };
    let list = p8(list);
    let link = dst.byte_add(0x34);
    let first = rd::<u32>(list, 0);
    wr::<u32>(dst, 0x38, a32(list));
    wr::<u32>(link, 0, first);
    wr::<u32>(p8(rd::<u32>(list, 0)), 4, a32(link));
    wr::<u32>(list, 0, a32(link));
    dst
}

/// `cb_CLUMPMODULE_RwEngine_Close(instance, offset, size)` (0x749480): destroys every used atomic
/// (`RwFreeListForAllUsed(atomic freelist, 0x749520, NULL)`) and clump (`.., 0x7496A0, NULL`), then
/// destroys the clump freelist and the atomic freelist, clears both and decrements the clump module
/// count. Returns `instance`.
#[hook(addr = 0x749480, conv = cdecl, name = "cb_CLUMPMODULE_RwEngine_Close", expect = "A1 AC 24 C9 00")]
pub fn clump_module_close(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    free_list_for_all_used(p8(rd::<u32>(module(CLUMP_MODULE), 4)), 0x749520, 0);
    free_list_for_all_used(p8(rd::<u32>(module(CLUMP_MODULE), 0)), 0x7496A0, 0);
    free_list_destroy(p8(rd::<u32>(module(CLUMP_MODULE), 0)));
    free_list_destroy(p8(rd::<u32>(module(CLUMP_MODULE), 4)));
    wr::<u32>(module(CLUMP_MODULE), 0, 0);
    wr::<u32>(module(CLUMP_MODULE), 4, 0);
    let c = g::<i32>(CLUMP_INSTANCES);
    c.write(c.read().wrapping_sub(1));
    instance
}

orig! {
    /// The camera's previous begin/end update callback (stored in its world extension)
    fn call_camera_update(f: u32, camera: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
}

orig! {
    /// The camera's previous frame-sync callback (world extension +0x18)
    fn call_camera_sync(f: u32, camera: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
    /// `RwFrameGetLTM(frame)` (0x7F0990)
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => crate::frame::get_ltm;
    /// A standard function `(out, inout, n)` called through its stored pointer
    fn call_std(f: u32, out: u32, inout: u32, n: i32) -> i32 = cdecl dyn => unported;
    /// `G->rwmalloc` (+0x134) / `rwrealloc` (+0x13C)
    fn sync_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl table 0xC97B24 0x134 => unported;
    fn sync_realloc(p: GPtr<u8>, size: u32, hint: u32) -> GPtr<u8> = cdecl table 0xC97B24 0x13C => unported;
}

/// `_rpWorldObjCameraSync(camera)` (0x750480, the world-object camera frame-sync callback): the
/// previous sync first (0 → 0). With a world, the camera's sector list (extension +0 array, +4
/// capacity, +8 count) is rebuilt by walking the world's sector tree: a leaf (type < 0) is listed
/// when no frustum plane (camera +0x94, 0x14 each: normal, distance, closest-corner indices) has
/// its closest bounding-box corner in front (`n·c - d` as a float whose bits are > 0); a plane
/// sector (type = axis offset) is descended by the camera frustum's box (camera +0x10C sup, +0x118
/// inf) against the sector's left/right values, both sides when the box straddles them (the near
/// side first: the side holding the camera position, reversed for back-to-front worlds). The list
/// grows by 50 (`(capacity + 50) * 4`, hint 0x1030507; failure reports `{2, E_RW_NOMEM}` and keeps
/// what was listed). Finally the stored `RENDERF2B` hint standard function (`G + 0xA0`, read
/// before the walk) gets `(0, 0, renderOrder == 1)`. Returns the camera. The arithmetic is f32 (the
/// x87 runs at 24-bit precision here). Written by STD-3, unverified.
#[hook(addr = 0x750480, conv = cdecl, name = "_rpWorldObjCameraSync", expect = "A1 A4 25 C9 00 81 EC 44 01 00 00")]
pub fn world_camera_sync(camera: GPtr<u8>) -> GPtr<u8> {
    let ext = camera.byte_add(g::<i32>(CAMERA_WORLD_OFFSET).read() as isize);
    if call_camera_sync(rd::<u32>(ext, 0x18), camera).is_null() {
        return GPtr::NULL;
    }
    let world = p8(rd::<u32>(ext, 0xC));
    if world.is_null() {
        return camera;
    }
    let hint = rd::<u32>(rw_globals(), 0xA0);
    let ltm = frame_get_ltm(p8(rd::<u32>(camera, 4)));
    let f = |p: GPtr<u8>, o: isize| f32::from_bits(rd::<u32>(p, o));
    let pos = [f(ltm, 0x30), f(ltm, 0x34), f(ltm, 0x38)];
    let inf = [f(camera, 0x118), f(camera, 0x11C), f(camera, 0x120)];
    let sup = [f(camera, 0x10C), f(camera, 0x110), f(camera, 0x114)];
    let b2f = rd::<u32>(world, 0xC) == 2;
    let mut count: u32 = 0;
    // the original keeps the pending sectors in a frame array (index 0 never written)
    let mut stack = [0u32; 64];
    let mut top: usize = 0;
    let mut sector = p8(rd::<u32>(world, 0x1C));
    let mut failed = false;
    loop {
        let ty = rd::<i32>(sector, 0);
        if ty < 0 {
            let mut inside = true;
            for i in 0..6isize {
                let pl = camera.byte_add(0x94 + 0x14 * i);
                let c = |k: isize| rd::<u8>(pl, 0x10 + k) as isize;
                let x = f(sector, 0x48 + 12 * c(0));
                let y = f(sector, 0x4C + 12 * c(1));
                let z = f(sector, 0x50 + 12 * c(2));
                let v = f(pl, 4) * y + f(pl, 0) * x + f(pl, 8) * z - f(pl, 0xC);
                if (v.to_bits() as i32) > 0 {
                    inside = false;
                    break;
                }
            }
            if inside {
                if count >= rd::<u32>(ext, 4) {
                    let size = rd::<u32>(ext, 4).wrapping_mul(4).wrapping_add(0xC8);
                    let old = p8(rd::<u32>(ext, 0));
                    let n = if old.is_null() { sync_malloc(size, 0x103_0507) } else { sync_realloc(old, size, 0x103_0507) };
                    if n.is_null() {
                        let mut e = crate::common::RwError { plugin_id: 2, error_code: 0 };
                        e.error_code = crate::common::rw_error2(0x8000_0013, size) as i32;
                        crate::common::rw_error_set(GPtr::from_raw(&mut e));
                        failed = true;
                    } else {
                        wr::<u32>(ext, 0, n.addr() as u32);
                        wr::<u32>(ext, 4, rd::<u32>(ext, 4).wrapping_add(0x32));
                    }
                }
                if failed {
                    break;
                }
                wr::<u32>(p8(rd::<u32>(ext, 0)), 4 * count as isize, sector.addr() as u32);
                count += 1;
            }
            sector = p8(stack[top]);
            if top == 0 {
                break;
            }
            top -= 1;
            continue;
        }
        let a = (ty / 4) as usize;
        let t1 = inf[a] - f(sector, 0x10);
        let t2 = f(sector, 0x14) - sup[a];
        if (t1.to_bits() as i32) < 0 && (t2.to_bits() as i32) < 0 {
            let beyond = pos[a] > f(sector, 4);
            if beyond != b2f {
                top += 1;
                stack[top] = rd::<u32>(sector, 8);
                sector = p8(rd::<u32>(sector, 0xC));
            } else {
                top += 1;
                stack[top] = rd::<u32>(sector, 0xC);
                sector = p8(rd::<u32>(sector, 8));
            }
        } else if (t1.to_bits() as i32) >= 0 {
            sector = p8(rd::<u32>(sector, 0xC));
        } else {
            sector = p8(rd::<u32>(sector, 8));
        }
    }
    wr::<u32>(ext, 8, count);
    call_std(hint, 0, 0, (rd::<u32>(world, 0xC) == 1) as i32);
    camera
}

/// The world camera begin update (0x750430, installed by the world-object camera constructor):
/// `G->curWorld` (+4) = the camera's world (extension +0xC), `G->renderFrame` (u16 +8) += 1, then
/// a tail jump to the previous begin update (extension +0x10). Written by STD-3, unverified.
#[hook(addr = 0x750430, conv = cdecl, name = "_rpWorldObjCameraBeginUpdate", expect = "8B 4C 24 04 A1 A4 25 C9 00")]
pub fn world_camera_begin_update(camera: GPtr<u8>) -> GPtr<u8> {
    let ext = camera.byte_add(g::<i32>(CAMERA_WORLD_OFFSET).read() as isize);
    wr::<u32>(rw_globals(), 4, rd::<u32>(ext, 0xC));
    let f = rw_globals().byte_add(8).cast::<u16>();
    f.write(f.read().wrapping_add(1));
    call_camera_update(rd::<u32>(ext, 0x10), camera)
}

/// The world camera end update (0x750460): `G->curWorld = NULL`, then a tail jump to the previous
/// end update (extension +0x14). Written by STD-3, unverified.
#[hook(addr = 0x750460, conv = cdecl, name = "_rpWorldObjCameraEndUpdate", expect = "8B 15 24 7B C9 00")]
pub fn world_camera_end_update(camera: GPtr<u8>) -> GPtr<u8> {
    let ext = camera.byte_add(g::<i32>(CAMERA_WORLD_OFFSET).read() as isize);
    wr::<u32>(rw_globals(), 4, 0);
    call_camera_update(rd::<u32>(ext, 0x14), camera)
}

/// `RpWorldSetDefaultSectorPipeline(pipeline)` (0x75E270): the pipe module's default sector
/// pipeline (+0x40) becomes `pipeline`, or the generic one (+0x58) for NULL; returns it (STD-3,
/// written, unverified).
#[hook(addr = 0x75E270, conv = cdecl, name = "RpWorldSetDefaultSectorPipeline", expect = "8B 44 24 04 85 C0")]
pub fn world_set_default_sector_pipeline(p: GPtr<u8>) -> GPtr<u8> {
    let m = rw_globals().byte_add(g::<u32>(0xC9BC60).read() as isize);
    let v = if p.is_null() { rd::<u32>(m, 0x58) } else { p.addr() as u32 };
    wr::<u32>(m, 0x40, v);
    p8(v)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_WORLD_SET_DEFAULT_SECTOR_PIPELINE,
    HOOK_WORLD_CAMERA_SYNC,
    HOOK_WORLD_CAMERA_BEGIN_UPDATE,
    HOOK_WORLD_CAMERA_END_UPDATE,
    HOOK_RP_WORLD_PLUGIN_ATTACH,
    HOOK_WORLD_MODULE_OPEN,
    HOOK_WORLD_MODULE_CLOSE,
    HOOK_SECTOR_MODULE_OPEN,
    HOOK_SECTOR_MODULE_CLOSE,
    HOOK_BINWORLD_MODULE_OPEN,
    HOOK_BINWORLD_MODULE_CLOSE,
    HOOK_RP_WORLD_OBJ_REGISTER_PLUGINS,
    HOOK_RP_WORLD_OBJ_DUMMY_TRUE,
    HOOK_WORLD_OBJ_MODULE_OPEN,
    HOOK_WORLD_OBJ_MODULE_CLOSE,
    HOOK_RP_WORLD_SECTOR_REGISTER_PLUGIN,
    HOOK_RP_WORLD_SECTOR_REGISTER_PLUGIN_STREAM,
    HOOK_RP_WORLD_REGISTER_PLUGIN,
    HOOK_RP_WORLD_REGISTER_PLUGIN_STREAM,
    HOOK_RP_WORLD_PIPE_OPEN,
    HOOK_RP_WORLD_PIPE_CLOSE,
    HOOK_RP_WORLD_PIPE_DUMMY_TRUE,
    HOOK_RP_WORLD_DEVICE_REGISTER_PLUGINS,
    HOOK_RP_WORLD_DEVICE_REGISTER_PLUGINS2,
    HOOK_WORLD_DEVICE_OBJ_CTOR,
    HOOK_WORLD_DEVICE_OBJ_DTOR,
    HOOK_WORLD_DEVICE_OBJ_COPY,
    HOOK_WORLD_DEVICE_MODULE_OPEN,
    HOOK_WORLD_DEVICE_MODULE_CLOSE,
    HOOK_WORLD_OBJ_CAMERA_CTOR,
    HOOK_WORLD_OBJ_CAMERA_DTOR,
    HOOK_WORLD_OBJ_CAMERA_COPY,
    HOOK_WORLD_OBJ_ATOMIC_CTOR,
    HOOK_WORLD_OBJ_ATOMIC_DTOR,
    HOOK_WORLD_OBJ_ATOMIC_COPY,
    HOOK_WORLD_OBJ_CLUMP_CTOR,
    HOOK_WORLD_OBJ_CLUMP_COPY,
    HOOK_WORLD_OBJ_CLUMP_DTOR,
    HOOK_WORLD_OBJ_LIGHT_CTOR,
    HOOK_WORLD_OBJ_LIGHT_DTOR,
    HOOK_WORLD_OBJ_LIGHT_COPY,
    HOOK_CLUMP_MODULE_CLOSE,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_RP_WORLD_PIPE_DUMMY_VOID];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = WORLDPLUGIN_GLOBALS;
