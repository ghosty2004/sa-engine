//! LIGHTMODULE (RpWorld): `RpLight` and its world links (PORT-RW-1). Ported from the 1.0 US bytes;
//! `docs/re/specs/RW/RpLight.md` is the reading aid.
//!
//! `RpLight` (0x40 + plugins, registry 0x8D62F8, freelist `G + [0xC925D8]`): +0 RwObject (type 3,
//! subType = light type (>= 0x80 local), flags, privateFlags 1 = grey colour), +4 parent frame, +8
//! lFrame link, +0x10 sync, +0x14 radius, +0x18 RwRGBAReal colour, +0x28 minusCosAngle, +0x2C
//! WorldSectorsInLight list (ties at link - 0xC), +0x34 inWorld link, +0x3C u16 lightFrame.
//! `RpLightTie` (0x18, freelist `*(G + [0xC9259C] + 4)`): +0 lightInWorldSector link, +8 light, +0xC
//! WorldSectorInLight link, +0x14 sector. The light's world is at `light + [0xC925B0]`.
//!
//! `RpLightStreamRead` keeps the chunk length in its `stream` argument slot (not reproduced).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, llink, module, p8, rd, rw_error1, rw_error_set, rw_globals, rw_memory_alloc, rw_memory_free, wr, x, RwError};
use crate::stream::rw_free_list_create_and_preallocate;

/// LIGHTMODULE offset global.
pub const LIGHT_MODULE: usize = 0xC925D8;
/// LIGHTMODULE open count.
const LIGHT_INSTANCES: usize = 0xC925DC;
/// Static freelist struct.
const LIGHT_FL_STATIC: u32 = 0xC925B4;
/// Plugin registry (`sizeOfStruct` at +0).
pub const LIGHT_REGISTRY: u32 = 0x8D62F8;
/// Freelist entries per block (0x20) and blocks to preallocate (1).
const LIGHT_FL_PER_BLOCK: usize = 0x8D6310;
const LIGHT_FL_PREALLOC: usize = 0x8D6314;
/// WORLDOBJMODULE offset global (its engine data +4 = the light-tie freelist).
const WORLDOBJ_MODULE: usize = 0xC9259C;
/// Offset of the light's world pointer (WORLDOBJ light extension).
const LIGHT_WORLD_OFFSET: usize = 0xC925B0;
/// `_rpLightDefaultSyncCallBack`.
const DEFAULT_SYNC: u32 = 0x7521A0;
/// `_rpLightDestructor` (freelist walk callback of the module close).
const LIGHT_DESTRUCTOR: u32 = 0x752210;
/// 1.0f (0x858624).
const ONE: f32 = 1.0;

const E_RW_BADVERSION: u32 = 0x8000_0004;

