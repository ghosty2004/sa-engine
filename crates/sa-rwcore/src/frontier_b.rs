//! FRONTIER-2 lane B (batches `frontier2-b2`, `frontier2-b4`) and lane C (`frontier2-c17`: the RtDict stream read): RenderWare functions on the phase B frontier: world-sector teardown and
//! lookup (`_rpWorldSectorDeinstanceAll`, `_rpWorldSectorDestroyRecurse`, `RpWorldSectorGetWorld`,
//! `RwCameraForAllSectorsInFrustum`, `RpWorldAddClump`, the D3D9 geometry-list close), geometry / atomic helpers
//! (`_rpGeometryStructGetSize`, the geometry free helper, `RpAtomicInstance`), the UV-animation material plugin,
//! string streams, free lists and resources (`_rwSListGetNewEntry`, `RwFreeListForAllUsed`, `_rwResourcesClose`),
//! the image / format helpers (`_rwFileNameIsAbsolute`, the RGBA-to-pixel converter lookup, the same-depth image
//! copy, the MatFX bumped-texture check) and `_rwMatrixIdentityError`. Ported from the 1.0 US bytes (RenderWare is
//! not reversed in gta-reversed).

use sa_core::abi::{HookReg, NativeReg};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::bindings::{
    clump_for_all_cameras, free_list_destroy, mesh_destroy, plugin_deinit_object, resources_free_res_entry,
    rp_clump_for_all_atomics, rw_frame_update_objects, rw_stream_write, stream_write_chunk_header,
    atomic_set_default_pipeline,
};
use crate::common::{g, p8, rd, rw_error2, rw_free, rw_globals, rw_malloc, rw_memory_free, rw_realloc, wr};
use sa_core::bindings::{material_list_deinitialize, rw_error_set, rw_matrix_rotate, rw_matrix_translate, rx_pipeline_destroy};

orig! {
    /// `G->rwstrlen(s)` (`RwGlobals` +0x120)
    fn rw_strlen(s: GPtr<u8>) -> u32 = cdecl table 0xC97B24 0x120 => unported;
    /// A `RwCameraForAllSectorsInFrustum` / `RwFreeListForAllUsed` callback `(object, data)`
    fn call_cb2(f: u32, obj: u32, data: u32) -> u32 = cdecl dyn => unported;
    /// `RpAtomic::renderCallBack(atomic)` (+0x48) as `RpAtomicInstance` calls it
    fn call_render_cb(f: u32, atomic: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
    /// `RtAnimInterpolatorAddAnimTime(interp, time)` (0x7CD8D0)
    fn anim_interpolator_add_anim_time(interp: GPtr<u8>, t: u32) -> u32 = cdecl 0x7CD8D0 => unported;
    /// `_rpWorldSectorLightTieDestroy(tie)` (0x74FFF0)
    fn sector_light_tie_destroy(tie: GPtr<u8>) -> u32 = cdecl 0x74FFF0 => rp_world_sector_light_tie_destroy;
    /// `_rpWorldSectorTieDestroy(tie)` (0x74FFA0)
    fn sector_tie_destroy(tie: GPtr<u8>) -> u32 = cdecl 0x74FFA0 => rp_world_sector_tie_destroy;
    /// `_rpD3D9VertexShaderCachePurge()` (0x75EE60)
    fn vertex_shader_cache_purge() = cdecl 0x75EE60 => crate::d3d9misc::vertex_shader_cache_purge;
    /// `_rwD3D9LightsClose()` (0x755FE0)
    fn d3d9_lights_close() = cdecl 0x755FE0 => crate::d3d9light::rw_d3d9_lights_close;
    /// `RwD3D9RasterIsCompressed(raster)` (0x820C90)
    fn raster_is_compressed(r: GPtr<u8>) -> u32 = cdecl 0x820C90 => crate::raster::d3d9_raster_is_compressed;
    /// The MatFX bumped-texture builder's worker (0x810830): `(base, bump)`
    fn matfx_build_bumped_texture(base: GPtr<u8>, bump: GPtr<u8>) -> GPtr<u8> = cdecl 0x810830 => unported;
}

/// The `RpWorldSector` plugin registry (0x8D736C) and the `RpGeometry` one (0x8D628C).
const SECTOR_REGISTRY: usize = 0x8D_736C;
const GEOMETRY_REGISTRY: usize = 0x8D_628C;
/// The empty string `""` (0x884F6A).
const EMPTY_STRING: u32 = 0x88_4F6A;
/// The UV-animation material plugin offset (0xC9B8D0).
const UVANIM_OFFSET: usize = 0xC9_B8D0;

fn off(a: usize) -> isize {
    g::<i32>(a).read() as isize
}

// ---------------------------------------------------------------------------------------------------------------
// World sectors

/// The sector teardown both recursions share for a leaf (type -1): the resource entry (+0x34) freed, the light ties
/// (ring +0x38) and the object ties (ring +0x40) destroyed, the plugin data deinitialised.
fn sector_leaf_untie(s: GPtr<u8>) {
    let res: GPtr<u8> = rd(s, 0x34);
    if !res.is_null() {
        resources_free_res_entry(res);
    }
    for (head_off, f) in [(0x38isize, sector_light_tie_destroy as fn(GPtr<u8>) -> u32), (0x40, sector_tie_destroy)] {
        let head = s.byte_add(head_off);
        let mut n: GPtr<u8> = rd(head, 0);
        while n != head {
            let next: GPtr<u8> = rd(n, 0);
            f(n);
            n = next;
        }
    }
    plugin_deinit_object(p8(SECTOR_REGISTRY as u32), s);
}

/// `_rpWorldSectorDeinstanceAll(RpSector*)` (0x74ECA0): walks the sector tree (type +0: -2 empty, -1 a world sector,
/// else a plane with its children +8 / +0xC; the left child recursively, the right one iteratively); each world
/// sector is untied and its mesh header (+0x78) destroyed.
#[hook(addr = 0x74ECA0, conv = cdecl, name = "_rpWorldSectorDeinstanceAll", expect = "56 8B 74 24 08",
       unverified = "frontier2-b2", route = "L2", hazards = "heap-writes")]
pub fn rp_world_sector_deinstance_all(sector: GPtr<u8>) {
    let mut s = sector;
    loop {
        let t: i32 = rd(s, 0);
        if t == -2 {
            return;
        }
        if t == -1 {
            sector_leaf_untie(s);
            let mesh: GPtr<u8> = rd(s, 0x78);
            if !mesh.is_null() {
                mesh_destroy(mesh);
                wr::<u32>(s, 0x78, 0);
            }
            return;
        }
        let left: GPtr<u8> = rd(s, 8);
        if !left.is_null() {
            rp_world_sector_deinstance_all_call(left);
        }
        s = rd(s, 0xC);
        if s.is_null() {
            return;
        }
    }
}

use crate::worldcreate::{sector_deinstance_all as rp_world_sector_deinstance_all_call, sector_destroy_recurse as rp_world_sector_destroy_recurse_call};

/// `_rpWorldSectorDestroyRecurse(RpSector*)` (0x74ED50): an empty sector (-2) is freed; a plane's children are
/// destroyed recursively (each pointer cleared after), then the plane freed; a world sector is untied, its arrays
/// (+8, +0xC, +0x30, +4 and the eight +0x10..+0x2C) freed and cleared, its mesh header (+0x78) destroyed, the sector
/// freed.
#[hook(addr = 0x74ED50, conv = cdecl, name = "_rpWorldSectorDestroyRecurse", expect = "55 56 8B 74 24 0C",
       unverified = "frontier2-b2", route = "L2", hazards = "heap-writes")]
