//! FRONTIER-2 (batch `frontier2-1`): the procedural objects placed on collision triangles (`ProcObjectMan_c`,
//! `ProcSurfaceInfo_c`; user-hit walking around: `ProcessTriangleAdded` from the plant manager's loc-tri pass).
//! Ported from the 1.0 US bytes; gta-reversed differs in three places, the bytes are followed: the grid pass
//! restarts y at every x column, the scale fix-up runs for buildings (not objects), and it runs when either scale
//! differs from 1 (not both).

use sa_core::abi::HookReg;
use sa_core::field::gf;
use sa_core::bindings::{
    cross_product, entity_update_rw_frame, list_add_item, list_remove_head, list_remove_item, matrix_ctor_rw,
    matrix_dtor, matrix_update_rw, matrix_update_rw_matrix, object_ctor_5a1d70, object_new, vector_normalise,
    world_add, world_remove,
};
use sa_core::crt::{rand_at, srand};
use sa_core::orig::at_site;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::fpu::{fcom_parity, ge_or_nan, gt, le_or_nan, lt};
use sa_stream::bindings::{allocate_static_matrix, building_ctor, building_new};
use sa_world::bindings::{
    get_modelling_matrix, placeable_set_matrix, placeable_set_posn, set_orientation, set_velocity,
    simple_transform_update_rw_matrix, surface_is_water,
};
use sa_world::world::common::cam_pos;

use crate::common::{ftol32, g, rd, wr, x, xi};

/// `g_procObjMan` (0xBB7CB0): +0 allocated matrices, +4 surface-info count, +8 the 0x48-byte surface infos.
const PROC_OBJ_MAN: usize = 0xBB_7CB0;
/// `g_procObjMan.m_ObjectsList` (the free `ProcObjectListItem` list).
const OBJECTS_LIST: usize = 0xBB_C8B8;
/// `CObject::nNoTempObjects` (int16).
const NO_TEMP_OBJECTS: usize = 0xBB_4A70;
/// `CPools::ms_pObjectPool` / `ms_pBuildingPool` (their +0x11 is `m_bDealWithNoMemory`).
const OBJECT_POOL: usize = 0xB7_449C;
const BUILDING_POOL: usize = 0xB7_4498;
/// `SurfaceInfos_c g_surfaceInfos`.
const SURFACE_INFOS: usize = 0xB7_9538;
/// `CModelInfo::ms_modelInfoPtrs`.
const MODEL_INFO_PTRS: usize = 0xA9_B0C8;

orig! {
    /// virtual `CEntity::SetIsStatic(bool)` (vtable +0x10)
    fn entity_set_is_static(e: GPtr<u8>, v: u32) = thiscall vslot 0x10 => unported;
    /// virtual `CEntity::SetModelIndexNoCreate(uint32)` (vtable +0x18)
    fn entity_set_model_index_no_create(e: GPtr<u8>, m: u32) = thiscall vslot 0x18 => unported;
    /// virtual `CEntity::CreateRwObject()` (vtable +0x1C)
    fn entity_create_rw_object(e: GPtr<u8>) = thiscall vslot 0x1C => unported;
    /// virtual `CEntity::DeleteRwObject()` (vtable +0x20)
    fn entity_delete_rw_object(e: GPtr<u8>) = thiscall vslot 0x20 => unported;
    /// virtual scalar deleting destructor (vtable +0)
    fn entity_vdelete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `ProcSurfaceInfo_c::AddObjects(CPlantLocTri*)` (0x5A3850)
    fn add_objects_o(this: GPtr<u8>, tri: GPtr<u8>) -> i32 = thiscall 0x5A3850 => add_objects;
    /// `ProcSurfaceInfo_c::AddObject(CVector pos, CVector normal, tColLighting)` (0x5A32D0)
    fn add_object_o(this: GPtr<u8>, px: f32, py: f32, pz: f32, nx: f32, ny: f32, nz: f32, lighting: u32) -> GPtr<u8>
        = thiscall 0x5A32D0 => add_object;
    /// `ProcSurfaceInfo_c::IsPtInTriangle2D` (0x5A2F80; ECX unread, `ret 0x3C`)
    fn is_pt_in_triangle_2d_o(px: f32, py: f32, ax: f32, ay: f32, az: f32, bx: f32, by: f32, bz: f32, cx: f32, cy: f32,
        cz: f32, nx: f32, ny: f32, nz: f32, z: GPtr<f32>) -> u8 = stdcall 0x5A2F80 => is_pt_in_triangle_2d;
}

