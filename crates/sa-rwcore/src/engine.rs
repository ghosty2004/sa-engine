//! RW engine life cycle (STD-3): device system requests, `RwEngineInit`/`Open`/`Start`/`Stop`/
//! `Close`/`Term`, the subsystem and video-mode queries, the error module, the string and file
//! function tables, and the small engine-module callbacks `RwEngineInit` registers (colour, pipe,
//! chunk group). Ported from the 1.0 US bytes; `docs/re/specs/RW/RwEngine.md` is the reading aid.
//! Every port here is written, not yet verified (phase A verification queue).
//!
//! `G` = `[0xC97B24]`. The static `RwGlobals` image at 0xC979C8 (0x158 bytes = 0x56 dwords) is the
//! engine instance until `RwEngineOpen` allocates the real one (copied from the image, plugin space
//! beyond 0x158 left uninitialised) and is copied back by `RwEngineClose`. `G + 0x10` is the
//! engine's copy of the `RwDevice` (fpSystem at +4), `G + 0x48` the standard functions (29),
//! `G + 0xC4..` the file functions, `G + 0xF0..` the string functions, `G + 0x134` the memory
//! functions, `G + 0x150` the engine status (0 none, 1 initialised, 2 opened, 3 started).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, native, orig};

use crate::common::{RW_ENGINE_INSTANCE, RwError, g, p8, rd, rw_globals, wr};

/// The static `RwGlobals` image.
const STATIC_GLOBALS: u32 = 0xC979C8;
/// Its size in dwords.
const GLOBALS_DWORDS: usize = 0x56;
/// Engine plugin registry (`sizeOfStruct` first).
const ENGINE_REGISTRY: u32 = 0x8E2298;
/// Number of open engine instances.
const NUM_INSTANCES: usize = 0xC97B20;
/// The static image's `memoryAlloc`/`memoryFree` (+0x144/+0x148).
const STATIC_MEMORY_ALLOC: usize = 0xC97B0C;
const STATIC_MEMORY_FREE: usize = 0xC97B10;
/// "Only rendering sub system" (`.data`).
const SUBSYSTEM_NAME: u32 = 0x8E23B0;
/// ERRORMODULE: offset global and instance counter.
const ERROR_OFFSET: usize = 0xC9A6F0;
const ERROR_INSTANCES: usize = 0xC9A6F4;
/// COLORMODULE instance counter.
const COLOR_INSTANCES: usize = 0xC9A728;
/// PIPEMODULE offset global.
const PIPE_OFFSET: usize = 0xC9BC60;
/// CHUNKGROUPMODULE: offset global, instance counter, freelist parameters, in-place space.
const CHUNK_OFFSET: usize = 0xC9A6F8;
const CHUNK_INSTANCES: usize = 0xC9A6FC;
const CHUNK_FL_PER_BLOCK: usize = 0x8E26D8;
const CHUNK_FL_BLOCKS: usize = 0x8E26DC;
const CHUNK_FL_SPACE: u32 = 0xC9A700;

/// `RwEngineStatus`.
const STATUS: isize = 0x150;
/// `RwGlobals::dOpenDevice`.
const OPEN_DEVICE: isize = 0x10;

/// Device system requests.
mod req {
    pub const OPEN: i32 = 0;
    pub const CLOSE: i32 = 1;
    pub const START: i32 = 2;
    pub const STOP: i32 = 3;
    pub const REGISTER: i32 = 4;
    pub const GET_NUM_MODES: i32 = 5;
    pub const USE_MODE: i32 = 7;
    pub const STANDARDS: i32 = 0xB;
    pub const GET_NUM_SUBSYSTEMS: i32 = 0xD;
    pub const GET_SUBSYSTEM_INFO: i32 = 0xE;
    pub const GET_CURRENT_SUBSYSTEM: i32 = 0xF;
    pub const SET_SUBSYSTEM: i32 = 0x10;
    pub const FINALIZE_START: i32 = 0x11;
    pub const INITIATE_STOP: i32 = 0x12;
}

