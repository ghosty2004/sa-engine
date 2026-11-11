//! PORT-RW-1 module `atomic`: `RpAtomic` (see docs/arch/rw-port.md). Ported from the 1.0 US bytes;
//! `docs/re/specs/RW/RpAtomic.md` as a reading aid.
//!
//! `RpAtomic` (0x70 base): +0 type, +1 subType, +2 flags, +3 privateFlags (bit 0 = world sphere dirty),
//! +4 frame, +8 lFrame link, +0x10 sync callback, +0x14 repEntry, +0x18 geometry, +0x1C boundingSphere,
//! +0x2C worldBoundingSphere, +0x3C clump, +0x40 inClumpLink, +0x48 renderCallBack, +0x4C interpolator
//! (flags +0x4C, start +0x50 i16, end +0x52 i16, time +0x54, recipTime +0x58, position +0x5C),
//! +0x60 renderFrame, +0x64 llWorldSectorsInAtomic, +0x6C pipeline. RpGeometry: +0x18 numMorphTargets,
//! +0x5C morph targets (0x1C each, bounding sphere at +4, radius at +0x10).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::clump::CLUMP_MODULE_OFFSET;
use crate::common::{f32_of, g, module, p8, rd, rw_globals, rw_memory_alloc, rw_memory_free, wr, x};

/// WORLDOBJMODULE: offset of the atomic extension (+0 = the `RpWorld*`).
const WORLD_ATOMIC_EXT_OFFSET: usize = 0xC925A8;
/// PIPEMODULE offset global (+0x3C default atomic pipeline, +0x54 generic atomic pipeline).
const PIPE_MODULE: usize = 0xC9BC60;
/// `_rpAtomicDefaultFrameSyncCallBack` / `AtomicDefaultRenderCallBack`.
const DEFAULT_SYNC_CB: u32 = 0x749D20;
const DEFAULT_RENDER_CB: u32 = 0x7491C0;

sa_core::globals! {
    list ATOMIC_GLOBALS;
    /// Atomic `RwPluginRegistry`
    pub static ATOMIC_REGISTRY @ 0x8D624C: [u32; 6] = [0x70, 0x70, 0, 0, 0, 0];
    /// Rights-to-render plugin id (read by `cb_RIGHTTORENDER_RpAtomic_StreamRead`)
    pub static RIGHTS_PLUGIN_ID @ 0xC924A8: u32 = 0;
    /// Rights-to-render extra data
    pub static RIGHTS_EXTRA_DATA @ 0xC92480: u32 = 0;
}