/// `ProcObjectMan_c::ProcessTriangleAdded(CPlantLocTri*)` (0x5A3F20, `ret 4`): every surface info whose surface id
/// (+0) is the triangle's (+0x46) adds its objects (0x5A3850); the sum, returned in AL.
#[hook(addr = 0x5A3F20, conv = thiscall, name = "ProcObjectMan_c::ProcessTriangleAdded", expect = "51 55 57 8B F9 8B 4F 04",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes")]
pub fn process_triangle_added(this: GPtr<u8>, tri: GPtr<u8>) -> u8 {
    let mut total: i32 = 0;
    let mut i = 0;
    while i < rd::<i32>(this, 4) {
        let info = this.byte_add(8 + 0x48 * i as isize);
        if rd::<u8>(info, 0) == rd::<u8>(tri, 0x46) {
            total = total.wrapping_add(add_objects_o(info, tri));
        }
        i += 1;
    }
    total as u8
}

/// `ProcObjectMan_c::ProcessTriangleRemoved(CPlantLocTri*)` (0x5A3F70, `ret 4`): every object a matching surface
/// info placed on the triangle (item +0xC) goes: an object (type 4) decrements `nNoTempObjects`; the item leaves the
/// info's list for the free list; `DeleteRwObject`, `CWorld::Remove`, the deleting destructor; a matrix it allocated
/// (+0x10) decrements the manager's count.
#[hook(addr = 0x5A3F70, conv = thiscall, name = "ProcObjectMan_c::ProcessTriangleRemoved", expect = "51 53 8B D9 8B 43 04 85",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes")]
pub fn process_triangle_removed(this: GPtr<u8>, tri: GPtr<u8>) {
    let mut i = 0;
    while i < rd::<i32>(this, 4) {
        let list = this.byte_add(0x44 + 0x48 * i as isize);
        if rd::<u8>(list, -0x3C) == rd::<u8>(tri, 0x46) {
            let mut it = rd::<GPtr<u8>>(list, 0);
            while !it.is_null() {
                let next = rd::<GPtr<u8>>(it, 4);
                if rd::<GPtr<u8>>(it, 0xC) == tri {
                    if rd::<u8>(rd::<GPtr<u8>>(it, 8), 0x36) & 7 == 4 {
                        let n = g::<u16>(NO_TEMP_OBJECTS);
                        n.write(n.read().wrapping_sub(1));
                    }
                    list_remove_item(list, it);
                    list_add_item(g(OBJECTS_LIST), it);
                    entity_delete_rw_object(rd::<GPtr<u8>>(it, 8));
                    world_remove(rd::<GPtr<u8>>(it, 8));
                    let o = rd::<GPtr<u8>>(it, 8);
                    if !o.is_null() {
                        entity_vdelete(o, 1);
                    }
                    wr::<u32>(it, 8, 0);
                    if rd::<u8>(it, 0x10) != 0 {
                        wr(this, 0, rd::<i32>(this, 0).wrapping_sub(1));
                        wr::<u8>(it, 0x10, 0);
                    }
                }
                it = next;
            }
        }
        i += 1;
    }
}

/// The 0x5A2E60 row scale (`ret 0xC`): the right, forward and up rows' x / y / z times `sx` / `sy` / `sz`.
#[hook(addr = 0x5A2E60, conv = thiscall, name = "FUN_005a2e60", expect = "8D 41 08 B9 03 00 00 00",
       unverified = "frontier2-1", route = "L2", hazards = "none")]
pub fn matrix_scale_xyz(m: GPtr<u8>, sx: f32, sy: f32, sz: f32) {
    for r in 0..3isize {
        let o = 0x10 * r;
        wr(m, o, x(sx).fmul_m(rd::<f32>(m, o)).to_f32());
        wr(m, o + 4, x(sy).fmul_m(rd::<f32>(m, o + 4)).to_f32());
        wr(m, o + 8, x(sz).fmul_m(rd::<f32>(m, o + 8)).to_f32());
    }
}

