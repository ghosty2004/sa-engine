//! GEOMETRYMODULE (RpWorld): `RpGeometry` and `RpMorphTarget` (PORT-RW-1). Ported from the 1.0 US
//! bytes; `docs/re/specs/RW/RpGeometry.md` is the reading aid.
//!
//! `RpGeometry` (0x60 + plugins, registry 0x8D628C; one malloc'd block, not a freelist object): +0
//! RwObject (type 8), +8 flags, +0xC u16 lockedSinceLastInst, +0xE i16 refCount, +0x10 numTriangles,
//! +0x14 numVertices, +0x18 numMorphTargets, +0x1C numTexCoordSets, +0x20 RpMaterialList, +0x2C
//! triangles (8 bytes: u16 v[3], u16 matIndex), +0x30 preLitLum, +0x34 texCoords[8], +0x54 mesh, +0x58
//! repEntry, +0x5C morph targets (0x1C each: +0 parent, +4 RwSphere, +0x14 verts, +0x18 normals).
//! RpWorld errors are `RwErrorSet({2, _rwerror(..)})`.
//!
//! Argument slots the originals reuse as locals (not reproduced): `RpGeometryCreate` keeps the low flag
//! byte in its `format` slot, `RpGeometryForAllMaterials` the material count in its `geometry` slot.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{
    RwError, f32_of, g, p8, rd, rw_error_set, rw_error1, rw_error2, rw_free, rw_malloc, rw_realloc, wr, x,
};

/// GEOMETRYMODULE offset global.
pub const GEOMETRY_MODULE: usize = 0xC924E0;
/// Module open count.
const GEOMETRY_INSTANCES: usize = 0xC924E4;
/// Plugin registry (`sizeOfStruct` at +0).
pub const GEOMETRY_REGISTRY: u32 = 0x8D628C;
/// 0.5f (0x858B8C), 0.0f (0x858B50), 1.001f (0x872490).
const HALF: f32 = 0.5;
const ZERO: f32 = 0.0;
const RADIUS_SCALE: f32 = f32::from_bits(0x3F80_20C5);

const FLAG_TRISTRIP: u32 = 0x1;
const FLAG_NORMALS: u32 = 0x10;
const FLAG_NATIVE: u32 = 0x0100_0000;

const E_RW_NOMEM: u32 = 0x8000_0013;
const E_RW_BADVERSION: u32 = 0x8000_0004;
/// Core error 6 (too many vertices).
const E_TOO_MANY_VERTICES: u32 = 6;

