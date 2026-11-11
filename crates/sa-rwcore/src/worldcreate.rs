//! RpWorld creation as `CGame::InitialiseRenderWare` (0x5BD600) runs it: `RpWorldCreate` with the
//! bounding box (-10000..10000 on each axis), `RpWorldUnlock` and `_rpMeshOptimise` for its single
//! empty sector, `RpWorldAddCamera` / `RpWorldRemoveCamera`, and the light world-sync callback the
//! world-object light extension installs (`RpWorldAddLight` of `Light::LightsCreate` runs it through
//! the frame). PORT-RW-1 follow-up, written for the native boot; ported from the 1.0 US bytes.
//!
//! `RpWorld` (0x70 + plugins, registry 0x8D62D0): +0 RwObject (type 7), +8 flags, +0xC render order,
//! +0x10 RpMaterialList (+0 array, +4 count, +8 space), +0x1C root sector, +0x20 numTexCoordSets,
//! +0x24 numClumpsIn..., +0x28 current clump link, +0x2C clump list, +0x34 / +0x3C light lists (local /
//! directional), +0x44..+0x4C, +0x50 bbox sup, +0x5C bbox inf, +0x68 sector render callback,
//! +0x6C. `RpWorldSector` (0x88 + plugins, registry 0x8D736C): +0 type (-1 = leaf), +4 triangles
//! (8 bytes: u16 v0, v1, v2, matIndex), +8..+0x30, +0x34, +0x38 / +0x40 atomic / light tie lists,
//! +0x48 bbox sup, +0x54 bbox inf, +0x60 / +0x6C tight bbox, +0x78 mesh header, +0x7C, u16 +0x80
//! material-list window base, u16 +0x82 numVertices, u16 +0x84 numTriangles. Plane node: +0 type
//! (byte offset of the axis: 0, 4, 8), +8 left, +0xC right, +0x10 left value, +0x14 right value.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

use crate::common::{
    RwError, f32_of, g, p8, rd, rw_error1, rw_error2, rw_error_set, rw_free, rw_globals, rw_malloc,
    rw_memory_alloc, rw_memory_free, wr, x,
};

/// WORLDMODULE / WORLDOBJMODULE offset globals.
const WORLD_MODULE: usize = 0xC9254C;
const WORLDOBJ_MODULE: usize = 0xC9259C;
/// Camera / light world-object plugin offsets.
const CAMERA_WORLD_OFFSET: usize = 0xC925A4;
const LIGHT_WORLD_OFFSET: usize = 0xC925B0;
/// World / sector plugin registries (`sizeOfStruct` at +0).
const WORLD_REGISTRY: u32 = 0x8D62D0;
const SECTOR_REGISTRY: u32 = 0x8D736C;
/// `_rpSectorDefaultRenderCallBack`.
const DEFAULT_SECTOR_RENDER_CB: u32 = 0x74EEC0;
/// The static empty `RpMeshHeader` `_rpMeshOptimise` returns for a build mesh without triangles.
const EMPTY_MESH_HEADER: u32 = 0x8D6378;
/// Tri-strip generator and its data (`RpMeshSetTriStripMethod` state).
const TRISTRIP_METHOD: usize = 0x8D6388;
const TRISTRIP_DATA: usize = 0x8D638C;
/// `_rpTriListMeshGenerate`.
const TRILIST_GENERATE: u32 = 0x75D530;