/// `ProcSurfaceInfo_c::IsPtInTriangle2D(float x, float y, CVector v1, CVector v2, CVector v3, CVector normal,
/// float* z)` (0x5A2F80, the arguments by value on the stack, ECX unread, `ret 0x3C`): false when any edge
/// function is positive or unordered; else `*z` is the plane's height at (x, y), true.
#[hook(addr = 0x5A2F80, conv = stdcall, name = "ProcSurfaceInfo_c::IsPtInTriangle2D", expect = "D9 44 24 0C D8 64 24 18",
       unverified = "frontier2-1", route = "L2", hazards = "none")]
#[allow(clippy::too_many_arguments)]
pub fn is_pt_in_triangle_2d(px: f32, py: f32, ax: f32, ay: f32, az: f32, bx: f32, by: f32, _bz: f32, cx: f32, cy: f32,
                            _cz: f32, nx: f32, ny: f32, nz: f32, z: GPtr<f32>) -> u8 {
    let zero = x(gf(0x85_8B50));
    let e1 = x(ax).fsub_m(bx).fmul(x(ay).fsub_m(py)).fadd(x(by).fsub_m(ay).fmul(x(ax).fsub_m(px)));
    if fcom_parity(e1, zero, 0x41) {
        return 0;
    }
    let e2 = x(bx).fsub_m(cx).fmul(x(by).fsub_m(py)).fadd(x(cy).fsub_m(by).fmul(x(bx).fsub_m(px)));
    if fcom_parity(e2, zero, 0x41) {
        return 0;
    }
    let e3 = x(ay).fsub_m(cy).fmul(x(cx).fsub_m(px)).fadd(x(cx).fsub_m(ax).fmul(x(cy).fsub_m(py)));
    if fcom_parity(e3, zero, 0x41) {
        return 0;
    }
    let a = x(nx).fmul_m(px).fchs().fsub(x(ny).fmul_m(py));
    let d = x(az).fmul_m(nz).fadd(x(ny).fmul_m(ay)).fadd(x(nx).fmul_m(ax)).fchs();
    z.write(a.fsub(d).fdiv_m(nz).to_f32());
    1
}

/// A CMatrix temporary on the stack (0x48 bytes).
type Mat = [u32; 18];

fn mp(m: &mut Mat) -> GPtr<u8> {
    GPtr::from_raw(m.as_mut_ptr()).cast()
}

