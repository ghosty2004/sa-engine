//! `CCollisionData`, `CColModel`, `CColTrianglePlane`, the triangle-plane cache
//! (`CCollision::ms_colModelCache`, a `CLinkList<CCollisionData*>` of 50 links) and `CCollision`'s
//! init/shutdown.
//!
//! Ported from the 1.0 US bytes (capstone over the exe, `.HOODLUM` bodies followed); gta-reversed
//! `Collision/CollisionData.cpp`, `ColModel.cpp`, `ColTrianglePlane.cpp`, `Collision.cpp` and
//! `docs/re/specs/COLL/{CLinkList,CCollisionData.deviations,CColModel.deviations}.md` as reading
//! aids. Kept from the original (see `docs/arch/coll-port.md`):
//! - `CColModel::AllocateData(...)` writes only the counts, the pointers, `+0x1C = 0` and bit 0 of
//!   `+7`; the shadow fields and flag bits 1..7 keep what `CMemoryMgr::Malloc` returned (gr
//!   placement-news a zeroed `CCollisionData`, ColModel.cpp:127);
//! - `AllocateData()` and `MakeMultipleAlloc` keep bits 3..7 of `+7` of the new block (gr zeroes the
//!   byte); `MakeMultipleAlloc` copies into a null block when `operator new` fails;
//! - `CColModel::CalculateTrianglePlanes` calls `CCollisionData::CalculateTrianglePlanes` directly
//!   (no cache), as the bytes do;
//! - counts are read sign-extended (`movsx`) where the original does (`m_nNumTriangles` in
//!   Set/GetLinkPtr and CalculateTrianglePlanes).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use super::shared::{
    CColModel, CColTriangle, CColTrianglePlane, CCollisionData, CLink, CLinkList, CompressedVector, cf, k,
    read_vec, write_vec,
};
use crate::field;

pub type CdPtr = GPtr<CCollisionData>;
pub type CmPtr = GPtr<CColModel>;

sa_core::globals! {
    list GLOBALS;
    /// `CCollision::ms_colModelCache` (CLinkList<CCollisionData*>)
    pub static COL_MODEL_CACHE @ 0x96592C: CLinkList = CLinkList {
        used_head: LINK0, used_tail: LINK0, free_head: LINK0, free_tail: LINK0, links: GPtr::NULL,
    };
    /// `CCollision::ms_collisionInMemory`
    pub static COLLISION_IN_MEMORY @ 0x9655D4: u32 = 0;
    // foreign (owner: CCamera): `TheCamera` position words read by SortOutCollisionAfterLoad
    /// `TheCamera.GetPosition()` (x, y, z at 0xB6F9CC)
    pub static CAMERA_POSITION @ 0xB6F9CC: [f32; 3] = [0.0; 3];
}

const LINK0: CLink = CLink {
    data: GPtr::NULL,
    prev: GPtr::NULL,
    next: GPtr::NULL,
};

