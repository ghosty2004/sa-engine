//! **Temporary** native stand-ins for RW D3D9 driver functions (PORT-RW-4's lane) on the native
//! boot path, registered at their original addresses only where no port exists (the registry
//! takes `#[hook]`/`#[native]` ports first, so a merged PORT-RW-4 port replaces its stand-in).
//!
//! These are not ports: each implements what the boot path observes as a transcription of the
//! 1.0 US bytes (no-data cases of the skin / MatFX object callbacks, the anisotropy plugin).
//! They are listed in docs/arch/platform.md ("RW driver
//! stand-ins") and are deleted once ports replace them. The D3D9 driver's system function, raster
//! create / destroy, camera begin update / clear and raster show stand-ins are gone (STD-4: ported
//! in sa-rwcore `d3d9sys.rs`, `d3d9rast.rs`, `d3d9cam.rs`; the device lives in [`crate::d3d9`]).

use sa_core::mem::GPtr;

fn g<T: sa_core::mem::GameData>(a: u32) -> GPtr<T> {
    GPtr::from_addr(a as usize)
}
fn rd(a: u32) -> u32 {
    g::<u32>(a).read()
}
fn wr(a: u32, v: u32) {
    g::<u32>(a).write(v)
}

sa_core::orig! {
    /// `_rwPluginRegistryAddPlugin(reg, size, id, ctor, dtor, copy)` (0x8084A0)
    fn registry_add_plugin(reg: GPtr<u8>, size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x8084A0 => unported;
    /// `RwRasterRegisterPlugin(size, id, ctor, dtor, copy)` (0x7FB0B0)
    fn raster_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x7FB0B0 => unported;
    /// `G->memoryFuncs.rwmalloc` (+0x134)
    fn rw_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl table 0xC97B24 0x134 => unported;
    /// `G->memoryFuncs.rwrealloc` (+0x13C)
    fn rw_realloc(p: GPtr<u8>, size: u32, hint: u32) -> GPtr<u8> = cdecl table 0xC97B24 0x13C => unported;
    /// `G->stringFuncs.rwstrcpy` (+0xF8)
    fn rw_strcpy(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl table 0xC97B24 0xF8 => unported;
}

// ------------------------------------------------------------------ registration -------------






// ------------------------------------------------------------------ rasters ------------------

sa_core::orig! {
    /// `G->memoryAlloc(freelist, hint)` (+0x144)
    fn memory_alloc(fl: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl table 0xC97B24 0x144 => unported;
    /// `G->memoryFree(freelist, p)` (+0x148)
    fn memory_free(fl: GPtr<u8>, p: GPtr<u8>) -> GPtr<u8> = cdecl table 0xC97B24 0x148 => unported;
    /// `RwFreeListCreateAndPreallocateSpace(...)` (0x801B70)
    fn freelist_create(size: u32, per_block: u32, align: u32, n: u32, space: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl 0x801B70 => unported;
    /// `RwFreeListDestroy(fl)` (0x801B80)
    fn freelist_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => unported;
    /// `_rwPluginRegistryInitObject(reg, obj)` (0x8086E0)
    fn registry_init_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x8086E0 => unported;
    /// `_rwPluginRegistryDeInitObject(reg, obj)` (0x808740)
    fn registry_deinit_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808740 => unported;
    /// `G->stdFunc[i](pOut, pInOut, nI)` (standard functions, `G + 0x48 + 4i`)
    fn std_func(f: u32, out: GPtr<u8>, inout: GPtr<u8>, n: i32) -> i32 = cdecl dyn => unported;
    /// `RwStreamRead(stream, buf, size)` (0x7EC9D0)
    fn stream_read(s: GPtr<u8>, buf: GPtr<u8>, size: u32) -> u32 = cdecl 0x7EC9D0 => unported;
    /// `RwStreamFindChunk(stream, type, &len, &ver)` (0x7ED2D0)
    fn stream_find_chunk(s: GPtr<u8>, ty: u32, len: GPtr<u32>, ver: GPtr<u32>) -> i32 = cdecl 0x7ED2D0 => unported;
    /// `RwRasterCreate(w, h, depth, flags)` (0x7FB230)
    fn raster_create(w: i32, h: i32, depth: i32, flags: i32) -> GPtr<u8> = cdecl 0x7FB230 => unported;
    /// `RwRasterDestroy(raster)` (0x7FB020)
    fn raster_destroy(r: GPtr<u8>) -> i32 = cdecl 0x7FB020 => unported;
    /// `RwTextureCreate(raster)` (0x7F37C0)
    fn texture_create(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F37C0 => unported;
    /// `RwTextureSetName(tex, name)` (0x7F38A0)
    fn texture_set_name(t: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F38A0 => unported;
    /// `RwTextureSetMaskName(tex, name)` (0x7F3910)
    fn texture_set_mask_name(t: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3910 => unported;
}

// ------------------------------------------------------------------ render states, Im2D ------






// ------------------------------------------------------------------ world pipelines ----------

sa_core::orig! {
    /// `RxPipelineCreate()` (0x8057B0)
    fn rx_pipeline_create() -> GPtr<u8> = cdecl 0x8057B0 => unported;
    /// `_rxPipelineDestroy(pipeline)` (0x805820)
    fn rx_pipeline_destroy(p: GPtr<u8>) = cdecl 0x805820 => unported;
    /// `RpWorldSetDefaultSectorPipeline(pipeline)` (0x75E270)
    fn world_set_default_sector_pipeline(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x75E270 => unported;
    /// `RpAtomicSetDefaultPipeline(pipeline)` (0x75E2A0)
    fn atomic_set_default_pipeline(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x75E2A0 => unported;
}

// ------------------------------------------------------------------ RpSkin, RpMatFX, RpAnisot -

sa_core::orig! {
    fn engine_register_plugin(size: i32, id: u32, open: u32, close: u32) -> i32 = cdecl 0x7F2BB0 => unported;
    fn engine_get_plugin_offset(id: u32) -> i32 = cdecl 0x7F2BE0 => unported;
    fn atomic_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x74BDA0 => unported;
    fn atomic_register_plugin_stream(id: u32, read: u32, write: u32, size: u32) -> i32 = cdecl 0x74BE00 => unported;
    fn atomic_set_stream_always_cb(id: u32, cb: u32) -> i32 = cdecl 0x74BE30 => unported;
    fn atomic_set_stream_rights_cb(id: u32, cb: u32) -> i32 = cdecl 0x74BE50 => unported;
    fn geometry_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x74CD70 => unported;
    fn geometry_register_plugin_stream(id: u32, read: u32, write: u32, size: u32) -> i32 = cdecl 0x74CDA0 => unported;
    fn material_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x74DBF0 => unported;
    fn material_register_plugin_stream(id: u32, read: u32, write: u32, size: u32) -> i32 = cdecl 0x74DC20 => unported;
    fn sector_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x761C90 => unported;
    fn sector_register_plugin_stream(id: u32, read: u32, write: u32, size: u32) -> i32 = cdecl 0x761CC0 => unported;
    fn texture_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x7F3BB0 => unported;
    fn texture_register_plugin_stream(id: u32, read: u32, write: u32, size: u32) -> i32 = cdecl 0x804550 => unported;
}

/// Skin geometry/atomic extension constructors and copies (0x7C69F0, 0x7C6A80, 0x7C6AA0,
/// 0x7C6AC0, 0x7C6A70), transcribed; the geometry destructor 0x7C6A10 only handles the
/// no-skin case.
fn skin_geometry_ctor(obj: u32, _o: u32, _s: u32) -> u32 {
    wr(rd(0xC978A8).wrapping_add(obj), 0);
    obj
}
fn skin_geometry_dtor(obj: u32, _o: u32, _s: u32) -> u32 {
    if rd(rd(0xC978A8).wrapping_add(obj)) != 0 {
        panic!("rw_standin: RpSkin geometry destructor with a skin is not handled by the stand-in");
    }
    obj
}
fn skin_geometry_copy(dst: u32, _src: u32, _o: u32, _s: u32) -> u32 {
    dst
}
fn skin_atomic_ctor(obj: u32, _o: u32, _s: u32) -> u32 {
    wr(rd(0xC978A4).wrapping_add(obj), 0);
    obj
}
fn skin_atomic_dtor(obj: u32, _o: u32, _s: u32) -> u32 {
    let a = rd(0xC978A4).wrapping_add(obj);
    if rd(a) != 0 {
        wr(a, 0);
    }
    obj
}
fn skin_atomic_copy(dst: u32, src: u32, _o: u32, _s: u32) -> u32 {
    let off = rd(0xC978A4);
    wr(off.wrapping_add(dst), rd(off.wrapping_add(src)));
    dst
}

/// MatFX material/atomic/sector extension callbacks (0x810C40, 0x810C60 no-data case, 0x810D20
/// no-data case, 0x811A00/20/40, 0x811B00/20/40), transcribed.
fn matfx_material_ctor(obj: u32, _o: u32, _s: u32) -> u32 {
    wr(rd(0xC9AB74).wrapping_add(obj), 0);
    obj
}
fn matfx_material_dtor(obj: u32, _o: u32, _s: u32) -> u32 {
    if rd(rd(0xC9AB74).wrapping_add(obj)) != 0 {
        panic!("rw_standin: RpMatFX material destructor with effects is not handled by the stand-in");
    }
    obj
}
fn matfx_material_copy(_dst: u32, src: u32, _o: u32, _s: u32) -> u32 {
    if rd(rd(0xC9AB74).wrapping_add(src)) != 0 {
        panic!("rw_standin: RpMatFX material copy with effects is not handled by the stand-in");
    }
    0
}
fn matfx_atomic_ctor(obj: u32, _o: u32, _s: u32) -> u32 {
    wr(rd(0xC9AB84).wrapping_add(obj), 0);
    obj
}
fn matfx_atomic_copy(dst: u32, src: u32, _o: u32, _s: u32) -> u32 {
    let off = rd(0xC9AB84);
    if rd(off.wrapping_add(src)) != 0 {
        wr(off.wrapping_add(dst), 1);
    }
    dst
}
fn matfx_sector_ctor(obj: u32, _o: u32, _s: u32) -> u32 {
    wr(rd(0xC9AB88).wrapping_add(obj), 0);
    obj
}
fn matfx_sector_copy(dst: u32, src: u32, _o: u32, _s: u32) -> u32 {
    let off = rd(0xC9AB88);
    if rd(off.wrapping_add(src)) != 0 {
        wr(off.wrapping_add(dst), 1);
    }
    dst
}

/// `RpAnisotPluginAttach()` (0x748F70), transcribed: engine module (0, 0x127, 0x749000,
/// 0x749010), the 1-byte texture extension → 0xC92478 (ctor 0x749020, dtor 0x749030, copy
/// 0x749040) with its stream → 0xC9247C, then `_rwD3D9SetMaxAnisotropy(offset)` (0xC9A5E8). 1.
fn rp_anisot_plugin_attach() -> i32 {
    if engine_register_plugin(0, 0x127, 0x749000, 0x749010) < 0 {
        return 0;
    }
    let o = texture_register_plugin(1, 0x127, 0x749020, 0x749030, 0x749040);
    wr(0xC92478, o as u32);
    if o < 0 {
        return 0;
    }
    let s = texture_register_plugin_stream(0x127, 0x749060, 0x7490B0, 0x7490F0);
    wr(0xC9247C, s as u32);
    if s < 0 {
        return 0;
    }
    wr(0xC9A5E8, rd(0xC92478));
    1
}
fn anisot_engine_open(instance: u32, _o: u32, _s: u32) -> u32 {
    wr(0xC92474, rd(0xC92474).wrapping_add(1));
    instance
}
fn anisot_engine_close(instance: u32, _o: u32, _s: u32) -> u32 {
    wr(0xC92474, rd(0xC92474).wrapping_sub(1));
    instance
}


macro_rules! cdecl_entries {
    ($( $addr:literal, $name:literal => $f:path : fn($($a:ident : $t:ty),*) -> $r:ty ;)*) => {
        /// `(address, name, native entry)` of every stand-in.
        pub fn entries() -> Vec<(u32, &'static str, usize)> {
            let mut v: Vec<(u32, &'static str, usize)> = Vec::new();
            $({
                extern "C-unwind" fn w($($a: $t),*) -> $r {
                    $f($($a),*)
                }
                v.push(($addr, $name, w as *const () as usize));
            })*
            v
        }
    };
}

cdecl_entries! {
    0x007C_69F0, "RpSkin geometry Ctor (stand-in)" => skin_geometry_ctor: fn(obj: u32, o: u32, s: u32) -> u32;
    0x007C_6A10, "RpSkin geometry Dtor (stand-in)" => skin_geometry_dtor: fn(obj: u32, o: u32, s: u32) -> u32;
    0x007C_6A70, "RpSkin geometry Copy (stand-in)" => skin_geometry_copy: fn(dst: u32, src: u32, o: u32, s: u32) -> u32;
    0x007C_6A80, "RpSkin atomic Ctor (stand-in)" => skin_atomic_ctor: fn(obj: u32, o: u32, s: u32) -> u32;
    0x007C_6AA0, "RpSkin atomic Dtor (stand-in)" => skin_atomic_dtor: fn(obj: u32, o: u32, s: u32) -> u32;
    0x007C_6AC0, "RpSkin atomic Copy (stand-in)" => skin_atomic_copy: fn(dst: u32, src: u32, o: u32, s: u32) -> u32;
    0x0081_0C40, "RpMatFX material Ctor (stand-in)" => matfx_material_ctor: fn(obj: u32, o: u32, s: u32) -> u32;
    0x0081_0C60, "RpMatFX material Dtor (stand-in)" => matfx_material_dtor: fn(obj: u32, o: u32, s: u32) -> u32;
    0x0081_0D20, "RpMatFX material Copy (stand-in)" => matfx_material_copy: fn(dst: u32, src: u32, o: u32, s: u32) -> u32;
    0x0081_1A00, "RpMatFX atomic Ctor (stand-in)" => matfx_atomic_ctor: fn(obj: u32, o: u32, s: u32) -> u32;
    0x0081_1A20, "RpMatFX atomic Dtor (stand-in)" => matfx_atomic_ctor: fn(obj: u32, o: u32, s: u32) -> u32;
    0x0081_1A40, "RpMatFX atomic Copy (stand-in)" => matfx_atomic_copy: fn(dst: u32, src: u32, o: u32, s: u32) -> u32;
    0x0081_1B00, "RpMatFX sector Ctor (stand-in)" => matfx_sector_ctor: fn(obj: u32, o: u32, s: u32) -> u32;
    0x0081_1B20, "RpMatFX sector Dtor (stand-in)" => matfx_sector_ctor: fn(obj: u32, o: u32, s: u32) -> u32;
    0x0081_1B40, "RpMatFX sector Copy (stand-in)" => matfx_sector_copy: fn(dst: u32, src: u32, o: u32, s: u32) -> u32;
    0x0074_8F70, "RpAnisotPluginAttach (stand-in)" => rp_anisot_plugin_attach: fn() -> i32;
    0x0074_9000, "RpAnisot engine Open (stand-in)" => anisot_engine_open: fn(instance: u32, o: u32, s: u32) -> u32;
    0x0074_9010, "RpAnisot engine Close (stand-in)" => anisot_engine_close: fn(instance: u32, o: u32, s: u32) -> u32;
}