fn vp(v: &mut [f32; 3]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

/// `ProcSurfaceInfo_c::AddObject(CVector pos, CVector normal, tColLighting lighting)` (0x5A32D0, `ret 0x1C`).
///
/// Takes an item off the free list (none: null). With alignment (+0x34) on a slope (`normal.z < [0x858EF0]`) and 200
/// allocated matrices: null (the item stays off the list, as in the original). A model with an object info (+0x10 !=
/// -1): with 150 temp objects null, else a `CObject(model, false)` (pool flag set around it), type 5, zero velocity,
/// `nNoTempObjects++`; otherwise a `CBuilding` with `SetModelIndexNoCreate`. Water surfaces set the underwater flag,
/// every object the proc-object flag. Four `rand` draws give the xy scale, z scale, rotation and z offset (each
/// `(max - min) * r + min`). Aligned: a static matrix from `normal x (cos r, sin r, 0)`, its cross with the normal, the
/// normal, and `pos + normal * (zOffset - bbox.min.z)`; else `pos.z = (pos.z - bbox.min.z) + zOffset`, the position
/// and the z rotation. Area, static, RW object, RW matrix and frame, `CWorld::Add`, the surface's list. A building
/// whose xy or z scale is not 1 gets its modelling matrix scaled and lowered by `min.z * zs - min.z`.
#[hook(addr = 0x5A32D0, conv = thiscall, name = "ProcSurfaceInfo_c::AddObject", expect = "6A FF 64 A1 00 00 00 00",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes,x87-transcendental",
       note = "rand at_site 0x5A347B/0x5A34B0/0x5A34E5/0x5A351A; the SEH frame is not reproduced")]
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn add_object(this: GPtr<u8>, px: f32, py: f32, pz: f32, nx: f32, ny: f32, nz: f32, _lighting: u32) -> GPtr<u8> {
    let mut pos = [px, py, pz];
    let mut normal = [nx, ny, nz];
    let item = list_remove_head(g(OBJECTS_LIST));
    if item.is_null() {
        return GPtr::NULL;
    }
    let slope = |n: f32| lt(x(n), x(gf(0x85_8EF0)));
    if rd::<u8>(this, 0x34) != 0 && slope(nz) && g::<i32>(PROC_OBJ_MAN).read() >= 200 {
        return GPtr::NULL;
    }
    let model = rd::<i32>(this, 4);
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * model as usize).read();
    let is_object = rd::<i16>(mi, 0x10) != -1;
    if is_object {
        if g::<u16>(NO_TEMP_OBJECTS).read() >= 150 {
            return GPtr::NULL;
        }
        wr::<u8>(g::<GPtr<u8>>(OBJECT_POOL).read(), 0x11, 1);
        let mem = object_new(0x17C);
        let o = if mem.is_null() { GPtr::NULL } else { object_ctor_5a1d70(mem, rd::<i32>(this, 4), 0) };
        wr(item, 8, o);
        if o.is_null() {
            wr::<u8>(g::<GPtr<u8>>(OBJECT_POOL).read(), 0x11, 0);
            return GPtr::NULL;
        }
        wr::<u8>(o, 0x13C, 5);
        set_velocity(rd::<GPtr<u8>>(item, 8), 0, 0, 0);
        wr::<u8>(g::<GPtr<u8>>(OBJECT_POOL).read(), 0x11, 0);
        let n = g::<u16>(NO_TEMP_OBJECTS);
        n.write(n.read().wrapping_add(1));
    } else {
        wr::<u8>(g::<GPtr<u8>>(BUILDING_POOL).read(), 0x11, 1);
        let mem = building_new(0x38);
        let b = if mem.is_null() { GPtr::NULL } else { building_ctor(mem) };
        wr(item, 8, b);
        if b.is_null() {
            wr::<u8>(g::<GPtr<u8>>(BUILDING_POOL).read(), 0x11, 0);
            return GPtr::NULL;
        }
        entity_set_model_index_no_create(b, rd::<u32>(this, 4));
        wr::<u8>(g::<GPtr<u8>>(BUILDING_POOL).read(), 0x11, 0);
    }
    if surface_is_water(g(SURFACE_INFOS), u32::from(rd::<u8>(this, 0))) != 0 {
        let o = rd::<GPtr<u8>>(item, 8);
        wr(o, 0x1C, rd::<u32>(o, 0x1C) | 0x10_0000);
    }
    let o = rd::<GPtr<u8>>(item, 8);
    wr(o, 0x1C, rd::<u32>(o, 0x1C) | 0x400_0000);
    let k = gf(0x85_8C7C);
    let lerp = |site: u32, lo: f32, hi: f32| {
        let r = xi(rand_at(site)).fmul_m(k);
        x(hi).fsub_m(lo).fmul(r).fadd_m(lo).to_f32()
    };
    let xy = lerp(0x5A_347B, rd(this, 0x1C), rd(this, 0x20));
    let zs = lerp(0x5A_34B0, rd(this, 0x24), rd(this, 0x28));
    let rot = lerp(0x5A_34E5, rd(this, 0x14), rd(this, 0x18));
    let zoff = lerp(0x5A_351A, rd(this, 0x2C), rd(this, 0x30));
    let col = rd::<GPtr<u8>>(g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * rd::<i32>(this, 4) as usize).read(), 0x14);
    let minz = rd::<f32>(col, 8);
    if rd::<u8>(this, 0x34) != 0 && slope(normal[2]) {
        let mut m: Mat = [0; 18];
        let mut rv = [x(rot).fcos().to_f32(), x(rot).fsin().to_f32(), 0.0];
        let mut fwd = [0f32; 3];
        cross_product(vp(&mut fwd), vp(&mut normal), vp(&mut rv));
        vector_normalise(vp(&mut fwd).cast());
        let mut right = [0f32; 3];
        cross_product(vp(&mut right), vp(&mut fwd), vp(&mut normal));
        vector_normalise(vp(&mut right).cast());
        let zm = x(zoff).fsub_m(minz);
        let dz = x(normal[2]).fmul(zm).to_f32();
        let dy = x(normal[1]).fmul(zm).to_f32();
        let dx = zm.fmul_m(normal[0]).to_f32();
        let mv = mp(&mut m);
        for (o, v) in [(0, right), (0x10, fwd), (0x20, normal)] {
            wr(mv, o, v[0]);
            wr(mv, o + 4, v[1]);
            wr(mv, o + 8, v[2]);
        }
        let tz = x(dz).fadd_m(pos[2]);
        let ty = x(dy).fadd_m(pos[1]);
        wr(mv, 0x30, x(dx).fadd_m(pos[0]).to_f32());
        wr(mv, 0x34, ty.to_f32());
        wr(mv, 0x38, tz.to_f32());
        allocate_static_matrix(rd::<GPtr<u8>>(item, 8));
        placeable_set_matrix(rd::<GPtr<u8>>(item, 8), mv);
        wr::<u8>(item, 0x10, 1);
        let n = g::<i32>(PROC_OBJ_MAN);
        n.write(n.read().wrapping_add(1));
        matrix_dtor(mv);
    } else {
        pos[2] = x(pos[2]).fsub_m(minz).fadd_m(zoff).to_f32();
        placeable_set_posn(rd::<GPtr<u8>>(item, 8), vp(&mut pos).cast());
        set_orientation(rd::<GPtr<u8>>(item, 8), 0.0, 0.0, rot);
        wr::<u8>(item, 0x10, 0);
    }
    wr::<u8>(rd::<GPtr<u8>>(item, 8), 0x2F, g::<u8>(0xB7_2914).read());
    entity_set_is_static(rd::<GPtr<u8>>(item, 8), 1);
    entity_create_rw_object(rd::<GPtr<u8>>(item, 8));
    let o = rd::<GPtr<u8>>(item, 8);
    let rw = rd::<GPtr<u8>>(o, 0x18);
    if !rw.is_null() {
        let rwm = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let mat = rd::<GPtr<u8>>(o, 0x14);
        if mat.is_null() {
            simple_transform_update_rw_matrix(o.byte_add(4), rwm);
        } else {
            matrix_update_rw_matrix(mat, rwm);
        }
    }
    entity_update_rw_frame(rd::<GPtr<u8>>(item, 8));
    world_add(rd::<GPtr<u8>>(item, 8));
    list_add_item(this.byte_add(0x3C), item);
    if !is_object {
        let one = x(gf(0x85_8624));
        if fcom_parity(x(xy), one, 0x44) || fcom_parity(x(zs), one, 0x44) {
            let o = rd::<GPtr<u8>>(item, 8);
            let rw = rd::<GPtr<u8>>(o, 0x18);
            if !rw.is_null() && !rd::<GPtr<u8>>(rw, 4).byte_add(0x10).is_null() {
                let mut t: Mat = [0; 18];
                let tm = mp(&mut t);
                let src = get_modelling_matrix(o);
                matrix_ctor_rw(tm, src, 0);
                crate::markers::matrix_scale_xyz(tm, xy, xy, zs);
                let d = x(minz).fmul_m(zs).fsub_m(minz);
                wr(tm, 0x38, x(rd::<f32>(tm, 0x38)).fsub(d).to_f32());
                matrix_update_rw(tm);
                entity_update_rw_frame(rd::<GPtr<u8>>(item, 8));
                matrix_dtor(tm);
            }
        }
    }
    item
}