orig! {
    /// `RwFrameUpdateObjects(frame)` (0x7F0910)
    fn rw_frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => crate::frame::update_objects;
    /// `_rwObjectHasFrameReleaseFrame(object)` (0x804F40)
    fn object_release_frame(o: GPtr<u8>) -> GPtr<u8> = cdecl 0x804F40 => crate::frame::object_has_frame_release_frame;
    /// `_rwPluginRegistryInitObject(reg, obj)` (0x8086E0)
    fn plugin_init_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x8086E0 => crate::plugin::registry_init_object;
    /// `_rwPluginRegistryDeInitObject(reg, obj)` (0x808740)
    fn plugin_deinit_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808740 => crate::plugin::registry_deinit_object;
    /// `_rwPluginRegistryAddPlugin(reg, size, id, ctor, dtor, copy)` (0x8084A0)
    fn plugin_add(reg: GPtr<u8>, size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x8084A0 => crate::plugin::registry_add_plugin;
    /// `_rwPluginRegistryReadDataChunks(reg, stream, obj)` (0x808980)
    fn plugin_read_data_chunks(reg: GPtr<u8>, s: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808980 => crate::plugin::registry_read_data_chunks;
    /// `RwFreeListForAllUsed(fl, cb, data)` (0x801E90)
    fn rw_free_list_for_all_used(fl: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x801E90 => unported;
    /// `RwFreeListDestroy(fl)` (0x801B80)
    fn rw_free_list_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => crate::memory::rw_freelist_destroy;
    /// `RwStreamFindChunk(stream, type, length, version)` (0x7ED2D0)
    fn rw_stream_find_chunk_call(s: GPtr<u8>, ty: u32, len: GPtr<u32>, ver: GPtr<u32>) -> u32 = cdecl 0x7ED2D0 => crate::stream::rw_stream_find_chunk;
    /// `RwStreamRead(stream, buf, len)` (0x7EC9D0)
    fn rw_stream_read_call(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => crate::stream::rw_stream_read;
    /// `_rwSqrt(x)` (0x7EDB30), result in ST0
    fn rw_sqrt(v: f32) -> f64 = cdecl 0x7EDB30 => crate::vector::rw_sqrt_f64;
    /// `RpLightCreate(type)` (0x752110)
    fn rp_light_create_call(ty: u32) -> GPtr<u8> = cdecl 0x752110 => rp_light_create;
}

fn light_free_list() -> GPtr<u8> {
    p8(rd::<u32>(module(LIGHT_MODULE), 0))
}

/// The grey flag test of `RpLightSetColor` / `RpLightStreamRead`: `r == g` and `r == b` (x87 compare,
/// NaN is not equal).
fn set_grey_flag(l: GPtr<u8>) {
    let r = rd::<f32>(l, 0x18);
    let grey = !x(r).fcom(x(rd::<f32>(l, 0x1C))).test_parity(0x44) && !x(r).fcom(x(rd::<f32>(l, 0x20))).test_parity(0x44);
    wr::<u8>(l, 3, grey as u8);
}

/// `RpWorldAddLight(world, light)` (0x751910): sets the light's world; directional lights (subType <
/// 0x80) go to `world+0x3C`, local ones update their frame first and go to `world+0x34`, at the head.
/// Returns world.
#[hook(addr = 0x751910, conv = cdecl, name = "RpWorldAddLight", expect = "A1 B0 25 C9 00")]
pub fn rp_world_add_light(world: GPtr<u8>, l: GPtr<u8>) -> GPtr<u8> {
    let ext = g::<u32>(LIGHT_WORLD_OFFSET).read();
    wr::<u32>(l, ext as i32 as isize, world.addr() as u32);
    let list = if rd::<u8>(l, 1) < 0x80 {
        world.byte_add(0x3C)
    } else {
        let f = rd::<u32>(l, 4);
        if f != 0 {
            rw_frame_update_objects(p8(f));
        }
        world.byte_add(0x34)
    };
    llink::add_head(list, l.byte_add(0x34));
    world
}

/// `RpWorldRemoveLight(world, light)` (0x751960): clears the light's world, frees every light tie
/// (both links unlinked), unlinks the light from its world list (links not cleared). Returns world.
#[hook(addr = 0x751960, conv = cdecl, name = "RpWorldRemoveLight", expect = "A1 B0 25 C9 00")]
pub fn rp_world_remove_light(world: GPtr<u8>, l: GPtr<u8>) -> GPtr<u8> {
    let ext = g::<u32>(LIGHT_WORLD_OFFSET).read();
    wr::<u32>(l, ext as i32 as isize, 0);
    let head = l.byte_add(0x2C);
    let mut link = llink::next(head);
    while link != head {
        let tie = link.byte_add(-0xC);
        let next = llink::next(link);
        llink::remove(link);
        llink::remove(tie);
        let fl = p8(rd::<u32>(module(WORLDOBJ_MODULE), 4));
        rw_memory_free(fl, tie);
        link = next;
    }
    llink::remove(l.byte_add(0x34));
    world
}

/// `RpLightSetRadius(light, radius)` (0x751A70): raw store, frame update when attached. Returns light.
#[hook(addr = 0x751A70, conv = cdecl, name = "RpLightSetRadius", expect = "8B 4C 24 08 56")]
pub fn rp_light_set_radius(l: GPtr<u8>, radius: u32) -> GPtr<u8> {
    let f = rd::<u32>(l, 4);
    wr::<u32>(l, 0x14, radius);
    if f != 0 {
        rw_frame_update_objects(p8(f));
    }
    l
}

/// `RpLightSetColor(light, color)` (0x751A90): copies the 4 floats, sets the grey flag. Returns light.
#[hook(addr = 0x751A90, conv = cdecl, name = "RpLightSetColor", expect = "8B 4C 24 04 8B 44 24 08")]
pub fn rp_light_set_color(l: GPtr<u8>, c: GPtr<u8>) -> GPtr<u8> {
    for k in 0..4 {
        wr::<u32>(l, 0x18 + k * 4, rd::<u32>(c, k * 4));
    }
    set_grey_flag(l);
    l
}

/// `RpLightRegisterPlugin(size, id, ctor, dtor, copy)` (0x751D60).
#[hook(addr = 0x751D60, conv = cdecl, name = "RpLightRegisterPlugin", expect = "8B 44 24 14 8B 4C 24 10")]
pub fn rp_light_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 {
    plugin_add(p8(LIGHT_REGISTRY), size, id, ctor, dtor, copy)
}

/// `RpLightStreamRead(stream)` (0x751F00): the struct chunk (version checked, error {2,
/// E_RW_BADVERSION}) read unclamped into a zeroed `{radius, r, g, b, minusCosAngle, u16 flags, u16
/// type}`; `RpLightCreate((u8)type)`, radius and colour copied (alpha stays 1), minusCosAngle raw (the
/// pre-0x30300 cone formula `-_rwSqrt(1 / (m*m + 1))` is dead after the version check but ported),
/// grey flag, object flags = (u8)flags, plugin data (failure -> NULL, light leaked).
/// Original bug kept: `len` > 0x18 overruns the struct (the original's return address; the port's own
/// stack).
#[hook(addr = 0x751F00, conv = cdecl, name = "RpLightStreamRead", expect = "83 EC 24 8D 44 24 00")]
pub fn rp_light_stream_read(s: GPtr<u8>) -> GPtr<u8> {
    let (mut ver, mut len) = (0u32, 0u32);
    let (vp, lp) = (GPtr::from_raw(&mut ver), GPtr::from_raw(&mut len));
    if rw_stream_find_chunk_call(s, 1, lp, vp) == 0 {
        return GPtr::NULL;
    }
    let v = vp.read();
    if !(0x34000..=0x36003).contains(&v) {
        let mut e = RwError { plugin_id: 2, error_code: 0 };
        e.error_code = rw_error1(E_RW_BADVERSION) as i32;
        rw_error_set(GPtr::from_raw(&mut e));
        return GPtr::NULL;
    }
    let mut st = [0u32; 6];
    let sp: GPtr<u8> = GPtr::from_raw(st.as_mut_ptr()).cast();
    if rw_stream_read_call(s, sp, lp.read()) != lp.read() {
        return GPtr::NULL;
    }
    let l = rp_light_create_call(rd::<u8>(sp, 0x16) as u32);
    if l.is_null() {
        return GPtr::NULL;
    }
    for k in 0..4 {
        wr::<u32>(l, 0x14 + k * 4, rd::<u32>(sp, k * 4));
    }
    if vp.read() < 0x30300 {
        let m = rd::<f32>(sp, 0x10);
        let t = f32_of(x(m).fmul_m(m).fadd_m(ONE).fdivr_m(ONE));
        wr::<f32>(l, 0x28, f32_of(X87::from_f64_exact(rw_sqrt(t)).fchs()));
    } else {
        wr::<u32>(l, 0x28, rd::<u32>(sp, 0x10));
    }
    set_grey_flag(l);
    wr::<u8>(l, 2, rd::<u8>(sp, 0x14));
    if plugin_read_data_chunks(p8(LIGHT_REGISTRY), s, l).is_null() {
        GPtr::NULL
    } else {
        l
    }
}

/// `RpLightDestroy(light)` (0x7520D0): plugin deinit, frame released, freed. Returns 1.
#[hook(addr = 0x7520D0, conv = cdecl, name = "RpLightDestroy", expect = "56 8B 74 24 08")]
pub fn rp_light_destroy(l: GPtr<u8>) -> u32 {
    plugin_deinit_object(p8(LIGHT_REGISTRY), l);
    object_release_frame(l);
    rw_memory_free(light_free_list(), l);
    1
}

/// `RpLightCreate(type)` (0x752110): from the light freelist (hint 0x30012; NULL -> NULL); type 3,
/// subType = type, white, no frame, default sync, radius / cone 0, grey, empty sector list, inWorld
/// {0, 0}, flags 3, lightFrame = `G->lightFrame - 1`; plugin init.
#[hook(addr = 0x752110, conv = cdecl, name = "RpLightCreate", expect = "A1 24 7B C9 00")]
pub fn rp_light_create(ty: u32) -> GPtr<u8> {
    let l = rw_memory_alloc(light_free_list(), 0x30012);
    if l.is_null() {
        return GPtr::NULL;
    }
    wr::<u8>(l, 0, 3);
    wr::<u8>(l, 1, ty as u8);
    for k in 0..4 {
        wr::<u32>(l, 0x18 + k * 4, 0x3F80_0000);
    }
    wr::<u8>(l, 2, 0);
    wr::<u32>(l, 4, 0);
    wr::<u32>(l, 0x10, DEFAULT_SYNC);
    wr::<u32>(l, 0x14, 0);
    wr::<u32>(l, 0x28, 0);
    wr::<u8>(l, 3, 1);
    llink::init(l.byte_add(0x2C));
    wr::<u32>(l, 0x38, 0);
    wr::<u32>(l, 0x34, 0);
    let frame = rd::<u16>(rw_globals(), 0xA).wrapping_sub(1);
    wr::<u8>(l, 2, 3);
    wr::<u16>(l, 0x3C, frame);
    plugin_init_object(p8(LIGHT_REGISTRY), l);
    l
}

/// `_rpLightDefaultSyncCallBack(light)` (0x7521A0): returns light.
#[hook(addr = 0x7521A0, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 44 24 04 C3")]
pub fn rp_light_default_sync(l: GPtr<u8>) -> GPtr<u8> {
    l
}

/// `cb_LIGHTMODULE_RwEngine_Close(instance, offset, size)` (0x7521B0): `_rpLightDestructor` over every
/// used light, freelist destroyed and cleared, count decremented. Returns instance.
#[hook(addr = 0x7521B0, conv = cdecl, name = "cb_LIGHTMODULE_RwEngine_Close", expect = "A1 D8 25 C9 00")]
pub fn light_module_close(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    rw_free_list_for_all_used(light_free_list(), LIGHT_DESTRUCTOR, 0);
    rw_free_list_destroy(light_free_list());
    wr::<u32>(module(LIGHT_MODULE), 0, 0);
    let c = g::<u32>(LIGHT_INSTANCES);
    c.write(c.read().wrapping_sub(1));
    instance
}

/// `_rpLightDestructor(light)` (0x752210): the body of `RpLightDestroy`; EAX is left as the freelist
/// free call returned it.
#[hook(addr = 0x752210, conv = cdecl, name = "_rpLightDestructor", expect = "56 8B 74 24 08")]
pub fn rp_light_destructor(l: GPtr<u8>) -> GPtr<u8> {
    plugin_deinit_object(p8(LIGHT_REGISTRY), l);
    object_release_frame(l);
    rw_memory_free(light_free_list(), l)
}

/// `cb_LIGHTMODULE_RwEngine_Open(instance, offset, size)` (0x752270): the light freelist (entry size from
/// the registry, hint 0x40012); NULL -> NULL, else counts the instance and returns it.
#[hook(addr = 0x752270, conv = cdecl, name = "cb_LIGHTMODULE_RwEngine_Open", expect = "8B 44 24 08 8B 0D 14 63 8D 00")]
pub fn light_module_open(instance: GPtr<u8>, offset: u32, _size: u32) -> GPtr<u8> {
    let prealloc = g::<u32>(LIGHT_FL_PREALLOC).read();
    let per_block = g::<u32>(LIGHT_FL_PER_BLOCK).read();
    g::<u32>(LIGHT_MODULE).write(offset);
    let size = g::<u32>(LIGHT_REGISTRY as usize).read();
    let fl = rw_free_list_create_and_preallocate(size, per_block, 4, prealloc, p8(LIGHT_FL_STATIC), 0x40012);
    wr::<u32>(module(LIGHT_MODULE), 0, fl.addr() as u32);
    if rd::<u32>(module(LIGHT_MODULE), 0) == 0 {
        return GPtr::NULL;
    }
    let c = g::<u32>(LIGHT_INSTANCES);
    c.write(c.read().wrapping_add(1));
    instance
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RP_WORLD_ADD_LIGHT,
    HOOK_RP_WORLD_REMOVE_LIGHT,
    HOOK_RP_LIGHT_SET_RADIUS,
    HOOK_RP_LIGHT_SET_COLOR,
    HOOK_RP_LIGHT_REGISTER_PLUGIN,
    HOOK_RP_LIGHT_STREAM_READ,
    HOOK_RP_LIGHT_DESTROY,
    HOOK_RP_LIGHT_CREATE,
    HOOK_RP_LIGHT_DEFAULT_SYNC,
    HOOK_LIGHT_MODULE_CLOSE,
    HOOK_RP_LIGHT_DESTRUCTOR,
    HOOK_LIGHT_MODULE_OPEN,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