pub fn rp_world_sector_destroy_recurse(s: GPtr<u8>) {
    let t: i32 = rd(s, 0);
    if t == -2 {
        rw_free(s);
        return;
    }
    if t != -1 {
        let left: GPtr<u8> = rd(s, 8);
        if !left.is_null() {
            rp_world_sector_destroy_recurse_call(left);
        }
        let right: GPtr<u8> = rd(s, 0xC);
        wr::<u32>(s, 8, 0);
        if !right.is_null() {
            rp_world_sector_destroy_recurse_call(right);
        }
        wr::<u32>(s, 0xC, 0);
        rw_free(s);
        return;
    }
    sector_leaf_untie(s);
    for o in [8isize, 0xC, 0x30, 4, 0x10, 0x14, 0x18, 0x1C, 0x20, 0x24, 0x28, 0x2C] {
        let p: GPtr<u8> = rd(s, o);
        if !p.is_null() {
            rw_free(p);
            wr::<u32>(s, o, 0);
        }
    }
    let mesh: GPtr<u8> = rd(s, 0x78);
    if !mesh.is_null() {
        mesh_destroy(mesh);
        wr::<u32>(s, 0x78, 0);
    }
    rw_free(s);
}

/// `RpWorldSectorGetWorld(const RpWorldSector*)` (0x74F4E0): over the world module's world list (`G + [0xC9254C] +
/// 4`, a ring whose links sit 8 bytes into each entry: +0 world, +4 size): a world with the flag bit 0 of byte +3
/// owns the sector when it lies inside `[world, world + size)`; otherwise the world's sector tree (root +0x1C) is
/// searched depth first (left +8 followed, right +0xC pushed on a 64-entry stack) for the sector. Null when no world
/// has it.
#[hook(addr = 0x74F4E0, conv = cdecl, name = "RpWorldSectorGetWorld", expect = "A1 24 7B C9 00",
       unverified = "frontier2-b2", route = "L2", hazards = "none")]
pub fn rp_world_sector_get_world(sector: GPtr<u8>) -> GPtr<u8> {
    let head = rw_globals().byte_add(off(0xC9_254C) + 4);
    let target = sector.addr() as u32;
    let mut stack = [0u32; 64];
    let mut n: GPtr<u8> = rd(head, 0);
    while n != head {
        let world: u32 = rd(n, -8);
        if rd::<u8>(p8(world), 3) & 1 != 0 {
            if target >= world && target < world.wrapping_add(rd::<u32>(n, -4)) {
                return p8(world);
            }
        } else {
            let mut e: u32 = rd(p8(world), 0x1C);
            let mut i = 0i32;
            loop {
                if rd::<i32>(p8(e), 0) < 0 {
                    if target == e {
                        return p8(rd(n, -8));
                    }
                    e = stack[(i & 63) as usize];
                    i -= 1;
                } else {
                    let node = e;
                    e = rd(p8(node), 8);
                    i += 1;
                    stack[(i & 63) as usize] = rd(p8(node), 0xC);
                }
                if i < 0 {
                    break;
                }
            }
        }
        n = rd(n, 0);
    }
    GPtr::NULL
}

/// `RwCameraForAllSectorsInFrustum(RwCamera*, callback, data)` (0x751660): the camera's frustum sector list (`camera +
/// [0xC925A4]`: +0 the array, +8 the count) is walked; a callback returning null stops it. Returns the camera.
#[hook(addr = 0x751660, conv = cdecl, name = "RwCameraForAllSectorsInFrustum", expect = "A1 A4 25 C9 00",
       unverified = "frontier2-b2", route = "L2", hazards = "none")]
pub fn rw_camera_for_all_sectors_in_frustum(camera: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> {
    let ext = camera.byte_add(off(0xC9_25A4));
    let mut p: GPtr<u8> = rd(ext, 0);
    let mut n: u32 = rd(ext, 8);
    while n != 0 {
        if call_cb2(cb, rd(p, 0), data) == 0 {
            return camera;
        }
        p = p.byte_add(4);
        n -= 1;
    }
    camera
}

use crate::camera::matrix_optimize;
use crate::clumpstream::clump_for_all_lights;

/// `RpWorldAddClump(RpWorld*, RpClump*)` (0x751300): the clump is linked at the head of the world's clump ring
/// (+0x2C; the clump's link +0x20), the world's clump count (+0x24) incremented, the clump's world extension (`clump
/// + [0xC925AC]`) set to the world; its atomics (0x751390), lights (0x7513D0) and cameras (0x751420) are added; its
/// frame (+4), when present, gets `RwMatrixOptimize(ltm, NULL)` and `RwFrameUpdateObjects`; the extension's +4 = the
/// world module's +8 (`G + [0xC9259C]`). Returns the world.
#[hook(addr = 0x751300, conv = cdecl, name = "RpWorldAddClump", expect = "A1 AC 25 C9 00",
       unverified = "frontier2-b2", route = "L2", hazards = "heap-writes")]
pub fn rp_world_add_clump(world: GPtr<u8>, clump: GPtr<u8>) -> GPtr<u8> {
    let ext = clump.byte_add(off(0xC9_25AC));
    let frame: GPtr<u8> = rd(clump, 4);
    let old: u32 = rd(world, 0x2C);
    let link = clump.byte_add(0x20);
    wr::<u32>(clump, 0x24, world.byte_add(0x2C).addr() as u32);
    wr::<u32>(link, 0, old);
    wr::<u32>(p8(rd(world, 0x2C)), 4, link.addr() as u32);
    wr::<u32>(world, 0x2C, link.addr() as u32);
    wr::<i32>(world, 0x24, rd::<i32>(world, 0x24).wrapping_add(1));
    wr::<u32>(ext, 0, world.addr() as u32);
    let w = world.addr() as u32;
    rp_clump_for_all_atomics(clump, 0x75_1390, w);
    clump_for_all_lights(clump, 0x75_13D0, w);
    clump_for_all_cameras(clump, 0x75_1420, w);
    if !frame.is_null() {
        matrix_optimize(frame.byte_add(0x10), GPtr::NULL);
        rw_frame_update_objects(frame);
    }
    wr::<u32>(ext, 4, rd(rw_globals(), off(0xC9_259C) + 8));
    world
}

/// `_rpD3D9GeometryListDestroy()` (0x74EC00): every entry of the geometry list (0xC92520, next +4) is cleared (+0)
/// and returned to its free list (0xC92524) through `G->memoryFree`; the list head = 0; the free list destroyed and
/// cleared; the vertex-shader cache purged, the D3D9 lights closed, the default atomic pipeline reset; the world
/// module's pipeline (+0x54 of `G + [0xC9BC60]`) destroyed and cleared. EAX: 0 without a pipeline, else
/// `_rxPipelineDestroy`'s (its `G->memoryFree` result); the only consumer (`_rpWorldPipeClose`'s caller) ignores it,
/// so the port returns the module address there.
#[hook(addr = 0x74EC00, conv = cdecl, name = "FUN_0074ec00", expect = "56 8B 35 20 25 C9 00",
       unverified = "frontier2-b2", route = "L2", hazards = "heap-writes")]