/// `ProcSurfaceInfo_c::AddObjects(CPlantLocTri* tri)` (0x5A3850, `ret 4`): the number of objects placed on the
/// triangle (each placed item's +0xC = the triangle).
///
/// Nothing when the camera is nearer the centroid than `sqrt(+0x10)`. The normal is `(v3 - v1) x (v2 - v1)`. With the
/// grid (+0x38 != 0): the triangle's bounding box snapped to the spacing (+8) through `_ftol`, the normal
/// normalised once, then every grid point (y restarting at each x) inside the triangle (0x5A2F80) gets an object at
/// the plane's height. Without: the density `|normal| * 0.5 * (+0xC)` objects (the last one with a random cut-off),
/// each at `v1 + (v2 - v1) r1 + (v3 - v2) r1 r2`, the normal normalised again each time, under `srand(ftol(sum of
/// the nine coordinates + model))`, `rand`'s previous draw restored with `srand` at the end.
#[hook(addr = 0x5A3850, conv = thiscall, name = "ProcSurfaceInfo_c::AddObjects", expect = "83 EC 7C 53 55 56 8B B4",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes",
       note = "rand at_site 0x5A3C7A/0x5A3CBE/0x5A3CE0/0x5A3CF7; srand at_site 0x5A3C82/0x5A3E00")]