orig! {
    /// `RwStreamReadInt32(stream, dst, numBytes)` (0x7ED540)
    pub fn stream_read_int32(s: GPtr<u8>, dst: GPtr<u32>, n: u32) -> GPtr<u8> = cdecl 0x7ED540 => unported;
    /// `RxPipelineExecute(pipeline, data, heapReset)` (0x805710)
    pub fn rx_pipeline_execute(p: GPtr<u8>, data: GPtr<u8>, reset: i32) -> GPtr<u8> = cdecl 0x805710 => unported;
    /// `_rpGeometryAddRef(geometry)` (0x74CCB0)
    pub fn geometry_add_ref(g: GPtr<u8>) -> GPtr<u8> = cdecl 0x74CCB0 => unported;
    /// `RpGeometryDestroy(geometry)` (0x74CCC0)
    pub fn geometry_destroy(g: GPtr<u8>) -> i32 = cdecl 0x74CCC0 => unported;
    /// `RwResourcesFreeResEntry(entry)` (0x807DE0)
    pub fn resources_free_res_entry(e: GPtr<u8>) -> i32 = cdecl 0x807DE0 => unported;
    /// `RwV3dTransformPoint(out, in, matrix)` (0x7EDD60)
    pub fn v3d_transform_point(out: GPtr<u8>, inp: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD60 => unported;
    /// `_rwSqrt(x)` (0x7EDB30; the f32 argument passed as its dword, result in ST0)
    pub fn rw_sqrt(v: u32) -> f64 = cdecl 0x7EDB30 => unported;

    /// `RwFrameDirty` (0x7F0340)
    pub fn frame_dirty(f: GPtr<u8>) -> u32 = cdecl 0x7F0340 => crate::frame::dirty;
    /// `RwFrameGetLTM` (0x7F0990)
    pub fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => crate::frame::get_ltm;
    /// `RwFrameUpdateObjects` (0x7F0910)
    pub fn frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => crate::frame::update_objects;
    /// `_rwObjectHasFrameSetFrame` (0x804EF0)
    pub fn object_set_frame(obj: GPtr<u8>, f: GPtr<u8>) = cdecl 0x804EF0 => crate::frame::object_has_frame_set_frame;
    /// `_rwObjectHasFrameReleaseFrame` (0x804F40)
    pub fn object_release_frame(obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x804F40 => crate::frame::object_has_frame_release_frame;
    /// `_rwPluginRegistryInitObject` (0x8086E0)
    pub fn registry_init_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x8086E0 => crate::plugin::registry_init_object;
    /// `_rwPluginRegistryDeInitObject` (0x808740)
    pub fn registry_deinit_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808740 => crate::plugin::registry_deinit_object;
    /// `_rwPluginRegistryCopyObject` (0x808770)
    pub fn registry_copy_object(reg: GPtr<u8>, dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl 0x808770 => crate::plugin::registry_copy_object;
    /// `_rwPluginRegistryAddPlugin` (0x8084A0)
    pub fn registry_add_plugin(reg: GPtr<u8>, size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x8084A0 => crate::plugin::registry_add_plugin;
    /// `_rwPluginRegistryAddPluginStream` (0x8088E0)
    pub fn registry_add_plugin_stream(reg: GPtr<u8>, id: u32, read: u32, write: u32, get_size: u32) -> i32 = cdecl 0x8088E0 => crate::plugin::registry_add_plugin_stream;
    /// `_rwPluginRegistryAddPlgnStrmlwysCB` (0x808920)
    pub fn registry_add_always_cb(reg: GPtr<u8>, id: u32, cb: u32) -> i32 = cdecl 0x808920 => crate::plugin::registry_add_plugin_stream_always_cb;
    /// `_rwPluginRegistryAddPlgnStrmRightsCB` (0x808950)
    pub fn registry_add_rights_cb(reg: GPtr<u8>, id: u32, cb: u32) -> i32 = cdecl 0x808950 => crate::plugin::registry_add_plugin_stream_rights_cb;
    /// `_rwPluginRegistryGetPluginOffset` (0x808470)
    pub fn registry_get_plugin_offset(reg: GPtr<u8>, id: u32) -> i32 = cdecl 0x808470 => crate::plugin::registry_get_plugin_offset;

    /// `_rpAtomicResyncInterpolatedSphere` (0x7491F0)
    pub fn resync_interpolated_sphere_call(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x7491F0 => resync_interpolated_sphere;
    /// `RpAtomicCreate` (0x749C50)
    pub fn create_call() -> GPtr<u8> = cdecl 0x749C50 => create;
    /// `RpAtomicGetWorld` (0x751050)
    pub fn get_world_call(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x751050 => get_world;
}

fn a32(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// The atomic registry (`0x8D624C`).
pub fn atomic_registry() -> GPtr<u8> {
    ATOMIC_REGISTRY.ptr().cast()
}

/// The atomic freelist: `[G + [0xC924AC]]`.
pub fn atomic_freelist() -> GPtr<u8> {
    p8(rd::<u32>(rw_globals(), CLUMP_MODULE_OFFSET.get() as i32 as isize))
}

fn copy4(dst: GPtr<u8>, src: GPtr<u8>) {
    for i in 0..4 {
        wr::<u32>(dst, i * 4, rd::<u32>(src, i * 4));
    }
}

/// Morph target `i` of `geom` (`[geom + 0x5C] + i * 0x1C`, 32-bit wrapping).
fn morph_target(geom: GPtr<u8>, i: i32) -> GPtr<u8> {
    p8(rd::<u32>(geom, 0x5C).wrapping_add((i as u32).wrapping_mul(0x1C)))
}

/// `cb_RIGHTTORENDER_RpAtomic_StreamRead` (0x749120): the rights plugin id, and the extra data when the
/// chunk is exactly 8 bytes. Returns stream or NULL.
#[hook(addr = 0x749120, conv = cdecl, name = "cb_RIGHTTORENDER_RpAtomic_StreamRead", expect = "56 8B 74 24 08")]
pub fn rights_stream_read(s: GPtr<u8>, len: i32, _obj: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let r = stream_read_int32(s, RIGHTS_PLUGIN_ID.ptr(), 4);
    if r.is_null() {
        return r;
    }
    if len == 8 {
        let r = stream_read_int32(s, RIGHTS_EXTRA_DATA.ptr(), 4);
        if r.is_null() {
            return r;
        }
    }
    s
}

/// `AtomicDefaultRenderCallBack` (0x7491C0): runs the atomic's pipeline (or the default one).
#[hook(addr = 0x7491C0, conv = cdecl, name = "AtomicDefaultRenderCallBack", expect = "56 8B 74 24 08")]
pub fn default_render_callback(a: GPtr<u8>) -> GPtr<u8> {
    let mut pipe = p8(rd::<u32>(a, 0x6C));
    if pipe.is_null() {
        pipe = p8(rd::<u32>(module(PIPE_MODULE), 0x3C));
    }
    if rx_pipeline_execute(pipe, a, 1).is_null() {
        GPtr::NULL
    } else {
        a
    }
}

/// `_rpAtomicResyncInterpolatedSphere` (0x7491F0): the bounding sphere from the interpolated morph
/// targets (x87 order of the original: the x centre adds the register value, y/z their stored f32), else
/// a copy of morph target `start` (or 0 when an index is out of range; negative indices pass the signed
/// test, as in the original). Returns EAX = atomic.
#[hook(addr = 0x7491F0, conv = cdecl, name = "_rpAtomicResyncInterpolatedSphere", expect = "51 8B 44 24 08")]
pub fn resync_interpolated_sphere(a: GPtr<u8>) -> GPtr<u8> {
    let geom = p8(rd::<u32>(a, 0x18));
    if geom.is_null() {
        return a;
    }
    let end = rd::<i16>(a, 0x52);
    let start = rd::<i16>(a, 0x50);
    let mut done = false;
    if start != end {
        let n = rd::<i32>(geom, 0x18);
        let (s, e) = (start as i32, end as i32);
        if s < n && e < n {
            let sp = morph_target(geom, s);
            let ep = morph_target(geom, e);
            let f = x(rd::<f32>(a, 0x5C)).fmul_m(rd::<f32>(a, 0x58));
            let r = x(rd::<f32>(ep, 0x10))
                .fsub_m(rd::<f32>(sp, 0x10))
                .fmul(f)
                .fadd_m(rd::<f32>(sp, 0x10));
            wr::<f32>(a, 0x28, f32_of(r));
            for k in 0..3 {
                let d = x(rd::<f32>(ep, 4 + k * 4)).fsub_m(rd::<f32>(sp, 4 + k * 4));
                wr::<f32>(a, 0x1C + k * 4, f32_of(d));
            }
            let tx = x(rd::<f32>(a, 0x1C)).fmul(f);
            wr::<f32>(a, 0x1C, f32_of(tx));
            let ty = f32_of(x(rd::<f32>(a, 0x20)).fmul(f));
            wr::<f32>(a, 0x20, ty);
            let tz = f32_of(x(rd::<f32>(a, 0x24)).fmul(f));
            wr::<f32>(a, 0x24, tz);
            wr::<f32>(a, 0x1C, f32_of(x(rd::<f32>(sp, 4)).fadd(tx)));
            wr::<f32>(a, 0x20, f32_of(x(rd::<f32>(sp, 8)).fadd_m(ty)));
            wr::<f32>(a, 0x24, f32_of(x(rd::<f32>(sp, 0xC)).fadd_m(tz)));
            done = true;
        }
    }
    if !done {
        let n = rd::<i32>(geom, 0x18);
        let (s, e) = (start as i32, end as i32);
        let src = if s < n && e < n {
            morph_target(geom, s)
        } else {
            morph_target(geom, 0)
        };
        copy4(a.byte_add(0x1C), src.byte_add(4));
    }
    wr::<u32>(a, 0x4C, rd::<u32>(a, 0x4C) & 0xFFFF_FFFD);
    wr::<u8>(a, 3, rd::<u8>(a, 3) | 1);
    a
}

/// `(x*x + y*y) + z*z` of the row at `p` (loads y, x, z as the original does).
fn row_len2(p: GPtr<u8>) -> X87 {
    let y = x(rd::<f32>(p, 4));
    let xx = x(rd::<f32>(p, 0));
    let z = x(rd::<f32>(p, 8));
    xx.fmul(xx).fadd(y.fmul(y)).fadd(z.fmul(z))
}

/// `RpAtomicGetWorldBoundingSphere` (0x749330): transforms the centre by the LTM; the radius is copied for
/// an orthonormal LTM, else scaled by `_rwSqrt` of the largest squared row length (right row kept in an
/// x87 register, up/at rounded to f32). Returns `&atomic->worldBoundingSphere`.
#[hook(addr = 0x749330, conv = cdecl, name = "RpAtomicGetWorldBoundingSphere", expect = "51 56 57 8B 7C 24 10")]
pub fn get_world_bounding_sphere(a: GPtr<u8>) -> GPtr<u8> {
    let frame = p8(rd::<u32>(a, 4));
    if rd::<u8>(a, 0x4C) & 2 != 0 {
        resync_interpolated_sphere_call(a);
    }
    let ws = a.byte_add(0x2C);
    if frame_dirty(frame) == 0 && rd::<u8>(a, 3) & 1 == 0 {
        return ws;
    }
    let ltm = frame_get_ltm(frame);
    v3d_transform_point(ws, a.byte_add(0x1C), ltm);
    if rd::<u32>(ltm, 0xC) & 3 == 3 {
        wr::<u32>(a, 0x38, rd::<u32>(a, 0x28));
        wr::<u8>(a, 3, rd::<u8>(a, 3) & 0xFE);
        return ws;
    }
    let r2 = row_len2(ltm);
    let u2 = f32_of(row_len2(ltm.byte_add(0x10)));
    let a2 = f32_of(row_len2(ltm.byte_add(0x20)));
    let m = if x(u2).fcom(x(a2)).c0() { x(a2) } else { x(u2) };
    let v: u32 = if !r2.fcom(m).c0() {
        f32_of(r2).to_bits()
    } else if x(u2).fcom(x(a2)).c0() {
        a2.to_bits()
    } else {
        u2.to_bits()
    };
    let r = X87::from_f64_exact(rw_sqrt(v)).fmul_m(rd::<f32>(a, 0x28));
    wr::<u8>(a, 3, rd::<u8>(a, 3) & 0xFE);
    wr::<f32>(a, 0x38, f32_of(r));
    ws
}

/// `RpAtomicCreate` (0x749C50): no RwError on allocation failure; renderFrame (+0x60) left as is.
#[hook(addr = 0x749C50, conv = cdecl, name = "RpAtomicCreate", expect = "A1 24 7B C9 00")]
pub fn create() -> GPtr<u8> {
    let a = rw_memory_alloc(atomic_freelist(), 0x30014);
    if a.is_null() {
        return a;
    }
    wr::<u8>(a, 0, 1);
    wr::<u8>(a, 1, 0);
    wr::<u32>(a, 4, 0);
    wr::<u32>(a, 0x10, DEFAULT_SYNC_CB);
    wr::<u32>(a, 0x14, 0);
    wr::<u8>(a, 2, 5);
    wr::<u8>(a, 3, 1);
    object_set_frame(a, GPtr::NULL);
    let pf = rd::<u8>(a, 3) | 1;
    wr::<u32>(a, 0x18, 0);
    wr::<u8>(a, 3, pf);
    for off in [0x28, 0x1C, 0x20, 0x24, 0x38, 0x2C, 0x30, 0x34] {
        wr::<u32>(a, off, 0);
    }
    wr::<u32>(a, 0x48, DEFAULT_RENDER_CB);
    wr::<u32>(a, 0x54, 0x3F80_0000);
    wr::<u32>(a, 0x58, 0x3F80_0000);
    wr::<u16>(a, 0x50, 0);
    wr::<u16>(a, 0x52, 0);
    wr::<u32>(a, 0x5C, 0);
    wr::<u32>(a, 0x4C, 3);
    wr::<u32>(a, 0x44, 0);
    wr::<u32>(a, 0x40, 0);
    wr::<u32>(a, 0x3C, 0);
    wr::<u32>(a, 0x6C, 0);
    let list = a.byte_add(0x64);
    wr::<u32>(list, 0, a32(list));
    wr::<u32>(a, 0x68, a32(list));
    registry_init_object(atomic_registry(), a);
    a
}

/// `_rpAtomicDefaultFrameSyncCallBack` (0x749D20, uncatalogued name): resync the interpolated sphere when
/// dirty, mark the world sphere dirty; returns atomic.
#[hook(addr = 0x749D20, conv = cdecl, name = "FUN_00749d20", expect = "56 8B 74 24 08")]
pub fn default_frame_sync_callback(a: GPtr<u8>) -> GPtr<u8> {
    if rd::<u8>(a, 0x4C) & 2 != 0 {
        resync_interpolated_sphere_call(a);
    }
    wr::<u8>(a, 3, rd::<u8>(a, 3) | 1);
    a
}

/// The geometry switch shared by `RpAtomicSetGeometry` and its inlined copies (without the flags test):
/// AddRef new, destroy old, store, copy morph target 0's sphere.
fn swap_geometry(a: GPtr<u8>, geom: GPtr<u8>) {
    if !geom.is_null() {
        geometry_add_ref(geom);
    }
    let old = p8(rd::<u32>(a, 0x18));
    if !old.is_null() {
        geometry_destroy(old);
    }
    wr::<u32>(a, 0x18, a32(geom));
}

fn copy_geometry_sphere_and_update(a: GPtr<u8>, geom: GPtr<u8>) {
    if !geom.is_null() {
        copy4(a.byte_add(0x1C), p8(rd::<u32>(geom, 0x5C)).byte_add(4));
    }
    let frame = p8(rd::<u32>(a, 4));
    if !frame.is_null() && !get_world_call(a).is_null() {
        frame_update_objects(frame);
    }
}

/// Inline `RpAtomicSetGeometry(a, geom, 0)` as RpAtomicClone / the clump readers expand it.
pub fn set_geometry_inline(a: GPtr<u8>, geom: GPtr<u8>) {
    if geom == p8(rd::<u32>(a, 0x18)) {
        return;
    }
    swap_geometry(a, geom);
    copy_geometry_sphere_and_update(a, geom);
}

/// `RpAtomicSetGeometry` (0x749D40): no work for the same geometry; AddRef before the old one is
/// destroyed; the frame update only when the atomic is in a world.
#[hook(addr = 0x749D40, conv = cdecl, name = "RpAtomicSetGeometry", expect = "56 8B 74 24 08")]
pub fn set_geometry(a: GPtr<u8>, geom: GPtr<u8>, flags: u32) -> GPtr<u8> {
    if geom == p8(rd::<u32>(a, 0x18)) {
        return a;
    }
    swap_geometry(a, geom);
    if flags & 1 != 0 {
        return a;
    }
    copy_geometry_sphere_and_update(a, geom);
    a
}

/// The body of `RpAtomicDestroy` (also inlined by `RpClumpDestroy`).
pub fn destroy_body(a: GPtr<u8>) {
    registry_deinit_object(atomic_registry(), a);
    let rep = p8(rd::<u32>(a, 0x14));
    if !rep.is_null() {
        resources_free_res_entry(rep);
    }
    let geom = p8(rd::<u32>(a, 0x18));
    if !geom.is_null() {
        geometry_destroy(geom);
        let frame = p8(rd::<u32>(a, 4));
        wr::<u32>(a, 0x18, 0);
        if !frame.is_null() && !get_world_call(a).is_null() {
            frame_update_objects(frame);
        }
    }
    object_release_frame(a);
    rw_memory_free(atomic_freelist(), a);
}

/// `RpAtomicDestroy` (0x749DC0): plugin destructors first; not removed from its clump. Returns 1.
#[hook(addr = 0x749DC0, conv = cdecl, name = "RpAtomicDestroy", expect = "56 8B 74 24 08")]
pub fn destroy(a: GPtr<u8>) -> u32 {
    destroy_body(a);
    1
}

/// `RpAtomicClone` (0x749E60): header bytes, inline SetGeometry, then the source's bounding sphere,
/// render callback, pipeline and interpolator; plugin copy. Not copied: world sphere, clump, repEntry.
#[hook(addr = 0x749E60, conv = cdecl, name = "RpAtomicClone", expect = "53 E8 EA FD FF FF")]
pub fn clone(src: GPtr<u8>) -> GPtr<u8> {
    let c = create_call();
    if c.is_null() {
        return c;
    }
    for i in 0..4 {
        wr::<u8>(c, i, rd::<u8>(src, i));
    }
    wr::<u32>(c, 4, 0);
    wr::<u8>(c, 2, rd::<u8>(src, 2));
    object_set_frame(c, GPtr::NULL);
    wr::<u8>(c, 3, rd::<u8>(c, 3) | 1);
    set_geometry_inline(c, p8(rd::<u32>(src, 0x18)));
    copy4(c.byte_add(0x1C), src.byte_add(0x1C));
    wr::<u32>(c, 0x48, rd::<u32>(src, 0x48));
    wr::<u32>(c, 0x6C, rd::<u32>(src, 0x6C));
    for i in 0..5 {
        wr::<u32>(c, 0x4C + i * 4, rd::<u32>(src, 0x4C + i * 4));
    }
    registry_copy_object(atomic_registry(), c, src);
    c
}

/// `RpAtomicRegisterPlugin` (0x74BDA0).
#[hook(addr = 0x74BDA0, conv = cdecl, name = "RpAtomicRegisterPlugin", expect = "8B 44 24 14 8B 4C 24 10")]
pub fn register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 {
    registry_add_plugin(atomic_registry(), size, id, ctor, dtor, copy)
}

/// `RpAtomicRegisterPluginStream` (0x74BE00).
#[hook(addr = 0x74BE00, conv = cdecl, name = "RpAtomicRegisterPluginStream", expect = "8B 44 24 10 8B 4C 24 0C")]
pub fn register_plugin_stream(id: u32, read: u32, write: u32, get_size: u32) -> i32 {
    registry_add_plugin_stream(atomic_registry(), id, read, write, get_size)
}

/// `RpAtomicSetStreamAlwaysCallBack` (0x74BE30).
#[hook(addr = 0x74BE30, conv = cdecl, name = "RpAtomicSetStreamAlwaysCallBack", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn set_stream_always_callback(id: u32, cb: u32) -> i32 {
    registry_add_always_cb(atomic_registry(), id, cb)
}

/// `RpAtomicSetStreamRightsCallBack` (0x74BE50).
#[hook(addr = 0x74BE50, conv = cdecl, name = "RpAtomicSetStreamRightsCallBack", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn set_stream_rights_callback(id: u32, cb: u32) -> i32 {
    registry_add_rights_cb(atomic_registry(), id, cb)
}

/// `RpAtomicGetPluginOffset` (0x74BEC0).
#[hook(addr = 0x74BEC0, conv = cdecl, name = "RpAtomicGetPluginOffset", expect = "8B 44 24 04 50")]
pub fn get_plugin_offset(id: u32) -> i32 {
    registry_get_plugin_offset(atomic_registry(), id)
}

/// `RpAtomicSetFrame` (0x74BF20): attach, mark the world sphere dirty; returns atomic.
#[hook(addr = 0x74BF20, conv = cdecl, name = "RpAtomicSetFrame", expect = "8B 44 24 08 56")]
pub fn set_frame(a: GPtr<u8>, frame: GPtr<u8>) -> GPtr<u8> {
    object_set_frame(a, frame);
    wr::<u8>(a, 3, rd::<u8>(a, 3) | 1);
    a
}

/// `RpAtomicGetWorld` (0x751050): `*(atomic + [0xC925A8])`.
#[hook(addr = 0x751050, conv = cdecl, name = "RpAtomicGetWorld", expect = "8B 44 24 04 8B 0D A8 25 C9 00")]
pub fn get_world(a: GPtr<u8>) -> GPtr<u8> {
    let off = g::<u32>(WORLD_ATOMIC_EXT_OFFSET).read();
    p8(rd::<u32>(a, off as i32 as isize))
}

/// `RpAtomicSetDefaultPipeline` (0x75E2A0): NULL restores the generic pipeline (+0x54). Returns the
/// pipeline now installed.
#[hook(addr = 0x75E2A0, conv = cdecl, name = "RpAtomicSetDefaultPipeline", expect = "8B 44 24 04 85 C0")]
pub fn set_default_pipeline(p: GPtr<u8>) -> GPtr<u8> {
    if p.is_null() {
        let m = module(PIPE_MODULE);
        let generic = rd::<u32>(m, 0x54);
        wr::<u32>(m, 0x3C, generic);
        return p8(generic);
    }
    wr::<u32>(module(PIPE_MODULE), 0x3C, a32(p));
    p
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RIGHTS_STREAM_READ,
    HOOK_DEFAULT_RENDER_CALLBACK,
    HOOK_RESYNC_INTERPOLATED_SPHERE,
    HOOK_GET_WORLD_BOUNDING_SPHERE,
    HOOK_CREATE,
    HOOK_DEFAULT_FRAME_SYNC_CALLBACK,
    HOOK_SET_GEOMETRY,
    HOOK_DESTROY,
    HOOK_CLONE,
    HOOK_REGISTER_PLUGIN,
    HOOK_REGISTER_PLUGIN_STREAM,
    HOOK_SET_STREAM_ALWAYS_CALLBACK,
    HOOK_SET_STREAM_RIGHTS_CALLBACK,
    HOOK_GET_PLUGIN_OFFSET,
    HOOK_SET_FRAME,
    HOOK_GET_WORLD,
    HOOK_SET_DEFAULT_PIPELINE,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = ATOMIC_GLOBALS;