pub fn rp_d3d9_geometry_list_destroy() -> u32 {
    let list = g::<GPtr<u8>>(0xC9_2520);
    let fl = g::<GPtr<u8>>(0xC9_2524);
    let mut e = list.read();
    if !e.is_null() {
        while !e.is_null() {
            wr::<u32>(e, 0, 0);
            rw_memory_free(fl.read(), e);
            e = rd(e, 4);
        }
        list.write(e);
    }
    if !fl.read().is_null() {
        free_list_destroy(fl.read());
        fl.write(GPtr::NULL);
    }
    vertex_shader_cache_purge();
    d3d9_lights_close();
    atomic_set_default_pipeline(GPtr::NULL);
    let m = rw_globals().byte_add(off(0xC9_BC60));
    let pipe: GPtr<u8> = rd(m, 0x54);
    if pipe.is_null() {
        return 0;
    }
    rx_pipeline_destroy(pipe);
    let m = rw_globals().byte_add(off(0xC9_BC60));
    wr::<u32>(m, 0x54, 0);
    m.addr() as u32
}

// ---------------------------------------------------------------------------------------------------------------
// Geometry / atomics

/// `_rpGeometryStructGetSize(const RpGeometry*)` (0x74CE60): 0x28 for a native geometry (flag 0x1000000 in +8); else
/// 0x10, with vertices (+0x14) `4 * verts + 0x10` for prelit (flag 8), plus `8 * (texCoordSets (+0x1C) * verts +
/// triangles (+0x10))`; per morph target (+0x18 of them, 0x1C bytes from +0x5C) 0x18 plus `12 * verts` for points
/// (+0x14 of the target) and for normals (+0x18).
#[hook(addr = 0x74CE60, conv = cdecl, name = "_rpGeometryStructGetSize", expect = "8B 4C 24 04 56",
       unverified = "frontier2-b2", route = "L2", hazards = "none")]
pub fn rp_geometry_struct_get_size(geo: GPtr<u8>) -> u32 {
    let flags: u32 = rd(geo, 8);
    if flags & 0x100_0000 != 0 {
        return 0x28;
    }
    let mut size = 0x10u32;
    let verts: u32 = rd(geo, 0x14);
    if verts != 0 {
        if flags & 8 != 0 {
            size = verts.wrapping_mul(4).wrapping_add(0x10);
        }
        let n = rd::<u32>(geo, 0x1C).wrapping_mul(verts).wrapping_add(rd(geo, 0x10));
        size = size.wrapping_add(n.wrapping_mul(8));
    }
    let mut targets: i32 = rd(geo, 0x18);
    if targets <= 0 {
        return size;
    }
    let mut t = p8(rd(geo, 0x5C)).byte_add(0x18);
    while targets != 0 {
        size = size.wrapping_add(0x18);
        if rd::<u32>(t, -4) != 0 {
            size = size.wrapping_add(verts.wrapping_mul(12));
        }
        if rd::<u32>(t, 0) != 0 {
            size = size.wrapping_add(verts.wrapping_mul(12));
        }
        t = t.byte_add(0x1C);
        targets -= 1;
    }
    size
}

/// The geometry free helper (0x74D6D0, cdecl): the geometry's lock word +0xE is raised while it runs and its flags
/// +0xC get 0xFFF; the mesh header (+0x54) destroyed and cleared, the plugin data deinitialised, the pre-light /
/// texture-coordinate block (+0x5C) freed and cleared, the material list (+0x20) deinitialised, the lock dropped,
/// the geometry freed. Returns 1.
#[hook(addr = 0x74D6D0, conv = cdecl, name = "FUN_0074d6d0", expect = "56 8B 74 24 08",
       unverified = "frontier2-b2", route = "L2", hazards = "heap-writes")]
pub fn rp_geometry_free(geo: GPtr<u8>) -> u32 {
    let mesh: GPtr<u8> = rd(geo, 0x54);
    wr::<u16>(geo, 0xE, rd::<u16>(geo, 0xE).wrapping_add(1));
    wr::<u16>(geo, 0xC, rd::<u16>(geo, 0xC) | 0xFFF);
    if !mesh.is_null() {
        mesh_destroy(mesh);
        wr::<u32>(geo, 0x54, 0);
    }
    plugin_deinit_object(p8(GEOMETRY_REGISTRY as u32), geo);
    let mt: GPtr<u8> = rd(geo, 0x5C);
    if !mt.is_null() {
        rw_free(mt);
        wr::<u32>(geo, 0x5C, 0);
    }
    material_list_deinitialize(geo.byte_add(0x20));
    wr::<u16>(geo, 0xE, rd::<u16>(geo, 0xE).wrapping_sub(1));
    rw_free(geo);
    1
}

/// `RpAtomicInstance(RpAtomic*)` (0x74BF40): 0 unless the geometry (+0x18) has one morph target (+0x18 == 1); when
/// it is not native yet (flag 0x1000000 of +8): its resource entry (+0x58) freed, the flag 0x2000000 set while the
/// atomic's render callback (+0x48) runs, then 0x2000000 cleared and 0x1000000 set. Returns 1.
#[hook(addr = 0x74BF40, conv = cdecl, name = "RpAtomicInstance", expect = "56 57 8B 7C 24 0C",
       unverified = "frontier2-b2", route = "L2", hazards = "heap-writes")]
pub fn rp_atomic_instance(atomic: GPtr<u8>) -> u32 {
    let geo: GPtr<u8> = rd(atomic, 0x18);
    if rd::<i32>(geo, 0x18) != 1 {
        return 0;
    }
    if rd::<u32>(geo, 8) & 0x100_0000 == 0 {
        let res: GPtr<u8> = rd(geo, 0x58);
        if !res.is_null() {
            resources_free_res_entry(res);
        }
        wr::<u32>(geo, 8, rd::<u32>(geo, 8) | 0x200_0000);
        call_render_cb(rd(atomic, 0x48), atomic);
        wr::<u32>(geo, 8, (rd::<u32>(geo, 8) & 0xFDFF_FFFF) | 0x100_0000);
    }
    1
}

// ---------------------------------------------------------------------------------------------------------------
// UV-animation materials

/// The material's eight UV-animation interpolator slots (`material + [0xC9B8D0] + 8`).
fn uv_slots(m: GPtr<u8>) -> GPtr<u8> {
    m.byte_add(off(UVANIM_OFFSET) + 8)
}

/// `RpMaterialUVAnimExists(const RpMaterial*)` (0x7CC530): 1 when any of the eight slots is set.
#[hook(addr = 0x7CC530, conv = cdecl, name = "RpMaterialUVAnimExists", expect = "8B 4C 24 04 8B 15 D0 B8 C9 00",
       unverified = "frontier2-b2", route = "L2", hazards = "none")]
pub fn rp_material_uv_anim_exists(m: GPtr<u8>) -> u32 {
    let s = uv_slots(m);
    u32::from((0..8isize).any(|i| rd::<u32>(s, 4 * i) != 0))
}

/// `RpMaterialUVAnimAddAnimTime(RpMaterial*, float)` (0x7CC4B0): `RtAnimInterpolatorAddAnimTime(slot, t)` for every
/// set slot. Returns the material.
#[hook(addr = 0x7CC4B0, conv = cdecl, name = "RpMaterialUVAnimAddAnimTime", expect = "A1 D0 B8 C9 00",
       unverified = "frontier2-b2", route = "L2", hazards = "none")]
pub fn rp_material_uv_anim_add_anim_time(m: GPtr<u8>, t: u32) -> GPtr<u8> {
    let s = uv_slots(m);
    for i in 0..8isize {
        let interp: GPtr<u8> = rd(s, 4 * i);
        if !interp.is_null() {
            anim_interpolator_add_anim_time(interp, t);
        }
    }
    m
}