orig! {
    /// `CMemoryMgr::Malloc(size, hint)` (0x72F420)
    pub fn mem_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// `CMemoryMgr::Free` (0x72F430)
    pub fn mem_free(p: GPtr<u8>) = cdecl 0x72F430 => sa_core::memory::mgr_free;
    /// CRT `operator new` (0x82119A)
    pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// CRT `operator new` (0x821195), the `new CLink[50]` of `CCollision::Init`
    pub fn operator_new_links(size: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
    /// CRT `free` (0x8214BD)
    pub fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// CRT `operator delete` (0x8213AE), the links of `CCollision::Shutdown`
    pub fn operator_delete_links(p: GPtr<u8>) = cdecl 0x8213AE => unported;
    /// `CColModelPool::New` (0x40FB80)
    pub fn col_model_pool_new(pool: sa_core::pools::PoolPtr<sa_core::pools::ColModelSlot>)
        -> GPtr<sa_core::pools::ColModelSlot> = thiscall 0x40FB80 => sa_core::pools::col_model_pool_new;
    /// `CCollisionData::Copy` (0x40F120)
    pub fn col_data_copy(this: CdPtr, src: CdPtr) = thiscall 0x40F120 => unported;
    /// `CCollisionData::RemoveCollisionVolumes` (0x40F070)
    pub fn col_data_remove_collision_volumes(this: CdPtr) = thiscall 0x40F070 => remove_collision_volumes;
    /// `CCollisionData::CalculateTrianglePlanes` (0x40F590)
    pub fn col_data_calculate_triangle_planes(this: CdPtr) = thiscall 0x40F590 => calculate_triangle_planes;
    /// `CCollisionData::RemoveTrianglePlanes` (0x40F6A0)
    pub fn col_data_remove_triangle_planes(this: CdPtr) = thiscall 0x40F6A0 => remove_triangle_planes;
    /// `CCollisionData::GetLinkPtr` (0x40F6E0)
    pub fn col_data_get_link_ptr(this: CdPtr) -> GPtr<CLink> = thiscall 0x40F6E0 => get_link_ptr;
    /// `CCollisionData::SetLinkPtr` (0x40F6C0)
    pub fn col_data_set_link_ptr(this: CdPtr, link: GPtr<CLink>) = thiscall 0x40F6C0 => set_link_ptr;
    /// `CColTrianglePlane::Set` (0x411660)
    pub fn col_triangle_plane_set(this: GPtr<CColTrianglePlane>, verts: GPtr<CompressedVector>,
        tri: GPtr<CColTriangle>) = thiscall 0x411660 => triangle_plane_set;
    /// `CLinkList<CCollisionData*>::Insert` (0x4161F0)
    pub fn link_list_insert(this: GPtr<CLinkList>, data: GPtr<CdPtr>) -> GPtr<CLink>
        = thiscall 0x4161F0 => link_list_insert_port;
    /// `CCollision::RemoveTrianglePlanes(CCollisionData*)` (0x416400)
    pub fn collision_remove_triangle_planes(cd: CdPtr) = cdecl 0x416400 => remove_triangle_planes_col_data;
    /// `CCollision::CalculateTrianglePlanes(CCollisionData*)` (0x416330)
    pub fn collision_calculate_triangle_planes(cd: CdPtr) = cdecl 0x416330 => calculate_triangle_planes_col_data;
    /// `CColStore::Initialise` (0x4113F0)
    pub fn col_store_initialise() = cdecl 0x4113F0 => sa_stream::colstore::initialise;
    /// `CColStore::Shutdown` (0x4114D0)
    pub fn col_store_shutdown() = cdecl 0x4114D0 => sa_stream::colstore::shutdown;
    /// `CColStore::LoadCollision(CVector, bool)` (0x410860)
    pub fn col_store_load_collision(x: f32, y: f32, z: f32, ignore_player_veh: u8)
        = cdecl 0x410860 => sa_stream::colstore::load_collision;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    pub fn load_all_requested_models(only_priority: u8) = cdecl 0x40EA10 => sa_stream::streaming::load_all_requested_models;
    /// `CVector::CrossProduct(CVector* out, const CVector* a, const CVector* b)` (0x59C730)
    pub fn cross_product(out: GPtr<CVector>, a: GPtr<CVector>, b: GPtr<CVector>) -> GPtr<CVector>
        = cdecl 0x59C730 => cross_product_port;
    /// `CVector::Normalise` (0x59C910)
    pub fn normalise(v: GPtr<CVector>) = thiscall 0x59C910 => normalise_port;
}

/// The Rust side of `CrossProduct` 0x59C730 (`sa_math::vector::cross_product`, alias-safe).
pub fn cross_product_port(out: GPtr<CVector>, a: GPtr<CVector>, b: GPtr<CVector>) -> GPtr<CVector> {
    write_vec(out, sa_math::vector::cross_product(&read_vec(a), &read_vec(b)));
    out
}

/// The Rust side of `CVector::Normalise` 0x59C910.
pub fn normalise_port(v: GPtr<CVector>) {
    let mut x = read_vec(v);
    x.normalise();
    write_vec(v, x);
}

fn cd_field<T>(cd: CdPtr, off: isize) -> GPtr<T> {
    cd.cast::<u8>().byte_add(off).cast()
}

/// `fild(int16) * 1/128` stored as f32 (exact).
fn decompress(c: i16) -> f32 {
    X87::from_i32(c as i32).fmul_m(cf(k::F1_128)).to_f32()
}

/// `CCollisionData::GetTrianglePoint(CVector& out, int idx)` (0x40F5E0, `ret 8`): `out =
/// m_pVertices[idx] / 128`, the vertex pointer re-read for each component.
#[hook(addr = 0x40F5E0, conv = thiscall, name = "CCollisionData::GetTrianglePoint", expect = "8B 44 24 08 8B 51 14", this = "CCollisionData")]
pub fn get_triangle_point(this: CdPtr, out: GPtr<CVector>, idx: i32) {
    vertex_point(this, 0x14, out, idx)
}

/// `CCollisionData::GetShadTrianglePoint(CVector& out, int idx)` (0x40F640, `ret 8`): as
/// GetTrianglePoint with `m_pShadowVertices` (+0x28).
#[hook(addr = 0x40F640, conv = thiscall, name = "CCollisionData::GetShadTrianglePoint", expect = "8B 44 24 08 8B 51 28", this = "CCollisionData")]
pub fn get_shad_triangle_point(this: CdPtr, out: GPtr<CVector>, idx: i32) {
    vertex_point(this, 0x28, out, idx)
}

fn vertex_point(this: CdPtr, verts_off: isize, out: GPtr<CVector>, idx: i32) {
    let o = idx.wrapping_mul(6) as isize;
    let comp = |c: isize| -> f32 {
        let verts: GPtr<u8> = cd_field::<GPtr<u8>>(this, verts_off).read();
        decompress(verts.byte_add(o + c).cast::<i16>().read())
    };
    let o32 = out.cast::<f32>();
    o32.write(comp(0));
    o32.wrapping_index(1).write(comp(2));
    o32.wrapping_index(2).write(comp(4));
}

/// The link slot after the triangle planes: `(planes + (int16)numTris * 10 + 3) & ~3`.
fn link_slot(this: CdPtr) -> GPtr<GPtr<CLink>> {
    let n = field!(this, CCollisionData, num_triangles).cast::<i16>().read() as isize;
    let planes: GPtr<u8> = field!(this, CCollisionData, triangle_planes)
        .cast::<GPtr<u8>>()
        .read();
    let base = planes.addr();
    let aligned = base.wrapping_add((n * 10) as usize).wrapping_add(3) & !3usize;
    planes.byte_add(aligned.wrapping_sub(base) as isize).cast()
}

/// `CCollisionData::SetLinkPtr` (stub 0x40F6C0 → 0x1568990, `ret 4`).
#[hook(addr = 0x40F6C0, conv = thiscall, name = "CCollisionData::SetLinkPtr", expect = "E9 CB 92 15 01", this = "CCollisionData")]
pub fn set_link_ptr(this: CdPtr, link: GPtr<CLink>) {
    link_slot(this).write(link)
}

/// `CCollisionData::GetLinkPtr` (stub 0x40F6E0 → 0x156D850). No check of the link's data (gr assert).
#[hook(addr = 0x40F6E0, conv = thiscall, name = "CCollisionData::GetLinkPtr", expect = "E9 6B E1 15 01", this = "CCollisionData")]
pub fn get_link_ptr(this: CdPtr) -> GPtr<CLink> {
    link_slot(this).read()
}

/// `CCollisionData::RemoveTrianglePlanes` (0x40F6A0): `CMemoryMgr::Free(planes)` when set; `planes = 0`
/// in every case (protection chunk 0x401F88).
#[hook(addr = 0x40F6A0, conv = thiscall, name = "CCollisionData::RemoveTrianglePlanes", expect = "56 8B F1 8B 46 1C", this = "CCollisionData")]
pub fn remove_triangle_planes(this: CdPtr) {
    let p: GPtr<GPtr<u8>> = field!(this, CCollisionData, triangle_planes).cast();
    let planes = p.read();
    if !planes.is_null() {
        mem_free(planes);
    }
    p.write(GPtr::NULL);
}

/// `CCollisionData::CalculateTrianglePlanes` (0x40F590): `planes = Malloc(((int16)numTris + 1) * 10)`,
/// then `CColTrianglePlane::Set(&planes[i], vertices, &triangles[i])` for every triangle (count,
/// pointers re-read every iteration).
#[hook(addr = 0x40F590, conv = thiscall, name = "CCollisionData::CalculateTrianglePlanes", expect = "56 8B F1 0F BF 46 04", this = "CCollisionData")]
pub fn calculate_triangle_planes(this: CdPtr) {
    let num = |t: CdPtr| field!(t, CCollisionData, num_triangles).cast::<i16>().read() as i32;
    let size = (num(this).wrapping_add(1)).wrapping_mul(10) as u32;
    let planes = mem_malloc(size, 0);
    let pp: GPtr<GPtr<u8>> = field!(this, CCollisionData, triangle_planes).cast();
    pp.write(planes);
    let mut i = 0i32;
    while i < num(this) {
        let planes: GPtr<u8> = pp.read();
        let verts: GPtr<CompressedVector> = field!(this, CCollisionData, vertices)
            .cast::<GPtr<CompressedVector>>()
            .read();
        let tris: GPtr<CColTriangle> = field!(this, CCollisionData, triangles)
            .cast::<GPtr<CColTriangle>>()
            .read();
        col_triangle_plane_set(
            planes.byte_add(i.wrapping_mul(10) as isize).cast(),
            verts,
            tris.wrapping_index(i),
        );
        i += 1;
    }
}

/// `CCollisionData::RemoveCollisionVolumes` (stub 0x40F070 → 0x156FAE0): `CMemoryMgr::Free` of
/// spheres, lines, boxes, vertices, triangles, shadow triangles, shadow vertices (each when set), then
/// `CCollision::RemoveTrianglePlanes(this)`, then the counts and those pointers are zeroed (the shadow
/// counts and the flags byte are left).
#[hook(addr = 0x40F070, conv = thiscall, name = "CCollisionData::RemoveCollisionVolumes", expect = "E9 6B 0A 16 01", this = "CCollisionData")]
pub fn remove_collision_volumes(this: CdPtr) {
    const FREED: [isize; 7] = [0x08, 0x10, 0x0C, 0x14, 0x18, 0x2C, 0x28];
    for off in FREED {
        let p = cd_field::<GPtr<u8>>(this, off).read();
        if !p.is_null() {
            mem_free(p);
        }
    }
    collision_remove_triangle_planes(this);
    cd_field::<u16>(this, 0).write(0);
    cd_field::<u8>(this, 6).write(0);
    cd_field::<u16>(this, 2).write(0);
    cd_field::<u16>(this, 4).write(0);
    for off in [0x08, 0x10, 0x0C, 0x14, 0x18, 0x2C, 0x28] {
        cd_field::<GPtr<u8>>(this, off).write(GPtr::NULL);
    }
}

/// A new `CCollisionData` as `operator new(0x30)` + the inlined constructor: counts and pointers 0,
/// `+7 &= 0xF8` (bits 3..7 keep the heap's bytes). Null when `new` fails.
fn new_col_data() -> CdPtr {
    let p: CdPtr = operator_new(0x30).cast();
    if p.is_null() {
        return p;
    }
    let f7 = cd_field::<u8>(p, 7);
    let b = f7.read() & 0xF8;
    cd_field::<u16>(p, 0).write(0);
    cd_field::<u16>(p, 2).write(0);
    cd_field::<u16>(p, 4).write(0);
    cd_field::<u8>(p, 6).write(0);
    f7.write(b);
    for off in (0x08..0x30).step_by(4) {
        cd_field::<u32>(p, off).write(0);
    }
    p
}

fn cm_flags(this: CmPtr) -> GPtr<u8> {
    field!(this, CColModel, flags)
}

fn cm_data(this: CmPtr) -> GPtr<CdPtr> {
    field!(this, CColModel, col_data).cast()
}

/// `CColModel::MakeMultipleAlloc` (stub 0x40F740 → 0x1564A10): when bit 1 of `+0x29` is set, a new
/// `CCollisionData` (null if `new` fails) gets a `Copy` of the single-block data, the block is freed
/// (`CMemoryMgr::Free`), and the bit is cleared.
#[hook(addr = 0x40F740, conv = thiscall, name = "CColModel::MakeMultipleAlloc", expect = "E9 CB 52 15 01", this = "CColModel")]
pub fn make_multiple_alloc(this: CmPtr) {
    if cm_flags(this).read() & 2 == 0 {
        return;
    }
    let nd = new_col_data();
    col_data_copy(nd, cm_data(this).read());
    mem_free(cm_data(this).read().cast());
    let f = cm_flags(this).read() & 0xFD;
    cm_data(this).write(nd);
    cm_flags(this).write(f);
}

/// `CColModel::operator=` (stub 0x40F7C0 → 0x15697F0, `ret 4`, EAX = this): copies the bounding
/// sphere (+0x18, 4 dwords), then the box (6 dwords), then `m_pColData->Copy(*o.m_pColData)` when this
/// model has data. `+0x28..+0x2B` are not copied.
#[hook(addr = 0x40F7C0, conv = thiscall, name = "CColModel::operator=", expect = "E9 2B A0 15 01", this = "CColModel")]
pub fn assign(this: CmPtr, o: CmPtr) -> CmPtr {
    let (d, s) = (this.cast::<u32>(), o.cast::<u32>());
    for w in 6..10 {
        d.wrapping_index(w).write(s.wrapping_index(w).read());
    }
    for w in 0..6 {
        d.wrapping_index(w).write(s.wrapping_index(w).read());
    }
    let cd = cm_data(this).read();
    if !cd.is_null() {
        col_data_copy(cd, cm_data(o).read());
    }
    this
}

/// `CColModel::AllocateData()` (stub 0x40F810 → 0x156DEB0): clears bit 1 of `+0x29`, then
/// `m_pColData = new CCollisionData` (null when `new` fails).
#[hook(addr = 0x40F810, conv = thiscall, name = "CColModel::AllocateData-void", expect = "E9 9B E6 15 01", this = "CColModel")]
pub fn allocate_data(this: CmPtr) {
    let f = cm_flags(this).read() & 0xFD;
    cm_flags(this).write(f);
    let p = new_col_data();
    cm_data(this).write(p);
}

/// `CColModel::AllocateData(numSpheres, numBoxes, numLines, numVertices, numTriangles, bUsesDisks)`
/// (stub 0x40F870 → 0x1561730, `ret 0x18`): one `CMemoryMgr::Malloc` block (header 0x30, spheres,
/// lines (0x24 per disk / 0x20 per line), boxes, vertices, rounded up to 4, triangles); sets bit 1 of
/// `+0x29`. Writes only the fields listed in the module docs.
#[hook(addr = 0x40F870, conv = thiscall, name = "CColModel::AllocateData-params", expect = "E9 BB 1E 15 01", this = "CColModel")]
pub fn allocate_data_params(
    this: CmPtr,
    num_spheres: i32,
    num_boxes: i32,
    num_lines: i32,
    num_vertices: i32,
    num_triangles: i32,
    uses_disks: u32,
) {
    let disks = uses_disks as u8 != 0;
    let f = cm_flags(this).read() | 2;
    cm_flags(this).write(f);
    let after_spheres = num_spheres.wrapping_mul(20).wrapping_add(0x30);
    let line_size = if disks {
        num_lines.wrapping_mul(36)
    } else {
        num_lines.wrapping_mul(32)
    };
    let boxes = num_boxes.wrapping_mul(0x1C);
    let verts = num_vertices.wrapping_mul(6);
    let tris_off = after_spheres
        .wrapping_add(line_size)
        .wrapping_add(verts.wrapping_add(boxes))
        .wrapping_add(3)
        & !3;
    let block: CdPtr = mem_malloc(tris_off.wrapping_add(num_triangles.wrapping_mul(8)) as u32, 0).cast();
    cm_data(this).write(block);
    let cd = || cm_data(this).read();
    cd_field::<u16>(cd(), 0).write(num_spheres as u16);
    cd_field::<u8>(cd(), 6).write(num_lines as u8);
    cd_field::<u16>(cd(), 2).write(num_boxes as u16);
    cd_field::<u16>(cd(), 4).write(num_triangles as u16);
    let c = cd();
    cd_field::<GPtr<u8>>(c, 8).write(c.cast::<u8>().byte_add(0x30));
    let at = |off: i32| -> GPtr<u8> {
        let c = cd();
        c.cast::<u8>().byte_add(off as isize)
    };
    let mut off = after_spheres;
    if num_lines != 0 {
        cd_field::<GPtr<u8>>(cd(), 0x10).write(at(off));
        off = off.wrapping_add(line_size);
    } else {
        cd_field::<GPtr<u8>>(cd(), 0x10).write(GPtr::NULL);
    }
    if num_boxes != 0 {
        cd_field::<GPtr<u8>>(cd(), 0x0C).write(at(off));
        off = off.wrapping_add(boxes);
    } else {
        cd_field::<GPtr<u8>>(cd(), 0x0C).write(GPtr::NULL);
    }
    if num_vertices != 0 {
        cd_field::<GPtr<u8>>(cd(), 0x14).write(at(off));
        off = verts.wrapping_add(off).wrapping_add(3) & !3;
    } else {
        cd_field::<GPtr<u8>>(cd(), 0x14).write(GPtr::NULL);
    }
    if num_triangles != 0 {
        cd_field::<GPtr<u8>>(cd(), 0x18).write(at(off));
    } else {
        cd_field::<GPtr<u8>>(cd(), 0x18).write(GPtr::NULL);
    }
    cd_field::<GPtr<u8>>(cd(), 0x1C).write(GPtr::NULL);
    let f7 = cd_field::<u8>(cd(), 7);
    let b = f7.read();
    f7.write(b ^ ((b ^ uses_disks as u8) & 1));
}

/// `CColModel::RemoveCollisionVolumes` (0x40F9E0): with data, a single block (bit 1 of `+0x29`) gets
/// `CCollision::RemoveTrianglePlanes` + `CMemoryMgr::Free`; otherwise
/// `CCollisionData::RemoveCollisionVolumes` + CRT `free`. Then `m_pColData = 0`.
#[hook(addr = 0x40F9E0, conv = thiscall, name = "CColModel::RemoveCollisionVolumes", expect = "56 8B F1 8B 4E 2C", this = "CColModel")]
pub fn col_model_remove_collision_volumes(this: CmPtr) {
    let cd = cm_data(this).read();
    if cd.is_null() {
        return;
    }
    if cm_flags(this).read() & 2 != 0 {
        collision_remove_triangle_planes(cd);
        mem_free(cm_data(this).read().cast());
    } else {
        col_data_remove_collision_volumes(cd);
        crt_free(cm_data(this).read().cast());
    }
    cm_data(this).write(GPtr::NULL);
}

/// `CColModel::CalculateTrianglePlanes` (0x40FA30): `CCollisionData::CalculateTrianglePlanes` of the
/// data when set (tail jump).
#[hook(addr = 0x40FA30, conv = thiscall, name = "CColModel::CalculateTrianglePlanes", expect = "8B 49 2C 85 C9", this = "CColModel")]
pub fn col_model_calculate_triangle_planes(this: CmPtr) {
    let cd = cm_data(this).read();
    if !cd.is_null() {
        col_data_calculate_triangle_planes(cd);
    }
}

/// `CColModel::RemoveTrianglePlanes` (0x40FA40): with data, `CMemoryMgr::Free(planes)` when set and
/// `planes = 0` (inlined `CCollisionData::RemoveTrianglePlanes`).
#[hook(addr = 0x40FA40, conv = thiscall, name = "CColModel::RemoveTrianglePlanes", expect = "56 8B 71 2C 85 F6", this = "CColModel")]
pub fn col_model_remove_triangle_planes(this: CmPtr) {
    let cd = cm_data(this).read();
    if cd.is_null() {
        return;
    }
    let p: GPtr<GPtr<u8>> = field!(cd, CCollisionData, triangle_planes).cast();
    let planes = p.read();
    if !planes.is_null() {
        mem_free(planes);
    }
    p.write(GPtr::NULL);
}

/// `CColModel::operator new` (0x40FC30): `mov ecx, ms_pColModelPool; jmp CColModelPool::New`.
#[hook(addr = 0x40FC30, conv = cdecl, name = "CColModel::operator new", expect = "8B 0D A4 44 B7 00")]
pub fn col_model_operator_new(_size: u32) -> GPtr<sa_core::pools::ColModelSlot> {
    col_model_pool_new(sa_core::pools::COL_MODEL_POOL.get())
}

/// `CColModel::operator delete` (stub 0x40FC40 → 0x1570190): the inlined `CPool::Delete` (slot 0x30,
/// magic 0x2AAAAAAB `sar 3`), no null check.
#[hook(addr = 0x40FC40, conv = cdecl, name = "CColModel::operator delete", expect = "E9 4B 05 16 01")]
pub fn col_model_operator_delete(obj: GPtr<sa_core::pools::ColModelSlot>) {
    sa_core::pools::pool_delete_exact(sa_core::pools::COL_MODEL_POOL.get(), obj)
}

/// `CColTrianglePlane::GetNormal(CVector& out)` (0x411610, `ret 4`): `fild(int16) * 1/4096` per
/// component.
#[hook(addr = 0x411610, conv = thiscall, name = "CColTrianglePlane::GetNormal-Out", expect = "51 0F BF 01 89 44 24 00", this = "CColTrianglePlane")]
pub fn get_normal(this: GPtr<CColTrianglePlane>, out: GPtr<CVector>) {
    let p = this.read();
    let s = |c: i16| X87::from_i32(c as i32).fmul_m(cf(k::F1_4096)).to_f32();
    let o = out.cast::<f32>();
    o.write(s(p.nx));
    o.wrapping_index(1).write(s(p.ny));
    o.wrapping_index(2).write(s(p.nz));
}

fn vertex(verts: GPtr<CompressedVector>, idx: u16) -> CVector {
    let v = verts.wrapping_index(idx as i32).read();
    CVector::new(decompress(v.x), decompress(v.y), decompress(v.z))
}

/// `CColTrianglePlane::Set(const CompressedVector* verts, const CColTriangle& tri)` (0x411660,
/// `ret 8`). `n = Normalise(CrossProduct(C - A, B - A))` (both callees called), stored as
/// `int16(_ftol(n * 4096))`; `dist = int16(_ftol(((A.z*n'.z + A.y*n'.y) + A.x*n'.x) * 128))` with `n'` the
/// stored normal re-expanded; orientation 0/1 (+x/-x), 2/3 (y), 4/5 (z) of the largest |n'| component
/// (x needs to be strictly largest, then y strictly larger than z), `> 0` for the even value.
#[hook(addr = 0x411660, conv = thiscall, name = "CColTrianglePlane::Set", expect = "83 EC 3C 8B 54 24 40", this = "CColTrianglePlane")]
pub fn triangle_plane_set(
    this: GPtr<CColTrianglePlane>,
    verts: GPtr<CompressedVector>,
    tri: GPtr<CColTriangle>,
) {
    let t = tri.read();
    let a = vertex(verts, t.a);
    let b = vertex(verts, t.b);
    let c = vertex(verts, t.c);
    let sub = |p: f32, q: f32| X87::from_f32(p).fsub_m(q).to_f32();
    let mut ab = CVector::new(sub(b.x, a.x), sub(b.y, a.y), sub(b.z, a.z));
    let mut ac = CVector::new(sub(c.x, a.x), sub(c.y, a.y), sub(c.z, a.z));
    let mut n = CVector::default();
    let np = GPtr::from_raw(&mut n as *mut CVector);
    let r = cross_product(
        np,
        GPtr::from_raw(&mut ac as *mut CVector),
        GPtr::from_raw(&mut ab as *mut CVector),
    );
    let mut nn = read_vec(r);
    normalise(GPtr::from_raw(&mut nn as *mut CVector));
    let w = |v: f32| X87::from_f32(v).fmul_m(cf(k::F4096)).ftol() as u16 as i16;
    let (wx, wy, wz) = (w(nn.x), w(nn.y), w(nn.z));
    let p = this.cast::<i16>();
    p.write(wx);
    p.wrapping_index(1).write(wy);
    p.wrapping_index(2).write(wz);
    let e = |v: i16| X87::from_i32(v as i32).fmul_m(cf(k::F1_4096));
    let (ex, ey, ez) = (e(wx), e(wy).to_f32(), e(wz).to_f32());
    let d = X87::from_f32(a.z)
        .fmul_m(ez)
        .fadd(X87::from_f32(a.y).fmul_m(ey))
        .fadd(X87::from_f32(a.x).fmul(ex))
        .fmul_m(cf(k::F128))
        .ftol() as u16 as i16;
    p.wrapping_index(3).write(d);
    let (x, y, z) = (ex.to_f64(), ey as f64, ez as f64);
    let o = if x.abs() > y.abs() && x.abs() > z.abs() {
        if x > 0.0 { 0 } else { 1 }
    } else if y.abs() > z.abs() {
        if y > 0.0 { 2 } else { 3 }
    } else if z > 0.0 {
        4
    } else {
        5
    };
    this.cast::<u8>().byte_add(8).write(o);
}

/// `CLinkList<CCollisionData*>::Insert(const T&)` (stub 0x4161F0 → 0x1570010, `ret 4`; spec
/// `docs/re/specs/COLL/CLinkList.md`): takes the first free link (null when the free list is empty),
/// stores the data, unlinks it and inserts it after the used-list head.
#[hook(addr = 0x4161F0, conv = thiscall, name = "thunk_FUN_01570010", expect = "E9 1B 9E 15 01", this = "CLinkList<void*>")]
pub fn link_list_insert_port(this: GPtr<CLinkList>, data: GPtr<CdPtr>) -> GPtr<CLink> {
    let free_tail: GPtr<CLink> = field!(this, CLinkList, free_tail).cast();
    let link: GPtr<CLink> = field!(this, CLinkList, free_head).cast::<CLink>().read().next;
    if link == free_tail {
        return GPtr::NULL;
    }
    lk_data(link).write(data.read());
    unlink(link);
    insert_after(this.cast(), link);
    link
}

fn lk_data(l: GPtr<CLink>) -> GPtr<CdPtr> {
    field!(l, CLink, data).cast()
}
fn lk_prev(l: GPtr<CLink>) -> GPtr<GPtr<CLink>> {
    field!(l, CLink, prev).cast()
}
fn lk_next(l: GPtr<CLink>) -> GPtr<GPtr<CLink>> {
    field!(l, CLink, next).cast()
}

/// `link->next->prev = link->prev; link->prev->next = link->next` (each pointer re-read).
fn unlink(l: GPtr<CLink>) {
    let next = lk_next(l).read();
    lk_prev(next).write(lk_prev(l).read());
    let prev = lk_prev(l).read();
    lk_next(prev).write(lk_next(l).read());
}

/// Inserts `l` after `head`: `l->next = head->next; head->next->prev = l; l->prev = head;
/// head->next = l` (head->next re-read).
fn insert_after(head: GPtr<CLink>, l: GPtr<CLink>) {
    lk_next(l).write(lk_next(head).read());
    lk_prev(lk_next(head).read()).write(l);
    lk_prev(l).write(head);
    lk_next(head).write(l);
}

fn cache() -> GPtr<CLinkList> {
    COL_MODEL_CACHE.ptr()
}

fn used_head() -> GPtr<CLink> {
    field!(cache(), CLinkList, used_head).cast()
}

fn free_head() -> GPtr<CLink> {
    field!(cache(), CLinkList, free_head).cast()
}

/// `CCollision::Update` (0x411E20): a lone `ret` (too short to hook).
pub fn update() {}

/// `CCollision::SortOutCollisionAfterLoad` (stub 0x411E30 → 0x15678D0):
/// `CColStore::LoadCollision(TheCamera.GetPosition(), false)`, `CStreaming::LoadAllRequestedModels(false)`.
#[hook(addr = 0x411E30, conv = cdecl, name = "CCollision::SortOutCollisionAfterLoad", expect = "E9 9B 5A 15 01")]
pub fn sort_out_collision_after_load() {
    let p = CAMERA_POSITION.get();
    col_store_load_collision(p[0], p[1], p[2], 0);
    load_all_requested_models(0);
}

/// `CCollision::Init` (0x416260): `ms_colModelCache.Init(50)` (`operator new(0x258)` 0x821195, the
/// used and free lists emptied, the 50 links pushed onto the free list from the last to the first),
/// `ms_collisionInMemory = 0`, `CColStore::Initialise()` (tail jump).
#[hook(addr = 0x416260, conv = cdecl, name = "CCollision::Init", expect = "68 58 02 00 00")]
pub fn init() {
    let links: GPtr<CLink> = operator_new_links(0x258).cast();
    let c = cache();
    field!(c, CLinkList, links).cast::<GPtr<CLink>>().write(links);
    let (uh, fh) = (used_head(), free_head());
    let ut: GPtr<CLink> = field!(c, CLinkList, used_tail).cast();
    let ft: GPtr<CLink> = field!(c, CLinkList, free_tail).cast();
    lk_next(uh).write(ut);
    lk_prev(ut).write(uh);
    lk_next(fh).write(ft);
    lk_prev(ft).write(fh);
    let mut first = ft;
    let mut i = 50;
    while i > 0 {
        i -= 1;
        let links: GPtr<CLink> = field!(c, CLinkList, links).cast::<GPtr<CLink>>().read();
        let l = links.wrapping_index(i);
        lk_next(l).write(first);
        lk_prev(lk_next(fh).read()).write(l);
        first = l;
        lk_prev(l).write(fh);
        lk_next(fh).write(l);
    }
    COLLISION_IN_MEMORY.set(0);
    col_store_initialise();
}

/// `CCollision::Shutdown` (0x4162E0): walks the used list backwards from the tail (`prev` links) and
/// calls `CCollisionData::RemoveTrianglePlanes` for every link with data; frees the links (0x8213AE),
/// `links = 0`; `CColStore::Shutdown()` (tail jump). The lists themselves are left as they are.
#[hook(addr = 0x4162E0, conv = cdecl, name = "CCollision::Shutdown", expect = "56 8B 35 3C 59 96 00")]
pub fn shutdown() {
    let c = cache();
    let uh = used_head();
    let ut: GPtr<CLink> = field!(c, CLinkList, used_tail).cast();
    let mut l = lk_prev(ut).read();
    while l != uh {
        let d = lk_data(l).read();
        if !d.is_null() {
            col_data_remove_triangle_planes(d);
        }
        l = lk_prev(l).read();
    }
    let links_p: GPtr<GPtr<CLink>> = field!(c, CLinkList, links).cast();
    operator_delete_links(links_p.read().cast());
    links_p.write(GPtr::NULL);
    col_store_shutdown();
}

/// `CCollision::CalculateTrianglePlanes(CCollisionData*)` (0x416330). Nothing without triangles.
/// With planes already present the data's link moves to the front of the used list. Otherwise a link
/// is inserted; when the cache is full the least recently used entry (the used tail's `prev`) loses
/// its planes and its link goes back to the free list first. Then the planes are computed and the link
/// stored after them.
#[hook(addr = 0x416330, conv = cdecl, name = "CCollision::CalculateTrianglePlanes-colData", expect = "57 8B 7C 24 08")]
pub fn calculate_triangle_planes_col_data(cd: CdPtr) {
    if field!(cd, CCollisionData, num_triangles).cast::<u16>().read() == 0 {
        return;
    }
    if !field!(cd, CCollisionData, triangle_planes)
        .cast::<GPtr<u8>>()
        .read()
        .is_null()
    {
        let l = col_data_get_link_ptr(cd);
        unlink(l);
        insert_after(used_head(), l);
        return;
    }
    let mut arg = cd;
    let argp = GPtr::from_raw(&mut arg as *mut CdPtr);
    let mut link = link_list_insert(cache(), argp);
    if link.is_null() {
        let ut: GPtr<CLink> = field!(cache(), CLinkList, used_tail).cast();
        let lru = lk_prev(ut).read();
        col_data_remove_triangle_planes(lk_data(lru).read());
        unlink(lru);
        insert_after(free_head(), lru);
        link = link_list_insert(cache(), argp);
    }
    col_data_calculate_triangle_planes(cd);
    col_data_set_link_ptr(cd, link);
}

/// `CCollision::RemoveTrianglePlanes(CCollisionData*)` (0x416400): with planes, the data's link goes
/// back to the front of the free list, then `CCollisionData::RemoveTrianglePlanes` (tail jump).
#[hook(addr = 0x416400, conv = cdecl, name = "CCollision::RemoveTrianglePlanes-colData", expect = "56 8B 74 24 08")]
pub fn remove_triangle_planes_col_data(cd: CdPtr) {
    if field!(cd, CCollisionData, triangle_planes)
        .cast::<GPtr<u8>>()
        .read()
        .is_null()
    {
        return;
    }
    let l = col_data_get_link_ptr(cd);
    unlink(l);
    insert_after(free_head(), l);
    col_data_remove_triangle_planes(cd);
}

/// `CCollision::CalculateTrianglePlanes(CColModel*)` (0x418580): the colData version when the model
/// has data (tail jump).
#[hook(addr = 0x418580, conv = cdecl, name = "CCollision::CalculateTrianglePlanes-CColModelPtr", expect = "8B 44 24 04 8B 40 2C")]
pub fn calculate_triangle_planes_col_model(cm: CmPtr) {
    let cd = cm_data(cm).read();
    if !cd.is_null() {
        collision_calculate_triangle_planes(cd);
    }
}

/// `CCollision::RemoveTrianglePlanes(CColModel*)` (0x4185A0).
#[hook(addr = 0x4185A0, conv = cdecl, name = "CCollision::RemoveTrianglePlanes-CColModelPtr", expect = "8B 44 24 04 8B 40 2C")]
pub fn remove_triangle_planes_col_model(cm: CmPtr) {
    let cd = cm_data(cm).read();
    if !cd.is_null() {
        collision_remove_triangle_planes(cd);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_REMOVE_COLLISION_VOLUMES,
    HOOK_CALCULATE_TRIANGLE_PLANES,
    HOOK_GET_TRIANGLE_POINT,
    HOOK_GET_SHAD_TRIANGLE_POINT,
    HOOK_REMOVE_TRIANGLE_PLANES,
    HOOK_SET_LINK_PTR,
    HOOK_GET_LINK_PTR,
    HOOK_MAKE_MULTIPLE_ALLOC,
    HOOK_ASSIGN,
    HOOK_ALLOCATE_DATA,
    HOOK_ALLOCATE_DATA_PARAMS,
    HOOK_COL_MODEL_REMOVE_COLLISION_VOLUMES,
    HOOK_COL_MODEL_CALCULATE_TRIANGLE_PLANES,
    HOOK_COL_MODEL_REMOVE_TRIANGLE_PLANES,
    HOOK_COL_MODEL_OPERATOR_NEW,
    HOOK_COL_MODEL_OPERATOR_DELETE,
    HOOK_GET_NORMAL,
    HOOK_TRIANGLE_PLANE_SET,
    HOOK_SORT_OUT_COLLISION_AFTER_LOAD,
    HOOK_LINK_LIST_INSERT_PORT,
    HOOK_INIT,
    HOOK_SHUTDOWN,
    HOOK_CALCULATE_TRIANGLE_PLANES_COL_DATA,
    HOOK_REMOVE_TRIANGLE_PLANES_COL_DATA,
    HOOK_CALCULATE_TRIANGLE_PLANES_COL_MODEL,
    HOOK_REMOVE_TRIANGLE_PLANES_COL_MODEL,
];