orig! {
    /// `_rpMaterialListInitialize(list)` (0x74E1B0)
    fn material_list_initialize(l: GPtr<u8>) -> GPtr<u8> = cdecl 0x74E1B0 => crate::material::material_list_initialize;
    /// `_rpMaterialListDeinitialize(list)` (0x74E150)
    fn material_list_deinitialize(l: GPtr<u8>) -> GPtr<u8> = cdecl 0x74E150 => crate::material::material_list_deinitialize;
    /// `_rwPluginRegistryInitObject(reg, obj)` (0x8086E0)
    fn plugin_init_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x8086E0 => crate::plugin::registry_init_object;
    /// `_rwPluginRegistryDeInitObject(reg, obj)` (0x808740)
    fn plugin_deinit_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808740 => crate::plugin::registry_deinit_object;
    /// `RpWorldUnlock(world)` (0x74F210)
    fn world_unlock_call(w: GPtr<u8>) -> GPtr<u8> = cdecl 0x74F210 => rp_world_unlock;
    /// `_rpMeshDestroy(header)` (0x758BC0)
    fn mesh_destroy(h: GPtr<u8>) -> u32 = cdecl 0x758BC0 => crate::mesh::rp_mesh_destroy;
    /// `_rpWorldSectorDeinstanceAll(sector)` (0x74ECA0)
    fn sector_deinstance_all(s: GPtr<u8>) = cdecl 0x74ECA0 => unported;
    /// `_rpWorldSectorDestroyRecurse(sector)` (0x74ED50)
    fn sector_destroy_recurse(s: GPtr<u8>) = cdecl 0x74ED50 => unported;
    /// `_rpBuildMeshCreate(bufferSize)` (0x758A90)
    fn build_mesh_create(n: u32) -> GPtr<u8> = cdecl 0x758A90 => crate::mesh::rp_build_mesh_create;
    /// `_rpBuildMeshDestroy(buildMesh)` (0x758B80)
    fn build_mesh_destroy(bm: GPtr<u8>) -> u32 = cdecl 0x758B80 => crate::mesh::rp_build_mesh_destroy;
    /// `_rpBuildMeshAddTriangle(bm, material, v0, v1, v2, matIndex, texIndex, rasIndex, pipeIndex)` (0x758C00)
    fn build_mesh_add_triangle(bm: GPtr<u8>, mat: GPtr<u8>, v0: u32, v1: u32, v2: u32, mi: u32, ti: u32, ri: u32, pi: u32) -> GPtr<u8> = cdecl 0x758C00 => crate::mesh::rp_build_mesh_add_triangle;
    /// `_rpMeshOptimise(buildMesh, flags)` (0x75D970)
    fn mesh_optimise_call(bm: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x75D970 => rp_mesh_optimise;
    /// A mesh generator `gen(buildMesh, data)` (`_rpTriListMeshGenerate` or the tri-strip method)
    fn call_mesh_generator(f: u32, bm: GPtr<u8>, data: u32) -> GPtr<u8> = cdecl dyn => unported;
    /// `RwFrameUpdateObjects(frame)` (0x7F0910)
    fn frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => crate::frame::update_objects;
    /// `RwFrameGetLTM(frame)` (0x7F0990)
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => crate::frame::get_ltm;
    /// A light's saved frame-sync callback `cb(light)`
    fn call_light_sync(f: u32, light: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
}

fn a32(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// `G + [offset global]`, recomputed on every access as the originals reload both globals.
fn module(offset_global: usize) -> GPtr<u8> {
    let off = g::<u32>(offset_global).read();
    rw_globals().byte_add(off as isize)
}

/// `RwErrorSet({2, _rwerror(code, a)})` (RpWorld's plugin id).
fn world_error2(code: u32, a: u32) {
    let mut e = RwError {
        plugin_id: 2,
        error_code: 0,
    };
    e.error_code = rw_error2(code, a) as i32;
    rw_error_set(GPtr::from_raw(&mut e));
}

/// `RwErrorSet({2, _rwerror(code)})`.
fn world_error1(code: u32) {
    let mut e = RwError {
        plugin_id: 2,
        error_code: 0,
    };
    e.error_code = rw_error1(code) as i32;
    rw_error_set(GPtr::from_raw(&mut e));
}

/// Copies the three words `src+so..` to `dst+do_`.
fn copy3(dst: GPtr<u8>, d: isize, src: GPtr<u8>, s: isize) {
    for k in 0..3isize {
        wr::<u32>(dst, d + k * 4, rd::<u32>(src, s + k * 4));
    }
}

/// `RpWorldCreate(boundingBox)` (0x74F760): the world (`rwmalloc(registry size, 0x3000B)`; failure
/// reports `{2, E_RW_NOMEM(size)}`) with an empty material list, flags 0, render order 2; one leaf
/// sector (`rwmalloc(sector registry size, 0x3000B)`; failure reports `{2, E_RW_NOMEM(4)}` (sic) and
/// frees the world) with empty tie lists, no geometry and both its boxes = the box; the world's box =
/// the box, empty clump and light lists, the default sector render callback. The world is linked into
/// the WORLDMODULE list (a 0x10-byte link {world, the world registry's size, RwLLLink}; an allocation
/// failure only skips the link), both plugin registries' constructors run, then `RpWorldUnlock`.
/// When that fails: the link is removed and freed, every leaf's mesh destroyed (sector-stack walk),
/// the material list deinitialised, the sectors deinstanced (flag bit 0 of `world+3`) or destroyed,
/// the world's plugin destructors run, the world freed, NULL returned.
#[hook(addr = 0x74F760, conv = cdecl, name = "RpWorldCreate", expect = "A1 D0 62 8D 00")]
pub fn rp_world_create(bbox: GPtr<u8>) -> GPtr<u8> {
    let w = rw_malloc(rd::<u32>(p8(WORLD_REGISTRY), 0), 0x3000B);
    if w.is_null() {
        world_error2(0x8000_0013, rd::<u32>(p8(WORLD_REGISTRY), 0));
        return GPtr::NULL;
    }
    wr::<u8>(w, 0, 7);
    wr::<u8>(w, 1, 0);
    wr::<u8>(w, 2, 0);
    wr::<u8>(w, 3, 0);
    wr::<u32>(w, 4, 0);
    material_list_initialize(w.byte_add(0x10));
    wr::<u32>(w, 8, 0);
    wr::<u32>(w, 0xC, 2);
    let s = rw_malloc(rd::<u32>(p8(SECTOR_REGISTRY), 0), 0x3000B);
    if s.is_null() {
        world_error2(0x8000_0013, 4);
        rw_free(w);
        return GPtr::NULL;
    }
    let l = s.byte_add(0x38);
    wr::<u32>(s, 0x3C, a32(l));
    wr::<u32>(l, 0, a32(l));
    let l = s.byte_add(0x40);
    wr::<u32>(s, 0x44, a32(l));
    wr::<u32>(s, 0, 0xFFFF_FFFF);
    wr::<u32>(l, 0, a32(l));
    wr::<u32>(s, 0x34, 0);
    wr::<u32>(s, 0x78, 0);
    wr::<u16>(s, 0x82, 0);
    wr::<u16>(s, 0x84, 0);
    wr::<u32>(s, 8, 0);
    wr::<u32>(s, 4, 0);
    wr::<u32>(s, 0xC, 0);
    for k in 0..8isize {
        wr::<u32>(s, 0x10 + k * 4, 0);
    }
    wr::<u32>(s, 0x30, 0);
    copy3(s, 0x54, bbox, 0xC);
    copy3(s, 0x48, bbox, 0);
    copy3(s, 0x6C, bbox, 0xC);
    wr::<u32>(s, 0x60, rd::<u32>(bbox, 0));
    wr::<u32>(s, 0x64, rd::<u32>(bbox, 4));
    wr::<u32>(s, 0x7C, 0);
    wr::<u32>(s, 0x68, rd::<u32>(bbox, 8));
    wr::<u32>(w, 0x1C, a32(s));
    wr::<u32>(w, 0x20, 0);
    wr::<u32>(w, 0x4C, 0);
    wr::<u32>(w, 0x48, 0);
    wr::<u32>(w, 0x44, 0);
    copy3(w, 0x5C, bbox, 0xC);
    wr::<u32>(w, 0x50, rd::<u32>(bbox, 0));
    wr::<u32>(w, 0x54, rd::<u32>(bbox, 4));
    wr::<u32>(w, 0x24, 0);
    wr::<u32>(w, 0x58, rd::<u32>(bbox, 8));
    let l = w.byte_add(0x2C);
    wr::<u32>(w, 0x30, a32(l));
    wr::<u32>(w, 0x28, a32(l));
    wr::<u32>(l, 0, a32(l));
    let l = w.byte_add(0x34);
    wr::<u32>(w, 0x38, a32(l));
    wr::<u32>(l, 0, a32(l));
    let l = w.byte_add(0x3C);
    wr::<u32>(l, 0, a32(l));
    wr::<u32>(w, 0x40, a32(l));
    wr::<u32>(w, 0x68, DEFAULT_SECTOR_RENDER_CB);
    wr::<u32>(w, 0x6C, 0);
    let wsize = rd::<u32>(p8(WORLD_REGISTRY), 0);
    let link = rw_memory_alloc(p8(rd::<u32>(module(WORLD_MODULE), 0)), 0x40507);
    if !link.is_null() {
        wr::<u32>(link, 0, a32(w));
        wr::<u32>(link, 4, wsize);
        let ll = link.byte_add(8);
        wr::<u32>(ll, 0, rd::<u32>(module(WORLD_MODULE), 4));
        wr::<u32>(link, 0xC, a32(module(WORLD_MODULE).byte_add(4)));
        wr::<u32>(p8(rd::<u32>(module(WORLD_MODULE), 4)), 4, a32(ll));
        wr::<u32>(module(WORLD_MODULE), 4, a32(ll));
    }
    plugin_init_object(p8(WORLD_REGISTRY), w);
    plugin_init_object(p8(SECTOR_REGISTRY), s);
    if !world_unlock_call(w).is_null() {
        return w;
    }
    // Failure: unlink and free the world's link.
    let head = a32(module(WORLD_MODULE).byte_add(4));
    let mut e = rd::<u32>(module(WORLD_MODULE), 4);
    while e != head {
        let lk = p8(e.wrapping_sub(8));
        if rd::<u32>(lk, 0) == a32(w) {
            let (n, p) = (rd::<u32>(lk, 8), rd::<u32>(lk, 0xC));
            wr::<u32>(p8(p), 0, n);
            let (n, p) = (rd::<u32>(lk, 8), rd::<u32>(lk, 0xC));
            wr::<u32>(p8(n), 4, p);
            rw_memory_free(p8(rd::<u32>(module(WORLD_MODULE), 0)), lk);
            break;
        }
        e = rd::<u32>(p8(e), 0);
    }
    // Destroy every leaf's mesh (sector-stack walk).
    let mut stack = [0u32; 64];
    let mut sp: i32 = 0;
    let mut sec = rd::<u32>(w, 0x1C);
    if sec != 0 {
        loop {
            let sc = p8(sec);
            if rd::<i32>(sc, 0) < 0 {
                let m = rd::<u32>(sc, 0x78);
                if m != 0 {
                    mesh_destroy(p8(m));
                    wr::<u32>(sc, 0x78, 0);
                }
                sec = stack[sp as usize];
                sp -= 1;
            } else {
                sp += 1;
                stack[sp as usize] = rd::<u32>(sc, 0xC);
                sec = rd::<u32>(sc, 8);
            }
            if sp < 0 {
                break;
            }
        }
    }
    material_list_deinitialize(w.byte_add(0x10));
    let root = rd::<u32>(w, 0x1C);
    if rd::<u8>(w, 3) & 1 != 0 {
        if root != 0 {
            sector_deinstance_all(p8(root));
        }
    } else if root != 0 {
        sector_destroy_recurse(p8(root));
    }
    plugin_deinit_object(p8(WORLD_REGISTRY), w);
    rw_free(w);
    GPtr::NULL
}

/// Linear search of `arr[0..count)` (u16 index and count, as the bytes compare 16-bit halves); when
/// not found the value is appended and the count (a full dword) incremented. Returns the index.
fn find_or_add(arr: GPtr<u8>, count: &mut u32, v: u32) -> u32 {
    let mut i: u32 = 0;
    if (*count as u16) > 0 {
        loop {
            if rd::<u32>(arr, ((i & 0xFFFF) * 4) as isize) == v {
                break;
            }
            i = i.wrapping_add(1);
            if (i as u16) >= (*count as u16) {
                break;
            }
        }
    }
    if (i as u16) == (*count as u16) {
        wr::<u32>(arr, ((i & 0xFFFF) * 4) as isize, v);
        *count = count.wrapping_add(1);
    }
    i
}

/// `RpWorldUnlock(world)` (0x74F210): NULL root returns NULL. Walks the sector tree (explicit stack of
/// right children, 60 entries in the original's frame); for every leaf without a mesh: a build mesh
/// for its triangles (failure returns NULL); when the world has materials, three scratch arrays of
/// `numMaterials` words (hints 0x10006 / 0x10507 / 0x10507, not NULL-checked) collect the distinct
/// textures, rasters and pipelines (u16 linear searches) and every triangle is added with its
/// material (`window[matIndex]`, window = materials + u16 sector+0x80) and the three indices; then
/// `_rpMeshOptimise(buildMesh, world flags & 1)` becomes the sector's mesh (failure destroys the
/// build mesh and returns NULL). Returns the world.
#[hook(addr = 0x74F210, conv = cdecl, name = "RpWorldUnlock", expect = "81 EC 30 01 00 00")]
pub fn rp_world_unlock(w: GPtr<u8>) -> GPtr<u8> {
    let mut sec = rd::<u32>(w, 0x1C);
    if sec == 0 {
        return GPtr::NULL;
    }
    let mut stack = [0u32; 64];
    let mut sp: i32 = 0;
    loop {
        let s = p8(sec);
        if rd::<i32>(s, 0) < 0 {
            let window = p8(rd::<u32>(w, 0x10).wrapping_add(u32::from(rd::<u16>(s, 0x80)) * 4));
            if rd::<u32>(s, 0x78) == 0 {
                let bm = build_mesh_create(u32::from(rd::<u16>(s, 0x84)));
                if bm.is_null() {
                    return GPtr::NULL;
                }
                let nmat = rd::<u32>(w, 0x14);
                if nmat != 0 {
                    let size = nmat.wrapping_mul(4);
                    let tex_arr = rw_malloc(size, 0x10006);
                    let ras_arr = rw_malloc(size, 0x10507);
                    let pipe_arr = rw_malloc(size, 0x10507);
                    let (mut ntex, mut nras, mut npipe) = (0u32, 0u32, 0u32);
                    let mut i: i32 = 0;
                    if rd::<u16>(s, 0x84) > 0 {
                        loop {
                            let tri = p8(rd::<u32>(s, 4).wrapping_add((i as u32).wrapping_mul(8)));
                            let mi = rd::<u16>(tri, 6);
                            let mat = p8(rd::<u32>(window, (mi as isize) * 4));
                            let tex = rd::<u32>(mat, 0);
                            let ti = find_or_add(tex_arr, &mut ntex, tex);
                            let ras = if tex != 0 { rd::<u32>(p8(tex), 0) } else { 0 };
                            let ri = find_or_add(ras_arr, &mut nras, ras);
                            let pi = find_or_add(pipe_arr, &mut npipe, rd::<u32>(mat, 8));
                            build_mesh_add_triangle(
                                bm,
                                mat,
                                u32::from(rd::<u16>(tri, 0)),
                                u32::from(rd::<u16>(tri, 2)),
                                u32::from(rd::<u16>(tri, 4)),
                                u32::from(rd::<u16>(tri, 6)),
                                ti,
                                ri,
                                pi,
                            );
                            i += 1;
                            if i >= i32::from(rd::<u16>(s, 0x84)) {
                                break;
                            }
                        }
                    }
                    rw_free(tex_arr);
                    rw_free(ras_arr);
                    rw_free(pipe_arr);
                }
                let flags = if rd::<u8>(w, 8) & 1 != 0 { 1 } else { 0 };
                let m = mesh_optimise_call(bm, flags);
                if m.is_null() {
                    build_mesh_destroy(bm);
                    return GPtr::NULL;
                }
                wr::<u32>(s, 0x78, a32(m));
            }
            sec = stack[sp as usize];
            sp -= 1;
        } else {
            sp += 1;
            stack[sp as usize] = rd::<u32>(s, 0xC);
            sec = rd::<u32>(s, 8);
        }
        if sp < 0 {
            break;
        }
    }
    w
}

/// `_rpMeshOptimise(buildMesh, flags)` (0x75D970): NULL returns NULL; a build mesh without triangles
/// is destroyed and the static empty mesh header 0x8D6378 returned; otherwise the generator (flag bit
/// 0: the tri-strip method and its data from 0x8D6388 / 0x8D638C, else `_rpTriListMeshGenerate` with
/// data 0) builds the mesh, and on success the build mesh is destroyed. On failure the build mesh is
/// kept (the caller destroys it) and NULL returned.
#[hook(addr = 0x75D970, conv = cdecl, name = "_rpMeshOptimise", expect = "56 57 8B 7C 24 0C")]
pub fn rp_mesh_optimise(bm: GPtr<u8>, flags: u32) -> GPtr<u8> {
    if bm.is_null() {
        return GPtr::NULL;
    }
    if rd::<u32>(bm, 4) == 0 {
        build_mesh_destroy(bm);
        return p8(EMPTY_MESH_HEADER);
    }
    let (f, data) = if flags & 1 != 0 {
        (g::<u32>(TRISTRIP_METHOD).read(), g::<u32>(TRISTRIP_DATA).read())
    } else {
        (TRILIST_GENERATE, 0)
    };
    let m = call_mesh_generator(f, bm, data);
    if m.is_null() {
        return GPtr::NULL;
    }
    build_mesh_destroy(bm);
    m
}

/// `RpWorldAddCamera(world, camera)` (0x750F20): updates the camera's frame objects when it has a
/// frame, then stores the world in the camera's world extension (+0xC). Returns `world`.
#[hook(addr = 0x750F20, conv = cdecl, name = "RpWorldAddCamera", expect = "8B 44 24 08 8B 0D A4 25 C9 00")]
pub fn rp_world_add_camera(world: GPtr<u8>, camera: GPtr<u8>) -> GPtr<u8> {
    let c = camera.byte_add(g::<i32>(CAMERA_WORLD_OFFSET).read() as isize);
    let f = rd::<u32>(camera, 4);
    if f != 0 {
        frame_update_objects(p8(f));
    }
    wr::<u32>(c, 0xC, a32(world));
    world
}

/// `RpWorldRemoveCamera(world, camera)` (0x750F50): when the camera is in a world, clears its world
/// and its visible-sector count (+8) and returns `world` (not checked against the camera's world);
/// else NULL.
#[hook(addr = 0x750F50, conv = cdecl, name = "RpWorldRemoveCamera", expect = "8B 44 24 08 8B 0D A4 25 C9 00")]
pub fn rp_world_remove_camera(world: GPtr<u8>, camera: GPtr<u8>) -> GPtr<u8> {
    let c = camera.byte_add(g::<i32>(CAMERA_WORLD_OFFSET).read() as isize);
    if rd::<u32>(c, 0xC) == 0 {
        return GPtr::NULL;
    }
    wr::<u32>(c, 0xC, 0);
    wr::<u32>(c, 8, 0);
    world
}

/// `_rpWorldObjLightSync(light)` (0x750AC0), the frame-sync callback the world-object light
/// constructor installs: calls the saved sync callback (FALSE reports `{2, E_RW_NULLP}` and returns
/// the light). Directional/ambient lights (subtype < 0x80) and lights without world or frame stop
/// there. Otherwise every tie of the light is unlinked and freed, the light's box `pos ± radius`
/// (x87 single adds stored as f32: sup x, y, z, then inf x, y, z) is pushed down the world's sector
/// tree (plane node: inf < left value goes left, and also right when sup > right value; else right
/// when sup > right value; unordered compares take the "not less" / "not greater" side as the bytes
/// test them), and each reached leaf gets a new tie (`O+4` freelist, hint 0x30507, not NULL-checked)
/// linked at the head of the sector's light list (+0x40) and of the light's tie list (+0x2C).
/// Returns the light.
#[hook(addr = 0x750AC0, conv = cdecl, name = "_rpWorldObjLightSync", expect = "A1 B0 25 C9 00")]
pub fn rp_world_obj_light_sync(light: GPtr<u8>) -> GPtr<u8> {
    let e = light.byte_add(g::<i32>(LIGHT_WORLD_OFFSET).read() as isize);
    if call_light_sync(rd::<u32>(e, 4), light).is_null() {
        world_error1(0x8000_0016);
        return light;
    }
    if rd::<u8>(light, 1) < 0x80 {
        return light;
    }
    let world = rd::<u32>(e, 0);
    let frame = rd::<u32>(light, 4);
    if world == 0 || frame == 0 {
        return light;
    }
    let radius = rd::<f32>(light, 0x14);
    crate::worldplugin::free_ties(light, 0x2C, 4, false);
    let pos = frame_get_ltm(p8(frame)).byte_add(0x30);
    let (px, py, pz) = (rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8));
    // sup at [0..3), inf at [3..6) (the original's locals +0x14.. and +0x20..)
    let b = [
        f32_of(x(px).fadd_m(radius)),
        f32_of(x(py).fadd_m(radius)),
        f32_of(x(pz).fadd_m(radius)),
        f32_of(x(px).fsub_m(radius)),
        f32_of(x(py).fsub_m(radius)),
        f32_of(x(pz).fsub_m(radius)),
    ];
    let sup = |ty: u32| b[(ty / 4) as usize];
    let inf = |ty: u32| b[3 + (ty / 4) as usize];
    let ties_head = light.byte_add(0x2C);
    let mut stack = [0u32; 64];
    let mut sp: i32 = 0;
    let mut sec = p8(rd::<u32>(p8(world), 0x1C));
    loop {
        let ty = rd::<i32>(sec, 0);
        if ty < 0 {
            let tie = rw_memory_alloc(p8(rd::<u32>(module(WORLDOBJ_MODULE), 4)), 0x30507);
            wr::<u32>(tie, 0x14, a32(sec));
            wr::<u32>(tie, 8, a32(light));
            let head = sec.byte_add(0x40);
            wr::<u32>(tie, 4, a32(head));
            wr::<u32>(tie, 0, rd::<u32>(head, 0));
            wr::<u32>(p8(rd::<u32>(head, 0)), 4, a32(tie));
            wr::<u32>(head, 0, a32(tie));
            let tl = tie.byte_add(0xC);
            wr::<u32>(tie, 0x10, a32(ties_head));
            wr::<u32>(tl, 0, rd::<u32>(ties_head, 0));
            wr::<u32>(p8(rd::<u32>(ties_head, 0)), 4, a32(tl));
            wr::<u32>(ties_head, 0, a32(tl));
            sec = p8(stack[sp as usize]);
            sp -= 1;
        } else {
            let t = ty as u32;
            let lv = rd::<f32>(sec, 0x10);
            let rv = rd::<f32>(sec, 0x14);
            // fcomp inf, left; test ah, 5; jp -> not (inf < left)
            if X87::from_f32(inf(t)).fcom(X87::from_f32(lv)) == Cmp::Less {
                let node = sec;
                sec = p8(rd::<u32>(node, 8));
                // fcomp sup, right; and 0x4100; jne -> sup <= right or unordered
                if X87::from_f32(sup(t)).fcom(X87::from_f32(rv)) == Cmp::Greater {
                    sp += 1;
                    stack[sp as usize] = rd::<u32>(node, 0xC);
                }
            } else if X87::from_f32(sup(t)).fcom(X87::from_f32(rv)) == Cmp::Greater {
                sec = p8(rd::<u32>(sec, 0xC));
            } else {
                sec = p8(stack[sp as usize]);
                sp -= 1;
            }
        }
        if sp < 0 {
            break;
        }
    }
    light
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RP_WORLD_CREATE,
    HOOK_RP_WORLD_UNLOCK,
    HOOK_RP_MESH_OPTIMISE,
    HOOK_RP_WORLD_ADD_CAMERA,
    HOOK_RP_WORLD_REMOVE_CAMERA,
    HOOK_RP_WORLD_OBJ_LIGHT_SYNC,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