/// The UV-animation keyframe size (0x7C8D40, cdecl): `0xC` without frames (+0x28 == 0), else `+0 + 2 * (+0x2C +
/// +0x28) + 0xC`.
#[hook(addr = 0x7C8D40, conv = cdecl, name = "FUN_007c8d40", expect = "8B 4C 24 04 B8 0C 00 00 00",
       unverified = "frontier2-b2", route = "L2", hazards = "none")]
pub fn uv_anim_stream_get_size(a: GPtr<u8>) -> u32 {
    let n: u32 = rd(a, 0x28);
    if n == 0 {
        return 0xC;
    }
    rd::<u32>(a, 0).wrapping_add(rd::<u32>(a, 0x2C).wrapping_add(n).wrapping_mul(2)).wrapping_add(0xC)
}

/// `FUN_007c8830` (0x7C8830, cdecl): returns 0 (`xor eax, eax; ret`; 3 bytes, not hookable).
#[sa_core::native(addr = 0x7C8830, conv = cdecl, name = "FUN_007c8830")]
pub fn uv_anim_return_zero(_a: u32) -> u32 {
    0
}

/// The UV-animation matrix decomposition (0x7CC760, cdecl `(float out[6], const RwMatrix* m)`): a copy of `*m`;
/// `out[0] = atan2(-m.up.x, m.right.x)` (`FPATAN`); the copy translated by 0x882ED4, rotated about z (0, 0, 1) by
/// `out[0] * [0x882F2C]` and translated by 0x882EE0 (all post-concatenated); `out[1..5] = (right.x, up.y, right.y,
/// pos.x, pos.y)` of the copy. Returns `out`.
#[hook(addr = 0x7CC760, conv = cdecl, name = "FUN_007cc760", expect = "83 EC 4C 56 8B 74 24 58",
       unverified = "frontier2-b2", route = "L2", hazards = "x87-transcendental")]
pub fn uv_anim_matrix_decompose(out: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> {
    let mut l = [0u32; 16];
    let lp: GPtr<u8> = GPtr::from_raw(l.as_mut_ptr()).cast();
    for i in 0..16isize {
        wr::<u32>(lp, 4 * i, rd(m, 4 * i));
    }
    let a = X87::from_f32(rd(lp, 0x10)).fchs();
    let angle = a.fpatan(X87::from_f32(rd(lp, 0)));
    let mut axis = [0u32, 0, 0x3F80_0000];
    wr(out, 0, angle.to_f32());
    rw_matrix_translate(lp, p8(0x88_2ED4), 2);
    let r = X87::from_f32(rd(out, 0)).fmul_m(g::<f32>(0x88_2F2C).read()).to_f32();
    rw_matrix_rotate(lp, GPtr::from_raw(axis.as_mut_ptr()).cast(), r, 2);
    rw_matrix_translate(lp, p8(0x88_2EE0), 2);
    wr::<u32>(out, 4, rd(lp, 0));
    wr::<u32>(out, 0xC, rd(lp, 4));
    wr::<u32>(out, 8, rd(lp, 0x14));
    wr::<u32>(out, 0x10, rd(lp, 0x30));
    wr::<u32>(out, 0x14, rd(lp, 0x34));
    out
}

/// The UV key frame interpolation shared by the two schemes: `r = (time - a.t) / (b.t - a.t)` (+4, kept in ST0),
/// then for each of the six values (+8 .. +0x1C) `out = (b - a) * r + a`, stored as f32; `wrap` first brings the
/// +8 angle difference into range (below -pi (0x858CC0): + 2pi (0x858CBC); above pi (0x858CB8): - 2pi). The
/// output's time (+4) is not written.
fn uv_key_frame_lerp(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, time: f32, wrap: bool) {
    let r = X87::from_f32(time)
        .fsub_m(rd(a, 4))
        .fdiv(X87::from_f32(rd(b, 4)).fsub_m(rd(a, 4)));
    for o in (8..0x20isize).step_by(4) {
        let mut d = X87::from_f32(rd(b, o)).fsub_m(rd(a, o));
        if wrap && o == 8 {
            if !d.fcom(X87::from_f32(g::<f32>(0x85_8CC0).read())).test_parity(5) {
                d = d.fadd_m(g::<f32>(0x85_8CBC).read());
            } else if !d.fcom(X87::from_f32(g::<f32>(0x85_8CB8).read())).test_any(0x41) {
                d = d.fsub_m(g::<f32>(0x85_8CBC).read());
            }
        }
        wr(out, o, d.fmul(r).fadd_m(rd(a, o)).to_f32());
    }
}

/// `RpUVAnimParamKeyFrameInterpolate(out, a, b, time, customData)` (0x7CC600): [`uv_key_frame_lerp`] with the
/// angle wrap.
#[hook(addr = 0x7CC600, conv = cdecl, name = "RpUVAnimParamKeyFrameInterpolate", expect = "8B 4C 24 08 8B 54 24 0C",
       unverified = "frontier2-c17", route = "L2", hazards = "none")]
pub fn uv_anim_param_key_frame_interpolate(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, time: f32, _custom: u32) {
    uv_key_frame_lerp(out, a, b, time, true);
}

/// `RpUVAnimLinearKeyFrameInterpolate(out, a, b, time, customData)` (0x7CCA40): [`uv_key_frame_lerp`].
#[hook(addr = 0x7CCA40, conv = cdecl, name = "RpUVAnimLinearKeyFrameInterpolate", expect = "8B 44 24 08 8B 4C 24 0C",
       unverified = "frontier2-c17", route = "L2", hazards = "none")]
pub fn uv_anim_linear_key_frame_interpolate(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, time: f32, _custom: u32) {
    uv_key_frame_lerp(out, a, b, time, false);
}

/// `RpUVAnimParamKeyFrameApply(RwMatrix* m, const RpUVAnimParamKeyFrameData* kf)` (0x7CC560): right = (kf +0xC,
/// +0x14, 0), up = (0, kf +0x10, 0), at = 0, pos = (kf +0x18, +0x1C, 0) (flags and pads untouched),
/// `RwMatrixUpdate`, then translated by 0x882ED4, rotated about z (0, 0, 1) by `kf +8 * [0x859878]` (degrees)
/// and translated by 0x882EE0 (all post-concatenated): the inverse of [`uv_anim_matrix_decompose`].
#[hook(addr = 0x7CC560, conv = cdecl, name = "RpUVAnimParamKeyFrameApply", expect = "83 EC 0C 56 8B 74 24 14",
       unverified = "frontier2-c17", route = "L2", hazards = "none")]
pub fn uv_anim_param_key_frame_apply(m: GPtr<u8>, kf: GPtr<u8>) {
    let mut axis = [0u32, 0, 0];
    wr::<u32>(m, 0, rd(kf, 0xC));
    wr::<u32>(m, 4, rd(kf, 0x14));
    for o in [8, 0x10, 0x18] {
        wr::<u32>(m, o, 0);
    }
    wr::<u32>(m, 0x14, rd(kf, 0x10));
    for o in [0x20, 0x24, 0x28] {
        wr::<u32>(m, o, 0);
    }
    axis[2] = 0x3F80_0000;
    wr::<u32>(m, 0x30, rd(kf, 0x18));
    wr::<u32>(m, 0x34, rd(kf, 0x1C));
    wr::<u32>(m, 0x38, 0);
    sa_core::bindings::rw_matrix_update(m);
    rw_matrix_translate(m, p8(0x88_2ED4), 2);
    let angle = X87::from_f32(rd(kf, 8)).fmul_m(g::<f32>(0x85_9878).read()).to_f32();
    rw_matrix_rotate(m, GPtr::from_raw(axis.as_mut_ptr()).cast(), angle, 2);
    rw_matrix_translate(m, p8(0x88_2EE0), 2);
}