orig! {
    /// `_rwPluginRegistryInitObject(reg, obj)` (0x8086E0)
    fn plugin_init_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x8086E0 => crate::plugin::registry_init_object;
    /// `_rwPluginRegistryDeInitObject(reg, obj)` (0x808740)
    fn plugin_deinit_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808740 => crate::plugin::registry_deinit_object;
    /// `_rwPluginRegistryAddPlugin(reg, size, id, ctor, dtor, copy)` (0x8084A0)
    fn plugin_add(reg: GPtr<u8>, size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x8084A0 => crate::plugin::registry_add_plugin;
    /// `_rwPluginRegistryAddPluginStream(reg, id, read, write, getSize)` (0x8088E0)
    fn plugin_add_stream(reg: GPtr<u8>, id: u32, read: u32, write: u32, get_size: u32) -> i32 = cdecl 0x8088E0 => crate::plugin::registry_add_plugin_stream;
    /// `RwBBoxCalculate(box, verts, n)` (0x808F60)
    fn rw_bbox_calculate(bx: GPtr<u8>, verts: GPtr<u8>, n: i32) -> GPtr<u8> = cdecl 0x808F60 => crate::matrix::rw_bbox_calculate;
    /// `_rwSqrt(x)` (0x7EDB30), result in ST0
    fn rw_sqrt(v: f32) -> X87 = cdecl 0x7EDB30 => crate::vector::rw_sqrt;
    /// `RwResourcesFreeResEntry(entry)` (0x807DE0)
    fn rw_resources_free_res_entry(e: GPtr<u8>) -> u32 = cdecl 0x807DE0 => unported;
    /// `_rpMeshDestroy(header)` (0x758BC0)
    fn rp_mesh_destroy(h: GPtr<u8>) -> u32 = cdecl 0x758BC0 => crate::mesh::rp_mesh_destroy;
    /// `_rpBuildMeshCreate(bufferSize)` (0x758A90)
    fn rp_build_mesh_create(n: u32) -> GPtr<u8> = cdecl 0x758A90 => crate::mesh::rp_build_mesh_create;
    /// `_rpBuildMeshDestroy(bm)` (0x758B80)
    fn rp_build_mesh_destroy(bm: GPtr<u8>) -> u32 = cdecl 0x758B80 => crate::mesh::rp_build_mesh_destroy;
    /// `_rpBuildMeshAddTriangle(bm, mat, v0, v1, v2, matIndex, texIndex, rasIndex, pipeIndex)` (0x758C00)
    fn rp_build_mesh_add_triangle(bm: GPtr<u8>, mat: GPtr<u8>, v0: u32, v1: u32, v2: u32, mi: u32, ti: u32, ri: u32, pi: u32) -> GPtr<u8> = cdecl 0x758C00 => crate::mesh::rp_build_mesh_add_triangle;
    /// `_rpMeshOptimise(bm, flags)` (0x75D970)
    fn rp_mesh_optimise(bm: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x75D970 => unported;

    /// `_rpMaterialListInitialize(list)` (0x74E1B0)
    fn material_list_initialize_call(l: GPtr<u8>) -> GPtr<u8> = cdecl 0x74E1B0 => crate::material::material_list_initialize;
    /// `_rpMaterialListDeinitialize(list)` (0x74E150)
    fn material_list_deinitialize_call(l: GPtr<u8>) -> GPtr<u8> = cdecl 0x74E150 => crate::material::material_list_deinitialize;
    /// `_rpMaterialListGetMaterial(list, index)` (0x74E2B0)
    fn material_list_get_material_call(l: GPtr<u8>, i: i32) -> GPtr<u8> = cdecl 0x74E2B0 => crate::material::material_list_get_material;
    /// `_rpMaterialListFindMaterialIndex(list, material)` (0x74E420)
    fn material_list_find_call(l: GPtr<u8>, m: GPtr<u8>) -> i32 = cdecl 0x74E420 => crate::material::material_list_find_material_index;
    /// `_rpMaterialListAppendMaterial(list, material)` (0x74E350)
    fn material_list_append_call(l: GPtr<u8>, m: GPtr<u8>) -> i32 = cdecl 0x74E350 => crate::material::material_list_append_material;
    /// `RpGeometryAddMorphTargets(geometry, n)` (0x74C310)
    fn add_morph_targets_call(gm: GPtr<u8>, n: i32) -> i32 = cdecl 0x74C310 => rp_geometry_add_morph_targets;
    /// A material callback `cb(material, data)` (`RpGeometryForAllMaterials`)
    fn call_material_cb(f: u32, m: GPtr<u8>, data: u32) -> GPtr<u8> = cdecl dyn => unported;
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

/// `cb_GEOMETRYMODULE_RwEngine_Open(instance, offset, size)` (0x74BFA0).
#[hook(addr = 0x74BFA0, conv = cdecl, name = "cb_GEOMETRYMODULE_RwEngine_Open", expect = "8B 44 24 08 A3 E0 24 C9 00")]
pub fn geometry_module_open(instance: GPtr<u8>, offset: u32, _size: u32) -> GPtr<u8> {
    g::<u32>(GEOMETRY_MODULE).write(offset);
    let c = g::<u32>(GEOMETRY_INSTANCES);
    c.write(c.read().wrapping_add(1));
    instance
}

/// `cb_GEOMETRYMODULE_RwEngine_Close(instance, offset, size)` (0x74BFC0).
#[hook(addr = 0x74BFC0, conv = cdecl, name = "cb_GEOMETRYMODULE_RwEngine_Close", expect = "A1 E4 24 C9 00")]
pub fn geometry_module_close(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    let c = g::<u32>(GEOMETRY_INSTANCES);
    c.write(c.read().wrapping_sub(1));
    instance
}

/// `RpMorphTargetCalcBoundingSphere(mt, sphere)` (0x74C200): centre = half the bbox sum (x87, each
/// coordinate stored f32; z is stored before scaling), radius = `sqrt(max squared distance) * 1.001f`
/// (the sqrt only when the maximum is > 0; no vertices -> radius 0). Returns mt.
#[hook(addr = 0x74C200, conv = cdecl, name = "RpMorphTargetCalcBoundingSphere", expect = "83 EC 2C 56 57")]
pub fn rp_morph_target_calc_bounding_sphere(mt: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> {
    let mut max_d2: f32 = f32::from_bits(0);
    let n = rd::<u32>(p8(rd::<u32>(mt, 0)), 0x14);
    // RwBBox {sup, inf}
    let mut bb = [0f32; 6];
    rw_bbox_calculate(
        GPtr::from_raw(bb.as_mut_ptr()).cast(),
        p8(rd::<u32>(mt, 0x14)),
        n as i32,
    );
    let sx = x(bb[0]).fadd_m(bb[3]);
    let sy = x(bb[1]).fadd_m(bb[4]);
    let sz = x(bb[2]).fadd_m(bb[5]);
    let verts = p8(rd::<u32>(mt, 0x14));
    let cz0 = f32_of(sz);
    let cx = f32_of(sx.fmul_m(HALF));
    let cy = f32_of(sy.fmul_m(HALF));
    let cz = f32_of(x(cz0).fmul_m(HALF));
    if n != 0 {
        let mut v = verts;
        let mut k = n;
        loop {
            let dx = x(rd::<f32>(v, 0)).fsub_m(cx);
            let dy = x(rd::<f32>(v, 4)).fsub_m(cy);
            let dz = x(rd::<f32>(v, 8)).fsub_m(cz);
            let d2 = dy.fmul(dy).fadd(dx.fmul(dx)).fadd(dz.fmul(dz));
            if !d2.fcom(x(max_d2)).test_any(0x41) {
                max_d2 = f32_of(d2);
            }
            v = v.byte_add(0xC);
            k = k.wrapping_sub(1);
            if k == 0 {
                break;
            }
        }
        if !x(max_d2).fcom(x(ZERO)).test_any(0x41) {
            max_d2 = f32_of(rw_sqrt(max_d2));
        }
    }
    wr::<f32>(out, 0, cx);
    wr::<f32>(out, 4, cy);
    wr::<f32>(out, 8, cz);
    wr::<f32>(out, 0xC, f32_of(x(max_d2).fmul_m(RADIUS_SCALE)));
    mt
}

/// `RpGeometryAddMorphTargets(geometry, n)` (0x74C310): grows the morph-target block (realloc, moving
/// the old vertex data up by `n * 0x1C` bytes backwards, or malloc; hint 0x3000F; failure -> error
/// {2, E_RW_NOMEM(total)}, -1), re-points every target's verts/normals, initialises the new targets
/// (zero sphere, parent). Returns the first new index.
#[hook(addr = 0x74C310, conv = cdecl, name = "RpGeometryAddMorphTargets", expect = "83 EC 08 53 55")]
pub fn rp_geometry_add_morph_targets(gm: GPtr<u8>, n: i32) -> i32 {
    let flags = rd::<u32>(gm, 8);
    let per: u32 = if flags & FLAG_NATIVE != 0 {
        0x1C
    } else {
        let a = rd::<u32>(gm, 0x14).wrapping_mul(12);
        let mut p = a.wrapping_add(0x1C);
        if flags & FLAG_NORMALS != 0 {
            p = p.wrapping_add(a);
        }
        p
    };
    let total = (rd::<i32>(gm, 0x18).wrapping_add(n) as u32).wrapping_mul(per);
    let old = p8(rd::<u32>(gm, 0x5C));
    let p = if !old.is_null() {
        let p = rw_realloc(old, total, 0x3000F);
        if p.is_null() {
            error2_p2(E_RW_NOMEM, total);
            return -1;
        }
        let old_num = rd::<u32>(gm, 0x18);
        let mut src = p.byte_add((old_num.wrapping_mul(per) as i32 as isize) - 1);
        let mut dst = src.byte_add((n as u32).wrapping_mul(0x1C) as i32 as isize);
        let mut count = per.wrapping_sub(0x1C).wrapping_mul(old_num);
        while count != 0 {
            dst.write(src.read());
            dst = dst.byte_add(-1);
            src = src.byte_add(-1);
            count -= 1;
        }
        p
    } else {
        let p = rw_malloc(total, 0x3000F);
        if p.is_null() {
            error2_p2(E_RW_NOMEM, total);
            return -1;
        }
        p
    };
    let num = rd::<i32>(gm, 0x18).wrapping_add(n);
    wr::<i32>(gm, 0x18, num);
    wr::<u32>(gm, 0x5C, p.addr() as u32);
    let mut cur = p.byte_add((num as u32).wrapping_mul(0x1C) as i32 as isize);
    let mut i: i32 = 0;
    if num > 0 {
        loop {
            let mt = p8(rd::<u32>(gm, 0x5C)).byte_add(i as isize * 0x1C);
            wr::<u32>(mt, 0x14, 0);
            wr::<u32>(mt, 0x18, 0);
            if rd::<u32>(gm, 8) & FLAG_NATIVE == 0 && rd::<u32>(gm, 0x14) != 0 {
                wr::<u32>(mt, 0x14, cur.addr() as u32);
                cur = cur.byte_add(rd::<u32>(gm, 0x14).wrapping_mul(12) as i32 as isize);
                if rd::<u8>(gm, 8) & 0x10 != 0 {
                    wr::<u32>(mt, 0x18, cur.addr() as u32);
                    cur = cur.byte_add(rd::<u32>(gm, 0x14).wrapping_mul(12) as i32 as isize);
                }
            }
            i += 1;
            if i >= rd::<i32>(gm, 0x18) {
                break;
            }
        }
    }
    let mut i = rd::<i32>(gm, 0x18).wrapping_sub(n);
    while i < rd::<i32>(gm, 0x18) {
        let mt = p8(rd::<u32>(gm, 0x5C)).byte_add((i as u32).wrapping_mul(0x1C) as i32 as isize);
        wr::<u32>(mt, 4, 0);
        wr::<u32>(mt, 8, 0);
        wr::<u32>(mt, 0xC, 0);
        wr::<u32>(mt, 0x10, 0);
        wr::<u32>(mt, 0, gm.addr() as u32);
        i += 1;
    }
    rd::<i32>(gm, 0x18).wrapping_sub(n)
}

/// `RpGeometryAddMorphTarget(geometry)` (0x74C4D0): `RpGeometryAddMorphTargets(geometry, 1)`.
#[hook(addr = 0x74C4D0, conv = cdecl, name = "RpGeometryAddMorphTarget", expect = "8B 44 24 04 6A 01")]
pub fn rp_geometry_add_morph_target(gm: GPtr<u8>) -> i32 {
    add_morph_targets_call(gm, 1)
}

/// `RpGeometryTriangleSetVertexIndices(geometry, tri, v1, v2, v3)` (0x74C690). Returns geometry.
#[hook(addr = 0x74C690, conv = cdecl, name = "RpGeometryTriangleSetVertexIndices", expect = "8B 44 24 08 66 8B 4C 24 0C")]
pub fn rp_geometry_triangle_set_vertex_indices(
    gm: GPtr<u8>,
    tri: GPtr<u8>,
    v1: u32,
    v2: u32,
    v3: u32,
) -> GPtr<u8> {
    wr::<u16>(tri, 0, v1 as u16);
    wr::<u16>(tri, 2, v2 as u16);
    wr::<u16>(tri, 4, v3 as u16);
    gm
}

/// `RpGeometryTriangleSetMaterial(geometry, tri, material)` (0x74C6C0): NULL -> matIndex 0xFFFF; else the
/// material's list index (appended when missing; failure -> NULL, tri unchanged). Returns geometry.
#[hook(addr = 0x74C6C0, conv = cdecl, name = "RpGeometryTriangleSetMaterial", expect = "53 8B 5C 24 08")]
pub fn rp_geometry_triangle_set_material(gm: GPtr<u8>, tri: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> {
    if m.is_null() {
        wr::<u16>(tri, 6, 0xFFFF);
        return gm;
    }
    let list = gm.byte_add(0x20);
    let mut i = material_list_find_call(list, m);
    if i < 0 {
        i = material_list_append_call(list, m);
        if i < 0 {
            return GPtr::NULL;
        }
    }
    wr::<u16>(tri, 6, i as u16);
    gm
}

/// `RpGeometryTriangleGetVertexIndices(geometry, tri, v1, v2, v3)` (0x74C720): the non-NULL outputs.
#[hook(addr = 0x74C720, conv = cdecl, name = "RpGeometryTriangleGetVertexIndices", expect = "8B 44 24 0C 8B 4C 24 08")]
pub fn rp_geometry_triangle_get_vertex_indices(
    gm: GPtr<u8>,
    tri: GPtr<u8>,
    v1: GPtr<u16>,
    v2: GPtr<u16>,
    v3: GPtr<u16>,
) -> GPtr<u8> {
    if !v1.is_null() {
        v1.write(rd::<u16>(tri, 0));
    }
    if !v2.is_null() {
        v2.write(rd::<u16>(tri, 2));
    }
    if !v3.is_null() {
        v3.write(rd::<u16>(tri, 4));
    }
    gm
}

/// `RpGeometryTriangleGetMaterial(geometry, tri)` (0x74C760): matIndex 0xFFFF -> NULL, else
/// `materials[matIndex]` (no bounds check).
#[hook(addr = 0x74C760, conv = cdecl, name = "RpGeometryTriangleGetMaterial", expect = "8B 44 24 08 66 8B 40 06")]
pub fn rp_geometry_triangle_get_material(gm: GPtr<u8>, tri: GPtr<u8>) -> GPtr<u8> {
    let mi = rd::<u16>(tri, 6);
    if mi == 0xFFFF {
        return GPtr::NULL;
    }
    p8(rd::<u32>(p8(rd::<u32>(gm, 0x20)), mi as isize * 4))
}

/// `RpGeometryForAllMaterials(geometry, cb, data)` (0x74C790): count read once, materials pointer
/// re-read; stops when `cb` returns NULL. Returns geometry.
#[hook(addr = 0x74C790, conv = cdecl, name = "RpGeometryForAllMaterials", expect = "53 55 56 57 8B 7C 24 14")]
pub fn rp_geometry_for_all_materials(gm: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> {
    let n = rd::<i32>(gm, 0x24);
    let mut i = 0;
    while i < n {
        let m = p8(rd::<u32>(p8(rd::<u32>(gm, 0x20)), i as isize * 4));
        if call_material_cb(cb, m, data).is_null() {
            break;
        }
        i += 1;
    }
    gm
}

/// `RpGeometryLock(geometry, lockMode)` (0x74C7D0): `lockedSinceLastInst |= lockMode`; polygon locks
/// destroy the mesh. Returns geometry.
#[hook(addr = 0x74C7D0, conv = cdecl, name = "RpGeometryLock", expect = "8B 44 24 08 56")]
pub fn rp_geometry_lock(gm: GPtr<u8>, mode: u32) -> GPtr<u8> {
    wr::<u16>(gm, 0xC, rd::<u16>(gm, 0xC) | mode as u16);
    if mode & 1 != 0 {
        let mesh = rd::<u32>(gm, 0x54);
        if mesh != 0 {
            rp_mesh_destroy(p8(mesh));
            wr::<u32>(gm, 0x54, 0);
        }
    }
    gm
}

/// Index of `v` in the first `n` (u16) entries of `arr`, appending it (and counting) when missing.
fn find_or_append(arr: GPtr<u8>, count: &mut u32, v: u32) -> u32 {
    let mut i: u32 = 0;
    while (i as u16) < (*count as u16) {
        if rd::<u32>(arr, (i & 0xFFFF) as isize * 4) == v {
            break;
        }
        i += 1;
    }
    if (i as u16) == (*count as u16) {
        wr::<u32>(arr, (i & 0xFFFF) as isize * 4, v);
        *count = count.wrapping_add(1);
    }
    i
}

/// `RpGeometryUnlock(geometry)` (0x74C800): with no mesh, builds one: every triangle (only when there
/// are materials) goes into a build mesh with its texture / raster / pipeline indices (three
/// `numMaterials * 4` scratch arrays, hints 0x10006 / 0x10503 / 0x10503, unchecked); then
/// `_rpMeshOptimise(bm, tristrip)` (NULL -> build mesh destroyed, NULL). Returns geometry.
#[hook(addr = 0x74C800, conv = cdecl, name = "RpGeometryUnlock", expect = "83 EC 24 53 8B 5C 24 2C")]
pub fn rp_geometry_unlock(gm: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(gm, 0x54) != 0 {
        return gm;
    }
    let bm = rp_build_mesh_create(rd::<u32>(gm, 0x10));
    if bm.is_null() {
        return GPtr::NULL;
    }
    let nmat = rd::<i32>(gm, 0x24);
    if nmat > 0 {
        let size = (nmat as u32).wrapping_mul(4);
        let texs = rw_malloc(size, 0x10006);
        let rasters = rw_malloc(size, 0x10503);
        let pipes = rw_malloc(size, 0x10503);
        let (mut n_tex, mut n_ras, mut n_pipe) = (0u32, 0u32, 0u32);
        let mut i: i32 = 0;
        if rd::<i32>(gm, 0x10) > 0 {
            loop {
                let t = p8(rd::<u32>(gm, 0x2C)).byte_add(i as isize * 8);
                let mat = material_list_get_material_call(gm.byte_add(0x20), rd::<u16>(t, 6) as i32);
                let tex = rd::<u32>(mat, 0);
                let ti = find_or_append(texs, &mut n_tex, tex);
                let ras = if tex != 0 { rd::<u32>(p8(tex), 0) } else { 0 };
                let ri = find_or_append(rasters, &mut n_ras, ras);
                let pipe = rd::<u32>(mat, 8);
                let pi = find_or_append(pipes, &mut n_pipe, pipe);
                rp_build_mesh_add_triangle(
                    bm,
                    mat,
                    rd::<u16>(t, 0) as u32,
                    rd::<u16>(t, 2) as u32,
                    rd::<u16>(t, 4) as u32,
                    rd::<u16>(t, 6) as u32,
                    ti,
                    ri,
                    pi,
                );
                i += 1;
                if i >= rd::<i32>(gm, 0x10) {
                    break;
                }
            }
        }
        rw_free(texs);
        rw_free(rasters);
        rw_free(pipes);
    }
    let strip = if rd::<u8>(gm, 8) & FLAG_TRISTRIP as u8 != 0 {
        1
    } else {
        0
    };
    let mesh = rp_mesh_optimise(bm, strip);
    if mesh.is_null() {
        rp_build_mesh_destroy(bm);
        return GPtr::NULL;
    }
    wr::<u32>(gm, 0x54, mesh.addr() as u32);
    gm
}

/// `RpGeometryForAllMeshes(geometry, cb, data)` (0x74CA60): NULL without a mesh or when
/// `_rpMeshHeaderForAllMeshes` returns NULL, else geometry.
#[hook(addr = 0x74CA60, conv = cdecl, name = "RpGeometryForAllMeshes", expect = "56 8B 74 24 08")]
pub fn rp_geometry_for_all_meshes(gm: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> {
    let mesh = rd::<u32>(gm, 0x54);
    if mesh == 0 || mesh_header_for_all_meshes_call(p8(mesh), cb, data).is_null() {
        return GPtr::NULL;
    }
    gm
}

orig! {
    /// `_rpMeshHeaderForAllMeshes(header, cb, data)` (0x758D30)
    fn mesh_header_for_all_meshes_call(h: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x758D30 => crate::mesh::rp_mesh_header_for_all_meshes;
}

/// `RpGeometryCreate(numVerts, numTriangles, format)` (0x74CA90): validates the counts (numVerts >=
/// 0x10000 -> error {2, 6}), derives the texture coordinate set count and the low flag byte, mallocs
/// the geometry with its vertex/triangle arrays (hint 0x3000F), initialises it (matIndex 0xFFFF), adds
/// the first morph target (failure -> list deinit, free, NULL) and inits the plugins.
/// Original bug kept: more than 8 coordinate sets overrun `texCoords[8]`.
#[hook(addr = 0x74CA90, conv = cdecl, name = "RpGeometryCreate", expect = "8B 44 24 04 83 EC 08")]
pub fn rp_geometry_create(num_verts: i32, num_tris: i32, format: u32) -> GPtr<u8> {
    if num_verts < 0 {
        return GPtr::NULL;
    }
    if num_verts >= 0x10000 {
        error1_p2(E_TOO_MANY_VERTICES);
        return GPtr::NULL;
    }
    if num_tris < 0 {
        return GPtr::NULL;
    }
    let mut size = g::<u32>(GEOMETRY_REGISTRY as usize).read();
    let nt: u32 = if format & 0xFF_0000 != 0 {
        (format >> 16) & 0xFF
    } else if format & 0x80 != 0 {
        2
    } else {
        (format >> 2) & 1
    };
    let bits = if nt == 1 {
        4
    } else if 1 < nt {
        0x80
    } else {
        0
    };
    let fl = (format & 0x7B) | bits;
    let native = format & FLAG_NATIVE;
    if native == 0 {
        if fl & 8 != 0 {
            size = size.wrapping_add((num_verts as u32).wrapping_mul(4));
        }
        if nt > 0 {
            size = size.wrapping_add(nt.wrapping_mul(num_verts as u32).wrapping_mul(8));
        }
        size = size.wrapping_add((num_tris as u32).wrapping_mul(8));
    }
    let gm = rw_malloc(size, 0x3000F);
    if gm.is_null() {
        return GPtr::NULL;
    }
    if material_list_initialize_call(gm.byte_add(0x20)).is_null() {
        return GPtr::NULL;
    }
    wr::<u32>(gm, 0x5C, 0);
    wr::<u32>(gm, 0x18, 0);
    wr::<u32>(gm, 4, 0);
    wr::<u32>(gm, 0x58, 0);
    wr::<u16>(gm, 0xC, 0);
    wr::<u32>(gm, 0x54, 0);
    wr::<u8>(gm, 0, 8);
    wr::<u8>(gm, 1, 0);
    wr::<u8>(gm, 2, 0);
    wr::<u8>(gm, 3, 0);
    wr::<i16>(gm, 0xE, 1);
    wr::<u32>(gm, 0x1C, nt);
    for k in 0..8 {
        wr::<u32>(gm, 0x34 + k * 4, 0);
    }
    wr::<u32>(gm, 0x30, 0);
    wr::<u32>(gm, 0x2C, 0);
    wr::<i32>(gm, 0x10, num_tris);
    wr::<u32>(gm, 8, (format & 0x0F00_0000) | fl);
    wr::<i32>(gm, 0x14, num_verts);
    if native == 0 {
        let mut p = gm.byte_add(g::<u32>(GEOMETRY_REGISTRY as usize).read() as i32 as isize);
        let nv = num_verts as u32;
        if fl & 8 != 0 && nv != 0 {
            wr::<u32>(gm, 0x30, p.addr() as u32);
            p = p.byte_add(nv.wrapping_mul(4) as isize);
        }
        if nt > 0 && nv != 0 {
            for k in 0..nt as isize {
                wr::<u32>(gm, 0x34 + k * 4, p.addr() as u32);
                p = p.byte_add(nv.wrapping_mul(8) as isize);
            }
        }
        if num_tris != 0 {
            wr::<u32>(gm, 0x2C, p.addr() as u32);
            for i in 0..num_tris as isize {
                wr::<u16>(p8(rd::<u32>(gm, 0x2C)), i * 8 + 6, 0xFFFF);
            }
        }
    }
    if add_morph_targets_call(gm, 1) < 0 {
        material_list_deinitialize_call(gm.byte_add(0x20));
        rw_free(gm);
        return GPtr::NULL;
    }
    plugin_init_object(p8(GEOMETRY_REGISTRY), gm);
    gm
}

/// `_rpGeometryAddRef(geometry)` (0x74CCB0): refCount++ (16-bit). EAX = geometry.
#[hook(addr = 0x74CCB0, conv = cdecl, name = "_rpGeometryAddRef", expect = "8B 44 24 04 66 FF 40 0E")]
pub fn rp_geometry_add_ref(gm: GPtr<u8>) -> GPtr<u8> {
    wr::<i16>(gm, 0xE, rd::<i16>(gm, 0xE).wrapping_add(1));
    gm
}

/// `RpGeometryDestroy(geometry)` (0x74CCC0): `refCount - 1 > 0` -> refCount--. Otherwise frees the
/// resource entry, locks everything (0xFFF), destroys the mesh, plugin deinit, frees the morph
/// targets, releases the materials, refCount-- and frees the geometry. Returns 1.
#[hook(addr = 0x74CCC0, conv = cdecl, name = "RpGeometryDestroy", expect = "56 8B 74 24 08")]
pub fn rp_geometry_destroy(gm: GPtr<u8>) -> u32 {
    let rc = rd::<i16>(gm, 0xE);
    if (rc as i32) - 1 > 0 {
        wr::<i16>(gm, 0xE, rc.wrapping_sub(1));
        return 1;
    }
    let re = rd::<u32>(gm, 0x58);
    if re != 0 {
        rw_resources_free_res_entry(p8(re));
    }
    let mesh = rd::<u32>(gm, 0x54);
    wr::<u16>(gm, 0xC, rd::<u16>(gm, 0xC) | 0xFFF);
    if mesh != 0 {
        rp_mesh_destroy(p8(mesh));
        wr::<u32>(gm, 0x54, 0);
    }
    plugin_deinit_object(p8(GEOMETRY_REGISTRY), gm);
    let mt = rd::<u32>(gm, 0x5C);
    if mt != 0 {
        rw_free(p8(mt));
        wr::<u32>(gm, 0x5C, 0);
    }
    material_list_deinitialize_call(gm.byte_add(0x20));
    wr::<i16>(gm, 0xE, rd::<i16>(gm, 0xE).wrapping_sub(1));
    rw_free(gm);
    1
}

/// `RpGeometryRegisterPlugin(size, id, ctor, dtor, copy)` (0x74CD70).
#[hook(addr = 0x74CD70, conv = cdecl, name = "RpGeometryRegisterPlugin", expect = "8B 44 24 14 8B 4C 24 10")]
pub fn rp_geometry_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 {
    plugin_add(p8(GEOMETRY_REGISTRY), size, id, ctor, dtor, copy)
}

/// `RpGeometryRegisterPluginStream(id, read, write, getSize)` (0x74CDA0).
#[hook(addr = 0x74CDA0, conv = cdecl, name = "RpGeometryRegisterPluginStream", expect = "8B 44 24 10 8B 4C 24 0C")]
pub fn rp_geometry_register_plugin_stream(id: u32, read: u32, write: u32, get_size: u32) -> i32 {
    plugin_add_stream(p8(GEOMETRY_REGISTRY), id, read, write, get_size)
}

orig! {
    /// `RwStreamFindChunk(stream, type, length, version)` (0x7ED2D0)
    fn rw_stream_find_chunk_call(s: GPtr<u8>, ty: u32, len: GPtr<u32>, ver: GPtr<u32>) -> u32 = cdecl 0x7ED2D0 => crate::stream::rw_stream_find_chunk;
    /// `RwStreamRead(stream, buf, len)` (0x7EC9D0)
    fn rw_stream_read_call(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => crate::stream::rw_stream_read;
    /// `RwStreamReadReal(stream, p, numBytes)` (0x7ED4F0)
    fn rw_stream_read_real_call(s: GPtr<u8>, p: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x7ED4F0 => crate::stream::rw_stream_read_real;
    /// `RpGeometryCreate(numVerts, numTriangles, format)` (0x74CA90)
    fn rp_geometry_create_call(nv: i32, nt: i32, format: u32) -> GPtr<u8> = cdecl 0x74CA90 => rp_geometry_create;
    /// `RpGeometryLock(geometry, mode)` (0x74C7D0)
    fn rp_geometry_lock_call(gm: GPtr<u8>, mode: u32) -> GPtr<u8> = cdecl 0x74C7D0 => rp_geometry_lock;
    /// `RpGeometryUnlock(geometry)` (0x74C800)
    fn rp_geometry_unlock_call(gm: GPtr<u8>) -> GPtr<u8> = cdecl 0x74C800 => rp_geometry_unlock;
    /// `_rpMaterialSetDefaultSurfaceProperties(props)` (0x74D870)
    fn set_default_surface_properties(p: GPtr<u8>) -> u32 = cdecl 0x74D870 => unported;
    /// `_rpMaterialListStreamRead(stream, list)` (0x74E600)
    fn material_list_stream_read_call(s: GPtr<u8>, l: GPtr<u8>) -> GPtr<u8> = cdecl 0x74E600 => crate::material::material_list_stream_read;
    /// `_rwPluginRegistryReadDataChunks(reg, stream, obj)` (0x808980)
    fn plugin_read_data_chunks(reg: GPtr<u8>, s: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808980 => crate::plugin::registry_read_data_chunks;
    /// `FUN_0074d6d0(geometry)`: the geometry free helper of `RpGeometryStreamRead`'s late failures
    fn geometry_free_helper(gm: GPtr<u8>) -> u32 = cdecl 0x74D6D0 => unported;
}

/// Failure path "F" of `RpGeometryStreamRead` (0x74D3AC / 0x74D522 / 0x74D600): refCount - 1 > 0 ->
/// refCount--; else resource entry freed, `RpGeometryLock(g, 0xFFF)`, plugin deinit, morph targets
/// freed, materials released, refCount--, geometry freed.
fn stream_read_fail(gm: GPtr<u8>) {
    let rc = rd::<i16>(gm, 0xE);
    if (rc as i32) - 1 > 0 {
        wr::<i16>(gm, 0xE, rc.wrapping_sub(1));
        return;
    }
    let re = rd::<u32>(gm, 0x58);
    if re != 0 {
        rw_resources_free_res_entry(p8(re));
    }
    rp_geometry_lock_call(gm, 0xFFF);
    plugin_deinit_object(p8(GEOMETRY_REGISTRY), gm);
    let mt = rd::<u32>(gm, 0x5C);
    if mt != 0 {
        rw_free(p8(mt));
        wr::<u32>(gm, 0x5C, 0);
    }
    material_list_deinitialize_call(gm.byte_add(0x20));
    wr::<i16>(gm, 0xE, rd::<i16>(gm, 0xE).wrapping_sub(1));
    rw_free(gm);
}

/// Failure path "F2" (0x74D5B4): refCount - 1 > 0 -> refCount--; else resource entry freed,
/// refCount--, `FUN_0074d6d0(g)`.
fn stream_read_fail2(gm: GPtr<u8>) {
    let rc = rd::<i16>(gm, 0xE);
    if (rc as i32) - 1 > 0 {
        wr::<i16>(gm, 0xE, rc.wrapping_sub(1));
        return;
    }
    let re = rd::<u32>(gm, 0x58);
    if re != 0 {
        rw_resources_free_res_entry(p8(re));
    }
    wr::<i16>(gm, 0xE, rd::<i16>(gm, 0xE).wrapping_sub(1));
    geometry_free_helper(gm);
}

fn version_ok(v: u32) -> bool {
    (0x34000..=0x36003).contains(&v)
}

/// `RpGeometryStreamRead(stream)` (0x74D190): the struct chunk (version checked, error {2,
/// E_RW_BADVERSION}) gives the 16-byte header (+ 12 bytes of surface properties before 0x34001), the
/// geometry is created and gets its extra morph targets, then (non-native, with vertices) prelit
/// colours, texture coordinates (`RwStreamReadReal`), triangles (each dword's halves swapped: file
/// `[w0 w1 w2 w3]` -> `v = {w1, w0, w3}`, `matIndex = w2`), per morph target the sphere and the verts /
/// normals (one read when both), the material list (with the old surface properties as defaults
/// around it), plugin data and `RpGeometryUnlock`. Failures go through F / F2 (see the helpers).
/// Original bugs kept: a missing material-list chunk returns NULL without freeing the geometry; a
/// failed material-list read before 0x34001 leaves the header's surface properties as the defaults.
#[hook(addr = 0x74D190, conv = cdecl, name = "RpGeometryStreamRead", expect = "83 EC 3C 8D 44 24 40")]
pub fn rp_geometry_stream_read(s: GPtr<u8>) -> GPtr<u8> {
    let mut ver: u32 = 0;
    let vp = GPtr::from_raw(&mut ver);
    // {format, numTriangles, numVertices, numMorphTargets}
    let mut hdr = [0u32; 4];
    let hp: GPtr<u8> = GPtr::from_raw(hdr.as_mut_ptr()).cast();
    let mut surf = [0u32; 3];
    let sp: GPtr<u8> = GPtr::from_raw(surf.as_mut_ptr()).cast();
    if rw_stream_find_chunk_call(s, 1, GPtr::NULL, vp) == 0 {
        return GPtr::NULL;
    }
    let v = vp.read();
    if !version_ok(v) {
        error1_p2(E_RW_BADVERSION);
        return GPtr::NULL;
    }
    if rw_stream_read_call(s, hp, 0x10) != 0x10 {
        return GPtr::NULL;
    }
    if v < 0x34001 && rw_stream_read_call(s, sp, 0xC) != 0xC {
        return GPtr::NULL;
    }
    let format = rd::<u32>(hp, 0);
    let gm = rp_geometry_create_call(rd::<i32>(hp, 8), rd::<i32>(hp, 4), format);
    if gm.is_null() {
        return GPtr::NULL;
    }
    let nmt = rd::<i32>(hp, 0xC);
    if nmt > 1 && add_morph_targets_call(gm, nmt - 1) < 0 {
        stream_read_fail(gm);
        return GPtr::NULL;
    }
    if rd::<u32>(gm, 8) & FLAG_NATIVE == 0 && rd::<u32>(gm, 0x14) != 0 {
        let nv = rd::<u32>(gm, 0x14);
        if format & 8 != 0 {
            let n = nv.wrapping_mul(4);
            if rw_stream_read_call(s, p8(rd::<u32>(gm, 0x30)), n) != n {
                stream_read_fail(gm);
                return GPtr::NULL;
            }
        }
        if rd::<i32>(gm, 0x1C) > 0 {
            let size = rd::<u32>(gm, 0x14).wrapping_mul(8);
            let mut k = 0;
            while k < rd::<i32>(gm, 0x1C) {
                if rw_stream_read_real_call(s, p8(rd::<u32>(gm, 0x34 + k as isize * 4)), size).is_null() {
                    stream_read_fail(gm);
                    return GPtr::NULL;
                }
                k += 1;
            }
        }
        let nt = rd::<u32>(gm, 0x10);
        if nt != 0 {
            let tri = p8(rd::<u32>(gm, 0x2C));
            let n = nt.wrapping_mul(8);
            if rw_stream_read_call(s, tri, n) != n {
                stream_read_fail(gm);
                return GPtr::NULL;
            }
            let mut t = tri;
            let mut c = nt;
            loop {
                let w0 = rd::<u16>(t, 0);
                let w1 = rd::<u16>(t, 2);
                wr::<u16>(t, 0, w1);
                let w3 = rd::<u16>(t, 6);
                wr::<u16>(t, 2, w0);
                let w2 = rd::<u16>(t, 4);
                wr::<u16>(t, 4, w3);
                wr::<u16>(t, 6, w2);
                t = t.byte_add(8);
                c = c.wrapping_sub(1);
                if c == 0 {
                    break;
                }
            }
        }
    }
    // {RwSphere sphere; hasVerts; hasNormals}
    let mut mh = [0u32; 6];
    let mp: GPtr<u8> = GPtr::from_raw(mh.as_mut_ptr()).cast();
    let mut i: i32 = 0;
    while i < rd::<i32>(gm, 0x18) {
        let mt = p8(rd::<u32>(gm, 0x5C)).byte_add(i as isize * 0x1C);
        if rw_stream_read_call(s, mp, 0x18) != 0x18 {
            stream_read_fail(gm);
            return GPtr::NULL;
        }
        for k in 0..4 {
            wr::<u32>(mt, 4 + k * 4, rd::<u32>(mp, k * 4));
        }
        let has_verts = rd::<u32>(mp, 0x10) != 0;
        let has_normals = rd::<u32>(mp, 0x14) != 0;
        let nv12 = rd::<u32>(gm, 0x14).wrapping_mul(12);
        let ok = if has_verts && has_normals {
            !rw_stream_read_real_call(s, p8(rd::<u32>(mt, 0x14)), rd::<u32>(gm, 0x14).wrapping_mul(24)).is_null()
        } else if has_verts {
            !rw_stream_read_real_call(s, p8(rd::<u32>(mt, 0x14)), nv12).is_null()
        } else if has_normals {
            !rw_stream_read_real_call(s, p8(rd::<u32>(mt, 0x18)), nv12).is_null()
        } else {
            true
        };
        if !ok {
            stream_read_fail(gm);
            return GPtr::NULL;
        }
        i += 1;
    }
    if rw_stream_find_chunk_call(s, 8, GPtr::NULL, vp) == 0 {
        return GPtr::NULL;
    }
    let v = vp.read();
    if !version_ok(v) {
        stream_read_fail(gm);
        error1_p2(E_RW_BADVERSION);
        return GPtr::NULL;
    }
    if v < 0x34001 {
        set_default_surface_properties(sp);
    }
    if material_list_stream_read_call(s, gm.byte_add(0x20)).is_null() {
        stream_read_fail2(gm);
        return GPtr::NULL;
    }
    if vp.read() < 0x34001 {
        set_default_surface_properties(GPtr::NULL);
    }
    if plugin_read_data_chunks(p8(GEOMETRY_REGISTRY), s, gm).is_null() || rp_geometry_unlock_call(gm).is_null() {
        stream_read_fail2(gm);
        return GPtr::NULL;
    }
    gm
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_GEOMETRY_MODULE_OPEN,
    HOOK_GEOMETRY_MODULE_CLOSE,
    HOOK_RP_MORPH_TARGET_CALC_BOUNDING_SPHERE,
    HOOK_RP_GEOMETRY_ADD_MORPH_TARGETS,
    HOOK_RP_GEOMETRY_ADD_MORPH_TARGET,
    HOOK_RP_GEOMETRY_TRIANGLE_SET_VERTEX_INDICES,
    HOOK_RP_GEOMETRY_TRIANGLE_SET_MATERIAL,
    HOOK_RP_GEOMETRY_TRIANGLE_GET_VERTEX_INDICES,
    HOOK_RP_GEOMETRY_TRIANGLE_GET_MATERIAL,
    HOOK_RP_GEOMETRY_FOR_ALL_MATERIALS,
    HOOK_RP_GEOMETRY_LOCK,
    HOOK_RP_GEOMETRY_UNLOCK,
    HOOK_RP_GEOMETRY_FOR_ALL_MESHES,
    HOOK_RP_GEOMETRY_CREATE,
    HOOK_RP_GEOMETRY_ADD_REF,
    HOOK_RP_GEOMETRY_DESTROY,
    HOOK_RP_GEOMETRY_REGISTER_PLUGIN,
    HOOK_RP_GEOMETRY_REGISTER_PLUGIN_STREAM,
    HOOK_RP_GEOMETRY_STREAM_READ,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