orig! {
    /// `RwDevice::fpSystem(request, pOut, pInOut, nIn)` (stored pointer)
    fn device_system(f: u32, request: i32, out: GPtr<u8>, inout: GPtr<u8>, n: i32) -> i32 = cdecl dyn => unported;
    /// `G->stringFuncs.rwstrcpy` (+0xF8)
    fn rw_strcpy(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl table 0xC97B24 0xF8 => unported;
    /// `G->memoryFuncs.rwmalloc` (+0x134)
    fn rw_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl table 0xC97B24 0x134 => unported;
    /// the static image's `rwfree` (`call [0xC97B00]`)
    fn static_rwfree(p: GPtr<u8>) = cdecl cell 0xC97B00 => unported;
    /// `_rwerror(code, a)` (0x8088D0)
    fn rw_error2(code: u32, a: u32) -> u32 = cdecl 0x8088D0 => unported;
    /// `_rwerror(code)` (0x8088D0)
    fn rw_error1(code: u32) -> u32 = cdecl 0x8088D0 => rw_error;
    /// `RwErrorSet(const RwError*)` (0x808820)
    fn error_set(e: GPtr<RwError>) -> GPtr<RwError> = cdecl 0x808820 => rw_error_set;
    /// `_rwDeviceSystemRequest` (0x7F2AB0)
    fn system_request(dev: GPtr<u8>, request: i32, out: GPtr<u8>, inout: GPtr<u8>, n: i32) -> i32 = cdecl 0x7F2AB0 => device_system_request;
    /// `_rwPluginRegistryInitObject(reg, obj)` (0x8086E0)
    fn registry_init_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x8086E0 => crate::plugin::registry_init_object;
    /// `_rwPluginRegistryDeInitObject(reg, obj)` (0x808740)
    fn registry_deinit_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808740 => crate::plugin::registry_deinit_object;
    /// `_rwPluginRegistryAddPlugin(reg, size, id, ctor, dtor, copy)` (0x8084A0)
    fn registry_add_plugin(reg: GPtr<u8>, size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x8084A0 => crate::plugin::registry_add_plugin;
    /// `RwImageSetGamma(RwReal)` (0x803FE0)
    fn image_set_gamma(gamma: f32) -> i32 = cdecl 0x803FE0 => unported;
    /// `_rwDeviceGetHandle()` (0x7F9C20, D3D9 driver)
    fn device_get_handle() -> GPtr<u8> = cdecl 0x7F9C20 => unported;
    /// `_rwFreeListSetFlags(flag)` (0x801970)
    fn freelist_set_flags(flag: u32) = cdecl 0x801970 => crate::memory::rw_freelist_set_flags;
    /// `_rwStringOpen()` (0x80A240)
    fn string_open() -> i32 = cdecl 0x80A240 => rw_string_open;
    /// `_rwStringClose()` (0x80A440)
    fn string_close() = cdecl 0x80A440 => rw_string_close;
    /// `_rwMemoryOpen(const RwMemoryFunctions*)` (0x801FD0)
    fn memory_open(funcs: GPtr<u8>) -> u32 = cdecl 0x801FD0 => crate::memory::rw_memory_open;
    /// `_rwMemoryClose()` (0x8020F0)
    fn memory_close() = cdecl 0x8020F0 => unported;
    /// `_rwFileSystemOpen()` (0x804140)
    fn file_system_open() -> i32 = cdecl 0x804140 => rw_file_system_open;
    /// `_rwFileSystemClose()` (0x804240)
    fn file_system_close() = cdecl 0x804240 => rw_file_system_close;
    /// `_rwResourcesOpen()` (0x8082E0)
    fn resources_open() -> u32 = cdecl 0x8082E0 => crate::memory::rw_resources_open;
    /// `_rwResourcesClose()` (0x808320)
    fn resources_close() = cdecl 0x808320 => unported;
    /// `_rwPipeAttachDummy()` (0x807C70)
    fn pipe_attach_dummy() -> i32 = cdecl 0x807C70 => rw_pipe_attach_dummy;
    /// `_rwDeviceRegisterPlugin()` (0x7F5F60, `jmp _rwD3D9RasterPluginAttach`)
    fn device_register_plugin() -> i32 = cdecl 0x7F5F60 => rw_device_register_plugin;
    /// `_rwD3D9RasterPluginAttach()` (0x4C9AB0)
    fn d3d9_raster_plugin_attach() -> i32 = cdecl 0x4C9AB0 => unported;
    /// `_rxPipelineOpen()` (0x804FE0)
    fn rx_pipeline_open() -> i32 = cdecl 0x804FE0 => unported;
    /// `_rxPipelineClose()` (0x804F60)
    fn rx_pipeline_close() -> i32 = cdecl 0x804F60 => unported;
    /// `RwFreeListCreateAndPreallocateSpace(...)` (0x801B70)
    fn freelist_create_prealloc(size: u32, per_block: u32, align: u32, n: u32, space: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl 0x801B70 => crate::memory::rw_freelist_create_and_preallocate_space;
    /// `RwFreeListDestroy(fl)` (0x801B80)
    fn freelist_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => crate::memory::rw_freelist_destroy;
}

fn status() -> i32 {
    rd::<i32>(rw_globals(), STATUS)
}
fn set_status(s: i32) {
    wr::<i32>(rw_globals(), STATUS, s)
}
fn open_device() -> GPtr<u8> {
    rw_globals().byte_add(OPEN_DEVICE)
}
fn set_globals(p: u32) {
    g::<u32>(RW_ENGINE_INSTANCE).write(p)
}
/// `rep movsd` of the 0x56-dword `RwGlobals` image.
fn copy_globals(dst: GPtr<u8>, src: GPtr<u8>) {
    for i in 0..GLOBALS_DWORDS {
        let v = src.cast::<u32>().wrapping_index(i as i32).read();
        dst.cast::<u32>().wrapping_index(i as i32).write(v);
    }
}
/// `RwErrorSet({1, code})` with `code` already returned by `_rwerror`.
fn set_error(code: u32) {
    let mut e = RwError {
        plugin_id: 1,
        error_code: code as i32,
    };
    error_set(GPtr::from_raw(&mut e));
}

// ------------------------------------------------------------------ device requests ----------

/// `_rwDeviceSystemRequest(RwDevice* dev, RwInt32 request, void* pOut, void* pInOut, RwInt32 nIn)`
/// (0x7F2AB0): the device's `fpSystem` first; when it declines (0), the defaults for the subsystem
/// requests (one subsystem "Only rendering sub system", current 0, only index 0 settable,
/// FINALIZESTART/INITIATESTOP succeed); any other declined request sets
/// `{1, _rwerror(E_RW_SYSTEMREQUEST 0x18, request)}` and returns 0.
#[hook(addr = 0x7F2AB0, conv = cdecl, name = "_rwDeviceSystemRequest", expect = "83 EC 08 8B 44 24 18")]
pub fn device_system_request(dev: GPtr<u8>, request: i32, out: GPtr<u8>, inout: GPtr<u8>, n: i32) -> i32 {
    let r = device_system(rd::<u32>(dev, 4), request, out, inout, n);
    if r != 0 {
        return r;
    }
    let ok = match request {
        req::GET_NUM_SUBSYSTEMS => {
            out.cast::<i32>().write(1);
            return 1;
        }
        req::GET_SUBSYSTEM_INFO => {
            let ok = (n == 0) as i32;
            if ok != 0 {
                rw_strcpy(out, p8(SUBSYSTEM_NAME));
            }
            ok
        }
        req::GET_CURRENT_SUBSYSTEM => {
            out.cast::<i32>().write(0);
            return 1;
        }
        req::SET_SUBSYSTEM => (n == 0) as i32,
        req::FINALIZE_START | req::INITIATE_STOP => return 1,
        _ => 0,
    };
    if ok == 0 {
        set_error(rw_error2(0x18, request as u32));
    }
    ok
}

/// `_rwGetNumEngineInstances()` (0x7F2B90): `[0xC97B20]`.
#[hook(addr = 0x7F2B90, conv = cdecl, name = "FUN_007f2b90", expect = "A1 20 7B C9 00")]
pub fn get_num_engine_instances() -> i32 {
    g::<i32>(NUM_INSTANCES).read()
}

/// `RwEngineGetNumSubSystems()` (0x7F2C00): request GETNUMSUBSYSTEMS into a local preset to 1
/// (the request's result is ignored).
#[hook(addr = 0x7F2C00, conv = cdecl, name = "RwEngineGetNumSubSystems", expect = "51 8B 0D 24 7B C9 00")]
pub fn engine_get_num_sub_systems() -> i32 {
    let mut n: i32 = 1;
    system_request(open_device(), req::GET_NUM_SUBSYSTEMS, GPtr::from_raw(&mut n).cast(), GPtr::NULL, 0);
    n
}

/// `RwEngineGetSubSystemInfo(RwSubSystemInfo* info, RwInt32 index)` (0x7F2C30): `info`, or NULL
/// when the request fails.
#[hook(addr = 0x7F2C30, conv = cdecl, name = "RwEngineGetSubSystemInfo", expect = "8B 44 24 08 8B 0D 24 7B C9 00")]
pub fn engine_get_sub_system_info(info: GPtr<u8>, index: i32) -> GPtr<u8> {
    if system_request(open_device(), req::GET_SUBSYSTEM_INFO, info, GPtr::NULL, index) != 0 {
        info
    } else {
        GPtr::NULL
    }
}

/// The getters that request into an uninitialised local and return it, or -1 on failure
/// (`RwEngineGetCurrentSubSystem`, `RwEngineGetNumVideoModes`, `RwEngineGetCurrentVideoMode`).
/// The port's local starts at 0 (the original's is stack garbage, only read when a device leaves
/// it unwritten).
fn query_local(request: i32) -> i32 {
    let mut v: i32 = 0;
    if system_request(open_device(), request, GPtr::from_raw(&mut v).cast(), GPtr::NULL, 0) != 0 {
        v
    } else {
        -1
    }
}

/// `RwEngineGetCurrentSubSystem()` (0x7F2C60).
#[hook(addr = 0x7F2C60, conv = cdecl, name = "RwEngineGetCurrentSubSystem", expect = "51 8B 0D 24 7B C9 00")]
pub fn engine_get_current_sub_system() -> i32 {
    query_local(req::GET_CURRENT_SUBSYSTEM)
}

/// `RwEngineSetSubSystem(RwInt32 index)` (0x7F2C90): 1 if the request succeeds, else 0.
#[hook(addr = 0x7F2C90, conv = cdecl, name = "RwEngineSetSubSystem", expect = "8B 44 24 04 8B 0D 24 7B C9 00")]
pub fn engine_set_sub_system(index: i32) -> i32 {
    (system_request(open_device(), req::SET_SUBSYSTEM, GPtr::NULL, GPtr::NULL, index) != 0) as i32
}

/// `RwEngineGetNumVideoModes()` (0x7F2CC0).
#[hook(addr = 0x7F2CC0, conv = cdecl, name = "RwEngineGetNumVideoModes", expect = "51 8B 0D 24 7B C9 00")]
pub fn engine_get_num_video_modes() -> i32 {
    query_local(req::GET_NUM_MODES)
}

/// `RwEngineSetVideoMode(RwInt32 index)` (0x7F2D50): 1 if the request succeeds, else 0.
#[hook(addr = 0x7F2D50, conv = cdecl, name = "RwEngineSetVideoMode", expect = "8B 44 24 04 8B 0D 24 7B C9 00")]
pub fn engine_set_video_mode(index: i32) -> i32 {
    (system_request(open_device(), req::USE_MODE, GPtr::NULL, GPtr::NULL, index) != 0) as i32
}

// ------------------------------------------------------------------ life cycle ---------------

/// `RwEngineStop()` (0x7F2E20): INITIATESTOP, de-initialise the engine plugins, STOP; when STOP
/// succeeds the status goes back to 2 (opened). Returns STOP's result.
#[hook(addr = 0x7F2E20, conv = cdecl, name = "RwEngineStop", expect = "A1 24 7B C9 00 56")]
pub fn engine_stop() -> i32 {
    let dev = open_device();
    system_request(dev, req::INITIATE_STOP, GPtr::NULL, GPtr::NULL, 0);
    registry_deinit_object(p8(ENGINE_REGISTRY), rw_globals());
    let r = system_request(dev, req::STOP, GPtr::NULL, GPtr::NULL, 0);
    if r != 0 {
        set_status(2);
    }
    r
}

/// `RwEngineStart()` (0x7F2E70): START, initialise every engine plugin (the module Open
/// callbacks, in registration order; on failure STOP and return 0), `RwImageSetGamma(device
/// gamma)`, FINALIZESTART (result ignored), status 3. No status check on entry. The device pointer
/// is `G + 0x10` read once at entry (ESI), as the original.
#[hook(addr = 0x7F2E70, conv = cdecl, name = "RwEngineStart", expect = "A1 24 7B C9 00 56")]
pub fn engine_start() -> i32 {
    let dev = open_device();
    if system_request(dev, req::START, GPtr::NULL, GPtr::NULL, 0) == 0 {
        return 0;
    }
    if registry_init_object(p8(ENGINE_REGISTRY), rw_globals()).is_null() {
        system_request(dev, req::STOP, GPtr::NULL, GPtr::NULL, 0);
        return 0;
    }
    image_set_gamma(rd::<f32>(rw_globals(), OPEN_DEVICE));
    system_request(dev, req::FINALIZE_START, GPtr::NULL, GPtr::NULL, 0);
    set_status(3);
    1
}

/// `RwEngineClose()` (0x7F2F00): CLOSE; on success copy the instance back into the static image,
/// make the image the instance again, free the instance through the image's `rwfree`, one engine
/// instance fewer, status 1. Returns CLOSE's result.
#[hook(addr = 0x7F2F00, conv = cdecl, name = "RwEngineClose", expect = "A1 24 7B C9 00 53")]
pub fn engine_close() -> i32 {
    let r = system_request(open_device(), req::CLOSE, GPtr::NULL, GPtr::NULL, 0);
    if r == 0 {
        return 0;
    }
    let inst = rw_globals();
    set_globals(STATIC_GLOBALS);
    copy_globals(p8(STATIC_GLOBALS), inst);
    static_rwfree(inst);
    let n = g::<i32>(NUM_INSTANCES);
    n.write(n.read().wrapping_sub(1));
    set_status(1);
    r
}

/// `RwEngineOpen(RwEngineOpenParams* params)` (0x7F2F70); see the module docs. The instance is
/// allocated with the engine registry's `sizeOfStruct` (hint 0x40000), the static image copied
/// in (plugin space left as allocated), then the device gets REGISTER (result ignored), OPEN with
/// `params` (failure: copy back, free, return 0) and STANDARDS into `G->stdFunc` (29 entries).
#[hook(addr = 0x7F2F70, conv = cdecl, name = "RwEngineOpen", expect = "A1 24 7B C9 00 83 EC 08")]
pub fn engine_open(params: GPtr<u8>) -> i32 {
    if g::<u32>(RW_ENGINE_INSTANCE).read() == 0 {
        set_globals(STATIC_GLOBALS);
    }
    if status() != 1 {
        set_error(rw_error1(0x8000_0001));
        return 0;
    }
    if params.is_null() {
        set_error(rw_error1(0x8000_0016));
        return 0;
    }
    let dev = device_get_handle();
    if dev.is_null() {
        return 0;
    }
    let size = g::<u32>(ENGINE_REGISTRY as usize).read();
    let inst = rw_malloc(size, 0x40000);
    set_globals(inst.addr() as u32);
    if inst.is_null() {
        set_globals(STATIC_GLOBALS);
        set_error(rw_error2(0x8000_0013, g::<u32>(ENGINE_REGISTRY as usize).read()));
        return 0;
    }
    copy_globals(inst, p8(STATIC_GLOBALS));
    let gl = rw_globals();
    system_request(dev, req::REGISTER, gl.byte_add(OPEN_DEVICE), gl.byte_add(0x134), 0);
    if system_request(dev, req::OPEN, GPtr::NULL, params, 0) == 0 {
        set_globals(STATIC_GLOBALS);
        copy_globals(p8(STATIC_GLOBALS), inst);
        static_rwfree(inst);
        return 0;
    }
    system_request(dev, req::STANDARDS, rw_globals().byte_add(0x48), GPtr::NULL, 0x1D);
    let n = g::<i32>(NUM_INSTANCES);
    n.write(n.read().wrapping_add(1));
    set_status(2);
    1
}

/// `RwEngineTerm()` (0x7F3130): only with no open engine instance: close resources, the file
/// system and memory, status 0; returns whether it did.
#[hook(addr = 0x7F3130, conv = cdecl, name = "RwEngineTerm", expect = "8B 0D 20 7B C9 00")]
pub fn engine_term() -> i32 {
    let ok = (g::<i32>(NUM_INSTANCES).read() == 0) as i32;
    if ok != 0 {
        resources_close();
        file_system_close();
        memory_close();
        set_status(0);
    }
    ok
}

/// The core engine modules `RwEngineInit` registers: (size, id, open, close).
const CORE_MODULES_1: [(i32, u32, u32, u32); 12] = [
    (8, 0x40F, 0x8087D0, 0x808810),      // ERROR
    (0x18, 0x401, 0x7EDE90, 0x7EDE20),   // VECTOR
    (0, 0x40D, 0x80AA40, 0x80AA50),      // COLOR
    (0x18, 0x402, 0x7F16C0, 0x7F1660),   // MATRIX
    (4, 0x403, 0x7EFEF0, 0x7EFF70),      // FRAME
    (4, 0x404, 0x7EC780, 0x7EC7E0),      // STREAM
    (4, 0x405, 0x7EE110, 0x7EE0B0),      // CAMERA
    (0x220, 0x406, 0x802280, 0x8024E0),  // IMAGE
    (0x64, 0x407, 0x7FB370, 0x7FB310),   // RASTER
    (0x34, 0x408, 0x7F3EA0, 0x7F3D00),   // TEXTURE
    (0x60, 0x409, 0x807C40, 0x807C60),   // PIPE
    (4, 0x412, 0x80A780, 0x80A7E0),      // CHUNKGROUP
];
const CORE_MODULES_2: [(i32, u32, u32, u32); 2] = [
    (0x74, 0x40A, 0x7EFE20, 0x7EFDE0),   // IMMEDIATE
    (0x28, 0x40B, 0x807C90, 0x807D80),   // RESOURCES
];

/// `RwEngineInit(const RwMemoryFunctions* memFuncs, RwUInt32 initFlags, RwUInt32 resArenaSize)`
/// (0x7F3170): the static image becomes the instance; `initFlags & 1` (low byte) selects the
/// freelist bypass (`memoryAlloc`/`memoryFree` = 0x7F3490/0x7F34B0, freelist flags 0) or the real
/// freelists (0x801C30/0x801D50, flags 1); the arena size is stored before the status check (a
/// non-zero status returns 0). Then string, memory, file system and resources open (each failure
/// closes the earlier ones in reverse and returns 0), the core modules are registered (every
/// registration runs; their results and `_rwPipeAttachDummy`'s are OR'ed and a negative OR fails
/// with the full cleanup), `_rwDeviceRegisterPlugin`; status 1 and its result.
#[hook(addr = 0x7F3170, conv = cdecl, name = "RwEngineInit", expect = "8A 44 24 08 83 EC 18")]
pub fn engine_init(mem_funcs: GPtr<u8>, init_flags: u32, res_arena_size: u32) -> i32 {
    set_globals(STATIC_GLOBALS);
    if init_flags as u8 & 1 != 0 {
        g::<u32>(STATIC_MEMORY_ALLOC).write(0x7F3490);
        g::<u32>(STATIC_MEMORY_FREE).write(0x7F34B0);
        freelist_set_flags(0);
    } else {
        g::<u32>(STATIC_MEMORY_ALLOC).write(0x801C30);
        g::<u32>(STATIC_MEMORY_FREE).write(0x801D50);
        freelist_set_flags(1);
    }
    wr::<u32>(rw_globals(), 0x154, res_arena_size);
    if status() != 0 {
        return 0;
    }
    if string_open() == 0 {
        return 0;
    }
    if memory_open(mem_funcs) == 0 {
        string_close();
        return 0;
    }
    if file_system_open() == 0 {
        memory_close();
        string_close();
        return 0;
    }
    if resources_open() == 0 {
        file_system_close();
        memory_close();
        string_close();
        return 0;
    }
    let reg = p8(ENGINE_REGISTRY);
    let mut acc: i32 = 0;
    for (size, id, open, close) in CORE_MODULES_1 {
        acc |= registry_add_plugin(reg, size, id, open, close, 0);
    }
    acc |= pipe_attach_dummy();
    for (size, id, open, close) in CORE_MODULES_2 {
        acc |= registry_add_plugin(reg, size, id, open, close, 0);
    }
    let r = if acc >= 0 { device_register_plugin() } else { 0 };
    if r == 0 {
        resources_close();
        file_system_close();
        memory_close();
        string_close();
        return 0;
    }
    set_status(1);
    r
}

/// `_rwDeviceRegisterPlugin()` (0x7F5F60): a `jmp _rwD3D9RasterPluginAttach` (the D3D9 build's
/// device plugin registration). Written by STD-3, unverified.
// Not hooked: a 5-byte `jmp` into .text (sa-hook only takes jmp stubs into .HOODLUM).
#[native(addr = 0x7F5F60, conv = cdecl, name = "_rwDeviceRegisterPlugin")]
pub fn rw_device_register_plugin() -> i32 {
    d3d9_raster_plugin_attach()
}

// ------------------------------------------------------------------ errors -------------------

/// `cb_ERRORMODULE_RwEngine_Open(instance, offset, size)` (0x8087D0): store the module offset,
/// count the instance, no pending error (`{0, E_RW_NOERROR 0x80000000}`); returns `instance`.
#[hook(addr = 0x8087D0, conv = cdecl, name = "cb_ERRORMODULE_RwEngine_Open", expect = "8B 0D F4 A6 C9 00")]
pub fn error_module_open(instance: GPtr<u8>, offset: i32, _size: i32) -> GPtr<u8> {
    let n = g::<i32>(ERROR_INSTANCES).read().wrapping_add(1);
    g::<i32>(ERROR_OFFSET).write(offset);
    g::<i32>(ERROR_INSTANCES).write(n);
    wr::<u32>(rw_globals(), offset as isize, 0);
    wr::<u32>(rw_globals(), g::<i32>(ERROR_OFFSET).read() as isize + 4, 0x8000_0000);
    instance
}

/// `cb_ERRORMODULE_RwEngine_Close(instance, offset, size)` (0x808810).
#[hook(addr = 0x808810, conv = cdecl, name = "cb_ERRORMODULE_RwEngine_Close", expect = "A1 F4 A6 C9 00")]
pub fn error_module_close(instance: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let n = g::<i32>(ERROR_INSTANCES);
    n.write(n.read().wrapping_sub(1));
    instance
}

fn error_module() -> GPtr<u8> {
    rw_globals().byte_add(g::<i32>(ERROR_OFFSET).read() as isize)
}

/// `RwErrorSet(RwError* err)` (0x808820): only when no error is pending (`{0, 0x80000000}`) the
/// error is stored, its plugin id cleared when the code has bit 31 set (a core error); otherwise the
/// new error is dropped. Returns `err`.
#[hook(addr = 0x808820, conv = cdecl, name = "RwErrorSet", expect = "8B 0D F0 A6 C9 00")]
pub fn rw_error_set(err: GPtr<RwError>) -> GPtr<RwError> {
    let m = error_module();
    if rd::<u32>(m, 0) != 0 || rd::<u32>(m, 4) != 0x8000_0000 {
        return err;
    }
    let e = err.read();
    let plugin = if e.error_code as u32 & 0x8000_0000 != 0 { 0 } else { e.plugin_id };
    wr::<i32>(m, 0, plugin);
    wr::<i32>(error_module(), 4, err.read().error_code);
    err
}

/// `RwErrorGet(RwError* out)` (0x808880): the pending error into `out`, then none pending.
#[hook(addr = 0x808880, conv = cdecl, name = "RwErrorGet", expect = "8B 0D F0 A6 C9 00")]
pub fn rw_error_get(out: GPtr<RwError>) -> GPtr<RwError> {
    let m = error_module();
    out.cast::<u32>().write(rd::<u32>(m, 0));
    out.cast::<u32>().wrapping_index(1).write(rd::<u32>(error_module(), 4));
    wr::<u32>(error_module(), 0, 0);
    wr::<u32>(error_module(), 4, 0x8000_0000);
    out
}

/// `_rwerror(RwInt32 code, ...)` (0x8088D0): returns `code` (the release build ignores the
/// format arguments; cdecl, so callers passing more arguments are served by this signature).
#[hook(addr = 0x8088D0, conv = cdecl, name = "_rwerror", expect = "8B 44 24 04 C3")]
pub fn rw_error(code: u32) -> u32 {
    code
}

// ------------------------------------------------------------------ string / file tables -----

/// `_rwStringOpen()` (0x80A240): fills `G->stringFuncs` (+0xF0..+0x130) with the CRT functions:
/// sprintf, vsprintf, strcpy, strncpy, strcat, strncat, strrchr, strchr, strstr, strcmp, strncmp,
/// stricmp, strlen, strupr, strlwr, strtok, sscanf. Returns 1.
#[hook(addr = 0x80A240, conv = cdecl, name = "_rwStringOpen", expect = "A1 24 7B C9 00")]
pub fn rw_string_open() -> i32 {
    const FUNCS: [u32; 17] = [
        0x821BB5, 0x8230B8, 0x826590, 0x821F40, 0x8265A0, 0x826450, 0x80A420, 0x80A400, 0x822650,
        0x8263C0, 0x8214D0, 0x80A350, 0x826330, 0x80A3A0, 0x80A3D0, 0x82244B, 0x8220AD,
    ];
    for (i, f) in FUNCS.iter().enumerate() {
        wr::<u32>(rw_globals(), 0xF0 + 4 * i as isize, *f);
    }
    1
}

/// `_rwStringClose()` (0x80A440): empty (a lone `ret`, not hookable).
#[native(addr = 0x80A440, conv = cdecl, name = "_rwStringClose")]
pub fn rw_string_close() {}

/// `_rwFileSystemOpen()` (0x804140): fills `G->fileFuncs` (+0xC4..+0xEC): rwfexist 0x804200 and
/// the CRT fopen, fclose, fread, fwrite, fgets, fputs, feof, fseek, fflush, ftell. Returns 1.
#[hook(addr = 0x804140, conv = cdecl, name = "_rwFileSystemOpen", expect = "A1 24 7B C9 00")]
pub fn rw_file_system_open() -> i32 {
    const FUNCS: [u32; 11] = [
        0x804200, 0x8232D8, 0x82318B, 0x823521, 0x823674, 0x823798, 0x8262B8, 0x8262A2, 0x82374F,
        0x823E86, 0x826261,
    ];
    for (i, f) in FUNCS.iter().enumerate() {
        wr::<u32>(rw_globals(), 0xC4 + 4 * i as isize, *f);
    }
    1
}

/// `_rwFileSystemClose()` (0x804240): empty (a lone `ret`, not hookable).
#[native(addr = 0x804240, conv = cdecl, name = "_rwFileSystemClose")]
pub fn rw_file_system_close() {}

// ------------------------------------------------------------------ small core modules -------

/// `_rwPipeAttachDummy()` (0x807C70): returns 1.
#[hook(addr = 0x807C70, conv = cdecl, name = "FUN_00807c70", expect = "B8 01 00 00 00")]
pub fn rw_pipe_attach_dummy() -> i32 {
    1
}

/// `cb_COLORMODULE_RwEngine_Open(instance, offset, size)` (0x80AA40): counts the instance (a
/// size-0 module).
#[hook(addr = 0x80AA40, conv = cdecl, name = "cb_COLORMODULE_RwEngine_Open", expect = "A1 28 A7 C9 00")]
pub fn color_module_open(instance: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let n = g::<i32>(COLOR_INSTANCES);
    n.write(n.read().wrapping_add(1));
    instance
}

/// `cb_COLORMODULE_RwEngine_Close(instance, offset, size)` (0x80AA50).
#[hook(addr = 0x80AA50, conv = cdecl, name = "cb_COLORMODULE_RwEngine_Close", expect = "A1 28 A7 C9 00")]
pub fn color_module_close(instance: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let n = g::<i32>(COLOR_INSTANCES);
    n.write(n.read().wrapping_sub(1));
    instance
}

/// `cb_PIPEMODULE_RwEngine_Open(instance, offset, size)` (0x807C40): stores the module offset;
/// `instance` if `_rxPipelineOpen()` succeeds, else NULL (`neg; sbb; and`).
#[hook(addr = 0x807C40, conv = cdecl, name = "cb_PIPEMODULE_RwEngine_Open", expect = "8B 44 24 08 A3 60 BC C9 00")]
pub fn pipe_module_open(instance: GPtr<u8>, offset: i32, _size: i32) -> GPtr<u8> {
    g::<i32>(PIPE_OFFSET).write(offset);
    if rx_pipeline_open() != 0 { instance } else { GPtr::NULL }
}

/// `cb_PIPEMODULE_RwEngine_Close(instance, offset, size)` (0x807C60): `_rxPipelineClose()`.
#[hook(addr = 0x807C60, conv = cdecl, name = "cb_PIPEMODULE_RwEngine_Close", expect = "E8 FB D2 FF FF")]
pub fn pipe_module_close(instance: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    rx_pipeline_close();
    instance
}

/// `cb_CHUNKGROUPMODULE_RwEngine_Open(instance, offset, size)` (0x80A780): the module holds a
/// freelist of 0x21-byte chunk groups (`[0x8E26D8]` per block, alignment 4, `[0x8E26DC]` blocks,
/// in-place space 0xC9A700, hint 0x40412); NULL if it cannot be created, else the instance is
/// counted and returned.
#[hook(addr = 0x80A780, conv = cdecl, name = "cb_CHUNKGROUPMODULE_RwEngine_Open", expect = "8B 0D DC 26 8E 00")]
pub fn chunk_group_module_open(instance: GPtr<u8>, offset: i32, _size: i32) -> GPtr<u8> {
    let blocks = g::<u32>(CHUNK_FL_BLOCKS).read();
    let per_block = g::<u32>(CHUNK_FL_PER_BLOCK).read();
    g::<i32>(CHUNK_OFFSET).write(offset);
    let fl = freelist_create_prealloc(0x21, per_block, 4, blocks, p8(CHUNK_FL_SPACE), 0x40412);
    wr::<u32>(rw_globals(), g::<i32>(CHUNK_OFFSET).read() as isize, fl.addr() as u32);
    if rd::<u32>(rw_globals(), g::<i32>(CHUNK_OFFSET).read() as isize) == 0 {
        return GPtr::NULL;
    }
    let n = g::<i32>(CHUNK_INSTANCES);
    n.write(n.read().wrapping_add(1));
    instance
}

/// `cb_CHUNKGROUPMODULE_RwEngine_Close(instance, offset, size)` (0x80A7E0): destroys the freelist
/// when there is one, one instance fewer.
#[hook(addr = 0x80A7E0, conv = cdecl, name = "cb_CHUNKGROUPMODULE_RwEngine_Close", expect = "A1 24 7B C9 00 8B 0D F8 A6 C9 00")]
pub fn chunk_group_module_close(instance: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let fl = rd::<u32>(rw_globals(), g::<i32>(CHUNK_OFFSET).read() as isize);
    if fl != 0 {
        freelist_destroy(p8(fl));
    }
    let n = g::<i32>(CHUNK_INSTANCES);
    n.write(n.read().wrapping_sub(1));
    instance
}

// ------------------------------------------------------------------ Im2D entry points --------

orig! {
    /// `G->dOpenDevice.fpIm2DRenderPrimitive(type, verts, n)` (+0x30)
    fn im2d_render_primitive_fn(ty: u32, verts: GPtr<u8>, n: i32) -> i32 = cdecl table 0xC97B24 0x30 => unported;
    /// `G->dOpenDevice.fpIm2DRenderIndexedPrimitive(type, verts, nv, indices, ni)` (+0x34)
    fn im2d_render_indexed_primitive_fn(ty: u32, verts: GPtr<u8>, nv: i32, idx: GPtr<u16>, ni: i32) -> i32 = cdecl table 0xC97B24 0x34 => unported;
}

/// `RwIm2DRenderPrimitive(type, verts, numVerts)` (0x734E90, catalogue `FUN_00734e90`): a tail jump
/// through the device's Im2D primitive function (`G + 0x30`). Written by STD-3, unverified.
#[hook(addr = 0x734E90, conv = cdecl, name = "FUN_00734e90", expect = "A1 24 7B C9 00")]
pub fn rw_im2d_render_primitive(ty: u32, verts: GPtr<u8>, n: i32) -> i32 {
    im2d_render_primitive_fn(ty, verts, n)
}

/// `RwIm2DRenderIndexedPrimitive(type, verts, numVerts, indices, numIndices)` (0x734EA0, catalogue
/// `thunk_FUN_00403927`): `nop; jmp` into the protection chunk 0x403927 that loads `G` (`[ror(
/// [0x1560778], [0x698EFA]) ]` = `[0xC97B24]`) and returns to a tail jump through `G + 0x34`. Not
/// hookable (the entry is a `nop` plus a relative `jmp`). Written by STD-3, unverified.
#[native(addr = 0x734EA0, conv = cdecl, name = "thunk_FUN_00403927")]
pub fn rw_im2d_render_indexed_primitive(ty: u32, verts: GPtr<u8>, nv: i32, idx: GPtr<u16>, ni: i32) -> i32 {
    im2d_render_indexed_primitive_fn(ty, verts, nv, idx, ni)
}

/// Every `#[hook]` of this module.
/// `RwOsGetFileInterface()` (0x804130): `&RwEngineInstance->fileFuncs` (`G + 0xC4`).
#[hook(addr = 0x804130, conv = cdecl, name = "RwOsGetFileInterface", expect = "A1 24 7B C9 00 05 C4 00 00 00",
       route = "L2", hazards = "none")]
pub fn rw_os_get_file_interface() -> GPtr<u8> {
    rw_globals().byte_add(0xC4)
}

pub const HOOKS: &[HookReg] = &[
    HOOK_RW_OS_GET_FILE_INTERFACE,
    HOOK_RW_IM2D_RENDER_PRIMITIVE,
    HOOK_DEVICE_SYSTEM_REQUEST,
    HOOK_GET_NUM_ENGINE_INSTANCES,
    HOOK_ENGINE_GET_NUM_SUB_SYSTEMS,
    HOOK_ENGINE_GET_SUB_SYSTEM_INFO,
    HOOK_ENGINE_GET_CURRENT_SUB_SYSTEM,
    HOOK_ENGINE_SET_SUB_SYSTEM,
    HOOK_ENGINE_GET_NUM_VIDEO_MODES,
    HOOK_ENGINE_SET_VIDEO_MODE,
    HOOK_ENGINE_STOP,
    HOOK_ENGINE_START,
    HOOK_ENGINE_CLOSE,
    HOOK_ENGINE_OPEN,
    HOOK_ENGINE_TERM,
    HOOK_ENGINE_INIT,
    HOOK_ERROR_MODULE_OPEN,
    HOOK_ERROR_MODULE_CLOSE,
    HOOK_RW_ERROR_SET,
    HOOK_RW_ERROR_GET,
    HOOK_RW_ERROR,
    HOOK_RW_STRING_OPEN,
    HOOK_RW_FILE_SYSTEM_OPEN,
    HOOK_RW_PIPE_ATTACH_DUMMY,
    HOOK_COLOR_MODULE_OPEN,
    HOOK_COLOR_MODULE_CLOSE,
    HOOK_PIPE_MODULE_OPEN,
    HOOK_PIPE_MODULE_CLOSE,
    HOOK_CHUNK_GROUP_MODULE_OPEN,
    HOOK_CHUNK_GROUP_MODULE_CLOSE,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] =
    &[NATIVE_RW_STRING_CLOSE, NATIVE_RW_FILE_SYSTEM_CLOSE, NATIVE_RW_IM2D_RENDER_INDEXED_PRIMITIVE, NATIVE_RW_DEVICE_REGISTER_PLUGIN];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