// ---------------------------------------------------------------------------------------------------------------
// Strings, lists, resources

/// The stream size of a string (`""` for null): `(strlen + 4) & ~3` (only AL is masked; the same value).
fn string_size(s: GPtr<u8>) -> u32 {
    let s = if s.is_null() { p8(EMPTY_STRING) } else { s };
    rw_strlen(s).wrapping_add(4) & !3
}

/// `_rwStringStreamGetSize(const RwChar*)` (0x80A470).
#[hook(addr = 0x80A470, conv = cdecl, name = "_rwStringStreamGetSize", expect = "8B 44 24 04 85 C0",
       unverified = "frontier2-b2", route = "L2", hazards = "none")]
pub fn rw_string_stream_get_size(s: GPtr<u8>) -> u32 {
    string_size(s)
}

/// `_rwStringStreamWrite(const RwChar*, RwStream*)` (0x80A4A0): a string chunk header (type 2, the padded size,
/// version 0x36003, build 0xFFFF), then the padded size of bytes from the string (`""` for null). Returns the string
/// (`""` for null) or null when a write fails.
#[hook(addr = 0x80A4A0, conv = cdecl, name = "_rwStringStreamWrite", expect = "53 56 57 8B 7C 24 10",
       unverified = "frontier2-b2", route = "L2", hazards = "none")]
pub fn rw_string_stream_write(s: GPtr<u8>, stream: GPtr<u8>) -> GPtr<u8> {
    let s = if s.is_null() { p8(EMPTY_STRING) } else { s };
    let size = (rw_strlen(s).wrapping_add(4)) & !3;
    if stream_write_chunk_header(stream, 2, size, 0x36003, 0xFFFF).is_null() {
        return GPtr::NULL;
    }
    if rw_stream_write(stream, s, size).is_null() {
        GPtr::NULL
    } else {
        s
    }
}

/// `_rwSListGetNewEntry(RwSList*, hint)` (0x809240): `{+0 entries, +4 count, +8 capacity, +0xC entry size}`; a full
/// list grows by a quarter (`cap + cap / 4`, signed) with `G->rwrealloc` (failure: `_rwerror(0x80000013, bytes)`
/// set as a plugin-1 error, null); returns `entries + count++ * size`.
#[hook(addr = 0x809240, conv = cdecl, name = "_rwSListGetNewEntry", expect = "83 EC 08 56 8B 74 24 10",
       unverified = "frontier2-b2", route = "L2", hazards = "none")]
pub fn rw_slist_get_new_entry(sl: GPtr<u8>, hint: u32) -> GPtr<u8> {
    let cap: i32 = rd(sl, 8);
    if rd::<i32>(sl, 4) >= cap {
        let grown = (cap / 4).wrapping_add(cap);
        let p = rw_realloc(rd(sl, 0), (grown.wrapping_mul(rd(sl, 0xC))) as u32, hint);
        let cap: i32 = rd(sl, 8);
        if p.is_null() {
            let grown = (cap / 4).wrapping_add(cap);
            let mut err = [1u32, 0];
            err[1] = rw_error2(0x8000_0013, grown.wrapping_mul(rd(sl, 0xC)) as u32);
            rw_error_set(GPtr::from_raw(err.as_mut_ptr()).cast());
            return GPtr::NULL;
        }
        wr(sl, 0, p);
        wr(sl, 8, (cap / 4).wrapping_add(cap));
    }
    let n: i32 = rd(sl, 4);
    let e = rd::<u32>(sl, 0xC).wrapping_mul(n as u32).wrapping_add(rd(sl, 0));
    wr(sl, 4, n.wrapping_add(1));
    p8(e)
}

/// `RwFreeListForAllUsed(RwFreeList*, callback, data)` (0x801E90): for each block of the ring +0x10 (next +0) a copy
/// of its used-bitmap (`+8` bytes from block +8, `G->rwmalloc(n, 0x10000)`; failure → null) is scanned, each set bit
/// (MSB first) giving the entry `align_up(block + align (+0xC) + n + 7, align) + index * entry size (+0)` to the
/// callback; the copy freed. Returns the free list. The original keeps the current block in its first argument's
/// slot.
#[hook(addr = 0x801E90, conv = cdecl, name = "RwFreeListForAllUsed", expect = "83 EC 0C 53 55",
       unverified = "frontier2-b2", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the current block is kept in its own first argument slot")]
pub fn rw_free_list_for_all_used(fl: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> {
    let head = fl.byte_add(0x10);
    let n: u32 = rd(fl, 8);
    let mut block: GPtr<u8> = rd(fl, 0x10);
    while block != head {
        let bits = rw_malloc(n, 0x1_0000);
        if bits.is_null() {
            return GPtr::NULL;
        }
        for i in 0..n as isize {
            wr::<u8>(bits, i, rd(block, 8 + i));
        }
        let next: GPtr<u8> = rd(block, 0);
        for i in 0..n {
            let byte = u32::from(rd::<u8>(bits, i as isize));
            if byte == 0 {
                continue;
            }
            for j in 0..8u32 {
                if byte & (0x80 >> j) != 0 {
                    let align: u32 = rd(fl, 0xC);
                    let base = (align.wrapping_add(block.addr() as u32).wrapping_add(n).wrapping_add(7)) & !align.wrapping_sub(1);
                    let e = base.wrapping_add(j.wrapping_add(i.wrapping_mul(8)).wrapping_mul(rd(fl, 0)));
                    call_cb2(cb, e, data);
                }
            }
        }
        rw_free(bits);
        block = next;
    }
    fl
}

/// `_rwResourcesClose()` (0x808320): with a resources free list (0xC9A6E4): `RwFreeListForAllUsed(list, 0x8083F0,
/// list)`; unless `G->memoryAlloc` is the default (0x801C30), every arena (array 0xC9A6E8, count 0xC9A6EC) has its
/// chain of entries (+0x10, next +0x30) given back through `G->memoryFree(NULL, entry)` and the arena's last-entry
/// record (+0x38 of the first entry) reset (+0x10 = 0, +0 = +4, +0x14 = 0); the array freed and cleared; the free
/// list destroyed and cleared. Returns 1.
#[hook(addr = 0x808320, conv = cdecl, name = "FUN_00808320", expect = "A1 E4 A6 C9 00",
       unverified = "frontier2-b2", route = "L2", hazards = "heap-writes")]
pub fn rw_resources_close() -> u32 {
    let fl = g::<GPtr<u8>>(0xC9_A6E4);
    if fl.read().is_null() {
        return 1;
    }
    crate::bindings::free_list_for_all_used(fl.read(), 0x80_83F0, fl.read().addr() as u32);
    if rd::<u32>(rw_globals(), 0x144) != 0x80_1C30 {
        let arr = g::<GPtr<u8>>(0xC9_A6E8);
        let mut i = 0u32;
        while i < g::<u32>(0xC9_A6EC).read() {
            let a: GPtr<u8> = rd(arr.read(), 4 * i as isize);
            let mut e: GPtr<u8> = rd(a, 0x10);
            if !e.is_null() {
                let last: GPtr<u8> = rd(e, 0x38);
                loop {
                    let next: GPtr<u8> = rd(e, 0x30);
                    rw_memory_free(GPtr::NULL, e);
                    e = next;
                    if e.is_null() {
                        break;
                    }
                }
                if !last.is_null() && rd::<u32>(last, 0x10) != 0 {
                    wr::<u32>(last, 0x10, 0);
                    wr::<u32>(last, 0, rd(last, 4));
                    wr::<u32>(last, 0x14, 0);
                }
            }
            i += 1;
        }
        if !arr.read().is_null() {
            rw_free(arr.read());
            arr.write(GPtr::NULL);
        }
    }
    free_list_destroy(fl.read());
    fl.write(GPtr::NULL);
    1
}