#[allow(clippy::too_many_lines)]
pub fn add_objects(this: GPtr<u8>, tri: GPtr<u8>) -> i32 {
    let v = |o: isize| rd::<f32>(tri, o);
    let third = gf(0x86_7028);
    let x12 = x(v(0)).fadd_m(v(0xC)).fadd_m(v(0x18)).to_f32();
    let z12 = x(v(0x14)).fadd_m(v(8)).to_f32();
    let y123 = x(v(0x10)).fadd_m(v(4)).fadd_m(v(0x1C));
    let z123 = x(z12).fadd_m(v(0x20));
    let cx = x(x12).fmul_m(third).to_f32();
    let cy = y123.fmul_m(third).to_f32();
    let cz = z123.fmul_m(third).to_f32();
    let cam = cam_pos();
    let dx = x(cam.read()).fsub_m(cx);
    let dy = x(cam.byte_add(4).read()).fsub_m(cy);
    let dz = x(cam.byte_add(8).read()).fsub_m(cz);
    let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    if lt(d2, x(rd(this, 0x10))) {
        return 0;
    }
    let mut v1 = [x(v(0xC)).fsub_m(v(0)).to_f32(), x(v(0x10)).fsub_m(v(4)).to_f32(), x(v(0x14)).fsub_m(v(8)).to_f32()];
    let v2 = [x(v(0x18)).fsub_m(v(0xC)).to_f32(), x(v(0x1C)).fsub_m(v(0x10)).to_f32(), x(v(0x20)).fsub_m(v(0x14)).to_f32()];
    let mut v3 = [x(v(0x18)).fsub_m(v(0)).to_f32(), x(v(0x1C)).fsub_m(v(4)).to_f32(), x(v(0x20)).fsub_m(v(8)).to_f32()];
    let mut n = [0f32; 3];
    cross_product(vp(&mut n), vp(&mut v3), vp(&mut v1));
    let mut count: i32 = 0;
    let zero = x(gf(0x85_8B50));
    let spacing = || rd::<f32>(this, 8);
    let lighting = u32::from(rd::<u8>(tri, 0x47));
    let place = |p: [f32; 3], n: [f32; 3], count: &mut i32| {
        let r = add_object_o(this, p[0], p[1], p[2], n[0], n[1], n[2], lighting);
        if !r.is_null() {
            *count += 1;
            wr(r, 0xC, tri);
        }
    };
    if fcom_parity(x(rd(this, 0x38)), zero, 0x44) {
        // the grid
        let min3 = |a: f32, b: f32, c: f32| {
            let m = if lt(x(a), x(b)) { a } else { b };
            if lt(x(m), x(c)) { m } else { c }
        };
        let max3 = |a: f32, b: f32, c: f32| {
            let m = if le_or_nan(x(a), x(b)) { b } else { a };
            if gt(x(m), x(c)) { m } else { c }
        };
        let min_x = min3(v(0), v(0xC), v(0x18));
        let min_y = min3(v(4), v(0x10), v(0x1C));
        let max_x = max3(v(0), v(0xC), v(0x18));
        let max_y = max3(v(4), v(0x10), v(0x1C));
        let step = x(gf(0x85_8624)).fdiv_m(spacing());
        let snap = |i: i32| ftol32(xi(i).fmul_m(spacing()));
        let xs = snap(ftol32(x(min_x).fmul(step)));
        let xe = snap(ftol32(x(max_x).fmul(step)).wrapping_add(1));
        let ys = snap(ftol32(x(min_y).fmul(step)));
        let ye = snap(ftol32(x(max_y).fmul(step)).wrapping_add(1));
        vector_normalise(vp(&mut n).cast());
        let mut fx = xi(xs).to_f32();
        let fxe = xi(xe).to_f32();
        if !lt(x(fx), x(fxe)) {
            return count;
        }
        let fys = xi(ys).to_f32();
        let fye = xi(ye).to_f32();
        loop {
            let mut fy = fys;
            if lt(x(fy), x(fye)) {
                loop {
                    let mut z = 0f32;
                    if is_pt_in_triangle_2d_o(fx, fy, v(0), v(4), v(8), v(0xC), v(0x10), v(0x14), v(0x18), v(0x1C),
                                              v(0x20), n[0], n[1], n[2], GPtr::from_raw(&mut z)) != 0
                    {
                        place([fx, fy, z], n, &mut count);
                    }
                    fy = x(fy).fadd_m(spacing()).to_f32();
                    if !lt(x(fy), x(fye)) {
                        break;
                    }
                }
            }
            fx = x(fx).fadd_m(spacing()).to_f32();
            if !lt(x(fx), x(fxe)) {
                return count;
            }
        }
    }
    let mut density = x(n[2])
        .fmul_m(n[2])
        .fadd(x(n[1]).fmul_m(n[1]))
        .fadd(x(n[0]).fmul_m(n[0]))
        .fsqrt()
        .fmul_m(gf(0x85_8B8C))
        .fmul_m(rd(this, 0xC))
        .to_f32();
    let a = x(v(0x20)).fadd_m(v(0x1C)).fadd_m(v(0x18));
    let b = x(v(0x14)).fadd_m(v(0x10)).fadd_m(v(0xC));
    let c = x(v(8)).fadd_m(v(4)).fadd_m(v(0));
    let seed = ftol32(a.fadd(b).fadd(c).fadd(xi(rd::<i32>(this, 4))));
    let old = rand_at(0x5A_3C7A);
    at_site(0x5A_3C82, || srand(seed as u32));
    if le_or_nan(x(density), zero) {
        at_site(0x5A_3E00, || srand(old as u32));
        return count;
    }
    let k = gf(0x85_8C7C);
    let one = gf(0x85_8624);
    loop {
        let cut = if ge_or_nan(x(density), x(one)) { x(one) } else { xi(rand_at(0x5A_3CBE)).fmul_m(k) };
        if lt(cut, x(density)) {
            let o1 = xi(rand_at(0x5A_3CE0)).fmul_m(k).to_f32();
            let o2 = xi(rand_at(0x5A_3CF7)).fmul_m(k).fmul_m(o1);
            let a = x(v2[0]).fmul(o2);
            let a_y = x(v2[1]).fmul(o2).to_f32();
            let a_z = x(v2[2]).fmul(o2).to_f32();
            let b_x = x(v1[0]).fmul_m(o1);
            let b_y = x(v1[1]).fmul_m(o1);
            let b_z = x(v1[2]).fmul_m(o1).to_f32();
            let p_x = b_x.fadd_m(v(0)).to_f32();
            let p_y = b_y.fadd_m(v(4)).to_f32();
            let p_z = x(b_z).fadd_m(v(8)).to_f32();
            let p = [x(p_x).fadd(a).to_f32(), x(p_y).fadd_m(a_y).to_f32(), x(p_z).fadd_m(a_z).to_f32()];
            vector_normalise(vp(&mut n).cast());
            place(p, n, &mut count);
        }
        density = x(density).fsub_m(one).to_f32();
        if !gt(x(density), zero) {
            break;
        }
    }
    at_site(0x5A_3E00, || srand(old as u32));
    count
}

pub const HOOKS: &[HookReg] = &[
    HOOK_PROCESS_TRIANGLE_ADDED,
    HOOK_PROCESS_TRIANGLE_REMOVED,
    HOOK_MATRIX_SCALE_XYZ,
    HOOK_IS_PT_IN_TRIANGLE_2D,
    HOOK_ADD_OBJECT,
    HOOK_ADD_OBJECTS,
];