// ---------------------------------------------------------------------------------------------------------------
// Files, images, MatFX

/// `_rwFileNameIsAbsolute(const RwChar*)` (0x80E9B0): 1 for a leading `\` or a drive letter (`A`-`Z` / `a`-`z`,
/// signed byte compares) followed by `:`.
#[hook(addr = 0x80E9B0, conv = cdecl, name = "_rwFileNameIsAbsolute", expect = "8B 4C 24 04 8A 01",
       unverified = "frontier2-b2", route = "L2", hazards = "none")]
pub fn rw_file_name_is_absolute(name: GPtr<u8>) -> u32 {
    let c = rd::<i8>(name, 0);
    if c == 0x5C {
        return 1;
    }
    let letter = (0x41..=0x5A).contains(&c) || (0x61..=0x7A).contains(&c);
    u32::from(letter && rd::<u8>(name, 1) == 0x3A)
}

/// `FUN_00800810(D3DFORMAT)` (0x800810): the RGBA-to-pixel converter of a D3D format (the original's byte / jump
/// tables 0x8008C8 / 0x800888 decoded); 0 for a format without one.
#[hook(addr = 0x800810, conv = cdecl, name = "FUN_00800810", expect = "8B 4C 24 04 33 C0",
       unverified = "frontier2-b2", route = "L2", hazards = "none")]
pub fn rgba_to_pixel_converter(fmt: u32) -> u32 {
    match fmt {
        0x15 => 0x80_0B00,
        0x16 => 0x80_0AA0,
        0x17 => 0x80_0960,
        0x18 => 0x80_0930,
        0x19 => 0x80_0990,
        0x1A => 0x80_09D0,
        0x1D => 0x80_0A10,
        0x1E => 0x80_0A40,
        0x20 | 0x3F => 0x80_0B30,
        0x21 | 0x3E => 0x80_0AD0,
        0x32 => 0x80_0B70,
        0x33 => 0x80_0BD0,
        0x3C | 0x75 => 0x80_0B60,
        0x3D => 0x80_0A70,
        0x51 => 0x80_0C20,
        _ => 0,
    }
}

/// The same-depth image copy (0x803790, cdecl `(RwImage* dst, const RwImage* src)`): the palette (+0x18, `4 << depth`
/// bytes, only for depth <= 8 and both palettes present), then `height` (+8, re-read) rows of `((depth (+0xC) + 7) >>
/// 3) * width (+4)` bytes with each image's stride (+0x10) from the pixels (+0x14). The row counter lives in the
/// original's first argument slot. Returns 1.
#[hook(addr = 0x803790, conv = cdecl, name = "FUN_00803790", expect = "51 8B 44 24 08",
       unverified = "frontier2-b2", route = "L2", hazards = "stack-residue",
       note = "the row counter is kept in its own first argument slot")]
pub fn image_copy_same_depth(dst: GPtr<u8>, src: GPtr<u8>) -> u32 {
    let dp: GPtr<u8> = rd(dst, 0x18);
    let sp: GPtr<u8> = rd(src, 0x18);
    if !dp.is_null() && !sp.is_null() {
        let depth: i32 = rd(src, 0xC);
        if depth <= 8 {
            let n = (1u32 << (depth as u32 & 31)) << 2;
            for i in 0..n as isize {
                wr::<u8>(dp, i, rd(sp, i));
            }
        }
    }
    let row = ((rd::<i32>(dst, 0xC).wrapping_add(7)) >> 3).wrapping_mul(rd(dst, 4)) as u32;
    let mut s: GPtr<u8> = rd(src, 0x14);
    let mut d: GPtr<u8> = rd(dst, 0x14);
    let mut y = 0i32;
    while y < rd::<i32>(dst, 8) {
        for i in 0..row as isize {
            wr::<u8>(d, i, rd(s, i));
        }
        d = d.byte_add(rd::<i32>(dst, 0x10) as isize);
        s = s.byte_add(rd::<i32>(src, 0x10) as isize);
        y += 1;
    }
    1
}

/// The MatFX bumped-texture check (0x816290, cdecl `(base, bump)`): without a bump texture the base is returned;
/// a compressed raster on either texture (the base only when present) gives null; else the worker 0x810830.
#[hook(addr = 0x816290, conv = cdecl, name = "FUN_00816290", expect = "53 56 8B 74 24 10",
       unverified = "frontier2-b2", route = "L2", hazards = "none")]
pub fn matfx_create_bumped_texture(base: GPtr<u8>, bump: GPtr<u8>) -> GPtr<u8> {
    if bump.is_null() {
        return base;
    }
    if !base.is_null() && raster_is_compressed(rd(base, 0)) != 0 {
        return GPtr::NULL;
    }
    if raster_is_compressed(rd(bump, 0)) != 0 {
        return GPtr::NULL;
    }
    matfx_build_bumped_texture(base, bump)
}

// ---------------------------------------------------------------------------------------------------------------
// Matrices

/// `_rwMatrixIdentityError(const RwMatrix*)` (0x7F1590, ST0): the squared distance from the identity, summed on the
/// x87 stack in the original's order: `(m4² + m8² + (m0-1)²) + ((m30² + m34²) + m38²)`, then `+ ((m20² + m24²) +
/// (m28-1)²)`, then `+ ((m10² + m18²) + (m14-1)²)`.
#[hook(addr = 0x7F1590, conv = cdecl, name = "_rwMatrixIdentityError", expect = "83 EC 18 8B 44 24 1C",
       unverified = "frontier2-b2", route = "L2", hazards = "none")]
pub fn rw_matrix_identity_error(m: GPtr<u8>) -> X87 {
    let f = |o: isize| rd::<f32>(m, o);
    let sq = |o: isize| X87::from_f32(f(o)).fmul_m(f(o));
    let one = g::<f32>(0x85_8624).read();
    let a = X87::from_f32(f(0)).fsub_m(one);
    let b = X87::from_f32(f(0x14)).fsub_m(one);
    let c = X87::from_f32(f(0x28)).fsub_m(one);
    let d = X87::from_f32(f(0x38));
    let e = X87::from_f32(f(0x34));
    let s = sq(4).fadd(sq(8)).fadd(a.fmul(a));
    let t = sq(0x30).fadd(e.fmul(e)).fadd(d.fmul(d));
    let u = s.fadd(t);
    let v = sq(0x20).fadd(sq(0x24)).fadd(c.fmul(c));
    let w = u.fadd(v);
    let y = sq(0x10).fadd(sq(0x18)).fadd(b.fmul(b));
    w.fadd(y)
}

// ---------------------------------------------------------------------------------------------------------------
// Sector ties, dictionaries (batch frontier2-b4)

/// Unlinks a world-sector tie from both of its rings (object ring at +0 / +4, sector ring at +0xC / +0x10).
fn tie_unlink(tie: GPtr<u8>) {
    wr::<u32>(p8(rd(tie, 0x10)), 0, rd(tie, 0xC));
    wr::<u32>(p8(rd(tie, 0xC)), 4, rd(tie, 0x10));
    wr::<u32>(p8(rd(tie, 4)), 0, rd(tie, 0));
    wr::<u32>(p8(rd(tie, 0)), 4, rd(tie, 4));
}

/// `_rpWorldSectorTieDestroy(RpTie*)` (0x74FFA0): the tie unlinked from both rings and given back to the world
/// module's tie free list (`G + [0xC9259C]` +4) through `G->memoryFree`. Returns 1.
#[hook(addr = 0x74FFA0, conv = cdecl, name = "_rpWorldSectorTieDestroy", expect = "8B 44 24 04 50",
       unverified = "frontier2-b4", route = "L2", hazards = "heap-writes")]
pub fn rp_world_sector_tie_destroy(tie: GPtr<u8>) -> u32 {
    tie_unlink(tie);
    rw_memory_free(rd(rw_globals(), off(0xC9_259C) + 4), tie);
    1
}

/// `_rpWorldSectorLightTieDestroy(RpLightTie*)` (0x74FFF0): only a tie with both its light (+8) and sector (+0x14)
/// set is unlinked and given back to the light-tie free list (`G + [0xC9259C]` +0). Returns 1.
#[hook(addr = 0x74FFF0, conv = cdecl, name = "_rpWorldSectorLightTieDestroy", expect = "8B 44 24 04 8B 48 08",
       unverified = "frontier2-b4", route = "L2", hazards = "heap-writes")]
pub fn rp_world_sector_light_tie_destroy(tie: GPtr<u8>) -> u32 {
    if rd::<u32>(tie, 8) != 0 && rd::<u32>(tie, 0x14) != 0 {
        tie_unlink(tie);
        rw_memory_free(rd(rw_globals(), off(0xC9_259C)), tie);
    }
    1
}

use crate::bindings::dict_destroy;
orig! {
    /// `_rwSListGetBegin(sl)` (0x809530)
    fn slist_get_begin(sl: GPtr<u8>) -> GPtr<u8> = cdecl 0x809530 => unported;
    /// `_rwSListGetEnd(sl)` (0x809540)
    fn slist_get_end(sl: GPtr<u8>) -> GPtr<u8> = cdecl 0x809540 => unported;
    /// `_rwSListGetNumEntries(sl)` (0x8094B0)
    fn slist_get_num_entries(sl: GPtr<u8>) -> i32 = cdecl 0x8094B0 => unported;
    /// `_rwSListGetEntry(sl, i)` (0x809510)
    fn slist_get_entry(sl: GPtr<u8>, i: i32) -> GPtr<u8> = cdecl 0x809510 => unported;
    /// `_rwSListDestroyEndEntries(sl, n)` (0x809400)
    fn slist_destroy_end_entries(sl: GPtr<u8>, n: i32) -> GPtr<u8> = cdecl 0x809400 => unported;
    /// A dictionary schema callback with one argument: the entry add-ref (+0x18) / destructor (+0x1C) and the
    /// stream read (+0x28) / compatibility read (+0x2C) callbacks
    fn call_dict_cb(f: u32, a: u32) -> u32 = cdecl dyn => unported;
    /// `RtDictSchemaCreateDict(schema)` (0x7CED90)
    fn schema_create_dict(schema: GPtr<u8>) -> GPtr<u8> = cdecl 0x7CED90 => rt_dict_schema_create_dict;
    /// `_rwSListGetNewEntry(sl, hint)` (0x809240)
    fn slist_get_new_entry(sl: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl 0x809240 => rw_slist_get_new_entry;
}

/// `RtDictDestroy(RtDict*)` (0x7CF130): the schema (+0) forgets it as its current dictionary (+0x14); it is
/// removed from the schema's dictionary list (+0x10; the last entry moved into its slot, the list shortened by one);
/// every entry of its own list (+4) goes to the schema's entry destructor (+0x1C); its list destroyed, the dictionary
/// freed. Returns 1.
#[hook(addr = 0x7CF130, conv = cdecl, name = "RtDictDestroy", expect = "53 56 57 8B 7C 24 10",
       unverified = "frontier2-b4", route = "L2", hazards = "heap-writes")]
pub fn rt_dict_destroy(dict: GPtr<u8>) -> u32 {
    let schema: GPtr<u8> = rd(dict, 0);
    if rd::<u32>(schema, 0x14) == dict.addr() as u32 {
        wr::<u32>(schema, 0x14, 0);
    }
    let schema: GPtr<u8> = rd(dict, 0);
    let mut p = slist_get_begin(rd(schema, 0x10));
    let end = slist_get_end(rd(schema, 0x10));
    while p != end {
        if rd::<u32>(p, 0) == dict.addr() as u32 {
            let n = slist_get_num_entries(rd(schema, 0x10));
            let last = slist_get_entry(rd(schema, 0x10), n.wrapping_sub(1));
            wr::<u32>(p, 0, rd(last, 0));
            slist_destroy_end_entries(rd(schema, 0x10), 1);
            break;
        }
        p = p.byte_add(4);
    }
    let end = slist_get_end(rd(dict, 4));
    let mut p = slist_get_begin(rd(dict, 4));
    while p != end {
        let e: u32 = rd(p, 0);
        p = p.byte_add(4);
        call_dict_cb(rd(rd::<GPtr<u8>>(dict, 0), 0x1C), e);
    }
    crate::anim::slist_destroy(rd(dict, 4));
    rw_free(dict);
    1
}

/// `RtDictFindNamedEntry(RtDict*, const RwChar*)` (0x7CEFE0): the first entry of the dictionary's list (+4)
/// whose name (the schema's +0x20 callback, `(entry)`) compares equal (`G->rwstrcmp(entryName, name)`); null when
/// none does.
#[hook(addr = 0x7CEFE0, conv = cdecl, name = "RtDictFindNamedEntry", expect = "83 EC 08 53 55 56",
       unverified = "frontier2-c17", route = "L2", hazards = "none")]
pub fn rt_dict_find_named_entry(dict: GPtr<u8>, name: GPtr<u8>) -> u32 {
    let get_name: u32 = rd(rd::<GPtr<u8>>(dict, 0), 0x20);
    let end = slist_get_end(rd(dict, 4));
    let mut p = slist_get_begin(rd(dict, 4));
    while p != end {
        let e: u32 = rd(p, 0);
        if crate::image2::rw_strcmp(p8(call_dict_cb(get_name, e)), name) == 0 {
            return e;
        }
        p = p.byte_add(4);
    }
    0
}

/// The memory hint of a schema's allocations: its dictionary chunk type (+4) | `rwMEMHINTDUR_EVENT`.
fn dict_hint(schema: GPtr<u8>) -> u32 {
    rd::<u32>(schema, 4) | 0x30000
}

/// `RtDictSchemaCreateDict(RtDictSchema*)` (0x7CED90): an 8-byte dictionary (`G->rwmalloc`; failure:
/// `RwErrorSet({0x1BF, _rwerror(0x80000013, 8)})`, null) with its entry list (+4, `_rwSListCreate(4, hint)`),
/// schema +0; then added to the schema's dictionary list (+0x10). Either list step failing frees what was made
/// (the entry list destroyed first when the second fails) and returns null. The hint is the schema's +4 | 0x30000.
#[hook(addr = 0x7CED90, conv = cdecl, name = "RtDictSchemaCreateDict", expect = "83 EC 08 8B 0D 24 7B C9 00",
       unverified = "frontier2-c17", route = "L2", hazards = "heap-writes")]
pub fn rt_dict_schema_create_dict(schema: GPtr<u8>) -> GPtr<u8> {
    let dict = rw_malloc(8, dict_hint(schema));
    if dict.is_null() {
        crate::common::error2_plugin(0x1BF, 0x8000_0013, 8);
        return GPtr::NULL;
    }
    let list = crate::anim::slist_create(4, dict_hint(schema));
    wr::<u32>(dict, 4, list.addr() as u32);
    if !list.is_null() {
        wr::<u32>(dict, 0, schema.addr() as u32);
        let slot = slist_get_new_entry(rd(schema, 0x10), dict_hint(schema));
        if !slot.is_null() {
            wr::<u32>(slot, 0, dict.addr() as u32);
            return dict;
        }
        crate::anim::slist_destroy(rd(dict, 4));
    }
    rw_free(dict);
    GPtr::NULL
}

/// The entry-count read of [`rt_dict_schema_stream_read_dict`]: `RwStreamRead(stream, &count, len)`. A struct
/// chunk of at most 4 bytes is the original's single read; a longer one (the original overwrites its caller's
/// frame past the count) is read as the count's 4 bytes and the rest into a discarded buffer, in pieces. Returns
/// the bytes read (0 when the first read gives 0).
fn read_count(stream: GPtr<u8>, count: &mut u32, len: u32) -> u32 {
    if len <= 4 {
        return crate::bindings::stream_read(stream, GPtr::from_raw(count).cast(), len);
    }
    let mut got = crate::bindings::stream_read(stream, GPtr::from_raw(count).cast(), 4);
    let mut scratch = [0u8; 0x40];
    let mut left = len - 4;
    while got != 0 && left != 0 {
        let n = left.min(scratch.len() as u32);
        got += crate::bindings::stream_read(stream, GPtr::from_raw(scratch.as_mut_ptr()), n);
        left -= n;
    }
    got
}

/// `RtDictSchemaStreamReadDict(RtDictSchema*, RwStream*)` (0x7CF240): finds the struct chunk (type 1); a version
/// below the schema's compatibility version (+0xC, unsigned) is read by the schema's compatibility callback
/// (+0x2C, `(stream)`, its result returned). Else the entry count is read (`RwStreamRead` of the chunk's length;
/// the original reads it into its own argument slot: see [`read_count`] for chunks longer than 4 bytes), a
/// dictionary created (`RtDictSchemaCreateDict`, inlined in the original) and, per entry, the entry chunk (+8)
/// found and read by the schema's read callback (+0x28); the entry is appended to the dictionary's list (the new
/// slot is not checked), then add-ref'd (+0x18) and released (+0x1C). A missing chunk or a null entry destroys
/// the dictionary (`RtDictDestroy`, inlined) and returns null.
#[hook(addr = 0x7CF240, conv = cdecl, name = "RtDictSchemaStreamReadDict", expect = "83 EC 10 8D 44 24 00",
       unverified = "frontier2-c17", route = "L2", hazards = "heap-writes")]
pub fn rt_dict_schema_stream_read_dict(schema: GPtr<u8>, stream: GPtr<u8>) -> GPtr<u8> {
    let mut version = 0u32;
    let mut len = 0u32;
    if sa_core::bindings::rw_stream_find_chunk_call(stream, 1, GPtr::from_raw(&mut len), GPtr::from_raw(&mut version))
        == 0
    {
        return GPtr::NULL;
    }
    if version < rd::<u32>(schema, 0xC) {
        return p8(call_dict_cb(rd(schema, 0x2C), stream.addr() as u32));
    }
    let mut count = 0u32;
    if read_count(stream, &mut count, len) == 0 {
        return GPtr::NULL;
    }
    let dict = schema_create_dict(schema);
    if dict.is_null() {
        return GPtr::NULL;
    }
    while count != 0 {
        count -= 1;
        if sa_core::bindings::rw_stream_find_chunk_call(
            stream,
            rd(schema, 8),
            GPtr::from_raw(&mut len),
            GPtr::from_raw(&mut version),
        ) == 0
        {
            dict_destroy(dict);
            return GPtr::NULL;
        }
        let entry = call_dict_cb(rd(schema, 0x28), stream.addr() as u32);
        if entry == 0 {
            dict_destroy(dict);
            return GPtr::NULL;
        }
        let own: GPtr<u8> = rd(dict, 0);
        let slot = slist_get_new_entry(rd(dict, 4), dict_hint(own));
        wr::<u32>(slot, 0, entry);
        call_dict_cb(rd(rd::<GPtr<u8>>(dict, 0), 0x18), entry);
        call_dict_cb(rd(schema, 0x1C), entry);
    }
    dict
}

pub const HOOKS: &[HookReg] = &[
    HOOK_RP_WORLD_SECTOR_DEINSTANCE_ALL,
    HOOK_RP_WORLD_SECTOR_DESTROY_RECURSE,
    HOOK_RP_WORLD_SECTOR_GET_WORLD,
    HOOK_RW_CAMERA_FOR_ALL_SECTORS_IN_FRUSTUM,
    HOOK_RP_WORLD_ADD_CLUMP,
    HOOK_RP_D3D9_GEOMETRY_LIST_DESTROY,
    HOOK_RP_GEOMETRY_STRUCT_GET_SIZE,
    HOOK_RP_GEOMETRY_FREE,
    HOOK_RP_ATOMIC_INSTANCE,
    HOOK_RP_MATERIAL_UV_ANIM_EXISTS,
    HOOK_RP_MATERIAL_UV_ANIM_ADD_ANIM_TIME,
    HOOK_UV_ANIM_STREAM_GET_SIZE,
    HOOK_UV_ANIM_MATRIX_DECOMPOSE,
    HOOK_UV_ANIM_PARAM_KEY_FRAME_INTERPOLATE,
    HOOK_UV_ANIM_LINEAR_KEY_FRAME_INTERPOLATE,
    HOOK_UV_ANIM_PARAM_KEY_FRAME_APPLY,
    HOOK_RW_STRING_STREAM_GET_SIZE,
    HOOK_RW_STRING_STREAM_WRITE,
    HOOK_RW_SLIST_GET_NEW_ENTRY,
    HOOK_RW_FREE_LIST_FOR_ALL_USED,
    HOOK_RW_RESOURCES_CLOSE,
    HOOK_RW_FILE_NAME_IS_ABSOLUTE,
    HOOK_RGBA_TO_PIXEL_CONVERTER,
    HOOK_IMAGE_COPY_SAME_DEPTH,
    HOOK_MATFX_CREATE_BUMPED_TEXTURE,
    HOOK_RW_MATRIX_IDENTITY_ERROR,
    HOOK_RP_WORLD_SECTOR_TIE_DESTROY,
    HOOK_RP_WORLD_SECTOR_LIGHT_TIE_DESTROY,
    HOOK_RT_DICT_DESTROY,
    HOOK_RT_DICT_SCHEMA_CREATE_DICT,
    HOOK_RT_DICT_FIND_NAMED_ENTRY,
    HOOK_RT_DICT_SCHEMA_STREAM_READ_DICT,
];

/// This module's phase B natives (entries shorter than the 5-byte patch).
pub const NATIVES: &[NativeReg] = &[NATIVE_UV_ANIM_RETURN_ZERO];
